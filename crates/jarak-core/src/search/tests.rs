//! Find and replace per mode and option (`JARAK-FR-070`, `JARAK-FR-071`), and markers moving
//! with the edits.

use super::*;
use crate::document::{BOOKMARK, Marker};
use crate::selection::Selection;

fn q(text: &str) -> Query {
    Query {
        text: text.into(),
        ..Query::default()
    }
}

fn all(doc: &Document, query: &Query) -> Vec<(usize, usize)> {
    let h = Haystack::of(doc);
    query.compile().unwrap().find_all(&h, None, &|| false).0
}

#[test]
fn normal_mode_finds_the_text_as_it_is_with_case_and_whole_word_options() {
    let d = Document::from_text("Cat cat concat (cat) CAT_x cat.");
    assert_eq!(all(&d, &q("cat")).len(), 6, "any case, inside words too");
    let case = Query {
        match_case: true,
        ..q("cat")
    };
    assert_eq!(all(&d, &case).len(), 4);
    let word = Query {
        whole_word: true,
        ..q("cat")
    };
    assert_eq!(
        all(&d, &word),
        vec![(0, 3), (4, 7), (16, 19), (27, 30)],
        "not in concat or CAT_x"
    );
    // Regex characters are literal.
    let d = Document::from_text("a.b axb (x)");
    assert_eq!(all(&d, &q("a.b")), vec![(0, 3)]);
    assert_eq!(all(&d, &q("(x)")), vec![(8, 11)]);
}

#[test]
fn extended_mode_understands_escapes() {
    assert_eq!(unescape(r"a\tb\n\x41é\\\0z").unwrap(), "a\tb\nAé\\\0z");
    assert!(unescape(r"\xZZ").is_err());
    let d = Document::from_text("one\ttwo\r\nthree\n");
    let ext = Query {
        mode: Mode::Extended,
        ..q(r"\t")
    };
    assert_eq!(all(&d, &ext), vec![(3, 4)]);
    let crlf = Query {
        mode: Mode::Extended,
        ..q(r"\r\n")
    };
    assert_eq!(all(&d, &crlf), vec![(7, 9)]);
    let normal = q(r"\t");
    assert!(
        all(&d, &normal).is_empty(),
        "Normal mode searches the backslash"
    );
}

#[test]
fn regex_mode_with_lines_dot_all_and_bad_patterns() {
    let d = Document::from_text("x1 y22\nz333\nend");
    let re = Query {
        mode: Mode::Regex,
        ..q(r"\d+")
    };
    assert_eq!(all(&d, &re), vec![(1, 2), (4, 6), (8, 11)]);
    let line_start = Query {
        mode: Mode::Regex,
        ..q("^z")
    };
    assert_eq!(all(&d, &line_start), vec![(7, 8)], "^ at each line");
    let across = Query {
        mode: Mode::Regex,
        ..q("2.z")
    };
    assert!(all(&d, &across).is_empty(), ". does not match a line break");
    let dot = Query {
        dot_all: true,
        ..across
    };
    assert_eq!(all(&d, &dot), vec![(5, 8)]);
    let bad = Query {
        mode: Mode::Regex,
        ..q("(")
    };
    assert!(bad.compile().is_err());
    assert!(q("").compile().is_err());
    // Korean text and positions in characters.
    let d = Document::from_text("한글 문장 한글");
    assert_eq!(all(&d, &q("한글")), vec![(0, 2), (6, 8)]);
}

#[test]
fn find_next_previous_wraps_and_stays_in_the_selection() {
    let d = Document::from_text("ab ab ab");
    let h = Haystack::of(&d);
    let m = q("ab").compile().unwrap();
    assert_eq!(m.find(&h, 0, false, true, None), Some(((0, 2), false)));
    assert_eq!(m.find(&h, 1, false, true, None), Some(((3, 5), false)));
    assert_eq!(
        m.find(&h, 7, false, true, None),
        Some(((0, 2), true)),
        "wrapped"
    );
    assert_eq!(m.find(&h, 7, false, false, None), None, "no wrap");
    assert_eq!(
        m.find(&h, 6, true, true, None),
        Some(((3, 5), false)),
        "backward"
    );
    assert_eq!(
        m.find(&h, 0, true, true, None),
        Some(((6, 8), true)),
        "backward wraps"
    );
    // In selection: only between 2 and 6.
    assert_eq!(
        m.find(&h, 0, false, true, Some((2, 6))),
        Some(((3, 5), false))
    );
    assert_eq!(
        m.find(&h, 5, false, true, Some((2, 6))),
        Some(((3, 5), true))
    );
    assert_eq!(m.find_all(&h, Some((2, 6)), &|| false).0, vec![(3, 5)]);
    // A wrapped search finds a match that starts before the caret and ends after it.
    let d = Document::from_text("xx cat. yy");
    let h = Haystack::of(&d);
    let m = Query {
        mode: Mode::Regex,
        ..q(r"cat\.")
    }
    .compile()
    .unwrap();
    assert_eq!(m.find(&h, 5, false, true, None), Some(((3, 7), true)));
}

#[test]
fn replace_all_is_one_undo_step_in_every_mode() {
    let mut d = Document::from_text("cat Cat cat");
    let m = q("cat").compile().unwrap();
    let h = Haystack::of(&d);
    assert_eq!(
        replace_all(&mut d, &m, &h, "dog", None, &|| false).unwrap(),
        3
    );
    assert_eq!(d.text().to_string(), "dog dog dog");
    assert!(d.undo());
    assert_eq!(d.text().to_string(), "cat Cat cat", "one step");
    // Regex groups, \1 and $1, \n in the replacement.
    let mut d = Document::from_text("a=1 b=22");
    let m = Query {
        mode: Mode::Regex,
        ..q(r"(\w)=(\d+)")
    }
    .compile()
    .unwrap();
    let h = Haystack::of(&d);
    replace_all(&mut d, &m, &h, r"\2:$1\n", None, &|| false).unwrap();
    assert_eq!(d.text().to_string(), "1:a\n 22:b\n");
    // Extended: escapes in the replacement.
    let mut d = Document::from_text("a,b");
    let m = Query {
        mode: Mode::Extended,
        ..q(",")
    }
    .compile()
    .unwrap();
    let h = Haystack::of(&d);
    replace_all(&mut d, &m, &h, r"\t", None, &|| false).unwrap();
    assert_eq!(d.text().to_string(), "a\tb");
    // In selection only.
    let mut d = Document::from_text("x x x x");
    let m = q("x").compile().unwrap();
    let h = Haystack::of(&d);
    assert_eq!(
        replace_all(&mut d, &m, &h, "y", Some((2, 5)), &|| false).unwrap(),
        2
    );
    assert_eq!(d.text().to_string(), "x y y x");
    // A cancelled scan replaces nothing.
    let text = "z ".repeat(10_000);
    let mut d = Document::from_text(&text);
    let m = q("z").compile().unwrap();
    let h = Haystack::of(&d);
    assert!(replace_all(&mut d, &m, &h, "w", None, &|| true).is_err());
    assert_eq!(d.text().to_string(), text);
}

#[test]
fn many_replacements_rewrite_the_text_in_one_edit() {
    let text = "ab".repeat(BULK + 10);
    let mut d = Document::from_text(&text);
    let m = q("b").compile().unwrap();
    let h = Haystack::of(&d);
    assert_eq!(
        replace_all(&mut d, &m, &h, "XY", None, &|| false).unwrap(),
        BULK + 10
    );
    assert_eq!(d.text().to_string(), "aXY".repeat(BULK + 10));
    assert_eq!(d.history().undo_len(), 1);
    d.undo();
    assert_eq!(d.text().to_string(), text);
}

#[test]
fn the_haystack_is_made_once_per_document_state() {
    let mut d = Document::from_text("abc");
    let h1 = Haystack::of(&d);
    let h2 = Haystack::reuse(Some(&h1), &d);
    assert!(Arc::ptr_eq(&h1.text, &h2.text), "the same text, not a copy");
    d.type_text("x");
    let h3 = Haystack::reuse(Some(&h1), &d);
    assert!(!Arc::ptr_eq(&h1.text, &h3.text));
    assert_eq!(h3.text.as_str(), "xabc");
}

#[test]
fn markers_move_with_edits_undo_and_redo_and_empty_ones_go() {
    let mut d = Document::from_text("one two three\nfour\n");
    d.add_markers([
        Marker {
            start: 4,
            end: 7,
            kind: 0,
        },
        Marker {
            start: 8,
            end: 13,
            kind: 2,
        },
    ]);
    d.bookmark_lines([1]);
    d.set_selection(&Selection::point(0));
    d.type_text("X");
    assert_eq!(
        (d.markers()[0].start, d.markers()[0].end),
        (5, 8),
        "moved by the insertion before"
    );
    assert_eq!(d.bookmarked_lines(), vec![1]);
    d.undo();
    assert_eq!(
        (d.markers()[0].start, d.markers()[0].end),
        (4, 7),
        "undo moves them back"
    );
    d.redo();
    assert_eq!(d.markers()[1].start, 9);
    // Deleting a marked word removes its mark; the bookmark stays with its line.
    d.set_selection(&Selection::single(crate::selection::Range::new(5, 8)));
    d.delete_backward();
    assert!(d.markers().iter().all(|m| m.kind != 0));
    d.set_selection(&Selection::point(0));
    d.type_text("new line\n");
    assert_eq!(
        d.bookmarked_lines(),
        vec![2],
        "the bookmarked line moved down"
    );
    d.retain_markers(|m| m.kind != BOOKMARK);
    assert!(d.bookmarked_lines().is_empty());
}

/// Step 94 (#113): Replace All on a 50 MB document (`JARAK-FR-070`), with its one-step undo, and
/// counting the matches. `cargo test --release -p jarak-core replace_all_on_50_mb -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn replace_all_on_50_mb() {
    use std::time::Instant;
    let line = "The quick brown fox jumps over the lazy dog 0123456789 한글\r\n";
    let text = line.repeat(50 * 1024 * 1024 / line.len() + 1);
    let mut d = Document::from_text(&text);
    let t = Instant::now();
    let h = Haystack::of(&d);
    let hay_ms = t.elapsed().as_secs_f64() * 1000.0;
    let m = q("fox").compile().unwrap();
    let t = Instant::now();
    let (found, _) = m.find_all(&h, None, &|| false);
    let count_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let n = replace_all(&mut d, &m, &h, "cat", None, &|| false).unwrap();
    let replace_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    assert!(d.undo());
    let undo_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(d.text().len_bytes(), text.len());
    let re = Query {
        mode: Mode::Regex,
        ..q(r"\d{10}")
    }
    .compile()
    .unwrap();
    let t = Instant::now();
    let h2 = Haystack::of(&d);
    let rn = replace_all(&mut d, &re, &h2, "N", None, &|| false).unwrap();
    let regex_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "50 MB ({} lines): haystack {hay_ms:.0} ms, count {} in {count_ms:.0} ms, Replace All {n} in {replace_ms:.0} ms, undo {undo_ms:.0} ms, regex Replace All {rn} in {regex_ms:.0} ms",
        d.text().len_lines(),
        found.len()
    );
    assert!(replace_ms < 3000.0 && undo_ms < 1000.0, "within budget");
}

#[test]
fn line_anchors_work_at_crlf_line_ends() {
    let d = Document::from_text("cat\r\nconcat\r\ndog\r\n");
    let h = Haystack::of(&d);
    let m = Query {
        mode: Mode::Regex,
        ..q("c[a-z]t$")
    }
    .compile()
    .unwrap();
    assert_eq!(m.find_all(&h, None, &|| false).0, vec![(0, 3), (8, 11)]);
    let m = Query {
        mode: Mode::Regex,
        ..q("^d.*$")
    }
    .compile()
    .unwrap();
    assert_eq!(
        m.find_all(&h, None, &|| false).0,
        vec![(13, 16)],
        "`.` stops before \r"
    );
}
