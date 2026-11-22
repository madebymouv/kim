//! Step 88 (#107): the cost of one frame after a keystroke in a 10 MB document (`JARAK-NFR-002`:
//! keystroke to painted frame ≤ 16 ms p95). Headless (egui's layout and tessellation on the CPU,
//! no GPU upload), so it measures what Jarak itself does per frame.
//!
//! `cargo test --release -p jarak-gui --test perf -- --ignored --nocapture`

use eframe::egui;
use egui_kittest::Harness;
use jarak_gui::{Incoming, JarakApp};
use kim_design::Lang;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn a_keystroke_frame_in_a_10_mb_document() {
    let line = "The quick brown fox 한글 문장 jumps over the lazy dog 0123456789\r\n";
    let text = line.repeat(10 * 1024 * 1024 / line.len() + 1);
    let dir = std::env::temp_dir().join(format!("jarak-perf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ten.txt");
    std::fs::write(&file, &text).unwrap();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(Lang::En, &[format!("{}:70000", file.display())], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    let mut times = Vec::new();
    for i in 0..300 {
        let ev = if i % 10 == 9 {
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }
        } else {
            egui::Event::Text(if i % 2 == 0 { "a" } else { "한" }.into())
        };
        let t = Instant::now();
        h.event(ev);
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "keystroke frame (10 MB, {} lines): p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        h.state().tab().file.doc.text().len_lines(),
        p(0.5),
        p(0.95),
        p(1.0)
    );
    assert!(p(0.95) < 16.0, "p95 {:.2} ms", p(0.95));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 89 (#108): switching a 1 MB Markdown document to the preview (`JARAK-NFR-005`: ≤ 100 ms),
/// the first switch (parse + layout of the visible blocks) and the following ones (cached parse).
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn preview_switch_of_a_1_mb_markdown_document() {
    let unit = "## Section\n\nSome text with *emphasis*, `code` and a [link](https://x.org).\n\n- item one\n- item two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```\ncode block\n```\n\n";
    let text = unit.repeat(1024 * 1024 / unit.len());
    let dir = std::env::temp_dir().join(format!("jarak-perf-md-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("big.md");
    std::fs::write(&file, &text).unwrap();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(Lang::En, &[format!("{}:20000", file.display())], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    let ctrl_e = egui::Event::Key {
        key: egui::Key::E,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    };
    let mut times = Vec::new();
    for _ in 0..20 {
        let t = Instant::now();
        h.event(ctrl_e.clone());
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let first = times[0];
    let mut rest = times[1..].to_vec();
    rest.sort_by(f64::total_cmp);
    println!(
        "preview switch (1 MB Markdown): first {first:.1} ms, then p50 {:.2} ms, max {:.2} ms",
        rest[rest.len() / 2],
        rest[rest.len() - 1]
    );
    assert!(first < 100.0, "first switch {first:.1} ms");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 91 (#110): a Markdown document with 50 screenshot-sized pictures (1920 × 1080 PNG,
/// 8.3 MB of pixels each): the preview's frame time while scrolling through them as they decode
/// on the worker, a keystroke in split view while pictures load, and the texture cache's size
/// (`JARAK-FR-044`: bounded; budget `media::BUDGET`).
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn fifty_pictures_scroll_and_type_within_budget() {
    let dir = std::env::temp_dir().join(format!("jarak-perf-img-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let (w, h_px) = (1920u32, 1080u32);
    let mut rgba = vec![0u8; (w * h_px * 4) as usize];
    let mut text = String::from("# Fifty pictures\n\n");
    for i in 0..50u32 {
        for (k, px) in rgba.chunks_mut(4).enumerate() {
            let (x, y) = (k as u32 % w, k as u32 / w);
            px.copy_from_slice(&[
                (x * 255 / w) as u8,
                (y * 255 / h_px) as u8,
                (i * 5) as u8,
                255,
            ]);
        }
        let png = jarak_gui::images::encode_png(w, h_px, &rgba).unwrap();
        std::fs::write(dir.join("assets").join(format!("shot-{i:02}.png")), png).unwrap();
        text.push_str(&format!(
            "Picture {i}:\n\n![shot](assets/shot-{i:02}.png)\n\n"
        ));
    }
    let file = dir.join("fifty.md");
    std::fs::write(&file, &text).unwrap();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(Lang::En, &[file.display().to_string()], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    let key = |k: egui::Key, m: egui::Modifiers| egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    };
    // Preview, then page down through all fifty while they decode.
    h.event(key(egui::Key::E, egui::Modifiers::CTRL));
    h.step();
    let mut frames = Vec::new();
    let mut peak = 0usize;
    let start = Instant::now();
    for i in 0..400 {
        if i % 4 == 0 {
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -400.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: egui::Modifiers::NONE,
            });
            h.event(egui::Event::PointerMoved(egui::pos2(640.0, 450.0)));
        }
        let t = Instant::now();
        h.step();
        frames.push(t.elapsed().as_secs_f64() * 1000.0);
        peak = peak.max(h.state().tab().preview.media.lock().unwrap().bytes());
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    let scroll_s = start.elapsed().as_secs_f64();
    let (decoded, evicted, bytes) = {
        let m = h.state().tab().preview.media.lock().unwrap();
        (m.decoded, m.evicted, m.bytes())
    };
    frames.sort_by(f64::total_cmp);
    let p = |v: &[f64], q: f64| v[((v.len() as f64 - 1.0) * q) as usize];
    println!(
        "50 pictures (1920×1080): scroll frames p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms over {scroll_s:.1} s; decoded {decoded}, dropped {evicted}; textures now {:.1} MB, peak {:.1} MB (budget {:.0} MB)",
        p(&frames, 0.5),
        p(&frames, 0.95),
        p(&frames, 1.0),
        bytes as f64 / 1048576.0,
        peak as f64 / 1048576.0,
        jarak_gui::media::BUDGET as f64 / 1048576.0
    );
    // Split view: typing while the pictures on screen decode.
    h.event(key(egui::Key::E, egui::Modifiers::CTRL));
    h.step();
    h.event(key(
        egui::Key::E,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    ));
    h.event(key(egui::Key::Home, egui::Modifiers::CTRL));
    h.step();
    let mut typing = Vec::new();
    for i in 0..100 {
        let t = Instant::now();
        h.event(egui::Event::Text(
            if i % 2 == 0 { "a" } else { "한" }.into(),
        ));
        h.step();
        typing.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    typing.sort_by(f64::total_cmp);
    println!(
        "keystroke in split view with pictures: p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        p(&typing, 0.5),
        p(&typing, 0.95),
        p(&typing, 1.0)
    );
    assert!(
        p(&frames, 0.95) < 16.0,
        "scroll p95 {:.2} ms",
        p(&frames, 0.95)
    );
    assert!(
        p(&typing, 0.95) < 16.0,
        "typing p95 {:.2} ms",
        p(&typing, 0.95)
    );
    // On screen at most a few 8 MB pictures: the cache stays near its budget.
    assert!(
        peak <= jarak_gui::media::BUDGET + 4 * 8_294_400,
        "peak {peak}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 92 (#111): Vim editing in the 10 MB document — Normal-mode motions and operators
/// (`j w e b x dw u .`) each in one frame (`JARAK-NFR-002`: ≤ 16 ms p95).
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn vim_keys_in_a_10_mb_document() {
    let line = "The quick brown fox 한글 문장 jumps over the lazy dog 0123456789\r\n";
    let text = line.repeat(10 * 1024 * 1024 / line.len() + 1);
    let dir = std::env::temp_dir().join(format!("jarak-perf-vim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ten.txt");
    std::fs::write(&file, &text).unwrap();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(Lang::En, &[format!("{}:70000", file.display())], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    h.state_mut().editing = jarak_gui::Editing::Vim;
    h.run();
    let keys = ["j", "w", "e", "b", "x", "d", "w", "u", ".", "j", "w", "x"];
    let mut times = Vec::new();
    for i in 0..300 {
        let k = keys[i % keys.len()];
        let t = Instant::now();
        h.event(egui::Event::Text(k.into()));
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "Vim keys (10 MB): p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        p(0.5),
        p(0.95),
        p(1.0)
    );
    assert!(p(0.95) < 16.0, "p95 {:.2} ms", p(0.95));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 98 (#117): a keystroke frame in a 10 MB Rust file with highlighting, after the first jump
/// to its end (which lexes every line once). `cargo test --release -p jarak-gui --test perf
/// highlighted -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn a_highlighted_keystroke_frame_in_a_10_mb_rust_file() {
    let unit = "/* block\n   comment */\nfn main() { let s = \"한글 string\"; // line comment\n    let n = 0x1F + 42; }\n";
    let text = unit.repeat(10 * 1024 * 1024 / unit.len() + 1);
    let dir = std::env::temp_dir().join(format!("jarak-perf-rs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ten.rs");
    std::fs::write(&file, &text).unwrap();
    let lines = text.lines().count();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let t = Instant::now();
    let app = JarakApp::new(Lang::En, &[format!("{}:{lines}", file.display())], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    let first_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(!h.state().tab().view.syntax.is_empty(), "highlighted");
    let mut times = Vec::new();
    for i in 0..300 {
        let ev = if i % 10 == 9 {
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }
        } else {
            egui::Event::Text(if i % 2 == 0 { "a" } else { "한" }.into())
        };
        let t = Instant::now();
        h.event(ev);
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "highlighted 10 MB Rust ({lines} lines): opening (reading 10 MB, lexing to the end, the first frames) {first_ms:.0} ms; keystroke frame p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        p(0.5),
        p(0.95),
        p(1.0)
    );
    assert!(p(0.95) < 16.0, "p95 {:.2} ms", p(0.95));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 99 (#118): folding and the Function List in a 10 MB Rust file — the structure is found on
/// a worker thread after a pause in typing, so keystroke frames stay within budget; then every
/// fold folded (Alt+0) and moves through the folded text. `cargo test --release -p jarak-gui
/// --test perf folded -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn folded_structure_of_a_10_mb_rust_file() {
    let unit = "/* block\n   comment */\nfn main() {\n    let s = \"한글 string\"; // line comment\n    if s.is_empty() {\n        return;\n    }\n}\n";
    let text = unit.repeat(10 * 1024 * 1024 / unit.len() + 1);
    let dir = std::env::temp_dir().join(format!("jarak-perf-fold-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ten.rs");
    std::fs::write(&file, &text).unwrap();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(Lang::En, &[format!("{}:1000", file.display())], incoming);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.state_mut().function_list = true;
    h.run();
    // The first structure: wait for the worker.
    let t = Instant::now();
    while h.state().tab().outline.folds.is_empty() || h.state().tab().outline.symbols.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.step();
        assert!(t.elapsed().as_secs() < 30, "the structure arrives");
    }
    let first_ms = t.elapsed().as_secs_f64() * 1000.0;
    let folds = h.state().tab().outline.folds.len();
    let symbols = h.state().tab().outline.symbols.len();
    // Typing: every keystroke frame, while the structure waits for a pause.
    let mut times = Vec::new();
    for _ in 0..300 {
        let t = Instant::now();
        h.event(egui::Event::Text("x".into()));
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p = |v: &[f64], q: f64| v[((v.len() as f64 - 1.0) * q) as usize];
    // After the pause, the structure is found again.
    let t = Instant::now();
    let edited = h.state().tab().file.doc.edit_gen();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.step();
        let o = &h.state().tab().outline;
        if o.fresh(edited, "Rust") {
            break;
        }
        assert!(t.elapsed().as_secs() < 30, "found again");
    }
    let again_ms = t.elapsed().as_secs_f64() * 1000.0;
    // Fold everything, then move through the folded text.
    let t = Instant::now();
    h.event(egui::Event::Key {
        key: egui::Key::Num0,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    });
    h.step();
    let fold_all_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut moves = Vec::new();
    for i in 0..200 {
        let t = Instant::now();
        h.event(egui::Event::Key {
            key: if i % 20 < 15 {
                egui::Key::ArrowDown
            } else {
                egui::Key::PageDown
            },
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
        moves.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    moves.sort_by(f64::total_cmp);
    // Typing with every fold folded (the folded folds follow their markers until the structure
    // is found again).
    let mut typing = Vec::new();
    for _ in 0..100 {
        let t = Instant::now();
        h.event(egui::Event::Text("y".into()));
        h.step();
        typing.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    typing.sort_by(f64::total_cmp);
    println!(
        "folds (10 MB Rust): {folds} folds, {symbols} symbols; first structure {first_ms:.0} ms; keystroke p50 {:.2} ms p95 {:.2} ms max {:.2} ms; found again {again_ms:.0} ms after the last key; fold all {fold_all_ms:.1} ms; folded moves p95 {:.2} ms max {:.2} ms; typing with all folded p50 {:.2} ms p95 {:.2} ms",
        p(&times, 0.5),
        p(&times, 0.95),
        p(&times, 1.0),
        p(&moves, 0.95),
        p(&moves, 1.0),
        p(&typing, 0.5),
        p(&typing, 0.95),
    );
    assert!(
        p(&times, 0.95) < 16.0,
        "keystroke p95 {:.2} ms",
        p(&times, 0.95)
    );
    assert!(
        p(&moves, 0.95) < 16.0,
        "folded moves p95 {:.2} ms",
        p(&moves, 0.95)
    );
    assert!(
        p(&typing, 0.95) < 16.0,
        "typing with all folded p95 {:.2} ms",
        p(&typing, 0.95)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 100 (#119): typing with the completion popup, closing brackets and the bracket highlight
/// in a 10 MB Rust file — the popup is found again at every keystroke from the words around the
/// caret, the highlight looks a bounded distance for the partner. `cargo test --release -p
/// jarak-gui --test perf assisted -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run with --release --ignored --nocapture"]
fn assisted_typing_in_a_10_mb_rust_file() {
    let unit =
        "fn main() {\n    let counter = vec![1, 2, 3];\n    if counter.is_empty() { return; }\n}\n";
    let text = unit.repeat(10 * 1024 * 1024 / unit.len() + 1);
    let dir = std::env::temp_dir().join(format!("jarak-perf-assist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ten.rs");
    std::fs::write(&file, &text).unwrap();
    let lines = text.lines().count();
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let app = JarakApp::new(
        Lang::En,
        &[format!("{}:{}", file.display(), lines / 2)],
        incoming,
    );
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.state_mut().assist.auto_close = true;
    h.run();
    let mut times = Vec::new();
    let mut opened = 0;
    let typed = ["c", "o", "u", "(", "x", ")", " ", "\n"];
    for i in 0..400 {
        let s = typed[i % typed.len()];
        let t = Instant::now();
        if s == "\n" {
            h.event(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        } else {
            h.event(egui::Event::Text(s.into()));
        }
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        // JARAK_PERF_SLOW=1 names the slow keystrokes (how the Step 99 worker storm was found).
        if std::env::var_os("JARAK_PERF_SLOW").is_some() && *times.last().unwrap() > 30.0 {
            eprintln!(
                "slow key {i} {s:?}: {:.1} ms (popup {}, hint {})",
                times.last().unwrap(),
                h.state().completion.is_some(),
                h.state().hint.is_some()
            );
        }
        if h.state().completion.is_some() {
            opened += 1;
        }
    }
    times.sort_by(f64::total_cmp);
    let p = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "assisted typing (10 MB Rust): p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms; the popup open after {opened} of 400 keys",
        p(0.5),
        p(0.95),
        p(1.0)
    );
    assert!(opened > 0, "the popup opened");
    assert!(p(0.95) < 16.0, "p95 {:.2} ms", p(0.95));
    let _ = std::fs::remove_dir_all(&dir);
}
