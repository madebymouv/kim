# Jarak — real Windows acceptance script

Status: **current** (Step 111) · Run on a real Windows 10/11 desktop with a real keyboard and
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
| 10 | Clipboard formats | *Copy as HTML* then paste into WordPad (formatted); copy formatted text from WordPad, *Paste HTML as source* / *Paste RTF as source* in Jarak (the markup as text) | `code_exports_as_rtf_and_copies_as_html_and_rtf`, `pasting_html_and_rtf_as_source_inserts_their_text` (the real clipboard: **this row**) |
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
| 2026-11-22 | Development PC, Windows 10 Pro 22H2 (19045), 20 logical processors, one monitor | 1–20 | **not run as a whole**: the maintainer was using the PC, so no visible test windows, real input or registry changes on their account were allowed. Earlier Steps' real-desktop checks cover rows 3, 4, 5, 12, 13 (recorded in [JARAK_ARCHITECTURE](JARAK_ARCHITECTURE.md) and DEV_CHECKPOINT); rows 1, 17, 19, 20 are covered by the e2e `step111_jarak_release` run (PASS, CLI only). |

## Not testable on this PC

- A paper printer (row 11 uses Microsoft Print to PDF).
- A second monitor, HDR, Windows 11 (rows 2, 8 on those).
- A screen reader run with Narrator and Windows High Contrast ([JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md)).
