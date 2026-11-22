//! The Settings window's Keyboard section (Step 105, SET-23): every command of the palette and
//! the menu bar with its keys, searchable by name or keys.
//!
//! Changing a shortcut (the user layer `hotkeys.json` on the shared `kim-hotkeys` library, with
//! conflicts checked and Reset) is **not in this build yet** — the section says so; it follows in
//! the next part of Step 105.

use crate::JarakApp;
use eframe::egui;
use kim_design::space;

/// The mapper's state while the Keyboard section is open.
#[derive(Debug, Clone, Default)]
pub struct MapperState {
    /// The search text.
    pub filter: String,
    /// The command whose keys are being recorded (the remapping part).
    pub capture: Option<String>,
}

impl JarakApp {
    /// The Keyboard section: the commands with their keys.
    pub(crate) fn shortcut_mapper(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        kim_ui::notice(
            ui,
            kim_ui::Tone::Info,
            lang.pick(
                "Changing shortcuts is not available in this build yet; this list shows every command and its keys.",
                "이 빌드에서는 아직 단축키를 바꿀 수 없습니다. 이 목록은 모든 명령과 그 키를 보여 줍니다.",
            ),
            None,
            None,
        );
        ui.add_space(space::S);
        let Some(w) = self.settings_win.as_mut() else {
            return;
        };
        let width = ui.available_width();
        kim_ui::search_field(
            ui,
            &mut w.keys.filter,
            lang.pick("Search commands", "명령 검색"),
            lang.pick(
                "Name or keys: save, ctrl+shift…",
                "이름 또는 키: 저장, ctrl+shift…",
            ),
            width,
        );
        let filter = w.keys.filter.to_lowercase();
        ui.add_space(space::S);
        let mut rows: Vec<(String, &'static str)> = crate::palette::entries()
            .into_iter()
            .filter_map(|e| {
                let keys = e.keys?;
                let name = lang.pick(&e.en, &e.ko).to_string();
                (filter.is_empty()
                    || name.to_lowercase().contains(&filter)
                    || keys.to_lowercase().contains(&filter))
                .then_some((name, keys))
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows.dedup();
        let title = format!("{} ({})", lang.pick("Commands", "명령"), rows.len());
        kim_ui::setting_group(ui, &title, |ui| {
            for (i, (name, keys)) in rows.iter().enumerate() {
                if i > 0 {
                    kim_ui::row_divider(ui);
                }
                kim_ui::setting_row(ui, name, None, |ui| {
                    // The row's name for screen readers (the title is painted).
                    let label = format!("{name}: {keys}");
                    kim_ui::keycaps(ui, keys)
                        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &label));
                });
            }
        });
    }
}
