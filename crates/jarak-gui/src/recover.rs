//! Recovery snapshots in the window (Step 104, `JARAK-FR-121`, REL-02, JRK-16; the folder format:
//! `jarak_core::recovery`).
//!
//! - Every [`Recovery::interval`] seconds (the `recovery_interval` setting, 10 by default) the
//!   documents with unsaved changes — untitled ones too — whose text changed since the last
//!   snapshot are written by a short-lived thread (the text is a cheap copy of the rope, so the
//!   window never waits). A document that is saved, discarded or closed leaves the snapshot at
//!   once. A normal exit removes the window's folder.
//! - At start, folders left by windows that no longer run are offered: **Restore** opens their
//!   documents with the snapshot's text, unsaved (a document with a file gets the text as one
//!   undo step over the file's, so Ctrl+Z shows what is on disk), **Not now** keeps them for the
//!   next start, **Discard** (asked twice) removes them.

use crate::{JarakApp, Tab};
use eframe::egui;
use jarak_core::file::{self, FileDocument};
use jarak_core::recovery::{self, Entry, Orphan, Owner, Snapshot, Store};
use jarak_core::selection::Selection;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::mpsc;

/// The default seconds between snapshots.
pub const DEFAULT_INTERVAL: f64 = 10.0;

/// The window's snapshots.
#[derive(Debug)]
pub struct Recovery {
    pub store: Store,
    /// Seconds between snapshots.
    pub interval: f64,
    /// Snapshots are written (the setting; off keeps no folder).
    pub on: bool,
    /// Per document id: the history state its snapshot holds.
    written: HashMap<u64, u64>,
    last: f64,
    job: Option<mpsc::Receiver<io::Result<recovery::Written>>>,
    /// Folders restored from: removed once this window's own snapshot of them is written.
    claimed: Vec<Orphan>,
    noted: bool,
}

impl Recovery {
    pub fn new(store: Store, interval: f64) -> Recovery {
        Recovery {
            store,
            interval,
            on: interval > 0.0,
            written: HashMap::new(),
            last: f64::NEG_INFINITY,
            job: None,
            claimed: Vec::new(),
            noted: false,
        }
    }

    /// Waits for a write in progress (before the folder is removed at exit).
    fn finish(&mut self) -> Option<io::Result<recovery::Written>> {
        self.job.take().and_then(|rx| rx.recv().ok())
    }
}

/// The folders offered at start.
#[derive(Debug)]
pub struct Offer {
    pub orphans: Vec<Orphan>,
    /// Discard was pressed once: the second press removes them.
    pub confirm_discard: bool,
}

impl Offer {
    pub fn count(&self) -> usize {
        self.orphans.iter().map(|o| o.manifest.entries.len()).sum()
    }
}

impl JarakApp {
    /// Keeps snapshots in `root` for this window (`owner`) and offers the folders of windows that
    /// no longer run (`alive` tells; `jarak_core::recovery::owner_alive` on Windows).
    pub fn with_recovery(
        mut self,
        root: &Path,
        owner: Owner,
        alive: &dyn Fn(&Owner) -> bool,
        interval: f64,
    ) -> JarakApp {
        let orphans = recovery::orphans(root, alive);
        if !orphans.is_empty() {
            self.recovery_offer = Some(Offer {
                orphans,
                confirm_discard: false,
            });
        }
        self.recovery = Some(Recovery::new(Store::new(root, owner), interval));
        self
    }

    /// Each frame: takes a finished write and starts the next one when it is due.
    pub(crate) fn recovery_frame(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let Some(r) = self.recovery.as_mut() else {
            return;
        };
        if let Some(rx) = &r.job {
            match rx.try_recv() {
                Ok(res) => {
                    r.job = None;
                    match res {
                        Ok(w) => {
                            for o in r.claimed.drain(..) {
                                let _ = o.remove();
                            }
                            if !w.left_out.is_empty() && !r.noted {
                                r.noted = true;
                                let msg = format!(
                                    "{} {}",
                                    self.t(
                                        "Too large to keep for recovery:",
                                        "너무 커서 복구용으로 보관하지 않음:"
                                    ),
                                    w.left_out.join(", ")
                                );
                                self.message = Some(msg);
                            }
                        }
                        Err(e) => {
                            if !r.noted {
                                r.noted = true;
                                let msg = format!(
                                    "{} {e}",
                                    self.t(
                                        "Recovery snapshots could not be written:",
                                        "복구 스냅숏을 쓰지 못했습니다:"
                                    )
                                );
                                self.message = Some(msg);
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => r.job = None,
                Err(mpsc::TryRecvError::Empty) => return,
            }
        }
        let Some(r) = self.recovery.as_ref() else {
            return;
        };
        if !r.on {
            return;
        }
        let current: Vec<(u64, u64)> = self
            .tabs
            .iter()
            .filter(|t| wants_snapshot(t))
            .map(|t| (t.id, t.file.doc.history().state()))
            .collect();
        let shrank = r
            .written
            .keys()
            .any(|id| !current.iter().any(|(c, _)| c == id));
        let changed = current.iter().any(|(id, st)| r.written.get(id) != Some(st));
        let due = now - r.last >= r.interval;
        if shrank || (changed && due) {
            self.write_snapshots(ctx, current, now);
        } else if changed {
            let wait = (r.interval - (now - r.last)).max(0.1);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
    }

    fn write_snapshots(&mut self, ctx: &egui::Context, current: Vec<(u64, u64)>, now: f64) {
        let Some(r) = self.recovery.as_ref() else {
            return;
        };
        let snaps: Vec<Snapshot> = self
            .tabs
            .iter()
            .filter(|t| current.iter().any(|(id, _)| *id == t.id))
            .map(|t| {
                let state = t.file.doc.history().state();
                let unchanged = r.written.get(&t.id) == Some(&state);
                Snapshot {
                    entry: entry_of(t),
                    text: (!unchanged).then(|| t.file.doc.text().clone()),
                }
            })
            .collect();
        let store = r.store.clone();
        let (tx, rx) = mpsc::channel();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(store.write(&snaps));
            ctx2.request_repaint();
        });
        if let Some(r) = self.recovery.as_mut() {
            r.written = current.into_iter().collect();
            r.last = now;
            r.job = Some(rx);
        }
    }

    /// Writes the snapshots now and waits (tests; and the real-desktop check reads the folder).
    pub fn snapshot_now(&mut self) -> io::Result<recovery::Written> {
        let Some(r) = self.recovery.as_mut() else {
            return Ok(recovery::Written::default());
        };
        let _ = r.finish();
        let snaps: Vec<Snapshot> = self
            .tabs
            .iter()
            .filter(|t| wants_snapshot(t))
            .map(|t| Snapshot {
                entry: entry_of(t),
                text: Some(t.file.doc.text().clone()),
            })
            .collect();
        let Some(r) = self.recovery.as_mut() else {
            return Ok(recovery::Written::default());
        };
        let w = r.store.write(&snaps)?;
        r.written = self
            .tabs
            .iter()
            .filter(|t| wants_snapshot(t))
            .map(|t| (t.id, t.file.doc.history().state()))
            .collect();
        for o in r.claimed.drain(..) {
            let _ = o.remove();
        }
        Ok(w)
    }

    /// A normal exit: the window's folder goes (after a write in progress finishes).
    pub fn recovery_exit(&mut self) {
        if let Some(r) = self.recovery.as_mut() {
            let _ = r.finish();
            let _ = r.store.clear();
            r.on = false;
        }
    }

    /// Recovery snapshots on or off (kept in the settings file as the interval; off removes the
    /// window's folder).
    pub fn toggle_recovery(&mut self) {
        let Some(r) = self.recovery.as_mut() else {
            self.message = Some(
                self.t(
                    "Recovery snapshots are not available in this window.",
                    "이 창에서는 복구 스냅숏을 쓸 수 없습니다.",
                )
                .into(),
            );
            return;
        };
        r.on = !r.on;
        let on = r.on;
        if on {
            if r.interval <= 0.0 {
                r.interval = DEFAULT_INTERVAL;
            }
            r.written.clear();
            r.last = f64::NEG_INFINITY;
        } else {
            let _ = r.finish();
            let _ = r.store.clear();
            r.written.clear();
        }
        let secs = r.interval;
        if let Some(p) = &self.settings {
            let v = if on {
                format!("{secs}")
            } else {
                "0".to_string()
            };
            let _ = crate::write_setting(p, "recovery_interval", &v);
        }
        self.message = Some(if on {
            format!(
                "{} {secs} {}",
                self.t(
                    "Unsaved documents are kept for recovery every",
                    "저장하지 않은 문서를 복구용으로 보관합니다. 간격(초):"
                ),
                self.t("seconds.", "")
            )
        } else {
            self.t(
                "Recovery snapshots are off (none are kept).",
                "복구 스냅숏을 끕니다(보관하지 않음).",
            )
            .to_string()
        });
    }

    /// Restore: the offered documents open with their snapshot's text, unsaved.
    pub fn restore_offered(&mut self) {
        let Some(offer) = self.recovery_offer.take() else {
            return;
        };
        let owner = self
            .recovery
            .as_ref()
            .map_or(Owner { pid: 0, started: 0 }, |r| r.store.owner());
        let (mut restored, mut problems) = (0usize, Vec::new());
        for o in offer.orphans {
            let claimed = match o.claim(&owner) {
                Ok(c) => c,
                Err(_) => {
                    problems.push(
                        self.t(
                            "a folder was taken by another window",
                            "다른 창이 폴더를 가져갔습니다",
                        )
                        .to_string(),
                    );
                    continue;
                }
            };
            for e in &claimed.manifest.entries {
                match claimed.text(e) {
                    Ok(text) => {
                        self.restore_entry(e, &text);
                        restored += 1;
                    }
                    Err(err) => problems.push(format!("{}: {err}", e.title)),
                }
            }
            if let Some(r) = self.recovery.as_mut() {
                r.claimed.push(claimed);
            }
        }
        // This window's own snapshot of them comes first; then the old folders go.
        if let Some(r) = self.recovery.as_mut() {
            r.written.clear();
            r.last = f64::NEG_INFINITY;
        }
        let mut msg = format!(
            "{} {restored} {}",
            self.t("Restored", "복원:"),
            self.t(
                "documents with their unsaved text — save them to keep it.",
                "개 문서(저장하지 않은 텍스트). 남기려면 저장하세요."
            )
        );
        if !problems.is_empty() {
            msg.push_str(&format!(" · {}", problems.join(", ")));
        }
        self.message = Some(msg);
    }

    fn restore_entry(&mut self, e: &Entry, text: &str) {
        let blank = self.tabs.len() == 1 && self.tab().is_blank();
        let i = match &e.path {
            Some(p) => {
                if let Some(i) = self
                    .tabs
                    .iter()
                    .position(|t| t.file.path.as_deref() == Some(p.as_path()))
                {
                    i
                } else {
                    let f = FileDocument::open(p, self.fallback, file::DEFAULT_SIZE_LIMIT)
                        .unwrap_or_else(|_| FileDocument {
                            path: Some(p.clone()),
                            ..FileDocument::untitled()
                        });
                    self.tabs.push(Tab::new(f));
                    self.tabs.len() - 1
                }
            }
            None => {
                self.tabs.push(Tab::new(FileDocument::untitled()));
                self.tabs.len() - 1
            }
        };
        let t = &mut self.tabs[i];
        t.file.doc.set_text_minimal(text);
        t.file.format = e.format();
        t.file.doc.eol = e.eol();
        if !t.file.doc.is_dirty() {
            // The same text as the file, but kept for a conversion: still to be saved.
            t.file.doc.mark_unsaved();
        }
        let len = t.file.doc.len_chars();
        t.file
            .doc
            .set_selection(&Selection::point(e.caret.min(len)));
        t.hay = None;
        t.view.after_edit();
        let id = t.id;
        if blank && self.tabs.len() > 1 && self.tabs[0].is_blank() {
            self.tabs.remove(0);
        }
        if let Some(k) = self.index_of(id) {
            self.activate(k, crate::views::PaneId::Main);
        }
    }

    /// Discard (the second press): the offered folders are removed.
    pub fn discard_offered(&mut self) {
        let Some(offer) = self.recovery_offer.take() else {
            return;
        };
        let n = offer.count();
        let owner = self
            .recovery
            .as_ref()
            .map_or(Owner { pid: 0, started: 0 }, |r| r.store.owner());
        for o in offer.orphans {
            if let Ok(c) = o.claim(&owner) {
                let _ = c.remove();
            }
        }
        self.message = Some(format!(
            "{} {n}",
            self.t(
                "Discarded the recovered documents:",
                "복구 문서를 버렸습니다:"
            )
        ));
    }

    /// The offer at start.
    pub(crate) fn recovery_offer_ui(&mut self, ctx: &egui::Context, c: &kim_ui::Colors) {
        use kim_design::{font, space};
        let Some(offer) = self.recovery_offer.as_ref() else {
            return;
        };
        let lang = self.lang;
        let title = self
            .t(
                "Recover unsaved documents?",
                "저장하지 않은 문서를 복구할까요?",
            )
            .to_string();
        let intro = self
            .t(
                "Jarak did not close normally. These documents had unsaved changes:",
                "자락이 정상적으로 닫히지 않았습니다. 저장하지 않은 변경이 있던 문서:",
            )
            .to_string();
        let after = self
            .t(
                "Restore opens them with that text, unsaved; the files on disk stay as they are until you save.",
                "복원하면 그 텍스트로 열리며 저장되지 않은 상태입니다. 저장하기 전까지 디스크의 파일은 그대로입니다.",
            )
            .to_string();
        let rows: Vec<String> = offer
            .orphans
            .iter()
            .flat_map(|o| o.manifest.entries.iter())
            .map(|e| {
                let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.saved_at);
                let place = e.path.as_ref().map_or_else(
                    || lang.pick("untitled", "제목 없음").to_string(),
                    |p| p.display().to_string(),
                );
                format!(
                    "{} — {place} · {} {} · {} {}",
                    e.title,
                    e.chars,
                    lang.pick("characters", "자"),
                    lang.pick("kept", "보관"),
                    crate::navmap::when(at)
                )
            })
            .collect();
        let confirm = offer.confirm_discard;
        let mut act: Option<u8> = None;
        kim_ui::dialog_sized(ctx, "jarak-recovery", &title, 560.0, |ui| {
            ui.label(
                egui::RichText::new(&intro)
                    .size(font::BODY)
                    .color(c.text_primary),
            );
            ui.add_space(space::XS);
            for (n, row) in rows.iter().enumerate() {
                let r = ui.label(
                    egui::RichText::new(row)
                        .size(font::BODY)
                        .color(c.text_secondary),
                );
                if kim_ui::inspect::active() {
                    kim_ui::inspect::record(
                        ui,
                        &format!("recovery:row#{n}"),
                        "label",
                        r.rect,
                        Some(kim_ui::inspect::Painted {
                            content: row,
                            rect: r.rect,
                            size: font::BODY,
                            truncated: false,
                        }),
                        false,
                        &[],
                        Some(c.text_secondary),
                        None,
                    );
                }
            }
            ui.add_space(space::XS);
            ui.label(
                egui::RichText::new(&after)
                    .size(font::CAPTION)
                    .color(c.text_muted),
            );
            ui.add_space(space::S);
            kim_ui::dialog_actions(ui, |ui| {
                let discard = if confirm {
                    kim_ui::Button::danger(lang.pick("Discard for good", "완전히 버리기"))
                } else {
                    kim_ui::Button::ghost(lang.pick("Discard…", "버리기…"))
                };
                if ui.add(discard).clicked() {
                    act = Some(if confirm { 3 } else { 2 });
                }
                if ui
                    .add(kim_ui::Button::secondary(lang.pick("Not now", "나중에")))
                    .clicked()
                {
                    act = Some(1);
                }
                if ui
                    .add(kim_ui::Button::primary(lang.pick("Restore", "복원")))
                    .clicked()
                {
                    act = Some(0);
                }
            });
        });
        match act {
            Some(0) => self.restore_offered(),
            Some(1) => {
                self.recovery_offer = None;
                self.message = Some(
                    self.t(
                        "The recovered documents are kept for the next start.",
                        "복구 문서는 다음 시작 때까지 보관합니다.",
                    )
                    .into(),
                );
            }
            Some(2) => {
                if let Some(o) = self.recovery_offer.as_mut() {
                    o.confirm_discard = true;
                }
            }
            Some(3) => self.discard_offered(),
            _ => {}
        }
    }
}

/// A document is kept for recovery: unsaved changes, and not an empty untitled one.
fn wants_snapshot(t: &Tab) -> bool {
    t.file.doc.is_dirty()
        && !(t.file.path.is_none() && t.file.doc.len_chars() == 0)
        && t.label.is_none()
}

fn entry_of(t: &Tab) -> Entry {
    let disk = t.file.disk;
    Entry {
        id: t.id,
        path: t.file.path.clone(),
        title: t.title(),
        text: format!("doc-{}.txt", t.id),
        encoding: t.file.format.encoding.name().to_string(),
        bom: t.file.format.bom,
        eol: Entry::eol_name(t.file.doc.eol).to_string(),
        caret: t.file.doc.selection().primary().head,
        chars: t.file.doc.len_chars(),
        saved_at: recovery::now_secs(),
        disk_len: disk.map(|d| d.len),
        disk_modified_ms: disk.and_then(|d| d.modified).and_then(recovery::millis),
    }
}
