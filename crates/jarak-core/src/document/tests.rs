//! Step 86 (#105): the document model. `JARAK-FR-010…014`, `JARAK-NFR-008`.

use super::*;
use std::collections::HashMap;

fn doc(s: &str) -> Document {
    Document::from_text(s)
}

fn at(d: &mut Document, pos: usize) {
    d.set_selection(&Selection::point(pos));
}

#[test]
fn typing_replaces_selections_and_undo_restores_text_and_carets() {
    let mut d = doc("hello world");
    d.set_selection(&Selection::single(Range::new(6, 11)));
    d.type_text("한");
    d.type_text("글");
    assert_eq!(d.text().to_string(), "hello 한글");
    assert_eq!(d.selection().primary(), Range::point(8));
    assert!(d.is_dirty());
    assert!(d.undo());
    assert_eq!(d.text().to_string(), "hello world");
    assert_eq!(
        d.selection().primary(),
        Range::new(6, 11),
        "the selection comes back"
    );
    assert!(!d.is_dirty(), "back at the opened state");
    assert!(d.redo());
    assert_eq!(d.text().to_string(), "hello 한글");
    assert_eq!(d.selection().primary(), Range::point(8));
    assert!(!d.redo());
}

#[test]
fn typed_runs_are_one_step_per_word_and_moves_seal_them() {
    let mut d = doc("");
    for c in "ab cd".chars() {
        d.type_text(&c.to_string());
    }
    assert_eq!(d.history().undo_len(), 2, "`ab ` and `cd`");
    d.undo();
    assert_eq!(d.text().to_string(), "ab ");
    d.undo();
    assert_eq!(d.text().to_string(), "");
    d.redo();
    d.redo();
    d.move_left(false);
    d.type_text("x");
    assert_eq!(d.text().to_string(), "ab cxd");
    assert_eq!(d.history().undo_len(), 3, "a caret move starts a new step");
    // A paste is always its own step.
    d.paste("yy");
    assert_eq!(d.history().undo_len(), 4);
}

#[test]
fn backspace_and_delete_never_split_a_grapheme() {
    for (s, len) in [
        ("e\u{301}", 0),
        ("\u{1112}\u{1161}\u{11AB}", 0),
        ("👨\u{200D}👩\u{200D}👧", 0),
        ("🇰🇷", 0),
        ("a\r\n", 1),
    ] {
        let mut d = doc(s);
        let end = d.len_chars();
        at(&mut d, end);
        d.delete_backward();
        assert_eq!(d.len_chars(), len, "backspace in {s:?}");
        let mut d = doc(s);
        at(&mut d, if s.starts_with('a') { 1 } else { 0 });
        d.delete_forward();
        assert_eq!(d.len_chars(), len, "delete in {s:?}");
    }
    // A caret inside a cluster (set from outside) snaps to its start.
    let mut d = doc("e\u{301}x");
    at(&mut d, 1);
    assert_eq!(d.selection().primary(), Range::point(0));
}

#[test]
fn backspace_runs_merge_and_word_deletes_do_not() {
    let mut d = doc("abc def");
    at(&mut d, 7);
    d.delete_backward();
    d.delete_backward();
    assert_eq!(d.history().undo_len(), 1);
    d.delete_word_backward();
    assert_eq!(d.text().to_string(), "abc ");
    d.delete_word_backward();
    assert_eq!(d.text().to_string(), "");
    assert_eq!(d.history().undo_len(), 3);
    d.undo();
    d.undo();
    d.undo();
    assert_eq!(d.text().to_string(), "abc def");
}

#[test]
fn enter_keeps_the_indentation_and_the_documents_line_ending() {
    let mut d = doc("    let a = 1;\nb");
    assert_eq!(d.eol, Eol::Lf);
    at(&mut d, 14);
    d.newline();
    assert_eq!(d.text().to_string(), "    let a = 1;\n    \nb");
    assert_eq!(d.selection().primary(), Range::point(19));
    let mut d = doc("x");
    assert_eq!(
        d.eol,
        Eol::CrLf,
        "Windows line endings for a file without one"
    );
    at(&mut d, 1);
    d.newline();
    assert_eq!(d.text().to_string(), "x\r\n");
    assert_eq!(Eol::detect(&Rope::from_str("a\rb")), Some(Eol::Cr));
    assert_eq!(Eol::detect(&Rope::from_str("a\r")), Some(Eol::Cr));
    assert_eq!(Eol::detect(&Rope::from_str("a\r\nb\n")), Some(Eol::CrLf));
}

#[test]
fn several_carets_type_delete_and_undo_together() {
    let mut d = doc("ab\ncd\nef");
    d.set_selection(&Selection::new(
        vec![Range::point(1), Range::point(4), Range::point(7)],
        2,
    ));
    d.type_text("X");
    assert_eq!(d.text().to_string(), "aXb\ncXd\neXf");
    let heads: Vec<usize> = d.selection().ranges().iter().map(|r| r.head).collect();
    assert_eq!(heads, [2, 6, 10]);
    assert_eq!(d.selection().primary_index(), 2);
    d.delete_backward();
    d.delete_backward();
    assert_eq!(
        d.text().to_string(),
        "b
d
f"
    );
    d.undo();
    assert_eq!(
        d.text().to_string(),
        "ab
cd
ef"
    );
    d.undo();
    assert_eq!(
        d.text().to_string(),
        "aXb
cXd
eXf"
    );
    d.undo();
    assert_eq!(
        d.text().to_string(),
        "ab
cd
ef"
    );
    let heads: Vec<usize> = d.selection().ranges().iter().map(|r| r.head).collect();
    assert_eq!(heads, [1, 4, 7]);
}

#[test]
fn moves_by_grapheme_word_line_and_document() {
    let mut d = doc("가나 다\n  ab\nlast");
    d.move_right(false);
    assert_eq!(d.selection().primary(), Range::point(1));
    d.word_right(true);
    assert_eq!(d.selection().primary(), Range::new(1, 3));
    d.move_left(false);
    assert_eq!(
        d.selection().primary(),
        Range::point(1),
        "a selection collapses to its start"
    );
    d.line_end(false);
    assert_eq!(d.selection().primary(), Range::point(4));
    d.move_lines(1, false);
    assert_eq!(
        d.selection().primary().head,
        9,
        "line 2 has fewer columns: its end"
    );
    d.line_start(false);
    assert_eq!(
        d.selection().primary(),
        Range::point(7),
        "smart home: the first non-blank"
    );
    d.line_start(false);
    assert_eq!(d.selection().primary(), Range::point(5), "then column 0");
    d.doc_end(true);
    assert_eq!(d.selection().primary(), Range::new(5, 14));
    d.doc_start(false);
    d.move_lines(-1, false);
    assert_eq!(
        d.selection().primary(),
        Range::point(0),
        "Up on the first line stays"
    );
}

#[test]
fn vertical_moves_keep_the_goal_column_across_wide_characters() {
    // Column 4 on line 0 is after "한글"; line 1 is short; line 2 continues at column 4.
    let mut d = doc("한글x\nab\nabcdef");
    at(&mut d, 2);
    d.move_lines(1, false);
    assert_eq!(d.selection().primary().head, 6, "end of the short line");
    d.move_lines(1, false);
    assert_eq!(d.selection().primary().head, 11, "back at column 4");
    d.move_lines(-2, false);
    assert_eq!(d.selection().primary().head, 2);
    assert_eq!(d.line_col(2), (0, 4));
}

#[test]
fn select_word_line_all_and_copy_text() {
    let mut d = doc("one two\r\nthree");
    d.select_word_at(5);
    assert_eq!(d.selected_text(), "two");
    d.select_line_at(2);
    assert_eq!(d.selected_text(), "one two\r\n");
    d.select_all();
    assert_eq!(d.selected_text(), "one two\r\nthree");
    d.set_selection(&Selection::new(vec![Range::new(0, 3), Range::new(4, 7)], 0));
    assert_eq!(d.selected_text(), "one\r\ntwo");
}

#[test]
fn the_save_point_follows_undo_and_redo() {
    let mut d = doc("a");
    at(&mut d, 1);
    d.type_text("b");
    d.mark_saved();
    assert!(!d.is_dirty());
    d.type_text("c");
    assert!(d.is_dirty());
    d.undo();
    assert!(!d.is_dirty(), "undo back to the saved text");
    d.undo();
    assert!(d.is_dirty(), "before the saved text is dirty too");
    d.redo();
    assert!(!d.is_dirty());
}

/// A small deterministic generator (no extra dependency).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn random_edits_undo_and_redo_stay_consistent() {
    const POOL: [&str; 9] = [
        "a",
        " ",
        "한",
        "e\u{301}",
        "👨\u{200D}👩\u{200D}👧",
        "🇰🇷",
        "\r\n",
        "xy",
        "\t",
    ];
    for seed in 1..=40u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let original = "start 한글\r\nline two\n";
        let mut d = doc(original);
        // Every state id the history reports must always mean the same text.
        let mut seen: HashMap<u64, String> = HashMap::new();
        seen.insert(0, original.to_string());
        for _ in 0..300 {
            let len = d.len_chars();
            match rng.below(10) {
                0 => {
                    let a = rng.below(len + 1);
                    let b = rng.below(len + 1);
                    d.set_selection(&Selection::single(Range::new(a, b)));
                }
                1..=3 => d.type_text(POOL[rng.below(POOL.len())]),
                4 => d.paste(&POOL[rng.below(POOL.len())].repeat(3)),
                5 => d.delete_backward(),
                6 => d.delete_forward(),
                7 => d.newline(),
                8 => {
                    d.undo();
                }
                _ => {
                    d.redo();
                }
            }
            let text = d.text().to_string();
            // Graphemes stay whole: every caret is on a boundary.
            for r in d.selection().ranges() {
                assert!(text::is_grapheme_boundary(d.text().slice(..), r.head));
                assert!(r.head <= d.len_chars());
            }
            let id = d.history().state();
            if let Some(prev) = seen.get(&id) {
                assert_eq!(prev, &text, "seed {seed}: state {id} changed its text");
            } else {
                seen.insert(id, text);
            }
        }
        while d.redo() {}
        let last = d.text().to_string();
        while d.undo() {}
        assert_eq!(d.text().to_string(), original, "seed {seed}: undo all");
        while d.redo() {}
        assert_eq!(d.text().to_string(), last, "seed {seed}: redo all");
    }
}
