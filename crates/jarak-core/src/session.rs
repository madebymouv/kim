//! The session (`JARAK-FR-062`, `JARAK-FR-028`, Step 93): what a window shows, written at exit
//! and read at start, and the recent files.
//!
//! The file is `%LOCALAPPDATA%\mouv\jarak\session.json` (per machine, not roaming), JSON with a
//! schema (`docs/contracts/schemas/jarak-session-v1.schema.json`), written atomically. Untitled
//! documents keep their text in files next to it (`recovery\untitled-<n>.txt`); the session names
//! them. A session written by a newer Jarak (another `schema_version`) is not read.
//!
//! Step 104: before a save replaces a good session file, that file is kept as `session.json.bak`;
//! a damaged session file falls back to it ([`Session::load_or_backup`]), and the damaged file is
//! kept as `session.json.broken` (never deleted, never overwritten by the next save).

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// The schema name and version this Jarak writes and reads.
pub const SCHEMA: &str = "jarak.session";
pub const SCHEMA_VERSION: u32 = 1;
/// The most recent files kept.
pub const RECENT_MAX: usize = 20;

/// Which views show a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Views {
    #[default]
    Main,
    Second,
    Both,
}

/// How a document is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Shown {
    #[default]
    Source,
    Preview,
    Split,
}

/// One open document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Doc {
    /// The file (`None`: an untitled document).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// An untitled document's text file in the recovery folder (a file name, no folders).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untitled: Option<String>,
    /// The caret (a character position; clamped to the text when restored).
    #[serde(default)]
    pub caret: usize,
    /// The first line on screen.
    #[serde(default)]
    pub top_line: usize,
    #[serde(default)]
    pub shown: Shown,
    #[serde(default)]
    pub views: Views,
    /// Step 110 (VIEWS-08): the tab's colour (0 purple, 1 green, 2 amber, 3 red, 4 blue).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u8>,
}

/// The second view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Second {
    /// The documents it shows, in tab order (indices into [`Session::documents`]).
    pub order: Vec<usize>,
    /// The active one (an index into [`Session::documents`]).
    pub active: usize,
    /// Side by side (`true`) or one above the other.
    #[serde(default = "yes")]
    pub vertical: bool,
    /// It had the focus.
    #[serde(default)]
    pub focus: bool,
}

fn yes() -> bool {
    true
}

/// A window's session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub schema: String,
    pub schema_version: u32,
    #[serde(default)]
    pub documents: Vec<Doc>,
    /// The main view's active document (an index into `documents`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<Second>,
    /// The folder shown as a workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<PathBuf>,
    /// Recently closed and opened files, newest first.
    #[serde(default)]
    pub recent: Vec<PathBuf>,
    /// Step 94: the find and replace boxes' histories, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub find_history: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replace_history: Vec<String>,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            schema: SCHEMA.to_string(),
            schema_version: SCHEMA_VERSION,
            documents: Vec::new(),
            active: None,
            second: None,
            workspace: None,
            recent: Vec::new(),
            find_history: Vec::new(),
            replace_history: Vec::new(),
        }
    }
}

/// Why a session file was not read.
#[derive(Debug)]
pub enum LoadError {
    Missing,
    Io(io::Error),
    Invalid(String),
    /// Another schema or version (a newer Jarak): left as it is.
    Version(String, u32),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Missing => write!(f, "no session yet"),
            LoadError::Io(e) => write!(f, "the session cannot be read: {e}"),
            LoadError::Invalid(e) => write!(f, "the session file is not valid: {e}"),
            LoadError::Version(s, v) => {
                write!(f, "the session was written by another Jarak ({s} {v})")
            }
        }
    }
}

impl Session {
    /// Reads `path` (and checks it: indices in range, untitled names without folders).
    pub fn load(path: &Path) -> Result<Session, LoadError> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(LoadError::Missing),
            Err(e) => return Err(LoadError::Io(e)),
        };
        let v: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| LoadError::Invalid(e.to_string()))?;
        let schema = v
            .get("schema")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let version = v
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32;
        if schema != SCHEMA || version != SCHEMA_VERSION {
            return Err(LoadError::Version(schema, version));
        }
        let s: Session =
            serde_json::from_value(v).map_err(|e| LoadError::Invalid(e.to_string()))?;
        s.check().map_err(LoadError::Invalid)?;
        Ok(s)
    }

    /// Step 104: reads the session; a damaged file (unreadable or not a valid session) falls back
    /// to the backup the last save kept, and is kept as `session.json.broken`. The note says what
    /// happened, for the window to show. A session of a newer Jarak is left as it is.
    pub fn load_or_backup(path: &Path) -> (Result<Session, LoadError>, Option<String>) {
        let backup = sibling(path, "bak");
        match Session::load(path) {
            Ok(s) => (Ok(s), None),
            Err(LoadError::Missing) => match Session::load(&backup) {
                Ok(s) => (
                    Ok(s),
                    Some("The session file was missing; the backup was opened.".into()),
                ),
                Err(_) => (Err(LoadError::Missing), None),
            },
            Err(e @ (LoadError::Io(_) | LoadError::Invalid(_))) => {
                let broken = sibling(path, "broken");
                let _ = std::fs::remove_file(&broken);
                let kept = std::fs::rename(path, &broken).is_ok();
                let where_ = if kept {
                    format!("kept as {}", broken.display())
                } else {
                    "left where it is".to_string()
                };
                match Session::load(&backup) {
                    Ok(s) => (
                        Ok(s),
                        Some(format!(
                            "{e}; the previous session was opened instead (the damaged file is {where_})."
                        )),
                    ),
                    Err(_) => (
                        Err(e),
                        Some(format!("The damaged session file is {where_}.")),
                    ),
                }
            }
            Err(e @ LoadError::Version(..)) => (Err(e), None),
        }
    }

    /// The indices and names point where they should.
    pub fn check(&self) -> Result<(), String> {
        let n = self.documents.len();
        if self.active.is_some_and(|a| a >= n) {
            return Err("the active document is not in the list".into());
        }
        if let Some(s) = &self.second
            && (s.order.iter().any(|i| *i >= n)
                || (!s.order.is_empty() && !s.order.contains(&s.active)))
        {
            return Err("the second view names a document that is not in the list".into());
        }
        for d in &self.documents {
            if let Some(u) = &d.untitled
                && (u.is_empty() || u.contains(['/', '\\', ':']) || u.starts_with('.'))
            {
                return Err(format!("not a plain file name: {u}"));
            }
            if d.path.is_none() && d.untitled.is_none() {
                return Err("a document without a file or an untitled text".into());
            }
        }
        Ok(())
    }

    /// Writes the session atomically (the folder is created when needed); a good session file it
    /// replaces is kept as the backup first (Step 104).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        if Session::load(path).is_ok() {
            let _ = std::fs::copy(path, sibling(path, "bak"));
        }
        let mut text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        text.push('\n');
        crate::file::save_atomic(path, text.as_bytes())
    }

    /// Puts `p` first in the recent files (once, at most [`RECENT_MAX`]).
    pub fn remember(recent: &mut Vec<PathBuf>, p: &Path) {
        let key = p.to_string_lossy().to_lowercase();
        recent.retain(|r| r.to_string_lossy().to_lowercase() != key);
        recent.insert(0, p.to_path_buf());
        recent.truncate(RECENT_MAX);
    }
}

/// `session.json` → `session.json.<ext>` next to it.
fn sibling(path: &Path, ext: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(ext);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-session-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_session_round_trips_through_its_file() {
        let d = temp("round");
        let s = Session {
            documents: vec![
                Doc {
                    path: Some(PathBuf::from(r"C:\notes\a.md")),
                    caret: 12,
                    top_line: 3,
                    shown: Shown::Preview,
                    views: Views::Both,
                    ..Doc::default()
                },
                Doc {
                    untitled: Some("untitled-1.txt".into()),
                    views: Views::Second,
                    // Step 110 (VIEWS-08): the tab's colour.
                    color: Some(3),
                    ..Doc::default()
                },
            ],
            active: Some(0),
            second: Some(Second {
                order: vec![0, 1],
                active: 1,
                vertical: false,
                focus: true,
            }),
            workspace: Some(PathBuf::from(r"C:\notes")),
            recent: vec![PathBuf::from(r"C:\notes\old.txt")],
            ..Session::default()
        };
        let f = d.join("sub").join("session.json");
        s.save(&f).unwrap();
        assert_eq!(Session::load(&f).unwrap(), s);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Step 104: a save keeps the good file it replaces; a damaged file falls back to that backup
    /// and is kept as `.broken`; without a backup the damaged file is still kept.
    #[test]
    fn a_damaged_session_falls_back_to_the_backup_and_is_kept() {
        let d = temp("backup");
        let f = d.join("session.json");
        let first = Session {
            recent: vec![PathBuf::from(
                r"C:
otesirst.md",
            )],
            ..Session::default()
        };
        first.save(&f).unwrap();
        assert!(
            !d.join("session.json.bak").exists(),
            "nothing to back up yet"
        );
        let second = Session {
            recent: vec![PathBuf::from(
                r"C:
otes\second.md",
            )],
            ..Session::default()
        };
        second.save(&f).unwrap();
        assert_eq!(Session::load(&d.join("session.json.bak")).unwrap(), first);
        // A crash mid-write elsewhere, a disk error: the file is damaged.
        std::fs::write(&f, b"{\"schema\":\"jarak.ses").unwrap();
        let (s, note) = Session::load_or_backup(&f);
        assert_eq!(s.unwrap(), first, "the backup is opened");
        assert!(note.unwrap().contains("session.json.broken"));
        assert_eq!(
            std::fs::read(d.join("session.json.broken")).unwrap(),
            b"{\"schema\":\"jarak.ses",
            "the damaged file is kept as it was"
        );
        assert!(!f.exists());
        // No backup either: the damaged file is kept, nothing is opened.
        std::fs::remove_file(d.join("session.json.bak")).unwrap();
        std::fs::write(&f, b"not json").unwrap();
        let (s, note) = Session::load_or_backup(&f);
        assert!(matches!(s, Err(LoadError::Invalid(_))));
        assert!(note.unwrap().contains("broken"));
        assert_eq!(
            std::fs::read(d.join("session.json.broken")).unwrap(),
            b"not json"
        );
        // A good file is read as it is, with no note; a newer Jarak's file is left alone.
        second.save(&f).unwrap();
        assert!(matches!(Session::load_or_backup(&f), (Ok(_), None)));
        std::fs::write(&f, r#"{"schema":"jarak.session","schema_version":9}"#).unwrap();
        assert!(matches!(
            Session::load_or_backup(&f),
            (Err(LoadError::Version(_, 9)), None)
        ));
        assert!(f.exists(), "a newer session stays where it is");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn broken_foreign_and_newer_sessions_are_not_read() {
        let d = temp("bad");
        let f = d.join("session.json");
        assert!(matches!(Session::load(&f), Err(LoadError::Missing)));
        std::fs::write(&f, "{not json").unwrap();
        assert!(matches!(Session::load(&f), Err(LoadError::Invalid(_))));
        std::fs::write(
            &f,
            r#"{"schema":"jarak.session","schema_version":2,"documents":[]}"#,
        )
        .unwrap();
        assert!(matches!(Session::load(&f), Err(LoadError::Version(_, 2))));
        std::fs::write(
            &f,
            r#"{"schema":"jarak.session","schema_version":1,"documents":[],"active":3}"#,
        )
        .unwrap();
        assert!(matches!(Session::load(&f), Err(LoadError::Invalid(_))));
        std::fs::write(&f, r#"{"schema":"jarak.session","schema_version":1,"documents":[{"untitled":"..\\x.txt"}]}"#).unwrap();
        assert!(matches!(Session::load(&f), Err(LoadError::Invalid(_))));
        std::fs::write(
            &f,
            r#"{"schema":"jarak.session","schema_version":1,"surprise":1}"#,
        )
        .unwrap();
        assert!(
            matches!(Session::load(&f), Err(LoadError::Invalid(_))),
            "unknown keys are refused"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_golden_example_is_a_session_jarak_reads() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/examples/jarak-session-v1/two-views.json");
        let s = Session::load(&p).unwrap();
        assert_eq!(s.documents.len(), 3);
        assert_eq!(s.documents[0].views, Views::Both);
        assert_eq!(s.second.as_ref().unwrap().order, vec![0, 2]);
        assert_eq!(s.documents[2].untitled.as_deref(), Some("untitled-1.txt"));
    }

    #[test]
    fn recent_files_are_newest_first_once_and_bounded() {
        let mut r = Vec::new();
        for i in 0..25 {
            Session::remember(&mut r, Path::new(&format!(r"C:\f\{i}.txt")));
        }
        Session::remember(&mut r, Path::new(r"C:\F\20.TXT"));
        assert_eq!(r.len(), RECENT_MAX);
        assert_eq!(r[0], PathBuf::from(r"C:\F\20.TXT"));
        assert_eq!(
            r.iter()
                .filter(|p| p.to_string_lossy().to_lowercase().ends_with("20.txt"))
                .count(),
            1
        );
    }
}
