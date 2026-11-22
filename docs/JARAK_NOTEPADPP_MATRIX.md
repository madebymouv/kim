# Jarak–Notepad++ Feature Parity Matrix

Status: **current** (inventory, Step 84; Step 85 made `HELP-01`, `HELP-02` partial and
`HELP-06` supported; Step 86 `EDIT-01`, `EDIT-03` partial; Step 87 eleven file and encoding
rows partial; Step 88 fourteen rows supported and six partial, `EDIT-18` and `UI-08` moved to
Step 110; Step 89 `UI-07`, `JRK-01`, `JRK-02` supported; Step 90 `JRK-03`, `JRK-04` supported;
Step 91 `JRK-05`, `JRK-06` supported; Step 92 `UI-10`, `JRK-07`…`JRK-11` supported; Step 93 the Window/View rows (`FILE-04`…`FILE-23` of it, `VIEWS-*`, `WIN-01/02`, `RUN-05`, `REL-01`, `UI-01`) supported; Step 94 the search and mark rows (`SRCH-01`…`SRCH-12`, `SRCH-15`, `SRCH-16`, `MARK-01`…`MARK-08`) supported and `AUTO-10` partial; Step 95 `SRCH-13`, `SRCH-14`, `FIF-01`…`FIF-07` supported; Step 96 `MULTI-01`…`MULTI-10`, `EDIT-04`, `EDIT-05` supported; Step 97 the line, sort, case, blank, edit-tool, digest and MIME rows (45) supported; Step 98 the LANG and CMT rows supported; Step 99 `FOLD-01`…`FOLD-06`, `VIEWS-11`, `SRCH-18`, `SRCH-19` and `LANG-08` supported; Step 100 `AUTO-01`…`AUTO-09` and `JRK-12` supported; Step 101 `MARK-09`…`MARK-15`, `VIEW-14`, `VIEW-15`, `VIEWS-09`, `JRK-13`, `EDIT-14` supported; Step 102 `MAC-01`…`MAC-07`, `RUN-01`…`RUN-04`, `EDIT-15`, `VIEW-05` supported; Step 103 `ENC-01`…`ENC-16`, `BLANK-08`, `VIEW-06`…`VIEW-11`, `UI-06`, `EDIT-12` supported; Step 104 `REL-02`…`REL-05`, `JRK-16`, `VIEW-17` supported, `SET-14` partial; Step 105 `SET-01`, `SET-04`, `SET-08`, `SET-15`, `SET-16`, `SET-20`, `SET-23`, `SET-26` supported, `SET-02`, `SET-03`, `SET-05`, `SET-06`, `SET-10`, `SET-21`, `SET-22` partial; Step 106 `FILE-20`, `SET-12` supported, `FILE-19`, `PLUG-07` partial; Step 107 `HELP-03`, `HELP-04`, `HELP-05`, `HELP-10`, `JRK-14` supported, `JRK-15` partial; Step 108 `PLUG-08`, `PLUG-09` supported; Step 109 `SET-18`, `REL-06` supported; Step 110 every row reviewed — 20 rows supported, 10 partial with their gaps named, `SRCH-17`, `VIEW-16`, `SET-24` not intended, only `SET-09` and `HELP-11` (Step 111) planned; Step 111 `HELP-11` supported, `SET-09` partial (Explorer's real Open with list not yet seen) — the Step that implements a row updates it in the same change) · Reference: **Notepad++ 8.9.x** (8.9.7 released
2026-07; the user manual covers v8.x up to 8.9.9.1), official documentation read on 2026-11-22 ·
Owner: mouv · Applies to: Jarak · Requirements: [JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md) ·
Decision: [ADR 0102](adr/0102-jarak-fourth-kim-tool.md)

This matrix lists the **built-in** features of Notepad++, menu by menu, and Jarak's answer to
each: its status, the requirement that covers it, the Step that owns it and how it is verified.
It is the tracking document for parity ([JARAK_PHILOSOPHY §2.8](JARAK_PHILOSOPHY.md)): a row
becomes `supported` only with its own test or real-window evidence, never because a similar
feature exists. Notepad++ is described here in Jarak's own words, for comparison only; nothing
from Notepad++ or Scintilla is copied ([JARAK_PHILOSOPHY §6](JARAK_PHILOSOPHY.md)).

**Statuses:** `planned` (owned by a Step, not implemented) · `partial` (implemented with named
gaps, evidence given) · `supported` (implemented and verified, evidence given) · `not intended`
(a deliberate non-goal, reason given). `cargo xtask docs-check` verifies that every row has a
status, an owner Step in 84–111, unless it is `not intended` a defined `JARAK-*`
requirement, and for `partial` and `supported` an Evidence cell (test names, commands or the
real-window check) instead of `—`.

## Sources and method

| Code | Source | What was used |
| --- | --- | --- |
| **I** | Notepad++ User Manual, Introduction, <https://npp-user-manual.org/> | Scope of the manual (v8.x up to 8.9.9.1) |
| **U** | User Interface, <https://npp-user-manual.org/docs/user-interface/> | Menus (File, Edit, Search, View, Encoding, Language, Settings, Tools, Macro, Run, Plugins, Window, ?), tab bar, status bar |
| **E** | Editing, <https://npp-user-manual.org/docs/editing/> | Column mode, multi-editing, the Column Editor, line and blank operations, comments |
| **S** | Searching, <https://npp-user-manual.org/docs/searching/> | Find/Replace/Mark/Find in Files, search modes, regular expressions, results window, incremental search, styles |
| **V** | Views, <https://npp-user-manual.org/docs/views/> | Two views, move/clone, synchronized scrolling, panels, folding, document map |
| **P** | Preferences, <https://npp-user-manual.org/docs/preferences/> | Every Preferences page (General … MISC) and the options inside them |
| **A** | Auto-Completion, <https://npp-user-manual.org/docs/auto-completion/> | Word/function completion, parameter hints, auto-insert |
| **M** | Macros, <https://npp-user-manual.org/docs/macros/> | Recording, playback, run multiple times, saving, recordable commands |
| **X** | Running External Commands, <https://npp-user-manual.org/docs/run-menu/> | The Run dialog, variables, saved commands |
| **C** | Command Line Arguments, <https://npp-user-manual.org/docs/command-prompt/> | Startup switches (-n, -c, -l, -udl, -ro, -nosession, -multiInst, …) |
| **L** | Syntax Highlighting – Built-in Languages, <https://npp-user-manual.org/docs/programing-languages/>, and the User Defined Language pages | Built-in languages, UDL definition and import |
| **R** | Configuration Files Details, <https://npp-user-manual.org/docs/config-files/> | Shortcut Mapper, Style Configurator and themes, session and backup files |
| **G** | Notepad++ release notes, <https://notepad-plus-plus.org/news/> | Current version (8.9.x), bundled plugins (MIME Tools, Converter, NppExport), the updater |

Method: every menu of the User Interface page was walked item by item; options that change a
feature's behaviour are folded into that feature's row unless the manual gives them a page of
their own (Preferences rows). Jarak's own additions (Markdown preview, image paste, Vim, the
palette, the headless CLI) follow in §2.

## 1. Notepad++ built-in features

### FILE — File menu and opening files (sources U, E, C)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FILE-01 | New untitled document (Ctrl+N) | supported | `JARAK-FR-020` | 87 | T, M | Step 88: Ctrl+N opens a new untitled document (key test); Step 93: it is a tab of its own (`tabs_switch_reorder_close_and_reopen_by_mouse_and_keys`); Step 110: in the settings' language when set (`new_language_open_folder_find_fill_and_backup_settings_apply`) |
| FILE-02 | Open one or several files with a dialog (Ctrl+O) | partial | `JARAK-FR-020` | 87 | T, M | Step 88: Ctrl+O with the Windows Open dialog (`GetOpenFileNameW`); the dialog itself not exercised by a test yet |
| FILE-03 | Open files by drag and drop onto the window | partial | `JARAK-FR-032` | 88 | M | Step 88: files dropped on the window open (`dropped_files`); a real drop not yet verified |
| FILE-04 | Open containing folder in Explorer / in a command prompt | supported | `JARAK-FR-060` | 93 | T | Step 93: tab menu and palette — Show in Explorer (selected), a command prompt in the folder (window test with a recording opener) |
| FILE-05 | Open the file in the default viewer (shell association) | supported | `JARAK-FR-060` | 93 | T | Step 93: the default program, never for programs or scripts (`exe`, `bat`, `ps1`, `lnk`, … shown in Explorer instead; unit + window test) |
| FILE-06 | Open Folder as Workspace | supported | `JARAK-FR-063` | 93 | T, M | Step 93: a dropped folder, `jarak open <folder>` or the document's folder becomes the workspace; restored by the session (window test, e2e) |
| FILE-07 | Reload from disk | supported | `JARAK-FR-020` | 87 | T | Step 87: `FileDocument::reload` (tests); Step 104: File → Reload from disk (`file.reload`) and a file changed on disk reloading as one undo step (`a_file_changed_on_disk_is_asked_about_and_reloads_as_one_undo_step`, `unmodified_documents_reload_silently_when_set`) |
| FILE-08 | Save (Ctrl+S) | supported | `JARAK-FR-020` | 87 | T, M | Step 88: Ctrl+S, atomic (window test; real desktop: the file written with CRLF, nothing left behind) |
| FILE-09 | Save As | partial | `JARAK-FR-020` | 87 | T, M | Step 88: Ctrl+Shift+S and Ctrl+S on an untitled document ask for a name (window test with a scripted dialog); the real dialog not exercised yet |
| FILE-10 | Save a Copy As (document keeps its path) | supported | `JARAK-FR-020` | 87 | T | Step 110: File → Save a copy as… writes the text to another file; the document keeps its own file and its unsaved state; its own file is refused as the copy's name (`save_a_copy_keeps_the_documents_file_and_unsaved_state`; the Save dialog itself is `FILE-09`'s) |
| FILE-11 | Save All | supported | `JARAK-FR-020` | 93 | T | Step 93: Save All (tab menu, palette, File menu; window test) |
| FILE-12 | Rename the file of the document | supported | `JARAK-FR-020` | 93 | T | Step 93: Rename moves the file without replacing another; the document follows (window test) |
| FILE-13 | Close (Ctrl+W) | supported | `JARAK-FR-020` | 87 | T, M | Step 88: Ctrl+W/Ctrl+F4 with the unsaved-changes question (window tests; real desktop) |
| FILE-14 | Close All | supported | `JARAK-FR-020` | 93 | T | Step 93: Close All, asking about each unsaved document in turn (window test) |
| FILE-15 | Close more: all but active, to the left, to the right, all unchanged | supported | `JARAK-FR-020` | 93 | T | Step 93: close the others, to the left, to the right, the unchanged ones (window test) |
| FILE-16 | Move the file to the Recycle Bin | supported | `JARAK-FR-020` | 93 | T | Step 93: asked first, SHFileOperationW with FOF_ALLOWUNDO (never a deletion), the document closes (window test with a recording opener) |
| FILE-17 | Load Session from a file | supported | `JARAK-FR-062` | 93 | T | Step 93: Open a session file… (window test) |
| FILE-18 | Save Session to a file | supported | `JARAK-FR-062` | 93 | T | Step 93: Save the session to a file… (documents with a file; window test) |
| FILE-19 | Print (dialog) | partial | `JARAK-FR-140` | 106 | M | Step 106: File → Print… — the Windows print dialog (`PrintDlgW`: printer, copies, paper, all / selection / page range) and Jarak's own layout through GDI (highlighted text with line numbers, rendered Markdown with images, header and footer); tested with a scripted dialog and a recording printer (`printing_uses_the_dialogs_printer_settings_and_range`), the GDI backend for real through Microsoft Print to PDF (PDFs reviewed); partial: the real dialog not yet opened on the desktop, no paper printer on this PC ([JARAK_EXPORT](JARAK_EXPORT.md)) |
| FILE-20 | Print Now (default printer, no dialog) | supported | `JARAK-FR-140` | 106 | M | Step 106: File → Print now — the default printer without a dialog (a file-writing default such as Microsoft Print to PDF gets its file from the Save dialog, never from the driver); `jarak print` without `--printer` (`a_pdf_printer_gets_its_file_from_the_save_dialog`, CLI `print_checks_its_options_and_dry_runs_on_the_real_printer`, e2e `step106_jarak_export`) |
| FILE-21 | Recent files list | supported | `JARAK-FR-028` | 93 | T | Step 93: newest first, at most 20, in the palette and the File menu (window test) |
| FILE-22 | Restore recently closed file (Ctrl+Shift+T) | supported | `JARAK-FR-028` | 93 | T | Step 93: Ctrl+Shift+T reopens the closed files newest first (window test) |
| FILE-23 | Open all recent files / empty the recent files list | supported | `JARAK-FR-028` | 93 | T | Step 93: open all recent files, empty the list (palette, File menu) |
| FILE-24 | Exit with unsaved-change prompts | supported | `JARAK-FR-024` | 87 | T, M | Step 88: closing the window asks for each unsaved document (window test; real desktop: Esc kept it, Enter saved and closed) |
| FILE-25 | Prompt Save / Don't save / Cancel when closing a dirty document | supported | `JARAK-FR-024` | 87 | T, M | Step 88: Save (Enter) / Don't save / Cancel (Esc) (window tests; real desktop; audited `question` view) |

### EDIT — Edit menu: clipboard, selection and basic editing (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| EDIT-01 | Undo / Redo (Ctrl+Z, Ctrl+Y) | supported | `JARAK-FR-013` | 86 | T | Step 88: Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z on the core transactions (window tests; real desktop: Ctrl+Z back to the saved state) |
| EDIT-02 | Cut / Copy / Paste / Delete | supported | `JARAK-FR-031` | 88 | T, M | Step 88: Ctrl+C/X/V, Ctrl+Insert, Shift+Delete, Shift+Insert, Delete through the Windows clipboard (window test; real desktop: copy and paste) |
| EDIT-03 | Select All | supported | `JARAK-FR-012` | 86 | T | Step 88: Ctrl+A (window tests; real desktop) |
| EDIT-04 | Begin/End Select (stream) for keyboard-free range selection | supported | `JARAK-FR-012` | 96 | T | Step 96: palette *Begin/End Select*: the first run marks, the second selects to the caret (`begin_end_select_in_stream_and_column_mode`) |
| EDIT-05 | Begin/End Select in column mode | supported | `JARAK-FR-081` | 96 | T | Step 96: *Begin/End Select in column mode* makes a column selection (`begin_end_select_in_stream_and_column_mode`) |
| EDIT-06 | Copy the current line when nothing is selected (setting) | supported | `JARAK-FR-031` | 97 | T | Step 97: on by default; palette *Copy the line when nothing is selected* switches it (kept in settings.toml); copying nothing leaves the clipboard (`dates_paths_digests_and_the_copy_line_setting`) |
| EDIT-07 | Insert date and time: short, long, customised format | supported | `JARAK-FR-087` | 97 | T | Step 97: short and long in the user's Windows formats, custom from `date_format` (Notepad++'s tokens) (`lineops::tests::the_custom_date_format`, `dates_paths_digests_and_the_copy_line_setting`, real desktop r97) |
| EDIT-08 | Copy to clipboard: full path, file name, directory of the document | supported | `JARAK-FR-087` | 97 | T | Step 97: palette *Copy the full path / file name / directory* (`dates_paths_digests_and_the_copy_line_setting`) |
| EDIT-09 | Copy to clipboard: names and paths of all open documents | supported | `JARAK-FR-087` | 97 | T | Step 97: palette *Copy the names / paths of all open documents* (window test) |
| EDIT-10 | Increase / decrease line indent (Tab, Shift+Tab) | supported | `JARAK-FR-085` | 88 | T | Step 88: Tab/Shift+Tab indent and unindent the selected lines, Tab inserts a tab otherwise (core tests, key test) |
| EDIT-11 | Set read-only for the document / clear the file's read-only attribute | supported | `JARAK-FR-027` | 87 | T | Step 110: Edit → Read-only on/off (typing refused while on; `--read-only` sets the same flag); a lossy, binary or monitored document stays read-only with its reason; File → Clear the file's read-only attribute (`the_read_only_flag_toggles_and_the_files_attribute_clears`) |
| EDIT-12 | Paste special: copy/cut/paste binary content (with NUL) | supported | `JARAK-FR-027` | 103 | T | Step 103: copy / cut / paste binary content keep NUL characters through a clipboard format of their own (`show_symbol_settings_reach_the_view_and_binary_content_keeps_nul`, memory clipboard); the real Windows clipboard checked 2026-11-22: copied from a read-only NUL file, pasted into another document and saved with the NUL |
| EDIT-13 | Paste HTML / RTF clipboard content as source | partial | `JARAK-FR-087` | 110 | T | Step 110: Edit → Paste HTML as source / Paste RTF as source insert the clipboard's `HTML Format` fragment or `Rich Text Format` as text, one undo step; without them Jarak says so (`pasting_html_and_rtf_as_source_inserts_their_text`, `a_clipboard_html_fragment_is_found`); partial: reading those formats from the real Windows clipboard is not yet exercised (the clipboard was the maintainer's, in use) |
| EDIT-14 | On selection: open the selected path as a file / its folder | supported | `JARAK-FR-103` | 101 | T | Step 101: *Open the selected path* / *Show the selected path's folder*, relative to the document's folder (palette, Edit) |
| EDIT-15 | On selection: search on the internet (explicit, configurable engine) | supported | `JARAK-FR-111` | 102 | T | Step 102: the selection or the word at the caret, `search_engine` in the settings (`$(QUERY)`, DuckDuckGo by default), only on the command (`run_commands_show_the_exact_line_and_run_only_on_the_button`) |
| EDIT-16 | Character panel (insert a character by code) | supported | `JARAK-FR-087` | 110 | T | Step 110: Edit → Character panel… inserts a character by its code (`U+00E9`, `0xE9`, `&#233;`, `233`) or from the 32–255 table (`the_character_panel_inserts_by_code_and_from_the_table`, `character_codes_in_every_notation`; audited, `the_step_110_surfaces_pass_the_ui_audit_rules`, e2e `step110_jarak_parity`) |
| EDIT-17 | Clipboard history panel | partial | `JARAK-FR-087` | 110 | T | Step 110: Edit → Paste from clipboard history… lists what Jarak copied, cut or pasted (newest first, each once, 20, in memory only) and pastes a pick (`the_clipboard_history_pastes_an_earlier_copy`; audited); partial by design: copies made in other programs are not listed — Jarak does not watch the system clipboard (`JARAK-SEC-003`) |
| EDIT-18 | Drag and drop of selected text inside the document | supported | `JARAK-FR-032` | 110 | M | Step 110: dragging the selection moves it as one undo step; Ctrl held at the release copies it; a drop caret shows where it lands; a click inside the selection only places the caret; a read-only document keeps its text (`dragging_the_selection_moves_or_copies_it`, pointer events through egui) |

### CASE — Edit → Convert Case to (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CASE-01 | UPPERCASE | supported | `JARAK-FR-084` | 97 | T | Step 97: Ctrl+Shift+U, every selected range (`lineops::tests`::every_case_conversion, window test, real desktop r97) |
| CASE-02 | lowercase | supported | `JARAK-FR-084` | 97 | T | Step 97: Ctrl+U (`lineops::tests`::every_case_conversion, window test) |
| CASE-03 | Proper Case and Proper Case (blend) | supported | `JARAK-FR-084` | 97 | T | Step 97: Alt+U, Alt+Shift+U for the blend (`lineops::tests`::every_case_conversion, window test, real desktop r97) |
| CASE-04 | Sentence case and Sentence case (blend) | supported | `JARAK-FR-084` | 97 | T | Step 97: Ctrl+Alt+U, Ctrl+Alt+Shift+U for the blend; full-width `. ! ?` too (`lineops::tests`::every_case_conversion) |
| CASE-05 | iNVERT cASE | supported | `JARAK-FR-084` | 97 | T | Step 97: palette (`lineops::tests`::every_case_conversion) |
| CASE-06 | ranDOm CasE | supported | `JARAK-FR-084` | 97 | T | Step 97: palette (`lineops::tests`::every_case_conversion) |

### LINE — Edit → Line Operations (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LINE-01 | Duplicate current line / selection (Ctrl+D) | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+D, the line or the selection, every caret (`lineops::tests`::duplicate_a_line_or_a_selection, window test, real desktop r97) |
| LINE-02 | Delete current line | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+Shift+L, every caret's lines (`lineops::tests`::delete_lines_including_the_last, window test, real desktop r97) |
| LINE-03 | Remove duplicate lines | supported | `JARAK-FR-082` | 97 | T | Step 97: palette, the first stays, the final line break kept (`lineops::tests`::remove_duplicate_consecutive_and_empty_lines, window test) |
| LINE-04 | Remove consecutive duplicate lines | supported | `JARAK-FR-082` | 97 | T | Step 97: palette (`lineops::tests`::remove_duplicate_consecutive_and_empty_lines, real desktop r97) |
| LINE-05 | Split lines at the edge or a width | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+I at the view's width in cells, word boundaries, Hangul two cells (`lineops::tests`::join_and_split_lines) |
| LINE-06 | Join lines | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+J, a space only where words would touch (`lineops::tests`::join_and_split_lines, window test, real desktop r97) |
| LINE-07 | Move line up / down | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+Shift+Up / Down keeping the selection (`lineops::tests`::move_lines_up_and_down_keeping_the_selection, window test, real desktop r97) |
| LINE-08 | Remove empty lines | supported | `JARAK-FR-082` | 97 | T | Step 97: palette (`lineops::tests`::remove_duplicate_consecutive_and_empty_lines) |
| LINE-09 | Remove empty lines (containing blank characters) | supported | `JARAK-FR-082` | 97 | T | Step 97: palette (`lineops::tests`::remove_duplicate_consecutive_and_empty_lines) |
| LINE-10 | Insert blank line above / below current | supported | `JARAK-FR-082` | 97 | T | Step 97: Ctrl+Alt+Enter / Ctrl+Alt+Shift+Enter, the caret on the new line (`lineops::tests`::insert_blank_lines_above_and_below, window test, real desktop r97) |
| LINE-11 | Reverse line order | supported | `JARAK-FR-082` | 97 | T | Step 97: palette (`lineops::tests`::reverse_and_randomize_lines) |
| LINE-12 | Randomize line order | supported | `JARAK-FR-082` | 97 | T | Step 97: palette (`lineops::tests`::reverse_and_randomize_lines) |
| LINE-13 | Sort lines lexicographically ascending / descending | supported | `JARAK-FR-083` | 97 | T | Step 97: palette, by code point, stable (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length, real desktop r97) |
| LINE-14 | Sort lines lexicographically ignoring case | supported | `JARAK-FR-083` | 97 | T | Step 97: palette (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length) |
| LINE-15 | Sort lines in locale order | supported | `JARAK-FR-083` | 97 | T | Step 97: palette, Windows `CompareStringEx` in the user's locale (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length) |
| LINE-16 | Sort lines as integers ascending / descending | supported | `JARAK-FR-083` | 97 | T | Step 97: palette, lines without a number first (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length, e2e step97, window test) |
| LINE-17 | Sort lines as decimals (comma / dot) ascending / descending | supported | `JARAK-FR-083` | 97 | T | Step 97: palette, `.` and `,` (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length) |
| LINE-18 | Sort lines by length | supported | `JARAK-FR-083` | 97 | T | Step 97: palette, in characters (`lineops::tests`::sort_lexicographic_case_locale_numbers_and_length) |
| LINE-19 | Sort only the lines or the column of the selection | supported | `JARAK-FR-083` | 97 | T | Step 97: the selected lines, or with a column selection by its columns (`lineops::tests`::sort_only_the_selected_lines_or_by_a_column) |

### CMT — Edit → Comment/Uncomment (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CMT-01 | Toggle single-line comment (Ctrl+Q) | supported | `JARAK-FR-086` | 98 | T | Step 98: Ctrl+Q in the document's language, the selection kept (`lineops::tests::comments_toggle_add_remove_block_and_markup`, `comment_keys_follow_the_language`) |
| CMT-02 | Single-line comment / uncomment | supported | `JARAK-FR-086` | 98 | T | Step 98: Ctrl+K / Ctrl+Shift+K at the common indentation; block comments per line where a language has no line comment (`lineops::tests::comments_toggle_add_remove_block_and_markup`, window test) |
| CMT-03 | Block comment / block uncomment | supported | `JARAK-FR-086` | 98 | T | Step 98: Ctrl+Shift+Q and the palette's block uncomment (`lineops::tests::comments_toggle_add_remove_block_and_markup`, `comment_keys_follow_the_language`) |

### BLANK — Edit → Blank Operations and EOL (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| BLANK-01 | Trim trailing space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette (`lineops::tests`::every_blank_operation, window test) |
| BLANK-02 | Trim leading space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette (`lineops::tests`::every_blank_operation) |
| BLANK-03 | Trim leading and trailing space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette (`lineops::tests`::every_blank_operation) |
| BLANK-04 | EOL to space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette (`lineops::tests`::every_blank_operation) |
| BLANK-05 | Trim both and EOL to space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette (`lineops::tests`::every_blank_operation) |
| BLANK-06 | TAB to space | supported | `JARAK-FR-085` | 97 | T | Step 97: palette, tab stops with wide characters two cells (`lineops::tests`::every_blank_operation) |
| BLANK-07 | Space to TAB (all / leading) | supported | `JARAK-FR-085` | 97 | T | Step 97: palette, all or leading only (`lineops::tests`::every_blank_operation) |
| BLANK-08 | EOL conversion: Windows (CR LF), Unix (LF), Macintosh (CR) | supported | `JARAK-FR-026` | 103 | T | Step 103: Windows / Unix / Macintosh in one undo step, marks kept on their lines (`document::tests::line_endings_convert_in_one_undo_step`, `line_endings_convert_and_new_documents_take_the_settings_encoding`) |

### MULTI — Multi-editing, column mode and the Column Editor (sources E, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MULTI-01 | Ctrl+click adds a caret / selection (multi-editing) | supported | `JARAK-FR-080` | 96 | T, M | Step 96: Ctrl+click adds (or removes) a caret, Ctrl+drag a selection; typing, Backspace, paste and undo on every caret (`multi::tests`, `ctrl_click_adds_carets_that_type_delete_paste_and_undo_together`, `ctrl_clicks_past_line_ends_add_carets_at_the_ends`; real desktop r96: a click and two real Ctrl+clicks typing at three line ends) |
| MULTI-02 | Multi-select all occurrences (match case / whole word options) | supported | `JARAK-FR-080` | 96 | T | Step 96: Alt+F3 and the palette's four variants (ignore case and whole word, match case only, whole word only, match case and whole word) (`multi::tests::next_and_all_occurrences_with_their_options`, `ctrl_shift_d_adds_the_next_occurrence_and_alt_f3_selects_all`, UI audit `multi`) |
| MULTI-03 | Multi-select next occurrence / skip current | supported | `JARAK-FR-080` | 96 | T | Step 96: Ctrl+Shift+D (Ctrl+D stays Duplicate line, Notepad++'s key) and *skip this occurrence* (`multi::tests`, window test) |
| MULTI-04 | Column selection with Alt+drag | supported | `JARAK-FR-081` | 96 | M | Step 96: Alt+drag by display column, past short lines in cells (`alt_drag_draws_a_column_block_with_the_mouse_and_virtual_space_pads`; real desktop r96: a real Alt+drag down column 0 typed into four lines) |
| MULTI-05 | Column selection with Alt+Shift+arrows | supported | `JARAK-FR-081` | 96 | T | Step 96: Alt+Shift+arrows from the caret (`alt_shift_arrows_make_a_column_block_aligned_by_display_column`; real desktop r96: columns 0-4 over a Hangul line) |
| MULTI-06 | Typing, deleting and pasting into every row of a column selection | supported | `JARAK-FR-081` | 96 | T | Step 96: a column selection is a multi-selection: typing, deleting and pasting (one line per row when the counts match) into every row, aligned by display column with Hangul and emoji (`multi::tests::a_column_block_lines_up_by_display_column_with_hangul_and_emoji`, window tests, real desktop r96, UI audit `column`) |
| MULTI-07 | Column Editor: insert text into each row | supported | `JARAK-FR-081` | 96 | T | Step 96: Alt+C, *Text to insert* into every row (or from the caret down), padding short lines (`the_column_editor_inserts_text_or_numbers_in_every_row`, `multi::tests`, real desktop r96) |
| MULTI-08 | Column Editor: insert incrementing numbers (initial, increase, repeat, leading, base) | supported | `JARAK-FR-081` | 96 | T | Step 96: initial, increase by, repeat, no/zero/space leading, Dec/Hex/Oct/Bin (`multi::tests::column_editor_numbers_in_every_base_with_repeat_and_leading`, window test, UI audit `column-editor`; real desktop r96: real clicks on the Number page, the initial number 8 and leading zeros inserted 08…12 into five lines, undone in one step) |
| MULTI-09 | Convert a column selection to a multi-selection (setting) | supported | `JARAK-FR-081` | 96 | T | Step 96: a column selection always is a multi-selection of one range per row (the setting's "on" state; there is no "off" state to switch to) (`multi::tests`, window tests) |
| MULTI-10 | Virtual space past line ends (setting) | supported | `JARAK-FR-081` | 96 | T | Step 96: palette *Virtual space* (kept in settings.toml) pads short rows in the same undo step and draws their carets at the block's column (`multi::tests::virtual_space_pads_short_lines_and_column_editor_inserts_numbers`, `alt_drag_draws_a_column_block_with_the_mouse_and_virtual_space_pads`) |

### AUTO — Auto-completion and editing assistance (sources A, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| AUTO-01 | Word completion from the document | supported | `JARAK-FR-094` | 100 | T | Step 100: words around the caret (Korean too), while typing and Ctrl+Enter (`assist::tests::words_keywords_api_and_snippets_complete_a_prefix`, `completion_pops_up_while_typing_and_never_takes_enter_unasked`); 10 MB keystroke p95 1.30 ms |
| AUTO-02 | Function / keyword completion from language definitions | supported | `JARAK-FR-094` | 100 | T | Step 100: the language's keywords, types, constants and Notepad++ API files (`autoCompletion\\*.xml`, built-in C, Python, JavaScript) (`assist::tests::notepad_plus_plus_api_files_are_read`, `the_users_snippets_and_notepad_plus_plus_api_files_are_read`) |
| AUTO-03 | Function parameters hint (and previous/next hint) | supported | `JARAK-FR-094` | 100 | T | Step 100: the call's signature, current parameter, Alt+Up/Down overloads, Ctrl+Shift+Space (`parameter_hints_follow_the_call_and_matching_brackets_are_highlighted`, `paths_complete_tags_match_and_hints_switch_overloads`) |
| AUTO-04 | Path completion | supported | `JARAK-FR-094` | 100 | T | Step 100: Ctrl+Alt+Space from the document's folder or an absolute path, never a network path (`assist::tests::paths_complete_inside_the_documents_folder_and_never_on_the_network`, `paths_complete_tags_match_and_hints_switch_overloads`) |
| AUTO-05 | Auto-insert closing brackets and quotes (setting) | supported | `JARAK-FR-096` | 100 | T | Step 100: off by default; pairs, stepping over and Backspace for the ones it inserted, wrapping a selection (`assist::tests::brackets_and_quotes_close_step_over_wrap_and_delete_in_pairs`, `closers_and_tags_only_when_turned_on_and_smart_indentation`) |
| AUTO-06 | HTML/XML closing tag auto-insertion | supported | `JARAK-FR-096` | 100 | T | Step 100: off by default; not for void, closing, `<!`, `<?` or self-closing tags (`assist::tests::tags_close_in_markup_and_enter_indents_smartly`, `closers_and_tags_only_when_turned_on_and_smart_indentation`) |
| AUTO-07 | Auto-indent (keep / smart per language) | supported | `JARAK-FR-096` | 100 | T | Step 100: off / keep / smart (brackets, Python and YAML colons, a closer's dedent, the document's unit) (`assist::tests::tags_close_in_markup_and_enter_indents_smartly`, `closers_and_tags_only_when_turned_on_and_smart_indentation`) |
| AUTO-08 | Brace highlighting | supported | `JARAK-FR-096` | 100 | T | Step 100: the bracket and its partner, an unmatched one in the danger colour, strings and comments skipped, bounded to 3 000 lines (`assist::tests::bracket_and_tag_matching_is_bounded_and_skips_strings`, `parameter_hints_follow_the_call_and_matching_brackets_are_highlighted`) |
| AUTO-09 | HTML/XML tag matching highlight | supported | `JARAK-FR-096` | 100 | T | Step 100: the tag name and its partner's, void elements without (`assist::tests::bracket_and_tag_matching_is_bounded_and_skips_strings`, `paths_complete_tags_match_and_hints_switch_overloads`) |
| AUTO-10 | Smart highlighting of the selected word (with options) | supported | `JARAK-FR-071` | 94 | T | Step 94: a selected word (letters, digits, `_`) highlights its other occurrences on screen (`incremental_search_highlight_all_and_smart_highlighting`); Step 110: its options in Settings → Editor → Highlighting — on/off, match case, whole words (`smart_highlighting_word_characters_and_the_length_summary`) |

### SRCH — Search menu: find and replace in the document (sources S, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| SRCH-01 | Find dialog (Ctrl+F) | supported | `JARAK-FR-070` | 94 | T, M | Step 94: Ctrl+F opens a non-modal find panel at the editor's top right (Find, Replace and Mark pages, kim-ui components), Enter / Shift+Enter, Esc (`the_find_panel_finds_with_every_option_and_f3_repeats`, UI audit `find`, real desktop r94) |
| SRCH-02 | Replace dialog (Ctrl+H) | supported | `JARAK-FR-070` | 94 | T, M | Step 94: Ctrl+H opens the panel on Replace, Tab moves to the replace box (`replace_and_replace_all_in_every_mode_and_in_the_selection`, UI audit `replace`, real desktop r94) |
| SRCH-03 | Search mode Normal | supported | `JARAK-FR-070` | 94 | T | Step 94: `search::tests`, window tests, `jarak find` (e2e step94, real desktop r94) |
| SRCH-04 | Search mode Extended (\n \r \t \0 \x \u escapes) | supported | `JARAK-FR-070` | 94 | T | Step 94: `\n \r \t \0 \\ \xNN \uNNNN` (`search::tests::extended_mode_understands_escapes`, the Replace All window test, `jarak find --extended` in e2e step94, real desktop r94b (`\tbeta`)) |
| SRCH-05 | Search mode Regular expression (and '. matches newline') | supported | `JARAK-FR-070` | 94 | T | Step 94: Rust `regex` syntax (no look-around or back-references, linear time; JARAK_ARCHITECTURE §16), `^`/`$` at CRLF line ends, `\1`/`$1` in the replacement, `. matches newline` (`search::tests`, window tests, e2e step94, real desktop r94b (`a.p`, `two.*end` only with `.` matches newline)) |
| SRCH-06 | Options: match case, whole word, wrap around, backward direction | supported | `JARAK-FR-070` | 94 | T | Step 94: match case, whole word, wrap around (with a "wrapped" status), backward (`the_find_panel_finds_with_every_option_and_f3_repeats`, real desktop: whole-word Replace All left `concat` alone; r94b: match case skipped `Alpha`, backward wrapped to the last `line`, wrap off said "Not found") |
| SRCH-07 | In selection | supported | `JARAK-FR-070` | 94 | T | Step 94: Find, Count, Replace All and Mark All inside the selection (`replace_and_replace_all_in_every_mode_and_in_the_selection`, `search::tests`, real desktop r94b (Replace All inside the selected line only)) |
| SRCH-08 | Find Next / Find Previous (F3, Shift+F3) | supported | `JARAK-FR-070` | 94 | T | Step 94: F3 / Shift+F3, also inside the panel's boxes (window tests); the real desktop used the palette's Find next / previous because F3 is another program's global hotkey on the test PC |
| SRCH-09 | Select and Find Next / Previous (Ctrl+F3) | supported | `JARAK-FR-072` | 94 | T | Step 94: Ctrl+F3 / Ctrl+Shift+F3 take the selection or the word at the caret into the find box (`ctrl_f3_takes_the_word_and_ctrl_alt_f3_leaves_the_find_box`, real desktop via the palette command) |
| SRCH-10 | Find (volatile) next / previous of the word under the caret | supported | `JARAK-FR-072` | 94 | T | Step 94: Ctrl+Alt+F3 / Ctrl+Alt+Shift+F3, match case and whole word, the find box untouched (`ctrl_f3_takes_the_word_and_ctrl_alt_f3_leaves_the_find_box`) |
| SRCH-11 | Count matches | supported | `JARAK-FR-070` | 94 | T | Step 94: Count (Alt+T) in the panel and `jarak find --count` (window tests, e2e step94; 832 204 matches in 50 MB counted in 125 ms) |
| SRCH-12 | Replace / Replace All (one undo step) | supported | `JARAK-FR-070` | 94 | T | Step 94: Replace selects then replaces; Replace All (Alt+A) is one undo step: 832 204 replacements in a 50 MB document in 347 ms, undone in 87 ms (`search::tests::replace_all_on_50_mb`, release); real desktop Ctrl+Z / Ctrl+Y |
| SRCH-13 | Find All in current document (results panel) | supported | `JARAK-FR-073` | 95 | T | Step 95: *Find all in this document* (panel, palette) lists the matches in the results panel; F4 opens each (`find_all_in_documents_and_replace_all_in_open_documents`) |
| SRCH-14 | Find All / Replace All in all opened documents | supported | `JARAK-FR-074` | 95 | T | Step 95: *In all open documents* (untitled ones too) and *Replace all in all open documents*, one undo step per document, read-only ones left (`find_all_in_documents_and_replace_all_in_open_documents`) |
| SRCH-15 | Incremental search bar (Ctrl+Alt+I) | supported | `JARAK-FR-072` | 94 | T | Step 94: Ctrl+Alt+I, the first match from where it started as you type, Enter / Shift+Enter, the count, Esc (`incremental_search_highlight_all_and_smart_highlighting`, UI audit `incremental`, real desktop) |
| SRCH-16 | Search history (find/replace combo history) | supported | `JARAK-FR-072` | 94 | T | Step 94: 20 entries each for find and replace, Up / Down in the find box, kept in the session (`the_find_history_walks_with_the_arrows_and_lives_in_the_session`, e2e step94, schema `jarak-session-v1`) |
| SRCH-17 | Find dialog transparency on losing focus | not intended | — | 110 | M | Step 110: Jarak's find panel is docked in the window under the text, never a floating dialog over it, so there is nothing to make transparent |
| SRCH-18 | Go to line / offset (Ctrl+G) | supported | `JARAK-FR-103` | 99 | T | Step 99: Ctrl+G, a line or a character offset, a wrong entry kept with a message; a fold around the target opens (`go_to_line_and_matching_brackets_open_folds`, UI audit `goto`) |
| SRCH-19 | Go to matching brace / select all between matching braces | supported | `JARAK-FR-096` | 99 | T | Step 99: Ctrl+B both ways and Ctrl+Alt+B, skipping brackets in strings and comments (`structure::tests`::brackets_match_both_ways_skipping_strings, `go_to_line_and_matching_brackets_open_folds`) |
| SRCH-20 | Find characters in range (e.g. non-ASCII) | supported | `JARAK-FR-070` | 110 | T | Step 110: Search → Find the next non-ASCII character, the next character above U+00FF, Find characters in a range… (code points as U+, 0x or decimal; wrapping) (`characters_in_a_range_are_found_one_after_another`, `the_next_character_in_a_range_wraps`) |

### MARK — Search → Mark, style tokens and bookmarks (sources S, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MARK-01 | Mark All with the Mark dialog (optionally bookmark the lines) | supported | `JARAK-FR-071` | 94 | T | Step 94: the Mark page's Mark All (Alt+M) with "Bookmark the lines" (dots in the gutter); the bookmark commands are Step 101 (`mark_all_bookmarks_lines_clears_and_copies_marks`, UI audit `mark`, e2e step94, real desktop) |
| MARK-02 | Purge marks before each search / clear all marks | supported | `JARAK-FR-071` | 94 | T | Step 94: "Clear marks first" and Clear marks; marks move with edits and undo (`mark_all_bookmarks_lines_clears_and_copies_marks`) |
| MARK-03 | Copy marked text | supported | `JARAK-FR-071` | 94 | T | Step 94: Copy marked joins the marked texts with the document's line ending (`mark_all_bookmarks_lines_clears_and_copies_marks`) |
| MARK-04 | Style all occurrences of the token (five styles) | supported | `JARAK-FR-071` | 94 | T | Step 94: palette *Style all occurrences of the token: style 1…5* (`token_styles_jump_copy_and_clear`, UI audit `mark`) |
| MARK-05 | Style one token | supported | `JARAK-FR-071` | 94 | T | Step 94: *Style one token: style 1…5* (`token_styles_jump_copy_and_clear`) |
| MARK-06 | Clear a style / all styles | supported | `JARAK-FR-071` | 94 | T | Step 94: *Clear style 1…5*, *Clear all styles* (`token_styles_jump_copy_and_clear`) |
| MARK-07 | Jump up / down to the next styled occurrence | supported | `JARAK-FR-071` | 94 | T | Step 94: *Jump to the next / previous styled text*, wrapping (`token_styles_jump_copy_and_clear`) |
| MARK-08 | Copy styled text | supported | `JARAK-FR-071` | 94 | T | Step 94: *Copy the styled text* (`token_styles_jump_copy_and_clear`) |
| MARK-09 | Toggle bookmark (Ctrl+F2) | supported | `JARAK-FR-100` | 101 | T | Step 101: Ctrl+F2 on every caret's line, also in read-only documents (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`, `bookmark_keys_and_the_bookmarked_lines_with_the_clipboard`) |
| MARK-10 | Next / previous bookmark (F2, Shift+F2) | supported | `JARAK-FR-100` | 101 | T | Step 101: F2 / Shift+F2, wrapping (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`, `bookmark_keys_and_the_bookmarked_lines_with_the_clipboard`) |
| MARK-11 | Clear all bookmarks | supported | `JARAK-FR-100` | 101 | T | Step 101: *Clear all bookmarks* (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`) |
| MARK-12 | Cut / copy / paste-replace / remove bookmarked lines | supported | `JARAK-FR-100` | 101 | T | Step 101: copy, cut, paste to (replace) line by line, remove the bookmarked lines, one undo step (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`, `bookmark_keys_and_the_bookmarked_lines_with_the_clipboard`) |
| MARK-13 | Remove unmarked lines | supported | `JARAK-FR-100` | 101 | T | Step 101: *Remove the lines without a bookmark* (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`) |
| MARK-14 | Inverse bookmarks | supported | `JARAK-FR-100` | 101 | T | Step 101: *Inverse bookmarks* (`navigate::tests::bookmarks_toggle_step_invert_and_take_their_lines`) |
| MARK-15 | Change history: go to next / previous change, clear history | supported | `JARAK-FR-101` | 101 | T | Step 101: modified / saved bars in the margin, next / previous change, clear (`navigate::tests::the_change_history_marks_edits_and_saving`, `the_change_history_shows_in_the_margin_and_hidden_lines_come_back`; real desktop pending) |

### FIF — Find in Files and the search results window (sources S)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FIF-01 | Find in Files: directory, filters, recursion, hidden folders | supported | `JARAK-FR-073` | 95 | T | Step 95: the Files page (Ctrl+Shift+F): folder with a folder dialog, filters, sub-folders, hidden folders; binary and oversized files skipped and listed (`infiles::tests::find_in_files_walks_filters_and_skips_binary_and_hidden`, `jarak find <folder>` in e2e step95, real desktop r95) |
| FIF-02 | Filters with exclusions (!pattern, !+\folder) | supported | `JARAK-FR-073` | 95 | T | Step 95: `!name`, `!\folder` (top level), `!+\folder` (any depth), `*`/`?`, `*.*` (`infiles::tests::filters_parse_like_notepad_plus_plus`, e2e step95) |
| FIF-03 | Follow the current document's folder | supported | `JARAK-FR-073` | 95 | T | Step 95: *Follow the current document* (and the document's folder by default) (`find_in_files_follows_the_document_folder_and_copies_one_line`) |
| FIF-04 | Replace in Files (with confirmation) | supported | `JARAK-FR-073` | 95 | T, M | Step 95: a preview with a check box per change and per file, then only the checked changes, each file verified against the preview, journaled and saved in its own encoding/BOM/line endings; *Undo the last Replace in Files*; `jarak replace --dry-run/--yes` (`infiles::tests` byte-level, `replace_in_files_previews_applies_only_checked_changes_and_undoes`, e2e step95, real desktop r95 with CP949 and CRLF) |
| FIF-05 | Search results window grouped by file, double-click to open | supported | `JARAK-FR-073` | 95 | T, M | Step 95: the results panel (F7) groups each search by file with the line and the match highlighted; a double-click opens it (`find_in_files_lists_results_by_file_and_f4_opens_each_match`, UI audit `results`, real desktop r95 double-click) |
| FIF-06 | Next / previous search result (F4, Shift+F4) | supported | `JARAK-FR-073` | 95 | T | Step 95: F4 / Shift+F4 across files, wrapping (window test; real desktop r95 via the palette's next/previous result, F-keys being other programs' global hotkeys there) |
| FIF-07 | Copy selected / all result lines and paths | supported | `JARAK-FR-073` | 95 | T | Step 95: a row's menu copies the line, every line or the paths; *Copy lines* / *Copy paths* buttons (`find_in_files_lists_results_by_file_and_f4_opens_each_match`, `find_in_files_follows_the_document_folder_and_copies_one_line`) |
| FIF-08 | Find in Projects (project panels) | not intended | — | 110 | D | — |

### VIEW — View menu: window, symbols, zoom and wrap (sources V, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| VIEW-01 | Always on top | partial | `JARAK-FR-036` | 110 | T | Step 110: View → Always on top: on/off; the window is told once per change (`window_modes_toggle_and_hide_the_chrome`); `--always-on-top` (`HELP-07`); partial: the real window's level not yet seen on the desktop |
| VIEW-02 | Toggle full screen (F11) | partial | `JARAK-FR-036` | 110 | M | Step 110: F11 / View → Full screen; the window is told once per change (`window_modes_toggle_and_hide_the_chrome`); partial: the real full-screen window not yet seen on the desktop |
| VIEW-03 | Post-it mode (F12: borderless window) | partial | `JARAK-FR-036` | 110 | M | Step 110: F12 / View → Post-it mode: no window frame, no menu, tab or status bar (`window_modes_toggle_and_hide_the_chrome`; audit screen `post-it`); partial: the real frameless window not yet seen on the desktop |
| VIEW-04 | Distraction-free mode | partial | `JARAK-FR-036` | 110 | M | Step 110: View → Distraction-free mode: full screen without the bars, the function list or the map, the text in a centred column (`window_modes_toggle_and_hide_the_chrome`; audit screen `distraction-free`); partial: the real full-screen window not yet seen on the desktop |
| VIEW-05 | View the current file in a browser (explicit) | supported | `JARAK-FR-111` | 102 | T | Step 102: HTML, SVG and XML files in the default browser, others refused with the reason (`run_commands_show_the_exact_line_and_run_only_on_the_button`, `the_manager_changes_shortcuts_and_removes_commands_and_html_opens_in_the_browser`) |
| VIEW-06 | Show spaces and tabs | supported | `JARAK-FR-037` | 103 | T | Step 103: dots for spaces, arrows for tabs (`show_symbol_settings_reach_the_view_and_binary_content_keeps_nul`, UI audit `symbols`; real desktop pending) |
| VIEW-07 | Show end of line | supported | `JARAK-FR-037` | 103 | T | Step 103: a CRLF / LF / CR / NEL / LS / PS tag after each line (`show_symbol_settings_reach_the_view_and_binary_content_keeps_nul`, UI audit `symbols`; real desktop pending) |
| VIEW-08 | Show non-printing and control characters, Unicode EOL | supported | `JARAK-FR-037` | 103 | T | Step 103: control characters drawn as their names in a box, Unicode line endings tagged (`view::tests::control_characters_have_names_when_shown`, UI audit `symbols`) |
| VIEW-09 | Show all characters | supported | `JARAK-FR-037` | 103 | T | Step 103: *Show all characters* toggles spaces, line endings and control characters together (`show_symbol_settings_reach_the_view_and_binary_content_keeps_nul`) |
| VIEW-10 | Show indent guide | supported | `JARAK-FR-037` | 103 | T | Step 103: dotted guides at each level, blank lines from their neighbours (UI audit `symbols`, headless audit test; real desktop pending) |
| VIEW-11 | Show wrap symbol | supported | `JARAK-FR-037` | 103 | T | Step 103: a wrap arrow at the end of each wrapped row (UI audit `symbols`; real desktop pending) |
| VIEW-12 | Zoom in / out / restore (Ctrl+wheel, Ctrl+Num+/-/) | supported | `JARAK-FR-034` | 88 | T | Step 88: Ctrl+=, Ctrl+-, Ctrl+0, Ctrl+wheel (window test) |
| VIEW-13 | Word wrap | supported | `JARAK-FR-030` | 88 | T | Step 88: Alt+Z, wrapped selections and caret (window test; audited `editor-wrap` view) |
| VIEW-14 | Hide lines | supported | `JARAK-FR-104` | 101 | T | Step 101: Alt+H, the mark above shows them again, *Show all hidden lines* (`navigate::tests::hidden_lines_hide_and_show_and_the_summary_counts`, `the_change_history_shows_in_the_margin_and_hidden_lines_come_back`) |
| VIEW-15 | Summary (characters, words, lines, bytes) | supported | `JARAK-FR-104` | 101 | T | Step 101: path, created, modified, characters, words, lines, bytes as saved, the selection (`navigate::tests::hidden_lines_hide_and_show_and_the_summary_counts`, `quick_open_finds_open_documents_and_the_workspaces_files`, UI audit `summary`) |
| VIEW-16 | Text direction right-to-left / left-to-right | not intended | — | 110 | M | Step 110: egui lays text out left to right without bidirectional reordering; right-to-left scripts are shown in logical order ([JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md)). A right-to-left mode would need a bidi layout under the whole editor (carets, selections, columns); not offered in v1 |
| VIEW-17 | Monitoring (tail -f) | supported | `JARAK-FR-122` | 104 | T | Step 104: *Monitoring (tail -f)* follows the file read-only at its end, a grown file gets the new lines (`monitoring_follows_a_growing_file_read_only_at_its_end`) |

### VIEWS — View → documents, views and panels (sources V, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| VIEWS-01 | Move current document to the other view | supported | `JARAK-FR-061` | 93 | T, M | Step 93: move to the other view (tab menu, palette, Window menu; window test, e2e) |
| VIEWS-02 | Clone current document to the other view | supported | `JARAK-FR-061` | 93 | T, M | Step 93: clone — one buffer, a view and carets per pane, one undo history (window test, e2e) |
| VIEWS-03 | Move / open the document in a new instance | supported | `JARAK-FR-002` | 93 | T | Step 93: open / move the document to a new window (`--new-window`; saved documents; window test) |
| VIEWS-04 | Focus on another view (F8) | supported | `JARAK-FR-061` | 93 | T | Step 93: F8 (window test) |
| VIEWS-05 | Synchronize vertical / horizontal scrolling | supported | `JARAK-FR-061` | 93 | T | Step 93: synchronized vertical and horizontal scrolling keep the views' distance (palette, Window menu) |
| VIEWS-06 | Go to tab 1–9, next / previous tab | supported | `JARAK-FR-060` | 93 | T | Step 93: Ctrl+1…9 (9 the last), Ctrl+Tab / Ctrl+Shift+Tab within the focused view (window test) |
| VIEWS-07 | Move tab forward / backward | supported | `JARAK-FR-060` | 93 | T | Step 93: Ctrl+Shift+PageUp/PageDown and dragging a tab (window test) |
| VIEWS-08 | Tab colouring | supported | `JARAK-FR-060` | 110 | T | Step 110: Window → Tab colour (five design-system colours, or none) drawn along the tab's edge and named in its accessible label; the session keeps it (`a_tab_takes_a_colour_and_the_session_keeps_it`, `a_session_round_trips_through_its_file`, audit screen `tab-color`) |
| VIEWS-09 | Document Map panel | supported | `JARAK-FR-102` | 101 | T, M | Step 101: bars per row, the lines on screen framed, click and drag; 10 MB: first frame 0.8 ms, keystroke p95 0.85 ms (`the_document_map_scrolls_and_the_navigation_history_goes_back_and_forward`, `the_document_map_of_a_10_mb_document`, UI audit `docmap`; real desktop pending) |
| VIEWS-10 | Document List panel | supported | `JARAK-FR-064` | 93 | T | Step 93: the side panel's Documents page (window test, UI audit) |
| VIEWS-11 | Function List panel | supported | `JARAK-FR-093` | 99 | T, M | Step 99: symbols per language and Markdown/HTML headings, filter, jump, next/previous symbol, rows in view only (`structure::tests`::symbols_of_each_language_family, `the_function_list_lists_filters_and_jumps_to_symbols`, headless audit, UI audit `functions`; real desktop pending) |
| VIEWS-12 | Folder as Workspace panel | supported | `JARAK-FR-063` | 93 | T, M | Step 93: the side panel's Folder page: lazy tree, filter, Refresh (unit + window test, UI audit) |
| VIEWS-13 | Project panels (1–3, .workspace files) | not intended | — | 110 | D | — |

### FOLD — View → folding (sources V)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FOLD-01 | Fold all / unfold all | supported | `JARAK-FR-092` | 99 | T | Step 99: Alt+0 / Alt+Shift+0 (`folding_by_keys_and_the_margin_hides_lines_that_moves_skip`); 10 MB: fold all 74 ms, typing with all folded p95 9.23 ms |
| FOLD-02 | Collapse / uncollapse current level | supported | `JARAK-FR-092` | 99 | T | Step 99: Ctrl+Alt+F / Ctrl+Alt+Shift+F, the innermost fold around the caret (`folding_by_keys_and_the_margin_hides_lines_that_moves_skip`) |
| FOLD-03 | Collapse / uncollapse level 1–8 | supported | `JARAK-FR-092` | 99 | T | Step 99: Alt+1…8 / Alt+Shift+1…8 by fold level (`folding_by_keys_and_the_margin_hides_lines_that_moves_skip`) |
| FOLD-04 | Fold margin with mouse folding | supported | `JARAK-FR-092` | 99 | M | Step 99: triangles between the line numbers and the text, a click folds/unfolds and leaves the caret (`folding_by_keys_and_the_margin_hides_lines_that_moves_skip`, headless audit; real desktop pending) |
| FOLD-05 | Folding by syntax for built-in languages | supported | `JARAK-FR-092` | 99 | T | Step 99: brackets, indentation, sections, tags, headings and fences, block comments per language (`structure::tests`::brackets_fold_outside_strings_and_comments_with_levels, indentation_sections_markdown_and_tags_fold, `python_folds_by_indentation`) |
| FOLD-06 | Folding of user-defined languages by markers | supported | `JARAK-FR-092` | 99 | T | Step 99: a user language's `fold` markers (`structure::tests`::indentation_sections_markdown_and_tags_fold, `syntax::tests::a_user_defined_language_from_json_highlights_and_folds`) |

### ENC — Encoding menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ENC-01 | Interpret as ANSI (system code page) | supported | `JARAK-FR-025` | 103 | T | Step 103: *Reinterpret as ANSI* (the system code page) (palette, `reinterpret_reads_the_bytes_again_and_asks_before_losing_changes` for the path) |
| ENC-02 | Interpret as UTF-8 | supported | `JARAK-FR-021` | 87 | T | Step 87: UTF-8 detected and saved byte for byte (corpus test, e2e); Step 103: *Reinterpret as UTF-8* in the Encoding menu (`file::tests::utf8_and_utf16_with_and_without_bom_are_reinterpreted`) |
| ENC-03 | Interpret as UTF-8-BOM | supported | `JARAK-FR-021` | 87 | T | Step 87: UTF-8 BOM detected and kept (corpus test, e2e); Step 103: *Reinterpret as UTF-8-BOM* (`file::tests::utf8_and_utf16_with_and_without_bom_are_reinterpreted`) |
| ENC-04 | Interpret as UTF-16 BE BOM / UTF-16 LE BOM | supported | `JARAK-FR-021` | 87 | T | Step 87: UTF-16 LE/BE with BOM detected and kept (corpus test, e2e); Step 103: *Reinterpret as UTF-16 BE / LE BOM* (`file::tests::utf8_and_utf16_with_and_without_bom_are_reinterpreted`) |
| ENC-05 | Character sets: Korean (EUC-KR / Windows-949) | supported | `JARAK-FR-025` | 103 | T | Step 103: EUC-KR / Windows-949 round trip and reinterpret (`file::tests::every_listed_character_set_round_trips_and_is_reinterpreted`, `file::tests::utf8_and_utf16_with_and_without_bom_are_reinterpreted`, real `jarak info`) |
| ENC-06 | Character sets: Japanese (Shift-JIS, EUC-JP) | supported | `JARAK-FR-025` | 103 | T | Step 103: Shift-JIS, EUC-JP, ISO-2022-JP round trips; Shift-JIS reinterpreted in the window (`every_listed_character_set_round_trips_and_is_reinterpreted`, `reinterpret_reads_the_bytes_again_and_asks_before_losing_changes`, real `jarak info`) |
| ENC-07 | Character sets: Chinese (GB2312/GBK, Big5) | supported | `JARAK-FR-025` | 103 | T | Step 103: GB2312/GBK, GB18030, Big5 round trips (`every_listed_character_set_round_trips_and_is_reinterpreted`, real `jarak info`) |
| ENC-08 | Character sets: Cyrillic (Windows-1251, KOI8-R, KOI8-U, ISO 8859-5) | supported | `JARAK-FR-025` | 103 | T | Step 103: Windows-1251, KOI8-R, KOI8-U, ISO 8859-5, OEM 866 round trips (`every_listed_character_set_round_trips_and_is_reinterpreted`, real `jarak info`) |
| ENC-09 | Character sets: Western and Central European (Windows-1252/1250, ISO 8859-1/2/15) | supported | `JARAK-FR-025` | 103 | T | Step 103: Windows-1252 (ISO 8859-1 as WHATWG), ISO 8859-15, Windows-1250, ISO 8859-2 round trips; converting to Windows-1252 saved (`every_listed_character_set_round_trips_and_is_reinterpreted`, `convert_names_what_it_cannot_hold_and_saves_in_the_new_encoding`) |
| ENC-10 | Character sets: Greek, Turkish, Hebrew, Arabic, Baltic, Vietnamese, Thai | supported | `JARAK-FR-025` | 103 | T | Step 103: Greek, Turkish, Hebrew, Arabic, Baltic, Vietnamese (Windows-1258), Thai round trips (`every_listed_character_set_round_trips_and_is_reinterpreted`, real `jarak info`) |
| ENC-11 | Convert to ANSI | supported | `JARAK-FR-025` | 103 | T | Step 103: *Convert to ANSI* with the lossy question (`convert_names_what_it_cannot_hold_and_saves_in_the_new_encoding` for the path) |
| ENC-12 | Convert to UTF-8 / UTF-8-BOM | supported | `JARAK-FR-025` | 103 | T | Step 103: *Convert to UTF-8 / UTF-8-BOM* (palette; `file::tests::utf8_and_utf16_with_and_without_bom_are_reinterpreted`) |
| ENC-13 | Convert to UTF-16 BE BOM / UTF-16 LE BOM | supported | `JARAK-FR-025` | 103 | T | Step 103: *Convert to UTF-16 BE / LE BOM*, saved with its BOM (`convert_names_what_it_cannot_hold_and_saves_in_the_new_encoding`) |
| ENC-14 | Automatic encoding detection on open | supported | `JARAK-FR-021` | 87 | T | Step 87: BOM, UTF-8, UTF-16 without BOM, then the locale's legacy code page (tests, `jarak info`, e2e `step87`); Step 88: the detected encoding in the status bar (`the_status_bar_carries_the_gear_at_its_right_end_and_no_toolbar` reads its `UTF-8` item); Step 103: reinterpreting and the encoding tests |
| ENC-15 | Default encoding for new documents (setting) | supported | `JARAK-FR-021` | 87 | T | Step 103: *New documents in UTF-8 / UTF-8-BOM / ANSI* (setting `new_encoding`) (`line_endings_convert_and_new_documents_take_the_settings_encoding`) |
| ENC-16 | Status-bar encoding and EOL indicators (clickable) | supported | `JARAK-FR-035` | 88 | T, M | Step 88: encoding label and line ending (or Mixed EOL) in the status bar (window tests); clickable in Step 103; Step 103: a click on the encoding or the line ending opens its entries (`convert_names_what_it_cannot_hold_and_saves_in_the_new_encoding`, `line_endings_convert_and_new_documents_take_the_settings_encoding`) |

### LANG — Language menu and syntax highlighting (sources L, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LANG-01 | Built-in language highlighting (the JARAK-FR-090 list) | supported | `JARAK-FR-090` | 98 | T | Step 98: 23 languages, own line-state lexers (ADR 0103), golden fixtures for each (`syntax::tests`: c_and_cpp … sql_markdown_lua_php_ruby, UI audit `syntax`); 10 MB: 164 ms to lex to the end, keystroke frame p95 0.29 ms |
| LANG-02 | Further Notepad++ languages (Ada, ASM, AutoIt, COBOL, D, Fortran, Haskell, Lisp, Pascal, Perl, R, Scheme, Swift, VHDL, Verilog, …) | partial | `JARAK-FR-090` | 110 | T | Step 110: Ada, Assembly, AutoIt, COBOL, D, Fortran, Haskell, Lisp, Pascal, Perl, R, Scheme, Swift, VHDL and Verilog are built in, detected by extension (and Perl, R by `#!`) (`the_further_notepad_plus_plus_languages`, e2e `step110_jarak_parity`); partial: Notepad++'s other languages (for example Erlang, OCaml, Tcl, Nim, fixed-form COBOL's column-7 comments) are not built in — they can be added as user-defined languages (Step 98) |
| LANG-03 | Language detection by extension | supported | `JARAK-FR-090` | 98 | T | Step 98: by extension, user languages first (`syntax::tests`::every_listed_language_is_built_in_and_detected, `a_rust_file_is_highlighted_and_the_language_menu_changes_it`) |
| LANG-04 | Language detection by first line / shebang | supported | `JARAK-FR-090` | 98 | T | Step 98: `#!` lines (with `env` and version suffixes) and `<?xml` (`syntax::tests`::every_listed_language_is_built_in_and_detected) |
| LANG-05 | Manual language selection (Language menu) | supported | `JARAK-FR-090` | 98 | T | Step 98: the Language menu in the palette and the menu bar, the status bar names the language (`a_rust_file_is_highlighted_and_the_language_menu_changes_it`, UI audit `palette-language`) |
| LANG-06 | Normal text (no highlighting) | supported | `JARAK-FR-090` | 98 | T | Step 98: *Language: Normal text*, and undetected files (`a_rust_file_is_highlighted_and_the_language_menu_changes_it`) |
| LANG-07 | User Defined Language: define keywords, comments, operators, delimiters, numbers | supported | `JARAK-FR-091` | 98 | T | Step 98: JSON user languages — keywords (3 lists), comments, operators, delimiters with escapes, numbers (`syntax::tests`::a_user_defined_language_from_json_highlights_and_folds, `user_languages_load_import_from_notepad_plus_plus_and_export`) |
| LANG-08 | User Defined Language: folding markers | supported | `JARAK-FR-091` | 98 | T | Step 98: `fold` markers in user languages and in imported UDLs, matched outside comments and strings into fold ranges (`syntax::tests`::a_user_defined_language_from_json_highlights_and_folds); Step 99: folded in the window with the margin and the fold commands |
| LANG-09 | Import / export of user-defined languages | supported | `JARAK-FR-091` | 98 | T | Step 98: import (Notepad++ XML → JSON, never replacing a file) and export of the current language as JSON (`syntax::tests`::a_notepad_plus_plus_udl_is_imported_and_highlights_a_sample, `user_languages_load_import_from_notepad_plus_plus_and_export`) |
| LANG-10 | Import Notepad++ UDL XML files | supported | `JARAK-FR-091` | 98 | T | Step 98: UDL 2.x XML: name, extensions, case, comments, Keywords1–8, operators, delimiters, code folding; styles, prefixes, number and comment-folding rules are not taken (documented) (`syntax::tests`::a_notepad_plus_plus_udl_is_imported_and_highlights_a_sample) |
| LANG-11 | Embedded languages (script and style inside HTML, PHP) | supported | `JARAK-FR-090` | 98 | T | Step 98: JavaScript in `<script>`, CSS in `<style>`, PHP blocks in PHP files, fenced code in Markdown (`syntax::tests`::html_with_script_and_style_css_xml, sql_markdown_lua_php_ruby) |

### SET — Settings menu (sources R, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| SET-01 | Preferences: General (localization, status bar, menu) | supported | `JARAK-FR-130` | 105 | T, M | Step 105: Settings → General: language (System / English / 한국어), the status bar, the menu bar (with Alt / always) (`a_changed_setting_is_written_and_applied_at_once`, `multi_editing_and_the_menu_bar_follow_the_settings`) |
| SET-02 | Preferences: Tab bar options (close button, double-click to close, multi-line) | partial | `JARAK-FR-130` | 105 | T | Step 105: close buttons on every tab or only the active / hovered one, double-click closes a tab; multi-line tab bars are not implemented |
| SET-03 | Preferences: Editing (caret, current line, line wrap, multi-editing, scrolling beyond last line) | partial | `JARAK-FR-130` | 105 | T | Step 105: current-line highlight, a blinking caret (off by default), word wrap, multi-editing (Ctrl+click), virtual space; scrolling beyond the last line is not implemented |
| SET-04 | Preferences: Dark mode | supported | `JARAK-FR-132` | 105 | T, M | Step 105: theme System / Dark / Light from `kim-design`, live (`a_changed_setting_is_written_and_applied_at_once`; ui-audit dark and light) |
| SET-05 | Preferences: Margins, border, edge (line numbers, bookmark margin, edge column) | partial | `JARAK-FR-130` | 105 | T | Step 105: line numbers on/off, edge lines at columns; the bookmark margin cannot be hidden |
| SET-06 | Preferences: New document (format, encoding, default language) | supported | `JARAK-FR-130` | 105 | T | Step 105: new documents' encoding and line ending (`tab_width_spaces_and_new_documents_follow_the_settings`); Step 110: their language (`new_language`, `new_language_open_folder_find_fill_and_backup_settings_apply`) |
| SET-07 | Preferences: Default directory | supported | `JARAK-FR-130` | 105 | T | Step 110: the Open dialog starts in the current document's folder, Windows' last folder or a set folder (`open_folder`, `open_folder_path`; a folder that is gone falls back to the document's) (`new_language_open_folder_find_fill_and_backup_settings_apply`) |
| SET-08 | Preferences: Recent files history size | supported | `JARAK-FR-130` | 105 | T | Step 105: *Recent files kept* (0–50) |
| SET-09 | Preferences: File association (explicit, reversible) | partial | `JARAK-FR-130` | 111 | M | Step 111: Help → *Offer Jarak in "Open with"…* and `jarak associate add/remove` — per user, one ProgID and an `OpenWithProgids` entry per checked extension, never a default program, programs and scripts refused, taken back exactly (`offering_jarak_in_open_with_and_taking_it_back`, core `removing_takes_back_exactly_what_was_added`, `the_real_registry_offer_and_removal_under_a_test_key` on the real registry under a private test key, CLI `associate_offers_refuses_and_takes_back`, e2e `step111_jarak_release` dry runs); partial: Explorer's real *Open with* list not yet checked on the desktop (it would change the maintainer's own account) |
| SET-10 | Preferences: Language (tab size per language, replace by space) | supported | `JARAK-FR-130` | 105 | T | Step 105: tab width and *Tab inserts spaces* for every document (`tab_inserts_spaces_to_the_next_stop_when_set`); Step 110: a language's own tab width and spaces (`language_tabs`, `a_language_has_its_own_tab_width`) |
| SET-11 | Preferences: Highlighting (smart highlight, mark-all options) | supported | `JARAK-FR-130` | 105 | T | Step 110: Settings → Editor → Highlighting — smart highlighting on/off, match case, whole words (`smart_highlighting_word_characters_and_the_length_summary`); Mark All keeps its own case and whole-word options on the find panel's Mark page (`mark_all_bookmarks_lines_clears_and_copies_marks`) |
| SET-12 | Preferences: Print (margins, header, footer, colour mode) | supported | `JARAK-FR-140` | 106 | T | Step 106: Settings → Files → Printing — header and footer (left, centre and right parts with `{file}`, `{path}`, `{page}`, `{pages}`, `{date}`, `{time}`), line numbers, colour or black on white, text size, four margins; live for the next job (`the_print_settings_apply_to_the_next_job`); header/footer fonts are not settable |
| SET-13 | Preferences: Searching (minimum size, fill find field, confirm replace all) | partial | `JARAK-FR-130` | 105 | T | Step 110: *Selected text fills the find field* (`find_fill_selection`, `new_language_open_folder_find_fill_and_backup_settings_apply`); partial: no minimum size for *in selection* and no Replace All confirmation setting (Replace All is one undo step; Replace in Files always previews, Step 95) |
| SET-14 | Preferences: Backup (session snapshot, periodic backup, backup on save) | supported | `JARAK-FR-121` | 104 | T | Step 104: the session (Step 93) and periodic recovery snapshots, on/off and the interval (`recovery_interval`); Step 110: backup on save, simple (`<name>.bak`) or dated copies in `jarak-backup` (`backup`; `a_backup_copies_the_file_beside_it`, `new_language_open_folder_find_fill_and_backup_settings_apply`; a backup that fails stops the save) |
| SET-15 | Preferences: Auto-completion | supported | `JARAK-FR-094` | 105 | T | Step 105: the completion and typing-aid settings of Step 100 in Settings → Editor |
| SET-16 | Preferences: Multi-instance and date format | supported | `JARAK-FR-002` | 105 | T | Step 105: *One window* (off: every start opens its own window) and the custom date format |
| SET-17 | Preferences: Delimiter (word characters, double-click delimiters) | partial | `JARAK-FR-130` | 105 | T | Step 110: further word characters for double-click, Ctrl+arrows and select word (`word_chars`; `further_word_characters_join_words`, `smart_highlighting_word_characters_and_the_length_summary`); partial: Notepad++'s Ctrl+double-click delimiter selection is not implemented |
| SET-18 | Preferences: Performance (large file restriction) | supported | `JARAK-NFR-004` | 109 | T | Step 109: Settings → Files → Performance — *Large file from (MB)* (200 by default) (`a_large_file_is_plain_text_without_wrap_or_map`, [JARAK_PERFORMANCE](JARAK_PERFORMANCE.md)) |
| SET-19 | Preferences: Cloud settings path | not intended | — | 105 | D | — |
| SET-20 | Preferences: Search engine for 'search on internet' | supported | `JARAK-FR-111` | 105 | T | Step 105: the search engine address, checked to hold `$(QUERY)` |
| SET-21 | Preferences: MISC (file status auto-detection, document peeker, smooth font, clickable links) | partial | `JARAK-FR-120` | 105 | T | Step 105: file status auto-detection (Step 104) in Settings → Files; document peeker, smooth font and clickable-link options are not implemented |
| SET-22 | Style Configurator (themes, per-language token colours, global font) | partial | `JARAK-FR-132` | 105 | T, M | Step 105: themes and the editor font family and size (Windows' fonts when present); per-language token colours are not implemented |
| SET-23 | Shortcut Mapper (main menu, macros, run commands, editor commands; conflicts) | supported | `JARAK-FR-112` | 105 | T, M | Step 105: Settings → Keyboard and `jarak hotkeys` on the shared hotkey contract — every command with keys (Change by recording, Off, Reset, Take it for a clash, Reset all), conflicts checked, the user's changes listed first (`a_remapped_command_runs_from_its_new_keys`, `the_keyboard_section_records_refuses_and_resets`, e2e `step105_jarak_settings`); macros and run commands keep their own shortcut fields (Step 102); the navigation and editing keys every Kim window reserves are not remappable |
| SET-24 | Import style themes | not intended | — | 105 | T | Step 110: Jarak's themes are the Kim design system's (`kim-design` tokens, contrast-audited, the same in every Kim tool); a Notepad++ style theme would bring colours the design audit cannot vouch for. User-defined languages keep their token kinds (Step 98) |
| SET-25 | Edit the context menu | supported | `JARAK-FR-130` | 110 | T | Step 110: Settings → Advanced → Context menu of the text: palette command ids and `-` separators; unknown ids are skipped (`the_text_context_menu_runs_the_settings_commands`, `the_default_context_menu_names_real_commands`) |
| SET-26 | Localization (Korean, English) | supported | `JARAK-FR-131` | 105 | T, M | Step 105: English and Korean for every string, following Windows unless set (ui-audit `ko` runs of every Settings section) |

### TOOL — Tools menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| TOOL-01 | MD5: generate, from files, from selection to clipboard | supported | `JARAK-FR-088` | 97 | T | Step 97: of the selection or a file to the clipboard, and the *Generate a digest* dialog; RFC 1321 test vectors (`lineops::tests`::digests_match_the_standard_test_vectors, `dates_paths_digests_and_the_copy_line_setting`, UI audit `hash`) |
| TOOL-02 | SHA-1: generate, from files, from selection | supported | `JARAK-FR-088` | 97 | T | Step 97: the same; FIPS 180 test vectors (`lineops::tests`::digests_match_the_standard_test_vectors, window test) |
| TOOL-03 | SHA-256: generate, from files, from selection | supported | `JARAK-FR-088` | 97 | T | Step 97: the same with `sha2` (`lineops::tests`::digests_match_the_standard_test_vectors, window test, e2e step97) |
| TOOL-04 | SHA-512: generate, from files, from selection | supported | `JARAK-FR-088` | 97 | T | Step 97: the same with `sha2` (`lineops::tests`::digests_match_the_standard_test_vectors) |

### MAC — Macro menu (sources M)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MAC-01 | Start / stop recording (Ctrl+Shift+R) | supported | `JARAK-FR-110` | 102 | T, M | Step 102: Ctrl+Shift+R, commands and typed text recorded, *● Recording* in the status bar (`a_recorded_macro_plays_back_as_one_undo_step_and_runs_to_the_end_safely`; real desktop pending) |
| MAC-02 | Playback (Ctrl+Shift+P in Notepad++; Jarak keys configurable) | supported | `JARAK-FR-110` | 102 | T | Step 102: F9 (Notepad++'s Ctrl+Shift+P is Jarak's palette), each run one undo step (`a_recorded_macro_plays_back_as_one_undo_step_and_runs_to_the_end_safely`) |
| MAC-03 | Save the recorded macro with a name and shortcut | supported | `JARAK-FR-110` | 102 | T | Step 102: a name and a shortcut checked against reserved keys, Jarak's keys, typing keys and each other; `macros.json` (`saved_macros_get_shortcuts_checked_against_reserved_and_jarak_keys`) |
| MAC-04 | Run a macro multiple times / until the end of the file | supported | `JARAK-FR-110` | 102 | T | Step 102: a number of times or until the end of the file, stopping at the end, without progress, on a wrap, at 100 000 runs or 20 s (`a_recorded_macro_plays_back_as_one_undo_step_and_runs_to_the_end_safely`) |
| MAC-05 | Built-in 'Trim trailing space and save' macro | supported | `JARAK-FR-110` | 102 | T | Step 102: *Trim Trailing Space and Save* (`saved_macros_get_shortcuts_checked_against_reserved_and_jarak_keys`) |
| MAC-06 | Modify shortcut / delete macro | supported | `JARAK-FR-110` | 102 | T | Step 102: the manager changes a shortcut (checked) and removes a macro (`the_manager_changes_shortcuts_and_removes_commands_and_html_opens_in_the_browser`, `saved_macros_get_shortcuts_checked_against_reserved_and_jarak_keys`) |
| MAC-07 | Search and replace actions are macro-recordable | supported | `JARAK-FR-110` | 102 | T | Step 102: Find Next, Replace and Replace All with their options; playback leaves the panel's own (`find_and_replace_are_recorded_with_their_options`) |

### RUN — Run menu (sources X)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| RUN-01 | Run… a command with variables ($(FULL_CURRENT_PATH), $(CURRENT_WORD), …) | supported | `JARAK-FR-111` | 102 | T | Step 102: F5, Notepad++'s variables, the exact line shown, run only on the button, no shell, never elevated (`macros::tests::variables_expand_and_lines_split_into_targets`, `run_commands_show_the_exact_line_and_run_only_on_the_button`, `the_process_runner_starts_programs_and_names_failures`) |
| RUN-02 | Save run commands with a name and shortcut | supported | `JARAK-FR-111` | 102 | T | Step 102: saved with a name and a shortcut; a saved one asks with its exact line (`run_commands_show_the_exact_line_and_run_only_on_the_button`) |
| RUN-03 | Modify shortcut / delete command | supported | `JARAK-FR-111` | 102 | T | Step 102: the manager's Run commands page changes a shortcut and removes a command (`the_manager_changes_shortcuts_and_removes_commands_and_html_opens_in_the_browser`) |
| RUN-04 | Launch in browsers / Wikipedia / PHP help entries | supported | `JARAK-FR-111` | 102 | T | Step 102: built-in *Open the file with its default program*, *Wikipedia search*, *PHP help* (`run_commands_show_the_exact_line_and_run_only_on_the_button`) |
| RUN-05 | Open the file in another instance | supported | `JARAK-FR-002` | 93 | T | Step 93: Open in a new window (the same as VIEWS-03) |

### PLUG — Plugins menu and bundled plugin functions (sources G)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| PLUG-01 | Load third-party Notepad++ plugin DLLs | not intended | — | 108 | D | Step 108: Notepad++ plugin DLLs talk to Notepad++'s and Scintilla's window messages (`NPPM_*`, `SCI_*`), which Jarak does not have; they are never loaded ([JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)) |
| PLUG-02 | Plugins Admin (download and update plugins from a list) | not intended | — | 108 | D | Step 108: nothing is downloaded or updated (`KIM-SEC-001`); extensions are installed from a local folder by an explicit command ([JARAK_EXTENSIONS](JARAK_EXTENSIONS.md)) |
| PLUG-03 | MIME Tools: Base64 encode / decode (also with padding / URL-safe) | supported | `JARAK-FR-088` | 97 | T | Step 97: Base64 encode (padded, unpadded, URL-safe) and decode of the selection, a refused decode changes nothing (`lineops::tests`::every_codec_round_trips_unicode, window test, real desktop r97) |
| PLUG-04 | MIME Tools: URL encode / decode | supported | `JARAK-FR-088` | 97 | T | Step 97: URL encode / decode (`lineops::tests`::every_codec_round_trips_unicode) |
| PLUG-05 | MIME Tools: quoted-printable encode / decode | supported | `JARAK-FR-088` | 97 | T | Step 97: quoted-printable encode / decode, soft breaks at 76 (`lineops::tests`::every_codec_round_trips_unicode) |
| PLUG-06 | Converter: ASCII to hex / hex to ASCII | supported | `JARAK-FR-088` | 97 | T | Step 97: ASCII to hex / hex to ASCII, odd or non-hex input refused (`lineops::tests`::every_codec_round_trips_unicode, window test) |
| PLUG-07 | NppExport: export to RTF / HTML, copy as RTF / HTML | partial | `JARAK-FR-141` | 106 | T | Step 106: built in — File → Export as HTML / RTF (highlighted text; Markdown rendered with embedded images), Publish Markdown to a folder, Edit → Copy as HTML / RTF (`HTML Format` / `Rich Text Format` beside the text), `jarak export` (golden tests `export_golden.rs`, window flows `code_exports_as_rtf_and_copies_as_html_and_rtf`, `exporting_markdown_embeds_its_images_and_keeps_the_document`, e2e `step106_jarak_export`); partial: pasting the copied formats into another program from the real Windows clipboard is not tested yet ([JARAK_EXPORT](JARAK_EXPORT.md)) |
| PLUG-08 | Safe extension mechanism (Jarak's own) | supported | `JARAK-FR-160` | 108 | T | Step 108: extensions — languages, snippets, commands running an .exe of the extension or a System32 program with the selection or document on stdin; permissions declared and granted for exactly the manifest (SHA-256), revocable; a Windows job, an empty folder, a minimal environment, time and output limits (`extensions::tests`, window flows `an_extension_command_asks_once_and_its_output_is_undoable`, e2e `step108_jarak_extensions`); the sample `examples/jarak-extensions/mouv.sample-tools` |
| PLUG-09 | Plugin compatibility audit document | supported | `JARAK-FR-161` | 108 | D | Step 108: [JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md) — 26 plugins with honest statuses (built in, partly, by extension, not covered, not intended) |

### WIN — Window menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| WIN-01 | Window list of open documents (activate) | supported | `JARAK-FR-064` | 93 | T | Step 93: the Window menu lists the open documents; a click shows one |
| WIN-02 | Windows… dialog: sort, activate, save, close several | supported | `JARAK-FR-064` | 93 | T | Step 93: the Documents… dialog sorts (name, folder, unsaved first), shows, saves and closes the checked ones (window test, UI audit) |

### HELP — ? menu and command line (sources C, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| HELP-01 | Command line: open files | supported | `JARAK-FR-003` | 85 | T | Step 85: `jarak open` / `jarak-gui <file>…` take the files (single-instance hand-off, real desktop); Step 88: they open as documents (`files_open_at_a_line_and_a_second_start_hands_files_over`, e2e `step88`) |
| HELP-02 | Command line: -n line / -c column / -p position | supported | `JARAK-FR-003` | 85 | T | Step 85: `file:line:column` parsed (`jarak-core` `target` tests, `open --dry-run` in e2e step85); Step 88: the caret goes there (`files_open_at_a_line_and_a_second_start_hands_files_over`) |
| HELP-03 | Command line: -l language / -udl | supported | `JARAK-FR-150` | 107 | T | Step 107: `--language <name>` (`jarak open`, `jarak-gui`; any case, the user's languages too, `text` for none) applies to the files after it, also through the hand-off to a running window (`language_and_read_only_options_apply_to_the_files_after_them`); Notepad++'s `-udl=` is the same option with a user language's name |
| HELP-04 | Command line: -ro read-only | supported | `JARAK-FR-150` | 107 | T | Step 107: `--read-only` opens the files after it read-only, with the reason in the status bar (`language_and_read_only_options_apply_to_the_files_after_them`, e2e `step107_jarak_cli_interop`) |
| HELP-05 | Command line: -nosession / -openSession | supported | `JARAK-FR-150` | 107 | T | Step 107: `jarak-gui --no-session` (no last session, none saved at exit, the saved one kept) and `--session <file>` (a saved session's documents; also through the hand-off) (`session_options_open_a_session_file_and_leave_the_saved_one`) |
| HELP-06 | Command line: -multiInst (separate instance) | supported | `JARAK-FR-002` | 85 | T | Step 85: `--new-window` / `jarak open --new-window` start a separate window; without it a second start hands its files to the running window (real desktop: title changed, second process exit 0) |
| HELP-07 | Command line: -x / -y window position, -alwaysOnTop | partial | `JARAK-FR-150` | 110 | T | Step 110: `jarak-gui --position x,y --always-on-top` for a new window (`a_position_is_two_numbers`; a bad position exits 2 before any window, e2e `step110_jarak_parity`); partial: the window's real place and level not yet checked on the desktop |
| HELP-08 | Command line: -qt / -qf ghost typing | not intended | — | 107 | D | — |
| HELP-09 | Built-in updater (WinGUp) | not intended | — | 111 | D | — |
| HELP-10 | Debug info (versions, paths) | supported | `JARAK-FR-150` | 107 | T | Step 107: `jarak debug-info` (version, programs, folders and files, the Windows build, printers; never contents) and Window → Copy debug info (`copy_debug_info_names_the_version_and_windows`, CLI `debug_info_names_the_version_and_windows`) |
| HELP-11 | Help / about / user manual link | supported | `JARAK-FR-171` | 111 | D | Step 111: Help → *User guide* opens `docs\JARAK_GUIDE.md` from beside the program (the package carries it) read-only in the preview; *About Jarak* (version, program, folders, licence, privacy, Copy debug info) (`the_help_menu_opens_the_guide_and_about`, `the_help_dialogs_pass_the_ui_audit_rules`, e2e `step111_jarak_release` checks the installed `docs\`); [JARAK_GUIDE](JARAK_GUIDE.md) |

### REL — Sessions, backup and file status (sources U, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| REL-01 | Remember the session at exit and restore it at start | supported | `JARAK-FR-062` | 93 | T | Step 93: session.json (schema, atomic), restored at start (a setting), untitled text kept (window test, e2e across a restart) |
| REL-02 | Session snapshot and periodic backup of dirty documents | supported | `JARAK-FR-121` | 104 | T, M | Step 104: dirty documents kept every 10 s in the window's own recovery folder, never the originals (`recovery` tests, `a_saved_document_leaves_its_snapshot_and_a_normal_exit_keeps_none`); real desktop: a killed test instance's text restored intact ([JARAK_RECOVERY](JARAK_RECOVERY.md)) |
| REL-03 | File status auto-detection with reload prompt | supported | `JARAK-FR-120` | 104 | T, M | Step 104: checked on focus and every 2 s off the UI thread; Reload / Keep this version, and Keep mine / Reload / Compare with unsaved changes; a reload is one undo step (`a_file_changed_on_disk_is_asked_about_and_reloads_as_one_undo_step`, `a_conflict_offers_keep_mine_reload_and_compare`, `the_window_checks_its_files_by_itself`); real desktop: noticed in under 1.1 s |
| REL-04 | Update silently (auto-reload unmodified documents) | supported | `JARAK-FR-120` | 104 | T | Step 104: *Reload unmodified documents silently* (off by default), still asking for a document with unsaved changes (`unmodified_documents_reload_silently_when_set`) |
| REL-05 | Deleted file kept as an unsaved document | supported | `JARAK-FR-120` | 104 | T | Step 104: the text stays, unsaved, and Save writes the file again (`a_deleted_file_stays_open_unsaved_and_save_writes_it_again`; real desktop) |
| REL-06 | Large-file mode (features turned off above a size) | supported | `JARAK-NFR-004` | 109 | T | Step 109: above the limit a document is plain text — no highlighting, folding, word wrap, Document Map or brace highlighting — and the status bar says *Large file* (`a_large_file_is_plain_text_without_wrap_or_map`, perf `a_10_mb_file_in_large_file_mode`) |

### UI — General user interface (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| UI-01 | Tab bar with dirty markers, close buttons and context menu | supported | `JARAK-FR-060` | 93 | T, M | Step 93: tab bars from the design tokens with dirty dots, close buttons, middle click, drag, context menu (window tests, UI audit 95 runs 0 failures) |
| UI-02 | Status bar: language, length/lines, position, EOL, encoding, insert/overwrite | supported | `JARAK-FR-035` | 88 | T, M | Step 88: Ln/Col, selection, encoding, EOL, language, INS/OVR (window tests, audited views); Step 110: the document's length and lines (`23 chars · 1 lines`, `smart_highlighting_word_characters_and_the_length_summary`) |
| UI-03 | Overwrite mode (Insert key) | supported | `JARAK-FR-031` | 88 | T | Step 88: Insert toggles overwrite; typing replaces the next grapheme, never a line break (core test, window test) |
| UI-04 | Line number margin | supported | `JARAK-FR-030` | 88 | T, M | Step 88: line numbers, the caret's line brighter (audited views, real screenshots) |
| UI-05 | Current line highlight | supported | `JARAK-FR-030` | 88 | T | Step 88: the caret's line highlighted (audited views) |
| UI-06 | Vertical edge line(s) at configured columns | supported | `JARAK-FR-037` | 103 | T | Step 103: edge lines at `edge_columns` (80 by default) (`show_symbol_settings_reach_the_view_and_binary_content_keeps_nul`, UI audit `symbols`; real desktop pending) |
| UI-07 | Clickable links (Ctrl+click) | supported | `JARAK-FR-046` | 89 | T | Step 89: links in the preview open with Ctrl+click only, web and mail only (window tests); a plain click explains |
| UI-08 | Context menu in the text | supported | `JARAK-FR-031` | 110 | M | Step 110: a right click on the text (either view) opens its context menu — Undo, Redo, Cut, Copy, Paste, Select all, Copy as HTML by default; an item runs its command (`the_text_context_menu_runs_the_settings_commands`; audited, `the_step_110_surfaces_pass_the_ui_audit_rules`) |
| UI-09 | Toolbar with icons | not intended | — | 88 | D | — |
| UI-10 | Menu bar (Jarak: hidden by default, shown with Alt or from the palette) | supported | `JARAK-FR-054` | 92 | T | Step 92: hidden by default, Alt alone (traced events) or the palette shows it; six menus with the keys (window test, e2e step92, real desktop) |

## 2. Jarak additions (not Notepad++ features)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| JRK-01 | Markdown: GitHub-flavoured preview rendered natively | supported | `JARAK-FR-040` | 89 | T, M | Step 89: GFM render model (7 tests) drawn natively (UI audit `preview`, real screenshot) |
| JRK-02 | Markdown: instant Source / Preview / Split toggle keeping caret and scroll | supported | `JARAK-FR-041` | 89 | T, M | Step 89: Ctrl+E / Esc / Ctrl+Shift+E keep caret, text and clean state and map the position (window tests; real desktop; 38 ms first switch of 1 MB) |
| JRK-03 | Markdown: Ctrl+V image paste into portable assets with a relative reference | supported | `JARAK-FR-042` | 90 | T, M | Step 90: Ctrl+V of PNG / CF_DIBV5 / CF_DIB / copied image files into `assets/<doc>-<date>-<n>.png`, never overwriting, one undo step (window MVP test; e2e step90; real desktop with the real clipboard) |
| JRK-04 | Markdown: image paste into an untitled document (explicit behaviour) | supported | `JARAK-FR-043` | 90 | T, M | Step 90: an untitled document asks to be saved first; a cancelled dialog pastes nothing (window test) |
| JRK-05 | Markdown: inline image rendering, scaling, cache, placeholders | supported | `JARAK-FR-044` | 91 | T, M | Step 91: PNG/JPEG/GIF/BMP/WebP decoded on a worker into a 128 MB cache, at pixel size within the column, placeholders with the path, remote never fetched; 50 × 1080p measured (window tests, media tests, e2e step91, real desktop 100 %/150 % UI scale); thumbnails in the source are not offered |
| JRK-06 | Markdown: asset navigation, orphans, rename with reference update | supported | `JARAK-FR-045` | 91 | T | Step 91: Ctrl+click reveals a picture in Explorer (preview and source), `jarak assets list` (orphans), `jarak assets rename` (never replaces, rewrites references), Save As offers to copy (window tests, CLI tests, e2e step91, real desktop); GUI entries for list/rename with the palette (Step 92) |
| JRK-07 | Vim: editing mode with a visible mode indicator | supported | `JARAK-FR-050` | 92 | T, M | Step 92: Editing mode Standard/Vim (palette, menu, remembered in settings.toml); the mode and pending keys lead the status bar; click returns to Standard (window tests, real desktop) |
| JRK-08 | Vim: motions, operators, text objects, counts, repeat, registers | supported | `JARAK-FR-051` | 92 | T | Step 92: motions, operators d c y > < g~ gu gU, text objects, counts, `.`, registers incl. `+` (18 engine tests, window session, real desktop); macros and marks not yet |
| JRK-09 | Vim: visual, visual-line, visual-block | supported | `JARAK-FR-052` | 92 | T | Step 92: v, V, Ctrl+V; a block is one range per line by display columns, I/A/c type on every line (engine tests, real desktop V-LINE); full column mode is Step 96 |
| JRK-10 | Vim: : command line | supported | `JARAK-FR-053` | 92 | T | Step 92: :w :q :q! :wq :x :e :e! :s with ranges and flags, :noh, :set, :N, / ? (engine tests, window test, e2e step92 writes with :w, real desktop :wq) |
| JRK-11 | Command palette listing every command with its keys | supported | `JARAK-FR-054` | 92 | T, M | Step 92: Ctrl+Shift+P lists every window action and registry command with its keys (labels checked against the bindings), filter in English/Korean/id/keys (window test, e2e, UI audit) |
| JRK-12 | Snippets with tab stops | supported | `JARAK-FR-095` | 100 | T | Step 100: `snippets.json` (per language and `*`), `$n`, `${n:placeholder}`, mirrors, `$0`, Tab/Shift+Tab, built-ins (`assist::tests::snippets_expand_with_tab_stops_mirrors_and_indentation`, `a_snippet_expands_from_the_list_and_tab_walks_its_stops`) |
| JRK-13 | Quick open of files (Ctrl+P) and navigation history | supported | `JARAK-FR-103` | 101 | T | Step 101: Ctrl+P (open documents, the workspace's files on a worker, letters in order) and Alt+Left / Alt+Right (`quick_open_finds_open_documents_and_the_workspaces_files`, `the_document_map_scrolls_and_the_navigation_history_goes_back_and_forward`) |
| JRK-14 | Headless CLI with --json (info, find, replace --dry-run, convert, export) | supported | `JARAK-FR-150` | 107 | T | Step 107: every command with `--json`, documented exit codes and a golden example (`every_command_has_a_golden_json_example`): `info`, `find`, `replace --dry-run`, `new`, `convert`, `export`, `print`, `session list`, `assets add`, `debug-info`, … ([JARAK_CLI](JARAK_CLI.md)); e2e `step107_jarak_cli_interop` |
| JRK-15 | Kim interoperability (Subak, Noeul, Ddalgak) | partial | `JARAK-FR-151` | 107 | T, M | Step 107: `jarak interop invoke` provides `document.info`, `text.find`, `document.export`, `markdown.insert-image`, `document.open` — Kim installs Jarak, finds and invokes them with its consent rules in the e2e (a screenshot PNG put into a note); Ddalgak's `tool` step calls them through Kim; `jarak open file:line` hands files to the running window; partial: opening into a running window from a Subak terminal and a real Noeul capture are not yet shown on the desktop ([JARAK_CLI](JARAK_CLI.md), [INTEROP](INTEROP.md)) |
| JRK-16 | Recovery of untitled documents after a crash | supported | `JARAK-FR-121` | 104 | T, M | Step 104: untitled and file documents offered at the next start (Restore / Not now / Discard twice) (`unsaved_documents_are_kept_and_offered_back_after_a_crash`, `recovery_kill.rs` with a killed child process, e2e `step104_jarak_recovery`); real desktop: a killed test instance's untitled text restored |

## 3. Counts

After Step 111 (the per-area counts are in [JARAK_PARITY_REPORT](JARAK_PARITY_REPORT.md)):

| | Notepad++ built-in features | Jarak additions |
| --- | --- | --- |
| Rows | 285 | 16 |
| … `supported` | 253 | 15 |
| … `partial` | 21 | 1 |
| … `planned` | 0 | 0 |
| … `not intended` | 11 | 0 |

## 4. Not intended, and why

- **Notepad++ plugin DLLs and Plugins Admin** (`PLUG-01`, `PLUG-02`): plugins are native DLLs
  bound to Scintilla's and Notepad++'s window-message APIs; Jarak is not built on Scintilla and
  does not run foreign native code inside its process. Plugin downloads would also contradict
  `KIM-SEC-001`. The common plugin functions are covered by built-in rows or by Jarak's own
  extensions (Step 108 audit).
- **Built-in updater** (`HELP-09`): Kim installs and updates tools only when the user asks
  (`KIM-SEC-001`, `KIM-SEC-002`).
- **Project panels and Find in Projects** (`VIEWS-13`, `FIF-08`): Folder as Workspace and Find in
  Files cover the use; `.workspace` project trees are not planned for v1 (revisited in Step 110:
  still not intended — the workspace folder, its document list and Find in Files cover them).
- **Cloud settings path** (`SET-19`): no cloud features (`JARAK-SEC-003`); the settings folder is
  a plain folder the user may sync with tools of their choice.
- **Ghost typing** (`HELP-08`): typing a file's content into the editor as a demo is not an
  editing feature.
- **Toolbar** (`UI-09`): a deliberate design decision ([JARAK_PHILOSOPHY §1](JARAK_PHILOSOPHY.md));
  every toolbar command is in the palette, the menus and the keys.
- **Find dialog transparency** (`SRCH-17`, Step 110): the find panel is docked under the text,
  never a floating dialog over it.
- **Right-to-left text direction** (`VIEW-16`, Step 110): egui has no bidirectional text layout;
  right-to-left scripts are shown in logical order. A right-to-left editor needs a bidi layout
  for carets, selections and columns — not offered in v1, recorded in
  [JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md).
- **Importing Notepad++ style themes** (`SET-24`, Step 110): the colours are the Kim design
  system's, audited for contrast and shared by every Kim tool.
