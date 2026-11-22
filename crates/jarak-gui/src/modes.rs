//! Step 110: window modes and the text's context menu.
//!
//! - **Always on top** (VIEW-01), **full screen** (F11, VIEW-02), **post-it** (F12: no window
//!   frame, no menu, tab or status bar; VIEW-03) and **distraction-free** (full screen without them,
//!   the text in a centred column; VIEW-04). The window gets the viewport commands once per change.
//! - `jarak-gui --position x,y --always-on-top` (HELP-07).
//! - The context menu of the text (UI-08), its commands from the `context_menu` setting (SET-25).

use crate::JarakApp;
use eframe::egui;

/// Which window modes are on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modes {
    pub on_top: bool,
    pub full_screen: bool,
    pub post_it: bool,
    pub distraction_free: bool,
}

/// The widest column of text in distraction-free mode (points at zoom 1).
pub const FOCUS_WIDTH: f32 = 860.0;

/// `x,y` as a window position (`--position 100,80`).
pub fn parse_position(s: &str) -> Option<(f32, f32)> {
    let (x, y) = s.split_once(',')?;
    let x: f32 = x.trim().parse().ok()?;
    let y: f32 = y.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The default commands of the text's context menu (`-` is a separator).
pub const DEFAULT_CONTEXT_MENU: &str =
    "edit.undo,edit.redo,-,edit.cut,edit.copy,edit.paste,-,select.all,-,edit.copy-html";

impl JarakApp {
    /// The menu, tab and status bars are hidden (post-it or distraction-free).
    pub fn chrome_hidden(&self) -> bool {
        self.modes.post_it || self.modes.distraction_free
    }

    /// Toggles a mode: 0 always on top, 1 full screen, 2 post-it, 3 distraction-free.
    pub fn toggle_mode(&mut self, m: u8) {
        let md = &mut self.modes;
        let (on, en, ko) = match m {
            0 => {
                md.on_top = !md.on_top;
                (md.on_top, "Always on top", "항상 위")
            }
            1 => {
                md.full_screen = !md.full_screen;
                if !md.full_screen {
                    md.distraction_free = false;
                }
                (md.full_screen, "Full screen", "전체 화면")
            }
            2 => {
                md.post_it = !md.post_it;
                (md.post_it, "Post-it mode", "포스트잇 모드")
            }
            _ => {
                md.distraction_free = !md.distraction_free;
                (md.distraction_free, "Distraction-free mode", "집중 모드")
            }
        };
        self.message = Some(format!(
            "{}: {}",
            self.t(en, ko),
            if on {
                self.t("on", "켜짐")
            } else {
                self.t("off", "꺼짐")
            }
        ));
    }

    /// Each frame: the viewport commands for the modes that changed.
    pub(crate) fn modes_frame(&mut self, ctx: &egui::Context) {
        let (want, have) = (self.modes, self.modes_applied);
        if want == have {
            return;
        }
        if want.on_top != have.on_top {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if want.on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        }
        let full = want.full_screen || want.distraction_free;
        if full != (have.full_screen || have.distraction_free) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(full));
        }
        if want.post_it != have.post_it {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!want.post_it));
        }
        self.modes_applied = want;
    }

    /// The editor's area: in distraction-free mode a centred column.
    pub(crate) fn focus_rect(&self, rect: egui::Rect, zoom: f32) -> egui::Rect {
        if !self.modes.distraction_free {
            return rect;
        }
        let w = (FOCUS_WIDTH * zoom).min(rect.width());
        let x = rect.center().x - w / 2.0;
        egui::Rect::from_min_max(
            egui::pos2(x.round(), rect.top()),
            egui::pos2((x + w).round(), rect.bottom()),
        )
    }

    /// The text's context menu (the `context_menu` setting's commands).
    pub(crate) fn text_context_menu(&mut self, ui: &egui::Ui, salt: u8) {
        let Some(resp) = ui.ctx().read_response(crate::view::view_id_for(salt)) else {
            return;
        };
        let ids: Vec<String> = self.prefs.context_menu.clone();
        let mut entries = crate::palette::entries();
        entries.extend(self.extension_entries());
        let lang = self.lang;
        let mut run = None;
        resp.context_menu(|ui| {
            ui.set_min_width(220.0);
            for id in &ids {
                if id == "-" {
                    ui.separator();
                    continue;
                }
                let Some(e) = entries.iter().find(|e| e.id == id.as_str()) else {
                    continue;
                };
                let label = match lang {
                    kim_design::Lang::Ko => e.ko.as_ref(),
                    _ => e.en.as_ref(),
                };
                if kim_ui::menu_item(ui, label, false).clicked() {
                    run = Some(e.run);
                    ui.close();
                }
            }
        });
        if let Some(r) = run {
            self.run_entry(r);
        }
    }
}

#[cfg(test)]
mod tests {
    /// HELP-07 (`JARAK-FR-150`): `--position x,y`.
    #[test]
    fn a_position_is_two_numbers() {
        assert_eq!(super::parse_position("100,80"), Some((100.0, 80.0)));
        assert_eq!(super::parse_position(" -20 , 0.5 "), Some((-20.0, 0.5)));
        for bad in ["", "100", "a,b", "1,2,3", "inf,0", "NaN,1"] {
            assert_eq!(super::parse_position(bad), None, "{bad}");
        }
    }

    /// UI-08 (`JARAK-FR-031`): every default context menu command exists.
    #[test]
    fn the_default_context_menu_names_real_commands() {
        let e = crate::palette::entries();
        for id in super::DEFAULT_CONTEXT_MENU.split(',') {
            assert!(id == "-" || e.iter().any(|x| x.id == id), "{id}");
        }
    }
}
