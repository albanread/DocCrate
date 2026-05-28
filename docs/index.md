# DocCrate

| ![Home](icons/home.png) | ![Features](icons/features.png) | ![Architecture](icons/arch.png) | ![Getting Started](icons/start.png) | ![Markdown](icons/markdown.png) | ![Shortcuts](icons/keys.png) |
|---|---|---|---|---|---|
| [Home](index.md) | [Features](features.md) | [Architecture](architecture.md) | [Getting Started](getting-started.md) | [Markdown Reference](markdown-reference.md) | [Shortcuts](shortcuts.md) |

A **fast, native Windows documentation browser** for local Markdown files.
Built entirely on Direct2D and DirectWrite — no Electron, no browser engine, no WebView overhead.
Instant rendering, minimal memory footprint, single self-contained `.exe`.

---

## Why DocCrate?

Most documentation tools either require a running server, embed a full browser engine, or depend
on an internet connection. DocCrate renders your Markdown files *directly* using the same graphics
stack that powers Windows itself. The result is a viewer that starts instantly, uses almost no RAM,
and works completely offline.

> **Design philosophy:** a documentation viewer should get out of the way.
> No loading spinners, no JavaScript, no network requests — just your text, rendered crisply.

---

## Feature Highlights

- **CommonMark Markdown** — headings H1–H6, paragraphs, bold, italic, ***bold-italic***,
  inline code, fenced code blocks, blockquotes, bullet lists, ordered lists, tables,
  and horizontal rules
- **Sidebar file browser** — collapsible panel listing all `.md` files; drag the divider to resize
- **Back / Forward navigation** — full history stack, navigable with `←` / `→` arrow keys
- **Local link resolution** — `[text](other-file.md)` navigates within DocCrate;
  `#fragment` suffixes are stripped cleanly before file resolution
- **External links** — `https://` URLs open in your default browser
- **DPI-aware** — Per-Monitor Aware v2, crisp on 4K and mixed-DPI setups
- **VS Code Dark+ colour palette** — six distinct heading colours, syntax-aware code tint
- **Keyboard-first** — scroll, navigate history, toggle sidebar without the mouse

---

## Documentation Contents

| Document | What it covers |
|----------|---------------|
| [Getting Started](getting-started.md) | Building, installing, first run, adding your own docs |
| [Features](features.md) | Detailed tour of every feature |
| [Markdown Reference](markdown-reference.md) | Every supported syntax element with examples |
| [Keyboard Shortcuts](shortcuts.md) | Complete keyboard and mouse reference |
| [Architecture](architecture.md) | Internals — modules, data flow, rendering pipeline |

## Guides

Step-by-step walkthroughs for common tasks:

- [Installation](guides/installation.md) — build from source, requirements, portable use
- [Configuration](guides/configuration.md) — theme constants, font settings, layout tuning

## API & Internals

For developers extending or embedding DocCrate:

- [API Overview](api/index.md) — module map and data flow
- [Path Resolution](api/authentication.md) — how links and images are resolved safely
- [Internal Messages](api/endpoints.md) — Win32 messages and the relayout protocol

## Reference

- [Command-Line Reference](reference/cli.md) — arguments, exit codes, environment
- [Theming](reference/theming.md) — colour format, heading palette, light theme guide

---

## Quick Start

1. Install [Rust](https://rustup.rs) (stable toolchain)
2. Build the binary:

```
cargo build --release
```

3. Place your `.md` files in a `docs/` folder beside the `.exe`
4. Run `doc-crate.exe`

No configuration files, no package managers, no runtime dependencies beyond standard Windows DLLs.

---

## License

MIT — see the repository for the full license text.
Project home: [github.com/albanread/DocCrate](https://github.com/albanread/DocCrate)
