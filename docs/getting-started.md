# Getting Started

This guide walks you from a fresh machine to a running DocCrate instance with your own documentation.

---

## Prerequisites

- **Windows 10** (version 1903+) or **Windows 11**
- **Rust stable toolchain** — install from [rustup.rs](https://rustup.rs)
- No other dependencies: DocCrate links against system DLLs (`d2d1.dll`, `dwrite.dll`, `shell32.dll`)
  that ship with every supported version of Windows

> If you already have Rust installed, make sure it is up to date:
> `rustup update stable`

---

## Building from Source

Clone the repository and run a release build:

```
git clone https://github.com/albanread/DocCrate
cd DocCrate
cargo build --release
```

The compiled binary lands at:

```
target\release\doc-crate.exe
```

Build time on a modern machine is roughly 20–30 seconds for a clean build
(most of that is compiling the `windows` crate). Incremental rebuilds are
under a second.

### Release profile

The `Cargo.toml` release profile is configured for maximum performance:

```toml
[profile.release]
opt-level = 3
lto = "thin"
```

`lto = "thin"` keeps link times fast while still enabling cross-crate inlining.

---

## Running DocCrate

### With a docs directory argument

Pass the path to any folder containing `.md` files:

```
doc-crate.exe C:\Users\you\projects\myapp\docs
```

### Without an argument

DocCrate looks for a `docs\` folder ***in the same directory as the executable***:

```
C:\tools\
  doc-crate.exe
  docs\
    index.md
    api-reference.md
    changelog.md
```

Launching `doc-crate.exe` with no arguments opens the `docs\` sibling automatically.

> **Tip:** For a portable setup, copy the `.exe` and your `docs\` folder together as a unit.
> The viewer is entirely self-contained — no installer, no registry entries.

---

## Adding Your Own Documentation

1. Create a folder (e.g. `docs\`) and drop in `.md` files
2. Name files descriptively — the file stem becomes the sidebar label
3. Launch DocCrate pointing at that folder

### File naming and sidebar order

| File name | Sidebar label | Position |
|-----------|--------------|----------|
| `index.md` | Index | ***First*** |
| `readme.md` | Readme | ***First*** |
| `getting-started.md` | Getting Started | Alphabetical |
| `api-reference.md` | Api Reference | Alphabetical |
| `z-appendix.md` | Z Appendix | Last |

`index.md` and `readme.md` are always pinned to the top of the sidebar regardless of
alphabetical order. All other files sort case-insensitively by name.

---

## Writing Links Between Pages

### Local links

Use a relative path — with or without the `.md` extension:

```
[API Reference](api-reference.md)
[API Reference](api-reference)
```

Both forms resolve correctly. Anchors (`#section`) are stripped before resolution,
so `[details](api-reference.md#parameters)` still navigates to `api-reference.md`.

### External links

Any `http://` or `https://` URL opens in your system default browser:

```
[Rust docs](https://doc.rust-lang.org)
```

---

## Tips for Good Docs

- Use `index.md` as your landing page with a table linking to all other sections
- Keep individual files focused — one topic per file works better than one giant file
- Fenced code blocks look best with a language hint (e.g. ` ```rust `) even though
  DocCrate does not yet apply syntax colouring — the hint is there for future use
- Headings H1–H3 carry the most visual weight; use H4–H6 sparingly for fine-grained
  sub-sections within a page

---

Back to [Home](index.md) | Next: [Features](features.md)

---

## Further Reading

- Full build options and portable setup: [Installation Guide](guides/installation.md)
- All theme constants explained: [Configuration](guides/configuration.md)
- Command-line arguments: [CLI Reference](reference/cli.md)
