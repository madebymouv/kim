//! E2E — ADR 0100: Ddalgak's Settings is a window of its own, and the main window's Rules toolbar
//! is one system. Requirements: `DDALGAK-FR-050…053`, `DDALGAK-FR-004`, `KIM-NFR-012`.
//!
//! The real debug `ddalgak-editor` with a sandboxed `%APPDATA%` holding the UI fixture's rules
//! (`design/ui-fixtures/ddalgak-rules.json`), a private endpoint (no app answers it) and the UI
//! audit's environment (`KIM_UI_INSPECT`, `KIM_UI_SNAPSHOT`, `KIM_UI_EXIT`, zoom 100 %):
//!
//! 1. `KIM_UI_SCREEN=settings-general`: the report comes from the *Settings window* — its size is
//!    `size::SETTINGS_WINDOW_WIDTH × HEIGHT`, it holds the engine's two switches (Turn on Ddalgak,
//!    Pause automation) and the rail of six sections, and the audit finds no failure; the
//!    photograph is the real window's pixels at that size.
//! 2. `KIM_UI_SCREEN=rules-search`: the Rules page with a search typed: search, Filters and New
//!    Rule share the control height, one centre line and the toolbar gap; no audit failure.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized, nothing is saved to the user's profile, nothing runs.
#![cfg(windows)]

use kim_e2e::{TempDir, bin_debug};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(sb: &TempDir, screen: &str) -> (serde_json::Value, std::path::PathBuf) {
    let dir = sb.path().join(screen);
    let cfg = dir.join("Roaming").join("mouv").join("ddalgak");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("design")
        .join("ui-fixtures")
        .join("ddalgak-rules.json");
    std::fs::copy(fixture, cfg.join("rules.json")).unwrap();
    let json = dir.join("geometry.json");
    let shot = dir.join("frame.png");
    let mut child = Command::new(bin_debug("ddalgak-editor"))
        .args([
            "--no-hotkeys",
            "--endpoint",
            &format!("e2e-adr0100-{}", std::process::id()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_NO_INPUT", "1")
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", "1")
        .env("KIM_UI_SCREEN", screen)
        .env("KIM_UI_THEME", "dark")
        .env("KIM_UI_LANG", "en")
        .env("KIM_UI_POS", "40,40")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(40);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json).expect("the geometry report")).unwrap();
    (v, shot)
}

fn audited(v: &serde_json::Value) -> kim_design::inspect::Report {
    let r = kim_design::inspect::Report::from_json(v).expect("schema kim.ui-geometry");
    let f = kim_design::audit::audit(&r);
    assert_eq!(
        kim_design::audit::counts(&f).2,
        0,
        "{}",
        kim_design::audit::render(&r, &f, false)
    );
    r
}

fn png_size(path: &std::path::Path) -> (u32, u32) {
    let b = std::fs::read(path).expect("the photograph");
    assert_eq!(&b[1..4], b"PNG");
    let be = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    (be(16), be(20))
}

#[test]
fn adr0100_settings_is_its_own_window_and_the_toolbar_is_one_system() {
    use kim_design::size;
    let sb = TempDir::new("e2e-adr0100");
    // 1. The Settings window.
    let (v, shot) = run(&sb, "settings-general");
    let r = audited(&v);
    assert_eq!(r.screen, "settings-general");
    assert_eq!(
        (r.window.0.round(), r.window.1.round()),
        (size::SETTINGS_WINDOW_WIDTH, size::SETTINGS_WINDOW_HEIGHT),
        "the report is the Settings window's own"
    );
    let ids: Vec<&str> = r.components.iter().map(|c| c.id.as_str()).collect();
    for want in [
        "row:Turn on Ddalgak",
        "row:Pause automation",
        "nav:General",
        "nav:Profiles",
        "nav:Modes",
        "nav:Shortcuts",
        "nav:Appearance",
        "nav:Advanced",
    ] {
        assert!(ids.contains(&want), "{want} in {ids:?}");
    }
    assert!(
        !ids.iter().any(|i| i.starts_with("search:")),
        "the main window's controls are not in the Settings report"
    );
    let (w, h) = png_size(&shot);
    assert_eq!(
        (w, h),
        (r.window.0 as u32, r.window.1 as u32),
        "the real window's pixels"
    );
    // 2. The Rules toolbar with a search typed.
    let (v, _) = run(&sb, "rules-search");
    let r = audited(&v);
    let part = |id: &str| {
        r.components
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id}"))
            .bounds
    };
    let (s, f, n) = (
        part("search:Search rules"),
        part("button:Filters"),
        part("button:New Rule"),
    );
    for b in [s, f, n] {
        assert_eq!(b.h, size::CONTROL, "one control height");
        assert_eq!(b.y, s.y, "one line");
    }
    assert_eq!(f.x - (s.x + s.w), kim_design::space::S, "the toolbar gap");
    assert_eq!(n.x - (f.x + f.w), kim_design::space::S, "the toolbar gap");
}
