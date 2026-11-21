//! Document operations of Step 93 (`JARAK-FR-020`, `JARAK-FR-028`, `JARAK-FR-060`): closing
//! several documents (asking about each unsaved one in turn), Save All, renaming the file,
//! moving it to the Recycle Bin, copying its path, showing its folder, a command prompt there,
//! opening it in its default program or in a new window, and reopening a closed one.

use crate::views::PaneId;
use crate::{JarakApp, Pending, tabs};
use jarak_core::session::Session;
use jarak_core::target::OpenTarget;
use std::path::{Path, PathBuf};

/// Extensions Jarak never hands to their default program (they run code).
pub const RUNS_CODE: &[&str] = &[
    "exe",
    "com",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "msi",
    "msp",
    "scr",
    "lnk",
    "hta",
    "cpl",
    "reg",
    "jar",
    "pif",
    "appref-ms",
    "application",
    "gadget",
    "msc",
    "inf",
    "sct",
    "url",
    "py",
    "pyw",
];

/// A file Jarak may open in its default program (not one that runs code).
pub fn safe_to_open(p: &Path) -> bool {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    !RUNS_CODE.contains(&ext.as_str())
}

impl JarakApp {
    /// Closes the documents `ids` one after another, asking about each one with unsaved changes
    /// (Cancel stops; the others stay open).
    pub fn close_many(&mut self, mut ids: Vec<u64>) {
        while let Some(id) = ids.first().copied() {
            ids.remove(0);
            let Some(i) = self.index_of(id) else {
                continue;
            };
            if self.tabs[i].file.doc.is_dirty() {
                let pane = if self.order(PaneId::Main).contains(&i) {
                    PaneId::Main
                } else {
                    PaneId::Second
                };
                self.activate(i, pane);
                self.pending = Some(Pending::CloseQueue(i, ids));
                return;
            }
            self.close_now(i);
        }
    }

    /// Saves every document with a file and unsaved changes (untitled ones ask for a name).
    pub fn save_all(&mut self) {
        let mut saved = 0;
        let mut failed = Vec::new();
        for i in 0..self.tabs.len() {
            let t = &self.tabs[i];
            if !t.file.doc.is_dirty() || t.file.read_only {
                continue;
            }
            if self.save(i) {
                saved += 1;
            } else {
                failed.push(self.tabs[i].file.title());
            }
        }
        self.message = Some(if failed.is_empty() {
            format!("{} {saved}", self.t("Saved documents:", "저장한 문서:"))
        } else {
            format!(
                "{} {saved} · {} {}",
                self.t("Saved documents:", "저장한 문서:"),
                self.t("not saved:", "저장 못 함:"),
                failed.join(", ")
            )
        });
    }

    /// Runs a tab context menu command on the tab at `k` of `pane` (already active).
    pub fn tab_menu(&mut self, pane: PaneId, k: usize, m: tabs::Menu) {
        let order = self.order(pane);
        let i = self.active;
        let ids = |sel: &dyn Fn(usize, usize) -> bool| -> Vec<u64> {
            order
                .iter()
                .enumerate()
                .filter(|(n, x)| sel(*n, **x))
                .map(|(_, x)| self.tabs[*x].id)
                .collect()
        };
        let path = self.tabs[i].file.path.clone();
        use tabs::Menu as M;
        match m {
            M::Close => self.close_in_pane(i, pane),
            M::CloseOthers => {
                let v = ids(&|_, x| x != i);
                self.close_many(v);
            }
            M::CloseLeft => {
                let v = ids(&|n, _| n < k);
                self.close_many(v);
            }
            M::CloseRight => {
                let v = ids(&|n, _| n > k);
                self.close_many(v);
            }
            M::CloseUnchanged => {
                let v = ids(&|_, x| !self.tabs[x].file.doc.is_dirty());
                self.close_many(v);
            }
            M::CloseAll => {
                let v = ids(&|_, _| true);
                self.close_many(v);
            }
            M::SaveAll => self.save_all(),
            M::CopyPath | M::CopyName | M::CopyFolder => {
                if let Some(p) = path {
                    let s = match m {
                        M::CopyPath => p.display().to_string(),
                        M::CopyName => p
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        _ => p
                            .parent()
                            .map(|d| d.display().to_string())
                            .unwrap_or_default(),
                    };
                    self.clipboard.set_text(&s);
                    self.message = Some(format!("{} {s}", self.t("Copied:", "복사함:")));
                }
            }
            M::OpenFolder => {
                if let Some(p) = path {
                    self.reveal(&p);
                }
            }
            M::OpenTerminal => {
                if let Some(d) = path.as_deref().and_then(Path::parent) {
                    let ok = self.opener.open_terminal(d);
                    self.message = Some(format!(
                        "{} {}",
                        if ok {
                            self.t("Command prompt in", "명령 프롬프트:")
                        } else {
                            self.t(
                                "Could not open a command prompt in",
                                "명령 프롬프트를 열지 못함:",
                            )
                        },
                        d.display()
                    ));
                }
            }
            M::OpenDefault => {
                if let Some(p) = path {
                    self.open_default(&p);
                }
            }
            M::MoveOther => self.move_to_other(),
            M::CloneOther => self.clone_to_other(),
            M::NewWindow => self.open_in_new_window(false),
            M::MoveNewWindow => self.open_in_new_window(true),
            M::Rename => self.rename_file(),
            M::Recycle => {
                if self.tabs[i].file.path.is_some() {
                    self.pending = Some(Pending::Recycle(self.tabs[i].id));
                }
            }
        }
    }

    /// Opens a file in its default program, never one that runs code (`FILE-05`).
    pub fn open_default(&mut self, p: &Path) {
        if !safe_to_open(p) {
            self.message = Some(format!(
                "{} {}",
                self.t(
                    "Jarak does not start programs or scripts; shown in Explorer instead:",
                    "Jarak은 프로그램이나 스크립트를 실행하지 않습니다. 대신 탐색기에서 보여 줍니다:"
                ),
                p.display()
            ));
            self.opener.reveal(p);
            return;
        }
        let ok = self.opener.open_file(p);
        self.message = Some(format!(
            "{} {}",
            if ok {
                self.t("Opened in its program:", "기본 프로그램으로 열었습니다:")
            } else {
                self.t("Could not open:", "열지 못했습니다:")
            },
            p.display()
        ));
    }

    /// Opens the active document in a new Jarak window (`VIEWS-03`, `RUN-05`); `close_here`
    /// moves it (it closes here once the other window has it). It must be saved first.
    pub fn open_in_new_window(&mut self, close_here: bool) {
        let i = self.active;
        let Some(p) = self.tabs[i].file.path.clone() else {
            return;
        };
        if self.tabs[i].file.doc.is_dirty() {
            self.message = Some(
                self.t(
                    "Save the document first: the new window opens the file.",
                    "먼저 문서를 저장하세요. 새 창은 파일을 엽니다.",
                )
                .to_string(),
            );
            return;
        }
        let ok = self.opener.new_window(&p);
        self.message = Some(format!(
            "{} {}",
            if ok {
                self.t("Opened in a new window:", "새 창에서 열었습니다:")
            } else {
                self.t(
                    "Could not start a new window for",
                    "새 창을 열지 못했습니다:",
                )
            },
            p.display()
        ));
        if ok && close_here {
            self.close_now(i);
        }
    }

    /// Renames the file of the active document (`FILE-12`): a new name from the Save dialog; the
    /// file moves (never replacing another) and the document follows it.
    pub fn rename_file(&mut self) {
        let i = self.active;
        let Some(old) = self.tabs[i].file.path.clone() else {
            return;
        };
        let Some(new) = self.dialogs.save_as(&self.tabs[i].file.title()) else {
            return;
        };
        if new == old {
            return;
        }
        match jarak_core::assets::move_no_replace(&old, &new) {
            Ok(()) => {
                self.tabs[i].file.path = Some(new.clone());
                Session::remember(&mut self.recent, &new);
                self.message = Some(format!(
                    "{} {} → {}",
                    self.t("Renamed:", "이름 바꿈:"),
                    old.display(),
                    new.display()
                ));
            }
            Err(e) => {
                self.message = Some(format!(
                    "{} {e}",
                    self.t("Not renamed:", "이름을 바꾸지 않음:")
                ));
            }
        }
    }

    /// After the user confirmed: moves the document's file to the Recycle Bin and closes it.
    pub(crate) fn recycle_now(&mut self, id: u64) {
        let Some(i) = self.index_of(id) else {
            return;
        };
        let Some(p) = self.tabs[i].file.path.clone() else {
            return;
        };
        if self.opener.recycle(&p) {
            self.message = Some(format!(
                "{} {}",
                self.t("Moved to the Recycle Bin:", "휴지통으로 옮겼습니다:"),
                p.display()
            ));
            self.closed.retain(|c| c != &p);
            self.recent.retain(|c| c != &p);
            self.tabs.remove(i);
            self.normalize();
        } else {
            self.message = Some(format!(
                "{} {}",
                self.t(
                    "Not moved to the Recycle Bin:",
                    "휴지통으로 옮기지 못했습니다:"
                ),
                p.display()
            ));
        }
    }

    /// Reopens the most recently closed file (Ctrl+Shift+T, `FILE-22`).
    pub fn reopen_closed(&mut self) {
        while let Some(p) = self.closed.pop() {
            if p.is_file() {
                self.open_target(&OpenTarget {
                    path: p,
                    line: None,
                    column: None,
                });
                return;
            }
        }
        self.message = Some(
            self.t("No closed file to reopen.", "다시 열 닫은 파일이 없습니다.")
                .to_string(),
        );
    }

    /// Opens a recent file (`FILE-21`).
    pub fn open_recent(&mut self, k: usize) {
        if let Some(p) = self.recent.get(k).cloned() {
            self.open_target(&OpenTarget {
                path: p,
                line: None,
                column: None,
            });
        }
    }

    /// Opens every recent file that still exists (`FILE-23`).
    pub fn open_all_recent(&mut self) {
        let list: Vec<PathBuf> = self
            .recent
            .iter()
            .filter(|p| p.is_file())
            .cloned()
            .collect();
        for p in list {
            self.open_target(&OpenTarget {
                path: p,
                line: None,
                column: None,
            });
        }
    }

    /// Empties the recent files list (`FILE-23`).
    pub fn clear_recent(&mut self) {
        self.recent.clear();
        self.message = Some(
            self.t(
                "The recent files list is empty.",
                "최근 파일 목록을 비웠습니다.",
            )
            .to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_and_scripts_are_never_opened_by_association() {
        for p in [
            "a.exe", "b.BAT", "c.ps1", "d.lnk", "e.vbs", "f.js", "g.msi", "h.reg", "i.py",
        ] {
            assert!(!safe_to_open(Path::new(p)), "{p}");
        }
        for p in [
            "notes.md",
            "a.txt",
            "pic.png",
            "doc.pdf",
            "page.html",
            "data.csv",
            "README",
        ] {
            assert!(safe_to_open(Path::new(p)), "{p}");
        }
    }
}
