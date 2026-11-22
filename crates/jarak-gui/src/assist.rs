//! Editing assistance in the window (`JARAK-FR-094`, `JARAK-FR-095`, `JARAK-FR-096`, Step 100):
//! the completion popup (the document's words, the language's keywords, API functions, snippets,
//! paths), parameter hints, the typing aids each document gets from the settings and its
//! language, and the matching bracket or tag highlighted at the caret.
//!
//! A popup that opened by itself while typing takes neither Tab nor Enter (they indent and break
//! the line) until an arrow key chose an entry — unless the setting says always; one asked for
//! (Ctrl+Space …) takes them. Every aid is a setting of its own.

use crate::JarakApp;
use eframe::egui::{self, RichText};
use jarak_core::assist::{self as core, Api, CandKind, Candidate, Snippet};
use jarak_core::document::assist::{Aids, IndentMode};
use jarak_core::selection::{Range, Selection};
use jarak_core::syntax::{Lang, Style};
use kim_design::{font, size, space};
use kim_ui::Colors;
use std::path::{Path, PathBuf};

/// When Tab or Enter takes an entry of a popup that opened by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepts {
    Never,
    /// After Up/Down chose an entry (the default).
    Chosen,
    Always,
}

/// The assistance settings (each one on its own; kept in the settings file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistSettings {
    /// The popup opens while typing a word (from `complete_min` characters).
    pub complete_auto: bool,
    pub complete_min: usize,
    pub tab: Accepts,
    pub enter: Accepts,
    pub hints: bool,
    pub snippets: bool,
    pub auto_close: bool,
    pub close_tags: bool,
    pub indent: IndentMode,
    pub brace_highlight: bool,
}

impl Default for AssistSettings {
    fn default() -> Self {
        AssistSettings {
            complete_auto: true,
            complete_min: 2,
            tab: Accepts::Chosen,
            enter: Accepts::Chosen,
            hints: true,
            snippets: true,
            // Notepad++ inserts no closers or tags until asked to.
            auto_close: false,
            close_tags: false,
            indent: IndentMode::Smart,
            brace_highlight: true,
        }
    }
}

impl AssistSettings {
    /// The settings in `path` (the defaults for what it does not say).
    pub fn read(path: &Path) -> AssistSettings {
        let d = AssistSettings::default();
        let get = |k: &str| crate::read_setting(path, k);
        let flag = |k: &str, def: bool| get(k).map_or(def, |v| v == "true");
        AssistSettings {
            complete_auto: flag("complete_auto", d.complete_auto),
            complete_min: get("complete_min")
                .and_then(|v| v.parse().ok())
                .filter(|n| (1..=9).contains(n))
                .unwrap_or(d.complete_min),
            tab: accepts(get("complete_tab").as_deref(), d.tab),
            enter: accepts(get("complete_enter").as_deref(), d.enter),
            hints: flag("param_hints", d.hints),
            snippets: flag("snippets", d.snippets),
            auto_close: flag("auto_close", d.auto_close),
            close_tags: flag("close_tags", d.close_tags),
            indent: match get("indent").as_deref() {
                Some("off") => IndentMode::Off,
                Some("keep") => IndentMode::Keep,
                _ => d.indent,
            },
            brace_highlight: flag("brace_highlight", d.brace_highlight),
        }
    }
}

/// How the popup was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// While typing a word.
    Auto,
    /// Ctrl+Space: everything.
    All,
    /// Ctrl+Enter: the document's words.
    Words,
    /// Ctrl+Alt+Space: paths.
    Paths,
}

/// The open completion popup.
#[derive(Debug, Clone)]
pub struct Completion {
    pub items: Vec<Candidate>,
    pub selected: usize,
    /// An arrow key chose the entry (Enter may take it).
    pub chosen: bool,
    /// Where the completed text starts (a char position) and in which document.
    pub from: usize,
    pub tab: u64,
    pub trigger: Trigger,
    /// The word the entries were found for.
    pub prefix: String,
}

/// The parameter hint of the call the caret is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub tab: u64,
    pub name: String,
    pub overload: usize,
    pub arg: usize,
}

/// How far around the caret the document's words are collected, and how far a highlighted
/// bracket's partner is looked for.
const WORD_LINES: usize = 2000;
const MATCH_LINES: usize = 3000;

/// The aids a document in `lang` gets from `s`.
pub fn aids_for(lang: Option<&Lang>, s: &AssistSettings) -> Aids {
    let quotes: String = match lang {
        Some(l) => l
            .strings
            .iter()
            .filter(|d| d.open.chars().count() == 1 && d.open == d.close)
            .map(|d| d.open.clone())
            .collect(),
        None => "\"".into(),
    };
    let name = lang.map_or("", |l| l.name.as_str());
    Aids {
        auto_close: s.auto_close,
        close_tags: s.close_tags,
        indent: s.indent,
        quotes,
        colon_indents: matches!(name, "Python" | "YAML"),
        markup: lang.is_some_and(|l| l.style == Style::Markup),
        unit: match name {
            "Python" => "    ".into(),
            "YAML" => "  ".into(),
            _ => String::new(),
        },
    }
}

impl JarakApp {
    /// Reads the snippets file and the API files of `dir` (the configuration folder).
    pub fn with_assist(mut self, dir: PathBuf) -> JarakApp {
        self.assist_dir = Some(dir);
        self.reload_assist();
        self
    }

    /// Reads the user's snippets and API files again; a broken one is named.
    pub fn reload_assist(&mut self) {
        let Some(dir) = self.assist_dir.clone() else {
            self.snippets = core::snippets_with(&[]);
            return;
        };
        let mut bad = Vec::new();
        let user: Vec<Snippet> = match std::fs::read_to_string(dir.join("snippets.json")) {
            Ok(s) => core::parse_snippets(&s).unwrap_or_else(|e| {
                bad.push(format!("snippets.json: {e}"));
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        self.snippets = core::snippets_with(&user);
        let (apis, broken) = core::load_apis(&dir.join("autoCompletion"));
        self.apis = apis;
        bad.extend(broken.iter().map(|(p, e)| {
            format!(
                "{}: {e}",
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            )
        }));
        if !bad.is_empty() {
            self.message = Some(format!(
                "{} {}",
                self.t("Not read:", "읽지 못함:"),
                bad.join("; ")
            ));
        }
    }

    /// The API of tab `k`'s language: the user's file, else the built-in one.
    pub(crate) fn api_for(&self, k: usize) -> Option<&Api> {
        let lang = self.lang_for(k)?;
        self.apis
            .iter()
            .find(|a| a.language.eq_ignore_ascii_case(&lang.name))
            .or_else(|| core::builtin_api(&lang.name))
    }

    /// Before drawing: each view's document gets its aids; the bracket or tag at the caret and its
    /// partner are highlighted; the hint follows the caret.
    pub(crate) fn assist_frame(&mut self) {
        let mut panes = vec![crate::views::PaneId::Main];
        if self.second.is_some() {
            panes.push(crate::views::PaneId::Second);
        }
        for pane in panes {
            let k = self.active_of(pane);
            let lang = self.lang_for(k).cloned();
            let aids = aids_for(lang.as_ref(), &self.assist);
            let on = self.assist.brace_highlight && !self.is_large(k);
            let t = &mut self.tabs[k];
            if t.file.doc.aids != aids {
                t.file.doc.aids = aids;
            }
            let d = &t.file.doc;
            let key = (d.edit_gen(), d.selection().primary().head, on);
            if t.brace_key == Some(key) {
                continue;
            }
            t.brace_key = Some(key);
            t.view.braces.clear();
            if !on || d.selection().ranges().len() != 1 {
                continue;
            }
            // Step 109: no bracket matching on a caret line of more than 20 000 characters (its
            // text is not copied on every key).
            {
                let r = d.text();
                let l = r.line(r.char_to_line(d.selection().primary().head.min(r.len_chars())));
                if l.len_chars() > 20_000 {
                    continue;
                }
            }
            let r = d.text().clone();
            let head = d.selection().primary().head.min(r.len_chars());
            let line = r.char_to_line(head);
            let col = head - r.line_to_char(line);
            let total = r.len_lines();
            let text_of = |i: usize| {
                let l = r.line(i);
                l.slice(..jarak_core::text::line_len(l)).to_string()
            };
            let mut braces = Vec::new();
            let pos = |l: usize, c: usize| r.line_to_char(l) + c;
            match lang.as_ref() {
                Some(lg) => {
                    let hl = &mut t.hl;
                    let mut toks = |i: usize| hl.line(lg, i, &text_of);
                    if lg.style == Style::Markup
                        && let Some(((l0, a0, e0), (l1, a1, e1))) =
                            core::match_tag_by(line, col, total, &text_of, &mut toks, MATCH_LINES)
                    {
                        braces.push((pos(l0, a0), pos(l0, e0), true));
                        braces.push((pos(l1, a1), pos(l1, e1), true));
                    } else if let Some(((l0, c0), (l1, c1))) =
                        core::match_bracket_by(line, col, total, &text_of, &mut toks, MATCH_LINES)
                    {
                        braces.push((pos(l0, c0), pos(l0, c0) + 1, true));
                        braces.push((pos(l1, c1), pos(l1, c1) + 1, true));
                    } else if let Some(c) = bracket_near(&text_of(line), col) {
                        // A bracket without a partner (within the bound) is marked as such.
                        let p = pos(line, c);
                        if !in_string_or_comment(&toks(line), c) {
                            braces.push((p, p + 1, false));
                        }
                    }
                }
                None => {
                    let mut toks = |_: usize| Vec::new();
                    if let Some(((l0, c0), (l1, c1))) =
                        core::match_bracket_by(line, col, total, &text_of, &mut toks, MATCH_LINES)
                    {
                        braces.push((pos(l0, c0), pos(l0, c0) + 1, true));
                        braces.push((pos(l1, c1), pos(l1, c1) + 1, true));
                    }
                }
            }
            t.view.braces = braces;
        }
        self.hint_follow();
    }

    /// The word (or path) the completion would replace: its start and text.
    fn prefix_at(&self, paths: bool) -> (usize, String) {
        let d = &self.tab().file.doc;
        let head = d.selection().primary().head;
        let line = d.text().char_to_line(head);
        let line_start = d.text().line_to_char(line);
        let l = d.text().line(line);
        // Step 109: at most 4 096 characters before the caret (a long line is not copied whole on
        // every key).
        let col = head - line_start;
        let skip = col.saturating_sub(4096);
        let start = line_start + skip;
        let s = l
            .slice(skip..jarak_core::text::line_len(l).min(col + 256).max(col))
            .to_string();
        let extra = self
            .lang_for(self.active)
            .map_or(String::new(), |l| l.ident_chars.clone());
        let (c, p) = if paths {
            core::path_before(&s, head - start)
        } else {
            core::word_before(&s, head - start, &extra)
        };
        (start + c, p)
    }

    /// The candidates for the popup.
    fn candidates_for(&self, trigger: Trigger, prefix: &str) -> Vec<Candidate> {
        let k = self.active;
        if trigger == Trigger::Paths {
            let base = self.tabs[k].file.path.as_deref().and_then(Path::parent);
            return core::path_candidates(prefix, base);
        }
        let d = &self.tabs[k].file.doc;
        let r = d.text();
        let line = r.char_to_line(d.selection().primary().head);
        let lo = line.saturating_sub(WORD_LINES);
        let hi = (line + WORD_LINES).min(r.len_lines());
        let extra = self
            .lang_for(k)
            .map_or(String::new(), |l| l.ident_chars.clone());
        // Step 109: at most 100 000 characters on each side of the caret (a very long line is
        // not scanned whole on every key).
        let head = d.selection().primary().head;
        let from = r.line_to_char(lo).max(head.saturating_sub(100_000));
        let to = r.line_to_char(hi).min(head + 100_000).max(from);
        let span = r.slice(from..to);
        let words = core::words_in(span.chars(), prefix, &extra);
        if trigger == Trigger::Words {
            return core::candidates(prefix, &words, None, None, &[]);
        }
        let lang = self.lang_for(k);
        let snippets: &[Snippet] = if self.assist.snippets {
            &self.snippets
        } else {
            &[]
        };
        let mut v = core::candidates(prefix, &words, lang, self.api_for(k), snippets);
        // A snippet whose prefix is the typed word is offered too (Tab expands it).
        if self.assist.snippets {
            for s in snippets
                .iter()
                .filter(|s| s.prefix == prefix && s.applies(lang.map(|l| l.name.as_str())))
            {
                v.insert(
                    0,
                    Candidate {
                        text: s.prefix.clone(),
                        kind: CandKind::Snippet,
                        detail: s.description.clone(),
                    },
                );
            }
        }
        v
    }

    /// Opens the popup (`Auto`: only from the setting's number of characters).
    pub fn open_completion(&mut self, trigger: Trigger) {
        if self.tab().file.read_only || self.tab().file.doc.selection().ranges().len() != 1 {
            return;
        }
        let (from, prefix) = self.prefix_at(trigger == Trigger::Paths);
        if trigger == Trigger::Auto && prefix.chars().count() < self.assist.complete_min {
            self.completion = None;
            return;
        }
        let tab = self.tab().id;
        // A longer word where a shorter one found nothing finds nothing either.
        if trigger == Trigger::Auto
            && self
                .completion_miss
                .as_ref()
                .is_some_and(|(t, f, p)| *t == tab && *f == from && prefix.starts_with(p.as_str()))
        {
            self.completion = None;
            return;
        }
        let items = self.candidates_for(trigger, &prefix);
        if items.is_empty() {
            self.completion = None;
            if trigger == Trigger::Auto {
                self.completion_miss = Some((tab, from, prefix));
            } else {
                self.message = Some(self.t("No completion.", "완성할 것이 없습니다.").into());
            }
            return;
        }
        self.completion_miss = None;
        self.completion = Some(Completion {
            items,
            selected: 0,
            chosen: false,
            from,
            tab,
            trigger,
            prefix,
        });
        // Asked for with one answer: it is inserted at once (as in Notepad++).
        if trigger != Trigger::Auto && self.completion.as_ref().is_some_and(|c| c.items.len() == 1)
        {
            self.accept_completion();
        }
    }

    /// After typing `s`: the popup follows the word (or closes), a call's `(` shows its hint.
    pub(crate) fn after_typed(&mut self, s: &str) {
        let extra = self
            .lang_for(self.active)
            .map_or(String::new(), |l| l.ident_chars.clone());
        let wordy = s.chars().all(|c| core::is_word(c, &extra));
        match &self.completion {
            Some(c) if c.trigger == Trigger::Paths && !s.contains(char::is_whitespace) => {
                self.open_completion(Trigger::Paths);
            }
            Some(c) if wordy => {
                let t = c.trigger;
                // The word grew: the entries are narrowed, not found again.
                let (from, prefix) = self.prefix_at(false);
                if let Some(c) = self.completion.as_mut()
                    && from == c.from
                    && prefix.starts_with(c.prefix.as_str())
                {
                    let low = prefix.to_lowercase();
                    c.items.retain(|it| {
                        it.text.to_lowercase().starts_with(&low)
                            && (it.text != prefix || it.kind == CandKind::Snippet)
                    });
                    c.prefix = prefix.clone();
                    c.selected = 0;
                    c.chosen = false;
                    if c.items.is_empty() {
                        self.completion = None;
                        self.completion_miss = Some((self.tab().id, from, prefix));
                    }
                    return self.after_typed_hint(s);
                }
                self.open_completion(if t == Trigger::Auto { Trigger::Auto } else { t });
                if t != Trigger::Auto && self.completion.is_none() {
                    self.message = None;
                }
            }
            Some(_) => self.completion = None,
            None if wordy && self.assist.complete_auto => self.open_completion(Trigger::Auto),
            None => {}
        }
        self.after_typed_hint(s);
    }

    /// A typed `(` of the language's API opens the call's hint.
    fn after_typed_hint(&mut self, s: &str) {
        if self.assist.hints
            && let Some(api) = self.api_for(self.active)
            && s.contains(api.start_func)
        {
            self.open_hint(false);
        }
    }

    /// After Backspace or Delete: the popup follows the shorter word, or closes.
    pub(crate) fn after_deleted(&mut self) {
        if let Some(c) = &self.completion {
            let t = c.trigger;
            let head = self.tab().file.doc.selection().primary().head;
            if head <= c.from && t == Trigger::Auto {
                self.completion = None;
            } else {
                self.open_completion(t);
                self.message = None;
            }
        }
    }

    /// Inserts the selected candidate (a snippet expands with its tab stops).
    pub fn accept_completion(&mut self) {
        let Some(c) = self.completion.take() else {
            return;
        };
        let Some(item) = c.items.get(c.selected).cloned() else {
            return;
        };
        let k = self.active;
        if self.tabs[k].id != c.tab {
            return;
        }
        let lang_name = self.lang_for(k).map(|l| l.name.clone());
        let snippet = (item.kind == CandKind::Snippet)
            .then(|| {
                self.snippets
                    .iter()
                    .rev()
                    .find(|s| s.prefix == item.text && s.applies(lang_name.as_deref()))
                    .cloned()
            })
            .flatten();
        let d = &mut self.tabs[k].file.doc;
        let head = d.selection().primary().head;
        let from = c.from.min(head);
        if let Some(s) = snippet {
            let line = d.text().char_to_line(from);
            let start = d.text().line_to_char(line);
            let indent: String = d
                .text()
                .slice(start..from)
                .chars()
                .take_while(|x| *x == ' ' || *x == '\t')
                .collect();
            let e = core::expand(&s.body, &indent, &d.indent_unit(), d.eol.as_str());
            d.insert_snippet(from, head, &e);
        } else {
            d.set_selection(&Selection::single(Range::new(from, head)));
            d.type_text(&item.text);
        }
        self.tabs[k].view.after_edit();
    }

    /// Ctrl+Shift+Space (or a typed `(`): the hint of the call the caret is in.
    pub fn open_hint(&mut self, manual: bool) {
        match self.call_here() {
            Some((name, arg)) => {
                let tab = self.tab().id;
                self.hint = Some(Hint {
                    tab,
                    name,
                    overload: 0,
                    arg,
                });
            }
            None => {
                self.hint = None;
                if manual {
                    self.message = Some(
                        self.t(
                            "No parameter hint here.",
                            "여기에는 매개변수 힌트가 없습니다.",
                        )
                        .into(),
                    );
                }
            }
        }
    }

    fn call_here(&mut self) -> Option<(String, usize)> {
        let k = self.active;
        let api = self.api_for(k)?.clone();
        let lang = self.lang_for(k).cloned();
        let t = &mut self.tabs[k];
        let d = &t.file.doc;
        let r = d.text().clone();
        let head = d.selection().primary().head.min(r.len_chars());
        let line = r.char_to_line(head);
        let col = head - r.line_to_char(line);
        let text_of = |i: usize| {
            let l = r.line(i);
            l.slice(..jarak_core::text::line_len(l)).to_string()
        };
        let toks = match &lang {
            Some(lg) => t.hl.line(lg, line, &text_of),
            None => Vec::new(),
        };
        core::call_at(&text_of(line), col, &toks, &api)
    }

    /// The hint follows the caret: another parameter, another call, or gone.
    fn hint_follow(&mut self) {
        let Some(h) = self.hint.clone() else {
            return;
        };
        if self.tab().id != h.tab {
            self.hint = None;
            return;
        }
        match self.call_here() {
            Some((name, arg)) if name == h.name => {
                if let Some(x) = self.hint.as_mut() {
                    x.arg = arg;
                }
            }
            Some((name, arg)) => {
                self.hint = Some(Hint {
                    name,
                    arg,
                    overload: 0,
                    ..h
                });
            }
            None => self.hint = None,
        }
    }

    /// A key while the popup, a hint or a snippet is open: whether it was taken.
    pub(crate) fn assist_key(&mut self, key: egui::Key, m: egui::Modifiers) -> bool {
        use egui::Key as K;
        let plain = !m.ctrl && !m.command && !m.alt;
        if let Some(c) = self.completion.as_mut() {
            let n = c.items.len();
            match key {
                K::ArrowDown if plain && !m.shift => {
                    c.selected = (c.selected + 1) % n;
                    c.chosen = true;
                    return true;
                }
                K::ArrowUp if plain && !m.shift => {
                    c.selected = (c.selected + n - 1) % n;
                    c.chosen = true;
                    return true;
                }
                K::PageDown if plain => {
                    c.selected = (c.selected + 8).min(n - 1);
                    c.chosen = true;
                    return true;
                }
                K::PageUp if plain => {
                    c.selected = c.selected.saturating_sub(8);
                    c.chosen = true;
                    return true;
                }
                K::Tab | K::Enter if plain && !m.shift => {
                    let rule = if key == K::Tab {
                        self.assist.tab
                    } else {
                        self.assist.enter
                    };
                    let take = c.trigger != Trigger::Auto
                        || match rule {
                            Accepts::Always => true,
                            Accepts::Chosen => c.chosen,
                            Accepts::Never => false,
                        };
                    if take {
                        self.accept_completion();
                        return true;
                    }
                    // Not taken: the popup closes and the key indents or breaks the line.
                    self.completion = None;
                    return false;
                }
                K::Escape => {
                    self.completion = None;
                    return true;
                }
                K::Backspace | K::Delete => return false,
                _ => {
                    // Moving the caret or any other key closes the popup and does its own thing.
                    self.completion = None;
                }
            }
        }
        if self.hint.is_some() {
            match key {
                // Alt+Up / Alt+Down: the previous / next overload.
                K::ArrowUp | K::ArrowDown if m.alt && !m.ctrl && !m.shift => {
                    let n = self.hint_overloads();
                    if n > 1
                        && let Some(h) = self.hint.as_mut()
                    {
                        h.overload = if key == K::ArrowDown {
                            (h.overload + 1) % n
                        } else {
                            (h.overload + n - 1) % n
                        };
                    }
                    return true;
                }
                K::Escape if plain => {
                    self.hint = None;
                    return true;
                }
                _ => {}
            }
        }
        // In a snippet Tab goes to the next stop, Shift+Tab back, Esc leaves it.
        let d = &mut self.tab_mut().file.doc;
        if d.snippet_active() {
            match key {
                K::Tab if plain => {
                    d.snippet_step(m.shift);
                    self.tab_mut().view.after_edit();
                    return true;
                }
                K::Escape => {
                    d.clear_snippet();
                    return false;
                }
                _ => {}
            }
        }
        false
    }

    fn hint_overloads(&self) -> usize {
        let Some(h) = &self.hint else {
            return 0;
        };
        self.api_for(self.active)
            .and_then(|a| a.get(&h.name))
            .map_or(0, |w| w.overloads.len())
    }

    /// The popup under the completed word and the hint above the caret's line.
    pub(crate) fn assist_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let lang = self.lang;
        if self
            .completion
            .as_ref()
            .is_some_and(|x| x.tab != self.tab().id)
        {
            self.completion = None;
        }
        let mut accept = None;
        if let Some(comp) = &self.completion {
            let t = self.tab();
            let anchor = t.view.pos_of_char(&t.file.doc, comp.from);
            if let Some(p) = anchor {
                let row_h = size::CONTROL_COMPACT;
                let rows = comp.items.len().min(8);
                egui::Area::new(egui::Id::unique("jarak-completion"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(
                        p.x - space::S,
                        p.y + t.view.row_height() / 2.0 + space::XS,
                    ))
                    .show(ctx, |ui| {
                        kim_ui::popover_frame(ctx).show(ui, |ui| {
                            ui.set_width(320.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            egui::ScrollArea::vertical()
                                .max_height(row_h * rows as f32)
                                .auto_shrink([false, true])
                                .show_rows(ui, row_h, comp.items.len(), |ui, range| {
                                    for i in range {
                                        let it = &comp.items[i];
                                        let (rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), row_h),
                                            egui::Sense::click(),
                                        );
                                        let selected = i == comp.selected;
                                        if selected {
                                            ui.painter().rect_filled(rect, 2.0, c.bg_selected);
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(rect, 2.0, c.bg_hover);
                                        }
                                        let (en, ko) = it.kind.label();
                                        let kind = lang.pick(en, ko);
                                        let label = format!("{}  {kind}", it.text);
                                        ui.painter().text(
                                            egui::pos2(rect.left() + space::S, rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &it.text,
                                            kim_ui::ui_font(ui.ctx(), font::BODY, selected),
                                            c.text_primary,
                                        );
                                        ui.painter().text(
                                            egui::pos2(rect.right() - space::S, rect.center().y),
                                            egui::Align2::RIGHT_CENTER,
                                            kind,
                                            kim_ui::ui_font(ui.ctx(), font::CAPTION, false),
                                            c.text_muted,
                                        );
                                        resp.widget_info(|| {
                                            egui::WidgetInfo::labeled(
                                                egui::Role::Button,
                                                true,
                                                label.clone(),
                                            )
                                        });
                                        if kim_ui::inspect::active() {
                                            kim_ui::inspect::record(
                                                ui,
                                                &format!("completion:row#{i}"),
                                                "tree-row",
                                                rect,
                                                Some(kim_ui::inspect::Painted {
                                                    content: &label,
                                                    rect,
                                                    size: font::BODY,
                                                    truncated: false,
                                                }),
                                                true,
                                                if selected { &["selected"] } else { &[] },
                                                Some(c.text_primary),
                                                None,
                                            );
                                        }
                                        if resp.clicked() {
                                            accept = Some(i);
                                        }
                                    }
                                });
                            if let Some(it) = comp.items.get(comp.selected)
                                && !it.detail.is_empty()
                            {
                                ui.add_space(space::XS);
                                ui.label(
                                    RichText::new(&it.detail)
                                        .size(font::CAPTION)
                                        .color(c.text_secondary),
                                );
                            }
                        });
                    });
            }
        }
        if let Some(i) = accept {
            if let Some(comp) = self.completion.as_mut() {
                comp.selected = i;
            }
            self.accept_completion();
        }
        self.hint_ui(ctx, c);
    }

    fn hint_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(h) = self.hint.clone() else {
            return;
        };
        let k = self.active;
        let Some(api) = self.api_for(k) else {
            return;
        };
        let Some(w) = api.get(&h.name) else {
            return;
        };
        let n = w.overloads.len();
        let Some(o) = w.overloads.get(h.overload.min(n.saturating_sub(1))) else {
            return;
        };
        let (sig, ranges) = core::signature(w, o, api.separator);
        let t = self.tab();
        let d = &t.file.doc;
        let head = d.selection().primary().head;
        let line_start = d.text().line_to_char(d.text().char_to_line(head));
        let Some(p) = t.view.pos_of_char(d, line_start) else {
            return;
        };
        let caret = t.view.pos_of_char(d, head).map_or(p.x, |q| q.x);
        let mut job = egui::text::LayoutJob::default();
        let fid = kim_ui::ui_font(ctx, font::BODY, false);
        let bold = kim_ui::ui_font(ctx, font::BODY, true);
        let chars: Vec<char> = sig.chars().collect();
        let cur = ranges.get(h.arg).copied();
        let mut i = 0;
        let put = |job: &mut egui::text::LayoutJob, a: usize, b: usize, on: bool| {
            let s: String = chars[a..b].iter().collect();
            job.append(
                &s,
                0.0,
                egui::TextFormat {
                    font_id: if on { bold.clone() } else { fid.clone() },
                    color: if on { c.accent } else { c.text_primary },
                    ..Default::default()
                },
            );
        };
        if let Some((a, b)) = cur {
            put(&mut job, i, a, false);
            put(&mut job, a, b, true);
            i = b;
        }
        put(&mut job, i, chars.len(), false);
        let counter = (n > 1).then(|| format!("{}/{n}", h.overload + 1));
        egui::Area::new(egui::Id::unique("jarak-hint"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(egui::pos2(
                caret - space::S,
                p.y - t.view.row_height() / 2.0 - space::XS,
            ))
            .show(ctx, |ui| {
                kim_ui::popover_frame(ctx).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(cn) = &counter {
                            ui.label(RichText::new(cn).size(font::CAPTION).color(c.text_muted));
                        }
                        let r = ui.label(job);
                        if kim_ui::inspect::active() {
                            kim_ui::inspect::record(
                                ui,
                                "hint:signature",
                                "label",
                                r.rect,
                                Some(kim_ui::inspect::Painted {
                                    content: &sig,
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
                    });
                    if !o.descr.is_empty() {
                        ui.label(
                            RichText::new(&o.descr)
                                .size(font::CAPTION)
                                .color(c.text_secondary),
                        );
                    }
                });
            });
    }

    /// The assistance settings and files (the palette's Edit menu): 0 completion while typing,
    /// 1–3 Enter accepts never / after choosing / always (15–17 the same for Tab), 4 parameter hints, 5 snippets in the
    /// list, 6 closing brackets and quotes, 7 closing tags, 8–10 indentation off / keep / smart,
    /// 11 bracket highlighting, 12 open the snippets file, 13 open the API files folder, 14 read
    /// them again.
    pub fn assist_tool(&mut self, k: u8) {
        let s = &mut self.assist;
        let onoff = |b: bool| if b { ("on", "켬") } else { ("off", "끔") };
        let (key, value, what): (&str, String, (&str, &str)) = match k {
            0 => {
                s.complete_auto = !s.complete_auto;
                (
                    "complete_auto",
                    s.complete_auto.to_string(),
                    ("Completion while typing", "입력 중 자동 완성"),
                )
            }
            1..=3 => {
                s.enter = [Accepts::Never, Accepts::Chosen, Accepts::Always][usize::from(k - 1)];
                let v = ["never", "chosen", "always"][usize::from(k - 1)];
                (
                    "complete_enter",
                    v.into(),
                    ("Enter accepts a completion", "Enter로 완성 받기"),
                )
            }
            4 => {
                s.hints = !s.hints;
                (
                    "param_hints",
                    s.hints.to_string(),
                    ("Parameter hints", "매개변수 힌트"),
                )
            }
            5 => {
                s.snippets = !s.snippets;
                (
                    "snippets",
                    s.snippets.to_string(),
                    ("Snippets in the completion list", "완성 목록의 스니펫"),
                )
            }
            6 => {
                s.auto_close = !s.auto_close;
                (
                    "auto_close",
                    s.auto_close.to_string(),
                    ("Closing brackets and quotes", "괄호와 따옴표 자동 닫기"),
                )
            }
            7 => {
                s.close_tags = !s.close_tags;
                (
                    "close_tags",
                    s.close_tags.to_string(),
                    ("Closing HTML/XML tags", "HTML/XML 태그 자동 닫기"),
                )
            }
            8..=10 => {
                s.indent =
                    [IndentMode::Off, IndentMode::Keep, IndentMode::Smart][usize::from(k - 8)];
                let v = ["off", "keep", "smart"][usize::from(k - 8)];
                (
                    "indent",
                    v.into(),
                    ("Indentation on Enter", "Enter 들여쓰기"),
                )
            }
            11 => {
                s.brace_highlight = !s.brace_highlight;
                (
                    "brace_highlight",
                    s.brace_highlight.to_string(),
                    ("Matching bracket highlight", "짝 괄호 강조"),
                )
            }
            12..=14 => {
                self.assist_files(k);
                return;
            }
            15..=17 => {
                s.tab = [Accepts::Never, Accepts::Chosen, Accepts::Always][usize::from(k - 15)];
                let v = ["never", "chosen", "always"][usize::from(k - 15)];
                (
                    "complete_tab",
                    v.into(),
                    ("Tab accepts a completion", "Tab으로 완성 받기"),
                )
            }
            _ => return,
        };
        let shown = match value.as_str() {
            "true" | "false" => {
                let (en, ko) = onoff(value == "true");
                self.t(en, ko).to_string()
            }
            "never" => self.t("never", "받지 않음").into(),
            "chosen" => self
                .t("after choosing with the arrow keys", "화살표로 고른 뒤")
                .into(),
            "always" => self.t("always", "항상").into(),
            "off" => self.t("off", "끔").into(),
            "keep" => self.t("keep the line's", "줄의 들여쓰기 유지").into(),
            _ => self.t("smart", "똑똑하게").into(),
        };
        self.message = Some(format!("{}: {shown}", self.t(what.0, what.1)));
        if let Some(p) = &self.settings
            && let Err(e) = crate::write_setting(p, key, &value)
        {
            self.message = Some(format!(
                "{} {e}",
                self.t("The setting was not saved:", "설정을 저장하지 못했습니다:")
            ));
        }
        // The aids change at once.
        for t in &mut self.tabs {
            t.brace_key = None;
        }
    }

    fn assist_files(&mut self, k: u8) {
        let Some(dir) = self.assist_dir.clone() else {
            self.message = Some(
                self.t("No configuration folder.", "설정 폴더가 없습니다.")
                    .into(),
            );
            return;
        };
        match k {
            12 => {
                let f = dir.join("snippets.json");
                if !f.exists() {
                    // A starting file (never replacing one): one example per language family.
                    let start = "{\n  \"*\": {\n    \"sig\": { \"body\": [\"-- \", \"${1:name}\"], \"description\": \"A signature\" }\n  }\n}\n";
                    let _ = std::fs::create_dir_all(&dir);
                    if let Err(e) = std::fs::write(&f, start) {
                        self.message = Some(format!("{e}"));
                        return;
                    }
                }
                self.open_args(&[f.display().to_string()]);
            }
            13 => {
                let d = dir.join("autoCompletion");
                let _ = std::fs::create_dir_all(&d);
                self.opener.open_file(&d);
            }
            _ => {
                self.reload_assist();
                if self.message.is_none() {
                    self.message = Some(format!(
                        "{} {} · API {}",
                        self.t("Read:", "읽음:"),
                        self.snippets.len(),
                        self.apis.len()
                    ));
                }
            }
        }
    }
}

fn accepts(v: Option<&str>, default: Accepts) -> Accepts {
    match v {
        Some("never") => Accepts::Never,
        Some("chosen") => Accepts::Chosen,
        Some("always") => Accepts::Always,
        _ => default,
    }
}

fn bracket_near(line: &str, col: usize) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    [col, col.wrapping_sub(1)]
        .into_iter()
        .find(|c| chars.get(*c).is_some_and(|ch| "()[]{}".contains(*ch)))
}

fn in_string_or_comment(toks: &[jarak_core::syntax::Token], c: usize) -> bool {
    toks.iter().any(|t| {
        t.start <= c
            && c < t.end
            && matches!(
                t.kind,
                jarak_core::syntax::Kind::String | jarak_core::syntax::Kind::Comment
            )
    })
}
