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
    // The image decoder (a real thread) may wake the window a few more times.
    let _ = h.try_run();
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
        Some("TODO\r\nTODO\r\nTODO")
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
    assert_eq!(h.state().tab().file.doc.markers().len(), 1);
    h.state_mut().style(StyleCmd::Clear(0));
    assert!(h.state().tab().file.doc.markers().is_empty());
}

#[test]
fn the_find_history_walks_with_the_arrows_and_lives_in_the_session() {
    use jarak_gui::sessions::SessionFiles;
    let d = temp("find-history");
    let sf = SessionFiles {
        file: d.join("session.json"),
        recovery: d.join("recovery"),
        restore: true,
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

#[test]
fn the_gear_says_honestly_that_settings_come_later() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.get_by_label("Settings").click();
    h.run();
    assert!(h.state().settings_note);
    assert!(h.query_by_label_contains("Step 105").is_some());
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
    assert_eq!(
        h.state().tab().file.doc.markers().len(),
        2,
        "Alt+M: mark all"
    );
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
