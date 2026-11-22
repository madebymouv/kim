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
    /// Ctrl+E: the Markdown source or its preview.
    TogglePreview,
    /// Ctrl+Shift+E: source and preview side by side.
    ToggleSplit,
    /// Ctrl+Shift+P: the command palette (Step 92).
    Palette,
    /// Step 93: F8, the other view takes the focus.
    FocusOther,
    /// Ctrl+1 … Ctrl+9: the tab of that number (9: the last).
    GoToTab(u8),
    /// Ctrl+Shift+PageUp / PageDown: the tab one place left / right.
    MoveTabLeft,
    MoveTabRight,
    /// Ctrl+Shift+T: the last closed file again.
    ReopenClosed,
    /// Step 94: Ctrl+F / Ctrl+H, the find panel.
    Find,
    Replace,
    /// F3 / Shift+F3.
    FindNext,
    FindPrevious,
    /// Ctrl+F3 / Ctrl+Shift+F3: select the word and find it.
    SelectFindNext,
    SelectFindPrevious,
    /// Ctrl+Alt+F3 / Ctrl+Alt+Shift+F3: the word under the caret, the find box untouched.
    VolatileNext,
    VolatilePrevious,
    /// Ctrl+Alt+I: the incremental search bar.
    Incremental,
    /// Step 95: Ctrl+Shift+F Find in Files; F4 / Shift+F4 the next / previous search result; F7
    /// the search results panel.
    FindInFiles,
    NextResult,
    PreviousResult,
    ToggleResults,
    /// Step 96: Ctrl+Shift+D add the next occurrence, Alt+F3 select all occurrences, Esc back
    /// to one caret, Alt+Shift+arrows a column selection, Alt+C the Column Editor.
    AddNextOccurrence,
    SelectAllOccurrences,
    SingleCaret,
    BlockUp,
    BlockDown,
    BlockLeft,
    BlockRight,
    ColumnEditor,
    /// Step 99: fold (`true`) or unfold — 0 every fold (Alt+0 / Alt+Shift+0), 1–8 a level
    /// (Alt+1…8 / Alt+Shift+1…8), 9 the current one (Ctrl+Alt+F / Ctrl+Alt+Shift+F); Ctrl+G Go to,
    /// Ctrl+B the matching bracket, Ctrl+Alt+B select between them.
    Fold(u8, bool),
    /// Step 100: completion — 0 everything (Ctrl+Space), 1 the document's words (Ctrl+Enter),
    /// 2 paths (Ctrl+Alt+Space), 3 the parameter hint (Ctrl+Shift+Space).
    Complete(u8),
    /// Step 101: Alt+Left / Alt+Right the navigation history (back: `true`), Ctrl+P quick open.
    Navigate(bool),
    QuickOpen,
    /// Step 102: 0 start / stop recording (Ctrl+Shift+R), 1 play back the recorded macro (F9 —
    /// Notepad++'s Ctrl+Shift+P is Jarak's command palette); F5 the Run dialog.
    Macro(u8),
    RunDialog,
    /// Step 105: Ctrl+, the Settings window.
    Settings,
    /// Step 106: File → Print… (the print dialog) and Print now (the default printer). No
    /// default keys (Ctrl+P is Go to file); Settings → Keyboard gives them some.
    Print,
    PrintNow,
    GoTo,
    MatchBrace,
    SelectBraces,
}

/// The action for `key` with `m` held, if any.
pub fn action_for(key: Key, m: Modifiers) -> Option<Action> {
    use Action as A;
    use Command as C;
    let ctrl = m.ctrl || m.command;
    let shift = m.shift;
    if m.alt && !ctrl {
        if let Some(n) = fold_digit(key) {
            return Some(A::Fold(n, !shift));
        }
        return match key {
            // Step 101: Notepad++'s Alt+H hides lines; the navigation history.
            Key::H if !shift => Some(A::Cmd(C::HideLines)),
            Key::ArrowLeft if !shift => Some(A::Navigate(true)),
            Key::ArrowRight if !shift => Some(A::Navigate(false)),
            Key::Z if !shift => Some(A::ToggleWrap),
            Key::C if !shift => Some(A::ColumnEditor),
            // Step 97: Alt+U Proper Case, Alt+Shift+U Proper Case (blend).
            Key::U => Some(A::Cmd(if shift {
                C::ProperCaseBlend
            } else {
                C::ProperCase
            })),
            Key::F3 if !shift => Some(A::SelectAllOccurrences),
            Key::ArrowUp if shift => Some(A::BlockUp),
            Key::ArrowDown if shift => Some(A::BlockDown),
            Key::ArrowLeft if shift => Some(A::BlockLeft),
            Key::ArrowRight if shift => Some(A::BlockRight),
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
        (true, Key::E) => Some(if shift {
            A::ToggleSplit
        } else {
            A::TogglePreview
        }),
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
        (true, Key::P) if shift => Some(A::Palette),
        (false, Key::F8) => Some(A::FocusOther),
        (false, Key::F3) => Some(if shift { A::FindPrevious } else { A::FindNext }),
        (true, Key::F3) if m.alt => Some(if shift {
            A::VolatilePrevious
        } else {
            A::VolatileNext
        }),
        (true, Key::F3) => Some(if shift {
            A::SelectFindPrevious
        } else {
            A::SelectFindNext
        }),
        (true, Key::I) if m.alt && !shift => Some(A::Incremental),
        (true, Key::F) if shift && !m.alt => Some(A::FindInFiles),
        (true, Key::F) if m.alt => Some(A::Fold(9, !shift)),
        (true, Key::G) if !shift && !m.alt => Some(A::GoTo),
        (true, Key::P) if !shift && !m.alt => Some(A::QuickOpen),
        (true, Key::R) if shift && !m.alt => Some(A::Macro(0)),
        (false, Key::F9) if !shift => Some(A::Macro(1)),
        (false, Key::F5) if !shift => Some(A::RunDialog),
        (true, Key::Comma) if !shift && !m.alt => Some(A::Settings),
        (true, Key::F2) if !shift => Some(A::Cmd(C::ToggleBookmark)),
        (false, Key::F2) => Some(A::Cmd(if shift {
            C::PreviousBookmark
        } else {
            C::NextBookmark
        })),
        (true, Key::Space) => Some(A::Complete(if m.alt {
            2
        } else if shift {
            3
        } else {
            0
        })),
        (true, Key::Enter) if !shift && !m.alt => Some(A::Complete(1)),
        (true, Key::B) if !shift => Some(if m.alt {
            A::SelectBraces
        } else {
            A::MatchBrace
        }),
        (true, Key::D) if shift && !m.alt => Some(A::AddNextOccurrence),
        // Step 97: Notepad++'s line and case keys.
        (true, Key::D) if !m.alt => Some(A::Cmd(C::DuplicateLine)),
        (true, Key::L) if shift && !m.alt => Some(A::Cmd(C::DeleteLine)),
        (true, Key::ArrowUp) if shift && !m.alt => Some(A::Cmd(C::MoveLineUp)),
        (true, Key::ArrowDown) if shift && !m.alt => Some(A::Cmd(C::MoveLineDown)),
        (true, Key::J) if !shift && !m.alt => Some(A::Cmd(C::JoinLines)),
        (true, Key::I) if !shift && !m.alt => Some(A::Cmd(C::SplitLines)),
        (true, Key::Enter) if m.alt => Some(A::Cmd(if shift {
            C::InsertLineBelow
        } else {
            C::InsertLineAbove
        })),
        (true, Key::U) if m.alt => Some(A::Cmd(if shift {
            C::SentenceCaseBlend
        } else {
            C::SentenceCase
        })),
        (true, Key::U) => Some(A::Cmd(if shift { C::UpperCase } else { C::LowerCase })),
        // Step 98: Notepad++'s comment keys.
        (true, Key::Q) if !m.alt => Some(A::Cmd(if shift {
            C::BlockComment
        } else {
            C::ToggleComment
        })),
        (true, Key::K) if !m.alt => Some(A::Cmd(if shift {
            C::LineUncomment
        } else {
            C::LineComment
        })),
        (false, Key::Escape) if !shift => Some(A::SingleCaret),
        (true, Key::F) if !shift && !m.alt => Some(A::Find),
        (false, Key::F4) => Some(if shift {
            A::PreviousResult
        } else {
            A::NextResult
        }),
        (false, Key::F7) if !shift => Some(A::ToggleResults),
        (true, Key::H) if !shift && !m.alt => Some(A::Replace),
        (true, Key::PageUp) if shift => Some(A::MoveTabLeft),
        (true, Key::PageDown) if shift => Some(A::MoveTabRight),
        (true, Key::T) if shift => Some(A::ReopenClosed),
        (true, k) if !shift && digit(k).is_some() => digit(k).map(A::GoToTab),
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

/// Alt+0 … Alt+8 (Step 99: the fold levels).
fn fold_digit(k: Key) -> Option<u8> {
    Some(match k {
        Key::Num0 => 0,
        Key::Num9 => return None,
        _ => digit(k)?,
    })
}

/// Ctrl+1 … Ctrl+9.
fn digit(k: Key) -> Option<u8> {
    Some(match k {
        Key::Num1 => 1,
        Key::Num2 => 2,
        Key::Num3 => 3,
        Key::Num4 => 4,
        Key::Num5 => 5,
        Key::Num6 => 6,
        Key::Num7 => 7,
        Key::Num8 => 8,
        Key::Num9 => 9,
        _ => return None,
    })
}

/// The keys of `a` as the palette and the menus show them (the default bindings above; a test
/// checks every label against [`action_for`]).
pub fn key_label(a: Action) -> Option<&'static str> {
    use Action as A;
    use Command as C;
    Some(match a {
        A::Cmd(c) => match c {
            C::Undo => "Ctrl+Z",
            C::Redo => "Ctrl+Y",
            C::DeleteBackward => "Backspace",
            C::DeleteForward => "Delete",
            C::DeleteWordBackward => "Ctrl+Backspace",
            C::DeleteWordForward => "Ctrl+Delete",
            C::Newline => "Enter",
            C::SelectAll => "Ctrl+A",
            C::MoveLeft => "Left",
            C::MoveRight => "Right",
            C::MoveUp => "Up",
            C::MoveDown => "Down",
            C::WordLeft => "Ctrl+Left",
            C::WordRight => "Ctrl+Right",
            C::LineStart => "Home",
            C::LineEnd => "End",
            C::DocStart => "Ctrl+Home",
            C::DocEnd => "Ctrl+End",
            C::PageUp => "PageUp",
            C::PageDown => "PageDown",
            C::SelectLeft => "Shift+Left",
            C::SelectRight => "Shift+Right",
            C::SelectUp => "Shift+Up",
            C::SelectDown => "Shift+Down",
            C::SelectWordLeft => "Ctrl+Shift+Left",
            C::SelectWordRight => "Ctrl+Shift+Right",
            C::SelectLineStart => "Shift+Home",
            C::SelectLineEnd => "Shift+End",
            C::SelectDocStart => "Ctrl+Shift+Home",
            C::SelectDocEnd => "Ctrl+Shift+End",
            C::SelectPageUp => "Shift+PageUp",
            C::SelectPageDown => "Shift+PageDown",
            C::DuplicateLine => "Ctrl+D",
            C::DeleteLine => "Ctrl+Shift+L",
            C::MoveLineUp => "Ctrl+Shift+Up",
            C::MoveLineDown => "Ctrl+Shift+Down",
            C::JoinLines => "Ctrl+J",
            C::SplitLines => "Ctrl+I",
            C::InsertLineAbove => "Ctrl+Alt+Enter",
            C::InsertLineBelow => "Ctrl+Alt+Shift+Enter",
            C::UpperCase => "Ctrl+Shift+U",
            C::LowerCase => "Ctrl+U",
            C::ProperCase => "Alt+U",
            C::ProperCaseBlend => "Alt+Shift+U",
            C::SentenceCase => "Ctrl+Alt+U",
            C::SentenceCaseBlend => "Ctrl+Alt+Shift+U",
            C::ToggleComment => "Ctrl+Q",
            C::BlockComment => "Ctrl+Shift+Q",
            C::LineComment => "Ctrl+K",
            C::LineUncomment => "Ctrl+Shift+K",
            C::ToggleBookmark => "Ctrl+F2",
            C::NextBookmark => "F2",
            C::PreviousBookmark => "Shift+F2",
            C::HideLines => "Alt+H",
            // The rest are in the palette and the menus only.
            _ => return None,
        },
        A::New => "Ctrl+N",
        A::Open => "Ctrl+O",
        A::Save => "Ctrl+S",
        A::SaveAs => "Ctrl+Shift+S",
        A::Close => "Ctrl+W",
        A::Indent => "Tab",
        A::Outdent => "Shift+Tab",
        A::ToggleOverwrite => "Insert",
        A::ZoomIn => "Ctrl+Plus",
        A::ZoomOut => "Ctrl+Minus",
        A::ZoomReset => "Ctrl+0",
        A::NextDocument => "Ctrl+Tab",
        A::PreviousDocument => "Ctrl+Shift+Tab",
        A::ToggleWrap => "Alt+Z",
        A::Copy => "Ctrl+C",
        A::Cut => "Ctrl+X",
        A::Paste => "Ctrl+V",
        A::TogglePreview => "Ctrl+E",
        A::ToggleSplit => "Ctrl+Shift+E",
        A::Palette => "Ctrl+Shift+P",
        A::FocusOther => "F8",
        A::GoToTab(n) => [
            "Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8",
            "Ctrl+9",
        ][usize::from(n.clamp(1, 9)) - 1],
        A::MoveTabLeft => "Ctrl+Shift+PageUp",
        A::MoveTabRight => "Ctrl+Shift+PageDown",
        A::ReopenClosed => "Ctrl+Shift+T",
        A::Find => "Ctrl+F",
        A::Replace => "Ctrl+H",
        A::FindNext => "F3",
        A::FindPrevious => "Shift+F3",
        A::SelectFindNext => "Ctrl+F3",
        A::SelectFindPrevious => "Ctrl+Shift+F3",
        A::VolatileNext => "Ctrl+Alt+F3",
        A::VolatilePrevious => "Ctrl+Alt+Shift+F3",
        A::Incremental => "Ctrl+Alt+I",
        A::FindInFiles => "Ctrl+Shift+F",
        A::NextResult => "F4",
        A::PreviousResult => "Shift+F4",
        A::ToggleResults => "F7",
        A::AddNextOccurrence => "Ctrl+Shift+D",
        A::SelectAllOccurrences => "Alt+F3",
        A::SingleCaret => "Esc",
        A::BlockUp => "Alt+Shift+Up",
        A::BlockDown => "Alt+Shift+Down",
        A::BlockLeft => "Alt+Shift+Left",
        A::BlockRight => "Alt+Shift+Right",
        A::ColumnEditor => "Alt+C",
        A::Fold(0, true) => "Alt+0",
        A::Fold(0, false) => "Alt+Shift+0",
        A::Fold(9, true) => "Ctrl+Alt+F",
        A::Fold(9, false) => "Ctrl+Alt+Shift+F",
        A::Fold(n, true) => [
            "Alt+1", "Alt+2", "Alt+3", "Alt+4", "Alt+5", "Alt+6", "Alt+7", "Alt+8",
        ][usize::from(n.clamp(1, 8)) - 1],
        A::Fold(n, false) => [
            "Alt+Shift+1",
            "Alt+Shift+2",
            "Alt+Shift+3",
            "Alt+Shift+4",
            "Alt+Shift+5",
            "Alt+Shift+6",
            "Alt+Shift+7",
            "Alt+Shift+8",
        ][usize::from(n.clamp(1, 8)) - 1],
        A::GoTo => "Ctrl+G",
        A::Navigate(true) => "Alt+Left",
        A::Navigate(false) => "Alt+Right",
        A::QuickOpen => "Ctrl+P",
        A::Macro(0) => "Ctrl+Shift+R",
        A::Macro(_) => "F9",
        A::RunDialog => "F5",
        A::Settings => "Ctrl+,",
        A::Print | A::PrintNow => return None,
        A::Complete(0) => "Ctrl+Space",
        A::Complete(1) => "Ctrl+Enter",
        A::Complete(2) => "Ctrl+Alt+Space",
        A::Complete(_) => "Ctrl+Shift+Space",
        A::MatchBrace => "Ctrl+B",
        A::SelectBraces => "Ctrl+Alt+B",
    })
}

/// A label of [`key_label`] as the key and modifiers it names.
pub fn parse_label(label: &str) -> Option<(Key, Modifiers)> {
    let mut m = Modifiers::NONE;
    let mut key = None;
    for part in label.split('+') {
        match part {
            "Ctrl" => {
                m.ctrl = true;
                m.command = true;
            }
            "Shift" => m.shift = true,
            "Alt" => m.alt = true,
            "Plus" => key = Some(Key::Plus),
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && d != "0" => {
                key = Key::from_name(&format!("Num{d}")).or_else(|| Key::from_name(d));
            }
            "Minus" => key = Some(Key::Minus),
            "0" => key = Some(Key::Num0),
            "Left" => key = Some(Key::ArrowLeft),
            "Right" => key = Some(Key::ArrowRight),
            "Up" => key = Some(Key::ArrowUp),
            "Down" => key = Some(Key::ArrowDown),
            name => key = Key::from_name(name),
        }
    }
    key.map(|k| (k, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_label_is_the_binding_of_its_action() {
        let mut actions: Vec<Action> = jarak_core::command::COMMANDS
            .iter()
            .map(|c| Action::Cmd(c.command))
            // Step 97: commands without default keys are in the palette only.
            .filter(|a| key_label(*a).is_some())
            .collect();
        actions.extend([
            Action::New,
            Action::Open,
            Action::Save,
            Action::SaveAs,
            Action::Close,
            Action::Indent,
            Action::Outdent,
            Action::ToggleOverwrite,
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomReset,
            Action::NextDocument,
            Action::PreviousDocument,
            Action::ToggleWrap,
            Action::Copy,
            Action::Cut,
            Action::Paste,
            Action::TogglePreview,
            Action::ToggleSplit,
            Action::Palette,
            Action::FocusOther,
            Action::GoToTab(1),
            Action::GoToTab(9),
            Action::MoveTabLeft,
            Action::MoveTabRight,
            Action::ReopenClosed,
            Action::Find,
            Action::Replace,
            Action::FindNext,
            Action::FindPrevious,
            Action::SelectFindNext,
            Action::SelectFindPrevious,
            Action::VolatileNext,
            Action::VolatilePrevious,
            Action::Incremental,
            Action::FindInFiles,
            Action::NextResult,
            Action::PreviousResult,
            Action::ToggleResults,
            Action::AddNextOccurrence,
            Action::SelectAllOccurrences,
            Action::SingleCaret,
            Action::BlockUp,
            Action::BlockDown,
            Action::BlockLeft,
            Action::BlockRight,
            Action::ColumnEditor,
            Action::Fold(0, true),
            Action::Fold(0, false),
            Action::Fold(1, true),
            Action::Fold(4, false),
            Action::Fold(8, true),
            Action::Fold(8, false),
            Action::Fold(9, true),
            Action::Fold(9, false),
            Action::GoTo,
            Action::Navigate(true),
            Action::Navigate(false),
            Action::QuickOpen,
            Action::Macro(0),
            Action::Macro(1),
            Action::RunDialog,
            Action::Settings,
            Action::Complete(0),
            Action::Complete(1),
            Action::Complete(2),
            Action::Complete(3),
            Action::MatchBrace,
            Action::SelectBraces,
        ]);
        for a in actions {
            let label = key_label(a).expect("every action has keys");
            let (k, m) = parse_label(label).unwrap_or_else(|| panic!("{label}"));
            assert_eq!(action_for(k, m), Some(a), "{label}");
        }
    }

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
        assert_eq!(k(Key::E, true, false), Some(A::TogglePreview));
        assert_eq!(k(Key::E, true, true), Some(A::ToggleSplit));
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
        assert_eq!(k(Key::R, true, false), None);
    }
}
