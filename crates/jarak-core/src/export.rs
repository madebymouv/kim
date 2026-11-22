//! Export (`JARAK-FR-141`, Step 106): a document as HTML, RTF or a published folder, locally.
//!
//! - Highlighted code as HTML or RTF: the lexer of Step 98 with kim-design's colours (light, and
//!   dark under `prefers-color-scheme: dark` in HTML), optionally with line numbers.
//! - Markdown as rendered HTML (the render model of Step 89). Images are embedded as `data:` URIs
//!   (self-contained, the default) or linked relative to the output file; publishing writes the
//!   page and copies of its images (`assets/`) into a folder.
//! - Only images inside the document's folder are used ([`assets::resolve`], as the preview);
//!   remote images are never fetched (`JARAK-SEC-003`: they become links); every image is checked
//!   by its first bytes and its size before it is read (`JARAK-SEC-004`).
//! - Raw HTML stays text, as in the preview; links keep only `http`, `https`, `mailto`, `#…` and
//!   relative targets; a Content-Security-Policy keeps the page from loading anything remote.
//! - Exporting never changes the document (`JARAK-SEC-001`). An output file is replaced only with
//!   `overwrite` (the window's Save dialog asks first; the CLI needs `--overwrite`), never the
//!   document itself, and it is written to a temporary file first and then moved into place.

use crate::assets;
use crate::markdown::{self, Align, Block, Inline, Item};
use crate::syntax::{self, Kind, Lang, State};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

/// The largest image file an export reads.
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// The most image bytes one export embeds or copies.
pub const MAX_TOTAL_IMAGE_BYTES: u64 = 256 * 1024 * 1024;

/// What an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Html,
    Rtf,
    /// A folder: the HTML page and copies of its images.
    Folder,
    /// Printed to a PDF printer (Step 106, `print`).
    Pdf,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Html, Format::Rtf, Format::Folder, Format::Pdf];

    pub fn name(self) -> &'static str {
        match self {
            Format::Html => "html",
            Format::Rtf => "rtf",
            Format::Folder => "folder",
            Format::Pdf => "pdf",
        }
    }

    pub fn from_name(s: &str) -> Option<Format> {
        Format::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(s))
    }

    /// The format an output file's extension names (`.html`, `.htm`, `.rtf`, `.pdf`).
    pub fn from_extension(p: &Path) -> Option<Format> {
        match p
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str()
        {
            "html" | "htm" => Some(Format::Html),
            "rtf" => Some(Format::Rtf),
            "pdf" => Some(Format::Pdf),
            _ => None,
        }
    }
}

/// How a Markdown export uses the document's images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Images {
    /// In the page as `data:` URIs: one file that works anywhere (self-contained).
    #[default]
    Embed,
    /// Referred to by their path relative to the output file.
    Link,
}

/// Export options.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub line_numbers: bool,
    pub images: Images,
    /// A Markdown document as its highlighted source instead of rendered.
    pub source: bool,
}

/// What became of one image reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageUse {
    Embedded,
    Linked,
    /// Copied into the published folder under this name (`assets/…`).
    Copied(String),
    /// No such file.
    Missing,
    /// A remote image: never fetched, shown as a link.
    Remote,
    /// Not used, and why (outside the folder, too large, not an image).
    Refused(String),
}

impl ImageUse {
    pub fn name(&self) -> &'static str {
        match self {
            ImageUse::Embedded => "embedded",
            ImageUse::Linked => "linked",
            ImageUse::Copied(_) => "copied",
            ImageUse::Missing => "missing",
            ImageUse::Remote => "remote",
            ImageUse::Refused(_) => "refused",
        }
    }
}

/// One image reference of an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReport {
    pub reference: String,
    pub used: ImageUse,
}

/// A rendered HTML page.
#[derive(Debug, Clone, Default)]
pub struct Page {
    pub html: String,
    pub images: Vec<ImageReport>,
    /// Publishing: the image files to copy and their path in the folder (`assets/…`).
    pub copies: Vec<(PathBuf, String)>,
}

/// A file an export wrote (or would write, in a dry run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Why an export wrote nothing.
#[derive(Debug)]
pub enum ExportError {
    /// These output files exist; they are replaced only with `overwrite`.
    Exists(Vec<PathBuf>),
    /// The output is the document itself, or (publishing) the document's own folder.
    IntoSource(PathBuf),
    Io(io::Error),
}

impl ExportError {
    /// The CLI error code and exit code ([CLI_CONTRACT](../../docs/CLI_CONTRACT.md) §4).
    pub fn code(&self) -> (&'static str, u8) {
        match self {
            ExportError::Exists(_) => ("export_exists", 7),
            ExportError::IntoSource(_) => ("export_into_source", 2),
            ExportError::Io(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                ("permission_denied", 1)
            }
            ExportError::Io(_) => ("io", 1),
        }
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Exists(p) => {
                let names: Vec<String> = p.iter().map(|p| p.display().to_string()).collect();
                if let [one] = names.as_slice() {
                    write!(f, "{one} already exists")
                } else {
                    write!(f, "{} files already exist: {}", p.len(), names.join(", "))
                }
            }
            ExportError::IntoSource(p) => write!(
                f,
                "{} is the document or its own folder; an export never replaces the document or its images",
                p.display()
            ),
            ExportError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        ExportError::Io(e)
    }
}

// ---------------------------------------------------------------------------------------------
// Highlighted code.

/// The lines of `text` (a final line break ends the last line; it starts no new one).
pub fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find(['\r', '\n']) {
            Some(i) => {
                out.push(&rest[..i]);
                let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[i + skip..];
            }
            None => {
                out.push(rest);
                rest = "";
            }
        }
    }
    if out.is_empty() {
        out.push("");
    }
    out
}

/// The CSS class name of a kind of token (`t-keyword`, …).
pub fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Keyword => "keyword",
        Kind::Type => "type",
        Kind::Constant => "constant",
        Kind::String => "string",
        Kind::Number => "number",
        Kind::Comment => "comment",
        Kind::Operator => "operator",
        Kind::Preprocessor => "preprocessor",
        Kind::Tag => "tag",
        Kind::Attribute => "attribute",
        Kind::Variable => "variable",
        Kind::Heading => "heading",
        Kind::Emphasis => "emphasis",
        Kind::Code => "code",
        Kind::Link => "link",
    }
}

/// The colour role of a kind of token, as the window draws it (ADR 0103).
pub fn kind_color(p: &kim_design::Palette, k: Kind) -> kim_design::Rgb {
    match k {
        Kind::Keyword | Kind::Tag | Kind::Heading | Kind::Link => p.accent,
        Kind::Type | Kind::Attribute | Kind::Variable => p.info,
        Kind::String | Kind::Code => p.success,
        Kind::Number | Kind::Constant | Kind::Preprocessor => p.warning,
        Kind::Comment => p.text_muted,
        Kind::Operator => p.text_secondary,
        Kind::Emphasis => p.text_primary,
    }
}

/// `rgb` as `#rrggbb`.
pub fn hex(c: kim_design::Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// A run of text of one kind (`None`: plain).
pub type Run = (Option<Kind>, String);

/// The tokens of every line of `text` in `lang` (none without a language): each line with its
/// runs.
pub fn highlight(text: &str, lang: Option<&Lang>) -> Vec<(String, Vec<Run>)> {
    let mut st = State::Normal;
    lines(text)
        .into_iter()
        .map(|line| {
            let runs = match lang {
                Some(l) => {
                    let (tokens, next) = syntax::lex_line(l, line, std::mem::take(&mut st));
                    st = next;
                    runs(line, &tokens)
                }
                None => vec![(None, line.to_string())],
            };
            (line.to_string(), runs)
        })
        .collect()
}

/// A line cut into runs of one kind (tokens are in characters).
fn runs(line: &str, tokens: &[syntax::Token]) -> Vec<(Option<Kind>, String)> {
    let at: Vec<usize> = line
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(line.len()))
        .collect();
    let byte = |c: usize| at[c.min(at.len() - 1)];
    let mut out = Vec::new();
    let mut pos = 0;
    let mut sorted: Vec<&syntax::Token> = tokens.iter().filter(|t| t.end > t.start).collect();
    sorted.sort_by_key(|t| t.start);
    for t in sorted {
        if t.start < pos {
            continue;
        }
        if t.start > pos {
            out.push((None, line[byte(pos)..byte(t.start)].to_string()));
        }
        out.push((Some(t.kind), line[byte(t.start)..byte(t.end)].to_string()));
        pos = t.end;
    }
    if byte(pos) < line.len() {
        out.push((None, line[byte(pos)..].to_string()));
    }
    // Neighbours of one kind (a run of operators) become one run.
    let mut merged: Vec<(Option<Kind>, String)> = Vec::with_capacity(out.len());
    for (k, t) in out {
        match merged.last_mut() {
            Some((lk, lt)) if *lk == k => lt.push_str(&t),
            _ => merged.push((k, t)),
        }
    }
    merged
}

/// How a page's code colours are given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Styling {
    /// Classes and the page's stylesheet (light and dark).
    Classes,
    /// Colours on each element (light): what the clipboard needs, without a stylesheet.
    Inline,
}

/// The `<pre>` of highlighted code.
pub fn code_fragment(text: &str, lang: Option<&Lang>, line_numbers: bool, s: Styling) -> String {
    let rows = highlight(text, lang);
    let width = rows.len().to_string().len();
    let p = &kim_design::LIGHT;
    let mut out = match s {
        Styling::Classes => String::from("<pre class=\"code\"><code>"),
        Styling::Inline => format!(
            "<pre style=\"font-family: Consolas, 'D2Coding', 'Malgun Gothic', monospace; font-size: 10pt; color: {}; background: #ffffff;\"><code>",
            hex(p.text_primary)
        ),
    };
    for (i, (_, runs)) in rows.iter().enumerate() {
        if line_numbers {
            let n = format!("{:>width$} ", i + 1);
            match s {
                Styling::Classes => {
                    let _ = write!(out, "<span class=\"ln\">{n}</span>");
                }
                Styling::Inline => {
                    let _ = write!(
                        out,
                        "<span style=\"color: {}\">{n}</span>",
                        hex(p.text_muted)
                    );
                }
            }
        }
        for (kind, t) in runs {
            match kind {
                Some(k) => match s {
                    Styling::Classes => {
                        let _ =
                            write!(out, "<span class=\"t-{}\">{}</span>", kind_name(*k), esc(t));
                    }
                    Styling::Inline => {
                        let _ = write!(
                            out,
                            "<span style=\"color: {}{}\">{}</span>",
                            hex(kind_color(p, *k)),
                            if *k == Kind::Emphasis {
                                "; font-style: italic"
                            } else {
                                ""
                            },
                            esc(t)
                        );
                    }
                },
                None => out.push_str(&esc(t)),
            }
        }
        if i + 1 < rows.len() {
            out.push('\n');
        }
    }
    out.push_str("</code></pre>");
    out
}

/// A highlighted document as an HTML page.
pub fn code_html(text: &str, lang: Option<&Lang>, title: &str, line_numbers: bool) -> String {
    page(
        title,
        "code-page",
        &code_fragment(text, lang, line_numbers, Styling::Classes),
    )
}

/// A highlighted document as RTF (Consolas 10 pt, kim-design's light colours).
pub fn code_rtf(text: &str, lang: Option<&Lang>, line_numbers: bool) -> String {
    let p = &kim_design::LIGHT;
    // 1 text, 2 accent, 3 info, 4 success, 5 warning, 6 muted, 7 secondary.
    let colors = [
        p.text_primary,
        p.accent,
        p.info,
        p.success,
        p.warning,
        p.text_muted,
        p.text_secondary,
    ];
    let index = |k: Kind| -> usize {
        let c = kind_color(p, k);
        colors.iter().position(|x| *x == c).map_or(1, |i| i + 1)
    };
    let mut out = String::from(
        "{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1{\\fonttbl{\\f0\\fmodern\\fprq1\\fcharset0 Consolas;}}\n{\\colortbl ;",
    );
    for c in colors {
        let _ = write!(out, "\\red{}\\green{}\\blue{};", c.r, c.g, c.b);
    }
    out.push_str("}\n\\f0\\fs20\\cf1 ");
    let rows = highlight(text, lang);
    let width = rows.len().to_string().len();
    for (i, (_, runs)) in rows.iter().enumerate() {
        if line_numbers {
            let _ = write!(out, "{{\\cf6 {:>width$} }}", i + 1);
        }
        for (kind, t) in runs {
            match kind {
                Some(k) => {
                    let italic = if *k == Kind::Emphasis { "\\i " } else { "" };
                    let _ = write!(out, "{{\\cf{} {italic}{}}}", index(*k), rtf_text(t));
                }
                None => out.push_str(&rtf_text(t)),
            }
        }
        if i + 1 < rows.len() {
            out.push_str("\\par\n");
        }
    }
    out.push_str("}\n");
    out
}

/// Text escaped for RTF: `\ { }`, tabs, and every non-ASCII character as `\uN?`.
fn rtf_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\t' => out.push_str("\\tab "),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c if c.is_ascii() => {}
            c => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{}?", *u as i16);
                }
            }
        }
    }
    out
}

/// The Windows clipboard's `HTML Format` for an HTML fragment (the header's byte offsets).
pub fn cf_html(fragment: &str) -> String {
    const HEAD: &str = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
    let pre = "<html><head><meta charset=\"utf-8\"></head><body><!--StartFragment-->";
    let post = "<!--EndFragment--></body></html>";
    let start_html = HEAD.len();
    let start_frag = start_html + pre.len();
    let end_frag = start_frag + fragment.len();
    let end_html = end_frag + post.len();
    format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_frag:010}\r\nEndFragment:{end_frag:010}\r\n{pre}{fragment}{post}"
    )
}

/// Step 110 (EDIT-13): the HTML a clipboard `HTML Format` entry carries — between its
/// `StartFragment`/`EndFragment` offsets (or markers), else everything after the header.
pub fn cf_html_fragment(cf: &str) -> String {
    let offset = |key: &str| -> Option<usize> {
        let i = cf.find(key)? + key.len();
        cf[i..].split(['\r', '\n']).next()?.trim().parse().ok()
    };
    if let (Some(a), Some(b)) = (offset("StartFragment:"), offset("EndFragment:"))
        && a <= b
        && b <= cf.len()
        && cf.is_char_boundary(a)
        && cf.is_char_boundary(b)
    {
        return cf[a..b].to_string();
    }
    if let (Some(a), Some(b)) = (
        cf.find("<!--StartFragment-->"),
        cf.find("<!--EndFragment-->"),
    ) && a < b
    {
        return cf[a + "<!--StartFragment-->".len()..b].to_string();
    }
    match cf.find('<') {
        Some(i) if cf[..i].contains("Version:") => cf[i..].to_string(),
        _ => cf.to_string(),
    }
}

// ---------------------------------------------------------------------------------------------
// HTML pages.

/// Nothing remote: styles in the page, images from `data:` or local files only.
pub const CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src data: file: 'self'";

/// `s` escaped for HTML text and attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The page's stylesheet: kim-design's light palette, its dark one when the reader's system is
/// dark, black on white when printed.
fn css() -> String {
    fn vars(p: &kim_design::Palette, page: kim_design::Rgb) -> String {
        let mut v = format!(
            "--bg: {}; --raised: {}; --border: {}; --text: {}; --secondary: {}; --muted: {}; --accent: {};",
            hex(page),
            hex(p.bg_raised),
            hex(p.border_subtle),
            hex(p.text_primary),
            hex(p.text_secondary),
            hex(p.text_muted),
            hex(p.accent),
        );
        for k in [
            Kind::Keyword,
            Kind::Type,
            Kind::Constant,
            Kind::String,
            Kind::Number,
            Kind::Comment,
            Kind::Operator,
            Kind::Preprocessor,
            Kind::Tag,
            Kind::Attribute,
            Kind::Variable,
            Kind::Heading,
            Kind::Emphasis,
            Kind::Code,
            Kind::Link,
        ] {
            let _ = write!(v, " --t-{}: {};", kind_name(k), hex(kind_color(p, k)));
        }
        v
    }
    let light = vars(&kim_design::LIGHT, kim_design::LIGHT.bg_overlay);
    let dark = vars(&kim_design::DARK, kim_design::DARK.bg_base);
    let mut tokens = String::new();
    for k in [
        Kind::Keyword,
        Kind::Type,
        Kind::Constant,
        Kind::String,
        Kind::Number,
        Kind::Comment,
        Kind::Operator,
        Kind::Preprocessor,
        Kind::Tag,
        Kind::Attribute,
        Kind::Variable,
        Kind::Heading,
        Kind::Emphasis,
        Kind::Code,
        Kind::Link,
    ] {
        let n = kind_name(k);
        let _ = writeln!(tokens, ".t-{n} {{ color: var(--t-{n}); }}");
    }
    format!(
        r#":root {{ color-scheme: light dark; {light} }}
@media (prefers-color-scheme: dark) {{ :root {{ {dark} }} }}
body {{ margin: 0 auto; max-width: 52rem; padding: 2rem 1.25rem; background: var(--bg); color: var(--text); font: 16px/1.6 "Segoe UI", "Malgun Gothic", system-ui, sans-serif; overflow-wrap: break-word; }}
body.code-page {{ max-width: none; padding: 1rem 1.25rem; }}
pre, code {{ font-family: Consolas, "D2Coding", "Malgun Gothic", monospace; font-size: 0.875rem; }}
pre {{ background: var(--raised); border: 1px solid var(--border); border-radius: 6px; padding: 0.75rem 1rem; overflow-x: auto; line-height: 1.45; }}
body.code-page pre {{ background: none; border: none; padding: 0; }}
:not(pre) > code {{ background: var(--raised); border-radius: 4px; padding: 0.1em 0.3em; }}
.ln {{ color: var(--muted); user-select: none; }}
{tokens}.t-emphasis {{ font-style: italic; }}
a {{ color: var(--accent); }}
h1, h2 {{ border-bottom: 1px solid var(--border); padding-bottom: 0.25em; }}
blockquote {{ margin: 0; padding: 0 1rem; color: var(--secondary); border-left: 4px solid var(--border); }}
table {{ border-collapse: collapse; }}
th, td {{ border: 1px solid var(--border); padding: 0.3rem 0.75rem; }}
th {{ background: var(--raised); }}
hr {{ border: none; border-top: 1px solid var(--border); }}
img {{ max-width: 100%; }}
li.task {{ list-style: none; }}
li.task > input {{ margin: 0 0.4em 0 -1.4em; }}
.raw-html {{ color: var(--secondary); }}
.missing-image, .remote-image {{ color: var(--muted); font-style: italic; }}
.footnotes {{ border-top: 1px solid var(--border); margin-top: 2rem; font-size: 0.9rem; color: var(--secondary); }}
@media print {{ :root {{ {light} }} body {{ max-width: none; padding: 0; }} pre {{ white-space: pre-wrap; }} }}
"#
    )
}

/// A complete page around `body`.
fn page(title: &str, class: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"{CSP}\">\n<meta name=\"generator\" content=\"Jarak\">\n<title>{}</title>\n<style>\n{}</style>\n</head>\n<body class=\"{class}\">\n{body}\n</body>\n</html>\n",
        esc(title),
        css()
    )
}

/// Where a Markdown page's images go.
enum Target<'a> {
    Embed,
    /// Linked relative to this folder (the output file's).
    LinkFrom(&'a Path),
    Publish,
}

struct Render<'a> {
    doc: Option<&'a Path>,
    target: Target<'a>,
    images: Vec<ImageReport>,
    copies: Vec<(PathBuf, String)>,
    budget: u64,
    slugs: std::collections::HashMap<String, usize>,
}

/// Rendered Markdown as an HTML page: images embedded or linked relative to `output`.
pub fn markdown_html(
    text: &str,
    doc: Option<&Path>,
    title: &str,
    images: Images,
    output: &Path,
) -> Page {
    let target = match images {
        Images::Embed => Target::Embed,
        Images::Link => Target::LinkFrom(output.parent().unwrap_or(Path::new("."))),
    };
    render_page(text, doc, title, target)
}

fn render_page(text: &str, doc: Option<&Path>, title: &str, target: Target<'_>) -> Page {
    let mut r = Render {
        doc,
        target,
        images: Vec::new(),
        copies: Vec::new(),
        budget: MAX_TOTAL_IMAGE_BYTES,
        slugs: std::collections::HashMap::new(),
    };
    let rendered = markdown::parse(text);
    let mut body = String::new();
    let mut notes = String::new();
    for b in &rendered.blocks {
        if let Block::Footnote { .. } = b.block {
            r.block(&mut notes, &b.block);
        } else {
            r.block(&mut body, &b.block);
        }
    }
    if !notes.is_empty() {
        let _ = write!(body, "<section class=\"footnotes\">\n{notes}</section>\n");
    }
    Page {
        html: page(title, "markdown", body.trim_end()),
        images: r.images,
        copies: r.copies,
    }
}

/// A GitHub-style anchor for a heading.
pub fn slug(text: &str) -> String {
    let mut s = String::new();
    for c in text.trim().chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            s.push(c);
        } else if c.is_whitespace() {
            s.push('-');
        }
    }
    s
}

/// A link target the page may keep: `http(s)`, `mailto`, `#…` and relative ones.
pub fn safe_href(url: &str) -> Option<String> {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || u.starts_with('#')
    {
        return Some(u.to_string());
    }
    // Any other scheme (`javascript:`, `file:`, `data:`, a drive letter) is dropped.
    let head = u.split(['/', '?', '#']).next().unwrap_or("");
    (!head.contains(':') && !u.starts_with('\\') && !u.is_empty()).then(|| u.to_string())
}

/// A path as a relative URL (`/` separators, unsafe characters escaped).
fn url_path(parts: &[String]) -> String {
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        for b in p.bytes() {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                out.push(b as char);
            } else {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

/// `to` relative to the folder `from` (`None` on another drive).
fn relative(from: &Path, to: &Path) -> Option<Vec<String>> {
    let f: Vec<String> = from
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let t: Vec<std::path::Component<'_>> = to.components().collect();
    let tl: Vec<String> = t
        .iter()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if f.first() != tl.first() {
        return None;
    }
    let common = f.iter().zip(&tl).take_while(|(a, b)| a == b).count();
    let mut out: Vec<String> = std::iter::repeat_n("..".to_string(), f.len() - common).collect();
    out.extend(
        t[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(out)
}

impl Render<'_> {
    fn heading_id(&mut self, text: &str) -> String {
        let base = slug(text);
        let n = self.slugs.entry(base.clone()).or_insert(0);
        let id = if *n == 0 { base } else { format!("{base}-{n}") };
        *n += 1;
        id
    }

    fn block(&mut self, out: &mut String, b: &Block) {
        match b {
            Block::Heading { level, inlines } => {
                let id = self.heading_id(&markdown::plain(inlines));
                let _ = write!(out, "<h{level} id=\"{}\">", esc(&id));
                self.inlines(out, inlines);
                let _ = writeln!(out, "</h{level}>");
            }
            Block::Paragraph(v) => {
                out.push_str("<p>");
                self.inlines(out, v);
                out.push_str("</p>\n");
            }
            Block::List { start, items } => {
                match start {
                    Some(1) => out.push_str("<ol>\n"),
                    Some(n) => {
                        let _ = writeln!(out, "<ol start=\"{n}\">");
                    }
                    None => out.push_str("<ul>\n"),
                }
                for it in items {
                    self.item(out, it);
                }
                out.push_str(if start.is_some() {
                    "</ol>\n"
                } else {
                    "</ul>\n"
                });
            }
            Block::Quote(v) => {
                out.push_str("<blockquote>\n");
                for b in v {
                    self.block(out, b);
                }
                out.push_str("</blockquote>\n");
            }
            Block::Code { lang, text } => {
                let l = syntax::fence_lang(&lang.to_ascii_lowercase());
                let mut frag =
                    code_fragment(text.trim_end_matches('\n'), l, false, Styling::Classes);
                if !lang.is_empty() {
                    frag = frag.replacen(
                        "<pre class=\"code\"><code>",
                        &format!(
                            "<pre class=\"code\"><code class=\"language-{}\">",
                            esc(lang)
                        ),
                        1,
                    );
                }
                out.push_str(&frag);
                out.push('\n');
            }
            Block::Table {
                align,
                header,
                rows,
            } => {
                let style = |i: usize| match align.get(i) {
                    Some(Align::Left) => " style=\"text-align: left\"",
                    Some(Align::Center) => " style=\"text-align: center\"",
                    Some(Align::Right) => " style=\"text-align: right\"",
                    _ => "",
                };
                out.push_str("<table>\n<thead>\n<tr>");
                for (i, c) in header.iter().enumerate() {
                    let _ = write!(out, "<th{}>", style(i));
                    self.inlines(out, c);
                    out.push_str("</th>");
                }
                out.push_str("</tr>\n</thead>\n<tbody>\n");
                for row in rows {
                    out.push_str("<tr>");
                    for (i, c) in row.iter().enumerate() {
                        let _ = write!(out, "<td{}>", style(i));
                        self.inlines(out, c);
                        out.push_str("</td>");
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Block::Rule => out.push_str("<hr>\n"),
            Block::Html(h) => {
                let _ = writeln!(out, "<pre class=\"raw-html\">{}</pre>", esc(h.trim_end()));
            }
            Block::Footnote { label, blocks } => {
                let id = slug(label);
                let _ = writeln!(
                    out,
                    "<div class=\"footnote\" id=\"fn-{}\"><sup>{}</sup>",
                    esc(&id),
                    esc(label)
                );
                for b in blocks {
                    self.block(out, b);
                }
                out.push_str("</div>\n");
            }
        }
    }

    fn item(&mut self, out: &mut String, it: &Item) {
        match it.task {
            Some(done) => {
                let _ = write!(
                    out,
                    "<li class=\"task\"><input type=\"checkbox\" disabled{}> ",
                    if done { " checked" } else { "" }
                );
            }
            None => out.push_str("<li>"),
        }
        if let [Block::Paragraph(v)] = it.blocks.as_slice() {
            self.inlines(out, v);
        } else {
            out.push('\n');
            for b in &it.blocks {
                self.block(out, b);
            }
        }
        out.push_str("</li>\n");
    }

    fn inlines(&mut self, out: &mut String, v: &[Inline]) {
        for i in v {
            match i {
                Inline::Text(t) => out.push_str(&esc(t)),
                Inline::Code(t) => {
                    let _ = write!(out, "<code>{}</code>", esc(t));
                }
                Inline::Emph(v) => {
                    out.push_str("<em>");
                    self.inlines(out, v);
                    out.push_str("</em>");
                }
                Inline::Strong(v) => {
                    out.push_str("<strong>");
                    self.inlines(out, v);
                    out.push_str("</strong>");
                }
                Inline::Strike(v) => {
                    out.push_str("<del>");
                    self.inlines(out, v);
                    out.push_str("</del>");
                }
                Inline::Link { url, children } => match safe_href(url) {
                    Some(h) => {
                        let _ = write!(out, "<a href=\"{}\">", esc(&h));
                        self.inlines(out, children);
                        out.push_str("</a>");
                    }
                    None => self.inlines(out, children),
                },
                Inline::Image { url, alt } => self.image(out, url, alt),
                Inline::SoftBreak => out.push('\n'),
                Inline::HardBreak => out.push_str("<br>\n"),
                Inline::FootnoteRef(l) => {
                    let _ = write!(
                        out,
                        "<sup class=\"fnref\"><a href=\"#fn-{}\">{}</a></sup>",
                        esc(&slug(l)),
                        esc(l)
                    );
                }
                Inline::Html(h) => {
                    let _ = write!(out, "<span class=\"raw-html\">{}</span>", esc(h));
                }
            }
        }
    }

    fn report(&mut self, reference: &str, used: ImageUse) {
        self.images.push(ImageReport {
            reference: reference.to_string(),
            used,
        });
    }

    fn image(&mut self, out: &mut String, url: &str, alt: &str) {
        let lower = url.trim().to_ascii_lowercase();
        let shown = if alt.is_empty() { url } else { alt };
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
        {
            let href = safe_href(url).unwrap_or_default();
            let _ = write!(
                out,
                "<a class=\"remote-image\" href=\"{}\">{} (remote image, not loaded)</a>",
                esc(&href),
                esc(shown)
            );
            self.report(url, ImageUse::Remote);
            return;
        }
        let refused = |r: &mut Self, out: &mut String, why: String| {
            let _ = write!(
                out,
                "<span class=\"missing-image\">{} ({})</span>",
                esc(shown),
                esc(&why)
            );
            r.report(url, ImageUse::Refused(why));
        };
        let Some(doc) = self.doc else {
            refused(self, out, "the document is not saved".into());
            return;
        };
        let Some(path) = assets::resolve(doc, url) else {
            refused(self, out, "outside the document's folder".into());
            return;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            let _ = write!(
                out,
                "<span class=\"missing-image\">{} (image not found: {})</span>",
                esc(shown),
                esc(url)
            );
            self.report(url, ImageUse::Missing);
            return;
        };
        if !meta.is_file() {
            refused(self, out, "not a file".into());
            return;
        }
        if meta.len() > MAX_IMAGE_BYTES {
            refused(self, out, "larger than 64 MiB".into());
            return;
        }
        let Some(format) = assets::image_format(&assets::file_head(&path)) else {
            refused(
                self,
                out,
                "not an image Jarak knows (PNG, JPEG, GIF, BMP, WebP)".into(),
            );
            return;
        };
        if matches!(self.target, Target::Embed | Target::Publish) && meta.len() > self.budget {
            refused(self, out, "the export's images exceed 256 MiB".into());
            return;
        }
        let src = match &self.target {
            Target::Embed => match std::fs::read(&path) {
                Ok(bytes) => {
                    self.budget -= meta.len();
                    self.report(url, ImageUse::Embedded);
                    format!(
                        "data:image/{format};base64,{}",
                        crate::lineops::base64_encode(&bytes, true, false)
                    )
                }
                Err(e) => {
                    refused(self, out, e.to_string());
                    return;
                }
            },
            Target::LinkFrom(dir) => {
                let rel = relative(dir, &path);
                self.report(url, ImageUse::Linked);
                match rel {
                    Some(parts) => url_path(&parts),
                    None => format!(
                        "file:///{}",
                        url_path(
                            &path
                                .to_string_lossy()
                                .replace('\\', "/")
                                .split('/')
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        )
                        .replace("%3A", ":")
                    ),
                }
            }
            Target::Publish => {
                let name = self.asset_name(&path);
                let rel = format!("{}/{name}", assets::ASSET_DIR);
                let src = url_path(&[assets::ASSET_DIR.to_string(), name]);
                self.report(url, ImageUse::Copied(rel.clone()));
                if !self.copies.iter().any(|(p, _)| *p == path) {
                    self.budget -= meta.len();
                    self.copies.push((path, rel));
                }
                src
            }
        };
        let _ = write!(out, "<img src=\"{}\" alt=\"{}\">", esc(&src), esc(alt));
    }

    /// A file name in the published `assets/` folder for `path` (the same file keeps its name;
    /// another file with that name gets `-2`, `-3`, …).
    fn asset_name(&self, path: &Path) -> String {
        if let Some((_, rel)) = self.copies.iter().find(|(p, _)| p == path) {
            return rel.rsplit('/').next().unwrap_or(rel).to_string();
        }
        let name = path
            .file_name()
            .map_or_else(|| "image".into(), |n| n.to_string_lossy().into_owned());
        let taken = |n: &str| {
            self.copies.iter().any(|(_, r)| {
                r.rsplit('/')
                    .next()
                    .is_some_and(|x| x.eq_ignore_ascii_case(n))
            })
        };
        if !taken(&name) {
            return name;
        }
        let (stem, ext) = match name.rfind('.') {
            Some(i) if i > 0 => (&name[..i], &name[i..]),
            _ => (name.as_str(), ""),
        };
        (2..)
            .map(|n| format!("{stem}-{n}{ext}"))
            .find(|n| !taken(n))
            .unwrap_or(name.clone())
    }
}

// ---------------------------------------------------------------------------------------------
// Writing.

/// Whether two paths name the same file (by identity when both exist, else by name).
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => {
            let n = |p: &Path| {
                std::path::absolute(p)
                    .unwrap_or_else(|_| p.to_path_buf())
                    .to_string_lossy()
                    .to_lowercase()
            };
            n(a) == n(b)
        }
    }
}

/// Writes `bytes` to `path` through a temporary file in its folder. An existing file is replaced
/// only with `overwrite`, the document (`source`) never; a dry run only checks.
pub fn write_output(
    path: &Path,
    bytes: &[u8],
    source: Option<&Path>,
    overwrite: bool,
    dry_run: bool,
) -> Result<Written, ExportError> {
    if source.is_some_and(|s| same_file(path, s)) {
        return Err(ExportError::IntoSource(path.to_path_buf()));
    }
    if path.exists() && !overwrite {
        return Err(ExportError::Exists(vec![path.to_path_buf()]));
    }
    let w = Written {
        path: path.to_path_buf(),
        bytes: bytes.len() as u64,
    };
    if !dry_run {
        put(path, |tmp| std::fs::write(tmp, bytes))?;
    }
    Ok(w)
}

/// Fills a temporary file beside `path` with `fill`, flushes it and moves it into place.
fn put(path: &Path, fill: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    if let Some(d) = dir {
        std::fs::create_dir_all(d)?;
    }
    let name = path
        .file_name()
        .map_or_else(|| "export".into(), |n| n.to_string_lossy().into_owned());
    let tmp = dir
        .unwrap_or(Path::new("."))
        .join(format!(".{name}.jarak-{}.tmp", std::process::id()));
    let r = fill(&tmp)
        .and_then(|()| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&tmp)?
                .sync_all()
        })
        .and_then(|()| std::fs::rename(&tmp, path));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// A published folder.
#[derive(Debug, Clone)]
pub struct Published {
    /// The page (`<document name>.html`).
    pub page: PathBuf,
    /// Every file written (or, in a dry run, to be written): the page first, then the images.
    pub files: Vec<Written>,
    pub images: Vec<ImageReport>,
}

/// Publishes the Markdown document `doc` (its text `text`) to the folder `out`: the rendered page
/// and copies of the images it shows (`assets/…`). Files that exist are replaced only with
/// `overwrite`; the document's own folder is refused (its images would be written onto
/// themselves); a dry run only checks.
pub fn publish(
    doc: &Path,
    text: &str,
    out: &Path,
    overwrite: bool,
    dry_run: bool,
) -> Result<Published, ExportError> {
    if doc.parent().is_some_and(|d| same_file(d, out)) {
        return Err(ExportError::IntoSource(out.to_path_buf()));
    }
    let stem = doc
        .file_stem()
        .map_or_else(|| "document".into(), |s| s.to_string_lossy().into_owned());
    let title = doc
        .file_name()
        .map_or_else(|| stem.clone(), |s| s.to_string_lossy().into_owned());
    let p = render_page(text, Some(doc), &title, Target::Publish);
    let page_path = out.join(format!("{stem}.html"));
    let mut files = vec![Written {
        path: page_path.clone(),
        bytes: p.html.len() as u64,
    }];
    for (src, rel) in &p.copies {
        files.push(Written {
            path: out.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR)),
            bytes: std::fs::metadata(src).map_or(0, |m| m.len()),
        });
    }
    let existing: Vec<PathBuf> = files
        .iter()
        .filter(|f| f.path.exists())
        .map(|f| f.path.clone())
        .collect();
    if !existing.is_empty() && !overwrite {
        return Err(ExportError::Exists(existing));
    }
    if !dry_run {
        for ((src, _), w) in p.copies.iter().zip(&files[1..]) {
            put(&w.path, |tmp| std::fs::copy(src, tmp).map(|_| ()))?;
        }
        put(&page_path, |tmp| std::fs::write(tmp, &p.html))?;
    }
    Ok(Published {
        page: page_path,
        files,
        images: p.images,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0,
        0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89, 0, 0, 0, 13, b'I', b'D', b'A',
        b'T', 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0, 0x05, 0x00, 0x01, 0xFF, 0x89,
        0x99, 0x3D, 0x1D, 0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn lines_and_runs_cover_the_text() {
        assert_eq!(lines("a\r\nb\nc\rd\n"), vec!["a", "b", "c", "d"]);
        assert_eq!(lines(""), vec![""]);
        assert_eq!(lines("x\n\n"), vec!["x", ""]);
        let rust = syntax::builtin("Rust").unwrap();
        let rows = highlight("fn 한글() { \"s\" } // c", Some(rust));
        let joined: String = rows[0].1.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(
            joined, "fn 한글() { \"s\" } // c",
            "runs keep every character"
        );
        assert!(rows[0].1.contains(&(Some(Kind::Keyword), "fn".into())));
        assert!(rows[0].1.contains(&(Some(Kind::Comment), "// c".into())));
    }

    #[test]
    fn code_html_is_escaped_highlighted_and_numbered() {
        let rust = syntax::builtin("Rust").unwrap();
        let h = code_html(
            "fn main() {\n    let s = \"<b>&\";\n}\n",
            Some(rust),
            "a<b>.rs",
            true,
        );
        assert!(h.contains("<title>a&lt;b&gt;.rs</title>"));
        assert!(h.contains("<span class=\"t-keyword\">fn</span>"));
        assert!(h.contains("&quot;&lt;b&gt;&amp;&quot;"), "{h}");
        assert!(h.contains("<span class=\"ln\">1 </span>"));
        assert!(h.contains(CSP));
        assert!(!h.contains("<b>"), "no markup from the text");
        let inline = code_fragment("fn x", Some(rust), false, Styling::Inline);
        assert!(inline.contains(&format!(
            "<span style=\"color: {}\">fn</span>",
            hex(kim_design::LIGHT.accent)
        )));
    }

    #[test]
    fn rtf_escapes_and_colours() {
        let rust = syntax::builtin("Rust").unwrap();
        let r = code_rtf("fn {x}\\ 한\t\n// é", Some(rust), true);
        assert!(r.starts_with("{\\rtf1"));
        assert!(r.contains("{\\cf2 fn}"), "{r}");
        assert!(code_rtf("{x}\\", None, false).contains("\\{x\\}\\\\"));
        assert!(r.contains("\\u-10916?"), "한 is U+D55C: {r}");
        assert!(r.contains("\\tab "));
        assert!(r.contains("\\u233?"), "é");
        assert!(r.contains("{\\cf6 1 }"));
        assert_eq!(r.matches("\\par").count(), 1);
    }

    #[test]
    fn cf_html_offsets_point_at_the_fragment() {
        let c = cf_html("<b>한</b>");
        let num = |k: &str| -> usize {
            let i = c.find(k).unwrap() + k.len();
            c[i..i + 10].parse().unwrap()
        };
        assert_eq!(&c[num("StartFragment:")..num("EndFragment:")], "<b>한</b>");
        assert!(c[num("StartHTML:")..].starts_with("<html>"));
        assert_eq!(num("EndHTML:"), c.len());
    }

    #[test]
    fn a_clipboard_html_fragment_is_found() {
        let c = cf_html("<b>한</b>");
        assert_eq!(cf_html_fragment(&c), "<b>한</b>");
        assert_eq!(
            cf_html_fragment("<html><!--StartFragment--><i>x</i><!--EndFragment--></html>"),
            "<i>x</i>"
        );
        assert_eq!(cf_html_fragment("<p>plain</p>"), "<p>plain</p>");
    }

    #[test]
    fn links_keep_only_safe_targets() {
        assert_eq!(safe_href("https://x.org").as_deref(), Some("https://x.org"));
        assert_eq!(safe_href("docs/a.md#b").as_deref(), Some("docs/a.md#b"));
        assert_eq!(safe_href("#top").as_deref(), Some("#top"));
        assert_eq!(safe_href("javascript:alert(1)"), None);
        assert_eq!(safe_href("JavaScript:alert(1)"), None);
        assert_eq!(safe_href("file:///c:/x"), None);
        assert_eq!(safe_href("C:\\x"), None);
        assert_eq!(slug("Hello, World 한글"), "hello-world-한글");
    }

    #[test]
    fn markdown_images_are_embedded_linked_or_refused_and_the_document_kept() {
        let d = temp("md");
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/dot.png"), PNG).unwrap();
        std::fs::write(d.join("assets/fake.png"), b"not an image").unwrap();
        let text = "# T\n\n![dot](assets/dot.png) ![gone](assets/gone.png) ![r](https://example.com/r.png) ![up](../x.png) ![fake](assets/fake.png)\n\n<script>alert(1)</script>\n\n[bad](javascript:alert(1))\n";
        let doc = d.join("a.md");
        std::fs::write(&doc, text).unwrap();
        let p = markdown_html(
            text,
            Some(&doc),
            "a.md",
            Images::Embed,
            &d.join("out/a.html"),
        );
        assert!(p.html.contains("src=\"data:image/png;base64,iVBORw0KGgo"));
        let used: Vec<&str> = p.images.iter().map(|i| i.used.name()).collect();
        assert_eq!(
            used,
            ["embedded", "missing", "remote", "refused", "refused"]
        );
        assert!(!p.html.contains("<script>"), "raw HTML stays text");
        assert!(p.html.contains("&lt;script&gt;"));
        assert!(!p.html.contains("javascript:"));
        assert!(!p.html.contains("src=\"http"), "nothing remote is loaded");
        let l = markdown_html(
            text,
            Some(&doc),
            "a.md",
            Images::Link,
            &d.join("out/a.html"),
        );
        assert!(l.html.contains("src=\"../assets/dot.png\""), "{}", l.html);
        assert_eq!(std::fs::read_to_string(&doc).unwrap(), text, "unchanged");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn writing_never_replaces_without_overwrite_or_the_document() {
        let d = temp("write");
        let doc = d.join("a.txt");
        std::fs::write(&doc, "doc").unwrap();
        let out = d.join("a.html");
        assert!(write_output(&out, b"one", Some(&doc), false, true).is_ok());
        assert!(!out.exists(), "a dry run writes nothing");
        write_output(&out, b"one", Some(&doc), false, false).unwrap();
        let e = write_output(&out, b"two", Some(&doc), false, false).unwrap_err();
        assert_eq!(e.code(), ("export_exists", 7));
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "one");
        write_output(&out, b"two", Some(&doc), true, false).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "two");
        let e = write_output(&doc, b"x", Some(&doc), true, false).unwrap_err();
        assert_eq!(e.code(), ("export_into_source", 2));
        assert_eq!(std::fs::read_to_string(&doc).unwrap(), "doc");
        let left: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(left.is_empty(), "no temporary file left");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn publishing_copies_the_images_and_asks_before_replacing() {
        let d = temp("publish");
        std::fs::create_dir_all(d.join("doc/assets")).unwrap();
        std::fs::create_dir_all(d.join("doc/pics")).unwrap();
        std::fs::write(d.join("doc/assets/dot.png"), PNG).unwrap();
        std::fs::write(d.join("doc/pics/dot.png"), PNG).unwrap();
        let text = "![a](assets/dot.png) ![b](pics/dot.png) ![a again](assets/dot.png)\n";
        let doc = d.join("doc/note.md");
        std::fs::write(&doc, text).unwrap();
        let out = d.join("site");
        let dry = publish(&doc, text, &out, false, true).unwrap();
        assert!(!out.exists(), "a dry run writes nothing");
        assert_eq!(dry.files.len(), 3, "the page and two different files");
        let p = publish(&doc, text, &out, false, false).unwrap();
        assert_eq!(p.page, out.join("note.html"));
        assert!(out.join("assets/dot.png").is_file());
        assert!(out.join("assets/dot-2.png").is_file());
        let html = std::fs::read_to_string(&p.page).unwrap();
        assert!(html.contains("src=\"assets/dot.png\""));
        assert!(html.contains("src=\"assets/dot-2.png\""));
        match publish(&doc, text, &out, false, false) {
            Err(ExportError::Exists(v)) => assert_eq!(v.len(), 3),
            other => panic!("{other:?}"),
        }
        assert!(publish(&doc, text, &out, true, false).is_ok());
        let e = publish(&doc, text, &d.join("doc"), true, false).unwrap_err();
        assert_eq!(e.code().0, "export_into_source");
        assert_eq!(std::fs::read_to_string(&doc).unwrap(), text);
        assert_eq!(std::fs::read(d.join("doc/assets/dot.png")).unwrap(), PNG);
        let _ = std::fs::remove_dir_all(&d);
    }
}
