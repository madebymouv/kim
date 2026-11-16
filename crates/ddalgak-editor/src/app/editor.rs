//! ADR 0100: the rule editor — one page that reads as a sentence, *When I press [keys], do
//! [action]*: four numbered sections on one scrolling column (Trigger, Action, the chosen
//! action's settings, Test and save) and a fixed footer with Cancel and Save. Only the chosen
//! action's fields are shown; conditions, priority, the description and macro steps wait under
//! *Advanced*. Every control is a `kim-ui` component (one control height, one field rhythm),
//! so the editor cannot drift into one-off sizes again.
//!
//! The data flow is unchanged: the fields are [`RuleForm`]'s, checked by its `problems`, saved
//! by [`Act::SaveForm`] through the same reviewed store transaction.

use super::*;
use crate::form::Combo;
use kim_design::{font, size, space};
use kim_ui::Icon;
use view::{capability_finder, problems_for, program_picker, program_suggestions, screen_map};

fn caption(text: impl Into<String>, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(font::CAPTION)
        .color(color)
}

/// ADR 0101: the remap targets offered as one-click buttons under *Acts as*.
const QUICK_TARGETS: [&str; 10] = [
    "Up", "Down", "Left", "Right", "Home", "End", "PageUp", "PageDown", "Escape", "Enter",
];

fn kim_lang(lang: Lang) -> kim_design::Lang {
    match lang {
        Lang::Ko => kim_design::Lang::Ko,
        Lang::En => kim_design::Lang::En,
    }
}

/// A full-width single-line text field.
fn line(ui: &mut egui::Ui, value: &mut String, label: &str, hint: &str) -> egui::Response {
    let w = ui.available_width();
    kim_ui::text_field(ui, value, label, hint, w)
}

/// A number field of the standard width.
fn number<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    suffix: &str,
    label: &str,
) -> egui::Response {
    kim_ui::number_field(ui, value, range, suffix, label, 120.0)
}

/// One row of fields side by side (each a labelled field).
fn pair(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = space::L;
        add(ui);
    });
}

/// The keys of one field: the keys as keycaps in a field that records what is pressed while it
/// is active (click it or the record button), the key picker (keys that cannot be pressed in a
/// window: CapsLock, left/right modifiers, mouse buttons) and **Type**, which opens a text field
/// for key names (keys Windows takes first). The keys follow in words. Returns `true` when the
/// picker should open.
#[allow(clippy::too_many_arguments)]
fn key_input(
    ui: &mut egui::Ui,
    lang: Lang,
    label: &str,
    value: &mut String,
    capture: &mut Capture,
    target: &mut CaptureTarget,
    me: CaptureTarget,
    help: Option<&str>,
) -> bool {
    let c = Colors::get();
    let mut pick = false;
    let typing_id = ui.scope_id().with(("ddalgak-type-keys", label));
    let mut typing: bool = ui.data(|d| d.get_temp(typing_id)).unwrap_or(false);
    let type_label = lang.t("form.type_keys");
    kim_ui::field(ui, label, help, |ui| {
        kim_ui::toolbar(ui, |ui| {
            let recording = capture.active && *target == me;
            let tw = ui
                .painter()
                .layout_no_wrap(
                    type_label.to_string(),
                    kim_ui::ui_font(ui.ctx(), font::BODY, false),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
                + 2.0 * size::BUTTON_PAD_X;
            let reserved = 2.0 * size::CONTROL + tw + 3.0 * space::S;
            let w = kim_ui::remaining_width(ui, reserved).max(160.0);
            let field = kim_ui::shortcut_field(
                ui,
                &format!("{}: {label}", lang.t("form.capture_label")),
                value.trim(),
                recording,
                lang.t("form.capture_empty"),
                lang.t("form.capture"),
                w,
            );
            let rec = kim_ui::icon_button_sized(
                ui,
                Icon::Record,
                &format!("{}: {label}", lang.t("form.record")),
                size::CONTROL,
                recording,
            );
            if field.clicked() || rec.clicked() {
                if recording && rec.clicked() {
                    capture.active = false;
                } else {
                    capture.active = true;
                    *target = me;
                    field.request_focus();
                }
            }
            if recording
                && !field.has_focus()
                && !field.clicked()
                && !rec.clicked()
                && field.ctx.memory(|m| m.focused().is_some())
            {
                capture.active = false;
            }
            if kim_ui::icon_button(
                ui,
                Icon::Keyboard,
                &format!("{}: {label}", lang.t("form.pick_keys")),
            )
            .clicked()
            {
                pick = true;
            }
            let tb = ui.add(
                kim_ui::Button::ghost(type_label)
                    .selected(typing)
                    .named(format!("{type_label}: {label}")),
            );
            if tb.on_hover_text(lang.t("form.type_instead")).clicked() {
                typing = !typing;
            }
        });
        if typing {
            let r = kim_ui::text_field(
                ui,
                value,
                &format!("{}: {label}", lang.t("form.type_instead")),
                lang.t("form.type_instead"),
                ui.available_width(),
            );
            if r.gained_focus() {
                capture.active = false;
            }
        }
        if !value.trim().is_empty() {
            ui.label(caption(capture::announce(value.trim()), c.text_muted));
        }
    });
    ui.data_mut(|d| d.insert_temp(typing_id, typing));
    pick
}

impl EditorApp {
    /// The rule editor page (the form is open).
    pub(super) fn editor_page(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        let profile = {
            let pid = self
                .form
                .as_ref()
                .map(|f| f.profile.clone())
                .unwrap_or_default();
            self.loaded
                .doc
                .as_ref()
                .and_then(|d| d.profile(&pid))
                .cloned()
        };
        let Some(f) = self.form.as_ref() else {
            return;
        };
        let heading = if f.id.is_some() {
            "form.edit"
        } else {
            "form.new"
        };
        let existing = f.id.clone();
        // The header: back to the list, the title.
        let mut back = false;
        let mut test = None;
        ui.horizontal(|ui| {
            back = ui
                .add(kim_ui::Button::ghost("← Rules").compact())
                .on_hover_text(lang.t("form.back_hint"))
                .clicked();
            kim_ui::title(ui, lang.t(heading));
            if let Some(id) = existing.clone() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(kim_ui::Button::ghost("Test in Playground").compact())
                        .on_hover_text(lang.t("play.hint_rule"))
                        .clicked()
                    {
                        test = Some(id);
                    }
                });
            }
        });
        if back {
            self.acts.push(Act::CloseForm);
        }
        if let Some(id) = test {
            self.acts.push(Act::Test(id));
        }
        ui.add_space(space::S);
        let problems = self
            .form
            .as_ref()
            .map(|f| f.problems(profile.as_ref()))
            .unwrap_or_default();
        // The footer stays put while the sections scroll.
        egui::Panel::bottom("rule-editor-actions")
            .frame(
                egui::Frame::new()
                    .fill(c.bg_base)
                    .inner_margin(egui::Margin::symmetric(0, space::M as i8)),
            )
            .show_separator_line(true)
            .show(ui, |ui| self.editor_footer(ui, profile.as_ref(), &problems));
        if self.banner.is_some() {
            self.banner_ui(ui);
        }
        if let Some(why) = self.form.as_ref().and_then(|f| f.unsupported.clone()) {
            kim_ui::notice(
                ui,
                kim_ui::Tone::Warning,
                &format!("{}: {why}", lang.t("form.not_editable")),
                None,
                None,
            );
            return;
        }
        if self.external_change {
            let mut reload = false;
            if kim_ui::notice(
                ui,
                kim_ui::Tone::Warning,
                lang.t("form.changed_elsewhere"),
                None,
                Some(lang.t("btn.reload")),
            ) {
                reload = true;
            }
            if reload {
                self.acts.push(Act::ReloadFromDisk);
            }
        }
        let width = ui.available_width().min(size::FORM_MAX_WIDTH);
        egui::ScrollArea::vertical()
            // A new scroll position for every opened rule: it starts at the top.
            .id_salt(("rule-editor", self.form_gen))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(width);
                self.editor_sections(ui, profile.as_ref(), &problems);
            });
    }

    fn editor_sections(
        &mut self,
        ui: &mut egui::Ui,
        profile: Option<&Profile>,
        problems: &[ddalgak::text::FieldProblem],
    ) {
        let lang = self.lang;
        let Some(f) = self.form.as_mut() else {
            return;
        };
        // A new, untouched rule shows no errors yet; they appear once something changed.
        let shown: Vec<_> = if f.id.is_none() && f.fields == f.pristine {
            Vec::new()
        } else {
            problems.to_vec()
        };
        // The rule as a sentence, once it says something.
        let caps_hold = f
            .fields
            .trigger
            .hold_key
            .trim()
            .eq_ignore_ascii_case("CapsLock");
        let keys = if f.fields.trigger.combo == Combo::Hold
            && !f.fields.trigger.hold_key.trim().is_empty()
            && !caps_hold
        {
            format!(
                "{} + {}",
                f.fields.trigger.hold_key.trim(),
                f.trigger_text()
            )
        } else {
            f.trigger_text()
        };
        let summary = f.summary(profile);
        let sentence = f.sentence(profile, lang);
        self.rule_sentence(ui, &keys, &summary, &sentence);
        let mut capture = std::mem::take(&mut self.capture);
        let mut target = self.capture_target;
        let mut pick = None;
        let Some(f) = self.form.as_mut() else {
            return;
        };
        kim_ui::form_section(
            ui,
            Some(1),
            "Trigger",
            Some(lang.t("form.trigger_sub")),
            |ui| {
                pick = trigger_section(ui, lang, f, &mut capture, &mut target, &shown);
            },
        );
        let effect_before = f.fields.effect;
        kim_ui::form_section(
            ui,
            Some(2),
            "Action",
            Some(lang.t("form.action_sub")),
            |ui| {
                action_section(ui, lang, f, profile);
            },
        );
        // ADR 0101: a chosen action scrolls its settings into view (measured: they sat about
        // 700 px below a 900 px window, so every rule needed a long scroll).
        let chosen = f.fields.effect.is_some() && f.fields.effect != effect_before;
        let mut acts = Vec::new();
        if let Some(effect) = f.fields.effect {
            let title = lang.t(&format!("tile.{}", effect.key()));
            let example = lang.t(&format!("tile.{}.example", effect.key()));
            let focus_title = std::mem::take(&mut self.focus_title);
            if self.audit_details || chosen {
                ui.scroll_to_cursor(Some(egui::Align::TOP));
            }
            let _ = example;
            kim_ui::form_section(
                ui,
                Some(3),
                title,
                Some(lang.t("form.settings_sub")),
                |ui| {
                    details_section(
                        ui,
                        lang,
                        f,
                        profile,
                        &mut capture,
                        &mut target,
                        &shown,
                        &self.layout,
                        self.cursor,
                        &mut self.new_mode_title,
                        &mut acts,
                        focus_title,
                        &mut pick,
                    );
                },
            );
        }
        self.capture = capture;
        self.capture_target = target;
        self.acts.extend(acts);
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.fields.effect.is_some() || !f.fields.macro_steps.is_empty())
        {
            let open = self
                .form
                .as_ref()
                .is_some_and(|f| !f.fields.macro_steps.is_empty())
                || self.recorder.is_on()
                || self.mouse_rec.is_some();
            ui.add_space(-space::S);
            egui::CollapsingHeader::new(format!("Advanced · {}", lang.t("macro.title")))
                .id_salt("macro-section")
                .default_open(open)
                .show(ui, |ui| self.macro_section(ui, profile));
            ui.add_space(space::M);
        }
        kim_ui::form_section(
            ui,
            Some(4),
            "Test and save",
            Some(lang.t("form.test_sub")),
            |ui| {
                self.test_section(ui, profile, problems);
            },
        );
        if let Some(t) = pick {
            self.capture.active = false;
            let mut p = kim_ui::keys::KeyPicker::default();
            // The picker opens on what the field holds.
            if let Some(f) = &self.form {
                let cur = match t {
                    CaptureTarget::Trigger => f.fields.trigger.keys.clone(),
                    CaptureTarget::HoldKey => f.fields.trigger.hold_key.clone(),
                    CaptureTarget::RemapTo => f.fields.remap.to.clone(),
                    CaptureTarget::RemapTap => f.fields.remap.tap.clone(),
                    CaptureTarget::SendKeys => f.fields.keys.keys.clone(),
                };
                if !cur.contains(',') && !cur.contains(' ') {
                    p = kim_ui::keys::KeyPicker::from_chord(&cur, &view::trigger_keys());
                }
            }
            self.key_picker = Some((t, p));
        }
        self.key_picker_dialog(ui.ctx());
    }

    /// *When I press [CapsLock] → Types "…"*: the rule in one line above the sections.
    fn rule_sentence(&self, ui: &mut egui::Ui, keys: &str, summary: &str, sentence: &str) {
        let c = Colors::get();
        let lang = self.lang;
        egui::Frame::new()
            .fill(c.accent_soft)
            .corner_radius(egui::CornerRadius::same(kim_design::radius::M as u8))
            .inner_margin(egui::Margin::symmetric(space::M as i8, space::S as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(size::CONTROL);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = space::S;
                    ui.label(caption(lang.t("form.when_press"), c.text_secondary));
                    if keys.trim().is_empty() {
                        ui.label(caption(lang.t("form.sentence_keys"), c.text_muted));
                    } else {
                        kim_ui::keycaps(ui, keys.trim());
                    }
                    ui.label(caption("→", c.text_muted));
                    if summary.trim().is_empty() || summary.trim() == "—" {
                        ui.label(caption(lang.t("form.sentence_action"), c.text_muted));
                    } else {
                        ui.label(
                            egui::RichText::new(summary)
                                .font(kim_ui::ui_font(ui.ctx(), font::BODY, false))
                                .color(c.text_primary),
                        );
                    }
                });
                // ADR 0101: the whole rule in words, in the window's language.
                if !sentence.is_empty() {
                    let r = ui.label(caption(sentence, c.text_secondary));
                    kim_ui::inspect::record(
                        ui,
                        "rule-editor:sentence",
                        "text",
                        r.rect,
                        Some(kim_ui::inspect::Painted {
                            content: sentence,
                            rect: r.rect,
                            size: font::CAPTION,
                            truncated: false,
                        }),
                        false,
                        &[],
                        Some(c.text_secondary),
                        None,
                    );
                }
            });
        ui.add_space(space::M);
    }

    /// The last section: the simulation (nothing is pressed or opened), what still blocks the
    /// rule, what it is allowed to do.
    fn test_section(
        &mut self,
        ui: &mut egui::Ui,
        profile: Option<&Profile>,
        problems: &[ddalgak::text::FieldProblem],
    ) {
        let c = Colors::get();
        let lang = self.lang;
        let key = (
            self.form.as_ref().map(|f| f.fields.clone()),
            self.loaded.hash,
        );
        let doc_problems = match &self.doc_check {
            Some((k, p)) if *k == key => p.clone(),
            _ => {
                let p = self.candidate_problems(profile);
                self.doc_check = Some((key, p.clone()));
                p
            }
        };
        let Some(f) = self.form.as_mut() else {
            return;
        };
        let untouched = f.id.is_none() && f.fields == f.pristine;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    problems.is_empty(),
                    kim_ui::Button::secondary(lang.t("form.simulate")),
                )
                .on_hover_text(lang.t("form.try_intro"))
                .clicked()
            {
                let sim = f.simulate(profile, &self.layout, self.cursor);
                self.test = Some((f.id.clone().unwrap_or_default(), sim));
            }
            ui.label(caption(lang.t("form.try_intro"), c.text_muted));
        });
        if let Some((_, sim)) = &self.test {
            Self::simulation_ui(ui, lang, sim, &self.layout);
        }
        if !untouched {
            for p in problems {
                kim_ui::field_error(ui, &lang.problem(&p.message));
            }
        } else if !problems.is_empty() {
            ui.label(caption(lang.t("form.fill_in"), c.text_muted));
        }
        for p in &doc_problems {
            let tone = if p.severity == ddalgak::validate::Severity::Error {
                kim_ui::Tone::Error
            } else {
                kim_ui::Tone::Warning
            };
            kim_ui::notice(ui, tone, &p.message, Some(p.code), None);
            for o in &p.offers {
                if let ddalgak::validate::Offer::Rebind(k) = o
                    && ui
                        .add(
                            kim_ui::Button::ghost(format!("{} {k}", lang.t("form.try_keys")))
                                .compact(),
                        )
                        .clicked()
                {
                    f.fields.trigger.keys = k.clone();
                }
            }
        }
        let perms = f.permissions(profile);
        if !perms.is_empty() {
            let words: Vec<&str> = perms
                .iter()
                .map(|p| lang.t(&format!("perm.short.{}", p.as_str())))
                .collect();
            ui.label(caption(
                format!("{}: {}", lang.t("form.permission"), words.join(", ")),
                c.text_secondary,
            ));
        }
        if f.id.is_some() {
            kim_ui::checkbox(ui, &mut f.fields.enabled, lang.t("form.enabled"));
        }
        self.doc_blocked = doc_problems
            .iter()
            .any(|p| p.severity == ddalgak::validate::Severity::Error);
    }

    /// Cancel on the left; Save (or, for a new rule, Save off / Save and turn on) on the right.
    fn editor_footer(
        &mut self,
        ui: &mut egui::Ui,
        _profile: Option<&Profile>,
        problems: &[ddalgak::text::FieldProblem],
    ) {
        let lang = self.lang;
        let blocked = !problems.is_empty() || self.doc_blocked;
        let Some(f) = self.form.as_mut() else {
            return;
        };
        kim_ui::inspect::scope("rule-editor.footer", || {
            kim_ui::toolbar(ui, |ui| {
                if ui
                    .add(kim_ui::Button::ghost(lang.t("btn.cancel")))
                    .clicked()
                {
                    self.acts.push(Act::CloseForm);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = space::S;
                    if f.id.is_some() {
                        if ui
                            .add_enabled(
                                !blocked && f.dirty(),
                                kim_ui::Button::primary(lang.t("btn.save")),
                            )
                            .clicked()
                        {
                            self.acts.push(Act::SaveForm);
                        }
                    } else {
                        if ui
                            .add_enabled(
                                !blocked,
                                kim_ui::Button::primary(lang.t("btn.save_enable")),
                            )
                            .clicked()
                        {
                            f.fields.enabled = true;
                            self.acts.push(Act::SaveForm);
                        }
                        if ui
                            .add_enabled(
                                !blocked,
                                kim_ui::Button::secondary(lang.t("btn.save_off")),
                            )
                            .clicked()
                        {
                            f.fields.enabled = false;
                            self.acts.push(Act::SaveForm);
                        }
                    }
                    if blocked && !(f.id.is_none() && f.fields == f.pristine) {
                        ui.label(caption(lang.t("form.blocked"), Colors::get().text_muted));
                    }
                });
            });
        });
    }
}

/// Section 1: what starts the rule — keys (how they combine, the keys, rarely needed options),
/// a typed abbreviation or a schedule. Returns the field whose keys should be picked.
fn trigger_section(
    ui: &mut egui::Ui,
    lang: Lang,
    f: &mut RuleForm,
    capture: &mut Capture,
    target: &mut CaptureTarget,
    problems: &[ddalgak::text::FieldProblem],
) -> Option<CaptureTarget> {
    let c = Colors::get();
    let allows_text = f.fields.effect.is_none_or(Effect::allows_hotstring);
    let t = &mut f.fields.trigger;
    let mut pick = None;
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Keys,
        Text,
        Schedule,
    }
    let mut kind = if t.scheduled {
        Kind::Schedule
    } else if t.hotstring && allows_text {
        Kind::Text
    } else {
        Kind::Keys
    };
    let mut kinds = vec![(Kind::Keys, "Keys")];
    if allows_text {
        kinds.push((Kind::Text, "Typed text"));
    }
    kinds.push((Kind::Schedule, "Schedule"));
    kim_ui::segmented(ui, &mut kind, &kinds);
    t.scheduled = kind == Kind::Schedule;
    t.hotstring = kind == Kind::Text;
    ui.add_space(space::M);
    if t.scheduled {
        let mut every = t.every.is_some();
        kim_ui::field(ui, lang.t("form.schedule"), None, |ui| {
            kim_ui::segmented(
                ui,
                &mut every,
                &[
                    (true, lang.t("form.schedule_every")),
                    (false, lang.t("form.schedule_at")),
                ],
            );
        });
        if every && t.every.is_none() {
            t.every = Some(60);
        } else if !every {
            t.every = None;
        }
        match &mut t.every {
            Some(n) => {
                kim_ui::field(ui, lang.t("form.schedule_every"), None, |ui| {
                    number(ui, n, 1..=1440, " min", lang.t("form.schedule_every"));
                });
            }
            None => {
                kim_ui::field(ui, lang.t("form.schedule_at"), None, |ui| {
                    kim_ui::text_field(ui, &mut t.at, lang.t("form.schedule_at"), "09:00", 120.0);
                });
                kim_ui::field(ui, lang.t("form.days"), None, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (i, d) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                            .iter()
                            .enumerate()
                        {
                            kim_ui::checkbox(ui, &mut t.days[i], d);
                        }
                    });
                });
            }
        }
        kim_ui::checkbox(ui, &mut t.catch_up, lang.t("form.schedule_catch_up"));
        ui.label(caption(lang.t("form.schedule_note"), c.text_muted));
        problems_for(ui, lang, problems, &["trigger"]);
        return None;
    }
    if t.hotstring {
        kim_ui::field(
            ui,
            lang.t("form.abbreviation"),
            Some(lang.t("form.abbreviation_help")),
            |ui| {
                kim_ui::text_field(
                    ui,
                    &mut t.abbreviation,
                    lang.t("form.abbreviation"),
                    "btw",
                    200.0,
                );
            },
        );
        problems_for(ui, lang, problems, &["trigger"]);
        return None;
    }
    // How the keys combine: never one silently treated as another.
    kim_ui::field(
        ui,
        lang.t("form.combo_label"),
        Some(lang.t(match t.combo {
            Combo::Together => "form.combo_together_hint",
            Combo::Sequence => "form.combo_sequence_hint",
            Combo::Hold => "form.combo_hold_hint",
        })),
        |ui| {
            kim_ui::segmented(
                ui,
                &mut t.combo,
                &[
                    (Combo::Together, "Together"),
                    (Combo::Sequence, "One after another"),
                    (Combo::Hold, "Hold + press"),
                ],
            );
        },
    );
    capture.add_step = t.combo == Combo::Sequence;
    if t.combo == Combo::Hold
        && key_input(
            ui,
            lang,
            lang.t("form.hold_key"),
            &mut t.hold_key,
            capture,
            target,
            CaptureTarget::HoldKey,
            None,
        )
    {
        pick = Some(CaptureTarget::HoldKey);
    }
    let label = lang.t(if t.combo == Combo::Hold {
        "form.press_key"
    } else {
        "form.keys"
    });
    if key_input(
        ui,
        lang,
        label,
        &mut t.keys,
        capture,
        target,
        CaptureTarget::Trigger,
        Some(lang.t("form.capture_hint")),
    ) {
        pick = Some(CaptureTarget::Trigger);
    }
    if capture.composing {
        ui.label(egui::RichText::new(lang.t("form.ime")).color(c.warning));
    }
    if t.combo == Combo::Hold && t.hold_key.trim().is_empty() && !t.keys.trim().is_empty() {
        kim_ui::field_error(ui, lang.t("form.hold_key_missing"));
    }
    egui::CollapsingHeader::new(lang.t("form.trigger_options"))
        .id_salt("trigger-advanced")
        .show(ui, |ui| {
            kim_ui::field(ui, lang.t("form.fires"), None, |ui| {
                kim_ui::segmented(
                    ui,
                    &mut t.on,
                    &[
                        (TriggerOn::Press, lang.t("form.on_press")),
                        (TriggerOn::Release, lang.t("form.on_release")),
                        (TriggerOn::Hold, lang.t("form.on_hold")),
                    ],
                );
            });
            if t.on == TriggerOn::Hold {
                kim_ui::field(ui, lang.t("form.hold_ms"), None, |ui| {
                    number(
                        ui,
                        &mut t.hold_ms,
                        200..=5000,
                        " ms",
                        lang.t("form.hold_ms"),
                    );
                });
            }
            kim_ui::checkbox(ui, &mut t.pass_through, lang.t("form.pass_through"));
        });
    problems_for(ui, lang, problems, &["trigger"]);
    pick
}

/// Section 2: the action types as cards of one size in an even grid.
fn action_section(ui: &mut egui::Ui, lang: Lang, f: &mut RuleForm, profile: Option<&Profile>) {
    let all: Vec<Effect> = Effect::TILES.into_iter().chain(Effect::MORE).collect();
    let gap = space::S;
    let w = ui.available_width();
    let cols = (((w + gap) / (size::CHOICE_CARD_MIN_WIDTH + gap)).floor() as usize).clamp(1, 4);
    let card_w = ((w - gap * (cols - 1) as f32) / cols as f32).floor();
    kim_ui::inspect::scope("rule-editor.actions", || {
        for chunk in all.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for e in chunk {
                    let title = lang.t(&format!("tile.{}", e.key()));
                    let example = lang.t(&format!("tile.{}.example", e.key()));
                    let chosen = f.fields.effect == Some(*e);
                    if kim_ui::choice_card(ui, chosen, e.icon(), title, example, card_w).clicked() {
                        f.fields.effect = Some(*e);
                        f.refresh_title(profile, lang);
                    }
                }
            });
            ui.add_space(gap - ui.spacing().item_spacing.y);
        }
    });
}

/// Section 3: the name and the chosen action's own fields (nothing else); the conditions under
/// *Advanced*.
#[allow(clippy::too_many_arguments)]
fn details_section(
    ui: &mut egui::Ui,
    lang: Lang,
    f: &mut RuleForm,
    profile: Option<&Profile>,
    capture: &mut Capture,
    target: &mut CaptureTarget,
    problems: &[ddalgak::text::FieldProblem],
    layout: &Layout,
    cursor: Point,
    new_mode: &mut String,
    acts: &mut Vec<Act>,
    focus_title: bool,
    pick: &mut Option<CaptureTarget>,
) {
    let c = Colors::get();
    let Some(effect) = f.fields.effect else {
        return;
    };
    kim_ui::field(ui, lang.t("form.title"), None, |ui| {
        let name = line(
            ui,
            &mut f.fields.title,
            lang.t("form.title"),
            lang.t("form.title_hint"),
        );
        if focus_title {
            name.request_focus();
        }
        if name.changed() {
            f.fields.title_auto = false;
        }
    });
    problems_for(ui, lang, problems, &["title"]);
    let fl = &mut f.fields;
    match effect {
        Effect::Remap => {
            if key_input(
                ui,
                lang,
                lang.t("field.acts_as"),
                &mut fl.remap.to,
                capture,
                target,
                CaptureTarget::RemapTo,
                None,
            ) {
                *pick = Some(CaptureTarget::RemapTo);
            }
            // ADR 0101: the targets people remap to most, one click each (the picker has the
            // rest). Measured on the real desktop: a CapsLock+W → Up rule took 12–13 clicks
            // through the picker, 10 with these.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(space::XS, space::XS);
                ui.label(caption(lang.t("field.quick_keys"), c.text_muted));
                for k in QUICK_TARGETS {
                    let shown = kim_ui::keys::readable_key(k, kim_lang(lang));
                    if ui
                        .add(
                            kim_ui::Button::secondary(k)
                                .compact()
                                .selected(fl.remap.to.trim() == k)
                                .named(format!("{}: {k}", lang.t("field.acts_as"))),
                        )
                        .on_hover_text(shown)
                        .clicked()
                    {
                        fl.remap.to = k.to_string();
                    }
                }
            });
            if key_input(
                ui,
                lang,
                lang.t("field.tap"),
                &mut fl.remap.tap,
                capture,
                target,
                CaptureTarget::RemapTap,
                Some(lang.t("field.tap_help")),
            ) {
                *pick = Some(CaptureTarget::RemapTap);
            }
            if !fl.remap.tap.trim().is_empty() {
                kim_ui::field(ui, lang.t("field.tap_ms"), None, |ui| {
                    number(
                        ui,
                        &mut fl.remap.tap_ms,
                        100..=1000,
                        " ms",
                        lang.t("field.tap_ms"),
                    );
                });
            }
            let trig = fl.trigger.keys.clone();
            if ddalgak::remap::widened(&trig) && !fl.remap.to.trim().is_empty() {
                ui.label(caption(
                    format!(
                        "{} Shift+{trig} → Shift+{}",
                        lang.t("form.held_modifiers"),
                        fl.remap.to.trim()
                    ),
                    c.text_muted,
                ));
            }
            problems_for(
                ui,
                lang,
                problems,
                &["steps[0].to", "steps[0].tap", "trigger.pass_through"],
            );
        }
        Effect::Block => {
            ui.label(caption(lang.t("tile.block.example"), c.text_muted));
        }
        Effect::SendKeys => {
            if key_input(
                ui,
                lang,
                lang.t("field.shortcut"),
                &mut fl.keys.keys,
                capture,
                target,
                CaptureTarget::SendKeys,
                None,
            ) {
                *pick = Some(CaptureTarget::SendKeys);
            }
            kim_ui::checkbox(ui, &mut capture.add_step, lang.t("form.add_step"));
            kim_ui::field(ui, lang.t("field.step_delay"), None, |ui| {
                number(
                    ui,
                    &mut fl.keys.step_delay_ms,
                    0..=1000,
                    " ms",
                    lang.t("field.step_delay"),
                );
            });
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::TypeText => {
            kim_ui::field(ui, lang.t("field.text"), None, |ui| {
                kim_ui::text_area(
                    ui,
                    &mut fl.text.text,
                    lang.t("field.text"),
                    lang.t("field.text_hint"),
                    4,
                );
            });
            kim_ui::checkbox(ui, &mut fl.text.variables, lang.t("field.variables"));
            kim_ui::checkbox(ui, &mut fl.text.paste, lang.t("field.paste"));
            kim_ui::field(ui, lang.t("field.key_delay"), None, |ui| {
                number(
                    ui,
                    &mut fl.text.key_delay_ms,
                    0..=200,
                    " ms",
                    lang.t("field.key_delay"),
                );
            });
            let preview = f.text_draft().preview(&form::local_now());
            if !preview.rendered.is_empty() {
                kim_ui::field(
                    ui,
                    lang.t("form.as_typed"),
                    Some(lang.t("form.plain_note")),
                    |ui| {
                        let mut shown = preview.rendered;
                        ui.add_enabled_ui(false, |ui| {
                            kim_ui::text_area(ui, &mut shown, lang.t("form.as_typed"), "", 2);
                        });
                    },
                );
            }
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::OpenProgram => {
            kim_ui::field(ui, lang.t("field.program"), None, |ui| {
                line(
                    ui,
                    &mut fl.program.path,
                    lang.t("field.program"),
                    r"C:\Windows\System32\notepad.exe",
                );
            });
            kim_ui::field(ui, lang.t("field.args"), None, |ui| {
                kim_ui::text_area(ui, &mut fl.program.args, lang.t("field.args"), "", 2);
            });
            kim_ui::field(ui, lang.t("field.window"), None, |ui| {
                let mut w = match fl.program.window.as_str() {
                    "maximized" => 1,
                    "minimized" => 2,
                    _ => 0,
                };
                if kim_ui::segmented(
                    ui,
                    &mut w,
                    &[
                        (0, lang.t("field.window.normal")),
                        (1, lang.t("field.window.maximized")),
                        (2, lang.t("field.window.minimized")),
                    ],
                ) {
                    fl.program.window = ["normal", "maximized", "minimized"][w].to_string();
                }
            });
            kim_ui::checkbox(ui, &mut fl.program.activate, lang.t("field.activate"));
            egui::CollapsingHeader::new(lang.t("field.program_more"))
                .id_salt("program-more")
                .show(ui, |ui| {
                    kim_ui::field(ui, lang.t("field.app_id"), None, |ui| {
                        line(
                            ui,
                            &mut fl.program.app_id,
                            lang.t("field.app_id"),
                            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
                        );
                    });
                    kim_ui::field(ui, lang.t("field.start_in"), None, |ui| {
                        line(
                            ui,
                            &mut fl.program.working_dir,
                            lang.t("field.start_in"),
                            r"C:\Users\me",
                        );
                    });
                });
            let d = f.program_draft();
            let pv = d.preview(&ddalgak::launch::process_var);
            if !pv.program.is_empty() {
                let (color, word) = if pv.found && pv.is_program {
                    (c.success, lang.t("form.found"))
                } else {
                    (c.warning, lang.t("form.not_found"))
                };
                ui.label(caption(format!("{} — {word}", pv.program), color));
            }
            if let Some(cl) = d.command_line(&ddalgak::launch::process_var) {
                ui.label(caption(
                    format!("{}: {cl}", lang.t("form.command_line")),
                    c.text_muted,
                ));
            }
            ui.label(caption(lang.t("form.program_note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::OpenUrl => {
            kim_ui::field(
                ui,
                lang.t("field.url"),
                Some(lang.t("form.program_note")),
                |ui| {
                    line(ui, &mut fl.url, lang.t("field.url"), "github.com");
                },
            );
            match f.url_draft().normalized() {
                Ok(u) if !f.fields.url.trim().is_empty() => {
                    ui.label(caption(format!("{}: {u}", lang.t("form.opens")), c.success));
                }
                _ => {}
            }
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Mouse => mouse_fields(ui, lang, f, problems, layout, cursor),
        Effect::Mode => {
            kim_ui::field(ui, lang.t("field.switch"), None, |ui| {
                kim_ui::segmented(
                    ui,
                    &mut fl.mode.op,
                    &[
                        (ModeOp::Set, "Switch to"),
                        (ModeOp::Toggle, "Toggle"),
                        (ModeOp::Hold, "Hold"),
                        (ModeOp::Cycle, "Cycle"),
                        (ModeOp::Previous, "Back"),
                        (ModeOp::Default, "Default"),
                    ],
                );
            });
            let modes: Vec<(String, String)> = profile
                .map(|p| {
                    p.modes
                        .iter()
                        .map(|m| (m.id.clone(), m.title.clone()))
                        .collect()
                })
                .unwrap_or_default();
            match fl.mode.op {
                ModeOp::Set | ModeOp::Toggle | ModeOp::Hold => {
                    let mut opts = vec![(String::new(), "—".to_string())];
                    opts.extend(modes.iter().cloned());
                    kim_ui::field(ui, lang.t("field.mode"), None, |ui| {
                        kim_ui::dropdown(
                            ui,
                            "mode-target",
                            lang.t("field.mode"),
                            &mut fl.mode.mode,
                            &opts,
                        );
                    });
                }
                ModeOp::Cycle => {
                    kim_ui::field(ui, lang.t("field.cycle_modes"), None, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (id, t) in &modes {
                                let mut on = fl.mode.modes.contains(id);
                                if kim_ui::checkbox(ui, &mut on, t).changed() {
                                    if on {
                                        fl.mode.modes.push(id.clone());
                                    } else {
                                        fl.mode.modes.retain(|m| m != id);
                                    }
                                }
                            }
                        });
                    });
                }
                ModeOp::Previous | ModeOp::Default => {}
            }
            kim_ui::field(ui, lang.t("field.new_mode"), None, |ui| {
                kim_ui::toolbar(ui, |ui| {
                    let w = kim_ui::remaining_width(ui, size::BUTTON_MIN_WIDTH + space::S + 24.0);
                    kim_ui::text_field(
                        ui,
                        new_mode,
                        lang.t("field.new_mode"),
                        lang.t("field.new_mode"),
                        w.min(280.0),
                    );
                    if ui
                        .add_enabled(
                            !new_mode.trim().is_empty(),
                            kim_ui::Button::secondary(lang.t("modes.add")),
                        )
                        .clicked()
                    {
                        acts.push(Act::NewMode(new_mode.clone()));
                    }
                });
            });
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Engine => {
            kim_ui::field(
                ui,
                lang.t("field.engine_op"),
                Some(lang.t("field.engine_note")),
                |ui| {
                    kim_ui::segmented(
                        ui,
                        &mut fl.engine,
                        &[
                            (EngineOp::TogglePause, lang.t("field.engine.toggle")),
                            (EngineOp::Pause, lang.t("field.engine.pause")),
                            (EngineOp::Resume, lang.t("field.engine.resume")),
                            (EngineOp::Disable, lang.t("field.engine.disable")),
                        ],
                    );
                },
            );
        }
        // ADR 0101: media and volume keys by name, never as key codes.
        Effect::Media | Effect::Volume => {
            let (list, value, label): (&[(&str, &str, &str)], &mut String, &str) =
                if effect == Effect::Media {
                    (&form::MEDIA_KEYS, &mut fl.media, lang.t("field.media"))
                } else {
                    (&form::VOLUME_KEYS, &mut fl.volume, lang.t("field.volume"))
                };
            kim_ui::field(ui, label, Some(lang.t("field.media_note")), |ui| {
                let opts: Vec<(usize, &str)> = list
                    .iter()
                    .enumerate()
                    .map(|(i, (_, en, ko))| (i, if lang == Lang::Ko { *ko } else { *en }))
                    .collect();
                let mut i = list.iter().position(|(k, _, _)| k == value).unwrap_or(0);
                kim_ui::segmented(ui, &mut i, &opts);
                *value = list[i].0.to_string();
            });
            if effect == Effect::Volume && fl.volume != "VolumeMute" {
                ui.label(caption(lang.t("field.volume_repeat"), c.text_muted));
            }
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Settings => {
            ui.label(caption(lang.t("tile.settings.note"), c.text_muted));
        }
        Effect::Window => {
            use ddalgak::model::WindowOp as W;
            kim_ui::field(ui, lang.t("field.win.op"), None, |ui| {
                let opts: Vec<(W, &str)> = [
                    (W::Activate, "field.win.activate"),
                    (W::Minimize, "field.win.minimize"),
                    (W::Maximize, "field.win.maximize"),
                    (W::Restore, "field.win.restore"),
                    (W::Move, "field.win.move"),
                    (W::Topmost, "field.win.topmost"),
                ]
                .into_iter()
                .map(|(o, k)| (o, lang.t(k)))
                .collect();
                let owned: Vec<(W, String)> =
                    opts.iter().map(|(o, l)| (*o, l.to_string())).collect();
                kim_ui::dropdown(
                    ui,
                    "window-op",
                    lang.t("field.win.op"),
                    &mut fl.window.op,
                    &owned,
                );
            });
            kim_ui::field(ui, lang.t("field.win.app"), None, |ui| {
                line(ui, &mut fl.window.app, lang.t("field.win.app"), "Code.exe");
                program_picker(ui, lang, &mut fl.window.app, false);
                program_suggestions(ui, &mut fl.window.app);
            });
            if fl.window.op == W::Move {
                pair(ui, |ui| {
                    for (v, key) in [
                        (&mut fl.window.x, "field.win.x"),
                        (&mut fl.window.y, "field.win.y"),
                        (&mut fl.window.width, "field.win.width"),
                        (&mut fl.window.height, "field.win.height"),
                    ] {
                        ui.vertical(|ui| {
                            kim_ui::field(ui, lang.t(key), None, |ui| {
                                kim_ui::text_field(ui, v, lang.t(key), "", 72.0);
                            });
                        });
                    }
                });
            }
            if fl.window.op == W::Topmost {
                kim_ui::checkbox(ui, &mut fl.window.on, lang.t("field.win.on"));
            }
            ui.label(caption(lang.t("field.win.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Clipboard => {
            use ddalgak::model::{ClipTransform as T, ClipboardOp as C};
            kim_ui::field(ui, lang.t("field.clip.op"), None, |ui| {
                kim_ui::segmented(
                    ui,
                    &mut fl.clip.op,
                    &[
                        (C::Set, lang.t("field.clip.set")),
                        (C::Clear, lang.t("field.clip.clear")),
                        (C::Transform, lang.t("field.clip.transform")),
                    ],
                );
            });
            match fl.clip.op {
                C::Set => {
                    kim_ui::field(ui, lang.t("field.clip.text"), None, |ui| {
                        kim_ui::text_area(ui, &mut fl.clip.text, lang.t("field.clip.text"), "", 2);
                    });
                }
                C::Transform => {
                    kim_ui::field(ui, lang.t("field.clip.to"), None, |ui| {
                        kim_ui::segmented(
                            ui,
                            &mut fl.clip.to,
                            &[
                                (T::Plain, lang.t("field.clip.plain")),
                                (T::Trim, lang.t("field.clip.trim")),
                                (T::Upper, lang.t("field.clip.upper")),
                                (T::Lower, lang.t("field.clip.lower")),
                            ],
                        );
                    });
                }
                C::Clear => {}
            }
            ui.label(caption(lang.t("field.clip.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Command => {
            kim_ui::field(ui, lang.t("field.cmd.program"), None, |ui| {
                line(
                    ui,
                    &mut fl.command.program,
                    lang.t("field.cmd.program"),
                    "C:\\Tools\\backup.exe",
                );
            });
            kim_ui::field(ui, lang.t("field.cmd.args"), None, |ui| {
                kim_ui::text_area(ui, &mut fl.command.args, lang.t("field.cmd.args"), "", 2);
            });
            kim_ui::field(ui, lang.t("field.cmd.folder"), None, |ui| {
                line(
                    ui,
                    &mut fl.command.working_dir,
                    lang.t("field.cmd.folder"),
                    "",
                );
            });
            kim_ui::field(ui, lang.t("field.cmd.timeout"), None, |ui| {
                number(
                    ui,
                    &mut fl.command.timeout_ms,
                    100..=600_000,
                    " ms",
                    lang.t("field.cmd.timeout"),
                );
            });
            ui.label(caption(lang.t("field.cmd.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Timer => {
            let mut opts = vec![(String::new(), "—".to_string())];
            for r in profile.map(|p| p.rules.as_slice()).unwrap_or_default() {
                if let Some(id) = r.id() {
                    opts.push((id.to_string(), r.title().unwrap_or(id).to_string()));
                }
            }
            kim_ui::field(ui, lang.t("field.timer.rule"), None, |ui| {
                kim_ui::dropdown(
                    ui,
                    "timer-rule",
                    lang.t("field.timer.rule"),
                    &mut fl.timer.rule,
                    &opts,
                );
            });
            kim_ui::field(ui, lang.t("field.timer.after"), None, |ui| {
                number(
                    ui,
                    &mut fl.timer.after_s,
                    1..=86_400,
                    " s",
                    lang.t("field.timer.after"),
                );
            });
            ui.label(caption(lang.t("field.timer.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Tool => {
            kim_ui::field(ui, lang.t("field.tool.capability"), None, |ui| {
                line(
                    ui,
                    &mut fl.tool.capability,
                    lang.t("field.tool.capability"),
                    "pin.list",
                );
                capability_finder(ui, lang, &mut fl.tool.capability);
            });
            kim_ui::field(ui, lang.t("field.tool.args"), None, |ui| {
                kim_ui::text_area(ui, &mut fl.tool.args, lang.t("field.tool.args"), "", 2);
            });
            ui.label(caption(lang.t("field.tool.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
        Effect::Notify => {
            kim_ui::field(ui, lang.t("field.notify.title"), None, |ui| {
                line(ui, &mut fl.notify.title, lang.t("field.notify.title"), "");
            });
            kim_ui::field(ui, lang.t("field.notify.body"), None, |ui| {
                kim_ui::text_area(ui, &mut fl.notify.body, lang.t("field.notify.body"), "", 2);
            });
            ui.label(caption(lang.t("field.notify.note"), c.text_muted));
            problems_for(ui, lang, problems, &["steps[0]"]);
        }
    }
    egui::CollapsingHeader::new(format!("Advanced · {}", lang.t("form.advanced_sub")))
        .id_salt("advanced")
        .show(ui, |ui| advanced_fields(ui, lang, f, profile, problems));
}

/// When the rule may run (modes, programs, window, time), its priority and description.
fn advanced_fields(
    ui: &mut egui::Ui,
    lang: Lang,
    f: &mut RuleForm,
    profile: Option<&Profile>,
    problems: &[ddalgak::text::FieldProblem],
) {
    let c = Colors::get();
    let fl = &mut f.fields;
    if let Some(p) = profile
        && p.modes.len() > 1
    {
        kim_ui::field(ui, lang.t("form.only_modes"), None, |ui| {
            ui.horizontal_wrapped(|ui| {
                for m in &p.modes {
                    let mut on = fl.modes.contains(&m.id);
                    if kim_ui::checkbox(ui, &mut on, &m.title).changed() {
                        if on {
                            fl.modes.push(m.id.clone());
                        } else {
                            fl.modes.retain(|x| x != &m.id);
                        }
                    }
                }
            });
        });
    }
    kim_ui::field(ui, lang.t("form.only_apps"), None, |ui| {
        kim_ui::text_area(ui, &mut fl.apps, lang.t("form.only_apps"), "Code.exe", 2);
        program_picker(ui, lang, &mut fl.apps, true);
    });
    kim_ui::field(ui, lang.t("form.except_apps"), None, |ui| {
        kim_ui::text_area(
            ui,
            &mut fl.except_apps,
            lang.t("form.except_apps"),
            "KeePassXC.exe",
            1,
        );
        program_picker(ui, lang, &mut fl.except_apps, true);
    });
    kim_ui::field(ui, lang.t("form.title_cond"), None, |ui| {
        line(ui, &mut fl.window_title, lang.t("form.title_cond"), "");
        kim_ui::checkbox(ui, &mut fl.title_exact, lang.t("form.title_exact"));
    });
    kim_ui::field(ui, lang.t("form.class_cond"), None, |ui| {
        line(
            ui,
            &mut fl.window_class,
            lang.t("form.class_cond"),
            "Chrome_WidgetWin_1",
        );
        kim_ui::checkbox(ui, &mut fl.class_exact, lang.t("form.title_exact"));
    });
    kim_ui::checkbox(ui, &mut fl.time_on, lang.t("form.time_cond"));
    if fl.time_on {
        kim_ui::toolbar(ui, |ui| {
            kim_ui::text_field(
                ui,
                &mut fl.time_from,
                lang.t("form.time_from"),
                "09:00",
                72.0,
            );
            ui.label("–");
            kim_ui::text_field(ui, &mut fl.time_to, lang.t("form.time_to"), "18:00", 72.0);
        });
        ui.horizontal_wrapped(|ui| {
            for (i, d) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                .iter()
                .enumerate()
            {
                kim_ui::checkbox(ui, &mut fl.time_days[i], d);
            }
        });
    }
    if !fl.apps.trim().is_empty()
        || !fl.except_apps.trim().is_empty()
        || !fl.window_title.trim().is_empty()
        || !fl.window_class.trim().is_empty()
        || fl.time_on
    {
        ui.label(caption(lang.t("form.apps_note"), c.text_muted));
    }
    pair(ui, |ui| {
        ui.vertical(|ui| {
            kim_ui::field(ui, lang.t("form.priority"), None, |ui| {
                number(
                    ui,
                    &mut fl.priority,
                    -1000..=1000,
                    "",
                    lang.t("form.priority"),
                );
            });
        });
    });
    kim_ui::field(ui, lang.t("form.description"), None, |ui| {
        line(ui, &mut fl.description, lang.t("form.description"), "");
    });
    problems_for(
        ui,
        lang,
        problems,
        &["modes", "when", "priority", "description"],
    );
}

/// The mouse action's fields: what kind (move by, move to, scroll, click) and only its values.
fn mouse_fields(
    ui: &mut egui::Ui,
    lang: Lang,
    f: &mut RuleForm,
    problems: &[ddalgak::text::FieldProblem],
    layout: &Layout,
    cursor: Point,
) {
    let c = Colors::get();
    let mut kind = form::mouse_kind(&f.fields.mouse);
    kim_ui::field(ui, lang.t("field.mouse.kind"), None, |ui| {
        if kim_ui::segmented(
            ui,
            &mut kind,
            &[
                (0, lang.t("field.mouse.move_by")),
                (1, lang.t("field.mouse.move_to")),
                (2, lang.t("field.mouse.scroll")),
                (3, lang.t("field.mouse.click")),
            ],
        ) {
            f.fields.mouse = form::mouse_default(kind);
        }
    });
    let anchors = [
        (Anchor::VirtualScreen, "The desktop".to_string()),
        (Anchor::PrimaryMonitor, "The primary monitor".to_string()),
        (Anchor::ActiveWindow, "The active window".to_string()),
    ];
    let xy = |ui: &mut egui::Ui, x: &mut i32, y: &mut i32, lx: &str, ly: &str| {
        pair(ui, |ui| {
            ui.vertical(|ui| {
                kim_ui::field(ui, lx, None, |ui| {
                    number(ui, x, -32768..=32767, " px", lx);
                });
            });
            ui.vertical(|ui| {
                kim_ui::field(ui, ly, None, |ui| {
                    number(ui, y, -32768..=32767, " px", ly);
                });
            });
        });
    };
    match &mut f.fields.mouse {
        MouseAction::MoveBy { dx, dy, grid, hold } => {
            xy(ui, dx, dy, lang.t("field.right"), lang.t("field.down"));
            let mut cells = grid.is_some();
            if kim_ui::checkbox(ui, &mut cells, lang.t("field.unit_grid")).changed() {
                *grid = cells.then_some(Grid {
                    columns: 8,
                    rows: 8,
                });
                if cells && matches!(hold, HoldChoice::Continuous { .. }) {
                    *hold = HoldChoice::Once;
                }
            }
            if let Some(g) = grid {
                pair(ui, |ui| {
                    ui.vertical(|ui| {
                        kim_ui::field(ui, lang.t("field.columns"), None, |ui| {
                            number(ui, &mut g.columns, 2..=64, "", lang.t("field.columns"));
                        });
                    });
                    ui.vertical(|ui| {
                        kim_ui::field(ui, lang.t("field.rows"), None, |ui| {
                            number(ui, &mut g.rows, 2..=64, "", lang.t("field.rows"));
                        });
                    });
                });
            }
            let mut h = match hold {
                HoldChoice::Once => 0,
                HoldChoice::Continuous { .. } => 1,
                HoldChoice::Repeat { .. } => 2,
            };
            let mut opts = vec![(0, lang.t("field.hold.once"))];
            if grid.is_none() {
                opts.push((1, lang.t("field.hold.glide")));
            }
            opts.push((2, lang.t("field.hold.repeat")));
            kim_ui::field(ui, lang.t("field.while_held"), None, |ui| {
                if kim_ui::segmented(ui, &mut h, &opts) {
                    *hold = match h {
                        1 => HoldChoice::Continuous {
                            speed: 600,
                            acceleration: 0,
                        },
                        2 => HoldChoice::Repeat {
                            repeat_ms: 100,
                            delay_ms: mouse::DEFAULT_DELAY_MS,
                        },
                        _ => HoldChoice::Once,
                    };
                }
            });
            match hold {
                HoldChoice::Continuous { speed, .. } => {
                    kim_ui::field(ui, lang.t("field.speed"), None, |ui| {
                        number(ui, speed, 50..=5000, " px/s", lang.t("field.speed"));
                    });
                }
                HoldChoice::Repeat { repeat_ms, .. } => {
                    kim_ui::field(ui, lang.t("field.repeat_ms"), None, |ui| {
                        number(ui, repeat_ms, 20..=2000, " ms", lang.t("field.repeat_ms"));
                    });
                }
                HoldChoice::Once => {}
            }
        }
        MouseAction::MoveTo(p) => {
            xy(ui, &mut p.x, &mut p.y, lang.t("field.x"), lang.t("field.y"));
            kim_ui::field(ui, lang.t("field.relative_to"), None, |ui| {
                kim_ui::dropdown(
                    ui,
                    "move-to-anchor",
                    lang.t("field.relative_to"),
                    &mut p.anchor,
                    &anchors,
                );
            });
        }
        MouseAction::Scroll { dir, notches } => {
            kim_ui::field(ui, lang.t("field.direction"), None, |ui| {
                kim_ui::segmented(
                    ui,
                    dir,
                    &[
                        (ScrollDir::Up, "Up"),
                        (ScrollDir::Down, "Down"),
                        (ScrollDir::Left, "Left"),
                        (ScrollDir::Right, "Right"),
                    ],
                );
            });
            kim_ui::field(ui, lang.t("field.notches"), None, |ui| {
                number(ui, notches, 1..=50, "", lang.t("field.notches"));
            });
        }
        MouseAction::Click { button, op, at } => {
            let buttons: Vec<(_, &str)> = [
                ("left", "Left"),
                ("right", "Right"),
                ("middle", "Middle"),
                ("x1", "Back"),
                ("x2", "Forward"),
            ]
            .into_iter()
            .filter_map(|(b, l)| mouse::button(b).map(|bt| (bt, l)))
            .collect();
            kim_ui::field(ui, lang.t("field.button"), None, |ui| {
                kim_ui::segmented(ui, button, &buttons);
            });
            let mut o = match *op {
                "double_click" => 1,
                "hold" => 2,
                _ => 0,
            };
            kim_ui::field(ui, lang.t("field.click_op"), None, |ui| {
                if kim_ui::segmented(
                    ui,
                    &mut o,
                    &[(0, "Click"), (1, "Double-click"), (2, "Hold")],
                ) {
                    *op = ["click", "double_click", "hold"][o];
                }
            });
            let mut has_at = at.is_some();
            if kim_ui::checkbox(ui, &mut has_at, lang.t("field.at_point")).changed() {
                *at = has_at.then_some(mouse::PointSpec {
                    x: 0,
                    y: 0,
                    anchor: Anchor::PrimaryMonitor,
                });
            }
            if let Some(p) = at {
                xy(ui, &mut p.x, &mut p.y, lang.t("field.x"), lang.t("field.y"));
            }
        }
    }
    let map = f.mouse_draft().screen_map(layout, cursor, None, 220, 110);
    let to = map.to.map(|_| {
        let sim = f.simulate(None, layout, cursor);
        sim.pointer.map_or(cursor, |(_, t)| t)
    });
    kim_ui::field(ui, lang.t("form.screen_map"), None, |ui| {
        screen_map(ui, layout, cursor, to);
    });
    if map.clamped {
        ui.label(caption(lang.t("form.clamped"), c.warning));
    }
    problems_for(ui, lang, problems, &["steps[0]"]);
}
