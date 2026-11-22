//! The text view (`JARAK-FR-030…034`): Jarak draws the document itself on egui's painter, one
//! laid-out galley per visible line (egui's `TextEdit` lays out the whole text every frame and
//! has no rope, multiple carets or columns, ADR 0102 §2).
//!
//! - Only the lines on screen are laid out (`top_line` plus as many as fit), so a million-line
//!   file costs the same per frame as a short one.
//! - Tabs are expanded to the next tab stop in the drawn text; a map turns document offsets into
//!   galley positions and back, so the caret, selections and clicks stay exact.
//! - The view scrolls by line (with a pixel offset into the top line), so word wrap and huge files
//!   need no total height; the scrollbar maps to the line index.
//! - The caret does not blink (no repaint while idle, `JARAK-NFR-003`); it is the accent colour.
//! - IME composition (Korean) is drawn inline at the caret, underlined, and inserted only when
//!   committed (`JARAK-FR-033`); the IME window is told where the caret is.

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Galley, Pos2, Rect, Sense, Stroke, Vec2, text::CCursor,
};
use jarak_core::document::Document;
use jarak_core::document::multi::Block;
use jarak_core::selection::{Range, Selection};
use jarak_core::text;
use kim_design::{font, size, space};
use kim_ui::Colors;
use std::sync::Arc;

/// The view's state for one document.
#[derive(Debug, Clone)]
pub struct ViewState {
    /// The first line on screen and how far it is scrolled up (points).
    pub top_line: usize,
    pub top_px: f32,
    /// Horizontal scroll (points; without word wrap).
    pub scroll_x: f32,
    /// Text zoom (1.0 = the design system's terminal size).
    pub zoom: f32,
    pub wrap: bool,
    /// Typed text replaces the character after the caret (Insert).
    pub overwrite: bool,
    /// The IME composition shown at the caret (not in the document yet).
    pub preedit: String,
    /// Keep the caret on screen after the next layout.
    pub follow: bool,
    /// The lines that fitted in the last frame (Page Up/Down).
    pub page_lines: usize,
    /// Step 92: Vim's Normal and Visual modes draw the caret as a block over the character.
    pub block_caret: bool,
    /// The IME may compose (off in Vim's Normal mode, so keys stay keys with Hangul on).
    pub ime: bool,
    /// Ranges to mark behind the text (Vim's search highlight), in document positions.
    pub marks: Vec<(usize, usize)>,
    /// The view keeps the focus on Esc (Vim editing: Esc is a key of its own).
    pub keep_escape: bool,
    /// Step 93: which view draws it (0 main, 1 second): its widget id.
    pub salt: u8,
    /// Step 94: styled ranges (marks, token styles) with their colours, and bookmarked lines.
    pub styled: Vec<(usize, usize, Color32)>,
    pub bookmarks: Vec<usize>,
    /// Step 96: where each visible line was drawn last frame (its first char and origin) and
    /// the cell size — for [`ViewState::pos_of_char`].
    drawn: Vec<(usize, Pos2)>,
    cell: Vec2,
    drag: Option<Drag>,
    clicks: Option<(f64, Pos2, u8)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// Selecting text from this anchor; `unit` 1 = characters, 2 = words, 3 = lines.
    Text { anchor: usize, unit: u8 },
    /// Step 96: Ctrl+drag, a further selection from this anchor (the others stay).
    Add { anchor: usize },
    /// Step 96: Alt+drag, a column selection from this (line, display column).
    Column { anchor: (usize, usize) },
    /// The scrollbar thumb, grabbed at this offset.
    Scrollbar { grab: f32 },
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            top_line: 0,
            top_px: 0.0,
            scroll_x: 0.0,
            zoom: 1.0,
            wrap: false,
            overwrite: false,
            preedit: String::new(),
            follow: true,
            page_lines: 20,
            block_caret: false,
            ime: true,
            marks: Vec::new(),
            keep_escape: false,
            salt: 0,
            styled: Vec::new(),
            bookmarks: Vec::new(),
            drawn: Vec::new(),
            cell: Vec2::ZERO,
            drag: None,
            clicks: None,
        }
    }
}

/// Font metrics of the view.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub font_size: f32,
    pub row: f32,
    pub char_w: f32,
}

impl Metrics {
    pub fn new(ui: &egui::Ui, zoom: f32) -> Metrics {
        let font_size = (font::TERMINAL * zoom).clamp(6.0, 72.0);
        let fid = FontId::monospace(font_size);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&fid, 'M')).max(1.0);
        Metrics {
            font_size,
            row: (font_size * font::LINE_HEIGHT_TERMINAL * 1.05).round(),
            char_w,
        }
    }

    fn font(&self) -> FontId {
        FontId::monospace(self.font_size)
    }
}

/// The layout of one drawn line: the monospace font, every wide character (Hangul, CJK, emoji)
/// exactly two cells wide (Step 96: the fallback font's glyphs are not, so column selections and
/// tab stops would drift), the rest in runs.
fn line_job(
    ui: &egui::Ui,
    text: &str,
    m: &Metrics,
    color: Color32,
    wrap_w: f32,
) -> egui::text::LayoutJob {
    let font = m.font();
    let fmt = egui::text::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping {
            max_width: wrap_w,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut run = String::new();
    for ch in text.chars() {
        if unicode_width_of(ch) == 2 {
            if !run.is_empty() {
                job.append(&run, 0.0, fmt.clone());
                run.clear();
            }
            let w = ui.fonts_mut(|f| f.glyph_width(&font, ch));
            let mut wide = fmt.clone();
            wide.extra_letter_spacing = 2.0 * m.char_w - w;
            job.append(ch.encode_utf8(&mut [0u8; 4]), 0.0, wide);
        } else {
            run.push(ch);
        }
    }
    if !run.is_empty() || job.sections.is_empty() {
        job.append(&run, 0.0, fmt);
    }
    job
}

/// One line as drawn: the text with tabs expanded (and the IME composition inserted), and the
/// map from document offsets in the line to char indices in that text.
#[derive(Debug, Clone)]
pub struct VisualLine {
    pub text: String,
    /// `map[o]` is the drawn char index of document offset `o` (0..=line length).
    pub map: Vec<usize>,
}

impl VisualLine {
    /// `line` without its line break; `preedit` is inserted at `caret` (an offset in the line).
    pub fn build(line: &str, tab_width: usize, preedit: Option<(usize, &str)>) -> VisualLine {
        let mut text = String::with_capacity(line.len());
        let mut map = Vec::with_capacity(line.len() + 1);
        let mut col = 0usize;
        let mut drawn = 0usize;
        let tab = tab_width.max(1);
        for (o, ch) in line.chars().enumerate() {
            if let Some((at, p)) = preedit
                && at == o
            {
                text.push_str(p);
                drawn += p.chars().count();
            }
            map.push(drawn);
            if ch == '\t' {
                let n = tab - col % tab;
                text.extend(std::iter::repeat_n(' ', n));
                drawn += n;
                col += n;
            } else {
                text.push(ch);
                drawn += 1;
                col += unicode_width_of(ch);
            }
        }
        let len = line.chars().count();
        if let Some((at, p)) = preedit
            && at >= len
        {
            text.push_str(p);
            drawn += p.chars().count();
        }
        map.push(drawn);
        VisualLine { text, map }
    }

    /// The document offset of drawn char index `c` (the nearest offset at or before it).
    pub fn offset_of(&self, c: usize) -> usize {
        match self.map.binary_search(&c) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
}

fn unicode_width_of(ch: char) -> usize {
    let mut b = [0u8; 4];
    text::grapheme_width(ch.encode_utf8(&mut b), 0, 1)
}

/// A line laid out this frame.
struct Laid {
    line: usize,
    start: usize,
    origin: Pos2,
    galley: Arc<Galley>,
    vis: VisualLine,
    height: f32,
}

/// What the view did this frame (for the window to react).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewOutput {
    /// The view has keyboard focus.
    pub focused: bool,
    /// The document position of a Ctrl+click (an image reference there is shown in Explorer).
    pub ctrl_click: Option<usize>,
    /// Step 93: the view was clicked or a drag started in it this frame (a view without the
    /// focus takes it then).
    pub clicked: bool,
}

/// The id of the text view (focus, IME).
pub fn view_id() -> egui::Id {
    view_id_for(0)
}

/// The text view of a view (0 main, 1 second).
pub fn view_id_for(salt: u8) -> egui::Id {
    egui::Id::unique(("jarak-text-view", salt))
}

/// Draws `doc` into `rect` and handles the mouse; keys are handled by the window, which calls
/// [`ViewState::after_edit`] so the caret stays visible.
pub fn show(
    ui: &mut egui::Ui,
    rect: Rect,
    doc: &mut Document,
    state: &mut ViewState,
    read_only: bool,
) -> ViewOutput {
    let c = Colors::get(ui.ctx());
    let m = Metrics::new(ui, state.zoom);
    let id = view_id_for(state.salt);
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, "Text"));
    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    if focused {
        ui.memory_mut(|mem| {
            mem.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: state.keep_escape,
                },
            );
        });
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, c.bg_base);
    let lines = doc.text().len_lines();
    let digits = lines.to_string().len().max(3) as f32;
    let gutter = (digits + 2.0) * m.char_w;
    let bar_w = size::PROGRESS * 2.0;
    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + gutter, rect.top()),
        egui::pos2(rect.right() - bar_w, rect.bottom()),
    );
    let pad = space::S;
    let wrap_w = if state.wrap {
        (text_rect.width() - pad * 2.0).max(m.char_w * 8.0)
    } else {
        f32::INFINITY
    };
    // Follow the caret before laying out, using whole rows (exact enough without wrap).
    let caret = doc.selection().primary().head;
    let caret_line = doc.text().char_to_line(caret);
    let visible_rows = ((rect.height() - pad) / m.row).floor().max(1.0) as usize;
    state.page_lines = visible_rows.saturating_sub(1).max(1);
    if state.follow {
        if caret_line < state.top_line {
            state.top_line = caret_line;
            state.top_px = 0.0;
        } else if !state.wrap && caret_line >= state.top_line + visible_rows {
            state.top_line = caret_line + 1 - visible_rows;
            state.top_px = 0.0;
        }
    }
    state.top_line = state.top_line.min(lines.saturating_sub(1));
    // Lay out the visible lines.
    let tab = doc.tab_width;
    let mut laid: Vec<Laid> = Vec::new();
    let mut y = rect.top() + pad - state.top_px;
    let mut line = state.top_line;
    let x0 = text_rect.left() + pad - if state.wrap { 0.0 } else { state.scroll_x };
    while line < lines && y < rect.bottom() {
        let l = doc.text().line(line);
        let len = text::line_len(l);
        let start = doc.text().line_to_char(line);
        let s: String = l.slice(..len).to_string();
        let pre = (!state.preedit.is_empty() && line == caret_line)
            .then(|| (caret - start, state.preedit.as_str()));
        let vis = VisualLine::build(&s, tab, pre);
        let job = line_job(ui, &vis.text, &m, c.text_primary, wrap_w);
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let height = (galley.rows.len().max(1) as f32) * m.row;
        laid.push(Laid {
            line,
            start,
            origin: egui::pos2(x0, y),
            galley,
            vis,
            height,
        });
        y += height;
        line += 1;
    }
    // A wrapped caret line below the window: scroll down until it fits.
    if state.follow && state.wrap {
        if let Some(l) = laid.iter().find(|l| l.line == caret_line)
            && l.origin.y + l.height > rect.bottom()
            && state.top_line < caret_line
        {
            state.top_line += 1;
            ui.ctx().request_repaint();
        } else if !laid.iter().any(|l| l.line == caret_line) && caret_line > state.top_line {
            state.top_line += 1;
            ui.ctx().request_repaint();
        }
    }
    // Horizontal follow (no wrap): keep the caret inside the text area.
    if state.follow && !state.wrap {
        if let Some(l) = laid.iter().find(|l| l.line == caret_line) {
            let off = caret - l.start;
            let cx = x_of(l, off) - x0;
            let visible = text_rect.width() - pad * 2.0;
            if cx < state.scroll_x {
                state.scroll_x = (cx - m.char_w * 4.0).max(0.0);
                ui.ctx().request_repaint();
            } else if cx > state.scroll_x + visible - m.char_w {
                state.scroll_x = cx - visible + m.char_w * 4.0;
                ui.ctx().request_repaint();
            }
        }
        state.follow = false;
    } else if state.follow {
        state.follow = false;
    }
    // Draw: current line, selections, text, gutter, caret.
    let sel = doc.selection().clone();
    for l in &laid {
        let row_rect = Rect::from_min_size(
            egui::pos2(rect.left(), l.origin.y),
            Vec2::new(rect.width(), l.height),
        );
        if l.line == caret_line && sel.len() == 1 && sel.primary().is_empty() {
            painter.rect_filled(row_rect, CornerRadius::ZERO, c.bg_raised);
        }
        for (a, b, col) in &state.styled {
            paint_selection(&painter, l, &Range::new(*a, *b), doc, &m, *col);
        }
        for (a, b) in &state.marks {
            paint_selection(&painter, l, &Range::new(*a, *b), doc, &m, c.accent_soft);
        }
        for r in sel.ranges() {
            paint_selection(&painter, l, r, doc, &m, c.bg_selected);
        }
        painter.galley(l.origin, l.galley.clone(), c.text_primary);
        if !state.preedit.is_empty() && l.line == caret_line {
            // `map` puts the caret after the composition; it starts that many chars before.
            let off = caret - l.start;
            let end = l.vis.map[off];
            let a = l.galley.pos_from_cursor(CCursor::new(
                end.saturating_sub(state.preedit.chars().count()),
            ));
            let b = l.galley.pos_from_cursor(CCursor::new(end));
            painter.hline(
                (l.origin.x + a.left())..=(l.origin.x + b.left()),
                l.origin.y + a.bottom() - 1.0,
                Stroke::new(size::BORDER, c.accent),
            );
        }
    }
    // The gutter over horizontally scrolled text.
    let gutter_rect = Rect::from_min_max(rect.min, egui::pos2(text_rect.left(), rect.bottom()));
    painter.rect_filled(gutter_rect, CornerRadius::ZERO, c.bg_base);
    for l in &laid {
        if state.bookmarks.binary_search(&l.line).is_ok() {
            // A bookmark: a dot at the gutter's left edge.
            painter.circle_filled(
                egui::pos2(rect.left() + m.char_w * 0.7, l.origin.y + m.row / 2.0),
                (m.row * 0.22).max(3.0),
                c.accent,
            );
        }
        let n = (l.line + 1).to_string();
        let col = if l.line == caret_line {
            c.text_secondary
        } else {
            c.text_muted
        };
        painter.text(
            egui::pos2(text_rect.left() - m.char_w, l.origin.y),
            egui::Align2::RIGHT_TOP,
            n,
            m.font(),
            col,
        );
    }
    // Carets (every range's head; the primary is the one the IME follows).
    let mut ime_rect = None;
    for (i, r) in sel.ranges().iter().enumerate() {
        if let Some(l) = laid.iter().find(|l| {
            let end = l.start + l.vis.map.len() - 1;
            r.head >= l.start && r.head <= end
        }) {
            let off = r.head - l.start;
            let p = l.galley.pos_from_cursor(CCursor::new(l.vis.map[off]));
            let mut x = l.origin.x + p.left();
            if let Some(b) = doc.block().filter(|_| doc.virtual_space && r.is_empty()) {
                let (line, col) = doc.line_col_display(r.head);
                let left = b.cols().0;
                if col < left && doc.line_width(line) == col {
                    x += (left - col) as f32 * m.char_w;
                }
            }
            let w = if state.block_caret {
                // The width of the character under the caret (two cells for a wide one).
                let next = l
                    .vis
                    .map
                    .get(off + 1)
                    .map(|n| l.galley.pos_from_cursor(CCursor::new(*n)));
                next.filter(|q| (q.top() - p.top()).abs() < 0.5 && q.left() > p.left())
                    .map_or(m.char_w, |q| q.left() - p.left())
            } else if state.overwrite {
                m.char_w
            } else {
                size::FOCUS_RING
            };
            let caret_rect =
                Rect::from_min_size(egui::pos2(x, l.origin.y + p.top()), Vec2::new(w, m.row));
            if text_rect.intersects(caret_rect) {
                let col = if focused { c.accent } else { c.text_muted };
                if state.block_caret {
                    painter.rect_filled(caret_rect, CornerRadius::ZERO, col.gamma_multiply(0.55));
                } else if state.overwrite {
                    painter.rect_stroke(
                        caret_rect,
                        CornerRadius::ZERO,
                        Stroke::new(size::BORDER, col),
                        egui::StrokeKind::Inside,
                    );
                } else {
                    painter.rect_filled(caret_rect, CornerRadius::ZERO, col);
                }
            }
            if i == sel.primary_index() {
                ime_rect = Some(caret_rect);
            }
        }
    }
    if read_only {
        // A quiet marker in the corner says the document cannot be edited.
        painter.rect_stroke(
            rect.shrink(1.0),
            CornerRadius::ZERO,
            Stroke::new(size::BORDER, c.warning.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
    }
    if focused
        && state.ime
        && let Some(cr) = ime_rect
    {
        let to_global = ui
            .ctx()
            .layer_transform_to_global(ui.layer_id())
            .unwrap_or_default();
        ui.output_mut(|o| {
            o.ime = Some(egui::output::IMEOutput {
                purpose: egui::IMEPurpose::Normal,
                rect: to_global * text_rect,
                cursor_rect: to_global * cr,
                should_interrupt_composition: false,
            });
        });
    }
    // The scrollbar.
    paint_scrollbar(ui, rect, bar_w, lines, state, &c);
    // Mouse: clicks, drags, double and triple clicks, the wheel.
    state.drawn = laid.iter().map(|l| (l.start, l.origin)).collect();
    state.cell = Vec2::new(m.char_w, m.row);
    let ctrl_click = mouse(ui, &resp, &laid, doc, state, &m, text_rect);
    ViewOutput {
        focused,
        ctrl_click,
        clicked: resp.clicked() || resp.drag_started(),
    }
}

fn x_of(l: &Laid, off: usize) -> f32 {
    let off = off.min(l.vis.map.len() - 1);
    l.origin.x
        + l.galley
            .pos_from_cursor(CCursor::new(l.vis.map[off]))
            .left()
}

fn paint_selection(
    painter: &egui::Painter,
    l: &Laid,
    r: &Range,
    doc: &Document,
    m: &Metrics,
    col: Color32,
) {
    if r.is_empty() {
        return;
    }
    let len = l.vis.map.len() - 1;
    let line_end = l.start + len;
    // Nothing of this line, nor its line break, is selected.
    if r.to() <= l.start || r.from() > line_end {
        return;
    }
    let _ = doc;
    let a = r.from().max(l.start) - l.start;
    let b = r.to().min(line_end) - l.start;
    let ca = l.vis.map[a.min(len)];
    let cb = l.vis.map[b.min(len)];
    let pa = l.galley.pos_from_cursor(CCursor::new(ca));
    let pb = l.galley.pos_from_cursor(CCursor::new(cb));
    let crosses_eol = r.to() > line_end && r.from() <= line_end;
    if (pa.top() - pb.top()).abs() < 0.5 {
        let right = pb.left() + if crosses_eol { m.char_w * 0.6 } else { 0.0 };
        painter.rect_filled(
            Rect::from_min_max(
                l.origin + Vec2::new(pa.left(), pa.top()),
                l.origin + Vec2::new(right, pa.top() + m.row),
            ),
            CornerRadius::ZERO,
            col,
        );
    } else {
        // Across wrapped rows: the first row to its end, full rows, the last row to `b`.
        for row in &l.galley.rows {
            let top = row.rect().top();
            if top + 0.5 < pa.top() || top - 0.5 > pb.top() {
                continue;
            }
            let left = if (top - pa.top()).abs() < 0.5 {
                pa.left()
            } else {
                0.0
            };
            let right = if (top - pb.top()).abs() < 0.5 {
                pb.left() + if crosses_eol { m.char_w * 0.6 } else { 0.0 }
            } else {
                row.rect().right()
            };
            painter.rect_filled(
                Rect::from_min_max(
                    l.origin + Vec2::new(left, top),
                    l.origin + Vec2::new(right, top + m.row),
                ),
                CornerRadius::ZERO,
                col,
            );
        }
    }
}

fn paint_scrollbar(
    ui: &mut egui::Ui,
    rect: Rect,
    bar_w: f32,
    lines: usize,
    state: &mut ViewState,
    c: &Colors,
) {
    if lines <= state.page_lines {
        return;
    }
    let track = Rect::from_min_max(
        egui::pos2(rect.right() - bar_w, rect.top()),
        rect.right_bottom(),
    );
    let total = lines as f32;
    let frac = state.page_lines as f32 / total;
    let h = (track.height() * frac).max(size::HIT_MIN);
    let max_top = lines.saturating_sub(state.page_lines).max(1) as f32;
    let t = (state.top_line as f32 / max_top).clamp(0.0, 1.0);
    let thumb = Rect::from_min_size(
        egui::pos2(track.left() + 2.0, track.top() + (track.height() - h) * t),
        Vec2::new(bar_w - 4.0, h),
    );
    let resp = ui.interact(
        track,
        egui::Id::unique(("jarak-scrollbar", state.salt)),
        Sense::click_and_drag(),
    );
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.drag_started() {
            let grab = if thumb.contains(p) {
                p.y - thumb.top()
            } else {
                h / 2.0
            };
            state.drag = Some(Drag::Scrollbar { grab });
        }
        if let Some(Drag::Scrollbar { grab }) = state.drag
            && resp.dragged()
        {
            let t = ((p.y - grab - track.top()) / (track.height() - h).max(1.0)).clamp(0.0, 1.0);
            state.top_line = (t * max_top).round() as usize;
            state.top_px = 0.0;
        } else if resp.clicked() {
            let t = ((p.y - h / 2.0 - track.top()) / (track.height() - h).max(1.0)).clamp(0.0, 1.0);
            state.top_line = (t * max_top).round() as usize;
            state.top_px = 0.0;
        }
    }
    if resp.drag_stopped() {
        state.drag = None;
    }
    let hot = resp.hovered() || matches!(state.drag, Some(Drag::Scrollbar { .. }));
    ui.painter().rect_filled(
        thumb,
        CornerRadius::same(2),
        if hot {
            c.border_strong
        } else {
            c.border_subtle
        },
    );
}

/// The document position under `p` (snapped to a grapheme start).
fn hit(laid: &[Laid], doc: &Document, p: Pos2) -> usize {
    let Some(first) = laid.first() else {
        return 0;
    };
    let l = laid
        .iter()
        .find(|l| p.y < l.origin.y + l.height)
        .unwrap_or_else(|| laid.last().unwrap_or(first));
    if p.y < first.origin.y {
        return first.start;
    }
    let cur = l.galley.cursor_from_pos(p - l.origin);
    let off = l.vis.offset_of(cur.index.0).min(l.vis.map.len() - 1);
    text::floor_grapheme(doc.text().slice(..), l.start + off)
}

/// The (line, display column) under the pointer; past a line's end the column goes on in
/// character cells (a column selection may reach past short lines).
fn column_at(laid: &[Laid], doc: &Document, m: &Metrics, p: Pos2) -> (usize, usize) {
    let pos = hit(laid, doc, p);
    let (line, col) = doc.line_col_display(pos);
    let Some(l) = laid
        .iter()
        .find(|l| p.y < l.origin.y + l.height)
        .or(laid.last())
    else {
        return (line, col);
    };
    let end = l.start + l.vis.map.len() - 1;
    if pos == end && l.line == line {
        let e = l
            .galley
            .pos_from_cursor(CCursor::new(l.vis.map[end - l.start]));
        let extra = ((p.x - (l.origin.x + e.left())) / m.char_w).round();
        if extra > 0.0 {
            return (line, col + extra as usize);
        }
    }
    (line, col)
}

fn mouse(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    laid: &[Laid],
    doc: &mut Document,
    state: &mut ViewState,
    m: &Metrics,
    text_rect: Rect,
) -> Option<usize> {
    let (shift, ctrl, alt, scroll, time) = ui.input(|i| {
        (
            i.modifiers.shift,
            i.modifiers.command,
            i.modifiers.alt,
            i.smooth_scroll_delta,
            i.time,
        )
    });
    // The wheel: lines; Shift+wheel or a horizontal wheel: sideways; Ctrl+wheel is zoom
    // (handled by the window).
    if resp.hovered() && !ctrl && scroll != Vec2::ZERO {
        let (dx, dy) = if shift && scroll.x == 0.0 {
            (scroll.y, 0.0)
        } else {
            (scroll.x, scroll.y)
        };
        if !state.wrap && dx != 0.0 {
            state.scroll_x = (state.scroll_x - dx).max(0.0);
        }
        if dy != 0.0 {
            state.top_px -= dy;
            let lines = doc.text().len_lines();
            while state.top_px >= m.row && state.top_line + 1 < lines {
                state.top_px -= m.row;
                state.top_line += 1;
            }
            while state.top_px < 0.0 && state.top_line > 0 {
                state.top_px += m.row;
                state.top_line -= 1;
            }
            if state.top_line == 0 && state.top_px < 0.0 {
                state.top_px = 0.0;
            }
            if state.top_line + 1 >= lines {
                state.top_px = 0.0;
            }
        }
    }
    let p = resp.interact_pointer_pos()?;
    let pos = hit(laid, doc, p);
    let pressed = ui.input(|i| i.pointer.primary_pressed());
    if pressed {
        // Count clicks: within 0.4 s and a few points of the last one.
        let n = match state.clicks {
            Some((t, q, n)) if time - t < 0.4 && q.distance(p) < 6.0 => n % 3 + 1,
            _ => 1,
        };
        state.clicks = Some((time, p, n));
        match n {
            // Step 96: Alt+click / Alt+drag starts a column selection; Ctrl+click adds a caret
            // (Ctrl+drag a selection) and keeps the others.
            1 if alt && !ctrl => {
                let a = column_at(laid, doc, m, p);
                doc.select_block(Block { anchor: a, head: a });
                state.drag = Some(Drag::Column { anchor: a });
                state.preedit.clear();
                return None;
            }
            1 if ctrl && !shift => {
                doc.add_caret(pos);
                state.drag = Some(Drag::Add { anchor: pos });
                state.preedit.clear();
                return Some(pos);
            }
            2 => {
                doc.select_word_at(pos);
                state.drag = Some(Drag::Text {
                    anchor: doc.selection().primary().from(),
                    unit: 2,
                });
            }
            3 => {
                doc.select_line_at(pos);
                state.drag = Some(Drag::Text {
                    anchor: doc.selection().primary().from(),
                    unit: 3,
                });
            }
            _ => {
                let anchor = if shift {
                    doc.selection().primary().anchor
                } else {
                    pos
                };
                doc.set_selection(&Selection::single(Range::new(anchor, pos)));
                state.drag = Some(Drag::Text { anchor, unit: 1 });
            }
        }
        state.preedit.clear();
        return (ctrl && !shift && n == 1).then_some(pos);
    }
    match state.drag {
        Some(Drag::Add { anchor }) if resp.dragged() => {
            doc.set_primary_range(Range::new(anchor, pos));
            ui.ctx().request_repaint();
        }
        Some(Drag::Column { anchor }) if resp.dragged() => {
            doc.select_block(Block {
                anchor,
                head: column_at(laid, doc, m, p),
            });
            ui.ctx().request_repaint();
        }
        _ => {}
    }
    if let Some(Drag::Text { anchor, unit }) = state.drag
        && resp.dragged()
    {
        let r = match unit {
            2 => {
                let (a, b) = text::word_at(doc.text().slice(..), pos);
                if pos < anchor {
                    Range::new(text::word_at(doc.text().slice(..), anchor).1, a)
                } else {
                    Range::new(anchor, b)
                }
            }
            3 => {
                let line = doc.text().char_to_line(pos);
                let a = doc.text().line_to_char(line);
                let b = if line + 1 < doc.text().len_lines() {
                    doc.text().line_to_char(line + 1)
                } else {
                    doc.len_chars()
                };
                if pos < anchor {
                    let al = doc.text().char_to_line(anchor);
                    let end = if al + 1 < doc.text().len_lines() {
                        doc.text().line_to_char(al + 1)
                    } else {
                        doc.len_chars()
                    };
                    Range::new(end, a)
                } else {
                    Range::new(anchor, b)
                }
            }
            _ => Range::new(anchor, pos),
        };
        doc.set_selection(&Selection::single(r));
        // Dragging past the top or bottom scrolls.
        if p.y < text_rect.top() && state.top_line > 0 {
            state.top_line -= 1;
        } else if p.y > text_rect.bottom() {
            state.top_line += 1;
        }
        ui.ctx().request_repaint();
    }
    if resp.drag_stopped() {
        state.drag = None;
    }
    None
}

impl ViewState {
    /// Where the view drew char position `pos` last frame (the left edge of its cell, half a row
    /// down), when its line was on screen — tests and tools point the mouse with it. Unwrapped
    /// lines; a wide character spans two cells.
    pub fn pos_of_char(&self, doc: &Document, pos: usize) -> Option<Pos2> {
        let (line, col) = doc.line_col_display(pos);
        let start = doc.text().line_to_char(line);
        let (_, origin) = self.drawn.iter().find(|(s, _)| *s == start)?;
        Some(egui::pos2(
            origin.x + col as f32 * self.cell.x + 0.1,
            origin.y + self.cell.y / 2.0,
        ))
    }

    /// After an edit or a caret move: show the caret, drop the composition.
    pub fn after_edit(&mut self) {
        self.follow = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_expand_to_stops_and_map_both_ways() {
        let v = VisualLine::build("a\tb\t\tc", 4, None);
        assert_eq!(v.text, "a   b       c");
        assert_eq!(v.map, vec![0, 1, 4, 5, 8, 12, 13]);
        assert_eq!(v.offset_of(2), 1, "inside the tab: the tab's offset");
        assert_eq!(v.offset_of(4), 2);
        assert_eq!(v.offset_of(13), 6);
    }

    #[test]
    fn wide_characters_count_two_columns_for_tab_stops() {
        let v = VisualLine::build("한\tx", 4, None);
        assert_eq!(v.text, "한  x", "한 is two columns, the tab fills two");
        assert_eq!(v.map, vec![0, 1, 3, 4]);
    }

    #[test]
    fn the_composition_is_drawn_at_the_caret_without_entering_the_document() {
        let v = VisualLine::build("ab", 4, Some((1, "한")));
        assert_eq!(v.text, "a한b");
        assert_eq!(
            v.map,
            vec![0, 2, 3],
            "offsets after the caret move by the composition"
        );
        let v = VisualLine::build("ab", 4, Some((2, "ㄱ")));
        assert_eq!(v.text, "abㄱ");
        assert_eq!(v.map, vec![0, 1, 3]);
    }
}
