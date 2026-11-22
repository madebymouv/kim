//! Every Step 97 operation with Unicode text (Hangul, accents, emoji), one undo step each.

use super::*;

fn doc(s: &str) -> Document {
    Document::from_text(s)
}

fn sel(d: &mut Document, a: usize, b: usize) {
    d.set_selection(&Selection::single(Range::new(a, b)));
}

fn undo_restores(d: &mut Document, before: &str) {
    assert!(d.undo(), "one undo step");
    assert_eq!(d.text().to_string(), before);
}

#[test]
fn duplicate_a_line_or_a_selection() {
    let mut d = doc("한글 줄\nnext\n");
    d.duplicate();
    assert_eq!(d.text().to_string(), "한글 줄\n한글 줄\nnext\n");
    undo_restores(&mut d, "한글 줄\nnext\n");
    sel(&mut d, 0, 2);
    d.duplicate();
    assert_eq!(d.text().to_string(), "한글한글 줄\nnext\n");
}

#[test]
fn delete_lines_including_the_last() {
    let mut d = doc("가\n나\n다");
    d.set_selection(&Selection::point(2));
    d.delete_lines();
    assert_eq!(d.text().to_string(), "가\n다");
    d.set_selection(&Selection::point(3));
    d.delete_lines();
    assert_eq!(
        d.text().to_string(),
        "가",
        "the last line takes the break before it"
    );
    undo_restores(&mut d, "가\n다");
}

#[test]
fn move_lines_up_and_down_keeping_the_selection() {
    let mut d = doc("one\n둘\nthree\n");
    d.set_selection(&Selection::point(5));
    d.move_selected_lines(true);
    assert_eq!(d.text().to_string(), "one\nthree\n둘\n");
    assert_eq!(d.text().char_to_line(d.selection().primary().head), 2);
    d.move_selected_lines(false);
    d.move_selected_lines(false);
    assert_eq!(d.text().to_string(), "둘\none\nthree\n");
    d.move_selected_lines(false);
    assert_eq!(
        d.text().to_string(),
        "둘\none\nthree\n",
        "already at the top"
    );
}

#[test]
fn join_and_split_lines() {
    let mut d = doc("가나\n다라\n  indented\nend");
    sel(&mut d, 0, 9);
    d.join_lines();
    assert_eq!(
        d.text().to_string(),
        "가나 다라  indented\nend",
        "a space only where words touch"
    );
    undo_restores(&mut d, "가나\n다라\n  indented\nend");
    let mut d = doc("한글 두 글자 단어가 길게 이어지는 줄\n");
    d.set_selection(&Selection::point(0));
    d.split_lines(10);
    assert_eq!(
        d.text().to_string(),
        "한글 두\n글자\n단어가\n길게\n이어지는\n줄\n",
        "at most 10 columns, Hangul two each"
    );
}

#[test]
fn remove_duplicate_consecutive_and_empty_lines() {
    let mut d = doc("사과\nbanana\n사과\nbanana\nbanana\n\n   \ncafé\n");
    d.remove_duplicate_lines(true);
    assert_eq!(
        d.text().to_string(),
        "사과\nbanana\n사과\nbanana\n\n   \ncafé\n"
    );
    d.remove_duplicate_lines(false);
    assert_eq!(
        d.text().to_string(),
        "사과\nbanana\n\n   \ncafé\n",
        "the final line break stays"
    );
    d.remove_empty_lines(false);
    assert_eq!(d.text().to_string(), "사과\nbanana\n   \ncafé\n");
    d.remove_empty_lines(true);
    assert_eq!(d.text().to_string(), "사과\nbanana\ncafé\n");
}

#[test]
fn insert_blank_lines_above_and_below() {
    let mut d = doc("가\n나");
    d.set_selection(&Selection::point(3));
    d.insert_blank_line(true);
    assert_eq!(d.text().to_string(), "가\n\n나");
    assert_eq!(d.selection().primary().head, 2, "on the new line");
    d.insert_blank_line(false);
    assert_eq!(d.text().to_string(), "가\n\n\n나");
    undo_restores(&mut d, "가\n\n나");
}

#[test]
fn reverse_and_randomize_lines() {
    let mut d = doc("1 가\n2 나\n3 다\n");
    d.reverse_lines();
    assert_eq!(d.text().to_string(), "3 다\n2 나\n1 가\n");
    let mut d = doc("a\nb\nc\nd\ne\nf\n");
    d.randomize_lines(&mut Rng::new(7));
    let mut got: Vec<String> = d.text().to_string().lines().map(str::to_string).collect();
    assert_ne!(got, ["a", "b", "c", "d", "e", "f"], "shuffled");
    got.sort();
    assert_eq!(got, ["a", "b", "c", "d", "e", "f"], "the same lines");
}

#[test]
fn sort_lexicographic_case_locale_numbers_and_length() {
    let mut d = doc("banana\nÄpfel\napple\n가나\nApple\n");
    d.sort(Sort::Lex, false);
    assert_eq!(
        d.text().to_string(),
        "Apple\napple\nbanana\nÄpfel\n가나\n",
        "by code point"
    );
    d.sort(Sort::Lex, true);
    assert_eq!(d.text().to_string(), "가나\nÄpfel\nbanana\napple\nApple\n");
    d.sort(Sort::LexNoCase, false);
    assert_eq!(
        d.text().to_string(),
        "apple\nApple\nbanana\nÄpfel\n가나\n",
        "equal keys keep their order"
    );
    d.sort(Sort::Locale, false);
    let s = d.text().to_string();
    assert!(
        s.find("Äpfel").unwrap() < s.find("banana").unwrap(),
        "the locale puts Ä with A: {s}"
    );
    let mut d = doc("10\n-2\n9\nx\n3.5\n");
    d.sort(Sort::Integer, false);
    assert_eq!(
        d.text().to_string(),
        "x\n-2\n3.5\n9\n10\n",
        "3.5 counts as 3; no number first"
    );
    let mut d = doc("1,5\n1,25\n10\n");
    d.sort(Sort::DecimalComma, true);
    assert_eq!(d.text().to_string(), "10\n1,5\n1,25\n");
    let mut d = doc("2.5\n2.25\n");
    d.sort(Sort::DecimalDot, false);
    assert_eq!(d.text().to_string(), "2.25\n2.5\n");
    let mut d = doc("가나다\nab\n😀\n");
    d.sort(Sort::Length, false);
    assert_eq!(d.text().to_string(), "😀\nab\n가나다\n");
}

#[test]
fn sort_only_the_selected_lines_or_by_a_column() {
    let mut d = doc("z\nc 3\na 1\nb 2\ny\n");
    sel(&mut d, 2, 13);
    d.sort(Sort::Lex, false);
    assert_eq!(
        d.text().to_string(),
        "z\na 1\nb 2\nc 3\ny\n",
        "only lines 2-4"
    );
    let mut d = doc("가 3\n나 1\n다 2\n");
    d.select_block(crate::document::multi::Block {
        anchor: (0, 3),
        head: (2, 4),
    });
    d.sort(Sort::Integer, false);
    assert_eq!(
        d.text().to_string(),
        "나 1\n다 2\n가 3\n",
        "by the number in column 3"
    );
}

#[test]
fn every_case_conversion() {
    let mut r = Rng::new(3);
    let c = |s: &str, k: Case, r: &mut Rng| convert_case(s, k, r);
    assert_eq!(
        c("héllo wörld 한글", Case::Upper, &mut r),
        "HÉLLO WÖRLD 한글"
    );
    assert_eq!(c("HÉLLO", Case::Lower, &mut r), "héllo");
    assert_eq!(
        c("hELLO wORLD it's", Case::Proper, &mut r),
        "Hello World It's"
    );
    assert_eq!(c("hELLO wORLD", Case::ProperBlend, &mut r), "HELLO WORLD");
    assert_eq!(
        c("hello. WORLD! ünd", Case::Sentence, &mut r),
        "Hello. World! Ünd"
    );
    assert_eq!(
        c("hello. wORLD", Case::SentenceBlend, &mut r),
        "Hello. WORLD"
    );
    assert_eq!(c("Hello Ünd", Case::Invert, &mut r), "hELLO üND");
    let rnd = c("abcdefghij", Case::Random, &mut r);
    assert_eq!(rnd.to_lowercase(), "abcdefghij");
    assert!(
        rnd.chars().any(char::is_uppercase) && rnd.chars().any(char::is_lowercase),
        "{rnd}"
    );
    let mut d = doc("abc 가나 def");
    d.set_selection(&Selection::new(
        vec![Range::new(0, 3), Range::new(7, 10)],
        0,
    ));
    assert!(d.change_case(Case::Upper, &mut r));
    assert_eq!(d.text().to_string(), "ABC 가나 DEF");
    undo_restores(&mut d, "abc 가나 def");
    d.set_selection(&Selection::point(0));
    assert!(!d.change_case(Case::Upper, &mut r));
    assert_eq!(d.take_notice().as_deref(), Some("Select the text first."));
}

#[test]
fn every_blank_operation() {
    let blank = |s: &str, op: Blank| {
        let mut d = doc(s);
        d.blank(op);
        d.text().to_string()
    };
    assert_eq!(blank("  가 \t\n b  \n", Blank::TrimTrailing), "  가\n b\n");
    assert_eq!(blank("  가 \n\tb\n", Blank::TrimLeading), "가 \nb\n");
    assert_eq!(blank("  가 \n b \n", Blank::TrimBoth), "가\nb\n");
    assert_eq!(blank("한\n글\n", Blank::EolToSpace), "한 글\n");
    assert_eq!(blank(" 한 \n 글 \n", Blank::TrimBothEolToSpace), "한 글\n");
    assert_eq!(
        blank("a\tb\n가\tc\n", Blank::TabToSpace),
        "a   b\n가  c\n",
        "tab stops at 4 with Hangul two wide"
    );
    assert_eq!(
        blank("        x  y\n", Blank::SpaceToTabLeading),
        "\t\tx  y\n"
    );
    assert_eq!(
        blank("ab  cd      e\n", Blank::SpaceToTabAll),
        "ab\tcd\t\te\n"
    );
}

#[test]
fn every_codec_round_trips_unicode() {
    let s = "한글 é 😀 a+b/c?=x";
    for c in [
        Codec::Base64,
        Codec::Base64NoPad,
        Codec::Base64Url,
        Codec::Url,
        Codec::QuotedPrintable,
        Codec::Hex,
    ] {
        let e = c.encode(s);
        assert_eq!(c.decode(&e).unwrap(), s, "{c:?}: {e}");
    }
    assert_eq!(Codec::Base64.encode("Man"), "TWFu");
    assert_eq!(Codec::Base64.encode("Ma"), "TWE=");
    assert_eq!(Codec::Base64NoPad.encode("Ma"), "TWE");
    assert_eq!(Codec::Base64Url.encode("\u{fb}\u{ff}"), "w7vDvw");
    assert_eq!(Codec::Url.encode("a b/한"), "a%20b%2F%ED%95%9C");
    assert_eq!(Codec::Hex.encode("Hi"), "4869");
    assert_eq!(
        Codec::QuotedPrintable.encode("caf\u{e9} =\r\n"),
        "caf=C3=A9 =3D\r\n"
    );
    assert!(Codec::Hex.decode("FF").is_err(), "not UTF-8");
    assert!(Codec::Base64.decode("@@").is_err());
    let long = "x".repeat(100);
    assert!(
        Codec::QuotedPrintable
            .encode(&long)
            .lines()
            .all(|l| l.len() <= 76),
        "soft breaks"
    );
    let mut d = doc("[한글]");
    sel(&mut d, 1, 3);
    assert!(d.encode(Codec::Base64));
    assert_eq!(d.text().to_string(), "[7ZWc6riA]");
    assert!(d.decode(Codec::Base64));
    assert_eq!(d.text().to_string(), "[한글]");
    sel(&mut d, 1, 3);
    assert!(!d.decode(Codec::Hex), "한글 is not hex");
    assert_eq!(d.text().to_string(), "[한글]", "nothing changed");
    assert!(d.take_notice().unwrap().contains("hex"));
}

#[test]
fn digests_match_the_standard_test_vectors() {
    assert_eq!(Hash::Md5.hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(Hash::Md5.hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        Hash::Md5.hex(
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        ),
        "57edf4a22be3c955ac49da2e2107b67a"
    );
    assert_eq!(
        Hash::Sha1.hex(b"abc"),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        Hash::Sha1.hex(b""),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    assert_eq!(
        Hash::Sha1.hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
    );
    assert_eq!(
        Hash::Sha256.hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(Hash::Sha512.hex(b"abc").starts_with("ddaf35a193617aba"));
    assert_eq!(Hash::Sha256.hex("한글".as_bytes()).len(), 64);
}

#[test]
fn the_custom_date_format() {
    let t = DateTime {
        year: 2026,
        month: 11,
        day: 2,
        hour: 15,
        minute: 4,
        second: 9,
    };
    assert_eq!(
        format_datetime("yyyy-MM-dd HH:mm:ss", t),
        "2026-11-02 15:04:09"
    );
    assert_eq!(format_datetime("yy/M/d h:m:s tt", t), "26/11/2 3:4:9 PM");
    assert_eq!(
        format_datetime("yyyy년 M월 d일 'at' HH시", t),
        "2026년 11월 2일 at 15시"
    );
}

#[test]
fn comments_toggle_add_remove_block_and_markup() {
    let src = "fn a() {\n    한글();\n\n}\n";
    let mut d = doc(src);
    d.comment_line = Some("//".into());
    d.comment_block = Some(("/*".into(), "*/".into()));
    sel(&mut d, 0, 20);
    assert!(d.comment_lines(CommentMode::Toggle));
    assert_eq!(
        d.text().to_string(),
        "// fn a() {\n//     한글();\n\n}\n",
        "at the common indentation; the empty line stays"
    );
    sel(&mut d, 0, 25);
    assert!(d.comment_lines(CommentMode::Toggle));
    assert_eq!(
        d.text().to_string(),
        src,
        "every line commented: toggle uncomments"
    );
    sel(&mut d, 0, 20);
    d.comment_lines(CommentMode::Remove);
    assert_eq!(d.text().to_string(), src, "nothing to remove");
    sel(&mut d, 0, 8);
    assert!(d.block_comment(false));
    assert_eq!(d.text().to_string(), "/* fn a() { */\n    한글();\n\n}\n");
    assert!(d.block_comment(true), "the wrapped selection is unwrapped");
    assert_eq!(d.text().to_string(), src);
    undo_restores(&mut d, "/* fn a() { */\n    한글();\n\n}\n");
    // A language with only block comments (HTML) wraps each line.
    let mut h = doc("<p>한글</p>\n<b>x</b>\n");
    h.comment_block = Some(("<!--".into(), "-->".into()));
    sel(&mut h, 0, 18);
    assert!(h.comment_lines(CommentMode::Toggle));
    assert_eq!(
        h.text().to_string(),
        "<!-- <p>한글</p> -->\n<!-- <b>x</b> -->\n"
    );
    sel(&mut h, 0, 36);
    h.comment_lines(CommentMode::Toggle);
    assert_eq!(h.text().to_string(), "<p>한글</p>\n<b>x</b>\n");
    let mut n = doc("x");
    assert!(!n.comment_lines(CommentMode::Toggle));
    assert_eq!(
        n.take_notice().as_deref(),
        Some("This language has no comments.")
    );
}

#[test]
fn the_edit_log_says_from_where_the_text_changed() {
    let mut d = doc("a\nb\nc\n");
    let g0 = d.edit_gen();
    assert_eq!(d.edits_since(g0), None);
    d.set_selection(&Selection::point(4));
    d.type_text("X");
    d.set_selection(&Selection::point(2));
    d.type_text("Y");
    assert_eq!(d.edits_since(g0), Some(2), "the smallest position");
    let g1 = d.edit_gen();
    d.undo();
    assert_eq!(d.edits_since(g1), Some(2), "undo is an edit too");
    assert_eq!(d.edits_since(d.edit_gen()), None);
}
