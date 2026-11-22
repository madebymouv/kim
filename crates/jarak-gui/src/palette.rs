//! The command palette (Ctrl+Shift+P, `JARAK-FR-054`) and the menu bar (Alt or the palette,
//! UI-10), Step 92: every command of the window and of the editing registry, with its keys.
//!
//! One list feeds both: the palette filters it by words (English, Korean, the command's id or
//! its keys) and runs the chosen command; the menu bar groups it by menu. The keys come from
//! [`crate::keys::key_label`], checked against the real bindings by a test.

use crate::Editing;
use crate::keys::{Action, key_label};
use crate::tabs::Menu as TabMenu;
use crate::workspace::Side;
use jarak_core::command::COMMANDS;
use std::borrow::Cow;

/// What a palette entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Act(Action),
    Editing(Editing),
    MenuBar,
    Assets,
    /// Step 93: a tab command on the active document.
    Tab(TabMenu),
    /// The `n`th recent file.
    Recent(usize),
    RecentAll,
    RecentClear,
    Orientation,
    SyncV,
    SyncH,
    Side(Side),
    Documents,
    RestoreSession,
    SessionSave,
    SessionLoad,
    /// Step 94: 0 the Mark page, 1 Mark All, 2 clear marks, 3 copy marked text, 4 count.
    FindCmd(u8),
    Style(crate::find::StyleCmd),
    /// Step 95: 0 Find All in the current document, 1 in every open document, 2 Replace All in
    /// every open document, 3 undo the last Replace in Files, 4 copy the result lines, 5 their
    /// paths, 6 Find in Files, 7 Replace in Files (the Files page's folder and filters).
    InFiles(u8),
}

/// The menu an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Search,
    Selection,
    Go,
    View,
    Window,
    Markdown,
}

impl Menu {
    pub const ALL: [Menu; 8] = [
        Menu::File,
        Menu::Edit,
        Menu::Search,
        Menu::Selection,
        Menu::Go,
        Menu::View,
        Menu::Window,
        Menu::Markdown,
    ];

    /// The menu's name (English, Korean).
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            Menu::File => ("File", "파일"),
            Menu::Edit => ("Edit", "편집"),
            Menu::Search => ("Search", "찾기"),
            Menu::Selection => ("Selection", "선택"),
            Menu::Go => ("Go", "이동"),
            Menu::View => ("View", "보기"),
            Menu::Window => ("Window", "창"),
            Menu::Markdown => ("Markdown", "Markdown"),
        }
    }
}

/// A command as the palette and the menus show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: &'static str,
    pub en: Cow<'static, str>,
    pub ko: Cow<'static, str>,
    pub keys: Option<&'static str>,
    pub run: Run,
    pub menu: Menu,
}

fn run(id: &'static str, en: &'static str, ko: &'static str, r: Run, menu: Menu) -> Entry {
    Entry {
        id,
        en: Cow::Borrowed(en),
        ko: Cow::Borrowed(ko),
        keys: None,
        run: r,
        menu,
    }
}

fn act(id: &'static str, en: &'static str, ko: &'static str, a: Action, menu: Menu) -> Entry {
    Entry {
        id,
        en: Cow::Borrowed(en),
        ko: Cow::Borrowed(ko),
        keys: key_label(a),
        run: Run::Act(a),
        menu,
    }
}

/// Every command, in menu order.
pub fn entries() -> Vec<Entry> {
    use Action as A;
    use Menu as M;
    let mut v = vec![
        act("file.new", "New document", "새 문서", A::New, M::File),
        act("file.open", "Open…", "열기…", A::Open, M::File),
        act("file.save", "Save", "저장", A::Save, M::File),
        act(
            "file.save-as",
            "Save as…",
            "다른 이름으로 저장…",
            A::SaveAs,
            M::File,
        ),
        act(
            "file.close",
            "Close the document",
            "문서 닫기",
            A::Close,
            M::File,
        ),
        act(
            "file.next",
            "Next document",
            "다음 문서",
            A::NextDocument,
            M::File,
        ),
        act(
            "file.previous",
            "Previous document",
            "이전 문서",
            A::PreviousDocument,
            M::File,
        ),
        run(
            "file.save-all",
            "Save all",
            "모두 저장",
            Run::Tab(TabMenu::SaveAll),
            M::File,
        ),
        run(
            "file.close-others",
            "Close the other documents",
            "다른 문서 닫기",
            Run::Tab(TabMenu::CloseOthers),
            M::File,
        ),
        run(
            "file.close-unchanged",
            "Close the unchanged documents",
            "변경 없는 문서 닫기",
            Run::Tab(TabMenu::CloseUnchanged),
            M::File,
        ),
        run(
            "file.close-all",
            "Close all documents",
            "모든 문서 닫기",
            Run::Tab(TabMenu::CloseAll),
            M::File,
        ),
        act(
            "file.reopen",
            "Reopen the closed document",
            "닫은 문서 다시 열기",
            A::ReopenClosed,
            M::File,
        ),
        run(
            "file.recent-all",
            "Open all recent files",
            "최근 파일 모두 열기",
            Run::RecentAll,
            M::File,
        ),
        run(
            "file.recent-clear",
            "Empty the recent files list",
            "최근 파일 목록 비우기",
            Run::RecentClear,
            M::File,
        ),
        run(
            "file.rename",
            "Rename the file…",
            "파일 이름 바꾸기…",
            Run::Tab(TabMenu::Rename),
            M::File,
        ),
        run(
            "file.recycle",
            "Move the file to the Recycle Bin…",
            "파일을 휴지통으로…",
            Run::Tab(TabMenu::Recycle),
            M::File,
        ),
        run(
            "file.show-folder",
            "Show the file in Explorer",
            "탐색기에서 파일 보기",
            Run::Tab(TabMenu::OpenFolder),
            M::File,
        ),
        run(
            "file.terminal",
            "Open a command prompt in the file's folder",
            "파일 폴더에서 명령 프롬프트 열기",
            Run::Tab(TabMenu::OpenTerminal),
            M::File,
        ),
        run(
            "file.default-program",
            "Open the file in its default program",
            "기본 프로그램으로 파일 열기",
            Run::Tab(TabMenu::OpenDefault),
            M::File,
        ),
        run(
            "file.copy-path",
            "Copy the full path",
            "전체 경로 복사",
            Run::Tab(TabMenu::CopyPath),
            M::File,
        ),
        run(
            "file.session-save",
            "Save the session to a file…",
            "세션을 파일로 저장…",
            Run::SessionSave,
            M::File,
        ),
        run(
            "file.session-load",
            "Open a session file…",
            "세션 파일 열기…",
            Run::SessionLoad,
            M::File,
        ),
        run(
            "file.restore-session",
            "Restore the session at start",
            "시작할 때 세션 복원",
            Run::RestoreSession,
            M::File,
        ),
        act("edit.cut", "Cut", "잘라내기", A::Cut, M::Edit),
        act("edit.copy", "Copy", "복사", A::Copy, M::Edit),
        act(
            "edit.paste",
            "Paste (text or an image)",
            "붙여넣기(텍스트 또는 이미지)",
            A::Paste,
            M::Edit,
        ),
        act("edit.indent", "Indent", "들여쓰기", A::Indent, M::Edit),
        act("edit.outdent", "Outdent", "내어쓰기", A::Outdent, M::Edit),
        act(
            "edit.overwrite",
            "Overwrite mode",
            "겹쳐 쓰기",
            A::ToggleOverwrite,
            M::Edit,
        ),
        Entry {
            id: "editing.vim",
            en: Cow::Borrowed("Editing mode: Vim"),
            ko: Cow::Borrowed("편집 모드: Vim"),
            keys: None,
            run: Run::Editing(Editing::Vim),
            menu: M::Edit,
        },
        Entry {
            id: "editing.standard",
            en: Cow::Borrowed("Editing mode: Standard"),
            ko: Cow::Borrowed("편집 모드: 표준"),
            keys: None,
            run: Run::Editing(Editing::Standard),
            menu: M::Edit,
        },
        act("view.wrap", "Word wrap", "줄 바꿈", A::ToggleWrap, M::View),
        act("view.zoom-in", "Zoom in", "확대", A::ZoomIn, M::View),
        act("view.zoom-out", "Zoom out", "축소", A::ZoomOut, M::View),
        act(
            "view.zoom-reset",
            "Reset the zoom",
            "확대 초기화",
            A::ZoomReset,
            M::View,
        ),
        Entry {
            id: "view.menu-bar",
            en: Cow::Borrowed("Menu bar"),
            ko: Cow::Borrowed("메뉴 막대"),
            keys: Some("Alt"),
            run: Run::MenuBar,
            menu: M::View,
        },
        act(
            "view.palette",
            "Command palette",
            "명령 팔레트",
            A::Palette,
            M::View,
        ),
        run(
            "view.folder",
            "Folder as workspace",
            "작업 공간 폴더",
            Run::Side(Side::Folder),
            M::View,
        ),
        run(
            "view.documents",
            "Document list",
            "문서 목록",
            Run::Side(Side::Documents),
            M::View,
        ),
        run(
            "window.move-other",
            "Move the document to the other view",
            "문서를 다른 보기로 옮기기",
            Run::Tab(TabMenu::MoveOther),
            M::Window,
        ),
        run(
            "window.clone-other",
            "Clone the document to the other view",
            "문서를 다른 보기에 복제",
            Run::Tab(TabMenu::CloneOther),
            M::Window,
        ),
        act(
            "window.focus-other",
            "Focus the other view",
            "다른 보기로 이동",
            A::FocusOther,
            M::Window,
        ),
        run(
            "window.orientation",
            "Views side by side / one above the other",
            "보기 나란히 / 위아래",
            Run::Orientation,
            M::Window,
        ),
        run(
            "window.sync-v",
            "Synchronize vertical scrolling",
            "세로 스크롤 맞추기",
            Run::SyncV,
            M::Window,
        ),
        run(
            "window.sync-h",
            "Synchronize horizontal scrolling",
            "가로 스크롤 맞추기",
            Run::SyncH,
            M::Window,
        ),
        act(
            "window.tab-left",
            "Move the tab left",
            "탭을 왼쪽으로",
            A::MoveTabLeft,
            M::Window,
        ),
        act(
            "window.tab-right",
            "Move the tab right",
            "탭을 오른쪽으로",
            A::MoveTabRight,
            M::Window,
        ),
        act(
            "window.tab-first",
            "Go to the first tab",
            "첫 탭으로",
            A::GoToTab(1),
            M::Window,
        ),
        act(
            "window.tab-last",
            "Go to the last tab",
            "마지막 탭으로",
            A::GoToTab(9),
            M::Window,
        ),
        run(
            "window.new-window",
            "Open the document in a new window",
            "문서를 새 창에서 열기",
            Run::Tab(TabMenu::NewWindow),
            M::Window,
        ),
        run(
            "window.documents",
            "Documents…",
            "문서…",
            Run::Documents,
            M::Window,
        ),
        act(
            "markdown.preview",
            "Source / preview",
            "원문 / 미리보기",
            A::TogglePreview,
            M::Markdown,
        ),
        act(
            "markdown.split",
            "Source and preview side by side",
            "원문과 미리보기 나란히",
            A::ToggleSplit,
            M::Markdown,
        ),
        Entry {
            id: "markdown.assets",
            en: Cow::Borrowed("Images and orphaned assets…"),
            ko: Cow::Borrowed("이미지와 고아 asset…"),
            keys: None,
            run: Run::Assets,
            menu: M::Markdown,
        },
    ];
    use crate::find::StyleCmd as S;
    v.extend([
        act("search.find", "Find…", "찾기…", A::Find, M::Search),
        act(
            "search.replace",
            "Replace…",
            "바꾸기…",
            A::Replace,
            M::Search,
        ),
        act(
            "search.next",
            "Find next",
            "다음 찾기",
            A::FindNext,
            M::Search,
        ),
        act(
            "search.previous",
            "Find previous",
            "이전 찾기",
            A::FindPrevious,
            M::Search,
        ),
        act(
            "search.select-next",
            "Select and find next",
            "선택해서 다음 찾기",
            A::SelectFindNext,
            M::Search,
        ),
        act(
            "search.select-previous",
            "Select and find previous",
            "선택해서 이전 찾기",
            A::SelectFindPrevious,
            M::Search,
        ),
        act(
            "search.word-next",
            "Find the word under the caret (next)",
            "커서 단어 다음 찾기",
            A::VolatileNext,
            M::Search,
        ),
        act(
            "search.word-previous",
            "Find the word under the caret (previous)",
            "커서 단어 이전 찾기",
            A::VolatilePrevious,
            M::Search,
        ),
        act(
            "search.incremental",
            "Incremental search",
            "점진적 찾기",
            A::Incremental,
            M::Search,
        ),
        act(
            "search.in-files",
            "Find in files…",
            "파일에서 찾기…",
            A::FindInFiles,
            M::Search,
        ),
        act(
            "search.next-result",
            "Next search result",
            "다음 찾기 결과",
            A::NextResult,
            M::Search,
        ),
        act(
            "search.previous-result",
            "Previous search result",
            "이전 찾기 결과",
            A::PreviousResult,
            M::Search,
        ),
        act(
            "search.results",
            "Search results panel",
            "찾기 결과 패널",
            A::ToggleResults,
            M::Search,
        ),
        run(
            "search.all-in-document",
            "Find all in the current document",
            "현재 문서에서 모두 찾기",
            Run::InFiles(0),
            M::Search,
        ),
        run(
            "search.all-in-open",
            "Find all in all open documents",
            "열린 문서 모두에서 찾기",
            Run::InFiles(1),
            M::Search,
        ),
        run(
            "search.replace-in-open",
            "Replace all in all open documents",
            "열린 문서 모두에서 바꾸기",
            Run::InFiles(2),
            M::Search,
        ),
        run(
            "search.undo-in-files",
            "Undo the last Replace in Files",
            "마지막 파일 바꾸기 되돌리기",
            Run::InFiles(3),
            M::Search,
        ),
        run(
            "search.copy-results",
            "Copy the search result lines",
            "찾기 결과 줄 복사",
            Run::InFiles(4),
            M::Search,
        ),
        run(
            "search.copy-result-paths",
            "Copy the search result paths",
            "찾기 결과 경로 복사",
            Run::InFiles(5),
            M::Search,
        ),
        run(
            "search.find-all-in-files",
            "Find all in files (the Files page's folder)",
            "파일에서 모두 찾기 (파일 페이지의 폴더)",
            Run::InFiles(6),
            M::Search,
        ),
        run(
            "search.replace-in-files",
            "Replace in files…",
            "파일에서 바꾸기…",
            Run::InFiles(7),
            M::Search,
        ),
        run(
            "search.count",
            "Count the matches",
            "찾은 개수 세기",
            Run::FindCmd(4),
            M::Search,
        ),
        run("search.mark", "Mark…", "표시…", Run::FindCmd(0), M::Search),
        run(
            "search.mark-all",
            "Mark all",
            "모두 표시",
            Run::FindCmd(1),
            M::Search,
        ),
        run(
            "search.clear-marks",
            "Clear all marks",
            "모든 표시 지우기",
            Run::FindCmd(2),
            M::Search,
        ),
        run(
            "search.copy-marked",
            "Copy the marked text",
            "표시한 텍스트 복사",
            Run::FindCmd(3),
            M::Search,
        ),
        run(
            "search.style-1",
            "Style all occurrences of the token: style 1",
            "토큰 모두 스타일: 1",
            Run::Style(S::All(1)),
            M::Search,
        ),
        run(
            "search.style-2",
            "Style all occurrences of the token: style 2",
            "토큰 모두 스타일: 2",
            Run::Style(S::All(2)),
            M::Search,
        ),
        run(
            "search.style-3",
            "Style all occurrences of the token: style 3",
            "토큰 모두 스타일: 3",
            Run::Style(S::All(3)),
            M::Search,
        ),
        run(
            "search.style-4",
            "Style all occurrences of the token: style 4",
            "토큰 모두 스타일: 4",
            Run::Style(S::All(4)),
            M::Search,
        ),
        run(
            "search.style-5",
            "Style all occurrences of the token: style 5",
            "토큰 모두 스타일: 5",
            Run::Style(S::All(5)),
            M::Search,
        ),
        run(
            "search.style-one-1",
            "Style one token: style 1",
            "토큰 하나 스타일: 1",
            Run::Style(S::One(1)),
            M::Search,
        ),
        run(
            "search.style-one-2",
            "Style one token: style 2",
            "토큰 하나 스타일: 2",
            Run::Style(S::One(2)),
            M::Search,
        ),
        run(
            "search.style-one-3",
            "Style one token: style 3",
            "토큰 하나 스타일: 3",
            Run::Style(S::One(3)),
            M::Search,
        ),
        run(
            "search.style-one-4",
            "Style one token: style 4",
            "토큰 하나 스타일: 4",
            Run::Style(S::One(4)),
            M::Search,
        ),
        run(
            "search.style-one-5",
            "Style one token: style 5",
            "토큰 하나 스타일: 5",
            Run::Style(S::One(5)),
            M::Search,
        ),
        run(
            "search.clear-style-1",
            "Clear style 1",
            "스타일 1 지우기",
            Run::Style(S::Clear(1)),
            M::Search,
        ),
        run(
            "search.clear-style-2",
            "Clear style 2",
            "스타일 2 지우기",
            Run::Style(S::Clear(2)),
            M::Search,
        ),
        run(
            "search.clear-style-3",
            "Clear style 3",
            "스타일 3 지우기",
            Run::Style(S::Clear(3)),
            M::Search,
        ),
        run(
            "search.clear-style-4",
            "Clear style 4",
            "스타일 4 지우기",
            Run::Style(S::Clear(4)),
            M::Search,
        ),
        run(
            "search.clear-style-5",
            "Clear style 5",
            "스타일 5 지우기",
            Run::Style(S::Clear(5)),
            M::Search,
        ),
        run(
            "search.clear-styles",
            "Clear all styles",
            "모든 스타일 지우기",
            Run::Style(S::Clear(0)),
            M::Search,
        ),
        run(
            "search.styled-next",
            "Jump to the next styled text",
            "다음 스타일 텍스트로",
            Run::Style(S::Next),
            M::Search,
        ),
        run(
            "search.styled-previous",
            "Jump to the previous styled text",
            "이전 스타일 텍스트로",
            Run::Style(S::Previous),
            M::Search,
        ),
        run(
            "search.copy-styled",
            "Copy the styled text",
            "스타일 텍스트 복사",
            Run::Style(S::Copy),
            M::Search,
        ),
    ]);
    for c in COMMANDS {
        let menu = if c.id.starts_with("select.") {
            M::Selection
        } else if c.id.starts_with("cursor.") {
            M::Go
        } else {
            M::Edit
        };
        v.push(act(c.id, c.en, c.ko, Action::Cmd(c.command), menu));
    }
    v
}

/// The entries whose name, id or keys hold every word of `query` (any case), in list order.
pub fn filter(entries: &[Entry], query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let phrase = words.join(" ");
    let mut found: Vec<(u8, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let (en, ko) = (e.en.to_lowercase(), e.ko.to_lowercase());
            let hay = format!("{en} {ko} {} {}", e.id, e.keys.unwrap_or("").to_lowercase());
            if !words.iter().all(|w| hay.contains(w.as_str())) {
                return None;
            }
            // A label that starts with the query first, then one holding it as a phrase.
            let rank = if en.starts_with(&phrase) || ko.starts_with(&phrase) {
                0
            } else if en.contains(&phrase) || ko.contains(&phrase) {
                1
            } else {
                2
            };
            Some((rank, i))
        })
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, i)| i).collect()
}

/// The palette while it is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaletteState {
    pub query: String,
    pub selected: usize,
    /// The search field takes the focus on the next frame.
    pub focus: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_starting_with_the_query_comes_first() {
        let e = entries();
        let first = |q: &str| e[filter(&e, q)[0]].id;
        assert_eq!(first("Replace in files"), "search.replace-in-files");
        assert_eq!(first("next search result"), "search.next-result");
        assert_eq!(first("파일에서 바꾸기"), "search.replace-in-files");
    }

    #[test]
    fn every_registry_command_and_window_action_is_listed_with_its_keys() {
        let e = entries();
        for c in COMMANDS {
            assert!(e.iter().any(|x| x.id == c.id), "{}", c.id);
        }
        let ids: Vec<&str> = e.iter().map(|x| x.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "ids are unique");
        // Every action entry shows a key; the others say how they are reached.
        for x in &e {
            if let Run::Act(_) = x.run {
                assert!(x.keys.is_some(), "{}", x.id);
            }
        }
        assert_eq!(
            e.iter().find(|x| x.id == "file.save").unwrap().keys,
            Some("Ctrl+S")
        );
    }

    #[test]
    fn the_filter_matches_words_in_any_language_id_or_key() {
        let e = entries();
        let names = |q: &str| -> Vec<&str> { filter(&e, q).into_iter().map(|i| e[i].id).collect() };
        assert_eq!(names("save as"), ["file.save-as"]);
        assert!(names("vim").contains(&"editing.vim"));
        assert_eq!(names("다른 이름"), ["file.save-as"]);
        assert!(names("ctrl+e").contains(&"markdown.preview"));
        assert_eq!(names("select.all"), ["select.all"]);
        assert_eq!(filter(&e, "").len(), e.len());
        assert!(names("zzzz").is_empty());
    }
}
