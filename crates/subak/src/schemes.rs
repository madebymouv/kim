//! #98 T4 (`PLUG-05`, `CORE-38`): terminal colour schemes as plain data files.
//!
//! A scheme is the Windows Terminal JSON shape — `name`, `foreground`, `background`,
//! `cursorColor`, `selectionBackground` and the sixteen ANSI colours `black` … `brightWhite` as
//! `#RRGGBB` — so a scheme copied from a Windows Terminal `settings.json` (or one of the many
//! published in that format) works as it is. A file holds one scheme, a list of them, or a
//! settings fragment `{"schemes": [...]}`. Subak ships Windows Terminal's default schemes (MIT,
//! [`NOTICE`]) and reads the user's own from `%APPDATA%\mouv\subak\schemes\*.json`; a user scheme
//! with the name of a bundled one replaces it. Nothing here runs code: a scheme is colours only.
//!
//! `appearance.color_scheme` picks one by name (`kim`, the default, keeps the theme's own terminal
//! colours); Windows High Contrast always wins. The selection is the scheme's selection colour
//! laid over the background at 30 % (Windows Terminal draws it translucent), and search matches
//! keep Kim's tokens for a dark or light background.

use serde_json::Value;
use std::path::{Path, PathBuf};
use subak_term::render::theme::{Rgb, Theme};

/// The bundled schemes (Windows Terminal's defaults).
const BUNDLED: &str = include_str!("schemes/windows-terminal.json");
/// Their licence notice (packaged in `THIRD-PARTY-NOTICES.txt`).
pub const NOTICE: &str = include_str!("schemes/NOTICE.txt");
/// The value of `appearance.color_scheme` that keeps the theme's own terminal colours.
pub const KIM: &str = "kim";
/// The largest scheme file read, and the most schemes one file may hold.
const MAX_FILE: u64 = 256 * 1024;
const MAX_SCHEMES: usize = 200;
/// The sixteen ANSI colour keys in order.
const ANSI_KEYS: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "purple",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightPurple",
    "brightCyan",
    "brightWhite",
];

/// One colour scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    pub name: String,
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Option<Rgb>,
    pub selection: Option<Rgb>,
    pub ansi: [Rgb; 16],
    /// Where it came from: `bundled` or the file's path.
    pub source: String,
}

impl Scheme {
    /// The background is dark (relative luminance below one half).
    pub fn is_dark(&self) -> bool {
        luminance(self.background) < 0.5
    }

    /// The terminal colours this scheme draws with.
    pub fn theme(&self) -> Theme {
        let base = if self.is_dark() {
            Theme::kim_dark()
        } else {
            Theme::kim_light()
        };
        let sel = self.selection.unwrap_or(self.foreground);
        Theme {
            name: self.name.clone(),
            foreground: self.foreground,
            background: self.background,
            cursor: self.cursor.unwrap_or(self.foreground),
            // 30 % of the selection colour over the background (77 / 255).
            selection: self.background.mix(sel, 77),
            ansi: self.ansi,
            ..base
        }
    }
}

/// WCAG relative luminance of an sRGB colour.
fn luminance(c: Rgb) -> f64 {
    let lin = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.040_45 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2)
}

/// The schemes in `text`: one scheme object, a list of them, or `{"schemes": [...]}`.
pub fn parse(text: &str, source: &str) -> Result<Vec<Scheme>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let list = match &v {
        Value::Array(a) => a.clone(),
        Value::Object(o) if o.contains_key("schemes") => match &o["schemes"] {
            Value::Array(a) => a.clone(),
            _ => return Err("`schemes` must be a list of schemes".into()),
        },
        Value::Object(_) => vec![v.clone()],
        _ => {
            return Err(
                "a scheme file holds a scheme, a list of them or {\"schemes\": [...]}".into(),
            );
        }
    };
    if list.len() > MAX_SCHEMES {
        return Err(format!("more than {MAX_SCHEMES} schemes in one file"));
    }
    list.iter()
        .enumerate()
        .map(|(i, s)| one(s, source).map_err(|e| format!("scheme {}: {e}", i + 1)))
        .collect()
}

fn one(v: &Value, source: &str) -> Result<Scheme, String> {
    let o = v.as_object().ok_or("a scheme must be an object")?;
    let name = o
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.chars().count() <= 64 && !n.chars().any(char::is_control))
        .ok_or("`name` must be a text of 1-64 characters")?
        .to_string();
    let color = |k: &str| -> Result<Option<Rgb>, String> {
        match o.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Rgb::hex(s.trim())
                .map(Some)
                .ok_or_else(|| format!("`{name}`: `{k}` must be #RRGGBB")),
            Some(_) => Err(format!("`{name}`: `{k}` must be #RRGGBB")),
        }
    };
    let need = |k: &str| -> Result<Rgb, String> {
        color(k)?.ok_or_else(|| format!("`{name}` has no `{k}`"))
    };
    let (foreground, background) = (need("foreground")?, need("background")?);
    let mut ansi = [Rgb(0, 0, 0); 16];
    for (i, k) in ANSI_KEYS.iter().enumerate() {
        ansi[i] = need(k)?;
    }
    Ok(Scheme {
        foreground,
        background,
        cursor: color("cursorColor")?,
        selection: color("selectionBackground")?,
        ansi,
        source: source.to_string(),
        name,
    })
}

/// The schemes Subak ships.
pub fn bundled() -> Vec<Scheme> {
    parse(BUNDLED, "bundled").expect("the bundled schemes parse")
}

/// `%APPDATA%\mouv\subak\schemes`.
pub fn user_dir() -> Option<PathBuf> {
    crate::config::config_dir().map(|d| d.join("schemes"))
}

/// The user's schemes from `dir` (`*.json`, by file name), and a problem per file that was
/// not read.
pub fn read_dir(dir: &Path) -> (Vec<Scheme>, Vec<String>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (out, problems);
    };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();
    for f in files {
        let shown = f.display().to_string();
        match std::fs::metadata(&f) {
            Ok(m) if m.len() > MAX_FILE => {
                problems.push(format!("{shown}: larger than {} KiB", MAX_FILE / 1024));
                continue;
            }
            Err(e) => {
                problems.push(format!("{shown}: {e}"));
                continue;
            }
            _ => {}
        }
        match std::fs::read_to_string(&f)
            .map_err(|e| e.to_string())
            .and_then(|t| parse(&t, &shown))
        {
            Ok(s) => out.extend(s),
            Err(e) => problems.push(format!("{shown}: {e}")),
        }
    }
    (out, problems)
}

/// Every scheme by name: the bundled ones, replaced or joined by the user's (names compare
/// without case).
pub fn all(user: Vec<Scheme>) -> Vec<Scheme> {
    let mut out = bundled();
    for s in user {
        match out
            .iter_mut()
            .find(|b| b.name.eq_ignore_ascii_case(&s.name))
        {
            Some(b) => *b = s,
            None => out.push(s),
        }
    }
    out
}

/// The scheme called `name` (without case) among `schemes`.
pub fn find<'a>(schemes: &'a [Scheme], name: &str) -> Option<&'a Scheme> {
    schemes
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name.trim()))
}

/// The scheme `appearance.color_scheme` names, from the bundled ones and the user's folder;
/// `None` for `kim` or an unknown name (the theme's own colours are used then).
pub fn resolve(name: &str) -> Option<Scheme> {
    if name.trim().eq_ignore_ascii_case(KIM) || name.trim().is_empty() {
        return None;
    }
    let user = user_dir().map(|d| read_dir(&d).0).unwrap_or_default();
    find(&all(user), name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_schemes_parse_with_sixteen_distinct_enough_colours() {
        let b = bundled();
        let names: Vec<&str> = b.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Campbell",
                "Campbell Powershell",
                "Vintage",
                "One Half Dark",
                "One Half Light",
                "Solarized Dark",
                "Solarized Light",
                "Tango Dark",
                "Tango Light"
            ]
        );
        for s in &b {
            assert_eq!(s.source, "bundled");
            // The text stays readable on its own background (Solarized Light is 4.1:1 by design;
            // `appearance.minimum_contrast`, 4.5 by default, lifts text below it).
            let (a, c) = (luminance(s.foreground), luminance(s.background));
            let ratio = (a.max(c) + 0.05) / (a.min(c) + 0.05);
            assert!(
                ratio >= 4.0,
                "{}: foreground on background {ratio:.2}",
                s.name
            );
        }
        assert!(find(&b, "one half light").is_some_and(|s| !s.is_dark()));
        assert!(find(&b, "Campbell").is_some_and(Scheme::is_dark));
        assert!(NOTICE.contains("MIT License"));
    }

    #[test]
    fn a_scheme_file_can_hold_one_a_list_or_a_settings_fragment() {
        let all: Value = serde_json::from_str(BUNDLED).unwrap();
        let first = all[0].to_string();
        assert_eq!(parse(&first, "f").unwrap()[0].name, "Campbell");
        assert_eq!(parse(BUNDLED, "f").unwrap().len(), 9);
        let frag = format!("{{\"schemes\": {BUNDLED}, \"profiles\": {{}}}}");
        assert_eq!(parse(&frag, "f").unwrap().len(), 9);
        let c = &parse(&first, "f").unwrap()[0];
        assert_eq!(c.ansi[1], Rgb(0xC5, 0x0F, 0x1F));
        assert_eq!(c.cursor, Some(Rgb(255, 255, 255)));
    }

    #[test]
    fn a_bad_scheme_is_refused_with_its_name_and_key() {
        let e = parse(r##"{"name": "X", "foreground": "#FFFFFF"}"##, "f").unwrap_err();
        assert!(e.contains("`X` has no `background`"), "{e}");
        let e = parse(
            r##"{"name": "X", "foreground": "#FFF", "background": "#000000"}"##,
            "f",
        )
        .unwrap_err();
        assert!(e.contains("`foreground` must be #RRGGBB"), "{e}");
        assert!(parse("[1]", "f").unwrap_err().contains("must be an object"));
        assert!(parse("\"x\"", "f").is_err());
        assert!(parse("{", "f").unwrap_err().starts_with("not JSON"));
        assert!(parse(r##"{"name": "", "foreground": "#000000"}"##, "f").is_err());
    }

    #[test]
    fn user_schemes_join_or_replace_the_bundled_ones_and_bad_files_are_reported() {
        let d = std::env::temp_dir().join(format!("subak-schemes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let mut mine = bundled()[0].clone();
        mine.name = "Mine".into();
        let to_json = |s: &Scheme| {
            let h = |c: Rgb| format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2);
            let mut m = serde_json::Map::new();
            m.insert("name".into(), s.name.clone().into());
            m.insert("foreground".into(), h(s.foreground).into());
            m.insert("background".into(), h(s.background).into());
            for (i, k) in ANSI_KEYS.iter().enumerate() {
                m.insert((*k).into(), h(s.ansi[i]).into());
            }
            Value::Object(m).to_string()
        };
        std::fs::write(d.join("mine.json"), to_json(&mine)).unwrap();
        let mut campbell = bundled()[0].clone();
        campbell.background = Rgb(1, 2, 3);
        std::fs::write(d.join("campbell.JSON"), to_json(&campbell)).unwrap();
        std::fs::write(d.join("broken.json"), "{").unwrap();
        std::fs::write(d.join("notes.txt"), "not a scheme").unwrap();
        let (user, problems) = read_dir(&d);
        assert_eq!(user.len(), 2);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("broken.json"));
        let all = all(user);
        assert_eq!(all.len(), 10);
        assert_eq!(find(&all, "campbell").unwrap().background, Rgb(1, 2, 3));
        assert!(find(&all, "MINE").unwrap().source.ends_with("mine.json"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_scheme_draws_with_its_colours_a_translucent_selection_and_kims_search_tokens() {
        let b = bundled();
        let dark = find(&b, "Campbell").unwrap().theme();
        assert_eq!(dark.background, Rgb(0x0C, 0x0C, 0x0C));
        assert_eq!(dark.ansi[9], Rgb(0xE7, 0x48, 0x56));
        assert_eq!(dark.search_match, Theme::kim_dark().search_match);
        // 30 % white over #0C0C0C.
        assert_eq!(
            dark.selection,
            Rgb(0x0C, 0x0C, 0x0C).mix(Rgb(255, 255, 255), 77)
        );
        let light = find(&b, "Solarized Light").unwrap().theme();
        assert_eq!(light.search_match, Theme::kim_light().search_match);
        assert_eq!(light.cursor, Rgb(0x00, 0x2B, 0x36));
        assert!(resolve("kim").is_none());
        assert!(resolve("no such scheme").is_none());
    }
}
