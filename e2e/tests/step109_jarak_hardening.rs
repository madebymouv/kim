//! E2E — Step 109 (#128): hostile and large input through the real `jarak`
//! (docs/JARAK_SECURITY.md, docs/JARAK_PERFORMANCE.md). Requirements: `JARAK-SEC-001`,
//! `JARAK-SEC-003`, `JARAK-SEC-004`, `JARAK-SEC-005`, `JARAK-SEC-007`, `JARAK-NFR-004`.
//!
//! 1. Random bytes and malformed UTF-8: `info` answers (binary / not lossless) and the files stay
//!    byte for byte.
//! 2. A note with a decompression-bomb PNG, a traversal image, raw `<script>` and a `javascript:`
//!    link: `export` to HTML refuses both images, keeps the script as text and drops the link;
//!    `print --dry-run` of 500 nested quotes lays out pages; the note is unchanged.
//! 3. An extension whose command is a script is refused (`extension_invalid`).
//! 4. A 10 MB file with a single line: `find --count` and `convert --dry-run` answer quickly, and
//!    `settings set large_file_mb 5` is taken.
//!
//! Machine safety: no window opens, nothing is printed (dry runs only), nothing is downloaded;
//! everything happens in a temporary folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

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

#[test]
#[ignore = "e2e: run by `cargo xtask e2e` (KIM_E2E=1)"]
fn step109_hostile_and_large_input() {
    if std::env::var_os("KIM_E2E").is_none() {
        return;
    }
    let tmp = TempDir::new("step109");
    let dir = tmp.path();
    let s = |p: &Path| p.to_string_lossy().into_owned();

    // 1. Random bytes, malformed UTF-8.
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let random: Vec<u8> = (0..4096)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect();
    let bin_file = dir.join("random.bin");
    std::fs::write(&bin_file, &random).unwrap();
    let (code, v) = jarak(dir, &["info", &s(&bin_file)]);
    assert_eq!(code, 0, "{v}");
    assert!(
        v["data"]["binary"] == true || v["data"]["lossless"] == false,
        "{v}"
    );
    let bad = dir.join("bad.txt");
    std::fs::write(&bad, b"ok \xC3\x28 \xF0\x28\x8C\xBC end\n").unwrap();
    let (code, v) = jarak(dir, &["info", &s(&bad)]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(std::fs::read(&bin_file).unwrap(), random, "unchanged");

    // 2. A hostile note.
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let mut bomb = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bomb.extend(100_000u32.to_be_bytes());
    bomb.extend(100_000u32.to_be_bytes());
    bomb.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    std::fs::write(dir.join("assets/bomb.png"), &bomb).unwrap();
    let note = dir.join("note.md");
    let text = format!(
        "# Note\n\n![bomb](assets/bomb.png) ![up](../../Windows/win.ini)\n\n<script>alert(1)</script>\n\n[x](javascript:alert(1))\n\n{}deep\n",
        "> ".repeat(500)
    );
    std::fs::write(&note, &text).unwrap();
    let out = dir.join("note.html");
    let (code, v) = jarak(dir, &["export", &s(&note), "--output", &s(&out)]);
    assert_eq!(code, 0, "{v}");
    let uses: Vec<&str> = v["data"]["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["use"].as_str().unwrap())
        .collect();
    // The bomb is embedded as bytes (never decoded by an export) only if it is small; the
    // traversal is refused.
    assert_eq!(uses[1], "refused", "{v}");
    let html = std::fs::read_to_string(&out).unwrap();
    let body = &html[html.find("<body").unwrap()..];
    assert!(!body.contains("<script>") && body.contains("&lt;script&gt;"));
    assert!(!body.contains("javascript:"));
    let (code, v) = jarak(
        dir,
        &[
            "print",
            &s(&note),
            "--printer",
            "Microsoft Print to PDF",
            "--output",
            &s(&dir.join("x.pdf")),
            "--dry-run",
        ],
    );
    if code == 0 {
        assert!(v["data"]["pages"].as_u64().unwrap() >= 1, "{v}");
    } else {
        assert_eq!(v["error"]["code"], "printer_not_found", "{v}");
    }
    assert_eq!(std::fs::read_to_string(&note).unwrap(), text, "unchanged");

    // 3. A script as an extension command.
    let ext = dir.join("evil");
    std::fs::create_dir_all(&ext).unwrap();
    std::fs::write(
        ext.join("extension.json"),
        r#"{"schema":"jarak.extension","schema_version":1,"id":"evil","name":"Evil","version":"1",
            "contributes":{"commands":[{"id":"run","title":"Run","program":"..\\..\\run.cmd"}]},
            "permissions":["run-process"]}"#,
    )
    .unwrap();
    let (code, v) = jarak(dir, &["extensions", "check", &s(&ext)]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (4, Some("extension_invalid"))
    );

    // 4. A 10 MB single line.
    let big = dir.join("one-line.txt");
    std::fs::write(&big, "word 한글 ".repeat(10 * 1024 * 1024 / 12)).unwrap();
    let t = Instant::now();
    let (code, v) = jarak(dir, &["find", &s(&big), "한글", "--count"]);
    let ms = t.elapsed().as_millis();
    assert_eq!(code, 0, "{v}");
    assert!(ms < 10_000, "find took {ms} ms");
    let (code, v) = jarak(dir, &["convert", &s(&big), "--eol", "lf", "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    let (code, v) = jarak(dir, &["settings", "set", "large_file_mb", "5"]);
    assert_eq!(code, 0, "{v}");
    let (_, v) = jarak(dir, &["settings", "get", "large_file_mb"]);
    assert_eq!(v["data"]["value"], "5");
}
