//! Ddalgak's visual rule editor (Step 74; docs/DDALGAK_GUI_JOURNEYS.md, ADR 0049).
//!
//! `ddalgak-editor.exe` is a separate, on-demand process: the tray app (`ddalgak-gui.exe`)
//! starts it from its menu or the `editor.open` shortcut, and the user can start it directly.
//! It never runs the engine, installs a hook, injects input, opens a program or a website: it
//! edits `rules.json` through the rule store exactly like the CLI ([`repo`]) and asks the
//! running app, over the same identity-checked pipe as the CLI, to load what it saved
//! ([`sync`]). Closing the window ends the process and frees its memory; the engine's idle cost
//! does not change (ADR 0049).
//!
//! - [`form`]: one rule as a sentence, the eight effect tiles and their forms on the engine's
//!   own data models, the simulation for **Test**;
//! - [`listing`]: rows, badges, search, filters and grouping;
//! - [`capture`]: key capture; [`shortcuts`]: the rule list's shortcuts from `hotkeys.json`;
//! - [`transfer`]: export and import hooks (imports arrive turned off, without consent);
//! - [`app`]: the window; [`theme`]: the Kim design tokens; [`i18n`]: Korean and English.

pub mod app;
pub mod capture;
pub mod form;
pub mod i18n;
pub mod listing;
pub mod recorder;
pub mod repo;
pub mod shortcuts;
pub mod sync;
pub mod theme;

/// ADR 0099 (replaces ADR 0091's own rule): the editor's zoom on a screen whose work area is
/// `w` × `h` logical pixels: the UI zoom every Kim program shares, kept so that the first size
/// (1060 × 680 plus the title bar) stays within 90 % of the screen.
pub fn window_scale(w: f32, h: f32) -> f32 {
    let fit = (w * 0.9 / 1060.0).min(h * 0.9 / 712.0);
    kim_design::scale::auto_zoom(w, h).min(kim_design::scale::quantize(fit))
}

#[cfg(test)]
mod scale_tests {
    #[test]
    fn the_editor_follows_the_shared_ui_zoom() {
        assert!((super::window_scale(5120.0, 1410.0) - 1.5).abs() < 1e-6);
        assert!((super::window_scale(1920.0, 1040.0) - 1.0).abs() < 1e-6);
        assert!((super::window_scale(1366.0, 728.0) - 1.0).abs() < 1e-6);
        assert!((super::window_scale(7680.0, 4280.0) - 2.0).abs() < 1e-6);
    }
}
pub mod transfer;

pub use app::{EditorApp, Options};

/// ADR 0101: another start with `--settings` (a rule on CapsLock+/ through the tray app) asks
/// the running editor to open its Settings window; the next frame does.
pub static OPEN_SETTINGS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The editor's file name; the tray app starts it from its own folder.
pub const EDITOR_EXE: &str = "ddalgak-editor.exe";
