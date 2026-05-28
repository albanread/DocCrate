# Path Resolution

DocCrate only loads files from the **docs directory** passed at startup (or the
`docs/` folder next to the binary). No file outside this root can be opened
through a link.

## How Links Are Resolved

When a Markdown link is clicked, `docs::resolve_href` is called:

1. External links (`http://`, `https://`) are opened in the default browser via
   `ShellExecuteW` and never loaded into the viewer.
2. Relative paths are resolved against the **directory of the current file**.
3. If the resolved path exists, it opens in the viewer.
4. If not found, `.md` is appended and the lookup is retried.
5. Anchor-only links (`#section`) are silently ignored (no in-page scrolling yet).

## Example

Given current file `docs/guides/installation.md`:

```
[Config](../reference/cli.md)     →  docs/reference/cli.md   ✓
[Home](../index.md)               →  docs/index.md           ✓
[External](https://example.com)   →  browser                 ✓
[Missing](nope.md)                →  no-op                   –
```

## Security Notes

- The viewer never writes to disk.
- No code is executed from Markdown content.
- Image paths follow the same resolution rules as links.

---

Back to [API Overview](index.md) | See also: [Internal Messages](endpoints.md) | [Home](../index.md)
