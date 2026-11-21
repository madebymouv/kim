//! What to open: `file[:line[:column]]` from the command line (`JARAK-FR-003`).
//!
//! Windows paths contain a drive colon (`C:\notes\a.md`), and a file name may itself end in
//! `:12` on other systems, so the rule is: an argument that names an existing file is that file;
//! otherwise up to two trailing `:<number>` parts are the line and the column (1-based), and the
//! rest is the path. A path with nothing left (`:12`) is not a target.

use std::path::{Path, PathBuf};

/// One file to open, with an optional caret position (1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTarget {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl OpenTarget {
    /// Parses one argument; `exists` says whether a path names an existing file (the real
    /// file system in [`OpenTarget::parse`], a fake one in tests).
    pub fn parse_with(arg: &str, exists: &dyn Fn(&Path) -> bool) -> Option<OpenTarget> {
        if arg.is_empty() {
            return None;
        }
        let whole = OpenTarget {
            path: PathBuf::from(arg),
            line: None,
            column: None,
        };
        if exists(Path::new(arg)) {
            return Some(whole);
        }
        let (rest, last) = split_number(arg);
        let Some(last) = last else {
            return Some(whole);
        };
        let (rest2, before) = split_number(rest);
        let (path, line, column) = match before {
            Some(line) => (rest2, line, Some(last)),
            None => (rest, last, None),
        };
        // A drive letter alone (`C` from `C:12`, a drive-relative path) or nothing at all is
        // not a file with a position.
        if path.is_empty() || (path.len() == 1 && path.as_bytes()[0].is_ascii_alphabetic()) {
            return Some(whole);
        }
        Some(OpenTarget {
            path: PathBuf::from(path),
            line: (line > 0).then_some(line),
            column: column.filter(|c| *c > 0),
        })
    }

    /// [`OpenTarget::parse_with`] against the real file system.
    pub fn parse(arg: &str) -> Option<OpenTarget> {
        OpenTarget::parse_with(arg, &|p| p.is_file())
    }

    /// The argument form again (`path:line:column`), for handing the target to another process.
    pub fn to_arg(&self) -> String {
        let mut s = self.path.display().to_string();
        if let Some(l) = self.line {
            s.push_str(&format!(":{l}"));
            if let Some(c) = self.column {
                s.push_str(&format!(":{c}"));
            }
        }
        s
    }
}

/// `("a.txt", Some(12))` for `"a.txt:12"`, `(s, None)` when `s` does not end in `:<digits>`.
fn split_number(s: &str) -> (&str, Option<u32>) {
    match s.rsplit_once(':') {
        Some((rest, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            match n.parse() {
                Ok(v) => (rest, Some(v)),
                Err(_) => (s, None),
            }
        }
        _ => (s, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(arg: &str, existing: &[&str]) -> Option<OpenTarget> {
        let set: Vec<PathBuf> = existing.iter().map(PathBuf::from).collect();
        OpenTarget::parse_with(arg, &|p| set.iter().any(|e| e == p))
    }

    fn at(path: &str, line: Option<u32>, column: Option<u32>) -> Option<OpenTarget> {
        Some(OpenTarget {
            path: PathBuf::from(path),
            line,
            column,
        })
    }

    #[test]
    fn windows_paths_lines_and_columns() {
        assert_eq!(t(r"C:\n\a.md", &[]), at(r"C:\n\a.md", None, None));
        assert_eq!(t(r"C:\n\a.md:12", &[]), at(r"C:\n\a.md", Some(12), None));
        assert_eq!(
            t(r"C:\n\a.md:12:3", &[]),
            at(r"C:\n\a.md", Some(12), Some(3))
        );
        assert_eq!(t("notes.txt:7", &[]), at("notes.txt", Some(7), None));
        assert_eq!(
            t(r"한글\메모.md:2:5", &[]),
            at(r"한글\메모.md", Some(2), Some(5))
        );
    }

    #[test]
    fn an_existing_file_wins_and_odd_input_stays_a_path() {
        // A file really named `log:12` (possible on other file systems) is that file.
        assert_eq!(t("log:12", &["log:12"]), at("log:12", None, None));
        // Nothing but a number, or a bare drive, is a path, not a position.
        assert_eq!(t(":12", &[]), at(":12", None, None));
        assert_eq!(t("C:12", &[]), at("C:12", None, None));
        // Line 0 means no line; an overlong number is part of the name.
        assert_eq!(t("a.txt:0", &[]), at("a.txt", None, None));
        assert_eq!(
            t("a.txt:99999999999", &[]),
            at("a.txt:99999999999", None, None)
        );
        assert_eq!(t("", &[]), None);
    }

    #[test]
    fn the_argument_form_round_trips() {
        for arg in [r"C:\n\a.md", r"C:\n\a.md:12", r"C:\n\a.md:12:3"] {
            assert_eq!(t(arg, &[]).unwrap().to_arg(), arg);
        }
    }
}
