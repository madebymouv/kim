//! E2E — Step 104 (#123): recovery snapshots offered by the real window, a damaged session falling
//! back to its backup, and the disk question (docs/JARAK_ARCHITECTURE.md §26).
//! Requirements: `JARAK-FR-120`, `JARAK-FR-121`.
//!
//! 1. A recovery folder left by a window that no longer runs (a process id no process can have)
//!    is offered at start by the release window with a session (`KIM_UI_SESSION=1`): both of its
//!    documents are listed; nothing is restored or removed without the user (the folder and the
//!    document's file stay as they are); the window's own folder is gone after its normal exit.
//! 2. A damaged `session.json` falls back to `session.json.bak`: the backup's document opens and
//!    the damaged file is kept as `session.json.broken`, byte for byte.
//! 3. `disk-question` (a file changed on disk and here) and `recovery` pass the audit's rules.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; everything happens in a temporary folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::{Value, json};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(
    dir: &Path,
    doc: Option<&Path>,
    screen: &str,
    session: bool,
) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let mut args = vec![
        "--instance".to_string(),
        format!("e2e-step104-{}", std::process::id()),
        "--lang".to_string(),
        "en".to_string(),
    ];
    if let Some(d) = doc {
        args.push(format!("{}:1:1", d.display()));
    }
    let mut cmd = Command::new(bin("jarak-gui"));
    cmd.args(&args)
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
        .stderr(Stdio::null());
    if session {
        cmd.env("KIM_UI_SESSION", "1");
    }
    let mut child = cmd.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(40);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let exited = child.try_wait().unwrap().is_some();
    let _ = child.kill();
    let _ = child.wait();
    assert!(exited, "{screen}: the window closed by itself");
    let v: Value =
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

fn text_of(r: &kim_design::inspect::Report, id: &str) -> Option<String> {
    r.components
        .iter()
        .find(|c| c.id == id)
        .and_then(|c| c.text.as_ref().map(|t| t.content.clone()))
}

fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step104_recovery_offer_session_fallback_and_the_disk_question() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step104");
    // 1. A dead window's recovery folder.
    let dir = sb.path().join("offer");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let notes = docs.join("notes.md");
    std::fs::write(&notes, "# 자락\n").unwrap();
    let root = dir
        .join("Local")
        .join("mouv")
        .join("jarak")
        .join("recovery");
    let gone = root.join("w-4294967294-1");
    std::fs::create_dir_all(&gone).unwrap();
    std::fs::write(gone.join("doc-1.txt"), "# 자락\n저장하지 않은 편집\n").unwrap();
    std::fs::write(gone.join("doc-2.txt"), "untitled note").unwrap();
    let entry = |id: u64, path: Option<&Path>, title: &str, chars: usize| {
        let mut e = json!({
            "id": id, "title": title, "text": format!("doc-{id}.txt"), "encoding": "utf-8",
            "bom": false, "eol": "lf", "caret": 0, "chars": chars, "saved_at": 1_795_000_000u64
        });
        if let Some(p) = path {
            e["path"] = json!(p);
        }
        e
    };
    let manifest = json!({
        "schema": "jarak.recovery", "schema_version": 1,
        "owner": { "pid": 4_294_967_294u32, "started": 1 },
        "entries": [entry(1, Some(&notes), "notes.md", 16), entry(2, None, "Untitled", 13)]
    });
    std::fs::write(
        gone.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let r = window(&dir, None, "recovery-start", true);
    let row0 = text_of(&r, "recovery:row#0").expect("the offer lists the documents");
    assert!(row0.starts_with("notes.md"), "{row0}");
    assert!(text_of(&r, "recovery:row#1").is_some_and(|t| t.starts_with("Untitled")));
    assert!(texts(&r).iter().any(|t| t == "Restore"));
    assert!(
        gone.join("manifest.json").is_file(),
        "nothing removed without the user"
    );
    assert_eq!(
        std::fs::read_to_string(&notes).unwrap(),
        "# 자락\n",
        "the file is untouched"
    );
    let own: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "w-4294967294-1")
        .collect();
    assert!(
        own.is_empty(),
        "the window keeps no folder after it exits: {own:?}"
    );
    // 2. A damaged session falls back to the backup.
    let dir = sb.path().join("session");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let a = docs.join("from-backup.md");
    std::fs::write(&a, "kept\n").unwrap();
    // Two documents, so the tab bar shows their names.
    let b = docs.join("second.txt");
    std::fs::write(&b, "two\n").unwrap();
    let local = dir.join("Local").join("mouv").join("jarak");
    std::fs::create_dir_all(&local).unwrap();
    let good = json!({
        "schema": "jarak.session", "schema_version": 1,
        "documents": [{ "path": a }, { "path": b }], "active": 0
    });
    std::fs::write(
        local.join("session.json.bak"),
        serde_json::to_vec(&good).unwrap(),
    )
    .unwrap();
    std::fs::write(local.join("session.json"), b"{\"schema\":\"jarak.sess").unwrap();
    let r = window(&dir, None, "session-fallback", true);
    for name in ["from-backup.md", "second.txt"] {
        assert!(
            texts(&r).iter().any(|t| t == name),
            "the backup's documents are open: {:?}",
            texts(&r)
        );
    }
    assert_eq!(
        std::fs::read(local.join("session.json.broken")).unwrap(),
        b"{\"schema\":\"jarak.sess",
        "the damaged file is kept as it was"
    );
    // 3. The disk question and the recovery offer under the audit's rules.
    let dir = sb.path().join("screens");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let doc = docs.join("notes.md");
    std::fs::write(&doc, "# notes\n").unwrap();
    let r = window(&dir, Some(&doc), "disk-question", false);
    assert!(text_of(&r, "disk:question").is_some_and(|t| t.contains("another program")));
    assert!(texts(&r).iter().any(|t| t == "Keep mine"));
    assert!(texts(&r).iter().any(|t| t == "Compare"));
    let r = window(&dir, Some(&doc), "recovery", false);
    assert!(text_of(&r, "recovery:row#0").is_some_and(|t| t.starts_with("sample.rs")));
    assert_eq!(std::fs::read_to_string(&doc).unwrap(), "# notes\n");
}
