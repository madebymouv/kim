//! The shortcut mapper (Step 105, SET-23, docs/HOTKEY_CONTRACT.md; the rules:
//! `jarak_core::hotkeys`): the Settings window's Keyboard section and the window's key dispatch
//! with the user's `hotkeys.json`.
//!
//! - **The manifest** is every command of the palette that has keys ([`manifest_json`]); its
//!   defaults are Jarak's key table (`keys::key_label`). Commands on keys the contract reserves
//!   for every Kim window (the navigation and editing keys: arrows, Home, End, Page Up/Down, Tab,
//!   Enter, Backspace, Delete, Insert, Esc and their Shift forms — §7.2) are the editor's own and
//!   are not remappable. A test keeps the shipped `jarak-core/src/hotkeys.manifest.json` equal to
//!   it.
//! - **Dispatch** ([`Bindings`]): a key the user bound runs its command; the default keys of a
//!   command the user rebound or turned off do nothing (unless another command took them);
//!   every other key works as before. The window reads `hotkeys.json` again when it gets the focus
//!   and the file changed (`jarak hotkeys set` while it runs).
//! - **The Keyboard section**: every command with its keys, searchable by name or keys; *Change*
//!   records keys (Esc cancels), *Off* removes them, *Reset* gives the defaults back; a change that
//!   takes keys another command has is refused and named, with *Take it* to move them; *Reset all
//!   shortcuts* asks twice.

use crate::JarakApp;
use crate::keys::{Action, parse_label};
use eframe::egui::{self, Key, Modifiers};
use jarak_core::hotkeys::{self as jh, Change};
use kim_design::space;
use std::path::{Path, PathBuf};

/// A pressed chord: the key and Ctrl, Alt, Shift.
pub type Chord4 = (Key, bool, bool, bool);

/// egui's keys and the contract's names for them (docs/HOTKEY_CONTRACT.md §5.2).
const NAMES: &[(Key, &str)] = &[
    (Key::A, "A"),
    (Key::B, "B"),
    (Key::C, "C"),
    (Key::D, "D"),
    (Key::E, "E"),
    (Key::F, "F"),
    (Key::G, "G"),
    (Key::H, "H"),
    (Key::I, "I"),
    (Key::J, "J"),
    (Key::K, "K"),
    (Key::L, "L"),
    (Key::M, "M"),
    (Key::N, "N"),
    (Key::O, "O"),
    (Key::P, "P"),
    (Key::Q, "Q"),
    (Key::R, "R"),
    (Key::S, "S"),
    (Key::T, "T"),
    (Key::U, "U"),
    (Key::V, "V"),
    (Key::W, "W"),
    (Key::X, "X"),
    (Key::Y, "Y"),
    (Key::Z, "Z"),
    (Key::Num0, "0"),
    (Key::Num1, "1"),
    (Key::Num2, "2"),
    (Key::Num3, "3"),
    (Key::Num4, "4"),
    (Key::Num5, "5"),
    (Key::Num6, "6"),
    (Key::Num7, "7"),
    (Key::Num8, "8"),
    (Key::Num9, "9"),
    (Key::F1, "F1"),
    (Key::F2, "F2"),
    (Key::F3, "F3"),
    (Key::F4, "F4"),
    (Key::F5, "F5"),
    (Key::F6, "F6"),
    (Key::F7, "F7"),
    (Key::F8, "F8"),
    (Key::F9, "F9"),
    (Key::F10, "F10"),
    (Key::F11, "F11"),
    (Key::F12, "F12"),
    (Key::Escape, "Escape"),
    (Key::Tab, "Tab"),
    (Key::Enter, "Enter"),
    (Key::Space, "Space"),
    (Key::Backspace, "Backspace"),
    (Key::Insert, "Insert"),
    (Key::Delete, "Delete"),
    (Key::Home, "Home"),
    (Key::End, "End"),
    (Key::PageUp, "PageUp"),
    (Key::PageDown, "PageDown"),
    (Key::ArrowUp, "Up"),
    (Key::ArrowDown, "Down"),
    (Key::ArrowLeft, "Left"),
    (Key::ArrowRight, "Right"),
    (Key::Plus, "Plus"),
    (Key::Equals, "Plus"),
    (Key::Minus, "Minus"),
    (Key::Comma, "Comma"),
    (Key::Period, "Period"),
];

/// The contract's name of an egui key.
pub fn key_name(k: Key) -> Option<&'static str> {
    NAMES.iter().find(|(x, _)| *x == k).map(|(_, n)| *n)
}

/// The egui key of a contract name.
pub fn name_key(n: &str) -> Option<Key> {
    NAMES.iter().find(|(_, x)| *x == n).map(|(k, _)| *k)
}

/// The contract's text of a pressed chord (`Ctrl+Shift+R`), or `None` for a key it has no name
/// for.
pub fn chord_text(key: Key, m: Modifiers) -> Option<String> {
    let name = key_name(key)?;
    let mut s = String::new();
    if m.ctrl || m.command {
        s.push_str("Ctrl+");
    }
    if m.alt {
        s.push_str("Alt+");
    }
    if m.shift {
        s.push_str("Shift+");
    }
    s.push_str(name);
    Some(s)
}

/// A Jarak key label (`Ctrl+,`, `Alt+Shift+0`) in the contract's syntax (`Ctrl+Comma`).
pub fn canonical(label: &str) -> Option<String> {
    let (k, m) = parse_label(label)?;
    chord_text(k, m)
}

/// The chord of a binding in the contract's syntax (one step; Win, AltGr and `code:` / `char:`
/// keys are not chords this window matches).
pub fn spec_chord(spec: &str) -> Option<Chord4> {
    let c = kim_hotkeys::keys::Chord::parse(spec).ok()?;
    if c.mods.win.on() || c.mods.altgr {
        return None;
    }
    let kim_hotkeys::keys::Key::Named(n) = c.key else {
        return None;
    };
    Some((
        name_key(n)?,
        c.mods.ctrl.on(),
        c.mods.alt.on(),
        c.mods.shift.on(),
    ))
}

fn pressed(key: Key, m: Modifiers) -> Chord4 {
    let key = if key == Key::Equals { Key::Plus } else { key };
    (key, m.ctrl || m.command, m.alt, m.shift)
}

/// The manifest built from the window's commands (shipped as jarak-core's
/// `hotkeys.manifest.json`; the contract example `kim-hotkey-manifest-v1/jarak.json`).
pub fn manifest_json() -> serde_json::Value {
    let mut actions = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in crate::palette::entries() {
        let crate::palette::Run::Act(_) = e.run else {
            continue;
        };
        if !seen.insert(e.id) {
            continue;
        }
        // Step 106: a command without default keys (Print…) is listed too, so it can get some.
        let defaults: Vec<String> = match e.keys {
            None => Vec::new(),
            Some(keys) => {
                let Some(spec) = canonical(keys) else {
                    continue;
                };
                // A key reserved for every Kim window (navigation and editing) is the editor's
                // own.
                if kim_hotkeys::keys::Chord::parse(&spec).is_ok_and(|c| {
                    kim_hotkeys::reserved::chord(&c, &kim_hotkeys::scope::Scope::App, true)
                        .is_some()
                }) {
                    continue;
                }
                vec![spec]
            }
        };
        actions.push(serde_json::json!({
            "id": e.id, "title": e.en, "scopes": ["app"], "defaults": defaults
        }));
    }
    serde_json::json!({
        "schema": "kim.hotkey-manifest",
        "schema_version": 1,
        "hotkey_contract": 1,
        "tool": jh::TOOL,
        "version": jarak_core::VERSION,
        "backend": "focus",
        "actions": actions,
    })
}

/// The command of a manifest action id.
fn action_of_id(id: &str) -> Option<Action> {
    crate::palette::entries()
        .into_iter()
        .find(|e| e.id == id)
        .and_then(|e| match e.run {
            crate::palette::Run::Act(a) => Some(a),
            _ => None,
        })
}

/// The user's bindings as the window matches them.
#[derive(Debug, Clone, Default)]
pub struct Bindings {
    pub path: Option<PathBuf>,
    /// Chords the user bound, and their commands.
    pub user: Vec<(Chord4, Action)>,
    /// Default chords of commands the user rebound or turned off.
    pub freed: Vec<Chord4>,
    /// Why the file is not used (safe mode), for the Keyboard section.
    pub problem: Option<String>,
    pub stamp: Option<std::time::SystemTime>,
}

impl Bindings {
    /// The user layer at `path` (a missing file: no changes; a broken or newer one: safe mode).
    pub fn load(path: &Path) -> Bindings {
        let (tool, loaded) = jh::load(path);
        let mut b = Bindings {
            path: Some(path.to_path_buf()),
            stamp: std::fs::metadata(path).and_then(|m| m.modified()).ok(),
            ..Bindings::default()
        };
        if loaded.state.safe_mode() {
            b.problem = Some(loaded.problems.join("; "));
            return b;
        }
        for a in &tool.manifest.actions {
            let (keys, changed) = jh::keys_of(&tool, &a.id);
            if !changed {
                continue;
            }
            for d in &a.defaults {
                if let Some(c) = spec_chord(d.keys()) {
                    b.freed.push(c);
                }
            }
            if let Some(act) = action_of_id(&a.id) {
                for k in keys {
                    if let Some(c) = spec_chord(&k) {
                        b.user.push((c, act));
                    }
                }
            }
        }
        b
    }

    /// What a key does with the user's bindings: `Some(Some(a))` a command the user bound,
    /// `Some(None)` a freed default (nothing), `None` the window's own table decides.
    pub fn lookup(&self, key: Key, m: Modifiers) -> Option<Option<Action>> {
        let c = pressed(key, m);
        if let Some((_, a)) = self.user.iter().find(|(x, _)| *x == c) {
            return Some(Some(*a));
        }
        self.freed.contains(&c).then_some(None)
    }
}

/// The mapper's state while the Keyboard section is open.
#[derive(Debug, Clone, Default)]
pub struct MapperState {
    /// The search text.
    pub filter: String,
    /// The command whose keys are being recorded.
    pub capture: Option<String>,
    /// The last refusal: the command, the keys, the message, whether *Take it* can resolve it.
    pub refused: Option<(String, String, String, bool)>,
    /// *Reset all shortcuts* was pressed once.
    pub confirm_reset: bool,
}

impl JarakApp {
    /// Keeps the shortcut file `path` (`%APPDATA%\mouv\jarak\hotkeys.json`) and applies it.
    pub fn with_hotkeys(mut self, path: &Path) -> JarakApp {
        self.bindings = Bindings::load(path);
        self
    }

    /// The command of a key: the user's bindings first, then Jarak's key table.
    pub(crate) fn action_of(&self, key: Key, m: Modifiers) -> Option<Action> {
        match self.bindings.lookup(key, m) {
            Some(a) => a,
            None => crate::keys::action_for(key, m),
        }
    }

    /// Reads `hotkeys.json` again when it changed since the window read it; `true` when it did.
    pub fn hotkeys_changed_outside(&mut self) -> bool {
        let Some(p) = self.bindings.path.clone() else {
            return false;
        };
        let stamp = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
        if stamp == self.bindings.stamp {
            return false;
        }
        self.bindings = Bindings::load(&p);
        true
    }

    /// Applies a shortcut change and reloads the bindings; a refusal is kept for the row.
    pub fn change_shortcut(&mut self, c: &Change, replace: bool) -> bool {
        let Some(path) = self.bindings.path.clone() else {
            self.message = Some(
                self.t(
                    "There is no shortcut file in this window; nothing was saved.",
                    "이 창에는 단축키 파일이 없어 저장하지 않았습니다.",
                )
                .into(),
            );
            return false;
        };
        match jh::change(&path, c, replace, false) {
            Ok(o) => {
                self.bindings = Bindings::load(&path);
                if let Some(w) = self.settings_win.as_mut() {
                    w.keys.refused = None;
                }
                // What the change means for other commands (a default it replaces).
                let notes: Vec<String> = o
                    .applied
                    .conflicts
                    .iter()
                    .map(|c| c.message.clone())
                    .collect();
                if !notes.is_empty() {
                    self.message = Some(notes.join(" "));
                }
                true
            }
            Err(e) => {
                let (action, keys) = match c {
                    Change::Set { action, keys } => (action.clone(), keys.join(" ")),
                    Change::Disable(a) | Change::Reset(a) => (a.clone(), String::new()),
                    Change::ResetAll => (String::new(), String::new()),
                };
                let takeable = e.code().0 == "hotkey_conflict";
                let msg = e.to_string();
                match self.settings_win.as_mut() {
                    Some(w) => w.keys.refused = Some((action, keys, msg, takeable)),
                    None => self.message = Some(msg),
                }
                false
            }
        }
    }

    /// While a command's keys are recorded: the next chord (Esc cancels).
    pub(crate) fn capture_shortcut(&mut self, ctx: &egui::Context) {
        let Some(action) = self
            .settings_win
            .as_ref()
            .and_then(|w| w.keys.capture.clone())
        else {
            return;
        };
        // Recording stops when the window loses the focus (HOTKEY_CONTRACT §11.2).
        if ctx.input(|i| !i.focused) {
            if let Some(w) = self.settings_win.as_mut() {
                w.keys.capture = None;
            }
            return;
        }
        let mut done: Option<Option<String>> = None;
        ctx.input_mut(|i| {
            i.events.retain(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if *key == Key::Escape && modifiers.is_none() {
                        done = Some(None);
                    } else if let Some(t) = chord_text(*key, *modifiers) {
                        done = Some(Some(t));
                    }
                    false
                }
                egui::Event::Text(_) => false,
                _ => true,
            });
        });
        let Some(result) = done else {
            return;
        };
        if let Some(w) = self.settings_win.as_mut() {
            w.keys.capture = None;
        }
        if let Some(keys) = result {
            self.change_shortcut(
                &Change::Set {
                    action,
                    keys: vec![keys],
                },
                false,
            );
        }
    }

    /// The Keyboard section.
    pub(crate) fn shortcut_mapper(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(path) = self.bindings.path.clone() else {
            kim_ui::notice(
                ui,
                kim_ui::Tone::Warning,
                lang.pick(
                    "This window has no shortcut file: shortcuts cannot be changed.",
                    "이 창에는 단축키 파일이 없어 단축키를 바꿀 수 없습니다.",
                ),
                None,
                None,
            );
            return;
        };
        if let Some(p) = self.bindings.problem.clone() {
            let m = format!(
                "{} {p}",
                lang.pick(
                    "hotkeys.json is not used (the defaults apply) and is not rewritten:",
                    "hotkeys.json을 쓰지 않고(기본값 적용) 다시 쓰지도 않습니다:"
                )
            );
            kim_ui::notice(ui, kim_ui::Tone::Warning, &m, None, None);
        }
        let (tool, _) = jh::load(&path);
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
        let capture = w.keys.capture.clone();
        let refused = w.keys.refused.clone();
        let confirm = w.keys.confirm_reset;
        ui.add_space(space::S);
        let names: std::collections::HashMap<&str, String> = crate::palette::entries()
            .into_iter()
            .map(|e| (e.id, lang.pick(&e.en, &e.ko).to_string()))
            .collect();
        let mut rows: Vec<(String, String, Vec<String>, bool)> = tool
            .manifest
            .actions
            .iter()
            .map(|a| {
                let (keys, changed) = jh::keys_of(&tool, &a.id);
                let name = names
                    .get(a.id.as_str())
                    .cloned()
                    .unwrap_or_else(|| a.title.clone());
                (a.id.clone(), name, keys, changed)
            })
            .filter(|(id, name, keys, _)| {
                filter.is_empty()
                    || name.to_lowercase().contains(&filter)
                    || id.contains(&filter)
                    || keys.iter().any(|k| k.to_lowercase().contains(&filter))
            })
            .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        let mut act: Option<(Change, bool)> = None;
        let mut start: Option<String> = None;
        let mut reset_all = None;
        // The user's changes first (they are what one looks for), then every command.
        let changed_rows: Vec<_> = rows.iter().filter(|r| r.3).cloned().collect();
        let groups = [
            (
                format!(
                    "{} ({})",
                    lang.pick("Your changes", "바꾼 단축키"),
                    changed_rows.len()
                ),
                changed_rows,
            ),
            (
                format!("{} ({})", lang.pick("Commands", "명령"), rows.len()),
                rows.clone(),
            ),
        ];
        for (title, rows) in groups.iter().filter(|(_, r)| !r.is_empty()) {
            kim_ui::setting_group(ui, title, |ui| {
                for (i, (id, name, keys, changed)) in rows.iter().enumerate() {
                    if i > 0 {
                        kim_ui::row_divider(ui);
                    }
                    kim_ui::setting_row(ui, name, None, |ui| {
                        if *changed
                            && ui
                                .add(kim_ui::Button::ghost(lang.pick("Reset", "기본값")).compact())
                                .on_hover_text(
                                    lang.pick("Back to Jarak's default", "자락 기본값으로"),
                                )
                                .clicked()
                        {
                            act = Some((Change::Reset(id.clone()), false));
                        }
                        if !keys.is_empty()
                            && ui
                                .add(kim_ui::Button::ghost(lang.pick("Off", "끄기")).compact())
                                .clicked()
                        {
                            act = Some((Change::Disable(id.clone()), false));
                        }
                        if ui
                            .add(kim_ui::Button::secondary(lang.pick("Change", "바꾸기")).compact())
                            .clicked()
                        {
                            start = Some(id.clone());
                        }
                        let label = if capture.as_deref() == Some(id.as_str()) {
                            lang.pick("Press keys… (Esc cancels)", "키를 누르세요… (Esc 취소)")
                                .to_string()
                        } else if keys.is_empty() {
                            lang.pick("Off", "꺼짐").to_string()
                        } else {
                            String::new()
                        };
                        if label.is_empty() {
                            let shown = keys.join(" ");
                            let a11y = format!("{name}: {shown}");
                            kim_ui::keycaps(ui, &shown).widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Label, true, &a11y)
                            });
                        } else {
                            ui.label(kim_ui::secondary(ui, label));
                        }
                    });
                    if let Some((a, k, msg, take)) = refused.as_ref().filter(|r| r.0 == *id) {
                        kim_ui::field_error(ui, msg);
                        if *take
                            && ui
                                .add(
                                    kim_ui::Button::ghost(lang.pick("Take it", "가져오기"))
                                        .compact(),
                                )
                                .on_hover_text(lang.pick(
                                    "Give these keys to this command (the other one loses them)",
                                    "이 키를 이 명령에 줍니다(다른 명령은 잃습니다)",
                                ))
                                .clicked()
                        {
                            act = Some((
                                Change::Set {
                                    action: a.clone(),
                                    keys: vec![k.clone()],
                                },
                                true,
                            ));
                        }
                    }
                }
            });
        }
        kim_ui::setting_group(ui, lang.pick("All shortcuts", "모든 단축키"), |ui| {
            kim_ui::setting_row(
                ui,
                lang.pick("Reset all shortcuts", "모든 단축키 초기화"),
                Some(lang.pick(
                    "Every command gets Jarak's default keys again (the previous file is kept as hotkeys.json.bak).",
                    "모든 명령이 자락 기본 키로 돌아갑니다(이전 파일은 hotkeys.json.bak으로 남습니다).",
                )),
                |ui| {
                    let b = if confirm {
                        kim_ui::Button::danger(lang.pick("Reset for good", "정말 초기화"))
                    } else {
                        kim_ui::Button::ghost(lang.pick("Reset all…", "모두 초기화…"))
                    };
                    if ui.add(b).clicked() {
                        reset_all = Some(confirm);
                    }
                },
            );
        });
        if let Some(id) = start
            && let Some(w) = self.settings_win.as_mut()
        {
            w.keys.capture = Some(id);
            w.keys.refused = None;
        }
        if let Some((c, replace)) = act {
            self.change_shortcut(&c, replace);
        }
        match reset_all {
            Some(true) => {
                self.change_shortcut(&Change::ResetAll, false);
                if let Some(w) = self.settings_win.as_mut() {
                    w.keys.confirm_reset = false;
                }
            }
            Some(false) => {
                if let Some(w) = self.settings_win.as_mut() {
                    w.keys.confirm_reset = true;
                }
            }
            None => {}
        }
    }
}

/// The commands' key labels the contract has no name for (empty: the manifest has every command;
/// Alt alone, which shows the menu bar, is not a command's chord).
pub fn unnamed_defaults() -> Vec<&'static str> {
    crate::palette::entries()
        .into_iter()
        .filter(|e| matches!(e.run, crate::palette::Run::Act(_)))
        .filter_map(|e| e.keys)
        .filter(|k| canonical(k).is_none())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_become_contract_chords_and_back() {
        assert_eq!(canonical("Ctrl+,").as_deref(), Some("Ctrl+Comma"));
        assert_eq!(canonical("Alt+Shift+0").as_deref(), Some("Alt+Shift+0"));
        assert_eq!(canonical("Esc").as_deref(), Some("Escape"));
        assert_eq!(canonical("Alt+Left").as_deref(), Some("Alt+Left"));
        assert_eq!(
            spec_chord("Ctrl+Shift+P"),
            Some((Key::P, true, false, true))
        );
        assert_eq!(spec_chord("Win+E"), None);
        assert!(unnamed_defaults().is_empty(), "{:?}", unnamed_defaults());
    }

    /// The shipped manifest (jarak-core) and the contract example are this build's commands (set
    /// `JARAK_WRITE_MANIFEST=1` to write them after a command or its keys changed).
    #[test]
    fn the_shipped_manifest_is_the_commands() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let shipped = root.join("crates/jarak-core/src/hotkeys.manifest.json");
        let example = root.join("docs/contracts/examples/kim-hotkey-manifest-v1/jarak.json");
        let want = manifest_json();
        let text = format!("{}\n", serde_json::to_string_pretty(&want).unwrap());
        if std::env::var("JARAK_WRITE_MANIFEST").is_ok_and(|v| v == "1") {
            std::fs::write(&shipped, &text).unwrap();
            std::fs::write(&example, &text).unwrap();
        }
        for p in [&shipped, &example] {
            let mut got: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
            // The contract example documents the commands; its version may lag the crate's
            // (`cargo xtask bump` keeps the shipped file's).
            if *p == example {
                got["version"] = want["version"].clone();
            }
            assert_eq!(
                got,
                want,
                "{} (regenerate with JARAK_WRITE_MANIFEST=1)",
                p.display()
            );
        }
    }
}
