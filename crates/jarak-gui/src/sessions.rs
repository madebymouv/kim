//! The window's session (`JARAK-FR-062`, REL-01, Step 93): written when the window closes,
//! restored when it starts (a setting, on by default), and the recent files (always kept).
//!
//! Untitled documents with text are not asked about when the window closes while the session is
//! kept: their text goes to `recovery\untitled-<n>.txt` next to the session file and comes back
//! with it (Step 104 adds periodic recovery). Documents with a file and unsaved changes are still
//! asked about, so the file on disk is never older than what the user chose to keep.

use crate::views::{Pane, PaneId, PaneState, Second};
use crate::{JarakApp, Mode, Tab};
use jarak_core::file::{self, FileDocument};
use jarak_core::selection::Selection;
use jarak_core::session::{Doc, Session, Shown, Views};
use std::path::{Path, PathBuf};

/// Where the session lives and whether it is restored.
#[derive(Debug, Clone)]
pub struct SessionFiles {
    pub file: PathBuf,
    pub recovery: PathBuf,
    pub restore: bool,
    /// Step 107: write the session at exit (not with `--no-session`).
    pub save: bool,
}

impl JarakApp {
    /// Keeps the session in `files` (the program passes `%LOCALAPPDATA%\mouv\jarak`): reads the
    /// recent files, and the documents when `files.restore` is set.
    pub fn with_session(mut self, files: SessionFiles) -> JarakApp {
        // Step 104: a damaged session file falls back to the backup (and is kept).
        let (loaded, note) = Session::load_or_backup(&files.file);
        if let Some(n) = note {
            self.message = Some(n);
        }
        match loaded {
            Ok(s) => {
                self.recent = s.recent.clone();
                self.find_history = s.find_history.clone();
                self.replace_history = s.replace_history.clone();
                if files.restore {
                    self.restore(&s, &files.recovery);
                }
            }
            Err(jarak_core::session::LoadError::Missing) => {}
            Err(e) => {
                self.message = Some(match self.message.take() {
                    Some(n) => format!("{e}. {n}"),
                    None => e.to_string(),
                });
            }
        }
        self.session = Some(files);
        self
    }

    /// Untitled documents are kept by the session instead of being asked about at exit.
    pub(crate) fn keeps_untitled(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.restore)
    }

    /// The session of the window as it is now; untitled texts are named `untitled-<n>.txt`.
    pub fn session_now(&self) -> (Session, Vec<(String, String)>) {
        let mut s = Session {
            recent: self.recent.clone(),
            find_history: self.find_history.clone(),
            replace_history: self.replace_history.clone(),
            workspace: self.workspace.as_ref().map(|w| w.root.clone()),
            ..Session::default()
        };
        let mut texts = Vec::new();
        let mut index = Vec::new();
        for (i, t) in self.tabs.iter().enumerate() {
            let untitled = if t.file.path.is_none() {
                if t.file.doc.len_chars() == 0 {
                    index.push(None);
                    continue;
                }
                let name = format!("untitled-{}.txt", texts.len() + 1);
                texts.push((name.clone(), t.file.doc.text().to_string()));
                Some(name)
            } else {
                None
            };
            // The main view's state: the alt for a clone while the second view has the focus.
            let (view, mode) = if t.pane == Pane::Both && self.second_focused() {
                t.alt
                    .as_ref()
                    .map_or((&t.view, t.mode), |a| (&a.view, a.mode))
            } else {
                (&t.view, t.mode)
            };
            let caret = if t.pane == Pane::Both && self.second_focused() {
                t.alt.as_ref().map_or(0, |a| a.sel.primary().head)
            } else {
                t.file.doc.selection().primary().head
            };
            index.push(Some(s.documents.len()));
            s.documents.push(Doc {
                path: t.file.path.clone(),
                untitled,
                caret,
                top_line: view.top_line,
                shown: match mode {
                    Mode::Source => Shown::Source,
                    Mode::Preview => Shown::Preview,
                    Mode::Split => Shown::Split,
                },
                views: match t.pane {
                    Pane::Main => Views::Main,
                    Pane::Second => Views::Second,
                    Pane::Both => Views::Both,
                },
                color: t.color,
            });
            let _ = i;
        }
        let pos = |i: usize| index.get(i).copied().flatten();
        s.active = pos(self.active_of(PaneId::Main));
        if let Some(sec) = &self.second {
            let order: Vec<usize> = self
                .order(PaneId::Second)
                .into_iter()
                .filter_map(pos)
                .collect();
            if let Some(active) = pos(self.active_of(PaneId::Second)).or(order.first().copied()) {
                s.second = Some(jarak_core::session::Second {
                    order,
                    active,
                    vertical: sec.vertical,
                    focus: sec.focus,
                });
            }
        }
        if s.check().is_err() {
            s.second = None;
        }
        (s, texts)
    }

    /// Writes the session (and the untitled texts) when the window closes.
    pub fn save_session(&mut self) {
        let Some(files) = self.session.clone() else {
            return;
        };
        // Step 107: `--no-session` leaves the saved session (and its untitled texts) as it is.
        if !files.save {
            return;
        }
        let (mut s, texts) = self.session_now();
        if !files.restore {
            s.documents.clear();
            s.active = None;
            s.second = None;
        }
        let _ = std::fs::create_dir_all(&files.recovery);
        // Old untitled texts go; the current ones are written.
        if let Ok(rd) = std::fs::read_dir(&files.recovery) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with("untitled-") && n.ends_with(".txt") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        if files.restore {
            for (name, text) in &texts {
                let _ = file::save_atomic(&files.recovery.join(name), text.as_bytes());
            }
        }
        let _ = s.save(&files.file);
    }

    /// Saves the session to a file the user names (FILE-18): the documents with a file and the
    /// views (untitled documents are not part of a saved session).
    pub fn save_session_as(&mut self) {
        let Some(p) = self.dialogs.save_as("session.json") else {
            return;
        };
        let (mut s, _) = self.session_now();
        let keep: Vec<usize> = (0..s.documents.len())
            .filter(|i| s.documents[*i].path.is_some())
            .collect();
        let map = |i: usize| keep.iter().position(|k| *k == i);
        s.active = s.active.and_then(map);
        if let Some(sec) = s.second.as_mut() {
            sec.order = sec.order.iter().filter_map(|i| map(*i)).collect();
            sec.active = map(sec.active).or(sec.order.first().copied()).unwrap_or(0);
        }
        if s.second.as_ref().is_some_and(|x| x.order.is_empty()) {
            s.second = None;
        }
        s.documents = keep.iter().map(|i| s.documents[*i].clone()).collect();
        self.message = Some(match s.save(&p) {
            Ok(()) => format!(
                "{} {}",
                self.t("Session saved:", "세션을 저장했습니다:"),
                p.display()
            ),
            Err(e) => format!(
                "{} {e}",
                self.t("The session was not saved:", "세션을 저장하지 못했습니다:")
            ),
        });
    }

    /// Opens a saved session (FILE-17): its documents are opened next to the open ones.
    pub fn load_session_from(&mut self) {
        let Some(p) = self.dialogs.open() else {
            return;
        };
        self.open_session_file(&p);
    }

    /// Step 107 (HELP-05, `--session <file>`): opens the documents of the session file `p`.
    pub fn open_session_file(&mut self, p: &Path) {
        let p = p.to_path_buf();
        match Session::load(&p) {
            Ok(s) => {
                self.restore(&s, Path::new(""));
                if self.message.is_none() {
                    self.message = Some(format!(
                        "{} {}",
                        self.t("Session opened:", "세션을 열었습니다:"),
                        p.display()
                    ));
                }
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    /// Opens the session's documents again.
    fn restore(&mut self, s: &Session, recovery: &Path) {
        let blank = self.tabs.len() == 1 && self.tab().is_blank();
        let mut ids: Vec<Option<u64>> = Vec::new();
        let mut missing = Vec::new();
        for d in &s.documents {
            if let Some(p) = &d.path
                && let Some(i) = self
                    .tabs
                    .iter()
                    .position(|t| t.file.path.as_deref() == Some(p.as_path()))
            {
                // Already open: the session's views use it as it is.
                ids.push(Some(self.tabs[i].id));
                continue;
            }
            let opened = if let Some(p) = &d.path {
                match FileDocument::open(p, self.fallback, file::DEFAULT_SIZE_LIMIT) {
                    Ok(f) => Some(Tab::new(f)),
                    Err(_) => {
                        missing.push(p.display().to_string());
                        None
                    }
                }
            } else if let Some(u) = d
                .untitled
                .as_ref()
                .filter(|_| !recovery.as_os_str().is_empty())
            {
                match std::fs::read_to_string(recovery.join(u)) {
                    Ok(text) => {
                        let mut f = FileDocument::untitled();
                        f.doc.paste(&text);
                        Some(Tab::new(f))
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            let Some(mut t) = opened else {
                ids.push(None);
                continue;
            };
            let len = t.file.doc.len_chars();
            t.file
                .doc
                .set_selection(&Selection::point(d.caret.min(len)));
            t.view.top_line = d
                .top_line
                .min(t.file.doc.text().len_lines().saturating_sub(1));
            t.view.follow = false;
            t.mode = if t.language() == "Markdown" {
                match d.shown {
                    Shown::Source => Mode::Source,
                    Shown::Preview => Mode::Preview,
                    Shown::Split => Mode::Split,
                }
            } else {
                Mode::Source
            };
            t.color = d.color.filter(|k| *k < 5);
            t.pane = match d.views {
                Views::Main => Pane::Main,
                Views::Second => Pane::Second,
                Views::Both => Pane::Both,
            };
            if t.pane == Pane::Both {
                t.alt = Some(Box::new(PaneState {
                    view: t.view.clone(),
                    mode: t.mode,
                    top_block: 0,
                    top_px: 0.0,
                    sel: t.file.doc.selection().clone(),
                    lc: crate::views::to_lc(&t.file.doc, t.file.doc.selection()),
                }));
            }
            // Step 105: a restored document keeps its own mode; word wrap is the setting's.
            t.fresh = false;
            t.view.wrap = self.prefs.word_wrap;
            ids.push(Some(t.id));
            self.tabs.push(t);
        }
        let restored = ids.iter().any(Option::is_some);
        if restored && blank {
            self.tabs.remove(0);
        }
        if !restored {
            return;
        }
        let id_at = |k: usize| ids.get(k).copied().flatten();
        if let Some(sec) = &s.second {
            let order: Vec<u64> = sec.order.iter().filter_map(|k| id_at(*k)).collect();
            if let Some(&first) = order.first() {
                let active = id_at(sec.active)
                    .filter(|a| order.contains(a))
                    .unwrap_or(first);
                self.second = Some(Second {
                    order,
                    active,
                    vertical: sec.vertical,
                    focus: false,
                    ratio: 0.5,
                    sync_v: None,
                    sync_h: None,
                });
            }
        }
        // Documents in the second view only that it does not list go back to the main view.
        let listed: Vec<u64> = self
            .second
            .as_ref()
            .map(|s| s.order.clone())
            .unwrap_or_default();
        for t in &mut self.tabs {
            if t.pane != Pane::Main && !listed.contains(&t.id) {
                t.pane = Pane::Main;
                t.alt = None;
            }
        }
        if let Some(a) = s.active.and_then(id_at) {
            self.main_active = a;
        }
        self.active = self.index_of(self.main_active).unwrap_or(0);
        self.normalize();
        if s.second.as_ref().is_some_and(|x| x.focus) && self.second.is_some() {
            self.focus_pane(PaneId::Second);
        }
        if let Some(w) = &s.workspace
            && w.is_dir()
        {
            self.open_workspace(w);
        }
        if !missing.is_empty() {
            self.message = Some(format!(
                "{} {}",
                self.t(
                    "Not found since the last session:",
                    "지난 세션 이후 찾을 수 없음:"
                ),
                missing.join(", ")
            ));
        }
    }
}
