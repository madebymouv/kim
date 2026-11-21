//! The tab bar (`JARAK-FR-060`, UI-01, Step 93): one tab per document of a view, with its
//! name, a dot while it has unsaved changes, a close `×`, a middle click to close, dragging to
//! reorder and a context menu.
//!
//! Laid out from the design system's tab tokens (the compact bar of `size::TABBAR_HEIGHT_COMPACT`,
//! tabs of one width between `TAB_MIN_WIDTH` and `TAB_MAX_WIDTH`) and recorded for the UI audit
//! (`tabbar` and its `tab` components), so the audit's tab rules measure it. When the tabs do not
//! fit, the bar scrolls with the wheel and always keeps the active tab in view.

use eframe::egui::{self, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use kim_design::{Lang, font, size, space};
use kim_ui::Colors;

/// A tab as the bar shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub title: String,
    /// The file's path (the tooltip), when it has one.
    pub path: Option<String>,
    pub dirty: bool,
    pub active: bool,
    /// The document is in both views (a clone).
    pub cloned: bool,
}

/// A command of the tab context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Close,
    CloseOthers,
    CloseLeft,
    CloseRight,
    CloseUnchanged,
    CloseAll,
    SaveAll,
    CopyPath,
    CopyName,
    CopyFolder,
    OpenFolder,
    OpenTerminal,
    OpenDefault,
    MoveOther,
    CloneOther,
    NewWindow,
    MoveNewWindow,
    Rename,
    Recycle,
}

impl Menu {
    /// The menu's items in order (`None` is a separator).
    pub const ORDER: [Option<Menu>; 22] = [
        Some(Menu::Close),
        Some(Menu::CloseOthers),
        Some(Menu::CloseLeft),
        Some(Menu::CloseRight),
        Some(Menu::CloseUnchanged),
        Some(Menu::CloseAll),
        None,
        Some(Menu::SaveAll),
        Some(Menu::Rename),
        Some(Menu::Recycle),
        None,
        Some(Menu::CopyPath),
        Some(Menu::CopyName),
        Some(Menu::CopyFolder),
        Some(Menu::OpenFolder),
        Some(Menu::OpenTerminal),
        Some(Menu::OpenDefault),
        None,
        Some(Menu::MoveOther),
        Some(Menu::CloneOther),
        Some(Menu::NewWindow),
        Some(Menu::MoveNewWindow),
    ];

    pub fn label(self, lang: Lang) -> &'static str {
        let (en, ko) = match self {
            Menu::Close => ("Close", "닫기"),
            Menu::CloseOthers => ("Close the others", "다른 문서 닫기"),
            Menu::CloseLeft => ("Close the ones to the left", "왼쪽 문서 닫기"),
            Menu::CloseRight => ("Close the ones to the right", "오른쪽 문서 닫기"),
            Menu::CloseUnchanged => ("Close the unchanged ones", "변경 없는 문서 닫기"),
            Menu::CloseAll => ("Close all", "모두 닫기"),
            Menu::SaveAll => ("Save all", "모두 저장"),
            Menu::CopyPath => ("Copy the full path", "전체 경로 복사"),
            Menu::CopyName => ("Copy the file name", "파일 이름 복사"),
            Menu::CopyFolder => ("Copy the folder path", "폴더 경로 복사"),
            Menu::OpenFolder => ("Show in Explorer", "탐색기에서 보기"),
            Menu::OpenTerminal => ("Open a command prompt here", "여기서 명령 프롬프트 열기"),
            Menu::OpenDefault => ("Open in the default program", "기본 프로그램으로 열기"),
            Menu::MoveOther => ("Move to the other view", "다른 보기로 옮기기"),
            Menu::CloneOther => ("Clone to the other view", "다른 보기에 복제"),
            Menu::NewWindow => ("Open in a new window", "새 창에서 열기"),
            Menu::MoveNewWindow => ("Move to a new window", "새 창으로 옮기기"),
            Menu::Rename => ("Rename the file…", "파일 이름 바꾸기…"),
            Menu::Recycle => ("Move the file to the Recycle Bin…", "파일을 휴지통으로…"),
        };
        lang.pick(en, ko)
    }

    /// The command needs the document's file.
    pub fn needs_file(self) -> bool {
        matches!(
            self,
            Menu::CopyPath
                | Menu::CopyName
                | Menu::CopyFolder
                | Menu::OpenFolder
                | Menu::OpenTerminal
                | Menu::OpenDefault
                | Menu::NewWindow
                | Menu::MoveNewWindow
                | Menu::Rename
                | Menu::Recycle
        )
    }
}

/// What happened on the bar this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Activate(usize),
    Close(usize),
    Menu(usize, Menu),
    /// Move tab `from` to position `to`.
    Reorder(usize, usize),
}

/// The width of each of `n` tabs in a bar `w` wide (the same width for all).
pub fn tab_width(w: f32, n: usize) -> f32 {
    if n == 0 {
        return size::TAB_MAX_WIDTH;
    }
    ((w - 2.0 * space::S) / n as f32).clamp(size::TAB_MIN_WIDTH, size::TAB_MAX_WIDTH)
}

/// Draws the bar for `items` in `rect` (its height is `size::TABBAR_HEIGHT_COMPACT`); `id` keeps
/// its scroll and drag apart from another view's bar.
pub fn bar(
    ui: &mut egui::Ui,
    rect: Rect,
    items: &[Item],
    id: &str,
    lang: Lang,
    focused: bool,
) -> Vec<Event> {
    let c = Colors::get(ui.ctx());
    let mut events = Vec::new();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, c.bg_raised);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(size::BORDER, c.border_subtle),
    );
    let bar_id = format!("tabbar:{id}");
    kim_ui::inspect::record(
        ui,
        &bar_id,
        "tabbar",
        rect,
        None,
        false,
        &[],
        None,
        Some(c.bg_raised),
    );
    // Whole points: neighbouring tabs never share a pixel.
    let w = tab_width(rect.width(), items.len()).floor();
    let h = size::TABBAR_HEIGHT_COMPACT - size::TAB_INSET_TOP;
    let total = w * items.len() as f32;
    let room = rect.width() - 2.0 * space::S;
    // The scroll offset (memory), clamped, the active tab kept in view.
    let mem = egui::Id::unique(("jarak-tab-scroll", id));
    let mut off: f32 = ui.ctx().data(|d| d.get_temp(mem)).unwrap_or(0.0);
    let hovered_bar = ui.rect_contains_pointer(rect);
    if hovered_bar {
        let d = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
        off -= d;
    }
    if let Some(a) = items.iter().position(|t| t.active) {
        let (l, r) = (a as f32 * w, (a + 1) as f32 * w);
        if l < off {
            off = l;
        } else if r > off + room {
            off = r - room;
        }
    }
    off = off.clamp(0.0, (total - room).max(0.0));
    ui.ctx().data_mut(|d| d.insert_temp(mem, off));
    let drag_mem = egui::Id::unique(("jarak-tab-drag", id));
    let dragging: Option<usize> = ui.ctx().data(|d| d.get_temp(drag_mem));
    let pointer = ui.input(|i| i.pointer.hover_pos());
    kim_ui::inspect::scope(&bar_id, || {
        for (i, t) in items.iter().enumerate() {
            let x = (rect.left() + space::S + i as f32 * w - off).round();
            let tr = Rect::from_min_size(
                Pos2::new(x, (rect.top() + size::TAB_INSET_TOP).round()),
                Vec2::new(w, h),
            );
            if tr.right() < rect.left() || tr.left() > rect.right() {
                continue;
            }
            let resp = ui.interact(
                tr,
                egui::Id::unique(("jarak-tab", id, i)),
                Sense::click_and_drag(),
            );
            let label = format!("{}{}", t.title, if t.dirty { " •" } else { "" });
            resp.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::Tab, true, t.active, &label)
            });
            let hot = resp.hovered();
            let fill = if t.active {
                c.bg_base
            } else if hot {
                c.bg_hover
            } else {
                c.bg_raised
            };
            let r = CornerRadius {
                nw: kim_design::radius::S as u8,
                ne: kim_design::radius::S as u8,
                sw: 0,
                se: 0,
            };
            painter.rect_filled(tr, r, fill);
            if t.active {
                // The focused view's active tab carries the accent; the other view's a quiet line.
                let marker = if focused { c.accent } else { c.border_strong };
                painter.hline(tr.x_range(), tr.top() + 1.0, Stroke::new(2.0, marker));
            }
            // The close button at the right; the title before it, cut with an ellipsis.
            let close = Rect::from_center_size(
                Pos2::new(tr.right() - size::TAB_CLOSE / 2.0 - 2.0, tr.center().y),
                Vec2::splat(size::TAB_CLOSE.min(h)),
            );
            let close_resp = ui.interact(
                close,
                egui::Id::unique(("jarak-tab-close", id, i)),
                Sense::click(),
            );
            let close_label = format!("{} {}", lang.pick("Close", "닫기"), t.title);
            close_resp
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &close_label));
            let max_text = (close.left() - tr.left() - size::TAB_PAD_X - space::XS).max(10.0);
            let fg = if t.active {
                c.text_primary
            } else {
                c.text_secondary
            };
            let font_id = FontId::proportional(font::CAPTION);
            let full = format!("{}{}", if t.cloned { "⧉ " } else { "" }, t.title);
            let mut shown = full.clone();
            let mut g = painter.layout_no_wrap(shown.clone(), font_id.clone(), fg);
            let mut truncated = false;
            while g.size().x > max_text && shown.chars().count() > 1 {
                shown.pop();
                truncated = true;
                g = painter.layout_no_wrap(format!("{shown}…"), font_id.clone(), fg);
            }
            let text_rect = Rect::from_min_size(
                Pos2::new(
                    tr.left() + size::TAB_PAD_X,
                    tr.center().y - g.size().y / 2.0,
                ),
                g.size(),
            );
            painter.galley(text_rect.min, g, fg);
            let x_color = if close_resp.hovered() {
                c.text_primary
            } else {
                c.text_muted
            };
            if t.dirty && !close_resp.hovered() {
                painter.circle_filled(close.center(), 3.5, c.accent);
            } else {
                let k = size::TAB_CLOSE_ICON / 2.0;
                let cc = close.center();
                if close_resp.hovered() {
                    painter.rect_filled(
                        close.shrink(3.0),
                        CornerRadius::same(kim_design::radius::S as u8),
                        c.bg_hover,
                    );
                }
                painter.line_segment(
                    [cc - Vec2::splat(k), cc + Vec2::splat(k)],
                    Stroke::new(1.2, x_color),
                );
                painter.line_segment(
                    [cc + Vec2::new(-k, k), cc + Vec2::new(k, -k)],
                    Stroke::new(1.2, x_color),
                );
            }
            let mut states = Vec::new();
            if t.active {
                states.push("active");
            }
            if t.dirty {
                states.push("dirty");
            }
            kim_ui::inspect::record(
                ui,
                &format!("tab[{i}]"),
                "tab",
                tr,
                Some(kim_ui::inspect::Painted {
                    content: &full,
                    rect: text_rect,
                    size: font::CAPTION,
                    truncated,
                }),
                true,
                &states,
                Some(fg),
                Some(fill),
            );
            if let Some(p) = &t.path {
                resp.clone().on_hover_text_at_pointer(p);
            }
            if close_resp.clicked() || resp.middle_clicked() {
                events.push(Event::Close(i));
            } else if resp.clicked() {
                events.push(Event::Activate(i));
            }
            if resp.drag_started() {
                ui.ctx().data_mut(|d| d.insert_temp(drag_mem, i));
                events.push(Event::Activate(i));
            }
            if let (Some(from), Some(p)) = (dragging, pointer)
                && from != i
                && tr.contains(p)
                && ui.input(|inp| inp.pointer.primary_down())
            {
                events.push(Event::Reorder(from, i));
                ui.ctx().data_mut(|d| d.insert_temp(drag_mem, i));
            }
            resp.context_menu(|ui| {
                ui.set_min_width(260.0);
                for m in Menu::ORDER {
                    match m {
                        None => {
                            ui.separator();
                        }
                        Some(m) => {
                            if m.needs_file() && t.path.is_none() {
                                continue;
                            }
                            if kim_ui::menu_item(ui, m.label(lang), m == Menu::Recycle).clicked() {
                                events.push(Event::Menu(i, m));
                                ui.close();
                            }
                        }
                    }
                }
            });
        }
    });
    if !ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().data_mut(|d| d.remove::<usize>(drag_mem));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_share_one_width_within_the_design_limits() {
        assert_eq!(tab_width(1000.0, 3), size::TAB_MAX_WIDTH);
        assert_eq!(
            tab_width(1000.0, 40),
            size::TAB_MIN_WIDTH,
            "many tabs scroll at the minimum"
        );
        let w = tab_width(600.0, 4);
        assert!((size::TAB_MIN_WIDTH..=size::TAB_MAX_WIDTH).contains(&w));
        assert!((w * 4.0 - (600.0 - 2.0 * space::S)).abs() < 0.01);
    }

    #[test]
    fn the_context_menu_hides_file_commands_for_an_untitled_document() {
        let file_only: Vec<Menu> = Menu::ORDER
            .iter()
            .flatten()
            .copied()
            .filter(|m| m.needs_file())
            .collect();
        assert!(file_only.contains(&Menu::Recycle) && file_only.contains(&Menu::OpenFolder));
        assert!(!Menu::Close.needs_file() && !Menu::CloneOther.needs_file());
        for m in Menu::ORDER.iter().flatten() {
            assert!(!m.label(Lang::En).is_empty() && !m.label(Lang::Ko).is_empty());
        }
    }
}
