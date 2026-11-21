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
