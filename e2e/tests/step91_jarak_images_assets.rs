//! E2E — Step 91 (#110): pictures in the preview and the assets of a Markdown document, through
//! the real window and the real CLI (ADR 0102, docs/JARAK_ARCHITECTURE.md §13).
//! Requirements: `JARAK-FR-044`, `JARAK-FR-045`, `JARAK-SEC-005`, `KIM-NFR-012`.
//!
//! 1. The release `jarak-gui` with the UI audit's environment shows its `images` view on a note
//!    in a temporary folder: it writes `assets/wide.png` (1600 × 400), `assets/small.png`
//!    (160 × 90) and `assets/small.webp` (200 × 120) next to the note and previews them with a
//!    missing and a remote reference. The geometry report holds three drawn pictures: the wide one
//!    scaled down to the column (aspect kept), the small PNG and the WebP at their pixel size.
//! 2. `jarak assets list` on a second note that refers to two of them and a missing file: two
//!    found, one missing, and the files no note refers to are listed as orphans (nothing deleted).
//! 3. `jarak assets rename --dry-run` changes nothing; the rename moves `small.png` to
//!    `tiny picture.png` and rewrites the reference (`assets/tiny%20picture.png`); a rename onto an
//!    existing file is refused with `asset_exists` (exit 7) and replaces nothing.
//! 4. The window previews the renamed note: the picture is drawn from its new name.
//!
//! Machine safety: the windows open for about a second each at the audit position; no input is
//! synthesized; nothing outside the temporary folder is touched; no network.
#![cfg(windows)]

use kim_e2e::{TempDir, bin};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn window(dir: &Path, doc: &Path, screen: &str) -> kim_design::inspect::Report {
    std::fs::create_dir_all(dir.join("Roaming")).unwrap();
    std::fs::create_dir_all(dir.join("Local")).unwrap();
    let json = dir.join(format!("{screen}.json"));
    let shot = dir.join(format!("{screen}.png"));
    let mut child = Command::new(bin("jarak-gui"))
        .args([
            "--instance",
            &format!("e2e-step91-{}", std::process::id()),
            "--lang",
            "en",
            &format!("{}:1:1", doc.display()),
        ])
        .env("APPDATA", dir.join("Roaming"))
        .env("LOCALAPPDATA", dir.join("Local"))
        .env("KIM_UI_INSPECT", &json)
        .env("KIM_UI_SNAPSHOT", &shot)
        .env("KIM_UI_EXIT", "1")
        .env("KIM_UI_ZOOM", "1")
        .env("KIM_UI_SCREEN", screen)
        .env("KIM_UI_THEME", "dark")
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
    let exited = child.try_wait().unwrap().is_some();
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        exited,
        "{screen}: the window closed by itself after the measurement"
    );
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json).expect("the geometry report")).unwrap();
    let r = kim_design::inspect::Report::from_json(&v).expect("schema kim.ui-geometry");
    let f = kim_design::audit::audit(&r);
    assert_eq!(
        kim_design::audit::counts(&f).2,
        0,
        "{}",
        kim_design::audit::render(&r, &f, false)
    );
    r
}

fn pictures(r: &kim_design::inspect::Report) -> Vec<(f32, f32)> {
    r.components
        .iter()
        // Ids stay unique in the report: `preview:image`, `preview:image#1`, …
        .filter(|c| c.id == "preview:image" || c.id.starts_with("preview:image#"))
        .map(|c| (c.bounds.w, c.bounds.h))
        .collect()
}

fn jarak(args: &[&str]) -> (i32, serde_json::Value) {
    let out = Command::new(bin("jarak"))
        .args(args)
        .arg("--json")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: one JSON document ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out.status.code().unwrap_or(-1), v)
}

#[test]
#[ignore = "e2e: run via `cargo xtask e2e` only (AGENTS.md §4)"]
fn step91_pictures_in_the_preview_and_assets_through_the_cli() {
    let sb = TempDir::new("e2e-step91");
    let dir = sb.path().join("notes");
    std::fs::create_dir_all(&dir).unwrap();
    // 1. The picture view.
    let sample = dir.join("sample.md");
    std::fs::write(&sample, "# Sample\n").unwrap();
    let r = window(&dir, &sample, "images");
    assert_eq!(r.screen, "images");
    let pics = pictures(&r);
    assert_eq!(pics.len(), 3, "wide, small PNG, WebP: {pics:?}");
    let (w, h) = pics[0];
    assert!(
        w <= 820.5 && w > 300.0,
        "the wide picture fits the column: {pics:?}"
    );
    assert!((w / h - 4.0).abs() < 0.05, "its aspect is kept: {pics:?}");
    assert!(
        (pics[1].0 - 160.0).abs() < 0.5 && (pics[1].1 - 90.0).abs() < 0.5,
        "{pics:?}"
    );
    assert!(
        (pics[2].0 - 200.0).abs() < 0.5 && (pics[2].1 - 120.0).abs() < 0.5,
        "{pics:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&sample).unwrap(),
        "# Sample\n",
        "the view saves nothing"
    );
    let assets = dir.join("assets");
    for f in ["wide.png", "small.png", "small.webp"] {
        assert!(assets.join(f).is_file(), "{f}");
    }
    // 2. The list.
    let note = dir.join("pics.md");
    let text = "# Pics\r\n\r\n![w](assets/wide.png)\r\n\r\n![s](assets/small.png)\r\n![m](assets/missing.png)\r\n";
    std::fs::write(&note, text).unwrap();
    let n = note.to_str().unwrap();
    let (code, v) = jarak(&["assets", "list", n]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["command"], "assets.list");
    let d = &v["data"];
    assert_eq!(
        (d["found"].as_u64(), d["missing"].as_u64()),
        (Some(2), Some(1)),
        "{d}"
    );
    let orphans: Vec<&str> = d["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap())
        .collect();
    assert_eq!(orphans, ["assets/small.webp"], "{d}");
    assert_eq!(d["references"][0]["format"], "png");
    // 3. Rename: a dry run, the rename, a refused one.
    let (code, v) = jarak(&[
        "assets",
        "rename",
        n,
        "assets/small.png",
        "tiny picture",
        "--dry-run",
    ]);
    assert_eq!(
        (code, &v["data"]["renamed"]),
        (0, &serde_json::json!(false)),
        "{v}"
    );
    assert!(assets.join("small.png").is_file());
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        text,
        "the dry run changed nothing"
    );
    let (code, v) = jarak(&["assets", "rename", n, "small.png", "tiny picture"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["reference"], "assets/tiny%20picture.png");
    assert!(assets.join("tiny picture.png").is_file() && !assets.join("small.png").exists());
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        text.replace("assets/small.png", "assets/tiny%20picture.png"),
        "only the reference changed (CRLF kept)"
    );
    let webp = std::fs::read(assets.join("small.webp")).unwrap();
    let (code, v) = jarak(&["assets", "rename", n, "assets/wide.png", "small.webp"]);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (7, Some("asset_exists")),
        "{v}"
    );
    assert_eq!(
        std::fs::read(assets.join("small.webp")).unwrap(),
        webp,
        "nothing replaced"
    );
    assert!(assets.join("wide.png").is_file());
    // 4. The renamed picture in the preview.
    let r = window(&dir, &note, "preview");
    let drawn = pictures(&r);
    assert!(
        drawn
            .iter()
            .any(|(w, h)| (w - 160.0).abs() < 0.5 && (h - 90.0).abs() < 0.5),
        "the renamed picture is drawn: {drawn:?}"
    );
}
