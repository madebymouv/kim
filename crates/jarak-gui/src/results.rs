//! Find in Files, Find All and the search results panel (`JARAK-FR-073`, `JARAK-FR-074`, Step 95):
//! the find panel's *Files* page searches a folder on a worker thread (with progress and Esc to
//! cancel); Find All lists the current or every open document's matches; the results panel at the
//! bottom groups them by file (each search a block, the newest on top), opens a match on
//! double-click or Enter, steps with F4 / Shift+F4 and copies lines or paths. Replace in Files
//! shows every change in a preview where each one can be left out, then applies only the confirmed
//! ones (files open here with unsaved changes are skipped; open, saved ones are reloaded), with
//! the batch's journal for *Undo the last Replace in Files*.

use crate::JarakApp;
use eframe::egui::{self, Color32, RichText};
use jarak_core::infiles::{self, FilePlan, Found, Outcome, Scope, Skip};
use jarak_core::search::{Haystack, Matcher};
use kim_design::{Lang, font, size, space};
use kim_ui::Colors;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

/// One match in the results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultHit {
    pub line: usize,
    /// Characters, from 1.
    pub column: usize,
    /// Characters.
    pub len: usize,
    pub line_text: String,
    /// The match inside `line_text` (bytes), for the highlight.
    pub mark: (usize, usize),
}

/// The matches of one file (or open document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultFile {
    pub path: Option<PathBuf>,
    /// An open document (Find All in documents).
    pub tab: Option<u64>,
    pub label: String,
    pub hits: Vec<ResultHit>,
    pub collapsed: bool,
}

/// One search's results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub title: String,
    pub files: Vec<ResultFile>,
    pub collapsed: bool,
}

impl Block {
    pub fn hits(&self) -> usize {
        self.files.iter().map(|f| f.hits.len()).sum()
    }
}

/// A row of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Search(usize),
    File(usize, usize),
    Hit(usize, usize, usize),
}

/// The results panel.
#[derive(Debug, Clone, Default)]
pub struct Results {
    pub blocks: Vec<Block>,
    pub selected: Option<Row>,
    pub open: bool,
    /// Scroll the selected row into view on the next frame.
    reveal: bool,
}

impl Results {
    /// The rows shown (collapsed blocks and files hide their children).
    pub fn rows(&self) -> Vec<Row> {
        let mut v = Vec::new();
        for (b, block) in self.blocks.iter().enumerate() {
            v.push(Row::Search(b));
            if block.collapsed {
                continue;
            }
            for (f, file) in block.files.iter().enumerate() {
                v.push(Row::File(b, f));
                if !file.collapsed {
                    v.extend((0..file.hits.len()).map(|h| Row::Hit(b, f, h)));
                }
            }
        }
        v
    }

    /// The text of a row (as copied).
    pub fn row_text(&self, r: Row) -> String {
        match r {
            Row::Search(b) => self.blocks[b].title.clone(),
            Row::File(b, f) => self.blocks[b].files[f].label.clone(),
            Row::Hit(b, f, h) => {
                let x = &self.blocks[b].files[f].hits[h];
                format!("{}:{}: {}", x.line, x.column, x.line_text)
            }
        }
    }

    /// The next (or previous) match after the selection, across files and blocks (wrapping).
    pub fn step(&mut self, back: bool) -> Option<Row> {
        let hits: Vec<Row> =
            self.blocks
                .iter()
                .enumerate()
                .take(1)
                .flat_map(|(b, block)| {
                    block.files.iter().enumerate().flat_map(move |(f, file)| {
                        (0..file.hits.len()).map(move |h| Row::Hit(b, f, h))
                    })
                })
                .collect();
        if hits.is_empty() {
            return None;
        }
        let at = self
            .selected
            .and_then(|s| hits.iter().position(|r| *r == s));
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => hits.len() - 1,
            (Some(i), false) => (i + 1) % hits.len(),
            (Some(i), true) => (i + hits.len() - 1) % hits.len(),
        };
        // The newest block's files stay open while stepping.
        if let Row::Hit(b, f, _) = hits[next] {
            self.blocks[b].collapsed = false;
            self.blocks[b].files[f].collapsed = false;
        }
        self.selected = Some(hits[next]);
        self.reveal = true;
        self.selected
    }
}

/// What a background job reports.
#[derive(Debug)]
pub enum JobDone {
    Found(Block, Found),
    Planned(Vec<FilePlan>, Found),
    Applied(Outcome),
}

/// A running Find in Files or Replace in Files.
#[derive(Debug)]
pub struct Job {
    pub what: &'static str,
    pub cancel: Arc<AtomicBool>,
    pub done: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    pub rx: mpsc::Receiver<JobDone>,
    pub started: Instant,
}

/// The Replace in Files preview: every change with its own check box.
#[derive(Debug, Clone)]
pub struct Preview {
    pub plans: Vec<FilePlan>,
    pub keep: Vec<Vec<bool>>,
    pub skipped: Vec<(PathBuf, Skip)>,
    pub find: String,
    pub replace: String,
    pub fallback: jarak_core::file::TextEncoding,
}

impl Preview {
    pub fn chosen(&self) -> (usize, usize) {
        let files = self.keep.iter().filter(|k| k.iter().any(|x| *x)).count();
        let changes = self
            .keep
            .iter()
            .map(|k| k.iter().filter(|x| **x).count())
            .sum();
        (changes, files)
    }
}

fn hit_of_line(line: usize, column_chars: usize, len: usize, line_text: &str) -> ResultHit {
    let a = line_text
        .char_indices()
        .nth(column_chars)
        .map_or(line_text.len(), |(i, _)| i);
    let b = line_text[a..]
        .char_indices()
        .nth(len)
        .map_or(line_text.len(), |(i, _)| a + i);
    ResultHit {
        line,
        column: column_chars + 1,
        len,
        line_text: line_text.chars().take(400).collect(),
        mark: (a.min(400), b.min(400)),
    }
}

/// The block of a Find in Files.
/// "n hits" / "n건".
fn hits_text(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Ko => format!("{n}건"),
        _ => format!("{n} {}", if n == 1 { "hit" } else { "hits" }),
    }
}

/// Why a file was skipped, in the window's language.
pub fn skip_text(lang: Lang, s: &Skip) -> String {
    match (lang, s) {
        (Lang::Ko, Skip::Binary) => "바이너리 파일".into(),
        (Lang::Ko, Skip::TooLarge(n)) => format!("{n}바이트, 크기 제한 초과"),
        (Lang::Ko, Skip::Lossy) => "인코딩이 모든 바이트를 되살릴 수 없음".into(),
        (Lang::Ko, Skip::Changed) => "미리보기 뒤에 바뀜".into(),
        (Lang::Ko, Skip::Unsaved) => "저장하지 않은 변경이 있는 열린 문서".into(),
        (Lang::Ko, Skip::Unmappable(e) | Skip::Unreadable(e)) => e.clone(),
        _ => s.message(),
    }
}

/// The block of a Find in Files.
pub fn block_of(found: &Found, query: &str, dir: &Path, lang: Lang) -> Block {
    let files = found
        .files
        .iter()
        .map(|f| ResultFile {
            path: Some(f.path.clone()),
            tab: None,
            label: format!("{} ({})", f.path.display(), hits_text(lang, f.hits.len())),
            hits: f
                .hits
                .iter()
                .map(|h| {
                    // The match inside its line: from its column, as many bytes as it has (to the
                    // line's end when it runs past it).
                    let t = &h.line_text;
                    let a = t
                        .char_indices()
                        .nth(h.column - 1)
                        .map_or(t.len(), |(k, _)| k);
                    let mut b = (a + (h.end - h.start)).min(t.len());
                    while !t.is_char_boundary(b) {
                        b += 1;
                    }
                    hit_of_line(h.line, h.column - 1, t[a..b].chars().count(), t)
                })
                .collect(),
            collapsed: false,
        })
        .collect();
    let mut title = match lang {
        Lang::Ko => format!(
            "찾기 \"{query}\" (파일 {}개 중 {}개에서 {}) — {}",
            found.searched,
            found.files.len(),
            hits_text(lang, found.hits),
            dir.display()
        ),
        _ => format!(
            "Search \"{query}\" ({} in {} files of {} searched) — {}",
            hits_text(lang, found.hits),
            found.files.len(),
            found.searched,
            dir.display()
        ),
    };
    let ko = lang == Lang::Ko;
    if !found.skipped.is_empty() {
        title.push_str(&if ko {
            format!(", {}개 건너뜀", found.skipped.len())
        } else {
            format!(", {} skipped", found.skipped.len())
        });
    }
    if found.truncated {
        title.push_str(if ko {
            ", 처음 100 000건만 표시"
        } else {
            ", the first 100 000 listed"
        });
    }
    if found.cancelled {
        title.push_str(if ko { ", 취소됨" } else { ", cancelled" });
    }
    Block {
        title,
        files,
        collapsed: false,
    }
}

impl JarakApp {
    /// The folder Find in Files starts from: the field, or the current document's folder.
    pub fn files_dir(&self) -> Option<PathBuf> {
        let f = self.find.dir.trim();
        if !f.is_empty() && !self.find.follow_doc {
            return Some(PathBuf::from(f));
        }
        self.tab()
            .file
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| (!f.is_empty()).then(|| PathBuf::from(f)))
    }

    fn files_scope(&mut self) -> Option<(Scope, Matcher)> {
        let q = self.find.query();
        let m = match q.compile() {
            Ok(m) => m,
            Err(e) => {
                self.find.status = Some(e);
                return None;
            }
        };
        let Some(dir) = self.files_dir().filter(|d| d.is_dir()) else {
            self.find.status = Some(
                self.t("Choose a folder to search.", "찾을 폴더를 고르세요.")
                    .into(),
            );
            return None;
        };
        crate::find::remember(&mut self.find_history, &q.text);
        Some((
            Scope {
                dir,
                filters: infiles::Filters::parse(&self.find.filters),
                recursive: self.find.subfolders,
                hidden: self.find.hidden,
                fallback: self.fallback,
            },
            m,
        ))
    }

    fn spawn(
        &mut self,
        what: &'static str,
        work: impl FnOnce(&AtomicBool, &(dyn Fn(usize, usize) + Sync)) -> JobDone + Send + 'static,
    ) {
        let cancel = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let (c, d, t) = (cancel.clone(), done.clone(), total.clone());
        std::thread::spawn(move || {
            let progress = |n: usize, of: usize| {
                d.store(n, Ordering::Relaxed);
                t.store(of, Ordering::Relaxed);
            };
            let _ = tx.send(work(&c, &progress));
        });
        self.job = Some(Job {
            what,
            cancel,
            done,
            total,
            rx,
            started: Instant::now(),
        });
    }

    /// Find in Files: the folder searched on a worker thread.
    pub fn find_in_files(&mut self) {
        if self.job.is_some() {
            return;
        }
        let Some((scope, m)) = self.files_scope() else {
            return;
        };
        let query = self.find.find.clone();
        let lang = self.lang;
        self.find.status = Some(self.t("Searching…", "찾는 중…").into());
        self.spawn("find", move |cancel, progress| {
            let found = infiles::find_in_files(&scope, &m, cancel, progress);
            JobDone::Found(block_of(&found, &query, &scope.dir, lang), found)
        });
    }

    /// Find in Files at once, on this thread (the UI audit).
    pub fn find_in_files_now(&mut self) {
        let Some((scope, m)) = self.files_scope() else {
            return;
        };
        let found = infiles::find_in_files(&scope, &m, &AtomicBool::new(false), &|_, _| {});
        let block = block_of(&found, &self.find.find, &scope.dir, self.lang);
        self.find.status = None;
        self.push_block(block);
    }

    /// The Replace in Files preview at once, on this thread (the UI audit).
    pub fn plan_in_files_now(&mut self) {
        let Some((scope, m)) = self.files_scope() else {
            return;
        };
        let (plans, found) = infiles::plan(&scope, &m, &self.find.replace, &AtomicBool::new(false));
        let keep = plans.iter().map(|p| vec![true; p.changes.len()]).collect();
        self.preview = Some(Preview {
            plans,
            keep,
            skipped: found.skipped,
            find: self.find.find.clone(),
            replace: self.find.replace.clone(),
            fallback: self.fallback,
        });
    }

    /// Replace in Files, step 1: the plan, shown in the preview.
    pub fn replace_in_files(&mut self) {
        if self.job.is_some() {
            return;
        }
        let Some((scope, m)) = self.files_scope() else {
            return;
        };
        let with = self.find.replace.clone();
        crate::find::remember(&mut self.replace_history, &with);
        self.find.status = Some(
            self.t("Preparing the preview…", "미리보기를 준비하는 중…")
                .into(),
        );
        self.spawn("plan", move |cancel, _| {
            let (plans, found) = infiles::plan(&scope, &m, &with, cancel);
            JobDone::Planned(plans, found)
        });
    }

    /// Replace in Files, step 2: the confirmed changes applied.
    pub fn apply_preview(&mut self) {
        let Some(p) = self.preview.take() else {
            return;
        };
        let Some(root) = self.journal_root.clone() else {
            self.message = Some(
                self.t(
                    "No place for the journal: nothing was changed.",
                    "기록을 둘 곳이 없어 아무것도 바꾸지 않았습니다.",
                )
                .into(),
            );
            return;
        };
        // Files open here with unsaved changes are not touched.
        let unsaved: Vec<PathBuf> = self
            .tabs
            .iter()
            .filter(|t| t.file.doc.is_dirty())
            .filter_map(|t| t.file.path.clone())
            .collect();
        let keep = p.keep.clone();
        let plans = p.plans.clone();
        let index: std::collections::HashMap<PathBuf, usize> = plans
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.clone(), i))
            .collect();
        let (find, replace, fallback) = (p.find.clone(), p.replace.clone(), p.fallback);
        self.spawn("apply", move |_, _| {
            let mut out = infiles::apply(
                &plans,
                &|path, k| {
                    !unsaved.iter().any(|u| u == path)
                        && index
                            .get(path)
                            .is_some_and(|&i| keep[i].get(k).copied().unwrap_or(false))
                },
                fallback,
                &root,
                &find,
                &replace,
            )
            .unwrap_or_default();
            for (i, plan) in plans.iter().enumerate() {
                if unsaved.contains(&plan.path) && keep[i].iter().any(|x| *x) {
                    out.skipped.push((plan.path.clone(), Skip::Unsaved));
                }
            }
            JobDone::Applied(out)
        });
    }

    /// Reloads open documents (without unsaved changes) whose files a batch rewrote.
    fn reload_changed(&mut self, paths: &[PathBuf]) {
        for t in &mut self.tabs {
            if t.file.doc.is_dirty() {
                continue;
            }
            if t.file.path.as_ref().is_some_and(|p| paths.contains(p)) {
                let caret = t.file.doc.selection().primary().head;
                if t.file
                    .reload(self.fallback, jarak_core::file::DEFAULT_SIZE_LIMIT)
                    .is_ok()
                {
                    let len = t.file.doc.len_chars();
                    t.file
                        .doc
                        .set_selection(&jarak_core::selection::Selection::point(caret.min(len)));
                    t.hay = None;
                    t.view.after_edit();
                }
            }
        }
    }

    /// Undoes the last Replace in Files batch (files changed since are left as they are).
    pub fn undo_replace_in_files(&mut self) {
        let (Some(root), Some(id)) = (self.journal_root.clone(), self.last_batch.clone()) else {
            self.message = Some(
                self.t(
                    "No Replace in Files to undo.",
                    "되돌릴 파일 바꾸기가 없습니다.",
                )
                .into(),
            );
            return;
        };
        match infiles::undo(&root, &id, false) {
            Ok(out) => {
                let paths: Vec<PathBuf> = out.applied.iter().map(|a| a.path.clone()).collect();
                self.reload_changed(&paths);
                self.message = Some(format!(
                    "{} {} {}{}",
                    self.t("Restored", "복원:"),
                    out.applied.len(),
                    self.t("files", "개 파일"),
                    if out.skipped.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ", {} {}",
                            out.skipped.len(),
                            self.t(
                                "changed since (left as they are)",
                                "개는 그 뒤 바뀌어 그대로 둠"
                            )
                        )
                    }
                ));
                if out.skipped.is_empty() {
                    self.last_batch = None;
                }
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    /// The job's result, when it is done (every frame).
    pub(crate) fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else {
            return;
        };
        let done = match job.rx.try_recv() {
            Ok(d) => d,
            Err(mpsc::TryRecvError::Empty) => {
                let (n, of) = (
                    job.done.load(Ordering::Relaxed),
                    job.total.load(Ordering::Relaxed),
                );
                self.find.status = Some(format!(
                    "{} {n} / {of}",
                    self.t("Searching… files", "찾는 중… 파일")
                ));
                ctx.request_repaint_after(std::time::Duration::from_millis(60));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                return;
            }
        };
        self.job = None;
        match done {
            JobDone::Found(block, found) => {
                self.find.status = Some(format!(
                    "{} {} / {} {}",
                    self.t("Found:", "찾음:"),
                    found.hits,
                    found.files.len(),
                    self.t("files", "개 파일")
                ));
                self.push_block(block);
            }
            JobDone::Planned(plans, found) => {
                if plans.is_empty() {
                    self.find.status =
                        Some(self.t("Nothing to replace.", "바꿀 것이 없습니다.").into());
                    return;
                }
                let keep = plans.iter().map(|p| vec![true; p.changes.len()]).collect();
                self.preview = Some(Preview {
                    plans,
                    keep,
                    skipped: found.skipped,
                    find: self.find.find.clone(),
                    replace: self.find.replace.clone(),
                    fallback: self.fallback,
                });
                self.find.status = None;
            }
            JobDone::Applied(out) => {
                let lang = self.lang;
                let paths: Vec<PathBuf> = out.applied.iter().map(|a| a.path.clone()).collect();
                self.reload_changed(&paths);
                let n: usize = out.applied.iter().map(|a| a.replaced).sum();
                if let Some(b) = &out.batch {
                    self.last_batch = Some(b.id.clone());
                }
                let mut files: Vec<ResultFile> = out
                    .applied
                    .iter()
                    .map(|a| ResultFile {
                        path: Some(a.path.clone()),
                        tab: None,
                        label: match lang {
                            Lang::Ko => format!("{} ({}건 바꿈)", a.path.display(), a.replaced),
                            _ => format!("{} ({} replaced)", a.path.display(), a.replaced),
                        },
                        hits: Vec::new(),
                        collapsed: false,
                    })
                    .collect();
                files.extend(out.skipped.iter().map(|(p, s)| ResultFile {
                    path: Some(p.clone()),
                    tab: None,
                    label: format!(
                        "{} — {} {}",
                        p.display(),
                        if lang == Lang::Ko {
                            "건너뜀:"
                        } else {
                            "skipped:"
                        },
                        skip_text(lang, s)
                    ),
                    hits: Vec::new(),
                    collapsed: false,
                }));
                let undo = out.batch.as_ref().map_or(String::new(), |b| match lang {
                    Lang::Ko => format!(" (되돌리기: 배치 {})", b.id),
                    _ => format!(" (undo: batch {})", b.id),
                });
                let title = match lang {
                    Lang::Ko => format!(
                        "파일에서 바꾸기: 파일 {}개에서 {n}건 바꿈, {}개 건너뜀{undo}",
                        out.applied.len(),
                        out.skipped.len()
                    ),
                    _ => format!(
                        "Replace in Files: {n} replaced in {} files, {} skipped{undo}",
                        out.applied.len(),
                        out.skipped.len()
                    ),
                };
                self.message = Some(title.clone());
                self.find.status = Some(title.clone());
                self.push_block(Block {
                    title,
                    files,
                    collapsed: false,
                });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        let r = self.results.get_or_insert_with(Results::default);
        for b in &mut r.blocks {
            b.collapsed = true;
        }
        r.blocks.insert(0, block);
        r.blocks.truncate(20);
        r.selected = None;
        r.open = true;
    }

    /// Find All in the current document or in every open document (`SRCH-13`, `SRCH-14`).
    pub fn find_all_docs(&mut self, every: bool) {
        let q = self.find.query();
        let m = match q.compile() {
            Ok(m) => m,
            Err(e) => {
                self.find.status = Some(e);
                return;
            }
        };
        crate::find::remember(&mut self.find_history, &q.text);
        let list: Vec<usize> = if every {
            (0..self.tabs.len()).collect()
        } else {
            vec![self.active]
        };
        let mut files = Vec::new();
        let mut total = 0;
        for i in list {
            let h = Haystack::reuse(self.tabs[i].hay.as_ref(), &self.tabs[i].file.doc);
            self.tabs[i].hay = Some(h.clone());
            let (found, _) = m.find_all(&h, None, &|| false);
            if found.is_empty() {
                continue;
            }
            total += found.len();
            let r = self.tabs[i].file.doc.text();
            let hits = found
                .iter()
                .take(infiles::MAX_HITS)
                .map(|&(a, b)| {
                    let line = r.char_to_line(a);
                    let start = r.line_to_char(line);
                    let text: String = r
                        .line(line)
                        .chars()
                        .filter(|c| *c != '\n' && *c != '\r')
                        .collect();
                    hit_of_line(line + 1, a - start, b - a, &text)
                })
                .collect::<Vec<_>>();
            let t = &self.tabs[i];
            files.push(ResultFile {
                path: t.file.path.clone(),
                tab: Some(t.id),
                label: format!(
                    "{} ({})",
                    t.file
                        .path
                        .as_ref()
                        .map_or_else(|| t.file.title(), |p| p.display().to_string()),
                    hits_text(self.lang, hits.len())
                ),
                hits,
                collapsed: false,
            });
        }
        let title = match self.lang {
            Lang::Ko => format!(
                "찾기 \"{}\" (문서 {}개에서 {}) — {}",
                q.text,
                files.len(),
                hits_text(self.lang, total),
                if every {
                    "열린 문서"
                } else {
                    "현재 문서"
                }
            ),
            _ => format!(
                "Search \"{}\" ({} in {} {}) — {}",
                q.text,
                hits_text(self.lang, total),
                files.len(),
                if files.len() == 1 {
                    "document"
                } else {
                    "documents"
                },
                if every {
                    "open documents"
                } else {
                    "current document"
                }
            ),
        };
        self.find.status = Some(format!("{} {total}", self.t("Found:", "찾음:")));
        self.push_block(Block {
            title,
            files,
            collapsed: false,
        });
    }

    /// Replace All in every open document (each one undo step; read-only ones left).
    pub fn replace_all_docs(&mut self) {
        let q = self.find.query();
        let m = match q.compile() {
            Ok(m) => m,
            Err(e) => {
                self.find.status = Some(e);
                return;
            }
        };
        crate::find::remember(&mut self.find_history, &q.text);
        let with = self.find.replace.clone();
        crate::find::remember(&mut self.replace_history, &with);
        let (mut n, mut docs, mut ro) = (0, 0, 0);
        for i in 0..self.tabs.len() {
            if self.tabs[i].file.read_only {
                ro += 1;
                continue;
            }
            let h = Haystack::reuse(self.tabs[i].hay.as_ref(), &self.tabs[i].file.doc);
            if let Ok(k) = jarak_core::search::replace_all(
                &mut self.tabs[i].file.doc,
                &m,
                &h,
                &with,
                None,
                &|| false,
            ) && k > 0
            {
                n += k;
                docs += 1;
                self.tabs[i].view.after_edit();
            }
        }
        let s = format!(
            "{} {n} / {docs} {}{}",
            self.t("Replaced:", "바꾼 수:"),
            self.t("documents", "개 문서"),
            if ro > 0 {
                format!(" ({ro} {})", self.t("read-only left", "개 읽기 전용 제외"))
            } else {
                String::new()
            }
        );
        self.find.status = Some(s.clone());
        self.message = Some(s);
    }

    /// Opens a result's match (its file, or the open document) and selects it.
    pub fn open_result(&mut self, r: Row) {
        let Some(res) = &self.results else {
            return;
        };
        let Row::Hit(b, f, h) = r else {
            return;
        };
        let file = &res.blocks[b].files[f];
        let hit = file.hits[h].clone();
        let tab = file
            .tab
            .and_then(|id| self.tabs.iter().position(|t| t.id == id));
        let path = file.path.clone();
        match (tab, path) {
            (Some(i), _) => self.show_doc(i),
            (None, Some(p)) => {
                self.open_target(&jarak_core::target::OpenTarget {
                    path: p,
                    line: None,
                    column: None,
                });
                if self.tab().file.path.as_deref() != file_path(&self.results, r).as_deref() {
                    return;
                }
            }
            (None, None) => {
                self.message = Some(
                    self.t("That document is closed.", "그 문서는 닫혔습니다.")
                        .into(),
                );
                return;
            }
        }
        let doc = &mut self.tabs[self.active].file.doc;
        let r = doc.text();
        let l = (hit.line - 1).min(r.len_lines().saturating_sub(1));
        let start = r.line_to_char(l);
        let line_len = jarak_core::text::line_len(r.line(l));
        let a = start + (hit.column - 1).min(line_len);
        let b = (a + hit.len).min(doc.len_chars());
        doc.set_selection(&jarak_core::selection::Selection::single(
            jarak_core::selection::Range::new(a, b),
        ));
        self.tabs[self.active].view.after_edit();
    }

    /// F4 / Shift+F4: the next / previous result, opened.
    pub fn result_step(&mut self, back: bool) {
        let Some(r) = self.results.as_mut().and_then(|r| r.step(back)) else {
            self.message = Some(self.t("No search results.", "찾기 결과가 없습니다.").into());
            return;
        };
        if let Some(res) = self.results.as_mut() {
            res.open = true;
        }
        self.open_result(r);
    }

    /// Copies every result line of the newest search (`FIF-07`), or its file paths.
    pub fn copy_results(&mut self, paths: bool) {
        let Some(res) = &self.results else {
            return;
        };
        let Some(block) = res.blocks.first() else {
            return;
        };
        let mut lines = Vec::new();
        for f in &block.files {
            if paths {
                if let Some(p) = &f.path {
                    lines.push(p.display().to_string());
                }
            } else {
                for h in &f.hits {
                    lines.push(format!(
                        "{}:{}:{}: {}",
                        f.path
                            .as_ref()
                            .map_or_else(|| f.label.clone(), |p| p.display().to_string()),
                        h.line,
                        h.column,
                        h.line_text
                    ));
                }
            }
        }
        let n = lines.len();
        self.clipboard.set_text(&lines.join("\r\n"));
        self.message = Some(format!("{} {n}", self.t("Copied lines:", "복사한 줄:")));
    }

    /// Copies the selected row.
    pub fn copy_selected_result(&mut self) {
        let Some(res) = &self.results else {
            return;
        };
        let Some(r) = res.selected else {
            return;
        };
        let s = res.row_text(r);
        self.clipboard.set_text(&s);
    }
}

fn file_path(res: &Option<Results>, r: Row) -> Option<PathBuf> {
    let Row::Hit(b, f, _) = r else {
        return None;
    };
    res.as_ref()?.blocks.get(b)?.files.get(f)?.path.clone()
}

impl JarakApp {
    /// The results panel (above the status bar): a header with the newest search's summary and
    /// the copy/clear/close buttons, then the rows (virtualized).
    pub(crate) fn results_ui(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let Some(res) = self.results.as_ref() else {
            return;
        };
        if !res.open {
            return;
        }
        let lang = self.lang;
        let rows = res.rows();
        let mut act: Option<&'static str> = None;
        let mut clicked: Option<(Row, bool)> = None;
        let mut menu: Option<(&'static str, Row)> = None;
        // Dense like the editor's lines, at the smallest pointer target (24 points).
        let row_h = size::CONTROL_COMPACT;
        egui::Panel::bottom("jarak-results")
            .resizable(true)
            .default_size(220.0)
            .min_size(size::CONTROL * 3.0)
            .frame(
                egui::Frame::NONE
                    .fill(c.bg_raised)
                    .inner_margin(egui::Margin::symmetric(space::S as i8, space::XS as i8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(lang.pick("Search results", "찾기 결과"))
                            .size(font::CAPTION)
                            .strong()
                            .color(c.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            act = Some("close");
                        }
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Clear", "지우기")))
                            .clicked()
                        {
                            act = Some("clear");
                        }
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Copy paths", "경로 복사")))
                            .clicked()
                        {
                            act = Some("copy-paths");
                        }
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Copy lines", "줄 복사")))
                            .clicked()
                        {
                            act = Some("copy-lines");
                        }
                    });
                });
                let Some(res) = self.results.as_mut() else {
                    return;
                };
                let reveal = std::mem::take(&mut res.reveal)
                    .then(|| res.selected.and_then(|s| rows.iter().position(|r| *r == s)))
                    .flatten();
                let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                if let Some(k) = reveal {
                    area = area.vertical_scroll_offset((k as f32 * row_h - 2.0 * row_h).max(0.0));
                }
                // Rows touch (a dense listing); `show_rows` adds the spacing to each row.
                ui.spacing_mut().item_spacing.y = 0.0;
                area.show_rows(ui, row_h, rows.len(), |ui, range| {
                    for k in range {
                        let r = rows[k];
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_h),
                            egui::Sense::click(),
                        );
                        let selected = res.selected == Some(r);
                        if selected {
                            ui.painter().rect_filled(rect, 2.0, c.accent_soft);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(rect, 2.0, c.bg_hover);
                        }
                        let p = ui.painter_at(rect);
                        let mono = egui::FontId::monospace(font::BODY);
                        let ui_font = kim_ui::ui_font(ui.ctx(), font::BODY, false);
                        let y = rect.center().y;
                        match r {
                            Row::Search(b) => {
                                let blk = &res.blocks[b];
                                let tri = if blk.collapsed { "▸" } else { "▾" };
                                p.text(
                                    egui::pos2(rect.left() + space::XS, y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{tri} {}", blk.title),
                                    kim_ui::ui_font(ui.ctx(), font::BODY, true),
                                    c.text_primary,
                                );
                            }
                            Row::File(b, f) => {
                                let file = &res.blocks[b].files[f];
                                let tri = if file.collapsed { "▸" } else { "▾" };
                                p.text(
                                    egui::pos2(rect.left() + space::L, y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{tri} {}", file.label),
                                    ui_font.clone(),
                                    c.info,
                                );
                            }
                            Row::Hit(b, f, h) => {
                                let x = &res.blocks[b].files[f].hits[h];
                                let left = rect.left() + space::L * 2.0;
                                let prefix = format!("{:>6}: ", x.line);
                                let g = p.layout_no_wrap(prefix, mono.clone(), c.text_secondary);
                                let w = g.size().x;
                                p.galley(
                                    egui::pos2(left, y - g.size().y / 2.0),
                                    g,
                                    c.text_secondary,
                                );
                                // The line with its match highlighted.
                                let mut job = egui::text::LayoutJob::default();
                                let t = &x.line_text;
                                let (ma, mb) = (
                                    x.mark.0.min(t.len()),
                                    x.mark.1.min(t.len()).max(x.mark.0.min(t.len())),
                                );
                                let fmt = |col: Color32, bg: Color32| egui::TextFormat {
                                    font_id: mono.clone(),
                                    color: col,
                                    background: bg,
                                    ..Default::default()
                                };
                                job.append(
                                    &t[..ma],
                                    0.0,
                                    fmt(c.text_primary, Color32::TRANSPARENT),
                                );
                                job.append(
                                    &t[ma..mb],
                                    0.0,
                                    fmt(c.text_primary, crate::find::style_color(c, 1)),
                                );
                                job.append(
                                    &t[mb..],
                                    0.0,
                                    fmt(c.text_primary, Color32::TRANSPARENT),
                                );
                                let g = p.layout_job(job);
                                p.galley(
                                    egui::pos2(left + w, y - g.size().y / 2.0),
                                    g,
                                    c.text_primary,
                                );
                            }
                        }
                        if kim_ui::inspect::active() {
                            kim_ui::inspect::record(
                                ui,
                                &format!("results:row#{k}"),
                                "tree-row",
                                rect,
                                Some(kim_ui::inspect::Painted {
                                    content: &res.row_text(r),
                                    rect,
                                    size: font::BODY,
                                    truncated: false,
                                }),
                                true,
                                if selected { &["selected"] } else { &[] },
                                Some(c.text_primary),
                                None,
                            );
                        }
                        // Screen readers (and tests) read the row's text.
                        let label = res.row_text(r);
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone())
                        });
                        if resp.double_clicked() {
                            clicked = Some((r, true));
                        } else if resp.clicked() {
                            clicked = Some((r, false));
                        }
                        // The row's menu (FIF-07): open, copy this line, every line, the paths.
                        resp.context_menu(|ui| {
                            if matches!(r, Row::Hit(..))
                                && kim_ui::menu_item(ui, lang.pick("Open", "열기"), false).clicked()
                            {
                                menu = Some(("open", r));
                                ui.close();
                            }
                            for (id, en, ko) in [
                                ("copy-one", "Copy this line", "이 줄 복사"),
                                ("copy-lines", "Copy all lines", "모든 줄 복사"),
                                ("copy-paths", "Copy the paths", "경로 복사"),
                            ] {
                                if kim_ui::menu_item(ui, lang.pick(en, ko), false).clicked() {
                                    menu = Some((id, r));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            });
        if let Some((r, double)) = clicked {
            let res = self.results.as_mut().expect("drawn");
            res.selected = Some(r);
            match r {
                Row::Search(b) => res.blocks[b].collapsed ^= true,
                Row::File(b, f) if double => res.blocks[b].files[f].collapsed ^= true,
                Row::Hit(..) if double => self.open_result(r),
                _ => {}
            }
        }
        if let Some((what, r)) = menu {
            if let Some(res) = self.results.as_mut() {
                res.selected = Some(r);
            }
            match what {
                "open" => self.open_result(r),
                "copy-one" => self.copy_selected_result(),
                "copy-lines" => self.copy_results(false),
                _ => self.copy_results(true),
            }
        }
        match act {
            Some("close") => {
                if let Some(r) = self.results.as_mut() {
                    r.open = false;
                }
            }
            Some("clear") => self.results = None,
            Some("copy-lines") => self.copy_results(false),
            Some("copy-paths") => self.copy_results(true),
            _ => {}
        }
    }

    /// The Replace in Files preview: every change with a check box; Replace applies the checked
    /// ones.
    pub(crate) fn preview_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(p) = self.preview.as_mut() else {
            return;
        };
        let lang = self.lang;
        let mut act = None;
        let (n, files) = p.chosen();
        // Enter and Esc are read before the dialog draws (a focused check box would take Enter).
        let (enter, esc) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        kim_ui::dialog_sized(
            ctx,
            "jarak-replace-preview",
            lang.pick("Replace in Files", "파일에서 바꾸기"),
            640.0,
            |ui| {
                ui.label(
                    RichText::new(format!(
                        "\"{}\" → \"{}\" · {} {}",
                        p.find,
                        p.replace,
                        n,
                        lang.pick("changes checked", "개 변경 선택됨")
                    ))
                    .size(font::CAPTION)
                    .color(c.text_secondary),
                );
                ui.add_space(space::S);
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for (i, plan) in p.plans.iter().enumerate() {
                            let all = p.keep[i].iter().all(|x| *x);
                            let mut file_on = all;
                            if kim_ui::checkbox(
                                ui,
                                &mut file_on,
                                &format!("{} ({})", plan.path.display(), plan.changes.len()),
                            )
                            .changed()
                            {
                                p.keep[i].iter_mut().for_each(|x| *x = file_on);
                            }
                            for (k, ch) in plan.changes.iter().enumerate().take(500) {
                                ui.horizontal(|ui| {
                                    ui.add_space(space::L);
                                    kim_ui::checkbox(
                                        ui,
                                        &mut p.keep[i][k],
                                        &format!("{}:{}", ch.hit.line, ch.hit.column),
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&ch.hit.line_text)
                                                .monospace()
                                                .size(font::CAPTION)
                                                .color(c.text_secondary),
                                        );
                                        ui.label(
                                            RichText::new(&ch.new_line)
                                                .monospace()
                                                .size(font::CAPTION)
                                                .color(c.text_primary),
                                        );
                                    });
                                });
                            }
                        }
                        if !p.skipped.is_empty() {
                            ui.add_space(space::S);
                            for (path, s) in &p.skipped {
                                ui.label(
                                    RichText::new(format!("{} — {}", path.display(), s.message()))
                                        .size(font::CAPTION)
                                        .color(c.text_muted),
                                );
                            }
                        }
                    });
                ui.add_space(space::S);
                ui.label(
                RichText::new(lang.pick(
                    "Each file is saved in its own encoding and line endings; the originals go to a journal (Undo the last Replace in Files).",
                    "각 파일은 원래 인코딩과 줄 끝으로 저장되고, 원본은 기록에 남습니다 (마지막 파일 바꾸기 되돌리기).",
                ))
                .size(font::CAPTION)
                .color(c.text_muted),
            );
                kim_ui::dialog_actions(ui, |ui| {
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                        .clicked()
                    {
                        act = Some(false);
                    }
                    let label = match lang {
                        Lang::Ko => format!("바꾸기: 파일 {files}개에서 {n}개"),
                        _ => format!(
                            "Replace {n} in {files} file{}",
                            if files == 1 { "" } else { "s" }
                        ),
                    };
                    if ui
                        .add_enabled(n > 0, kim_ui::Button::primary(&label))
                        .clicked()
                    {
                        act = Some(true);
                    }
                });
            },
        );
        if esc {
            act = Some(false);
        } else if n > 0 && enter && act.is_none() {
            // Enter replaces the checked changes (the dialog's default button).
            act = Some(true);
        }
        match act {
            Some(true) => self.apply_preview(),
            Some(false) => self.preview = None,
            None => {}
        }
    }
}
