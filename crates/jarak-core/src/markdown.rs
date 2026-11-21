//! The Markdown render model (`JARAK-FR-040`, `JARAK-SEC-001`): GitHub-flavoured Markdown parsed
//! with `pulldown-cmark` into blocks and inlines that the window draws natively (no web view).
//!
//! - Tables, task lists, strikethrough, footnotes and `<…>` autolinks come from the parser; bare
//!   `http://` / `https://` / `www.` URLs in text become links here (GFM's extended autolinks).
//! - Every top-level block remembers the source lines it came from, so the window can keep the
//!   reading position when it switches between the source and the preview.
//! - Raw HTML is shown as text, never interpreted; images are references to draw (Step 91), never
//!   fetched from the network here.
//! - Parsing is a pure function of the text: it never changes the document.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A block of the rendered document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        /// The first number of an ordered list, `None` for bullets.
        start: Option<u64>,
        items: Vec<Item>,
    },
    Quote(Vec<Block>),
    Code {
        lang: String,
        text: String,
    },
    Table {
        align: Vec<Align>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
    /// Raw HTML, shown as text.
    Html(String),
    Footnote {
        label: String,
        blocks: Vec<Block>,
    },
}

/// A list item; `task` is `Some(checked)` for `- [ ]` / `- [x]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

/// Column alignment of a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// Inline content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Code(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: String },
    SoftBreak,
    HardBreak,
    FootnoteRef(String),
    Html(String),
}

/// A top-level block with the source lines (0-based, inclusive start, exclusive end) it came
/// from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub block: Block,
    pub lines: std::ops::Range<usize>,
}

/// The parsed document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rendered {
    pub blocks: Vec<Located>,
}

impl Rendered {
    /// The index of the block that shows source line `line` (or the nearest one after it).
    pub fn block_for_line(&self, line: usize) -> Option<usize> {
        self.blocks
            .iter()
            .position(|b| b.lines.end > line)
            .or_else(|| self.blocks.len().checked_sub(1))
    }

    /// Every image reference, in order (for the asset tools of Steps 90–91).
    pub fn images(&self) -> Vec<String> {
        fn inl(v: &[Inline], out: &mut Vec<String>) {
            for i in v {
                match i {
                    Inline::Image { url, .. } => out.push(url.clone()),
                    Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => inl(c, out),
                    Inline::Link { children, .. } => inl(children, out),
                    _ => {}
                }
            }
        }
        fn blk(b: &Block, out: &mut Vec<String>) {
            match b {
                Block::Heading { inlines, .. } | Block::Paragraph(inlines) => inl(inlines, out),
                Block::List { items, .. } => {
                    for it in items {
                        for b in &it.blocks {
                            blk(b, out);
                        }
                    }
                }
                Block::Quote(bs) | Block::Footnote { blocks: bs, .. } => {
                    for b in bs {
                        blk(b, out);
                    }
                }
                Block::Table { header, rows, .. } => {
                    for c in header {
                        inl(c, out);
                    }
                    for r in rows {
                        for c in r {
                            inl(c, out);
                        }
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for b in &self.blocks {
            blk(&b.block, &mut out);
        }
        out
    }
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

/// Parses `text` (GitHub-flavoured Markdown).
pub fn parse(text: &str) -> Rendered {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line_of = |byte: usize| match line_starts.binary_search(&byte) {
        Ok(i) => i,
        Err(i) => i - 1,
    };
    let mut b = Builder::default();
    for (ev, range) in Parser::new_ext(text, options()).into_offset_iter() {
        let depth_before = b.stack.len();
        b.event(ev);
        // A top-level block just finished: give it its source lines.
        if (depth_before == 1 && b.stack.is_empty()) || (depth_before == 0 && b.top_added) {
            // The parser's range can include the blank lines after a block; they are not part of it.
            let bytes = text.as_bytes();
            let mut e = range.end;
            while e > range.start && matches!(bytes[e - 1], b'\r' | b'\n' | b' ' | b'\t') {
                e -= 1;
            }
            let start = line_of(range.start);
            let end = line_of(e.saturating_sub(1).max(range.start)) + 1;
            if let Some(last) = b.done.last_mut() {
                last.lines = start..end.max(start + 1);
            }
        }
        b.top_added = false;
    }
    Rendered { blocks: b.done }
}

/// What is being built.
#[derive(Debug)]
enum Frame {
    Heading(u8, Vec<Inline>),
    Paragraph(Vec<Inline>),
    Quote(Vec<Block>),
    List(Option<u64>, Vec<Item>),
    Item(Option<bool>, Vec<Block>, Vec<Inline>),
    Code(String, String),
    Table(Vec<Align>, Vec<Vec<Inline>>, Vec<Vec<Vec<Inline>>>, bool),
    Row(Vec<Vec<Inline>>),
    Cell(Vec<Inline>),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link(String, Vec<Inline>),
    Image(String, String),
    Footnote(String, Vec<Block>),
    Html(String),
}

#[derive(Debug, Default)]
struct Builder {
    stack: Vec<Frame>,
    done: Vec<Located>,
    top_added: bool,
}

impl Builder {
    fn event(&mut self, ev: Event<'_>) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(end) => self.end(end),
            Event::Text(t) => self.text(&t),
            Event::Code(t) => self.inline(Inline::Code(t.into_string())),
            Event::InlineMath(t) | Event::DisplayMath(t) => {
                self.inline(Inline::Code(t.into_string()));
            }
            Event::Html(t) => match self.stack.last_mut() {
                Some(Frame::Html(s)) => s.push_str(&t),
                _ => {
                    self.stack.push(Frame::Html(t.into_string()));
                }
            },
            Event::InlineHtml(t) => self.inline(Inline::Html(t.into_string())),
            Event::FootnoteReference(l) => self.inline(Inline::FootnoteRef(l.into_string())),
            Event::SoftBreak => self.inline(Inline::SoftBreak),
            Event::HardBreak => self.inline(Inline::HardBreak),
            Event::Rule => self.block(Block::Rule),
            Event::TaskListMarker(checked) => {
                if let Some(Frame::Item(task, _, _)) = self.stack.last_mut() {
                    *task = Some(checked);
                }
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        // Raw HTML that ended without an explicit end tag closes before the next block.
        if let Some(Frame::Html(_)) = self.stack.last() {
            self.close_html();
        }
        let f = match tag {
            Tag::Paragraph => Frame::Paragraph(Vec::new()),
            Tag::Heading { level, .. } => Frame::Heading(heading_level(level), Vec::new()),
            Tag::BlockQuote(_) => Frame::Quote(Vec::new()),
            Tag::CodeBlock(kind) => Frame::Code(
                match kind {
                    CodeBlockKind::Fenced(l) => {
                        l.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                },
                String::new(),
            ),
            Tag::List(start) => Frame::List(start, Vec::new()),
            Tag::Item => Frame::Item(None, Vec::new(), Vec::new()),
            Tag::Table(a) => Frame::Table(
                a.iter().map(|x| align(*x)).collect(),
                Vec::new(),
                Vec::new(),
                false,
            ),
            Tag::TableHead => Frame::Row(Vec::new()),
            Tag::TableRow => Frame::Row(Vec::new()),
            Tag::TableCell => Frame::Cell(Vec::new()),
            Tag::Emphasis => Frame::Emph(Vec::new()),
            Tag::Strong => Frame::Strong(Vec::new()),
            Tag::Strikethrough => Frame::Strike(Vec::new()),
            Tag::Link { dest_url, .. } => Frame::Link(dest_url.into_string(), Vec::new()),
            Tag::Image { dest_url, .. } => Frame::Image(dest_url.into_string(), String::new()),
            Tag::FootnoteDefinition(l) => Frame::Footnote(l.into_string(), Vec::new()),
            Tag::HtmlBlock => Frame::Html(String::new()),
            // Not produced with these options; kept as a paragraph so nothing is lost.
            _ => Frame::Paragraph(Vec::new()),
        };
        self.stack.push(f);
    }

    fn close_html(&mut self) {
        if let Some(Frame::Html(s)) = self.stack.pop() {
            self.block(Block::Html(s.trim_end().to_string()));
        }
    }

    fn end(&mut self, end: TagEnd) {
        if matches!(end, TagEnd::HtmlBlock) {
            self.close_html();
            return;
        }
        if let Some(Frame::Html(_)) = self.stack.last() {
            self.close_html();
        }
        let Some(f) = self.stack.pop() else {
            return;
        };
        match f {
            Frame::Paragraph(v) => {
                // Inside a tight list item the text goes straight into the item.
                self.block(Block::Paragraph(linkify(v)));
            }
            Frame::Heading(l, v) => self.block(Block::Heading {
                level: l,
                inlines: linkify(v),
            }),
            Frame::Quote(b) => self.block(Block::Quote(b)),
            Frame::Code(lang, text) => self.block(Block::Code { lang, text }),
            Frame::List(start, items) => self.block(Block::List { start, items }),
            Frame::Item(task, mut blocks, loose) => {
                if !loose.is_empty() {
                    blocks.insert(0, Block::Paragraph(linkify(loose)));
                }
                if let Some(Frame::List(_, items)) = self.stack.last_mut() {
                    items.push(Item { task, blocks });
                }
            }
            Frame::Table(align, header, rows, _) => self.block(Block::Table {
                align,
                header,
                rows,
            }),
            Frame::Row(cells) => {
                if let Some(Frame::Table(_, header, rows, seen_head)) = self.stack.last_mut() {
                    if !*seen_head && matches!(end, TagEnd::TableHead) {
                        *header = cells;
                        *seen_head = true;
                    } else {
                        rows.push(cells);
                    }
                }
            }
            Frame::Cell(v) => {
                if let Some(Frame::Row(cells)) = self.stack.last_mut() {
                    cells.push(linkify(v));
                }
            }
            Frame::Emph(v) => self.inline(Inline::Emph(v)),
            Frame::Strong(v) => self.inline(Inline::Strong(v)),
            Frame::Strike(v) => self.inline(Inline::Strike(v)),
            Frame::Link(url, v) => self.inline(Inline::Link { url, children: v }),
            Frame::Image(url, alt) => self.inline(Inline::Image { url, alt }),
            Frame::Footnote(label, blocks) => self.block(Block::Footnote { label, blocks }),
            Frame::Html(s) => self.block(Block::Html(s)),
        }
    }

    fn text(&mut self, t: &str) {
        match self.stack.last_mut() {
            Some(Frame::Code(_, s)) | Some(Frame::Html(s)) => s.push_str(t),
            Some(Frame::Image(_, alt)) => alt.push_str(t),
            _ => self.inline(Inline::Text(t.to_string())),
        }
    }

    fn inline(&mut self, i: Inline) {
        match self.stack.last_mut() {
            Some(
                Frame::Paragraph(v)
                | Frame::Heading(_, v)
                | Frame::Cell(v)
                | Frame::Emph(v)
                | Frame::Strong(v)
                | Frame::Strike(v)
                | Frame::Link(_, v),
            ) => v.push(i),
            Some(Frame::Item(_, _, v)) => v.push(i),
            Some(Frame::Image(_, alt)) => {
                if let Inline::Text(t) | Inline::Code(t) = i {
                    alt.push_str(&t);
                }
            }
            // Inline content outside any block (should not happen): its own paragraph.
            _ => self.block(Block::Paragraph(vec![i])),
        }
    }

    fn block(&mut self, b: Block) {
        match self.stack.last_mut() {
            Some(Frame::Quote(v) | Frame::Footnote(_, v)) => v.push(b),
            Some(Frame::Item(_, v, loose)) => {
                if !loose.is_empty() {
                    let p = Block::Paragraph(linkify(std::mem::take(loose)));
                    v.push(p);
                }
                v.push(b);
            }
            _ => {
                self.done.push(Located {
                    block: b,
                    lines: 0..1,
                });
                self.top_added = true;
            }
        }
    }
}

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn align(a: Alignment) -> Align {
    match a {
        Alignment::None => Align::None,
        Alignment::Left => Align::Left,
        Alignment::Center => Align::Center,
        Alignment::Right => Align::Right,
    }
}

/// Bare URLs in text become links (GFM extended autolinks: `http://`, `https://`, `www.`); text
/// inside links and code is left alone.
fn linkify(v: Vec<Inline>) -> Vec<Inline> {
    let mut out = Vec::with_capacity(v.len());
    for i in v {
        match i {
            Inline::Text(t) => split_urls(&t, &mut out),
            Inline::Emph(c) => out.push(Inline::Emph(linkify(c))),
            Inline::Strong(c) => out.push(Inline::Strong(linkify(c))),
            Inline::Strike(c) => out.push(Inline::Strike(linkify(c))),
            other => out.push(other),
        }
    }
    out
}

fn split_urls(t: &str, out: &mut Vec<Inline>) {
    let mut rest = t;
    loop {
        let found = ["https://", "http://", "www."]
            .iter()
            .filter_map(|p| rest.find(p).map(|i| (i, *p)))
            .filter(|(i, _)| {
                // Only at a word start.
                *i == 0
                    || rest[..*i]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_whitespace() || "(<*_~".contains(c))
            })
            .min_by_key(|(i, _)| *i);
        let Some((at, prefix)) = found else {
            if !rest.is_empty() {
                out.push(Inline::Text(rest.to_string()));
            }
            return;
        };
        let end = rest[at..]
            .find(|c: char| c.is_whitespace() || c == '<')
            .map_or(rest.len(), |e| at + e);
        // Trailing punctuation is not part of the URL (GFM's rule for `.,:;!?` and an unmatched `)`).
        let mut url_end = end;
        while url_end > at + prefix.len() {
            let last = rest[..url_end].chars().next_back().unwrap_or(' ');
            let unmatched_paren = last == ')'
                && rest[at..url_end].matches('(').count() < rest[at..url_end].matches(')').count();
            if ".,:;!?\"'".contains(last) || unmatched_paren {
                url_end -= last.len_utf8();
            } else {
                break;
            }
        }
        if url_end <= at + prefix.len() {
            out.push(Inline::Text(rest[..end].to_string()));
            rest = &rest[end..];
            continue;
        }
        if at > 0 {
            out.push(Inline::Text(rest[..at].to_string()));
        }
        let shown = &rest[at..url_end];
        let url = if prefix == "www." {
            format!("http://{shown}")
        } else {
            shown.to_string()
        };
        out.push(Inline::Link {
            url,
            children: vec![Inline::Text(shown.to_string())],
        });
        rest = &rest[url_end..];
    }
}

/// The plain text of inlines (for alt texts, links, search in the preview later).
pub fn plain(v: &[Inline]) -> String {
    let mut s = String::new();
    for i in v {
        match i {
            Inline::Text(t) | Inline::Code(t) | Inline::Html(t) => s.push_str(t),
            Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => s.push_str(&plain(c)),
            Inline::Link { children, .. } => s.push_str(&plain(children)),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::SoftBreak => s.push(' '),
            Inline::HardBreak => s.push('\n'),
            Inline::FootnoteRef(l) => s.push_str(&format!("[^{l}]")),
        }
    }
    s
}

#[cfg(test)]
mod tests;
