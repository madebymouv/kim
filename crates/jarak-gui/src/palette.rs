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
    /// Step 96: the multi-selection commands (see `JarakApp::multi`).
    Multi(u8),
    /// Step 97: dates, paths, digests and the copy-line setting (see `JarakApp::edit_tool`).
    Edit(u8),
    /// Step 98: the Language menu (0 detect, 1 plain text, then the languages) and the user
    /// language tools (see `JarakApp::udl_tool`).
    Language(u16),
    Udl(u8),
    /// Step 99: 0 the Function List on/off, 1 the next symbol, 2 the previous one.
    Outline(u8),
    /// Step 100: the assistance settings and files (see `JarakApp::assist_tool`).
    Assist(u8),
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
    Language,
}

impl Menu {
    pub const ALL: [Menu; 9] = [
        Menu::File,
        Menu::Edit,
        Menu::Search,
        Menu::Selection,
        Menu::Go,
        Menu::View,
        Menu::Window,
        Menu::Markdown,
        Menu::Language,
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
            Menu::Language => ("Language", "언어"),
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
    v.extend([
        act(
            "edit.add-next-occurrence",
            "Multi-select: add the next occurrence",
            "다중 선택: 다음 위치 추가",
            A::AddNextOccurrence,
            M::Selection,
        ),
        act(
            "edit.select-all-occurrences",
            "Multi-select: all occurrences",
            "다중 선택: 모든 위치",
            A::SelectAllOccurrences,
            M::Selection,
        ),
        act(
            "edit.single-caret",
            "Back to one caret",
            "캐럿 하나로",
            A::SingleCaret,
            M::Selection,
        ),
        act(
            "edit.column-editor",
            "Column Editor…",
            "열 편집기…",
            A::ColumnEditor,
            M::Edit,
        ),
        act(
            "edit.block-up",
            "Column selection: up",
            "열 선택: 위",
            A::BlockUp,
            M::Selection,
        ),
        act(
            "edit.block-down",
            "Column selection: down",
            "열 선택: 아래",
            A::BlockDown,
            M::Selection,
        ),
        act(
            "edit.block-left",
            "Column selection: left",
            "열 선택: 왼쪽",
            A::BlockLeft,
            M::Selection,
        ),
        act(
            "edit.block-right",
            "Column selection: right",
            "열 선택: 오른쪽",
            A::BlockRight,
            M::Selection,
        ),
        run(
            "edit.all-ignore",
            "Multi-select all: ignore case and whole word",
            "모두 다중 선택: 대소문자·단어 무시",
            Run::Multi(2),
            M::Selection,
        ),
        run(
            "edit.all-case",
            "Multi-select all: match case only",
            "모두 다중 선택: 대소문자만 구분",
            Run::Multi(3),
            M::Selection,
        ),
        run(
            "edit.all-word",
            "Multi-select all: whole word only",
            "모두 다중 선택: 단어 단위만",
            Run::Multi(4),
            M::Selection,
        ),
        run(
            "edit.all-case-word",
            "Multi-select all: match case and whole word",
            "모두 다중 선택: 대소문자·단어 단위",
            Run::Multi(5),
            M::Selection,
        ),
        run(
            "edit.next-ignore",
            "Multi-select next: ignore case and whole word",
            "다음 다중 선택: 대소문자·단어 무시",
            Run::Multi(6),
            M::Selection,
        ),
        run(
            "edit.next-case",
            "Multi-select next: match case only",
            "다음 다중 선택: 대소문자만 구분",
            Run::Multi(7),
            M::Selection,
        ),
        run(
            "edit.next-word",
            "Multi-select next: whole word only",
            "다음 다중 선택: 단어 단위만",
            Run::Multi(8),
            M::Selection,
        ),
        run(
            "edit.next-case-word",
            "Multi-select next: match case and whole word",
            "다음 다중 선택: 대소문자·단어 단위",
            Run::Multi(9),
            M::Selection,
        ),
        run(
            "edit.skip-occurrence",
            "Multi-select: skip this occurrence and add the next",
            "다중 선택: 이곳 건너뛰고 다음 추가",
            Run::Multi(10),
            M::Selection,
        ),
        run(
            "edit.begin-end-select",
            "Begin/End Select",
            "선택 시작/끝",
            Run::Multi(11),
            M::Selection,
        ),
        run(
            "edit.begin-end-select-column",
            "Begin/End Select in column mode",
            "열 모드로 선택 시작/끝",
            Run::Multi(12),
            M::Selection,
        ),
        run(
            "edit.virtual-space",
            "Virtual space in column selections (on/off)",
            "열 선택의 가상 공간 (켬/끔)",
            Run::Multi(13),
            M::Selection,
        ),
    ]);
    v.extend([
        run(
            "edit.date-short",
            "Insert the date and time (short)",
            "날짜와 시간 넣기(짧게)",
            Run::Edit(0),
            M::Edit,
        ),
        run(
            "edit.date-long",
            "Insert the date and time (long)",
            "날짜와 시간 넣기(길게)",
            Run::Edit(1),
            M::Edit,
        ),
        run(
            "edit.date-custom",
            "Insert the date and time (custom format)",
            "날짜와 시간 넣기(사용자 형식)",
            Run::Edit(2),
            M::Edit,
        ),
        run(
            "edit.copy-full-path",
            "Copy the full path",
            "전체 경로 복사",
            Run::Edit(3),
            M::Edit,
        ),
        run(
            "edit.copy-file-name",
            "Copy the file name",
            "파일 이름 복사",
            Run::Edit(4),
            M::Edit,
        ),
        run(
            "edit.copy-directory",
            "Copy the directory",
            "폴더 경로 복사",
            Run::Edit(5),
            M::Edit,
        ),
        run(
            "edit.copy-all-names",
            "Copy the names of all open documents",
            "열린 문서 모두의 이름 복사",
            Run::Edit(6),
            M::Edit,
        ),
        run(
            "edit.copy-all-paths",
            "Copy the paths of all open documents",
            "열린 문서 모두의 경로 복사",
            Run::Edit(7),
            M::Edit,
        ),
        run(
            "tools.md5-selection",
            "MD5 of the selection to the clipboard",
            "선택의 MD5를 클립보드로",
            Run::Edit(8),
            M::Edit,
        ),
        run(
            "tools.sha1-selection",
            "SHA-1 of the selection to the clipboard",
            "선택의 SHA-1을 클립보드로",
            Run::Edit(9),
            M::Edit,
        ),
        run(
            "tools.sha256-selection",
            "SHA-256 of the selection to the clipboard",
            "선택의 SHA-256을 클립보드로",
            Run::Edit(10),
            M::Edit,
        ),
        run(
            "tools.sha512-selection",
            "SHA-512 of the selection to the clipboard",
            "선택의 SHA-512를 클립보드로",
            Run::Edit(11),
            M::Edit,
        ),
        run(
            "tools.md5-file",
            "MD5 of a file to the clipboard…",
            "파일의 MD5를 클립보드로…",
            Run::Edit(12),
            M::Edit,
        ),
        run(
            "tools.sha1-file",
            "SHA-1 of a file to the clipboard…",
            "파일의 SHA-1을 클립보드로…",
            Run::Edit(13),
            M::Edit,
        ),
        run(
            "tools.sha256-file",
            "SHA-256 of a file to the clipboard…",
            "파일의 SHA-256을 클립보드로…",
            Run::Edit(14),
            M::Edit,
        ),
        run(
            "tools.sha512-file",
            "SHA-512 of a file to the clipboard…",
            "파일의 SHA-512를 클립보드로…",
            Run::Edit(15),
            M::Edit,
        ),
        run(
            "tools.hash-generate",
            "Generate a digest (MD5, SHA-1, SHA-256, SHA-512)…",
            "다이제스트 만들기(MD5, SHA-1, SHA-256, SHA-512)…",
            Run::Edit(16),
            M::Edit,
        ),
        run(
            "edit.copy-line-setting",
            "Copy the line when nothing is selected (on/off)",
            "선택이 없을 때 줄 복사 (켬/끔)",
            Run::Edit(17),
            M::Edit,
        ),
    ]);
    v.extend([
        run(
            "language.import-udl",
            "Import a Notepad++ user language (XML)…",
            "Notepad++ 사용자 언어 가져오기(XML)…",
            Run::Udl(0),
            M::Language,
        ),
        run(
            "language.export-udl",
            "Export the language as a user language (JSON)…",
            "언어를 사용자 언어로 내보내기(JSON)…",
            Run::Udl(1),
            M::Language,
        ),
        run(
            "language.open-folder",
            "Open the user languages folder",
            "사용자 언어 폴더 열기",
            Run::Udl(2),
            M::Language,
        ),
        run(
            "language.reload",
            "Read the user languages again",
            "사용자 언어 다시 읽기",
            Run::Udl(3),
            M::Language,
        ),
    ]);
    // Step 99: folding, the Function List and structure navigation.
    v.extend([
        act(
            "view.fold-all",
            "Fold all",
            "모두 접기",
            A::Fold(0, true),
            M::View,
        ),
        act(
            "view.unfold-all",
            "Unfold all",
            "모두 펼치기",
            A::Fold(0, false),
            M::View,
        ),
        act(
            "view.fold-current",
            "Fold the current level",
            "현재 단계 접기",
            A::Fold(9, true),
            M::View,
        ),
        act(
            "view.unfold-current",
            "Unfold the current level",
            "현재 단계 펼치기",
            A::Fold(9, false),
            M::View,
        ),
    ]);
    const FOLD_IDS: [&str; 8] = [
        "view.fold-level-1",
        "view.fold-level-2",
        "view.fold-level-3",
        "view.fold-level-4",
        "view.fold-level-5",
        "view.fold-level-6",
        "view.fold-level-7",
        "view.fold-level-8",
    ];
    const UNFOLD_IDS: [&str; 8] = [
        "view.unfold-level-1",
        "view.unfold-level-2",
        "view.unfold-level-3",
        "view.unfold-level-4",
        "view.unfold-level-5",
        "view.unfold-level-6",
        "view.unfold-level-7",
        "view.unfold-level-8",
    ];
    for n in 1..=8u8 {
        for (on, ids) in [(true, FOLD_IDS), (false, UNFOLD_IDS)] {
            let a = A::Fold(n, on);
            v.push(Entry {
                id: ids[usize::from(n) - 1],
                en: Cow::Owned(format!("{} level {n}", if on { "Fold" } else { "Unfold" })),
                ko: Cow::Owned(format!("{n}단계 {}", if on { "접기" } else { "펼치기" })),
                keys: key_label(a),
                run: Run::Act(a),
                menu: M::View,
            });
        }
    }
    v.extend([
        run(
            "view.function-list",
            "Function list",
            "함수 목록",
            Run::Outline(0),
            M::View,
        ),
        act("go.line", "Go to line…", "줄로 이동…", A::GoTo, M::Go),
        act(
            "go.matching-brace",
            "Go to the matching bracket",
            "짝 괄호로 이동",
            A::MatchBrace,
            M::Go,
        ),
        act(
            "edit.select-braces",
            "Select everything between the matching brackets",
            "짝 괄호 사이 모두 선택",
            A::SelectBraces,
            M::Selection,
        ),
        run(
            "go.next-symbol",
            "Next symbol",
            "다음 심볼",
            Run::Outline(1),
            M::Go,
        ),
        // Step 100: completion and the typing aids.
        act(
            "edit.complete",
            "Complete the word",
            "단어 완성",
            A::Complete(0),
            M::Edit,
        ),
        act(
            "edit.complete-words",
            "Complete from the document's words",
            "문서의 단어로 완성",
            A::Complete(1),
            M::Edit,
        ),
        act(
            "edit.complete-path",
            "Complete a path",
            "경로 완성",
            A::Complete(2),
            M::Edit,
        ),
        act(
            "edit.parameter-hint",
            "Parameter hint",
            "매개변수 힌트",
            A::Complete(3),
            M::Edit,
        ),
        run(
            "edit.assist-auto",
            "Completion while typing: on/off",
            "입력 중 자동 완성 켜기/끄기",
            Run::Assist(0),
            M::Edit,
        ),
        run(
            "edit.assist-enter-never",
            "Enter never accepts a completion",
            "Enter로 완성 받지 않기",
            Run::Assist(1),
            M::Edit,
        ),
        run(
            "edit.assist-enter-chosen",
            "Enter accepts a completion chosen with the arrow keys",
            "화살표로 고른 완성만 Enter로 받기",
            Run::Assist(2),
            M::Edit,
        ),
        run(
            "edit.assist-enter-always",
            "Enter always accepts a completion",
            "Enter로 항상 완성 받기",
            Run::Assist(3),
            M::Edit,
        ),
        run(
            "edit.assist-tab-never",
            "Tab never accepts a completion",
            "Tab으로 완성 받지 않기",
            Run::Assist(15),
            M::Edit,
        ),
        run(
            "edit.assist-tab-chosen",
            "Tab accepts a completion chosen with the arrow keys",
            "화살표로 고른 완성만 Tab으로 받기",
            Run::Assist(16),
            M::Edit,
        ),
        run(
            "edit.assist-tab-always",
            "Tab always accepts a completion",
            "Tab으로 항상 완성 받기",
            Run::Assist(17),
            M::Edit,
        ),
        run(
            "edit.assist-hints",
            "Parameter hints: on/off",
            "매개변수 힌트 켜기/끄기",
            Run::Assist(4),
            M::Edit,
        ),
        run(
            "edit.assist-snippets",
            "Snippets in the completion list: on/off",
            "완성 목록의 스니펫 켜기/끄기",
            Run::Assist(5),
            M::Edit,
        ),
        run(
            "edit.assist-close",
            "Close brackets and quotes: on/off",
            "괄호와 따옴표 자동 닫기 켜기/끄기",
            Run::Assist(6),
            M::Edit,
        ),
        run(
            "edit.assist-tags",
            "Close HTML/XML tags: on/off",
            "HTML/XML 태그 자동 닫기 켜기/끄기",
            Run::Assist(7),
            M::Edit,
        ),
        run(
            "edit.indent-off",
            "Indentation on Enter: off",
            "Enter 들여쓰기: 끔",
            Run::Assist(8),
            M::Edit,
        ),
        run(
            "edit.indent-keep",
            "Indentation on Enter: keep the line's",
            "Enter 들여쓰기: 줄의 들여쓰기 유지",
            Run::Assist(9),
            M::Edit,
        ),
        run(
            "edit.indent-smart",
            "Indentation on Enter: smart",
            "Enter 들여쓰기: 똑똑하게",
            Run::Assist(10),
            M::Edit,
        ),
        run(
            "view.brace-highlight",
            "Highlight matching brackets: on/off",
            "짝 괄호 강조 켜기/끄기",
            Run::Assist(11),
            M::View,
        ),
        run(
            "edit.snippets-file",
            "Edit the snippets file",
            "스니펫 파일 편집",
            Run::Assist(12),
            M::Edit,
        ),
        run(
            "edit.api-folder",
            "Open the API files folder (Notepad++ autoCompletion)",
            "API 파일 폴더 열기(Notepad++ autoCompletion)",
            Run::Assist(13),
            M::Edit,
        ),
        run(
            "edit.assist-reload",
            "Read the snippets and API files again",
            "스니펫과 API 파일 다시 읽기",
            Run::Assist(14),
            M::Edit,
        ),
        run(
            "go.previous-symbol",
            "Previous symbol",
            "이전 심볼",
            Run::Outline(2),
            M::Go,
        ),
    ]);
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
        // Every window action shows a key; a registry command shows its key when it has one
        // (Step 97: many are in the palette and the menus only).
        for x in &e {
            match x.run {
                Run::Act(Action::Cmd(c)) => {
                    assert_eq!(x.keys, key_label(Action::Cmd(c)), "{}", x.id);
                }
                Run::Act(_) => assert!(x.keys.is_some(), "{}", x.id),
                _ => {}
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
