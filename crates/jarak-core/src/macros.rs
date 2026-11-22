//! Macros and user commands (`JARAK-FR-110`, `JARAK-FR-111`, Step 102): what a macro records
//! (registry commands, typed text, clipboard actions, find and replace with their options), the
//! files that keep named macros and run commands, the variables of a run command, and how its
//! line is split into a program and its arguments. Nothing here runs a program or touches a
//! window; the window plays macros back and runs commands only when the user confirms.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One recorded step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// A command of the registry, by its id (`line.duplicate`, `cursor.down` …).
    Cmd(String),
    /// Typed text (a run of typing is one step).
    Type(String),
    /// `paste` (the clipboard at playback), `copy`, `cut`, `indent`, `outdent`, `save`.
    Action(String),
    /// A find or replace with the options it was made with.
    Find(FindStep),
}

/// A recorded find or replace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindStep {
    /// `next`, `replace` (one) or `replace_all`.
    pub op: String,
    pub find: String,
    #[serde(default)]
    pub replace: String,
    /// `normal`, `extended` or `regex`.
    #[serde(default = "normal")]
    pub mode: String,
    #[serde(default)]
    pub match_case: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default = "yes")]
    pub wrap: bool,
    #[serde(default)]
    pub backward: bool,
    #[serde(default)]
    pub in_selection: bool,
    #[serde(default)]
    pub dot_all: bool,
}

fn normal() -> String {
    "normal".into()
}

fn yes() -> bool {
    true
}

/// A named macro with its shortcut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,
    pub steps: Vec<Step>,
}

/// A user command (Notepad++'s Run…): a command line with variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub name: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,
}

/// `macros.json`: the saved macros and run commands (`format` names the layout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCommands {
    pub format: String,
    #[serde(default)]
    pub macros: Vec<Macro>,
    #[serde(default)]
    pub commands: Vec<RunCommand>,
}

/// The layout of `macros.json`.
pub const FORMAT: &str = "jarak.macros/1";

impl Default for UserCommands {
    fn default() -> Self {
        UserCommands {
            format: FORMAT.into(),
            macros: Vec::new(),
            commands: Vec::new(),
        }
    }
}

impl UserCommands {
    /// Reads `path` (none yet: empty); a broken file is an error, never overwritten silently.
    pub fn load(path: &Path) -> Result<UserCommands, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                let u: UserCommands = serde_json::from_str(&s).map_err(|e| e.to_string())?;
                if u.format != FORMAT {
                    return Err(format!(
                        "unknown format `{}` (expected `{FORMAT}`)",
                        u.format
                    ));
                }
                Ok(u)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UserCommands::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Writes `path` atomically (a temporary file renamed over it).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json + "\n")?;
        std::fs::rename(&tmp, path)
    }
}

/// The built-in macro (Notepad++'s *Trim Trailing Space and Save*).
pub fn builtin_macros() -> Vec<Macro> {
    vec![Macro {
        name: "Trim Trailing Space and Save".into(),
        keys: None,
        steps: vec![
            Step::Cmd("blank.trim-trailing".into()),
            Step::Action("save".into()),
        ],
    }]
}

/// The built-in run commands (Notepad++'s *Launch in …*, *Wikipedia Search*, *Get PHP help*):
/// they open the default browser, and only when chosen and confirmed.
pub fn builtin_commands() -> Vec<RunCommand> {
    vec![
        RunCommand {
            name: "Open the file in the default browser".into(),
            command: "$(FULL_CURRENT_PATH)".into(),
            keys: None,
        },
        RunCommand {
            name: "Wikipedia search for the current word".into(),
            command: "https://en.wikipedia.org/wiki/Special:Search?search=$(CURRENT_WORD)".into(),
            keys: None,
        },
        RunCommand {
            name: "PHP help for the current word".into(),
            command: "https://www.php.net/$(CURRENT_WORD)".into(),
            keys: None,
        },
    ]
}

/// The values a run command's `$(…)` variables take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vars {
    pub full_current_path: String,
    pub current_directory: String,
    pub file_name: String,
    pub name_part: String,
    pub ext_part: String,
    pub current_word: String,
    pub current_line: usize,
    pub current_column: usize,
    pub current_line_text: String,
    pub jarak_directory: String,
    pub jarak_full_file_path: String,
}

/// The names `expand` knows (shown in the Run dialog).
pub const VARIABLES: &[&str] = &[
    "FULL_CURRENT_PATH",
    "CURRENT_DIRECTORY",
    "FILE_NAME",
    "NAME_PART",
    "EXT_PART",
    "CURRENT_WORD",
    "CURRENT_LINE",
    "CURRENT_COLUMN",
    "CURRENT_LINESTR",
    "JARAK_DIRECTORY",
    "JARAK_FULL_FILE_PATH",
    "SYS.<name>",
];

/// `line` with its `$(…)` variables replaced (Notepad++'s names; `NPP_DIRECTORY` and
/// `NPP_FULL_FILE_PATH` are Jarak's folder and program; `$(SYS.NAME)` an environment variable,
/// empty when unset). An unknown name stays as written. `env` gives environment variables.
pub fn expand(line: &str, v: &Vars, env: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(p) = rest.find("$(") {
        out.push_str(&rest[..p]);
        let after = &rest[p + 2..];
        let Some(e) = after.find(')') else {
            out.push_str(&rest[p..]);
            return out;
        };
        let name = &after[..e];
        let value = match name {
            "FULL_CURRENT_PATH" => Some(v.full_current_path.clone()),
            "CURRENT_DIRECTORY" => Some(v.current_directory.clone()),
            "FILE_NAME" => Some(v.file_name.clone()),
            "NAME_PART" => Some(v.name_part.clone()),
            "EXT_PART" => Some(v.ext_part.clone()),
            "CURRENT_WORD" => Some(v.current_word.clone()),
            "CURRENT_LINE" => Some(v.current_line.to_string()),
            "CURRENT_COLUMN" => Some(v.current_column.to_string()),
            "CURRENT_LINESTR" => Some(v.current_line_text.clone()),
            "JARAK_DIRECTORY" | "NPP_DIRECTORY" => Some(v.jarak_directory.clone()),
            "JARAK_FULL_FILE_PATH" | "NPP_FULL_FILE_PATH" => Some(v.jarak_full_file_path.clone()),
            _ => name
                .strip_prefix("SYS.")
                .map(|n| env(n).unwrap_or_default()),
        };
        match value {
            Some(x) => out.push_str(&x),
            None => out.push_str(&rest[p..p + 2 + e + 1]),
        }
        rest = &after[e + 1..];
    }
    out.push_str(rest);
    out
}

/// What a run command's expanded line is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A web address (`http`, `https`): opened in the default browser.
    Web(String),
    /// A program and the rest of the line as its arguments (passed as written).
    Program { program: String, args: String },
    /// A file that is not a program: opened with its default program (a browser for HTML).
    File(String),
}

/// Splits an expanded line: a quoted or space-ended first word is the program; a web address or a
/// file that is not a program is opened instead of run. Empty: `None`.
pub fn target(line: &str) -> Option<Target> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if (lower.starts_with("http://") || lower.starts_with("https://"))
        && !t.contains(char::is_whitespace)
    {
        return Some(Target::Web(t.to_string()));
    }
    let (first, rest) = if let Some(q) = t.strip_prefix('"') {
        match q.find('"') {
            Some(e) => (q[..e].to_string(), q[e + 1..].trim_start().to_string()),
            None => (q.to_string(), String::new()),
        }
    } else {
        match t.find(char::is_whitespace) {
            Some(e) => (t[..e].to_string(), t[e..].trim_start().to_string()),
            None => (t.to_string(), String::new()),
        }
    };
    let ext = Path::new(&first)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let program = matches!(ext.as_str(), "" | "exe" | "com" | "bat" | "cmd");
    if !program && rest.is_empty() {
        return Some(Target::File(first));
    }
    Some(Target::Program {
        program: first,
        args: rest,
    })
}

#[cfg(test)]
mod tests;
