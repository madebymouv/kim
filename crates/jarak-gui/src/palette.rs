//! The command palette (Ctrl+Shift+P, `JARAK-FR-054`) and the menu bar (Alt or the palette,
//! UI-10), Step 92: every command of the window and of the editing registry, with its keys.
//!
//! One list feeds both: the palette filters it by words (English, Korean, the command's id or
//! its keys) and runs the chosen command; the menu bar groups it by menu. The keys come from
//! [`crate::keys::key_label`], checked against the real bindings by a test.

use crate::Editing;
use crate::keys::{Action, key_label};
use jarak_core::command::COMMANDS;

/// What a palette entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Act(Action),
    Editing(Editing),
    MenuBar,
    Assets,
}

/// The menu an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Selection,
    Go,
    View,
    Markdown,
}

impl Menu {
    pub const ALL: [Menu; 6] = [
        Menu::File,
        Menu::Edit,
        Menu::Selection,
        Menu::Go,
        Menu::View,
        Menu::Markdown,
    ];

    /// The menu's name (English, Korean).
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            Menu::File => ("File", "파일"),
            Menu::Edit => ("Edit", "편집"),
            Menu::Selection => ("Selection", "선택"),
            Menu::Go => ("Go", "이동"),
            Menu::View => ("View", "보기"),
            Menu::Markdown => ("Markdown", "Markdown"),
        }
    }
}

/// A command as the palette and the menus show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: &'static str,
    pub en: &'static str,
    pub ko: &'static str,
    pub keys: Option<&'static str>,
    pub run: Run,
    pub menu: Menu,
}

fn act(id: &'static str, en: &'static str, ko: &'static str, a: Action, menu: Menu) -> Entry {
    Entry {
        id,
        en,
        ko,
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
            en: "Editing mode: Vim",
            ko: "편집 모드: Vim",
            keys: None,
            run: Run::Editing(Editing::Vim),
            menu: M::Edit,
        },
        Entry {
            id: "editing.standard",
            en: "Editing mode: Standard",
            ko: "편집 모드: 표준",
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
            en: "Menu bar",
            ko: "메뉴 막대",
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
            en: "Images and orphaned assets…",
            ko: "이미지와 고아 asset…",
            keys: None,
            run: Run::Assets,
            menu: M::Markdown,
        },
    ];
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
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            let hay = format!(
                "{} {} {} {}",
                e.en.to_lowercase(),
                e.ko,
                e.id,
                e.keys.unwrap_or("").to_lowercase()
            );
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|(i, _)| i)
        .collect()
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
