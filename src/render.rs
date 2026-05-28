use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::*,
            Direct2D::Common::*,
            DirectWrite::*,
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::*,
        },
        System::Com::*,
        UI::{
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::*,
            Shell::ShellExecuteW,
            WindowsAndMessaging::*,
        },
        Graphics::Imaging::*,
    },
};
use windows_numerics::Vector2;

use crate::{docs, layout::{self, DrawCmd, Layout}, mermaid, parser, search, theme};

const WM_RELAYOUT: u32 = WM_APP;
const D2DERR_RECREATE_TARGET: i32 = 0x8899000Cu32 as i32;

// Sidebar geometry constants
const DIV_HIT: f32 = 5.0;   // px each side of divider that counts as "on divider"
const TAB_W:   f32 = 16.0;  // reveal-tab width when sidebar is hidden
const BTN_H:   f32 = 24.0;  // toggle-button height
const BTN_W:   f32 = 16.0;  // toggle-button width
const SB_MIN:  f32 = 80.0;  // minimum sidebar width when dragging
const SCRW:    f32 = theme::SCROLLBAR_W;

// ── Find panel geometry (DIPs) ─────────────────────────────────────────────
const FIND_W:        f32 = 380.0;
const FIND_TOP:      f32 = 8.0;
const FIND_RIGHT:    f32 = 16.0;    // gap from scrollbar
const FIND_INPUT_H:  f32 = 34.0;
const FIND_ROW_H:    f32 = 28.0;
const FIND_MAX_ROWS: usize = 14;    // visible result rows before overflow truncates
const FIND_PAD:      f32 = 8.0;

// ── process-wide singletons ────────────────────────────────────────────────
use std::sync::OnceLock;
static G_D2D: OnceLock<ID2D1Factory1>   = OnceLock::new();
static G_DW:  OnceLock<IDWriteFactory2> = OnceLock::new();

fn d2d() -> &'static ID2D1Factory1   { G_D2D.get().unwrap() }
fn dw()  -> &'static IDWriteFactory2 { G_DW.get().unwrap() }

// ── per-thread caches ──────────────────────────────────────────────────────
#[derive(PartialEq, Eq, Hash, Clone)]
struct FmtKey { family: String, size_q: u32, bold: bool, italic: bool }

#[derive(PartialEq, Eq, Hash, Clone)]
struct MeasureKey { text: String, family: String, size_q: u32, bold: bool, italic: bool }

thread_local! {
    static FMT_CACHE: RefCell<std::collections::HashMap<FmtKey, IDWriteTextFormat>> =
        RefCell::new(std::collections::HashMap::new());
    static MEASURE_CACHE: RefCell<std::collections::HashMap<MeasureKey, f32>> =
        RefCell::new(std::collections::HashMap::new());
}

unsafe fn get_fmt(family: &str, size: f32, bold: bool, italic: bool) -> Result<IDWriteTextFormat> {
    let key = FmtKey { family: family.to_owned(), size_q: (size * 64.0) as u32, bold, italic };
    FMT_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        let fmt = if let Some(f) = cache.get(&key) {
            f.clone()
        } else {
            let weight = if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_REGULAR };
            let style  = if italic { DWRITE_FONT_STYLE_ITALIC } else { DWRITE_FONT_STYLE_NORMAL };
            let fw: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
            let fmt = dw().CreateTextFormat(
                PCWSTR(fw.as_ptr()), None, weight, style,
                DWRITE_FONT_STRETCH_NORMAL, size, w!("en-us"),
            )?;
            cache.insert(key, fmt.clone());
            fmt
        };
        // Always reset alignment to defaults — callers that want different
        // alignment must set it themselves. This keeps the cache safe to share.
        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
        let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
        Ok(fmt)
    })
}

// ── app state ─────────────────────────────────────────────────────────────
struct App {
    hwnd: HWND,
    target: Option<ID2D1HwndRenderTarget>,
    docs_dir: PathBuf,
    files: Vec<docs::DocFile>,
    sidebar: Vec<docs::SidebarEntry>,
    current: usize,
    layout: Option<Layout>,
    scroll_y: f32,
    max_scroll: f32,
    width: f32,   // in DIPs
    height: f32,  // in DIPs
    dpi: f32,     // current monitor DPI (e.g. 96, 120, 144)
    // Sidebar resize / hide
    sidebar_w: f32,
    sidebar_saved: f32,
    dragging_div: bool,
    div_start_x: f32,
    div_start_w: f32,
    hover_div: bool,
    hover_toggle: bool,
    hover_tab: bool,
    // Scroll bar drag
    dragging_scroll: bool,
    drag_start_y: f32,
    drag_start_scroll: f32,
    // Content
    hover_sidebar: Option<usize>,
    hover_link: Option<usize>,
    // Navigation
    history: Vec<usize>,
    forward: Vec<usize>,
    // Header image (optional docs/header.png)
    header_path:      Option<std::path::PathBuf>,
    header_bitmap:    Option<ID2D1Bitmap>,
    header_display_h: f32,
    // Cached bitmaps for toolbar / inline images (keyed by absolute path string)
    image_cache:      std::collections::HashMap<String, ID2D1Bitmap>,
    hover_toolbar:    Option<usize>,
    // Cached D2D brushes keyed by RRGGBB hex — avoids CreateSolidColorBrush per draw call.
    // RefCell allows interior mutability so draw() methods can keep &self.
    brushes:          std::cell::RefCell<std::collections::HashMap<u32, ID2D1SolidColorBrush>>,
    // Cached parse result for the current doc — invalidated only on navigation
    parsed:           Option<Vec<crate::parser::Block>>,
    // Pre-warmed parse cache for non-current docs, populated by a background thread.
    // Shared via Arc<Mutex<..>> with the warm-up worker spawned in `run()`.
    parsed_cache:     Arc<Mutex<std::collections::HashMap<usize, Vec<crate::parser::Block>>>>,
    // ── Find feature ──────────────────────────────────────────────────────
    find_open:        bool,
    find_query:       String,
    find_results:     Vec<search::Hit>,
    find_selected:    usize,
    hover_find_row:   Option<usize>,
    /// After a search-triggered navigate, scroll the new layout to the heading
    /// whose plain text matches this string. Cleared after one relayout.
    pending_scroll_heading: Option<String>,
}

impl App {
    fn new(hwnd: HWND, docs_dir: &Path) -> Self {
        let (files, sidebar) = docs::scan(docs_dir);
        Self {
            hwnd,
            target: None,
            docs_dir: docs_dir.to_path_buf(),
            files,
            sidebar,
            current: 0,
            layout: None,
            scroll_y: 0.0,
            max_scroll: 0.0,
            width: 1100.0,
            height: 720.0,
            dpi: 96.0,
            sidebar_w: theme::SIDEBAR_W,
            sidebar_saved: theme::SIDEBAR_W,
            dragging_div: false,
            div_start_x: 0.0,
            div_start_w: 0.0,
            hover_div: false,
            hover_toggle: false,
            hover_tab: false,
            dragging_scroll: false,
            drag_start_y: 0.0,
            drag_start_scroll: 0.0,
            hover_sidebar: None,
            hover_link: None,
            history: Vec::new(),
            forward: Vec::new(),
            header_path: {
                let p = docs_dir.join("header.png");
                if p.exists() { Some(p) } else { None }
            },
            header_bitmap:    None,
            header_display_h: 0.0,
            image_cache:      std::collections::HashMap::new(),
            hover_toolbar:    None,
            brushes:          std::cell::RefCell::new(std::collections::HashMap::new()),
            parsed:           None,
            parsed_cache:     Arc::new(Mutex::new(std::collections::HashMap::new())),
            find_open:        false,
            find_query:       String::new(),
            find_results:     Vec::new(),
            find_selected:    0,
            hover_find_row:   None,
            pending_scroll_heading: None,
        }
    }

    fn sw(&self) -> f32 { self.sidebar_w }
    fn hidden(&self) -> bool { self.sidebar_w < 1.0 }
    /// DPI scale factor: physical pixels per DIP.
    fn scale(&self) -> f32 { self.dpi / 96.0 }

    fn toggle_sidebar(&mut self) {
        if self.hidden() {
            self.sidebar_w = if self.sidebar_saved >= SB_MIN { self.sidebar_saved } else { theme::SIDEBAR_W };
        } else {
            self.sidebar_saved = self.sidebar_w;
            self.sidebar_w = 0.0;
        }
        self.layout = None; // content width changed
        unsafe { let _ = PostMessageW(Some(self.hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0)); }
    }

    // The ‹/› toggle button rect (straddles the divider line when visible;
    // or the right edge of the reveal tab when hidden).
    fn toggle_btn_rect(&self) -> (f32, f32, f32, f32) {
        let by = (self.height / 2.0 - BTN_H / 2.0).round();
        if self.hidden() {
            (0.0, by, TAB_W, BTN_H)
        } else {
            (self.sw() - BTN_W / 2.0, by, BTN_W, BTN_H)
        }
    }

    fn on_toggle_btn(&self, x: f32, y: f32) -> bool {
        let (bx, by, bw, bh) = self.toggle_btn_rect();
        x >= bx && x <= bx + bw && y >= by && y <= by + bh
    }

    // True if mouse is in the divider drag zone (only when sidebar is visible)
    fn on_divider(&self, x: f32) -> bool {
        !self.hidden() && (x - self.sw()).abs() < DIV_HIT
    }

    // True if mouse is in the reveal tab (only when sidebar is hidden)
    fn on_tab(&self, x: f32) -> bool {
        self.hidden() && x < TAB_W
    }

    fn relayout(&mut self) {
        if self.files.is_empty() { return; }
        // Only re-read and re-parse when the document changes (not on resize/sidebar drag).
        if self.parsed.is_none() {
            // Try the pre-warmed cache first; fall back to a synchronous parse
            // if the background thread hasn't reached this doc yet.
            let cached = self.parsed_cache.lock().unwrap().remove(&self.current);
            self.parsed = Some(cached.unwrap_or_else(|| {
                let md = docs::load(&self.files[self.current].path);
                parser::parse(&md)
            }));
        }
        let blocks = self.parsed.as_ref().unwrap();
        let content_left = self.sw() + (if self.hidden() { TAB_W } else { 0.0 }) + theme::H_PAD;
        let content_w = (self.width - content_left - theme::H_PAD - SCRW).max(80.0);
        // Compute header display height from bitmap aspect ratio + content width.
        self.header_display_h = self.header_bitmap.as_ref().map(|bmp| unsafe {
            let sz = bmp.GetSize();
            if sz.width > 0.0 { content_w * (sz.height / sz.width) } else { 0.0 }
        }).unwrap_or(0.0);
        let y_start = self.header_display_h + theme::V_PAD;
        let ly = layout::layout(&blocks, content_left, content_w, y_start, measure_text);
        self.max_scroll = (ly.total_h - self.height + theme::V_PAD).max(0.0);
        // Apply any pending scroll-to-heading from a find activation.
        if let Some(target) = self.pending_scroll_heading.take() {
            if let Some((_, y)) = ly.headings.iter().find(|(t, _)| t == &target) {
                self.scroll_y = (y - theme::V_PAD).clamp(0.0, self.max_scroll);
            }
        }
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll);
        self.layout = Some(ly);
        // Update window title to show current document
        if let Some(file) = self.files.get(self.current) {
            unsafe {
                let title = format!("DocCrate \u{2014} {}", pretty(&file.name));
                let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = SetWindowTextW(self.hwnd, PCWSTR(title_w.as_ptr()));
            }
        }
    }

    fn navigate(&mut self, idx: usize) {
        if idx == self.current { return; }
        self.history.push(self.current);
        self.forward.clear();
        // Stash the current doc's parse back into the cache so navigating
        // backward is also instant.
        if let Some(blocks) = self.parsed.take() {
            self.parsed_cache.lock().unwrap().insert(self.current, blocks);
        }
        self.current = idx;
        self.scroll_y = 0.0;
        self.layout = None;
        self.parsed = None;
        unsafe { let _ = PostMessageW(Some(self.hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0)); }
    }

    fn nav_href(&mut self, href: &str) {
        if href.starts_with('#') { return; }
        let cur = self.files.get(self.current).map(|f| f.path.clone()).unwrap_or_default();
        if let Some(path) = docs::resolve_href(href, &cur, &self.docs_dir) {
            if let Some(idx) = self.files.iter().position(|f| f.path == path) {
                self.navigate(idx);
                return;
            }
        }
        unsafe {
            let hw: Vec<u16> = href.encode_utf16().chain(std::iter::once(0)).collect();
            ShellExecuteW(None, w!("open"), PCWSTR(hw.as_ptr()), None, None, SW_SHOWNORMAL);
        }
    }

    // ── Find feature ──────────────────────────────────────────────────────

    fn open_find(&mut self) {
        self.find_open = true;
        self.find_selected = 0;
        self.hover_find_row = None;
        self.find_rerun();
    }

    fn close_find(&mut self) {
        self.find_open = false;
        self.find_query.clear();
        self.find_results.clear();
        self.find_selected = 0;
        self.hover_find_row = None;
    }

    /// Rebuild `find_results` from the current query, scanning the parse cache
    /// plus the currently-displayed doc. Headings come before text matches.
    fn find_rerun(&mut self) {
        self.find_results.clear();
        self.find_selected = 0;
        let q = self.find_query.trim().to_lowercase();
        if q.is_empty() { return; }

        let mut headings: Vec<search::Hit> = Vec::new();
        let mut texts:    Vec<search::Hit> = Vec::new();

        // Search the currently-displayed doc (held in `self.parsed`).
        if let Some(blocks) = &self.parsed {
            search::search_doc(self.current, blocks, &q, &mut headings, &mut texts);
        }
        // Search every other warm-cached doc.
        {
            let cache = self.parsed_cache.lock().unwrap();
            for (idx, blocks) in cache.iter() {
                search::search_doc(*idx, blocks, &q, &mut headings, &mut texts);
            }
        }
        // Headings first, then text. Cap at MAX_RESULTS overall.
        let mut combined = headings;
        combined.append(&mut texts);
        combined.truncate(search::MAX_RESULTS);
        self.find_results = combined;
    }

    fn find_panel_rect(&self) -> (f32, f32, f32, f32) {
        let visible = self.find_results.len().min(FIND_MAX_ROWS);
        let h = FIND_INPUT_H + (visible as f32) * FIND_ROW_H + FIND_PAD * 2.0;
        let x = self.width - SCRW - FIND_RIGHT - FIND_W;
        (x, FIND_TOP, FIND_W, h)
    }

    fn find_input_rect(&self) -> (f32, f32, f32, f32) {
        let (px, py, pw, _ph) = self.find_panel_rect();
        (px + FIND_PAD, py + FIND_PAD, pw - FIND_PAD * 2.0, FIND_INPUT_H)
    }

    fn find_row_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let (px, py, pw, _ph) = self.find_panel_rect();
        let y = py + FIND_PAD + FIND_INPUT_H + (i as f32) * FIND_ROW_H;
        (px + FIND_PAD, y, pw - FIND_PAD * 2.0, FIND_ROW_H)
    }

    fn point_in_find_panel(&self, x: f32, y: f32) -> bool {
        if !self.find_open { return false; }
        let (px, py, pw, ph) = self.find_panel_rect();
        x >= px && x <= px + pw && y >= py && y <= py + ph
    }

    fn hit_find_row(&self, x: f32, y: f32) -> Option<usize> {
        if !self.find_open { return None; }
        let visible = self.find_results.len().min(FIND_MAX_ROWS);
        for i in 0..visible {
            let (rx, ry, rw, rh) = self.find_row_rect(i);
            if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                return Some(i);
            }
        }
        None
    }

    fn find_activate(&mut self) {
        let Some(hit) = self.find_results.get(self.find_selected).cloned() else { return; };
        let target_heading = hit.heading_text.clone();
        let target_file    = hit.file_idx;
        self.close_find();
        if target_file != self.current {
            // Stash current and navigate. Pending heading scroll is applied
            // after the new layout is built.
            if let Some(blocks) = self.parsed.take() {
                self.parsed_cache.lock().unwrap().insert(self.current, blocks);
            }
            self.history.push(self.current);
            self.forward.clear();
            self.current = target_file;
            self.scroll_y = 0.0;
            self.layout = None;
            self.parsed = None;
            self.pending_scroll_heading = target_heading;
            unsafe { let _ = PostMessageW(Some(self.hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0)); }
        } else if let Some(heading) = target_heading {
            // Same document: just scroll.
            if let Some(ly) = &self.layout {
                if let Some((_, y)) = ly.headings.iter().find(|(t, _)| t == &heading) {
                    self.scroll_y = (y - theme::V_PAD).clamp(0.0, self.max_scroll);
                }
            }
        }
    }

    fn thumb_rect(&self) -> (f32, f32, f32, f32) {
        let tx = self.width - SCRW;
        if self.max_scroll <= 0.0 { return (tx, 0.0, SCRW, self.height); }
        let ratio = self.height / (self.height + self.max_scroll);
        let h = (self.height * ratio).max(30.0).min(self.height);
        let y = (self.scroll_y / self.max_scroll) * (self.height - h);
        (tx, y, SCRW, h)
    }

    fn on_scrollbar(&self, x: f32) -> bool { x >= self.width - SCRW }

    fn hit_sidebar(&self, x: f32, y: f32) -> Option<usize> {
        if self.hidden() || x >= self.sw() { return None; }
        let i = ((y - 36.0) / theme::SIDEBAR_ITEM_H) as usize;
        match self.sidebar.get(i)? {
            docs::SidebarEntry::File { file_idx, .. } => Some(*file_idx),
            docs::SidebarEntry::Dir  { .. }           => None,
        }
    }

    /// Returns the raw sidebar row index under (x, y), regardless of entry type.
    fn hit_sidebar_row(&self, x: f32, y: f32) -> Option<usize> {
        if self.hidden() || x >= self.sw() { return None; }
        let i = ((y - 36.0) / theme::SIDEBAR_ITEM_H) as usize;
        if i < self.sidebar.len() { Some(i) } else { None }
    }

    fn hit_link(&self, x: f32, y: f32) -> Option<usize> {
        let ly = self.layout.as_ref()?;
        let dy = y + self.scroll_y;
        ly.hits.iter().position(|hr| x >= hr.x0 && x <= hr.x1 && dy >= hr.y0 && dy <= hr.y1)
    }

    fn hit_toolbar(&self, x: f32, y: f32) -> Option<usize> {
        let ly = self.layout.as_ref()?;
        let dy = y + self.scroll_y;
        ly.toolbar_hits.iter().position(|hr| x >= hr.x0 && x <= hr.x1 && dy >= hr.y0 && dy <= hr.y1)
    }

    /// Returns a cached brush for `hex`, creating it on first use.
    /// Uses RefCell interior mutability so callers can keep &self.
    /// COM clone (AddRef) is orders of magnitude cheaper than CreateSolidColorBrush.
    unsafe fn brush(&self, t: &ID2D1HwndRenderTarget, hex: u32) -> Result<ID2D1SolidColorBrush> {
        {
            let cache = self.brushes.borrow();
            if let Some(br) = cache.get(&hex) { return Ok(br.clone()); }
        }
        let c = color(hex);
        let br = t.CreateSolidColorBrush(std::ptr::addr_of!(c), None)?;
        self.brushes.borrow_mut().insert(hex, br.clone());
        Ok(br)
    }

    fn ensure_target(&mut self) -> Result<()> {
        if self.target.is_some() { return Ok(()); }
        unsafe {
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: self.dpi, dpiY: self.dpi,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let hw_p = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: D2D_SIZE_U {
                    width:  (self.width  * self.scale()) as u32,
                    height: (self.height * self.scale()) as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            let target = d2d().CreateHwndRenderTarget(&props, &hw_p)?;
            // Load header bitmap if a header.png exists in the docs dir.
            self.header_bitmap = self.header_path.as_deref()
                .and_then(|p| load_header_bitmap(&target, p).ok());
            self.target = Some(target);
        }
        Ok(())
    }

    fn resize_target(&mut self, w: u32, h: u32) {
        if let Some(t) = &self.target {
            let sz = D2D_SIZE_U { width: w, height: h };
            unsafe { let _ = t.Resize(std::ptr::addr_of!(sz)); }
        }
    }

    fn paint(&mut self) -> Result<()> {
        self.ensure_target()?;
        let t = match &self.target { Some(t) => t.clone(), None => return Ok(()) };
        // Pre-load any image bitmaps referenced by the current layout so that
        // draw() (which takes &self) can look them up without mutation.
        unsafe {
            if let Some(ly) = &self.layout {
                for cmd in &ly.cmds {
                    if let DrawCmd::Image { path, .. } = cmd {
                        let abs = self.docs_dir.join(path);
                        let key = abs.to_string_lossy().into_owned();
                        if !self.image_cache.contains_key(&key) {
                            if let Ok(bmp) = load_header_bitmap(&t, &abs) {
                                self.image_cache.insert(key, bmp);
                            }
                        }
                    }
                }
            }
        }
        unsafe {
            t.BeginDraw();
            self.draw(&t)?;
            match t.EndDraw(None, None) {
                Err(e) if e.code().0 == D2DERR_RECREATE_TARGET => {
                    self.target = None;
                    self.header_bitmap = None;
                    self.image_cache.clear();
                    self.brushes.borrow_mut().clear();
                }
                Err(e) => return Err(e),
                Ok(_) => {}
            }
        }
        Ok(())
    }

    unsafe fn draw(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let bg = color(theme::BG);
        t.Clear(Some(std::ptr::addr_of!(bg)));

        if self.hidden() {
            self.draw_reveal_tab(t)?;
        } else {
            // Sidebar background + items
            let br = self.brush(t, theme::SIDEBAR_BG)?;
            frect(t, 0.0, 0.0, self.sw(), self.height, &br);
            self.draw_sidebar(t)?;
            // Divider line
            let br = self.brush(t, theme::BORDER)?;
            frect(t, self.sw(), 0.0, 1.0, self.height, &br);
        }

        // Toggle button (always visible, straddles divider or sits on tab edge)
        self.draw_toggle_btn(t)?;

        // Scrollbar track
        let br = self.brush(t, theme::SCROLLBAR)?;
        frect(t, self.width - SCRW, 0.0, SCRW, self.height, &br);
        if self.max_scroll > 0.0 {
            let (tx, ty, tw, th) = self.thumb_rect();
            let br = self.brush(t, theme::SCROLLTHUMB)?;
            frect_r(t, tx + 2.0, ty + 3.0, tw - 4.0, th - 6.0, 3.0, &br);
        }

        // Content clipped
        let content_x = self.sw() + if self.hidden() { TAB_W } else { 1.0 };
        let clip = D2D_RECT_F { left: content_x, top: 0.0, right: self.width - SCRW, bottom: self.height };
        t.PushAxisAlignedClip(std::ptr::addr_of!(clip), D2D1_ANTIALIAS_MODE_ALIASED);
        self.draw_content(t)?;
        t.PopAxisAlignedClip();

        // Find panel overlays everything (drawn last)
        if self.find_open {
            self.draw_find_panel(t)?;
        }

        Ok(())
    }

    unsafe fn draw_reveal_tab(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let bg_hex = if self.hover_tab { theme::SIDEBAR_HVR } else { theme::SIDEBAR_BG };
        let br = self.brush(t, bg_hex)?;
        frect(t, 0.0, 0.0, TAB_W, self.height, &br);
        // Right-edge separator
        let br = self.brush(t, theme::BORDER)?;
        frect(t, TAB_W - 1.0, 0.0, 1.0, self.height, &br);
        Ok(())
    }

    unsafe fn draw_toggle_btn(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let (bx, by, bw, bh) = self.toggle_btn_rect();
        let bg_hex = if self.hover_toggle { theme::SIDEBAR_SEL } else { theme::SIDEBAR_BG };
        let fg_hex = if self.hover_toggle { theme::TEXT_BRIGHT } else { theme::TEXT_DIM };
        let bg_br = self.brush(t, bg_hex)?;
        frect_r(t, bx, by, bw, bh, 3.0, &bg_br);

        // Thin border around button
        let border_br = self.brush(t, theme::BORDER)?;
        let rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + bh },
            radiusX: 3.0, radiusY: 3.0,
        };
        t.DrawRoundedRectangle(std::ptr::addr_of!(rr), &border_br, 0.5, None::<&ID2D1StrokeStyle>);

        // Chevron glyph
        let ch: Vec<u16> = if self.hidden() { "›" } else { "‹" }.encode_utf16().collect();
        let fmt = get_fmt(theme::BODY_FONT, 11.0, true, false)?;
        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
        let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        let fg_br = self.brush(t, fg_hex)?;
        let gr = D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + bh };
        t.DrawText(&ch, &fmt, std::ptr::addr_of!(gr), &fg_br,
            D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        Ok(())
    }

    unsafe fn draw_sidebar(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let sw = self.sw();

        // "DOCS" header label
        let fmt_hdr = get_fmt(theme::BODY_FONT, 10.5, true, false)?;
        let br_dim  = self.brush(t, theme::TEXT_DIM)?;
        let label: Vec<u16> = "DOCS".encode_utf16().collect();
        let r = D2D_RECT_F { left: 14.0, top: 13.0, right: sw - BTN_W / 2.0 - 4.0, bottom: 29.0 };
        t.DrawText(&label, &fmt_hdr, std::ptr::addr_of!(r), &br_dim,
            D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);

        for (i, entry) in self.sidebar.iter().enumerate() {
            let y   = 36.0 + i as f32 * theme::SIDEBAR_ITEM_H;
            let hov = self.hover_sidebar == Some(i);

            match entry {
                docs::SidebarEntry::File { file_idx, depth } => {
                    let sel    = *file_idx == self.current;
                    let indent = 14.0 + *depth as f32 * theme::SIDEBAR_INDENT;

                    if sel {
                        let br = self.brush(t, theme::SIDEBAR_SEL)?;
                        frect(t, 0.0, y, sw - 1.0, theme::SIDEBAR_ITEM_H, &br);
                        let br = self.brush(t, theme::LINK)?;
                        frect(t, 0.0, y, 2.0, theme::SIDEBAR_ITEM_H, &br);
                    } else if hov {
                        let br = self.brush(t, theme::SIDEBAR_HVR)?;
                        frect(t, 0.0, y, sw - 1.0, theme::SIDEBAR_ITEM_H, &br);
                    }

                    let nc   = if sel { theme::TEXT_BRIGHT } else { theme::TEXT };
                    let fmt2 = get_fmt(theme::BODY_FONT, theme::SIDEBAR_FONT_SIZE, sel, false)?;
                    let br2  = self.brush(t, nc)?;
                    let display: Vec<u16> = pretty(&self.files[*file_idx].name).encode_utf16().collect();
                    let r2 = D2D_RECT_F { left: indent, top: y + 5.0, right: sw - 10.0, bottom: y + theme::SIDEBAR_ITEM_H };
                    t.DrawText(&display, &fmt2, std::ptr::addr_of!(r2), &br2,
                        D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
                }

                docs::SidebarEntry::Dir { name, depth } => {
                    let indent = 14.0 + *depth as f32 * theme::SIDEBAR_INDENT;

                    // Subtle separator above the dir header (except at the very top)
                    if y > 36.0 {
                        let br_sep = self.brush(t, theme::BORDER)?;
                        frect(t, indent, y, sw - indent - 10.0, 1.0, &br_sep);
                    }

                    // "▸ Name" in small caps / dim colour
                    let fmt_dir = get_fmt(theme::BODY_FONT, theme::SIDEBAR_FONT_SIZE - 1.5, true, false)?;
                    let br_dir  = self.brush(t, theme::SIDEBAR_DIR)?;
                    let display: Vec<u16> = format!("▸ {}", pretty(name)).encode_utf16().collect();
                    let r2 = D2D_RECT_F { left: indent, top: y + 6.0, right: sw - 10.0, bottom: y + theme::SIDEBAR_ITEM_H };
                    t.DrawText(&display, &fmt_dir, std::ptr::addr_of!(r2), &br_dir,
                        D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
                }
            }
        }
        Ok(())
    }

    unsafe fn draw_content(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let content_x = self.sw() + if self.hidden() { TAB_W } else { 1.0 } + theme::H_PAD;
        let content_right = self.width - SCRW - theme::H_PAD;

        if self.layout.is_none() {
            let fmt = get_fmt(theme::BODY_FONT, theme::BODY_SIZE, false, true)?;
            let br  = self.brush(t, theme::TEXT_DIM)?;
            let s: Vec<u16> = "Loading…".encode_utf16().collect();
            let r = D2D_RECT_F { left: content_x, top: 40.0, right: content_right, bottom: 80.0 };
            t.DrawText(&s, &fmt, std::ptr::addr_of!(r), &br,
                D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
            return Ok(());
        }
        let ly = self.layout.as_ref().unwrap();

        let oy = -self.scroll_y;

        // Draw header image (scrolls with content)
        if let Some(bmp) = &self.header_bitmap {
            let hh = self.header_display_h;
            let ry = oy;
            if ry + hh > 0.0 && ry < self.height {
                let dest = D2D_RECT_F {
                    left:   content_x,
                    top:    ry,
                    right:  self.width - SCRW,
                    bottom: ry + hh,
                };
                t.DrawBitmap(bmp, Some(std::ptr::addr_of!(dest)), 1.0,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
            }
        }

        for cmd in &ly.cmds {
            match cmd {
                DrawCmd::FillRect { x, y, w, h, color: c } => {
                    let ry = y + oy;
                    if ry + h < 0.0 || ry > self.height { continue; }
                    let br = self.brush(t, *c)?;
                    frect(t, *x, ry, *w, *h, &br);
                }
                DrawCmd::StrokeLine { x0, y0, x1, y1, color: c } => {
                    let ry0 = y0 + oy; let ry1 = y1 + oy;
                    if ry0 > self.height || ry1 < 0.0 { continue; }
                    let br = self.brush(t, *c)?;
                    t.DrawLine(
                        Vector2 { X: *x0, Y: ry0 },
                        Vector2 { X: *x1, Y: ry1 },
                        &br, 1.0, None::<&ID2D1StrokeStyle>,
                    );
                }
                DrawCmd::Image { x, y, w, h, path } => {
                    let ry = y + oy;
                    if ry + h < 0.0 || ry > self.height { continue; }
                    let abs = self.docs_dir.join(path);
                    let key = abs.to_string_lossy().into_owned();
                    if let Some(bmp) = self.image_cache.get(&key) {
                        let dest = D2D_RECT_F { left: *x, top: ry, right: x + w, bottom: ry + h };
                        t.DrawBitmap(bmp, Some(std::ptr::addr_of!(dest)), 1.0,
                            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
                    }
                }
                DrawCmd::Mermaid { x, y, scale, graph } => {
                    let ry = y + oy;
                    let h  = graph.height() * scale;
                    if ry + h < 0.0 || ry > self.height { continue; }
                    // Hand the on-screen origin to the mermaid renderer and let it
                    // walk the IR. Brushes + text formats come from the existing
                    // caches via thin closures.
                    let res = mermaid::render::draw_graph(
                        t, d2d(), graph, *x, ry, *scale,
                        |hex| self.brush(t, hex),
                        |font, size, bold, italic| get_fmt(font, size, bold, italic),
                    );
                    let _ = res;
                }
                DrawCmd::Text { x, y, max_w, text, font, size, bold, italic, color: c, underline } => {
                    let ry = y + oy;
                    let lh = size * theme::LINE_EXTRA;
                    if ry + lh < 0.0 || ry > self.height { continue; }
                    let fmt = get_fmt(font, *size, *bold, *italic)?;
                    let br  = self.brush(t, *c)?;
                    let r = D2D_RECT_F { left: *x, top: ry, right: x + max_w, bottom: ry + lh * 200.0 };
                    t.DrawText(text, &fmt, std::ptr::addr_of!(r), &br,
                        D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                    if *underline {
                        let uy = ry + size * 1.18;
                        let uw = max_w.min(text.len() as f32 * size * 0.52);
                        t.DrawLine(
                            Vector2 { X: *x,       Y: uy },
                            Vector2 { X: x + uw,   Y: uy },
                            &br, 0.8, None::<&ID2D1StrokeStyle>,
                        );
                    }
                }
            }
        }

        // Hovered link highlight
        if let Some(hi) = self.hover_link {
            let hr_data = self.layout.as_ref().and_then(|ly| ly.hits.get(hi)).cloned();
            if let Some(hr) = hr_data {
                let ry0 = hr.y0 + oy; let ry1 = hr.y1 + oy;
                let br = self.brush(t, theme::LINK_HVR)?;
                let r = D2D_RECT_F { left: hr.x0, top: ry0, right: hr.x1, bottom: ry1 };
                t.FillRectangle(std::ptr::addr_of!(r), &br);
            }
        }

        // Hovered toolbar item highlight: thin outline so the icon stays visible.
        if let Some(ti) = self.hover_toolbar {
            let hr_data = self.layout.as_ref().and_then(|ly| ly.toolbar_hits.get(ti)).cloned();
            if let Some(hr) = hr_data {
                let ry0 = hr.y0 + oy; let ry1 = hr.y1 + oy;
                let br = self.brush(t, theme::LINK_HVR)?;
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left:   hr.x0 + 2.0,
                        top:    ry0   + 2.0,
                        right:  hr.x1 - 2.0,
                        bottom: ry1   - 2.0,
                    },
                    radiusX: 3.0, radiusY: 3.0,
                };
                t.DrawRoundedRectangle(std::ptr::addr_of!(rr), &br, 1.5, None::<&ID2D1StrokeStyle>);
            }
        }

        Ok(())
    }

    unsafe fn draw_find_panel(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let (px, py, pw, ph) = self.find_panel_rect();

        // Panel background + border
        let br_bg = self.brush(t, theme::SIDEBAR_BG)?;
        frect_r(t, px, py, pw, ph, 4.0, &br_bg);
        let br_border = self.brush(t, theme::BORDER)?;
        let rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F { left: px, top: py, right: px + pw, bottom: py + ph },
            radiusX: 4.0, radiusY: 4.0,
        };
        t.DrawRoundedRectangle(std::ptr::addr_of!(rr), &br_border, 1.0, None::<&ID2D1StrokeStyle>);

        // ── Input field ────────────────────────────────────────────────────
        let (ix, iy, iw, ih) = self.find_input_rect();
        let br_ibg = self.brush(t, theme::BG)?;
        frect_r(t, ix, iy, iw, ih, 3.0, &br_ibg);
        let irr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F { left: ix, top: iy, right: ix + iw, bottom: iy + ih },
            radiusX: 3.0, radiusY: 3.0,
        };
        t.DrawRoundedRectangle(std::ptr::addr_of!(irr), &br_border, 1.0, None::<&ID2D1StrokeStyle>);

        let text_inset = 10.0;
        let text_rect = D2D_RECT_F {
            left: ix + text_inset, top: iy,
            right: ix + iw - text_inset, bottom: iy + ih,
        };
        let fmt_in = get_fmt(theme::BODY_FONT, theme::BODY_SIZE, false, false)?;
        let _ = fmt_in.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
        let _ = fmt_in.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

        if self.find_query.is_empty() {
            let br_dim = self.brush(t, theme::TEXT_DIM)?;
            let placeholder: Vec<u16> = "Search headings and text\u{2026}".encode_utf16().collect();
            t.DrawText(&placeholder, &fmt_in, std::ptr::addr_of!(text_rect), &br_dim,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
        } else {
            let br_t = self.brush(t, theme::TEXT)?;
            let text_utf: Vec<u16> = self.find_query.encode_utf16().collect();
            t.DrawText(&text_utf, &fmt_in, std::ptr::addr_of!(text_rect), &br_t,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
            // Caret (static, not blinking)
            let w = measure_text(&self.find_query, theme::BODY_FONT, theme::BODY_SIZE, false, false);
            let cursor_x = (ix + text_inset + w + 1.0).min(ix + iw - 4.0);
            let cy = iy + 7.0;
            let ch = ih - 14.0;
            frect(t, cursor_x, cy, 1.5, ch, &br_t);
        }

        // ── Empty-results message ──────────────────────────────────────────
        if !self.find_query.trim().is_empty() && self.find_results.is_empty() {
            let (rx, ry, rw, rh) = self.find_row_rect(0);
            let r = D2D_RECT_F { left: rx + 10.0, top: ry, right: rx + rw - 10.0, bottom: ry + rh };
            let fmt = get_fmt(theme::BODY_FONT, 13.0, false, true)?;
            let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let br_d = self.brush(t, theme::TEXT_DIM)?;
            let s: Vec<u16> = "No results".encode_utf16().collect();
            t.DrawText(&s, &fmt, std::ptr::addr_of!(r), &br_d,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
        }

        // ── Result rows ────────────────────────────────────────────────────
        let visible = self.find_results.len().min(FIND_MAX_ROWS);
        for i in 0..visible {
            let hit = &self.find_results[i];
            let (rx, ry, rw, rh) = self.find_row_rect(i);
            let sel = i == self.find_selected;
            let hov = self.hover_find_row == Some(i);

            // Row background
            if sel || hov {
                let bg = if sel { theme::SIDEBAR_SEL } else { theme::SIDEBAR_HVR };
                let br_rbg = self.brush(t, bg)?;
                frect_r(t, rx, ry, rw, rh, 3.0, &br_rbg);
            }

            // Badge (left)
            let (badge_text, badge_color) = match hit.kind {
                search::HitKind::Heading { level } => {
                    let lab = match level {
                        1 => "H1", 2 => "H2", 3 => "H3",
                        4 => "H4", 5 => "H5", _ => "H6",
                    };
                    let c = match level {
                        1 => theme::H1, 2 => theme::H2, 3 => theme::H3,
                        4 => theme::H4, 5 => theme::H5, _ => theme::H6,
                    };
                    (lab, c)
                }
                search::HitKind::Text => ("\u{00B7}", theme::TEXT_DIM),
            };
            let fmt_b = get_fmt(theme::BODY_FONT, 11.0, true, false)?;
            let _ = fmt_b.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = fmt_b.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let br_b = self.brush(t, badge_color)?;
            let bd_utf: Vec<u16> = badge_text.encode_utf16().collect();
            let bd_rect = D2D_RECT_F { left: rx + 8.0, top: ry, right: rx + 32.0, bottom: ry + rh };
            t.DrawText(&bd_utf, &fmt_b, std::ptr::addr_of!(bd_rect), &br_b,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);

            // Filename (right, dim)
            let file_name = self.files.get(hit.file_idx).map(|f| pretty(&f.name)).unwrap_or_default();
            let file_w = measure_text(&file_name, theme::BODY_FONT, 11.5, false, false) + 4.0;
            let fmt_f = get_fmt(theme::BODY_FONT, 11.5, false, false)?;
            let _ = fmt_f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING);
            let _ = fmt_f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let br_f = self.brush(t, theme::TEXT_DIM)?;
            let fn_rect = D2D_RECT_F {
                left: rx + rw - 8.0 - file_w, top: ry,
                right: rx + rw - 8.0, bottom: ry + rh,
            };
            let fn_utf: Vec<u16> = file_name.encode_utf16().collect();
            t.DrawText(&fn_utf, &fmt_f, std::ptr::addr_of!(fn_rect), &br_f,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);

            // Label (middle) — heading text or text snippet
            let label_fg = if sel || hov { theme::TEXT_BRIGHT } else { theme::TEXT };
            let fmt_l = get_fmt(theme::BODY_FONT, 13.0, false, false)?;
            let _ = fmt_l.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = fmt_l.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let br_l = self.brush(t, label_fg)?;
            let lab_utf: Vec<u16> = hit.label.encode_utf16().collect();
            let lab_rect = D2D_RECT_F {
                left: rx + 34.0, top: ry,
                right: rx + rw - 12.0 - file_w, bottom: ry + rh,
            };
            t.DrawText(&lab_utf, &fmt_l, std::ptr::addr_of!(lab_rect), &br_l,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
        }

        Ok(())
    }
}

// ── Text measurement ───────────────────────────────────────────────────────

/// Measures the advance width of `text` rendered at the given font/size/style.
/// Results are cached per thread so each unique (text, font, size, style) tuple
/// is measured at most once per session.
pub fn measure_text(text: &str, font: &str, size: f32, bold: bool, italic: bool) -> f32 {
    if text.is_empty() { return 0.0; }
    let key = MeasureKey {
        text: text.to_owned(), family: font.to_owned(),
        size_q: (size * 64.0) as u32, bold, italic,
    };
    MEASURE_CACHE.with(|c| {
        {
            let cache = c.borrow();
            if let Some(&w) = cache.get(&key) { return w; }
        }
        let w = unsafe {
            let Ok(fmt) = get_fmt(font, size, bold, italic) else {
                return text.chars().count() as f32 * size * 0.52;
            };
            let tw: Vec<u16> = text.encode_utf16().collect();
            let Ok(layout) = dw().CreateTextLayout(&tw, &fmt, f32::MAX, size * 4.0) else {
                return text.chars().count() as f32 * size * 0.52;
            };
            let mut m = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut m).is_ok() { m.widthIncludingTrailingWhitespace }
            else { text.chars().count() as f32 * size * 0.52 }
        };
        c.borrow_mut().insert(key, w);
        w
    })
}

// ── Win32 entry ────────────────────────────────────────────────────────────

pub fn run(docs_dir: &Path) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        init_gfx().expect("D2D/DWrite init failed");

        let class = w!("DocCrate");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            lpszClassName: class,
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW, class, w!("DocCrate"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, 1100, 720,
            None, None, None, None,
        ).expect("CreateWindowExW failed");

        let mut app = Box::new(App::new(hwnd, docs_dir));
        app.dpi = GetDpiForWindow(hwnd) as f32;
        // Snapshot the data needed by the warm-up thread before App is moved
        // into the window's user-data slot.
        let cache_handle = app.parsed_cache.clone();
        let file_paths:   Vec<PathBuf> = app.files.iter().map(|f| f.path.clone()).collect();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app) as isize);
        let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        // Pre-warm the parse cache for all non-current docs on a background
        // thread. The UI thread parses doc 0 itself (current), so we skip it.
        // If the user navigates to a doc the worker hasn't reached, the UI
        // thread falls back to a synchronous parse — no coordination needed.
        std::thread::spawn(move || {
            for (idx, path) in file_paths.iter().enumerate() {
                if idx == 0 { continue; }
                if cache_handle.lock().unwrap().contains_key(&idx) { continue; }
                let md = docs::load(path);
                let blocks = parser::parse(&md);
                cache_handle.lock().unwrap().insert(idx, blocks);
            }
        });

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe fn init_gfx() -> Result<()> {
    let d2d: ID2D1Factory1  = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
    let dw:  IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
    let _ = G_D2D.set(d2d);
    let _ = G_DW.set(dw);
    Ok(())
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;

    match msg {
        WM_DESTROY => {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        WM_SIZE => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wp, lp); }
            let a = &mut *ptr;
            let pw = lo_word(lp) as f32;
            let ph = hi_word(lp) as f32;
            a.width  = pw / a.scale();
            a.height = ph / a.scale();
            a.resize_target(pw as u32, ph as u32);
            a.relayout();
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }

        WM_RELAYOUT => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            a.relayout();
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }

        WM_PAINT => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wp, lp); }
            let a = &mut *ptr;
            let mut ps = PAINTSTRUCT::default();
            let _dc = BeginPaint(hwnd, &mut ps);
            let _ = a.paint();
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_ERASEBKGND => LRESULT(1),

        WM_MOUSEWHEEL => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let delta = (wp.0 >> 16) as i16 as f32;
            a.scroll_y = (a.scroll_y - delta * 0.5).clamp(0.0, a.max_scroll);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let x = lo_word(lp) as i16 as f32 / a.scale();
            let y = hi_word(lp) as i16 as f32 / a.scale();

            // ── dragging scroll bar ────────────────────────────────────────
            if a.dragging_scroll {
                let dy = y - a.drag_start_y;
                let (_, _, _, th) = a.thumb_rect();
                let track = a.height - th;
                if track > 0.0 {
                    a.scroll_y = (a.drag_start_scroll + dy / track * a.max_scroll).clamp(0.0, a.max_scroll);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }

            // ── dragging divider ───────────────────────────────────────────
            if a.dragging_div {
                let new_w = (a.div_start_w + (x - a.div_start_x))
                    .clamp(SB_MIN, a.width * 0.65);
                a.sidebar_w = new_w;
                a.layout = None;
                let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
                return LRESULT(0);
            }

            // ── hover updates ──────────────────────────────────────────────
            let prev_div    = a.hover_div;
            let prev_toggle = a.hover_toggle;
            let prev_tab    = a.hover_tab;
            let prev_sb     = a.hover_sidebar;
            let prev_lk     = a.hover_link;
            let prev_tb     = a.hover_toolbar;
            let prev_fr     = a.hover_find_row;

            a.hover_toggle  = a.on_toggle_btn(x, y);
            a.hover_div     = a.on_divider(x) && !a.hover_toggle;
            a.hover_tab     = a.on_tab(x) && !a.hover_toggle;
            a.hover_sidebar = a.hit_sidebar_row(x, y);
            a.hover_toolbar = a.hit_toolbar(x, y);
            a.hover_link    = a.hit_link(x, y);
            a.hover_find_row = a.hit_find_row(x, y);

            // cursor: hand only over clickable sidebar files, not dir headers
            let sb_is_file = a.hover_sidebar
                .and_then(|i| a.sidebar.get(i))
                .map_or(false, |e| matches!(e, docs::SidebarEntry::File { .. }));
            let in_find_input = a.find_open && a.point_in_find_panel(x, y) && a.hover_find_row.is_none();
            let cursor = if a.hover_div {
                LoadCursorW(None, IDC_SIZEWE).unwrap()
            } else if in_find_input {
                LoadCursorW(None, IDC_IBEAM).unwrap()
            } else if a.hover_find_row.is_some() || a.hover_link.is_some() || a.hover_toolbar.is_some()
                    || a.hover_toggle || a.hover_tab || sb_is_file {
                LoadCursorW(None, IDC_HAND).unwrap()
            } else {
                LoadCursorW(None, IDC_ARROW).unwrap()
            };
            SetCursor(Some(cursor));

            if a.hover_div != prev_div || a.hover_toggle != prev_toggle
                || a.hover_tab != prev_tab || a.hover_sidebar != prev_sb
                || a.hover_link != prev_lk || a.hover_toolbar != prev_tb
                || a.hover_find_row != prev_fr
            {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let x = lo_word(lp) as i16 as f32 / a.scale();
            let y = hi_word(lp) as i16 as f32 / a.scale();
            let _ = SetCapture(hwnd);

            // Find panel interception
            if a.find_open {
                if let Some(i) = a.hit_find_row(x, y) {
                    a.find_selected = i;
                    a.find_activate();
                    let _ = ReleaseCapture();
                    let _ = InvalidateRect(Some(hwnd), None, false);
                    return LRESULT(0);
                }
                if !a.point_in_find_panel(x, y) {
                    // Click outside the panel closes find and falls through to
                    // normal click handling on the underlying UI.
                    a.close_find();
                    let _ = InvalidateRect(Some(hwnd), None, false);
                } else {
                    // Click landed inside the panel (input field, padding):
                    // consume but don't change state.
                    let _ = ReleaseCapture();
                    return LRESULT(0);
                }
            }

            // Toggle button (check before divider — it overlaps the divider zone)
            if a.on_toggle_btn(x, y) {
                let _ = ReleaseCapture();
                a.toggle_sidebar();
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }
            // Divider drag
            if a.on_divider(x) {
                a.dragging_div = true;
                a.div_start_x  = x;
                a.div_start_w  = a.sidebar_w;
                return LRESULT(0);
            }
            // Hidden reveal tab
            if a.on_tab(x) {
                let _ = ReleaseCapture();
                a.toggle_sidebar();
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }
            // Scroll bar
            if a.on_scrollbar(x) {
                a.dragging_scroll  = true;
                a.drag_start_y     = y;
                a.drag_start_scroll = a.scroll_y;
                return LRESULT(0);
            }
            // Sidebar item
            if let Some(idx) = a.hit_sidebar(x, y) {
                let _ = ReleaseCapture();
                a.navigate(idx);
                return LRESULT(0);
            }
            // Toolbar item
            if let Some(ti) = a.hit_toolbar(x, y) {
                let _ = ReleaseCapture();
                if let Some(ly) = &a.layout {
                    if let Some(hr) = ly.toolbar_hits.get(ti) {
                        let href = hr.href.clone();
                        a.nav_href(&href);
                    }
                }
                return LRESULT(0);
            }
            // Content link
            let _ = ReleaseCapture();
            if let Some(hi) = a.hit_link(x, y) {
                if let Some(ly) = &a.layout {
                    if let Some(hr) = ly.hits.get(hi) {
                        let href = hr.href.clone();
                        a.nav_href(&href);
                    }
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONUP => {
            if !ptr.is_null() {
                let a = &mut *ptr;
                if a.dragging_scroll { a.dragging_scroll = false; }
                if a.dragging_div    { a.dragging_div    = false; }
                let _ = ReleaseCapture();
            }
            LRESULT(0)
        }

        WM_CHAR => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            if !a.find_open { return LRESULT(0); }
            if let Some(c) = char::from_u32(wp.0 as u32) {
                if !c.is_control() {
                    a.find_query.push(c);
                    a.find_rerun();
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
            }
            LRESULT(0)
        }

        WM_KEYDOWN => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let vk = VIRTUAL_KEY(wp.0 as u16);
            let ctrl = GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;
            let line = theme::BODY_SIZE * theme::LINE_EXTRA;
            let page = a.height * 0.85;

            // Ctrl+F toggles the find panel (works whether open or closed).
            if vk == VK_F && ctrl {
                if a.find_open { a.close_find(); } else { a.open_find(); }
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }

            // While find is open, capture keys to drive the panel instead of
            // letting them reach scroll/sidebar shortcuts.
            if a.find_open {
                match vk {
                    VK_ESCAPE => a.close_find(),
                    VK_DOWN => {
                        if !a.find_results.is_empty() {
                            let max = a.find_results.len().min(FIND_MAX_ROWS) - 1;
                            a.find_selected = (a.find_selected + 1).min(max);
                        }
                    }
                    VK_UP => { a.find_selected = a.find_selected.saturating_sub(1); }
                    VK_RETURN => { a.find_activate(); }
                    VK_BACK   => { a.find_query.pop(); a.find_rerun(); }
                    _ => {}
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }

            match vk {
                VK_B if ctrl => {
                    a.toggle_sidebar();
                }
                VK_DOWN  => a.scroll_y = (a.scroll_y + line).min(a.max_scroll),
                VK_UP    => a.scroll_y = (a.scroll_y - line).max(0.0),
                VK_NEXT  => a.scroll_y = (a.scroll_y + page).min(a.max_scroll),
                VK_PRIOR => a.scroll_y = (a.scroll_y - page).max(0.0),
                VK_HOME  => a.scroll_y = 0.0,
                VK_END   => a.scroll_y = a.max_scroll,
                VK_LEFT if !ctrl => {
                    if let Some(prev) = a.history.pop() {
                        a.forward.push(a.current);
                        a.current = prev;
                        a.scroll_y = 0.0;
                        a.layout = None;
                        a.parsed = None;
                        let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
                    }
                }
                VK_RIGHT if !ctrl => {
                    if let Some(next) = a.forward.pop() {
                        a.history.push(a.current);
                        a.current = next;
                        a.scroll_y = 0.0;
                        a.layout = None;
                        a.parsed = None;
                        let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
                    }
                }
                _ => {}
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }

        WM_GETMINMAXINFO => {
            let info = &mut *(lp.0 as *mut MINMAXINFO);
            info.ptMinTrackSize.x = 320;
            info.ptMinTrackSize.y = 240;
            LRESULT(0)
        }

        WM_DPICHANGED => {
            if !ptr.is_null() {
                let a = &mut *ptr;
                a.dpi = (wp.0 & 0xFFFF) as f32; // lo-word of wParam is new DPI
                a.target = None;
                a.header_bitmap = None;
                a.image_cache.clear(); // bitmaps are tied to the render target
                a.brushes.borrow_mut().clear();
            }
            let r = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(hwnd, None,
                r.left, r.top, r.right - r.left, r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ── Header image loader ────────────────────────────────────────────────────

/// Load a PNG from disk into a Direct2D bitmap using WIC.
unsafe fn load_header_bitmap(target: &ID2D1HwndRenderTarget, path: &std::path::Path)
    -> Result<ID2D1Bitmap>
{
    let factory: IWICImagingFactory = CoCreateInstance(
        &CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

    let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    // GENERIC_READ = 0x80000000
    let decoder = factory.CreateDecoderFromFilename(
        PCWSTR(wide.as_ptr()),
        None,
        windows::Win32::Foundation::GENERIC_ACCESS_RIGHTS(0x8000_0000u32),
        WICDecodeMetadataCacheOnLoad,
    )?;

    let frame = decoder.GetFrame(0)?;

    // Convert to 32bppPBGRA (pre-multiplied BGRA) — what D2D expects.
    let converter: IWICFormatConverter = factory.CreateFormatConverter()?;
    converter.Initialize(
        &frame,
        &GUID_WICPixelFormat32bppPBGRA,
        WICBitmapDitherTypeNone,
        None,
        0.0,
        WICBitmapPaletteTypeMedianCut,
    )?;

    let bitmap = target.CreateBitmapFromWicBitmap(&converter, None)?;
    Ok(bitmap)
}

// ── D2D helpers ────────────────────────────────────────────────────────────

fn color(hex: u32) -> D2D1_COLOR_F { theme::hex(hex) }

unsafe fn frect(t: &ID2D1HwndRenderTarget, x: f32, y: f32, w: f32, h: f32, br: &ID2D1SolidColorBrush) {
    let r = D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h };
    t.FillRectangle(std::ptr::addr_of!(r), br);
}

unsafe fn frect_r(t: &ID2D1HwndRenderTarget, x: f32, y: f32, w: f32, h: f32, r: f32, br: &ID2D1SolidColorBrush) {
    let rr = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h },
        radiusX: r, radiusY: r,
    };
    t.FillRoundedRectangle(std::ptr::addr_of!(rr), br);
}

fn lo_word(lp: LPARAM) -> u32 { (lp.0 & 0xFFFF) as u32 }
fn hi_word(lp: LPARAM) -> u32 { ((lp.0 >> 16) & 0xFFFF) as u32 }

fn pretty(name: &str) -> String {
    name.replace(['-', '_'], " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
