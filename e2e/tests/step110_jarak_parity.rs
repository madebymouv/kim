//! E2E — Step 110 (#129): the remaining Notepad++ built-ins through the real `jarak` and
//! `jarak-gui` (docs/JARAK_NOTEPADPP_MATRIX.md, docs/JARAK_PARITY_REPORT.md). Requirements:
//! `JARAK-FR-036`, `JARAK-FR-087`, `JARAK-FR-090`, `JARAK-FR-130`, `JARAK-FR-150`.
//!
//! 1. The Step 110 settings through `jarak settings`: a backup on save, new documents' language,
//!    further word characters, the Open dialog's folder (a refused value writes nothing), the
//!    text's context menu.
//! 2. `jarak export --dry-run` detects the further languages (Perl, Pascal, Haskell, Lisp,
//!    Scheme, Fortran, Ada, Assembly, D, R, Swift, VHDL, Verilog, AutoIt, COBOL) and writes
//!    nothing.
//! 3. The release window at the audit position: the character panel, the clipboard history, the
//!    range dialog, distraction-free and post-it layouts and a coloured tab pass the audit's
//!    rules and show their content; `--always-on-top --position` start the window;
//!    `--position nope` is refused (exit 2) before any window opens.
//!
//! Machine safety: `jarak` runs without a window; the windows open for about a second each at the
//! audit position and are never made full screen or frameless (the audit screens change the
//! layout only); no input is synthesized; everything happens in a temporary folder.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

fn window(dir: &Path, doc: &Path, screen: &str, extra: &[&str]) -> kim_design::inspect::Report {
    let json = dir.join(format!("{screen}.json"));
    let mut args: Vec<String> = vec![
        "--instance".into(),
        format!("e2e-step110-{}", std::process::id()),
        "--new-window".into(),
        "--no-session".into(),
        // The checks read English labels; without this the window follows the PC's language.
        "--lang".into(),
        "en".into(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    args.push(doc.display().to_string());
    let mut child = Command::new(bin("jarak-gui"))
        .args(&args)
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
        "{screen}: {}",
        kim_design::audit::render(&r, &f, false)
    );
    r
}

fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step110_the_remaining_built_ins() {
    let sb = TempDir::new("e2e-step110");
    let dir = sb.path().join("run");
    std::fs::create_dir_all(dir.join("Roaming").join("mouv").join("jarak")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let file = dir
        .join("Roaming")
        .join("mouv")
        .join("jarak")
        .join("settings.toml");

    // 1. The settings.
    for (k, v) in [
        ("backup", "dated"),
        ("new_language", "Markdown"),
        ("word_chars", "-"),
        ("open_folder", "last"),
        ("smart_match_case", "false"),
        ("find_fill_selection", "false"),
        ("context_menu", "edit.copy,-,select.all,edit.char-panel"),
    ] {
        let (code, out) = jarak(&dir, &["settings", "set", k, v]);
        assert_eq!((code, out["data"]["value"].as_str()), (0, Some(v)), "{k}");
    }
    let before = std::fs::read_to_string(&file).unwrap();
    let (code, out) = jarak(&dir, &["settings", "set", "open_folder", "nowhere"]);
    assert_eq!(
        (code, out["error"]["code"].as_str()),
        (2, Some("setting_invalid"))
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        before,
        "nothing written"
    );
    let (code, out) = jarak(&dir, &["settings", "list", "advanced"]);
    assert_eq!(code, 0);
    assert!(
        out["data"]["settings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["key"] == "context_menu" && s["is_default"] == false)
    );
    // The window opens with the defaults again (the audit screens below need them).
    for k in [
        "backup",
        "new_language",
        "word_chars",
        "open_folder",
        "smart_match_case",
        "find_fill_selection",
        "context_menu",
    ] {
        assert_eq!(jarak(&dir, &["settings", "reset", k]).0, 0, "{k}");
    }

    // 2. The further languages.
    for (name, text, lang) in [
        ("a.pl", "my $x = 1;\n", "Perl"),
        ("a.pas", "program p; begin end.\n", "Pascal"),
        ("a.hs", "main = print 1\n", "Haskell"),
        ("a.lisp", "(defun f () nil)\n", "Lisp"),
        ("a.scm", "(define x 1)\n", "Scheme"),
        ("a.f90", "program p\nend program\n", "Fortran"),
        ("a.adb", "procedure P is begin null; end P;\n", "Ada"),
        ("a.asm", "mov eax, 1\n", "Assembly"),
        ("a.d", "void main() {}\n", "D"),
        ("a.r", "x <- 1\n", "R"),
        ("a.swift", "let x = 1\n", "Swift"),
        ("a.vhd", "entity e is end e;\n", "VHDL"),
        ("a.sv", "module m; endmodule\n", "Verilog"),
        ("a.au3", "Local $x = 1\n", "AutoIt"),
        ("a.cbl", "PROCEDURE DIVISION.\n", "COBOL"),
    ] {
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        // `export --dry-run` names the language it highlights with and writes nothing.
        let html = dir.join(format!("{name}.html"));
        let (code, out) = jarak(
            &dir,
            &[
                "export",
                &p.to_string_lossy(),
                "--output",
                &html.to_string_lossy(),
                "--dry-run",
            ],
        );
        assert_eq!(code, 0, "{name}: {out}");
        assert_eq!(out["data"]["language"].as_str(), Some(lang), "{name}");
        assert!(!html.exists(), "{name}: a dry run");
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            text,
            "{name} unchanged"
        );
    }

    // 3. The windows.
    let doc = dir.join("note.txt");
    let body = "Café 中文 and ASCII\nsecond line\n";
    std::fs::write(&doc, body).unwrap();
    let r = window(&dir, &doc, "char-panel", &[]);
    let t = texts(&r);
    assert!(t.iter().any(|s| s == "233   0xE9   é"), "the table: {t:?}");
    assert!(t.iter().any(|s| s == "Insert"), "{t:?}");
    let r = window(&dir, &doc, "clipboard-history", &[]);
    let t = texts(&r);
    assert!(t.iter().any(|s| s == "alpha"), "{t:?}");
    assert!(t.iter().any(|s| s.contains("(2 lines)")), "{t:?}");
    let r = window(&dir, &doc, "char-range", &[]);
    assert!(texts(&r).iter().any(|s| s == "Find next"));
    for screen in ["distraction-free", "post-it"] {
        let r = window(&dir, &doc, screen, &[]);
        let t = texts(&r);
        assert!(
            t.iter().any(|s| s == "2"),
            "{screen}: the line numbers: {t:?}"
        );
        assert!(
            !t.iter().any(|s| s.contains("UTF-8")),
            "{screen}: no status bar: {t:?}"
        );
    }
    let r = window(
        &dir,
        &doc,
        "tab-color",
        &["--always-on-top", "--position", "60,60"],
    );
    let t = texts(&r);
    assert!(
        t.iter().any(|s| s == "note.txt") && t.iter().any(|s| s == "Untitled"),
        "two tabs: {t:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        body,
        "the note is unchanged"
    );

    // A bad position is refused before any window opens.
    let st = Command::new(bin("jarak-gui"))
        .args(["--position", "nope", "--instance", "e2e-step110-bad"])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(st.code(), Some(2));
}
