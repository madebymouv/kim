//! Editing assistance (`JARAK-FR-094`, `JARAK-FR-095`, `JARAK-FR-096`, Step 100): completion
//! candidates from the document's words, the language's keywords and Notepad++ API files, path
//! completion, parameter hints, snippets with tab stops, and bracket and tag matching bounded for
//! highlighting. No UI here; the document's typing aids are in `document::assist`.

use crate::syntax::{Kind, Lang, Token};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// What a completion candidate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandKind {
    Snippet,
    Function,
    Keyword,
    Word,
    Path,
}

impl CandKind {
    /// A short label for the popup (English, Korean).
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            CandKind::Snippet => ("snippet", "스니펫"),
            CandKind::Function => ("function", "함수"),
            CandKind::Keyword => ("keyword", "키워드"),
            CandKind::Word => ("word", "단어"),
            CandKind::Path => ("path", "경로"),
        }
    }
}

/// A completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub kind: CandKind,
    /// A signature, a description, or "folder".
    pub detail: String,
}

/// Whether `c` belongs to a word (letters of any script, digits, `_`, and `extra`).
pub fn is_word(c: char, extra: &str) -> bool {
    c.is_alphanumeric() || c == '_' || extra.contains(c)
}

/// The word ending at char column `col` of `line`: its start column and text.
pub fn word_before(line: &str, col: usize, extra: &str) -> (usize, String) {
    let chars: Vec<char> = line.chars().collect();
    let col = col.min(chars.len());
    let mut s = col;
    while s > 0 && is_word(chars[s - 1], extra) {
        s -= 1;
    }
    (s, chars[s..col].iter().collect())
}

/// The distinct words of `lines` (at least two characters, not only digits) that start with
/// `prefix` (case-insensitively), without `prefix` itself, sorted.
pub fn document_words<'a>(
    lines: impl Iterator<Item = &'a str>,
    prefix: &str,
    extra: &str,
) -> Vec<String> {
    words_in(
        lines.flat_map(|l| l.chars().chain(std::iter::once('\n'))),
        prefix,
        extra,
    )
}

/// [`document_words`] over any characters (the window passes a rope's, without a string per
/// line).
pub fn words_in(chars: impl Iterator<Item = char>, prefix: &str, extra: &str) -> Vec<String> {
    let p: Vec<char> = prefix.chars().flat_map(char::to_lowercase).collect();
    let mut set = std::collections::BTreeSet::new();
    let mut word = String::new();
    // Whether the word so far still starts with the prefix (checked as it grows).
    let mut n = 0usize;
    let mut fits = true;
    for c in chars.chain(std::iter::once(' ')) {
        if is_word(c, extra) {
            if fits && n < p.len() {
                let mut low = c.to_lowercase();
                fits = low.next() == Some(p[n]) && low.next().is_none();
            }
            n += 1;
            if fits {
                word.push(c);
            }
        } else if n > 0 {
            if fits
                && n >= p.len()
                && n >= 2
                && word != prefix
                && !word.chars().all(|c| c.is_ascii_digit())
            {
                set.insert(std::mem::take(&mut word));
            }
            word.clear();
            n = 0;
            fits = true;
        }
    }
    set.into_iter().collect()
}

// ---- Notepad++ API files (autoCompletion\*.xml) ------------------------------------------------

/// One overload of a function: its return type, description and parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overload {
    pub ret: String,
    pub descr: String,
    pub params: Vec<String>,
}

/// A keyword or function of an API file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiWord {
    pub name: String,
    pub func: bool,
    pub overloads: Vec<Overload>,
}

/// A language's API (Notepad++'s `autoCompletion\<language>.xml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Api {
    pub language: String,
    pub ignore_case: bool,
    pub start_func: char,
    pub stop_func: char,
    pub separator: char,
    pub words: Vec<ApiWord>,
}

impl Api {
    /// The word named `name`.
    pub fn get(&self, name: &str) -> Option<&ApiWord> {
        self.words.iter().find(|w| {
            if self.ignore_case {
                w.name.eq_ignore_ascii_case(name)
            } else {
                w.name == name
            }
        })
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    // ` name="…"` or ` name='…'` (a space before, so `retVal` is not found inside `xretVal`).
    for q in ['"', '\''] {
        let key = format!("{name}={q}");
        let mut from = 0;
        while let Some(p) = tag[from..].find(&key) {
            let at = from + p;
            if at == 0 || tag[..at].ends_with(char::is_whitespace) {
                let rest = &tag[at + key.len()..];
                let e = rest.find(q)?;
                return Some(crate::syntax::xml_unescape(&rest[..e]));
            }
            from = at + key.len();
        }
    }
    None
}

/// The tags `<name …>` / `<name …/>` in `xml` from `from`: each tag's text and where it ends.
fn tags<'a>(xml: &'a str, name: &str) -> Vec<(usize, &'a str)> {
    let open = format!("<{name}");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = xml[from..].find(&open) {
        let at = from + p;
        let after = xml[at + open.len()..].chars().next();
        if !after.is_some_and(|c| c.is_whitespace() || c == '>' || c == '/') {
            from = at + open.len();
            continue;
        }
        let Some(e) = xml[at..].find('>') else { break };
        out.push((at, &xml[at..at + e + 1]));
        from = at + e + 1;
    }
    out
}

/// Reads a Notepad++ API file: `<AutoComplete language="…">`, its `<Environment>` (ignoreCase,
/// startFunc, stopFunc, paramSeparator) and every `<KeyWord name="…" func="yes">` with its
/// `<Overload retVal="…" descr="…">` and `<Param name="…"/>`.
pub fn parse_npp_api(xml: &str) -> Result<Api, String> {
    let ac = tags(xml, "AutoComplete")
        .into_iter()
        .next()
        .ok_or("not a Notepad++ API file (no <AutoComplete>)")?;
    let mut api = Api {
        language: attr(ac.1, "language").unwrap_or_default(),
        ignore_case: false,
        start_func: '(',
        stop_func: ')',
        separator: ',',
        words: Vec::new(),
    };
    if let Some((_, env)) = tags(xml, "Environment").into_iter().next() {
        api.ignore_case = attr(env, "ignoreCase").is_some_and(|v| v == "yes");
        let ch = |k: &str, d: char| attr(env, k).and_then(|v| v.chars().next()).unwrap_or(d);
        api.start_func = ch("startFunc", '(');
        api.stop_func = ch("stopFunc", ')');
        api.separator = ch("paramSeparator", ',');
    }
    let kws = tags(xml, "KeyWord");
    for (k, (at, tag)) in kws.iter().enumerate() {
        let Some(name) = attr(tag, "name").filter(|n| !n.is_empty()) else {
            continue;
        };
        let func = attr(tag, "func").is_some_and(|v| v == "yes");
        // The keyword's body: up to its `</KeyWord>` (a self-closing tag has none).
        let body = if tag.ends_with("/>") {
            ""
        } else {
            let end = kws.get(k + 1).map_or(xml.len(), |n| n.0);
            let s = &xml[at + tag.len()..end];
            s.find("</KeyWord>").map_or(s, |e| &s[..e])
        };
        let mut overloads = Vec::new();
        let ovs = tags(body, "Overload");
        for (j, (oat, otag)) in ovs.iter().enumerate() {
            let end = ovs.get(j + 1).map_or(body.len(), |n| n.0);
            let ob = &body[oat + otag.len()..end];
            let params = tags(ob, "Param")
                .iter()
                .filter_map(|(_, p)| attr(p, "name"))
                .collect();
            overloads.push(Overload {
                ret: attr(otag, "retVal").unwrap_or_default(),
                descr: attr(otag, "descr").unwrap_or_default(),
                params,
            });
        }
        api.words.push(ApiWord {
            name,
            func,
            overloads,
        });
    }
    if api.words.is_empty() {
        return Err("the API file has no <KeyWord>".into());
    }
    Ok(api)
}

/// The built-in API files: a few common functions of C, Python and JavaScript, in Notepad++'s
/// format (more come from `autoCompletion\*.xml` files the user adds).
const BUILTIN_APIS: [&str; 3] = [
    r#"<NotepadPlus><AutoComplete language="C"><Environment ignoreCase="no" startFunc="(" stopFunc=")" paramSeparator="," terminal=";"/>
<KeyWord name="calloc" func="yes"><Overload retVal="void *" descr="Allocates zeroed memory"><Param name="size_t count"/><Param name="size_t size"/></Overload></KeyWord>
<KeyWord name="fclose" func="yes"><Overload retVal="int"><Param name="FILE *stream"/></Overload></KeyWord>
<KeyWord name="fopen" func="yes"><Overload retVal="FILE *" descr="Opens a file"><Param name="const char *path"/><Param name="const char *mode"/></Overload></KeyWord>
<KeyWord name="fprintf" func="yes"><Overload retVal="int"><Param name="FILE *stream"/><Param name="const char *format"/><Param name="..."/></Overload></KeyWord>
<KeyWord name="free" func="yes"><Overload retVal="void"><Param name="void *ptr"/></Overload></KeyWord>
<KeyWord name="malloc" func="yes"><Overload retVal="void *" descr="Allocates memory"><Param name="size_t size"/></Overload></KeyWord>
<KeyWord name="memcpy" func="yes"><Overload retVal="void *"><Param name="void *dest"/><Param name="const void *src"/><Param name="size_t n"/></Overload></KeyWord>
<KeyWord name="printf" func="yes"><Overload retVal="int" descr="Prints formatted output"><Param name="const char *format"/><Param name="..."/></Overload></KeyWord>
<KeyWord name="snprintf" func="yes"><Overload retVal="int"><Param name="char *buf"/><Param name="size_t size"/><Param name="const char *format"/><Param name="..."/></Overload></KeyWord>
<KeyWord name="strcmp" func="yes"><Overload retVal="int"><Param name="const char *a"/><Param name="const char *b"/></Overload></KeyWord>
<KeyWord name="strlen" func="yes"><Overload retVal="size_t"><Param name="const char *s"/></Overload></KeyWord>
</AutoComplete></NotepadPlus>"#,
    r#"<NotepadPlus><AutoComplete language="Python"><Environment ignoreCase="no" startFunc="(" stopFunc=")" paramSeparator=","/>
<KeyWord name="enumerate" func="yes"><Overload retVal="enumerate"><Param name="iterable"/><Param name="start=0"/></Overload></KeyWord>
<KeyWord name="isinstance" func="yes"><Overload retVal="bool"><Param name="obj"/><Param name="classinfo"/></Overload></KeyWord>
<KeyWord name="len" func="yes"><Overload retVal="int" descr="The number of items"><Param name="obj"/></Overload></KeyWord>
<KeyWord name="open" func="yes"><Overload retVal="file" descr="Opens a file"><Param name="file"/><Param name="mode='r'"/><Param name="encoding=None"/></Overload></KeyWord>
<KeyWord name="print" func="yes"><Overload retVal="None"><Param name="*values"/><Param name="sep=' '"/><Param name="end='\n'"/></Overload></KeyWord>
<KeyWord name="range" func="yes"><Overload retVal="range"><Param name="stop"/></Overload><Overload retVal="range"><Param name="start"/><Param name="stop"/><Param name="step=1"/></Overload></KeyWord>
<KeyWord name="sorted" func="yes"><Overload retVal="list"><Param name="iterable"/><Param name="key=None"/><Param name="reverse=False"/></Overload></KeyWord>
<KeyWord name="zip" func="yes"><Overload retVal="zip"><Param name="*iterables"/></Overload></KeyWord>
</AutoComplete></NotepadPlus>"#,
    r#"<NotepadPlus><AutoComplete language="JavaScript"><Environment ignoreCase="no" startFunc="(" stopFunc=")" paramSeparator=","/>
<KeyWord name="addEventListener" func="yes"><Overload retVal="void"><Param name="type"/><Param name="listener"/><Param name="options"/></Overload></KeyWord>
<KeyWord name="clearTimeout" func="yes"><Overload retVal="void"><Param name="id"/></Overload></KeyWord>
<KeyWord name="fetch" func="yes"><Overload retVal="Promise"><Param name="resource"/><Param name="options"/></Overload></KeyWord>
<KeyWord name="parseFloat" func="yes"><Overload retVal="number"><Param name="string"/></Overload></KeyWord>
<KeyWord name="parseInt" func="yes"><Overload retVal="number"><Param name="string"/><Param name="radix"/></Overload></KeyWord>
<KeyWord name="querySelector" func="yes"><Overload retVal="Element"><Param name="selectors"/></Overload></KeyWord>
<KeyWord name="setTimeout" func="yes"><Overload retVal="number"><Param name="callback"/><Param name="delay"/><Param name="...args"/></Overload></KeyWord>
</AutoComplete></NotepadPlus>"#,
];

/// The built-in API of a language (C++ uses C's).
pub fn builtin_api(lang: &str) -> Option<&'static Api> {
    static APIS: std::sync::OnceLock<Vec<Api>> = std::sync::OnceLock::new();
    let apis = APIS.get_or_init(|| {
        BUILTIN_APIS
            .iter()
            .filter_map(|x| parse_npp_api(x).ok())
            .collect()
    });
    let name = if lang == "C++" { "C" } else { lang };
    apis.iter().find(|a| a.language == name)
}

/// Reads every `*.xml` API file of `dir` (a broken one is named with its reason).
pub fn load_apis(dir: &Path) -> (Vec<Api>, Vec<(std::path::PathBuf, String)>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (ok, bad);
    };
    let mut paths: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xml")))
        .collect();
    paths.sort();
    for p in paths {
        match std::fs::read_to_string(&p)
            .map_err(|e| e.to_string())
            .and_then(|s| parse_npp_api(&s))
        {
            Ok(a) => ok.push(a),
            Err(e) => bad.push((p, e)),
        }
    }
    (ok, bad)
}

/// A function's signature as a hint: `ret name(a, b)` and the char ranges of each parameter in
/// it (to underline the current one).
pub fn signature(w: &ApiWord, o: &Overload, sep: char) -> (String, Vec<(usize, usize)>) {
    let mut s = String::new();
    if !o.ret.is_empty() {
        s.push_str(&o.ret);
        s.push(' ');
    }
    s.push_str(&w.name);
    s.push('(');
    let mut ranges = Vec::new();
    for (i, p) in o.params.iter().enumerate() {
        if i > 0 {
            s.push(sep);
            s.push(' ');
        }
        let a = s.chars().count();
        s.push_str(p);
        ranges.push((a, s.chars().count()));
    }
    s.push(')');
    (s, ranges)
}

/// The call the caret is in on its line: the function's name and the index of the parameter
/// (separators at the call's depth before the caret), outside strings and comments.
pub fn call_at(line: &str, col: usize, toks: &[Token], api: &Api) -> Option<(String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let col = col.min(chars.len());
    let skip = |c: usize| {
        toks.iter()
            .any(|t| t.start <= c && c < t.end && matches!(t.kind, Kind::String | Kind::Comment))
    };
    let mut depth = 0i32;
    let mut arg = 0;
    let mut i = col;
    while i > 0 {
        i -= 1;
        if skip(i) {
            continue;
        }
        let c = chars[i];
        if c == api.stop_func || c == ']' || c == '}' {
            depth += 1;
        } else if c == api.start_func || c == '[' || c == '{' {
            if depth == 0 {
                if c != api.start_func {
                    return None;
                }
                let (_, name) = word_before(line, i, ".");
                let name = name.rsplit('.').next().unwrap_or("").to_string();
                return api
                    .get(&name)
                    .filter(|w| w.func && !w.overloads.is_empty())
                    .map(|w| (w.name.clone(), arg));
            }
            depth -= 1;
        } else if c == api.separator && depth == 0 {
            arg += 1;
        }
    }
    None
}

/// The completion candidates for `prefix`: snippets for the language, the API's functions and
/// keywords, the language's keywords, types and constants, and `words` (the document's), matched
/// case-insensitively, the same text once (the richer kind wins), sorted by text.
pub fn candidates(
    prefix: &str,
    words: &[String],
    lang: Option<&Lang>,
    api: Option<&Api>,
    snippets: &[Snippet],
) -> Vec<Candidate> {
    let p = prefix.to_lowercase();
    let hit = |s: &str| s.to_lowercase().starts_with(&p) && s != prefix;
    let mut by: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut add = |text: &str, kind: CandKind, detail: String| {
        if !hit(text) {
            return;
        }
        let e = by.entry(text.to_string()).or_insert_with(|| Candidate {
            text: text.to_string(),
            kind,
            detail: detail.clone(),
        });
        if kind < e.kind {
            *e = Candidate {
                text: text.to_string(),
                kind,
                detail,
            };
        }
    };
    for s in snippets {
        if s.applies(lang.map(|l| l.name.as_str())) {
            add(&s.prefix, CandKind::Snippet, s.description.clone());
        }
    }
    if let Some(a) = api {
        for w in &a.words {
            let detail = w
                .overloads
                .first()
                .map(|o| signature(w, o, a.separator).0)
                .unwrap_or_default();
            add(
                &w.name,
                if w.func {
                    CandKind::Function
                } else {
                    CandKind::Keyword
                },
                detail,
            );
        }
    }
    if let Some(l) = lang {
        for k in l.keywords.iter().chain(&l.types).chain(&l.constants) {
            add(k, CandKind::Keyword, String::new());
        }
    }
    for w in words {
        add(w, CandKind::Word, String::new());
    }
    let mut v: Vec<Candidate> = by.into_values().collect();
    v.sort_by(|a, b| {
        a.text
            .to_lowercase()
            .cmp(&b.text.to_lowercase())
            .then(a.text.cmp(&b.text))
    });
    v
}

// ---- path completion ---------------------------------------------------------------------------

/// The path fragment before char column `col`: from the last space, quote, `(` or `=` on the line.
pub fn path_before(line: &str, col: usize) -> (usize, String) {
    let chars: Vec<char> = line.chars().collect();
    let col = col.min(chars.len());
    let mut s = col;
    while s > 0
        && !matches!(
            chars[s - 1],
            ' ' | '\t' | '"' | '\'' | '(' | '=' | '<' | '>' | '`' | '[' | ']'
        )
    {
        s -= 1;
    }
    (s, chars[s..col].iter().collect())
}

/// The entries of the folder `fragment` points into whose names start with its last part
/// (case-insensitively; folders end with `\`), relative fragments from `base` (the document's
/// folder). Network paths (`\\server\…`, `//…`) are never read — reading them would reach the
/// network — and neither is anything when there is no folder to start from. At most 300.
pub fn path_candidates(fragment: &str, base: Option<&Path>) -> Vec<Candidate> {
    if fragment.starts_with("\\\\") || fragment.starts_with("//") {
        return Vec::new();
    }
    let cut = fragment.rfind(['\\', '/']).map_or(0, |i| i + 1);
    let (dir, name) = fragment.split_at(cut);
    let dir_path = if dir.is_empty() {
        match base {
            Some(b) => b.to_path_buf(),
            None => return Vec::new(),
        }
    } else {
        let d = Path::new(dir);
        if d.is_absolute() || dir.len() >= 2 && dir.as_bytes()[1] == b':' {
            d.to_path_buf()
        } else {
            match base {
                Some(b) => b.join(d),
                None => return Vec::new(),
            }
        }
    };
    let Ok(rd) = std::fs::read_dir(&dir_path) else {
        return Vec::new();
    };
    let n = name.to_lowercase();
    let mut v: Vec<Candidate> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().to_string();
            if !file.to_lowercase().starts_with(&n) {
                return None;
            }
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            Some(Candidate {
                text: format!("{dir}{file}{}", if is_dir { "\\" } else { "" }),
                kind: CandKind::Path,
                detail: if is_dir {
                    "folder".into()
                } else {
                    String::new()
                },
            })
        })
        .take(300)
        .collect();
    v.sort_by_key(|a| a.text.to_lowercase());
    v
}

// ---- snippets ----------------------------------------------------------------------------------

/// A snippet: what is typed (`prefix`), what it becomes (`body`, with `$1`, `${1:placeholder}`,
/// `$0` the final caret, `\t` one indentation level) and for which language (`None`: every one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub prefix: String,
    pub body: String,
    pub description: String,
    pub language: Option<String>,
}

impl Snippet {
    /// Whether the snippet is offered in `lang` (case-insensitive; `None` is plain text).
    pub fn applies(&self, lang: Option<&str>) -> bool {
        match (&self.language, lang) {
            (None, _) => true,
            (Some(l), Some(x)) => l.eq_ignore_ascii_case(x),
            (Some(_), None) => false,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Body {
    One(String),
    Lines(Vec<String>),
}

#[derive(Deserialize)]
struct RawSnippet {
    body: Body,
    #[serde(default)]
    description: String,
}

/// Reads a snippets file: `{ "<language or *>": { "<prefix>": { "body": "…" or ["line", …],
/// "description": "…" } } }`.
pub fn parse_snippets(json: &str) -> Result<Vec<Snippet>, String> {
    let raw: BTreeMap<String, BTreeMap<String, RawSnippet>> =
        serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (lang, list) in raw {
        for (prefix, r) in list {
            if prefix.trim().is_empty() {
                return Err("a snippet without a prefix".into());
            }
            let body = match r.body {
                Body::One(s) => s,
                Body::Lines(v) => v.join("\n"),
            };
            out.push(Snippet {
                prefix,
                body,
                description: r.description,
                language: (lang != "*").then(|| lang.clone()),
            });
        }
    }
    Ok(out)
}

/// The built-in snippets (the user's file adds to them; the same prefix there wins).
pub const BUILTIN_SNIPPETS: &str = r##"{
  "*": {
    "todo": { "body": "TODO(${1:who}): $0", "description": "A TODO note" }
  },
  "Rust": {
    "fn": { "body": ["fn ${1:name}(${2}) {", "\t$0", "}"], "description": "A function" },
    "test": { "body": ["#[test]", "fn ${1:it_works}() {", "\t$0", "}"], "description": "A test" },
    "match": { "body": ["match ${1:value} {", "\t${2:_} => $0,", "}"], "description": "A match" }
  },
  "Python": {
    "def": { "body": ["def ${1:name}(${2}):", "\t${0:pass}"], "description": "A function" },
    "ifmain": { "body": ["if __name__ == \"__main__\":", "\t${0:main()}"], "description": "The main guard" }
  },
  "JavaScript": {
    "fn": { "body": ["function ${1:name}(${2}) {", "\t$0", "}"], "description": "A function" },
    "log": { "body": "console.log($0);", "description": "console.log" }
  },
  "Markdown": {
    "link": { "body": "[${1:text}](${2:url})$0", "description": "A link" },
    "code": { "body": ["```${1:rust}", "$0", "```"], "description": "A code block" },
    "table": { "body": ["| ${1:A} | ${2:B} |", "| --- | --- |", "| $0 |  |"], "description": "A table" }
  }
}"##;

/// The built-in snippets with the user's (`user`, already read) over them.
pub fn snippets_with(user: &[Snippet]) -> Vec<Snippet> {
    let mut v = parse_snippets(BUILTIN_SNIPPETS).unwrap_or_default();
    v.retain(|b| {
        !user.iter().any(|u| {
            u.prefix == b.prefix
                && u.language.as_deref().map(str::to_lowercase)
                    == b.language.as_deref().map(str::to_lowercase)
        })
    });
    v.extend(user.iter().cloned());
    v
}

/// A snippet ready to insert: the text and its tab stops (number, char ranges in the text; `0`
/// last, the same number in several places for mirrored fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expanded {
    pub text: String,
    pub stops: Vec<(u32, Vec<(usize, usize)>)>,
}

/// Expands `body`: `$n` and `${n:placeholder}` become tab stops, `\$` a dollar, `\t` at a line's
/// start `unit` (the indentation level), and every line after the first gets `indent` (the line
/// the snippet is typed on). Without a `$0` the end of the text is the last stop.
pub fn expand(body: &str, indent: &str, unit: &str, eol: &str) -> Expanded {
    let mut text = String::new();
    let mut n_chars = 0usize;
    let mut stops: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    let push = |text: &mut String, n: &mut usize, s: &str| {
        text.push_str(s);
        *n += s.chars().count();
    };
    let mut at_line_start = true;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            push(&mut text, &mut n_chars, eol);
            push(&mut text, &mut n_chars, indent);
            at_line_start = true;
            i += 1;
            continue;
        }
        if c == '\t' && at_line_start {
            push(&mut text, &mut n_chars, unit);
            i += 1;
            continue;
        }
        at_line_start = false;
        if c == '\\' && chars.get(i + 1) == Some(&'$') {
            push(&mut text, &mut n_chars, "$");
            i += 2;
            continue;
        }
        if c == '$' {
            // `$n`
            let digits: String = chars[i + 1..]
                .iter()
                .take_while(|d| d.is_ascii_digit())
                .collect();
            if !digits.is_empty() {
                let n: u32 = digits.parse().unwrap_or(0);
                stops.entry(n).or_default().push((n_chars, n_chars));
                i += 1 + digits.len();
                continue;
            }
            // `${n:placeholder}` / `${n}`
            if chars.get(i + 1) == Some(&'{') {
                let digits: String = chars[i + 2..]
                    .iter()
                    .take_while(|d| d.is_ascii_digit())
                    .collect();
                if !digits.is_empty() {
                    let n: u32 = digits.parse().unwrap_or(0);
                    let mut j = i + 2 + digits.len();
                    let mut ph = String::new();
                    if chars.get(j) == Some(&':') {
                        j += 1;
                        while j < chars.len() && chars[j] != '}' {
                            ph.push(chars[j]);
                            j += 1;
                        }
                    }
                    if chars.get(j) == Some(&'}') {
                        let a = n_chars;
                        push(&mut text, &mut n_chars, &ph);
                        stops.entry(n).or_default().push((a, n_chars));
                        i = j + 1;
                        continue;
                    }
                }
            }
        }
        let mut b = [0u8; 4];
        push(&mut text, &mut n_chars, c.encode_utf8(&mut b));
        i += 1;
    }
    let end = n_chars;
    let zero = stops.remove(&0).unwrap_or_else(|| vec![(end, end)]);
    let mut list: Vec<(u32, Vec<(usize, usize)>)> = stops.into_iter().collect();
    list.push((0, zero));
    Expanded { text, stops: list }
}

// ---- bracket and tag matching (bounded, for highlighting) --------------------------------------

/// The bracket at char column `col` of line `line` (or just before it) and its partner, skipping
/// strings and comments: `text_of` gives a line, `toks_of` its tokens; at most `max_lines` lines
/// away from the start.
pub fn match_bracket_by(
    line: usize,
    col: usize,
    total: usize,
    text_of: &dyn Fn(usize) -> String,
    toks_of: &mut dyn FnMut(usize) -> Vec<Token>,
    max_lines: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];
    let skip = |t: &[Token], c: usize| {
        t.iter()
            .any(|x| x.start <= c && c < x.end && matches!(x.kind, Kind::String | Kind::Comment))
    };
    let first: Vec<char> = text_of(line).chars().collect();
    let ftoks = toks_of(line);
    let at = [col, col.wrapping_sub(1)].into_iter().find(|c| {
        first
            .get(*c)
            .is_some_and(|ch| pairs.iter().any(|(a, b)| a == ch || b == ch))
            && !skip(&ftoks, *c)
    })?;
    let ch = first[at];
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
    let mut n = line;
    let mut row = first;
    let mut toks = ftoks;
    let mut c = at as isize;
    loop {
        while c >= 0 && (c as usize) < row.len() {
            let x = row[c as usize];
            if !skip(&toks, c as usize) {
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
            if n >= total || n - line > max_lines {
                return None;
            }
        } else {
            if n == 0 || line - n >= max_lines {
                return None;
            }
            n -= 1;
        }
        row = text_of(n).chars().collect();
        toks = toks_of(n);
        c = if forward { 0 } else { row.len() as isize - 1 };
    }
}

/// A tag name: its line and char range.
pub type TagSpan = (usize, usize, usize);

/// HTML/XML: the tag name at char column `col` of `line` (inside `<name` or `</name`) and its
/// partner's name, as (line, from, to) char ranges; void elements and self-closing tags have
/// none. At most `max_lines` lines away.
pub fn match_tag_by(
    line: usize,
    col: usize,
    total: usize,
    text_of: &dyn Fn(usize) -> String,
    toks_of: &mut dyn FnMut(usize) -> Vec<Token>,
    max_lines: usize,
) -> Option<(TagSpan, TagSpan)> {
    const VOID: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ];
    // The tag tokens of a line: (start of the name, end, name lowercased, closing).
    let tags_of = |chars: &[char], toks: &[Token]| -> Vec<(usize, usize, String, bool)> {
        toks.iter()
            .filter(|t| {
                t.kind == Kind::Tag && t.end > t.start + 1 && chars.get(t.start) == Some(&'<')
            })
            .map(|t| {
                let closing = chars.get(t.start + 1) == Some(&'/');
                let a = t.start + 1 + usize::from(closing);
                (
                    a,
                    t.end,
                    chars[a..t.end].iter().collect::<String>().to_lowercase(),
                    closing,
                )
            })
            .filter(|(a, e, _, _)| e > a)
            .collect()
    };
    let chars: Vec<char> = text_of(line).chars().collect();
    let toks = toks_of(line);
    let here = tags_of(&chars, &toks);
    let &(a, e, ref name, closing) = here
        .iter()
        .find(|(a, e, _, _)| col + 1 >= *a && col <= *e)?;
    if VOID.contains(&name.as_str()) {
        return None;
    }
    // A self-closing tag (`/>` before the next `<` on the line) has no partner.
    if !closing {
        let rest: String = chars[e..].iter().collect();
        let gt = rest.find('>');
        if gt.is_some_and(|g| rest[..g].ends_with('/')) {
            return None;
        }
    }
    let mut depth = 0i32;
    let forward = !closing;
    let mut n = line;
    let mut list = here.clone();
    let mut k: isize = here.iter().position(|t| t.0 == a).unwrap_or(0) as isize;
    loop {
        while k >= 0 && (k as usize) < list.len() {
            let (ta, te, tn, tc) = &list[k as usize];
            if tn == name {
                if *tc == closing {
                    depth += 1;
                } else {
                    depth -= 1;
                }
                if depth == 0 {
                    return Some(((line, a, e), (n, *ta, *te)));
                }
            }
            k += if forward { 1 } else { -1 };
        }
        if forward {
            n += 1;
            if n >= total || n - line > max_lines {
                return None;
            }
        } else {
            if n == 0 || line - n >= max_lines {
                return None;
            }
            n -= 1;
        }
        let ch: Vec<char> = text_of(n).chars().collect();
        let tk = toks_of(n);
        list = tags_of(&ch, &tk);
        k = if forward { 0 } else { list.len() as isize - 1 };
    }
}

#[cfg(test)]
mod tests;
