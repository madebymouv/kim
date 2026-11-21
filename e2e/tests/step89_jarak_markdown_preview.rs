//! E2E — Step 89 (#108): Jarak's Markdown preview, measured and photographed (ADR 0099, ADR 0102).
//! Requirements: `JARAK-FR-040`, `JARAK-FR-041`, `JARAK-SEC-001`, `KIM-NFR-012`.
//!
//! The release `jarak-gui` opens the UI fixture (`design/ui-fixtures/jarak-sample.md`: headings,
//! emphasis, links, task and ordered lists, a quote, a code block, a table, a rule, a footnote,
//! Korean) in a sandboxed profile with the UI audit's environment, in the `preview` view (the
//! rendered document alone) and the `split` view (source and preview side by side), at 100 % and
//! 150 %: the geometry report is Jarak's, the design-system audit finds no failure, the
//! photograph is the real window's pixels, and the Markdown file on disk is byte-identical
//! afterwards — rendering never writes.
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
            &format!("e2e-step89-{}", std::process::id()),
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
fn step89_the_preview_and_split_views_pass_the_audit_and_never_write_the_markdown() {
    let sb = TempDir::new("e2e-step89");
    for screen in ["preview", "split"] {
        for zoom in ["1", "1.5"] {
            let (v, shot, _) = run(&sb, screen, zoom);
            let r = audited(&v);
            assert_eq!(
                (r.application.as_str(), r.screen.as_str()),
                ("jarak", screen)
            );
            assert_eq!(
                png_size(&shot),
                (r.window.0 as u32, r.window.1 as u32),
                "the real window's pixels"
            );
        }
    }
}
