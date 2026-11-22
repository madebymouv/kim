//! E2E — #98 T4 (follow-up of Step 36): Docker container profiles
//! (docs/TABBY_FEATURE_MATRIX.md `SHELL-26`). `SUBAK-FR-013`.
//!
//! With the released `subak` in a private `APPDATA` sandbox (no Host, no window, no Docker call):
//! a `[[profiles]]` entry with `docker` (and `docker_shell`) is listed as a profile that runs
//! Docker's own client, `docker.exe exec -it <container> <shell>` (`sh` by default), and a Docker
//! profile with a program, or with a container name Docker would read as an option, is refused
//! with its reason while the others stay. Nothing here starts or enters a container (the
//! maintainer's Docker is theirs); the window starts such a profile like any other.
#![cfg(windows)]

use kim_e2e::{TempDir, bin, run_sandboxed};
use serde_json::{Value, json};

fn run(sb: &TempDir, args: &[&str]) -> (i32, Value) {
    let mut a = vec!["--json"];
    a.extend_from_slice(args);
    let out = run_sandboxed(&bin("subak"), &a, sb);
    let doc = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), doc)
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn issue98_docker_profiles_run_docker_exec() {
    let sb = TempDir::new("issue98-docker");
    let dir = sb.path().join("Roaming/mouv/subak");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "config_version = 1\n\n[terminal]\ndetect_shells = false\n\n\
         [[profiles]]\nname = \"Web\"\ndocker = \"web-1\"\ndocker_shell = \"/bin/bash\"\n\n\
         [[profiles]]\nname = \"Db\"\ndocker = \"db\"\n\n\
         [[profiles]]\nname = \"Option\"\ndocker = \"--privileged\"\n\n\
         [[profiles]]\nname = \"Both\"\ndocker = \"x\"\nprogram = \"cmd.exe\"\n",
    )
    .unwrap();
    let (code, doc) = run(&sb, &["profile", "show", "Web"]);
    assert_eq!(code, 0, "{doc}");
    let p = &doc["data"]["profile"];
    assert_eq!(p["program"], "docker.exe");
    assert_eq!(p["args"], json!(["exec", "-it", "web-1", "/bin/bash"]));
    assert_eq!(p["docker"], "web-1");
    let (code, doc) = run(&sb, &["profile", "show", "Db"]);
    assert_eq!(code, 0, "{doc}");
    assert_eq!(
        doc["data"]["profile"]["args"],
        json!(["exec", "-it", "db", "sh"])
    );
    let (code, list) = run(&sb, &["profile", "list"]);
    assert_eq!(code, 0, "{list}");
    let names: Vec<&str> = list["data"]["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    assert_eq!(names, ["Web", "Db"], "{list}");
    let (_, show) = run(&sb, &["config", "show"]);
    let text = show.to_string();
    assert!(text.contains("must be a container name or id"), "{text}");
    assert!(text.contains("instead of `program`"), "{text}");
}
