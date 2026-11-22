//! Printing from the window (`JARAK-FR-140`, Step 106): File → Print… (the Windows print
//! dialog: printer, copies, all pages, the selection or a page range), Print Now (the default
//! printer, no dialog) and Export as PDF… (Microsoft Print to PDF into the file the Save dialog
//! names).
//!
//! - The job prints the text as it is in the window, saved or not; a Markdown document prints
//!   rendered, with its local images; everything else as highlighted text with the print settings
//!   (Settings → Files → Printing).
//! - A printer that writes a file (Microsoft Print to PDF, the XPS writer) gets its file from
//!   Jarak's Save dialog, which asks before it replaces one, so the driver never asks on its own.
//! - The job runs on a thread; the status bar says when it is done or why it failed.

use crate::JarakApp;
use jarak_core::export::Format;
use jarak_core::print::{self as pr, Job, PrintError, Printed};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// What the print dialog chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintChoice {
    pub printer: String,
    /// The dialog's settings for that printer (`DEVMODEW` bytes: copies, paper, orientation).
    pub devmode: Option<Vec<u8>>,
    pub range: Range,
}

/// Which pages a job prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Range {
    #[default]
    All,
    /// Only the selected text.
    Selection,
    /// Pages `from..=to` (1-based).
    Pages(u32, u32),
}

/// A job on its way to the printer.
#[derive(Debug)]
pub struct Running {
    pub printer: String,
    pub output: Option<PathBuf>,
    pub result: Arc<Mutex<Option<Result<Printed, PrintError>>>>,
}

impl JarakApp {
    /// File → Print…: the print dialog, then the job.
    pub fn print_dialog(&mut self) {
        if self.printing.is_some() {
            self.message = Some(
                self.t("A print job is still running", "인쇄가 아직 진행 중입니다")
                    .into(),
            );
            return;
        }
        let has_selection = !self.tab().file.doc.selected_text().is_empty();
        let Some(choice) = self.dialogs.print(has_selection) else {
            return;
        };
        self.print_with(choice);
    }

    /// Prints the active document as the print dialog chose (a file-writing printer's file asked
    /// first).
    pub fn print_with(&mut self, choice: PrintChoice) {
        let Some(output) = self.output_for(&choice.printer) else {
            return;
        };
        self.start_print(
            Job {
                printer: choice.printer,
                output,
                devmode: choice.devmode,
                ..Job::default()
            },
            choice.range,
        );
    }

    /// File → Print Now: the default printer, no dialog.
    pub fn print_now(&mut self) {
        if self.printing.is_some() {
            self.message = Some(
                self.t("A print job is still running", "인쇄가 아직 진행 중입니다")
                    .into(),
            );
            return;
        }
        let Some(p) = self.printers.list().into_iter().find(|p| p.default) else {
            self.message = Some(
                self.t(
                    "There is no default printer (Windows Settings → Printers)",
                    "기본 프린터가 없습니다(Windows 설정 → 프린터)",
                )
                .into(),
            );
            return;
        };
        let Some(output) = self.output_for(&p.name) else {
            return;
        };
        self.start_print(
            Job {
                printer: p.name,
                output,
                ..Job::default()
            },
            Range::All,
        );
    }

    /// File → Export as PDF…: Microsoft Print to PDF into the file the Save dialog names.
    pub fn export_pdf(&mut self) {
        if self.printing.is_some() {
            self.message = Some(
                self.t("A print job is still running", "인쇄가 아직 진행 중입니다")
                    .into(),
            );
            return;
        }
        if !self
            .printers
            .list()
            .iter()
            .any(|p| p.name == pr::PDF_PRINTER)
        {
            self.message = Some(
                self.t(
                    "Microsoft Print to PDF is not installed (Windows optional features)",
                    "Microsoft Print to PDF가 설치되어 있지 않습니다(Windows 선택적 기능)",
                )
                .into(),
            );
            return;
        }
        let suggested = self.export_name("pdf");
        let Some(out) = self.dialogs.save_export(&suggested, Format::Pdf) else {
            return;
        };
        if let Some(src) = self.tab().file.path.as_deref()
            && jarak_core::export::same_file(&out, src)
        {
            self.message = Some(
                self.t(
                    "Not exported: an export never replaces the document itself",
                    "내보내지 않음: 내보내기는 문서 자신을 바꾸지 않습니다",
                )
                .into(),
            );
            return;
        }
        self.start_print(
            Job {
                printer: pr::PDF_PRINTER.into(),
                output: Some(out),
                ..Job::default()
            },
            Range::All,
        );
    }

    /// The file a file-writing printer writes (asked with the Save dialog); `Some(None)` for a
    /// real printer, `None` when the user cancelled.
    fn output_for(&mut self, printer: &str) -> Option<Option<PathBuf>> {
        let writes = self
            .printers
            .list()
            .iter()
            .any(|p| p.name == printer && p.writes_a_file());
        if !writes {
            return Some(None);
        }
        let suggested = self.export_name("pdf");
        let out = self.dialogs.save_export(&suggested, Format::Pdf)?;
        Some(Some(out))
    }

    /// Lays out and prints the active document on a thread.
    fn start_print(&mut self, mut job: Job, range: Range) {
        let i = self.active;
        let markdown = self.tab_is_markdown(i);
        let t = &self.tabs[i];
        let text = if range == Range::Selection {
            t.file.doc.selected_text()
        } else {
            t.file.doc.text().to_string()
        };
        let doc = t.file.path.clone();
        let title = t.title().trim_start_matches('*').trim().to_string();
        job.name = format!("Jarak — {title}");
        let lang = self.lang_for(i).cloned();
        let setup = self.prefs.print.clone();
        let (date, time) = pr::local_now();
        let vars = pr::Vars {
            file: title,
            path: doc
                .as_deref()
                .map_or_else(String::new, |p| p.display().to_string()),
            date,
            time,
        };
        let result = Arc::new(Mutex::new(None));
        self.printing = Some(Running {
            printer: job.printer.clone(),
            output: job.output.clone(),
            result: result.clone(),
        });
        self.message = Some(format!(
            "{} {}…",
            self.t("Printing on", "인쇄 중:"),
            job.printer
        ));
        let printers = self.printers.clone();
        std::thread::spawn(move || {
            let mut layout = |m: &dyn pr::Measure, g: &pr::Geometry| {
                let pages = if markdown && range != Range::Selection {
                    pr::markdown_pages(&text, doc.as_deref(), &setup, g, m, &vars)
                } else {
                    pr::code_pages(&text, lang.as_ref(), &setup, g, m, &vars)
                };
                match range {
                    Range::Pages(a, b) => pr::page_range(pages, a as usize, b as usize),
                    _ => pages,
                }
            };
            let r = printers.print(&job, &mut layout);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(r);
            }
        });
    }

    /// Each frame: the running job's result, once it is there.
    pub(crate) fn print_frame(&mut self, ctx: &eframe::egui::Context) {
        let Some(run) = self.printing.as_ref() else {
            return;
        };
        let done = run.result.lock().ok().and_then(|mut r| r.take());
        let Some(r) = done else {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        };
        let run = self.printing.take();
        let printer = run.as_ref().map(|r| r.printer.clone()).unwrap_or_default();
        let ko = self.lang == kim_design::Lang::Ko;
        let plural = |n: usize| if n == 1 { "" } else { "s" };
        self.message = Some(match r {
            Ok(p) => match (p.output, ko) {
                (Some(o), false) => format!(
                    "Saved {} ({} page{})",
                    o.display(),
                    p.pages,
                    plural(p.pages)
                ),
                (Some(o), true) => format!("저장함: {} ({}쪽)", o.display(), p.pages),
                (None, false) => {
                    format!("Printed {} page{} on {printer}", p.pages, plural(p.pages))
                }
                (None, true) => format!("인쇄함: {printer}, {}쪽", p.pages),
            },
            Err(e) => format!("{}: {e}", self.t("Not printed", "인쇄하지 않음")),
        });
    }

    /// The suggested output name: the document's with `ext`, in its folder.
    pub(crate) fn export_name(&self, ext: &str) -> String {
        let t = self.tab();
        match t.file.path.as_deref() {
            Some(p) => p.with_extension(ext).to_string_lossy().into_owned(),
            None => format!("{}.{ext}", t.title().trim_start_matches('*').trim()),
        }
    }
}
