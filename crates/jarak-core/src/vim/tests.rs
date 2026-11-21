//! The Vim engine against a buffer (`JARAK-FR-051`…`JARAK-FR-053`): `|` marks the cursor.

use super::*;
use std::cell::RefCell;

/// A clipboard for the `+` register.
#[derive(Default)]
struct Clip(RefCell<Option<String>>);

impl Host for Clip {
    fn clipboard(&self) -> Option<String> {
        self.0.borrow().clone()
    }
    fn set_clipboard(&self, s: &str) {
        *self.0.borrow_mut() = Some(s.to_string());
    }
}

struct T {
    vim: Vim,
    doc: Document,
    clip: Clip,
    last: Outcome,
}

/// A buffer with the cursor at `|` (LF line endings).
fn ed(s: &str) -> T {
    let at = s.find('|').expect("a cursor mark");
    let text = s.replacen('|', "", 1);
    let mut doc = Document::from_text(&text);
    let pos = text[..at].chars().count();
    doc.set_selection(&Selection::point(pos));
    T {
        vim: Vim::new(),
        doc,
        clip: Clip::default(),
        last: Outcome::default(),
    }
}

impl T {
    /// Keys: plain characters, `<Esc>`, `<CR>`, `<BS>`, `<C-r>`, `<C-v>`, `<Tab>`.
    fn k(&mut self, keys: &str) -> &mut T {
        let view = View { top: 0, rows: 10 };
        let mut rest = keys;
        while !rest.is_empty() {
            let (key, used) = if rest.starts_with('<')
                && let Some(end) = rest.find('>')
            {
                let name = &rest[1..end];
                let key = match name {
                    "Esc" => Key::Esc,
                    "CR" => Key::Enter,
                    "BS" => Key::Backspace,
                    "Tab" => Key::Tab,
                    "Left" => Key::Left,
                    "Right" => Key::Right,
                    "Up" => Key::Up,
                    "Down" => Key::Down,
                    n if n.starts_with("C-") => Key::Ctrl(n.chars().nth(2).unwrap()),
                    _ => Key::Char('<'),
                };
                if key == Key::Char('<') {
                    (key, 1)
                } else {
                    (key, end + 1)
                }
            } else {
                let c = rest.chars().next().unwrap();
                (Key::Char(c), c.len_utf8())
            };
            self.last = self.vim.key(&mut self.doc, key, &self.clip, view);
            rest = &rest[used..];
        }
        self
    }

    /// The text with `|` at the cursor (the head of the selection).
    fn s(&self) -> String {
        let t = self.doc.text().to_string();
        let head = self.doc.selection().primary().head;
        let b = self.doc.text().char_to_byte(head);
        format!("{}|{}", &t[..b], &t[b..])
    }

    fn text(&self) -> String {
        self.doc.text().to_string()
    }

    fn sel(&self) -> String {
        self.doc.selected_text()
    }
}

#[test]
fn motions_move_the_cursor_like_vim() {
    let mut t = ed("|one two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("w").s(), "one |two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("w").s(), "one two|-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("W").s(), "one two-three  |four\nfive\n\nsix seven\n");
    assert_eq!(t.k("e").s(), "one two-three  fou|r\nfive\n\nsix seven\n");
    assert_eq!(t.k("b").s(), "one two-three  |four\nfive\n\nsix seven\n");
    assert_eq!(t.k("B").s(), "one |two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("$").s(), "one two-three  fou|r\nfive\n\nsix seven\n");
    assert_eq!(t.k("0").s(), "|one two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("j").s(), "one two-three  four\n|five\n\nsix seven\n");
    // `w` stops at an empty line.
    assert_eq!(t.k("w").s(), "one two-three  four\nfive\n|\nsix seven\n");
    assert_eq!(t.k("G").s(), "one two-three  four\nfive\n\n|six seven\n");
    assert_eq!(t.k("gg").s(), "|one two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("3l").s(), "one| two-three  four\nfive\n\nsix seven\n");
    assert_eq!(t.k("2G").s(), "one two-three  four\n|five\n\nsix seven\n");
    assert_eq!(t.k("ge").s(), "one two-three  fou|r\nfive\n\nsix seven\n");
    let mut t = ed("  |  indented(a, [b]) x\n");
    assert_eq!(t.k("^").s(), "    |indented(a, [b]) x\n");
    assert_eq!(t.k("f(").s(), "    indented|(a, [b]) x\n");
    assert_eq!(t.k("%").s(), "    indented(a, [b]|) x\n");
    assert_eq!(t.k("%").s(), "    indented|(a, [b]) x\n");
    assert_eq!(t.k("t]").s(), "    indented(a, [|b]) x\n");
    assert_eq!(t.k("F(").s(), "    indented|(a, [b]) x\n");
    assert_eq!(t.k("fa").s(), "    indented(|a, [b]) x\n");
    assert_eq!(
        t.k(",").s(),
        "    indented(|a, [b]) x\n",
        "no `a` before on the line to the left of `(`"
    );
    let mut t = ed("|a.b.c.d\n");
    assert_eq!(t.k("f.;").s(), "a.b|.c.d\n");
    assert_eq!(t.k(",").s(), "a|.b.c.d\n");
    assert_eq!(t.k("2;").s(), "a.b.c|.d\n");
}

#[test]
fn paragraph_and_screen_motions() {
    let mut t = ed("|a\nb\n\nc\nd\n\ne\n");
    assert_eq!(t.k("}").s(), "a\nb\n|\nc\nd\n\ne\n");
    assert_eq!(t.k("}").s(), "a\nb\n\nc\nd\n|\ne\n");
    assert_eq!(t.k("{").s(), "a\nb\n|\nc\nd\n\ne\n");
    assert_eq!(t.k("L").s(), "a\nb\n\nc\nd\n\n|e\n");
    assert_eq!(t.k("H").s(), "|a\nb\n\nc\nd\n\ne\n");
}

#[test]
fn operators_with_motions_text_objects_and_counts() {
    let mut t = ed("one |two three four\n");
    assert_eq!(t.k("dw").s(), "one |three four\n");
    assert_eq!(t.k("u").s(), "one |two three four\n");
    assert_eq!(t.k("d2w").s(), "one |four\n");
    assert_eq!(t.k("u2dw").s(), "one |four\n");
    assert_eq!(t.k("u").k("de").s(), "one | three four\n");
    assert_eq!(t.k("u").k("D").s(), "one| \n");
    assert_eq!(t.k("u").k("d$").text(), "one \n");
    assert_eq!(t.k("u").k("d0").s(), "|two three four\n");
    // `dw` on the last word stops at the end of the line.
    let mut t = ed("a |end\nnext\n");
    assert_eq!(t.k("dw").s(), "a| \nnext\n");
    let mut t = ed("x = call(|arg1, \"q u\", [1, 2]) + 1;\n");
    assert_eq!(t.k("di(").s(), "x = call(|) + 1;\n");
    assert_eq!(t.k("u").k("da(").s(), "x = call| + 1;\n");
    assert_eq!(
        t.k("u").k("f\"ci\"Z<Esc>").text(),
        "x = call(arg1, \"Z\", [1, 2]) + 1;\n"
    );
    assert_eq!(
        t.k("u").k("f[").k("yi[").vim.register('"').unwrap().text,
        "1, 2"
    );
    assert_eq!(t.k("ya[").vim.register('"').unwrap().text, "[1, 2]");
    let mut t = ed("foo b|ar baz\n");
    assert_eq!(t.k("diw").s(), "foo | baz\n");
    // On white space, `iw` is the white space.
    let mut t = ed("foo bar| baz\n");
    assert_eq!(t.k("diw").s(), "foo bar|baz\n");
    let mut t = ed("foo b|ar baz\n");
    assert_eq!(t.k("daw").s(), "foo |baz\n");
    let mut t = ed("<div><p>|hi</p></div>\n");
    assert_eq!(t.k("dit").text(), "<div><p></p></div>\n");
    assert_eq!(t.k("u").k("dat").text(), "<div></div>\n");
    let mut t = ed("a\n|b\nc\n\nd\n");
    assert_eq!(t.k("dap").text(), "d\n");
    assert_eq!(
        t.k("u").k("yip").vim.register('"').unwrap(),
        &Reg {
            text: "a\nb\nc\n".into(),
            linewise: true,
            block: false
        }
    );
}

#[test]
fn line_operators_case_and_indent() {
    let mut t = ed("one\n|two\nthree\nfour\n");
    assert_eq!(t.k("dd").s(), "one\n|three\nfour\n");
    assert_eq!(
        t.k("2dd").s(),
        "|one\n",
        "the cursor goes to the line left at the end"
    );
    assert_eq!(t.k("u").s(), "one\n|three\nfour\n");
    let mut t = ed("a\nb\n|c");
    assert_eq!(
        t.k("dd").s(),
        "a\n|b",
        "the last line takes the line break before it"
    );
    let mut t = ed("|word Two\n");
    assert_eq!(t.k("g~w").text(), "WORD Two\n");
    assert_eq!(t.k("gUU").text(), "WORD TWO\n");
    assert_eq!(t.k("guiw").text(), "word TWO\n");
    assert_eq!(t.k("~").s(), "W|ord TWO\n");
    assert_eq!(t.k(">>").text(), "\tWord TWO\n");
    assert_eq!(t.k("<<").text(), "Word TWO\n");
    let mut t = ed("|a\nb\nc\n");
    assert_eq!(t.k("2>>").text(), "\ta\n\tb\nc\n");
    assert_eq!(t.k(">j").text(), "\t\ta\n\t\tb\nc\n");
    let mut t = ed("    |x\n");
    assert_eq!(t.k("<<").text(), "x\n");
    let mut t = ed("  |old line\nnext\n");
    assert_eq!(
        t.k("ccnew<Esc>").s(),
        "  ne|w\nnext\n",
        "cc keeps the indentation"
    );
    assert_eq!(t.k("S").k("again<Esc>").text(), "  again\nnext\n");
}

#[test]
fn change_and_insert_commands_are_one_undo_step_each() {
    let mut t = ed("one |two three\n");
    assert_eq!(t.k("cwTWO<Esc>").s(), "one TW|O three\n");
    assert_eq!(t.vim.mode, Mode::Normal);
    assert_eq!(t.k("u").text(), "one two three\n");
    assert_eq!(t.k("<C-r>").text(), "one TWO three\n");
    let mut t = ed("|ab\n");
    assert_eq!(t.k("ix<Esc>").s(), "|xab\n");
    assert_eq!(t.k("ay<Esc>").s(), "x|yab\n");
    assert_eq!(t.k("AZ<Esc>").s(), "xyab|Z\n");
    assert_eq!(t.k("I<<Esc>").s(), "|<xyabZ\n");
    assert_eq!(t.k("onext<Esc>").s(), "<xyabZ\nnex|t\n");
    assert_eq!(t.k("Oup<Esc>").s(), "<xyabZ\nu|p\nnext\n");
    assert_eq!(t.k("uuuuuu").text(), "ab\n");
    // Typing over several lines, Backspace, Enter: one step.
    let mut t = ed("|\n");
    t.k("ihello<CR>world<BS><BS>ld<Esc>");
    assert_eq!(t.text(), "hello\nworld\n");
    assert_eq!(t.k("u").text(), "\n");
    let mut t = ed("|abc\n");
    assert_eq!(t.k("Rxy<Esc>").s(), "x|yc\n");
    assert_eq!(t.k("u").text(), "abc\n");
    let mut t = ed("|abc\n");
    assert_eq!(t.k("3rz").s(), "zz|z\n");
    assert_eq!(t.k("x").s(), "z|z\n");
    let mut t = ed("|a\n  b\nc\n");
    assert_eq!(t.k("J").s(), "a| b\nc\n");
    assert_eq!(t.k("u3J").text(), "a b c\n");
}

#[test]
fn dot_repeats_the_last_change_with_its_count_or_a_new_one() {
    let mut t = ed("|a b c d e f\n");
    t.k("dw");
    assert_eq!(t.k(".").s(), "|c d e f\n");
    assert_eq!(t.k("2.").s(), "|e f\n");
    let mut t = ed("|one one one\n");
    t.k("cwtwo<Esc>w");
    assert_eq!(t.k(".").text(), "two two one\n");
    assert_eq!(t.k("w.").text(), "two two two\n");
    let mut t = ed("|x\n");
    t.k("Aab<Esc>");
    assert_eq!(t.k(".").text(), "xabab\n");
    let mut t = ed("|a\nb\nc\n");
    t.k("dd");
    assert_eq!(t.k(".").text(), "c\n");
}

#[test]
fn registers_named_yank_append_and_the_clipboard() {
    let mut t = ed("|alpha beta\n");
    t.k("\"ayw").k("w\"byw");
    assert_eq!(t.vim.register('a').unwrap().text, "alpha ");
    assert_eq!(t.vim.register('b').unwrap().text, "beta");
    t.k("\"Ayw");
    assert_eq!(t.vim.register('a').unwrap().text, "alpha beta");
    t.k("0\"ap");
    assert_eq!(t.text(), "aalpha betalpha beta\n");
    // A delete goes to the unnamed register, the yank stays in "0.
    let mut t = ed("|keep\ndrop\n");
    t.k("yyjdd");
    assert_eq!(t.vim.register('"').unwrap().text, "drop\n");
    assert_eq!(t.vim.register('0').unwrap().text, "keep\n");
    t.k("\"0p");
    assert_eq!(t.text(), "keep\nkeep\n");
    // "_ discards; "+ is the clipboard.
    let mut t = ed("|one two\n");
    t.k("\"_dw");
    assert!(t.vim.register('"').is_none());
    t.k("\"+yw");
    assert_eq!(t.clip.clipboard().as_deref(), Some("two"));
    *t.clip.0.borrow_mut() = Some("line\n".into());
    t.k("\"+p");
    assert_eq!(
        t.text(),
        "two\nline\n",
        "a clipboard text ending in a line break pastes as lines"
    );
}

#[test]
fn paste_charwise_and_linewise_before_and_after() {
    let mut t = ed("|ab\ncd\n");
    t.k("ylp");
    assert_eq!(t.s(), "a|ab\ncd\n");
    t.k("yyp");
    assert_eq!(t.s(), "aab\n|aab\ncd\n");
    t.k("P");
    assert_eq!(t.s(), "aab\n|aab\naab\ncd\n");
    t.k("G3p");
    assert_eq!(t.text(), "aab\naab\naab\ncd\naab\naab\naab\n");
    let mut t = ed("x\n|last");
    t.k("yyp");
    assert_eq!(
        t.s(),
        "x\nlast\n|last",
        "after a last line without a line break"
    );
}

#[test]
fn visual_modes_select_and_operate() {
    let mut t = ed("|one two three\nfour\n");
    t.k("vee");
    assert_eq!(t.vim.mode, Mode::Visual);
    assert_eq!(t.sel(), "one two");
    t.k("d");
    assert_eq!(
        (t.s(), t.vim.mode),
        ("| three\nfour\n".into(), Mode::Normal)
    );
    let mut t = ed("a|bc\ndef\nghi\n");
    t.k("Vj");
    assert_eq!(t.sel(), "abc\ndef");
    t.k("y");
    assert_eq!(
        t.vim.register('"').unwrap(),
        &Reg {
            text: "abc\ndef\n".into(),
            linewise: true,
            block: false
        }
    );
    t.k("Vjd");
    assert_eq!(t.text(), "ghi\n");
    let mut t = ed("|abc\n");
    t.k("vlo");
    assert_eq!(t.s(), "|abc\n");
    t.k("U");
    assert_eq!(t.text(), "ABc\n");
    let mut t = ed("x(|a b)y\n");
    t.k("vi(");
    assert_eq!(t.sel(), "a b");
    t.k("cZ<Esc>");
    assert_eq!(t.text(), "x(Z)y\n");
    let mut t = ed("|one\ntwo\n");
    t.k("yiwjviwp");
    assert_eq!(t.text(), "one\none\n");
    assert_eq!(
        t.vim.register('"').unwrap().text,
        "two",
        "the replaced text is in the register"
    );
    let mut t = ed("|a\nb\nc\n");
    t.k("VjJ");
    assert_eq!(t.text(), "a b\nc\n");
    t.k("v<Esc>");
    assert_eq!(t.vim.mode, Mode::Normal);
}

#[test]
fn visual_block_selects_columns_and_inserts_on_every_line() {
    let mut t = ed("|abcd\nefgh\nijkl\n");
    t.k("l<C-v>jjl");
    assert_eq!(t.vim.mode, Mode::VisualBlock);
    assert_eq!(t.doc.selection().len(), 3);
    assert_eq!(t.sel(), "bc\nfg\njk");
    t.k("d");
    assert_eq!(t.text(), "ad\neh\nil\n");
    assert!(t.vim.register('"').unwrap().block);
    t.k("u");
    t.k("0<C-v>jjI# <Esc>");
    assert_eq!(t.text(), "# abcd\n# efgh\n# ijkl\n");
    assert_eq!(
        t.k("u").text(),
        "abcd\nefgh\nijkl\n",
        "the block insert is one undo step"
    );
    t.k("$<C-v>jA;<Esc>");
    assert_eq!(t.text(), "abcd;\nefgh;\nijkl\n");
}

#[test]
fn search_star_and_the_command_line() {
    let mut t = ed("|foo bar\nbaz foo\nFoo\n");
    t.k("/foo<CR>");
    assert_eq!(t.s(), "foo bar\nbaz |foo\nFoo\n");
    assert!(t.vim.highlight);
    t.k("n");
    assert_eq!(t.s(), "|foo bar\nbaz foo\nFoo\n");
    assert_eq!(t.k("N").s(), "foo bar\nbaz |foo\nFoo\n");
    t.k(":set ic<CR>").k("n");
    assert_eq!(t.s(), "foo bar\nbaz foo\n|Foo\n");
    t.k(":noh<CR>");
    assert!(!t.vim.highlight);
    let mut t = ed("|word other word\n");
    t.k("*");
    assert_eq!(t.s(), "word other |word\n");
    t.k("#");
    assert_eq!(t.s(), "|word other word\n");
    t.k("?oth<CR>");
    assert_eq!(t.s(), "word |other word\n");
    t.k("/nothing<CR>");
    assert!(t.last.error);
    assert_eq!(
        t.last.message.as_deref(),
        Some("E486: Pattern not found: nothing")
    );
}

#[test]
fn substitute_with_ranges_flags_and_vim_patterns() {
    let mut t = ed("|a a\nb a\nc\n");
    t.k(":s/a/X/<CR>");
    assert_eq!(t.text(), "X a\nb a\nc\n");
    t.k("u:%s/a/Y/g<CR>");
    assert_eq!(t.text(), "Y Y\nb Y\nc\n");
    assert_eq!(
        t.last.message.as_deref(),
        Some("3 substitutions on 2 lines")
    );
    assert_eq!(t.k("u").text(), "a a\nb a\nc\n", "one undo step");
    t.k(":2,3s/\\(b\\|c\\)/[&]/<CR>");
    assert_eq!(t.text(), "a a\n[b] a\n[c]\n");
    t.k(":%s/\\<a\\>/A/gn<CR>");
    assert_eq!(t.last.message.as_deref(), Some("3 matches on 2 lines"));
    assert_eq!(t.text(), "a a\n[b] a\n[c]\n", "n only counts");
    t.k(":%s#\\[\\(.\\)\\]#\\1#g<CR>");
    assert_eq!(t.text(), "a a\nb a\nc\n");
    t.k("ggVj:s/a/Q/g<CR>");
    assert_eq!(t.text(), "Q Q\nb Q\nc\n");
    t.k(":%s/zzz/x/<CR>");
    assert!(t.last.error);
    t.k(":%s/a/b/c<CR>");
    assert!(t.last.error, "confirm is not supported");
}

#[test]
fn ex_commands_return_what_the_window_must_do() {
    let mut t = ed("|a\nb\nc\n");
    t.k(":w<CR>");
    assert_eq!(
        t.last.ex,
        vec![Ex::Write {
            path: None,
            force: false
        }]
    );
    t.k(":wq<CR>");
    assert_eq!(t.last.ex, vec![Ex::WriteQuit { force: false }]);
    t.k(":q!<CR>");
    assert_eq!(t.last.ex, vec![Ex::Quit { force: true }]);
    t.k(":x<CR>");
    assert_eq!(t.last.ex, vec![Ex::WriteQuit { force: false }]);
    t.k(":e other.md<CR>");
    assert_eq!(
        t.last.ex,
        vec![Ex::Edit {
            path: Some("other.md".into()),
            force: false
        }]
    );
    t.k(":w copy.txt<CR>");
    assert_eq!(
        t.last.ex,
        vec![Ex::Write {
            path: Some("copy.txt".into()),
            force: false
        }]
    );
    t.k(":set nowrap ts=2<CR>");
    assert_eq!(t.last.ex, vec![Ex::Wrap(false)]);
    assert_eq!(t.doc.tab_width, 2);
    t.k(":3<CR>");
    assert_eq!(t.s(), "a\nb\n|c\n");
    t.k(":bogus<CR>");
    assert!(t.last.error);
    t.k(":set nosuch<CR>");
    assert!(t.last.error);
    // Esc leaves the command line; Backspace past the colon too.
    t.k(":wq<Esc>");
    assert_eq!((t.vim.prompt.clone(), t.last.ex.clone()), (None, vec![]));
    t.k(":<BS>");
    assert!(t.vim.prompt.is_none());
}

#[test]
fn counts_registers_and_pending_keys() {
    let mut t = ed("|a b c d e\n");
    t.k("\"a2");
    assert_eq!(t.vim.pending(), "\"a2");
    t.k("d");
    assert_eq!(t.vim.pending(), "\"a2d");
    t.k("w");
    assert_eq!(t.vim.pending(), "");
    assert_eq!(t.vim.register('a').unwrap().text, "a b ");
    // An invalid sequence is dropped.
    t.k("zq");
    assert_eq!(t.vim.pending(), "");
    t.k("d<Esc>");
    assert_eq!(t.text(), "c d e\n");
}

#[test]
fn korean_text_and_wide_characters_in_motions_and_visual_blocks() {
    let mut t = ed("|한글 문장 입니다\n가나다라\n");
    assert_eq!(t.k("w").s(), "한글 |문장 입니다\n가나다라\n");
    assert_eq!(t.k("dw").s(), "한글 |입니다\n가나다라\n");
    assert_eq!(t.k("x").s(), "한글 |니다\n가나다라\n");
    let mut t = ed("|가나다\nabcdef\n");
    t.k("l<C-v>jl");
    // From 나 (columns 2-3) to d (column 3): columns 2-3 on both lines.
    assert_eq!(t.sel(), "나\ncd");
}

#[test]
fn insert_mode_text_and_arrow_keys_start_a_new_change() {
    let mut t = ed("|\n");
    t.k("i");
    let host = Clip::default();
    t.vim.text(&mut t.doc, "자락", &host, View::default());
    t.k("<Esc>");
    assert_eq!(t.text(), "자락\n");
    let mut t = ed("|\n");
    t.k("ihello<Left>X<Esc>");
    assert_eq!(t.text(), "hellXo\n");
    t.k("$.");
    assert_eq!(
        t.text(),
        "hellXXo\n",
        "the repeat inserts what was typed after the move, before the cursor"
    );
}

#[test]
fn standard_document_editing_is_unchanged_outside_a_group() {
    let mut d = Document::from_text("ab");
    d.set_selection(&Selection::point(2));
    d.type_text("c");
    d.type_text("d");
    assert_eq!(
        d.history().undo_len(),
        1,
        "typing still merges into one step"
    );
    d.begin_group();
    d.type_text(" ");
    d.paste("x");
    d.newline();
    d.end_group();
    assert_eq!(d.history().undo_len(), 2, "a group is one step");
    d.undo();
    assert_eq!(d.text().to_string(), "abcd");
}

#[test]
fn vim_patterns_translate_to_regular_expressions() {
    assert_eq!(vim_regex("a\\(b\\|c\\)\\+"), "a(b|c)+");
    assert_eq!(vim_regex("f(x)"), "f\\(x\\)");
    assert_eq!(vim_regex("\\<word\\>"), "\\bword\\b");
    assert_eq!(vim_regex("\\v(a|b)+"), "(a|b)+");
    assert_eq!(vim_regex("a.b*"), "a.b*");
    assert_eq!(vim_replacement("[&] \\1 $x", "\n"), "[${0}] ${1} $$x");
}
