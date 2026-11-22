//! E2E — #98 T4 (follow-up of Step 36): defaults per kind of profile
//! (docs/TABBY_FEATURE_MATRIX.md `SHELL-14`). `SUBAK-FR-013`.
//!
//! With the released `subak` in a private `APPDATA` sandbox (no Host, no window), a hand-written
//! `config.toml` with `[profile_defaults.local]` and `[profile_defaults.ssh]`: `profile show`
//! reports a local profile with the local folder, group and variables, a profile's own values
//! winning, an SSH profile with the SSH identity and group (and no local folder), and
//! `config show` reports a wrong default as a problem while every profile is still listed.
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

fn profile(sb: &TempDir, name: &str) -> Value {
    let (code, doc) = json(sb, &["profile", "show", name]);
    assert_eq!(code, 0, "{doc}");
    let p = &doc["data"]["profile"];
    assert_eq!(p["name"], name, "{doc}");
    p.clone()
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn issue98_profile_defaults_fill_local_and_ssh_profiles() {
    let sb = TempDir::new("issue98-defaults");
    let dir = sb.path().join("Roaming/mouv/subak");
    std::fs::create_dir_all(&dir).unwrap();
    let work = sb.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let key = sb.path().join("id_ed25519");
    let text = format!(
        "config_version = 1\n\n[terminal]\ndetect_shells = false\n\n\
         [profile_defaults.local]\ncwd = '{}'\nenv = {{ LANG = \"ko_KR.UTF-8\", EDITOR = \"code\" }}\ngroup = \"Local\"\n\n\
         [profile_defaults.ssh]\nidentity = '{}'\ngroup = \"Servers\"\n\n\
         [[profiles]]\nname = \"Plain\"\nprogram = \"cmd.exe\"\n\n\
         [[profiles]]\nname = \"Own\"\nprogram = \"cmd.exe\"\nenv = {{ EDITOR = \"vim\" }}\nunset_env = [\"LANG\"]\n\n\
         [[profiles]]\nname = \"Prod\"\nssh = \"ops@prod.example\"\n",
        work.display(),
        key.display()
    );
    std::fs::write(dir.join("config.toml"), text).unwrap();
    let plain = profile(&sb, "Plain");
    assert_eq!(plain["cwd"], work.display().to_string());
    assert_eq!(plain["group"], "Local");
    assert_eq!(plain["env"]["LANG"], "ko_KR.UTF-8");
    assert_eq!(plain["env"]["EDITOR"], "code");
    let own = profile(&sb, "Own");
    assert_eq!(own["env"]["EDITOR"], "vim", "the profile's own value wins");
    assert!(
        own["env"]["LANG"].is_null(),
        "a removed variable is not set back"
    );
    assert_eq!(own["unset_env"], serde_json::json!(["LANG"]));
    let prod = profile(&sb, "Prod");
    assert_eq!(prod["identity"], key.display().to_string());
    assert_eq!(prod["group"], "Servers");
    assert!(prod["cwd"].is_null(), "local defaults stay local");
    // A wrong default is a problem and is not used; the profiles stay.
    let broken = std::fs::read_to_string(dir.join("config.toml"))
        .unwrap()
        .replace("group = \"Local\"", "group = \"Local\"\nidentity = 'C:\\k'");
    std::fs::write(dir.join("config.toml"), broken).unwrap();
    let (code, show) = json(&sb, &["config", "show"]);
    assert_eq!(code, 0, "{show}");
    assert!(
        show.to_string()
            .contains("is not a default of local profiles"),
        "{show}"
    );
    let plain = profile(&sb, "Plain");
    assert!(
        plain["cwd"].is_null(),
        "the wrong local default is not used"
    );
}
