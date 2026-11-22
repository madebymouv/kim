//! Find and replace in a document (`JARAK-FR-070`, `JARAK-FR-071`, Step 94).
//!
//! Three modes, as in Notepad++:
//!
//! - **Normal**: the text as it is.
//! - **Extended**: the text with `\n \r \t \0 \\ \xNN \uNNNN` escapes.
//! - **Regular expression**: the `regex` crate's syntax (Rust regular expressions). Differences
//!   from Notepad++'s Boost syntax: no look-ahead or look-behind (`(?=…)`, `(?<=…)`), no back
//!   references inside the pattern (`\1`), no atomic groups or possessive quantifiers, no `\<`
//!   `\>` (use `\b`); `^` and `$` match at line ends (multi-line), `.` matches a line break only
//!   with ". matches newline". In the replacement, `$1`, `${name}`, `\1`…`\9`, `$0`/`\0` (the
//!   match) and `\n \r \t` work.
//!
//! Options: match case, whole word (the match neither starts nor ends inside a word), wrap around,
//! backward, in selection. Positions are characters of the rope; the search runs on a copy of the
//! text made once per document state ([`Haystack`]), so typing in the find box does not copy the
//! document again, and long scans check a cancel function.

use crate::document::Document;
use ropey::Rope;
use std::sync::Arc;

/// The search mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Extended,
    Regex,
}

/// What to look for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub text: String,
    pub mode: Mode,
    pub match_case: bool,
    pub whole_word: bool,
    /// `.` matches a line break (regular expressions).
    pub dot_all: bool,
}

/// A compiled query.
#[derive(Debug, Clone)]
pub struct Matcher {
    re: regex::Regex,
    whole_word: bool,
    mode: Mode,
}

/// The document's text as one string, for one history state (made once, shared).
#[derive(Debug, Clone)]
pub struct Haystack {
    pub state: u64,
    pub text: Arc<String>,
    rope: Rope,
}

impl Haystack {
    /// The text of `doc` (cheap to clone; the rope is shared).
    pub fn of(doc: &Document) -> Haystack {
        Haystack {
            state: doc.history().state(),
            text: Arc::new(doc.text().to_string()),
            rope: doc.text().clone(),
        }
    }

    /// The haystack for `doc`, reusing `cached` when the document did not change.
    pub fn reuse(cached: Option<&Haystack>, doc: &Document) -> Haystack {
        match cached {
            Some(h)
                if h.state == doc.history().state() && h.rope.len_chars() == doc.len_chars() =>
            {
                h.clone()
            }
            _ => Haystack::of(doc),
        }
    }

    pub fn char_to_byte(&self, c: usize) -> usize {
        self.rope.char_to_byte(c.min(self.rope.len_chars()))
    }

    pub fn byte_to_char(&self, b: usize) -> usize {
        self.rope.byte_to_char(b.min(self.text.len()))
    }
}

/// `\n \r \t \0 \\ \xNN \uNNNN` in `s` (Extended mode); an unknown escape stays as it is.
pub fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some(k @ ('x' | 'u')) => {
                let n = if k == 'x' { 2 } else { 4 };
                let hex: String = (0..n).filter_map(|_| it.next()).collect();
                let v = u32::from_str_radix(&hex, 16)
                    .ok()
                    .filter(|_| hex.len() == n)
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("\\{k}{hex}: not a character code"))?;
                out.push(v);
            }
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    Ok(out)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Query {
    /// Compiles the query (an empty one or a bad pattern is an error to show).
    pub fn compile(&self) -> Result<Matcher, String> {
        if self.text.is_empty() {
            return Err("nothing to find".into());
        }
        let body = match self.mode {
            Mode::Normal => regex::escape(&self.text),
            Mode::Extended => regex::escape(&unescape(&self.text)?),
            Mode::Regex => self.text.clone(),
        };
        let mut b = regex::RegexBuilder::new(&body);
        b.case_insensitive(!self.match_case)
            .multi_line(true)
            // `^` and `$` at CRLF line ends too, as in Notepad++ (and `.` never matches a
            // carriage return).
            .crlf(true)
            .dot_matches_new_line(self.mode == Mode::Regex && self.dot_all)
            .size_limit(64 << 20);
        let re = b.build().map_err(|e| e.to_string())?;
        Ok(Matcher {
            re,
            whole_word: self.whole_word,
            mode: self.mode,
        })
    }
}

impl Matcher {
    /// The match must not start or end inside a word (whole word).
    fn word_ok(&self, h: &str, a: usize, b: usize) -> bool {
        if !self.whole_word {
            return true;
        }
        let before = h[..a].chars().next_back();
        let after = h[b..].chars().next();
        let first = h[a..b].chars().next();
        let last = h[a..b].chars().next_back();
        !(before.is_some_and(is_word) && first.is_some_and(is_word))
            && !(after.is_some_and(is_word) && last.is_some_and(is_word))
    }

    /// The first match starting at or after byte `from` and ending by byte `to` (bytes).
    fn next_bytes(&self, h: &str, from: usize, to: usize) -> Option<(usize, usize)> {
        let mut at = from;
        while at <= to.min(h.len()) {
            let m = self.re.find_at(&h[..to.min(h.len())], at)?;
            if self.word_ok(h, m.start(), m.end()) {
                return Some((m.start(), m.end()));
            }
            // Past this start, one character on.
            at = m.start() + h[m.start()..].chars().next().map_or(1, char::len_utf8);
        }
        None
    }

    /// Every match in `range` (chars), stopping when `cancel` says so (checked every 4096
    /// matches). Returns the matches and whether the scan finished.
    pub fn find_all(
        &self,
        h: &Haystack,
        range: Option<(usize, usize)>,
        cancel: &dyn Fn() -> bool,
    ) -> (Vec<(usize, usize)>, bool) {
        let (fa, fb) = range.map_or((0, h.text.len()), |(a, b)| {
            (h.char_to_byte(a), h.char_to_byte(b))
        });
        let mut out = Vec::new();
        let mut at = fa;
        while let Some((a, b)) = self.next_bytes(&h.text, at, fb) {
            out.push((a, b));
            at = if b > a {
                b
            } else {
                b + h.text[b..].chars().next().map_or(1, char::len_utf8)
            };
            if out.len() % 4096 == 0 && cancel() {
                return (to_chars(h, out), false);
            }
            if at > fb {
                break;
            }
        }
        (to_chars(h, out), true)
    }

    /// The next match from char `from` (forward: starting after it; backward: ending before
    /// it), wrapping around when `wrap`; inside `range` when given. Returns the match and whether
    /// it wrapped.
    pub fn find(
        &self,
        h: &Haystack,
        from: usize,
        backward: bool,
        wrap: bool,
        range: Option<(usize, usize)>,
    ) -> Option<((usize, usize), bool)> {
        let (fa, fb) = range.map_or((0, h.text.len()), |(a, b)| {
            (h.char_to_byte(a), h.char_to_byte(b))
        });
        let pos = h.char_to_byte(from).clamp(fa, fb);
        let ch = |a: usize, b: usize| (h.byte_to_char(a), h.byte_to_char(b));
        if !backward {
            if let Some((a, b)) = self.next_bytes(&h.text, pos, fb) {
                return Some((ch(a, b), false));
            }
            // Wrapping: the first match of the whole range, when it starts before the caret
            // (it may end after it).
            if wrap
                && let Some((a, b)) = self.next_bytes(&h.text, fa, fb)
                && a < pos
            {
                return Some((ch(a, b), true));
            }
            return None;
        }
        // Backward: the last match ending at or before `pos`, else (wrapping) the last one.
        let mut last_before = None;
        let mut last = None;
        let mut at = fa;
        while let Some((a, b)) = self.next_bytes(&h.text, at, fb) {
            if b <= pos && a < pos {
                last_before = Some((a, b));
            }
            last = Some((a, b));
            at = if b > a {
                b
            } else {
                b + h.text[b..].chars().next().map_or(1, char::len_utf8)
            };
            if at > fb {
                break;
            }
        }
        if let Some((a, b)) = last_before {
            return Some((ch(a, b), false));
        }
        if wrap && let Some((a, b)) = last {
            return Some((ch(a, b), true));
        }
        None
    }

    /// The replacement text of the match `m` (chars) for `with` (expanded in regex mode,
    /// unescaped in extended mode, as it is in normal mode).
    pub fn replacement(&self, h: &Haystack, m: (usize, usize), with: &str) -> String {
        match self.mode {
            Mode::Normal => with.to_string(),
            Mode::Extended => unescape(with).unwrap_or_else(|_| with.to_string()),
            Mode::Regex => {
                let (a, b) = (h.char_to_byte(m.0), h.char_to_byte(m.1));
                let hay = &h.text[..b];
                let Some(caps) = self
                    .re
                    .captures_at(hay, a)
                    .filter(|c| c.get(0).is_some_and(|g| g.start() == a))
                else {
                    return with.to_string();
                };
                let mut out = String::new();
                caps.expand(&regex_replacement(with), &mut out);
                out
            }
        }
    }
}

fn to_chars(h: &Haystack, v: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    // Byte offsets in order: convert with a running rope walk (cheap per call).
    v.into_iter()
        .map(|(a, b)| (h.byte_to_char(a), h.byte_to_char(b)))
        .collect()
}

/// A Notepad++-style replacement (`\1`, `\0`, `\n`, `\t`, `\r`) in the regex crate's syntax.
pub fn regex_replacement(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some(d @ '0'..='9') => out.push_str(&format!("${{{d}}}")),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('$') => out.push_str("$$"),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// More matches than this are replaced by rewriting the whole text in one edit (fast, one undo
/// step; fewer are replaced one by one, so undo restores the carets precisely).
pub const BULK: usize = 20_000;

/// Replaces every match of `m` in `range` (or the whole document) with `with`, as one undo step.
/// Returns the number of replacements.
pub fn replace_all(
    doc: &mut Document,
    m: &Matcher,
    h: &Haystack,
    with: &str,
    range: Option<(usize, usize)>,
    cancel: &dyn Fn() -> bool,
) -> Result<usize, String> {
    let (found, done) = m.find_all(h, range, cancel);
    if !done {
        return Err("stopped before the end: nothing was replaced".into());
    }
    let n = found.len();
    if n == 0 {
        return Ok(0);
    }
    let caret = doc.selection().primary().head;
    if n > BULK {
        // One edit for the whole text.
        let mut out = String::with_capacity(h.text.len());
        let mut last = 0;
        for (a, b) in &found {
            let (ba, bb) = (h.char_to_byte(*a), h.char_to_byte(*b));
            out.push_str(&h.text[last..ba]);
            out.push_str(&m.replacement(h, (*a, *b), with));
            last = bb;
        }
        out.push_str(&h.text[last..]);
        let len = doc.len_chars();
        doc.replace(vec![(0, len, out)], Some(caret));
    } else {
        let parts: Vec<(usize, usize, String)> = found
            .iter()
            .map(|(a, b)| (*a, *b, m.replacement(h, (*a, *b), with)))
            .collect();
        doc.replace(parts, Some(caret));
    }
    Ok(n)
}

#[cfg(test)]
mod tests;
