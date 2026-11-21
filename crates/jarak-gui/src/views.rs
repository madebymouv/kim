//! Two views (`JARAK-FR-061`, Step 93): the main view and an optional second view, side by side
//! or one above the other, each with its tab bar.
//!
//! A document is in the main view, the second view, or both (a clone). A clone is one buffer —
//! the same text, undo history and file — with a view of its own in each pane: its scroll, mode
//! and carets. `Tab::view` (and the document's selection) always belong to the pane that has the
//! focus; the other pane's are kept in `Tab::alt` and swapped when the focus moves, so the rest
//! of the window works on "the active document" as before.

use crate::{JarakApp, Mode, Tab, preview, tabs, view};
use eframe::egui::{self, Rect};
use jarak_core::file::FileDocument;
use jarak_core::selection::Selection;
use kim_design::size;
use kim_ui::Colors;

/// A view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneId {
    Main,
    Second,
}

/// Which views show a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Main,
    Second,
    Both,
}

/// The other pane's view of a cloned document.
#[derive(Debug, Clone)]
pub struct PaneState {
    pub view: view::ViewState,
    pub mode: Mode,
    pub top_block: usize,
    pub top_px: f32,
    pub sel: Selection,
    /// The carets as lines and columns (characters): edits in the other view leave them on
    /// their line instead of shifting them by character positions.
    pub lc: LineCols,
}

/// Anchors and heads as (line, column) pairs.
pub type LineCols = Vec<((usize, usize), (usize, usize))>;

/// A selection as (line, column) pairs of anchor and head.
pub fn to_lc(doc: &jarak_core::document::Document, sel: &Selection) -> LineCols {
    let r = doc.text();
    let lc = |p: usize| {
        let p = p.min(r.len_chars());
        let l = r.char_to_line(p);
        (l, p - r.line_to_char(l))
    };
    sel.ranges()
        .iter()
        .map(|x| (lc(x.anchor), lc(x.head)))
        .collect()
}

/// (line, column) pairs back to a selection of `doc` (clamped to the text).
pub fn from_lc(doc: &jarak_core::document::Document, lc: &LineCols, primary: usize) -> Selection {
    let r = doc.text();
    let pos = |(l, c): (usize, usize)| {
        let l = l.min(r.len_lines().saturating_sub(1));
        let start = r.line_to_char(l);
        start + c.min(jarak_core::text::line_len(r.line(l)))
    };
    let ranges: Vec<jarak_core::selection::Range> = lc
        .iter()
        .map(|(a, h)| jarak_core::selection::Range::new(pos(*a), pos(*h)))
        .collect();
    if ranges.is_empty() {
        Selection::point(0)
    } else {
        Selection::new(ranges, primary)
    }
}

/// The second view.
#[derive(Debug, Clone)]
pub struct Second {
    /// Its documents in tab order (document ids).
    pub order: Vec<u64>,
    /// Its active document.
    pub active: u64,
    /// Side by side (`true`) or one above the other.
    pub vertical: bool,
    /// The second view has the keyboard focus.
    pub focus: bool,
    /// The share of the first view (0.2 … 0.8).
    pub ratio: f32,
    /// Synchronized scrolling (the other view keeps its distance in lines / points).
    pub sync_v: Option<isize>,
    pub sync_h: Option<f32>,
}

impl Second {
    fn new(id: u64) -> Second {
        Second {
            order: vec![id],
            active: id,
            vertical: true,
            focus: false,
            ratio: 0.5,
            sync_v: None,
            sync_h: None,
        }
    }
}

/// What drawing the documents gave.
#[derive(Debug, Default)]
pub struct Drawn {
    pub focused: bool,
    pub clicked: bool,
    pub link: preview::PreviewOutput,
    pub ctrl_click: Option<usize>,
}

impl JarakApp {
    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// The second view has the focus.
    pub fn second_focused(&self) -> bool {
        self.second.as_ref().is_some_and(|s| s.focus)
    }

    pub fn focused_pane(&self) -> PaneId {
        if self.second_focused() {
            PaneId::Second
        } else {
            PaneId::Main
        }
    }

    /// The documents of a view in tab order (indices into `tabs`).
    pub fn order(&self, pane: PaneId) -> Vec<usize> {
        match pane {
            PaneId::Main => (0..self.tabs.len())
                .filter(|i| self.tabs[*i].pane != Pane::Second)
                .collect(),
            PaneId::Second => self
                .second
                .as_ref()
                .map(|s| s.order.iter().filter_map(|id| self.index_of(*id)).collect())
                .unwrap_or_default(),
        }
    }

    /// The active document of a view.
    pub fn active_of(&self, pane: PaneId) -> usize {
        if pane == self.focused_pane() {
            return self.active;
        }
        let id = match pane {
            PaneId::Main => self.main_active,
            PaneId::Second => self.second.as_ref().map_or(0, |s| s.active),
        };
        self.index_of(id).unwrap_or(self.active)
    }

    /// Swaps a clone's focused-pane state with the other pane's.
    pub(crate) fn swap_alt(&mut self, i: usize) {
        let t = &mut self.tabs[i];
        let Some(a) = t.alt.as_mut() else {
            return;
        };
        std::mem::swap(&mut t.view, &mut a.view);
        std::mem::swap(&mut t.mode, &mut a.mode);
        std::mem::swap(&mut t.preview.top_block, &mut a.top_block);
        std::mem::swap(&mut t.preview.top_px, &mut a.top_px);
        let restored = from_lc(&t.file.doc, &a.lc, a.sel.primary_index());
        let cur = t.file.doc.swap_selection(&restored);
        a.lc = to_lc(&t.file.doc, &cur);
        a.sel = cur;
    }

    /// Moves the keyboard focus to a view.
    pub fn focus_pane(&mut self, pane: PaneId) {
        let want = pane == PaneId::Second;
        let Some(s) = self.second.as_ref() else {
            return;
        };
        if s.focus == want {
            return;
        }
        let cur = self.tabs[self.active].id;
        for i in 0..self.tabs.len() {
            if self.tabs[i].pane == Pane::Both {
                self.swap_alt(i);
            }
        }
        let main_active = self.main_active;
        let s = self.second.as_mut().unwrap_or_else(|| unreachable!());
        if want {
            self.main_active = cur;
            s.focus = true;
            let a = s.active;
            self.active = self.index_of(a).unwrap_or(self.active);
        } else {
            s.active = cur;
            s.focus = false;
            self.active = self.index_of(main_active).unwrap_or(0);
        }
    }

    /// Shows document `i` in a view and gives that view the focus.
    pub fn activate(&mut self, i: usize, pane: PaneId) {
        self.focus_pane(pane);
        self.active = i;
        if pane == PaneId::Second
            && let Some(s) = self.second.as_mut()
        {
            s.active = self.tabs[i].id;
        }
    }

    /// Restores the invariants after documents moved or closed: the main view has a document,
    /// an empty second view goes away, the active documents exist where they are shown.
    pub(crate) fn normalize(&mut self) {
        if let Some(s) = self.second.as_mut() {
            let ids: Vec<u64> = self.tabs.iter().map(|t| t.id).collect();
            s.order.retain(|id| ids.contains(id));
        }
        if self.second.as_ref().is_some_and(|s| s.order.is_empty()) {
            let was_focus = self.second_focused();
            self.second = None;
            for t in &mut self.tabs {
                if t.pane != Pane::Main {
                    t.pane = Pane::Main;
                    t.alt = None;
                }
            }
            if was_focus {
                self.active = self.index_of(self.main_active).unwrap_or(0);
            }
        }
        if self.order(PaneId::Main).is_empty() {
            let t = Tab::new(FileDocument::untitled());
            self.main_active = t.id;
            self.tabs.push(t);
            if !self.second_focused() {
                self.active = self.tabs.len() - 1;
            }
        }
        let main = self.order(PaneId::Main);
        if self
            .index_of(self.main_active)
            .is_none_or(|i| !main.contains(&i))
        {
            self.main_active = self.tabs[main[0]].id;
        }
        if let Some(s) = self.second.as_ref() {
            let sec = self.order(PaneId::Second);
            if self.index_of(s.active).is_none_or(|i| !sec.contains(&i)) {
                let id = self.tabs[sec[0]].id;
                if let Some(s) = self.second.as_mut() {
                    s.active = id;
                }
            }
        }
        let pane = self.focused_pane();
        let order = self.order(pane);
        if self.active >= self.tabs.len() || !order.contains(&self.active) {
            self.active = match pane {
                PaneId::Main => self.index_of(self.main_active).unwrap_or(order[0]),
                PaneId::Second => self
                    .second
                    .as_ref()
                    .and_then(|s| self.index_of(s.active))
                    .unwrap_or(order[0]),
            };
        }
    }

    /// The document next to `i` in a view's order (for a new active after a removal).
    fn neighbor(&self, pane: PaneId, i: usize) -> Option<usize> {
        let o = self.order(pane);
        let k = o.iter().position(|x| *x == i)?;
        if k > 0 {
            o.get(k - 1).copied()
        } else {
            o.get(k + 1).copied()
        }
    }

    /// Moves the active document to the other view (`VIEWS-01`).
    pub fn move_to_other(&mut self) {
        let i = self.active;
        let id = self.tabs[i].id;
        let from = self.focused_pane();
        let to = if from == PaneId::Main {
            PaneId::Second
        } else {
            PaneId::Main
        };
        match self.tabs[i].pane {
            Pane::Both => {
                // It stays in the other view only (with that view's state).
                let next = self.neighbor(from, i);
                self.swap_alt(i);
                self.tabs[i].alt = None;
                self.tabs[i].pane = if from == PaneId::Main {
                    Pane::Second
                } else {
                    Pane::Main
                };
                if let Some(s) = self.second.as_mut()
                    && from == PaneId::Second
                {
                    s.order.retain(|x| *x != id);
                }
                // The view left behind shows its neighbour; the focus follows the document.
                if from == PaneId::Main {
                    if let Some(n) = next {
                        self.active = n;
                    }
                    self.normalize();
                    let now = self.index_of(id).unwrap_or(0);
                    self.activate(now, to);
                } else {
                    self.normalize();
                    let now = self.index_of(id).unwrap_or(0);
                    self.activate(now, PaneId::Main);
                }
            }
            Pane::Main => {
                let next = self.neighbor(PaneId::Main, i);
                match self.second.as_mut() {
                    Some(s) => {
                        s.order.push(id);
                        s.active = id;
                    }
                    None => self.second = Some(Second::new(id)),
                }
                self.tabs[i].pane = Pane::Second;
                let keep = match next {
                    Some(n) => self.tabs[n].id,
                    None => {
                        let t = Tab::new(FileDocument::untitled());
                        let nid = t.id;
                        self.tabs.push(t);
                        nid
                    }
                };
                self.focus_pane(PaneId::Second);
                self.main_active = keep;
                let now = self.index_of(id).unwrap_or(0);
                self.activate(now, PaneId::Second);
            }
            Pane::Second => {
                let next = self.neighbor(PaneId::Second, i);
                self.tabs[i].pane = Pane::Main;
                if let Some(s) = self.second.as_mut() {
                    s.order.retain(|x| *x != id);
                    if let Some(n) = next {
                        s.active = self.tabs[n].id;
                    }
                }
                self.normalize();
                let now = self.index_of(id).unwrap_or(0);
                if self.second.is_some() {
                    self.activate(now, PaneId::Main);
                } else {
                    self.active = now;
                }
            }
        }
    }

    /// Shows the active document in the other view too (`VIEWS-02`): one buffer, two views.
    pub fn clone_to_other(&mut self) {
        let i = self.active;
        if self.tabs[i].pane == Pane::Both {
            self.message = Some(
                self.t(
                    "The document is in both views already.",
                    "문서가 이미 두 보기에 있습니다.",
                )
                .to_string(),
            );
            return;
        }
        let id = self.tabs[i].id;
        let from = self.focused_pane();
        let t = &mut self.tabs[i];
        let state = PaneState {
            view: t.view.clone(),
            mode: t.mode,
            top_block: t.preview.top_block,
            top_px: t.preview.top_px,
            sel: t.file.doc.selection().clone(),
            lc: to_lc(&t.file.doc, t.file.doc.selection()),
        };
        t.alt = Some(Box::new(state));
        t.pane = Pane::Both;
        match from {
            PaneId::Main => {
                match self.second.as_mut() {
                    Some(s) => {
                        s.order.push(id);
                        s.active = id;
                    }
                    None => self.second = Some(Second::new(id)),
                }
                self.focus_pane(PaneId::Second);
            }
            PaneId::Second => {
                self.main_active = id;
                self.focus_pane(PaneId::Main);
            }
        }
        let now = self.index_of(id).unwrap_or(0);
        self.active = now;
    }

    /// Closes document `i` in one view: a clone leaves that view only; otherwise the document
    /// closes (asking first when it has unsaved changes).
    pub fn close_in_pane(&mut self, i: usize, pane: PaneId) {
        if self.tabs[i].pane == Pane::Both {
            let id = self.tabs[i].id;
            let focused = self.focused_pane() == pane;
            let next = self.neighbor(pane, i);
            if focused {
                self.swap_alt(i);
            }
            self.tabs[i].alt = None;
            self.tabs[i].pane = if pane == PaneId::Main {
                Pane::Second
            } else {
                Pane::Main
            };
            if pane == PaneId::Second
                && let Some(s) = self.second.as_mut()
            {
                s.order.retain(|x| *x != id);
            }
            if focused {
                if let Some(n) = next {
                    self.active = n;
                }
            } else if pane == PaneId::Main {
                if let Some(n) = next {
                    self.main_active = self.tabs[n].id;
                }
            } else if let (Some(n), Some(s)) = (next, self.second.as_mut()) {
                s.active = self.tabs[n].id;
            }
            self.normalize();
            return;
        }
        self.activate(i, pane);
        self.request_close(i);
    }

    /// Splits the views side by side or one above the other.
    pub fn toggle_orientation(&mut self) {
        if let Some(s) = self.second.as_mut() {
            s.vertical = !s.vertical;
        }
    }

    /// Moves the active tab one place left (`-1`) or right (`1`) in its view (`VIEWS-07`).
    pub fn move_tab(&mut self, d: isize) {
        let pane = self.focused_pane();
        let order = self.order(pane);
        let Some(k) = order.iter().position(|x| *x == self.active) else {
            return;
        };
        let j = k as isize + d;
        if j < 0 || j as usize >= order.len() {
            return;
        }
        self.reorder(pane, k, j as usize);
    }

    /// Moves the tab at `from` to `to` in a view's order.
    pub fn reorder(&mut self, pane: PaneId, from: usize, to: usize) {
        match pane {
            PaneId::Second => {
                if let Some(s) = self.second.as_mut()
                    && from < s.order.len()
                    && to < s.order.len()
                {
                    let id = s.order.remove(from);
                    s.order.insert(to, id);
                }
            }
            PaneId::Main => {
                let order = self.order(PaneId::Main);
                let (Some(&a), Some(&b)) = (order.get(from), order.get(to)) else {
                    return;
                };
                let active_id = self.tabs[self.active].id;
                let t = self.tabs.remove(a);
                self.tabs.insert(b.min(self.tabs.len()), t);
                self.active = self.index_of(active_id).unwrap_or(0);
            }
        }
    }

    /// Goes to the `n`th tab (1-based) of the focused view (`VIEWS-06`; 9 is the last).
    pub fn go_to_tab(&mut self, n: usize) {
        let pane = self.focused_pane();
        let order = self.order(pane);
        if order.is_empty() {
            return;
        }
        let k = if n >= 9 {
            order.len() - 1
        } else {
            (n - 1).min(order.len() - 1)
        };
        self.activate(order[k], pane);
    }

    /// Next / previous tab of the focused view.
    pub fn cycle_tab(&mut self, d: isize) {
        let pane = self.focused_pane();
        let order = self.order(pane);
        let Some(k) = order.iter().position(|x| *x == self.active) else {
            return;
        };
        let n = order.len() as isize;
        let j = ((k as isize + d) % n + n) % n;
        self.activate(order[j as usize], pane);
    }

    /// The tab bars are shown when there is more than one document or a second view.
    pub fn tab_bars_shown(&self) -> bool {
        self.tabs.len() > 1 || self.second.is_some()
    }

    /// Draws the views (their tab bars and documents) into `rect`.
    pub(crate) fn draw_views(&mut self, ui: &mut egui::Ui, rect: Rect, c: &Colors) -> Drawn {
        let mut drawn = Drawn::default();
        let panes: Vec<(PaneId, Rect)> = match self.second.as_ref() {
            None => vec![(PaneId::Main, rect)],
            Some(s) => {
                let r = s.ratio.clamp(0.2, 0.8);
                if s.vertical {
                    let x = (rect.left() + rect.width() * r).round();
                    vec![
                        (
                            PaneId::Main,
                            Rect::from_min_max(rect.min, egui::pos2(x, rect.bottom())),
                        ),
                        (
                            PaneId::Second,
                            Rect::from_min_max(egui::pos2(x + 1.0, rect.top()), rect.max),
                        ),
                    ]
                } else {
                    let y = (rect.top() + rect.height() * r).round();
                    vec![
                        (
                            PaneId::Main,
                            Rect::from_min_max(rect.min, egui::pos2(rect.right(), y)),
                        ),
                        (
                            PaneId::Second,
                            Rect::from_min_max(egui::pos2(rect.left(), y + 1.0), rect.max),
                        ),
                    ]
                }
            }
        };
        let mut focus_to = None;
        let mut events: Vec<(PaneId, tabs::Event)> = Vec::new();
        for (pane, prect) in &panes {
            let pane = *pane;
            let order = self.order(pane);
            if order.is_empty() {
                continue;
            }
            let i = self.active_of(pane);
            let mut content = *prect;
            if self.tab_bars_shown() {
                let bar = Rect::from_min_size(
                    prect.min,
                    egui::vec2(prect.width(), size::TABBAR_HEIGHT_COMPACT),
                );
                content.min.y = bar.bottom();
                let items: Vec<tabs::Item> = order
                    .iter()
                    .map(|k| {
                        let t = &self.tabs[*k];
                        tabs::Item {
                            title: t.file.title(),
                            path: t.file.path.as_ref().map(|p| p.display().to_string()),
                            dirty: t.file.doc.is_dirty(),
                            active: *k == i,
                            cloned: t.pane == Pane::Both,
                        }
                    })
                    .collect();
                let id = if pane == PaneId::Main {
                    "main"
                } else {
                    "second"
                };
                let focused = pane == self.focused_pane();
                for e in tabs::bar(ui, bar, &items, id, self.lang, focused) {
                    events.push((pane, e));
                }
            }
            let focused_pane = pane == self.focused_pane();
            let clone_unfocused = !focused_pane && self.tabs[i].pane == Pane::Both;
            if clone_unfocused {
                self.swap_alt(i);
            }
            let salt = u8::from(pane == PaneId::Second);
            let out = self.draw_doc(ui, content, i, salt, c);
            if clone_unfocused {
                self.swap_alt(i);
            }
            if focused_pane {
                drawn.focused = out.focused;
                drawn.link = out.link;
                drawn.ctrl_click = out.ctrl_click;
            } else if out.clicked || out.link != preview::PreviewOutput::default() {
                // A click in the other view moves the focus there.
                focus_to = Some(pane);
            }
        }
        if panes.len() == 2 {
            let (a, b) = (panes[0].1, panes[1].1);
            let vertical = self.second.as_ref().is_some_and(|s| s.vertical);
            let line = if vertical {
                Rect::from_min_max(
                    egui::pos2(a.right(), rect.top()),
                    egui::pos2(b.left(), rect.bottom()),
                )
            } else {
                Rect::from_min_max(
                    egui::pos2(rect.left(), a.bottom()),
                    egui::pos2(rect.right(), b.top()),
                )
            };
            ui.painter()
                .rect_filled(line, egui::CornerRadius::ZERO, c.border_subtle);
            // Dragging the divider changes the share.
            let grab = line.expand(3.0);
            let r = ui.interact(
                grab,
                egui::Id::unique("jarak-views-divider"),
                egui::Sense::drag(),
            );
            if r.hovered() || r.dragged() {
                ui.ctx().set_cursor_icon(if vertical {
                    egui::CursorIcon::ResizeHorizontal
                } else {
                    egui::CursorIcon::ResizeVertical
                });
            }
            if r.dragged()
                && let (Some(p), Some(s)) =
                    (ui.input(|i| i.pointer.hover_pos()), self.second.as_mut())
            {
                s.ratio = if vertical {
                    (p.x - rect.left()) / rect.width()
                } else {
                    (p.y - rect.top()) / rect.height()
                }
                .clamp(0.2, 0.8);
            }
        }
        for (pane, e) in events {
            self.tab_event(pane, e);
        }
        if let Some(p) = focus_to {
            self.focus_pane(p);
        }
        self.sync_scroll();
        drawn
    }

    /// Synchronized scrolling: the other view keeps its distance from the focused one.
    fn sync_scroll(&mut self) {
        let Some(s) = self.second.as_ref() else {
            return;
        };
        let (sv, sh) = (s.sync_v, s.sync_h);
        if sv.is_none() && sh.is_none() {
            return;
        }
        let other = if self.second_focused() {
            PaneId::Main
        } else {
            PaneId::Second
        };
        let i = self.active;
        let j = self.active_of(other);
        let (top, x) = (
            self.tabs[i].view.top_line as isize,
            self.tabs[i].view.scroll_x,
        );
        let set = |v: &mut view::ViewState| {
            if let Some(d) = sv {
                v.top_line = (top + d).max(0) as usize;
            }
            if let Some(d) = sh {
                v.scroll_x = (x + d).max(0.0);
            }
        };
        if i == j {
            if let Some(a) = self.tabs[i].alt.as_mut() {
                set(&mut a.view);
            }
        } else {
            set(&mut self.tabs[j].view);
        }
    }

    /// Turns synchronized vertical or horizontal scrolling on or off (`VIEWS-05`).
    pub fn toggle_sync(&mut self, vertical: bool) {
        let other = if self.second_focused() {
            PaneId::Main
        } else {
            PaneId::Second
        };
        if self.second.is_none() {
            return;
        }
        let i = self.active;
        let j = self.active_of(other);
        let (ov, ox) = if i == j {
            self.tabs[i]
                .alt
                .as_ref()
                .map_or((0, 0.0), |a| (a.view.top_line, a.view.scroll_x))
        } else {
            (self.tabs[j].view.top_line, self.tabs[j].view.scroll_x)
        };
        let (mv, mx) = (self.tabs[i].view.top_line, self.tabs[i].view.scroll_x);
        if let Some(s) = self.second.as_mut() {
            if vertical {
                s.sync_v = if s.sync_v.is_some() {
                    None
                } else {
                    Some(ov as isize - mv as isize)
                };
            } else {
                s.sync_h = if s.sync_h.is_some() {
                    None
                } else {
                    Some(ox - mx)
                };
            }
        }
    }

    /// Draws one document (source, preview or both) into `rect`.
    fn draw_doc(&mut self, ui: &mut egui::Ui, rect: Rect, i: usize, salt: u8, c: &Colors) -> Drawn {
        let lang = self.lang;
        let read_only = self.tabs[i].file.read_only;
        let t = &mut self.tabs[i];
        t.view.salt = salt;
        t.preview.salt = salt;
        let mut d = Drawn::default();
        match t.mode {
            Mode::Source => {
                let v = view::show(ui, rect, &mut t.file.doc, &mut t.view, read_only);
                d.ctrl_click = v.ctrl_click;
                d.focused = v.focused;
                d.clicked = v.clicked;
            }
            Mode::Preview => {
                let r = t.rendered();
                let doc = t.file.path.clone();
                d.link = preview::show(
                    ui,
                    rect,
                    &r,
                    &mut t.preview,
                    t.view.zoom,
                    doc.as_deref(),
                    lang,
                );
            }
            Mode::Split => {
                let mid = rect.center().x.round();
                let left = Rect::from_min_max(rect.min, egui::pos2(mid, rect.bottom()));
                let right = Rect::from_min_max(egui::pos2(mid + 1.0, rect.top()), rect.max);
                let v = view::show(ui, left, &mut t.file.doc, &mut t.view, read_only);
                d.ctrl_click = v.ctrl_click;
                d.focused = v.focused;
                d.clicked = v.clicked;
                // The preview follows the source's top line.
                let r = t.rendered();
                t.preview.scroll_to_line(&r, t.view.top_line);
                let doc = t.file.path.clone();
                d.link = preview::show(
                    ui,
                    right,
                    &r,
                    &mut t.preview,
                    t.view.zoom,
                    doc.as_deref(),
                    lang,
                );
                ui.painter().vline(
                    mid + 0.5,
                    rect.y_range(),
                    egui::Stroke::new(size::BORDER, c.border_subtle),
                );
            }
        }
        d
    }

    fn tab_event(&mut self, pane: PaneId, e: tabs::Event) {
        let order = self.order(pane);
        match e {
            tabs::Event::Activate(k) => {
                if let Some(&i) = order.get(k) {
                    self.activate(i, pane);
                }
            }
            tabs::Event::Close(k) => {
                if let Some(&i) = order.get(k) {
                    self.close_in_pane(i, pane);
                }
            }
            tabs::Event::Reorder(a, b) => self.reorder(pane, a, b),
            tabs::Event::Menu(k, m) => {
                if let Some(&i) = order.get(k) {
                    self.activate(i, pane);
                    self.tab_menu(pane, k, m);
                }
            }
        }
    }
}
