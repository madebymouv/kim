//! Jarak's editor window (`jarak-gui.exe`, ADR 0102).
//!
//! Step 85 is the shell: the window in the design system, an empty document, the status bar
//! with the Settings gear at its right end, no toolbar, and the hand-off of files from a second
//! start. The text view arrives in Step 88 (on the core of Steps 86 and 87); until then the
//! document area says so instead of pretending to edit (AGENTS.md: no fake completeness).

pub mod handoff;

use eframe::egui::{self, Align, Color32, FontId, Layout, Rect, RichText, Sense, Vec2};
use jarak_core::target::OpenTarget;
use kim_design::{Lang, font, size, space};
use kim_ui::Colors;
use std::sync::{Arc, Mutex};

/// The window title of the empty window.
pub const TITLE: &str = "Jarak";

/// A document the window was asked to open (Step 85 shows it; editing starts in Step 88).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requested {
    pub target: OpenTarget,
}

impl Requested {
    pub fn name(&self) -> String {
        self.target.path.file_name().map_or_else(
            || self.target.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }
}

/// Files handed over by later starts, filled by the hand-off thread.
pub type Incoming = Arc<Mutex<Vec<Vec<String>>>>;

/// The window's state.
#[derive(Debug)]
pub struct JarakApp {
    pub lang: Lang,
    pub docs: Vec<Requested>,
    pub incoming: Incoming,
    /// The note under the gear is open.
    pub settings_note: bool,
    title: String,
    /// The design system is installed (on the first frame, also in the headless tests).
    themed: bool,
}

impl JarakApp {
    pub fn new(lang: Lang, args: &[String], incoming: Incoming) -> JarakApp {
        let mut app = JarakApp {
            lang,
            docs: Vec::new(),
            incoming,
            settings_note: false,
            title: String::new(),
            themed: false,
        };
        app.open_args(args);
        app
    }

    /// Adds the documents named by `path[:line[:column]]` arguments.
    pub fn open_args(&mut self, args: &[String]) {
        for a in args {
            if let Some(target) = OpenTarget::parse(a) {
                let r = Requested { target };
                if !self.docs.contains(&r) {
                    self.docs.push(r);
                }
            }
        }
    }

    fn t<'a>(&self, en: &'a str, ko: &'a str) -> &'a str {
        self.lang.pick(en, ko)
    }

    /// The window title for the current documents.
    pub fn window_title(&self) -> String {
        match self.docs.last() {
            Some(d) => format!("{} — {TITLE}", d.name()),
            None => TITLE.to_string(),
        }
    }

    /// One frame: status bar, document area, the gear's note.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.themed {
            kim_ui::apply(&ctx, kim_ui::Options::default());
            self.themed = true;
        }
        let pending: Vec<Vec<String>> = self
            .incoming
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        for args in pending {
            self.open_args(&args);
        }
        let title = self.window_title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let c = Colors::get(&ctx);
        self.status_bar(ui, &c);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(c.bg_base))
            .show(ui, |ui| self.document(ui, &c));
        if kim_ui::inspect::active() {
            kim_ui::inspect::finish_frame(&ctx, "jarak", "editor");
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let ctx = ui.ctx().clone();
        egui::Panel::bottom("jarak-status")
            .exact_size(size::CONTROL)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::S as i8, 0)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                let bar = ui.max_rect();
                ui.painter().hline(
                    bar.x_range().expand(space::S),
                    bar.top(),
                    egui::Stroke::new(size::BORDER, c.border_subtle),
                );
                let gear = Rect::from_min_size(
                    egui::pos2(bar.right() - size::CONTROL_COMPACT, bar.center().y - size::CONTROL_COMPACT / 2.0),
                    Vec2::splat(size::CONTROL_COMPACT),
                );
                let label = self.t("Settings", "설정").to_string();
                if kim_ui::gear_at(ui, gear, &label, self.settings_note).clicked() {
                    self.settings_note = !self.settings_note;
                }
                let text_area = Rect::from_min_max(bar.min, egui::pos2(gear.left() - space::S, bar.max.y));
                ui.scope_builder(egui::UiBuilder::new().max_rect(text_area), |ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = space::L;
                        for item in [
                            self.t("Ln 1, Col 1", "줄 1, 열 1"),
                            "UTF-8",
                            "CRLF",
                            self.t("Plain text", "일반 텍스트"),
                        ] {
                            ui.label(
                                RichText::new(item)
                                    .size(font::CAPTION)
                                    .color(c.text_secondary),
                            );
                        }
                    });
                });
                if self.settings_note {
                    let note = self.t(
                        "Settings open in a window of their own once Step 105 (#124) is done; nothing to set yet.",
                        "설정은 Step 105(#124)가 끝나면 별도 창으로 열립니다. 아직 바꿀 설정이 없습니다.",
                    );
                    egui::Area::new(egui::Id::unique("jarak-settings-note"))
                        .order(egui::Order::Foreground)
                        .pivot(egui::Align2::RIGHT_BOTTOM)
                        .fixed_pos(egui::pos2(bar.right(), bar.top() - space::XS))
                        .show(&ctx, |ui| {
                            kim_ui::popover_frame(&ctx).show(ui, |ui| {
                                ui.set_max_width(size::POPOVER_WIDTH);
                                ui.label(RichText::new(note).size(font::BODY).color(c.text_primary));
                            });
                        });
                }
            });
    }

    fn document(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let area = ui.max_rect();
        let mono = FontId::monospace(font::TERMINAL);
        let row = (font::TERMINAL * font::LINE_HEIGHT_TERMINAL).round();
        let gutter = space::XXL + space::S;
        let top = area.top() + space::S;
        let p = ui.painter();
        // The gutter: line 1 of the empty document.
        p.text(
            egui::pos2(area.left() + gutter - space::S, top),
            egui::Align2::RIGHT_TOP,
            "1",
            mono.clone(),
            c.text_muted,
        );
        // The caret at line 1, column 1, in the accent (JARAK_PHILOSOPHY §1: purple where it means
        // something).
        let x = area.left() + gutter + space::S;
        p.rect_filled(
            Rect::from_min_size(egui::pos2(x, top), Vec2::new(size::FOCUS_RING, row)),
            0.0,
            c.accent,
        );
        // What this build can do, said plainly in the empty document.
        let mut lines: Vec<(String, Color32)> = vec![(
            self.t(
                "Jarak is in development: this window does not edit text yet (Steps 86–88).",
                "Jarak은 개발 중입니다. 이 창은 아직 텍스트를 편집하지 않습니다(Step 86–88).",
            )
            .to_string(),
            c.text_muted,
        )];
        for d in &self.docs {
            let pos = match (d.target.line, d.target.column) {
                (Some(l), Some(col)) => format!(":{l}:{col}"),
                (Some(l), None) => format!(":{l}"),
                _ => String::new(),
            };
            lines.push((
                format!(
                    "{} {}{pos}",
                    self.t("Asked to open", "열기 요청:"),
                    d.target.path.display()
                ),
                c.text_secondary,
            ));
        }
        for (i, (text, col)) in lines.iter().enumerate() {
            p.text(
                egui::pos2(x + space::M, top + row * (i as f32 + 2.0)),
                egui::Align2::LEFT_TOP,
                text,
                mono.clone(),
                *col,
            );
        }
        ui.allocate_rect(area, Sense::hover());
    }
}

impl eframe::App for JarakApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        JarakApp::ui(self, ui);
    }
}
