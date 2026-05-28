# Architecture

DocCrate is approximately **1,600 lines of Rust** spread across five modules.
This document describes each module, the data flow between them, and key design decisions.

---

## Module Overview

| Module | File | Responsibility |
|--------|------|---------------|
| `main` | `src/main.rs` | DPI setup, docs-dir resolution, calls `render::run` |
| `render` | `src/render.rs` | Win32 window, Direct2D rendering, all input handling |
| `parser` | `src/parser.rs` | Markdown → `Vec<Block>` AST using pulldown-cmark |
| `layout` | `src/layout.rs` | `Vec<Block>` → `Vec<DrawCmd>` + click `HitRegion`s |
| `theme` | `src/theme.rs` | All colours, fonts, and sizing constants |
| `docs` | `src/docs.rs` | File scanning, loading, relative-link resolution |

---

## Data Flow

Every time a document is opened, data moves through this pipeline:

```
docs::scan(dir)         Vec<DocFile>        All .md files, sorted
       │
docs::load(path)        String              Raw Markdown UTF-8 text
       │
parser::parse(md)       Vec<Block>          Abstract Syntax Tree
       │
layout::layout(blocks)  Layout              Draw commands + hit regions
       │
render::draw(layout)    Direct2D calls      Pixels on screen
```

The `Layout` struct is cached in `App::layout` and only recomputed when:
- A new document is selected
- The window is resized (content width changes)
- The sidebar is toggled or resized (content left edge changes)

---

## `docs.rs` — File Scanning and Loading

### `scan(dir: &Path) -> Vec<DocFile>`

Reads all `.md` files from the given directory (non-recursive). Each file becomes a
`DocFile { name: String, path: PathBuf }` where `name` is the file stem.

Sorting rules:
1. `index` and `readme` (case-insensitive) sort first
2. Everything else sorts alphabetically, case-insensitive

### `load(path: &Path) -> String`

A simple `fs::read_to_string` call. On failure, returns an error document:

```rust
fs::read_to_string(path)
    .unwrap_or_else(|e| format!("# Error\n\nCould not load `{}`: {}", path.display(), e))
```

### `resolve_href(href, current, docs_dir) -> Option<PathBuf>`

Resolves a link target to an absolute path.

1. `http://` / `https://` → returns `None` (caller opens in browser)
2. Strip any `#fragment` suffix before touching the filesystem
3. Try `base.join(href)` — works if the link already has `.md`
4. Try `base.join(href + ".md")` — works for extension-less links
5. Returns `None` if neither path exists

> **Why strip fragments here?** A link like `[details](api.md#params)` is valid
> CommonMark. Without fragment stripping the filesystem lookup for `"api.md#params"`
> would always fail, making the link silently dead.

---

## `parser.rs` — Markdown to AST

DocCrate uses [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) as the
Markdown parser with `ENABLE_STRIKETHROUGH` and `ENABLE_TABLES` options active.

### Block types

```rust
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph(Vec<Inline>),
    CodeBlock { lang: String, code: String },
    Blockquote(Vec<Block>),
    BulletList(Vec<Vec<Inline>>),
    OrderedList { start: u64, items: Vec<Vec<Inline>> },
    ThematicBreak,
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
}
```

### Inline types

```rust
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Code(String),
    Link { text: String, href: String },
    SoftBreak,
    HardBreak,
}
```

### BoldItalic detection

pulldown-cmark represents `***text***` as nested `Strong → Emphasis` events.
The parser detects this by checking the inner inline list after parsing a `Strong` span:

```rust
Event::Start(Tag::Strong) => {
    *pos += 1;
    let inlines = parse_inlines(events, pos, TagEnd::Strong);
    if inlines.len() == 1 {
        if let Inline::Italic(t) = &inlines[0] {
            return Some(Inline::BoldItalic(t.clone()));
        }
    }
    Some(Inline::Bold(collect_inline_text(&inlines)))
}
```

The same check runs for `Emphasis` wrapping a `Bold` span, handling both nesting orders.

---

## `layout.rs` — AST to Draw Commands

### Output types

```rust
pub struct Layout {
    pub cmds:    Vec<DrawCmd>,
    pub hits:    Vec<HitRegion>,
    pub total_h: f32,
}
```

`DrawCmd` is a flat list of rendering primitives that `render.rs` executes in order:

```rust
pub enum DrawCmd {
    FillRect   { x, y, w, h, color: u32 },
    StrokeLine { x0, y0, x1, y1, color: u32 },
    Text {
        x, y, max_w,
        text: String,
        font: String,
        size: f32,
        bold: bool, italic: bool,
        color: u32,
        underline: bool,
    },
}
```

### Layout context

The `Ctx` struct threads state through the recursive layout functions:

| Field | Purpose |
|-------|---------|
| `x_base` | Left edge of the content area in window coordinates |
| `width` | Available content width |
| `y` | Current vertical pen position |
| `indent` | Accumulated left indent (for blockquotes and lists) |

### Mixed-inline paragraphs

When a paragraph contains spans of different styles (bold, italic, code, links),
DocCrate approximates horizontal flow using character-width estimation:

```rust
fn estimate_text_w(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.52
}
```

The factor `0.52` is calibrated for Segoe UI with typical prose. It is accurate for
ASCII/Latin text but will drift for CJK or wide glyphs. Each span is emitted as a
separate `DrawCmd::Text`; DirectWrite handles word-wrapping within each span's rect.

> **Known limitation:** because each span is a separate draw call with its own
> bounding rect, a link that wraps onto two visual lines gets a single `HitRegion`
> covering only the first-line bounding box. The second line is not clickable.

### Table layout

Tables are rendered as a fixed-width grid divided equally across columns:

1. **Header row** — `SIDEBAR_BG` background, bold `TEXT_BRIGHT` text, `BORDER` underline
2. **Body rows** — alternating `BG` / `SIDEBAR_BG` backgrounds
3. **Grid lines** — `StrokeLine` commands drawn *after* all rows (to appear on top)

---

## `render.rs` — Window and Drawing

### Process-wide singletons

`ID2D1Factory1` and `IDWriteFactory2` are created once at startup and stored in
`OnceLock` statics:

```rust
static G_D2D: OnceLock<ID2D1Factory1>   = OnceLock::new();
static G_DW:  OnceLock<IDWriteFactory2> = OnceLock::new();
```

### TextFormat cache

Creating a new `IDWriteTextFormat` for every text run is expensive.
DocCrate uses a thread-local `HashMap` keyed on `(family, size×64, bold, italic)`:

```rust
#[derive(PartialEq, Eq, Hash, Clone)]
struct FmtKey { family: String, size_q: u32, bold: bool, italic: bool }

thread_local! {
    static FMT_CACHE: RefCell<HashMap<FmtKey, IDWriteTextFormat>> =
        RefCell::new(HashMap::new());
}
```

The size is multiplied by 64 before casting to `u32` so fractional point sizes
like 13.5 remain distinct from 14.0 as cache keys.

### App state

The `App` struct lives at `GWLP_USERDATA` on the `HWND`. It is heap-allocated,
converted to a raw pointer on window creation, and freed in `WM_DESTROY`:

```rust
let app = Box::new(App::new(hwnd, docs_dir));
SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app) as isize);
```

### Rendering pipeline

Each `WM_PAINT` calls `App::paint()` → `App::draw()`:

1. **Clear** the render target with `BG`
2. **Sidebar** — background fill, file list, active-item highlight, divider line
3. **Toggle button** — chevron glyph straddling the divider (or reveal tab edge)
4. **Scrollbar track** — `SCROLLBAR` colour fill; thumb as a rounded rect
5. **Content area** — push axis-aligned clip rect, iterate `DrawCmd` list, pop clip

The content clip rect excludes the sidebar and scrollbar, so neither the sidebar
list nor the scrollbar thumb ever bleeds into the document area.

### Device loss recovery

If `ID2D1HwndRenderTarget::EndDraw` returns `D2DERR_RECREATE_TARGET`
(which happens after events like display driver reset or DPI change),
`App::target` is set to `None`. The next `WM_PAINT` re-enters `ensure_target()`
and creates a fresh render target transparently:

```rust
match t.EndDraw(None, None) {
    Err(e) if e.code().0 == D2DERR_RECREATE_TARGET => { self.target = None; }
    Err(e) => return Err(e),
    Ok(_) => {}
}
```

---

## `theme.rs` — Colours and Constants

All visual parameters are `pub const` values — no runtime configuration.
To change the look, edit this file and recompile.

### Colour palette

| Constant | Hex | Usage |
|----------|-----|-------|
| `BG` | `#1E1E1E` | Main content background |
| `SIDEBAR_BG` | `#252526` | Sidebar and scrollbar track |
| `SIDEBAR_SEL` | `#37373D` | Selected sidebar item |
| `SIDEBAR_HVR` | `#2A2D2E` | Hovered sidebar item |
| `BORDER` | `#3C3C3C` | Divider lines, table grid, H1/H2 underlines |
| `SCROLLBAR` | `#424242` | Scrollbar track background |
| `SCROLLTHUMB` | `#686868` | Scrollbar thumb |
| `TEXT` | `#D4D4D4` | Body text |
| `TEXT_DIM` | `#808080` | Sidebar header, list markers |
| `TEXT_BRIGHT` | `#FFFFFF` | Active sidebar item, table headers |
| `H1` | `#4EC9B0` | H1 heading (teal) |
| `H2` | `#9CDCFE` | H2 heading (light blue) |
| `H3` | `#DCDCAA` | H3 heading (yellow) |
| `H4` | `#C586C0` | H4 heading (purple) |
| `H5` | `#CE9178` | H5 heading (orange) |
| `H6` | `#808080` | H6 heading (dim) |
| `LINK` | `#4FC1FF` | Hyperlink text |
| `LINK_HVR` | `#87D7FF` | Hovered link highlight overlay |
| `CODE_FG` | `#CE9178` | Inline and block code text |
| `CODE_BG` | `#0D0D0D` | Code block background |
| `BLOCKQUOTE` | `#608B4E` | Blockquote left accent bar |
| `RULE` | `#3C3C3C` | Horizontal rule |

---

## Known Limitations and Planned Work

> These are documented here so contributors know what is intentionally deferred
> rather than accidentally overlooked.

1. ***No syntax highlighting*** — the `lang` field of `CodeBlock` is captured
   and stored but not yet acted upon. A future pass could tokenise common languages
   and emit multiple coloured `Text` draw commands per code line.

2. ***Approximate text width*** — `0.52 × size` per character is accurate for
   Segoe UI with Latin text. Using `IDWriteTextLayout::GetMetrics` would give
   exact per-run widths but adds cost; acceptable for the current use case.

3. ***Single-line link hit regions*** — links that wrap across visual lines get
   a hit region only on the first line.

4. ***No subdirectory scanning*** — `docs::scan` is flat. Supporting nested folders
   would require changes to the sidebar layout (tree vs. flat list).

5. ***No anchor scroll*** — `#heading` fragments are stripped before navigation
   and scroll-to-anchor is not implemented.

---

Back to [Home](index.md) | [Keyboard Shortcuts](shortcuts.md)
