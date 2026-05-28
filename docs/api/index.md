# API Overview

DocCrate exposes no network API — it is a local viewer. This section documents
the **internal data structures and extension points** that developers can use
when building on top of the source.

## Core Modules

| Module | Responsibility |
|--------|---------------|
| `docs` | File discovery and path resolution |
| `parser` | Markdown → `Vec<Block>` AST |
| `layout` | AST → `Vec<DrawCmd>` + hit regions |
| `render` | Win32 window, Direct2D paint loop |
| `theme` | All colours, fonts, and size constants |

## Data Flow

```
docs::scan()   →  (Vec<DocFile>, Vec<SidebarEntry>)
docs::load()   →  String  (raw Markdown)
parser::parse() →  Vec<Block>
layout::layout() →  Layout { cmds, hits, total_h }
render / draw loop  →  Direct2D calls
```

## See Also

- [Authentication](authentication.md) — how file paths are resolved safely
- [Endpoints](endpoints.md) — the custom Win32 messages used internally

---

Back to [Home](../index.md) | See also: [Architecture](../architecture.md)
