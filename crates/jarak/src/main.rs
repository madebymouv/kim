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
network and changes a file only when a command says so (`jarak assets rename`, `jarak replace`,
`jarak journal undo`).

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
  find <file|folder> <text>
                The matches of <text> in a file, or in a folder's files, with their line and
                column (read-only); --extended (\\n \\t \\xNN escapes) or --regex, --match-case,
                --whole-word, --dot-all, --count (only the number); for a folder --filters
                '*.md !*.min.md !+\\build', --no-subfolders, --hidden
  replace <file|folder> <text> <with>
                Replace in Files: --dry-run lists every change and writes nothing; --yes
                replaces (each file kept in its encoding and line endings, the originals in a
                journal); the same search options as find
  export <file> --output <path>
                HTML, RTF or a folder, locally (nothing is uploaded or fetched): highlighted
                text (--line-numbers), a Markdown document rendered with its images embedded
                (--linked: referred to instead; --source: its highlighted text); --to folder
                publishes the page and copies of its images; the format follows the output's
                extension or --to html|rtf|folder; an existing output is replaced only with
                --overwrite, the document never; --to pdf (or a .pdf output) prints to
                Microsoft Print to PDF; --dry-run checks only
  print <file>  Prints on the default printer or --printer <name> with the print settings
                (jarak settings list files): a Markdown document rendered, other text
                highlighted; --pages 2-3; a printer that writes a file (Microsoft Print to PDF)
                needs --output <file> (an existing one only with --overwrite); no dialog ever
                opens; --dry-run lays the pages out and prints nothing
  printers      The printers Windows knows, the default one and those that write a file
                (read-only)
  journal list  The Replace in Files batches kept for undo (read-only)
  journal undo <batch>
                Gives the files of a batch their original bytes back, unless changed since;
                --dry-run checks, --yes restores
  settings list [<section>]
                Every setting (or a section's: general, editor, appearance, markdown, images,
                vim, files, advanced) with its value and whether it is the default (read-only)
  settings get <key>
                One setting's value (read-only)
  settings set <key> <value>
                Checks the value and writes it into settings.toml (its other lines kept);
                --dry-run checks only; a running window applies it when it gets the focus
  settings reset <key>
                The setting goes back to its default; --dry-run checks only
  hotkeys list  Every remappable command with its keys (the user's or the defaults) and
                conflicts (read-only; the shared hotkey contract)
  hotkeys show <action>
                One command's keys and conflicts (read-only)
  hotkeys set <action> <keys>...
                Gives the command these keys (Ctrl+Shift+R, F5); keys another of your own
                bindings has are refused unless --replace; --dry-run checks only
  hotkeys disable <action>
                The command gets no keys; --dry-run checks only
  hotkeys reset <action>
                The command's default keys again; --dry-run checks only
  hotkeys check Conflicts of Jarak's shortcuts (read-only)
  paths         Where Jarak keeps its settings, session and recovery files (read-only)
  version       Jarak's version and the contract and schema versions it speaks (read-only)
  capabilities  Jarak's capabilities document: its commands and what each may change (read-only)
  help          Print this message

Options:
      --json      Print exactly one JSON document on stdout instead of human-readable text
      --dry-run   Show what would happen and change nothing (open, assets rename, replace,
                  journal undo)
  -y, --yes       Confirm a command that changes files (replace, journal undo)
  -q, --quiet     Print only results and errors
  -v, --verbose   Print extra diagnostics on stderr
      --no-input  Never ask questions (Jarak's CLI never asks)
  -h, --help      Print help
  -V, --version   Print version

Exit codes: 0 ok · 1 failure (I/O, permission) · 2 usage · 3 not found · 5 too large ·
7 conflict (a file exists, an asset is shared, the document is read-only, --yes needed)
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
    /// Step 94: `jarak find` options.
    search: jarak_core::search::Query,
    count: bool,
    search_flags: bool,
    /// Step 95: Find / Replace in Files.
    filters: Option<String>,
    no_subfolders: bool,
    hidden: bool,
    folder_flags: bool,
    yes: bool,
    /// Step 105: `jarak hotkeys set --replace` takes keys another binding has.
    replace: bool,
    /// Step 106: `jarak export` options.
    to: Option<String>,
    output: Option<String>,
    linked: bool,
    line_numbers: bool,
    source: bool,
    overwrite: bool,
    export_flags: bool,
    /// Step 106: `jarak print` options.
    printer: Option<String>,
    pages: Option<String>,
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
            "--filters" => match it.next() {
                Some(v) => {
                    g.filters = Some(v.clone());
                    g.folder_flags = true;
                }
                None => missing_value = Some("--filters"),
            },
            "--no-subfolders" => {
                g.no_subfolders = true;
                g.folder_flags = true;
            }
            "--hidden" => {
                g.hidden = true;
                g.folder_flags = true;
            }
            "--to" => match it.next() {
                Some(v) => {
                    g.to = Some(v.clone());
                    g.export_flags = true;
                }
                None => missing_value = Some("--to"),
            },
            "-o" | "--output" => match it.next() {
                Some(v) => {
                    g.output = Some(v.clone());
                    g.export_flags = true;
                }
                None => missing_value = Some("--output"),
            },
            "--printer" => match it.next() {
                Some(v) => g.printer = Some(v.clone()),
                None => missing_value = Some("--printer"),
            },
            "--pages" => match it.next() {
                Some(v) => g.pages = Some(v.clone()),
                None => missing_value = Some("--pages"),
            },
            "--linked" => {
                g.linked = true;
                g.export_flags = true;
            }
            "--line-numbers" => {
                g.line_numbers = true;
                g.export_flags = true;
            }
            "--source" => {
                g.source = true;
                g.export_flags = true;
            }
            "--overwrite" => {
                g.overwrite = true;
                g.export_flags = true;
            }
            "-y" | "--yes" => g.yes = true,
            "--replace" => g.replace = true,
            "--" => rest_are_files = true,
            "--json" => g.json = true,
            "--dry-run" => g.dry_run = true,
            "-q" | "--quiet" => g.quiet = true,
            "-v" | "--verbose" => g.verbose = true,
            "--no-input" => {}
            "--new-window" => g.new_window = true,
            "--regex" => {
                g.search.mode = jarak_core::search::Mode::Regex;
                g.search_flags = true;
            }
            "--extended" => {
                g.search.mode = jarak_core::search::Mode::Extended;
                g.search_flags = true;
            }
            "--match-case" => {
                g.search.match_case = true;
                g.search_flags = true;
            }
            "--whole-word" => {
                g.search.whole_word = true;
                g.search_flags = true;
            }
            "--dot-all" => {
                g.search.dot_all = true;
                g.search_flags = true;
            }
            "--count" => {
                g.count = true;
                g.search_flags = true;
            }
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
        (Some("journal"), Some("list")) => "journal.list",
        (Some("journal"), Some("undo")) => "journal.undo",
        (Some("hotkeys"), Some("list")) => "hotkeys.list",
        (Some("hotkeys"), Some("show")) => "hotkeys.show",
        (Some("hotkeys"), Some("set")) => "hotkeys.set",
        (Some("hotkeys"), Some("disable")) => "hotkeys.disable",
        (Some("hotkeys"), Some("reset")) => "hotkeys.reset",
        (Some("hotkeys"), Some("check")) => "hotkeys.check",
        (Some("settings"), Some("list")) => "settings.list",
        (Some("settings"), Some("get")) => "settings.get",
        (Some("settings"), Some("set")) => "settings.set",
        (Some("settings"), Some("reset")) => "settings.reset",
        (
            Some(
                c @ ("open" | "info" | "find" | "replace" | "export" | "print" | "printers"
                | "paths" | "version" | "capabilities"),
            ),
            _,
        ) => c,
        _ => "jarak",
    };
    let skip = if command.contains('.') { 2 } else { 1 };
    let result = if let Some(flag) = unknown {
        Err(Fail::usage(format!("unknown option `{flag}`")))
    } else if let Some(flag) = missing_value {
        Err(Fail::usage(format!("{flag} needs a value")))
    } else if g.fallback.is_some() && !matches!(command, "info" | "find" | "replace") {
        Err(Fail::usage(
            "--fallback belongs to `jarak info`, `jarak find` and `jarak replace`",
        ))
    } else if g.search_flags && !matches!(command, "find" | "replace") {
        Err(Fail::usage(
            "search options belong to `jarak find` and `jarak replace`",
        ))
    } else if g.count && command == "replace" {
        Err(Fail::usage("--count belongs to `jarak find`"))
    } else if g.folder_flags && !matches!(command, "find" | "replace") {
        Err(Fail::usage(
            "--filters, --no-subfolders and --hidden belong to `jarak find` and `jarak replace`",
        ))
    } else if g.yes && !matches!(command, "replace" | "journal.undo") {
        Err(Fail::usage(
            "--yes belongs to `jarak replace` and `jarak journal undo`",
        ))
    } else if g.replace && command != "hotkeys.set" {
        Err(Fail::usage("--replace belongs to `jarak hotkeys set`"))
    } else if g.export_flags && !matches!(command, "export" | "print") {
        Err(Fail::usage(
            "--to, --output, --linked, --line-numbers, --source and --overwrite belong to `jarak export` (--output and --overwrite also to `jarak print`)",
        ))
    } else if (g.printer.is_some() || g.pages.is_some()) && command != "print" {
        Err(Fail::usage("--printer and --pages belong to `jarak print`"))
    } else if g.quiet && g.verbose {
        Err(Fail::usage("--quiet and --verbose conflict"))
    } else if command == "jarak" {
        Err(Fail::usage(match (words.first(), words.get(1)) {
            (Some(&"assets"), Some(w)) => format!("unknown command `assets {w}` (list, rename)"),
            (Some(&"assets"), None) => "`jarak assets` needs list or rename".into(),
            (Some(&"journal"), Some(w)) => format!("unknown command `journal {w}` (list, undo)"),
            (Some(&"journal"), None) => "`jarak journal` needs list or undo".into(),
            (Some(&"settings"), Some(w)) => {
                format!("unknown command `settings {w}` (list, get, set, reset)")
            }
            (Some(&"settings"), None) => "`jarak settings` needs list, get, set or reset".into(),
            (Some(&"hotkeys"), Some(w)) => {
                format!("unknown command `hotkeys {w}` (list, show, set, disable, reset, check)")
            }
            (Some(&"hotkeys"), None) => {
                "`jarak hotkeys` needs list, show, set, disable, reset or check".into()
            }
            (Some(w), _) => format!("unknown command `{w}`"),
            (None, _) => "no command given".into(),
        }))
    } else if g.new_window && command != "open" {
        Err(Fail::usage("--new-window belongs to `jarak open`"))
    } else if g.dry_run
        && !matches!(
            command,
            "open"
                | "assets.rename"
                | "replace"
                | "export"
                | "print"
                | "journal.undo"
                | "settings.set"
                | "settings.reset"
                | "hotkeys.set"
                | "hotkeys.disable"
                | "hotkeys.reset"
        )
    {
        Err(Fail::usage(format!(
            "`jarak {}` changes nothing; --dry-run belongs to `jarak open`, `jarak assets rename`, `jarak replace`, `jarak export`, `jarak print`, `jarak journal undo`, `jarak settings set/reset` and `jarak hotkeys set/disable/reset`",
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
        "version" | "capabilities" | "paths" | "printers" if !rest.is_empty() => {
            Err(Fail::usage(format!(
                "`jarak {command}` takes no arguments (got `{}`)",
                rest.join(" ")
            )))
        }
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
        "find" => find(rest, g),
        "replace" => replace(rest, g),
        "export" => export(rest, g),
        "print" => print(rest, g),
        "printers" => printers(),
        "journal.list" | "journal.undo" => journal(command, rest, g),
        "assets.list" => assets_list(rest),
        "assets.rename" => assets_rename(rest, g),
        "settings.list" | "settings.get" | "settings.set" | "settings.reset" => {
            settings(command, rest, g)
        }
        c if c.starts_with("hotkeys.") => {
            let p = Paths::from_env()
                .ok_or_else(|| Fail::new("io", 1, "APPDATA or LOCALAPPDATA is not set"))?;
            hotkeys_in(command, rest, g, &p.hotkeys_file())
        }
        _ => unreachable!("checked in run"),
    }
}

/// Step 105: `jarak hotkeys …` on the user layer `file` (the shared hotkey contract; headless —
/// the window reads the file again when it gets the focus).
fn hotkeys_in(command: &str, rest: &[&str], g: &Global, file: &Path) -> Result<Done, Fail> {
    use jarak_core::hotkeys::{self as jh, Change};
    use kim_hotkeys::check::Conflict;
    let registration = |_: &str, _: &str| "not_applicable";
    let refused = |e: jh::Refused| {
        let (code, exit) = e.code();
        let mut f = Fail::new(code, exit, format!("{e}; nothing was changed"));
        if let jh::Refused::Edit(r) = &e {
            f.hint.clone_from(&r.hint);
            f.details = r.conflicts.iter().map(Conflict::to_json).collect();
        } else if code == "hotkey_invalid" {
            f.hint = Some("keys look like Ctrl+Shift+R or F5 (docs/HOTKEY_CONTRACT.md §5)".into());
        }
        f
    };
    let (tool, loaded) = jh::load(file);
    let path = loaded.path.display().to_string();
    match command {
        "hotkeys.list" | "hotkeys.check" if !rest.is_empty() => Err(Fail::usage(format!(
            "`jarak {}` takes no arguments",
            command.replace('.', " ")
        ))),
        "hotkeys.list" => {
            let a = jh::analysis(&tool);
            let data = kim_hotkeys::view::list_data(&tool, &a, &path, loaded.state, &registration);
            let human = tool
                .manifest
                .actions
                .iter()
                .map(|act| {
                    let (keys, changed) = jh::keys_of(&tool, &act.id);
                    let shown = if keys.is_empty() {
                        "(off)".to_string()
                    } else {
                        keys.join(" ")
                    };
                    format!(
                        "{:32} {shown}{}",
                        act.id,
                        if changed { "  (changed)" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok((data, human))
        }
        "hotkeys.show" => {
            let [action] = rest else {
                return Err(Fail::usage("`jarak hotkeys show` needs one action"));
            };
            let a = jh::analysis(&tool);
            let data = kim_hotkeys::view::show_data(&tool, &a, action, &registration).ok_or_else(
                || Fail {
                    hint: Some("`jarak hotkeys list` names every action".into()),
                    ..Fail::new(
                        "action_not_found",
                        3,
                        format!("jarak has no action `{action}`"),
                    )
                },
            )?;
            let (keys, _) = jh::keys_of(&tool, action);
            Ok((data, format!("{action}: {}", keys.join(" "))))
        }
        "hotkeys.set" | "hotkeys.disable" | "hotkeys.reset" => {
            let c = match (command, rest) {
                ("hotkeys.set", [action, keys @ ..]) if !keys.is_empty() => Change::Set {
                    action: (*action).to_string(),
                    keys: keys.iter().map(|k| (*k).to_string()).collect(),
                },
                ("hotkeys.disable", [action]) => Change::Disable((*action).to_string()),
                ("hotkeys.reset", [action]) => Change::Reset((*action).to_string()),
                _ => {
                    return Err(Fail::usage(format!(
                        "`jarak {}` needs an action{}",
                        command.replace('.', " "),
                        if command == "hotkeys.set" {
                            " and keys"
                        } else {
                            ""
                        }
                    )));
                }
            };
            let o = jh::change(file, &c, g.replace, g.dry_run).map_err(refused)?;
            let changes: Vec<Value> = o
                .applied
                .changes
                .iter()
                .map(kim_hotkeys::edit::Change::to_json)
                .collect();
            let conflicts: Vec<Value> = o.applied.conflicts.iter().map(Conflict::to_json).collect();
            let mut human: Vec<String> = o
                .applied
                .changes
                .iter()
                .filter(|c| c.before != c.after)
                .map(|c| {
                    format!(
                        "{}: {} -> {}",
                        c.action,
                        c.before.bindings.join(", "),
                        c.after.bindings.join(", ")
                    )
                })
                .collect();
            human.extend(o.applied.conflicts.iter().map(|c| c.message.clone()));
            if !o.applied.changed {
                human.push("Nothing to change.".into());
            } else if g.dry_run {
                human.push("Dry run: nothing was written.".into());
            } else {
                human.push(format!(
                    "Saved to {path}. An open Jarak window applies it when it gets the focus."
                ));
            }
            Ok((
                json!({
                    "tool": jh::TOOL, "dry_run": g.dry_run, "written": o.written, "path": path,
                    "changes": changes, "conflicts": conflicts,
                }),
                human.join("\n"),
            ))
        }
        "hotkeys.check" => {
            let a = jh::analysis(&tool);
            let entries = vec![kim_hotkeys::check::ToolEntry {
                tool: jh::TOOL.into(),
                version: Some(jarak_core::VERSION.into()),
                source: "self",
                enabled: tool.enabled(),
            }];
            let data =
                kim_hotkeys::check::report_json(&entries, &a, kim_hotkeys::check::Severity::Info);
            let human = if a.conflicts.is_empty() {
                "No conflicts.".to_string()
            } else {
                a.conflicts
                    .iter()
                    .map(|c| format!("{}: {}", c.severity().as_str(), c.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok((data, human))
        }
        _ => unreachable!("checked in run"),
    }
}

/// Step 106: `jarak export <file> --output <path>` (`JARAK-FR-141`): HTML, RTF or a published
/// folder, written locally; the document is never changed, an existing output replaced only with
/// `--overwrite`, and nothing is fetched or uploaded.
fn export(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    use jarak_core::export::{self as ex, ExportError, Format, Images};
    let [file] = rest else {
        return Err(Fail::usage(
            "`jarak export` needs one file (and --output <path>)",
        ));
    };
    let Some(output) = g.output.as_deref() else {
        return Err(Fail::usage(
            "`jarak export` needs --output <path> (a file, or a folder with --to folder)",
        ));
    };
    let path = absolute(Path::new(file));
    let out = absolute(Path::new(output));
    let format = match g.to.as_deref() {
        Some(t) => Format::from_name(t).ok_or_else(|| {
            Fail::usage(format!("--to takes html, rtf, folder or pdf (got `{t}`)"))
        })?,
        None => Format::from_extension(&out).ok_or_else(|| Fail {
            hint: Some("name the format with --to html, rtf, folder or pdf".into()),
            ..Fail::usage(format!(
                "{} has no export extension (.html, .htm, .rtf, .pdf)",
                out.display()
            ))
        })?,
    };
    let markdown = jarak_core::markdown::is_markdown_path(&path);
    let rendered = markdown && !g.source && format != Format::Rtf;
    if g.linked && format == Format::Pdf {
        return Err(Fail::usage(
            "--linked belongs to the HTML export of a rendered Markdown document",
        ));
    }
    if g.line_numbers && format == Format::Pdf {
        return Err(Fail::usage(
            "a PDF follows the print settings: `jarak settings set print_line_numbers true`",
        ));
    }
    if g.source && !markdown {
        return Err(Fail::usage(
            "--source belongs to Markdown documents (other documents export their text anyway)",
        ));
    }
    if g.linked && !(rendered && format == Format::Html) {
        return Err(Fail::usage(
            "--linked belongs to the HTML export of a rendered Markdown document",
        ));
    }
    if g.line_numbers && rendered {
        return Err(Fail::usage(
            "--line-numbers belongs to highlighted text; a rendered Markdown document has none (add --source)",
        ));
    }
    if format == Format::Folder && !rendered {
        return Err(Fail::usage(
            "--to folder publishes a rendered Markdown document (.md)",
        ));
    }
    if format == Format::Folder && out.is_file() {
        return Err(Fail::usage(format!(
            "{} is a file; --to folder needs a folder",
            out.display()
        )));
    }
    let d = read_file(&path, jarak_core::file::system_encoding())?;
    if d.binary {
        return Err(Fail::usage(format!(
            "{} is a binary file; Jarak exports text",
            path.display()
        )));
    }
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let user_langs = Paths::from_env()
        .map(|p| jarak_core::syntax::load_udls(&p.languages_dir()).0)
        .unwrap_or_default();
    let first_line = d.text.lines().next().unwrap_or("");
    let lang = jarak_core::syntax::detect(&name, first_line, &user_langs);
    let fail = |e: ExportError| {
        let (code, exit) = e.code();
        let mut f = Fail::new(code, exit, format!("{e}; nothing was written"));
        if let ExportError::Exists(v) = &e {
            f.hint = Some("pass --overwrite to replace them".into());
            f.details = v.iter().map(|p| json!(p.display().to_string())).collect();
        }
        f
    };
    let (files, images) = match format {
        Format::Folder => {
            let p = ex::publish(&path, &d.text, &out, g.overwrite, g.dry_run).map_err(fail)?;
            (p.files, p.images)
        }
        Format::Html => {
            let (html, images) = if rendered {
                let images = if g.linked {
                    Images::Link
                } else {
                    Images::Embed
                };
                let p = ex::markdown_html(&d.text, Some(&path), &name, images, &out);
                (p.html, p.images)
            } else {
                (
                    ex::code_html(&d.text, lang, &name, g.line_numbers),
                    Vec::new(),
                )
            };
            let w = ex::write_output(&out, html.as_bytes(), Some(&path), g.overwrite, g.dry_run)
                .map_err(fail)?;
            (vec![w], images)
        }
        Format::Pdf => {
            if ex::same_file(&out, &path) {
                return Err(fail(ExportError::IntoSource(out)));
            }
            if out.exists() && !g.overwrite {
                return Err(fail(ExportError::Exists(vec![out])));
            }
            print_document(
                &path,
                &d.text,
                rendered,
                lang,
                jarak_core::print::PDF_PRINTER,
                Some(out.clone()),
                None,
                g.dry_run,
            )?;
            let bytes = if g.dry_run {
                0
            } else {
                std::fs::metadata(&out).map_or(0, |m| m.len())
            };
            (
                vec![ex::Written {
                    path: out.clone(),
                    bytes,
                }],
                Vec::new(),
            )
        }
        Format::Rtf => {
            let rtf = ex::code_rtf(&d.text, lang, g.line_numbers);
            let w = ex::write_output(&out, rtf.as_bytes(), Some(&path), g.overwrite, g.dry_run)
                .map_err(fail)?;
            (vec![w], Vec::new())
        }
    };
    let files_json: Vec<Value> = files
        .iter()
        .map(|w| json!({ "path": w.path.display().to_string(), "bytes": w.bytes }))
        .collect();
    let images_json: Vec<Value> = images
        .iter()
        .map(|i| {
            let mut v = json!({ "reference": i.reference, "use": i.used.name() });
            match &i.used {
                ex::ImageUse::Refused(why) => v["reason"] = json!(why),
                ex::ImageUse::Copied(to) => v["published"] = json!(to),
                _ => {}
            }
            v
        })
        .collect();
    let count = |n: &str| images.iter().filter(|i| i.used.name() == n).count();
    let mut human = vec![format!(
        "{} {name} as {} to {} ({} file{}, {} bytes).",
        if g.dry_run {
            "Would export"
        } else {
            "Exported"
        },
        match format {
            Format::Html if rendered => "rendered HTML",
            Format::Html => "highlighted HTML",
            Format::Rtf => "RTF",
            Format::Pdf => "PDF (Microsoft Print to PDF)",
            Format::Folder => "a published folder",
        },
        out.display(),
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        files.iter().map(|w| w.bytes).sum::<u64>()
    )];
    if !images.is_empty() {
        let parts: Vec<String> = [
            ("embedded", "embedded"),
            ("linked", "linked"),
            ("copied", "copied"),
            ("remote", "remote (not loaded)"),
            ("missing", "missing"),
            ("refused", "not used"),
        ]
        .iter()
        .filter(|(k, _)| count(k) > 0)
        .map(|(k, label)| format!("{} {label}", count(k)))
        .collect();
        human.push(format!("Images: {}.", parts.join(", ")));
    }
    if g.dry_run {
        human.push("Dry run: nothing was written.".into());
    }
    Ok((
        json!({
            "source": path.display().to_string(),
            "format": format.name(),
            "output": out.display().to_string(),
            "rendered": rendered,
            "language": lang.map(|l| l.name.clone()),
            "dry_run": g.dry_run,
            "written": !g.dry_run,
            "files": files_json,
            "images": images_json,
        }),
        human.join("\n"),
    ))
}

/// Step 106: lays `text` out with the print settings and prints it on `printer` (into `output`
/// for a printer that writes a file); `pages` limits it to a range. Headless: no dialog.
#[allow(clippy::too_many_arguments)]
fn print_document(
    path: &Path,
    text: &str,
    rendered: bool,
    lang: Option<&jarak_core::syntax::Lang>,
    printer: &str,
    output: Option<PathBuf>,
    pages: Option<(usize, usize)>,
    dry_run: bool,
) -> Result<(jarak_core::print::Printed, String), Fail> {
    use jarak_core::print::{self as pr, Printers};
    let settings =
        Paths::from_env().map(|p| jarak_core::settings::Settings::new(&p.settings_file()));
    let get = |k: &str| settings.as_ref().and_then(|s| s.get(k));
    let tab = get("tab_width")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4);
    let setup = jarak_core::settings::print_setup(&get, tab);
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let (date, time) = pr::local_now();
    let vars = pr::Vars {
        file: name.clone(),
        path: path.display().to_string(),
        date,
        time,
    };
    let backend = pr::gdi::WinPrinters;
    let Some(p) = backend
        .list()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(printer))
    else {
        return Err(Fail {
            hint: Some("`jarak printers` names them".into()),
            ..Fail::new(
                "printer_not_found",
                3,
                format!("there is no printer called `{printer}`"),
            )
        });
    };
    if p.writes_a_file() && output.is_none() {
        return Err(Fail::usage(format!(
            "`{}` writes a file: name it with --output <file> (it would ask otherwise, and the command line never asks)",
            p.name
        )));
    }
    if !p.writes_a_file() && output.is_some() {
        return Err(Fail::usage(format!(
            "--output belongs to a printer that writes a file; `{}` prints on paper",
            p.name
        )));
    }
    let job = pr::Job {
        printer: p.name.clone(),
        output,
        devmode: None,
        name: format!("Jarak — {name}"),
        dry_run,
    };
    let lang = lang.cloned();
    let text = text.to_string();
    let doc = path.to_path_buf();
    let mut layout = move |m: &dyn pr::Measure, g: &pr::Geometry| {
        let all = if rendered {
            pr::markdown_pages(&text, Some(&doc), &setup, g, m, &vars)
        } else {
            pr::code_pages(&text, lang.as_ref(), &setup, g, m, &vars)
        };
        match pages {
            Some((a, b)) => pr::page_range(all, a, b),
            None => all,
        }
    };
    let printed = backend.print(&job, &mut layout).map_err(|e| {
        let (code, exit) = e.code();
        Fail::new(code, exit, format!("{e}; nothing was printed"))
    })?;
    Ok((printed, p.name))
}

/// Step 106: `jarak print <file> [--printer <name>] [--output <file>] [--pages a-b]`.
fn print(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    let [file] = rest else {
        return Err(Fail::usage("`jarak print` needs one file"));
    };
    if g.to.is_some() || g.linked || g.source || g.line_numbers {
        return Err(Fail::usage(
            "--to, --linked, --source and --line-numbers belong to `jarak export`; printing follows the print settings (`jarak settings list files`)",
        ));
    }
    let pages = match g.pages.as_deref() {
        None => None,
        Some(r) => {
            let parse = |s: &str| s.trim().parse::<usize>().ok().filter(|n| *n > 0);
            let range = match r.split_once('-') {
                Some((a, b)) => parse(a).zip(parse(b)),
                None => parse(r).map(|n| (n, n)),
            };
            match range {
                Some((a, b)) if a <= b => Some((a, b)),
                _ => {
                    return Err(Fail::usage(format!(
                        "--pages takes a page or a range like 2-5 (got `{r}`)"
                    )));
                }
            }
        }
    };
    let path = absolute(Path::new(file));
    let printer = match g.printer.clone() {
        Some(p) => p,
        None => jarak_core::print::gdi::default_printer().ok_or_else(|| Fail {
            hint: Some("name one with --printer (`jarak printers` lists them)".into()),
            ..Fail::new("printer_not_found", 3, "Windows has no default printer")
        })?,
    };
    let out = g.output.as_deref().map(|o| absolute(Path::new(o)));
    if let Some(o) = &out {
        if jarak_core::export::same_file(o, &path) {
            return Err(Fail::new(
                "export_into_source",
                2,
                format!(
                    "{} is the document; printing never replaces it",
                    o.display()
                ),
            ));
        }
        if o.exists() && !g.overwrite {
            return Err(Fail {
                hint: Some("pass --overwrite to replace it".into()),
                details: vec![json!(o.display().to_string())],
                ..Fail::new(
                    "export_exists",
                    7,
                    format!("{} already exists; nothing was printed", o.display()),
                )
            });
        }
    }
    let d = read_file(&path, jarak_core::file::system_encoding())?;
    if d.binary {
        return Err(Fail::usage(format!(
            "{} is a binary file; Jarak prints text",
            path.display()
        )));
    }
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let user_langs = Paths::from_env()
        .map(|p| jarak_core::syntax::load_udls(&p.languages_dir()).0)
        .unwrap_or_default();
    let lang = jarak_core::syntax::detect(&name, d.text.lines().next().unwrap_or(""), &user_langs);
    let rendered = jarak_core::markdown::is_markdown_path(&path);
    let (printed, printer) = print_document(
        &path,
        &d.text,
        rendered,
        lang,
        &printer,
        out.clone(),
        pages,
        g.dry_run,
    )?;
    let human = match (&out, g.dry_run) {
        (_, true) => format!(
            "Would print {} page{} of {name} on {printer}. Dry run: nothing was printed.",
            printed.pages,
            if printed.pages == 1 { "" } else { "s" }
        ),
        (Some(o), false) => format!(
            "Printed {} page{} of {name} on {printer} into {}.",
            printed.pages,
            if printed.pages == 1 { "" } else { "s" },
            o.display()
        ),
        (None, false) => format!(
            "Printed {} page{} of {name} on {printer}.",
            printed.pages,
            if printed.pages == 1 { "" } else { "s" }
        ),
    };
    Ok((
        json!({
            "source": path.display().to_string(),
            "printer": printer,
            "output": out.map(|o| o.display().to_string()),
            "rendered": rendered,
            "pages": printed.pages,
            "dry_run": g.dry_run,
            "printed": !g.dry_run,
        }),
        human,
    ))
}

/// Step 106: `jarak printers`.
fn printers() -> Result<Done, Fail> {
    let list = jarak_core::print::gdi::printers();
    let data: Vec<Value> = list
        .iter()
        .map(|p| {
            json!({
                "name": p.name, "port": p.port, "default": p.default,
                "writes_file": p.writes_a_file(),
            })
        })
        .collect();
    let human = if list.is_empty() {
        "No printers.".to_string()
    } else {
        list.iter()
            .map(|p| {
                format!(
                    "{}{}{}",
                    p.name,
                    if p.default { "  (default)" } else { "" },
                    if p.writes_a_file() {
                        "  (writes a file: --output)"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok((json!({ "printers": data }), human))
}

/// What values a setting takes, for the CLI's JSON.
fn kind_json(k: jarak_core::settings::Kind) -> Value {
    use jarak_core::settings::Kind;
    match k {
        Kind::Bool => json!({ "type": "bool" }),
        Kind::Int { min, max } => json!({ "type": "int", "min": min, "max": max }),
        Kind::Choice(list) => {
            json!({ "type": "choice", "values": list.iter().map(|(v, _, _)| *v).collect::<Vec<_>>() })
        }
        Kind::Text { must: "" } => json!({ "type": "text" }),
        Kind::Text { must } => json!({ "type": "text", "must_contain": must }),
        Kind::Columns => json!({ "type": "columns" }),
    }
}

/// Step 105 (`JARAK-FR-130`): `jarak settings list | get | set | reset` on `settings.toml`
/// (headless; the window re-reads the file when it gets the focus).
fn settings(command: &str, rest: &[&str], g: &Global) -> Result<Done, Fail> {
    let p = Paths::from_env()
        .ok_or_else(|| Fail::new("io", 1, "APPDATA or LOCALAPPDATA is not set"))?;
    settings_in(command, rest, g, &p.settings_file())
}

/// The settings commands on the settings file `file`.
fn settings_in(command: &str, rest: &[&str], g: &Global, file: &Path) -> Result<Done, Fail> {
    use jarak_core::settings::{self as st, Refused, Section, SetError, Settings};
    let store = Settings::new(file);
    let not_found = |k: &str| Fail {
        hint: Some("`jarak settings list` names every setting".into()),
        ..Fail::new(
            "setting_not_found",
            3,
            format!("there is no setting `{k}`; nothing was changed"),
        )
    };
    let refused = |r: Refused| match r {
        Refused::UnknownSetting(k) => not_found(&k),
        Refused::InvalidValue { key, why } => Fail {
            hint: Some(format!("`{key}` takes {why}")),
            ..Fail::new(
                "setting_invalid",
                2,
                format!("`{key}` does not take that value ({why}); nothing was changed"),
            )
        },
    };
    let io = |e: SetError| match e {
        SetError::Refused(r) => refused(r),
        SetError::Io(e) => Fail::new("io", 1, format!("{}: {e}", file.display())),
    };
    match command {
        "settings.list" => {
            let section = match rest {
                [] => None,
                [s] => Some(Section::from_id(s).ok_or_else(|| {
                    Fail::usage(format!(
                        "unknown section `{s}` (general, editor, appearance, markdown, images, vim, files, advanced)"
                    ))
                })?),
                _ => return Err(Fail::usage("`jarak settings list` takes at most a section")),
            };
            let list: Vec<_> = store
                .list()
                .into_iter()
                .filter(|(s, _, _)| section.is_none_or(|x| s.section == x))
                .collect();
            let width = list.iter().map(|(s, _, _)| s.key.len()).max().unwrap_or(0);
            let human = list
                .iter()
                .map(|(s, v, d)| {
                    let shown = if v.is_empty() { "(none)" } else { v.as_str() };
                    let mark = if *d { "" } else { "  (changed)" };
                    format!("{:width$}  {shown}{mark}", s.key)
                })
                .collect::<Vec<_>>()
                .join("\n");
            let items: Vec<Value> = list
                .iter()
                .map(|(s, v, d)| {
                    json!({
                        "key": s.key,
                        "section": s.section.id(),
                        "title": s.en,
                        "value": v,
                        "default": s.default,
                        "is_default": d,
                        "kind": kind_json(s.kind),
                    })
                })
                .collect();
            Ok((json!({ "file": file, "settings": items }), human))
        }
        "settings.get" => {
            let [key] = rest else {
                return Err(Fail::usage("`jarak settings get` needs one setting"));
            };
            let s = st::find(key).ok_or_else(|| not_found(key))?;
            let v = store.get(key).unwrap_or_default();
            let is_default = v == s.default;
            Ok((
                json!({ "key": key, "value": v, "default": s.default, "is_default": is_default }),
                v,
            ))
        }
        "settings.set" => {
            let (key, value) = match rest {
                [key, value @ ..] if !value.is_empty() => (*key, value.join(" ")),
                _ => {
                    return Err(Fail::usage(
                        "`jarak settings set` needs a setting and a value",
                    ));
                }
            };
            let s = st::find(key).ok_or_else(|| not_found(key))?;
            let v = st::validate(s, &value).map_err(refused)?;
            let previous = store.get(key).unwrap_or_default();
            let changed = previous != v;
            if !g.dry_run && changed {
                store.set(key, &v).map_err(io)?;
            }
            let verb = if g.dry_run { "would be" } else { "is" };
            Ok((
                json!({ "key": key, "value": v, "previous": previous, "changed": changed, "dry_run": g.dry_run, "file": file }),
                format!("{key} {verb} {v}"),
            ))
        }
        "settings.reset" => {
            let [key] = rest else {
                return Err(Fail::usage("`jarak settings reset` needs one setting"));
            };
            let s = st::find(key).ok_or_else(|| not_found(key))?;
            let previous = store.get(key).unwrap_or_default();
            let written = store.raw(key).is_some();
            if !g.dry_run && written {
                store.reset(key).map_err(io)?;
            }
            let verb = if g.dry_run { "would be" } else { "is" };
            Ok((
                json!({ "key": key, "value": s.default, "previous": previous, "changed": written, "dry_run": g.dry_run, "file": file }),
                format!("{key} {verb} back to its default ({})", s.default),
            ))
        }
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
        "journal": p.journal_dir(),
    });
    let text = format!(
        "settings  {}\nhotkeys   {}\nsession   {}\nrecovery  {}\njournal   {}",
        p.settings_file().display(),
        p.hotkeys_file().display(),
        p.session_file().display(),
        p.recovery_dir().display(),
        p.journal_dir().display()
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

/// `jarak find <file> <text>`: the matches with their line and column (read-only).
fn find(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    use jarak_core::search::{Haystack, Query};
    let [arg, text] = rest else {
        return Err(Fail::usage(
            "`jarak find` takes a file or folder and the text to find",
        ));
    };
    let path = absolute(Path::new(arg));
    let fallback = fallback_of(g)?;
    if path.is_dir() {
        return find_folder(&path, text, g, fallback);
    }
    if g.folder_flags {
        return Err(Fail::usage(
            "--filters, --no-subfolders and --hidden are for a folder",
        ));
    }
    let d = read_file(&path, fallback)?;
    let q = Query {
        text: (*text).to_string(),
        ..g.search.clone()
    };
    let m = q
        .compile()
        .map_err(|e| Fail::usage(format!("the search `{text}` is not valid: {e}")))?;
    let doc = jarak_core::document::Document::from_text(&d.text);
    let h = Haystack::of(&doc);
    let (found, _) = m.find_all(&h, None, &|| false);
    let r = doc.text();
    let mut matches = Vec::new();
    let mut lines_text = Vec::new();
    for (a, b) in found.iter().take(if g.count { 0 } else { 10_000 }) {
        let line = r.char_to_line(*a);
        let start = r.line_to_char(line);
        let line_text: String = r
            .line(line)
            .chars()
            .filter(|c| *c != '\n' && *c != '\r')
            .take(300)
            .collect();
        let col = a - start;
        matches.push(json!({
            "line": line + 1,
            "column": col + 1,
            "length": b - a,
            "text": r.slice(*a..*b).to_string(),
            "line_text": line_text,
        }));
        lines_text.push(format!("{}:{}: {}", line + 1, col + 1, line_text));
    }
    let data = json!({
        "path": path,
        "query": text,
        "mode": mode_name(q.mode),
        "match_case": q.match_case,
        "whole_word": q.whole_word,
        "count": found.len(),
        "matches": matches,
        "truncated": !g.count && found.len() > 10_000,
    });
    let mut text_out = format!(
        "{}: {} match{}",
        path.display(),
        found.len(),
        if found.len() == 1 { "" } else { "es" }
    );
    for l in lines_text {
        text_out.push('\n');
        text_out.push_str(&l);
    }
    Ok((data, text_out))
}

fn fallback_of(g: &Global) -> Result<jarak_core::file::TextEncoding, Fail> {
    use jarak_core::file;
    match g.fallback.as_deref() {
        Some(l) => file::encoding_for_label(l)
            .ok_or_else(|| Fail::usage(format!("unknown encoding `{l}`"))),
        None => Ok(file::system_encoding()),
    }
}

fn compile(text: &str, g: &Global) -> Result<jarak_core::search::Matcher, Fail> {
    jarak_core::search::Query {
        text: text.to_string(),
        ..g.search.clone()
    }
    .compile()
    .map_err(|e| Fail::usage(format!("the search `{text}` is not valid: {e}")))
}

fn mode_name(m: jarak_core::search::Mode) -> &'static str {
    match m {
        jarak_core::search::Mode::Normal => "normal",
        jarak_core::search::Mode::Extended => "extended",
        jarak_core::search::Mode::Regex => "regex",
    }
}

/// Step 95: `jarak find <folder> <text>` — Find in Files with the filters.
fn find_folder(
    dir: &Path,
    text: &str,
    g: &Global,
    fallback: jarak_core::file::TextEncoding,
) -> Result<Done, Fail> {
    use jarak_core::infiles;
    let m = compile(text, g)?;
    let scope = folder_scope(dir, g, fallback);
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let found = infiles::find_in_files(&scope, &m, &cancel, &|_, _| {});
    let mut listed = 0usize;
    let mut human = format!(
        "{}: {} match{} in {} file{} ({} searched{})",
        dir.display(),
        found.hits,
        if found.hits == 1 { "" } else { "es" },
        found.files.len(),
        if found.files.len() == 1 { "" } else { "s" },
        found.searched,
        if found.skipped.is_empty() {
            String::new()
        } else {
            format!(", {} skipped", found.skipped.len())
        }
    );
    let files: Vec<Value> = found
        .files
        .iter()
        .map(|f| {
            let take = if g.count {
                0
            } else {
                10_000usize.saturating_sub(listed)
            };
            let matches: Vec<Value> = f
                .hits
                .iter()
                .take(take)
                .map(|h| {
                    if !g.count {
                        human.push_str(&format!(
                            "\n{}:{}:{}: {}",
                            f.path.display(),
                            h.line,
                            h.column,
                            h.line_text
                        ));
                    }
                    hit_json(h)
                })
                .collect();
            listed += matches.len();
            json!({
                "path": f.path,
                "encoding": f.format.encoding.label(f.format.bom),
                "count": f.hits.len(),
                "matches": matches,
            })
        })
        .collect();
    let data = json!({
        "folder": dir,
        "query": text,
        "mode": mode_name(g.search.mode),
        "match_case": g.search.match_case,
        "whole_word": g.search.whole_word,
        "filters": g.filters.clone().unwrap_or_default(),
        "subfolders": !g.no_subfolders,
        "hidden": g.hidden,
        "count": found.hits,
        "files_searched": found.searched,
        "files": files,
        "skipped": skipped_json(&found.skipped),
        "truncated": found.truncated || (!g.count && found.hits > listed),
    });
    Ok((data, human))
}

fn hit_json(h: &jarak_core::infiles::Hit) -> Value {
    json!({
        "line": h.line,
        "column": h.column,
        "line_text": h.line_text.chars().take(300).collect::<String>(),
    })
}

fn skipped_json(s: &[(PathBuf, jarak_core::infiles::Skip)]) -> Vec<Value> {
    s.iter()
        .map(|(p, k)| json!({ "path": p, "reason": k.code(), "message": k.message() }))
        .collect()
}

fn folder_scope(
    dir: &Path,
    g: &Global,
    fallback: jarak_core::file::TextEncoding,
) -> jarak_core::infiles::Scope {
    jarak_core::infiles::Scope {
        dir: dir.to_path_buf(),
        filters: jarak_core::infiles::Filters::parse(g.filters.as_deref().unwrap_or("")),
        recursive: !g.no_subfolders,
        hidden: g.hidden,
        fallback,
    }
}

/// `jarak replace <file|folder> <text> <with>`: Replace in Files. `--dry-run` lists every change
/// and writes nothing; without it `--yes` is needed (the command changes files; it never asks).
fn replace(rest: &[&str], g: &Global) -> Result<Done, Fail> {
    use jarak_core::infiles;
    let [arg, text, with] = rest else {
        return Err(Fail::usage(
            "`jarak replace` takes a file or folder, the text to find and its replacement",
        ));
    };
    let path = absolute(Path::new(arg));
    let fallback = fallback_of(g)?;
    let m = compile(text, g)?;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let (plans, skipped) = if path.is_dir() {
        let (p, found) = infiles::plan(&folder_scope(&path, g, fallback), &m, with, &cancel);
        (p, found.skipped)
    } else if path.is_file() {
        match infiles::plan_file(&path, &m, with, fallback) {
            Ok(p) => (p.into_iter().collect(), Vec::new()),
            Err(s) => (Vec::new(), vec![(path.clone(), s)]),
        }
    } else {
        return Err(Fail::new(
            "file_not_found",
            3,
            format!("{} does not exist", path.display()),
        ));
    };
    let planned: usize = plans.iter().map(|p| p.changes.len()).sum();
    if !g.dry_run && !g.yes && planned > 0 {
        return Err(Fail {
            hint: Some("preview with --dry-run, then run again with --yes".into()),
            ..Fail::new(
                "confirmation_required",
                7,
                format!(
                    "`jarak replace` would change {planned} match{} in {} file{}; it needs --yes",
                    if planned == 1 { "" } else { "es" },
                    plans.len(),
                    if plans.len() == 1 { "" } else { "s" }
                ),
            )
        });
    }
    let mut out = infiles::Outcome::default();
    if !g.dry_run && planned > 0 {
        let root = journal_root()?;
        out = infiles::apply(&plans, &|_, _| true, fallback, &root, text, with)
            .map_err(|e| Fail::new("io", 1, e.to_string()))?;
    }
    let mut human = String::new();
    let files: Vec<Value> = plans
        .iter()
        .map(|p| {
            let applied = out.applied.iter().find(|a| a.path == p.path);
            let skip = out.skipped.iter().find(|(x, _)| *x == p.path);
            human.push_str(&format!(
                "{}: {} change{}{}\n",
                p.path.display(),
                p.changes.len(),
                if p.changes.len() == 1 { "" } else { "s" },
                match (g.dry_run, applied, skip) {
                    (true, _, _) => String::new(),
                    (false, Some(_), _) => " (replaced)".into(),
                    (false, None, Some((_, s))) => format!(" (skipped: {})", s.message()),
                    (false, None, None) => String::new(),
                }
            ));
            for c in p.changes.iter().take(200) {
                human.push_str(&format!(
                    "  {}:{}: {}\n  {}  → {}\n",
                    c.hit.line,
                    c.hit.column,
                    c.hit.line_text,
                    " ".repeat(c.hit.line.to_string().len() + c.hit.column.to_string().len()),
                    c.new_line
                ));
            }
            json!({
                "path": p.path,
                "encoding": p.format.encoding.label(p.format.bom),
                "replacements": p.changes.len(),
                "replaced": applied.is_some(),
                "skipped": skip.map(|(_, s)| json!({ "reason": s.code(), "message": s.message() })),
                "changes": p.changes.iter().take(1000).map(|c| json!({
                    "line": c.hit.line,
                    "column": c.hit.column,
                    "old_line": c.hit.line_text.chars().take(300).collect::<String>(),
                    "new_line": c.new_line.chars().take(300).collect::<String>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut all_skipped = skipped_json(&skipped);
    all_skipped.extend(skipped_json(&out.skipped));
    let replaced: usize = out.applied.iter().map(|a| a.replaced).sum();
    let data = json!({
        "path": path,
        "query": text,
        "replace": with,
        "mode": mode_name(g.search.mode),
        "dry_run": g.dry_run,
        "planned": planned,
        "replaced": replaced,
        "files": files,
        "skipped": all_skipped,
        "batch": out.batch.as_ref().map(|b| json!({ "id": b.id, "journal": out.journal })),
    });
    human.push_str(&if g.dry_run {
        format!(
            "dry run: {planned} change(s) in {} file(s); nothing was written",
            plans.len()
        )
    } else if let Some(b) = &out.batch {
        format!(
            "replaced {replaced} in {} file(s); undo with `jarak journal undo {}`",
            out.applied.len(),
            b.id
        )
    } else {
        "nothing replaced".into()
    });
    Ok((data, human))
}

fn journal_root() -> Result<PathBuf, Fail> {
    Paths::from_env().map(|p| p.journal_dir()).ok_or_else(|| {
        Fail::new(
            "io",
            1,
            "%LOCALAPPDATA% is not set: no place for the journal",
        )
    })
}

/// `jarak journal list` / `jarak journal undo <batch>`.
fn journal(command: &str, rest: &[&str], g: &Global) -> Result<Done, Fail> {
    use jarak_core::infiles;
    let root = journal_root()?;
    if command == "journal.list" {
        if !rest.is_empty() {
            return Err(Fail::usage("`jarak journal list` takes no arguments"));
        }
        let list = infiles::batches(&root);
        let mut human = format!("{} batch(es) in {}", list.len(), root.display());
        let v: Vec<Value> = list
            .iter()
            .map(|b| {
                human.push_str(&format!(
                    "\n{}  {} file(s)  \"{}\" → \"{}\"{}",
                    b.id,
                    b.files.len(),
                    b.find,
                    b.replace,
                    if b.undone { "  (undone)" } else { "" }
                ));
                json!({
                    "id": b.id,
                    "created": b.created,
                    "find": b.find,
                    "replace": b.replace,
                    "files": b.files.iter().map(|f| json!({ "path": f.path, "replaced": f.replaced })).collect::<Vec<_>>(),
                    "undone": b.undone,
                })
            })
            .collect();
        return Ok((json!({ "journal": root, "batches": v }), human));
    }
    let [id] = rest else {
        return Err(Fail::usage(
            "`jarak journal undo` takes a batch id (see `jarak journal list`)",
        ));
    };
    if !root.join(id).join("batch.json").is_file() || id.contains(['/', '\\', '.']) {
        return Err(Fail::new(
            "batch_not_found",
            3,
            format!("no batch `{id}` in {}", root.display()),
        ));
    }
    if !g.dry_run && !g.yes {
        return Err(Fail {
            hint: Some("check with --dry-run, then run again with --yes".into()),
            ..Fail::new(
                "confirmation_required",
                7,
                "`jarak journal undo` rewrites files; it needs --yes",
            )
        });
    }
    let out = infiles::undo(&root, id, g.dry_run).map_err(|e| Fail::new("io", 1, e.to_string()))?;
    let human = format!(
        "{}{} file(s) {}, {} left (changed since the batch)",
        if g.dry_run { "dry run: " } else { "" },
        out.applied.len(),
        if g.dry_run {
            "would be restored"
        } else {
            "restored"
        },
        out.skipped.len()
    );
    Ok((
        json!({
            "id": id,
            "dry_run": g.dry_run,
            "restored": out.applied.iter().map(|a| json!({ "path": a.path })).collect::<Vec<_>>(),
            "skipped": skipped_json(&out.skipped),
        }),
        human,
    ))
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
                "find",
                "replace",
                "export",
                "print",
                "printers",
                "journal.list",
                "journal.undo",
                "assets.list",
                "assets.rename",
                "settings.list",
                "settings.get",
                "settings.set",
                "settings.reset",
                "hotkeys.list",
                "hotkeys.show",
                "hotkeys.set",
                "hotkeys.disable",
                "hotkeys.reset",
                "hotkeys.check"
            ]
        );
        // Only `open` may open a window; the commands that change something have a dry run
        // (CLI_CONTRACT §4).
        for c in doc["commands"].as_array().unwrap() {
            let open = c["name"] == "open";
            assert_eq!(c["opens_window"], open, "{c}");
            assert_eq!(c["dry_run"], c["effect"] != "read", "{c}");
            // Step 95: what changes the user's files needs --yes (it never asks).
            assert_eq!(c["requires_yes"], c["effect"] == "destructive", "{c}");
        }
    }

    /// Step 105: `jarak settings` lists, reads, checks before writing, dry-runs and resets.
    #[test]
    fn settings_are_listed_checked_written_and_reset() {
        let d = std::env::temp_dir().join(format!("jarak-cli-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let f = d.join("settings.toml");
        let mut g = Global::default();
        let (data, _) = settings_in("settings.list", &["vim"], &g, &f).unwrap();
        assert_eq!(data["settings"][0]["key"], "editing_mode");
        assert_eq!(data["settings"][0]["is_default"], true);
        assert_eq!(
            settings_in("settings.list", &["nope"], &g, &f)
                .unwrap_err()
                .code,
            "usage"
        );
        let (data, _) = settings_in("settings.set", &["theme", "Dark"], &g, &f).unwrap();
        assert_eq!(data["value"], "dark");
        assert_eq!(data["changed"], true);
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "theme = \"dark\"
"
        );
        let e = settings_in("settings.set", &["tab_width", "0"], &g, &f).unwrap_err();
        assert_eq!((e.code, e.exit), ("setting_invalid", 2));
        let e = settings_in("settings.get", &["colour"], &g, &f).unwrap_err();
        assert_eq!((e.code, e.exit), ("setting_not_found", 3));
        g.dry_run = true;
        let (data, text) = settings_in("settings.set", &["tab_width", "8"], &g, &f).unwrap();
        assert_eq!(data["dry_run"], true);
        assert!(text.contains("would be"));
        let (data, _) = settings_in("settings.reset", &["theme"], &g, &f).unwrap();
        assert_eq!(data["changed"], true);
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "theme = \"dark\"
",
            "a dry run writes nothing"
        );
        g.dry_run = false;
        settings_in("settings.reset", &["theme"], &g, &f).unwrap();
        let (data, _) = settings_in("settings.get", &["theme"], &g, &f).unwrap();
        assert_eq!(data["value"], "system");
        let (data, _) = settings_in(
            "settings.set",
            &["search_engine", "https://x.org/?q=$(QUERY)", "&a=1"],
            &g,
            &f,
        )
        .unwrap();
        assert_eq!(
            data["value"], "https://x.org/?q=$(QUERY) &a=1",
            "the value's words are joined"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Step 105: `jarak hotkeys` lists, shows, sets (refusing a clash with the user's own binding
    /// unless --replace), disables, resets and checks, on the shared contract.
    #[test]
    fn hotkeys_are_listed_changed_and_checked() {
        let d = std::env::temp_dir().join(format!("jarak-cli-hotkeys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let f = d.join("hotkeys.json");
        let mut g = Global::default();
        let (data, _) = hotkeys_in("hotkeys.list", &[], &g, &f).unwrap();
        assert_eq!(data["tool"], "jarak");
        assert!(data["actions"].as_array().unwrap().len() > 100);
        let (data, _) = hotkeys_in("hotkeys.show", &["run.dialog"], &g, &f).unwrap();
        assert_eq!(data["action"]["id"], "run.dialog");
        let e = hotkeys_in("hotkeys.show", &["nope.nope"], &g, &f).unwrap_err();
        assert_eq!((e.code, e.exit), ("action_not_found", 3));
        let (data, _) = hotkeys_in("hotkeys.set", &["run.dialog", "Ctrl+Alt+R"], &g, &f).unwrap();
        assert_eq!(data["written"], true);
        let e = hotkeys_in("hotkeys.set", &["view.palette", "Ctrl+Alt+R"], &g, &f).unwrap_err();
        assert_eq!((e.code, e.exit), ("hotkey_conflict", 7));
        let e = hotkeys_in("hotkeys.set", &["run.dialog", "Ctrl+Nope"], &g, &f).unwrap_err();
        assert_eq!((e.code, e.exit), ("hotkey_invalid", 2));
        g.replace = true;
        hotkeys_in("hotkeys.set", &["view.palette", "Ctrl+Alt+R"], &g, &f).unwrap();
        g.replace = false;
        g.dry_run = true;
        let (data, _) = hotkeys_in("hotkeys.disable", &["view.palette"], &g, &f).unwrap();
        assert_eq!(data["written"], false);
        g.dry_run = false;
        hotkeys_in("hotkeys.reset", &["view.palette"], &g, &f).unwrap();
        let (data, _) = hotkeys_in("hotkeys.check", &[], &g, &f).unwrap();
        assert!(data.get("conflicts").is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Step 106 (`JARAK-FR-141`): HTML, RTF and a published folder; an existing output only
    /// with --overwrite, the document never; options that do not fit are refused.
    #[test]
    fn export_writes_locally_and_never_replaces_without_overwrite() {
        let d = std::env::temp_dir().join(format!("jarak-cli-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let md = d.join("n.md");
        std::fs::write(
            &md,
            "# N

![x](missing.png)
",
        )
        .unwrap();
        let rs = d.join("a.rs");
        std::fs::write(
            &rs,
            "fn main() {}
",
        )
        .unwrap();
        let s = |p: &Path| p.to_string_lossy().into_owned();
        let g = |out: &Path| Global {
            output: Some(s(out)),
            ..Global::default()
        };
        let (data, _) = export(&[&s(&md)], &g(&d.join("o/n.html"))).unwrap();
        assert_eq!(
            (data["format"].as_str(), data["rendered"].as_bool()),
            (Some("html"), Some(true))
        );
        assert_eq!(data["images"][0]["use"], "missing");
        let e = export(&[&s(&md)], &g(&d.join("o/n.html"))).unwrap_err();
        assert_eq!((e.code, e.exit), ("export_exists", 7));
        let mut o = g(&d.join("o/n.html"));
        o.overwrite = true;
        export(&[&s(&md)], &o).unwrap();
        let (data, _) = export(&[&s(&rs)], &g(&d.join("a.rtf"))).unwrap();
        assert_eq!(
            (data["format"].as_str(), data["language"].as_str()),
            (Some("rtf"), Some("Rust"))
        );
        let mut o = g(&rs);
        o.to = Some("html".into());
        o.overwrite = true;
        let e = export(&[&s(&rs)], &o).unwrap_err();
        assert_eq!(e.code, "export_into_source", "never the document itself");
        let mut o = g(&d.join("site"));
        o.to = Some("folder".into());
        o.dry_run = true;
        let (data, _) = export(&[&s(&md)], &o).unwrap();
        assert_eq!(data["written"], false);
        assert!(!d.join("site").exists());
        for (file, out, set) in [
            (&rs, "x.html", "linked"),
            (&rs, "x.html", "source"),
            (&md, "x.html", "line_numbers"),
            (&rs, "site", "folder"),
            (&md, "x.txt", ""),
        ] {
            let mut o = g(&d.join(out));
            match set {
                "linked" => o.linked = true,
                "source" => o.source = true,
                "line_numbers" => o.line_numbers = true,
                "folder" => o.to = Some("folder".into()),
                _ => {}
            }
            assert_eq!(
                export(&[&s(file)], &o).unwrap_err().code,
                "usage",
                "{set} {out}"
            );
        }
        assert_eq!(
            std::fs::read_to_string(&rs).unwrap(),
            "fn main() {}
"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Step 106 (`JARAK-FR-140`): `jarak print` checks its options, never asks (a printer that
    /// writes a file needs --output), never replaces a file without --overwrite; the dry run
    /// lays the pages out on the real printer and prints nothing.
    #[test]
    fn print_checks_its_options_and_dry_runs_on_the_real_printer() {
        let d = std::env::temp_dir().join(format!("jarak-cli-print-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let rs = d.join("a.rs");
        std::fs::write(
            &rs,
            "fn main() {}
",
        )
        .unwrap();
        let f = rs.to_string_lossy().into_owned();
        let g = Global::default();
        assert_eq!(print(&[], &g).unwrap_err().code, "usage");
        let bad = Global {
            pages: Some("3-1".into()),
            ..Global::default()
        };
        assert_eq!(print(&[&f], &bad).unwrap_err().code, "usage");
        let to = Global {
            to: Some("pdf".into()),
            ..Global::default()
        };
        assert_eq!(print(&[&f], &to).unwrap_err().code, "usage");
        let nope = Global {
            printer: Some("No such printer here".into()),
            ..Global::default()
        };
        assert_eq!(print(&[&f], &nope).unwrap_err().code, "printer_not_found");
        let (data, _) = printers().unwrap();
        let has_pdf = data["printers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == jarak_core::print::PDF_PRINTER);
        if has_pdf {
            let pdf = |out: Option<&Path>, dry_run: bool| Global {
                printer: Some(jarak_core::print::PDF_PRINTER.into()),
                output: out.map(|o| o.to_string_lossy().into_owned()),
                dry_run,
                ..Global::default()
            };
            let e = print(&[&f], &pdf(None, true)).unwrap_err();
            assert_eq!(e.code, "usage", "a file-writing printer needs --output");
            let out = d.join("a.pdf");
            let (data, _) = print(&[&f], &pdf(Some(&out), true)).unwrap();
            assert_eq!(
                (data["pages"].as_u64(), data["printed"].as_bool()),
                (Some(1), Some(false))
            );
            assert!(!out.exists(), "a dry run prints nothing");
            std::fs::write(&out, "kept").unwrap();
            let e = print(&[&f], &pdf(Some(&out), true)).unwrap_err();
            assert_eq!((e.code, e.exit), ("export_exists", 7));
            let e = print(&[&f], &pdf(Some(&rs), true)).unwrap_err();
            assert_eq!(e.code, "export_into_source");
        }
        let _ = std::fs::remove_dir_all(&d);
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
