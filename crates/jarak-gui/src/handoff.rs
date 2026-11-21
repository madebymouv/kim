//! Files a second `jarak-gui` start hands to the running window (`JARAK-FR-002`).
//!
//! The second process writes one small JSON file into `%LOCALAPPDATA%\mouv\jarak\handoff\`
//! (written to a temporary name, then renamed, so the owner never reads half a file), signals the
//! owner's named event and exits; the owner reads every file in name order, deletes it and opens
//! the targets. Files older than a start of the owner are stale (a sender that crashed between
//! the write and the signal) and are removed without opening anything.

use serde_json::{Value, json};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes one hand-off with `targets` (`path[:line[:column]]` arguments) into `dir`.
pub fn write(dir: &Path, targets: &[String]) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let name = format!("{nanos:024}-{}", std::process::id());
    let tmp = dir.join(format!("{name}.tmp"));
    let done = dir.join(format!("{name}.json"));
    std::fs::write(&tmp, json!({ "targets": targets }).to_string())?;
    std::fs::rename(&tmp, &done)?;
    Ok(done)
}

/// Reads and removes every complete hand-off in `dir`, oldest first; unreadable files are
/// removed too (they would never become readable).
pub fn drain(dir: &Path) -> Vec<Vec<String>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let parsed = std::fs::read(&f)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        let _ = std::fs::remove_file(&f);
        if let Some(Value::Array(a)) = parsed.as_ref().map(|v| &v["targets"]) {
            out.push(
                a.iter()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect(),
            );
        }
    }
    out
}

/// Removes every hand-off and temporary file in `dir` (at the owner's start: none can be for it).
pub fn clear(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json" || x == "tmp") {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-handoff-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn hand_offs_arrive_in_order_once() {
        let d = temp("order");
        write(&d, &["a.md:3".into()]).unwrap();
        write(&d, &["b.txt".into(), "c.txt:1:2".into()]).unwrap();
        std::fs::write(d.join("broken.json"), "{not json").unwrap();
        let got = drain(&d);
        assert_eq!(
            got,
            vec![
                vec!["a.md:3".to_string()],
                vec!["b.txt".to_string(), "c.txt:1:2".to_string()]
            ]
        );
        assert!(drain(&d).is_empty(), "each hand-off is read once");
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_files_are_cleared_without_opening() {
        let d = temp("stale");
        write(&d, &["old.md".into()]).unwrap();
        std::fs::write(d.join("x.tmp"), "half").unwrap();
        clear(&d);
        assert!(drain(&d).is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }
}
