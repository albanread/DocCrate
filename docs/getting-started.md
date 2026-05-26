# Getting Started

## Requirements

- Windows 10 or 11
- A `docs/` folder containing `.md` files

## Running

```
doc-crate.exe [docs-directory]
```

If no argument is provided, DocCrate looks for a `docs/` folder next to the executable.

## Adding Documentation

Drop any `.md` files into the `docs/` folder. DocCrate will pick them up automatically on next launch.

File names become the sidebar labels:

- `getting-started.md` → **Getting Started**
- `api-reference.md` → **Api Reference**
- `index.md` or `readme.md` appear first

## Markdown Support

DocCrate supports:

- Headings (H1–H6)
- **Bold**, *italic*, `inline code`
- Fenced code blocks with language hints
- Bullet and numbered lists
- Blockquotes
- Hyperlinks (local `.md` files + external URLs)
- Horizontal rules

## Tips

- Keep lines under 100 characters for best layout
- Use `index.md` as your landing page
- Internal links: `[text](other-file.md)` or `[text](other-file)`
- External links open in your default browser

---

Back to [Home](index.md).
