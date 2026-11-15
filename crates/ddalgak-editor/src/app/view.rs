//! Drawing the editor's main window (ADR 0100): the rail (the name, a small engine status, the
//! two primary activities: Rules and Playground), the page, the status line with the settings
//! gear in its lower-right corner, and the dialogs. Settings live in their own window
//! (`app/settings.rs`); the Rules page is `app/rules.rs`, the rule editor `app/editor.rs`.
//! Every frame only reads the state and records [`Act`]s, applied afterwards.

use super::*;
use crate::theme;
use kim_design::{size, space};

fn small(text: impl Into<String>, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(Metrics::get().caption)
        .color(color)
}

/// A primary (accent) button.
fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(kim_ui::Button::primary(text))
}

impl EditorApp {
    /// Draws one frame (also used by the headless tests).
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.themed {
            theme::apply(&ctx);
            self.themed = true;
            let c2 = ctx.clone();
            if let Ok(mut w) = self.wake.lock() {
                *w = Some(Box::new(move || c2.request_repaint()));
            }
        }
        self.poll();
        self.refresh_status(&ctx);
        self.check_engine_timeout();
        // Step 78: while recording, keys go to the recorder only (before any shortcut).
        self.record_events(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_now && !self.request_close()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.close_now {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.compute_visible();
        self.keys(&ctx);
        self.apply_theme(&ctx);
        let c = Colors::get();
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::M as i8, space::XS as i8)),
            )
            .show(ui, |ui| self.status_line(ui));
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(size::NAV_WIDTH)
            .frame(
                egui::Frame::new()
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::S as i8, space::L as i8)),
            )
            .show(ui, |ui| {
                kim_ui::inspect::scope("ddalgak.nav", || self.nav(ui));
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(c.bg_base)
                    .inner_margin(egui::Margin::symmetric(space::XL as i8, space::L as i8)),
            )
            .show(ui, |ui| match self.page {
                Page::Playground => {
                    egui::ScrollArea::vertical()
                        .id_salt("playground-scroll")
                        .show(ui, |ui| self.playground_page(ui));
                }
                // ADR 0096/0100: a rule opens in its own page, the list stays clean.
                Page::Rules if self.form.is_some() => self.editor_page(ui),
                Page::Rules => self.rules_page(ui),
            });
        if self.dialog.is_some() && !self.dialog_in_settings {
            self.dialog_ui(&ctx);
        }
        for a in std::mem::take(&mut self.acts) {
            self.apply(a);
        }
        // ADR 0100: Settings, a window of its own (after the main window's acts are applied, so
        // what it does is applied as its own).
        self.settings_window(&ctx);
    }

    fn compute_visible(&mut self) {
        let shown: Vec<usize> = (0..self.rows.len())
            .filter(|i| self.filter.matches(&self.rows[*i]))
            .collect();
        let groups = listing::group(
            &self.rows,
            &shown,
            self.grouping,
            self.lang.t("group.every_mode"),
            self.lang.t("group.every_app"),
        );
        let mut seen = BTreeSet::new();
        self.visible = groups
            .iter()
            .flat_map(|g| g.rows.iter().copied())
            .filter(|i| seen.insert(*i))
            .collect();
    }

    // --- Rail and status ---------------------------------------------------------------------

    /// The engine's state in words for the rail's indicator (never a control: Settings has the
    /// switches) and its tone.
    pub(super) fn engine_status(&self) -> (Option<kim_ui::Tone>, &'static str) {
        let lang = self.lang;
        if self.engine_pending.is_some() {
            return (Some(kim_ui::Tone::Info), lang.t("engine.status_changing"));
        }
        match &self.engine {
            EngineView::Running { state, .. } => match state.as_str() {
                "running" => (Some(kim_ui::Tone::Success), lang.t("engine.status_on")),
                "paused" => (Some(kim_ui::Tone::Warning), lang.t("engine.status_paused")),
                _ => (None, lang.t("engine.status_off")),
            },
            EngineView::Unknown => (None, lang.t("engine.unknown")),
            EngineView::NotRunning | EngineView::Error { .. } => (
                Some(kim_ui::Tone::Error),
                lang.t("engine.status_not_running"),
            ),
        }
    }

    /// ADR 0100: the rail: the name, the engine's state (a status, not a switch), the profile
    /// and mode only when there is a choice, and the primary activities.
    fn nav(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        ui.horizontal(|ui| {
            ui.add_space(space::XS);
            kim_ui::title(ui, "Ddalgak");
        });
        ui.add_space(space::XS);
        ui.horizontal(|ui| {
            ui.add_space(space::XS);
            let (tone, text) = self.engine_status();
            kim_ui::status_indicator(ui, tone, text).on_hover_text(lang.t("engine.status_hint"));
        });
        let multiple = self.doc().is_some_and(|d| d.profiles.len() > 1);
        if multiple {
            let pid = self.active_profile_id();
            let ptitle = self
                .profile(&pid)
                .map_or_else(|| pid.clone(), |p| p.title.clone());
            let ptitle = if self.switching_to() == Some(pid.as_str()) {
                format!("{ptitle} ({})", lang.t("profiles.switching"))
            } else {
                ptitle
            };
            ui.horizontal(|ui| {
                ui.add_space(space::XS);
                ui.label(small(
                    format!("{}: {ptitle}", lang.t("header.profile")),
                    c.text_muted,
                ));
            });
        }
        let several_modes = self
            .profile(&self.active_profile_id())
            .is_some_and(|p| p.modes.len() > 1);
        if let EngineView::Running {
            mode: Some(m),
            state,
            ..
        } = &self.engine
            && state != "off"
            && several_modes
        {
            ui.horizontal(|ui| {
                ui.add_space(space::XS);
                ui.label(small(
                    format!("{}: {m}", lang.t("header.mode")),
                    c.text_muted,
                ));
            });
        }
        ui.add_space(space::L);
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, (p, key)) in [
            (Page::Rules, "nav.rules"),
            (Page::Playground, "nav.playground"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = kim_ui::nav_item(ui, self.page == p, lang.t(key));
            if i == 0 {
                self.anchors[0] = Some(r.id);
            }
            if r.clicked() {
                self.acts.push(Act::Page(p));
            }
        }
    }

    /// ADR 0096: the theme from Settings, applied when it changes.
    fn apply_theme(&mut self, ctx: &egui::Context) {
        let want = self.prefs.theme.resolve();
        if self.applied_theme != Some(want) {
            theme::set_theme(ctx, want);
            self.applied_theme = Some(want);
        }
    }

    fn status_line(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        ui.horizontal_wrapped(|ui| {
            let n = self.rows.len();
            let problems = self.rows.iter().filter(|r| r.has_problems()).count();
            let review = self.rows.iter().filter(|r| r.needs_review).count();
            let mut parts = vec![format!("{n} {}", self.t("status.rules"))];
            if self.page == Page::Rules && self.form.is_none() {
                // The Rules page's title shows the count.
                parts.clear();
            }
            if problems > 0 {
                parts.push(format!("{problems} {}", self.t("status.problems")));
            }
            if review > 0 {
                parts.push(format!("{review} {}", self.t("status.review")));
            }
            if self.has_unsaved_changes() {
                parts.push(self.t("status.draft").to_string());
            }
            let rev = self.loaded.revision();
            let (key, r) = self.save.label().unwrap_or((
                if self.loaded.state.usable() {
                    "status.waiting"
                } else {
                    "status.kept_previous"
                },
                rev,
            ));
            let color = match &self.save {
                SaveState::Active { .. } => c.success,
                SaveState::KeptPrevious { .. } | SaveState::Unconfirmed { .. } => c.warning,
                _ => c.text_secondary,
            };
            let mut line = self.t(key).to_string();
            let rev_note = format!("{} {r}", self.t("status.revision"));
            match &self.save {
                SaveState::KeptPrevious { reason, .. } => line.push_str(&format!(": {reason}")),
                SaveState::Unconfirmed { why, .. } => line.push_str(&format!(": {why}")),
                _ => {}
            }
            if self.loaded.state == store::FileState::Missing {
                line = self.t("status.nothing_saved").to_string();
            }
            if !self.loaded.state.usable() {
                line = format!(
                    "rules.json is not usable ({}): {}",
                    self.loaded.state.as_str(),
                    self.loaded.state.problem().unwrap_or_default()
                );
            }
            if !parts.is_empty() {
                ui.label(small(parts.join(" · "), c.text_secondary));
                ui.label(small("·", c.text_muted));
            }
            ui.label(small(line, color)).on_hover_text(rev_note);
            if let Some(t) = self.toast.clone() {
                ui.separator();
                ui.label(small(t.text, c.text_primary));
                if t.undo
                    && ui
                        .add(kim_ui::Button::ghost(self.t("btn.undo")).compact())
                        .clicked()
                {
                    self.acts.push(Act::Undo);
                    self.acts.push(Act::DismissToast);
                }
                if ui
                    .add(kim_ui::Button::ghost(self.t("btn.dismiss")).compact())
                    .clicked()
                {
                    self.acts.push(Act::DismissToast);
                }
            }
            // ADR 0092/0096: the settings gear in the window's lower-right corner.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(size::GEAR), egui::Sense::hover());
                let open = self.settings.is_some();
                if kim_ui::gear_at(ui, rect, "Settings", open).clicked() {
                    // ADR 0100: a separate window; again: brings it forward.
                    self.acts
                        .push(Act::OpenSettings(self.settings.unwrap_or_default()));
                }
            });
        });
    }

    pub(super) fn banner_ui(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let Some(b) = self.banner.clone() else {
            return;
        };
        let (color, text, details) = match b {
            Banner::Error {
                code,
                message,
                details,
            } => (c.danger, format!("✕ {message} ({code})"), details),
            Banner::Info(m) => (c.info, format!("ⓘ {m}"), Vec::new()),
        };
        egui::Frame::new()
            .fill(c.bg_overlay)
            .stroke(egui::Stroke::new(1.0, color))
            .corner_radius(4)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(text).color(color));
                    if ui
                        .add(kim_ui::Button::ghost(self.t("btn.dismiss")).compact())
                        .clicked()
                    {
                        self.banner = None;
                    }
                });
                for d in details {
                    ui.label(small(d, c.text_secondary));
                }
            });
    }

    pub(super) fn simulation_ui(ui: &mut egui::Ui, lang: Lang, sim: &Simulation, layout: &Layout) {
        let c = Colors::get();
        egui::Frame::new()
            .fill(c.bg_overlay)
            .corner_radius(4)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.label(small(lang.t("form.try_intro"), c.text_muted));
                for (i, l) in sim.lines.iter().enumerate() {
                    let color = if i == 0 {
                        if sim.fires { c.success } else { c.danger }
                    } else {
                        c.text_secondary
                    };
                    let l = match l.strip_prefix("✕ ") {
                        Some(r) => format!("✕ {}", lang.problem(r)),
                        None => l.clone(),
                    };
                    ui.label(egui::RichText::new(l).color(color));
                }
                if let Some(t) = &sim.typed {
                    let mut shown = t.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .interactive(false)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    )
                    .accessible_name(lang.t("form.as_typed"));
                }
                if let Some((from, to)) = sim.pointer {
                    screen_map(ui, layout, from, Some(to));
                }
            });
    }

    /// The document checks for the form's rule as if it were saved now (duplicate triggers,
    /// collisions with Ddalgak's own shortcuts, warnings), for the Try step.
    pub(super) fn candidate_problems(
        &self,
        profile: Option<&Profile>,
    ) -> Vec<ddalgak::validate::Problem> {
        let (Some(f), Some(doc)) = (&self.form, &self.loaded.doc) else {
            return Vec::new();
        };
        let id =
            f.id.clone()
                .unwrap_or_else(|| ddalgak::model::new_rule_id(|c| doc.has_rule_id(c)));
        let Ok(mut rule) = f.build(&id, profile) else {
            return Vec::new();
        };
        rule.id = id.clone();
        let mut d = doc.clone();
        match d.find_rule(&id) {
            Some((pi, ri)) => {
                d.profiles[pi].rules[ri] = ddalgak::model::RuleEntry::Typed(Box::new(rule));
            }
            None => {
                if let Some(pi) = d.profile_index(&f.profile) {
                    d.profiles[pi]
                        .rules
                        .push(ddalgak::model::RuleEntry::Typed(Box::new(rule)));
                }
            }
        }
        // Consent is asked when saving; a missing grant is not a problem of the form.
        let ctx = self.repo.context();
        let r = repo::isolated(|| ddalgak::validate::check(&d.to_value(), &ctx));
        r.all()
            .filter(|p| p.rule.as_deref() == Some(id.as_str()))
            .filter(|p| p.severity != ddalgak::validate::Severity::Info)
            .filter(|p| !p.code.starts_with("consent"))
            .filter(|p| p.code != "schema_violation" || !p.message.contains("consent"))
            .cloned()
            .collect()
    }

    // --- Dialogs ---------------------------------------------------------------------------------

    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context) {
        let c = Colors::get();
        let lang = self.lang;
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        let focus = std::mem::take(&mut self.dialog_focus);
        let modal = egui::Modal::new(egui::Id::unique("editor-dialog")).show(ctx, |ui| {
            ui.set_max_width(520.0);
            match dialog {
                Dialog::Delete { ids, titles } => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.delete"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    ui.label(format!(
                        "{} {}: {}?",
                        lang.t("btn.delete"),
                        ids.len(),
                        titles.join(", ")
                    ));
                    ui.horizontal(|ui| {
                        let cancel = ui.add(kim_ui::Button::secondary(lang.t("btn.cancel")));
                        if focus {
                            cancel.request_focus();
                        }
                        if cancel.clicked() {
                            self.acts.push(Act::CloseDialog);
                        }
                        if ui
                            .add(kim_ui::Button::danger(format!(
                                "{} ({})",
                                lang.t("btn.delete"),
                                ids.len()
                            )))
                            .clicked()
                        {
                            self.acts.push(Act::Delete(ids.clone()));
                        }
                    });
                }
                Dialog::Discard(then) => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.discard"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    ui.label(lang.t("dialog.discard_text"));
                    ui.horizontal(|ui| {
                        let keep = ui.add(kim_ui::Button::secondary(lang.t("btn.keep_editing")));
                        if focus {
                            keep.request_focus();
                        }
                        if keep.clicked() {
                            self.acts.push(Act::CloseDialog);
                        }
                        if ui
                            .add(kim_ui::Button::secondary(lang.t("btn.discard")))
                            .clicked()
                        {
                            self.acts.push(Act::Discard(then.clone()));
                        }
                    });
                }
                Dialog::Consent { perms, targets, .. } => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.consent"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    for p in &perms {
                        ui.label(lang.t(&form::permission_key(*p)));
                    }
                    for t in &targets {
                        ui.label(egui::RichText::new(t).monospace().color(c.text_primary));
                    }
                    ui.label(small(lang.t("dialog.consent_text"), c.text_muted));
                    ui.horizontal(|ui| {
                        let cancel = ui.add(kim_ui::Button::secondary(lang.t("btn.cancel")));
                        if focus {
                            cancel.request_focus();
                        }
                        if cancel.clicked() {
                            self.acts.push(Act::CloseDialog);
                        }
                        if ui
                            .add(kim_ui::Button::secondary(lang.t("btn.allow")))
                            .clicked()
                        {
                            self.acts.push(Act::Allow);
                        }
                    });
                }
                Dialog::Export {
                    ids,
                    mut path,
                    done,
                } => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.export"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    ui.label(small(lang.t("dialog.export_text"), c.text_muted));
                    ui.label(format!("{} {}", ids.len(), lang.t("status.rules")));
                    let r = ui
                        .add(egui::TextEdit::singleline(&mut path).desired_width(480.0))
                        .accessible_name(lang.t("dialog.file"));
                    if focus {
                        r.request_focus();
                    }
                    let mut new_done = done.clone();
                    if let Some(d) = &done {
                        ui.label(egui::RichText::new(d).color(c.success));
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add(kim_ui::Button::secondary(lang.t("btn.close")))
                            .clicked()
                        {
                            self.acts.push(Act::CloseDialog);
                        }
                        if done.is_none() && primary(ui, lang.t("btn.export")).clicked() {
                            new_done = Some(self.export(&ids, &path));
                        }
                    });
                    if let Some(Dialog::Export {
                        path: p, done: d, ..
                    }) = &mut self.dialog
                    {
                        *p = path;
                        *d = new_done;
                    }
                }
                Dialog::Import {
                    mut path,
                    plan,
                    error,
                } => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.import"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    ui.label(small(lang.t("dialog.import_text"), c.text_muted));
                    let r = ui
                        .add(egui::TextEdit::singleline(&mut path).desired_width(480.0))
                        .accessible_name(lang.t("dialog.file"));
                    if focus {
                        r.request_focus();
                    }
                    let mut next_plan = plan.clone();
                    let mut next_error = error.clone();
                    if ui
                        .add(kim_ui::Button::secondary(lang.t("dialog.check")))
                        .clicked()
                    {
                        match self.plan_import(&path) {
                            Ok(p) => {
                                next_plan = Some(Box::new(p));
                                next_error = None;
                            }
                            Err(e) => {
                                next_plan = None;
                                next_error = Some(e);
                            }
                        }
                    }
                    if let Some(e) = &error {
                        ui.label(egui::RichText::new(format!("✕ {e}")).color(c.danger));
                    }
                    if let Some(p) = &plan {
                        ui.label(format!(
                            "{} {} → {}",
                            p.rules.len(),
                            lang.t("dialog.import_adds"),
                            p.target_profile
                        ));
                        for r in &p.rules {
                            ui.label(format!("• {} — {}", r.title, r.summary));
                            for t in &r.launch_targets {
                                ui.label(small(
                                    format!("   {}: {t}", lang.t("dialog.import_opens")),
                                    c.warning,
                                ));
                            }
                            if r.injects {
                                ui.label(small(
                                    format!("   {}", lang.t("dialog.import_injects")),
                                    c.warning,
                                ));
                            }
                            if let Some(from) = &r.renamed_from {
                                ui.label(small(
                                    format!(
                                        "   {} {from} → {}",
                                        lang.t("dialog.import_renamed"),
                                        r.id
                                    ),
                                    c.text_muted,
                                ));
                            }
                        }
                        if !p.new_modes.is_empty() {
                            ui.label(small(
                                format!(
                                    "{}: {}",
                                    lang.t("dialog.import_modes"),
                                    p.new_modes.join(", ")
                                ),
                                c.text_muted,
                            ));
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add(kim_ui::Button::secondary(lang.t("btn.cancel")))
                            .clicked()
                        {
                            self.acts.push(Act::CloseDialog);
                        }
                        if plan.as_ref().is_some_and(|p| !p.rules.is_empty())
                            && ui
                                .add(kim_ui::Button::secondary(lang.t("btn.add_disabled")))
                                .clicked()
                            && let Some(p) = plan.clone()
                        {
                            self.import(&p);
                        }
                    });
                    if let Some(Dialog::Import {
                        path: pp,
                        plan: pl,
                        error: er,
                    }) = &mut self.dialog
                    {
                        *pp = path;
                        *pl = next_plan;
                        *er = next_error;
                    }
                }
                d @ (Dialog::SwitchProfile { .. }
                | Dialog::EditProfile { .. }
                | Dialog::DeleteProfile { .. }
                | Dialog::ExportProfile { .. }
                | Dialog::ImportProfile { .. }
                | Dialog::History { .. }) => self.profile_dialog(ui, d, focus),
                Dialog::Help => {
                    ui.label(
                        egui::RichText::new(lang.t("dialog.help"))
                            .size(Metrics::get().title)
                            .strong(),
                    );
                    egui::Grid::new("help-grid")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            let m = ddalgak::shortcuts::manifest();
                            for b in &self.shortcuts.bindings {
                                let title = m
                                    .actions
                                    .iter()
                                    .find(|a| a.id == b.action)
                                    .map_or(b.action.as_str(), |a| a.title.as_str());
                                ui.label(egui::RichText::new(&b.keys).monospace());
                                ui.label(title);
                                ui.end_row();
                            }
                            for (k, what) in shortcuts::FIXED {
                                ui.label(egui::RichText::new(*k).monospace());
                                ui.label(*what);
                                ui.end_row();
                            }
                        });
                    if self.shortcuts.off {
                        ui.label(small(lang.t("help.off"), c.warning));
                    }
                    ui.label(small(lang.t("help.rebind"), c.text_muted));
                    let close = ui.add(kim_ui::Button::secondary(lang.t("btn.close")));
                    if focus {
                        close.request_focus();
                    }
                    if close.clicked() {
                        self.acts.push(Act::CloseDialog);
                    }
                }
            }
        });
        if modal.should_close() {
            self.acts.push(Act::CloseDialog);
        }
    }

    fn export(&mut self, ids: &[String], path: &str) -> String {
        let Some(doc) = self.loaded.doc.as_ref() else {
            return "nothing to export".into();
        };
        let out = transfer::export_rules(doc, ids, "Exported rules");
        match transfer::write_export(std::path::Path::new(path.trim()), &out) {
            Ok(()) => format!("✓ {} {path}", self.t("dialog.export_done")),
            Err(e) => format!("✕ {e}"),
        }
    }

    fn plan_import(&self, path: &str) -> Result<ImportPlan, String> {
        let bytes = std::fs::read(path.trim()).map_err(|e| format!("cannot read {path}: {e}"))?;
        let doc = self.loaded.doc.as_ref().ok_or("rules.json is not usable")?;
        let target = self
            .form
            .as_ref()
            .map_or_else(|| self.active_profile_id(), |f| f.profile.clone());
        transfer::plan_import(&bytes, doc, &target)
    }

    fn import(&mut self, plan: &ImportPlan) {
        let p = plan.clone();
        let n = p.rules.len();
        if let Some(c) = self.commit(
            self.lang.rules_done("toast.imported", n),
            "rule.import",
            move |d| transfer::apply_import(d, &p),
        ) {
            self.selected = c.change.rules;
            self.dialog = None;
            self.filter = Filter::default();
        }
    }
}

/// The small screen map under the Mouse form and in the simulation: every monitor at its place,
/// the simulated pointer (ring) and the target (disc). Nothing moves.
pub(super) fn screen_map(ui: &mut egui::Ui, layout: &Layout, from: Point, to: Option<Point>) {
    let c = Colors::get();
    let (w, h) = (220.0f32, 110.0f32);
    let map = mouse::screen_map(layout, from, to.map(|p| (p, false)), w as i32, h as i32);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, c.bg_base);
    let at = |x: i32, y: i32| rect.min + egui::vec2(x as f32, y as f32);
    for m in &map.monitors {
        let r =
            egui::Rect::from_min_max(at(m.rect.left, m.rect.top), at(m.rect.right, m.rect.bottom));
        p.rect_stroke(
            r.shrink(1.0),
            2.0,
            egui::Stroke::new(
                1.0,
                if m.primary {
                    c.border_strong
                } else {
                    c.border_subtle
                },
            ),
            egui::StrokeKind::Inside,
        );
        p.text(
            r.left_top() + egui::vec2(3.0, 2.0),
            egui::Align2::LEFT_TOP,
            format!("{}%", m.scale_percent),
            egui::FontId::proportional(9.0),
            c.text_muted,
        );
    }
    p.circle_stroke(
        at(map.from.x, map.from.y),
        4.0,
        egui::Stroke::new(1.5, c.text_secondary),
    );
    if let Some(t) = map.to {
        p.circle_filled(at(t.x, t.y), 3.5, c.accent);
    }
}

/// #102: the running programs (file names only, never titles), read at most every 2 s while a
/// picker or a suggestion list is shown.
fn running_programs(ctx: &egui::Context) -> Vec<String> {
    let id = egui::Id::unique("ddalgak-running-programs");
    let now = ctx.input(|i| i.time);
    if let Some((at, list)) = ctx.data(|d| d.get_temp::<(f64, Vec<String>)>(id))
        && now - at < 2.0
    {
        return list;
    }
    let list = ddalgak::win::desktop::running_programs();
    ctx.data_mut(|d| d.insert_temp(id, (now, list.clone())));
    list
}

/// #102: a "Running programs" menu that fills `target` with a program's file name (`lines`:
/// adds it as a new line of a list instead).
pub(super) fn program_picker(ui: &mut egui::Ui, lang: Lang, target: &mut String, lines: bool) {
    ui.menu_button(lang.t("picker.running"), |ui| {
        let list = running_programs(ui.ctx());
        if list.is_empty() {
            ui.label(small(lang.t("picker.none"), Colors::get().text_muted));
        }
        egui::ScrollArea::vertical()
            .max_height(240.0)
            .show(ui, |ui| {
                for name in list {
                    if ui.add(kim_ui::Button::secondary(&name)).clicked() {
                        if lines {
                            if !target.lines().any(|l| l.trim().eq_ignore_ascii_case(&name)) {
                                if !target.is_empty() && !target.ends_with('\n') {
                                    target.push('\n');
                                }
                                target.push_str(&name);
                            }
                        } else {
                            *target = name;
                        }
                        ui.close();
                    }
                }
            });
        ui.label(small(lang.t("picker.note"), Colors::get().text_muted));
    });
}

/// #102: running programs whose name starts with what was typed (at most 5), as buttons.
pub(super) fn program_suggestions(ui: &mut egui::Ui, target: &mut String) {
    let typed = target.trim().to_lowercase();
    if typed.is_empty() {
        return;
    }
    let hits: Vec<String> = running_programs(ui.ctx())
        .into_iter()
        .filter(|n| n.starts_with(&typed) && *n != typed)
        .take(5)
        .collect();
    if !hits.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for n in hits {
                if ui.add(kim_ui::Button::ghost(&n).compact()).clicked() {
                    *target = n;
                }
            }
        });
    }
}

/// #102: the capabilities the installed Kim tools provide, found on request in the background.
#[derive(Clone, Default)]
enum Discovery {
    #[default]
    Idle,
    Searching,
    Done(Option<Vec<ddalgak::win::desktop::Capability>>),
}

pub(super) fn capability_finder(ui: &mut egui::Ui, lang: Lang, target: &mut String) {
    let id = egui::Id::unique("ddalgak-capabilities");
    let slot: Arc<Mutex<Discovery>> = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Arc<Mutex<Discovery>>>(id)
            .clone()
    });
    let state = slot.lock().map(|g| g.clone()).unwrap_or_default();
    let c = Colors::get();
    match state {
        Discovery::Idle => {
            if ui
                .add(kim_ui::Button::secondary(lang.t("field.tool.find")))
                .clicked()
            {
                if let Ok(mut g) = slot.lock() {
                    *g = Discovery::Searching;
                }
                let ctx = ui.ctx().clone();
                let s = Arc::clone(&slot);
                std::thread::spawn(move || {
                    let found = ddalgak::win::desktop::discover_capabilities();
                    if let Ok(mut g) = s.lock() {
                        *g = Discovery::Done(found);
                    }
                    ctx.request_repaint();
                });
            }
        }
        Discovery::Searching => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(small(lang.t("field.tool.searching"), c.text_muted));
            });
        }
        Discovery::Done(None) => {
            ui.label(small(lang.t("field.tool.no_kim"), c.warning));
        }
        Discovery::Done(Some(caps)) => {
            if caps.is_empty() {
                ui.label(small(lang.t("field.tool.none"), c.text_muted));
            }
            for cap in caps {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(*target == cap.id, &cap.id)
                        .on_hover_text(&cap.summary)
                        .clicked()
                    {
                        *target = cap.id.clone();
                    }
                    ui.label(small(
                        format!("{} · {} · {}", cap.tool, cap.effect, cap.summary),
                        c.text_muted,
                    ));
                });
            }
        }
    }
}

pub(super) fn problems_for(
    ui: &mut egui::Ui,
    lang: Lang,
    problems: &[ddalgak::text::FieldProblem],
    prefix: &[&str],
) {
    for p in problems
        .iter()
        .filter(|p| prefix.iter().any(|x| p.path.starts_with(x)))
    {
        kim_ui::field_error(ui, &lang.problem(&p.message));
    }
}

/// The keys a picker offers for a trigger (contract names, the lock keys, bare side
/// modifiers and the mouse buttons Ddalgak can use as triggers).
pub(crate) fn trigger_keys() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = vec![
        "CapsLock",
        "Tab",
        "Space",
        "Enter",
        "Escape",
        "Backspace",
        "Up",
        "Down",
        "Left",
        "Right",
        "LCtrl",
        "RCtrl",
        "LShift",
        "RShift",
        "LAlt",
        "RAlt",
        "NumLock",
        "ScrollLock",
        "Pause",
        "Insert",
        "Delete",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "PrintScreen",
        "ContextMenu",
        "MouseMiddle",
        "MouseX1",
        "MouseX2",
    ];
    for k in kim_hotkeys::keys::NAMED_KEYS {
        if !v.contains(k) {
            v.push(k);
        }
    }
    v
}
