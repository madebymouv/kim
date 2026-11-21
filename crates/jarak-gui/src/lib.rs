//! Jarak's editor window (`jarak-gui.exe`, ADR 0102).
//!
//! - Step 85: the shell (status bar with the gear, no toolbar) and the single-instance hand-off.
//! - Step 88: the editor — the text view on `jarak-core`'s document ([`view`]), the standard keys
//!   ([`keys`]), Korean IME, open/save/save-as/close with the Windows dialogs, the unsaved-changes
//!   prompt on close and quit, the status bar's real values, zoom and word wrap. One document is
//!   shown at a time; Ctrl+Tab switches between open documents until Step 93 adds tabs.
//! - Step 89: the Markdown preview ([`preview`]) — Ctrl+E switches between the source and the
//!   rendered preview, Ctrl+Shift+E shows both side by side; the reading position carries over.
//! - Step 90: Ctrl+V of an image in a Markdown document saves it next to the document
//!   (`assets/<name>-<date>-<n>.png`) and inserts a relative reference ([`images`]); the preview
//!   shows local images.

pub mod clipboard;
pub mod handoff;
pub mod images;
pub mod keys;
pub mod media;
pub mod palette;
pub mod preview;
pub mod view;

use eframe::egui::{self, Align, Layout, Rect, RichText, Vec2};
use jarak_core::command::Command;
use jarak_core::file::{self, FileDocument, SaveError, TextEncoding};
use jarak_core::target::OpenTarget;
use keys::Action;
use kim_design::{Lang, font, size, space};
use kim_ui::Colors;
use std::path::{Path, PathBuf};
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
        if self
            .file
            .path
            .as_deref()
            .is_some_and(jarak_core::markdown::is_markdown_path)
        {
            "Markdown"
        } else {
            "Plain text"
        }
    }
}

/// The editing mode (`JARAK-FR-050`): Standard (the default) or Vim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editing {
    Standard,
    Vim,
}

/// The `+` register is the window's clipboard.
struct ClipHost<'a>(&'a dyn clipboard::Clipboard);

impl jarak_core::vim::Host for ClipHost<'_> {
    fn clipboard(&self) -> Option<String> {
        self.0.get_text()
    }
    fn set_clipboard(&self, s: &str) {
        self.0.set_text(s);
    }
}

/// The key Vim gets for `key` with `m` held (`None`: the window's own key).
fn vim_key(key: egui::Key, m: egui::Modifiers) -> Option<jarak_core::vim::Key> {
    use egui::Key as K;
    use jarak_core::vim::Key as V;
    let ctrl = m.ctrl || m.command;
    if m.alt {
        return None;
    }
    if ctrl {
        if m.shift {
            return None;
        }
        return Some(V::Ctrl(match key {
            K::R => 'r',
            K::V => 'v',
            K::D => 'd',
            K::U => 'u',
            K::F => 'f',
            K::B => 'b',
            K::W => 'w',
            K::OpenBracket => '[',
            _ => return None,
        }));
    }
    Some(match key {
        K::Escape => V::Esc,
        K::Enter => V::Enter,
        K::Backspace => V::Backspace,
        K::Delete => V::Delete,
        K::Tab if !m.shift => V::Tab,
        K::ArrowLeft if !m.shift => V::Left,
        K::ArrowRight if !m.shift => V::Right,
        K::ArrowUp if !m.shift => V::Up,
        K::ArrowDown if !m.shift => V::Down,
        K::Home if !m.shift => V::Home,
        K::End if !m.shift => V::End,
        K::PageUp if !m.shift => V::PageUp,
        K::PageDown if !m.shift => V::PageDown,
        _ => return None,
    })
}

/// The editing mode in the settings file (`editing_mode = "vim"`; Step 105 brings the whole
/// settings model, this is its first key).
pub fn read_editing(path: &Path) -> Option<Editing> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "editing_mode").then(|| match v.trim().trim_matches('"') {
            "vim" => Editing::Vim,
            _ => Editing::Standard,
        })
    })
}

/// Writes the editing mode into the settings file, keeping its other lines.
pub fn write_editing(path: &Path, e: Editing) -> std::io::Result<()> {
    let value = format!(
        "editing_mode = \"{}\"",
        if e == Editing::Vim { "vim" } else { "standard" }
    );
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut found = false;
    let mut lines: Vec<String> = old
        .lines()
        .map(|l| {
            if l.split_once('=')
                .is_some_and(|(k, _)| k.trim() == "editing_mode")
            {
                found = true;
                value.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(value);
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    file::save_atomic(path, format!("{}\n", lines.join("\n")).as_bytes())
}

/// What a pending question is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Close this document (unsaved changes).
    Close(usize),
    /// Quit the window (asked for each dirty document in turn).
    Quit,
    /// Step 91 (`JARAK-FR-045`): after Save As to another folder, copy the images the document
    /// refers to (source, destination) so its references keep working.
    CopyAssets(Vec<(PathBuf, PathBuf)>),
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
    /// Step 92: Standard or Vim editing, and Vim's state (for the active document).
    pub editing: Editing,
    pub vim: jarak_core::vim::Vim,
    /// The document Vim's state belongs to.
    vim_tab: usize,
    /// Where the editing mode is kept (`None` in tests: nothing is written).
    settings: Option<PathBuf>,
    /// Step 92: the command palette while it is open, the menu bar, Alt pressed alone.
    pub palette: Option<palette::PaletteState>,
    pub menu_bar: bool,
    alt_armed: bool,
    /// Step 92: the document's images and the folder's orphans (`markdown.assets`).
    pub assets_view: Option<AssetsView>,
}

/// The assets dialog's content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetsView {
    /// Each image reference: as written, its local file (when it has one) and what it is.
    pub refs: Vec<(String, Option<PathBuf>, String)>,
    pub orphans: Vec<PathBuf>,
    /// A rename being typed: the reference and the new name.
    pub rename: Option<(String, String)>,
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

    /// Reads and keeps the editing mode in `path` (the program passes the settings file).
    pub fn with_settings(mut self, path: PathBuf) -> JarakApp {
        if let Some(e) = read_editing(&path) {
            self.editing = e;
        }
        self.settings = Some(path);
        self
    }

    /// Switches between Standard and Vim editing (and remembers it).
    pub fn set_editing(&mut self, e: Editing) {
        self.editing = e;
        let i = self.active;
        self.vim.reset(&mut self.tabs[i].file.doc);
        self.vim_tab = i;
        self.message = Some(match e {
            Editing::Vim => self
                .t(
                    "Vim editing: Esc for Normal mode, i to type; Ctrl+Shift+P → Standard editing to switch back.",
                    "Vim 편집: Esc는 Normal 모드, i는 입력. 되돌리려면 Ctrl+Shift+P → 표준 편집.",
                )
                .to_string(),
            Editing::Standard => self.t("Standard editing.", "표준 편집.").to_string(),
        });
        if let Some(p) = &self.settings
            && let Err(e) = write_editing(p, e)
        {
            self.message = Some(format!(
                "{} {e}",
                self.t(
                    "The editing mode was not saved:",
                    "편집 모드를 저장하지 못했습니다:"
                )
            ));
        }
    }

    /// Vim's view of the window: the lines on screen.
    fn vim_view(&self) -> jarak_core::vim::View {
        let v = &self.tab().view;
        jarak_core::vim::View {
            top: v.top_line,
            rows: v.page_lines.max(1),
        }
    }

    /// Runs one Vim key or text and carries out what it asks of the window. `false` when the
    /// key was not Vim's.
    fn vim_input(&mut self, input: Result<jarak_core::vim::Key, String>) -> bool {
        let view = self.vim_view();
        let i = self.active;
        let read_only = self.tabs[i].file.read_only;
        let before = self.tabs[i].file.doc.history().state();
        let clip = self.clipboard.clone();
        let host = ClipHost(clip.as_ref());
        let doc = &mut self.tabs[i].file.doc;
        let out = match input {
            Ok(k) => self.vim.key(doc, k, &host, view),
            Err(s) => self.vim.text(doc, &s, &host, view),
        };
        if read_only && self.tabs[i].file.doc.history().state() != before {
            // A read-only document never changes: the edit is taken back.
            self.tabs[i].file.doc.undo();
            self.vim.reset(&mut self.tabs[i].file.doc);
            self.refuse_read_only();
            return true;
        }
        if read_only && self.vim.mode.is_insert() {
            self.vim.reset(&mut self.tabs[i].file.doc);
            self.refuse_read_only();
            return true;
        }
        if !out.handled {
            return false;
        }
        self.tabs[i].view.after_edit();
        if let Some(m) = out.message {
            self.message = Some(m);
        }
        for e in out.ex {
            self.vim_ex(e);
        }
        true
    }

    /// A command line's request (`:w`, `:q`, `:e`, `:set wrap`).
    fn vim_ex(&mut self, e: jarak_core::vim::Ex) {
        use jarak_core::vim::Ex;
        let i = self.active;
        let dirty = self.tabs[i].file.doc.is_dirty();
        let not_saved = || "E37: No write since last change (add ! to override)".to_string();
        let resolve = |s: &str, doc: Option<&Path>| -> PathBuf {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                p
            } else {
                doc.and_then(Path::parent).map_or_else(
                    || std::path::absolute(&p).unwrap_or(p.clone()),
                    |d| d.join(&p),
                )
            }
        };
        match e {
            Ex::Write { path: None, .. } => {
                self.save(i);
            }
            Ex::Write { path: Some(s), .. } => {
                let target = resolve(&s, self.tabs[i].file.path.as_deref());
                if self.tabs[i].file.path.is_none() {
                    let r = self.tabs[i].file.save_as(&target);
                    self.report_save(i, r);
                } else {
                    let r = self.tabs[i].file.save_copy(&target);
                    self.message = Some(match r {
                        Ok(()) => format!("{} {}", self.t("Written:", "저장함:"), target.display()),
                        Err(e) => format!("{} {e}", self.t("Not written:", "저장하지 못함:")),
                    });
                }
            }
            Ex::Quit { force } => {
                if dirty && !force {
                    self.message = Some(not_saved());
                } else {
                    self.close_or_quit(i);
                }
            }
            Ex::WriteQuit { .. } => {
                if (!dirty && self.tabs[i].file.path.is_some()) || self.save(i) {
                    self.close_or_quit(i);
                }
            }
            Ex::Edit { path: None, force } => {
                if dirty && !force {
                    self.message = Some(not_saved());
                } else if self.tabs[i].file.path.is_some() {
                    let fallback = self.fallback;
                    let r = self.tabs[i].file.reload(fallback, file::DEFAULT_SIZE_LIMIT);
                    self.message = Some(match r {
                        Ok(()) => format!(
                            "{} {}",
                            self.t("Reloaded", "다시 읽음:"),
                            self.tabs[i].file.title()
                        ),
                        Err(e) => format!("{} {e}", self.t("Not reloaded:", "다시 읽지 못함:")),
                    });
                    self.vim.reset(&mut self.tabs[i].file.doc);
                }
            }
            Ex::Edit { path: Some(s), .. } => {
                let target = resolve(&s, self.tabs[i].file.path.as_deref());
                self.open_target(&OpenTarget {
                    path: target,
                    line: None,
                    column: None,
                });
            }
            Ex::Wrap(on) => {
                let v = &mut self.tabs[i].view;
                v.wrap = on;
                v.scroll_x = 0.0;
                v.after_edit();
            }
        }
    }

    /// Before drawing: Vim's state follows the active document, and the view shows Vim's caret,
    /// IME switch and search marks.
    fn vim_frame(&mut self) {
        let i = self.active;
        if self.editing != Editing::Vim {
            let v = &mut self.tabs[i].view;
            v.block_caret = false;
            v.ime = true;
            v.marks.clear();
            v.keep_escape = false;
            return;
        }
        if self.vim_tab != i || self.vim_tab >= self.tabs.len() {
            self.vim.reset(&mut self.tabs[i].file.doc);
            self.vim_tab = i;
        }
        let mode = self.vim.mode;
        let prompt = self.vim.prompt.is_some();
        let marks = self.vim.highlight_regex().map(|re| {
            let t = &self.tabs[i];
            let r = t.file.doc.text();
            let first = t.view.top_line.min(r.len_lines().saturating_sub(1));
            let last = (first + t.view.page_lines + 2).min(r.len_lines());
            let a = r.line_to_char(first);
            let b = if last < r.len_lines() {
                r.line_to_char(last)
            } else {
                r.len_chars()
            };
            let s = r.slice(a..b).to_string();
            re.find_iter(&s)
                .filter(|m| m.end() > m.start())
                .map(|m| {
                    (
                        a + s[..m.start()].chars().count(),
                        a + s[..m.end()].chars().count(),
                    )
                })
                .collect::<Vec<_>>()
        });
        let v = &mut self.tabs[i].view;
        v.block_caret = !mode.is_insert() || prompt;
        v.ime = mode.is_insert() && !prompt;
        v.overwrite = false;
        v.keep_escape = true;
        v.marks = marks.unwrap_or_default();
    }

    /// Runs a palette or menu entry.
    pub fn run_entry(&mut self, r: palette::Run) {
        match r {
            palette::Run::Act(a) => self.act(a),
            palette::Run::Editing(e) => self.set_editing(e),
            palette::Run::MenuBar => self.menu_bar = !self.menu_bar,
            palette::Run::Assets => self.open_assets(),
        }
    }

    /// Opens the assets dialog for the document (a saved Markdown document).
    pub fn open_assets(&mut self) {
        let t = self.tab();
        let Some(path) = t.file.path.clone().filter(|_| t.language() == "Markdown") else {
            self.message = Some(
                self.t(
                    "Images and assets belong to a saved Markdown document.",
                    "이미지와 asset은 저장된 Markdown 문서에 있습니다.",
                )
                .to_string(),
            );
            return;
        };
        let text = t.file.doc.text().to_string();
        let mut refs = Vec::new();
        for r in jarak_core::assets::references(&text) {
            let lower = r.url.trim().to_ascii_lowercase();
            let local = jarak_core::assets::resolve(&path, &r.url);
            let what = if lower.contains("://") || lower.starts_with("data:") {
                self.t("remote, never loaded", "원격, 불러오지 않음")
                    .to_string()
            } else if let Some(p) = local.as_deref().filter(|p| p.is_file()) {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                let fmt = jarak_core::assets::image_format(&jarak_core::assets::file_head(p))
                    .unwrap_or("?");
                format!("{fmt}, {} KB", size.div_ceil(1024))
            } else if local.is_some() {
                self.t("missing", "없음").to_string()
            } else {
                self.t("outside the document's folder", "문서 폴더 밖")
                    .to_string()
            };
            refs.push((r.url, local, what));
        }
        // The open document's references count as they are now (saved or not).
        let mine: Vec<String> = refs
            .iter()
            .filter_map(|(_, p, _)| p.as_ref())
            .map(|p| p.to_string_lossy().to_lowercase())
            .collect();
        let orphans = jarak_core::assets::orphans(&path)
            .into_iter()
            .filter(|o| !mine.contains(&o.to_string_lossy().to_lowercase()))
            .collect();
        self.assets_view = Some(AssetsView {
            refs,
            orphans,
            rename: None,
        });
    }

    /// Renames an asset of the open document (saved first) and its references, then saves.
    pub fn rename_asset(&mut self, reference: &str, new: &str) {
        let i = self.active;
        let Some(path) = self.tabs[i].file.path.clone() else {
            return;
        };
        if self.tabs[i].file.doc.is_dirty() {
            self.message = Some(
                self.t(
                    "Save the document first: the rename saves it with the new references.",
                    "먼저 문서를 저장하세요. 이름을 바꾸면 새 참조로 저장합니다.",
                )
                .to_string(),
            );
            return;
        }
        if self.tabs[i].file.read_only {
            self.refuse_read_only();
            return;
        }
        let text = self.tabs[i].file.doc.text().to_string();
        let plan = match jarak_core::assets::plan_rename(&path, &text, reference, new) {
            Ok(p) => p,
            Err(e) => {
                self.message = Some(format!(
                    "{} {e}",
                    self.t("Not renamed:", "이름을 바꾸지 않음:")
                ));
                return;
            }
        };
        if let Err(e) = jarak_core::assets::move_no_replace(&plan.from, &plan.to) {
            self.message = Some(format!(
                "{} {e}",
                self.t("Not renamed:", "이름을 바꾸지 않음:")
            ));
            return;
        }
        let parts: Vec<(usize, usize, String)> = {
            let r = self.tabs[i].file.doc.text();
            plan.edits
                .iter()
                .map(|(b, s)| (r.byte_to_char(b.start), r.byte_to_char(b.end), s.clone()))
                .collect()
        };
        self.tabs[i].file.doc.replace(parts, None);
        match self.tabs[i].file.save() {
            Ok(()) => {
                self.message = Some(format!(
                    "{} {} → {}",
                    self.t("Renamed:", "이름 바꿈:"),
                    plan.from
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                    plan.to
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                ));
            }
            Err(e) => {
                // Back to the old name and text: the file on disk still refers to it.
                self.tabs[i].file.doc.undo();
                let _ = jarak_core::assets::move_no_replace(&plan.to, &plan.from);
                self.message = Some(format!(
                    "{} {e}",
                    self.t("Not renamed:", "이름을 바꾸지 않음:")
                ));
            }
        }
        self.open_assets();
    }

    /// Alt pressed and released alone shows or hides the menu bar (UI-10).
    fn alt_toggle(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());

        for e in events {
            match e {
                egui::Event::ModifiersChanged(m) => {
                    let alone = m.alt && !m.ctrl && !m.shift && !m.command && !m.mac_cmd;
                    if alone {
                        self.alt_armed = true;
                    } else if !m.alt && self.alt_armed && !m.any() {
                        self.alt_armed = false;
                        self.menu_bar = !self.menu_bar;
                    } else {
                        self.alt_armed = false;
                    }
                }
                // egui also reports the Alt key itself (AltLeft / AltRight): not another key.
                egui::Event::Key {
                    key: egui::Key::AltLeft | egui::Key::AltRight,
                    ..
                } => {}
                egui::Event::Key { .. }
                | egui::Event::Text(_)
                | egui::Event::PointerButton { .. }
                | egui::Event::MouseWheel { .. } => self.alt_armed = false,
                _ => {}
            }
        }
    }

    /// The command palette (kim-ui dialog, search field, menu items with their keys).
    fn palette_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(mut st) = self.palette.take() else {
            return;
        };
        let entries = palette::entries();
        let found = palette::filter(&entries, &st.query);
        st.selected = st.selected.min(found.len().saturating_sub(1));
        let (up, down, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if up {
            st.selected = st.selected.saturating_sub(1);
        }
        if down && st.selected + 1 < found.len() {
            st.selected += 1;
        }
        let lang = self.lang;
        let editing = self.editing;
        let mut run = None;
        let title = self.t("Commands", "명령").to_string();
        let label = self.t("Search commands", "명령 검색").to_string();
        let hint = self
            .t(
                "Type to filter: save, vim, ctrl+e…",
                "입력해 거르기: 저장, vim, ctrl+e…",
            )
            .to_string();
        let resp = kim_ui::dialog_sized(ctx, "jarak-palette", &title, 560.0, |ui| {
            let w = ui.available_width();
            let r = kim_ui::search_field(ui, &mut st.query, &label, &hint, w);
            if st.focus {
                r.request_focus();
                st.focus = false;
            }
            if r.changed() {
                st.selected = 0;
            }
            ui.add_space(space::S);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (row, &k) in found.iter().enumerate() {
                        let e = &entries[k];
                        let current = matches!(e.run, palette::Run::Editing(m) if m == editing);
                        let name = format!(
                            "{}{}",
                            lang.pick(e.en, e.ko),
                            if current { "  ✓" } else { "" }
                        );
                        // The selection goes under the item's text (a shape reserved before it).
                        let under = ui.painter().add(egui::Shape::Noop);
                        let resp = kim_ui::menu_item(ui, &name, false);
                        if row == st.selected {
                            ui.painter().set(
                                under,
                                egui::epaint::RectShape::filled(
                                    resp.rect,
                                    egui::CornerRadius::same(kim_design::radius::S as u8),
                                    c.bg_selected,
                                ),
                            );
                            if up || down {
                                resp.scroll_to_me(None);
                            }
                        }
                        if let Some(keys) = e.keys {
                            ui.painter().text(
                                egui::pos2(resp.rect.right() - space::S, resp.rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                keys,
                                egui::FontId::proportional(font::CAPTION),
                                c.text_muted,
                            );
                        }
                        if resp.clicked() {
                            run = Some(e.run);
                        }
                    }
                    if found.is_empty() {
                        ui.label(
                            RichText::new(
                                lang.pick("No command matches.", "맞는 명령이 없습니다."),
                            )
                            .size(font::BODY)
                            .color(c.text_muted),
                        );
                    }
                });
        });
        if enter && run.is_none() {
            run = found.get(st.selected).map(|k| entries[*k].run);
        }
        if let Some(r) = run {
            // The palette closes before the command runs (a command may open a dialog).
            if r != palette::Run::Act(Action::Palette) {
                self.run_entry(r);
            }
            ctx.memory_mut(|m| m.request_focus(view::view_id()));
            return;
        }
        if resp.should_close() {
            ctx.memory_mut(|m| m.request_focus(view::view_id()));
            return;
        }
        self.palette = Some(st);
    }

    /// The menu bar (hidden by default; Alt or the palette shows it).
    fn menu_bar_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        if !self.menu_bar {
            return;
        }
        let entries = palette::entries();
        let lang = self.lang;
        let mut run = None;
        egui::Panel::top("jarak-menu")
            .exact_size(size::CONTROL)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::XS as i8, 0)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                let bar = ui.max_rect();
                ui.painter().hline(
                    bar.x_range(),
                    bar.bottom(),
                    egui::Stroke::new(size::BORDER, c.border_subtle),
                );
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = space::XS;
                    for menu in palette::Menu::ALL {
                        let (en, ko) = menu.name();
                        let b = ui.add(kim_ui::Button::ghost(lang.pick(en, ko)));
                        kim_ui::popover(&b, 300.0, |ui| {
                            for e in entries.iter().filter(|e| e.menu == menu) {
                                let resp = kim_ui::menu_item(ui, lang.pick(e.en, e.ko), false);
                                if let Some(keys) = e.keys {
                                    ui.painter().text(
                                        egui::pos2(
                                            resp.rect.right() - space::S,
                                            resp.rect.center().y,
                                        ),
                                        egui::Align2::RIGHT_CENTER,
                                        keys,
                                        egui::FontId::proportional(font::CAPTION),
                                        c.text_muted,
                                    );
                                }
                                if resp.clicked() {
                                    run = Some(e.run);
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            });
        if let Some(r) = run {
            self.run_entry(r);
        }
    }

    /// The assets dialog: the document's images and the folder's orphans, with Show and Rename.
    fn assets_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(mut view) = self.assets_view.take() else {
            return;
        };
        let lang = self.lang;
        let mut reveal = None;
        let mut rename = None;
        let mut close = false;
        let title = self.t("Images and assets", "이미지와 asset").to_string();
        let resp = kim_ui::dialog_sized(ctx, "jarak-assets", &title, 620.0, |ui| {
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                ui.label(
                    RichText::new(lang.pick("In this document", "이 문서의 이미지"))
                        .size(font::BODY)
                        .strong()
                        .color(c.text_primary),
                );
                if view.refs.is_empty() {
                    ui.label(RichText::new(lang.pick("No images.", "이미지가 없습니다.")).color(c.text_muted));
                }
                for (url, local, what) in &view.refs {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(url).size(font::BODY).color(c.text_primary));
                        ui.label(RichText::new(what).size(font::CAPTION).color(c.text_muted));
                        let exists = local.as_deref().is_some_and(Path::is_file);
                        if exists {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(kim_ui::Button::ghost(lang.pick("Rename…", "이름 바꾸기…"))).clicked() {
                                    let stem = local
                                        .as_deref()
                                        .and_then(Path::file_stem)
                                        .map(|s| s.to_string_lossy().into_owned())
                                        .unwrap_or_default();
                                    view.rename = Some((url.clone(), stem));
                                }
                                if ui.add(kim_ui::Button::ghost(lang.pick("Show", "보기"))).clicked() {
                                    reveal = local.clone();
                                }
                            });
                        }
                    });
                    if let Some((r, name)) = view.rename.as_mut()
                        && r == url
                    {
                        ui.horizontal(|ui| {
                            let w = (ui.available_width() - 200.0).max(120.0);
                            kim_ui::text_field(ui, name, lang.pick("New name", "새 이름"), "", w);
                            if ui.add(kim_ui::Button::primary(lang.pick("Rename", "바꾸기"))).clicked() {
                                rename = Some((r.clone(), name.clone()));
                            }
                        });
                    }
                }
                ui.add_space(space::M);
                ui.label(
                    RichText::new(lang.pick(
                        "Orphaned (no Markdown file in this folder refers to them; nothing is deleted)",
                        "고아 파일(이 폴더의 어떤 Markdown도 참조하지 않음, 아무것도 지우지 않음)",
                    ))
                    .size(font::BODY)
                    .strong()
                    .color(c.text_primary),
                );
                if view.orphans.is_empty() {
                    ui.label(RichText::new(lang.pick("None.", "없음.")).color(c.text_muted));
                }
                for p in &view.orphans {
                    ui.horizontal(|ui| {
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.label(RichText::new(name).size(font::BODY).color(c.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kim_ui::Button::ghost(lang.pick("Show", "보기"))).clicked() {
                                reveal = Some(p.clone());
                            }
                        });
                    });
                }
            });
            kim_ui::dialog_actions(ui, |ui| {
                if ui
                    .add(kim_ui::Button::secondary(lang.pick("Close", "닫기")))
                    .clicked()
                {
                    close = true;
                }
            });
        });
        if let Some(p) = reveal {
            self.reveal(&p);
        }
        if close || resp.should_close() {
            return;
        }
        self.assets_view = Some(view);
        if let Some((r, n)) = rename {
            self.rename_asset(&r, &n);
        }
    }

    /// Closes document `i` without asking (its changes were saved or given up), or the window
    /// when it is the only one.
    fn close_or_quit(&mut self, i: usize) {
        if self.tabs.len() > 1 {
            self.close_now(i);
        } else {
            self.quitting = true;
        }
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
            editing: Editing::Standard,
            vim: jarak_core::vim::Vim::new(),
            vim_tab: 0,
            settings: None,
            palette: None,
            menu_bar: false,
            alt_armed: false,
            assets_view: None,
        };
        app.open_args(args);
        app
    }

    /// Sets up a UI audit's view (ADR 0099): `editor`, `editor-empty`, `editor-selection`,
    /// `editor-wrap`, `question`, `preview`, `split`, `paste-image` (pastes a sample bitmap and
    /// saves: the UI audit runs on a copy), `images` (sample pictures next to the copy), `vim`,
    /// `vim-insert`, `vim-command`, `palette`, `menu-bar`, `assets` (Step 92). Only used while `KIM_UI_INSPECT` is set.
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
            // Step 92: Vim editing (Normal with a search highlight and an unfinished command,
            // Insert, the command line), the palette, the menu bar, the assets dialog. The mode
            // is set directly: an audit writes no settings.
            "vim" | "vim-insert" | "vim-command" | "vim-session" => {
                self.editing = Editing::Vim;
                let i = self.active;
                self.vim.reset(&mut self.tabs[i].file.doc);
                // `\r` is Enter, `\u{1b}` Esc. The session (the e2e's) edits and writes with `:w`.
                let keys = match screen {
                    "vim" => "gg/Markdown\r2d",
                    "vim-insert" => "ggA",
                    "vim-session" => "ggOtop\u{1b}j0dwGdd:%s/gamma/GAMMA/g\r:w\r",
                    _ => ":%s/Markdown/GFM/g",
                };
                for c in keys.chars() {
                    match c {
                        '\r' => self.vim_input(Ok(jarak_core::vim::Key::Enter)),
                        '\u{1b}' => self.vim_input(Ok(jarak_core::vim::Key::Esc)),
                        c => self.vim_input(Err(c.to_string())),
                    };
                }
                if screen != "vim-session" {
                    self.message = None;
                }
            }
            "palette" => {
                self.palette = Some(palette::PaletteState {
                    query: "save".into(),
                    selected: 0,
                    focus: true,
                });
            }
            "menu-bar" => self.menu_bar = true,
            "assets" => {
                let dir = self
                    .tab()
                    .file
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                if let Some(dir) = dir {
                    let _ = images::write_samples(&dir.join(jarak_core::assets::ASSET_DIR));
                }
                let t = self.tab_mut();
                t.file.doc.select_all();
                t.file.doc.paste(images::SAMPLE_DOC);
                self.open_assets();
            }
            // Step 91: pictures in the preview — scaled to the column, at their size (PNG and
            // WebP), a missing one and a remote one (the audit runs on a copy; the pictures are
            // written next to it, the document is not saved).
            "images" => {
                let dir = self
                    .tab()
                    .file
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                if let Some(dir) = dir {
                    let _ = images::write_samples(&dir.join(jarak_core::assets::ASSET_DIR));
                }
                let t = self.tab_mut();
                t.file.doc.select_all();
                t.file.doc.paste(images::SAMPLE_DOC);
                t.file.doc.doc_start(false);
                t.view.top_line = 0;
                t.preview.top_block = 0;
                t.preview.top_px = 0.0;
                t.set_mode(Mode::Preview);
            }
            // Step 90: the MVP journey's middle — paste a bitmap, save, show the preview.
            "paste-image" => {
                let t = self.tab_mut();
                t.file.doc.doc_end(false);
                self.paste_image(&images::ClipImage::Dib(images::sample_dib(320, 180)));
                self.save(self.active);
                // Show the pasted picture: the preview starts at the caret's paragraph.
                let t = self.tab_mut();
                let head = t.file.doc.selection().primary().head;
                t.view.top_line = t.file.doc.text().char_to_line(head);
                t.set_mode(Mode::Preview);
            }
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
            Action::Palette => {
                self.palette = if self.palette.is_some() {
                    None
                } else {
                    Some(palette::PaletteState {
                        focus: true,
                        ..Default::default()
                    })
                };
            }
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
            Action::Paste => self.paste_from_clipboard(),
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

    /// Ctrl+V: text when the clipboard has text (also when it has an image too, as office
    /// programs put both), otherwise an image into a Markdown document.
    pub fn paste_from_clipboard(&mut self) {
        if let Some(s) = self.clipboard.get_text() {
            self.paste(&s);
            return;
        }
        let Some(img) = self.clipboard.get_image() else {
            return;
        };
        self.paste_image(&img);
    }

    /// Saves `img` as asset(s) next to the document and inserts the reference(s) as one undo step
    /// (`JARAK-FR-042`, `JARAK-FR-043`). An untitled document is saved first (Save As); only
    /// Markdown documents take images; nothing is overwritten, and undo leaves the files.
    pub fn paste_image(&mut self, img: &images::ClipImage) {
        if self.tab().file.read_only {
            self.refuse_read_only();
            return;
        }
        if self.tab().file.path.is_none() {
            self.message = Some(
                self.t(
                    "Save the document first: a pasted image is stored next to it.",
                    "먼저 문서를 저장하세요. 붙여넣은 이미지는 문서 옆에 저장됩니다.",
                )
                .to_string(),
            );
            if !self.save_as(self.active) {
                return;
            }
        }
        if self.tab().language() != "Markdown" {
            self.message = Some(
                self.t(
                    "Images paste into Markdown documents (.md).",
                    "이미지는 Markdown 문서(.md)에 붙여넣을 수 있습니다.",
                )
                .to_string(),
            );
            return;
        }
        let doc_path = self.tab().file.path.clone().unwrap_or_default();
        let list = match images::asset_bytes(img) {
            Ok(l) => l,
            Err(e) => {
                self.message = Some(format!(
                    "{} {e}",
                    self.t("Not pasted:", "붙여넣지 못했습니다:")
                ));
                return;
            }
        };
        let date = today();
        let mut refs = Vec::new();
        let mut names = Vec::new();
        for (ext, bytes) in &list {
            match jarak_core::assets::write_new(&doc_path, ext, date, bytes) {
                Ok(a) => {
                    refs.push(a.markdown("image"));
                    names.push(a.reference);
                }
                Err(e) => {
                    self.message = Some(format!(
                        "{} {e}",
                        self.t("Not pasted:", "붙여넣지 못했습니다:")
                    ));
                    break;
                }
            }
        }
        if refs.is_empty() {
            return;
        }
        let eol = self.tab().file.doc.eol.as_str();
        let text = refs.join(eol);
        let t = self.tab_mut();
        t.file.doc.paste(&text);
        t.view.after_edit();
        self.message = Some(format!(
            "{} {}",
            self.t("Pasted image:", "이미지를 붙여넣었습니다:"),
            names.join(", ")
        ));
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
        let old = self.tabs[i].file.path.clone();
        let r = self.tabs[i].file.save_as(&p);
        let saved = self.report_save(i, r);
        if saved
            && self.pending.is_none()
            && let Some(old) = old
        {
            let list = assets_to_copy(&self.tabs[i].file.doc.text().to_string(), &old, &p);
            if !list.is_empty() {
                self.active = i;
                self.pending = Some(Pending::CopyAssets(list));
            }
        }
        saved
    }

    /// Copies the images of a Save As to another folder (never replacing a file).
    fn copy_assets(&mut self, list: &[(PathBuf, PathBuf)]) {
        let (mut copied, mut same, mut kept, mut failed) = (0, 0, Vec::new(), Vec::new());
        for (from, to) in list {
            match copy_new(from, to) {
                Ok(true) => copied += 1,
                Ok(false) => same += 1,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    kept.push(to.display().to_string());
                }
                Err(e) => failed.push(format!("{}: {e}", from.display())),
            }
        }
        let mut msg = format!("{} {copied}", self.t("Images copied:", "복사한 이미지:"));
        if same > 0 {
            msg.push_str(&format!(
                " · {} {same}",
                self.t("already there:", "이미 있음:")
            ));
        }
        if !kept.is_empty() {
            msg.push_str(&format!(
                " · {} {}",
                self.t(
                    "not copied, a different file has the name:",
                    "다른 파일이 같은 이름이라 복사하지 않음:"
                ),
                kept.join(", ")
            ));
        }
        if !failed.is_empty() {
            msg.push_str(&format!(
                " · {} {}",
                self.t("failed:", "실패:"),
                failed.join(", ")
            ));
        }
        self.message = Some(msg);
    }

    /// Shows the local image a reference names in Explorer (`JARAK-FR-045`).
    pub fn reveal(&mut self, path: &Path) {
        self.message = Some(if !path.is_file() {
            format!(
                "{} {}",
                self.t("Not found:", "찾을 수 없습니다:"),
                path.display()
            )
        } else if self.opener.reveal(path) {
            format!(
                "{} {}",
                self.t("Shown in Explorer:", "탐색기에서 표시:"),
                path.display()
            )
        } else {
            format!(
                "{} {}",
                self.t("Could not show", "표시하지 못했습니다:"),
                path.display()
            )
        });
    }

    /// A Ctrl+click in the source at `pos`: an image reference there is shown in Explorer.
    /// `true` when there was one (Step 96 gives Ctrl+click elsewhere a second caret).
    pub fn reveal_reference_at(&mut self, pos: usize) -> bool {
        let t = self.tab();
        if t.language() != "Markdown" {
            return false;
        }
        let Some(doc) = t.file.path.clone() else {
            return false;
        };
        let text = t.file.doc.text();
        let line = text.char_to_line(pos.min(text.len_chars()));
        let start = text.line_to_char(line);
        let line_text = text.line(line).to_string();
        let at = line_text
            .char_indices()
            .nth(pos - start)
            .map_or(line_text.len(), |(b, _)| b);
        let Some(url) = jarak_core::assets::reference_at(&line_text, at) else {
            return false;
        };
        match jarak_core::assets::resolve(&doc, &url) {
            Some(p) => self.reveal(&p),
            None => {
                self.message = Some(format!(
                    "{} {url}",
                    self.t(
                        "Not a local image in the document's folder:",
                        "문서 폴더 안의 로컬 이미지가 아닙니다:"
                    )
                ));
            }
        }
        true
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
        if let Pending::CopyAssets(list) = &p {
            if save {
                self.copy_assets(list);
            }
            return;
        }
        if save && !self.save(i) {
            // Not saved (cancelled dialog, an error): nothing is closed.
            return;
        }
        match p {
            Pending::CopyAssets(_) => {}
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
        self.alt_toggle(&ctx);
        self.vim_frame();
        self.menu_bar_ui(ui, &c);
        self.status_bar(ui, &c);
        let mut ctrl_click = None;
        let lang = self.lang;
        let (focused, link) = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(c.bg_base))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let read_only = self.tab().file.read_only;
                let t = &mut self.tabs[self.active];
                match t.mode {
                    Mode::Source => {
                        let v = view::show(ui, rect, &mut t.file.doc, &mut t.view, read_only);
                        ctrl_click = v.ctrl_click;
                        (v.focused, preview::PreviewOutput::default())
                    }
                    Mode::Preview => {
                        let r = t.rendered();
                        let doc = t.file.path.clone();
                        let out = preview::show(
                            ui,
                            rect,
                            &r,
                            &mut t.preview,
                            t.view.zoom,
                            doc.as_deref(),
                            lang,
                        );
                        (false, out)
                    }
                    Mode::Split => {
                        let mid = rect.center().x.round();
                        let left = Rect::from_min_max(rect.min, egui::pos2(mid, rect.bottom()));
                        let right = Rect::from_min_max(egui::pos2(mid + 1.0, rect.top()), rect.max);
                        let v = view::show(ui, left, &mut t.file.doc, &mut t.view, read_only);
                        ctrl_click = v.ctrl_click;
                        let focused = v.focused;
                        // The preview follows the source's top line.
                        let r = t.rendered();
                        t.preview.scroll_to_line(&r, t.view.top_line);
                        let doc = t.file.path.clone();
                        let out = preview::show(
                            ui,
                            right,
                            &r,
                            &mut t.preview,
                            t.view.zoom,
                            doc.as_deref(),
                            lang,
                        );
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
        } else if let Some(p) = link.reveal {
            self.reveal(&p);
        } else if let Some(p) = link.plain_image {
            self.message = Some(format!(
                "{} {}",
                self.t(
                    "Ctrl+click to show it in Explorer:",
                    "Ctrl+클릭하면 탐색기에서 보여 줍니다:"
                ),
                p.display()
            ));
        }
        if let Some(pos) = ctrl_click {
            self.reveal_reference_at(pos);
        }
        let dialog_open = self.palette.is_some() || self.assets_view.is_some();
        if self.pending.is_none() && !dialog_open {
            let preview_only = self.tab().mode == Mode::Preview;
            if !preview_only && !focused && !ctx.memory(|m| m.focused().is_some()) {
                ctx.memory_mut(|m| m.request_focus(view::view_id()));
            }
            if focused || preview_only {
                self.keyboard(&ctx);
            }
        }
        self.question(&ctx, &c);
        // Ctrl+Shift+P toggles the palette even while it has the focus.
        if dialog_open
            && ctx.input(|i| {
                i.events.iter().any(|e| {
                    matches!(e, egui::Event::Key { key: egui::Key::P, pressed: true, modifiers, .. }
                        if modifiers.shift && (modifiers.ctrl || modifiers.command))
                })
            })
        {
            self.palette = None;
        } else {
            self.palette_ui(&ctx, &c);
        }
        self.assets_ui(&ctx, &c);
        if kim_ui::inspect::active() {
            // Measure once the pictures on screen are decoded (at most 15 s after the start).
            let loading = self
                .tab()
                .preview
                .media
                .lock()
                .map(|m| m.loading())
                .unwrap_or(0);
            if loading == 0 || ctx.input(|i| i.time) > 15.0 {
                kim_ui::inspect::finish_frame(&ctx, "jarak", &self.audit_screen);
            } else {
                ctx.request_repaint();
            }
        }
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        if self.tab().mode == Mode::Preview {
            self.preview_keys(&events);
            return;
        }
        if self.editing == Editing::Vim {
            self.vim_keyboard(events);
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

    /// Keys in Vim editing: Vim's first; what Vim does not take (Ctrl+S, Ctrl+E, …) works as in
    /// Standard editing.
    fn vim_keyboard(&mut self, events: Vec<egui::Event>) {
        for e in events {
            match e {
                egui::Event::Text(s) => {
                    if !s.chars().all(char::is_control) {
                        self.vim_input(Err(s));
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    if self.vim.mode.is_insert() && self.vim.prompt.is_none() {
                        let v = &mut self.tab_mut().view;
                        v.preedit = text;
                        v.after_edit();
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Commit(s)) => {
                    self.tab_mut().view.preedit.clear();
                    if !s.is_empty() {
                        self.vim_input(Err(s));
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let taken = vim_key(key, modifiers).is_some_and(|k| self.vim_input(Ok(k)));
                    if !taken && let Some(a) = keys::action_for(key, modifiers) {
                        // Plain editing keys belong to Vim's modes; the rest are the window's.
                        let plain_edit = matches!(
                            a,
                            Action::Cmd(_)
                                | Action::Indent
                                | Action::Outdent
                                | Action::ToggleOverwrite
                        ) && !modifiers.command
                            && !modifiers.ctrl
                            && !self.vim.mode.is_insert();
                        if !plain_edit {
                            self.act(a);
                            let i = self.active;
                            if !self.vim.mode.is_insert() && !self.vim.mode.is_visual() {
                                self.vim.reset(&mut self.tabs[i].file.doc);
                            }
                        }
                    }
                }
                egui::Event::Copy => self.act(Action::Copy),
                egui::Event::Cut => self.act(Action::Cut),
                egui::Event::Paste(s) => {
                    self.vim_input(Err(s));
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
        let mut message = self.message.clone();
        // Vim: the mode (and the keys of an unfinished command) lead; the command line replaces
        // the message while it is typed.
        let mut vim_label = None;
        if self.editing == Editing::Vim && t.mode != Mode::Preview {
            if let Some(p) = &self.vim.prompt {
                message = Some(format!("{p}\u{2581}"));
            }
            let pending = self.vim.pending();
            let mode = match self.vim.prompt.as_deref().map(|p| p.chars().next()) {
                Some(Some(':')) => "COMMAND",
                Some(_) => "SEARCH",
                None => self.vim.mode.label(),
            };
            vim_label = Some(if pending.is_empty() {
                mode.to_string()
            } else {
                format!("{mode}  {pending}")
            });
            // INS/OVR belong to Standard editing.
            items.retain(|(s, _)| s != "INS" && s != "OVR");
        }
        let mut leave_vim = false;
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
                        if let Some(l) = &vim_label {
                            let tip = self.t(
                                "Vim editing — click to switch to Standard editing",
                                "Vim 편집 — 누르면 표준 편집으로 바꿉니다",
                            );
                            let r = ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(l).size(font::CAPTION).strong().color(c.accent),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text(tip);
                            r.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Vim mode {l}"))
                            });
                            if r.clicked() {
                                leave_vim = true;
                            }
                        }
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
                if leave_vim {
                    self.set_editing(Editing::Standard);
                }
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
        let copy = match &self.pending {
            Some(Pending::CopyAssets(list)) => Some(list.len()),
            _ => None,
        };
        let (title, body) = match copy {
            Some(n) => (
                format!(
                    "{} {n}",
                    self.t(
                        "Copy the document's images to the new folder? Images:",
                        "문서의 이미지를 새 폴더로 복사할까요? 이미지:"
                    )
                ),
                self.t(
                    "Its image references are relative; without the images they show as missing. Existing files are never replaced.",
                    "이미지 참조는 상대 경로라서 이미지가 없으면 '찾을 수 없음'으로 보입니다. 기존 파일은 절대 덮어쓰지 않습니다.",
                )
                .to_string(),
            ),
            None => (
                format!(
                    "{} {name}?",
                    self.t("Save changes to", "변경 내용을 저장할까요:")
                ),
                self.t(
                    "Your changes are lost if you don't save them.",
                    "저장하지 않으면 변경 내용이 사라집니다.",
                )
                .to_string(),
            ),
        };
        let (save, dont, cancel) = if copy.is_some() {
            (
                self.t("Copy", "복사").to_string(),
                self.t("Don't copy", "복사 안 함").to_string(),
                String::new(),
            )
        } else {
            (
                self.t("Save", "저장").to_string(),
                self.t("Don't save", "저장 안 함").to_string(),
                self.t("Cancel", "취소").to_string(),
            )
        };
        let mut choice: Option<Option<bool>> = None;
        egui::Modal::new(egui::Id::unique("jarak-unsaved")).show(ctx, |ui| {
            ui.set_width(size::DIALOG_WIDTH);
            ui.label(
                RichText::new(&title)
                    .size(font::TITLE)
                    .color(c.text_primary),
            );
            ui.add_space(space::S);
            ui.label(
                RichText::new(&body)
                    .size(font::BODY)
                    .color(c.text_secondary),
            );
            ui.add_space(space::L);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(kim_ui::Button::primary(&save)).clicked() {
                    choice = Some(Some(true));
                }
                if ui.add(kim_ui::Button::secondary(&dont)).clicked() {
                    choice = Some(Some(false));
                }
                if !cancel.is_empty() && ui.add(kim_ui::Button::secondary(&cancel)).clicked() {
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

/// The images a document saved from `old` to `new` refers to that exist next to `old` and not
/// yet next to `new` (source, destination); empty when the folder did not change.
pub fn assets_to_copy(text: &str, old: &Path, new: &Path) -> Vec<(PathBuf, PathBuf)> {
    let same = |a: &Path, b: &Path| {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    };
    match (old.parent(), new.parent()) {
        (Some(a), Some(b)) if !same(a, b) => {}
        _ => return Vec::new(),
    }
    if !jarak_core::markdown::is_markdown_path(new) {
        return Vec::new();
    }
    let mut out: Vec<(PathBuf, PathBuf)> = Vec::new();
    for r in jarak_core::assets::references(text) {
        let (Some(from), Some(to)) = (
            jarak_core::assets::resolve(old, &r.url),
            jarak_core::assets::resolve(new, &r.url),
        ) else {
            continue;
        };
        if from.is_file() && !to.exists() && !out.iter().any(|(f, _)| same(f, &from)) {
            out.push((from, to));
        }
    }
    out
}

/// Copies `from` to `to` without replacing anything: `Ok(true)` copied, `Ok(false)` the same
/// bytes are there already, `AlreadyExists` a different file has the name.
pub fn copy_new(from: &Path, to: &Path) -> std::io::Result<bool> {
    let bytes = std::fs::read(from)?;
    if to.exists() {
        return if std::fs::read(to)? == bytes {
            Ok(false)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "a different file has the name",
            ))
        };
    }
    if let Some(d) = to.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    use std::io::Write as _;
    if let Err(e) = f.write_all(&bytes).and_then(|()| f.sync_all()) {
        drop(f);
        // Only the file this call created (and could not fill) is removed.
        let _ = std::fs::remove_file(to);
        return Err(e);
    }
    Ok(true)
}

/// Today's local date for asset names (year, month, day).
pub fn today() -> (i32, u32, u32) {
    #[cfg(windows)]
    #[allow(unsafe_code)]
    {
        // SAFETY: GetLocalTime fills the structure it is given.
        let t = unsafe {
            let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
            t
        };
        (i32::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay))
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
