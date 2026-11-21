//! E2E — Step 87 (#106): Jarak reads files without changing them (docs/JARAK_ARCHITECTURE.md §9).
//! Requirements: `JARAK-FR-021`, `JARAK-FR-022`, `JARAK-FR-023`, `JARAK-FR-027`,
//! `JARAK-SEC-001`, `JARAK-SEC-009`.
//!
//! The release `jarak.exe` inspects a corpus written into a temporary folder: UTF-8 with and
//! without a byte-order mark, UTF-16 LE with a BOM, UTF-16 LE without one, EUC-KR read with
//! `--fallback euc-kr`, mixed line endings without a final newline, a binary file and invalid
//! UTF-8 read as UTF-8; each answer names the encoding, BOM and line endings, and whether a save
//! would give back the same bytes (lossless) or the window opens it read-only. Every file is
//! byte-identical afterwards; a missing file is `file_not_found` (exit 3), a folder a usage error.
//! No window opens.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

fn info(file: &Path, extra: &[&str]) -> (i32, Value) {
    let out = Command::new(bin("jarak"))
        .arg("info")
        .arg(file)
        .args(extra)
        .arg("--json")
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{}: not JSON ({e})", file.display()));
    (out.status.code().unwrap_or(-1), v)
}

fn utf16le(s: &str, bom: bool) -> Vec<u8> {
    let mut v = if bom { vec![0xFF, 0xFE] } else { vec![] };
    for u in s.encode_utf16() {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

/// One file of the corpus and what `jarak info` must say about it.
struct Case {
    name: &'static str,
    bytes: Vec<u8>,
    extra: Vec<&'static str>,
    encoding: &'static str,
    bom: bool,
    eol: &'static str,
    lossless: bool,
    read_only: bool,
}

impl Case {
    fn new(name: &'static str, bytes: Vec<u8>, encoding: &'static str, eol: &'static str) -> Case {
        Case {
            name,
            bytes,
            extra: Vec::new(),
            encoding,
            bom: false,
            eol,
            lossless: true,
            read_only: false,
        }
    }

    fn bom(mut self) -> Case {
        self.bom = true;
        self
    }

    fn with(mut self, extra: &[&'static str]) -> Case {
        self.extra = extra.to_vec();
        self
    }

    /// Opened read-only; `lossless` says whether the bytes would still come back unchanged.
    fn read_only(mut self, lossless: bool) -> Case {
        self.read_only = true;
        self.lossless = lossless;
        self
    }
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn jarak_info_names_encodings_and_line_endings_and_changes_nothing() {
    let dir = TempDir::new("jarak87");
    let d = dir.path();
    let euc_kr: Vec<u8> = vec![0xC7, 0xD1, 0xB1, 0xDB, 0x0D, 0x0A]; // "한글\r\n" in EUC-KR
    let cases = [
        Case::new("plain.txt", b"plain\nascii\n".to_vec(), "utf-8", "lf"),
        Case::new(
            "bom.md",
            b"\xEF\xBB\xBF# Title\r\n".to_vec(),
            "utf-8",
            "crlf",
        )
        .bom(),
        Case::new("wide.txt", utf16le("한글\r\n", true), "utf-16le", "crlf").bom(),
        Case::new(
            "nobom16.txt",
            utf16le("hello\r\nworld\r\n", false),
            "utf-16le",
            "crlf",
        ),
        Case::new("korean.txt", euc_kr, "EUC-KR", "crlf").with(&["--fallback", "euc-kr"]),
        Case::new("mixed.txt", b"a\r\nb\nc".to_vec(), "utf-8", "mixed"),
        Case::new(
            "blob.bin",
            b"PK\x03\x04\x00\x00\x00data".to_vec(),
            "utf-8",
            "none",
        )
        .read_only(true),
        Case::new("broken.txt", b"ok \xFF\xFE\n".to_vec(), "utf-8", "lf")
            .with(&["--fallback", "utf-8"])
            .read_only(false),
    ];
    for c in &cases {
        let p = d.join(c.name);
        std::fs::write(&p, &c.bytes).unwrap();
        let (code, v) = info(&p, &c.extra);
        let name = c.name;
        assert_eq!(code, 0, "{name}: {v}");
        let data = &v["data"];
        assert_eq!(data["encoding"], c.encoding, "{name}: {v}");
        assert_eq!(data["bom"], c.bom, "{name}");
        assert_eq!(data["eol"], c.eol, "{name}");
        assert_eq!(data["lossless"], c.lossless, "{name}");
        assert_eq!(data["read_only"], c.read_only, "{name}");
        assert_eq!(data["size"], c.bytes.len(), "{name}");
        assert_eq!(std::fs::read(&p).unwrap(), c.bytes, "{name} unchanged");
    }
    let (code, v) = info(&d.join("missing.txt"), &[]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (3, Some("file_not_found")),
        "{v}"
    );
    let (code, v) = info(d, &[]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (2, Some("usage")),
        "{v}"
    );
    let (code, v) = info(&d.join("plain.txt"), &["--fallback", "klingon"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (2, Some("usage")),
        "{v}"
    );
}
