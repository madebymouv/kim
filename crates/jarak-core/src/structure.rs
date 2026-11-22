//! Document structure (`JARAK-FR-092`, `JARAK-FR-093`, `JARAK-FR-096`, Step 99): fold ranges by
//! syntax (brackets outside strings and comments, tags, multi-line block comments), by indentation
//! (Python, YAML, Lua, Ruby, Shell, …), by sections (INI, TOML), by Markdown headings and fenced
//! code, and by a user language's markers; the symbols of the Function List (functions, types,
//! headings, sections …, found per language from the tokens of Step 98); and bracket matching.
//!
//! Everything here reads the lines and the lexer; nothing changes the text.

use crate::syntax::{self, Kind, Lang, State, Style, Token};

/// A fold: the first line (stays visible) and the last line (hidden with the rest when folded),
/// with its nesting level (1 = outermost).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fold {
    pub start: usize,
    pub end: usize,
    pub level: usize,
}

/// A symbol of the Function List.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// `fn`, `class`, `heading 2`, `section`, …
    pub kind: String,
    pub line: usize,
    /// Nesting (Markdown heading level − 1, or 0).
    pub depth: usize,
}

/// How a language folds.
fn fold_style(lang: &Lang) -> &'static str {
    match lang.name.as_str() {
        "Python" | "YAML" | "Lua" | "Ruby" | "Shell" | "Batch" | "SQL" => "indent",
        "INI" | "TOML" => "sections",
        _ if lang.style == Style::Markdown => "markdown",
        _ if lang.style == Style::Markup => "tags",
        _ if !lang.fold.is_empty() => "markers",
        _ => "brackets",
    }
}

/// Every line's tokens (the state carried).
pub fn tokens_of(lang: &Lang, lines: &[String]) -> Vec<Vec<Token>> {
    let mut st = State::Normal;
    lines
        .iter()
        .map(|l| {
            let (t, next) = syntax::lex_line(lang, l, st.clone());
            st = next;
            t
        })
        .collect()
}

fn in_kinds(toks: &[Token], c: usize, kinds: &[Kind]) -> bool {
    toks.iter()
        .any(|t| t.start <= c && c < t.end && kinds.contains(&t.kind))
}

/// The folds and (with `with_symbols`) the symbols of `lines` in `lang`, lexing the text once.
pub fn outline(
    lang: Option<&Lang>,
    lines: &[String],
    with_symbols: bool,
) -> (Vec<Fold>, Vec<Symbol>) {
    let Some(l) = lang else {
        return (Vec::new(), Vec::new());
    };
    let toks = if l.style == Style::Markdown {
        vec![Vec::new(); lines.len()]
    } else {
        tokens_of(l, lines)
    };
    let f = folds_with(l, lines, &toks);
    let s = if with_symbols {
        symbols_with(l, lines, &toks)
    } else {
        Vec::new()
    };
    (f, s)
}

/// The fold ranges of `lines` in `lang` (none for plain text), each with its level.
pub fn folds(lang: Option<&Lang>, lines: &[String]) -> Vec<Fold> {
    outline(lang, lines, false).0
}

fn folds_with(lang: &Lang, lines: &[String], toks: &[Vec<Token>]) -> Vec<Fold> {
    let mut ranges: Vec<(usize, usize)> = match fold_style(lang) {
        "indent" => indent_folds(lines),
        "sections" => section_folds(lines),
        "markdown" => markdown_folds(lines),
        "tags" => tag_folds(lines, toks),
        "markers" => syntax::fold_ranges(lang, lines),
        _ => bracket_folds(lines, toks),
    };
    // Multi-line block comments fold too (code languages).
    if lang.style == Style::Code && !lang.block_comment.is_empty() {
        ranges.extend(comment_folds(lang, lines, toks));
    }
    ranges.retain(|(a, b)| b > a);
    ranges.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)));
    ranges.dedup_by_key(|r| r.0);
    // Levels: how many folds contain each one.
    let mut out: Vec<Fold> = Vec::with_capacity(ranges.len());
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for (a, b) in ranges {
        while stack.last().is_some_and(|(_, e)| *e < a) {
            stack.pop();
        }
        let level = stack.len() + 1;
        stack.push((a, b));
        out.push(Fold {
            start: a,
            end: b,
            level,
        });
    }
    out
}

fn bracket_folds(lines: &[String], toks: &[Vec<Token>]) -> Vec<(usize, usize)> {
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut out = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if !matches!(ch, '{' | '}' | '[' | ']')
                || in_kinds(
                    &toks[n],
                    c,
                    &[Kind::String, Kind::Comment, Kind::Preprocessor],
                )
            {
                continue;
            }
            match ch {
                '{' | '[' => stack.push((ch, n)),
                _ => {
                    let open = if ch == '}' { '{' } else { '[' };
                    if let Some(p) = stack.iter().rposition(|(o, _)| *o == open) {
                        let (_, from) = stack.remove(p);
                        if n > from {
                            // As in Notepad++, the fold hides everything through the closing line.
                            out.push((from, n));
                        }
                    }
                }
            }
        }
    }
    out
}

/// A block comment that starts a line and ends on a later one folds (from the line it opens on
/// through the line it closes on).
fn comment_folds(lang: &Lang, lines: &[String], toks: &[Vec<Token>]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (n, line) in lines.iter().enumerate() {
        let t = line.trim();
        match start {
            None => {
                let lead = line.chars().count() - line.trim_start().chars().count();
                let opens = lang
                    .block_comment
                    .iter()
                    .find(|d| t.starts_with(d.open.as_str()));
                if let Some(d) = opens
                    && in_kinds(&toks[n], lead, &[Kind::Comment])
                    && !t[d.open.len()..].contains(d.close.as_str())
                {
                    start = Some(n);
                }
            }
            Some(s) => {
                if lang
                    .block_comment
                    .iter()
                    .any(|d| t.contains(d.close.as_str()))
                {
                    out.push((s, n));
                    start = None;
                }
            }
        }
    }
    out
}

fn indent_of(l: &str) -> Option<usize> {
    if l.trim().is_empty() {
        return None;
    }
    Some(
        l.chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum(),
    )
}

/// A line folds the following lines that are indented deeper (blank lines inside count).
fn indent_folds(lines: &[String]) -> Vec<(usize, usize)> {
    let ind: Vec<Option<usize>> = lines.iter().map(|l| indent_of(l)).collect();
    let mut out = Vec::new();
    for n in 0..lines.len() {
        let Some(me) = ind[n] else {
            continue;
        };
        let mut last = n;
        for (k, i) in ind.iter().enumerate().skip(n + 1) {
            match i {
                Some(x) if *x > me => last = k,
                Some(_) => break,
                None => {}
            }
        }
        if last > n {
            out.push((n, last));
        }
    }
    out
}

fn section_folds(lines: &[String]) -> Vec<(usize, usize)> {
    let heads: Vec<usize> = (0..lines.len())
        .filter(|n| lines[*n].trim_start().starts_with('['))
        .collect();
    let mut out = Vec::new();
    for (k, &h) in heads.iter().enumerate() {
        let mut end = heads.get(k + 1).map_or(lines.len(), |x| *x) - 1;
        while end > h && lines[end].trim().is_empty() {
            end -= 1;
        }
        if end > h {
            out.push((h, end));
        }
    }
    out
}

fn heading_level(l: &str) -> Option<usize> {
    let t = l.trim_start();
    let n = t.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&n) && t[n..].starts_with([' ', '\t'])
        || (1..=6).contains(&n) && t.len() == n)
        .then_some(n)
}

/// Markdown: a heading folds until the next heading of its level or above; fenced code folds.
fn markdown_folds(lines: &[String]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut heads: Vec<(usize, usize)> = Vec::new();
    let mut fence: Option<(usize, String)> = None;
    for (n, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if let Some((s, f)) = &fence {
            if t.starts_with(f.as_str()) {
                out.push((*s, n));
                fence = None;
            }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some((n, t[..3].to_string()));
            continue;
        }
        if let Some(lv) = heading_level(l) {
            heads.push((n, lv));
        }
    }
    for (k, &(h, lv)) in heads.iter().enumerate() {
        let next = heads[k + 1..]
            .iter()
            .find(|(_, l)| *l <= lv)
            .map_or(lines.len(), |(n, _)| *n);
        let mut end = next - 1;
        while end > h && lines[end].trim().is_empty() {
            end -= 1;
        }
        if end > h {
            out.push((h, end));
        }
    }
    out
}

/// HTML / XML: an element opened on one line and closed on a later one (void elements and
/// self-closing tags do not fold); multi-line comments.
fn tag_folds(lines: &[String], toks: &[Vec<Token>]) -> Vec<(usize, usize)> {
    const VOID: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr", "!doctype",
    ];
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut out = Vec::new();
    let mut comment: Option<usize> = None;
    for (n, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        if line.contains("<!--") && !line.contains("-->") {
            comment = Some(n);
        } else if let Some(s) = comment
            && line.contains("-->")
        {
            out.push((s, n));
            comment = None;
        }
        for t in &toks[n] {
            if t.kind != Kind::Tag {
                continue;
            }
            let text: String = chars[t.start..t.end].iter().collect();
            if let Some(name) = text.strip_prefix("</") {
                let name = name.to_lowercase();
                if let Some(p) = stack.iter().rposition(|(x, _)| *x == name) {
                    let (_, from) = stack.remove(p);
                    stack.truncate(p);
                    if n > from {
                        out.push((from, n));
                    }
                }
            } else if let Some(name) = text.strip_prefix('<') {
                let name = name.to_lowercase();
                if !VOID.contains(&name.as_str()) && !name.is_empty() {
                    stack.push((name, n));
                }
            } else if text == "/>"
                && let Some(_) = stack.pop()
            {
                // A self-closing tag closes the element it opened.
            }
        }
    }
    out
}

// ---- symbols ------------------------------------------------------------------------------------------

fn ident_after(s: &str) -> Option<String> {
    let t = s.trim_start();
    let n: String = t
        .chars()
        .take_while(|c| {
            c.is_alphanumeric() || *c == '_' || *c == '$' || *c == '.' || *c == ':' || *c == '-'
        })
        .collect();
    let n = n.trim_end_matches([':', '.']).to_string();
    (!n.is_empty()).then_some(n)
}

/// The symbols of `lines` for the Function List: per language, the definitions its keywords
/// introduce (`fn`, `def`, `class`, `function`, `func`, `struct`, `enum`, `trait`, `interface`,
/// `impl`, `mod`, `type`, `module`), C-like function definitions (a name, its parameters and a
/// `{`), Markdown headings, INI/TOML sections, JSON/YAML top-level keys, Batch labels, CSS rules,
/// SQL `create` statements. Lines in comments and strings are skipped.
pub fn symbols(lang: Option<&Lang>, lines: &[String]) -> Vec<Symbol> {
    outline(lang, lines, true).1
}

fn symbols_with(lang: &Lang, lines: &[String], toks: &[Vec<Token>]) -> Vec<Symbol> {
    let mut out = Vec::new();
    match lang.style {
        Style::Markdown => {
            let mut fence = false;
            for (n, l) in lines.iter().enumerate() {
                let t = l.trim_start();
                if t.starts_with("```") || t.starts_with("~~~") {
                    fence = !fence;
                    continue;
                }
                if !fence && let Some(lv) = heading_level(l) {
                    out.push(Symbol {
                        name: t[lv..].trim().trim_end_matches('#').trim().to_string(),
                        kind: format!("heading {lv}"),
                        line: n,
                        depth: lv - 1,
                    });
                }
            }
            return out;
        }
        Style::Markup => {
            for (n, l) in lines.iter().enumerate() {
                let lower = l.to_lowercase();
                for lv in 1..=6 {
                    if let Some(p) = lower.find(&format!("<h{lv}")) {
                        let after = &l[p..];
                        let text: String = after
                            .split('>')
                            .nth(1)
                            .unwrap_or("")
                            .split('<')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        out.push(Symbol {
                            name: text,
                            kind: format!("heading {lv}"),
                            line: n,
                            depth: lv - 1,
                        });
                    }
                }
                if lang.name == "PHP"
                    && let Some(p) = lower.find("function ")
                    && let Some(name) = ident_after(&l[p + 9..])
                {
                    out.push(Symbol {
                        name,
                        kind: "function".into(),
                        line: n,
                        depth: 0,
                    });
                }
            }
            return out;
        }
        Style::Code => {}
    }
    let defs: &[&str] = &[
        "fn",
        "def",
        "class",
        "function",
        "func",
        "struct",
        "enum",
        "trait",
        "interface",
        "impl",
        "mod",
        "module",
        "type",
        "record",
        "namespace",
        "union",
        "procedure",
    ];
    // JSON: the bracket depth at each line's start (the keys of the outermost object are listed).
    let mut depth = 0usize;
    for (n, l) in lines.iter().enumerate() {
        let start_depth = depth;
        if lang.name == "JSON" {
            for (c, ch) in l.chars().enumerate() {
                if in_kinds(&toks[n], c, &[Kind::String, Kind::Attribute]) {
                    continue;
                }
                match ch {
                    '{' | '[' => depth += 1,
                    '}' | ']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
        }
        let t = l.trim_start();
        if t.is_empty() {
            continue;
        }
        let lead = l.chars().count() - t.chars().count();
        if in_kinds(&toks[n], lead, &[Kind::Comment, Kind::String]) {
            continue;
        }
        match lang.name.as_str() {
            "INI" | "TOML" => {
                if t.starts_with('[') {
                    out.push(Symbol {
                        name: t.trim_end().trim_matches(['[', ']']).to_string(),
                        kind: "section".into(),
                        line: n,
                        depth: 0,
                    });
                }
                continue;
            }
            "JSON" | "YAML" => {
                let top = if lang.name == "YAML" {
                    lead == 0
                } else {
                    start_depth == 1
                };
                if top && let Some(k) = toks[n].iter().find(|x| x.kind == Kind::Attribute) {
                    let name: String = l.chars().skip(k.start).take(k.end - k.start).collect();
                    out.push(Symbol {
                        name: name.trim_matches('"').to_string(),
                        kind: "key".into(),
                        line: n,
                        depth: 0,
                    });
                }
                continue;
            }
            "Batch" => {
                if let Some(label) = t.strip_prefix(':').filter(|x| !x.starts_with(':')) {
                    out.push(Symbol {
                        name: label.trim().to_string(),
                        kind: "label".into(),
                        line: n,
                        depth: 0,
                    });
                }
                continue;
            }
            "CSS" => {
                if t.trim_end().ends_with('{') && !t.starts_with('}') {
                    out.push(Symbol {
                        name: t.trim_end().trim_end_matches('{').trim().to_string(),
                        kind: "rule".into(),
                        line: n,
                        depth: usize::from(lead > 0),
                    });
                }
                continue;
            }
            "SQL" => {
                let lower = t.to_lowercase();
                if let Some(rest) = lower.strip_prefix("create ") {
                    let rest = rest.trim_start_matches("or replace ");
                    let kind: String = rest.split_whitespace().next().unwrap_or("").to_string();
                    let name = t
                        .split_whitespace()
                        .nth(2 + usize::from(lower.contains("or replace")) * 2)
                        .unwrap_or("")
                        .trim_matches(['(', '`', '"', ';'])
                        .to_string();
                    out.push(Symbol {
                        name,
                        kind,
                        line: n,
                        depth: 0,
                    });
                }
                continue;
            }
            _ => {}
        }
        // Keyword-introduced definitions (after modifiers like `pub`, `async`, `export`).
        let words: Vec<&str> = t
            .split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == '{' || c == ':')
            .filter(|w| !w.is_empty())
            .collect();
        let modifiers = [
            "pub",
            "pub(crate)",
            "async",
            "export",
            "default",
            "public",
            "private",
            "protected",
            "internal",
            "static",
            "abstract",
            "final",
            "sealed",
            "unsafe",
            "extern",
            "const",
            "local",
            "partial",
            "override",
            "virtual",
            "inline",
        ];
        let mut i = 0;
        while i < words.len() && modifiers.contains(&words[i]) {
            i += 1;
        }
        if let Some(&w) = words.get(i) {
            let is_def = if lang.case_sensitive {
                defs.contains(&w)
            } else {
                defs.iter().any(|d| d.eq_ignore_ascii_case(w))
            };
            // `type` is a definition in Go and TypeScript only; `class` in Python, Ruby, JS, C-like.
            if is_def
                && !(w == "type" && !matches!(lang.name.as_str(), "Go" | "TypeScript" | "Rust"))
            {
                let rest = t.splitn(2 + i, char::is_whitespace).last().unwrap_or("");
                let rest = if i > 0 {
                    t[t.find(w).unwrap_or(0) + w.len()..].trim_start()
                } else {
                    rest
                };
                if let Some(name) = ident_after(rest).filter(|x| !defs.contains(&x.as_str())) {
                    out.push(Symbol {
                        name,
                        kind: w.to_lowercase(),
                        line: n,
                        depth: usize::from(lead > 0),
                    });
                    continue;
                }
            }
        }
        // `name = function`, `const name = (…) =>` (JavaScript, TypeScript, Lua).
        if matches!(lang.name.as_str(), "JavaScript" | "TypeScript" | "Lua")
            && (t.contains("=> {") || t.contains("= function") || t.contains("=function"))
            && let Some(eq) = t.find('=')
        {
            let left = t[..eq]
                .trim()
                .trim_start_matches("const ")
                .trim_start_matches("let ")
                .trim_start_matches("var ")
                .trim_start_matches("local ");
            if let Some(name) = ident_after(left) {
                out.push(Symbol {
                    name,
                    kind: "function".into(),
                    line: n,
                    depth: usize::from(lead > 0),
                });
                continue;
            }
        }
        // Shell `name() {`.
        if lang.name == "Shell" && t.contains("()") && t.trim_end().ends_with('{') {
            if let Some(name) = ident_after(&t[..t.find("()").unwrap_or(0)]) {
                out.push(Symbol {
                    name,
                    kind: "function".into(),
                    line: n,
                    depth: 0,
                });
            }
            continue;
        }
        // C-like function definitions: `type name(params) {` (or `{` on the next line).
        if matches!(
            lang.name.as_str(),
            "C" | "C++" | "C#" | "Java" | "Go" | "PHP code"
        ) && let Some(p) = t.find('(')
        {
            let head = t[..p].trim_end();
            let ends_open = t.trim_end().ends_with('{')
                || (t.trim_end().ends_with(')')
                    && lines
                        .get(n + 1)
                        .is_some_and(|x| x.trim_start().starts_with('{')));
            let name = head
                .rsplit(|c: char| c.is_whitespace() || c == '*' || c == '&')
                .next()
                .unwrap_or("");
            let control = [
                "if", "for", "while", "switch", "catch", "return", "else", "sizeof", "foreach",
                "using", "lock", "new",
            ];
            if ends_open
                && !name.is_empty()
                && head.contains(' ')
                && !control.contains(&name)
                && !control.iter().any(|c| head.starts_with(c))
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '~')
            {
                out.push(Symbol {
                    name: name.to_string(),
                    kind: "function".into(),
                    line: n,
                    depth: usize::from(lead > 0),
                });
            }
        }
    }
    out
}

/// The bracket that matches the one at char `pos` (or just before it) in `text`, skipping strings
/// and comments: `(from, to)` with `from` the bracket at the caret.
pub fn matching_bracket(
    lang: Option<&Lang>,
    lines: &[String],
    line: usize,
    col: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let toks = lang.map(|l| tokens_of(l, lines));
    let skip = |n: usize, c: usize| {
        toks.as_ref()
            .is_some_and(|t| in_kinds(&t[n], c, &[Kind::String, Kind::Comment]))
    };
    let chars: Vec<char> = lines.get(line)?.chars().collect();
    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];
    let at = [col, col.wrapping_sub(1)].into_iter().find(|c| {
        chars
            .get(*c)
            .is_some_and(|ch| pairs.iter().any(|(a, b)| a == ch || b == ch))
            && !skip(line, *c)
    })?;
    let ch = chars[at];
    let (open, close, forward) = pairs.iter().find_map(|(a, b)| {
        if *a == ch {
            Some((*a, *b, true))
        } else if *b == ch {
            Some((*a, *b, false))
        } else {
            None
        }
    })?;
    let mut depth = 0i32;
    let (mut n, mut c) = (line, at as isize);
    loop {
        let row: Vec<char> = lines[n].chars().collect();
        while c >= 0 && (c as usize) < row.len() {
            let x = row[c as usize];
            if !skip(n, c as usize) {
                if x == open {
                    depth += if forward { 1 } else { -1 };
                } else if x == close {
                    depth += if forward { -1 } else { 1 };
                }
                if depth == 0 {
                    return Some(((line, at), (n, c as usize)));
                }
            }
            c += if forward { 1 } else { -1 };
        }
        if forward {
            n += 1;
            if n >= lines.len() {
                return None;
            }
            c = 0;
        } else {
            if n == 0 {
                return None;
            }
            n -= 1;
            c = lines[n].chars().count() as isize - 1;
        }
    }
}

#[cfg(test)]
mod tests;
