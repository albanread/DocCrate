// Search across the parsed-document cache. Headings are surfaced before
// regular-text matches; the "index" is just the warm-up `parsed_cache` —
// no separate data structure is maintained.

use crate::parser::{Block, Inline};

const SNIPPET_BEFORE: usize = 30;
const SNIPPET_AFTER:  usize = 60;
pub  const MAX_RESULTS: usize = 50;

#[derive(Clone)]
pub struct Hit {
    pub file_idx: usize,
    pub kind:     HitKind,
    /// Display label (heading text, or text snippet around the match).
    pub label:    String,
    /// For heading hits: the original heading text, used by the renderer
    /// to scroll to the matching y-position after navigation.
    pub heading_text: Option<String>,
}

#[derive(Clone, Copy)]
pub enum HitKind {
    Heading { level: u8 },
    Text,
}

/// Search every block in one document. Heading hits go into `headings`,
/// everything else into `texts`.
pub fn search_doc(file_idx: usize, blocks: &[Block], q_lower: &str,
                  headings: &mut Vec<Hit>, texts: &mut Vec<Hit>) {
    for block in blocks {
        search_block(file_idx, block, q_lower, headings, texts);
    }
}

fn search_block(file_idx: usize, block: &Block, q: &str,
                headings: &mut Vec<Hit>, texts: &mut Vec<Hit>) {
    match block {
        Block::Heading { level, inlines } => {
            let text = inlines_text(inlines);
            if text.to_lowercase().contains(q) {
                headings.push(Hit {
                    file_idx,
                    kind: HitKind::Heading { level: *level },
                    label: text.clone(),
                    heading_text: Some(text),
                });
            }
        }
        Block::Paragraph(inlines) => {
            let text = inlines_text(inlines);
            if let Some(snippet) = make_snippet(&text, q) {
                texts.push(Hit { file_idx, kind: HitKind::Text, label: snippet, heading_text: None });
            }
        }
        Block::BulletList(items) => {
            for item in items {
                let text = inlines_text(item);
                if let Some(snippet) = make_snippet(&text, q) {
                    texts.push(Hit { file_idx, kind: HitKind::Text, label: snippet, heading_text: None });
                }
            }
        }
        Block::OrderedList { items, .. } => {
            for item in items {
                let text = inlines_text(item);
                if let Some(snippet) = make_snippet(&text, q) {
                    texts.push(Hit { file_idx, kind: HitKind::Text, label: snippet, heading_text: None });
                }
            }
        }
        Block::Blockquote(inner) => {
            for b in inner {
                search_block(file_idx, b, q, headings, texts);
            }
        }
        Block::CodeBlock { code, .. } => {
            if let Some(snippet) = make_snippet(code, q) {
                texts.push(Hit { file_idx, kind: HitKind::Text, label: snippet, heading_text: None });
            }
        }
        Block::Table { headers, rows, .. } => {
            for h in headers {
                if let Some(s) = make_snippet(h, q) {
                    texts.push(Hit { file_idx, kind: HitKind::Text, label: s, heading_text: None });
                }
            }
            for row in rows {
                for cell in row {
                    if let Some(s) = make_snippet(cell, q) {
                        texts.push(Hit { file_idx, kind: HitKind::Text, label: s, heading_text: None });
                    }
                }
            }
        }
        _ => {}
    }
}

fn inlines_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::Text(t) | Inline::Bold(t) | Inline::Italic(t)
            | Inline::BoldItalic(t) | Inline::Code(t) => s.push_str(t),
            Inline::Link { text, .. } => s.push_str(text),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::SoftBreak | Inline::HardBreak => s.push(' '),
        }
    }
    s
}

/// Returns a UTF-8-safe snippet around the first match, with ellipses if
/// the text was truncated on either side. Returns None if no match.
fn make_snippet(text: &str, q_lower: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let pos   = lower.find(q_lower)?;
    let end   = pos + q_lower.len();

    // Char-aligned window edges (lowercase byte-pos matches original for ASCII;
    // for non-ASCII the snippet may be slightly off but still UTF-8 valid).
    let start_byte = text.get(..pos)
        .and_then(|s| s.char_indices().rev().nth(SNIPPET_BEFORE).map(|(i, _)| i))
        .unwrap_or(0);
    let end_byte = text.get(end..)
        .and_then(|s| s.char_indices().nth(SNIPPET_AFTER).map(|(i, _)| end + i))
        .unwrap_or(text.len());

    let prefix = if start_byte > 0           { "\u{2026} " } else { "" };
    let suffix = if end_byte   < text.len()  { " \u{2026}" } else { "" };
    Some(format!("{}{}{}", prefix, &text[start_byte..end_byte], suffix))
}
