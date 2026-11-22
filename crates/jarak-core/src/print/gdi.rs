//! The Windows printer backend (Step 106): the spooler's printers (`EnumPrintersW`), and a job
//! drawn with GDI on a printer device context (`CreateDCW` with the print dialog's `DEVMODEW`,
//! `StartDocW` with an output file for a file-writing printer such as Microsoft Print to PDF, so
//! no dialog ever appears; `JARAK-FR-140`, `KIM-FR-012`).
//!
//! Fonts: Consolas for text and code, Segoe UI for rendered Markdown, and Malgun Gothic for any
//! character those lack (Korean): each character is measured with the font that draws it, so the
//! layout and the printed page agree.

#![allow(unsafe_code)]

use super::{Face, Font, Geometry, Job, Measure, Op, Page, PrintError, Printed, Printer, Printers};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, CreateFontIndirectW, CreateSolidBrush,
    DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject, FW_BOLD, FW_NORMAL, FillRect,
    GGI_MARK_NONEXISTING_GLYPHS, GetDeviceCaps, GetGlyphIndicesW, GetTextExtentPoint32W,
    GetTextMetricsW, HALFTONE, HDC, HFONT, HGDIOBJ, LOGFONTW, LOGPIXELSX, OUT_TT_PRECIS,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, PROOF_QUALITY, SRCCOPY,
    SelectObject, SetBkMode, SetBrushOrgEx, SetStretchBltMode, SetTextAlign, SetTextColor,
    StretchDIBits, TA_BASELINE, TA_LEFT, TEXTMETRICW, TRANSPARENT, TextOutW,
};
use windows_sys::Win32::Graphics::Printing::{
    EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
    PRINTER_INFO_2W,
};
use windows_sys::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};

/// The Windows spooler.
#[derive(Debug, Default)]
pub struct WinPrinters;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A NUL-terminated wide string.
///
/// # Safety
/// `p` is null or points to a NUL-terminated UTF-16 string.
unsafe fn from_wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: the caller promises a NUL-terminated string.
    unsafe {
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
}

/// The default printer's name.
pub fn default_printer() -> Option<String> {
    let mut n = 0u32;
    // SAFETY: a null buffer asks for the size.
    unsafe { GetDefaultPrinterW(std::ptr::null_mut(), &mut n) };
    if n == 0 {
        return None;
    }
    let mut buf = vec![0u16; n as usize];
    // SAFETY: `buf` holds `n` characters.
    if unsafe { GetDefaultPrinterW(buf.as_mut_ptr(), &mut n) } == 0 {
        return None;
    }
    // SAFETY: the API wrote a NUL-terminated name.
    Some(unsafe { from_wide(buf.as_ptr()) })
}

/// The local and connected printers.
pub fn printers() -> Vec<Printer> {
    let default = default_printer();
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let (mut needed, mut count) = (0u32, 0u32);
    // SAFETY: a null buffer asks for the size.
    unsafe {
        EnumPrintersW(
            flags,
            std::ptr::null(),
            2,
            std::ptr::null_mut(),
            0,
            &mut needed,
            &mut count,
        )
    };
    if needed == 0 {
        return Vec::new();
    }
    // Eight-byte aligned, as the structures in it need.
    let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: `buf` holds `needed` bytes.
    let ok = unsafe {
        EnumPrintersW(
            flags,
            std::ptr::null(),
            2,
            buf.as_mut_ptr().cast::<u8>(),
            needed,
            &mut needed,
            &mut count,
        )
    };
    if ok == 0 {
        return Vec::new();
    }
    // SAFETY: the API wrote `count` PRINTER_INFO_2W structures at the start of `buf`, their
    // strings after them in the same buffer.
    let infos = unsafe {
        std::slice::from_raw_parts(buf.as_ptr().cast::<PRINTER_INFO_2W>(), count as usize)
    };
    infos
        .iter()
        .map(|i| {
            // SAFETY: the strings are NUL-terminated inside `buf`.
            let (name, port) = unsafe { (from_wide(i.pPrinterName), from_wide(i.pPortName)) };
            Printer {
                default: default.as_deref() == Some(name.as_str()),
                name,
                port,
            }
        })
        .collect()
}

/// A printer device context, deleted when dropped.
struct Dc(HDC);

impl Drop for Dc {
    fn drop(&mut self) {
        // SAFETY: the context came from CreateDCW and is deleted once.
        unsafe { DeleteDC(self.0) };
    }
}

/// The device's fonts and their metrics.
struct Fonts {
    dc: HDC,
    dpi: f32,
    fonts: RefCell<HashMap<(Font, bool), HFONT>>,
    widths: RefCell<HashMap<(char, Font), (f32, bool)>>,
    metrics: RefCell<HashMap<Font, (f32, f32)>>,
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for (_, f) in self.fonts.borrow_mut().drain() {
            // SAFETY: each font came from CreateFontIndirectW and is no longer selected (the DC
            // is about to be deleted; Windows ignores a still-selected font's deletion).
            unsafe { DeleteObject(f as HGDIOBJ) };
        }
    }
}

impl Fonts {
    /// The GDI font for `f` (`fallback`: the one for Korean and other characters the face lacks).
    fn handle(&self, f: Font, fallback: bool) -> HFONT {
        if let Some(h) = self.fonts.borrow().get(&(f, fallback)) {
            return *h;
        }
        let face = match (fallback, f.face) {
            (true, _) => "Malgun Gothic",
            (false, Face::Mono) => "Consolas",
            (false, Face::Sans) => "Segoe UI",
        };
        let mut lf = LOGFONTW {
            lfHeight: -(f.pt() * self.dpi / 72.0).round() as i32,
            lfWeight: if f.bold { FW_BOLD } else { FW_NORMAL } as i32,
            lfItalic: u8::from(f.italic),
            lfCharSet: DEFAULT_CHARSET,
            lfOutPrecision: OUT_TT_PRECIS,
            lfQuality: PROOF_QUALITY,
            ..LOGFONTW::default()
        };
        for (i, u) in face.encode_utf16().take(31).enumerate() {
            lf.lfFaceName[i] = u;
        }
        // SAFETY: a filled LOGFONTW.
        let h = unsafe { CreateFontIndirectW(&lf) };
        self.fonts.borrow_mut().insert((f, fallback), h);
        h
    }

    fn select(&self, f: Font, fallback: bool) {
        // SAFETY: a font of this DC's fonts.
        unsafe { SelectObject(self.dc, self.handle(f, fallback) as HGDIOBJ) };
    }

    /// The width of `c` and whether the fallback font draws it.
    fn char_info(&self, c: char, f: Font) -> (f32, bool) {
        if let Some(v) = self.widths.borrow().get(&(c, f)) {
            return *v;
        }
        let mut units = [0u16; 2];
        let units = c.encode_utf16(&mut units);
        let mut fallback = units.len() > 1;
        if !fallback {
            self.select(f, false);
            let mut glyph = [0u16; 1];
            // SAFETY: one character in, one glyph index out.
            unsafe {
                GetGlyphIndicesW(
                    self.dc,
                    units.as_ptr(),
                    1,
                    glyph.as_mut_ptr(),
                    GGI_MARK_NONEXISTING_GLYPHS,
                )
            };
            fallback = glyph[0] == 0xFFFF;
        }
        self.select(f, fallback);
        let mut size = SIZE { cx: 0, cy: 0 };
        // SAFETY: the character's units and a SIZE to fill.
        unsafe { GetTextExtentPoint32W(self.dc, units.as_ptr(), units.len() as i32, &mut size) };
        let v = (size.cx as f32, fallback);
        self.widths.borrow_mut().insert((c, f), v);
        v
    }

    /// Ascent and line height of `f`.
    fn metrics(&self, f: Font) -> (f32, f32) {
        if let Some(v) = self.metrics.borrow().get(&f) {
            return *v;
        }
        self.select(f, false);
        // SAFETY: TEXTMETRICW is plain data, all-zero valid, filled by the call.
        let tm: TEXTMETRICW = unsafe {
            let mut tm: TEXTMETRICW = std::mem::zeroed();
            GetTextMetricsW(self.dc, &mut tm);
            tm
        };
        let height = (tm.tmHeight + tm.tmExternalLeading) as f32 * 1.12;
        let ascent = tm.tmAscent as f32 + (height - tm.tmHeight as f32) / 2.0;
        let v = (ascent, height.max(1.0));
        self.metrics.borrow_mut().insert(f, v);
        v
    }
}

impl Measure for Fonts {
    fn char_width(&self, c: char, f: Font) -> f32 {
        self.char_info(c, f).0
    }
    fn ascent(&self, f: Font) -> f32 {
        self.metrics(f).0
    }
    fn height(&self, f: Font) -> f32 {
        self.metrics(f).1
    }
}

fn colorref(c: kim_design::Rgb) -> u32 {
    u32::from(c.r) | (u32::from(c.g) << 8) | (u32::from(c.b) << 16)
}

fn fill(dc: HDC, x: f32, y: f32, w: f32, h: f32, c: kim_design::Rgb) {
    let r = RECT {
        left: x.round() as i32,
        top: y.round() as i32,
        right: (x + w).round().max(x.round() + 1.0) as i32,
        bottom: (y + h).round().max(y.round() + 1.0) as i32,
    };
    // SAFETY: a brush created, used and deleted here.
    unsafe {
        let b = CreateSolidBrush(colorref(c));
        FillRect(dc, &r, b);
        DeleteObject(b as HGDIOBJ);
    }
}

/// Draws an image file scaled into the rectangle (alpha over white).
fn image(dc: HDC, path: &Path, x: f32, y: f32, w: f32, h: f32) -> bool {
    let Ok((iw, ih, rgba)) = super::decode_image(path) else {
        return false;
    };
    let mut bgra = Vec::with_capacity(rgba.len());
    for [r, g, b, a] in rgba.as_chunks::<4>().0 {
        let a = u32::from(*a);
        let over = |c: u8| ((u32::from(c) * a + 255 * (255 - a)) / 255) as u8;
        bgra.extend_from_slice(&[over(*b), over(*g), over(*r), 0]);
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: iw as i32,
            biHeight: -(ih as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    // SAFETY: `bgra` holds iw × ih top-down 32-bit pixels, as `bmi` describes.
    unsafe {
        SetStretchBltMode(dc, HALFTONE);
        SetBrushOrgEx(dc, 0, 0, std::ptr::null_mut());
        StretchDIBits(
            dc,
            x.round() as i32,
            y.round() as i32,
            w.round() as i32,
            h.round() as i32,
            0,
            0,
            iw as i32,
            ih as i32,
            bgra.as_ptr().cast(),
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        ) > 0
    }
}

/// Draws one page; the device's origin is the printable area's corner (`off`).
fn draw(fonts: &Fonts, page: &Page, off: (f32, f32)) {
    let dc = fonts.dc;
    // SAFETY: plain state changes of this DC.
    unsafe {
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextAlign(dc, TA_BASELINE | TA_LEFT);
    }
    for op in &page.ops {
        match op {
            Op::Fill { x, y, w, h, color } => fill(dc, x - off.0, y - off.1, *w, *h, *color),
            Op::Image { x, y, w, h, path } => {
                if !image(dc, path, x - off.0, y - off.1, *w, *h) {
                    fill(
                        dc,
                        x - off.0,
                        y - off.1,
                        *w,
                        1.0,
                        kim_design::LIGHT.text_muted,
                    );
                }
            }
            Op::Text {
                x,
                y,
                font,
                color,
                text,
                strike,
                underline,
            } => {
                // SAFETY: plain state change.
                unsafe { SetTextColor(dc, colorref(*color)) };
                let mut cx = x - off.0;
                let by = y - off.1;
                let start = cx;
                // Runs of characters one font draws.
                let mut run: Vec<u16> = Vec::new();
                let mut run_fb = false;
                let mut run_x = cx;
                let flush = |run: &mut Vec<u16>, fb: bool, at: f32| {
                    if run.is_empty() {
                        return;
                    }
                    fonts.select(*font, fb);
                    // SAFETY: the run's UTF-16 units.
                    unsafe {
                        TextOutW(
                            dc,
                            at.round() as i32,
                            by.round() as i32,
                            run.as_ptr(),
                            run.len() as i32,
                        )
                    };
                    run.clear();
                };
                for c in text.chars() {
                    let (w, fb) = fonts.char_info(c, *font);
                    if fb != run_fb && !run.is_empty() {
                        flush(&mut run, run_fb, run_x);
                        run_x = cx;
                    }
                    if run.is_empty() {
                        run_x = cx;
                    }
                    run_fb = fb;
                    let mut u = [0u16; 2];
                    run.extend_from_slice(c.encode_utf16(&mut u));
                    cx += w;
                }
                flush(&mut run, run_fb, run_x);
                let em = font.pt() * fonts.dpi / 72.0;
                let thick = (em / 16.0).max(1.0);
                if *underline {
                    fill(dc, start, by + em * 0.12, cx - start, thick, *color);
                }
                if *strike {
                    fill(dc, start, by - em * 0.3, cx - start, thick, *color);
                }
            }
        }
    }
}

/// Waits until the printer has written `path` completely (it exists, has bytes, and nobody holds
/// it open any more).
fn wait_for_file(path: &Path, limit: Duration) -> Result<(), PrintError> {
    use std::os::windows::fs::OpenOptionsExt;
    let start = Instant::now();
    let mut last = None;
    while start.elapsed() < limit {
        if let Ok(m) = std::fs::metadata(path) {
            let len = m.len();
            let free = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(path)
                .is_ok();
            if len > 0 && free && last == Some(len) {
                return Ok(());
            }
            last = Some(len);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(PrintError::Failed(format!(
        "{} was not written within {} s (see the printer's queue)",
        path.display(),
        limit.as_secs()
    )))
}

impl Printers for WinPrinters {
    fn list(&self) -> Vec<Printer> {
        printers()
    }

    fn print(&self, job: &Job, layout: &mut super::Layout<'_>) -> Result<Printed, PrintError> {
        let all = printers();
        let p = all
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&job.printer))
            .ok_or_else(|| PrintError::NoPrinter(job.printer.clone()))?;
        if p.writes_a_file() && job.output.is_none() {
            return Err(PrintError::NeedsOutput(p.name.clone()));
        }
        let name = wide(&p.name);
        let driver = wide("WINSPOOL");
        // The dialog's DEVMODEW, copied into aligned memory.
        let devmode: Option<Vec<u64>> = job.devmode.as_ref().map(|b| {
            let mut v = vec![0u64; b.len().div_ceil(8)];
            // SAFETY: `v` holds at least `b.len()` bytes.
            unsafe {
                std::ptr::copy_nonoverlapping(b.as_ptr(), v.as_mut_ptr().cast::<u8>(), b.len());
            }
            v
        });
        let dm = devmode
            .as_ref()
            .map_or(std::ptr::null(), |v| v.as_ptr().cast());
        // SAFETY: NUL-terminated names; the DEVMODEW (if any) lives across the call.
        let dc = unsafe { CreateDCW(driver.as_ptr(), name.as_ptr(), std::ptr::null(), dm) };
        if dc.is_null() {
            return Err(PrintError::Failed(format!(
                "{} could not be opened",
                p.name
            )));
        }
        let dc = Dc(dc);
        // SAFETY: plain queries of the printer's context.
        let (dpi, w, h, ox, oy) = unsafe {
            (
                GetDeviceCaps(dc.0, LOGPIXELSX as i32) as f32,
                GetDeviceCaps(dc.0, PHYSICALWIDTH as i32) as f32,
                GetDeviceCaps(dc.0, PHYSICALHEIGHT as i32) as f32,
                GetDeviceCaps(dc.0, PHYSICALOFFSETX as i32) as f32,
                GetDeviceCaps(dc.0, PHYSICALOFFSETY as i32) as f32,
            )
        };
        if dpi <= 0.0 || w <= 0.0 || h <= 0.0 {
            return Err(PrintError::Failed(format!("{} reports no page", p.name)));
        }
        let fonts = Fonts {
            dc: dc.0,
            dpi,
            fonts: RefCell::default(),
            widths: RefCell::default(),
            metrics: RefCell::default(),
        };
        let pages = layout(
            &fonts,
            &Geometry {
                width: w,
                height: h,
                dpi,
            },
        );
        if job.dry_run {
            return Ok(Printed {
                pages: pages.len(),
                output: None,
            });
        }
        let doc = wide(&job.name);
        let out = job.output.as_ref().map(|o| wide(&o.to_string_lossy()));
        let di = DOCINFOW {
            cbSize: size_of::<DOCINFOW>() as i32,
            lpszDocName: doc.as_ptr(),
            lpszOutput: out.as_ref().map_or(std::ptr::null(), |o| o.as_ptr()),
            lpszDatatype: std::ptr::null(),
            fwType: 0,
        };
        // SAFETY: `di` and its strings live across the call.
        if unsafe { StartDocW(dc.0, &di) } <= 0 {
            return Err(PrintError::Failed(format!(
                "the job could not start: {}",
                std::io::Error::last_os_error()
            )));
        }
        for page in &pages {
            // SAFETY: a started document on this DC.
            if unsafe { StartPage(dc.0) } <= 0 {
                // SAFETY: the document started on this DC.
                unsafe { AbortDoc(dc.0) };
                return Err(PrintError::Failed("a page could not start".into()));
            }
            draw(&fonts, page, (ox, oy));
            // SAFETY: the page started above.
            if unsafe { EndPage(dc.0) } <= 0 {
                // SAFETY: the document started on this DC.
                unsafe { AbortDoc(dc.0) };
                return Err(PrintError::Failed("a page could not be finished".into()));
            }
        }
        // SAFETY: the document started above.
        if unsafe { EndDoc(dc.0) } <= 0 {
            return Err(PrintError::Failed("the job could not be finished".into()));
        }
        drop(fonts);
        drop(dc);
        if let Some(o) = &job.output {
            wait_for_file(o, Duration::from_secs(60))?;
        }
        Ok(Printed {
            pages: pages.len(),
            output: job.output.clone(),
        })
    }
}
