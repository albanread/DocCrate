# Configuration

DocCrate has **no runtime configuration file**. All settings live in
`src/theme.rs` as `pub const` values — change them and recompile.

## Theme Constants

### Colours

| Constant | Default | Purpose |
|----------|---------|---------|
| `BG` | `#1E1E1E` | Main background |
| `SIDEBAR_BG` | `#252526` | Sidebar background |
| `SIDEBAR_SEL` | `#37373D` | Selected item highlight |
| `TEXT` | `#D4D4D4` | Body text |
| `TEXT_BRIGHT` | `#FFFFFF` | Selected / heading text |
| `LINK` | `#4FC1FF` | Hyperlink colour |
| `CODE_BG` | `#0D0D0D` | Code block background |
| `CODE_FG` | `#CE9178` | Code block text |

### Typography

| Constant | Default | Purpose |
|----------|---------|---------|
| `BODY_FONT` | `"Segoe UI"` | Prose font family |
| `CODE_FONT` | `"Cascadia Code"` | Code block font |
| `BODY_SIZE` | `15.0` | Body text size (DIPs) |
| `LINE_EXTRA` | `1.4` | Line-height multiplier |

### Layout

| Constant | Default | Purpose |
|----------|---------|---------|
| `SIDEBAR_W` | `220.0` | Default sidebar width (DIPs) |
| `H_PAD` | `32.0` | Horizontal content padding |
| `V_PAD` | `20.0` | Vertical content padding |
| `PARA_GAP` | `10.0` | Space between blocks |

## Changing the Docs Folder

Pass the path as the first argument:

```
doc-crate.exe D:\work\my-project\docs
```

Or set a wrapper script / shortcut that always passes your preferred path.

---

See also: [Installation](installation.md) | [Theming](../reference/theming.md) | [Home](../index.md)
