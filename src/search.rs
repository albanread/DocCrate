// Heading search over parsed documents. The "index" is the warm parse cache;
// no separate search structure is maintained yet.

use crate::parser::{Block, Inline};

pub const MAX_RESULTS: usize = 50;

#[derive(Clone)]
pub struct Hit {
    pub file_idx: usize,
    pub kind: HitKind,
    pub label: String,
    pub heading_text: Option<String>,
}

#[derive(Clone, Copy)]
pub enum HitKind {
    Heading { level: u8 },
}

pub fn search_doc(file_idx: usize, blocks: &[Block], q_lower: &str, headings: &mut Vec<Hit>) {
    for block in blocks {
        search_block(file_idx, block, q_lower, headings);
    }
}

fn search_block(file_idx: usize, block: &Block, q: &str, headings: &mut Vec<Hit>) {
    match block {
        Block::Located { block, .. } => search_block(file_idx, block, q, headings),
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
        Block::Blockquote(inner) => {
            for block in inner {
                search_block(file_idx, block, q, headings);
            }
        }
        _ => {}
    }
}

fn inlines_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t)
            | Inline::Bold(t)
            | Inline::Italic(t)
            | Inline::BoldItalic(t)
            | Inline::Code(t) => s.push_str(t),
            Inline::Link { text, .. } => s.push_str(text),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::SoftBreak | Inline::HardBreak => s.push(' '),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_setext_headings_but_not_body_text() {
        let blocks = crate::parser::parse(
            "Version 1.95.0 (2026-04-16)\n===========================\n\ncompiler body text\n",
        );

        let mut hits = Vec::new();
        search_doc(0, &blocks, "version 1.95", &mut hits);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, "Version 1.95.0 (2026-04-16)");

        hits.clear();
        search_doc(0, &blocks, "compiler", &mut hits);
        assert!(hits.is_empty());
    }
}
