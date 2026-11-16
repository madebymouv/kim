//! ADR 0100: Ddalgak's Settings window — a native window of its own (an egui viewport; an
//! embedded window where the backend has only one, as in the headless tests), opened by the
//! gear in the main window's lower-right corner. The main window keeps its state underneath;
//! opening Settings again focuses the open window; closing it never ends the editor or the
//! engine.
//!
//! Sections: **General** (the engine's on/off switch and Pause, both showing the engine's real
//! state, the emergency chord, start at sign-in), **Profiles**, **Modes**, **Shortcuts**
//! (Ddalgak's own global shortcuts in `hotkeys.json`, the fixed emergency chord, the rule list's
//! keys), **Appearance** (theme, language) and **Advanced** (rules import/export, release held
//! keys, where the files are, the keyboard help).

use super::*;
use kim_design::{size, space};
use kim_ui::Tone;

/// The sections of the Settings window, in rail order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Profiles,
    Modes,
    Shortcuts,
    Appearance,
    Advanced,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 6] = [
        SettingsSection::General,
        SettingsSection::Profiles,
        SettingsSection::Modes,
        SettingsSection::Shortcuts,
        SettingsSection::Appearance,
        SettingsSection::Advanced,
    ];

    /// The section's name (English in both languages, ADR 0092).
    pub fn title(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Profiles => "Profiles",
            SettingsSection::Modes => "Modes",
            SettingsSection::Shortcuts => "Shortcuts",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Advanced => "Advanced",
        }
    }

    /// The section a UI audit or a test names (`settings-general`, …).
    pub fn parse(s: &str) -> Option<SettingsSection> {
        SettingsSection::ALL
            .into_iter()
            .find(|x| x.title().eq_ignore_ascii_case(s))
    }
}

/// How long the editor waits for the app to answer an engine request before it says so.
pub const ENGINE_TIMEOUT: Duration = Duration::from_secs(5);

/// The Settings window's id (one window; opening it again focuses it).
pub fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("ddalgak-settings")
}

impl EditorApp {
    /// Whether the Settings window is open, and on which section.
    pub fn settings_section(&self) -> Option<SettingsSection> {
        self.settings
    }

    /// Opens Settings on `section` (focuses the window when it is already open).
    pub fn open_settings(&mut self, section: SettingsSection) {
        self.settings = Some(section);
        self.settings_raise = true;
    }

    /// Closes the Settings window (the editor and the engine keep running).
    pub fn close_settings(&mut self) {
        self.settings = None;
        if self.dialog_in_settings {
            self.dialog = None;
            self.dialog_in_settings = false;
        }
    }

    /// Draws the Settings window for this frame while it is open, and applies what was done in
    /// it (dialogs it opens stay in it).
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        let Some(_) = self.settings else {
            return;
        };
        // A child window's size is in the window's own points: egui applies the UI zoom to it
        // (the main window is sized in screen points before the zoom exists, ADR 0099).
        let mut vb = egui::ViewportBuilder::default()
            .with_title("Ddalgak Settings")
            .with_inner_size([size::SETTINGS_WINDOW_WIDTH, size::SETTINGS_WINDOW_HEIGHT])
            .with_min_inner_size([520.0, 400.0]);
        // An audit photographs the window from the screen: it opens beside the main window,
        // never under it.
        if let Some(p) = kim_ui::inspect::audit_position() {
            let right = ctx
                .input(|i| i.viewport().outer_rect)
                .map_or(p[0] + 1200.0, |r| r.right() + 24.0);
            vb = vb.with_position([right, p[1]]);
        }
        let raise = std::mem::take(&mut self.settings_raise);
        ctx.show_viewport_immediate(viewport_id(), vb, |ui, class| {
            let ctx = ui.ctx().clone();
            if raise && class != egui::ViewportClass::EmbeddedWindow {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            let auditing = self.audit_settings && class != egui::ViewportClass::EmbeddedWindow;
            // While a developer asks for geometry (`KIM_UI_INSPECT`, a real-desktop test), this
            // window's own report goes next to the main window's (`….settings.json`).
            let live = (!auditing && class != egui::ViewportClass::EmbeddedWindow)
                .then(kim_design::inspect::requested_path)
                .flatten();
            let main_items = live.as_ref().map(|_| kim_ui::inspect::take());
            if auditing {
                // ADR 0099: the audit of a Settings screen measures this window only.
                kim_ui::inspect::clear();
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.close_settings();
                return;
            }
            if self.dialog.is_none()
                && !ctx.egui_wants_keyboard_input()
                && self.shortcut_picker.is_none()
                && ctx.input(|i| i.key_pressed(egui::Key::Escape))
                && !egui::Popup::is_any_open(&ctx)
            {
                self.close_settings();
                return;
            }
            let c = Colors::get();
            let draw = |ui: &mut egui::Ui, me: &mut EditorApp| {
                egui::Frame::new().fill(c.bg_base).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    me.settings_layout(ui);
                });
            };
            if class == egui::ViewportClass::EmbeddedWindow {
                ui.set_min_size(egui::vec2(size::SETTINGS_WINDOW_WIDTH - 40.0, 380.0));
            }
            draw(ui, self);
            for a in std::mem::take(&mut self.acts) {
                self.dialog_from_settings = true;
                self.apply(a);
                self.dialog_from_settings = false;
            }
            if self.dialog.is_some() && self.dialog_in_settings {
                self.dialog_ui(&ctx);
                for a in std::mem::take(&mut self.acts) {
                    self.dialog_from_settings = true;
                    self.apply(a);
                    self.dialog_from_settings = false;
                }
            }
            if let (Some(path), Some(items)) = (live, main_items) {
                let section = self.settings.unwrap_or_default().title().to_lowercase();
                let report =
                    kim_ui::inspect::report(&ctx, "ddalgak", &format!("settings-{section}"));
                let _ = report.write(&path.with_extension("settings.json"));
                kim_ui::inspect::restore(items);
            }
            if auditing {
                let name = format!(
                    "settings-{}",
                    self.settings.unwrap_or_default().title().to_lowercase()
                );
                kim_ui::inspect::finish_frame_with(
                    &ctx,
                    "ddalgak",
                    &name,
                    Some(&|p: &std::path::Path| {
                        kim_ui::inspect::capture_window_png("Ddalgak Settings", p)
                    }),
                );
            }
        });
    }

    /// The rail (sections) and the page of the open section.
    fn settings_layout(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
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
            kim_ui::title(ui, "Settings");
        });
        rail.add_space(space::M);
        rail.spacing_mut().item_spacing.y = 2.0;
        let current = self.settings.unwrap_or_default();
        kim_ui::inspect::scope("settings.nav", || {
            for s in SettingsSection::ALL {
                if kim_ui::nav_item(&mut rail, current == s, s.title()).clicked() {
                    self.settings = Some(s);
                }
            }
        });
        let body = egui::Rect::from_min_max(egui::pos2(nav.right(), full.top()), full.max);
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body.shrink2(egui::vec2(space::XL, space::L)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        egui::ScrollArea::vertical()
            .id_salt(("settings-page", current.title()))
            .auto_shrink([false, false])
            .show(&mut page, |ui| {
                ui.set_width(ui.available_width() - space::XS);
                kim_ui::title(ui, current.title());
                ui.add_space(space::M);
                if self.dialog_in_settings || self.dialog.is_none() {
                    self.banner_ui(ui);
                }
                match current {
                    SettingsSection::General => self.settings_general(ui),
                    SettingsSection::Profiles => self.profiles_page(ui),
                    SettingsSection::Modes => self.settings_modes(ui),
                    SettingsSection::Shortcuts => self.settings_shortcuts(ui),
                    SettingsSection::Appearance => self.settings_appearance(ui),
                    SettingsSection::Advanced => self.settings_advanced(ui),
                }
                // The result of the last action here (saved, refused, switched), in words.
                if let Some((tone, text)) = self.settings_note.clone()
                    && kim_ui::notice(ui, tone, &text, None, Some("Dismiss"))
                {
                    self.settings_note = None;
                }
            });
    }

    /// The engine's state for the switches: `Some("running"|"paused"|"off")` while the app
    /// answers, else `None`.
    fn engine_state(&self) -> Option<String> {
        match &self.engine {
            EngineView::Running { state, .. } => Some(state.clone()),
            _ => None,
        }
    }

    /// Asks the app to change the engine (the switches never show a state the app has not
    /// confirmed: they stay as they were, disabled, until the answer comes).
    pub(super) fn request_engine(&mut self, r: Request) {
        self.engine_pending = Some((r, Instant::now()));
        self.engine_error = None;
        self.link.send(Job::Engine(r));
    }

    /// No answer within [`ENGINE_TIMEOUT`]: the request is reported, never assumed done.
    pub(super) fn check_engine_timeout(&mut self) {
        if let Some((r, t)) = self.engine_pending
            && t.elapsed() >= ENGINE_TIMEOUT
        {
            self.engine_pending = None;
            self.engine_error = Some((
                "engine_timeout".into(),
                self.lang.t("engine.timeout").into(),
                r,
            ));
            self.ask_status();
        }
    }

    fn settings_general(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let state = self.engine_state();
        let pending = self.engine_pending.map(|(r, _)| r);
        let mut act = None;
        kim_ui::setting_group(ui, "Ddalgak", |ui| {
            // On/off: the engine's real state.
            let mut on = matches!(state.as_deref(), Some(s) if s != "off");
            let off_reason = match &self.engine {
                EngineView::Running { off_reason, .. } => off_reason.clone(),
                _ => None,
            };
            let desc = match (pending, state.as_deref()) {
                (Some(Request::Start), _) => lang.t("engine.turning_on"),
                (Some(Request::Stop), _) => lang.t("engine.turning_off"),
                (_, Some("running")) => lang.t("engine.desc_running"),
                (_, Some("paused")) => lang.t("engine.desc_paused"),
                // ADR 0101: why it is off, when that is not the user's own choice.
                (_, Some(_)) if off_reason.as_deref() == Some("hook_lost") => {
                    lang.t("engine.desc_hook_lost")
                }
                (_, Some(_)) if off_reason.as_deref() == Some("emergency") => {
                    lang.t("engine.desc_emergency")
                }
                (_, Some(_)) => lang.t("engine.desc_off"),
                (_, None) => lang.t(match &self.engine {
                    EngineView::Unknown => "engine.unknown",
                    _ => "engine.desc_not_running",
                }),
            };
            let r = ui.add_enabled(
                state.is_some() && pending.is_none(),
                kim_ui::SwitchRow::new(&mut on, "Turn on Ddalgak").description(desc),
            );
            if r.changed() {
                act = Some(if on { Request::Start } else { Request::Stop });
            }
            kim_ui::row_divider(ui);
            // Pause: only while the engine is on.
            let mut paused = state.as_deref() == Some("paused");
            let pdesc = match (pending, state.as_deref()) {
                (Some(Request::Pause), _) => lang.t("engine.pausing"),
                (Some(Request::Resume), _) => lang.t("engine.resuming"),
                (_, Some("running" | "paused")) => lang.t("engine.pause_desc"),
                _ => lang.t("engine.pause_needs_on"),
            };
            let r = ui.add_enabled(
                matches!(state.as_deref(), Some("running" | "paused")) && pending.is_none(),
                kim_ui::SwitchRow::new(&mut paused, "Pause automation").description(pdesc),
            );
            if r.changed() {
                act = Some(if paused {
                    Request::Pause
                } else {
                    Request::Resume
                });
            }
        });
        if let Some((code, message, r)) = self.engine_error.clone() {
            ui.add_space(-space::S);
            if kim_ui::notice(
                ui,
                Tone::Error,
                &format!("{message} ({code})"),
                None,
                Some("Try again"),
            ) {
                act = Some(r);
            }
            ui.add_space(space::S);
        }
        if let Some(r) = act {
            self.acts.push(Act::Engine(r));
        }
        self.settings_capslock(ui);
        kim_ui::setting_group(ui, "Emergency off", |ui| {
            kim_ui::setting_row(
                ui,
                "Ctrl+Alt+Shift+Esc",
                Some(lang.t("settings.emergency_desc")),
                |ui| {
                    // The keys for screen readers too (the row's text is painted).
                    kim_ui::keycaps(ui, "Ctrl+Alt+Shift+Esc").widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Label, true, "Ctrl+Alt+Shift+Esc")
                    });
                },
            );
        });
        // Start at sign-in (the user's own Run key; nothing else).
        let st = ddalgak::startup::state(self.startup.as_ref(), &self.gui_exe);
        let mut startup_act = None;
        kim_ui::setting_group(ui, "Startup", |ui| match &st {
            Ok(s) => {
                let mut on = s.enabled;
                let desc = if s.foreign {
                    lang.t("settings.startup_foreign")
                } else {
                    lang.t("settings.startup_desc")
                };
                if ui
                    .add(kim_ui::SwitchRow::new(&mut on, "Start at sign-in").description(desc))
                    .changed()
                {
                    startup_act = Some(on);
                }
            }
            Err(e) => {
                ui.label(kim_ui::caption(
                    ui,
                    format!("{}: {e}", lang.t("settings.startup_unreadable")),
                ));
            }
        });
        if let Some(on) = startup_act {
            self.acts.push(Act::SetStartup(on));
        }
    }

    /// ADR 0101: CapsLock (as a modifier, kept off) and the shortcuts that still work while the
    /// automation is off.
    fn settings_capslock(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(doc) = self.doc() else {
            return;
        };
        let always_off = doc.settings.as_ref().and_then(|s| s.capslock)
            == Some(ddalgak::model::CapsLockPolicy::AlwaysOff);
        let active = doc.profile(&doc.active_profile);
        let rules: Vec<&ddalgak::model::Rule> = active
            .map(|p| {
                p.rules
                    .iter()
                    .filter_map(|e| e.rule())
                    .filter(|r| r.enabled)
                    .collect()
            })
            .unwrap_or_default();
        let caps_rules = rules
            .iter()
            .filter(|r| {
                r.trigger
                    .keys_text()
                    .starts_with(ddalgak::trigger::CAPS_PREFIX)
            })
            .count();
        let control: Vec<(String, String)> = rules
            .iter()
            .filter(|r| {
                !r.steps.is_empty()
                    && r.steps
                        .iter()
                        .all(|s| matches!(s, ddalgak::model::Action::Engine(_)))
            })
            .filter_map(|r| {
                let f = RuleForm::edit(&doc.active_profile, r);
                let keys = r.trigger.keys_text().to_string();
                (!keys.is_empty()).then(|| {
                    let what = f.sentence(active, lang);
                    (
                        keys,
                        if what.is_empty() {
                            r.title.clone()
                        } else {
                            what
                        },
                    )
                })
            })
            .collect();
        let capslock_on = match &self.engine {
            EngineView::Running { capslock_on, .. } => *capslock_on,
            _ => None,
        };
        let mut set = None;
        kim_ui::setting_group(ui, "CapsLock", |ui| {
            let mut on = always_off;
            if ui
                .add(
                    kim_ui::SwitchRow::new(&mut on, lang.t("settings.caps_off"))
                        .description(lang.t("settings.caps_off_desc")),
                )
                .changed()
            {
                set = Some(on);
            }
            kim_ui::row_divider(ui);
            let layer = if caps_rules == 0 {
                lang.t("settings.caps_layer_none").to_string()
            } else {
                format!(
                    "{} {caps_rules}. {}",
                    lang.t("settings.caps_layer_rules"),
                    lang.t(if always_off {
                        "settings.caps_layer_tap_off"
                    } else {
                        "settings.caps_layer_tap"
                    })
                )
            };
            kim_ui::setting_row(ui, lang.t("settings.caps_layer"), Some(&layer), |ui| {
                let now = match capslock_on {
                    Some(true) => lang.t("settings.caps_now_on"),
                    Some(false) => lang.t("settings.caps_now_off"),
                    None => "—",
                };
                ui.label(kim_ui::caption(ui, now.to_string()));
            });
        });
        if let Some(on) = set {
            self.acts.push(Act::SetCapsPolicy(on));
        }
        kim_ui::setting_group(ui, lang.t("settings.control_title"), |ui| {
            if control.is_empty() {
                ui.label(kim_ui::caption(
                    ui,
                    lang.t("settings.control_none").to_string(),
                ));
            }
            for (i, (keys, what)) in control.iter().enumerate() {
                if i > 0 {
                    kim_ui::row_divider(ui);
                }
                let named = kim_ui::keys::readable_chord(keys, self.kim_lang());
                kim_ui::setting_row(ui, &named, Some(what), |ui| {
                    // The keys for screen readers too (the row's text is painted).
                    kim_ui::keycaps(ui, keys)
                        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &named));
                });
            }
        });
    }

    /// ADR 0101: saves `settings.capslock` (the running app applies it with the next reload).
    pub(super) fn set_caps_policy(&mut self, always_off: bool) {
        let label = if always_off {
            "CapsLock stays off"
        } else {
            "CapsLock works as in Windows"
        };
        self.commit(label.into(), "settings.capslock", move |d| {
            let s = d.settings.get_or_insert_with(Default::default);
            s.capslock = always_off.then_some(ddalgak::model::CapsLockPolicy::AlwaysOff);
            Ok(Change {
                changed: true,
                ..Default::default()
            })
        });
    }

    fn settings_appearance(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let mut changed = false;
        let mut lang_changed = false;
        kim_ui::setting_group(ui, "Appearance", |ui| {
            kim_ui::setting_row(ui, "Theme", Some(lang.t("settings.theme")), |ui| {
                changed |= kim_ui::segmented(
                    ui,
                    &mut self.prefs.theme,
                    &[
                        (kim_ui::ThemePref::System, "System"),
                        (kim_ui::ThemePref::Dark, "Dark"),
                        (kim_ui::ThemePref::Light, "Light"),
                    ],
                );
            });
            kim_ui::row_divider(ui);
            let system = kim_ui::LangPref::System.label();
            kim_ui::setting_row(ui, "Language", Some(lang.t("settings.language")), |ui| {
                lang_changed |= kim_ui::segmented(
                    ui,
                    &mut self.prefs.lang,
                    &[
                        (kim_ui::LangPref::System, system.as_str()),
                        (kim_ui::LangPref::En, "English"),
                        (kim_ui::LangPref::Ko, "한국어"),
                    ],
                );
            });
        });
        ui.label(kim_ui::caption(ui, lang.t("settings.size_note")));
        // Only the language choice changes the language (choosing a theme kept a `--lang` window
        // in English until the theme was changed, then it switched to Windows' language).
        if lang_changed {
            self.lang = match self.prefs.lang.resolve() {
                kim_ui::Lang::Ko => Lang::Ko,
                kim_ui::Lang::En => Lang::En,
            };
        }
        if changed || lang_changed {
            self.save_prefs();
        }
    }

    fn settings_advanced(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        kim_ui::setting_group(ui, "Rules data", |ui| {
            kim_ui::setting_row(
                ui,
                "Import and export",
                Some(lang.t("settings.transfer")),
                |ui| {
                    if ui
                        .add(kim_ui::Button::secondary(lang.t("btn.export")))
                        .clicked()
                    {
                        self.acts.push(Act::Export(Vec::new()));
                    }
                    if ui
                        .add(kim_ui::Button::secondary(lang.t("btn.import")))
                        .clicked()
                    {
                        self.acts.push(Act::Import);
                    }
                },
            );
            kim_ui::row_divider(ui);
            let folder = self.repo.paths.config.display().to_string();
            kim_ui::setting_row(ui, "Rules folder", Some(&folder), |_| {});
        });
        kim_ui::setting_group(ui, "Recovery", |ui| {
            kim_ui::setting_row(
                ui,
                "Release held keys",
                Some(lang.t("settings.release_desc")),
                |ui| {
                    if ui
                        .add_enabled(
                            self.engine_state().is_some(),
                            kim_ui::Button::secondary("Release"),
                        )
                        .clicked()
                    {
                        self.acts.push(Act::Release);
                    }
                },
            );
        });
        kim_ui::setting_group(ui, "Help", |ui| {
            kim_ui::setting_row(ui, "Keyboard shortcuts", Some(lang.t("help.f1")), |ui| {
                if ui.add(kim_ui::Button::secondary("Open")).clicked() {
                    self.acts.push(Act::Help);
                }
            });
        });
    }

    /// Modes of the active profile: each with its rules and its on/off switch (the default mode
    /// is always on), the mode the engine is in now with Switch / Back to default, and a new mode.
    fn settings_modes(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        let pid = self.active_profile_id();
        let Some(p) = self.profile(&pid).cloned() else {
            return;
        };
        ui.label(kim_ui::caption(
            ui,
            format!("{} · {}", lang.t("settings.modes_of"), p.title),
        ));
        ui.add_space(space::S);
        let now = match &self.engine {
            EngineView::Running {
                mode: Some(m),
                state,
                ..
            } if state != "off" => Some(m.clone()),
            _ => None,
        };
        kim_ui::setting_group(ui, "Modes of this profile", |ui| {
            for (i, m) in p.modes.iter().enumerate() {
                if i > 0 {
                    kim_ui::row_divider(ui);
                }
                let is_default = m.id == p.default_mode;
                let rules = ddalgak::mode_edit::rules_of(&p, &m.id).len();
                let mut notes = vec![format!("{rules} {}", lang.t("status.rules"))];
                if is_default {
                    notes.push(lang.t("modes.default").to_string());
                }
                if let Some(parent) = &m.parent {
                    let pt = p
                        .modes
                        .iter()
                        .find(|x| &x.id == parent)
                        .map_or(parent.as_str(), |x| x.title.as_str());
                    notes.push(format!("{} {pt}", lang.t("modes.layer_over")));
                }
                let active_now = now.as_deref() == Some(m.title.as_str())
                    || now.as_deref() == Some(m.id.as_str());
                if active_now {
                    notes.push(lang.t("modes.active_now").to_string());
                }
                let desc = notes.join(" · ");
                kim_ui::setting_row(ui, &m.title, Some(&desc), |ui| {
                    if !is_default {
                        let mut on = m.enabled != Some(false);
                        if ui
                            .add(kim_ui::Switch::new(
                                &mut on,
                                format!("{}: {}", lang.t("filter.enabled"), m.title),
                            ))
                            .changed()
                        {
                            self.acts.push(Act::ModeEnabled(m.id.clone(), on));
                        }
                    }
                    if now.is_some()
                        && !active_now
                        && m.enabled != Some(false)
                        && ui
                            .add(kim_ui::Button::ghost("Switch"))
                            .on_hover_text(lang.t("modes.switch_hint"))
                            .clicked()
                    {
                        self.acts.push(Act::ModeSet(m.id.clone()));
                    }
                });
            }
        });
        let mut add = false;
        kim_ui::setting_group(ui, "New mode", |ui| {
            ui.horizontal(|ui| {
                let w = kim_ui::remaining_width(ui, size::BUTTON_MIN_WIDTH + space::S + 40.0);
                let r = kim_ui::text_field(
                    ui,
                    &mut self.new_mode_title,
                    lang.t("field.new_mode"),
                    lang.t("field.new_mode"),
                    w,
                );
                add |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                add |= ui
                    .add_enabled(
                        !self.new_mode_title.trim().is_empty(),
                        kim_ui::Button::secondary(lang.t("modes.add")),
                    )
                    .clicked();
            });
        });
        if add && !self.new_mode_title.trim().is_empty() {
            // The profile this page shows, never the profile of a rule open in the main window.
            self.acts
                .push(Act::NewModeIn(pid, self.new_mode_title.clone()));
        }
        ui.label(
            egui::RichText::new(lang.t("modes.note"))
                .size(Metrics::get().caption)
                .color(c.text_muted),
        );
    }

    /// Ddalgak's own global shortcuts (`hotkeys.json`): each with its keys, Pick (the key
    /// picker), Off and Reset; conflicts and refusals are shown as the hotkey contract reports
    /// them. The tray app reads the file when it starts, which the page says.
    fn settings_shortcuts(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let global = ddalgak::shortcuts::Shortcuts::load(&self.repo.paths, false);
        let m = ddalgak::shortcuts::manifest();
        let none = lang.t("shortcuts.off").to_string();
        let mut act: Option<(String, &'static str)> = None;
        ui.label(kim_ui::caption(ui, lang.t("shortcuts.intro")));
        ui.add_space(space::S);
        kim_ui::setting_group(ui, "Global shortcuts", |ui| {
            for (i, action) in ddalgak::shortcuts::REGISTERED.iter().enumerate() {
                if i > 0 {
                    kim_ui::row_divider(ui);
                }
                let title = m
                    .actions
                    .iter()
                    .find(|a| a.id == *action)
                    .map_or(action.to_string(), |a| a.title.clone());
                let keys = global
                    .list
                    .iter()
                    .find(|s| s.action == *action)
                    .map(|s| s.keys.clone())
                    .unwrap_or_default();
                kim_ui::setting_row(ui, &title, None, |ui| {
                    if !keys.is_empty() && ui.add(kim_ui::Button::ghost("Off")).clicked() {
                        act = Some((action.to_string(), "off"));
                    }
                    if ui.add(kim_ui::Button::ghost("Pick")).clicked() {
                        act = Some((action.to_string(), "pick"));
                    }
                    let f = kim_ui::shortcut_field(ui, &title, &keys, false, &none, "", 150.0);
                    if f.clicked() {
                        act = Some((action.to_string(), "pick"));
                    }
                });
            }
        });
        if let Some(p) = &global.problem {
            kim_ui::notice(ui, Tone::Warning, &format!("hotkeys.json: {p}"), None, None);
        }
        kim_ui::setting_group(ui, "Always available", |ui| {
            kim_ui::setting_row(
                ui,
                lang.t("shortcuts.emergency"),
                Some(lang.t("settings.emergency_desc")),
                |ui| {
                    // The keys for screen readers too (the row's text is painted).
                    kim_ui::keycaps(ui, "Ctrl+Alt+Shift+Esc").widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Label, true, "Ctrl+Alt+Shift+Esc")
                    });
                },
            );
        });
        kim_ui::setting_group(ui, "Rule list", |ui| {
            if self.shortcuts.bindings.is_empty() {
                ui.label(kim_ui::caption(ui, lang.t("shortcuts.list_off")));
            }
            for (i, b) in self.shortcuts.bindings.clone().iter().enumerate() {
                if i > 0 {
                    kim_ui::row_divider(ui);
                }
                let title = m
                    .actions
                    .iter()
                    .find(|a| a.id == b.action)
                    .map_or(b.action.clone(), |a| a.title.clone());
                kim_ui::setting_row(ui, &title, None, |ui| {
                    kim_ui::keycaps(ui, &b.keys);
                });
            }
        });
        match act {
            Some((a, "off")) => self
                .acts
                .push(Act::Shortcut(kim_hotkeys::edit::Edit::Disable {
                    action: a,
                })),
            Some((a, _)) => self.shortcut_picker = Some((a, kim_ui::keys::KeyPicker::default())),
            None => {}
        }
        self.shortcut_picker_dialog(ui.ctx());
    }

    fn shortcut_picker_dialog(&mut self, ctx: &egui::Context) {
        let Some((action, mut picker)) = self.shortcut_picker.take() else {
            return;
        };
        let lang = self.kim_lang();
        let keys: Vec<&'static str> = kim_hotkeys::keys::NAMED_KEYS.to_vec();
        let mut picked = kim_ui::keys::Picked::None;
        let check = |chord: &str| {
            kim_hotkeys::keys::Chord::parse(chord)
                .err()
                .map(|e| e.to_string())
                .or_else(|| {
                    (!chord.contains('+')).then(|| {
                        lang.pick(
                            "A global shortcut needs Ctrl, Alt, Shift or Win with a key.",
                            "전역 단축키에는 Ctrl, Alt, Shift, Win 중 하나와 키가 필요합니다.",
                        )
                        .to_string()
                    })
                })
        };
        let r = kim_ui::dialog_sized(ctx, "ddalgak-shortcut-picker", "Pick keys", 520.0, |ui| {
            picked = picker.ui_checked(ui, &keys, false, lang, &check);
        });
        match picked {
            kim_ui::keys::Picked::Confirm(chord) => {
                self.acts.push(Act::Shortcut(kim_hotkeys::edit::Edit::Set {
                    action,
                    bindings: vec![kim_hotkeys::manifest::BindingSpec::Text(chord)],
                }));
            }
            kim_ui::keys::Picked::Cancel => {}
            kim_ui::keys::Picked::None if r.should_close() => {}
            kim_ui::keys::Picked::None => self.shortcut_picker = Some((action, picker)),
        }
    }

    /// Applies one change to Ddalgak's global shortcuts in `hotkeys.json` (the hotkey contract
    /// checks it; a refusal or conflict is shown, nothing is forced).
    pub(super) fn edit_global_shortcut(&mut self, edit: &kim_hotkeys::edit::Edit) {
        let lang = self.lang;
        let m = ddalgak::shortcuts::manifest();
        let path = self.repo.paths.hotkeys();
        let loaded = kim_hotkeys::store::load(&path, &m);
        if loaded.state.safe_mode() {
            self.settings_note = Some((
                Tone::Error,
                format!(
                    "{} ({})",
                    lang.t("shortcuts.safe_mode"),
                    loaded.problems.join("; ")
                ),
            ));
            return;
        }
        let state = kim_hotkeys::check::ToolState::new(m, loaded.layer);
        match kim_hotkeys::edit::apply(
            &[state],
            &[],
            0,
            edit,
            &kim_hotkeys::edit::Options::default(),
        ) {
            Ok(a) => {
                if let Some(layer) = a.layer
                    && let Err(e) = kim_hotkeys::store::save(&path, &layer, true)
                {
                    self.settings_note = Some((Tone::Error, format!("hotkeys.json: {e}")));
                    return;
                }
                let mut text = lang.t("shortcuts.saved").to_string();
                for cf in &a.conflicts {
                    text.push_str(&format!(" {}", cf.message));
                }
                self.settings_note = Some((Tone::Success, text));
            }
            Err(r) => {
                let mut text = r.message.clone();
                if let Some(h) = &r.hint {
                    text.push_str(&format!(" {h}"));
                }
                self.settings_note = Some((Tone::Error, format!("{text} ({})", r.error.code())));
            }
        }
    }

    /// Turns start at sign-in on or off (only the Run value; the result is shown).
    pub(super) fn set_startup(&mut self, on: bool) {
        let lang = self.lang;
        let r = if on {
            ddalgak::startup::enable(self.startup.as_mut(), &self.gui_exe, false)
        } else {
            ddalgak::startup::disable(self.startup.as_mut(), false)
        };
        self.settings_note = Some(match r {
            Ok(_) => (
                Tone::Success,
                lang.t(if on {
                    "settings.startup_on"
                } else {
                    "settings.startup_off"
                })
                .to_string(),
            ),
            Err(e) => (Tone::Error, e),
        });
    }

    /// The design system's language for this window.
    pub(crate) fn kim_lang(&self) -> kim_ui::Lang {
        match self.lang {
            Lang::Ko => kim_ui::Lang::Ko,
            Lang::En => kim_ui::Lang::En,
        }
    }
}
