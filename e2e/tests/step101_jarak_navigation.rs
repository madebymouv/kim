//! E2E — Step 101 (#120): the Document Map, quick open and the Summary in the real window
//! (docs/JARAK_ARCHITECTURE.md §23).
//! Requirements: `JARAK-FR-100`…`JARAK-FR-104`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. `docmap`: the Rust sample with the map open, a bookmark and a changed line — the map's
//!    frame of the lines on screen is in the geometry report, the audit finds no failure.
//! 2. `quick-open`: the dialog lists the documents' folder's files.
//! 3. `summary`: the Summary names its rows.
//!
//! None of them changes the document they were started with. Machine safety: the windows open
//! for about a second each at the audit position; no input is synthesized; everything happens in
//! a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`).
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step101-{}", std::process::id()),
            "--lang".to_string(),
            "en".to_string(),
            format!("{}:1:1", doc.display()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", dir.join(format!("{screen}.png")))
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", "1")
        .env("KIM_UI_SCREEN", screen)
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
    assert!(exited, "{screen}: the window closed by itself");
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

fn text_of(r: &kim_design::inspect::Report, prefix: &str) -> Vec<String> {
    r.components
        .iter()
        .filter(|c| c.id.starts_with(prefix))
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step101_the_document_map_quick_open_and_the_summary() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step101");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let doc = docs.join("notes.md");
    let text = "# 자락

The audit writes its samples next to this file.
";
    std::fs::write(&doc, text).unwrap();
    let r = window(&dir, &doc, "docmap");
    assert!(
        r.components.iter().any(|c| c.id == "docmap:view"),
        "the map's frame"
    );
    let r = window(&dir, &doc, "quick-open");
    let names = text_of(&r, "");
    assert!(names.iter().any(|s| s.contains("notes.md")), "{names:?}");
    let r = window(&dir, &doc, "summary");
    let labels = text_of(&r, "summary:");
    for row in ["Words", "Lines", "Document length (bytes)"] {
        assert!(labels.iter().any(|s| s == row), "{row}: {labels:?}");
    }
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        text,
        "the document is untouched"
    );
}
