use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, HeadingLevel, CodeBlockKind};

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Code(String),
    Link { text: String, href: String },
    SoftBreak,
    HardBreak,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph(Vec<Inline>),
    CodeBlock { lang: String, code: String },
    Blockquote(Vec<Block>),
    BulletList(Vec<Vec<Inline>>),
    OrderedList { start: u64, items: Vec<Vec<Inline>> },
    ThematicBreak,
}

pub fn parse(md: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let events: Vec<Event> = Parser::new_ext(md, opts).collect();
    let mut pos = 0;
    parse_blocks(&events, &mut pos, None)
}

fn parse_blocks(events: &[Event], pos: &mut usize, end_tag: Option<TagEnd>) -> Vec<Block> {
    let mut blocks = Vec::new();
    while *pos < events.len() {
        match &events[*pos] {
            Event::End(t) if Some(t.clone()) == end_tag => {
                *pos += 1;
                return blocks;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                *pos += 1;
                let level = hl(*level);
                let inlines = parse_inlines(events, pos, TagEnd::Heading(heading_level(level)));
                blocks.push(Block::Heading { level, inlines });
            }
            Event::Start(Tag::Paragraph) => {
                *pos += 1;
                let inlines = parse_inlines(events, pos, TagEnd::Paragraph);
                if !inlines.is_empty() {
                    blocks.push(Block::Paragraph(inlines));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                *pos += 1;
                let mut code = String::new();
                while *pos < events.len() {
                    match &events[*pos] {
                        Event::Text(t) => { code.push_str(t); *pos += 1; }
                        Event::End(TagEnd::CodeBlock) => { *pos += 1; break; }
                        _ => { *pos += 1; }
                    }
                }
                // Strip trailing newline from code
                if code.ends_with('\n') { code.pop(); }
                blocks.push(Block::CodeBlock { lang, code });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                *pos += 1;
                let inner = parse_blocks(events, pos, Some(TagEnd::BlockQuote(None)));
                blocks.push(Block::Blockquote(inner));
            }
            Event::Start(Tag::List(start_num)) => {
                let ordered = start_num.is_some();
                let start = start_num.unwrap_or(1);
                *pos += 1;
                let items = parse_list_items(events, pos);
                if ordered {
                    blocks.push(Block::OrderedList { start, items });
                } else {
                    blocks.push(Block::BulletList(items));
                }
            }
            Event::Rule => {
                blocks.push(Block::ThematicBreak);
                *pos += 1;
            }
            Event::End(_) => {
                *pos += 1;
            }
            _ => { *pos += 1; }
        }
    }
    blocks
}

fn parse_list_items(events: &[Event], pos: &mut usize) -> Vec<Vec<Inline>> {
    let mut items = Vec::new();
    while *pos < events.len() {
        match &events[*pos] {
            Event::End(TagEnd::List(_)) => { *pos += 1; break; }
            Event::Start(Tag::Item) => {
                *pos += 1;
                let inlines = collect_item_inlines(events, pos);
                items.push(inlines);
            }
            _ => { *pos += 1; }
        }
    }
    items
}

fn collect_item_inlines(events: &[Event], pos: &mut usize) -> Vec<Inline> {
    let mut result = Vec::new();
    while *pos < events.len() {
        match &events[*pos] {
            Event::End(TagEnd::Item) => { *pos += 1; break; }
            Event::Start(Tag::Paragraph) => {
                *pos += 1;
                let mut inner = parse_inlines(events, pos, TagEnd::Paragraph);
                result.append(&mut inner);
            }
            _ => {
                // Bare inline in item
                let inline = parse_one_inline(events, pos);
                if let Some(i) = inline { result.push(i); }
            }
        }
    }
    result
}

fn parse_inlines(events: &[Event], pos: &mut usize, end: TagEnd) -> Vec<Inline> {
    let mut inlines = Vec::new();
    while *pos < events.len() {
        match &events[*pos] {
            Event::End(t) if *t == end => { *pos += 1; break; }
            _ => {
                let inline = parse_one_inline(events, pos);
                if let Some(i) = inline { inlines.push(i); }
            }
        }
    }
    inlines
}

fn parse_one_inline(events: &[Event], pos: &mut usize) -> Option<Inline> {
    match &events[*pos] {
        Event::Text(t) => {
            let s = t.to_string();
            *pos += 1;
            Some(Inline::Text(s))
        }
        Event::Code(t) => {
            let s = t.to_string();
            *pos += 1;
            Some(Inline::Code(s))
        }
        Event::SoftBreak => { *pos += 1; Some(Inline::SoftBreak) }
        Event::HardBreak => { *pos += 1; Some(Inline::HardBreak) }
        Event::Start(Tag::Strong) => {
            *pos += 1;
            let inlines = parse_inlines(events, pos, TagEnd::Strong);
            Some(Inline::Bold(collect_inline_text(&inlines)))
        }
        Event::Start(Tag::Emphasis) => {
            *pos += 1;
            let inlines = parse_inlines(events, pos, TagEnd::Emphasis);
            Some(Inline::Italic(collect_inline_text(&inlines)))
        }
        Event::Start(Tag::Link { dest_url, .. }) => {
            let href = dest_url.to_string();
            *pos += 1;
            let inlines = parse_inlines(events, pos, TagEnd::Link);
            let text = collect_inline_text(&inlines);
            Some(Inline::Link { text, href })
        }
        _ => { *pos += 1; None }
    }
}

fn collect_inline_text(inlines: &[Inline]) -> String {
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

fn hl(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn heading_level(n: u8) -> HeadingLevel {
    match n {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        _ => HeadingLevel::H6,
    }
}
