#![allow(dead_code)]
#![allow(clippy::boxed_local)]
#![forbid(unsafe_code)]

pub const LEAF_MAX: usize = 2048;
pub const NEWLINE: u32 = 0x0A;

#[derive(Debug)]
pub enum Node {
    Leaf {
        buf: Vec<u32>,
        newline_count: usize,
    },
    Branch {
        left: Box<Node>,
        right: Box<Node>,
        left_len: usize,
        total_len: usize,
        total_newlines: usize,
        height: u32,
    },
}

impl Node {
    fn leaf(buf: Vec<u32>) -> Box<Node> {
        let newline_count = count_newlines(&buf);
        Box::new(Node::Leaf { buf, newline_count })
    }

    fn empty_leaf() -> Box<Node> {
        Self::leaf(Vec::new())
    }

    fn branch(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let l_len = left.text_len();
        let r_len = right.text_len();
        let l_nl = left.newline_count();
        let r_nl = right.newline_count();
        let l_h = left.height();
        let r_h = right.height();
        Box::new(Node::Branch {
            left,
            right,
            left_len: l_len,
            total_len: l_len + r_len,
            total_newlines: l_nl + r_nl,
            height: 1 + l_h.max(r_h),
        })
    }

    pub fn text_len(&self) -> usize {
        match self {
            Node::Leaf { buf, .. } => buf.len(),
            Node::Branch { total_len, .. } => *total_len,
        }
    }

    pub fn newline_count(&self) -> usize {
        match self {
            Node::Leaf { newline_count, .. } => *newline_count,
            Node::Branch { total_newlines, .. } => *total_newlines,
        }
    }

    pub fn line_count(&self) -> usize {
        self.newline_count() + 1
    }

    pub fn height(&self) -> u32 {
        match self {
            Node::Leaf { .. } => 0,
            Node::Branch { height, .. } => *height,
        }
    }

    fn balance_factor(&self) -> i32 {
        match self {
            Node::Leaf { .. } => 0,
            Node::Branch { left, right, .. } => left.height() as i32 - right.height() as i32,
        }
    }
}

fn count_newlines(buf: &[u32]) -> usize {
    buf.iter().filter(|&&cp| cp == NEWLINE).count()
}

#[derive(Debug)]
pub struct RopeBuffer {
    root: Box<Node>,
}

impl Default for RopeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RopeBuffer {
    pub fn new() -> Self {
        RopeBuffer {
            root: Node::empty_leaf(),
        }
    }

    pub fn from_slice(text: &[u32]) -> Self {
        RopeBuffer {
            root: build_rope_from_slice(text),
        }
    }

    pub fn from_utf8(utf8: &[u8]) -> Self {
        Self::from_slice(&utf8_to_codepoints(utf8))
    }

    pub fn len(&self) -> usize {
        self.root.text_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn line_count(&self) -> usize {
        self.root.line_count()
    }

    pub fn newline_count(&self) -> usize {
        self.root.newline_count()
    }

    pub fn height(&self) -> u32 {
        self.root.height()
    }

    pub fn char_at(&self, pos: usize) -> Option<u32> {
        if pos >= self.len() {
            None
        } else {
            Some(char_at_node(&self.root, pos))
        }
    }

    pub fn line_start(&self, line_idx: usize) -> Option<usize> {
        if line_idx == 0 {
            return Some(0);
        }
        if line_idx >= self.line_count() {
            return None;
        }
        nth_newline_pos(&self.root, line_idx - 1)
    }

    pub fn line_range(&self, line_idx: usize) -> Option<(usize, usize)> {
        let total = self.line_count();
        if line_idx >= total {
            return None;
        }
        let start = if line_idx == 0 {
            0
        } else {
            nth_newline_pos(&self.root, line_idx - 1)?
        };
        let end = if line_idx + 1 < total {
            nth_newline_pos_raw(&self.root, line_idx).unwrap_or_else(|| self.len())
        } else {
            self.len()
        };
        Some((start, end))
    }

    pub fn slice(&self, start: usize, end: usize) -> Vec<u32> {
        let total = self.len();
        let s = start.min(total);
        let e = end.min(total);
        if s >= e {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(e - s);
        collect_range(&self.root, s, e, &mut out);
        out
    }

    pub fn get_line(&self, line_idx: usize) -> Vec<u32> {
        match self.line_range(line_idx) {
            Some((s, e)) => self.slice(s, e),
            None => Vec::new(),
        }
    }

    pub fn to_utf8(&self) -> String {
        let cps = self.slice(0, self.len());
        codepoints_to_utf8(&cps)
    }
}

fn build_rope_from_slice(text: &[u32]) -> Box<Node> {
    if text.is_empty() {
        return Node::empty_leaf();
    }
    let chunk_count = text.len().div_ceil(LEAF_MAX);
    if chunk_count == 1 {
        return Node::leaf(text.to_vec());
    }

    let mut nodes: Vec<Box<Node>> = (0..chunk_count)
        .map(|i| {
            let start = i * LEAF_MAX;
            let end = (start + LEAF_MAX).min(text.len());
            Node::leaf(text[start..end].to_vec())
        })
        .collect();

    while nodes.len() > 1 {
        let mut next: Vec<Box<Node>> = Vec::with_capacity(nodes.len().div_ceil(2));
        let mut iter = nodes.into_iter();
        loop {
            match (iter.next(), iter.next()) {
                (Some(l), Some(r)) => next.push(Node::branch(l, r)),
                (Some(l), None) => next.push(l),
                _ => break,
            }
        }
        nodes = next;
    }
    nodes.pop().expect("at least one rope node")
}

fn char_at_node(node: &Node, pos: usize) -> u32 {
    match node {
        Node::Leaf { buf, .. } => buf[pos],
        Node::Branch {
            left,
            right,
            left_len,
            ..
        } => {
            if pos < *left_len {
                char_at_node(left, pos)
            } else {
                char_at_node(right, pos - *left_len)
            }
        }
    }
}

fn collect_range(node: &Node, start: usize, end: usize, out: &mut Vec<u32>) {
    if start >= end {
        return;
    }
    match node {
        Node::Leaf { buf, .. } => {
            let s = start.min(buf.len());
            let e = end.min(buf.len());
            out.extend_from_slice(&buf[s..e]);
        }
        Node::Branch {
            left,
            right,
            left_len,
            ..
        } => {
            if start < *left_len {
                collect_range(left, start, end.min(*left_len), out);
            }
            if end > *left_len {
                let r_start = start.saturating_sub(*left_len);
                let r_end = end - *left_len;
                collect_range(right, r_start, r_end, out);
            }
        }
    }
}

fn split_node(node: Box<Node>, pos: usize) -> (Box<Node>, Box<Node>) {
    let node_len = node.text_len();
    if pos == 0 {
        return (Node::empty_leaf(), node);
    }
    if pos >= node_len {
        return (node, Node::empty_leaf());
    }
    match *node {
        Node::Leaf { mut buf, .. } => {
            let right_vec = buf.split_off(pos);
            (Node::leaf(buf), Node::leaf(right_vec))
        }
        Node::Branch {
            left,
            right,
            left_len,
            ..
        } => {
            if pos == left_len {
                (left, right)
            } else if pos < left_len {
                let (ll, lr) = split_node(left, pos);
                let new_right = join_nodes(lr, right);
                (ll, new_right)
            } else {
                let (rl, rr) = split_node(right, pos - left_len);
                let new_left = join_nodes(left, rl);
                (new_left, rr)
            }
        }
    }
}

fn join_nodes(left: Box<Node>, right: Box<Node>) -> Box<Node> {
    if left.text_len() == 0 {
        return right;
    }
    if right.text_len() == 0 {
        return left;
    }
    if matches!(*left, Node::Leaf { .. }) && matches!(*right, Node::Leaf { .. }) {
        let (Node::Leaf { buf: lbuf, .. }, Node::Leaf { buf: rbuf, .. }) = (&*left, &*right) else {
            unreachable!()
        };
        if lbuf.len() + rbuf.len() <= LEAF_MAX {
            let mut merged = Vec::with_capacity(lbuf.len() + rbuf.len());
            merged.extend_from_slice(lbuf);
            merged.extend_from_slice(rbuf);
            return Node::leaf(merged);
        }
    }

    let lh = left.height();
    let rh = right.height();
    if lh.abs_diff(rh) <= 1 {
        return Node::branch(left, right);
    }

    if lh > rh {
        let (ll, lr) = take_children(left);
        let new_right = join_nodes(lr, right);
        balance(Node::branch(ll, new_right))
    } else {
        let (rl, rr) = take_children(right);
        let new_left = join_nodes(left, rl);
        balance(Node::branch(new_left, rr))
    }
}

fn balance(node: Box<Node>) -> Box<Node> {
    let bf = node.balance_factor();
    if bf > 1 {
        let (left, right) = take_children(node);
        if left.balance_factor() < 0 {
            let new_left = rotate_left(left);
            rotate_right(Node::branch(new_left, right))
        } else {
            rotate_right(Node::branch(left, right))
        }
    } else if bf < -1 {
        let (left, right) = take_children(node);
        if right.balance_factor() > 0 {
            let new_right = rotate_right(right);
            rotate_left(Node::branch(left, new_right))
        } else {
            rotate_left(Node::branch(left, right))
        }
    } else {
        node
    }
}

fn take_children(node: Box<Node>) -> (Box<Node>, Box<Node>) {
    match *node {
        Node::Branch { left, right, .. } => (left, right),
        Node::Leaf { .. } => panic!("take_children called on a leaf"),
    }
}

fn rotate_right(node: Box<Node>) -> Box<Node> {
    let Node::Branch {
        left,
        right: node_r,
        ..
    } = *node
    else {
        return node_back_from(*node);
    };
    let Node::Branch {
        left: ll,
        right: lr,
        ..
    } = *left
    else {
        return Node::branch(left, node_r);
    };
    let new_right = Node::branch(lr, node_r);
    Node::branch(ll, new_right)
}

fn rotate_left(node: Box<Node>) -> Box<Node> {
    let Node::Branch {
        left: node_l,
        right,
        ..
    } = *node
    else {
        return node_back_from(*node);
    };
    let Node::Branch {
        left: rl,
        right: rr,
        ..
    } = *right
    else {
        return Node::branch(node_l, right);
    };
    let new_left = Node::branch(node_l, rl);
    Node::branch(new_left, rr)
}

fn node_back_from(n: Node) -> Box<Node> {
    Box::new(n)
}

fn nth_newline_pos(node: &Node, n: usize) -> Option<usize> {
    nth_newline_pos_raw(node, n).map(|p| p + 1)
}

fn nth_newline_pos_raw(node: &Node, n: usize) -> Option<usize> {
    match node {
        Node::Leaf { buf, .. } => {
            let mut count = 0usize;
            for (i, &cp) in buf.iter().enumerate() {
                if cp == NEWLINE {
                    if count == n {
                        return Some(i);
                    }
                    count += 1;
                }
            }
            None
        }
        Node::Branch {
            left,
            right,
            left_len,
            ..
        } => {
            let left_nl = left.newline_count();
            if n < left_nl {
                nth_newline_pos_raw(left, n)
            } else {
                let right_pos = nth_newline_pos_raw(right, n - left_nl)?;
                Some(*left_len + right_pos)
            }
        }
    }
}

pub fn utf8_to_codepoints(utf8: &[u8]) -> Vec<u32> {
    let mut result: Vec<u32> = Vec::with_capacity(utf8.len());
    let mut i = 0usize;

    if utf8.len() >= 3 && utf8[0] == 0xEF && utf8[1] == 0xBB && utf8[2] == 0xBF {
        i = 3;
    }

    while i < utf8.len() {
        let byte = utf8[i];
        if byte == b'\r' {
            result.push(NEWLINE);
            if i + 1 < utf8.len() && utf8[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if byte < 0x80 {
            result.push(byte as u32);
            i += 1;
        } else if byte & 0xE0 == 0xC0 {
            if i + 1 < utf8.len() && utf8[i + 1] & 0xC0 == 0x80 {
                let cp = ((byte & 0x1F) as u32) << 6 | (utf8[i + 1] & 0x3F) as u32;
                result.push(cp);
                i += 2;
            } else {
                result.push(0xFFFD);
                i += 1;
            }
        } else if byte & 0xF0 == 0xE0 {
            if i + 2 < utf8.len() && utf8[i + 1] & 0xC0 == 0x80 && utf8[i + 2] & 0xC0 == 0x80 {
                let cp = ((byte & 0x0F) as u32) << 12
                    | ((utf8[i + 1] & 0x3F) as u32) << 6
                    | (utf8[i + 2] & 0x3F) as u32;
                result.push(cp);
                i += 3;
            } else {
                result.push(0xFFFD);
                i += 1;
            }
        } else if byte & 0xF8 == 0xF0 {
            if i + 3 < utf8.len()
                && utf8[i + 1] & 0xC0 == 0x80
                && utf8[i + 2] & 0xC0 == 0x80
                && utf8[i + 3] & 0xC0 == 0x80
            {
                let cp = ((byte & 0x07) as u32) << 18
                    | ((utf8[i + 1] & 0x3F) as u32) << 12
                    | ((utf8[i + 2] & 0x3F) as u32) << 6
                    | (utf8[i + 3] & 0x3F) as u32;
                result.push(cp);
                i += 4;
            } else {
                result.push(0xFFFD);
                i += 1;
            }
        } else {
            result.push(0xFFFD);
            i += 1;
        }
    }
    result
}

pub fn codepoints_to_utf8(cps: &[u32]) -> String {
    let mut out = String::with_capacity(cps.len() * 2);
    for &cp in cps {
        match char::from_u32(cp) {
            Some(c) => out.push(c),
            None => out.push('\u{FFFD}'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_line_count() {
        let buf = RopeBuffer::from_utf8(b"A\r\nB\rC\nD");
        assert_eq!(buf.line_count(), 4);
        assert_eq!(buf.to_utf8(), "A\nB\nC\nD");
    }

    #[test]
    fn large_rope_stays_balanced() {
        let line = b"This is a reasonably long line for a rope-buffer balance test.\n";
        let mut bytes = Vec::with_capacity(line.len() * 1000);
        for _ in 0..1000 {
            bytes.extend_from_slice(line);
        }
        let buf = RopeBuffer::from_utf8(&bytes);
        assert_eq!(buf.line_count(), 1001);
        assert!(buf.height() <= 20, "height was {}", buf.height());
    }
}
