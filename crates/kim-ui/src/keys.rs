//! Keyboard shortcuts in egui windows (HOTKEY_CONTRACT §5, ADR 0092, ADR 0100): egui's keys as
//! the contract's key names, and the **key picker**: a shortcut built by clicking keys instead of
//! pressing them (keys that are hard to press in a window, like `PrintScreen`, side-specific
//! modifiers or `CapsLock`).

use crate::{Button, Colors, ui_font};
use egui::{Align, Layout, RichText, Ui};
use kim_design::{font, size, space};

/// The contract's name of an egui key (`ArrowUp` → `Up`, `F5`, `A`, …); `None` for keys the
/// contract cannot bind.
pub fn key_name(k: egui::Key) -> Option<&'static str> {
    use egui::Key as K;
    let name = match k {
        K::ArrowUp => "Up",
        K::ArrowDown => "Down",
        K::ArrowLeft => "Left",
        K::ArrowRight => "Right",
        K::Num0 => "0",
        K::Num1 => "1",
        K::Num2 => "2",
        K::Num3 => "3",
        K::Num4 => "4",
        K::Num5 => "5",
        K::Num6 => "6",
        K::Num7 => "7",
        K::Num8 => "8",
        K::Num9 => "9",
        K::Plus | K::Equals => "Plus",
        K::Minus => "Minus",
        K::Comma => "Comma",
        K::Period => "Period",
        other => other.name(),
    };
    kim_hotkeys::keys::NAMED_KEYS
        .iter()
        .find(|n| **n == name)
        .copied()
}

/// `Ctrl+Shift+K` for a key and its modifiers (contract order: Ctrl, Alt, Shift, Win); `None`
/// for a key without a contract name.
pub fn chord_text(k: egui::Key, m: egui::Modifiers) -> Option<String> {
    let key = key_name(k)?;
    let mut parts: Vec<&str> = Vec::new();
    if m.ctrl {
        parts.push("Ctrl");
    }
    if m.alt {
        parts.push("Alt");
    }
    if m.shift {
        parts.push("Shift");
    }
    parts.push(key);
    Some(parts.join("+"))
}

/// The modifiers a picker offers: either-side ones, or side-specific ones as well.
pub fn modifiers(sides: bool) -> Vec<&'static str> {
    let mut v = vec!["Ctrl", "Alt", "Shift", "Win"];
    if sides {
        v.extend([
            "LCtrl", "RCtrl", "LAlt", "RAlt", "LShift", "RShift", "LWin", "RWin",
        ]);
    }
    v
}

/// The modifier family of a key name (`LCtrl` → `Ctrl`), `None` for other keys.
pub fn modifier_family(k: &str) -> Option<&'static str> {
    let base = if k.len() > 3 && (k.starts_with('L') || k.starts_with('R')) {
        &k[1..]
    } else {
        k
    };
    ["Ctrl", "Alt", "Shift", "Win"]
        .into_iter()
        .find(|f| *f == base)
}

/// ADR 0101: the punctuation keys a picker offers, as the symbol on the key and the key name
/// (with Shift for the upper symbol). The names follow US-style layouts, which Korean keyboards
/// share; the engine matches the key, so another layout types its own symbol on that key.
pub const SYMBOLS: [(&str, &str, bool); 14] = [
    (",", "Comma", false),
    (".", "Period", false),
    ("/", "code:Slash", false),
    ("<", "Comma", true),
    (">", "Period", true),
    ("?", "code:Slash", true),
    (";", "code:Semicolon", false),
    ("'", "code:Quote", false),
    ("[", "code:BracketLeft", false),
    ("]", "code:BracketRight", false),
    ("\\", "code:Backslash", false),
    ("`", "code:Backquote", false),
    ("-", "Minus", false),
    ("=", "Plus", false),
];

/// ADR 0101: the key and Shift of a symbol (`<` → `Comma` with Shift).
pub fn symbol_key(sym: &str) -> Option<(&'static str, bool)> {
    SYMBOLS
        .iter()
        .find(|(s, _, _)| *s == sym)
        .map(|(_, k, shift)| (*k, *shift))
}

/// ADR 0101: the symbol a key types (`code:Slash` → `/`; with `shift`, `Comma` → `<`).
pub fn key_symbol(key: &str, shift: bool) -> Option<&'static str> {
    SYMBOLS
        .iter()
        .find(|(_, k, sh)| *k == key && *sh == shift)
        .map(|(s, _, _)| *s)
}

/// ADR 0101: a chord as the keys people see: `Shift+Comma` is `<`, `code:Slash` is `/`,
/// `CapsLock+Shift+Comma` is `CapsLock`, `<`. A side-specific Shift (`RShift+Comma`) stays
/// written out, because only one Shift fires it.
pub fn display_parts(chord: &str) -> Vec<String> {
    let parts = crate::split_chord(chord);
    let Some((key, mods)) = parts.split_last() else {
        return parts;
    };
    if mods.iter().any(|m| m == "Shift")
        && let Some(sym) = key_symbol(key, true)
    {
        let mut out: Vec<String> = mods.iter().filter(|m| *m != "Shift").cloned().collect();
        out.push(sym.to_string());
        return out;
    }
    let mut out = mods.to_vec();
    out.push(key_symbol(key, false).map_or_else(|| key.clone(), str::to_string));
    out
}

/// A key name for people: `LCtrl` → `Left Ctrl`, `Up` → `Up arrow` (`ko`: `왼쪽 Ctrl`).
pub fn readable_key(k: &str, lang: kim_design::Lang) -> String {
    if let Some(sym) =
        key_symbol(k, false).or_else(|| SYMBOLS.iter().find(|x| x.0 == k).map(|x| x.0))
    {
        return sym.to_string();
    }
    if let Some(f) = modifier_family(k)
        && k.len() > f.len()
    {
        let side = if k.starts_with('L') {
            lang.pick("Left", "왼쪽")
        } else {
            lang.pick("Right", "오른쪽")
        };
        return format!("{side} {f}");
    }
    match k {
        "Up" | "Down" | "Left" | "Right" => format!("{k} {}", lang.pick("arrow", "화살표")),
        "MouseX1" => lang.pick("Mouse back button", "마우스 뒤로 버튼").into(),
        "MouseX2" => lang
            .pick("Mouse forward button", "마우스 앞으로 버튼")
            .into(),
        "MouseMiddle" => lang
            .pick("Middle mouse button", "마우스 가운데 버튼")
            .into(),
        other => other.to_string(),
    }
}

/// A chord for people: `LCtrl+Space` → `Left Ctrl + Space`, `CapsLock+Shift+Comma` →
/// `CapsLock + <`.
pub fn readable_chord(chord: &str, lang: kim_design::Lang) -> String {
    display_parts(chord)
        .iter()
        .map(|k| readable_key(k, lang))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The keys every keyboard has and triggers often use (shown first, after the modifiers).
const COMMON: [&str; 11] = [
    "CapsLock",
    "Tab",
    "Space",
    "Enter",
    "Escape",
    "Backspace",
    "Delete",
    "Up",
    "Down",
    "Left",
    "Right",
];
const NAVIGATION: [&str; 15] = [
    "Insert",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    "PrintScreen",
    "Pause",
    "ScrollLock",
    "NumLock",
    "ContextMenu",
];

/// The height of the key grid: four rows of keys (switching category never resizes the dialog
/// under the pointer).
pub const GRID_HEIGHT: f32 = 4.0 * size::KEY + 3.0 * space::XS;

/// The key picker's state: the chosen modifiers and key, the open category and the search.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyPicker {
    pub mods: Vec<&'static str>,
    pub key: Option<String>,
    /// ADR 0101: CapsLock is chosen. With another key it is held as a modifier
    /// (`CapsLock+W`); alone it is the key (`RCtrl+CapsLock`).
    pub caps: bool,
    /// ADR 0101: the generic Shift was added by a shifted symbol (`<`), so another symbol
    /// takes it away again.
    pub sym_shift: bool,
    pub search: String,
    /// ADR 0100: the open category (an index of [`KeyPicker::categories`]).
    pub tab: usize,
}

/// What the picker's buttons did this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    None,
    /// `Confirm`: the chord text (`Ctrl+PrintScreen`).
    Confirm(String),
    Cancel,
}

/// One category of keys: its name and keys.
pub type Category = (String, Vec<&'static str>);

impl KeyPicker {
    /// The chord built so far (`RCtrl+CapsLock`); modifiers alone (`RAlt`) are a chord too.
    /// `None` when nothing is chosen.
    pub fn chord(&self) -> Option<String> {
        let mut parts: Vec<&str> = self.mods.clone();
        match (&self.key, self.caps) {
            (Some(k), true) => {
                parts.insert(0, "CapsLock");
                parts.push(k);
            }
            (Some(k), false) => parts.push(k),
            (None, true) => parts.push("CapsLock"),
            (None, false) => {}
        }
        (!parts.is_empty()).then(|| parts.join("+"))
    }

    /// ADR 0101: a picker that shows `chord` (`CapsLock+Shift+Comma`, `RCtrl+Space`); parts the
    /// picker does not offer are left out.
    pub fn from_chord(chord: &str, keys: &[&'static str]) -> KeyPicker {
        let mut p = KeyPicker::default();
        for part in crate::split_chord(chord.trim()) {
            if part == "CapsLock" {
                p.caps = true;
            } else if let Some(m) = modifiers(true).into_iter().find(|m| *m == part) {
                p.toggle(m);
            } else if let Some(k) = keys
                .iter()
                .copied()
                .chain(SYMBOLS.iter().map(|s| s.1))
                .find(|k| *k == part)
            {
                p.key = Some(k.to_string());
            }
        }
        p
    }

    /// The categories of `keys`, in tab order: the common keys with the (left/right, when
    /// `sides`) modifiers, all modifiers, F1–F24, letters and digits, navigation, the mouse
    /// buttons and the rest. Empty categories are left out.
    pub fn categories(keys: &[&'static str], sides: bool, lang: kim_design::Lang) -> Vec<Category> {
        let has = |k: &str| keys.contains(&k);
        let mods = modifiers(sides);
        let sided: Vec<&'static str> = if sides {
            mods.iter()
                .copied()
                .filter(|m| modifier_family(m).is_some_and(|f| m.len() > f.len()))
                .collect()
        } else {
            mods.clone()
        };
        let mut common = sided;
        common.extend(COMMON.iter().copied().filter(|k| has(k)));
        let fkeys: Vec<&'static str> = keys
            .iter()
            .copied()
            .filter(|k| k.len() >= 2 && k.starts_with('F') && k[1..].parse::<u8>().is_ok())
            .collect();
        let letters: Vec<&'static str> = keys
            .iter()
            .copied()
            .filter(|k| k.len() == 1 && k.chars().all(|c| c.is_ascii_alphanumeric()))
            .collect();
        let nav: Vec<&'static str> = NAVIGATION.iter().copied().filter(|k| has(k)).collect();
        let mouse: Vec<&'static str> = keys
            .iter()
            .copied()
            .filter(|k| k.starts_with("Mouse"))
            .collect();
        let placed: Vec<&'static str> = common
            .iter()
            .chain(&fkeys)
            .chain(&letters)
            .chain(&nav)
            .chain(&mouse)
            .copied()
            .collect();
        let more: Vec<&'static str> = keys
            .iter()
            .copied()
            .filter(|k| {
                !placed.contains(k)
                    && modifier_family(k).is_none()
                    && key_symbol(k, false).is_none()
            })
            .collect();
        let mut out = vec![
            (lang.pick("Common", "자주 쓰는 키").to_string(), common),
            (lang.pick("Modifiers", "보조 키").to_string(), mods),
            ("F1–F24".to_string(), fkeys),
            (lang.pick("A–Z, 0–9", "문자·숫자").to_string(), letters),
            (
                lang.pick("Symbols", "기호").to_string(),
                SYMBOLS.iter().map(|s| s.0).collect(),
            ),
            (lang.pick("Navigation", "이동").to_string(), nav),
            (lang.pick("Mouse", "마우스").to_string(), mouse),
            (lang.pick("More", "기타").to_string(), more),
        ];
        out.retain(|(_, v)| !v.is_empty());
        out
    }

    /// Clicks `k`. A modifier is added or removed: an either-side `Ctrl` replaces `LCtrl` and
    /// `RCtrl` and the other way round, while `LShift` and `RShift` may both be chosen (the
    /// program's check says whether it can use that). Any other key becomes the chord's key; a
    /// second click removes it.
    pub fn toggle(&mut self, k: &'static str) {
        if k == "CapsLock" {
            self.caps = !self.caps;
            return;
        }
        if let Some((key, shift)) = symbol_key(k) {
            let same = self.key.as_deref() == Some(key)
                && (shift == self.mods.contains(&"Shift") || !shift && self.sym_shift);
            if self.sym_shift {
                self.mods.retain(|m| *m != "Shift");
                self.sym_shift = false;
            }
            if same {
                self.key = None;
                return;
            }
            self.key = Some(key.to_string());
            if shift
                && !self
                    .mods
                    .iter()
                    .any(|m| modifier_family(m) == Some("Shift"))
            {
                self.toggle("Shift");
                self.sym_shift = true;
            }
            return;
        }
        if let Some(fam) = modifier_family(k) {
            if fam == "Shift" {
                self.sym_shift = false;
            }
            if self.mods.contains(&k) {
                self.mods.retain(|x| *x != k);
                return;
            }
            let either = k.len() == fam.len();
            self.mods
                .retain(|x| modifier_family(x) != Some(fam) || (!either && x.len() > fam.len()));
            self.mods.push(k);
            self.mods.sort_by_key(|x| {
                (
                    ["Ctrl", "Alt", "Shift", "Win"]
                        .iter()
                        .position(|f| Some(*f) == modifier_family(x))
                        .unwrap_or(9),
                    !x.starts_with('L'),
                )
            });
        } else if self.key.as_deref() == Some(k) {
            self.key = None;
        } else {
            self.key = Some(k.to_string());
        }
    }

    /// Removes one part of the chord (a modifier or the key).
    pub fn remove(&mut self, part: &str) {
        if part == "CapsLock" {
            self.caps = false;
        }
        self.mods.retain(|m| *m != part);
        if part == "Shift" {
            self.sym_shift = false;
        }
        if self.key.as_deref() == Some(part) {
            self.key = None;
            if self.sym_shift {
                self.mods.retain(|m| *m != "Shift");
                self.sym_shift = false;
            }
        }
    }

    /// Whether key button `k` shows as chosen.
    fn is_on(&self, k: &str) -> bool {
        if k == "CapsLock" {
            return self.caps;
        }
        if let Some((key, shift)) = symbol_key(k) {
            return self.key.as_deref() == Some(key) && shift == self.mods.contains(&"Shift");
        }
        self.mods.contains(&k) || self.key.as_deref() == Some(k)
    }

    /// Draws the picker without a check (every chord can be confirmed).
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        keys: &[&'static str],
        sides: bool,
        lang: kim_design::Lang,
    ) -> Picked {
        self.ui_checked(ui, keys, sides, lang, &|_| None)
    }

    /// ADR 0100: draws the picker: the chosen keys as removable chips with the chord in words,
    /// the program's verdict (`check` returns why a chord cannot be used, so Confirm never
    /// accepts one the program cannot run), the categories, a search across all keys, and
    /// Clear / Cancel / Confirm. Keys are buttons of the control height; Tab moves through them
    /// and Space or Enter presses one.
    pub fn ui_checked(
        &mut self,
        ui: &mut Ui,
        keys: &[&'static str],
        sides: bool,
        lang: kim_design::Lang,
        check: &dyn Fn(&str) -> Option<String>,
    ) -> Picked {
        let c = Colors::get(ui.ctx());
        let mut out = Picked::None;
        let chord = self.chord();
        // The selection: chips that remove their key, and the chord in words.
        let sel = egui::Frame::new()
            .fill(c.bg_base)
            .stroke(egui::Stroke::new(size::BORDER, c.border_subtle))
            .corner_radius(egui::CornerRadius::same(kim_design::radius::M as u8))
            .inner_margin(egui::Margin::symmetric(space::M as i8, space::S as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(size::CONTROL + 2.0 * space::M);
                let mut parts: Vec<String> = Vec::new();
                if self.caps {
                    parts.push("CapsLock".into());
                }
                parts.extend(self.mods.iter().map(|m| (*m).to_string()));
                parts.extend(self.key.clone());
                if parts.is_empty() {
                    ui.add_space(space::S);
                    ui.label(
                        RichText::new(lang.pick(
                            "Click keys below to build the combination.",
                            "아래 키를 눌러 조합을 만드세요.",
                        ))
                        .color(c.text_muted),
                    );
                    return;
                }
                let mut remove = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = space::XS;
                    for (i, p) in parts.iter().enumerate() {
                        if i > 0 {
                            ui.label(RichText::new("+").color(c.text_muted));
                        }
                        let shown = key_symbol(p, false).unwrap_or(p);
                        if ui
                            .add(
                                Button::secondary(format!("{shown}  ×"))
                                    .compact()
                                    .min_width(0.0),
                            )
                            .on_hover_text(lang.pick("Remove this key", "이 키 빼기"))
                            .clicked()
                        {
                            remove = Some(p.clone());
                        }
                    }
                });
                if let Some(ch) = &chord {
                    ui.label(
                        RichText::new(readable_chord(ch, lang))
                            .font(ui_font(ui.ctx(), font::CAPTION, false))
                            .color(c.text_secondary),
                    );
                }
                if let Some(p) = remove {
                    self.remove(&p);
                }
            });
        crate::inspect::record(
            ui,
            "key-picker:selection",
            "region",
            sel.response.rect,
            None,
            false,
            &[],
            None,
            Some(c.bg_base),
        );
        ui.add_space(space::XS);
        let verdict = chord.as_deref().and_then(check);
        let caption = |ui: &mut Ui, text: String, col| {
            ui.label(
                RichText::new(text)
                    .font(ui_font(ui.ctx(), font::CAPTION, false))
                    .color(col),
            );
        };
        match (&chord, verdict) {
            (Some(_), Some(why)) => {
                crate::field_error(ui, &why);
            }
            (Some(_), None) => caption(
                ui,
                lang.pick("This combination can be used.", "이 조합을 쓸 수 있습니다.")
                    .into(),
                c.success,
            ),
            _ => caption(
                ui,
                lang.pick(
                    "Left and right modifiers are different keys; Ctrl means either side.",
                    "왼쪽과 오른쪽 보조 키는 서로 다른 키입니다. Ctrl은 양쪽 모두를 뜻합니다.",
                )
                .into(),
                c.text_muted,
            ),
        }
        ui.add_space(space::S);
        // The categories, the search, the keys.
        let cats = Self::categories(keys, sides, lang);
        self.tab = self.tab.min(cats.len().saturating_sub(1));
        let names: Vec<(usize, &str)> = cats
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (i, n.as_str()))
            .collect();
        egui::ScrollArea::horizontal()
            .id_salt("kim-ui-key-picker-tabs")
            .show(ui, |ui| {
                crate::segmented(ui, &mut self.tab, &names);
            });
        ui.add_space(space::XS);
        crate::search_field(
            ui,
            &mut self.search,
            lang.pick("Search keys", "키 검색"),
            lang.pick(
                "Search all keys (e.g. Print, Caps, F13)",
                "모든 키 검색 (예: Print, Caps, F13)",
            ),
            ui.available_width(),
        );
        ui.add_space(space::XS);
        let q = self.search.trim().to_lowercase();
        let shown: Vec<&'static str> = if q.is_empty() {
            cats.get(self.tab).map(|c| c.1.clone()).unwrap_or_default()
        } else {
            let mut all = modifiers(sides);
            all.extend(
                keys.iter()
                    .copied()
                    .filter(|k| modifier_family(k).is_none()),
            );
            all.extend(SYMBOLS.iter().map(|s| s.0));
            all.into_iter()
                .filter(|k| {
                    k.to_lowercase().contains(&q)
                        || readable_key(k, lang).to_lowercase().contains(&q)
                })
                .collect()
        };
        let mut click = None;
        egui::ScrollArea::vertical()
            .id_salt("kim-ui-key-picker")
            .min_scrolled_height(GRID_HEIGHT)
            .max_height(GRID_HEIGHT)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(space::XS, space::XS);
                    for k in &shown {
                        let on = self.is_on(k);
                        if ui
                            .add(
                                Button::secondary(*k)
                                    .min_width(size::KEY_MIN_WIDTH)
                                    .selected(on),
                            )
                            .on_hover_text(readable_key(k, lang))
                            .clicked()
                        {
                            click = Some(*k);
                        }
                    }
                    if shown.is_empty() {
                        ui.label(
                            RichText::new(lang.pick("No key matches.", "맞는 키가 없습니다."))
                                .color(c.text_muted),
                        );
                    }
                });
            });
        if let Some(k) = click {
            self.toggle(k);
        }
        crate::dialog_actions(ui, |ui| {
            let ok = self.chord().filter(|ch| check(ch).is_none());
            if ui
                .add_enabled(ok.is_some(), Button::primary("Confirm"))
                .clicked()
                && let Some(t) = ok
            {
                out = Picked::Confirm(t);
            }
            if ui.add(Button::secondary("Cancel")).clicked() {
                out = Picked::Cancel;
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_enabled(self.chord().is_some(), Button::ghost("Clear"))
                    .clicked()
                {
                    self.mods.clear();
                    self.key = None;
                    self.caps = false;
                    self.sym_shift = false;
                }
            });
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kim_design::Lang;

    #[test]
    fn egui_keys_get_contract_names() {
        assert_eq!(key_name(egui::Key::ArrowUp), Some("Up"));
        assert_eq!(key_name(egui::Key::F5), Some("F5"));
        assert_eq!(key_name(egui::Key::A), Some("A"));
        assert_eq!(
            chord_text(egui::Key::K, egui::Modifiers::CTRL | egui::Modifiers::SHIFT).as_deref(),
            Some("Ctrl+Shift+K")
        );
    }

    #[test]
    fn the_picker_builds_side_specific_chords() {
        let mut p = KeyPicker::default();
        assert_eq!(p.chord(), None);
        p.toggle("RCtrl");
        p.toggle("CapsLock");
        assert_eq!(p.chord().as_deref(), Some("RCtrl+CapsLock"));
        assert!(modifiers(true).contains(&"LShift"));
        assert!(!modifiers(false).contains(&"LShift"));
    }

    /// ADR 0100: several modifiers, left and right apart, either-side replacing a side, keys
    /// removed one by one, a bare modifier as a chord.
    #[test]
    fn modifiers_combine_and_parts_are_removed_one_by_one() {
        let mut p = KeyPicker::default();
        for k in ["A", "RShift", "RCtrl"] {
            p.toggle(k);
        }
        assert_eq!(p.chord().as_deref(), Some("RCtrl+RShift+A"));
        p.toggle("LShift");
        assert_eq!(p.chord().as_deref(), Some("RCtrl+LShift+RShift+A"));
        p.toggle("Shift");
        assert_eq!(
            p.chord().as_deref(),
            Some("RCtrl+Shift+A"),
            "either side replaces both"
        );
        p.remove("A");
        p.remove("Shift");
        assert_eq!(
            p.chord().as_deref(),
            Some("RCtrl"),
            "a bare modifier is a chord"
        );
        p.toggle("RCtrl");
        assert_eq!(p.chord(), None);
        p.toggle("Space");
        p.toggle("Tab");
        assert_eq!(
            p.chord().as_deref(),
            Some("Tab"),
            "one key: a new key replaces it"
        );
    }

    /// ADR 0101: CapsLock with another key is a modifier (`CapsLock+W`), alone it is the key;
    /// `<` is Shift and Comma, shown as `<`; another symbol takes the added Shift away again.
    #[test]
    fn capslock_combines_and_symbols_carry_their_shift() {
        let mut p = KeyPicker::default();
        p.toggle("CapsLock");
        assert_eq!(p.chord().as_deref(), Some("CapsLock"));
        p.toggle("W");
        assert_eq!(p.chord().as_deref(), Some("CapsLock+W"));
        p.toggle("<");
        assert_eq!(p.chord().as_deref(), Some("CapsLock+Shift+Comma"));
        assert!(p.is_on("<") && !p.is_on(","));
        assert_eq!(
            readable_chord("CapsLock+Shift+Comma", Lang::En),
            "CapsLock + <"
        );
        p.toggle("/");
        assert_eq!(
            p.chord().as_deref(),
            Some("CapsLock+code:Slash"),
            "the added Shift goes"
        );
        assert_eq!(
            readable_chord("CapsLock+code:Slash", Lang::Ko),
            "CapsLock + /"
        );
        p.toggle(">");
        assert_eq!(p.chord().as_deref(), Some("CapsLock+Shift+Period"));
        p.remove("Period");
        assert_eq!(
            p.chord().as_deref(),
            Some("CapsLock"),
            "its Shift went with it"
        );
        p.toggle("Backspace");
        assert_eq!(p.chord().as_deref(), Some("CapsLock+Backspace"));
        p.remove("CapsLock");
        assert_eq!(p.chord().as_deref(), Some("Backspace"));
        let mut r = KeyPicker::default();
        r.toggle("RShift");
        r.toggle("Enter");
        assert_eq!(r.chord().as_deref(), Some("RShift+Enter"));
        assert_eq!(
            readable_chord("RShift+Enter", Lang::En),
            "Right Shift + Enter"
        );
        assert_eq!(display_parts("RShift+Comma"), ["RShift", ","]);
        let keys = ["W", "Enter", "Backspace"];
        let back = KeyPicker::from_chord("CapsLock+Shift+Comma", &keys);
        assert_eq!(back.chord().as_deref(), Some("CapsLock+Shift+Comma"));
        assert!(back.is_on("<"));
    }

    #[test]
    fn categories_put_capslock_and_the_side_modifiers_first() {
        let keys = [
            "CapsLock", "Tab", "F1", "F13", "A", "1", "Home", "MouseX1", "VolumeUp",
        ];
        let cats = KeyPicker::categories(&keys, true, Lang::En);
        let common = &cats[0].1;
        for k in [
            "LCtrl", "RCtrl", "LShift", "RShift", "LAlt", "RAlt", "CapsLock", "Tab",
        ] {
            assert!(common.contains(&k), "{k} in {common:?}");
        }
        assert!(
            !common.contains(&"Ctrl"),
            "either-side modifiers live under Modifiers"
        );
        let names: Vec<&str> = cats.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(
            names,
            [
                "Common",
                "Modifiers",
                "F1–F24",
                "A–Z, 0–9",
                "Symbols",
                "Navigation",
                "Mouse",
                "More"
            ]
        );
        assert_eq!(cats[7].1, ["VolumeUp"]);
        assert_eq!(readable_chord("LCtrl+Space", Lang::En), "Left Ctrl + Space");
        assert_eq!(readable_chord("RAlt+Tab", Lang::Ko), "오른쪽 Alt + Tab");
    }
}
