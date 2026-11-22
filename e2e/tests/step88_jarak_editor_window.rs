//! E2E — Step 88 (#107): Jarak's editor window, measured and photographed (ADR 0099, ADR 0102).
//! Requirements: `JARAK-FR-030`, `JARAK-FR-034`, `JARAK-FR-035`, `JARAK-FR-024`,
//! `JARAK-SEC-001`, `KIM-NFR-012`.
//!
//! The release `jarak-gui` opens the UI fixture (`design/ui-fixtures/jarak-sample.md`: Korean,
//! tabs, a long line, emoji) in a sandboxed profile with the UI audit's environment
//! (`KIM_UI_INSPECT`, `KIM_UI_SNAPSHOT`, `KIM_UI_EXIT`, zoom 100 % and 150 %):
//!
//! 1. `editor`: the geometry report is Jarak's, the design-system audit finds no failure (the gear
//!    in the status bar is `size::GEAR`), the photograph is the real window's pixels.
//! 2. `question`: the window typed into its document and asked about the unsaved change; the
//!    audit finds no failure in the question.
//! 3. In both runs the document on disk is byte-identical afterwards: viewing never writes, and an
//!    unsaved change is never saved without the user's answer.
//!
//! Machine safety: the window opens for about a second at the audit position; no input is
//! synthesized; nothing outside the temporary folder is touched.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(sb: &TempDir, screen: &str, zoom: &str) -> (serde_json::Value, std::path::PathBuf, Vec<u8>) {
    let dir = sb.path().join(format!("{screen}-{zoom}"));
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("design")
        .join("ui-fixtures")
        .join("jarak-sample.md");
    let doc = dir.join("sample.md");
    std::fs::copy(&fixture, &doc).unwrap();
    let before = std::fs::read(&doc).unwrap();
    let json = dir.join("geometry.json");
    let shot = dir.join("frame.png");
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance",
            &format!("e2e-step88-{}", std::process::id()),
            "--lang",
            "en",
            &format!("{}:3:9", doc.display()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", zoom)
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
    assert!(
        exited,
        "{screen}: the window closed by itself after the measurement"
    );
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json).expect("the geometry report")).unwrap();
    assert_eq!(
        std::fs::read(&doc).unwrap(),
        before,
        "{screen}: the document is unchanged"
    );
    (v, shot, before)
}

fn audited(v: &serde_json::Value) -> kim_design::inspect::Report {
    let r = kim_design::inspect::Report::from_json(v).expect("schema kim.ui-geometry");
    let f = kim_design::audit::audit(&r);
    assert_eq!(
        kim_design::audit::counts(&f).2,
        0,
        "{}",
        kim_design::audit::render(&r, &f, false)
    );
    r
}

fn png_size(path: &std::path::Path) -> (u32, u32) {
    let b = std::fs::read(path).expect("the photograph");
    assert_eq!(&b[1..4], b"PNG");
    let be = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    (be(16), be(20))
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step88_the_editor_window_passes_the_audit_and_never_writes_the_document() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step88");
    for zoom in ["1", "1.5"] {
        let (v, shot, _) = run(&sb, "editor", zoom);
        let r = audited(&v);
        assert_eq!(
            (r.application.as_str(), r.screen.as_str()),
            ("jarak", "editor")
        );
        let gear = r
            .components
            .iter()
            .find(|c| c.id.starts_with("gear:"))
            .expect("the Settings gear is in the report");
        assert!(
            gear.bounds.x + gear.bounds.w > r.window.0 - 24.0 * zoom.parse::<f32>().unwrap()
                && gear.bounds.y + gear.bounds.h > r.window.1 - 40.0 * zoom.parse::<f32>().unwrap(),
            "the gear sits in the bottom-right corner: {:?} in {:?}",
            gear.bounds,
            r.window
        );
        assert_eq!(
            png_size(&shot),
            (r.window.0 as u32, r.window.1 as u32),
            "the real window's pixels"
        );
    }
    let (v, _, _) = run(&sb, "question", "1");
    let r = audited(&v);
    assert_eq!(r.screen, "question");
}
