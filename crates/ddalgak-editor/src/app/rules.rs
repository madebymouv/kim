//! ADR 0100: the Rules page — the main window's one job. A title, one toolbar (search that
//! fills the row, Filters in a popover, the primary New Rule), the bulk bar while several rules
//! are selected, and the list: each rule one row of `size::LIST_ROW_DETAIL` with its name and
//! keys over a one-line summary, its badges, its on/off switch and a `⋯` menu (Edit, Duplicate,
//! Enable/Disable, Delete). A click opens the rule; Ctrl/Shift+click selects.

use super::*;
use crate::form::Effect;
use kim_design::{font, size, space};
use kim_ui::Icon;

/// The drag-and-drop payload of a rule row (reordering within its profile).
#[derive(Clone, Debug)]
struct DragRule {
    id: String,
    profile: String,
}

fn caption(text: impl Into<String>, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(font::CAPTION)
        .color(color)
}

fn badge_color(k: BadgeKind) -> egui::Color32 {
    let c = Colors::get();
    match k {
        BadgeKind::Conflict | BadgeKind::Problem => c.danger,
        BadgeKind::Warning | BadgeKind::Inactive => c.warning,
        BadgeKind::Review => c.info,
    }
}

impl EditorApp {
    /// How many filters narrow the list now (the search is not counted).
    pub fn active_filters(&self) -> usize {
        let f = &self.filter;
        usize::from(f.state != StateFilter::default())
            + usize::from(f.profile.is_some())
            + usize::from(f.mode.is_some())
            + usize::from(f.app.is_some())
            + usize::from(f.snippet_text)
            + usize::from(self.grouping != Grouping::None)
    }

    pub(super) fn rules_page(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        // The title row: "Rules" and how many there are.
        ui.horizontal(|ui| {
            kim_ui::title(ui, lang.t("nav.rules"));
            if !self.rows.is_empty() {
                let shown = self.visible.len();
                let n = self.rows.len();
                let text = if shown == n {
                    format!("{n}")
                } else {
                    format!("{shown} / {n}")
                };
                ui.label(caption(text, c.text_muted));
            }
        });
        ui.add_space(space::M);
        self.rules_toolbar(ui);
        ui.add_space(space::M);
        self.banner_ui(ui);
        self.bulk_bar(ui);
        if self.rows.is_empty() {
            self.empty_state(ui);
            return;
        }
        if self.visible.is_empty() {
            ui.add_space(space::XL);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(lang.t("list.no_match")).color(c.text_muted));
                ui.add_space(space::S);
                if ui
                    .add(kim_ui::Button::ghost("Clear search and filters"))
                    .clicked()
                {
                    self.filter = Filter::default();
                    self.grouping = Grouping::None;
                }
            });
            return;
        }
        self.rule_list(ui);
    }

    /// Search (fills the row), Filters, New Rule: one toolbar, one control height, one gap.
    fn rules_toolbar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let n = self.active_filters();
        let filters_label = "Filters";
        let fw = {
            let g = ui.painter().layout_no_wrap(
                filters_label.into(),
                kim_ui::ui_font(ui.ctx(), font::BODY, false),
                egui::Color32::PLACEHOLDER,
            );
            2.0 * size::BUTTON_PAD_X + size::ICON_M + space::XS + g.size().x
        };
        let nw = {
            let g = ui.painter().layout_no_wrap(
                lang.t("list.new").into(),
                kim_ui::ui_font(ui.ctx(), font::BODY, true),
                egui::Color32::PLACEHOLDER,
            );
            (g.size().x + 2.0 * size::BUTTON_PAD_X).max(size::BUTTON_MIN_WIDTH)
        };
        kim_ui::inspect::scope("rules.toolbar", || {
            kim_ui::toolbar(ui, |ui| {
                let w = kim_ui::remaining_width(ui, fw + nw + 2.0 * space::S);
                let search = kim_ui::search_field(
                    ui,
                    &mut self.filter.query,
                    lang.t("list.search_label"),
                    lang.t("list.search"),
                    w,
                );
                self.anchors[1] = Some(search.id);
                if std::mem::take(&mut self.focus_search) {
                    search.request_focus();
                }
                let fb = kim_ui::icon_text_button(ui, Icon::Filter, filters_label, n > 0)
                    .on_hover_text(lang.t("list.filters_hint"));
                if n > 0 {
                    kim_ui::count_badge(ui, fb.rect, n);
                }
                if self.audit_filters {
                    // A UI audit photographs the open popover.
                    egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&fb));
                }
                kim_ui::popover(&fb, size::POPOVER_WIDTH, |ui| {
                    kim_ui::inspect::scope("rules.filters", || self.filter_popover(ui));
                });
                if ui
                    .add(kim_ui::Button::primary(lang.t("list.new")))
                    .on_hover_text(self.shortcuts.keys_of("rules.new").unwrap_or_default())
                    .clicked()
                {
                    self.acts.push(Act::New(None));
                }
            });
        });
    }

    /// The filters (state, profile, mode, program, grouping, snippet search) in the popover.
    fn filter_popover(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let any = lang.t("filter.any").to_string();
        let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.horizontal(|ui| {
                ui.label(caption(label, Colors::get().text_secondary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    add(ui);
                });
            });
        };
        let states: Vec<(StateFilter, String)> = StateFilter::ALL
            .iter()
            .map(|s| (*s, lang.t(s.key()).to_string()))
            .collect();
        row(ui, lang.t("filter.state"), &mut |ui| {
            kim_ui::dropdown(
                ui,
                "filter-state",
                lang.t("filter.state"),
                &mut self.filter.state,
                &states,
            );
        });
        let (modes, apps) = self.doc().map(listing::filter_choices).unwrap_or_default();
        let profiles: Vec<(Option<String>, String)> = std::iter::once((None, any.clone()))
            .chain(
                self.doc()
                    .map(|d| {
                        d.profiles
                            .iter()
                            .map(|p| (Some(p.id.clone()), p.title.clone()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
            .collect();
        if profiles.len() > 2 {
            row(ui, lang.t("filter.profile"), &mut |ui| {
                kim_ui::dropdown(
                    ui,
                    "filter-profile",
                    lang.t("filter.profile"),
                    &mut self.filter.profile,
                    &profiles,
                );
            });
        }
        if modes.len() > 1 {
            let opts: Vec<(Option<String>, String)> = std::iter::once((None, any.clone()))
                .chain(modes.iter().map(|(id, t)| (Some(id.clone()), t.clone())))
                .collect();
            row(ui, lang.t("filter.mode"), &mut |ui| {
                kim_ui::dropdown(
                    ui,
                    "filter-mode",
                    lang.t("filter.mode"),
                    &mut self.filter.mode,
                    &opts,
                );
            });
        }
        if !apps.is_empty() {
            let opts: Vec<(Option<String>, String)> = std::iter::once((None, any))
                .chain(apps.iter().map(|a| (Some(a.clone()), a.clone())))
                .collect();
            row(ui, lang.t("filter.app"), &mut |ui| {
                kim_ui::dropdown(
                    ui,
                    "filter-app",
                    lang.t("filter.app"),
                    &mut self.filter.app,
                    &opts,
                );
            });
        }
        let groups: Vec<(Grouping, String)> = Grouping::ALL
            .iter()
            .map(|g| (*g, lang.t(g.key()).to_string()))
            .collect();
        row(ui, lang.t("group.label"), &mut |ui| {
            kim_ui::dropdown(
                ui,
                "grouping",
                lang.t("group.label"),
                &mut self.grouping,
                &groups,
            );
        });
        kim_ui::checkbox(
            ui,
            &mut self.filter.snippet_text,
            lang.t("list.snippet_search"),
        );
        ui.add_space(space::XS);
        kim_ui::row_divider(ui);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.active_filters() > 0,
                    kim_ui::Button::ghost("Reset filters").compact(),
                )
                .clicked()
            {
                let q = std::mem::take(&mut self.filter.query);
                self.filter = Filter {
                    query: q,
                    ..Filter::default()
                };
                self.grouping = Grouping::None;
            }
        });
    }

    /// While several rules are selected: what can be done to all of them.
    fn bulk_bar(&mut self, ui: &mut egui::Ui) {
        if self.selected.len() < 2 {
            return;
        }
        let c = Colors::get();
        let lang = self.lang;
        let sel = self.selected.clone();
        egui::Frame::new()
            .fill(c.accent_soft)
            .corner_radius(egui::CornerRadius::same(kim_design::radius::M as u8))
            .inner_margin(egui::Margin::symmetric(space::M as i8, space::XS as i8))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{} {}", sel.len(), lang.t("bulk.selected")));
                    for (key, act) in [
                        ("btn.enable", Act::SetEnabled(sel.clone(), true)),
                        ("btn.disable", Act::SetEnabled(sel.clone(), false)),
                        ("btn.duplicate", Act::Duplicate(sel.clone())),
                        ("btn.delete", Act::AskDelete(sel.clone())),
                        ("btn.export", Act::Export(sel.clone())),
                    ] {
                        if ui
                            .add(kim_ui::Button::ghost(lang.t(key)).compact())
                            .clicked()
                        {
                            self.acts.push(act);
                        }
                    }
                    if ui
                        .add(kim_ui::Button::ghost(lang.t("bulk.clear")).compact())
                        .clicked()
                    {
                        self.selected.clear();
                    }
                });
            });
        ui.add_space(space::S);
    }

    fn rule_list(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        let shown: Vec<usize> = (0..self.rows.len())
            .filter(|i| self.filter.matches(&self.rows[*i]))
            .collect();
        let groups = listing::group(
            &self.rows,
            &shown,
            self.grouping,
            lang.t("group.every_mode"),
            lang.t("group.every_app"),
        );
        let row_h = size::LIST_ROW_DETAIL + 2.0;
        // The list leaves room for the scroll bar on its right, never under a row's menu.
        let width = ui.available_width();
        if self.grouping == Grouping::None {
            let rows = groups
                .into_iter()
                .next()
                .map(|g| g.rows)
                .unwrap_or_default();
            egui::ScrollArea::vertical()
                .id_salt("rules")
                .auto_shrink([false, false])
                .show_rows(ui, row_h, rows.len(), |ui, range| {
                    ui.set_width(width - space::M);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for i in &rows[range] {
                        self.rule_row(ui, *i);
                    }
                });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("rules")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(width - space::M);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (gi, g) in groups.iter().enumerate() {
                        if gi > 0 {
                            ui.add_space(space::M);
                        }
                        ui.label(
                            egui::RichText::new(&g.heading)
                                .font(kim_ui::ui_font(ui.ctx(), font::CAPTION, true))
                                .color(c.text_secondary),
                        );
                        for i in &g.rows {
                            self.rule_row(ui, *i);
                        }
                    }
                });
        }
    }

    pub(super) fn row_state(&self, r: &Row) -> (String, egui::Color32) {
        let c = Colors::get();
        let lang = self.lang;
        let active_now = self.save.is_active(self.loaded.revision());
        match r.state {
            "active" if active_now => (format!("● {}", lang.t("state.active")), c.success),
            "active" => (format!("○ {}", lang.t("state.ready")), c.text_muted),
            "disabled" => (lang.t("state.disabled").to_string(), c.text_muted),
            "other_mode" => (lang.t("state.other_mode").to_string(), c.text_muted),
            "other_profile" => (lang.t("state.other_profile").to_string(), c.text_muted),
            _ => (
                format!(
                    "{}: {}",
                    lang.t("badge.inactive"),
                    r.code.unwrap_or("rule_invalid")
                ),
                c.warning,
            ),
        }
    }

    /// One rule: `[⋮] Name [keys]` over `what it does`, then badges, the switch and `⋯`.
    fn rule_row(&mut self, ui: &mut egui::Ui, i: usize) {
        let c = Colors::get();
        let lang = self.lang;
        let r = self.rows[i].clone();
        let selected = self.selected.contains(&r.id);
        let at_cursor = self.cursor_row.as_deref() == Some(r.id.as_str());
        let trigger = self.trigger_display(&r);
        let id = egui::Id::unique(("rule-row", &r.id));
        let mut menu_open = false;
        let (resp, ()) = kim_ui::list_row(ui, id, selected, size::LIST_ROW_DETAIL, |ui| {
            ui.spacing_mut().item_spacing.x = space::S;
            // The drag handle (reorder within the profile; Alt+Up/Down from the keyboard).
            let (hr, handle) =
                ui.allocate_exact_size(egui::vec2(12.0, size::HIT_MIN), egui::Sense::drag());
            let hc = if handle.hovered() {
                c.text_secondary
            } else {
                c.text_muted.gamma_multiply(0.7)
            };
            for dy in [-4.0, 0.0, 4.0] {
                for dx in [-2.0, 2.0] {
                    ui.painter()
                        .circle_filled(hr.center() + egui::vec2(dx, dy), 1.1, hc);
                }
            }
            let handle = handle
                .on_hover_text(lang.t("list.drag"))
                .on_hover_cursor(egui::CursorIcon::Grab);
            let dname = format!("{}: {}", lang.t("list.drag"), r.title);
            handle.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &dname));
            handle.dnd_set_drag_payload(DragRule {
                id: r.id.clone(),
                profile: r.profile.clone(),
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = space::S;
                let more = kim_ui::icon_button(
                    ui,
                    Icon::More,
                    &format!("{}: {}", lang.t("list.more"), r.title),
                );
                menu_open =
                    egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&more));
                egui::Popup::menu(&more)
                    .frame(kim_ui::popover_frame(ui.ctx()))
                    .width(160.0)
                    .show(|ui| {
                        ui.set_min_width(160.0);
                        if r.editable && kim_ui::menu_item(ui, lang.t("btn.edit"), false).clicked()
                        {
                            self.acts.push(Act::Edit(r.id.clone()));
                        }
                        if kim_ui::menu_item(ui, lang.t("btn.duplicate"), false).clicked() {
                            self.acts.push(Act::Duplicate(vec![r.id.clone()]));
                        }
                        let t = if r.enabled {
                            "btn.disable"
                        } else {
                            "btn.enable"
                        };
                        if kim_ui::menu_item(ui, lang.t(t), false).clicked() {
                            self.acts
                                .push(Act::SetEnabled(vec![r.id.clone()], !r.enabled));
                        }
                        if kim_ui::menu_item(ui, "Test in Playground", false).clicked() {
                            self.acts.push(Act::Test(r.id.clone()));
                        }
                        kim_ui::row_divider(ui);
                        if kim_ui::menu_item(ui, lang.t("btn.delete"), true).clicked() {
                            self.acts.push(Act::AskDelete(vec![r.id.clone()]));
                        }
                    });
                let mut on = r.enabled;
                if ui
                    .add(kim_ui::Switch::new(
                        &mut on,
                        format!("{}: {}", lang.t("filter.enabled"), r.title),
                    ))
                    .changed()
                {
                    self.acts.push(Act::SetEnabled(vec![r.id.clone()], on));
                }
                for b in r.badges.iter().rev() {
                    ui.label(caption(
                        format!("{} {}", b.kind.glyph(), lang.t(b.kind.key())),
                        badge_color(b.kind),
                    ))
                    .on_hover_text(format!("{} ({})", b.message, b.code));
                }
                // A "hold + press" rule lives in its layer: not "in another mode".
                let layered = trigger != r.trigger && r.state == "other_mode";
                if !layered && !r.state.starts_with("active") && r.state != "disabled" {
                    let (state, color) = self.row_state(&r);
                    ui.label(caption(state, color));
                }
                // The text block takes what is left, clipped there: keys that do not fit in a
                // narrow window are cut, never drawn under the switch.
                let room = ui.available_rect_before_wrap();
                ui.set_clip_rect(ui.clip_rect().intersect(room));
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = space::S;
                            let name = egui::RichText::new(&r.title)
                                .font(kim_ui::ui_font(ui.ctx(), font::BODY, true))
                                .color(if r.enabled {
                                    c.text_primary
                                } else {
                                    c.text_secondary
                                });
                            ui.add(egui::Label::new(name).truncate().selectable(false));
                            if !trigger.is_empty() {
                                kim_ui::keycaps(ui, &trigger);
                            }
                        });
                        // ADR 0101: the rule in the window's language.
                        let said = if self.lang == Lang::Ko && !r.summary_ko.is_empty() {
                            &r.summary_ko
                        } else {
                            &r.summary
                        };
                        ui.add(
                            egui::Label::new(caption(said, c.text_muted))
                                .truncate()
                                .selectable(false),
                        );
                    });
                });
            });
        });
        let label = format!("{}  →  {}", r.trigger, r.summary);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        if at_cursor {
            ui.painter().rect_stroke(
                resp.rect,
                egui::CornerRadius::same(kim_design::radius::M as u8),
                egui::Stroke::new(size::FOCUS_RING, c.focus),
                egui::StrokeKind::Inside,
            );
        }
        if resp.clicked() && !menu_open {
            let m = ui.input(|i| i.modifiers);
            if (m.ctrl || m.shift) || !r.editable {
                self.acts.push(Act::Select {
                    id: r.id.clone(),
                    ctrl: m.ctrl,
                    shift: m.shift,
                });
            } else {
                self.acts.push(Act::Edit(r.id.clone()));
            }
        }
        let rect = resp.rect;
        if let Some(p) = resp.dnd_hover_payload::<DragRule>()
            && p.profile == r.profile
            && p.id != r.id
        {
            let y = ui
                .input(|i| i.pointer.hover_pos())
                .map_or(rect.center().y, |p| p.y);
            let line_y = if y < rect.center().y {
                rect.top()
            } else {
                rect.bottom()
            };
            ui.painter()
                .hline(rect.x_range(), line_y, egui::Stroke::new(2.0, c.accent));
        }
        if let Some(p) = resp.dnd_release_payload::<DragRule>()
            && p.profile == r.profile
            && p.id != r.id
        {
            let y = ui
                .input(|i| i.pointer.interact_pos())
                .map_or(rect.center().y, |p| p.y);
            let pos = if y < rect.center().y {
                Position::Before(r.id.clone())
            } else {
                Position::After(r.id.clone())
            };
            self.acts.push(Act::Move(p.id.clone(), pos));
        }
    }

    /// No rules yet: a centred title, one sentence, the starters as quiet buttons (New Rule,
    /// the primary action, stays in the toolbar) and the emergency stop as a caption.
    fn empty_state(&mut self, ui: &mut egui::Ui) {
        let c = Colors::get();
        let lang = self.lang;
        ui.vertical_centered(|ui| {
            ui.add_space(space::XXL * 2.0);
            ui.label(
                egui::RichText::new(lang.t("list.empty_title"))
                    .font(kim_ui::ui_font(ui.ctx(), font::TITLE, true))
                    .color(c.text_primary),
            );
            ui.add_space(space::XS);
            ui.label(egui::RichText::new(lang.t("list.empty_body")).color(c.text_muted));
            ui.add_space(space::L);
            let starters = [
                ("list.starter_remap", Effect::Remap),
                ("list.starter_text", Effect::TypeText),
                ("list.starter_program", Effect::OpenProgram),
            ];
            let gap = space::S;
            let widths: Vec<f32> = starters
                .iter()
                .map(|(k, _)| {
                    let w = ui
                        .painter()
                        .layout_no_wrap(
                            lang.t(k).to_string(),
                            kim_ui::ui_font(ui.ctx(), font::BODY, false),
                            egui::Color32::PLACEHOLDER,
                        )
                        .size()
                        .x;
                    (w + 2.0 * size::BUTTON_PAD_X).max(size::BUTTON_MIN_WIDTH)
                })
                .collect();
            let total = widths.iter().sum::<f32>() + gap * (widths.len() - 1) as f32;
            let left = (ui.available_width() - total).max(0.0) / 2.0;
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.spacing_mut().item_spacing.x = gap;
                for (key, e) in starters {
                    if ui.add(kim_ui::Button::secondary(lang.t(key))).clicked() {
                        self.acts.push(Act::New(Some(e)));
                    }
                }
            });
            ui.add_space(space::XL);
            ui.label(caption(lang.t("header.emergency"), c.text_muted));
        });
    }
}
