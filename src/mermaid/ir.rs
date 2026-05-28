//! Doccrate-owned intermediate representation for laid-out mermaid diagrams.
//!
//! Currently supports two diagram families. They share the [`Graph`] enum but
//! have completely different internal layouts:
//!
//! * [`FlowchartGraph`] — nodes + edges + subgraph groups, positioned by selkie's
//!   dagre-style layout engine, then refined by `@annotation` overrides.
//! * [`SequenceGraph`] — actors + lifelines + time-ordered messages, with
//!   layout computed by us (selkie has no LayoutGraph adapter for sequences).
//!
//! Everything in both subtrees is fully resolved: colours are `u32` (RGB in
//! low 24 bits), positions are `f32` DIPs in the graph's natural coordinate
//! space starting at `(0, 0)` and bounded by `width × height`.

// ===========================================================================
// Top-level enum
// ===========================================================================

#[derive(Debug, Clone)]
pub enum Graph {
    Flowchart(FlowchartGraph),
    Sequence(SequenceGraph),
}

impl Graph {
    pub fn width(&self) -> f32 {
        match self {
            Graph::Flowchart(g) => g.width,
            Graph::Sequence(g)  => g.width,
        }
    }
    pub fn height(&self) -> f32 {
        match self {
            Graph::Flowchart(g) => g.height,
            Graph::Sequence(g)  => g.height,
        }
    }
}

// ===========================================================================
// Flowchart
// ===========================================================================

#[derive(Debug, Clone)]
pub struct FlowchartGraph {
    pub width:  f32,
    pub height: f32,
    /// `@graph canvas_fill` if set; otherwise `None` (transparent).
    pub background: Option<u32>,
    pub groups: Vec<Group>,
    pub nodes:  Vec<Node>,
    pub edges:  Vec<Edge>,
}

/// Node shape. Anything mermaid supports that we don't yet handle natively
/// falls back to [`Shape::Rect`] at build time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Rect,
    RoundedRect,
    Stadium,
    Circle,
    /// `(((text)))` — concentric circles
    DoubleCircle,
    Ellipse,
    Diamond,
    Hexagon,
    /// `[(text)]` — classic database / data-store cylinder
    Cylinder,
    /// `[[text]]` — rectangle with inner vertical bars
    Subroutine,
    /// `[/text\]` — wider at the bottom
    Trapezoid,
    /// `[\text/]` — wider at the top
    InvTrapezoid,
    /// `[/text/]` — parallelogram leaning right
    LeanRight,
    /// `[\text\]` — parallelogram leaning left
    LeanLeft,
    /// `>text]` — asymmetric pentagonal (flag) shape
    Odd,
    /// Used for fork/join bars in state diagrams. Thin filled bar, no label.
    HorizontalBar,
    /// Renderer-defined extension shape — index into the App-wide shape
    /// registry (built-ins + `docs/.shapes/*.shape`). Kept as a `u32` so
    /// `Shape` stays `Copy` and `DrawCmd::Mermaid` doesn't have to clone
    /// strings each frame.
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align { Left, Center, Right }

#[derive(Debug, Clone)]
pub struct Node {
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub shape: Shape,
    pub label: String,
    pub label_align: Align,
    pub fill: u32,
    pub stroke: u32,
    pub stroke_w: f32,
    pub text_color: u32,
    pub font_size: f32,
    pub bold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle { Solid, Dash, Dot }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    None,
    Triangle,
    Circle,
    Cross,
}

#[derive(Debug, Clone)]
pub struct EdgeLabel {
    pub x: f32, pub y: f32,
    pub w: f32, pub h: f32,
    pub text: String,
    pub text_color: u32,
    pub font_size: f32,
}

#[derive(Debug, Clone)]
pub struct Edge {
    /// Polyline path. Always ≥ 2 points (start, end).
    pub points: Vec<(f32, f32)>,
    pub line_color: u32,
    pub line_w: f32,
    pub line_style: LineStyle,
    pub start_arrow: Arrow,
    pub end_arrow:   Arrow,
    pub label: Option<EdgeLabel>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub title: Option<String>,
    pub fill: u32,
    pub stroke: u32,
    pub stroke_w: f32,
    pub title_font_size: f32,
    pub title_color: u32,
}

// ===========================================================================
// Sequence
// ===========================================================================

#[derive(Debug, Clone)]
pub struct SequenceGraph {
    pub width:  f32,
    pub height: f32,
    pub actors:   Vec<SeqActor>,
    pub messages: Vec<SeqMessage>,
}

#[derive(Debug, Clone)]
pub struct SeqActor {
    /// Top participant box.
    pub box_x: f32, pub box_y: f32, pub box_w: f32, pub box_h: f32,
    /// Vertical lifeline beneath the box.
    pub lifeline_x:  f32,
    pub lifeline_y0: f32,
    pub lifeline_y1: f32,
    pub label: String,
    pub fill: u32,
    pub stroke: u32,
    pub text_color: u32,
    pub font_size: f32,
    /// Optional renderer-defined shape for the participant box.
    /// `None` → the default rounded rectangle.
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStyle { Solid, Dotted }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageArrow {
    /// Filled triangle — sync request
    Filled,
    /// Open (stick) arrow — async / return
    Open,
    /// "X" mark — destroy / lost
    Cross,
    None,
}

#[derive(Debug, Clone)]
pub struct SeqMessage {
    pub from_x: f32,
    pub to_x:   f32,
    pub y:      f32,
    pub label:  String,
    pub style:        MessageStyle,
    pub start_arrow:  MessageArrow,
    pub end_arrow:    MessageArrow,
    /// `true` when the sender and receiver are the same actor; rendered as a
    /// short rectangular loop on the right side of the lifeline.
    pub self_loop: bool,
    pub color: u32,
    pub label_color: u32,
    pub font_size: f32,
}
