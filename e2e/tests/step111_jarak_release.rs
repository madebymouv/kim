//! E2E — Step 111 (#130): Jarak delivered through Kim as the fourth Windows tool
//! (docs/JARAK_GUIDE.md, docs/JARAK_ACCEPTANCE.md). Requirements: `KIM-FR-020`, `KIM-SEC-001`,
//! `JARAK-FR-130`, `JARAK-FR-171`.
//!
//! 1. The release `jarak.exe` and `jarak-gui.exe` packed as `cargo xtask package jarak` lays them
//!    out (capabilities, licences, the user guide and its references under `docs\`), offered by a
//!    test-signed catalog over loopback to the debug `kim`.
//! 2. `kim install jarak --dry-run` writes nothing; `kim install jarak` installs it; the installed
//!    folder holds both programs, the guide and the licences; the installed `jarak` answers
//!    (`version`, `capabilities`: implemented, `associate list`: not offered).
//! 3. The Open with offer as dry runs only (`associate add md txt --dry-run` names the installed
//!    `jarak-gui.exe` and writes nothing; `associate add exe` refused, `association_invalid`;
//!    `associate remove` with nothing offered changes nothing).
//! 4. The other tools still answer (`kim`, `subak`, `noeul`, `ddalgak` `--version`).
//! 5. `kim uninstall jarak` removes the programs and keeps the user's settings.
//!
//! Machine safety: no window opens, the registry is never written (dry runs only), nothing is
//! downloaded beyond loopback; everything happens in a temporary folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, bin_debug};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const SEED: [u8; 32] = [111u8; 32];

/// The documents the package carries under `docs\` (the guide and the references it links).
const DOCS: &[&str] = &[
    "JARAK_GUIDE.md",
    "JARAK_KEYS.md",
    "JARAK_SETTINGS.md",
    "JARAK_CLI.md",
    "JARAK_VIM.md",
];

fn serve(files: Vec<(&'static str, Vec<u8>)>) -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let mut buf = [0u8; 4096];
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

/// The release programs as `cargo xtask package jarak` lays them out.
fn package() -> (Vec<u8>, String) {
    let exe = bin("jarak");
    let caps = Command::new(&exe)
        .args(["capabilities", "--json"])
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    let doc: Value = serde_json::from_slice(&caps.stdout).unwrap();
    let version = doc["data"]["version"].as_str().unwrap().to_string();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("jarak.exe".into(), std::fs::read(&exe).unwrap()),
        (
            "jarak-gui.exe".into(),
            std::fs::read(bin("jarak-gui")).unwrap(),
        ),
        (
            "capabilities.json".into(),
            serde_json::to_vec_pretty(&doc["data"]).unwrap(),
        ),
        (
            "LICENSE-MIT".into(),
            std::fs::read(root.join("LICENSE-MIT")).unwrap(),
        ),
        (
            "LICENSE-APACHE".into(),
            std::fs::read(root.join("LICENSE-APACHE")).unwrap(),
        ),
    ];
    for d in DOCS {
        files.push((
            format!("docs/{d}"),
            std::fs::read(root.join("docs").join(d)).unwrap(),
        ));
    }
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in files {
        z.start_file(name, o).unwrap();
        z.write_all(&bytes).unwrap();
    }
    (z.finish().unwrap().into_inner(), version)
}

fn publish(sandbox: &Path, url: &str, body: &[u8], version: &str) {
    let doc = json!({"schema": "kim.catalog", "schema_version": 1, "catalog_version": 111,
        "generated_at": "2026-11-22T00:00:00Z", "expires_at": "2099-01-01T00:00:00Z", "publisher": "e2e",
        "tools": [{"id": "jarak", "name": "Jarak", "summary": "s", "description": "d", "license": "MIT OR Apache-2.0",
            "support": "stable", "releases": [{"version": version, "channel": "stable", "published_at": "2026-11-22T00:00:00Z",
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

fn run(sandbox: &Path, exe: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(exe)
        .args(args)
        .arg("--json")
        .env("LOCALAPPDATA", sandbox.join("Local"))
        .env("APPDATA", sandbox.join("Roaming"))
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
    )
}

fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find(&p, name) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == name) {
            return Some(p);
        }
    }
    None
}

#[test]
#[ignore = "e2e: run by `cargo xtask e2e` (KIM_E2E=1)"]
fn step111_jarak_through_kim() {
    if std::env::var_os("KIM_E2E").is_none() {
        return;
    }
    let tmp = TempDir::new("step111");
    let dir = tmp.path();
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    let programs = dir.join("Local").join("Programs").join("mouv");

    // 1–2. Install from a signed catalog.
    let (pkg, version) = package();
    let port = serve(vec![("/jarak.zip", pkg.clone())]);
    publish(
        dir,
        &format!("http://127.0.0.1:{port}/jarak.zip"),
        &pkg,
        &version,
    );
    let (code, v) = kim(dir, &["install", "jarak", "--dry-run", "--json"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        find(&programs, "jarak.exe").is_none(),
        "a dry run installs nothing"
    );
    let (code, v) = kim(dir, &["install", "jarak", "--json"]);
    assert_eq!(code, 0, "{v}");
    let jarak = find(&programs, "jarak.exe").expect("installed");
    let home = jarak.parent().unwrap().to_path_buf();
    for f in [
        "jarak-gui.exe",
        "capabilities.json",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        assert!(home.join(f).is_file(), "{f}");
    }
    for d in DOCS {
        assert!(home.join("docs").join(d).is_file(), "docs/{d}");
    }
    let (code, v) = run(dir, &jarak, &["version"]);
    assert_eq!(
        (code, v["data"]["version"].as_str()),
        (0, Some(version.as_str()))
    );
    let (code, v) = run(dir, &jarak, &["capabilities"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["implementation"], "implemented");
    assert!(
        v["data"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "associate.add" && c["dry_run"] == true)
    );

    // 3. The Open with offer, dry runs only.
    let (code, v) = run(dir, &jarak, &["associate", "list"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["offered"], json!([]));
    let (code, v) = run(
        dir,
        &jarak,
        &["associate", "add", "md", ".TXT", "--dry-run"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["written"], false);
    assert_eq!(v["data"]["extensions"], json!(["md", "txt"]));
    assert_eq!(
        v["data"]["program"].as_str().map(PathBuf::from),
        Some(home.join("jarak-gui.exe")),
        "the installed window is what Open with would start"
    );
    assert!(v["data"]["changes"].as_array().unwrap().iter().any(|c| {
        c["key"]
            .as_str()
            .is_some_and(|k| k.ends_with(".md\\OpenWithProgids"))
    }));
    let (code, v) = run(dir, &jarak, &["associate", "list"]);
    assert_eq!(
        (code, &v["data"]["offered"]),
        (0, &json!([])),
        "still nothing"
    );
    let (code, v) = run(dir, &jarak, &["associate", "add", "exe"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (2, Some("association_invalid"))
    );
    let (code, v) = run(dir, &jarak, &["associate", "remove", "--dry-run"]);
    assert_eq!((code, &v["data"]["changes"]), (0, &json!([])));

    // 4. The other tools still answer.
    for t in ["subak", "noeul", "ddalgak"] {
        let out = Command::new(bin(t))
            .arg("--version")
            .creation_flags(0x0800_0000)
            .output()
            .unwrap();
        assert!(out.status.success(), "{t} --version");
    }
    let (code, _) = kim(dir, &["--version", "--json"]);
    assert_eq!(code, 0);

    // 5. Uninstall keeps the user's settings.
    let (code, _) = run(dir, &jarak, &["settings", "set", "word_wrap", "true"]);
    assert_eq!(code, 0);
    let settings = dir
        .join("Roaming")
        .join("mouv")
        .join("jarak")
        .join("settings.toml");
    assert!(settings.is_file());
    let (code, v) = kim(dir, &["uninstall", "jarak", "--yes", "--json"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        find(&programs, "jarak.exe").is_none(),
        "the programs are gone"
    );
    assert!(settings.is_file(), "the settings stay (no --purge-data)");
}
