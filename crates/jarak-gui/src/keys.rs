//! Keys to actions (`JARAK-FR-031`): the standard Windows editing keys. Every action is a
//! registry command or a window action; Step 105 makes them configurable (`hotkeys.json`).
//! Copy, Cut and Paste (Ctrl+C/X/V, Ctrl+Insert, Shift+Delete, Shift+Insert) are keys here too:
//! eframe's own clipboard shortcuts are off and Jarak uses the clipboard itself ([`crate::clipboard`]).

use eframe::egui::{Key, Modifiers};
use jarak_core::command::Command;

/// What a key press does in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Cmd(Command),
    New,
    Open,
    Save,
    SaveAs,
    Close,
    /// Tab: a tab character, or indent the selected lines.
    Indent,
    /// Shift+Tab: unindent the selected lines (or the caret's line).
    Outdent,
    ToggleOverwrite,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    NextDocument,
    PreviousDocument,
    ToggleWrap,
    Copy,
    Cut,
    Paste,
}

/// The action for `key` with `m` held, if any.
pub fn action_for(key: Key, m: Modifiers) -> Option<Action> {
    use Action as A;
    use Command as C;
    let ctrl = m.ctrl || m.command;
    let shift = m.shift;
    if m.alt && !ctrl {
        return match key {
            Key::Z if !shift => Some(A::ToggleWrap),
            _ => None,
        };
    }
    let cmd = |plain: C, selecting: C| Some(A::Cmd(if shift { selecting } else { plain }));
    match (ctrl, key) {
        (false, Key::ArrowLeft) => cmd(C::MoveLeft, C::SelectLeft),
        (false, Key::ArrowRight) => cmd(C::MoveRight, C::SelectRight),
        (false, Key::ArrowUp) => cmd(C::MoveUp, C::SelectUp),
        (false, Key::ArrowDown) => cmd(C::MoveDown, C::SelectDown),
        (true, Key::ArrowLeft) => cmd(C::WordLeft, C::SelectWordLeft),
        (true, Key::ArrowRight) => cmd(C::WordRight, C::SelectWordRight),
        (false, Key::Home) => cmd(C::LineStart, C::SelectLineStart),
        (false, Key::End) => cmd(C::LineEnd, C::SelectLineEnd),
        (true, Key::Home) => cmd(C::DocStart, C::SelectDocStart),
        (true, Key::End) => cmd(C::DocEnd, C::SelectDocEnd),
        (false, Key::PageUp) => cmd(C::PageUp, C::SelectPageUp),
        (false, Key::PageDown) => cmd(C::PageDown, C::SelectPageDown),
        // Shift+Backspace is Backspace too (as in Windows edit controls).
        (false, Key::Backspace) => Some(A::Cmd(C::DeleteBackward)),
        (true, Key::Backspace) => Some(A::Cmd(C::DeleteWordBackward)),
        (false, Key::Delete) if !shift => Some(A::Cmd(C::DeleteForward)),
        (true, Key::Delete) if !shift => Some(A::Cmd(C::DeleteWordForward)),
        (false, Key::Enter) => Some(A::Cmd(C::Newline)),
        (false, Key::Tab) => Some(if shift { A::Outdent } else { A::Indent }),
        (false, Key::Insert) if !shift => Some(A::ToggleOverwrite),
        (true, Key::A) if !shift => Some(A::Cmd(C::SelectAll)),
        (true, Key::C) if !shift => Some(A::Copy),
        (true, Key::Insert) if !shift => Some(A::Copy),
        (true, Key::X) if !shift => Some(A::Cut),
        (false, Key::Delete) => Some(A::Cut),
        (true, Key::V) if !shift => Some(A::Paste),
        (false, Key::Insert) => Some(A::Paste),
        (true, Key::Z) if !shift => Some(A::Cmd(C::Undo)),
        (true, Key::Z) => Some(A::Cmd(C::Redo)),
        (true, Key::Y) if !shift => Some(A::Cmd(C::Redo)),
        (true, Key::N) if !shift => Some(A::New),
        (true, Key::O) if !shift => Some(A::Open),
        (true, Key::S) if !shift => Some(A::Save),
        (true, Key::S) => Some(A::SaveAs),
        (true, Key::W) if !shift => Some(A::Close),
        (true, Key::F4) if !shift => Some(A::Close),
        (true, Key::Plus | Key::Equals) => Some(A::ZoomIn),
        (true, Key::Minus) => Some(A::ZoomOut),
        (true, Key::Num0) if !shift => Some(A::ZoomReset),
        (true, Key::Tab) => Some(if shift {
            A::PreviousDocument
        } else {
            A::NextDocument
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(ctrl: bool, shift: bool) -> Modifiers {
        Modifiers {
            ctrl,
            shift,
            command: ctrl,
            ..Modifiers::NONE
        }
    }

    #[test]
    fn the_standard_editing_keys() {
        use Action as A;
        use Command as C;
        let k = |key, c, s| action_for(key, m(c, s));
        assert_eq!(k(Key::ArrowLeft, false, false), Some(A::Cmd(C::MoveLeft)));
        assert_eq!(k(Key::ArrowLeft, false, true), Some(A::Cmd(C::SelectLeft)));
        assert_eq!(
            k(Key::ArrowRight, true, true),
            Some(A::Cmd(C::SelectWordRight))
        );
        assert_eq!(k(Key::Home, true, false), Some(A::Cmd(C::DocStart)));
        assert_eq!(k(Key::End, false, true), Some(A::Cmd(C::SelectLineEnd)));
        assert_eq!(
            k(Key::PageDown, false, true),
            Some(A::Cmd(C::SelectPageDown))
        );
        assert_eq!(
            k(Key::Backspace, true, false),
            Some(A::Cmd(C::DeleteWordBackward))
        );
        assert_eq!(k(Key::Delete, false, false), Some(A::Cmd(C::DeleteForward)));
        assert_eq!(k(Key::Enter, false, false), Some(A::Cmd(C::Newline)));
        assert_eq!(k(Key::Tab, false, false), Some(A::Indent));
        assert_eq!(k(Key::Tab, false, true), Some(A::Outdent));
        assert_eq!(k(Key::A, true, false), Some(A::Cmd(C::SelectAll)));
        assert_eq!(k(Key::Z, true, false), Some(A::Cmd(C::Undo)));
        assert_eq!(k(Key::Z, true, true), Some(A::Cmd(C::Redo)));
        assert_eq!(k(Key::Y, true, false), Some(A::Cmd(C::Redo)));
        assert_eq!(k(Key::S, true, false), Some(A::Save));
        assert_eq!(k(Key::S, true, true), Some(A::SaveAs));
        assert_eq!(k(Key::O, true, false), Some(A::Open));
        assert_eq!(k(Key::N, true, false), Some(A::New));
        assert_eq!(k(Key::W, true, false), Some(A::Close));
        assert_eq!(k(Key::Insert, false, false), Some(A::ToggleOverwrite));
        assert_eq!(k(Key::Equals, true, false), Some(A::ZoomIn));
        assert_eq!(k(Key::Num0, true, false), Some(A::ZoomReset));
        assert_eq!(k(Key::Tab, true, false), Some(A::NextDocument));
        assert_eq!(k(Key::C, true, false), Some(A::Copy));
        assert_eq!(k(Key::X, true, false), Some(A::Cut));
        assert_eq!(k(Key::V, true, false), Some(A::Paste));
        assert_eq!(k(Key::Insert, true, false), Some(A::Copy));
        assert_eq!(k(Key::Delete, false, true), Some(A::Cut));
        assert_eq!(k(Key::Insert, false, true), Some(A::Paste));
        let alt = Modifiers {
            alt: true,
            ..Modifiers::NONE
        };
        assert_eq!(action_for(Key::Z, alt), Some(A::ToggleWrap));
        // Letters without Ctrl are text, not actions.
        assert_eq!(k(Key::A, false, false), None);
        assert_eq!(k(Key::Q, true, false), None);
    }
}
