//! The command registry (`JARAK-FR-014`): every user-visible action has one stable id and a
//! title in English and Korean. Keys, the palette (Step 92), menus, macros (Step 102) and the Vim
//! layer all name these commands; nothing edits the document another way.
//!
//! Step 86 registers the editing and caret commands of the text model; later Steps add theirs.

use crate::document::Document;
use crate::lineops::{Blank, Case, Codec, CommentMode, Rng, Sort};

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
    // Step 97: line operations and text transformations (`crate::lineops`).
    DuplicateLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,
    JoinLines,
    SplitLines,
    RemoveDuplicateLines,
    RemoveConsecutiveDuplicates,
    RemoveEmptyLines,
    RemoveBlankLines,
    InsertLineAbove,
    InsertLineBelow,
    ReverseLines,
    RandomizeLines,
    SortLexAsc,
    SortLexDesc,
    SortNoCaseAsc,
    SortNoCaseDesc,
    SortLocaleAsc,
    SortLocaleDesc,
    SortIntAsc,
    SortIntDesc,
    SortDecDotAsc,
    SortDecDotDesc,
    SortDecCommaAsc,
    SortDecCommaDesc,
    SortLengthAsc,
    SortLengthDesc,
    UpperCase,
    LowerCase,
    ProperCase,
    ProperCaseBlend,
    SentenceCase,
    SentenceCaseBlend,
    InvertCase,
    RandomCase,
    TrimTrailing,
    TrimLeading,
    TrimBoth,
    EolToSpace,
    TrimBothEolToSpace,
    TabToSpace,
    SpaceToTabAll,
    SpaceToTabLeading,
    Base64Encode,
    Base64EncodeNoPad,
    Base64EncodeUrl,
    Base64Decode,
    UrlEncode,
    UrlDecode,
    QpEncode,
    QpDecode,
    HexEncode,
    HexDecode,
    // Step 98: comments in the language of the document.
    ToggleComment,
    LineComment,
    LineUncomment,
    BlockComment,
    BlockUncomment,
    // Step 101: bookmarks, the change history, hidden lines.
    ToggleBookmark,
    NextBookmark,
    PreviousBookmark,
    ClearBookmarks,
    InvertBookmarks,
    RemoveBookmarkedLines,
    RemoveUnmarkedLines,
    NextChange,
    PreviousChange,
    ClearChanges,
    HideLines,
    ShowHiddenLines,
}

/// A command's identity in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandInfo {
    pub command: Command,
    /// Stable, dotted, lowercase letters, digits and `-` (`edit.undo`, `mime.base64-encode`);
    /// used in key bindings and macros.
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
    DuplicateLine, "line.duplicate", "Duplicate the line or the selection", "줄 또는 선택 복제";
    DeleteLine, "line.delete", "Delete the line", "줄 지우기";
    MoveLineUp, "line.move-up", "Move the line up", "줄을 위로 옮기기";
    MoveLineDown, "line.move-down", "Move the line down", "줄을 아래로 옮기기";
    JoinLines, "line.join", "Join lines", "줄 합치기";
    SplitLines, "line.split", "Split lines at the window's width", "창 너비에서 줄 나누기";
    RemoveDuplicateLines, "line.remove-duplicates", "Remove duplicate lines", "중복 줄 지우기";
    RemoveConsecutiveDuplicates, "line.remove-consecutive-duplicates", "Remove consecutive duplicate lines", "연속 중복 줄 지우기";
    RemoveEmptyLines, "line.remove-empty", "Remove empty lines", "빈 줄 지우기";
    RemoveBlankLines, "line.remove-blank", "Remove empty lines (also with blanks only)", "빈 줄 지우기(공백만 있는 줄 포함)";
    InsertLineAbove, "line.insert-above", "Insert a blank line above", "위에 빈 줄 넣기";
    InsertLineBelow, "line.insert-below", "Insert a blank line below", "아래에 빈 줄 넣기";
    ReverseLines, "line.reverse", "Reverse the line order", "줄 순서 뒤집기";
    RandomizeLines, "line.randomize", "Randomize the line order", "줄 순서 섞기";
    SortLexAsc, "sort.lex-asc", "Sort lines lexicographically ascending", "줄 정렬: 사전순 오름차순";
    SortLexDesc, "sort.lex-desc", "Sort lines lexicographically descending", "줄 정렬: 사전순 내림차순";
    SortNoCaseAsc, "sort.nocase-asc", "Sort lines ignoring case ascending", "줄 정렬: 대소문자 무시 오름차순";
    SortNoCaseDesc, "sort.nocase-desc", "Sort lines ignoring case descending", "줄 정렬: 대소문자 무시 내림차순";
    SortLocaleAsc, "sort.locale-asc", "Sort lines in locale order ascending", "줄 정렬: 로캘 순서 오름차순";
    SortLocaleDesc, "sort.locale-desc", "Sort lines in locale order descending", "줄 정렬: 로캘 순서 내림차순";
    SortIntAsc, "sort.integer-asc", "Sort lines as integers ascending", "줄 정렬: 정수 오름차순";
    SortIntDesc, "sort.integer-desc", "Sort lines as integers descending", "줄 정렬: 정수 내림차순";
    SortDecDotAsc, "sort.decimal-dot-asc", "Sort lines as decimals (dot) ascending", "줄 정렬: 소수(점) 오름차순";
    SortDecDotDesc, "sort.decimal-dot-desc", "Sort lines as decimals (dot) descending", "줄 정렬: 소수(점) 내림차순";
    SortDecCommaAsc, "sort.decimal-comma-asc", "Sort lines as decimals (comma) ascending", "줄 정렬: 소수(쉼표) 오름차순";
    SortDecCommaDesc, "sort.decimal-comma-desc", "Sort lines as decimals (comma) descending", "줄 정렬: 소수(쉼표) 내림차순";
    SortLengthAsc, "sort.length-asc", "Sort lines by length ascending", "줄 정렬: 길이 오름차순";
    SortLengthDesc, "sort.length-desc", "Sort lines by length descending", "줄 정렬: 길이 내림차순";
    UpperCase, "case.upper", "UPPERCASE", "대문자로";
    LowerCase, "case.lower", "lowercase", "소문자로";
    ProperCase, "case.proper", "Proper Case", "단어 첫 글자 대문자";
    ProperCaseBlend, "case.proper-blend", "Proper Case (blend)", "단어 첫 글자 대문자(나머지 유지)";
    SentenceCase, "case.sentence", "Sentence case", "문장 첫 글자 대문자";
    SentenceCaseBlend, "case.sentence-blend", "Sentence case (blend)", "문장 첫 글자 대문자(나머지 유지)";
    InvertCase, "case.invert", "iNVERT cASE", "대소문자 뒤집기";
    RandomCase, "case.random", "ranDOm CasE", "무작위 대소문자";
    TrimTrailing, "blank.trim-trailing", "Trim trailing space", "줄 끝 공백 지우기";
    TrimLeading, "blank.trim-leading", "Trim leading space", "줄 앞 공백 지우기";
    TrimBoth, "blank.trim-both", "Trim leading and trailing space", "줄 앞뒤 공백 지우기";
    EolToSpace, "blank.eol-to-space", "EOL to space", "줄 끝을 공백으로";
    TrimBothEolToSpace, "blank.trim-eol-to-space", "Trim both and EOL to space", "앞뒤 공백 지우고 줄 끝을 공백으로";
    TabToSpace, "blank.tab-to-space", "TAB to space", "탭을 공백으로";
    SpaceToTabAll, "blank.space-to-tab", "Space to TAB (all)", "공백을 탭으로(모두)";
    SpaceToTabLeading, "blank.space-to-tab-leading", "Space to TAB (leading)", "공백을 탭으로(앞쪽만)";
    Base64Encode, "mime.base64-encode", "Base64 encode", "Base64 인코딩";
    Base64EncodeNoPad, "mime.base64-encode-nopad", "Base64 encode (no padding)", "Base64 인코딩(패딩 없음)";
    Base64EncodeUrl, "mime.base64url-encode", "Base64 encode (URL-safe)", "Base64 인코딩(URL 안전)";
    Base64Decode, "mime.base64-decode", "Base64 decode", "Base64 디코딩";
    UrlEncode, "mime.url-encode", "URL encode", "URL 인코딩";
    UrlDecode, "mime.url-decode", "URL decode", "URL 디코딩";
    QpEncode, "mime.qp-encode", "Quoted-printable encode", "Quoted-printable 인코딩";
    QpDecode, "mime.qp-decode", "Quoted-printable decode", "Quoted-printable 디코딩";
    HexEncode, "convert.ascii-to-hex", "ASCII to hex", "ASCII를 16진으로";
    HexDecode, "convert.hex-to-ascii", "Hex to ASCII", "16진을 ASCII로";
    ToggleComment, "comment.toggle", "Toggle the line comment", "줄 주석 켜기/끄기";
    LineComment, "comment.line", "Comment the lines", "줄 주석 달기";
    LineUncomment, "comment.line-remove", "Uncomment the lines", "줄 주석 없애기";
    BlockComment, "comment.block", "Block comment", "블록 주석 달기";
    BlockUncomment, "comment.block-remove", "Block uncomment", "블록 주석 없애기";
    ToggleBookmark, "bookmark.toggle", "Toggle the bookmark", "책갈피 켜기/끄기";
    NextBookmark, "bookmark.next", "Next bookmark", "다음 책갈피";
    PreviousBookmark, "bookmark.previous", "Previous bookmark", "이전 책갈피";
    ClearBookmarks, "bookmark.clear", "Clear all bookmarks", "책갈피 모두 지우기";
    InvertBookmarks, "bookmark.invert", "Inverse bookmarks", "책갈피 반전";
    RemoveBookmarkedLines, "bookmark.remove-lines", "Remove the bookmarked lines", "책갈피 줄 지우기";
    RemoveUnmarkedLines, "bookmark.remove-unmarked", "Remove the lines without a bookmark", "책갈피 없는 줄 지우기";
    NextChange, "change.next", "Go to the next change", "다음 변경으로";
    PreviousChange, "change.previous", "Go to the previous change", "이전 변경으로";
    ClearChanges, "change.clear", "Clear the change history", "변경 기록 지우기";
    HideLines, "view.hide-lines", "Hide lines", "줄 숨기기";
    ShowHiddenLines, "view.show-lines", "Show all hidden lines", "숨긴 줄 모두 보이기";
}

impl Command {
    /// Whether the command changes the text (a read-only document refuses those). Moving,
    /// selecting, bookmarks, the change history and hidden lines do not.
    pub fn edits(self) -> bool {
        let id = self.id();
        !(id.starts_with("cursor.")
            || id.starts_with("select.")
            || id.starts_with("change.")
            || id.starts_with("view.")
            || matches!(
                self,
                Command::ToggleBookmark
                    | Command::NextBookmark
                    | Command::PreviousBookmark
                    | Command::ClearBookmarks
                    | Command::InvertBookmarks
            ))
    }

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
            Command::DeleteBackward => self.delete_backward_assisted(),
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
            Command::DuplicateLine => self.duplicate(),
            Command::DeleteLine => self.delete_lines(),
            Command::MoveLineUp => self.move_selected_lines(false),
            Command::MoveLineDown => self.move_selected_lines(true),
            Command::JoinLines => self.join_lines(),
            Command::SplitLines => self.split_lines(self.split_width),
            Command::RemoveDuplicateLines => self.remove_duplicate_lines(false),
            Command::RemoveConsecutiveDuplicates => self.remove_duplicate_lines(true),
            Command::RemoveEmptyLines => self.remove_empty_lines(false),
            Command::RemoveBlankLines => self.remove_empty_lines(true),
            Command::InsertLineAbove => self.insert_blank_line(true),
            Command::InsertLineBelow => self.insert_blank_line(false),
            Command::ReverseLines => self.reverse_lines(),
            Command::RandomizeLines => self.randomize_lines(&mut Rng::from_clock()),
            Command::SortLexAsc => self.sort(Sort::Lex, false),
            Command::SortLexDesc => self.sort(Sort::Lex, true),
            Command::SortNoCaseAsc => self.sort(Sort::LexNoCase, false),
            Command::SortNoCaseDesc => self.sort(Sort::LexNoCase, true),
            Command::SortLocaleAsc => self.sort(Sort::Locale, false),
            Command::SortLocaleDesc => self.sort(Sort::Locale, true),
            Command::SortIntAsc => self.sort(Sort::Integer, false),
            Command::SortIntDesc => self.sort(Sort::Integer, true),
            Command::SortDecDotAsc => self.sort(Sort::DecimalDot, false),
            Command::SortDecDotDesc => self.sort(Sort::DecimalDot, true),
            Command::SortDecCommaAsc => self.sort(Sort::DecimalComma, false),
            Command::SortDecCommaDesc => self.sort(Sort::DecimalComma, true),
            Command::SortLengthAsc => self.sort(Sort::Length, false),
            Command::SortLengthDesc => self.sort(Sort::Length, true),
            Command::UpperCase => return self.change_case(Case::Upper, &mut Rng::from_clock()),
            Command::LowerCase => return self.change_case(Case::Lower, &mut Rng::from_clock()),
            Command::ProperCase => return self.change_case(Case::Proper, &mut Rng::from_clock()),
            Command::ProperCaseBlend => {
                return self.change_case(Case::ProperBlend, &mut Rng::from_clock());
            }
            Command::SentenceCase => {
                return self.change_case(Case::Sentence, &mut Rng::from_clock());
            }
            Command::SentenceCaseBlend => {
                return self.change_case(Case::SentenceBlend, &mut Rng::from_clock());
            }
            Command::InvertCase => return self.change_case(Case::Invert, &mut Rng::from_clock()),
            Command::RandomCase => return self.change_case(Case::Random, &mut Rng::from_clock()),
            Command::TrimTrailing => self.blank(Blank::TrimTrailing),
            Command::TrimLeading => self.blank(Blank::TrimLeading),
            Command::TrimBoth => self.blank(Blank::TrimBoth),
            Command::EolToSpace => self.blank(Blank::EolToSpace),
            Command::TrimBothEolToSpace => self.blank(Blank::TrimBothEolToSpace),
            Command::TabToSpace => self.blank(Blank::TabToSpace),
            Command::SpaceToTabAll => self.blank(Blank::SpaceToTabAll),
            Command::SpaceToTabLeading => self.blank(Blank::SpaceToTabLeading),
            Command::Base64Encode => return self.encode(Codec::Base64),
            Command::Base64EncodeNoPad => return self.encode(Codec::Base64NoPad),
            Command::Base64EncodeUrl => return self.encode(Codec::Base64Url),
            Command::Base64Decode => return self.decode(Codec::Base64),
            Command::UrlEncode => return self.encode(Codec::Url),
            Command::UrlDecode => return self.decode(Codec::Url),
            Command::QpEncode => return self.encode(Codec::QuotedPrintable),
            Command::QpDecode => return self.decode(Codec::QuotedPrintable),
            Command::HexEncode => return self.encode(Codec::Hex),
            Command::HexDecode => return self.decode(Codec::Hex),
            Command::ToggleComment => return self.comment_lines(CommentMode::Toggle),
            Command::LineComment => return self.comment_lines(CommentMode::Add),
            Command::LineUncomment => return self.comment_lines(CommentMode::Remove),
            Command::BlockComment => return self.block_comment(false),
            Command::BlockUncomment => return self.block_comment(true),
            Command::ToggleBookmark => self.toggle_bookmarks(),
            Command::NextBookmark | Command::PreviousBookmark => {
                if !self.step_bookmark(cmd == Command::PreviousBookmark) {
                    self.notice = Some("No bookmark.".into());
                }
            }
            Command::ClearBookmarks => self.clear_bookmarks(),
            Command::InvertBookmarks => self.invert_bookmarks(),
            Command::RemoveBookmarkedLines | Command::RemoveUnmarkedLines => {
                if self.remove_lines_by_bookmark(cmd == Command::RemoveBookmarkedLines) == 0 {
                    self.notice = Some("No line to remove.".into());
                }
            }
            Command::NextChange | Command::PreviousChange => {
                if !self.step_change(cmd == Command::PreviousChange) {
                    self.notice = Some("No change.".into());
                }
            }
            Command::ClearChanges => self.clear_changes(),
            Command::HideLines => {
                if self.hide_lines() == 0 {
                    self.notice = Some("The first line cannot be hidden.".into());
                }
            }
            Command::ShowHiddenLines => self.show_lines(None),
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
                c.id.chars().all(|ch| ch.is_ascii_lowercase()
                    || ch.is_ascii_digit()
                    || ch == '.'
                    || ch == '-'),
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
