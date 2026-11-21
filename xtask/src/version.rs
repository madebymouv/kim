//! `cargo xtask bump` and `version-check` (ADR 0097): every development change raises the
//! version of the tools it changes by one patch step (`0.0.1` → `0.0.2`).
//!
//! - A *group* is what ships under one version: `kim` (kim, kim-manager, kim-setup), `subak`
//!   (subak, subak-term), `noeul` (noeul, noeul-settings), `ddalgak` (ddalgak, ddalgak-editor),
//!   `jarak` (jarak, jarak-core, jarak-gui; ADR 0102);
//!   every other workspace library (kim-design, kim-ui, kim-hotkeys, kim-winres) is its own group.
//! - `bump --changed` reads `git status`: a crate changed outside its `tests/`, `examples/` and
//!   `benches/` marks its group, and every group that depends on a marked one (a shared library
//!   changed, so the tools built on it changed too). A group already raised since `HEAD` is not
//!   raised again, so running it twice before a commit is harmless.
//! - `bump <group>...` raises the named groups (and their dependents) explicitly.
//! - Raising writes each crate's `[package] version`, every workspace path dependency's version
//!   requirement on it, and the top-level `"version"` of the crate's `capabilities.json` and
//!   `hotkeys.manifest.json` where it equals the old version; `Cargo.lock` follows.
//! - `version-check` (part of `ci`) fails when a group's crates, those JSON files or the path
//!   requirements disagree.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const GROUPS: [(&str, &[&str]); 5] = [
    ("kim", &["kim", "kim-manager", "kim-setup"]),
    ("subak", &["subak", "subak-term"]),
    ("noeul", &["noeul", "noeul-settings"]),
    ("ddalgak", &["ddalgak", "ddalgak-editor"]),
    ("jarak", &["jarak", "jarak-core", "jarak-gui"]),
];
/// Never versioned by this tool (development helpers, not shipped).
const SKIP: [&str; 2] = ["xtask", "kim-e2e"];
/// Published contract examples that tests compare with a group's real output: the group's
/// version is their top-level or `data.version`. Kim's CLI golden files are re-blessed instead.
const EXAMPLES: [(&str, &str); 6] = [
    (
        "kim",
        "docs/contracts/examples/tool-capabilities-v1/kim.json",
    ),
    (
        "subak",
        "docs/contracts/examples/tool-capabilities-v1/subak.json",
    ),
    (
        "subak",
        "docs/contracts/examples/cli-envelope-v1/subak-capabilities.json",
    ),
    (
        "noeul",
        "docs/contracts/examples/tool-capabilities-v1/noeul.json",
    ),
    (
        "ddalgak",
        "docs/contracts/examples/tool-capabilities-v1/ddalgak.json",
    ),
    (
        "jarak",
        "docs/contracts/examples/tool-capabilities-v1/jarak.json",
    ),
];
const JSON_FILES: [&str; 3] = [
    "src/capabilities.json",
    "src/hotkeys.manifest.json",
    "hotkeys.manifest.json",
];

#[derive(Debug, Clone)]
struct Pkg {
    name: String,
    version: String,
    dir: PathBuf,
    /// Workspace crates this one depends on (by path).
    deps: Vec<String>,
}

fn group_of(name: &str) -> String {
    GROUPS
        .iter()
        .find(|(_, cs)| cs.contains(&name))
        .map_or(name, |(g, _)| g)
        .to_string()
}

fn workspace(root: &Path) -> Result<BTreeMap<String, Pkg>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let doc: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut map = BTreeMap::new();
    for p in doc["packages"].as_array().into_iter().flatten() {
        let name = p["name"].as_str().unwrap_or_default().to_string();
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        let manifest = PathBuf::from(p["manifest_path"].as_str().unwrap_or_default());
        let deps = p["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| d["path"].is_string())
            .filter_map(|d| d["name"].as_str().map(String::from))
            .collect();
        map.insert(
            name.clone(),
            Pkg {
                name,
                version: p["version"].as_str().unwrap_or_default().to_string(),
                dir: manifest.parent().unwrap_or(root).to_path_buf(),
                deps,
            },
        );
    }
    Ok(map)
}

/// `0.0.1` → `0.0.2`, `0.0.9` → `0.0.10`; a pre-release suffix is dropped.
pub fn next_patch(v: &str) -> Result<String, String> {
    let core = v.split(['-', '+']).next().unwrap_or(v);
    let parts: Vec<u64> = core
        .split('.')
        .map(|p| p.parse::<u64>().map_err(|_| format!("not a version: {v}")))
        .collect::<Result<_, _>>()?;
    let [a, b, c] = parts[..] else {
        return Err(format!("not a version: {v}"));
    };
    Ok(format!("{a}.{b}.{}", c + 1))
}

fn cmp_version(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |v: &str| -> Vec<u64> {
        v.split(['-', '+'])
            .next()
            .unwrap_or(v)
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    key(a).cmp(&key(b))
}

/// Paths changed against `HEAD` (staged, unstaged, untracked), relative to the root.
fn changed_paths(root: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["status", "--porcelain=v1", "-uall", "--no-renames"])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("git status failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.get(3..))
        .map(|p| p.trim_matches('"').replace('\\', "/"))
        .collect())
}

/// The crate a changed path belongs to, when the change can reach its shipped program.
fn crate_of_change(path: &str, pkgs: &BTreeMap<String, Pkg>, root: &Path) -> Option<String> {
    pkgs.values().find_map(|p| {
        let rel = p
            .dir
            .strip_prefix(root)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        let rest = path.strip_prefix(&format!("{rel}/"))?;
        let ignored = ["tests/", "examples/", "benches/"]
            .iter()
            .any(|d| rest.starts_with(d))
            || rest.ends_with(".md");
        (!ignored).then(|| p.name.clone())
    })
}

/// The version `HEAD` has for a crate (None for a crate that is new).
fn head_version(root: &Path, pkg: &Pkg) -> Option<String> {
    let rel = pkg
        .dir
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let out = Command::new("git")
        .args(["show", &format!("HEAD:{rel}/Cargo.toml")])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    package_version(&String::from_utf8_lossy(&out.stdout))
}

fn package_version(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
        } else if in_package && t.starts_with("version") {
            return t.split('"').nth(1).map(String::from);
        }
    }
    None
}

/// Groups to raise: the marked ones and, repeatedly, every group depending on a raised one.
fn with_dependents(marked: BTreeSet<String>, pkgs: &BTreeMap<String, Pkg>) -> BTreeSet<String> {
    let mut groups = marked;
    loop {
        let before = groups.len();
        for p in pkgs.values() {
            if p.deps.iter().any(|d| groups.contains(&group_of(d))) {
                groups.insert(group_of(&p.name));
            }
        }
        if groups.len() == before {
            return groups;
        }
    }
}

fn replace_package_version(toml: &str, new: &str) -> String {
    let mut in_package = false;
    let mut done = false;
    let mut out = String::with_capacity(toml.len());
    for line in toml.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
        }
        if in_package && !done && t.starts_with("version") && t.contains('"') {
            let start = line.find('"').unwrap_or(0);
            let end = line[start + 1..]
                .find('"')
                .map_or(line.len(), |e| start + 1 + e);
            out.push_str(&line[..=start]);
            out.push_str(new);
            out.push_str(&line[end..]);
            done = true;
        } else {
            out.push_str(line);
        }
    }
    out
}

fn replace_dep_requirement(toml: &str, dep: &str, old: &str, new: &str) -> String {
    let re = regex::Regex::new(&format!(
        r#"(?m)^(\s*{} = \{{ version = "){}(")"#,
        regex::escape(dep),
        regex::escape(old)
    ))
    .expect("valid regex");
    re.replace_all(toml, format!("${{1}}{new}${{2}}"))
        .into_owned()
}

fn json_version_line(text: &str, old: &str, new: &str) -> Option<String> {
    let doc: Value = serde_json::from_str(text).ok()?;
    if doc["version"].as_str() != Some(old) && doc["data"]["version"].as_str() != Some(old) {
        return None;
    }
    let needle = format!("\"version\": \"{old}\"");
    // Exactly one such line, so nothing else (a catalog entry, another tool) moves with it.
    (text.matches(&needle).count() == 1)
        .then(|| text.replacen(&needle, &format!("\"version\": \"{new}\""), 1))
}

fn example_version(text: &str) -> Option<String> {
    let doc: Value = serde_json::from_str(text).ok()?;
    doc["version"]
        .as_str()
        .or_else(|| doc["data"]["version"].as_str())
        .map(String::from)
}

fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut v = vec![root.join("xtask/Cargo.toml"), root.join("e2e/Cargo.toml")];
    if let Ok(rd) = std::fs::read_dir(root.join("crates")) {
        v.extend(
            rd.flatten()
                .map(|e| e.path().join("Cargo.toml"))
                .filter(|p| p.exists()),
        );
    }
    v
}

pub fn run_bump(root: &Path, args: &[String]) -> Result<(), String> {
    let dry = args.iter().any(|a| a == "--dry-run");
    let pkgs = workspace(root)?;
    let named: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let mut marked = BTreeSet::new();
    if args.iter().any(|a| a == "--changed") {
        for path in changed_paths(root)? {
            if let Some(c) = crate_of_change(&path, &pkgs, root) {
                marked.insert(group_of(&c));
            }
        }
    }
    for n in named {
        let known = pkgs.keys().any(|k| group_of(k) == *n);
        if !known {
            return Err(format!("unknown group `{n}`"));
        }
        marked.insert(n.clone());
    }
    if marked.is_empty() && !args.iter().any(|a| a == "--changed") {
        return Err("usage: cargo xtask bump --changed | <group>... [--dry-run]".into());
    }
    let groups = with_dependents(marked, &pkgs);
    // Each group's crates: current version, new version (one step above the group's highest).
    let mut plan: Vec<(Pkg, String)> = Vec::new();
    for g in &groups {
        let members: Vec<&Pkg> = pkgs.values().filter(|p| group_of(&p.name) == *g).collect();
        let raised = members.iter().any(|p| {
            head_version(root, p)
                .is_some_and(|h| cmp_version(&p.version, &h) == std::cmp::Ordering::Greater)
        });
        if raised {
            println!("bump: {g} already raised since HEAD; left as it is");
            continue;
        }
        let top = members
            .iter()
            .map(|p| p.version.as_str())
            .max_by(|a, b| cmp_version(a, b))
            .unwrap_or("0.0.0");
        let new = next_patch(top)?;
        for p in members {
            println!("bump: {} {} -> {new}", p.name, p.version);
            plan.push(((*p).clone(), new.clone()));
        }
    }
    if plan.is_empty() {
        println!("bump: nothing to raise");
        return Ok(());
    }
    if dry {
        println!("bump: --dry-run, nothing written");
        return Ok(());
    }
    let write =
        |p: &Path, s: &str| std::fs::write(p, s).map_err(|e| format!("{}: {e}", p.display()));
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    for (pkg, new) in &plan {
        let m = pkg.dir.join("Cargo.toml");
        write(&m, &replace_package_version(&read(&m)?, new))?;
        for f in JSON_FILES {
            let p = pkg.dir.join(f);
            if let Ok(text) = std::fs::read_to_string(&p)
                && let Some(t) = json_version_line(&text, &pkg.version, new)
            {
                write(&p, &t)?;
            }
        }
    }
    for (g, f) in EXAMPLES {
        let Some((pkg, new)) = plan.iter().find(|(p, _)| group_of(&p.name) == g) else {
            continue;
        };
        let p = root.join(f);
        let text = read(&p)?;
        match json_version_line(&text, &pkg.version, new) {
            Some(t) => write(&p, &t)?,
            None => return Err(format!("{f}: no single version {} to raise", pkg.version)),
        }
    }
    for m in manifests(root) {
        let mut text = read(&m)?;
        let before = text.clone();
        for (pkg, new) in &plan {
            text = replace_dep_requirement(&text, &pkg.name, &pkg.version, new);
        }
        if text != before {
            write(&m, &text)?;
        }
    }
    // Cargo.lock follows the new versions (workspace members only, no network).
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let st = Command::new(cargo)
        .args(["update", "--workspace", "--offline"])
        .current_dir(root)
        .status()
        .map_err(|e| e.to_string())?;
    if !st.success() {
        return Err("cargo update --workspace failed".into());
    }
    // Kim's CLI golden files carry its version in several outputs: re-bless them from the real
    // program (review the diff before committing).
    if plan.iter().any(|(p, _)| p.name == "kim") {
        println!("bump: re-blessing docs/contracts/examples/kim-cli-v1 (KIM_BLESS=1)");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let st = Command::new(cargo)
            .args(["test", "-p", "kim", "--test", "cli_contract"])
            .env("KIM_BLESS", "1")
            .current_dir(root)
            .status()
            .map_err(|e| e.to_string())?;
        if !st.success() {
            return Err("re-blessing Kim's golden files failed".into());
        }
    }
    check(root)
}

/// The rules `bump` keeps; `ci` runs it.
pub fn check(root: &Path) -> Result<(), String> {
    let pkgs = workspace(root)?;
    let mut problems = Vec::new();
    let mut by_group: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for p in pkgs.values() {
        by_group
            .entry(group_of(&p.name))
            .or_default()
            .insert(&p.version);
        for f in ["src/capabilities.json", "src/hotkeys.manifest.json"] {
            if let Ok(text) = std::fs::read_to_string(p.dir.join(f)) {
                let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{f}: {e}"))?;
                if let Some(v) = doc["version"].as_str()
                    && v != p.version
                {
                    problems.push(format!(
                        "{}/{f} says {v}, the crate is {}",
                        p.name, p.version
                    ));
                }
            }
        }
    }
    for (g, f) in EXAMPLES {
        let want = pkgs
            .values()
            .find(|p| group_of(&p.name) == g)
            .map(|p| p.version.clone());
        let have = std::fs::read_to_string(root.join(f))
            .ok()
            .and_then(|t| example_version(&t));
        if want != have {
            problems.push(format!("{f} says {have:?}, {g} is {want:?}"));
        }
    }
    for (g, vs) in &by_group {
        if vs.len() > 1 {
            problems.push(format!("group {g} has several versions: {vs:?}"));
        }
    }
    for m in manifests(root) {
        let text = std::fs::read_to_string(&m).unwrap_or_default();
        for p in pkgs.values() {
            let re = regex::Regex::new(&format!(
                r#"(?m)^\s*{} = \{{ version = "([^"]+)""#,
                regex::escape(&p.name)
            ))
            .expect("valid regex");
            for c in re.captures_iter(&text) {
                if c[1] != *p.version {
                    problems.push(format!(
                        "{} asks for {} {}, the crate is {}",
                        m.display(),
                        p.name,
                        &c[1],
                        p.version
                    ));
                }
            }
        }
    }
    if problems.is_empty() {
        println!("version-check: {} groups consistent: PASS", by_group.len());
        Ok(())
    } else {
        Err(format!(
            "version-check: FAIL\n  {}\n  (run `cargo xtask bump`, ADR 0097)",
            problems.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_step() {
        assert_eq!(next_patch("0.0.1").unwrap(), "0.0.2");
        assert_eq!(next_patch("0.0.9").unwrap(), "0.0.10");
        assert_eq!(next_patch("0.1.0").unwrap(), "0.1.1");
        assert_eq!(next_patch("1.2.3-preview.4").unwrap(), "1.2.4");
        assert!(next_patch("1.2").is_err());
    }

    #[test]
    fn only_the_package_version_and_the_path_requirements_change() {
        let toml = "[package]\nname = \"kim\"\nversion = \"0.0.1\"\n\n[dependencies]\nregex = { version = \"0.0.1\" }\nkim-ui = { version = \"0.0.1\", path = \"../kim-ui\" }\n";
        let t = replace_package_version(toml, "0.0.2");
        assert!(t.contains("version = \"0.0.2\"\n\n[dependencies]"));
        assert!(t.contains("regex = { version = \"0.0.1\" }"));
        let t = replace_dep_requirement(&t, "kim-ui", "0.0.1", "0.0.5");
        assert!(t.contains("kim-ui = { version = \"0.0.5\", path"));
        assert!(t.contains("regex = { version = \"0.0.1\" }"));
        assert_eq!(package_version(&t).as_deref(), Some("0.0.2"));
    }

    #[test]
    fn a_json_version_moves_only_when_it_was_the_old_one() {
        let j = "{\n  \"schema\": \"x\",\n  \"version\": \"0.0.1\"\n}\n";
        assert!(
            json_version_line(j, "0.0.1", "0.0.2")
                .unwrap()
                .contains("\"0.0.2\"")
        );
        assert!(json_version_line(j, "0.1.0", "0.1.1").is_none());
    }

    #[test]
    fn groups_and_versions_compare() {
        assert_eq!(group_of("noeul-settings"), "noeul");
        assert_eq!(group_of("kim-ui"), "kim-ui");
        assert_eq!(cmp_version("0.0.10", "0.0.9"), std::cmp::Ordering::Greater);
    }
}
