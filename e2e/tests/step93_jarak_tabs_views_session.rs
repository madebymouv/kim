//! E2E — Step 93 (#112): tabs, two views and the session through the real window
//! (ADR 0102, docs/JARAK_ARCHITECTURE.md §15).
//! Requirements: `JARAK-FR-060`, `JARAK-FR-061`, `JARAK-FR-062`, `KIM-NFR-012`.
//!
//! The release `jarak-gui` with the UI audit's environment and a session kept in its sandbox
//! (`KIM_UI_SESSION=1`):
//!
//! 1. `views`: three documents in the main view's tabs, the first cloned into the second view.
//!    The geometry report holds both tab bars (three tabs and one) and the audit finds no failure;
//!    closing the window writes `%LOCALAPPDATA%\mouv\jarak\session.json`, which names the three
//!    documents and the clone in both views (and validates against the schema's shape).
//! 2. A second start without files restores them: the same tab bars, the clone in the second view.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; everything happens in a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`).
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(dir: &Path, doc: Option<&Path>, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let shot = dir.join(format!("{screen}.png"));
    let mut args = vec![
        "--instance".to_string(),
        format!("e2e-step93-{}", std::process::id()),
        "--lang".to_string(),
        "en".to_string(),
    ];
    if let Some(d) = doc {
        args.push(format!("{}:1:1", d.display()));
    }
    let mut child = Command::new(bin("jarak-gui"))
        .args(&args)
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

/// The tabs of each bar: (bar id, number of tabs).
fn bars(r: &kim_design::inspect::Report) -> Vec<(String, usize)> {
    r.components
        .iter()
        .filter(|c| c.kind == "tabbar")
        .map(|b| {
            let n = r
                .components
                .iter()
                .filter(|c| c.kind == "tab" && c.parent.as_deref() == Some(b.id.as_str()))
                .count();
            (b.id.clone(), n)
        })
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step93_tabs_two_views_and_the_session_across_a_restart() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step93");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let sample = docs.join("sample.md");
    std::fs::write(&sample, "# Sample\n\nThe first document.\n").unwrap();
    // 1. Three documents, the first cloned into the second view.
    let r = window(&dir, Some(&sample), "views");
    assert_eq!(
        bars(&r),
        vec![
            ("tabbar:main".to_string(), 3),
            ("tabbar:second".to_string(), 1)
        ],
        "both tab bars"
    );
    let session = dir
        .join("Local")
        .join("mouv")
        .join("jarak")
        .join("session.json");
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&session).expect("the session was written")).unwrap();
    assert_eq!(v["schema"], "jarak.session");
    assert_eq!(v["schema_version"], 1);
    let docs_v = v["documents"].as_array().unwrap();
    assert_eq!(docs_v.len(), 3, "{v}");
    let both: Vec<&serde_json::Value> = docs_v.iter().filter(|d| d["views"] == "both").collect();
    assert_eq!(both.len(), 1, "the clone is in both views: {v}");
    assert!(both[0]["path"].as_str().unwrap().ends_with("sample.md"));
    assert_eq!(v["second"]["order"].as_array().unwrap().len(), 1);
    // 2. A new start without files restores the documents and the views.
    let r = window(&dir, None, "editor");
    assert_eq!(
        bars(&r),
        vec![
            ("tabbar:main".to_string(), 3),
            ("tabbar:second".to_string(), 1)
        ],
        "restored after the restart"
    );
}
