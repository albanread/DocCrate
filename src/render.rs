use std::path::{Path, PathBuf};
use std::cell::RefCell;
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
            Input::KeyboardAndMouse::*,
            Shell::ShellExecuteW,
            WindowsAndMessaging::*,
        },
    },
};
use windows_numerics::Vector2;

use crate::{docs, layout::{self, DrawCmd, Layout}, parser, theme};

const WM_RELAYOUT: u32 = WM_APP;
const D2DERR_RECREATE_TARGET: i32 = 0x8899000Cu32 as i32;

// Sidebar geometry constants
const DIV_HIT: f32 = 5.0;   // px each side of divider that counts as "on divider"
const TAB_W:   f32 = 16.0;  // reveal-tab width when sidebar is hidden
const BTN_H:   f32 = 24.0;  // toggle-button height
const BTN_W:   f32 = 16.0;  // toggle-button width
const SB_MIN:  f32 = 80.0;  // minimum sidebar width when dragging
const SCRW:    f32 = theme::SCROLLBAR_W;

// ── process-wide singletons ────────────────────────────────────────────────
use std::sync::OnceLock;
static G_D2D: OnceLock<ID2D1Factory1>   = OnceLock::new();
static G_DW:  OnceLock<IDWriteFactory2> = OnceLock::new();

fn d2d() -> &'static ID2D1Factory1   { G_D2D.get().unwrap() }
fn dw()  -> &'static IDWriteFactory2 { G_DW.get().unwrap() }

// ── per-thread format cache ────────────────────────────────────────────────
#[derive(PartialEq, Eq, Hash, Clone)]
struct FmtKey { family: String, size_q: u32, bold: bool, italic: bool }

thread_local! {
    static FMT_CACHE: RefCell<std::collections::HashMap<FmtKey, IDWriteTextFormat>> =
        RefCell::new(std::collections::HashMap::new());
}

unsafe fn get_fmt(family: &str, size: f32, bold: bool, italic: bool) -> Result<IDWriteTextFormat> {
    let key = FmtKey { family: family.to_owned(), size_q: (size * 64.0) as u32, bold, italic };
    FMT_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if let Some(f) = cache.get(&key) { return Ok(f.clone()); }
        let weight = if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_REGULAR };
        let style  = if italic { DWRITE_FONT_STYLE_ITALIC } else { DWRITE_FONT_STYLE_NORMAL };
        let fw: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
        let fmt = dw().CreateTextFormat(
            PCWSTR(fw.as_ptr()), None, weight, style,
            DWRITE_FONT_STRETCH_NORMAL, size, w!("en-us"),
        )?;
        cache.insert(key, fmt.clone());
        Ok(fmt)
    })
}

// ── app state ─────────────────────────────────────────────────────────────
struct App {
    hwnd: HWND,
    target: Option<ID2D1HwndRenderTarget>,
    docs_dir: PathBuf,
    files: Vec<docs::DocFile>,
    current: usize,
    layout: Option<Layout>,
    scroll_y: f32,
    max_scroll: f32,
    width: f32,
    height: f32,
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
}

impl App {
    fn new(hwnd: HWND, docs_dir: &Path) -> Self {
        let files = docs::scan(docs_dir);
        Self {
            hwnd,
            target: None,
            docs_dir: docs_dir.to_path_buf(),
            files,
            current: 0,
            layout: None,
            scroll_y: 0.0,
            max_scroll: 0.0,
            width: 1100.0,
            height: 720.0,
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
        }
    }

    fn sw(&self) -> f32 { self.sidebar_w }
    fn hidden(&self) -> bool { self.sidebar_w < 1.0 }

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
        let md = docs::load(&self.files[self.current].path);
        let blocks = parser::parse(&md);
        let content_left = self.sw() + (if self.hidden() { TAB_W } else { 0.0 }) + theme::H_PAD;
        let content_w = (self.width - content_left - theme::H_PAD - SCRW).max(80.0);
        let ly = layout::layout(&blocks, content_left, content_w);
        self.max_scroll = (ly.total_h - self.height + theme::V_PAD).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll);
        self.layout = Some(ly);
    }

    fn navigate(&mut self, idx: usize) {
        if idx == self.current { return; }
        self.history.push(self.current);
        self.forward.clear();
        self.current = idx;
        self.scroll_y = 0.0;
        self.layout = None;
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
        if i < self.files.len() { Some(i) } else { None }
    }

    fn hit_link(&self, x: f32, y: f32) -> Option<usize> {
        let ly = self.layout.as_ref()?;
        let dy = y + self.scroll_y;
        ly.hits.iter().position(|hr| x >= hr.x0 && x <= hr.x1 && dy >= hr.y0 && dy <= hr.y1)
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
                dpiX: 0.0, dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let hw_p = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: D2D_SIZE_U { width: self.width as u32, height: self.height as u32 },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            self.target = Some(d2d().CreateHwndRenderTarget(&props, &hw_p)?);
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
        unsafe {
            t.BeginDraw();
            self.draw(&t)?;
            match t.EndDraw(None, None) {
                Err(e) if e.code().0 == D2DERR_RECREATE_TARGET => { self.target = None; }
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
            frect(t, 0.0, 0.0, self.sw(), self.height, theme::SIDEBAR_BG)?;
            self.draw_sidebar(t)?;
            // Divider line
            frect(t, self.sw(), 0.0, 1.0, self.height, theme::BORDER)?;
        }

        // Toggle button (always visible, straddles divider or sits on tab edge)
        self.draw_toggle_btn(t)?;

        // Scrollbar
        frect(t, self.width - SCRW, 0.0, SCRW, self.height, theme::SIDEBAR_BG)?;
        if self.max_scroll > 0.0 {
            let (tx, ty, tw, th) = self.thumb_rect();
            frect_r(t, tx + 2.0, ty + 3.0, tw - 4.0, th - 6.0, 3.0, theme::SCROLLTHUMB)?;
        }

        // Content clipped
        let content_x = self.sw() + if self.hidden() { TAB_W } else { 1.0 };
        let clip = D2D_RECT_F { left: content_x, top: 0.0, right: self.width - SCRW, bottom: self.height };
        t.PushAxisAlignedClip(std::ptr::addr_of!(clip), D2D1_ANTIALIAS_MODE_ALIASED);
        self.draw_content(t)?;
        t.PopAxisAlignedClip();

        Ok(())
    }

    unsafe fn draw_reveal_tab(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let bg = if self.hover_tab { theme::SIDEBAR_HVR } else { theme::SIDEBAR_BG };
        frect(t, 0.0, 0.0, TAB_W, self.height, bg)?;
        // Right-edge separator
        frect(t, TAB_W - 1.0, 0.0, 1.0, self.height, theme::BORDER)?;
        Ok(())
    }

    unsafe fn draw_toggle_btn(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let (bx, by, bw, bh) = self.toggle_btn_rect();
        let bg = if self.hover_toggle { theme::SIDEBAR_SEL } else { theme::SIDEBAR_BG };
        let fg = if self.hover_toggle { theme::TEXT_BRIGHT } else { theme::TEXT_DIM };
        frect_r(t, bx, by, bw, bh, 3.0, bg)?;

        // Thin border around button
        let border_c = if self.hover_toggle { theme::BORDER } else { theme::BORDER };
        let br = cbr(t, border_c)?;
        let rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + bh },
            radiusX: 3.0, radiusY: 3.0,
        };
        t.DrawRoundedRectangle(std::ptr::addr_of!(rr), &br, 0.5, None::<&ID2D1StrokeStyle>);

        // Chevron glyph
        let ch: Vec<u16> = if self.hidden() { "›" } else { "‹" }.encode_utf16().collect();
        let fmt = get_fmt(theme::BODY_FONT, 11.0, true, false)?;
        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
        let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        let glyph_br = cbr(t, fg)?;
        let gr = D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + bh };
        t.DrawText(&ch, &fmt, std::ptr::addr_of!(gr), &glyph_br,
            D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        Ok(())
    }

    unsafe fn draw_sidebar(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let sw = self.sw();

        // Header
        let fmt = get_fmt(theme::BODY_FONT, 10.5, true, false)?;
        let br  = cbr(t, theme::TEXT_DIM)?;
        let label: Vec<u16> = "DOCS".encode_utf16().collect();
        let r = D2D_RECT_F { left: 14.0, top: 13.0, right: sw - BTN_W / 2.0 - 4.0, bottom: 29.0 };
        t.DrawText(&label, &fmt, std::ptr::addr_of!(r), &br,
            D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);

        // Items
        for (i, file) in self.files.iter().enumerate() {
            let y = 36.0 + i as f32 * theme::SIDEBAR_ITEM_H;
            let sel = i == self.current;
            let hov = self.hover_sidebar == Some(i);

            if sel {
                frect(t, 0.0, y, sw - 1.0, theme::SIDEBAR_ITEM_H, theme::SIDEBAR_SEL)?;
                frect(t, 0.0, y, 2.0, theme::SIDEBAR_ITEM_H, theme::LINK)?;
            } else if hov {
                frect(t, 0.0, y, sw - 1.0, theme::SIDEBAR_ITEM_H, theme::SIDEBAR_HVR)?;
            }

            let nc = if sel { theme::TEXT_BRIGHT } else { theme::TEXT };
            let fmt2 = get_fmt(theme::BODY_FONT, theme::SIDEBAR_FONT_SIZE, sel, false)?;
            let br2  = cbr(t, nc)?;
            let display: Vec<u16> = pretty(&file.name).encode_utf16().collect();
            let r2 = D2D_RECT_F { left: 14.0, top: y + 5.0, right: sw - 10.0, bottom: y + theme::SIDEBAR_ITEM_H };
            t.DrawText(&display, &fmt2, std::ptr::addr_of!(r2), &br2,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
        }
        Ok(())
    }

    unsafe fn draw_content(&self, t: &ID2D1HwndRenderTarget) -> Result<()> {
        let content_x = self.sw() + if self.hidden() { TAB_W } else { 1.0 } + theme::H_PAD;
        let content_right = self.width - SCRW - theme::H_PAD;

        let Some(ly) = &self.layout else {
            let fmt = get_fmt(theme::BODY_FONT, theme::BODY_SIZE, false, true)?;
            let br  = cbr(t, theme::TEXT_DIM)?;
            let s: Vec<u16> = "Loading…".encode_utf16().collect();
            let r = D2D_RECT_F { left: content_x, top: 40.0, right: content_right, bottom: 80.0 };
            t.DrawText(&s, &fmt, std::ptr::addr_of!(r), &br,
                D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
            return Ok(());
        };

        let oy = -self.scroll_y;

        for cmd in &ly.cmds {
            match cmd {
                DrawCmd::FillRect { x, y, w, h, color: c } => {
                    let ry = y + oy;
                    if ry + h < 0.0 || ry > self.height { continue; }
                    frect(t, *x, ry, *w, *h, *c)?;
                }
                DrawCmd::StrokeLine { x0, y0, x1, y1, color: c } => {
                    let ry0 = y0 + oy; let ry1 = y1 + oy;
                    if ry0 > self.height || ry1 < 0.0 { continue; }
                    let br = cbr(t, *c)?;
                    t.DrawLine(
                        Vector2 { X: *x0, Y: ry0 },
                        Vector2 { X: *x1, Y: ry1 },
                        &br, 1.0, None::<&ID2D1StrokeStyle>,
                    );
                }
                DrawCmd::Text { x, y, max_w, text, font, size, bold, italic, color: c, underline } => {
                    let ry = y + oy;
                    let lh = size * theme::LINE_EXTRA;
                    if ry + lh < 0.0 || ry > self.height { continue; }
                    let fmt = get_fmt(font, *size, *bold, *italic)?;
                    let br  = cbr(t, *c)?;
                    let tw: Vec<u16> = text.encode_utf16().collect();
                    let r = D2D_RECT_F { left: *x, top: ry, right: x + max_w, bottom: ry + lh * 25.0 };
                    t.DrawText(&tw, &fmt, std::ptr::addr_of!(r), &br,
                        D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                    if *underline {
                        let uy = ry + size * 1.18;
                        let uw = (*max_w).min(text.len() as f32 * size * 0.52);
                        let ubr = cbr(t, *c)?;
                        t.DrawLine(
                            Vector2 { X: *x,     Y: uy },
                            Vector2 { X: x + uw, Y: uy },
                            &ubr, 0.8, None::<&ID2D1StrokeStyle>,
                        );
                    }
                }
            }
        }

        // Hovered link highlight
        if let Some(hi) = self.hover_link {
            if let Some(hr) = ly.hits.get(hi) {
                let ry0 = hr.y0 + oy; let ry1 = hr.y1 + oy;
                let br = cbr(t, theme::LINK_HVR)?;
                let r = D2D_RECT_F { left: hr.x0, top: ry0, right: hr.x1, bottom: ry1 };
                t.FillRectangle(std::ptr::addr_of!(r), &br);
            }
        }
        Ok(())
    }
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

        let app = Box::new(App::new(hwnd, docs_dir));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app) as isize);
        let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

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
            a.width  = lo_word(lp) as f32;
            a.height = hi_word(lp) as f32;
            a.resize_target(a.width as u32, a.height as u32);
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
            let x = lo_word(lp) as i16 as f32;
            let y = hi_word(lp) as i16 as f32;

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

            a.hover_toggle  = a.on_toggle_btn(x, y);
            a.hover_div     = a.on_divider(x) && !a.hover_toggle;
            a.hover_tab     = a.on_tab(x) && !a.hover_toggle;
            a.hover_sidebar = a.hit_sidebar(x, y);
            a.hover_link    = a.hit_link(x, y);

            // cursor
            let cursor = if a.hover_div {
                LoadCursorW(None, IDC_SIZEWE).unwrap()
            } else if a.hover_link.is_some() || a.hover_toggle || a.hover_tab {
                LoadCursorW(None, IDC_HAND).unwrap()
            } else {
                LoadCursorW(None, IDC_ARROW).unwrap()
            };
            SetCursor(Some(cursor));

            if a.hover_div != prev_div || a.hover_toggle != prev_toggle
                || a.hover_tab != prev_tab || a.hover_sidebar != prev_sb
                || a.hover_link != prev_lk
            {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let x = lo_word(lp) as i16 as f32;
            let y = hi_word(lp) as i16 as f32;
            let _ = SetCapture(hwnd);

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

        WM_KEYDOWN => {
            if ptr.is_null() { return LRESULT(0); }
            let a = &mut *ptr;
            let vk = VIRTUAL_KEY(wp.0 as u16);
            let ctrl = GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;
            let line = theme::BODY_SIZE * theme::LINE_EXTRA;
            let page = a.height * 0.85;

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
                        let _ = PostMessageW(Some(hwnd), WM_RELAYOUT, WPARAM(0), LPARAM(0));
                    }
                }
                VK_RIGHT if !ctrl => {
                    if let Some(next) = a.forward.pop() {
                        a.history.push(a.current);
                        a.current = next;
                        a.scroll_y = 0.0;
                        a.layout = None;
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
            if !ptr.is_null() { (*ptr).target = None; }
            let r = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(hwnd, None,
                r.left, r.top, r.right - r.left, r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ── D2D helpers ────────────────────────────────────────────────────────────

fn color(hex: u32) -> D2D1_COLOR_F { theme::hex(hex) }

unsafe fn cbr(t: &ID2D1HwndRenderTarget, hex: u32) -> Result<ID2D1SolidColorBrush> {
    let c = color(hex);
    t.CreateSolidColorBrush(std::ptr::addr_of!(c), None)
}

unsafe fn frect(t: &ID2D1HwndRenderTarget, x: f32, y: f32, w: f32, h: f32, hex: u32) -> Result<()> {
    let br = cbr(t, hex)?;
    let r = D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h };
    t.FillRectangle(std::ptr::addr_of!(r), &br);
    Ok(())
}

unsafe fn frect_r(t: &ID2D1HwndRenderTarget, x: f32, y: f32, w: f32, h: f32, r: f32, hex: u32) -> Result<()> {
    let br = cbr(t, hex)?;
    let rr = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h },
        radiusX: r, radiusY: r,
    };
    t.FillRoundedRectangle(std::ptr::addr_of!(rr), &br);
    Ok(())
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
