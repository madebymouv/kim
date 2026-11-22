//! Jarak's keyboard shortcuts on the shared `kim-hotkeys` library (Step 105, SET-23,
//! docs/HOTKEY_CONTRACT.md; used by `jarak hotkeys …` and the Settings window's Keyboard section).
//!
//! - **Manifest.** Every command that has keys is an `app` action (backend `focus`: Jarak reads
//!   keys only while its window has the focus; it registers nothing system-wide and installs no
//!   hook). The manifest is built from the window's commands (`jarak-gui`'s palette) and shipped as
//!   [`MANIFEST_JSON`] (`hotkeys.manifest.json`, the contract example
//!   `docs/contracts/examples/kim-hotkey-manifest-v1/jarak.json`); a test in `jarak-gui` keeps it
//!   equal to the commands.
//! - **The user layer** is `%APPDATA%\mouv\jarak\hotkeys.json` (only what the user changed). A
//!   broken or newer file means safe mode: the defaults apply and the file is never rewritten
//!   implicitly (`KIM-SEC-023`).
//! - **Changes** ([`change`]) go through `kim_hotkeys::edit` (the contract's checks: syntax,
//!   reserved keys, conflicts with Jarak's other commands — refused unless `replace`), and are
//!   written atomically; a refused change writes nothing.

use kim_hotkeys::check::{Analysis, ToolState, analyze};
use kim_hotkeys::edit::{self, Applied, Edit, Options, Refusal, Switch};
use kim_hotkeys::keys::Sequence;
use kim_hotkeys::layer::UserLayer;
use kim_hotkeys::manifest::{BindingSpec, Manifest};
use kim_hotkeys::store::{self, ConfigState, Loaded};
use std::path::Path;

pub const TOOL: &str = "jarak";

/// This build's manifest (`version` = the crate's version, kept by `cargo xtask bump`).
pub const MANIFEST_JSON: &str = include_str!("hotkeys.manifest.json");

/// The compiled-in manifest.
pub fn manifest() -> Manifest {
    Manifest::parse(MANIFEST_JSON.as_bytes()).expect("the compiled-in hotkey manifest is valid")
}

/// The user layer at `path` and the tool state the checks and the window use.
pub fn load(path: &Path) -> (ToolState, Loaded) {
    let m = manifest();
    let loaded = store::load(path, &m);
    let mut s = ToolState::new(m, loaded.layer.clone());
    s.safe_mode = loaded.state.safe_mode();
    (s, loaded)
}

/// Conflicts and the bindings that would fire.
pub fn analysis(tool: &ToolState) -> Analysis {
    analyze(std::slice::from_ref(tool), &[])
}

/// A change to the user layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The action's keys become these (each `Ctrl+Shift+R`, `F5`, …).
    Set { action: String, keys: Vec<String> },
    /// The action gets no keys.
    Disable(String),
    /// The action's default keys again.
    Reset(String),
    /// Every shortcut back to its default (the previous file kept as `hotkeys.json.bak`).
    ResetAll,
}

/// Why a change was refused (nothing was written).
#[derive(Debug)]
pub enum Refused {
    /// The keys do not parse (`hotkey_invalid`).
    Keys(String),
    /// The contract refused it (code, message, hint, conflicts).
    Edit(Refusal),
    /// The file is invalid or from a newer Jarak: it is never rewritten implicitly.
    ReadOnly(String),
    Io(std::io::Error),
}

impl Refused {
    /// The CLI code and exit code.
    pub fn code(&self) -> (&'static str, u8) {
        match self {
            Refused::Keys(_) => ("hotkey_invalid", 2),
            Refused::Edit(r) => (r.error.code(), r.error.exit_code()),
            Refused::ReadOnly(_) => ("hotkeys_invalid", 4),
            Refused::Io(_) => ("io", 1),
        }
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Keys(e) => write!(f, "{e}"),
            Refused::Edit(r) => write!(f, "{}", r.message),
            Refused::ReadOnly(why) => write!(f, "{why}"),
            Refused::Io(e) => write!(f, "{e}"),
        }
    }
}

/// What a change did.
#[derive(Debug)]
pub struct Outcome {
    pub applied: Applied,
    /// The file was written (not a dry run, and something changed).
    pub written: bool,
}

/// Applies `c` to the user layer at `path` (written unless `dry_run`; `replace` takes the keys
/// away from another Jarak command that has them).
pub fn change(path: &Path, c: &Change, replace: bool, dry_run: bool) -> Result<Outcome, Refused> {
    let (tool, loaded) = load(path);
    // A newer file is never replaced; an invalid one only by Reset all (kept as the backup).
    let refuses = match c {
        Change::ResetAll => loaded.state == ConfigState::NewerVersion,
        _ => loaded.state.safe_mode(),
    };
    if refuses {
        return Err(Refused::ReadOnly(format!(
            "{} is {} and is not rewritten implicitly: {}",
            path.display(),
            loaded.state.as_str(),
            loaded.problems.join("; ")
        )));
    }
    let tools = vec![tool];
    let opts = Options {
        replace,
        ..Options::default()
    };
    let applied = match c {
        Change::Set { action, keys } => {
            let mut bindings = Vec::new();
            for k in keys {
                let seq = Sequence::parse_input(k).map_err(|e| Refused::Keys(e.to_string()))?;
                bindings.push(BindingSpec::Text(seq.to_string()));
            }
            edit::apply(
                &tools,
                &[],
                0,
                &Edit::Set {
                    action: action.clone(),
                    bindings,
                },
                &opts,
            )
            .map_err(Refused::Edit)?
        }
        Change::Disable(a) => {
            edit::apply(&tools, &[], 0, &Edit::Disable { action: a.clone() }, &opts)
                .map_err(Refused::Edit)?
        }
        Change::Reset(a) => edit::apply(&tools, &[], 0, &Edit::Reset { action: a.clone() }, &opts)
            .map_err(Refused::Edit)?,
        Change::ResetAll => edit::apply_switch(&tools, &[], 0, &Switch::ResetAll),
    };
    let written = !dry_run && applied.changed;
    if written {
        let layer = applied
            .layer
            .clone()
            .unwrap_or_else(|| UserLayer::new(TOOL));
        let backup = matches!(c, Change::ResetAll) || loaded.migrated_from.is_some();
        store::save(path, &layer, backup).map_err(Refused::Io)?;
    }
    Ok(Outcome { applied, written })
}

/// An action's keys now (the user's, or the defaults) and whether the user changed them; empty
/// when it has none (turned off).
pub fn keys_of(tool: &ToolState, action: &str) -> (Vec<String>, bool) {
    match tool.effective_override(action) {
        Some((o, _)) if o.is_disabled() => (Vec::new(), true),
        Some((o, _)) => (
            o.specs().iter().map(|s| s.keys().to_string()).collect(),
            true,
        ),
        None => (
            tool.manifest
                .action(action)
                .map(|a| a.defaults.iter().map(|s| s.keys().to_string()).collect())
                .unwrap_or_default(),
            false,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-hotkeys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The shipped manifest is valid, Jarak's, this version's, and its defaults do not conflict.
    #[test]
    fn the_manifest_is_valid_and_its_defaults_do_not_conflict() {
        let m = manifest();
        assert_eq!(m.tool, TOOL);
        assert_eq!(m.version, crate::VERSION);
        assert!(m.actions.len() > 100, "{}", m.actions.len());
        let (tool, _) = load(Path::new("Z:\\nowhere\\hotkeys.json"));
        let a = analysis(&tool);
        let errors: Vec<String> = a
            .conflicts
            .iter()
            .filter(|c| c.severity() == kim_hotkeys::check::Severity::Error)
            .map(|c| c.to_json().to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// Set, a conflict refused (and taken with `replace`), disable, reset, reset all; a refused
    /// change writes nothing; an invalid file is never rewritten by a change.
    #[test]
    fn changes_are_checked_and_written_atomically() {
        let d = temp("change");
        let f = d.join("hotkeys.json");
        let set = |a: &str, k: &str| Change::Set {
            action: a.into(),
            keys: vec![k.into()],
        };
        let o = change(&f, &set("run.dialog", "Ctrl+Alt+R"), false, false).unwrap();
        assert!(o.written);
        let (tool, _) = load(&f);
        assert_eq!(
            keys_of(&tool, "run.dialog"),
            (vec!["Ctrl+Alt+R".to_string()], true)
        );
        // Another command's default: the user's binding wins, and the change says so.
        let o = change(&f, &set("run.dialog", "Ctrl+S"), false, true).unwrap();
        assert!(
            o.applied
                .conflicts
                .iter()
                .any(|c| c.message.contains("Save"))
        );
        assert!(!o.written, "a dry run");
        // Keys another of the user's own bindings has: refused, nothing written; taken with replace.
        let before = std::fs::read(&f).unwrap();
        let e = change(&f, &set("view.palette", "Ctrl+Alt+R"), false, false).unwrap_err();
        assert_eq!(e.code().0, "hotkey_conflict");
        assert_eq!(std::fs::read(&f).unwrap(), before);
        change(&f, &set("view.palette", "Ctrl+Alt+R"), true, false).unwrap();
        let (tool, _) = load(&f);
        assert_eq!(keys_of(&tool, "view.palette").0, ["Ctrl+Alt+R"]);
        assert!(keys_of(&tool, "run.dialog").0.is_empty(), "it lost them");
        change(&f, &Change::Reset("view.palette".into()), false, false).unwrap();
        change(&f, &set("run.dialog", "Ctrl+Alt+R"), false, false).unwrap();
        let before = std::fs::read(&f).unwrap();
        assert!(matches!(
            change(&f, &set("run.dialog", "Ctrl+Nope"), false, false),
            Err(Refused::Keys(_))
        ));
        // A dry run writes nothing.
        let o = change(&f, &Change::Disable("run.dialog".into()), false, true).unwrap();
        assert!(!o.written);
        assert_eq!(std::fs::read(&f).unwrap(), before);
        change(&f, &Change::Disable("run.dialog".into()), false, false).unwrap();
        let (tool, _) = load(&f);
        assert_eq!(keys_of(&tool, "run.dialog"), (Vec::new(), true));
        change(&f, &Change::Reset("run.dialog".into()), false, false).unwrap();
        let (tool, _) = load(&f);
        assert_eq!(
            keys_of(&tool, "run.dialog"),
            (vec!["F5".to_string()], false)
        );
        change(&f, &set("run.dialog", "Ctrl+Alt+R"), false, false).unwrap();
        change(&f, &Change::ResetAll, false, false).unwrap();
        let (tool, _) = load(&f);
        assert!(!keys_of(&tool, "run.dialog").1);
        // An invalid file: safe mode, never rewritten by a change.
        std::fs::write(&f, b"{ broken").unwrap();
        assert!(matches!(
            change(&f, &Change::Disable("run.dialog".into()), false, false),
            Err(Refused::ReadOnly(_))
        ));
        assert_eq!(std::fs::read(&f).unwrap(), b"{ broken");
        let _ = std::fs::remove_dir_all(&d);
    }
}
