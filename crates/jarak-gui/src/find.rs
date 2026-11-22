//! Find, Replace and Mark in the window (`JARAK-FR-070`…`JARAK-FR-072`, Step 94): the find
//! panel (Ctrl+F / Ctrl+H, a page for marks), F3 / Shift+F3, Ctrl+F3 (select and find), Ctrl+Alt+F3
//! (the word under the caret, without touching the find box), the incremental search bar
//! (Ctrl+Alt+I), highlight all, smart highlighting of the selected word, Mark All with bookmarks,
//! the five token styles and the find history.
//!
//! The search itself is `jarak-core::search`; the text it scans is made once per document state
//! and kept on the tab, so typing in the find box or pressing F3 again does not copy the document.

use crate::JarakApp;
use eframe::egui::{self, RichText};
use jarak_core::document::{BOOKMARK, Marker};
use jarak_core::search::{self, Haystack, Matcher, Mode, Query};
use jarak_core::selection::{Range, Selection};
use kim_design::{font, size, space};
use kim_ui::Colors;
use std::time::{Duration, Instant};

/// The most entries of each history.
pub const HISTORY: usize = 20;
/// The most marks one Mark All or style makes (more would slow every edit down).
pub const MAX_MARKS: usize = 200_000;

/// The panel's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Find,
    Replace,
    Mark,
    /// Step 95: Find and Replace in Files.
    Files,
}

/// The find panel's fields and options (kept while it is closed).
#[derive(Debug, Clone)]
pub struct FindPanel {
    pub open: bool,
    pub page: Page,
    pub find: String,
    pub replace: String,
    pub mode: Mode,
    pub match_case: bool,
    pub whole_word: bool,
    pub wrap: bool,
    pub backward: bool,
    pub in_selection: bool,
    pub dot_all: bool,
    pub highlight_all: bool,
    pub bookmark: bool,
    pub purge: bool,
    pub status: Option<String>,
    /// The find box takes the focus on the next frame.
    pub focus: bool,
    /// Walking through the history with Up/Down.
    history_at: Option<usize>,
    /// Step 95: Find in Files — the folder, the filters (`*.md !*.min.md !+\build`), into
    /// sub-folders and hidden folders, and following the current document's folder.
    pub dir: String,
    pub filters: String,
    pub subfolders: bool,
    pub hidden: bool,
    pub follow_doc: bool,
}

impl Default for FindPanel {
    fn default() -> Self {
        FindPanel {
            open: false,
            page: Page::Find,
            find: String::new(),
            replace: String::new(),
            mode: Mode::Normal,
            match_case: false,
            whole_word: false,
            wrap: true,
            backward: false,
            in_selection: false,
            dot_all: false,
            highlight_all: true,
            bookmark: false,
            purge: true,
            status: None,
            focus: false,
            history_at: None,
            dir: String::new(),
            filters: String::new(),
            subfolders: true,
            hidden: false,
            follow_doc: false,
        }
    }
}

impl FindPanel {
    pub fn query(&self) -> Query {
        Query {
            text: self.find.clone(),
            mode: self.mode,
            match_case: self.match_case,
            whole_word: self.whole_word,
            dot_all: self.dot_all,
        }
    }
}

/// The incremental search bar.
#[derive(Debug, Clone, Default)]
pub struct Incremental {
    pub text: String,
    pub match_case: bool,
    /// Where the search started (the caret when the bar opened).
    pub start: usize,
    pub status: Option<String>,
    pub focus: bool,
}

/// A style command (MARK-04…MARK-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleCmd {
    /// Style all occurrences of the token (style 1–5).
    All(u8),
    /// Style the selected occurrence only.
    One(u8),
    /// Clear a style (0: every style).
    Clear(u8),
    /// Jump to the next / previous styled text.
    Next,
    Previous,
    /// Copy the styled text of every style.
    Copy,
}

/// Adds `s` first to `list` (once, at most [`HISTORY`]).
pub fn remember(list: &mut Vec<String>, s: &str) {
    if s.is_empty() {
        return;
    }
    list.retain(|x| x != s);
    list.insert(0, s.to_string());
    list.truncate(HISTORY);
}

/// The word around `pos` (letters, digits, `_`), as a range.
fn word_at(doc: &jarak_core::document::Document, pos: usize) -> Option<(usize, usize)> {
    let (a, b) = jarak_core::text::word_at(doc.text().slice(..), pos.min(doc.len_chars()));
    let w: String = doc.text().slice(a..b).to_string();
    (b > a && w.chars().all(|c| c.is_alphanumeric() || c == '_')).then_some((a, b))
}

impl JarakApp {
    /// The active document's search text for its current state.
    fn haystack(&mut self) -> Haystack {
        let i = self.active;
        let h = Haystack::reuse(self.tabs[i].hay.as_ref(), &self.tabs[i].file.doc);
        self.tabs[i].hay = Some(h.clone());
        h
    }

    /// The selection the "in selection" option searches (a non-empty primary selection).
    fn search_range(&self) -> Option<(usize, usize)> {
        let r = self.tab().file.doc.selection().primary();
        (self.find.in_selection && !r.is_empty()).then(|| (r.from(), r.to()))
    }

    /// Opens the find panel on a page (Ctrl+F, Ctrl+H, the Mark page); a selected text on one
    /// line becomes the text to find.
    pub fn open_find(&mut self, page: Page) {
        let d = &self.tab().file.doc;
        let sel = d.selected_text();
        if !sel.is_empty() && !sel.contains(['\n', '\r']) && sel.chars().count() < 500 {
            self.find.find = sel;
            self.find.in_selection = false;
        }
        self.find.open = true;
        self.find.page = page;
        self.find.focus = true;
        self.find.status = None;
        // Find in Files starts in the document's folder (or the workspace) when none is set.
        if page == Page::Files && (self.find.dir.is_empty() || self.find.follow_doc) {
            let dir = self
                .tab()
                .file
                .path
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(std::path::Path::to_path_buf)
                .or_else(|| self.workspace.as_ref().map(|w| w.root.clone()));
            if let Some(d) = dir {
                self.find.dir = d.display().to_string();
            }
        }
    }

    fn compiled(&mut self, q: &Query) -> Option<Matcher> {
        match q.compile() {
            Ok(m) => Some(m),
            Err(e) => {
                self.find.status = Some(e.clone());
                self.message = Some(format!("{} {e}", self.t("Find:", "찾기:")));
                None
            }
        }
    }

    /// Selects the next (or previous) match of `q`; `true` when one was found.
    pub fn find_with(&mut self, q: &Query, backward: bool) -> bool {
        let Some(m) = self.compiled(q) else {
            return false;
        };
        let h = self.haystack();
        let r = self.tab().file.doc.selection().primary();
        let range = self.search_range();
        let from = if backward { r.from() } else { r.to() };
        let wrap = self.find.wrap;
        match m.find(&h, from, backward, wrap, range) {
            Some(((a, b), wrapped)) => {
                let t = self.tab_mut();
                t.file
                    .doc
                    .set_selection(&Selection::single(Range::new(a, b)));
                t.view.after_edit();
                self.find.status = wrapped.then(|| {
                    if backward {
                        self.t("Wrapped to the end.", "끝으로 돌아왔습니다.")
                    } else {
                        self.t("Wrapped to the start.", "처음으로 돌아왔습니다.")
                    }
                    .to_string()
                });
                self.message = self.find.status.clone();
                true
            }
            None => {
                let s = format!("{} {}", self.t("Not found:", "찾을 수 없음:"), q.text);
                self.find.status = Some(s.clone());
                self.message = Some(s);
                false
            }
        }
    }

    /// F3 / Shift+F3 (and the panel's Find Next): the find box's text with its options.
    pub fn find_next(&mut self, backward: bool) {
        // Step 102: recordable, with the panel's options.
        self.record_find("next", backward);
        if self.find.find.is_empty() {
            self.open_find(Page::Find);
            return;
        }
        let q = self.find.query();
        remember(&mut self.find_history, &q.text);
        let back = backward != self.find.backward;
        self.find_with(&q, back);
    }

    /// Ctrl+F3 (`select`): the selection or the word at the caret becomes the find text and the
    /// next occurrence is selected; Ctrl+Alt+F3 (`volatile`): the same without touching the find
    /// box (match case and whole word).
    pub fn find_word(&mut self, backward: bool, volatile: bool) {
        let d = &self.tab().file.doc;
        let r = d.selection().primary();
        let range = if r.is_empty() {
            word_at(d, r.head)
        } else {
            Some((r.from(), r.to()))
        };
        let Some((a, b)) = range else {
            return;
        };
        let text = d.text().slice(a..b).to_string();
        if text.contains(['\n', '\r']) {
            return;
        }
        // Start from the word itself, so the next one is found.
        self.tab_mut()
            .file
            .doc
            .set_selection(&Selection::single(Range::new(a, b)));
        let q = if volatile {
            Query {
                text,
                mode: Mode::Normal,
                match_case: true,
                whole_word: true,
                dot_all: false,
            }
        } else {
            self.find.find = text.clone();
            self.find.mode = Mode::Normal;
            remember(&mut self.find_history, &text);
            Query {
                text,
                mode: Mode::Normal,
                match_case: self.find.match_case,
                whole_word: self.find.whole_word,
                dot_all: false,
            }
        };
        let saved = self.find.in_selection;
        self.find.in_selection = false;
        self.find_with(&q, backward);
        self.find.in_selection = saved;
    }

    /// Count: the matches in the document (or the selection).
    pub fn count_matches(&mut self) {
        let q = self.find.query();
        let Some(m) = self.compiled(&q) else {
            return;
        };
        remember(&mut self.find_history, &q.text);
        let h = self.haystack();
        let deadline = Instant::now() + Duration::from_secs(5);
        let (found, done) = m.find_all(&h, self.search_range(), &|| Instant::now() > deadline);
        let s = if done {
            format!("{} {}", self.t("Matches:", "찾은 수:"), found.len())
        } else {
            format!(
                "{} {}+ ({})",
                self.t("Matches:", "찾은 수:"),
                found.len(),
                self.t("stopped after 5 s", "5초 후 멈춤")
            )
        };
        self.find.status = Some(s.clone());
        self.message = Some(s);
    }

    /// Replace: the selected match is replaced, then the next one is selected.
    pub fn replace_one(&mut self) {
        self.record_find("replace", false);
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let q = self.find.query();
        let Some(m) = self.compiled(&q) else {
            return;
        };
        remember(&mut self.find_history, &q.text);
        remember(&mut self.replace_history, &self.find.replace.clone());
        let h = self.haystack();
        let r = self.tab().file.doc.selection().primary();
        // Is the selection exactly a match?
        let (found, _) = m.find_all(&h, Some((r.from(), r.to())), &|| false);
        if found.first() == Some(&(r.from(), r.to())) {
            let with = m.replacement(&h, (r.from(), r.to()), &self.find.replace);
            let n = with.chars().count();
            let t = self.tab_mut();
            t.file
                .doc
                .replace(vec![(r.from(), r.to(), with)], Some(r.from() + n));
            t.view.after_edit();
        }
        let back = self.find.backward;
        self.find_with(&q, back);
    }

    /// Replace All, as one undo step (in the selection with "in selection").
    pub fn replace_all(&mut self) {
        self.record_find("replace_all", false);
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let q = self.find.query();
        let Some(m) = self.compiled(&q) else {
            return;
        };
        remember(&mut self.find_history, &q.text);
        remember(&mut self.replace_history, &self.find.replace.clone());
        let h = self.haystack();
        let range = self.search_range();
        let with = self.find.replace.clone();
        let deadline = Instant::now() + Duration::from_secs(30);
        let i = self.active;
        let r = search::replace_all(&mut self.tabs[i].file.doc, &m, &h, &with, range, &|| {
            Instant::now() > deadline
        });
        let s = match r {
            Ok(n) => format!("{} {n}", self.t("Replaced:", "바꾼 수:")),
            Err(e) => e,
        };
        self.tabs[i].view.after_edit();
        self.find.status = Some(s.clone());
        self.message = Some(s);
    }

    /// Mark All (MARK-01/02): the matches become marks (style 0), and their lines bookmarks
    /// with the option; earlier marks go first with "purge".
    pub fn mark_all(&mut self) {
        let q = self.find.query();
        let Some(m) = self.compiled(&q) else {
            return;
        };
        remember(&mut self.find_history, &q.text);
        let h = self.haystack();
        let deadline = Instant::now() + Duration::from_secs(10);
        let (found, _) = m.find_all(&h, self.search_range(), &|| Instant::now() > deadline);
        let found: Vec<(usize, usize)> = found.into_iter().take(MAX_MARKS).collect();
        let i = self.active;
        let doc = &mut self.tabs[i].file.doc;
        if self.find.purge {
            doc.retain_markers(|mk| mk.kind != 0);
        }
        let lines: Vec<usize> = found
            .iter()
            .map(|(a, _)| doc.text().char_to_line(*a))
            .collect();
        doc.add_markers(found.iter().map(|(a, b)| Marker {
            start: *a,
            end: *b,
            kind: 0,
        }));
        if self.find.bookmark {
            doc.bookmark_lines(lines);
        }
        let s = format!("{} {}", self.t("Marked:", "표시함:"), found.len());
        self.find.status = Some(s.clone());
        self.message = Some(s);
    }

    /// Clears the marks of Mark All (MARK-02).
    pub fn clear_marks(&mut self) {
        self.tab_mut().file.doc.retain_markers(|m| m.kind != 0);
        self.find.status = Some(self.t("Marks cleared.", "표시를 지웠습니다.").to_string());
    }

    /// The text of the markers of `kinds`, one per line (MARK-03, MARK-08).
    fn marked_text(&self, kinds: &[u8]) -> (usize, String) {
        let d = &self.tab().file.doc;
        let mut list: Vec<&Marker> = d
            .markers()
            .iter()
            .filter(|m| kinds.contains(&m.kind))
            .collect();
        list.sort_by_key(|m| m.start);
        let eol = d.eol.as_str();
        let parts: Vec<String> = list
            .iter()
            .map(|m| d.text().slice(m.start..m.end).to_string())
            .collect();
        (parts.len(), parts.join(eol))
    }

    /// Copies the marked text (MARK-03).
    pub fn copy_marked(&mut self) {
        let (n, s) = self.marked_text(&[0]);
        self.clipboard.set_text(&s);
        self.find.status = Some(format!("{} {n}", self.t("Copied marks:", "복사한 표시:")));
        self.message = self.find.status.clone();
    }

    /// The token styles (MARK-04…MARK-08).
    pub fn style(&mut self, cmd: StyleCmd) {
        let i = self.active;
        match cmd {
            StyleCmd::All(k) | StyleCmd::One(k) => {
                let d = &self.tabs[i].file.doc;
                let r = d.selection().primary();
                let range = if r.is_empty() {
                    word_at(d, r.head)
                } else {
                    Some((r.from(), r.to()))
                };
                let Some((a, b)) = range else {
                    return;
                };
                let found = if let StyleCmd::All(_) = cmd {
                    let text = d.text().slice(a..b).to_string();
                    let q = Query {
                        text,
                        mode: Mode::Normal,
                        match_case: true,
                        whole_word: false,
                        dot_all: false,
                    };
                    let Some(m) = self.compiled(&q) else {
                        return;
                    };
                    let h = self.haystack();
                    m.find_all(&h, None, &|| false)
                        .0
                        .into_iter()
                        .take(MAX_MARKS)
                        .collect()
                } else {
                    vec![(a, b)]
                };
                let doc = &mut self.tabs[i].file.doc;
                doc.add_markers(found.iter().map(|(a, b)| Marker {
                    start: *a,
                    end: *b,
                    kind: k,
                }));
                self.message = Some(format!(
                    "{} {k}: {}",
                    self.t("Style", "스타일"),
                    found.len()
                ));
            }
            StyleCmd::Clear(0) => self.tabs[i]
                .file
                .doc
                .retain_markers(|m| !(1..=5).contains(&m.kind)),
            StyleCmd::Clear(k) => self.tabs[i].file.doc.retain_markers(|m| m.kind != k),
            StyleCmd::Next | StyleCmd::Previous => {
                let d = &self.tabs[i].file.doc;
                let r = d.selection().primary();
                let mut list: Vec<(usize, usize)> = d
                    .markers()
                    .iter()
                    .filter(|m| m.kind <= 5)
                    .map(|m| (m.start, m.end))
                    .collect();
                list.sort_unstable();
                list.dedup();
                let pick = if cmd == StyleCmd::Next {
                    list.iter().find(|(a, _)| *a > r.from()).or(list.first())
                } else {
                    list.iter()
                        .rev()
                        .find(|(a, _)| *a < r.from())
                        .or(list.last())
                };
                if let Some(&(a, b)) = pick {
                    let t = &mut self.tabs[i];
                    t.file
                        .doc
                        .set_selection(&Selection::single(Range::new(a, b)));
                    t.view.after_edit();
                }
            }
            StyleCmd::Copy => {
                let (n, s) = self.marked_text(&[1, 2, 3, 4, 5]);
                self.clipboard.set_text(&s);
                self.message = Some(format!(
                    "{} {n}",
                    self.t("Copied styled text:", "복사한 스타일 텍스트:")
                ));
            }
        }
    }

    /// Opens the incremental search bar (Ctrl+Alt+I).
    pub fn open_incremental(&mut self) {
        let start = self.tab().file.doc.selection().primary().from();
        self.incremental = Some(Incremental {
            start,
            focus: true,
            ..Incremental::default()
        });
    }

    /// The incremental search for the bar's text, from where it started (the UI audit).
    pub fn incremental_search_now(&mut self) {
        self.incremental_search(None);
    }

    /// The incremental search after its text changed: the first match from where it started.
    fn incremental_search(&mut self, again: Option<bool>) {
        let Some(inc) = self.incremental.clone() else {
            return;
        };
        if inc.text.is_empty() {
            if let Some(b) = self.incremental.as_mut() {
                b.status = None;
            }
            return;
        }
        let q = Query {
            text: inc.text.clone(),
            mode: Mode::Normal,
            match_case: inc.match_case,
            whole_word: false,
            dot_all: false,
        };
        let Ok(m) = q.compile() else {
            return;
        };
        let h = self.haystack();
        let r = self.tab().file.doc.selection().primary();
        let (from, back) = match again {
            None => (inc.start, false),
            Some(true) => (r.from(), true),
            Some(false) => (r.to(), false),
        };
        let found = m.find(&h, from, back, true, None);
        let small = h.text.len() < 16 << 20;
        let total = small.then(|| m.find_all(&h, None, &|| false).0.len());
        let status = match (found, total) {
            (Some(((a, b), _)), Some(n)) => {
                let t = self.tab_mut();
                t.file
                    .doc
                    .set_selection(&Selection::single(Range::new(a, b)));
                t.view.after_edit();
                format!("{} {n}", self.t("matches:", "찾은 수:"))
            }
            (Some(((a, b), _)), None) => {
                let t = self.tab_mut();
                t.file
                    .doc
                    .set_selection(&Selection::single(Range::new(a, b)));
                t.view.after_edit();
                String::new()
            }
            (None, _) => self.t("not found", "없음").to_string(),
        };
        if let Some(b) = self.incremental.as_mut() {
            b.status = Some(status);
        }
    }

    /// The highlights of the frame: the find text's matches (highlight all), the selected word's
    /// other occurrences (smart highlighting) — on the lines on screen only.
    pub(crate) fn search_marks(&mut self) -> Vec<(usize, usize)> {
        let i = self.active;
        let t = &self.tabs[i];
        let r = t.file.doc.text();
        let first = t.view.top_line.min(r.len_lines().saturating_sub(1));
        let last = (first + t.view.page_lines + 2).min(r.len_lines());
        let a = r.line_to_char(first);
        let b = if last < r.len_lines() {
            r.line_to_char(last)
        } else {
            r.len_chars()
        };
        let mut out = Vec::new();
        let mut queries = Vec::new();
        if let Some(inc) = &self.incremental
            && !inc.text.is_empty()
        {
            queries.push(Query {
                text: inc.text.clone(),
                match_case: inc.match_case,
                ..Query::default()
            });
        } else if self.find.open
            && self.find.page != Page::Mark
            && self.find.highlight_all
            && !self.find.find.is_empty()
        {
            queries.push(self.find.query());
        }
        // Smart highlighting: the selected word (one word, on one line).
        let sel = t.file.doc.selection().primary();
        if !sel.is_empty() && sel.len() < 200 {
            let w = r.slice(sel.from()..sel.to()).to_string();
            if w.chars().all(|c| c.is_alphanumeric() || c == '_') {
                queries.push(Query {
                    text: w,
                    match_case: true,
                    whole_word: true,
                    ..Query::default()
                });
            }
        }
        if queries.is_empty() {
            return out;
        }
        let view_text = r.slice(a..b).to_string();
        let doc = jarak_core::document::Document::from_text(&view_text);
        let h = Haystack::of(&doc);
        for q in queries {
            if let Ok(m) = q.compile() {
                for (x, y) in m.find_all(&h, None, &|| false).0 {
                    if (a + x, a + y) != (sel.from(), sel.to()) {
                        out.push((a + x, a + y));
                    }
                }
            }
        }
        out
    }

    /// The find panel (non-modal, at the top right of the editor).
    pub(crate) fn find_ui(&mut self, ctx: &egui::Context, c: &Colors, area: egui::Rect) {
        // Hidden while the Replace in Files preview (a modal) is open.
        if !self.find.open || self.preview.is_some() {
            return;
        }
        let lang = self.lang;
        let mut act: Option<&'static str> = None;
        let job = self.job.is_some();
        let width = 440.0_f32.min(area.width() - 2.0 * space::M);
        let pos = egui::pos2(area.right() - width - space::M, area.top() + space::M);
        egui::Area::new(egui::Id::unique("jarak-find"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                kim_ui::popover_frame(ctx).show(ui, |ui| {
                    ui.set_width(width);
                    let f = &mut self.find;
                    ui.horizontal(|ui| {
                        kim_ui::segmented(
                            ui,
                            &mut f.page,
                            &[
                                (Page::Find, lang.pick("Find", "찾기")),
                                (Page::Replace, lang.pick("Replace", "바꾸기")),
                                (Page::Mark, lang.pick("Mark", "표시")),
                                (Page::Files, lang.pick("Files", "파일에서")),
                            ],
                        );
                    });
                    ui.add_space(space::S);
                    let w = ui.available_width();
                    let r = kim_ui::text_field(
                        ui,
                        &mut f.find,
                        lang.pick("Find what", "찾을 내용"),
                        "",
                        w,
                    );
                    if f.focus {
                        r.request_focus();
                        f.focus = false;
                    }
                    if r.has_focus() {
                        // Up / Down walk through the history.
                        let (up, down) = ui.input(|i| {
                            (
                                i.key_pressed(egui::Key::ArrowUp),
                                i.key_pressed(egui::Key::ArrowDown),
                            )
                        });
                        if (up || down) && !self.find_history.is_empty() {
                            let n = self.find_history.len();
                            let k = match (f.history_at, up) {
                                (None, true) => 0,
                                (None, false) => n - 1,
                                (Some(k), true) => (k + 1).min(n - 1),
                                (Some(k), false) => k.saturating_sub(1),
                            };
                            f.history_at = Some(k);
                            f.find = self.find_history[k].clone();
                        }
                    }
                    // Enter (Shift+Enter: backwards), with the event's own modifiers.
                    let enter = ui.input(|i| {
                        i.events.iter().find_map(|e| match e {
                            egui::Event::Key {
                                key: egui::Key::Enter,
                                pressed: true,
                                modifiers,
                                ..
                            } => Some(modifiers.shift),
                            _ => None,
                        })
                    });
                    if let Some(shift) = enter
                        && (r.has_focus() || r.lost_focus())
                    {
                        act = Some(match (f.page, shift) {
                            (Page::Files, _) => "find-in-files",
                            (_, true) => "previous",
                            (_, false) => "next",
                        });
                        // The box keeps the focus: Enter again finds the next one.
                        r.request_focus();
                    }
                    let mut boxes_focused = r.has_focus();
                    if matches!(f.page, Page::Replace | Page::Files) {
                        ui.add_space(space::XS);
                        boxes_focused |= kim_ui::text_field(
                            ui,
                            &mut f.replace,
                            lang.pick("Replace with", "바꿀 내용"),
                            "",
                            w,
                        )
                        .has_focus();
                    }
                    if f.page == Page::Files {
                        // The folder (with the folder dialog) and the filters.
                        ui.add_space(space::XS);
                        ui.horizontal(|ui| {
                            let bw = size::CONTROL;
                            kim_ui::text_field(
                                ui,
                                &mut f.dir,
                                lang.pick("Folder", "폴더"),
                                "",
                                (w - bw - space::XS).max(80.0),
                            );
                            if kim_ui::icon_button(
                                ui,
                                kim_ui::Icon::More,
                                lang.pick("Choose a folder", "폴더 고르기"),
                            )
                            .clicked()
                            {
                                act = Some("browse");
                            }
                        });
                        ui.add_space(space::XS);
                        kim_ui::text_field(
                            ui,
                            &mut f.filters,
                            lang.pick(
                                "Filters (*.md !*.min.md !+\\build)",
                                "필터 (*.md !*.min.md !+\\build)",
                            ),
                            "",
                            w,
                        );
                    }
                    // F3 / Shift+F3 work in the panel's boxes too.
                    if boxes_focused {
                        let f3 = ui.input(|i| {
                            i.events.iter().find_map(|e| match e {
                                egui::Event::Key {
                                    key: egui::Key::F3,
                                    pressed: true,
                                    modifiers: m,
                                    ..
                                } if !m.ctrl && !m.command && !m.alt => Some(m.shift),
                                _ => None,
                            })
                        });
                        if let Some(shift) = f3 {
                            act = Some(if shift { "previous" } else { "next" });
                        }
                    }
                    ui.add_space(space::S);
                    kim_ui::segmented(
                        ui,
                        &mut f.mode,
                        &[
                            (Mode::Normal, lang.pick("Normal", "일반")),
                            (
                                Mode::Extended,
                                lang.pick("Extended (\\n \\t)", "확장 (\\n \\t)"),
                            ),
                            (Mode::Regex, lang.pick("Regular expression", "정규식")),
                        ],
                    );
                    ui.add_space(space::XS);
                    ui.horizontal_wrapped(|ui| {
                        kim_ui::checkbox(
                            ui,
                            &mut f.match_case,
                            lang.pick("Match case", "대소문자 구분"),
                        );
                        kim_ui::checkbox(
                            ui,
                            &mut f.whole_word,
                            lang.pick("Whole word", "단어 단위"),
                        );
                        if f.page == Page::Files {
                            kim_ui::checkbox(
                                ui,
                                &mut f.subfolders,
                                lang.pick("In sub-folders", "하위 폴더까지"),
                            );
                            kim_ui::checkbox(
                                ui,
                                &mut f.hidden,
                                lang.pick("In hidden folders", "숨김 폴더까지"),
                            );
                            kim_ui::checkbox(
                                ui,
                                &mut f.follow_doc,
                                lang.pick("Follow the current document", "현재 문서 폴더 따라가기"),
                            );
                        } else {
                            kim_ui::checkbox(
                                ui,
                                &mut f.wrap,
                                lang.pick("Wrap around", "처음부터 계속"),
                            );
                            kim_ui::checkbox(ui, &mut f.backward, lang.pick("Backward", "위로"));
                            kim_ui::checkbox(
                                ui,
                                &mut f.in_selection,
                                lang.pick("In selection", "선택 영역에서"),
                            );
                        }
                        if f.mode == Mode::Regex {
                            kim_ui::checkbox(
                                ui,
                                &mut f.dot_all,
                                lang.pick(". matches newline", ". 이 줄바꿈과 일치"),
                            );
                        }
                        match f.page {
                            Page::Find => {
                                kim_ui::checkbox(
                                    ui,
                                    &mut f.highlight_all,
                                    lang.pick("Highlight all", "모두 강조"),
                                );
                            }
                            Page::Mark => {
                                kim_ui::checkbox(
                                    ui,
                                    &mut f.bookmark,
                                    lang.pick("Bookmark the lines", "줄에 책갈피"),
                                );
                                kim_ui::checkbox(
                                    ui,
                                    &mut f.purge,
                                    lang.pick("Clear marks first", "먼저 표시 지우기"),
                                );
                            }
                            Page::Replace | Page::Files => {}
                        }
                    });
                    // Find All / Replace All over documents (Notepad++'s extra buttons).
                    if matches!(f.page, Page::Find | Page::Replace) {
                        ui.add_space(space::XS);
                        ui.horizontal_wrapped(|ui| {
                            if f.page == Page::Find {
                                if ui
                                    .add(kim_ui::Button::ghost(lang.pick(
                                        "Find all in this document",
                                        "이 문서에서 모두 찾기",
                                    )))
                                    .clicked()
                                {
                                    act = Some("find-all-doc");
                                }
                                if ui
                                    .add(kim_ui::Button::ghost(
                                        lang.pick("In all open documents", "열린 문서 모두에서"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("find-all-open");
                                }
                            } else if ui
                                .add(kim_ui::Button::ghost(lang.pick(
                                    "Replace all in all open documents",
                                    "열린 문서 모두에서 바꾸기",
                                )))
                                .clicked()
                            {
                                act = Some("replace-all-open");
                            }
                        });
                    }
                    if let Some(s) = &f.status {
                        ui.add_space(space::XS);
                        let r =
                            ui.label(RichText::new(s).size(font::CAPTION).color(c.text_secondary));
                        if kim_ui::inspect::active() {
                            kim_ui::inspect::record(
                                ui,
                                "find:status",
                                "label",
                                r.rect,
                                Some(kim_ui::inspect::Painted {
                                    content: s,
                                    rect: r.rect,
                                    size: font::CAPTION,
                                    truncated: false,
                                }),
                                false,
                                &[],
                                Some(c.text_secondary),
                                None,
                            );
                        }
                    }
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            act = Some("close");
                        }
                        match f.page {
                            Page::Files => {
                                if job {
                                    if ui
                                        .add(kim_ui::Button::secondary(lang.pick("Cancel", "취소")))
                                        .clicked()
                                    {
                                        act = Some("cancel-job");
                                    }
                                } else {
                                    if ui
                                        .add(kim_ui::Button::secondary(
                                            lang.pick("Replace in Files…", "파일에서 바꾸기…"),
                                        ))
                                        .clicked()
                                    {
                                        act = Some("replace-in-files");
                                    }
                                    if ui
                                        .add(kim_ui::Button::primary(
                                            lang.pick("Find All", "모두 찾기"),
                                        ))
                                        .clicked()
                                    {
                                        act = Some("find-in-files");
                                    }
                                }
                            }
                            Page::Find => {
                                if ui
                                    .add(kim_ui::Button::secondary(lang.pick("Count", "개수")))
                                    .clicked()
                                {
                                    act = Some("count");
                                }
                                if ui
                                    .add(kim_ui::Button::primary(
                                        lang.pick("Find Next", "다음 찾기"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("next");
                                }
                            }
                            Page::Replace => {
                                if ui
                                    .add(kim_ui::Button::secondary(
                                        lang.pick("Replace All", "모두 바꾸기"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("replace-all");
                                }
                                if ui
                                    .add(kim_ui::Button::secondary(lang.pick("Replace", "바꾸기")))
                                    .clicked()
                                {
                                    act = Some("replace");
                                }
                                if ui
                                    .add(kim_ui::Button::primary(
                                        lang.pick("Find Next", "다음 찾기"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("next");
                                }
                            }
                            Page::Mark => {
                                if ui
                                    .add(kim_ui::Button::secondary(
                                        lang.pick("Copy marked", "표시 복사"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("copy-marked");
                                }
                                if ui
                                    .add(kim_ui::Button::secondary(
                                        lang.pick("Clear marks", "표시 지우기"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("clear-marks");
                                }
                                if ui
                                    .add(kim_ui::Button::primary(
                                        lang.pick("Mark All", "모두 표시"),
                                    ))
                                    .clicked()
                                {
                                    act = Some("mark");
                                }
                            }
                        }
                    });
                    let _ = size::CONTROL;
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Esc stops a running Find in Files first.
            act = Some(if job { "cancel-job" } else { "close" });
        }
        // The dialog's Alt mnemonics, as in Notepad++: Alt+A Replace All, Alt+R Replace, Alt+T
        // Count, Alt+M Mark All, Alt+C match case, Alt+W whole word, Alt+P wrap around, Alt+B
        // backward, Alt+I in selection, Alt+N / Alt+X / Alt+G the Normal / Extended / Regular
        // expression mode, Alt+D `.` matches newline.
        use egui::Key as K;
        // Only while the panel has the keyboard (in the editor Alt+C is the Column Editor).
        let in_view = ctx.memory(|m| m.focused()).is_some_and(|id| {
            id == crate::view::view_id_for(0) || id == crate::view::view_id_for(1)
        });
        let mnemonic = (!in_view)
            .then(|| {
                ctx.input(|i| {
                    i.events.iter().find_map(|e| match e {
                        egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers: m,
                            ..
                        } if m.alt
                            && !m.ctrl
                            && !m.command
                            && !m.shift
                            && [
                                K::A,
                                K::R,
                                K::T,
                                K::M,
                                K::C,
                                K::W,
                                K::P,
                                K::B,
                                K::I,
                                K::N,
                                K::X,
                                K::G,
                                K::D,
                            ]
                            .contains(key) =>
                        {
                            Some(*key)
                        }
                        _ => None,
                    })
                })
            })
            .flatten();
        match (mnemonic, self.find.page) {
            (Some(K::A), Page::Replace) => act = Some("replace-all"),
            (Some(K::R), Page::Replace) => act = Some("replace"),
            (Some(K::T), _) => act = Some("count"),
            (Some(K::M), Page::Mark) => act = Some("mark"),
            (Some(K::C), _) => self.find.match_case ^= true,
            (Some(K::W), _) => self.find.whole_word ^= true,
            (Some(K::P), _) => self.find.wrap ^= true,
            (Some(K::B), _) => self.find.backward ^= true,
            (Some(K::I), _) => self.find.in_selection ^= true,
            (Some(K::N), _) => self.find.mode = Mode::Normal,
            (Some(K::X), _) => self.find.mode = Mode::Extended,
            (Some(K::G), _) => self.find.mode = Mode::Regex,
            (Some(K::D), _) => self.find.dot_all ^= true,
            _ => {}
        }
        match act {
            Some("next") => self.find_next(false),
            Some("previous") => self.find_next(true),
            Some("count") => self.count_matches(),
            Some("replace") => self.replace_one(),
            Some("replace-all") => self.replace_all(),
            Some("mark") => self.mark_all(),
            Some("clear-marks") => self.clear_marks(),
            Some("copy-marked") => self.copy_marked(),
            Some("find-all-doc") => self.find_all_docs(false),
            Some("find-all-open") => self.find_all_docs(true),
            Some("replace-all-open") => self.replace_all_docs(),
            Some("find-in-files") => self.find_in_files(),
            Some("replace-in-files") => self.replace_in_files(),
            Some("cancel-job") => {
                if let Some(j) = &self.job {
                    j.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Some("browse") => {
                if let Some(d) = self.dialogs.folder() {
                    self.find.dir = d.display().to_string();
                    self.find.follow_doc = false;
                }
            }
            Some("close") => {
                self.find.open = false;
                let salt = u8::from(self.second_focused());
                ctx.memory_mut(|m| m.request_focus(crate::view::view_id_for(salt)));
            }
            _ => {}
        }
    }

    /// The incremental search bar (above the status bar).
    pub(crate) fn incremental_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let Some(mut inc) = self.incremental.take() else {
            return;
        };
        let lang = self.lang;
        let mut changed = false;
        let mut step = None;
        let mut close = false;
        egui::Panel::bottom("jarak-incremental")
            .exact_size(size::CONTROL + 2.0 * space::XS)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::S as i8, space::XS as i8)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let r = kim_ui::text_field(
                        ui,
                        &mut inc.text,
                        lang.pick("Incremental search", "점진적 찾기"),
                        "",
                        280.0,
                    );
                    if inc.focus {
                        r.request_focus();
                        inc.focus = false;
                    }
                    changed |= r.changed();
                    if kim_ui::checkbox(
                        ui,
                        &mut inc.match_case,
                        lang.pick("Match case", "대소문자 구분"),
                    )
                    .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Previous", "이전")))
                        .clicked()
                    {
                        step = Some(true);
                    }
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Next", "다음")))
                        .clicked()
                    {
                        step = Some(false);
                    }
                    if let Some(s) = &inc.status {
                        ui.label(RichText::new(s).size(font::CAPTION).color(c.text_secondary));
                    }
                    if r.has_focus() || r.lost_focus() {
                        let (enter, shift, esc) = ui.input(|i| {
                            (
                                i.key_pressed(egui::Key::Enter),
                                i.modifiers.shift,
                                i.key_pressed(egui::Key::Escape),
                            )
                        });
                        if enter {
                            step = Some(shift);
                            r.request_focus();
                        }
                        if esc {
                            close = true;
                        }
                    }
                });
            });
        if close {
            let salt = u8::from(self.second_focused());
            ui.ctx()
                .memory_mut(|m| m.request_focus(crate::view::view_id_for(salt)));
            return;
        }
        self.incremental = Some(inc);
        if changed {
            self.incremental_search(None);
        } else if let Some(back) = step {
            self.incremental_search(Some(back));
        }
    }
}

/// The colours of the marker kinds (0 the Mark All mark, 1–5 the token styles).
pub fn style_color(c: &Colors, kind: u8) -> egui::Color32 {
    let base = match kind {
        0 => c.danger,
        1 => c.info,
        2 => c.warning,
        3 => c.success,
        4 => c.accent,
        _ => egui::Color32::from_rgb(0xE0, 0x7A, 0xB0),
    };
    base.gamma_multiply(0.35)
}

/// The marker kinds the view draws (bookmarks go to the gutter).
pub fn drawn(m: &Marker) -> bool {
    // Step 99: and the folded markers are not drawn either.
    m.kind < BOOKMARK
}
