//! The side panel (Step 93): Folder as Workspace (`JARAK-FR-063`, FILE-06, VIEWS-12) and the
//! document list (`JARAK-FR-064`, VIEWS-10), and the Documents dialog (WIN-01, WIN-02).
//!
//! The folder is read lazily, a folder at a time when it is expanded, at most
//! [`MAX_ENTRIES`] entries each; `.git` is not shown. The filter searches names below the folder
//! (six levels, [`MAX_MATCHES`] results). Clicking a file opens it; nothing in the panel changes
//! or deletes a file.

use crate::JarakApp;
use crate::views::PaneId;
use eframe::egui::{self, RichText};
use jarak_core::target::OpenTarget;
use kim_design::{font, space};
use kim_ui::Colors;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The most entries shown of one folder.
pub const MAX_ENTRIES: usize = 5000;
/// The most files the filter lists.
pub const MAX_MATCHES: usize = 2000;

/// What the side panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Folder,
    Documents,
}

/// A folder shown as a workspace.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub root: PathBuf,
    pub filter: String,
    pub expanded: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Vec<(PathBuf, bool)>>,
}

impl Workspace {
    pub fn new(root: &Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            ..Workspace::default()
        }
    }

    /// The entries of `dir`: folders first, then files, by name (any case); `.git` skipped.
    pub fn entries(&mut self, dir: &Path) -> Vec<(PathBuf, bool)> {
        if let Some(v) = self.cache.get(dir) {
            return v.clone();
        }
        let mut v: Vec<(PathBuf, bool)> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.file_name() != ".git")
                    .take(MAX_ENTRIES)
                    .map(|e| {
                        let p = e.path();
                        let d = e.file_type().is_ok_and(|t| t.is_dir());
                        (p, d)
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by_key(|(p, d)| {
            (
                !*d,
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
            )
        });
        self.cache.insert(dir.to_path_buf(), v.clone());
        v
    }

    /// Reads the folders again (the Refresh button).
    pub fn refresh(&mut self) {
        self.cache.clear();
    }

    /// The rows to show: (path, is a folder, depth), the expanded folders opened, depth first.
    pub fn rows(&mut self) -> Vec<(PathBuf, bool, usize)> {
        if !self.filter.trim().is_empty() {
            return self.matches();
        }
        let mut out = Vec::new();
        let root = self.root.clone();
        self.walk(&root, 0, &mut out);
        out
    }

    fn walk(&mut self, dir: &Path, depth: usize, out: &mut Vec<(PathBuf, bool, usize)>) {
        for (p, d) in self.entries(dir) {
            out.push((p.clone(), d, depth));
            if out.len() > MAX_ENTRIES * 4 {
                return;
            }
            if d && self.expanded.contains(&p) {
                self.walk(&p, depth + 1, out);
            }
        }
    }

    /// Files below the root whose name holds the filter (any case).
    fn matches(&mut self) -> Vec<(PathBuf, bool, usize)> {
        let q = self.filter.trim().to_lowercase();
        let mut out = Vec::new();
        let mut stack = vec![(self.root.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            for (p, d) in self.entries(&dir) {
                if d {
                    if depth < 6 {
                        stack.push((p, depth + 1));
                    }
                } else if p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().to_lowercase().contains(&q))
                {
                    out.push((p, false, 0));
                    if out.len() >= MAX_MATCHES {
                        return out;
                    }
                }
            }
        }
        out.sort();
        out
    }
}

/// The Documents dialog's state.
#[derive(Debug, Clone, Default)]
pub struct DocsDialog {
    /// Sort by 0 name, 1 folder, 2 state.
    pub sort: u8,
    pub checked: HashSet<u64>,
}

impl JarakApp {
    /// Shows `root` as the workspace (FILE-06).
    pub fn open_workspace(&mut self, root: &Path) {
        self.workspace = Some(Workspace::new(root));
        self.side = Some(Side::Folder);
    }

    /// Shows or hides the side panel's `which` page.
    pub fn toggle_side(&mut self, which: Side) {
        self.side = if self.side == Some(which) {
            None
        } else {
            Some(which)
        };
        if which == Side::Folder && self.workspace.is_none() {
            let dir = self
                .tab()
                .file
                .path
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            match dir {
                Some(d) => self.open_workspace(&d),
                None => {
                    self.side = None;
                    self.message = Some(
                        self.t(
                            "Drop a folder on the window, or open a saved document: its folder becomes the workspace.",
                            "폴더를 창에 끌어 놓거나 저장된 문서를 여세요. 그 폴더가 작업 공간이 됩니다.",
                        )
                        .to_string(),
                    );
                }
            }
        }
    }

    /// The side panel (left).
    pub(crate) fn side_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let Some(which) = self.side else {
            return;
        };
        let lang = self.lang;
        let mut open = None;
        let mut activate = None;
        let mut page = which;
        egui::Panel::left("jarak-side")
            .resizable(true)
            .default_size(240.0)
            .size_range(160.0..=480.0)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::same(space::S as i8)),
            )
            .show(ui, |ui| {
                kim_ui::segmented(
                    ui,
                    &mut page,
                    &[
                        (Side::Folder, lang.pick("Folder", "폴더")),
                        (Side::Documents, lang.pick("Documents", "문서")),
                    ],
                );
                ui.add_space(space::S);
                match page {
                    Side::Folder => {
                        let Some(ws) = self.workspace.as_mut() else {
                            ui.label(
                                RichText::new(lang.pick(
                                    "No folder yet: drop one on the window.",
                                    "아직 폴더가 없습니다. 창에 끌어 놓으세요.",
                                ))
                                .size(font::BODY)
                                .color(c.text_muted),
                            );
                            return;
                        };
                        let name = ws
                            .root
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| ws.root.display().to_string());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&name)
                                    .size(font::BODY)
                                    .strong()
                                    .color(c.text_primary),
                            )
                            .on_hover_text(ws.root.display().to_string());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(kim_ui::Button::ghost(
                                            lang.pick("Refresh", "새로 고침"),
                                        ))
                                        .clicked()
                                    {
                                        ws.refresh();
                                    }
                                },
                            );
                        });
                        let w = ui.available_width();
                        kim_ui::search_field(
                            ui,
                            &mut ws.filter,
                            lang.pick("Filter files", "파일 거르기"),
                            lang.pick("Filter by name", "이름으로 거르기"),
                            w,
                        );
                        ui.add_space(space::XS);
                        let rows = ws.rows();
                        let filtering = !ws.filter.trim().is_empty();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                for (p, dir, depth) in rows {
                                    let base = if filtering {
                                        p.strip_prefix(&ws.root).unwrap_or(&p).display().to_string()
                                    } else {
                                        p.file_name()
                                            .map(|n| n.to_string_lossy().into_owned())
                                            .unwrap_or_default()
                                    };
                                    let mark = if dir {
                                        if ws.expanded.contains(&p) {
                                            "▾ "
                                        } else {
                                            "▸ "
                                        }
                                    } else {
                                        "   "
                                    };
                                    let label = format!("{}{mark}{base}", "    ".repeat(depth));
                                    if kim_ui::menu_item(ui, &label, false).clicked() {
                                        if dir {
                                            if !ws.expanded.remove(&p) {
                                                ws.expanded.insert(p.clone());
                                            }
                                        } else {
                                            open = Some(p.clone());
                                        }
                                    }
                                }
                            });
                    }
                    Side::Documents => {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                for (i, t) in self.tabs.iter().enumerate() {
                                    let label = format!(
                                        "{}{}",
                                        if t.file.doc.is_dirty() { "• " } else { "" },
                                        t.file.title()
                                    );
                                    let r = kim_ui::menu_item(ui, &label, false);
                                    if let Some(p) = &t.file.path {
                                        r.clone().on_hover_text_at_pointer(p.display().to_string());
                                    }
                                    if r.clicked() {
                                        activate = Some(i);
                                    }
                                }
                            });
                    }
                }
            });
        self.side = Some(page);
        if let Some(p) = open {
            self.open_target(&OpenTarget {
                path: p,
                line: None,
                column: None,
            });
        }
        if let Some(i) = activate {
            self.show_doc(i);
        }
    }

    /// Shows document `i` in the view that has it (the focused one if both).
    pub fn show_doc(&mut self, i: usize) {
        let pane = if self.order(self.focused_pane()).contains(&i) {
            self.focused_pane()
        } else if self.order(PaneId::Main).contains(&i) {
            PaneId::Main
        } else {
            PaneId::Second
        };
        self.activate(i, pane);
    }

    /// The Documents dialog (WIN-02): every open document, sorted, several at a time.
    pub(crate) fn docs_dialog_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(mut d) = self.docs_dialog.take() else {
            return;
        };
        let lang = self.lang;
        let mut rows: Vec<(usize, String, String, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                (
                    i,
                    t.file.title(),
                    t.file
                        .path
                        .as_deref()
                        .and_then(Path::parent)
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    t.file.doc.is_dirty(),
                )
            })
            .collect();
        match d.sort {
            1 => rows.sort_by_key(|a| a.2.to_lowercase()),
            2 => rows.sort_by_key(|r| !r.3),
            _ => rows.sort_by_key(|a| a.1.to_lowercase()),
        }
        let (mut activate, mut save, mut close, mut done) = (None, false, false, false);
        let title = lang.pick("Documents", "문서").to_string();
        let resp = kim_ui::dialog_sized(ctx, "jarak-documents", &title, 600.0, |ui| {
            kim_ui::segmented(
                ui,
                &mut d.sort,
                &[
                    (0, lang.pick("Name", "이름")),
                    (1, lang.pick("Folder", "폴더")),
                    (2, lang.pick("Unsaved first", "저장 안 된 것 먼저")),
                ],
            );
            ui.add_space(space::S);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (i, name, folder, dirty) in &rows {
                        let id = self.tabs[*i].id;
                        ui.horizontal(|ui| {
                            let mut on = d.checked.contains(&id);
                            let label = format!("{}{name}", if *dirty { "• " } else { "" });
                            if kim_ui::checkbox(ui, &mut on, &label).changed() {
                                if on {
                                    d.checked.insert(id);
                                } else {
                                    d.checked.remove(&id);
                                }
                            }
                            ui.label(
                                RichText::new(folder)
                                    .size(font::CAPTION)
                                    .color(c.text_muted),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(kim_ui::Button::ghost(lang.pick("Show", "보기")))
                                        .clicked()
                                    {
                                        activate = Some(*i);
                                    }
                                },
                            );
                        });
                    }
                });
            kim_ui::dialog_actions(ui, |ui| {
                if ui
                    .add(kim_ui::Button::secondary(lang.pick("Done", "닫기")))
                    .clicked()
                {
                    done = true;
                }
                let n = d.checked.len();
                if ui
                    .add_enabled(
                        n > 0,
                        kim_ui::Button::secondary(lang.pick("Close checked", "선택한 문서 닫기")),
                    )
                    .clicked()
                {
                    close = true;
                }
                if ui
                    .add_enabled(
                        n > 0,
                        kim_ui::Button::secondary(lang.pick("Save checked", "선택한 문서 저장")),
                    )
                    .clicked()
                {
                    save = true;
                }
            });
        });
        if let Some(i) = activate {
            self.show_doc(i);
            return;
        }
        if save {
            for id in d.checked.clone() {
                if let Some(i) = self.index_of(id)
                    && self.tabs[i].file.doc.is_dirty()
                {
                    self.save(i);
                }
            }
        }
        if close {
            let ids: Vec<u64> = d.checked.drain().collect();
            self.close_many(ids);
            if self.pending.is_some() {
                return;
            }
        }
        if done || resp.should_close() {
            return;
        }
        self.docs_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workspace_lists_folders_first_and_filters_by_name() {
        let d = std::env::temp_dir().join(format!("jarak-ws-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src").join("deep")).unwrap();
        std::fs::create_dir_all(d.join(".git")).unwrap();
        std::fs::write(d.join("b.md"), "").unwrap();
        std::fs::write(d.join("A.txt"), "").unwrap();
        std::fs::write(d.join("src").join("main.rs"), "").unwrap();
        std::fs::write(d.join("src").join("deep").join("notes.md"), "").unwrap();
        let mut w = Workspace::new(&d);
        let names = |rows: &[(PathBuf, bool, usize)]| -> Vec<String> {
            rows.iter()
                .map(|(p, _, depth)| {
                    format!("{}{}", depth, p.file_name().unwrap().to_string_lossy())
                })
                .collect()
        };
        assert_eq!(
            names(&w.rows()),
            ["0src", "0A.txt", "0b.md"],
            ".git is not shown"
        );
        w.expanded.insert(d.join("src"));
        assert_eq!(
            names(&w.rows()),
            ["0src", "1deep", "1main.rs", "0A.txt", "0b.md"]
        );
        w.filter = "MD".into();
        let found: Vec<String> = w
            .rows()
            .iter()
            .map(|(p, _, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&"notes.md".to_string()) && found.contains(&"b.md".to_string()));
        // A new file shows after Refresh.
        std::fs::write(d.join("c.md"), "").unwrap();
        w.filter.clear();
        assert!(
            !names(&w.rows()).contains(&"0c.md".to_string()),
            "cached until Refresh"
        );
        w.refresh();
        assert!(names(&w.rows()).contains(&"0c.md".to_string()));
        let _ = std::fs::remove_dir_all(&d);
    }
}
