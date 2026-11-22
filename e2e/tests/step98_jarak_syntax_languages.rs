//! E2E — Step 98 (#117): syntax highlighting, the Language menu and user-defined languages in the
//! real window (ADR 0103, docs/JARAK_ARCHITECTURE.md §20).
//! Requirements: `JARAK-FR-090`, `JARAK-FR-091`.
//!
//! The release `jarak-gui` with the UI audit's environment:
//!
//! 1. A Rust file: the status bar names the language and the audit finds no failure.
//! 2. A user-defined language in the sandbox's `%APPDATA%\mouv\jarak\languages` (JSON) and a
//!    Notepad++ UDL converted by `syntax::import_npp_udl` are both picked by extension.
//! 3. `palette-language`: the Language menu lists the built-in languages.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; everything happens in a temporary folder (its own `%APPDATA%`/`%LOCALAPPDATA%`).
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance".to_string(),
            format!("e2e-step98-{}", std::process::id()),
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

fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step98_languages_are_detected_user_languages_load_and_the_menu_lists_them() {
    if let Some(why) = kim_e2e::jarak_window_unavailable() {
        eprintln!("skipped (no window can be opened on this machine): {why}");
        return;
    }
    let sb = TempDir::new("e2e-step98");
    let dir = sb.path().join("run");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let langs = dir
        .join("Roaming")
        .join("mouv")
        .join("jarak")
        .join("languages");
    std::fs::create_dir_all(&langs).unwrap();
    std::fs::write(
        langs.join("kimlang.json"),
        r#"{"name": "Kimlang", "extensions": ["kim"], "line_comment": ["//"], "keywords": ["say"]}"#,
    )
    .unwrap();
    let npp = r#"<NotepadPlus><UserLang name="Simple" ext="smp"><KeywordLists><Keywords name="Comments">00#</Keywords><Keywords name="Keywords1">print</Keywords></KeywordLists></UserLang></NotepadPlus>"#;
    let simple = jarak_core::syntax::import_npp_udl(npp).unwrap();
    std::fs::write(
        langs.join("simple.json"),
        serde_json::to_string_pretty(&simple).unwrap(),
    )
    .unwrap();
    let files = [
        (
            "main.rs",
            "fn main() {} // 주석
",
            "Rust",
        ),
        (
            "a.kim",
            "say 1 // c
",
            "Kimlang",
        ),
        (
            "b.smp",
            "print 1 # c
",
            "Simple",
        ),
    ];
    for (name, text, lang) in files {
        let f = docs.join(name);
        std::fs::write(&f, text).unwrap();
        let r = window(&dir, &f, "editor");
        let t = texts(&r);
        assert!(
            t.iter().any(|s| s == lang),
            "{name}: {lang} in the status bar: {t:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            text,
            "highlighting changes nothing"
        );
    }
    let r = window(&dir, &docs.join("main.rs"), "palette-language");
    let t = texts(&r);
    assert!(t.iter().any(|s| s.starts_with("Language: ")), "{t:?}");
}
