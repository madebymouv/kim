//! The one scale model of every Kim program (ADR 0099, DESIGN_SYSTEM §2.7).
//!
//! Four factors, each applied exactly once:
//!
//! 1. **DPI scale** — Windows' display scaling of the monitor (`dpi / 96`). Logical pixels times
//!    this are physical pixels. Kim never changes it.
//! 2. **UI zoom** — Kim's own enlargement for a large screen, the same for every program: the
//!    user's choice in `%APPDATA%\mouv\ui.json` ([`crate::prefs`]) or [`auto_zoom`] of the
//!    monitor's work area. egui windows apply it as egui's zoom factor; Win32 chrome multiplies
//!    its token sizes by `dpi × zoom` ([`Scale::factor`]).
//! 3. **Terminal text size** — Subak's `appearance.font_size`, the user's own; when unset it is
//!    [`default_terminal_px`] of the UI zoom, so chrome and text start in proportion.
//! 4. **Chrome cap** — Subak's tab bar never grows larger than the terminal text it frames:
//!    [`chrome_zoom`] is the UI zoom, but no more than the text's own enlargement over
//!    [`crate::font::TERMINAL`] (a user who chose small text gets a small bar, never a bar that
//!    dwarfs the text; a large default text gets a matching bar).
//!
//! The functions are pure so every program, test and the UI audit compute the same numbers.

use crate::font;

/// Windows' reference DPI (100 %).
pub const BASE_DPI: u32 = 96;
/// Zoom steps.
pub const ZOOM_STEP: f32 = 0.25;
pub const ZOOM_MIN: f32 = 1.0;
pub const ZOOM_MAX: f32 = 2.0;
/// The logical work-area height a 100 % layout is designed for (a 1080p screen's work area is
/// about 1040; anything up to 1124 stays at 100 %).
pub const REFERENCE_HEIGHT: f32 = 900.0;
/// The logical work-area width below which no zoom is applied (narrow and portrait screens).
pub const REFERENCE_WIDTH: f32 = 1200.0;
/// The percentages a user can choose.
pub const CHOICES: [u16; 5] = [100, 125, 150, 175, 200];

/// The DPI scale of `dpi` (96 → 1.0, 144 → 1.5).
pub fn dpi_scale(dpi: u32) -> f32 {
    dpi.max(1) as f32 / BASE_DPI as f32
}

/// `z` down to a whole zoom step, within the allowed range.
pub fn quantize(z: f32) -> f32 {
    if !z.is_finite() {
        return ZOOM_MIN;
    }
    ((z / ZOOM_STEP + 1e-4).floor() * ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX)
}

/// The automatic UI zoom for a monitor whose work area is `w` × `h` **logical** pixels (physical
/// pixels divided by the DPI scale): the height over [`REFERENCE_HEIGHT`], limited by the width
/// over [`REFERENCE_WIDTH`], in steps of 25 % between 100 % and 200 %.
///
/// 5120 × 1410 at 100 % → 150 %; 2560 × 1400 → 150 %; 1920 × 1200 → 125 %; 1920 × 1040 →
/// 100 %; 3840 × 2100 at 100 % → 200 %; 3840 × 2160 at 200 % (1920 × 1040 logical) → 100 %.
pub fn auto_zoom(w: f32, h: f32) -> f32 {
    quantize((h / REFERENCE_HEIGHT).min(w / REFERENCE_WIDTH))
}

/// The UI zoom a program uses: the user's choice in percent (any value is rounded to a step and
/// kept in range) or [`auto_zoom`].
pub fn ui_zoom(choice: Option<u16>, work_w: f32, work_h: f32) -> f32 {
    choice.map_or_else(
        || auto_zoom(work_w, work_h),
        |p| quantize(f32::from(p) / 100.0 + ZOOM_STEP / 2.0 - 1e-3),
    )
}

/// The environment variable an audit sets to open a window at a fixed UI zoom (`1.25`).
pub const ENV_ZOOM: &str = "KIM_UI_ZOOM";

/// The UI zoom of a window that opens now: `KIM_UI_ZOOM` (the UI audit's fixtures), else the
/// user's choice in `ui.json`, else [`auto_zoom`] of the primary monitor's work area.
pub fn startup_zoom() -> f32 {
    if let Some(z) = std::env::var(ENV_ZOOM)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        return quantize(z);
    }
    let (w, h) = crate::system::work_area_logical().unwrap_or((1920.0, 1040.0));
    ui_zoom(crate::prefs::UiPrefs::load().zoom, w, h)
}

/// The terminal text size (logical pixels) when the user has not chosen one.
pub fn default_terminal_px(zoom: f32) -> f32 {
    (font::TERMINAL * zoom).round()
}

/// The zoom of chrome that frames terminal text of `text_px` logical pixels (Subak's tab bar):
/// the UI zoom, but never more than the text's own enlargement over the default size, in whole
/// steps and at least 100 %.
pub fn chrome_zoom(ui_zoom: f32, text_px: f32) -> f32 {
    let content = quantize(text_px / font::TERMINAL);
    quantize(ui_zoom).min(content)
}

/// A resolved scale: the monitor's DPI scale and the zoom that applies to one surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub dpi: f32,
    pub zoom: f32,
}

impl Scale {
    pub const fn new(dpi: f32, zoom: f32) -> Scale {
        Scale { dpi, zoom }
    }

    /// Physical pixels per logical pixel of a token (`dpi × zoom`).
    pub fn factor(self) -> f32 {
        self.dpi * self.zoom
    }

    /// A token's size in whole physical pixels (a hairline stays at least one pixel).
    pub fn px(self, logical: f32) -> i32 {
        crate::px(logical, self.factor())
    }

    /// [`Scale::px`] as an unsigned size.
    pub fn upx(self, logical: f32) -> u32 {
        self.px(logical).max(0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_zoom_follows_the_work_area_height_in_steps() {
        assert_eq!(auto_zoom(5120.0, 1410.0), 1.5, "this PC's 32:9 screen");
        assert_eq!(auto_zoom(2560.0, 1400.0), 1.5);
        assert_eq!(auto_zoom(1920.0, 1160.0), 1.25);
        assert_eq!(auto_zoom(1920.0, 1040.0), 1.0, "1080p");
        assert_eq!(auto_zoom(1366.0, 728.0), 1.0, "never below 100 %");
        assert_eq!(auto_zoom(3840.0, 2100.0), 2.0, "4K at 100 %");
        assert_eq!(
            auto_zoom(1920.0, 1040.0),
            1.0,
            "4K at 200 % is 1920 × 1040 logical"
        );
        assert_eq!(
            auto_zoom(1080.0, 1880.0),
            1.0,
            "portrait: the width limits it"
        );
        assert_eq!(auto_zoom(f32::NAN, 1000.0), 1.0);
    }

    #[test]
    fn a_choice_wins_and_is_kept_to_the_steps() {
        assert_eq!(ui_zoom(Some(150), 1920.0, 1040.0), 1.5);
        assert_eq!(ui_zoom(Some(130), 1920.0, 1040.0), 1.25);
        assert_eq!(
            ui_zoom(Some(140), 1920.0, 1040.0),
            1.5,
            "rounded to the nearest step"
        );
        assert_eq!(ui_zoom(Some(300), 1920.0, 1040.0), 2.0);
        assert_eq!(ui_zoom(Some(50), 5120.0, 1410.0), 1.0);
        assert_eq!(ui_zoom(None, 5120.0, 1410.0), 1.5);
        for c in CHOICES {
            assert_eq!(ui_zoom(Some(c), 0.0, 0.0), f32::from(c) / 100.0);
        }
    }

    #[test]
    fn chrome_never_outgrows_the_text_it_frames() {
        // The default text of a 150 % screen (20 px) gets a 150 % bar.
        assert_eq!(default_terminal_px(1.5), 20.0);
        assert_eq!(chrome_zoom(1.5, 20.0), 1.5);
        // The user chose 13 px text: the bar stays at 100 % (the 0.0.1 defect: 175 % over 13 px).
        assert_eq!(chrome_zoom(1.75, 13.0), 1.0);
        // Very large text never makes the bar larger than the UI zoom.
        assert_eq!(chrome_zoom(1.5, 40.0), 1.5);
        assert_eq!(chrome_zoom(1.0, 8.0), 1.0, "at least 100 %");
        // 17 px over 13 px is 1.31: one step.
        assert_eq!(chrome_zoom(2.0, 17.0), 1.25);
    }

    #[test]
    fn a_scale_multiplies_dpi_and_zoom_once() {
        let s = Scale::new(dpi_scale(144), 1.25);
        assert_eq!(s.factor(), 1.875);
        assert_eq!(s.px(32.0), 60);
        assert_eq!(s.px(1.0), 2);
        assert_eq!(Scale::new(1.0, 1.0).px(0.4), 1, "a hairline stays visible");
    }
}
