//! Macros and user commands in the window (`JARAK-FR-110`, `JARAK-FR-111`, Step 102).
//!
//! - Recording takes the window's commands as they run (registry commands, typed text, the
//!   clipboard actions, indentation, saving, find and replace with the panel's options), not raw
//!   keys; playback runs them again (each run one undo step), a number of times or until the end of the
//!   document (stopping when a run makes no progress, wraps back, or the time limit is reached).
//! - Saved macros and run commands live in `macros.json`; their shortcuts are checked against the
//!   keys Windows and the input method reserve (`kim-hotkeys`), Jarak's own keys and each other.
//! - A run command never runs by itself: the Run dialog shows the exact line (variables
//!   expanded) and runs it only on the Run button; programs start without elevation (one that
//!   needs administrator rights is refused, not elevated).

use crate::JarakApp;
use crate::keys::Action;
use eframe::egui::{self, RichText};
use jarak_core::command::COMMANDS;
use jarak_core::macros::{self as core, FindStep, Macro, RunCommand, Step, Target, UserCommands};
use kim_design::{font, space};
use kim_ui::Colors;
use std::path::{Path, PathBuf};

/// Starts programs for run commands (the program asks Windows; tests record).
pub trait Runner: Send + Sync {
    /// Starts `program` with `args` passed as written, in `dir`; does not wait for it.
    fn run(&self, program: &str, args: &str, dir: Option<&Path>) -> Result<(), String>;
}

/// The real runner: `CreateProcess` (no shell, no elevation).
#[derive(Debug, Default)]
pub struct ProcessRunner;

/// `ERROR_ELEVATION_REQUIRED`: the program asks for administrator rights.
const ELEVATION_REQUIRED: i32 = 740;

impl Runner for ProcessRunner {
    fn run(&self, program: &str, args: &str, dir: Option<&Path>) -> Result<(), String> {
        let mut c = std::process::Command::new(program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            if !args.is_empty() {
                c.raw_arg(args);
            }
        }
        if let Some(d) = dir.filter(|d| d.is_dir()) {
            c.current_dir(d);
        }
        c.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        match c.spawn() {
            Ok(_) => Ok(()),
            Err(e) if e.raw_os_error() == Some(ELEVATION_REQUIRED) => {
                Err("it needs administrator rights, and Jarak never elevates".into())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// A runner that starts nothing (until one is given).
#[derive(Debug, Default)]
pub struct NoRunner;

impl Runner for NoRunner {
    fn run(&self, _: &str, _: &str, _: Option<&Path>) -> Result<(), String> {
        Err("no program runner".into())
    }
}

/// How many times a playback runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Times {
    Count(u32),
    /// Until the end of the document (or no progress, a wrap, the time limit).
    ToEnd,
}

/// The longest a playback runs.
pub const PLAY_LIMIT: std::time::Duration = std::time::Duration::from_secs(20);
/// The most runs of one playback.
pub const PLAY_MAX: u32 = 100_000;

/// The open macro or command dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// Save the recorded macro: its name, keys, whether keys are being recorded, a problem, and
    /// whether the name field still has to take the keyboard focus (when the dialog opens).
    Save {
        name: String,
        keys: String,
        capturing: bool,
        error: Option<String>,
        focus: bool,
    },
    /// Run a macro several times: the macro (`None` the recorded one), times or to the end.
    Multi {
        pick: Option<usize>,
        times: String,
        to_end: bool,
    },
    /// The saved macros and commands: which page, the row capturing keys, a problem.
    Manage {
        commands: bool,
        capturing: Option<usize>,
        error: Option<String>,
    },
    /// Run…: the line, whether it is a saved command being confirmed (its name), the name and keys
    /// to save it with, a problem, and whether the command field still has to take the keyboard
    /// focus (F5 opens it ready to type, as Notepad++'s Run box).
    Run {
        command: String,
        saved: Option<String>,
        name: String,
        keys: String,
        capturing: bool,
        error: Option<String>,
        focus: bool,
    },
}

impl JarakApp {
    /// Reads `macros.json` from the configuration folder (a broken file is named and kept).
    pub fn with_macros(mut self, path: PathBuf) -> JarakApp {
        match UserCommands::load(&path) {
            Ok(u) => self.user_cmds = u,
            Err(e) => {
                self.message = Some(format!("macros.json: {e}"));
                self.user_cmds_broken = true;
            }
        }
        self.macros_path = Some(path);
        self
    }

    pub fn with_runner(mut self, r: std::sync::Arc<dyn Runner>) -> JarakApp {
        self.runner = r;
        self
    }

    /// Writes `macros.json` (not over a file that could not be read).
    fn save_user_cmds(&mut self) {
        if self.user_cmds_broken {
            self.message = Some(
                self.t(
                    "macros.json could not be read, so it is not overwritten.",
                    "macros.json을 읽지 못해 덮어쓰지 않습니다.",
                )
                .into(),
            );
            return;
        }
        if let Some(p) = &self.macros_path
            && let Err(e) = self.user_cmds.save(p)
        {
            self.message = Some(format!("{} {e}", self.t("Not saved:", "저장 못 함:")));
        }
    }

    // ---- recording -----------------------------------------------------------------------------

    /// Ctrl+Shift+R: starts recording, or stops and keeps the recorded macro.
    pub fn toggle_recording(&mut self) {
        match self.recording.take() {
            Some(steps) => {
                let n = steps.len();
                if n > 0 {
                    self.last_macro = Some(steps);
                }
                self.message = Some(format!("{} {n}", self.t("Recorded steps:", "기록한 단계:")));
            }
            None => {
                self.recording = Some(Vec::new());
                self.message = Some(
                    self.t(
                        "Recording a macro… (Ctrl+Shift+R stops)",
                        "매크로 기록 중… (Ctrl+Shift+R로 멈춤)",
                    )
                    .into(),
                );
            }
        }
    }

    /// The step an action makes while recording (a playback records nothing).
    pub(crate) fn record_action(&mut self, a: Action) {
        if self.playing {
            return;
        }
        let Some(steps) = self.recording.as_mut() else {
            return;
        };
        let step = match a {
            Action::Cmd(c) => Step::Cmd(c.id().into()),
            Action::Indent => Step::Action("indent".into()),
            Action::Outdent => Step::Action("outdent".into()),
            Action::Copy => Step::Action("copy".into()),
            Action::Cut => Step::Action("cut".into()),
            Action::Paste => Step::Action("paste".into()),
            Action::Save => Step::Action("save".into()),
            _ => return,
        };
        steps.push(step);
    }

    /// Typed text while recording (a run of typing is one step).
    pub(crate) fn record_typed(&mut self, s: &str) {
        if self.playing {
            return;
        }
        if let Some(steps) = self.recording.as_mut() {
            if let Some(Step::Type(t)) = steps.last_mut() {
                t.push_str(s);
            } else {
                steps.push(Step::Type(s.into()));
            }
        }
    }

    /// A find or replace while recording, with the panel's options.
    pub(crate) fn record_find(&mut self, op: &str, backward: bool) {
        if self.playing || self.recording.is_none() {
            return;
        }
        let f = &self.find;
        let step = Step::Find(FindStep {
            op: op.into(),
            find: f.find.clone(),
            replace: f.replace.clone(),
            mode: match f.mode {
                jarak_core::search::Mode::Normal => "normal",
                jarak_core::search::Mode::Extended => "extended",
                jarak_core::search::Mode::Regex => "regex",
            }
            .into(),
            match_case: f.match_case,
            whole_word: f.whole_word,
            wrap: f.wrap,
            backward,
            in_selection: f.in_selection,
            dot_all: f.dot_all,
        });
        if let Some(steps) = self.recording.as_mut() {
            steps.push(step);
        }
    }

    // ---- playback ------------------------------------------------------------------------------

    /// F9: the recorded macro once.
    pub fn play_recorded(&mut self) {
        match self.last_macro.clone() {
            Some(steps) => self.play(&steps, Times::Count(1)),
            None => {
                self.message = Some(
                    self.t(
                        "No macro recorded yet (Ctrl+Shift+R).",
                        "기록한 매크로가 없습니다(Ctrl+Shift+R).",
                    )
                    .into(),
                );
            }
        }
    }

    /// Runs `steps` (each run one undo step).
    pub fn play(&mut self, steps: &[Step], times: Times) {
        if self.recording.is_some() {
            self.message = Some(
                self.t("Stop recording first.", "먼저 기록을 멈추세요.")
                    .into(),
            );
            return;
        }
        if self.tab().file.read_only && steps.iter().any(edits) {
            self.refuse_read_only();
            return;
        }
        self.playing = true;
        let i = self.active;
        let id = self.tabs[i].id;
        let start = std::time::Instant::now();
        let mut runs = 0u32;
        let mut why: Option<&str> = None;
        let target = match times {
            Times::Count(n) => n.min(PLAY_MAX),
            Times::ToEnd => PLAY_MAX,
        };
        while runs < target {
            if self.tab().id != id {
                why = Some("other document");
                break;
            }
            let before = self.progress();
            // Each run is one undo step (JARAK-FR-110).
            self.tab_mut().file.doc.begin_group();
            for s in steps {
                self.play_step(s);
            }
            if let Some(k) = self.index_of(id) {
                self.tabs[k].file.doc.end_group();
            }
            runs += 1;
            if start.elapsed() > PLAY_LIMIT {
                why = Some("time");
                break;
            }
            if times == Times::ToEnd {
                let after = self.progress();
                let len = self.tab().file.doc.len_chars();
                if after.1 >= len {
                    break;
                }
                if after == before {
                    why = Some("no progress");
                    break;
                }
                if after.1 < before.1 {
                    why = Some("wrapped");
                    break;
                }
            }
        }
        if let Some(k) = self.index_of(id) {
            self.tabs[k].view.after_edit();
        }
        self.playing = false;
        let n = runs.to_string();
        self.message = Some(match why {
            Some("time") => format!(
                "{} {n} ({})",
                self.t("Played", "재생"),
                self.t("stopped at the time limit", "시간 한도에서 멈춤")
            ),
            Some("no progress") => format!(
                "{} {n} ({})",
                self.t("Played", "재생"),
                self.t(
                    "stopped: a run changed nothing",
                    "멈춤: 한 번 돌아도 바뀐 것이 없음"
                )
            ),
            Some("wrapped") => format!(
                "{} {n} ({})",
                self.t("Played", "재생"),
                self.t("stopped: it went back to the top", "멈춤: 맨 위로 돌아감")
            ),
            Some(_) => format!(
                "{} {n} ({})",
                self.t("Played", "재생"),
                self.t("stopped: another document", "멈춤: 다른 문서")
            ),
            None => format!("{} {n}", self.t("Played:", "재생:")),
        });
    }

    /// The document's edit generation and the caret (a run's progress).
    fn progress(&self) -> (u64, usize) {
        let d = &self.tab().file.doc;
        (d.edit_gen(), d.selection().primary().head)
    }

    fn play_step(&mut self, s: &Step) {
        match s {
            Step::Cmd(id) => {
                if let Some(c) = COMMANDS.iter().find(|c| c.id == id) {
                    self.act(Action::Cmd(c.command));
                }
            }
            Step::Type(t) => self.type_text(t),
            Step::Action(a) => match a.as_str() {
                "paste" => self.act(Action::Paste),
                "copy" => self.act(Action::Copy),
                "cut" => self.act(Action::Cut),
                "indent" => self.act(Action::Indent),
                "outdent" => self.act(Action::Outdent),
                "save" => self.act(Action::Save),
                _ => {}
            },
            Step::Find(f) => {
                // The recorded options for this step; the panel's own come back after it.
                let saved = self.find.clone();
                self.find.find = f.find.clone();
                self.find.replace = f.replace.clone();
                self.find.mode = match f.mode.as_str() {
                    "extended" => jarak_core::search::Mode::Extended,
                    "regex" => jarak_core::search::Mode::Regex,
                    _ => jarak_core::search::Mode::Normal,
                };
                self.find.match_case = f.match_case;
                self.find.whole_word = f.whole_word;
                self.find.wrap = f.wrap;
                self.find.in_selection = f.in_selection;
                self.find.dot_all = f.dot_all;
                match f.op.as_str() {
                    "replace" => self.replace_one(),
                    "replace_all" => self.replace_all(),
                    _ => self.find_next(f.backward),
                }
                let status = self.find.status.clone();
                self.find = saved;
                self.find.status = status;
            }
        }
    }

    // ---- shortcuts ---------------------------------------------------------------------------

    /// Why `keys` cannot be a shortcut of macro or command `me` (its index, `commands` page), if
    /// it cannot: not a shortcut, reserved by Windows or the input method (the hotkey
    /// contract), one of Jarak's keys, a typing key, or another macro's or command's.
    pub fn shortcut_problem(&self, keys: &str, me: Option<(bool, usize)>) -> Option<String> {
        let chord = match kim_hotkeys::Chord::parse(keys) {
            Ok(c) => c,
            Err(_) => {
                return Some(format!(
                    "{} {keys}",
                    self.t("Not a shortcut:", "단축키가 아닙니다:")
                ));
            }
        };
        if let Some(r) = kim_hotkeys::reserved::chord(&chord, &kim_hotkeys::scope::Scope::App, true)
        {
            return Some(format!("{} ({})", self.t("Reserved", "예약된 키"), r.name));
        }
        if let Some((k, m)) = crate::keys::parse_label(keys) {
            if let Some(a) = crate::keys::action_for(k, m) {
                let name = crate::palette::entries()
                    .into_iter()
                    .find(|e| e.run == crate::palette::Run::Act(a))
                    .map_or_else(
                        || format!("{a:?}"),
                        |e| self.lang.pick(&e.en, &e.ko).to_string(),
                    );
                return Some(format!(
                    "{} {name}",
                    self.t("Jarak already uses it:", "Jarak이 이미 씁니다:")
                ));
            }
            if !m.ctrl && !m.alt && !m.command && k.symbol_or_name().chars().count() == 1 {
                return Some(
                    self.t("That key types text.", "글자를 입력하는 키입니다.")
                        .into(),
                );
            }
        }
        let norm = |s: &str| s.to_ascii_lowercase();
        for (i, x) in self.user_cmds.macros.iter().enumerate() {
            if me != Some((false, i)) && x.keys.as_deref().map(norm) == Some(norm(keys)) {
                return Some(format!(
                    "{} {}",
                    self.t("The macro has it:", "이 매크로가 씁니다:"),
                    x.name
                ));
            }
        }
        for (i, x) in self.user_cmds.commands.iter().enumerate() {
            if me != Some((true, i)) && x.keys.as_deref().map(norm) == Some(norm(keys)) {
                return Some(format!(
                    "{} {}",
                    self.t("The command has it:", "이 명령이 씁니다:"),
                    x.name
                ));
            }
        }
        None
    }

    /// A key press that is a saved macro's or command's shortcut: played, or the command shown
    /// for confirmation. Whether it was one.
    pub(crate) fn user_shortcut(&mut self, key: egui::Key, m: egui::Modifiers) -> bool {
        let Some(text) = kim_ui::keys::chord_text(key, m) else {
            return false;
        };
        let t = text.to_ascii_lowercase();
        if let Some(x) = self
            .user_cmds
            .macros
            .iter()
            .find(|x| x.keys.as_deref().map(str::to_ascii_lowercase) == Some(t.clone()))
        {
            let steps = x.steps.clone();
            self.play(&steps, Times::Count(1));
            return true;
        }
        if let Some(i) = self
            .user_cmds
            .commands
            .iter()
            .position(|x| x.keys.as_deref().map(str::to_ascii_lowercase) == Some(t.clone()))
        {
            self.confirm_command(i);
            return true;
        }
        false
    }

    // ---- run commands ------------------------------------------------------------------------

    /// The variables of the active document now.
    pub fn run_vars(&self) -> core::Vars {
        let t = self.tab();
        let d = &t.file.doc;
        let path = t.file.path.clone().unwrap_or_default();
        let head = d.selection().primary().head.min(d.len_chars());
        let line = d.text().char_to_line(head);
        let start = d.text().line_to_char(line);
        let l = d.text().line(line);
        let text = l.slice(..jarak_core::text::line_len(l)).to_string();
        let (_, word) = jarak_core::assist::word_before(&text, head - start, "");
        let after: String = text
            .chars()
            .skip(head - start)
            .take_while(|c| jarak_core::assist::is_word(*c, ""))
            .collect();
        let exe = std::env::current_exe().unwrap_or_default();
        core::Vars {
            full_current_path: path.display().to_string(),
            current_directory: path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            name_part: path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            ext_part: path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default(),
            current_word: if d.selection().primary().is_empty() {
                format!("{word}{after}")
            } else {
                let r = d.selection().primary();
                d.text().slice(r.from()..r.to()).to_string()
            },
            current_line: line + 1,
            current_column: head - start + 1,
            current_line_text: text,
            jarak_directory: exe
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            jarak_full_file_path: exe.display().to_string(),
        }
    }

    /// The line `command` runs as now.
    pub fn expanded(&self, command: &str) -> String {
        core::expand(command, &self.run_vars(), &|n| std::env::var(n).ok())
    }

    /// F5: the Run dialog.
    pub fn open_run(&mut self) {
        self.macro_dialog = Some(Dialog::Run {
            command: String::new(),
            saved: None,
            name: String::new(),
            keys: String::new(),
            capturing: false,
            error: None,
            focus: true,
        });
    }

    /// A saved command (the palette, its shortcut): the dialog shows its exact line to confirm.
    pub fn confirm_command(&mut self, i: usize) {
        let Some(c) = self.user_cmds.commands.get(i).cloned().or_else(|| {
            core::builtin_commands()
                .get(i.wrapping_sub(self.user_cmds.commands.len()))
                .cloned()
        }) else {
            return;
        };
        self.macro_dialog = Some(Dialog::Run {
            command: c.command,
            saved: Some(c.name),
            name: String::new(),
            keys: String::new(),
            capturing: false,
            error: None,
            focus: false,
        });
    }

    /// Runs the expanded `line` (after the user pressed Run): a web address in the browser, a
    /// file with its default program, a program without a shell or elevation.
    pub fn run_line(&mut self, line: &str) -> Result<(), String> {
        let dir = self
            .tab()
            .file
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        match core::target(line) {
            None => Err(self.t("Nothing to run.", "실행할 것이 없습니다.").into()),
            Some(Target::Web(u)) => {
                if self.opener.open_url(&u) {
                    Ok(())
                } else {
                    Err(self
                        .t(
                            "The address could not be opened.",
                            "주소를 열지 못했습니다.",
                        )
                        .into())
                }
            }
            Some(Target::File(f)) => {
                let p = PathBuf::from(&f);
                if self.opener.open_file(&p) {
                    Ok(())
                } else {
                    Err(format!("{} {f}", self.t("Could not open:", "열지 못함:")))
                }
            }
            Some(Target::Program { program, args }) => {
                self.runner.run(&program, &args, dir.as_deref())
            }
        }
    }

    // ---- the browser ---------------------------------------------------------------------------

    /// The selection (or the word at the caret) searched with the engine of the settings.
    pub fn search_web(&mut self) {
        let q = self.run_vars().current_word;
        if q.trim().is_empty() {
            self.message = Some(
                self.t("Select what to search for.", "검색할 것을 선택하세요.")
                    .into(),
            );
            return;
        }
        let url = self
            .search_engine
            .replace("$(QUERY)", &url_encode(q.trim()));
        if !self.opener.open_url(&url) {
            self.message = Some(
                self.t(
                    "The search engine's address is not a web address.",
                    "검색 엔진 주소가 웹 주소가 아닙니다.",
                )
                .into(),
            );
        }
    }

    /// The document (an HTML, SVG or XML file) in the default browser.
    pub fn view_in_browser(&mut self) {
        let Some(p) = self.tab().file.path.clone() else {
            self.message = Some(
                self.t("Save the document first.", "먼저 문서를 저장하세요.")
                    .into(),
            );
            return;
        };
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "html" | "htm" | "xhtml" | "svg" | "xml") {
            self.message = Some(
                self.t(
                    "Only HTML, SVG and XML files open in the browser.",
                    "HTML, SVG, XML 파일만 브라우저로 엽니다.",
                )
                .into(),
            );
            return;
        }
        if self.tab().file.doc.is_dirty() {
            self.message = Some(
                self.t(
                    "The browser shows the saved file; save first to see the changes.",
                    "브라우저는 저장된 파일을 보여 줍니다. 바뀐 내용을 보려면 먼저 저장하세요.",
                )
                .into(),
            );
        }
        if !self.opener.open_file(&p) {
            self.message = Some(
                self.t("The file could not be opened.", "파일을 열지 못했습니다.")
                    .into(),
            );
        }
    }

    /// The palette's macro and command entries: 0 save the recorded macro, 1 run several times,
    /// 2 the macros and commands, 3 search on the internet, 4 view in the browser.
    pub fn macro_tool(&mut self, k: u8) {
        match k {
            0 => {
                if self.last_macro.as_ref().is_none_or(Vec::is_empty) {
                    self.message = Some(
                        self.t(
                            "No macro recorded yet (Ctrl+Shift+R).",
                            "기록한 매크로가 없습니다(Ctrl+Shift+R).",
                        )
                        .into(),
                    );
                    return;
                }
                self.macro_dialog = Some(Dialog::Save {
                    name: String::new(),
                    keys: String::new(),
                    capturing: false,
                    error: None,
                    focus: true,
                });
            }
            1 => {
                self.macro_dialog = Some(Dialog::Multi {
                    pick: None,
                    times: "2".into(),
                    to_end: false,
                });
            }
            2 => {
                self.macro_dialog = Some(Dialog::Manage {
                    commands: false,
                    capturing: None,
                    error: None,
                });
            }
            3 => self.search_web(),
            _ => self.view_in_browser(),
        }
    }

    /// The saved macros (with the built-in ones) and run commands as palette entries.
    pub(crate) fn macro_entries(&self) -> Vec<crate::palette::Entry> {
        let mut v = Vec::new();
        let macros: Vec<Macro> = self
            .user_cmds
            .macros
            .iter()
            .cloned()
            .chain(core::builtin_macros())
            .collect();
        for (i, m) in macros.iter().enumerate() {
            v.push(crate::palette::Entry {
                id: "macro.saved",
                en: std::borrow::Cow::Owned(format!("Macro: {}", m.name)),
                ko: std::borrow::Cow::Owned(format!("매크로: {}", m.name)),
                keys: None,
                run: crate::palette::Run::PlayMacro(i as u16),
                menu: crate::palette::Menu::Edit,
            });
        }
        let commands: Vec<RunCommand> = self
            .user_cmds
            .commands
            .iter()
            .cloned()
            .chain(core::builtin_commands())
            .collect();
        for (i, c) in commands.iter().enumerate() {
            v.push(crate::palette::Entry {
                id: "run.saved",
                en: std::borrow::Cow::Owned(format!("Run: {}", c.name)),
                ko: std::borrow::Cow::Owned(format!("실행: {}", c.name)),
                keys: None,
                run: crate::palette::Run::RunCommand(i as u16),
                menu: crate::palette::Menu::Edit,
            });
        }
        v
    }

    /// Plays saved macro `i` (the user's, then the built-in ones).
    pub fn play_saved(&mut self, i: usize) {
        let macros: Vec<Macro> = self
            .user_cmds
            .macros
            .iter()
            .cloned()
            .chain(core::builtin_macros())
            .collect();
        if let Some(m) = macros.get(i) {
            let steps = m.steps.clone();
            self.play(&steps, Times::Count(1));
        }
    }

    // ---- the dialogs -------------------------------------------------------------------------

    /// The key pressed this frame as a shortcut (Esc: cancel; Backspace / Delete: none).
    fn captured(ctx: &egui::Context) -> Option<Option<String>> {
        ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => match key {
                    egui::Key::Escape => Some(None),
                    egui::Key::Backspace | egui::Key::Delete => Some(Some(String::new())),
                    _ => kim_ui::keys::chord_text(*key, *modifiers).map(Some),
                },
                _ => None,
            })
        })
    }

    pub(crate) fn macro_dialog_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(mut d) = self.macro_dialog.take() else {
            return;
        };
        let lang = self.lang;
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut keep = true;
        match &mut d {
            Dialog::Save {
                name,
                keys,
                capturing,
                error,
                focus,
            } => {
                if *capturing {
                    if let Some(k) = Self::captured(ctx) {
                        *capturing = false;
                        if let Some(k) = k {
                            *keys = k;
                        }
                    }
                } else if esc {
                    keep = false;
                }
                let problem = (!keys.is_empty())
                    .then(|| self.shortcut_problem(keys, None))
                    .flatten();
                let mut save = false;
                let title = self
                    .t("Save the recorded macro", "기록한 매크로 저장")
                    .to_string();
                kim_ui::dialog_sized(ctx, "jarak-macro-save", &title, 420.0, |ui| {
                    let w = ui.available_width();
                    let r = kim_ui::text_field(
                        ui,
                        name,
                        lang.pick("Name", "이름"),
                        lang.pick("e.g. Tidy the list", "예: 목록 정리"),
                        w,
                    );
                    if *focus {
                        r.request_focus();
                        *focus = false;
                    }
                    ui.add_space(space::S);
                    let f = kim_ui::shortcut_field(
                        ui,
                        lang.pick("Shortcut", "단축키"),
                        keys,
                        *capturing,
                        lang.pick("None (click to set)", "없음(눌러서 정하기)"),
                        lang.pick("Press the keys…", "키를 누르세요…"),
                        w,
                    );
                    if f.clicked() {
                        *capturing = true;
                    }
                    if let Some(p) = problem.as_ref().or(error.as_ref()) {
                        kim_ui::field_error(ui, p);
                    }
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                            .clicked()
                        {
                            keep = false;
                        }
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Save", "저장")))
                            .clicked()
                        {
                            save = true;
                        }
                    });
                });
                if save {
                    if name.trim().is_empty() {
                        *error = Some(lang.pick("A name, please.", "이름을 넣으세요.").into());
                    } else if problem.is_none() {
                        let steps = self.last_macro.clone().unwrap_or_default();
                        self.user_cmds.macros.retain(|m| m.name != name.trim());
                        self.user_cmds.macros.push(Macro {
                            name: name.trim().into(),
                            keys: (!keys.is_empty()).then(|| keys.clone()),
                            steps,
                        });
                        self.save_user_cmds();
                        keep = false;
                    }
                }
            }
            Dialog::Multi {
                pick,
                times,
                to_end,
            } => {
                if esc {
                    keep = false;
                }
                let macros: Vec<Macro> = self
                    .user_cmds
                    .macros
                    .iter()
                    .cloned()
                    .chain(core::builtin_macros())
                    .collect();
                let mut options: Vec<(Option<usize>, String)> = Vec::new();
                if self.last_macro.is_some() {
                    options.push((
                        None,
                        lang.pick("The recorded macro", "기록한 매크로").into(),
                    ));
                }
                options.extend(
                    macros
                        .iter()
                        .enumerate()
                        .map(|(i, m)| (Some(i), m.name.clone())),
                );
                let mut run = false;
                let title = self
                    .t("Run a macro multiple times", "매크로 여러 번 실행")
                    .to_string();
                kim_ui::dialog_sized(ctx, "jarak-macro-multi", &title, 420.0, |ui| {
                    let w = ui.available_width();
                    if pick.is_none() && self.last_macro.is_none() && !options.is_empty() {
                        *pick = options[0].0;
                    }
                    kim_ui::dropdown(
                        ui,
                        "jarak-macro-pick",
                        lang.pick("Macro", "매크로"),
                        pick,
                        &options,
                    );
                    let _ = w;
                    ui.add_space(space::S);
                    kim_ui::segmented(
                        ui,
                        to_end,
                        &[
                            (false, lang.pick("Times", "횟수")),
                            (true, lang.pick("Until the end of the file", "파일 끝까지")),
                        ],
                    );
                    if !*to_end {
                        ui.add_space(space::XS);
                        kim_ui::text_field(ui, times, lang.pick("How many times", "몇 번"), "2", w);
                    }
                    ui.label(RichText::new(lang.pick("Each run is one undo step; it stops at the end of the file, when a run changes nothing, or after 20 seconds.", "한 번 실행할 때마다 실행 취소 한 단계입니다. 파일 끝에 닿거나, 한 번 돌아도 바뀐 것이 없거나, 20초가 지나면 멈춥니다.")).size(font::CAPTION).color(c.text_muted));
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                            .clicked()
                        {
                            keep = false;
                        }
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Run", "실행")))
                            .clicked()
                        {
                            run = true;
                        }
                    });
                });
                if run {
                    let steps = match pick {
                        Some(i) => macros.get(*i).map(|m| m.steps.clone()),
                        None => self.last_macro.clone(),
                    };
                    let n: Option<u32> = times.trim().parse().ok().filter(|n| *n >= 1);
                    match (steps, *to_end, n) {
                        (Some(s), true, _) => {
                            keep = false;
                            self.play(&s, Times::ToEnd);
                        }
                        (Some(s), false, Some(n)) => {
                            keep = false;
                            self.play(&s, Times::Count(n));
                        }
                        (None, _, _) => {
                            self.message = Some(
                                lang.pick("No macro to run.", "실행할 매크로가 없습니다.")
                                    .into(),
                            );
                        }
                        _ => {
                            self.message = Some(
                                lang.pick(
                                    "How many times: a number from 1.",
                                    "횟수: 1 이상의 숫자.",
                                )
                                .into(),
                            );
                        }
                    }
                }
            }
            Dialog::Manage {
                commands,
                capturing,
                error,
            } => {
                if let Some(row) = *capturing {
                    if let Some(k) = Self::captured(ctx) {
                        *capturing = None;
                        if let Some(k) = k {
                            match (!k.is_empty())
                                .then(|| self.shortcut_problem(&k, Some((*commands, row))))
                                .flatten()
                            {
                                Some(p) => *error = Some(p),
                                None => {
                                    *error = None;
                                    let keys = (!k.is_empty()).then_some(k);
                                    if *commands {
                                        if let Some(x) = self.user_cmds.commands.get_mut(row) {
                                            x.keys = keys;
                                        }
                                    } else if let Some(x) = self.user_cmds.macros.get_mut(row) {
                                        x.keys = keys;
                                    }
                                    self.save_user_cmds();
                                }
                            }
                        }
                    }
                } else if esc {
                    keep = false;
                }
                let mut remove = None;
                let mut capture = None;
                let rows: Vec<(String, String)> = if *commands {
                    self.user_cmds
                        .commands
                        .iter()
                        .map(|x| (x.name.clone(), x.keys.clone().unwrap_or_default()))
                        .collect()
                } else {
                    self.user_cmds
                        .macros
                        .iter()
                        .map(|x| (x.name.clone(), x.keys.clone().unwrap_or_default()))
                        .collect()
                };
                let title = self.t("Macros and commands", "매크로와 명령").to_string();
                kim_ui::dialog_sized(ctx, "jarak-macro-manage", &title, 520.0, |ui| {
                    kim_ui::segmented(
                        ui,
                        commands,
                        &[
                            (false, lang.pick("Macros", "매크로")),
                            (true, lang.pick("Run commands", "실행 명령")),
                        ],
                    );
                    ui.add_space(space::S);
                    if rows.is_empty() {
                        ui.label(
                            RichText::new(
                                lang.pick("None saved yet.", "아직 저장한 것이 없습니다."),
                            )
                            .size(font::BODY)
                            .color(c.text_muted),
                        );
                    }
                    for (i, (name, keys)) in rows.iter().enumerate() {
                        kim_ui::setting_row(ui, name, None, |ui| {
                            if ui
                                .add(kim_ui::Button::ghost(lang.pick("Remove", "지우기")).compact())
                                .clicked()
                            {
                                remove = Some(i);
                            }
                            let f = kim_ui::shortcut_field(
                                ui,
                                name,
                                keys,
                                *capturing == Some(i),
                                lang.pick("None", "없음"),
                                lang.pick("Press the keys…", "키를 누르세요…"),
                                160.0,
                            );
                            if f.clicked() {
                                capture = Some(i);
                            }
                        });
                    }
                    if let Some(e) = error.as_ref() {
                        kim_ui::field_error(ui, e);
                    }
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            keep = false;
                        }
                    });
                });
                if let Some(i) = capture {
                    *capturing = Some(i);
                }
                if let Some(i) = remove {
                    if *commands {
                        if i < self.user_cmds.commands.len() {
                            self.user_cmds.commands.remove(i);
                        }
                    } else if i < self.user_cmds.macros.len() {
                        self.user_cmds.macros.remove(i);
                    }
                    self.save_user_cmds();
                }
            }
            Dialog::Run {
                command,
                saved,
                name,
                keys,
                capturing,
                error,
                focus,
            } => {
                if *capturing {
                    if let Some(k) = Self::captured(ctx) {
                        *capturing = false;
                        if let Some(k) = k {
                            *keys = k;
                        }
                    }
                } else if esc {
                    keep = false;
                }
                let line = self.expanded(command);
                let mut run = false;
                let mut save = false;
                let title = match saved {
                    Some(n) => format!("{} {n}", self.t("Run:", "실행:")),
                    None => self.t("Run…", "실행…").to_string(),
                };
                let problem = (!keys.is_empty())
                    .then(|| self.shortcut_problem(keys, None))
                    .flatten();
                kim_ui::dialog_sized(ctx, "jarak-run", &title, 560.0, |ui| {
                    let w = ui.available_width();
                    if saved.is_none() {
                        let r = kim_ui::text_field(
                            ui,
                            command,
                            lang.pick("Command", "명령"),
                            "notepad.exe \"$(FULL_CURRENT_PATH)\"",
                            w,
                        );
                        if *focus {
                            r.request_focus();
                            *focus = false;
                        }
                        ui.label(RichText::new(format!("{} $(FULL_CURRENT_PATH) $(CURRENT_DIRECTORY) $(FILE_NAME) $(NAME_PART) $(EXT_PART) $(CURRENT_WORD) $(CURRENT_LINE) $(CURRENT_COLUMN) $(CURRENT_LINESTR) $(JARAK_DIRECTORY) $(SYS.…)", lang.pick("Variables:", "변수:"))).size(font::CAPTION).color(c.text_muted));
                        ui.add_space(space::S);
                    }
                    // The exact line that runs (JARAK-FR-111: shown before anything runs).
                    ui.label(
                        RichText::new(lang.pick("This runs exactly:", "정확히 이것을 실행합니다:"))
                            .size(font::CAPTION)
                            .color(c.text_secondary),
                    );
                    let r = ui.label(
                        RichText::new(if line.trim().is_empty() {
                            "—".to_string()
                        } else {
                            line.clone()
                        })
                        .size(font::BODY)
                        .monospace()
                        .color(c.text_primary),
                    );
                    if kim_ui::inspect::active() {
                        kim_ui::inspect::record(
                            ui,
                            "run:line",
                            "label",
                            r.rect,
                            Some(kim_ui::inspect::Painted {
                                content: &line,
                                rect: r.rect,
                                size: font::BODY,
                                truncated: false,
                            }),
                            false,
                            &[],
                            Some(c.text_primary),
                            None,
                        );
                    }
                    ui.label(RichText::new(lang.pick("Programs start as you, never as administrator; web addresses open in the browser.", "프로그램은 관리자 권한 없이 사용자 권한으로 실행되고, 웹 주소는 브라우저로 엽니다.")).size(font::CAPTION).color(c.text_muted));
                    if saved.is_none() {
                        ui.add_space(space::S);
                        kim_ui::text_field(
                            ui,
                            name,
                            lang.pick("Name (to save it)", "이름(저장할 때)"),
                            "",
                            w,
                        );
                        let f = kim_ui::shortcut_field(
                            ui,
                            lang.pick("Shortcut", "단축키"),
                            keys,
                            *capturing,
                            lang.pick("None (click to set)", "없음(눌러서 정하기)"),
                            lang.pick("Press the keys…", "키를 누르세요…"),
                            w,
                        );
                        if f.clicked() {
                            *capturing = true;
                        }
                    }
                    if let Some(p) = problem.as_ref().or(error.as_ref()) {
                        kim_ui::field_error(ui, p);
                    }
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                            .clicked()
                        {
                            keep = false;
                        }
                        if saved.is_none()
                            && ui
                                .add(kim_ui::Button::secondary(lang.pick("Save", "저장")))
                                .clicked()
                        {
                            save = true;
                        }
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Run", "실행")))
                            .clicked()
                        {
                            run = true;
                        }
                    });
                });
                if save {
                    if name.trim().is_empty() || command.trim().is_empty() {
                        *error = Some(
                            lang.pick("A name and a command, please.", "이름과 명령을 넣으세요.")
                                .into(),
                        );
                    } else if problem.is_none() {
                        self.user_cmds.commands.retain(|x| x.name != name.trim());
                        self.user_cmds.commands.push(RunCommand {
                            name: name.trim().into(),
                            command: command.clone(),
                            keys: (!keys.is_empty()).then(|| keys.clone()),
                        });
                        self.save_user_cmds();
                        *error = None;
                        self.message = Some(format!(
                            "{} {}",
                            lang.pick("Saved:", "저장함:"),
                            name.trim()
                        ));
                    }
                }
                if run {
                    match self.run_line(&line) {
                        Ok(()) => {
                            keep = false;
                            self.message =
                                Some(format!("{} {line}", lang.pick("Started:", "실행함:")));
                        }
                        Err(e) => {
                            *error = Some(format!("{} {e}", lang.pick("Not run:", "실행 못 함:")));
                        }
                    }
                }
            }
        }
        if keep {
            self.macro_dialog = Some(d);
        }
    }
}

/// Whether a step changes the text (a read-only document refuses a macro with one).
fn edits(s: &Step) -> bool {
    match s {
        Step::Cmd(id) => COMMANDS
            .iter()
            .find(|c| c.id == id)
            .is_none_or(|c| c.command.edits()),
        Step::Type(_) => true,
        Step::Action(a) => matches!(a.as_str(), "paste" | "cut" | "indent" | "outdent"),
        Step::Find(f) => f.op != "next",
    }
}

/// `s` for a URL's query (UTF-8 bytes, letters, digits and `-._~` as they are).
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
