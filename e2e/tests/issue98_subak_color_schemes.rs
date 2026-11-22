//! E2E — #98 T4 (follow-up of Step 36): Subak's colour schemes (docs/TABBY_FEATURE_MATRIX.md
//! `PLUG-05`, `CORE-38`). `SUBAK-FR-013`.
//!
//! The way a user or an agent sets them up, with the released `subak` in a private `APPDATA`
//! sandbox (no Host, no window): `config show` has `appearance.color_scheme` = `kim`; a bundled
//! Windows Terminal scheme (`One Half Dark`) is set without a warning (`--dry-run` first, which
//! writes nothing); an unknown name is kept but warned about with the names there are; a scheme
//! file the user puts in `%APPDATA%\mouv\subak\schemes` (Windows Terminal's format) is found and
//! set without a warning; an empty name is refused. The window's use of them is tested in
//! `crates/subak/tests/gui_window.rs::colour_schemes_from_the_bundle_and_the_folder_draw_the_terminal`.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, run_sandboxed};
use serde_json::Value;

fn json(sb: &TempDir, args: &[&str]) -> (i32, Value) {
    let mut a = vec!["--json"];
    a.extend_from_slice(args);
    let out = run_sandboxed(&bin("subak"), &a, sb);
    let doc = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), doc)
}

fn value(sb: &TempDir) -> Value {
    let (code, show) = json(sb, &["config", "show"]);
    assert_eq!(code, 0, "{show}");
    show["data"]["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["key"] == "appearance.color_scheme")
        .unwrap_or_else(|| panic!("no color_scheme in {show}"))["value"]
        .clone()
}

/// The warnings about colour schemes (`config set` also says when a running window applies it).
fn warnings(doc: &Value) -> Vec<String> {
    doc["warnings"]
        .as_array()
        .map(|w| {
            w.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .filter(|x| x.contains("colour scheme"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn issue98_colour_schemes_are_data_files_and_bundled() {
    let sb = TempDir::new("issue98-schemes");
    assert_eq!(value(&sb), "kim");
    let config = sb.path().join("Roaming/mouv/subak/config.toml");
    // A bundled scheme: a dry run writes nothing, then the change, no warning.
    let (code, doc) = json(
        &sb,
        &[
            "config",
            "set",
            "appearance.color_scheme",
            "One Half Dark",
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "{doc}");
    assert!(!config.exists(), "a dry run wrote nothing");
    let (code, doc) = json(
        &sb,
        &["config", "set", "appearance.color_scheme", "One Half Dark"],
    );
    assert_eq!(code, 0, "{doc}");
    assert!(warnings(&doc).is_empty(), "{doc}");
    assert_eq!(value(&sb), "One Half Dark");
    // An unknown name: kept, warned about with the names there are.
    let (code, doc) = json(
        &sb,
        &["config", "set", "appearance.color_scheme", "Dracula"],
    );
    assert_eq!(code, 0, "{doc}");
    let w = warnings(&doc).join(" ");
    assert!(
        w.contains("Dracula") && w.contains("Campbell") && w.contains("Tango Light"),
        "{w}"
    );
    // The user's own scheme file (Windows Terminal's format) is found.
    let dir = sb.path().join("Roaming/mouv/subak/schemes");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("dracula.json"),
        r##"{"schemes": [{"name": "Dracula", "foreground": "#F8F8F2", "background": "#282A36",
        "cursorColor": "#F8F8F2", "selectionBackground": "#44475A",
        "black": "#21222C", "red": "#FF5555", "green": "#50FA7B", "yellow": "#F1FA8C",
        "blue": "#BD93F9", "purple": "#FF79C6", "cyan": "#8BE9FD", "white": "#F8F8F2",
        "brightBlack": "#6272A4", "brightRed": "#FF6E6E", "brightGreen": "#69FF94",
        "brightYellow": "#FFFFA5", "brightBlue": "#D6ACFF", "brightPurple": "#FF92DF",
        "brightCyan": "#A4FFFF", "brightWhite": "#FFFFFF"}]}"##,
    )
    .unwrap();
    let (code, doc) = json(
        &sb,
        &["config", "set", "appearance.color_scheme", "dracula"],
    );
    assert_eq!(code, 0, "{doc}");
    assert!(
        warnings(&doc).is_empty(),
        "the folder's scheme is found: {doc}"
    );
    // An empty name is refused and changes nothing.
    let (code, doc) = json(&sb, &["config", "set", "appearance.color_scheme", ""]);
    assert_eq!(code, 2, "{doc}");
    assert_eq!(value(&sb), "dracula");
}
