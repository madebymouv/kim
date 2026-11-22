//! Completion, API files, paths, snippets, matching, and the document's typing aids.

use super::*;
use crate::document::Document;
use crate::document::assist::{Aids, IndentMode};
use crate::selection::{Range, Selection};
use crate::syntax::{State, builtin, lex_line};

fn doc(s: &str, aids: Aids) -> Document {
    let mut d = Document::from_text(s);
    d.aids = aids;
    d
}

fn all() -> Aids {
    Aids {
        auto_close: true,
        close_tags: true,
        indent: IndentMode::Smart,
        quotes: "\"'".into(),
        colon_indents: false,
        markup: false,
        unit: String::new(),
    }
}

fn at(d: &mut Document, p: usize) {
    d.set_selection(&Selection::point(p));
}

fn caret(d: &Document) -> usize {
    d.selection().primary().head
}

#[test]
fn words_keywords_api_and_snippets_complete_a_prefix() {
    let text = [
        "let counter = 1;",
        "count_all(counter); // Counter 한글단어",
        "12 c",
    ];
    let words = document_words(text.iter().copied(), "cou", "");
    assert_eq!(words, vec!["Counter", "count_all", "counter"]);
    assert!(document_words(text.iter().copied(), "한", "").contains(&"한글단어".to_string()));
    let rust = builtin("Rust");
    let snippets = snippets_with(&[]);
    let c = candidates("ma", &[], rust, None, &snippets);
    assert!(
        c.iter()
            .any(|x| x.text == "match" && x.kind == CandKind::Snippet),
        "the snippet wins over the keyword: {c:?}"
    );
    let api = builtin_api("C").unwrap();
    let c = candidates(
        "mal",
        &["malformed".into()],
        builtin("C"),
        Some(api),
        &snippets,
    );
    assert_eq!(c[0].text, "malformed");
    let m = c.iter().find(|x| x.text == "malloc").unwrap();
    assert_eq!(
        (m.kind, m.detail.as_str()),
        (CandKind::Function, "void * malloc(size_t size)")
    );
    assert!(candidates("zzz", &[], rust, None, &snippets).is_empty());
    assert_eq!(word_before("x = foo.ba", 10, ""), (8, "ba".to_string()));
}

#[test]
fn notepad_plus_plus_api_files_are_read() {
    let xml = r#"<?xml version="1.0"?><NotepadPlus><AutoComplete language="Kim">
      <Environment ignoreCase="yes" startFunc="[" stopFunc="]" paramSeparator=";"/>
      <KeyWord name="Draw" func="yes"><Overload retVal="void" descr="Draws"><Param name="x"/><Param name="y"/></Overload>
        <Overload retVal="void"><Param name="point"/></Overload></KeyWord>
      <KeyWord name="loop"/>
    </AutoComplete></NotepadPlus>"#;
    let api = parse_npp_api(xml).unwrap();
    assert_eq!(
        (
            api.language.as_str(),
            api.ignore_case,
            api.start_func,
            api.separator
        ),
        ("Kim", true, '[', ';')
    );
    let w = api.get("draw").expect("ignoreCase");
    assert_eq!(w.overloads.len(), 2);
    assert_eq!(w.overloads[0].params, vec!["x", "y"]);
    assert!(!api.get("loop").unwrap().func);
    let (sig, ranges) = signature(w, &w.overloads[0], api.separator);
    assert_eq!(sig, "void Draw(x; y)");
    assert_eq!(ranges, vec![(10, 11), (13, 14)]);
    assert!(parse_npp_api("<NotepadPlus/>").is_err());
    // The call the caret is in, its parameter, outside strings.
    let line = "Draw[a; \"x;y\"; ";
    let toks = lex_line(builtin("C").unwrap(), line, State::Normal).0;
    assert_eq!(
        call_at(line, line.chars().count(), &toks, &api),
        Some(("Draw".into(), 2))
    );
    let py = builtin_api("Python").unwrap();
    assert_eq!(
        call_at("print(range(1, ", 15, &[], py),
        Some(("range".into(), 1))
    );
    assert_eq!(call_at("print(x) ", 9, &[], py), None);
}

#[test]
fn paths_complete_inside_the_documents_folder_and_never_on_the_network() {
    let d = std::env::temp_dir().join(format!("jarak-paths-{}", std::process::id()));
    std::fs::create_dir_all(d.join("images")).unwrap();
    std::fs::write(d.join("index.md"), "").unwrap();
    std::fs::write(d.join("Image.png"), "").unwrap();
    let c = path_candidates("im", Some(&d));
    let names: Vec<&str> = c.iter().map(|x| x.text.as_str()).collect();
    assert_eq!(names, vec!["Image.png", "images\\"]);
    assert_eq!(path_candidates("images\\", Some(&d)).len(), 0);
    let abs = format!("{}\\in", d.display());
    assert_eq!(
        path_candidates(&abs, None).len(),
        1,
        "absolute paths need no document folder"
    );
    assert!(
        path_candidates("\\\\server\\share\\", Some(&d)).is_empty(),
        "no network"
    );
    assert!(path_candidates("//server/x", Some(&d)).is_empty());
    assert!(
        path_candidates("im", None).is_empty(),
        "an untitled document has no folder"
    );
    assert_eq!(path_before("src=\"./ima", 10), (5, "./ima".into()));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn snippets_expand_with_tab_stops_mirrors_and_indentation() {
    let e = expand("fn ${1:name}($2) {\n\t$0\n}", "    ", "    ", "\n");
    assert_eq!(e.text, "fn name() {\n        \n    }");
    assert_eq!(e.stops[0], (1, vec![(3, 7)]));
    assert_eq!(e.stops[1], (2, vec![(8, 8)]));
    assert_eq!(e.stops[2], (0, vec![(20, 20)]));
    let e = expand("$1 = $1 \\$x", "", "\t", "\n");
    assert_eq!(e.text, " =  $x");
    assert_eq!(e.stops[0], (1, vec![(0, 0), (3, 3)]), "a mirrored field");
    assert_eq!(
        e.stops.last().unwrap(),
        &(0, vec![(6, 6)]),
        "the end when there is no $0"
    );
    // The user's file adds and replaces by prefix and language.
    let user = parse_snippets(r#"{"Rust": {"fn": {"body": "fn mine() {}"}}, "*": {"sig": {"body": ["-- ", "Kim"], "description": "signature"}}}"#).unwrap();
    let all = snippets_with(&user);
    let rust_fn: Vec<_> = all
        .iter()
        .filter(|s| s.prefix == "fn" && s.applies(Some("rust")))
        .collect();
    assert_eq!(rust_fn.len(), 1);
    assert_eq!(rust_fn[0].body, "fn mine() {}");
    assert!(
        all.iter()
            .any(|s| s.prefix == "sig" && s.body == "-- \nKim" && s.applies(None))
    );
    assert!(parse_snippets("{\"*\": {\"\": {\"body\": \"x\"}}}").is_err());
    assert!(parse_snippets("not json").is_err());
    // In a document: the first stop selected, Tab to the next, the last one ends it.
    let mut d = doc("    fn", all_aids_off());
    let e = expand("fn ${1:name}($2) {\n\t$0\n}", "    ", "    ", "\n");
    d.insert_snippet(4, 6, &e);
    assert_eq!(d.text().to_string(), "    fn name() {\n        \n    }");
    let r = d.selection().primary();
    assert_eq!((r.from(), r.to()), (7, 11), "name selected");
    d.type_assisted("go");
    assert!(d.snippet_step(false));
    assert_eq!(caret(&d), 10, "inside the parentheses");
    assert!(d.snippet_step(false));
    assert!(!d.snippet_active(), "$0 ends it");
    assert_eq!(d.text().line_to_char(1) + 8, caret(&d));
    d.undo();
    assert_eq!(
        d.text().to_string(),
        "    fn name() {\n        \n    }",
        "undo takes back the typing"
    );
}

fn all_aids_off() -> Aids {
    Aids::default()
}

#[test]
fn brackets_and_quotes_close_step_over_wrap_and_delete_in_pairs() {
    let mut d = doc("", all());
    d.type_assisted("(");
    assert_eq!((d.text().to_string(), caret(&d)), ("()".into(), 1));
    d.type_assisted("a");
    d.type_assisted(")");
    assert_eq!(
        (d.text().to_string(), caret(&d)),
        ("(a)".into(), 3),
        "typed over the closer"
    );
    d.type_assisted(")");
    assert_eq!(
        d.text().to_string(),
        "(a))",
        "a closer it did not insert is typed"
    );
    // Backspace in an empty pair it inserted removes both.
    let mut d = doc("x ", all());
    at(&mut d, 2);
    d.type_assisted("[");
    d.delete_backward_assisted();
    assert_eq!(d.text().to_string(), "x ");
    // Before a word nothing closes; a quote after a word does not close (don't).
    let mut d = doc("word", all());
    at(&mut d, 0);
    d.type_assisted("(");
    assert_eq!(d.text().to_string(), "(word");
    let mut d = doc("don", all());
    at(&mut d, 3);
    d.type_assisted("'");
    assert_eq!(d.text().to_string(), "don'");
    // Quotes pair; a selection is wrapped and stays selected.
    let mut d = doc("say hi", all());
    d.set_selection(&Selection::single(Range::new(4, 6)));
    d.type_assisted("\"");
    assert_eq!(d.text().to_string(), "say \"hi\"");
    let r = d.selection().primary();
    assert_eq!((r.from(), r.to()), (5, 7));
    // Every caret.
    let mut d = doc("a\nb\n", all());
    d.set_selection(&Selection::new(vec![Range::point(1), Range::point(3)], 0));
    d.type_assisted("{");
    assert_eq!(d.text().to_string(), "a{}\nb{}\n");
    // Off: plain typing.
    let mut d = doc("", all_aids_off());
    d.type_assisted("(");
    assert_eq!(d.text().to_string(), "(");
}

#[test]
fn tags_close_in_markup_and_enter_indents_smartly() {
    let mut a = all();
    a.markup = true;
    let mut d = doc("<div class=\"a>b\"", a.clone());
    let n = d.len_chars();
    at(&mut d, n);
    d.type_assisted(">");
    assert_eq!(
        (d.text().to_string(), caret(&d)),
        ("<div class=\"a>b\"></div>".into(), 17)
    );
    for (src, typed) in [
        ("<br", "<br>"),
        ("<p/", "<p/>"),
        ("</p", "</p>"),
        ("<!-- x", "<!-- x>"),
    ] {
        let mut d = doc(src, a.clone());
        let n = d.len_chars();
        at(&mut d, n);
        d.type_assisted(">");
        assert_eq!(d.text().to_string(), typed, "{src}");
    }
    // Smart: one level more after `{`, the closer on its own line between a pair.
    let mut d = doc("fn a() {}\n", all());
    at(&mut d, 8);
    d.newline();
    assert_eq!(d.text().to_string(), "fn a() {\n\t\n}\n");
    assert_eq!(caret(&d), 10);
    // The document's own unit (4 spaces); a closer typed on an empty line goes back a level.
    let mut d = doc("if x {\n    y\n    ", all());
    let n = d.len_chars();
    at(&mut d, n);
    d.type_assisted("}");
    assert_eq!(d.text().to_string(), "if x {\n    y\n}");
    // Python's colon.
    let mut a = all();
    a.colon_indents = true;
    a.unit = "    ".into();
    let mut d = doc("def f():\n", a);
    at(&mut d, 8);
    d.newline();
    assert_eq!(d.text().to_string(), "def f():\n    \n");
    // Keep and off.
    let mut a = all();
    a.indent = IndentMode::Keep;
    let mut d = doc("  x {\n", a.clone());
    at(&mut d, 5);
    d.newline();
    assert_eq!(d.text().to_string(), "  x {\n  \n");
    a.indent = IndentMode::Off;
    let mut d = doc("  x\n", a);
    at(&mut d, 3);
    d.newline();
    assert_eq!(d.text().to_string(), "  x\n\n");
}

#[test]
fn bracket_and_tag_matching_is_bounded_and_skips_strings() {
    let lines: Vec<String> = ["f(\")\", [1]) {", "  x", "}"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let rust = builtin("Rust").unwrap();
    let text_of = |n: usize| lines[n].clone();
    let mut toks = |n: usize| {
        let mut st = State::Normal;
        let mut out = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let (t, s) = lex_line(rust, l, st);
            st = s;
            if i == n {
                out = t;
            }
        }
        out
    };
    assert_eq!(
        match_bracket_by(0, 1, 3, &text_of, &mut toks, 100),
        Some(((0, 1), (0, 10)))
    );
    assert_eq!(
        match_bracket_by(0, 12, 3, &text_of, &mut toks, 100),
        Some(((0, 12), (2, 0)))
    );
    assert_eq!(
        match_bracket_by(0, 12, 3, &text_of, &mut toks, 1),
        None,
        "too far for the bound"
    );
    let html: Vec<String> = ["<div>", "  <div><br></div>", "</div>"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let h = builtin("HTML").unwrap();
    let text_of = |n: usize| html[n].clone();
    let mut toks = |n: usize| lex_line(h, &html[n], State::Normal).0;
    assert_eq!(
        match_tag_by(0, 2, 3, &text_of, &mut toks, 100),
        Some(((0, 1, 4), (2, 2, 5)))
    );
    assert_eq!(
        match_tag_by(2, 3, 3, &text_of, &mut toks, 100),
        Some(((2, 2, 5), (0, 1, 4)))
    );
    assert_eq!(
        match_tag_by(1, 4, 3, &text_of, &mut toks, 100),
        Some(((1, 3, 6), (1, 13, 16)))
    );
    assert_eq!(
        match_tag_by(1, 9, 3, &text_of, &mut toks, 100),
        None,
        "a void element"
    );
}
