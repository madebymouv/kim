//! The UI preferences every Kim program shares (ADR 0099): `%APPDATA%\mouv\ui.json`.
//!
//! ```json
//! { "schema": "mouv.ui", "schema_version": 1, "zoom": 150 }
//! ```
//!
//! `zoom` is the UI zoom in percent (`null` or missing: *Auto*, [`crate::scale::auto_zoom`]).
//! Kim Manager's Settings → General → Size writes it; Subak, Noeul's settings window and the
//! Ddalgak rule editor read it when a window opens. A missing or unreadable file means Auto.

use std::path::{Path, PathBuf};

/// The shared preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiPrefs {
    /// The UI zoom in percent; `None` is Auto.
    pub zoom: Option<u16>,
}

impl UiPrefs {
    /// `%APPDATA%\mouv\ui.json` (`None` without `APPDATA`).
    pub fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| Path::new(&a).join("mouv").join("ui.json"))
    }

    /// The preferences in `path`; defaults when missing or unreadable.
    pub fn load_from(path: &Path) -> UiPrefs {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| UiPrefs::parse(&s))
            .unwrap_or_default()
    }

    /// The shared preferences of this user.
    pub fn load() -> UiPrefs {
        UiPrefs::path().map_or_else(UiPrefs::default, |p| UiPrefs::load_from(&p))
    }

    /// Reads the JSON text (unknown keys are ignored, a wrong `schema` gives defaults).
    pub fn parse(s: &str) -> UiPrefs {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(s) else {
            return UiPrefs::default();
        };
        if v.get("schema").and_then(|s| s.as_str()) != Some("mouv.ui") {
            return UiPrefs::default();
        }
        UiPrefs {
            zoom: v
                .get("zoom")
                .and_then(serde_json::Value::as_u64)
                .and_then(|z| u16::try_from(z).ok())
                .map(|z| z.clamp(100, 200)),
        }
    }

    /// The JSON text.
    pub fn to_json(self) -> String {
        let v = serde_json::json!({
            "schema": "mouv.ui",
            "schema_version": 1,
            "zoom": self.zoom,
        });
        let mut s = serde_json::to_string_pretty(&v).unwrap_or_default();
        s.push('\n');
        s
    }

    /// Writes `path` (its folder is created; written to a temporary file and renamed, so a
    /// reader never sees half a file).
    pub fn save_to(self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_round_trips_and_bad_files_mean_auto() {
        let p = UiPrefs { zoom: Some(150) };
        assert_eq!(UiPrefs::parse(&p.to_json()), p);
        assert_eq!(
            UiPrefs::parse(&UiPrefs::default().to_json()),
            UiPrefs::default()
        );
        assert_eq!(UiPrefs::parse("not json"), UiPrefs::default());
        assert_eq!(
            UiPrefs::parse(r#"{"zoom":150}"#),
            UiPrefs::default(),
            "no schema"
        );
        assert_eq!(
            UiPrefs::parse(r#"{"schema":"mouv.ui","zoom":900}"#).zoom,
            Some(200),
            "kept in range"
        );
        let dir = std::env::temp_dir().join(format!("kim-ui-prefs-{}", std::process::id()));
        let file = dir.join("mouv").join("ui.json");
        p.save_to(&file).unwrap();
        assert_eq!(UiPrefs::load_from(&file), p);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
