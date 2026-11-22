//! `cargo xtask ui-audit [--quick] [--only <app>] [--bless]` (ADR 0099, docs/UI_QUALITY_PIPELINE.md).
//!
//! Builds the debug programs, opens each audited view of the **real executables** in a sandboxed
//! profile (Kim Manager, Noeul's settings window and the Ddalgak rule editor at a fixed UI zoom,
//! theme and language through `KIM_UI_*`; Subak's window off-screen through its test mode at a
//! DPI and zoom fixture with real Terminals of a private Host), collects the geometry report each
//! program writes from its own layout code (`KIM_UI_INSPECT`) and the screenshot of the same
//! frame (`KIM_UI_SNAPSHOT`), checks every report with `kim_design::audit`, and compares the
//! curated screenshots with `design/ui-baselines/`. Everything is written under
//! `target/ui-audit/` (`report.txt`, `summary.json`, one JSON and one image per run).
//!
//! Nothing touches the user's programs: private endpoints, sandboxed `%APPDATA%` and
//! `%LOCALAPPDATA%`, no hotkeys, windows placed away from the visible desktop where the program
//! allows it. A failed layout rule fails the command; a changed screenshot is reported for review
//! and never approved automatically (`--bless` copies the current images over the baselines).

use kim_design::audit::{self, Level};
use kim_design::inspect::Report;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// One audited run.
#[derive(Debug, Clone)]
struct Run {
    name: String,
    app: &'static str,
    screen: String,
    zoom: f32,
    theme: &'static str,
    lang: &'static str,
    /// Subak only: the DPI fixture, the font size, compact, the window size, the tabs.
    dpi: u32,
    font: Option<u32>,
    compact: bool,
    size: (u32, u32),
    tabs: usize,
    baseline: bool,
}

fn runs(quick: bool, only: Option<&str>) -> Vec<Run> {
    let mut out = Vec::new();
    let zooms: &[f32] = if quick {
        &[1.0, 1.5]
    } else {
        &[1.0, 1.25, 1.5, 2.0]
    };
    let egui_run = |app: &'static str, screen: &str, z: f32, theme, lang, baseline| Run {
        name: format!("{app}-{screen}-z{z}-{theme}-{lang}"),
        app,
        screen: screen.to_string(),
        zoom: z,
        theme,
        lang,
        dpi: 96,
        font: None,
        compact: false,
        size: (0, 0),
        tabs: 0,
        baseline,
    };
    let screens: [(&str, &[&str]); 4] = [
        (
            "kim",
            &[
                "launcher",
                "settings",
                "settings-shortcuts",
                "settings-catalog",
            ],
        ),
        (
            "noeul",
            &[
                "general",
                "capture",
                "pins",
                "shortcuts",
                "appearance",
                "advanced",
            ],
        ),
        // ADR 0100: the Rules page with rules and without, the filters popover, the rule editor,
        // the key picker, two action forms and every section of the Settings window.
        (
            "ddalgak",
            &[
                "rules",
                "rules-empty",
                "rules-search",
                "filters",
                "new-rule",
                "key-picker",
                "action-text",
                "action-remap",
                "settings-general",
                "settings-profiles",
                "settings-modes",
                "settings-shortcuts",
                "settings-appearance",
                "settings-advanced",
            ],
        ),
        // Step 88 (ADR 0102): Jarak's editor with a document, empty, with a selection, with word
        // wrap, and the unsaved-changes question; Step 89 the preview and the split view.
        (
            "jarak",
            &[
                "editor",
                "editor-empty",
                "editor-selection",
                "editor-wrap",
                "question",
                // Step 89: the Markdown preview alone and side by side with the source.
                "preview",
                "split",
                // Step 90: a pasted bitmap saved as an asset and shown in the preview.
                "paste-image",
                // Step 91: pictures scaled to the column, PNG and WebP, missing and remote.
                "images",
                // Step 92: Vim editing, the command line, the palette, the menu bar, the assets.
                "vim",
                "vim-insert",
                "vim-command",
                "palette",
                "menu-bar",
                "assets",
                // Step 93: tabs, two views with a clone, the side panel, the Documents dialog.
                "tabs",
                "views",
                "side",
                "documents",
                // Step 94: find, replace, mark (with bookmarks and a style), incremental search.
                "find",
                "replace",
                "mark",
                "incremental",
                // Step 95: the Files page, the results panel, the Replace in Files preview.
                "files",
                "results",
                "replace-preview",
            ],
        ),
    ];
    for (app, list) in screens {
        for s in list {
            for &z in zooms {
                // A small curated baseline set: two views per program at 100 %.
                let base = z == 1.0
                    && matches!(
                        *s,
                        "launcher"
                            | "settings"
                            | "general"
                            | "capture"
                            | "rules"
                            | "new-rule"
                            | "key-picker"
                            | "settings-general"
                    );
                out.push(egui_run(app, s, z, "dark", "en", base));
            }
            out.push(egui_run(app, s, 1.5, "light", "ko", false));
        }
    }
    let subak = |name: &str, dpi: u32, z: f32, font, compact, size, tabs, theme, baseline| Run {
        name: format!("subak-{name}"),
        app: "subak",
        screen: "terminal".into(),
        zoom: z,
        theme,
        lang: "en",
        dpi,
        font,
        compact,
        size,
        tabs,
        baseline,
    };
    for &(dpi, pct) in &[(96u32, 100u32), (120, 125), (144, 150), (192, 200)] {
        if quick && dpi != 96 && dpi != 144 {
            continue;
        }
        let sz = (1100 * dpi / 96, 640 * dpi / 96);
        out.push(subak(
            &format!("1tab-dpi{pct}"),
            dpi,
            1.0,
            None,
            false,
            sz,
            1,
            "dark",
            false,
        ));
        out.push(subak(
            &format!("5tabs-dpi{pct}"),
            dpi,
            1.0,
            None,
            false,
            sz,
            5,
            "dark",
            dpi == 96,
        ));
        out.push(subak(
            &format!("20tabs-dpi{pct}"),
            dpi,
            1.0,
            None,
            false,
            (900 * dpi / 96, 600 * dpi / 96),
            20,
            "dark",
            false,
        ));
    }
    out.push(subak(
        "5tabs-zoom150",
        96,
        1.5,
        None,
        false,
        (1650, 960),
        5,
        "dark",
        true,
    ));
    out.push(subak(
        "5tabs-zoom175-font13",
        96,
        1.75,
        Some(13),
        false,
        (1200, 800),
        5,
        "dark",
        false,
    ));
    out.push(subak(
        "5tabs-compact",
        96,
        1.0,
        None,
        true,
        (1100, 640),
        5,
        "dark",
        false,
    ));
    out.push(subak(
        "5tabs-light",
        96,
        1.0,
        None,
        false,
        (1100, 640),
        5,
        "light",
        false,
    ));
    out.push(subak(
        "5tabs-narrow",
        96,
        1.0,
        None,
        false,
        (520, 480),
        5,
        "dark",
        false,
    ));
    out.push(subak(
        "5tabs-wide",
        96,
        1.0,
        None,
        false,
        (2400, 700),
        5,
        "dark",
        false,
    ));
    out.retain(|r| only.is_none_or(|o| o == r.app));
    out
}

fn exe(root: &Path, name: &str) -> PathBuf {
    root.join("target")
        .join("debug")
        .join(format!("{name}.exe"))
}

fn command(path: &Path) -> Command {
    let mut c = Command::new(path);
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    c
}

/// Waits for `child` at most `limit`, then ends it.
fn wait(mut child: std::process::Child, limit: Duration) {
    let until = Instant::now() + limit;
    while Instant::now() < until {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn sandbox(dir: &Path) -> (PathBuf, PathBuf) {
    let (r, l) = (dir.join("Roaming"), dir.join("Local"));
    let _ = std::fs::create_dir_all(&r);
    let _ = std::fs::create_dir_all(&l);
    (r, l)
}

/// ADR 0100: the rules a Ddalgak audit shows (no consent: nothing here can run).
const DDALGAK_FIXTURE: &str = include_str!("../../design/ui-fixtures/ddalgak-rules.json");
/// Step 88: the document a Jarak audit opens (Korean, tabs, a long line, emoji).
const JARAK_FIXTURE: &str = include_str!("../../design/ui-fixtures/jarak-sample.md");

/// One egui program's view.
fn run_egui(root: &Path, out: &Path, r: &Run) -> Result<(PathBuf, PathBuf), String> {
    let (roaming, local) = sandbox(&out.join("sandbox").join(&r.name));
    let json = out.join(format!("{}.json", r.name));
    let shot = out.join(format!("{}.png", r.name));
    let _ = std::fs::remove_file(&json);
    let _ = std::fs::remove_file(&shot);
    let (bin, args): (&str, Vec<String>) = match r.app {
        "kim" => {
            let d = roaming.join("mouv").join("kim");
            let _ = std::fs::create_dir_all(&d);
            let _ = std::fs::write(
                d.join("manager.json"),
                format!(r#"{{"theme":"{}","language":"{}"}}"#, r.theme, r.lang),
            );
            ("kim-manager", vec!["--no-hotkeys".into()])
        }
        "jarak" => {
            let mut args = vec![
                "--instance".into(),
                format!("ui-audit-{}", std::process::id()),
                "--lang".into(),
                r.lang.into(),
            ];
            if r.screen != "editor-empty" {
                let docs = roaming.join("docs");
                let _ = std::fs::create_dir_all(&docs);
                let doc = docs.join("sample.md");
                let _ = std::fs::write(&doc, JARAK_FIXTURE);
                args.push(format!("{}:3:9", doc.display()));
            }
            ("jarak-gui", args)
        }
        "noeul" => (
            "noeul-settings",
            vec![
                "--lang".into(),
                r.lang.into(),
                "--endpoint".into(),
                format!("kim-ui-audit-noeul-{}", std::process::id()),
            ],
        ),
        _ => {
            // ADR 0100: a realistic rules file (two profiles, two modes, four rules) unless the
            // run photographs the empty state.
            if r.screen != "rules-empty" {
                let d = roaming.join("mouv").join("ddalgak");
                let _ = std::fs::create_dir_all(&d);
                let _ = std::fs::write(d.join("rules.json"), DDALGAK_FIXTURE);
            }
            (
                "ddalgak-editor",
                vec![
                    "--no-hotkeys".into(),
                    "--endpoint".into(),
                    format!("kim-ui-audit-ddalgak-{}", std::process::id()),
                ],
            )
        }
    };
    let child = command(&exe(root, bin))
        .args(&args)
        .env("APPDATA", &roaming)
        .env("LOCALAPPDATA", &local)
        .env("KIM_NO_INPUT", "1")
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", r.zoom.to_string())
        .env("KIM_UI_SCREEN", &r.screen)
        .env("KIM_UI_THEME", r.theme)
        .env("KIM_UI_LANG", r.lang)
        .env("KIM_UI_POS", "40,40")
        .env(
            "KIM_UI_FIXTURE",
            format!("zoom={} theme={} lang={}", r.zoom, r.theme, r.lang),
        )
        .spawn()
        .map_err(|e| format!("{bin}: {e}"))?;
    wait(child, Duration::from_secs(40));
    Ok((json, shot))
}

/// Subak's window in its off-screen test mode on a private Host with `r.tabs` Terminals.
fn run_subak(root: &Path, out: &Path, r: &Run) -> Result<(PathBuf, PathBuf), String> {
    let (roaming, local) = sandbox(&out.join("sandbox").join(&r.name));
    let json = out.join(format!("{}.json", r.name));
    let shot = out.join(format!("{}.bmp", r.name));
    let _ = std::fs::remove_file(&json);
    let _ = std::fs::remove_file(&shot);
    let ep = format!("kim-ui-audit-{}-{}", r.name, std::process::id());
    let cli = exe(root, "subak");
    let sub = |args: &[&str]| {
        // `--endpoint` before a `--` (what follows it is the program's own command line).
        let cut = args.iter().position(|a| *a == "--").unwrap_or(args.len());
        command(&cli)
            .args(&args[..cut])
            .args(["--endpoint", &ep])
            .args(&args[cut..])
            .env("APPDATA", &roaming)
            .env("LOCALAPPDATA", &local)
            .status()
    };
    sub(&["host", "start", "--no-shell"]).map_err(|e| format!("subak host: {e}"))?;
    let names = [
        "main",
        "build",
        "로그",
        "ssh-prod",
        "a-much-longer-terminal-name",
    ];
    let cwd = out.to_string_lossy().to_string();
    for i in 0..r.tabs {
        let n = names
            .get(i)
            .map_or_else(|| format!("t{i}"), |s| (*s).to_string());
        let _ = sub(&[
            "new", &n, "--cwd", &cwd, "--", "cmd.exe", "/d", "/k", "prompt", "$G",
        ]);
    }
    let dir = roaming.join("mouv").join("subak");
    let _ = std::fs::create_dir_all(&dir);
    let mut cfg = String::from("config_version = 1\n[appearance]\n");
    if let Some(f) = r.font {
        cfg.push_str(&format!("font_size = {f}\n"));
    }
    if r.compact {
        cfg.push_str("density = \"compact\"\n");
    }
    cfg.push_str(&format!(
        "theme = \"{}\"\n",
        if r.theme == "light" {
            "kim-light"
        } else {
            "kim-dark"
        }
    ));
    let _ = std::fs::write(dir.join("config.toml"), cfg);
    let _ = std::fs::write(
        dir.join("gui.json"),
        format!(
            r#"{{"schema":"subak.gui","schema_version":2,"window":{{"dpi":{},"width":{},"height":{},"x":0,"y":0,"maximized":false}}}}"#,
            r.dpi, r.size.0, r.size.1
        ),
    );
    let child = command(&exe(root, "subak-gui"))
        .args([
            "--test-offscreen",
            "--test-max-seconds",
            "6",
            "--endpoint",
            &ep,
            "main",
        ])
        .env("APPDATA", &roaming)
        .env("LOCALAPPDATA", &local)
        .env("KIM_NO_INPUT", "1")
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("SUBAK_TEST_SCREEN_ZOOM", r.zoom.to_string())
        .env("SUBAK_TEST_DPI", r.dpi.to_string())
        .env(
            "KIM_UI_FIXTURE",
            format!(
                "dpi={} zoom={} font={:?} compact={} tabs={} theme={}",
                r.dpi, r.zoom, r.font, r.compact, r.tabs, r.theme
            ),
        )
        .spawn()
        .map_err(|e| format!("subak-gui: {e}"))?;
    wait(child, Duration::from_secs(20));
    let _ = sub(&["host", "stop", "--kill-terminals", "--yes"]);
    Ok((json, shot))
}

/// An image as (width, height, RGB bytes) from a PNG or an uncompressed BMP.
fn load_image(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(b"BM") {
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap_or([0; 4]));
        let off = u32_at(10) as usize;
        let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?);
        let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?);
        let bpp = u16::from_le_bytes(bytes[28..30].try_into().ok()?) as usize;
        let (wu, hu) = (w.unsigned_abs(), h.unsigned_abs());
        let stride = (wu as usize * bpp / 8).div_ceil(4) * 4;
        let mut rgb = Vec::with_capacity(wu as usize * hu as usize * 3);
        for y in 0..hu as usize {
            let row = if h > 0 { hu as usize - 1 - y } else { y };
            for x in 0..wu as usize {
                let p = off + row * stride + x * bpp / 8;
                rgb.extend_from_slice(&[*bytes.get(p + 2)?, *bytes.get(p + 1)?, *bytes.get(p)?]);
            }
        }
        return Some((wu, hu, rgb));
    }
    let dec = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut rd = dec.read_info().ok()?;
    let mut buf = vec![0; rd.output_buffer_size()?];
    let info = rd.next_frame(&mut buf).ok()?;
    let ch = info.color_type.samples();
    let rgb = buf[..info.buffer_size()]
        .chunks(ch)
        .flat_map(|p| [p[0], p[1.min(ch - 1)], p[2.min(ch - 1)]])
        .collect();
    Some((info.width, info.height, rgb))
}

fn save_png(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgb).map_err(|e| e.to_string())
}

/// The share of pixels whose largest channel difference exceeds `THRESHOLD`, and a diff image.
const THRESHOLD: u8 = 40;
fn compare(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> Option<(f64, Vec<u8>)> {
    if (a.0, a.1) != (b.0, b.1) {
        return None;
    }
    let mut changed = 0usize;
    let mut diff = Vec::with_capacity(a.2.len());
    for (p, q) in a.2.chunks(3).zip(b.2.chunks(3)) {
        let d = p
            .iter()
            .zip(q)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if d > THRESHOLD {
            changed += 1;
            diff.extend_from_slice(&[255, 64, 64]);
        } else {
            let g = p[0] / 3;
            diff.extend_from_slice(&[g, g, g]);
        }
    }
    Some((changed as f64 / (a.2.len() / 3).max(1) as f64, diff))
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let quick = args.iter().any(|a| a == "--quick");
    let bless = args.iter().any(|a| a == "--bless");
    let only = args
        .iter()
        .position(|a| a == "--only")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);
    crate::cargo(
        root,
        &[
            "build",
            "-p",
            "subak",
            "-p",
            "kim-manager",
            "-p",
            "noeul-settings",
            "-p",
            "noeul",
            "-p",
            "ddalgak-editor",
            "-p",
            "ddalgak",
            "-p",
            "jarak-gui",
        ],
    )?;
    let out = root.join("target").join("ui-audit");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let baselines = root.join("design").join("ui-baselines");
    let mut text = String::from("Kim UI Audit\n\n");
    let mut summary = Vec::new();
    let (mut fails, mut warns, mut visual) = (0usize, 0usize, 0usize);
    for r in runs(quick, only) {
        let started = Instant::now();
        let res = if r.app == "subak" {
            run_subak(root, &out, &r)
        } else {
            run_egui(root, &out, &r)
        };
        let (json, shot) = res?;
        let report = std::fs::read_to_string(&json)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| Report::from_json(&v));
        let Some(report) = report else {
            fails += 1;
            text.push_str(&format!(
                "FAIL  {}\n      no geometry report was written\n\n",
                r.name
            ));
            summary.push(serde_json::json!({"run": r.name, "error": "no report"}));
            continue;
        };
        let findings = audit::audit(&report);
        let (p, w, f) = audit::counts(&findings);
        fails += f;
        warns += w;
        text.push_str(&format!(
            "== {} ({:.1} s)\n",
            r.name,
            started.elapsed().as_secs_f32()
        ));
        text.push_str(&audit::render(&report, &findings, false));
        // The screenshot: PNG next to the report, compared with its baseline.
        let mut vis = serde_json::Value::Null;
        if let Some(img) = load_image(&shot) {
            let png_path = out.join(format!("{}.png", r.name));
            if shot.extension().is_some_and(|e| e == "bmp") {
                save_png(&png_path, img.0, img.1, &img.2)?;
                let _ = std::fs::remove_file(&shot);
            }
            if r.baseline {
                let base = baselines.join(format!("{}.png", r.name));
                if bless {
                    std::fs::create_dir_all(&baselines).map_err(|e| e.to_string())?;
                    std::fs::copy(&png_path, &base).map_err(|e| e.to_string())?;
                    text.push_str(&format!("BLESS {}\n", base.display()));
                } else if let Some(b) = load_image(&base) {
                    match compare(&img, &b) {
                        Some((share, diff)) if share > 0.005 => {
                            visual += 1;
                            let dp = out.join(format!("{}.diff.png", r.name));
                            save_png(&dp, img.0, img.1, &diff)?;
                            text.push_str(&format!(
                                "VISUAL {}: {:.2} % of the pixels differ from the baseline (review {})\n",
                                r.name,
                                share * 100.0,
                                dp.display()
                            ));
                            vis = serde_json::json!({"changed": share});
                        }
                        Some((share, _)) => {
                            text.push_str(&format!(
                                "VISUAL {}: matches the baseline ({:.3} %)\n",
                                r.name,
                                share * 100.0
                            ));
                            vis = serde_json::json!({"changed": share});
                        }
                        None => {
                            visual += 1;
                            text.push_str(&format!(
                                "VISUAL {}: size {}×{} differs from the baseline {}×{}\n",
                                r.name, img.0, img.1, b.0, b.1
                            ));
                            vis = serde_json::json!({"size_changed": [img.0, img.1, b.0, b.1]});
                        }
                    }
                } else {
                    text.push_str(&format!(
                        "VISUAL {}: no baseline yet (--bless to record)\n",
                        r.name
                    ));
                }
            }
        } else {
            text.push_str("      (no screenshot)\n");
        }
        text.push('\n');
        summary.push(serde_json::json!({
            "run": r.name, "app": r.app, "screen": report.screen, "fixture": report.fixture,
            "window": [report.window.0, report.window.1], "scale": report.scale,
            "pass": p, "warn": w, "fail": f, "visual": vis,
            "extra": report.extra,
        }));
        if f > 0 {
            for x in findings.iter().filter(|x| x.level == Level::Fail).take(3) {
                eprintln!(
                    "FAIL {} {} [{}]: {} (actual {}, expected {})",
                    r.name, x.component, x.rule, x.message, x.actual, x.expected
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(out.join("sandbox"));
    text.push_str(&format!(
        "\n{} runs · {fails} layout failures · {warns} warnings · {visual} visual changes to review\n",
        summary.len()
    ));
    std::fs::write(out.join("report.txt"), &text).map_err(|e| e.to_string())?;
    std::fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    println!("{text}");
    println!("written: {}", out.join("report.txt").display());
    if fails > 0 {
        Err(format!("ui-audit: {fails} layout rule failures"))
    } else {
        Ok(())
    }
}
