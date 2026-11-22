//! The settings applied to the window, live (Step 105, `JARAK-FR-130…132`; the registry and the
//! file: `jarak_core::settings`).
//!
//! Every change goes to `settings.toml` first (atomically) and is then read back and applied, so
//! what the window shows is always what the file says. The window also reads the file again when
//! it gets the focus and the file changed (`jarak settings set` while it runs).
//!
//! - Language (unless `--lang` fixed it for this run) and theme (System follows Windows' app mode,
//!   read again when the window gets the focus) apply at once.
//! - The editor font is installed as its own family (`view::EDITOR_FONT`) from the Windows fonts
//!   folder or the user's fonts folder, with the built-in monospace chain behind it for the
//!   characters it lacks (Hangul); a font the PC does not have is said and the built-in one is used.
//! - Tab width and *Tab inserts spaces* apply to every open document; word wrap applies to every
//!   document when the setting changes and to each new one; *Markdown files open as* to each
//!   Markdown document opened (not to a restored session's, which keeps its own mode).

use crate::tabs::BarOptions;
use crate::view::{EDITOR_FONT, Look};
use crate::{JarakApp, Mode};
use eframe::egui;
use jarak_core::document::Eol;
use jarak_core::settings::Settings;
use kim_design::{Lang, Theme, ThemePref};
use std::path::PathBuf;

/// The window's settings, as read from the file (defaults for what it does not set).
#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub language: String,
    pub theme: ThemePref,
    pub menu_always: bool,
    pub status_bar: bool,
    pub tabs: BarOptions,
    pub single_window: bool,
    pub tab_width: usize,
    pub use_spaces: bool,
    pub word_wrap: bool,
    pub look: Look,
    pub font_family: String,
    pub markdown_open_as: String,
    pub image_folder: String,
    pub new_eol: Eol,
    pub recent_max: usize,
    pub recovery_interval: f64,
    /// Step 106: the print settings (Files → Printing).
    pub print: jarak_core::print::Setup,
    /// Step 109: a document larger than this (bytes) is a large file.
    pub large_file_bytes: usize,
    /// Step 110: the text's context menu (command ids, `-` separators).
    pub context_menu: Vec<String>,
    /// Step 110: smart highlighting and its options, the find field, the Open dialog's folder,
    /// new documents' language, a backup on save.
    pub smart_highlight: bool,
    pub smart_match_case: bool,
    pub smart_whole_word: bool,
    pub find_fill_selection: bool,
    /// `document`, `last` or `folder` (then `open_folder_path`).
    pub open_folder: String,
    pub open_folder_path: String,
    pub new_language: String,
    /// `off`, `simple` or `dated`.
    pub backup: String,
    pub word_chars: String,
    /// Step 110 (SET-10): a language's own tab width and, when given, whether Tab inserts spaces.
    pub lang_tabs: Vec<(String, usize, Option<bool>)>,
}

/// Step 110 (SET-10): `Python=4s, Go=8t, YAML=2` — a width from 1 to 16, `s` spaces, `t` a tab
/// character; entries that do not read are skipped.
pub fn parse_lang_tabs(s: &str) -> Vec<(String, usize, Option<bool>)> {
    s.split(',')
        .filter_map(|e| {
            let (name, v) = e.split_once('=')?;
            let name = name.trim();
            let v = v.trim();
            let (num, spaces) = match v.strip_suffix(['s', 'S']) {
                Some(n) => (n, Some(true)),
                None => match v.strip_suffix(['t', 'T']) {
                    Some(n) => (n, Some(false)),
                    None => (v, None),
                },
            };
            let w: usize = num.trim().parse().ok().filter(|w| (1..=16).contains(w))?;
            (!name.is_empty()).then(|| (name.to_string(), w, spaces))
        })
        .collect()
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs::from_get(&|k| jarak_core::settings::find(k).map(|s| s.default.to_string()))
    }
}

impl Prefs {
    /// The settings in the file (each one's default when the file does not set it).
    pub fn read(st: &Settings) -> Prefs {
        Prefs::from_get(&|k| st.get(k))
    }

    fn from_get(get: &dyn Fn(&str) -> Option<String>) -> Prefs {
        let g = |k: &str| get(k).unwrap_or_default();
        let b = |k: &str| g(k) == "true";
        let n = |k: &str, lo: i64, hi: i64| g(k).parse::<i64>().unwrap_or(lo).clamp(lo, hi);
        #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
        Prefs {
            language: g("language"),
            theme: ThemePref::parse(&g("theme")).unwrap_or_default(),
            menu_always: g("menu_bar") == "always",
            status_bar: b("status_bar"),
            tabs: BarOptions {
                close_all: b("tab_close_buttons"),
                double_click_close: b("tab_double_click_close"),
            },
            single_window: b("single_window"),
            tab_width: n("tab_width", 1, 16) as usize,
            use_spaces: b("use_spaces"),
            word_wrap: b("word_wrap"),
            look: Look {
                font_size: n("font_size", 8, 40) as f32,
                custom_font: g("font_family") != "default",
                line_numbers: b("line_numbers"),
                highlight_line: b("highlight_current_line"),
                caret_blink: b("caret_blink"),
                multi_editing: b("multi_editing"),
            },
            font_family: g("font_family"),
            markdown_open_as: g("markdown_open_as"),
            image_folder: g("image_folder"),
            new_eol: match g("new_eol").as_str() {
                "lf" => Eol::Lf,
                "cr" => Eol::Cr,
                _ => Eol::CrLf,
            },
            recent_max: n("recent_max", 0, 50) as usize,
            recovery_interval: n("recovery_interval", 0, 3600) as f64,
            large_file_bytes: n("large_file_mb", 1, 4096) as usize * 1024 * 1024,
            smart_highlight: b("smart_highlight"),
            smart_match_case: b("smart_match_case"),
            smart_whole_word: b("smart_whole_word"),
            find_fill_selection: b("find_fill_selection"),
            open_folder: g("open_folder"),
            open_folder_path: g("open_folder_path").trim().to_string(),
            new_language: g("new_language").trim().to_string(),
            backup: g("backup"),
            word_chars: g("word_chars"),
            lang_tabs: parse_lang_tabs(&g("language_tabs")),
            context_menu: {
                let v = g("context_menu");
                let v = if v.trim().is_empty() {
                    crate::modes::DEFAULT_CONTEXT_MENU.to_string()
                } else {
                    v
                };
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            },
            print: jarak_core::settings::print_setup(get, n("tab_width", 1, 16) as usize),
        }
    }

    /// The theme to show (System: Windows' app mode now).
    pub fn theme(&self) -> Theme {
        match self.theme {
            ThemePref::System => Theme::system(),
            ThemePref::Dark => Theme::Dark,
            ThemePref::Light => Theme::Light,
        }
    }

    /// The window's language (System: Windows' language).
    pub fn lang(&self) -> Lang {
        match self.language.as_str() {
            "en" => Lang::En,
            "ko" => Lang::Ko,
            _ => Lang::system(),
        }
    }
}

/// The file of an editor font choice on this PC (the Windows fonts folder, then the user's).
pub fn font_file(id: &str) -> Option<PathBuf> {
    let names: &[&str] = match id {
        "consolas" => &["consola.ttf"],
        "cascadia-mono" => &["CascadiaMono.ttf"],
        "courier-new" => &["cour.ttf"],
        "d2coding" => &["D2Coding.ttf", "D2Coding.ttc"],
        _ => return None,
    };
    let mut dirs = vec![
        std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| r"C:\Windows".into())
            .join("Fonts"),
    ];
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(l).join(r"Microsoft\Windows\Fonts"));
    }
    for d in &dirs {
        for n in names {
            let p = d.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
        // D2Coding installs with its version in the name (D2Coding-Ver1.3.2-20180524.ttf).
        if id == "d2coding"
            && let Ok(rd) = std::fs::read_dir(d)
        {
            let mut found: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    n.starts_with("d2coding")
                        && !n.contains("bold")
                        && !n.contains("ligature")
                        && (n.ends_with(".ttf") || n.ends_with(".ttc"))
                })
                .collect();
            found.sort();
            if let Some(p) = found.into_iter().next() {
                return Some(p);
            }
        }
    }
    None
}

impl JarakApp {
    /// Reads every setting from the file again and applies it to the window and its documents.
    pub fn apply_settings(&mut self) {
        let Some(path) = self.settings.clone() else {
            return;
        };
        self.read_settings_file(&path);
        let old = self.prefs.clone();
        let p = Prefs::read(&Settings::new(&path));
        for t in &mut self.tabs {
            t.file.doc.tab_width = p.tab_width;
            t.file.doc.use_spaces = p.use_spaces;
            if p.word_wrap != old.word_wrap {
                t.view.wrap = p.word_wrap;
            }
            t.view.look = p.look;
        }
        if !self.lang_fixed {
            self.lang = p.lang();
        }
        if let Some(r) = self.recovery.as_mut() {
            r.interval = p.recovery_interval;
            r.on = p.recovery_interval > 0.0;
        }
        self.recent.truncate(p.recent_max);
        self.theme_now = Some(p.theme());
        self.prefs = p;
        self.settings_stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    }

    /// Each frame: the theme and the editor font when they change, each new document's
    /// settings, and the file read again when the window gets the focus after it changed.
    pub(crate) fn prefs_frame(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.focused);
        if focused && !self.prefs_focused {
            // Windows' app mode may have changed; `jarak settings set` may have written the file.
            if self.prefs.theme == ThemePref::System {
                self.theme_now = Some(Theme::system());
            }
            self.settings_changed_outside();
            // `jarak hotkeys set` may have changed the shortcuts too.
            self.hotkeys_changed_outside();
        }
        self.prefs_focused = focused;
        let theme = self.theme_now.unwrap_or_else(|| self.prefs.theme());
        if self.applied_theme != Some(theme) {
            kim_ui::set_theme(ctx, theme);
            self.applied_theme = Some(theme);
            self.theme_now = Some(theme);
        }
        if self.applied_font.as_deref() != Some(self.prefs.font_family.as_str()) {
            self.install_editor_font(ctx);
        }
        // Step 110 (SET-17): the further word characters, on the thread that edits.
        jarak_core::text::set_word_chars(&self.prefs.word_chars);
        let p = &self.prefs;
        for t in &mut self.tabs {
            t.file.doc.tab_width = p.tab_width;
            t.file.doc.use_spaces = p.use_spaces;
            if t.view.look != p.look {
                t.view.look = p.look;
            }
            if t.fresh {
                t.fresh = false;
                t.view.wrap = p.word_wrap;
                if t.language() == "Markdown" && t.mode == Mode::Source {
                    match p.markdown_open_as.as_str() {
                        "split" => t.set_mode(Mode::Split),
                        "preview" => t.set_mode(Mode::Preview),
                        _ => {}
                    }
                }
            }
        }
        // Step 110 (SET-10): a language's own tab settings, over the general ones above.
        if !self.prefs.lang_tabs.is_empty() {
            for i in 0..self.tabs.len() {
                let Some(name) = self.lang_for(i).map(|l| l.name.clone()) else {
                    continue;
                };
                let Some((_, w, sp)) = self
                    .prefs
                    .lang_tabs
                    .iter()
                    .find(|(l, ..)| l.eq_ignore_ascii_case(&name))
                    .cloned()
                else {
                    continue;
                };
                let d = &mut self.tabs[i].file.doc;
                d.tab_width = w;
                if let Some(sp) = sp {
                    d.use_spaces = sp;
                }
            }
        }
        let max = self.prefs.recent_max;
        if self.recent.len() > max {
            self.recent.truncate(max);
        }
    }

    /// Reads the settings file again when it changed since the window last read it (`jarak
    /// settings set` while the window runs); `true` when it did.
    pub fn settings_changed_outside(&mut self) -> bool {
        let Some(path) = self.settings.clone() else {
            return false;
        };
        let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if stamp == self.settings_stamp {
            return false;
        }
        self.apply_settings();
        true
    }

    /// Installs the settings' editor font as `EDITOR_FONT` (the built-in monospace chain behind
    /// it), or says the PC does not have it and keeps the built-in one.
    fn install_editor_font(&mut self, ctx: &egui::Context) {
        let id = self.prefs.font_family.clone();
        self.applied_font = Some(id.clone());
        if id == "default" {
            return;
        }
        let Some(file) = font_file(&id) else {
            self.prefs.look.custom_font = false;
            self.message = Some(format!(
                "{} {id} {}",
                self.t("The font", "글꼴"),
                self.t(
                    "is not on this PC; the built-in font is used.",
                    "이(가) 이 PC에 없어 기본 글꼴을 씁니다."
                )
            ));
            return;
        };
        let Ok(bytes) = std::fs::read(&file) else {
            self.prefs.look.custom_font = false;
            return;
        };
        let mut defs = ctx.fonts(|f| f.definitions().clone());
        let name = format!("jarak-editor-{id}");
        defs.font_data.insert(
            name.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        let mut chain = vec![name];
        chain.extend(
            defs.families
                .get(&egui::FontFamily::Monospace)
                .cloned()
                .unwrap_or_default(),
        );
        defs.families
            .insert(egui::FontFamily::Name(EDITOR_FONT.into()), chain);
        ctx.set_fonts(defs);
    }
}
