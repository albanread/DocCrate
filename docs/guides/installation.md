# Installation

DocCrate is a single self-contained executable. There is no installer.

## Requirements

- **Windows 10 version 1903** or later (Direct2D and DirectWrite are inbox)
- **Cascadia Code** font for code blocks (falls back to Consolas if absent)
- No .NET, no Visual C++ redistributable, no Electron

## Build From Source

```
git clone https://github.com/your-org/doccrate
cd doccrate
cargo build --release
```

The binary appears at `target/release/doc-crate.exe` (~2 MB stripped).

### Build Dependencies

All dependencies are pure Rust or link against system DLLs:

| Crate | Purpose |
|-------|---------|
| `windows` | Bindings for Direct2D, DirectWrite, WIC, Win32 |
| `pulldown-cmark` | CommonMark parser |
| `windows-numerics` | `Vector2` / `Matrix3x2` types |

## Running

```
doc-crate.exe                        # opens docs/ next to the exe
doc-crate.exe C:\my-project\docs     # opens a specific folder
```

Double-clicking the executable works too — it auto-discovers a `docs/` sibling.

## Portable Use

Copy `doc-crate.exe` and your `docs/` folder anywhere. No registry entries,
no config files, nothing else needed.

---

See also: [Configuration](configuration.md) | [CLI Reference](../reference/cli.md) | [Home](../index.md)
