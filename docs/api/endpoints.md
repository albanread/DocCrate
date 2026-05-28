# Internal Messages

DocCrate uses a small set of custom Win32 messages to coordinate between the
message pump and the rendering state machine.

## WM_RELAYOUT

```
PostMessageW(hwnd, WM_APP, 0, 0)
```

Posted whenever the content needs to be re-laid out without triggering a full
navigation. This happens on:

- **Window resize** — content width changes
- **Sidebar resize** — content left edge changes
- **Sidebar toggle** — content area shifts

The handler calls `App::relayout()`, which re-runs `layout::layout()` using
the cached `Vec<Block>` (no re-parse if the document hasn't changed).

## Standard Messages Handled

| Message | Action |
|---------|--------|
| `WM_PAINT` | Full Direct2D redraw |
| `WM_SIZE` | Update DIP dimensions, resize render target, post WM_RELAYOUT |
| `WM_MOUSEWHEEL` | Adjust `scroll_y` |
| `WM_MOUSEMOVE` | Update hover state, set cursor |
| `WM_LBUTTONDOWN` | Hit-test and dispatch (toggle, drag, nav, link) |
| `WM_KEYDOWN` | Scroll keys, Ctrl+B, ← / → history |
| `WM_DPICHANGED` | Drop render target, reposition window |
| `WM_ERASEBKGND` | Returns 1 (suppressed — D2D owns the background) |

---

Back to [API Overview](index.md) | See also: [Architecture](../architecture.md) | [Home](../index.md)
