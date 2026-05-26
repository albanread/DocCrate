# DocCrate — Codebase Guide

## What Is This

DocCrate is a native Windows desktop documentation browser written in Rust (~1,500 lines).
It renders local Markdown files using Direct2D / DirectWrite — no Electron, no browser engine.
The binary is `doc-crate.exe`; it accepts an optional path to a docs folder as `argv[1]`,
otherwise falls back to a `docs/` sibling directory next to the executable.

## Build

```
cargo build --release
```

Requires Windows. The `windows` crate links against system DLLs (Direct2D, DirectWrite, etc.)
so no third-party native libraries need to be installed. The release profile uses `opt-level = 3`
and `lto = "thin"` for a compact binary.

## Module Overview

| File | Role |
|------|------|
| `src/main.rs` | Entry point — sets DPI awareness, resolves docs dir, calls `render::run` |
| `src/render.rs` | Win32 window + Direct2D rendering + all input handling |
| `src/parser.rs` | Wraps pulldown-cmark, produces a `Vec<Block>` AST |
| `src/layout.rs` | Converts AST blocks → `Vec<DrawCmd>` + click `HitRegion`s |
| `src/theme.rs` | All colours (VS Code Dark+ palette), fonts, and sizing constants |
| `src/docs.rs` | Scans docs dir for `.md` files, loads them, resolves relative hrefs |

### Data Flow

```
docs::scan()         → Vec<DocFile>
docs::load()         → String (raw Markdown)
parser::parse()      → Vec<Block>
layout::layout()     → Layout { cmds, hits, total_h }
render (draw loop)   → Direct2D calls
```

## render.rs in Detail

`App` is the single state struct held in `GWLP_USERDATA` on the HWND:

- `files` / `current` — the doc list and which one is open
- `layout` — cached `Layout`; set to `None` to trigger a `WM_RELAYOUT` re-parse
- `scroll_y` / `max_scroll` — vertical scroll position
- `sidebar_w` / `sidebar_saved` — width in DIPs; `0.0` means hidden
- `history` / `forward` — navigation stacks (indices into `files`)

Key message handlers:

| Message | What It Does |
|---------|-------------|
| `WM_RELAYOUT` (custom) | Calls `relayout()`, which re-parses and re-layouts the current doc |
| `WM_PAINT` | Calls `paint()` → `draw()` — full redraw every frame |
| `WM_SIZE` | Updates `width`/`height`, resizes render target, triggers relayout |
| `WM_MOUSEWHEEL` | Adjusts `scroll_y` |
| `WM_MOUSEMOVE` | Updates all hover states; sets cursor (arrow / hand / resize) |
| `WM_LBUTTONDOWN` | Hit-tests in priority order: toggle btn → divider → tab → scrollbar → sidebar → link |
| `WM_KEYDOWN` | Ctrl+B sidebar toggle; arrow/page/home/end scroll; ←/→ history |
| `WM_DPICHANGED` | Drops render target (recreated on next paint), repositions window |

Rendering uses a per-thread `IDWriteTextFormat` cache keyed on `(family, size×64, bold, italic)`
to avoid creating a new format object for every text run.

## parser.rs in Detail

Options enabled: `ENABLE_STRIKETHROUGH` only (tables are **not** enabled).

Block types handled: `Heading`, `Paragraph`, `CodeBlock`, `Blockquote`, `BulletList`,
`OrderedList`, `ThematicBreak`. Everything else is silently skipped.

The `Inline::BoldItalic` variant is defined but **never produced** — nested `***bold italic***`
is currently parsed as `Bold(Italic(...))` by pulldown-cmark, which the parser flattens
via `collect_inline_text`, losing the combined style.

## layout.rs in Detail

`layout()` takes `(blocks, x_base, width)` and returns:

- `cmds: Vec<DrawCmd>` — ordered draw list
- `hits: Vec<HitRegion>` — bounding boxes for clickable links
- `total_h: f32` — total content height (used to compute `max_scroll`)

Width is approximated as `char_count × size × 0.52`; this is accurate for ASCII/Latin with
proportional fonts but will drift for CJK or unusual glyphs.

Code blocks: height is computed exactly from line count (`lines × line_h + 2×pad`).
Paragraphs: height uses `estimate_lines()` and can be off by ±1 line for wrapped text.

Inline code within a paragraph gets spaces padded (`" code "`) as a visual workaround
since there is no per-span background color support in this renderer.

## theme.rs in Detail

All constants are `pub const` — no runtime configuration. To change fonts/colours, edit this
file and recompile. Notable constants:

- `SIDEBAR_W` — default sidebar width (220 DIPs)
- `H_PAD` / `V_PAD` — content area padding
- `LINE_EXTRA` — line-height multiplier (1.4×)
- `CODE_FONT` / `CODE_FONT2` — Cascadia Code with Consolas fallback

## docs.rs in Detail

`scan()` reads all `.md` files from the docs dir. Sorting: `index`/`readme` first,
rest alphabetical (case-insensitive). Subdirectories and non-`.md` files are ignored.

`resolve_href(href, current, docs_dir)`:
- `http://` / `https://` → returns `None` (caller opens in browser via `ShellExecuteW`)
- Relative path → tries `base/href` then `base/href.md`
- Anchor-only links (`#section`) are silently ignored in `nav_href`

## Known Limitations / Areas to Improve

1. **Tables not rendered** — `ENABLE_TABLES` is not set; table syntax renders as
   pipe-separated paragraph text. The bundled `shortcuts.md` uses tables and currently
   displays them as raw text.

2. **`Inline::BoldItalic` is dead code** — defined but never constructed.

3. **`border_c` variable in `draw_toggle_btn`** — both branches of the if/else assign
   the same value (`theme::BORDER`), making the condition dead code (`render.rs:330`).

4. **Approximate text width** — `0.52 × size` per character works for Segoe UI with ASCII
   but breaks for wide scripts. Could use `IDWriteTextLayout::GetMetrics` for precision.

5. **No syntax highlighting** — `lang` field in `CodeBlock` is captured but never used
   (noted with `_lang` parameter name in `layout_code`).

6. **Anchor links silently dropped** — `#section` hrefs return early with no feedback.

7. **Large text bounding rect** — `render.rs:425` uses `ry + lh * 25.0` as the bottom
   bound for text draw calls, which clips runs longer than ~25 wrapped lines.

8. **No subdirectory support** — `docs::scan` is flat; nested `docs/api/*.md` files
   are not discovered.
