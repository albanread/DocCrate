# Selkie + Annotations Design

## Purpose

This document describes how to evolve Selkie from a Mermaid parser/rendering engine into a
flowchart engine that can also support:

- a typed editable object model
- annotation-driven layout and rendering overrides
- reliable Mermaid source regeneration
- SVG rendering that reflects both automatic layout and user overrides

The goal is not to replace Selkie's Mermaid renderer with a second renderer. The goal is to extend
the real Selkie pipeline so it can support an editor-facing object model and annotation layer.

## Review Of How Selkie Works Today

For flowcharts, Selkie currently has a strong three-stage pipeline:

1. Mermaid text is parsed into [`FlowchartDb`](../src/diagrams/flowchart/types.rs).
2. [`FlowchartDb`](../src/diagrams/flowchart/types.rs) is converted into a typed
   [`LayoutGraph`](../src/layout/types.rs) by [`render/flowchart.rs`](../src/render/flowchart.rs)
   and [`layout/adapter.rs`](../src/layout/adapter.rs).
3. The laid-out graph is rendered to SVG by [`SvgRenderer`](../src/render/svg/mod.rs), with node
   shapes in [`shapes.rs`](../src/render/svg/shapes.rs) and edges in
   [`edges.rs`](../src/render/svg/edges.rs).

### What is already good

- Flowchart parsing is real and substantial.
- The layout layer is already separated from the parser.
- The SVG path is already inspectable and testable.
- Subgraphs already map naturally to compound layout.
- The renderer already has a useful container structure:
  `clusters -> edgePaths -> edgeLabels -> nodes`.

### What is missing for the editor use case

- Comment statements are parsed lexically, but ignored semantically.
- The flowchart database is Mermaid-oriented, not editor-oriented.
- Styling is mostly string-based (`styles`, `classes`, metadata `HashMap<String, String>`).
- Layout nodes and edges do not expose typed override fields for fixed positions, sizes, routing,
  label placement, or canvas properties.
- There is no source-regeneration model beyond "parse Mermaid and render it".

## Design Principles

### 1. Keep Mermaid semantics and editor semantics separate

Selkie should not overload `FlowchartDb` until it becomes both a parser DB and an editor model.

Instead:

- `FlowchartDb` stays Mermaid-shaped
- a new editor-facing model is introduced for annotated flowcharts
- conversion happens explicitly between them

### 2. The editable graph becomes the source of truth after parsing

Once a flowchart is loaded for editing:

- the editable graph is authoritative
- annotations are typed data on graph objects
- source regeneration comes from the editable graph
- preserved source hints may guide formatting, but never override semantics

### 3. Annotations are an override pass

From a user perspective:

- no annotations means normal automatic layout/rendering
- annotations feel like a second pass over the graph before rendering

That should also be true in the implementation:

- automatic layout remains the default
- annotation overrides are applied after parse and before final render
- partial overrides must be supported

### 4. Use typed data at the core, string comments only at the boundary

In Mermaid source, annotations are strings embedded in comments.

Inside Selkie, they should become typed fields:

- positions
- sizes
- colors
- line styles
- label alignment
- connection points
- routing points
- canvas defaults

## Proposed Architecture

## Layer 1: Mermaid Parse Layer

Keep the current parser and `FlowchartDb`, but extend comment handling.

New module:

- `src/diagrams/flowchart/annotations.rs`

New responsibility:

- parse recognized annotation comments from `comment_stmt`
- preserve unknown comments separately
- attach structured annotation records to parsed flowchart data

Suggested comment syntax:

```mermaid
%% @graph width_cm="18" height_cm="10" font_face="Aptos"
%% @node Review x="120" y="80" w="140" h="64" fill="#fff4cc"
%% @edge A->B#2 line_color="#3366cc" line_width="2" line_style="dash"
%% @group Ops x="40" y="50" w="540" h="260"
```

Current Selkie implementation notes:

- graph, node, edge, and group annotations are now parsed in the real flowchart parser
- style and font overrides already flow through to the real SVG renderer
- geometry overrides now act as a post-layout pass for flowcharts
- `path_mode="straight"` and `path_mode="orthogonal"` now preserve exact SVG segments
- Selkie now also has an explicit flowchart formatter path for canonical Mermaid output, which is
  useful for diff-based tests when source is semantically unchanged but not already in Selkie's
  preferred formatting
- that formatter currently prefers explicit node declarations plus explicit edge statements over
  inline shorthand like `A[Start] --> B[Finish]`, so formatter-based tests should compare against
  Selkie's canonical output, not the original shorthand
- `bend_points` currently accepts either `;` or `|` separated point lists at the annotation
  parser level, and `|` is the safer Mermaid-facing form inside comment lines, for example:

```mermaid
%% @edge A->B start_connection="right" end_connection="left" bend_points="180,65|180,230"
```

Suggested parse target:

```rust
pub enum AnnotationTargetRef {
    Graph,
    Node { mermaid_id: String },
    Edge { from: String, to: String, ordinal: u32 },
    Group { id: String },
}

pub struct ParsedAnnotation {
    pub target: AnnotationTargetRef,
    pub key: String,
    pub value: String,
    pub source_line: Option<usize>,
}
```

This layer should not yet decide how layout is overridden. It should only parse and normalize.

## Layer 2: Editable Object Model

Introduce a new editor-facing model for annotated flowcharts.

Suggested module:

- `src/diagrams/flowchart/editable.rs`

Suggested root type:

```rust
pub struct EditableFlowchart {
    pub graph: EditableFlowGraph,
    pub source_hints: SourceHints,
}
```

Suggested object graph:

```rust
pub struct EditableFlowGraph {
    pub direction: Direction,
    pub nodes: Vec<EditableNode>,
    pub edges: Vec<EditableEdge>,
    pub groups: Vec<EditableGroup>,
    pub canvas: CanvasOverrides,
}

pub struct EditableNode {
    pub id: NodeId,
    pub mermaid_id: String,
    pub label: String,
    pub shape: FlowVertexType,
    pub parent_group: Option<GroupId>,
    pub overrides: NodeOverrides,
}

pub struct EditableEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub sibling_index: u32,
    pub label: Option<String>,
    pub stroke: EdgeStroke,
    pub arrow_type: ArrowType,
    pub overrides: EdgeOverrides,
}

pub struct EditableGroup {
    pub id: GroupId,
    pub mermaid_id: Option<String>,
    pub label: String,
    pub parent_group: Option<GroupId>,
    pub overrides: GroupOverrides,
}
```

### Why a separate editable model is worth it

Selkie's current `FlowchartDb` is optimized for parsing Mermaid statements, not for:

- graph editing
- group membership editing
- relative coordinates inside groups
- stable edge identity for duplicate endpoint edges
- typed annotation editing
- source round-tripping

Trying to make `FlowchartDb` do all of that directly will make the parser DB harder to reason
about and harder to keep compatible with Mermaid.

## Layer 3: Typed Override Schema

The override schema should be explicit and shared by parsing, editing, layout, and rendering.

Suggested module:

- `src/diagrams/flowchart/overrides.rs`

Suggested types:

```rust
pub struct CanvasOverrides {
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub canvas_fill: Option<String>,
    pub font_face: Option<String>,
    pub node_label_font_size: Option<f64>,
    pub group_label_font_size: Option<f64>,
    pub edge_label_font_size: Option<f64>,
    pub node_label_align: Option<LabelAlign>,
    pub group_label_align: Option<LabelAlign>,
    pub edge_label_align: Option<LabelAlign>,
}

pub struct NodeOverrides {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub line_width: Option<f64>,
    pub label_align: Option<LabelAlign>,
}

pub struct EdgeOverrides {
    pub line_color: Option<String>,
    pub line_width: Option<f64>,
    pub line_style: Option<LineStyle>,
    pub start_connection: Option<ConnectionPoint>,
    pub end_connection: Option<ConnectionPoint>,
    pub start_arrow: Option<ArrowHead>,
    pub end_arrow: Option<ArrowHead>,
    pub path_mode: Option<PathMode>,
    pub bend_points: Vec<Point>,
    pub label_offset: Option<Point>,
}

pub struct GroupOverrides {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub label_align: Option<LabelAlign>,
}
```

Group membership itself should remain structural state, not an annotation key.

## Layer 4: Source Hints And Writeback

The editable model needs writeback hints, but they must remain subordinate to graph semantics.

Suggested module:

- `src/diagrams/flowchart/writeback.rs`

Suggested responsibility:

- preserve `graph` vs `flowchart`
- preserve `TB` vs `TD`
- preserve node delimiter choices where valid
- preserve group header form where valid
- preserve stable statement order where possible
- emit annotation comments in a stable order

This should support:

1. parse Mermaid to editable model
2. edit object model
3. write Mermaid
4. parse Mermaid again
5. confirm object model survived

The critical rule:

- source hints may preserve formatting choices
- source hints must never reinsert stale semantics after an edit

## Layer 5: Layout Integration

Selkie's existing layout engine is the right foundation, but it needs typed constraint hooks.

Today, the flowchart path is:

- `FlowchartDb -> LayoutGraph -> layout::layout -> SVG`

For annotated flowcharts, the path should become:

- `EditableFlowchart -> LayoutGraphWithOverrides -> layout::layout_with_overrides -> SVG`

### Minimal viable layout extension

Extend layout types rather than hiding editor data in `metadata`.

Suggested additions to `LayoutNode`:

```rust
pub struct LayoutNodeOverrides {
    pub fixed_x: Option<f64>,
    pub fixed_y: Option<f64>,
    pub fixed_width: Option<f64>,
    pub fixed_height: Option<f64>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub line_width: Option<f64>,
    pub label_align: Option<LabelAlign>,
}
```

Suggested additions to `LayoutEdge`:

```rust
pub struct LayoutEdgeOverrides {
    pub line_color: Option<String>,
    pub line_width: Option<f64>,
    pub line_style: Option<LineStyle>,
    pub start_connection: Option<ConnectionPoint>,
    pub end_connection: Option<ConnectionPoint>,
    pub start_arrow: Option<ArrowHead>,
    pub end_arrow: Option<ArrowHead>,
    pub bend_points: Vec<Point>,
    pub label_position: Option<Point>,
}
```

Suggested additions to `LayoutGraph`:

```rust
pub struct LayoutCanvasOverrides {
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub canvas_fill: Option<String>,
    pub font_face: Option<String>,
    pub node_label_font_size: Option<f64>,
    pub group_label_font_size: Option<f64>,
    pub edge_label_font_size: Option<f64>,
}
```

### How overrides should affect layout

- Node `x/y` present:
  node is placed relative to its containing group, or to the canvas if ungrouped.
- Group `x/y` present:
  group origin is fixed relative to its parent group or canvas.
- Node/group `w/h` present:
  override estimated size.
- Edge `bend_points` present:
  use explicit route instead of automatic route.
- Edge `label_offset` present:
  adjust label position after automatic midpoint is computed.
- Canvas size present:
  influence SVG viewBox and document-space allocation.

### Important design choice

Partial constraints should be legal.

Examples:

- fixed node positions, but automatic edge routing
- automatic node layout, but manual edge bend points
- automatic layout, but manual colors/fonts

That keeps the "annotation override pass" mental model intact.

## Layer 6: SVG Rendering Integration

The renderer should not parse annotation strings. By the time SVG is rendered, everything should
already be typed.

That means:

- node colors, sizes, and label alignment come from typed node/group/canvas overrides
- edge line width/style/color/markers come from typed edge overrides
- canvas fill and fonts come from typed graph overrides

### Good news from the current SVG path

The existing renderer is already close to what we need:

- node shapes are centralized in [`shapes.rs`](../src/render/svg/shapes.rs)
- edges are centralized in [`edges.rs`](../src/render/svg/edges.rs)
- the document wrapper is centralized in [`document.rs`](../src/render/svg/document.rs)

### Recommended renderer changes

1. Add typed render styling inputs instead of depending on string metadata.
2. Let the document root render canvas background fill if present.
3. Let node rendering honor explicit `fill`, `stroke`, `line_width`, and label alignment.
4. Let edge rendering honor explicit `line_color`, `line_width`, `line_style`,
   `start_arrow`, `end_arrow`, and explicit routing points.
5. Let label rendering honor graph-level font family and font sizes.

## Recommended Public API Shape

Flowchart support should grow in stages.

### Stage 1 API

Keep the current API intact, and add new editor-facing flowchart entry points:

```rust
pub fn parse_flowchart_editable(input: &str) -> Result<EditableFlowchart>;
pub fn write_flowchart_editable(model: &EditableFlowchart) -> Result<String>;
pub fn render_flowchart_editable_svg(model: &EditableFlowchart) -> Result<String>;
```

This avoids breaking the general `Diagram` API too early.

### Stage 2 API

Once the model stabilizes, consider a new diagram enum variant or a more general editor API:

```rust
pub enum EditableDiagram {
    Flowchart(EditableFlowchart),
}
```

For now, flowcharts should be the first and only annotation-aware diagram family.

## Parsing Strategy For Annotation Comments

Selkie already recognizes comment statements in the grammar:

- `leading_comment`
- `comment_stmt`

That means we do not need a separate text preprocessor.

Recommended approach:

1. keep comments in the parse stream
2. when a `comment_stmt` starts with `@graph`, `@node`, `@edge`, or `@group`, parse it as an
   annotation directive
3. store typed annotation records
4. preserve unrecognized comments separately for round-trip if needed

This is cleaner than raw string scanning before the parser because:

- it stays inside Selkie's real grammar
- it preserves line ordering more reliably
- it gives better diagnostics

## Source Regeneration Strategy

Selkie will need a real Mermaid writer for editable flowcharts.

Recommended order of implementation:

1. exact writeback for the supported annotated subset
2. edit-roundtrip tests
3. broader format preservation

### Required tests

#### Lossless parse/write tests

- parse Mermaid with annotations
- write Mermaid
- compare expected supported-form output exactly

#### Edit roundtrip tests

- parse Mermaid
- mutate editable graph
- write Mermaid
- parse again
- compare object model

#### SVG reflection tests

- render without annotations and assert automatic output markers
- render with annotations and assert SVG changes for:
  - canvas fill
  - node position/size/fill/stroke
  - group bounds
  - edge color/width/style
  - edge routing points
  - font family / font size

These SVG tests are especially valuable because SVG is text and easy to review.

## Recommended Implementation Phases

### Phase 1: Parse annotations into typed records

- add flowchart annotation parser
- parse `@graph`, `@node`, `@edge`, `@group`
- add diagnostics for unresolved targets

### Phase 2: Add editable flowchart model

- add `EditableFlowchart`
- map `FlowchartDb + annotations -> EditableFlowchart`
- preserve duplicate-edge identity and group membership

### Phase 3: Add Mermaid writeback

- emit Mermaid from editable model
- preserve stable source hints
- add lossless round-trip tests

### Phase 4: Add layout override support

- extend layout types with typed override fields
- support partial fixed positions/sizes/routes
- preserve automatic layout as default behavior

### Phase 5: Add annotation-aware SVG rendering

- render typed overrides in the real SVG renderer
- add SVG text-based tests

### Phase 6: Add edit API

- add node/edge/group create-update-delete operations
- add annotation mutation helpers
- add edit-roundtrip tests

## Recommended Boundary With Brochure Maker

Selkie should own:

- Mermaid parsing
- annotation comment parsing
- editable flowchart model
- Mermaid writeback
- layout override application
- SVG rendering

Brochure Maker should own:

- brochure documents
- PDF/HTML/Word container exports
- multi-diagram document workflow
- application UI and persistence

That keeps Selkie as the diagram engine, and Brochure Maker as the product.

## Recommendation

The cleanest path is:

- extend real Selkie for flowcharts first
- make annotations first-class in Selkie
- create the editable object model inside Selkie
- keep Mermaid comments as the external storage format
- keep typed overrides as the internal model

This gives us a true "Selkie + annotations" engine rather than a second diagram system sitting
next to Selkie.
