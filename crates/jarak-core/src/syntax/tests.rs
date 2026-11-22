//! Golden tests: a fixture per language and the kind of its key constructs (state carried from
//! line to line as the window does), user-defined languages, the Notepad++ UDL import, folding
//! markers and the incremental state cache.

use super::*;
use Kind as K;

/// Every token of `src` lexed as `lang` (carrying the state), with its text.
fn tokens(lang: &Lang, src: &str) -> Vec<(String, Kind)> {
    let mut st = State::Normal;
    let mut out = Vec::new();
    for line in src.lines() {
        let (toks, next) = lex_line(lang, line, st);
        st = next;
        let chars: Vec<char> = line.chars().collect();
        for t in toks {
            out.push((chars[t.start..t.end].iter().collect(), t.kind));
        }
    }
    out
}

/// Checks that each `(text, kind)` appears as a token (in order).
#[track_caller]
fn golden(name: &str, src: &str, want: &[(&str, Kind)]) {
    let lang = builtin(name).unwrap_or_else(|| panic!("no language {name}"));
    let got = tokens(lang, src);
    let mut at = 0;
    for (text, kind) in want {
        match got[at..].iter().position(|(t, k)| t == text && k == kind) {
            Some(p) => at += p + 1,
            None => panic!("{name}: no {kind:?} token {text:?} after #{at} in\n{got:#?}"),
        }
    }
}

#[test]
fn c_and_cpp() {
    golden(
        "C",
        "#include <stdio.h>\nint main(void) { /* hi */\n  return 0x1F; // done\n}",
        &[
            ("#include <stdio.h>", K::Preprocessor),
            ("int", K::Type),
            ("void", K::Type),
            ("/* hi */", K::Comment),
            ("return", K::Keyword),
            ("0x1F", K::Number),
            ("// done", K::Comment),
        ],
    );
    golden(
        "C++",
        "template<class T>\nauto x = nullptr; /* a\nb */ std::string s = \"q\\\"\";",
        &[
            ("template", K::Keyword),
            ("class", K::Keyword),
            ("auto", K::Keyword),
            ("nullptr", K::Constant),
            ("/* a", K::Comment),
            ("b */", K::Comment),
            ("string", K::Type),
            ("\"q\\\"\"", K::String),
        ],
    );
}

#[test]
fn csharp_rust_go_java() {
    golden(
        "C#",
        "#region X\npublic async Task<int> F() => await G(@\"c:\\x\");",
        &[
            ("#region X", K::Preprocessor),
            ("public", K::Keyword),
            ("async", K::Keyword),
            ("int", K::Type),
            ("await", K::Keyword),
            ("@\"c:\\x\"", K::String),
        ],
    );
    golden(
        "Rust",
        "fn f<'a>(s: &'a str) -> Option<char> { 'x' } // ok\nlet n = 1_000u32; let s = \"line\nnext\";",
        &[
            ("fn", K::Keyword),
            ("str", K::Type),
            ("Option", K::Type),
            ("char", K::Type),
            ("'x'", K::String),
            ("// ok", K::Comment),
            ("let", K::Keyword),
            ("1_000u32", K::Number),
            ("\"line", K::String),
            ("next\"", K::String),
        ],
    );
    golden(
        "Go",
        "package main\nfunc f() error { return nil } // c\ns := `raw\nline`",
        &[
            ("package", K::Keyword),
            ("func", K::Keyword),
            ("error", K::Type),
            ("nil", K::Constant),
            ("// c", K::Comment),
            ("`raw", K::String),
            ("line`", K::String),
        ],
    );
    golden(
        "Java",
        "public class A { static final int N = 3; String s = null; }",
        &[
            ("public", K::Keyword),
            ("class", K::Keyword),
            ("int", K::Type),
            ("3", K::Number),
            ("String", K::Type),
            ("null", K::Constant),
        ],
    );
}

#[test]
fn javascript_typescript_python() {
    golden(
        "JavaScript",
        "const f = async () => `t ${x}\nend`; // c",
        &[
            ("const", K::Keyword),
            ("async", K::Keyword),
            ("`t ${x}", K::String),
            ("end`", K::String),
            ("// c", K::Comment),
        ],
    );
    golden(
        "TypeScript",
        "interface P { readonly n: number }\ntype T = string | undefined;",
        &[
            ("interface", K::Keyword),
            ("readonly", K::Keyword),
            ("number", K::Type),
            ("type", K::Keyword),
            ("string", K::Type),
            ("undefined", K::Constant),
        ],
    );
    golden(
        "Python",
        "@dataclass\ndef f(x: int) -> None:\n    \"\"\"doc\n    more\"\"\"\n    return True  # yes",
        &[
            ("@dataclass", K::Preprocessor),
            ("def", K::Keyword),
            ("int", K::Type),
            ("None", K::Constant),
            ("\"\"\"doc", K::String),
            ("    more\"\"\"", K::String),
            ("return", K::Keyword),
            ("True", K::Constant),
            ("# yes", K::Comment),
        ],
    );
}

#[test]
fn powershell_batch_shell() {
    golden(
        "PowerShell",
        "<# block\n#>\nfunction Get-X { param($Name) if ($Name -eq $null) { 'q' } } # c",
        &[
            ("<# block", K::Comment),
            ("#>", K::Comment),
            ("function", K::Keyword),
            ("param", K::Keyword),
            ("$Name", K::Variable),
            ("-eq", K::Keyword),
            ("$null", K::Constant),
            ("'q'", K::String),
            ("# c", K::Comment),
        ],
    );
    golden(
        "Batch",
        "@echo off\nREM 주석\n:: comment\nset NAME=%1\nif exist \"%NAME%\" goto :end",
        &[
            ("echo", K::Keyword),
            ("REM 주석", K::Comment),
            (":: comment", K::Comment),
            ("set", K::Keyword),
            ("%1", K::Variable),
            ("if", K::Keyword),
            ("exist", K::Keyword),
            ("\"%NAME%\"", K::String),
            ("goto", K::Keyword),
        ],
    );
    golden(
        "Shell",
        "#!/bin/bash\nfor f in *.txt; do echo \"$f\" ${#f}; done # 끝",
        &[
            ("#!/bin/bash", K::Comment),
            ("for", K::Keyword),
            ("in", K::Keyword),
            ("do", K::Keyword),
            ("\"$f\"", K::String),
            ("${#f}", K::Variable),
            ("done", K::Keyword),
            ("# 끝", K::Comment),
        ],
    );
}

#[test]
fn html_with_script_and_style_css_xml() {
    golden(
        "HTML",
        "<!-- c -->\n<div class=\"a\" id='b'>&amp; 한글</div>\n<script>let x = 1; // j\n</script><style>p { color: red; }</style>",
        &[
            ("<!--", K::Comment),
            (" c -->", K::Comment),
            ("<div", K::Tag),
            ("class", K::Attribute),
            ("\"a\"", K::String),
            ("id", K::Attribute),
            ("'b'", K::String),
            (">", K::Tag),
            ("&amp;", K::Constant),
            ("</div", K::Tag),
            ("<script", K::Tag),
            ("let", K::Keyword),
            ("1", K::Number),
            ("// j", K::Comment),
            ("</script", K::Tag),
            ("<style", K::Tag),
            ("color", K::Attribute),
            ("</style", K::Tag),
        ],
    );
    golden(
        "CSS",
        "/* c */\n@media screen { .a:hover { color: #ff00aa; margin: 1.5em !important; } }",
        &[
            ("/* c */", K::Comment),
            ("@media", K::Keyword),
            ("color", K::Attribute),
            ("#ff00aa", K::Number),
            ("margin", K::Attribute),
            ("1.5em", K::Number),
            ("!important", K::Keyword),
        ],
    );
    golden(
        "XML",
        "<?xml version=\"1.0\"?>\n<root a=\"1\"><![CDATA[x<y]]><!-- c --></root>",
        &[
            ("<?xml", K::Preprocessor),
            ("version", K::Attribute),
            ("\"1.0\"", K::String),
            ("?>", K::Preprocessor),
            ("<root", K::Tag),
            ("a", K::Attribute),
            ("<![CDATA[", K::String),
            ("x<y]]>", K::String),
            ("<!--", K::Comment),
        ],
    );
}

#[test]
fn json_yaml_toml_ini() {
    golden(
        "JSON",
        "{\"name\": \"한글\", \"n\": -1.5e3, \"ok\": true, \"x\": null}",
        &[
            ("\"name\"", K::Attribute),
            ("\"한글\"", K::String),
            ("\"n\"", K::Attribute),
            ("1.5e3", K::Number),
            ("true", K::Constant),
            ("null", K::Constant),
        ],
    );
    golden(
        "YAML",
        "# c\nname: \"x\"\nitems:\n  - true\n  - &anchor 3\nurl: http://a",
        &[
            ("# c", K::Comment),
            ("name", K::Attribute),
            ("\"x\"", K::String),
            ("items", K::Attribute),
            ("true", K::Constant),
            ("&anchor", K::Variable),
            ("3", K::Number),
            ("url", K::Attribute),
        ],
    );
    golden(
        "TOML",
        "[package]\nname = \"jarak\" # c\nversion = '0.0.1'\ntext = \"\"\"\nmulti\n\"\"\"\non = true\nwhen = 2026-11-22",
        &[
            ("[package]", K::Keyword),
            ("name", K::Attribute),
            ("\"jarak\"", K::String),
            ("# c", K::Comment),
            ("version", K::Attribute),
            ("\"\"\"", K::String),
            ("multi", K::String),
            ("true", K::Constant),
            ("2026-11-22", K::Number),
        ],
    );
    golden(
        "INI",
        "; c\n[section]\nkey=value\n# also",
        &[
            ("; c", K::Comment),
            ("[section]", K::Keyword),
            ("key", K::Attribute),
            ("# also", K::Comment),
        ],
    );
}

#[test]
fn sql_markdown_lua_php_ruby() {
    golden(
        "SQL",
        "SELECT name FROM t WHERE id = 1 -- c\n/* b */ insert into x values ('it''s');",
        &[
            ("SELECT", K::Keyword),
            ("FROM", K::Keyword),
            ("WHERE", K::Keyword),
            ("1", K::Number),
            ("-- c", K::Comment),
            ("/* b */", K::Comment),
            ("insert", K::Keyword),
            ("'it'", K::String),
        ],
    );
    golden(
        "Markdown",
        "# 제목\nSome *em* and **strong** with `code` and [link](http://x).\n> quote\n- item\n```rust\nfn main() {}\n```\nafter",
        &[
            ("# 제목", K::Heading),
            ("*em*", K::Emphasis),
            ("**strong**", K::Emphasis),
            ("`code`", K::Code),
            ("[link]", K::Link),
            ("(http://x)", K::String),
            (">", K::Comment),
            ("-", K::Operator),
            ("```rust", K::Code),
            ("fn", K::Keyword),
            ("```", K::Code),
        ],
    );
    golden(
        "Lua",
        "--[[ block\n]] local x = nil -- c\nlocal s = [[long\nstring]]",
        &[
            ("--[[ block", K::Comment),
            ("]]", K::Comment),
            ("local", K::Keyword),
            ("nil", K::Constant),
            ("-- c", K::Comment),
            ("[[long", K::String),
            ("string]]", K::String),
        ],
    );
    golden(
        "PHP",
        "<p><?php echo $name; // c ?></p>\n<?php\nfunction f() { return true; }\n?>",
        &[
            ("<p", K::Tag),
            ("<?php", K::Preprocessor),
            ("echo", K::Keyword),
            ("$name", K::Variable),
            ("?>", K::Preprocessor),
            ("</p", K::Tag),
            ("<?php", K::Preprocessor),
            ("function", K::Keyword),
            ("return", K::Keyword),
            ("true", K::Constant),
            ("?>", K::Preprocessor),
        ],
    );
    golden(
        "Ruby",
        "=begin\ndoc\n=end\ndef hi(name) @x = \"#{name}\" end # c\n$g = nil",
        &[
            ("=begin", K::Comment),
            ("doc", K::Comment),
            ("=end", K::Comment),
            ("def", K::Keyword),
            ("@x", K::Variable),
            ("end", K::Keyword),
            ("# c", K::Comment),
            ("$g", K::Variable),
            ("nil", K::Constant),
        ],
    );
}

#[test]
fn every_listed_language_is_built_in_and_detected() {
    for (file, name) in [
        ("a.c", "C"),
        ("a.hpp", "C++"),
        ("a.cs", "C#"),
        ("a.rs", "Rust"),
        ("a.go", "Go"),
        ("A.java", "Java"),
        ("a.mjs", "JavaScript"),
        ("a.tsx", "TypeScript"),
        ("a.py", "Python"),
        ("a.ps1", "PowerShell"),
        ("a.cmd", "Batch"),
        ("a.sh", "Shell"),
        ("a.html", "HTML"),
        ("a.css", "CSS"),
        ("a.json", "JSON"),
        ("a.yml", "YAML"),
        ("Cargo.toml", "TOML"),
        ("a.xml", "XML"),
        ("a.sql", "SQL"),
        ("README.md", "Markdown"),
        ("a.ini", "INI"),
        ("a.lua", "Lua"),
        ("a.php", "PHP"),
        ("a.rb", "Ruby"),
    ] {
        assert_eq!(
            detect(file, "", &[]).map(|l| l.name.as_str()),
            Some(name),
            "{file}"
        );
    }
    assert_eq!(
        detect("script", "#!/usr/bin/env python3", &[])
            .unwrap()
            .name,
        "Python"
    );
    assert_eq!(detect("run", "#!/bin/bash -e", &[]).unwrap().name, "Shell");
    assert_eq!(
        detect("x", "<?xml version=\"1.0\"?>", &[]).unwrap().name,
        "XML"
    );
    assert!(detect("notes.txt", "hello", &[]).is_none(), "plain text");
}

#[test]
fn a_user_defined_language_from_json_highlights_and_folds() {
    let json = r#"{
        "name": "Kimlang",
        "extensions": ["kim"],
        "case_sensitive": false,
        "line_comment": ["//"],
        "block_comment": [{"open": "(*", "close": "*)", "multiline": true}],
        "strings": [{"open": "\"", "close": "\"", "escape": "\\"}],
        "keywords": ["BEGIN", "END", "SAY"],
        "operators": "+-=",
        "fold": [["BEGIN", "END"]]
    }"#;
    let l = parse_udl(json).unwrap();
    let got = tokens(&l, "begin (* 주석\n*) say \"hi\" // c\nx = 1\nEND");
    for (t, k) in [
        ("begin", K::Keyword),
        ("(* 주석", K::Comment),
        ("*)", K::Comment),
        ("say", K::Keyword),
        ("\"hi\"", K::String),
        ("// c", K::Comment),
        ("=", K::Operator),
        ("1", K::Number),
        ("END", K::Keyword),
    ] {
        assert!(
            got.iter().any(|(x, kk)| x == t && *kk == k),
            "{t:?} {k:?} in {got:?}"
        );
    }
    let lines: Vec<String> = "BEGIN\n  say (* END *) \"END\"\n  begin\n  end\nend"
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(
        fold_ranges(&l, &lines),
        vec![(0, 4), (2, 3)],
        "markers in comments and strings do not count"
    );
    assert_eq!(
        detect("a.kim", "", std::slice::from_ref(&l)).unwrap().name,
        "Kimlang"
    );
    assert!(parse_udl(r#"{"name": ""}"#).is_err());
}

#[test]
fn a_notepad_plus_plus_udl_is_imported_and_highlights_a_sample() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<NotepadPlus>
    <UserLang name="Simple &amp; Co" ext="smp sim" udlVersion="2.1">
        <Settings>
            <Global caseIgnored="yes" allowFoldOfComments="no" foldCompact="no" forcePureLC="0" decimalSeparator="0" />
        </Settings>
        <KeywordLists>
            <Keywords name="Comments">00# 01 02 03{* 04*}</Keywords>
            <Keywords name="Numbers, prefix1"></Keywords>
            <Keywords name="Operators1">+ - = &lt;</Keywords>
            <Keywords name="Folders in code1, open">do</Keywords>
            <Keywords name="Folders in code1, middle"></Keywords>
            <Keywords name="Folders in code1, close">done</Keywords>
            <Keywords name="Keywords1">print loop</Keywords>
            <Keywords name="Keywords2">int text</Keywords>
            <Keywords name="Keywords3">yes no</Keywords>
            <Keywords name="Delimiters">00&quot; 01\ 02&quot; 03' 04 05'</Keywords>
        </KeywordLists>
        <Styles><WordsStyle name="DEFAULT" fgColor="000000" /></Styles>
    </UserLang>
</NotepadPlus>"#;
    let l = import_npp_udl(xml).unwrap();
    assert_eq!(l.name, "Simple & Co");
    assert_eq!(l.extensions, ["smp", "sim"]);
    assert!(!l.case_sensitive);
    assert_eq!(l.line_comment, ["#"]);
    assert_eq!(l.block_comment[0].open, "{*");
    assert_eq!(l.operators, "+-=<");
    assert_eq!(l.fold, vec![["do".to_string(), "done".to_string()]]);
    assert_eq!(l.strings.len(), 2);
    assert_eq!(l.strings[0].escape, Some('\\'));
    let got = tokens(
        &l,
        "PRINT \"a\\\"b\" 'c' # note\nint x = yes {* 블록 *}\ndo\nloop\ndone",
    );
    for (t, k) in [
        ("PRINT", K::Keyword),
        ("\"a\\\"b\"", K::String),
        ("'c'", K::String),
        ("# note", K::Comment),
        ("int", K::Type),
        ("=", K::Operator),
        ("yes", K::Constant),
        ("{* 블록 *}", K::Comment),
        ("loop", K::Keyword),
    ] {
        assert!(
            got.iter().any(|(x, kk)| x == t && *kk == k),
            "{t:?} {k:?} in {got:?}"
        );
    }
    let lines: Vec<String> = "do\nloop\ndone".lines().map(str::to_string).collect();
    assert_eq!(fold_ranges(&l, &lines), vec![(0, 2)]);
    // Round trip through the JSON the window keeps.
    let json = serde_json::to_string_pretty(&l).unwrap();
    assert_eq!(parse_udl(&json).unwrap(), l);
    assert!(import_npp_udl("<NotepadPlus/>").is_err());
}

#[test]
fn the_highlighter_keeps_states_and_drops_them_after_an_edit() {
    let rust = builtin("Rust").unwrap();
    let mut lines = vec![
        "/* open".to_string(),
        "still".to_string(),
        "close */ fn".to_string(),
        "fn x".to_string(),
    ];
    let mut h = Highlighter::default();
    let text = |ls: &Vec<String>| {
        let ls = ls.clone();
        move |i: usize| ls[i].clone()
    };
    let t = h.line(rust, 3, &text(&lines));
    assert_eq!(t[0].kind, K::Keyword);
    assert_eq!(h.known(), 5, "the lines before were lexed once");
    let mid = h.line(rust, 1, &text(&lines));
    assert_eq!(mid[0].kind, K::Comment, "inside the block comment");
    // An edit on line 0 closes the comment: the states after it are dropped and lexed again.
    lines[0] = "/* open */".into();
    h.invalidate_from(0);
    let mid = h.line(rust, 1, &text(&lines));
    assert!(mid.is_empty(), "plain text now: {mid:?}");
    let last = h.line(rust, 2, &text(&lines));
    assert!(last.iter().any(|t| t.kind == K::Keyword));
}

/// Step 98 (#117): a 10 MB Rust file — the first jump to its end lexes every line once, a
/// screenful costs little after that.
/// `cargo test --release -p jarak-core highlighting_10_mb -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn highlighting_10_mb() {
    use std::time::Instant;
    let unit = "/* block\n   comment */\nfn main() { let s = \"한글 string\"; // line comment\n    let n = 0x1F + 42; }\n";
    let text = unit.repeat(10 * 1024 * 1024 / unit.len() + 1);
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let rust = builtin("Rust").unwrap();
    let mut h = Highlighter::default();
    let t = Instant::now();
    let last = lines.len() - 1;
    let _ = h.line(rust, last, &|i| lines[i].clone());
    let first_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    for l in last - 60..=last {
        let _ = h.line(rust, l, &|i| lines[i].clone());
    }
    let screen_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    h.invalidate_from(last - 100);
    for l in last - 60..=last {
        let _ = h.line(rust, l, &|i| lines[i].clone());
    }
    let edit_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "10 MB Rust ({} lines): first jump to the end {first_ms:.0} ms, a screen (61 lines) {screen_ms:.2} ms, after an edit 100 lines up {edit_ms:.2} ms",
        lines.len()
    );
    assert!(first_ms < 2000.0 && screen_ms < 16.0);
}
