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
    /// Step 106: the export Save dialog and the publishing folder.
    export: Mutex<Vec<PathBuf>>,
    folder: Mutex<Vec<PathBuf>>,
    /// Step 106: the print dialog's answers.
    print: Mutex<Vec<jarak_gui::print_ui::PrintChoice>>,
    /// Step 110: the folders the Open dialog was started in.
    open_dirs: Mutex<Vec<Option<PathBuf>>>,
    /// FILE-02: several files picked in one Open dialog (used before `open`).
    open_many: Mutex<Vec<Vec<PathBuf>>>,
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
    fn save_export(&self, _suggested: &str, _f: jarak_core::export::Format) -> Option<PathBuf> {
        *self.asked.lock().unwrap() += 1;
        self.export.lock().unwrap().pop()
    }
    fn export_folder(&self) -> Option<PathBuf> {
        *self.asked.lock().unwrap() += 1;
        self.folder.lock().unwrap().pop()
    }
    fn print(&self, _has_selection: bool) -> Option<jarak_gui::print_ui::PrintChoice> {
        *self.asked.lock().unwrap() += 1;
        self.print.lock().unwrap().pop()
    }
    fn open_in(&self, dir: Option<&std::path::Path>) -> Option<PathBuf> {
        self.open_dirs
            .lock()
            .unwrap()
            .push(dir.map(std::path::Path::to_path_buf));
        self.open()
    }
    fn open_many(&self, dir: Option<&std::path::Path>) -> Vec<PathBuf> {
        if let Some(many) = self.open_many.lock().unwrap().pop() {
            *self.asked.lock().unwrap() += 1;
            return many;
        }
        self.open_in(dir).into_iter().collect()
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
    // The image decoder (a real thread) may wake the window a few more times.
    let _ = h.try_run();
}

fn text(h: &mut Harness<'static, JarakApp>, s: &str) {
    h.event(egui::Event::Text(s.into()));
    // Background threads (the image decoder, recovery snapshots) may wake the window a few more
    // times under load, as in `key`.
    let _ = h.try_run();
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

/// A link opener that records instead of opening.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl jarak_gui::preview::Opener for Recorder {
    fn open_url(&self, url: &str) -> bool {
        self.0.lock().unwrap().push(url.to_string());
        true
    }
    fn reveal(&self, path: &std::path::Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .push(format!("reveal:{}", path.display()));
        true
    }
    fn open_terminal(&self, dir: &std::path::Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .push(format!("terminal:{}", dir.display()));
        true
    }
    fn open_file(&self, path: &std::path::Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .push(format!("open:{}", path.display()));
        true
    }
    fn new_window(&self, path: &std::path::Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .push(format!("new:{}", path.display()));
        true
    }
    fn recycle(&self, path: &std::path::Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .push(format!("recycle:{}", path.display()));
        true
    }
}

/// Runs frames until the preview drew `n` decoded pictures (the decoder is a real thread).
fn wait_pictures(h: &mut Harness<'static, JarakApp>, n: usize) {
    let t = std::time::Instant::now();
    loop {
        h.step();
        let m = h.state().tab().preview.media.lock().unwrap();
        if (m.ready().len() >= n && m.loading() == 0) || t.elapsed().as_secs() > 20 {
            break;
        }
        drop(m);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let _ = h.try_run();
}

/// A click at `p` with `m` held, through the pointer (hover, press, release).
fn click_at(h: &mut Harness<'static, JarakApp>, p: egui::Pos2, m: egui::Modifiers) {
    h.event(egui::Event::ModifiersChanged(m));
    h.event(egui::Event::PointerMoved(p));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: m,
        });
    }
    h.run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
}

#[test]
fn ctrl_e_switches_to_the_preview_and_back_without_touching_the_document() {
    let d = temp("preview");
    let p = d.join("notes.md");
    let mut src = String::from("# Notes\n\n");
    for i in 0..60 {
        src.push_str(&format!(
            "## Section {i}\n\nParagraph {i} with *emphasis*.\n\n"
        ));
    }
    std::fs::write(&p, &src).unwrap();
    // Open at line 121 (Section 30's heading is line 122, 1-based).
    let (a, _) = app_with(&[&format!("{}:121", p.display())], Arc::default());
    let mut h = harness(a);
    let top_before = h.state().tab().view.top_line;
    assert!(top_before > 0, "the source scrolled to the caret");
    let caret = h.state().tab().file.doc.selection().primary();
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Preview);
    assert!(
        h.query_by_label("Preview").is_some(),
        "the mode in the status bar"
    );
    // The preview starts at the block that holds the source's top line.
    let r = h.state_mut().tabs[0].rendered();
    let block = &r.blocks[h.state().tab().preview.top_block];
    assert!(
        block.lines.contains(&top_before) || block.lines.start >= top_before,
        "{:?} for top line {top_before}",
        block.lines
    );
    // Typing in the preview edits nothing and says how to edit.
    text(&mut h, "x");
    assert!(h.query_by_label_contains("cannot be edited").is_some());
    // Scrolling the preview, then back: the source shows the preview's top block, the caret
    // did not move, the text is byte-identical and clean.
    key(&mut h, egui::Key::PageDown, egui::Modifiers::NONE);
    let preview_line = {
        let r = h.state_mut().tabs[0].rendered();
        r.blocks[h.state().tab().preview.top_block].lines.start
    };
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Source);
    assert_eq!(h.state().tab().view.top_line, preview_line);
    assert_eq!(h.state().tab().file.doc.selection().primary(), caret);
    assert_eq!(doc_text(&h), src);
    assert!(!h.state().tab().file.doc.is_dirty());
    assert_eq!(std::fs::read_to_string(&p).unwrap(), src);
    // Esc in the preview also returns to the source.
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Source);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_split_view_edits_the_source_and_the_preview_follows() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    key(
        &mut h,
        egui::Key::E,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Split);
    assert!(h.query_by_label("Split").is_some());
    text(&mut h, "# Title");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "body");
    // The parse follows the edit (a new history state, a new parse).
    let r = h.state_mut().tabs[0].rendered();
    assert_eq!(r.blocks.len(), 2, "{:?}", r.blocks);
    assert!(
        h.query_by_label("Markdown preview").is_some(),
        "the preview is on screen"
    );
    key(
        &mut h,
        egui::Key::E,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Source);
}

#[test]
fn links_open_only_on_ctrl_click_and_only_web_or_mail() {
    let rec = Arc::new(Recorder::default());
    let (a, _) = app_with(&[], Arc::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(
            |ui, app: &mut JarakApp| app.ui(ui),
            a.with_opener(rec.clone()),
        );
    h.run();
    h.state_mut().open_link("https://example.com/a");
    h.state_mut().open_link("file:///C:/Windows/notepad.exe");
    h.state_mut().open_link("javascript:alert(1)");
    assert_eq!(
        *rec.0.lock().unwrap(),
        vec!["https://example.com/a".to_string()]
    );
    h.run();
    assert!(
        h.query_by_label_contains("web and mail links only")
            .is_some()
    );
}

fn app_clip(
    args: &[&str],
    dialogs: Arc<Scripted>,
) -> (
    Harness<'static, JarakApp>,
    Arc<jarak_gui::clipboard::MemoryClipboard>,
) {
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a, _) = app_with(args, dialogs);
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(
            |ui, app: &mut JarakApp| app.ui(ui),
            a.with_clipboard(clip.clone()),
        );
    h.run();
    (h, clip)
}

fn assets_of(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir.join("assets"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn the_mvp_journey_paste_a_screenshot_save_preview_and_reopen() {
    use jarak_gui::images::{ClipImage, sample_dib};
    let d = temp("mvp");
    let doc = d.join("ideas.md");
    std::fs::write(&doc, "# Ideas\n\n").unwrap();
    let (mut h, clip) = app_clip(&[doc.to_str().unwrap()], Arc::default());
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    clip.set_image(ClipImage::Dib(sample_dib(64, 48)));
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    let (y, m, dd) = jarak_gui::today();
    let name = format!("ideas-{y:04}{m:02}{dd:02}-001.png");
    assert_eq!(assets_of(&d), vec![name.clone()]);
    assert_eq!(doc_text(&h), format!("# Ideas\n\n![image](assets/{name})"));
    assert!(h.query_by_label_contains("Pasted image").is_some());
    // The asset is a real 64 × 48 PNG.
    let png = std::fs::read(d.join("assets").join(&name)).unwrap();
    assert_eq!(jarak_gui::images::png_size(&png).unwrap(), (64, 48));
    // Undo removes the reference, never the file.
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "# Ideas\n\n");
    assert!(d.join("assets").join(&name).exists());
    key(&mut h, egui::Key::Y, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    // The preview shows the image.
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    h.run();
    wait_pictures(&mut h, 1);
    assert_eq!(
        h.state().tab().preview.media.lock().unwrap().ready(),
        vec![[64, 48]]
    );
    // A second paste gets the next number; nothing is overwritten.
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(assets_of(&d).len(), 2);
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    drop(h);
    // Reopen: the document and both images are intact.
    let (mut h2, _) = app_clip(&[doc.to_str().unwrap()], Arc::default());
    key(&mut h2, egui::Key::E, egui::Modifiers::CTRL);
    wait_pictures(&mut h2, 2);
    assert_eq!(
        h2.state().tab().preview.media.lock().unwrap().ready(),
        vec![[64, 48], [64, 48]]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn pasting_an_image_into_an_untitled_document_saves_it_first() {
    use jarak_gui::images::{ClipImage, sample_dib};
    let d = temp("untitled-img");
    let dialogs = Arc::new(Scripted::default());
    dialogs.save.lock().unwrap().push(d.join("new note.md"));
    let (mut h, clip) = app_clip(&[], dialogs);
    text(&mut h, "Shot: ");
    clip.set_image(ClipImage::Dib(sample_dib(8, 8)));
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    let assets = assets_of(&d);
    assert_eq!(assets.len(), 1, "{assets:?}");
    assert!(
        assets[0].starts_with("new-note-"),
        "a portable name: {}",
        assets[0]
    );
    assert!(doc_text(&h).starts_with("Shot: ![image](assets/new-note-"));
    assert_eq!(h.state().tab().file.title(), "new note.md");
    // Cancelling the Save As dialog pastes nothing.
    let (mut h, clip) = app_clip(&[], Arc::default());
    clip.set_image(ClipImage::Dib(sample_dib(8, 8)));
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "");
    assert!(
        h.query_by_label_contains("Save the document first")
            .is_some()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn images_paste_only_into_markdown_and_text_wins_when_both_are_there() {
    use jarak_gui::clipboard::Clipboard as _;
    use jarak_gui::images::{ClipImage, sample_dib};
    let d = temp("img-rules");
    let txt = d.join("plain.txt");
    std::fs::write(&txt, "").unwrap();
    let (mut h, clip) = app_clip(&[txt.to_str().unwrap()], Arc::default());
    clip.set_image(ClipImage::Dib(sample_dib(8, 8)));
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    assert!(assets_of(&d).is_empty());
    assert!(h.query_by_label_contains("Markdown documents").is_some());
    // Text and an image (as office programs copy): the text is pasted.
    let md = d.join("both.md");
    std::fs::write(&md, "").unwrap();
    let (mut h, clip) = app_clip(&[md.to_str().unwrap()], Arc::default());
    clip.set_image(ClipImage::Dib(sample_dib(8, 8)));
    clip.set_text("the text");
    key(&mut h, egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "the text");
    assert!(assets_of(&d).is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_preview_reveals_a_picture_on_ctrl_click_and_labels_missing_ones() {
    use jarak_gui::images::encode_png;
    let d = temp("reveal");
    std::fs::create_dir_all(d.join("assets")).unwrap();
    std::fs::write(
        d.join("assets").join("a.png"),
        encode_png(40, 30, &[0x60u8; 40 * 30 * 4]).unwrap(),
    )
    .unwrap();
    let doc = d.join("n.md");
    std::fs::write(
        &doc,
        "# N\n\n![a](assets/a.png)\n\n![b](assets/missing.png)\n\n![r](https://example.com/r.png)\n\nsee ![a](assets/a.png) inline\n",
    )
    .unwrap();
    let before = std::fs::read(&doc).unwrap();
    let rec = Arc::new(Recorder::default());
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(
            |ui, app: &mut JarakApp| app.ui(ui),
            a.with_opener(rec.clone()),
        );
    h.run();
    // Source: Ctrl+click inside a reference shows the file; elsewhere it does nothing.
    let at = doc_text(&h).find("assets/a.png").unwrap() + 2;
    assert!(h.state_mut().reveal_reference_at(at));
    assert!(!h.state_mut().reveal_reference_at(1));
    assert_eq!(
        *rec.0.lock().unwrap(),
        vec![format!(
            "reveal:{}",
            d.join("assets").join("a.png").display()
        )]
    );
    rec.0.lock().unwrap().clear();
    // Preview: the picture (twice: on its own and inside a paragraph), the missing one labelled
    // with its path, the remote one never loaded.
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    wait_pictures(&mut h, 1);
    let failed = h.state().tab().preview.media.lock().unwrap().failed();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].contains("missing.png"), "{failed:?}");
    let pics = h.state().tab().preview.pictures();
    assert_eq!(pics.len(), 3, "two drawn, one missing frame: {pics:?}");
    let (r, _) = pics[0].clone();
    assert!(
        (r.width() - 40.0).abs() < 0.5 && (r.height() - 30.0).abs() < 0.5,
        "{r:?}"
    );
    // A plain click explains; Ctrl+click shows it in Explorer.
    click_at(&mut h, r.center(), egui::Modifiers::NONE);
    assert!(rec.0.lock().unwrap().is_empty());
    assert!(
        h.query_by_label_contains("Ctrl+click to show it in Explorer")
            .is_some()
    );
    // Windows sets both Ctrl and the command modifier.
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    click_at(&mut h, r.center(), ctrl);
    assert_eq!(
        *rec.0.lock().unwrap(),
        vec![format!(
            "reveal:{}",
            d.join("assets").join("a.png").display()
        )]
    );
    assert_eq!(
        std::fs::read(&doc).unwrap(),
        before,
        "nothing rewrote the document"
    );
    assert!(!h.state().tab().file.doc.is_dirty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_as_to_another_folder_offers_to_copy_the_images_and_never_replaces_files() {
    use jarak_gui::images::encode_png;
    let d = temp("copy-assets");
    let (a_dir, b_dir) = (d.join("a"), d.join("b"));
    std::fs::create_dir_all(a_dir.join("assets")).unwrap();
    std::fs::create_dir_all(b_dir.join("assets")).unwrap();
    let px = |v: u8| encode_png(4, 4, &[v; 64]).unwrap();
    std::fs::write(a_dir.join("assets").join("one.png"), px(1)).unwrap();
    std::fs::write(a_dir.join("assets").join("two.png"), px(2)).unwrap();
    // Already in the new folder with other bytes: never replaced.
    std::fs::write(b_dir.join("assets").join("two.png"), px(9)).unwrap();
    let doc = a_dir.join("n.md");
    std::fs::write(
        &doc,
        "![](assets/one.png)\n![](assets/two.png)\n![](assets/none.png)\n",
    )
    .unwrap();
    let dialogs = Arc::new(Scripted::default());
    // The scripted dialog answers from the end of its list.
    dialogs.save.lock().unwrap().push(b_dir.join("again.md"));
    dialogs.save.lock().unwrap().push(b_dir.join("n.md"));
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs);
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), a);
    h.run();
    key(
        &mut h,
        egui::Key::S,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    // Only one.png is offered (two.png has a file of that name there, none.png does not exist).
    assert_eq!(
        h.state().pending,
        Some(Pending::CopyAssets(vec![(
            a_dir.join("assets").join("one.png"),
            b_dir.join("assets").join("one.png")
        )]))
    );
    assert!(
        h.query_by_label_contains("Copy the document's images")
            .is_some()
    );
    h.get_by_label("Copy").click();
    h.run();
    assert_eq!(h.state().pending, None);
    assert_eq!(
        std::fs::read(b_dir.join("assets").join("one.png")).unwrap(),
        px(1)
    );
    assert_eq!(
        std::fs::read(b_dir.join("assets").join("two.png")).unwrap(),
        px(9),
        "kept"
    );
    assert!(
        a_dir.join("assets").join("one.png").exists(),
        "the originals stay"
    );
    // The same folder again: nothing to offer.
    key(
        &mut h,
        egui::Key::S,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(h.state().pending, None);
    assert_eq!(h.state().tab().file.title(), "again.md");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn copying_never_replaces_a_different_file() {
    let d = temp("copy-new");
    std::fs::write(d.join("src.png"), b"one").unwrap();
    std::fs::write(d.join("other.png"), b"two").unwrap();
    assert!(jarak_gui::copy_new(&d.join("src.png"), &d.join("sub").join("dst.png")).unwrap());
    assert!(
        !jarak_gui::copy_new(&d.join("src.png"), &d.join("sub").join("dst.png")).unwrap(),
        "same bytes"
    );
    let e = jarak_gui::copy_new(&d.join("src.png"), &d.join("other.png")).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(d.join("other.png")).unwrap(), b"two");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_preview_never_scrolls_past_the_end_of_the_document() {
    use jarak_gui::images::encode_png;
    let d = temp("end");
    std::fs::create_dir_all(d.join("assets")).unwrap();
    std::fs::write(
        d.join("assets").join("a.png"),
        encode_png(400, 300, &vec![0x60u8; 400 * 300 * 4]).unwrap(),
    )
    .unwrap();
    let mut text = String::new();
    for i in 0..8 {
        text.push_str(&format!("P{i}\n\n![a](assets/a.png)\n\n"));
    }
    let doc = d.join("n.md");
    std::fs::write(&doc, text).unwrap();
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), a);
    h.run();
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    wait_pictures(&mut h, 1);
    h.event(egui::Event::PointerMoved(egui::pos2(480.0, 300.0)));
    for _ in 0..60 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -500.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    let _ = h.try_run();
    // The last picture is on screen, ending near the bottom of the view.
    let pics = h.state().tab().preview.pictures();
    let last = pics.last().expect("the end of the document is on screen");
    assert!(
        last.0.bottom() > 400.0 && last.0.bottom() < 640.0,
        "{pics:?}"
    );
    // End does the same.
    key(&mut h, egui::Key::Home, egui::Modifiers::NONE);
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    let pics = h.state().tab().preview.pictures();
    assert!(
        pics.last().is_some_and(|p| p.0.bottom() > 400.0),
        "{pics:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn vim_keys(h: &mut Harness<'static, JarakApp>, s: &str) {
    for c in s.chars() {
        h.event(egui::Event::Text(c.to_string()));
    }
    let _ = h.try_run();
}

#[test]
fn a_vim_session_edits_with_motions_operators_visual_and_wq() {
    use jarak_gui::Editing;
    let d = temp("vim");
    let doc = d.join("notes.md");
    std::fs::write(&doc, "one two three\nfour five\nsix\n").unwrap();
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    h.state_mut().set_editing(Editing::Vim);
    h.run();
    assert!(h.query_by_label_contains("Vim mode NORMAL").is_some());
    assert!(
        !h.state().tab().view.ime,
        "keys stay keys in Normal mode (IME off)"
    );
    assert!(h.state().tab().view.block_caret);
    vim_keys(&mut h, "wdw");
    assert_eq!(doc_text(&h), "one three\nfour five\nsix\n");
    // A count shows while it is typed.
    vim_keys(&mut h, "2");
    assert!(h.query_by_label_contains("NORMAL  2").is_some());
    vim_keys(&mut h, "x");
    assert_eq!(doc_text(&h), "one ree\nfour five\nsix\n");
    // Insert mode: the IME composes, typed text goes in, Esc returns.
    vim_keys(&mut h, "A");
    assert!(h.query_by_label_contains("Vim mode INSERT").is_some());
    assert!(h.state().tab().view.ime);
    vim_keys(&mut h, "!한글");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.query_by_label_contains("Vim mode NORMAL").is_some());
    assert_eq!(doc_text(&h), "one ree!한글\nfour five\nsix\n");
    vim_keys(&mut h, "u");
    assert_eq!(
        doc_text(&h),
        "one ree\nfour five\nsix\n",
        "the insert was one undo step"
    );
    key(&mut h, egui::Key::R, egui::Modifiers::CTRL);
    assert_eq!(
        doc_text(&h),
        "one ree!한글\nfour five\nsix\n",
        "Ctrl+R redoes"
    );
    // Visual line: two lines yanked, pasted at the end.
    vim_keys(&mut h, "jVjyGp");
    assert_eq!(
        doc_text(&h),
        "one ree!한글\nfour five\nsix\nfour five\nsix\n"
    );
    // The command line shows in the status bar while it is typed.
    vim_keys(&mut h, ":%s/six/6/g");
    assert!(h.query_by_label_contains(":%s/six/6/g").is_some());
    assert!(h.query_by_label_contains("Vim mode COMMAND").is_some());
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "one ree!한글\nfour five\n6\nfour five\n6\n");
    assert!(
        h.query_by_label_contains("2 substitutions on 2 lines")
            .is_some()
    );
    // Standard keys still work in Vim editing: Ctrl+S saves.
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "one ree!한글\nfour five\n6\nfour five\n6\n"
    );
    // :q with unsaved changes refuses; :wq saves and closes.
    vim_keys(&mut h, "dd:q");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.query_by_label_contains("E37").is_some());
    assert!(!h.state().may_quit());
    vim_keys(&mut h, ":wq");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    // `dd` took the line the substitution left the cursor on (the last one).
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "one ree!한글\nfour five\n6\nfour five\n"
    );
    assert!(h.state().may_quit(), ":wq closes the window");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn vim_search_highlights_matches_and_noh_clears_them() {
    use jarak_gui::Editing;
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "alpha beta alpha");
    h.state_mut().set_editing(Editing::Vim);
    vim_keys(&mut h, "gg/alpha");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(h.state().tab().view.marks, vec![(0, 5), (11, 16)]);
    vim_keys(&mut h, ":noh");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().tab().view.marks.is_empty());
    // Back to Standard: Esc does nothing, the caret is a line again.
    h.state_mut().set_editing(Editing::Standard);
    h.run();
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    text(&mut h, "!");
    assert!(doc_text(&h).contains('!'));
    assert!(!h.state().tab().view.block_caret);
    assert!(h.query_by_label_contains("Vim mode").is_none());
}

#[test]
fn a_read_only_document_stays_unchanged_in_vim() {
    use jarak_gui::Editing;
    let d = temp("vim-ro");
    let f = d.join("bin.dat");
    std::fs::write(&f, b"ab\x00cd\n").unwrap();
    let (a, _) = app_with(&[f.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    assert!(h.state().tab().file.read_only);
    h.state_mut().set_editing(Editing::Vim);
    vim_keys(&mut h, "ddxix");
    assert_eq!(std::fs::read(&f).unwrap(), b"ab\x00cd\n");
    assert!(!h.state().tab().file.doc.is_dirty());
    assert!(
        h.query_by_label_contains("read-only").is_some()
            || h.query_by_label_contains("Read-only").is_some()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_editing_mode_is_remembered_in_the_settings_file() {
    use jarak_gui::Editing;
    let d = temp("settings");
    let file = d.join("settings.toml");
    std::fs::write(&file, "# kept\ntheme = \"dark\"\n").unwrap();
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_settings(file.clone()));
    assert_eq!(h.state().editing, Editing::Standard);
    h.state_mut().set_editing(Editing::Vim);
    let saved = std::fs::read_to_string(&file).unwrap();
    assert_eq!(saved, "# kept\ntheme = \"dark\"\nediting_mode = \"vim\"\n");
    let (b, _) = app_with(&[], Arc::default());
    let h2 = harness(b.with_settings(file));
    assert_eq!(
        h2.state().editing,
        Editing::Vim,
        "a new window starts in Vim editing"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_palette_lists_commands_with_their_keys_and_runs_one() {
    use jarak_gui::Editing;
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    key(
        &mut h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert!(h.state().palette.is_some());
    assert!(h.query_by_label_contains("Save as…").is_some());
    assert!(h.query_by_label_contains("Editing mode: Vim").is_some());
    assert!(
        h.query_by_label_contains("Extend the selection to the line end")
            .is_some()
    );
    // Typing filters (the search field has the focus); Enter runs the first match.
    text(&mut h, "editing vim");
    assert!(h.query_by_label_contains("Save as…").is_none());
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().palette.is_none());
    assert_eq!(h.state().editing, Editing::Vim);
    // Typing in the palette never reached the document.
    assert_eq!(doc_text(&h), "");
    // Esc closes it; a click on an entry runs it.
    key(
        &mut h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().palette.is_none());
    key(
        &mut h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    text(&mut h, "standard");
    h.get_by_label("Editing mode: Standard").click();
    h.run();
    assert_eq!(h.state().editing, Editing::Standard);
}

#[test]
fn alt_alone_shows_the_menu_bar_whose_menus_run_commands() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    assert!(!h.state().menu_bar);
    let alt = egui::Modifiers {
        alt: true,
        ..Default::default()
    };
    // What Windows gives for Alt alone (traced on the desktop): the modifier, the Alt key
    // itself, and back.
    let press_alt = |h: &mut Harness<'static, JarakApp>| {
        h.event(egui::Event::ModifiersChanged(alt));
        h.event(egui::Event::Key {
            key: egui::Key::AltLeft,
            physical_key: Some(egui::Key::AltLeft),
            pressed: true,
            repeat: false,
            modifiers: alt,
        });
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.event(egui::Event::Key {
            key: egui::Key::AltLeft,
            physical_key: Some(egui::Key::AltLeft),
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    };
    press_alt(&mut h);
    assert!(h.state().menu_bar);
    h.get_by_label("View").click();
    h.run();
    assert!(h.query_by_label_contains("Word wrap").is_some());
    h.get_by_label("Word wrap").click();
    h.run();
    assert!(h.state().tab().view.wrap);
    // Alt with a key (Alt+Z) is that key, not the menu bar.
    h.event(egui::Event::ModifiersChanged(alt));
    key(&mut h, egui::Key::Z, alt);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
    assert!(h.state().menu_bar, "still shown");
    assert!(!h.state().tab().view.wrap, "Alt+Z toggled the wrap back");
    press_alt(&mut h);
    assert!(!h.state().menu_bar, "Alt again hides it");
}

#[test]
fn the_assets_dialog_lists_images_and_orphans_and_renames_one() {
    use jarak_gui::images::encode_png;
    let d = temp("assets-dialog");
    std::fs::create_dir_all(d.join("assets")).unwrap();
    let png = encode_png(4, 4, &[9u8; 64]).unwrap();
    std::fs::write(d.join("assets").join("shot.png"), &png).unwrap();
    std::fs::write(d.join("assets").join("old.png"), &png).unwrap();
    let doc = d.join("n.md");
    std::fs::write(&doc, "![a](assets/shot.png)\n![m](assets/missing.png)\n").unwrap();
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let rec = Arc::new(Recorder::default());
    let mut h = harness(a.with_opener(rec));
    h.state_mut().run_entry(jarak_gui::palette::Run::Assets);
    h.run();
    let v = h.state().assets_view.clone().expect("the dialog is open");
    assert_eq!(v.refs.len(), 2);
    assert!(v.refs[0].2.starts_with("png"), "{:?}", v.refs);
    assert_eq!(v.refs[1].2, "missing");
    assert_eq!(v.orphans, vec![d.join("assets").join("old.png")]);
    assert!(h.query_by_label_contains("Orphaned").is_some());
    // Rename through the dialog's button and field.
    h.get_all_by_label("Rename…").next().unwrap().click();
    h.run();
    h.state_mut().rename_asset("assets/shot.png", "login");
    h.run();
    assert!(
        d.join("assets").join("login.png").is_file(),
        "{:?}",
        h.state().message
    );
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "![a](assets/login.png)\n![m](assets/missing.png)\n"
    );
    assert!(
        !h.state().tab().file.doc.is_dirty(),
        "saved with the new reference"
    );
    // A dirty document is not renamed (typing reaches the document once the dialog is closed).
    h.state_mut().assets_view = None;
    h.run();
    text(&mut h, "x");
    assert!(h.state().tab().file.doc.is_dirty());
    h.state_mut().rename_asset("assets/login.png", "again");
    h.run();
    assert!(d.join("assets").join("login.png").is_file());
    assert!(
        h.query_by_label_contains("Save the document first")
            .is_some()
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn files(d: &std::path::Path, names: &[(&str, &str)]) -> Vec<String> {
    names
        .iter()
        .map(|(n, t)| {
            let p = d.join(n);
            std::fs::write(&p, t).unwrap();
            p.to_str().unwrap().to_string()
        })
        .collect()
}

fn titles(h: &Harness<'static, JarakApp>, pane: jarak_gui::views::PaneId) -> Vec<String> {
    let a = h.state();
    a.order(pane)
        .iter()
        .map(|i| a.tabs[*i].file.title())
        .collect()
}

#[test]
fn tabs_switch_reorder_close_and_reopen_by_mouse_and_keys() {
    use jarak_gui::views::PaneId;
    let d = temp("tabs");
    let f = files(&d, &[("a.txt", "A"), ("b.txt", "B"), ("c.txt", "C")]);
    let args: Vec<&str> = f.iter().map(String::as_str).collect();
    let (a, _) = app_with(&args, Arc::default());
    let mut h = harness(a);
    assert_eq!(titles(&h, PaneId::Main), ["a.txt", "b.txt", "c.txt"]);
    assert_eq!(h.state().tab().file.title(), "c.txt");
    // The bar's tabs and their close buttons.
    h.get_by_label("a.txt").click();
    h.run();
    assert_eq!(h.state().tab().file.title(), "a.txt");
    key(&mut h, egui::Key::Tab, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().file.title(), "b.txt");
    key(&mut h, egui::Key::Num9, egui::Modifiers::CTRL);
    assert_eq!(
        h.state().tab().file.title(),
        "c.txt",
        "Ctrl+9 is the last tab"
    );
    key(&mut h, egui::Key::Num1, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().file.title(), "a.txt");
    key(
        &mut h,
        egui::Key::PageDown,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(
        titles(&h, PaneId::Main),
        ["b.txt", "a.txt", "c.txt"],
        "moved right"
    );
    h.get_by_label("Close c.txt").click();
    h.run();
    assert_eq!(titles(&h, PaneId::Main), ["b.txt", "a.txt"]);
    key(&mut h, egui::Key::W, egui::Modifiers::CTRL);
    assert_eq!(titles(&h, PaneId::Main), ["b.txt"]);
    // Ctrl+Shift+T brings back the closed files, newest first; they are recent files too.
    key(
        &mut h,
        egui::Key::T,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(h.state().tab().file.title(), "a.txt");
    key(
        &mut h,
        egui::Key::T,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(titles(&h, PaneId::Main), ["b.txt", "a.txt", "c.txt"]);
    assert!(h.state().recent.iter().any(|p| p.ends_with("c.txt")));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_cloned_document_is_one_buffer_with_a_caret_in_each_view() {
    use jarak_gui::views::{Pane, PaneId};
    let d = temp("clone");
    let f = files(&d, &[("a.txt", "one\ntwo\n"), ("b.txt", "B")]);
    let (a, _) = app_with(&[&f[0], &f[1]], Arc::default());
    let mut h = harness(a);
    h.get_by_label("a.txt").click();
    h.run();
    h.state_mut().clone_to_other();
    h.run();
    assert!(h.state().second.is_some());
    assert!(h.state().second_focused(), "the clone has the focus");
    assert_eq!(titles(&h, PaneId::Second), ["a.txt"]);
    let i = h.state().active;
    assert_eq!(h.state().tabs[i].pane, Pane::Both);
    // Typing in the second view changes the one buffer; each view keeps its caret.
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    text(&mut h, "three");
    assert_eq!(doc_text(&h), "one\ntwo\nthree");
    key(&mut h, egui::Key::F8, egui::Modifiers::NONE);
    assert!(!h.state().second_focused());
    assert_eq!(h.state().tab().file.title(), "a.txt");
    assert_eq!(
        doc_text(&h),
        "one\ntwo\nthree",
        "the main view shows the same text"
    );
    assert_eq!(
        h.state().tab().file.doc.selection().primary().head,
        0,
        "its own caret"
    );
    text(&mut h, ">");
    key(&mut h, egui::Key::F8, egui::Modifiers::NONE);
    assert_eq!(
        h.state().tab().file.doc.selection().primary().head,
        14,
        "the second view's caret came back"
    );
    assert_eq!(doc_text(&h), ">one\ntwo\nthree");
    // One undo history: Ctrl+Z in either view.
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "one\ntwo\nthree");
    // Closing the clone in one view keeps the document in the other; the empty view goes.
    h.state_mut().close_in_pane(i, PaneId::Second);
    h.run();
    assert!(h.state().second.is_none());
    assert_eq!(titles(&h, PaneId::Main), ["a.txt", "b.txt"]);
    assert!(
        h.state().tab().file.doc.is_dirty(),
        "nothing was lost or asked"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn moving_documents_between_the_views() {
    use jarak_gui::views::PaneId;
    let d = temp("move");
    let f = files(&d, &[("a.txt", "A"), ("b.txt", "B"), ("c.txt", "C")]);
    let args: Vec<&str> = f.iter().map(String::as_str).collect();
    let (a, _) = app_with(&args, Arc::default());
    let mut h = harness(a);
    h.state_mut().move_to_other();
    h.run();
    assert_eq!(titles(&h, PaneId::Main), ["a.txt", "b.txt"]);
    assert_eq!(titles(&h, PaneId::Second), ["c.txt"]);
    assert!(h.state().second_focused());
    // A file opened while the second view has the focus opens there.
    let e = files(&d, &[("e.txt", "E")]);
    h.state_mut().open_args(&e);
    h.run();
    assert_eq!(titles(&h, PaneId::Second), ["c.txt", "e.txt"]);
    // Side by side and one above the other.
    h.state_mut().toggle_orientation();
    assert!(!h.state().second.as_ref().unwrap().vertical);
    // Moving the last documents back closes the second view (the focus follows a moved
    // document, so the second view is focused again for the last one).
    h.state_mut().move_to_other();
    h.state_mut().focus_pane(PaneId::Second);
    h.state_mut().move_to_other();
    h.run();
    assert!(h.state().second.is_none());
    assert_eq!(titles(&h, PaneId::Main).len(), 4);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn closing_several_documents_asks_about_each_unsaved_one() {
    use jarak_gui::views::PaneId;
    let d = temp("close-many");
    let f = files(
        &d,
        &[
            ("a.txt", "A"),
            ("b.txt", "B"),
            ("c.txt", "C"),
            ("d.txt", "D"),
        ],
    );
    let args: Vec<&str> = f.iter().map(String::as_str).collect();
    let (a, _) = app_with(&args, Arc::default());
    let mut h = harness(a);
    h.get_by_label("b.txt").click();
    h.run();
    text(&mut h, "!");
    h.get_by_label("d.txt").click();
    h.run();
    h.state_mut().run_entry(jarak_gui::palette::Run::Tab(
        jarak_gui::tabs::Menu::CloseOthers,
    ));
    h.run();
    // a closed; b asks.
    assert!(matches!(h.state().pending, Some(Pending::CloseQueue(..))));
    assert_eq!(h.state().tab().file.title(), "b.txt");
    h.get_by_label("Don't save").click();
    h.run();
    assert_eq!(
        titles(&h, PaneId::Main),
        ["d.txt"],
        "the queue went on after the answer"
    );
    assert_eq!(std::fs::read_to_string(&f[1]).unwrap(), "B", "not saved");
    // Close unchanged keeps the changed ones.
    let more = files(&d, &[("x.txt", "X")]);
    h.state_mut().open_args(&more);
    h.run();
    text(&mut h, "?");
    h.state_mut().run_entry(jarak_gui::palette::Run::Tab(
        jarak_gui::tabs::Menu::CloseUnchanged,
    ));
    h.run();
    assert_eq!(titles(&h, PaneId::Main), ["x.txt"]);
    // Save all saves the changed one.
    h.state_mut()
        .run_entry(jarak_gui::palette::Run::Tab(jarak_gui::tabs::Menu::SaveAll));
    assert_eq!(std::fs::read_to_string(d.join("x.txt")).unwrap(), "?X");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_session_restores_documents_views_carets_and_untitled_text() {
    use jarak_gui::sessions::SessionFiles;
    use jarak_gui::views::PaneId;
    let d = temp("session");
    let f = files(
        &d,
        &[
            ("a.md", "# A\n\nline\n"),
            ("b.txt", "1\n2\n3\n4\n"),
            ("c.txt", "C"),
        ],
    );
    let sf = SessionFiles {
        file: d.join("local").join("session.json"),
        recovery: d.join("local").join("recovery"),
        restore: true,
        save: true,
    };
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_session(sf.clone()));
    h.state_mut().open_args(&f);
    h.run();
    // b: caret on line 3; a: preview, cloned to the second view; an untitled text.
    h.get_by_label("b.txt").click();
    h.run();
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    h.get_by_label("a.md").click();
    h.run();
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    h.state_mut().clone_to_other();
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::F8, egui::Modifiers::NONE);
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    text(&mut h, "keep me");
    // Closing the window: the untitled text is kept without asking.
    assert!(
        h.state_mut().request_quit(),
        "nothing to ask: the untitled text is kept"
    );
    h.state_mut().save_session();
    let saved = std::fs::read_to_string(&sf.file).unwrap();
    assert!(saved.contains("\"schema\": \"jarak.session\""), "{saved}");
    drop(h);
    // A new window restores it.
    let (b, _) = app_with(&[], Arc::default());
    let mut h2 = harness(b.with_session(sf.clone()));
    let st = h2.state();
    assert_eq!(
        titles(&h2, PaneId::Main),
        ["a.md", "b.txt", "c.txt", "new 1"]
            .map(String::from)
            .to_vec()
            .iter()
            .map(|s| if s == "new 1" {
                st.tabs.last().unwrap().file.title()
            } else {
                s.clone()
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(titles(&h2, PaneId::Second), ["a.md"]);
    let b_i = st
        .tabs
        .iter()
        .position(|t| t.file.title() == "b.txt")
        .unwrap();
    assert_eq!(
        st.tabs[b_i]
            .file
            .doc
            .line_col(st.tabs[b_i].file.doc.selection().primary().head)
            .0,
        2
    );
    let a_i = st
        .tabs
        .iter()
        .position(|t| t.file.title() == "a.md")
        .unwrap();
    assert_eq!(
        st.tabs[a_i].mode,
        jarak_gui::Mode::Preview,
        "the main view's preview"
    );
    let untitled = st.tabs.iter().find(|t| t.file.path.is_none()).unwrap();
    assert_eq!(untitled.file.doc.text().to_string(), "keep me");
    h2.run();
    drop(h2);
    // Without restoring, only the recent files come back.
    let (c, _) = app_with(&[], Arc::default());
    let h3 = harness(c.with_session(SessionFiles {
        restore: false,
        save: true,
        ..sf
    }));
    assert_eq!(h3.state().tabs.len(), 1);
    assert!(h3.state().recent.len() >= 3);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn file_commands_rename_recycle_show_and_open_elsewhere() {
    let d = temp("file-cmds");
    let f = files(&d, &[("a.txt", "A"), ("run.bat", "echo hi")]);
    let dialogs = Arc::new(Scripted::default());
    dialogs.save.lock().unwrap().push(d.join("renamed.txt"));
    let rec = Arc::new(Recorder::default());
    let (a, _) = app_with(&[&f[0], &f[1]], dialogs);
    let mut h = harness(a.with_opener(rec.clone()));
    use jarak_gui::palette::Run;
    use jarak_gui::tabs::Menu;
    // A program is never started by association: Explorer shows it.
    h.state_mut().run_entry(Run::Tab(Menu::OpenDefault));
    assert_eq!(
        rec.0.lock().unwrap().last().unwrap(),
        &format!("reveal:{}", f[1])
    );
    h.get_by_label("a.txt").click();
    h.run();
    h.state_mut().run_entry(Run::Tab(Menu::OpenDefault));
    h.state_mut().run_entry(Run::Tab(Menu::OpenTerminal));
    h.state_mut().run_entry(Run::Tab(Menu::NewWindow));
    {
        let r = rec.0.lock().unwrap();
        assert!(r.contains(&format!("open:{}", f[0])));
        assert!(r.contains(&format!("terminal:{}", d.display())));
        assert!(r.contains(&format!("new:{}", f[0])));
    }
    // Rename: the file moves, the document follows.
    h.state_mut().run_entry(Run::Tab(Menu::Rename));
    assert!(d.join("renamed.txt").is_file() && !d.join("a.txt").exists());
    assert_eq!(h.state().tab().file.title(), "renamed.txt");
    // Recycle Bin: asked first, then the document closes.
    h.state_mut().run_entry(Run::Tab(Menu::Recycle));
    h.run();
    assert!(matches!(h.state().pending, Some(Pending::Recycle(_))));
    h.get_by_label("Move to the Recycle Bin").click();
    h.run();
    assert!(
        rec.0
            .lock()
            .unwrap()
            .contains(&format!("recycle:{}", d.join("renamed.txt").display()))
    );
    assert!(
        h.state()
            .tabs
            .iter()
            .all(|t| t.file.title() != "renamed.txt")
    );
    // Copy the path.
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_dropped_folder_becomes_the_workspace_and_the_document_list_shows_all() {
    use jarak_gui::workspace::Side;
    let d = temp("workspace");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    files(&d, &[("one.md", "1"), ("sub/two.txt", "2")]);
    let (a, _) = app_with(&[d.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    assert_eq!(h.state().side, Some(Side::Folder));
    assert_eq!(h.state().workspace.as_ref().unwrap().root, d);
    // Folders first; a click opens a file, another expands a folder.
    h.get_by_label_contains("one.md").click();
    h.run();
    assert_eq!(h.state().tab().file.title(), "one.md");
    h.get_by_label_contains("▸ sub").click();
    h.run();
    h.get_by_label_contains("two.txt").click();
    h.run();
    assert_eq!(h.state().tab().file.title(), "two.txt");
    // The document list and the Documents dialog.
    h.state_mut()
        .run_entry(jarak_gui::palette::Run::Side(Side::Documents));
    h.run();
    h.state_mut().run_entry(jarak_gui::palette::Run::Documents);
    h.run();
    assert!(h.query_by_label_contains("Close checked").is_some());
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "one.md")
        .click();
    h.run();
    h.get_by_label("Close checked").click();
    h.run();
    assert!(h.state().tabs.iter().all(|t| t.file.title() != "one.md"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_session_saved_to_a_file_opens_again_elsewhere() {
    use jarak_gui::views::PaneId;
    let d = temp("session-file");
    let f = files(&d, &[("a.txt", "A"), ("b.md", "# B")]);
    let dialogs = Arc::new(Scripted::default());
    dialogs.save.lock().unwrap().push(d.join("work.json"));
    let (a, _) = app_with(&[&f[0], &f[1]], dialogs);
    let mut h = harness(a);
    h.state_mut().clone_to_other();
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    text(&mut h, "untitled is not saved in a session file");
    h.state_mut()
        .run_entry(jarak_gui::palette::Run::SessionSave);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(d.join("work.json")).unwrap()).unwrap();
    assert_eq!(saved["documents"].as_array().unwrap().len(), 2, "{saved}");
    // Another window opens it.
    let dialogs = Arc::new(Scripted::default());
    dialogs.open.lock().unwrap().push(d.join("work.json"));
    let (b, _) = app_with(&[], dialogs);
    let mut h2 = harness(b);
    h2.state_mut()
        .run_entry(jarak_gui::palette::Run::SessionLoad);
    h2.run();
    assert_eq!(titles(&h2, PaneId::Main), ["a.txt", "b.md"]);
    assert_eq!(titles(&h2, PaneId::Second), ["b.md"]);
    let _ = std::fs::remove_dir_all(&d);
}

fn sel_text(h: &Harness<'static, JarakApp>) -> String {
    h.state().tab().file.doc.selected_text()
}

fn sel_range(h: &Harness<'static, JarakApp>) -> (usize, usize) {
    let r = h.state().tab().file.doc.selection().primary();
    (r.from(), r.to())
}

#[test]
fn the_find_panel_finds_with_every_option_and_f3_repeats() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "Cat cat concat\nthe cat. CAT");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::F, egui::Modifiers::CTRL);
    assert!(h.state().find.open);
    text(&mut h, "cat");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (0, 3), "any case by default");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (4, 7), "Enter again: the box kept the focus");
    key(&mut h, egui::Key::Enter, egui::Modifiers::SHIFT);
    assert_eq!(sel_range(&h), (0, 3), "Shift+Enter: the previous one");
    key(&mut h, egui::Key::F3, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (4, 7));
    // Match case and whole word.
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Match case")
        .click();
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Whole word")
        .click();
    h.run();
    key(&mut h, egui::Key::F3, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (19, 22), "not in concat, not CAT");
    key(&mut h, egui::Key::F3, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (4, 7), "wrapped around");
    assert!(h.query_all_by_label_contains("Wrapped").next().is_some());
    key(&mut h, egui::Key::F3, egui::Modifiers::SHIFT);
    assert_eq!(sel_range(&h), (19, 22), "Shift+F3 goes back");
    // Count.
    h.get_by_label("Count").click();
    h.run();
    assert!(h.query_all_by_label_contains("Matches: 2").next().is_some());
    // Regular expressions and a bad pattern.
    assert!(h.query_by_label("Regular expression").is_some());
    h.state_mut().find.mode = jarak_core::search::Mode::Regex;
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Whole word")
        .click();
    h.run();
    h.state_mut().find.find = "c[a-z]t\\.".into();
    h.get_by_label("Find Next").click();
    h.run();
    assert_eq!(sel_text(&h), "cat.");
    h.state_mut().find.find = "(".into();
    h.get_by_label("Find Next").click();
    h.run();
    assert!(
        h.state()
            .find
            .status
            .as_deref()
            .unwrap()
            .contains("unclosed")
    );
    // Esc closes the panel; F3 still works.
    h.state_mut().find.find = "CAT".into();
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!h.state().find.open);
    key(&mut h, egui::Key::F3, egui::Modifiers::NONE);
    assert_eq!(sel_text(&h), "CAT");
}

#[test]
fn replace_and_replace_all_in_every_mode_and_in_the_selection() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "a=1 b=22 c=333");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::H, egui::Modifiers::CTRL);
    assert_eq!(h.state().find.page, jarak_gui::find::Page::Replace);
    {
        let f = &mut h.state_mut().find;
        f.find = "=".into();
        f.replace = ": ".into();
    }
    // Replace: the first press selects, the next replaces and moves on.
    h.get_all_by_label("Replace").last().unwrap().click();
    h.run();
    assert_eq!(sel_text(&h), "=");
    h.get_all_by_label("Replace").last().unwrap().click();
    h.run();
    assert_eq!(doc_text(&h), "a: 1 b=22 c=333");
    // Replace All with regex groups: one undo step.
    {
        let f = &mut h.state_mut().find;
        f.mode = jarak_core::search::Mode::Regex;
        f.find = r"(\w)=(\d+)".into();
        f.replace = r"\2-$1".into();
    }
    h.get_by_label("Replace All").click();
    h.run();
    assert_eq!(doc_text(&h), "a: 1 22-b 333-c");
    assert!(
        h.query_all_by_label_contains("Replaced: 2")
            .next()
            .is_some()
    );
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "a: 1 b=22 c=333", "one undo step");
    // Extended mode and in selection.
    {
        let f = &mut h.state_mut().find;
        f.mode = jarak_core::search::Mode::Extended;
        f.find = " ".into();
        f.replace = r"\t".into();
        f.in_selection = true;
    }
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(0, 9),
        ));
    h.get_by_label("Replace All").click();
    h.run();
    assert_eq!(
        doc_text(&h),
        "a:\t1\tb=22 c=333",
        "only inside the selection"
    );
}

#[test]
fn mark_all_bookmarks_lines_clears_and_copies_marks() {
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_clipboard(clip.clone()));
    text(&mut h, "TODO one\nnothing\nTODO two TODO\n");
    h.state_mut().open_find(jarak_gui::find::Page::Mark);
    h.state_mut().find.find = "TODO".into();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Bookmark the lines")
        .click();
    h.get_by_label("Mark All").click();
    h.run();
    let d = &h.state().tab().file.doc;
    assert_eq!(d.markers().iter().filter(|m| m.kind == 0).count(), 3);
    assert_eq!(d.bookmarked_lines(), vec![0, 2]);
    assert_eq!(
        h.state().tab().view.bookmarks,
        vec![0, 2],
        "drawn in the gutter"
    );
    assert!(
        h.state().tab().view.styled.len() >= 3,
        "drawn behind the text"
    );
    // Marks move with edits.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, ">>");
    let first = h
        .state()
        .tab()
        .file
        .doc
        .markers()
        .iter()
        .filter(|m| m.kind == 0)
        .map(|m| m.start)
        .min();
    assert_eq!(first, Some(2));
    h.get_by_label("Copy marked").click();
    h.run();
    assert_eq!(
        jarak_gui::clipboard::Clipboard::get_text(clip.as_ref()).as_deref(),
        // Step 103: the text holds only LF line endings, so the document takes LF (as on open).
        Some("TODO\nTODO\nTODO")
    );
    h.get_by_label("Clear marks").click();
    h.run();
    assert!(
        h.state()
            .tab()
            .file
            .doc
            .markers()
            .iter()
            .all(|m| m.kind != 0)
    );
    assert_eq!(
        h.state().tab().file.doc.bookmarked_lines(),
        vec![0, 2],
        "bookmarks stay"
    );
}

#[test]
fn ctrl_f3_takes_the_word_and_ctrl_alt_f3_leaves_the_find_box() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "alpha beta alpha alphabet");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::F3, egui::Modifiers::CTRL);
    assert_eq!(h.state().find.find, "alpha");
    assert_eq!(sel_range(&h), (11, 16));
    h.state_mut().find.find = "zzz".into();
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    let ca = egui::Modifiers {
        ctrl: true,
        alt: true,
        command: true,
        ..Default::default()
    };
    key(&mut h, egui::Key::F3, ca);
    assert_eq!(sel_range(&h), (11, 16), "whole word: not alphabet");
    assert_eq!(h.state().find.find, "zzz", "the find box is untouched");
    key(&mut h, egui::Key::F3, ca);
    assert_eq!(sel_range(&h), (0, 5), "wrapped, never alphabet");
}

#[test]
fn incremental_search_highlight_all_and_smart_highlighting() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "one two one three one");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    let ca = egui::Modifiers {
        ctrl: true,
        alt: true,
        command: true,
        ..Default::default()
    };
    key(&mut h, egui::Key::I, ca);
    assert!(h.state().incremental.is_some());
    text(&mut h, "on");
    assert_eq!(sel_range(&h), (0, 2), "the first match as it is typed");
    text(&mut h, "e t");
    assert_eq!(
        sel_range(&h),
        (0, 5),
        "\"one t\" from where it started: one t[wo]"
    );
    // The matches on screen are highlighted.
    assert!(!h.state().tab().view.marks.is_empty());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().incremental.is_none());
    // Smart highlighting: a selected word marks its other occurrences.
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(4, 7),
        ));
    h.run();
    h.run();
    assert_eq!(
        h.state().tab().view.marks,
        Vec::<(usize, usize)>::new(),
        "two appears once"
    );
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(0, 3),
        ));
    h.run();
    h.run();
    assert_eq!(h.state().tab().view.marks, vec![(8, 11), (18, 21)]);
}

#[test]
fn token_styles_jump_copy_and_clear() {
    use jarak_gui::find::StyleCmd;
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_clipboard(clip.clone()));
    text(&mut h, "red green red blue red");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    h.state_mut().style(StyleCmd::All(1));
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(4, 9),
        ));
    h.state_mut().style(StyleCmd::One(3));
    h.run();
    let kinds: Vec<u8> = h
        .state()
        .tab()
        .file
        .doc
        .markers()
        .iter()
        .map(|m| m.kind)
        .collect();
    assert_eq!(kinds.iter().filter(|k| **k == 1).count(), 3);
    assert_eq!(kinds.iter().filter(|k| **k == 3).count(), 1);
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    h.state_mut().style(StyleCmd::Next);
    assert_eq!(sel_text(&h), "green");
    h.state_mut().style(StyleCmd::Next);
    assert_eq!(sel_range(&h), (10, 13));
    h.state_mut().style(StyleCmd::Copy);
    assert_eq!(
        jarak_gui::clipboard::Clipboard::get_text(clip.as_ref()).as_deref(),
        Some("red\r\ngreen\r\nred\r\nred")
    );
    h.state_mut().style(StyleCmd::Clear(1));
    assert_eq!(marks_and_styles(&h), 1);
    h.state_mut().style(StyleCmd::Clear(0));
    assert_eq!(marks_and_styles(&h), 0);
}

#[test]
fn the_find_history_walks_with_the_arrows_and_lives_in_the_session() {
    use jarak_gui::sessions::SessionFiles;
    let d = temp("find-history");
    let sf = SessionFiles {
        file: d.join("session.json"),
        recovery: d.join("recovery"),
        restore: true,
        save: true,
    };
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_session(sf.clone()));
    text(&mut h, "x y z");
    for w in ["x", "y", "z"] {
        h.state_mut().find.find = w.into();
        key(&mut h, egui::Key::F3, egui::Modifiers::NONE);
    }
    assert_eq!(h.state().find_history, ["z", "y", "x"]);
    key(&mut h, egui::Key::F, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(h.state().find.find, "z");
    key(&mut h, egui::Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(h.state().find.find, "y");
    h.state_mut().save_session();
    drop(h);
    let (b, _) = app_with(&[], Arc::default());
    let h2 = harness(b.with_session(sf));
    assert_eq!(h2.state().find_history, ["z", "y", "x"]);
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 105: the gear opens the Settings window (a click again brings it forward).
#[test]
fn the_gear_opens_the_settings_window() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.get_by_label("Settings").click();
    h.run();
    assert!(h.state().settings_win.is_some());
    assert!(
        h.query_by_label("Appearance").is_some(),
        "the sections are listed"
    );
}

#[test]
fn the_find_panel_has_notepad_plus_plus_alt_mnemonics() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "x=1 X=2 x=3");
    key(&mut h, egui::Key::H, egui::Modifiers::CTRL);
    {
        let f = &mut h.state_mut().find;
        f.find = "x".into();
        f.replace = "y".into();
    }
    key(&mut h, egui::Key::C, egui::Modifiers::ALT);
    assert!(h.state().find.match_case, "Alt+C: match case");
    key(&mut h, egui::Key::T, egui::Modifiers::ALT);
    assert_eq!(
        h.state().find.status.as_deref(),
        Some("Matches: 2"),
        "Alt+T: count"
    );
    key(&mut h, egui::Key::A, egui::Modifiers::ALT);
    assert_eq!(doc_text(&h), "y=1 X=2 y=3", "Alt+A: replace all");
    assert!(!h.state().menu_bar, "Alt+letter does not open the menu bar");
    key(&mut h, egui::Key::W, egui::Modifiers::ALT);
    assert!(h.state().find.whole_word, "Alt+W: whole word");
    h.state_mut().find.page = jarak_gui::find::Page::Mark;
    h.state_mut().find.find = "y".into();
    h.run();
    key(&mut h, egui::Key::M, egui::Modifiers::ALT);
    assert_eq!(marks_and_styles(&h), 2, "Alt+M: mark all");
    // The options and modes.
    let f0 = h.state().find.clone();
    for k in [egui::Key::P, egui::Key::B, egui::Key::I, egui::Key::D] {
        key(&mut h, k, egui::Modifiers::ALT);
    }
    let f = &h.state().find;
    assert_eq!(
        (f.wrap, f.backward, f.in_selection, f.dot_all),
        (!f0.wrap, !f0.backward, !f0.in_selection, !f0.dot_all),
        "Alt+P, Alt+B, Alt+I, Alt+D toggle"
    );
    use jarak_core::search::Mode;
    for (k, m) in [
        (egui::Key::G, Mode::Regex),
        (egui::Key::X, Mode::Extended),
        (egui::Key::N, Mode::Normal),
    ] {
        key(&mut h, k, egui::Modifiers::ALT);
        assert_eq!(h.state().find.mode, m);
    }
}

/// On Windows an Alt+letter chord also arrives as the letter's text (found in the real window):
/// it never types, in the find panel's boxes or in the document; AltGr still types.
fn alt_chord(h: &mut Harness<'static, JarakApp>, k: egui::Key, letter: &str, m: egui::Modifiers) {
    // Both in one frame, as egui-winit delivers one key press.
    let events = &mut h.input_mut().events;
    events.push(egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    });
    events.push(egui::Event::Text(letter.into()));
    let _ = h.try_run();
}

#[test]
fn alt_letters_never_type_their_letter_but_altgr_does() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "x=1 X=2");
    key(&mut h, egui::Key::H, egui::Modifiers::CTRL);
    {
        let f = &mut h.state_mut().find;
        f.find = "x".into();
        f.replace = "y".into();
    }
    alt_chord(&mut h, egui::Key::W, "w", egui::Modifiers::ALT);
    alt_chord(&mut h, egui::Key::A, "a", egui::Modifiers::ALT);
    assert_eq!(h.state().find.find, "x", "the find box got no letter");
    assert_eq!(h.state().find.replace, "y", "the replace box got no letter");
    assert_eq!(doc_text(&h), "y=1 y=2", "Alt+A replaced all (whole word)");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    let wrap = h.state().tab().view.wrap;
    alt_chord(&mut h, egui::Key::Z, "z", egui::Modifiers::ALT);
    assert_eq!(doc_text(&h), "y=1 y=2", "Alt+Z typed nothing");
    assert_ne!(h.state().tab().view.wrap, wrap, "Alt+Z toggled the wrap");
    let altgr = egui::Modifiers {
        ctrl: true,
        alt: true,
        ..Default::default()
    };
    alt_chord(&mut h, egui::Key::E, "€", altgr);
    assert!(doc_text(&h).contains('€'), "AltGr types: {}", doc_text(&h));
}

/// Waits (frame by frame) until the Find / Replace in Files job is done.
fn finish_job(h: &mut Harness<'static, JarakApp>) {
    for _ in 0..400 {
        if h.state().job.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = h.try_run();
    }
    panic!("the job did not finish");
}

fn fixture95(name: &str) -> PathBuf {
    let d = temp(name);
    std::fs::write(d.join("a.txt"), b"one cat\r\ntwo cats\r\n").unwrap();
    std::fs::write(d.join("b.md"), b"# cat\n\nconcat cat\n").unwrap();
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::write(d.join("sub").join("c.txt"), b"deep cat\n").unwrap();
    std::fs::write(d.join("x.bin"), b"cat\0\0bin").unwrap();
    d
}

#[test]
fn find_in_files_lists_results_by_file_and_f4_opens_each_match() {
    let d = fixture95("fif");
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    key(
        &mut h,
        egui::Key::F,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(h.state().find.page, jarak_gui::find::Page::Files);
    {
        let f = &mut h.state_mut().find;
        f.find = "cat".into();
        f.whole_word = true;
        f.dir = d.display().to_string();
        f.filters = "*.txt *.md".into();
    }
    h.get_by_label("Find All").click();
    h.run();
    finish_job(&mut h);
    let res = h.state().results.clone().expect("results");
    assert!(res.open);
    let b = &res.blocks[0];
    assert_eq!(b.hits(), 4, "{}", b.title);
    assert!(
        b.title.contains("4 hits in 3 files of 3 searched"),
        "{}",
        b.title
    );
    let labels: Vec<&str> = b.files.iter().map(|f| f.label.as_str()).collect();
    assert!(
        labels[0].ends_with("a.txt (1 hit)") && labels[2].contains("c.txt"),
        "{labels:?}"
    );
    assert!(
        h.query_all_by_label_contains("Search \"cat\"")
            .next()
            .is_some(),
        "drawn"
    );
    // F4 opens the first match with it selected; again the next one; Shift+F4 goes back.
    key(&mut h, egui::Key::F4, egui::Modifiers::NONE);
    assert!(
        h.state()
            .tab()
            .file
            .path
            .as_ref()
            .unwrap()
            .ends_with("a.txt")
    );
    assert_eq!(h.state().tab().file.doc.selected_text(), "cat");
    key(&mut h, egui::Key::F4, egui::Modifiers::NONE);
    assert!(
        h.state()
            .tab()
            .file
            .path
            .as_ref()
            .unwrap()
            .ends_with("b.md")
    );
    let r = h.state().tab().file.doc.selection().primary();
    assert_eq!((r.from(), r.to()), (2, 5), "# cat");
    key(&mut h, egui::Key::F4, egui::Modifiers::SHIFT);
    assert!(
        h.state()
            .tab()
            .file
            .path
            .as_ref()
            .unwrap()
            .ends_with("a.txt")
    );
    // The last match (sub/c.txt, below the panel's fold) opens like a double-click.
    assert!(
        h.query_by_label_contains("one cat").is_some(),
        "rows are labelled"
    );
    let rows: Vec<_> = h.state().results.as_ref().unwrap().rows();
    let last = *rows.last().unwrap();
    h.state_mut().open_result(last);
    h.run();
    assert!(
        h.state()
            .tab()
            .file
            .path
            .as_ref()
            .unwrap()
            .ends_with("c.txt")
    );
    // Copy lines and paths.
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a2, _) = app_with(&[], Arc::default());
    let mut h2 = harness(a2.with_clipboard(clip.clone()));
    h2.state_mut().results = h.state().results.clone();
    h2.state_mut().copy_results(true);
    let got = jarak_gui::clipboard::Clipboard::get_text(clip.as_ref()).unwrap();
    assert_eq!(got.lines().count(), 3);
    h2.state_mut().copy_results(false);
    let got = jarak_gui::clipboard::Clipboard::get_text(clip.as_ref()).unwrap();
    assert!(
        got.lines().next().unwrap().ends_with("a.txt:1:5: one cat"),
        "{got}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn replace_in_files_previews_applies_only_checked_changes_and_undoes() {
    let d = fixture95("rif");
    let journal = d.join("journal");
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_journal(journal));
    key(
        &mut h,
        egui::Key::F,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    {
        let f = &mut h.state_mut().find;
        f.find = "cat".into();
        f.replace = "dog".into();
        f.whole_word = true;
        f.dir = d.display().to_string();
        f.filters = "*.txt *.md".into();
    }
    h.get_by_label("Replace in Files…").click();
    h.run();
    finish_job(&mut h);
    let p = h.state().preview.clone().expect("the preview");
    assert_eq!(p.chosen(), (4, 3));
    assert_eq!(
        std::fs::read(d.join("a.txt")).unwrap(),
        b"one cat\r\ntwo cats\r\n",
        "nothing written yet"
    );
    // Leave out b.md's first change and the whole sub/c.txt.
    {
        let pv = h.state_mut().preview.as_mut().unwrap();
        let bi = pv
            .plans
            .iter()
            .position(|x| x.path.ends_with("b.md"))
            .unwrap();
        pv.keep[bi][0] = false;
        let ci = pv
            .plans
            .iter()
            .position(|x| x.path.ends_with("c.txt"))
            .unwrap();
        pv.keep[ci].iter_mut().for_each(|k| *k = false);
    }
    h.run();
    h.get_by_label_contains("Replace 2 in 2 files").click();
    h.run();
    finish_job(&mut h);
    assert_eq!(
        std::fs::read(d.join("a.txt")).unwrap(),
        b"one dog\r\ntwo cats\r\n"
    );
    assert_eq!(
        std::fs::read(d.join("b.md")).unwrap(),
        b"# cat\n\nconcat dog\n"
    );
    assert_eq!(std::fs::read(d.join("sub/c.txt")).unwrap(), b"deep cat\n");
    let title = &h.state().results.as_ref().unwrap().blocks[0].title;
    assert!(
        title.starts_with("Replace in Files: 2 replaced in 2 files, 0 skipped"),
        "{title}"
    );
    // Undo the batch.
    h.state_mut().undo_replace_in_files();
    assert_eq!(
        std::fs::read(d.join("a.txt")).unwrap(),
        b"one cat\r\ntwo cats\r\n"
    );
    assert_eq!(
        std::fs::read(d.join("b.md")).unwrap(),
        b"# cat\n\nconcat cat\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn replace_in_files_skips_documents_with_unsaved_changes_and_reloads_saved_ones() {
    let d = fixture95("rif-open");
    let a_path = d.join("a.txt");
    let b_path = d.join("b.md");
    let (ap, bp) = (a_path.display().to_string(), b_path.display().to_string());
    let (a, _) = app_with(&[ap.as_str(), bp.as_str()], Arc::default());
    let mut h = harness(a.with_journal(d.join("j")));
    // b.md (active) gets an unsaved edit; a.txt stays saved.
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    text(&mut h, "unsaved");
    h.state_mut().open_find(jarak_gui::find::Page::Files);
    {
        let f = &mut h.state_mut().find;
        f.find = "cat".into();
        f.replace = "dog".into();
        f.whole_word = true;
        f.dir = d.display().to_string();
        f.filters = "a.txt b.md".into();
    }
    h.state_mut().replace_in_files();
    finish_job(&mut h);
    h.state_mut().apply_preview();
    finish_job(&mut h);
    assert_eq!(std::fs::read(&a_path).unwrap(), b"one dog\r\ntwo cats\r\n");
    assert_eq!(
        std::fs::read(&b_path).unwrap(),
        b"# cat\n\nconcat cat\n",
        "unsaved: skipped"
    );
    let a_tab = h
        .state()
        .tabs
        .iter()
        .find(|t| t.file.path.as_deref() == Some(a_path.as_path()))
        .unwrap();
    assert_eq!(
        a_tab.file.doc.text().to_string(),
        "one dog\r\ntwo cats\r\n",
        "reloaded"
    );
    let title = &h.state().results.as_ref().unwrap().blocks[0].title;
    assert!(title.contains("1 skipped"), "{title}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn find_all_in_documents_and_replace_all_in_open_documents() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "red one red");
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    text(&mut h, "blue red");
    h.state_mut().find.find = "red".into();
    h.state_mut().find_all_docs(false);
    assert_eq!(
        h.state().results.as_ref().unwrap().blocks[0].hits(),
        1,
        "this document"
    );
    h.state_mut().find_all_docs(true);
    let b = &h.state().results.as_ref().unwrap().blocks[0];
    assert_eq!((b.hits(), b.files.len()), (3, 2));
    // The result of an untitled document opens it.
    key(&mut h, egui::Key::F4, egui::Modifiers::NONE);
    assert_eq!(h.state().tab().file.doc.text().to_string(), "red one red");
    assert_eq!(h.state().tab().file.doc.selected_text(), "red");
    h.state_mut().find.replace = "green".into();
    h.state_mut().replace_all_docs();
    let texts: Vec<String> = h
        .state()
        .tabs
        .iter()
        .map(|t| t.file.doc.text().to_string())
        .collect();
    assert_eq!(texts, ["green one green", "blue green"]);
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(
        h.state().tab().file.doc.text().to_string(),
        "red one red",
        "one undo step per document"
    );
}

/// Found on the real desktop (Step 95): with the keyboard in a box of the find panel, the
/// window's shortcuts (the palette, save) still work and the typing stays in the box.
#[test]
fn window_shortcuts_work_while_the_find_panel_has_the_keyboard() {
    let d = temp("panel-keys");
    let f = d.join("k.txt");
    std::fs::write(&f, b"cat\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    key(&mut h, egui::Key::H, egui::Modifiers::CTRL);
    text(&mut h, "x");
    assert_eq!(h.state().find.find, "x", "typing goes to the find box");
    key(
        &mut h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert!(
        h.state().palette.is_some(),
        "Ctrl+Shift+P from the find box"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.state_mut().tab_mut().file.doc.select_all();
    h.state_mut().tab_mut().file.doc.paste("dog\n");
    key(&mut h, egui::Key::F, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(
        std::fs::read(&f).unwrap(),
        b"dog\n",
        "Ctrl+S from the find box"
    );
    assert_eq!(
        h.state().tab().file.doc.text().to_string(),
        "dog\n",
        "nothing typed into the document"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// FIF-03 and FIF-07: Find in Files follows the current document's folder when asked, and a
/// selected result line is copied on its own.
#[test]
fn find_in_files_follows_the_document_folder_and_copies_one_line() {
    let d = fixture95("follow");
    let sub = d.join("sub").join("c.txt");
    let ss = sub.display().to_string();
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let (a, _) = app_with(&[ss.as_str()], Arc::default());
    let mut h = harness(a.with_clipboard(clip.clone()));
    h.state_mut().find.dir = d.display().to_string();
    h.state_mut().find.follow_doc = true;
    h.state_mut().open_find(jarak_gui::find::Page::Files);
    assert_eq!(
        h.state().files_dir().unwrap(),
        d.join("sub"),
        "the document's folder"
    );
    h.state_mut().find.find = "cat".into();
    h.state_mut().find_in_files_now();
    let b = &h.state().results.as_ref().unwrap().blocks[0];
    assert_eq!((b.files.len(), b.hits()), (1, 1), "{}", b.title);
    h.state_mut().result_step(false);
    h.state_mut().copy_selected_result();
    assert_eq!(
        jarak_gui::clipboard::Clipboard::get_text(clip.as_ref()).as_deref(),
        Some("1:6: deep cat")
    );
    h.state_mut().find.follow_doc = false;
    assert_eq!(
        h.state().files_dir().unwrap(),
        d.join("sub"),
        "the field keeps the last folder"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 96: multiple carets, multi-selection and column mode ----------------------------------

/// A primary click with `modifiers` at the document position `pos` of the first view (the
/// pointer goes where the view draws that character).
fn click_at_pos(h: &mut Harness<'static, JarakApp>, pos: usize, modifiers: egui::Modifiers) {
    let p = screen_of(h, pos);
    click_at(h, p, modifiers);
}

fn screen_of(h: &Harness<'static, JarakApp>, pos: usize) -> egui::Pos2 {
    let t = h.state().tab();
    t.view.pos_of_char(&t.file.doc, pos).expect("on screen")
}

/// Ctrl as Windows reports it (both Ctrl and the command modifier).
const CTRL_CMD: egui::Modifiers = egui::Modifiers {
    alt: false,
    ctrl: true,
    shift: false,
    mac_cmd: false,
    command: true,
};

fn texts_of(h: &Harness<'static, JarakApp>) -> Vec<String> {
    let d = &h.state().tab().file.doc;
    d.selection()
        .ranges()
        .iter()
        .map(|r| d.text().slice(r.from()..r.to()).to_string())
        .collect()
}

#[test]
fn ctrl_click_adds_carets_that_type_delete_paste_and_undo_together() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "ab\ncd\nef");
    // Wait a click-count interval between the clicks (no double click).
    // Different lines: never counted as a double click.
    click_at_pos(&mut h, 1, egui::Modifiers::NONE);
    click_at_pos(&mut h, 4, CTRL_CMD);
    click_at_pos(&mut h, 7, CTRL_CMD);
    assert_eq!(h.state().tab().file.doc.selection().len(), 3);
    assert!(
        h.query_all_by_label_contains("3 carets").next().is_some(),
        "the status bar counts them"
    );
    text(&mut h, "한");
    assert_eq!(doc_text(&h), "a한b\nc한d\ne한f");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "ab\ncd\nef");
    // Three lines on the clipboard: one per caret.
    h.state_mut().paste("1\r\n2\r\n3");
    assert_eq!(doc_text(&h), "a1b\nc2d\ne3f");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "a한b\nc한d\ne한f");
    assert_eq!(
        h.state().tab().file.doc.selection().len(),
        3,
        "undo brings the carets back"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert_eq!(
        h.state().tab().file.doc.selection().len(),
        1,
        "Esc: one caret"
    );
}

#[test]
fn ctrl_shift_d_adds_the_next_occurrence_and_alt_f3_selects_all() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "cat Cat cat concat cat");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    let cs = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    key(&mut h, egui::Key::D, cs);
    assert_eq!(texts_of(&h), ["cat"], "the word at the caret");
    key(&mut h, egui::Key::D, cs);
    key(&mut h, egui::Key::D, cs);
    assert_eq!(
        texts_of(&h),
        ["cat", "cat", "cat"],
        "whole word, match case: not Cat, not concat"
    );
    text(&mut h, "dog");
    assert_eq!(doc_text(&h), "dog Cat dog concat dog");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::F3, egui::Modifiers::ALT);
    assert_eq!(h.state().tab().file.doc.selection().len(), 3);
    // Notepad++'s variants from the palette: ignore case and whole word.
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(0, 3),
        ));
    h.state_mut().multi(2);
    assert_eq!(h.state().tab().file.doc.selection().len(), 5);
}

#[test]
fn alt_shift_arrows_make_a_column_block_aligned_by_display_column() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "한글abc\nabcdef\nxy");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    let asf = egui::Modifiers::ALT | egui::Modifiers::SHIFT;
    key(&mut h, egui::Key::ArrowDown, asf);
    key(&mut h, egui::Key::ArrowRight, asf);
    key(&mut h, egui::Key::ArrowRight, asf);
    // From column 2 (after 한) to column 4 on both lines: 글 and "cd".
    assert_eq!(texts_of(&h), ["글", "cd"]);
    text(&mut h, "|");
    assert_eq!(doc_text(&h), "한|abc\nab|ef\nxy");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(texts_of(&h), ["글", "cd"]);
}

#[test]
fn alt_drag_draws_a_column_block_with_the_mouse_and_virtual_space_pads() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "abcdef\nab\nabcdef");
    h.state_mut().virtual_space = true;
    h.run();
    let from = screen_of(&h, 4);
    let to = screen_of(&h, 14);
    let alt = egui::Modifiers::ALT;
    h.event(egui::Event::ModifiersChanged(alt));
    h.event(egui::Event::PointerMoved(from));
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: alt,
    });
    let _ = h.try_run();
    for k in 1..=4 {
        let p = from + (to - from) * (k as f32 / 4.0);
        h.event(egui::Event::PointerMoved(p));
        let _ = h.try_run();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: alt,
    });
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    let _ = h.try_run();
    let b = h
        .state()
        .tab()
        .file
        .doc
        .block()
        .expect("a column selection");
    assert_eq!(
        (b.lines(), b.cols()),
        ((0, 2), (4, 4)),
        "zero width at column 4, lines 1-3"
    );
    assert!(
        !h.state().menu_bar,
        "Alt with the mouse does not open the menu bar"
    );
    text(&mut h, "X");
    assert_eq!(
        doc_text(&h),
        "abcdXef\nab  X\nabcdXef",
        "the short line padded to column 4"
    );
}

#[test]
fn the_column_editor_inserts_text_or_numbers_in_every_row() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "a\nb\nc\nd");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    key(&mut h, egui::Key::C, egui::Modifiers::ALT);
    assert!(h.state().column_editor_open);
    h.get_by_label("Number to insert").click();
    h.run();
    {
        let e = h.state_mut().column_editor.as_mut().unwrap();
        e.initial = "9".into();
        e.leading = jarak_core::document::multi::Leading::Zeros;
    }
    h.run();
    h.get_by_label("Insert").click();
    h.run();
    assert_eq!(doc_text(&h), "a09\nb10\nc11\nd12");
    assert!(!h.state().column_editor_open);
    // Text into a column selection's rows; a bad number is refused with the reason.
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::C, egui::Modifiers::ALT);
    h.state_mut().column_editor.as_mut().unwrap().initial = "x".into();
    h.run();
    h.get_by_label("Insert").click();
    h.run();
    assert!(h.state().column_editor_open, "stays open");
    assert!(h.query_by_label_contains("a whole number").is_some());
    h.get_by_label("Text to insert").click();
    h.run();
    h.state_mut().column_editor.as_mut().unwrap().text = "=".into();
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "a=\nb=\nc=\nd=");
}

#[test]
fn begin_end_select_in_stream_and_column_mode() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "abcd\nefgh\nijkl");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.state_mut().multi(11);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.state_mut().multi(11);
    assert_eq!(texts_of(&h), ["bcd\nef"]);
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.state_mut().multi(12);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.state_mut().multi(12);
    assert_eq!(texts_of(&h), ["bc", "fg", "jk"]);
}

/// Found on the real desktop (Step 96): Ctrl+clicks past the ends of lines add carets at those
/// line ends.
#[test]
fn ctrl_clicks_past_line_ends_add_carets_at_the_ends() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "alpha one\nbeta two\ngamma three\n한글 넷\n");
    h.run();
    let far =
        |h: &Harness<'static, JarakApp>, pos: usize| screen_of(h, pos) + egui::vec2(400.0, 0.0);
    let p1 = far(&h, 0);
    click_at(&mut h, p1, egui::Modifiers::NONE);
    let p2 = far(&h, 10);
    click_at(&mut h, p2, CTRL_CMD);
    let p3 = far(&h, 19);
    click_at(&mut h, p3, CTRL_CMD);
    let heads: Vec<usize> = h
        .state()
        .tab()
        .file
        .doc
        .selection()
        .ranges()
        .iter()
        .map(|r| r.head)
        .collect();
    assert_eq!(heads, [9, 18, 30], "the ends of lines 1, 2 and 3");
    text(&mut h, "!");
    assert_eq!(
        doc_text(&h),
        "alpha one!\nbeta two!\ngamma three!\n한글 넷\n"
    );
}

// ---- Step 97: line operations and text transformations ------------------------------------------

/// Runs the palette's first entry for `query`.
fn palette_run(h: &mut Harness<'static, JarakApp>, query: &str) {
    key(
        h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    text(h, query);
    key(h, egui::Key::Enter, egui::Modifiers::NONE);
}

#[test]
fn notepad_plus_plus_keys_run_the_line_and_case_operations() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "가나다\r\nsecond line\r\nthird");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    let cs = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    key(&mut h, egui::Key::D, egui::Modifiers::CTRL);
    assert_eq!(
        doc_text(&h),
        "가나다\r\n가나다\r\nsecond line\r\nthird",
        "Ctrl+D duplicates"
    );
    key(&mut h, egui::Key::L, cs);
    assert_eq!(
        doc_text(&h),
        "가나다\r\nsecond line\r\nthird",
        "Ctrl+Shift+L deletes"
    );
    key(&mut h, egui::Key::ArrowDown, cs);
    assert_eq!(
        doc_text(&h),
        "second line\r\n가나다\r\nthird",
        "Ctrl+Shift+Down moves"
    );
    key(&mut h, egui::Key::ArrowUp, cs);
    key(&mut h, egui::Key::J, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "가나다 second line\r\nthird", "Ctrl+J joins");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(
        doc_text(&h),
        "가나다\r\nsecond line\r\nthird",
        "one undo step"
    );
    let ca = egui::Modifiers {
        ctrl: true,
        alt: true,
        command: true,
        ..Default::default()
    };
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Enter, ca);
    assert_eq!(
        doc_text(&h),
        "\r\n가나다\r\nsecond line\r\nthird",
        "Ctrl+Alt+Enter: a line above"
    );
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    // Case keys on a selection.
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(5, 16),
        ));
    key(&mut h, egui::Key::U, cs);
    assert_eq!(
        doc_text(&h),
        "가나다\r\nSECOND LINE\r\nthird",
        "Ctrl+Shift+U"
    );
    key(&mut h, egui::Key::U, egui::Modifiers::ALT);
    assert_eq!(
        doc_text(&h),
        "가나다\r\nSecond Line\r\nthird",
        "Alt+U Proper Case"
    );
    key(&mut h, egui::Key::U, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "가나다\r\nsecond line\r\nthird", "Ctrl+U");
}

#[test]
fn the_palette_reaches_sorts_blanks_and_codecs_and_says_when_one_cannot_run() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "10\r\n9 \r\n-1\r\n9 \r\n");
    palette_run(&mut h, "Remove duplicate lines");
    palette_run(&mut h, "Trim trailing space");
    palette_run(&mut h, "Sort lines as integers ascending");
    assert_eq!(doc_text(&h), "-1\r\n9\r\n10\r\n");
    h.state_mut().tab_mut().file.doc.select_all();
    palette_run(&mut h, "Base64 encode");
    assert_eq!(doc_text(&h), "LTENCjkNCjEwDQo=");
    palette_run(&mut h, "Base64 decode");
    assert_eq!(doc_text(&h), "-1\r\n9\r\n10\r\n");
    h.state_mut().tab_mut().file.doc.select_all();
    palette_run(&mut h, "Hex to ASCII");
    assert_eq!(
        doc_text(&h),
        "-1\r\n9\r\n10\r\n",
        "not hex: nothing changed"
    );
    assert!(
        h.query_all_by_label_contains("hex digits").next().is_some(),
        "and it says why"
    );
    // A read-only document refuses the operations.
    h.state_mut().tab_mut().file.read_only = true;
    key(&mut h, egui::Key::D, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "-1\r\n9\r\n10\r\n");
}

#[test]
fn dates_paths_digests_and_the_copy_line_setting() {
    let d = temp("edit-tools");
    let f = d.join("노트.txt");
    std::fs::write(&f, b"abc\n").unwrap();
    let settings = d.join("settings.toml");
    std::fs::write(&settings, "date_format = \"'Y'yyyy\"\n").unwrap();
    let clip = Arc::new(jarak_gui::clipboard::MemoryClipboard::default());
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(
        a.with_clipboard(clip.clone())
            .with_settings(settings.clone()),
    );
    let got = |c: &Arc<jarak_gui::clipboard::MemoryClipboard>| {
        jarak_gui::clipboard::Clipboard::get_text(c.as_ref()).unwrap_or_default()
    };
    // The custom date format from settings.toml.
    h.state_mut().edit_tool(2);
    let year = jarak_gui::hashdlg::now().year;
    assert_eq!(doc_text(&h), format!("Y{year}abc\n"));
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    h.state_mut().edit_tool(0);
    assert!(
        doc_text(&h).len() > "abc\n".len(),
        "the short system date and time"
    );
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    // Paths.
    h.state_mut().edit_tool(3);
    assert_eq!(got(&clip), fs);
    h.state_mut().edit_tool(4);
    assert_eq!(got(&clip), "노트.txt");
    h.state_mut().edit_tool(5);
    assert_eq!(got(&clip), d.display().to_string());
    // Digests of the selection and the dialog.
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(0, 3),
        ));
    h.state_mut().edit_tool(10);
    assert_eq!(
        got(&clip),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    h.state_mut().edit_tool(8);
    assert_eq!(got(&clip), "900150983cd24fb0d6963f7d28e17f72");
    h.state_mut().edit_tool(16);
    h.run();
    assert_eq!(
        h.state().hash_dialog.as_ref().unwrap().text,
        "abc",
        "starts with the selection"
    );
    h.get_by_label("SHA-1").click();
    h.run();
    h.get_by_label("Copy").click();
    h.run();
    assert_eq!(got(&clip), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert!(h.state().hash_dialog.is_none());
    // Copy without a selection: the line, unless the setting is off (kept in settings.toml).
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::point(1));
    jarak_gui::clipboard::Clipboard::set_text(clip.as_ref(), "-");
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    assert_eq!(got(&clip), "abc\n");
    h.state_mut().edit_tool(17);
    jarak_gui::clipboard::Clipboard::set_text(clip.as_ref(), "-");
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    assert_eq!(got(&clip), "-", "nothing copied");
    assert!(
        std::fs::read_to_string(&settings)
            .unwrap()
            .contains("copy_line = false")
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 98: syntax highlighting, the Language menu, user languages, comments ------------------

fn kinds_on(h: &Harness<'static, JarakApp>, line: usize) -> Vec<jarak_core::syntax::Kind> {
    h.state()
        .tab()
        .view
        .syntax
        .iter()
        .find(|(l, _)| *l == line)
        .map(|(_, t)| t.iter().map(|x| x.kind).collect())
        .unwrap_or_default()
}

#[test]
fn a_rust_file_is_highlighted_and_the_language_menu_changes_it() {
    use jarak_core::syntax::Kind as K;
    let d = temp("syntax");
    let f = d.join("main.rs");
    std::fs::write(&f, "fn main() { // 주석\n    let s = \"x\";\n}\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    assert!(
        h.query_by_label("Rust").is_some(),
        "the status bar names the language"
    );
    assert_eq!(
        kinds_on(&h, 0),
        [
            K::Keyword,
            K::Operator,
            K::Operator,
            K::Operator,
            K::Comment
        ]
    );
    assert!(kinds_on(&h, 1).contains(&K::String));
    // Normal text, then Python, from the Language menu (the palette).
    palette_run(&mut h, "Language: Normal text");
    h.run();
    assert!(kinds_on(&h, 0).is_empty());
    assert!(h.query_by_label("Plain text").is_some());
    palette_run(&mut h, "Language: Python");
    h.run();
    assert!(h.query_by_label("Python").is_some());
    assert_eq!(
        kinds_on(&h, 0)[0],
        K::Operator,
        "fn is not a Python keyword; ( is an operator"
    );
    palette_run(&mut h, "Language: detect");
    h.run();
    assert_eq!(kinds_on(&h, 0)[0], K::Keyword);
    // Typing re-highlights from the edited line on.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "/* ");
    h.run();
    assert_eq!(
        kinds_on(&h, 1),
        [K::Comment],
        "the comment now runs over line 2"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn comment_keys_follow_the_language() {
    let d = temp("comments");
    let f = d.join("a.py");
    std::fs::write(&f, "x = 1\ny = 2\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    key(&mut h, egui::Key::A, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Q, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "# x = 1\n# y = 2\n", "Ctrl+Q with Python's #");
    key(&mut h, egui::Key::Q, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "x = 1\ny = 2\n");
    key(&mut h, egui::Key::K, egui::Modifiers::CTRL);
    key(
        &mut h,
        egui::Key::K,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(doc_text(&h), "x = 1\ny = 2\n", "Ctrl+K then Ctrl+Shift+K");
    // CSS has only block comments.
    palette_run(&mut h, "Language: CSS");
    h.run();
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(
        &mut h,
        egui::Key::Q,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(doc_text(&h), "/* x = 1 */\ny = 2\n", "Ctrl+Shift+Q");
    // Plain text has none: it says so.
    palette_run(&mut h, "Language: Normal text");
    key(&mut h, egui::Key::Q, egui::Modifiers::CTRL);
    assert!(
        h.query_all_by_label_contains("no comments")
            .next()
            .is_some()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn user_languages_load_import_from_notepad_plus_plus_and_export() {
    use jarak_core::syntax::Kind as K;
    let d = temp("udl");
    let langs = d.join("languages");
    std::fs::create_dir_all(&langs).unwrap();
    std::fs::write(
        langs.join("kimlang.json"),
        r#"{"name": "Kimlang", "extensions": ["kim"], "line_comment": ["//"], "keywords": ["say"]}"#,
    )
    .unwrap();
    let f = d.join("a.kim");
    std::fs::write(&f, "say 1 // c\n").unwrap();
    let xml = d.join("simple.xml");
    std::fs::write(
        &xml,
        r#"<NotepadPlus><UserLang name="Simple" ext="smp"><KeywordLists><Keywords name="Comments">00#</Keywords><Keywords name="Keywords1">print</Keywords></KeywordLists></UserLang></NotepadPlus>"#,
    )
    .unwrap();
    let dialogs = Arc::new(Scripted::default());
    dialogs.open.lock().unwrap().push(xml);
    dialogs.save.lock().unwrap().push(d.join("out.json"));
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], dialogs);
    let mut h = harness(a.with_languages(langs.clone()));
    h.run();
    assert!(h.query_by_label("Kimlang").is_some());
    assert_eq!(kinds_on(&h, 0), [K::Keyword, K::Number, K::Comment]);
    // Import a Notepad++ UDL: it is kept as JSON and in the Language menu.
    palette_run(&mut h, "Import a Notepad++ user language");
    assert!(langs.join("Simple.json").is_file());
    assert_eq!(h.state().user_langs.len(), 2);
    palette_run(&mut h, "Language: Simple");
    h.run();
    assert!(h.query_by_label("Simple").is_some());
    // Export the current language as JSON.
    palette_run(&mut h, "Export the language");
    let out = std::fs::read_to_string(d.join("out.json")).unwrap();
    assert_eq!(jarak_core::syntax::parse_udl(&out).unwrap().name, "Simple");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 99: folding, the Function List, Go to, matching brackets ----------------------------

/// The find marks and token styles of the active document (not bookmarks, folds, the change
/// history …).
fn marks_and_styles(h: &Harness<'static, JarakApp>) -> usize {
    h.state()
        .tab()
        .file
        .doc
        .markers()
        .iter()
        .filter(|m| m.kind < jarak_core::document::BOOKMARK)
        .count()
}

fn drawn_lines(h: &Harness<'static, JarakApp>) -> Vec<usize> {
    let t = h.state().tab();
    t.view.drawn_lines(&t.file.doc)
}

fn caret_line(h: &Harness<'static, JarakApp>) -> usize {
    let d = &h.state().tab().file.doc;
    d.text().char_to_line(d.selection().primary().head)
}

const RUST: &str =
    "fn main() {\n    let a = 1;\n    if a > 0 {\n        b();\n    }\n}\nfn other() {}\n";

#[test]
fn folding_by_keys_and_the_margin_hides_lines_that_moves_skip() {
    let d = temp("fold");
    let f = d.join("a.rs");
    std::fs::write(&f, RUST).unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    let folds: Vec<(usize, usize, usize)> = h
        .state()
        .tab()
        .outline
        .folds
        .iter()
        .map(|f| (f.start, f.end, f.level))
        .collect();
    assert_eq!(folds, vec![(0, 5, 1), (2, 4, 2)]);
    // Alt+0 folds everything: only the first line of main and the lines after it are drawn.
    key(&mut h, egui::Key::Num0, egui::Modifiers::ALT);
    h.run();
    assert_eq!(drawn_lines(&h), vec![0, 6, 7]);
    // Down from the folded line goes past the fold, up comes back.
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    assert_eq!(caret_line(&h), 6);
    key(&mut h, egui::Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(caret_line(&h), 0);
    // Alt+Shift+0 unfolds everything.
    key(
        &mut h,
        egui::Key::Num0,
        egui::Modifiers::ALT | egui::Modifiers::SHIFT,
    );
    h.run();
    assert_eq!(drawn_lines(&h), vec![0, 1, 2, 3, 4, 5, 6, 7]);
    // Alt+2 folds the second level only; Alt+Shift+2 opens it again.
    key(&mut h, egui::Key::Num2, egui::Modifiers::ALT);
    h.run();
    assert_eq!(drawn_lines(&h), vec![0, 1, 2, 5, 6, 7]);
    key(
        &mut h,
        egui::Key::Num2,
        egui::Modifiers::ALT | egui::Modifiers::SHIFT,
    );
    h.run();
    assert_eq!(drawn_lines(&h).len(), 8);
    // A click in the fold margin beside the `if` line folds it, another opens it; the caret
    // does not move.
    let p = {
        let t = h.state().tab();
        t.view.fold_margin_of(&t.file.doc, 2).expect("drawn")
    };
    click_at(&mut h, p, egui::Modifiers::NONE);
    h.run();
    assert_eq!(
        drawn_lines(&h),
        vec![0, 1, 2, 5, 6, 7],
        "the margin folded it"
    );
    assert_eq!(caret_line(&h), 0, "the caret stayed");
    assert_eq!(
        h.state().tab().outline.marks.as_slice(),
        &[(0, false), (2, true)]
    );
    click_at(&mut h, p, egui::Modifiers::NONE);
    h.run();
    assert_eq!(drawn_lines(&h).len(), 8, "and opened it again");
    // Ctrl+Alt+F inside the `if` folds the innermost fold around the caret and moves the caret
    // to its first line.
    click_at_pos(&mut h, RUST.find("b()").unwrap(), egui::Modifiers::NONE);
    key(
        &mut h,
        egui::Key::F,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    h.run();
    assert_eq!(caret_line(&h), 2);
    assert_eq!(drawn_lines(&h), vec![0, 1, 2, 5, 6, 7]);
    // Typing a line above keeps the fold folded where its text moved.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "// top");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run();
    h.run();
    assert_eq!(h.state().tab().file.doc.folded_lines(), vec![3]);
    assert_eq!(drawn_lines(&h), vec![0, 1, 2, 3, 6, 7, 8]);
    // Ctrl+Alt+Shift+F on the folded line opens it.
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    assert_eq!(caret_line(&h), 3);
    key(
        &mut h,
        egui::Key::F,
        egui::Modifiers::CTRL | egui::Modifiers::ALT | egui::Modifiers::SHIFT,
    );
    h.run();
    assert!(h.state().tab().file.doc.folded_lines().is_empty());
    assert_eq!(drawn_lines(&h).len(), 9);
    // Folding never touched the file.
    assert_eq!(std::fs::read_to_string(&f).unwrap(), RUST);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn go_to_line_and_matching_brackets_open_folds() {
    let d = temp("goto");
    let f = d.join("a.rs");
    std::fs::write(&f, RUST).unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    key(&mut h, egui::Key::Num0, egui::Modifiers::ALT);
    h.run();
    // Ctrl+G to line 4 (inside the folded main): the folds around it open.
    key(&mut h, egui::Key::G, egui::Modifiers::CTRL);
    h.run();
    assert!(h.state().goto.is_some(), "the Go to dialog");
    text(&mut h, "4");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run();
    assert!(h.state().goto.is_none());
    assert_eq!(caret_line(&h), 3);
    assert!(drawn_lines(&h).contains(&3), "the fold opened");
    // From the palette: its Enter is not the dialog's.
    palette_run(&mut h, "Go to line");
    h.run();
    assert!(h.state().goto.is_some(), "open");
    assert!(
        h.query_by_label_contains("A number").is_none(),
        "nothing submitted"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run();
    // Not a number: it says so and stays open; Esc closes it.
    key(&mut h, egui::Key::G, egui::Modifiers::CTRL);
    h.run();
    text(&mut h, "x");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run();
    assert!(h.state().goto.is_some());
    assert!(h.query_by_label_contains("A number").is_some());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run();
    assert!(h.state().goto.is_none());
    // Ctrl+B from main's `{` to its `}`, and back.
    let open = RUST.find('{').unwrap();
    click_at_pos(&mut h, open, egui::Modifiers::NONE);
    key(&mut h, egui::Key::B, egui::Modifiers::CTRL);
    let close = RUST.find("}\nfn other").unwrap();
    assert_eq!(h.state().tab().file.doc.selection().primary().head, close);
    key(&mut h, egui::Key::B, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().file.doc.selection().primary().head, open);
    // Ctrl+Alt+B selects everything between them.
    key(
        &mut h,
        egui::Key::B,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    let r = h.state().tab().file.doc.selection().primary();
    assert_eq!(
        (r.anchor.min(r.head), r.anchor.max(r.head)),
        (open + 1, close)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_function_list_lists_filters_and_jumps_to_symbols() {
    let d = temp("functions");
    let f = d.join("notes.md");
    std::fs::write(
        &f,
        "# 소개\ntext\n## 설치\nmore\n```sh\n# not a heading\n```\n## 사용법\nend\n",
    )
    .unwrap();
    let fs = f.display().to_string();
    let g = d.join("b.rs");
    std::fs::write(&g, RUST).unwrap();
    let gs = g.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    palette_run(&mut h, "Function list");
    h.run();
    assert!(h.state().function_list);
    for row in ["소개  heading 1", "설치  heading 2", "사용법  heading 2"] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    assert!(h.query_by_label_contains("not a heading").is_none());
    // A click on a row goes to it.
    h.get_by_label("사용법  heading 2").click();
    h.run();
    assert_eq!(caret_line(&h), 7);
    // The filter keeps the matching rows.
    h.state_mut().function_filter = "설치".into();
    h.run();
    assert!(h.query_by_label("설치  heading 2").is_some());
    assert!(h.query_by_label("소개  heading 1").is_none());
    h.state_mut().function_filter.clear();
    h.run();
    // Previous / next symbol from the caret.
    palette_run(&mut h, "Previous symbol");
    assert_eq!(caret_line(&h), 2);
    palette_run(&mut h, "Next symbol");
    assert_eq!(caret_line(&h), 7);
    palette_run(&mut h, "Next symbol");
    assert_eq!(caret_line(&h), 7, "none after the last");
    // A Rust file lists its functions: the list follows the active document.
    let (b, _) = app_with(&[gs.as_str()], Arc::default());
    let mut h2 = harness(b);
    h2.run();
    palette_run(&mut h2, "Function list");
    h2.run();
    assert!(h2.query_by_label("main  fn").is_some());
    assert!(h2.query_by_label("other  fn").is_some());
    // Closing the panel.
    h2.get_by_label("Close").click();
    h2.run();
    assert!(!h2.state().function_list);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn python_folds_by_indentation() {
    let d = temp("indentfold");
    let f = d.join("a.py");
    std::fs::write(&f, "class A:\n    def f(self):\n        pass\n\nx = 1\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    key(&mut h, egui::Key::Num1, egui::Modifiers::ALT);
    h.run();
    assert_eq!(
        drawn_lines(&h),
        vec![0, 3, 4, 5],
        "class A folded through `pass`"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 99: the Function List, a folded block's margin and line numbers and the Go to dialog pass
/// the UI audit's rules, measured headless (the geometry this frame really laid out).
#[test]
fn the_structure_views_pass_the_ui_audit_rules() {
    let d = temp("audit99");
    let f = d.join("a.rs");
    std::fs::write(&f, RUST).unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    palette_run(&mut h, "Function list");
    key(&mut h, egui::Key::Num2, egui::Modifiers::ALT);
    h.run();
    for (screen, goto) in [("functions", false), ("goto", true)] {
        if goto {
            key(&mut h, egui::Key::G, egui::Modifiers::CTRL);
            h.run();
        }
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{}",
            kim_design::audit::render(&r, &fails, false)
        );
        let numbers: Vec<&str> = r
            .components
            .iter()
            .filter(|c| c.id.starts_with("gutter:line#"))
            .filter_map(|c| c.text.as_ref().map(|t| t.content.as_str()))
            .collect();
        assert_eq!(
            numbers,
            ["1", "2", "3", "6", "7", "8"],
            "lines 4 and 5 are folded away"
        );
        let rows: Vec<&str> = r
            .components
            .iter()
            .filter(|c| c.id.starts_with("functions:row#"))
            .filter_map(|c| c.text.as_ref().map(|t| t.content.as_str()))
            .collect();
        assert_eq!(rows, ["main  fn", "other  fn"]);
        // The panel's title is in the report too (found by the e2e first run: it was not).
        assert!(
            r.components.iter().any(|c| c.id == "functions:title"
                && c.text
                    .as_ref()
                    .is_some_and(|t| t.content == "Function list")),
            "the Function list title is checked by the audit"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 100: completion, snippets, hints, typing aids, bracket highlight ---------------------

fn open_doc(name: &str, file: &str, content: &str) -> (Harness<'static, JarakApp>, PathBuf) {
    let d = temp(name);
    let f = d.join(file);
    std::fs::write(&f, content).unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    (h, d)
}

fn to_end(h: &mut Harness<'static, JarakApp>) {
    key(h, egui::Key::End, egui::Modifiers::CTRL);
}

fn popup(h: &Harness<'static, JarakApp>) -> Vec<String> {
    h.state()
        .completion
        .as_ref()
        .map(|c| c.items.iter().map(|i| i.text.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn completion_pops_up_while_typing_and_never_takes_enter_unasked() {
    let (mut h, d) = open_doc("complete", "a.rs", "let counter = 1;\n");
    to_end(&mut h);
    text(&mut h, "co");
    assert_eq!(
        popup(&h),
        ["const", "continue", "counter"],
        "keywords and the document's words"
    );
    assert!(
        h.query_by_label("counter  word").is_some(),
        "the popup's row"
    );
    // Enter without choosing: a new line, the popup gone.
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().completion.is_none());
    assert_eq!(doc_text(&h), "let counter = 1;\nco\n");
    // Tab is not taken either (JARAK-FR-094): it indents and closes the popup.
    text(&mut h, "cou");
    assert_eq!(popup(&h), ["counter"]);
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert!(h.state().completion.is_none());
    assert_eq!(doc_text(&h), "let counter = 1;\nco\ncou\t");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    // An entry chosen with an arrow key is taken by Tab.
    text(&mut h, "u");
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "let counter = 1;\nco\ncounter");
    // Down chooses; then Enter takes it.
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "co");
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(doc_text(&h).ends_with("\ncontinue"), "{}", doc_text(&h));
    // Esc closes; Backspace follows the word; a non-word character closes.
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "cou");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    assert_eq!(popup(&h), ["const", "continue", "counter"]);
    text(&mut h, " ");
    assert!(h.state().completion.is_none());
    text(&mut h, "co");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().completion.is_none());
    // Turned off: no popup while typing; Ctrl+Space still asks; Ctrl+Enter only the words.
    palette_run(&mut h, "Completion while typing");
    text(&mut h, "\nco");
    assert!(h.state().completion.is_none());
    key(&mut h, egui::Key::Space, egui::Modifiers::CTRL);
    assert_eq!(popup(&h).len(), 3);
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Enter, egui::Modifiers::CTRL);
    let kinds: Vec<jarak_core::assist::CandKind> = h
        .state()
        .completion
        .as_ref()
        .expect("the words")
        .items
        .iter()
        .map(|i| i.kind)
        .collect();
    assert!(
        !kinds.is_empty()
            && kinds
                .iter()
                .all(|k| *k == jarak_core::assist::CandKind::Word),
        "{kinds:?}"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    // A popup asked for takes Enter at once; one that opened by itself only once set so.
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "co");
    key(&mut h, egui::Key::Space, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(doc_text(&h).ends_with("\nconst"), "{}", doc_text(&h));
    palette_run(&mut h, "Completion while typing");
    palette_run(&mut h, "Enter always accepts");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "co");
    assert!(h.state().completion.is_some(), "on again");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(doc_text(&h).ends_with("\nconst"), "{}", doc_text(&h));
    palette_run(&mut h, "Tab always accepts");
    key(&mut h, egui::Key::Enter, egui::Modifiers::CTRL);
    assert!(h.state().completion.is_none());
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "cou");
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert!(doc_text(&h).ends_with("\ncounter"), "{}", doc_text(&h));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_snippet_expands_from_the_list_and_tab_walks_its_stops() {
    let (mut h, d) = open_doc("snippet", "a.rs", "\n");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "fn");
    assert_eq!(
        popup(&h).first().map(String::as_str),
        Some("fn"),
        "the snippet is offered"
    );
    assert_eq!(
        h.state().completion.as_ref().unwrap().items[0].kind,
        jarak_core::assist::CandKind::Snippet
    );
    // Tab indents a popup that opened by itself; Ctrl+Space asks: one entry expands at once.
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Space, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "fn name() {\n\t\n}\n");
    let r = h.state().tab().file.doc.selection().primary();
    assert_eq!((r.from(), r.to()), (3, 7), "the name is selected");
    text(&mut h, "go");
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert_eq!(h.state().tab().file.doc.selection().primary().head, 6);
    text(&mut h, "x: u8");
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert!(!h.state().tab().file.doc.snippet_active());
    text(&mut h, "x;");
    assert_eq!(doc_text(&h), "fn go(x: u8) {\n\tx;\n}\n");
    // Undo takes back the typing and then the snippet.
    for _ in 0..6 {
        key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    }
    assert_eq!(doc_text(&h), "\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn closers_and_tags_only_when_turned_on_and_smart_indentation() {
    let (mut h, d) = open_doc("aids", "a.rs", "\n");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "(");
    assert_eq!(doc_text(&h), "(\n", "off by default (as in Notepad++)");
    key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
    palette_run(&mut h, "Close brackets and quotes");
    for c in ["f", "(", "\"", "x"] {
        text(&mut h, c);
    }
    assert_eq!(doc_text(&h), "f(\"x\")\n");
    text(&mut h, "\"");
    text(&mut h, ")");
    assert_eq!(doc_text(&h), "f(\"x\")\n", "typed over the closers");
    assert_eq!(h.state().tab().file.doc.selection().primary().head, 6);
    // Smart indentation between braces.
    text(&mut h, " ");
    text(&mut h, "{");
    assert_eq!(doc_text(&h), "f(\"x\") {}\n");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "f(\"x\") {\n\t\n}\n");
    // Keep: the line's indentation only.
    palette_run(&mut h, "Indentation on Enter: keep");
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "f(\"x\") {\n\t\n\t\n}\n");
    let _ = std::fs::remove_dir_all(&d);
    // HTML: tags close once turned on.
    let (mut h, d) = open_doc("tags", "a.html", "\n");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    for c in ["<", "p", ">"] {
        text(&mut h, c);
    }
    assert_eq!(doc_text(&h), "<p>\n");
    key(&mut h, egui::Key::A, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Delete, egui::Modifiers::NONE);
    palette_run(&mut h, "Close HTML/XML tags");
    for c in ["<", "d", "i", "v", ">"] {
        text(&mut h, c);
    }
    assert_eq!(doc_text(&h), "<div></div>");
    assert_eq!(h.state().tab().file.doc.selection().primary().head, 5);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn parameter_hints_follow_the_call_and_matching_brackets_are_highlighted() {
    let (mut h, d) = open_doc("hint", "a.c", "int main(void) {\n}\n");
    click_at_pos(&mut h, 16, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "printf(");
    let hint = h.state().hint.clone().expect("the hint");
    assert_eq!((hint.name.as_str(), hint.arg), ("printf", 0));
    assert!(
        h.query_by_label_contains("int printf(const char *format, ...)")
            .is_some()
    );
    text(&mut h, "\"a, b\", ");
    assert_eq!(
        h.state().hint.as_ref().unwrap().arg,
        1,
        "the comma in the string does not count"
    );
    text(&mut h, ")");
    h.run();
    assert!(h.state().hint.is_none(), "outside the call");
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::NONE);
    key(
        &mut h,
        egui::Key::Space,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert!(h.state().hint.is_some(), "asked for");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().hint.is_none());
    // The bracket after the caret and its partner.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::NONE);
    h.run();
    let text_now = doc_text(&h);
    let open = text_now.find('{').unwrap();
    let close = text_now.rfind('}').unwrap();
    assert_eq!(
        h.state().tab().view.braces,
        vec![(open, open + 1, true), (close, close + 1, true)]
    );
    palette_run(&mut h, "Highlight matching brackets");
    h.run();
    assert!(h.state().tab().view.braces.is_empty(), "turned off");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_users_snippets_and_notepad_plus_plus_api_files_are_read() {
    let d = temp("userassist");
    let cfg = d.join("config");
    std::fs::create_dir_all(cfg.join("autoCompletion")).unwrap();
    std::fs::write(
        cfg.join("snippets.json"),
        r#"{"Python": {"main": {"body": ["def main():", "\t$0"], "description": "mine"}}}"#,
    )
    .unwrap();
    std::fs::write(
        cfg.join("autoCompletion").join("python.xml"),
        r#"<NotepadPlus><AutoComplete language="Python"><KeyWord name="greet" func="yes"><Overload retVal="str" descr="Says hello"><Param name="name"/></Overload></KeyWord></AutoComplete></NotepadPlus>"#,
    )
    .unwrap();
    std::fs::write(cfg.join("autoCompletion").join("broken.xml"), "<x/>").unwrap();
    let f = d.join("a.py");
    std::fs::write(&f, "\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let a = a.with_assist(cfg.clone());
    assert!(
        a.message
            .as_deref()
            .is_some_and(|m| m.contains("broken.xml")),
        "{:?}",
        a.message
    );
    let mut h = harness(a);
    h.run();
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "gre");
    assert_eq!(popup(&h), ["greet"]);
    let item = &h.state().completion.as_ref().unwrap().items[0];
    assert_eq!(
        (item.kind, item.detail.as_str()),
        (jarak_core::assist::CandKind::Function, "str greet(name)")
    );
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    text(&mut h, "(");
    assert_eq!(
        h.state().hint.as_ref().map(|x| x.name.as_str()),
        Some("greet")
    );
    text(&mut h, ")");
    h.run();
    assert!(h.state().hint.is_none());
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "mai");
    assert!(popup(&h).contains(&"main".to_string()));
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Space, egui::Modifiers::CTRL);
    // Python's unit (four spaces) for the snippet's `\t`.
    assert_eq!(doc_text(&h), "greet()\ndef main():\n    \n");
    // The files on disk are untouched.
    assert!(
        std::fs::read_to_string(cfg.join("snippets.json"))
            .unwrap()
            .contains("mine")
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 100: the completion popup and the hint pass the UI audit's rules (headless).
#[test]
fn the_completion_popup_and_the_hint_pass_the_ui_audit_rules() {
    let (mut h, d) = open_doc(
        "audit100",
        "a.c",
        "int counter, count;\nint main(void) {\n}\n",
    );
    click_at_pos(&mut h, 36, egui::Modifiers::NONE);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    text(&mut h, "co");
    for (screen, hint) in [("completion", false), ("hint", true)] {
        if hint {
            key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
            text(&mut h, " ");
            text(&mut h, "printf(");
        }
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{}",
            kim_design::audit::render(&r, &fails, false)
        );
        let want = if hint {
            "hint:signature"
        } else {
            "completion:row#0"
        };
        assert!(r.components.iter().any(|c| c.id == want), "{want}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn paths_complete_tags_match_and_hints_switch_overloads() {
    // Ctrl+Alt+Space: the document's folder.
    let (mut h, d) = open_doc("pathcomp", "notes.md", "\n");
    std::fs::create_dir_all(d.join("images")).unwrap();
    std::fs::write(d.join("index.md"), "").unwrap();
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "![x](im");
    h.state_mut().completion = None;
    key(
        &mut h,
        egui::Key::Space,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    assert!(
        h.state().completion.is_none(),
        "one entry: inserted at once"
    );
    assert_eq!(doc_text(&h), "![x](images\\\n");
    // Into the folder: nothing there yet says so.
    key(
        &mut h,
        egui::Key::Space,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    assert!(h.query_by_label_contains("No completion").is_some());
    let _ = std::fs::remove_dir_all(&d);
    // HTML: the tag at the caret and its partner.
    let (mut h, d) = open_doc("tagmatch", "a.html", "<div>\n  <p>x</p>\n</div>\n");
    click_at_pos(&mut h, 2, egui::Modifiers::NONE);
    h.run();
    assert_eq!(
        h.state().tab().view.braces,
        vec![(1, 4, true), (19, 22, true)]
    );
    let _ = std::fs::remove_dir_all(&d);
    // Python's range has two overloads: Alt+Down shows the next one.
    let (mut h, d) = open_doc("overloads", "a.py", "\n");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    text(&mut h, "range(");
    assert!(h.query_by_label_contains("range range(stop)").is_some());
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::ALT);
    assert_eq!(h.state().hint.as_ref().unwrap().overload, 1);
    assert!(
        h.query_by_label_contains("range(start, stop, step=1)")
            .is_some()
    );
    key(&mut h, egui::Key::ArrowUp, egui::Modifiers::ALT);
    assert_eq!(h.state().hint.as_ref().unwrap().overload, 0);
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 99, found by Step 100's measurement: typing in a large document before its first
/// structure arrived started a new worker every frame (each one lexing the whole text). One runs
/// at a time; its result is taken even though the text changed meanwhile.
#[test]
fn typing_before_the_first_structure_starts_one_worker_at_a_time() {
    let unit = "fn f() {\n    x();\n}\n";
    let (mut h, d) = open_doc("onejob", "big.rs", &unit.repeat(3000));
    for _ in 0..40 {
        h.event(egui::Event::Text("y".into()));
        h.step();
    }
    let started_while_typing = h.state().tab().outline.jobs_started;
    assert!(
        started_while_typing <= 2,
        "{started_while_typing} workers for 40 keys"
    );
    // The structure arrives (from the worker that was running), then again after the pause.
    let t = std::time::Instant::now();
    while h.state().tab().outline.folds.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
        assert!(t.elapsed().as_secs() < 20, "the structure arrives");
    }
    assert!(h.state().tab().outline.folds.len() >= 2999);
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 101: bookmarks, the change history, hidden lines, the map, history, quick open ------

#[test]
fn bookmark_keys_and_the_bookmarked_lines_with_the_clipboard() {
    use jarak_gui::clipboard::Clipboard as _;
    let d = temp("bookmarks");
    let f = d.join("a.txt");
    std::fs::write(&f, "one\ntwo\nthree\nfour\n").unwrap();
    let fs = f.display().to_string();
    let (mut h, clip) = app_clip(&[fs.as_str()], Arc::default());
    // Ctrl+F2 on lines 2 and 4, F2 / Shift+F2 between them.
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::F2, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::F2, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().file.doc.bookmarked_lines(), vec![1, 3]);
    h.run();
    assert_eq!(
        h.state().tab().view.bookmarks,
        vec![1, 3],
        "drawn in the margin"
    );
    key(&mut h, egui::Key::F2, egui::Modifiers::NONE);
    assert_eq!(caret_line(&h), 1, "wrapped to the first");
    key(&mut h, egui::Key::F2, egui::Modifiers::SHIFT);
    assert_eq!(caret_line(&h), 3);
    // Copy, then paste into the bookmarked lines; cut them.
    palette_run(&mut h, "Copy the bookmarked lines");
    assert_eq!(clip.get_text().as_deref(), Some("two\nfour\n"));
    clip.set_text("2\n4");
    palette_run(&mut h, "Paste to (replace) the bookmarked lines");
    assert_eq!(doc_text(&h), "one\n2\nthree\n4\n");
    palette_run(&mut h, "Cut the bookmarked lines");
    assert_eq!(doc_text(&h), "one\nthree\n");
    assert_eq!(clip.get_text().as_deref(), Some("2\n4\n"));
    // Bookmarks are not edits: a read-only document takes them.
    h.state_mut().tab_mut().file.read_only = true;
    key(&mut h, egui::Key::F2, egui::Modifiers::CTRL);
    assert!(!h.state().tab().file.doc.bookmarked_lines().is_empty());
    palette_run(&mut h, "Remove the bookmarked lines");
    assert_eq!(
        doc_text(&h),
        "one\nthree\n",
        "but removing lines is refused"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_change_history_shows_in_the_margin_and_hidden_lines_come_back() {
    let (mut h, d) = open_doc("changes", "a.txt", "a\nb\nc\nd\ne\n");
    h.run();
    assert!(h.state().tab().view.changes.is_empty());
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    text(&mut h, "x");
    h.run();
    use jarak_core::document::navigate::Change;
    assert_eq!(h.state().tab().view.changes, vec![(1, Change::Modified)]);
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    h.run();
    assert_eq!(
        h.state().tab().view.changes,
        vec![(1, Change::Saved)],
        "saved"
    );
    // Next change from the top.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    palette_run(&mut h, "Go to the next change");
    assert_eq!(caret_line(&h), 1);
    // Alt+H hides the selected lines; the line above shows a mark; a click on it shows them.
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::SHIFT);
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::SHIFT);
    // (ending at the start of line 5, which is not taken)
    key(&mut h, egui::Key::H, egui::Modifiers::ALT);
    h.run();
    assert_eq!(drawn_lines(&h), vec![0, 1, 4, 5], "lines 3 and 4 hidden");
    assert_eq!(caret_line(&h), 1);
    let p = {
        let t = h.state().tab();
        t.view.fold_margin_of(&t.file.doc, 1).expect("drawn")
    };
    click_at(&mut h, p, egui::Modifiers::NONE);
    h.run();
    assert_eq!(drawn_lines(&h).len(), 6, "shown again");
    // Hiding does not change the text.
    assert_eq!(doc_text(&h), "a\nxb\nc\nd\ne\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_document_map_scrolls_and_the_navigation_history_goes_back_and_forward() {
    let text: String = (0..2000).map(|i| format!("line {i}\n")).collect();
    let (mut h, d) = open_doc("docmap", "a.txt", &text);
    palette_run(&mut h, "Document Map");
    h.run();
    assert!(h.state().doc_map);
    let map = h.get_by_label("Document map");
    let r = map.rect();
    // A click near the map's bottom scrolls there.
    click_at(
        &mut h,
        egui::pos2(r.center().x, r.bottom() - 4.0),
        egui::Modifiers::NONE,
    );
    h.run();
    assert!(
        h.state().tab().view.top_line > 1500,
        "{}",
        h.state().tab().view.top_line
    );
    // Ctrl+End is a jump: Alt+Left comes back, Alt+Right goes there again.
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    h.run();
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    h.run();
    assert_eq!(caret_line(&h), 2000);
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::ALT);
    h.run();
    assert_eq!(caret_line(&h), 0, "back");
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::ALT);
    h.run();
    assert_eq!(caret_line(&h), 2000, "forward");
    key(&mut h, egui::Key::ArrowRight, egui::Modifiers::ALT);
    assert!(h.query_by_label_contains("Nothing to go forward").is_some());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn quick_open_finds_open_documents_and_the_workspaces_files() {
    let d = temp("quickopen");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::create_dir_all(d.join("target")).unwrap();
    std::fs::write(d.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(d.join("README.md"), "# r\n").unwrap();
    std::fs::write(d.join("target").join("skip.rs"), "").unwrap();
    let f = d.join("notes.txt");
    std::fs::write(&f, "n\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.state_mut().open_workspace(&d);
    h.run();
    key(&mut h, egui::Key::P, egui::Modifiers::CTRL);
    // The walk runs on a worker: wait for it.
    let t = std::time::Instant::now();
    while h.query_by_label("main.rs").is_none() {
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.run();
        assert!(t.elapsed().as_secs() < 10, "the files arrive");
    }
    assert!(h.query_by_label("notes.txt").is_some(), "the open document");
    assert!(h.query_by_label("skip.rs").is_none(), "target/ is skipped");
    // A fuzzy query and Enter open the file.
    text(&mut h, "mnrs");
    assert!(h.query_by_label("README.md").is_none());
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run();
    assert!(h.state().quick.is_none());
    assert_eq!(h.state().tab().file.title(), "main.rs");
    assert_eq!(doc_text(&h), "fn main() {}\n");
    // The Summary of it.
    palette_run(&mut h, "Summary");
    h.run();
    assert!(h.query_by_label("Words").is_some());
    assert!(h.query_by_label("13").is_some(), "the bytes of the file");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run();
    assert!(!h.state().summary_open);
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 101: the map, quick open and the Summary pass the UI audit's rules (headless).
#[test]
fn the_map_quick_open_and_the_summary_pass_the_ui_audit_rules() {
    let (mut h, d) = open_doc("audit101", "a.rs", RUST);
    palette_run(&mut h, "Document Map");
    for screen in ["docmap", "quick-open", "summary"] {
        match screen {
            "quick-open" => key(&mut h, egui::Key::P, egui::Modifiers::CTRL),
            "summary" => {
                key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
                palette_run(&mut h, "Summary");
            }
            _ => {}
        }
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        if screen == "summary" {
            assert!(
                r.components
                    .iter()
                    .any(|c| c.id.starts_with("summary:Words"))
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 102: macros, run commands, the browser ----------------------------------------------

/// A program runner that records instead of starting anything (or refuses, as for a program that
/// needs administrator rights).
#[derive(Default)]
struct FakeRunner {
    ran: Mutex<Vec<(String, String)>>,
    refuse: Option<String>,
}

impl jarak_gui::macros::Runner for FakeRunner {
    fn run(&self, program: &str, args: &str, _dir: Option<&std::path::Path>) -> Result<(), String> {
        if let Some(e) = &self.refuse {
            return Err(e.clone());
        }
        self.ran.lock().unwrap().push((program.into(), args.into()));
        Ok(())
    }
}

fn macro_app(
    name: &str,
    file: &str,
    content: &str,
) -> (
    Harness<'static, JarakApp>,
    PathBuf,
    Arc<FakeRunner>,
    Arc<Recorder>,
) {
    let d = temp(name);
    let f = d.join(file);
    std::fs::write(&f, content).unwrap();
    let fs = f.display().to_string();
    let runner = Arc::new(FakeRunner::default());
    let opener = Arc::new(Recorder::default());
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let a = a
        .with_runner(runner.clone())
        .with_opener(opener.clone())
        .with_macros(d.join("config").join("macros.json"));
    let mut h = harness(a);
    h.run();
    (h, d, runner, opener)
}

#[test]
fn a_recorded_macro_plays_back_as_one_undo_step_and_runs_to_the_end_safely() {
    let (mut h, d, _, _) = macro_app("macro", "a.txt", "one\ntwo\nthree\n");
    // Record: Home, type "- ", down.
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert!(h.state().recording.is_some());
    assert!(
        h.query_by_label("● Recording").is_some(),
        "the status bar says so"
    );
    key(&mut h, egui::Key::Home, egui::Modifiers::NONE);
    text(&mut h, "-");
    text(&mut h, " ");
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    use jarak_core::macros::Step;
    assert_eq!(
        h.state().last_macro.clone().unwrap(),
        vec![
            Step::Cmd("cursor.line-start".into()),
            Step::Type("- ".into()),
            Step::Cmd("cursor.down".into()),
        ],
        "commands and typed text, not keys"
    );
    assert_eq!(doc_text(&h), "- one\ntwo\nthree\n");
    // F9 plays it once; one Ctrl+Z takes the whole playback back.
    key(&mut h, egui::Key::F9, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "- one\n- two\nthree\n");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "- one\ntwo\nthree\n");
    // Until the end of the file: it stops there (the empty last line takes the last run).
    palette_run(&mut h, "Run a macro multiple times");
    h.run();
    h.state_mut().macro_dialog = Some(jarak_gui::macros::Dialog::Multi {
        pick: None,
        times: "2".into(),
        to_end: true,
    });
    h.run();
    h.get_by_label("Run").click();
    h.run();
    // It stops at the end of the text (the empty last line is the end).
    assert_eq!(doc_text(&h), "- one\n- two\n- three\n");
    // Each run is one undo step: Ctrl+Z takes back the last run only.
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "- one\n- two\nthree\n");
    assert!(h.state().macro_dialog.is_none());
    let _ = std::fs::remove_dir_all(&d);
    // A macro that makes no progress stops at once instead of running 100 000 times.
    let (mut h, d, _, _) = macro_app("noprogress", "a.txt", "x\n");
    h.state_mut().last_macro = Some(vec![Step::Cmd("cursor.doc-start".into())]);
    h.state_mut().play(
        &[Step::Cmd("cursor.doc-start".into())],
        jarak_gui::macros::Times::ToEnd,
    );
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("changed nothing")),
        "{:?}",
        h.state().message
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn find_and_replace_are_recorded_with_their_options() {
    let (mut h, d, _, _) = macro_app("macrofind", "a.txt", "cat Cat cat\n");
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    h.state_mut().find.find = "cat".into();
    h.state_mut().find.replace = "dog".into();
    h.state_mut().find.match_case = true;
    h.state_mut().replace_all();
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(doc_text(&h), "dog Cat dog\n");
    // Change the panel; the playback uses the recorded options and leaves the panel's.
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    h.state_mut().find.find = "zzz".into();
    h.state_mut().find.match_case = false;
    key(&mut h, egui::Key::F9, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "dog Cat dog\n", "match case as recorded");
    assert_eq!(h.state().find.find, "zzz");
    assert!(!h.state().find.match_case);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn saved_macros_get_shortcuts_checked_against_reserved_and_jarak_keys() {
    let (mut h, d, _, _) = macro_app("macrosave", "a.txt", "a  \nb \n");
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    text(&mut h, "!");
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    // The conflicts.
    let s = h.state();
    assert!(
        s.shortcut_problem("Ctrl+S", None)
            .unwrap()
            .contains("Jarak already uses it")
    );
    assert!(
        s.shortcut_problem("Win+E", None)
            .unwrap()
            .contains("Reserved")
    );
    let typing = s
        .shortcut_problem("A", None)
        .expect("a typing key is refused");
    assert!(
        typing.contains("types text") || typing.contains("Reserved"),
        "{typing}"
    );
    // Ctrl+Alt+1 is taken too: Jarak's Ctrl+1 (the first tab) takes it with Alt as well.
    assert!(
        s.shortcut_problem("Ctrl+Alt+1", None)
            .unwrap()
            .contains("Jarak already uses it")
    );
    assert!(
        s.shortcut_problem("Ctrl+Shift+1", None).is_none(),
        "{:?}",
        s.shortcut_problem("Ctrl+Shift+1", None)
    );
    // Save it with Ctrl+Shift+1 through the dialog (the keys are pressed in the shortcut field).
    palette_run(&mut h, "Save the recorded macro");
    h.run();
    // The name field has the keyboard focus when the dialog opens: the name is typed at once.
    text(&mut h, "Shout");
    assert!(
        matches!(
            h.state().macro_dialog.as_ref(),
            Some(jarak_gui::macros::Dialog::Save { name, .. }) if name == "Shout"
        ),
        "typed into the name field"
    );
    h.state_mut().macro_dialog = Some(jarak_gui::macros::Dialog::Save {
        name: "Shout".into(),
        keys: String::new(),
        capturing: true,
        error: None,
        focus: false,
    });
    h.run();
    key(
        &mut h,
        egui::Key::Num1,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(h.state().macro_dialog.is_none());
    let saved = std::fs::read_to_string(d.join("config").join("macros.json")).unwrap();
    assert!(
        saved.contains("\"Shout\"") && saved.contains("Ctrl+Shift+1"),
        "{saved}"
    );
    // The shortcut plays it.
    key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);
    key(
        &mut h,
        egui::Key::Num1,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_eq!(doc_text(&h), "a  !\nb !\n");
    // The built-in Trim Trailing Space and Save: trims, and the file on disk has it.
    palette_run(&mut h, "Macro: Trim Trailing Space and Save");
    h.run();
    assert_eq!(
        std::fs::read_to_string(d.join("a.txt")).unwrap(),
        "a  !\nb !\n".replace(" \n", "\n")
    );
    assert!(!h.state().tab().file.doc.is_dirty());
    // Removing it in the manager.
    palette_run(&mut h, "Macros and run commands");
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    assert!(h.state().user_cmds.macros.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn run_commands_show_the_exact_line_and_run_only_on_the_button() {
    let (mut h, d, runner, opener) = macro_app("runcmd", "a b.md", "hello 자락\n");
    // F5: the command field has the keyboard focus, so the command is typed straight away (found
    // in the real window: typing went nowhere before); nothing runs while it is typed.
    key(&mut h, egui::Key::F5, egui::Modifiers::NONE);
    h.run();
    text(
        &mut h,
        "notepad.exe \"$(FULL_CURRENT_PATH)\" $(CURRENT_LINE)",
    );
    if let Some(jarak_gui::macros::Dialog::Run { command, .. }) = h.state().macro_dialog.as_ref() {
        assert_eq!(
            command, "notepad.exe \"$(FULL_CURRENT_PATH)\" $(CURRENT_LINE)",
            "typed into the command field"
        );
    } else {
        panic!("the Run dialog is open");
    }
    assert_eq!(doc_text(&h), "hello 자락\n", "the document took none of it");
    let full = d.join("a b.md").display().to_string();
    let exact = format!("notepad.exe \"{full}\" 1");
    assert!(
        h.query_by_label(&exact).is_some(),
        "the exact line is shown"
    );
    assert!(runner.ran.lock().unwrap().is_empty(), "nothing ran yet");
    // Saving it, then Run.
    if let Some(jarak_gui::macros::Dialog::Run { name, .. }) = h.state_mut().macro_dialog.as_mut() {
        *name = "Notepad".into();
    }
    h.run();
    h.get_by_label("Save").click();
    h.run();
    h.get_by_label("Run").click();
    h.run();
    assert_eq!(
        *runner.ran.lock().unwrap(),
        vec![("notepad.exe".to_string(), format!("\"{full}\" 1"))]
    );
    assert!(h.state().macro_dialog.is_none());
    // A saved command from the palette asks first, showing its line.
    palette_run(&mut h, "Run: Notepad");
    h.run();
    assert!(h.query_by_label(&exact).is_some());
    assert_eq!(
        runner.ran.lock().unwrap().len(),
        1,
        "not run without the button"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run();
    assert_eq!(runner.ran.lock().unwrap().len(), 1, "cancelled");
    // The built-in Wikipedia search opens the browser with the word at the caret.
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    palette_run(&mut h, "Run: Wikipedia search");
    h.run();
    h.get_by_label("Run").click();
    h.run();
    assert_eq!(
        opener.0.lock().unwrap().last().cloned(),
        Some("https://en.wikipedia.org/wiki/Special:Search?search=자락".to_string())
    );
    // Search the selection on the internet (encoded).
    key(&mut h, egui::Key::Home, egui::Modifiers::NONE);
    key(&mut h, egui::Key::End, egui::Modifiers::SHIFT);
    palette_run(&mut h, "Search the selection on the internet");
    assert_eq!(
        opener.0.lock().unwrap().last().cloned(),
        Some("https://duckduckgo.com/?q=hello%20%EC%9E%90%EB%9D%BD".to_string())
    );
    // A Markdown file does not open in the browser; it says why.
    palette_run(&mut h, "View the file in the default browser");
    assert!(h.query_by_label_contains("Only HTML").is_some());
    let _ = std::fs::remove_dir_all(&d);
    // A program that needs administrator rights is refused, not elevated.
    let d = temp("runelevate");
    let f = d.join("a.txt");
    std::fs::write(&f, "x\n").unwrap();
    let fs = f.display().to_string();
    let runner = Arc::new(FakeRunner {
        refuse: Some("it needs administrator rights, and Jarak never elevates".into()),
        ..FakeRunner::default()
    });
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a.with_runner(runner));
    h.run();
    key(&mut h, egui::Key::F5, egui::Modifiers::NONE);
    if let Some(jarak_gui::macros::Dialog::Run { command, .. }) =
        h.state_mut().macro_dialog.as_mut()
    {
        *command = "setup.exe".into();
    }
    h.run();
    h.get_by_label("Run").click();
    h.run();
    assert!(h.query_by_label_contains("never elevates").is_some());
    assert!(
        h.state().macro_dialog.is_some(),
        "the dialog stays with the reason"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The real runner starts a program without a shell and does not wait (a console program from a
/// console test process has no window of its own).
#[test]
fn the_process_runner_starts_programs_and_names_failures() {
    use jarak_gui::macros::Runner;
    let r = jarak_gui::macros::ProcessRunner;
    assert!(r.run("cmd", "/c exit 0", None).is_ok());
    assert!(r.run("no-such-program-jarak.exe", "", None).is_err());
}

/// Step 102: the Run dialog and the macros dialog pass the UI audit's rules (headless).
#[test]
fn the_run_and_macro_dialogs_pass_the_ui_audit_rules() {
    let (mut h, d, _, _) = macro_app("audit102", "a.rs", RUST);
    h.state_mut()
        .user_cmds
        .macros
        .push(jarak_core::macros::Macro {
            name: "Tidy".into(),
            keys: Some("Ctrl+Shift+1".into()),
            steps: vec![],
        });
    for screen in ["run", "macros"] {
        if screen == "run" {
            h.state_mut().open_run();
        } else {
            h.state_mut().macro_dialog = None;
            h.state_mut().macro_tool(2);
        }
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        // What the e2e scenario reads from the geometry report.
        let texts: Vec<&str> = r
            .components
            .iter()
            .filter_map(|c| c.text.as_ref().map(|t| t.content.as_str()))
            .collect();
        if screen == "macros" {
            assert!(texts.contains(&"Tidy"), "{texts:?}");
        } else {
            assert!(r.components.iter().any(|c| c.id == "run:line"));
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_manager_changes_shortcuts_and_removes_commands_and_html_opens_in_the_browser() {
    let (mut h, d, _, opener) = macro_app("manager", "page.html", "<p>x</p>\n");
    h.state_mut()
        .user_cmds
        .macros
        .push(jarak_core::macros::Macro {
            name: "M".into(),
            keys: Some("Ctrl+Shift+1".into()),
            steps: vec![jarak_core::macros::Step::Type("!".into())],
        });
    h.state_mut()
        .user_cmds
        .commands
        .push(jarak_core::macros::RunCommand {
            name: "C".into(),
            command: "notepad.exe".into(),
            keys: None,
        });
    // A new shortcut for the macro: pressed in its row's field; a taken one is refused.
    palette_run(&mut h, "Macros and run commands");
    h.run();
    h.state_mut().macro_dialog = Some(jarak_gui::macros::Dialog::Manage {
        commands: false,
        capturing: Some(0),
        error: None,
    });
    h.run();
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    h.run();
    assert!(h.query_by_label_contains("Jarak already uses it").is_some());
    assert_eq!(
        h.state().user_cmds.macros[0].keys.as_deref(),
        Some("Ctrl+Shift+1")
    );
    h.state_mut().macro_dialog = Some(jarak_gui::macros::Dialog::Manage {
        commands: false,
        capturing: Some(0),
        error: None,
    });
    h.run();
    key(
        &mut h,
        egui::Key::Num2,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    h.run();
    assert_eq!(
        h.state().user_cmds.macros[0].keys.as_deref(),
        Some("Ctrl+Shift+2")
    );
    let saved = std::fs::read_to_string(d.join("config").join("macros.json")).unwrap();
    assert!(saved.contains("Ctrl+Shift+2"), "{saved}");
    // The commands page: removing the command.
    h.state_mut().macro_dialog = Some(jarak_gui::macros::Dialog::Manage {
        commands: true,
        capturing: None,
        error: None,
    });
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    assert!(h.state().user_cmds.commands.is_empty());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run();
    // An HTML file opens in the default browser (its default program).
    palette_run(&mut h, "View the file in the default browser");
    assert_eq!(
        opener.0.lock().unwrap().last().cloned(),
        Some(format!("open:{}", d.join("page.html").display()))
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 103: the Encoding menu, line endings, Show Symbol, binary content -------------------

#[test]
fn reinterpret_reads_the_bytes_again_and_asks_before_losing_changes() {
    let d = temp("reinterpret");
    let f = d.join("ja.txt");
    let sjis = jarak_core::file::encoding_for_label("shift_jis").unwrap();
    let bytes = jarak_core::file::encode(
        "日本語\n",
        jarak_core::file::Format {
            encoding: sjis,
            bom: false,
        },
    )
    .unwrap();
    std::fs::write(&f, &bytes).unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    assert_ne!(
        doc_text(&h),
        "日本語\n",
        "the Korean fallback misreads Shift-JIS"
    );
    palette_run(&mut h, "Encoding: reinterpret as Japanese: Shift-JIS");
    h.run();
    assert_eq!(doc_text(&h), "日本語\n");
    assert_eq!(h.state().tab().file.format.encoding, sjis);
    assert!(!h.state().tab().file.read_only);
    assert!(
        h.query_by_label("Shift_JIS").is_some(),
        "the status bar names it"
    );
    // With unsaved changes it asks first; Cancel keeps them.
    text(&mut h, "x");
    palette_run(&mut h, "Encoding: reinterpret as Japanese: EUC-JP");
    h.run();
    assert!(h.state().enc_question.is_some());
    h.get_by_label("Cancel").click();
    h.run();
    assert!(doc_text(&h).starts_with('x'));
    palette_run(&mut h, "Encoding: reinterpret as Japanese: Shift-JIS");
    h.run();
    h.get_by_label("Read again").click();
    h.run();
    assert_eq!(doc_text(&h), "日本語\n", "read again, the typing gone");
    assert_eq!(
        std::fs::read(&f).unwrap(),
        bytes,
        "the file is never written by Reinterpret"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn convert_names_what_it_cannot_hold_and_saves_in_the_new_encoding() {
    let d = temp("convert");
    let f = d.join("a.txt");
    std::fs::write(&f, "Café 한글\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a);
    h.run();
    palette_run(
        &mut h,
        "Encoding: convert to Western European: Windows-1252",
    );
    h.run();
    assert!(
        h.query_by_label_contains("2 characters cannot be written")
            .is_some()
    );
    assert!(
        h.query_by_label_contains("U+D55C").is_some(),
        "the first one named"
    );
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(
        h.state().tab().file.format.encoding,
        jarak_core::file::TextEncoding::Utf8,
        "unchanged"
    );
    palette_run(
        &mut h,
        "Encoding: convert to Western European: Windows-1252",
    );
    h.run();
    h.get_by_label("Replace and convert").click();
    h.run();
    assert_eq!(doc_text(&h), "Café ??\n");
    assert!(h.state().tab().file.doc.is_dirty());
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    let w1252 = jarak_core::file::encoding_for_label("windows-1252").unwrap();
    assert_eq!(
        std::fs::read(&f).unwrap(),
        jarak_core::file::encode(
            "Café ??\n",
            jarak_core::file::Format {
                encoding: w1252,
                bom: false
            }
        )
        .unwrap()
    );
    // Without loss nothing is asked: UTF-16 LE BOM, saved with its BOM.
    palette_run(&mut h, "Encoding: convert to UTF-16 LE BOM");
    h.run();
    assert!(h.state().enc_question.is_none());
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert!(std::fs::read(&f).unwrap().starts_with(&[0xFF, 0xFE]));
    // The status bar's encoding opens the Encoding menu.
    h.get_by_label("UTF-16 LE BOM").click();
    h.run();
    assert_eq!(
        h.state().palette.as_ref().map(|p| p.query.as_str()),
        Some("Encoding:")
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn line_endings_convert_and_new_documents_take_the_settings_encoding() {
    let (mut h, d) = open_doc("eolconv", "a.txt", "a\r\nb\r\n");
    palette_run(&mut h, "EOL: Unix (LF)");
    h.run();
    assert_eq!(doc_text(&h), "a\nb\n");
    assert!(h.query_by_label("LF").is_some());
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "a\r\nb\r\n", "one undo step");
    // The status bar's line ending opens the EOL entries.
    h.get_by_label("CRLF").click();
    h.run();
    assert_eq!(
        h.state().palette.as_ref().map(|p| p.query.as_str()),
        Some("EOL:")
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    palette_run(&mut h, "Encoding: new documents in UTF-8-BOM");
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    h.run();
    assert!(h.state().tab().file.format.bom);
    assert!(
        h.query_by_label("UTF-8 BOM").is_some(),
        "the status bar's name of it"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn show_symbol_settings_reach_the_view_and_binary_content_keeps_nul() {
    use jarak_gui::clipboard::Clipboard as _;
    let d = temp("symbols");
    let f = d.join("a.txt");
    std::fs::write(&f, "\tindented line\n    four spaces\n").unwrap();
    let fs = f.display().to_string();
    let (mut h, clip) = app_clip(&[fs.as_str()], Arc::default());
    palette_run(&mut h, "Show all characters");
    palette_run(&mut h, "Show the indent guides");
    palette_run(&mut h, "Show the edge line");
    h.run();
    let s = &h.state().tab().view.symbols;
    assert!(s.spaces && s.eol && s.controls && s.indent_guides);
    assert_eq!(s.edges, vec![80]);
    palette_run(&mut h, "Show all characters");
    h.run();
    assert!(!h.state().tab().view.symbols.spaces, "toggled off together");
    // Binary content: NUL characters survive the copy and the paste.
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    text(&mut h, "a\u{0}b");
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::SHIFT);
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::SHIFT);
    key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::SHIFT);
    palette_run(&mut h, "Copy binary content");
    assert_eq!(clip.get_binary().as_deref(), Some("a\u{0}b"));
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    palette_run(&mut h, "Paste binary content");
    assert!(
        doc_text(&h).ends_with("a\u{0}ba\u{0}b"),
        "{:?}",
        doc_text(&h)
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 103: the encoding question and a view with every symbol on pass the UI audit's rules.
#[test]
fn the_encoding_question_and_the_symbols_pass_the_ui_audit_rules() {
    let (mut h, d) = open_doc(
        "audit103",
        "a.rs",
        "fn main() {\n\tlet s = \"한 \u{1}\";\n}\n",
    );
    palette_run(&mut h, "Show all characters");
    palette_run(&mut h, "Show the indent guides");
    palette_run(&mut h, "Show the edge line");
    for screen in ["symbols", "encoding-question"] {
        if screen == "encoding-question" {
            palette_run(
                &mut h,
                "Encoding: convert to Western European: Windows-1252",
            );
        }
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
    }
    assert!(h.state().enc_question.is_some());
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 104: files changed on disk, recovery snapshots, robust sessions ----------------------

fn disk_question(h: &Harness<'static, JarakApp>) -> Option<jarak_gui::watch::DiskQuestion> {
    h.state().watch.questions.front().copied()
}

/// JARAK-FR-120, REL-03: another program's change to a document without unsaved changes is
/// asked about; Reload is one undo step (Ctrl+Z brings the text from before back, unsaved);
/// keeping the version here acknowledges the change (it is not asked about again).
#[test]
fn a_file_changed_on_disk_is_asked_about_and_reloads_as_one_undo_step() {
    use jarak_gui::watch::DiskQuestion;
    let (mut h, d) = open_doc("disk-changed", "a.txt", "one\n");
    let f = d.join("a.txt");
    h.state_mut().check_disk_now();
    assert_eq!(disk_question(&h), None, "nothing changed");
    std::fs::write(&f, "one\ntwo from outside\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    assert!(matches!(disk_question(&h), Some(DiskQuestion::Changed(_))));
    assert!(
        h.query_by_label_contains("was changed by another program")
            .is_some()
    );
    h.get_by_label("Reload").click();
    h.run();
    assert_eq!(doc_text(&h), "one\ntwo from outside\n");
    assert!(!h.state().tab().file.doc.is_dirty());
    assert_eq!(disk_question(&h), None);
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(
        doc_text(&h),
        "one\n",
        "Ctrl+Z brings the text from before back"
    );
    assert!(h.state().tab().file.doc.is_dirty());
    key(&mut h, egui::Key::Y, egui::Modifiers::CTRL);
    // Keeping this version: the change is acknowledged, the document differs from the file.
    std::fs::write(&f, "three\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    h.get_by_label("Keep this version").click();
    h.run();
    assert_eq!(doc_text(&h), "one\ntwo from outside\n");
    assert!(
        h.state().tab().file.doc.is_dirty(),
        "it is not what the file holds"
    );
    h.state_mut().check_disk_now();
    assert_eq!(disk_question(&h), None, "not asked again");
    let _ = std::fs::remove_dir_all(&d);
}

/// REL-04: with *Reload unmodified documents silently*, a clean document follows its file
/// without a question; a document with unsaved changes is still asked about.
#[test]
fn unmodified_documents_reload_silently_when_set() {
    use jarak_gui::watch::DiskQuestion;
    let (mut h, d) = open_doc("disk-silent", "a.txt", "one\n");
    let f = d.join("a.txt");
    palette_run(&mut h, "Reload unmodified documents silently");
    assert!(h.state().reload_unmodified);
    std::fs::write(&f, "one\nnew\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    assert_eq!(disk_question(&h), None);
    assert_eq!(doc_text(&h), "one\nnew\n");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("Reloaded"))
    );
    text(&mut h, "mine ");
    std::fs::write(&f, "outside again\n").unwrap();
    h.state_mut().check_disk_now();
    assert!(matches!(disk_question(&h), Some(DiskQuestion::Conflict(_))));
    let _ = std::fs::remove_dir_all(&d);
}

/// JARAK-FR-120: a document with unsaved changes whose file changed offers keep mine, reload and
/// compare; Compare opens the file's version read-only in the second view; keep mine lets the
/// save write this text; Reload takes the file's text and Ctrl+Z brings mine back.
#[test]
fn a_conflict_offers_keep_mine_reload_and_compare() {
    use jarak_gui::watch::DiskQuestion;
    let (mut h, d) = open_doc("disk-conflict", "a.txt", "base\n");
    let f = d.join("a.txt");
    text(&mut h, "mine ");
    std::fs::write(&f, "theirs\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    assert!(matches!(disk_question(&h), Some(DiskQuestion::Conflict(_))));
    h.get_by_label("Compare").click();
    h.run();
    let disk_tab = h
        .state()
        .tabs
        .iter()
        .find(|t| t.label.is_some())
        .expect("the version on disk opened");
    assert_eq!(disk_tab.title(), "a.txt (on disk)");
    assert_eq!(disk_tab.file.doc.text().to_string(), "theirs\n");
    assert!(disk_tab.file.read_only);
    assert!(h.state().second.is_some(), "it is beside the document");
    assert_eq!(disk_question(&h), None);
    let mine = h
        .state()
        .tabs
        .iter()
        .position(|t| t.file.path.is_some())
        .unwrap();
    assert_eq!(
        h.state().tabs[mine].file.doc.text().to_string(),
        "mine base\n"
    );
    assert!(
        h.state_mut().save(mine),
        "keep mine: the save writes this text"
    );
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "mine base\n");
    // Again, and Reload this time.
    h.state_mut().tabs[mine]
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::point(0));
    h.state_mut().tabs[mine].file.doc.paste("more ");
    std::fs::write(&f, "theirs 2\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    h.get_by_label("Reload").click();
    h.run();
    assert_eq!(
        h.state().tabs[mine].file.doc.text().to_string(),
        "theirs 2\n"
    );
    assert!(h.state_mut().tabs[mine].file.doc.undo());
    assert_eq!(
        h.state().tabs[mine].file.doc.text().to_string(),
        "more mine base\n",
        "nothing is lost"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// REL-05: a deleted file's document stays open with its text, unsaved; Save writes it again.
#[test]
fn a_deleted_file_stays_open_unsaved_and_save_writes_it_again() {
    let (mut h, d) = open_doc("disk-deleted", "a.txt", "keep this\n");
    let f = d.join("a.txt");
    std::fs::remove_file(&f).unwrap();
    h.state_mut().check_disk_now();
    h.run();
    assert!(h.state().tab().missing);
    assert!(h.state().tab().file.doc.is_dirty());
    assert_eq!(doc_text(&h), "keep this\n");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("deleted"))
    );
    assert_eq!(
        disk_question(&h),
        None,
        "no question: the text simply stays"
    );
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "keep this\n");
    assert!(!h.state().tab().file.doc.is_dirty());
    let _ = std::fs::remove_dir_all(&d);
}

/// A save never overwrites a change it has not seen: the question comes instead.
#[test]
fn a_save_never_overwrites_an_unseen_change() {
    use jarak_gui::watch::DiskQuestion;
    let (mut h, d) = open_doc("disk-unseen", "a.txt", "base\n");
    let f = d.join("a.txt");
    text(&mut h, "mine ");
    std::fs::write(&f, "written meanwhile\n").unwrap();
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "written meanwhile\n");
    assert!(matches!(disk_question(&h), Some(DiskQuestion::Conflict(_))));
    assert!(h.state().tab().file.doc.is_dirty());
    let _ = std::fs::remove_dir_all(&d);
}

/// JARAK-FR-122, VIEW-17: monitoring follows a growing file, read-only, at its end.
#[test]
fn monitoring_follows_a_growing_file_read_only_at_its_end() {
    let (mut h, d) = open_doc("disk-monitor", "log.txt", "line 1\n");
    let f = d.join("log.txt");
    palette_run(&mut h, "Monitoring (tail -f)");
    assert!(h.state().tab().monitor);
    assert!(h.state().tab().file.read_only);
    std::fs::write(&f, "line 1\nline 2\nline 3\n").unwrap();
    h.state_mut().check_disk_now();
    h.run();
    assert_eq!(doc_text(&h), "line 1\nline 2\nline 3\n");
    assert_eq!(disk_question(&h), None, "no question while monitoring");
    let end = h.state().tab().file.doc.len_chars();
    assert_eq!(h.state().tab().file.doc.selection().primary().head, end);
    text(&mut h, "typed");
    assert_eq!(
        doc_text(&h),
        "line 1\nline 2\nline 3\n",
        "read-only while monitored"
    );
    palette_run(&mut h, "Monitoring (tail -f)");
    assert!(!h.state().tab().monitor);
    assert!(!h.state().tab().file.read_only);
    let _ = std::fs::remove_dir_all(&d);
}

/// The window's own checks (as the program runs them): a change on disk is found off the UI
/// thread within a few frames once the window has the focus.
#[test]
fn the_window_checks_its_files_by_itself() {
    use jarak_gui::watch::DiskQuestion;
    let d = temp("disk-auto");
    let f = d.join("a.txt");
    std::fs::write(
        &f, "one
",
    )
    .unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .build_ui_state(|ui, app: &mut JarakApp| app.ui(ui), a.with_watching());
    h.step();
    std::fs::write(
        &f, "changed
",
    )
    .unwrap();
    let mut found = false;
    for _ in 0..400 {
        h.step();
        if matches!(disk_question(&h), Some(DiskQuestion::Changed(_))) {
            found = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(found, "the window found the change by itself");
    let _ = std::fs::remove_dir_all(&d);
}

fn wait_for(h: &mut Harness<'static, JarakApp>, what: impl Fn() -> bool) -> bool {
    for _ in 0..100 {
        if what() {
            return true;
        }
        h.run();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    what()
}

/// JARAK-FR-121, REL-02, JRK-16: unsaved documents (one with a file, one untitled) are kept;
/// after the window is gone, the next window offers them and Restore brings back their text,
/// unsaved (Ctrl+Z on the file's document shows what is on disk); the old folder goes once the
/// new window's own snapshot is written.
#[test]
fn unsaved_documents_are_kept_and_offered_back_after_a_crash() {
    use jarak_core::recovery::Owner;
    let d = temp("recovery-offer");
    let root = d.join("recovery");
    let f = d.join("notes.md");
    std::fs::write(&f, "# 자락\n").unwrap();
    let fs = f.display().to_string();
    let first = Owner {
        pid: 11,
        started: 1,
    };
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let a = a.with_recovery(&root, first, &|_| true, 10.0);
    let mut h = harness(a);
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    text(&mut h, "편집 중");
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    text(&mut h, "untitled note");
    let w = h.state_mut().snapshot_now().unwrap();
    assert_eq!(w.kept.len(), 2);
    assert!(root.join("w-11-1").join("manifest.json").is_file());
    // The window is gone without a normal exit; the next one finds its folder.
    let second = Owner {
        pid: 12,
        started: 2,
    };
    let (b, _) = app_with(&[], Arc::default());
    let b = b.with_recovery(&root, second, &|o| *o == second, 10.0);
    let mut h2 = harness(b);
    assert!(h2.query_by_label("Recover unsaved documents?").is_some());
    assert!(h2.query_by_label_contains("notes.md").is_some());
    h2.get_by_label("Restore").click();
    h2.run();
    assert!(h2.state().recovery_offer.is_none());
    let tabs = &h2.state().tabs;
    let file_tab = tabs.iter().find(|t| t.file.path.is_some()).unwrap();
    assert_eq!(file_tab.file.doc.text().to_string(), "# 자락\n편집 중");
    assert!(file_tab.file.doc.is_dirty());
    let untitled = tabs
        .iter()
        .find(|t| t.file.path.is_none() && t.file.doc.len_chars() > 0)
        .unwrap();
    assert_eq!(untitled.file.doc.text().to_string(), "untitled note");
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "# 자락\n",
        "the file is untouched"
    );
    let k = h2
        .state()
        .tabs
        .iter()
        .position(|t| t.file.path.is_some())
        .unwrap();
    assert!(h2.state_mut().tabs[k].file.doc.undo());
    assert_eq!(h2.state().tabs[k].file.doc.text().to_string(), "# 자락\n");
    assert!(h2.state_mut().tabs[k].file.doc.redo());
    h2.state_mut().snapshot_now().unwrap();
    assert!(root.join("w-12-2").join("manifest.json").is_file());
    assert!(
        !std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("claimed-")),
        "the old folder goes once this window keeps the documents"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Not now keeps the documents for the next start; Discard asks twice and removes them.
#[test]
fn recovered_documents_wait_with_not_now_and_go_with_discard_twice() {
    use jarak_core::recovery::Owner;
    let d = temp("recovery-discard");
    let root = d.join("recovery");
    let gone = Owner {
        pid: 21,
        started: 1,
    };
    let (a, _) = app_with(&[], Arc::default());
    let mut a = a.with_recovery(&root, gone, &|_| true, 10.0);
    a.tab_mut().file.doc.paste("lost unless kept");
    a.snapshot_now().unwrap();
    let open = |pid| {
        let me = Owner { pid, started: 3 };
        let (b, _) = app_with(&[], Arc::default());
        harness(b.with_recovery(&root, me, &move |o| *o == me, 10.0))
    };
    let mut h = open(22);
    h.get_by_label("Not now").click();
    h.run();
    assert!(h.state().recovery_offer.is_none());
    let mut h = open(23);
    assert!(
        h.state().recovery_offer.is_some(),
        "offered again at the next start"
    );
    h.get_by_label("Discard…").click();
    h.run();
    assert!(
        h.state().recovery_offer.is_some(),
        "the first press only asks"
    );
    h.get_by_label("Discard for good").click();
    h.run();
    assert!(h.state().recovery_offer.is_none());
    assert!(
        std::fs::read_dir(&root).unwrap().next().is_none(),
        "removed"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Saving a document removes its snapshot at once; a normal exit removes the window's folder.
#[test]
fn a_saved_document_leaves_its_snapshot_and_a_normal_exit_keeps_none() {
    use jarak_core::recovery::Owner;
    let d = temp("recovery-clean");
    let root = d.join("recovery");
    let f = d.join("a.txt");
    std::fs::write(&f, "a\n").unwrap();
    let fs = f.display().to_string();
    let me = Owner {
        pid: 31,
        started: 1,
    };
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    // A snapshot every frame (a tiny interval), written by the window's own thread.
    let mut h = harness(a.with_recovery(&root, me, &|_| true, 0.001));
    let dir = root.join("w-31-1");
    text(&mut h, "x");
    assert!(wait_for(&mut h, || dir.join("manifest.json").is_file()));
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert!(
        wait_for(&mut h, || !dir.exists()),
        "saved: nothing left to recover"
    );
    text(&mut h, "y");
    assert!(wait_for(&mut h, || dir.join("manifest.json").is_file()));
    h.state_mut().recovery_exit();
    assert!(!dir.exists(), "a normal exit keeps no snapshots");
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 104: a damaged session file falls back to the backup the last save kept (and the
/// window says so); the damaged file is kept.
#[test]
fn a_damaged_session_opens_the_backup_and_says_so() {
    let d = temp("session-backup");
    let f = d.join("a.txt");
    std::fs::write(&f, "a\n").unwrap();
    let session = d.join("session.json");
    let s = jarak_core::session::Session {
        documents: vec![jarak_core::session::Doc {
            path: Some(f.clone()),
            ..jarak_core::session::Doc::default()
        }],
        active: Some(0),
        ..jarak_core::session::Session::default()
    };
    s.save(&session).unwrap();
    s.save(&session).unwrap();
    std::fs::write(&session, "{ damaged").unwrap();
    let (a, _) = app_with(&[], Arc::default());
    let a = a.with_session(jarak_gui::sessions::SessionFiles {
        file: session,
        recovery: d.join("recovery"),
        restore: true,
        save: true,
    });
    let h = harness(a);
    assert_eq!(h.state().tab().file.path.as_deref(), Some(f.as_path()));
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("broken"))
    );
    assert!(d.join("session.json.broken").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// The disk question and the recovery offer under the UI audit's rules.
#[test]
fn the_disk_question_and_the_recovery_offer_pass_the_ui_audit_rules() {
    let (mut h, d) = open_doc("audit104", "a.txt", "text\n");
    for screen in ["disk-question", "recovery"] {
        h.state_mut().prepare_audit(screen);
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        let id = if screen == "disk-question" {
            "disk:question"
        } else {
            "recovery:row#1"
        };
        assert!(r.components.iter().any(|c| c.id == id), "{screen}: {id}");
        h.state_mut().watch.questions.clear();
        h.state_mut().recovery_offer = None;
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 105: the Settings window, live settings ----------------------------------------------

fn settings_app(name: &str, files: &[&str]) -> (Harness<'static, JarakApp>, PathBuf) {
    let d = temp(name);
    let mut args = Vec::new();
    for f in files {
        let p = d.join(f);
        std::fs::write(&p, "one\ntwo\n").unwrap();
        args.push(p.display().to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (a, _) = app_with(&refs, Arc::default());
    let h = harness(
        a.with_settings(d.join("settings.toml"))
            .with_hotkeys(&d.join("hotkeys.json")),
    );
    (h, d)
}

fn settings_file(d: &std::path::Path) -> String {
    std::fs::read_to_string(d.join("settings.toml")).unwrap_or_default()
}

/// JARAK-FR-130: Ctrl+, opens the Settings window at General; every section lists its settings
/// from the registry; Esc closes it.
#[test]
fn the_settings_window_lists_every_section() {
    let (mut h, d) = settings_app("settings-open", &["a.txt"]);
    key(&mut h, egui::Key::Comma, egui::Modifiers::CTRL);
    assert!(h.state().settings_win.is_some());
    assert!(h.query_by_label("Theme").is_some(), "General first");
    for sec in jarak_core::settings::Section::ALL {
        h.state_mut().open_settings(Some(sec));
        h.run();
        for s in jarak_core::settings::of(sec) {
            assert!(
                h.query_all_by_label_contains(s.en).next().is_some(),
                "{}: {}",
                sec.id(),
                s.en
            );
        }
    }
    h.state_mut()
        .open_settings(Some(jarak_core::settings::Section::Keyboard));
    h.run();
    assert!(
        h.query_all_by_label_contains("Save: Ctrl+S")
            .next()
            .is_some(),
        "commands with their keys"
    );
    assert!(h.query_all_by_label("Change").next().is_some());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().settings_win.is_none(), "Esc closes it");
    let _ = std::fs::remove_dir_all(&d);
}

/// JARAK-FR-130: a switch writes `settings.toml` atomically and applies at once — the status bar
/// goes, line numbers go, the theme and the language change, word wrap reaches open documents.
#[test]
fn a_changed_setting_is_written_and_applied_at_once() {
    let (mut h, d) = settings_app("settings-live", &["a.txt"]);
    assert!(
        h.query_by_label("Settings").is_some(),
        "the gear in the status bar"
    );
    assert!(h.state_mut().change_setting("status_bar", "false"));
    h.run();
    assert!(settings_file(&d).contains("status_bar = false"));
    assert!(
        h.query_by_label("Settings").is_none(),
        "no status bar, no gear"
    );
    assert!(h.state_mut().change_setting("line_numbers", "off"));
    h.run();
    assert!(!h.state().tab().view.look.line_numbers);
    assert!(h.state_mut().change_setting("word_wrap", "true"));
    h.run();
    assert!(
        h.state().tab().view.wrap,
        "word wrap reaches the open document"
    );
    assert!(h.state_mut().change_setting("theme", "light"));
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
    assert!(h.state_mut().change_setting("theme", "dark"));
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    // The language follows the setting once the program lets it (no --lang).
    h.state_mut().lang = Lang::En;
    let (a, _) = app_with(&[], Arc::default());
    let mut k = harness(
        a.with_settings(d.join("settings.toml"))
            .with_lang_from_settings(),
    );
    k.state_mut().change_setting("language", "ko");
    k.run();
    assert_eq!(k.state().lang, Lang::Ko);
    assert!(
        k.query_by_label("설정").is_none(),
        "no status bar here either"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A value the setting does not take is said beside it, and nothing is written.
#[test]
fn a_refused_value_is_said_and_nothing_is_written() {
    let (mut h, d) = settings_app("settings-refused", &["a.txt"]);
    h.state_mut()
        .open_settings(Some(jarak_core::settings::Section::Advanced));
    h.run();
    assert!(
        !h.state_mut()
            .change_setting("search_engine", "https://example.org/?q=")
    );
    h.run();
    assert!(
        h.query_by_label_contains("it has to contain $(QUERY)")
            .is_some()
    );
    assert!(!settings_file(&d).contains("search_engine"));
    assert!(!h.state_mut().change_setting("tab_width", "40"));
    assert!(h.state_mut().change_setting("tab_width", "8"));
    assert_eq!(h.state().tab().file.doc.tab_width, 8);
    let _ = std::fs::remove_dir_all(&d);
}

/// SET-10, SET-06: Tab inserts spaces to the next stop when set; new documents take the line
/// ending of the settings; pasted images go to the settings' folder.
#[test]
fn tab_width_spaces_and_new_documents_follow_the_settings() {
    let (mut h, d) = settings_app("settings-editor", &["a.txt"]);
    h.state_mut().change_setting("use_spaces", "true");
    h.state_mut().change_setting("tab_width", "2");
    h.run();
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    key(&mut h, egui::Key::Tab, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "  one\ntwo\n");
    h.state_mut().change_setting("new_eol", "lf");
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().file.doc.eol, jarak_core::document::Eol::Lf);
    h.state_mut().change_setting("image_folder", "img");
    assert_eq!(h.state().prefs.image_folder, "img");
    let _ = std::fs::remove_dir_all(&d);
}

/// SET-02, SET-03: Ctrl+click adds a caret unless multi-editing is off; the menu bar can stay.
#[test]
fn multi_editing_and_the_menu_bar_follow_the_settings() {
    let (mut h, d) = settings_app("settings-multi", &["a.txt"]);
    assert!(h.state().tab().view.look.multi_editing);
    h.state_mut().change_setting("multi_editing", "false");
    h.run();
    assert!(!h.state().tab().view.look.multi_editing);
    assert!(
        h.query_by_label("File").is_none(),
        "the menu bar waits for Alt"
    );
    h.state_mut().change_setting("menu_bar", "always");
    h.run();
    assert!(h.query_by_label("File").is_some(), "the menu bar stays");
    let _ = std::fs::remove_dir_all(&d);
}

/// Markdown files open as the setting says (a session's documents keep their own mode).
#[test]
fn markdown_files_open_as_the_setting_says() {
    let d = temp("settings-md");
    std::fs::write(d.join("settings.toml"), "markdown_open_as = \"split\"\n").unwrap();
    let f = d.join("n.md");
    std::fs::write(&f, "# t\n").unwrap();
    let fs = f.display().to_string();
    let (a, _) = app_with(&[fs.as_str()], Arc::default());
    let mut h = harness(a.with_settings(d.join("settings.toml")));
    h.run();
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Split);
    let _ = std::fs::remove_dir_all(&d);
}

/// `jarak settings set` while the window runs: the window applies it when it gets the focus.
#[test]
fn a_change_from_the_command_line_is_applied_on_focus() {
    let (mut h, d) = settings_app("settings-cli", &["a.txt"]);
    let st = jarak_core::settings::Settings::new(&d.join("settings.toml"));
    std::thread::sleep(std::time::Duration::from_millis(20));
    st.set("tab_width", "3").unwrap();
    // What the window does when it gets the focus.
    assert!(h.state_mut().settings_changed_outside());
    h.run();
    assert_eq!(h.state().tab().file.doc.tab_width, 3);
    assert!(!h.state_mut().settings_changed_outside(), "read once");
    let _ = std::fs::remove_dir_all(&d);
}

/// Advanced: Reset every setting asks twice, then every setting is its default again (lines
/// Jarak does not know stay).
#[test]
fn every_setting_resets_after_a_second_press() {
    let (mut h, d) = settings_app("settings-reset", &["a.txt"]);
    std::fs::write(d.join("settings.toml"), "mine = 1\n").unwrap();
    h.state_mut().change_setting("tab_width", "8");
    h.state_mut()
        .open_settings(Some(jarak_core::settings::Section::Advanced));
    h.run();
    // Step 110: the page is taller than the window now; scroll down to its last group, as a
    // user would.
    let over = h.get_by_label("Search engine").rect().center();
    h.event(egui::Event::PointerMoved(over));
    for _ in 0..4 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -500.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    let _ = h.try_run();
    h.get_by_label("Reset…").click();
    h.run();
    assert!(
        settings_file(&d).contains("tab_width"),
        "the first press only asks"
    );
    assert!(h.query_by_label("Reset for good").is_some(), "it asks");
    h.get_by_label("Reset for good").click();
    h.run();
    assert_eq!(settings_file(&d), "mine = 1\n");
    assert_eq!(h.state().tab().file.doc.tab_width, 4);
    let _ = std::fs::remove_dir_all(&d);
}

/// Every Settings section under the UI audit's rules (the embedded window of the headless
/// backend), in English and Korean.
#[test]
fn every_settings_section_passes_the_ui_audit_rules() {
    for lang in [Lang::En, Lang::Ko] {
        let (mut h, d) = settings_app("settings-audit", &["a.txt"]);
        h.state_mut().lang = lang;
        for sec in jarak_core::settings::Section::ALL {
            h.state_mut().open_settings(Some(sec));
            h.run();
            kim_ui::inspect::force(true);
            h.run();
            let screen = format!("settings-{}", sec.id());
            let r = kim_ui::inspect::report(&h.ctx, "jarak", &screen);
            kim_ui::inspect::force(false);
            let fails = kim_design::audit::audit(&r);
            assert_eq!(
                kim_design::audit::counts(&fails).2,
                0,
                "{screen} {lang:?}: {}",
                kim_design::audit::render(&r, &fails, false)
            );
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// SET-23: a command given other keys runs from them, and its default does nothing; Off leaves
/// it without keys; Reset gives the default back.
#[test]
fn a_remapped_command_runs_from_its_new_keys() {
    use jarak_core::hotkeys::Change;
    let (mut h, d) = settings_app("keys-remap", &["a.txt"]);
    assert!(h.state_mut().change_shortcut(
        &Change::Set {
            action: "run.dialog".into(),
            keys: vec!["Ctrl+Alt+R".into()],
        },
        false
    ));
    key(&mut h, egui::Key::F5, egui::Modifiers::NONE);
    assert!(h.state().macro_dialog.is_none(), "F5 is free now");
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    assert!(h.state().macro_dialog.is_some(), "Ctrl+Alt+R opens Run");
    h.state_mut().macro_dialog = None;
    assert!(
        h.state_mut()
            .change_shortcut(&Change::Disable("run.dialog".into()), false)
    );
    key(
        &mut h,
        egui::Key::R,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    assert!(h.state().macro_dialog.is_none(), "off");
    assert!(
        h.state_mut()
            .change_shortcut(&Change::Reset("run.dialog".into()), false)
    );
    key(&mut h, egui::Key::F5, egui::Modifiers::NONE);
    assert!(h.state().macro_dialog.is_some(), "the default again");
    let _ = std::fs::remove_dir_all(&d);
}

/// The Keyboard section records keys (Esc cancels), refuses keys another of the user's own
/// bindings has and moves them with Take it; Reset all asks twice.
#[test]
fn the_keyboard_section_records_refuses_and_resets() {
    use jarak_core::hotkeys::Change;
    let (mut h, d) = settings_app("keys-mapper", &["a.txt"]);
    h.state_mut()
        .open_settings(Some(jarak_core::settings::Section::Keyboard));
    h.run();
    // Recording: Esc cancels, a chord is taken.
    if let Some(w) = h.state_mut().settings_win.as_mut() {
        w.keys.capture = Some("run.dialog".into());
    }
    h.run();
    assert!(h.query_by_label_contains("Press keys").is_some());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(
        h.state()
            .settings_win
            .as_ref()
            .is_none_or(|w| w.keys.capture.is_none()),
        "Esc cancels"
    );
    h.state_mut()
        .open_settings(Some(jarak_core::settings::Section::Keyboard));
    if let Some(w) = h.state_mut().settings_win.as_mut() {
        w.keys.capture = Some("run.dialog".into());
    }
    h.run();
    key(
        &mut h,
        egui::Key::K,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    let file = std::fs::read_to_string(d.join("hotkeys.json")).unwrap();
    assert!(file.contains("Ctrl+Alt+K"), "{file}");
    h.run();
    assert!(
        h.query_all_by_label_contains("Run…: Ctrl+Alt+K")
            .next()
            .is_some()
    );
    // The palette asks for the same keys: refused and named, then taken.
    assert!(!h.state_mut().change_shortcut(
        &Change::Set {
            action: "view.palette".into(),
            keys: vec!["Ctrl+Alt+K".into()],
        },
        false
    ));
    if let Some(w) = h.state_mut().settings_win.as_mut() {
        w.keys.filter = "command palette".into();
    }
    h.run();
    h.get_by_label("Take it").click();
    h.run();
    let file = std::fs::read_to_string(d.join("hotkeys.json")).unwrap();
    assert!(file.contains("view.palette"), "{file}");
    // Reset all (below the list: the search narrows it): the first press only asks.
    if let Some(w) = h.state_mut().settings_win.as_mut() {
        w.keys.filter = "no such command".into();
    }
    h.run();
    h.get_by_label("Reset all…").click();
    h.run();
    assert!(
        std::fs::read_to_string(d.join("hotkeys.json"))
            .unwrap()
            .contains("view.palette")
    );
    h.get_by_label("Reset for good").click();
    h.run();
    let (tool, _) = jarak_core::hotkeys::load(&d.join("hotkeys.json"));
    assert!(!jarak_core::hotkeys::keys_of(&tool, "view.palette").1);
    assert!(
        d.join("hotkeys.json.bak").is_file(),
        "the previous file is kept"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `jarak hotkeys set` while the window runs: applied when it gets the focus.
#[test]
fn a_shortcut_changed_from_the_command_line_is_applied_on_focus() {
    use jarak_core::hotkeys::{Change, change};
    let (mut h, d) = settings_app("keys-cli", &["a.txt"]);
    std::thread::sleep(std::time::Duration::from_millis(20));
    change(
        &d.join("hotkeys.json"),
        &Change::Set {
            action: "run.dialog".into(),
            keys: vec!["Ctrl+Alt+J".into()],
        },
        false,
        false,
    )
    .unwrap();
    assert!(h.state_mut().hotkeys_changed_outside());
    key(
        &mut h,
        egui::Key::J,
        egui::Modifiers::CTRL | egui::Modifiers::ALT,
    );
    assert!(h.state().macro_dialog.is_some());
    let _ = std::fs::remove_dir_all(&d);
}

/// A 1×1 PNG (the export reads images only by their first bytes and size).
const DOT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0,
    1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89, 0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE,
    0x42, 0x60, 0x82,
];

/// Step 106 (`JARAK-FR-141`, `JARAK-SEC-001`): a Markdown document exports rendered with its
/// image embedded, unsaved text included, and the document stays as it is.
#[test]
fn exporting_markdown_embeds_its_images_and_keeps_the_document() {
    let d = temp("export-md");
    std::fs::create_dir_all(d.join("assets")).unwrap();
    std::fs::write(d.join("assets/dot.png"), DOT_PNG).unwrap();
    let doc = d.join("note.md");
    std::fs::write(&doc, "# Note\n\n![dot](assets/dot.png)\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    dialogs.export.lock().unwrap().push(d.join("out/note.html"));
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs.clone());
    let mut h = harness(a);
    h.state_mut().tab_mut().file.doc.type_text("unsaved ");
    h.state_mut().export_tool(0);
    h.run();
    let html = std::fs::read_to_string(d.join("out/note.html")).unwrap();
    assert!(html.contains("src=\"data:image/png;base64,"), "embedded");
    assert!(html.contains("unsaved"), "the text as it is in the window");
    assert!(
        html.contains("<p>unsaved # Note</p>"),
        "rendered (the typed text starts the line)"
    );
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "# Note\n\n![dot](assets/dot.png)\n",
        "the document is never written by an export"
    );
    assert!(h.state().tab().file.doc.is_dirty(), "still unsaved");
    let msg = h.state().message.clone().unwrap_or_default();
    assert!(
        msg.contains("Exported to") && msg.contains("1 embedded"),
        "{msg}"
    );
    // A cancelled Save dialog writes nothing.
    h.state_mut().export_tool(0);
    assert_eq!(*dialogs.asked.lock().unwrap(), 2);
    let _ = std::fs::remove_dir_all(&d);
}

/// Code exports highlighted as RTF; Copy as HTML / RTF put the selection on the clipboard with
/// its colours beside the plain text.
#[test]
fn code_exports_as_rtf_and_copies_as_html_and_rtf() {
    let d = temp("export-code");
    let doc = d.join("a.rs");
    std::fs::write(&doc, "fn main() {}\nlet x = 1;\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    dialogs.export.lock().unwrap().push(d.join("a.rtf"));
    let (mut h, clip) = app_clip(&[doc.to_str().unwrap()], dialogs);
    h.state_mut().export_tool(1);
    let rtf = std::fs::read_to_string(d.join("a.rtf")).unwrap();
    assert!(
        rtf.starts_with("{\\rtf1") && rtf.contains("{\\cf2 fn}"),
        "{rtf}"
    );
    h.state_mut().tab_mut().file.doc.select_all();
    h.state_mut().export_tool(3);
    let html = clip.html.lock().unwrap().clone().unwrap();
    assert!(
        html.contains(">fn</span>") && html.starts_with("<pre style="),
        "{html}"
    );
    assert_eq!(
        clip.text.lock().unwrap().as_deref(),
        Some("fn main() {}\nlet x = 1;\n"),
        "the plain text beside it"
    );
    h.state_mut().export_tool(4);
    assert!(
        clip.rtf
            .lock()
            .unwrap()
            .as_deref()
            .is_some_and(|r| r.contains("{\\cf2 let}"))
    );
    assert_eq!(h.state().message.as_deref(), Some("Copied as RTF"));
    let _ = std::fs::remove_dir_all(&d);
}

/// Publishing writes the page and copies of its images; into a folder that holds them already it
/// asks first: Cancel keeps the files, Replace writes them.
#[test]
fn publishing_asks_before_it_replaces_files() {
    let d = temp("publish");
    std::fs::create_dir_all(d.join("doc/assets")).unwrap();
    std::fs::write(d.join("doc/assets/dot.png"), DOT_PNG).unwrap();
    let doc = d.join("doc/note.md");
    std::fs::write(&doc, "![dot](assets/dot.png)\n").unwrap();
    let site = d.join("site");
    let dialogs = Arc::new(Scripted::default());
    dialogs
        .folder
        .lock()
        .unwrap()
        .extend([site.clone(), site.clone(), site.clone()]);
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs);
    let mut h = harness(a);
    h.state_mut().export_tool(2);
    assert!(site.join("note.html").is_file());
    assert_eq!(std::fs::read(site.join("assets/dot.png")).unwrap(), DOT_PNG);
    std::fs::write(site.join("note.html"), "mine").unwrap();
    h.state_mut().export_tool(2);
    h.run();
    assert!(h.state().publish_question.is_some(), "asked");
    assert!(h.query_by_label_contains("Replace them?").is_some());
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().publish_question.is_none());
    assert_eq!(
        std::fs::read_to_string(site.join("note.html")).unwrap(),
        "mine",
        "Cancel replaces nothing"
    );
    h.state_mut().export_tool(2);
    h.run();
    h.get_by_label("Replace").click();
    h.run();
    assert!(
        std::fs::read_to_string(site.join("note.html"))
            .unwrap()
            .contains("<img src=\"assets/dot.png\"")
    );
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "![dot](assets/dot.png)\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Publishing an untitled document or a text file says why it does not.
#[test]
fn publishing_needs_a_saved_markdown_document() {
    let d = temp("publish-no");
    let doc = d.join("a.txt");
    std::fs::write(&doc, "text").unwrap();
    let dialogs = Arc::new(Scripted::default());
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs.clone());
    let mut h = harness(a);
    h.state_mut().export_tool(2);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("Markdown"))
    );
    assert_eq!(*dialogs.asked.lock().unwrap(), 0, "no dialog");
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 106: a paper printer (the default) and Microsoft Print to PDF, recording their jobs.
fn recording_printers(default_pdf: bool) -> Arc<jarak_core::print::RecordingPrinters> {
    use jarak_core::print::{PDF_PRINTER, Printer, RecordingPrinters};
    Arc::new(RecordingPrinters {
        printers: vec![
            Printer {
                name: "Paper".into(),
                port: "USB001:".into(),
                default: !default_pdf,
            },
            Printer {
                name: PDF_PRINTER.into(),
                port: "PORTPROMPT:".into(),
                default: default_pdf,
            },
        ],
        ..RecordingPrinters::default()
    })
}

/// Runs frames until the print job has finished (it runs on a thread).
fn wait_print(h: &mut Harness<'static, JarakApp>) {
    for _ in 0..500 {
        let _ = h.try_run();
        if h.state().printing.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the print job did not finish");
}

/// The text of a recorded job's pages.
fn printed_text(pages: &[jarak_core::print::Page]) -> String {
    jarak_core::print::describe(pages)
}

/// Step 106 (`JARAK-FR-140`): File → Print… prints on the printer the dialog chose, with the
/// dialog's settings, the text as it is in the window; Selection prints only the selection.
#[test]
fn printing_uses_the_dialogs_printer_settings_and_range() {
    use jarak_gui::print_ui::{PrintChoice, Range};
    let d = temp("print-dialog");
    let doc = d.join("a.rs");
    std::fs::write(&doc, "fn main() {}\nlet chosen = 1;\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    dialogs.print.lock().unwrap().push(PrintChoice {
        printer: "Paper".into(),
        devmode: Some(vec![1, 2, 3]),
        range: Range::All,
    });
    let printers = recording_printers(false);
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs.clone());
    let mut h = harness(a.with_printers(printers.clone()));
    h.state_mut().tab_mut().file.doc.type_text("// unsaved\n");
    h.state_mut().act(Action::Print);
    wait_print(&mut h);
    {
        let jobs = printers.jobs.lock().unwrap();
        let (job, pages) = &jobs[0];
        assert_eq!(job.printer, "Paper");
        assert_eq!(
            job.devmode.as_deref(),
            Some(&[1u8, 2, 3][..]),
            "the dialog's settings"
        );
        assert_eq!(job.output, None, "a paper printer writes no file");
        let t = printed_text(pages);
        assert!(t.contains("\"// unsaved\"") && t.contains("chosen"), "{t}");
        assert!(t.contains("\"a.rs\""), "the header names the document");
    }
    let msg = h.state().message.clone().unwrap_or_default();
    assert!(msg.contains("Printed 1 page on Paper"), "{msg}");
    // Only the selection.
    let sel = jarak_core::selection::Selection::single(jarak_core::selection::Range::new(0, 2));
    h.state_mut().tab_mut().file.doc.set_selection(&sel);
    dialogs.print.lock().unwrap().push(PrintChoice {
        printer: "Paper".into(),
        devmode: None,
        range: Range::Selection,
    });
    h.state_mut().act(Action::Print);
    wait_print(&mut h);
    let jobs = printers.jobs.lock().unwrap();
    let t = printed_text(&jobs[1].1);
    assert!(t.contains("\"//\"") && !t.contains("chosen"), "{t}");
    let _ = std::fs::remove_dir_all(&d);
}

/// A printer that writes a file gets it from Jarak's Save dialog (the driver never asks); a
/// cancelled Save dialog prints nothing; Export as PDF prints to Microsoft Print to PDF.
#[test]
fn a_pdf_printer_gets_its_file_from_the_save_dialog() {
    use jarak_gui::print_ui::{PrintChoice, Range};
    let d = temp("print-pdf");
    let doc = d.join("n.md");
    std::fs::write(&doc, "# Heading\n\nBody text.\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    let printers = recording_printers(true);
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs.clone());
    let mut h = harness(a.with_printers(printers.clone()));
    // Cancelled Save dialog: nothing.
    dialogs.print.lock().unwrap().push(PrintChoice {
        printer: jarak_core::print::PDF_PRINTER.into(),
        devmode: None,
        range: Range::All,
    });
    h.state_mut().act(Action::Print);
    assert!(h.state().printing.is_none());
    assert!(printers.jobs.lock().unwrap().is_empty());
    // Print now on the default printer (the PDF one): its file from the Save dialog.
    dialogs.export.lock().unwrap().push(d.join("now.pdf"));
    h.state_mut().act(Action::PrintNow);
    wait_print(&mut h);
    // Export as PDF.
    dialogs.export.lock().unwrap().push(d.join("n.pdf"));
    h.state_mut().export_tool(5);
    wait_print(&mut h);
    let jobs = printers.jobs.lock().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(
        jobs[0].0.output.as_deref(),
        Some(d.join("now.pdf").as_path())
    );
    assert_eq!(jobs[1].0.printer, jarak_core::print::PDF_PRINTER);
    let t = printed_text(&jobs[1].1);
    assert!(
        t.contains("Sans bold") && t.contains("\"Heading\""),
        "rendered: {t}"
    );
    assert!(d.join("n.pdf").is_file());
    let msg = h.state().message.clone().unwrap_or_default();
    assert!(msg.starts_with("Saved") && msg.contains("n.pdf"), "{msg}");
    assert_eq!(
        std::fs::read_to_string(&doc).unwrap(),
        "# Heading\n\nBody text.\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Without printers: Print now and Export as PDF say why, and nothing is asked.
#[test]
fn without_printers_printing_says_why() {
    let d = temp("print-none");
    let doc = d.join("a.txt");
    std::fs::write(&doc, "x").unwrap();
    let dialogs = Arc::new(Scripted::default());
    let (a, _) = app_with(&[doc.to_str().unwrap()], dialogs.clone());
    let mut h = harness(a);
    h.state_mut().act(Action::PrintNow);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("no default printer"))
    );
    h.state_mut().export_tool(5);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("Microsoft Print to PDF is not installed"))
    );
    assert_eq!(*dialogs.asked.lock().unwrap(), 0);
    let _ = std::fs::remove_dir_all(&d);
}

/// The print settings apply live: black on white, no line numbers, another header.
#[test]
fn the_print_settings_apply_to_the_next_job() {
    use jarak_gui::print_ui::{PrintChoice, Range};
    let (mut h, d) = settings_app("print-settings", &["a.rs"]);
    let printers = recording_printers(false);
    h.state_mut().printers = printers.clone();
    for (k, v) in [
        ("print_colors", "black"),
        ("print_line_numbers", "false"),
        ("print_header", "Jarak||"),
    ] {
        assert!(h.state_mut().change_setting(k, v), "{k}");
    }
    let dialogs_choice = PrintChoice {
        printer: "Paper".into(),
        devmode: None,
        range: Range::All,
    };
    h.state_mut().print_with(dialogs_choice);
    wait_print(&mut h);
    let jobs = printers.jobs.lock().unwrap();
    let t = printed_text(&jobs[0].1);
    assert!(t.contains("\"Jarak\""), "the header: {t}");
    assert!(!t.contains("\"a.rs\""), "the default header is gone");
    for line in t.lines().filter(|l| l.contains(" text ")) {
        assert!(line.contains("#000000"), "black: {line}");
    }
    assert!(!t.contains("\"1\""), "no line numbers: {t}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 106: the *Replace files?* dialog of publishing under the UI audit's rules (English and
/// Korean).
#[test]
fn the_publish_question_passes_the_ui_audit_rules() {
    let (mut h, d) = open_doc("audit106", "a.md", "# A\n");
    for lang in [Lang::En, Lang::Ko] {
        h.state_mut().lang = lang;
        h.state_mut().prepare_audit("publish-question");
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", "publish-question");
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{lang:?}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        assert!(r.components.iter().any(|c| c.id == "publish:question"));
        h.state_mut().publish_question = None;
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 107: command-line options of the window --------------------------------------------

/// HELP-03, HELP-04: `--language <name>` and `--read-only` apply to the files after them (also
/// when a running window gets them through the hand-off); an unknown language says so.
#[test]
fn language_and_read_only_options_apply_to_the_files_after_them() {
    use jarak_gui::LangChoice;
    let d = temp("open-options");
    for (f, t) in [
        ("a.txt", "fn main() {}\n"),
        ("b.txt", "plain\n"),
        ("c.txt", "x\n"),
    ] {
        std::fs::write(d.join(f), t).unwrap();
    }
    let p = |f: &str| d.join(f).display().to_string();
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.state_mut().open_args(&[
        "--language".into(),
        "rust".into(),
        p("a.txt"),
        "--read-only".into(),
        p("b.txt"),
    ]);
    h.run();
    let tab = |h: &Harness<'static, JarakApp>, name: &str| {
        h.state()
            .tabs
            .iter()
            .find(|t| t.file.path.as_deref().is_some_and(|x| x.ends_with(name)))
            .cloned()
            .unwrap()
    };
    let a = tab(&h, "a.txt");
    assert_eq!(a.lang_choice, LangChoice::Named("Rust".into()));
    assert!(!a.file.read_only, "read-only comes after a.txt");
    let b = tab(&h, "b.txt");
    assert_eq!(
        b.lang_choice,
        LangChoice::Named("Rust".into()),
        "the language stays"
    );
    assert!(b.file.read_only);
    assert!(
        b.file
            .read_only_reason
            .as_deref()
            .is_some_and(|r| r.contains("--read-only"))
    );
    h.state_mut()
        .open_args(&["--language".into(), "klingon".into(), p("c.txt")]);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("klingon"))
    );
    assert_eq!(tab(&h, "c.txt").lang_choice, LangChoice::Auto);
    let _ = std::fs::remove_dir_all(&d);
}

/// HELP-05: `--session <file>` opens a saved session's documents; `--no-session` (save off)
/// leaves the saved session as it is at exit.
#[test]
fn session_options_open_a_session_file_and_leave_the_saved_one() {
    use jarak_gui::sessions::SessionFiles;
    let d = temp("session-options");
    std::fs::write(d.join("x.md"), "# X\n").unwrap();
    let saved = jarak_core::session::Session {
        documents: vec![jarak_core::session::Doc {
            path: Some(d.join("x.md")),
            ..jarak_core::session::Doc::default()
        }],
        active: Some(0),
        ..jarak_core::session::Session::default()
    };
    saved.save(&d.join("work.json")).unwrap();
    let (a, _) = app_with(&[], Arc::default());
    let local = d.join("local");
    let mut h = harness(a.with_session(SessionFiles {
        file: local.join("session.json"),
        recovery: local.join("recovery"),
        restore: false,
        save: false,
    }));
    h.state_mut().open_args(&[
        "--session".into(),
        d.join("work.json").display().to_string(),
    ]);
    h.run();
    assert!(
        h.state()
            .tabs
            .iter()
            .any(|t| t.file.path.as_deref() == Some(d.join("x.md").as_path()))
    );
    h.state_mut().save_session();
    assert!(
        !local.join("session.json").exists(),
        "--no-session writes no session"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// HELP-10: Window → Copy debug info puts the versions and files on the clipboard.
#[test]
fn copy_debug_info_names_the_version_and_windows() {
    let (mut h, clip) = app_clip(&[], Arc::default());
    let e = jarak_gui::palette::entries();
    let i = e.iter().position(|x| x.id == "window.debug-info").unwrap();
    h.state_mut().run_entry(e[i].run);
    let t = clip.text.lock().unwrap().clone().unwrap_or_default();
    assert!(
        t.starts_with(&format!("Jarak {} (jarak-gui)", jarak_core::VERSION)),
        "{t}"
    );
    assert!(t.contains("Windows 10."), "{t}");
    assert_eq!(h.state().message.as_deref(), Some("Debug info copied"));
}

// ---- Step 108: extensions -------------------------------------------------------------------

/// The sample extension installed into `root` (a copy of examples/jarak-extensions).
fn install_sample(root: &std::path::Path) {
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/jarak-extensions/mouv.sample-tools");
    jarak_core::extensions::install(&sample, root, false, false).unwrap();
}

/// Runs frames until the extension command has finished (it runs on a thread).
fn wait_extension(h: &mut Harness<'static, JarakApp>) {
    for _ in 0..500 {
        let _ = h.try_run();
        if h.state().ext_running.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the extension command did not finish");
}

/// Step 108 (`JARAK-FR-160`, `JARAK-SEC-007`): an extension's language and commands are there;
/// the first run asks for the grant (Cancel runs nothing); once allowed, the program's output
/// replaces the selection as one undo step and the grant is kept for this manifest.
#[test]
fn an_extension_command_asks_once_and_its_output_is_undoable() {
    let d = temp("ext-run");
    let root = d.join("extensions");
    install_sample(&root);
    let doc = d.join("list.txt");
    std::fs::write(&doc, "pear\r\napple\r\nfig\r\n").unwrap();
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let grants = d.join("grants.json");
    let mut h = harness(a.with_extensions(&root, grants.clone()));
    assert!(
        h.state().user_langs.iter().any(|l| l.name == "TODO list"),
        "the extension's language"
    );
    let entries = h.state().extension_entries_for_test();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].contains("Sort the selected lines"));
    h.state_mut().tab_mut().file.doc.select_all();
    h.state_mut().run_extension(0);
    h.run();
    assert!(h.state().ext_question.is_some(), "asked first");
    assert!(h.query_by_label_contains("asks to run sort.exe").is_some());
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(
        doc_text(&h),
        "pear\r\napple\r\nfig\r\n",
        "Cancel runs nothing"
    );
    assert!(!grants.exists());
    h.state_mut().run_extension(0);
    h.run();
    h.get_by_label("Allow and run").click();
    wait_extension(&mut h);
    assert_eq!(doc_text(&h), "apple\r\nfig\r\npear\r\n");
    assert!(grants.is_file(), "the grant is kept");
    assert!(h.state_mut().tab_mut().file.doc.undo());
    assert_eq!(doc_text(&h), "pear\r\napple\r\nfig\r\n", "one undo step");
    // Granted now: the count runs without asking; its result is a message.
    h.state_mut().run_extension(1);
    assert!(h.state().ext_question.is_none());
    wait_extension(&mut h);
    let msg = h.state().message.clone().unwrap_or_default();
    assert!(
        msg.contains("Count the document's lines") && msg.contains('3'),
        "{msg}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A document changed while the command ran keeps the change; the output is not used.
#[test]
fn an_extension_output_never_overwrites_a_document_changed_meanwhile() {
    use jarak_core::extensions::Output;
    let d = temp("ext-changed");
    let doc = d.join("a.txt");
    std::fs::write(&doc, "b\na\n").unwrap();
    let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    let id = h.state().tab().id;
    let result = Arc::new(Mutex::new(Some(Ok("a\nb\n".to_string()))));
    h.state_mut().ext_running = Some(jarak_gui::ext_ui::ExtRun {
        title: "Sort".into(),
        output: Output::ReplaceSelection,
        tab: id,
        before: "something else".into(),
        range: (0, 4),
        result,
    });
    h.run();
    assert_eq!(doc_text(&h), "b\na\n");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("changed while it ran"))
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 108: the grant question under the UI audit's rules (English and Korean).
#[test]
fn the_extension_question_passes_the_ui_audit_rules() {
    let (mut h, d) = open_doc("audit108", "a.txt", "x\n");
    for lang in [Lang::En, Lang::Ko] {
        h.state_mut().lang = lang;
        h.state_mut().prepare_audit("extension-question");
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", "extension-question");
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{lang:?}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        assert!(r.components.iter().any(|c| c.id == "extension:question"));
        h.state_mut().ext_question = None;
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 109 (REL-06, SET-18): above the large-file setting a document is plain text — no
/// language, no wrap (the toggle says why), no Document Map, a *Large file* status item; below it
/// everything is back.
#[test]
fn a_large_file_is_plain_text_without_wrap_or_map() {
    let (mut h, d) = settings_app("large-file", &[]);
    let big = d.join("big.rs");
    std::fs::write(&big, "fn x() {}\n".repeat(150_000)).unwrap();
    h.state_mut().open_args(&[big.display().to_string()]);
    h.run();
    let i = h.state().active;
    assert!(
        h.state().lang_for(i).is_some(),
        "1.5 MB is not large by default"
    );
    assert!(h.state_mut().change_setting("large_file_mb", "1"));
    h.run();
    assert!(h.state().is_large(i));
    assert!(h.state().lang_for(i).is_none(), "plain text");
    h.state_mut().act(Action::ToggleWrap);
    assert!(!h.state().tab().view.wrap);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("large files"))
    );
    h.state_mut().doc_map = true;
    h.run();
    assert!(
        h.query_by_label("Large file").is_some(),
        "the status bar says so"
    );
    assert!(h.state_mut().change_setting("large_file_mb", "200"));
    h.run();
    assert!(!h.state().is_large(i));
    assert!(h.state().lang_for(i).is_some());
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 109 (`JARAK-NFR-004`): a 60 000-character line (laid out only around what is on screen)
/// still takes End, typing at its end, Home and a click at the right place.
#[test]
fn a_very_long_line_is_edited_at_both_ends() {
    let line = "abcdefghij".repeat(6_000);
    let (mut h, d) = open_doc("long-line", "long.txt", &format!("{line}\nshort\n"));
    key(&mut h, egui::Key::End, egui::Modifiers::NONE);
    text(&mut h, "Z");
    let t = doc_text(&h);
    assert_eq!(t.lines().next().unwrap().len(), 60_001);
    assert!(t.starts_with("abcdefghij") && t.lines().next().unwrap().ends_with("jZ"));
    h.run();
    // The caret at the end is drawn where it is, and a click there keeps it there.
    let head = h.state().tab().file.doc.selection().primary().head;
    assert_eq!(head, 60_001);
    let p = {
        let tb = h.state().tab();
        tb.view
            .pos_of_char(&tb.file.doc, head - 1)
            .expect("on screen")
    };
    h.event(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    let _ = h.try_run();
    let at = h.state().tab().file.doc.selection().primary().head;
    assert!(
        (59_999..=60_001).contains(&at),
        "clicked near the end: {at}"
    );
    key(&mut h, egui::Key::Home, egui::Modifiers::NONE);
    text(&mut h, "A");
    assert!(doc_text(&h).starts_with("Aabcdefghij"));
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 110: the remaining Notepad++ built-ins (#129) ----------------------------------------

fn two_files(name: &str) -> (PathBuf, Vec<String>) {
    let d = temp(name);
    let mut v = Vec::new();
    for (f, t) in [("a.txt", "one\n"), ("b.txt", "two\n")] {
        std::fs::write(d.join(f), t).unwrap();
        v.push(d.join(f).display().to_string());
    }
    (d, v)
}

fn select(h: &mut Harness<'static, JarakApp>, a: usize, b: usize) {
    h.state_mut()
        .tab_mut()
        .file
        .doc
        .set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(a, b),
        ));
    h.run();
}

fn tab_count(h: &Harness<'static, JarakApp>) -> usize {
    h.query_all_by_role(egui::accesskit::Role::Tab).count()
}

/// VIEW-01…04 (`JARAK-FR-036`): always on top, full screen (F11), post-it (F12) and
/// distraction-free toggle; post-it and distraction-free hide the menu, tab and status bars;
/// distraction-free draws the text in a centred column; the window is told once per change.
#[test]
fn window_modes_toggle_and_hide_the_chrome() {
    let (d, files) = two_files("modes");
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let (a, _) = app_with(&refs, Arc::default());
    let mut h = harness(a);
    assert!(h.query_by_label("Settings").is_some(), "the status bar");
    assert_eq!(tab_count(&h), 2, "the tab bar");
    palette_run(&mut h, "Always on top");
    h.run();
    assert!(h.state().modes.on_top);
    assert_eq!(
        h.state().modes_applied,
        h.state().modes,
        "the window was told"
    );
    key(&mut h, egui::Key::F11, egui::Modifiers::NONE);
    assert!(h.state().modes.full_screen);
    key(&mut h, egui::Key::F12, egui::Modifiers::NONE);
    h.run();
    assert!(h.state().modes.post_it);
    assert!(
        h.query_by_label("Settings").is_none(),
        "post-it: no status bar"
    );
    assert_eq!(tab_count(&h), 0, "post-it: no tab bar");
    key(&mut h, egui::Key::F12, egui::Modifiers::NONE);
    h.run();
    assert!(h.query_by_label("Settings").is_some());
    assert_eq!(tab_count(&h), 2);
    let full_width = h.get_by_label("Text").rect().width();
    palette_run(&mut h, "Distraction-free mode");
    h.run();
    assert!(h.state().modes.distraction_free);
    assert!(
        h.query_by_label("Settings").is_none(),
        "distraction-free: no status bar"
    );
    assert_eq!(tab_count(&h), 0);
    let w = h.get_by_label("Text").rect().width();
    assert!(
        w <= jarak_gui::modes::FOCUS_WIDTH + 1.0 && w < full_width,
        "a centred column: {w} of {full_width}"
    );
    palette_run(&mut h, "Distraction-free mode");
    key(&mut h, egui::Key::F11, egui::Modifiers::NONE);
    palette_run(&mut h, "Always on top");
    h.run();
    assert_eq!(h.state().modes, jarak_gui::modes::Modes::default());
    assert_eq!(h.state().modes_applied, jarak_gui::modes::Modes::default());
    assert!(h.query_by_label("Settings").is_some());
    let _ = std::fs::remove_dir_all(&d);
}

/// Two clicks on consecutive frames (a harness stepping less than the double-click time).
fn double_click_at(h: &mut Harness<'static, JarakApp>, p: egui::Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run();
    for _ in 0..2 {
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.step();
    }
    h.run();
}

fn right_click(h: &mut Harness<'static, JarakApp>, p: egui::Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    h.run();
}

/// UI-08, SET-25 (`JARAK-FR-031`, `JARAK-FR-130`): a right click on the text opens its context
/// menu with the setting's commands; an item runs its command; changing the setting changes the
/// menu.
#[test]
fn the_text_context_menu_runs_the_settings_commands() {
    let (mut h, d) = settings_app("ctx-menu", &["a.txt"]);
    let p = screen_of(&h, 1);
    right_click(&mut h, p);
    for item in [
        "Undo",
        "Redo",
        "Cut",
        "Copy",
        "Paste (text or an image)",
        "Select all",
        "Copy as HTML",
    ] {
        assert!(h.query_by_label(item).is_some(), "{item} in the menu");
    }
    h.get_by_label("Select all").click();
    h.run();
    assert_eq!(sel_text(&h), "one\ntwo\n");
    assert!(h.query_by_label("Redo").is_none(), "the menu closed");
    assert!(
        h.state_mut()
            .change_setting("context_menu", "edit.copy,-,search.next-non-ascii")
    );
    h.run();
    right_click(&mut h, p);
    assert!(
        h.query_by_label("Find the next non-ASCII character")
            .is_some()
    );
    assert!(
        h.query_by_label("Undo").is_none(),
        "only the setting's commands"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// FILE-10 (`JARAK-FR-002`): Save a copy writes the text to another file; the document keeps its
/// own file and its unsaved state; its own file is refused as the copy's name.
#[test]
fn save_a_copy_keeps_the_documents_file_and_unsaved_state() {
    let d = temp("save-copy");
    let a = d.join("a.txt");
    std::fs::write(&a, "one\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    dialogs.save.lock().unwrap().push(d.join("copy.txt"));
    let (app, _) = app_with(&[a.to_str().unwrap()], dialogs.clone());
    let mut h = harness(app);
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    text(&mut h, "two");
    palette_run(&mut h, "Save a copy as");
    assert_eq!(
        std::fs::read_to_string(d.join("copy.txt")).unwrap(),
        "one\ntwo"
    );
    assert_eq!(
        std::fs::read_to_string(&a).unwrap(),
        "one\n",
        "the file is unchanged"
    );
    assert_eq!(h.state().tab().file.path.as_deref(), Some(a.as_path()));
    assert!(h.state().tab().file.doc.is_dirty(), "still unsaved");
    dialogs.save.lock().unwrap().push(a.clone());
    palette_run(&mut h, "Save a copy as");
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "one\n");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("another name"))
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// EDIT-11 (`JARAK-FR-024`): Edit → Read-only toggles the document's flag (typing is refused
/// while it is on); a document read-only because its bytes cannot be reproduced stays so; Clear
/// the file's read-only attribute makes the file writable.
#[test]
fn the_read_only_flag_toggles_and_the_files_attribute_clears() {
    let d = temp("ro-toggle");
    let a = d.join("a.txt");
    std::fs::write(&a, "one\n").unwrap();
    let mut perm = std::fs::metadata(&a).unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(&a, perm).unwrap();
    let blob = d.join("blob.bin");
    std::fs::write(&blob, b"PK\x03\x04\x00\x00data").unwrap();
    let (app, _) = app_with(&[a.to_str().unwrap()], Arc::default());
    let mut h = harness(app);
    palette_run(&mut h, "Read-only: on/off");
    assert!(h.state().tab().file.read_only);
    text(&mut h, "x");
    assert_eq!(doc_text(&h), "one\n", "typing refused");
    palette_run(&mut h, "Read-only: on/off");
    assert!(!h.state().tab().file.read_only);
    text(&mut h, "x");
    assert_eq!(doc_text(&h), "xone\n");
    palette_run(&mut h, "Clear the file's read-only attribute");
    assert!(!std::fs::metadata(&a).unwrap().permissions().readonly());
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(
        std::fs::read_to_string(&a).unwrap(),
        "xone\n",
        "now it saves"
    );
    // Binary text stays read-only.
    h.state_mut().open_args(&[blob.display().to_string()]);
    h.run();
    palette_run(&mut h, "Read-only: on/off");
    assert!(h.state().tab().file.read_only);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("stays read-only"))
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// VIEWS-08 (`JARAK-FR-060`): a tab takes a colour (named in its accessible label, drawn along
/// its edge), loses it again, and a saved session keeps it.
#[test]
fn a_tab_takes_a_colour_and_the_session_keeps_it() {
    use jarak_gui::sessions::SessionFiles;
    let (d, files) = two_files("tab-color");
    let sf = SessionFiles {
        file: d.join("local").join("session.json"),
        recovery: d.join("local").join("recovery"),
        restore: true,
        save: true,
    };
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_session(sf.clone()));
    h.state_mut().open_args(&files);
    h.run();
    palette_run(&mut h, "Tab colour: green");
    h.run();
    assert_eq!(h.state().tab().color, Some(1));
    assert!(h.query_by_label_contains("b.txt (Green)").is_some());
    h.state_mut().save_session();
    let (b, _) = app_with(&[], Arc::default());
    let h2 = harness(b.with_session(sf));
    let t = h2
        .state()
        .tabs
        .iter()
        .find(|t| t.title() == "b.txt")
        .expect("b restored");
    assert_eq!(t.color, Some(1), "the session keeps the colour");
    palette_run(&mut h, "Tab colour: none");
    h.run();
    assert_eq!(h.state().tab().color, None);
    assert!(h.query_by_label_contains("(Green)").is_none());
    let _ = std::fs::remove_dir_all(&d);
}

/// EDIT-13 (`JARAK-FR-087`): Paste HTML / RTF as source inserts the clipboard's HTML fragment or
/// RTF as text; without them it says so and leaves the document alone.
#[test]
fn pasting_html_and_rtf_as_source_inserts_their_text() {
    let (mut h, clip) = app_clip(&[], Arc::default());
    palette_run(&mut h, "Paste HTML as source");
    assert_eq!(doc_text(&h), "");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("no HTML"))
    );
    *clip.html.lock().unwrap() = Some("<b>한</b>".into());
    *clip.rtf.lock().unwrap() = Some(r"{\rtf1 x}".into());
    palette_run(&mut h, "Paste HTML as source");
    assert_eq!(doc_text(&h), "<b>한</b>");
    palette_run(&mut h, "Paste RTF as source");
    assert_eq!(doc_text(&h), r"<b>한</b>{\rtf1 x}");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "<b>한</b>", "each paste is one undo step");
}

/// EDIT-16 (`JARAK-FR-087`): the character panel inserts a character by its code or from its
/// 32–255 table, and closes with Esc.
#[test]
fn the_character_panel_inserts_by_code_and_from_the_table() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    palette_run(&mut h, "Character panel");
    assert!(h.state().gap_dialog.is_some());
    h.get_by_label("Character code").click();
    h.run();
    text(&mut h, "U+00E9");
    assert!(
        h.query_by_label("é  U+00E9").is_some(),
        "the panel shows it"
    );
    h.get_by_label("Insert").click();
    h.run();
    assert_eq!(doc_text(&h), "é");
    h.get_by_label(" 35   0x23   #").click();
    h.run();
    assert_eq!(doc_text(&h), "é#");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().gap_dialog.is_none());
    for (code, ch) in [
        ("0x41", 'A'),
        ("233", 'é'),
        ("&#xD55C;", '한'),
        ("U+1F600", '😀'),
    ] {
        assert_eq!(
            jarak_core::lineops::parse_char_code(code),
            Some(ch),
            "{code}"
        );
    }
    assert_eq!(
        jarak_core::lineops::parse_char_code("U+D800"),
        None,
        "a surrogate"
    );
}

/// EDIT-17 (`JARAK-FR-087`): what Jarak copies, cuts or pastes is remembered (newest first,
/// once each); a pick from the history pastes it.
#[test]
fn the_clipboard_history_pastes_an_earlier_copy() {
    let (mut h, _clip) = app_clip(&[], Arc::default());
    text(&mut h, "alpha beta");
    select(&mut h, 0, 5);
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    select(&mut h, 6, 10);
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    select(&mut h, 0, 5);
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    assert_eq!(
        h.state().clip_history,
        ["alpha", "beta"],
        "newest first, once"
    );
    key(&mut h, egui::Key::End, egui::Modifiers::CTRL);
    palette_run(&mut h, "Paste from clipboard history");
    h.get_by_label("beta").click();
    h.run();
    assert_eq!(doc_text(&h), "alpha betabeta");
    assert!(h.state().gap_dialog.is_none(), "a pick closes it");
}

/// SRCH-20 (`JARAK-FR-070`): the next non-ASCII character, the next one above U+00FF and a
/// range from the dialog select the next match after the caret, wrapping.
#[test]
fn characters_in_a_range_are_found_one_after_another() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "ab é c 中 d");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    palette_run(&mut h, "Find the next non-ASCII character");
    assert_eq!(sel_text(&h), "é");
    palette_run(&mut h, "Find the next non-ASCII character");
    assert_eq!(sel_text(&h), "中");
    palette_run(&mut h, "Find the next non-ASCII character");
    assert_eq!(sel_text(&h), "é", "it wraps");
    palette_run(&mut h, "Find the next character above U+00FF");
    assert_eq!(sel_text(&h), "中");
    key(&mut h, egui::Key::Home, egui::Modifiers::CTRL);
    palette_run(&mut h, "Find characters in a range");
    h.get_by_label("Find next").click();
    h.run();
    assert_eq!(sel_text(&h), "é", "U+0080–U+00FF");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.state_mut().find_char_range(0x400, 0x4FF);
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("No character"))
    );
}

fn drag_text(
    h: &mut Harness<'static, JarakApp>,
    from: egui::Pos2,
    to: egui::Pos2,
    release: egui::Modifiers,
) {
    h.event(egui::Event::PointerMoved(from));
    h.run();
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    let _ = h.try_run();
    for k in 1..=4 {
        h.event(egui::Event::PointerMoved(
            from + (to - from) * (k as f32 / 4.0),
        ));
        let _ = h.try_run();
    }
    if release != egui::Modifiers::NONE {
        h.event(egui::Event::ModifiersChanged(release));
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: release,
    });
    let _ = h.try_run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    let _ = h.try_run();
}

/// EDIT-18 (`JARAK-FR-032`): dragging the selection moves it as one undo step; Ctrl held at the
/// release copies it; a click inside the selection only places the caret; a read-only document
/// keeps its text.
#[test]
fn dragging_the_selection_moves_or_copies_it() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    text(&mut h, "abc def ghi");
    select(&mut h, 0, 3);
    let (from, to) = (screen_of(&h, 1), screen_of(&h, 11));
    drag_text(&mut h, from, to, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), " def ghiabc");
    assert_eq!(sel_text(&h), "abc", "the dropped text is selected");
    key(&mut h, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(doc_text(&h), "abc def ghi", "one undo step");
    select(&mut h, 4, 7);
    let (from, to) = (screen_of(&h, 5), screen_of(&h, 0));
    drag_text(&mut h, from, to, CTRL_CMD);
    assert_eq!(doc_text(&h), "defabc def ghi", "Ctrl copies");
    assert_eq!(sel_range(&h), (0, 3));
    // Away from the last press (no double click).
    select(&mut h, 4, 7);
    let p = screen_of(&h, 6);
    click_at(&mut h, p, egui::Modifiers::NONE);
    assert_eq!(sel_range(&h), (6, 6), "a click inside only moves the caret");
    h.state_mut().file_op(1);
    select(&mut h, 0, 3);
    let (from, to) = (screen_of(&h, 1), screen_of(&h, 10));
    drag_text(&mut h, from, to, egui::Modifiers::NONE);
    assert_eq!(doc_text(&h), "defabc def ghi", "read-only: nothing moves");
}

/// SET-06, SET-07, SET-13, SET-14 (`JARAK-FR-130`): new documents take the settings' language;
/// the Open dialog starts in the document's folder (or Windows' last one); Ctrl+F fills the
/// find field from the selection only when set; a save keeps the previous file as `.bak` when
/// set.
#[test]
fn new_language_open_folder_find_fill_and_backup_settings_apply() {
    use jarak_gui::LangChoice;
    let d = temp("s110");
    let a = d.join("a.txt");
    std::fs::write(&a, "one\ntwo\n").unwrap();
    let dialogs = Arc::new(Scripted::default());
    let (app, _) = app_with(&[a.to_str().unwrap()], dialogs.clone());
    let mut h = harness(
        app.with_settings(d.join("settings.toml"))
            .with_hotkeys(&d.join("hotkeys.json")),
    );
    // SET-07: the document's folder by default, Windows' remembered folder when set.
    key(&mut h, egui::Key::O, egui::Modifiers::CTRL);
    assert_eq!(
        dialogs.open_dirs.lock().unwrap().last(),
        Some(&Some(d.clone()))
    );
    assert!(h.state_mut().change_setting("open_folder", "last"));
    key(&mut h, egui::Key::O, egui::Modifiers::CTRL);
    assert_eq!(dialogs.open_dirs.lock().unwrap().last(), Some(&None));
    // SET-13: the selection fills the find field only when set.
    select(&mut h, 0, 3);
    key(&mut h, egui::Key::F, egui::Modifiers::CTRL);
    assert_eq!(h.state().find.find, "one");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state_mut().change_setting("find_fill_selection", "false"));
    h.state_mut().find.find = "kept".into();
    select(&mut h, 4, 7);
    key(&mut h, egui::Key::F, egui::Modifiers::CTRL);
    assert_eq!(h.state().find.find, "kept");
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    // SET-14: no backup by default; `<name>.bak` with the previous text when set.
    select(&mut h, 8, 8);
    text(&mut h, "3");
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert!(!d.join("a.txt.bak").exists());
    assert!(h.state_mut().change_setting("backup", "simple"));
    text(&mut h, "4");
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "one\ntwo\n34");
    assert_eq!(
        std::fs::read_to_string(d.join("a.txt.bak")).unwrap(),
        "one\ntwo\n3",
        "the previous file"
    );
    assert!(h.state_mut().change_setting("backup", "dated"));
    text(&mut h, "5");
    key(&mut h, egui::Key::S, egui::Modifiers::CTRL);
    let dated: Vec<_> = std::fs::read_dir(d.join("jarak-backup"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dated.len(), 1, "one dated copy");
    assert_eq!(std::fs::read_to_string(&dated[0]).unwrap(), "one\ntwo\n34");
    // SET-07: a fixed folder (one that is gone falls back to the document's).
    let fixed = d.join("fixed");
    std::fs::create_dir_all(&fixed).unwrap();
    assert!(h.state_mut().change_setting("open_folder", "folder"));
    assert!(
        h.state_mut()
            .change_setting("open_folder_path", &fixed.display().to_string())
    );
    key(&mut h, egui::Key::O, egui::Modifiers::CTRL);
    assert_eq!(
        dialogs.open_dirs.lock().unwrap().last(),
        Some(&Some(fixed.clone()))
    );
    std::fs::remove_dir_all(&fixed).unwrap();
    key(&mut h, egui::Key::O, egui::Modifiers::CTRL);
    assert_eq!(
        dialogs.open_dirs.lock().unwrap().last(),
        Some(&Some(d.clone()))
    );
    // SET-06: a new document in the settings' language.
    assert!(h.state_mut().change_setting("new_language", "Markdown"));
    key(&mut h, egui::Key::N, egui::Modifiers::CTRL);
    assert_eq!(
        h.state().tab().lang_choice,
        LangChoice::Named("Markdown".into())
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 110 (`JARAK-NFR-007`): the character panel, the clipboard history, the range dialog, the
/// context menu and distraction-free mode pass the UI audit rules.
#[test]
fn the_step_110_surfaces_pass_the_ui_audit_rules() {
    let (mut h, _clip) = app_clip(&[], Arc::default());
    text(&mut h, "alpha é");
    select(&mut h, 0, 5);
    key(&mut h, egui::Key::C, egui::Modifiers::CTRL);
    for screen in [
        "char-panel",
        "clipboard-history",
        "char-range",
        "context-menu",
        "distraction-free",
    ] {
        match screen {
            "char-panel" => palette_run(&mut h, "Character panel"),
            "clipboard-history" => palette_run(&mut h, "Paste from clipboard history"),
            "char-range" => palette_run(&mut h, "Find characters in a range"),
            "context-menu" => {
                let p = screen_of(&h, 2);
                right_click(&mut h, p);
            }
            _ => {
                key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
                palette_run(&mut h, "Distraction-free mode");
            }
        }
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        if screen != "context-menu" {
            key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
        }
    }
}

/// Step 110: the UI audit's screens (the e2e's `step110_jarak_parity` photographs them in the
/// release window) show what the e2e looks for, headless.
#[test]
fn the_step_110_audit_screens_show_their_content() {
    let d = temp("audit110");
    let doc = d.join("note.txt");
    std::fs::write(&doc, "Café 中文 and ASCII\nsecond line\n").unwrap();
    for screen in [
        "char-panel",
        "clipboard-history",
        "char-range",
        "distraction-free",
        "post-it",
        "tab-color",
    ] {
        let (a, _) = app_with(&[doc.to_str().unwrap()], Arc::default());
        let mut h = harness(a);
        h.state_mut().prepare_audit(screen);
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        let t: Vec<String> = r
            .components
            .iter()
            .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
            .collect();
        let has = |s: &str| t.iter().any(|x| x.contains(s));
        match screen {
            "char-panel" => assert!(has("233   0xE9   é") && has("Insert"), "{t:?}"),
            "clipboard-history" => assert!(has("alpha") && has("(2 lines)"), "{t:?}"),
            "char-range" => assert!(has("Find next"), "{t:?}"),
            "tab-color" => {
                assert!(has("note.txt") && has("Untitled"), "two tabs: {t:?}");
                assert_eq!(h.state().tab().color, Some(1));
            }
            _ => {
                assert!(t.iter().any(|x| x == "2"), "the line numbers: {t:?}");
                assert!(!has("UTF-8"), "no status bar: {t:?}");
                assert_eq!(
                    h.state().modes_applied,
                    h.state().modes,
                    "the audit's window is never made full screen or frameless"
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// SET-11, SET-17, UI-02 (`JARAK-FR-130`, `JARAK-FR-033`): smart highlighting follows its
/// settings (off, any case, part words); further word characters join words for a double click;
/// the status bar shows the document's length and lines.
#[test]
fn smart_highlighting_word_characters_and_the_length_summary() {
    let d = temp("s110b");
    let (a, _) = app_with(&[], Arc::default());
    // Frames 1/60 s apart, so two clicks on consecutive frames are a double click.
    let mut h = Harness::builder()
        .with_size(egui::vec2(960.0, 640.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, app: &mut JarakApp| app.ui(ui),
            a.with_settings(d.join("settings.toml"))
                .with_hotkeys(&d.join("hotkeys.json")),
        );
    h.run();
    text(&mut h, "one One someone foo-bar");
    select(&mut h, 0, 3);
    h.run();
    assert_eq!(
        h.state().tab().view.marks,
        Vec::<(usize, usize)>::new(),
        "case, whole words"
    );
    assert!(h.state_mut().change_setting("smart_match_case", "false"));
    select(&mut h, 0, 3);
    h.run();
    assert_eq!(h.state().tab().view.marks, vec![(4, 7)]);
    assert!(h.state_mut().change_setting("smart_whole_word", "false"));
    select(&mut h, 0, 3);
    h.run();
    assert_eq!(h.state().tab().view.marks, vec![(4, 7), (12, 15)]);
    assert!(h.state_mut().change_setting("smart_highlight", "false"));
    select(&mut h, 0, 3);
    h.run();
    assert_eq!(
        h.state().tab().view.marks,
        Vec::<(usize, usize)>::new(),
        "off"
    );
    // SET-17: a double click on "bar" takes "foo-bar" once `-` is a word character.
    let p = screen_of(&h, 21);
    double_click_at(&mut h, p);
    assert_eq!(sel_text(&h), "bar");
    assert!(h.state_mut().change_setting("word_chars", "-"));
    h.run();
    let p = screen_of(&h, 21);
    // A click elsewhere first, so the next two are a new double click.
    let q = screen_of(&h, 2);
    click_at(&mut h, q, egui::Modifiers::NONE);
    double_click_at(&mut h, p);
    assert_eq!(sel_text(&h), "foo-bar");
    // UI-02: the length and lines.
    assert!(h.query_by_label("23 chars · 1 lines").is_some());
    let _ = std::fs::remove_dir_all(&d);
}

/// SET-10 (`JARAK-FR-130`): a language's own tab width and spaces override the general ones for
/// its documents only.
#[test]
fn a_language_has_its_own_tab_width() {
    assert_eq!(
        jarak_gui::prefs::parse_lang_tabs("Python=4s, Go = 8t ,YAML=2, bad, X=0, Y=17s, =3"),
        vec![
            ("Python".to_string(), 4, Some(true)),
            ("Go".to_string(), 8, Some(false)),
            ("YAML".to_string(), 2, None),
        ]
    );
    let (mut h, d) = settings_app("lang-tabs", &["a.rs", "b.txt"]);
    assert!(h.state_mut().change_setting("language_tabs", "Rust=2s"));
    h.run();
    let tab = |h: &Harness<'static, JarakApp>, n: &str| {
        let t = h.state().tabs.iter().find(|t| t.title() == n).unwrap();
        (t.file.doc.tab_width, t.file.doc.use_spaces)
    };
    assert_eq!(tab(&h, "a.rs"), (2, true), "Rust's own");
    assert_eq!(tab(&h, "b.txt"), (4, false), "the general settings");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 111: the Help menu, the Open with offer (#130) ---------------------------------------

/// HELP-11 (`JARAK-FR-171`): Help → User guide opens the guide beside the program read-only in the
/// preview (the file unchanged); About shows the version and copies the debug information; a
/// missing guide is said.
#[test]
fn the_help_menu_opens_the_guide_and_about() {
    let d = temp("help");
    std::fs::create_dir_all(d.join("docs")).unwrap();
    let guide = d.join("docs").join("JARAK_GUIDE.md");
    let body = "# Jarak — user guide\n\nText.\n";
    std::fs::write(&guide, body).unwrap();
    let (mut h, clip) = app_clip(&[], Arc::default());
    h.state_mut().guide_dir = Some(d.clone());
    palette_run(&mut h, "User guide");
    {
        let t = h.state().tab();
        assert_eq!(
            t.file.path.as_deref(),
            Some(std::path::absolute(&guide).unwrap().as_path())
        );
        assert!(t.file.read_only, "the guide is read-only");
        assert_eq!(t.mode, jarak_gui::Mode::Preview);
    }
    h.state_mut().tab_mut().set_mode(jarak_gui::Mode::Source);
    h.run();
    text(&mut h, "x");
    assert_eq!(doc_text(&h), body, "typing refused");
    assert_eq!(std::fs::read_to_string(&guide).unwrap(), body);
    palette_run(&mut h, "About Jarak");
    assert!(h.state().help_dialog.is_some());
    h.get_by_label("Copy debug info").click();
    h.run();
    assert!(
        clip.text
            .lock()
            .unwrap()
            .as_deref()
            .is_some_and(|t| t.starts_with("Jarak 0.0."))
    );
    h.get_by_label("Close").click();
    h.run();
    assert!(h.state().help_dialog.is_none());
    h.state_mut().guide_dir = Some(d.join("nowhere"));
    palette_run(&mut h, "User guide");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("not beside this program"))
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// SET-09 (`JARAK-FR-130`, `KIM-SEC-001`): Help → Offer Jarak in "Open with"… writes Jarak's
/// entries for the checked extensions only (a memory store here — tests never touch the real
/// registry) and never a default; Remove takes back exactly that.
#[test]
fn offering_jarak_in_open_with_and_taking_it_back() {
    use jarak_core::assoc::{PROG_ID, SharedMemory, Store};
    let d = temp("offer");
    let shared = SharedMemory::default();
    let record = d.join("associations.json");
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a.with_associations(
        Box::new(shared.clone()),
        record.clone(),
        Some(PathBuf::from(r"C:\x\jarak-gui.exe")),
    ));
    palette_run(&mut h, "Remove Jarak from");
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("nothing to remove"))
    );
    palette_run(&mut h, "Offer Jarak in");
    assert!(matches!(
        h.state().help_dialog,
        Some(jarak_gui::help::HelpDialog::Offer(_))
    ));
    h.get_by_label(".log").click();
    h.run();
    h.get_by_label("Offer").click();
    h.run();
    for (ext, on) in [
        ("md", true),
        ("txt", true),
        ("markdown", true),
        ("log", false),
        ("json", false),
    ] {
        assert_eq!(
            shared
                .get(&format!(".{ext}\\OpenWithProgids"), PROG_ID)
                .unwrap()
                .is_some(),
            on,
            ".{ext}"
        );
    }
    assert_eq!(shared.get(".md", "").unwrap(), None, "no default is set");
    assert!(record.is_file());
    assert!(
        h.state()
            .message
            .as_deref()
            .is_some_and(|m| m.contains("Default apps"))
    );
    palette_run(&mut h, "Remove Jarak from");
    assert!(
        shared.0.lock().unwrap().keys.is_empty(),
        "everything Jarak added is gone"
    );
    assert!(!record.exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 111 (`JARAK-NFR-007`): About and the Open with offer pass the UI audit rules.
#[test]
fn the_help_dialogs_pass_the_ui_audit_rules() {
    for screen in ["about", "offer-open-with"] {
        let (a, _) = app_with(&[], Arc::default());
        let mut h = harness(a.with_associations(
            Box::new(jarak_core::assoc::SharedMemory::default()),
            temp("audit111").join("associations.json"),
            None,
        ));
        h.state_mut().prepare_audit(screen);
        h.run();
        kim_ui::inspect::force(true);
        h.run();
        let r = kim_ui::inspect::report(&h.ctx, "jarak", screen);
        kim_ui::inspect::force(false);
        let fails = kim_design::audit::audit(&r);
        assert_eq!(
            kim_design::audit::counts(&fails).2,
            0,
            "{screen}: {}",
            kim_design::audit::render(&r, &fails, false)
        );
        let t: Vec<String> = r
            .components
            .iter()
            .filter_map(|c| c.text.as_ref().map(|t| t.content.clone()))
            .collect();
        let want = if screen == "about" {
            "Copy debug info"
        } else {
            "Offer"
        };
        assert!(t.iter().any(|x| x == want), "{screen}: {t:?}");
    }
}

/// FILE-02 (`JARAK-FR-020`): one Ctrl+O opens every file picked in the dialog (the real check
/// of 2026-11-23 found the Windows dialog allowed only one); a file already open is not opened
/// twice.
#[test]
fn one_open_dialog_opens_several_files() {
    let d = temp("multi-open");
    let (a, b, c) = (d.join("a.txt"), d.join("b.md"), d.join("c.log"));
    for (p, t) in [(&a, "A\n"), (&b, "# B\n"), (&c, "C\n")] {
        std::fs::write(p, t).unwrap();
    }
    let dialogs = Arc::new(Scripted::default());
    dialogs
        .open_many
        .lock()
        .unwrap()
        .push(vec![a.clone(), b, c]);
    let (app, _) = app_with(&[a.to_str().unwrap()], dialogs.clone());
    let mut h = harness(app);
    key(&mut h, egui::Key::O, egui::Modifiers::CTRL);
    assert_eq!(*dialogs.asked.lock().unwrap(), 1, "one dialog");
    assert_eq!(h.state().tabs.len(), 3, "a.txt was open already");
    assert_eq!(h.state().window_title(), "c.log — Jarak");
    let _ = std::fs::remove_dir_all(&d);
}

/// FILE-02: the multiple-selection dialog's answer — one full path, or the folder and the names
/// picked in it.
#[test]
fn a_multiple_selection_answer_is_read_as_paths() {
    use jarak_gui::multi_select_paths;
    let w = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    assert_eq!(
        multi_select_paths(&w("C:\\notes\\a.txt\0\0")),
        [PathBuf::from("C:\\notes\\a.txt")]
    );
    assert_eq!(
        multi_select_paths(&w("C:\\notes\0a.txt\0자락 b.md\0\0junk")),
        [
            PathBuf::from("C:\\notes\\a.txt"),
            PathBuf::from("C:\\notes\\자락 b.md")
        ]
    );
    assert!(multi_select_paths(&w("\0\0")).is_empty());
}

/// `JARAK-FR-040`, HELP-11: the preview (where the read-only user guide opens) runs the commands
/// that do not edit the text — the palette, the window modes — and still refuses typing (the real
/// check of 2026-11-23 found Ctrl+Shift+P refused in the guide).
#[test]
fn the_preview_opens_the_palette_and_the_window_modes() {
    let d = temp("preview-cmds");
    let p = d.join("n.md");
    std::fs::write(&p, "# Note\n\ntext\n").unwrap();
    let (a, _) = app_with(&[p.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    key(&mut h, egui::Key::E, egui::Modifiers::CTRL);
    assert_eq!(h.state().tab().mode, jarak_gui::Mode::Preview);
    key(
        &mut h,
        egui::Key::P,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert!(
        h.state().palette.is_some(),
        "the palette opens in the preview"
    );
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().palette.is_none());
    key(&mut h, egui::Key::F11, egui::Modifiers::NONE);
    assert!(h.state().modes.full_screen);
    key(&mut h, egui::Key::F11, egui::Modifiers::NONE);
    assert!(!h.state().modes.full_screen);
    assert_eq!(
        h.state().tab().mode,
        jarak_gui::Mode::Preview,
        "still the preview"
    );
    text(&mut h, "x");
    assert_eq!(
        h.state().tab().file.doc.text(),
        "# Note\n\ntext\n",
        "typing changes nothing"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `JARAK-NFR-007`, DESIGN_SYSTEM §4.3: under a Windows contrast theme the window draws with its
/// system colours only (no lilac accent, the editor's keywords in the highlight colour, readable
/// selection), and goes back to its own theme when the contrast theme is turned off.
#[test]
fn a_contrast_theme_draws_with_the_system_colours() {
    use jarak_core::syntax::Kind;
    use kim_ui::{Colors, SystemColors};
    let d = temp("contrast");
    let p = d.join("a.rs");
    std::fs::write(&p, "fn main() {}\n").unwrap();
    let (mut a, _) = app_with(&[p.to_str().unwrap()], Arc::default());
    a.high_contrast = Some(Some(SystemColors::BLACK));
    let mut h = harness(a);
    h.run();
    let c = Colors::get(&h.ctx);
    assert_eq!(c.bg_base, egui::Color32::BLACK);
    assert_eq!(c.text_primary, egui::Color32::WHITE);
    assert_eq!(c.accent, egui::Color32::from_rgb(0x1A, 0xEB, 0xFF));
    assert_eq!(c.text_on_accent, egui::Color32::BLACK);
    assert!(
        h.ctx.global_style().visuals.dark_mode,
        "High Contrast Black is dark"
    );
    assert_eq!(
        jarak_gui::languages::syntax_color(&c, Kind::Keyword),
        c.accent,
        "keywords in the highlight colour"
    );
    assert_eq!(
        jarak_gui::languages::syntax_color(&c, Kind::String),
        egui::Color32::WHITE
    );
    h.state_mut().high_contrast = Some(Some(SystemColors::WHITE));
    h.run();
    let c = Colors::get(&h.ctx);
    assert_eq!(c.bg_base, egui::Color32::WHITE);
    assert!(!h.ctx.global_style().visuals.dark_mode);
    h.state_mut().high_contrast = Some(None);
    h.run();
    let c = Colors::get(&h.ctx);
    assert_ne!(c.bg_base, egui::Color32::WHITE);
    assert_eq!(kim_ui::high_contrast(&h.ctx), None, "back to the theme");
    let _ = std::fs::remove_dir_all(&d);
}

/// `JARAK-NFR-003`, `JARAK-NFR-004`: a large plain-text document starts no structure worker after
/// an edit (the real 1 GB check of 2026-11-23 measured ~5 s of CPU after one typed character), and
/// a large code file shown as plain text (large-file mode) neither; a code file below the limit
/// still gets its structure.
#[test]
fn plain_and_large_documents_find_no_structure_after_an_edit() {
    let d = temp("no-outline");
    let p = d.join("big.txt");
    let line = "a line of plain text for the large document test\n";
    std::fs::write(&p, line.repeat(60_000)).unwrap();
    let (a, _) = app_with(&[p.to_str().unwrap()], Arc::default());
    let mut h = harness(a);
    h.run();
    text(&mut h, "x");
    for _ in 0..20 {
        h.step();
        assert!(
            !h.state().tabs[0].outline.working(),
            "no worker for plain text"
        );
    }
    let r = d.join("big.rs");
    std::fs::write(&r, "fn f() {\n    let x = 1;\n}\n".repeat(60_000)).unwrap();
    let (a, _) = app_with(&[r.to_str().unwrap()], Arc::default());
    let mut h = harness(a.with_settings(d.join("settings.toml")));
    assert!(h.state_mut().change_setting("large_file_mb", "1"));
    h.run();
    assert!(
        h.state().is_large(0),
        "{} bytes",
        h.state().tabs[0].file.doc.text().len_bytes()
    );
    text(&mut h, "x");
    for _ in 0..20 {
        h.step();
        assert!(
            !h.state().tabs[0].outline.working(),
            "large-file mode: plain text"
        );
    }
    assert!(h.state().tabs[0].outline.folds.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}
