//! Fold ranges, symbols and bracket matching for the Step 98 languages and Markdown.

use super::*;
use crate::syntax::builtin;

fn lines(s: &str) -> Vec<String> {
    s.lines().map(str::to_string).collect()
}

fn ranges(lang: &str, src: &str) -> Vec<(usize, usize)> {
    folds(builtin(lang), &lines(src))
        .iter()
        .map(|f| (f.start, f.end))
        .collect()
}

fn names(lang: &str, src: &str) -> Vec<(String, String, usize)> {
    symbols(builtin(lang), &lines(src))
        .into_iter()
        .map(|s| (s.name, s.kind, s.line))
        .collect()
}

#[track_caller]
fn has(syms: &[(String, String, usize)], name: &str, kind: &str, line: usize) {
    assert!(
        syms.iter()
            .any(|(n, k, l)| n == name && k == kind && *l == line),
        "{name} {kind} {line} in {syms:?}"
    );
}

#[test]
fn brackets_fold_outside_strings_and_comments_with_levels() {
    let src = "fn main() {\n    let s = \"{\";\n    if x {\n        y();\n    }\n    // }\n}\n/* a\n   b */";
    assert_eq!(ranges("Rust", src), vec![(0, 6), (2, 4), (7, 8)]);
    let f = folds(builtin("Rust"), &lines(src));
    assert_eq!(f.iter().map(|x| x.level).collect::<Vec<_>>(), vec![1, 2, 1]);
    assert_eq!(
        ranges("JSON", "{\n  \"a\": [\n    1\n  ]\n}"),
        vec![(0, 4), (1, 3)]
    );
}

#[test]
fn indentation_sections_markdown_and_tags_fold() {
    assert_eq!(
        ranges(
            "Python",
            "class A:\n    def f(self):\n        pass\n\n    x = 1\ny = 2"
        ),
        vec![(0, 4), (1, 2)]
    );
    assert_eq!(
        ranges("YAML", "a:\n  b: 1\n  c:\n    - 2\nd: 3"),
        vec![(0, 3), (2, 3)]
    );
    assert_eq!(
        ranges("INI", "[one]\na=1\n\n[two]\nb=2"),
        vec![(0, 1), (3, 4)]
    );
    assert_eq!(
        ranges(
            "Markdown",
            "# 제목\ntext\n## sub\nmore\n```rust\nfn a() {}\n```\n# next\nend"
        ),
        vec![(0, 6), (2, 6), (4, 6), (7, 8)]
    );
    assert_eq!(
        ranges(
            "HTML",
            "<div>\n  <p>x</p>\n  <br>\n  <ul>\n    <li>a</li>\n  </ul>\n</div>"
        ),
        vec![(0, 6), (3, 5)]
    );
    let udl: Lang = syntax::parse_udl(r#"{"name": "K", "fold": [["begin", "end"]]}"#).unwrap();
    assert_eq!(folds(Some(&udl), &lines("begin\nx\nend")).len(), 1);
    assert!(
        folds(None, &lines("{\n}")).is_empty(),
        "plain text does not fold"
    );
}

#[test]
fn symbols_of_each_language_family() {
    let s = names(
        "Rust",
        "pub struct P;\nimpl P {\n    pub fn new() -> Self { P }\n}\nfn main() {}\n// fn hidden() {}",
    );
    has(&s, "P", "struct", 0);
    has(&s, "P", "impl", 1);
    has(&s, "new", "fn", 2);
    has(&s, "main", "fn", 4);
    assert!(!s.iter().any(|x| x.0 == "hidden"), "not in comments");
    let s = names(
        "Python",
        "class A:\n    def f(self):\n        pass\nasync def g():\n    pass",
    );
    has(&s, "A", "class", 0);
    has(&s, "f", "def", 1);
    has(&s, "g", "def", 3);
    let s = names(
        "C",
        "#include <x.h>\nstatic int add(int a, int b) {\n    if (a) { return a; }\n}\nvoid run(void)\n{\n}",
    );
    has(&s, "add", "function", 1);
    has(&s, "run", "function", 4);
    assert!(!s.iter().any(|x| x.0 == "if"));
    let s = names(
        "Go",
        "package main\ntype T struct {}\nfunc (t T) M() {}\nfunc main() {}",
    );
    has(&s, "T", "type", 1);
    has(&s, "main", "func", 3);
    let s = names(
        "JavaScript",
        "export default function app() {}\nconst onClick = (e) => {\nclass Store {}",
    );
    has(&s, "app", "function", 0);
    has(&s, "onClick", "function", 1);
    has(&s, "Store", "class", 2);
    let s = names(
        "Markdown",
        "# 제목\ntext\n## 소제목 ##\n```\n# not a heading\n```\n### three",
    );
    assert_eq!(
        s,
        vec![
            ("제목".into(), "heading 1".into(), 0),
            ("소제목".into(), "heading 2".into(), 2),
            ("three".into(), "heading 3".into(), 6)
        ]
    );
    let s = names("TOML", "[package]\nname = \"x\"\n[dependencies]");
    has(&s, "package", "section", 0);
    has(&s, "dependencies", "section", 2);
    let s = names("Batch", "@echo off\n:start\ngoto :end\n:end");
    has(&s, "start", "label", 1);
    has(&s, "end", "label", 3);
    let s = names(
        "SQL",
        "CREATE TABLE users (\n  id int\n);\ncreate view v as select 1;",
    );
    has(&s, "users", "table", 0);
    has(&s, "v", "view", 3);
    let s = names("CSS", "body {\n  color: red;\n}\n.a:hover {\n}");
    has(&s, "body", "rule", 0);
    has(&s, ".a:hover", "rule", 3);
    let s = names("Shell", "deploy() {\n  echo hi\n}\nfunction build {\n}");
    has(&s, "deploy", "function", 0);
    has(&s, "build", "function", 3);
    let s = names("HTML", "<h1>Title</h1>\n<h2 id=\"x\">Sub</h2>");
    has(&s, "Title", "heading 1", 0);
    has(&s, "Sub", "heading 2", 1);
    let s = names("PowerShell", "function Get-Thing {\n}");
    has(&s, "Get-Thing", "function", 0);
    let s = names(
        "Ruby",
        "module M\n  class C\n    def run\n    end\n  end\nend",
    );
    has(&s, "M", "module", 0);
    has(&s, "run", "def", 2);
    let s = names(
        "JSON",
        "{\n  \"name\": \"x\",\n  \"deps\": {\n    \"a\": 1\n  }\n}",
    );
    has(&s, "name", "key", 1);
    has(&s, "deps", "key", 2);
    assert!(
        !s.iter().any(|x| x.0 == "a"),
        "only the outermost keys: {s:?}"
    );
    let s = names("YAML", "name: x\ndeps:\n  a: 1");
    has(&s, "name", "key", 0);
    has(&s, "deps", "key", 1);
    assert_eq!(s.len(), 2);
    let s = names("Lua", "local function f()\nend\nM.g = function()\nend");
    has(&s, "f", "function", 0);
    has(&s, "M.g", "function", 2);
}

#[test]
fn brackets_match_both_ways_skipping_strings() {
    let src = lines("f(a, \")\", [1, 2]) {\n  x\n}");
    let rust = builtin("Rust");
    assert_eq!(
        matching_bracket(rust, &src, 0, 1),
        Some(((0, 1), (0, 16))),
        "( to the last )"
    );
    assert_eq!(
        matching_bracket(rust, &src, 0, 17),
        Some(((0, 16), (0, 1))),
        "just after ) works too"
    );
    assert_eq!(
        matching_bracket(rust, &src, 0, 18),
        Some(((0, 18), (2, 0))),
        "{{ across lines"
    );
    assert_eq!(matching_bracket(rust, &src, 2, 0), Some(((2, 0), (0, 18))));
    assert_eq!(matching_bracket(rust, &src, 1, 2), None, "no bracket here");
}

#[test]
fn folded_lines_follow_edits_and_vertical_moves_skip_them() {
    use crate::document::Document;
    use crate::selection::Selection;
    let mut d = Document::from_text("fn a() {\n    x();\n    y();\n}\nz\n");
    d.set_folded(0, true);
    assert_eq!(d.folded_lines(), vec![0]);
    // A line typed above moves the fold with its text.
    d.set_selection(&Selection::point(0));
    d.type_text("// top\n");
    assert_eq!(
        d.folded_lines(),
        vec![1],
        "the folded marker moved with its line"
    );
    d.hidden = vec![(2, 4)];
    assert_eq!(d.hidden_range(3), Some((2, 4)));
    assert_eq!(d.hidden_range(5), None);
    // Down from the fold's first line lands after the fold; up comes back to it.
    d.set_selection(&Selection::point(d.text().line_to_char(1)));
    d.move_lines(1, false);
    assert_eq!(d.text().char_to_line(d.selection().primary().head), 5);
    d.move_lines(-1, false);
    assert_eq!(d.text().char_to_line(d.selection().primary().head), 1);
    // Unfolding, and keeping only the folds that still exist.
    d.set_folded(1, false);
    assert!(d.folded_lines().is_empty());
    d.set_folded(1, true);
    d.retain_folded(|l| l != 1);
    assert!(d.folded_lines().is_empty());
}
