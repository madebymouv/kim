//! E2E — Step 92 (#111): Vim editing, the command palette and the menu bar in the real window
//! (ADR 0102, docs/JARAK_VIM.md).
//! Requirements: `JARAK-FR-050`…`JARAK-FR-054`, `KIM-NFR-012`.
//!
//! The release `jarak-gui` with the UI audit's environment on notes in a temporary folder:
//!
//! 1. `vim-session`: the window switches to Vim editing and runs a session through its Vim
//!    layer — `O` opens a line and types `top`, Esc, `j0dw` deletes a word, `G` and `dd` delete
//!    the last line, `:%s/gamma/GAMMA/g` substitutes, `:w` writes. The file holds the result
//!    (its line endings kept) and the geometry audit finds no failure.
//! 2. `palette`: the command palette lists the commands matching "save" with their keys
//!    (`Save`, `Save as…`).
//! 3. `menu-bar`: the menu bar shows its six menus.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized (the session runs inside the window); nothing outside the temporary folder is
//! touched, no settings are written.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let shot = dir.join(format!("{screen}.png"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance",
            &format!("e2e-step92-{}", std::process::id()),
            "--lang",
            "en",
            &format!("{}:1:1", doc.display()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
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
    assert!(
        exited,
        "{screen}: the window closed by itself after the measurement"
    );
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

fn ids(r: &kim_design::inspect::Report) -> Vec<String> {
    r.components.iter().map(|c| c.id.clone()).collect()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step92_a_vim_session_the_palette_and_the_menu_bar() {
    let sb = TempDir::new("e2e-step92");
    let dir = sb.path().join("notes");
    std::fs::create_dir_all(&dir).unwrap();
    // 1. A Vim session that writes the file.
    let note = dir.join("vim.md");
    std::fs::write(&note, "alpha beta\r\ngamma\r\ndelta\r\n").unwrap();
    let r = window(&dir, &note, "vim-session");
    assert_eq!(r.screen, "vim-session");
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "top\r\nbeta\r\nGAMMA\r\n",
        "O, dw, dd, :%s and :w through the real window (CRLF kept)"
    );
    assert!(
        !dir.join("Roaming")
            .join("mouv")
            .join("jarak")
            .join("settings.toml")
            .exists(),
        "an audit run writes no settings"
    );
    // 2. The palette.
    let other = dir.join("other.md");
    std::fs::write(&other, "# Other\n").unwrap();
    let r = window(&dir, &other, "palette");
    let found = ids(&r);
    for want in [
        "search:Search commands",
        "menu-item:Save",
        "menu-item:Save as…",
    ] {
        assert!(found.iter().any(|i| i == want), "{want}: {found:?}");
    }
    // 3. The menu bar.
    let r = window(&dir, &other, "menu-bar");
    let found = ids(&r);
    for want in ["File", "Edit", "Selection", "Go", "View", "Markdown"] {
        assert!(
            found.iter().any(|i| *i == format!("button:{want}")),
            "{want}: {found:?}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(&other).unwrap(),
        "# Other\n",
        "nothing changed it"
    );
}
