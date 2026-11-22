//! Jarak's own extensions (`JARAK-FR-160`, `JARAK-SEC-007`, Step 108): folders with a manifest
//! (`extension.json`, schema `jarak-extension-v1`) that add languages, snippets and commands.
//!
//! - **Installed only by an explicit action** (`jarak extensions install <folder>` copies a local
//!   folder into `%APPDATA%\mouv\jarak\extensions\<id>`); nothing is ever downloaded or updated.
//!   Notepad++ plugin DLLs are not loaded (they are not compatible; docs/JARAK_PLUGIN_AUDIT.md).
//! - **Languages and snippets are data**: they need no permission and are used as soon as the
//!   extension is installed.
//! - **A command runs a program** the extension names (an `.exe` inside its own folder, or a
//!   Windows program from System32) with the document's text or selection on stdin, and uses what
//!   it prints. It needs the permissions its manifest declares (`run-process`, `read-document`,
//!   `write-document`) **and the user's grant** of exactly those for exactly this manifest (its
//!   SHA-256): a changed manifest asks again, and a grant is revoked with one command.
//! - The program runs in a Windows job (killed with it, 512 MiB), from an empty temporary folder,
//!   with a minimal environment and no window, for at most its timeout (10 s by default, 60 s at
//!   most); its output is limited to 1 MiB of UTF-8. It still runs with the user's rights: Jarak
//!   cannot stop it from reading files, which is why it needs the grant — grant only programs you
//!   trust.

use crate::syntax::Lang;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "jarak.extension";
pub const SCHEMA_VERSION: u32 = 1;
/// The manifest's file name in an extension folder.
pub const MANIFEST: &str = "extension.json";
/// The grants file's schema.
pub const GRANTS_SCHEMA: &str = "jarak.extension-grants";
/// The largest manifest read.
const MAX_MANIFEST: u64 = 256 * 1024;
/// An installed extension holds at most this many files and bytes.
const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 50 * 1024 * 1024;
/// A command's output limit.
pub const MAX_OUTPUT: usize = 1024 * 1024;
/// A command's time limits.
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_TIMEOUT_MS: u64 = 60_000;

/// What an extension may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// Run the program a command names.
    RunProcess,
    /// Give the program the document's text or selection.
    ReadDocument,
    /// Put the program's output into the document (as one undo step).
    WriteDocument,
}

impl Permission {
    pub fn name(self) -> &'static str {
        match self {
            Permission::RunProcess => "run-process",
            Permission::ReadDocument => "read-document",
            Permission::WriteDocument => "write-document",
        }
    }
}

/// What a command gives the program on stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Input {
    #[default]
    None,
    Selection,
    Document,
}

/// What a command does with the program's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Output {
    /// Shown in the status bar (its first line).
    #[default]
    Message,
    ReplaceSelection,
    /// Inserted at the caret.
    Insert,
    NewDocument,
}

/// A command an extension adds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ko: Option<String>,
    /// An `.exe` inside the extension's folder (a relative path), or a Windows program's name
    /// (`sort.exe`) found in System32.
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub input: Input,
    #[serde(default)]
    pub output: Output,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl Command {
    /// The permissions the command needs.
    pub fn needs(&self) -> BTreeSet<Permission> {
        let mut p = BTreeSet::from([Permission::RunProcess]);
        if self.input != Input::None {
            p.insert(Permission::ReadDocument);
        }
        if matches!(self.output, Output::ReplaceSelection | Output::Insert) {
            p.insert(Permission::WriteDocument);
        }
        p
    }
}

/// What an extension adds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributes {
    /// User-language files (Step 98's JSON) inside the folder.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Snippet files (Step 100's `snippets.json` format) inside the folder.
    #[serde(default)]
    pub snippets: Vec<String>,
    #[serde(default)]
    pub commands: Vec<Command>,
}

/// An extension's manifest (`extension.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub schema_version: u32,
    /// Lower-case letters, digits, `.` and `-` (`mouv.sample-tools`).
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub contributes: Contributes,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

/// A checked extension.
#[derive(Debug, Clone)]
pub struct Extension {
    pub dir: PathBuf,
    pub manifest: Manifest,
    /// The manifest file's SHA-256 (a grant is for exactly this manifest).
    pub sha256: String,
    pub languages: Vec<Lang>,
    pub snippets: Vec<crate::assist::Snippet>,
}

impl Extension {
    /// The permissions its commands need.
    pub fn needs(&self) -> BTreeSet<Permission> {
        self.manifest
            .contributes
            .commands
            .iter()
            .flat_map(Command::needs)
            .collect()
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.split(['.', '-']).all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// A path inside the extension's folder (relative, no `..`, no drive), if it is one.
fn inside(dir: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if rel.is_empty()
        || rel.contains(':')
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    Some(dir.join(p))
}

/// The program a command runs: an `.exe` inside the folder, or a System32 program by name.
pub fn program_path(dir: &Path, program: &str) -> Result<PathBuf, String> {
    let lower = program.to_ascii_lowercase();
    if !lower.ends_with(".exe") {
        return Err(format!(
            "`{program}` is not an .exe (scripts and shell commands are not run)"
        ));
    }
    if !program.contains(['/', '\\']) {
        // A bare name: Windows' own programs only.
        let sys = std::env::var_os("SystemRoot").ok_or("SystemRoot is not set")?;
        let p = Path::new(&sys).join("System32").join(program);
        if p.is_file() {
            return Ok(p);
        }
        let local = dir.join(program);
        return if local.is_file() {
            Ok(local)
        } else {
            Err(format!(
                "`{program}` is neither in the extension nor in System32"
            ))
        };
    }
    let p =
        inside(dir, program).ok_or_else(|| format!("`{program}` leaves the extension's folder"))?;
    if p.is_file() {
        Ok(p)
    } else {
        Err(format!("`{program}` does not exist in the extension"))
    }
}

/// The SHA-256 of `bytes` as hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Reads and checks the extension in `dir`: every problem found, or the extension.
pub fn check(dir: &Path) -> Result<Extension, Vec<String>> {
    let mut problems = Vec::new();
    let mf = dir.join(MANIFEST);
    let bytes = match std::fs::metadata(&mf) {
        Ok(m) if m.len() > MAX_MANIFEST => {
            return Err(vec![format!("{MANIFEST} is larger than 256 KiB")]);
        }
        Ok(_) => std::fs::read(&mf).map_err(|e| vec![format!("{MANIFEST}: {e}")])?,
        Err(_) => return Err(vec![format!("{} has no {MANIFEST}", dir.display())]),
    };
    let manifest: Manifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(e) => return Err(vec![format!("{MANIFEST}: {e}")]),
    };
    if manifest.schema != SCHEMA || manifest.schema_version != SCHEMA_VERSION {
        problems.push(format!(
            "not a {SCHEMA} version {SCHEMA_VERSION} manifest (Jarak {} reads only that)",
            crate::VERSION
        ));
    }
    if !valid_id(&manifest.id) {
        problems.push(format!(
            "the id `{}` is not lower-case letters and digits joined by `.` or `-` (64 at most)",
            manifest.id
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.chars().count() > 80 {
        problems.push("the name is empty or longer than 80 characters".into());
    }
    if manifest.version.is_empty()
        || !manifest
            .version
            .split(['.', '-', '+'])
            .next()
            .is_some_and(|v| v.bytes().all(|b| b.is_ascii_digit()))
    {
        problems.push(format!(
            "the version `{}` is not like 1.0.0",
            manifest.version
        ));
    }
    let mut languages = Vec::new();
    for rel in &manifest.contributes.languages {
        match inside(dir, rel) {
            None => problems.push(format!("the language file `{rel}` leaves the folder")),
            Some(p) => match std::fs::read_to_string(&p) {
                Err(e) => problems.push(format!("the language file `{rel}`: {e}")),
                Ok(t) => match crate::syntax::parse_udl(&t) {
                    Ok(l) => languages.push(l),
                    Err(e) => problems.push(format!("the language file `{rel}`: {e}")),
                },
            },
        }
    }
    let mut snippets = Vec::new();
    for rel in &manifest.contributes.snippets {
        match inside(dir, rel) {
            None => problems.push(format!("the snippet file `{rel}` leaves the folder")),
            Some(p) => match std::fs::read_to_string(&p) {
                Err(e) => problems.push(format!("the snippet file `{rel}`: {e}")),
                Ok(t) => match crate::assist::parse_snippets(&t) {
                    Ok(s) => snippets.extend(s),
                    Err(e) => problems.push(format!("the snippet file `{rel}`: {e}")),
                },
            },
        }
    }
    let declared: BTreeSet<Permission> = manifest.permissions.iter().copied().collect();
    let mut ids = BTreeSet::new();
    for c in &manifest.contributes.commands {
        if !valid_id(&c.id) || c.id.contains('.') {
            problems.push(format!(
                "the command id `{}` is not lower-case letters and digits joined by `-`",
                c.id
            ));
        }
        if !ids.insert(c.id.clone()) {
            problems.push(format!("the command id `{}` is used twice", c.id));
        }
        if c.title.trim().is_empty() {
            problems.push(format!("the command `{}` has no title", c.id));
        }
        if let Err(e) = program_path(dir, &c.program) {
            problems.push(format!("the command `{}`: {e}", c.id));
        }
        if c.timeout_ms.is_some_and(|t| t == 0 || t > MAX_TIMEOUT_MS) {
            problems.push(format!(
                "the command `{}`: timeout_ms is 1 to {MAX_TIMEOUT_MS}",
                c.id
            ));
        }
        for p in c.needs() {
            if !declared.contains(&p) {
                problems.push(format!(
                    "the command `{}` needs `{}`; the manifest's permissions must name it",
                    c.id,
                    p.name()
                ));
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(Extension {
        dir: dir.to_path_buf(),
        manifest,
        sha256: sha256_hex(&bytes),
        languages,
        snippets,
    })
}

/// The installed extensions under `root` (each folder named by its id), and the folders that do
/// not check (with their problems; they are not used).
pub fn load_all(root: &Path) -> (Vec<Extension>, Vec<(PathBuf, Vec<String>)>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return (ok, bad);
    };
    let mut dirs: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for d in dirs {
        match check(&d) {
            Ok(e) => {
                let named = d
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy() == e.manifest.id);
                if named {
                    ok.push(e);
                } else {
                    bad.push((
                        d,
                        vec![format!(
                            "the folder is not named by its id `{}`",
                            e.manifest.id
                        )],
                    ));
                }
            }
            Err(p) => bad.push((d, p)),
        }
    }
    (ok, bad)
}

/// Why an install or removal did nothing.
#[derive(Debug)]
pub enum InstallError {
    Invalid(Vec<String>),
    /// An extension with this id is installed (replaced only when asked).
    Exists(String),
    NotInstalled(String),
    TooBig(String),
    Io(io::Error),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Invalid(p) => write!(f, "the extension does not check: {}", p.join("; ")),
            InstallError::Exists(id) => write!(f, "`{id}` is installed already"),
            InstallError::NotInstalled(id) => write!(f, "`{id}` is not installed"),
            InstallError::TooBig(why) => write!(f, "{why}"),
            InstallError::Io(e) => write!(f, "{e}"),
        }
    }
}

/// The files of `dir` (regular files only; links are not followed), relative to it.
fn files_of(dir: &Path) -> Result<Vec<PathBuf>, InstallError> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    let mut total = 0u64;
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(InstallError::Io)?.flatten() {
            let t = e.file_type().map_err(InstallError::Io)?;
            if t.is_symlink() {
                continue;
            }
            let p = e.path();
            if t.is_dir() {
                stack.push(p);
            } else if t.is_file() {
                total += e.metadata().map_or(0, |m| m.len());
                out.push(p.strip_prefix(dir).unwrap_or(&p).to_path_buf());
                if out.len() > MAX_FILES || total > MAX_BYTES {
                    return Err(InstallError::TooBig(format!(
                        "an extension has at most {MAX_FILES} files and 50 MiB"
                    )));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Copies the checked extension in `src` into `root/<id>`; an installed one is replaced only with
/// `replace` (its grant ends when the manifest differs). A dry run only checks.
pub fn install(
    src: &Path,
    root: &Path,
    replace: bool,
    dry_run: bool,
) -> Result<Extension, InstallError> {
    let e = check(src).map_err(InstallError::Invalid)?;
    let target = root.join(&e.manifest.id);
    if target.exists() && !replace {
        return Err(InstallError::Exists(e.manifest.id));
    }
    let files = files_of(src)?;
    if dry_run {
        return Ok(Extension { dir: target, ..e });
    }
    // Copied beside the target first, then moved into place.
    let staging = root.join(format!(
        ".{}.installing-{}",
        e.manifest.id,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&staging);
    let copy = || -> io::Result<()> {
        for f in &files {
            let to = staging.join(f);
            if let Some(p) = to.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(src.join(f), &to)?;
        }
        Ok(())
    };
    if let Err(err) = copy() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(InstallError::Io(err));
    }
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(InstallError::Io)?;
    }
    std::fs::rename(&staging, &target).map_err(InstallError::Io)?;
    check(&target).map_err(InstallError::Invalid)
}

/// Removes the installed extension `id` (its folder in `root`). A dry run only checks.
pub fn remove(root: &Path, id: &str, dry_run: bool) -> Result<PathBuf, InstallError> {
    if !valid_id(id) {
        return Err(InstallError::NotInstalled(id.into()));
    }
    let dir = root.join(id);
    if !dir.join(MANIFEST).is_file() {
        return Err(InstallError::NotInstalled(id.into()));
    }
    if !dry_run {
        std::fs::remove_dir_all(&dir).map_err(InstallError::Io)?;
    }
    Ok(dir)
}

/// One extension's grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub sha256: String,
    pub permissions: Vec<Permission>,
}

/// The user's grants (`%APPDATA%\mouv\jarak\extension-grants.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grants {
    #[serde(default)]
    pub grants: BTreeMap<String, Grant>,
}

impl Grants {
    /// The grants in `path` (none when missing or unreadable — nothing is granted then).
    pub fn load(path: &Path) -> Grants {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .filter(|v| v["schema"] == GRANTS_SCHEMA)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    /// Writes them atomically.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut v = serde_json::to_value(self).map_err(io::Error::other)?;
        v["schema"] = serde_json::json!(GRANTS_SCHEMA);
        v["schema_version"] = serde_json::json!(1);
        let text = serde_json::to_string_pretty(&v).map_err(io::Error::other)?;
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        crate::file::save_atomic(path, text.as_bytes())
    }

    /// Whether `e` may do everything its commands need (the grant is for this very manifest).
    pub fn covers(&self, e: &Extension) -> bool {
        self.grants.get(&e.manifest.id).is_some_and(|g| {
            g.sha256 == e.sha256 && e.needs().iter().all(|p| g.permissions.contains(p))
        })
    }

    /// Grants `e` the permissions its manifest declares.
    pub fn grant(&mut self, e: &Extension) {
        let mut p: Vec<Permission> = e.manifest.permissions.clone();
        p.sort();
        p.dedup();
        self.grants.insert(
            e.manifest.id.clone(),
            Grant {
                sha256: e.sha256.clone(),
                permissions: p,
            },
        );
    }

    /// Ends `id`'s grant; `true` when there was one.
    pub fn revoke(&mut self, id: &str) -> bool {
        self.grants.remove(id).is_some()
    }
}

/// Why a command did not run or its output is not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    NotGranted,
    Program(String),
    Start(String),
    Timeout(u64),
    /// The program ended with this code; the first line of what it wrote to stderr.
    Failed(i32, String),
    TooMuchOutput,
    NotText,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NotGranted => write!(f, "the extension has no grant for this"),
            RunError::Program(e) | RunError::Start(e) => write!(f, "{e}"),
            RunError::Timeout(ms) => write!(f, "the program did not finish within {} s", ms / 1000),
            RunError::Failed(code, err) => {
                if err.is_empty() {
                    write!(f, "the program ended with code {code}")
                } else {
                    write!(f, "the program ended with code {code}: {err}")
                }
            }
            RunError::TooMuchOutput => write!(f, "the program printed more than 1 MiB"),
            RunError::NotText => write!(f, "the program's output is not UTF-8 text"),
        }
    }
}

/// Runs command `c` of the extension in `dir` with `input` on stdin; its output (UTF-8). The
/// caller checks the grant first ([`Grants::covers`]).
#[cfg(windows)]
pub fn run(dir: &Path, c: &Command, input: &str) -> Result<String, RunError> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    let program = program_path(dir, &c.program).map_err(RunError::Program)?;
    let timeout = c
        .timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(1, MAX_TIMEOUT_MS);
    let work = std::env::temp_dir().join(format!(
        "jarak-ext-{}-{}",
        std::process::id(),
        crate::recovery::millis(std::time::SystemTime::now()).unwrap_or(0)
    ));
    std::fs::create_dir_all(&work).map_err(|e| RunError::Start(e.to_string()))?;
    let sys = std::env::var_os("SystemRoot").unwrap_or_default();
    let system32 = Path::new(&sys).join("System32");
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut child = std::process::Command::new(&program)
        .args(&c.args)
        .current_dir(&work)
        .env_clear()
        .env("SystemRoot", &sys)
        .env("TEMP", &work)
        .env("TMP", &work)
        .env("PATH", &system32)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| RunError::Start(format!("{} could not start: {e}", program.display())))?;
    let job = job::Job::new_for(&child);
    let mut stdin = child.stdin.take();
    let feed = input.as_bytes().to_vec();
    let writer = std::thread::spawn(move || {
        if let Some(s) = stdin.as_mut() {
            let _ = s.write_all(&feed);
        }
        drop(stdin);
    });
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(s) = stdout.as_mut() {
            let _ = s.take(MAX_OUTPUT as u64 + 1).read_to_end(&mut buf);
        }
        buf
    });
    let mut stderr = child.stderr.take();
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(s) = stderr.as_mut() {
            let _ = s.take(64 * 1024).read_to_end(&mut buf);
        }
        buf
    });
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed().as_millis() as u64 >= timeout => break None,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(15)),
            Err(_) => break None,
        }
    };
    if status.is_none() {
        drop(job);
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = writer.join();
    let out = reader.join().unwrap_or_default();
    let err = err_reader.join().unwrap_or_default();
    let _ = std::fs::remove_dir_all(&work);
    let Some(status) = status else {
        return Err(RunError::Timeout(timeout));
    };
    if !status.success() {
        let first = String::from_utf8_lossy(&err)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return Err(RunError::Failed(status.code().unwrap_or(-1), first));
    }
    if out.len() > MAX_OUTPUT {
        return Err(RunError::TooMuchOutput);
    }
    String::from_utf8(out).map_err(|_| RunError::NotText)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod job {
    //! A Windows job for a command's program: killed when the job closes, 512 MiB of memory.
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    pub struct Job(HANDLE);

    impl Job {
        /// A job holding `child` (`None` when Windows refuses one; the program then runs without
        /// the limits, still killed on timeout).
        pub fn new_for(child: &std::process::Child) -> Option<Job> {
            // SAFETY: an unnamed job with default security; the limits structure is plain data;
            // the process handle belongs to `child`, alive here.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
                info.ProcessMemoryLimit = 512 * 1024 * 1024;
                SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                AssignProcessToJobObject(h, child.as_raw_handle() as HANDLE);
                Some(Job(h))
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the job handle is closed once; the job's processes end with it.
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-ext-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_ext(dir: &Path, manifest: &serde_json::Value) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(MANIFEST),
            serde_json::to_vec_pretty(manifest).unwrap(),
        )
        .unwrap();
    }

    fn sample() -> serde_json::Value {
        serde_json::json!({
            "schema": SCHEMA, "schema_version": 1, "id": "test.sorter", "name": "Sorter",
            "version": "1.0.0",
            "contributes": {
                "snippets": ["snippets.json"],
                "commands": [{ "id": "sort", "title": "Sort lines", "program": "sort.exe",
                    "input": "selection", "output": "replace-selection" }]
            },
            "permissions": ["run-process", "read-document", "write-document"]
        })
    }

    /// The sample extension checks, and the contract example is its manifest.
    #[test]
    fn the_sample_extension_checks_and_is_the_contract_example() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sample = root.join("examples/jarak-extensions/mouv.sample-tools");
        let e = check(&sample).unwrap();
        assert_eq!(e.manifest.id, "mouv.sample-tools");
        assert_eq!(e.languages[0].name, "TODO list");
        assert_eq!(e.snippets.len(), 2);
        assert_eq!(e.manifest.contributes.commands.len(), 2);
        let example = std::fs::read_to_string(
            root.join("docs/contracts/examples/jarak-extension-v1/sample-tools.json"),
        )
        .unwrap();
        let manifest = std::fs::read_to_string(sample.join(MANIFEST)).unwrap();
        let lf = |t: &str| t.replace("\r\n", "\n");
        assert_eq!(lf(&example), lf(&manifest));
    }

    #[test]
    fn manifests_are_checked_and_permissions_must_be_declared() {
        let d = temp("check");
        let ext = d.join("sorter");
        write_ext(&ext, &sample());
        std::fs::write(
            ext.join("snippets.json"),
            r#"{ "*": { "todo": { "body": "TODO: $0" } } }"#,
        )
        .unwrap();
        let e = check(&ext).unwrap();
        assert_eq!(e.snippets.len(), 1);
        assert_eq!(e.needs().len(), 3);
        // A command needing a permission the manifest does not name; a path leaving the folder;
        // a script instead of a program.
        let mut bad = sample();
        bad["permissions"] = serde_json::json!(["run-process"]);
        bad["contributes"]["snippets"] = serde_json::json!(["../snippets.json"]);
        bad["contributes"]["commands"][0]["program"] = serde_json::json!("run.cmd");
        write_ext(&ext, &bad);
        let p = check(&ext).unwrap_err();
        assert!(
            p.iter().any(|x| x.contains("needs `read-document`")),
            "{p:?}"
        );
        assert!(p.iter().any(|x| x.contains("leaves the folder")), "{p:?}");
        assert!(p.iter().any(|x| x.contains("not an .exe")), "{p:?}");
        let mut id = sample();
        id["id"] = serde_json::json!("Bad Id");
        write_ext(&ext, &id);
        assert!(
            check(&ext)
                .unwrap_err()
                .iter()
                .any(|x| x.contains("the id"))
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn install_never_replaces_without_asking_and_grants_follow_the_manifest() {
        let d = temp("install");
        let src = d.join("src");
        write_ext(&src, &sample());
        std::fs::write(src.join("snippets.json"), "{}").unwrap();
        let root = d.join("extensions");
        let dry = install(&src, &root, false, true).unwrap();
        assert_eq!(dry.dir, root.join("test.sorter"));
        assert!(!root.exists(), "a dry run copies nothing");
        let e = install(&src, &root, false, false).unwrap();
        assert!(e.dir.join(MANIFEST).is_file());
        assert!(matches!(
            install(&src, &root, false, false),
            Err(InstallError::Exists(_))
        ));
        let (all, bad) = load_all(&root);
        assert_eq!((all.len(), bad.len()), (1, 0));
        let mut g = Grants::default();
        assert!(!g.covers(&e), "nothing is granted by itself");
        g.grant(&e);
        assert!(g.covers(&e));
        let gp = d.join("grants.json");
        g.save(&gp).unwrap();
        assert!(Grants::load(&gp).covers(&e));
        // A changed manifest is not covered by the old grant.
        let mut changed = sample();
        changed["contributes"]["commands"][0]["args"] = serde_json::json!(["/r"]);
        write_ext(&src, &changed);
        let e2 = install(&src, &root, true, false).unwrap();
        assert!(!g.covers(&e2));
        assert!(g.revoke("test.sorter"));
        assert!(!g.covers(&e));
        assert_eq!(
            remove(&root, "test.sorter", true).unwrap(),
            root.join("test.sorter")
        );
        assert!(
            root.join("test.sorter").exists(),
            "a dry run removes nothing"
        );
        remove(&root, "test.sorter", false).unwrap();
        assert!(!root.join("test.sorter").exists());
        assert!(matches!(
            remove(&root, "test.sorter", false),
            Err(InstallError::NotInstalled(_))
        ));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_command_runs_with_its_input_and_limits() {
        let d = temp("run");
        let c = Command {
            id: "sort".into(),
            title: "Sort".into(),
            title_ko: None,
            program: "sort.exe".into(),
            args: vec![],
            input: Input::Selection,
            output: Output::ReplaceSelection,
            timeout_ms: None,
        };
        let out = run(&d, &c, "pear\r\napple\r\n한글\r\n").unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "apple");
        assert!(lines.contains(&"pear"));
        let fail = Command {
            program: "where.exe".into(),
            args: vec!["no-such-program-anywhere-xyz".into()],
            input: Input::None,
            output: Output::Message,
            ..c.clone()
        };
        assert!(matches!(run(&d, &fail, ""), Err(RunError::Failed(..))));
        let slow = Command {
            program: "ping.exe".into(),
            args: vec!["-n".into(), "30".into(), "127.0.0.1".into()],
            input: Input::None,
            output: Output::Message,
            timeout_ms: Some(300),
            ..c
        };
        let t = std::time::Instant::now();
        assert_eq!(run(&d, &slow, ""), Err(RunError::Timeout(300)));
        assert!(t.elapsed().as_secs() < 5, "killed at its timeout");
        let _ = std::fs::remove_dir_all(&d);
    }
}
