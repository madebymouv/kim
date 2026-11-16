//! Key capture for the trigger and output fields (docs/DDALGAK_GUI_JOURNEYS.md J3,
//! docs/HOTKEY_CONTRACT.md §11.2).
//!
//! The capture box records chords on key-down from the window's own keyboard events: nothing is
//! registered and no hook is installed. `Esc` cancels, `Tab` leaves the box (focus is never
//! trapped), **Add step** records the next chord of a sequence (up to four), and the captured
//! keys are announced in words ("Control plus Shift plus F2"). During an IME composition the box
//! records nothing and asks for English input or typed keys (HOTKEY_CONTRACT §6.2).
//!
//! The window only sees what Windows passes on: keys Windows or another program takes first
//! (Win chords, a global shortcut), lock keys and bare modifiers (CapsLock, Right Alt) are
//! entered with **Type instead** or the special-key chips. ADR 0101: the window does not say
//! which Shift, Ctrl or Alt is held, so the box asks Windows (`GetKeyState`) for the side when
//! a key arrives: a right-side modifier held alone is recorded as `RShift`, `RCtrl` or `RAlt`,
//! the left side as the generic `Shift`/`Ctrl`/`Alt` (what people usually mean); a held CapsLock
//! makes the chord `CapsLock+…`. When Ddalgak itself owns CapsLock (a CapsLock rule is active),
//! Windows never sees it held, so such chords are picked by clicking.

use eframe::egui;

/// Keys the window cannot capture (lock keys, bare and side-specific modifiers), offered as chips.
pub const SPECIAL_KEYS: [&str; 10] = [
    "CapsLock",
    "RAlt",
    "RCtrl",
    "LCtrl",
    "RShift",
    "ScrollLock",
    "Pause",
    "ContextMenu",
    "NumLock",
    "Insert",
];

/// The longest sequence the capture records.
pub const MAX_STEPS: usize = 4;

/// A contract key name for an egui key (`None`: not a key a trigger can use).
pub fn key_name(k: egui::Key) -> Option<&'static str> {
    use egui::Key as K;
    Some(match k {
        K::ArrowUp => "Up",
        K::ArrowDown => "Down",
        K::ArrowLeft => "Left",
        K::ArrowRight => "Right",
        K::Escape => "Escape",
        K::Tab => "Tab",
        K::Backspace => "Backspace",
        K::Enter => "Enter",
        K::Space => "Space",
        K::Insert => "Insert",
        K::Delete => "Delete",
        K::Home => "Home",
        K::End => "End",
        K::PageUp => "PageUp",
        K::PageDown => "PageDown",
        K::Minus => "Minus",
        K::Plus | K::Equals => "Plus",
        K::Comma => "Comma",
        K::Period => "Period",
        // ADR 0101: punctuation without a logical name is the key at that position.
        K::Slash => "code:Slash",
        K::Semicolon => "code:Semicolon",
        K::Quote => "code:Quote",
        K::OpenBracket => "code:BracketLeft",
        K::CloseBracket => "code:BracketRight",
        K::Backslash => "code:Backslash",
        K::Backtick => "code:Backquote",
        _ => {
            let n = k.name();
            return kim_hotkeys::keys::NAMED_KEYS
                .iter()
                .copied()
                .find(|x| *x == n);
        }
    })
}

/// The canonical chord text for a key press (`Ctrl+Shift+F2`); `None` for keys a trigger cannot
/// use. The `Win` key never reaches the window as a modifier, so it is never part of a chord.
pub fn chord(key: egui::Key, m: egui::Modifiers) -> Option<String> {
    let name = key_name(key)?;
    let mut s = String::new();
    if m.ctrl || m.command && !m.mac_cmd {
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

/// The keys in words, for screen readers and the field's caption ("Control plus Shift plus F2,
/// then K").
pub fn announce(text: &str) -> String {
    text.split_whitespace()
        .map(|chord| {
            chord
                .split('+')
                .map(|k| match k {
                    "Ctrl" => "Control",
                    "LCtrl" => "Left Control",
                    "RCtrl" => "Right Control",
                    "RAlt" => "Right Alt",
                    "LAlt" => "Left Alt",
                    "RShift" => "Right Shift",
                    "LShift" => "Left Shift",
                    "Esc" => "Escape",
                    "Up" => "Up arrow",
                    "Down" => "Down arrow",
                    "Left" => "Left arrow",
                    "Right" => "Right arrow",
                    other => other,
                })
                .collect::<Vec<_>>()
                .join(" plus ")
        })
        .collect::<Vec<_>>()
        .join(", then ")
}

/// ADR 0101: what Windows says is held when a key arrives (the window only reports generic
/// modifiers).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Held {
    pub caps: bool,
    pub lshift: bool,
    pub rshift: bool,
    pub lctrl: bool,
    pub rctrl: bool,
    pub lalt: bool,
    pub ralt: bool,
}

impl Held {
    /// The state as this thread's input sees it now (Windows; nothing elsewhere).
    pub fn now() -> Held {
        #[cfg(windows)]
        {
            let d = ddalgak::win::inject::key_down;
            Held {
                caps: d(0x14),
                lshift: d(0xA0),
                rshift: d(0xA1),
                lctrl: d(0xA2),
                rctrl: d(0xA3),
                lalt: d(0xA4),
                ralt: d(0xA5),
            }
        }
        #[cfg(not(windows))]
        Held::default()
    }

    /// `Ctrl+Shift+K` with the sides and CapsLock applied: a right-side modifier held without
    /// its left twin is written with its side.
    pub fn refine(self, chord: &str) -> String {
        let mut parts: Vec<String> = chord.split('+').map(str::to_string).collect();
        for p in &mut parts {
            let side = match p.as_str() {
                "Shift" if self.rshift && !self.lshift => "RShift",
                "Ctrl" if self.rctrl && !self.lctrl && !self.ralt => "RCtrl",
                "Alt" if self.ralt && !self.lalt => "RAlt",
                _ => continue,
            };
            *p = side.to_string();
        }
        let out = parts.join("+");
        if self.caps {
            format!("CapsLock+{out}")
        } else {
            out
        }
    }
}

/// What one key press in the capture box did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Captured {
    /// The chord was recorded; the text is the whole value now.
    Keys(String),
    /// `Esc`: capture ended, the previous value stays.
    Cancelled,
    /// `Tab`: focus moves on, nothing recorded.
    Leave,
    /// A key that cannot be part of a trigger (or an IME composition): nothing recorded.
    Ignored,
}

/// The capture box's state.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Capture {
    /// Recording (the box has focus and was started).
    pub active: bool,
    /// Add the next chord to the sequence instead of replacing it.
    pub add_step: bool,
    pub composing: bool,
}

impl Capture {
    /// Feeds one key-down; `current` is the field's value before it.
    pub fn press(&mut self, current: &str, key: egui::Key, m: egui::Modifiers) -> Captured {
        self.press_held(current, key, m, Held::default())
    }

    /// ADR 0101: [`Capture::press`] with what Windows says is held (sides, CapsLock).
    pub fn press_held(
        &mut self,
        current: &str,
        key: egui::Key,
        m: egui::Modifiers,
        held: Held,
    ) -> Captured {
        if self.composing {
            return Captured::Ignored;
        }
        if key == egui::Key::Escape && !m.any() {
            self.active = false;
            return Captured::Cancelled;
        }
        if key == egui::Key::Tab && !m.ctrl && !m.alt {
            self.active = false;
            return Captured::Leave;
        }
        let Some(c) = chord(key, m).map(|c| held.refine(&c)) else {
            return Captured::Ignored;
        };
        let steps: Vec<&str> = current.split_whitespace().collect();
        let value = if self.add_step && !steps.is_empty() && steps.len() < MAX_STEPS {
            format!("{current} {c}")
        } else {
            c
        };
        self.add_step = false;
        Captured::Keys(value)
    }

    /// Feeds the frame's events (key-downs and IME composition) while the box records.
    pub fn handle(&mut self, current: &str, events: &[egui::Event]) -> Vec<Captured> {
        let mut out = Vec::new();
        for e in events {
            match e {
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    self.composing = !text.is_empty();
                }
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => {
                    self.composing = false;
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if self.active => {
                    let cur = match out.last() {
                        Some(Captured::Keys(k)) => k.clone(),
                        _ => current.to_string(),
                    };
                    out.push(self.press_held(&cur, *key, *modifiers, Held::now()));
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    /// ADR 0101: Right Shift + Enter, CapsLock + W and CapsLock + / are recorded as such; the
    /// left Shift stays the generic `Shift`.
    #[test]
    fn sides_and_capslock_come_from_what_windows_says_is_held() {
        let mut c = Capture {
            active: true,
            ..Capture::default()
        };
        let right = Held {
            rshift: true,
            ..Held::default()
        };
        assert_eq!(
            c.press_held("", Key::Enter, Modifiers::SHIFT, right),
            Captured::Keys("RShift+Enter".into())
        );
        let left = Held {
            lshift: true,
            ..Held::default()
        };
        assert_eq!(
            c.press_held("", Key::Enter, Modifiers::SHIFT, left),
            Captured::Keys("Shift+Enter".into())
        );
        let caps = Held {
            caps: true,
            ..Held::default()
        };
        assert_eq!(
            c.press_held("", Key::W, Modifiers::NONE, caps),
            Captured::Keys("CapsLock+W".into())
        );
        assert_eq!(
            c.press_held("", Key::Slash, Modifiers::NONE, caps),
            Captured::Keys("CapsLock+code:Slash".into())
        );
        assert_eq!(
            c.press_held(
                "",
                Key::Comma,
                Modifiers::SHIFT,
                Held {
                    caps: true,
                    lshift: true,
                    ..Held::default()
                }
            ),
            Captured::Keys("CapsLock+Shift+Comma".into())
        );
        assert!(ddalgak::trigger::Trigger::parse("CapsLock+code:Slash").is_ok());
    }

    #[test]
    fn chords_use_the_contract_names_and_validate_as_triggers() {
        let cs = Modifiers::CTRL | Modifiers::SHIFT;
        assert_eq!(chord(Key::F2, cs).as_deref(), Some("Ctrl+Shift+F2"));
        assert_eq!(
            chord(Key::ArrowUp, Modifiers::ALT).as_deref(),
            Some("Alt+Up")
        );
        assert_eq!(chord(Key::F13, Modifiers::NONE).as_deref(), Some("F13"));
        assert_eq!(chord(Key::J, Modifiers::CTRL).as_deref(), Some("Ctrl+J"));
        assert_eq!(
            chord(Key::F35, Modifiers::NONE),
            None,
            "F25 and above are not contract keys"
        );
        for k in [
            Key::F2,
            Key::J,
            Key::ArrowUp,
            Key::Insert,
            Key::Num7,
            Key::PageDown,
        ] {
            let c = chord(k, Modifiers::CTRL | Modifiers::ALT).unwrap();
            assert!(ddalgak::edit::canonical_trigger(&c).is_ok(), "{c}");
        }
        for s in SPECIAL_KEYS {
            assert!(ddalgak::edit::canonical_trigger(s).is_ok(), "{s}");
        }
    }

    #[test]
    fn capture_records_sequences_and_never_traps_focus() {
        let mut c = Capture {
            active: true,
            ..Capture::default()
        };
        assert_eq!(
            c.press("", Key::K, Modifiers::CTRL),
            Captured::Keys("Ctrl+K".into())
        );
        c.add_step = true;
        assert_eq!(
            c.press("Ctrl+K", Key::U, Modifiers::CTRL),
            Captured::Keys("Ctrl+K Ctrl+U".into())
        );
        // Without "add step" a new chord replaces the value.
        assert_eq!(
            c.press("Ctrl+K Ctrl+U", Key::F5, Modifiers::NONE),
            Captured::Keys("F5".into())
        );
        assert_eq!(c.press("F5", Key::Tab, Modifiers::NONE), Captured::Leave);
        assert!(!c.active);
        c.active = true;
        assert_eq!(
            c.press("F5", Key::Escape, Modifiers::NONE),
            Captured::Cancelled
        );
        assert!(!c.active);
        // An IME composition records nothing.
        c.active = true;
        let ev = [
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "ㅎ".into(),
                active_range_chars: None,
            }),
            egui::Event::Key {
                key: Key::G,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ];
        assert_eq!(c.handle("F5", &ev), [Captured::Ignored]);
    }

    #[test]
    fn captured_keys_are_announced_in_words() {
        assert_eq!(announce("Ctrl+Shift+F2"), "Control plus Shift plus F2");
        assert_eq!(
            announce("Ctrl+K Ctrl+U"),
            "Control plus K, then Control plus U"
        );
        assert_eq!(announce("RAlt"), "Right Alt");
    }
}
