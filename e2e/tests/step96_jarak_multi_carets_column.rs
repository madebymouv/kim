//! E2E — Step 96 (#115): multiple carets, multi-selection and column mode in the real window
//! (ADR 0102, docs/JARAK_ARCHITECTURE.md §18).
//! Requirements: `JARAK-FR-080`, `JARAK-FR-081`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. `multi`: every "Step" of a document selected at once — the status bar counts the carets and
//!    the audit finds no failure; the file on disk is untouched.
//! 2. `column`: a column selection over lines with Hangul (display columns 4 to 12) drawn with no
//!    failure.
//! 3. `column-editor`: the Column Editor's text and number pages, leading zeros and the bases.
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
    let shot = dir.join(format!("{screen}.png"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step96-{}", std::process::id()),
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
fn step96_multiple_carets_column_selection_and_the_column_editor() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step96");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let md = docs.join("notes.md");
    let bytes = "# Step one

Jarak은 Step two
    Tab으로 Step three
        두 번 줄
"
    .as_bytes()
    .to_vec();
    std::fs::write(&md, &bytes).unwrap();

    // 1. Every "Step" selected: three carets in the status bar.
    let r = window(&dir, &md, "multi");
    let t = texts(&r);
    assert!(t.iter().any(|s| s.contains("3 carets")), "{t:?}");
    assert_eq!(
        std::fs::read(&md).unwrap(),
        bytes,
        "selecting changes nothing"
    );

    // 2. A column selection over Hangul lines.
    let r = window(&dir, &md, "column");
    let t = texts(&r);
    assert!(t.iter().any(|s| s.contains("carets")), "{t:?}");

    // 3. The Column Editor.
    let r = window(&dir, &md, "column-editor");
    let t = texts(&r);
    for want in [
        "Text to insert",
        "Number to insert",
        "Leading zeros",
        "Hex",
        "Insert",
    ] {
        assert!(t.iter().any(|s| s == want), "{want}: {t:?}");
    }
    assert_eq!(std::fs::read(&md).unwrap(), bytes);
}
