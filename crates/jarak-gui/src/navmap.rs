//! Quick navigation in the window (`JARAK-FR-100`…`JARAK-FR-104`, Step 101): the Document Map,
//! the navigation history (Alt+Left / Alt+Right), quick open (Ctrl+P) of open documents and the
//! workspace's files, and the bookmark commands that need the clipboard.
//!
//! The map draws one bar per map row from the lines' indentation and length, found again only
//! when the text or the map's size changes; it asks for no repaint of its own.

use crate::JarakApp;
use eframe::egui::{self, RichText};
use jarak_core::selection::Selection;
use kim_design::{font, size, space};
use kim_ui::Colors;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The map's width (points) and how many columns of text it shows across.
pub const MAP_WIDTH: f32 = 112.0;
const MAP_COLS: usize = 120;

/// A document's map rows: (edit generation, lines, rows) → each row's indentation and end column.
#[derive(Debug, Clone, Default)]
pub struct MapCache {
    key: (u64, usize, usize),
    pub rows: Vec<(u16, u16)>,
    /// Lines per row (at least one: a short document has rows of several pixels).
    pub per_row: f32,
}

/// Columns of `line` (its leading whitespace and its end), capped at [`MAP_COLS`].
fn cols(line: impl Iterator<Item = char>, tab: usize) -> (u16, u16) {
    let mut col = 0usize;
    let mut indent = None;
    for c in line.take(MAP_COLS * 2) {
        if c == '\n' || c == '\r' {
            break;
        }
        if indent.is_none() && c != ' ' && c != '\t' {
            indent = Some(col);
        }
        col += if c == '\t' { tab - col % tab } else { 1 };
        if col >= MAP_COLS {
            break;
        }
    }
    let end = col.min(MAP_COLS);
    (indent.unwrap_or(end).min(end) as u16, end as u16)
}

/// Where a navigation history entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub tab: u64,
    pub pos: usize,
    pub line: usize,
}

/// A quick open entry: the name, where it is, and an open document (its index) or a file.
type QuickEntry = (String, String, Result<usize, PathBuf>);

/// The quick open dialog.
#[derive(Debug, Clone, Default)]
pub struct QuickOpen {
    pub query: String,
    pub selected: usize,
    pub focus: bool,
    /// The workspace's files (found on a worker thread), and whether the walk is done.
    files: Arc<Mutex<(Vec<PathBuf>, bool)>>,
    root: Option<PathBuf>,
}

/// How many files quick open lists from the workspace (deeper or more are not walked).
pub const QUICK_MAX: usize = 50_000;
const SKIP: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".svn",
    ".hg",
    "__pycache__",
    ".venv",
];

/// Whether `q`'s characters appear in `s` in order (case-insensitive), with a score: earlier,
/// contiguous and file-name matches first. `None`: no match.
pub fn fuzzy(q: &str, s: &str, name_from: usize) -> Option<i64> {
    if q.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = s.chars().flat_map(char::to_lowercase).collect();
    let mut score = 0i64;
    let mut at = 0usize;
    let mut last: Option<usize> = None;
    for qc in q.chars().flat_map(char::to_lowercase) {
        let p = hay[at..].iter().position(|c| *c == qc)? + at;
        score += 10;
        if last.is_some_and(|l| l + 1 == p) {
            score += 15;
        }
        if p >= name_from {
            score += 5;
        }
        score -= (p - at) as i64 / 4;
        last = Some(p);
        at = p + 1;
    }
    Some(score)
}

impl JarakApp {
    /// The view showing document `i` (the main one when both do).
    fn pane_of(&self, i: usize) -> crate::views::PaneId {
        if self.tabs[i].pane == crate::views::Pane::Second {
            crate::views::PaneId::Second
        } else {
            crate::views::PaneId::Main
        }
    }

    // ---- the Document Map --------------------------------------------------------------------

    /// The map panel (right): the whole document, the lines on screen in a frame; a click or a
    /// drag scrolls there.
    pub(crate) fn doc_map_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        if !self.doc_map {
            return;
        }
        let k = self.active;
        egui::Panel::right("jarak-doc-map")
            .resizable(false)
            .exact_size(MAP_WIDTH)
            .frame(egui::Frame::NONE.fill(c.bg_raised))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let resp = ui.interact(
                    rect,
                    egui::Id::unique("jarak-doc-map-area"),
                    egui::Sense::click_and_drag(),
                );
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::ScrollBar, true, "Document map")
                });
                let t = &mut self.tabs[k];
                let d = &t.file.doc;
                let total = d.text().len_lines().max(1);
                let height = rect.height().max(1.0);
                // A short document: up to 3 points a line; a long one: lines per row.
                let px = (height / total as f32).min(3.0);
                let rows = if px >= 1.0 { total } else { height as usize };
                let per_row = total as f32 / rows.max(1) as f32;
                let key = (d.edit_gen(), total, rows);
                if t.map.key != key {
                    let tab = d.tab_width.max(1);
                    let r = d.text();
                    t.map.rows = (0..rows)
                        .map(|i| {
                            cols(
                                r.line(((i as f32 * per_row) as usize).min(total - 1))
                                    .chars(),
                                tab,
                            )
                        })
                        .collect();
                    t.map.key = key;
                    t.map.per_row = per_row;
                }
                let row_h = if px >= 1.0 { px } else { 1.0 };
                let scale = (rect.width() - 2.0 * space::XS) / MAP_COLS as f32;
                let painter = ui.painter_at(rect);
                let ink = c.text_muted.gamma_multiply(0.55);
                for (i, (a, b)) in t.map.rows.iter().enumerate() {
                    if b <= a {
                        continue;
                    }
                    let y = rect.top() + i as f32 * row_h;
                    let x0 = rect.left() + space::XS + *a as f32 * scale;
                    let x1 = rect.left() + space::XS + *b as f32 * scale;
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x0, y),
                            egui::pos2(x1, y + (row_h * 0.7).max(1.0)),
                        ),
                        0.0,
                        ink,
                    );
                }
                // The lines on screen.
                let top = t.view.top_line as f32;
                let shown = t.view.page_lines.max(1) as f32 + 1.0;
                let y0 = rect.top() + top / per_row * row_h;
                let y1 = rect.top() + (top + shown) / per_row * row_h;
                let frame = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), y0),
                    egui::pos2(rect.right(), y1.max(y0 + 4.0)),
                );
                painter.rect_filled(frame, 0.0, c.accent_soft.gamma_multiply(0.6));
                painter.rect_stroke(
                    frame,
                    0.0,
                    egui::Stroke::new(size::BORDER, c.accent),
                    egui::StrokeKind::Inside,
                );
                if kim_ui::inspect::active() {
                    kim_ui::inspect::record(
                        ui,
                        "docmap:view",
                        "region",
                        frame,
                        None,
                        false,
                        &[],
                        None,
                        None,
                    );
                }
                if (resp.clicked() || resp.dragged())
                    && let Some(p) = resp.interact_pointer_pos()
                {
                    let line =
                        (((p.y - rect.top()) / row_h) * per_row - shown / 2.0).max(0.0) as usize;
                    t.view.top_line = line.min(total.saturating_sub(1));
                    t.view.top_px = 0.0;
                }
            });
    }

    // ---- the navigation history --------------------------------------------------------------

    /// Every frame: a jump (another document, or more than a screen away) remembers where the
    /// caret was.
    pub(crate) fn nav_frame(&mut self) {
        let t = self.tab();
        let d = &t.file.doc;
        let pos = d.selection().primary().head;
        let here = Place {
            tab: t.id,
            pos,
            line: d.text().char_to_line(pos.min(d.len_chars())),
        };
        let page = t.view.page_lines.max(10);
        if let Some(last) = self.nav_last
            && !self.nav_moving
            && (last.tab != here.tab || last.line.abs_diff(here.line) > page)
        {
            self.nav.truncate(self.nav_at);
            if self.nav.last() != Some(&last) {
                self.nav.push(last);
            }
            if self.nav.len() > 100 {
                self.nav.remove(0);
            }
            self.nav_at = self.nav.len();
        }
        self.nav_moving = false;
        self.nav_last = Some(here);
    }

    /// Alt+Left / Alt+Right: back to where a jump came from, or forward again.
    pub fn navigate(&mut self, back: bool) {
        if back {
            if self.nav_at == 0 {
                self.message = Some(
                    self.t("Nothing to go back to.", "돌아갈 곳이 없습니다.")
                        .into(),
                );
                return;
            }
            if self.nav_at == self.nav.len()
                && let Some(here) = self.nav_last
            {
                self.nav.push(here);
            }
            self.nav_at -= 1;
        } else {
            if self.nav_at + 1 >= self.nav.len() {
                self.message = Some(
                    self.t("Nothing to go forward to.", "앞으로 갈 곳이 없습니다.")
                        .into(),
                );
                return;
            }
            self.nav_at += 1;
        }
        let p = self.nav[self.nav_at];
        match self.index_of(p.tab) {
            Some(i) => {
                let pane = self.pane_of(i);
                self.activate(i, pane);
                let d = &mut self.tabs[i].file.doc;
                d.set_selection(&Selection::point(p.pos.min(d.len_chars())));
                self.apply_folds(i);
                self.tabs[i].view.after_edit();
                self.nav_moving = true;
            }
            None => {
                self.message = Some(
                    self.t("That document is closed.", "그 문서는 닫혔습니다.")
                        .into(),
                );
            }
        }
    }

    // ---- quick open --------------------------------------------------------------------------

    /// Ctrl+P: the dialog, the workspace's files found on a worker thread.
    pub fn open_quick(&mut self) {
        let root = self.workspace.as_ref().map(|w| w.root.clone()).or_else(|| {
            self.tab()
                .file
                .path
                .as_deref()
                .and_then(|p| p.parent())
                .map(PathBuf::from)
        });
        let files: Arc<Mutex<(Vec<PathBuf>, bool)>> = Arc::default();
        if let Some(r) = root.clone() {
            let out = files.clone();
            let _ = std::thread::Builder::new()
                .name("jarak-quick-open".into())
                .spawn(move || {
                    let mut stack = vec![(r, 0usize)];
                    let mut found = Vec::new();
                    while let Some((dir, depth)) = stack.pop() {
                        let Ok(rd) = std::fs::read_dir(&dir) else {
                            continue;
                        };
                        for e in rd.flatten() {
                            let name = e.file_name();
                            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
                            if is_dir {
                                if depth < 12 && !SKIP.iter().any(|s| name == *s) {
                                    stack.push((e.path(), depth + 1));
                                }
                            } else {
                                found.push(e.path());
                            }
                            if found.len() >= QUICK_MAX {
                                break;
                            }
                        }
                        if found.len() >= QUICK_MAX {
                            break;
                        }
                    }
                    found.sort();
                    if let Ok(mut x) = out.lock() {
                        *x = (found, true);
                    }
                });
        } else if let Ok(mut x) = files.lock() {
            x.1 = true;
        }
        self.quick = Some(QuickOpen {
            focus: true,
            files,
            root,
            ..QuickOpen::default()
        });
    }

    /// The entries for the query: open documents first, then the workspace's files, best first.
    fn quick_entries(&self, q: &QuickOpen) -> (Vec<QuickEntry>, bool) {
        let mut out: Vec<(i64, String, String, Result<usize, PathBuf>)> = Vec::new();
        let open: Vec<PathBuf> = self
            .tabs
            .iter()
            .filter_map(|t| t.file.path.clone())
            .collect();
        for (i, t) in self.tabs.iter().enumerate() {
            let title = t.file.title();
            let detail = t
                .file
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            if let Some(s) = fuzzy(&q.query, &title, 0) {
                out.push((s + 1000, title, detail, Ok(i)));
            }
        }
        let (files, done) = q
            .files
            .lock()
            .map(|x| (x.0.clone(), x.1))
            .unwrap_or_default();
        let mut file_hits: Vec<(i64, String, String, Result<usize, PathBuf>)> = files
            .iter()
            .filter(|p| !open.contains(p))
            .filter_map(|p| {
                let rel = q
                    .root
                    .as_ref()
                    .and_then(|r| p.strip_prefix(r).ok())
                    .unwrap_or(p)
                    .display()
                    .to_string();
                let name_from = rel
                    .rfind(['\\', '/'])
                    .map_or(0, |i| rel[..i + 1].chars().count());
                fuzzy(&q.query, &rel, name_from).map(|s| {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    (s, name, rel, Err(p.clone()))
                })
            })
            .collect();
        file_hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
        out.sort_by_key(|e| std::cmp::Reverse(e.0));
        out.extend(file_hits.into_iter().take(200));
        (
            out.into_iter().map(|(_, a, b, c)| (a, b, c)).collect(),
            done,
        )
    }

    pub(crate) fn quick_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(mut q) = self.quick.take() else {
            return;
        };
        let (list, done) = self.quick_entries(&q);
        q.selected = q.selected.min(list.len().saturating_sub(1));
        let (up, down, enter, esc) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if up {
            q.selected = q.selected.saturating_sub(1);
        }
        if down && q.selected + 1 < list.len() {
            q.selected += 1;
        }
        let lang = self.lang;
        let mut pick = None;
        let title = self.t("Go to file", "파일로 이동").to_string();
        kim_ui::dialog_sized(ctx, "jarak-quick-open", &title, 560.0, |ui| {
            let w = ui.available_width();
            let r = kim_ui::search_field(
                ui,
                &mut q.query,
                lang.pick("Search open documents and files", "열린 문서와 파일 검색"),
                lang.pick(
                    "Type part of a name: mdrd → README.md",
                    "이름 일부 입력: mdrd → README.md",
                ),
                w,
            );
            if q.focus {
                r.request_focus();
                q.focus = false;
            }
            if r.changed() {
                q.selected = 0;
            }
            ui.add_space(space::S);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (row, (name, detail, _)) in list.iter().enumerate() {
                        let under = ui.painter().add(egui::Shape::Noop);
                        let resp = kim_ui::menu_item(ui, name, false);
                        if row == q.selected {
                            ui.painter().set(
                                under,
                                egui::epaint::RectShape::filled(
                                    resp.rect,
                                    egui::CornerRadius::same(kim_design::radius::S as u8),
                                    c.bg_selected,
                                ),
                            );
                            if up || down {
                                resp.scroll_to_me(None);
                            }
                        }
                        ui.painter().text(
                            egui::pos2(resp.rect.right() - space::S, resp.rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            detail,
                            egui::FontId::proportional(font::CAPTION),
                            c.text_muted,
                        );
                        if resp.clicked() {
                            pick = Some(row);
                        }
                    }
                    if list.is_empty() {
                        let note = if done {
                            lang.pick(
                                "No document or file matches.",
                                "맞는 문서나 파일이 없습니다.",
                            )
                        } else {
                            lang.pick("Looking through the folder…", "폴더를 살펴보는 중…")
                        };
                        ui.label(RichText::new(note).size(font::BODY).color(c.text_muted));
                    }
                });
        });
        if !done {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if esc {
            return;
        }
        if enter && pick.is_none() && !list.is_empty() {
            pick = Some(q.selected);
        }
        match pick.and_then(|i| list.get(i)) {
            Some((_, _, Ok(i))) => {
                let i = *i;
                let pane = self.pane_of(i);
                self.activate(i, pane);
            }
            Some((_, _, Err(p))) => {
                let p = p.display().to_string();
                self.open_args(&[p]);
            }
            None => self.quick = Some(q),
        }
    }

    // ---- bookmark commands that use the clipboard ---------------------------------------------

    /// 0 copy the bookmarked lines, 1 cut them, 2 paste into them (replacing each with the next
    /// clipboard line), 3 open the selected path, 4 show the selected path's folder, 5 the
    /// Summary.
    pub fn bookmark_tool(&mut self, k: u8) {
        let read_only = self.tab().file.read_only;
        match k {
            0 | 1 => {
                let text = self.tab().file.doc.bookmarked_text();
                if text.is_empty() {
                    self.message = Some(
                        self.t("No bookmarked line.", "책갈피가 있는 줄이 없습니다.")
                            .into(),
                    );
                    return;
                }
                if k == 1 && read_only {
                    self.refuse_read_only();
                    return;
                }
                self.clipboard.set_text(&text);
                if k == 1 {
                    let n = self.tab_mut().file.doc.remove_lines_by_bookmark(true);
                    self.message = Some(format!("{} {n}", self.t("Cut lines:", "잘라 낸 줄:")));
                }
            }
            2 => {
                if read_only {
                    self.refuse_read_only();
                    return;
                }
                let Some(text) = self.clipboard.get_text() else {
                    self.message = Some(
                        self.t(
                            "The clipboard has no text.",
                            "클립보드에 텍스트가 없습니다.",
                        )
                        .into(),
                    );
                    return;
                };
                let n = self.tab_mut().file.doc.paste_into_bookmarked(&text);
                if n == 0 {
                    self.message = Some(
                        self.t("No bookmarked line.", "책갈피가 있는 줄이 없습니다.")
                            .into(),
                    );
                }
            }
            3 | 4 => self.open_selected_path(k == 4),
            _ => self.summary_open = true,
        }
        self.tab_mut().view.after_edit();
    }

    /// The selection as a path (relative to the document's folder): opened, or its folder shown.
    fn open_selected_path(&mut self, folder: bool) {
        let d = &self.tab().file.doc;
        let r = d.selection().primary();
        let text = d.text().slice(r.from()..r.to()).to_string();
        let text = text.trim().trim_matches(['"', '\'']);
        if text.is_empty() || text.contains('\n') {
            self.message = Some(
                self.t("Select a path first.", "먼저 경로를 선택하세요.")
                    .into(),
            );
            return;
        }
        let p = std::path::Path::new(text);
        let full = if p.is_absolute() {
            p.to_path_buf()
        } else {
            match self.tab().file.path.as_deref().and_then(|x| x.parent()) {
                Some(dir) => dir.join(p),
                None => p.to_path_buf(),
            }
        };
        if !full.exists() {
            self.message = Some(format!(
                "{} {}",
                self.t("Not found:", "없음:"),
                full.display()
            ));
            return;
        }
        if folder || full.is_dir() {
            self.reveal(&full);
        } else {
            self.open_args(&[full.display().to_string()]);
        }
    }

    /// The Summary dialog (Notepad++'s View → Summary).
    pub(crate) fn summary_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.summary_open {
            return;
        }
        let t = self.tab();
        let fmt = jarak_core::file::Format {
            bom: false,
            ..t.file.format
        };
        let mut s = t
            .file
            .doc
            .summary(|chunk| jarak_core::file::encode(chunk, fmt).map_or(chunk.len(), |b| b.len()));
        if t.file.format.bom {
            s.bytes += jarak_core::file::encode("", t.file.format).map_or(0, |b| b.len());
        }
        let path = t
            .file
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| self.t("(not saved yet)", "(아직 저장 안 함)").into());
        let (modified, created) = t
            .file
            .path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| (m.modified().ok().map(when), m.created().ok().map(when)))
            .unwrap_or((None, None));
        let lang = self.lang;
        let rows: Vec<(&str, String)> = vec![
            (lang.pick("Full path", "전체 경로"), path),
            (lang.pick("Created", "만든 때"), created.unwrap_or_default()),
            (
                lang.pick("Modified", "고친 때"),
                modified.unwrap_or_default(),
            ),
            (
                lang.pick("Characters (without line endings)", "글자(줄 끝 제외)"),
                s.chars.to_string(),
            ),
            (lang.pick("Words", "단어"), s.words.to_string()),
            (lang.pick("Lines", "줄"), s.lines.to_string()),
            (
                lang.pick("Document length (bytes)", "문서 길이(바이트)"),
                s.bytes.to_string(),
            ),
            (
                lang.pick("Selected characters", "선택한 글자"),
                format!(
                    "{} ({} {})",
                    s.selected_chars,
                    s.selected_ranges,
                    lang.pick("ranges", "구간")
                ),
            ),
        ];
        // Esc or the button closes it (Enter may be the palette's that opened it this frame).
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let title = self.t("Summary", "요약").to_string();
        kim_ui::dialog_sized(ctx, "jarak-summary", &title, 480.0, |ui| {
            egui::Grid::new("jarak-summary-grid")
                .num_columns(2)
                .spacing([space::L, space::XS])
                .show(ui, |ui| {
                    for (k, v) in &rows {
                        let rk =
                            ui.label(RichText::new(*k).size(font::BODY).color(c.text_secondary));
                        let rv = ui.label(RichText::new(v).size(font::BODY).color(c.text_primary));
                        // The rows are in the geometry report (the UI audit, e2e).
                        if kim_ui::inspect::active() {
                            for (n, r, text, col) in [
                                (0, rk.rect, *k, c.text_secondary),
                                (1, rv.rect, v.as_str(), c.text_primary),
                            ] {
                                kim_ui::inspect::record(
                                    ui,
                                    &format!("summary:{k}#{n}"),
                                    "label",
                                    r,
                                    Some(kim_ui::inspect::Painted {
                                        content: text,
                                        rect: r,
                                        size: font::BODY,
                                        truncated: false,
                                    }),
                                    false,
                                    &[],
                                    Some(col),
                                    None,
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
            kim_ui::dialog_actions(ui, |ui| {
                if ui
                    .add(kim_ui::Button::primary(lang.pick("Close", "닫기")))
                    .clicked()
                {
                    close = true;
                }
            });
        });
        if close {
            self.summary_open = false;
        }
    }
}

/// A file time as local `YYYY-MM-DD HH:MM:SS` (the PC's current offset from UTC).
fn when(t: std::time::SystemTime) -> String {
    let utc = t
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (y, mo, d, h, mi, s) = civil(utc + local_offset());
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Seconds since 1970 → (year, month, day, hour, minute, second), UTC.
fn civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (rest / 3600) as u32,
        (rest % 3600 / 60) as u32,
        (rest % 60) as u32,
    )
}

/// The PC's offset from UTC now (seconds; local time minus UTC).
fn local_offset() -> i64 {
    let local = crate::hashdlg::now();
    let utc = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (y, mo, d, h, mi, _) = civil(utc);
    let as_minutes = |y: i64, mo: u32, d: u32, h: u32, mi: u32| -> i64 {
        // Days from a fixed epoch are enough for a difference of at most a day.
        ((y * 372 + i64::from(mo) * 31 + i64::from(d)) * 24 + i64::from(h)) * 60 + i64::from(mi)
    };
    let diff = as_minutes(
        i64::from(local.year),
        local.month,
        local.day,
        local.hour,
        local.minute,
    ) - as_minutes(y, mo, d, h, mi);
    // Month lengths make the day count above wrong across a month's end: keep it within a day,
    // rounded to a quarter of an hour.
    let diff = ((diff + 12 * 60).rem_euclid(24 * 60)) - 12 * 60;
    (diff as f64 / 15.0).round() as i64 * 15 * 60
}
