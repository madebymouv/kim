//! E2E — Step 85 (#104): Jarak's foundation (ADR 0102). Requirements: `JARAK-FR-001`,
//! `JARAK-FR-003`, `JARAK-SEC-009`, `KIM-FR-012`, `KIM-SEC-001`.
//!
//! The release `jarak.exe` and `jarak-gui.exe`, packaged as `cargo xtask package jarak` lays them
//! out, offered by a test-signed catalog over loopback to the debug `kim`: a user installs Jarak
//! explicitly; Kim reads its capabilities from the manifest; the installed `jarak` answers
//! `--version`, `version --json` and `capabilities --json` (the contract's document, `open` the
//! only command that opens a window), reports its per-user paths in the sandbox, and `open
//! --dry-run` resolves `file:line:column` targets without starting the window; `--json` usage
//! errors are one JSON document with exit 2; Kim uninstalls it. No window opens.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, bin_debug};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const SEED: [u8; 32] = [85u8; 32];
const NO_WINDOW: u32 = 0x0800_0000;

fn serve(files: Vec<(&'static str, Vec<u8>)>) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
            if let Some((_, body)) = files.iter().find(|(p, _)| *p == path) {
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(body);
            } else {
                let _ = s.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        }
    });
    port
}

/// The release layout of Jarak: both programs, the capabilities document and the licenses.
fn jarak_package(version: &str) -> Vec<u8> {
    let exe = bin("jarak");
    let caps = Command::new(&exe)
        .args(["capabilities", "--json"])
        .creation_flags(NO_WINDOW)
        .output()
        .unwrap();
    let mut doc: Value = serde_json::from_slice(&caps.stdout).unwrap();
    doc["data"]["version"] = json!(version);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("jarak.exe", std::fs::read(&exe).unwrap()),
        ("jarak-gui.exe", std::fs::read(bin("jarak-gui")).unwrap()),
        (
            "capabilities.json",
            serde_json::to_vec_pretty(&doc["data"]).unwrap(),
        ),
        (
            "LICENSE-MIT",
            std::fs::read(root.join("LICENSE-MIT")).unwrap(),
        ),
        (
            "LICENSE-APACHE",
            std::fs::read(root.join("LICENSE-APACHE")).unwrap(),
        ),
    ] {
        z.start_file(name, o).unwrap();
        z.write_all(&bytes).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn publish(sandbox: &Path, url: &str, body: &[u8]) {
    let release = json!({"version": "0.0.1", "channel": "stable", "published_at": "2026-11-22T00:00:00Z",
        "release_notes": "e2e", "assets": [{"os": "windows", "arch": "x86_64", "url": url, "size": body.len(),
        "sha256": kim::catalog::signature::hex(&Sha256::digest(body)), "kind": "zip", "entrypoint": "jarak.exe"}]});
    let doc = json!({"schema": "kim.catalog", "schema_version": 1, "catalog_version": 100,
        "generated_at": "2026-11-22T00:00:00Z", "expires_at": "2099-01-01T00:00:00Z", "publisher": "e2e",
        "tools": [{"id": "jarak", "name": "Jarak", "summary": "s", "description": "d", "license": "MIT OR Apache-2.0",
                   "support": "stable", "releases": [release]}]});
    let bytes = serde_json::to_vec_pretty(&doc).unwrap();
    let sig = serde_json::to_vec(&kim::catalog::signature::sign(&bytes, &SEED)).unwrap();
    let dir = sandbox
        .join("Local")
        .join("mouv")
        .join("kim")
        .join("catalog");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("catalog.json"), bytes).unwrap();
    std::fs::write(dir.join("catalog.json.sig"), sig).unwrap();
}

fn run(exe: &Path, sandbox: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(exe)
        .args(args)
        .env("LOCALAPPDATA", sandbox.join("Local"))
        .env("APPDATA", sandbox.join("Roaming"))
        .env(
            "KIM_DEV_TRUSTED_KEY",
            kim::catalog::signature::public_key_hex(&SEED),
        )
        .creation_flags(NO_WINDOW)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn json_of(exe: &Path, sandbox: &Path, args: &[&str]) -> (i32, Value) {
    let (code, out) = run(exe, sandbox, args);
    let v: Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("{args:?}: not JSON ({e}): {out}"));
    (code, v)
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn kim_installs_jarak_whose_cli_is_headless_and_follows_the_contract() {
    let sb = TempDir::new("jarak85");
    let p = sb.path();
    std::fs::create_dir_all(p.join("Roaming")).unwrap();
    let kim = bin_debug("kim");
    let pkg = jarak_package("0.0.1");
    let port = serve(vec![("/j1.zip", pkg.clone())]);
    publish(p, &format!("http://127.0.0.1:{port}/j1.zip"), &pkg);

    let (code, v) = json_of(&kim, p, &["install", "jarak", "--json"]);
    assert_eq!(code, 0, "{v}");
    let jarak = Path::new(v["data"]["result"]["entrypoint"].as_str().unwrap()).to_path_buf();
    assert!(jarak.is_file());
    assert!(jarak.with_file_name("jarak-gui.exe").is_file());
    let (_, v) = json_of(&kim, p, &["tools", "describe", "jarak", "--json"]);
    assert_eq!(v["data"]["capabilities_source"], "manifest", "{v}");

    // Version and capabilities.
    let (code, out) = run(&jarak, p, &["--version"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("jarak "), "{out}");
    let (code, v) = json_of(&jarak, p, &["version", "--json"]);
    assert_eq!((code, v["ok"].as_bool()), (0, Some(true)), "{v}");
    assert_eq!(v["data"]["cli_contract"], 1);
    let (_, v) = json_of(&jarak, p, &["capabilities", "--json"]);
    assert_eq!(v["data"]["tool"], "jarak");
    for c in v["data"]["commands"].as_array().unwrap() {
        assert_eq!(c["opens_window"], c["name"] == "open", "{c}");
    }

    // Paths live in the sandbox; nothing is created by asking.
    let (_, v) = json_of(&jarak, p, &["paths", "--json"]);
    let settings = v["data"]["settings"].as_str().unwrap();
    assert!(
        settings.starts_with(p.join("Roaming").to_str().unwrap()),
        "{v}"
    );
    assert!(!p.join("Roaming").join("mouv").join("jarak").exists());

    // `open --dry-run` resolves targets and starts nothing.
    let note = p.join("ideas.md");
    std::fs::write(&note, "# Ideas\n").unwrap();
    let arg = format!("{}:2:5", note.display());
    let (code, v) = json_of(&jarak, p, &["open", &arg, "--dry-run", "--json"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["started"], false);
    let t = &v["data"]["targets"][0];
    assert_eq!(
        (t["line"].as_u64(), t["column"].as_u64()),
        (Some(2), Some(5))
    );
    assert_eq!(t["exists"], true);
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "# Ideas\n");

    // Usage errors are one JSON document with exit 2.
    for args in [
        vec!["nope", "--json"],
        vec!["--json"],
        vec!["version", "--dry-run", "--json"],
    ] {
        let (code, v) = json_of(&jarak, p, &args);
        assert_eq!(
            (code, v["error"]["code"].as_str()),
            (2, Some("usage")),
            "{args:?}: {v}"
        );
    }

    let (code, v) = json_of(&kim, p, &["uninstall", "jarak", "--yes", "--json"]);
    assert_eq!(code, 0, "{v}");
    assert!(!jarak.exists());
}
