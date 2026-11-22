//! E2E — Step 105 (#124): the settings through the real `jarak settings` commands, and the real
//! Settings window showing them (docs/JARAK_SETTINGS.md, docs/JARAK_ARCHITECTURE.md §27).
//! Requirements: `JARAK-FR-130`, `JARAK-FR-131`, `JARAK-FR-132`.
//!
//! 1. `jarak settings set theme light`, `set font_size 16`, `get`, a refused value (exit 2,
//!    `setting_invalid`, nothing written), an unknown key (exit 3), `reset` — the file holds only
//!    what was set, and lines it does not know stay.
//! 3. `jarak hotkeys set` (a clash with the user's own binding refused, exit 7), `show`, `check`;
//!    the Keyboard section shows the changed command (its Reset).
//! 2. The release window with that settings file: Appearance shows the font size 16 and the window
//!    is light (the theme comes from the file: no `KIM_UI_THEME` in this run); a document's text
//!    is drawn at 16 points; General in Korean; the Settings window passes the audit's rules.
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

fn window(dir: &Path, doc: Option<&Path>, screen: &str, lang: &str) -> kim_design::inspect::Report {
    let json = dir.join(format!("{screen}-{lang}.json"));
    let mut args = vec![
        "--instance".to_string(),
        format!("e2e-step105-{}", std::process::id()),
        "--lang".to_string(),
        lang.to_string(),
    ];
    if let Some(d) = doc {
        args.push(d.display().to_string());
    }
    let mut child = Command::new(bin("jarak-gui"))
        .args(&args)
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", dir.join(format!("{screen}-{lang}.png")))
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", "1")
        .env("KIM_UI_SCREEN", screen)
        .env_remove("KIM_UI_THEME")
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

fn texts(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components
        .iter()
        .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
        .collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step105_settings_through_the_cli_and_the_settings_window() {
    let sb = TempDir::new("e2e-step105");
    let dir = sb.path().join("run");
    std::fs::create_dir_all(dir.join("Roaming").join("mouv").join("jarak")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let file = dir
        .join("Roaming")
        .join("mouv")
        .join("jarak")
        .join("settings.toml");
    std::fs::write(&file, "# mine\nunknown_key = 1\n").unwrap();
    // 1. The command line.
    let (code, v) = jarak(&dir, &["settings", "set", "theme", "light"]);
    assert_eq!((code, v["data"]["value"].as_str()), (0, Some("light")));
    let (code, _) = jarak(&dir, &["settings", "set", "font_size", "16"]);
    assert_eq!(code, 0);
    let (code, v) = jarak(&dir, &["settings", "get", "font_size"]);
    assert_eq!((code, v["data"]["value"].as_str()), (0, Some("16")));
    let (code, v) = jarak(&dir, &["settings", "set", "tab_width", "99"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (2, Some("setting_invalid"))
    );
    let (code, v) = jarak(&dir, &["settings", "get", "colour"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (3, Some("setting_not_found"))
    );
    let (code, _) = jarak(&dir, &["settings", "set", "status_bar", "false"]);
    assert_eq!(code, 0);
    let (code, _) = jarak(&dir, &["settings", "reset", "status_bar"]);
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# mine\nunknown_key = 1\ntheme = \"light\"\nfont_size = 16\n",
        "only what was set; the file's own lines stay"
    );
    let (code, v) = jarak(&dir, &["settings", "list", "appearance"]);
    assert_eq!(code, 0);
    assert!(
        v["data"]["settings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["key"] == "font_size" && s["value"] == "16" && s["is_default"] == false)
    );
    // 2. The window shows them.
    let r = window(&dir, None, "settings-appearance", "en");
    assert!(
        r.theme.contains("light"),
        "the theme from the file: {}",
        r.theme
    );
    assert!(texts(&r).iter().any(|t| t == "Editor font size"));
    // The editor draws its text at the size the file sets (the line numbers carry it).
    let doc = dir.join("notes.txt");
    std::fs::write(
        &doc, "one
two
",
    )
    .unwrap();
    let r = window(&dir, Some(&doc), "editor-font", "en");
    let line = r
        .components
        .iter()
        .find(|c| c.id == "gutter:line#1")
        .and_then(|c| c.text.as_ref())
        .expect("the first line number");
    assert!(
        (line.font_px - 16.0).abs() < 0.5,
        "font {} px",
        line.font_px
    );
    // 3. Shortcuts: `jarak hotkeys` on the shared contract, and the Keyboard section shows them.
    let (code, v) = jarak(&dir, &["hotkeys", "set", "run.dialog", "Ctrl+Alt+R"]);
    assert_eq!(
        (code, v["data"]["written"].as_bool()),
        (0, Some(true)),
        "{v}"
    );
    let (code, v) = jarak(&dir, &["hotkeys", "set", "view.palette", "Ctrl+Alt+R"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("hotkey_conflict")),
        "{v}"
    );
    let (code, v) = jarak(&dir, &["hotkeys", "show", "run.dialog"]);
    assert_eq!(code, 0);
    assert!(
        v["data"]["action"].to_string().contains("Ctrl+Alt+R"),
        "{v}"
    );
    let (code, v) = jarak(&dir, &["hotkeys", "check"]);
    assert_eq!(code, 0, "{v}");
    let r = window(&dir, None, "settings-keyboard", "en");
    assert!(
        r.components.iter().any(|c| c.id == "button:Reset"),
        "the changed command offers Reset: the window read hotkeys.json"
    );
    let r = window(&dir, None, "settings-general", "ko");
    assert!(texts(&r).iter().any(|t| t == "일반"), "{:?}", texts(&r));
    assert!(texts(&r).iter().any(|t| t == "테마"));
}
