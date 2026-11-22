//! Step 100: typing aids (`JARAK-FR-095`, `JARAK-FR-096`) — closing brackets and quotes (typing
//! over a closer an aid inserted, Backspace removing an empty pair it inserted, wrapping a
//! selection), HTML/XML closing tags, the indentation Enter gives (off, keep, smart) and a closing
//! bracket's dedent, and snippets with tab stops. Each aid is a setting the window passes in
//! [`Aids`]; with all off, typing is plain [`Document::type_text`].

use super::{BOOKMARK, Document, Marker};
use crate::assist::Expanded;
use crate::history::Kind;
use crate::selection::{Range, Selection};

/// How Enter indents the new line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndentMode {
    /// No indentation.
    Off,
    /// The indentation of the line (the document's default).
    #[default]
    Keep,
    /// The line's, one level more after an opening bracket (or a `:` in Python and YAML), and a
    /// closing bracket typed on an empty line goes one level back.
    Smart,
}

/// The typing aids in effect (the window sets them from the settings and the language).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Aids {
    pub auto_close: bool,
    pub close_tags: bool,
    pub indent: IndentMode,
    /// The language's quotes that close (`"`, `'`, `` ` ``).
    pub quotes: String,
    /// A line ending in `:` opens a level (Python, YAML).
    pub colon_indents: bool,
    /// HTML or XML: `>` closes the tag.
    pub markup: bool,
    /// One indentation level when the document shows none (`\t` or spaces).
    pub unit: String,
}

/// An empty marker before a closer an aid inserted (typing it steps over; Backspace removes the
/// pair).
pub const AUTOCLOSE: u8 = 209;
/// Snippet tab stops: `SNIPPET + n` for stop `n` (0 the last; at most 40).
pub const SNIPPET: u8 = 210;

const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

fn closer_of(c: char) -> Option<char> {
    PAIRS.iter().find(|(o, _)| *o == c).map(|(_, k)| *k)
}

fn is_closer(c: char) -> bool {
    PAIRS.iter().any(|(_, k)| *k == c)
}

/// The name of the tag `before` (a line up to the caret) leaves open: `<div class="a"` → `div`.
fn open_tag(before: &str) -> Option<String> {
    let lt = before.rfind('<')?;
    let tag = &before[lt + 1..];
    if tag.starts_with(['/', '!', '?']) || tag.trim_end().ends_with('/') {
        return None;
    }
    let name: String = tag
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
        .collect();
    // The tag is still open: no `>` outside an attribute value, and no value left open.
    let mut quote = None;
    for c in tag[name.len()..].chars() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '>') => return None,
            _ => {}
        }
    }
    (!name.is_empty() && quote.is_none() && !VOID.contains(&name.to_lowercase().as_str()))
        .then_some(name)
}

impl Document {
    fn autoclose_at(&self, p: usize) -> bool {
        self.markers
            .iter()
            .any(|m| m.kind == AUTOCLOSE && m.start == p)
    }

    /// `change`, then each caret `back[i]` characters before where it would be.
    pub(crate) fn change_back(
        &mut self,
        parts: Vec<(usize, usize, String)>,
        kind: Kind,
        back: &[usize],
    ) {
        self.change(parts, kind);
        if back.iter().all(|b| *b == 0) || back.len() != self.sel.ranges().len() {
            return;
        }
        let ranges: Vec<Range> = self
            .sel
            .ranges()
            .iter()
            .zip(back)
            .map(|(r, b)| Range::point(r.head.saturating_sub(*b)))
            .collect();
        let sel = Selection::new(ranges, self.sel.primary_index());
        self.history.set_last_after(sel.clone());
        self.sel = sel;
    }

    /// Typing `s` with the aids: one character may step over a closer, open a pair, wrap the
    /// selection, close a tag or take back a level; anything else is [`Document::type_text`].
    pub fn type_assisted(&mut self, s: &str) {
        let mut it = s.chars();
        let (Some(c), None) = (it.next(), it.next()) else {
            return self.type_text(s);
        };
        let a = self.aids.clone();
        let ranges = self.sel.ranges().to_vec();
        let n = self.len_chars();
        let text = &self.text;
        let next = |p: usize| (p < n).then(|| text.char(p));
        let prev = |p: usize| (p > 0).then(|| text.char(p - 1));
        let quote = a.quotes.contains(c);
        let empty = ranges.iter().all(Range::is_empty);
        // Over a closer an aid inserted: step over it.
        if a.auto_close
            && empty
            && (is_closer(c) || quote)
            && ranges
                .iter()
                .all(|r| next(r.head) == Some(c) && self.autoclose_at(r.head))
        {
            let heads: Vec<usize> = ranges.iter().map(|r| r.head).collect();
            self.markers
                .retain(|m| m.kind != AUTOCLOSE || !heads.contains(&m.start));
            let moved: Vec<Range> = heads.iter().map(|h| Range::point(h + 1)).collect();
            self.set_selection(&Selection::new(moved, self.sel.primary_index()));
            return;
        }
        let close = closer_of(c).or(quote.then_some(c));
        // Wrap each selection.
        if a.auto_close
            && let Some(k) = close
            && ranges.iter().all(|r| !r.is_empty())
        {
            let mut parts = Vec::new();
            for r in &ranges {
                parts.push((r.from(), r.from(), c.to_string()));
                parts.push((r.to(), r.to(), k.to_string()));
            }
            let before = self.sel.clone();
            self.change(parts, Kind::Other);
            // The wrapped text stays selected.
            let mut shift = 0;
            let inner: Vec<Range> = before
                .ranges()
                .iter()
                .map(|r| {
                    let (f, t) = (r.from() + shift + 1, r.to() + shift + 1);
                    shift += 2;
                    if r.head >= r.anchor {
                        Range::new(f, t)
                    } else {
                        Range::new(t, f)
                    }
                })
                .collect();
            let sel = Selection::new(inner, before.primary_index());
            self.history.set_last_after(sel.clone());
            self.sel = sel;
            return;
        }
        // Open a pair: before whitespace, a closer or the end; a quote not after a word or the
        // same quote.
        if a.auto_close
            && empty
            && let Some(k) = close
        {
            let fits = ranges.iter().all(|r| {
                let after_ok = next(r.head)
                    .is_none_or(|x| x.is_whitespace() || is_closer(x) || matches!(x, ',' | ';'));
                let before_ok = !quote
                    || prev(r.head).is_none_or(|x| !(x.is_alphanumeric() || x == '_' || x == c));
                after_ok && before_ok
            });
            if fits {
                let lines: Vec<usize> = ranges
                    .iter()
                    .map(|r| self.text.char_to_line(r.head))
                    .collect();
                // Forget the closers of other lines (only the line being typed steps over).
                let text = &self.text;
                self.markers.retain(|m| {
                    m.kind != AUTOCLOSE
                        || lines.contains(&text.char_to_line(m.start.min(text.len_chars())))
                });
                let parts = ranges
                    .iter()
                    .map(|r| (r.head, r.head, format!("{c}{k}")))
                    .collect();
                self.change_back(parts, Kind::Typing, &vec![1; ranges.len()]);
                let heads: Vec<usize> = self.sel.ranges().iter().map(|r| r.head).collect();
                for h in heads {
                    self.markers.push(Marker {
                        start: h,
                        end: h,
                        kind: AUTOCLOSE,
                    });
                }
                return;
            }
        }
        // `>` after `<name …`: the closing tag after the caret.
        if a.close_tags && a.markup && c == '>' && ranges.len() == 1 && empty {
            let p = ranges[0].head;
            let start = self.text.line_to_char(self.text.char_to_line(p));
            let before = self.text.slice(start..p).to_string();
            if let Some(name) = open_tag(&before) {
                let closing = format!("</{name}>");
                let back = closing.chars().count();
                self.change_back(vec![(p, p, format!(">{closing}"))], Kind::Typing, &[back]);
                return;
            }
        }
        // A closing bracket on a line of only indentation: one level back.
        if a.indent == IndentMode::Smart && is_closer(c) && ranges.len() == 1 && empty {
            let p = ranges[0].head;
            let start = self.text.line_to_char(self.text.char_to_line(p));
            let ws = self.text.slice(start..p).to_string();
            if !ws.is_empty() && ws.chars().all(|x| x == ' ' || x == '\t') {
                let unit = self.indent_unit();
                let less = ws
                    .strip_suffix(unit.as_str())
                    .or_else(|| ws.strip_suffix('\t'))
                    .unwrap_or(&ws)
                    .to_string();
                if less != ws {
                    self.change(vec![(start, p, format!("{less}{c}"))], Kind::Typing);
                    return;
                }
            }
        }
        self.type_text(s);
    }

    /// Backspace with the aids: between an empty pair an aid inserted, both go.
    pub fn delete_backward_assisted(&mut self) {
        let n = self.len_chars();
        let pair_here = |d: &Document, p: usize| {
            p > 0 && p < n && d.autoclose_at(p) && {
                let (o, k) = (d.text.char(p - 1), d.text.char(p));
                closer_of(o) == Some(k) || (o == k && d.aids.quotes.contains(o))
            }
        };
        if self.aids.auto_close
            && self
                .sel
                .ranges()
                .iter()
                .all(|r| r.is_empty() && pair_here(self, r.head))
        {
            let heads: Vec<usize> = self.sel.ranges().iter().map(|r| r.head).collect();
            self.markers
                .retain(|m| m.kind != AUTOCLOSE || !heads.contains(&m.start));
            let parts = heads
                .iter()
                .map(|h| (h - 1, h + 1, String::new()))
                .collect();
            self.change(parts, Kind::Deleting);
            return;
        }
        self.delete_backward();
    }

    /// One indentation level of this document: a tab if its first indented lines use tabs, else
    /// the smallest run of leading spaces there (2 or more); without indented lines the aids'
    /// unit (a tab by default).
    pub fn indent_unit(&self) -> String {
        let mut spaces = usize::MAX;
        for line in self.text.lines().take(2000) {
            let mut it = line.chars();
            match it.next() {
                Some('\t') => return "\t".into(),
                Some(' ') => {
                    let k = 1 + it.take_while(|c| *c == ' ').count();
                    if k >= 2 && line.chars().nth(k).is_some_and(|c| !c.is_whitespace()) {
                        spaces = spaces.min(k);
                    }
                }
                _ => {}
            }
        }
        if spaces != usize::MAX {
            " ".repeat(spaces)
        } else if self.aids.unit.is_empty() {
            "\t".into()
        } else {
            self.aids.unit.clone()
        }
    }

    /// What Enter inserts at `from..to` (and how far back the caret goes): the line ending and the
    /// indentation of [`Aids::indent`].
    pub(crate) fn newline_text(&self, from: usize, to: usize) -> (String, usize) {
        let eol = self.eol.as_str();
        let line = self.text.char_to_line(from);
        let start = self.text.line_to_char(line);
        let head = self.text.slice(start..from).to_string();
        let indent: String = head
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        match self.aids.indent {
            IndentMode::Off => (eol.to_string(), 0),
            IndentMode::Keep => (format!("{eol}{indent}"), 0),
            IndentMode::Smart => {
                let last = head.trim_end().chars().last();
                let opens = last.is_some_and(|c| closer_of(c).is_some())
                    || (self.aids.colon_indents && last == Some(':'));
                if !opens {
                    return (format!("{eol}{indent}"), 0);
                }
                let unit = self.indent_unit();
                // Between a pair (`{|}`): the closer goes to a line of its own.
                let end =
                    self.text.line_to_char(line) + crate::text::line_len(self.text.line(line));
                let after = self.text.slice(to.min(end)..end).to_string();
                let pair = last
                    .and_then(closer_of)
                    .is_some_and(|k| after.trim_start().starts_with(k));
                if pair {
                    let tail = format!("{eol}{indent}");
                    (format!("{eol}{indent}{unit}{tail}"), tail.chars().count())
                } else {
                    (format!("{eol}{indent}{unit}"), 0)
                }
            }
        }
    }

    // ---- snippets ------------------------------------------------------------------------------

    /// Replaces `from..to` (the typed prefix) with the expanded snippet and selects its first
    /// stop (all of its places).
    pub fn insert_snippet(&mut self, from: usize, to: usize, e: &Expanded) {
        self.clear_snippet();
        self.change(vec![(from, to, e.text.clone())], Kind::Other);
        for (n, ranges) in &e.stops {
            let kind = SNIPPET + (*n).min(40) as u8;
            for (a, b) in ranges {
                self.markers.push(Marker {
                    start: from + a,
                    end: from + b,
                    kind,
                });
            }
        }
        self.snippet = e.stops.iter().map(|(n, _)| (*n).min(40) as u8).collect();
        self.snippet_at = 0;
        self.snippet_select();
        let sel = self.sel.clone();
        self.history.set_last_after(sel);
    }

    /// Whether a snippet's tab stops are being filled.
    pub fn snippet_active(&self) -> bool {
        !self.snippet.is_empty()
    }

    /// Tab / Shift+Tab in a snippet: the next (previous) stop; the last one (`$0`) ends it.
    pub fn snippet_step(&mut self, back: bool) -> bool {
        if self.snippet.is_empty() {
            return false;
        }
        if back {
            self.snippet_at = self.snippet_at.saturating_sub(1);
        } else {
            self.snippet_at += 1;
        }
        self.snippet_select();
        true
    }

    /// Ends the snippet (its stops are forgotten).
    pub fn clear_snippet(&mut self) {
        self.snippet.clear();
        self.snippet_at = 0;
        self.markers
            .retain(|m| m.kind < SNIPPET || m.kind > SNIPPET + 40);
    }

    fn snippet_select(&mut self) {
        while self.snippet_at < self.snippet.len() {
            let kind = SNIPPET + self.snippet[self.snippet_at];
            let mut ranges: Vec<Range> = self
                .markers
                .iter()
                .filter(|m| m.kind == kind)
                .map(|m| Range::new(m.start, m.end))
                .collect();
            if ranges.is_empty() {
                self.snippet_at += 1;
                continue;
            }
            ranges.sort_by_key(|r| r.from());
            ranges.dedup_by_key(|r| r.from());
            let last = kind == SNIPPET;
            self.sel = Selection::new(ranges, 0);
            if last {
                self.clear_snippet();
            }
            return;
        }
        self.clear_snippet();
    }
}

// Keep the marker kinds apart from the bookmark's.
const _: () = assert!(AUTOCLOSE > BOOKMARK && SNIPPET > AUTOCLOSE);
