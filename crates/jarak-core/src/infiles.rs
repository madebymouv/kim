//! Find and Replace in Files (`JARAK-FR-073`, Step 95): a folder walked with Notepad++-style
//! filters, every file searched with the Step 94 matcher on a worker pool, and Replace in Files as a
//! plan (every change listed) that is applied only to the changes the user confirmed, one file at a
//! time, each saved atomically in its own encoding, byte-order mark and line endings, after its
//! original bytes went into a journal the batch can be undone from.
//!
//! Safety: a file is only rewritten when its bytes are still the ones the plan was made from (a
//! SHA-256 check), it decodes losslessly, it is not binary and the new text can be encoded; any
//! other file is skipped with the reason. Nothing here deletes a user file; the journal keeps the
//! last [`JOURNAL_KEEP`] batches.

use crate::file::{self, Format, TextEncoding};
use crate::search::Matcher;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Files above this size are skipped (Find in Files reads each one whole).
pub const SIZE_LIMIT: u64 = 64 * 1024 * 1024;
/// The most matches a search lists (the count goes on).
pub const MAX_HITS: usize = 100_000;
/// The journal keeps this many Replace in Files batches.
pub const JOURNAL_KEEP: usize = 20;

/// Notepad++-style filters: names separated by spaces or `;` with `*` and `?`; `!name` excludes
/// files, `!\folder` excludes a folder directly in the searched folder and `!+\folder` one at any
/// depth. No include pattern means every file. Names compare without case (Windows).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// (pattern, at any depth)
    pub exclude_dirs: Vec<(String, bool)>,
}

impl Filters {
    pub fn parse(s: &str) -> Filters {
        let mut f = Filters::default();
        for t in s.split([' ', ';', ',']).filter(|t| !t.is_empty()) {
            let t = t.to_lowercase();
            if let Some(d) = t.strip_prefix("!+\\").or_else(|| t.strip_prefix("!+/")) {
                f.exclude_dirs
                    .push((d.trim_end_matches(['\\', '/']).to_string(), true));
            } else if let Some(d) = t.strip_prefix("!\\").or_else(|| t.strip_prefix("!/")) {
                f.exclude_dirs
                    .push((d.trim_end_matches(['\\', '/']).to_string(), false));
            } else if let Some(x) = t.strip_prefix('!') {
                f.exclude.push(x.to_string());
            } else {
                f.include.push(t);
            }
        }
        f
    }

    /// Does a file name pass?
    pub fn file(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        (self.include.is_empty() || self.include.iter().any(|p| glob(p, &n)))
            && !self.exclude.iter().any(|p| glob(p, &n))
    }

    /// Is a folder (at `depth` 1 = directly in the searched folder) excluded?
    pub fn dir_excluded(&self, name: &str, depth: usize) -> bool {
        let n = name.to_lowercase();
        self.exclude_dirs
            .iter()
            .any(|(p, any)| (*any || depth == 1) && glob(p, &n))
    }
}

/// `*` and `?` wildcards (`*.*` is every name, as in Notepad++).
pub fn glob(p: &str, s: &str) -> bool {
    if p == "*.*" || p == "*" {
        return true;
    }
    let (p, s): (Vec<char>, Vec<char>) = (p.chars().collect(), s.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while j < s.len() {
        if i < p.len() && (p[i] == '?' || p[i] == s[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some(i);
            mark = j;
            i += 1;
        } else if let Some(k) = star {
            i = k + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

/// Where to search.
#[derive(Debug, Clone)]
pub struct Scope {
    pub dir: PathBuf,
    pub filters: Filters,
    /// Into sub-folders.
    pub recursive: bool,
    /// Into hidden folders and hidden files.
    pub hidden: bool,
    /// The encoding of text that is neither UTF-8 nor UTF-16.
    pub fallback: TextEncoding,
}

fn hidden(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x2 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        false
    }
}

/// The files of `scope`: each folder's own files, then its sub-folders (depth first), each in
/// name order without case. Links and junctions are not followed.
pub fn files(scope: &Scope, cancel: &AtomicBool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(scope.dir.clone(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<(String, PathBuf, std::fs::Metadata)> = rd
            .filter_map(Result::ok)
            .filter_map(|e| {
                let m = std::fs::symlink_metadata(e.path()).ok()?;
                Some((e.file_name().to_string_lossy().into_owned(), e.path(), m))
            })
            .collect();
        entries.sort_by_key(|(n, _, _)| n.to_lowercase());
        let mut sub = Vec::new();
        for (name, path, meta) in entries {
            if meta.file_type().is_symlink() || (!scope.hidden && hidden(&meta)) {
                continue;
            }
            if meta.is_dir() {
                if scope.recursive && !scope.filters.dir_excluded(&name, depth + 1) {
                    sub.push((path, depth + 1));
                }
            } else if meta.is_file() && scope.filters.file(&name) {
                out.push(path);
            }
        }
        // Depth first, in name order.
        stack.extend(sub.into_iter().rev());
    }
    out
}

/// One match in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// From 1.
    pub line: usize,
    /// Characters, from 1.
    pub column: usize,
    /// Bytes in the decoded text.
    pub start: usize,
    pub end: usize,
    /// The line without its line ending.
    pub line_text: String,
}

/// A file with matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHits {
    pub path: PathBuf,
    pub format: Format,
    pub hits: Vec<Hit>,
    /// The SHA-256 of the bytes searched.
    pub sha256: [u8; 32],
}

/// Why a file was not searched (or not changed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    Binary,
    TooLarge(u64),
    Unreadable(String),
    /// It does not decode losslessly (a replace would change other bytes).
    Lossy,
    /// It changed after the preview.
    Changed,
    /// The new text cannot be written in the file's encoding.
    Unmappable(String),
    /// Open in Jarak with unsaved changes.
    Unsaved,
}

impl Skip {
    pub fn code(&self) -> &'static str {
        match self {
            Skip::Binary => "binary",
            Skip::TooLarge(_) => "too_large",
            Skip::Unreadable(_) => "unreadable",
            Skip::Lossy => "lossy",
            Skip::Changed => "changed",
            Skip::Unmappable(_) => "unmappable",
            Skip::Unsaved => "unsaved",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Skip::Binary => "a binary file".into(),
            Skip::TooLarge(n) => format!("{n} bytes, above the {SIZE_LIMIT}-byte limit"),
            Skip::Unreadable(e) => e.clone(),
            Skip::Lossy => "its encoding cannot reproduce every byte".into(),
            Skip::Changed => "it changed after the preview".into(),
            Skip::Unmappable(e) => e.clone(),
            Skip::Unsaved => "it is open with unsaved changes".into(),
        }
    }
}

/// A search over files.
#[derive(Debug, Clone, Default)]
pub struct Found {
    pub files: Vec<FileHits>,
    pub searched: usize,
    pub skipped: Vec<(PathBuf, Skip)>,
    pub hits: usize,
    /// More than [`MAX_HITS`] matches: the rest are counted, not listed.
    pub truncated: bool,
    pub cancelled: bool,
}

/// The lines and columns of byte ranges in `text`.
pub fn hits_of(text: &str, ranges: &[(usize, usize)]) -> Vec<Hit> {
    let mut out = Vec::with_capacity(ranges.len());
    let (mut line, mut line_start, mut at) = (1usize, 0usize, 0usize);
    for &(a, b) in ranges {
        for (k, c) in text[at..a].char_indices() {
            if c == '\n' {
                line += 1;
                line_start = at + k + 1;
            }
        }
        at = a;
        let end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |k| line_start + k);
        let line_text = text[line_start..end].trim_end_matches('\r').to_string();
        out.push(Hit {
            line,
            column: text[line_start..a].chars().count() + 1,
            start: a,
            end: b,
            line_text,
        });
    }
    out
}

/// One file's search: its matches (none: `Ok(None)`) or why it was skipped.
type Searched = Result<Option<FileHits>, Skip>;

/// Searches one file's bytes.
pub fn search_bytes(
    path: &Path,
    bytes: &[u8],
    m: &Matcher,
    fallback: TextEncoding,
    cancel: &AtomicBool,
) -> Result<Option<FileHits>, Skip> {
    let d = file::decode(bytes, fallback);
    if d.binary {
        return Err(Skip::Binary);
    }
    let (ranges, _) = m.find_bytes(&d.text, &|| cancel.load(Ordering::Relaxed));
    if ranges.is_empty() {
        return Ok(None);
    }
    Ok(Some(FileHits {
        path: path.to_path_buf(),
        format: d.format,
        hits: hits_of(&d.text, &ranges),
        sha256: Sha256::digest(bytes).into(),
    }))
}

fn read_limited(path: &Path) -> Result<Vec<u8>, Skip> {
    let meta = std::fs::metadata(path).map_err(|e| Skip::Unreadable(e.to_string()))?;
    if meta.len() > SIZE_LIMIT {
        return Err(Skip::TooLarge(meta.len()));
    }
    std::fs::read(path).map_err(|e| Skip::Unreadable(e.to_string()))
}

/// Find in Files: every file of `scope` searched on a pool of threads, `progress` called with the
/// number of files done. Results come in the walk's order.
pub fn find_in_files(
    scope: &Scope,
    m: &Matcher,
    cancel: &AtomicBool,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Found {
    let list = files(scope, cancel);
    let n = list.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<Searched>>> = (0..n).map(|_| Mutex::new(None)).collect();
    let workers = std::thread::available_parallelism()
        .map_or(4, |p| p.get())
        .clamp(1, 8)
        .min(n.max(1));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n || cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let r = read_limited(&list[i])
                        .and_then(|b| search_bytes(&list[i], &b, m, scope.fallback, cancel));
                    *slots[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1, n);
                }
            });
        }
    });
    let mut f = Found {
        cancelled: cancel.load(Ordering::Relaxed),
        ..Found::default()
    };
    for (path, slot) in list.into_iter().zip(slots) {
        match slot.into_inner().unwrap_or_else(|e| e.into_inner()) {
            None => {}
            Some(Ok(None)) => f.searched += 1,
            Some(Ok(Some(mut h))) => {
                f.searched += 1;
                f.hits += h.hits.len();
                let room = MAX_HITS.saturating_sub(f.hits - h.hits.len());
                if h.hits.len() > room {
                    h.hits.truncate(room);
                    f.truncated = true;
                }
                if !h.hits.is_empty() {
                    f.files.push(h);
                }
            }
            Some(Err(e)) => f.skipped.push((path, e)),
        }
    }
    f
}

/// One planned replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub hit: Hit,
    pub replacement: String,
    /// The line as it will read.
    pub new_line: String,
}

/// The planned replacements of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePlan {
    pub path: PathBuf,
    pub format: Format,
    pub sha256: [u8; 32],
    pub changes: Vec<Change>,
}

/// Replace in Files, step 1: the changes, nothing written. Files that could be searched but not
/// rewritten (lossy encodings) are listed as skipped.
pub fn plan(scope: &Scope, m: &Matcher, with: &str, cancel: &AtomicBool) -> (Vec<FilePlan>, Found) {
    let found = find_in_files(scope, m, cancel, &|_, _| {});
    let mut plans = Vec::new();
    let mut skipped = found.skipped.clone();
    for fh in &found.files {
        match plan_file(&fh.path, m, with, scope.fallback) {
            Ok(Some(p)) => plans.push(p),
            Ok(None) => {}
            Err(e) => skipped.push((fh.path.clone(), e)),
        }
    }
    (plans, Found { skipped, ..found })
}

/// The plan of one file (read again: the plan is made from the bytes it checks later).
pub fn plan_file(
    path: &Path,
    m: &Matcher,
    with: &str,
    fallback: TextEncoding,
) -> Result<Option<FilePlan>, Skip> {
    let bytes = read_limited(path)?;
    plan_bytes(path, &bytes, m, with, fallback)
}

pub fn plan_bytes(
    path: &Path,
    bytes: &[u8],
    m: &Matcher,
    with: &str,
    fallback: TextEncoding,
) -> Result<Option<FilePlan>, Skip> {
    let d = file::decode(bytes, fallback);
    if d.binary {
        return Err(Skip::Binary);
    }
    if !d.lossless {
        return Err(Skip::Lossy);
    }
    let (ranges, _) = m.find_bytes(&d.text, &|| false);
    if ranges.is_empty() {
        return Ok(None);
    }
    let hits = hits_of(&d.text, &ranges);
    let changes = hits
        .into_iter()
        .map(|hit| {
            let replacement = m.replacement_bytes(&d.text, (hit.start, hit.end), with);
            let line_start = hit.start - text_before_in_line(&d.text, hit.start);
            let col_a = hit.start - line_start;
            let col_b = (hit.end - line_start).min(hit.line_text.len()).max(col_a);
            let mut new_line = String::new();
            new_line.push_str(&hit.line_text[..col_a.min(hit.line_text.len())]);
            new_line.push_str(&replacement);
            new_line.push_str(&hit.line_text[col_b..]);
            Change {
                hit,
                replacement,
                new_line,
            }
        })
        .collect();
    Ok(Some(FilePlan {
        path: path.to_path_buf(),
        format: d.format,
        sha256: Sha256::digest(bytes).into(),
        changes,
    }))
}

fn text_before_in_line(text: &str, at: usize) -> usize {
    at - text[..at].rfind('\n').map_or(0, |k| k + 1)
}

/// One file's result of applying a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub path: PathBuf,
    pub replaced: usize,
}

/// A Replace in Files batch (its journal).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Batch {
    pub schema: String,
    pub schema_version: u32,
    pub id: String,
    /// Seconds since 1970 (UTC).
    pub created: u64,
    pub find: String,
    pub replace: String,
    pub files: Vec<JournalFile>,
    #[serde(default)]
    pub undone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JournalFile {
    pub path: PathBuf,
    /// The original bytes, under the batch's folder.
    pub backup: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub replaced: usize,
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// The result of applying (or undoing) a batch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub applied: Vec<Applied>,
    pub skipped: Vec<(PathBuf, Skip)>,
    pub batch: Option<Batch>,
    pub journal: Option<PathBuf>,
}

/// Replace in Files, step 2: applies the changes `keep` confirms (by file and change index) —
/// one file at a time, each checked against its plan, its original bytes saved in the journal
/// first, then written atomically. `journal_root` holds one folder per batch.
pub fn apply(
    plans: &[FilePlan],
    keep: &dyn Fn(&Path, usize) -> bool,
    fallback: TextEncoding,
    journal_root: &Path,
    find: &str,
    replace: &str,
) -> std::io::Result<Outcome> {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let id = new_batch_id(journal_root, created);
    let dir = journal_root.join(&id);
    let mut out = Outcome::default();
    let mut batch = Batch {
        schema: "jarak.replace-journal".into(),
        schema_version: 1,
        id: id.clone(),
        created,
        find: find.into(),
        replace: replace.into(),
        files: Vec::new(),
        undone: false,
    };
    for p in plans {
        let chosen: Vec<&Change> = p
            .changes
            .iter()
            .enumerate()
            .filter(|(i, _)| keep(&p.path, *i))
            .map(|(_, c)| c)
            .collect();
        if chosen.is_empty() {
            continue;
        }
        let bytes = match read_limited(&p.path) {
            Ok(b) => b,
            Err(e) => {
                out.skipped.push((p.path.clone(), e));
                continue;
            }
        };
        if <[u8; 32]>::from(Sha256::digest(&bytes)) != p.sha256 {
            out.skipped.push((p.path.clone(), Skip::Changed));
            continue;
        }
        let d = file::decode(&bytes, fallback);
        if d.binary || !d.lossless || d.format != p.format {
            out.skipped.push((p.path.clone(), Skip::Lossy));
            continue;
        }
        let mut text = String::with_capacity(d.text.len());
        let mut at = 0;
        for c in &chosen {
            text.push_str(&d.text[at..c.hit.start]);
            text.push_str(&c.replacement);
            at = c.hit.end;
        }
        text.push_str(&d.text[at..]);
        let new_bytes = match file::encode(&text, p.format) {
            Ok(b) => b,
            Err(e) => {
                out.skipped
                    .push((p.path.clone(), Skip::Unmappable(e.to_string())));
                continue;
            }
        };
        // The original first (the journal is written before the file changes).
        std::fs::create_dir_all(&dir)?;
        let backup = format!("{}.bak", batch.files.len() + 1);
        std::fs::write(dir.join(&backup), &bytes)?;
        batch.files.push(JournalFile {
            path: p.path.clone(),
            backup,
            before_sha256: hex(&p.sha256),
            after_sha256: hex(&Sha256::digest(&new_bytes)),
            replaced: chosen.len(),
        });
        write_manifest(&dir, &batch)?;
        if let Err(e) = file::save_atomic(&p.path, &new_bytes) {
            batch.files.pop();
            write_manifest(&dir, &batch)?;
            out.skipped
                .push((p.path.clone(), Skip::Unreadable(e.to_string())));
            continue;
        }
        out.applied.push(Applied {
            path: p.path.clone(),
            replaced: chosen.len(),
        });
    }
    if !batch.files.is_empty() {
        out.journal = Some(dir);
        out.batch = Some(batch);
        prune(journal_root, &id);
    }
    Ok(out)
}

fn new_batch_id(root: &Path, created: u64) -> String {
    let mut id = format!("{created}");
    let mut n = 1;
    while root.join(&id).exists() {
        n += 1;
        id = format!("{created}-{n}");
    }
    id
}

fn write_manifest(dir: &Path, b: &Batch) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(b).map_err(std::io::Error::other)?;
    file::save_atomic(&dir.join("batch.json"), &json)
}

/// Keeps the newest [`JOURNAL_KEEP`] batches (never the one just made).
fn prune(root: &Path, keep_id: &str) {
    let mut all = batches(root);
    all.retain(|b| b.id != keep_id);
    // Newest first; the just-made one counts as one.
    for b in all.into_iter().skip(JOURNAL_KEEP - 1) {
        let _ = std::fs::remove_dir_all(root.join(&b.id));
    }
}

/// The batches in the journal, newest first.
pub fn batches(root: &Path) -> Vec<Batch> {
    let mut v: Vec<Batch> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| std::fs::read(e.path().join("batch.json")).ok())
        .filter_map(|b| serde_json::from_slice::<Batch>(&b).ok())
        .filter(|b| b.schema == "jarak.replace-journal")
        .collect();
    v.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    v
}

/// Undoes a batch: every file whose bytes are still what the batch wrote gets its original bytes
/// back (atomically); a file changed since is left as it is and reported. `dry_run` only checks.
pub fn undo(root: &Path, id: &str, dry_run: bool) -> std::io::Result<Outcome> {
    let dir = root.join(id);
    let mut b: Batch = serde_json::from_slice(&std::fs::read(dir.join("batch.json"))?)
        .map_err(std::io::Error::other)?;
    let mut out = Outcome::default();
    for f in &b.files {
        let now = match std::fs::read(&f.path) {
            Ok(x) => x,
            Err(e) => {
                out.skipped
                    .push((f.path.clone(), Skip::Unreadable(e.to_string())));
                continue;
            }
        };
        if hex(&Sha256::digest(&now)) != f.after_sha256 {
            out.skipped.push((f.path.clone(), Skip::Changed));
            continue;
        }
        let original = std::fs::read(dir.join(&f.backup))?;
        if hex(&Sha256::digest(&original)) != f.before_sha256 {
            out.skipped.push((
                f.path.clone(),
                Skip::Unreadable("the backup is damaged".into()),
            ));
            continue;
        }
        if !dry_run {
            file::save_atomic(&f.path, &original)?;
        }
        out.applied.push(Applied {
            path: f.path.clone(),
            replaced: f.replaced,
        });
    }
    if !dry_run && out.skipped.is_empty() {
        b.undone = true;
        write_manifest(&dir, &b)?;
    }
    out.journal = Some(dir);
    out.batch = Some(b);
    Ok(out)
}

#[cfg(test)]
mod tests;
