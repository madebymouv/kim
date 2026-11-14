//! E2E — ADR 0096: a "hold + press" combination made in the rule editor is the engine's hold
//! layer. Requirements: `DDALGAK-FR-016`, `DDALGAK-FR-050`, `KIM-FR-012`.
//!
//! The release `ddalgak` in an isolated profile (no app): `rules.json` holds what the editor
//! writes for "hold CapsLock, press J → Down" (the mode `hold-capslock` titled *Hold CapsLock*,
//! the rule that holds it while CapsLock is down, and J → Down limited to it). The simulation
//! (`ddalgak test`, nothing is sent) shows: J alone in the base mode matches nothing and reaches
//! the application; CapsLock fires the hold rule; J in the layer fires the remap to Down.
//!
//! Machine safety: sandboxed `%APPDATA%`/`%LOCALAPPDATA%`; `test` never talks to an app; no input.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, require_e2e};
use serde_json::{Value, json};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn run(sb: &TempDir, args: &[&str]) -> Value {
    let p = sb.path();
    let out = Command::new(bin("ddalgak"))
        .args(args)
        .arg("--json")
        .env("APPDATA", p.join("Roaming"))
        .env("LOCALAPPDATA", p.join("Local"))
        .env_remove("KIM_NO_INPUT")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{args:?}: not JSON ({e})"))
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn adr0096_hold_and_press_is_the_engines_hold_layer() {
    require_e2e();
    let sb = TempDir::new("adr0096");
    let dir = sb.path().join("Roaming/mouv/ddalgak");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(sb.path().join("Local")).unwrap();
    let doc = json!({
        "schema": "ddalgak.rules", "schema_version": 1, "hotkey_contract": 1, "revision": 1,
        "active_profile": "default",
        "profiles": [ { "id": "default", "title": "Default", "default_mode": "base",
            "modes": [ { "id": "base", "title": "Base" },
                       { "id": "hold-capslock", "title": "Hold CapsLock" } ],
            "rules": [
                { "id": "hold-capslock-for-its-layer", "title": "Hold CapsLock for its layer",
                  "enabled": true, "trigger": { "keys": "CapsLock" },
                  "steps": [ { "type": "mode", "op": "hold", "mode": "hold-capslock" } ] },
                { "id": "j-acts-as-down", "title": "J acts as Down", "enabled": true,
                  "modes": ["hold-capslock"], "trigger": { "keys": "J" },
                  "steps": [ { "type": "remap", "to": "Down" } ],
                  "consent": { "input-injection": { "granted_at": "2026-11-15T00:00:00Z", "via": "gui" } } }
            ] } ]
    });
    std::fs::write(
        dir.join("rules.json"),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();

    let alone = run(&sb, &["test", "J"]);
    assert_eq!(alone["ok"], true, "{alone:#}");
    assert_eq!(
        alone["data"]["decision"]["outcome"], "no_match",
        "{alone:#}"
    );
    assert_eq!(alone["data"]["decision"]["key_reaches_app"], true);

    let hold = run(&sb, &["test", "CapsLock"]);
    assert_eq!(
        hold["data"]["decision"]["rule"], "hold-capslock-for-its-layer",
        "{hold:#}"
    );

    let layered = run(&sb, &["test", "J", "--mode", "hold-capslock"]);
    assert_eq!(
        layered["data"]["decision"]["rule"], "j-acts-as-down",
        "{layered:#}"
    );
    assert!(
        layered["data"]["effects"].to_string().contains("Down"),
        "{layered:#}"
    );
}
