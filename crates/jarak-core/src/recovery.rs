//! Recovery snapshots (Step 104, `JARAK-FR-121`, REL-02, JRK-16): while a window runs, the text of
//! its documents with unsaved changes — untitled ones too — is copied now and then into a folder of
//! its own under `%LOCALAPPDATA%\mouv\jarak\recovery\`. The documents' own files are never written.
//! A normal exit removes the folder; after a crash it stays, and the next start offers the
//! documents back with their text intact.
//!
//! - A window's folder is `w-<pid>-<start>` (its process id and start time). Its `manifest.json`
//!   (schema `jarak.recovery` 1) names the owner and one entry per document; each text is a UTF-8
//!   file `doc-<id>.txt`. Texts are written first and the manifest last, each through a temporary
//!   file and a rename, so a manifest never names a text that is not complete.
//! - A folder whose owner no longer runs (or whose process id now belongs to another process: the
//!   start time differs) is an orphan. Restoring or discarding one first claims it by renaming it
//!   to `claimed-<pid>-<start>-<n>` for the claiming window, so two windows starting together
//!   never both take it; a running window's folder is never touched. A claimed folder whose
//!   claiming window is gone is an orphan again.
//! - Bounded: at most [`Limits::docs`] documents and [`Limits::bytes`] of text per window; the
//!   documents beyond that (the largest first) are left out and named.

use crate::document::Eol;
use crate::file::{Format, TextEncoding};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "jarak.recovery";
pub const SCHEMA_VERSION: u32 = 1;
const MANIFEST: &str = "manifest.json";

/// The window that wrote a folder: its process id and the process's start time (100 ns units
/// since 1601, as Windows gives it), which tells a reused process id apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub pid: u32,
    pub started: u64,
}

/// One document's snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The window's document id (stable while it runs).
    pub id: u64,
    /// The document's file (`None`: untitled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// The name its tab showed.
    pub title: String,
    /// The text's file in the folder (a plain file name).
    pub text: String,
    /// [`TextEncoding::name`] and the byte-order mark the document saves with.
    pub encoding: String,
    #[serde(default)]
    pub bom: bool,
    /// `crlf`, `lf` or `cr`: the line ending new lines use.
    pub eol: String,
    /// The caret (a character position).
    #[serde(default)]
    pub caret: usize,
    /// The text's length in characters.
    pub chars: usize,
    /// When the snapshot was taken (seconds since 1970).
    pub saved_at: u64,
    /// The file as the document had last seen it (size; last write in ms since 1970): a restore
    /// can tell whether another program changed it since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_len: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_modified_ms: Option<u64>,
}

impl Entry {
    /// The format the document saved with (UTF-8 for a name this Jarak does not know).
    pub fn format(&self) -> Format {
        Format {
            encoding: TextEncoding::from_name(&self.encoding).unwrap_or(TextEncoding::Utf8),
            bom: self.bom,
        }
    }

    pub fn eol(&self) -> Eol {
        match self.eol.as_str() {
            "lf" => Eol::Lf,
            "cr" => Eol::Cr,
            _ => Eol::CrLf,
        }
    }

    /// The name for a line ending in [`Entry::eol`].
    pub fn eol_name(eol: Eol) -> &'static str {
        match eol {
            Eol::CrLf => "crlf",
            Eol::Lf => "lf",
            Eol::Cr => "cr",
        }
    }
}

/// A folder's manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub schema_version: u32,
    pub owner: Owner,
    pub entries: Vec<Entry>,
}

/// What a window hands over to be written: the entry, and the text — `None` when it did not
/// change since the last write (the file already there is kept).
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub entry: Entry,
    pub text: Option<Rope>,
}

/// How much one window keeps.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub docs: usize,
    pub bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            docs: 100,
            bytes: 512 * 1024 * 1024,
        }
    }
}

/// The result of a write: the documents kept, and the titles of those left out by the limits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Written {
    pub kept: Vec<u64>,
    pub left_out: Vec<String>,
}

/// A window's own folder.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
    owner: Owner,
    pub limits: Limits,
}

impl Store {
    pub fn new(root: &Path, owner: Owner) -> Store {
        Store {
            root: root.to_path_buf(),
            owner,
            limits: Limits::default(),
        }
    }

    pub fn owner(&self) -> Owner {
        self.owner
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `w-<pid>-<start>` under the recovery folder.
    pub fn dir(&self) -> PathBuf {
        self.root
            .join(format!("w-{}-{}", self.owner.pid, self.owner.started))
    }

    /// Writes these snapshots and only these: texts of documents no longer listed go. An empty
    /// list removes the folder (nothing to recover).
    pub fn write(&self, docs: &[Snapshot]) -> io::Result<Written> {
        let dir = self.dir();
        if docs.is_empty() {
            self.clear()?;
            return Ok(Written::default());
        }
        std::fs::create_dir_all(&dir)?;
        // The limits: the smallest documents first, so one huge text does not push out the rest.
        let mut order: Vec<usize> = (0..docs.len()).collect();
        let size = |s: &Snapshot| {
            s.text.as_ref().map_or_else(
                || {
                    std::fs::metadata(dir.join(&s.entry.text))
                        .map(|m| m.len())
                        .unwrap_or(0)
                },
                |t| t.len_bytes() as u64,
            )
        };
        order.sort_by_key(|i| size(&docs[*i]));
        let mut out = Written::default();
        let mut total = 0u64;
        let mut entries = Vec::new();
        for i in order {
            let s = &docs[i];
            if !plain_name(&s.entry.text) {
                return Err(io::Error::other(format!(
                    "not a plain file name: {}",
                    s.entry.text
                )));
            }
            let n = size(s);
            if entries.len() >= self.limits.docs || total + n > self.limits.bytes {
                out.left_out.push(s.entry.title.clone());
                continue;
            }
            match &s.text {
                Some(t) => write_rope(&dir.join(&s.entry.text), t)?,
                None if dir.join(&s.entry.text).is_file() => {}
                // Asked to keep a text that is not there: nothing to recover for it.
                None => continue,
            }
            total += n;
            out.kept.push(s.entry.id);
            entries.push(s.entry.clone());
        }
        entries.sort_by_key(|e| e.id);
        out.kept.sort_unstable();
        let m = Manifest {
            schema: SCHEMA.into(),
            schema_version: SCHEMA_VERSION,
            owner: self.owner,
            entries,
        };
        let mut json = serde_json::to_string_pretty(&m).map_err(io::Error::other)?;
        json.push('\n');
        write_bytes(&dir.join(MANIFEST), json.as_bytes())?;
        // Texts no longer named go (the manifest no longer points at them).
        let keep: Vec<&str> = m.entries.iter().map(|e| e.text.as_str()).collect();
        for e in std::fs::read_dir(&dir)?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name != MANIFEST && !keep.contains(&name.as_str()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
        Ok(out)
    }

    /// Removes the window's folder (a normal exit, or nothing left to recover).
    pub fn clear(&self) -> io::Result<()> {
        match std::fs::remove_dir_all(self.dir()) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// A folder left by a window that no longer runs.
#[derive(Debug, Clone)]
pub struct Orphan {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

/// The orphaned folders under `root`, oldest snapshot first. `alive` tells whether a window still
/// runs ([`owner_alive`] on Windows). Folders without a readable manifest of this schema, and
/// anything else in the folder, are left as they are.
pub fn orphans(root: &Path, alive: &dyn Fn(&Owner) -> bool) -> Vec<Orphan> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let holder = if let Some(rest) = name.strip_prefix("w-") {
            parse_owner(rest)
        } else if let Some(rest) = name.strip_prefix("claimed-") {
            parse_owner(rest)
        } else {
            continue;
        };
        let Some(holder) = holder else { continue };
        if alive(&holder) {
            continue;
        }
        let Ok(m) = read_manifest(&e.path()) else {
            continue;
        };
        if m.entries.is_empty() {
            continue;
        }
        out.push(Orphan {
            dir: e.path(),
            manifest: m,
        });
    }
    out.sort_by_key(|o| o.manifest.entries.iter().map(|e| e.saved_at).max());
    out
}

impl Orphan {
    /// Takes the folder for window `by` (a rename): an error when another window took it first.
    pub fn claim(&self, by: &Owner) -> io::Result<Orphan> {
        let root = self.dir.parent().unwrap_or(Path::new("."));
        for n in 0..1000 {
            let to = root.join(format!("claimed-{}-{}-{n}", by.pid, by.started));
            if to.exists() {
                continue;
            }
            std::fs::rename(&self.dir, &to)?;
            return Ok(Orphan {
                dir: to,
                manifest: self.manifest.clone(),
            });
        }
        Err(io::Error::other(
            "no free name to claim the recovery folder",
        ))
    }

    /// The text of entry `e` (its file name is checked to stay in the folder).
    pub fn text(&self, e: &Entry) -> io::Result<String> {
        if !plain_name(&e.text) {
            return Err(io::Error::other(format!(
                "not a plain file name: {}",
                e.text
            )));
        }
        let bytes = std::fs::read(self.dir.join(&e.text))?;
        String::from_utf8(bytes).map_err(io::Error::other)
    }

    /// Removes the folder (restored, or discarded by the user).
    pub fn remove(self) -> io::Result<()> {
        std::fs::remove_dir_all(&self.dir)
    }
}

/// `<pid>-<start>` (and a claim's `-<n>`).
fn parse_owner(rest: &str) -> Option<Owner> {
    let mut it = rest.split('-');
    let pid = it.next()?.parse().ok()?;
    let started = it.next()?.parse().ok()?;
    Some(Owner { pid, started })
}

fn read_manifest(dir: &Path) -> io::Result<Manifest> {
    let bytes = std::fs::read(dir.join(MANIFEST))?;
    let m: Manifest = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if m.schema != SCHEMA || m.schema_version != SCHEMA_VERSION {
        return Err(io::Error::other("another recovery schema"));
    }
    if m.entries.iter().any(|e| !plain_name(&e.text)) {
        return Err(io::Error::other("a text outside the folder"));
    }
    Ok(m)
}

fn plain_name(n: &str) -> bool {
    !n.is_empty() && !n.contains(['/', '\\', ':']) && !n.starts_with('.') && n != MANIFEST
}

/// Writes through a temporary file and a rename (the old file stays whole until the new one is).
fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn write_rope(path: &Path, text: &Rope) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = io::BufWriter::new(std::fs::File::create(&tmp)?);
        for chunk in text.chunks() {
            f.write_all(chunk.as_bytes())?;
        }
        f.flush()?;
        f.get_ref().sync_all()?;
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Seconds since 1970 now.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Milliseconds since 1970 of a file time.
pub fn millis(t: std::time::SystemTime) -> Option<u64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
}

/// This process as a recovery folder's owner.
#[cfg(windows)]
pub fn current_owner() -> Owner {
    let pid = std::process::id();
    Owner {
        pid,
        started: process_started(pid).unwrap_or(0),
    }
}

/// The window that wrote a folder still runs: a live process with that id and start time.
#[cfg(windows)]
pub fn owner_alive(o: &Owner) -> bool {
    process_started(o.pid) == Some(o.started)
}

/// The start time of a running process (`None`: no such process, or it has exited).
#[cfg(windows)]
#[allow(unsafe_code)]
fn process_started(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: OpenProcess only opens a handle (null when it cannot); it is closed below.
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if h.is_null() {
        return None;
    }
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    let mut code: u32 = 0;
    // SAFETY: `h` is a valid process handle; each pointer is to a local of the right type.
    let (times, status) = unsafe {
        (
            GetProcessTimes(
                h,
                &raw mut created,
                &raw mut exited,
                &raw mut kernel,
                &raw mut user,
            ),
            GetExitCodeProcess(h, &raw mut code),
        )
    };
    // SAFETY: the handle came from OpenProcess and is closed once.
    unsafe { CloseHandle(h) };
    #[allow(clippy::cast_sign_loss)]
    let running = status != 0 && code == STILL_ACTIVE as u32;
    (times != 0 && running)
        .then(|| (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-recovery-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn entry(id: u64, path: Option<&str>, text: &str) -> Snapshot {
        Snapshot {
            entry: Entry {
                id,
                path: path.map(PathBuf::from),
                title: path.unwrap_or("Untitled").to_string(),
                text: format!("doc-{id}.txt"),
                encoding: "euc-kr".into(),
                bom: false,
                eol: "crlf".into(),
                caret: 3,
                chars: text.chars().count(),
                saved_at: now_secs(),
                disk_len: Some(7),
                disk_modified_ms: Some(1),
            },
            text: Some(Rope::from_str(text)),
        }
    }

    const ME: Owner = Owner {
        pid: 4242,
        started: 99,
    };

    /// JARAK-FR-121: snapshots of a window that no longer runs come back with their text intact
    /// (Korean, CRLF, an untitled document); a running window's folder is never offered.
    #[test]
    fn a_dead_windows_snapshots_are_offered_with_their_text() {
        let root = temp("offer");
        let s = Store::new(&root, ME);
        let w = s
            .write(&[
                entry(1, Some(r"C:\notes\a.md"), "# 자락\r\n편집 중\r\n"),
                entry(2, None, "untitled text"),
            ])
            .unwrap();
        assert_eq!(w.kept, [1, 2]);
        assert!(
            orphans(&root, &|_| true).is_empty(),
            "a live window is left alone"
        );
        let o = orphans(&root, &|_| false);
        assert_eq!(o.len(), 1);
        let m = &o[0].manifest;
        assert_eq!(m.owner, ME);
        assert_eq!(m.entries.len(), 2);
        assert_eq!(o[0].text(&m.entries[0]).unwrap(), "# 자락\r\n편집 중\r\n");
        assert_eq!(o[0].text(&m.entries[1]).unwrap(), "untitled text");
        assert_eq!(
            m.entries[0].path.as_deref(),
            Some(Path::new(r"C:\notes\a.md"))
        );
        assert_eq!(m.entries[0].format().encoding.name(), "EUC-KR");
        assert_eq!(m.entries[0].eol(), Eol::CrLf);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Only the listed documents stay; an unchanged text is kept without being written again; an
    /// empty list removes the folder; the limits leave the largest documents out and name them.
    #[test]
    fn writes_keep_only_the_listed_documents_within_the_limits() {
        let root = temp("write");
        let mut s = Store::new(&root, ME);
        s.write(&[entry(1, None, "one"), entry(2, None, "two")])
            .unwrap();
        let mut unchanged = entry(2, None, "");
        unchanged.text = None;
        s.write(&[unchanged]).unwrap();
        assert!(
            !s.dir().join("doc-1.txt").exists(),
            "a saved document's snapshot goes"
        );
        assert_eq!(
            std::fs::read_to_string(s.dir().join("doc-2.txt")).unwrap(),
            "two"
        );
        s.limits = Limits {
            docs: 1,
            bytes: 1000,
        };
        let w = s
            .write(&[
                entry(3, Some("big.txt"), &"x".repeat(50)),
                entry(4, None, "small"),
            ])
            .unwrap();
        assert_eq!(w.kept, [4]);
        assert_eq!(w.left_out, ["big.txt"]);
        s.limits = Limits {
            docs: 10,
            bytes: 10,
        };
        let w = s
            .write(&[entry(5, Some("long.txt"), &"y".repeat(11))])
            .unwrap();
        assert_eq!(w.left_out, ["long.txt"]);
        s.write(&[]).unwrap();
        assert!(!s.dir().exists(), "nothing to recover: no folder");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Claiming renames the folder, so a second window cannot take it too; a claimed folder whose
    /// window is gone is offered again; removing it deletes it; a manifest naming a text outside
    /// the folder is not read.
    #[test]
    fn a_claim_is_taken_once_and_a_dead_claim_is_offered_again() {
        let root = temp("claim");
        Store::new(&root, ME)
            .write(&[entry(1, None, "keep me")])
            .unwrap();
        let o = orphans(&root, &|_| false).remove(0);
        let other = Owner { pid: 7, started: 8 };
        let claimed = o.claim(&other).unwrap();
        assert!(
            o.claim(&Owner { pid: 9, started: 9 }).is_err(),
            "taken already"
        );
        assert!(
            claimed
                .dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("claimed-7-8-")
        );
        assert!(
            orphans(&root, &|w| *w == other).is_empty(),
            "its window runs"
        );
        let again = orphans(&root, &|_| false);
        assert_eq!(again.len(), 1, "its window is gone: offered again");
        assert_eq!(
            again[0].text(&again[0].manifest.entries[0]).unwrap(),
            "keep me"
        );
        again.into_iter().next().unwrap().remove().unwrap();
        assert!(orphans(&root, &|_| false).is_empty());
        // A manifest pointing outside its folder.
        let bad = root.join("w-1-2");
        std::fs::create_dir_all(&bad).unwrap();
        let mut m = Manifest {
            schema: SCHEMA.into(),
            schema_version: SCHEMA_VERSION,
            owner: Owner { pid: 1, started: 2 },
            entries: vec![entry(1, None, "x").entry],
        };
        m.entries[0].text = r"..\..\secret.txt".into();
        std::fs::write(bad.join(MANIFEST), serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(orphans(&root, &|_| false).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The golden example (`docs/contracts/examples/jarak-recovery-v1`) is a manifest this Jarak
    /// reads.
    #[test]
    fn the_golden_example_is_a_manifest_jarak_reads() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/examples/jarak-recovery-v1/two-documents.json");
        let m: Manifest = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(m.schema, SCHEMA);
        assert_eq!(m.owner.pid, 4812);
        assert_eq!(m.entries.len(), 2);
        assert_eq!(m.entries[0].format().encoding.name(), "utf-8");
        assert_eq!(m.entries[1].path, None);
        assert_eq!(m.entries[1].eol(), Eol::CrLf);
    }

    /// This process is alive with its own start time; a made-up start time or process is not.
    #[cfg(windows)]
    #[test]
    fn the_running_process_is_alive_and_others_are_not() {
        let me = current_owner();
        assert_ne!(me.started, 0);
        assert!(owner_alive(&me));
        assert!(!owner_alive(&Owner {
            pid: me.pid,
            started: me.started + 1
        }));
        assert!(!owner_alive(&Owner {
            pid: u32::MAX - 1,
            started: 1
        }));
    }
}
