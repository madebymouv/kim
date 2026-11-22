//! E2E — Step 95 (#114): Find and Replace in Files — `jarak find <folder>`, `jarak replace`
//! (`--dry-run`, `--yes`), `jarak journal list/undo` and the real window's Files page and results
//! panel (ADR 0102, docs/JARAK_ARCHITECTURE.md §17).
//! Requirements: `JARAK-FR-073`, `JARAK-FR-074`, `JARAK-SEC-009`, `KIM-FR-012`.
//!
//! 1. A fixture tree (UTF-8 LF, UTF-8 CRLF, UTF-16LE with a BOM, CP949, a binary file, a
//!    sub-folder and an excluded `build` folder): `jarak find <folder>` with filters lists the files
//!    and lines and skips the binary file.
//! 2. `jarak replace --dry-run` lists every change and writes nothing (every byte compared);
//!    without `--yes` it is `confirmation_required` (exit 7) and writes nothing; with `--yes` it
//!    replaces, each file keeping its encoding, byte-order mark and line endings (byte-level).
//! 3. `jarak journal list` names the batch; `jarak journal undo <batch> --yes` restores every byte.
//! 4. The release window with the UI audit's environment, screen `results`: the Files page and the
//!    results panel grouped by file are drawn with no audit failure.
//!
//! Machine safety: everything happens in a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`,
//! so the journal is the sandbox's); the window opens for about a second; no input is synthesized.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn jarak(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = Command::new(bin("jarak"))
        .args(args)
        .arg("--json")
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: one JSON document ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out.status.code().unwrap_or(-1), v)
}

fn utf16le_bom(s: &str) -> Vec<u8> {
    let mut v = vec![0xFF, 0xFE];
    for u in s.encode_utf16() {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

/// "한글 cat\r\n" in CP949.
const CP949: &[u8] = b"\xc7\xd1\xb1\xdb cat\r\n";

fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("a.txt", b"one cat\r\ntwo cats\r\n".to_vec()),
        ("b.md", b"# cat\n\nconcat cat\n".to_vec()),
        ("c.txt", utf16le_bom("UTF-16 cat\r\n")),
        ("d.txt", CP949.to_vec()),
        ("e.bin", b"cat\0\0binary".to_vec()),
        ("sub/f.txt", b"deep cat\n".to_vec()),
        ("build/g.txt", b"built cat\n".to_vec()),
    ];
    files
        .into_iter()
        .map(|(n, b)| {
            let p = root.join(n);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, &b).unwrap();
            (p, b)
        })
        .collect()
}

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    let json = dir.join(format!("{screen}.json"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step95-{}", std::process::id()),
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

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step95_find_and_replace_in_files_with_the_journal_and_the_results_panel() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step95");
    let dir = sb.path().join("run");
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let root = dir.join("tree");
    let before = tree(&root);
    let r = root.to_str().unwrap();
    let unchanged = |what: &str| {
        for (p, b) in &before {
            assert_eq!(
                &std::fs::read(p).unwrap(),
                b,
                "{what}: {} untouched",
                p.display()
            );
        }
    };

    // 1. Find in Files.
    let (code, v) = jarak(
        &dir,
        &[
            "find",
            r,
            "cat",
            "--whole-word",
            "--filters",
            "*.txt *.md *.bin !\\build",
        ],
    );
    assert_eq!(code, 0, "{v}");
    let d = &v["data"];
    assert_eq!(d["count"], 6, "{v}");
    let files: Vec<&str> = d["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 5, "{files:?}");
    assert!(
        files[0].ends_with("a.txt") && files[4].ends_with("f.txt"),
        "{files:?}"
    );
    assert_eq!(d["files"][2]["encoding"], "UTF-16 LE BOM", "{v}");
    assert_eq!(d["skipped"][0]["reason"], "binary", "{v}");
    assert_eq!(
        d["files"][3]["matches"][0]["line_text"], "한글 cat",
        "CP949 decoded"
    );

    // 2. Replace: dry run, refused without --yes, then done.
    let args = [
        "replace",
        r,
        "cat",
        "dog",
        "--whole-word",
        "--filters",
        "*.txt *.md !\\build",
    ];
    let (code, v) = jarak(&dir, &[&args[..], &["--dry-run"]].concat());
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["planned"], 6);
    assert_eq!(
        v["data"]["files"][1]["changes"][1]["new_line"],
        "concat dog"
    );
    unchanged("--dry-run");
    let (code, v) = jarak(&dir, &args);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("confirmation_required")),
        "{v}"
    );
    unchanged("without --yes");
    let (code, v) = jarak(&dir, &[&args[..], &["--yes"]].concat());
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["replaced"], 6);
    let batch = v["data"]["batch"]["id"].as_str().unwrap().to_string();
    let rd = |n: &str| std::fs::read(root.join(n)).unwrap();
    assert_eq!(rd("a.txt"), b"one dog\r\ntwo cats\r\n");
    assert_eq!(rd("b.md"), b"# dog\n\nconcat dog\n");
    assert_eq!(rd("c.txt"), utf16le_bom("UTF-16 dog\r\n"));
    assert_eq!(rd("d.txt"), b"\xc7\xd1\xb1\xdb dog\r\n");
    assert_eq!(rd("e.bin"), b"cat\0\0binary");
    assert_eq!(rd("build/g.txt"), b"built cat\n");

    // 3. The journal and undo.
    let (code, v) = jarak(&dir, &["journal", "list"]);
    assert_eq!(
        (code, v["data"]["batches"][0]["id"].as_str()),
        (0, Some(batch.as_str())),
        "{v}"
    );
    let (code, v) = jarak(&dir, &["journal", "undo", &batch, "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["restored"].as_array().unwrap().len(), 5);
    unchanged("after undo");

    // 4. The window's Files page and results panel.
    let rep = window(&dir, &root.join("a.txt"), "results");
    let texts: Vec<String> = rep
        .components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect();
    assert!(texts.iter().any(|t| t == "Find All"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t.starts_with("Search \"cat\"")),
        "{texts:?}"
    );
    assert!(
        rep.components
            .iter()
            .filter(|c| c.kind == "tree-row")
            .count()
            >= 4,
        "rows drawn"
    );
}
