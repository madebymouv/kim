//! E2E — Step 106 (#125): exports through the real `jarak export` (docs/JARAK_EXPORT.md).
//! Requirements: `JARAK-FR-141`, `JARAK-SEC-001`, `JARAK-SEC-003`, `JARAK-SEC-004`.
//!
//! 1. A Markdown note with a local image, a missing one, a remote one and raw HTML: `jarak export
//!    note.md --output out/note.html` embeds the image (a `data:` URI), loads nothing remote, keeps
//!    the raw HTML as text and reports each image; the note's bytes are unchanged.
//! 2. The same again is refused (exit 7, `export_exists`, the file untouched); `--overwrite`
//!    replaces it; the document itself as the output is refused (exit 2, `export_into_source`).
//! 3. `--linked` refers to the image relative to the output; `--to folder` publishes the page and a
//!    copy of the image (`--dry-run` first writes nothing).
//! 4. A Rust file as RTF and as highlighted HTML with line numbers.
//!
//! Machine safety: `jarak` runs without a window and without the network; everything happens in a
//! temporary folder.

#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

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

/// A 1×1 PNG.
const DOT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0,
    1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89, 0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE,
    0x42, 0x60, 0x82,
];

#[test]
#[ignore = "e2e: run by `cargo xtask e2e` (KIM_E2E=1)"]
fn step106_exports_through_the_cli() {
    if std::env::var_os("KIM_E2E").is_none() {
        return;
    }
    let tmp = TempDir::new("step106");
    let dir = tmp.path();
    let doc_dir = dir.join("doc");
    std::fs::create_dir_all(doc_dir.join("assets")).unwrap();
    std::fs::write(doc_dir.join("assets/dot.png"), DOT_PNG).unwrap();
    let note = doc_dir.join("note.md");
    let text = "# 노트\n\n![dot](assets/dot.png) ![gone](assets/gone.png) ![r](https://example.com/r.png)\n\n<script>alert(1)</script>\n";
    std::fs::write(&note, text).unwrap();
    let n = note.to_str().unwrap();
    let out = dir.join("out/note.html");
    let o = out.to_str().unwrap();

    // 1. Rendered, the image embedded, nothing remote.
    let (code, v) = jarak(dir, &["export", n, "--output", o]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["rendered"], true);
    let uses: Vec<&str> = v["data"]["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["use"].as_str().unwrap())
        .collect();
    assert_eq!(uses, ["embedded", "missing", "remote"]);
    let html = std::fs::read_to_string(&out).unwrap();
    assert!(html.contains("src=\"data:image/png;base64,"));
    assert!(html.contains("<h1 id=\"노트\">노트</h1>"));
    assert!(html.contains("&lt;script&gt;") && !html.contains("<script>"));
    assert!(!html.contains("src=\"http"), "nothing remote is loaded");
    assert!(html.contains("Content-Security-Policy"));

    // 2. Never replaced without --overwrite; never the document.
    std::fs::write(&out, "kept").unwrap();
    let (code, v) = jarak(dir, &["export", n, "--output", o]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("export_exists")),
        "{v}"
    );
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "kept");
    let (code, _) = jarak(dir, &["export", n, "--output", o, "--overwrite"]);
    assert_eq!(code, 0);
    assert!(std::fs::read_to_string(&out).unwrap().contains("<h1"));
    let (code, v) = jarak(
        dir,
        &["export", n, "--to", "html", "--output", n, "--overwrite"],
    );
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (2, Some("export_into_source")),
        "{v}"
    );

    // 3. Linked, and published to a folder.
    let linked = dir.join("out/linked.html");
    let (code, _) = jarak(
        dir,
        &[
            "export",
            n,
            "--output",
            linked.to_str().unwrap(),
            "--linked",
        ],
    );
    assert_eq!(code, 0);
    assert!(
        std::fs::read_to_string(&linked)
            .unwrap()
            .contains("src=\"../doc/assets/dot.png\"")
    );
    let site = dir.join("site");
    let s = site.to_str().unwrap();
    let (code, v) = jarak(
        dir,
        &["export", n, "--to", "folder", "--output", s, "--dry-run"],
    );
    assert_eq!((code, v["data"]["written"].as_bool()), (0, Some(false)));
    assert!(!site.exists(), "a dry run writes nothing");
    let (code, v) = jarak(dir, &["export", n, "--to", "folder", "--output", s]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(std::fs::read(site.join("assets/dot.png")).unwrap(), DOT_PNG);
    assert!(
        std::fs::read_to_string(site.join("note.html"))
            .unwrap()
            .contains("src=\"assets/dot.png\"")
    );
    assert_eq!(std::fs::read_to_string(&note).unwrap(), text, "unchanged");

    // 4. Code as RTF and highlighted HTML.
    let rs = dir.join("a.rs");
    std::fs::write(&rs, "fn main() {\n    let s = \"한글\";\n}\n").unwrap();
    let r = rs.to_str().unwrap();
    let rtf = dir.join("a.rtf");
    let (code, v) = jarak(dir, &["export", r, "--output", rtf.to_str().unwrap()]);
    assert_eq!((code, v["data"]["language"].as_str()), (0, Some("Rust")));
    let body = std::fs::read_to_string(&rtf).unwrap();
    assert!(
        body.starts_with("{\\rtf1") && body.contains("\\u-10916?"),
        "{body}"
    );
    let code_html = dir.join("a.html");
    let (code, _) = jarak(
        dir,
        &[
            "export",
            r,
            "--output",
            code_html.to_str().unwrap(),
            "--line-numbers",
        ],
    );
    assert_eq!(code, 0);
    let body = std::fs::read_to_string(&code_html).unwrap();
    assert!(body.contains("<span class=\"t-keyword\">fn</span>"));
    assert!(body.contains("<span class=\"ln\">1 </span>"));
}
