//! Step 104 (`JARAK-FR-121`): a window killed with unsaved documents leaves its recovery
//! snapshots, and the next start finds them with the text intact — with a real child process that
//! is killed (TerminateProcess), and the real Windows check of whether the owner still runs.
#![cfg(windows)]

use jarak_core::recovery::{self, Entry, Snapshot, Store};
use ropey::Rope;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const CHILD: &str = "JARAK_RECOVERY_CHILD";
const TEXT: &str = "# 자락\r\n저장하지 않은 편집\r\nunsaved line\r\n";

fn snapshot(id: u64, path: Option<&str>, text: &str) -> Snapshot {
    Snapshot {
        entry: Entry {
            id,
            path: path.map(PathBuf::from),
            title: path.unwrap_or("Untitled").into(),
            text: format!("doc-{id}.txt"),
            encoding: "utf-8".into(),
            bom: false,
            eol: "crlf".into(),
            caret: 4,
            chars: text.chars().count(),
            saved_at: recovery::now_secs(),
            disk_len: None,
            disk_modified_ms: None,
        },
        text: Some(Rope::from_str(text)),
    }
}

#[test]
fn a_killed_window_leaves_snapshots_that_the_next_start_finds() {
    // The child: writes its snapshots as a window would, says it is ready, and waits to be killed.
    if let Ok(root) = std::env::var(CHILD) {
        let root = PathBuf::from(root);
        let s = Store::new(&root, recovery::current_owner());
        s.write(&[
            snapshot(1, Some(r"C:\notes\a.md"), TEXT),
            snapshot(2, None, "untitled 메모"),
        ])
        .unwrap();
        std::fs::write(root.join("ready"), b"").unwrap();
        loop {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let root = std::env::temp_dir().join(format!("jarak-recovery-kill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_killed_window_leaves_snapshots_that_the_next_start_finds",
            "--test-threads=1",
        ])
        .env(CHILD, &root)
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !root.join("ready").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the child never wrote its snapshots"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    // While the window runs, its snapshots are its own.
    assert!(
        recovery::orphans(&root, &recovery::owner_alive).is_empty(),
        "a running window's folder is never offered"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    let found = recovery::orphans(&root, &recovery::owner_alive);
    assert_eq!(found.len(), 1, "the killed window's folder is offered");
    let o = &found[0];
    assert_eq!(o.manifest.owner.pid, child.id());
    let e = &o.manifest.entries;
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].path.as_deref(), Some(Path::new(r"C:\notes\a.md")));
    assert_eq!(o.text(&e[0]).unwrap(), TEXT, "the text is intact");
    assert_eq!(o.text(&e[1]).unwrap(), "untitled 메모");
    let _ = std::fs::remove_dir_all(&root);
}
