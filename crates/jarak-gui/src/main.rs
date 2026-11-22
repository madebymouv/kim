//! `jarak-gui.exe`: Jarak's editor window (ADR 0102). One window process per user session
//! (`JARAK-FR-002`): a second start hands its files to the running window and exits, unless it
//! is started with `--new-window`.

#![cfg_attr(not(test), windows_subsystem = "windows")]

use std::process::ExitCode;

const HELP: &str = "\
jarak-gui: Jarak's editor window.

Usage: jarak-gui [<file>[:<line>[:<column>]]...] [--new-window] [--lang en|ko]

A running Jarak window takes the files instead (no second window) unless --new-window is given.
Use `jarak` for the command line (`jarak --help`).
";

#[cfg(windows)]
#[allow(unsafe_code)]
mod single {
    //! The owner holds a named mutex; a second start writes a hand-off file, sets the named
    //! event and exits. A thread of the owner sleeps in the wait (no polling).

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

    /// The owner's handles, kept for the life of the process (closed by Windows at exit).
    pub struct Owner {
        _mutex: HANDLE,
        event: HANDLE,
    }

    // SAFETY: kernel handles are process-wide and usable from any thread.
    unsafe impl Send for Owner {}

    pub enum Claim {
        Owner(Owner),
        /// Another window owns the session; `signal` tells it that files are waiting.
        Other(Box<dyn FnOnce()>),
    }

    /// `Local\` names are per logon session already; `tag` separates test instances.
    pub fn claim(tag: Option<&str>) -> io::Result<Claim> {
        let base = format!(
            "Local\\mouv-jarak-gui{}",
            tag.map(|t| format!("-{t}")).unwrap_or_default()
        );
        let m = wide(&base);
        let e = wide(&format!("{base}-open"));
        // SAFETY: NUL-terminated names; default security; an auto-reset event, not signaled.
        let (mutex, existed, event) = unsafe {
            let mutex = CreateMutexW(std::ptr::null(), 0, m.as_ptr());
            let existed = GetLastError() == ERROR_ALREADY_EXISTS;
            let event = CreateEventW(std::ptr::null(), 0, 0, e.as_ptr());
            (mutex, existed, event)
        };
        if mutex.is_null() || event.is_null() {
            return Err(io::Error::last_os_error());
        }
        if existed {
            let (mutex, event) = (mutex as usize, event as usize);
            return Ok(Claim::Other(Box::new(move || {
                // SAFETY: valid handles from above; the owner may take the foreground this
                // process got from its starter; the handles are closed here.
                unsafe {
                    AllowSetForegroundWindow(ASFW_ANY);
                    SetEvent(event as HANDLE);
                    CloseHandle(event as HANDLE);
                    CloseHandle(mutex as HANDLE);
                }
            })));
        }
        Ok(Claim::Owner(Owner {
            _mutex: mutex,
            event,
        }))
    }

    /// Calls `f` every time a second start signals.
    pub fn on_open(owner: &Owner, f: impl Fn() + Send + 'static) {
        let event = owner.event as usize;
        let _ = std::thread::Builder::new()
            .name("jarak-handoff".into())
            .spawn(move || {
                loop {
                    // SAFETY: the event handle lives as long as the process (the owner is kept).
                    let r = unsafe { WaitForSingleObject(event as HANDLE, INFINITE) };
                    if r != WAIT_OBJECT_0 {
                        break;
                    }
                    f();
                }
            });
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = Vec::new();
    let mut new_window = false;
    let mut lang = None;
    let mut tag: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("jarak-gui {}", jarak_core::VERSION);
                return ExitCode::SUCCESS;
            }
            "--new-window" => new_window = true,
            "--lang" => {
                i += 1;
                match args.get(i).and_then(|l| kim_design::Lang::parse(l)) {
                    Some(l) => lang = Some(l),
                    None => return ExitCode::from(2),
                }
            }
            // Separate single-instance names for test-started windows (never the user's).
            "--instance" => {
                i += 1;
                tag = args.get(i).cloned();
            }
            "--" => files.extend(args[i + 1..].iter().cloned()),
            a if a.starts_with("--") => return ExitCode::from(2),
            a => files.push(a.to_string()),
        }
        if args[i] == "--" {
            break;
        }
        i += 1;
    }
    run(&files, new_window, lang, tag.as_deref())
}

/// Absolute targets, so the running window resolves them against the right folder.
fn absolute_args(files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter_map(|a| jarak_core::target::OpenTarget::parse(a))
        .map(|t| {
            jarak_core::target::OpenTarget {
                path: std::path::absolute(&t.path).unwrap_or(t.path.clone()),
                ..t
            }
            .to_arg()
        })
        .collect()
}

#[cfg(windows)]
fn run(
    files: &[String],
    new_window: bool,
    lang: Option<kim_design::Lang>,
    tag: Option<&str>,
) -> ExitCode {
    use eframe::egui;
    use jarak_gui::{Incoming, JarakApp, handoff};
    use std::sync::{Arc, Mutex};

    let files = absolute_args(files);
    let Some(paths) = jarak_core::paths::Paths::from_env() else {
        return ExitCode::from(1);
    };
    let dir = match tag {
        Some(t) => paths.handoff_dir().join(t),
        None => paths.handoff_dir(),
    };
    let owner = if new_window {
        None
    } else {
        match single::claim(tag) {
            Ok(single::Claim::Owner(o)) => Some(o),
            Ok(single::Claim::Other(signal)) => {
                if handoff::write(&dir, &files).is_err() {
                    return ExitCode::from(1);
                }
                signal();
                return ExitCode::SUCCESS;
            }
            Err(_) => return ExitCode::from(1),
        }
    };
    if owner.is_some() {
        handoff::clear(&dir);
    }
    let lang = lang.unwrap_or_else(kim_design::Lang::system);
    let zoom = kim_design::scale::startup_zoom();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(jarak_gui::TITLE)
        .with_app_id("mouv.jarak")
        .with_inner_size([960.0 * zoom, 640.0 * zoom])
        .with_min_inner_size([360.0 * zoom, 240.0 * zoom]);
    // The window's own icon in the title bar and the taskbar (the .exe carries it too).
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/jarak/jarak-256.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: kim_ui::inspect::audit_viewport(viewport),
        renderer: eframe::Renderer::Glow,
        // Ctrl+C/X/V reach Jarak as keys; it uses the clipboard itself (text, and images in
        // Step 90).
        clipboard_shortcuts: false,
        ..Default::default()
    };
    let result = eframe::run_native(
        "jarak-gui",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(zoom);
            if let Some(owner) = &owner {
                let ctx = cc.egui_ctx.clone();
                let queue = incoming.clone();
                let dir = dir.clone();
                single::on_open(owner, move || {
                    let got = handoff::drain(&dir);
                    if let Ok(mut q) = queue.lock() {
                        q.extend(got);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.request_repaint();
                });
            }
            // `Owner` has no `Drop`: its handles stay open until the process ends.
            // Step 93: with a session (not in a UI audit), the session's documents come first and
            // the command line's files are opened after them.
            let paths = jarak_core::paths::Paths::from_env();
            // An e2e may keep a session in a UI audit's sandbox (`KIM_UI_SESSION=1`).
            let with_session = paths.is_some()
                && (!kim_ui::inspect::active()
                    || std::env::var("KIM_UI_SESSION").is_ok_and(|v| v == "1"));
            let mut app = JarakApp::with(
                lang,
                if with_session { &[] } else { &files },
                incoming,
                Arc::new(jarak_gui::WinDialogs),
                jarak_core::file::system_encoding(),
            )
            .with_clipboard(Arc::new(jarak_gui::clipboard::WinClipboard))
            .with_opener(Arc::new(jarak_gui::preview::ShellOpener))
            .with_runner(Arc::new(jarak_gui::macros::ProcessRunner))
            // Step 104: files changed by other programs are checked by the window itself.
            .with_watching();
            // Step 92: the editing mode (Standard or Vim) lives in the settings file.
            if let Some(p) = paths {
                app = app.with_settings(p.settings_file());
                // Step 98: the user-defined languages.
                app = app.with_languages(p.languages_dir());
                // Step 100: the snippets file and Notepad++ API files.
                app = app.with_assist(p.config.clone());
                // Step 102: the saved macros and run commands.
                app = app.with_macros(p.config.join("macros.json"));
                // Step 95: Replace in Files journals the originals here (not in a UI audit).
                if !kim_ui::inspect::active() {
                    app = app.with_journal(p.journal_dir());
                }
                if with_session {
                    let restore = jarak_gui::read_setting(&p.settings_file(), "restore_session")
                        .is_none_or(|v| v != "false");
                    app = app.with_session(jarak_gui::sessions::SessionFiles {
                        file: p.session_file(),
                        recovery: p.recovery_dir(),
                        restore,
                    });
                    app.open_args(&files);
                    // Step 104: recovery snapshots of unsaved documents, and the offer of the
                    // documents a window that no longer runs left behind.
                    #[cfg(windows)]
                    {
                        let interval =
                            jarak_gui::read_setting(&p.settings_file(), "recovery_interval")
                                .and_then(|v| v.trim().parse::<f64>().ok())
                                .unwrap_or(jarak_gui::recover::DEFAULT_INTERVAL);
                        app = app.with_recovery(
                            &p.recovery_dir(),
                            jarak_core::recovery::current_owner(),
                            &jarak_core::recovery::owner_alive,
                            interval,
                        );
                    }
                }
            }
            // ADR 0099: a UI audit names the view to photograph.
            if kim_ui::inspect::active()
                && let Ok(screen) = std::env::var("KIM_UI_SCREEN")
            {
                app.prepare_audit(&screen);
            }
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
