# Features

A complete tour of everything DocCrate can do, illustrated with live examples as you read.

---

## Markdown Rendering

DocCrate parses CommonMark Markdown using [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)
and converts the AST into Direct2D draw commands. Every element described below is rendered
natively — no HTML, no CSS, no browser.

### Headings

All six heading levels are supported, each rendered in a distinct colour from the
VS Code Dark+ palette:

# H1 — Teal — section titles
## H2 — Light blue — major chapters
### H3 — Yellow — topics within a chapter
#### H4 — Purple — sub-topics
##### H5 — Orange — notes and asides
###### H6 — Dim grey — fine print and footnotes

H1 and H2 headings are followed by a subtle horizontal separator line.
H1–H3 are rendered **bold**; H4–H6 use regular weight.

### Inline Text Styles

Plain body text uses **Segoe UI** at 15 DIP with a line-height multiplier of 1.4×.

**Bold text** is rendered with `DWRITE_FONT_WEIGHT_BOLD`.

*Italic text* is rendered with `DWRITE_FONT_STYLE_ITALIC`.

***Bold-italic text*** combines both — DocCrate detects the nested Strong/Emphasis
span and emits the `BoldItalic` variant directly.

`Inline code` switches to **Cascadia Code** at 13.5 DIP with the orange code colour.
In mixed paragraphs each code span renders in the code font inline with body text.

---

## Code Blocks

Fenced code blocks render on a near-black background (`#0D0D0D`) with the code font
and orange foreground. The language hint after the opening fence is captured and
available for future syntax-highlighting support.

```rust
pub fn layout(blocks: &[Block], x_base: f32, width: f32) -> Layout {
    let mut ctx = Ctx::new(x_base, width, theme::V_PAD);
    layout_blocks(&mut ctx, blocks, 0);
    ctx.y += theme::V_PAD;
    Layout { cmds: ctx.cmds, hits: ctx.hits, total_h: ctx.y }
}
```

```toml
[dependencies]
pulldown-cmark = "0.12"
windows-numerics = "0.3"
```

```
Plain fenced block — no language hint.
Still uses Cascadia Code, just without a hint tag.
```

---

## Tables

Tables are rendered as a fixed-width grid. The header row has a slightly lighter
background and bold white text. Body rows alternate between the standard background
and a faintly lighter shade, separated by thin border lines.

| Feature | Status | Notes |
|---------|--------|-------|
| Headings H1–H6 | Supported | Six colours from VS Code Dark+ |
| Bold / Italic | Supported | Handled in parser and layout |
| Bold-Italic | Supported | Detected as nested Strong/Emphasis |
| Inline code | Supported | Cascadia Code font, orange tint |
| Fenced code blocks | Supported | Background box, monospace font |
| Tables | Supported | Header + alternating rows |
| Blockquotes | Supported | Green accent bar on left |
| Bullet lists | Supported | Bullet marker in dim colour |
| Ordered lists | Supported | Numeric marker, configurable start |
| Horizontal rules | Supported | 1px separator line |
| Local links | Supported | Fragment (`#anchor`) stripped first |
| External links | Supported | Opens in default browser |
| Syntax highlighting | Planned | `lang` hint captured, not yet used |
| Subdirectories | Planned | Flat scan only for now |

---

## Blockquotes

Blockquotes are indented and marked with a green accent bar on the left. They can
contain any block-level content including paragraphs, lists, and nested blockquotes.

> This is a standard blockquote. Use it for tips, warnings, and callouts.
> The green bar comes from `theme::BLOCKQUOTE` (`#608B4E`).

> **Tip:** You can put *formatted text* and `inline code` inside a blockquote.
> All inline styles work normally within the quoted block.

---

## Lists

### Bullet Lists

- First item — plain text
- Second item — **with bold**
- Third item — *with italic*
- Fourth item — with `inline code`
- Fifth item — with a [local link](index.md) to the home page
- A longer item that contains enough text to potentially wrap onto the next line when the
  content area is narrow, testing that the bullet marker stays aligned with the first line

### Ordered Lists

1. Clone the repository
2. Run `cargo build --release`
3. Copy the `.exe` and a `docs\` folder to your desired location
4. Launch `doc-crate.exe`

### Mixed Markers

Ordered lists can start at any number:

7. Item seven
8. Item eight
9. Item nine
10. Item ten — double-digit numbers

---

## Navigation

### Sidebar

The sidebar on the left lists every `.md` file in the docs directory.
`index.md` and `readme.md` are pinned first; all others sort alphabetically.

- The **currently open** file is highlighted with a blue accent strip on the left edge
- **Hovering** a file dims the row
- **Clicking** a file navigates immediately and pushes the previous page onto the history stack

#### Sidebar resize

Drag the thin vertical divider between the sidebar and content to resize the sidebar.
The minimum width is 80 px; the maximum is 65% of the window width.

##### Hiding the sidebar

Click the `‹` chevron button to collapse the sidebar to a thin reveal tab.
Click `›` or the tab strip to restore it. Press `Ctrl+B` to toggle from the keyboard.

###### State persistence

The sidebar width you drag to is saved across toggle cycles. Hiding and showing
the sidebar restores your last explicit width.

---

## Scrolling

Content longer than the window height scrolls vertically. Three input methods are supported:

- **Mouse wheel** — smooth continuous scroll
- **Scrollbar** — click-drag the thumb on the right edge; the track is always visible
- **Keyboard** — see the [Keyboard Shortcuts](shortcuts.md) page for the full list

---

## History Navigation

Every time you click a sidebar item or a local link, the previous document is pushed
onto a history stack. Use `←` Left Arrow to go back and `→` Right Arrow to go forward,
exactly like browser history. The forward stack is cleared when you navigate to a new page.

---

## DPI Awareness

DocCrate registers as **Per-Monitor Aware v2** (`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`).
When you drag the window to a monitor with a different DPI, Windows sends `WM_DPICHANGED`
and DocCrate drops and recreates its render target at the new scale. Text and UI elements
remain crisp on any combination of display resolutions.

---

Back to [Home](index.md) | [Markdown Reference](markdown-reference.md)
