//! The Column Editor (Alt+C, `JARAK-FR-081`, Step 96): text, or incrementing numbers (initial,
//! increase by, repeat, leading zeros or spaces, decimal / hexadecimal / octal / binary), inserted
//! into every row of the column selection — or from the caret's line down to the last line, at
//! the caret's display column — as one undo step.

use crate::JarakApp;
use eframe::egui::{self, RichText};
use jarak_core::document::multi::{Leading, Numbers, numbers};
use kim_design::{font, space};
use kim_ui::Colors;

/// The dialog's fields (kept between uses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnEditor {
    pub numbers: bool,
    pub text: String,
    pub initial: String,
    pub step: String,
    pub repeat: String,
    pub leading: Leading,
    pub base: u32,
    pub error: Option<String>,
    pub focus: bool,
}

impl Default for ColumnEditor {
    fn default() -> Self {
        ColumnEditor {
            numbers: false,
            text: String::new(),
            initial: "1".into(),
            step: "1".into(),
            repeat: "1".into(),
            leading: Leading::None,
            base: 10,
            error: None,
            focus: true,
        }
    }
}

impl ColumnEditor {
    /// The numbers' settings, or what is wrong with them.
    pub fn numbers_of(&self) -> Result<Numbers, String> {
        let int = |s: &str, what: &str| {
            s.trim()
                .parse::<i64>()
                .map_err(|_| format!("{what}: a whole number, please"))
        };
        let repeat = self
            .repeat
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| "Repeat: 1 or more".to_string())?;
        Ok(Numbers {
            initial: int(&self.initial, "Initial number")?,
            step: int(&self.step, "Increase by")?,
            repeat,
            leading: self.leading,
            base: self.base,
        })
    }
}

impl JarakApp {
    /// Alt+C: opens the Column Editor.
    pub fn open_column_editor(&mut self) {
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let mut e = self.column_editor.take().unwrap_or_default();
        e.focus = true;
        e.error = None;
        self.column_editor = Some(e);
        self.column_editor_open = true;
    }

    /// Inserts the Column Editor's text or numbers; `false` (with the reason shown) when its
    /// numbers are not valid.
    pub fn column_insert(&mut self) -> bool {
        let Some(e) = self.column_editor.clone() else {
            return false;
        };
        let rows = self.tab().file.doc.column_rows();
        let list: Vec<String> = if e.numbers {
            match e.numbers_of() {
                Ok(n) => numbers(&n, rows),
                Err(why) => {
                    if let Some(c) = self.column_editor.as_mut() {
                        c.error = Some(why);
                    }
                    return false;
                }
            }
        } else {
            vec![e.text.clone(); rows]
        };
        let t = self.tab_mut();
        let mut it = list.into_iter();
        let n = t
            .file
            .doc
            .column_insert(&mut |_| it.next().unwrap_or_default());
        t.view.after_edit();
        self.column_editor = Some(ColumnEditor { error: None, ..e });
        self.column_editor_open = false;
        self.message = Some(format!(
            "{} {n}",
            self.t("Column Editor: rows", "열 편집기: 행")
        ));
        true
    }

    /// The dialog.
    pub(crate) fn column_editor_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.column_editor_open {
            return;
        }
        let lang = self.lang;
        let Some(e) = self.column_editor.as_mut() else {
            return;
        };
        // Enter and Esc are read before the dialog draws (a focused button would take Enter).
        let (enter, esc) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        let mut act = None;
        kim_ui::dialog_sized(
            ctx,
            "jarak-column-editor",
            lang.pick("Column Editor", "열 편집기"),
            420.0,
            |ui| {
                kim_ui::segmented(
                    ui,
                    &mut e.numbers,
                    &[
                        (false, lang.pick("Text to insert", "넣을 텍스트")),
                        (true, lang.pick("Number to insert", "넣을 숫자")),
                    ],
                );
                ui.add_space(space::S);
                let w = ui.available_width();
                if e.numbers {
                    // Three labelled fields side by side.
                    let fw = ((w - 2.0 * space::S) / 3.0).floor();
                    let focus = std::mem::take(&mut e.focus);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = space::S;
                        for (k, (value, en, ko)) in [
                            (&mut e.initial, "Initial number", "시작 숫자"),
                            (&mut e.step, "Increase by", "증가값"),
                            (&mut e.repeat, "Repeat", "반복"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            ui.vertical(|ui| {
                                ui.set_width(fw);
                                kim_ui::field(ui, lang.pick(en, ko), None, |ui| {
                                    let r =
                                        kim_ui::text_field(ui, value, lang.pick(en, ko), "", fw);
                                    if focus && k == 0 {
                                        r.request_focus();
                                    }
                                });
                            });
                        }
                    });
                    ui.add_space(space::S);
                    kim_ui::segmented(
                        ui,
                        &mut e.leading,
                        &[
                            (Leading::None, lang.pick("No leading", "앞 채움 없음")),
                            (Leading::Zeros, lang.pick("Leading zeros", "앞에 0")),
                            (Leading::Spaces, lang.pick("Leading spaces", "앞에 공백")),
                        ],
                    );
                    ui.add_space(space::XS);
                    kim_ui::segmented(
                        ui,
                        &mut e.base,
                        &[
                            (10, lang.pick("Dec", "10진")),
                            (16, lang.pick("Hex", "16진")),
                            (8, lang.pick("Oct", "8진")),
                            (2, lang.pick("Bin", "2진")),
                        ],
                    );
                } else {
                    let r = kim_ui::text_field(ui, &mut e.text, lang.pick("Text", "텍스트"), "", w);
                    if e.focus {
                        r.request_focus();
                        e.focus = false;
                    }
                }
                ui.add_space(space::S);
                ui.label(
                    RichText::new(lang.pick(
                        "Into every row of the column selection, or from the caret's line to the last line at its column.",
                        "열 선택의 모든 행에, 없으면 커서 줄부터 마지막 줄까지 커서 열에 넣습니다.",
                    ))
                    .size(font::CAPTION)
                    .color(c.text_muted),
                );
                if let Some(err) = &e.error {
                    ui.label(RichText::new(err).size(font::CAPTION).color(c.danger));
                }
                kim_ui::dialog_actions(ui, |ui| {
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                        .clicked()
                    {
                        act = Some(false);
                    }
                    if ui
                        .add(kim_ui::Button::primary(lang.pick("Insert", "넣기")))
                        .clicked()
                    {
                        act = Some(true);
                    }
                });
            },
        );
        if esc {
            act = Some(false);
        } else if enter && act.is_none() {
            act = Some(true);
        }
        match act {
            Some(true) => {
                self.column_insert();
            }
            Some(false) => self.column_editor_open = false,
            None => {}
        }
    }
}
