# Theming

All visual parameters are compile-time constants in `src/theme.rs`.

## Colour Format

Colours are stored as `u32` in `0xRRGGBB` format and converted to
`D2D1_COLOR_F` by `theme::hex()` at draw time. Alpha is always `1.0`.

## Heading Colours

DocCrate uses the VS Code Dark+ syntax palette for headings:

| Level | Colour | Hex |
|-------|--------|-----|
| H1 | Teal | `#4EC9B0` |
| H2 | Light blue | `#9CDCFE` |
| H3 | Yellow | `#DCDCAA` |
| H4 | Purple | `#C586C0` |
| H5 | Orange | `#CE9178` |
| H6 | Dim grey | `#808080` |

## Adding a Light Theme

There is no runtime theme switching yet. To create a light theme:

1. Duplicate `src/theme.rs` or add a `#[cfg]` feature flag.
2. Swap `BG` / `SIDEBAR_BG` to near-white values.
3. Adjust text colours for contrast.
4. Recompile.

## Font Fallback

The `CODE_FONT` constant (`"Cascadia Code"`) is tried first. If DirectWrite
cannot find it, it falls back to whatever font the system maps for monospace
rendering (typically Consolas). There is no explicit second-font constant yet.

---

See also: [CLI Reference](cli.md) | [Configuration](../guides/configuration.md) | [Home](../index.md)
