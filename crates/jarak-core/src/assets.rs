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

use std::io::{self, Write};
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
    Some(doc.parent()?.join(rel))
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
            Some(PathBuf::from(r"C:\notes\.\assets\my shot.png"))
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
}
