//! Folding, the Function List and structure navigation in the window (`JARAK-FR-092`,
//! `JARAK-FR-093`, `JARAK-FR-096`, `JARAK-FR-103`, Step 99).
//!
//! - Each view's document has its folds (and, with the Function List open, its symbols) found
//!   again after an edit: at once up to [`SMALL`] lines, else on a worker thread after a pause in
//!   typing, so a keystroke in a large file never waits for the whole text to be lexed. Meanwhile
//!   the folded folds follow their markers.
//! - A folded fold hides its lines: they are not drawn, vertical moves skip them, a caret that
//!   lands inside (a search hit, Go to line, an edit) opens it.
//! - The fold margin's triangles and Notepad++'s keys fold and unfold; the Function List lists
//!   the symbols with a filter; Go to line, matching brace and next / previous symbol move the
//!   caret.

use crate::{JarakApp, views};
use eframe::egui::{self, RichText};
use jarak_core::selection::{Range, Selection};
use jarak_core::structure::{self, Fold, Symbol};
use kim_design::{font, size, space};
use kim_ui::Colors;
use std::sync::{Arc, Mutex};

/// Documents up to this many lines get their structure right after each edit; larger ones on a
/// worker thread after a pause of [`PAUSE`] seconds in typing.
pub const SMALL: usize = 5_000;
pub const PAUSE: f64 = 0.3;

type Found = Arc<Mutex<Option<(Vec<Fold>, Vec<Symbol>)>>>;

/// A document's structure, for one edit generation and language.
#[derive(Debug, Clone, Default)]
pub struct Outline {
    pub folds: Vec<Fold>,
    pub symbols: Vec<Symbol>,
    /// The fold margin's marks: each fold's first line and whether it is folded (shared with
    /// the views).
    pub marks: Arc<Vec<(usize, bool)>>,
    /// The Function List's rows for a filter (indices into `symbols`).
    filtered: Option<(String, Vec<usize>)>,
    /// What the hidden lines were last laid out for (edit and fold generations, the carets).
    applied: Option<(u64, u64, Vec<usize>)>,
    /// The document's line count at that layout.
    applied_total: usize,
    at_gen: u64,
    lang: String,
    with_symbols: bool,
    valid: bool,
    /// The folded folds as (first line, lines below it) when last laid out: while a large
    /// document's structure is found again, they follow their markers.
    spans: Vec<(usize, usize)>,
    /// When the text last changed (its generation, egui seconds), and the worker finding a large
    /// document's structure (for a generation, a language, with symbols).
    changed_at: Option<(u64, f64)>,
    job: Option<(u64, String, bool, Found)>,
    /// How many workers this document started (typing must not start one per keystroke).
    pub jobs_started: u64,
}

impl Outline {
    /// Whether the folds are those of generation `at_gen` in language `lang`.
    pub fn fresh(&self, at_gen: u64, lang: &str) -> bool {
        self.valid && self.at_gen == at_gen && self.lang == lang
    }
}

/// The Go to dialog (a line, or a character offset).
#[derive(Debug, Clone, Default)]
pub struct GotoDialog {
    pub offset: bool,
    pub text: String,
    pub error: Option<String>,
    pub focus: bool,
}

/// The lines of `r` without their line breaks.
fn lines_of_rope(r: &jarak_core::Rope) -> Vec<String> {
    (0..r.len_lines())
        .map(|i| {
            let l = r.line(i);
            l.slice(..jarak_core::text::line_len(l)).to_string()
        })
        .collect()
}

fn lines_of(doc: &jarak_core::document::Document) -> Vec<String> {
    lines_of_rope(doc.text())
}

impl JarakApp {
    /// Every frame, before drawing: the structure of each view's document, its hidden lines and
    /// the margin's marks.
    pub(crate) fn outline_frame(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let mut panes = vec![views::PaneId::Main];
        if self.second.is_some() {
            panes.push(views::PaneId::Second);
        }
        let want = self.function_list;
        for pane in panes {
            let k = self.active_of(pane);
            let lang = self.lang_for(k).cloned();
            let name = lang.as_ref().map(|l| l.name.clone()).unwrap_or_default();
            let t = &mut self.tabs[k];
            let at_gen = t.file.doc.edit_gen();
            let o = &mut t.outline;
            if !(o.fresh(at_gen, &name) && (!want || o.with_symbols)) {
                // Step 109: small in lines and in bytes (a few very long lines go to the worker).
                if t.file.doc.text().len_lines() <= SMALL
                    && t.file.doc.text().len_bytes() <= 2 * 1024 * 1024
                {
                    let (f, s) = structure::outline(lang.as_ref(), &lines_of(&t.file.doc), want);
                    install(t, f, s, at_gen, name, want);
                } else {
                    // One worker per document at a time: while it runs nothing else starts (a
                    // keystroke every frame would otherwise start a new one every frame).
                    let finished = o.job.as_ref().map(|(g, l, w, found)| {
                        let r = found.lock().ok().and_then(|mut x| x.take());
                        (*g, l.clone(), *w, r)
                    });
                    match finished {
                        Some((g, l, w, Some((f, s)))) => {
                            if l == name {
                                // Taken even when the text changed meanwhile: the newest structure
                                // there is (its folded folds follow their markers until the next).
                                install(t, f, s, g, l, w);
                            } else {
                                o.job = None;
                            }
                        }
                        Some((_, _, _, None)) => {
                            ctx.request_repaint_after(std::time::Duration::from_millis(30));
                        }
                        None => {
                            let since = match o.changed_at {
                                Some((g, at)) if g == at_gen => at,
                                _ => {
                                    o.changed_at = Some((at_gen, now));
                                    now
                                }
                            };
                            // At once for a new document or language; after a pause in typing.
                            if !o.valid || o.lang != name || now - since >= PAUSE {
                                let found: Found = Arc::default();
                                o.job = Some((at_gen, name.clone(), want, found.clone()));
                                let rope = t.file.doc.text().clone();
                                let ctx2 = ctx.clone();
                                let spawned = std::thread::Builder::new()
                                    .name("jarak-outline".into())
                                    .spawn(move || {
                                        let r = structure::outline(
                                            lang.as_ref(),
                                            &lines_of_rope(&rope),
                                            want,
                                        );
                                        if let Ok(mut x) = found.lock() {
                                            *x = Some(r);
                                        }
                                        ctx2.request_repaint();
                                    });
                                if spawned.is_err() {
                                    o.job = None;
                                } else {
                                    o.jobs_started += 1;
                                }
                            } else {
                                ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                                    PAUSE,
                                ));
                            }
                        }
                    }
                }
            }
            self.apply_folds(k);
        }
    }

    /// The hidden lines and margin marks of tab `k` from its folded folds; a caret inside one
    /// opens it.
    pub(crate) fn apply_folds(&mut self, k: usize) {
        let t = &mut self.tabs[k];
        let doc = &t.file.doc;
        let o = &t.outline;
        // Nothing changed since the last frame: the hidden lines stand.
        let key = (
            doc.edit_gen(),
            doc.fold_gen(),
            doc.selection()
                .ranges()
                .iter()
                .map(|r| r.head)
                .collect::<Vec<usize>>(),
        );
        if o.applied.as_ref() == Some(&key) {
            return;
        }
        // Only the carets moved, and none into a folded fold: nothing to lay out again.
        let carets_only = o
            .applied
            .as_ref()
            .is_some_and(|(e, f, _)| (*e, *f) == (key.0, key.1));
        if carets_only
            && !doc.selection().ranges().iter().any(|r| {
                let line = doc.text().char_to_line(r.head.min(doc.len_chars()));
                doc.hidden_range(line).is_some()
            })
        {
            t.outline.applied = Some(key);
            return;
        }
        let at_gen = doc.edit_gen();
        let fresh = o.valid && o.at_gen == at_gen;
        let total = doc.text().len_lines();
        // One edit with one caret since the last layout (typing): the folded folds below it move
        // by the lines it added or removed, without asking every marker for its line — unless it
        // touched a fold's first line or removed the lines holding one.
        let single = match &o.applied {
            Some((e, f, _))
                if !fresh
                    && *f == doc.fold_gen()
                    && *e + 1 == at_gen
                    && doc.selection().ranges().len() == 1 =>
            {
                doc.edits_since(*e).map(|p| {
                    let line = doc.text().char_to_line(p.min(doc.len_chars()));
                    (line, total as isize - o.applied_total as isize)
                })
            }
            _ => None,
        };
        let shifted = single
            .filter(|&(line, delta)| {
                !o.spans.iter().any(|&(s, _)| {
                    (s == line && delta != 0)
                        || (delta < 0 && s > line && s as isize <= line as isize - delta)
                })
            })
            .map(|(line, delta)| {
                o.spans
                    .iter()
                    .map(|&(s, n)| {
                        if s > line {
                            ((s as isize + delta) as usize, n)
                        } else {
                            (s, n)
                        }
                    })
                    .collect::<Vec<_>>()
            });
        let mut spans: Vec<(usize, usize)> = match shifted {
            Some(v) => v,
            None => {
                let folded = doc.folded_lines();
                if !fresh && o.spans.len() == folded.len() {
                    // The text changed since the folds were found: each folded fold keeps its
                    // length at where its marker is now.
                    folded
                        .iter()
                        .zip(&o.spans)
                        .map(|(l, (_, n))| (*l, *n))
                        .collect()
                } else {
                    o.folds
                        .iter()
                        .filter(|f| folded.binary_search(&f.start).is_ok())
                        .map(|f| (f.start, f.end - f.start))
                        .collect()
                }
            }
        };
        let last = doc.text().len_lines().saturating_sub(1);
        let carets: Vec<usize> = doc
            .selection()
            .ranges()
            .iter()
            .map(|r| doc.text().char_to_line(r.head.min(doc.len_chars())))
            .collect();
        let mut open = Vec::new();
        spans.retain(|(s, n)| {
            let inside = carets.iter().any(|c| *s < *c && *c <= s + n);
            if inside {
                open.push(*s);
            }
            !inside
        });
        // Step 101: lines the user hid (Alt+H) — like folded folds; a caret inside shows them.
        let mut reveal = Vec::new();
        let blocks: Vec<(usize, usize)> = doc
            .hidden_blocks()
            .into_iter()
            .filter(|(s, n)| {
                let inside = carets.iter().any(|c| *s < *c && *c <= s + n);
                if inside {
                    reveal.push(*s);
                }
                !inside
            })
            .collect();
        // The margin: every fold when they are current; while they are found again, those above
        // the first edit and the folded ones.
        let from = if fresh {
            usize::MAX
        } else {
            doc.edits_since(o.at_gen).map_or(usize::MAX, |p| {
                doc.text().char_to_line(p.min(doc.len_chars()))
            })
        };
        let is_span = |l: usize| spans.binary_search_by_key(&l, |s| s.0).is_ok();
        let mut marks: Vec<(usize, bool)> = if fresh {
            o.folds
                .iter()
                .map(|f| (f.start, is_span(f.start)))
                .collect()
        } else {
            // Two sorted runs (the open folds above the edit, the folded ones): a stable sort
            // merges them in one pass.
            let mut m: Vec<(usize, bool)> = o
                .folds
                .iter()
                .take_while(|f| f.start < from)
                .filter(|f| !is_span(f.start))
                .map(|f| (f.start, false))
                .collect();
            m.extend(spans.iter().map(|(s, _)| (*s, true)));
            m.sort();
            m
        };
        if !blocks.is_empty() {
            marks.extend(blocks.iter().map(|(s, _)| (*s, true)));
            // A folded mark first, so the line's mark says folded.
            marks.sort_unstable_by_key(|m| (m.0, !m.1));
        }
        marks.dedup_by_key(|m| m.0);
        let mut hidden: Vec<(usize, usize)> = spans
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(s, n)| (s + 1, (s + n).min(last)))
            .filter(|(a, b)| a <= b)
            .collect();
        hidden.extend(
            blocks
                .iter()
                .map(|(s, n)| (s + 1, (s + n).min(last)))
                .filter(|(a, b)| a <= b),
        );
        hidden.sort();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in hidden {
            match merged.last_mut() {
                Some(m) if s <= m.1 + 1 => m.1 = m.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        // A fold inside a hidden range has no visible triangle (its line is not drawn).
        let t = &mut self.tabs[k];
        if !open.is_empty() {
            t.file.doc.set_folded_many(&open, false);
        }
        for s in reveal {
            t.file.doc.show_lines(Some(s));
        }
        t.outline.spans = spans;
        t.outline.marks = Arc::new(marks);
        t.outline.applied_total = t.file.doc.text().len_lines();
        t.file.doc.hidden = merged;
        t.outline.applied = Some((
            t.file.doc.edit_gen(),
            t.file.doc.fold_gen(),
            t.file
                .doc
                .selection()
                .ranges()
                .iter()
                .map(|r| r.head)
                .collect(),
        ));
    }

    /// Folds or unfolds the fold starting on `line` of the active document (the margin).
    pub fn toggle_fold(&mut self, line: usize) {
        let k = self.active;
        // Step 101: the mark above hidden lines shows them again.
        if self.tabs[k]
            .file
            .doc
            .hidden_blocks()
            .iter()
            .any(|(s, _)| *s == line)
        {
            self.tabs[k].file.doc.show_lines(Some(line));
            self.apply_folds(k);
            return;
        }
        let folded = self.tabs[k]
            .file
            .doc
            .folded_lines()
            .binary_search(&line)
            .is_ok();
        if self.tabs[k].outline.folds.iter().any(|f| f.start == line) {
            self.tabs[k].file.doc.set_folded(line, !folded);
            self.apply_folds(k);
        }
    }

    /// Folds (`on`) or unfolds: every fold (`level` 0, Alt+0 / Alt+Shift+0), the folds of a level
    /// (1–8, Alt+1…8 / Alt+Shift+1…8) or the innermost fold around the caret (9, Ctrl+Alt+F /
    /// Ctrl+Alt+Shift+F) — Notepad++'s keys.
    pub fn fold_command(&mut self, level: u8, on: bool) {
        let current = level == 9;
        let k = self.active;
        let d = &self.tabs[k].file.doc;
        let caret_line = d.text().char_to_line(d.selection().primary().head);
        let folds = self.tabs[k].outline.folds.clone();
        let chosen: Vec<Fold> = if current {
            folds
                .iter()
                .filter(|f| f.start <= caret_line && caret_line <= f.end)
                .max_by_key(|f| f.level)
                .copied()
                .into_iter()
                .collect()
        } else {
            folds
                .iter()
                .filter(|f| level == 0 || f.level == usize::from(level))
                .copied()
                .collect()
        };
        if chosen.is_empty() {
            self.message = Some(
                self.t("No fold here.", "여기에는 접을 곳이 없습니다.")
                    .into(),
            );
            return;
        }
        // Folding away the caret's line moves the caret to the fold's first line.
        if on
            && let Some(f) = chosen
                .iter()
                .filter(|f| f.start < caret_line && caret_line <= f.end)
                .min_by_key(|f| f.level)
        {
            let p = d.text().line_to_char(f.start);
            self.tabs[k].file.doc.set_selection(&Selection::point(p));
        }
        let starts: Vec<usize> = chosen.iter().map(|f| f.start).collect();
        self.tabs[k].file.doc.set_folded_many(&starts, on);
        self.apply_folds(k);
        self.tabs[k].view.after_edit();
    }

    /// Puts the caret at the start of `line` (0-based), opening a fold around it.
    pub fn go_to_line(&mut self, line: usize) {
        let k = self.active;
        let d = &mut self.tabs[k].file.doc;
        let line = line.min(d.text().len_lines().saturating_sub(1));
        let p = d.text().line_to_char(line);
        d.set_selection(&Selection::point(p));
        self.apply_folds(k);
        self.tabs[k].view.after_edit();
    }

    /// Ctrl+B: to the matching bracket; `select` (Ctrl+Alt+B): select everything between them.
    pub fn match_brace(&mut self, select: bool) {
        let k = self.active;
        let d = &self.tabs[k].file.doc;
        let head = d.selection().primary().head;
        let line = d.text().char_to_line(head);
        let col = head - d.text().line_to_char(line);
        let lines = lines_of(d);
        let found = structure::matching_bracket(self.lang_for(k), &lines, line, col);
        let Some(((l0, c0), (l1, c1))) = found else {
            self.message = Some(
                self.t("No matching bracket.", "짝이 맞는 괄호가 없습니다.")
                    .into(),
            );
            return;
        };
        let d = &mut self.tabs[k].file.doc;
        let p0 = d.text().line_to_char(l0) + c0;
        let p1 = d.text().line_to_char(l1) + c1;
        let sel = if select {
            let (a, b) = if p0 < p1 { (p0 + 1, p1) } else { (p1 + 1, p0) };
            Range::new(a, b)
        } else {
            Range::point(p1)
        };
        d.set_selection(&Selection::single(sel));
        self.apply_folds(k);
        self.tabs[k].view.after_edit();
    }

    /// The symbols of the active document (found now if the Function List has not asked yet).
    fn symbols_now(&mut self) -> Vec<Symbol> {
        let k = self.active;
        if !self.tabs[k].outline.with_symbols
            || self.tabs[k].outline.at_gen != self.tabs[k].file.doc.edit_gen()
        {
            let lines = lines_of(&self.tabs[k].file.doc);
            let syms = structure::symbols(self.lang_for(k), &lines);
            return syms;
        }
        self.tabs[k].outline.symbols.clone()
    }

    /// Next / previous symbol from the caret.
    pub fn symbol_step(&mut self, back: bool) {
        let syms = self.symbols_now();
        let d = &self.tabs[self.active].file.doc;
        let caret_line = d.text().char_to_line(d.selection().primary().head);
        let target = if back {
            syms.iter().rev().find(|s| s.line < caret_line)
        } else {
            syms.iter().find(|s| s.line > caret_line)
        };
        match target {
            Some(s) => {
                let l = s.line;
                self.go_to_line(l);
            }
            None => {
                self.message = Some(
                    self.t("No more symbols.", "더 이상 심볼이 없습니다.")
                        .into(),
                );
            }
        }
    }

    /// The Function List (a right panel): the filter, the symbols indented by depth (only the
    /// rows in view are laid out), the one around the caret marked; a click goes there.
    pub(crate) fn function_list_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        if !self.function_list {
            return;
        }
        let lang = self.lang;
        let k = self.active;
        let mut o = std::mem::take(&mut self.tabs[k].outline);
        let d = &self.tabs[k].file.doc;
        let caret_line = d.text().char_to_line(d.selection().primary().head);
        let current = o
            .symbols
            .partition_point(|s| s.line <= caret_line)
            .checked_sub(1);
        let mut go = None;
        let mut close = false;
        let row_h = size::CONTROL_COMPACT;
        egui::Panel::right("jarak-functions")
            .resizable(true)
            .default_size(260.0)
            .size_range(180.0..=480.0)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::same(space::S as i8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let title = lang.pick("Function list", "함수 목록");
                    let r = ui.label(
                        RichText::new(title)
                            .size(font::CAPTION)
                            .strong()
                            .color(c.text_secondary),
                    );
                    // The panel's title is in the geometry report, so the audit rules check it.
                    if kim_ui::inspect::active() {
                        kim_ui::inspect::record(
                            ui,
                            "functions:title",
                            "label",
                            r.rect,
                            Some(kim_ui::inspect::Painted {
                                content: title,
                                rect: r.rect,
                                size: font::CAPTION,
                                truncated: false,
                            }),
                            false,
                            &[],
                            Some(c.text_secondary),
                            Some(c.bg_raised),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(space::XS);
                let w = ui.available_width();
                kim_ui::text_field(
                    ui,
                    &mut self.function_filter,
                    lang.pick("Filter", "거르기"),
                    "",
                    w,
                );
                ui.add_space(space::XS);
                // The rows for the filter, found again only when it or the symbols change.
                let filter = self.function_filter.to_lowercase();
                if filter.is_empty() {
                    o.filtered = None;
                } else if o.filtered.as_ref().is_none_or(|(f, _)| *f != filter) {
                    let rows = o
                        .symbols
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.name.to_lowercase().contains(&filter))
                        .map(|(i, _)| i)
                        .collect();
                    o.filtered = Some((filter, rows));
                }
                let count = o
                    .filtered
                    .as_ref()
                    .map_or(o.symbols.len(), |(_, r)| r.len());
                if count == 0 {
                    ui.label(
                        RichText::new(lang.pick("No symbols.", "심볼이 없습니다."))
                            .size(font::CAPTION)
                            .color(c.text_muted),
                    );
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(ui, row_h, count, |ui, range| {
                        for n in range {
                            let i = o.filtered.as_ref().map_or(n, |(_, r)| r[n]);
                            let s = &o.symbols[i];
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_h),
                                egui::Sense::click(),
                            );
                            let selected = current == Some(i);
                            if selected {
                                ui.painter().rect_filled(rect, 2.0, c.accent_soft);
                            } else if resp.hovered() {
                                ui.painter().rect_filled(rect, 2.0, c.bg_hover);
                            }
                            let label = format!("{}  {}", s.name, s.kind);
                            let x = rect.left() + space::XS + space::M * s.depth as f32;
                            let g = ui.painter().layout_no_wrap(
                                s.name.clone(),
                                kim_ui::ui_font(ui.ctx(), font::BODY, false),
                                c.text_primary,
                            );
                            let gw = g.size().x;
                            ui.painter().galley(
                                egui::pos2(x, rect.center().y - g.size().y / 2.0),
                                g,
                                c.text_primary,
                            );
                            ui.painter().text(
                                egui::pos2(x + gw + space::S, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                &s.kind,
                                kim_ui::ui_font(ui.ctx(), font::CAPTION, false),
                                c.text_muted,
                            );
                            resp.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone())
                            });
                            if kim_ui::inspect::active() {
                                kim_ui::inspect::record(
                                    ui,
                                    &format!("functions:row#{i}"),
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
                                go = Some(s.line);
                            }
                        }
                    });
            });
        self.tabs[k].outline = o;
        if close {
            self.function_list = false;
        }
        if let Some(l) = go {
            self.go_to_line(l);
            let salt = u8::from(self.second_focused());
            ui.ctx()
                .memory_mut(|m| m.request_focus(crate::view::view_id_for(salt)));
        }
    }

    /// Ctrl+G: the Go to line dialog.
    pub(crate) fn goto_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let lang = self.lang;
        let k = self.active;
        let total = self.tabs[k].file.doc.text().len_lines();
        let chars = self.tabs[k].file.doc.len_chars();
        let d = &self.tabs[k].file.doc;
        let (cur_line, cur_off) = {
            let h = d.selection().primary().head;
            (d.text().char_to_line(h) + 1, h)
        };
        let Some(g) = self.goto.as_mut() else {
            return;
        };
        // The frame it opens in, Enter is the palette's that opened it (not a "Go").
        let fresh = g.focus;
        let (enter, esc) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        let mut act = None;
        kim_ui::dialog_sized(
            ctx,
            "jarak-goto",
            lang.pick("Go to", "이동"),
            360.0,
            |ui| {
                kim_ui::segmented(
                    ui,
                    &mut g.offset,
                    &[
                        (false, lang.pick("Line", "줄")),
                        (true, lang.pick("Offset", "오프셋")),
                    ],
                );
                ui.add_space(space::S);
                let w = ui.available_width();
                let label = if g.offset {
                    lang.pick("Character offset", "문자 오프셋")
                } else {
                    lang.pick("Line number", "줄 번호")
                };
                let r = kim_ui::text_field(ui, &mut g.text, label, "", w);
                if g.focus {
                    r.request_focus();
                    g.focus = false;
                }
                ui.add_space(space::XS);
                let info = if g.offset {
                    format!(
                        "{} {cur_off} · {} {chars}",
                        lang.pick("You are at", "현재"),
                        lang.pick("last", "마지막")
                    )
                } else {
                    format!(
                        "{} {cur_line} · {} {total}",
                        lang.pick("You are on line", "현재 줄"),
                        lang.pick("last", "마지막")
                    )
                };
                let r = ui.label(RichText::new(&info).size(font::CAPTION).color(c.text_muted));
                // Where the caret is, in the geometry report (so the audit rules check it).
                if kim_ui::inspect::active() {
                    kim_ui::inspect::record(
                        ui,
                        "goto:where",
                        "label",
                        r.rect,
                        Some(kim_ui::inspect::Painted {
                            content: &info,
                            rect: r.rect,
                            size: font::CAPTION,
                            truncated: false,
                        }),
                        false,
                        &[],
                        Some(c.text_muted),
                        None,
                    );
                }
                if let Some(e) = &g.error {
                    ui.label(RichText::new(e).size(font::CAPTION).color(c.danger));
                }
                kim_ui::dialog_actions(ui, |ui| {
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                        .clicked()
                    {
                        act = Some(false);
                    }
                    if ui
                        .add(kim_ui::Button::primary(lang.pick("Go", "이동")))
                        .clicked()
                    {
                        act = Some(true);
                    }
                });
            },
        );
        if esc {
            act = Some(false);
        } else if enter && act.is_none() && !fresh {
            act = Some(true);
        }
        match act {
            Some(true) => {
                let g = self.goto.clone().unwrap_or_default();
                let n: Option<usize> = g.text.trim().parse().ok();
                match (n, g.offset) {
                    (Some(n), false) if n >= 1 => {
                        self.goto = None;
                        self.go_to_line(n - 1);
                    }
                    (Some(n), true) => {
                        self.goto = None;
                        let d = &mut self.tabs[k].file.doc;
                        d.set_selection(&Selection::point(n.min(chars)));
                        self.apply_folds(k);
                        self.tabs[k].view.after_edit();
                    }
                    _ => {
                        if let Some(g) = self.goto.as_mut() {
                            g.error =
                                Some(lang.pick("A number, please.", "숫자를 넣으세요.").into());
                        }
                    }
                }
            }
            Some(false) => self.goto = None,
            None => {}
        }
    }
}

/// Takes the structure found for generation `at_gen`: the folded markers of folds that are gone are
/// dropped.
fn install(
    t: &mut crate::Tab,
    folds: Vec<Fold>,
    symbols: Vec<Symbol>,
    at_gen: u64,
    lang: String,
    with_symbols: bool,
) {
    // Only a structure of the text as it is now says which folded folds are gone.
    if at_gen == t.file.doc.edit_gen() {
        let starts: Vec<usize> = folds.iter().map(|f| f.start).collect();
        t.file
            .doc
            .retain_folded(|l| starts.binary_search(&l).is_ok());
    }
    let marks = t.outline.marks.clone();
    let jobs_started = t.outline.jobs_started;
    t.outline = Outline {
        folds,
        symbols,
        marks,
        at_gen,
        lang,
        with_symbols,
        valid: true,
        jobs_started,
        ..Outline::default()
    };
}
