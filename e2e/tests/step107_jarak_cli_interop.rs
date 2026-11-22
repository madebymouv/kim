//! E2E — Step 107 (#126): Jarak's headless commands and Kim interoperability
//! (docs/JARAK_CLI.md, docs/INTEROP.md). Requirements: `JARAK-FR-150`, `JARAK-FR-151`,
//! `KIM-FR-012`, `KIM-FR-020`, `KIM-SEC-001`.
//!
//! 1. The real `jarak`: `new` (encoding, line endings, text; never replacing a file; a character
//!    the encoding cannot hold refused), `convert` (line endings and encoding, a dry run first),
//!    `session list` (no session yet), `assets add` (a "screenshot" PNG copied into the document's
//!    assets folder and referenced; the document saved in its own line endings), `debug-info`,
//!    `open --dry-run` with `--language` and `--read-only` (nothing started).
//! 2. Kim: the release `jarak.exe` packaged with its capabilities and offered by a test-signed
//!    catalog over loopback to the debug `kim`; a user installs Jarak explicitly; Kim finds it as
//!    the provider of `document.info`, `text.find`, `document.export`, `markdown.insert-image` and
//!    `document.open`; `kim tools invoke document.info` answers; `markdown.insert-image` needs the
//!    user's consent (`confirmation_required`) and with `--allow-write` puts the image into the
//!    note; `document.open` needs `--allow-window` even for a dry run (asked only with `--dry-run`
//!    here: nothing opens). Kim uninstalls it.
//!
//! Machine safety: no window opens (only dry runs of `open`), nothing is printed, no input is
//! synthesized; everything happens in a temporary folder and on loopback.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, bin_debug};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const SEED: [u8; 32] = [107u8; 32];

/// A 1×1 PNG header (Jarak checks images by their first bytes).
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0,
    1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89, 0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE,
    0x42, 0x60, 0x82,
];

fn jarak(dir: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(bin("jarak"))
        .args(args)
        .arg("--json")
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    (out.status.code().unwrap_or(-1), v)
}

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

/// The release `jarak.exe` with its capabilities document, as `cargo xtask package` lays it out.
fn package(version: &str) -> Vec<u8> {
    let exe = bin("jarak");
    let caps = Command::new(&exe)
        .args(["capabilities", "--json"])
        .creation_flags(0x0800_0000)
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
    let doc = json!({"schema": "kim.catalog", "schema_version": 1, "catalog_version": 100,
        "generated_at": "2026-11-22T00:00:00Z", "expires_at": "2099-01-01T00:00:00Z", "publisher": "e2e",
        "tools": [{"id": "jarak", "name": "Jarak", "summary": "s", "description": "d", "license": "MIT OR Apache-2.0",
            "support": "stable", "releases": [{"version": "0.0.1", "channel": "stable", "published_at": "2026-11-22T00:00:00Z",
                "release_notes": "e2e", "assets": [{"os": "windows", "arch": "x86_64", "url": url, "size": body.len(),
                "sha256": kim::catalog::signature::hex(&Sha256::digest(body)), "kind": "zip", "entrypoint": "jarak.exe"}]}]}]});
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

fn kim(sandbox: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(bin_debug("kim"))
        .args(args)
        .env("LOCALAPPDATA", sandbox.join("Local"))
        .env("APPDATA", sandbox.join("Roaming"))
        .env(
            "KIM_DEV_TRUSTED_KEY",
            kim::catalog::signature::public_key_hex(&SEED),
        )
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
    )
}

#[test]
#[ignore = "e2e: run by `cargo xtask e2e` (KIM_E2E=1)"]
fn step107_headless_commands_and_kim_interop() {
    if std::env::var_os("KIM_E2E").is_none() {
        return;
    }
    let tmp = TempDir::new("step107");
    let dir = tmp.path();
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    let s = |p: &Path| p.to_string_lossy().into_owned();

    // 1. new, convert, session list, assets add, debug-info, open --dry-run.
    let txt = dir.join("made.txt");
    let (code, v) = jarak(
        dir,
        &[
            "new",
            &s(&txt),
            "--encoding",
            "utf-8-bom",
            "--eol",
            "lf",
            "--text",
            "첫 줄\n둘째 줄",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        std::fs::read(&txt).unwrap(),
        "\u{feff}첫 줄\n둘째 줄".as_bytes()
    );
    let (code, v) = jarak(dir, &["new", &s(&txt)]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("file_exists"))
    );
    let (code, v) = jarak(
        dir,
        &[
            "new",
            &s(&dir.join("ansi.txt")),
            "--encoding",
            "windows-1252",
            "--text",
            "한",
        ],
    );
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("text_unencodable")),
        "{v}"
    );
    let (code, v) = jarak(dir, &["convert", &s(&txt), "--eol", "crlf", "--dry-run"]);
    assert_eq!((code, v["data"]["written"].as_bool()), (0, Some(false)));
    let (code, v) = jarak(
        dir,
        &[
            "convert",
            &s(&txt),
            "--eol",
            "crlf",
            "--encoding",
            "utf-16le",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert!(std::fs::read(&txt).unwrap().starts_with(b"\xFF\xFE"));
    let (code, v) = jarak(dir, &["session", "list"]);
    assert_eq!((code, v["data"]["exists"].as_bool()), (0, Some(false)));

    let note = dir.join("note.md");
    std::fs::write(&note, "# Note\r\n\r\nText.\r\n").unwrap();
    let shot = dir.join("capture.png");
    std::fs::write(&shot, PNG).unwrap();
    let (code, v) = jarak(
        dir,
        &["assets", "add", &s(&note), &s(&shot), "--alt", "capture"],
    );
    assert_eq!(code, 0, "{v}");
    let reference = v["data"]["reference"].as_str().unwrap().to_string();
    assert!(reference.starts_with("assets/note-"), "{v}");
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        format!("# Note\r\n\r\nText.\r\n![capture]({reference})\r\n")
    );
    assert_eq!(std::fs::read(dir.join(&reference)).unwrap(), PNG);
    let (code, v) = jarak(dir, &["debug-info"]);
    assert_eq!((code, v["data"]["os"].as_str()), (0, Some("windows")));
    let (code, v) = jarak(
        dir,
        &[
            "open",
            &format!("{}:3", s(&note)),
            "--language",
            "Markdown",
            "--read-only",
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["started"], false);
    assert_eq!(v["data"]["read_only"], true);
    assert_eq!(v["data"]["targets"][0]["line"], 3);

    // 2. Kim finds and invokes Jarak's capabilities.
    let pkg = package("0.0.1");
    let port = serve(vec![("/j1.zip", pkg.clone())]);
    publish(dir, &format!("http://127.0.0.1:{port}/j1.zip"), &pkg);
    let (code, v) = kim(dir, &["install", "jarak", "--json"]);
    assert_eq!(code, 0, "{v}");
    for cap in [
        "document.info",
        "text.find",
        "document.export",
        "markdown.insert-image",
        "document.open",
    ] {
        let (code, v) = kim(dir, &["tools", "find", cap, "--json"]);
        assert_eq!(
            (code, v["data"]["providers"][0]["tool"].as_str()),
            (0, Some("jarak")),
            "{cap}: {v}"
        );
    }
    let params = json!({ "path": s(&note) }).to_string();
    let (code, v) = kim(
        dir,
        &[
            "tools",
            "invoke",
            "document.info",
            "--params",
            &params,
            "--json",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["result"]["eol"], "crlf", "{v}");
    let insert = json!({ "document": s(&note), "image": s(&shot), "alt": "second" }).to_string();
    let (code, v) = kim(
        dir,
        &[
            "tools",
            "invoke",
            "markdown.insert-image",
            "--params",
            &insert,
            "--json",
        ],
    );
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("confirmation_required")),
        "{v}"
    );
    let (code, v) = kim(
        dir,
        &[
            "tools",
            "invoke",
            "markdown.insert-image",
            "--allow-write",
            "--params",
            &insert,
            "--json",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert!(
        std::fs::read_to_string(&note)
            .unwrap()
            .contains("![second](assets/note-")
    );
    let open = json!({ "path": s(&note), "line": 2 }).to_string();
    let (code, v) = kim(
        dir,
        &[
            "tools",
            "invoke",
            "document.open",
            "--params",
            &open,
            "--json",
        ],
    );
    assert_ne!(code, 0, "a window needs --allow-window: {v}");
    let (code, v) = kim(
        dir,
        &[
            "tools",
            "invoke",
            "document.open",
            "--allow-window",
            "--params",
            &open,
            "--dry-run",
            "--json",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["data"]["result"]["started"], false,
        "a dry run starts nothing: {v}"
    );
    let (code, v) = kim(dir, &["uninstall", "jarak", "--yes", "--json"]);
    assert_eq!(code, 0, "{v}");
}
