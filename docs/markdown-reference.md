# Markdown Reference

Complete reference for every Markdown element DocCrate supports, with the raw syntax
shown in a code block followed immediately by the rendered result.

---

## Headings

Headings are created with `#` prefix characters. One `#` = H1, six `######` = H6.

```
# H1 Heading
## H2 Heading
### H3 Heading
#### H4 Heading
##### H5 Heading
###### H6 Heading
```

Rendered result — all six levels with their colour assignments:

# H1 — Teal (#4EC9B0)
## H2 — Light Blue (#9CDCFE)
### H3 — Yellow (#DCDCAA)
#### H4 — Purple (#C586C0)
##### H5 — Orange (#CE9178)
###### H6 — Dim (#808080)

---

## Paragraphs

Consecutive lines separated by a blank line form distinct paragraphs.

```
This is the first paragraph. It can span
multiple source lines.

This is the second paragraph.
```

This is the first paragraph. It can span
multiple source lines.

This is the second paragraph.

---

## Inline Text Formatting

### Bold

Wrap text in `**double asterisks**` or `__double underscores__`.

```
**This text is bold.**
```

**This text is bold.**

### Italic

Wrap text in `*single asterisks*` or `_single underscores_`.

```
*This text is italic.*
```

*This text is italic.*

### Bold and Italic Combined

Wrap text in `***triple asterisks***`.

```
***This text is bold and italic simultaneously.***
```

***This text is bold and italic simultaneously.***

### Inline Code

Wrap text in single backticks. Renders in Cascadia Code with orange tint.

```
Use `cargo build --release` to produce the optimised binary.
```

Use `cargo build --release` to produce the optimised binary.

### Mixed Inline Styles

All inline styles can appear within the same paragraph. Each span is laid out
sequentially, with approximate width used to track horizontal position:

The function **parses** the `&str` input, returns *either* a `Vec<Block>` or an
***empty vector*** if the source is blank.

---

## Fenced Code Blocks

A fenced code block starts and ends with three backticks. An optional language hint
follows the opening fence on the same line.

````
```rust
fn hello() {
    println!("Hello, world!");
}
```
````

```rust
fn hello() {
    println!("Hello, world!");
}
```

````
```toml
[package]
name = "doc-crate"
version = "0.1.0"
edition = "2021"
```
````

```toml
[package]
name = "doc-crate"
version = "0.1.0"
edition = "2021"
```

````
```
No language hint — plain preformatted text.
Indentation and spacing are preserved exactly.
  Column two
    Column four
```
````

```
No language hint — plain preformatted text.
Indentation and spacing are preserved exactly.
  Column two
    Column four
```

---

## Blockquotes

Begin lines with `>` to create a blockquote. The block is indented and marked
with a green left border.

```
> This is a blockquote. It renders with a green accent bar on the left edge
> and indented text to its right.
```

> This is a blockquote. It renders with a green accent bar on the left edge
> and indented text to its right.

Blockquotes support inline formatting:

```
> **Warning:** never call `unsafe` code without reading the *Safety* section
> of the relevant API documentation.
```

> **Warning:** never call `unsafe` code without reading the *Safety* section
> of the relevant API documentation.

Blockquotes can be nested by using multiple `>` characters:

```
> Outer blockquote level.
>
> > Inner blockquote — a quote within a quote.
```

> Outer blockquote level.
>
> > Inner blockquote — a quote within a quote.

---

## Bullet Lists

An unordered list begins each item with `- `, `* `, or `+ `.

```
- Alpha
- **Beta** — bold item
- *Gamma* — italic item
- `Delta` — code item
- [Epsilon](index.md) — link item
```

- Alpha
- **Beta** — bold item
- *Gamma* — italic item
- `Delta` — code item
- [Epsilon](index.md) — link item

---

## Ordered Lists

An ordered list prefixes each item with a number and a period. The rendered
markers always follow the source order from the starting number.

```
1. First step
2. Second step
3. Third step
```

1. First step
2. Second step
3. Third step

Lists can start at any number:

```
42. The answer
43. The question
44. The meaning
```

42. The answer
43. The question
44. The meaning

---

## Tables

Tables use the GitHub-flavoured Markdown pipe syntax. The separator row (dashes)
must be present between the header row and the body rows.

```
| Column A | Column B | Column C |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |
```

| Column A | Column B | Column C |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

Tables support inline formatting in cells:

```
| Name | Type | Notes |
|------|------|-------|
| `scroll_y` | `f32` | Current vertical scroll offset in DIPs |
| `max_scroll` | `f32` | Maximum scrollable distance |
| `sidebar_w` | `f32` | Current sidebar width; **0.0** = hidden |
```

| Name | Type | Notes |
|------|------|-------|
| `scroll_y` | `f32` | Current vertical scroll offset in DIPs |
| `max_scroll` | `f32` | Maximum scrollable distance |
| `sidebar_w` | `f32` | Current sidebar width; **0.0** = hidden |

---

## Horizontal Rules

Three or more hyphens on a line by themselves produce a thin horizontal rule.

```
---
```

---

## Links

### Local Links

Reference another `.md` file by its name, with or without the extension.
Fragment suffixes are automatically stripped before file resolution.

```
[Getting Started](getting-started.md)
[Features](features)
[Section anchor](architecture.md#rendering-pipeline)
```

[Getting Started](getting-started.md)
[Features](features)
[Section anchor](architecture.md#rendering-pipeline)

### External Links

Any `http://` or `https://` URL opens in the system default browser.

```
[Rust Programming Language](https://www.rust-lang.org)
[pulldown-cmark on GitHub](https://github.com/raphlinus/pulldown-cmark)
```

[Rust Programming Language](https://www.rust-lang.org)
[pulldown-cmark on GitHub](https://github.com/raphlinus/pulldown-cmark)

---

## Unsupported Elements

The following CommonMark elements are ***not*** rendered and are silently ignored or
displayed as raw text:

| Element | Behaviour |
|---------|-----------|
| Images `![alt](url)` | Displayed as raw Markdown text |
| HTML blocks `<div>` | Skipped |
| Footnotes `[^1]` | Skipped |
| Task list items `- [x]` | Rendered as plain list items |
| Strikethrough `~~text~~` | Parser parses it; layout ignores it |
| Anchor navigation `#heading` | Fragment stripped; no scroll-to-heading |

---

Back to [Home](index.md) | [Architecture](architecture.md)
