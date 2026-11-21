//! A document: text, selections and history, edited only through commands (`JARAK-FR-010…014`).
//!
//! The GUI turns keys and the mouse into calls here and draws the result; it never touches the
//! rope itself (ADR 0102 §2). Every edit is a transaction recorded for undo with the selections
//! before and after; caret moves seal the current typed run.

use crate::history::{Edit, History, Kind};
use crate::selection::{Range, Selection};
use crate::text;
use ropey::Rope;

/// A line ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    CrLf,
    Lf,
    Cr,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::CrLf => "\r\n",
            Eol::Lf => "\n",
            Eol::Cr => "\r",
        }
    }

    /// The first line ending in `text`, if any.
    pub fn detect(text: &Rope) -> Option<Eol> {
        let mut prev_cr = false;
        for c in text.chars() {
            match c {
                '\n' => return Some(if prev_cr { Eol::CrLf } else { Eol::Lf }),
                '\r' if prev_cr => return Some(Eol::Cr),
                '\r' => prev_cr = true,
                _ if prev_cr => return Some(Eol::Cr),
                _ => {}
            }
        }
        prev_cr.then_some(Eol::Cr)
    }
}

/// The text, the carets and the undo history of one open document.
#[derive(Debug, Clone)]
pub struct Document {
    text: Rope,
    sel: Selection,
    history: History,
    saved: u64,
    /// Columns per tab stop (display and vertical moves).
    pub tab_width: usize,
    /// What Enter inserts.
    pub eol: Eol,
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

impl Document {
    /// An empty document (Windows line endings until the file says otherwise).
    pub fn new() -> Document {
        Document::from_text("")
    }

    /// A document holding `s`, clean, with the caret at the start.
    pub fn from_text(s: &str) -> Document {
        let text = Rope::from_str(s);
        let eol = Eol::detect(&text).unwrap_or(Eol::CrLf);
        Document {
            text,
            sel: Selection::point(0),
            history: History::default(),
            saved: 0,
            tab_width: 4,
            eol,
        }
    }

    pub fn text(&self) -> &Rope {
        &self.text
    }

    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    pub fn selection(&self) -> &Selection {
        &self.sel
    }

    /// Replaces the selection (a click, a search hit); positions snap to grapheme starts.
    pub fn set_selection(&mut self, sel: &Selection) {
        let len = self.len_chars();
        let s = self.text.slice(..);
        self.sel = sel.map(|r| Range {
            anchor: text::floor_grapheme(s, r.anchor.min(len)),
            head: text::floor_grapheme(s, r.head.min(len)),
            goal: r.goal,
        });
        self.history.seal();
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Unsaved changes since the last [`Document::mark_saved`] (or since opening).
    pub fn is_dirty(&self) -> bool {
        self.history.state() != self.saved
    }

    /// The current state is what is on disk now.
    pub fn mark_saved(&mut self) {
        self.saved = self.history.state();
        self.history.seal();
    }

    /// 0-based line and display column of `pos`.
    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.len_chars());
        let line = self.text.char_to_line(pos);
        let start = self.text.line_to_char(line);
        (
            line,
            text::column_of(self.text.line(line), pos - start, self.tab_width),
        )
    }

    /// The selected text of every range, joined with the line ending (what Copy takes).
    pub fn selected_text(&self) -> String {
        let parts: Vec<String> = self
            .sel
            .ranges()
            .iter()
            .filter(|r| !r.is_empty())
            .map(|r| self.text.slice(r.from()..r.to()).to_string())
            .collect();
        parts.join(self.eol.as_str())
    }

    // ---- editing ---------------------------------------------------------------------------

    fn apply(text: &mut Rope, e: &Edit) {
        let n = e.removed.chars().count();
        debug_assert_eq!(text.slice(e.at..e.at + n).to_string(), e.removed);
        text.remove(e.at..e.at + n);
        text.insert(e.at, &e.inserted);
    }

    /// Replaces `parts` (ascending, one per range of the selection, possibly overlapping after a
    /// delete widened them) and leaves a caret after each insertion.
    fn change(&mut self, parts: Vec<(usize, usize, String)>, kind: Kind) {
        let parts = merge_parts(parts);
        if parts.iter().all(|(f, t, s)| f == t && s.is_empty()) {
            return;
        }
        let before = self.sel.clone();
        let merge = self.may_merge(&parts, kind);
        let mut edits = Vec::with_capacity(parts.len());
        for (from, to, ins) in parts.iter().rev() {
            let e = Edit {
                at: *from,
                removed: self.text.slice(*from..*to).to_string(),
                inserted: ins.clone(),
            };
            Self::apply(&mut self.text, &e);
            edits.push(e);
        }
        let mut delta: isize = 0;
        let mut ranges = Vec::with_capacity(parts.len());
        for (from, to, ins) in &parts {
            let n = ins.chars().count();
            let at = (*from as isize + delta) as usize + n;
            ranges.push(Range::point(at));
            delta += n as isize - (*to - *from) as isize;
        }
        let primary = before.primary_index().min(ranges.len() - 1);
        let after = Selection::new(ranges, primary);
        self.history
            .record(edits, before, after.clone(), kind, merge);
        self.sel = after;
    }

    /// Whether this change continues the newest undo entry: one caret, typing or deleting in
    /// place; a typed run breaks after whitespace (each word is its own undo step).
    fn may_merge(&self, parts: &[(usize, usize, String)], kind: Kind) -> bool {
        let Some(last) = self.history.last() else {
            return false;
        };
        if parts.len() != 1 || last.kind != kind || last.after.len() != 1 || self.sel.len() != 1 {
            return false;
        }
        let (from, to, ins) = &parts[0];
        let caret = last.after.primary().head;
        match kind {
            Kind::Typing => {
                let prev_space = last
                    .edits
                    .last()
                    .and_then(|e| e.inserted.chars().last())
                    .is_some_and(char::is_whitespace);
                let new_space = ins.chars().next().is_some_and(char::is_whitespace);
                from == to && *from == caret && !(prev_space && !new_space)
            }
            // Backspace ends where the last one left the caret; Delete starts there.
            Kind::Deleting => *to == caret || *from == caret,
            Kind::Other => false,
        }
    }

    /// Types `s` at every caret, replacing selections. One grapheme without a line break is
    /// typing (merged into a run); anything longer is one step of its own (a paste).
    pub fn type_text(&mut self, s: &str) {
        let kind = if s.contains(['\n', '\r']) || s.is_empty() || {
            let r = Rope::from_str(s);
            text::next_grapheme(r.slice(..), 0) != r.len_chars()
        } {
            Kind::Other
        } else {
            Kind::Typing
        };
        self.insert_kind(s, kind);
    }

    /// Inserts `s` at every caret as one undo step (paste).
    pub fn paste(&mut self, s: &str) {
        self.insert_kind(s, Kind::Other);
    }

    fn insert_kind(&mut self, s: &str, kind: Kind) {
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| (r.from(), r.to(), s.to_string()))
            .collect();
        self.change(parts, kind);
    }

    /// Enter: the line ending plus the indentation of the line the caret is on (up to the
    /// caret), replacing selections.
    pub fn newline(&mut self) {
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                let line = self.text.char_to_line(r.from());
                let start = self.text.line_to_char(line);
                let indent: String = self
                    .text
                    .slice(start..r.from())
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                (r.from(), r.to(), format!("{}{indent}", self.eol.as_str()))
            })
            .collect();
        self.change(parts, Kind::Other);
    }

    /// Backspace: the selection, or the grapheme before each caret.
    pub fn delete_backward(&mut self) {
        let s = self.text.slice(..);
        let all_carets = self.sel.ranges().iter().all(Range::is_empty);
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                if r.is_empty() {
                    (text::prev_grapheme(s, r.head), r.head, String::new())
                } else {
                    (r.from(), r.to(), String::new())
                }
            })
            .collect();
        self.change(
            parts,
            if all_carets {
                Kind::Deleting
            } else {
                Kind::Other
            },
        );
    }

    /// Delete: the selection, or the grapheme after each caret.
    pub fn delete_forward(&mut self) {
        let s = self.text.slice(..);
        let all_carets = self.sel.ranges().iter().all(Range::is_empty);
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                if r.is_empty() {
                    (r.head, text::next_grapheme(s, r.head), String::new())
                } else {
                    (r.from(), r.to(), String::new())
                }
            })
            .collect();
        self.change(
            parts,
            if all_carets {
                Kind::Deleting
            } else {
                Kind::Other
            },
        );
    }

    /// Ctrl+Backspace: to the start of the word (or the selection).
    pub fn delete_word_backward(&mut self) {
        let s = self.text.slice(..);
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                if r.is_empty() {
                    (text::prev_word_start(s, r.head), r.head, String::new())
                } else {
                    (r.from(), r.to(), String::new())
                }
            })
            .collect();
        self.change(parts, Kind::Other);
    }

    /// Ctrl+Delete: to the start of the next word (or the selection).
    pub fn delete_word_forward(&mut self) {
        let s = self.text.slice(..);
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                if r.is_empty() {
                    (r.head, text::next_word_start(s, r.head), String::new())
                } else {
                    (r.from(), r.to(), String::new())
                }
            })
            .collect();
        self.change(parts, Kind::Other);
    }

    /// Insert (overwrite) mode: typed text replaces the grapheme after each caret (never a line
    /// break); a selection is replaced as usual.
    pub fn overwrite_text(&mut self, s: &str) {
        let t = self.text.slice(..);
        let parts = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                if r.is_empty() {
                    let next = text::next_grapheme(t, r.head);
                    let line = self.text.char_to_line(r.head);
                    let end = self.text.line_to_char(line) + text::line_len(self.text.line(line));
                    (r.head, next.min(end).max(r.head), s.to_string())
                } else {
                    (r.from(), r.to(), s.to_string())
                }
            })
            .collect();
        self.change(parts, Kind::Other);
    }

    /// The lines the selection touches (a range ending at a line start leaves that line out).
    fn selected_lines(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for r in self.sel.ranges() {
            let first = self.text.char_to_line(r.from());
            let mut last = self.text.char_to_line(r.to());
            if last > first && self.text.line_to_char(last) == r.to() {
                last -= 1;
            }
            for l in first..=last {
                if out.last() != Some(&l) {
                    out.push(l);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Tab: with a selection over several lines, indent each of them by one tab; otherwise a
    /// tab character at each caret.
    pub fn indent(&mut self) {
        let multi = self.sel.ranges().iter().any(|r| {
            !r.is_empty() && self.text.char_to_line(r.from()) != self.text.char_to_line(r.to())
        });
        if !multi {
            self.insert_kind("\t", Kind::Other);
            return;
        }
        let before = self.sel.clone();
        let lines = self.selected_lines();
        let parts: Vec<(usize, usize, String)> = lines
            .iter()
            .map(|l| {
                let at = self.text.line_to_char(*l);
                (at, at, "\t".to_string())
            })
            .collect();
        self.change_keep_selection(&parts, before, &lines, 1);
    }

    /// Shift+Tab: removes one level of indentation (a tab, or up to `tab_width` spaces) from
    /// each line the selection touches (the caret's line without a selection).
    pub fn outdent(&mut self) {
        let before = self.sel.clone();
        let lines = self.selected_lines();
        let parts: Vec<(usize, usize, String)> = lines
            .iter()
            .filter_map(|l| {
                let at = self.text.line_to_char(*l);
                let line = self.text.line(*l);
                let n = match line.chars().next() {
                    Some('\t') => 1,
                    Some(' ') => line
                        .chars()
                        .take(self.tab_width.max(1))
                        .take_while(|c| *c == ' ')
                        .count(),
                    _ => 0,
                };
                (n > 0).then(|| (at, at + n, String::new()))
            })
            .collect();
        if parts.is_empty() {
            return;
        }
        let removed: Vec<usize> = parts.iter().map(|(a, b, _)| b - a).collect();
        self.change_outdent(&parts, before, &removed);
    }

    /// Applies line-start insertions and keeps the selection over the same text.
    fn change_keep_selection(
        &mut self,
        parts: &[(usize, usize, String)],
        before: Selection,
        lines: &[usize],
        width: usize,
    ) {
        let starts: Vec<usize> = lines.iter().map(|l| self.text.line_to_char(*l)).collect();
        let mut edits = Vec::new();
        for (from, to, ins) in parts.iter().rev() {
            let e = Edit {
                at: *from,
                removed: self.text.slice(*from..*to).to_string(),
                inserted: ins.clone(),
            };
            Self::apply(&mut self.text, &e);
            edits.push(e);
        }
        // A position moves by the insertions at line starts at or before it.
        let shift = |p: usize| p + starts.iter().filter(|s| **s <= p).count() * width;
        let after = before.map(|r| Range {
            anchor: shift(r.anchor),
            head: shift(r.head),
            goal: None,
        });
        self.history
            .record(edits, before, after.clone(), Kind::Other, false);
        self.sel = after;
    }

    fn change_outdent(
        &mut self,
        parts: &[(usize, usize, String)],
        before: Selection,
        removed: &[usize],
    ) {
        let cuts: Vec<(usize, usize)> = parts.iter().map(|(a, b, _)| (*a, *b)).collect();
        let mut edits = Vec::new();
        for (from, to, ins) in parts.iter().rev() {
            let e = Edit {
                at: *from,
                removed: self.text.slice(*from..*to).to_string(),
                inserted: ins.clone(),
            };
            Self::apply(&mut self.text, &e);
            edits.push(e);
        }
        // A position inside a removed run lands at its start; after it, it moves back by it.
        let shift = |p: usize| {
            let mut out = p;
            for ((a, b), n) in cuts.iter().zip(removed) {
                if p >= *b {
                    out -= n;
                } else if p > *a {
                    out -= p - a;
                }
            }
            out
        };
        let after = before.map(|r| Range {
            anchor: shift(r.anchor),
            head: shift(r.head),
            goal: None,
        });
        self.history
            .record(edits, before, after.clone(), Kind::Other, false);
        self.sel = after;
    }

    /// Undo the newest step; `false` when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(entry) = self.history.undo() else {
            return false;
        };
        let entry = entry.clone();
        for e in entry.edits.iter().rev() {
            Self::apply(&mut self.text, &e.inverse());
        }
        self.sel = entry.before;
        true
    }

    /// Redo the newest undone step; `false` when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.history.redo() else {
            return false;
        };
        let entry = entry.clone();
        for e in &entry.edits {
            Self::apply(&mut self.text, e);
        }
        self.sel = entry.after;
        true
    }

    // ---- caret movement --------------------------------------------------------------------

    fn move_each(&mut self, extend: bool, f: impl Fn(&Rope, Range) -> usize) {
        let text = &self.text;
        self.sel = self.sel.map(|r| r.moved(f(text, r), extend));
        self.history.seal();
    }

    /// Left: one grapheme back; without Shift a selection collapses to its start.
    pub fn move_left(&mut self, extend: bool) {
        self.move_each(extend, |t, r| {
            if !extend && !r.is_empty() {
                r.from()
            } else {
                text::prev_grapheme(t.slice(..), r.head)
            }
        });
    }

    /// Right: one grapheme on; without Shift a selection collapses to its end.
    pub fn move_right(&mut self, extend: bool) {
        self.move_each(extend, |t, r| {
            if !extend && !r.is_empty() {
                r.to()
            } else {
                text::next_grapheme(t.slice(..), r.head)
            }
        });
    }

    pub fn word_left(&mut self, extend: bool) {
        self.move_each(extend, |t, r| text::prev_word_start(t.slice(..), r.head));
    }

    pub fn word_right(&mut self, extend: bool) {
        self.move_each(extend, |t, r| text::next_word_start(t.slice(..), r.head));
    }

    /// Home: to the first non-blank character of the line, or to column 0 when already there.
    pub fn line_start(&mut self, extend: bool) {
        self.move_each(extend, |t, r| {
            let line = t.char_to_line(r.head);
            let start = t.line_to_char(line);
            let l = t.line(line);
            let len = text::line_len(l);
            let indent = l
                .chars()
                .take(len)
                .take_while(|c| *c == ' ' || *c == '\t')
                .count();
            if r.head == start + indent {
                start
            } else {
                start + indent
            }
        });
    }

    /// End: to the end of the line (before its line break).
    pub fn line_end(&mut self, extend: bool) {
        self.move_each(extend, |t, r| {
            let line = t.char_to_line(r.head);
            t.line_to_char(line) + text::line_len(t.line(line))
        });
    }

    pub fn doc_start(&mut self, extend: bool) {
        self.move_each(extend, |_, _| 0);
    }

    pub fn doc_end(&mut self, extend: bool) {
        self.move_each(extend, |t, _| t.len_chars());
    }

    /// Up (`lines` < 0) or down (`lines` > 0) keeping the goal column; on the first or last line
    /// a single-line move stays put, a page move goes to that line.
    pub fn move_lines(&mut self, lines: isize, extend: bool) {
        let tab = self.tab_width;
        let text = &self.text;
        let last = text.len_lines().saturating_sub(1);
        self.sel = self.sel.map(|r| {
            let line = text.char_to_line(r.head);
            let start = text.line_to_char(line);
            let goal = r
                .goal
                .unwrap_or_else(|| text::column_of(text.line(line), r.head - start, tab));
            let target = (line as isize + lines).clamp(0, last as isize) as usize;
            if target == line {
                return r;
            }
            let pos =
                text.line_to_char(target) + text::offset_at_column(text.line(target), goal, tab);
            Range {
                anchor: if extend { r.anchor } else { pos },
                head: pos,
                goal: Some(goal),
            }
        });
        self.history.seal();
    }

    /// Step 92: the edits until [`Document::end_group`] are one undo step (a Vim change).
    pub fn begin_group(&mut self) {
        self.history.begin_group();
    }

    pub fn end_group(&mut self) {
        self.history.end_group();
    }

    /// Replaces `parts` (`(from, to, text)` in char positions, any order, not overlapping) as
    /// one undo step and puts the caret at `caret` (positions after the edit), or after the last
    /// insertion without one.
    pub fn replace(&mut self, parts: Vec<(usize, usize, String)>, caret: Option<usize>) {
        let len = self.len_chars();
        let parts: Vec<(usize, usize, String)> = parts
            .into_iter()
            .map(|(f, t, s)| (f.min(len), t.min(len).max(f.min(len)), s))
            .collect();
        if parts.is_empty() {
            return;
        }
        // One range per part, so `change` applies each exactly once.
        self.sel = Selection::new(
            parts.iter().map(|(f, t, _)| Range::new(*f, *t)).collect(),
            0,
        );
        self.change(parts, Kind::Other);
        if let Some(c) = caret {
            let c = c.min(self.len_chars());
            self.sel = Selection::point(text::floor_grapheme(self.text.slice(..), c));
        } else {
            self.sel = self.sel.only_primary_last();
        }
    }

    /// Moves the caret (or the primary range) without sealing an open group's undo step.
    pub fn set_caret(&mut self, sel: &Selection) {
        let len = self.len_chars();
        let s = self.text.slice(..);
        self.sel = sel.map(|r| Range {
            anchor: text::floor_grapheme(s, r.anchor.min(len)),
            head: text::floor_grapheme(s, r.head.min(len)),
            goal: r.goal,
        });
        if !self.history.in_group() {
            self.history.seal();
        }
    }

    pub fn select_all(&mut self) {
        self.sel = Selection::single(Range::new(0, self.len_chars()));
        self.history.seal();
    }

    /// The word (or run) around `pos`, as a double-click selects it.
    pub fn select_word_at(&mut self, pos: usize) {
        let (a, b) = text::word_at(self.text.slice(..), pos);
        self.sel = Selection::single(Range::new(a, b));
        self.history.seal();
    }

    /// The whole line around `pos` including its line break, as a triple-click selects it.
    pub fn select_line_at(&mut self, pos: usize) {
        let line = self.text.char_to_line(pos.min(self.len_chars()));
        let a = self.text.line_to_char(line);
        let b = if line + 1 < self.text.len_lines() {
            self.text.line_to_char(line + 1)
        } else {
            self.len_chars()
        };
        self.sel = Selection::single(Range::new(a, b));
        self.history.seal();
    }
}

/// Parts sorted, with overlapping deletions joined (two carets deleting into each other).
fn merge_parts(mut parts: Vec<(usize, usize, String)>) -> Vec<(usize, usize, String)> {
    parts.sort_by_key(|(f, t, _)| (*f, *t));
    let mut out: Vec<(usize, usize, String)> = Vec::with_capacity(parts.len());
    for (f, t, s) in parts {
        if let Some(last) = out.last_mut()
            && f < last.1
        {
            last.1 = last.1.max(t);
            last.2.push_str(&s);
            continue;
        }
        out.push((f, t, s));
    }
    out
}

#[cfg(test)]
mod tests;
