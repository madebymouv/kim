//! Printing (`JARAK-FR-140`, the PDF half of `JARAK-FR-141`; Step 106): a document laid out into
//! pages of drawing operations here, drawn by a printer backend ([`Printers`]; Windows GDI in
//! [`gdi`]).
//!
//! - The layout is a pure function of the text, the page's size and resolution, the settings and a
//!   [`Measure`] of the device's fonts, so it is tested without a printer (`FakeMeasure`).
//! - Highlighted text: monospace, wrapped at word boundaries (a long word at characters), tabs
//!   expanded, optional line numbers on a line's first row, the window's colours or black.
//! - A Markdown document prints rendered: headings, paragraphs, lists, quotes, code blocks, tables,
//!   rules, footnotes and its images (only those inside the document's folder, checked like the
//!   exports; never fetched).
//! - Header and footer are `left|center|right` templates with `{file}`, `{path}`, `{page}`,
//!   `{pages}`, `{date}` and `{time}`.
//! - PDF is printing to the *Microsoft Print to PDF* printer with an output file: no dialog.

use crate::export::{self, kind_color};
use crate::markdown::{self, Align, Block, Inline};
use crate::syntax::Lang;
use kim_design::Rgb;
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthChar;

#[cfg(windows)]
pub mod gdi;

/// The printer that writes PDF files on Windows 10 and 11.
pub const PDF_PRINTER: &str = "Microsoft Print to PDF";
/// The largest image file printing decodes, and its pixel limits (the preview's).
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_IMAGE_SIDE: u32 = 16_384;
pub const MAX_IMAGE_PIXELS: u64 = 40_000_000;

/// A printing typeface: the editor's monospace or the rendered Markdown's sans serif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Mono,
    Sans,
}

/// A font: face, size in tenths of a point, weight and slant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Font {
    pub face: Face,
    pub pt10: u16,
    pub bold: bool,
    pub italic: bool,
}

impl Font {
    pub fn new(face: Face, pt: f32) -> Font {
        Font {
            face,
            pt10: (pt * 10.0).round().clamp(10.0, 2000.0) as u16,
            bold: false,
            italic: false,
        }
    }
    pub fn bold(mut self) -> Font {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Font {
        self.italic = true;
        self
    }
    pub fn pt(self) -> f32 {
        f32::from(self.pt10) / 10.0
    }
}

/// The device's font metrics, in device pixels.
pub trait Measure {
    fn char_width(&self, c: char, f: Font) -> f32;
    /// From the top of a line to its baseline.
    fn ascent(&self, f: Font) -> f32;
    /// A line's height.
    fn height(&self, f: Font) -> f32;
}

/// Fixed metrics for tests: every narrow character 0.6 em, wide ones twice that.
#[derive(Debug, Clone, Copy)]
pub struct FakeMeasure {
    pub dpi: f32,
}

impl Measure for FakeMeasure {
    fn char_width(&self, c: char, f: Font) -> f32 {
        let em = f.pt() * self.dpi / 72.0;
        em * 0.6 * c.width().unwrap_or(0).max(1) as f32
    }
    fn ascent(&self, f: Font) -> f32 {
        f.pt() * self.dpi / 72.0
    }
    fn height(&self, f: Font) -> f32 {
        f.pt() * self.dpi / 72.0 * 1.25
    }
}

/// The physical page in device pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub width: f32,
    pub height: f32,
    pub dpi: f32,
}

impl Geometry {
    /// A4 portrait at `dpi`.
    pub fn a4(dpi: f32) -> Geometry {
        Geometry {
            width: 210.0 / 25.4 * dpi,
            height: 297.0 / 25.4 * dpi,
            dpi,
        }
    }
    fn mm(&self, mm: f32) -> f32 {
        mm / 25.4 * self.dpi
    }
}

/// Colours on paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colors {
    /// kim-design's light palette (the window's token colours).
    #[default]
    Color,
    /// Everything black on white.
    Black,
}

/// What the print settings say (`print_*` in settings.toml).
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// Left, top, right, bottom, in millimetres.
    pub margins_mm: [f32; 4],
    /// The text's size (highlighted text and code blocks); rendered Markdown's body is one point
    /// larger.
    pub font_pt: f32,
    pub line_numbers: bool,
    pub colors: Colors,
    pub header: String,
    pub footer: String,
    pub tab_width: usize,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            margins_mm: [15.0, 15.0, 15.0, 15.0],
            font_pt: 10.0,
            line_numbers: true,
            colors: Colors::Color,
            header: DEFAULT_HEADER.into(),
            footer: DEFAULT_FOOTER.into(),
            tab_width: 4,
        }
    }
}

pub const DEFAULT_HEADER: &str = "{file}||{date} {time}";
pub const DEFAULT_FOOTER: &str = "||{page} / {pages}";

/// The values of the header's and footer's variables.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vars {
    pub file: String,
    pub path: String,
    pub date: String,
    pub time: String,
}

/// A drawing operation; `y` of text is its baseline.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Text {
        x: f32,
        y: f32,
        font: Font,
        color: Rgb,
        text: String,
        strike: bool,
        underline: bool,
    },
    Fill {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Rgb,
    },
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        path: PathBuf,
    },
}

impl Op {
    fn shift(&mut self, dx: f32, dy: f32) {
        match self {
            Op::Text { x, y, .. } | Op::Fill { x, y, .. } | Op::Image { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
        }
    }
}

/// One printed page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub ops: Vec<Op>,
}

/// A template's three parts with the variables filled in.
pub fn expand(template: &str, vars: &Vars, page: usize, pages: usize) -> [String; 3] {
    let fill = |s: &str| {
        s.replace("{file}", &vars.file)
            .replace("{path}", &vars.path)
            .replace("{date}", &vars.date)
            .replace("{time}", &vars.time)
            .replace("{page}", &page.to_string())
            .replace("{pages}", &pages.to_string())
    };
    let mut parts = template.splitn(3, '|');
    [
        fill(parts.next().unwrap_or("")),
        fill(parts.next().unwrap_or("")),
        fill(parts.next().unwrap_or("")),
    ]
}

/// The paper colour of a token colour (black in black-and-white).
fn ink(setup: &Setup, c: Rgb) -> Rgb {
    match setup.colors {
        Colors::Color => c,
        Colors::Black => Rgb { r: 0, g: 0, b: 0 },
    }
}

/// A row of content: its height and its operations (relative to the row's top-left).
#[derive(Debug, Clone, Default)]
struct Row {
    h: f32,
    ops: Vec<Op>,
}

/// A character with its style, for wrapping.
#[derive(Debug, Clone, Copy)]
struct Styled {
    c: char,
    font: Font,
    color: Rgb,
    strike: bool,
    underline: bool,
}

/// Breaks styled characters into lines of at most `width` (at the last blank when there is one,
/// else at a character; every line holds at least one character). `'\n'` forces a break.
fn wrap(chars: &[Styled], width: f32, m: &dyn Measure) -> Vec<Vec<Styled>> {
    let mut lines = Vec::new();
    let mut cur: Vec<Styled> = Vec::new();
    let mut w = 0.0f32;
    let mut last_blank: Option<usize> = None;
    for s in chars {
        if s.c == '\n' {
            lines.push(std::mem::take(&mut cur));
            w = 0.0;
            last_blank = None;
            continue;
        }
        let cw = m.char_width(s.c, s.font);
        if w + cw > width && !cur.is_empty() {
            if s.c == ' ' {
                // The blank ends the line (it prints nothing at its end).
                cur.push(*s);
                lines.push(std::mem::take(&mut cur));
                w = 0.0;
                last_blank = None;
                continue;
            }
            match last_blank {
                Some(b) if b + 1 < cur.len() => {
                    let rest = cur.split_off(b + 1);
                    lines.push(std::mem::take(&mut cur));
                    cur = rest;
                }
                _ => lines.push(std::mem::take(&mut cur)),
            }
            last_blank = None;
            w = cur.iter().map(|x| m.char_width(x.c, x.font)).sum();
        }
        if s.c == ' ' {
            last_blank = Some(cur.len());
        }
        cur.push(*s);
        w += cw;
    }
    lines.push(cur);
    lines
}

/// Text operations for one wrapped line starting at `x0`; `baseline` is the row's.
fn line_ops(line: &[Styled], x0: f32, baseline: f32, m: &dyn Measure) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    let mut x = x0;
    for s in line {
        let same = matches!(ops.last(), Some(Op::Text { font, color, strike, underline, .. })
            if *font == s.font && *color == s.color && *strike == s.strike && *underline == s.underline);
        if same {
            if let Some(Op::Text { text, .. }) = ops.last_mut() {
                text.push(s.c);
            }
        } else {
            ops.push(Op::Text {
                x,
                y: baseline,
                font: s.font,
                color: s.color,
                text: s.c.to_string(),
                strike: s.strike,
                underline: s.underline,
            });
        }
        x += m.char_width(s.c, s.font);
    }
    // Blank-only runs draw nothing.
    ops.retain(|o| !matches!(o, Op::Text { text, .. } if text.trim().is_empty()));
    ops
}

/// The content area of a page, after margins, header and footer.
struct Area {
    x0: f32,
    width: f32,
    top: f32,
    bottom: f32,
}

fn area(setup: &Setup, g: &Geometry, m: &dyn Measure) -> Area {
    let [l, t, r, b] = setup.margins_mm.map(|v| g.mm(v.clamp(0.0, 80.0)));
    let hf = Font::new(Face::Sans, 8.0);
    let band = m.height(hf) * 1.6;
    let top = t + if setup.header.trim().is_empty() {
        0.0
    } else {
        band
    };
    let bottom = g.height
        - b
        - if setup.footer.trim().is_empty() {
            0.0
        } else {
            band
        };
    let width = (g.width - l - r).max(g.mm(20.0));
    Area {
        x0: l,
        width,
        top,
        bottom: bottom.max(top + m.height(hf) * 4.0),
    }
}

/// Rows into pages, then the header and footer on each.
fn paginate(
    rows: Vec<Row>,
    setup: &Setup,
    g: &Geometry,
    m: &dyn Measure,
    vars: &Vars,
) -> Vec<Page> {
    let a = area(setup, g, m);
    let mut pages: Vec<Page> = vec![Page::default()];
    let mut y = a.top;
    for mut row in rows {
        if y + row.h > a.bottom && y > a.top {
            pages.push(Page::default());
            y = a.top;
        }
        for op in &mut row.ops {
            op.shift(a.x0, y);
        }
        if let Some(p) = pages.last_mut() {
            p.ops.append(&mut row.ops);
        }
        y += row.h;
    }
    let n = pages.len();
    let hf = Font::new(Face::Sans, 8.0);
    let muted = ink(setup, kim_design::LIGHT.text_muted);
    let rule = ink(setup, kim_design::LIGHT.border_strong);
    let [l, t, r, b] = setup.margins_mm.map(|v| g.mm(v.clamp(0.0, 80.0)));
    let right = g.width - r;
    let text_w = |s: &str| s.chars().map(|c| m.char_width(c, hf)).sum::<f32>();
    let band = |parts: [String; 3], baseline: f32, line_y: f32, ops: &mut Vec<Op>| {
        let [left, center, rt] = parts;
        for (s, x) in [
            (left.as_str(), l),
            (center.as_str(), l + (right - l - text_w(&center)) / 2.0),
            (rt.as_str(), right - text_w(&rt)),
        ] {
            if !s.trim().is_empty() {
                ops.push(Op::Text {
                    x,
                    y: baseline,
                    font: hf,
                    color: muted,
                    text: s.to_string(),
                    strike: false,
                    underline: false,
                });
            }
        }
        ops.push(Op::Fill {
            x: l,
            y: line_y,
            w: right - l,
            h: (g.dpi / 150.0).max(1.0),
            color: rule,
        });
    };
    for (i, p) in pages.iter_mut().enumerate() {
        if !setup.header.trim().is_empty() {
            let parts = expand(&setup.header, vars, i + 1, n);
            band(parts, t + m.ascent(hf), t + m.height(hf) * 1.2, &mut p.ops);
        }
        if !setup.footer.trim().is_empty() {
            let parts = expand(&setup.footer, vars, i + 1, n);
            let fy = g.height - b - m.height(hf);
            band(
                parts,
                fy + m.ascent(hf),
                fy - m.height(hf) * 0.3,
                &mut p.ops,
            );
        }
    }
    pages
}

/// How highlighted text is set: its font, line numbers, a background behind each row.
#[derive(Debug, Clone, Copy)]
struct CodeStyle {
    font: Font,
    numbers: bool,
    background: Option<Rgb>,
}

/// Highlighted text laid out into rows of `width` (line numbers in a gutter when asked).
fn code_rows(
    text: &str,
    lang: Option<&Lang>,
    setup: &Setup,
    width: f32,
    m: &dyn Measure,
    cs: CodeStyle,
) -> Vec<Row> {
    let font = cs.font;
    let rows = export::highlight(text, lang);
    let digits = rows.len().to_string().len();
    let gutter = if cs.numbers {
        (0..digits + 2)
            .map(|_| m.char_width('0', font))
            .sum::<f32>()
    } else {
        0.0
    };
    let pad = if cs.background.is_some() {
        m.char_width(' ', font) * 1.5
    } else {
        0.0
    };
    let lh = m.height(font);
    let base = m.ascent(font);
    let plain = ink(setup, kim_design::LIGHT.text_primary);
    let muted = ink(setup, kim_design::LIGHT.text_muted);
    let tab = setup.tab_width.max(1);
    let mut out = Vec::new();
    for (n, (_, runs)) in rows.iter().enumerate() {
        let mut chars: Vec<Styled> = Vec::new();
        let mut col = 0;
        for (k, t) in runs {
            let color = k.map_or(plain, |k| ink(setup, kind_color(&kim_design::LIGHT, k)));
            let f = if *k == Some(crate::syntax::Kind::Emphasis) {
                font.italic()
            } else {
                font
            };
            let styled = |c: char| Styled {
                c,
                font: f,
                color,
                strike: false,
                underline: false,
            };
            for c in t.chars() {
                if c == '\t' {
                    let n = tab - col % tab;
                    chars.extend(std::iter::repeat_n(styled(' '), n));
                    col += n;
                } else {
                    chars.push(styled(c));
                    col += c.width().unwrap_or(0);
                }
            }
        }
        let lines = wrap(&chars, (width - gutter - pad * 2.0).max(lh), m);
        for (i, line) in lines.iter().enumerate() {
            let mut row = Row {
                h: lh,
                ops: Vec::new(),
            };
            if let Some(bg) = cs.background {
                row.ops.push(Op::Fill {
                    x: 0.0,
                    y: 0.0,
                    w: width,
                    h: lh,
                    color: bg,
                });
            }
            if cs.numbers && i == 0 {
                let label = (n + 1).to_string();
                let w: f32 = label.chars().map(|c| m.char_width(c, font)).sum();
                row.ops.push(Op::Text {
                    x: gutter - w - m.char_width('0', font),
                    y: base,
                    font,
                    color: muted,
                    text: label,
                    strike: false,
                    underline: false,
                });
            }
            row.ops.extend(line_ops(line, gutter + pad, base, m));
            out.push(row);
        }
    }
    out
}

/// Highlighted text as pages.
pub fn code_pages(
    text: &str,
    lang: Option<&Lang>,
    setup: &Setup,
    g: &Geometry,
    m: &dyn Measure,
    vars: &Vars,
) -> Vec<Page> {
    let a = area(setup, g, m);
    let font = Font::new(Face::Mono, setup.font_pt);
    let cs = CodeStyle {
        font,
        numbers: setup.line_numbers,
        background: None,
    };
    let rows = code_rows(text, lang, setup, a.width, m, cs);
    paginate(rows, setup, g, m, vars)
}

/// The size of an image file, read from its header within the limits (`None`: not shown).
pub fn image_size(path: &Path) -> Option<(u32, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let format = image_format_of(path)?;
    let file = std::fs::File::open(path).ok()?;
    let reader = image::ImageReader::with_format(std::io::BufReader::new(file), format);
    let (w, h) =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader.into_dimensions()))
            .ok()?
            .ok()?;
    (w > 0
        && h > 0
        && w <= MAX_IMAGE_SIDE
        && h <= MAX_IMAGE_SIDE
        && u64::from(w) * u64::from(h) <= MAX_IMAGE_PIXELS)
        .then_some((w, h))
}

fn image_format_of(path: &Path) -> Option<image::ImageFormat> {
    match crate::assets::image_format(&crate::assets::file_head(path))? {
        "png" => Some(image::ImageFormat::Png),
        "jpeg" => Some(image::ImageFormat::Jpeg),
        "gif" => Some(image::ImageFormat::Gif),
        "bmp" => Some(image::ImageFormat::Bmp),
        "webp" => Some(image::ImageFormat::WebP),
        _ => None,
    }
}

/// Decodes an image file for printing within the limits: width, height, RGBA.
pub fn decode_image(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_IMAGE_BYTES {
        return Err("the image file is too large".into());
    }
    let format = image_format_of(path).ok_or("not an image Jarak knows")?;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 8);
    reader.limits(limits);
    let img = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader.decode()))
        .map_err(|_| "the image could not be decoded".to_string())?
        .map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

/// Lays out rendered Markdown.
struct Md<'a> {
    setup: &'a Setup,
    m: &'a dyn Measure,
    doc: Option<&'a Path>,
    dpi: f32,
    width: f32,
    /// The content area's height (an image is never taller).
    max_h: f32,
    body: Font,
    rows: Vec<Row>,
}

impl Md<'_> {
    fn text_color(&self) -> Rgb {
        ink(self.setup, kim_design::LIGHT.text_primary)
    }

    fn gap(&mut self, h: f32) {
        if !self.rows.is_empty() {
            self.rows.push(Row { h, ops: Vec::new() });
        }
    }

    /// Inline content flattened into styled characters; images become their own rows (pushed
    /// through `flush`).
    fn styled(&self, v: &[Inline], font: Font, color: Rgb, out: &mut Vec<Item>) {
        fn push(out: &mut Vec<Item>, t: &str, f: Font, c: Rgb) {
            for ch in t.chars() {
                out.push(Item::Char(Styled {
                    c: if ch == '\t' || ch == '\r' { ' ' } else { ch },
                    font: f,
                    color: c,
                    strike: false,
                    underline: false,
                }));
            }
        }
        for i in v {
            match i {
                Inline::Text(t) => push(out, t, font, color),
                Inline::Code(t) => push(
                    out,
                    t,
                    Font::new(Face::Mono, self.setup.font_pt),
                    ink(self.setup, kim_design::LIGHT.success),
                ),
                Inline::Emph(v) => self.styled(v, font.italic(), color, out),
                Inline::Strong(v) => self.styled(v, font.bold(), color, out),
                Inline::Strike(v) => {
                    let mut inner = Vec::new();
                    self.styled(v, font, color, &mut inner);
                    for it in &mut inner {
                        if let Item::Char(s) = it {
                            s.strike = true;
                        }
                    }
                    out.extend(inner);
                }
                // A link the exports would drop (`javascript:`, …) prints as its text.
                Inline::Link { url, children } if export::safe_href(url).is_none() => {
                    self.styled(children, font, color, out);
                }
                Inline::Link { children, .. } => {
                    let mut inner = Vec::new();
                    self.styled(
                        children,
                        font,
                        ink(self.setup, kim_design::LIGHT.accent),
                        &mut inner,
                    );
                    for it in &mut inner {
                        if let Item::Char(s) = it {
                            s.underline = true;
                        }
                    }
                    out.extend(inner);
                }
                Inline::Image { url, alt } => out.push(Item::Image(url.clone(), alt.clone())),
                Inline::SoftBreak => push(out, " ", font, color),
                Inline::HardBreak => push(out, "\n", font, color),
                Inline::FootnoteRef(l) => push(
                    out,
                    &format!("[{l}]"),
                    Font::new(font.face, font.pt() * 0.8),
                    ink(self.setup, kim_design::LIGHT.accent),
                ),
                Inline::Html(h) => push(
                    out,
                    h,
                    font,
                    ink(self.setup, kim_design::LIGHT.text_secondary),
                ),
            }
        }
    }

    /// A flow of styled characters and images as rows at `indent`, the first row with `marker`.
    fn flow(
        &mut self,
        items: &[Item],
        indent: f32,
        marker: Option<(String, Font, Rgb)>,
        deco: Deco,
    ) {
        let mut chars: Vec<Styled> = Vec::new();
        let mut marker = marker;
        let flush =
            |me: &mut Self, chars: &mut Vec<Styled>, marker: &mut Option<(String, Font, Rgb)>| {
                if chars.is_empty() && marker.is_none() {
                    return;
                }
                let width = (me.width - indent - deco.pad * 2.0).max(10.0);
                for line in wrap(chars, width, me.m) {
                    let h = line
                        .iter()
                        .map(|s| me.m.height(s.font))
                        .fold(me.m.height(me.body), f32::max);
                    let asc = line
                        .iter()
                        .map(|s| me.m.ascent(s.font))
                        .fold(me.m.ascent(me.body), f32::max);
                    let mut row = Row { h, ops: Vec::new() };
                    deco.draw(&mut row, h);
                    if let Some((t, f, c)) = marker.take() {
                        let w: f32 = t.chars().map(|ch| me.m.char_width(ch, f)).sum();
                        row.ops.push(Op::Text {
                            x: indent - w - me.m.char_width(' ', f),
                            y: asc,
                            font: f,
                            color: c,
                            text: t,
                            strike: false,
                            underline: false,
                        });
                    }
                    row.ops
                        .extend(line_ops(&line, indent + deco.pad, asc, me.m));
                    me.rows.push(row);
                }
                chars.clear();
            };
        for it in items {
            match it {
                Item::Char(s) => chars.push(*s),
                Item::Image(url, alt) => {
                    flush(self, &mut chars, &mut marker);
                    self.image(url, alt, indent);
                }
            }
        }
        flush(self, &mut chars, &mut marker);
    }

    fn image(&mut self, url: &str, alt: &str, indent: f32) {
        let lower = url.trim().to_ascii_lowercase();
        let shown = if alt.is_empty() { url } else { alt };
        let note = |why: &str| format!("[{shown}: {why}]");
        let path = if lower.starts_with("http://") || lower.starts_with("https://") {
            Err(note("remote image, not loaded"))
        } else {
            match self.doc.and_then(|d| crate::assets::resolve(d, url)) {
                None => Err(note("not shown")),
                Some(p) if !p.exists() => Err(note("not found")),
                Some(p) => match image_size(&p) {
                    Some(s) => Ok((p, s)),
                    None => Err(note("not an image Jarak prints")),
                },
            }
        };
        match path {
            Ok((p, (w, h))) => {
                // At its size on a 96 dpi screen, narrower than the column, shorter than a page.
                let mut iw = w as f32 * self.dpi / 96.0;
                let mut ih = h as f32 * self.dpi / 96.0;
                let maxw = self.width - indent;
                if iw > maxw {
                    ih *= maxw / iw;
                    iw = maxw;
                }
                if ih > self.max_h * 0.9 {
                    iw *= self.max_h * 0.9 / ih;
                    ih = self.max_h * 0.9;
                }
                self.rows.push(Row {
                    h: ih + self.m.height(self.body) * 0.3,
                    ops: vec![Op::Image {
                        x: indent,
                        y: 0.0,
                        w: iw,
                        h: ih,
                        path: p,
                    }],
                });
            }
            Err(t) => {
                let f = self.body.italic();
                let c = ink(self.setup, kim_design::LIGHT.text_muted);
                let items: Vec<Item> = t
                    .chars()
                    .map(|ch| {
                        Item::Char(Styled {
                            c: ch,
                            font: f,
                            color: c,
                            strike: false,
                            underline: false,
                        })
                    })
                    .collect();
                self.flow(&items, indent, None, Deco::NONE);
            }
        }
    }

    fn block(&mut self, b: &Block, indent: f32, quote: u8) {
        let lh = self.m.height(self.body);
        let deco = Deco::quote(quote, self.setup, self.m, self.body);
        let color = if quote > 0 {
            ink(self.setup, kim_design::LIGHT.text_secondary)
        } else {
            self.text_color()
        };
        match b {
            Block::Heading { level, inlines } => {
                let scale = [2.0, 1.6, 1.3, 1.15, 1.0, 0.9][usize::from(*level).clamp(1, 6) - 1];
                let f = Font::new(Face::Sans, self.body.pt() * scale).bold();
                self.gap(lh * 0.6);
                let mut items = Vec::new();
                self.styled(inlines, f, color, &mut items);
                self.flow(&items, indent, None, deco);
                if *level <= 2 {
                    self.rows.push(Row {
                        h: lh * 0.35,
                        ops: vec![Op::Fill {
                            x: indent,
                            y: lh * 0.1,
                            w: self.width - indent,
                            h: (lh / 18.0).max(1.0),
                            color: ink(self.setup, kim_design::LIGHT.border_strong),
                        }],
                    });
                }
                self.gap(lh * 0.2);
            }
            Block::Paragraph(v) => {
                let mut items = Vec::new();
                self.styled(v, self.body, color, &mut items);
                self.flow(&items, indent, None, deco);
                self.gap(lh * 0.5);
            }
            Block::List { start, items } => {
                // Step 109: deep nesting stops indenting at 60 % of the column.
                let step = (self.m.char_width('M', self.body) * 2.2)
                    .min((self.width * 0.6 - indent).max(0.0));
                for (n, it) in items.iter().enumerate() {
                    let marker = match (it.task, start) {
                        (Some(true), _) => "[x]".to_string(),
                        (Some(false), _) => "[ ]".to_string(),
                        (None, Some(s)) => format!("{}.", s + n as u64),
                        (None, None) => "•".to_string(),
                    };
                    let mut first = Some((marker, self.body, color));
                    for b in &it.blocks {
                        match b {
                            Block::Paragraph(v) => {
                                let mut inl = Vec::new();
                                self.styled(v, self.body, color, &mut inl);
                                self.flow(&inl, indent + step, first.take(), deco);
                            }
                            other => {
                                if let Some(m) = first.take() {
                                    self.flow(&[], indent + step, Some(m), deco);
                                }
                                self.block(other, indent + step, quote);
                            }
                        }
                    }
                }
                self.gap(lh * 0.4);
            }
            Block::Quote(v) => {
                for b in v {
                    // Step 109: at most 16 bars (hostile nesting stays finite).
                    self.block(b, indent, quote.saturating_add(1).min(16));
                }
            }
            Block::Code { lang, text } => {
                let l = crate::syntax::fence_lang(&lang.to_ascii_lowercase());
                let font = Font::new(Face::Mono, self.setup.font_pt);
                let bg = match self.setup.colors {
                    Colors::Color => Some(kim_design::LIGHT.bg_raised),
                    Colors::Black => Some(Rgb {
                        r: 240,
                        g: 240,
                        b: 240,
                    }),
                };
                let cs = CodeStyle {
                    font,
                    numbers: false,
                    background: bg,
                };
                let mut rows = code_rows(
                    text.trim_end_matches('\n'),
                    l,
                    self.setup,
                    self.width - indent,
                    self.m,
                    cs,
                );
                for r in &mut rows {
                    for op in &mut r.ops {
                        op.shift(indent, 0.0);
                    }
                }
                self.rows.extend(rows);
                self.gap(lh * 0.5);
            }
            Block::Table {
                align,
                header,
                rows,
            } => self.table(align, header, rows, indent),
            Block::Rule => {
                self.rows.push(Row {
                    h: lh,
                    ops: vec![Op::Fill {
                        x: indent,
                        y: lh * 0.5,
                        w: self.width - indent,
                        h: (lh / 18.0).max(1.0),
                        color: ink(self.setup, kim_design::LIGHT.border_strong),
                    }],
                });
            }
            Block::Html(h) => {
                let cs = CodeStyle {
                    font: Font::new(Face::Mono, self.setup.font_pt),
                    numbers: false,
                    background: None,
                };
                let rows = code_rows(
                    h.trim_end(),
                    None,
                    self.setup,
                    self.width - indent,
                    self.m,
                    cs,
                );
                let sec = ink(self.setup, kim_design::LIGHT.text_secondary);
                for mut r in rows {
                    for op in &mut r.ops {
                        op.shift(indent, 0.0);
                        if let Op::Text { color, .. } = op {
                            *color = sec;
                        }
                    }
                    self.rows.push(r);
                }
                self.gap(lh * 0.5);
            }
            Block::Footnote { label, blocks } => {
                let small = Font::new(Face::Sans, self.body.pt() * 0.85);
                let saved = self.body;
                self.body = small;
                let mut first = Some((format!("[{label}]"), small, color));
                for b in blocks {
                    if let Block::Paragraph(v) = b {
                        let mut inl = Vec::new();
                        self.styled(v, small, color, &mut inl);
                        let step = self.m.char_width('M', small) * 3.0;
                        self.flow(&inl, indent + step, first.take(), deco);
                    } else {
                        self.block(b, indent, quote);
                    }
                }
                self.body = saved;
            }
        }
    }

    fn table(
        &mut self,
        align: &[Align],
        header: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
        indent: f32,
    ) {
        let cols = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if cols == 0 {
            return;
        }
        let pad = self.m.char_width(' ', self.body) * 1.2;
        let line = (self.m.height(self.body) / 18.0).max(1.0);
        let border = ink(self.setup, kim_design::LIGHT.border_strong);
        let color = self.text_color();
        let cells = |me: &Self, r: &[Vec<Inline>], bold: bool| -> Vec<Vec<Styled>> {
            (0..cols)
                .map(|i| {
                    let mut items = Vec::new();
                    let f = if bold { me.body.bold() } else { me.body };
                    if let Some(c) = r.get(i) {
                        me.styled(c, f, color, &mut items);
                    }
                    items
                        .into_iter()
                        .filter_map(|it| match it {
                            Item::Char(s) => Some(s),
                            Item::Image(_, alt) => alt.chars().next().map(|c| Styled {
                                c,
                                font: f,
                                color,
                                strike: false,
                                underline: false,
                            }),
                        })
                        .collect()
                })
                .collect()
        };
        let all: Vec<(Vec<Vec<Styled>>, bool)> = std::iter::once((cells(self, header, true), true))
            .chain(rows.iter().map(|r| (cells(self, r, false), false)))
            .collect();
        // Natural widths, scaled down to the column when they do not fit.
        let mut widths = vec![0.0f32; cols];
        for (r, _) in &all {
            for (i, c) in r.iter().enumerate() {
                let w: f32 = c.iter().map(|s| self.m.char_width(s.c, s.font)).sum();
                widths[i] = widths[i].max(w + pad * 2.0);
            }
        }
        let avail = self.width - indent;
        let total: f32 = widths.iter().sum();
        if total > avail {
            for w in &mut widths {
                *w *= avail / total;
            }
        }
        let table_w: f32 = widths.iter().sum();
        for (r, head) in &all {
            let wrapped: Vec<Vec<Vec<Styled>>> = r
                .iter()
                .zip(&widths)
                .map(|(c, w)| wrap(c, (w - pad * 2.0).max(1.0), self.m))
                .collect();
            let lines = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let lh = self.m.height(self.body);
            let h = lines as f32 * lh + pad;
            let mut row = Row { h, ops: Vec::new() };
            if *head {
                row.ops.push(Op::Fill {
                    x: indent,
                    y: 0.0,
                    w: table_w,
                    h,
                    color: match self.setup.colors {
                        Colors::Color => kim_design::LIGHT.bg_raised,
                        Colors::Black => Rgb {
                            r: 240,
                            g: 240,
                            b: 240,
                        },
                    },
                });
            }
            row.ops.push(Op::Fill {
                x: indent,
                y: 0.0,
                w: table_w,
                h: line,
                color: border,
            });
            let mut x = indent;
            for (i, (cell, w)) in wrapped.iter().zip(&widths).enumerate() {
                row.ops.push(Op::Fill {
                    x,
                    y: 0.0,
                    w: line,
                    h,
                    color: border,
                });
                for (k, l) in cell.iter().enumerate() {
                    let lw: f32 = l.iter().map(|s| self.m.char_width(s.c, s.font)).sum();
                    let dx = match align.get(i) {
                        Some(Align::Center) => (w - pad * 2.0 - lw) / 2.0,
                        Some(Align::Right) => w - pad * 2.0 - lw,
                        _ => 0.0,
                    }
                    .max(0.0);
                    let base = pad / 2.0 + k as f32 * lh + self.m.ascent(self.body);
                    row.ops.extend(line_ops(l, x + pad + dx, base, self.m));
                }
                x += w;
            }
            row.ops.push(Op::Fill {
                x,
                y: 0.0,
                w: line,
                h,
                color: border,
            });
            self.rows.push(row);
        }
        self.rows.push(Row {
            h: self.m.height(self.body) * 0.5,
            ops: vec![Op::Fill {
                x: indent,
                y: 0.0,
                w: table_w,
                h: line,
                color: border,
            }],
        });
    }
}

/// An inline item of rendered Markdown.
#[derive(Debug, Clone)]
enum Item {
    Char(Styled),
    Image(String, String),
}

/// A quote's bars and padding.
#[derive(Debug, Clone, Copy)]
struct Deco {
    bars: u8,
    bar_w: f32,
    step: f32,
    color: Rgb,
    pad: f32,
}

impl Deco {
    const NONE: Deco = Deco {
        bars: 0,
        bar_w: 0.0,
        step: 0.0,
        color: Rgb { r: 0, g: 0, b: 0 },
        pad: 0.0,
    };
    fn quote(level: u8, setup: &Setup, m: &dyn Measure, body: Font) -> Deco {
        let step = m.char_width('M', body) * 1.2;
        Deco {
            bars: level,
            bar_w: (m.height(body) / 6.0).max(1.0),
            step,
            color: ink(setup, kim_design::LIGHT.border_strong),
            pad: f32::from(level) * step,
        }
    }
    fn draw(&self, row: &mut Row, h: f32) {
        for i in 0..self.bars {
            row.ops.push(Op::Fill {
                x: f32::from(i) * self.step,
                y: 0.0,
                w: self.bar_w,
                h,
                color: self.color,
            });
        }
    }
}

/// Rendered Markdown as pages (images from the document's folder).
pub fn markdown_pages(
    text: &str,
    doc: Option<&Path>,
    setup: &Setup,
    g: &Geometry,
    m: &dyn Measure,
    vars: &Vars,
) -> Vec<Page> {
    let a = area(setup, g, m);
    let mut md = Md {
        setup,
        m,
        doc,
        dpi: g.dpi,
        width: a.width,
        max_h: a.bottom - a.top,
        body: Font::new(Face::Sans, setup.font_pt + 1.0),
        rows: Vec::new(),
    };
    let rendered = markdown::parse(text);
    let (notes, body): (Vec<_>, Vec<_>) = rendered
        .blocks
        .iter()
        .partition(|b| matches!(b.block, Block::Footnote { .. }));
    for b in &body {
        md.block(&b.block, 0.0, 0);
    }
    if !notes.is_empty() {
        let lh = m.height(md.body);
        md.rows.push(Row {
            h: lh,
            ops: vec![Op::Fill {
                x: 0.0,
                y: lh * 0.5,
                w: a.width * 0.3,
                h: (lh / 18.0).max(1.0),
                color: ink(setup, kim_design::LIGHT.border_strong),
            }],
        });
        for b in &notes {
            md.block(&b.block, 0.0, 0);
        }
    }
    // Trailing gaps print nothing.
    while md.rows.last().is_some_and(|r| r.ops.is_empty()) {
        md.rows.pop();
    }
    paginate(md.rows, setup, g, m, vars)
}

/// Only the pages `first..=last` (1-based) of `pages`.
pub fn page_range(pages: Vec<Page>, first: usize, last: usize) -> Vec<Page> {
    let first = first.max(1);
    pages
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i + 1 >= first && *i < last)
        .map(|(_, p)| p)
        .collect()
}

/// The pages as text (one operation a line), for golden tests and the CLI's dry run.
pub fn describe(pages: &[Page]) -> String {
    let mut out = String::new();
    for (i, p) in pages.iter().enumerate() {
        out.push_str(&format!("page {}\n", i + 1));
        for op in &p.ops {
            match op {
                Op::Text {
                    x,
                    y,
                    font,
                    color,
                    text,
                    strike,
                    underline,
                } => out.push_str(&format!(
                    "  text {x:.0},{y:.0} {:?}{}{} {:.1}pt {}{}{} {text:?}\n",
                    font.face,
                    if font.bold { " bold" } else { "" },
                    if font.italic { " italic" } else { "" },
                    font.pt(),
                    export::hex(*color),
                    if *strike { " strike" } else { "" },
                    if *underline { " underline" } else { "" },
                )),
                Op::Fill { x, y, w, h, color } => out.push_str(&format!(
                    "  fill {x:.0},{y:.0} {w:.0}x{h:.0} {}\n",
                    export::hex(*color)
                )),
                Op::Image { x, y, w, h, path } => out.push_str(&format!(
                    "  image {x:.0},{y:.0} {w:.0}x{h:.0} {}\n",
                    path.file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                )),
            }
        }
    }
    out
}

/// A printer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printer {
    pub name: String,
    pub port: String,
    pub default: bool,
}

impl Printer {
    /// The printer asks for a file to write (Microsoft Print to PDF, the XPS writer): printing
    /// to it needs an output file, or the driver would ask.
    pub fn writes_a_file(&self) -> bool {
        self.port.eq_ignore_ascii_case("PORTPROMPT:")
    }
}

/// A print job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Job {
    pub printer: String,
    /// The file a file-writing printer (Microsoft Print to PDF) writes.
    pub output: Option<PathBuf>,
    /// The print dialog's settings for the printer (`DEVMODEW` bytes).
    pub devmode: Option<Vec<u8>>,
    /// The spooler's name of the job (the document's).
    pub name: String,
    /// Lay the pages out for the printer and stop: nothing is printed.
    pub dry_run: bool,
}

/// Why nothing was printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintError {
    NoPrinter(String),
    /// The printer writes a file and no output was given (it would ask).
    NeedsOutput(String),
    Failed(String),
}

impl PrintError {
    pub fn code(&self) -> (&'static str, u8) {
        match self {
            PrintError::NoPrinter(_) => ("printer_not_found", 3),
            PrintError::NeedsOutput(_) => ("usage", 2),
            PrintError::Failed(_) => ("print_failed", 1),
        }
    }
}

impl std::fmt::Display for PrintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrintError::NoPrinter(p) => write!(f, "there is no printer called `{p}`"),
            PrintError::NeedsOutput(p) => write!(
                f,
                "`{p}` writes a file; name it with an output path (it would ask otherwise)"
            ),
            PrintError::Failed(e) => write!(f, "printing failed: {e}"),
        }
    }
}

/// What a job printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed {
    pub pages: usize,
    pub output: Option<PathBuf>,
}

/// The pages of a job, laid out for the device (its measure and page).
pub type Layout<'a> = dyn FnMut(&dyn Measure, &Geometry) -> Vec<Page> + Send + 'a;

/// Where print jobs go: the Windows spooler in the program, a recorder in tests.
pub trait Printers: Send + Sync {
    fn list(&self) -> Vec<Printer>;
    /// Lays the pages out for the printer and prints them (no dialog: the window asked first).
    fn print(&self, job: &Job, layout: &mut Layout<'_>) -> Result<Printed, PrintError>;
}

/// No printers at all (a window under test that has not been given any).
#[derive(Debug, Default)]
pub struct NoPrinters;

impl Printers for NoPrinters {
    fn list(&self) -> Vec<Printer> {
        Vec::new()
    }
    fn print(&self, job: &Job, _layout: &mut Layout<'_>) -> Result<Printed, PrintError> {
        Err(PrintError::NoPrinter(job.printer.clone()))
    }
}

/// A recording printer for tests: an A4 page at 300 dpi and [`FakeMeasure`].
#[derive(Debug, Default)]
pub struct RecordingPrinters {
    pub printers: Vec<Printer>,
    pub jobs: std::sync::Mutex<Vec<(Job, Vec<Page>)>>,
}

impl Printers for RecordingPrinters {
    fn list(&self) -> Vec<Printer> {
        self.printers.clone()
    }
    fn print(&self, job: &Job, layout: &mut Layout<'_>) -> Result<Printed, PrintError> {
        let p = self
            .printers
            .iter()
            .find(|p| p.name == job.printer)
            .ok_or_else(|| PrintError::NoPrinter(job.printer.clone()))?;
        if p.writes_a_file() && job.output.is_none() {
            return Err(PrintError::NeedsOutput(p.name.clone()));
        }
        let pages = layout(&FakeMeasure { dpi: 300.0 }, &Geometry::a4(300.0));
        let n = pages.len();
        if job.dry_run {
            return Ok(Printed {
                pages: n,
                output: None,
            });
        }
        if let Some(o) = &job.output {
            std::fs::write(o, describe(&pages)).map_err(|e| PrintError::Failed(e.to_string()))?;
        }
        if let Ok(mut j) = self.jobs.lock() {
            j.push((job.clone(), pages));
        }
        Ok(Printed {
            pages: n,
            output: job.output.clone(),
        })
    }
}

/// Today's local date (year, month, day), as asset names use it.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn local_date() -> (i32, u32, u32) {
    // SAFETY: GetLocalTime fills the structure it is given.
    let t = unsafe {
        let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
        t
    };
    (i32::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay))
}

/// Today's local date and time as `yyyy-MM-dd` and `HH:mm`.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn local_now() -> (String, String) {
    // SAFETY: GetLocalTime fills the structure it is given.
    let t = unsafe {
        let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
        t
    };
    (
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay),
        format!("{:02}:{:02}", t.wHour, t.wMinute),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: FakeMeasure = FakeMeasure { dpi: 300.0 };

    fn vars() -> Vars {
        Vars {
            file: "a.rs".into(),
            path: "C:\\x\\a.rs".into(),
            date: "2026-11-22".into(),
            time: "18:00".into(),
        }
    }

    fn texts(p: &Page) -> Vec<String> {
        p.ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn templates_fill_their_three_parts() {
        assert_eq!(
            expand("{file}|p {page}/{pages}|{date} {time}", &vars(), 2, 5),
            ["a.rs", "p 2/5", "2026-11-22 18:00"]
        );
        assert_eq!(expand("only", &vars(), 1, 1), ["only", "", ""]);
    }

    #[test]
    fn wrapping_breaks_at_blanks_then_characters() {
        let f = Font::new(Face::Mono, 10.0);
        let s = |t: &str| -> Vec<Styled> {
            t.chars()
                .map(|c| Styled {
                    c,
                    font: f,
                    color: Rgb { r: 0, g: 0, b: 0 },
                    strike: false,
                    underline: false,
                })
                .collect()
        };
        let cw = M.char_width('a', f);
        let lines = |t: &str, n: f32| -> Vec<String> {
            wrap(&s(t), cw * n + 0.1, &M)
                .iter()
                .map(|l| l.iter().map(|x| x.c).collect())
                .collect()
        };
        assert_eq!(lines("aaa bbb ccc", 7.0), ["aaa bbb ", "ccc"]);
        assert_eq!(lines("abcdefghij", 4.0), ["abcd", "efgh", "ij"]);
        assert_eq!(lines("a\nb", 9.0), ["a", "b"]);
        assert_eq!(
            lines("한글한글", 4.0),
            ["한글", "한글"],
            "wide characters count twice"
        );
    }

    #[test]
    fn code_pages_number_wrap_and_paginate() {
        let text: String = (1..=200).map(|i| format!("line {i}\n")).collect();
        let setup = Setup::default();
        let pages = code_pages(&text, None, &setup, &Geometry::a4(300.0), &M, &vars());
        assert!(pages.len() >= 3, "{} pages", pages.len());
        let first = texts(&pages[0]);
        assert!(first.contains(&"a.rs".to_string()), "header: {first:?}");
        assert!(first.contains(&format!("1 / {}", pages.len())), "footer");
        assert!(first.contains(&"1".to_string()) && first.contains(&"line 1".to_string()));
        let last = texts(pages.last().unwrap());
        assert!(last.contains(&"line 200".to_string()));
        // Black and white, no numbers, no header or footer.
        let bw = Setup {
            colors: Colors::Black,
            line_numbers: false,
            header: String::new(),
            footer: String::new(),
            ..Setup::default()
        };
        let rust = crate::syntax::builtin("Rust");
        let p = code_pages("fn main() {}", rust, &bw, &Geometry::a4(300.0), &M, &vars());
        assert_eq!(texts(&p[0]), ["fn main() {}"], "one style: one run");
        let c = code_pages(
            "fn main() {}",
            rust,
            &Setup::default(),
            &Geometry::a4(300.0),
            &M,
            &vars(),
        );
        assert!(
            texts(&c[0]).contains(&"fn".to_string()),
            "colours split the runs"
        );
        assert!(p[0].ops.iter().all(|o| match o {
            Op::Text { color, .. } => *color == Rgb { r: 0, g: 0, b: 0 },
            _ => true,
        }));
        // A long line wraps; tabs become spaces.
        let long = format!("\t{}", "word ".repeat(80));
        let p = code_pages(&long, None, &bw, &Geometry::a4(300.0), &M, &vars());
        let rows: Vec<f32> = p[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(rows.len() > 2, "wrapped into rows");
        assert!(texts(&p[0])[0].starts_with("    word"));
    }

    #[test]
    fn markdown_pages_render_blocks_and_images() {
        let d = std::env::temp_dir().join(format!("jarak-print-md-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let png = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/export/assets/dot.png"),
        )
        .unwrap();
        std::fs::write(d.join("dot.png"), &png).unwrap();
        let doc = d.join("n.md");
        let text = "# Title\n\nSome **bold** text.\n\n- one\n- two\n\n| a | b |\n|---|--:|\n| 1 | 2 |\n\n![dot](dot.png) ![r](https://x.org/r.png)\n\n```rust\nfn x() {}\n```\n";
        let pages = markdown_pages(
            text,
            Some(&doc),
            &Setup::default(),
            &Geometry::a4(300.0),
            &M,
            &vars(),
        );
        let all = describe(&pages);
        assert!(
            all.contains("\"Title\"") && all.contains("Sans bold 22.0pt"),
            "{all}"
        );
        assert!(all.contains("\"bold\"") && all.contains("\"•\""));
        assert!(all.contains("image ") && all.contains("dot.png"), "{all}");
        assert!(all.contains("remote image, not loaded"));
        assert!(all.contains("\"fn\""));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_recorder_needs_an_output_for_a_file_printer() {
        let r = RecordingPrinters {
            printers: vec![Printer {
                name: PDF_PRINTER.into(),
                port: "PORTPROMPT:".into(),
                default: false,
            }],
            ..RecordingPrinters::default()
        };
        let job = Job {
            printer: PDF_PRINTER.into(),
            ..Job::default()
        };
        let mut layout = |_: &dyn Measure, _: &Geometry| vec![Page::default()];
        assert!(matches!(
            r.print(&job, &mut layout),
            Err(PrintError::NeedsOutput(_))
        ));
        let missing = Job {
            printer: "nope".into(),
            ..Job::default()
        };
        assert_eq!(
            r.print(&missing, &mut layout).unwrap_err().code(),
            ("printer_not_found", 3)
        );
        assert_eq!(
            page_range(vec![Page::default(); 5], 2, 3).len(),
            2,
            "pages 2–3"
        );
    }
}
