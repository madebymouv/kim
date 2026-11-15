//! ADR 0100: the form, toolbar and menu components every egui window shares (DESIGN_SYSTEM
//! §3.10–§3.14): the search field, icon buttons, checkboxes, number fields, text areas, choice
//! cards, labelled fields, form sections, popovers with menu items, count badges and the status
//! indicator.
//!
//! Like [`crate::widgets`], each one takes its sizes from [`kim_design`] tokens (a control is
//! `size::CONTROL` high, whatever it is), shows the 2 px focus ring, fades when disabled, names
//! itself for screen readers and records its geometry for the UI audit, so a window built from
//! them cannot drift into one-off sizes.

use crate::inspect::{self, Painted};
use crate::widgets::focus_ring;
use crate::{Colors, Tone, ui_font};
use egui::{
    Align, Align2, Color32, CornerRadius, Layout, Margin, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, pos2, vec2,
};
use kim_design::{font, radius, size, space};

/// A search field that fills `width`: a quiet magnifier on the left, the hint, and a clear (×)
/// button once something is typed. Escape inside it clears it too. `label` is its accessible
/// name.
pub fn search_field(
    ui: &mut Ui,
    text: &mut String,
    label: &str,
    hint: &str,
    width: f32,
) -> Response {
    let c = Colors::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, size::CONTROL), Sense::hover());
    // The field's surface goes under the text (painted once the focus is known).
    let surface = ui.painter().add(egui::Shape::Noop);
    let icon_w = size::ICON_M + space::S;
    let clear_w = if text.is_empty() { 0.0 } else { size::HIT_MIN };
    let edit_rect = Rect::from_min_max(
        pos2(rect.left() + space::S + icon_w - space::XS, rect.top()),
        pos2(rect.right() - clear_w - space::XS, rect.bottom()),
    );
    let edit = egui::TextEdit::singleline(text)
        .hint_text(hint)
        .frame(egui::Frame::NONE)
        .margin(Margin::symmetric(0, space::XS as i8))
        .align(Align2::LEFT_CENTER)
        .desired_width(edit_rect.width());
    // A child of the field's own rectangle: placing the edit never moves the parent's cursor
    // (the next toolbar control started under the field's right end, measured on the desktop).
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(edit_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let r = child.add(edit);
    ui.ctx()
        .accesskit_node_builder(r.id, |b| b.set_label(label.to_string()));
    let focused = r.has_focus();
    // Esc clears the search (egui's edit gives up the focus on the same key press).
    if (focused || r.lost_focus())
        && ui.input(|i| i.key_pressed(egui::Key::Escape))
        && !text.is_empty()
    {
        text.clear();
    }
    let p = ui.painter();
    p.set(
        surface,
        egui::epaint::RectShape::new(
            rect,
            CornerRadius::same(radius::S as u8),
            c.bg_overlay,
            Stroke::new(
                size::BORDER,
                if focused {
                    c.border_strong
                } else {
                    c.border_subtle
                },
            ),
            StrokeKind::Inside,
        ),
    );
    // The magnifier: a ring and a handle, drawn so it never depends on an icon font.
    let mc = pos2(rect.left() + space::S + 6.0, rect.center().y - 1.0);
    p.circle_stroke(mc, 4.5, Stroke::new(1.4, c.text_muted));
    p.line_segment(
        [mc + vec2(3.3, 3.3), mc + vec2(6.5, 6.5)],
        Stroke::new(1.6, c.text_muted),
    );
    focus_ring(ui, &r, rect, radius::S);
    let mut out = r.clone();
    if !text.is_empty() {
        let cr = Rect::from_center_size(
            pos2(rect.right() - size::HIT_MIN / 2.0 - 2.0, rect.center().y),
            Vec2::splat(size::HIT_MIN),
        );
        let x = ui.interact(cr, r.id.with("clear"), Sense::click());
        x.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, "Clear search"));
        let col = if x.hovered() {
            c.text_primary
        } else {
            c.text_muted
        };
        if x.hovered() {
            ui.painter().rect_filled(
                cr.shrink(3.0),
                CornerRadius::same(radius::S as u8),
                c.bg_hover,
            );
        }
        let k = 3.5;
        let cc = cr.center();
        ui.painter()
            .line_segment([cc - vec2(k, k), cc + vec2(k, k)], Stroke::new(1.4, col));
        ui.painter()
            .line_segment([cc + vec2(-k, k), cc + vec2(k, -k)], Stroke::new(1.4, col));
        if x.clicked() {
            text.clear();
            out.mark_changed();
            r.request_focus();
        }
    }
    inspect::record(
        ui,
        &format!("search:{label}"),
        "text-input",
        rect,
        None,
        true,
        if focused { &["focused"] } else { &[] },
        None,
        Some(c.bg_overlay),
    );
    out
}

/// The vector icons of the shared components (drawn, so they never depend on an icon font).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// `⋯`: more actions (a row's menu).
    More,
    /// A small keyboard: pick keys by clicking.
    Keyboard,
    /// A dot: record keys (filled red while recording).
    Record,
    /// `×`.
    Close,
    /// `+`.
    Plus,
    /// A funnel: filters.
    Filter,
}

/// Paints `icon` centred in `rect` (an icon box of `size::ICON_M`).
pub fn paint_icon(p: &egui::Painter, rect: Rect, icon: Icon, col: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.4, col);
    match icon {
        Icon::More => {
            for dx in [-5.0, 0.0, 5.0] {
                p.circle_filled(c + vec2(dx, 0.0), 1.6, col);
            }
        }
        Icon::Keyboard => {
            let r = Rect::from_center_size(c, vec2(14.0, 9.0));
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Inside);
            for row in 0..2 {
                for k in 0..4 {
                    let x = r.left() + 3.0 + k as f32 * 2.7;
                    let y = r.top() + 2.8 + row as f32 * 2.2;
                    p.rect_filled(
                        Rect::from_min_size(pos2(x, y), vec2(1.3, 1.1)),
                        CornerRadius::ZERO,
                        col,
                    );
                }
            }
            p.line_segment(
                [
                    pos2(r.left() + 4.0, r.bottom() - 2.4),
                    pos2(r.right() - 4.0, r.bottom() - 2.4),
                ],
                s,
            );
        }
        Icon::Record => {
            p.circle_stroke(c, 6.0, s);
            p.circle_filled(c, 3.5, col);
        }
        Icon::Close => {
            let k = 4.0;
            p.line_segment([c - vec2(k, k), c + vec2(k, k)], s);
            p.line_segment([c + vec2(-k, k), c + vec2(k, -k)], s);
        }
        Icon::Plus => {
            let k = 5.0;
            p.line_segment([c - vec2(k, 0.0), c + vec2(k, 0.0)], s);
            p.line_segment([c - vec2(0.0, k), c + vec2(0.0, k)], s);
        }
        Icon::Filter => {
            let pts = vec![
                c + vec2(-6.0, -4.5),
                c + vec2(6.0, -4.5),
                c + vec2(1.5, 0.8),
                c + vec2(1.5, 5.0),
                c + vec2(-1.5, 3.5),
                c + vec2(-1.5, 0.8),
            ];
            p.add(egui::Shape::closed_line(pts, s));
        }
    }
}

/// A square ghost button with one icon: the control height on both sides, `label` as its
/// tooltip and accessible name.
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    icon_button_sized(ui, icon, label, size::CONTROL, false)
}

/// [`icon_button`] at `side`, optionally shown pressed (`on`: recording, a menu that is open).
pub fn icon_button_sized(ui: &mut Ui, icon: Icon, label: &str, side: f32, on: bool) -> Response {
    let c = Colors::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let l = label.to_string();
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &l));
    let hot = resp.hovered() && resp.enabled();
    let fill = if resp.is_pointer_button_down_on() || on {
        c.bg_selected
    } else if hot {
        c.bg_hover
    } else {
        Color32::TRANSPARENT
    };
    let fg = if icon == Icon::Record && on {
        c.danger
    } else if hot || on {
        c.text_primary
    } else {
        c.text_secondary
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius::S as u8), fill);
    let fg_drawn = if ui.is_enabled() {
        fg
    } else {
        fg.gamma_multiply(kim_design::state::DISABLED_OPACITY)
    };
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(size::ICON_M)),
        icon,
        fg_drawn,
    );
    focus_ring(ui, &resp, rect, radius::S);
    inspect::record(
        ui,
        &format!("icon-button:{label}"),
        "icon-button",
        rect,
        None,
        true,
        if on { &["selected"] } else { &[] },
        Some(fg),
        (fill.a() == 255).then_some(fill),
    );
    let resp = resp.on_hover_text(label);
    if resp.hovered() && resp.enabled() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A secondary toolbar button with an icon before its text (Filters), at the control height.
pub fn icon_text_button(ui: &mut Ui, icon: Icon, text: &str, on: bool) -> Response {
    let c = Colors::get(ui.ctx());
    let g = ui.painter().layout_no_wrap(
        text.to_string(),
        ui_font(ui.ctx(), font::BODY, false),
        c.text_primary,
    );
    let w = size::BUTTON_PAD_X + size::ICON_M + space::XS + g.size().x + size::BUTTON_PAD_X;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, size::CONTROL), Sense::click());
    let t = text.to_string();
    resp.widget_info(|| WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), on, &t));
    let hot = resp.hovered() && resp.enabled();
    let fill = if resp.is_pointer_button_down_on() || on {
        c.bg_selected
    } else if hot {
        c.bg_hover
    } else {
        c.bg_overlay
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(radius::S as u8),
        fill,
        Stroke::new(size::BORDER, c.border_subtle),
        StrokeKind::Inside,
    );
    let ir = Rect::from_min_size(
        pos2(
            rect.left() + size::BUTTON_PAD_X,
            rect.center().y - size::ICON_M / 2.0,
        ),
        Vec2::splat(size::ICON_M),
    );
    paint_icon(p, ir, icon, c.text_secondary);
    let tr = Rect::from_min_size(
        pos2(ir.right() + space::XS, rect.center().y - g.size().y / 2.0),
        g.size(),
    );
    p.galley(tr.min, g, c.text_primary);
    focus_ring(ui, &resp, rect, radius::S);
    inspect::record(
        ui,
        &format!("button:{text}"),
        "button",
        rect,
        Some(Painted {
            content: text,
            rect: tr,
            size: font::BODY,
            truncated: false,
        }),
        true,
        if on {
            &["secondary", "selected"]
        } else {
            &["secondary"]
        },
        Some(c.text_primary),
        Some(fill),
    );
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A checkbox with its label, one control high (the whole row is the hit target).
pub fn checkbox(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let c = Colors::get(ui.ctx());
    let g = ui.painter().layout_no_wrap(
        label.to_string(),
        ui_font(ui.ctx(), font::BODY, false),
        c.text_primary,
    );
    let w = size::CHECKBOX + space::S + g.size().x;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(w, size::CONTROL), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let v = *on;
    let l = label.to_string();
    resp.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), v, &l));
    let bx = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - size::CHECKBOX / 2.0),
        Vec2::splat(size::CHECKBOX),
    );
    let hot = resp.hovered() && resp.enabled();
    let p = ui.painter();
    if v {
        p.rect_filled(bx, CornerRadius::same(radius::S as u8), c.accent);
        let s = Stroke::new(1.8, c.text_on_accent);
        p.line_segment(
            [
                bx.left_center() + vec2(3.5, 0.5),
                bx.center() + vec2(-1.0, 4.0),
            ],
            s,
        );
        p.line_segment(
            [
                bx.center() + vec2(-1.0, 4.0),
                bx.right_top() + vec2(-3.5, 4.0),
            ],
            s,
        );
    } else {
        p.rect(
            bx,
            CornerRadius::same(radius::S as u8),
            if hot { c.bg_hover } else { c.bg_overlay },
            Stroke::new(size::BORDER, c.border_strong),
            StrokeKind::Inside,
        );
    }
    let tr = Rect::from_min_size(
        pos2(bx.right() + space::S, rect.center().y - g.size().y / 2.0),
        g.size(),
    );
    p.galley(tr.min, g, c.text_primary);
    focus_ring(ui, &resp, bx, radius::S);
    inspect::record(
        ui,
        &format!("checkbox:{label}"),
        "checkbox",
        rect,
        Some(Painted {
            content: label,
            rect: tr,
            size: font::BODY,
            truncated: false,
        }),
        true,
        &[if v { "on" } else { "off" }],
        Some(c.text_primary),
        None,
    );
    resp
}

/// A number field (drag or type) at the control height, with a unit suffix (` ms`).
pub fn number_field<N: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    suffix: &str,
    label: &str,
    width: f32,
) -> Response {
    let c = Colors::get(ui.ctx());
    let r = ui.add_sized(
        vec2(width, size::CONTROL),
        egui::DragValue::new(value)
            .range(range)
            .suffix(suffix.to_string()),
    );
    let l = label.to_string();
    ui.ctx()
        .accesskit_node_builder(r.id, |b| b.set_label(l.clone()));
    inspect::record(
        ui,
        &format!("number:{label}"),
        "number-input",
        r.rect,
        None,
        true,
        &[],
        Some(c.text_primary),
        None,
    );
    r
}

/// A multi-line text field of `rows` lines, the full available width.
pub fn text_area(ui: &mut Ui, text: &mut String, label: &str, hint: &str, rows: usize) -> Response {
    let r = ui.add(
        egui::TextEdit::multiline(text)
            .hint_text(hint)
            .desired_rows(rows)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(space::S as i8, space::XS as i8 + 2)),
    );
    let l = label.to_string();
    ui.ctx()
        .accesskit_node_builder(r.id, |b| b.set_label(l.clone()));
    inspect::record(
        ui,
        &format!("textarea:{label}"),
        "text-area",
        r.rect,
        None,
        true,
        &[],
        None,
        None,
    );
    r
}

/// A choice card (an action type in a grid): the icon and title on the first line, a muted
/// example on the second; selected cards get the accent outline and soft fill. Always
/// `size::CHOICE_CARD_HEIGHT` high and `width` wide, so a grid of them lines up.
pub fn choice_card(
    ui: &mut Ui,
    selected: bool,
    icon: &str,
    title: &str,
    detail: &str,
    width: f32,
) -> Response {
    let c = Colors::get(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(vec2(width, size::CHOICE_CARD_HEIGHT), Sense::click());
    let t = title.to_string();
    resp.widget_info(|| WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, &t));
    let hot = resp.hovered() && resp.enabled();
    let fill = if selected {
        c.accent_soft
    } else if hot {
        c.bg_hover
    } else {
        c.bg_overlay
    };
    let stroke = if selected {
        Stroke::new(size::BORDER, c.accent)
    } else {
        Stroke::new(size::BORDER, c.border_subtle)
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(radius::M as u8),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(vec2(space::M, space::S));
    let tw = inner.width() - size::ICON_M - space::S;
    // One line each, elided with "…" (a card never grows or clips a second line).
    let one_line = |text: &str, font_id: egui::FontId, color: Color32, width: f32| {
        let mut job = egui::text::LayoutJob::single_section(
            text.to_string(),
            egui::TextFormat::simple(font_id, color),
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let tg = one_line(
        title,
        ui_font(ui.ctx(), font::BODY, true),
        c.text_primary,
        tw,
    );
    let dg = one_line(
        detail,
        ui_font(ui.ctx(), font::CAPTION, false),
        c.text_muted,
        inner.width(),
    );
    let ig = p.layout_no_wrap(
        icon.to_string(),
        ui_font(ui.ctx(), size::ICON_S, false),
        if selected { c.accent } else { c.text_secondary },
    );
    let line1 = tg.size().y.max(ig.size().y);
    let total = line1 + 2.0 + dg.rows.first().map_or(0.0, |r| r.height());
    let top = rect.center().y - total / 2.0;
    p.galley(
        pos2(inner.left(), top + (line1 - ig.size().y) / 2.0),
        ig,
        c.text_secondary,
    );
    let title_rect = Rect::from_min_size(
        pos2(
            inner.left() + size::ICON_M + space::S,
            top + (line1 - tg.size().y) / 2.0,
        ),
        tg.size(),
    );
    let elided = tg.elided;
    p.galley(title_rect.min, tg, c.text_primary);
    // One line of detail, clipped to the card.
    p.with_clip_rect(inner)
        .galley(pos2(inner.left(), top + line1 + 2.0), dg, c.text_muted);
    focus_ring(ui, &resp, rect, radius::M);
    inspect::record(
        ui,
        &format!("card:{title}"),
        "choice-card",
        rect,
        Some(Painted {
            content: title,
            rect: title_rect,
            size: font::BODY,
            truncated: elided,
        }),
        true,
        if selected { &["selected"] } else { &[] },
        Some(c.text_primary),
        Some(fill),
    );
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A labelled form field: the label (caption, semibold) above the control `add` draws, an
/// optional muted help line below, and the field gap after it.
pub fn field<R>(ui: &mut Ui, label: &str, help: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let c = Colors::get(ui.ctx());
    let l = ui.label(
        RichText::new(label)
            .font(ui_font(ui.ctx(), font::CAPTION, true))
            .color(c.text_secondary),
    );
    inspect::record(
        ui,
        &format!("label:{label}"),
        "label",
        l.rect,
        Some(Painted {
            content: label,
            rect: l.rect,
            size: font::CAPTION,
            truncated: false,
        }),
        false,
        &[],
        Some(c.text_secondary),
        None,
    );
    ui.add_space(space::XS - ui.spacing().item_spacing.y.min(space::XS));
    let r = add(ui);
    if let Some(h) = help {
        ui.label(
            RichText::new(h)
                .font(ui_font(ui.ctx(), font::CAPTION, false))
                .color(c.text_muted),
        );
    }
    ui.add_space(space::S);
    r
}

/// A numbered section of a form (`1  Trigger`): a raised card with the step badge, the title,
/// an optional one-line subtitle and the content. Sections of a page share one width and gap.
pub fn form_section<R>(
    ui: &mut Ui,
    step: Option<usize>,
    title: &str,
    subtitle: Option<&str>,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let c = Colors::get(ui.ctx());
    let sid = format!("section:{title}");
    let r = egui::Frame::new()
        .fill(c.bg_raised)
        .stroke(Stroke::new(size::BORDER, c.border_subtle))
        .corner_radius(CornerRadius::same(radius::L as u8))
        .inner_margin(Margin::same(space::L as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = space::S;
                if let Some(n) = step {
                    let (br, _) = ui.allocate_exact_size(Vec2::splat(size::ICON_L), Sense::hover());
                    ui.painter()
                        .circle_filled(br.center(), size::ICON_L / 2.0, c.accent_soft);
                    ui.painter().text(
                        br.center(),
                        Align2::CENTER_CENTER,
                        n.to_string(),
                        ui_font(ui.ctx(), font::CAPTION, true),
                        c.accent,
                    );
                }
                ui.label(
                    RichText::new(title)
                        .font(ui_font(ui.ctx(), font::EMPHASIS, true))
                        .color(c.text_primary),
                );
            });
            if let Some(s) = subtitle {
                ui.label(
                    RichText::new(s)
                        .font(ui_font(ui.ctx(), font::CAPTION, false))
                        .color(c.text_muted),
                );
            }
            ui.add_space(space::S);
            inspect::scope(&sid, || add(ui))
        });
    inspect::record(
        ui,
        &sid,
        "form-section",
        r.response.rect,
        None,
        false,
        &[],
        None,
        Some(c.bg_raised),
    );
    ui.add_space(space::M);
    r.inner
}

/// An item of a popover menu: the full width, `size::MENU_ITEM` high, danger items in the
/// danger colour.
pub fn menu_item(ui: &mut Ui, label: &str, danger: bool) -> Response {
    let c = Colors::get(ui.ctx());
    let w = ui.available_width().max(140.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, size::MENU_ITEM), Sense::click());
    let l = label.to_string();
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::MenuItem, ui.is_enabled(), &l));
    let hot = resp.hovered() && resp.enabled();
    if hot || resp.has_focus() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::S as u8), c.bg_hover);
    }
    let fg = if danger { c.danger } else { c.text_primary };
    let g =
        ui.painter()
            .layout_no_wrap(label.to_string(), ui_font(ui.ctx(), font::BODY, false), fg);
    let tr = Rect::from_min_size(
        pos2(rect.left() + space::S, rect.center().y - g.size().y / 2.0),
        g.size(),
    );
    ui.painter().galley(tr.min, g, fg);
    inspect::record(
        ui,
        &format!("menu-item:{label}"),
        "menu-item",
        rect,
        Some(Painted {
            content: label,
            rect: tr,
            size: font::BODY,
            truncated: false,
        }),
        true,
        if danger { &["danger"] } else { &[] },
        Some(fg),
        None,
    );
    resp
}

/// The frame of popovers and menus: the overlay surface, a subtle border, `radius.m`.
pub fn popover_frame(ctx: &egui::Context) -> egui::Frame {
    let c = Colors::get(ctx);
    egui::Frame::new()
        .fill(c.bg_overlay)
        .stroke(Stroke::new(size::BORDER, c.border_subtle))
        .corner_radius(CornerRadius::same(radius::M as u8))
        .inner_margin(Margin::same(space::S as i8))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(60),
        })
}

/// A popover opened by `anchor` (a toggle button): below it, right-aligned, closed by a click
/// outside or Escape. `width` is the content width (`size::POPOVER_WIDTH` for filters).
pub fn popover<R>(anchor: &Response, width: f32, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let ctx = anchor.ctx.clone();
    egui::Popup::from_toggle_button_response(anchor)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .align(egui::RectAlign::BOTTOM_END)
        .gap(space::XS)
        .frame(popover_frame(&ctx))
        .width(width)
        .show(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = space::XS;
            let r = add(ui);
            if let Some(rect) = egui::Popup::from_toggle_button_response(anchor).get_popup_rect() {
                inspect::record(ui, "popover", "popover", rect, None, false, &[], None, None);
            }
            r
        })
        .map(|r| r.inner)
}

/// A small count pill (an active-filter count) painted at the top right of `rect`.
pub fn count_badge(ui: &Ui, rect: Rect, n: usize) {
    let c = Colors::get(ui.ctx());
    let t = n.to_string();
    let g =
        ui.painter()
            .layout_no_wrap(t, ui_font(ui.ctx(), font::CAPTION, true), c.text_on_accent);
    let w = (g.size().x + space::S).max(16.0);
    let r = Rect::from_center_size(pos2(rect.right() - 2.0, rect.top() + 2.0), vec2(w, 16.0));
    ui.painter()
        .rect_filled(r, CornerRadius::same(radius::PILL as u8), c.accent);
    ui.painter()
        .galley(r.center() - g.size() / 2.0, g, c.text_on_accent);
}

/// A non-interactive status: a dot in the tone's colour and the words (never colour alone).
/// `None` is a neutral (muted) dot.
pub fn status_indicator(ui: &mut Ui, tone: Option<Tone>, text: &str) -> Response {
    let c = Colors::get(ui.ctx());
    let col = match tone {
        Some(Tone::Success) => c.success,
        Some(Tone::Warning) => c.warning,
        Some(Tone::Error) => c.danger,
        Some(Tone::Info) => c.info,
        None => c.text_muted,
    };
    let g = ui.painter().layout_no_wrap(
        text.to_string(),
        ui_font(ui.ctx(), font::CAPTION, false),
        c.text_secondary,
    );
    let w = size::STATUS_DOT + space::S + g.size().x;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, size::HIT_MIN), Sense::hover());
    ui.painter().circle_filled(
        pos2(rect.left() + size::STATUS_DOT / 2.0, rect.center().y),
        size::STATUS_DOT / 2.0,
        col,
    );
    let tr = Rect::from_min_size(
        pos2(
            rect.left() + size::STATUS_DOT + space::S,
            rect.center().y - g.size().y / 2.0,
        ),
        g.size(),
    );
    ui.painter().galley(tr.min, g, c.text_secondary);
    let t = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, &t));
    inspect::record(
        ui,
        &format!("status:{text}"),
        "status",
        rect,
        Some(Painted {
            content: text,
            rect: tr,
            size: font::CAPTION,
            truncated: false,
        }),
        false,
        &[],
        Some(c.text_secondary),
        None,
    );
    resp
}

/// A row of controls of one toolbar, laid out left to right with the toolbar gap; the content
/// is vertically centred on the control height.
pub fn toolbar<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), size::CONTROL),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = space::S;
            add(ui)
        },
    )
    .inner
}

/// The point where a toolbar's right-hand controls start, for a field that fills the rest.
pub fn remaining_width(ui: &Ui, reserved: f32) -> f32 {
    (ui.available_width() - reserved).max(0.0)
}
