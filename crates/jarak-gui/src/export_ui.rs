//! Export from the window (`JARAK-FR-141`, Step 106): File → Export as HTML / RTF, Publish
//! Markdown to a folder, and Edit → Copy as HTML / RTF (Notepad++'s NppExport).
//!
//! - A Markdown document exports rendered, with its images embedded; any other document as its
//!   highlighted text. The export takes the text as it is in the window, saved or not; images are
//!   found next to the saved file (an untitled document has none).
//! - The Save dialog asks before it replaces a file; publishing into a folder that already holds
//!   the page or one of its images asks here first. Nothing is fetched or uploaded, and the
//!   document is never changed.

use crate::JarakApp;
use eframe::egui;
use jarak_core::export::{self as ex, ExportError, Format, Images};
use std::path::{Path, PathBuf};

/// Publishing would replace files: the folder and how many.
#[derive(Debug, Clone)]
pub struct PublishQuestion {
    pub tab: u64,
    pub folder: PathBuf,
    pub existing: usize,
}

impl JarakApp {
    /// Whether tab `i` is a Markdown document (by its name or its chosen language).
    pub(crate) fn tab_is_markdown(&self, i: usize) -> bool {
        self.tabs.get(i).is_some_and(|t| {
            t.file
                .path
                .as_deref()
                .is_some_and(jarak_core::markdown::is_markdown_path)
        }) || self
            .lang_for(i)
            .is_some_and(|l| l.style == jarak_core::syntax::Style::Markdown)
    }

    /// The export commands: 0 HTML, 1 RTF, 2 publish to a folder, 3 copy as HTML, 4 copy as RTF.
    pub fn export_tool(&mut self, n: u8) {
        let i = self.active;
        if i >= self.tabs.len() {
            return;
        }
        match n {
            0 => self.export_file(i, Format::Html),
            1 => self.export_file(i, Format::Rtf),
            2 => self.publish_ask(i),
            3 | 4 => self.copy_as(i, n == 3),
            _ => {}
        }
    }

    /// What an export says about the document's images.
    fn images_note(&self, images: &[ex::ImageReport]) -> String {
        let count = |k: &str| images.iter().filter(|i| i.used.name() == k).count();
        let parts: Vec<String> = [
            ("embedded", "embedded", "포함"),
            ("copied", "copied", "복사"),
            ("remote", "remote (not loaded)", "원격(불러오지 않음)"),
            ("missing", "missing", "없음"),
            ("refused", "not used", "사용 안 함"),
        ]
        .iter()
        .filter(|(k, ..)| count(k) > 0)
        .map(|(k, en, ko)| format!("{} {}", count(k), self.t(en, ko)))
        .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!(" — {}: {}", self.t("images", "이미지"), parts.join(", "))
        }
    }

    fn export_failed(&mut self, e: &ExportError) {
        let why = match e {
            ExportError::IntoSource(_) => self
                .t(
                    "an export never replaces the document itself",
                    "내보내기는 문서 자신을 바꾸지 않습니다",
                )
                .to_string(),
            other => other.to_string(),
        };
        self.message = Some(format!(
            "{}: {why}",
            self.t("Not exported", "내보내지 않음")
        ));
    }

    /// The suggested output name: the document's with `ext`, in its folder.
    fn suggested(&self, i: usize, ext: &str) -> String {
        let t = &self.tabs[i];
        match t.file.path.as_deref() {
            Some(p) => p.with_extension(ext).to_string_lossy().into_owned(),
            None => format!("{}.{ext}", t.title().trim_start_matches('*').trim()),
        }
    }

    fn export_file(&mut self, i: usize, format: Format) {
        let ext = if format == Format::Rtf { "rtf" } else { "html" };
        let suggested = self.suggested(i, ext);
        let Some(out) = self.dialogs.save_export(&suggested, format) else {
            return;
        };
        let t = &self.tabs[i];
        let text = t.file.doc.text().to_string();
        let doc_path = t.file.path.clone();
        let title = t.title().trim_start_matches('*').trim().to_string();
        let markdown = self.tab_is_markdown(i);
        let lang = self.lang_for(i).cloned();
        let (bytes, images) = match format {
            Format::Html if markdown => {
                let p = ex::markdown_html(&text, doc_path.as_deref(), &title, Images::Embed, &out);
                (p.html.into_bytes(), p.images)
            }
            Format::Html => (
                ex::code_html(&text, lang.as_ref(), &title, false).into_bytes(),
                Vec::new(),
            ),
            _ => (
                ex::code_rtf(&text, lang.as_ref(), false).into_bytes(),
                Vec::new(),
            ),
        };
        // The Save dialog asked before replacing a file.
        match ex::write_output(&out, &bytes, doc_path.as_deref(), true, false) {
            Ok(w) => {
                let note = self.images_note(&images);
                self.message = Some(format!(
                    "{} {}{note}",
                    self.t("Exported to", "내보냄:"),
                    w.path.display()
                ));
            }
            Err(e) => self.export_failed(&e),
        }
    }

    fn publish_ask(&mut self, i: usize) {
        if !self.tab_is_markdown(i) {
            self.message = Some(
                self.t(
                    "Publishing is for Markdown documents.",
                    "게시는 Markdown 문서에 씁니다.",
                )
                .into(),
            );
            return;
        }
        if self.tabs[i].file.path.is_none() {
            self.message = Some(
                self.t(
                    "Save the document first: its images are found next to it.",
                    "먼저 문서를 저장하세요. 이미지는 문서 옆에서 찾습니다.",
                )
                .into(),
            );
            return;
        }
        let Some(folder) = self.dialogs.export_folder() else {
            return;
        };
        let id = self.tabs[i].id;
        let (doc, text) = {
            let t = &self.tabs[i];
            (
                t.file.path.clone().unwrap_or_default(),
                t.file.doc.text().to_string(),
            )
        };
        match ex::publish(&doc, &text, &folder, false, true) {
            Ok(_) => self.publish_now(id, &folder, false),
            Err(ExportError::Exists(v)) => {
                self.publish_question = Some(PublishQuestion {
                    tab: id,
                    folder,
                    existing: v.len(),
                });
            }
            Err(e) => self.export_failed(&e),
        }
    }

    /// Publishes tab `id` into `folder` (`overwrite` after the user said so).
    pub fn publish_now(&mut self, id: u64, folder: &Path, overwrite: bool) {
        let Some(i) = self.index_of(id) else {
            return;
        };
        let (doc, text) = {
            let t = &self.tabs[i];
            (
                t.file.path.clone().unwrap_or_default(),
                t.file.doc.text().to_string(),
            )
        };
        match ex::publish(&doc, &text, folder, overwrite, false) {
            Ok(p) => {
                let note = self.images_note(&p.images);
                self.message = Some(format!(
                    "{} {}{note}",
                    self.t("Published to", "게시함:"),
                    p.page.display()
                ));
            }
            Err(e) => self.export_failed(&e),
        }
    }

    /// Copies the selection (or the whole document) as highlighted HTML or RTF, with its text.
    fn copy_as(&mut self, i: usize, html: bool) {
        let t = &self.tabs[i];
        let sel = t.file.doc.selected_text();
        let text = if sel.is_empty() {
            t.file.doc.text().to_string()
        } else {
            sel
        };
        let lang = self.lang_for(i).cloned();
        let fragment = ex::code_fragment(&text, lang.as_ref(), false, ex::Styling::Inline);
        let rtf = ex::code_rtf(&text, lang.as_ref(), false);
        let ok = if html {
            self.clipboard.set_rich(&text, Some(&fragment), None)
        } else {
            self.clipboard.set_rich(&text, None, Some(&rtf))
        };
        self.message = Some(
            match (ok, html) {
                (true, true) => self.t("Copied as HTML", "HTML로 복사함"),
                (true, false) => self.t("Copied as RTF", "RTF로 복사함"),
                (false, _) => self.t(
                    "The clipboard is busy; nothing was copied",
                    "클립보드를 쓸 수 없어 복사하지 않았습니다",
                ),
            }
            .into(),
        );
    }

    /// Asks before publishing replaces files.
    pub(crate) fn publish_question_ui(&mut self, ctx: &egui::Context, c: &kim_ui::Colors) {
        use kim_design::{font, space};
        let Some(q) = self.publish_question.clone() else {
            return;
        };
        let lang = self.lang;
        let body = format!(
            "{} {} {}",
            q.folder.display(),
            self.t("already holds", "에 이미"),
            match lang {
                kim_design::Lang::Ko =>
                    format!("이 게시가 쓸 파일 {}개가 있습니다. 바꿀까요?", q.existing),
                _ => format!(
                    "{} of the files this publishing writes. Replace them?",
                    q.existing
                ),
            }
        );
        let mut answer: Option<bool> = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            answer = Some(false);
        }
        let title = self.t("Replace files?", "파일을 바꿀까요?").to_string();
        kim_ui::dialog_sized(ctx, "jarak-publish-question", &title, 480.0, |ui| {
            let r = ui.label(
                egui::RichText::new(&body)
                    .size(font::BODY)
                    .color(c.text_primary),
            );
            if kim_ui::inspect::active() {
                kim_ui::inspect::record(
                    ui,
                    "publish:question",
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
                if ui
                    .add(kim_ui::Button::danger(lang.pick("Replace", "바꾸기")))
                    .clicked()
                {
                    answer = Some(true);
                }
            });
        });
        match answer {
            Some(true) => {
                self.publish_question = None;
                self.publish_now(q.tab, &q.folder, true);
            }
            Some(false) => {
                self.publish_question = None;
                self.message = Some(self.t("Not published", "게시하지 않음").into());
            }
            None => {}
        }
    }
}
