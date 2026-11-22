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

/// A range that moves with the edits (Step 94): a mark of Mark All, a styled token, a bookmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marker {
    pub start: usize,
    pub end: usize,
    /// 0 the Mark All mark, 1–5 the token styles, [`BOOKMARK`] a bookmarked line (empty range at
    /// the line's start).
    pub kind: u8,
}

/// The marker kind of a bookmarked line.
pub const BOOKMARK: u8 = 200;
/// Step 99: a folded fold, an empty range at the start of its first line (it moves with edits
/// like a bookmark).
pub const FOLDED: u8 = 201;

/// The text, the carets and the undo history of one open document.
#[derive(Debug, Clone)]
pub struct Document {
    text: Rope,
    sel: Selection,
    history: History,
    saved: u64,
    /// Step 94: marks, styled tokens and bookmarks, kept in place by every edit.
    markers: Vec<Marker>,
    /// Step 96: the column selection, while the selection is one.
    block: Option<multi::Block>,
    /// Step 96: typing into a column selection pads shorter lines with spaces (a setting).
    pub virtual_space: bool,
    /// Step 96: the occurrence commands started from the word at the caret (whole word, match
    /// case) and go on that way while they add ranges.
    occ_word: bool,
    /// Step 97: why the last text operation did not run (the window shows and clears it).
    pub(crate) notice: Option<String>,
    /// Step 97: the width (display columns) Split Lines splits at — the window sets its own.
    pub split_width: usize,
    /// Step 98: every edit's position with a generation number (the newest 1024), so a highlighter
    /// knows from where to lex again.
    edit_gen: u64,
    edit_log: std::collections::VecDeque<(u64, usize)>,
    /// Step 98: the language's comment tokens (the window sets them): a line comment, a block
    /// comment's open and close.
    pub comment_line: Option<String>,
    pub comment_block: Option<(String, String)>,
    /// Step 99: the lines folded away (first, last), sorted and apart — vertical moves skip them
    /// (the window sets them from its folds).
    pub hidden: Vec<(usize, usize)>,
    fold_gen: u64,
    /// Step 100: the typing aids in effect (the window sets them) and the snippet being filled
    /// (its stop numbers in order, the current one).
    pub aids: assist::Aids,
    /// Step 101: edits mark the change history (on unless a caller turns it off).
    pub track_changes: bool,
    snippet: Vec<u8>,
    snippet_at: usize,
    /// Columns per tab stop (display and vertical moves).
    pub tab_width: usize,
    /// Step 105: Tab and indentation insert spaces (up to the next tab stop) instead of a tab.
    pub use_spaces: bool,
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
            markers: Vec::new(),
            block: None,
            virtual_space: false,
            occ_word: false,
            notice: None,
            split_width: 80,
            edit_gen: 0,
            edit_log: std::collections::VecDeque::new(),
            comment_line: None,
            comment_block: None,
            hidden: Vec::new(),
            fold_gen: 0,
            aids: assist::Aids::default(),
            track_changes: true,
            snippet: Vec::new(),
            snippet_at: 0,
            tab_width: 4,
            use_spaces: false,
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
        self.block = None;
        self.occ_word = false;
        let len = self.len_chars();
        let s = self.text.slice(..);
        self.sel = sel.map(|r| Range {
            anchor: text::floor_grapheme(s, r.anchor.min(len)),
            head: text::floor_grapheme(s, r.head.min(len)),
            goal: r.goal,
        });
        self.history.seal();
    }

    /// Step 98: the generation of the newest edit (see [`Document::edits_since`]).
    pub fn edit_gen(&self) -> u64 {
        self.edit_gen
    }

    /// Step 98: the smallest position any edit after generation `since` touched (the text from
    /// there on may have changed); `None` without edits since. Older than the log: `Some(0)`.
    pub fn edits_since(&self, since: u64) -> Option<usize> {
        if since >= self.edit_gen {
            return None;
        }
        match self.edit_log.front() {
            Some((first, _)) if *first <= since + 1 => self
                .edit_log
                .iter()
                .filter(|(g, _)| *g > since)
                .map(|(_, at)| *at)
                .min(),
            _ => Some(0),
        }
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Unsaved changes since the last [`Document::mark_saved`] (or since opening).
    pub fn is_dirty(&self) -> bool {
        self.history.state() != self.saved
    }

    /// Step 103: there is something to save although the text did not change (a conversion to
    /// another encoding or line ending): the document is dirty until it is saved.
    pub fn mark_unsaved(&mut self) {
        self.saved = u64::MAX;
    }

    /// Step 103: every line ending becomes `eol` (one undo step; marks and bookmarks stay on their
    /// lines); `true` when something changed.
    pub fn convert_eol(&mut self, eol: Eol) -> bool {
        let target = eol.as_str();
        let mut parts = Vec::new();
        let mut i = 0usize;
        let mut chars = self.text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\r' if chars.peek() == Some(&'\n') => {
                    chars.next();
                    if target != "\r\n" {
                        parts.push((i, i + 2, target.to_string()));
                    }
                    i += 2;
                    continue;
                }
                '\r' if target != "\r" => parts.push((i, i + 1, target.to_string())),
                '\n' if target != "\n" => parts.push((i, i + 1, target.to_string())),
                _ => {}
            }
            i += 1;
        }
        self.eol = eol;
        if parts.is_empty() {
            return false;
        }
        let keep = self.sel.clone();
        self.change(parts, Kind::Other);
        // The carets stay on their lines (as close as the new endings allow).
        let n = self.len_chars();
        let ranges = keep
            .ranges()
            .iter()
            .map(|r| Range::point(r.head.min(n)))
            .collect();
        self.sel = Selection::new(ranges, keep.primary_index());
        true
    }

    /// Step 104: the text becomes `new` as one undo step that replaces only what differs — the
    /// common start and end stay, with their marks and bookmarks (a file that only grew gets an
    /// insertion at its end). The caret keeps its place before the change, moves with the text
    /// after it, and goes to the end of the new part from inside it. `false` when nothing differs.
    pub fn set_text_minimal(&mut self, new: &str) -> bool {
        let old_len = self.len_chars();
        let new_len = new.chars().count();
        let mut pre = 0usize;
        for (a, b) in self.text.chars().zip(new.chars()) {
            if a != b {
                break;
            }
            pre += 1;
        }
        if pre == old_len && pre == new_len {
            return false;
        }
        // The common end, not overlapping the common start.
        let most = (old_len - pre).min(new_len - pre);
        let mut suf = 0usize;
        let mut old_back = self.text.chars_at(old_len).reversed();
        let mut new_back = new.chars().rev();
        while suf < most {
            match (old_back.next(), new_back.next()) {
                (Some(a), Some(b)) if a == b => suf += 1,
                _ => break,
            }
        }
        let byte_at = |chars: usize| new.char_indices().nth(chars).map_or(new.len(), |(i, _)| i);
        let (from, to) = (pre, old_len - suf);
        let middle = &new[byte_at(pre)..byte_at(new_len - suf)];
        let inserted = new_len - suf - pre;
        let caret = self.sel.primary().head;
        let caret = if caret <= from {
            caret
        } else if caret >= to {
            caret - (to - from) + inserted
        } else {
            from + inserted
        };
        self.replace(vec![(from, to, middle.to_string())], Some(caret));
        true
    }

    /// Step 103: the characters at char positions `at` become `with` (one undo step).
    pub fn replace_chars(&mut self, at: &[usize], with: char) {
        if at.is_empty() {
            return;
        }
        let parts = at.iter().map(|p| (*p, *p + 1, with.to_string())).collect();
        self.change(parts, Kind::Other);
    }

    /// The current state is what is on disk now.
    pub fn mark_saved(&mut self) {
        self.saved = self.history.state();
        self.history.seal();
        // Step 101: the changes so far are saved ones.
        self.changes_saved();
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

    /// Applies one edit to the text and moves the markers with it.
    fn apply_edit(&mut self, e: &Edit) {
        self.edit_gen += 1;
        self.edit_log.push_back((self.edit_gen, e.at));
        if self.edit_log.len() > 1024 {
            self.edit_log.pop_front();
        }
        let n = e.removed.chars().count();
        debug_assert_eq!(self.text.slice(e.at..e.at + n).to_string(), e.removed);
        self.text.remove(e.at..e.at + n);
        self.text.insert(e.at, &e.inserted);
        if !self.markers.is_empty() {
            self.shift_markers(e.at, n, e.inserted.chars().count());
        }
        // Step 101: the change history.
        self.note_change(e.at, e.inserted.chars().count());
    }

    /// Markers after an edit at `at` that removed `removed` and inserted `inserted` characters:
    /// positions before it stay, positions after move by the difference, positions inside the
    /// removed text go to its start; a mark or style that became empty goes (bookmarks stay).
    fn shift_markers(&mut self, at: usize, removed: usize, inserted: usize) {
        let end = at + removed;
        // `start_side`: text inserted exactly at a marker's start pushes it on (the marker does
        // not grow over it); at its end it stays outside.
        let map = |p: usize, start_side: bool| {
            if p < at || (p == at && !(start_side && removed == 0)) {
                p
            } else if p >= end {
                p - removed + inserted
            } else {
                at
            }
        };
        for m in &mut self.markers {
            let s = map(m.start, true);
            let e = if m.end == m.start {
                s
            } else {
                map(m.end, false)
            };
            m.start = s;
            m.end = e.max(s);
        }
        self.markers
            .retain(|m| m.kind >= BOOKMARK || m.end > m.start);
    }

    /// The markers (marks, styled tokens, bookmarks), in the order they were added.
    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    /// Adds markers (positions clamped to the text).
    pub fn add_markers(&mut self, list: impl IntoIterator<Item = Marker>) {
        let len = self.len_chars();
        self.markers.extend(list.into_iter().map(|m| Marker {
            start: m.start.min(len),
            end: m.end.min(len).max(m.start.min(len)),
            kind: m.kind,
        }));
    }

    /// Removes the markers `keep` says no to.
    pub fn retain_markers(&mut self, keep: impl Fn(&Marker) -> bool) {
        self.markers.retain(|m| keep(m));
    }

    /// The lines with a bookmark (Step 94 sets them from Mark All; Step 101 adds the commands).
    pub fn bookmarked_lines(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .markers
            .iter()
            .filter(|m| m.kind == BOOKMARK)
            .map(|m| self.text.char_to_line(m.start.min(self.len_chars())))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Step 99: the first lines of the folded folds.
    pub fn folded_lines(&self) -> Vec<usize> {
        let n = self.len_chars();
        let mut v: Vec<usize> = self
            .markers
            .iter()
            .filter(|m| m.kind == FOLDED)
            .map(|m| self.text.char_to_line(m.start.min(n)))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Step 99: folds (`on`) or unfolds the fold starting on `line`.
    pub fn set_folded(&mut self, line: usize, on: bool) {
        self.set_folded_many(&[line], on);
    }

    /// Step 99: folds or unfolds the folds starting on `lines` (sorted), in one pass over the
    /// markers.
    pub fn set_folded_many(&mut self, lines: &[usize], on: bool) {
        let last = self.text.len_lines().saturating_sub(1);
        let text = &self.text;
        self.markers.retain(|m| {
            m.kind != FOLDED
                || lines
                    .binary_search(&text.char_to_line(m.start).min(last))
                    .is_err()
        });
        if on {
            for &l in lines {
                let p = self.text.line_to_char(l.min(last));
                self.markers.push(Marker {
                    start: p,
                    end: p,
                    kind: FOLDED,
                });
            }
        }
        self.fold_gen += 1;
    }

    /// Step 99: keeps the folded markers whose line `keep` accepts (folds that still exist).
    pub fn retain_folded(&mut self, keep: impl Fn(usize) -> bool) {
        let text = &self.text;
        let before = self.markers.len();
        self.markers
            .retain(|m| m.kind != FOLDED || keep(text.char_to_line(m.start)));
        if self.markers.len() != before {
            self.fold_gen += 1;
        }
    }

    /// Step 99: changes with every fold or unfold (the window lays out its folds again).
    pub fn fold_gen(&self) -> u64 {
        self.fold_gen
    }

    /// Step 99: the folded-away range holding `line`, if any.
    pub fn hidden_range(&self, line: usize) -> Option<(usize, usize)> {
        let k = self.hidden.partition_point(|(_, e)| *e < line);
        self.hidden.get(k).copied().filter(|(s, _)| *s <= line)
    }

    /// Bookmarks `lines` (once each).
    pub fn bookmark_lines(&mut self, lines: impl IntoIterator<Item = usize>) {
        let have = self.bookmarked_lines();
        let last = self.text.len_lines().saturating_sub(1);
        let mut lines: Vec<usize> = lines.into_iter().map(|l| l.min(last)).collect();
        lines.sort_unstable();
        lines.dedup();
        let add: Vec<Marker> = lines
            .into_iter()
            .filter(|l| have.binary_search(l).is_err())
            .map(|l| {
                let p = self.text.line_to_char(l);
                Marker {
                    start: p,
                    end: p,
                    kind: BOOKMARK,
                }
            })
            .collect();
        self.markers.extend(add);
        let mut seen = std::collections::HashSet::new();
        let text = &self.text;
        self.markers
            .retain(|m| m.kind != BOOKMARK || seen.insert(text.char_to_line(m.start)));
    }

    /// Replaces `parts` (ascending, one per range of the selection, possibly overlapping after a
    /// delete widened them) and leaves a caret after each insertion.
    fn change(&mut self, parts: Vec<(usize, usize, String)>, kind: Kind) {
        let parts = merge_parts(parts);
        if parts.iter().all(|(f, t, s)| f == t && s.is_empty()) {
            return;
        }
        // An edit ends a column selection (its carets stay).
        self.block = None;
        self.occ_word = false;
        let before = self.sel.clone();
        let merge = self.may_merge(&parts, kind);
        let mut edits = Vec::with_capacity(parts.len());
        for (from, to, ins) in parts.iter().rev() {
            let e = Edit {
                at: *from,
                removed: self.text.slice(*from..*to).to_string(),
                inserted: ins.clone(),
            };
            self.apply_edit(&e);
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
        // Step 96: a column selection with virtual space pads its shorter lines first.
        let parts = self.insert_parts(s);
        self.change(parts, kind);
    }

    /// Enter: the line ending plus the indentation of the line the caret is on (up to the
    /// caret), replacing selections.
    pub fn newline(&mut self) {
        // Step 100: the indentation of the aids (the line's by default).
        let (parts, back): (Vec<_>, Vec<usize>) = self
            .sel
            .ranges()
            .iter()
            .map(|r| {
                let (text, back) = self.newline_text(r.from(), r.to());
                ((r.from(), r.to(), text), back)
            })
            .unzip();
        self.change_back(parts, Kind::Other, &back);
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
        let tab = self.tab_width.max(1);
        if !multi {
            // Step 105: with spaces, as many as reach the next tab stop from the caret.
            let unit = if self.use_spaces {
                let col = if self.sel.len() == 1 {
                    self.line_col_display(self.sel.primary().from()).1
                } else {
                    0
                };
                " ".repeat(tab - col % tab)
            } else {
                "\t".to_string()
            };
            self.insert_kind(&unit, Kind::Other);
            return;
        }
        let unit = if self.use_spaces {
            " ".repeat(tab)
        } else {
            "\t".to_string()
        };
        let before = self.sel.clone();
        let lines = self.selected_lines();
        let parts: Vec<(usize, usize, String)> = lines
            .iter()
            .map(|l| {
                let at = self.text.line_to_char(*l);
                (at, at, unit.clone())
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
            self.apply_edit(&e);
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
            self.apply_edit(&e);
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
            self.apply_edit(&e.inverse());
        }
        self.sel = entry.before;
        self.block = None;
        self.occ_word = false;
        true
    }

    /// Redo the newest undone step; `false` when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.history.redo() else {
            return false;
        };
        let entry = entry.clone();
        for e in &entry.edits {
            self.apply_edit(e);
        }
        self.sel = entry.after;
        self.block = None;
        self.occ_word = false;
        true
    }

    // ---- caret movement --------------------------------------------------------------------

    fn move_each(&mut self, extend: bool, f: impl Fn(&Rope, Range) -> usize) {
        self.block = None;
        self.occ_word = false;
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
        self.block = None;
        self.occ_word = false;
        let tab = self.tab_width;
        let hidden = self.hidden.clone();
        let text = &self.text;
        let last = text.len_lines().saturating_sub(1);
        self.sel = self.sel.map(|r| {
            let line = text.char_to_line(r.head);
            let start = text.line_to_char(line);
            let goal = r
                .goal
                .unwrap_or_else(|| text::column_of(text.line(line), r.head - start, tab));
            let mut target = (line as isize + lines).clamp(0, last as isize) as usize;
            // Step 99: past folded lines in the direction of the move (a fold to the end of the
            // text stops the move on its first line).
            if let Some((s, e)) = hidden_range(&hidden, target) {
                target = if lines > 0 && e < last { e + 1 } else { s - 1 };
            }
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

    /// Step 93: puts `sel` in place of the selection and returns the old one, without sealing
    /// the undo step (a second view of the same document keeps carets of its own).
    pub fn swap_selection(&mut self, sel: &Selection) -> Selection {
        let len = self.len_chars();
        let s = self.text.slice(..);
        let new = sel.map(|r| Range {
            anchor: text::floor_grapheme(s, r.anchor.min(len)),
            head: text::floor_grapheme(s, r.head.min(len)),
            goal: r.goal,
        });
        std::mem::replace(&mut self.sel, new)
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

/// Step 96: multiple carets and column selections.
pub mod multi;

/// Step 100: typing aids and snippets.
pub mod assist;

/// Step 101: bookmarks, the change history, hidden lines and the summary.
pub mod navigate;

#[cfg(test)]
mod tests;

/// Step 99: the range of `hidden` (sorted, apart, never starting at line 0) holding `line`.
fn hidden_range(hidden: &[(usize, usize)], line: usize) -> Option<(usize, usize)> {
    let k = hidden.partition_point(|(_, e)| *e < line);
    hidden.get(k).copied().filter(|(s, _)| *s <= line && *s > 0)
}
