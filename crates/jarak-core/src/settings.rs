//! Jarak's settings (Step 105, `JARAK-FR-130`, SET-01…26): every setting the window and the CLI
//! know, in one registry — its key in `settings.toml`, the Settings section it belongs to, its
//! English and Korean names and help, what values it takes and its default.
//!
//! - The file is `%APPDATA%\mouv\jarak\settings.toml` (roaming, `JARAK-SEC-008`): one
//!   `key = value` line per setting the user changed (booleans and numbers bare, text quoted).
//!   Lines Jarak does not know, comments and the order are kept when a value is written; a write
//!   goes through a temporary file and a rename (atomic), so the file is never half written.
//! - A value is checked against its setting before it is written ([`validate`]); the window and
//!   `jarak settings set` refuse a value the setting does not take, and an unknown key.
//! - Reading never fails: a missing file or line, or a value the setting does not take, is the
//!   default ([`Settings::get`]).
//!
//! The schema of the CLI's view of the settings is `docs/contracts/schemas/jarak-settings-v1.schema.json`;
//! the reference is `docs/JARAK_SETTINGS.md`.

use std::io;
use std::path::{Path, PathBuf};

/// The Settings window's sections, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    General,
    Editor,
    Appearance,
    Markdown,
    Images,
    Vim,
    Keyboard,
    Files,
    Advanced,
}

impl Section {
    pub const ALL: [Section; 9] = [
        Section::General,
        Section::Editor,
        Section::Appearance,
        Section::Markdown,
        Section::Images,
        Section::Vim,
        Section::Keyboard,
        Section::Files,
        Section::Advanced,
    ];

    /// The stable id (`general`, …) used by the CLI and the UI audit's screens.
    pub fn id(self) -> &'static str {
        match self {
            Section::General => "general",
            Section::Editor => "editor",
            Section::Appearance => "appearance",
            Section::Markdown => "markdown",
            Section::Images => "images",
            Section::Vim => "vim",
            Section::Keyboard => "keyboard",
            Section::Files => "files",
            Section::Advanced => "advanced",
        }
    }

    pub fn from_id(id: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|s| s.id() == id)
    }

    /// The name (English, Korean).
    pub fn names(self) -> (&'static str, &'static str) {
        match self {
            Section::General => ("General", "일반"),
            Section::Editor => ("Editor", "편집기"),
            Section::Appearance => ("Appearance", "모양"),
            Section::Markdown => ("Markdown", "Markdown"),
            Section::Images => ("Images", "이미지"),
            Section::Vim => ("Vim", "Vim"),
            Section::Keyboard => ("Keyboard", "키보드"),
            Section::Files => ("Files", "파일"),
            Section::Advanced => ("Advanced", "고급"),
        }
    }
}

/// What values a setting takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    /// A whole number in a range.
    Int {
        min: i64,
        max: i64,
    },
    /// One of the values (value, English, Korean).
    Choice(&'static [(&'static str, &'static str, &'static str)]),
    /// Text (`must`: a part it has to contain, e.g. `$(QUERY)`; empty: any text).
    Text {
        must: &'static str,
    },
    /// Column numbers from 1 to 1000, comma separated (empty: none).
    Columns,
}

/// One setting.
#[derive(Debug, Clone, Copy)]
pub struct Setting {
    pub key: &'static str,
    pub section: Section,
    /// A group inside the section (English, Korean).
    pub group: (&'static str, &'static str),
    pub en: &'static str,
    pub ko: &'static str,
    pub help_en: &'static str,
    pub help_ko: &'static str,
    pub kind: Kind,
    pub default: &'static str,
}

const ACCEPTS: &[(&str, &str, &str)] = &[
    ("never", "Never", "안 함"),
    (
        "chosen",
        "After choosing with the arrow keys",
        "화살표로 고른 뒤",
    ),
    ("always", "Always", "항상"),
];

macro_rules! s {
    ($key:literal, $sec:ident, ($ge:literal, $gk:literal), $en:literal, $ko:literal, $he:literal, $hk:literal, $kind:expr, $def:literal) => {
        Setting {
            key: $key,
            section: Section::$sec,
            group: ($ge, $gk),
            en: $en,
            ko: $ko,
            help_en: $he,
            help_ko: $hk,
            kind: $kind,
            default: $def,
        }
    };
}

/// Every setting, in the order the window shows them.
pub const SETTINGS: &[Setting] = &[
    // General.
    s!(
        "language",
        General,
        ("Language and look", "언어와 모양"),
        "Language",
        "언어",
        "The language of Jarak's window; System follows Windows.",
        "자락 창의 언어. 시스템은 Windows를 따릅니다.",
        Kind::Choice(&[
            ("system", "System", "시스템"),
            ("en", "English", "English"),
            ("ko", "한국어", "한국어")
        ]),
        "system"
    ),
    s!(
        "theme",
        General,
        ("Language and look", "언어와 모양"),
        "Theme",
        "테마",
        "Dark or light (the Kim design system); System follows Windows.",
        "어두운 테마 또는 밝은 테마(Kim 디자인 시스템). 시스템은 Windows를 따릅니다.",
        Kind::Choice(&[
            ("system", "System", "시스템"),
            ("dark", "Dark", "어둡게"),
            ("light", "Light", "밝게")
        ]),
        "system"
    ),
    s!(
        "menu_bar",
        General,
        ("Window", "창"),
        "Menu bar",
        "메뉴 막대",
        "Shown while Alt is pressed (as Notepad++ with it hidden), or always.",
        "Alt를 누를 때만 보이거나, 항상 보입니다.",
        Kind::Choice(&[("alt", "With Alt", "Alt로"), ("always", "Always", "항상")]),
        "alt"
    ),
    s!(
        "status_bar",
        General,
        ("Window", "창"),
        "Status bar",
        "상태 표시줄",
        "The line under the text: position, encoding, line endings, language, mode.",
        "텍스트 아래의 줄: 위치, 인코딩, 줄 끝, 언어, 모드.",
        Kind::Bool,
        "true"
    ),
    s!(
        "tab_close_buttons",
        General,
        ("Window", "창"),
        "Close buttons on tabs",
        "탭의 닫기 단추",
        "Every tab shows its close button; off: only the active tab and the one under the pointer.",
        "모든 탭에 닫기 단추를 보입니다. 끄면 활성 탭과 포인터 아래 탭에만 보입니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "tab_double_click_close",
        General,
        ("Window", "창"),
        "Double-click closes a tab",
        "두 번 클릭으로 탭 닫기",
        "A double click on a tab closes it (unsaved changes are asked about).",
        "탭을 두 번 클릭하면 닫습니다(저장하지 않은 변경은 묻습니다).",
        Kind::Bool,
        "false"
    ),
    s!(
        "single_window",
        General,
        ("Window", "창"),
        "One window",
        "창 하나",
        "Opening files hands them to the running window; off: each start opens a window of its own.",
        "파일을 열면 실행 중인 창에 넘깁니다. 끄면 시작할 때마다 새 창을 엽니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "restore_session",
        General,
        ("Window", "창"),
        "Restore the session at start",
        "시작할 때 세션 복원",
        "The documents, views and untitled texts of the last session come back.",
        "지난 세션의 문서, 보기, 제목 없는 텍스트를 되살립니다.",
        Kind::Bool,
        "true"
    ),
    // Editor.
    s!(
        "tab_width",
        Editor,
        ("Indentation", "들여쓰기"),
        "Tab width",
        "탭 너비",
        "Columns per tab stop.",
        "탭 하나의 열 수.",
        Kind::Int { min: 1, max: 16 },
        "4"
    ),
    s!(
        "use_spaces",
        Editor,
        ("Indentation", "들여쓰기"),
        "Tab inserts spaces",
        "Tab은 공백 넣기",
        "Tab and indentation insert spaces up to the next tab stop.",
        "Tab과 들여쓰기가 다음 탭 위치까지 공백을 넣습니다.",
        Kind::Bool,
        "false"
    ),
    s!(
        "indent",
        Editor,
        ("Indentation", "들여쓰기"),
        "Indentation on Enter",
        "Enter 들여쓰기",
        "Keep: the line's indentation. Smart: one level more after an opening bracket.",
        "유지: 줄의 들여쓰기. 스마트: 여는 괄호 뒤에서 한 단계 더.",
        Kind::Choice(&[
            ("off", "Off", "끔"),
            ("keep", "Keep the line's", "줄 그대로"),
            ("smart", "Smart", "스마트")
        ]),
        "smart"
    ),
    s!(
        "word_wrap",
        Editor,
        ("Lines", "줄"),
        "Word wrap",
        "자동 줄 바꿈",
        "Documents wrap long lines at the window's edge (View menu per document).",
        "문서의 긴 줄을 창 가장자리에서 바꿉니다(보기 메뉴에서 문서마다).",
        Kind::Bool,
        "false"
    ),
    s!(
        "highlight_current_line",
        Editor,
        ("Lines", "줄"),
        "Highlight the current line",
        "현재 줄 강조",
        "The caret's line has a background of its own.",
        "캐럿이 있는 줄에 배경을 둡니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "caret_blink",
        Editor,
        ("Caret", "캐럿"),
        "Blinking caret",
        "깜박이는 캐럿",
        "The caret blinks while the window has the focus (at most two repaints a second); off: it stays still and an idle window never repaints.",
        "창에 포커스가 있을 때 캐럿이 깜박입니다(초당 최대 두 번 다시 그림). 끄면 캐럿이 가만히 있고 쉬는 창은 다시 그리지 않습니다.",
        Kind::Bool,
        "false"
    ),
    s!(
        "multi_editing",
        Editor,
        ("Caret", "캐럿"),
        "Ctrl+click adds a caret",
        "Ctrl+클릭으로 캐럿 추가",
        "Several carets typing together (Notepad++'s multi-editing).",
        "여러 캐럿으로 함께 입력합니다(Notepad++의 다중 편집).",
        Kind::Bool,
        "true"
    ),
    s!(
        "virtual_space",
        Editor,
        ("Caret", "캐럿"),
        "Virtual space",
        "가상 공간",
        "The caret can go past the end of a line.",
        "캐럿이 줄 끝 너머로 갈 수 있습니다.",
        Kind::Bool,
        "false"
    ),
    s!(
        "copy_line",
        Editor,
        ("Clipboard", "클립보드"),
        "Copy the line without a selection",
        "선택 없이 줄 복사",
        "Ctrl+C / Ctrl+X without a selection copy / cut the line.",
        "선택 없이 Ctrl+C / Ctrl+X 하면 줄을 복사하거나 잘라냅니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "complete_auto",
        Editor,
        ("Completion", "자동 완성"),
        "Completion while typing",
        "입력 중 자동 완성",
        "The popup opens by itself while a word is typed.",
        "단어를 입력하는 동안 팝업이 저절로 열립니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "complete_min",
        Editor,
        ("Completion", "자동 완성"),
        "From how many characters",
        "몇 글자부터",
        "The popup opens from this many characters of a word.",
        "단어가 이 글자 수가 되면 팝업이 열립니다.",
        Kind::Int { min: 1, max: 9 },
        "2"
    ),
    s!(
        "complete_tab",
        Editor,
        ("Completion", "자동 완성"),
        "Tab accepts a completion",
        "Tab으로 완성 받기",
        "A popup that opened by itself takes Tab only after an arrow key chose an entry.",
        "저절로 열린 팝업은 화살표로 고른 뒤에만 Tab을 받습니다.",
        Kind::Choice(ACCEPTS),
        "chosen"
    ),
    s!(
        "complete_enter",
        Editor,
        ("Completion", "자동 완성"),
        "Enter accepts a completion",
        "Enter로 완성 받기",
        "The same for Enter (a popup asked for with Ctrl+Space takes them at once).",
        "Enter도 같습니다(Ctrl+Space로 연 팝업은 바로 받습니다).",
        Kind::Choice(ACCEPTS),
        "chosen"
    ),
    s!(
        "param_hints",
        Editor,
        ("Completion", "자동 완성"),
        "Parameter hints",
        "매개변수 힌트",
        "A known function's signature above the line.",
        "알려진 함수의 시그니처를 줄 위에 보입니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "snippets",
        Editor,
        ("Completion", "자동 완성"),
        "Snippets in the completion list",
        "완성 목록의 스니펫",
        "Snippets are offered with the words.",
        "단어와 함께 스니펫을 제안합니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "auto_close",
        Editor,
        ("Typing aids", "입력 도우미"),
        "Close brackets and quotes",
        "괄호와 따옴표 닫기",
        "( [ { and quotes insert their closer.",
        "( [ {와 따옴표가 닫는 문자를 넣습니다.",
        Kind::Bool,
        "false"
    ),
    s!(
        "close_tags",
        Editor,
        ("Typing aids", "입력 도우미"),
        "Close HTML/XML tags",
        "HTML/XML 태그 닫기",
        "> after <name adds </name>.",
        "<name 뒤의 >가 </name>을 넣습니다.",
        Kind::Bool,
        "false"
    ),
    s!(
        "brace_highlight",
        Editor,
        ("Typing aids", "입력 도우미"),
        "Highlight matching brackets",
        "짝 괄호 강조",
        "The bracket at the caret and its partner.",
        "캐럿의 괄호와 그 짝을 강조합니다.",
        Kind::Bool,
        "true"
    ),
    // Appearance.
    s!(
        "font_family",
        Appearance,
        ("Text", "텍스트"),
        "Editor font",
        "편집기 글꼴",
        "Built in: the Kim design system's monospace font; the others are used when Windows has them.",
        "기본: Kim 디자인 시스템의 고정폭 글꼴. 나머지는 Windows에 있을 때 씁니다.",
        Kind::Choice(&[
            ("default", "Built in", "기본"),
            ("consolas", "Consolas", "Consolas"),
            ("cascadia-mono", "Cascadia Mono", "Cascadia Mono"),
            ("d2coding", "D2Coding", "D2Coding"),
            ("courier-new", "Courier New", "Courier New")
        ]),
        "default"
    ),
    s!(
        "font_size",
        Appearance,
        ("Text", "텍스트"),
        "Editor font size",
        "편집기 글꼴 크기",
        "In points at 100 % zoom (Ctrl+Plus / Ctrl+Minus zoom on top of it).",
        "100 % 확대에서의 포인트(Ctrl+Plus / Ctrl+Minus 확대는 그 위에).",
        Kind::Int { min: 8, max: 40 },
        "13"
    ),
    s!(
        "line_numbers",
        Appearance,
        ("Margins", "여백"),
        "Line numbers",
        "줄 번호",
        "The margin with the line numbers.",
        "줄 번호 여백.",
        Kind::Bool,
        "true"
    ),
    s!(
        "edge_columns",
        Appearance,
        ("Margins", "여백"),
        "Edge lines at columns",
        "경계선 열",
        "Vertical lines at these columns (e.g. 80, 120; empty: none).",
        "이 열에 세로선을 긋습니다(예: 80, 120. 비우면 없음).",
        Kind::Columns,
        ""
    ),
    s!(
        "show_spaces",
        Appearance,
        ("Symbols", "기호"),
        "Show spaces and tabs",
        "공백과 탭 보이기",
        "",
        "",
        Kind::Bool,
        "false"
    ),
    s!(
        "show_eol",
        Appearance,
        ("Symbols", "기호"),
        "Show the line endings",
        "줄 끝 보이기",
        "",
        "",
        Kind::Bool,
        "false"
    ),
    s!(
        "show_controls",
        Appearance,
        ("Symbols", "기호"),
        "Show control characters",
        "제어 문자 보이기",
        "",
        "",
        Kind::Bool,
        "false"
    ),
    s!(
        "show_indent_guides",
        Appearance,
        ("Symbols", "기호"),
        "Show the indent guides",
        "들여쓰기 안내선 보이기",
        "",
        "",
        Kind::Bool,
        "false"
    ),
    s!(
        "show_wrap_symbol",
        Appearance,
        ("Symbols", "기호"),
        "Show the wrap symbol",
        "줄 바꿈 기호 보이기",
        "",
        "",
        Kind::Bool,
        "false"
    ),
    // Markdown.
    s!(
        "markdown_open_as",
        Markdown,
        ("Opening", "열기"),
        "Markdown files open as",
        "Markdown 파일 열기",
        "Source, source and preview side by side, or the preview.",
        "원본, 원본과 미리 보기 나란히, 또는 미리 보기.",
        Kind::Choice(&[
            ("source", "Source", "원본"),
            ("split", "Split", "나란히"),
            ("preview", "Preview", "미리 보기")
        ]),
        "source"
    ),
    // Images.
    s!(
        "image_folder",
        Images,
        ("Pasted images", "붙여 넣은 이미지"),
        "Folder for pasted images",
        "붙여 넣은 이미지 폴더",
        "Ctrl+V saves an image here, next to the Markdown document (a folder name, no path).",
        "Ctrl+V가 이미지를 Markdown 문서 옆의 이 폴더에 저장합니다(경로 없이 폴더 이름).",
        Kind::Text { must: "" },
        "assets"
    ),
    // Vim.
    s!(
        "editing_mode",
        Vim,
        ("Editing", "편집"),
        "Editing",
        "편집 방식",
        "Standard editing, or Vim's modes (Esc for Normal mode, i to type).",
        "표준 편집 또는 Vim의 모드(Esc는 Normal 모드, i는 입력).",
        Kind::Choice(&[("standard", "Standard", "표준"), ("vim", "Vim", "Vim")]),
        "standard"
    ),
    // Files.
    s!(
        "new_encoding",
        Files,
        ("New documents", "새 문서"),
        "Encoding",
        "인코딩",
        "The encoding new documents are saved in.",
        "새 문서를 저장할 인코딩.",
        Kind::Choice(&[
            ("utf-8", "UTF-8", "UTF-8"),
            ("utf-8-bom", "UTF-8 with BOM", "UTF-8 BOM"),
            (
                "ansi",
                "ANSI (the system code page)",
                "ANSI(시스템 코드 페이지)"
            )
        ]),
        "utf-8"
    ),
    s!(
        "new_eol",
        Files,
        ("New documents", "새 문서"),
        "Line endings",
        "줄 끝",
        "The line ending new documents use.",
        "새 문서가 쓰는 줄 끝.",
        Kind::Choice(&[
            ("crlf", "Windows (CR LF)", "Windows (CR LF)"),
            ("lf", "Unix (LF)", "Unix (LF)"),
            ("cr", "Macintosh (CR)", "Macintosh (CR)")
        ]),
        "crlf"
    ),
    s!(
        "recent_max",
        Files,
        ("History", "기록"),
        "Recent files kept",
        "보관할 최근 파일",
        "How many recently used files the File menu lists.",
        "파일 메뉴에 보일 최근 파일 수.",
        Kind::Int { min: 0, max: 50 },
        "20"
    ),
    s!(
        "detect_changes",
        Files,
        ("Files changed on disk", "디스크에서 바뀐 파일"),
        "Detect files changed by other programs",
        "다른 프로그램이 바꾼 파일 알아채기",
        "Checked when the window gets the focus and every 2 seconds.",
        "창이 포커스를 얻을 때와 2초마다 확인합니다.",
        Kind::Bool,
        "true"
    ),
    s!(
        "reload_unmodified_silently",
        Files,
        ("Files changed on disk", "디스크에서 바뀐 파일"),
        "Reload unmodified documents silently",
        "변경 없는 문서는 묻지 않고 다시 읽기",
        "Off: Jarak asks first (a reload can always be undone).",
        "끄면 먼저 묻습니다(다시 읽기는 언제든 되돌릴 수 있습니다).",
        Kind::Bool,
        "false"
    ),
    s!(
        "recovery_interval",
        Files,
        ("Recovery", "복구"),
        "Keep unsaved documents every (seconds)",
        "저장하지 않은 문서 보관 간격(초)",
        "Recovery snapshots after a crash; 0 turns them off.",
        "충돌 뒤 복구용 스냅숏. 0이면 끕니다.",
        Kind::Int { min: 0, max: 3600 },
        "10"
    ),
    // Advanced.
    s!(
        "search_engine",
        Advanced,
        ("Internet", "인터넷"),
        "Search engine",
        "검색 엔진",
        "The address *Search on the internet* opens; $(QUERY) is the selected text.",
        "*인터넷에서 검색*이 여는 주소. $(QUERY)가 선택한 텍스트입니다.",
        Kind::Text { must: "$(QUERY)" },
        "https://duckduckgo.com/?q=$(QUERY)"
    ),
    s!(
        "date_format",
        Advanced,
        ("Date and time", "날짜와 시간"),
        "Custom date format",
        "사용자 날짜 형식",
        "For *Insert date and time (custom)*: yyyy MM dd HH mm ss.",
        "*날짜와 시간 넣기(사용자)*용: yyyy MM dd HH mm ss.",
        Kind::Text { must: "" },
        "yyyy-MM-dd HH:mm:ss"
    ),
];

/// The setting named `key`.
pub fn find(key: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|s| s.key == key)
}

/// The settings of a section, in order.
pub fn of(section: Section) -> impl Iterator<Item = &'static Setting> {
    SETTINGS.iter().filter(move |s| s.section == section)
}

/// Why a value was refused (the CLI's `invalid_value` / `unknown_setting`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    UnknownSetting(String),
    InvalidValue { key: String, why: String },
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::UnknownSetting(k) => write!(f, "there is no setting `{k}`"),
            Refused::InvalidValue { key, why } => write!(f, "`{key}`: {why}"),
        }
    }
}

/// Checks `value` for `s` and gives it in its stored form (`true`, `12`, `vim`, `80,120`).
pub fn validate(s: &Setting, value: &str) -> Result<String, Refused> {
    let v = value.trim();
    let bad = |why: String| Refused::InvalidValue {
        key: s.key.to_string(),
        why,
    };
    match s.kind {
        Kind::Bool => match v.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok("true".into()),
            "false" | "off" | "no" | "0" => Ok("false".into()),
            _ => Err(bad("true or false".into())),
        },
        Kind::Int { min, max } => match v.parse::<i64>() {
            Ok(n) if (min..=max).contains(&n) => Ok(n.to_string()),
            _ => Err(bad(format!("a whole number from {min} to {max}"))),
        },
        Kind::Choice(list) => list
            .iter()
            .find(|(c, _, _)| c.eq_ignore_ascii_case(v))
            .map(|(c, _, _)| (*c).to_string())
            .ok_or_else(|| {
                bad(format!(
                    "one of {}",
                    list.iter()
                        .map(|(c, _, _)| *c)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }),
        Kind::Text { must } => {
            if v.contains(['\n', '\r', '"']) {
                Err(bad("one line without double quotes".into()))
            } else if !must.is_empty() && !v.contains(must) {
                Err(bad(format!("it has to contain {must}")))
            } else if s.key == "image_folder"
                && (v.is_empty() || v.contains(['/', '\\', ':']) || v.starts_with('.'))
            {
                Err(bad("a folder name without a path".into()))
            } else {
                Ok(v.to_string())
            }
        }
        Kind::Columns => {
            let mut cols = Vec::new();
            for part in v.split([',', ' ']).filter(|p| !p.trim().is_empty()) {
                match part.trim().parse::<usize>() {
                    Ok(n) if (1..=1000).contains(&n) => cols.push(n),
                    _ => return Err(bad("column numbers from 1 to 1000, comma separated".into())),
                }
            }
            cols.sort_unstable();
            cols.dedup();
            Ok(cols
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","))
        }
    }
}

/// The settings file.
#[derive(Debug, Clone)]
pub struct Settings {
    path: PathBuf,
}

impl Settings {
    pub fn new(path: &Path) -> Settings {
        Settings {
            path: path.to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The value written in the file for `key` (unquoted), if any.
    pub fn raw(&self, key: &str) -> Option<String> {
        raw_in(&std::fs::read_to_string(&self.path).ok()?, key)
    }

    /// The value of a known setting: the file's when the setting takes it, else the default.
    pub fn get(&self, key: &str) -> Option<String> {
        let s = find(key)?;
        Some(
            self.raw(key)
                .and_then(|v| validate(s, &v).ok())
                .unwrap_or_else(|| s.default.to_string()),
        )
    }

    /// Every setting with its value and whether it is the default (no valid line in the file).
    pub fn list(&self) -> Vec<(&'static Setting, String, bool)> {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        SETTINGS
            .iter()
            .map(
                |s| match raw_in(&text, s.key).and_then(|v| validate(s, &v).ok()) {
                    Some(v) => {
                        let default = v == s.default;
                        (s, v, default)
                    }
                    None => (s, s.default.to_string(), true),
                },
            )
            .collect()
    }

    /// Writes a checked value (atomically, other lines kept); `Ok` gives the stored form.
    pub fn set(&self, key: &str, value: &str) -> Result<String, SetError> {
        let s = find(key).ok_or_else(|| SetError::Refused(Refused::UnknownSetting(key.into())))?;
        let v = validate(s, value).map_err(SetError::Refused)?;
        let line = match s.kind {
            Kind::Bool | Kind::Int { .. } => format!("{key} = {v}"),
            _ => format!("{key} = \"{v}\""),
        };
        write_line(&self.path, key, Some(&line)).map_err(SetError::Io)?;
        Ok(v)
    }

    /// Back to the default (the line goes).
    pub fn reset(&self, key: &str) -> Result<(), SetError> {
        if find(key).is_none() {
            return Err(SetError::Refused(Refused::UnknownSetting(key.into())));
        }
        write_line(&self.path, key, None).map_err(SetError::Io)
    }
}

/// Why a write did not happen (nothing was written).
#[derive(Debug)]
pub enum SetError {
    Refused(Refused),
    Io(io::Error),
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetError::Refused(r) => write!(f, "{r}"),
            SetError::Io(e) => write!(f, "the settings file cannot be written: {e}"),
        }
    }
}

/// The value of a `key = value` line (quotes taken off).
pub fn raw_in(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Replaces (or adds, or with `None` removes) the line of `key`, keeping every other line, and
/// writes the file atomically (the folder is created when needed).
pub fn write_line(path: &Path, key: &str, line: Option<&str>) -> io::Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut found = false;
    let mut lines: Vec<String> = Vec::new();
    for l in old.lines() {
        if l.split_once('=').is_some_and(|(k, _)| k.trim() == key) {
            if !found && let Some(n) = line {
                lines.push(n.to_string());
            }
            found = true;
        } else {
            lines.push(l.to_string());
        }
    }
    if !found && let Some(n) = line {
        lines.push(n.to_string());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    crate::file::save_atomic(path, text.as_bytes())
}

/// The JSON schema of the settings as a JSON object (`jarak-settings-v1`), built from the registry:
/// each key optional, its type, range or values, its default; unknown keys allowed (they are kept).
pub fn schema() -> serde_json::Value {
    use serde_json::{Map, Value, json};
    let mut props = Map::new();
    for s in SETTINGS {
        let mut p = match s.kind {
            Kind::Bool => json!({ "type": "boolean", "default": s.default == "true" }),
            Kind::Int { min, max } => json!({
                "type": "integer", "minimum": min, "maximum": max,
                "default": s.default.parse::<i64>().unwrap_or(0)
            }),
            Kind::Choice(list) => json!({
                "enum": list.iter().map(|(v, _, _)| *v).collect::<Vec<_>>(),
                "default": s.default
            }),
            Kind::Text { must: "" } => {
                json!({ "type": "string", "pattern": "^[^\"\\r\\n]*$", "default": s.default })
            }
            Kind::Text { .. } => json!({
                "type": "string", "pattern": "^[^\"\\r\\n]*\\$\\(QUERY\\)[^\"\\r\\n]*$", "default": s.default
            }),
            Kind::Columns => json!({
                "type": "string", "pattern": "^([0-9]{1,4}(,[0-9]{1,4})*)?$", "default": s.default
            }),
        };
        p["description"] = Value::String(format!("{} ({}).", s.en, s.section.names().0));
        props.insert(s.key.to_string(), p);
    }
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/madebymouv/kim/docs/contracts/schemas/jarak-settings-v1.schema.json",
        "title": "Jarak settings, version 1",
        "description": "%APPDATA%\\mouv\\jarak\\settings.toml read as a JSON object (Step 105, JARAK-FR-130): every key is optional (a missing one is its default), and keys Jarak does not know are allowed and kept. Built from jarak_core::settings::SETTINGS; see docs/JARAK_SETTINGS.md.",
        "type": "object",
        "properties": props,
        "additionalProperties": true
    })
}

/// Every setting at its default, as a JSON object (the schema's example).
pub fn defaults_json() -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut m = Map::new();
    for s in SETTINGS {
        let v = match s.kind {
            Kind::Bool => Value::Bool(s.default == "true"),
            Kind::Int { .. } => Value::from(s.default.parse::<i64>().unwrap_or(0)),
            _ => Value::String(s.default.to_string()),
        };
        m.insert(s.key.to_string(), v);
    }
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference (`docs/JARAK_SETTINGS.md`) names every setting with its default.
    #[test]
    fn the_reference_names_every_setting() {
        let doc = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/JARAK_SETTINGS.md"),
        )
        .unwrap();
        for s in SETTINGS {
            let d = if s.default.is_empty() {
                "(none)"
            } else {
                s.default
            };
            assert!(
                doc.contains(&format!("| `{}` | {} |", s.key, s.en))
                    && doc.contains(&format!("`{d}` |")),
                "{} is not in JARAK_SETTINGS.md",
                s.key
            );
        }
    }

    /// The schema and its example in `docs/contracts` are the registry's (set
    /// `JARAK_WRITE_SCHEMA=1` to write them after changing a setting).
    #[test]
    fn the_schema_and_its_example_follow_the_registry() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts");
        let schema_file = root.join("schemas/jarak-settings-v1.schema.json");
        let example_file = root.join("examples/jarak-settings-v1/defaults.json");
        let text =
            |v: &serde_json::Value| format!("{}\n", serde_json::to_string_pretty(v).unwrap());
        if std::env::var("JARAK_WRITE_SCHEMA").is_ok_and(|v| v == "1") {
            std::fs::create_dir_all(example_file.parent().unwrap()).unwrap();
            std::fs::write(&schema_file, text(&schema())).unwrap();
            std::fs::write(&example_file, text(&defaults_json())).unwrap();
        }
        let read = |p: &Path| {
            serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(p).unwrap()).unwrap()
        };
        assert_eq!(
            read(&schema_file),
            schema(),
            "regenerate with JARAK_WRITE_SCHEMA=1"
        );
        assert_eq!(
            read(&example_file),
            defaults_json(),
            "regenerate with JARAK_WRITE_SCHEMA=1"
        );
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jarak-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Every setting has a section, both names, a default it takes itself, and a unique key;
    /// every section but Keyboard (the shortcut mapper) has settings.
    #[test]
    fn the_registry_is_complete_and_consistent() {
        let mut keys = std::collections::HashSet::new();
        for s in SETTINGS {
            assert!(keys.insert(s.key), "{} twice", s.key);
            assert!(!s.en.is_empty() && !s.ko.is_empty(), "{}", s.key);
            assert_eq!(
                validate(s, s.default).as_deref(),
                Ok(s.default),
                "{}",
                s.key
            );
            assert!(!s.group.0.is_empty() && !s.group.1.is_empty(), "{}", s.key);
        }
        for sec in Section::ALL {
            assert_eq!(Section::from_id(sec.id()), Some(sec));
            let n = of(sec).count();
            if sec == Section::Keyboard {
                assert_eq!(n, 0);
            } else {
                assert!(n > 0, "{} has settings", sec.id());
            }
        }
    }

    /// Values are checked and normalised; refusals say what the setting takes.
    #[test]
    fn values_are_checked_before_they_are_written() {
        let v = |k: &str, x: &str| validate(find(k).unwrap(), x);
        assert_eq!(v("status_bar", "On").as_deref(), Ok("true"));
        assert_eq!(v("tab_width", " 8 ").as_deref(), Ok("8"));
        assert!(v("tab_width", "0").is_err());
        assert!(v("tab_width", "four").is_err());
        assert_eq!(v("theme", "DARK").as_deref(), Ok("dark"));
        assert!(
            matches!(v("theme", "pink"), Err(Refused::InvalidValue { why, .. }) if why.contains("system, dark, light"))
        );
        assert_eq!(v("edge_columns", "120, 80 80").as_deref(), Ok("80,120"));
        assert_eq!(v("edge_columns", "").as_deref(), Ok(""));
        assert!(v("edge_columns", "0").is_err());
        assert!(v("search_engine", "https://example.org/").is_err());
        assert!(v("image_folder", r"..\x").is_err());
        assert!(v("image_folder", "img").is_ok());
        assert!(v("date_format", "a\"b").is_err());
    }

    /// A write keeps the file's other lines and comments, replaces the setting's line, and a
    /// reset removes it; reading falls back to the default for a missing or refused value.
    #[test]
    fn writes_keep_the_other_lines_and_reads_fall_back_to_defaults() {
        let d = temp("rw");
        let f = d.join("sub").join("settings.toml");
        let st = Settings::new(&f);
        assert_eq!(st.get("tab_width").as_deref(), Some("4"));
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, "# mine\nunknown_key = \"kept\"\ntheme = \"pink\"\n").unwrap();
        assert_eq!(
            st.get("theme").as_deref(),
            Some("system"),
            "a refused value is the default"
        );
        assert_eq!(st.set("theme", "light").unwrap(), "light");
        assert_eq!(st.set("tab_width", "8").unwrap(), "8");
        assert_eq!(
            st.set("search_engine", "https://x.org/?q=$(QUERY)")
                .unwrap(),
            "https://x.org/?q=$(QUERY)"
        );
        let text = std::fs::read_to_string(&f).unwrap();
        assert_eq!(
            text,
            "# mine\nunknown_key = \"kept\"\ntheme = \"light\"\ntab_width = 8\nsearch_engine = \"https://x.org/?q=$(QUERY)\"\n"
        );
        assert!(matches!(
            st.set("tab_width", "99"),
            Err(SetError::Refused(_))
        ));
        assert!(matches!(
            st.set("nope", "1"),
            Err(SetError::Refused(Refused::UnknownSetting(_)))
        ));
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            text,
            "a refused write changes nothing"
        );
        st.reset("theme").unwrap();
        assert_eq!(st.get("theme").as_deref(), Some("system"));
        assert!(!std::fs::read_to_string(&f).unwrap().contains("theme"));
        let listed = st.list();
        assert_eq!(listed.len(), SETTINGS.len());
        assert!(
            listed
                .iter()
                .any(|(s, v, def)| s.key == "tab_width" && v == "8" && !def)
        );
        assert!(
            listed
                .iter()
                .any(|(s, v, def)| s.key == "theme" && v == "system" && *def)
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
