//! Where Jarak keeps its own files (docs/JARAK_PHILOSOPHY.md §5). Documents stay wherever the
//! user keeps them; these are only Jarak's settings, session and recovery files.
//!
//! - `%APPDATA%\mouv\jarak\` (roaming): `settings.toml`, `hotkeys.json`.
//! - `%LOCALAPPDATA%\mouv\jarak\` (this PC only, `JARAK-SEC-008`): `session.json`, `recovery\`,
//!   `handoff\` (files a second start passes to the running window, `JARAK-FR-002`).

use std::path::{Path, PathBuf};

/// Jarak's per-user folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// Roaming settings: `%APPDATA%\mouv\jarak`.
    pub config: PathBuf,
    /// Local state: `%LOCALAPPDATA%\mouv\jarak`.
    pub local: PathBuf,
}

impl Paths {
    /// The folders under the given base folders (`APPDATA`, `LOCALAPPDATA`).
    pub fn under(appdata: &Path, localappdata: &Path) -> Paths {
        Paths {
            config: appdata.join("mouv").join("jarak"),
            local: localappdata.join("mouv").join("jarak"),
        }
    }

    /// The folders from the environment; `None` when `APPDATA` or `LOCALAPPDATA` is not set.
    pub fn from_env() -> Option<Paths> {
        let base = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Some(Paths::under(&base("APPDATA")?, &base("LOCALAPPDATA")?))
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    pub fn hotkeys_file(&self) -> PathBuf {
        self.config.join("hotkeys.json")
    }

    pub fn session_file(&self) -> PathBuf {
        self.local.join("session.json")
    }

    pub fn recovery_dir(&self) -> PathBuf {
        self.local.join("recovery")
    }

    /// Step 95: the Replace in Files journal (the originals of each batch, for undo).
    pub fn journal_dir(&self) -> PathBuf {
        self.local.join("replace-journal")
    }

    /// Where a second `jarak-gui` start leaves the files for the running window.
    pub fn handoff_dir(&self) -> PathBuf {
        self.local.join("handoff")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roam_and_state_stays_local() {
        let p = Paths::under(Path::new(r"C:\U\Roaming"), Path::new(r"C:\U\Local"));
        assert_eq!(
            p.settings_file(),
            Path::new(r"C:\U\Roaming\mouv\jarak\settings.toml")
        );
        assert_eq!(
            p.hotkeys_file(),
            Path::new(r"C:\U\Roaming\mouv\jarak\hotkeys.json")
        );
        assert_eq!(
            p.session_file(),
            Path::new(r"C:\U\Local\mouv\jarak\session.json")
        );
        assert_eq!(
            p.recovery_dir(),
            Path::new(r"C:\U\Local\mouv\jarak\recovery")
        );
        assert_eq!(p.handoff_dir(), Path::new(r"C:\U\Local\mouv\jarak\handoff"));
    }
}
