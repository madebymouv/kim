//! E2E — Step 90 (#109): the MVP journey through the real window (ADR 0102, docs/JARAK_REQUIREMENTS.md §5).
//! Requirements: `JARAK-FR-042`, `JARAK-FR-044`, `JARAK-SEC-005`, `KIM-NFR-012`.
//!
//! The release `jarak-gui` with the UI audit's environment on a short Markdown note in a
//! temporary folder:
//!
//! 1. `paste-image`: the window pastes a 320 × 180 bitmap (what a screenshot tool puts on the
//!    clipboard) at the end of the Markdown document — Jarak writes
//!    `assets/sample-<date>-001.png` next to it and inserts `![image](assets/…)` — saves, and
//!    shows the preview; the audit finds no failure and the geometry report holds the drawn
//!    image.
//! 2. `preview` again, a new process on the saved document (close and reopen): the reference is
//!    in the file, the asset is a 320 × 180 PNG, and the preview draws it.
//!
//! Machine safety: the window opens for about a second at the audit position; no input is
//! synthesized; nothing outside the temporary folder is touched.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(sb: &TempDir, screen: &str, zoom: &str) -> (serde_json::Value, std::path::PathBuf, Vec<u8>) {
    let dir = sb.path().join("journey");
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let doc = dir.join("sample.md");
    if !doc.exists() {
        // A short note, so the pasted picture is on screen in the preview.
        std::fs::write(&doc, "# Journey\n\nA screenshot below:\n\n").unwrap();
    }
    let json = dir.join(format!("{screen}.json"));
    let shot = dir.join(format!("{screen}.png"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance",
            &format!("e2e-step90-{}", std::process::id()),
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
    let after = std::fs::read(&doc).unwrap();
    (v, shot, after)
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

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step90_paste_a_screenshot_save_close_reopen_and_see_it() {
    let sb = TempDir::new("e2e-step90");
    let dir = sb.path().join("journey");
    // 1. Paste, save, preview.
    let (v, _, saved) = run(&sb, "paste-image", "1");
    let r = audited(&v);
    assert_eq!(r.screen, "paste-image");
    let text = String::from_utf8(saved).unwrap();
    let start = text
        .find("![image](assets/")
        .expect("the reference was saved");
    let reference = &text[start + 9..text[start..].find(')').unwrap() + start];
    assert!(
        reference.starts_with("assets/sample-") && reference.ends_with("-001.png"),
        "{reference}"
    );
    let png = std::fs::read(dir.join(reference)).expect("the asset next to the document");
    assert_eq!(&png[1..4], b"PNG");
    let be = |i: usize| u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
    assert_eq!((be(16), be(20)), (320, 180), "the pasted picture's size");
    assert!(
        r.components.iter().any(|c| c.id == "preview:image"),
        "the preview drew the image"
    );
    // 2. Close and reopen: a new process shows the saved document's image.
    let (v, _, again) = run(&sb, "preview", "1");
    let r = audited(&v);
    assert_eq!(
        String::from_utf8(again).unwrap(),
        text,
        "reopening changed nothing"
    );
    let img = r
        .components
        .iter()
        .find(|c| c.id == "preview:image")
        .expect("the reopened preview drew the image");
    assert!(
        (img.bounds.w - 320.0).abs() < 1.0 && (img.bounds.h - 180.0).abs() < 1.0,
        "{:?}",
        img.bounds
    );
}
