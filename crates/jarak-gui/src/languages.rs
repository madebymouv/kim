//! Syntax highlighting in the window (`JARAK-FR-090`, `JARAK-FR-091`, Step 98, ADR 0103): each
//! frame the lines on screen of every view are lexed (the line-start states before them are kept per
//! document and dropped from an edit's line on), the language's comment tokens go to the document
//! for the comment commands, the Language menu chooses or detects the language, and user-defined
//! languages are read from their folder, imported from Notepad++'s UDL XML and exported as JSON.

use crate::{JarakApp, LangChoice, palette, views};
use eframe::egui::Color32;
use jarak_core::syntax::{self, Kind, Lang, Style};
use kim_ui::Colors;

/// The colour of a kind of token: kim-design's tokens — the text stays monochrome, keywords and
/// tags take the purple accent, the rest a quiet hue each (ADR 0103).
pub fn syntax_color(c: &Colors, k: Kind) -> Color32 {
    match k {
        Kind::Keyword | Kind::Tag | Kind::Heading | Kind::Link => c.accent,
        Kind::Type | Kind::Attribute | Kind::Variable => c.info,
        Kind::String | Kind::Code => c.success,
        Kind::Number | Kind::Constant | Kind::Preprocessor => c.warning,
        Kind::Comment => c.text_muted,
        Kind::Operator => c.text_secondary,
        Kind::Emphasis => c.text_primary,
    }
}

/// The comment tokens a language gives the comment commands.
fn comment_tokens(l: &Lang) -> (Option<String>, Option<(String, String)>) {
    match l.style {
        Style::Markup | Style::Markdown => (None, Some(("<!--".into(), "-->".into()))),
        Style::Code => (
            l.line_comment
                .first()
                .or(l.line_start_comment.first())
                .cloned(),
            l.block_comment
                .iter()
                .find(|d| !d.line_start)
                .map(|d| (d.open.clone(), d.close.clone())),
        ),
    }
}

impl JarakApp {
    /// Step 107 (`--language <name>`): the active document in the language called `name` (any
    /// case; the user's languages first); an unknown name says so and changes nothing.
    pub fn choose_language_named(&mut self, name: &str) {
        let found = self
            .user_langs
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
            .map(|l| l.name.clone())
            .or_else(|| syntax::builtin(name).map(|l| l.name.clone()));
        match found {
            Some(n) => {
                let i = self.active;
                if let Some(t) = self.tabs.get_mut(i) {
                    t.lang_choice = LangChoice::Named(n);
                }
            }
            None if name.eq_ignore_ascii_case("text") || name.eq_ignore_ascii_case("plain") => {
                let i = self.active;
                if let Some(t) = self.tabs.get_mut(i) {
                    t.lang_choice = LangChoice::Plain;
                }
            }
            None => {
                self.message = Some(format!(
                    "{} `{name}`",
                    self.t("No language called", "이런 이름의 언어가 없습니다:")
                ));
            }
        }
    }

    /// The language of tab `i`: the chosen one, none (plain text) or the detected one.
    pub fn lang_for(&self, i: usize) -> Option<&Lang> {
        let t = self.tabs.get(i)?;
        match &t.lang_choice {
            LangChoice::Plain => None,
            LangChoice::Named(n) => self
                .user_langs
                .iter()
                .find(|l| &l.name == n)
                .or_else(|| syntax::builtin(n)),
            LangChoice::Auto => {
                let name = t
                    .file
                    .path
                    .as_deref()
                    .and_then(std::path::Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let first: String = t.file.doc.text().line(0).chars().take(200).collect();
                syntax::detect(&name, &first, &self.user_langs)
            }
        }
    }

    /// Every frame: the visible lines' tokens of each view's document, and its comment tokens.
    pub(crate) fn highlight_frame(&mut self) {
        let panes = if self.second.is_some() {
            vec![views::PaneId::Main, views::PaneId::Second]
        } else {
            vec![views::PaneId::Main]
        };
        for pane in panes {
            let k = self.active_of(pane);
            let mut hl = std::mem::take(&mut self.tabs[k].hl);
            let (since, old) = (self.tabs[k].hl_gen, self.tabs[k].hl_lang.clone());
            let mut toks = Vec::new();
            let mut name = String::new();
            let mut comments = (None, None);
            if let Some(lang) = self.lang_for(k) {
                let t = &self.tabs[k];
                let d = &t.file.doc;
                if lang.name != old {
                    hl = syntax::Highlighter::default();
                }
                if let Some(p) = d.edits_since(since) {
                    hl.invalidate_from(d.text().char_to_line(p.min(d.len_chars())));
                }
                let r = d.text();
                let first = t.view.top_line.min(r.len_lines().saturating_sub(1));
                let rows = t.view.page_lines + 2;
                let text_of = |i: usize| {
                    let l = r.line(i);
                    l.slice(..jarak_core::text::line_len(l)).to_string()
                };
                // Step 99: the lines on screen, past folded-away ones.
                let mut line = first;
                while line < r.len_lines() && toks.len() < rows {
                    if let Some((_, e)) = d.hidden_range(line) {
                        line = e + 1;
                        continue;
                    }
                    toks.push((line, hl.line(lang, line, &text_of)));
                    line += 1;
                }
                name = lang.name.clone();
                comments = comment_tokens(lang);
            } else {
                hl = syntax::Highlighter::default();
            }
            let t = &mut self.tabs[k];
            t.hl = hl;
            t.hl_gen = t.file.doc.edit_gen();
            t.hl_lang = name;
            t.view.syntax = toks;
            t.file.doc.comment_line = comments.0;
            t.file.doc.comment_block = comments.1;
        }
    }

    /// The languages of the Language menu: the built-in ones (not the inner PHP code), then the
    /// user-defined ones.
    fn language_list(&self) -> Vec<String> {
        syntax::builtins()
            .iter()
            .filter(|l| l.name != "PHP code")
            .map(|l| l.name.clone())
            .chain(self.user_langs.iter().map(|l| l.name.clone()))
            .collect()
    }

    /// The Language menu's entries (0 auto-detect, 1 plain text, then the languages).
    pub(crate) fn language_entries(&self) -> Vec<palette::Entry> {
        let mut v = vec![
            palette::Entry {
                id: "language.auto",
                en: "Language: detect from the file".into(),
                ko: "언어: 파일에서 알아내기".into(),
                keys: None,
                run: palette::Run::Language(0),
                menu: palette::Menu::Language,
            },
            palette::Entry {
                id: "language.plain",
                en: "Language: Normal text".into(),
                ko: "언어: 일반 텍스트".into(),
                keys: None,
                run: palette::Run::Language(1),
                menu: palette::Menu::Language,
            },
        ];
        for (k, n) in self.language_list().into_iter().enumerate() {
            v.push(palette::Entry {
                id: "language.set",
                en: format!("Language: {n}").into(),
                ko: format!("언어: {n}").into(),
                keys: None,
                run: palette::Run::Language(k as u16 + 2),
                menu: palette::Menu::Language,
            });
        }
        v
    }

    /// The Language menu: the active document's language.
    pub fn set_language(&mut self, k: u16) {
        let choice = match k {
            0 => LangChoice::Auto,
            1 => LangChoice::Plain,
            _ => match self.language_list().get(usize::from(k - 2)) {
                Some(n) => LangChoice::Named(n.clone()),
                None => return,
            },
        };
        self.tab_mut().lang_choice = choice;
        let name = self.lang_for(self.active).map_or_else(
            || self.t("Plain text", "일반 텍스트").to_string(),
            |l| l.name.clone(),
        );
        self.message = Some(format!("{} {name}", self.t("Language:", "언어:")));
    }

    /// Reads the user-defined languages again.
    pub fn reload_languages(&mut self) {
        let Some(dir) = &self.languages_dir else {
            return;
        };
        let (ok, bad) = syntax::load_udls(dir);
        self.user_langs = ok;
        // Step 108: the extensions' languages stay.
        let ext: Vec<_> = self
            .extensions
            .iter()
            .flat_map(|e| e.languages.iter().cloned())
            .collect();
        self.user_langs.extend(ext);
        if let Some((p, e)) = bad.first() {
            self.message = Some(format!("{}: {e}", p.display()));
        }
    }

    /// The user-language tools: 0 import a Notepad++ UDL (XML), 1 export the current language
    /// (JSON), 2 open the folder, 3 read the folder again.
    pub fn udl_tool(&mut self, k: u8) {
        let Some(dir) = self.languages_dir.clone() else {
            self.message = Some(
                self.t(
                    "No folder for user languages.",
                    "사용자 언어 폴더가 없습니다.",
                )
                .into(),
            );
            return;
        };
        match k {
            0 => {
                let Some(p) = self.dialogs.open() else {
                    return;
                };
                let lang = match std::fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|x| syntax::import_npp_udl(&x))
                {
                    Ok(l) => l,
                    Err(e) => {
                        self.message = Some(format!("{}: {e}", p.display()));
                        return;
                    }
                };
                let _ = std::fs::create_dir_all(&dir);
                let stem: String = lang
                    .name
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .collect();
                // Never replace a file: name-2.json, name-3.json …
                let mut target = dir.join(format!("{stem}.json"));
                let mut n = 2;
                while target.exists() {
                    target = dir.join(format!("{stem}-{n}.json"));
                    n += 1;
                }
                let json = serde_json::to_string_pretty(&lang).unwrap_or_default();
                match jarak_core::file::save_atomic(&target, json.as_bytes()) {
                    Ok(()) => {
                        self.reload_languages();
                        self.message = Some(format!(
                            "{} {} → {}",
                            self.t("Imported", "가져옴:"),
                            lang.name,
                            target.display()
                        ));
                    }
                    Err(e) => self.message = Some(format!("{}: {e}", target.display())),
                }
            }
            1 => {
                let Some(lang) = self.lang_for(self.active).cloned() else {
                    self.message = Some(
                        self.t("The document is plain text.", "문서가 일반 텍스트입니다.")
                            .into(),
                    );
                    return;
                };
                let Some(p) = self.dialogs.save_as(&format!("{}.json", lang.name)) else {
                    return;
                };
                let json = serde_json::to_string_pretty(&lang).unwrap_or_default();
                self.message = Some(match jarak_core::file::save_atomic(&p, json.as_bytes()) {
                    Ok(()) => format!("{} {}", self.t("Exported:", "내보냄:"), p.display()),
                    Err(e) => format!("{}: {e}", p.display()),
                });
            }
            2 => {
                let _ = std::fs::create_dir_all(&dir);
                self.opener.open_file(&dir);
            }
            _ => {
                self.reload_languages();
                self.message = Some(format!(
                    "{} {}",
                    self.t("User languages:", "사용자 언어:"),
                    self.user_langs.len()
                ));
            }
        }
    }
}
