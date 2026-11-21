//! The Markdown preview (`JARAK-FR-040`, `JARAK-FR-041`, `JARAK-FR-046`, `JARAK-SEC-001`): the
//! render model of `jarak-core::markdown` drawn natively on egui's painter.
//!
//! - Like the text view it is virtualized: it scrolls by block (the index of the top block plus
//!   a pixel offset into it) and lays out only the blocks on screen, so a long document costs the
//!   same per frame as a short one and switching never waits for the whole document's layout.
//! - The parse is cached per history state: switching back and forth parses once.
//! - Switching keeps the reading position: the source's top line maps to the block that shows
//!   it, and back.
//! - Links open only with Ctrl+click (never a plain click or hover), and only web and mail
//!   links.
//! - Images (Steps 90–91, `JARAK-FR-044`): a picture is drawn from its local file (relative to
//!   the document, never outside its folder, never from the network) at one image pixel per
//!   screen pixel times the document's zoom, at most the column's width, aspect kept. Decoding
//!   runs on a worker thread into a bounded cache ([`crate::media`]); until it is ready a frame of
//!   the picture's size says "loading". A missing, broken, remote or refused image shows a
//!   labelled frame with its path. A paragraph with text and images shows the text runs and the
//!   pictures in order; an image inside a link or emphasis is a labelled placeholder.
//! - Hovering a local picture shows its file and size; Ctrl+click shows it in Explorer
//!   (`JARAK-FR-045`); a plain click only says so.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Sense, Stroke, Vec2,
    text::{CCursor, LayoutJob, TextFormat},
};
use jarak_core::markdown::{self, Align, Block, Inline, Rendered};
use kim_design::{font, radius, size, space};
use kim_ui::Colors;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::media::{Look as MediaLook, Media};

/// Opens a link the user Ctrl+clicked (the program asks Windows; tests record it).
pub trait Opener: Send + Sync {
    fn open_url(&self, url: &str) -> bool;
    /// Shows a local file selected in Explorer (`JARAK-FR-045`); never runs it.
    fn reveal(&self, _path: &Path) -> bool {
        false
    }
}

/// Opens nothing (tests, and before the program installs the real one).
#[derive(Debug, Default)]
pub struct NoOpener;

impl Opener for NoOpener {
    fn open_url(&self, _url: &str) -> bool {
        false
    }
}

/// The links Jarak opens: web pages and mail (`JARAK-SEC-006`); everything else is refused.
pub fn openable(url: &str) -> bool {
    let l = url.trim().to_ascii_lowercase();
    l.starts_with("https://") || l.starts_with("http://") || l.starts_with("mailto:")
}

/// The real opener: `ShellExecuteW("open", url)` for an openable link.
#[derive(Debug, Default)]
pub struct ShellOpener;

impl Opener for ShellOpener {
    #[allow(unsafe_code)]
    fn open_url(&self, url: &str) -> bool {
        if !openable(url) {
            return false;
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            let w = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
            let (verb, target) = (w("open"), w(url.trim()));
            // SAFETY: NUL-terminated strings that live across the call; no parent window.
            let r = unsafe {
                ShellExecuteW(
                    std::ptr::null_mut(),
                    verb.as_ptr(),
                    target.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    SW_SHOWNORMAL,
                )
            };
            r as usize > 32
        }
    }

    /// `explorer.exe /select,"<file>"`: Explorer opens the folder with the file selected.
    #[allow(unsafe_code)]
    fn reveal(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            let w = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
            let (verb, exe) = (w("open"), w("explorer.exe"));
            let params = w(&format!("/select,\"{}\"", path.display()));
            // SAFETY: NUL-terminated strings that live across the call; no parent window.
            let r = unsafe {
                ShellExecuteW(
                    std::ptr::null_mut(),
                    verb.as_ptr(),
                    exe.as_ptr(),
                    params.as_ptr(),
                    std::ptr::null(),
                    SW_SHOWNORMAL,
                )
            };
            r as usize > 32
        }
    }
}

/// The preview's state for one document.
#[derive(Debug, Clone, Default)]
pub struct PreviewState {
    /// The parse and the history state it belongs to.
    cache: Option<(u64, Arc<Rendered>)>,
    /// The top block on screen and how far it is scrolled up (points).
    pub top_block: usize,
    pub top_px: f32,
    /// Links on screen this frame (for Ctrl+click and the pointer).
    links: Vec<(Rect, String)>,
    /// Local pictures on screen this frame: where, the file and its hover text.
    hot: Vec<(Rect, PathBuf, String)>,
    /// The document's pictures (Step 91: decoded off the UI thread, bounded).
    pub media: Arc<Mutex<Media>>,
}

impl PreviewState {
    /// The parse of `text` at history state `state` (parsed again only after an edit).
    pub fn rendered(&mut self, state: u64, text: impl FnOnce() -> String) -> Arc<Rendered> {
        match &self.cache {
            Some((s, r)) if *s == state => r.clone(),
            _ => {
                let r = Arc::new(markdown::parse(&text()));
                self.cache = Some((state, r.clone()));
                r
            }
        }
    }

    /// Shows the block that holds source line `line` at the top.
    pub fn scroll_to_line(&mut self, r: &Rendered, line: usize) {
        self.top_block = r.block_for_line(line).unwrap_or(0);
        self.top_px = 0.0;
    }

    /// The local pictures on screen in the last frame (area and file).
    pub fn pictures(&self) -> Vec<(Rect, PathBuf)> {
        self.hot.iter().map(|(r, p, _)| (*r, p.clone())).collect()
    }

    /// The source line of the top block (for switching back to the source).
    pub fn top_line(&self, r: &Rendered) -> Option<usize> {
        r.blocks.get(self.top_block).map(|b| b.lines.start)
    }
}

/// Font sizes and colours of one frame, and where images come from.
struct Style {
    body: f32,
    zoom: f32,
    c: Colors,
    width: f32,
    ctx: egui::Context,
    doc: Option<PathBuf>,
    media: Arc<Mutex<Media>>,
    lang: kim_design::Lang,
}

/// What an image reference shows.
enum Img {
    Ready(egui::TextureId, [u32; 2]),
    Loading(Option<[u32; 2]>),
    /// Why it is not shown (missing, refused, remote).
    Not(String),
}

impl Style {
    /// The image `url` of the document and its local file (when it has one).
    fn image(&self, url: &str) -> (Img, Option<PathBuf>) {
        let lower = url.trim().to_ascii_lowercase();
        let t = |en: &str, ko: &str| self.lang.pick(en, ko).to_string();
        if lower.contains("://") || lower.starts_with("data:") {
            let why = t("remote image, never loaded:", "원격 이미지, 불러오지 않음:");
            return (Img::Not(format!("{why} {url}")), None);
        }
        let Some(doc) = &self.doc else {
            let why = t("save the document to show", "문서를 저장하면 표시합니다:");
            return (Img::Not(format!("{why} {url}")), None);
        };
        let Some(path) = jarak_core::assets::resolve(doc, url) else {
            return (
                Img::Not(format!(
                    "{} {url}",
                    t(
                        "outside the document's folder, not shown:",
                        "문서 폴더 밖이라 표시하지 않음:"
                    )
                )),
                None,
            );
        };
        let Ok(mut m) = self.media.lock() else {
            return (Img::Not(url.to_string()), None);
        };
        let img = match m.get(&self.ctx, &path) {
            MediaLook::Ready(id, size) => Img::Ready(id, size),
            MediaLook::Loading(size) => Img::Loading(size),
            MediaLook::Failed(why) if why == crate::media::NOT_FOUND => Img::Not(format!(
                "{} {url}",
                t("image not found:", "이미지를 찾을 수 없음:")
            )),
            MediaLook::Failed(why) => Img::Not(format!(
                "{} {url} ({why})",
                t("cannot show", "표시할 수 없음:")
            )),
        };
        (img, Some(path))
    }
}

/// A paragraph cut at its images: text runs and pictures in order.
enum Seg<'a> {
    Text(Vec<Inline>),
    Image(&'a str, &'a str),
}

/// The paragraph's segments when it holds a top-level image (`None` for text only). Breaks and
/// blank text next to an image are dropped, so `text ![](a.png)` is the text, then the picture.
fn segments(v: &[Inline]) -> Option<Vec<Seg<'_>>> {
    if !v.iter().any(|i| matches!(i, Inline::Image { .. })) {
        return None;
    }
    let mut out = Vec::new();
    let mut run: Vec<Inline> = Vec::new();
    let flush = |run: &mut Vec<Inline>, out: &mut Vec<Seg<'_>>| {
        while matches!(run.last(), Some(Inline::SoftBreak | Inline::HardBreak))
            || matches!(run.last(), Some(Inline::Text(t)) if t.trim().is_empty())
        {
            run.pop();
        }
        if !run.is_empty() {
            out.push(Seg::Text(std::mem::take(run)));
        }
    };
    for i in v {
        match i {
            Inline::Image { url, alt } => {
                flush(&mut run, &mut out);
                out.push(Seg::Image(url.as_str(), alt.as_str()));
            }
            Inline::SoftBreak | Inline::HardBreak if run.is_empty() => {}
            Inline::Text(t) if run.is_empty() && t.trim().is_empty() => {}
            other => run.push(other.clone()),
        }
    }
    flush(&mut run, &mut out);
    Some(out)
}

impl Style {
    fn prop(&self, size: f32) -> FontId {
        FontId::proportional(size)
    }
    fn bold(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(kim_ui::SEMIBOLD.into()))
    }
    fn mono(&self, size: f32) -> FontId {
        FontId::monospace(size)
    }
}

/// Something to paint, relative to the block's top-left corner.
enum Piece {
    Text(Pos2, Arc<Galley>, Vec<(std::ops::Range<usize>, String)>),
    Fill(Rect, Color32, f32),
    Line(Pos2, Pos2, Color32),
    Box(Rect, Color32),
    Check(Rect, bool, Color32, Color32),
    Image(Rect, egui::TextureId),
    /// A local picture's area: its file and hover text (Ctrl+click shows it in Explorer).
    Hot(Rect, PathBuf, String),
}

/// A laid-out block: what to paint and its height.
struct Laid {
    pieces: Vec<Piece>,
    height: f32,
}

/// How a run of inline text looks.
#[derive(Clone, Copy)]
struct Fmt {
    bold: bool,
    italic: bool,
    strike: bool,
    link: bool,
    code: bool,
}

/// The text style of one inline layout: size and base colour.
#[derive(Clone, Copy)]
struct Look {
    size: f32,
    bold: bool,
    color: Color32,
}

/// A layout job being built, with the char ranges of its links.
struct JobOut {
    job: LayoutJob,
    chars: usize,
    links: Vec<(std::ops::Range<usize>, String)>,
}

impl JobOut {
    fn push(&mut self, s: &str, f: Fmt, st: &Style, look: Look) {
        if s.is_empty() {
            return;
        }
        let font_id = if f.code {
            st.mono(look.size * 0.92)
        } else if f.bold {
            st.bold(look.size)
        } else {
            st.prop(look.size)
        };
        let col = if f.link { st.c.accent } else { look.color };
        let fmt = TextFormat {
            font_id,
            color: col,
            italics: f.italic,
            strikethrough: if f.strike {
                Stroke::new(1.0, col)
            } else {
                Stroke::NONE
            },
            underline: if f.link {
                Stroke::new(1.0, st.c.accent.gamma_multiply(0.6))
            } else {
                Stroke::NONE
            },
            background: if f.code {
                st.c.bg_raised
            } else {
                Color32::TRANSPARENT
            },
            ..Default::default()
        };
        self.job.append(s, 0.0, fmt);
        self.chars += s.chars().count();
    }

    fn walk(&mut self, v: &[Inline], f: Fmt, st: &Style, look: Look) {
        for i in v {
            match i {
                Inline::Text(t) => self.push(t, f, st, look),
                Inline::Code(t) | Inline::Html(t) => {
                    self.push(t, Fmt { code: true, ..f }, st, look);
                }
                Inline::Emph(c) => self.walk(c, Fmt { italic: true, ..f }, st, look),
                Inline::Strong(c) => self.walk(c, Fmt { bold: true, ..f }, st, look),
                Inline::Strike(c) => self.walk(c, Fmt { strike: true, ..f }, st, look),
                Inline::Link { url, children } => {
                    let start = self.chars;
                    self.walk(children, Fmt { link: true, ..f }, st, look);
                    self.links.push((start..self.chars, url.clone()));
                }
                Inline::Image { url, alt } => {
                    let label = if alt.is_empty() {
                        url.as_str()
                    } else {
                        alt.as_str()
                    };
                    self.push(
                        &format!("[image: {label}]"),
                        Fmt { code: true, ..f },
                        st,
                        look,
                    );
                }
                Inline::SoftBreak => self.push(" ", f, st, look),
                Inline::HardBreak => self.push("\n", f, st, look),
                Inline::FootnoteRef(l) => {
                    self.push(&format!("[{l}]"), Fmt { link: true, ..f }, st, look);
                }
            }
        }
    }
}

/// Inline content as a layout job with the char ranges of its links.
fn job(
    v: &[Inline],
    st: &Style,
    look: Look,
    width: f32,
) -> (LayoutJob, Vec<(std::ops::Range<usize>, String)>) {
    let mut out = JobOut {
        job: LayoutJob {
            wrap: egui::text::TextWrapping {
                max_width: width,
                ..Default::default()
            },
            ..Default::default()
        },
        chars: 0,
        links: Vec::new(),
    };
    out.walk(
        v,
        Fmt {
            bold: look.bold,
            italic: false,
            strike: false,
            link: false,
            code: false,
        },
        st,
        look,
    );
    (out.job, out.links)
}

fn text_piece(
    ui: &egui::Ui,
    v: &[Inline],
    st: &Style,
    look: Look,
    at: Pos2,
    width: f32,
) -> (Piece, f32) {
    let (j, links) = job(v, st, look, width);
    let g = ui.fonts_mut(|f| f.layout_job(j));
    let h = g.rect.height().max(look.size);
    (Piece::Text(at, g, links), h)
}

/// Lays out one block at `x` (indent) with `width`.
fn lay(ui: &egui::Ui, b: &Block, st: &Style, x: f32, width: f32, out: &mut Vec<Piece>) -> f32 {
    let gap = space::M * st.zoom;
    let c = &st.c;
    match b {
        Block::Heading { level, inlines } => {
            let scale = match level {
                1 => 1.8,
                2 => 1.5,
                3 => 1.25,
                4 => 1.1,
                _ => 1.0,
            };
            let (p, h) = text_piece(
                ui,
                inlines,
                st,
                Look {
                    size: st.body * scale,
                    bold: true,
                    color: c.text_primary,
                },
                Pos2::new(x, 0.0),
                width,
            );
            out.push(p);
            let mut h = h;
            if *level <= 2 {
                h += space::XS * st.zoom;
                out.push(Piece::Line(
                    Pos2::new(x, h),
                    Pos2::new(x + width, h),
                    c.border_subtle,
                ));
            }
            h + gap
        }
        Block::Paragraph(v) if segments(v).is_some() => {
            let mut y = 0.0;
            let ppp = st.ctx.pixels_per_point().max(0.5);
            let between = space::S * st.zoom;
            for seg in segments(v).unwrap_or_default() {
                match seg {
                    Seg::Text(t) => {
                        let (p, h) = text_piece(
                            ui,
                            &t,
                            st,
                            Look {
                                size: st.body,
                                bold: false,
                                color: c.text_primary,
                            },
                            Pos2::new(x, y),
                            width,
                        );
                        out.push(p);
                        y += h + between;
                    }
                    Seg::Image(url, alt) => {
                        let (img, path) = st.image(url);
                        // One image pixel per screen pixel times the zoom, at most the column.
                        let fit = |[w, h]: [u32; 2]| {
                            let natural = Vec2::new(w as f32, h as f32) / ppp * st.zoom;
                            natural * (width / natural.x.max(1.0)).min(1.0)
                        };
                        let (rect, label, tip) = match img {
                            Img::Ready(id, px) => {
                                let r = Rect::from_min_size(Pos2::new(x, y), fit(px));
                                out.push(Piece::Image(r, id));
                                (r, None, format!("{} × {} px", px[0], px[1]))
                            }
                            Img::Loading(px) => {
                                let size = px.map_or(Vec2::new(width, st.body * 6.0), fit);
                                let r = Rect::from_min_size(Pos2::new(x, y), size);
                                out.push(Piece::Fill(r, c.bg_raised, 0.0));
                                let tip =
                                    px.map_or(String::new(), |p| format!("{} × {} px", p[0], p[1]));
                                (
                                    r,
                                    Some((
                                        st.lang
                                            .pick("loading image…", "이미지 불러오는 중…")
                                            .to_string(),
                                        c.border_subtle,
                                    )),
                                    tip,
                                )
                            }
                            Img::Not(why) => {
                                let label = if alt.is_empty() {
                                    why
                                } else {
                                    format!("{alt} — {why}")
                                };
                                let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, 0.0));
                                (r, Some((label, c.border_strong)), String::new())
                            }
                        };
                        let mut rect = rect;
                        if let Some((label, edge)) = label {
                            let j = LayoutJob::simple(
                                label,
                                st.prop(st.body * 0.9),
                                c.text_muted,
                                (rect.width() - space::L * st.zoom).max(40.0),
                            );
                            let g = ui.fonts_mut(|f| f.layout_job(j));
                            let need = g.rect.height() + space::L * st.zoom;
                            if rect.height() < need {
                                rect.set_height(need);
                            }
                            out.push(Piece::Box(rect, edge));
                            if g.rect.width() + space::S * st.zoom <= rect.width() {
                                out.push(Piece::Text(
                                    Pos2::new(x + space::S * st.zoom, y + space::S * st.zoom),
                                    g,
                                    Vec::new(),
                                ));
                            }
                        }
                        if let Some(p) = path {
                            let tip = if tip.is_empty() {
                                p.display().to_string()
                            } else {
                                format!("{}\n{tip}", p.display())
                            };
                            out.push(Piece::Hot(rect, p, tip));
                        }
                        y += rect.height() + between;
                    }
                }
            }
            y + gap - between
        }
        Block::Paragraph(v) => {
            let (p, h) = text_piece(
                ui,
                v,
                st,
                Look {
                    size: st.body,
                    bold: false,
                    color: c.text_primary,
                },
                Pos2::new(x, 0.0),
                width,
            );
            out.push(p);
            h + gap
        }
        Block::Code { text, .. } => {
            let pad = space::S * st.zoom;
            let j = LayoutJob::simple(
                text.trim_end_matches('\n').to_string(),
                st.mono(st.body * 0.92),
                c.text_primary,
                width - pad * 2.0,
            );
            let g = ui.fonts_mut(|f| f.layout_job(j));
            let h = g.rect.height() + pad * 2.0;
            out.push(Piece::Fill(
                Rect::from_min_size(Pos2::new(x, 0.0), Vec2::new(width, h)),
                c.bg_raised,
                radius::M,
            ));
            out.push(Piece::Text(Pos2::new(x + pad, pad), g, Vec::new()));
            h + gap
        }
        Block::Html(t) => {
            let j = LayoutJob::simple(t.clone(), st.mono(st.body * 0.9), c.text_muted, width);
            let g = ui.fonts_mut(|f| f.layout_job(j));
            let h = g.rect.height();
            out.push(Piece::Text(Pos2::new(x, 0.0), g, Vec::new()));
            h + gap
        }
        Block::Rule => {
            let y = gap * 0.5;
            out.push(Piece::Line(
                Pos2::new(x, y),
                Pos2::new(x + width, y),
                c.border_strong,
            ));
            gap
        }
        Block::Quote(blocks) => {
            let indent = space::L * st.zoom;
            let mut inner = Vec::new();
            let mut y = 0.0;
            for b in blocks {
                let mut sub = Vec::new();
                let h = lay(ui, b, st, x + indent, width - indent, &mut sub);
                inner.extend(sub.into_iter().map(|p| offset(p, y)));
                y += h;
            }
            let h = (y - gap).max(st.body);
            out.push(Piece::Fill(
                Rect::from_min_size(Pos2::new(x, 0.0), Vec2::new(3.0 * st.zoom, h)),
                c.accent.gamma_multiply(0.5),
                0.0,
            ));
            out.extend(inner);
            h + gap
        }
        Block::List { start, items } => {
            let indent = space::XL * st.zoom;
            let mut y = 0.0;
            for (n, it) in items.iter().enumerate() {
                let marker_h = st.body * 1.3;
                match (it.task, start) {
                    (Some(done), _) => {
                        let s = st.body * 0.95;
                        let r = Rect::from_min_size(
                            Pos2::new(
                                x + indent - s - space::S * st.zoom,
                                y + (marker_h - s) / 2.0,
                            ),
                            Vec2::splat(s),
                        );
                        out.push(Piece::Check(r, done, c.border_strong, c.accent));
                    }
                    (None, Some(first)) => {
                        let label = format!("{}.", first + n as u64);
                        let g = ui.fonts_mut(|f| {
                            f.layout_no_wrap(label, st.prop(st.body), c.text_secondary)
                        });
                        let w = g.rect.width();
                        out.push(Piece::Text(
                            Pos2::new(x + indent - w - space::S * st.zoom, y),
                            g,
                            Vec::new(),
                        ));
                    }
                    (None, None) => {
                        let g = ui.fonts_mut(|f| {
                            f.layout_no_wrap("•".into(), st.prop(st.body), c.text_secondary)
                        });
                        let w = g.rect.width();
                        out.push(Piece::Text(
                            Pos2::new(x + indent - w - space::S * st.zoom, y),
                            g,
                            Vec::new(),
                        ));
                    }
                }
                let mut iy = 0.0;
                for (k, b) in it.blocks.iter().enumerate() {
                    let mut sub = Vec::new();
                    let mut h = lay(ui, b, st, x + indent, width - indent, &mut sub);
                    // Tight items: no paragraph gap inside the list.
                    if k + 1 == it.blocks.len() {
                        h -= gap * 0.6;
                    }
                    out.extend(sub.into_iter().map(|p| offset(p, y + iy)));
                    iy += h;
                }
                y += iy.max(marker_h);
            }
            y + gap * 0.6
        }
        Block::Table {
            align,
            header,
            rows,
        } => {
            let cols = header
                .len()
                .max(rows.iter().map(Vec::len).max().unwrap_or(0))
                .max(1);
            let pad = space::S * st.zoom;
            let col_w = width / cols as f32;
            let mut y = 0.0;
            let all: Vec<(bool, &Vec<Vec<Inline>>)> = std::iter::once((true, header))
                .chain(rows.iter().map(|r| (false, r)))
                .collect();
            for (row_i, (head, cells)) in all.iter().enumerate() {
                let mut row_pieces = Vec::new();
                let mut row_h: f32 = st.body;
                for (ci, cell) in cells.iter().enumerate().take(cols) {
                    let (j, links) = job(
                        cell,
                        st,
                        Look {
                            size: st.body,
                            bold: *head,
                            color: c.text_primary,
                        },
                        col_w - pad * 2.0,
                    );
                    let g = ui.fonts_mut(|f| f.layout_job(j));
                    let gw = g.rect.width();
                    let cx = match align.get(ci).copied().unwrap_or(Align::None) {
                        Align::Right => x + col_w * (ci as f32 + 1.0) - pad - gw,
                        Align::Center => x + col_w * ci as f32 + (col_w - gw) / 2.0,
                        _ => x + col_w * ci as f32 + pad,
                    };
                    row_h = row_h.max(g.rect.height());
                    row_pieces.push(Piece::Text(Pos2::new(cx, y + pad), g, links));
                }
                let h = row_h + pad * 2.0;
                if *head {
                    out.push(Piece::Fill(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, h)),
                        c.bg_raised,
                        0.0,
                    ));
                } else if row_i % 2 == 0 {
                    out.push(Piece::Fill(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, h)),
                        c.bg_raised.gamma_multiply(0.5),
                        0.0,
                    ));
                }
                out.extend(row_pieces);
                out.push(Piece::Line(
                    Pos2::new(x, y + h),
                    Pos2::new(x + width, y + h),
                    c.border_subtle,
                ));
                y += h;
            }
            out.push(Piece::Box(
                Rect::from_min_size(Pos2::new(x, 0.0), Vec2::new(width, y)),
                c.border_subtle,
            ));
            y + gap
        }
        Block::Footnote { label, blocks } => {
            let g = ui.fonts_mut(|f| {
                f.layout_no_wrap(format!("[{label}]"), st.prop(st.body * 0.85), c.text_muted)
            });
            let w = g.rect.width() + space::S * st.zoom;
            out.push(Piece::Text(Pos2::new(x, 0.0), g, Vec::new()));
            let mut y = 0.0;
            for b in blocks {
                let mut sub = Vec::new();
                let h = lay(ui, b, st, x + w, width - w, &mut sub);
                out.extend(sub.into_iter().map(|p| offset(p, y)));
                y += h;
            }
            y.max(st.body + gap)
        }
    }
}

fn offset(p: Piece, dy: f32) -> Piece {
    let d = Vec2::new(0.0, dy);
    match p {
        Piece::Text(at, g, l) => Piece::Text(at + d, g, l),
        Piece::Fill(r, c, rad) => Piece::Fill(r.translate(d), c, rad),
        Piece::Line(a, b, c) => Piece::Line(a + d, b + d, c),
        Piece::Box(r, c) => Piece::Box(r.translate(d), c),
        Piece::Check(r, on, a, b) => Piece::Check(r.translate(d), on, a, b),
        Piece::Image(r, id) => Piece::Image(r.translate(d), id),
        Piece::Hot(r, p, t) => Piece::Hot(r.translate(d), p, t),
    }
}

fn layout_block(ui: &egui::Ui, b: &Block, st: &Style, x: f32) -> Laid {
    let mut pieces = Vec::new();
    let height = lay(ui, b, st, x, st.width, &mut pieces);
    Laid { pieces, height }
}

/// What the preview did this frame.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PreviewOutput {
    /// A link the user Ctrl+clicked.
    pub open: Option<String>,
    /// A link the user clicked without Ctrl (the window says how to open it).
    pub plain_click: Option<String>,
    /// A local picture the user Ctrl+clicked (shown in Explorer).
    pub reveal: Option<PathBuf>,
    /// A local picture clicked without Ctrl.
    pub plain_image: Option<PathBuf>,
}

/// Draws `r` into `rect`.
pub fn show(
    ui: &mut egui::Ui,
    rect: Rect,
    r: &Rendered,
    state: &mut PreviewState,
    zoom: f32,
    doc: Option<&Path>,
    lang: kim_design::Lang,
) -> PreviewOutput {
    let c = Colors::get(ui.ctx());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, c.bg_base);
    let resp = ui.interact(rect, egui::Id::unique("jarak-preview"), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Document, true, "Markdown preview"));
    let pad = space::XL * zoom;
    let bar_w = size::PROGRESS * 2.0;
    let max_w = 820.0 * zoom;
    let width = (rect.width() - pad * 2.0 - bar_w).clamp(120.0, max_w);
    let x = rect.left() + ((rect.width() - bar_w - width) / 2.0).max(pad);
    let st = Style {
        body: (font::EMPHASIS + 1.0) * zoom,
        zoom,
        c,
        width,
        ctx: ui.ctx().clone(),
        doc: doc.map(Path::to_path_buf),
        media: state.media.clone(),
        lang,
    };
    if let Ok(mut m) = state.media.lock() {
        m.begin_frame(ui.ctx());
    }
    // The wheel scrolls by block.
    let scroll = if resp.hovered() {
        ui.input(|i| {
            if i.modifiers.command {
                0.0
            } else {
                i.smooth_scroll_delta.y
            }
        })
    } else {
        0.0
    };
    if scroll != 0.0 {
        state.top_px -= scroll;
        while state.top_px < 0.0 && state.top_block > 0 {
            state.top_block -= 1;
            state.top_px += layout_block(ui, &r.blocks[state.top_block].block, &st, x).height;
        }
        state.top_px = state.top_px.max(0.0);
        while state.top_block + 1 < r.blocks.len() {
            let h = layout_block(ui, &r.blocks[state.top_block].block, &st, x).height;
            if state.top_px < h {
                break;
            }
            state.top_px -= h;
            state.top_block += 1;
        }
    }
    state.top_block = state.top_block.min(r.blocks.len().saturating_sub(1));
    // Never past the end: when the document's end is on screen with blank space under it, move
    // up until its last block ends at the bottom of the view (or the document starts at the top).
    let view_h = rect.height() - pad * 2.0;
    let mut below = -state.top_px;
    let mut k = state.top_block;
    while k < r.blocks.len() && below < view_h {
        below += layout_block(ui, &r.blocks[k].block, &st, x).height;
        k += 1;
    }
    let mut need = view_h - below;
    while need > 0.5 {
        if state.top_px >= need {
            state.top_px -= need;
            break;
        }
        need -= state.top_px;
        state.top_px = 0.0;
        if state.top_block == 0 {
            break;
        }
        state.top_block -= 1;
        state.top_px = layout_block(ui, &r.blocks[state.top_block].block, &st, x).height;
    }
    state.links.clear();
    state.hot.clear();
    let mut y = rect.top() + pad - state.top_px;
    let mut i = state.top_block;
    let mut shown = 0usize;
    while i < r.blocks.len() && y < rect.bottom() {
        let laid = layout_block(ui, &r.blocks[i].block, &st, x);
        for p in &laid.pieces {
            match p {
                Piece::Text(at, g, links) => {
                    let pos = Pos2::new(at.x, y + at.y);
                    painter.galley(pos, g.clone(), c.text_primary);
                    for (range, url) in links {
                        let a = g.pos_from_cursor(CCursor::new(range.start));
                        let b = g.pos_from_cursor(CCursor::new(range.end));
                        let link_rect = if (a.top() - b.top()).abs() < 0.5 {
                            Rect::from_min_max(
                                pos + a.left_top().to_vec2(),
                                pos + b.right_bottom().to_vec2(),
                            )
                        } else {
                            // Across rows: the rows it touches, full width.
                            Rect::from_min_max(
                                pos + Vec2::new(0.0, a.top()),
                                pos + Vec2::new(g.rect.width(), b.bottom()),
                            )
                        };
                        state.links.push((link_rect, url.clone()));
                    }
                }
                Piece::Fill(r2, col, rad) => {
                    painter.rect_filled(
                        r2.translate(Vec2::new(0.0, y)),
                        CornerRadius::same(*rad as u8),
                        *col,
                    );
                }
                Piece::Line(a, b, col) => {
                    painter.line_segment(
                        [*a + Vec2::new(0.0, y), *b + Vec2::new(0.0, y)],
                        Stroke::new(size::BORDER, *col),
                    );
                }
                Piece::Box(r2, col) => {
                    painter.rect_stroke(
                        r2.translate(Vec2::new(0.0, y)),
                        CornerRadius::ZERO,
                        Stroke::new(size::BORDER, *col),
                        egui::StrokeKind::Inside,
                    );
                }
                Piece::Image(r2, id) => {
                    let r2 = r2.translate(Vec2::new(0.0, y));
                    painter.image(
                        *id,
                        r2,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    if kim_ui::inspect::active() {
                        kim_ui::inspect::record(
                            ui,
                            "preview:image",
                            "image",
                            r2,
                            None,
                            true,
                            &[],
                            None,
                            None,
                        );
                    }
                }
                Piece::Hot(r2, path, tip) => {
                    state
                        .hot
                        .push((r2.translate(Vec2::new(0.0, y)), path.clone(), tip.clone()));
                }
                Piece::Check(r2, on, edge, fill) => {
                    let r2 = r2.translate(Vec2::new(0.0, y));
                    if *on {
                        painter.rect_filled(r2, CornerRadius::same(2), *fill);
                        let m = r2.shrink(r2.width() * 0.22);
                        painter.line_segment(
                            [
                                Pos2::new(m.left(), m.center().y),
                                Pos2::new(m.center().x - m.width() * 0.1, m.bottom()),
                            ],
                            Stroke::new(1.6, c.text_on_accent),
                        );
                        painter.line_segment(
                            [
                                Pos2::new(m.center().x - m.width() * 0.1, m.bottom()),
                                Pos2::new(m.right(), m.top()),
                            ],
                            Stroke::new(1.6, c.text_on_accent),
                        );
                    } else {
                        painter.rect_stroke(
                            r2,
                            CornerRadius::same(2),
                            Stroke::new(size::BORDER, *edge),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
        }
        y += laid.height;
        i += 1;
        shown += 1;
    }
    // The scrollbar: the top block's place among all blocks.
    if r.blocks.len() > shown.max(1) {
        let track = Rect::from_min_max(
            Pos2::new(rect.right() - bar_w, rect.top()),
            rect.right_bottom(),
        );
        let frac = (shown as f32 / r.blocks.len() as f32).clamp(0.05, 1.0);
        let h = (track.height() * frac).max(size::HIT_MIN);
        let t = state.top_block as f32 / (r.blocks.len() - 1).max(1) as f32;
        let thumb = Rect::from_min_size(
            Pos2::new(track.left() + 2.0, track.top() + (track.height() - h) * t),
            Vec2::new(bar_w - 4.0, h),
        );
        painter.rect_filled(thumb, CornerRadius::same(2), c.border_subtle);
    }
    // Links: the pointer shows a hand and the address; only Ctrl+click opens.
    let mut out = PreviewOutput::default();
    if let Some(p) = ui.input(|i| i.pointer.hover_pos())
        && let Some((_, url)) = state.links.iter().find(|(r2, _)| r2.contains(p))
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let url = url.clone();
        resp.clone().on_hover_text_at_pointer(format!(
            "{url}\n{}",
            lang.pick("Ctrl+click to open", "Ctrl+클릭으로 엽니다")
        ));
        if resp.clicked() {
            if ui.input(|i| i.modifiers.command) {
                out.open = Some(url);
            } else {
                out.plain_click = Some(url);
            }
        }
    } else if let Some(p) = ui.input(|i| i.pointer.hover_pos())
        && let Some((_, path, tip)) = state.hot.iter().find(|(r2, _, _)| r2.contains(p))
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let path = path.clone();
        resp.clone().on_hover_text_at_pointer(format!(
            "{tip}\n{}",
            lang.pick(
                "Ctrl+click to show it in Explorer",
                "Ctrl+클릭하면 탐색기에서 보여 줍니다"
            )
        ));
        if resp.clicked() {
            if ui.input(|i| i.modifiers.command) {
                out.reveal = Some(path);
            } else {
                out.plain_image = Some(path);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_and_mail_links_open() {
        for ok in [
            "https://x.org",
            "HTTP://X.ORG/a",
            "mailto:a@b.c",
            " https://x.org ",
        ] {
            assert!(openable(ok), "{ok}");
        }
        for no in [
            "file:///C:/x.exe",
            "javascript:alert(1)",
            "C:\\x.exe",
            "assets/a.png",
            "ftp://x",
        ] {
            assert!(!openable(no), "{no}");
        }
        assert!(!NoOpener.open_url("https://x.org"));
    }

    #[test]
    fn the_parse_is_cached_per_history_state_and_maps_lines_both_ways() {
        let mut p = PreviewState::default();
        let mut parsed = 0;
        let src = "# A\n\none\n\ntwo\n";
        let r1 = p.rendered(1, || {
            parsed += 1;
            src.to_string()
        });
        let r2 = p.rendered(1, || {
            parsed += 1;
            src.to_string()
        });
        assert!(Arc::ptr_eq(&r1, &r2));
        assert_eq!(parsed, 1, "one parse for one state");
        p.scroll_to_line(&r1, 4);
        assert_eq!(p.top_block, 2);
        assert_eq!(p.top_line(&r1), Some(4));
        p.scroll_to_line(&r1, 1);
        assert_eq!(p.top_block, 1, "a blank line shows the next block");
    }
}
