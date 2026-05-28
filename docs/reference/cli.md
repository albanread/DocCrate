# Command-Line Reference

## Synopsis

```
doc-crate.exe [DOCS_DIR]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `DOCS_DIR` | No | Path to the folder containing `.md` files. Defaults to a `docs/` directory next to the executable. |

## Examples

```
# Use the default docs/ folder
doc-crate.exe

# Open a specific project's docs
doc-crate.exe C:\projects\myapp\docs

# Works with relative paths too (resolved from CWD)
doc-crate.exe ..\docs
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Normal exit (window closed) |
| `1` | Fatal error (D2D init failed, window creation failed) |

## Environment

DocCrate reads no environment variables. DPI awareness is set programmatically
via `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`.

---

See also: [Theming](theming.md) | [Installation](../guides/installation.md) | [Home](../index.md)
