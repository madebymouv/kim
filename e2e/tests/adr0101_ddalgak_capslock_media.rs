//! E2E — ADR 0101: CapsLock as a modifier, Right Shift media and volume keys, typed arrows, the
//! automation toggle and Settings by key. Requirements: `DDALGAK-FR-077`, `DDALGAK-FR-078`,
//! `DDALGAK-FR-004`, `DDALGAK-SEC-003`, `DDALGAK-SEC-009`.
//!
//! The real `ddalgak` CLI with a sandboxed `%APPDATA%` holding the 18 mappings exactly as the
//! editor saved them from an empty profile on the real desktop
//! (`docs/contracts/examples/ddalgak-rules-v1/18-keyboard-navigation-media.json`):
//!
//! 1. `rule check`: 18 rules, no error, no warning.
//! 2. `ddalgak test` (a simulation; nothing is pressed): `CapsLock+W` fires the Up remap,
//!    `RShift+Enter` the Play/Pause key, `CapsLock+Shift+Comma` the ← text; `LShift+Enter` and a
//!    plain `W` reach the application unchanged.
//! 3. Paused: `CapsLock+Backspace` (the engine-control rule) still fires, `CapsLock+W` steps aside
//!    with `skipped_paused`.
//! 4. `rule add` refuses `CapsLock+Alt+F4` and `CapsLock+CapsLock` with their reasons.
//!
//! Machine safety: CLI only; no app runs, no window opens, no key is pressed.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn sandbox() -> TempDir {
    let sb = TempDir::new("adr0101");
    let cfg = sb.path().join("Roaming").join("mouv").join("ddalgak");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(sb.path().join("Local")).unwrap();
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("contracts")
        .join("examples")
        .join("ddalgak-rules-v1")
        .join("18-keyboard-navigation-media.json");
    std::fs::copy(example, cfg.join("rules.json")).unwrap();
    sb
}

fn run(sb: &TempDir, args: &[&str]) -> (i32, Value) {
    let out = Command::new(bin("ddalgak"))
        .args(args)
        .arg("--json")
        .env("APPDATA", sb.path().join("Roaming"))
        .env("LOCALAPPDATA", sb.path().join("Local"))
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .unwrap();
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{args:?}: not JSON ({e})"));
    (out.status.code().unwrap_or(-1), doc)
}

fn decision(sb: &TempDir, args: &[&str]) -> Value {
    let mut a = vec!["test"];
    a.extend_from_slice(args);
    let (code, d) = run(sb, &a);
    assert_eq!(code, 0, "{args:?}: {d:#}");
    assert_eq!(
        d["data"]["injected_events"], 0,
        "a simulation presses nothing"
    );
    d["data"]["decision"].clone()
}

#[test]
fn adr0101_capslock_chords_media_keys_and_the_automation_toggle() {
    let sb = sandbox();
    let (code, check) = run(&sb, &["rule", "check"]);
    assert_eq!(code, 0, "{check:#}");
    assert_eq!(check["data"]["summary"]["errors"], 0, "{check:#}");
    assert_eq!(check["data"]["summary"]["warnings"], 0, "{check:#}");

    for (keys, rule) in [
        ("CapsLock+W", "caps-w"),
        ("CapsLock+Z", "caps-z"),
        ("RShift+Enter", "play-pause"),
        ("RShift+Up", "volume-up"),
        ("CapsLock+Shift+Comma", "left-arrow"),
        ("CapsLock+M", "both-arrows"),
        ("CapsLock+code:Slash", "open-settings"),
    ] {
        let d = decision(&sb, &[keys]);
        assert_eq!(
            (d["outcome"].as_str(), d["rule"].as_str()),
            (Some("fired"), Some(rule)),
            "{keys}: {d:#}"
        );
        assert_eq!(d["key_reaches_app"], false, "{keys}");
    }
    for keys in ["LShift+Enter", "W", "Enter"] {
        let d = decision(&sb, &[keys]);
        assert_eq!(d["outcome"], "no_match", "{keys}: {d:#}");
        assert_eq!(d["key_reaches_app"], true, "{keys}");
    }
    // Paused: only the engine-control rule.
    let d = decision(&sb, &["CapsLock+Backspace", "--paused"]);
    assert_eq!(
        (d["outcome"].as_str(), d["rule"].as_str()),
        (Some("fired"), Some("automation-toggle"))
    );
    let d = decision(&sb, &["CapsLock+W", "--paused"]);
    assert_eq!(
        (d["outcome"].as_str(), d["code"].as_str()),
        (Some("skipped"), Some("skipped_paused"))
    );
    // Reserved and invalid CapsLock chords never become rules.
    for (keys, code) in [
        ("CapsLock+Alt+F4", "hotkey_reserved"),
        ("CapsLock+CapsLock", "hotkey_invalid"),
    ] {
        let (exit, d) = run(
            &sb,
            &[
                "rule",
                "add",
                "--trigger",
                keys,
                "--action",
                "block",
                "--title",
                "x",
                "--dry-run",
            ],
        );
        assert_ne!(exit, 0, "{keys}: {d:#}");
        assert_eq!(d["error"]["code"], code, "{keys}: {d:#}");
    }
}
