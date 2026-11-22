//! Step 110: the remaining Notepad++ built-ins (each with its matrix row):
//!
//! - Save a copy as (FILE-10), the document's read-only flag and clearing the file's read-only
//!   attribute (EDIT-11), tab colours (VIEWS-08).
//! - Paste HTML or RTF from the clipboard as source text (EDIT-13), the character panel (insert a
//!   character by its code or from the 32–255 table, EDIT-16), the clipboard history of what Jarak
//!   copied, cut or pasted (EDIT-17; Jarak never watches the system clipboard).
//! - Find characters in a range: the next non-ASCII character, the next one above U+00FF, or a
//!   range of code points (SRCH-20).

use crate::JarakApp;
use eframe::egui;

/// The most clipboard history entries kept (in memory only).
pub const HISTORY_MAX: usize = 20;

/// An open small dialog of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapDialog {
    /// The character panel: the code being typed.
    Chars(String),
    /// The clipboard history.
    History,
    /// Find characters in a range: from and to (code points as typed).
    Range(String, String),
}

/// The colours a tab can take (kim-design roles): purple, green, amber, red, blue.
pub fn tab_color(c: &kim_ui::Colors, k: u8) -> egui::Color32 {
    match k {
        0 => c.accent,
        1 => c.success,
        2 => c.warning,
        3 => c.danger,
        _ => c.info,
    }
}

/// The names of the tab colours (English, Korean).
pub const TAB_COLORS: [(&str, &str); 5] = [
    ("Purple", "보라"),
    ("Green", "초록"),
    ("Amber", "호박색"),
    ("Red", "빨강"),
    ("Blue", "파랑"),
];

impl JarakApp {
    /// Remembers a text Jarak copied, cut or pasted (newest first, once, up to 1 MiB each).
    pub(crate) fn remember_clip(&mut self, s: &str) {
        if s.is_empty() || s.len() > 1024 * 1024 {
            return;
        }
        self.clip_history.retain(|x| x != s);
        self.clip_history.insert(0, s.to_string());
        self.clip_history.truncate(HISTORY_MAX);
    }

    /// The file commands: 0 save a copy as, 1 the document's read-only flag, 2 clear the file's
    /// read-only attribute.
    pub fn file_op(&mut self, k: u8) {
        let i = self.active;
        match k {
            0 => {
                let t = &self.tabs[i];
                let suggested = t.file.path.as_deref().map_or_else(
                    || format!("{} copy.txt", t.title().trim_start_matches('*').trim()),
                    |p| {
                        let stem = p
                            .file_stem()
                            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                        let ext = p
                            .extension()
                            .map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
                        p.with_file_name(format!("{stem} copy{ext}"))
                            .to_string_lossy()
                            .into_owned()
                    },
                );
                let Some(out) = self.dialogs.save_as(&suggested) else {
                    return;
                };
                if self.tabs[i].file.path.as_deref() == Some(out.as_path()) {
                    self.message = Some(
                        self.t("A copy needs another name", "사본은 다른 이름이어야 합니다")
                            .into(),
                    );
                    return;
                }
                self.message = Some(match self.tabs[i].file.save_copy(&out) {
                    Ok(()) => format!("{} {}", self.t("Copy saved:", "사본 저장:"), out.display()),
                    Err(e) => format!("{}: {e}", self.t("Not saved", "저장하지 않음")),
                });
            }
            1 => {
                let t = &self.tabs[i];
                if t.monitor {
                    self.message = Some(
                        self.t(
                            "A monitored document stays read-only; stop monitoring first",
                            "모니터링 중인 문서는 읽기 전용입니다. 먼저 모니터링을 멈추세요",
                        )
                        .into(),
                    );
                } else if t.file.read_only && !t.user_read_only && t.file.read_only_reason.is_some()
                {
                    // Lossy or binary text, the disk version opened to compare: saving could not
                    // reproduce the file, so it stays read-only.
                    let why = t.file.read_only_reason.clone().unwrap_or_default();
                    self.message = Some(format!(
                        "{} {why}",
                        self.t("It stays read-only:", "읽기 전용으로 남습니다:")
                    ));
                } else if t.file.read_only {
                    let t = &mut self.tabs[i];
                    t.file.read_only = false;
                    t.file.read_only_reason = None;
                    t.user_read_only = false;
                    self.message = Some(self.t("Read-only: off", "읽기 전용: 꺼짐").into());
                } else {
                    let reason = self
                        .t(
                            "set read-only (Edit → Read-only)",
                            "읽기 전용으로 설정함(편집 → 읽기 전용)",
                        )
                        .to_string();
                    let t = &mut self.tabs[i];
                    t.file.read_only = true;
                    t.file.read_only_reason = Some(reason);
                    t.user_read_only = true;
                    self.message = Some(self.t("Read-only: on", "읽기 전용: 켜짐").into());
                }
            }
            _ => {
                let Some(p) = self.tabs[i].file.path.clone() else {
                    return;
                };
                let r = std::fs::metadata(&p).and_then(|m| {
                    let mut perm = m.permissions();
                    if !perm.readonly() {
                        return Ok(false);
                    }
                    #[allow(clippy::permissions_set_readonly_false)]
                    perm.set_readonly(false);
                    std::fs::set_permissions(&p, perm).map(|()| true)
                });
                self.message = Some(match r {
                    Ok(true) => format!(
                        "{} {}",
                        self.t("The file is writable now:", "이제 파일에 쓸 수 있습니다:"),
                        p.display()
                    ),
                    Ok(false) => self
                        .t("The file is not read-only", "파일이 읽기 전용이 아닙니다")
                        .into(),
                    Err(e) => format!("{}: {e}", p.display()),
                });
            }
        }
    }

    /// Colours the active tab (`None`: no colour).
    pub fn color_tab(&mut self, k: Option<u8>) {
        let i = self.active;
        self.tabs[i].color = k.filter(|k| *k < 5);
    }

    /// Pastes the clipboard's HTML (`html`) or RTF as source text.
    pub fn paste_rich_source(&mut self, html: bool) {
        let (h, r) = self.clipboard.get_rich();
        let text = if html { h } else { r };
        match text {
            Some(t) if !t.is_empty() => {
                self.paste(&t);
                self.remember_clip(&t);
            }
            _ => {
                self.message = Some(
                    if html {
                        self.t("The clipboard holds no HTML", "클립보드에 HTML이 없습니다")
                    } else {
                        self.t("The clipboard holds no RTF", "클립보드에 RTF가 없습니다")
                    }
                    .into(),
                );
            }
        }
    }

    /// Selects the next character whose code point is in `lo..=hi`, after the caret (wrapping).
    pub fn find_char_range(&mut self, lo: u32, hi: u32) {
        let d = &self.tab().file.doc;
        let from = d.selection().primary().to();
        match jarak_core::text::next_char_in_range(d.text().slice(..), from, lo, hi) {
            Some(p) => {
                let doc = &mut self.tab_mut().file.doc;
                doc.set_selection(&jarak_core::selection::Selection::single(
                    jarak_core::selection::Range::new(p, p + 1),
                ));
                self.tab_mut().view.after_edit();
                let ch = self.tab().file.doc.text().char(p);
                self.message = Some(format!("U+{:04X} {ch}", ch as u32));
            }
            None => {
                self.message = Some(format!(
                    "{} U+{lo:04X}–U+{hi:04X}",
                    self.t("No character in", "이 범위의 문자가 없습니다:")
                ));
            }
        }
    }

    /// Inserts `ch` at the caret(s).
    fn insert_char(&mut self, ch: char) {
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let mut b = [0u8; 4];
        self.tab_mut().file.doc.type_text(ch.encode_utf8(&mut b));
        self.tab_mut().view.after_edit();
    }

    /// The dialogs of this module.
    pub(crate) fn gap_dialog_ui(&mut self, ctx: &egui::Context, c: &kim_ui::Colors) {
        use kim_design::space;
        let Some(dlg) = self.gap_dialog.clone() else {
            return;
        };
        let lang = self.lang;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        match dlg {
            GapDialog::Chars(mut code) => {
                let mut insert: Option<char> = None;
                let title = self.t("Character panel", "문자 패널").to_string();
                kim_ui::dialog_sized(ctx, "jarak-char-panel", &title, 420.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            lang.pick("Code (U+00E9, 0xE9 or 233)", "코드(U+00E9, 0xE9 또는 233)"),
                        );
                    });
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut code)
                            .desired_width(200.0)
                            .hint_text("U+00E9"),
                    );
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Character code")
                    });
                    let parsed = jarak_core::lineops::parse_char_code(&code);
                    ui.label(kim_ui::secondary(
                        ui,
                        parsed.map_or_else(
                            || {
                                lang.pick("Not a character code", "문자 코드가 아닙니다")
                                    .to_string()
                            },
                            |ch| format!("{ch}  U+{:04X}", ch as u32),
                        ),
                    ));
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.add_space(space::S);
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for n in 32u32..=255 {
                                let Some(ch) = char::from_u32(n).filter(|c| !c.is_control()) else {
                                    continue;
                                };
                                let label = format!("{n:>3}   0x{n:02X}   {ch}");
                                if kim_ui::menu_item(ui, &label, false).clicked() {
                                    insert = Some(ch);
                                }
                            }
                        });
                    ui.add_space(space::S);
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            close = true;
                        }
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Insert", "넣기")))
                            .clicked()
                            || enter
                        {
                            insert = parsed;
                        }
                    });
                });
                if let Some(ch) = insert {
                    self.insert_char(ch);
                }
                self.gap_dialog = if close {
                    None
                } else {
                    Some(GapDialog::Chars(code))
                };
            }
            GapDialog::History => {
                let mut pick: Option<String> = None;
                let title = self.t("Clipboard history", "클립보드 기록").to_string();
                let items = self.clip_history.clone();
                kim_ui::dialog_sized(ctx, "jarak-clip-history", &title, 480.0, |ui| {
                    if items.is_empty() {
                        ui.label(kim_ui::secondary(
                            ui,
                            lang.pick(
                                "Nothing copied, cut or pasted in Jarak yet",
                                "자락에서 아직 복사·잘라내기·붙여넣기한 것이 없습니다",
                            ),
                        ));
                    }
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for s in &items {
                                let first: String =
                                    s.lines().next().unwrap_or("").chars().take(60).collect();
                                let n = s.lines().count();
                                let label = if n > 1 {
                                    format!("{first} … ({n} lines)")
                                } else {
                                    first
                                };
                                if kim_ui::menu_item(ui, &label, false).clicked() {
                                    pick = Some(s.clone());
                                }
                            }
                        });
                    ui.add_space(space::S);
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                if let Some(s) = pick {
                    self.paste(&s);
                    self.remember_clip(&s);
                    close = true;
                }
                if close {
                    self.gap_dialog = None;
                }
            }
            GapDialog::Range(mut lo, mut hi) => {
                let mut find = false;
                let title = self
                    .t("Find characters in a range", "범위의 문자 찾기")
                    .to_string();
                kim_ui::dialog_sized(ctx, "jarak-char-range", &title, 420.0, |ui| {
                    ui.label(lang.pick(
                        "From and to (U+0080, 0x80 or 128)",
                        "시작과 끝(U+0080, 0x80 또는 128)",
                    ));
                    ui.horizontal(|ui| {
                        let a = ui.add(egui::TextEdit::singleline(&mut lo).desired_width(120.0));
                        a.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::TextInput, true, "From")
                        });
                        let b = ui.add(egui::TextEdit::singleline(&mut hi).desired_width(120.0));
                        b.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::TextInput, true, "To")
                        });
                    });
                    let _ = c;
                    ui.add_space(space::S);
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            close = true;
                        }
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Find next", "다음 찾기")))
                            .clicked()
                        {
                            find = true;
                        }
                    });
                });
                if find {
                    match (
                        jarak_core::lineops::parse_char_code(&lo),
                        jarak_core::lineops::parse_char_code(&hi),
                    ) {
                        (Some(a), Some(b)) if a <= b => self.find_char_range(a as u32, b as u32),
                        _ => {
                            self.message = Some(
                                self.t(
                                    "Two character codes, the first not above the second",
                                    "문자 코드 두 개(앞이 뒤보다 크지 않게)",
                                )
                                .into(),
                            );
                        }
                    }
                }
                self.gap_dialog = if close {
                    None
                } else {
                    Some(GapDialog::Range(lo, hi))
                };
            }
        }
    }

    /// The commands of this module from the palette: 0 character panel, 1 clipboard history,
    /// 2 next non-ASCII, 3 next above U+00FF, 4 the range dialog, 5 paste HTML, 6 paste RTF.
    pub fn gap_tool(&mut self, k: u8) {
        match k {
            0 => self.gap_dialog = Some(GapDialog::Chars(String::new())),
            1 => self.gap_dialog = Some(GapDialog::History),
            2 => self.find_char_range(0x80, 0x10_FFFF),
            3 => self.find_char_range(0x100, 0x10_FFFF),
            4 => self.gap_dialog = Some(GapDialog::Range("U+0080".into(), "U+00FF".into())),
            5 => self.paste_rich_source(true),
            _ => self.paste_rich_source(false),
        }
    }
}
