//! E2E — Step 108 (#127): Jarak's extensions through the real `jarak extensions`
//! (docs/JARAK_EXTENSIONS.md, docs/JARAK_PLUGIN_AUDIT.md). Requirements: `JARAK-FR-160`,
//! `JARAK-FR-161`, `JARAK-SEC-007`, `KIM-SEC-001`.
//!
//! 1. `check` of the sample extension (a language, snippets, two commands) passes; a broken folder
//!    is refused with every problem (exit 4, `extension_invalid`).
//! 2. `install --dry-run` copies nothing; `install` copies it; again is refused (exit 7,
//!    `extension_exists`) unless `--replace`.
//! 3. `list` shows it not granted; `grant` grants exactly its manifest's permissions; a changed
//!    manifest (installed with `--replace`) is no longer granted; `revoke`; `remove` (also its
//!    grant). A folder in the extensions folder that does not check is listed with its problems.
//!
//! Machine safety: nothing is downloaded and no window opens; everything happens in a temporary
//! folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

fn jarak(dir: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(bin("jarak"))
        .args(args)
        .arg("--json")
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (out.status.code().unwrap_or(-1), v)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
    }
}

#[test]
#[ignore = "e2e: run by `cargo xtask e2e` (KIM_E2E=1)"]
fn step108_extensions_are_installed_granted_and_removed() {
    if std::env::var_os("KIM_E2E").is_none() {
        return;
    }
    let tmp = TempDir::new("step108");
    let dir = tmp.path();
    let sample = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/jarak-extensions/mouv.sample-tools");
    let src = dir.join("download").join("mouv.sample-tools");
    copy_dir(&sample, &src);
    let s = src.to_str().unwrap();

    // 1. check
    let (code, v) = jarak(dir, &["extensions", "check", s]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["languages"][0], "TODO list");
    assert_eq!(v["data"]["commands"].as_array().unwrap().len(), 2);
    let broken = dir.join("broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(
        broken.join("extension.json"),
        r#"{"schema":"jarak.extension","schema_version":1,"id":"x","name":"x","version":"1",
            "contributes":{"commands":[{"id":"c","title":"C","program":"evil.cmd","input":"document"}]},
            "permissions":["run-process"]}"#,
    )
    .unwrap();
    let (code, v) = jarak(dir, &["extensions", "check", broken.to_str().unwrap()]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (4, Some("extension_invalid")),
        "{v}"
    );
    assert!(v["error"]["details"].as_array().unwrap().len() >= 2, "{v}");

    // 2. install
    let installed = dir
        .join("Roaming")
        .join("mouv")
        .join("jarak")
        .join("extensions")
        .join("mouv.sample-tools");
    let (code, _) = jarak(dir, &["extensions", "install", s, "--dry-run"]);
    assert_eq!(code, 0);
    assert!(!installed.exists(), "a dry run copies nothing");
    let (code, v) = jarak(dir, &["extensions", "install", s]);
    assert_eq!(code, 0, "{v}");
    assert!(installed.join("extension.json").is_file());
    let (code, v) = jarak(dir, &["extensions", "install", s]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("extension_exists"))
    );

    // 3. grants
    let (code, v) = jarak(dir, &["extensions", "list"]);
    assert_eq!(
        (code, v["data"]["extensions"][0]["granted"].as_bool()),
        (0, Some(false))
    );
    let (code, v) = jarak(dir, &["extensions", "grant", "mouv.sample-tools"]);
    assert_eq!(code, 0, "{v}");
    let (_, v) = jarak(dir, &["extensions", "list"]);
    assert_eq!(v["data"]["extensions"][0]["granted"], true);
    // A changed manifest is not covered by the old grant.
    let mf = src.join("extension.json");
    let changed = std::fs::read_to_string(&mf)
        .unwrap()
        .replace("\"version\": \"1.0.0\"", "\"version\": \"1.0.1\"");
    std::fs::write(&mf, changed).unwrap();
    let (code, v) = jarak(dir, &["extensions", "install", s, "--replace"]);
    assert_eq!(code, 0, "{v}");
    let (_, v) = jarak(dir, &["extensions", "list"]);
    assert_eq!(v["data"]["extensions"][0]["granted"], false, "asked again");
    let (code, _) = jarak(dir, &["extensions", "revoke", "mouv.sample-tools"]);
    assert_eq!(code, 0);
    // A broken folder among the installed ones is listed, not used.
    copy_dir(&broken, &installed.parent().unwrap().join("x"));
    let (_, v) = jarak(dir, &["extensions", "list"]);
    assert_eq!(v["data"]["problems"].as_array().unwrap().len(), 1, "{v}");
    let (code, v) = jarak(dir, &["extensions", "remove", "mouv.sample-tools"]);
    assert_eq!(code, 0, "{v}");
    assert!(!installed.exists());
}
