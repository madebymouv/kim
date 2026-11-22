//! Extensions in the window (`JARAK-FR-160`, `JARAK-SEC-007`, Step 108): their languages and
//! snippets join the user's; their commands are in the palette and the Edit menu. The first run
//! of a command asks for the grant (exactly the manifest's permissions, for exactly this
//! manifest); the program runs on a thread, and its output is applied as one undo step — only if
//! the document is still as it was when the command started.

use crate::JarakApp;
use eframe::egui;
use jarak_core::extensions::{self as ext, Extension, Grants, Input, Output, RunError};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The grant question: the extension and its command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtQuestion {
    pub ext: usize,
    pub cmd: usize,
}

/// A command on its way.
#[derive(Debug)]
pub struct ExtRun {
    pub title: String,
    pub output: Output,
    /// The document it works on, its text then and the selection's character range.
    pub tab: u64,
    pub before: String,
    pub range: (usize, usize),
    pub result: Arc<Mutex<Option<Result<String, RunError>>>>,
}

impl JarakApp {
    /// Step 108: reads the installed extensions in `root` and the grants in `grants`.
    pub fn with_extensions(mut self, root: &std::path::Path, grants: PathBuf) -> JarakApp {
        let (ok, bad) = ext::load_all(root);
        self.ext_grants = Grants::load(&grants);
        self.ext_grants_file = Some(grants);
        for e in &ok {
            self.user_langs.extend(e.languages.iter().cloned());
            self.snippets.extend(e.snippets.iter().cloned());
        }
        if let Some((d, p)) = bad.first() {
            self.message = Some(format!(
                "{} {}: {}",
                self.t("Extension not used:", "쓰지 않는 확장:"),
                d.display(),
                p.join("; ")
            ));
        }
        self.extensions = ok;
        self
    }

    /// The extensions' commands as palette and menu entries.
    pub(crate) fn extension_entries(&self) -> Vec<crate::palette::Entry> {
        let mut v = Vec::new();
        let mut k = 0u16;
        for e in &self.extensions {
            for c in &e.manifest.contributes.commands {
                v.push(crate::palette::Entry {
                    id: "extension.command",
                    en: std::borrow::Cow::Owned(format!("{}: {}", e.manifest.name, c.title)),
                    ko: std::borrow::Cow::Owned(format!(
                        "{}: {}",
                        e.manifest.name,
                        c.title_ko.as_deref().unwrap_or(&c.title)
                    )),
                    keys: None,
                    run: crate::palette::Run::Extension(k),
                    menu: crate::palette::Menu::Edit,
                });
                k += 1;
            }
        }
        v
    }

    /// The extensions' command labels (English), for tests.
    pub fn extension_entries_for_test(&self) -> Vec<String> {
        self.extension_entries()
            .into_iter()
            .map(|e| e.en.into_owned())
            .collect()
    }

    /// The `k`th command of all extensions: (extension, command).
    fn extension_command(&self, k: u16) -> Option<(usize, usize)> {
        let mut n = 0u16;
        for (i, e) in self.extensions.iter().enumerate() {
            for j in 0..e.manifest.contributes.commands.len() {
                if n == k {
                    return Some((i, j));
                }
                n += 1;
            }
        }
        None
    }

    /// Runs the `k`th extension command (asking for the grant first).
    pub fn run_extension(&mut self, k: u16) {
        let Some((i, j)) = self.extension_command(k) else {
            return;
        };
        if self.ext_running.is_some() {
            self.message = Some(
                self.t(
                    "An extension command is still running",
                    "확장 명령이 아직 실행 중입니다",
                )
                .into(),
            );
            return;
        }
        if self.ext_grants.covers(&self.extensions[i]) {
            self.start_extension(i, j);
        } else {
            self.ext_question = Some(ExtQuestion { ext: i, cmd: j });
        }
    }

    fn start_extension(&mut self, i: usize, j: usize) {
        let e: &Extension = &self.extensions[i];
        let c = e.manifest.contributes.commands[j].clone();
        let dir = e.dir.clone();
        let t = self.tab();
        if matches!(c.output, Output::ReplaceSelection | Output::Insert) && t.file.read_only {
            self.message = Some(
                self.t("The document is read-only", "읽기 전용 문서입니다")
                    .into(),
            );
            return;
        }
        let sel = t.file.doc.selection().primary();
        let (a, b) = (sel.anchor.min(sel.head), sel.anchor.max(sel.head));
        let input = match c.input {
            Input::None => String::new(),
            Input::Selection => t.file.doc.selected_text(),
            Input::Document => t.file.doc.text().to_string(),
        };
        if c.input == Input::Selection && input.is_empty() {
            self.message = Some(
                self.t("Select the text first", "먼저 텍스트를 선택하세요")
                    .into(),
            );
            return;
        }
        let result = Arc::new(Mutex::new(None));
        self.ext_running = Some(ExtRun {
            title: c.title.clone(),
            output: c.output,
            tab: t.id,
            before: t.file.doc.text().to_string(),
            range: (a, b),
            result: result.clone(),
        });
        self.message = Some(format!("{} {}…", self.t("Running", "실행 중:"), c.title));
        std::thread::spawn(move || {
            #[cfg(windows)]
            let r = ext::run(&dir, &c, &input);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(r);
            }
        });
    }

    /// Each frame: the running command's output, once it is there.
    pub(crate) fn extension_frame(&mut self, ctx: &egui::Context) {
        let Some(run) = self.ext_running.as_ref() else {
            return;
        };
        let done = run.result.lock().ok().and_then(|mut r| r.take());
        let Some(r) = done else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        };
        let Some(run) = self.ext_running.take() else {
            return;
        };
        let out = match r {
            Ok(o) => o,
            Err(e) => {
                self.message = Some(format!("{}: {e}", run.title));
                return;
            }
        };
        if run.output == Output::Message {
            let first = out
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim();
            self.message = Some(if first.is_empty() {
                format!("{}: {}", run.title, self.t("(no output)", "(출력 없음)"))
            } else {
                format!("{}: {first}", run.title)
            });
            return;
        }
        if run.output == Output::NewDocument {
            self.act(crate::keys::Action::New);
            let eol = self.tab().file.doc.eol.as_str();
            let text = crate::clipboard::with_line_endings(&out, eol);
            self.tab_mut().file.doc.type_text(&text);
            self.message = Some(format!(
                "{}: {}",
                run.title,
                self.t("a new document", "새 문서")
            ));
            return;
        }
        let Some(i) = self.index_of(run.tab) else {
            return;
        };
        if self.tabs[i].file.doc.text() != run.before.as_str() {
            self.message = Some(format!(
                "{}: {}",
                run.title,
                self.t(
                    "the document changed while it ran; its output was not used",
                    "실행 중 문서가 바뀌어 출력을 쓰지 않았습니다"
                )
            ));
            return;
        }
        // The output in the document's line endings.
        let eol = self.tabs[i].file.doc.eol.as_str();
        let text = crate::clipboard::with_line_endings(&out, eol);
        let (a, b) = run.range;
        let doc = &mut self.tabs[i].file.doc;
        match run.output {
            Output::ReplaceSelection => {
                // A program like sort ends its output with a line break the selection may lack.
                let sel_text: String = doc.text().slice(a..b).to_string();
                let text = if !sel_text.ends_with(['\n', '\r']) {
                    text.trim_end_matches(['\r', '\n']).to_string()
                } else {
                    text
                };
                doc.replace(vec![(a, b, text)], None);
            }
            Output::Insert => {
                let at = doc.selection().primary().head;
                doc.replace(vec![(at, at, text)], None);
            }
            Output::Message | Output::NewDocument => {}
        }
        self.message = Some(format!(
            "{}: {}",
            run.title,
            self.t("done (Ctrl+Z undoes it)", "완료(Ctrl+Z로 되돌림)")
        ));
    }

    /// Asks for the grant before a command first runs.
    pub(crate) fn extension_question_ui(&mut self, ctx: &egui::Context, c: &kim_ui::Colors) {
        use kim_design::{font, space};
        let Some(q) = self.ext_question else {
            return;
        };
        let Some(e) = self.extensions.get(q.ext) else {
            self.ext_question = None;
            return;
        };
        let Some(cmd) = e.manifest.contributes.commands.get(q.cmd) else {
            self.ext_question = None;
            return;
        };
        let lang = self.lang;
        let gets = match (cmd.input, lang) {
            (Input::None, kim_design::Lang::Ko) => "아무 텍스트도 받지 않고",
            (Input::Selection, kim_design::Lang::Ko) => "선택한 텍스트를 받아",
            (Input::Document, kim_design::Lang::Ko) => "문서 전체를 받아",
            (Input::None, _) => "It gets no text and",
            (Input::Selection, _) => "It gets the selected text and",
            (Input::Document, _) => "It gets the whole document and",
        };
        let does = match (cmd.output, lang) {
            (Output::Message, kim_design::Lang::Ko) => "결과를 상태 표시줄에 보입니다.",
            (Output::ReplaceSelection, kim_design::Lang::Ko) => "선택을 결과로 바꿉니다.",
            (Output::Insert, kim_design::Lang::Ko) => "결과를 캐럿 위치에 넣습니다.",
            (Output::NewDocument, kim_design::Lang::Ko) => "결과를 새 문서로 엽니다.",
            (Output::Message, _) => "shows its result in the status bar.",
            (Output::ReplaceSelection, _) => "replaces the selection with its result.",
            (Output::Insert, _) => "inserts its result at the caret.",
            (Output::NewDocument, _) => "opens its result as a new document.",
        };
        let perms: Vec<&str> = e.manifest.permissions.iter().map(|p| p.name()).collect();
        let body = match lang {
            kim_design::Lang::Ko => format!(
                "\"{}\"이(가) {}을(를) 실행하려 합니다(사용자 계정 권한으로). {gets} {does}\n권한: {}. 이 확장의 매니페스트가 바뀌면 다시 묻습니다.",
                e.manifest.name,
                cmd.program,
                perms.join(", ")
            ),
            _ => format!(
                "\"{}\" asks to run {} with your account's rights. {gets} {does}\nPermissions: {}. If the extension changes, Jarak asks again.",
                e.manifest.name,
                cmd.program,
                perms.join(", ")
            ),
        };
        let mut answer: Option<bool> = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            answer = Some(false);
        }
        let title = self
            .t("Allow this extension?", "이 확장을 허용할까요?")
            .to_string();
        kim_ui::dialog_sized(ctx, "jarak-extension-question", &title, 520.0, |ui| {
            let r = ui.label(
                egui::RichText::new(&body)
                    .size(font::BODY)
                    .color(c.text_primary),
            );
            if kim_ui::inspect::active() {
                kim_ui::inspect::record(
                    ui,
                    "extension:question",
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
                    .add(kim_ui::Button::primary(
                        lang.pick("Allow and run", "허용하고 실행"),
                    ))
                    .clicked()
                {
                    answer = Some(true);
                }
            });
        });
        match answer {
            Some(true) => {
                self.ext_question = None;
                let e = &self.extensions[q.ext];
                self.ext_grants.grant(e);
                if let Some(p) = &self.ext_grants_file
                    && let Err(err) = self.ext_grants.save(p)
                {
                    self.message = Some(format!("{}: {err}", p.display()));
                }
                self.start_extension(q.ext, q.cmd);
            }
            Some(false) => {
                self.ext_question = None;
                self.message = Some(self.t("Not run", "실행하지 않음").into());
            }
            None => {}
        }
    }
}
