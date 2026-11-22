//! E2E — #98 T4 (follow-up of Step 36): Subak's portable mode
//! (docs/TABBY_FEATURE_MATRIX.md `SHELL-24`). `SUBAK-FR-013`.
//!
//! A copy of the released `subak.exe` in a folder with a `data` folder next to it keeps its
//! settings in `data\config` (`config set` writes there, `config show` names that file) and writes
//! nothing to `%APPDATA%` (a private sandbox); without the `data` folder the same copy is an
//! ordinary one again. No Host is started: the pipe stays per user (SUBAK-SEC-010) and the
//! maintainer's own Host runs on it.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(exe: &Path, appdata: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(exe)
        .args(args)
        .arg("--json")
        .env("APPDATA", appdata)
        .env("LOCALAPPDATA", appdata.join("Local"))
        .output()
        .unwrap();
    let doc = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), doc)
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn issue98_a_data_folder_makes_a_portable_copy() {
    let sb = TempDir::new("issue98-portable");
    let appdata = sb.path().join("Roaming");
    let app = sb.path().join("Subak portable");
    std::fs::create_dir_all(app.join("data")).unwrap();
    std::fs::create_dir_all(&appdata).unwrap();
    let exe = app.join("subak.exe");
    std::fs::copy(bin("subak"), &exe).unwrap();
    let (code, doc) = run(
        &exe,
        &appdata,
        &["config", "set", "appearance.padding", "4"],
    );
    assert_eq!(code, 0, "{doc}");
    let file = app.join("data").join("config").join("config.toml");
    assert_eq!(
        doc["data"]["path"].as_str().map(PathBuf::from),
        Some(file.clone())
    );
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("padding = 4")
    );
    assert!(
        !appdata.join("mouv").exists(),
        "a portable copy writes nothing to %APPDATA%"
    );
    std::fs::remove_dir_all(app.join("data")).unwrap();
    let (code, doc) = run(&exe, &appdata, &["config", "show"]);
    assert_eq!(code, 0, "{doc}");
    assert_eq!(
        doc["data"]["path"].as_str().map(PathBuf::from),
        Some(appdata.join("mouv").join("subak").join("config.toml"))
    );
}
