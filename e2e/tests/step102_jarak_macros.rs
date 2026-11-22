//! E2E — Step 102 (#121): the Run dialog and the macros dialog in the real window
//! (docs/JARAK_MACROS.md, docs/JARAK_ARCHITECTURE.md §24).
//! Requirements: `JARAK-FR-110`, `JARAK-FR-111`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. `run`: the Run dialog with `notepad.exe "$(FULL_CURRENT_PATH)"` — it shows the exact line
//!    with the sample's path, and nothing runs (no Run button is pressed: no input is synthesized).
//! 2. `macros`: the macros dialog lists a saved macro with its shortcut.
//!
//! Neither changes the document it was started with. Machine safety: the windows open
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
            format!("e2e-step102-{}", std::process::id()),
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
fn step102_the_run_dialog_shows_the_exact_line_and_the_macros_dialog_lists_macros() {
    let sb = TempDir::new("e2e-step102");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let doc = docs.join("notes.md");
    let text = "# 자락\n\nThe audit writes its samples next to this file.\n";
    std::fs::write(&doc, text).unwrap();
    let r = window(&dir, &doc, "run");
    let line = text_of(&r, "run:line");
    let sample = docs.join("sample.rs").display().to_string();
    assert_eq!(
        line,
        [format!("notepad.exe \"{sample}\"")],
        "the exact line"
    );
    let r = window(&dir, &doc, "macros");
    let rows = text_of(&r, "");
    assert!(rows.iter().any(|s| s == "Tidy"), "{rows:?}");
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        text,
        "the document is untouched"
    );
}
