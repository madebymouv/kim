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

#[test]
fn the_gear_says_honestly_that_settings_come_later() {
    let (a, _) = app_with(&[], Arc::default());
    let mut h = harness(a);
    h.get_by_label("Settings").click();
    h.run();
    assert!(h.state().settings_note);
    assert!(h.query_by_label_contains("Step 105").is_some());
}
