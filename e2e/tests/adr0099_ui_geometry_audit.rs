//! E2E — ADR 0099: the real Subak window reports the geometry it laid out, and that geometry passes
//! the design audit; the tab bar follows the shared UI zoom but never outgrows the terminal text.
//! Requirements: `SUBAK-FR-031`, `SUBAK-FR-048`, `KIM-NFR-012`, `KIM-NFR-013`.
//!
//! A private Host (debug `subak`, sandboxed `%APPDATA%`) with three Terminals, one with a Korean
//! name; the debug window in its off-screen test mode with `KIM_UI_INSPECT` and
//! `KIM_UI_SNAPSHOT` set, twice: at a UI zoom of 150 % with the default text (the bar and the text
//! grow together: 54 px bar, 18 px labels, 20 px text) and at 175 % with the user's own 13 px text
//! (the bar stays 36 px with 12 px labels). Each report is read back, checked with
//! `kim_design::audit` (no failure), and the screenshot of the same frame is a real image of the
//! client size.
//!
//! Machine safety: nothing is shown (the window is off-screen), no input is synthesized, a private
//! endpoint and sandbox; the Host is stopped at the end.
#![cfg(windows)]

use kim_e2e::{TempDir, bin_debug};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn sub(sb: &TempDir, ep: &str, args: &[&str]) -> std::process::ExitStatus {
    let cut = args.iter().position(|a| *a == "--").unwrap_or(args.len());
    Command::new(bin_debug("subak"))
        .args(&args[..cut])
        .args(["--endpoint", ep])
        .args(&args[cut..])
        .env("APPDATA", sb.path().join("Roaming"))
        .env("LOCALAPPDATA", sb.path().join("Local"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .unwrap()
}

fn window(
    sb: &TempDir,
    ep: &str,
    tag: &str,
    zoom: &str,
    font: Option<u32>,
) -> (serde_json::Value, Vec<u8>) {
    let dir = sb.path().join(tag);
    let roaming = dir.join("Roaming");
    let cfg = roaming.join("mouv").join("subak");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let mut text = String::from("config_version = 1\n");
    if let Some(f) = font {
        text.push_str(&format!("[appearance]\nfont_size = {f}\n"));
    }
    std::fs::write(cfg.join("config.toml"), text).unwrap();
    let json = dir.join("geometry.json");
    let shot = dir.join("frame.bmp");
    let mut child = Command::new(bin_debug("subak-gui"))
        .args([
            "--test-offscreen",
            "--test-max-seconds",
            "6",
            "--endpoint",
            ep,
            "main",
        ])
        .env("APPDATA", &roaming)
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_NO_INPUT", "1")
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("SUBAK_TEST_SCREEN_ZOOM", zoom)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json).expect("a geometry report")).unwrap();
    (v, std::fs::read(&shot).expect("a screenshot"))
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn adr0099_the_real_window_reports_audited_geometry() {
    let sb = TempDir::new("adr0099");
    std::fs::create_dir_all(sb.path().join("Roaming")).unwrap();
    std::fs::create_dir_all(sb.path().join("Local")).unwrap();
    let ep = format!("kim-e2e-adr0099-{}", std::process::id());
    assert!(sub(&sb, &ep, &["host", "start", "--no-shell"]).success());
    for n in ["main", "로그", "a-much-longer-terminal-name"] {
        assert!(
            sub(
                &sb,
                &ep,
                &["new", n, "--", "cmd.exe", "/d", "/k", "prompt", "$G"]
            )
            .success()
        );
    }
    let mut seen = Vec::new();
    for (tag, zoom, font, bar, label, text) in [
        ("zoom150", "1.5", None, 54u64, 18u64, 20.0f64),
        ("zoom175-font13", "1.75", Some(13), 36, 12, 13.0),
    ] {
        let (v, bmp) = window(&sb, &ep, tag, zoom, font);
        let r = kim_design::inspect::Report::from_json(&v).expect("schema kim.ui-geometry");
        let f = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&f).2,
            0,
            "{}",
            kim_design::audit::render(&r, &f, false)
        );
        assert_eq!(v["extra"]["bar_height"], bar, "{tag}: {}", v["extra"]);
        assert_eq!(v["extra"]["tab_font_px"], label, "{tag}");
        assert_eq!(
            v["extra"]["terminal_font_logical"].as_f64(),
            Some(text),
            "{tag}"
        );
        let tabs = r.of_kind("tab").count();
        assert_eq!(tabs, 3, "{tag}: every Terminal is a tab");
        assert!(
            r.of_kind("tab")
                .any(|t| t.text.as_ref().is_some_and(|x| x.content == "로그")),
            "{tag}: the Korean name is drawn whole"
        );
        assert_eq!(&bmp[..2], b"BM", "{tag}: a real frame");
        let w = i32::from_le_bytes(bmp[18..22].try_into().unwrap());
        assert_eq!(w as f32, r.window.0, "{tag}: the frame is the client area");
        seen.push(format!("{tag}: bar {bar}, label {label}, text {text}"));
    }
    let _ = sub(&sb, &ep, &["host", "stop", "--kill-terminals", "--yes"]);
    println!("EVIDENCE adr0099: {}", seen.join("; "));
}
