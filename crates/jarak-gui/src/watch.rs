//! Files changed by other programs (Step 104, `JARAK-FR-120`, `JARAK-FR-122`, REL-03…05,
//! VIEW-17).
//!
//! - The open documents' files are checked (size and last-write time, off the UI thread) when
//!   the window gets the focus and every 2 seconds while it has it — every second while a
//!   document is monitored. No service and no thread stays behind: a check is one short-lived
//!   thread, and nothing runs while Jarak does not.
//! - A changed file of a document without unsaved changes is reloaded at once (*Reload unmodified
//!   documents silently*) or asked about; with unsaved changes the question offers **Keep mine**,
//!   **Reload** and **Compare** (the version on disk opens read-only beside it). A reload is one
//!   undo step: Ctrl+Z brings back the text from before it, unsaved — nothing is lost.
//! - A deleted (or moved away) file's document stays open with its text, unsaved: Save writes it
//!   again.
//! - Save checks first: a file changed since the document last saw it is never overwritten
//!   unasked — the question comes instead.
//! - *Monitoring (tail -f)*: the document follows its file (read-only while it does) and stays at
//!   its end; a file that only grew gets the new lines added.

use crate::JarakApp;
use crate::views::{Pane, PaneId, Second};
use eframe::egui;
use jarak_core::file::{self, DiskStamp, DiskState, FileDocument};
use jarak_core::selection::Selection;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc;

/// Seconds between checks while the window has the focus, and while a document is monitored.
pub const CHECK_EVERY: f64 = 2.0;
pub const MONITOR_EVERY: f64 = 1.0;

/// A change another program made, waiting for the user's answer (by document id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskQuestion {
    /// The file changed; the document has no unsaved changes: reload or keep.
    Changed(u64),
    /// The file changed and the document has unsaved changes: keep mine, reload or compare.
    Conflict(u64),
}

impl DiskQuestion {
    pub fn id(self) -> u64 {
        match self {
            DiskQuestion::Changed(id) | DiskQuestion::Conflict(id) => id,
        }
    }
}

type Results = Vec<(u64, Option<DiskStamp>, DiskState)>;

/// The checks' state.
#[derive(Debug, Default)]
pub struct Watch {
    job: Option<mpsc::Receiver<Results>>,
    last: f64,
    focused: bool,
    /// The questions on screen (the first) and waiting.
    pub questions: VecDeque<DiskQuestion>,
}

/// What each open file is checked against: (document id, file, stamp).
fn checklist(app: &JarakApp) -> Vec<(u64, PathBuf, Option<DiskStamp>)> {
    app.tabs
        .iter()
        .filter_map(|t| Some((t.id, t.file.path.clone()?, t.file.disk)))
        .collect()
}

fn check(list: &[(u64, PathBuf, Option<DiskStamp>)]) -> Results {
    list.iter()
        .map(|(id, p, s)| (*id, *s, file::disk_state(p, *s)))
        .collect()
}

impl JarakApp {
    /// Each frame: starts a check when it is due and takes the answers of the last one.
    pub(crate) fn watch_frame(&mut self, ctx: &egui::Context) {
        if !self.watch_auto {
            return;
        }
        let now = ctx.input(|i| i.time);
        let focused = ctx.input(|i| i.focused);
        let gained = focused && !self.watch.focused;
        self.watch.focused = focused;
        if let Some(rx) = &self.watch.job {
            match rx.try_recv() {
                Ok(res) => {
                    self.watch.job = None;
                    self.disk_results(res);
                }
                Err(mpsc::TryRecvError::Disconnected) => self.watch.job = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if !self.detect_changes {
            return;
        }
        let monitoring = self.tabs.iter().any(|t| t.monitor);
        let every = if monitoring {
            MONITOR_EVERY
        } else {
            CHECK_EVERY
        };
        let due = gained || ((focused || monitoring) && now - self.watch.last >= every);
        if due && self.watch.job.is_none() {
            self.watch.last = now;
            let list = checklist(self);
            if !list.is_empty() {
                let (tx, rx) = mpsc::channel();
                let ctx2 = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(check(&list));
                    ctx2.request_repaint();
                });
                self.watch.job = Some(rx);
            }
        }
        if focused || monitoring {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(every));
        }
    }

    /// The program checks the files by itself (the window tests check when they say so).
    pub fn with_watching(mut self) -> JarakApp {
        self.watch_auto = true;
        self
    }

    /// Checks every open file now, on this thread (tests, and before a save).
    pub fn check_disk_now(&mut self) {
        let list = checklist(self);
        let res = check(&list);
        self.disk_results(res);
    }

    fn disk_results(&mut self, res: Results) {
        for (id, seen, state) in res {
            let Some(i) = self.index_of(id) else { continue };
            // Saved or reloaded since the check started: its answer is out of date.
            if self.tabs[i].file.disk != seen {
                continue;
            }
            match state {
                DiskState::Same => {}
                DiskState::Missing => self.file_missing(i),
                DiskState::Changed(_) => {
                    if self.tabs[i].monitor {
                        self.follow_file(i);
                    } else if self.watch.questions.iter().any(|q| q.id() == id) {
                    } else if !self.tabs[i].file.doc.is_dirty() && self.reload_unmodified {
                        self.reload_tab(i);
                    } else if self.tabs[i].file.doc.is_dirty() {
                        self.watch.questions.push_back(DiskQuestion::Conflict(id));
                    } else {
                        self.watch.questions.push_back(DiskQuestion::Changed(id));
                    }
                }
            }
        }
    }

    /// REL-05: the file is gone; its document stays, unsaved, so Save writes it again.
    fn file_missing(&mut self, i: usize) {
        if self.tabs[i].missing {
            return;
        }
        let t = &mut self.tabs[i];
        t.missing = true;
        t.file.disk = None;
        t.file.doc.mark_unsaved();
        let name = t.title();
        self.message = Some(format!(
            "{name}: {}",
            self.t(
                "the file was deleted or moved by another program; its text stays here, unsaved (Save writes it again).",
                "다른 프로그램이 파일을 지우거나 옮겼습니다. 텍스트는 여기 남아 있고 저장되지 않은 상태입니다(저장하면 다시 씁니다)."
            )
        ));
    }

    /// Reads the file again as one undo step (Ctrl+Z brings back the text from before it).
    pub fn reload_tab(&mut self, i: usize) {
        let fallback = self.fallback;
        let t = &mut self.tabs[i];
        let name = t.title();
        match t.file.reload_undoable(fallback, file::DEFAULT_SIZE_LIMIT) {
            Ok(_) => {
                t.missing = false;
                t.hay = None;
                t.view.after_edit();
                self.message = Some(format!(
                    "{} {name} {}",
                    self.t("Reloaded", "다시 읽음:"),
                    self.t(
                        "(changed by another program; Ctrl+Z brings back the text from before).",
                        "(다른 프로그램이 바꿈. Ctrl+Z로 이전 텍스트를 되돌립니다)."
                    )
                ));
            }
            Err(e) => {
                self.message = Some(format!(
                    "{name} {}: {e}",
                    self.t("was not reloaded", "을(를) 다시 읽지 못했습니다")
                ));
            }
        }
        self.watch.questions.retain(|q| q.id() != self.tabs[i].id);
    }

    /// Keeps the text here: the change on disk is acknowledged, and the document is unsaved (it
    /// differs from the file now), so a save writes it over the other program's change.
    pub fn keep_mine(&mut self, i: usize) {
        let t = &mut self.tabs[i];
        t.file.disk = t.file.path.as_deref().and_then(|p| DiskStamp::of(p).ok());
        t.missing = false;
        t.file.doc.mark_unsaved();
        let id = t.id;
        self.watch.questions.retain(|q| q.id() != id);
    }

    /// Opens the version on disk read-only beside the document (the second view), to compare.
    pub fn compare_with_disk(&mut self, i: usize) {
        let Some(path) = self.tabs[i].file.path.clone() else {
            return;
        };
        match FileDocument::open(&path, self.fallback, file::DEFAULT_SIZE_LIMIT) {
            Ok(mut f) => {
                f.path = None;
                f.disk = None;
                f.read_only = true;
                f.read_only_reason = Some(
                    self.t(
                        "the version on disk, opened to compare",
                        "비교하려고 연 디스크의 버전",
                    )
                    .into(),
                );
                let mut t = crate::Tab::new(f);
                t.label = Some(format!(
                    "{} ({})",
                    self.tabs[i].title(),
                    self.t("on disk", "디스크")
                ));
                let id = t.id;
                let mine = self.tabs[i].id;
                self.tabs.push(t);
                self.show_beside(id, mine);
                // The document keeps its text; the question is answered as "keep mine" for now.
                self.keep_mine(i);
            }
            Err(e) => {
                self.message = Some(format!(
                    "{}: {e}",
                    self.t(
                        "The file on disk cannot be read",
                        "디스크의 파일을 읽을 수 없습니다"
                    )
                ));
            }
        }
    }

    /// Shows document `id` in the second view, the main view keeping document `mine` (focused).
    fn show_beside(&mut self, id: u64, mine: u64) {
        let Some(k) = self.index_of(id) else { return };
        if self.tabs[k].pane == Pane::Second || self.tabs[k].pane == Pane::Both {
            return;
        }
        self.tabs[k].pane = Pane::Second;
        match self.second.as_mut() {
            Some(s) => {
                s.order.push(id);
                s.active = id;
            }
            None => self.second = Some(Second::new(id)),
        }
        if let Some(m) = self.index_of(mine)
            && self.tabs[m].pane != Pane::Second
        {
            self.main_active = mine;
        }
        self.normalize();
        if let Some(m) = self.index_of(mine) {
            let pane = if self.tabs[m].pane == Pane::Second {
                PaneId::Second
            } else {
                PaneId::Main
            };
            self.activate(m, pane);
        }
    }

    /// VIEW-17: monitoring (tail -f) on or off for the active document.
    pub fn toggle_monitor(&mut self) {
        let i = self.active;
        let t = &mut self.tabs[i];
        if t.file.path.is_none() {
            self.message = Some(
                self.t(
                    "Monitoring follows a file: save the document first.",
                    "모니터링은 파일을 따라갑니다. 먼저 저장하세요.",
                )
                .into(),
            );
            return;
        }
        if !t.monitor && t.file.doc.is_dirty() {
            self.message = Some(
                self.t(
                    "Save or undo the changes before monitoring the file.",
                    "파일을 모니터링하기 전에 변경을 저장하거나 되돌리세요.",
                )
                .into(),
            );
            return;
        }
        t.monitor = !t.monitor;
        let name = t.title();
        if t.monitor {
            t.read_only_before_monitor = t.file.read_only;
            t.file.read_only = true;
            self.message = Some(format!(
                "{} {name} {}",
                self.t("Monitoring", "모니터링:"),
                self.t(
                    "(tail -f): it follows the file and stays read-only; the same entry stops it.",
                    "(tail -f): 파일을 따라가며 읽기 전용입니다. 같은 항목으로 멈춥니다."
                )
            ));
            self.follow_file(i);
        } else {
            t.file.read_only = t.read_only_before_monitor;
            self.message = Some(format!(
                "{} {name}",
                self.t("Monitoring stopped:", "모니터링을 멈췄습니다:")
            ));
        }
    }

    /// A monitored document takes the file's new text (a grown file: an insertion at the end)
    /// and goes to its end.
    fn follow_file(&mut self, i: usize) {
        let fallback = self.fallback;
        let t = &mut self.tabs[i];
        // The document is read-only while monitored; the reload keeps it so.
        let _ = t.file.reload_undoable(fallback, file::DEFAULT_SIZE_LIMIT);
        t.file.read_only = true;
        let end = t.file.doc.len_chars();
        t.file.doc.set_selection(&Selection::point(end));
        t.hay = None;
        t.view.after_edit();
        t.view.follow = true;
    }

    /// Before a save: a file another program changed since the document last saw it is not
    /// overwritten unasked (`false`: the question is on screen instead).
    pub(crate) fn safe_to_save(&mut self, i: usize) -> bool {
        let t = &self.tabs[i];
        let Some(p) = t.file.path.as_deref() else {
            return true;
        };
        match file::disk_state(p, t.file.disk) {
            DiskState::Changed(_) if !t.missing => {
                let id = t.id;
                if !self.watch.questions.iter().any(|q| q.id() == id) {
                    self.watch.questions.push_front(DiskQuestion::Conflict(id));
                }
                self.message = Some(format!(
                    "{}: {}",
                    t.title(),
                    self.t(
                        "another program changed the file since it was opened here — choose what to keep first.",
                        "여기서 연 뒤 다른 프로그램이 파일을 바꿨습니다. 먼저 무엇을 남길지 고르세요."
                    )
                ));
                false
            }
            _ => true,
        }
    }

    /// The disk question on screen (the first waiting one).
    pub(crate) fn disk_question_ui(&mut self, ctx: &egui::Context, c: &kim_ui::Colors) {
        use kim_design::{font, space};
        let Some(q) = self.watch.questions.front().copied() else {
            return;
        };
        let Some(i) = self.index_of(q.id()) else {
            self.watch.questions.pop_front();
            return;
        };
        let lang = self.lang;
        let name = self.tabs[i].title();
        let (title, body) = match q {
            DiskQuestion::Changed(_) => (
                self.t("Changed on disk", "디스크에서 바뀜").to_string(),
                format!(
                    "{name} {}",
                    self.t(
                        "was changed by another program. Reload it? (Ctrl+Z after a reload brings back the text from before.)",
                        "을(를) 다른 프로그램이 바꿨습니다. 다시 읽을까요? (다시 읽은 뒤 Ctrl+Z로 이전 텍스트를 되돌립니다.)"
                    )
                ),
            ),
            DiskQuestion::Conflict(_) => (
                self.t("Changed on disk and here", "디스크와 여기에서 모두 바뀜")
                    .to_string(),
                format!(
                    "{name} {}",
                    self.t(
                        "was changed by another program, and it has unsaved changes here. Keep mine keeps this text (a save writes it over the other change); Reload takes the file's text (Ctrl+Z brings yours back); Compare opens the file's version beside it.",
                        "을(를) 다른 프로그램이 바꿨고, 여기에도 저장하지 않은 변경이 있습니다. 내 것 유지는 이 텍스트를 남깁니다(저장하면 다른 변경 위에 씁니다). 다시 읽기는 파일의 텍스트를 가져옵니다(Ctrl+Z로 내 것을 되돌립니다). 비교는 파일의 버전을 옆에 엽니다."
                    )
                ),
            ),
        };
        let mut answer: Option<u8> = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Esc keeps what is here (nothing is lost either way).
            answer = Some(0);
        }
        kim_ui::dialog_sized(ctx, "jarak-disk-question", &title, 480.0, |ui| {
            let r = ui.label(
                egui::RichText::new(&body)
                    .size(font::BODY)
                    .color(c.text_primary),
            );
            if kim_ui::inspect::active() {
                kim_ui::inspect::record(
                    ui,
                    "disk:question",
                    "label",
                    r.rect,
                    Some(kim_ui::inspect::Painted {
                        content: &body,
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
            ui.add_space(space::S);
            kim_ui::dialog_actions(ui, |ui| {
                let keep = match q {
                    DiskQuestion::Changed(_) => lang.pick("Keep this version", "이 버전 유지"),
                    DiskQuestion::Conflict(_) => lang.pick("Keep mine", "내 것 유지"),
                };
                if ui.add(kim_ui::Button::ghost(keep)).clicked() {
                    answer = Some(0);
                }
                if matches!(q, DiskQuestion::Conflict(_))
                    && ui
                        .add(kim_ui::Button::secondary(lang.pick("Compare", "비교")))
                        .clicked()
                {
                    answer = Some(2);
                }
                if ui
                    .add(kim_ui::Button::primary(lang.pick("Reload", "다시 읽기")))
                    .clicked()
                {
                    answer = Some(1);
                }
            });
        });
        match answer {
            Some(0) => {
                // Keeping a clean document's version: nothing to save, only the new stamp.
                if matches!(q, DiskQuestion::Changed(_)) {
                    let t = &mut self.tabs[i];
                    t.file.disk = t.file.path.as_deref().and_then(|p| DiskStamp::of(p).ok());
                    t.file.doc.mark_unsaved();
                    self.watch.questions.pop_front();
                } else {
                    self.keep_mine(i);
                }
            }
            Some(1) => self.reload_tab(i),
            Some(2) => self.compare_with_disk(i),
            _ => {}
        }
    }

    /// Step 104's settings: 0 reload from disk, 1 monitoring on/off, 2 detecting changes on/off,
    /// 3 reloading unmodified documents silently on/off, 4 recovery snapshots on/off.
    pub fn disk_tool(&mut self, k: u8) {
        match k {
            0 => {
                let i = self.active;
                if self.tabs[i].file.path.is_none() {
                    self.message = Some(
                        self.t(
                            "The document has no file yet.",
                            "문서에 아직 파일이 없습니다.",
                        )
                        .into(),
                    );
                } else {
                    self.reload_tab(i);
                }
            }
            1 => self.toggle_monitor(),
            2 => {
                self.detect_changes = !self.detect_changes;
                self.save_flag("detect_changes", self.detect_changes);
                self.message = Some(
                    if self.detect_changes {
                        self.t(
                            "Files changed by other programs are detected.",
                            "다른 프로그램이 바꾼 파일을 알아챕니다.",
                        )
                    } else {
                        self.t(
                            "Files changed by other programs are not detected.",
                            "다른 프로그램이 바꾼 파일을 알아채지 않습니다.",
                        )
                    }
                    .into(),
                );
            }
            3 => {
                self.reload_unmodified = !self.reload_unmodified;
                self.save_flag("reload_unmodified_silently", self.reload_unmodified);
                self.message = Some(
                    if self.reload_unmodified {
                        self.t(
                            "Documents without unsaved changes reload silently.",
                            "저장하지 않은 변경이 없는 문서는 묻지 않고 다시 읽습니다.",
                        )
                    } else {
                        self.t(
                            "Jarak asks before reloading a changed file.",
                            "바뀐 파일을 다시 읽기 전에 묻습니다.",
                        )
                    }
                    .into(),
                );
            }
            _ => self.toggle_recovery(),
        }
    }

    pub(crate) fn save_flag(&mut self, key: &str, on: bool) {
        if let Some(p) = &self.settings
            && let Err(e) = crate::write_setting(p, key, if on { "true" } else { "false" })
        {
            self.message = Some(format!("{key}: {e}"));
        }
    }
}
