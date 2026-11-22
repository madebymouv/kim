# Jarak — Notepad++ parity report

Status: **current** (real-desktop acceptance run, 2026-11-23) · Source: [JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)
(this report is counted from it) · Reference: Notepad++ 8.9.x built-in features · Requirements:
[JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md) · Owner: mouv

Step 110 walked every row of the parity matrix. Each row now has a final status with its own
evidence: a named test, a command, an e2e scenario or a real-window check — never "a similar
feature exists". `partial` rows name exactly what is missing; `not intended` rows give the
reason. Step 111 (packaging) settled the last two: `HELP-11` supported, `SET-09` partial. The
real-desktop acceptance run of 2026-11-23 ([JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md)) checked the
thirteen rows that waited for the real desktop and moved them to supported; it also found and
fixed three faults: the Open dialog took one file only, the preview refused the palette, and
copied RTF pasted with the receiving program's paragraph spacing.

## Summary

- **285 Notepad++ built-in features**: 265 supported, 9 partial,
  0 planned, 11 not intended.
- Of the 274 features Jarak means to have, **96.7 %** are supported and
  **100.0 %** are there at least in part.
- **16 Jarak additions** (Markdown preview, pasted images, Vim, Kim interop, …):
  16 supported, 0 partial.
- Notepad++ plugin DLLs are **not** loaded and Jarak claims no binary compatibility with them
  (`PLUG-01`, [JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)); Jarak's own extensions cover the
  common plugin functions.

## Counts per area

| Area | Rows | Supported | Partial | Planned | Not intended |
| --- | --- | --- | --- | --- | --- |
| **FILE** File menu and opening files | 25 | 25 | 0 | 0 | 0 |
| **EDIT** Edit menu: clipboard, selection and basic editing | 18 | 17 | 1 | 0 | 0 |
| **CASE** Edit → Convert Case to | 6 | 6 | 0 | 0 | 0 |
| **LINE** Edit → Line Operations | 19 | 19 | 0 | 0 | 0 |
| **CMT** Edit → Comment/Uncomment | 3 | 3 | 0 | 0 | 0 |
| **BLANK** Edit → Blank Operations and EOL | 8 | 8 | 0 | 0 | 0 |
| **MULTI** Multi-editing, column mode and the Column Editor | 10 | 10 | 0 | 0 | 0 |
| **AUTO** Auto-completion and editing assistance | 10 | 10 | 0 | 0 | 0 |
| **SRCH** Search menu: find and replace in the document | 20 | 19 | 0 | 0 | 1 |
| **MARK** Search → Mark, style tokens and bookmarks | 15 | 15 | 0 | 0 | 0 |
| **FIF** Find in Files and the search results window | 8 | 7 | 0 | 0 | 1 |
| **VIEW** View menu: window, symbols, zoom and wrap | 17 | 16 | 0 | 0 | 1 |
| **VIEWS** View → documents, views and panels | 13 | 12 | 0 | 0 | 1 |
| **FOLD** View → folding | 6 | 6 | 0 | 0 | 0 |
| **ENC** Encoding menu | 16 | 16 | 0 | 0 | 0 |
| **LANG** Language menu and syntax highlighting | 11 | 10 | 1 | 0 | 0 |
| **SET** Settings menu | 26 | 17 | 7 | 0 | 2 |
| **TOOL** Tools menu | 4 | 4 | 0 | 0 | 0 |
| **MAC** Macro menu | 7 | 7 | 0 | 0 | 0 |
| **RUN** Run menu | 5 | 5 | 0 | 0 | 0 |
| **PLUG** Plugins menu and bundled plugin functions | 9 | 7 | 0 | 0 | 2 |
| **WIN** Window menu | 2 | 2 | 0 | 0 | 0 |
| **HELP** ? menu and command line | 11 | 9 | 0 | 0 | 2 |
| **REL** Sessions, backup and file status | 6 | 6 | 0 | 0 | 0 |
| **UI** General user interface | 10 | 9 | 0 | 0 | 1 |
| **All Notepad++ built-in features** | **285** | **265** | **9** | **0** | **11** |
| **JRK** Jarak additions (not Notepad++ features) | 16 | 16 | 0 | 0 | 0 |

## What `partial` means here, row by row

Every remaining partial row names a part Notepad++ has and Jarak does not have yet; none waits
for a real-desktop check any more.

- `EDIT-17` Clipboard history panel (Step 110): Step 110: Edit → Paste from clipboard history… lists what Jarak copied, cut or pasted (newest first, each once, 20, in memory only) and pastes a pick (`the_clipboard_history_pastes_an_earlier_copy`; audited); partial by design: copies made in other programs are not listed — Jarak does not watch the system clipboard (`JARAK-SEC-003`)
- `LANG-02` Further Notepad++ languages (Ada, ASM, AutoIt, COBOL, D, Fortran, Haskell, Lisp, Pascal, Perl, R, Scheme, Swift, VHDL, Verilog, …) (Step 110): Step 110: Ada, Assembly, AutoIt, COBOL, D, Fortran, Haskell, Lisp, Pascal, Perl, R, Scheme, Swift, VHDL and Verilog are built in, detected by extension (and Perl, R by `#!`) (`the_further_notepad_plus_plus_languages`, e2e `step110_jarak_parity`); partial: Notepad++'s other languages (for example Erlang, OCaml, Tcl, Nim, fixed-form COBOL's column-7 comments) are not built in — they can be added as user-defined languages (Step 98)
- `SET-02` Preferences: Tab bar options (close button, double-click to close, multi-line) (Step 105): Step 105: close buttons on every tab or only the active / hovered one, double-click closes a tab; multi-line tab bars are not implemented
- `SET-03` Preferences: Editing (caret, current line, line wrap, multi-editing, scrolling beyond last line) (Step 105): Step 105: current-line highlight, a blinking caret (off by default), word wrap, multi-editing (Ctrl+click), virtual space; scrolling beyond the last line is not implemented
- `SET-05` Preferences: Margins, border, edge (line numbers, bookmark margin, edge column) (Step 105): Step 105: line numbers on/off, edge lines at columns; the bookmark margin cannot be hidden
- `SET-13` Preferences: Searching (minimum size, fill find field, confirm replace all) (Step 105): Step 110: *Selected text fills the find field* (`find_fill_selection`, `new_language_open_folder_find_fill_and_backup_settings_apply`); partial: no minimum size for *in selection* and no Replace All confirmation setting (Replace All is one undo step; Replace in Files always previews, Step 95)
- `SET-17` Preferences: Delimiter (word characters, double-click delimiters) (Step 105): Step 110: further word characters for double-click, Ctrl+arrows and select word (`word_chars`; `further_word_characters_join_words`, `smart_highlighting_word_characters_and_the_length_summary`); partial: Notepad++'s Ctrl+double-click delimiter selection is not implemented
- `SET-21` Preferences: MISC (file status auto-detection, document peeker, smooth font, clickable links) (Step 105): Step 105: file status auto-detection (Step 104) in Settings → Files; document peeker, smooth font and clickable-link options are not implemented
- `SET-22` Style Configurator (themes, per-language token colours, global font) (Step 105): Step 105: themes and the editor font family and size (Windows' fonts when present); per-language token colours are not implemented

## Not intended, and why

- `SRCH-17` Find dialog transparency on losing focus (Step 110): Step 110: Jarak's find panel is docked in the window under the text, never a floating dialog over it, so there is nothing to make transparent
- `FIF-08` Find in Projects (project panels) (Step 110): —
- `VIEW-16` Text direction right-to-left / left-to-right (Step 110): Step 110: egui lays text out left to right without bidirectional reordering; right-to-left scripts are shown in logical order ([JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md)). A right-to-left mode would need a bidi layout under the whole editor (carets, selections, columns); not offered in v1
- `VIEWS-13` Project panels (1–3, .workspace files) (Step 110): —
- `SET-19` Preferences: Cloud settings path (Step 105): —
- `SET-24` Import style themes (Step 105): Step 110: Jarak's themes are the Kim design system's (`kim-design` tokens, contrast-audited, the same in every Kim tool); a Notepad++ style theme would bring colours the design audit cannot vouch for. User-defined languages keep their token kinds (Step 98)
- `PLUG-01` Load third-party Notepad++ plugin DLLs (Step 108): Step 108: Notepad++ plugin DLLs talk to Notepad++'s and Scintilla's window messages (`NPPM_*`, `SCI_*`), which Jarak does not have; they are never loaded ([JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md))
- `PLUG-02` Plugins Admin (download and update plugins from a list) (Step 108): Step 108: nothing is downloaded or updated (`KIM-SEC-001`); extensions are installed from a local folder by an explicit command ([JARAK_EXTENSIONS](JARAK_EXTENSIONS.md))
- `HELP-08` Command line: -qt / -qf ghost typing (Step 107): —
- `HELP-09` Built-in updater (WinGUp) (Step 111): —
- `UI-09` Toolbar with icons (Step 88): —

## How the evidence was gathered

- **Window tests** (`crates/jarak-gui/tests/window.rs`, egui_kittest, headless): the real
  `JarakApp` driven by keys, text, pointer and wheel events; its accessibility tree and document
  are checked. 155 tests after the 2026-11-23 run.
- **Core tests** (`crates/jarak-core`): the document model, encodings, search, syntax, Markdown,
  export, printing layout and settings (188 unit tests after Step 110), golden files and the
  security tests.
- **e2e scenarios** (`e2e/tests/step85…step110`): the release `jarak` and `jarak-gui` the way a
  user or an agent uses them; they run in the Daily workflow (AGENTS.md §4).
- **UI audit** (`cargo xtask ui-audit`, `kim_ui::inspect`): every screen's geometry checked
  against the design rules (contrast, sizes, overlap, clipping), Step 110's screens included.
- **Real desktop checks** recorded in [JARAK_ARCHITECTURE](JARAK_ARCHITECTURE.md) per Step and
  in [JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md), with real input under ADR 0088 when the maintainer
  was away: test-started windows and sandboxed profiles only.

## Real-desktop checks

All rows that waited for the real desktop were checked on 2026-11-23 with test-started windows:
the Open and Save As dialogs (`FILE-02`, `FILE-09`), a real OLE file drop (`FILE-03`), the print
dialog with Microsoft Print to PDF (`FILE-19`), the real clipboard's HTML and RTF with WordPad
and another program (`EDIT-13`, `PLUG-07`), always on top, full screen, post-it and
distraction-free (`VIEW-01`…`VIEW-04`), `--position` / `--always-on-top` (`HELP-07`), the shell's
*Open with* list after the offer and its exact removal (`SET-09`), and the Noeul and Subak
demonstrations through Kim (`JRK-15`). Not tested on this PC: a paper printer, a second monitor,
Windows 11. [JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md) has the run.
