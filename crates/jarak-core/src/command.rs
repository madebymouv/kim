//! The command registry (`JARAK-FR-014`): every user-visible action has one stable id and a
//! title in English and Korean. Keys, the palette (Step 92), menus, macros (Step 102) and the Vim
//! layer all name these commands; nothing edits the document another way.
//!
//! Step 86 registers the editing and caret commands of the text model; later Steps add theirs.

use crate::document::Document;

/// A command that needs no argument (typing text is [`Document::type_text`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Undo,
    Redo,
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    Newline,
    SelectAll,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
    PageUp,
    PageDown,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectWordLeft,
    SelectWordRight,
    SelectLineStart,
    SelectLineEnd,
    SelectDocStart,
    SelectDocEnd,
    SelectPageUp,
    SelectPageDown,
}

/// A command's identity in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandInfo {
    pub command: Command,
    /// Stable, dotted, lowercase (`edit.undo`); used in key bindings and macros.
    pub id: &'static str,
    pub en: &'static str,
    pub ko: &'static str,
}

macro_rules! registry {
    ($($cmd:ident, $id:literal, $en:literal, $ko:literal;)*) => {
        /// Every registered command, in palette order.
        pub const COMMANDS: &[CommandInfo] = &[
            $(CommandInfo { command: Command::$cmd, id: $id, en: $en, ko: $ko },)*
        ];
    };
}

registry! {
    Undo, "edit.undo", "Undo", "실행 취소";
    Redo, "edit.redo", "Redo", "다시 실행";
    DeleteBackward, "edit.delete-backward", "Delete the character before the caret", "앞 글자 지우기";
    DeleteForward, "edit.delete-forward", "Delete the character after the caret", "뒤 글자 지우기";
    DeleteWordBackward, "edit.delete-word-backward", "Delete to the start of the word", "단어 앞까지 지우기";
    DeleteWordForward, "edit.delete-word-forward", "Delete to the start of the next word", "다음 단어 앞까지 지우기";
    Newline, "edit.newline", "New line (keeps the indentation)", "줄 바꾸기(들여쓰기 유지)";
    SelectAll, "select.all", "Select all", "모두 선택";
    MoveLeft, "cursor.left", "Move left", "왼쪽으로 이동";
    MoveRight, "cursor.right", "Move right", "오른쪽으로 이동";
    MoveUp, "cursor.up", "Move up", "위로 이동";
    MoveDown, "cursor.down", "Move down", "아래로 이동";
    WordLeft, "cursor.word-left", "Move to the previous word", "이전 단어로 이동";
    WordRight, "cursor.word-right", "Move to the next word", "다음 단어로 이동";
    LineStart, "cursor.line-start", "Move to the line start", "줄 처음으로 이동";
    LineEnd, "cursor.line-end", "Move to the line end", "줄 끝으로 이동";
    DocStart, "cursor.doc-start", "Move to the document start", "문서 처음으로 이동";
    DocEnd, "cursor.doc-end", "Move to the document end", "문서 끝으로 이동";
    PageUp, "cursor.page-up", "Page up", "한 화면 위로";
    PageDown, "cursor.page-down", "Page down", "한 화면 아래로";
    SelectLeft, "select.left", "Extend the selection left", "왼쪽으로 선택";
    SelectRight, "select.right", "Extend the selection right", "오른쪽으로 선택";
    SelectUp, "select.up", "Extend the selection up", "위로 선택";
    SelectDown, "select.down", "Extend the selection down", "아래로 선택";
    SelectWordLeft, "select.word-left", "Extend the selection to the previous word", "이전 단어까지 선택";
    SelectWordRight, "select.word-right", "Extend the selection to the next word", "다음 단어까지 선택";
    SelectLineStart, "select.line-start", "Extend the selection to the line start", "줄 처음까지 선택";
    SelectLineEnd, "select.line-end", "Extend the selection to the line end", "줄 끝까지 선택";
    SelectDocStart, "select.doc-start", "Extend the selection to the document start", "문서 처음까지 선택";
    SelectDocEnd, "select.doc-end", "Extend the selection to the document end", "문서 끝까지 선택";
    SelectPageUp, "select.page-up", "Extend the selection a page up", "한 화면 위까지 선택";
    SelectPageDown, "select.page-down", "Extend the selection a page down", "한 화면 아래까지 선택";
}

impl Command {
    /// The registry entry.
    pub fn info(self) -> &'static CommandInfo {
        COMMANDS
            .iter()
            .find(|c| c.command == self)
            .expect("every command is registered")
    }

    pub fn id(self) -> &'static str {
        self.info().id
    }

    pub fn from_id(id: &str) -> Option<Command> {
        COMMANDS.iter().find(|c| c.id == id).map(|c| c.command)
    }
}

impl Document {
    /// Runs `cmd`; `page` is the number of lines a page move covers (the view knows it).
    /// Returns whether anything happened (false for undo with nothing to undo).
    pub fn execute(&mut self, cmd: Command, page: usize) -> bool {
        let page = page.max(1) as isize;
        let before = (self.history().state(), self.selection().clone());
        match cmd {
            Command::Undo => return self.undo(),
            Command::Redo => return self.redo(),
            Command::DeleteBackward => self.delete_backward(),
            Command::DeleteForward => self.delete_forward(),
            Command::DeleteWordBackward => self.delete_word_backward(),
            Command::DeleteWordForward => self.delete_word_forward(),
            Command::Newline => self.newline(),
            Command::SelectAll => self.select_all(),
            Command::MoveLeft => self.move_left(false),
            Command::MoveRight => self.move_right(false),
            Command::MoveUp => self.move_lines(-1, false),
            Command::MoveDown => self.move_lines(1, false),
            Command::WordLeft => self.word_left(false),
            Command::WordRight => self.word_right(false),
            Command::LineStart => self.line_start(false),
            Command::LineEnd => self.line_end(false),
            Command::DocStart => self.doc_start(false),
            Command::DocEnd => self.doc_end(false),
            Command::PageUp => self.move_lines(-page, false),
            Command::PageDown => self.move_lines(page, false),
            Command::SelectLeft => self.move_left(true),
            Command::SelectRight => self.move_right(true),
            Command::SelectUp => self.move_lines(-1, true),
            Command::SelectDown => self.move_lines(1, true),
            Command::SelectWordLeft => self.word_left(true),
            Command::SelectWordRight => self.word_right(true),
            Command::SelectLineStart => self.line_start(true),
            Command::SelectLineEnd => self.line_end(true),
            Command::SelectDocStart => self.doc_start(true),
            Command::SelectDocEnd => self.doc_end(true),
            Command::SelectPageUp => self.move_lines(-page, true),
            Command::SelectPageDown => self.move_lines(page, true),
        }
        before != (self.history().state(), self.selection().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_command_has_one_unique_id_and_both_titles() {
        let mut ids = HashSet::new();
        for c in COMMANDS {
            assert!(ids.insert(c.id), "duplicate id {}", c.id);
            assert!(
                c.id.chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch == '.' || ch == '-'),
                "{}",
                c.id
            );
            assert!(!c.en.is_empty() && !c.ko.is_empty(), "{}", c.id);
            assert_eq!(Command::from_id(c.id), Some(c.command));
            assert_eq!(c.command.id(), c.id);
        }
        assert_eq!(Command::from_id("edit.nope"), None);
    }

    #[test]
    fn execute_runs_the_document_operations() {
        let mut d = Document::from_text("ab\ncd");
        assert!(d.execute(Command::SelectDocEnd, 10));
        assert_eq!(d.selected_text(), "ab\ncd");
        assert!(d.execute(Command::DeleteBackward, 10));
        assert_eq!(d.len_chars(), 0);
        assert!(d.execute(Command::Undo, 10));
        assert_eq!(d.len_chars(), 5);
        assert!(d.execute(Command::Redo, 10));
        assert_eq!(d.len_chars(), 0);
        assert!(d.execute(Command::Undo, 10));
        assert!(!d.execute(Command::Undo, 10), "nothing left to undo");
        assert!(d.execute(Command::DocStart, 10));
        assert!(!d.execute(Command::MoveUp, 10), "already on the first line");
    }
}
