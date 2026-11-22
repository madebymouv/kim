//! The Encoding menu, line endings and Show Symbol in the window (`JARAK-FR-025`, `JARAK-FR-026`,
//! `JARAK-FR-037`, Step 103).
//!
//! - **Reinterpret** reads the file's bytes again in another encoding (nothing is written; unsaved
//!   changes are lost, so the window asks first when there are some).
//! - **Convert** keeps the text and saves it in another encoding next time; characters the encoding
//!   cannot hold are named first and replaced with `?` only when the user says so.
//! - **EOL conversion** rewrites every line ending (one undo step).
//! - **Show Symbol**: spaces and tabs, line endings, control characters, indent guides, the wrap
//!   symbol, edge lines — each a setting.

use crate::JarakApp;
use eframe::egui::{self, RichText};
use jarak_core::document::Eol;
use jarak_core::file::{CHARSETS, Format, TextEncoding, encoding_for_label};
use kim_design::{font, space};
use kim_ui::Colors;

/// What the Show Symbol settings draw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Symbols {
    pub spaces: bool,
    pub eol: bool,
    pub controls: bool,
    pub indent_guides: bool,
    pub wrap_symbol: bool,
    /// Columns of the vertical edge lines (empty: none).
    pub edges: Vec<usize>,
}

impl Symbols {
    pub fn read(path: &std::path::Path) -> Symbols {
        let flag = |k: &str| crate::read_setting(path, k).is_some_and(|v| v == "true");
        Symbols {
            spaces: flag("show_spaces"),
            eol: flag("show_eol"),
            controls: flag("show_controls"),
            indent_guides: flag("show_indent_guides"),
            wrap_symbol: flag("show_wrap_symbol"),
            edges: crate::read_setting(path, "edge_columns")
                .map(|v| parse_columns(&v))
                .unwrap_or_default(),
        }
    }
}

/// `"80, 120"` → `[80, 120]` (columns from 1 to 1000; others ignored).
pub fn parse_columns(v: &str) -> Vec<usize> {
    let mut c: Vec<usize> = v
        .split([',', ' '])
        .filter_map(|x| x.trim().parse().ok())
        .filter(|n| (1..=1000).contains(n))
        .collect();
    c.sort_unstable();
    c.dedup();
    c
}

/// An Encoding menu target: the names (English, Korean) and the format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub en: String,
    pub ko: String,
    pub format: Format,
}

/// The targets of the Encoding menu: UTF-8, UTF-8-BOM, UTF-16 BE / LE BOM, ANSI (the system code
/// page), then every character set by script.
pub fn targets() -> Vec<Target> {
    let mut v = vec![
        Target {
            en: "UTF-8".into(),
            ko: "UTF-8".into(),
            format: Format {
                encoding: TextEncoding::Utf8,
                bom: false,
            },
        },
        Target {
            en: "UTF-8-BOM".into(),
            ko: "UTF-8-BOM".into(),
            format: Format {
                encoding: TextEncoding::Utf8,
                bom: true,
            },
        },
        Target {
            en: "UTF-16 BE BOM".into(),
            ko: "UTF-16 BE BOM".into(),
            format: Format {
                encoding: TextEncoding::Utf16Be,
                bom: true,
            },
        },
        Target {
            en: "UTF-16 LE BOM".into(),
            ko: "UTF-16 LE BOM".into(),
            format: Format {
                encoding: TextEncoding::Utf16Le,
                bom: true,
            },
        },
    ];
    let ansi = jarak_core::file::system_encoding();
    v.push(Target {
        en: format!("ANSI ({})", ansi.label(false)),
        ko: format!("ANSI ({})", ansi.label(false)),
        format: Format {
            encoding: ansi,
            bom: false,
        },
    });
    for (group_en, group_ko, name, label) in CHARSETS {
        if let Some(e) = encoding_for_label(label) {
            v.push(Target {
                en: format!("{group_en}: {name}"),
                ko: format!("{group_ko}: {name}"),
                format: Format {
                    encoding: e,
                    bom: false,
                },
            });
        }
    }
    v
}

/// A question the Encoding menu asks first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// Reinterpret target `i` although the document has unsaved changes.
    Reinterpret(usize),
    /// Convert to target `i`: `n` characters it cannot hold (the first ones: char position,
    /// character, line), replaced with `?` if the user agrees.
    Lossy {
        target: usize,
        n: usize,
        first: Vec<(usize, char, usize)>,
    },
}

impl JarakApp {
    /// The Encoding menu's entries for the palette and the menu bar: reinterpret as and convert
    /// to every target, the line endings, the encoding of new documents.
    pub(crate) fn encoding_entries(&self) -> Vec<crate::palette::Entry> {
        use std::borrow::Cow;
        let mut v = Vec::new();
        for (i, t) in targets().iter().enumerate() {
            v.push(crate::palette::Entry {
                id: "encoding.reinterpret",
                en: Cow::Owned(format!("Encoding: reinterpret as {}", t.en)),
                ko: Cow::Owned(format!("인코딩: {}(으)로 다시 해석", t.ko)),
                keys: None,
                run: crate::palette::Run::Reinterpret(i as u16),
                menu: crate::palette::Menu::Encoding,
            });
        }
        for (i, t) in targets().iter().enumerate() {
            v.push(crate::palette::Entry {
                id: "encoding.convert",
                en: Cow::Owned(format!("Encoding: convert to {}", t.en)),
                ko: Cow::Owned(format!("인코딩: {}(으)로 변환", t.ko)),
                keys: None,
                run: crate::palette::Run::ConvertTo(i as u16),
                menu: crate::palette::Menu::Encoding,
            });
        }
        v
    }

    /// Reinterpret as target `i` (asks first when there are unsaved changes).
    pub fn reinterpret(&mut self, i: usize, asked: bool) {
        let Some(t) = targets().get(i).cloned() else {
            return;
        };
        if self.tab().file.path.is_none() {
            self.message = Some(
                self.t(
                    "An unsaved document has no bytes to read again; use Convert.",
                    "저장하지 않은 문서는 다시 읽을 바이트가 없습니다. 변환을 쓰세요.",
                )
                .into(),
            );
            return;
        }
        if !asked && self.tab().file.doc.is_dirty() {
            self.enc_question = Some(Question::Reinterpret(i));
            return;
        }
        let k = self.active;
        let limit = jarak_core::file::DEFAULT_SIZE_LIMIT;
        let line = {
            let d = &self.tabs[k].file.doc;
            d.text().char_to_line(d.selection().primary().head)
        };
        match self.tabs[k].file.reinterpret(t.format, limit) {
            Ok(()) => {
                let tab = &mut self.tabs[k];
                tab.view.after_edit();
                // The caret back on its line.
                let n = tab.file.doc.text().len_lines();
                let p = tab
                    .file
                    .doc
                    .text()
                    .line_to_char(line.min(n.saturating_sub(1)));
                tab.file
                    .doc
                    .set_selection(&jarak_core::selection::Selection::point(p));
                tab.hay = None;
                let ro = tab.file.read_only_reason.clone();
                self.message = Some(match ro {
                    Some(r) => format!("{} {} — {r}", self.t("Read as", "다시 읽음:"), t.en),
                    None => format!("{} {}", self.t("Read as", "다시 읽음:"), t.en),
                });
            }
            Err(e) => {
                self.message = Some(format!(
                    "{} {e}",
                    self.t("Not read again:", "다시 읽지 못함:")
                ));
            }
        }
    }

    /// Convert to target `i`: the next save writes it; unmappable characters are asked about.
    pub fn convert_to(&mut self, i: usize, replace: bool) {
        let Some(t) = targets().get(i).cloned() else {
            return;
        };
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let k = self.active;
        let text = self.tabs[k].file.doc.text().to_string();
        let (n, first) = jarak_core::file::unmappable(
            &text,
            t.format.encoding,
            if replace { usize::MAX } else { 3 },
        );
        if n > 0 && !replace {
            let d = &self.tabs[k].file.doc;
            let first = first
                .into_iter()
                .map(|(p, ch)| (p, ch, d.text().char_to_line(p) + 1))
                .collect();
            self.enc_question = Some(Question::Lossy {
                target: i,
                n,
                first,
            });
            return;
        }
        let d = &mut self.tabs[k].file.doc;
        if n > 0 {
            let at: Vec<usize> = first.iter().map(|(p, _)| *p).collect();
            d.replace_chars(&at, '?');
        }
        self.tabs[k].file.format = t.format;
        self.tabs[k].file.doc.mark_unsaved();
        self.tabs[k].view.after_edit();
        self.message = Some(format!(
            "{} {}{}",
            self.t("Saved as", "저장할 인코딩:"),
            t.en,
            if n > 0 {
                format!(" ({n} ?)")
            } else {
                String::new()
            }
        ));
    }

    /// EOL conversion: 0 Windows (CR LF), 1 Unix (LF), 2 Macintosh (CR).
    pub fn convert_eol(&mut self, k: u8) {
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let eol = [Eol::CrLf, Eol::Lf, Eol::Cr][usize::from(k.min(2))];
        let t = self.tab_mut();
        let changed = t.file.doc.convert_eol(eol);
        if !changed {
            // The same text, another ending for new lines: still something to save.
            t.file.doc.mark_unsaved();
        }
        t.view.after_edit();
    }

    /// The encoding of new documents: 0 UTF-8, 1 UTF-8-BOM, 2 ANSI (a setting).
    pub fn new_encoding_tool(&mut self, k: u8) {
        let (fmt, v) = match k {
            0 => (
                Format {
                    encoding: TextEncoding::Utf8,
                    bom: false,
                },
                "utf-8",
            ),
            1 => (
                Format {
                    encoding: TextEncoding::Utf8,
                    bom: true,
                },
                "utf-8-bom",
            ),
            _ => (
                Format {
                    encoding: jarak_core::file::system_encoding(),
                    bom: false,
                },
                "ansi",
            ),
        };
        self.new_format = fmt;
        if let Some(p) = &self.settings
            && let Err(e) = crate::write_setting(p, "new_encoding", v)
        {
            self.message = Some(format!("{e}"));
            return;
        }
        self.message = Some(format!(
            "{} {}",
            self.t("New documents:", "새 문서:"),
            fmt.encoding.label(fmt.bom)
        ));
    }

    /// Show Symbol: 0 spaces and tabs, 1 line endings, 2 control characters, 3 all of them, 4 indent
    /// guides, 5 the wrap symbol, 6 the edge line (the settings' columns, 80 when none).
    pub fn symbols_tool(&mut self, k: u8) {
        let s = &mut self.symbols;
        let (key, on) = match k {
            0 => {
                s.spaces = !s.spaces;
                ("show_spaces", s.spaces)
            }
            1 => {
                s.eol = !s.eol;
                ("show_eol", s.eol)
            }
            2 => {
                s.controls = !s.controls;
                ("show_controls", s.controls)
            }
            3 => {
                let all = !(s.spaces && s.eol && s.controls);
                s.spaces = all;
                s.eol = all;
                s.controls = all;
                for (key, v) in [
                    ("show_spaces", all),
                    ("show_eol", all),
                    ("show_controls", all),
                ] {
                    if let Some(p) = &self.settings {
                        let _ = crate::write_setting(p, key, &v.to_string());
                    }
                }
                return;
            }
            4 => {
                s.indent_guides = !s.indent_guides;
                ("show_indent_guides", s.indent_guides)
            }
            5 => {
                s.wrap_symbol = !s.wrap_symbol;
                ("show_wrap_symbol", s.wrap_symbol)
            }
            _ => {
                if s.edges.is_empty() {
                    s.edges = vec![80];
                } else {
                    s.edges.clear();
                }
                let v = s
                    .edges
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                if let Some(p) = &self.settings {
                    let _ = crate::write_setting(p, "edge_columns", &v);
                }
                return;
            }
        };
        if let Some(p) = &self.settings {
            let _ = crate::write_setting(p, key, &on.to_string());
        }
    }

    /// The question of the Encoding menu.
    pub(crate) fn enc_question_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(q) = self.enc_question.clone() else {
            return;
        };
        let lang = self.lang;
        let (title, body, yes) = match &q {
            Question::Reinterpret(i) => (
                self.t("Read the file again?", "파일을 다시 읽을까요?")
                    .to_string(),
                format!(
                    "{} {}",
                    self.t(
                        "The file is read again as",
                        "파일을 다음 인코딩으로 다시 읽습니다:"
                    ),
                    targets().get(*i).map(|t| t.en.clone()).unwrap_or_default()
                ) + &format!(
                    "\n{}",
                    self.t(
                        "The unsaved changes are lost (the file on disk stays as it is).",
                        "저장하지 않은 변경은 사라집니다(디스크의 파일은 그대로입니다)."
                    )
                ),
                self.t("Read again", "다시 읽기").to_string(),
            ),
            Question::Lossy { target, n, first } => {
                let name = targets()
                    .get(*target)
                    .map(|t| t.en.clone())
                    .unwrap_or_default();
                let list: Vec<String> = first
                    .iter()
                    .map(|(_, ch, line)| {
                        format!(
                            "{ch:?} (U+{:04X}, {} {line})",
                            *ch as u32,
                            self.t("line", "줄")
                        )
                    })
                    .collect();
                (
                    format!("{} {name}?", self.t("Convert to", "변환할까요:")),
                    format!(
                        "{n} {} {name}: {}.\n{}",
                        self.t(
                            "characters cannot be written in",
                            "개 글자는 다음 인코딩으로 쓸 수 없습니다:"
                        ),
                        list.join(", "),
                        self.t(
                            "Converting replaces them with ? (one undo step).",
                            "변환하면 ?로 바뀝니다(실행 취소 한 번으로 되돌림)."
                        )
                    ),
                    self.t("Replace and convert", "바꾸고 변환").to_string(),
                )
            }
        };
        let mut answer = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            answer = Some(false);
        }
        kim_ui::dialog_sized(ctx, "jarak-encoding-question", &title, 460.0, |ui| {
            let r = ui.label(RichText::new(&body).size(font::BODY).color(c.text_primary));
            if kim_ui::inspect::active() {
                kim_ui::inspect::record(
                    ui,
                    "encoding:question",
                    "label",
                    r.rect,
                    Some(kim_ui::inspect::Painted {
                        content: &body,
                        rect: r.rect,
                        size: font::BODY,
                        truncated: false,
                    }),
                    false,
                    &[],
                    Some(c.text_primary),
                    None,
                );
            }
            ui.add_space(space::S);
            kim_ui::dialog_actions(ui, |ui| {
                if ui
                    .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                    .clicked()
                {
                    answer = Some(false);
                }
                if ui.add(kim_ui::Button::primary(&yes)).clicked() {
                    answer = Some(true);
                }
            });
        });
        match answer {
            Some(true) => {
                self.enc_question = None;
                match q {
                    Question::Reinterpret(i) => self.reinterpret(i, true),
                    Question::Lossy { target, .. } => self.convert_to(target, true),
                }
            }
            Some(false) => self.enc_question = None,
            None => {}
        }
    }

    /// Binary content (Notepad++'s Paste Special): 0 copy, 1 cut, 2 paste — text that may hold NUL
    /// characters, through a clipboard format of its own (plain text stops at the first NUL).
    pub fn binary_tool(&mut self, k: u8) {
        let d = &self.tab().file.doc;
        let r = d.selection().primary();
        match k {
            0 | 1 => {
                if r.is_empty() {
                    self.message = Some(
                        self.t("Select the text first.", "먼저 텍스트를 선택하세요.")
                            .into(),
                    );
                    return;
                }
                if k == 1 && self.tab().file.read_only {
                    self.refuse_read_only();
                    return;
                }
                let text = d.text().slice(r.from()..r.to()).to_string();
                if !self.clipboard.set_binary(&text) {
                    self.message = Some(
                        self.t(
                            "The clipboard could not be written.",
                            "클립보드에 쓰지 못했습니다.",
                        )
                        .into(),
                    );
                    return;
                }
                if k == 1 {
                    let t = self.tab_mut();
                    t.file
                        .doc
                        .execute(jarak_core::command::Command::DeleteBackward, 1);
                    t.view.after_edit();
                }
            }
            _ => {
                if self.tab().file.read_only {
                    self.refuse_read_only();
                    return;
                }
                match self.clipboard.get_binary() {
                    Some(s) => {
                        let t = self.tab_mut();
                        t.file.doc.type_text(&s);
                        t.view.after_edit();
                    }
                    None => {
                        self.message = Some(
                            self.t(
                                "The clipboard has no text.",
                                "클립보드에 텍스트가 없습니다.",
                            )
                            .into(),
                        );
                    }
                }
            }
        }
    }
}
