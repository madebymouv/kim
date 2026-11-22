//! The window's Edit tools of Step 97 that need more than the document (`JARAK-FR-087`,
//! `JARAK-FR-088`): the date and time (Windows' short and long formats of the user's locale, or a
//! custom format from `settings.toml`'s `date_format`), copying the document's path, name or folder
//! and those of every open document, digests of the selection or of a file to the clipboard, the
//! digest dialog (type text, see its MD5 / SHA-1 / SHA-256 / SHA-512) and the copy-line setting.

use crate::{JarakApp, read_setting, write_setting};
use eframe::egui::{self, RichText};
use jarak_core::lineops::{DateTime, Hash, format_datetime};
use kim_design::{font, space};
use kim_ui::Colors;

/// The digest dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashDialog {
    pub text: String,
    pub hash: HashKind,
    pub focus: bool,
}

/// [`Hash`] as a value the segmented control can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashKind {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl HashKind {
    pub fn hash(self) -> Hash {
        match self {
            HashKind::Md5 => Hash::Md5,
            HashKind::Sha1 => Hash::Sha1,
            HashKind::Sha256 => Hash::Sha256,
            HashKind::Sha512 => Hash::Sha512,
        }
    }
}

/// The local date and time.
pub fn now() -> DateTime {
    #[cfg(windows)]
    #[allow(unsafe_code)]
    {
        // SAFETY: GetLocalTime fills a plain struct.
        let t = unsafe {
            let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
            t
        };
        DateTime {
            year: i32::from(t.wYear),
            month: u32::from(t.wMonth),
            day: u32::from(t.wDay),
            hour: u32::from(t.wHour),
            minute: u32::from(t.wMinute),
            second: u32::from(t.wSecond),
        }
    }
    #[cfg(not(windows))]
    {
        DateTime {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

/// The time and date in the user's short or long Windows format (as Notepad++ inserts them).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn system_date_time(long: bool) -> String {
    use windows_sys::Win32::Globalization::{
        DATE_LONGDATE, DATE_SHORTDATE, GetDateFormatEx, GetTimeFormatEx, TIME_NOSECONDS,
    };
    let mut date = vec![0u16; 128];
    let mut time = vec![0u16; 64];
    // SAFETY: the buffers outlive the calls; null locale = the user's, null time = now.
    let (d, t) = unsafe {
        (
            GetDateFormatEx(
                std::ptr::null(),
                if long { DATE_LONGDATE } else { DATE_SHORTDATE },
                std::ptr::null(),
                std::ptr::null(),
                date.as_mut_ptr(),
                date.len() as i32,
                std::ptr::null(),
            ),
            GetTimeFormatEx(
                std::ptr::null(),
                if long { 0 } else { TIME_NOSECONDS },
                std::ptr::null(),
                std::ptr::null(),
                time.as_mut_ptr(),
                time.len() as i32,
            ),
        )
    };
    let s = |b: &[u16], n: i32| String::from_utf16_lossy(&b[..(n.max(1) as usize - 1)]);
    format!("{} {}", s(&time, t), s(&date, d))
}

#[cfg(not(windows))]
pub fn system_date_time(_long: bool) -> String {
    format_datetime("HH:mm yyyy-MM-dd", now())
}

impl JarakApp {
    /// The Step 97 window tools (`palette::Run::Edit`): 0–2 insert the date and time (short,
    /// long, custom), 3–7 copy the path / name / folder / every name / every path, 8–11 the
    /// selection's MD5 / SHA-1 / SHA-256 / SHA-512 to the clipboard, 12–15 a file's, 16 the digest
    /// dialog, 17 the copy-line setting.
    pub fn edit_tool(&mut self, k: u8) {
        let hash_of =
            |i: u8| [Hash::Md5, Hash::Sha1, Hash::Sha256, Hash::Sha512][usize::from(i % 4)];
        match k {
            0..=2 => {
                if self.tab().file.read_only {
                    self.refuse_read_only();
                    return;
                }
                let s = match k {
                    0 => system_date_time(false),
                    1 => system_date_time(true),
                    _ => {
                        let fmt = self
                            .settings
                            .as_deref()
                            .and_then(|p| read_setting(p, "date_format"))
                            .unwrap_or_else(|| "yyyy-MM-dd HH:mm:ss".to_string());
                        format_datetime(&fmt, now())
                    }
                };
                self.paste(&s);
            }
            3..=7 => {
                let text = match k {
                    3..=5 => {
                        let Some(p) = self.tab().file.path.clone() else {
                            self.message = Some(
                                self.t(
                                    "The document has no file yet.",
                                    "문서에 아직 파일이 없습니다.",
                                )
                                .into(),
                            );
                            return;
                        };
                        match k {
                            3 => p.display().to_string(),
                            4 => p
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            _ => p
                                .parent()
                                .map(|d| d.display().to_string())
                                .unwrap_or_default(),
                        }
                    }
                    6 => self
                        .tabs
                        .iter()
                        .map(|t| t.file.title())
                        .collect::<Vec<_>>()
                        .join("\r\n"),
                    _ => self
                        .tabs
                        .iter()
                        .map(|t| {
                            t.file
                                .path
                                .as_ref()
                                .map_or_else(|| t.file.title(), |p| p.display().to_string())
                        })
                        .collect::<Vec<_>>()
                        .join("\r\n"),
                };
                self.clipboard.set_text(&text);
                self.message = Some(format!("{} {text}", self.t("Copied:", "복사함:")));
            }
            8..=11 => {
                let s = self.tab().file.doc.selected_text();
                if s.is_empty() {
                    self.message = Some(
                        self.t("Select the text first.", "먼저 텍스트를 선택하세요.")
                            .into(),
                    );
                    return;
                }
                let h = hash_of(k - 8);
                let d = h.hex(s.as_bytes());
                self.clipboard.set_text(&d);
                self.message = Some(format!("{} {d}", h.name()));
            }
            12..=15 => {
                let Some(p) = self.dialogs.open() else {
                    return;
                };
                let h = hash_of(k - 12);
                match std::fs::read(&p) {
                    Ok(b) => {
                        let d = h.hex(&b);
                        let line = format!(
                            "{d}  {}",
                            p.file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        );
                        self.clipboard.set_text(&line);
                        self.message = Some(format!("{} {line}", h.name()));
                    }
                    Err(e) => self.message = Some(format!("{}: {e}", p.display())),
                }
            }
            16 => {
                self.hash_dialog = Some(HashDialog {
                    text: self.tab().file.doc.selected_text(),
                    hash: HashKind::Sha256,
                    focus: true,
                });
            }
            _ => {
                self.copy_line = !self.copy_line;
                if let Some(p) = &self.settings {
                    let _ = write_setting(
                        p,
                        "copy_line",
                        if self.copy_line { "true" } else { "false" },
                    );
                }
                self.message = Some(if self.copy_line {
                    self.t(
                        "Copy without a selection takes the line: on",
                        "선택 없이 복사하면 줄 복사: 켬",
                    )
                    .to_string()
                } else {
                    self.t(
                        "Copy without a selection takes the line: off",
                        "선택 없이 복사하면 줄 복사: 끔",
                    )
                    .to_string()
                });
            }
        }
    }

    /// The digest dialog: the text, the algorithm, its digest (updated as it is typed), Copy.
    pub(crate) fn hash_dialog_ui(&mut self, ctx: &egui::Context, c: &Colors) {
        let lang = self.lang;
        let Some(d) = self.hash_dialog.as_mut() else {
            return;
        };
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut act = None;
        kim_ui::dialog_sized(
            ctx,
            "jarak-hash",
            lang.pick("Generate a digest", "다이제스트 만들기"),
            520.0,
            |ui| {
                kim_ui::segmented(
                    ui,
                    &mut d.hash,
                    &[
                        (HashKind::Md5, "MD5"),
                        (HashKind::Sha1, "SHA-1"),
                        (HashKind::Sha256, "SHA-256"),
                        (HashKind::Sha512, "SHA-512"),
                    ],
                );
                ui.add_space(space::S);
                let r = kim_ui::text_area(
                    ui,
                    &mut d.text,
                    lang.pick("Text (UTF-8)", "텍스트(UTF-8)"),
                    "",
                    4,
                );
                if d.focus {
                    r.request_focus();
                    d.focus = false;
                }
                ui.add_space(space::S);
                let digest = d.hash.hash().hex(d.text.as_bytes());
                let l = ui.add(
                    egui::Label::new(
                        RichText::new(&digest)
                            .monospace()
                            .size(font::CAPTION)
                            .color(c.text_primary),
                    )
                    .wrap(),
                );
                if kim_ui::inspect::active() {
                    kim_ui::inspect::record(
                        ui,
                        "hash:digest",
                        "label",
                        l.rect,
                        Some(kim_ui::inspect::Painted {
                            content: &digest,
                            rect: l.rect,
                            size: font::CAPTION,
                            truncated: false,
                        }),
                        false,
                        &[],
                        Some(c.text_primary),
                        None,
                    );
                }
                kim_ui::dialog_actions(ui, |ui| {
                    if ui
                        .add(kim_ui::Button::ghost(lang.pick("Close", "닫기")))
                        .clicked()
                    {
                        act = Some(false);
                    }
                    if ui
                        .add(kim_ui::Button::primary(lang.pick("Copy", "복사")))
                        .clicked()
                    {
                        act = Some(true);
                    }
                });
            },
        );
        if esc {
            act = Some(false);
        }
        match act {
            Some(true) => {
                if let Some(d) = &self.hash_dialog {
                    let h = d.hash.hash();
                    let digest = h.hex(d.text.as_bytes());
                    self.clipboard.set_text(&digest);
                    self.message = Some(format!("{} {digest}", h.name()));
                }
                self.hash_dialog = None;
            }
            Some(false) => self.hash_dialog = None,
            None => {}
        }
    }
}
