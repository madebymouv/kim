//! Step 111: the Help menu (HELP-11) — the user guide (opened read-only in the preview), About
//! Jarak (version, programs, folders, licences, the debug information) — and offering Jarak in
//! Windows' *Open with* list (SET-09: explicit, per user, reversible, never a default program).

use crate::JarakApp;
use eframe::egui;
use std::path::{Path, PathBuf};

/// An open Help dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpDialog {
    About,
    /// The extensions to offer, each checked or not.
    Offer(Vec<(String, bool)>),
}

/// Where the user guide is: `docs\JARAK_GUIDE.md` beside the program (the package), or the
/// repository's `docs` two folders up (a development build in `target\release`).
pub fn guide_path_from(exe_dir: &Path) -> Option<PathBuf> {
    [
        exe_dir.join("docs").join("JARAK_GUIDE.md"),
        exe_dir
            .join("..")
            .join("..")
            .join("docs")
            .join("JARAK_GUIDE.md"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// The extensions the offer dialog starts with checked.
const CHECKED: &[&str] = &["txt", "md", "markdown", "log"];

impl JarakApp {
    /// The Help commands: 0 the user guide, 1 About, 2 the offer dialog, 3 take the offer back.
    pub fn help_tool(&mut self, k: u8) {
        match k {
            0 => self.open_guide(),
            1 => self.help_dialog = Some(HelpDialog::About),
            2 => {
                if self.assoc.is_none() {
                    self.message = Some(
                        self.t("Not available in this window", "이 창에서는 쓸 수 없습니다")
                            .into(),
                    );
                    return;
                }
                let offered = self.offered_exts();
                self.help_dialog = Some(HelpDialog::Offer(
                    jarak_core::assoc::DEFAULT_EXTS
                        .iter()
                        .map(|e| {
                            (
                                (*e).to_string(),
                                CHECKED.contains(e) || offered.iter().any(|o| o == e),
                            )
                        })
                        .collect(),
                ));
            }
            _ => self.take_back_offer(),
        }
    }

    /// Opens the user guide read-only, in the preview.
    fn open_guide(&mut self) {
        let dir = self.guide_dir.clone().or_else(|| {
            std::env::current_exe()
                .ok()?
                .parent()
                .map(Path::to_path_buf)
        });
        let Some(p) = dir.as_deref().and_then(guide_path_from) else {
            self.message = Some(
                self.t(
                    "The user guide is not beside this program (docs\\JARAK_GUIDE.md)",
                    "이 프로그램 옆에 사용자 안내서가 없습니다(docs\\JARAK_GUIDE.md)",
                )
                .into(),
            );
            return;
        };
        let p = std::path::absolute(&p).unwrap_or(p);
        self.open_target(&jarak_core::target::OpenTarget {
            path: p.clone(),
            line: None,
            column: None,
        });
        if self.tab().file.path.as_deref() == Some(p.as_path()) {
            let reason = self.t("the user guide", "사용자 안내서").to_string();
            let t = self.tab_mut();
            t.file.read_only = true;
            t.file.read_only_reason = Some(reason);
            t.user_read_only = false;
            t.set_mode(crate::Mode::Preview);
        }
    }

    /// The extensions Jarak is offered for now.
    fn offered_exts(&self) -> Vec<String> {
        let (Some(store), Some(file)) = (self.assoc.as_ref(), self.assoc_record.as_ref()) else {
            return Vec::new();
        };
        jarak_core::assoc::Record::read(file)
            .and_then(|r| jarak_core::assoc::offered(store.as_ref(), &r).ok())
            .unwrap_or_default()
    }

    /// Offers Jarak for `exts` (the dialog's Offer).
    fn make_offer(&mut self, exts: &[String]) {
        let (Some(store), Some(file)) = (self.assoc.as_mut(), self.assoc_record.clone()) else {
            return;
        };
        let program = self
            .assoc_program
            .clone()
            .or_else(|| std::env::current_exe().ok())
            .unwrap_or_default();
        let rec = jarak_core::assoc::Record::read(&file);
        let r = jarak_core::assoc::offer(store.as_mut(), rec.as_ref(), &program, exts, false)
            .and_then(|(_, rec)| rec.write(&file));
        let list = exts
            .iter()
            .map(|e| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.message = Some(match r {
            Ok(()) => format!(
                "{} {list}. {}",
                self.t(
                    "Jarak is in Windows' Open with list for",
                    "Windows의 연결 프로그램 목록에 자락을 넣었습니다:"
                ),
                self.t(
                    "The default program stays your choice (Settings → Apps → Default apps).",
                    "기본 프로그램은 그대로입니다(설정 → 앱 → 기본 앱에서 고릅니다).",
                )
            ),
            Err(e) => format!("{}: {e}", self.t("Not offered", "넣지 못했습니다")),
        });
    }

    /// Takes back what the offer added.
    fn take_back_offer(&mut self) {
        let (Some(store), Some(file)) = (self.assoc.as_mut(), self.assoc_record.clone()) else {
            self.message = Some(
                self.t("Not available in this window", "이 창에서는 쓸 수 없습니다")
                    .into(),
            );
            return;
        };
        let Some(rec) = jarak_core::assoc::Record::read(&file) else {
            self.message = Some(
                self.t(
                    "Jarak was not offered in Open with; nothing to remove",
                    "연결 프로그램 목록에 자락을 넣은 적이 없어 지울 것이 없습니다",
                )
                .into(),
            );
            return;
        };
        let r = jarak_core::assoc::remove(store.as_mut(), &rec, false);
        if r.is_ok() {
            let _ = std::fs::remove_file(&file);
        }
        self.message = Some(match r {
            Ok(_) => self
                .t(
                    "Jarak is no longer in Windows' Open with list",
                    "Windows의 연결 프로그램 목록에서 자락을 뺐습니다",
                )
                .into(),
            Err(e) => format!("{}: {e}", self.t("Not removed", "빼지 못했습니다")),
        });
    }

    /// The Help dialogs.
    pub(crate) fn help_dialog_ui(&mut self, ctx: &egui::Context) {
        use kim_design::space;
        let Some(dlg) = self.help_dialog.clone() else {
            return;
        };
        let lang = self.lang;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        match dlg {
            HelpDialog::About => {
                let mut copy = false;
                let mut guide = false;
                let title = self.t("About Jarak", "자락 정보").to_string();
                let exe = std::env::current_exe()
                    .map_or_else(|_| "?".to_string(), |e| e.display().to_string());
                let paths = jarak_core::paths::Paths::from_env();
                kim_ui::dialog_sized(ctx, "jarak-about", &title, 520.0, |ui| {
                    let rows: Vec<(String, String)> = {
                        let mut v = vec![
                            (
                                lang.pick("Version", "버전").to_string(),
                                format!("Jarak {}", jarak_core::VERSION),
                            ),
                            (lang.pick("Program", "프로그램").to_string(), exe.clone()),
                        ];
                        if let Some(p) = &paths {
                            v.push((
                                lang.pick("Settings", "설정").to_string(),
                                p.settings_file().display().to_string(),
                            ));
                            v.push((
                                lang.pick("Session", "세션").to_string(),
                                p.session_file().display().to_string(),
                            ));
                        }
                        v.push((
                            lang.pick("Licence", "라이선스").to_string(),
                            lang.pick(
                                "MIT or Apache-2.0; third-party notices in THIRD-PARTY-NOTICES.txt",
                                "MIT 또는 Apache-2.0. 외부 구성 요소는 THIRD-PARTY-NOTICES.txt",
                            )
                            .to_string(),
                        ));
                        v.push((
                            lang.pick("Privacy", "개인 정보").to_string(),
                            lang.pick(
                                "No account, no telemetry, no network calls you did not ask for",
                                "계정·원격 측정이 없고, 요청하지 않은 네트워크 연결을 하지 않습니다",
                            )
                            .to_string(),
                        ));
                        v
                    };
                    for (i, (k, v)) in rows.iter().enumerate() {
                        if i > 0 {
                            kim_ui::row_divider(ui);
                        }
                        kim_ui::setting_row(ui, k, Some(v), |_| {});
                    }
                    ui.add_space(space::S);
                    kim_ui::dialog_actions(ui, |ui| {
                        if ui
                            .add(kim_ui::Button::primary(lang.pick("Close", "닫기")))
                            .clicked()
                        {
                            close = true;
                        }
                        if ui
                            .add(kim_ui::Button::ghost(
                                lang.pick("Copy debug info", "디버그 정보 복사"),
                            ))
                            .clicked()
                        {
                            copy = true;
                        }
                        if ui
                            .add(kim_ui::Button::ghost(
                                lang.pick("User guide", "사용자 안내서"),
                            ))
                            .clicked()
                        {
                            guide = true;
                        }
                    });
                });
                if copy {
                    self.run_entry(crate::palette::Run::DebugInfo);
                }
                if guide {
                    close = true;
                    self.open_guide();
                }
                if close {
                    self.help_dialog = None;
                }
            }
            HelpDialog::Offer(mut exts) => {
                let mut offer = false;
                let title = self
                    .t(
                        "Offer Jarak in \"Open with\"",
                        "연결 프로그램 목록에 자락 넣기",
                    )
                    .to_string();
                kim_ui::dialog_sized(ctx, "jarak-offer", &title, 520.0, |ui| {
                    ui.label(kim_ui::secondary(
                        ui,
                        lang.pick(
                            "Jarak is added to Windows' Open with list for the checked extensions, for you only. Your default programs do not change: Windows lets you choose them (Settings → Apps → Default apps). Help → Remove from \"Open with\" takes it back.",
                            "체크한 확장자의 Windows 연결 프로그램 목록에 자락을 넣습니다(이 사용자만). 기본 프로그램은 바뀌지 않습니다. 기본값은 Windows 설정 → 앱 → 기본 앱에서 고릅니다. 도움말 → 연결 프로그램 목록에서 빼기로 되돌립니다.",
                        ),
                    ));
                    ui.add_space(space::S);
                    egui::Grid::new("jarak-offer-exts")
                        .num_columns(4)
                        .show(ui, |ui| {
                            for (i, (e, on)) in exts.iter_mut().enumerate() {
                                kim_ui::checkbox(ui, on, &format!(".{e}"));
                                if i % 4 == 3 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(space::S);
                    kim_ui::dialog_actions(ui, |ui| {
                        let any = exts.iter().any(|(_, on)| *on);
                        if ui
                            .add_enabled(any, kim_ui::Button::primary(lang.pick("Offer", "넣기")))
                            .clicked()
                        {
                            offer = true;
                        }
                        if ui
                            .add(kim_ui::Button::ghost(lang.pick("Cancel", "취소")))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                if offer {
                    let chosen: Vec<String> = exts
                        .iter()
                        .filter(|(_, on)| *on)
                        .map(|(e, _)| e.clone())
                        .collect();
                    self.make_offer(&chosen);
                    close = true;
                }
                self.help_dialog = if close {
                    None
                } else {
                    Some(HelpDialog::Offer(exts))
                };
            }
        }
    }
}
