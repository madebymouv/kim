//! ADR 0099: Subak's UI text — the tab bar's labels and icons in the design system's UI font
//! (`kim_design::font::UI_FAMILY`: Segoe UI Variable Text or Segoe UI, Korean through Windows'
//! font linking) and Windows' icon font, measured and drawn with the same GDI fonts, so the
//! layout's numbers are the painted text's numbers. The terminal grid keeps its own renderer.

use super::tabs::{Face, Measure, Op, Px};
use std::collections::HashMap;
use subak_term::render::{Frame, Rgb};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DIB_RGB_COLORS, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FW_NORMAL, FW_SEMIBOLD, GdiFlush, GetTextFaceW,
    GetTextMetricsW, HBITMAP, HDC, HFONT, OUT_TT_PRECIS, SelectObject, SetBkMode, SetTextColor,
    TEXTMETRICW, TRANSPARENT,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// GDI text for the window's chrome: a memory DC, a DIB to draw into, the fonts by size.
pub struct UiText {
    dc: HDC,
    dib: HBITMAP,
    bits: *mut u32,
    size: (i32, i32),
    fonts: HashMap<(Face, u32, bool), HFONT>,
    faces: HashMap<Face, String>,
}

impl std::fmt::Debug for UiText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UiText({:?})", self.faces)
    }
}

#[allow(unsafe_code)]
impl UiText {
    pub fn new() -> UiText {
        // SAFETY: a memory DC compatible with the screen; deleted in `drop`.
        let dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        let mut t = UiText {
            dc,
            dib: std::ptr::null_mut(),
            bits: std::ptr::null_mut(),
            size: (0, 0),
            fonts: HashMap::new(),
            faces: HashMap::new(),
        };
        let ui = t.first_installed(&kim_design::font::UI_FAMILY[..2]);
        t.faces.insert(Face::Ui, ui);
        let icon = t.first_installed(&["Segoe Fluent Icons", "Segoe MDL2 Assets"]);
        t.faces.insert(Face::Icon, icon);
        t.faces.insert(Face::Symbol, "Segoe UI Symbol".into());
        t
    }

    /// The face each role resolved to (the geometry report records it).
    pub fn face(&self, f: Face) -> &str {
        self.faces.get(&f).map_or("", String::as_str)
    }

    /// The first of `names` GDI really selects (a missing face falls back silently otherwise).
    fn first_installed(&mut self, names: &[&str]) -> String {
        for n in names {
            let f = Self::create(n, 16, false);
            let mut buf = [0u16; 64];
            // SAFETY: our DC and a font we created and delete here; the buffer's length.
            let got = unsafe {
                let prev = SelectObject(self.dc, f);
                let k = GetTextFaceW(self.dc, buf.len() as i32, buf.as_mut_ptr());
                SelectObject(self.dc, prev);
                DeleteObject(f);
                String::from_utf16_lossy(&buf[..usize::try_from(k.max(1) - 1).unwrap_or(0)])
            };
            if got.eq_ignore_ascii_case(n) {
                return (*n).to_string();
            }
        }
        names.last().map_or_else(String::new, |n| (*n).to_string())
    }

    fn create(face: &str, px: u32, semibold: bool) -> HFONT {
        let name = wide(face);
        // SAFETY: plain values and a NUL-terminated name; the caller deletes the font.
        unsafe {
            CreateFontW(
                -i32::try_from(px.clamp(4, 512)).unwrap_or(12),
                0,
                0,
                0,
                if semibold { FW_SEMIBOLD } else { FW_NORMAL } as i32,
                0,
                0,
                0,
                u32::from(DEFAULT_CHARSET),
                u32::from(OUT_TT_PRECIS),
                u32::from(CLIP_DEFAULT_PRECIS),
                u32::from(CLEARTYPE_QUALITY),
                u32::from(DEFAULT_PITCH),
                name.as_ptr(),
            )
        }
    }

    fn font(&mut self, face: Face, px: u32, semibold: bool) -> HFONT {
        if let Some(f) = self.fonts.get(&(face, px, semibold)) {
            return *f;
        }
        let f = Self::create(self.face(face).to_string().as_str(), px, semibold);
        self.fonts.insert((face, px, semibold), f);
        f
    }

    /// The DIB at least `w` × `h`.
    fn ensure(&mut self, w: i32, h: i32) -> bool {
        if self.dc.is_null() {
            return false;
        }
        if self.size.0 >= w && self.size.1 >= h && !self.dib.is_null() {
            return true;
        }
        let (w, h) = (w.max(self.size.0).max(1), h.max(self.size.1).max(1));
        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                // SAFETY: plain data; all-zero is a valid BITMAPINFOHEADER remainder.
                ..unsafe { std::mem::zeroed() }
            },
            // SAFETY: plain data; an all-zero colour table is valid for 32-bit BI_RGB.
            ..unsafe { std::mem::zeroed() }
        };
        let mut bits = std::ptr::null_mut();
        // SAFETY: a top-down 32-bit DIB of `w` × `h`; the old one is deleted after the new one is
        // selected.
        unsafe {
            let dib = CreateDIBSection(
                self.dc,
                &raw const bi,
                DIB_RGB_COLORS,
                &raw mut bits,
                std::ptr::null_mut(),
                0,
            );
            if dib.is_null() || bits.is_null() {
                return false;
            }
            SelectObject(self.dc, dib);
            if !self.dib.is_null() {
                DeleteObject(self.dib);
            }
            self.dib = dib;
        }
        self.bits = bits.cast();
        self.size = (w, h);
        true
    }

    /// Draws `ops` into `frame` (a frame of the bar's size).
    pub fn paint(&mut self, frame: &mut Frame, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Fill {
                    rect,
                    color,
                    radius,
                } => fill_round_top(frame, *rect, *radius, *color),
                Op::Text {
                    rect,
                    text,
                    px,
                    semibold,
                    face,
                    color,
                    center,
                } => self.text(frame, *rect, text, *px, *semibold, *face, *color, *center),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        frame: &mut Frame,
        r: Px,
        text: &str,
        px: u32,
        semibold: bool,
        face: Face,
        color: Rgb,
        center: bool,
    ) {
        // The rectangle within the frame.
        let x0 = r.x.max(0);
        let y0 = r.y.max(0);
        let x1 = r.right().min(frame.width as i32);
        let y1 = r.bottom().min(frame.height as i32);
        if text.is_empty() || x1 <= x0 || y1 <= y0 || !self.ensure(x1 - x0, y1 - y0) {
            return;
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let stride = self.size.0 as usize;
        // What lies under the text, so ClearType blends onto the real background.
        for y in 0..h as usize {
            let src = (y0 as usize + y) * frame.width as usize + x0 as usize;
            // SAFETY: `ensure` made the DIB at least `w` × `h`; rows are `stride` pixels.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    frame.pixels.as_ptr().add(src),
                    self.bits.add(y * stride),
                    w as usize,
                );
            }
        }
        let f = self.font(face, px, semibold);
        let s: Vec<u16> = text.encode_utf16().collect();
        let mut rc = RECT {
            left: r.x - x0,
            top: r.y - y0,
            right: r.right() - x0,
            bottom: r.bottom() - y0,
        };
        let flags = DT_SINGLELINE
            | DT_VCENTER
            | DT_NOPREFIX
            | DT_END_ELLIPSIS
            | if center { DT_CENTER } else { DT_LEFT };
        // SAFETY: our DC with its DIB and font; the text and its length.
        unsafe {
            let prev = SelectObject(self.dc, f);
            SetBkMode(self.dc, TRANSPARENT as i32);
            SetTextColor(
                self.dc,
                u32::from(color.0) | u32::from(color.1) << 8 | u32::from(color.2) << 16,
            );
            DrawTextW(self.dc, s.as_ptr(), s.len() as i32, &raw mut rc, flags);
            SelectObject(self.dc, prev);
            GdiFlush();
        }
        for y in 0..h as usize {
            let dst = (y0 as usize + y) * frame.width as usize + x0 as usize;
            for x in 0..w as usize {
                // SAFETY: within the DIB (see above).
                let p = unsafe { *self.bits.add(y * stride + x) } & 0x00FF_FFFF;
                frame.pixels[dst + x] = p;
            }
        }
    }
}

#[allow(unsafe_code)]
impl Measure for UiText {
    fn width(&mut self, text: &str, px: u32, semibold: bool) -> u32 {
        if text.is_empty() || self.dc.is_null() {
            return 0;
        }
        let f = self.font(Face::Ui, px, semibold);
        let s: Vec<u16> = text.encode_utf16().collect();
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: our DC and font; DT_CALCRECT only measures (with font linking, like drawing).
        unsafe {
            let prev = SelectObject(self.dc, f);
            DrawTextW(
                self.dc,
                s.as_ptr(),
                s.len() as i32,
                &raw mut rc,
                DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(self.dc, prev);
        }
        (rc.right - rc.left).max(0) as u32
    }

    fn line(&mut self, px: u32) -> u32 {
        if self.dc.is_null() {
            return (px as f32 * 1.33).round() as u32;
        }
        let f = self.font(Face::Ui, px, false);
        // SAFETY: our DC and font; a correctly sized out-structure.
        let tm = unsafe {
            let mut tm: TEXTMETRICW = std::mem::zeroed();
            let prev = SelectObject(self.dc, f);
            GetTextMetricsW(self.dc, &raw mut tm);
            SelectObject(self.dc, prev);
            tm
        };
        tm.tmHeight.max(1) as u32
    }
}

#[allow(unsafe_code)]
impl Drop for UiText {
    fn drop(&mut self) {
        // SAFETY: objects this value created.
        unsafe {
            for f in self.fonts.values() {
                DeleteObject(*f);
            }
            if !self.dib.is_null() {
                DeleteObject(self.dib);
            }
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
        }
    }
}

/// Fills `r` with `c`, its top corners rounded by `radius` (anti-aliased, 4 × 4 samples).
pub fn fill_round_top(frame: &mut Frame, r: Px, radius: u32, c: Rgb) {
    let x0 = r.x.max(0);
    let y0 = r.y.max(0);
    let x1 = r.right().min(frame.width as i32);
    let y1 = r.bottom().min(frame.height as i32);
    let rad = radius as f32;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y as u32 * frame.width + x as u32) as usize;
            let cov = if radius == 0 || y >= r.y + radius as i32 {
                1.0
            } else {
                // Distance to the nearer top corner's centre, sampled inside the pixel.
                let cx = if x < r.x + radius as i32 {
                    r.x as f32 + rad
                } else if x >= r.right() - radius as i32 {
                    r.right() as f32 - rad
                } else {
                    f32::NAN
                };
                if cx.is_nan() {
                    1.0
                } else {
                    let cy = r.y as f32 + rad;
                    let mut n = 0;
                    for sy in 0..4 {
                        for sx in 0..4 {
                            let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                            let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                            let inside_band = if cx < r.x as f32 + rad + 0.01 {
                                px >= cx
                            } else {
                                px <= cx
                            };
                            if py >= cy
                                || inside_band
                                || (px - cx).powi(2) + (py - cy).powi(2) <= rad * rad
                            {
                                n += 1;
                            }
                        }
                    }
                    n as f32 / 16.0
                }
            };
            if cov >= 1.0 {
                frame.pixels[i] = c.to_u32();
            } else if cov > 0.0 {
                let under = Rgb::from_u32(frame.pixels[i]);
                let m = |a: u8, b: u8| {
                    (f32::from(a) + (f32::from(b) - f32::from(a)) * cov).round() as u8
                };
                frame.pixels[i] = Rgb(m(under.0, c.0), m(under.1, c.1), m(under.2, c.2)).to_u32();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_corners_blend_and_the_body_is_solid() {
        let mut f = Frame::new(40, 20, Rgb(0, 0, 0));
        fill_round_top(&mut f, Px::new(5, 2, 30, 18), 6, Rgb(200, 200, 200));
        assert_eq!(
            f.pixel(20, 2),
            Rgb(200, 200, 200),
            "top edge between the corners"
        );
        assert_eq!(f.pixel(5, 19), Rgb(200, 200, 200), "below the corners");
        assert_eq!(f.pixel(5, 2), Rgb(0, 0, 0), "outside the rounded corner");
        let edge = f.pixel(6, 4);
        assert!(edge.0 > 0 && edge.0 < 200, "anti-aliased: {edge:?}");
        assert_eq!(
            f.pixel(4, 10),
            Rgb(0, 0, 0),
            "nothing outside the rectangle"
        );
    }

    #[test]
    fn gdi_measures_the_ui_font_and_draws_text() {
        let mut t = UiText::new();
        assert!(
            t.face(Face::Ui).starts_with("Segoe UI"),
            "{}",
            t.face(Face::Ui)
        );
        let w12 = t.width("Terminal", 12, false);
        let w24 = t.width("Terminal", 24, false);
        assert!(w12 > 20 && w12 < 80, "{w12}");
        assert!(w24 >= 2 * w12 - 2, "{w24} vs {w12}");
        assert!(
            t.width("Terminal", 12, true) >= w12,
            "semibold is not narrower"
        );
        let line = t.line(12);
        assert!((14..=18).contains(&line), "12 px Segoe UI line {line}");
        assert!(
            t.width("로그", 12, false) > 10,
            "Korean through font linking"
        );
        let mut f = Frame::new(120, 32, Rgb(0x15, 0x15, 0x1B));
        t.paint(
            &mut f,
            &[Op::Text {
                rect: Px::new(4, 0, 112, 32),
                text: "main".into(),
                px: 12,
                semibold: false,
                face: Face::Ui,
                color: Rgb(0xE8, 0xE6, 0xF0),
                center: false,
            }],
        );
        let lit = f.pixels.iter().filter(|p| (**p & 0xFF) > 0x80).count();
        assert!(lit > 10, "text pixels drawn: {lit}");
    }
}
