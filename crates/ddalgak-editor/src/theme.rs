//! The Kim design system (`kim-ui`, ADR 0092) for the rule editor: the theme chosen in Settings
//! (system, dark or light; dark is the reference), the type scale and the Korean font fallback.
//! The colours and metrics below are the shared tokens, read under the names the editor's views
//! use.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};

static LIGHT: AtomicBool = AtomicBool::new(false);

/// The colors the editor uses, by token name (the theme in use).
#[derive(Debug, Clone, Copy)]
pub struct Colors;

impl Colors {
    pub fn get() -> kim_ui::Colors {
        kim_ui::Colors::of(if LIGHT.load(Ordering::Relaxed) {
            kim_ui::Theme::Light
        } else {
            kim_ui::Theme::Dark
        })
    }
}

/// Font sizes and control metrics from the tokens.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub caption: f32,
    pub body: f32,
    pub emphasis: f32,
    pub title: f32,
    pub control_height: f32,
}

impl Metrics {
    pub fn get() -> Metrics {
        use kim_design::{font, size};
        Metrics {
            caption: font::CAPTION,
            body: font::BODY,
            emphasis: font::EMPHASIS,
            title: font::TITLE,
            control_height: size::CONTROL,
        }
    }
}

/// Installs the design system (dark first; [`set_theme`] follows the user's choice).
pub fn apply(ctx: &egui::Context) {
    kim_ui::apply(
        ctx,
        kim_ui::Options {
            theme: kim_ui::Theme::Dark,
            ..kim_ui::Options::default()
        },
    );
    LIGHT.store(false, Ordering::Relaxed);
}

/// Switches the theme (Settings → Theme).
pub fn set_theme(ctx: &egui::Context, theme: kim_ui::Theme) {
    LIGHT.store(
        kim_ui::effective_theme(theme) == kim_ui::Theme::Light,
        Ordering::Relaxed,
    );
    kim_ui::set_theme(ctx, theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_is_the_near_black_kim_dark_tokens() {
        let c = Colors::get();
        assert_eq!(c.bg_base, egui::Color32::from_rgb(0x0E, 0x0E, 0x12));
        assert_eq!(c.accent, egui::Color32::from_rgb(0xB7, 0xA6, 0xF5));
        let m = Metrics::get();
        assert!((m.body - 12.0).abs() < f32::EPSILON);
        assert!((m.emphasis - 13.0).abs() < f32::EPSILON);
        assert!((m.control_height - 28.0).abs() < f32::EPSILON);
    }
}
