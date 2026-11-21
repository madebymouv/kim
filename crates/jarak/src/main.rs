//! `jarak.exe`: the headless CLI of Jarak (docs/CLI_CONTRACT.md, `JARAK-FR-001`,
//! `JARAK-SEC-009`). It never opens a window, tray icon or prompt — except `jarak open`, whose
//! capability says `opens_window` and which starts `jarak-gui.exe` because the user ran exactly
//! that command (CLI_CONTRACT §4.5).

use jarak_core::paths::Paths;
use jarak_core::target::OpenTarget;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const CAPABILITIES: &str = include_str!("capabilities.json");
const GUI_EXE: &str = "jarak-gui.exe";

const HELP: &str = "\
Jarak (자락): a native text editor for plain text, code and Markdown — the command line.

This CLI never opens a window except `jarak open`, never runs in the background, never uses the
network and changes a file only when a command says so (`jarak assets rename`).

Usage: jarak [OPTIONS] <COMMAND>

Commands:
  open          Start the Jarak window with files (file, file:line, file:line:column) or a
                folder (its workspace), or hand them to the running window; --new-window starts
                another window
  info          A file's encoding, byte-order mark, line endings, size and line count, and
                whether Jarak can save it unchanged (read-only); --fallback <encoding> reads
                text that is neither UTF-8 nor UTF-16 in that code page
  assets list <doc.md>
                The images a Markdown document refers to (found, missing, remote) and the
                orphaned files in its assets folder that no Markdown file there refers to
                (read-only; nothing is deleted)
  assets rename <doc.md> <asset> <new-name>
                Renames an asset and rewrites the document's references to it; never replaces a
                file, refuses an asset other documents use; --dry-run shows the change
  paths         Where Jarak keeps its settings, session and recovery files (read-only)
  version       Jarak's version and the contract and schema versions it speaks (read-only)
  capabilities  Jarak's capabilities document: its commands and what each may change (read-only)
  help          Print this message

Options:
      --json      Print exactly one JSON document on stdout instead of human-readable text
      --dry-run   Show what would happen and change nothing (open, assets rename)
  -q, --quiet     Print only results and errors
  -v, --verbose   Print extra diagnostics on stderr
      --no-input  Never ask questions (Jarak's CLI never asks)
  -h, --help      Print help
  -V, --version   Print version

Exit codes: 0 ok · 1 failure (I/O, permission) · 2 usage · 3 not found · 5 too large ·
7 conflict (a file exists, an asset is shared, the document is read-only)
";

/// A failure with its stable error code (CLI_CONTRACT §6) and exit code (§5).
#[derive(Debug)]
struct Fail {
    code: &'static str,
    exit: u8,
    message: String,
    hint: Option<String>,
    details: Vec<Value>,
}

impl Fail {
    fn new(code: &'static str, exit: u8, message: impl Into<String>) -> Fail {
        Fail {
            code,
            exit,
            message: message.into(),
            hint: None,
            details: Vec::new(),
        }
    }

    fn usage(message: impl Into<String>) -> Fail {
        Fail {
            hint: Some("see `jarak --help`".into()),
            ..Fail::new("usage", 2, message)
        }
    }
}

#[derive(Debug, Default)]
struct Global {
    json: bool,
    dry_run: bool,
    quiet: bool,
    verbose: bool,
    new_window: bool,
    /// `jarak info --fallback <label>`: the legacy encoding instead of the system's.
    fallback: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args)
}

fn run(args: &[String]) -> ExitCode {
    let mut g = Global::default();
    let mut words: Vec<&str> = Vec::new();
    let mut help = false;
    let mut version = false;
    let mut unknown: Option<&str> = None;
    let mut rest_are_files = false;
    let mut it = args.iter();
    let mut missing_value: Option<&str> = None;
    while let Some(a) = it.next() {
        if rest_are_files {
            words.push(a);
            continue;
        }
        match a.as_str() {
            "--fallback" => match it.next() {
                Some(v) => g.fallback = Some(v.clone()),
                None => missing_value = Some("--fallback"),
            },
            "--" => rest_are_files = true,
            "--json" => g.json = true,
            "--dry-run" => g.dry_run = true,
            "-q" | "--quiet" => g.quiet = true,
            "-v" | "--verbose" => g.verbose = true,
            "--no-input" => {}
            "--new-window" => g.new_window = true,
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            s if s.starts_with('-') && s.len() > 1 => unknown = unknown.or(Some(s)),
            s => words.push(s),
        }
    }
    if version {
        println!("jarak {}", jarak_core::VERSION);
        return ExitCode::SUCCESS;
    }
    if help || (words.is_empty() && !g.json) || words.first() == Some(&"help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let command = match (words.first().copied(), words.get(1).copied()) {
        (Some("assets"), Some("list")) => "assets.list",
        (Some("assets"), Some("rename")) => "assets.rename",
        (Some(c @ ("open" | "info" | "paths" | "version" | "capabilities")), _) => c,
        _ => "jarak",
    };
    let skip = if command.starts_with("assets.") { 2 } else { 1 };
    let result = if let Some(flag) = unknown {
        Err(Fail::usage(format!("unknown option `{flag}`")))
    } else if let Some(flag) = missing_value {
        Err(Fail::usage(format!("{flag} needs a value")))
    } else if g.fallback.is_some() && command != "info" {
        Err(Fail::usage("--fallback belongs to `jarak info`"))
    } else if g.quiet && g.verbose {
        Err(Fail::usage("--quiet and --verbose conflict"))
    } else if command == "jarak" {
        Err(Fail::usage(match (words.first(), words.get(1)) {
            (Some(&"assets"), Some(w)) => format!("unknown command `assets {w}` (list, rename)"),
            (Some(&"assets"), None) => "`jarak assets` needs list or rename".into(),
            (Some(w), _) => format!("unknown command `{w}`"),
            (None, _) => "no command given".into(),
        }))
    } else if g.new_window && command != "open" {
        Err(Fail::usage("--new-window belongs to `jarak open`"))
    } else if g.dry_run && !matches!(command, "open" | "assets.rename") {
        Err(Fail::usage(format!(
            "`jarak {}` changes nothing; --dry-run belongs to `jarak open` and `jarak assets rename`",
            command.replace('.', " ")
        )))
    } else {
        dispatch(command, &words[skip..], &g)
    };
    finish(command, &g, result)
}

/// The data of a command and the human text for it.
type Done = (Value, String);

fn dispatch(command: &str, rest: &[&str], g: &Global) -> Result<Done, Fail> {
    match command {
        "version" | "capabilities" | "paths" if !rest.is_empty() => Err(Fail::usage(format!(
            "`jarak {command}` takes no arguments (got `{}`)",
            rest.join(" ")
        ))),
        "version" => Ok((
            json!({
                "tool": "jarak",
                "version": jarak_core::VERSION,
                "cli_contract": 1,
                "json_schema_version": 1,
            }),
            format!(
                "jarak {} (CLI contract 1, JSON schema 1)",
                jarak_core::VERSION
            ),
        )),
        "capabilities" => {
            let doc: Value = serde_json::from_str(CAPABILITIES).expect("bundled capabilities");
            let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
            Ok((doc, text))
        }
        "paths" => paths(),
        "open" => open(rest, g),
        "info" => info(rest, g.fallback.as_deref()),
        "assets.list" => assets_list(rest),
        "assets.rename" => assets_rename(rest, g),
        _ => unreachable!("checked in run"),
    }
}

fn paths() -> Result<Done, Fail> {
    let p = Paths::from_env()
        .ok_or_else(|| Fail::new("io", 1, "APPDATA or LOCALAPPDATA is not set"))?;
    let data = json!({
        "config": p.config,
        "local": p.local,
        "settings": p.settings_file(),
        "hotkeys": p.hotkeys_file(),
        "session": p.session_file(),
        "recovery": p.recovery_dir(),
    });
    let text = format!(
        "settings  {}\nhotkeys   {}\nsession   {}\nrecovery  {}",
        p.settings_file().display(),
        p.hotkeys_file().display(),
        p.session_file().display(),
        p.recovery_dir().display()
    );
    Ok((data, text))
}

/// `jarak info <file>`: what Jarak would open, without opening a window or changing anything.
fn info(rest: &[&str], fallback: Option<&str>) -> Result<Done, Fail> {
    use jarak_core::file;
    let [arg] = rest else {
        return Err(Fail::usage("`jarak info` takes one file"));
    };
    let path = absolute(Path::new(arg));
    let fallback = match fallback {
        Some(l) => file::encoding_for_label(l).ok_or_else(|| {
            Fail::usage(format!(
                "unknown encoding `{l}` (use a label such as euc-kr, windows-1252, shift_jis)"
            ))
        })?,
        None => file::system_encoding(),
    };
    let d = read_file(&path, fallback)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let lines = if d.text.is_empty() {
        0
    } else {
        d.eol.crlf + d.eol.lf + d.eol.cr + usize::from(!d.text.ends_with(['\n', '\r']))
    };
    let data = json!({
        "path": path,
        "size": size,
        "encoding": d.format.encoding.name(),
        "label": d.format.encoding.label(d.format.bom),
        "bom": d.format.bom,
        "eol": d.eol.name(),
        "eol_counts": { "crlf": d.eol.crlf, "lf": d.eol.lf, "cr": d.eol.cr },
        "lines": lines,
        "chars": d.text.chars().count(),
        "binary": d.binary,
        "lossless": d.lossless,
        "read_only": d.read_only(),
        "fallback": fallback.name(),
    });
    let text = format!(
        "{}\n  encoding  {}\n  eol       {} (CRLF {}, LF {}, CR {})\n  size      {size} bytes, {lines} lines\n  saving    {}",
        path.display(),
        d.format.encoding.label(d.format.bom),
        d.eol.name(),
        d.eol.crlf,
        d.eol.lf,
        d.eol.cr,
        if d.binary {
            "read-only: a binary file (zero bytes)"
        } else if !d.lossless {
            "read-only: the encoding cannot reproduce every byte"
        } else {
            "unchanged bytes on save"
        }
    );
    Ok((data, text))
}

/// Reads a file for a command, with the CLI's errors.
fn read_file(
    path: &Path,
    fallback: jarak_core::file::TextEncoding,
) -> Result<jarak_core::file::Decoded, Fail> {
    use jarak_core::file::{self, OpenError};
    file::read(path, fallback, file::DEFAULT_SIZE_LIMIT).map_err(|e| match e {
        OpenError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => Fail::new(
            "file_not_found",
            3,
            format!("{} does not exist", path.display()),
        ),
        OpenError::Directory => Fail::usage(format!("{} is a folder", path.display())),
        OpenError::TooLarge { size, limit } => Fail::new(
            "file_too_large",
            5,
            format!(
                "{} has {size} bytes; Jarak opens up to {limit}",
                path.display()
            ),
        ),
        OpenError::Io(e) => Fail::new(
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "permission_denied"
            } else {
                "io"
            },
            1,
            format!("cannot read {}: {e}", path.display()),
        ),
    })
}

/// A Markdown document named on the command line, read.
fn markdown_doc(arg: &str) -> Result<(PathBuf, jarak_core::file::Decoded), Fail> {
    let path = absolute(Path::new(arg));
    if !jarak_core::markdown::is_markdown_path(&path) {
        return Err(Fail::usage(format!(
            "{} is not a Markdown file (.md, .markdown, .mdown, .mkd)",
            path.display()
        )));
    }
    let d = read_file(&path, jarak_core::file::system_encoding())?;
    Ok((path, d))
}

/// The path of an asset relative to the document's folder, with forward slashes.
fn shown(doc: &Path, p: &Path) -> String {
    doc.parent()
        .and_then(|d| p.strip_prefix(d).ok())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `jarak assets list <doc.md>`: the document's images and the folder's orphaned assets.
fn assets_list(rest: &[&str]) -> Result<Done, Fail> {
    use jarak_core::assets;
    let [arg] = rest else {
        return Err(Fail::usage("`jarak assets list` takes one Markdown file"));
    };
    let (path, d) = markdown_doc(arg)?;
    let mut refs = Vec::new();
    let mut lines = Vec::new();
    let (mut found, mut missing, mut remote) = (0, 0, 0);
    for r in assets::references(&d.text) {
        let lower = r.url.trim().to_ascii_lowercase();
        let is_remote = lower.contains("://") || lower.starts_with("data:");
        let local = assets::resolve(&path, &r.url);
        let meta = local
            .as_deref()
            .and_then(|p| std::fs::metadata(p).ok())
            .filter(|m| m.is_file());
        let format = meta
            .as_ref()
            .and(local.as_deref())
            .and_then(|p| assets::image_format(&assets::file_head(p)));
        let state = if is_remote {
            remote += 1;
            "remote, never loaded".to_string()
        } else if local.is_none() {
            missing += 1;
            "outside the document's folder (not shown)".to_string()
        } else if let Some(m) = &meta {
            found += 1;
            format!("{}, {} bytes", format.unwrap_or("not an image"), m.len())
        } else {
            missing += 1;
            "missing".to_string()
        };
        lines.push(format!("  line {:<5} {}  ({state})", r.line + 1, r.url));
        refs.push(json!({
            "reference": r.url,
            "line": r.line + 1,
            "remote": is_remote,
            "path": local,
            "exists": meta.is_some(),
            "bytes": meta.as_ref().map(std::fs::Metadata::len),
            "format": format,
            "rewritable": r.span.is_some(),
        }));
    }
    let orphans = assets::orphans(&path);
    let orphan_json: Vec<Value> = orphans
        .iter()
        .map(|p| {
            json!({
                "path": p,
                "name": shown(&path, p),
                "bytes": std::fs::metadata(p).map(|m| m.len()).ok(),
            })
        })
        .collect();
    let mut text = format!(
        "{}: {} image reference{} ({found} found, {missing} missing, {remote} remote), {} orphaned asset{}",
        path.display(),
        refs.len(),
        if refs.len() == 1 { "" } else { "s" },
        orphans.len(),
        if orphans.len() == 1 { "" } else { "s" },
    );
    for l in &lines {
        text.push('\n');
        text.push_str(l);
    }
    if !orphans.is_empty() {
        text.push_str(
            "\norphans (no Markdown file in this folder refers to them; nothing is deleted):",
        );
        for p in &orphans {
            text.push_str(&format!("\n  {}", shown(&path, p)));
        }
    }
    let data = json!({
        "document": path,
        "assets_dir": path.parent().map(|p| p.join(assets::ASSET_DIR)),
        "references": refs,
        "found": found,
        "missing": missing,
        "remote": remote,
        "orphans": orphan_json,
    });
    Ok((data, text))
}

/// `jarak assets rename <doc.md> <asset> <new-name>`: renames the file (never replacing one)
/// and rewrites the document's references in place (its encoding and line endings kept).
fn assets_rename(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    use jarak_core::assets::{self, RenameError};
    let [arg, old, new] = rest else {
        return Err(Fail::usage(
            "`jarak assets rename` takes a Markdown file, the asset and the new name",
        ));
    };
    let (path, d) = markdown_doc(arg)?;
    if d.read_only() {
        return Err(Fail {
            hint: Some("`jarak info` says why".into()),
            ..Fail::new(
                "document_read_only",
                7,
                format!(
                    "{} opens read-only in Jarak; nothing was changed",
                    path.display()
                ),
            )
        });
    }
    let plan = assets::plan_rename(&path, &d.text, old, new).map_err(|e| {
        let msg = format!("{e}; nothing was changed");
        match e {
            RenameError::NotReferenced(_) | RenameError::Missing(_) => {
                Fail::new("asset_not_found", 3, msg)
            }
            RenameError::BadName(_) => Fail::usage(msg),
            RenameError::Exists(_) => Fail {
                hint: Some("choose another name".into()),
                ..Fail::new("asset_exists", 7, msg)
            },
            RenameError::Shared(v) => Fail {
                hint: Some("rename it by hand in every document, or keep the name".into()),
                details: v.iter().map(|p| json!({ "document": p })).collect(),
                ..Fail::new("asset_shared", 7, msg)
            },
            RenameError::Unlocated(_) => Fail::new("reference_unlocated", 7, msg),
        }
    })?;
    let mut data = json!({
        "document": path,
        "from": plan.from,
        "to": plan.to,
        "reference": plan.reference,
        "lines": plan.lines.iter().map(|l| l + 1).collect::<Vec<_>>(),
        "dry_run": g.dry_run,
        "renamed": false,
    });
    let what = format!(
        "{} -> {} and {} reference{} on line{} {}",
        shown(&path, &plan.from),
        shown(&path, &plan.to),
        plan.lines.len(),
        if plan.lines.len() == 1 { "" } else { "s" },
        if plan.lines.len() == 1 { "" } else { "s" },
        plan.lines
            .iter()
            .map(|l| (l + 1).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if g.dry_run {
        return Ok((
            data,
            format!("dry run, nothing changed: would rename {what}"),
        ));
    }
    let bytes = jarak_core::file::encode(&plan.text, d.format).map_err(|e| {
        Fail::new(
            "document_read_only",
            7,
            format!("cannot encode the new text ({e:?}); nothing was changed"),
        )
    })?;
    assets::move_no_replace(&plan.from, &plan.to).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Fail::new(
                "asset_exists",
                7,
                format!("{} exists already; nothing was changed", plan.to.display()),
            )
        } else {
            Fail::new(
                "io",
                1,
                format!(
                    "cannot rename {}: {e}; nothing was changed",
                    plan.from.display()
                ),
            )
        }
    })?;
    if let Err(e) = jarak_core::file::save_atomic(&path, &bytes) {
        // Put the file back: the document still refers to the old name.
        let back = assets::move_no_replace(&plan.to, &plan.from);
        return Err(Fail::new(
            "io",
            1,
            format!(
                "cannot save {}: {e}; {}",
                path.display(),
                if back.is_ok() {
                    "the asset keeps its old name".to_string()
                } else {
                    format!("the asset is now {}", plan.to.display())
                }
            ),
        ));
    }
    data["renamed"] = json!(true);
    Ok((data, format!("renamed {what}")))
}

/// `jarak-gui.exe` next to this program.
fn gui_exe() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?.with_file_name(GUI_EXE);
    p.is_file().then_some(p)
}

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn open(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    let mut targets = Vec::new();
    for a in rest {
        let t = OpenTarget::parse(a).ok_or_else(|| Fail::usage("an empty file name"))?;
        // Step 93: a folder opens as the window's workspace.
        targets.push(OpenTarget {
            path: absolute(&t.path),
            ..t
        });
    }
    let gui = gui_exe().ok_or_else(|| Fail {
        hint: Some("reinstall Jarak with Kim (`kim install jarak`)".into()),
        ..Fail::new(
            "not_installed",
            3,
            format!("{GUI_EXE} is not next to jarak.exe"),
        )
    })?;
    let listed: Vec<Value> = targets
        .iter()
        .map(|t| {
            json!({
                "path": t.path,
                "line": t.line,
                "column": t.column,
                "exists": t.path.is_file(),
                "folder": t.path.is_dir(),
            })
        })
        .collect();
    let mut argv: Vec<String> = targets.iter().map(OpenTarget::to_arg).collect();
    if g.new_window {
        argv.push("--new-window".into());
    }
    let mut data = json!({
        "gui": gui,
        "targets": listed,
        "new_window": g.new_window,
        "dry_run": g.dry_run,
        "started": false,
    });
    let what = if targets.is_empty() {
        "a new document".to_string()
    } else {
        targets
            .iter()
            .map(|t| t.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if g.dry_run {
        return Ok((
            data,
            format!("dry run, nothing started: would open {what} in Jarak"),
        ));
    }
    let pid = start_detached(&gui, &argv)
        .map_err(|e| Fail::new("io", 1, format!("cannot start {}: {e}", gui.display())))?;
    data["started"] = json!(true);
    data["pid"] = json!(pid);
    Ok((data, format!("opened {what} in Jarak")))
}

/// Starts the window without a console, detached from this process (it keeps running after
/// `jarak` exits; a running window takes the files and the new process ends at once).
fn start_detached(exe: &Path, argv: &[String]) -> std::io::Result<u32> {
    let mut c = std::process::Command::new(exe);
    c.args(argv)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(dir) = exe.parent() {
        c.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        c.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    Ok(c.spawn()?.id())
}

fn finish(command: &str, g: &Global, result: Result<Done, Fail>) -> ExitCode {
    match result {
        Ok((data, text)) => {
            if g.json {
                let doc = json!({
                    "ok": true,
                    "command": command,
                    "schema_version": 1,
                    "data": data,
                    "warnings": [],
                });
                println!("{doc}");
            } else if !text.is_empty() {
                println!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(f) => {
            if g.json {
                let doc = json!({
                    "ok": false,
                    "command": command,
                    "schema_version": 1,
                    "error": {
                        "code": f.code,
                        "exit_code": f.exit,
                        "message": f.message,
                        "hint": f.hint,
                        "details": f.details,
                    },
                });
                println!("{doc}");
            } else {
                eprintln!("jarak: error: {}", f.message);
                if let Some(h) = &f.hint {
                    eprintln!("hint: {h}");
                }
                if g.verbose {
                    eprintln!("jarak: error code {} (exit {})", f.code, f.exit);
                }
            }
            ExitCode::from(f.exit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capabilities_document_names_this_version_and_every_command() {
        let doc: Value = serde_json::from_str(CAPABILITIES).unwrap();
        assert_eq!(doc["tool"], "jarak");
        assert_eq!(doc["version"], jarak_core::VERSION);
        let names: Vec<&str> = doc["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "capabilities",
                "version",
                "paths",
                "open",
                "info",
                "assets.list",
                "assets.rename"
            ]
        );
        // Only `open` may open a window; the commands that change something have a dry run
        // (CLI_CONTRACT §4).
        for c in doc["commands"].as_array().unwrap() {
            let open = c["name"] == "open";
            assert_eq!(c["opens_window"], open, "{c}");
            assert_eq!(c["dry_run"], c["effect"] == "write", "{c}");
        }
    }

    #[test]
    fn usage_errors_are_refused_before_anything_runs() {
        let g = Global::default();
        assert_eq!(dispatch("version", &["x"], &g).unwrap_err().code, "usage");
        assert_eq!(dispatch("paths", &["x"], &g).unwrap_err().code, "usage");
        let e = open(&[""], &g).unwrap_err();
        assert_eq!(e.code, "usage");
        assert_eq!(info(&[], None).unwrap_err().code, "usage");
        assert_eq!(info(&["a", "b"], None).unwrap_err().code, "usage");
        assert_eq!(info(&["a"], Some("klingon")).unwrap_err().code, "usage");
        let missing = std::env::temp_dir().join("jarak-info-missing-file.txt");
        assert_eq!(
            info(&[missing.to_str().unwrap()], None).unwrap_err().code,
            "file_not_found"
        );
    }
}
