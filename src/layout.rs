use crate::parser::{Block, Inline};
use crate::theme;

/// A rendered region that responds to mouse clicks (hyperlinks).
#[derive(Clone)]
pub struct HitRegion {
    pub x0: f32, pub y0: f32, pub x1: f32, pub y1: f32,
    pub href: String,
}

/// A single draw command for the content area.
#[derive(Clone)]
pub enum DrawCmd {
    FillRect  { x: f32, y: f32, w: f32, h: f32, color: u32 },
    StrokeLine { x0: f32, y0: f32, x1: f32, y1: f32, color: u32 },
    Text {
        x: f32, y: f32,
        max_w: f32,
        text: String,
        font: String,
        size: f32,
        bold: bool,
        italic: bool,
        color: u32,
        underline: bool,
    },
}

pub struct Layout {
    pub cmds: Vec<DrawCmd>,
    pub hits: Vec<HitRegion>,
    pub total_h: f32,
}

struct Ctx {
    cmds: Vec<DrawCmd>,
    hits: Vec<HitRegion>,
    x_base: f32,
    width: f32,   // content width
    y: f32,
    indent: f32,
}

impl Ctx {
    fn new(x_base: f32, width: f32, y_start: f32) -> Self {
        Self { cmds: Vec::new(), hits: Vec::new(), x_base, width, y: y_start, indent: 0.0 }
    }

    fn push(&mut self, cmd: DrawCmd) { self.cmds.push(cmd); }

    fn text(&mut self, text: &str, x: f32, y: f32, max_w: f32,
        font: &str, size: f32, bold: bool, italic: bool, color: u32, underline: bool)
    {
        if text.is_empty() { return; }
        self.push(DrawCmd::Text {
            x, y, max_w, text: text.to_owned(),
            font: font.to_owned(), size, bold, italic, color, underline,
        });
    }

    fn line_h(&self, size: f32) -> f32 { size * theme::LINE_EXTRA }

    fn x(&self) -> f32 { self.x_base + self.indent }
    fn avail_w(&self) -> f32 { self.width - self.indent }
}

pub fn layout(blocks: &[Block], x_base: f32, width: f32) -> Layout {
    let mut ctx = Ctx::new(x_base, width, theme::V_PAD);
    layout_blocks(&mut ctx, blocks, 0);
    ctx.y += theme::V_PAD;
    Layout { cmds: ctx.cmds, hits: ctx.hits, total_h: ctx.y }
}

fn layout_blocks(ctx: &mut Ctx, blocks: &[Block], depth: usize) {
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 { ctx.y += theme::PARA_GAP; }
        layout_block(ctx, block, depth);
    }
}

fn layout_block(ctx: &mut Ctx, block: &Block, depth: usize) {
    match block {
        Block::Heading { level, inlines } => layout_heading(ctx, *level, inlines),
        Block::Paragraph(inlines) => layout_paragraph(ctx, inlines),
        Block::CodeBlock { lang, code } => layout_code(ctx, lang, code),
        Block::Blockquote(inner) => layout_blockquote(ctx, inner, depth),
        Block::BulletList(items) => layout_list(ctx, items, false, 1, depth),
        Block::OrderedList { start, items } => layout_list(ctx, items, true, *start as usize, depth),
        Block::ThematicBreak => layout_rule(ctx),
    }
}

fn layout_heading(ctx: &mut Ctx, level: u8, inlines: &[Inline]) {
    let (size, color, top_gap, bot_gap) = match level {
        1 => (theme::H1_SIZE, theme::H1, 24.0_f32, 8.0_f32),
        2 => (theme::H2_SIZE, theme::H2, 20.0, 6.0),
        3 => (theme::H3_SIZE, theme::H3, 16.0, 4.0),
        4 => (theme::H4_SIZE, theme::H4, 12.0, 3.0),
        5 => (theme::H5_SIZE, theme::H5, 10.0, 2.0),
        _ => (theme::H6_SIZE, theme::H6, 8.0, 2.0),
    };
    ctx.y += top_gap;

    // H1 and H2 get a subtle separator line below
    let text = collect_inlines_text(inlines);
    let x = ctx.x();
    let y = ctx.y;
    let max_w = ctx.avail_w();
    ctx.text(&text, x, y, max_w, theme::BODY_FONT, size, level <= 3, false, color, false);
    ctx.y += ctx.line_h(size);
    ctx.y += bot_gap;

    if level <= 2 {
        let lx = ctx.x();
        let lw = ctx.avail_w();
        ctx.push(DrawCmd::StrokeLine { x0: lx, y0: ctx.y, x1: lx + lw, y1: ctx.y, color: theme::BORDER });
        ctx.y += 1.0;
    }
}

fn layout_paragraph(ctx: &mut Ctx, inlines: &[Inline]) {
    // Lay out inlines word-wrapped. We approximate by emitting each inline as its own
    // text run with max_w set to remaining width; DirectWrite handles the wrapping.
    // For multi-inline paragraphs we do a simple flow layout.
    let y = ctx.y;
    let x = ctx.x();
    let max_w = ctx.avail_w();

    // Build flattened inline list with proper spacing
    let mut parts: Vec<(String, bool, bool, bool, Option<String>)> = Vec::new();
    // (text, bold, italic, underline, href)

    for inline in inlines {
        match inline {
            Inline::Text(t) => parts.push((t.clone(), false, false, false, None)),
            Inline::Bold(t) => parts.push((t.clone(), true, false, false, None)),
            Inline::Italic(t) => parts.push((t.clone(), false, true, false, None)),
            Inline::BoldItalic(t) => parts.push((t.clone(), true, true, false, None)),
            Inline::Code(t) => parts.push((format!(" {} ", t), false, false, false, None)),
            Inline::Link { text, href } => parts.push((text.clone(), false, false, true, Some(href.clone()))),
            Inline::SoftBreak | Inline::HardBreak => {
                if let Some(last) = parts.last_mut() {
                    last.0.push(' ');
                }
            }
        }
    }

    if parts.is_empty() { return; }

    // Check if all parts are uniform (common case) → single text run
    let all_same = parts.iter().all(|(_, b, i, u, h)| *b == parts[0].1 && *i == parts[0].2 && *u == parts[0].3 && h.is_none());
    if all_same && parts[0].4.is_none() {
        let text: String = parts.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join("");
        ctx.text(&text, x, y, max_w, theme::BODY_FONT, theme::BODY_SIZE, parts[0].1, parts[0].2, theme::TEXT, parts[0].3);
        // Estimate height: assume DWrite wraps
        let approx_lines = estimate_lines(&text, max_w, theme::BODY_SIZE);
        ctx.y += approx_lines * ctx.line_h(theme::BODY_SIZE);
        return;
    }

    // Mixed inline paragraph: emit each run. We can't do inline flow in Direct2D text easily,
    // so join into a single text run with the dominant style. Links get their own run.
    // Simple approach: concatenate non-link parts, emit link parts separately by measuring.
    let mut cur_x = x;
    let mut max_y = y;
    let line_h = ctx.line_h(theme::BODY_SIZE);

    for (text, bold, italic, underline, href) in &parts {
        let color = if href.is_some() { theme::LINK } else { theme::TEXT };
        let font = theme::BODY_FONT;
        let remaining_w = (x + max_w) - cur_x;

        if remaining_w < 20.0 {
            cur_x = x;
            max_y += line_h;
        }

        ctx.text(text, cur_x, max_y, (x + max_w) - cur_x, font, theme::BODY_SIZE, *bold, *italic, color, *underline);

        // Track hit regions for links
        if let Some(h) = href {
            let approx_w = (text.len() as f32 * theme::BODY_SIZE * 0.52).min((x + max_w) - cur_x);
            ctx.hits.push(HitRegion {
                x0: cur_x, y0: max_y,
                x1: cur_x + approx_w, y1: max_y + line_h,
                href: h.clone(),
            });
            cur_x += approx_w;
        } else {
            let approx_w = estimate_text_w(text, theme::BODY_SIZE);
            cur_x += approx_w;
            if cur_x > x + max_w {
                let lines = estimate_lines(text, max_w, theme::BODY_SIZE);
                max_y += (lines - 1.0) * line_h;
                cur_x = x + (cur_x - (x + max_w));
            }
        }
    }

    ctx.y = max_y + line_h;
}

fn layout_code(ctx: &mut Ctx, _lang: &str, code: &str) {
    let pad = theme::CODE_PAD;
    let x = ctx.x();
    let w = ctx.avail_w();
    let lines: Vec<&str> = code.lines().collect();
    let line_h = ctx.line_h(theme::CODE_SIZE);
    let block_h = lines.len() as f32 * line_h + pad * 2.0;

    // Background rect
    ctx.push(DrawCmd::FillRect { x, y: ctx.y, w, h: block_h, color: theme::CODE_BG });

    // Lines of code
    let text_x = x + pad;
    let text_max_w = w - pad * 2.0;
    for (i, line) in lines.iter().enumerate() {
        let ty = ctx.y + pad + i as f32 * line_h;
        ctx.text(line, text_x, ty, text_max_w, theme::CODE_FONT, theme::CODE_SIZE, false, false, theme::CODE_FG, false);
    }

    ctx.y += block_h;
}

fn layout_blockquote(ctx: &mut Ctx, inner: &[Block], depth: usize) {
    let bar_x = ctx.x();
    let y_start = ctx.y;

    ctx.indent += theme::BQ_BAR_W + theme::BQ_PAD;
    layout_blocks(ctx, inner, depth + 1);
    ctx.indent -= theme::BQ_BAR_W + theme::BQ_PAD;

    let y_end = ctx.y;
    ctx.push(DrawCmd::FillRect { x: bar_x, y: y_start, w: theme::BQ_BAR_W, h: y_end - y_start, color: theme::BLOCKQUOTE });
}

fn layout_list(ctx: &mut Ctx, items: &[Vec<Inline>], ordered: bool, start: usize, _depth: usize) {
    let bullet_x = ctx.x();
    ctx.indent += 24.0;

    for (i, item_inlines) in items.iter().enumerate() {
        // Bullet or number
        let marker = if ordered {
            format!("{}.", start + i)
        } else {
            "•".to_string()
        };
        let bx = bullet_x;
        let by = ctx.y;
        ctx.text(&marker, bx, by, 20.0, theme::BODY_FONT, theme::BODY_SIZE, false, false, theme::TEXT_DIM, false);

        layout_paragraph(ctx, item_inlines);
        ctx.y += 2.0;
    }

    ctx.indent -= 24.0;
}

fn layout_rule(ctx: &mut Ctx) {
    ctx.y += 8.0;
    let x = ctx.x();
    let w = ctx.avail_w();
    ctx.push(DrawCmd::FillRect { x, y: ctx.y, w, h: theme::H_RULE_H, color: theme::RULE });
    ctx.y += theme::H_RULE_H + 8.0;
}

fn collect_inlines_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::Text(t) | Inline::Bold(t) | Inline::Italic(t)
            | Inline::BoldItalic(t) | Inline::Code(t) => s.push_str(t),
            Inline::Link { text, .. } => s.push_str(text),
            Inline::SoftBreak | Inline::HardBreak => s.push(' '),
        }
    }
    s
}

// Rough approximation of wrapped line count for height pre-calculation.
fn estimate_lines(text: &str, max_w: f32, size: f32) -> f32 {
    let char_w = size * 0.52;
    let chars_per_line = (max_w / char_w).max(1.0);
    let raw = text.len() as f32 / chars_per_line;
    raw.max(1.0).ceil()
}

fn estimate_text_w(text: &str, size: f32) -> f32 {
    text.len() as f32 * size * 0.52
}
