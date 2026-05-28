//! `mermaid::build` — run selkie's parse + layout + annotation overrides on a
//! mermaid source string and produce a doccrate-owned [`Graph`].
//!
//! After this step, the IR holds every property the renderer needs, including
//! colours / strokes / arrow heads / line styles resolved from `@annotation`
//! comments. Selkie's own types are not exposed beyond this module.

use crate::mermaid::ir::*;
use crate::mermaid::sequence;
use crate::theme;

use selkie::diagrams::flowchart::{
    ArrowHead as SArrow, FlowchartDb, LabelAlign as SLabelAlign, LineStyle as SLineStyle,
};
use selkie::diagrams::state::StateDb;
use selkie::diagrams::Diagram;
use selkie::layout::{
    self, CharacterSizeEstimator, LayoutEdge, LayoutGraph, LayoutNode, NodeShape as SNodeShape,
    ToLayoutGraph,
};
use selkie::render::apply_flowchart_annotation_layout_overrides;

/// Parse a mermaid source string and produce a rendered-ready [`Graph`].
///
/// Dispatches on diagram type:
/// * `flowchart` / `graph` → selkie parse + layout + annotation overrides →
///   [`FlowchartGraph`]
/// * `sequenceDiagram` → selkie parse only (no LayoutGraph for sequences);
///   layout is computed in [`crate::mermaid::sequence::build`] →
///   [`SequenceGraph`]
/// * everything else → `Err(...)` so the caller can fall back to showing the
///   raw fenced source as a code block.
pub fn build(source: &str) -> Result<Graph, String> {
    let diagram = selkie::parse(source).map_err(|e| format!("parse error: {e}"))?;
    match &diagram {
        Diagram::Flowchart(db) => Ok(Graph::Flowchart(build_flowchart(db)?)),
        Diagram::Sequence(db)  => Ok(Graph::Sequence(sequence::build(db))),
        Diagram::State(db)     => Ok(Graph::Flowchart(build_state(db)?)),
        _ => Err(format!(
            "unsupported diagram type (only flowchart, sequenceDiagram, and stateDiagram supported)"
        )),
    }
}

fn build_flowchart(db: &FlowchartDb) -> Result<FlowchartGraph, String> {
    let estimator = CharacterSizeEstimator::default();
    let mut lg = db
        .to_layout_graph(&estimator)
        .map_err(|e| format!("layout-graph build: {e}"))?;
    // Hard aspect-ratio enforcement for custom shapes — must run BEFORE
    // selkie's `layout()` so edge routing sees the final node dimensions.
    enforce_custom_aspect(&mut lg);
    let mut lg = layout::layout(lg).map_err(|e| format!("layout: {e}"))?;
    apply_flowchart_annotation_layout_overrides(db, &mut lg);
    Ok(convert(db, &lg))
}

/// State diagrams (`stateDiagram-v2`). Selkie already maps every
/// `StateType` to an appropriate `NodeShape` (Start → Circle, End →
/// DoubleCircle, Fork/Join → HorizontalBar, Choice → Diamond, default →
/// RoundedRect), so we reuse the `FlowchartGraph` IR and the same renderer.
/// `@annotation` overrides are flowchart-only at the selkie layer, so
/// state diagrams render with theme defaults.
fn build_state(db: &StateDb) -> Result<FlowchartGraph, String> {
    let estimator = CharacterSizeEstimator::default();
    let mut lg = db
        .to_layout_graph(&estimator)
        .map_err(|e| format!("layout-graph build: {e}"))?;
    enforce_custom_aspect(&mut lg);
    let lg = layout::layout(lg).map_err(|e| format!("layout: {e}"))?;
    Ok(convert_state(&lg))
}

/// Annotation-free LayoutGraph → FlowchartGraph conversion. Used by
/// state diagrams (and any future diagram type that produces a vanilla
/// LayoutGraph without an annotation database).
fn convert_state(lg: &LayoutGraph) -> FlowchartGraph {
    let bx = lg.bounds_x.unwrap_or(0.0) as f32;
    let by = lg.bounds_y.unwrap_or(0.0) as f32;
    let w  = lg.width.unwrap_or(0.0) as f32;
    let h  = lg.height.unwrap_or(0.0) as f32;

    let mut groups: Vec<Group> = Vec::new();
    let mut nodes:  Vec<Node>  = Vec::new();
    collect_state_nodes(&lg.nodes, bx, by, &mut groups, &mut nodes);

    let mut edges = Vec::new();
    for edge in &lg.edges {
        if let Some(e) = convert_edge_minimal(edge, bx, by) {
            edges.push(e);
        }
    }

    FlowchartGraph {
        width:  w.max(1.0),
        height: h.max(1.0),
        background: None,
        groups,
        nodes,
        edges,
    }
}

/// Walk LayoutGraph nodes. Selkie's state adapter keeps everything flat in
/// `lg.nodes` (composite parents and their children all at the top level)
/// and tags composites via `metadata["is_group"] = "true"`. We honour that
/// tag here. The `children` array on `LayoutNode` is empty for state
/// graphs, but we still recurse into it defensively in case some future
/// adapter nests them.
fn collect_state_nodes(
    nodes: &[LayoutNode],
    bx: f32, by: f32,
    groups: &mut Vec<Group>,
    out:    &mut Vec<Node>,
) {
    for n in nodes {
        if n.is_dummy { continue; }
        let (x, y) = node_origin(n, bx, by);
        let is_group = n.metadata.get("is_group").map(|s| s == "true").unwrap_or(false);
        if is_group {
            groups.push(Group {
                x, y,
                w: n.width as f32,
                h: n.height as f32,
                title: n.label.clone(),
                fill:   theme::MERMAID_GROUP_FILL,
                stroke: theme::MERMAID_GROUP_STROKE,
                stroke_w: theme::MERMAID_GROUP_STROKE_W,
                title_font_size: theme::MERMAID_GROUP_FONT_SIZE,
                title_color:    theme::MERMAID_GROUP_TITLE,
            });
        } else {
            // Leaf state. Start markers paint as a solid dot
            // (`fill = stroke`) per mermaid convention. End markers keep
            // the default theme fill — the inner circle of `DoubleCircle`
            // gives the visual cue.
            let state_type = n.metadata.get("state_type")
                .map(|s| s.as_str()).unwrap_or("");
            let fill = if state_type == "Start" {
                theme::MERMAID_NODE_STROKE
            } else {
                theme::MERMAID_NODE_FILL
            };
            out.push(Node {
                x, y,
                w: n.width as f32,
                h: n.height as f32,
                shape: convert_shape(n.shape, &n.metadata),
                label: n.label.clone().unwrap_or_default(),
                label_align: Align::Center,
                fill,
                stroke:      theme::MERMAID_NODE_STROKE,
                stroke_w:    theme::MERMAID_NODE_STROKE_W,
                text_color:  theme::MERMAID_NODE_TEXT,
                font_size:   theme::MERMAID_NODE_FONT_SIZE,
                bold:        false,
            });
        }
        if !n.children.is_empty() {
            collect_state_nodes(&n.children, bx, by, groups, out);
        }
    }
}

/// LayoutEdge → IR Edge with theme defaults. No annotation lookup.
fn convert_edge_minimal(edge: &LayoutEdge, bx: f32, by: f32) -> Option<Edge> {
    if edge.bend_points.len() < 2 { return None; }
    let points: Vec<(f32, f32)> = edge.bend_points
        .iter()
        .map(|p| (p.x as f32 - bx, p.y as f32 - by))
        .collect();
    let label = match (&edge.label, edge.label_position) {
        (Some(text), Some(pos)) if !text.is_empty() => Some(EdgeLabel {
            x: (pos.x as f32 - bx) - (edge.label_width as f32) / 2.0,
            y: (pos.y as f32 - by) - (edge.label_height as f32) / 2.0,
            w: edge.label_width as f32,
            h: edge.label_height as f32,
            text: text.clone(),
            text_color: theme::MERMAID_EDGE_LABEL,
            font_size: theme::MERMAID_EDGE_FONT_SIZE,
        }),
        _ => None,
    };
    Some(Edge {
        points,
        line_color: theme::MERMAID_EDGE,
        line_w:     theme::MERMAID_EDGE_W,
        line_style: LineStyle::Solid,
        start_arrow: Arrow::None,
        end_arrow:   Arrow::Triangle,
        label,
    })
}

/// Resize `NodeShape::Custom` nodes so their width/height match the
/// `aspect` declared in the shape file. Grows the smaller dimension so we
/// never shrink under the size estimator's text-based minimum.
fn enforce_custom_aspect(lg: &mut LayoutGraph) {
    let reg = crate::mermaid::shape_def::registry();
    lg.traverse_nodes_mut(|n| {
        if !matches!(n.shape, SNodeShape::Custom) { return; }
        let name = match n.metadata.get("shape") {
            Some(s) => s.as_str(),
            None    => return,
        };
        let aspect = match reg.lookup(name).and_then(|i| reg.get(i)).and_then(|d| d.aspect) {
            Some(a) => a as f64,
            None    => return,
        };
        if n.height <= 0.0 { return; }
        let cur = n.width / n.height;
        if cur < aspect {
            n.width = n.height * aspect;
        } else if cur > aspect {
            n.height = n.width / aspect;
        }
    });
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

fn convert(db: &FlowchartDb, lg: &LayoutGraph) -> FlowchartGraph {
    let graph_overrides = db.graph_annotation_overrides();
    let background = graph_overrides.canvas_fill.as_deref().and_then(parse_hex);

    let bx = lg.bounds_x.unwrap_or(0.0) as f32;
    let by = lg.bounds_y.unwrap_or(0.0) as f32;
    let w = lg.width.unwrap_or(0.0) as f32;
    let h = lg.height.unwrap_or(0.0) as f32;

    // Default font sizes from `@graph` if present.
    let node_font = graph_overrides
        .node_label_font_size
        .map(|v| v as f32)
        .unwrap_or(theme::MERMAID_NODE_FONT_SIZE);
    let edge_font = graph_overrides
        .edge_label_font_size
        .map(|v| v as f32)
        .unwrap_or(theme::MERMAID_EDGE_FONT_SIZE);
    let group_font = graph_overrides
        .group_label_font_size
        .map(|v| v as f32)
        .unwrap_or(theme::MERMAID_GROUP_FONT_SIZE);
    let default_node_align = graph_overrides
        .node_label_align
        .map(convert_align)
        .unwrap_or(Align::Center);

    // Subgraphs / groups come first so the renderer can paint them under the nodes.
    let mut groups = Vec::new();
    for sg in db.subgraphs() {
        if let Some(node) = lg.get_node(&sg.id) {
            let (x, y) = node_origin(node, bx, by);
            let ov = db.group_annotation_overrides(&sg.id);
            let fill = ov
                .fill
                .as_deref()
                .and_then(parse_hex)
                .unwrap_or(theme::MERMAID_GROUP_FILL);
            let stroke = ov
                .stroke
                .as_deref()
                .and_then(parse_hex)
                .unwrap_or(theme::MERMAID_GROUP_STROKE);
            let title = if sg.title.is_empty() { None } else { Some(sg.title.clone()) };
            groups.push(Group {
                x, y,
                w: node.width as f32,
                h: node.height as f32,
                title,
                fill,
                stroke,
                stroke_w: theme::MERMAID_GROUP_STROKE_W,
                title_font_size: group_font,
                title_color: theme::MERMAID_GROUP_TITLE,
            });
        }
    }

    // Walk all visible (non-dummy, non-subgraph) nodes recursively.
    let mut nodes = Vec::new();
    let subgraph_ids: std::collections::HashSet<&str> =
        db.subgraphs().iter().map(|s| s.id.as_str()).collect();
    collect_nodes(
        &lg.nodes,
        &subgraph_ids,
        db,
        bx,
        by,
        node_font,
        default_node_align,
        &mut nodes,
    );

    // Edges.
    let mut edges = Vec::new();
    for edge in &lg.edges {
        if let Some(e) = convert_edge(edge, db, bx, by, edge_font) {
            edges.push(e);
        }
    }

    FlowchartGraph {
        width: w.max(1.0),
        height: h.max(1.0),
        background,
        groups,
        nodes,
        edges,
    }
}

fn collect_nodes(
    nodes: &[LayoutNode],
    subgraph_ids: &std::collections::HashSet<&str>,
    db: &FlowchartDb,
    bx: f32,
    by: f32,
    default_font: f32,
    default_align: Align,
    out: &mut Vec<Node>,
) {
    for node in nodes {
        if !node.is_dummy && !subgraph_ids.contains(node.id.as_str()) {
            out.push(convert_node(node, db, bx, by, default_font, default_align));
        }
        if !node.children.is_empty() {
            collect_nodes(&node.children, subgraph_ids, db, bx, by, default_font, default_align, out);
        }
    }
}

fn convert_node(
    node: &LayoutNode,
    db: &FlowchartDb,
    bx: f32,
    by: f32,
    default_font: f32,
    default_align: Align,
) -> Node {
    let (x, y) = node_origin(node, bx, by);
    let ov = db.node_annotation_overrides(&node.id);

    let fill = ov
        .fill
        .as_deref()
        .and_then(parse_hex)
        .unwrap_or(theme::MERMAID_NODE_FILL);
    let stroke = ov
        .stroke
        .as_deref()
        .and_then(parse_hex)
        .unwrap_or(theme::MERMAID_NODE_STROKE);
    let stroke_w = ov
        .line_width
        .map(|v| v as f32)
        .unwrap_or(theme::MERMAID_NODE_STROKE_W);
    let label_align = ov.label_align.map(convert_align).unwrap_or(default_align);

    Node {
        x, y,
        w: node.width as f32,
        h: node.height as f32,
        shape: convert_shape(node.shape, &node.metadata),
        label: node.label.clone().unwrap_or_default(),
        label_align,
        fill,
        stroke,
        stroke_w,
        text_color: theme::MERMAID_NODE_TEXT,
        font_size: default_font,
        bold: false,
    }
}

fn convert_edge(
    edge: &LayoutEdge,
    db: &FlowchartDb,
    bx: f32,
    by: f32,
    edge_font: f32,
) -> Option<Edge> {
    // Skip degenerate edges that the layout couldn't route.
    if edge.bend_points.len() < 2 {
        return None;
    }
    let points: Vec<(f32, f32)> = edge
        .bend_points
        .iter()
        .map(|p| (p.x as f32 - bx, p.y as f32 - by))
        .collect();

    // Locate the originating flow-edge to query its annotation overrides.
    let flow_edge = db.edges().iter().find(|fe| {
        fe.id.as_deref() == Some(edge.id.as_str())
            || (fe.start == edge.source().unwrap_or("") && fe.end == edge.target().unwrap_or(""))
    });
    let ov = flow_edge.map(|fe| db.edge_annotation_overrides_for(fe));

    let line_color = ov
        .as_ref()
        .and_then(|o| o.line_color.as_deref().and_then(parse_hex))
        .unwrap_or(theme::MERMAID_EDGE);
    let line_w = ov
        .as_ref()
        .and_then(|o| o.line_width.map(|v| v as f32))
        .unwrap_or(theme::MERMAID_EDGE_W);
    let line_style = ov
        .as_ref()
        .and_then(|o| o.line_style.map(convert_line_style))
        .unwrap_or(LineStyle::Solid);
    let start_arrow = ov
        .as_ref()
        .and_then(|o| o.start_arrow.map(convert_arrow))
        .unwrap_or(Arrow::None);
    let end_arrow = ov
        .as_ref()
        .and_then(|o| o.end_arrow.map(convert_arrow))
        .unwrap_or(Arrow::Triangle);

    let label = match (&edge.label, edge.label_position) {
        (Some(text), Some(pos)) if !text.is_empty() => Some(EdgeLabel {
            x: (pos.x as f32 - bx) - (edge.label_width as f32) / 2.0,
            y: (pos.y as f32 - by) - (edge.label_height as f32) / 2.0,
            w: edge.label_width as f32,
            h: edge.label_height as f32,
            text: text.clone(),
            text_color: theme::MERMAID_EDGE_LABEL,
            font_size: edge_font,
        }),
        _ => None,
    };

    Some(Edge {
        points,
        line_color,
        line_w,
        line_style,
        start_arrow,
        end_arrow,
        label,
    })
}

// ---------------------------------------------------------------------------
// Small mapping helpers
// ---------------------------------------------------------------------------

fn node_origin(node: &LayoutNode, bx: f32, by: f32) -> (f32, f32) {
    let x = node.x.unwrap_or(0.0) as f32 - bx;
    let y = node.y.unwrap_or(0.0) as f32 - by;
    (x, y)
}

fn convert_shape(s: SNodeShape, metadata: &std::collections::HashMap<String, String>) -> Shape {
    match s {
        SNodeShape::Rectangle    => Shape::Rect,
        SNodeShape::RoundedRect  => Shape::RoundedRect,
        SNodeShape::Stadium      => Shape::Stadium,
        SNodeShape::Circle       => Shape::Circle,
        SNodeShape::DoubleCircle => Shape::DoubleCircle,
        SNodeShape::Ellipse      => Shape::Ellipse,
        SNodeShape::Diamond      => Shape::Diamond,
        SNodeShape::Hexagon      => Shape::Hexagon,
        SNodeShape::Cylinder     => Shape::Cylinder,
        SNodeShape::Subroutine   => Shape::Subroutine,
        SNodeShape::Trapezoid    => Shape::Trapezoid,
        SNodeShape::InvTrapezoid => Shape::InvTrapezoid,
        SNodeShape::LeanRight    => Shape::LeanRight,
        SNodeShape::LeanLeft     => Shape::LeanLeft,
        SNodeShape::Odd          => Shape::Odd,
        SNodeShape::HorizontalBar => Shape::HorizontalBar,
        SNodeShape::Custom       => {
            // Resolve @{ shape: name } against the registry. If the name is
            // unknown (typo or shape not shipped), fall back to a rectangle
            // so the node still appears on screen.
            let name = metadata.get("shape").map(|s| s.as_str()).unwrap_or("");
            match crate::mermaid::shape_def::registry().lookup(name) {
                Some(idx) => Shape::Custom(idx),
                None      => Shape::Rect,
            }
        }
    }
}

fn convert_align(a: SLabelAlign) -> Align {
    match a {
        SLabelAlign::Left => Align::Left,
        SLabelAlign::Center => Align::Center,
        SLabelAlign::Right => Align::Right,
    }
}

fn convert_line_style(s: SLineStyle) -> LineStyle {
    match s {
        SLineStyle::Solid => LineStyle::Solid,
        SLineStyle::Dash => LineStyle::Dash,
        SLineStyle::Dot => LineStyle::Dot,
    }
}

fn convert_arrow(a: SArrow) -> Arrow {
    match a {
        SArrow::None => Arrow::None,
        SArrow::Point => Arrow::Triangle,
        SArrow::Circle => Arrow::Circle,
        SArrow::Cross => Arrow::Cross,
    }
}

/// Parse a CSS-like hex colour. Accepts `#RGB`, `#RRGGBB`, or the same without
/// the leading `#`. Returns `None` on any malformed input — the caller falls
/// back to a theme default.
fn parse_hex(s: &str) -> Option<u32> {
    let raw = s.trim().trim_start_matches('#');
    let v = match raw.len() {
        3 => {
            let r = u32::from_str_radix(&raw[0..1], 16).ok()?;
            let g = u32::from_str_radix(&raw[1..2], 16).ok()?;
            let b = u32::from_str_radix(&raw[2..3], 16).ok()?;
            ((r * 0x11) << 16) | ((g * 0x11) << 8) | (b * 0x11)
        }
        6 => u32::from_str_radix(raw, 16).ok()?,
        _ => return None,
    };
    Some(v & 0x00FF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_forms() {
        assert_eq!(parse_hex("#abc"), Some(0xAABBCC));
        assert_eq!(parse_hex("aabbcc"), Some(0xAABBCC));
        assert_eq!(parse_hex("#AABBCC"), Some(0xAABBCC));
        assert_eq!(parse_hex("zzz"), None);
    }
}
