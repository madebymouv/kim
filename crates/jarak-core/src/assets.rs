//! Pasted images as portable local assets (`JARAK-FR-042`, `JARAK-SEC-005`).
//!
//! An image pasted into `notes/ideas.md` becomes `notes/assets/ideas-20261122-001.png` and the
//! document gets the relative reference `assets/ideas-20261122-001.png`:
//!
//! - the folder is `assets` next to the document (created when needed);
//! - the name is the document's name made portable (letters of any script, digits, `-`, `_`;
//!   anything else becomes `-`), the date and the first free three-digit number;
//! - the file is created with "create new" semantics, so an existing file is **never**
//!   overwritten, also when two pastes race; nothing is ever deleted here;
//! - the reference uses forward slashes and no spaces, so it works on every platform and in any
//!   Markdown renderer, and keeps working after the document and its `assets` folder move together.
//!
//! Step 91 (`JARAK-FR-045`): the image references of a document with the exact bytes of their
//! destinations ([`references`]), the assets no Markdown file of the folder refers to
//! ([`orphans`]), and renaming an asset together with its references ([`plan_rename`],
//! [`move_no_replace`]) — on explicit request only; nothing here deletes a file.

use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// The folder name next to the document.
pub const ASSET_DIR: &str = "assets";

/// The most images one document may get on one day before numbers run out.
const MAX_PER_DAY: u32 = 999;

/// A written asset: where it is and how the document refers to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub path: PathBuf,
    /// Relative to the document's folder, with forward slashes (`assets/x-20261122-001.png`).
    pub reference: String,
}

impl Asset {
    /// The Markdown that shows it: `![alt](reference)`.
    pub fn markdown(&self, alt: &str) -> String {
        let alt: String = alt.chars().filter(|c| !"[]\n\r".contains(*c)).collect();
        format!("![{alt}]({})", self.reference)
    }
}

/// A file-name-safe version of `stem` (the document's name without its extension).
pub fn portable_stem(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    let mut dash = false;
    for c in stem.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            dash = c == '-';
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "image".to_string()
    } else {
        trimmed.chars().take(60).collect()
    }
}

/// Writes `bytes` as a new asset of the document at `doc` (`ext` without the dot, `date` as
/// year, month, day). Refuses a document without a folder; never overwrites a file.
pub fn write_new(doc: &Path, ext: &str, date: (i32, u32, u32), bytes: &[u8]) -> io::Result<Asset> {
    let folder = doc
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the document has no folder"))?;
    let stem = portable_stem(
        &doc.file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default(),
    );
    let ext: String = ext
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(5)
        .collect::<String>()
        .to_ascii_lowercase();
    let ext = if ext.is_empty() {
        "png".to_string()
    } else {
        ext
    };
    let dir = folder.join(ASSET_DIR);
    std::fs::create_dir_all(&dir)?;
    let (y, m, d) = date;
    for n in 1..=MAX_PER_DAY {
        let name = format!("{stem}-{y:04}{m:02}{d:02}-{n:03}.{ext}");
        let path = dir.join(&name);
        let mut f = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let written = f.write_all(bytes).and_then(|()| f.sync_all());
        if let Err(e) = written {
            drop(f);
            // Only the file this call created (and could not fill) is removed.
            let _ = std::fs::remove_file(&path);
            return Err(e);
        }
        return Ok(Asset {
            path,
            reference: format!("{ASSET_DIR}/{name}"),
        });
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{MAX_PER_DAY} images for this document today already"),
    ))
}

/// The file a relative reference in the document at `doc` points to, when it stays inside the
/// document's folder (`..` and absolute references are not followed; remote ones are not files).
pub fn resolve(doc: &Path, reference: &str) -> Option<PathBuf> {
    let r = reference.trim();
    let lower = r.to_ascii_lowercase();
    if lower.contains("://")
        || lower.starts_with("data:")
        || r.starts_with('/')
        || r.starts_with('\\')
    {
        return None;
    }
    let r = r.split(['?', '#']).next().unwrap_or(r);
    let decoded = percent_decode(r);
    let rel = Path::new(&decoded);
    if rel.is_absolute()
        || rel.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return None;
    }
    // Rebuilt part by part: one kind of separator, no `.` parts.
    let mut out = doc.parent()?.to_path_buf();
    for c in rel.components() {
        if let std::path::Component::Normal(n) = c {
            out.push(n);
        }
    }
    Some(out)
}

/// `%20`-style escapes in a reference (Markdown writers often encode spaces).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (
                (b[i + 1] as char).to_digit(16),
                (b[i + 2] as char).to_digit(16),
            )
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// An image reference in a document's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The destination as the parser reads it (escapes removed).
    pub url: String,
    /// The bytes of the destination as written in the text (`None` when it cannot be located
    /// exactly, so it is never rewritten).
    pub span: Option<Range<usize>>,
    /// The line (0-based) of the image.
    pub line: usize,
}

/// Every image reference of the Markdown `text`, in order (inline images and the reference
/// definitions images use).
pub fn references(text: &str) -> Vec<Reference> {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let line_of = |b: usize| text[..b.min(text.len())].matches('\n').count();
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(text, opts);
    let defs: Vec<(String, Range<usize>)> = parser
        .reference_definitions()
        .iter()
        .map(|(_, d)| (d.dest.to_string(), d.span.clone()))
        .collect();
    let mut out = Vec::new();
    let mut used_defs: Vec<Range<usize>> = Vec::new();
    for (e, range) in parser.into_offset_iter() {
        let Event::Start(Tag::Image { dest_url, .. }) = e else {
            continue;
        };
        let url = dest_url.to_string();
        let mut span = inline_dest(text, range.clone(), &url);
        if span.is_none() {
            // `![alt][label]`: the destination is in the definition.
            if let Some((_, d)) = defs.iter().find(|(u, _)| *u == url) {
                span = def_dest(text, d.clone(), &url);
                if let Some(sp) = &span {
                    if used_defs.contains(sp) {
                        // One definition, several images: rewritten once.
                        out.push(Reference {
                            url,
                            span: None,
                            line: line_of(range.start),
                        });
                        continue;
                    }
                    used_defs.push(sp.clone());
                }
            }
        }
        out.push(Reference {
            url,
            span,
            line: line_of(range.start),
        });
    }
    out
}

/// Removes Markdown backslash escapes (`\(` → `(`).
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\'
            && let Some(&n) = it.peek()
            && n.is_ascii_punctuation()
        {
            out.push(n);
            it.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// The destination written at the start of `t` (`<…>` or up to white space or the closing
/// parenthesis): its offset in `t` and length.
fn written_dest(t: &str) -> Option<(usize, usize)> {
    let lead = t.len() - t.trim_start().len();
    let t2 = &t[lead..];
    if let Some(r) = t2.strip_prefix('<') {
        let end = r.find('>')?;
        return Some((lead + 1, end));
    }
    let mut depth = 0usize;
    let mut esc = false;
    for (k, ch) in t2.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        match ch {
            '\\' => esc = true,
            '(' => depth += 1,
            ')' if depth == 0 => return Some((lead, k)),
            ')' => depth -= 1,
            c if c.is_whitespace() => return Some((lead, k)),
            _ => {}
        }
    }
    Some((lead, t2.len()))
}

fn inline_dest(text: &str, range: Range<usize>, url: &str) -> Option<Range<usize>> {
    let s = text.get(range.clone())?;
    if !s.ends_with(')') {
        return None;
    }
    let mut end = s.len();
    while let Some(i) = s[..end].rfind("](") {
        let after = &s[i + 2..];
        if let Some((off, len)) = written_dest(after)
            && unescape(&after[off..off + len]) == url
        {
            let a = range.start + i + 2 + off;
            return Some(a..a + len);
        }
        end = i;
    }
    None
}

fn def_dest(text: &str, span: Range<usize>, url: &str) -> Option<Range<usize>> {
    let s = text.get(span.clone())?;
    let i = s.find("]:")?;
    let after = &s[i + 2..];
    let (off, len) = written_dest(after)?;
    (unescape(&after[off..off + len]) == url).then(|| {
        let a = span.start + i + 2 + off;
        a..a + len
    })
}

/// The image reference whose `![…](…)` covers byte `at` of `line` (a line of the source).
pub fn reference_at(line: &str, at: usize) -> Option<String> {
    use pulldown_cmark::{Event, Parser, Tag};
    let mut depth = 0usize;
    let mut found = None;
    for (e, range) in Parser::new(line).into_offset_iter() {
        match e {
            Event::Start(Tag::Image { dest_url, .. }) => {
                if depth == 0 && range.contains(&at) {
                    found = Some(dest_url.to_string());
                }
                depth += 1;
            }
            Event::End(pulldown_cmark::TagEnd::Image) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    found
}

/// The image format of a file by its first bytes (`png`, `jpeg`, `gif`, `bmp`, `webp`).
pub fn image_format(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpeg")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("gif")
    } else if head.starts_with(b"BM") && head.len() >= 14 {
        Some("bmp")
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// The first bytes of a file (for [`image_format`]).
pub fn file_head(p: &Path) -> Vec<u8> {
    use std::io::Read;
    let mut b = vec![0u8; 16];
    let n = std::fs::File::open(p)
        .and_then(|mut f| f.read(&mut b))
        .unwrap_or(0);
    b.truncate(n);
    b
}

/// A path for comparing files: `.` parts dropped, letters folded (Windows paths ignore case).
fn normal(p: &Path) -> String {
    let mut out = PathBuf::new();
    for c in p.components() {
        if !matches!(c, std::path::Component::CurDir) {
            out.push(c.as_os_str());
        }
    }
    out.to_string_lossy().to_lowercase()
}

/// The Markdown files of `dir` (not its subfolders).
pub fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && crate::markdown::is_markdown_path(p))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The largest Markdown file read when looking for references (bigger ones are skipped).
const SCAN_LIMIT: u64 = 16 << 20;

/// The local files the Markdown file `doc` refers to (normalised for comparison).
fn referenced(doc: &Path) -> Vec<String> {
    let Ok(m) = std::fs::metadata(doc) else {
        return Vec::new();
    };
    if m.len() > SCAN_LIMIT {
        return Vec::new();
    }
    let Ok(bytes) = std::fs::read(doc) else {
        return Vec::new();
    };
    let d = crate::file::decode(&bytes, crate::file::system_encoding());
    references(&d.text)
        .iter()
        .filter_map(|r| resolve(doc, &r.url))
        .map(|p| normal(&p))
        .collect()
}

/// The files in `assets` next to `doc` (and its subfolders, four levels deep) that no Markdown
/// file in the document's folder refers to. Only listed: nothing is deleted.
pub fn orphans(doc: &Path) -> Vec<PathBuf> {
    let Some(dir) = doc.parent() else {
        return Vec::new();
    };
    let used: std::collections::HashSet<String> = markdown_files(dir)
        .iter()
        .chain(std::iter::once(&doc.to_path_buf()))
        .flat_map(|m| referenced(m))
        .collect();
    let mut out = Vec::new();
    let mut stack = vec![(dir.join(ASSET_DIR), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth < 4 {
                    stack.push((p, depth + 1));
                }
            } else if !used.contains(&normal(&p)) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The other Markdown files of the document's folder that refer to `asset`.
pub fn shared_by(doc: &Path, asset: &Path) -> Vec<PathBuf> {
    let Some(dir) = doc.parent() else {
        return Vec::new();
    };
    let me = normal(doc);
    let target = normal(asset);
    markdown_files(dir)
        .into_iter()
        .filter(|m| normal(m) != me && referenced(m).contains(&target))
        .collect()
}

/// Why an asset cannot be renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameError {
    /// The document does not refer to it (or it is not a local file in its folder).
    NotReferenced(String),
    /// The file does not exist.
    Missing(PathBuf),
    /// The new name is not a plain file name.
    BadName(String),
    /// A file with the new name exists (it is never replaced).
    Exists(PathBuf),
    /// Other Markdown files of the folder refer to it too.
    Shared(Vec<PathBuf>),
    /// A reference could not be located exactly in the text.
    Unlocated(usize),
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenameError::NotReferenced(r) => {
                write!(f, "the document has no image reference to `{r}`")
            }
            RenameError::Missing(p) => write!(f, "{} does not exist", p.display()),
            RenameError::BadName(n) => write!(f, "`{n}` is not a plain file name"),
            RenameError::Exists(p) => {
                write!(f, "{} exists already; it is never replaced", p.display())
            }
            RenameError::Shared(v) => write!(
                f,
                "other documents refer to it too: {}",
                v.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            RenameError::Unlocated(line) => write!(
                f,
                "the reference on line {} cannot be located exactly; rename it by hand",
                line + 1
            ),
        }
    }
}

/// A rename to carry out: the file move and the document's new text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlan {
    pub from: PathBuf,
    pub to: PathBuf,
    /// The new reference (relative, forward slashes).
    pub reference: String,
    /// The lines (0-based) whose references change.
    pub lines: Vec<usize>,
    pub text: String,
    /// The replacements in bytes of the original text, ascending (for an open document).
    pub edits: Vec<(Range<usize>, String)>,
}

/// Plans renaming the asset `old` (a reference as written in the document, or a file name in
/// its `assets` folder) of the document at `doc` with text `text` to the file name `new` (the
/// old extension is kept when `new` has none). Changes nothing.
pub fn plan_rename(
    doc: &Path,
    text: &str,
    old: &str,
    new: &str,
) -> Result<RenamePlan, RenameError> {
    let refs = references(text);
    let old_path = resolve(doc, old)
        .filter(|p| p.is_file())
        .or_else(|| resolve(doc, &format!("{ASSET_DIR}/{old}")))
        .ok_or_else(|| RenameError::NotReferenced(old.to_string()))?;
    let key = normal(&old_path);
    let mine: Vec<&Reference> = refs
        .iter()
        .filter(|r| resolve(doc, &r.url).is_some_and(|p| normal(&p) == key))
        .collect();
    if mine.is_empty() {
        return Err(RenameError::NotReferenced(old.to_string()));
    }
    if !old_path.is_file() {
        return Err(RenameError::Missing(old_path));
    }
    let bad = new.is_empty()
        || new.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || new.starts_with('.')
        || new.ends_with(['.', ' '])
        || new.chars().any(char::is_control);
    if bad {
        return Err(RenameError::BadName(new.to_string()));
    }
    let mut name = new.to_string();
    if Path::new(new).extension().is_none()
        && let Some(ext) = old_path.extension()
    {
        name = format!("{new}.{}", ext.to_string_lossy());
    }
    let to = old_path.with_file_name(&name);
    if normal(&to) == key {
        return Err(RenameError::BadName(new.to_string()));
    }
    if to.exists() {
        return Err(RenameError::Exists(to));
    }
    let others = shared_by(doc, &old_path);
    if !others.is_empty() {
        return Err(RenameError::Shared(others));
    }
    // The new reference: the old one as written with its last part replaced.
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut lines = Vec::new();
    let mut reference = String::new();
    for r in mine {
        let Some(span) = r.span.clone() else {
            // An image that shares a definition already rewritten is fine; anything else is not.
            if refs.iter().any(|o| o.span.is_some() && o.url == r.url) {
                continue;
            }
            return Err(RenameError::Unlocated(r.line));
        };
        let written = &text[span.clone()];
        let cut = written.rfind('/').map_or(0, |i| i + 1);
        let encoded: String = name
            .replace('%', "%25")
            .replace(' ', "%20")
            .replace('(', "%28")
            .replace(')', "%29");
        reference = format!("{}{encoded}", &written[..cut]);
        edits.push((span, reference.clone()));
        lines.push(r.line);
    }
    edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    let mut out = text.to_string();
    for (r, s) in &edits {
        out.replace_range(r.clone(), s);
    }
    edits.reverse();
    lines.sort_unstable();
    lines.dedup();
    Ok(RenamePlan {
        from: old_path,
        to,
        reference,
        lines,
        text: out,
        edits,
    })
}

/// Moves `from` to `to`, failing when `to` exists (never replacing a file).
pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(windows)]
    #[allow(unsafe_code)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let w = |p: &Path| -> Vec<u16> {
            p.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        };
        let (a, b) = (w(from), w(to));
        // SAFETY: NUL-terminated paths that live across the call; flags 0 = never replace.
        if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-assets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn names_are_portable() {
        assert_eq!(portable_stem("ideas"), "ideas");
        assert_eq!(portable_stem("My notes (draft) v2"), "My-notes-draft-v2");
        assert_eq!(portable_stem("회의록 11월"), "회의록-11월");
        assert_eq!(portable_stem("..."), "image");
        assert_eq!(portable_stem(""), "image");
    }

    #[test]
    fn assets_get_the_next_free_number_and_never_overwrite() {
        let d = temp("write");
        let doc = d.join("ideas.md");
        let a = write_new(&doc, "png", (2026, 11, 22), b"one").unwrap();
        assert_eq!(a.reference, "assets/ideas-20261122-001.png");
        assert_eq!(std::fs::read(&a.path).unwrap(), b"one");
        // A file that already has the next name (a copy from elsewhere) is skipped, not replaced.
        std::fs::write(d.join("assets").join("ideas-20261122-002.png"), b"keep").unwrap();
        let b = write_new(&doc, "PNG", (2026, 11, 22), b"two").unwrap();
        assert_eq!(b.reference, "assets/ideas-20261122-003.png");
        assert_eq!(
            std::fs::read(d.join("assets").join("ideas-20261122-002.png")).unwrap(),
            b"keep"
        );
        assert_eq!(
            b.markdown("Screen [shot]"),
            "![Screen shot](assets/ideas-20261122-003.png)"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn references_resolve_inside_the_documents_folder_only() {
        let doc = Path::new(r"C:\notes\ideas.md");
        assert_eq!(
            resolve(doc, "assets/ideas-20261122-001.png"),
            Some(PathBuf::from(r"C:\notes\assets\ideas-20261122-001.png"))
        );
        assert_eq!(
            resolve(doc, "./assets/my%20shot.png?raw=1"),
            Some(PathBuf::from(r"C:\notes\assets\my shot.png"))
        );
        for no in [
            "../secret.png",
            "assets/../../x.png",
            "C:/Windows/x.png",
            "/etc/x.png",
            "https://example.com/x.png",
            "data:image/png;base64,AAAA",
        ] {
            assert_eq!(resolve(doc, no), None, "{no}");
        }
    }

    #[test]
    fn references_carry_the_exact_bytes_of_their_destinations() {
        let text = "# A\n\n![one](assets/a.png) and ![two](<assets/b c.png> \"t\")\n\n![three][d]\n\n[d]: ./assets/d%20e.png\n\n![w](https://x.org/r.png) ![p](assets/p\\(1\\).png)\n";
        let refs = references(text);
        let urls: Vec<&str> = refs.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "assets/a.png",
                "assets/b c.png",
                "./assets/d%20e.png",
                "https://x.org/r.png",
                "assets/p(1).png"
            ]
        );
        let written: Vec<&str> = refs
            .iter()
            .map(|r| &text[r.span.clone().unwrap()])
            .collect();
        assert_eq!(
            written,
            [
                "assets/a.png",
                "assets/b c.png",
                "./assets/d%20e.png",
                "https://x.org/r.png",
                "assets/p\\(1\\).png"
            ]
        );
        assert_eq!(
            refs.iter().map(|r| r.line).collect::<Vec<_>>(),
            [2, 2, 4, 8, 8]
        );
        assert_eq!(
            reference_at("see ![x](assets/x.png) here", 6).as_deref(),
            Some("assets/x.png")
        );
        assert_eq!(reference_at("see ![x](assets/x.png) here", 1), None);
    }

    #[test]
    fn orphans_are_assets_no_markdown_file_of_the_folder_refers_to() {
        let d = temp("orphans");
        std::fs::create_dir_all(d.join("assets").join("sub")).unwrap();
        for f in ["a.png", "b.png", "shared.png", "sub/c.png", "gone.png"] {
            std::fs::write(d.join("assets").join(f), b"x").unwrap();
        }
        std::fs::write(
            d.join("one.md"),
            "![a](assets/a.png)\n![s](./ASSETS/shared.png)\n![m](assets/missing.png)\n",
        )
        .unwrap();
        std::fs::write(
            d.join("two.markdown"),
            "![s](assets/shared.png) ![c](assets/sub/c.png)\n",
        )
        .unwrap();
        std::fs::write(d.join("notes.txt"), "![b](assets/b.png)\n").unwrap();
        let names: Vec<String> = orphans(&d.join("one.md"))
            .iter()
            .map(|p| {
                p.strip_prefix(d.join("assets"))
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(
            names,
            ["b.png", "gone.png"],
            "a text file's references do not count"
        );
        assert_eq!(
            shared_by(&d.join("one.md"), &d.join("assets").join("shared.png")),
            vec![d.join("two.markdown")]
        );
        assert!(shared_by(&d.join("one.md"), &d.join("assets").join("a.png")).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_rename_moves_the_file_and_rewrites_only_its_references() {
        let d = temp("rename");
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets").join("shot-1.png"), b"one").unwrap();
        std::fs::write(d.join("assets").join("taken.png"), b"keep").unwrap();
        let doc = d.join("n.md");
        let text = "![a](assets/shot-1.png)\r\ntext assets/shot-1.png stays\r\n![b](./assets/shot-1.png \"t\")\r\n";
        std::fs::write(&doc, text).unwrap();
        let p = plan_rename(&doc, text, "assets/shot-1.png", "login screen").unwrap();
        assert_eq!(p.to, d.join("assets").join("login screen.png"));
        assert_eq!(
            p.text,
            "![a](assets/login%20screen.png)\r\ntext assets/shot-1.png stays\r\n![b](./assets/login%20screen.png \"t\")\r\n"
        );
        assert_eq!(p.lines, [0, 2]);
        // Refusals change nothing.
        assert_eq!(
            plan_rename(&doc, text, "shot-1.png", "taken").unwrap_err(),
            RenameError::Exists(d.join("assets").join("taken.png"))
        );
        assert!(matches!(
            plan_rename(&doc, text, "assets/shot-1.png", "../x").unwrap_err(),
            RenameError::BadName(_)
        ));
        assert!(matches!(
            plan_rename(&doc, text, "assets/other.png", "x").unwrap_err(),
            RenameError::NotReferenced(_)
        ));
        std::fs::write(d.join("m.md"), "![](assets/shot-1.png)").unwrap();
        assert!(matches!(
            plan_rename(&doc, text, "shot-1.png", "y").unwrap_err(),
            RenameError::Shared(_)
        ));
        std::fs::remove_file(d.join("m.md")).unwrap();
        // The move never replaces a file.
        let e = move_no_replace(
            &d.join("assets").join("shot-1.png"),
            &d.join("assets").join("taken.png"),
        )
        .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{e}");
        assert_eq!(
            std::fs::read(d.join("assets").join("taken.png")).unwrap(),
            b"keep"
        );
        move_no_replace(&p.from, &p.to).unwrap();
        assert_eq!(std::fs::read(&p.to).unwrap(), b"one");
        assert!(!p.from.exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
