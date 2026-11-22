//! Shared harness for the e2e suite.
//!
//! Every e2e test is `#[ignore]`d and additionally calls [`require_e2e`], so it can only run
//! through `cargo xtask e2e`, which enforces the "once per 15 Step issues" cadence and points
//! the tests at freshly built release binaries.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Panics unless the suite was started by `cargo xtask e2e`.
pub fn require_e2e() {
    assert_eq!(
        std::env::var("KIM_E2E").as_deref(),
        Ok("1"),
        "e2e tests run only via `cargo xtask e2e` (AGENTS.md §4)"
    );
}

/// Path of a release binary built by `cargo xtask e2e`.
pub fn bin(name: &str) -> PathBuf {
    require_e2e();
    let dir = std::env::var_os("KIM_E2E_BIN_DIR").expect("KIM_E2E_BIN_DIR is set by xtask");
    let path = Path::new(&dir).join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(path.exists(), "missing e2e binary {}", path.display());
    path
}

/// Path of a *debug* binary built by `cargo xtask e2e`. Debug builds accept a development
/// catalog key from `KIM_DEV_TRUSTED_KEY` (compiled out of release builds), which scenarios need
/// to install test-signed packages.
pub fn bin_debug(name: &str) -> PathBuf {
    require_e2e();
    let dir =
        std::env::var_os("KIM_E2E_DEBUG_BIN_DIR").expect("KIM_E2E_DEBUG_BIN_DIR is set by xtask");
    let path = Path::new(&dir).join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(path.exists(), "missing e2e debug binary {}", path.display());
    path
}

/// Why the release `jarak-gui` cannot open a window on this machine, or `None` when it can: it is
/// started once (per test process) on a small document and asked to close after its first frame;
/// exit code 6 with its stderr means no window could be made (no OpenGL 2 driver, as on the
/// Daily's GitHub runner without a GPU). The window scenarios skip there, saying why.
#[cfg(windows)]
pub fn jarak_window_unavailable() -> Option<String> {
    use std::process::Stdio;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};
    static WHY: OnceLock<Option<String>> = OnceLock::new();
    WHY.get_or_init(|| {
        let dir = TempDir::new("jarak-window-probe");
        let doc = dir.path().join("probe.txt");
        std::fs::write(&doc, "probe\n").ok()?;
        let err = dir.path().join("stderr.txt");
        let file = std::fs::File::create(&err).ok()?;
        let mut child = Command::new(bin("jarak-gui"))
            .args([
                "--instance",
                &format!("e2e-probe-{}", std::process::id()),
                "--new-window",
                "--no-session",
                "--lang",
                "en",
            ])
            .arg(&doc)
            .env("APPDATA", dir.path().join("Roaming"))
            .env("LOCALAPPDATA", dir.path().join("Local"))
            .env("KIM_UI_INSPECT", dir.path().join("probe.json"))
            .env("KIM_UI_EXIT", "1")
            .env("KIM_UI_SCREEN", "editor")
            .env("KIM_UI_POS", "40,40")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(file)
            .spawn()
            .ok()?;
        let until = Instant::now() + Duration::from_secs(40);
        let status = loop {
            if let Ok(Some(s)) = child.try_wait() {
                break Some(s);
            }
            if Instant::now() > until {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        match status {
            Some(s) if s.code() == Some(6) => {
                let why = std::fs::read_to_string(&err).unwrap_or_default();
                Some(if why.trim().is_empty() {
                    "jarak-gui exited with code 6".to_string()
                } else {
                    why.trim().to_string()
                })
            }
            // Any other failure is not skipped (the scenarios then fail), but its reason goes to
            // the log so a Daily run shows it.
            Some(s) if !s.success() => {
                eprintln!(
                    "jarak-gui window probe: {s}: {}",
                    std::fs::read_to_string(&err).unwrap_or_default().trim()
                );
                None
            }
            _ => None,
        }
    })
    .clone()
}

/// Whether a program named `exe` (e.g. `noeul.exe`) runs on this PC: a scenario that checks
/// what happens when a tool's own instance is *not* running cannot check it on a developer PC
/// where the user runs that tool (the instance answers on the per-user default endpoint). Read
/// only (`tasklist`); the CI runner runs none of them.
#[cfg(windows)]
pub fn process_running(exe: &str) -> bool {
    use std::os::windows::process::CommandExt;
    Command::new("tasklist")
        .args(["/FO", "CSV", "/NH", "/FI", &format!("IMAGENAME eq {exe}")])
        .creation_flags(0x0800_0000)
        .output()
        .is_ok_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_lowercase()
                .contains(&exe.to_lowercase())
        })
}

/// A unique, empty directory under the system temp dir, removed on drop.
#[derive(Debug)]
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let path =
            std::env::temp_dir().join(format!("kim-e2e-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Number of entries (recursively) below the directory.
    pub fn entry_count(&self) -> usize {
        fn count(dir: &Path) -> usize {
            std::fs::read_dir(dir).map_or(0, |entries| {
                entries
                    .flatten()
                    .map(|e| {
                        1 + if e.path().is_dir() {
                            count(&e.path())
                        } else {
                            0
                        }
                    })
                    .sum()
            })
        }
        count(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs a binary with its per-user data locations redirected into `sandbox`, so a test can
/// prove the command wrote nothing there.
pub fn run_sandboxed(binary: &Path, args: &[&str], sandbox: &TempDir) -> Output {
    let p = sandbox.path();
    Command::new(binary)
        .args(args)
        .env("APPDATA", p.join("Roaming"))
        .env("LOCALAPPDATA", p.join("Local"))
        .env("USERPROFILE", p)
        .env("HOME", p)
        .env("XDG_CONFIG_HOME", p.join(".config"))
        .env("XDG_DATA_HOME", p.join(".local/share"))
        .output()
        .expect("run e2e binary")
}
