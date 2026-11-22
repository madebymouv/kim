//! E2E — Step 97 (#116): line operations and text transformations in the real window
//! (ADR 0102, docs/JARAK_ARCHITECTURE.md §19).
//! Requirements: `JARAK-FR-082`, `JARAK-FR-083`, `JARAK-FR-084`, `JARAK-FR-088`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. `lineops-session`: registry commands run on a CRLF document with Hangul — remove duplicate
//!    lines, sort as integers, select all, Proper Case — and the window saves it: the bytes on disk
//!    are the expected ones, line endings kept.
//! 2. `palette-lines`: the palette lists the sort commands; `hash`: the digest dialog shows a
//!    64-digit SHA-256; no audit failure in either.
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
            format!("e2e-step97-{}", std::process::id()),
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
fn step97_line_operations_run_and_save_and_the_tools_are_listed() {
    let sb = TempDir::new("e2e-step97");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let f = docs.join("list.txt");
    std::fs::write(
        &f,
        "10 바나나 b\r\n2 apple 사과\r\n10 바나나 b\r\n1 cherry\r\n".as_bytes(),
    )
    .unwrap();

    // 1. Remove duplicates, sort as integers, Proper Case, saved.
    window(&dir, &f, "lineops-session");
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "1 Cherry\r\n2 Apple 사과\r\n10 바나나 B\r\n"
    );

    // 2. The palette's sort commands and the digest dialog.
    let r = window(&dir, &f, "palette-lines");
    let t = texts(&r);
    for want in [
        "Sort lines as integers ascending",
        "Sort lines in locale order descending",
    ] {
        assert!(t.iter().any(|s| s == want), "{want}: {t:?}");
    }
    let r = window(&dir, &f, "hash");
    let t = texts(&r);
    assert!(
        t.iter()
            .any(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())),
        "a SHA-256 digest: {t:?}"
    );
}
