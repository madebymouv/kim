//! Bookmarks, the change history, hidden lines and the summary.

use super::*;
use crate::command::Command;

fn doc(s: &str) -> Document {
    Document::from_text(s)
}

fn at_line(d: &mut Document, l: usize) {
    let p = d.text().line_to_char(l);
    d.set_selection(&Selection::point(p));
}

fn line(d: &Document) -> usize {
    d.text().char_to_line(d.selection().primary().head)
}

#[test]
fn bookmarks_toggle_step_invert_and_take_their_lines() {
    let mut d = doc("a\nb\nc\nd\n");
    at_line(&mut d, 1);
    d.execute(Command::ToggleBookmark, 1);
    at_line(&mut d, 3);
    d.execute(Command::ToggleBookmark, 1);
    assert_eq!(d.bookmarked_lines(), vec![1, 3]);
    // F2 wraps; Shift+F2 back.
    d.execute(Command::NextBookmark, 1);
    assert_eq!(line(&d), 1, "after the last one: the first");
    d.execute(Command::PreviousBookmark, 1);
    assert_eq!(line(&d), 3);
    // Toggling again removes; several carets toggle each line.
    d.execute(Command::ToggleBookmark, 1);
    assert_eq!(d.bookmarked_lines(), vec![1]);
    d.set_selection(&Selection::new(vec![Range::point(0), Range::point(2)], 0));
    d.execute(Command::ToggleBookmark, 1);
    // Carets on lines 0 and 1: line 0 gets one, line 1 loses its own.
    assert_eq!(d.bookmarked_lines(), vec![0]);
    assert_eq!(d.bookmarked_text(), "a\n");
    // Inverse (the empty line after the last line ending counts as a line).
    d.execute(Command::InvertBookmarks, 1);
    assert_eq!(d.bookmarked_lines(), vec![1, 2, 3, 4]);
    // Bookmarks move with edits.
    at_line(&mut d, 0);
    d.type_text("new\n");
    assert_eq!(d.bookmarked_lines(), vec![2, 3, 4, 5]);
    // Removing the bookmarked lines, then all others; one undo step each.
    let mut d = doc("a\nb\nc\nd");
    d.bookmark_lines([1, 3]);
    d.execute(Command::RemoveBookmarkedLines, 1);
    assert_eq!(d.text().to_string(), "a\nc");
    assert!(
        d.bookmarked_lines().is_empty(),
        "their bookmarks went with them"
    );
    d.undo();
    assert_eq!(d.text().to_string(), "a\nb\nc\nd");
    // Undo brings the text back, not the bookmarks of the lines it removed (as in Notepad++).
    d.clear_bookmarks();
    d.bookmark_lines([1, 3]);
    d.execute(Command::RemoveUnmarkedLines, 1);
    assert_eq!(d.text().to_string(), "b\nd");
    // Paste into the bookmarked lines: line by line, the last one again when it runs out.
    let mut d = doc("a\nb\nc\nd\n");
    d.bookmark_lines([0, 2, 3]);
    assert_eq!(d.paste_into_bookmarked("X\nY"), 3);
    assert_eq!(d.text().to_string(), "X\nb\nY\nY\n");
    d.execute(Command::ClearBookmarks, 1);
    assert!(d.bookmarked_lines().is_empty());
    d.execute(Command::NextBookmark, 1);
    assert_eq!(d.take_notice().as_deref(), Some("No bookmark."));
}

#[test]
fn the_change_history_marks_edits_and_saving() {
    let mut d = doc("one\ntwo\nthree\nfour\n");
    assert!(d.changed_lines().is_empty(), "opening is not a change");
    at_line(&mut d, 1);
    d.type_text("2");
    d.type_text("2");
    assert_eq!(d.changed_lines(), vec![(1, Change::Modified)]);
    assert_eq!(
        d.markers().iter().filter(|m| m.kind == CHANGED).count(),
        1,
        "typing extends one marker"
    );
    d.mark_saved();
    assert_eq!(d.changed_lines(), vec![(1, Change::Saved)]);
    at_line(&mut d, 3);
    d.type_text("x\ny");
    assert_eq!(
        d.changed_lines(),
        vec![
            (1, Change::Saved),
            (3, Change::Modified),
            (4, Change::Modified)
        ]
    );
    assert_eq!(d.changed_lines_in(3, 3), vec![(3, Change::Modified)]);
    // A deletion marks its place; lines above move the marks.
    at_line(&mut d, 0);
    d.execute(Command::DeleteForward, 1);
    assert!(d.changed_lines().iter().any(|(l, _)| *l == 0));
    // Next / previous change: the first line of each run of changed lines (0–1, 3–4), wrapping.
    at_line(&mut d, 0);
    d.execute(Command::NextChange, 1);
    assert_eq!(line(&d), 3);
    d.execute(Command::NextChange, 1);
    assert_eq!(line(&d), 0, "wrapped");
    d.execute(Command::PreviousChange, 1);
    assert_eq!(line(&d), 3);
    d.execute(Command::ClearChanges, 1);
    assert!(d.changed_lines().is_empty());
    d.execute(Command::NextChange, 1);
    assert_eq!(d.take_notice().as_deref(), Some("No change."));
    // Not tracked when turned off.
    let mut d = doc("x\n");
    d.track_changes = false;
    d.type_text("y");
    assert!(d.changed_lines().is_empty());
}

#[test]
fn hidden_lines_hide_and_show_and_the_summary_counts() {
    let mut d = doc("zero\none\ntwo\nthree\n");
    d.set_selection(&Selection::single(Range::new(5, 10)));
    assert_eq!(d.hide_lines(), 2, "one and two");
    assert_eq!(d.hidden_blocks(), vec![(0, 2)]);
    assert_eq!(line(&d), 0, "the caret above them");
    // The first line cannot be hidden: from line 1.
    let mut e = doc("a\nb\n");
    e.set_selection(&Selection::point(0));
    assert_eq!(e.hide_lines(), 0);
    assert_eq!(e.take_notice(), None);
    e.execute(Command::HideLines, 1);
    assert_eq!(
        e.take_notice().as_deref(),
        Some("The first line cannot be hidden.")
    );
    // Edits above move them; showing.
    d.type_text("x\n");
    assert_eq!(d.hidden_blocks(), vec![(1, 2)]);
    d.show_lines(Some(1));
    assert!(d.hidden_blocks().is_empty());
    d.set_selection(&Selection::single(Range::new(6, 8)));
    d.hide_lines();
    d.execute(Command::ShowHiddenLines, 1);
    assert!(d.hidden_blocks().is_empty());
    // The summary: characters without line endings, words, lines, bytes, the selection.
    let mut d = doc("한글 word_1\r\nsecond line\r\n");
    d.set_selection(&Selection::new(vec![Range::new(0, 2), Range::point(5)], 0));
    let s = d.summary(str::len);
    assert_eq!(
        (
            s.chars,
            s.words,
            s.lines,
            s.bytes,
            s.selected_chars,
            s.selected_ranges
        ),
        (20, 4, 3, "한글 word_1\r\nsecond line\r\n".len(), 2, 1)
    );
}

#[test]
fn which_commands_edit() {
    assert!(!Command::ToggleBookmark.edits());
    assert!(!Command::NextChange.edits());
    assert!(!Command::HideLines.edits());
    assert!(!Command::MoveLeft.edits());
    assert!(Command::RemoveBookmarkedLines.edits());
    assert!(Command::DuplicateLine.edits());
}
