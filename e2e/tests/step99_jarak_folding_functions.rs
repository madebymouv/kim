//! E2E — Step 99 (#118): folding, the Function List and the Go to dialog in the real window
//! (docs/JARAK_ARCHITECTURE.md §21).
//! Requirements: `JARAK-FR-092`, `JARAK-FR-093`, `JARAK-FR-103`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. `functions`: a Rust sample with the Function List open and its `if` block folded — the
//!    list names the struct and the function, the folded lines are not drawn, the audit finds no
//!    failure, and the sample on disk is what was written.
//! 2. `goto`: the Go to dialog over the sample.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; everything happens in a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`).
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
            format!("e2e-step99-{}", std::process::id()),
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

fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step99_the_function_list_and_a_folded_block_and_the_go_to_dialog() {
    let sb = TempDir::new("e2e-step99");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let doc = docs.join("notes.md");
    let text = "# 자락\n\nThe audit writes `sample.rs` next to this file.\n";
    std::fs::write(&doc, text).unwrap();
    let r = window(&dir, &doc, "functions");
    let t = texts(&r);
    for row in ["Point  struct", "main  fn"] {
        assert!(
            t.iter().any(|s| s == row),
            "{row} in the Function List: {t:?}"
        );
    }
    assert!(t.iter().any(|s| s == "Function list"), "{t:?}");
    // The folded `if` block (line 10): its body (lines 11 and 12) is not drawn, the line after
    // the fold is.
    let numbers: Vec<String> = r
        .components
        .iter()
        .filter(|c| c.id.starts_with("gutter:line#"))
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect();
    for (n, drawn) in [("10", true), ("11", false), ("12", false), ("13", true)] {
        assert_eq!(
            numbers.iter().any(|x| x == n),
            drawn,
            "line {n}: {numbers:?}"
        );
    }
    let sample = std::fs::read_to_string(docs.join("sample.rs")).unwrap();
    assert!(sample.contains("println!"), "folding changes no file");
    assert_eq!(std::fs::read_to_string(&doc).unwrap(), text);
    let r = window(&dir, &doc, "goto");
    let t = texts(&r);
    // The Go to dialog (its title is drawn by the dialog frame, not in the report): Line / Offset,
    // Go, and where the caret is.
    for s in ["Line", "Offset", "Go"] {
        assert!(t.iter().any(|x| x == s), "{s}: {t:?}");
    }
    assert!(
        r.components.iter().any(|c| c.id == "goto:where"
            && c.text
                .as_ref()
                .is_some_and(|x| x.content.starts_with("You are on line"))),
        "{t:?}"
    );
}
