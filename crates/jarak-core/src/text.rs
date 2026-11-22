//! Text positions over a rope (`JARAK-FR-010`, `JARAK-FR-011`, `JARAK-NFR-008`).
//!
//! Positions are **char indices** into the rope (ropey's native unit). Everything that moves a
//! caret or deletes goes through grapheme boundaries (`unicode-segmentation`), so a Hangul
//! syllable written as jamo, a letter with combining marks, an emoji ZWJ sequence, a flag or a
//! CRLF line break is always one step. Display columns count East Asian wide characters as two
//! (`unicode-width`) and tabs to the next tab stop.
//!
//! Line breaks are LF, CR and CRLF only (ropey's `cr_lines`), like Notepad++: U+2028, U+0085 and
//! form feed are ordinary characters.

use ropey::RopeSlice;
use ropey::str_utils::byte_to_char_idx;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::UnicodeWidthStr;

/// Step 110 (SRCH-20): the position of the next character whose code point is in `lo..=hi`, from
/// `from` on and then from the start (wrapping); `None` when the text has none.
pub fn next_char_in_range(t: RopeSlice<'_>, from: usize, lo: u32, hi: u32) -> Option<usize> {
    let from = from.min(t.len_chars());
    let hit = |c: char| (lo..=hi).contains(&(c as u32));
    if let Some(i) = t.slice(from..).chars().position(hit) {
        return Some(from + i);
    }
    t.slice(..from).chars().position(hit)
}

/// The grapheme boundary before `char_idx` (0 at the start).
pub fn prev_grapheme(slice: RopeSlice, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    let byte_idx = slice.char_to_byte(char_idx);
    let (mut chunk, mut chunk_byte, mut chunk_char, _) = slice.chunk_at_byte(byte_idx);
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);
    loop {
        match gc.prev_boundary(chunk, chunk_byte) {
            Ok(None) => return 0,
            Ok(Some(n)) => return chunk_char + byte_to_char_idx(chunk, n - chunk_byte),
            Err(GraphemeIncomplete::PrevChunk) => {
                let (a, b, c, _) = slice.chunk_at_byte(chunk_byte - 1);
                chunk = a;
                chunk_byte = b;
                chunk_char = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx, n - ctx.len());
            }
            Err(_) => return char_idx - 1,
        }
    }
}

/// The grapheme boundary after `char_idx` (the length at the end).
pub fn next_grapheme(slice: RopeSlice, char_idx: usize) -> usize {
    let len = slice.len_chars();
    if char_idx >= len {
        return len;
    }
    let byte_idx = slice.char_to_byte(char_idx);
    let (mut chunk, mut chunk_byte, mut chunk_char, _) = slice.chunk_at_byte(byte_idx);
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);
    loop {
        match gc.next_boundary(chunk, chunk_byte) {
            Ok(None) => return len,
            Ok(Some(n)) => return chunk_char + byte_to_char_idx(chunk, n - chunk_byte),
            Err(GraphemeIncomplete::NextChunk) => {
                chunk_byte += chunk.len();
                let (a, _, c, _) = slice.chunk_at_byte(chunk_byte);
                chunk = a;
                chunk_char = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx, n - ctx.len());
            }
            Err(_) => return char_idx + 1,
        }
    }
}

/// Whether `char_idx` is on a grapheme boundary.
pub fn is_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> bool {
    if char_idx == 0 || char_idx >= slice.len_chars() {
        return true;
    }
    next_grapheme(slice, prev_grapheme(slice, char_idx)) == char_idx
}

/// The nearest grapheme boundary at or before `char_idx` (snaps a position into a cluster back
/// to its start).
pub fn floor_grapheme(slice: RopeSlice, char_idx: usize) -> usize {
    let char_idx = char_idx.min(slice.len_chars());
    if is_grapheme_boundary(slice, char_idx) {
        char_idx
    } else {
        prev_grapheme(slice, char_idx)
    }
}

/// What a word step treats as one kind of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Letters, digits (any script: Hangul, CJK, …) and `_`.
    Word,
    /// Spaces and tabs.
    Space,
    /// A line break.
    Eol,
    /// Everything else (punctuation, symbols, emoji).
    Punct,
}

thread_local! {
    /// Step 110 (SET-17): characters that count as word characters besides letters, digits and
    /// `_` (the settings' *word characters*), for the thread that edits (the window's).
    static WORD_CHARS: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Step 110 (SET-17): sets the further word characters (whitespace and line breaks never are).
pub fn set_word_chars(s: &str) {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    WORD_CHARS.with_borrow_mut(|w| {
        if *w != s {
            *w = s;
        }
    });
}

/// The further word characters in use.
pub fn word_chars() -> String {
    WORD_CHARS.with_borrow(Clone::clone)
}

impl CharClass {
    pub fn of(c: char) -> CharClass {
        if c == '\n' || c == '\r' {
            CharClass::Eol
        } else if c.is_whitespace() {
            CharClass::Space
        } else if c.is_alphanumeric()
            || c == '_'
            || WORD_CHARS.with_borrow(|w| !w.is_empty() && w.contains(c))
        {
            CharClass::Word
        } else {
            CharClass::Punct
        }
    }
}

fn class_at(slice: RopeSlice, char_idx: usize) -> CharClass {
    CharClass::of(slice.char(char_idx))
}

/// Ctrl+Right: to the start of the next word (past the rest of this run and the spaces after
/// it); a line break is a stop of its own.
pub fn next_word_start(slice: RopeSlice, char_idx: usize) -> usize {
    let len = slice.len_chars();
    if char_idx >= len {
        return len;
    }
    let first = class_at(slice, char_idx);
    let mut p = next_grapheme(slice, char_idx);
    if first == CharClass::Eol {
        return p;
    }
    while p < len && class_at(slice, p) == first && first != CharClass::Space {
        p = next_grapheme(slice, p);
    }
    while p < len && class_at(slice, p) == CharClass::Space {
        p = next_grapheme(slice, p);
    }
    p
}

/// Ctrl+Left: to the start of this word or the previous one (past spaces first).
pub fn prev_word_start(slice: RopeSlice, char_idx: usize) -> usize {
    let mut p = char_idx.min(slice.len_chars());
    while p > 0 {
        let q = prev_grapheme(slice, p);
        if class_at(slice, q) != CharClass::Space {
            break;
        }
        p = q;
    }
    if p == 0 {
        return 0;
    }
    let q = prev_grapheme(slice, p);
    let class = class_at(slice, q);
    if class == CharClass::Eol {
        return q;
    }
    p = q;
    while p > 0 {
        let q = prev_grapheme(slice, p);
        if class_at(slice, q) != class {
            break;
        }
        p = q;
    }
    p
}

/// The word (or run of the same class) around `char_idx`, as a double-click selects it.
pub fn word_at(slice: RopeSlice, char_idx: usize) -> (usize, usize) {
    let len = slice.len_chars();
    if len == 0 {
        return (0, 0);
    }
    let at = if char_idx >= len {
        prev_grapheme(slice, len)
    } else {
        floor_grapheme(slice, char_idx)
    };
    let class = class_at(slice, at);
    if class == CharClass::Eol {
        return (at, at);
    }
    let mut start = at;
    while start > 0 {
        let q = prev_grapheme(slice, start);
        if class_at(slice, q) != class {
            break;
        }
        start = q;
    }
    let mut end = at;
    while end < len && class_at(slice, end) == class {
        end = next_grapheme(slice, end);
    }
    (start, end)
}

/// The number of chars of `line` without its line break.
pub fn line_len(line: RopeSlice) -> usize {
    let n = line.len_chars();
    let mut cut = 0;
    if n > 0 && line.char(n - 1) == '\n' {
        cut = 1;
        if n > 1 && line.char(n - 2) == '\r' {
            cut = 2;
        }
    } else if n > 0 && line.char(n - 1) == '\r' {
        cut = 1;
    }
    n - cut
}

/// The display width of one grapheme starting at display column `col`.
pub fn grapheme_width(g: &str, col: usize, tab_width: usize) -> usize {
    if g == "\t" {
        let t = tab_width.max(1);
        t - col % t
    } else {
        g.width()
    }
}

/// The display column of char offset `offset` within `line` (0-based; wide characters count
/// two, a tab moves to the next multiple of `tab_width`).
pub fn column_of(line: RopeSlice, offset: usize, tab_width: usize) -> usize {
    let end = offset.min(line_len(line));
    let mut col = 0;
    let mut p = 0;
    while p < end {
        let q = next_grapheme(line, p).min(end);
        let g = line.slice(p..q).to_string();
        col += grapheme_width(&g, col, tab_width);
        p = q;
    }
    col
}

/// The char offset within `line` whose display column is the nearest at or before `col` (a
/// column inside a wide character or a tab lands on its start; past the end, the end).
pub fn offset_at_column(line: RopeSlice, col: usize, tab_width: usize) -> usize {
    let end = line_len(line);
    let mut c = 0;
    let mut p = 0;
    while p < end {
        let q = next_grapheme(line, p).min(end);
        let g = line.slice(p..q).to_string();
        let w = grapheme_width(&g, c, tab_width);
        if c + w > col {
            // Past the middle of a wide character or tab: the boundary after it is nearer.
            return if w > 1 && col >= c + w.div_ceil(2) && col > c {
                q
            } else {
                p
            };
        }
        c += w;
        p = q;
    }
    end
}

#[cfg(test)]
mod tests {
    /// Step 110 (SRCH-20): the next character in a code point range, wrapping.
    #[test]
    fn the_next_character_in_a_range_wraps() {
        let r = Rope::from_str("ab é c 中");
        let s = r.slice(..);
        assert_eq!(next_char_in_range(s, 0, 0x80, 0x10_FFFF), Some(3));
        assert_eq!(next_char_in_range(s, 4, 0x80, 0x10_FFFF), Some(7));
        assert_eq!(next_char_in_range(s, 8, 0x80, 0x10_FFFF), Some(3));
        assert_eq!(next_char_in_range(s, 0, 0x400, 0x4FF), None);
    }

    /// Step 110 (SET-17): further word characters join words, on this thread only.
    #[test]
    fn further_word_characters_join_words() {
        let r = Rope::from_str("foo-bar baz");
        let s = r.slice(..);
        assert_eq!(word_at(s, 5), (4, 7));
        set_word_chars("- ");
        assert_eq!(word_chars(), "-", "no whitespace");
        assert_eq!(word_at(s, 5), (0, 7));
        assert_eq!(
            std::thread::spawn(word_chars).join().unwrap(),
            "",
            "another thread keeps its own"
        );
        set_word_chars("");
        assert_eq!(word_at(s, 5), (4, 7));
    }

    use super::*;
    use ropey::Rope;

    fn steps(s: &str) -> Vec<usize> {
        let r = Rope::from_str(s);
        let mut out = vec![0];
        let mut p = 0;
        while p < r.len_chars() {
            p = next_grapheme(r.slice(..), p);
            out.push(p);
        }
        out
    }

    fn back_steps(s: &str) -> Vec<usize> {
        let r = Rope::from_str(s);
        let mut p = r.len_chars();
        let mut out = vec![p];
        while p > 0 {
            p = prev_grapheme(r.slice(..), p);
            out.push(p);
        }
        out.reverse();
        out
    }

    #[test]
    fn graphemes_are_never_split_in_either_direction() {
        let cases = [
            ("한글", vec![0, 1, 2]),
            // Hangul written as conjoining jamo: one syllable of three chars.
            ("\u{1112}\u{1161}\u{11AB}a", vec![0, 3, 4]),
            ("e\u{301}x", vec![0, 2, 3]),
            // A ZWJ family (five chars) and a flag (two regional indicators).
            ("👨\u{200D}👩\u{200D}👧!", vec![0, 5, 6]),
            ("🇰🇷🇯🇵", vec![0, 2, 4]),
            ("a\r\nb", vec![0, 1, 3, 4]),
            ("", vec![0]),
        ];
        for (s, want) in cases {
            assert_eq!(steps(s), want, "forward {s:?}");
            assert_eq!(back_steps(s), want, "backward {s:?}");
        }
    }

    #[test]
    fn boundaries_also_across_rope_chunks() {
        // Long enough for several ropey chunks, with clusters everywhere.
        let unit = "가e\u{301}👨\u{200D}👩\u{200D}👧🇰🇷\r\n";
        let s = unit.repeat(400);
        let r = Rope::from_str(&s);
        assert!(r.chunks().count() > 4);
        let mut p = 0;
        let mut n = 0;
        while p < r.len_chars() {
            let q = next_grapheme(r.slice(..), p);
            assert_eq!(prev_grapheme(r.slice(..), q), p);
            p = q;
            n += 1;
        }
        assert_eq!(n, 5 * 400);
    }

    #[test]
    fn floor_snaps_into_a_cluster_start() {
        let r = Rope::from_str("e\u{301}x");
        assert_eq!(floor_grapheme(r.slice(..), 1), 0);
        assert!(!is_grapheme_boundary(r.slice(..), 1));
        assert!(is_grapheme_boundary(r.slice(..), 2));
    }

    #[test]
    fn word_steps_and_double_click_words() {
        let r = Rope::from_str("let x_1 = 한글문장;  end\nnext");
        let s = r.slice(..);
        let mut p = 0;
        let mut stops = vec![];
        while p < r.len_chars() {
            p = next_word_start(s, p);
            stops.push(p);
        }
        // let | x_1 | = | 한글문장 | ; | end | \n | next
        assert_eq!(stops, vec![4, 8, 10, 14, 17, 20, 21, 25]);
        let mut back = vec![];
        let mut p = r.len_chars();
        while p > 0 {
            p = prev_word_start(s, p);
            back.push(p);
        }
        assert_eq!(back, vec![21, 20, 17, 14, 10, 8, 4, 0]);
        assert_eq!(word_at(s, 11), (10, 14), "Hangul is one word");
        assert_eq!(word_at(s, 5), (4, 7));
        assert_eq!(word_at(s, 14), (14, 15), "punctuation is its own run");
        assert_eq!(word_at(s, 15), (15, 17), "so are the spaces");
    }

    #[test]
    fn columns_count_wide_characters_and_tabs() {
        let r = Rope::from_str("한a\tb\n");
        let line = r.line(0);
        assert_eq!(line_len(line), 4);
        assert_eq!(column_of(line, 1, 4), 2, "a Hangul syllable is two columns");
        assert_eq!(column_of(line, 2, 4), 3);
        assert_eq!(column_of(line, 3, 4), 4, "the tab reaches the next stop");
        assert_eq!(column_of(line, 9, 4), 5, "past the end is the end");
        assert_eq!(offset_at_column(line, 0, 4), 0);
        assert_eq!(
            offset_at_column(line, 1, 4),
            1,
            "inside the wide char, nearer its end"
        );
        assert_eq!(offset_at_column(line, 2, 4), 1);
        assert_eq!(offset_at_column(line, 4, 4), 3);
        assert_eq!(offset_at_column(line, 99, 4), 4);
        let r = Rope::from_str("x\r\n");
        assert_eq!(line_len(r.line(0)), 1);
    }

    #[test]
    fn only_cr_lf_and_crlf_break_lines() {
        let r = Rope::from_str("a\u{2028}b\u{85}c\x0Cd\re\r\nf\ng");
        assert_eq!(r.len_lines(), 4);
    }
}
