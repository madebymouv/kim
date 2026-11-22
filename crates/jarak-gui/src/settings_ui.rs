//! The Settings window (Step 105, `JARAK-FR-130`, ADR 0100): a window of its own (a native
//! viewport titled *Jarak Settings*; an embedded window where the backend has one window, as in
//! the headless tests), opened from the gear in the status bar's corner, Ctrl+, or the palette's
//! *Settings…*; the gear brings an open window forward; Esc or the close button closes it.
//!
//! Sections: General, Editor, Appearance, Markdown, Images, Vim, Keyboard, Files, Advanced. Every
//! row comes from the registry (`jarak_core::settings::SETTINGS`): a switch, a choice (segmented
//! for up to three short choices, else a list), a number or a text field. A change is written to
//! `settings.toml` (checked, atomic) and then read back and applied (`JarakApp::apply_settings`);
//! a value the setting does not take is said beside it and nothing is written. Keyboard is the
//! shortcut mapper (`crate::shortcuts`).

use crate::JarakApp;
use eframe::egui;
use jarak_core::settings::{self as st, Kind, Section, Setting, Settings};
use kim_design::{size, space};
use std::collections::HashMap;

/// The window's state while it is open.
#[derive(Debug, Clone)]
pub struct SettingsWin {
    pub section: Section,
    /// Bring the window forward next frame.
    pub raise: bool,
    /// Text fields being typed in (by key), written when they lose the focus or on Enter.
    pub drafts: HashMap<&'static str, String>,
    /// The last refusal (key, why).
    pub error: Option<(String, String)>,
    /// *Reset every setting* was pressed once (the second press resets).
    pub confirm_reset: bool,
    /// The shortcut mapper's state.
    pub keys: crate::shortcuts::MapperState,
}

impl SettingsWin {
    pub fn new(section: Section) -> SettingsWin {
        SettingsWin {
            section,
            raise: true,
            drafts: HashMap::new(),
            error: None,
            confirm_reset: false,
            keys: crate::shortcuts::MapperState::default(),
        }
    }
}

impl JarakApp {
    /// Opens the Settings window (or brings it forward) at `section`.
    pub fn open_settings(&mut self, section: Option<Section>) {
        match self.settings_win.as_mut() {
            Some(w) => {
                w.raise = true;
                if let Some(s) = section {
                    w.section = s;
                }
            }
            None => {
                self.settings_win = Some(SettingsWin::new(section.unwrap_or(Section::General)));
            }
        }
    }

    /// Writes a setting and applies it; a refusal is kept for the row to show.
    pub fn change_setting(&mut self, key: &str, value: &str) -> bool {
        let Some(path) = self.settings.clone() else {
            self.message = Some(
                self.t(
                    "There is no settings file in this window; nothing was saved.",
                    "이 창에는 설정 파일이 없어 저장하지 않았습니다.",
                )
                .into(),
            );
            return false;
        };
        match Settings::new(&path).set(key, value) {
            Ok(v) => {
                if let Some(w) = self.settings_win.as_mut() {
                    w.error = None;
                }
                if key == "editing_mode" {
                    let e = if v == "vim" {
                        crate::Editing::Vim
                    } else {
                        crate::Editing::Standard
                    };
                    if e != self.editing {
                        self.set_editing(e);
                    }
                }
                self.apply_settings();
                true
            }
            Err(e) => {
                let why = match &e {
                    st::SetError::Refused(st::Refused::InvalidValue { why, .. }) => why.clone(),
                    other => other.to_string(),
                };
                if let Some(w) = self.settings_win.as_mut() {
                    w.error = Some((key.to_string(), why));
                } else {
                    self.message = Some(e.to_string());
                }
                false
            }
        }
    }

    /// Every setting back to its default (the file keeps lines Jarak does not know).
    pub fn reset_all_settings(&mut self) {
        let Some(path) = self.settings.clone() else {
            return;
        };
        let store = Settings::new(&path);
        let mut failed = None;
        for s in st::SETTINGS {
            if store.raw(s.key).is_some()
                && let Err(e) = store.reset(s.key)
            {
                failed = Some(e.to_string());
            }
        }
        self.apply_settings();
        if self.editing != crate::Editing::Standard {
            self.set_editing(crate::Editing::Standard);
        }
        self.message = Some(failed.unwrap_or_else(|| {
            self.t(
                "Every setting is back to its default.",
                "모든 설정을 기본값으로 되돌렸습니다.",
            )
            .into()
        }));
    }

    /// The Settings window, while it is open.
    pub(crate) fn settings_window(&mut self, ctx: &egui::Context) {
        let Some(w) = self.settings_win.as_mut() else {
            return;
        };
        let raise = std::mem::take(&mut w.raise);
        let title = self.t("Jarak Settings", "자락 설정").to_string();
        let mut vb = egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([size::SETTINGS_WINDOW_WIDTH, size::SETTINGS_WINDOW_HEIGHT])
            .with_min_inner_size([560.0, 420.0]);
        // An audit photographs the window from the screen: beside the main window, never under it.
        if let Some(p) = kim_ui::inspect::audit_position() {
            let right = ctx
                .input(|i| i.viewport().outer_rect)
                .map_or(p[0] + 1000.0, |r| r.right() + 24.0);
            vb = vb.with_position([right, p[1]]);
        }
        let audit = self.audit_settings;
        let screen = self.audit_screen.clone();
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("jarak-settings"),
            vb,
            |ui, class| {
                let ctx = ui.ctx().clone();
                let native = class != egui::ViewportClass::EmbeddedWindow;
                if raise && native {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                if audit && native {
                    kim_ui::inspect::clear();
                }
                if ctx.input(|i| i.viewport().close_requested()) {
                    self.settings_win = None;
                    return;
                }
                let c = kim_ui::Colors::get(&ctx);
                if !native {
                    ui.set_min_size(egui::vec2(size::SETTINGS_WINDOW_WIDTH - 40.0, 420.0));
                }
                egui::Frame::new().fill(c.bg_base).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    self.settings_panel(ui);
                });
                if audit && native {
                    kim_ui::inspect::finish_frame_with(
                        &ctx,
                        "jarak",
                        &screen,
                        Some(&|p: &std::path::Path| kim_ui::inspect::capture_window_png(&title, p)),
                    );
                }
            },
        );
    }

    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        let c = kim_ui::Colors::get(ui.ctx());
        let lang = self.lang;
        let full = ui.max_rect();
        let nav = egui::Rect::from_min_size(
            full.min,
            egui::vec2(size::SETTINGS_NAV_WIDTH, full.height()),
        );
        ui.painter().rect_filled(nav, 0.0, c.bg_raised);
        let mut rail = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(nav.shrink2(egui::vec2(space::S, space::L)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        rail.horizontal(|ui| {
            ui.add_space(space::XS);
            kim_ui::title(ui, lang.pick("Settings", "설정"));
        });
        rail.add_space(space::M);
        rail.spacing_mut().item_spacing.y = 2.0;
        let current = self
            .settings_win
            .as_ref()
            .map_or(Section::General, |w| w.section);
        for s in Section::ALL {
            let (en, ko) = s.names();
            if kim_ui::nav_item(&mut rail, current == s, lang.pick(en, ko)).clicked()
                && let Some(w) = self.settings_win.as_mut()
            {
                w.section = s;
                w.error = None;
                w.confirm_reset = false;
                w.keys = crate::shortcuts::MapperState::default();
            }
        }
        let body = egui::Rect::from_min_max(egui::pos2(nav.right(), full.top()), full.max);
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body.shrink2(egui::vec2(space::L + 4.0, space::L)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let capturing = self
            .settings_win
            .as_ref()
            .is_some_and(|w| w.keys.capture.is_some());
        if page.input(|i| i.key_pressed(egui::Key::Escape)) && !capturing {
            self.settings_win = None;
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut page, |ui| {
                ui.set_width(ui.available_width() - space::XS);
                let (en, ko) = current.names();
                kim_ui::title(ui, lang.pick(en, ko));
                ui.add_space(space::S);
                if self.settings.is_none() {
                    kim_ui::notice(
                        ui,
                        kim_ui::Tone::Warning,
                        lang.pick(
                            "This window has no settings file: changes are not saved.",
                            "이 창에는 설정 파일이 없어 변경을 저장하지 않습니다.",
                        ),
                        None,
                        None,
                    );
                }
                if current == Section::Keyboard {
                    self.shortcut_mapper(ui);
                } else {
                    self.settings_rows(ui, current);
                }
                if current == Section::Advanced {
                    self.settings_advanced(ui);
                }
            });
    }

    /// The section's settings, in their groups.
    fn settings_rows(&mut self, ui: &mut egui::Ui, section: Section) {
        let lang = self.lang;
        let store = self.settings.as_deref().map(Settings::new);
        let mut groups: Vec<(&str, &str)> = Vec::new();
        for s in st::of(section) {
            if !groups.contains(&s.group) {
                groups.push(s.group);
            }
        }
        let mut change: Option<(&'static str, String)> = None;
        for g in groups {
            kim_ui::setting_group(ui, lang.pick(g.0, g.1), |ui| {
                let rows: Vec<&'static Setting> =
                    st::of(section).filter(|s| s.group == g).collect();
                for (i, s) in rows.iter().enumerate() {
                    if i > 0 {
                        kim_ui::row_divider(ui);
                    }
                    let value = store
                        .as_ref()
                        .and_then(|st| st.get(s.key))
                        .unwrap_or_else(|| s.default.to_string());
                    if let Some(v) = self.setting_row(ui, s, &value) {
                        change = Some((s.key, v));
                    }
                }
            });
        }
        if let Some((k, v)) = change {
            self.change_setting(k, &v);
        }
    }

    /// One row; `Some(value)` when the user changed it.
    fn setting_row(
        &mut self,
        ui: &mut egui::Ui,
        s: &'static Setting,
        value: &str,
    ) -> Option<String> {
        let lang = self.lang;
        let title = lang.pick(s.en, s.ko);
        let help = lang.pick(s.help_en, s.help_ko);
        let help = (!help.is_empty()).then_some(help);
        let error = self
            .settings_win
            .as_ref()
            .and_then(|w| w.error.as_ref())
            .filter(|(k, _)| k == s.key)
            .map(|(_, why)| why.clone());
        let mut out = None;
        match s.kind {
            Kind::Bool => {
                let mut on = value == "true";
                let before = on;
                kim_ui::setting_row(ui, title, help, |ui| {
                    ui.add(kim_ui::Switch::new(&mut on, title));
                });
                if on != before {
                    out = Some(on.to_string());
                }
            }
            Kind::Choice(list) => {
                // A list (it carries the row's name for screen readers; a segmented choice would
                // not).
                let mut v = value.to_string();
                let opts: Vec<(String, String)> = list
                    .iter()
                    .map(|(c, en, ko)| ((*c).to_string(), lang.pick(en, ko).to_string()))
                    .collect();
                kim_ui::setting_row(ui, title, help, |ui| {
                    kim_ui::dropdown(
                        ui,
                        &format!("jarak-setting-{}", s.key),
                        title,
                        &mut v,
                        &opts,
                    );
                });
                if v != value {
                    out = Some(v);
                }
            }
            Kind::Int { min, max } => {
                let mut n: i64 = value.parse().unwrap_or(min);
                let before = n;
                kim_ui::setting_row(ui, title, help, |ui| {
                    kim_ui::number_field(ui, &mut n, min..=max, "", title, 96.0);
                });
                if n != before {
                    out = Some(n.to_string());
                }
            }
            Kind::Text { .. } | Kind::Columns => {
                let w = self.settings_win.as_mut()?;
                let draft = w.drafts.entry(s.key).or_insert_with(|| value.to_string());
                let mut commit = false;
                kim_ui::setting_stack(ui, title, help, |ui| {
                    let width = ui.available_width().min(size::FORM_MAX_WIDTH);
                    let r = kim_ui::text_field(ui, draft, title, s.default, width);
                    commit = r.lost_focus() && *draft != value;
                });
                if commit {
                    out = Some(draft.clone());
                }
            }
        }
        if let Some(why) = error {
            kim_ui::field_error(ui, &why);
        }
        out
    }

    /// Advanced's own rows: the settings folder, every setting back to its default.
    fn settings_advanced(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let confirm = self.settings_win.as_ref().is_some_and(|w| w.confirm_reset);
        let mut open = false;
        let mut reset = None;
        kim_ui::setting_group(ui, lang.pick("The settings file", "설정 파일"), |ui| {
            let file = self
                .settings
                .as_ref()
                .map_or_else(String::new, |p| p.display().to_string());
            kim_ui::setting_row(
                ui,
                lang.pick("settings.toml", "settings.toml"),
                Some(&file),
                |ui| {
                    if ui
                        .add(kim_ui::Button::ghost(
                            lang.pick("Show in Explorer", "탐색기에서 보기"),
                        ))
                        .clicked()
                    {
                        open = true;
                    }
                },
            );
            kim_ui::row_divider(ui);
            kim_ui::setting_row(
                ui,
                lang.pick("Reset every setting", "모든 설정 초기화"),
                Some(lang.pick(
                    "Every setting goes back to its default; lines Jarak does not know stay.",
                    "모든 설정을 기본값으로 되돌립니다. 자락이 모르는 줄은 남습니다.",
                )),
                |ui| {
                    let b = if confirm {
                        kim_ui::Button::danger(lang.pick("Reset for good", "정말 초기화"))
                    } else {
                        kim_ui::Button::ghost(lang.pick("Reset…", "초기화…"))
                    };
                    if ui.add(b).clicked() {
                        reset = Some(confirm);
                    }
                },
            );
        });
        if open && let Some(p) = self.settings.clone() {
            if !p.exists() {
                let _ = st::write_line(&p, "# Jarak settings", None);
            }
            self.reveal(&p);
        }
        match reset {
            Some(true) => {
                self.reset_all_settings();
                if let Some(w) = self.settings_win.as_mut() {
                    w.confirm_reset = false;
                    w.drafts.clear();
                }
            }
            Some(false) => {
                if let Some(w) = self.settings_win.as_mut() {
                    w.confirm_reset = true;
                }
            }
            None => {}
        }
    }
}
