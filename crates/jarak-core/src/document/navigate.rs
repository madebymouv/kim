//! Step 101: bookmarks, the change history, hidden lines and the summary (`JARAK-FR-100`,
//! `JARAK-FR-101`, `JARAK-FR-104`).
//!
//! - Bookmarks are empty markers at a line's start ([`BOOKMARK`], since Step 94): toggled on the
//!   carets' lines, stepped through (wrapping), cleared, inverted; the bookmarked lines can be
//!   copied, cut, removed, replaced by pasted lines, or all other lines removed.
//! - The change history marks what each edit inserted ([`CHANGED`]; a deletion an empty marker
//!   where it happened); saving turns them into [`SAVED`]. Both move with later edits.
//! - Hidden lines ([`HIDDEN`], a marker over the hidden lines) are drawn and moved over like folded
//!   ones; the line above them shows where they are.

use super::{BOOKMARK, Document, Marker};
use crate::history::Kind;
use crate::selection::{Range, Selection};

/// What an edit inserted (or where it deleted) since the document was saved.
pub const CHANGED: u8 = 203;
/// A change that has been saved.
pub const SAVED: u8 = 204;
/// Lines hidden by the user (the marker spans them).
pub const HIDDEN: u8 = 205;

/// Counts for the Summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// Characters without the line endings.
    pub chars: usize,
    pub words: usize,
    pub lines: usize,
    /// In the document's encoding with its line endings (what saving writes, without a BOM).
    pub bytes: usize,
    pub selected_chars: usize,
    pub selected_ranges: usize,
}

/// How a line of the change history looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Modified,
    Saved,
}

impl Document {
    fn fold_gen_bump(&mut self) {
        self.fold_gen += 1;
    }

    /// The lines of the carets (each once, in order).
    fn caret_lines(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .sel
            .ranges()
            .iter()
            .map(|r| self.text.char_to_line(r.head.min(self.len_chars())))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Ctrl+F2: the caret's line (every caret's) gets a bookmark, or loses its bookmark.
    pub fn toggle_bookmarks(&mut self) {
        let marked = self.bookmarked_lines();
        let lines = self.caret_lines();
        let text = &self.text;
        let off: Vec<usize> = lines
            .iter()
            .copied()
            .filter(|l| marked.binary_search(l).is_ok())
            .collect();
        let on: Vec<usize> = lines
            .iter()
            .copied()
            .filter(|l| marked.binary_search(l).is_err())
            .collect();
        self.markers.retain(|m| {
            m.kind != BOOKMARK || off.binary_search(&text.char_to_line(m.start)).is_err()
        });
        self.bookmark_lines(on);
    }

    /// F2 / Shift+F2: the caret to the next (previous) bookmarked line, wrapping; whether there is
    /// one.
    pub fn step_bookmark(&mut self, back: bool) -> bool {
        let marked = self.bookmarked_lines();
        if marked.is_empty() {
            return false;
        }
        let line = self
            .text
            .char_to_line(self.sel.primary().head.min(self.len_chars()));
        let target = if back {
            marked.iter().rev().find(|l| **l < line).or(marked.last())
        } else {
            marked.iter().find(|l| **l > line).or(marked.first())
        };
        let p = self.text.line_to_char(*target.unwrap_or(&line));
        self.set_selection(&Selection::point(p));
        true
    }

    /// Removes every bookmark.
    pub fn clear_bookmarks(&mut self) {
        self.markers.retain(|m| m.kind != BOOKMARK);
    }

    /// Bookmarks the lines without one and removes the others.
    pub fn invert_bookmarks(&mut self) {
        let marked = self.bookmarked_lines();
        let all = 0..self.text.len_lines();
        self.clear_bookmarks();
        self.bookmark_lines(all.filter(|l| marked.binary_search(l).is_err()));
    }

    /// The text of the bookmarked lines (each with the document's line ending).
    pub fn bookmarked_text(&self) -> String {
        let eol = self.eol.as_str();
        self.bookmarked_lines()
            .iter()
            .map(|l| {
                let line = self.text.line(*l);
                format!("{}{eol}", line.slice(..crate::text::line_len(line)))
            })
            .collect()
    }

    /// Removes the bookmarked lines (`marked`) or all the others; one undo step. Returns how many
    /// lines went.
    pub fn remove_lines_by_bookmark(&mut self, marked: bool) -> usize {
        let set = self.bookmarked_lines();
        let n = self.text.len_lines();
        let lines: Vec<usize> = (0..n)
            .filter(|l| set.binary_search(l).is_ok() == marked)
            // The empty line after a final line ending is not a line to remove.
            .filter(|l| *l + 1 < n || self.text.line(*l).len_chars() > 0)
            .collect();
        if lines.is_empty() {
            return 0;
        }
        let parts = self.line_cuts(&lines);
        self.change(parts, Kind::Other);
        // The bookmarks of removed lines go with them (they would land on the next line).
        if marked {
            self.clear_bookmarks();
        }
        lines.len()
    }

    /// The edits removing whole `lines` (sorted): each with its line ending; the last line of the
    /// text takes the ending before it.
    fn line_cuts(&self, lines: &[usize]) -> Vec<(usize, usize, String)> {
        let n = self.text.len_lines();
        let mut parts = Vec::new();
        for &l in lines {
            let a = self.text.line_to_char(l);
            let b = if l + 1 < n {
                self.text.line_to_char(l + 1)
            } else {
                self.len_chars()
            };
            if l + 1 >= n && l > 0 && lines.binary_search(&(l - 1)).is_err() {
                // The last line: take the line ending before it instead.
                let prev = self.text.line(l - 1);
                let end = self.text.line_to_char(l - 1) + crate::text::line_len(prev);
                parts.push((end, b, String::new()));
            } else {
                parts.push((a, b, String::new()));
            }
        }
        parts
    }

    /// Paste to (replace) the bookmarked lines: each bookmarked line becomes the next line of
    /// `text` (the last one used again when it runs out); one undo step.
    pub fn paste_into_bookmarked(&mut self, text: &str) -> usize {
        let lines = self.bookmarked_lines();
        let src: Vec<&str> = text.lines().collect();
        if lines.is_empty() || src.is_empty() {
            return 0;
        }
        let parts: Vec<(usize, usize, String)> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let a = self.text.line_to_char(*l);
                let b = a + crate::text::line_len(self.text.line(*l));
                (a, b, src[i.min(src.len() - 1)].to_string())
            })
            .collect();
        self.change(parts, Kind::Other);
        lines.len()
    }

    // ---- the change history --------------------------------------------------------------------

    /// After an edit at `at` that inserted `inserted` characters: the change is marked (a run of
    /// typing extends one marker).
    pub(crate) fn note_change(&mut self, at: usize, inserted: usize) {
        if !self.track_changes {
            return;
        }
        let (a, b) = (at, at + inserted);
        if let Some(m) = self
            .markers
            .iter_mut()
            .rev()
            .take(8)
            .find(|m| m.kind == CHANGED && m.start <= b && a <= m.end)
        {
            m.start = m.start.min(a);
            m.end = m.end.max(b);
            return;
        }
        self.markers.push(Marker {
            start: a,
            end: b,
            kind: CHANGED,
        });
    }

    /// Saving: the changes become saved changes.
    pub(crate) fn changes_saved(&mut self) {
        for m in &mut self.markers {
            if m.kind == CHANGED {
                m.kind = SAVED;
            }
        }
    }

    /// The lines of the change history (sorted, each once; a modified line wins over a saved one).
    pub fn changed_lines(&self) -> Vec<(usize, Change)> {
        let n = self.len_chars();
        let mut v: Vec<(usize, Change)> = Vec::new();
        for m in self
            .markers
            .iter()
            .filter(|m| m.kind == CHANGED || m.kind == SAVED)
        {
            let c = if m.kind == CHANGED {
                Change::Modified
            } else {
                Change::Saved
            };
            let a = self.text.char_to_line(m.start.min(n));
            let b = self.text.char_to_line(m.end.min(n));
            for l in a..=b {
                v.push((l, c));
            }
        }
        v.sort_unstable_by_key(|(l, c)| (*l, *c == Change::Saved));
        v.dedup_by_key(|(l, _)| *l);
        v
    }

    /// The change history's lines between `from` and `to` (for the margin of the lines on screen).
    pub fn changed_lines_in(&self, from: usize, to: usize) -> Vec<(usize, Change)> {
        let n = self.len_chars();
        let (a0, b0) = (
            self.text
                .line_to_char(from.min(self.text.len_lines().saturating_sub(1))),
            self.text
                .line_to_char(to.min(self.text.len_lines().saturating_sub(1))),
        );
        let mut v: Vec<(usize, Change)> = Vec::new();
        for m in self
            .markers
            .iter()
            .filter(|m| (m.kind == CHANGED || m.kind == SAVED) && m.end >= a0 && m.start <= b0)
        {
            let c = if m.kind == CHANGED {
                Change::Modified
            } else {
                Change::Saved
            };
            let a = self.text.char_to_line(m.start.min(n)).max(from);
            let b = self.text.char_to_line(m.end.min(n)).min(to);
            for l in a..=b {
                v.push((l, c));
            }
        }
        v.sort_unstable_by_key(|(l, c)| (*l, *c == Change::Saved));
        v.dedup_by_key(|(l, _)| *l);
        v
    }

    /// The caret to the start of the next (previous) changed block, wrapping; whether there is one.
    pub fn step_change(&mut self, back: bool) -> bool {
        let lines: Vec<usize> = self.changed_lines().into_iter().map(|(l, _)| l).collect();
        // The first line of each run of changed lines.
        let starts: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(i, l)| *i == 0 || lines[i - 1] + 1 != **l)
            .map(|(_, l)| *l)
            .collect();
        if starts.is_empty() {
            return false;
        }
        let line = self
            .text
            .char_to_line(self.sel.primary().head.min(self.len_chars()));
        let t = if back {
            starts.iter().rev().find(|l| **l < line).or(starts.last())
        } else {
            starts.iter().find(|l| **l > line).or(starts.first())
        };
        let p = self.text.line_to_char(*t.unwrap_or(&line));
        self.set_selection(&Selection::point(p));
        true
    }

    /// Forgets the change history (the text stays).
    pub fn clear_changes(&mut self) {
        self.markers
            .retain(|m| m.kind != CHANGED && m.kind != SAVED);
    }

    // ---- hidden lines --------------------------------------------------------------------------

    /// Alt+H: hides the selected lines (every range's; at least the caret's line). The first line
    /// of the text stays (the line above hidden lines shows them). Returns how many were hidden.
    pub fn hide_lines(&mut self) -> usize {
        let mut count = 0;
        let ranges: Vec<Range> = self.sel.ranges().to_vec();
        for r in ranges {
            let a = self.text.char_to_line(r.from()).max(1);
            let mut b = self.text.char_to_line(r.to());
            // A selection ending at a line's start does not take that line.
            if r.to() > r.from() && self.text.line_to_char(b) == r.to() && b > a {
                b -= 1;
            }
            if b < a {
                continue;
            }
            let s = self.text.line_to_char(a);
            let e = self.text.line_to_char(b) + crate::text::line_len(self.text.line(b));
            self.markers.push(Marker {
                start: s,
                end: e.max(s + 1).min(self.len_chars()),
                kind: HIDDEN,
            });
            count += b - a + 1;
        }
        if count > 0 {
            // The caret goes to the line above the first hidden block.
            let first = self
                .sel
                .ranges()
                .iter()
                .map(|r| self.text.char_to_line(r.from()).max(1))
                .min()
                .unwrap_or(1);
            let p = self.text.line_to_char(first - 1);
            self.sel = Selection::point(p);
            self.fold_gen_bump();
        }
        count
    }

    /// The hidden blocks: (the line above, how many lines).
    pub fn hidden_blocks(&self) -> Vec<(usize, usize)> {
        let n = self.len_chars();
        let mut v: Vec<(usize, usize)> = self
            .markers
            .iter()
            .filter(|m| m.kind == HIDDEN && m.end > m.start)
            .map(|m| {
                let a = self.text.char_to_line(m.start.min(n));
                let b = self.text.char_to_line(m.end.saturating_sub(1).min(n));
                (a.saturating_sub(1), b + 1 - a)
            })
            .filter(|(_, k)| *k > 0)
            .collect();
        v.sort_unstable();
        v
    }

    /// Shows the hidden block below `line` again (all of them with `None`).
    pub fn show_lines(&mut self, line: Option<usize>) {
        let text = &self.text;
        let n = text.len_chars();
        self.markers.retain(|m| {
            m.kind != HIDDEN
                || line.is_some_and(|l| text.char_to_line(m.start.min(n)).saturating_sub(1) != l)
        });
        self.fold_gen_bump();
    }

    // ---- the summary ---------------------------------------------------------------------------

    /// The Summary: characters (without line endings), words (runs of letters, digits and `_`),
    /// lines, bytes as saving would write them (`encoded_len`: the length of a line ending's or a
    /// text's bytes in the document's encoding), and the selection.
    pub fn summary(&self, encoded_len: impl Fn(&str) -> usize) -> Summary {
        let mut s = Summary {
            lines: self.text.len_lines(),
            ..Summary::default()
        };
        let mut in_word = false;
        for c in self.text.chars() {
            if c != '\n' && c != '\r' {
                s.chars += 1;
            }
            let w = c.is_alphanumeric() || c == '_';
            if w && !in_word {
                s.words += 1;
            }
            in_word = w;
        }
        s.bytes = self.text.chunks().map(&encoded_len).sum();
        let ranges = self.sel.ranges();
        s.selected_ranges = ranges.iter().filter(|r| !r.is_empty()).count();
        s.selected_chars = ranges.iter().map(|r| r.to() - r.from()).sum();
        s
    }
}

#[cfg(test)]
mod tests;
