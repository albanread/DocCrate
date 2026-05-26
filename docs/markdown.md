# Markdown Guide

This page demonstrates all supported formatting.

---

## Headings

# H1 — Section Title
## H2 — Chapter
### H3 — Topic
#### H4 — Subtopic
##### H5 — Note
###### H6 — Fine print

---

## Text Formatting

Plain paragraph text renders at 15 DIP with Segoe UI.

**Bold text** stands out with heavier weight.

*Italic text* provides emphasis.

`Inline code` uses Cascadia Code with a distinctive color.

---

## Code Blocks

```rust
fn main() {
    println!("Hello, DocCrate!");
}
```

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"
```

```
Plain preformatted text
with no language hint.
```

---

## Lists

### Bullet List

- First item
- Second item
- Third item with a longer description that may wrap onto the next line

### Numbered List

1. Install Rust
2. Clone the repo
3. Run `cargo build --release`
4. Launch `doc-crate.exe`

---

## Blockquote

> This is a blockquote. It can contain multiple sentences and will
> render with an accent bar on the left side.

---

## Links

- [Home](index.md) — local link to another doc
- [Getting Started](getting-started.md) — local navigation
- [Keyboard Shortcuts](shortcuts.md)

---

## Horizontal Rule

Three or more dashes produce a thin separator:

---

Back to [Home](index.md).
