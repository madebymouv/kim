//! `kim-manager` — the optional Kim Manager window. Started only by the user; closing it ends
//! the process (no tray icon, no background updater).
//!
//! Options: `--no-hotkeys` (start without any shortcut, HOTKEY_CONTRACT §8.3) and
//! `--catalog <file://…/catalog.json>` (ADR 0089: use a local mirror of packages built on this
//! PC for this session; a double-click on `kim.exe` passes the one next to it).

#![windows_subsystem = "windows"]

use eframe::egui;
use kim_manager::{Backend, KimManagerApp, Prefs};

/// The Kim icon (the laver sheet) for the window's title bar and taskbar button.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon/kim-128.png");

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let catalog = args
        .iter()
        .position(|a| a == "--catalog")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // ADR 0099: the UI zoom every Kim program shares (Settings → General → Size, or Auto).
    // A UI audit's fixed zoom (`KIM_UI_ZOOM`) is what Auto means for that run.
    let auto = match std::env::var(kim_ui::design::scale::ENV_ZOOM) {
        Ok(_) => kim_ui::design::scale::startup_zoom(),
        Err(_) => kim_ui::design::system::work_area_logical()
            .map_or(1.0, |(w, h)| kim_manager::auto_scale(egui::vec2(w, h))),
    };
    let backend = Backend::real().map_err(|e| eframe::Error::AppCreation(e.to_string().into()))?;
    let zoom = match std::env::var(kim_ui::design::scale::ENV_ZOOM) {
        Ok(_) => kim_ui::design::scale::startup_zoom(),
        Err(_) => backend
            .settings_path
            .as_deref()
            .map_or(Prefs::default(), Prefs::load_beside)
            .scale_or(auto),
    };
    let [w, h] = kim_manager::BASE_SIZE;
    let [mw, mh] = kim_manager::MIN_SIZE;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Kim")
        // ADR 0090: a small launcher: the tool tiles (four since ADR 0102) and the gear, generous
        // space around them.
        .with_inner_size([w * zoom, h * zoom])
        .with_min_inner_size([mw * zoom, mh * zoom]);
    viewport = kim_ui::inspect::audit_viewport(viewport);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "kim-manager",
        options,
        Box::new(move |cc| {
            let mut app = KimManagerApp::new(backend, Some(&cc.egui_ctx));
            app.set_window_scale(&cc.egui_ctx, auto, zoom);
            // Emergency start without any shortcut (HOTKEY_CONTRACT §8.3, KIM-SEC-023).
            if args.iter().any(|a| a == "--no-hotkeys") {
                app.disable_hotkeys();
            }
            if let Some(url) = &catalog {
                app.use_mirror(url);
            }
            // ADR 0099: the view a UI audit asks for.
            if let Ok(s) = std::env::var(kim_ui::design::inspect::ENV_SCREEN) {
                app.audit_screen(&s);
            }
            Ok(Box::new(app))
        }),
    )
}
