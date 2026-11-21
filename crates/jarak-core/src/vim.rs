//! Vim-style modal editing (`JARAK-FR-050`…`JARAK-FR-053`, Step 92).
//!
//! A key-to-command layer over the document's own editing operations: the engine reads keys,
//! keeps the Vim state (mode, pending keys, registers, the last change, the last search) and
//! edits the [`Document`] through the same primitives Standard mode uses, so undo, the save point
//! and every other part of Jarak see ordinary edits. One Vim change (`dw`, `cw` and the typed
//! text, `3p`) is one undo step.
//!
//! - Modes: Normal, Insert, Replace, Visual, Visual-line, Visual-block (a block is a selection of
//!   one range per line, the column selection of Step 96).
//! - Normal: motions `h j k l w b e W B E ge gE 0 ^ $ gg G f F t T ; , % { } H M L n N * #`,
//!   Ctrl+D/U/F/B; operators `d c y > < g~ gu gU` with a motion, a text object (`iw aw iW aW i"
//!   a" i' a' i` a` i( a( ib ab i[ a[ i{ a{ iB aB i< a< it at ip ap`) or doubled for lines; counts
//!   before the operator and the motion; `x X D C s S Y r R J ~ p P o O i a I A u Ctrl+R .`;
//!   registers: the unnamed one, `a`–`z` (`A`–`Z` append), `0` (the last yank), `_` (discard) and
//!   `+`/`*` (the Windows clipboard).
//! - The command line: `:w`, `:w <file>`, `:q`, `:q!`, `:wq`, `:x`, `:e`, `:e!`, `:e <file>`,
//!   `:s/…/…/[gin]` with ranges (`%`, `'<,'>`, `N,M`, `.`, `$`), `:noh`, `:set`, `:<line>`;
//!   `/` and `?` search. Patterns use Vim's "magic" syntax (`\(…\)`, `\|`, `\+`, `\<…\>`, `\v`
//!   for "very magic"), translated to Jarak's regular expressions; back references inside a
//!   pattern are not supported.
//! - What the window must do (write, quit, open a file, wrap lines) comes back as [`Ex`].

use crate::document::Document;
use crate::selection::{Range, Selection};
use crate::text;
use ropey::Rope;
use std::collections::HashMap;

/// The editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Replace,
    Visual,
    VisualLine,
    VisualBlock,
}

impl Mode {
    /// The status bar's word for it.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Replace => "REPLACE",
            Mode::Visual => "VISUAL",
            Mode::VisualLine => "V-LINE",
            Mode::VisualBlock => "V-BLOCK",
        }
    }

    pub fn is_visual(self) -> bool {
        matches!(self, Mode::Visual | Mode::VisualLine | Mode::VisualBlock)
    }

    /// Text is typed in this mode (the window lets the IME compose).
    pub fn is_insert(self) -> bool {
        matches!(self, Mode::Insert | Mode::Replace)
    }
}

/// A key the window passes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Esc,
    Enter,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    /// Ctrl with a letter (`Ctrl('r')`).
    Ctrl(char),
}

/// The clipboard, for the `+` and `*` registers.
pub trait Host {
    fn clipboard(&self) -> Option<String>;
    fn set_clipboard(&self, s: &str);
}

/// A host without a clipboard (tests).
#[derive(Debug, Default)]
pub struct NoHost;

impl Host for NoHost {
    fn clipboard(&self) -> Option<String> {
        None
    }
    fn set_clipboard(&self, _s: &str) {}
}

/// The lines on screen (for `H`, `M`, `L` and the page moves).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub top: usize,
    pub rows: usize,
}

impl Default for View {
    fn default() -> Self {
        View { top: 0, rows: 30 }
    }
}

/// What the window does for a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ex {
    Write { path: Option<String>, force: bool },
    Quit { force: bool },
    WriteQuit { force: bool },
    Edit { path: Option<String>, force: bool },
    Wrap(bool),
}

/// The result of a key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The key was Vim's (the window does nothing else with it).
    pub handled: bool,
    pub ex: Vec<Ex>,
    pub message: Option<String>,
    /// The message is an error.
    pub error: bool,
}

impl Outcome {
    fn handled() -> Outcome {
        Outcome {
            handled: true,
            ..Outcome::default()
        }
    }
    fn msg(m: impl Into<String>) -> Outcome {
        Outcome {
            handled: true,
            message: Some(m.into()),
            ..Outcome::default()
        }
    }
    fn err(m: impl Into<String>) -> Outcome {
        Outcome {
            handled: true,
            message: Some(m.into()),
            error: true,
            ..Outcome::default()
        }
    }
}

/// A register's content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reg {
    pub text: String,
    pub linewise: bool,
    pub block: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ins {
    Text(String),
    Back,
    Newline,
    DelWord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    keys: Vec<char>,
    insert: Vec<Ins>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Toggle,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mot {
    Left,
    Right,
    Up,
    Down,
    Word(bool),
    WordEnd(bool),
    WordBack(bool),
    WordEndBack(bool),
    Zero,
    FirstNonBlank,
    LineEnd,
    DocTop,
    DocBottom,
    Find(char, char),
    RepeatFind(bool),
    Match,
    ParaFwd,
    ParaBack,
    ScreenTop,
    ScreenMid,
    ScreenBottom,
    SearchNext(bool),
    Star(bool),
    HalfDown,
    HalfUp,
    PageDown,
    PageUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Obj {
    Word(bool),
    Pair(char, char),
    Quote(char),
    Tag,
    Para,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Mot(Mot),
    Obj(bool, Obj),
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Move(Mot),
    Op(Op, Target),
    Insert(char),
    Paste(bool),
    Replace(char),
    ReplaceMode,
    Join,
    ToggleCase,
    Undo,
    Redo,
    Repeat,
    Visual(Mode),
    Prompt(char),
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cmd {
    reg: Option<char>,
    count: Option<usize>,
    act: Act,
}

enum Parsed {
    More,
    Bad,
    Cmd(Cmd),
}

/// How a motion's range is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Exclusive,
    Inclusive,
    Linewise,
}

const ESC: char = '\u{1b}';
const CTRL_R: char = '\u{12}';
const CTRL_V: char = '\u{16}';
const CTRL_D: char = '\u{04}';
const CTRL_U: char = '\u{15}';
const CTRL_F: char = '\u{06}';
const CTRL_B: char = '\u{02}';

/// The Vim state of one window.
#[derive(Debug, Clone)]
pub struct Vim {
    pub mode: Mode,
    /// The command line being typed (`:…`, `/…`, `?…`).
    pub prompt: Option<String>,
    pending: Vec<char>,
    regs: HashMap<char, Reg>,
    anchor: usize,
    head: usize,
    last_find: Option<(char, char)>,
    /// The last search pattern (Vim syntax) and its direction.
    pub search: Option<(String, bool)>,
    /// Matches of the last search are highlighted (`:noh` turns it off until the next search).
    pub highlight: bool,
    pub ignorecase: bool,
    pub smartcase: bool,
    last_change: Option<Change>,
    recording: Option<Change>,
    /// The lines of the last visual selection (for `'<,'>`).
    last_visual: Option<(usize, usize)>,
    /// A visual-block insert: the carets to type at (`I`, `A`, `c` in a block).
    block_carets: Option<Vec<usize>>,
}

impl Default for Vim {
    fn default() -> Self {
        Vim::new()
    }
}

// ---- text helpers ---------------------------------------------------------------------------

fn rope(doc: &Document) -> &Rope {
    doc.text()
}

fn last_line(r: &Rope) -> usize {
    // A trailing line break opens an empty last line, which Vim does not count.
    let n = r.len_lines();
    if n > 1 && line_content_len(r, n - 1) == 0 {
        n - 2
    } else {
        n - 1
    }
}

fn line_start(r: &Rope, l: usize) -> usize {
    r.line_to_char(l.min(r.len_lines() - 1))
}

fn line_content_len(r: &Rope, l: usize) -> usize {
    text::line_len(r.line(l.min(r.len_lines() - 1)))
}

fn line_end(r: &Rope, l: usize) -> usize {
    line_start(r, l) + line_content_len(r, l)
}

/// The start of the next line (or the end of the text).
fn next_line_start(r: &Rope, l: usize) -> usize {
    if l + 1 < r.len_lines() {
        r.line_to_char(l + 1)
    } else {
        r.len_chars()
    }
}

fn line_of(r: &Rope, pos: usize) -> usize {
    r.char_to_line(pos.min(r.len_chars()))
}

fn first_nonblank(r: &Rope, l: usize) -> usize {
    let s = line_start(r, l);
    let e = line_end(r, l);
    let mut i = s;
    while i < e && matches!(r.char(i), ' ' | '\t') {
        i += 1;
    }
    if i == e && e > s { e - 1 } else { i }
}

/// The normal-mode cursor: on a character, never on the line break (except an empty line).
fn clamp(r: &Rope, pos: usize) -> usize {
    let l = line_of(r, pos);
    let s = line_start(r, l);
    let e = line_end(r, l);
    if e == s {
        return s;
    }
    let p = pos.clamp(s, e);
    if p >= e {
        text::prev_grapheme(r.slice(..), e)
    } else {
        text::floor_grapheme(r.slice(..), p)
    }
}

fn next_g(r: &Rope, pos: usize) -> usize {
    text::next_grapheme(r.slice(..), pos)
}

fn prev_g(r: &Rope, pos: usize) -> usize {
    text::prev_grapheme(r.slice(..), pos)
}

fn is_eol(c: char) -> bool {
    c == '\n' || c == '\r'
}

fn is_blank(c: char) -> bool {
    c.is_whitespace()
}

/// 0 blank, 1 punctuation (or any non-blank for WORDs), 2 keyword.
fn class(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big {
        1
    } else if c.is_alphanumeric() || c == '_' {
        2
    } else {
        1
    }
}

fn is_empty_line_at(r: &Rope, pos: usize) -> bool {
    let l = line_of(r, pos);
    line_start(r, l) == pos && line_content_len(r, l) == 0
}

fn word_fwd(r: &Rope, i: usize, big: bool) -> usize {
    let len = r.len_chars();
    if i >= len {
        return len;
    }
    let c0 = class(r.char(i), big);
    let mut j = i;
    if c0 != 0 {
        while j < len && class(r.char(j), big) == c0 {
            j += 1;
        }
    }
    while j < len {
        let c = r.char(j);
        if !is_blank(c) {
            break;
        }
        if c == '\n' && j + 1 < len && is_eol(r.char(j + 1)) {
            // An empty line is a word of its own.
            return j + 1;
        }
        if c == '\n' && j + 1 == len {
            return j + 1;
        }
        j += 1;
    }
    j
}

fn word_end_fwd(r: &Rope, i: usize, big: bool) -> usize {
    let len = r.len_chars();
    if len == 0 {
        return 0;
    }
    let mut j = i + 1;
    while j < len && is_blank(r.char(j)) {
        j += 1;
    }
    if j >= len {
        return len - 1;
    }
    let c0 = class(r.char(j), big);
    while j + 1 < len && class(r.char(j + 1), big) == c0 {
        j += 1;
    }
    j
}

fn word_back(r: &Rope, i: usize, big: bool) -> usize {
    if i == 0 {
        return 0;
    }
    let mut j = i - 1;
    while j > 0 && is_blank(r.char(j)) {
        if is_empty_line_at(r, j) {
            return j;
        }
        j -= 1;
    }
    let c0 = class(r.char(j), big);
    while j > 0 && c0 != 0 && class(r.char(j - 1), big) == c0 {
        j -= 1;
    }
    j
}

fn word_end_back(r: &Rope, i: usize, big: bool) -> usize {
    let mut j = i;
    let c0 = class(r.char(i.min(r.len_chars().saturating_sub(1))), big);
    while j > 0 && c0 != 0 && class(r.char(j - 1), big) == c0 {
        j -= 1;
    }
    if j == 0 {
        return 0;
    }
    j -= 1;
    while j > 0 && is_blank(r.char(j)) {
        j -= 1;
    }
    j
}

fn is_blank_line(r: &Rope, l: usize) -> bool {
    r.line(l).chars().all(char::is_whitespace)
}

fn pair_of(c: char) -> Option<(char, char, bool)> {
    match c {
        '(' => Some(('(', ')', true)),
        ')' => Some(('(', ')', false)),
        '[' => Some(('[', ']', true)),
        ']' => Some(('[', ']', false)),
        '{' => Some(('{', '}', true)),
        '}' => Some(('{', '}', false)),
        _ => None,
    }
}

/// The match of the bracket at `i`.
fn match_at(r: &Rope, i: usize) -> Option<usize> {
    let (open, close, fwd) = pair_of(r.char(i))?;
    let mut depth = 0i64;
    if fwd {
        for j in i..r.len_chars() {
            let c = r.char(j);
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
        }
    } else {
        let mut j = i + 1;
        while j > 0 {
            j -= 1;
            let c = r.char(j);
            if c == close {
                depth += 1;
            } else if c == open {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
        }
    }
    None
}

/// The unmatched `open` before `i` (and its `close` after), for `i(` and the like.
fn enclosing(r: &Rope, i: usize, open: char, close: char) -> Option<(usize, usize)> {
    let len = r.len_chars();
    // On a bracket itself: that pair.
    let start = if i < len && r.char(i) == open {
        Some(i)
    } else {
        let mut depth = 0i64;
        let mut j = i.min(len);
        let mut found = None;
        if i < len && r.char(i) == close {
            depth = 1;
        }
        while j > 0 {
            j -= 1;
            let c = r.char(j);
            if c == close {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    found = Some(j);
                    break;
                }
                depth -= 1;
            }
        }
        found
    }?;
    let end = match_at(r, start)?;
    Some((start, end))
}

fn upper(s: &str) -> String {
    s.chars().flat_map(char::to_uppercase).collect()
}

fn lower(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

fn toggle(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_uppercase() {
                c.to_lowercase().collect::<Vec<_>>()
            } else {
                c.to_uppercase().collect::<Vec<_>>()
            }
        })
        .collect()
}

/// A Vim "magic" pattern as a regular expression.
pub fn vim_regex(p: &str) -> String {
    if let Some(v) = p.strip_prefix("\\v") {
        return v.replace(['<', '>'], "\\b");
    }
    let mut out = String::with_capacity(p.len() + 8);
    let mut it = p.chars().peekable();
    // Inside `\{n,m}` the closing brace needs no backslash.
    let mut brace = false;
    while let Some(c) = it.next() {
        if brace && c == '}' {
            out.push('}');
            brace = false;
        } else if c == '\\' {
            match it.next() {
                Some('(') => out.push('('),
                Some(')') => out.push(')'),
                Some('|') => out.push('|'),
                Some('+') => out.push('+'),
                Some('?' | '=') => out.push('?'),
                Some('{') => {
                    out.push('{');
                    brace = true;
                }
                Some('}') => {
                    out.push('}');
                    brace = false;
                }
                Some('<' | '>') => out.push_str("\\b"),
                Some('n') => out.push_str("\\n"),
                Some('t') => out.push_str("\\t"),
                Some(n @ ('d' | 'D' | 'w' | 'W' | 's' | 'S')) => {
                    out.push('\\');
                    out.push(n);
                }
                Some(n) => {
                    out.push_str(&regex::escape(&n.to_string()));
                }
                None => out.push_str("\\\\"),
            }
        } else if "()|+?{}".contains(c) {
            out.push('\\');
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// A Vim replacement (`\1`, `&`, `\r` a line break) in the regex crate's syntax.
fn vim_replacement(s: &str, eol: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        match c {
            '\\' => match it.next() {
                Some(d @ '0'..='9') => out.push_str(&format!("${{{d}}}")),
                Some('r' | 'n') => out.push_str(eol),
                Some('t') => out.push('\t'),
                Some('&') => out.push('&'),
                Some(o) => {
                    if o == '$' {
                        out.push_str("$$");
                    } else {
                        out.push(o);
                    }
                }
                None => out.push('\\'),
            },
            '&' => out.push_str("${0}"),
            '$' => out.push_str("$$"),
            o => out.push(o),
        }
    }
    out
}

fn digits(k: &[char], mut i: usize) -> (Option<usize>, usize) {
    let start = i;
    if k.get(i).is_some_and(|c| ('1'..='9').contains(c)) {
        while k.get(i).is_some_and(char::is_ascii_digit) {
            i += 1;
        }
        let n: String = k[start..i].iter().collect();
        return (n.parse().ok(), i);
    }
    (None, i)
}

fn parse_motion(k: &[char]) -> Result<(Mot, usize), bool> {
    // Err(true): more keys needed; Err(false): not a motion.
    let Some(&c) = k.first() else {
        return Err(true);
    };
    let one = |m| Ok((m, 1));
    match c {
        'h' | '\u{8}' => one(Mot::Left),
        'l' | ' ' => one(Mot::Right),
        'j' => one(Mot::Down),
        'k' => one(Mot::Up),
        'w' => one(Mot::Word(false)),
        'W' => one(Mot::Word(true)),
        'e' => one(Mot::WordEnd(false)),
        'E' => one(Mot::WordEnd(true)),
        'b' => one(Mot::WordBack(false)),
        'B' => one(Mot::WordBack(true)),
        '0' => one(Mot::Zero),
        '^' => one(Mot::FirstNonBlank),
        '$' => one(Mot::LineEnd),
        'G' => one(Mot::DocBottom),
        'g' => match k.get(1) {
            None => Err(true),
            Some('g') => Ok((Mot::DocTop, 2)),
            Some('e') => Ok((Mot::WordEndBack(false), 2)),
            Some('E') => Ok((Mot::WordEndBack(true), 2)),
            _ => Err(false),
        },
        'f' | 'F' | 't' | 'T' => match k.get(1) {
            None => Err(true),
            Some(&ch) if ch != ESC => Ok((Mot::Find(c, ch), 2)),
            _ => Err(false),
        },
        ';' => one(Mot::RepeatFind(false)),
        ',' => one(Mot::RepeatFind(true)),
        '%' => one(Mot::Match),
        '}' => one(Mot::ParaFwd),
        '{' => one(Mot::ParaBack),
        'H' => one(Mot::ScreenTop),
        'M' => one(Mot::ScreenMid),
        'L' => one(Mot::ScreenBottom),
        'n' => one(Mot::SearchNext(false)),
        'N' => one(Mot::SearchNext(true)),
        '*' => one(Mot::Star(false)),
        '#' => one(Mot::Star(true)),
        CTRL_D => one(Mot::HalfDown),
        CTRL_U => one(Mot::HalfUp),
        CTRL_F => one(Mot::PageDown),
        CTRL_B => one(Mot::PageUp),
        _ => Err(false),
    }
}

fn parse_obj(c: char) -> Option<Obj> {
    Some(match c {
        'w' => Obj::Word(false),
        'W' => Obj::Word(true),
        '"' | '\'' | '`' => Obj::Quote(c),
        '(' | ')' | 'b' => Obj::Pair('(', ')'),
        '[' | ']' => Obj::Pair('[', ']'),
        '{' | '}' | 'B' => Obj::Pair('{', '}'),
        '<' | '>' => Obj::Pair('<', '>'),
        't' => Obj::Tag,
        'p' => Obj::Para,
        _ => return None,
    })
}

/// Parses normal-mode keys (`"a3d2w`, `ciw`, `gg`, `p`, …).
fn parse_normal(k: &[char]) -> Parsed {
    let mut i = 0;
    let mut reg = None;
    if k.first() == Some(&'"') {
        match k.get(1) {
            None => return Parsed::More,
            Some(&r) if r.is_ascii_alphanumeric() || "+*_\"".contains(r) => reg = Some(r),
            _ => return Parsed::Bad,
        }
        i = 2;
    }
    let (count, j) = digits(k, i);
    i = j;
    let Some(&c) = k.get(i) else {
        return Parsed::More;
    };
    let cmd = |act| Parsed::Cmd(Cmd { reg, count, act });
    let op = match c {
        'd' => Some((Op::Delete, 'd', 1)),
        'c' => Some((Op::Change, 'c', 1)),
        'y' => Some((Op::Yank, 'y', 1)),
        '>' => Some((Op::Indent, '>', 1)),
        '<' => Some((Op::Outdent, '<', 1)),
        'g' => match k.get(i + 1) {
            None => return Parsed::More,
            Some('~') => Some((Op::Toggle, '~', 2)),
            Some('u') => Some((Op::Lower, 'u', 2)),
            Some('U') => Some((Op::Upper, 'U', 2)),
            _ => None,
        },
        _ => None,
    };
    if let Some((op, line_key, used)) = op {
        let mut i = i + used;
        let (count2, j) = digits(k, i);
        i = j;
        let count = match (count, count2) {
            (Some(a), Some(b)) => Some(a * b),
            (a, b) => a.or(b),
        };
        let Some(&m) = k.get(i) else {
            return Parsed::More;
        };
        let op_cmd = |t| {
            Parsed::Cmd(Cmd {
                reg,
                count,
                act: Act::Op(op, t),
            })
        };
        if m == ESC {
            return Parsed::Cmd(Cmd {
                reg,
                count: None,
                act: Act::Nothing,
            });
        }
        if m == line_key {
            return op_cmd(Target::Line);
        }
        if used == 2 && m == 'g' {
            // g~g~, gugu, gUgU
            return match k.get(i + 1) {
                None => Parsed::More,
                Some(&n) if n == line_key => op_cmd(Target::Line),
                _ => match parse_motion(&k[i..]) {
                    Ok((mot, _)) => op_cmd(Target::Mot(mot)),
                    Err(true) => Parsed::More,
                    Err(false) => Parsed::Bad,
                },
            };
        }
        if m == 'i' || m == 'a' {
            return match k.get(i + 1) {
                None => Parsed::More,
                Some(&o) => match parse_obj(o) {
                    Some(obj) => op_cmd(Target::Obj(m == 'i', obj)),
                    None => Parsed::Bad,
                },
            };
        }
        return match parse_motion(&k[i..]) {
            Ok((mot, _)) => op_cmd(Target::Mot(mot)),
            Err(true) => Parsed::More,
            Err(false) => Parsed::Bad,
        };
    }
    match c {
        'i' | 'a' | 'I' | 'A' | 'o' | 'O' => cmd(Act::Insert(c)),
        'p' => cmd(Act::Paste(false)),
        'P' => cmd(Act::Paste(true)),
        'x' => cmd(Act::Op(Op::Delete, Target::Mot(Mot::Right))),
        'X' => cmd(Act::Op(Op::Delete, Target::Mot(Mot::Left))),
        'D' => cmd(Act::Op(Op::Delete, Target::Mot(Mot::LineEnd))),
        'C' => cmd(Act::Op(Op::Change, Target::Mot(Mot::LineEnd))),
        's' => cmd(Act::Op(Op::Change, Target::Mot(Mot::Right))),
        'S' => cmd(Act::Op(Op::Change, Target::Line)),
        'Y' => cmd(Act::Op(Op::Yank, Target::Line)),
        'r' => match k.get(i + 1) {
            None => Parsed::More,
            Some(&ESC) => cmd(Act::Nothing),
            Some(&ch) => cmd(Act::Replace(ch)),
        },
        'R' => cmd(Act::ReplaceMode),
        'J' => cmd(Act::Join),
        '~' => cmd(Act::ToggleCase),
        'u' => cmd(Act::Undo),
        CTRL_R => cmd(Act::Redo),
        '.' => cmd(Act::Repeat),
        'v' => cmd(Act::Visual(Mode::Visual)),
        'V' => cmd(Act::Visual(Mode::VisualLine)),
        CTRL_V => cmd(Act::Visual(Mode::VisualBlock)),
        ':' | '/' | '?' => cmd(Act::Prompt(c)),
        ESC => cmd(Act::Nothing),
        _ => match parse_motion(&k[i..]) {
            Ok((mot, _)) => cmd(Act::Move(mot)),
            Err(true) => Parsed::More,
            Err(false) => Parsed::Bad,
        },
    }
}

impl Vim {
    pub fn new() -> Vim {
        Vim {
            mode: Mode::Normal,
            prompt: None,
            pending: Vec::new(),
            regs: HashMap::new(),
            anchor: 0,
            head: 0,
            last_find: None,
            search: None,
            highlight: false,
            ignorecase: false,
            smartcase: false,
            last_change: None,
            recording: None,
            last_visual: None,
            block_carets: None,
        }
    }

    /// The keys typed so far of an unfinished command (the status bar shows them).
    pub fn pending(&self) -> String {
        self.pending
            .iter()
            .map(|c| match *c {
                CTRL_R => "^R".to_string(),
                CTRL_V => "^V".to_string(),
                c => c.to_string(),
            })
            .collect()
    }

    /// A register's content (`"` the unnamed one).
    pub fn register(&self, r: char) -> Option<&Reg> {
        self.regs.get(&r)
    }

    /// The regular expression of the highlighted search (for the window to mark matches).
    pub fn highlight_regex(&self) -> Option<regex::Regex> {
        if !self.highlight {
            return None;
        }
        self.search.as_ref().and_then(|(p, _)| self.regex(p).ok())
    }

    fn regex(&self, pat: &str) -> Result<regex::Regex, String> {
        // `\c` anywhere ignores case for the pattern (Vim's rule); otherwise the options decide.
        let forced = pat.contains("\\c");
        let body = pat.replace("\\c", "");
        let ci = forced
            || (self.ignorecase && !(self.smartcase && body.chars().any(char::is_uppercase)));
        let mut src = vim_regex(&body);
        if ci {
            src = format!("(?i){src}");
        }
        regex::RegexBuilder::new(&src)
            .multi_line(true)
            .build()
            .map_err(|e| format!("E486: bad pattern {pat}: {e}"))
    }

    /// Leaves any mode for Normal (the editing mode was switched, a document changed).
    pub fn reset(&mut self, doc: &mut Document) {
        if self.mode.is_insert() {
            doc.end_group();
        }
        self.mode = Mode::Normal;
        self.prompt = None;
        self.pending.clear();
        self.block_carets = None;
        let r = rope(doc);
        let p = clamp(r, doc.selection().primary().head);
        doc.set_caret(&Selection::point(p));
    }

    fn cursor(&self, doc: &Document) -> usize {
        if self.mode.is_visual() {
            self.head
        } else {
            doc.selection().primary().head
        }
    }

    fn put(&mut self, doc: &mut Document, pos: usize) {
        let p = if self.mode.is_insert() {
            pos.min(doc.len_chars())
        } else {
            clamp(rope(doc), pos)
        };
        if self.mode.is_visual() {
            self.head = p;
            self.show(doc);
        } else {
            doc.set_caret(&Selection::point(p));
        }
    }

    /// Text typed by the window (a commit of the IME, a pasted string, keys as text).
    pub fn text(&mut self, doc: &mut Document, s: &str, host: &dyn Host, view: View) -> Outcome {
        if self.prompt.is_none() && self.mode.is_insert() {
            self.insert_text(doc, s);
            return Outcome::handled();
        }
        let mut out = Outcome::handled();
        for c in s.chars() {
            let o = self.key(doc, Key::Char(c), host, view);
            out.ex.extend(o.ex);
            if o.message.is_some() {
                out.message = o.message;
                out.error = o.error;
            }
        }
        out
    }

    /// One key.
    pub fn key(&mut self, doc: &mut Document, key: Key, host: &dyn Host, view: View) -> Outcome {
        if self.prompt.is_some() {
            return self.prompt_key(doc, key, view);
        }
        if self.mode.is_insert() {
            return self.insert_key(doc, key);
        }
        let c = match key {
            Key::Char(c) => c,
            Key::Esc => ESC,
            Key::Enter => 'j',
            Key::Backspace | Key::Left => 'h',
            Key::Right => 'l',
            Key::Up => 'k',
            Key::Down => 'j',
            Key::Home => '0',
            Key::End => '$',
            Key::PageUp => CTRL_B,
            Key::PageDown => CTRL_F,
            Key::Delete => 'x',
            Key::Tab => return Outcome::handled(),
            Key::Ctrl('r') => CTRL_R,
            Key::Ctrl('v') => CTRL_V,
            Key::Ctrl('d') => CTRL_D,
            Key::Ctrl('u') => CTRL_U,
            Key::Ctrl('f') => CTRL_F,
            Key::Ctrl('b') => CTRL_B,
            Key::Ctrl('[') => ESC,
            Key::Ctrl(_) => return Outcome::default(),
        };
        if key == Key::Enter && self.mode.is_visual() {
            return Outcome::handled();
        }
        self.pending.push(c);
        if self.mode.is_visual() {
            self.visual_keys(doc, host, view)
        } else {
            let keys = self.pending.clone();
            match parse_normal(&keys) {
                Parsed::More => Outcome::handled(),
                Parsed::Bad => {
                    self.pending.clear();
                    Outcome::handled()
                }
                Parsed::Cmd(cmd) => {
                    self.pending.clear();
                    self.run(doc, cmd, &keys, host, view)
                }
            }
        }
    }

    // ---- insert mode ------------------------------------------------------------------------

    fn record(&mut self, i: Ins) {
        if let Some(r) = &mut self.recording {
            r.insert.push(i);
        }
    }

    fn insert_text(&mut self, doc: &mut Document, s: &str) {
        if self.mode == Mode::Replace {
            doc.overwrite_text(s);
        } else {
            doc.type_text(s);
        }
        self.record(Ins::Text(s.to_string()));
    }

    fn insert_key(&mut self, doc: &mut Document, key: Key) -> Outcome {
        match key {
            Key::Esc | Key::Ctrl('[') => {
                self.leave_insert(doc);
                Outcome::handled()
            }
            Key::Char(c) => {
                self.insert_text(doc, &c.to_string());
                Outcome::handled()
            }
            Key::Enter => {
                doc.newline();
                self.record(Ins::Newline);
                Outcome::handled()
            }
            Key::Tab => {
                self.insert_text(doc, "\t");
                Outcome::handled()
            }
            Key::Backspace => {
                doc.delete_backward();
                self.record(Ins::Back);
                Outcome::handled()
            }
            Key::Ctrl('w') => {
                doc.delete_word_backward();
                self.record(Ins::DelWord);
                Outcome::handled()
            }
            Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End => {
                // A move inside Insert mode starts a new change (as in Vim).
                if self.block_carets.is_some() {
                    return Outcome::handled();
                }
                match key {
                    Key::Left => doc.move_left(false),
                    Key::Right => doc.move_right(false),
                    Key::Up => doc.move_lines(-1, false),
                    Key::Down => doc.move_lines(1, false),
                    Key::Home => doc.line_start(false),
                    _ => doc.line_end(false),
                }
                doc.end_group();
                doc.begin_group();
                self.recording = Some(Change {
                    keys: vec!['i'],
                    insert: Vec::new(),
                });
                Outcome::handled()
            }
            _ => Outcome::default(),
        }
    }

    fn leave_insert(&mut self, doc: &mut Document) {
        doc.end_group();
        if let Some(r) = self.recording.take() {
            self.last_change = Some(r);
        }
        self.mode = Mode::Normal;
        let head = if self.block_carets.take().is_some() {
            doc.selection().ranges()[0].head
        } else {
            doc.selection().primary().head
        };
        let r = rope(doc);
        let l = line_of(r, head);
        let p = if head > line_start(r, l) {
            prev_g(r, head)
        } else {
            head
        };
        doc.set_caret(&Selection::point(clamp(rope(doc), p)));
    }

    fn enter_insert(&mut self, doc: &mut Document, keys: &[char], pos: usize) {
        self.mode = Mode::Insert;
        doc.set_caret(&Selection::point(pos));
        self.recording = Some(Change {
            keys: keys.to_vec(),
            insert: Vec::new(),
        });
    }

    // ---- registers --------------------------------------------------------------------------

    fn store(&mut self, reg: Option<char>, r: Reg, yank: bool, host: &dyn Host) {
        match reg {
            Some('_') => return,
            Some('+' | '*') => host.set_clipboard(&r.text),
            Some(c) if c.is_ascii_uppercase() => {
                let e = self.regs.entry(c.to_ascii_lowercase()).or_default();
                e.text.push_str(&r.text);
                e.linewise |= r.linewise;
            }
            Some(c) if c != '"' => {
                self.regs.insert(c, r.clone());
            }
            _ => {}
        }
        if yank && reg.is_none() {
            self.regs.insert('0', r.clone());
        }
        let stored = match reg {
            Some(c) if c.is_ascii_uppercase() => {
                self.regs.get(&c.to_ascii_lowercase()).cloned().unwrap_or(r)
            }
            _ => r,
        };
        self.regs.insert('"', stored);
    }

    fn fetch(&self, reg: Option<char>, host: &dyn Host) -> Option<Reg> {
        match reg {
            Some('+' | '*') => host.clipboard().map(|t| Reg {
                linewise: t.ends_with('\n'),
                text: t,
                block: false,
            }),
            Some('_') => None,
            Some(c) => self.regs.get(&c.to_ascii_lowercase()).cloned(),
            None => self.regs.get(&'"').cloned(),
        }
    }

    // ---- motions ----------------------------------------------------------------------------

    fn find_char(
        r: &Rope,
        from: usize,
        kind: char,
        ch: char,
        count: usize,
        repeat: bool,
    ) -> Option<usize> {
        let l = line_of(r, from);
        let (s, e) = (line_start(r, l), line_end(r, l));
        let mut p = from;
        for it in 0..count {
            // `t`/`T` repeated (`;`, a count) skip the character right next to the cursor.
            let skip = repeat || it > 0;
            match kind {
                'f' => {
                    let mut j = p + 1;
                    while j < e && r.char(j) != ch {
                        j += 1;
                    }
                    if j >= e {
                        return None;
                    }
                    p = j;
                }
                't' => {
                    let mut j = if skip { p + 2 } else { p + 1 };
                    while j < e && r.char(j) != ch {
                        j += 1;
                    }
                    if j >= e {
                        return None;
                    }
                    p = j - 1;
                }
                'F' => {
                    let mut j = p;
                    loop {
                        if j <= s {
                            return None;
                        }
                        j -= 1;
                        if r.char(j) == ch {
                            break;
                        }
                    }
                    p = j;
                }
                _ => {
                    let mut j = if skip { p.saturating_sub(1) } else { p };
                    loop {
                        if j <= s {
                            return None;
                        }
                        j -= 1;
                        if r.char(j) == ch {
                            break;
                        }
                    }
                    p = j + 1;
                }
            }
        }
        Some(p)
    }

    fn search_from(
        &self,
        doc: &Document,
        pat: &str,
        from: usize,
        fwd: bool,
    ) -> Result<(usize, bool), String> {
        let re = self.regex(pat)?;
        let r = rope(doc);
        let s = r.to_string();
        let from_b = r.char_to_byte(from.min(r.len_chars()));
        if fwd {
            let start = s[from_b..]
                .chars()
                .next()
                .map_or(s.len(), |c| from_b + c.len_utf8());
            if let Some(m) = re.find_at(&s, start.min(s.len())) {
                return Ok((r.byte_to_char(m.start()), false));
            }
            if let Some(m) = re.find(&s) {
                return Ok((r.byte_to_char(m.start()), true));
            }
        } else {
            let mut before = None;
            let mut last = None;
            for m in re.find_iter(&s) {
                if m.start() < from_b {
                    before = Some(m.start());
                }
                last = Some(m.start());
            }
            if let Some(b) = before {
                return Ok((r.byte_to_char(b), false));
            }
            if let Some(b) = last {
                return Ok((r.byte_to_char(b), true));
            }
        }
        Err(format!("E486: Pattern not found: {pat}"))
    }

    /// Where a motion goes from `cur`, and how its range is taken.
    fn eval(
        &mut self,
        doc: &Document,
        cur: usize,
        m: Mot,
        count: Option<usize>,
        op: bool,
        view: View,
    ) -> Result<(usize, Kind, Option<usize>), String> {
        let r = rope(doc);
        let n = count.unwrap_or(1).max(1);
        let len = r.len_chars();
        let l = line_of(r, cur);
        let mut goal = None;
        let res = match m {
            Mot::Left => {
                let s = line_start(r, l);
                let mut p = cur;
                for _ in 0..n {
                    if p > s {
                        p = prev_g(r, p);
                    }
                }
                (p, Kind::Exclusive)
            }
            Mot::Right => {
                let e = line_end(r, l);
                let mut p = cur;
                for _ in 0..n {
                    if p < e {
                        p = next_g(r, p);
                    }
                }
                if !op && p >= e && e > line_start(r, l) {
                    p = prev_g(r, e);
                }
                (p, Kind::Exclusive)
            }
            Mot::Up | Mot::Down => {
                let target = if m == Mot::Up {
                    l.saturating_sub(n)
                } else {
                    (l + n).min(last_line(r))
                };
                let (_, col) = doc.line_col(cur);
                let want = doc.selection().primary().goal.unwrap_or(col);
                goal = Some(want);
                let off = text::offset_at_column(r.line(target), want, doc.tab_width);
                let p = line_start(r, target) + off.min(line_content_len(r, target));
                (p, Kind::Linewise)
            }
            Mot::Word(big) => {
                let mut p = cur;
                for i in 0..n {
                    let next = word_fwd(r, p, big);
                    // An operator stops at the end of the line of the last word (`dw`).
                    if op && i + 1 == n && line_of(r, next) > line_of(r, p) {
                        let pl = line_of(r, p);
                        let at_end = line_end(r, pl);
                        p = if at_end > p { at_end } else { next };
                        break;
                    }
                    p = next;
                }
                (p.min(len), Kind::Exclusive)
            }
            Mot::WordEnd(big) => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_end_fwd(r, p, big);
                }
                (p, Kind::Inclusive)
            }
            Mot::WordBack(big) => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_back(r, p, big);
                }
                (p, Kind::Exclusive)
            }
            Mot::WordEndBack(big) => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_end_back(r, p, big);
                }
                (p, Kind::Inclusive)
            }
            Mot::Zero => (line_start(r, l), Kind::Exclusive),
            Mot::FirstNonBlank => (first_nonblank(r, l), Kind::Exclusive),
            Mot::LineEnd => {
                let tl = (l + n - 1).min(last_line(r));
                let e = line_end(r, tl);
                let p = if op || e == line_start(r, tl) {
                    e
                } else {
                    prev_g(r, e)
                };
                goal = Some(usize::MAX);
                (p, if op { Kind::Exclusive } else { Kind::Inclusive })
            }
            Mot::DocTop | Mot::DocBottom => {
                let tl = match (m, count) {
                    (_, Some(c)) => (c.max(1) - 1).min(last_line(r)),
                    (Mot::DocTop, None) => 0,
                    _ => last_line(r),
                };
                (first_nonblank(r, tl), Kind::Linewise)
            }
            Mot::Find(kind, ch) => {
                self.last_find = Some((kind, ch));
                let p = Self::find_char(r, cur, kind, ch, n, false).ok_or_else(String::new)?;
                (
                    p,
                    if kind == 'f' || kind == 't' {
                        Kind::Inclusive
                    } else {
                        Kind::Exclusive
                    },
                )
            }
            Mot::RepeatFind(rev) => {
                let (kind, ch) = self.last_find.ok_or_else(String::new)?;
                let kind = if rev {
                    match kind {
                        'f' => 'F',
                        'F' => 'f',
                        't' => 'T',
                        _ => 't',
                    }
                } else {
                    kind
                };
                let p = Self::find_char(r, cur, kind, ch, n, true).ok_or_else(String::new)?;
                (
                    p,
                    if kind == 'f' || kind == 't' {
                        Kind::Inclusive
                    } else {
                        Kind::Exclusive
                    },
                )
            }
            Mot::Match => {
                let e = line_end(r, l);
                let mut i = cur;
                while i < e && pair_of(r.char(i)).is_none() {
                    i += 1;
                }
                if i >= e {
                    return Err(String::new());
                }
                (match_at(r, i).ok_or_else(String::new)?, Kind::Inclusive)
            }
            Mot::ParaFwd => {
                let last = r.len_lines() - 1;
                let mut k = l;
                for _ in 0..n {
                    while k < last && is_blank_line(r, k) {
                        k += 1;
                    }
                    while k < last && !is_blank_line(r, k) {
                        k += 1;
                    }
                }
                let p = if is_blank_line(r, k) {
                    line_start(r, k)
                } else {
                    len
                };
                (p, Kind::Exclusive)
            }
            Mot::ParaBack => {
                let mut k = l;
                for _ in 0..n {
                    while k > 0 && is_blank_line(r, k) {
                        k -= 1;
                    }
                    while k > 0 && !is_blank_line(r, k) {
                        k -= 1;
                    }
                }
                (line_start(r, k), Kind::Exclusive)
            }
            Mot::ScreenTop | Mot::ScreenMid | Mot::ScreenBottom => {
                let bottom = (view.top + view.rows.max(1) - 1).min(last_line(r));
                let tl = match m {
                    Mot::ScreenTop => (view.top + n - 1).min(bottom),
                    Mot::ScreenBottom => bottom.saturating_sub(n - 1).max(view.top),
                    _ => (view.top + bottom) / 2,
                };
                (first_nonblank(r, tl), Kind::Linewise)
            }
            Mot::HalfDown | Mot::HalfUp | Mot::PageDown | Mot::PageUp => {
                let step = match m {
                    Mot::HalfDown | Mot::HalfUp => (view.rows / 2).max(1),
                    _ => view.rows.saturating_sub(2).max(1),
                } * n;
                let tl = if matches!(m, Mot::HalfDown | Mot::PageDown) {
                    (l + step).min(last_line(r))
                } else {
                    l.saturating_sub(step)
                };
                (first_nonblank(r, tl), Kind::Linewise)
            }
            Mot::SearchNext(rev) => {
                let (pat, fwd) = self
                    .search
                    .clone()
                    .ok_or("E35: No previous regular expression")?;
                let mut p = cur;
                let mut wrapped = false;
                for _ in 0..n {
                    let (q, w) = self.search_from(doc, &pat, p, fwd != rev)?;
                    wrapped |= w;
                    p = q;
                }
                self.highlight = true;
                let _ = wrapped;
                (p, Kind::Exclusive)
            }
            Mot::Star(back) => {
                let (a, b) = text::word_at(r.slice(..), cur);
                let word: String = r.slice(a..b).to_string();
                if word.trim().is_empty() || !word.chars().any(|c| c.is_alphanumeric() || c == '_')
                {
                    return Err("E348: No string under cursor".into());
                }
                let pat = format!("\\<{}\\>", word.replace('\\', "\\\\"));
                self.search = Some((pat.clone(), !back));
                self.highlight = true;
                let mut p = if back { a } else { cur };
                for _ in 0..n {
                    p = self.search_from(doc, &pat, p, !back)?.0;
                }
                (p, Kind::Exclusive)
            }
        };
        Ok((res.0, res.1, goal))
    }

    /// The range of a text object around `cur`: (from, to, linewise).
    fn object(
        &self,
        doc: &Document,
        cur: usize,
        inner: bool,
        o: Obj,
        count: usize,
    ) -> Option<(usize, usize, bool)> {
        let r = rope(doc);
        let len = r.len_chars();
        match o {
            Obj::Word(big) => {
                if len == 0 {
                    return None;
                }
                let at = cur.min(len - 1);
                let cls = |i: usize| class(r.char(i), big);
                let (mut a, mut b) = (at, at);
                let c0 = cls(at);
                let l = line_of(r, at);
                let (ls, le) = (line_start(r, l), line_end(r, l));
                while a > ls && cls(a - 1) == c0 {
                    a -= 1;
                }
                while b < le && cls(b) == c0 {
                    b += 1;
                }
                for _ in 1..count {
                    let mut nb = b;
                    let c1 = if nb < le { cls(nb) } else { break };
                    while nb < le && cls(nb) == c1 {
                        nb += 1;
                    }
                    b = nb;
                }
                if !inner {
                    // The white space after (or, at the end of the line, before).
                    let mut nb = b;
                    while nb < le && is_blank(r.char(nb)) {
                        nb += 1;
                    }
                    if nb > b && c0 != 0 {
                        b = nb;
                    } else {
                        while a > ls && is_blank(r.char(a - 1)) {
                            a -= 1;
                        }
                    }
                }
                Some((a, b, false))
            }
            Obj::Quote(q) => {
                let l = line_of(r, cur);
                let (ls, le) = (line_start(r, l), line_end(r, l));
                let quotes: Vec<usize> = (ls..le).filter(|i| r.char(*i) == q).collect();
                // The pair around the cursor (or the first pair after it).
                let mut pair = None;
                for w in quotes.chunks(2) {
                    if let [a, b] = w
                        && cur <= *b
                    {
                        pair = Some((*a, *b));
                        break;
                    }
                }
                let (a, b) = pair?;
                if inner {
                    Some((a + 1, b, false))
                } else {
                    let mut e = b + 1;
                    while e < le && is_blank(r.char(e)) {
                        e += 1;
                    }
                    Some((a, e, false))
                }
            }
            Obj::Pair(open, close) => {
                let (mut a, mut b) = enclosing(r, cur, open, close)?;
                for _ in 1..count {
                    let (na, nb) = enclosing(r, a.checked_sub(1)?, open, close)?;
                    a = na;
                    b = nb;
                }
                if inner {
                    let mut ia = a + 1;
                    let mut ib = b;
                    // A block on lines of its own: the lines between.
                    if ia < len && is_eol(r.char(ia)) {
                        ia = next_line_start(r, line_of(r, ia));
                        let bl = line_of(r, b);
                        if first_nonblank(r, bl) == b || line_start(r, bl) == b {
                            ib = line_start(r, bl);
                        }
                    }
                    Some((ia.min(ib), ib, false))
                } else {
                    Some((a, b + 1, false))
                }
            }
            Obj::Tag => {
                let s = r.to_string();
                let cb = r.char_to_byte(cur.min(len));
                // The innermost <name …>…</name> around the cursor.
                let re = regex::Regex::new(r"<(/?)([A-Za-z][\w:.-]*)[^>]*?(/?)>").ok()?;
                let mut stack: Vec<(String, usize, usize)> = Vec::new();
                let mut best: Option<(usize, usize, usize, usize)> = None;
                for c in re.captures_iter(&s) {
                    let m = c.get(0)?;
                    let name = c[2].to_string();
                    if &c[3] == "/" {
                        continue;
                    }
                    if &c[1] == "/" {
                        if let Some(k) = stack.iter().rposition(|(n, _, _)| *n == name) {
                            let (_, os, oe) = stack[k].clone();
                            stack.truncate(k);
                            if os <= cb && cb < m.end() {
                                let span = m.end() - os;
                                if best.is_none_or(|(a, _, _, d)| span < d - a) {
                                    best = Some((os, oe, m.start(), m.end()));
                                }
                            }
                        }
                    } else {
                        stack.push((name, m.start(), m.end()));
                    }
                }
                let (os, oe, cs, ce) = best?;
                let (a, b) = if inner { (oe, cs) } else { (os, ce) };
                Some((r.byte_to_char(a), r.byte_to_char(b), false))
            }
            Obj::Para => {
                let l = line_of(r, cur);
                let blank = is_blank_line(r, l);
                let mut a = l;
                while a > 0 && is_blank_line(r, a - 1) == blank {
                    a -= 1;
                }
                let last = last_line(r);
                let mut b = l;
                for i in 0..count {
                    if i > 0 && b < last {
                        b += 1;
                    }
                    let kind = is_blank_line(r, b);
                    while b < last && is_blank_line(r, b + 1) == kind {
                        b += 1;
                    }
                }
                if !inner && b < last {
                    let kind = is_blank_line(r, b + 1);
                    b += 1;
                    while b < last && is_blank_line(r, b + 1) == kind {
                        b += 1;
                    }
                }
                Some((line_start(r, a), line_end(r, b), true))
            }
        }
    }

    // ---- running commands -------------------------------------------------------------------

    /// The text of lines `a..=b` as a linewise register (ending with a line break).
    fn line_text(doc: &Document, a: usize, b: usize) -> String {
        let r = rope(doc);
        let mut t = r.slice(line_start(r, a)..next_line_start(r, b)).to_string();
        if !t.ends_with(['\n', '\r']) {
            t.push_str(doc.eol.as_str());
        }
        t
    }

    /// Deletes lines `a..=b` (the line break before them for the last lines).
    fn line_span(doc: &Document, a: usize, b: usize) -> (usize, usize) {
        let r = rope(doc);
        let from = line_start(r, a);
        let to = next_line_start(r, b);
        if b >= last_line(r)
            && to == r.len_chars()
            && a > 0
            && !r.slice(from..to).to_string().ends_with(['\n', '\r'])
        {
            (line_end(r, a - 1), to)
        } else {
            (from, to)
        }
    }

    fn shift(doc: &mut Document, a: usize, b: usize, count: usize, right: bool) {
        let r = rope(doc);
        let mut parts = Vec::new();
        for l in a..=b {
            let s = line_start(r, l);
            if right {
                if line_content_len(r, l) > 0 {
                    parts.push((s, s, "\t".repeat(count)));
                }
            } else {
                let mut i = s;
                let e = line_end(r, l);
                for _ in 0..count {
                    if i < e && r.char(i) == '\t' {
                        i += 1;
                    } else {
                        let mut n = 0;
                        while i < e && r.char(i) == ' ' && n < doc.tab_width.max(1) {
                            i += 1;
                            n += 1;
                        }
                    }
                }
                if i > s {
                    parts.push((s, i, String::new()));
                }
            }
        }
        if !parts.is_empty() {
            doc.replace(parts, None);
        }
        let p = first_nonblank(rope(doc), a);
        doc.set_caret(&Selection::point(p));
    }

    /// Applies `op` to `from..to` (`linewise`: whole lines). Returns the outcome; Change leaves
    /// the document in Insert mode.
    #[allow(clippy::too_many_arguments)]
    fn apply_op(
        &mut self,
        doc: &mut Document,
        op: Op,
        from: usize,
        to: usize,
        linewise: bool,
        reg: Option<char>,
        keys: &[char],
        host: &dyn Host,
    ) -> Outcome {
        let r = rope(doc);
        let la = line_of(r, from);
        let lb = if linewise {
            line_of(r, to)
        } else {
            line_of(r, to.saturating_sub(1).max(from))
        };
        match op {
            Op::Yank => {
                let text = if linewise {
                    Self::line_text(doc, la, lb)
                } else {
                    r.slice(from..to).to_string()
                };
                self.store(
                    reg,
                    Reg {
                        text,
                        linewise,
                        block: false,
                    },
                    true,
                    host,
                );
                // The cursor goes to the start of what was yanked (a line above keeps the column).
                let cur = doc.selection().primary().head;
                let p = if !linewise {
                    from
                } else if la < line_of(r, cur) {
                    let col = cur - line_start(r, line_of(r, cur));
                    line_start(r, la) + col.min(line_content_len(r, la))
                } else {
                    cur
                };
                self.put(doc, p);
                Outcome::handled()
            }
            Op::Delete | Op::Change => {
                if linewise {
                    let text = Self::line_text(doc, la, lb);
                    self.store(
                        reg,
                        Reg {
                            text,
                            linewise: true,
                            block: false,
                        },
                        false,
                        host,
                    );
                    if op == Op::Change {
                        // Keep one empty line, indented like the first, and type there.
                        let indent: String = rope(doc)
                            .slice(line_start(rope(doc), la)..line_end(rope(doc), la))
                            .chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .collect();
                        let s = line_start(rope(doc), la);
                        let e = line_end(rope(doc), lb);
                        doc.begin_group();
                        doc.replace(
                            vec![(s, e, indent.clone())],
                            Some(s + indent.chars().count()),
                        );
                        let at = s + indent.chars().count();
                        self.enter_insert(doc, keys, at);
                        return Outcome::handled();
                    }
                    let (a, b) = Self::line_span(doc, la, lb);
                    doc.replace(vec![(a, b, String::new())], Some(a));
                    let r = rope(doc);
                    let l = line_of(r, a).min(last_line(r));
                    doc.set_caret(&Selection::point(first_nonblank(r, l)));
                    return Outcome::handled();
                }
                let text = r.slice(from..to).to_string();
                self.store(
                    reg,
                    Reg {
                        text,
                        linewise: false,
                        block: false,
                    },
                    false,
                    host,
                );
                if op == Op::Change {
                    doc.begin_group();
                    doc.replace(vec![(from, to, String::new())], Some(from));
                    self.enter_insert(doc, keys, from);
                    return Outcome::handled();
                }
                doc.replace(vec![(from, to, String::new())], Some(from));
                self.put(doc, from);
                Outcome::handled()
            }
            Op::Indent | Op::Outdent => {
                Self::shift(doc, la, lb, 1, op == Op::Indent);
                Outcome::handled()
            }
            Op::Toggle | Op::Lower | Op::Upper => {
                let (a, b) = if linewise {
                    (line_start(r, la), line_end(r, lb))
                } else {
                    (from, to)
                };
                let t = r.slice(a..b).to_string();
                let n = match op {
                    Op::Toggle => toggle(&t),
                    Op::Lower => lower(&t),
                    _ => upper(&t),
                };
                if n != t {
                    doc.replace(vec![(a, b, n)], Some(a));
                }
                self.put(doc, a);
                Outcome::handled()
            }
        }
    }

    fn run(
        &mut self,
        doc: &mut Document,
        cmd: Cmd,
        keys: &[char],
        host: &dyn Host,
        view: View,
    ) -> Outcome {
        let cur = doc.selection().primary().head;
        let r = rope(doc);
        let n = cmd.count.unwrap_or(1).max(1);
        match cmd.act {
            Act::Nothing => Outcome::handled(),
            Act::Move(m) => match self.eval(doc, cur, m, cmd.count, false, view) {
                Ok((p, _, goal)) => {
                    let p = clamp(rope(doc), p);
                    let mut range = Range::point(p);
                    range.goal = goal;
                    doc.set_caret(&Selection::single(range));
                    Outcome::handled()
                }
                Err(e) if e.is_empty() => Outcome::handled(),
                Err(e) => Outcome::err(e),
            },
            Act::Op(op, target) => {
                let changes = op != Op::Yank;
                let range = match target {
                    Target::Line => {
                        let l = line_of(r, cur);
                        let b = (l + n - 1).min(last_line(r));
                        Some((line_start(r, l), line_end(r, b), true))
                    }
                    Target::Obj(inner, o) => self.object(doc, cur, inner, o, n),
                    Target::Mot(m) => {
                        // `cw` on a non-blank changes to the end of the word (as `ce`, but a word's
                        // last character changes alone).
                        let res = match m {
                            Mot::Word(big)
                                if op == Op::Change
                                    && cur < r.len_chars()
                                    && !is_blank(r.char(cur)) =>
                            {
                                let mut p = word_end_in_run(r, cur, big);
                                for _ in 1..n {
                                    p = word_end_fwd(r, p, big);
                                }
                                Ok((p, Kind::Inclusive, None))
                            }
                            m => self.eval(doc, cur, m, cmd.count, true, view),
                        };
                        match res {
                            Ok((p, kind, _)) => {
                                let r = rope(doc);
                                let (a, b) = (cur.min(p), cur.max(p));
                                match kind {
                                    Kind::Linewise => {
                                        let (la, lb) = (line_of(r, a), line_of(r, b));
                                        Some((line_start(r, la), line_end(r, lb), true))
                                    }
                                    Kind::Inclusive => {
                                        Some((a, next_g(r, b).min(r.len_chars()), false))
                                    }
                                    Kind::Exclusive => Some((a, b, false)),
                                }
                            }
                            Err(e) if e.is_empty() => None,
                            Err(e) => return Outcome::err(e),
                        }
                    }
                };
                let Some((from, to, linewise)) = range else {
                    return Outcome::handled();
                };
                if !linewise && from == to && op != Op::Change {
                    return Outcome::handled();
                }
                if changes {
                    doc.begin_group();
                }
                let out = self.apply_op(doc, op, from, to, linewise, cmd.reg, keys, host);
                if changes && !self.mode.is_insert() {
                    doc.end_group();
                    self.last_change = Some(Change {
                        keys: keys.to_vec(),
                        insert: Vec::new(),
                    });
                }
                out
            }
            Act::Insert(c) => {
                let l = line_of(r, cur);
                let (ls, le) = (line_start(r, l), line_end(r, l));
                let indent: String = r
                    .slice(ls..le)
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let simple = match c {
                    'i' => Some(cur),
                    'a' if le > ls => Some(next_g(r, cur).min(le)),
                    'a' => Some(cur),
                    'I' => Some(first_nonblank(r, l).min(le)),
                    'A' => Some(le),
                    _ => None,
                };
                doc.begin_group();
                let pos = match simple {
                    Some(p) => p,
                    None => {
                        let eol = doc.eol.as_str().to_string();
                        let n = indent.chars().count();
                        if c == 'o' {
                            let caret = le + eol.chars().count() + n;
                            doc.replace(vec![(le, le, format!("{eol}{indent}"))], Some(caret));
                            caret
                        } else {
                            doc.replace(vec![(ls, ls, format!("{indent}{eol}"))], Some(ls + n));
                            ls + n
                        }
                    }
                };
                self.enter_insert(doc, keys, pos);
                Outcome::handled()
            }
            Act::ReplaceMode => {
                doc.begin_group();
                self.enter_insert(doc, keys, cur);
                self.mode = Mode::Replace;
                Outcome::handled()
            }
            Act::Paste(before) => {
                let Some(reg) = self.fetch(cmd.reg, host) else {
                    return Outcome::err("E353: Nothing in register");
                };
                doc.begin_group();
                self.paste(doc, &reg, before, n, cur);
                doc.end_group();
                self.last_change = Some(Change {
                    keys: keys.to_vec(),
                    insert: Vec::new(),
                });
                Outcome::handled()
            }
            Act::Replace(ch) => {
                let l = line_of(r, cur);
                let e = line_end(r, l);
                let mut end = cur;
                for _ in 0..n {
                    if end >= e {
                        return Outcome::handled();
                    }
                    end = next_g(r, end);
                }
                let (ins, caret) = if ch == '\r' || ch == '\n' {
                    (
                        doc.eol.as_str().to_string(),
                        cur + doc.eol.as_str().chars().count(),
                    )
                } else {
                    (ch.to_string().repeat(n), cur + n - 1)
                };
                doc.begin_group();
                doc.replace(vec![(cur, end, ins)], Some(caret));
                doc.end_group();
                self.last_change = Some(Change {
                    keys: keys.to_vec(),
                    insert: Vec::new(),
                });
                self.put(doc, caret);
                Outcome::handled()
            }
            Act::Join => {
                let l = line_of(r, cur);
                let lines = n.max(2) - 1;
                doc.begin_group();
                let caret = Self::join(doc, l, lines);
                doc.end_group();
                self.last_change = Some(Change {
                    keys: keys.to_vec(),
                    insert: Vec::new(),
                });
                if let Some(c) = caret {
                    self.put(doc, c);
                }
                Outcome::handled()
            }
            Act::ToggleCase => {
                let l = line_of(r, cur);
                let e = line_end(r, l);
                if cur >= e {
                    return Outcome::handled();
                }
                let mut end = cur;
                for _ in 0..n {
                    if end < e {
                        end = next_g(r, end);
                    }
                }
                let t = r.slice(cur..end).to_string();
                doc.begin_group();
                doc.replace(vec![(cur, end, toggle(&t))], Some(end));
                doc.end_group();
                self.last_change = Some(Change {
                    keys: keys.to_vec(),
                    insert: Vec::new(),
                });
                self.put(doc, end);
                Outcome::handled()
            }
            Act::Undo => {
                let mut any = false;
                for _ in 0..n {
                    any |= doc.undo();
                }
                let p = clamp(rope(doc), doc.selection().primary().from());
                doc.set_caret(&Selection::point(p));
                if any {
                    Outcome::handled()
                } else {
                    Outcome::msg("Already at oldest change")
                }
            }
            Act::Redo => {
                let mut any = false;
                for _ in 0..n {
                    any |= doc.redo();
                }
                let p = clamp(rope(doc), doc.selection().primary().from());
                doc.set_caret(&Selection::point(p));
                if any {
                    Outcome::handled()
                } else {
                    Outcome::msg("Already at newest change")
                }
            }
            Act::Repeat => self.repeat(doc, cmd.count, host, view),
            Act::Visual(m) => {
                self.mode = m;
                self.anchor = cur;
                self.head = cur;
                self.show(doc);
                Outcome::handled()
            }
            Act::Prompt(c) => {
                self.prompt = Some(c.to_string());
                Outcome::handled()
            }
        }
    }

    fn paste(&mut self, doc: &mut Document, reg: &Reg, before: bool, n: usize, cur: usize) {
        let r = rope(doc);
        let l = line_of(r, cur);
        if reg.block {
            // One line of the register per line, at the cursor's column (Step 96 brings column
            // mode proper).
            let col =
                doc.line_col(cur).1 + usize::from(!before && line_end(r, l) > line_start(r, l));
            let lines: Vec<&str> = reg
                .text
                .split(['\n'])
                .map(|s| s.trim_end_matches('\r'))
                .collect();
            let mut parts = Vec::new();
            let mut extra = String::new();
            for (i, t) in lines.iter().enumerate() {
                let line = l + i;
                if line > last_line(r) {
                    extra.push_str(doc.eol.as_str());
                    extra.push_str(&" ".repeat(col));
                    extra.push_str(&t.repeat(n));
                    continue;
                }
                let s = line_start(r, line);
                let len = line_content_len(r, line);
                let off = text::offset_at_column(r.line(line), col, doc.tab_width).min(len);
                let have = doc.line_col(s + off).1;
                let pad = " ".repeat(col.saturating_sub(have));
                parts.push((s + off, s + off, format!("{pad}{}", t.repeat(n))));
            }
            if !extra.is_empty() {
                let e = r.len_chars();
                parts.push((e, e, extra));
            }
            doc.replace(parts, None);
            self.put(doc, cur);
            return;
        }
        let text = reg.text.repeat(n);
        if reg.linewise {
            let eol = doc.eol.as_str();
            let (at, ins) = if before {
                (line_start(r, l), text)
            } else {
                let at = next_line_start(r, l);
                if at == r.len_chars()
                    && !r
                        .slice(line_start(r, l)..at)
                        .to_string()
                        .ends_with(['\n', '\r'])
                {
                    // The last line has no line break: add one before.
                    let body = text
                        .strip_suffix(['\n'])
                        .map(|t| t.strip_suffix('\r').unwrap_or(t))
                        .unwrap_or(&text);
                    (at, format!("{eol}{body}"))
                } else {
                    (at, text)
                }
            };
            let skip = usize::from(ins.starts_with(['\r', '\n'])) * eol.chars().count();
            doc.replace(vec![(at, at, ins)], Some(at + skip));
            let r = rope(doc);
            let p = first_nonblank(r, line_of(r, at + skip));
            doc.set_caret(&Selection::point(p));
        } else {
            let at = if before || line_end(r, l) == line_start(r, l) {
                cur
            } else {
                next_g(r, cur).min(line_end(r, l))
            };
            let len = text.chars().count();
            doc.replace(vec![(at, at, text)], Some(at + len));
            let p = if len > 0 {
                prev_g(rope(doc), at + len)
            } else {
                at
            };
            self.put(doc, p);
        }
    }

    /// Joins `lines` lines after line `l` to it (one space between, leading blanks dropped).
    fn join(doc: &mut Document, l: usize, lines: usize) -> Option<usize> {
        let mut caret = None;
        for _ in 0..lines {
            let r = rope(doc);
            if l >= last_line(r) {
                break;
            }
            let e = line_end(r, l);
            let ns = next_line_start(r, l);
            let ne = line_end(r, l + 1);
            let mut k = ns;
            while k < ne && matches!(r.char(k), ' ' | '\t') {
                k += 1;
            }
            let mut ee = e;
            while ee > line_start(r, l) && matches!(r.char(ee - 1), ' ' | '\t') {
                ee -= 1;
            }
            let sep = if k == ne || ee == line_start(r, l) || r.char(k) == ')' {
                ""
            } else {
                " "
            };
            doc.replace(vec![(ee, k, sep.to_string())], Some(ee));
            caret = Some(ee);
        }
        caret
    }

    fn repeat(
        &mut self,
        doc: &mut Document,
        count: Option<usize>,
        host: &dyn Host,
        view: View,
    ) -> Outcome {
        let Some(ch) = self.last_change.clone() else {
            return Outcome::handled();
        };
        let mut keys = ch.keys.clone();
        if let Some(c) = count {
            // The new count replaces the old one (after a register prefix).
            let start = if keys.first() == Some(&'"') { 2 } else { 0 };
            let (_, end) = digits(&keys, start);
            keys.splice(start..end, c.to_string().chars());
        }
        let Parsed::Cmd(cmd) = parse_normal(&keys) else {
            return Outcome::handled();
        };
        let out = self.run(doc, cmd, &keys, host, view);
        if self.mode.is_insert() {
            for i in &ch.insert {
                match i {
                    Ins::Text(s) => self.insert_text(doc, s),
                    Ins::Back => {
                        doc.delete_backward();
                        self.record(Ins::Back);
                    }
                    Ins::Newline => {
                        doc.newline();
                        self.record(Ins::Newline);
                    }
                    Ins::DelWord => {
                        doc.delete_word_backward();
                        self.record(Ins::DelWord);
                    }
                }
            }
            self.leave_insert(doc);
        }
        out
    }

    // ---- visual mode ------------------------------------------------------------------------

    /// The visual selection: (from, to) chars, or lines for Visual-line.
    fn visual_span(&self, doc: &Document) -> (usize, usize) {
        let r = rope(doc);
        let (a, b) = (self.anchor.min(self.head), self.anchor.max(self.head));
        match self.mode {
            Mode::VisualLine => (line_start(r, line_of(r, a)), line_end(r, line_of(r, b))),
            _ => (a, next_g(r, b).min(r.len_chars()).max(a)),
        }
    }

    /// The block's ranges, one per line (display columns, wide characters and tabs counted).
    fn block_ranges(&self, doc: &Document) -> Vec<(usize, usize)> {
        let r = rope(doc);
        let (ca, cb) = (doc.line_col(self.anchor).1, doc.line_col(self.head).1);
        let (left, right) = (ca.min(cb), ca.max(cb) + 1);
        let (la, lb) = (
            line_of(r, self.anchor).min(line_of(r, self.head)),
            line_of(r, self.anchor).max(line_of(r, self.head)),
        );
        (la..=lb)
            .map(|l| {
                let s = line_start(r, l);
                let len = line_content_len(r, l);
                let a = text::offset_at_column(r.line(l), left, doc.tab_width).min(len);
                let b = text::offset_at_column(r.line(l), right, doc.tab_width).min(len);
                (s + a, s + b.max(a))
            })
            .collect()
    }

    /// Shows the visual selection as the document's selection.
    fn show(&self, doc: &mut Document) {
        match self.mode {
            Mode::VisualBlock => {
                let ranges: Vec<Range> = self
                    .block_ranges(doc)
                    .into_iter()
                    .map(|(a, b)| Range::new(a, b))
                    .collect();
                let primary = if line_of(rope(doc), self.head) >= line_of(rope(doc), self.anchor) {
                    ranges.len() - 1
                } else {
                    0
                };
                doc.set_caret(&Selection::new(ranges, primary));
            }
            _ => {
                let (from, to) = self.visual_span(doc);
                let range = if self.head >= self.anchor {
                    Range::new(from, to)
                } else {
                    Range::new(to, from)
                };
                doc.set_caret(&Selection::single(range));
            }
        }
    }

    fn exit_visual(&mut self, doc: &mut Document, at: usize) {
        let r = rope(doc);
        let (a, b) = (self.anchor.min(self.head), self.anchor.max(self.head));
        self.last_visual = Some((line_of(r, a), line_of(r, b)));
        self.mode = Mode::Normal;
        doc.set_caret(&Selection::point(clamp(rope(doc), at)));
    }

    fn visual_keys(&mut self, doc: &mut Document, host: &dyn Host, view: View) -> Outcome {
        let k = self.pending.clone();
        let mut i = 0;
        let mut reg = None;
        if k.first() == Some(&'"') {
            match k.get(1) {
                None => return Outcome::handled(),
                Some(&r) => reg = Some(r),
            }
            i = 2;
        }
        let (count, j) = digits(&k, i);
        i = j;
        let Some(&c) = k.get(i) else {
            return Outcome::handled();
        };
        let done = |s: &mut Vim| s.pending.clear();
        let mode = self.mode;
        let (from, to) = self.visual_span(doc);
        let lines = {
            let r = rope(doc);
            (
                line_of(r, self.anchor.min(self.head)),
                line_of(r, self.anchor.max(self.head)),
            )
        };
        let linewise = mode == Mode::VisualLine;
        let block = mode == Mode::VisualBlock;
        let op = match c {
            'd' | 'x' => Some(Op::Delete),
            'y' => Some(Op::Yank),
            'c' | 's' => Some(Op::Change),
            '>' => Some(Op::Indent),
            '<' => Some(Op::Outdent),
            '~' => Some(Op::Toggle),
            'u' => Some(Op::Lower),
            'U' => Some(Op::Upper),
            _ => None,
        };
        if let Some(op) = op {
            done(self);
            if block {
                return self.block_op(doc, op, reg, host);
            }
            self.exit_visual(doc, from);
            let changes = op != Op::Yank;
            if changes {
                doc.begin_group();
            }
            let keys: Vec<char> = vec!['i'];
            let out = if linewise {
                let r = rope(doc);
                self.apply_op(
                    doc,
                    op,
                    line_start(r, lines.0),
                    line_end(r, lines.1),
                    true,
                    reg,
                    &keys,
                    host,
                )
            } else {
                self.apply_op(doc, op, from, to, false, reg, &keys, host)
            };
            if changes && !self.mode.is_insert() {
                doc.end_group();
            }
            if op == Op::Yank {
                let start = self.anchor.min(self.head);
                self.put(doc, start);
            }
            return out;
        }
        match c {
            ESC => {
                done(self);
                let h = self.head;
                self.exit_visual(doc, h);
                Outcome::handled()
            }
            'v' | 'V' | CTRL_V => {
                done(self);
                let want = match c {
                    'v' => Mode::Visual,
                    'V' => Mode::VisualLine,
                    _ => Mode::VisualBlock,
                };
                if want == mode {
                    let h = self.head;
                    self.exit_visual(doc, h);
                } else {
                    self.mode = want;
                    self.show(doc);
                }
                Outcome::handled()
            }
            'o' => {
                done(self);
                std::mem::swap(&mut self.anchor, &mut self.head);
                self.show(doc);
                Outcome::handled()
            }
            'D' | 'X' | 'Y' | 'C' | 'S' | 'R' => {
                done(self);
                self.exit_visual(doc, from);
                let op = match c {
                    'Y' => Op::Yank,
                    'C' | 'S' | 'R' => Op::Change,
                    _ => Op::Delete,
                };
                if op != Op::Yank {
                    doc.begin_group();
                }
                let r = rope(doc);
                let out = self.apply_op(
                    doc,
                    op,
                    line_start(r, lines.0),
                    line_end(r, lines.1),
                    true,
                    reg,
                    &['i'],
                    host,
                );
                if op != Op::Yank && !self.mode.is_insert() {
                    doc.end_group();
                }
                out
            }
            'J' => {
                done(self);
                self.exit_visual(doc, from);
                doc.begin_group();
                let caret = Self::join(doc, lines.0, (lines.1 - lines.0).max(1));
                doc.end_group();
                if let Some(c) = caret {
                    self.put(doc, c);
                }
                Outcome::handled()
            }
            'p' | 'P' => {
                done(self);
                let Some(reg_v) = self.fetch(reg, host) else {
                    let h = self.head;
                    self.exit_visual(doc, h);
                    return Outcome::err("E353: Nothing in register");
                };
                self.exit_visual(doc, from);
                doc.begin_group();
                let (a, b) = if linewise {
                    let r = rope(doc);
                    (line_start(r, lines.0), line_end(r, lines.1))
                } else {
                    (from, to)
                };
                let old = rope(doc).slice(a..b).to_string();
                let mut text = reg_v.text.clone();
                if reg_v.linewise && !linewise {
                    let eol = doc.eol.as_str();
                    text = format!("{eol}{}", text.trim_end_matches(['\r', '\n']));
                    text.push_str(eol);
                } else if !reg_v.linewise && linewise {
                    // nothing extra
                } else if reg_v.linewise && linewise {
                    text = text.trim_end_matches(['\r', '\n']).to_string();
                }
                let n = text.chars().count();
                doc.replace(vec![(a, b, text)], Some(a + n));
                doc.end_group();
                self.store(
                    None,
                    Reg {
                        text: old,
                        linewise,
                        block: false,
                    },
                    false,
                    host,
                );
                let p = if n > 0 { prev_g(rope(doc), a + n) } else { a };
                self.put(doc, p);
                Outcome::handled()
            }
            'r' => match k.get(i + 1) {
                None => Outcome::handled(),
                Some(&ESC) => {
                    done(self);
                    Outcome::handled()
                }
                Some(&ch) => {
                    done(self);
                    let spans: Vec<(usize, usize)> = if block {
                        self.block_ranges(doc)
                    } else if linewise {
                        let r = rope(doc);
                        (lines.0..=lines.1)
                            .map(|l| (line_start(r, l), line_end(r, l)))
                            .collect()
                    } else {
                        vec![(from, to)]
                    };
                    self.exit_visual(doc, from);
                    let r = rope(doc);
                    let parts: Vec<(usize, usize, String)> = spans
                        .into_iter()
                        .map(|(a, b)| {
                            let t: String = r
                                .slice(a..b)
                                .chars()
                                .map(|c| if is_eol(c) { c } else { ch })
                                .collect();
                            (a, b, t)
                        })
                        .collect();
                    doc.begin_group();
                    doc.replace(parts, Some(from));
                    doc.end_group();
                    self.put(doc, from);
                    Outcome::handled()
                }
            },
            'I' | 'A' if block => {
                done(self);
                let ranges = self.block_ranges(doc);
                let carets: Vec<usize> = ranges
                    .iter()
                    .map(|(a, b)| if c == 'I' { *a } else { *b })
                    .collect();
                self.mode = Mode::Normal;
                self.start_block_insert(doc, carets);
                Outcome::handled()
            }
            ':' => {
                done(self);
                let h = self.head;
                self.exit_visual(doc, h);
                self.prompt = Some(":'<,'>".into());
                Outcome::handled()
            }
            'i' | 'a' => match k.get(i + 1) {
                None => Outcome::handled(),
                Some(&o) => {
                    done(self);
                    let n = count.unwrap_or(1).max(1);
                    if let Some(obj) = parse_obj(o)
                        && let Some((a, b, lw)) = self.object(doc, self.head, c == 'i', obj, n)
                    {
                        if lw && self.mode == Mode::Visual {
                            self.mode = Mode::VisualLine;
                        }
                        self.anchor = a;
                        self.head = if b > a { prev_g(rope(doc), b) } else { a };
                        self.show(doc);
                    }
                    Outcome::handled()
                }
            },
            _ => match parse_motion(&k[i..]) {
                Err(true) => Outcome::handled(),
                Err(false) => {
                    done(self);
                    Outcome::handled()
                }
                Ok((m, _)) => {
                    done(self);
                    let head = self.head;
                    match self.eval(doc, head, m, count, false, view) {
                        Ok((p, _, _)) => {
                            self.head = clamp(rope(doc), p);
                            self.show(doc);
                            Outcome::handled()
                        }
                        Err(e) if e.is_empty() => Outcome::handled(),
                        Err(e) => Outcome::err(e),
                    }
                }
            },
        }
    }

    fn start_block_insert(&mut self, doc: &mut Document, carets: Vec<usize>) {
        doc.begin_group();
        let ranges: Vec<Range> = carets.iter().map(|p| Range::point(*p)).collect();
        doc.set_caret(&Selection::new(ranges, 0));
        self.block_carets = Some(carets);
        self.mode = Mode::Insert;
        self.recording = None;
    }

    fn block_op(
        &mut self,
        doc: &mut Document,
        op: Op,
        reg: Option<char>,
        host: &dyn Host,
    ) -> Outcome {
        let ranges = self.block_ranges(doc);
        let r = rope(doc);
        let text: Vec<String> = ranges
            .iter()
            .map(|(a, b)| r.slice(*a..*b).to_string())
            .collect();
        let first = ranges.first().map_or(0, |(a, _)| *a);
        self.exit_visual(doc, first);
        match op {
            Op::Yank => {
                self.store(
                    reg,
                    Reg {
                        text: text.join("\n"),
                        linewise: false,
                        block: true,
                    },
                    true,
                    host,
                );
                self.put(doc, first);
                Outcome::handled()
            }
            Op::Delete | Op::Change => {
                self.store(
                    reg,
                    Reg {
                        text: text.join("\n"),
                        linewise: false,
                        block: true,
                    },
                    false,
                    host,
                );
                doc.begin_group();
                let parts: Vec<(usize, usize, String)> = ranges
                    .iter()
                    .map(|(a, b)| (*a, *b, String::new()))
                    .collect();
                doc.replace(parts, Some(first));
                if op == Op::Change {
                    // The same column on every line, after the deletion.
                    let mut shift = 0;
                    let carets: Vec<usize> = ranges
                        .iter()
                        .map(|(a, b)| {
                            let p = a - shift;
                            shift += b - a;
                            p
                        })
                        .collect();
                    doc.end_group();
                    self.start_block_insert(doc, carets);
                    return Outcome::handled();
                }
                doc.end_group();
                self.put(doc, first);
                Outcome::handled()
            }
            Op::Indent | Op::Outdent => {
                let (la, lb) = (
                    line_of(rope(doc), first),
                    line_of(rope(doc), ranges.last().map_or(first, |(a, _)| *a)),
                );
                doc.begin_group();
                Self::shift(doc, la, lb, 1, op == Op::Indent);
                doc.end_group();
                Outcome::handled()
            }
            Op::Toggle | Op::Lower | Op::Upper => {
                let parts: Vec<(usize, usize, String)> = ranges
                    .iter()
                    .zip(&text)
                    .map(|((a, b), t)| {
                        let n = match op {
                            Op::Toggle => toggle(t),
                            Op::Lower => lower(t),
                            _ => upper(t),
                        };
                        (*a, *b, n)
                    })
                    .collect();
                doc.begin_group();
                doc.replace(parts, Some(first));
                doc.end_group();
                self.put(doc, first);
                Outcome::handled()
            }
        }
    }

    // ---- the command line -------------------------------------------------------------------

    fn prompt_key(&mut self, doc: &mut Document, key: Key, view: View) -> Outcome {
        let Some(p) = self.prompt.as_mut() else {
            return Outcome::default();
        };
        match key {
            Key::Esc | Key::Ctrl('[') | Key::Ctrl('c') => {
                self.prompt = None;
                Outcome::handled()
            }
            Key::Backspace => {
                p.pop();
                if p.is_empty() || p == ":'<,'" {
                    self.prompt = None;
                }
                Outcome::handled()
            }
            Key::Char(c) => {
                p.push(c);
                Outcome::handled()
            }
            Key::Tab => Outcome::handled(),
            Key::Enter => {
                let line = self.prompt.take().unwrap_or_default();
                let (kind, rest) = line.split_at(1);
                match kind {
                    ":" => self.ex(doc, rest, view),
                    _ => {
                        let fwd = kind == "/";
                        let pat = if rest.is_empty() {
                            match &self.search {
                                Some((p, _)) => p.clone(),
                                None => return Outcome::err("E35: No previous regular expression"),
                            }
                        } else {
                            rest.to_string()
                        };
                        self.search = Some((pat.clone(), fwd));
                        self.highlight = true;
                        let cur = self.cursor(doc);
                        match self.search_from(doc, &pat, cur, fwd) {
                            Ok((p, wrapped)) => {
                                self.put(doc, p);
                                if wrapped {
                                    Outcome::msg(if fwd {
                                        "search hit BOTTOM, continuing at TOP"
                                    } else {
                                        "search hit TOP, continuing at BOTTOM"
                                    })
                                } else {
                                    Outcome::handled()
                                }
                            }
                            Err(e) => Outcome::err(e),
                        }
                    }
                }
            }
            _ => Outcome::handled(),
        }
    }

    /// Runs a command line (without the `:`).
    pub fn ex(&mut self, doc: &mut Document, line: &str, view: View) -> Outcome {
        let _ = view;
        let r = rope(doc);
        let cur_line = line_of(r, doc.selection().primary().head);
        let last = last_line(r);
        let mut s = line.trim_start();
        // The range.
        let mut range: Option<(usize, usize)> = None;
        let addr = |t: &str| -> Option<(usize, usize)> {
            // (line, chars used)
            let c = t.chars().next()?;
            match c {
                '.' => Some((cur_line, 1)),
                '$' => Some((last, 1)),
                d if d.is_ascii_digit() => {
                    let n: String = t.chars().take_while(char::is_ascii_digit).collect();
                    Some((
                        n.parse::<usize>().ok()?.saturating_sub(1).min(last),
                        n.len(),
                    ))
                }
                _ => None,
            }
        };
        if let Some(rest) = s.strip_prefix('%') {
            range = Some((0, last));
            s = rest;
        } else if let Some(rest) = s.strip_prefix("'<,'>") {
            range = self.last_visual.or(Some((cur_line, cur_line)));
            s = rest;
        } else if let Some((a, n)) = addr(s) {
            s = &s[n..];
            if let Some(rest) = s.strip_prefix(',') {
                if let Some((b, m)) = addr(rest) {
                    range = Some((a.min(b), a.max(b)));
                    s = &rest[m..];
                } else {
                    range = Some((a, a));
                    s = rest;
                }
            } else {
                range = Some((a, a));
            }
        }
        let s = s.trim_start();
        let name: String = s.chars().take_while(char::is_ascii_alphabetic).collect();
        let after = &s[name.len()..];
        let (bang, args) = match after.strip_prefix('!') {
            Some(a) => (true, a.trim()),
            None => (false, after.trim()),
        };
        let path = (!args.is_empty()).then(|| args.to_string());
        let ex = |e: Ex| Outcome {
            handled: true,
            ex: vec![e],
            ..Outcome::default()
        };
        match name.as_str() {
            "" => {
                if let Some((_, b)) = range {
                    let p = first_nonblank(rope(doc), b);
                    doc.set_caret(&Selection::point(p));
                    Outcome::handled()
                } else if s.starts_with(['s', '/', '&']) || s.is_empty() {
                    Outcome::handled()
                } else {
                    Outcome::err(format!("E492: Not an editor command: {line}"))
                }
            }
            "w" | "write" => ex(Ex::Write { path, force: bang }),
            "q" | "quit" | "clo" | "close" => ex(Ex::Quit { force: bang }),
            "wq" | "x" | "xit" | "exit" => ex(Ex::WriteQuit { force: bang }),
            "e" | "edit" => ex(Ex::Edit { path, force: bang }),
            "noh" | "nohlsearch" => {
                self.highlight = false;
                Outcome::handled()
            }
            "s" | "substitute" => {
                let (a, b) = range.unwrap_or((cur_line, cur_line));
                self.substitute(doc, a, b, after)
            }
            "set" | "se" => self.set(doc, args),
            _ if name.starts_with('s')
                && name.chars().skip(1).all(|c| "gin".contains(c))
                && after.is_empty() =>
            {
                // `:sg`-like shorthands are not Jarak's; say so.
                Outcome::err(format!("E492: Not an editor command: {line}"))
            }
            _ => Outcome::err(format!("E492: Not an editor command: {line}")),
        }
    }

    fn set(&mut self, doc: &mut Document, args: &str) -> Outcome {
        let mut out = Outcome::handled();
        for a in args.split_whitespace() {
            let (k, v) = match a.split_once('=') {
                Some((k, v)) => (k, Some(v)),
                None => (a, None),
            };
            match (k, v) {
                ("wrap", None) => out.ex.push(Ex::Wrap(true)),
                ("nowrap", None) => out.ex.push(Ex::Wrap(false)),
                ("ic" | "ignorecase", None) => self.ignorecase = true,
                ("noic" | "noignorecase", None) => self.ignorecase = false,
                ("scs" | "smartcase", None) => self.smartcase = true,
                ("noscs" | "nosmartcase", None) => self.smartcase = false,
                ("hls" | "hlsearch", None) => self.highlight = true,
                ("nohls" | "nohlsearch", None) => self.highlight = false,
                ("ts" | "tabstop", Some(v)) => match v.parse::<usize>() {
                    Ok(n) if (1..=32).contains(&n) => doc.tab_width = n,
                    _ => return Outcome::err(format!("E487: Argument must be 1 to 32: {a}")),
                },
                _ => return Outcome::err(format!("E518: Unknown option: {a}")),
            }
        }
        out
    }

    /// `:[range]s/pat/rep/[flags]` on lines `a..=b`.
    fn substitute(&mut self, doc: &mut Document, a: usize, b: usize, spec: &str) -> Outcome {
        let mut it = spec.chars();
        let Some(delim) = it.next() else {
            return Outcome::err("E35: No previous regular expression");
        };
        if delim.is_alphanumeric() || delim == '\\' || delim == '"' || delim == '|' {
            return Outcome::err("E146: Regular expressions can't be delimited by letters");
        }
        // Split at unescaped delimiters.
        let mut fields = vec![String::new()];
        let mut esc = false;
        for c in it {
            if esc {
                if c != delim {
                    fields.last_mut().unwrap().push('\\');
                }
                fields.last_mut().unwrap().push(c);
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == delim && fields.len() < 3 {
                fields.push(String::new());
            } else {
                fields.last_mut().unwrap().push(c);
            }
        }
        if esc {
            fields.last_mut().unwrap().push('\\');
        }
        let pat = if fields[0].is_empty() {
            match &self.search {
                Some((p, _)) => p.clone(),
                None => return Outcome::err("E35: No previous regular expression"),
            }
        } else {
            fields[0].clone()
        };
        let rep = fields.get(1).cloned().unwrap_or_default();
        let flags = fields.get(2).cloned().unwrap_or_default();
        let flags = flags.trim();
        if flags.contains('c') {
            return Outcome::err("Jarak: the c (confirm) flag is not supported");
        }
        if let Some(bad) = flags.chars().find(|c| !"giIn&".contains(*c)) {
            return Outcome::err(format!("E488: Trailing characters: {bad}"));
        }
        let all = flags.contains('g');
        let count_only = flags.contains('n');
        let saved = (self.ignorecase, self.smartcase);
        if flags.contains('i') {
            self.ignorecase = true;
            self.smartcase = false;
        }
        if flags.contains('I') {
            self.ignorecase = false;
        }
        let re = self.regex(&pat);
        (self.ignorecase, self.smartcase) = saved;
        let re = match re {
            Ok(re) => re,
            Err(e) => return Outcome::err(e),
        };
        self.search = Some((pat.clone(), true));
        self.highlight = true;
        let rep = vim_replacement(&rep, doc.eol.as_str());
        let r = rope(doc);
        let mut parts = Vec::new();
        let (mut subs, mut lines, mut last) = (0, 0, None);
        for l in a..=b.min(last_line(r)) {
            let s = line_start(r, l);
            let e = line_end(r, l);
            let t = r.slice(s..e).to_string();
            let n = if all {
                re.find_iter(&t).count()
            } else {
                usize::from(re.is_match(&t))
            };
            if n == 0 {
                continue;
            }
            subs += n;
            lines += 1;
            last = Some(l);
            if !count_only {
                let new = if all {
                    re.replace_all(&t, rep.as_str())
                } else {
                    re.replace(&t, rep.as_str())
                };
                parts.push((s, e, new.into_owned()));
            }
        }
        if subs == 0 {
            return Outcome::err(format!("E486: Pattern not found: {pat}"));
        }
        let unit = |n: usize, one: &str, many: &str| {
            if n == 1 {
                format!("1 {one}")
            } else {
                format!("{n} {many}")
            }
        };
        if count_only {
            return Outcome::msg(format!(
                "{} on {}",
                unit(subs, "match", "matches"),
                unit(lines, "line", "lines")
            ));
        }
        doc.begin_group();
        doc.replace(parts, None);
        doc.end_group();
        if let Some(l) = last {
            let p = first_nonblank(rope(doc), l);
            doc.set_caret(&Selection::point(p));
        }
        self.last_change = None;
        Outcome::msg(format!(
            "{} on {}",
            unit(subs, "substitution", "substitutions"),
            unit(lines, "line", "lines")
        ))
    }
}

/// The end of the word run at `p` (for `cw` inside a word).
fn word_end_in_run(r: &Rope, p: usize, big: bool) -> usize {
    let len = r.len_chars();
    let c0 = class(r.char(p), big);
    let mut j = p;
    while j + 1 < len && class(r.char(j + 1), big) == c0 && !is_eol(r.char(j + 1)) {
        j += 1;
    }
    j
}

#[cfg(test)]
mod tests;
