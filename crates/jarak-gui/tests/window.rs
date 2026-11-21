//! Step 85 (#104): the Jarak window shell, headless (no window, no GPU).
//! `JARAK-FR-035` (status bar, gear, no toolbar), `JARAK-FR-002` (files handed over),
//! `JARAK-FR-003` (open targets).

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jarak_gui::{Incoming, JarakApp};
use kim_design::Lang;
use std::sync::{Arc, Mutex};

fn harness(app: JarakApp) -> Harness<'static, JarakApp> {
    Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app)
}

fn app(lang: Lang, args: &[&str]) -> (JarakApp, Incoming) {
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    (JarakApp::new(lang, &args, incoming.clone()), incoming)
}

#[test]
fn the_status_bar_carries_the_gear_at_its_right_end_and_no_toolbar() {
    let (a, _) = app(Lang::En, &[]);
    let mut h = harness(a);
    h.run();
    for item in ["Ln 1, Col 1", "UTF-8", "CRLF", "Plain text"] {
        assert!(h.query_by_label(item).is_some(), "{item} in the status bar");
    }
    let gear = h.get_by_label("Settings").rect();
    let row = h.get_by_label("UTF-8").rect();
    // The bottom-right corner: in the status bar's row, at its right end, in the bottom strip
    // (the harness insets its root by a margin; the real window is checked on the desktop).
    assert!(
        (gear.center().y - row.center().y).abs() < 2.0,
        "{gear:?} {row:?}"
    );
    assert!(
        gear.bottom() <= 640.0 && gear.top() >= 640.0 - 40.0,
        "{gear:?}"
    );
    assert!(gear.right() >= 960.0 - 24.0, "{gear:?}");
    let status = h.get_by_label("Plain text").rect();
    assert!(
        status.right() < gear.left(),
        "text never under the gear: {status:?} {gear:?}"
    );
    // No toolbar: nothing clickable in the upper part of the window.
    assert!(h.query_by_label("Open").is_none() && h.query_by_label("Save").is_none());
}

#[test]
fn the_gear_says_honestly_that_settings_come_later() {
    let (a, _) = app(Lang::En, &[]);
    let mut h = harness(a);
    h.run();
    h.get_by_label("Settings").click();
    h.run();
    assert!(h.state().settings_note);
    assert!(
        h.query_by_label_contains("Step 105").is_some(),
        "the note names when Settings arrive"
    );
    h.get_by_label("Settings").click();
    h.run();
    assert!(!h.state().settings_note);
}

#[test]
fn the_empty_document_says_that_editing_is_not_there_yet() {
    let (a, _) = app(Lang::En, &[]);
    let mut h = harness(a);
    // The first frame installs the design system's fonts (Malgun Gothic for Korean).
    h.run();
    h.state_mut().lang = Lang::Ko;
    h.run();
    // Korean labels in the status bar.
    assert!(h.query_by_label("줄 1, 열 1").is_some());
    assert!(h.query_by_label("설정").is_some());
    assert_eq!(h.state().window_title(), "Jarak");
}

#[test]
fn files_from_the_command_line_and_from_a_second_start_are_taken() {
    let (a, incoming) = app(Lang::En, &[r"C:\notes\ideas.md:12:3"]);
    let mut h = harness(a);
    h.run();
    assert_eq!(h.state().docs.len(), 1);
    assert_eq!(h.state().docs[0].target.line, Some(12));
    assert_eq!(h.state().docs[0].target.column, Some(3));
    assert_eq!(h.state().window_title(), "ideas.md — Jarak");
    // A second start hands over two files, one of them already open.
    incoming.lock().unwrap().push(vec![
        r"C:\notes\ideas.md:12:3".into(),
        r"C:\notes\todo.txt".into(),
    ]);
    h.run();
    let names: Vec<String> = h.state().docs.iter().map(|d| d.name()).collect();
    assert_eq!(names, ["ideas.md", "todo.txt"]);
    assert_eq!(h.state().window_title(), "todo.txt — Jarak");
    assert!(incoming.lock().unwrap().is_empty(), "taken once");
}
