# Jarak — real Windows acceptance script

Status: **current** (Step 111; run on the real desktop 2026-11-23) · Run on a real Windows 10/11 desktop with a real keyboard and
mouse, the release build installed through Kim (or `cargo xtask package jarak`), test documents
in a folder of their own. Each row records **pass / fail / not run** with the date and machine; a
failing row is filed as an issue and blocks the release ([RELEASE_READINESS](RELEASE_READINESS.md)).
The automated evidence is listed so the manual run only checks what tests cannot: the real
windows, dialogs, clipboard, Explorer and the screen. Guide: [JARAK_GUIDE](JARAK_GUIDE.md) ·
Parity: [JARAK_PARITY_REPORT](JARAK_PARITY_REPORT.md).

## The script

| # | Check | How | Automated evidence |
| --- | --- | --- | --- |
| 1 | Install through Kim | Kim Manager → Jarak → *Install* (local mirror or catalog); the tile shows the version; *Open Jarak* opens the window; nothing else starts and nothing runs at sign-in | e2e `step111_jarak_release` (signed catalog, install, the installed `jarak`, uninstall), `step85` |
| 2 | First window | An empty untitled document, the status bar (Ln/Col, length, UTF-8, CRLF, Plain text, INS, the gear), no toolbar; Alt shows the menu bar, Ctrl+Shift+P the palette | window tests `the_status_bar_carries_the_gear…`, `alt_alone_shows_the_menu_bar…`; UI audit |
| 3 | Open, edit, save | Ctrl+O a UTF-8 file and a CP949 file; type Korean with the IME; Ctrl+S; reopen: the bytes are as expected (the CP949 file stays CP949) | `save_as_save_and_the_unsaved_changes_question`, Step 87 encoding tests, e2e `step87`, `step103` |
| 4 | Markdown and a screenshot | A `.md` note: Ctrl+V a screenshot → `assets\…png` and its Markdown image reference; Ctrl+E preview shows it; the note file changes only on save | `the_mvp_journey_paste_a_screenshot_save_preview_and_reopen`, e2e `step90` |
| 5 | Find and replace | Ctrl+F, Ctrl+H with a regular expression; Find in Files over the test folder; the preview of Replace in Files; `jarak journal undo` gives the bytes back | Step 94/95 window tests, e2e `step94`, `step95` |
| 6 | Several carets and columns | Ctrl+click carets, Alt+drag a column over Korean lines, the Column Editor | Step 96 window tests |
| 7 | Drag and drop of text | Drag a selection to another line (moved, one Ctrl+Z); hold Ctrl when dropping (copied) | `dragging_the_selection_moves_or_copies_it` |
| 8 | Window modes | View → Always on top (another window stays below), F11 full screen and back, F12 post-it (no frame or bars; F12 again restores), distraction-free (a centred column) | `window_modes_toggle_and_hide_the_chrome` (the real window: **this row**) |
| 9 | Context menu | Right-click the text: Undo, Redo, Cut, Copy, Paste, Select all, Copy as HTML; each works | `the_text_context_menu_runs_the_settings_commands` |
| 10 | Clipboard formats | *Copy as RTF* then paste into WordPad (formatted; WordPad reads RTF, not HTML); *Copy as HTML* read back from the clipboard by another program; copy formatted text from WordPad → *Paste RTF as source*, HTML put on the clipboard by another program → *Paste HTML as source* (the markup as text) | `code_exports_as_rtf_and_copies_as_html_and_rtf`, `pasting_html_and_rtf_as_source_inserts_their_text` (the real clipboard: **this row**) |
| 11 | Print and PDF | File → *Print…*: the Windows print dialog; *Microsoft Print to PDF* writes a PDF with the header, footer and line numbers | `printing_uses_the_dialogs_printer_settings_and_range`, e2e `step106` (PDF reviewed) |
| 12 | Crash recovery | Type in two documents without saving, end `jarak-gui.exe` in Task Manager; the next start offers both; *Restore* gives the text back | `recovery_kill.rs`, Step 104 real check |
| 13 | A file changed elsewhere | Change an open file in Notepad; Jarak asks; *Reload* is one Ctrl+Z; *Compare* opens the disk version | Step 104 window tests, real check |
| 14 | Settings | Ctrl+, : change the theme, the font size, word wrap, the backup on save (a `.bak` appears after a save); `jarak settings list` shows them | Step 105/110 window tests, e2e `step105`, `step110` |
| 15 | Open with offer | Help → *Offer Jarak in "Open with"…* for `.md` and `.txt`; Explorer → right-click a `.md` file → *Open with* lists Jarak, the default program is unchanged; Help → *Remove Jarak from "Open with"*; the entry is gone | `offering_jarak_in_open_with_and_taking_it_back`, core `the_real_registry_offer_and_removal_under_a_test_key`, CLI `associate_offers_refuses_and_takes_back`, e2e `step111` (dry runs) |
| 16 | Help | Help → *User guide* opens this guide read-only in the preview; *About Jarak* shows the version and copies the debug information | `the_help_menu_opens_the_guide_and_about`, `the_help_dialogs_pass_the_ui_audit_rules` |
| 17 | Headless | `jarak info`, `jarak find`, `jarak export --dry-run` in a terminal: no window opens, JSON with `--json` | e2e `step85`…`step110`, CLI tests |
| 18 | Kim interop | From a Subak terminal, `jarak open notes.md:12` brings the running window to the front at line 12; a Noeul capture inserted into a note through Kim (`markdown.insert-image`, consent asked) | e2e `step107` (the desktop demonstration: **this row**) |
| 19 | The other tools | Subak, Noeul and Ddalgak start from Kim Manager as before; their hotkeys still work | each tool's tests, `cargo xtask ci`, e2e `step111` (`--version`) |
| 20 | Uninstall | Kim Manager → Jarak → *Uninstall*: the programs go, `%APPDATA%\mouv\jarak` (settings) stays | e2e `step111` |

## Recorded runs

| Date | Machine | Rows | Result |
| --- | --- | --- | --- |
| 2026-11-23 | Development PC, Windows 10 Pro 22H2 (19045), one 5120×1440 monitor at 100 %, the maintainer away (ADR 0088: test-started windows, sandboxed profiles, the clipboard saved and restored, the registry exported and compared) | 1–20 | **pass**, with three faults found and fixed (below). Rows 1, 3 (dialogs), 4 (a capture), 7–11, 15, 16, 18–20 run now; rows 2, 3 (encodings, IME), 4, 5, 6, 12, 13, 14 by the Steps' earlier real checks; 17 by the e2e. |
| 2026-11-22 | Development PC, Windows 10 Pro 22H2 (19045), 20 logical processors, one monitor | 1–20 | **not run as a whole**: the maintainer was using the PC, so no visible test windows, real input or registry changes on their account were allowed. Earlier Steps' real-desktop checks cover rows 3, 4, 5, 12, 13 (recorded in [JARAK_ARCHITECTURE](JARAK_ARCHITECTURE.md) and DEV_CHECKPOINT); rows 1, 17, 19, 20 are covered by the e2e `step111_jarak_release` run (PASS, CLI only). |

## The run of 2026-11-23

Scripts drove only windows they had started, through a guard that refuses a key or a click when
the foreground or the point under the pointer is another program's window; every profile was a
sandbox (`%APPDATA%`, `%LOCALAPPDATA%`), the root `kim.exe` with the local mirror for Kim Manager.

| # | Result |
| --- | --- |
| 1 | **Pass.** Kim Manager: a click on the Jarak tile asked *Install / Cancel*; *Install* put Jarak 0.0.33 from the local mirror into `Programs\mouv\jarak` (the programs, capabilities, licences, the guide under `docs\`); nothing started, the sign-in Run key unchanged; the tile then said *Jarak 0.0.33 is installed* and a click opened the installed `jarak-gui.exe` |
| 3 | **Pass.** Ctrl+O opened the Windows Open dialog in the document's folder; two quoted names typed into it opened two tabs (**fault found**: the dialog first took one file only — fixed, `OFN_ALLOWMULTISELECT`); Ctrl+S on a new document opened Save As and wrote the typed path |
| 4 | **Pass** (with row 18): a Noeul capture inserted through Kim, shown in the preview |
| 7 | **Pass.** A real drag of the selected `alpha ` to line 3 moved it; one Ctrl+Z put it back; with Ctrl held it was copied |
| 8 | **Pass.** Always on top: `WS_EX_TOPMOST`, above a second window brought to the front, and not after it was turned off; F11 covered the 5120×1440 monitor and came back to the same rectangle; F12 removed the frame (the client area became the window) and every bar, typing still worked, F12 restored it; distraction-free: full screen, an 860-point centred column, no bars; `--position 300,200 --always-on-top` → top-left (300, 200), topmost; `--position 640,360` alone → there, not topmost |
| 9 | **Pass.** A right click showed Undo, Redo, Cut, Copy, Paste (text or an image), Select all, Copy as HTML; Undo, Redo, Select all, Copy (the clipboard = the document), Paste and Copy as HTML each did their work |
| 10 | **Pass.** Copy as HTML: another process read the `HTML Format` (fragment offsets, 14 coloured spans); Copy as RTF pasted into WordPad as coloured Consolas code (**fault found**: WordPad's paragraph spacing between the lines — the RTF now states single spacing); WordPad's bold text → *Paste RTF as source* inserted its `{\rtf1…` markup; `Set-Clipboard -AsHtml` → *Paste HTML as source* inserted `<p>from <b>another program</b> 자락</p>`; the maintainer's clipboard (6 formats) restored byte-identical |
| 11 | **Pass.** File → Print… opened the Windows print dialog (Microsoft Print to PDF, all / pages / selection, copies); Print → the driver's Save dialog → a one-page PDF with the file name and date, *1 / 1*, line numbers, colours and Korean text; the document unchanged. A paper printer: not tested (none) |
| 15 | **Pass.** Help → Offer for `.md` and `.txt` (`.markdown`, `.log` unchecked): the ProgID's command is the program, each extension's `OpenWithProgids` has Jarak, the defaults unchanged (`.md` stays `Markdown`); the shell's Open with list (`SHAssocEnumHandlers`, recommended — what Explorer's *Open with* shows) listed **Jarak** for both; Help → Remove: the five keys byte-identical to their export before, Jarak gone from the list. (Explorer's menu itself was not opened, to keep the test folder out of the maintainer's Explorer history) |
| 16 | **Pass.** The user guide opened read-only in the preview (**fault found**: the preview refused Ctrl+Shift+P — the palette, tabs, settings, printing and window modes now work there); About showed the version and copied the debug information |
| 18 | **Pass.** Kim Manager installed Noeul; a Noeul started without tray or shortcuts; `kim tools invoke screen.capture-file` captured a test window's region to a PNG (34 316 bytes); `markdown.insert-image` without `--allow-write` → `confirmation_required`, the note unchanged; with it → `assets/day-20261123-001.png` and its reference, previewed in the window. From a Subak window on a private Host, `jarak open notes.md:12` opened the file at line 12 in the running window (same process) and brought it to the front — after the terminal had the sandbox profile in its environment: Subak starts terminals with the user's own environment, so the first try handed the file over through the real profile's folder (removed after) |
| 19 | **Pass** for Noeul and Ddalgak (installed and started from their tiles; Ddalgak's app ran unelevated with its tray and the rule editor, ended with `quit`). Subak was not opened from Kim Manager: the maintainer's own Subak Host runs on the per-user endpoint and the window would show their Terminals; Subak's window was used on a private Host in row 18 |
| 20 | **Pass.** The Jarak tile's menu → *Uninstall…* → *Uninstall / Keep installed* → the programs gone, `%APPDATA%\mouv\jarak` kept |

An unintended side effect: a selection made with RShift+Right reached the maintainer's AutoHotkey
script, which maps it to the *next track* media key; YouTube Music skipped up to eight tracks.
The scripts select with LShift now.

## Not testable on this PC

- A paper printer (row 11 uses Microsoft Print to PDF).
- A second monitor, HDR, Windows 11 (rows 2, 8 on those).
- A screen reader run with Narrator and Windows High Contrast ([JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md)).
