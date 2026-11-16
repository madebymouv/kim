//! `ddalgak-editor` — Ddalgak's rule editor window (Step 74, ADR 0049).
//!
//! Started on demand: by the tray app (*Open rule editor*, the `editor.open` shortcut) or by the
//! user. One editor per user session: a second start brings the open window to the front and
//! exits. Closing the window ends the process. The editor never starts the tray app or turns the
//! engine on by itself; it edits `rules.json` through the rule store and asks a running app to
//! load what it saved.
//!
//! Options: `--lang en|ko`, `--no-hotkeys` (or `KIM_NO_HOTKEYS`: the rule list's shortcuts are
//! off), `--settings` (ADR 0101: open the Settings window too; a running editor is asked to),
//! `--help`, `--version`. Debug builds only: `--endpoint <name>` (talk to a test host).

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::ExitCode;

const HELP: &str = "\
ddalgak-editor — Ddalgak rule editor

USAGE:
    ddalgak-editor [--lang en|ko] [--no-hotkeys] [--settings]
    ddalgak-editor --help | --version

Opens the rule editor window. It edits rules.json (the same store as `ddalgak rule …`) and asks
a running Ddalgak (ddalgak-gui.exe) to load what you saved; it never starts Ddalgak and never
turns the engine on by itself. One editor per user session.
";

#[cfg(windows)]
#[allow(unsafe_code)]
mod single {
    //! One editor per user session: a named mutex marks the owner, a named event asks it to show
    //! its window. The second process lets the first take the foreground, signals and exits.

    use std::io;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, INFINITE, SetEvent, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The owner's handles: never closed by the editor, so they last as long as the process.
    pub struct Owner {
        _mutex: HANDLE,
        pub event: HANDLE,
        /// ADR 0101: a second start with `--settings` also sets this one.
        pub settings: HANDLE,
    }

    // SAFETY: kernel handles are process-wide and usable from any thread.
    unsafe impl Send for Owner {}

    pub enum Claim {
        Owner(Owner),
        /// Another editor owns the session; it was asked to show itself.
        Shown,
    }

    pub fn claim(tag: Option<&str>, settings: bool) -> io::Result<Claim> {
        let base = format!(
            "Local\\mouv-ddalgak-editor-{}-s{}{}",
            ddalgak::win::user_sid()?,
            ddalgak::win::session_id(),
            tag.map(|t| format!("-{t}")).unwrap_or_default()
        );
        let m = wide(&base);
        let e = wide(&format!("{base}-show"));
        let st = wide(&format!("{base}-settings"));
        // SAFETY: NUL-terminated names; default security; auto-reset events, not signaled.
        let (mutex, existed, event, settings_event) = unsafe {
            let mutex = CreateMutexW(std::ptr::null(), 0, m.as_ptr());
            let existed = GetLastError() == ERROR_ALREADY_EXISTS;
            let event = CreateEventW(std::ptr::null(), 0, 0, e.as_ptr());
            let settings_event = CreateEventW(std::ptr::null(), 0, 0, st.as_ptr());
            (mutex, existed, event, settings_event)
        };
        if mutex.is_null() || event.is_null() || settings_event.is_null() {
            return Err(io::Error::last_os_error());
        }
        if existed {
            // SAFETY: valid handles from above; the first editor may take the foreground this
            // process got from its starter; the handles are closed here.
            unsafe {
                AllowSetForegroundWindow(ASFW_ANY);
                if settings {
                    SetEvent(settings_event);
                }
                SetEvent(event);
                CloseHandle(event);
                CloseHandle(settings_event);
                CloseHandle(mutex);
            }
            return Ok(Claim::Shown);
        }
        Ok(Claim::Owner(Owner {
            _mutex: mutex,
            event,
            settings: settings_event,
        }))
    }

    /// The primary monitor's work area in points (read before the window exists: the process
    /// is not DPI aware yet, so Windows answers in 96-DPI points).
    pub fn work_area() -> Option<(f32, f32)> {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: SPI_GETWORKAREA writes one RECT.
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut r).cast(), 0) };
        (ok != 0 && r.right > r.left && r.bottom > r.top)
            .then(|| ((r.right - r.left) as f32, (r.bottom - r.top) as f32))
    }

    /// Calls `f` every time a second start with `--settings` asks.
    pub fn on_settings(owner: &Owner, f: impl Fn() + Send + 'static) {
        wait_loop(owner.settings as usize, "ddalgak-editor-settings", f);
    }

    /// Calls `show` every time a second start asks (a thread sleeping in the wait; no polling).
    pub fn on_show(owner: &Owner, show: impl Fn() + Send + 'static) {
        wait_loop(owner.event as usize, "ddalgak-editor-show", show);
    }

    fn wait_loop(event: usize, name: &str, show: impl Fn() + Send + 'static) {
        let _ = std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                loop {
                    // SAFETY: the event handle lives as long as the process (the owner is kept).
                    let r = unsafe { WaitForSingleObject(event as HANDLE, INFINITE) };
                    if r != WAIT_OBJECT_0 {
                        break;
                    }
                    show();
                }
            });
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut lang = None;
    let mut no_hotkeys = kim_hotkeys::no_hotkeys_env();
    let mut settings = false;
    // `--endpoint` exists in debug builds only.
    #[cfg_attr(not(debug_assertions), allow(unused_mut))]
    let mut endpoint: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("ddalgak-editor {}", ddalgak::VERSION);
                return ExitCode::SUCCESS;
            }
            "--no-hotkeys" => no_hotkeys = true,
            "--settings" => settings = true,
            "--lang" => {
                i += 1;
                match args
                    .get(i)
                    .and_then(|l| ddalgak_editor::i18n::Lang::parse(l))
                {
                    Some(l) => lang = Some(l),
                    None => {
                        eprintln!("ddalgak-editor: error: --lang takes en or ko (usage)");
                        return ExitCode::from(2);
                    }
                }
            }
            #[cfg(debug_assertions)]
            "--endpoint" => {
                i += 1;
                endpoint = args.get(i).cloned();
            }
            other => {
                eprintln!("ddalgak-editor: error: unknown option `{other}` (usage)");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    run(lang, no_hotkeys, endpoint, settings)
}

#[cfg(windows)]
fn run(
    lang: Option<ddalgak_editor::i18n::Lang>,
    no_hotkeys: bool,
    endpoint: Option<String>,
    settings: bool,
) -> ExitCode {
    use ddalgak_editor::sync::PipeEngine;
    use ddalgak_editor::{EditorApp, Options};
    use eframe::egui;
    use std::sync::Arc;

    let owner = match single::claim(endpoint.as_deref(), settings) {
        Ok(single::Claim::Owner(o)) => o,
        Ok(single::Claim::Shown) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ddalgak-editor: error: {e} (io)");
            return ExitCode::from(1);
        }
    };
    let paths = match ddalgak::paths::Paths::from_env() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ddalgak-editor: error: {e} (io)");
            return ExitCode::from(1);
        }
    };
    let endpoint = match endpoint.map_or_else(ddalgak::win::pipe::default_endpoint, Ok) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ddalgak-editor: error: {e} (io)");
            return ExitCode::from(1);
        }
    };
    let lang = lang.unwrap_or_else(system_lang);
    // Reading the monitor layout moves nothing; the mouse form's simulation draws on it.
    let layout =
        ddalgak::win::mouse::layout().unwrap_or_else(|| ddalgak::mouse::Layout::single(1920, 1080));
    let cursor = layout.primary().rect;
    let cursor = ddalgak::mouse::Point::new(
        cursor.left + cursor.width() / 2,
        cursor.top + cursor.height() / 2,
    );
    // ADR 0091: everything is drawn larger on a large screen.
    let zoom = match (
        std::env::var(kim_design::scale::ENV_ZOOM),
        kim_design::prefs::UiPrefs::load().zoom,
    ) {
        (Err(_), None) => kim_design::system::work_area_logical()
            .map_or(1.0, |(w, h)| ddalgak_editor::window_scale(w, h)),
        _ => kim_design::scale::startup_zoom(),
    };
    let _ = single::work_area;
    let options = eframe::NativeOptions {
        viewport: kim_ui::inspect::audit_viewport(
            egui::ViewportBuilder::default()
                .with_title("Ddalgak rules")
                .with_app_id("mouv.ddalgak.editor")
                .with_inner_size([1060.0 * zoom, 680.0 * zoom])
                .with_min_inner_size([420.0 * zoom, 360.0 * zoom]),
        ),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let result = eframe::run_native(
        "ddalgak-editor",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(zoom);
            let ctx = cc.egui_ctx.clone();
            single::on_show(&owner, move || {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint();
            });
            let ctx2 = cc.egui_ctx.clone();
            single::on_settings(&owner, move || {
                ddalgak_editor::OPEN_SETTINGS.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx2.request_repaint();
            });
            if settings {
                ddalgak_editor::OPEN_SETTINGS.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            // The owner's handles stay open until the process ends (no Drop: closed by Windows).
            let mut app = EditorApp::new(Options {
                paths,
                engine: Arc::new(PipeEngine { endpoint }),
                lang,
                no_hotkeys,
                watch: true,
                layout,
                cursor,
                launcher: Arc::new(ddalgak::win::launch::WinLauncher),
                // ADR 0100: Settings -> Start at sign-in (debug builds: a test key from
                // DDALGAK_DEV_STARTUP_KEY, like the CLI).
                startup: Box::new(ddalgak::win::startup::Registry::for_cli()),
                gui_exe: std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("ddalgak-gui.exe")))
                    .unwrap_or_else(|| "ddalgak-gui.exe".into()),
            });
            // ADR 0099: the view a UI audit asks for.
            if let Ok(s) = std::env::var(kim_design::inspect::ENV_SCREEN) {
                app.audit_screen(&s);
            }
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ddalgak-editor: error: the window could not open: {e} (io)");
            ExitCode::from(1)
        }
    }
}

/// Korean when Windows' display language is Korean, English otherwise (`--lang` overrides).
#[cfg(windows)]
fn system_lang() -> ddalgak_editor::i18n::Lang {
    if kim_design::Lang::system() == kim_design::Lang::Ko {
        ddalgak_editor::i18n::Lang::Ko
    } else {
        ddalgak_editor::i18n::Lang::En
    }
}

#[cfg(not(windows))]
fn run(_: Option<ddalgak_editor::i18n::Lang>, _: bool, _: Option<String>, _: bool) -> ExitCode {
    // ADR 0011: no Linux Ddalgak before milestone D7 (DDALGAK-NFR-009).
    eprintln!(
        "ddalgak-editor: error: the Ddalgak editor needs Windows for now (unsupported_platform)"
    );
    ExitCode::from(5)
}
