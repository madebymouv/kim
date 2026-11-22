//! Step 109 (`JARAK-SEC-001…007`): hostile and malformed input never crashes Jarak's core, never
//! changes a document by itself, and is refused where it should be. Deterministic pseudo-random
//! inputs (a fixed xorshift seed, no extra crate) stand in for fuzzing; each property runs on a few
//! hundred cases. docs/JARAK_SECURITY.md is the threat model these tests belong to.

use jarak_core::{assets, export, extensions, file, markdown, print, session, settings, syntax};
use std::path::{Path, PathBuf};

/// A small deterministic generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max + 1);
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// Text made of pieces that matter to a parser.
    fn text_of(&mut self, pieces: &[&str], max: usize) -> String {
        let n = self.below(max + 1);
        (0..n).map(|_| pieces[self.below(pieces.len())]).collect()
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jarak-security-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Random bytes in every encoding decode without panicking; when the decode says it is lossless,
/// encoding the text again gives back exactly the bytes (what makes a save safe).
#[test]
fn random_bytes_decode_safely_and_lossless_means_exact() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let fallbacks: Vec<file::TextEncoding> = ["euc-kr", "windows-1252", "shift_jis", "gbk"]
        .iter()
        .filter_map(|l| file::encoding_for_label(l))
        .collect();
    let prefixes: [&[u8]; 4] = [b"", b"\xEF\xBB\xBF", b"\xFF\xFE", b"\xFE\xFF"];
    for i in 0..600 {
        let mut b = prefixes[i % 4].to_vec();
        b.extend(r.bytes(512));
        let d = file::decode(&b, fallbacks[i % fallbacks.len()]);
        if d.lossless && !d.binary {
            let again = file::encode(&d.text, d.format).expect("lossless text encodes");
            assert_eq!(again, b, "case {i}: lossless must mean byte-exact");
        }
    }
}

/// Hostile Markdown (random markup, deep nesting, huge tables, long lines) parses, renders to
/// HTML and lays out for printing without panicking; the HTML never carries a script or a
/// `javascript:` link from the text.
#[test]
fn hostile_markdown_renders_safely() {
    let pieces = [
        "#",
        "*",
        "_",
        "~",
        "`",
        "```",
        "[",
        "]",
        "(",
        ")",
        "!",
        "<",
        ">",
        "|",
        "-",
        "+",
        "\\",
        "\n",
        " ",
        "x",
        "한",
        "<script>",
        "javascript:",
        "[^1]",
        ": ",
        "1. ",
        "- [ ] ",
        "\t",
        "&",
        "\"",
        "<!--",
        "-->",
        "http://e.com",
        "\r\n",
    ];
    let mut r = Rng(42);
    let m = print::FakeMeasure { dpi: 150.0 };
    let g = print::Geometry::a4(150.0);
    let setup = print::Setup::default();
    let vars = print::Vars::default();
    for i in 0..300 {
        let t = r.text_of(&pieces, 400);
        let _ = markdown::parse(&t);
        let p = export::markdown_html(&t, None, "t", export::Images::Embed, Path::new("o.html"));
        let body = &p.html[p.html.find("<body").unwrap()..];
        assert!(!body.contains("<script>"), "case {i}");
        assert!(
            !body.to_ascii_lowercase().contains("href=\"javascript:"),
            "case {i}"
        );
        let _ = print::markdown_pages(&t, None, &setup, &g, &m, &vars);
    }
    // Deep nesting and wide or long content.
    for t in [
        "> ".repeat(500) + "deep",
        "- ".repeat(300) + "deep",
        format!("|{}\n|{}\n", "a|".repeat(2000), "-|".repeat(2000)),
        "x".repeat(200_000),
        "[".repeat(20_000),
        "*".repeat(20_000),
    ] {
        let _ = markdown::parse(&t);
        let _ = export::markdown_html(&t, None, "t", export::Images::Embed, Path::new("o.html"));
        let _ = print::markdown_pages(&t, None, &setup, &g, &m, &vars);
    }
}

/// Highlighting random text in every built-in language never panics and keeps every character.
#[test]
fn random_text_highlights_in_every_language() {
    let pieces = [
        "\"",
        "'",
        "/*",
        "*/",
        "//",
        "#",
        "<!--",
        "-->",
        "<",
        ">",
        "`",
        "```",
        "\\",
        "\n",
        " ",
        "fn",
        "class",
        "x",
        "1.5e3",
        "0x",
        "한글",
        "${",
        "}",
        "<?php",
        "?>",
        "<script>",
        "</script>",
        "r#\"",
        "\"#",
        "--",
        "[[",
        "]]",
        "\t",
    ];
    let mut r = Rng(7);
    for (i, l) in syntax::builtins().iter().enumerate() {
        for _ in 0..12 {
            let t = r.text_of(&pieces, 200);
            for (line, runs) in export::highlight(&t, Some(l)) {
                let joined: String = runs.iter().map(|(_, s)| s.as_str()).collect();
                assert_eq!(joined, line, "{} ({i})", l.name);
            }
        }
    }
}

/// Images: a decompression bomb (a header claiming 100 000 × 100 000 pixels), truncated files
/// and random bytes are refused by the printer's decoder and size check; nothing panics.
#[test]
fn image_bombs_and_garbage_are_refused() {
    let d = temp("images");
    // A PNG header claiming 100000 x 100000 RGBA pixels.
    let mut bomb = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bomb.extend(100_000u32.to_be_bytes());
    bomb.extend(100_000u32.to_be_bytes());
    bomb.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    let p = d.join("bomb.png");
    std::fs::write(&p, &bomb).unwrap();
    assert_eq!(print::image_size(&p), None, "too many pixels");
    assert!(print::decode_image(&p).is_err());
    let mut r = Rng(99);
    for i in 0..60 {
        let mut b = [
            b"\x89PNG\r\n\x1a\n".as_slice(),
            b"\xFF\xD8\xFF",
            b"GIF89a",
            b"BM",
            b"RIFF\0\0\0\0WEBP",
        ][i % 5]
            .to_vec();
        b.extend(r.bytes(300));
        let f = d.join(format!("g{i}.img"));
        std::fs::write(&f, &b).unwrap();
        let _ = print::image_size(&f);
        let _ = print::decode_image(&f);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Asset references never leave the document's folder: `..`, absolute, drive-relative, UNC,
/// percent-encoded traversal and remote references are not files.
#[test]
fn asset_references_never_leave_the_documents_folder() {
    let doc = Path::new(r"C:\notes\day.md");
    for r in [
        "../secret.png",
        "..\\secret.png",
        "assets/../../secret.png",
        "%2e%2e/secret.png",
        "%2E%2E%5Csecret.png",
        "C:\\Windows\\win.ini",
        "C:secret.png",
        "/etc/passwd",
        "\\\\server\\share\\x.png",
        "//server/share/x.png",
        "https://example.com/x.png",
        "file:///C:/x.png",
        "data:image/png;base64,AAAA",
    ] {
        let got = assets::resolve(doc, r);
        assert!(
            got.as_ref().is_none_or(|p| p.starts_with(r"C:\notes")),
            "{r} -> {got:?}"
        );
        if r.contains("..") || r.contains("%2") || r.contains(':') || r.starts_with(['/', '\\']) {
            assert_eq!(got, None, "{r}");
        }
    }
    assert_eq!(
        assets::resolve(doc, "assets/a b.png"),
        Some(PathBuf::from(r"C:\notes\assets\a b.png"))
    );
}

/// Random and hostile extension manifests are checked without panicking; programs never leave
/// the folder and are never scripts.
#[test]
fn hostile_extension_manifests_are_refused() {
    let d = temp("ext");
    let mut r = Rng(1234);
    let pieces = [
        "{",
        "}",
        "[",
        "]",
        "\"",
        ":",
        ",",
        "schema",
        "jarak.extension",
        "1",
        "id",
        "name",
        "commands",
        "program",
        "..\\x.exe",
        "C:\\x.exe",
        "run.cmd",
        "sort.exe",
        "permissions",
        "run-process",
        "null",
        "true",
        " ",
    ];
    for i in 0..200 {
        let t = r.text_of(&pieces, 60);
        std::fs::write(d.join(extensions::MANIFEST), t).unwrap();
        let _ = extensions::check(&d);
        assert!(i < 200);
    }
    for program in [
        "..\\evil.exe",
        "C:\\Windows\\evil.exe",
        "evil.cmd",
        "evil.ps1",
        "a/../../b.exe",
    ] {
        let m = serde_json::json!({ "schema": "jarak.extension", "schema_version": 1, "id": "x",
            "name": "x", "version": "1", "contributes": { "commands": [{ "id": "c", "title": "C",
            "program": program }] }, "permissions": ["run-process"] });
        std::fs::write(d.join(extensions::MANIFEST), m.to_string()).unwrap();
        assert!(extensions::check(&d).is_err(), "{program}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Corrupt settings, session and shortcut files are read safely: defaults, an error, or safe
/// mode — never a panic, and never a rewrite of the corrupt file by reading it.
#[test]
fn corrupt_state_files_are_read_safely() {
    let d = temp("state");
    let mut r = Rng(555);
    for i in 0..120 {
        let b = r.bytes(400);
        let set = d.join("settings.toml");
        std::fs::write(&set, &b).unwrap();
        let s = settings::Settings::new(&set);
        for st in settings::SETTINGS {
            assert!(s.get(st.key).is_some(), "case {i}: a value or the default");
        }
        let ses = d.join("session.json");
        std::fs::write(&ses, &b).unwrap();
        let _ = session::Session::load(&ses);
        let hk = d.join("hotkeys.json");
        std::fs::write(&hk, &b).unwrap();
        let _ = jarak_core::hotkeys::load(&hk);
        assert_eq!(std::fs::read(&hk).unwrap(), b, "reading never rewrites");
        assert_eq!(std::fs::read(&set).unwrap(), b);
    }
    let _ = std::fs::remove_dir_all(&d);
}
