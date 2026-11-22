//! E2E — Step 103 (#122): the character sets of the Encoding menu through the real `jarak`, and
//! Show Symbol and the lossy-conversion question in the real window
//! (docs/JARAK_ARCHITECTURE.md §25).
//! Requirements: `JARAK-FR-025`, `JARAK-FR-026`, `JARAK-FR-037`.
//!
//! 1. For every character set (except ISO-2022-JP, whose 7-bit bytes are valid UTF-8 and so are
//!    read as UTF-8 unless reinterpreted): a sample written in it is read by `jarak info
//!    --fallback <label>` as that encoding, losslessly; the file stays byte for byte.
//! 2. `symbols`: the Rust sample with every symbol and an edge line — the audit finds no failure.
//! 3. `encoding-question`: converting the sample (with Korean) to Windows-1252 asks first and names
//!    what it cannot hold.
//!
//! Machine safety: `jarak` runs without a window; the windows open for about a second each at the
//! audit position; no input is synthesized; everything happens in a temporary folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn info(file: &Path, fallback: &str) -> (i32, Value) {
    let out = Command::new(bin("jarak"))
        .arg("info")
        .arg(file)
        .args(["--fallback", fallback, "--json"])
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{}: not JSON ({e})", file.display()));
    (out.status.code().unwrap_or(-1), v)
}

fn sample(label: &str) -> &'static str {
    match label {
        "euc-kr" => "한글 텍스트, 漢字 ABC",
        "shift_jis" | "euc-jp" | "iso-2022-jp" => "日本語のテキスト、カタカナ",
        "gbk" | "gb18030" => "中文文本 简体",
        "big5" => "中文繁體",
        "windows-1251" | "iso-8859-5" | "ibm866" | "koi8-r" => "Привет, мир!",
        "koi8-u" => "Привіт, світе!",
        "windows-1252" => "Café, déjà vu",
        "iso-8859-15" => "Café € œ",
        "windows-1250" | "iso-8859-2" => "Žluťoučký kůň",
        "windows-1253" | "iso-8859-7" => "Καλημέρα κόσμε",
        "windows-1254" => "Türkçe ğüşiöç",
        "windows-1255" | "iso-8859-8" => "שלום עולם",
        "windows-1256" | "iso-8859-6" => "مرحبا بالعالم",
        "windows-1257" | "iso-8859-4" | "iso-8859-13" => "Ąčęėįšųūž",
        "windows-1258" => "Viêt Đa ơ ư",
        "windows-874" => "ภาษาไทย",
        _ => panic!("no sample for {label}"),
    }
}

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step103-{}", std::process::id()),
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

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step103_character_sets_symbols_and_the_lossy_question() {
    let sb = TempDir::new("e2e-step103");
    let dir = sb.path().join("run");
    let files = dir.join("files");
    std::fs::create_dir_all(&files).unwrap();
    for (_, _, name, label) in jarak_core::file::CHARSETS {
        if *label == "iso-2022-jp" {
            continue;
        }
        let enc = jarak_core::file::encoding_for_label(label).unwrap();
        let bytes = jarak_core::file::encode(
            &format!("{}\r\n", sample(label)),
            jarak_core::file::Format {
                encoding: enc,
                bom: false,
            },
        )
        .unwrap();
        let p = files.join(format!("{label}.txt"));
        std::fs::write(&p, &bytes).unwrap();
        let (code, v) = info(&p, label);
        assert_eq!(code, 0, "{name}: {v}");
        assert_eq!(v["data"]["encoding"], enc.name(), "{name}: {v}");
        assert_eq!(v["data"]["lossless"], true, "{name}: {v}");
        assert_eq!(std::fs::read(&p).unwrap(), bytes, "{name}: unchanged");
    }
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let doc = docs.join("notes.md");
    let text = "# 자락\n";
    std::fs::write(&doc, text).unwrap();
    window(&dir, &doc, "symbols");
    let r = window(&dir, &doc, "encoding-question");
    let q: Vec<String> = r
        .components
        .iter()
        .filter(|c| c.id == "encoding:question")
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect();
    assert!(
        q.iter().any(|s| s.contains("cannot be written in")),
        "{q:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        text,
        "the document is untouched"
    );
}
