# Mermaid C4 Diagrams

DocCrate renders Mermaid C4 diagrams natively. They are useful for context,
container, component, and deployment views in software architecture docs.

```mermaid
C4Context
title DocCrate Context
Person(reader, "Reader", "Browses local Markdown documentation")
System(doccrate, "DocCrate", "Native Windows documentation viewer")
System_Ext(browser, "Default browser", "Opens external URLs")
Rel(reader, doccrate, "Reads documentation")
Rel(doccrate, browser, "Opens external links", "ShellExecute")
```

A container view with boundaries, relationship labels, and database shapes:

```mermaid
C4Container
title DocCrate Containers
Person(reader, "Reader", "Navigates local docs")
System_Boundary(app, "DocCrate") {
  Container(scanner, "Docs Scanner", "Rust", "Finds Markdown files")
  Container(parser, "Markdown Parser", "pulldown-cmark", "Builds block AST")
  Container(renderer, "Direct2D Renderer", "Win32", "Draws the native UI")
  ContainerDb(cache, "Layout Cache", "Memory", "Stores rendered layouts")
}
Rel(reader, scanner, "Chooses a docs folder")
Rel(scanner, parser, "Loads Markdown")
Rel(parser, renderer, "Sends blocks")
Rel(renderer, cache, "Reads cached layouts")
```

## Manual C4 Layout

Comment annotations can take over C4 placement when a diagram needs a more
deliberate presentation. Use `@node` for elements, `@group` for boundaries,
`@edge` for relationship routing or label placement, and `@graph` for the
canvas.

```mermaid
C4Container
title Annotated C4 Container View
Person(user, "Operator", "Runs local docs")
System_Boundary(docrate, "DocCrate") {
  Container(scanner, "Scanner", "Rust", "Finds Markdown files")
  Container(parser, "Parser", "pulldown-cmark", "Builds document blocks")
  Container(renderer, "Renderer", "Direct2D", "Draws native pages")
  ContainerDb(cache, "Layout Cache", "Memory", "Keeps parsed layouts")
}
System_Ext(shell, "Windows Shell", "Opens external URLs")
Rel(user, scanner, "opens folder")
Rel(scanner, parser, "loads")
Rel(parser, renderer, "blocks")
Rel(renderer, cache, "reads/writes")
Rel(renderer, shell, "ShellExecute")
%% @node user x=36 y=150 w=152 h=140
%% @node scanner x=245 y=78 w=160 h=100
%% @node parser x=460 y=78 w=160 h=100
%% @node renderer x=350 y=245 w=170 h=105
%% @node cache x=560 y=245 w=170 h=105
%% @node shell x=760 y=155 w=160 h=100
%% @group docrate x=220 y=35 w=530 h=350
%% @edge user->scanner points="188,220 210,220 210,128 245,128" label_offset="0,-13"
%% @edge scanner->parser points="405,128 460,128"
%% @edge parser->renderer bend_points="540,206 435,206" label_pos="490,206"
%% @edge renderer->cache points="520,298 560,298"
%% @edge renderer->shell points="520,298 520,368 745,368 745,205 760,205" label_pos="710,205"
%% @graph w=960 h=420
```

Deployment nodes use a solid boundary:

```mermaid
C4Deployment
title DocCrate Test Harness
Deployment_Node(workstation, "Windows workstation", "Win32") {
  Container(app, "doc-crate.exe", "Rust", "Renders Markdown and Mermaid")
  ContainerDb(screen, "screen.png", "PNG", "Snapshot written by --testsnap")
}
Rel(app, screen, "Captures the current view", "WIC")
```
