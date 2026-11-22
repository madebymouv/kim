//! E2E — Step 94 (#113): find, replace and mark — `jarak find` and the real window's find panel
//! (ADR 0102, docs/JARAK_ARCHITECTURE.md §16).
//! Requirements: `JARAK-FR-070`, `JARAK-FR-071`, `JARAK-FR-072`, `JARAK-SEC-009`.
//!
//! 1. `jarak find` through the real CLI: a whole-word search with line and column, a regular
//!    expression, an extended search for `\t`, `--count`, and a pattern that does not compile
//!    (`usage`, exit 2) — each one leaves the file's bytes as they were.
//! 2. The release `jarak-gui` with the UI audit's environment and a session in its sandbox
//!    (`KIM_UI_SESSION=1`), screen `mark`: the find panel's Mark page (Mark All with bookmarks and
//!    a token style) is drawn with no audit failure, the status says what was marked, the document
//!    on disk is untouched (marks are not text) and the session written on close keeps the find
//!    history.
//! 3. Screen `replace`: the Replace page with a regular expression counts its matches.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; everything happens in a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`).
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn jarak(args: &[&str]) -> (i32, serde_json::Value) {
    let out = Command::new(bin("jarak"))
        .args(args)
        .arg("--json")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: one JSON document ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out.status.code().unwrap_or(-1), v)
}

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let shot = dir.join(format!("{screen}.png"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step94-{}", std::process::id()),
            "--lang".to_string(),
            "en".to_string(),
            format!("{}:1:1", doc.display()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", "1")
        .env("KIM_UI_SCREEN", screen)
        .env("KIM_UI_SESSION", "1")
        .env("KIM_UI_THEME", "dark")
        .env("KIM_UI_POS", "40,40")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(40);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let exited = child.try_wait().unwrap().is_some();
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        exited,
        "{screen}: the window closed by itself after the measurement"
    );
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json).expect("the geometry report")).unwrap();
    let r = kim_design::inspect::Report::from_json(&v).expect("schema kim.ui-geometry");
    let f = kim_design::audit::audit(&r);
    assert_eq!(
        kim_design::audit::counts(&f).2,
        0,
        "{}",
        kim_design::audit::render(&r, &f, false)
    );
    r
}

/// The texts drawn in the window.
fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step94_jarak_find_and_the_find_panel_in_the_real_window() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step94");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let file = docs.join("notes.txt");
    let bytes = b"the cat sat\r\non a Cat mat.\tcat\r\nconcat\r\n".to_vec();
    std::fs::write(&file, &bytes).unwrap();
    let f = file.to_str().unwrap();

    // 1. The CLI.
    let (code, v) = jarak(&["find", f, "cat", "--whole-word"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["command"], "find");
    assert_eq!(v["data"]["count"], 3, "not in concat: {v}");
    let m = &v["data"]["matches"];
    assert_eq!(
        (m[0]["line"].as_u64(), m[0]["column"].as_u64()),
        (Some(1), Some(5))
    );
    assert_eq!(m[1]["text"], "Cat", "any case by default");
    let (code, v) = jarak(&["find", f, "c[a-z]t$", "--regex", "--match-case"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["count"], 2, "line ends: cat, concat: {v}");
    let (code, v) = jarak(&["find", f, r"\tcat", "--extended"]);
    assert_eq!((code, v["data"]["count"].as_u64()), (0, Some(1)), "{v}");
    let (code, v) = jarak(&["find", f, "a", "--count"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["count"], 7, "{v}");
    assert_eq!(v["data"]["matches"].as_array().map(Vec::len), Some(0));
    let (code, v) = jarak(&["find", f, "(", "--regex"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "usage");
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "find reads only");

    // 2. The Mark page in the real window.
    let md = docs.join("notes.md");
    let md_bytes = b"# Markdown notes\n\nA Step about Markdown.\nAnother Step.\n".to_vec();
    std::fs::write(&md, &md_bytes).unwrap();
    let r = window(&dir, &md, "mark");
    let t = texts(&r);
    for want in [
        "Mark All",
        "Clear marks",
        "Copy marked",
        "Bookmark the lines",
    ] {
        assert!(t.iter().any(|s| s == want), "{want} in the panel: {t:?}");
    }
    assert!(t.iter().any(|s| s == "Marked: 2"), "both Markdowns: {t:?}");
    assert_eq!(std::fs::read(&md).unwrap(), md_bytes, "marks are not text");
    let session = dir
        .join("Local")
        .join("mouv")
        .join("jarak")
        .join("session.json");
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&session).expect("the session was written")).unwrap();
    assert_eq!(v["find_history"][0], "Markdown", "{v}");

    // 3. The Replace page counts a regular expression's matches; nothing is replaced.
    let r = window(&dir, &md, "replace");
    let t = texts(&r);
    assert!(t.iter().any(|s| s == "Replace All"), "{t:?}");
    assert!(t.iter().any(|s| s == "Matches: 2"), "{t:?}");
    assert_eq!(std::fs::read(&md).unwrap(), md_bytes);
}
