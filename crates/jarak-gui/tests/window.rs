//! Steps 85 and 88 (#104, #107): the Jarak window, headless (no window, no GPU).
//! `JARAK-FR-030…035`, `JARAK-FR-024`, `JARAK-FR-002`, `JARAK-FR-003`, `JARAK-SEC-001`.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jarak_gui::keys::Action;
use jarak_gui::{Dialogs, Incoming, JarakApp, Pending};
use kim_design::Lang;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Dialogs that answer with prepared paths (and count how often they were asked).
#[derive(Default)]
struct Scripted {
    open: Mutex<Vec<PathBuf>>,
    save: Mutex<Vec<PathBuf>>,
    asked: Mutex<usize>,
}

impl Dialogs for Scripted {
    fn open(&self) -> Option<PathBuf> {
        *self.asked.lock().unwrap() += 1;
        self.open.lock().unwrap().pop()
    }
    fn save_as(&self, _suggested: &str) -> Option<PathBuf> {
        *self.asked.lock().unwrap() += 1;
        self.save.lock().unwrap().pop()
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jarak-window-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn app_with(args: &[&str], dialogs: Arc<Scripted>) -> (JarakApp, Incoming) {
    let incoming: Incoming = Arc::new(Mutex::new(Vec::new()));
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    (
        JarakApp::with(
            Lang::En,
            &args,
            incoming.clone(),
            dialogs,
            jarak_core::file::encoding_for_label("euc-kr").unwrap(),
        ),
        incoming,
    )
}

fn harness(app: JarakApp) -> Harness<'static, JarakApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), app);
    h.run();
    h
}

fn key(h: &mut Harness<'static, JarakApp>, key: egui::Key, modifiers: egui::Modifiers) {
    h.event(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    h.run();
}

fn text(h: &mut Harness<'static, JarakApp>, s: &str) {
    h.event(egui::Event::Text(s.into()));
    h.run();
}

fn doc_text(h: &Harness<'static, JarakApp>) -> String {
    h.state().tab().file.doc.text().to_string()
}

#[test]
fn the_status_bar_carries_the_gear_at_its_right_end_and_no_toolbar() {
    let (a, _) = app_with(&[], Arc::default());
    let h = harness(a);
    for item in ["Ln 1, Col 1", "UTF-8", "CRLF", "Plain text", "INS"] {
        assert!(h.query_by_label(item).is_some(), "{item} in the status bar");
    }
    let gear = h.get_by_label("Settings").rect();
    let row = h.get_by_label("UTF-8").rect();
    assert!(
        (gear.center().y - row.center().y).abs() < 2.0,
        "{gear:?} {row:?}"
    );
    assert!(
        gear.bottom() <= 640.0 && gear.top() >= 640.0 - 40.0,
        "{gear:?}"
    );
    assert!(gear.right() >= 960.0 - 24.0, "{gear:?}");
    assert!(h.query_by_label("Open").is_none() && h.query_by_label("Save").is_none());
    assert_eq!(h.state().window_title(), "Jarak");
}

#[test]
fn typing_keys_and_undo_edit_the_document() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "Hello 한글");
    assert_eq!(doc_text(&h), "Hello 한글");
    assert!(
        h.query_by_label("Ln 1, Col 11").is_some(),
        "Hangul counts two columns"
    );
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "x");
    assert_eq!(doc_text(&h), "Hello 한글\r\nx");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowUp, egui::Modifiers::NONE);
    key(&mut h, egui::Key::End, egui::Modifiers::SHIFT);
    assert!(h.query_by_label_contains("selected").is_some());
    key(&mut h, egui::Key::A, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Delete, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "Hello 한글\r\n");
    assert_eq!(h.state().window_title(), "*Untitled — Jarak");
}

#[test]
fn the_ime_composition_is_shown_and_only_the_commit_is_inserted() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.event(egui::Event::Ime(egui::ImeEvent::Preedit {
        text: "ㅎ".into(),
        active_range_chars: None,
    }));
    h.run();
    assert_eq!(h.state().tab().view.preedit, "ㅎ");
    assert_eq!(
        doc_text(&h),
        "",
        "nothing is in the document while composing"
    );
    h.event(egui::Event::Ime(egui::ImeEvent::Preedit {
        text: "한".into(),
        active_range_chars: None,
    }));
    h.event(egui::Event::Ime(egui::ImeEvent::Commit("한".into())));
    h.run();
    assert_eq!(doc_text(&h), "한");
    assert_eq!(h.state().tab().view.preedit, "");
}

#[test]
fn save_as_save_and_the_unsaved_changes_question() {
    let d = temp("save");
    let target = d.join("note.md");
    let dialogs = Arc::new(Scripted::default());
    dialogs.save.lock().unwrap().push(target.clone());
    let (a, _) = app_with(&[], dialogs);
    let mut h = harness(a);
    text(&mut h, "# 메모");
    // Ctrl+S on an untitled document asks for a name.
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# 메모");
    assert_eq!(h.state().window_title(), "note.md — Jarak");
    assert!(h.query_by_label("Markdown").is_some());
    // An edit, then Ctrl+W: the question; Cancel keeps everything.
    text(&mut h, "!");
    key(&mut h, egui::Key::W, egui::Modifiers::CTRL);
    assert_eq!(h.state().pending, Some(Pending::Close(0)));
    assert!(h.query_by_label("Save changes to note.md?").is_some());
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(h.state().pending, None);
    assert_eq!(doc_text(&h), "# 메모!");
    // Save from the question writes the file and closes the document.
    key(&mut h, egui::Key::W, egui::Modifiers::CTRL);
    h.get_by_label("Save").click();
    h.run();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# 메모!");
    assert_eq!(h.state().window_title(), "Jarak", "back to an empty window");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn quitting_asks_for_every_unsaved_document() {
    let d = temp("quit");
    let a_path = d.join("a.txt");
    let b_path = d.join("b.txt");
    std::fs::write(&a_path, "A").unwrap();
    std::fs::write(&b_path, "B").unwrap();
    let (app, _) = app_with(
        &[a_path.to_str().unwrap(), b_path.to_str().unwrap()],
        Arc::default(),
    );
    let mut h = harness(app);
    assert_eq!(h.state().tabs.len(), 2);
    text(&mut h, "2");
    key(&mut h, egui::Key::Tab, egui::Modifiers::CTRL);
    text(&mut h, "1");
    assert!(
        !h.state_mut().request_quit(),
        "unsaved changes keep the window"
    );
    assert_eq!(h.state().pending, Some(Pending::Quit));
    // Don't save the first one asked about, save the second.
    h.state_mut().answer(Some(false));
    assert_eq!(h.state().pending, Some(Pending::Quit));
    h.state_mut().answer(Some(true));
    assert!(h.state().may_quit());
    let a = std::fs::read_to_string(&a_path).unwrap();
    let b = std::fs::read_to_string(&b_path).unwrap();
    assert!(
        (a == "1A" && b == "B") || (a == "A" && b == "2B"),
        "one saved, one not: {a} {b}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn files_open_at_a_line_and_a_second_start_hands_files_over() {
    let d = temp("open");
    let p = d.join("lines.txt");
    std::fs::write(&p, "one\ntwo\nthree 한글\n").unwrap();
    let arg = format!("{}:3:7", p.display());
    let (a, incoming) = app_with(&[&arg], Arc::default());
    let mut h = harness(a);
    assert_eq!(h.state().tabs.len(), 1, "the blank document was replaced");
    assert!(h.query_by_label("Ln 3, Col 7").is_some());
    assert!(h.query_by_label("LF").is_some());
    let q = d.join("new.txt");
    incoming
        .lock()
        .unwrap()
        .push(vec![p.display().to_string(), q.display().to_string()]);
    h.run();
    assert_eq!(h.state().tabs.len(), 2, "an open file is not opened twice");
    assert_eq!(h.state().window_title(), "new.txt — Jarak");
    assert!(!q.exists(), "a missing file is created only on save");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn read_only_documents_refuse_edits_and_say_why() {
    let d = temp("ro");
    let p = d.join("blob.bin");
    std::fs::write(&p, b"PK\x03\x04\x00\x00data").unwrap();
    let (a, _) = app_with(&[p.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    assert!(h.query_by_label("Read-only").is_some());
    text(&mut h, "x");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    assert_eq!(std::fs::read(&p).unwrap(), b"PK\x03\x04\x00\x00data");
    assert!(!h.state().tab().file.doc.is_dirty());
    assert!(h.query_by_label_contains("read-only").is_some());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn zoom_wrap_overwrite_and_korean_labels() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.state_mut().act(Action::ZoomIn);
    h.state_mut().act(Action::ToggleWrap);
    h.state_mut().act(Action::ToggleOverwrite);
    h.run();
    for item in ["110%", "Wrap", "OVR"] {
        assert!(h.query_by_label(item).is_some(), "{item}");
    }
    h.state_mut().lang = Lang::Ko;
    h.run();
    assert!(h.query_by_label("줄 1, 열 1").is_some());
    assert!(h.query_by_label("설정").is_some());
}

#[test]
fn copy_cut_and_paste_use_the_clipboard_with_the_documents_line_endings() {
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a, _) = app_with(&[], Arc::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(
            |ui, app: &mut JarakApp| app.ui(ui),
            a.with_clipboard(clip.clone()),
        );
    h.run();
    text(&mut h, "one two");
    key(&mut h, egui::Key::A, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    use jarak_gui::clipboard::Clipboard as _;
    assert_eq!(clip.get_text().as_deref(), Some("one two"));
    key(&mut h, egui::Key::X, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "");
    // Text with LF pasted into a CRLF document gets CRLF.
    clip.set_text("a\nb");
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "a\r\nb");
    // Without a selection, Copy takes the current line.
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    assert_eq!(clip.get_text().as_deref(), Some("b"));
}

#[test]
fn the_gear_says_honestly_that_settings_come_later() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.get_by_label("Settings").click();
    h.run();
    assert!(h.state().settings_note);
    assert!(h.query_by_label_contains("Step 105").is_some());
}
