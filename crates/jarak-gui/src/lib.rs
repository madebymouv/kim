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

pub mod assist;
pub mod clipboard;
pub mod column;
pub mod docops;
pub mod encoding;
pub mod export_ui;
pub mod find;
pub mod handoff;
pub mod hashdlg;
pub mod images;
pub mod keys;
pub mod languages;
pub mod macros;
pub mod media;
pub mod navmap;
pub mod outline;
pub mod palette;
pub mod prefs;
pub mod preview;
pub mod recover;
pub mod results;
pub mod sessions;
pub mod settings_ui;
pub mod shortcuts;
pub mod tabs;
pub mod view;
pub mod views;
pub mod watch;
pub mod workspace;

use eframe::egui::{self, Align, Layout, Rect, RichText, Vec2};
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
    /// Step 95: a folder (Find in Files).
    fn folder(&self) -> Option<PathBuf> {
        None
    }
    /// Step 106: where an export goes (the Save dialog asks before it replaces a file).
    fn save_export(
        &self,
        _suggested: &str,
        _format: jarak_core::export::Format,
    ) -> Option<PathBuf> {
        None
    }
    /// Step 106: the folder a Markdown document is published to.
    fn export_folder(&self) -> Option<PathBuf> {
        None
    }
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
    /// Step 93: stable across reorders and closes (the views name documents by it).
    pub id: u64,
    /// Which views show it, and the other view's state of a clone.
    pub pane: views::Pane,
    pub alt: Option<Box<views::PaneState>>,
    /// Step 94: the text the search scans, for one history state.
    pub(crate) hay: Option<jarak_core::search::Haystack>,
    /// Step 98: the language (detected, plain or chosen), its line-start states and the edit
    /// generation and language they are for.
    pub lang_choice: LangChoice,
    pub(crate) hl: jarak_core::syntax::Highlighter,
    pub(crate) hl_gen: u64,
    pub(crate) hl_lang: String,
    /// Step 99: the folds and symbols.
    pub outline: outline::Outline,
    /// Step 100: what the bracket highlight was found for (edit generation, caret, on).
    pub(crate) brace_key: Option<(u64, usize, bool)>,
    /// Step 101: the Document Map's rows.
    pub map: navmap::MapCache,
    pub file: FileDocument,
    pub view: ViewState,
    pub mode: Mode,
    pub preview: preview::PreviewState,
    /// Whether the text mixes line endings, for the history state it was counted at (counting
    /// a large file every frame would cost a frame).
    eol_cache: Option<(u64, bool)>,
    /// When the history state last changed (large documents recount only after a pause).
    changed_at: Option<(u64, f64)>,
    /// Step 104: the name shown instead of the file's (the version on disk opened to compare).
    pub label: Option<String>,
    /// Step 104: the document follows its file (tail -f), and whether it was read-only before.
    pub monitor: bool,
    pub(crate) read_only_before_monitor: bool,
    /// Step 104: another program deleted or moved its file (the text stays, unsaved).
    pub missing: bool,
    /// Step 105: the settings for new documents (word wrap, how Markdown opens) are still to
    /// be applied (a restored session's documents keep their own).
    pub(crate) fresh: bool,
}

impl Tab {
    pub(crate) fn new(file: FileDocument) -> Tab {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Tab {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            pane: views::Pane::Main,
            alt: None,
            hay: None,
            lang_choice: LangChoice::Auto,
            hl: jarak_core::syntax::Highlighter::default(),
            hl_gen: 0,
            hl_lang: String::new(),
            outline: outline::Outline::default(),
            brace_key: None,
            map: navmap::MapCache::default(),
            file,
            view: ViewState::default(),
            mode: Mode::Source,
            preview: preview::PreviewState::default(),
            eol_cache: None,
            changed_at: None,
            label: None,
            monitor: false,
            read_only_before_monitor: false,
            missing: false,
            fresh: true,
        }
    }

    /// The name its tab and the window show.
    pub fn title(&self) -> String {
        self.label.clone().unwrap_or_else(|| self.file.title())
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
                // Step 103: one kind of line ending in the text is the document's (after an undo of
                // an EOL conversion too), so new lines and the status bar follow the text.
                if !mixed
                    && let Some(dominant) = counts.dominant()
                    && dominant != self.file.doc.eol
                {
                    self.file.doc.eol = dominant;
                }
                self.eol_cache = Some((state, mixed));
                mixed
            }
        }
    }

    /// A new document nobody typed into yet (replaced by the first file opened).
    pub(crate) fn is_blank(&self) -> bool {
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

/// Step 98: a document's language — detected from its name and first line, plain text, or chosen
/// in the Language menu (a built-in or a user-defined language, by name).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LangChoice {
    #[default]
    Auto,
    Plain,
    Named(String),
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

/// A `key = value` line of the settings file (`None` when it is not there).
pub fn read_setting(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Writes `key = value` into the settings file (a string value quoted), keeping its other lines.
pub fn write_setting(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
    let line = if value == "true" || value == "false" || value.parse::<f64>().is_ok() {
        format!("{key} = {value}")
    } else {
        format!("{key} = \"{value}\"")
    };
    write_line(path, key, &line)
}

/// Writes the editing mode into the settings file, keeping its other lines.
pub fn write_editing(path: &Path, e: Editing) -> std::io::Result<()> {
    write_setting(
        path,
        "editing_mode",
        if e == Editing::Vim { "vim" } else { "standard" },
    )
}

fn write_line(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
    let value = value.to_string();
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut found = false;
    let mut lines: Vec<String> = old
        .lines()
        .map(|l| {
            if l.split_once('=').is_some_and(|(k, _)| k.trim() == key) {
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
    /// Step 93: closing several documents — this one (unsaved), then the rest (ids).
    CloseQueue(usize, Vec<u64>),
    /// Step 93: move this document's file to the Recycle Bin (asked first).
    Recycle(u64),
}

/// The window's state.
pub struct JarakApp {
    pub lang: Lang,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub incoming: Incoming,
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
    /// Step 93: the second view, the main view's active document while the second has the
    /// focus, the recent and the closed files, the session, the side panel, the Documents dialog.
    pub second: Option<views::Second>,
    pub main_active: u64,
    pub recent: Vec<PathBuf>,
    pub closed: Vec<PathBuf>,
    pub session: Option<sessions::SessionFiles>,
    session_saved: bool,
    pub workspace: Option<workspace::Workspace>,
    pub side: Option<workspace::Side>,
    pub docs_dialog: Option<workspace::DocsDialog>,
    /// Step 94: the find panel, its histories and the incremental search bar.
    pub find: find::FindPanel,
    pub find_history: Vec<String>,
    pub replace_history: Vec<String>,
    pub incremental: Option<find::Incremental>,
    /// Where the documents were drawn last (the find panel goes at its top right).
    central: Rect,
    /// The theme's colours of the frame.
    colors: Colors,
    /// When an Alt+letter chord came (its text is dropped).
    alt_text: Option<f64>,
    /// Step 95: Find / Replace in Files running on a thread, the results panel, the Replace in
    /// Files preview, where batches are journaled and the last one (to undo).
    pub job: Option<results::Job>,
    pub results: Option<results::Results>,
    pub preview: Option<results::Preview>,
    pub journal_root: Option<PathBuf>,
    pub last_batch: Option<String>,
    /// Step 96: the Column Editor (its fields kept), virtual space (a setting) and where a
    /// Begin/End Select began (the tab and the position).
    pub column_editor: Option<column::ColumnEditor>,
    pub column_editor_open: bool,
    pub virtual_space: bool,
    pub begin_select: Option<(u64, usize)>,
    /// Step 97: Copy and Cut without a selection take the line (a setting, on by default), and
    /// the hash dialog.
    pub copy_line: bool,
    pub hash_dialog: Option<hashdlg::HashDialog>,
    /// Step 98: the user-defined languages and their folder.
    pub user_langs: Vec<jarak_core::syntax::Lang>,
    pub languages_dir: Option<PathBuf>,
    /// Step 99: the Function List (open, its filter) and the Go to dialog.
    pub function_list: bool,
    pub function_filter: String,
    pub goto: Option<outline::GotoDialog>,
    /// Step 100: the assistance settings, the completion popup, the parameter hint, the API
    /// files and snippets, and the folder they are read from.
    pub assist: assist::AssistSettings,
    pub completion: Option<assist::Completion>,
    /// The word (its document and start) for which the popup found nothing while typing.
    pub completion_miss: Option<(u64, usize, String)>,
    pub hint: Option<assist::Hint>,
    pub apis: Vec<jarak_core::assist::Api>,
    pub snippets: Vec<jarak_core::assist::Snippet>,
    pub assist_dir: Option<PathBuf>,
    /// Step 101: the Document Map, the navigation history (places, where in it, the last place
    /// seen, a jump of its own), quick open, the Summary.
    pub doc_map: bool,
    pub nav: Vec<navmap::Place>,
    pub nav_at: usize,
    pub nav_last: Option<navmap::Place>,
    pub nav_moving: bool,
    pub quick: Option<navmap::QuickOpen>,
    pub summary_open: bool,
    /// Step 102: the macro being recorded, the last recorded one, a playback running; the saved
    /// macros and commands (`macros.json`, not written over when it could not be read); the
    /// program runner; the open macro or command dialog; the web search address.
    pub recording: Option<Vec<jarak_core::macros::Step>>,
    pub last_macro: Option<Vec<jarak_core::macros::Step>>,
    pub playing: bool,
    pub user_cmds: jarak_core::macros::UserCommands,
    pub user_cmds_broken: bool,
    pub macros_path: Option<PathBuf>,
    runner: Arc<dyn macros::Runner>,
    pub macro_dialog: Option<macros::Dialog>,
    pub search_engine: String,
    /// Step 103: the Encoding menu's question, the Show Symbol settings, the format of new
    /// documents.
    pub enc_question: Option<encoding::Question>,
    pub symbols: encoding::Symbols,
    pub new_format: file::Format,
    /// Step 104: files changed by other programs, the settings for them, recovery snapshots and
    /// the offer of a crashed window's documents.
    pub watch: watch::Watch,
    watch_auto: bool,
    pub detect_changes: bool,
    pub reload_unmodified: bool,
    pub recovery: Option<recover::Recovery>,
    pub recovery_offer: Option<recover::Offer>,
    /// Step 106: publishing would replace files; asked first.
    pub publish_question: Option<export_ui::PublishQuestion>,
    /// Step 105: the settings applied to the window, the Settings window, and what was applied
    /// (the language is fixed for the run when `--lang` named it, or by a test).
    pub prefs: prefs::Prefs,
    pub settings_win: Option<settings_ui::SettingsWin>,
    pub(crate) lang_fixed: bool,
    pub(crate) theme_now: Option<kim_design::Theme>,
    pub(crate) applied_theme: Option<kim_design::Theme>,
    pub(crate) applied_font: Option<String>,
    pub(crate) settings_stamp: Option<std::time::SystemTime>,
    pub(crate) prefs_focused: bool,
    pub(crate) audit_settings: bool,
    /// Step 105: the user's shortcuts (`hotkeys.json`).
    pub bindings: shortcuts::Bindings,
}

/// Step 102: the web search's address unless the settings name another (`$(QUERY)`: the words).
pub const DEFAULT_SEARCH: &str = "https://duckduckgo.com/?q=$(QUERY)";

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
    /// Step 98: the folder of the user-defined languages (`*.json`), read now.
    pub fn with_languages(mut self, dir: PathBuf) -> JarakApp {
        self.languages_dir = Some(dir);
        self.reload_languages();
        self
    }

    /// Step 95: where Replace in Files keeps the originals (the program passes its journal folder).
    pub fn with_journal(mut self, dir: PathBuf) -> JarakApp {
        self.journal_root = Some(dir);
        self
    }

    pub fn with_clipboard(mut self, c: Arc<dyn clipboard::Clipboard>) -> JarakApp {
        self.clipboard = c;
        self
    }

    /// Keeps the settings file `path` (the program passes `%APPDATA%\mouv\jarak\settings.toml`)
    /// and applies every setting in it (Step 105: live from then on).
    pub fn with_settings(mut self, path: PathBuf) -> JarakApp {
        self.settings = Some(path);
        self.apply_settings();
        self
    }

    /// Step 105: the window's language follows the `language` setting (no `--lang` given).
    pub fn with_lang_from_settings(mut self) -> JarakApp {
        self.lang_fixed = false;
        self.apply_settings();
        self
    }

    /// Reads the settings the window's parts keep themselves (editing mode, assistance, Show
    /// Symbol, the new documents' encoding, …) from `path`.
    pub(crate) fn read_settings_file(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if let Some(e) = read_editing(&path) {
            self.editing = e;
        }
        // Step 96: virtual space (off unless the file says so).
        self.virtual_space = read_setting(&path, "virtual_space").is_some_and(|v| v == "true");
        // Step 97: copying the line without a selection (on unless the file says no).
        self.copy_line = read_setting(&path, "copy_line").is_none_or(|v| v != "false");
        // Step 100: completion, hints, snippets and the typing aids.
        self.assist = assist::AssistSettings::read(&path);
        // Step 102: the web search's address (`$(QUERY)` the words searched for).
        if let Some(s) = read_setting(&path, "search_engine").filter(|s| s.contains("$(QUERY)")) {
            self.search_engine = s;
        }
        // Step 103: Show Symbol and the encoding of new documents.
        self.symbols = encoding::Symbols::read(&path);
        self.new_format = match read_setting(&path, "new_encoding").as_deref() {
            Some("utf-8-bom") => file::Format {
                encoding: TextEncoding::Utf8,
                bom: true,
            },
            Some("ansi") => file::Format {
                encoding: file::system_encoding(),
                bom: false,
            },
            _ => file::Format::utf8(),
        };
        // Step 104: detecting changes on disk (on unless the file says no), reloading unmodified
        // documents silently (off unless it says so).
        self.detect_changes = read_setting(&path, "detect_changes").is_none_or(|v| v != "false");
        self.reload_unmodified =
            read_setting(&path, "reload_unmodified_silently").is_some_and(|v| v == "true");
    }

    /// A document's notice in the window's language (the core says them in English).
    pub(crate) fn notice_text(&self, n: &str) -> String {
        let ko = match n {
            "Select the text first." => "먼저 텍스트를 선택하세요.",
            "This language has no comments." => "이 언어에는 주석이 없습니다.",
            "The selection is not in a block comment." => {
                "선택 영역이 블록 주석 안에 있지 않습니다."
            }
            "No bookmark." => "책갈피가 없습니다.",
            "No line to remove." => "지울 줄이 없습니다.",
            "No change." => "변경이 없습니다.",
            "The first line cannot be hidden." => "첫 줄은 숨길 수 없습니다.",
            _ => return n.to_string(),
        };
        self.t(n, ko).to_string()
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
        self.highlight_frame();
        // Step 100: the typing aids and the bracket highlight.
        self.assist_frame();
        // Step 96: the virtual space setting applies to every document.
        let vs = self.virtual_space;
        for t in &mut self.tabs {
            t.file.doc.virtual_space = vs;
            // Step 97: Split Lines splits at the view's width.
            t.file.doc.split_width = t.view.cols.max(1);
        }
        // Step 94: the find highlights, the styled markers and the bookmarks on screen (of
        // every view's active document).
        let search = self.search_marks();
        let c = self.colors;
        for pane in [views::PaneId::Main, views::PaneId::Second] {
            if pane == views::PaneId::Second && self.second.is_none() {
                continue;
            }
            let k = self.active_of(pane);
            let t = &mut self.tabs[k];
            let r = t.file.doc.text();
            let first = t.view.top_line.min(r.len_lines().saturating_sub(1));
            let last = (first + t.view.page_lines + 2).min(r.len_lines());
            let (a, b) = (
                r.line_to_char(first),
                if last < r.len_lines() {
                    r.line_to_char(last)
                } else {
                    r.len_chars()
                },
            );
            t.view.styled = t
                .file
                .doc
                .markers()
                .iter()
                .filter(|m| find::drawn(m) && m.end > a && m.start < b)
                .take(5000)
                .map(|m| (m.start, m.end, find::style_color(&c, m.kind)))
                .collect();
            let mut marks: Vec<usize> = t
                .file
                .doc
                .markers()
                .iter()
                .filter(|m| {
                    m.kind == jarak_core::document::BOOKMARK && m.start >= a && m.start <= b
                })
                .map(|m| r.char_to_line(m.start))
                .collect();
            marks.sort_unstable();
            marks.dedup();
            t.view.bookmarks = marks;
            // Step 101: the change history of the lines on screen.
            t.view.changes = t.file.doc.changed_lines_in(first, last);
            // Step 103: Show Symbol.
            if t.view.symbols != self.symbols {
                t.view.symbols = self.symbols.clone();
            }
        }
        if self.editing != Editing::Vim {
            let v = &mut self.tabs[i].view;
            v.block_caret = false;
            v.ime = true;
            v.marks = search;
            // Step 96: Esc goes back to one caret (egui would drop the focus with it).
            let d = &self.tabs[i].file.doc;
            let many = d.selection().len() > 1 || d.block().is_some();
            // Step 100: and closes the completion popup, the hint or a snippet's stops.
            let assisting = self.completion.is_some() || self.hint.is_some() || d.snippet_active();
            self.tabs[i].view.keep_escape = many || assisting;
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
        let mut all = marks.unwrap_or_default();
        all.extend(search);
        v.marks = all;
    }

    /// Runs a palette or menu entry.
    pub fn run_entry(&mut self, r: palette::Run) {
        match r {
            palette::Run::Act(a) => self.act(a),
            palette::Run::Editing(e) => self.set_editing(e),
            palette::Run::MenuBar => self.menu_bar = !self.menu_bar,
            palette::Run::Assets => self.open_assets(),
            palette::Run::Tab(m) => {
                let pane = self.focused_pane();
                let k = self
                    .order(pane)
                    .iter()
                    .position(|x| *x == self.active)
                    .unwrap_or(0);
                if m.needs_file() && self.tab().file.path.is_none() {
                    self.message = Some(
                        self.t(
                            "The document has no file yet.",
                            "문서에 아직 파일이 없습니다.",
                        )
                        .to_string(),
                    );
                } else {
                    self.tab_menu(pane, k, m);
                }
            }
            palette::Run::Recent(k) => self.open_recent(k),
            palette::Run::RecentAll => self.open_all_recent(),
            palette::Run::RecentClear => self.clear_recent(),
            palette::Run::Orientation => self.toggle_orientation(),
            palette::Run::SyncV => self.toggle_sync(true),
            palette::Run::SyncH => self.toggle_sync(false),
            palette::Run::Side(s) => self.toggle_side(s),
            palette::Run::Documents => {
                self.docs_dialog = Some(workspace::DocsDialog::default());
            }
            palette::Run::RestoreSession => self.toggle_restore(),
            palette::Run::FindCmd(k) => match k {
                0 => self.open_find(find::Page::Mark),
                1 => self.mark_all(),
                2 => self.clear_marks(),
                3 => self.copy_marked(),
                _ => self.count_matches(),
            },
            palette::Run::Style(cmd) => self.style(cmd),
            palette::Run::Multi(k) => self.multi(k),
            palette::Run::Edit(k) => self.edit_tool(k),
            palette::Run::Language(k) => self.set_language(k),
            palette::Run::Udl(k) => self.udl_tool(k),
            palette::Run::Outline(0) => self.function_list = !self.function_list,
            palette::Run::Outline(k) => self.symbol_step(k == 2),
            palette::Run::Assist(k) => self.assist_tool(k),
            palette::Run::Nav(0) => self.doc_map = !self.doc_map,
            palette::Run::Nav(k) => self.bookmark_tool(k - 1),
            palette::Run::Macro(k) => self.macro_tool(k),
            palette::Run::PlayMacro(i) => self.play_saved(usize::from(i)),
            palette::Run::RunCommand(i) => self.confirm_command(usize::from(i)),
            palette::Run::Reinterpret(i) => self.reinterpret(usize::from(i), false),
            palette::Run::ConvertTo(i) => self.convert_to(usize::from(i), false),
            palette::Run::Eol(k) => self.convert_eol(k),
            palette::Run::NewEncoding(k) => self.new_encoding_tool(k),
            palette::Run::Symbols(k) => self.symbols_tool(k),
            palette::Run::Binary(k) => self.binary_tool(k),
            palette::Run::Disk(k) => self.disk_tool(k),
            palette::Run::Export(k) => self.export_tool(k),
            palette::Run::InFiles(k) => match k {
                0 => self.find_all_docs(false),
                1 => self.find_all_docs(true),
                2 => self.replace_all_docs(),
                3 => self.undo_replace_in_files(),
                4 => self.copy_results(false),
                5 => self.copy_results(true),
                6 => {
                    self.find.page = find::Page::Files;
                    self.find_in_files();
                }
                _ => {
                    self.find.page = find::Page::Files;
                    self.replace_in_files();
                }
            },
            palette::Run::SessionSave => self.save_session_as(),
            palette::Run::SessionLoad => self.load_session_from(),
        }
    }

    /// Turns restoring the session at start on or off (kept in the settings file).
    pub fn toggle_restore(&mut self) {
        let on = !self.session.as_ref().is_some_and(|s| s.restore);
        if let Some(s) = self.session.as_mut() {
            s.restore = on;
        }
        if let Some(p) = &self.settings {
            let _ = write_setting(p, "restore_session", if on { "true" } else { "false" });
        }
        self.message = Some(
            if on {
                self.t(
                    "The session is restored at start.",
                    "시작할 때 세션을 복원합니다.",
                )
            } else {
                self.t(
                    "The session is not restored at start.",
                    "시작할 때 세션을 복원하지 않습니다.",
                )
            }
            .to_string(),
        );
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

    /// On Windows an Alt+letter chord also arrives as the letter's text (in the same frame, or
    /// the next): that text is dropped before any widget sees it, so a shortcut (Alt+Z, the find
    /// panel's mnemonics) never types its letter. Only letters and digits (what Windows turns into
    /// text) and only within 0.1 s; AltGr (Ctrl+Alt) still types, and so do Alt+numpad codes
    /// (their character comes with Alt's release).
    fn drop_alt_text(&mut self, ctx: &egui::Context) {
        ctx.input_mut(|i| {
            let chord = i.events.iter().any(|e| {
                matches!(e, egui::Event::Key { key, pressed: true, modifiers: m, .. }
                if m.alt && !m.ctrl && !m.command && {
                    let n = key.name();
                    n.len() == 1 && n.chars().all(|c| c.is_ascii_alphanumeric())
                })
            });
            if chord {
                self.alt_text = Some(i.time);
            }
            if let Some(t) = self.alt_text {
                if i.time - t > 0.1 {
                    self.alt_text = None;
                } else if let Some(k) = i
                    .events
                    .iter()
                    .position(|e| matches!(e, egui::Event::Text(_)))
                {
                    i.events.remove(k);
                    self.alt_text = None;
                }
            }
        });
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
        let mut entries = palette::entries();
        // Step 93: the recent files, as commands of their own.
        for (k, p) in self.recent.iter().take(10).enumerate() {
            entries.push(palette::Entry {
                id: "file.recent",
                en: std::borrow::Cow::Owned(format!("Open recent: {}", p.display())),
                ko: std::borrow::Cow::Owned(format!("최근 파일 열기: {}", p.display())),
                keys: None,
                run: palette::Run::Recent(k),
                menu: palette::Menu::File,
            });
        }
        entries.extend(self.language_entries());
        entries.extend(self.macro_entries());
        entries.extend(self.encoding_entries());
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
                            lang.pick(&e.en, &e.ko),
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
            ctx.memory_mut(|m| m.request_focus(view::view_id_for(u8::from(self.second_focused()))));
            return;
        }
        if resp.should_close() {
            ctx.memory_mut(|m| m.request_focus(view::view_id_for(u8::from(self.second_focused()))));
            return;
        }
        self.palette = Some(st);
    }

    /// The menu bar (hidden by default; Alt or the palette shows it).
    fn menu_bar_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        // Step 105: the menu bar always, or only while Alt is pressed (the setting).
        if !self.menu_bar && !self.prefs.menu_always {
            return;
        }
        let mut entries = palette::entries();
        entries.extend(self.language_entries());
        entries.extend(self.macro_entries());
        entries.extend(self.encoding_entries());
        let lang = self.lang;
        let mut run = None;
        let mut show = None;
        let recent: Vec<String> = self
            .recent
            .iter()
            .take(10)
            .map(|p| p.display().to_string())
            .collect();
        let docs: Vec<String> = self
            .tabs
            .iter()
            .map(|t| {
                format!(
                    "{}{}",
                    if t.file.doc.is_dirty() { "• " } else { "" },
                    t.file.title()
                )
            })
            .collect();
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
                                let resp = kim_ui::menu_item(ui, lang.pick(&e.en, &e.ko), false);
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
                            if menu == palette::Menu::File && !recent.is_empty() {
                                ui.separator();
                                for (k, name) in recent.iter().enumerate() {
                                    if kim_ui::menu_item(ui, name, false).clicked() {
                                        run = Some(palette::Run::Recent(k));
                                        ui.close();
                                    }
                                }
                            }
                            if menu == palette::Menu::Window {
                                ui.separator();
                                for (i, name) in docs.iter().enumerate() {
                                    if kim_ui::menu_item(ui, name, false).clicked() {
                                        show = Some(i);
                                        ui.close();
                                    }
                                }
                            }
                        });
                    }
                });
            });
        if let Some(r) = run {
            self.run_entry(r);
        }
        if let Some(i) = show {
            self.show_doc(i);
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
            second: None,
            main_active: 0,
            recent: Vec::new(),
            closed: Vec::new(),
            session: None,
            session_saved: false,
            workspace: None,
            side: None,
            docs_dialog: None,
            find: find::FindPanel::default(),
            find_history: Vec::new(),
            replace_history: Vec::new(),
            incremental: None,
            central: Rect::NOTHING,
            colors: Colors::of(kim_design::Theme::Dark),
            alt_text: None,
            job: None,
            results: None,
            preview: None,
            journal_root: None,
            last_batch: None,
            column_editor: None,
            column_editor_open: false,
            virtual_space: false,
            begin_select: None,
            copy_line: true,
            hash_dialog: None,
            user_langs: Vec::new(),
            languages_dir: None,
            function_list: false,
            function_filter: String::new(),
            goto: None,
            assist: assist::AssistSettings::default(),
            completion: None,
            completion_miss: None,
            hint: None,
            apis: Vec::new(),
            snippets: jarak_core::assist::snippets_with(&[]),
            assist_dir: None,
            doc_map: false,
            nav: Vec::new(),
            nav_at: 0,
            nav_last: None,
            nav_moving: false,
            quick: None,
            summary_open: false,
            recording: None,
            last_macro: None,
            playing: false,
            user_cmds: jarak_core::macros::UserCommands::default(),
            user_cmds_broken: false,
            macros_path: None,
            runner: Arc::new(macros::NoRunner),
            macro_dialog: None,
            search_engine: DEFAULT_SEARCH.into(),
            enc_question: None,
            symbols: encoding::Symbols::default(),
            new_format: file::Format::utf8(),
            watch: watch::Watch::default(),
            watch_auto: false,
            detect_changes: true,
            reload_unmodified: false,
            recovery: None,
            recovery_offer: None,
            publish_question: None,
            prefs: prefs::Prefs::default(),
            settings_win: None,
            lang_fixed: true,
            theme_now: None,
            applied_theme: None,
            applied_font: None,
            settings_stamp: None,
            prefs_focused: false,
            audit_settings: false,
            bindings: shortcuts::Bindings::default(),
        };
        app.main_active = app.tabs[0].id;
        app.open_args(args);
        app
    }

    /// Sets up a UI audit's view (ADR 0099): `editor`, `editor-empty`, `editor-selection`,
    /// `editor-wrap`, `question`, `preview`, `split`, `paste-image` (pastes a sample bitmap and
    /// saves: the UI audit runs on a copy), `images` (sample pictures next to the copy), `vim`,
    /// `vim-insert`, `vim-command`, `palette`, `menu-bar`, `assets` (Step 92), `tabs`, `views`,
    /// `side`, `documents` (Step 93), `find`, `replace`, `mark`, `incremental` (Step 94), `files`,
    /// `results`, `replace-preview` (Step 95), `multi`, `column`, `column-editor` (Step 96), `hash`,
    /// `palette-lines` (Step 97), `syntax`, `palette-language` (Step 98). Only used while
    /// `KIM_UI_INSPECT` is set.
    pub fn prepare_audit(&mut self, screen: &str) {
        self.audit_screen = screen.to_string();
        // Step 105: `settings-<section>` photographs the Settings window at that section.
        if let Some(sec) = screen
            .strip_prefix("settings-")
            .and_then(jarak_core::settings::Section::from_id)
        {
            self.open_settings(Some(sec));
            self.audit_settings = true;
            return;
        }
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
            // Step 97: the digest dialog over a selected word; the palette's sort commands.
            "hash" => {
                let d = &mut self.tab_mut().file.doc;
                d.select_word_at(2);
                self.edit_tool(16);
            }
            // Step 97 (the e2e's): registry commands on the document, then saved.
            "lineops-session" => {
                use jarak_core::command::Command as C;
                let page = self.tab().view.page_lines;
                for c in [
                    C::RemoveDuplicateLines,
                    C::SortIntAsc,
                    C::SelectAll,
                    C::ProperCase,
                    C::DocEnd,
                ] {
                    self.tab_mut().file.doc.execute(c, page);
                }
                let i = self.active;
                self.save(i);
            }
            // Step 98: a Rust sample (written next to the audit's copy) and the Language menu.
            // Step 99: the same sample with the Function List open and the `if` block folded, and
            // the Go to dialog.
            "syntax" | "functions" | "goto" | "completion" | "hint" | "docmap" | "quick-open"
            | "summary" | "run" | "macros" | "symbols" | "encoding-question" | "disk-question"
            | "recovery" | "publish-question" => {
                let dir = self
                    .tab()
                    .file
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                if let Some(dir) = dir {
                    let p = dir.join("sample.rs");
                    let _ = std::fs::write(
                        &p,
                        "/// 한글 문서 주석
use std::fmt;

#[derive(Debug)]
pub struct Point { x: f64, y: f64 }

fn main() {
    let p = Point { x: 1.5, y: -2.0 }; // 주석
    let s = \"text \\\"quoted\\\"\";
    if p.x > 0.0 && s.len() != 0 {
        println!(\"{p:?} {}\", 0x1F);
    }
    /* block
       comment */
}
",
                    );
                    self.open_args(&[p.display().to_string()]);
                    if screen == "functions" {
                        self.function_list = true;
                        // The `if` line of `main` (line 10).
                        self.tab_mut().file.doc.set_folded(9, true);
                    } else if screen == "docmap" {
                        // Step 101: the map with a bookmark and a changed line.
                        self.doc_map = true;
                        let d = &mut self.tab_mut().file.doc;
                        d.bookmark_lines([3]);
                        let p = d.text().line_to_char(6);
                        d.set_selection(&jarak_core::selection::Selection::point(p));
                        d.type_text("// 고침\n");
                    } else if screen == "symbols" {
                        // Step 103: every symbol over the Rust sample, an edge line at 40.
                        self.symbols = encoding::Symbols {
                            spaces: true,
                            eol: true,
                            controls: true,
                            indent_guides: true,
                            wrap_symbol: true,
                            edges: vec![40],
                        };
                    } else if screen == "disk-question" {
                        // Step 104: the sample changed here and by another program.
                        self.tab_mut().file.doc.type_text(
                            "// 고침
",
                        );
                        let id = self.tab().id;
                        self.watch
                            .questions
                            .push_back(watch::DiskQuestion::Conflict(id));
                    } else if screen == "publish-question" {
                        // Step 106: publishing into a folder that holds two of its files.
                        let id = self.tab().id;
                        self.publish_question = Some(export_ui::PublishQuestion {
                            tab: id,
                            folder: dir.join("site"),
                            existing: 2,
                        });
                    } else if screen == "recovery" {
                        // Step 104: a crashed window's two documents offered back.
                        let entry = |id: u64, path: Option<PathBuf>, title: &str, chars| {
                            jarak_core::recovery::Entry {
                                id,
                                path,
                                title: title.into(),
                                text: format!("doc-{id}.txt"),
                                encoding: "utf-8".into(),
                                bom: false,
                                eol: "crlf".into(),
                                caret: 0,
                                chars,
                                saved_at: 1_795_000_000,
                                disk_len: None,
                                disk_modified_ms: None,
                            }
                        };
                        self.recovery_offer = Some(recover::Offer {
                            orphans: vec![jarak_core::recovery::Orphan {
                                dir: dir.join("w-1-1"),
                                manifest: jarak_core::recovery::Manifest {
                                    schema: jarak_core::recovery::SCHEMA.into(),
                                    schema_version: jarak_core::recovery::SCHEMA_VERSION,
                                    owner: jarak_core::recovery::Owner { pid: 1, started: 1 },
                                    entries: vec![
                                        entry(1, Some(p), "sample.rs", 412),
                                        entry(2, None, "Untitled", 38),
                                    ],
                                },
                            }],
                            confirm_discard: false,
                        });
                    } else if screen == "encoding-question" {
                        // The sample's Korean comment cannot be written in Windows-1252.
                        if let Some(i) = encoding::targets()
                            .iter()
                            .position(|t| t.en.contains("Windows-1252"))
                        {
                            self.convert_to(i, false);
                        }
                    } else if screen == "run" {
                        self.open_run();
                        if let Some(macros::Dialog::Run { command, .. }) =
                            self.macro_dialog.as_mut()
                        {
                            *command = "notepad.exe \"$(FULL_CURRENT_PATH)\"".into();
                        }
                    } else if screen == "macros" {
                        self.user_cmds.macros.push(jarak_core::macros::Macro {
                            name: "Tidy".into(),
                            keys: Some("Ctrl+Shift+1".into()),
                            steps: vec![jarak_core::macros::Step::Cmd(
                                "blank.trim-trailing".into(),
                            )],
                        });
                        self.macro_tool(2);
                    } else if screen == "quick-open" {
                        self.open_quick();
                    } else if screen == "summary" {
                        self.summary_open = true;
                    } else if screen == "completion" {
                        // Step 100: after `Po` of `Point` on the `let p` line.
                        let d = &mut self.tab_mut().file.doc;
                        let s = d.text().to_string();
                        let at = s
                            .match_indices("Point {")
                            .nth(1)
                            .map_or(0, |(b, _)| d.text().byte_to_char(b));
                        d.set_selection(&jarak_core::selection::Selection::point(at + 2));
                        // As while typing (a manual popup with one entry would insert it).
                        self.open_completion(assist::Trigger::Auto);
                    } else if screen == "hint" {
                        let p = dir.join("sample.c");
                        let _ = std::fs::write(
                            &p,
                            "#include <stdio.h>\n\nint main(void) {\n    printf(\"%d\\n\", 42);\n    return 0;\n}\n",
                        );
                        self.open_args(&[p.display().to_string()]);
                        let d = &mut self.tab_mut().file.doc;
                        let at = d.text().to_string().find("42").unwrap_or(0);
                        let at = d.text().byte_to_char(at);
                        d.set_selection(&jarak_core::selection::Selection::point(at));
                        self.open_hint(true);
                    } else if screen == "goto" {
                        self.goto = Some(outline::GotoDialog {
                            text: "12".into(),
                            focus: true,
                            ..Default::default()
                        });
                    }
                }
            }
            "palette-language" => {
                self.palette = Some(palette::PaletteState {
                    query: "language".into(),
                    selected: 0,
                    focus: true,
                });
            }
            "palette-lines" => {
                self.palette = Some(palette::PaletteState {
                    query: "sort lines".into(),
                    selected: 0,
                    focus: true,
                });
            }
            // Step 96: every "Step" selected (the status bar counts the carets), a column block
            // over the Korean lines (display columns 4 to 12), the Column Editor with numbers.
            "multi" | "column" | "column-editor" => {
                let d = &mut self.tab_mut().file.doc;
                match screen {
                    "multi" => {
                        let at = d
                            .text()
                            .to_string()
                            .find("Step")
                            .map_or(0, |b| d.text().byte_to_char(b));
                        d.set_selection(&jarak_core::selection::Selection::point(at));
                        d.select_all_occurrences(None);
                    }
                    "column" => {
                        d.virtual_space = true;
                        d.select_block(jarak_core::document::multi::Block {
                            anchor: (2, 4),
                            head: (4, 12),
                        });
                        self.virtual_space = true;
                    }
                    _ => {
                        self.open_column_editor();
                        if let Some(e) = self.column_editor.as_mut() {
                            e.numbers = true;
                            e.leading = jarak_core::document::multi::Leading::Zeros;
                        }
                    }
                }
            }
            // Step 95: the Files page, its results grouped by file (small files written next to
            // the audit's copy, searched at once), the Replace in Files preview.
            "files" | "results" | "replace-preview" => {
                let dir = self
                    .tab()
                    .file
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                if let Some(dir) = dir {
                    let _ = std::fs::create_dir_all(dir.join("notes"));
                    for (n, t) in [
                        (
                            "notes/todo.md",
                            "# cat\n\nfeed the cat\nconcat is not a cat word\n",
                        ),
                        ("notes/log.txt", "the cat sat\r\nno match here\r\n"),
                    ] {
                        let _ = std::fs::write(dir.join(n), t);
                    }
                    self.open_find(find::Page::Files);
                    self.find.find = "cat".into();
                    self.find.replace = "dog".into();
                    self.find.whole_word = true;
                    self.find.dir = dir.display().to_string();
                    self.find.filters = "*.md *.txt".into();
                    match screen {
                        "results" => self.find_in_files_now(),
                        "replace-preview" => self.plan_in_files_now(),
                        _ => {}
                    }
                }
            }
            // Step 94: the find panel's pages (highlight all, regular expressions), Mark All
            // with bookmarks and two token styles, the incremental search bar.
            "find" | "replace" | "mark" | "incremental" => {
                self.find.find = "Markdown".into();
                self.find.status = None;
                match screen {
                    "find" => {
                        self.open_find(find::Page::Find);
                        self.find.find = "Markdown".into();
                        self.find_next(false);
                    }
                    "replace" => {
                        self.open_find(find::Page::Replace);
                        self.find.mode = jarak_core::search::Mode::Regex;
                        self.find.find = r"\b(\w+)down\b".into();
                        self.find.replace = r"$1up".into();
                        self.count_matches();
                    }
                    "mark" => {
                        self.open_find(find::Page::Mark);
                        self.find.find = "Markdown".into();
                        self.find.bookmark = true;
                        self.mark_all();
                        // Style 1 on a token used three times ("Step").
                        let d = &mut self.tab_mut().file.doc;
                        let a = d
                            .text()
                            .to_string()
                            .find("Step")
                            .map_or(0, |b| d.text().byte_to_char(b));
                        d.set_selection(&jarak_core::selection::Selection::single(
                            jarak_core::selection::Range::new(a, a),
                        ));
                        self.style(find::StyleCmd::All(1));
                    }
                    _ => {
                        self.open_incremental();
                        if let Some(b) = self.incremental.as_mut() {
                            b.text = "Mark".into();
                        }
                        self.incremental_search_now();
                    }
                }
            }
            // Step 93: three documents in tabs, two views with a clone, the side panel, the
            // Documents dialog (small files written next to the audit's copy).
            "tabs" | "views" | "side" | "documents" => {
                let dir = self
                    .tab()
                    .file
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                if let Some(dir) = dir {
                    let mut names = Vec::new();
                    for (n, t) in [
                        ("todo.txt", "- write the docs\n- test the views\n"),
                        ("notes-2026.md", "# Notes\n\nA second file.\n"),
                    ] {
                        let p = dir.join(n);
                        let _ = std::fs::write(&p, t);
                        names.push(p.display().to_string());
                    }
                    self.open_args(&names);
                    let first = self.order(views::PaneId::Main)[0];
                    self.activate(first, views::PaneId::Main);
                    match screen {
                        "views" => {
                            self.clone_to_other();
                            self.focus_pane(views::PaneId::Main);
                        }
                        "side" => self.open_workspace(&dir),
                        "documents" => self.docs_dialog = Some(workspace::DocsDialog::default()),
                        _ => {}
                    }
                }
            }
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
        if t.path.is_dir() {
            // Step 93: a folder is shown as the workspace.
            self.open_workspace(&t.path);
            return;
        }
        if let Some(i) = self
            .tabs
            .iter()
            .position(|tab| tab.file.path.as_deref() == Some(t.path.as_path()))
        {
            self.show_doc(i);
        } else {
            match FileDocument::open(&t.path, self.fallback, file::DEFAULT_SIZE_LIMIT) {
                Ok(f) => {
                    let mut tab = Tab::new(f);
                    jarak_core::session::Session::remember(&mut self.recent, &t.path);
                    if self.tab().is_blank() && self.tabs.len() == 1 && self.second.is_none() {
                        self.main_active = tab.id;
                        self.tabs[0] = tab;
                    } else if self.second_focused() {
                        tab.pane = views::Pane::Second;
                        let id = tab.id;
                        self.tabs.push(tab);
                        if let Some(s) = self.second.as_mut() {
                            s.order.push(id);
                            s.active = id;
                        }
                        self.active = self.tabs.len() - 1;
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
        format!("{dirty}{} — {TITLE}", t.title())
    }

    // ---- actions -----------------------------------------------------------------------------

    /// Runs a window action (a key, or a test).
    pub fn act(&mut self, a: Action) {
        // Step 102: a macro being recorded takes the command.
        self.record_action(a);
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
                // Everything but moving the caret and selecting edits (Step 97 adds many).
                // Step 101: bookmarks, the change history and hidden lines are not edits either.
                let edits = c.edits();
                if edits && read_only {
                    self.refuse_read_only();
                    return;
                }
                let t = self.tab_mut();
                t.file.doc.execute(c, page);
                t.view.preedit.clear();
                t.view.after_edit();
                // A text operation that could not run says why.
                if let Some(n) = t.file.doc.take_notice() {
                    self.message = Some(self.notice_text(&n));
                }
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
                let mut t = Tab::new(FileDocument::untitled());
                // Step 103: new documents in the encoding of the settings; Step 105: and its
                // line ending.
                t.file.format = self.new_format;
                t.file.doc.eol = self.prefs.new_eol;
                if self.second_focused() {
                    t.pane = views::Pane::Second;
                    if let Some(s) = self.second.as_mut() {
                        s.order.push(t.id);
                        s.active = t.id;
                    }
                }
                self.tabs.push(t);
                self.active = self.tabs.len() - 1;
            }
            Action::FocusOther => {
                let other = if self.second_focused() {
                    views::PaneId::Main
                } else {
                    views::PaneId::Second
                };
                self.focus_pane(other);
            }
            Action::GoToTab(n) => self.go_to_tab(usize::from(n)),
            Action::MoveTabLeft => self.move_tab(-1),
            Action::MoveTabRight => self.move_tab(1),
            Action::ReopenClosed => self.reopen_closed(),
            Action::Find => self.open_find(find::Page::Find),
            Action::Replace => self.open_find(find::Page::Replace),
            Action::FindNext => self.find_next(false),
            Action::FindPrevious => self.find_next(true),
            Action::SelectFindNext => self.find_word(false, false),
            Action::SelectFindPrevious => self.find_word(true, false),
            Action::VolatileNext => self.find_word(false, true),
            Action::VolatilePrevious => self.find_word(true, true),
            Action::Incremental => self.open_incremental(),
            Action::Fold(level, on) => self.fold_command(level, on),
            Action::GoTo => {
                self.goto = Some(outline::GotoDialog {
                    focus: true,
                    ..Default::default()
                });
            }
            Action::MatchBrace => self.match_brace(false),
            Action::Navigate(back) => self.navigate(back),
            Action::Macro(0) => self.toggle_recording(),
            Action::Macro(_) => self.play_recorded(),
            Action::RunDialog => self.open_run(),
            Action::Settings => self.open_settings(None),
            Action::QuickOpen => self.open_quick(),
            Action::Complete(k) => match k {
                0 => self.open_completion(assist::Trigger::All),
                1 => self.open_completion(assist::Trigger::Words),
                2 => self.open_completion(assist::Trigger::Paths),
                _ => self.open_hint(true),
            },
            Action::SelectBraces => self.match_brace(true),
            Action::FindInFiles => self.open_find(find::Page::Files),
            Action::NextResult => self.result_step(false),
            Action::PreviousResult => self.result_step(true),
            Action::ToggleResults => {
                let r = self.results.get_or_insert_with(results::Results::default);
                r.open = !r.open;
            }
            Action::AddNextOccurrence => self.multi(0),
            Action::SelectAllOccurrences => self.multi(1),
            Action::SingleCaret => {
                if self.tab_mut().file.doc.single_caret() {
                    self.tab_mut().view.after_edit();
                }
            }
            Action::BlockUp => self.block(-1, 0),
            Action::BlockDown => self.block(1, 0),
            Action::BlockLeft => self.block(0, -1),
            Action::BlockRight => self.block(0, 1),
            Action::ColumnEditor => self.open_column_editor(),
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
            // Nothing to copy (the copy-line setting off): the clipboard stays as it is.
            Action::Copy => {
                let s = self.copy_text();
                if !s.is_empty() {
                    self.clipboard.set_text(&s);
                }
            }
            Action::Cut => {
                let s = self.cut();
                if !s.is_empty() {
                    self.clipboard.set_text(&s);
                }
            }
            Action::Paste => self.paste_from_clipboard(),
            Action::NextDocument => self.cycle_tab(1),
            Action::PreviousDocument => self.cycle_tab(-1),
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
            // Step 100: with the typing aids (closers, tags, a closing bracket's dedent).
            t.file.doc.type_assisted(s);
        }
        t.view.after_edit();
        self.after_typed(s);
        // Step 102: a macro being recorded takes the typing.
        self.record_typed(s);
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
            match jarak_core::assets::write_new_in(
                &doc_path,
                &self.prefs.image_folder,
                ext,
                date,
                bytes,
            ) {
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
        let eol = t.file.doc.eol.as_str();
        let s = clipboard::with_line_endings(s, eol);
        // Step 96: as many lines as carets: each caret gets its own line.
        let n = t.file.doc.selection().len();
        let mut lines: Vec<String> = s.split(eol).map(str::to_string).collect();
        if lines.len() > 1 && lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        if n > 1 && lines.len() == n {
            t.file.doc.paste_each(&lines);
        } else {
            t.file.doc.paste(&s);
        }
        t.view.after_edit();
    }

    /// The text Copy puts on the clipboard (the current line when nothing is selected, as
    /// Notepad++ does with its setting on).
    pub fn copy_text(&self) -> String {
        let d = &self.tab().file.doc;
        let s = d.selected_text();
        if s.is_empty() && !self.copy_line {
            return String::new();
        }
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
        // Step 104: a file another program changed is never overwritten unasked.
        if !self.safe_to_save(i) {
            return false;
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
        if let Some(p) = self.tabs[i].file.path.clone() {
            jarak_core::session::Session::remember(&mut self.recent, &p);
            self.closed.push(p);
        }
        // The view shows the document next to the closed one.
        let pane = self.focused_pane();
        let order = self.order(pane);
        let next = order.iter().position(|x| *x == i).and_then(|k| {
            if k > 0 {
                order.get(k - 1).copied()
            } else {
                order.get(k + 1).copied()
            }
        });
        let next_id = next.map(|n| self.tabs[n].id);
        let active_id = self.tabs[self.active].id;
        let closed_id = self.tabs[i].id;
        self.tabs.remove(i);
        let keep = if active_id == closed_id {
            next_id
        } else {
            Some(active_id)
        };
        self.active = keep
            .and_then(|id| self.index_of(id))
            .unwrap_or(0)
            .min(self.tabs.len().saturating_sub(1));
        if self.tabs.is_empty() {
            let t = Tab::new(FileDocument::untitled());
            self.main_active = t.id;
            self.tabs.push(t);
            self.active = 0;
        }
        self.normalize();
    }

    /// The first document the window must ask about before closing (untitled ones are kept by
    /// the session when it is restored).
    fn first_unsaved(&self) -> Option<usize> {
        let keep = self.keeps_untitled();
        self.tabs
            .iter()
            .position(|t| t.file.doc.is_dirty() && !(keep && t.file.path.is_none()))
    }

    /// The window was asked to close: `true` when it may (nothing unsaved, or every question
    /// answered); otherwise the first unsaved document is shown with the question.
    pub fn request_quit(&mut self) -> bool {
        if self.quitting {
            return true;
        }
        if let Some(i) = self.first_unsaved() {
            self.show_doc(i);
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
        if let Pending::Recycle(id) = &p {
            if save {
                self.recycle_now(*id);
            }
            return;
        }
        if save && !self.save(i) {
            // Not saved (cancelled dialog, an error): nothing is closed.
            return;
        }
        match p {
            Pending::CopyAssets(_) | Pending::Recycle(_) => {}
            Pending::Close(i) => self.close_now(i),
            Pending::CloseQueue(i, rest) => {
                self.close_now(i);
                self.close_many(rest);
            }
            Pending::Quit => {
                if !save {
                    // Don't save: forget this document's changes for the quit.
                    self.close_now(i);
                }
                if self.first_unsaved().is_some() {
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
        // Step 105: the settings (theme, editor font, new documents, the file changed outside).
        if self.settings.is_some() {
            self.prefs_frame(&ctx);
        }
        // Step 104: files changed by other programs, and the recovery snapshots (not in a UI
        // audit, unless it keeps a session as the program does: `KIM_UI_SESSION=1`).
        if !kim_ui::inspect::active() || self.session.is_some() {
            self.watch_frame(&ctx);
            self.recovery_frame(&ctx);
        }
        // Closing the window asks about unsaved changes first.
        // (A UI audit's run closes itself without asking: it holds no user data.)
        if ctx.input(|i| i.viewport().close_requested()) && !kim_ui::inspect::active() {
            if self.request_quit() {
                // Step 93: the session is written as the window closes.
                if !self.session_saved {
                    self.save_session();
                    self.session_saved = true;
                }
                // Step 104: a normal exit keeps no recovery snapshots.
                self.recovery_exit();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if self.quitting {
            if !self.session_saved {
                self.save_session();
                self.session_saved = true;
            }
            self.recovery_exit();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let title = self.window_title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let c = Colors::get(&ctx);
        self.colors = c;
        self.drop_alt_text(&ctx);
        self.alt_toggle(&ctx);
        self.vim_frame();
        self.outline_frame(&ctx);
        self.nav_frame();
        self.menu_bar_ui(ui, &c);
        if self.prefs.status_bar {
            self.status_bar(ui, &c);
        }
        self.poll_job(&ctx);
        self.results_ui(ui, &c);
        self.incremental_ui(ui, &c);
        self.side_ui(ui, &c);
        self.function_list_ui(ui, &c);
        self.doc_map_ui(ui, &c);
        // The keyboard focus belongs to the focused view's text (after a move, a clone, F8).
        if self.second.is_some() {
            let (mine, other) = if self.second_focused() {
                (1, 0)
            } else {
                (0, 1)
            };
            if ctx.memory(|m| m.focused()) == Some(view::view_id_for(other)) {
                ctx.memory_mut(|m| m.request_focus(view::view_id_for(mine)));
            }
        }
        let drawn = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(c.bg_base))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                self.central = rect;
                self.draw_views(ui, rect, &c)
            })
            .inner;
        let (focused, link, ctrl_click) = (drawn.focused, drawn.link, drawn.ctrl_click);
        if let Some(line) = drawn.fold_click {
            self.toggle_fold(line);
        }
        let opened_link = link.open.is_some();
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
            // Ctrl+click adds a caret (Step 96), unless it showed an image or opened a link.
            if self.reveal_reference_at(pos) || opened_link {
                self.tab_mut()
                    .file
                    .doc
                    .set_selection(&jarak_core::selection::Selection::point(pos));
            }
        }
        let dialog_open = self.palette.is_some()
            || self.assets_view.is_some()
            || self.docs_dialog.is_some()
            || self.preview.is_some()
            || self.column_editor_open
            || self.hash_dialog.is_some()
            || self.goto.is_some()
            || self.quick.is_some()
            || self.summary_open
            || self.macro_dialog.is_some()
            || self.enc_question.is_some()
            || !self.watch.questions.is_empty()
            || self.recovery_offer.is_some();
        if self.pending.is_none() && !dialog_open {
            let preview_only = self.tab().mode == Mode::Preview;
            let salt = u8::from(self.second_focused());
            if !preview_only && !focused && !ctx.memory(|m| m.focused().is_some()) {
                ctx.memory_mut(|m| m.request_focus(view::view_id_for(salt)));
            }
            if focused || preview_only {
                self.keyboard(&ctx);
            } else if self.find.open && ctx.memory(|m| m.focused().is_some()) {
                // Step 95 (found on the real desktop): a box of the find panel has the keyboard;
                // the window's own shortcuts still work there.
                self.panel_keys(&ctx);
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
        self.docs_dialog_ui(&ctx, &c);
        let central = self.central;
        self.find_ui(&ctx, &c, central);
        self.preview_ui(&ctx, &c);
        self.column_editor_ui(&ctx, &c);
        self.hash_dialog_ui(&ctx, &c);
        self.goto_ui(&ctx, &c);
        self.assist_ui(&ctx, &c);
        self.quick_ui(&ctx, &c);
        self.summary_ui(&ctx, &c);
        self.macro_dialog_ui(&ctx, &c);
        self.enc_question_ui(&ctx, &c);
        self.disk_question_ui(&ctx, &c);
        self.recovery_offer_ui(&ctx, &c);
        self.publish_question_ui(&ctx, &c);
        // Step 105: the Settings window (its audit is written by that window, ADR 0100).
        self.settings_window(&ctx);
        if kim_ui::inspect::active() && !self.audit_settings {
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

    /// The window's shortcuts while a box of the find panel has the keyboard: the palette, files,
    /// tabs, the find pages and the results; the box keeps text editing, the clipboard, undo and
    /// F3 (which the panel handles itself).
    /// Step 96: the multi-selection commands — 0 add the next occurrence, 1 select all (both
    /// automatic), 2–5 select all and 6–9 add the next with Notepad++'s four variants (ignore
    /// case and whole word, match case only, whole word only, match case and whole word), 10
    /// skip this occurrence, 11 Begin/End Select, 12 in column mode, 13 virtual space on/off.
    pub fn multi(&mut self, k: u8) {
        use jarak_core::document::multi::Occurrences;
        let variant = |v: u8| Occurrences {
            match_case: v % 2 == 1,
            whole_word: v >= 2,
        };
        let msg = match k {
            0 | 6..=9 | 10 => {
                let o = (6..=9).contains(&k).then(|| variant(k - 6));
                let t = self.tab_mut();
                let ok = t.file.doc.select_next_occurrence(o, k == 10);
                t.view.after_edit();
                (!ok).then(|| {
                    self.t("No other occurrence.", "다른 곳이 없습니다.")
                        .to_string()
                })
            }
            1..=5 => {
                let o = (2..=5).contains(&k).then(|| variant(k - 2));
                let t = self.tab_mut();
                let n = t.file.doc.select_all_occurrences(o);
                t.view.after_edit();
                Some(format!(
                    "{} {n}",
                    self.t("Selected occurrences:", "선택한 곳:")
                ))
            }
            11 | 12 => Some(self.begin_end_select(k == 12)),
            _ => {
                self.virtual_space = !self.virtual_space;
                if let Some(p) = &self.settings {
                    let _ = write_setting(
                        p,
                        "virtual_space",
                        if self.virtual_space { "true" } else { "false" },
                    );
                }
                Some(if self.virtual_space {
                    self.t("Virtual space: on", "가상 공간: 켬").to_string()
                } else {
                    self.t("Virtual space: off", "가상 공간: 끔").to_string()
                })
            }
        };
        if msg.is_some() {
            self.message = msg;
        }
    }

    /// Alt+Shift+arrows: the column selection grows or shrinks.
    fn block(&mut self, lines: isize, cols: isize) {
        let t = self.tab_mut();
        t.file.doc.extend_block(lines, cols);
        t.view.after_edit();
    }

    /// Begin/End Select: the first time marks the start, the second selects to the caret (in
    /// column mode as a column selection).
    fn begin_end_select(&mut self, column: bool) -> String {
        let id = self.tab().id;
        let head = self.tab().file.doc.selection().primary().head;
        match self.begin_select.take() {
            Some((tab, mark)) if tab == id => {
                let d = &mut self.tab_mut().file.doc;
                if column {
                    let (a, h) = (d.line_col_display(mark), d.line_col_display(head));
                    d.select_block(jarak_core::document::multi::Block { anchor: a, head: h });
                } else {
                    d.set_selection(&jarak_core::selection::Selection::single(
                        jarak_core::selection::Range::new(mark, head),
                    ));
                }
                self.tab_mut().view.after_edit();
                self.t("Selected.", "선택했습니다.").to_string()
            }
            _ => {
                self.begin_select = Some((id, head));
                self.t(
                    "The selection begins here: run Begin/End Select again where it ends.",
                    "선택이 여기서 시작합니다. 끝낼 곳에서 다시 실행하세요.",
                )
                .to_string()
            }
        }
    }

    fn panel_keys(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = e
                && let Some(a) = self.action_of(key, modifiers)
                && matches!(
                    a,
                    Action::New
                        | Action::Open
                        | Action::Save
                        | Action::SaveAs
                        | Action::Close
                        | Action::NextDocument
                        | Action::PreviousDocument
                        | Action::GoToTab(_)
                        | Action::ReopenClosed
                        | Action::Palette
                        | Action::Find
                        | Action::Replace
                        | Action::FindInFiles
                        | Action::Incremental
                        | Action::NextResult
                        | Action::PreviousResult
                        | Action::ToggleResults
                        | Action::TogglePreview
                        | Action::ToggleSplit
                        | Action::ZoomIn
                        | Action::ZoomOut
                        | Action::ZoomReset
                )
            {
                self.act(a);
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
                    // Step 100: the completion popup, a hint or a snippet's tab stops first.
                    if self.assist_key(key, modifiers) {
                        continue;
                    }
                    // Step 102: a saved macro's or command's shortcut.
                    if self.user_shortcut(key, modifiers) {
                        continue;
                    }
                    if let Some(a) = self.action_of(key, modifiers) {
                        self.act(a);
                        if matches!(
                            a,
                            Action::Cmd(
                                jarak_core::command::Command::DeleteBackward
                                    | jarak_core::command::Command::DeleteForward
                            )
                        ) {
                            self.after_deleted();
                        }
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
                    if !taken && let Some(a) = self.action_of(key, modifiers) {
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
                            if let Some(a) = self.action_of(*key, *modifiers)
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
        let carets = d.selection().len();
        let pos = if carets > 1 {
            format!("{pos} · {carets} {}", self.t("carets", "캐럿"))
        } else {
            pos
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
                self.lang_for(self.active).map_or_else(
                    || self.t("Plain text", "일반 텍스트").to_string(),
                    |l| l.name.clone(),
                ),
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
        // Step 102: a macro is being recorded.
        if self.recording.is_some() {
            items.push((self.t("● Recording", "● 기록 중").to_string(), c.danger));
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
        // Step 103: a click on the encoding, line-ending or language item opens its menu.
        let mut status_click: Option<usize> = None;
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
                if kim_ui::gear_at(ui, gear, &label, self.settings_win.is_some()).clicked() {
                    self.open_settings(None);
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
                                        RichText::new(l)
                                            .size(font::CAPTION)
                                            .strong()
                                            .color(c.accent),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text(tip);
                            r.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::Role::Button,
                                    true,
                                    format!("Vim mode {l}"),
                                )
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
                            for (k, (item, col)) in items.iter().enumerate().rev() {
                                let text = RichText::new(item).size(font::CAPTION).color(*col);
                                let r = if (1..=3).contains(&k) {
                                    let r = ui
                                        .add(egui::Label::new(text).sense(egui::Sense::click()))
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if r.clicked() {
                                        status_click = Some(k);
                                    }
                                    r
                                } else {
                                    ui.label(text)
                                };
                                // Step 96: the status items are in the geometry report too.
                                if kim_ui::inspect::active() {
                                    kim_ui::inspect::record(
                                        ui,
                                        &format!("status:item#{k}"),
                                        "label",
                                        r.rect,
                                        Some(kim_ui::inspect::Painted {
                                            content: item,
                                            rect: r.rect,
                                            size: font::CAPTION,
                                            truncated: false,
                                        }),
                                        false,
                                        &[],
                                        Some(*col),
                                        None,
                                    );
                                }
                            }
                        });
                    });
                });
                if leave_vim {
                    self.set_editing(Editing::Standard);
                }
                if let Some(k) = status_click {
                    let query = match k {
                        1 => "Encoding:",
                        2 => "EOL:",
                        _ => "Language:",
                    };
                    self.palette = Some(palette::PaletteState {
                        query: query.into(),
                        selected: 0,
                        focus: true,
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
        let recycle = matches!(self.pending, Some(Pending::Recycle(_)));
        let (title, body) = if recycle {
            (
                format!(
                    "{} {name}?",
                    self.t("Move to the Recycle Bin:", "휴지통으로 옮길까요:")
                ),
                self.t(
                    "The file goes to the Recycle Bin (it can be restored there) and the document closes.",
                    "파일이 휴지통으로 가고(거기서 복원할 수 있음) 문서가 닫힙니다.",
                )
                .to_string(),
            )
        } else {
            match copy {
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
        }
        };
        let (save, dont, cancel) = if recycle {
            (
                self.t("Move to the Recycle Bin", "휴지통으로 옮기기")
                    .to_string(),
                String::new(),
                self.t("Cancel", "취소").to_string(),
            )
        } else if copy.is_some() {
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
                if !dont.is_empty() && ui.add(kim_ui::Button::secondary(&dont)).clicked() {
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

    /// The window is gone: the session is written if closing did not write it already (a UI
    /// audit's own close, `KIM_UI_SESSION=1`).
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if !self.session_saved {
            self.save_session();
            self.session_saved = true;
        }
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
    fn folder(&self) -> Option<PathBuf> {
        win_folder_dialog("Find in Files: choose a folder").map(PathBuf::from)
    }
    fn save_export(&self, suggested: &str, format: jarak_core::export::Format) -> Option<PathBuf> {
        win_export_dialog(suggested, format).map(PathBuf::from)
    }
    fn export_folder(&self) -> Option<PathBuf> {
        win_folder_dialog("Publish to a folder (the page and an assets folder are written into it)")
            .map(PathBuf::from)
    }
}

/// The Windows folder picker (`SHBrowseForFolderW`, new dialog style).
#[cfg(windows)]
#[allow(unsafe_code)]
fn win_folder_dialog(title: &str) -> Option<String> {
    use windows_sys::Win32::UI::Shell::{
        BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW,
        SHGetPathFromIDListW,
    };
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: BROWSEINFOW is plain data; all-zero is its empty state.
    let mut bi: BROWSEINFOW = unsafe { std::mem::zeroed() };
    bi.lpszTitle = title.as_ptr();
    bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
    // SAFETY: `bi` and `title` live across the modal call.
    let pidl = unsafe { SHBrowseForFolderW(&raw const bi) };
    if pidl.is_null() {
        return None;
    }
    let mut buf = vec![0u16; 1024];
    // SAFETY: `buf` holds MAX_PATH characters and more; the PIDL is freed with CoTaskMemFree.
    let ok = unsafe { SHGetPathFromIDListW(pidl, buf.as_mut_ptr()) };
    // SAFETY: the PIDL came from SHBrowseForFolderW and is freed once.
    unsafe { windows_sys::Win32::System::Com::CoTaskMemFree(pidl as *const core::ffi::c_void) };
    if ok == 0 {
        return None;
    }
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

/// Step 106: the Save dialog of an export (its format's filter and extension; it asks before it
/// replaces a file).
#[cfg(windows)]
#[allow(unsafe_code)]
fn win_export_dialog(suggested: &str, format: jarak_core::export::Format) -> Option<String> {
    use jarak_core::export::Format;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetSaveFileNameW, OFN_NOREADONLYRETURN, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
        OPENFILENAMEW,
    };
    let (filter, ext) = match format {
        Format::Rtf => ("Rich Text (*.rtf)\0*.rtf\0\0", "rtf\0"),
        Format::Pdf => ("PDF (*.pdf)\0*.pdf\0\0", "pdf\0"),
        _ => ("HTML (*.html)\0*.html;*.htm\0\0", "html\0"),
    };
    let filter: Vec<u16> = filter.encode_utf16().collect();
    let ext: Vec<u16> = ext.encode_utf16().collect();
    let mut buf = vec![0u16; 4096];
    for (i, u) in suggested.encode_utf16().take(buf.len() - 1).enumerate() {
        buf[i] = u;
    }
    // SAFETY: OPENFILENAMEW is plain data; all-zero is its documented empty state.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrDefExt = ext.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.Flags = OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_NOREADONLYRETURN;
    // SAFETY: `ofn` points to buffers that live across the modal call.
    if unsafe { GetSaveFileNameW(&raw mut ofn) } == 0 {
        return None;
    }
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
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
