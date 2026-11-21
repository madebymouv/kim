//! Jarak's editor window (`jarak-gui.exe`, ADR 0102).
//!
//! - Step 85: the shell (status bar with the gear, no toolbar) and the single-instance hand-off.
//! - Step 88: the editor — the text view on `jarak-core`'s document ([`view`]), the standard keys
//!   ([`keys`]), Korean IME, open/save/save-as/close with the Windows dialogs, the unsaved-changes
//!   prompt on close and quit, the status bar's real values, zoom and word wrap. One document is
//!   shown at a time; Ctrl+Tab switches between open documents until Step 93 adds tabs.
//! - Step 89: the Markdown preview ([`preview`]) — Ctrl+E switches between the source and the
//!   rendered preview, Ctrl+Shift+E shows both side by side; the reading position carries over.

pub mod clipboard;
pub mod handoff;
pub mod keys;
pub mod preview;
pub mod view;

use eframe::egui::{self, Align, Layout, Rect, RichText, Vec2};
use jarak_core::command::Command;
use jarak_core::file::{self, FileDocument, SaveError, TextEncoding};
use jarak_core::target::OpenTarget;
use keys::Action;
use kim_design::{Lang, font, size, space};
use kim_ui::Colors;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use view::ViewState;

/// The window title without a document name.
pub const TITLE: &str = "Jarak";

/// Files handed over by later starts, filled by the hand-off thread.
pub type Incoming = Arc<Mutex<Vec<Vec<String>>>>;

/// The Open and Save As dialogs (the Windows ones in the program, scripted in tests).
pub trait Dialogs: Send + Sync {
    fn open(&self) -> Option<PathBuf>;
    fn save_as(&self, suggested: &str) -> Option<PathBuf>;
}

/// No dialogs (tests that never open one).
#[derive(Debug, Default)]
pub struct NoDialogs;

impl Dialogs for NoDialogs {
    fn open(&self) -> Option<PathBuf> {
        None
    }
    fn save_as(&self, _suggested: &str) -> Option<PathBuf> {
        None
    }
}

/// How a document is shown (`JARAK-FR-041`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Source,
    Preview,
    /// Source on the left, preview on the right (the preview follows the source).
    Split,
}

/// One open document and its view.
#[derive(Debug, Clone)]
pub struct Tab {
    pub file: FileDocument,
    pub view: ViewState,
    pub mode: Mode,
    pub preview: preview::PreviewState,
    /// Whether the text mixes line endings, for the history state it was counted at (counting
    /// a large file every frame would cost a frame).
    eol_cache: Option<(u64, bool)>,
    /// When the history state last changed (large documents recount only after a pause).
    changed_at: Option<(u64, f64)>,
}

impl Tab {
    fn new(file: FileDocument) -> Tab {
        Tab {
            file,
            view: ViewState::default(),
            mode: Mode::Source,
            preview: preview::PreviewState::default(),
            eol_cache: None,
            changed_at: None,
        }
    }

    /// The parse of the document (cached per history state).
    pub fn rendered(&mut self) -> Arc<jarak_core::markdown::Rendered> {
        let state = self.file.doc.history().state();
        let doc = &self.file.doc;
        self.preview.rendered(state, || doc.text().to_string())
    }

    /// Switches the mode, carrying the reading position: the source's top line to the block that
    /// shows it, and back (the caret never moves).
    pub fn set_mode(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        let r = self.rendered();
        match (self.mode, mode) {
            (Mode::Source, _) | (Mode::Split, Mode::Preview) => {
                self.preview.scroll_to_line(&r, self.view.top_line);
            }
            (Mode::Preview, _) => {
                if let Some(line) = self.preview.top_line(&r) {
                    self.view.top_line = line;
                    self.view.top_px = 0.0;
                }
            }
            (Mode::Split, _) => {}
        }
        self.view.preedit.clear();
        self.mode = mode;
    }

    /// The text mixes line endings: recounted after an edit — in a document above two million
    /// characters only once the typing paused for half a second (`ctx` is asked to come back
    /// then), so a keystroke never pays for a full scan.
    fn mixed_eol(&mut self, ctx: &egui::Context) -> bool {
        const LARGE: usize = 2_000_000;
        const PAUSE: f64 = 0.5;
        let state = self.file.doc.history().state();
        let now = ctx.input(|i| i.time);
        if self.changed_at.is_none_or(|(s, _)| s != state) {
            self.changed_at = Some((state, now));
        }
        match self.eol_cache {
            Some((s, mixed)) if s == state => mixed,
            Some((_, mixed))
                if self.file.doc.len_chars() > LARGE
                    && self.changed_at.is_some_and(|(_, t)| now - t < PAUSE) =>
            {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(PAUSE));
                mixed
            }
            _ => {
                let mut counts = file::EolCounts::default();
                let mut prev_cr = false;
                for chunk in self.file.doc.text().chunks() {
                    // A CRLF split across two chunks is one line ending.
                    let skip_lf = prev_cr && chunk.starts_with('\n');
                    let c = file::EolCounts::of(if skip_lf { &chunk[1..] } else { chunk });
                    if skip_lf {
                        counts.cr -= 1;
                        counts.crlf += 1;
                    }
                    counts.crlf += c.crlf;
                    counts.lf += c.lf;
                    counts.cr += c.cr;
                    prev_cr = chunk.ends_with('\r');
                }
                let mixed = counts.mixed();
                self.eol_cache = Some((state, mixed));
                mixed
            }
        }
    }

    /// A new document nobody typed into yet (replaced by the first file opened).
    fn is_blank(&self) -> bool {
        self.file.path.is_none() && self.file.doc.len_chars() == 0 && !self.file.doc.is_dirty()
    }

    /// The language shown in the status bar (detection proper is Step 98).
    pub fn language(&self) -> &'static str {
        match self
            .file
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("md" | "markdown" | "mdown" | "mkd") => "Markdown",
            _ => "Plain text",
        }
    }
}

/// What a pending unsaved-changes question is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    /// Close this document.
    Close(usize),
    /// Quit the window (asked for each dirty document in turn).
    Quit,
}

/// The window's state.
pub struct JarakApp {
    pub lang: Lang,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub incoming: Incoming,
    /// The note under the gear is open.
    pub settings_note: bool,
    /// The unsaved-changes question on screen.
    pub pending: Option<Pending>,
    /// A message in the status bar (an error, a saved file).
    pub message: Option<String>,
    /// The legacy encoding for files that are neither UTF-8 nor UTF-16.
    pub fallback: TextEncoding,
    dialogs: Arc<dyn Dialogs>,
    clipboard: Arc<dyn clipboard::Clipboard>,
    opener: Arc<dyn preview::Opener>,
    title: String,
    themed: bool,
    /// The window may close now (every question answered).
    quitting: bool,
    /// The UI audit's view (`KIM_UI_SCREEN`), named in the geometry report.
    audit_screen: String,
}

impl std::fmt::Debug for JarakApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JarakApp")
            .field("tabs", &self.tabs.len())
            .field("active", &self.active)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl JarakApp {
    pub fn new(lang: Lang, args: &[String], incoming: Incoming) -> JarakApp {
        JarakApp::with(
            lang,
            args,
            incoming,
            Arc::new(NoDialogs),
            TextEncoding::Utf8,
        )
    }

    /// Uses `c` for Copy, Cut and Paste (the program passes the Windows clipboard).
    pub fn with_clipboard(mut self, c: Arc<dyn clipboard::Clipboard>) -> JarakApp {
        self.clipboard = c;
        self
    }

    /// Uses `o` for links Ctrl+clicked in the preview (the program passes the Windows shell).
    pub fn with_opener(mut self, o: Arc<dyn preview::Opener>) -> JarakApp {
        self.opener = o;
        self
    }

    /// Opens a link the user Ctrl+clicked: web and mail links only (`JARAK-SEC-006`).
    pub fn open_link(&mut self, url: &str) {
        self.message = Some(if !preview::openable(url) {
            format!(
                "{} {url}",
                self.t(
                    "Jarak opens web and mail links only:",
                    "Jarak은 웹과 메일 링크만 엽니다:"
                )
            )
        } else if self.opener.open_url(url) {
            format!("{} {url}", self.t("Opened", "열었습니다:"))
        } else {
            format!("{} {url}", self.t("Could not open", "열지 못했습니다:"))
        });
    }

    pub fn with(
        lang: Lang,
        args: &[String],
        incoming: Incoming,
        dialogs: Arc<dyn Dialogs>,
        fallback: TextEncoding,
    ) -> JarakApp {
        let mut app = JarakApp {
            lang,
            tabs: vec![Tab::new(FileDocument::untitled())],
            active: 0,
            incoming,
            settings_note: false,
            pending: None,
            message: None,
            fallback,
            dialogs,
            clipboard: Arc::new(clipboard::MemoryClipboard::default()),
            opener: Arc::new(preview::NoOpener),
            title: String::new(),
            themed: false,
            quitting: false,
            audit_screen: "editor".into(),
        };
        app.open_args(args);
        app
    }

    /// Sets up a UI audit's view (ADR 0099): `editor`, `editor-empty`, `editor-selection`,
    /// `editor-wrap`, `question`, `preview`, `split`. Only used while `KIM_UI_INSPECT` is set.
    pub fn prepare_audit(&mut self, screen: &str) {
        self.audit_screen = screen.to_string();
        match screen {
            "editor-selection" => {
                let d = &mut self.tab_mut().file.doc;
                let a = d.text().line_to_char(2);
                let b = d.text().line_to_char(5.min(d.text().len_lines() - 1)) + 4;
                d.set_selection(&jarak_core::selection::Selection::single(
                    jarak_core::selection::Range::new(a, b.min(d.len_chars())),
                ));
            }
            "editor-wrap" => self.act(Action::ToggleWrap),
            "preview" => self.tab_mut().set_mode(Mode::Preview),
            "split" => self.tab_mut().set_mode(Mode::Split),
            "question" => {
                self.type_text("!");
                self.request_close(self.active);
            }
            _ => {}
        }
    }

    fn t<'a>(&self, en: &'a str, ko: &'a str) -> &'a str {
        self.lang.pick(en, ko)
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    /// Opens the files named by `path[:line[:column]]` arguments (an open file is shown again,
    /// a missing one becomes a new document created on the first save).
    pub fn open_args(&mut self, args: &[String]) {
        for a in args {
            if let Some(t) = OpenTarget::parse(a) {
                self.open_target(&t);
            }
        }
    }

    pub fn open_target(&mut self, t: &OpenTarget) {
        if let Some(i) = self
            .tabs
            .iter()
            .position(|tab| tab.file.path.as_deref() == Some(t.path.as_path()))
        {
            self.active = i;
        } else {
            match FileDocument::open(&t.path, self.fallback, file::DEFAULT_SIZE_LIMIT) {
                Ok(f) => {
                    let tab = Tab::new(f);
                    if self.tab().is_blank() && self.tabs.len() == 1 {
                        self.tabs[0] = tab;
                    } else {
                        self.tabs.push(tab);
                        self.active = self.tabs.len() - 1;
                    }
                    if let Some(r) = &self.tab().file.read_only_reason {
                        self.message = Some(format!(
                            "{} {r}",
                            self.t("Opened read-only:", "읽기 전용으로 열었습니다:")
                        ));
                    }
                }
                Err(e) => {
                    self.message = Some(format!("{}: {e}", t.path.display()));
                    return;
                }
            }
        }
        if let Some(line) = t.line {
            let doc = &mut self.tabs[self.active].file.doc;
            let l = (line as usize - 1).min(doc.text().len_lines().saturating_sub(1));
            let start = doc.text().line_to_char(l);
            let len = jarak_core::text::line_len(doc.text().line(l));
            let col = t.column.map_or(0, |c| c as usize - 1);
            let off =
                jarak_core::text::offset_at_column(doc.text().line(l), col, doc.tab_width).min(len);
            doc.set_selection(&jarak_core::selection::Selection::point(start + off));
            self.tabs[self.active].view.after_edit();
        }
    }

    /// The window title: `*name — Jarak` while it has unsaved changes.
    pub fn window_title(&self) -> String {
        let t = self.tab();
        let dirty = if t.file.doc.is_dirty() { "*" } else { "" };
        if t.file.path.is_none() && !t.file.doc.is_dirty() && self.tabs.len() == 1 {
            return TITLE.to_string();
        }
        format!("{dirty}{} — {TITLE}", t.file.title())
    }

    // ---- actions -----------------------------------------------------------------------------

    /// Runs a window action (a key, or a test).
    pub fn act(&mut self, a: Action) {
        let page = self.tab().view.page_lines;
        let read_only = self.tab().file.read_only;
        match a {
            Action::Cmd(c) => {
                let edits = matches!(
                    c,
                    Command::DeleteBackward
                        | Command::DeleteForward
                        | Command::DeleteWordBackward
                        | Command::DeleteWordForward
                        | Command::Newline
                        | Command::Undo
                        | Command::Redo
                );
                if edits && read_only {
                    self.refuse_read_only();
                    return;
                }
                let t = self.tab_mut();
                t.file.doc.execute(c, page);
                t.view.preedit.clear();
                t.view.after_edit();
            }
            Action::Indent | Action::Outdent => {
                if read_only {
                    self.refuse_read_only();
                    return;
                }
                let t = self.tab_mut();
                if a == Action::Indent {
                    t.file.doc.indent();
                } else {
                    t.file.doc.outdent();
                }
                t.view.after_edit();
            }
            Action::ToggleOverwrite => {
                let v = &mut self.tab_mut().view;
                v.overwrite = !v.overwrite;
            }
            Action::ZoomIn => self.zoom(0.1),
            Action::ZoomOut => self.zoom(-0.1),
            Action::ZoomReset => self.tab_mut().view.zoom = 1.0,
            Action::ToggleWrap => {
                let v = &mut self.tab_mut().view;
                v.wrap = !v.wrap;
                v.scroll_x = 0.0;
                v.after_edit();
            }
            Action::New => {
                self.tabs.push(Tab::new(FileDocument::untitled()));
                self.active = self.tabs.len() - 1;
            }
            Action::Open => {
                if let Some(p) = self.dialogs.open() {
                    self.open_target(&OpenTarget {
                        path: p,
                        line: None,
                        column: None,
                    });
                }
            }
            Action::Save => {
                self.save(self.active);
            }
            Action::SaveAs => {
                self.save_as(self.active);
            }
            Action::Close => self.request_close(self.active),
            Action::TogglePreview => {
                let t = self.tab_mut();
                let to = if t.mode == Mode::Source {
                    Mode::Preview
                } else {
                    Mode::Source
                };
                t.set_mode(to);
            }
            Action::ToggleSplit => {
                let t = self.tab_mut();
                let to = if t.mode == Mode::Split {
                    Mode::Source
                } else {
                    Mode::Split
                };
                t.set_mode(to);
            }
            Action::Copy => {
                let s = self.copy_text();
                self.clipboard.set_text(&s);
            }
            Action::Cut => {
                let s = self.cut();
                self.clipboard.set_text(&s);
            }
            Action::Paste => {
                if let Some(s) = self.clipboard.get_text() {
                    self.paste(&s);
                }
            }
            Action::NextDocument => {
                self.active = (self.active + 1) % self.tabs.len();
            }
            Action::PreviousDocument => {
                self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
            }
        }
    }

    fn zoom(&mut self, d: f32) {
        let v = &mut self.tab_mut().view;
        v.zoom = ((v.zoom + d) * 10.0).round().clamp(5.0, 40.0) / 10.0;
    }

    fn refuse_read_only(&mut self) {
        let why = self
            .tab()
            .file
            .read_only_reason
            .clone()
            .unwrap_or_else(|| self.t("read-only", "읽기 전용").to_string());
        self.message = Some(format!(
            "{} ({why})",
            self.t("This document is read-only", "읽기 전용 문서입니다")
        ));
    }

    /// Types text at the carets (typing, a commit from the IME).
    pub fn type_text(&mut self, s: &str) {
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let t = self.tab_mut();
        if t.view.overwrite {
            t.file.doc.overwrite_text(s);
        } else {
            t.file.doc.type_text(s);
        }
        t.view.after_edit();
    }

    /// Pastes text as one undo step, its line endings changed to the document's.
    pub fn paste(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        let t = self.tab_mut();
        let s = clipboard::with_line_endings(s, t.file.doc.eol.as_str());
        t.file.doc.paste(&s);
        t.view.after_edit();
    }

    /// The text Copy puts on the clipboard (the current line when nothing is selected, as
    /// Notepad++ does with its setting on).
    pub fn copy_text(&self) -> String {
        let d = &self.tab().file.doc;
        let s = d.selected_text();
        if s.is_empty() {
            let line = d.text().char_to_line(d.selection().primary().head);
            d.text().line(line).to_string()
        } else {
            s
        }
    }

    /// Cut: copies and removes the selection (or the current line).
    pub fn cut(&mut self) -> String {
        let s = self.copy_text();
        if self.tab().file.read_only {
            self.refuse_read_only();
            return s;
        }
        let t = self.tab_mut();
        if t.file.doc.selected_text().is_empty() {
            let pos = t.file.doc.selection().primary().head;
            t.file.doc.select_line_at(pos);
        }
        t.file.doc.delete_backward();
        t.view.after_edit();
        s
    }

    /// Saves document `i` (Save As when it has no file). `true` when it is saved.
    pub fn save(&mut self, i: usize) -> bool {
        if self.tabs[i].file.path.is_none() {
            return self.save_as(i);
        }
        let r = self.tabs[i].file.save();
        self.report_save(i, r)
    }

    /// Asks for a file name and saves document `i` there.
    pub fn save_as(&mut self, i: usize) -> bool {
        let suggested = self.tabs[i].file.title();
        let Some(p) = self.dialogs.save_as(&suggested) else {
            return false;
        };
        let r = self.tabs[i].file.save_as(&p);
        self.report_save(i, r)
    }

    fn report_save(&mut self, i: usize, r: Result<(), SaveError>) -> bool {
        match r {
            Ok(()) => {
                self.message = Some(format!(
                    "{} {}",
                    self.t("Saved", "저장했습니다:"),
                    self.tabs[i].file.title()
                ));
                true
            }
            Err(e) => {
                self.message = Some(format!(
                    "{} {}: {e}",
                    self.t("Not saved:", "저장하지 못했습니다:"),
                    self.tabs[i].file.title()
                ));
                false
            }
        }
    }

    /// Close document `i`, asking first when it has unsaved changes.
    pub fn request_close(&mut self, i: usize) {
        if self.tabs[i].file.doc.is_dirty() {
            self.active = i;
            self.pending = Some(Pending::Close(i));
        } else {
            self.close_now(i);
        }
    }

    fn close_now(&mut self, i: usize) {
        self.tabs.remove(i);
        if self.tabs.is_empty() {
            self.tabs.push(Tab::new(FileDocument::untitled()));
        }
        self.active = self.active.min(self.tabs.len() - 1);
    }

    /// The window was asked to close: `true` when it may (nothing unsaved, or every question
    /// answered); otherwise the first unsaved document is shown with the question.
    pub fn request_quit(&mut self) -> bool {
        if self.quitting {
            return true;
        }
        if let Some(i) = self.tabs.iter().position(|t| t.file.doc.is_dirty()) {
            self.active = i;
            self.pending = Some(Pending::Quit);
            false
        } else {
            true
        }
    }

    /// The answer to the unsaved-changes question: `Some(true)` save, `Some(false)` don't save,
    /// `None` cancel.
    pub fn answer(&mut self, save: Option<bool>) {
        let Some(p) = self.pending.take() else {
            return;
        };
        let Some(save) = save else {
            return;
        };
        let i = self.active;
        if save && !self.save(i) {
            // Not saved (cancelled dialog, an error): nothing is closed.
            return;
        }
        match p {
            Pending::Close(i) => self.close_now(i),
            Pending::Quit => {
                if !save {
                    // Don't save: forget this document's changes for the quit.
                    self.close_now(i);
                }
                if self.tabs.iter().any(|t| t.file.doc.is_dirty()) {
                    self.request_quit();
                } else {
                    self.quitting = true;
                }
            }
        }
    }

    /// The window may close now.
    pub fn may_quit(&self) -> bool {
        self.quitting
    }

    // ---- frame -------------------------------------------------------------------------------

    /// One frame: input, the text view, the status bar, the question.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.themed {
            kim_ui::apply(&ctx, kim_ui::Options::default());
            self.themed = true;
        }
        let pending: Vec<Vec<String>> = self
            .incoming
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        for args in pending {
            self.open_args(&args);
        }
        // Dropped files open.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        for p in dropped {
            self.open_target(&OpenTarget {
                path: p,
                line: None,
                column: None,
            });
        }
        // Closing the window asks about unsaved changes first.
        // (A UI audit's run closes itself without asking: it holds no user data.)
        if ctx.input(|i| i.viewport().close_requested())
            && !kim_ui::inspect::active()
            && !self.request_quit()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let title = self.window_title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let c = Colors::get(&ctx);
        self.status_bar(ui, &c);
        let (focused, link) = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(c.bg_base))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let read_only = self.tab().file.read_only;
                let t = &mut self.tabs[self.active];
                match t.mode {
                    Mode::Source => (
                        view::show(ui, rect, &mut t.file.doc, &mut t.view, read_only).focused,
                        preview::PreviewOutput::default(),
                    ),
                    Mode::Preview => {
                        let r = t.rendered();
                        let out = preview::show(ui, rect, &r, &mut t.preview, t.view.zoom);
                        (false, out)
                    }
                    Mode::Split => {
                        let mid = rect.center().x.round();
                        let left = Rect::from_min_max(rect.min, egui::pos2(mid, rect.bottom()));
                        let right = Rect::from_min_max(egui::pos2(mid + 1.0, rect.top()), rect.max);
                        let focused =
                            view::show(ui, left, &mut t.file.doc, &mut t.view, read_only).focused;
                        // The preview follows the source's top line.
                        let r = t.rendered();
                        t.preview.scroll_to_line(&r, t.view.top_line);
                        let out = preview::show(ui, right, &r, &mut t.preview, t.view.zoom);
                        ui.painter().vline(
                            mid + 0.5,
                            rect.y_range(),
                            egui::Stroke::new(size::BORDER, c.border_subtle),
                        );
                        (focused, out)
                    }
                }
            })
            .inner;
        if let Some(url) = link.open {
            self.open_link(&url);
        } else if let Some(url) = link.plain_click {
            self.message = Some(format!(
                "{} {url}",
                self.t("Ctrl+click to open", "Ctrl+클릭으로 엽니다:")
            ));
        }
        if self.pending.is_none() {
            let preview_only = self.tab().mode == Mode::Preview;
            if !preview_only && !focused && !ctx.memory(|m| m.focused().is_some()) {
                ctx.memory_mut(|m| m.request_focus(view::view_id()));
            }
            if focused || preview_only {
                self.keyboard(&ctx);
            }
        }
        self.question(&ctx, &c);
        if kim_ui::inspect::active() {
            kim_ui::inspect::finish_frame(&ctx, "jarak", &self.audit_screen);
        }
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        if self.tab().mode == Mode::Preview {
            self.preview_keys(&events);
            return;
        }
        for e in events {
            match e {
                egui::Event::Text(s) => {
                    if !s.chars().all(char::is_control) {
                        self.type_text(&s);
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    let v = &mut self.tab_mut().view;
                    v.preedit = text;
                    v.after_edit();
                }
                egui::Event::Ime(egui::ImeEvent::Commit(s)) => {
                    self.tab_mut().view.preedit.clear();
                    if !s.is_empty() {
                        self.type_text(&s);
                    }
                }
                egui::Event::Copy => self.act(Action::Copy),
                egui::Event::Cut => self.act(Action::Cut),
                egui::Event::Paste(s) => self.paste(&s),
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if let Some(a) = keys::action_for(key, modifiers) {
                        self.act(a);
                    }
                }
                egui::Event::MouseWheel {
                    delta, modifiers, ..
                } if modifiers.command => {
                    if delta.y > 0.0 {
                        self.act(Action::ZoomIn);
                    } else if delta.y < 0.0 {
                        self.act(Action::ZoomOut);
                    }
                }
                _ => {}
            }
        }
    }

    /// Keys while only the preview shows: scrolling, the mode, files; typing explains how to edit.
    fn preview_keys(&mut self, events: &[egui::Event]) {
        for e in events {
            match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let blocks = self.tabs[self.active].rendered().blocks.len();
                    let p = &mut self.tab_mut().preview;
                    let step = |p: &mut preview::PreviewState, d: isize| {
                        p.top_block = (p.top_block as isize + d)
                            .clamp(0, blocks.saturating_sub(1) as isize)
                            as usize;
                        p.top_px = 0.0;
                    };
                    match (key, modifiers.command) {
                        (egui::Key::ArrowDown, false) => step(p, 1),
                        (egui::Key::ArrowUp, false) => step(p, -1),
                        (egui::Key::PageDown, false) | (egui::Key::Space, false) => step(p, 5),
                        (egui::Key::PageUp, false) => step(p, -5),
                        (egui::Key::Home, _) => step(p, -(blocks as isize)),
                        (egui::Key::End, _) => step(p, blocks as isize),
                        (egui::Key::Escape, _) => self.tab_mut().set_mode(Mode::Source),
                        _ => {
                            if let Some(a) = keys::action_for(*key, *modifiers)
                                && matches!(
                                    a,
                                    Action::TogglePreview
                                        | Action::ToggleSplit
                                        | Action::Save
                                        | Action::SaveAs
                                        | Action::Open
                                        | Action::New
                                        | Action::Close
                                        | Action::Copy
                                        | Action::ZoomIn
                                        | Action::ZoomOut
                                        | Action::ZoomReset
                                        | Action::NextDocument
                                        | Action::PreviousDocument
                                )
                            {
                                self.act(a);
                            }
                        }
                    }
                }
                egui::Event::Text(_) | egui::Event::Paste(_) => {
                    self.message = Some(
                        self.t(
                            "The preview cannot be edited: Ctrl+E or Esc returns to the source",
                            "미리보기는 편집할 수 없습니다. Ctrl+E나 Esc로 원본으로 돌아갑니다",
                        )
                        .to_string(),
                    );
                }
                _ => {}
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let ctx = ui.ctx().clone();
        let mixed = self.tabs[self.active].mixed_eol(&ctx);
        let t = self.tab();
        let d = &t.file.doc;
        let head = d.selection().primary().head;
        let (line, col) = d.line_col(head);
        let sel: usize = d.selection().ranges().iter().map(|r| r.len()).sum();
        let pos = if sel > 0 {
            format!(
                "{} {}, {} {} ({} {sel})",
                self.t("Ln", "줄"),
                line + 1,
                self.t("Col", "열"),
                col + 1,
                self.t("selected", "선택")
            )
        } else {
            format!(
                "{} {}, {} {}",
                self.t("Ln", "줄"),
                line + 1,
                self.t("Col", "열"),
                col + 1
            )
        };
        let eol_label = if mixed {
            self.t("Mixed EOL", "줄 끝 혼합").to_string()
        } else {
            match d.eol {
                jarak_core::document::Eol::CrLf => "CRLF".to_string(),
                jarak_core::document::Eol::Lf => "LF".to_string(),
                jarak_core::document::Eol::Cr => "CR".to_string(),
            }
        };
        let mut items: Vec<(String, egui::Color32)> = vec![
            (pos, c.text_secondary),
            (
                t.file.format.encoding.label(t.file.format.bom),
                c.text_secondary,
            ),
            (eol_label, c.text_secondary),
            (
                self.t(
                    t.language(),
                    if t.language() == "Markdown" {
                        "Markdown"
                    } else {
                        "일반 텍스트"
                    },
                )
                .to_string(),
                c.text_secondary,
            ),
            (
                if t.view.overwrite { "OVR" } else { "INS" }.to_string(),
                c.text_muted,
            ),
        ];
        if (t.view.zoom - 1.0).abs() > 0.01 {
            items.push((format!("{:.0}%", t.view.zoom * 100.0), c.text_muted));
        }
        if t.view.wrap {
            items.push((self.t("Wrap", "줄 바꿈").to_string(), c.text_muted));
        }
        match t.mode {
            Mode::Preview => items.push((self.t("Preview", "미리보기").to_string(), c.accent)),
            Mode::Split => items.push((self.t("Split", "나란히 보기").to_string(), c.accent)),
            Mode::Source => {}
        }
        if t.file.read_only {
            items.push((self.t("Read-only", "읽기 전용").to_string(), c.warning));
        }
        if self.tabs.len() > 1 {
            items.push((
                format!("{}/{}", self.active + 1, self.tabs.len()),
                c.text_muted,
            ));
        }
        let message = self.message.clone();
        egui::Panel::bottom("jarak-status")
            .exact_size(size::CONTROL)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::S as i8, 0)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                let bar = ui.max_rect();
                ui.painter().hline(
                    bar.x_range().expand(space::S),
                    bar.top(),
                    egui::Stroke::new(size::BORDER, c.border_subtle),
                );
                // The design system's gear (DESIGN_SYSTEM §3.10): `size::GEAR`, the bar's height.
                let gear = Rect::from_min_size(
                    egui::pos2(bar.right() - size::GEAR, bar.center().y - size::GEAR / 2.0),
                    Vec2::splat(size::GEAR),
                );
                let label = self.t("Settings", "설정").to_string();
                if kim_ui::gear_at(ui, gear, &label, self.settings_note).clicked() {
                    self.settings_note = !self.settings_note;
                }
                let text_area =
                    Rect::from_min_max(bar.min, egui::pos2(gear.left() - space::S, bar.max.y));
                ui.scope_builder(egui::UiBuilder::new().max_rect(text_area), |ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = space::L;
                        if let Some(m) = &message {
                            ui.label(RichText::new(m).size(font::CAPTION).color(c.text_primary));
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = space::L;
                            for (item, col) in items.iter().rev() {
                                ui.label(RichText::new(item).size(font::CAPTION).color(*col));
                            }
                        });
                    });
                });
                if self.settings_note {
                    let note = self.t(
                        "Settings open in a window of their own once Step 105 (#124) is done; nothing to set yet.",
                        "설정은 Step 105(#124)가 끝나면 별도 창으로 열립니다. 아직 바꿀 설정이 없습니다.",
                    );
                    egui::Area::new(egui::Id::unique("jarak-settings-note"))
                        .order(egui::Order::Foreground)
                        .pivot(egui::Align2::RIGHT_BOTTOM)
                        .fixed_pos(egui::pos2(bar.right(), bar.top() - space::XS))
                        .show(&ctx, |ui| {
                            kim_ui::popover_frame(&ctx).show(ui, |ui| {
                                ui.set_max_width(size::POPOVER_WIDTH);
                                ui.label(RichText::new(note).size(font::BODY).color(c.text_primary));
                            });
                        });
                }
            });
    }

    /// The unsaved-changes question (kim-ui dialog).
    fn question(&mut self, ctx: &egui::Context, c: &Colors) {
        if self.pending.is_none() {
            return;
        }
        let name = self.tab().file.title();
        let title = format!(
            "{} {name}?",
            self.t("Save changes to", "변경 내용을 저장할까요:")
        );
        let body = self.t(
            "Your changes are lost if you don't save them.",
            "저장하지 않으면 변경 내용이 사라집니다.",
        );
        let (save, dont, cancel) = (
            self.t("Save", "저장").to_string(),
            self.t("Don't save", "저장 안 함").to_string(),
            self.t("Cancel", "취소").to_string(),
        );
        let mut choice: Option<Option<bool>> = None;
        egui::Modal::new(egui::Id::unique("jarak-unsaved")).show(ctx, |ui| {
            ui.set_width(size::DIALOG_WIDTH);
            ui.label(
                RichText::new(&title)
                    .size(font::TITLE)
                    .color(c.text_primary),
            );
            ui.add_space(space::S);
            ui.label(RichText::new(body).size(font::BODY).color(c.text_secondary));
            ui.add_space(space::L);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(kim_ui::Button::primary(&save)).clicked() {
                    choice = Some(Some(true));
                }
                if ui.add(kim_ui::Button::secondary(&dont)).clicked() {
                    choice = Some(Some(false));
                }
                if ui.add(kim_ui::Button::secondary(&cancel)).clicked() {
                    choice = Some(None);
                }
            });
            // The keyboard: Enter is the default button (Save), Esc cancels.
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                choice = Some(None);
            } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) && choice.is_none() {
                choice = Some(Some(true));
            }
        });
        if let Some(c) = choice {
            self.answer(c);
        }
    }
}

impl eframe::App for JarakApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        JarakApp::ui(self, ui);
    }
}

/// The real Windows dialogs (`GetOpenFileNameW` / `GetSaveFileNameW`).
#[derive(Debug, Default)]
pub struct WinDialogs;

impl Dialogs for WinDialogs {
    fn open(&self) -> Option<PathBuf> {
        win_dialog(true, "").map(PathBuf::from)
    }
    fn save_as(&self, suggested: &str) -> Option<PathBuf> {
        win_dialog(false, suggested).map(PathBuf::from)
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn win_dialog(open: bool, suggested: &str) -> Option<String> {
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, GetSaveFileNameW, OFN_FILEMUSTEXIST, OFN_OVERWRITEPROMPT,
        OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };
    let filter: Vec<u16> =
        "All files (*.*)\0*.*\0Text (*.txt)\0*.txt\0Markdown (*.md)\0*.md;*.markdown\0\0"
            .encode_utf16()
            .collect();
    let mut buf = vec![0u16; 4096];
    for (i, u) in suggested.encode_utf16().take(buf.len() - 1).enumerate() {
        buf[i] = u;
    }
    // SAFETY: OPENFILENAMEW is plain data; all-zero is its documented empty state.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.Flags = OFN_PATHMUSTEXIST
        | if open {
            OFN_FILEMUSTEXIST
        } else {
            OFN_OVERWRITEPROMPT
        };
    // SAFETY: `ofn` points to buffers that live across the call; the dialog is modal.
    let ok = unsafe {
        if open {
            GetOpenFileNameW(&raw mut ofn)
        } else {
            GetSaveFileNameW(&raw mut ofn)
        }
    };
    if ok == 0 {
        return None;
    }
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}
