//! Multiple carets, multi-selection and column (rectangular) selections (`JARAK-FR-080`,
//! `JARAK-FR-081`, Step 96).
//!
//! A column selection is a [`Block`]: from the anchor's (line, display column) to the head's. It
//! becomes one range per line, cut at the same *display* columns (a wide Hangul or CJK character
//! counts two, a tab runs to its stop), so the rectangle lines up on screen. Typing, deleting and
//! pasting then work on every range like any multi-selection; with virtual space on, lines
//! shorter than the block's left edge are padded with spaces first (in the same undo step). Every
//! edit records the selection before it, so undo brings all the carets back.

use super::{Document, Kind};
use crate::selection::{Range, Selection};
use crate::text;

/// A column selection: (line, display column) of the anchor and of the head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub anchor: (usize, usize),
    pub head: (usize, usize),
}

impl Block {
    /// The first and last line.
    pub fn lines(&self) -> (usize, usize) {
        (
            self.anchor.0.min(self.head.0),
            self.anchor.0.max(self.head.0),
        )
    }

    /// The left and right display column.
    pub fn cols(&self) -> (usize, usize) {
        (
            self.anchor.1.min(self.head.1),
            self.anchor.1.max(self.head.1),
        )
    }
}

/// How occurrences of the selected text are matched (Notepad++'s four multi-select variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Occurrences {
    pub match_case: bool,
    pub whole_word: bool,
}

/// The Column Editor's leading characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Leading {
    #[default]
    None,
    Zeros,
    Spaces,
}

/// The Column Editor's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Numbers {
    pub initial: i64,
    pub step: i64,
    /// Each number this many times (at least once).
    pub repeat: usize,
    pub leading: Leading,
    /// 2, 8, 10 or 16 (upper-case digits).
    pub base: u32,
}

impl Default for Numbers {
    fn default() -> Self {
        Numbers {
            initial: 1,
            step: 1,
            repeat: 1,
            leading: Leading::None,
            base: 10,
        }
    }
}

/// `count` numbers of the Column Editor, as text (all the same width with leading zeros or
/// spaces).
pub fn numbers(n: &Numbers, count: usize) -> Vec<String> {
    let rep = n.repeat.max(1);
    let digits = |v: i64| -> (bool, String) {
        let a = v.unsigned_abs();
        let s = match n.base {
            2 => format!("{a:b}"),
            8 => format!("{a:o}"),
            16 => format!("{a:X}"),
            _ => a.to_string(),
        };
        (v < 0, s)
    };
    let all: Vec<(bool, String)> = (0..count)
        .map(|i| {
            digits(
                n.initial
                    .saturating_add(n.step.saturating_mul((i / rep) as i64)),
            )
        })
        .collect();
    let width = all
        .iter()
        .map(|(neg, s)| s.len() + usize::from(*neg))
        .max()
        .unwrap_or(0);
    all.into_iter()
        .map(|(neg, s)| {
            let sign = if neg { "-" } else { "" };
            let len = s.len() + usize::from(neg);
            match n.leading {
                Leading::None => format!("{sign}{s}"),
                Leading::Zeros => format!("{sign}{}{s}", "0".repeat(width - len)),
                Leading::Spaces => format!("{}{sign}{s}", " ".repeat(width - len)),
            }
        })
        .collect()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Document {
    /// The column selection, while the selection is one (any other change ends it).
    pub fn block(&self) -> Option<Block> {
        self.block
    }

    /// The display width of a line (without its line break).
    pub fn line_width(&self, line: usize) -> usize {
        let l = self.text.line(line);
        text::column_of(l, text::line_len(l), self.tab_width)
    }

    /// (line, display column) of a char position.
    pub fn line_col_display(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.len_chars());
        let line = self.text.char_to_line(pos);
        let start = self.text.line_to_char(line);
        (
            line,
            text::column_of(self.text.line(line), pos - start, self.tab_width),
        )
    }

    /// The char position at (line, display column), clamped to the line.
    pub fn pos_at_display(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.text.len_lines().saturating_sub(1));
        self.text.line_to_char(line)
            + text::offset_at_column(self.text.line(line), col, self.tab_width)
    }

    /// Ctrl+click: a caret at `pos` is added (and made primary); clicking an existing caret of
    /// several removes it.
    pub fn add_caret(&mut self, pos: usize) {
        let pos = pos.min(self.len_chars());
        self.block = None;
        let ranges = self.sel.ranges();
        if ranges.len() > 1
            && let Some(k) = ranges.iter().position(|r| r.is_empty() && r.head == pos)
        {
            let mut v = ranges.to_vec();
            v.remove(k);
            let primary = if k == self.sel.primary_index() {
                v.len() - 1
            } else {
                self.sel.primary_index().min(v.len() - 1)
            };
            self.sel = Selection::new(v, primary);
        } else {
            self.sel = self.sel.with(Range::point(pos));
        }
        self.history.seal();
    }

    /// Ctrl+drag: the primary range becomes `r` (the others stay).
    pub fn set_primary_range(&mut self, r: Range) {
        self.block = None;
        let mut v = self.sel.ranges().to_vec();
        let k = self.sel.primary_index();
        v.remove(k);
        v.push(r);
        let n = v.len() - 1;
        self.sel = Selection::new(v, n);
    }

    /// Esc: back to one caret (the primary's head). Returns whether anything changed.
    pub fn single_caret(&mut self) -> bool {
        if self.sel.len() == 1 && self.block.is_none() {
            return false;
        }
        self.block = None;
        self.sel = Selection::point(self.sel.primary().head);
        self.history.seal();
        true
    }

    /// The text the occurrence commands look for: the primary selection, or (an empty one) the
    /// word at the caret, which is then selected. `None`: nothing to look for.
    fn occurrence_text(&mut self) -> Option<(String, bool)> {
        let r = self.sel.primary();
        if !r.is_empty() {
            return Some((self.text.slice(r.from()..r.to()).to_string(), false));
        }
        let (a, b) = text::word_at(self.text.slice(..), r.head);
        let w = self.text.slice(a..b).to_string();
        if b == a || !w.chars().all(is_word) {
            return None;
        }
        let mut v = self.sel.ranges().to_vec();
        let k = self.sel.primary_index();
        v[k] = Range::new(a, b);
        self.sel = Selection::new(v, k);
        Some((w, true))
    }

    /// Every occurrence of `needle` (chars), with the options.
    fn occurrences_of(&self, needle: &str, o: Occurrences) -> Vec<(usize, usize)> {
        let hay = self.text.to_string();
        let (h, n) = if o.match_case {
            (hay.clone(), needle.to_string())
        } else {
            (hay.to_lowercase(), needle.to_lowercase())
        };
        // Lower-casing may change byte lengths: fall back to the exact text when it does.
        let (h, n) = if h.len() == hay.len() && n.len() == needle.len() {
            (h, n)
        } else {
            (hay.clone(), needle.to_string())
        };
        if n.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(k) = h[at..].find(&n) {
            let a = at + k;
            let b = a + n.len();
            let ok = !o.whole_word
                || (!hay[..a].chars().next_back().is_some_and(is_word)
                    && !hay[b..].chars().next().is_some_and(is_word));
            if ok {
                out.push((self.text.byte_to_char(a), self.text.byte_to_char(b)));
            }
            at = a + n[..].chars().next().map_or(1, char::len_utf8);
        }
        out
    }

    /// Adds the next occurrence of the primary selection (or of the word at the caret) after the
    /// last range, wrapping (Ctrl+Shift+D). `skip`: the current primary is dropped (skip it).
    /// Returns whether a range was added.
    pub fn select_next_occurrence(&mut self, o: Option<Occurrences>, skip: bool) -> bool {
        let chained = self.occ_word;
        let Some((needle, word)) = self.occurrence_text() else {
            return false;
        };
        let from_word = word || chained;
        self.occ_word = from_word;
        // The first press on a caret only selects the word under it.
        if word && !skip {
            self.history.seal();
            return true;
        }
        // Automatic (None): a word matches as a whole word with its case, a selection as it is.
        let o = o.unwrap_or(Occurrences {
            whole_word: from_word,
            match_case: from_word,
        });
        let all = self.occurrences_of(&needle, o);
        let after = self.sel.primary().to();
        let taken = |a: usize, b: usize, s: &Selection| {
            s.ranges().iter().any(|r| r.from() == a && r.to() == b)
        };
        // The first one after the last range, else from the top (wrapping).
        let next = all
            .iter()
            .filter(|(a, _)| *a >= after)
            .chain(all.iter().filter(|(a, _)| *a < after))
            .find(|(a, b)| !taken(*a, *b, &self.sel))
            .copied();
        let Some((a, b)) = next else {
            return false;
        };
        self.block = None;
        let mut v = self.sel.ranges().to_vec();
        if skip && v.len() > 1 {
            v.remove(self.sel.primary_index());
        } else if skip {
            v.clear();
        }
        v.push(Range::new(a, b));
        let n = v.len() - 1;
        self.sel = Selection::new(v, n);
        self.history.seal();
        true
    }

    /// Selects every occurrence (Alt+F3). Returns how many there are.
    pub fn select_all_occurrences(&mut self, o: Option<Occurrences>) -> usize {
        let Some((needle, from_word)) = self.occurrence_text() else {
            return 0;
        };
        let o = o.unwrap_or(Occurrences {
            whole_word: from_word,
            match_case: from_word,
        });
        let all = self.occurrences_of(&needle, o);
        if all.is_empty() {
            return 0;
        }
        let here = self.sel.primary().from();
        let primary = all.iter().position(|(a, _)| *a >= here).unwrap_or(0);
        self.block = None;
        self.sel = Selection::new(
            all.iter().map(|(a, b)| Range::new(*a, *b)).collect(),
            primary,
        );
        self.history.seal();
        all.len()
    }

    /// Makes `b` the selection: one range per line between its display columns (the head line's
    /// range is primary; ranges point the way the block was drawn).
    pub fn select_block(&mut self, b: Block) {
        let last = self.text.len_lines().saturating_sub(1);
        let b = Block {
            anchor: (b.anchor.0.min(last), b.anchor.1),
            head: (b.head.0.min(last), b.head.1),
        };
        let (l0, l1) = b.lines();
        let (c0, c1) = b.cols();
        let rightwards = b.head.1 >= b.anchor.1;
        let mut v = Vec::with_capacity(l1 - l0 + 1);
        for line in l0..=l1 {
            let (a, h) = (self.pos_at_display(line, c0), self.pos_at_display(line, c1));
            v.push(if rightwards {
                Range::new(a, h)
            } else {
                Range::new(h, a)
            });
        }
        let primary = b.head.0 - l0;
        self.sel = Selection::new(v, primary);
        self.block = Some(b);
        self.history.seal();
    }

    /// Alt+Shift+arrows: the block's head moves by lines and display columns (a block starts at
    /// the primary caret). Without virtual space the head stops at its line's end.
    pub fn extend_block(&mut self, lines: isize, cols: isize) {
        let b = self.block.unwrap_or_else(|| {
            let p = self.line_col_display(self.sel.primary().head);
            Block { anchor: p, head: p }
        });
        let last = self.text.len_lines().saturating_sub(1) as isize;
        let line = (b.head.0 as isize + lines).clamp(0, last) as usize;
        let mut col = (b.head.1 as isize + cols).max(0) as usize;
        if !self.virtual_space {
            col = col.min(self.line_width(line).max(b.head.1.min(col)));
        }
        self.select_block(Block {
            anchor: b.anchor,
            head: (line, col),
        });
    }

    /// The pad that virtual space puts before an insertion at range `k` (spaces up to the block's
    /// left edge on a shorter line).
    fn virtual_pad(&self, r: &Range) -> String {
        let Some(b) = self.block.filter(|_| self.virtual_space) else {
            return String::new();
        };
        let line = self.text.char_to_line(r.from());
        let w = self.line_width(line);
        let (c0, _) = b.cols();
        if r.is_empty() && w < c0 {
            " ".repeat(c0 - w)
        } else {
            String::new()
        }
    }

    /// Inserts at every range (padding a column selection's short lines with virtual space).
    pub(super) fn insert_parts(&self, s: &str) -> Vec<(usize, usize, String)> {
        self.sel
            .ranges()
            .iter()
            .map(|r| (r.from(), r.to(), format!("{}{s}", self.virtual_pad(r))))
            .collect()
    }

    /// Paste of several lines into as many carets: each range gets its own line, as one undo
    /// step (`parts.len()` must equal the number of ranges, else every range gets them all).
    pub fn paste_each(&mut self, parts: &[String]) {
        if parts.len() != self.sel.len() {
            self.paste(&parts.join(self.eol.as_str()));
            return;
        }
        let v = self
            .sel
            .ranges()
            .iter()
            .zip(parts)
            .map(|(r, s)| (r.from(), r.to(), format!("{}{s}", self.virtual_pad(r))))
            .collect();
        self.change(v, Kind::Other);
    }

    /// How many rows the Column Editor fills (see [`Document::column_insert`]).
    pub fn column_rows(&self) -> usize {
        match self.block {
            Some(b) => {
                let (a, z) = b.lines();
                z - a + 1
            }
            None => {
                let (line, _) = self.line_col_display(self.sel.primary().head);
                self.text.len_lines() - line
            }
        }
    }

    /// The Column Editor: `texts` gives the text for each row (0, 1, …). The rows are the column
    /// selection's lines at its left edge, or else every line from the caret's down to the last
    /// at the caret's display column; shorter lines are padded with spaces. One undo step.
    pub fn column_insert(&mut self, texts: &mut dyn FnMut(usize) -> String) -> usize {
        let (l0, l1, col) = match self.block {
            Some(b) => {
                let (a, z) = b.lines();
                (a, z, b.cols().0)
            }
            None => {
                let (line, col) = self.line_col_display(self.sel.primary().head);
                (line, self.text.len_lines().saturating_sub(1), col)
            }
        };
        let mut parts = Vec::new();
        for (row, line) in (l0..=l1).enumerate() {
            let w = self.line_width(line);
            let pos = self.pos_at_display(line, col);
            let pad = if w < col {
                " ".repeat(col - w)
            } else {
                String::new()
            };
            parts.push((pos, pos, format!("{pad}{}", texts(row))));
        }
        let n = parts.len();
        self.change(parts, Kind::Other);
        n
    }
}

#[cfg(test)]
mod tests {
    //! Multiple carets and column selections with Hangul (two columns each) and emoji.
    use super::*;

    fn sel_texts(d: &Document) -> Vec<String> {
        d.selection()
            .ranges()
            .iter()
            .map(|r| d.text().slice(r.from()..r.to()).to_string())
            .collect()
    }

    #[test]
    fn carets_added_by_click_type_delete_paste_and_undo_together() {
        let mut d = Document::from_text("ab\ncd\nef\n");
        d.set_selection(&Selection::point(1));
        d.add_caret(4);
        d.add_caret(7);
        assert_eq!(d.selection().len(), 3);
        d.type_text("한");
        assert_eq!(d.text().to_string(), "a한b\nc한d\ne한f\n");
        d.delete_backward();
        d.delete_backward();
        assert_eq!(d.text().to_string(), "b\nd\nf\n");
        d.paste_each(&["1".into(), "2".into(), "3".into()]);
        assert_eq!(d.text().to_string(), "1b\n2d\n3f\n");
        // Paste, then each Backspace (several carets never merge into one step).
        assert!(d.undo());
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(d.text().to_string(), "a한b\nc한d\ne한f\n");
        assert_eq!(d.selection().len(), 3, "undo brings every caret back");
        // Clicking an existing caret of several removes it; Esc leaves the primary.
        let heads: Vec<usize> = d.selection().ranges().iter().map(|r| r.head).collect();
        d.add_caret(heads[1]);
        assert_eq!(d.selection().len(), 2);
        assert!(d.single_caret());
        assert_eq!(d.selection().len(), 1);
        assert!(!d.single_caret());
    }

    #[test]
    fn next_and_all_occurrences_with_their_options() {
        let mut d = Document::from_text("cat Cat cat concat cat");
        d.set_selection(&Selection::point(1));
        // From the word at the caret (the first press selects it): whole word, match case.
        assert!(d.select_next_occurrence(None, false));
        assert_eq!(sel_texts(&d), ["cat"]);
        assert!(d.select_next_occurrence(None, false));
        assert_eq!(sel_texts(&d), ["cat", "cat"]);
        assert_eq!(d.selection().primary().from(), 8);
        assert!(d.select_next_occurrence(None, false));
        assert_eq!(d.selection().primary().from(), 19, "not concat, not Cat");
        assert!(!d.select_next_occurrence(None, false), "all taken");
        // Skip: the primary is dropped and the next one added.
        d.set_selection(&Selection::single(Range::new(0, 3)));
        assert!(d.select_next_occurrence(None, true));
        assert_eq!(d.selection().len(), 1);
        assert_eq!(
            d.selection().primary().from(),
            4,
            "a selection: any case, inside words too"
        );
        // All: from a selection, ignoring case; whole word and match case options.
        d.set_selection(&Selection::single(Range::new(0, 3)));
        assert_eq!(d.select_all_occurrences(None), 5);
        d.set_selection(&Selection::single(Range::new(0, 3)));
        let o = Occurrences {
            match_case: true,
            whole_word: true,
        };
        assert_eq!(d.select_all_occurrences(Some(o)), 3);
        d.type_text("dog");
        assert_eq!(d.text().to_string(), "dog Cat dog concat dog");
        assert!(d.undo());
        assert_eq!(d.selection().len(), 3);
    }

    #[test]
    fn a_column_block_lines_up_by_display_column_with_hangul_and_emoji() {
        // 한글 is four columns; 😀 two.
        let mut d = Document::from_text("한글abc\nabcdef\n😀xyz\nab\n");
        d.select_block(Block {
            anchor: (0, 2),
            head: (2, 5),
        });
        assert_eq!(
            sel_texts(&d),
            ["글a", "cde", "xyz"],
            "columns 2..5 on every line"
        );
        assert_eq!(d.selection().primary_index(), 2, "the head's line");
        d.type_text("|");
        assert_eq!(d.text().to_string(), "한|bc\nab|f\n😀|\nab\n");
        assert!(d.block().is_none(), "the carets stay, the block ends");
        assert!(d.undo());
        assert_eq!(sel_texts(&d), ["글a", "cde", "xyz"]);
        // Alt+Shift+arrows from a caret.
        let mut d = Document::from_text("한글abc\nabcdef\nab\n");
        d.set_selection(&Selection::point(2));
        d.extend_block(1, 0);
        d.extend_block(0, 2);
        assert_eq!(
            d.block(),
            Some(Block {
                anchor: (0, 4),
                head: (1, 6)
            })
        );
        assert_eq!(sel_texts(&d), ["ab", "ef"]);
        d.extend_block(1, 0);
        assert_eq!(
            sel_texts(&d),
            ["ab", "ef", ""],
            "a short line: an empty range at its end"
        );
    }

    #[test]
    fn virtual_space_pads_short_lines_and_column_editor_inserts_numbers() {
        let mut d = Document::from_text("abcdef\nab\nabcd\n");
        d.virtual_space = true;
        d.select_block(Block {
            anchor: (0, 4),
            head: (2, 4),
        });
        d.type_text("X");
        assert_eq!(
            d.text().to_string(),
            "abcdXef\nab  X\nabcdX\n",
            "padded to column 4"
        );
        assert!(d.undo());
        assert_eq!(d.text().to_string(), "abcdef\nab\nabcd\n", "one undo step");
        // Column Editor from the caret's column down to the last line.
        let mut d = Document::from_text("a\nbb\nccc\n");
        d.set_selection(&Selection::point(1));
        let nums = numbers(
            &Numbers {
                initial: 8,
                step: 1,
                repeat: 1,
                leading: Leading::Zeros,
                base: 10,
            },
            4,
        );
        assert_eq!(nums, ["08", "09", "10", "11"]);
        let mut it = nums.into_iter();
        assert_eq!(d.column_insert(&mut |_| it.next().unwrap_or_default()), 4);
        assert_eq!(d.text().to_string(), "a08\nb09b\nc10cc\n 11");
        // A block: text into each row.
        let mut d = Document::from_text("한글\nab\n");
        d.select_block(Block {
            anchor: (0, 2),
            head: (1, 2),
        });
        d.column_insert(&mut |_| "|".into());
        assert_eq!(d.text().to_string(), "한|글\nab|\n");
    }

    #[test]
    fn column_editor_numbers_in_every_base_with_repeat_and_leading() {
        let n = |initial, step, repeat, leading, base| Numbers {
            initial,
            step,
            repeat,
            leading,
            base,
        };
        assert_eq!(
            numbers(&n(1, 2, 2, Leading::None, 10), 5),
            ["1", "1", "3", "3", "5"]
        );
        assert_eq!(
            numbers(&n(9, 1, 1, Leading::Spaces, 10), 3),
            [" 9", "10", "11"]
        );
        assert_eq!(
            numbers(&n(14, 1, 1, Leading::Zeros, 16), 3),
            ["0E", "0F", "10"]
        );
        assert_eq!(
            numbers(&n(1, 1, 1, Leading::Zeros, 2), 3),
            ["01", "10", "11"]
        );
        assert_eq!(numbers(&n(7, 1, 1, Leading::None, 8), 2), ["7", "10"]);
        assert_eq!(
            numbers(&n(1, -1, 1, Leading::Zeros, 10), 3),
            ["01", "00", "-1"]
        );
    }
}
