# DocCrate

A fast, native Windows documentation browser for local Markdown files.

Built on Direct2D and DirectWrite — no Electron, no browser engine, no web view overhead.
Instant rendering, minimal footprint.

![VS Code Dark+ theme — sidebar on the left, rendered Markdown on the right]

---

## Features

- Renders CommonMark Markdown: headings, paragraphs, bold/italic, inline code, fenced code blocks, blockquotes, bullet and numbered lists, horizontal rules, hyperlinks
- Sidebar file browser with collapsible/draggable panel
- Click-to-navigate local `.md` links; external URLs open in the default browser
- Back / Forward navigation history
- Mouse-wheel scrolling, draggable scrollbar, full keyboard navigation
- VS Code Dark+ colour palette
- DPI-aware (Per-Monitor Aware v2) — crisp on high-DPI displays
- Single self-contained `.exe`, no installer

## Requirements

- Windows 10 or 11

## Building

Install [Rust](https://rustup.rs), then:

```
cargo build --release
```

The binary is written to `target/release/doc-crate.exe`.

## Usage

```
doc-crate.exe [docs-directory]
```

If no argument is given, DocCrate looks for a `docs/` folder next to the executable.
The directory must contain at least one `.md` file or the app will exit with an error.

### Adding your own docs

Drop `.md` files into the docs folder and relaunch. File names become sidebar labels:

| File | Sidebar label |
|------|--------------|
| `index.md` | Index *(shown first)* |
| `readme.md` | Readme *(shown first)* |
| `getting-started.md` | Getting Started |
| `api-reference.md` | Api Reference |

Subdirectories are not scanned — all `.md` files must be at the top level of the docs folder.

### Links

- **Local links** — `[text](other-file.md)` or `[text](other-file)` navigate within DocCrate
- **External links** — `http://` / `https://` URLs open in the default browser
- **Anchor links** — `#section` references are not supported and are silently ignored

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| ↓ / ↑ | Scroll one line |
| Page Down / Page Up | Scroll one screen |
| Home / End | Jump to top / bottom |
| ← Left Arrow | Go back |
| → Right Arrow | Go forward |
| Ctrl+B | Toggle sidebar |
| Mouse wheel | Scroll |

## Project Layout

```
doc-crate/
├── src/
│   ├── main.rs       Entry point — DPI setup, docs-dir resolution
│   ├── render.rs     Win32 window, Direct2D rendering, input handling
│   ├── parser.rs     Markdown → AST (wraps pulldown-cmark)
│   ├── layout.rs     AST → draw commands + link hit regions
│   ├── theme.rs      Colours, fonts, sizing constants
│   └── docs.rs       File scanning, loading, relative-link resolution
├── docs/             Built-in documentation (shown on first launch)
└── Cargo.toml
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [`pulldown-cmark`](https://github.com/raphlinus/pulldown-cmark) | CommonMark parser |
| [`windows`](https://github.com/microsoft/windows-rs) | Win32 / Direct2D / DirectWrite bindings |
| [`windows-numerics`](https://crates.io/crates/windows-numerics) | `Vector2` math for D2D calls |

## Known Limitations

- **Tables** are not rendered (Markdown table syntax displays as raw text)
- **Syntax highlighting** is not supported — code blocks render in a single colour
- **Subdirectories** in the docs folder are not scanned
- Text width is approximated; layout can drift slightly for non-ASCII-heavy content

## License

MIT
