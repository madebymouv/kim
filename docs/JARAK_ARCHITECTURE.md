# Jarak Architecture

Status: **current** (Step 85: crates, CLI foundation, window shell, single instance, Kim
integration; Step 86: the text model, §8; Step 87: files and encodings, §9; Step 88: the editor
window, §10; Step 89: Markdown, §11; Step 90: Ctrl+V images, §12; Step 91: pictures and assets, §13; Step 92: Vim, the palette and the menu bar, §14; Step 93: tabs, views and the session, §15) · Owner: mouv · Decision:
[ADR 0102](adr/0102-jarak-fourth-kim-tool.md) · Requirements:
[JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)

## 1. Crates and programs

| Crate | Program | Contents now | Never links |
| --- | --- | --- | --- |
| `jarak-core` | — (library) | `paths` (per-user folders), `target` (`file[:line[:column]]` open targets), the text model (`text`, `selection`, `history`, `document`, `command`; §8) | a GUI toolkit, windowing, the clipboard (`cargo xtask deps-check`) |
| `jarak` | `jarak.exe` | The headless CLI: `version`, `capabilities`, `paths`, `open`, `info` | a GUI toolkit (`deps-check`) |
| `jarak-gui` | `jarak-gui.exe` | The editor window on `kim-ui` (egui + glow): the text view, keys, IME, clipboard, open/save/close with the Windows dialogs and the unsaved-changes question, the status bar (§10); the single-instance hand-off | — |

All three ship under one version (`cargo xtask bump` group `jarak`). The later Steps add their
modules to `jarak-core` (text model, files, search, Markdown render model) and their views to
`jarak-gui`; the GUI turns input into commands and draws state, it never edits the document
directly (ADR 0102 §2).

## 2. The CLI (`jarak.exe`)

Follows [CLI_CONTRACT](CLI_CONTRACT.md): human output or one `--json` envelope, exit codes
0/1/2/3, `--quiet`/`--verbose`, `--dry-run` on the one command that changes something.

| Command | Effect | What it does |
| --- | --- | --- |
| `jarak capabilities [--json]` | read | The capabilities document (`crates/jarak/src/capabilities.json`, also the package manifest and `docs/contracts/examples/tool-capabilities-v1/jarak.json`) |
| `jarak version [--json]` | read | Version, CLI contract 1, JSON schema 1 |
| `jarak paths [--json]` | read | The per-user files below; creates nothing |
| `jarak info <file> [--fallback <encoding>] [--json]` | read | Encoding, BOM, line endings (with counts), size, lines, characters, binary, lossless and read-only — what the window would open (§9); `file_not_found` (exit 3), `file_too_large` (exit 5), a folder is `usage` |
| `jarak find <file\|folder> <text> [--extended\|--regex] [--match-case] [--whole-word] [--dot-all] [--count] [--filters <filters>] [--no-subfolders] [--hidden] [--fallback <encoding>] [--json]` | read | Step 94: the matches with line, column (characters, from 1), length and the line's text, at most 10 000 listed (`truncated`); `--count` gives only the count; the same search as the window's find panel (§16); a pattern that does not compile is `usage`. Step 95: a folder is Find in Files (§17): each file's matches, `files_searched`, `skipped` (binary, too large, unreadable) |
| `jarak replace <file\|folder> <text> <with> [search options] [--filters …] [--no-subfolders] [--hidden] [--dry-run] [--yes] [--json]` | destructive | Step 95: Replace in Files (§17). `--dry-run` lists every change (old and new line) and writes nothing; without it `--yes` is needed (`confirmation_required`, exit 7; the CLI never asks); the batch id and journal folder are returned |
| `jarak journal list [--json]` | read | Step 95: the Replace in Files batches kept for undo, newest first |
| `jarak journal undo <batch> [--dry-run] [--yes] [--json]` | destructive | Step 95: the batch's files get their original bytes back unless changed since (those are listed and left); `batch_not_found` (exit 3) |
| `jarak open [<file>[:<line>[:<column>]]...] [--new-window] [--dry-run] [--json]` | write, opens a window | Starts `jarak-gui.exe` next to `jarak.exe` (detached, no console) with absolute targets, or a running window takes them; `not_installed` (exit 3) when the window program is missing; a folder is a `usage` error |

`file:line:column` (`jarak-core::target`): an argument that names an existing file is that
file; otherwise up to two trailing `:<number>` parts are the line and column. A drive letter
alone (`C:12`) is not a file with a position; line 0 means no line.

## 3. The window (`jarak-gui.exe`)

- **Shell (Step 85):** the design system's dark theme by default (`kim_ui::apply` on the first
  frame), the document area with a line-number gutter and the caret in the accent colour, a
  28-point status bar (line:column, encoding, line ending, language) with the Settings gear at
  its right end, no toolbar. The empty document says plainly that the window does not edit yet
  (Steps 86–88); the gear's note says Settings arrive with Step 105.
- **Size:** 960 × 640 points times the shared UI zoom (`kim_design::scale::startup_zoom`; 1.5 on
  the maintainer's 5120 × 1410 screen), at least 360 × 240.
- **Single instance (`JARAK-FR-002`):** a named mutex `Local\mouv-jarak-gui` marks the owner. A
  second start writes its targets to `%LOCALAPPDATA%\mouv\jarak\handoff\<time>-<pid>.json`
  (written under a temporary name, then renamed), sets the event `Local\mouv-jarak-gui-open`,
  lets the owner take the foreground and exits. A thread of the owner sleeps in the wait (no
  polling), reads and deletes every hand-off in order and brings the window to the front. Stale
  hand-offs are removed without opening anything when an owner starts. `--new-window` skips
  the claim. `--instance <tag>` gives test-started windows their own names and folder (never the
  user's window).

## 4. Files

| Path | Roams | Contents |
| --- | --- | --- |
| `%APPDATA%\mouv\jarak\settings.toml` | yes | Settings (Step 105) |
| `%APPDATA%\mouv\jarak\hotkeys.json` | yes | Key bindings (Step 105) |
| `%LOCALAPPDATA%\mouv\jarak\session.json` | no | The session (Step 93) |
| `%LOCALAPPDATA%\mouv\jarak\recovery\` | no | Recovery snapshots (Step 104, `JARAK-SEC-008`) |
| `%LOCALAPPDATA%\mouv\jarak\handoff\` | no | Hand-offs from a second start (consumed at once) |

## 5. Kim integration

- **Catalog:** `jarak` in `catalog/catalog.json` (catalog v11): `support: in-development`, no
  release, `runs: window`, planned entry points, the clipboard permission (read only on paste).
- **Package:** `cargo xtask package jarak` (and `local-mirror`) builds a zip with `jarak.exe`,
  `jarak-gui.exe`, the capabilities manifest, licenses and third-party notices; entry point
  `jarak.exe`.
- **Kim Manager:** a fourth tile (Jarak's icon, `assets/jarak/`) that starts `jarak-gui.exe`; the
  launcher is one tile wider (464 × 224 points at 100 %, minimum 432 × 196; was 368 × 224 with
  three tiles, ADR 0090). Its menu's *Settings…* says Settings arrive with Step 105.
- **Icons:** `assets/jarak/jarak.ico` linked into both programs by `kim-winres`.

## 6. Measurements (Step 85, release build, this PC: 5120 × 1410, Windows 10)

| What | Result | Budget |
| --- | --- | --- |
| `jarak.exe` / `jarak-gui.exe` size | 0.5 MB / 6.0 MB | ≤ 8 MB / ≤ 15 MB (`JARAK-NFR-006`) |
| Start to the window handle (three runs) | 103, 61, 62 ms | first frame ≤ 400 ms (`JARAK-NFR-001`; the text view is measured again in Step 88) |
| Idle CPU over 10 s, empty window | 0 ms | ≈ 0 (`JARAK-NFR-003`) |
| Memory at UI zoom 1 (Kim Manager for comparison) | Jarak 114.5 MB private / 87.9 MB working set; Kim Manager 110.1 / 89.6 MB | see below |
| Memory at the auto zoom 1.5 | 117.4 MB private / 87.9 MB working set | |

The memory is the shared egui + glow baseline of every Kim window on this PC (Kim Manager, which
draws much less, uses the same), mostly the OpenGL driver and the font atlas. `JARAK-NFR-003` was
corrected from an unmeasured 80 MB to the measured baseline (working set ≤ 100 MB with an empty
document); Step 109 tries to lower it for every Kim window.

## 7. Verification (Step 85)

- Unit tests: `jarak-core` (paths, open targets), `jarak` (capabilities, usage errors),
  `jarak-gui` (hand-off order, stale files) and `tests/window.rs` (status bar and gear at the
  right end, no toolbar, the gear's note, Korean labels, files from the command line and from a
  second start).
- Real desktop (sandboxed `APPDATA`/`LOCALAPPDATA`, `--instance` names, nothing of the
  maintainer's touched): the window's screenshot and geometry report (`KIM_UI_INSPECT`); a second
  start handed `todo.txt:4` to the first window (its title became "todo.txt — Jarak", the second
  process exited with 0, no hand-off file left); `--new-window` opened a separate window; both
  closed cleanly.
- e2e `step85_jarak_foundation.rs` (Daily): Kim installs Jarak from a test-signed package, the
  CLI's contract, `paths` in the sandbox, `open --dry-run`, usage errors, uninstall.

## 8. The text model (Step 86, `jarak-core`)

- **Rope** (`ropey` 1.6, MIT): logarithmic inserts, deletes and line lookups. Only LF, CR and
  CRLF break lines (ropey's `cr_lines` without `unicode_lines`), as in Notepad++.
- **Positions** are char indices. `text` moves by grapheme cluster (`unicode-segmentation`,
  across rope chunks): a jamo-written Hangul syllable, a letter with combining marks, an emoji
  ZWJ sequence, a flag and CRLF are each one step for the caret, Backspace and Delete. A
  position set from outside (a click, a search hit) snaps to its cluster's start. Words for
  Ctrl+arrows and double-click are runs of letters/digits/`_` (any script), punctuation or
  spaces; a line break is a stop of its own. Display columns count wide characters as two
  (`unicode-width`) and tabs to the next stop; vertical moves keep a goal column.
- **Selections**: a range has an anchor and a head (a caret when equal); a selection holds one or
  more ranges, sorted, merged when they overlap, with one primary range (ready for Step 96's
  multiple carets).
- **Transactions**: an edit replaces text at a position; a transaction is a list of edits applied
  in order (several carets edit from the last to the first so earlier positions stay valid),
  recorded with the selections before and after. Undo applies the inverses in reverse order and
  restores the selection before; redo re-applies and restores the selection after.
- **Grouping**: typed characters merge into one undo step while typing continues in place, with a
  break after whitespace (each word is a step); Backspace/Delete runs merge likewise; a caret
  move, a save or anything else (paste, Enter, word delete) starts a new step.
- **Save point**: each undo entry has a state id; the document is dirty when the newest entry's
  id differs from the id at the last save (undo back to the saved text is clean again; a merged
  entry gets a new id).
- **Enter** inserts the document's line ending (detected from the text, CRLF when there is
  none) plus the indentation before the caret.
- **Commands** (`command`): 32 registered commands with stable ids (`edit.undo`,
  `cursor.word-left`, `select.page-down`, …) and English/Korean titles; `Document::execute` runs
  them. Typing text is `type_text` (one grapheme is typing, longer text is a paste-like step).

| Measurement (release, 100 MB document of mixed ASCII/Hangul lines, `examples/text_bench.rs`, `docs/perf/jarak-text-model.json`) | Result | Budget |
| --- | --- | --- |
| Load from memory | 62 ms (1 497 967 lines) | — (file loading is Step 87) |
| Edit at a random place (set caret + type/Backspace/Enter), 4 000 edits | p50 2.7 µs, p95 3.9 µs, p99 7.4 µs, max 79.3 µs | ≤ 1 ms (`JARAK-NFR-002`) |
| Undo, per step | 0.7 µs | — |
| Line lookup + line/column | p50 1.0 µs, max 43.1 µs | — |

Tests (26): grapheme steps both ways for Hangul (precomposed and jamo), combining marks, ZWJ
emoji, flags and CRLF, also across rope chunks; word steps and double-click words with Hangul;
display columns with wide characters and tabs; line breaks; selections (sorting, merging,
primary); typing runs, word-wise undo groups, Backspace runs, word deletes; Enter with
indentation and line endings; three carets typing, deleting and undoing together; moves by
grapheme, word, line (goal column across wide characters), smart Home, document; select word,
line, all and the copied text; the save point through undo/redo; the command registry; and a
seeded random test (40 seeds × 300 operations: typing, pasting, deleting, Enter, selections,
undo, redo) checking that every caret stays on a grapheme boundary, that a history state id
always means the same text, and that undo-all/redo-all return the original and final text; and
a second random test (40 seeds × 400 edits) that keeps an independent `String` model in step with
the rope after every typed, pasted or deleted text.

## 9. Files, encodings and saving (Step 87, `jarak-core::file`)

- **Detection**, in order: a byte-order mark (UTF-8, UTF-16 LE, UTF-16 BE); valid UTF-8; UTF-16
  without a BOM (at least 70 % zero bytes on one side, almost none on the other); otherwise the
  legacy encoding. The legacy encoding is the system ANSI code page (`GetACP`) — or, when
  Windows' "Use Unicode UTF-8 for worldwide language support" makes that UTF-8 (as on the
  maintainer's PC), the ANSI code page of the user's locale (`LOCALE_IDEFAULTANSICODEPAGE`, 949
  for Korean), which is what older files were written in. `jarak info --fallback <label>` names
  another (`euc-kr`, `cp949`, `windows-1252`, `shift_jis`, …).
- **Lossless or read-only** (`JARAK-SEC-001`): after decoding, the text is encoded again with the
  same encoding and BOM; unless that reproduces the file byte for byte (an undefined byte in the
  code page, an unpaired UTF-16 surrogate, an odd byte count after a UTF-16 BOM, invalid UTF-8
  read as UTF-8), the document is read-only and says why. Zero bytes outside UTF-16 mark a binary
  file (read-only too). Opening and saving a lossless file without an edit writes the same bytes.
- **Line endings** stay exactly as they are in the text (nothing is normalised; mixed files stay
  mixed); `EolCounts` reports CRLF/LF/CR counts and the dominant kind, which Enter then inserts.
- **Encoding a save** refuses a character the encoding cannot hold (EUC-KR and an emoji) with its
  position instead of writing a substitute; nothing is written.
- **Atomic save** (`JARAK-SEC-002`): the bytes go to `.<name>.jarak-<pid>.tmp` in the same folder,
  are flushed (`sync_all`), and replace the original with `ReplaceFileW` (keeping its attributes,
  e.g. hidden) or a rename for a new file. A read-only original is refused. On any failure —
  checked by injecting one at create, write, flush and replace — the original keeps its bytes and
  the temporary file is removed.
- **`FileDocument`**: path (or untitled), format, read-only flag and reason, and the document;
  `open` (a missing file is a new document created on the first save), `save`, `save_as`,
  `save_copy` (the document keeps its file and dirty state), `reload`, `title`. Files above 1 GiB
  are refused (`file_too_large`); Step 109 measures what the editor can serve.
- **Not yet**: the window's Open/Save dialogs, the unsaved-changes prompt on close and the status
  bar's real values arrive with the text view (Step 88); reinterpreting and converting between
  encodings and the full character-set list are Step 103.

Tests (13 in `file`): the corpus round trip (empty, UTF-8, Hangul, UTF-8 BOM, UTF-16 LE/BE with a
BOM, UTF-16 LE without one, EUC-KR, windows-1252, mixed line endings without a final newline,
trailing whitespace) byte for byte; detection and labels; lossy and binary files read-only;
unmappable characters refused; line-ending statistics; saves creating and replacing with nothing
left behind; a failure injected at each of the four stages leaving the original untouched; a
read-only file refused and a hidden file still hidden after a save; folders, missing and
oversized files refused; `FileDocument` open/save/edit in EUC-KR/save-as/copy/reload/new file/
untitled; lossy files refusing save and save-as. e2e `step87_jarak_files.rs` runs `jarak info` on
such a corpus (first focused run PASS).

## 10. The editor window (Step 88, `jarak-gui`)

- **Text view** (`view.rs`): Jarak draws the document itself on egui's painter — one galley per
  visible line (`top_line` plus what fits), so cost per frame does not grow with the file. Tabs
  are expanded to the next stop (wide characters count two columns) with a map between document
  offsets and drawn characters, so the caret, selections and clicks stay exact. The view scrolls
  by line plus a pixel offset into the top line (word wrap and huge files need no total height);
  a thin scrollbar maps to the line index and can be dragged or clicked. Gutter with line numbers
  (the caret's line brighter), current-line highlight, selections (with an end-of-line mark when a
  line break is selected, across wrapped rows), the caret in the accent colour (a box in
  overwrite mode), steady (no blink: no repaint while idle). Word wrap (Alt+Z), zoom (Ctrl+=,
  Ctrl+−, Ctrl+0, Ctrl+wheel), a read-only document has a warning-coloured frame.
- **Mouse**: click places the caret (on a grapheme start), Shift+click extends, drag selects (by
  word after a double click, by line after a triple click), dragging past the edge scrolls, the
  wheel scrolls (Shift+wheel sideways), files dropped on the window open.
- **Keys** (`keys.rs`): the standard Windows editing keys — arrows, Ctrl+arrows, Home (smart) /
  End, Ctrl+Home/End, Page Up/Down, Shift to select, Backspace/Delete (Ctrl for words), Enter
  (keeps indentation), Tab / Shift+Tab (indent or unindent the selected lines), Insert
  (overwrite), Ctrl+A/Z/Y/Shift+Z, Ctrl+C/X/V and Ctrl+Insert/Shift+Delete/Shift+Insert,
  Ctrl+N/O/S/Shift+S/W/F4, Ctrl+Tab between open documents. One document is shown at a time; tabs
  are Step 93.
- **IME** (`JARAK-FR-033`): egui's composition events — the composition is drawn at the caret,
  underlined in the accent colour, and only the committed text enters the document; the IME
  window is told where the caret is (`IMEOutput`).
- **Clipboard** (`clipboard.rs`): eframe's clipboard shortcuts are off; Jarak uses the Windows
  clipboard itself (`CF_UNICODETEXT`, retrying while another program holds it), only on Copy,
  Cut or Paste. Copy without a selection takes the current line (as Notepad++ does by default);
  pasted line endings become the document's.
- **Files**: Ctrl+O and Save As use the Windows dialogs (`GetOpenFileNameW` /
  `GetSaveFileNameW`); Ctrl+S on an untitled document asks for a name; a save shows "Saved …" or
  why not in the status bar; a read-only document refuses edits and says why.
- **Unsaved changes** (`JARAK-FR-024`): Ctrl+W and closing the window ask "Save changes to …?"
  with Save (default, Enter), Don't save and Cancel (Esc); closing the window asks for each
  unsaved document in turn and closes only when all are answered; a failed or cancelled save
  closes nothing. A UI audit run (`KIM_UI_INSPECT`) closes without asking (it holds no user data).
- **Status bar**: Ln/Col (display columns, 1-based) and the selected count, encoding label,
  line ending (or "Mixed EOL"), language (Markdown by extension; detection is Step 98),
  INS/OVR, zoom, Wrap, Read-only, the document number when several are open, a message on the
  left, the gear (`size::GEAR`) at the right end.
- **UI audit**: `cargo xtask ui-audit --only jarak` — editor, empty, selection, wrap and the
  question at 100/125/150/200 % dark English and 150 % light Korean: 25 runs, 0 layout failures
  (the first run found the gear at 24 instead of `size::GEAR` 28 and an audit close blocked by
  the question; both fixed).

| Measurement (release, this PC) | Result | Budget |
| --- | --- | --- |
| Titled window after start: empty / 1 MB / 10 MB / 100 MB | 247 / 224 / 307 / 577 ms | ≤ 400 ms empty, ≤ 600 ms 1 MB (`JARAK-NFR-001`), ≤ 3 s 100 MB (`JARAK-NFR-004`) |
| Idle CPU over 10 s with each of those open | 0 ms | ≈ 0 (`JARAK-NFR-003`) |
| Working set: empty / 10 MB / 100 MB | 91.0 / 107.1 / 211.6 MB (private 113.8 / 133.7 / 238.7) | ≤ 100 MB empty; 100 MB file ≤ 3× (2.4×) |
| Keystroke frame in a 10 MB document (headless CPU layout, `tests/perf.rs`) | p50 0.11 ms, p95 0.13 ms, max 1.65 ms | ≤ 16 ms (`JARAK-NFR-002`) |
| Close right after the window appears | ends at once | (Step 85 observation; not reproduced) |

The first keystroke measurement was p95 12.1 ms: the status bar recounted the line endings of
the whole document every frame. It is now counted once per history state — in a document above
two million characters only after half a second without edits (one scheduled repaint).

**Real desktop (test-started window, sandboxed profile, every key only while the test window
had the foreground, the maintainer idle, the clipboard restored):** typing English; Enter;
Korean composed by the real IME (Hangul key, `g k s r m f`, Space → `한글`); Ctrl+S creating the
file with CRLF; Ctrl+Z back to the saved state (title without `*`); Ctrl+A Ctrl+C (the clipboard
held the text); Ctrl+V of LF text inserted with CRLF; Ctrl+Home Shift+End Delete and undo;
closing asked (Esc kept the window, Enter saved and closed, exit 0); no temporary file left. A
click past the end of a line, a double click replacing a word, a drag from the gutter selecting
three lines, three wheel notches scrolling eight lines.

**Known limits:** egui's fonts draw a ZWJ emoji family as separate glyphs and a flag as letters
(the caret still treats each as one); the wheel scrolls about 2.7 lines per notch (egui's points
per notch, not Windows' "lines to scroll" setting); dragging selected text inside the document,
the text context menu and clickable status-bar items are later Steps (110, 110, 103).

## 11. Markdown and the preview (Step 89)

- **Render model** (`jarak-core::markdown`, `pulldown-cmark` 0.13, MIT): GitHub-flavoured Markdown
  — headings, paragraphs, emphasis, strong, strikethrough, inline code, links, images, block
  quotes, bullet/ordered/task lists (nested), fenced and indented code blocks, tables with column
  alignment, rules, footnotes; GFM's extended autolinks (bare `http(s)://` and `www.` URLs, with
  GFM's trailing-punctuation rule) are added by Jarak. Raw HTML is shown as text, never
  interpreted. Every top-level block keeps its source lines (trailing blank lines excluded). The
  model is a pure function of the text and never touches the document (`JARAK-SEC-001`).
- **Preview** (`jarak-gui::preview`): drawn natively on egui's painter in the UI font (code in the
  monospace font), a reading column of at most 820 points, centred: headings with a rule under
  levels 1–2, task checkboxes, list markers, the quote bar in the accent colour, code blocks and
  table headers on `bg.raised`, banded table rows, footnotes. Like the text view it is
  virtualized: it scrolls by block (top block plus a pixel offset) and lays out only the blocks on
  screen. The parse is cached per history state.
- **Modes** per document: Source, Preview (Ctrl+E; Esc returns), Split (Ctrl+Shift+E: source left,
  preview right, the preview following the source's top line). Switching maps the source's top
  line to the block that shows it and back; the caret never moves; the document stays
  byte-identical and clean. In the preview typing edits nothing and says how to get back; the
  arrows, Page keys and Space scroll; Ctrl+S, Ctrl+W, zoom still work. The status bar shows
  "Preview" / "Split" in the accent colour.
- **Links** (`JARAK-FR-046`, `JARAK-SEC-006`): the pointer shows a hand and the address; a plain
  click only says "Ctrl+click to open"; Ctrl+click opens web and mail links (`http`, `https`,
  `mailto`) with the Windows shell (`ShellExecuteW`) — `file:`, `javascript:` and relative paths
  are refused. Images are labelled placeholders until Step 91.

| Measurement (release) | Result | Budget |
| --- | --- | --- |
| First switch of a 1 MB Markdown document to the preview (parse + visible layout, headless) | 38.4 ms | ≤ 100 ms (`JARAK-NFR-005`) |
| Later switches (cached parse) | p50 0.11 ms, max 1.54 ms | — |

Tests: 7 in `markdown` (inlines, task and nested lists, quotes, code, tables with alignment,
rules, HTML, links, images, autolinks with GFM punctuation, footnotes, source lines, CRLF, a
1 MB parse); `preview` (only web and mail links open; the parse cached per state; line ↔ block
mapping both ways); window flows (Ctrl+E keeps the caret, the text and the clean state and maps
the position both ways, typing in the preview edits nothing, Esc returns; the split view edits
the source and re-parses; links open only through the opener and only for web/mail); UI audit
`preview` and `split` (35 runs with the editor views, 0 failures); e2e
`step89_jarak_markdown_preview.rs` (first focused run PASS). Real desktop: Ctrl+E, typing in the
preview, Down/PageDown, Esc, Ctrl+Shift+E twice on the fixture — the title never showed `*`, the
file stayed byte-identical, the window closed with exit 0 (screenshot reviewed).

## 12. Ctrl+V images and portable assets (Step 90, the MVP)

- **What Ctrl+V does** (`JarakApp::paste_from_clipboard`): text when the clipboard has text (also
  when it has an image too, as office programs put both); otherwise an image, but only into a
  Markdown document (`.md`, `.markdown`, …; elsewhere the status bar says so). An untitled
  document asks to be saved first (Save As; cancelling pastes nothing). A read-only document
  refuses.
- **Clipboard formats** (`clipboard.rs`, `images.rs`): the registered `PNG` format (checked and
  kept as it is), `CF_DIBV5` and `CF_DIB` (24/32-bit, bottom-up or top-down, `BI_RGB` or
  `BI_BITFIELDS`, including Windows' synthesized `CF_DIBV5` that repeats the masks after the
  header; an all-zero fourth byte means opaque; converted to PNG), and image files copied in
  Explorer (`CF_HDROP`, recognised by their first bytes: PNG, JPEG, GIF, BMP, WebP; copied as
  they are, one reference each). Limits: 128 MiB, 16 384 pixels on a side, 40 million pixels
  (`JARAK-SEC-004`).
- **Assets** (`jarak-core::assets`, `JARAK-SEC-005`): `assets/` next to the document; the name is
  the document's name made portable (letters of any script, digits, `-`, `_`), the local date and
  the first free number — `ideas-20261122-001.png`; files are created with create-new semantics
  (an existing file is never overwritten, also when two pastes race; a file that could not be
  filled is removed, nothing else ever is). The document gets an image reference to `assets/…` at the caret
  as one undo step; undo removes the reference and leaves the file (orphans are listed in Step
  91). References are relative with forward slashes, so a note and its `assets` folder move
  together.
- **Preview images** (`preview.rs`): a reference resolves relative to the document and never
  outside its folder (`..`, absolute and remote references are not loaded; remote images say
  "remote image not loaded"); the file is decoded once (pure-Rust `image` decoders: PNG, JPEG,
  GIF's first frame, BMP, with the same limits) into a texture cached per document and reloaded
  when the file changes; drawn at its pixel size, at most the column's width. A paragraph with
  text and images shows the text runs and the pictures in order; a missing or refused image
  shows a labelled frame. (Step 91 moved decoding off the UI thread, §13.)

Tests: `assets` 3 (portable names, the next free number never overwriting, references resolving
only inside the folder); `images` 7 (24-bit bottom-up and 32-bit top-down DIBs, alpha and the
all-zero rule, `BI_BITFIELDS`, the synthesized `CF_DIBV5`, malformed and huge bitmaps refused,
PNG round trip, signatures, copied files recognised by their bytes); window flows 3 new: the MVP
journey (paste a bitmap into a saved note → the asset and the reference, undo keeps the file,
save, preview shows it, a second paste gets `-002`, reopen in a new app shows both), an untitled
document saved first (and nothing pasted when the dialog is cancelled), images only into
Markdown and text winning when both are on the clipboard. UI audit `paste-image` (40 runs, 0
failures — its first screenshot showed a reference glued to the previous paragraph drawn as a
placeholder, which led to drawing images inside paragraphs); e2e `step90_jarak_image_paste.rs`
(paste, save, close, reopen in a new process; the asset is a 320 × 180 PNG and both previews
draw it; first focused run PASS).

**Real desktop (the MVP journey, test-started windows, the maintainer's clipboard text kept and
restored):** a new note typed and saved; a 360 × 180 picture put on the real clipboard (as a
screenshot tool does); Ctrl+V → `assets/journey-20261122-001.png` and the reference; Ctrl+S,
Ctrl+E → the picture in the preview; closed (exit 0); reopened in a new process → Ctrl+E shows
it. The first run showed a 3-pixel strip at the picture's left edge — Windows' synthesized
`CF_DIBV5` repeats the masks after the header — fixed and re-run (screenshot reviewed).

## 13. Pictures in the preview and the assets of a document (Step 91)

- **Decoding off the UI thread** (`media.rs`, `JARAK-FR-044`): the preview asks
  `Media::get` for a picture every frame and gets an answer at once — the texture, "loading"
  (with the picture's size as soon as a worker thread read its header, so the layout does not
  jump) or the reason it is not shown. One worker thread per document (started on the first
  picture, ended with the document) reads the header, decodes with the limits of §12 (PNG,
  JPEG, GIF's first frame, BMP and now WebP, all pure Rust) and scales a picture larger than
  4096 pixels on a side down to fit; the window is woken when a result is ready. A file is looked
  at again at most once a second and decoded again when it changed.
- **A bounded cache**: decoded textures are kept up to 128 MB (`media::BUDGET`); beyond that the
  ones not drawn in the last frame are dropped, least recently drawn first, and decoded again
  when they come back on screen. Pictures on screen are never dropped.
- **Size on screen**: one image pixel per screen pixel, times the document's zoom, at most the
  column's width, aspect kept — a screenshot shows at its true size on any display scale and
  stays sharp (at 150 % the text is 1.5 × larger, a 360 × 180 picture still covers 360 × 180
  screen pixels). Placeholders (loading, missing, broken, remote, outside the folder, unsaved
  document) are frames with a label in the user's language and the reference.
- **No network**: a remote picture (`http(s)://`, `data:`) is never fetched; its frame says so.
  Jarak has no network code; an opt-in to load remote pictures is not offered (a decision of this
  Step: local-first, `JARAK-SEC-005`).
- **Navigation** (`JARAK-FR-045`): hovering a local picture shows its file and size (at the
  pointer); Ctrl+click shows it selected in Explorer (`explorer.exe /select,"…"` — Explorer only
  shows it, nothing is run); a plain click says how. In the source, Ctrl+click inside an image
  reference does the same (Step 96 gives Ctrl+click elsewhere a second caret).
- **Save As to another folder** offers to copy the pictures the document refers to (those next to
  the old location that the new folder lacks): a question with Copy / Don't copy; copying never
  replaces a file (same bytes: skipped; other bytes: kept and named in the message); the
  originals stay.
- **Assets on the command line** (`jarak-core::assets`): `jarak assets list <doc.md>` lists the
  document's image references (found with format and size, missing, remote, outside the folder)
  and the orphans — files in `assets/` (four levels deep) that no Markdown file of the folder
  refers to; nothing is deleted. `jarak assets rename <doc.md> <asset> <new-name>` renames the file
  (`MoveFileExW` without replace: an existing file is never replaced, `asset_exists`) and
  rewrites exactly the bytes of the document's references to it (inline and reference
  definitions, `%20` for spaces, encoding and line endings kept, atomic save); it refuses an
  asset other Markdown files of the folder use (`asset_shared`, listed in `details`), a document
  that opens read-only and a reference it cannot locate exactly; `--dry-run` shows the change. If
  the document cannot be saved, the file gets its old name back. The GUI entries for the list
  and the rename come with the command palette (Step 92).
- **Preview end**: scrolling stops when the document's end reaches the bottom of the view (before,
  the last block could scroll away and leave an empty page — found in the real check).

Tests: `assets` 6 (references with the exact bytes of their destinations, inline, `<…>`,
escaped, reference definitions; orphans across the folder's Markdown files; a rename that moves
the file and rewrites only references, its refusals, the never-replacing move); `images` 9 (+ the
scale-down to the texture limit, WebP decode and header size); `media` 3 (answers "loading" at
once and decodes on the worker; the cache drops pictures off screen to stay within its budget and
brings them back with their size known; a changed file is decoded again); window flows 3 new (the
preview reveals a picture on Ctrl+click through the pointer and labels a missing one with its
path, the source reveals a reference, the document stays untouched; Save As to another folder
offers the copy and never replaces files; the preview never scrolls past the end) + `copy_new`;
CLI unit tests and a smoke run (list, dry run, refused onto an existing name, rename with CRLF
kept). Golden examples `cli-envelope-v1/jarak-assets-list.json`, `jarak-asset-exists.json`,
`jarak-assets-rename-dry-run.json`. UI audit view `images` (45 runs, 0 failures; 100 % … 200 %
screenshots reviewed). e2e `step91_jarak_images_assets.rs` (the release window's picture view
with a geometry check of the three pictures, `jarak assets list/rename` through the real CLI, the
renamed picture in a new window's preview; first focused run PASS).

**Measured** (release): 50 screenshot-sized pictures (1920 × 1080) in one document, headless:
scroll frames p50 0.14 ms, p95 3.12 ms, max 7.48 ms while all 50 decode; textures peak 126.6 MB
(budget 128 MB), 34 dropped and decoded again; a keystroke in split view with pictures p95
0.41 ms. First preview switch of 1 MB Markdown 46.7 ms after the end-of-document clamp (budget
100). Real window on this PC: working set 92 MB in the source, 121 MB in the preview after
scrolling through all 50 (peak 143 MB, private bytes 294 MB including the graphics driver's
texture memory), CPU 0 ms in 3 idle seconds after scrolling.

**Real desktop** (test-started windows, a sandboxed profile, the maintainer's windows untouched):
the 50-picture document scrolled to its end and back with the wheel (the pointer over the
pictures; screenshots reviewed: the last picture ends at the bottom); Ctrl+click on a picture
opened one Explorer window on `assets` with the file selected, which the test closed (only that
window); `small.md` at UI scale 100 % and 150 % (`KIM_UI_ZOOM`): the 360 × 180 picture covers
360 × 180 screen pixels, the missing one is labelled. The real check found the empty page at the
end and the hover text placed at the bottom of the whole preview (now at the pointer). Windows
display scaling itself was not changed (it would rescale the maintainer's open windows): the
150 % checks use Jarak's own UI scale, which is the same pixels-per-point path — a real 150 %
display is **not tested**.

## 14. Vim editing, the command palette and the menu bar (Step 92)

- **One engine, two editing modes** (`jarak-core::vim`, `JARAK-FR-050`…`JARAK-FR-053`): the Vim
  layer reads keys and edits the `Document` through its own primitives (`replace`, `type_text`,
  `newline`, `delete_*`, `overwrite_text`, `undo`/`redo`, `set_caret`), so the save point, the
  preview cache, pasted images and every later feature see ordinary edits. `History` gained
  groups: a Vim change (`dw`; `cw`, the typed text and Esc; a block insert) is one undo step,
  Standard editing is unchanged (a test checks typing still merges).
- **State:** the mode, the keys of an unfinished command (shown in the status bar), registers
  (unnamed, `0`, `a`–`z` with append, `_`, `+`/`*` through the window's clipboard), the last
  change for `.` (its keys and the text typed in Insert mode, replayed), `f`/`t` for `; ,`, the
  last search and its highlight, the last visual lines for `'<,'>`. Visual block is a selection
  of one range per line by display columns (wide characters and tabs counted); `I`/`A`/`c` type
  at every caret.
- **The command line** returns what only the window can do as `Ex` values (write, quit,
  write-quit, edit, wrap); `:s`, `:set`, `:noh`, `:N` and searches run in the engine. Patterns are
  Vim's "magic" syntax translated to the `regex` crate (already in the workspace; Unicode
  classes, no backtracking, so no back references inside a pattern).
- **The window** (`jarak-gui`): keys go to Vim first; what Vim does not take (Ctrl+S, Ctrl+E,
  Ctrl+Shift+P, Ctrl+O, …) works as in Standard editing, Ctrl+V pastes in Insert mode. Normal and
  Visual mode draw a block caret and keep Esc (egui otherwise drops the focus on Esc — found by a
  window test); the IME is allowed only in Insert/Replace mode, so with Hangul on `j` is still a
  motion (no IME output → the window's input context is off). Search matches on screen are
  marked behind the text. A read-only document is never changed: an edit is taken back with a
  message. The editing mode is the first key of the settings file
  (`%APPDATA%\mouv\jarak\settings.toml`, `editing_mode = "vim"`, other lines kept, atomic
  write); the UI audit sets the mode without writing it.
- **The command palette** (`palette.rs`, Ctrl+Shift+P, `JARAK-FR-054`): one list of every window
  action and every registry command with its keys, filtered by words in English, Korean, the
  command id or the keys; Up/Down/Enter or a click runs one. The key labels come from
  `keys::key_label`, and a test parses every label and checks `keys::action_for` gives the same
  action — the list cannot drift from the bindings.
- **The menu bar** (UI-10): hidden by default; Alt pressed and released alone shows or hides it
  (egui reports the Alt key itself as a key event — traced on the desktop, the first build
  ignored it), and the palette has *Menu bar*. Six menus (File, Edit, Selection, Go, View,
  Markdown) from the same list, kim-ui ghost buttons and popover menu items with the keys.
- **Assets in the window** (*Images and orphaned assets…* in the palette and the Markdown menu):
  the document's references with their state, the folder's orphans (the open document's unsaved
  references count), Show (Explorer) and Rename (saved documents only: the file moves without
  replacing anything, the references change in the buffer, the document is saved; a failed save
  puts both back).

Tests: `vim` 18 against buffers (motions, paragraph/screen motions, operators with motions, text
objects and counts, line operators and case/indent, change and insert as one undo step, `.` with
counts, registers incl. append and the clipboard, paste, visual, visual block, search and `*`,
`:s` with ranges/flags/Vim patterns, ex commands, pending keys, Korean and wide characters,
insert-mode text and arrows, Standard editing unchanged, pattern translation); `keys` + the
label/binding check; `palette` 2; window flows 7 new (a Vim session: motions, counts shown,
Insert with the IME on, undo/redo, Visual line, `:%s` on the command line, Ctrl+S, `:q` refused,
`:wq`; search highlight and `:noh`, back to Standard; a read-only document stays unchanged; the
mode remembered in the settings file; the palette lists, filters, runs by Enter and click; Alt
alone with the traced events shows the menu bar, a menu runs a command, Alt+Z is not Alt; the
assets dialog lists and renames, a dirty document is not renamed). UI audit views `vim`,
`vim-insert`, `vim-command`, `palette`, `menu-bar`, `assets` (75 runs, 0 failures; screenshots
reviewed: the palette's selection drawn over its text and the assets dialog listing the open
document's own pictures as orphans were fixed). e2e `step92_jarak_vim_palette.rs` (a Vim session
in the release window that writes the file with `:w`, the palette's items, the menu bar's
menus; first focused run PASS). Measured (release, headless): Vim keys in a 10 MB document p50
0.11 ms, p95 4.82 ms.

**Real desktop** (test-started window, sandboxed profile): Ctrl+Shift+P, "editing vim", Enter →
`NORMAL`, the settings file says `editing_mode = "vim"`; `w d w` (real key presses); `i`, the
Hangul key, `g k s r m f Space` → 한글 composed and committed in Insert mode; Esc; `j d d` with
Hangul still toggled → a motion and a line delete (not jamo); `Shift+V` → `V-LINE`; `:wq` →
`COMMAND`, saved and closed (exit 0); the file is `one 한글 three⏎six⏎` with its CRLF
(screenshots reviewed). Alt alone showed the menu bar (after the fix above), the Edit menu listed
its commands with their keys, Esc closed it, Alt hid the bar, typing and Ctrl+S kept working.

## 15. Tabs, two views and the session (Step 93)

- **Tabs** (`tabs.rs`, `JARAK-FR-060`, UI-01): a bar per view with one tab per document — its
  name (`⧉` for a clone), a dot while unsaved (the close `×` on hover), middle click closes,
  dragging reorders, the context menu: close, close the others / to the left / to the right / the
  unchanged ones / all, save all, rename the file, move it to the Recycle Bin, copy the path, the
  name or the folder, show in Explorer, a command prompt there, the default program, move or
  clone to the other view, open or move to a new window. The bar is laid out from the design
  system's tab tokens (the compact 28-point bar, tabs of one whole-point width between 96 and
  220) and recorded for the UI audit (`tabbar`, `tab[i]`); the audit's first run found tabs
  overlapping by 2 px² at fractional widths, fixed with whole-point geometry. The bars show once
  there is more than one document (or a second view). Keys: Ctrl+Tab / Ctrl+Shift+Tab, Ctrl+1…9
  (9 the last), Ctrl+Shift+PageUp/PageDown move the tab, Ctrl+W, Ctrl+Shift+T reopens the last
  closed file.
- **Two views** (`views.rs`, `JARAK-FR-061`): documents are in the main view, the second, or both
  (a clone). A clone is one buffer — the text, the undo history, the file — with a view of its
  own in each pane (scroll, mode, carets); `Tab::view` and the document's selection belong to the
  focused pane, the other pane's live in `Tab::alt` and are swapped when the focus moves, so the
  rest of the window still works on "the active document". The unfocused clone's carets are kept
  as line and column (edits elsewhere leave them on their line; `Document::swap_selection` never
  seals an undo step, so typing still merges). Move, clone, F8 (focus the other view), side by
  side or one above the other, a draggable divider, synchronized vertical / horizontal scrolling
  (the other view keeps its distance). Opening a file while the second view has the focus opens
  it there; an empty second view goes away; the main view always has a document. The focused
  view's active tab carries the accent; a click in the other view moves the focus (a window test
  found the stale egui focus switching back — only clicks switch now).
- **The session** (`jarak-core::session`, `sessions.rs`, `JARAK-FR-062`, REL-01):
  `%LOCALAPPDATA%\mouv\jarak\session.json`, JSON with a schema
  (`docs/contracts/schemas/jarak-session-v1.schema.json`, golden examples in
  `docs/contracts/examples/jarak-session-v1/`, unknown keys and other versions refused), written
  atomically when the window closes (also from `on_exit`), read at start before the command line's
  files: the documents with their caret, top line, mode and views, the second view (order, active,
  orientation, focus), the workspace folder and the recent files. Untitled documents with text are
  kept by the session (not asked about at exit) in `recovery\untitled-<n>.txt`; documents with a
  file and unsaved changes are still asked about. Restoring is a setting
  (`restore_session = true` in `settings.toml`; the palette's *Restore the session at start*
  toggles it; the recent files are kept either way). A session can be saved to and opened from a
  file of the user's choice (FILE-17/18; documents with a file only). Files missing since the last
  session are named in the status bar.
- **Recent and closed files** (`JARAK-FR-028`): newest first, once, at most 20; the palette lists
  them (*Open recent: …*), the File menu too; open all, empty the list, Ctrl+Shift+T.
- **File operations** (`docops.rs`, `JARAK-FR-020`): closing several documents asks about each
  unsaved one in turn (Cancel stops, the rest stay); Save All; Rename moves the file without
  replacing another (`MoveFileExW` without replace) and the document follows; the Recycle Bin is
  asked first and uses `SHFileOperationW` with `FOF_ALLOWUNDO` (never a deletion); the default
  program refuses programs and scripts by extension (`exe`, `bat`, `cmd`, `ps1`, `vbs`, `js`,
  `lnk`, `msi`, `reg`, `py`, …: Explorer shows them instead); a new window starts this program with
  `--new-window <file>` (the document must be saved; *Move to a new window* closes it here).
- **Folder as Workspace and the document list** (`workspace.rs`, `JARAK-FR-063`, `JARAK-FR-064`):
  a left panel with *Folder* (the tree read a folder at a time, folders first, `.git` hidden, a
  filter over names six levels deep, Refresh) and *Documents* (every open document; a click shows
  it). A folder dropped on the window, given to `jarak open <folder>` or the document's folder
  (palette) becomes the workspace; the session restores it. The *Documents…* dialog (WIN-02) sorts
  by name, folder or unsaved first, shows one, saves or closes the checked ones; the Window menu
  lists the open documents (WIN-01).

Tests: `session` 4 (round trip, broken/foreign/newer/unknown-key files refused, the golden example
read by the Rust model, recent files); `tabs` 2, `docops` 1, `workspace` 1; window flows 8 new
(tabs by mouse and keys, reorder, close, reopen; a clone is one buffer with a caret per view and
one undo history; moving between the views and an empty second view going away; closing several
asks about each unsaved one, close unchanged, save all; the session restores documents, views,
carets, the preview mode and untitled text, and only the recent files when restoring is off;
rename, Recycle Bin, Explorer, command prompt, default program refusing a `.bat`, new window; a
dropped folder as the workspace, the document list and the Documents dialog; a session file saved
and opened elsewhere). UI audit views `tabs`, `views`, `side`, `documents` (95 runs, 0 failures
after the overlap fix). e2e `step93_jarak_tabs_views_session.rs` (two release-window runs with a
session in the sandbox: both tab bars, `session.json` naming the clone in both views, the same
bars after the restart; first focused run PASS).

## 16. Find, replace and mark (Step 94)

`JARAK-FR-070`…`JARAK-FR-072`, matrix rows `SRCH-01`…`SRCH-12`, `SRCH-15`, `SRCH-16`,
`MARK-01`…`MARK-08` (supported) and `AUTO-10` (partial).

- **The search** (`jarak-core::search`): a `Query` (text, mode, match case, whole word, `.` matches
  newline) compiles to one `regex::Regex` for every mode: Normal escapes the text, Extended first
  turns `\n \r \t \0 \\ \xNN \uNNNN` into characters, Regular expression uses it as written. The
  text the search scans (`Haystack`: the document as one `String` with a char/byte map) is made
  once per history state and kept on the tab, so F3 again or typing in the find box do not copy
  the document. `find` looks forward or backward from the caret, wraps (and says so) and stays in
  a range ("in selection"); `find_all` counts with a cancel check (Count stops after 5 s, Mark All
  after 10 s, Replace All after 30 s, and says so).
- **Regular expressions** use the Rust `regex` crate (linear time, no catastrophic backtracking),
  not Notepad++'s Boost syntax. The differences: no look-around (`(?=…)`, `(?<=…)`), no
  back-references in the pattern (`\1` inside it) and no possessive or atomic groups — a pattern
  using them is refused with the reason, never searched differently; `\b \w \d \s` and classes are
  Unicode-aware; `(?i)`, `(?m)`, `(?s)` work inline; `^`/`$` match at every line end including
  CRLF ones; `.` never matches `\r` or `\n` unless *`.` matches newline* is on. In the replacement,
  `\1`…`\9` and `$1`, `${name}` insert groups, `$0` the match, and `\n \r \t \\` are characters.
- **Replace All** (`search::replace_all`) is one undo step. Up to 20 000 replacements are one edit
  of many ranges; more rebuild the text once and replace the document's text in a single edit (the
  rope is rebuilt, not edited 832 204 times). Measured (release, `replace_all_on_50_mb`): a 50 MB
  document with 832 204 matches — the haystack 24 ms, Count 125 ms, Replace All 347 ms, undo 87 ms,
  a regex Replace All 609 ms.
- **Markers** (`Document::markers`): ranges with a kind (0 the marks of Mark All, 1–5 the token
  styles, 200 bookmarks) that every edit, undo and redo moves; text typed at a marker's start
  pushes it on, never grows it. They are not text: the file never changes because of them, and
  they are not saved (bookmark persistence comes with Step 101). The view draws the kinds on the
  lines on screen (Mark red, styles blue/amber/green/accent/pink at 35 %) and bookmarks as dots in
  the gutter; at most 200 000 marks per command.
- **The find panel** (`find.rs`): a non-modal popover at the editor's top right with the pages
  Find, Replace and Mark, built from kim-ui components (segmented, text field, checkbox,
  buttons), with Notepad++'s Alt mnemonics (actions A R T M; options C W P B I D; modes N X G). Enter keeps the focus in the find box; F3 works there
  too; Esc closes and gives the focus back to the view. Highlight all paints the find text's
  matches on screen while the Find or Replace page is open. On Windows an Alt+letter chord also
  arrives as the letter's text; the window drops that text when it comes with the chord, so no
  shortcut types its letter (AltGr still types).
- **Incremental search** (Ctrl+Alt+I): a bar above the status bar; each change searches from where
  it started, Enter / Shift+Enter step, the count is shown below 16 MB.
- **Smart highlighting** (`AUTO-10`, partial): a selected word (letters, digits, `_`, under 200
  characters) highlights its whole-word, same-case occurrences on screen; its options come with
  the settings window (Step 110).
- **History**: 20 entries each for find and replace, newest first, Up / Down in the find box,
  kept in the session (`find_history`, `replace_history` in `jarak-session-v1`).
- **`jarak find`** (§2) runs the same search on a file from the command line.

Tests: `search` 10 (each mode and option, wrap and range, one undo step in every mode, the bulk
path, the haystack cache, markers through edits/undo/redo, CRLF anchors; the 50 MB measurement is
ignored by default); window flows 9 new (every option and mode with F3, Enter and Shift+Enter;
Replace and Replace All in every mode and in the selection with one undo; Mark All with bookmarks,
clear and copy; Ctrl+F3 and Ctrl+Alt+F3; the incremental bar, highlight all and smart
highlighting; token styles; the history in the session; the Alt mnemonics; Alt letters never
typing). UI audit views `find`, `replace`, `mark`, `incremental` (115 runs, 0 failures). e2e
`step94_jarak_find_replace_mark.rs` (`jarak find` in every mode with a usage error, the release
window's Mark and Replace pages, the file untouched, the history in the session). Real desktop
(r94): Ctrl+F, Enter, Shift+Enter, Esc, Tab into the replace box, Alt+W and Alt+A (whole-word
Replace All left `concat` alone), Ctrl+Z / Ctrl+Y, a Korean search, the Mark page with Alt+M, the
incremental bar and select-and-find-next, checked byte for byte on disk; r94b drove every other
option and mode with the panel's mnemonics — Regular expression, Extended (`\t`), match case,
backward with wrapping, wrap off ("Not found"), Replace All in the selection and `.` matches
newline — each checked on disk; F3 itself was tested only
headless because F1/F3 are the global hotkeys of the maintainer's Noeul on that PC (one stray F3
made a pin, closed again by its ID).

## 17. Find and Replace in Files, the search results (Step 95)

`JARAK-FR-073`, `JARAK-FR-074`; matrix rows `SRCH-13`, `SRCH-14`, `FIF-01`…`FIF-07` (supported).

- **The walk** (`jarak-core::infiles`): a folder's own files first, then its sub-folders depth first,
  each in name order without case; links and junctions are not followed; hidden folders and files
  (the Windows attribute) only with *In hidden folders*. **Filters** as in Notepad++: names with
  `*` and `?` separated by spaces or `;`, `!name` excludes files, `!\folder` a folder directly in
  the searched one, `!+\folder` one at any depth; no include pattern (or `*.*`) is every file.
- **The search** reads each file (at most 64 MiB; larger ones are skipped and listed), decodes it
  like the window does (§9: UTF-8, UTF-16 with a BOM, the legacy fallback), skips binary files and
  runs the Step 94 matcher (`Matcher::find_bytes`) on a pool of up to eight threads; results come
  back in the walk's order, at most 100 000 listed (the count goes on), with cancellation (Esc).
  Measured (release, this repository): `crates/` 483 files, 13 MB, 11 049 matches in 85 ms;
  `docs/` 830 files in 118 ms.
- **Replace in Files** is two steps. `plan` lists every change (the line before and after) and
  writes nothing; files that do not decode losslessly are skipped with the reason. `apply` takes
  only the changes the user kept and, one file at a time: reads the file again and checks its
  SHA-256 against the plan (*changed after the preview* is skipped), decodes it the same way,
  splices the kept replacements into the text, encodes it in the file's own encoding and
  byte-order mark (an unencodable replacement is skipped), writes the original bytes and the
  batch manifest to the journal **before** replacing the file atomically (`save_atomic`). Line
  endings are untouched because only the matched bytes change. Nothing is deleted; a failure
  leaves the file as it was.
- **The journal** (`%LOCALAPPDATA%\mouv\jarak\replace-journal\<batch>\`, `batch.json` with the
  schema name `jarak.replace-journal` and `1.bak`, `2.bak`, …) keeps the newest 20 batches.
  *Undo the last Replace in Files* (palette) and `jarak journal undo` give each file its original
  bytes back only when it still has the bytes the batch wrote (checked by SHA-256); a file edited
  since is listed and left, and the batch is marked undone only when every file was restored.
- **The window**: the find panel's *Files* page (Ctrl+Shift+F) has the folder (with a folder
  dialog and *Follow the current document*), the filters, *In sub-folders*, *In hidden folders*,
  Find All and *Replace in Files…*; the search runs on a thread with its progress in the panel
  and Esc to stop it. The **results panel** (F7) at the bottom keeps the last 20 searches as
  blocks (the newest open, older ones folded), grouped by file with each line and the match
  highlighted; rows are 24 points (a dense listing, recorded as `tree-row`), a double-click or
  F4 / Shift+F4 opens a match with it selected (also in an untitled open document), the row's
  menu copies the line, every line or the paths. The **preview** (a modal: the find panel hides
  under it) lists every change with a check box per change and per file; Enter or *Replace N in M
  files* applies the checked ones. Files open here with unsaved changes are skipped (*open with
  unsaved changes*); open, saved ones are reloaded after the batch (and after its undo), keeping
  the caret.
- **Find All in documents** (`SRCH-13`, `SRCH-14`): the current or every open document's matches
  in the results panel (untitled documents too); *Replace all in all open documents* replaces in
  each editable document as one undo step per document, leaving read-only ones.
- While a box of the find panel has the keyboard, the window's shortcuts (the palette, files,
  tabs, the find pages, the results) still work; the box keeps text editing, the clipboard, undo
  and F3. The palette ranks a label that starts with the query first.

Tests: `infiles` 6 on a fixture tree (filters, recursion, hidden folders, binary skipped,
cancellation; Replace in Files byte for byte across UTF-8 LF/CRLF/mixed, UTF-16 LE with a BOM and
CP949, only confirmed changes, a left-out file untouched; a file changed after the preview and an
unencodable replacement skipped; undo leaving a later edit; regex groups; 20 journal batches); window
flows 7 new (Find in Files results, F4 / Shift+F4, copy lines and paths; the preview with unchecked
changes and undo; unsaved documents skipped and saved ones reloaded; Find All and Replace All in
documents; shortcuts from the find panel; following the document's folder and copying one line);
palette ranking; CLI capability invariants. UI audit views `files`, `results`, `replace-preview`
(130 runs, 0 failures). e2e `step95_jarak_find_replace_in_files.rs` (the CLI on a fixture tree:
find with filters, `--dry-run` writing nothing, `confirmation_required`, `--yes` keeping every
encoding, journal list and undo restoring every byte; the release window's Files page and results).
Real desktop (r95): Ctrl+Shift+F, Alt+W, Enter; a real double-click on a result row opened a.txt;
next/next/previous results reached b.md's `# cat`; then Replace in Files from the palette, the
preview and Enter (CP949 and CRLF kept, the binary file untouched, the open b.md reloaded) and
*Undo the last Replace in Files* restoring every byte, all checked on disk. Found by the real runs:
the window's shortcuts did not reach a box of the find panel, and the palette ranked "Undo the last
Replace in Files" above "Replace in files…" for that query (both fixed, with tests).

## 18. Multiple carets, multi-selection and column mode (Step 96)

`JARAK-FR-080`, `JARAK-FR-081`, `JARAK-FR-012`; matrix rows `MULTI-01`…`MULTI-10`, `EDIT-04`,
`EDIT-05` (supported).

- **The model** (`jarak-core::document::multi`): the selection was already a sorted set of ranges
  (Step 86) and every edit command works on each range and records the selection before it, so
  typing, Backspace/Delete, Enter, paste, indent and undo/redo apply to every caret and undo brings
  them all back. Step 96 adds the commands that make such selections and the column block.
- **Carets and occurrences**: Ctrl+click adds a caret (Ctrl+click on an existing one of several
  removes it; Ctrl+drag adds a selection); a Ctrl+click on a Markdown image reference or a link
  keeps its Step 89/91 meaning and leaves one caret. *Add the next occurrence* (Ctrl+Shift+D): the
  first press on a caret selects the word under it, each further press adds the next occurrence
  after the last range (wrapping); started from a word it matches whole words with their case,
  started from a selection the text as it is. *Select all occurrences* (Alt+F3) likewise; the
  palette has Notepad++'s four variants of each (ignore case and whole word, match case only,
  whole word only, match case and whole word) and *skip this occurrence*. Esc goes back to one
  caret (the view keeps Esc while there are several, so egui does not take it as "drop the
  focus"). Ctrl+D stays *Duplicate line* (Notepad++'s key, `LINE-01`, Step 97); the issue's
  "Ctrl+D" for the next occurrence became Ctrl+Shift+D for that reason.
- **Column selections** (`Block`: anchor and head as (line, display column)): one range per line
  between the same display columns — a wide Hangul/CJK character or an emoji counts two, a tab
  runs to its stop; an edge inside a wide character takes the nearer boundary — made by Alt+drag
  (past a line's end the pointer goes on in cells), Alt+Shift+arrows (from the caret; without
  virtual space the head stops at its line's end) or *Begin/End Select in column mode*. A column
  selection is a multi-selection (Notepad++'s "column to multi-editing" is always on), so every
  command works on each row; the block ends with the first edit, its carets stay. Several lines
  pasted into as many carets go one per caret; copying joins the rows with the document's line
  ending.
- **Virtual space** (palette, kept in `settings.toml` as `virtual_space`): typing or pasting into a
  column selection pads rows shorter than its left edge with spaces in the same undo step; the
  carets past a short line's end are drawn at the block's column.
- **Wide characters are two cells on screen**: the view lays out each wide character in its own
  section with letter spacing so that its advance is exactly two monospace cells (the system's
  Korean fallback font is narrower or wider than that), so column selections, tab stops and Vim's
  visual block line up on screen as they do in the model.
- **The Column Editor** (Alt+C; Notepad++'s dialog): text, or numbers with the initial value, the
  increase, the repeat count, no/zero/space leading and decimal, hexadecimal (upper case), octal or
  binary digits, all the same width; inserted into every row of the column selection at its left
  edge, or else from the caret's line to the last line at the caret's display column, padding
  shorter lines; one undo step. Enter inserts and Esc closes (both read before the dialog draws,
  so a focused button cannot swallow them).
- **Begin/End Select** (`EDIT-04`, `EDIT-05`, palette): the first run marks the start, the second
  selects to the caret, as a stream or as a column selection.
- The status bar counts the carets ("· 3 carets"); the find panel's Alt mnemonics act only while
  the panel has the keyboard, so Alt+C in the editor is the Column Editor.

Tests: `multi` 5 (carets typing, deleting, pasting one line each and undoing with every caret back;
next/all occurrences with their options and skip; a block lined up by display column across
Hangul, ASCII and an emoji, Alt+Shift+arrows, a short line; virtual space padding as one undo step;
the Column Editor from the caret and in a block; numbers in every base with repeat, leading zeros
and spaces, negative values); window flows 8 new (real pointer events: a click and Ctrl+clicks
typing, pasting, undoing, Esc; Ctrl+clicks past line ends; Ctrl+Shift+D and Alt+F3 and a palette
variant; Alt+Shift+arrows over Hangul; Alt+drag with virtual space and no menu bar; the Column
Editor's numbers, a bad number refused, text with Enter; Begin/End Select in both modes). UI audit
views `multi`, `column` (a block over the fixture's Hangul lines), `column-editor` (145 runs, 0
failures). e2e `step96_jarak_multi_carets_column.rs`. Perf (release): a keystroke frame in a 10 MB document p95 0.23 ms
with the two-cell layout of wide characters (0.13 ms before). Real desktop (r96, checked on disk):
a real click and two real Ctrl+clicks past three line ends typed "!" at each; a real Alt+drag down
column 0 typed "> " into four lines; Alt+Shift+Down ×3 and Right ×4 selected `> al`, `> be`,
`> ga` and `> 한` (display columns 0–4 with a Hangul line) and "*" replaced each; undo brought the
text and the four ranges back; the Column Editor through real clicks (the Number page, the initial
number, leading zeros) and Enter inserted `08`…`12` into five lines, undone in one step. Found by
the real runs and fixed: Enter was taken by the focused segment button in the Column Editor (Enter
and Esc are now read before the dialog draws, also in the Replace in Files preview); the number
fields had no visible labels. In the first run the two Ctrl+clicks did not land (one caret went to
the last line); three later runs and a step-by-step probe added the carets correctly and it did
not recur.

## 19. Line operations and text transformations (Step 97)

`JARAK-FR-082`…`JARAK-FR-085`, `JARAK-FR-087`, `JARAK-FR-088`; matrix rows `LINE-01`…`LINE-19`,
`CASE-01`…`CASE-06`, `BLANK-01`…`BLANK-07`, `EDIT-06`…`EDIT-09`, `TOOL-01`…`TOOL-04`,
`PLUG-03`…`PLUG-06` (supported).

- **Commands in the registry** (`jarak-core::command`, 54 new ids under `line.`, `sort.`, `case.`,
  `blank.`, `mime.`, `convert.`; ids may now hold digits): every operation is a registry command —
  in the palette, behind a key where Notepad++ has one, nameable by macros (Step 102) — and one undo
  step. They are refused in read-only documents (everything but moving the caret and selecting
  counts as editing now), and an operation that cannot run says why in the status bar
  (`Document::take_notice`).
- **Where they work** (`jarak-core::lineops`): line operations on the lines the selection touches
  (a selection that ends at a line's start leaves that line out); removing, sorting, reversing,
  shuffling and the blank operations on the whole document when nothing is selected (the empty line
  after a final line break is not a line of it); rewritten lines are joined with the document's line
  ending. Duplicate, delete, move, join, split (at the view's width in cells, at word boundaries,
  Hangul two cells wide), remove duplicate / consecutive duplicate / empty / blank lines, insert a
  blank line above or below, reverse, randomize.
- **Sorting**: by code point, ignoring case, in the user's locale (`CompareStringEx` of Windows), as
  integers or decimals with `.` or `,` (the number a line starts with; lines without one first),
  by length — ascending or descending, stable; with a column selection by the text in its columns.
- **Case** on every selected range: UPPER, lower, Proper (blend keeps the rest), Sentence (after
  `. ! ?` and their full-width forms; blend), iNVERT, ranDOm (Unicode case mapping).
- **Blanks**: trim trailing / leading / both, EOL to space, trim both and EOL to space, TAB to
  spaces (tab stops, wide characters two cells), spaces to TAB (runs of two or more reaching a stop;
  all or only the indentation).
- **Encodings of the selection** (its UTF-8 bytes): Base64 (padded, unpadded, URL-safe; decoding
  takes either alphabet and ignores blanks), URL percent-encoding, quoted-printable (RFC 2045, hard
  breaks kept as CRLF, soft breaks at 76), ASCII ↔ hex. Decoding refuses text that does not decode
  or bytes that are not UTF-8, and changes nothing.
- **Digests**: MD5 and SHA-1 implemented in `lineops` (RFC 1321, FIPS 180-4, checked against the
  standard test vectors; no new dependency), SHA-256 and SHA-512 from `sha2` — of the selection or of
  a file to the clipboard, or in the *Generate a digest* dialog (type text, see the digest).
- **The window's tools** (`hashdlg.rs`): the date and time — Windows' short and long formats of
  the user's locale (`GetDateFormatEx`, `GetTimeFormatEx`), or a custom format (`date_format` in
  `settings.toml`, default `yyyy-MM-dd HH:mm:ss`, Notepad++'s tokens; the settings window, Step 105,
  will show it) — copying the full path, the file name or the folder, and the names or paths of all
  open documents; Copy and Cut without a selection take the line unless *Copy the line when nothing
  is selected* is off (`copy_line` in `settings.toml`); copying nothing leaves the clipboard alone.

Tests: `lineops` 14 (one or more per operation, all with Hangul, accented or emoji text: duplicate,
delete, move, join, split, the removals, blank lines, reverse, shuffle, every sort including a
column, every case, every blank operation, every codec round trip and known values, the digest test
vectors, the date format); window flows 3 new (the keys, the palette with a refused decode and a
read-only document, dates/paths/digests/the dialog/the copy-line setting); palette and key tests
check every registry command's key. UI audit views `hash`, `palette-lines` (155 runs, 0 failures).
e2e `step97_jarak_line_operations.rs` (the release window runs commands on a CRLF file with Hangul
and saves it; the palette and the digest dialog). Real desktop (r97, checked on disk): Ctrl+D,
Ctrl+Shift+L, Ctrl+Shift+Down, the palette's consecutive-duplicate removal and descending sort,
Ctrl+A Ctrl+Shift+U, Alt+U, Base64 encode and decode, Ctrl+J, Ctrl+Alt+Enter with Korean typing,
the custom date — every step's bytes as expected, CRLF kept.

## 20. Syntax highlighting, languages and comments (Step 98)

`JARAK-FR-090`, `JARAK-FR-091`, `JARAK-FR-086`; [ADR 0103](adr/0103-jarak-syntax-highlighting.md);
matrix rows `LANG-01`, `LANG-03`…`LANG-07`, `LANG-09`…`LANG-11`, `CMT-01`…`CMT-03` (supported),
`LANG-08` (partial: folding itself is Step 99).

- **Languages** (`jarak-core::syntax`): 23 built in — C, C++, C#, Rust, Go, Java, JavaScript,
  TypeScript, Python, PowerShell, Batch, Shell, HTML, CSS, JSON, YAML, TOML, XML, SQL, Markdown,
  INI, Lua, PHP, Ruby — each a `Lang` declaration run by the code, markup or Markdown lexer. HTML
  embeds JavaScript in `<script>` and CSS in `<style>`; PHP files are markup with `<?php … ?>`
  blocks of PHP code; Markdown highlights fenced code in the fence's language (`rust`, `py`, `js`,
  `sh` …). Detection: the extension (user languages first), then a `#!` line (`env` understood,
  version suffixes such as `python3`), then `<?xml`; else plain text.
- **Tokens**: keyword, type, constant, string, number, comment, operator, preprocessor, tag,
  attribute, variable, heading, emphasis, code, link — coloured with kim-design's tokens
  (`languages::syntax_color`; emphasis in italics).
- **Incremental**: a line is lexed from the state its previous line ended in; the window keeps each
  document's line-start states, lexes the lines on screen of each view every frame and drops the
  states from the first line an edit touched (`Document::edits_since`, a log of the newest 1024
  edits); switching the language starts over. Measured (release): 10 MB of Rust, 415 280 lines —
  lexing to the end the first time 164 ms, a screen 0.02 ms, after an edit 0.04 ms; opening it at its
  end 514 ms; a keystroke frame p95 0.29 ms.
- **The Language menu** (palette and menu bar): *detect from the file*, *Normal text*, every
  built-in and user language; the status bar names the language. The choice is per document and
  not kept in the session yet.
- **User-defined languages**: JSON files in `%APPDATA%\mouv\jarak\languages\` — `name`,
  `extensions`, `shebang`, `case_sensitive`, `line_comment`, `line_start_comment`,
  `block_comment` and `strings` (`open`, `close`, `escape`, `multiline`, `line_start`, `max_len`),
  `keywords`, `types`, `constants`, `operators`, `variable_prefix`, `preprocessor`, `ident_chars`,
  `key_separator`, `sections`, `numbers`, `fold` ([open, close] pairs). A broken file is skipped and
  named. *Import a Notepad++ user language (XML)* converts a UDL 2.x file into such a JSON file
  (never replacing one: `name-2.json`); *Export the language* writes the current language (a
  built-in one too) as JSON; *Open the user languages folder*, *Read the user languages again*.
  Imported from a UDL: name, extensions, case, comments (`00` line, `03`/`04` block), Keywords1
  (keywords), 2 (types), 3 (constants), 4–8 (keywords), the operators, eight delimiters with their
  escapes, the code folding markers. Not imported: styles and colours, prefix and number
  definitions, comment folding, line-comment continuation and position rules.
- **Folding markers** (`syntax::fold_ranges`): matched open/close markers outside comments and
  strings become fold ranges; the folding margin and commands that use them are Step 99.
- **Comments** (registry commands; Notepad++'s keys): toggle the line comment (Ctrl+Q), comment
  (Ctrl+K), uncomment (Ctrl+Shift+K) — the language's line comment at the lines' common indentation,
  or its block comment around each line where it has none (HTML, CSS, XML, Markdown use
  `<!-- -->`); block comment (Ctrl+Shift+Q) around the selection or the caret's line and block
  uncomment (palette). A selection stays on the rewritten lines; plain text says it has no comments.

Tests: `syntax` 11 golden tests (a fixture per language with the kind of each key construct, the
state carried across lines; detection of every listed extension and of shebangs; a JSON user
language highlighting and folding; a Notepad++ UDL imported, highlighting a sample, folding and
surviving a JSON round trip; the state cache dropping states after an edit) plus the 10 MB
measurement; `lineops` comment and edit-log tests; window flows 3 new (a Rust file highlighted, the
Language menu to plain text, Python and back, re-highlighting after typing; comment keys following
Python, CSS and plain text; user languages loaded, a Notepad++ UDL imported through the dialog and
exported); a highlighted keystroke-frame measurement. UI audit views `syntax`, `palette-language`
(165 runs, 0 failures). e2e `step98_jarak_syntax_languages.rs` (a Rust file, a JSON user language
and an imported Notepad++ UDL picked by extension in the release window, the Language menu) —
first run PASS (2026-11-22). Real desktop (2026-11-22, a test-started window): a Rust file is shown
as Rust with its colours, the status bar's language opens the 26 `Language:` entries, Language:
Python and back to Rust. Folding with the markers: Step 99 (§21).

## 21. Folding, the Function List and structure navigation (Step 99)

`JARAK-FR-092`, `JARAK-FR-093`, the go-to / select-matching-bracket part of `JARAK-FR-096`, the
go-to-line-or-offset part of `JARAK-FR-103`; matrix rows `FOLD-01`…`FOLD-06`, `VIEWS-11`,
`SRCH-18`, `SRCH-19`, `LANG-08`.

- **Structure** (`jarak-core::structure`, no UI): `outline(lang, lines, with_symbols)` lexes the
  text once (Step 98's lexers) and returns the folds and the symbols.
  - Folds by language: brackets `{}`/`[]` outside strings, comments and preprocessor lines (C-like
    languages, Rust, Go, JavaScript, TypeScript, CSS, JSON, PowerShell, PHP code); indentation
    (Python, YAML, Lua, Ruby, Shell, Batch, SQL); sections (INI, TOML); elements opened and closed
    on different lines and multi-line comments (HTML, XML; void and self-closing elements do not
    fold); headings to the next heading of their level or above and fenced code (Markdown); a
    user language's `fold` markers. Multi-line block comments fold in code languages. Each fold
    has its level (1 = outermost); plain text has none.
  - Symbols: definitions introduced by `fn`, `def`, `class`, `function`, `func`, `struct`, `enum`,
    `trait`, `interface`, `impl`, `mod`, `module`, `type` (Go, TypeScript, Rust), `record`,
    `namespace` … after modifiers (`pub`, `async`, `export` …); C-like function definitions (a name,
    its parameters and `{` on the line or the next); `name = function` / `=> {` (JavaScript,
    TypeScript, Lua); `name() {` (Shell); Markdown and HTML headings (with their level); INI/TOML
    sections; the outermost keys of JSON and the top-level keys of YAML; Batch labels; CSS rules;
    SQL `create` statements. Lines in comments and strings are skipped.
  - `matching_bracket`: the bracket at or just before the caret and its partner, skipping strings
    and comments.
- **Folded state** (`Document`): a folded fold is a marker (`FOLDED`) at the start of its first
  line, so it moves with edits like a bookmark; `set_folded_many` changes many in one pass. The
  window gives the document its hidden ranges; vertical moves (arrows, Page Up/Down) skip them.
  Folding never changes the text or the file.
- **The window** (`jarak-gui::outline`): each view's document has its structure found again after
  an edit — at once up to 5 000 lines, else on a worker thread after a pause of 0.3 s in typing
  (the text is a cheap copy of the rope), so a keystroke never waits for the whole text. Meanwhile
  the folded folds keep their length and follow their markers; for one edit with one caret they
  just move by the lines it added or removed. Nothing is laid out again in a frame where neither
  the text, the folds nor the carets changed; a caret that lands inside a folded fold (a search
  hit, Go to, an edit) opens it. One worker runs per document at a time, and its result is
  taken even when the text changed meanwhile (found by Step 100's measurement: typing before the
  first structure arrived started a worker every frame). Folding is per document: both views of a cloned document show the
  same folds (Notepad++ folds each view on its own).
- **The fold margin**: a cell between the line numbers and the text; a down-pointing triangle
  beside an open fold's first line, a right-pointing accent one beside a folded fold, whose line is
  underlined. A click on it folds or unfolds without moving the caret.
- **Commands** (Notepad++'s keys; the View menu): fold all Alt+0, unfold all Alt+Shift+0, fold /
  unfold level 1–8 Alt+1…8 / Alt+Shift+1…8, fold / unfold the current level (the innermost fold
  around the caret; folding moves the caret to its first line) Ctrl+Alt+F / Ctrl+Alt+Shift+F.
  Go menu: Go to… Ctrl+G (a line, or a character offset; it says where the caret is and how far the
  text goes; a wrong entry says so and the dialog stays), the matching bracket Ctrl+B, the next /
  previous symbol (palette). Selection: everything between the matching brackets Ctrl+Alt+B.
- **The Function List** (View menu; a right panel): the active document's symbols indented by
  depth, with their kind, the one around the caret highlighted, a filter (case-insensitive), a
  click goes there; only the rows in view are laid out.
- Not in this Step: brace and quote highlighting, auto-close and tag matching of `JARAK-FR-096`;
  navigation history and quick open of `JARAK-FR-103`; folds kept in the session.

Measured (release, 10 MB of Rust, 249 663 folds and 83 221 symbols): the first structure 452 ms on
the worker; keystroke frames p95 0.35 ms while typing (the structure 460 ms after the last key);
fold all 74 ms; moves through the folded text p95 1.41 ms; typing with every fold folded p95
9.23 ms (`folded_structure_of_a_10_mb_rust_file`).

Tests: `structure` 5 (bracket folds with levels outside strings and comments; indentation,
sections, Markdown headings and fences, tags, markers, none for plain text; the symbols of every
language family; brackets both ways; folded markers following edits and moves skipping them);
window flows 5 (keys, the margin, levels, the current level, a fold following typing, the file
untouched; Go to and the matching brackets opening folds; the Function List with Korean Markdown
headings, a filter, next / previous symbol, a Rust file, closing it; Python by indentation; the
Function List, the margin and the Go to dialog under the UI audit's rules, headless); the 10 MB
measurement. UI audit views `functions`, `goto`; the line numbers on screen are in the geometry
report (`gutter:line#n`). e2e `step99_jarak_folding_functions.rs`: its first run (2026-11-22) found
that the Function List's title and the Go to dialog's position line were not in the geometry
report, so the audit rules never saw them; both are recorded now (`functions:title`, `goto:where`)
and the run passes. Real desktop: Alt+0 folds a Rust file to 7 drawn lines and Alt+Shift+0 unfolds
it to 13; the Function List lists `alpha`, `beta`, `Gamma`, and a click on `beta` puts the caret on
line 6; a Markdown file folds 8 → 3 lines and lists its headings (a click on `Three` goes to line
6); a Python file folds by indentation 7 → 4 and lists `first`, `Box`, `size`.

## 22. Editing assistance (Step 100)

`JARAK-FR-094`, `JARAK-FR-095`, `JARAK-FR-096`; matrix rows `AUTO-01`…`AUTO-09`, `JRK-12`; the
user-facing reference is [JARAK_ASSIST](JARAK_ASSIST.md).

- **`jarak-core::assist`** (no UI): `words_in` / `document_words` (the distinct words starting
  with a prefix), `candidates` (snippets, API functions and keywords, the language's keywords,
  types and constants, words; one text once, the richer kind winning), `path_candidates` (never a
  network path), Notepad++ API files (`parse_npp_api`, `load_apis`, `builtin_api` for C, Python,
  JavaScript), `call_at` and `signature` (the call around the caret, outside strings and comments),
  snippets (`parse_snippets`, `snippets_with`, `expand` into text and tab stops), and
  `match_bracket_by` / `match_tag_by` (bounded, taking the lines and tokens from the caller).
- **`Document` typing aids** (`document::assist`): `Aids` (closing brackets and quotes, closing
  tags, `IndentMode` off / keep / smart, the language's quotes, colon indentation, markup, the
  indentation unit) set by the window each frame; `type_assisted` (stepping over a closer an aid
  inserted, opening a pair, wrapping a selection, closing a tag, a closing bracket's dedent),
  `delete_backward_assisted` (an empty pair it inserted), `newline` with the indentation of the
  aids, `indent_unit` (the document's own); closers it inserted are marked (`AUTOCLOSE`, an empty
  marker that moves with edits) so only those are stepped over or removed in pairs; snippets as
  markers `SNIPPET + n`, `insert_snippet`, `snippet_step`, `clear_snippet`. The document's default
  is Keep and no closers (the behaviour before Step 100).
- **The window** (`jarak-gui::assist`): `AssistSettings` (read from and written to the settings
  file; palette entries), the popup (`Completion`: entries, choice, its start and word, how it was
  opened), the hint (`Hint`: call, overload, parameter), `assist_key` before the window's keys,
  `after_typed` / `after_deleted`, the bracket highlight cached per (edit, caret). Tab and Enter are
  taken by a popup that opened by itself only after an arrow key chose an entry (or when set to
  always); one asked for takes them. Esc closes the popup, the hint or a snippet's stops (the view
  keeps the keyboard for it).
- **Costs**: words from 2 000 lines around the caret, scanned on the rope (no string per line); an
  open popup is narrowed as the word grows; a word that found nothing is not searched again as it
  grows; the highlight looks at most 3 000 lines for a partner, with the cached line states of the
  highlighter. Measured (release, 10 MB of Rust): keystroke frames with the popup, closers and the
  highlight p95 1.30 ms; the Step 98 highlighted keystroke p95 1.10 ms and the Step 99 ones
  (keystroke p95 0.59 ms, typing with every fold folded p95 10.22 ms) with Step 100 in place.

Tests: `assist` 7 (completion sources, Notepad++ API files and calls, paths without the network,
snippets with stops and mirrors and the user's file, closers stepping over, wrapping and deleting in
pairs, tags and the three indentation modes, bounded bracket and tag matching); window flows 8
(the popup never taking Enter or Tab unasked, the settings; a snippet's stops and undo; closers and
tags only when on, smart and kept indentation; hints following the call and the bracket highlight;
the user's snippets and API files with a broken one named; paths, tag matching and overloads; the
popup and the hint under the UI audit's rules, headless; one structure worker at a time); the 10 MB
measurement. UI audit views `completion`, `hint`. e2e `step100_jarak_assist.rs` — first run PASS
(2026-11-22). Real desktop: typing `ap` opens the popup (apple, apricot), Down then Enter inserts
`apricot`, Esc closes it, and Ctrl+Space with one answer inserts it at once (`ba` → `banana`);
`print(` shows its hint and Esc closes it; the `def` snippet's two stops give `def go(x):` and
`pass`; closing brackets give `(1)` (the typed closer steps over); `<div>` closes to `</div>`; smart
indentation indents after `if v:`; the matching brackets are boxed. Each aid turned off in the
real window stops it (completion, hints, snippets, closing brackets, tags, indentation, the
highlight).

## 23. Bookmarks, the change history, the Document Map and quick navigation (Step 101)

`JARAK-FR-100`…`JARAK-FR-102`, `JARAK-FR-104`, the navigation history and quick open of
`JARAK-FR-103`; matrix rows `MARK-09`…`MARK-15`, `VIEW-14`, `VIEW-15`, `VIEWS-09`, `JRK-13`,
`EDIT-14`.

- **Bookmarks** (`document::navigate`; the marker of Step 94): toggle on every caret's line
  (Ctrl+F2), next / previous wrapping (F2 / Shift+F2), clear, inverse; copy, cut, remove the
  bookmarked lines, remove all the others, paste to (replace) them line by line (the last
  clipboard line again when it runs out). Removing a line removes its bookmark; undo brings the
  text back, not the bookmark (as in Notepad++). Bookmarks are not edits: a read-only document
  takes them (`Command::edits`).
- **The change history**: every edit (typing, paste, undo, a line operation) marks what it inserted
  (`CHANGED`, a marker that moves with later edits; a run of typing extends one, a deletion leaves
  an empty one); saving turns them into `SAVED`. The margin shows a bar between the line numbers
  and the fold margin — the warning colour for an unsaved change, the success colour for a saved
  one. Next / previous change (the first line of each run of changed lines, wrapping), clear.
- **Hidden lines** (Alt+H, Notepad++'s Hide Lines): a `HIDDEN` marker over the selected lines
  (a selection ending at a line's start does not take it; the first line of the text cannot be
  hidden). They are folded away like folded folds (not drawn, skipped by moves); the line above
  shows the folded triangle, a click on it shows them again; a caret that lands inside shows them;
  *Show all hidden lines*.
- **The Document Map** (View; a right panel 112 points wide): one bar per row from each sampled
  line's indentation and end (up to 120 columns; up to 3 points a line in a short document, many
  lines a row in a long one), the lines on screen in an accent frame; a click or a drag scrolls
  there. The rows are found again only when the text or the panel's height changes; the map asks
  for no repaint of its own. Measured (release, 10 MB, 149 798 lines, 856 rows): the first frame
  0.8 ms, keystroke p95 0.85 ms, scroll p95 0.40 ms.
- **The navigation history** (Alt+Left / Alt+Right): a jump — another document, or a move of more
  than a screen (Ctrl+End, Go to, a search hit, a symbol; the map scrolls without moving the caret) —
  remembers where the caret was (100 places); back and forward open the document again (a closed one is said so).
- **Quick open** (Ctrl+P; Go → *Go to file…*): open documents first, then the workspace's files (the
  open folder, else the active document's), walked on a worker thread (50 000 files at most, 12
  levels, `.git`, `target`, `node_modules`, `.svn`, `.hg`, `__pycache__`, `.venv` skipped), matched
  by the letters in order (contiguous and file-name matches first). Notepad++ prints with Ctrl+P;
  Jarak's printing (Step 106) takes another key.
- **The Summary** (View): full path, created, modified, characters without line endings, words,
  lines, the document's length in bytes as saving writes it (its encoding and BOM), the selection.
- **Open the selected path / its folder** (Edit): relative to the document's folder.
- The core's notices (*No bookmark.*, *No change.* …) are shown in the window's language.

Tests: `navigate` 4 (bookmarks toggled, stepped, inverted, their lines copied, removed, replaced;
the change history modified → saved, its runs and clearing; hidden lines and the summary's
counts; which commands edit); window flows 5 (bookmark keys and the clipboard commands, read-only
documents; the change bars, Alt+H and the margin; the map's click and the history back and
forward; quick open with the workspace and the Summary; the map, quick open and the Summary under
the UI audit's rules, headless); the 10 MB map measurement. A Go to dialog opened from the palette
no longer takes the palette's Enter (found while testing the Summary, which had the same problem).
UI audit views `docmap`, `quick-open`, `summary`. e2e `step101_jarak_navigation.rs` — first run
PASS (2026-11-22). Real desktop: Ctrl+F2 on line 5, then F2 from the end comes back to it; the
Document Map shows and a click near its bottom scrolls a 60-line file (first line 1 → 17);
Ctrl+P, `words`, Enter opens that file; Alt+Left returns to the first file on line 5; Alt+H hides
lines 3–5 (numbers 1, 2, 6, …) and *Show all hidden lines* shows them; *Copy the bookmarked lines*
puts `line 1`, `line 3` on the real clipboard (the maintainer's text put back after); the Summary
shows the path, dates and counts.

## 24. Macros and user commands (Step 102)

`JARAK-FR-110`, `JARAK-FR-111`; matrix rows `MAC-01`…`MAC-07`, `RUN-01`…`RUN-04`, `EDIT-15`,
`VIEW-05`; the reference and the security notes are [JARAK_MACROS](JARAK_MACROS.md).

- **`jarak-core::macros`** (no UI, nothing run): `Step` (a registry command by id, typed text, a
  clipboard / indentation / save action, a find or replace with its options), `Macro`,
  `RunCommand`, `UserCommands` (`macros.json`, `jarak.macros/1`, written atomically, a broken file an
  error), the built-in macro and commands, `Vars` and `expand` (Notepad++'s `$(…)` names, `SYS.`),
  and `target` (a web address, a file to open, or a program and its arguments as written).
- **The window** (`jarak-gui::macros`): recording hooks in `act` (the actions that edit, move,
  select, use the clipboard, indent or save), in typing and in Find Next / Replace / Replace All.
  Playback runs the steps through the same paths with recording off, each run one undo group, and
  stops for *until the end* at the end, no progress, a wrap, 100 000 runs or 20 seconds. Shortcuts
  are checked against `kim_hotkeys::reserved::chord` (app scope), Jarak's bindings, typing keys and
  each other; `user_shortcut` runs before the window's keys. The `Runner` trait
  (`ProcessRunner`: `CreateProcess` with the arguments raw, no shell, `ERROR_ELEVATION_REQUIRED`
  refused, not waited for) and the `Opener` (web addresses, files) do the running, both only from the
  Run dialog's button; tests give recording fakes.
- **Keys**: Ctrl+Shift+R record, F9 play back (Notepad++'s Ctrl+Shift+P is the palette), F5 Run….

Tests: `macros` 2 (the file's round trip, defaults, unknown format and broken file; variables,
`SYS.`, unknown names and the targets); window flows 7 (recording commands and text, F9 and undo,
until the end and per-run undo, a macro without progress stopping; find and replace recorded with
their options, the panel kept; shortcut conflicts, the save dialog capturing keys, the shortcut
playing, the built-in Trim and Save; the exact line shown, nothing run before the button, saving,
confirming a saved command, the Wikipedia and web search addresses, the browser refusal, the
elevation refusal; the real runner; the manager changing a shortcut and removing a command, an
HTML file opened in the browser; the dialogs under the UI audit's rules, headless). UI audit views
`run`, `macros`. e2e `step102_jarak_macros.rs` — first run PASS (2026-11-22). Real desktop: a macro
recorded with real keys (Home, `- `, Down) plays back with F9; one Ctrl+Z takes back one run; Until
the end of the file stops at the end (Played: 3); the recorded macro is saved by typing its name;
the Run dialog shows the typed line expanded and runs nothing on Esc. The real check found that
the Save and Run dialogs did not give their first field the keyboard focus (the typed name or line
went nowhere); both do now, as Notepad++'s Run box does, and the window tests type into them.

## 25. The Encoding menu, line endings and Show Symbol (Step 103)

`JARAK-FR-025`, `JARAK-FR-026`, `JARAK-FR-037`, the menu of `JARAK-FR-021`, the clickable status of
`JARAK-FR-035`, binary content of `JARAK-FR-027`; matrix rows `ENC-01`…`ENC-16`, `BLANK-08`,
`VIEW-06`…`VIEW-11`, `UI-06`, `EDIT-12`.

- **The character sets** (`file::CHARSETS`, `encoding_rs`): Korean (EUC-KR / Windows-949), Japanese
  (Shift-JIS, EUC-JP, ISO-2022-JP), Chinese (GB2312 / GBK, GB18030, Big5), Cyrillic (Windows-1251,
  KOI8-R, KOI8-U, ISO 8859-5, OEM 866), Western European (Windows-1252, ISO 8859-15), Central
  European (Windows-1250, ISO 8859-2), Greek (Windows-1253, ISO 8859-7), Turkish (Windows-1254),
  Hebrew (Windows-1255, ISO 8859-8), Arabic (Windows-1256, ISO 8859-6), Baltic (Windows-1257,
  ISO 8859-4, ISO 8859-13), Vietnamese (Windows-1258), Thai (Windows-874). ISO 8859-1 and
  ISO 8859-9 are read as Windows-1252 and Windows-1254, as browsers do (WHATWG). Windows-1258
  holds Vietnamese with combining marks; precomposed letters it lacks (ế) are unmappable.
- **The Encoding menu** (palette and menu bar), with targets UTF-8, UTF-8-BOM, UTF-16 BE BOM,
  UTF-16 LE BOM, ANSI (the system code page) and every character set:
  - *Reinterpret as …*: `file::decode_as` reads the file's bytes again in that encoding (a BOM of
    it is taken off and kept). Nothing is written. A document with unsaved changes asks first, and
    an untitled one has no bytes to read. A lossy reading stays read-only; reinterpreting with the
    right encoding makes it writable.
  - *Convert to …*: the text stays and the next save writes the new encoding.
    `file::unmappable` names the characters it cannot hold (how many, the first ones with their
    lines) and asks; agreeing replaces them with `?` in one undo step. Saving still refuses any
    character that cannot be written.
  - *New documents in UTF-8 / UTF-8-BOM / ANSI* (setting `new_encoding`).
  - **The status bar**: a click on the encoding, the line ending or the language opens the palette
    on those entries.
- **EOL conversion** (Edit; Windows CR LF, Unix LF, Macintosh CR): `Document::convert_eol` rewrites
  each line ending in one undo step; bookmarks and other marks stay on their lines. A text whose
  line endings are all of one kind now makes it the document's (as on open; after undoing a
  conversion too).
- **Show Symbol** (View; each a setting):
  - spaces and tabs: a dot, an arrow;
  - line endings: a CRLF / LF / CR / NEL / LS / PS tag after the line;
  - control characters: drawn as their names (`NUL`, `ESC`, `DEL`, `C1`, `ZWSP`, `ZWNJ`, `ZWJ`,
    `LRM`, `RLM`, `WJ`, `BOM`, `SHY`) in a box;
  - *Show all characters*: the three above together;
  - indent guides: dotted, at each level of the leading whitespace (a blank line takes the smaller
    of its neighbours');
  - the wrap symbol at the end of each wrapped row;
  - edge lines at the columns of `edge_columns` (80 by default).

  Only the lines on screen are drawn.
- **Binary content** (Edit): *copy*, *cut* and *paste binary content* keep NUL characters. They use
  a clipboard format of their own (`Jarak Binary Text`: the UTF-16 length, then the units),
  because plain text stops at the first NUL for every other program.

Tests:
- `file`, 4 tests:
  - every character set round-trips a sample in its script and is decoded back losslessly;
  - UTF-8 and UTF-16 with and without BOM are reinterpreted;
  - Korean bytes are garbled as Windows-1252 and right again as EUC-KR;
  - lossy conversions are found; Reinterpret reads the file again and never writes it.
- `document`: EOL conversion and undo, marking unsaved, replacing characters.
- `view`: control character names.
- Window flows, 5 new:
  - Reinterpret of a Shift-JIS file opened with the Korean fallback, asking about unsaved changes;
  - Convert naming the unmappable characters, Cancel, replacing, saving in Windows-1252 and in
    UTF-16 LE BOM;
  - the status bar's encoding and line ending opening their entries;
  - EOL conversion and undo, the new-document encoding;
  - Show Symbol reaching the view, binary content keeping NUL through the clipboard;
  - the question and every symbol under the UI audit's rules, headless.
- An older test now expects LF where its text held only LF line endings.

The real `jarak info --fallback` reads files that Python's codecs wrote (Shift-JIS, EUC-KR, KOI8-R,
Windows-874, GB18030, Windows-1256) as those encodings, losslessly, without changing them. UI
audit views `symbols`, `encoding-question`. e2e `step103_jarak_encodings.rs` (every character set
through the real `jarak info`, the two views) — first run PASS (2026-11-22). Real desktop: a Shift-JIS
file opens read-only as EUC-KR (that reading would lose characters); Reinterpret as Shift-JIS shows
it and makes it writable without touching the bytes; the status bar's encoding opens the
`Encoding:` entries; Convert to UTF-8 and to UTF-16 LE BOM and EOL Unix save exactly those bytes;
Show all characters draws the `LF` tag. Binary content through the real Windows clipboard: copied
from a NUL file (read-only, `JARAK-FR-027`) it is on the clipboard as `Jarak Binary Text` beside the
plain text, and pasted into another document it is saved with the NUL (`78 0D 0A 61 00 62 0D 0A`);
the maintainer's clipboard text was saved before and put back after.

## 26. Files changed on disk, recovery snapshots and the session (Step 104)

Requirements `JARAK-FR-120`, `JARAK-FR-121`, `JARAK-FR-122`; the user's reference is
[JARAK_RECOVERY](JARAK_RECOVERY.md).

- **`jarak-core::file`**: a `FileDocument` keeps the `DiskStamp` (size, last write) of its file,
  taken before the bytes are read (a write in between shows as a change) and after each save;
  `disk_state(path, seen)` says `Same`, `Changed(stamp)` or `Missing`. `reload_undoable` reads the
  file again through `Document::set_text_minimal`, which replaces only the part between the common
  start and end as one undo step (marks outside it stay; a grown file is an insertion at the end),
  then marks the result saved — Ctrl+Z brings back the text from before, unsaved. The format and a
  read-only reason follow the file; the user's own read-only choice stays.
- **`jarak-core::recovery`**: a window's `Store` writes `w-<pid>-<start>\manifest.json` (schema
  `jarak-recovery-v1`) and one UTF-8 text per document, texts first and the manifest last through a
  temporary file and a rename; a text that did not change is not written again; an empty list
  removes the folder; limits (100 documents, 512 MB) leave the largest out and name them.
  `orphans(root, alive)` lists folders whose owner (or claimer) no longer runs; `owner_alive` and
  `current_owner` (Windows: `OpenProcess`, `GetProcessTimes`, `GetExitCodeProcess`) compare the
  process id *and* start time, so a reused id is not the old window. `Orphan::claim` renames a
  folder to `claimed-<pid>-<start>-<n>` for one window; a file name outside the folder is refused.
- **`jarak-core::session`**: `save` keeps the good file it replaces as `session.json.bak`;
  `load_or_backup` falls back to it for a damaged file, which it keeps as `session.json.broken`.
- **`jarak-gui::watch`**: the program turns the checks on (`with_watching`); each frame starts a
  check on a short-lived thread when the window got the focus or 2 s passed (1 s while monitoring)
  and takes the last answers (stale ones, for a document saved meanwhile, are dropped). A change
  becomes a reload (monitored, or *silently* for a clean document) or a question in a queue; a
  missing file marks the document unsaved. `save` asks `safe_to_save` first. Compare opens the
  file's version as a read-only tab with a label (`Tab::title`) in the second view. Settings:
  `detect_changes`, `reload_unmodified_silently`.
- **`jarak-gui::recover`**: `with_recovery(root, owner, alive, interval)` (main.rs, Windows, with a
  session) offers orphans at start and keeps snapshots: each frame compares the dirty documents'
  history states with the last written ones and starts a write (a rope copy, a thread) when the set
  shrank (saved, discarded, closed) or a text changed and the interval passed. Restore claims the
  folders, opens the entries (a file's document gets the text by its difference, so Ctrl+Z shows
  the file) and removes the claimed folders after its own snapshot is written; Discard asks twice.
  A normal exit (`recovery_exit`) waits for a write and removes the folder. In a UI audit both run
  only with `KIM_UI_SESSION=1` (as e2e and real-desktop checks use it).

Tests: core file 2 (disk states including our own save and an appearing file; the undoable reload
with bookmarks, BOM and the user's read-only choice), document 1 (setting the text by its
difference), recovery 5 (offered text, limits and kept texts, claims, the golden example, the
process check), session 1 (backup, fallback, `.broken`, a newer session left alone), the
integration test `recovery_kill.rs` (a real child process writes snapshots, is killed, and the
next scan finds its text with the real process check); window flows 12 (see JARAK_RECOVERY). UI
audit views `disk-question`, `recovery`; schema `jarak-recovery-v1` with a golden and an invalid
example. e2e `step104_jarak_recovery.rs` — first run: its session check failed on the test's own
assumption (a single document hides the tab bar, so its name is not in the report); with two
documents it passes (the window opened the backup's documents both times). Real desktop
(2026-11-22): a test instance killed after real typing in a file and an untitled document; the
next start offered both and Restore gave back the exact text (`# 자락␍␊저장하지 않은 편집 unsaved`,
`untitled 메모`), which its own snapshot and the saved file hold; another program's change noticed
in 0.7 s (1.1 s with unsaved changes), Reload then Ctrl+Z and Save gave back the original, Compare
opened `a.txt (on disk)` beside it, Keep mine saved mine, a deleted file was written again by Save.

## 27. Settings (Step 105)

Requirements `JARAK-FR-130`, `JARAK-FR-131`, `JARAK-FR-132`; the reference is
[JARAK_SETTINGS](JARAK_SETTINGS.md).

- **`jarak-core::settings`**: the registry `SETTINGS` — each setting's key, section (General,
  Editor, Appearance, Markdown, Images, Vim, Keyboard, Files, Advanced), group, English and Korean
  name and help, kind (bool, a whole number in a range, a choice, text with a required part,
  columns) and default. `validate` normalises a value or refuses it with what the setting takes;
  `Settings::{get, set, reset, list}` work on `settings.toml` (`key = value` lines; other lines and
  comments kept; `write_line` writes through `file::save_atomic`). `schema()` and `defaults_json()`
  build `jarak-settings-v1` and its example (a test keeps the files in `docs/contracts` equal; another
  keeps every key in JARAK_SETTINGS.md).
- **`jarak settings list | get | set | reset`** (`jarak` CLI) on the same file; codes
  `setting_not_found` (3), `setting_invalid` (2); `set` / `reset` are `write` with `--dry-run`.
- **`jarak-gui::prefs`**: `Prefs` (the window's settings, read from the file) and
  `JarakApp::apply_settings` (re-reads the file: the parts' own settings through
  `read_settings_file`, then tab width / spaces / look for every document, word wrap when it
  changed, the language unless `--lang` fixed it, the recovery interval, the recent files).
  `prefs_frame`: the theme (`kim_ui::set_theme`; System read again on focus), the editor font
  (installed as the family `jarak-editor` from the Windows or the user's fonts folder, the built-in
  monospace chain behind it), each new document's word wrap and Markdown mode (`Tab::fresh`; a
  restored session's documents keep their mode), and `settings_changed_outside` on focus (the
  file's modification time). The theme and font apply only with a settings file (the program
  always has one; window tests opt in).
- **`jarak-gui::settings_ui`**: the window (an immediate viewport *Jarak Settings*, embedded under
  the headless backend), a rail of `kim_ui::nav_item`s, rows from the registry (`Switch`,
  `dropdown` — it carries the row's name for screen readers —, `number_field`, `text_field` saved on
  losing the focus), `change_setting` (write, then `apply_settings`; a refusal shown under the row),
  Advanced's *Show in Explorer* and *Reset every setting* (twice). `prepare_audit("settings-<id>")`
  opens a section and lets that window write the audit (ADR 0100).
- **Behaviour behind the settings**: `view::Look` (font size and family, line numbers,
  current-line highlight, a caret blinking at most twice a second while focused, Ctrl+click carets),
  `tabs::BarOptions` (close buttons, double-click), `Document::use_spaces` (Tab and indentation
  insert spaces to the next stop), `assets::write_new_in` (the pasted-image folder), the menu and
  status bars, new documents' line ending, `single_window` in `main.rs`, Ctrl+, (`Action::Settings`).
- **Shortcuts** (part 2): `jarak-core::hotkeys` — the manifest (`hotkeys.manifest.json`, 112 `app`
  actions in Step 105, 163 since Step 106 with the commands that have no default keys; backend
  `focus`), `load` (manifest + user layer, safe mode for a broken or newer file),
  `change` (`Set` / `Disable` / `Reset` / `ResetAll` through `kim_hotkeys::edit`, written with
  `store::save`; a refusal writes nothing; a newer file is never replaced, an invalid one only by
  Reset all), `keys_of`. `jarak hotkeys list/show/set/disable/reset/check` use it with the shared
  view and report data. `jarak-gui::shortcuts` — `manifest_json` (built from the palette's keyed
  commands, without the keys §7.2 reserves; a test keeps the shipped manifest and the contract
  example equal), the contract names of egui keys, `Bindings` (the user's chords and the freed
  defaults; `JarakApp::action_of` replaces every direct `keys::action_for`), the Keyboard section
  (your changes first, Change with capture before anything else sees the keys, Off, Reset, Take it,
  Reset all twice), `hotkeys_changed_outside` on focus.

Tests: core settings 6 (the registry, values checked, the file kept, schema and example, the
reference), hotkeys 2 (the manifest valid without conflicting defaults; set, a default replaced, a
clash refused and taken, dry run, disable, reset, reset all, an invalid file never rewritten),
document 1 (spaces), assets 1 (folder); CLI 2 (settings; hotkeys list/show/set/disable/reset/check
with codes); jarak-gui unit 2 (key names, the shipped manifest); window flows 3 more (a remapped
command, the Keyboard section's capture, refusal, Take it and Reset all, a change from the CLI);
window flows 10 (the gear, every section listed, live apply of the status bar, line numbers, word
wrap, theme and language, a refused value, tab width and spaces, new documents and the image
folder, multi-editing and the menu bar, Markdown's mode, a change from the CLI, reset twice, every
section under the audit's rules in English and Korean). UI audit views `settings-general` …
`settings-advanced` (275 runs in all, 0 failures; screenshots reviewed in English dark and Korean
light). e2e `step105_jarak_settings.rs`: its first run found that the line numbers, drawn at the
user's font size, were measured against the UI type scale — they are recorded as the user's size
now (`kim_ui::inspect::last_text_is_users_size`); then PASS.

## 28. Export (Step 106)

[JARAK_EXPORT](JARAK_EXPORT.md) is the user reference. Part 1: HTML, RTF, publishing, Copy as HTML /
RTF and `jarak export`; part 2: printing, PDF, the print settings, `jarak print` and
`jarak printers`.

- `jarak-core::export` (no UI toolkit; kim-design for its palettes, which are plain data):
  - `lines`, `highlight` (the Step 98 lexer line by line, its state carried; tokens cut into runs
    of one kind, neighbours of a kind merged), `kind_name` / `kind_color` (the window's colour
    roles, ADR 0103);
  - `code_fragment` (classes for a page, or `Styling::Inline` colours for the clipboard),
    `code_html`, `code_rtf` (`\uN?` for non-ASCII), `cf_html` (the clipboard's `HTML Format`
    header with byte offsets);
  - `markdown_html` renders the Step 89 render model (`markdown::parse`) block by block — headings
    with GitHub anchors, tables with their alignment, task lists, fenced code highlighted through
    `syntax::fence_lang`, footnotes at the end, raw HTML as text — and treats images by target
    (`Embed`: `data:` URIs; `LinkFrom`: relative to the output; `Publish`: copies into `assets/`);
    images come only from `assets::resolve` (inside the document's folder), are checked by
    `assets::image_format` and their size (64 MiB each, 256 MiB per export), never decoded, and
    remote ones become links;
  - `safe_href` (only `http`, `https`, `mailto`, `#`, relative), a fixed CSP, `esc`;
  - `write_output` (refuses the document itself and, without `overwrite`, an existing file; a
    temporary file in the target folder, flushed and renamed) and `publish` (the page and the
    copies planned first, every existing target reported at once, the document's own folder
    refused);
  - `ExportError::code` gives the CLI codes `export_exists` (7) and `export_into_source` (2).
- `jarak export` reads the file as `jarak info` does, detects its language with the user's
  languages, refuses options that do not fit before reading, and prints the files and image uses.
- `jarak-gui::export_ui`: `export_tool(0…4)` from the palette and menus (`Run::Export`): the
  export Save dialog (`Dialogs::save_export`, `GetSaveFileNameW` with the format's filter,
  `OFN_OVERWRITEPROMPT`), the publishing folder (`Dialogs::export_folder`) with a dry run first and
  the *Replace files?* dialog (`publish_question_ui`, audit view `publish-question`), and Copy as
  HTML / RTF through `Clipboard::set_rich` (`HTML Format` / `Rich Text Format` registered formats
  beside `CF_UNICODETEXT`; the memory clipboard keeps them for tests).

Tests: core export 8 (lines and runs, escaping, RTF, `HTML Format` offsets, links, images embedded /
linked / refused with the document unchanged, writing without replacing, publishing) and golden
tests `export_golden.rs` 2 (`tests/golden/export/expected/`: code HTML with and without line
numbers, RTF, the clipboard HTML, rendered Markdown embedded / linked / published; the outputs were
reviewed by reading them); CLI 1 (formats, `export_exists`, `--overwrite`, `export_into_source`,
dry run, refused options); window flows 4 (Markdown export with unsaved text, RTF and Copy as HTML
/ RTF, publishing asks before replacing, publishing needs a saved Markdown document). e2e
`step106_jarak_export.rs`: PASS on its first run.

Part 2 (printing):

- `jarak-core::print` lays a document out into pages of drawing operations (`Op::Text` with a
  baseline, `Op::Fill`, `Op::Image`), a pure function of the text, the page (`Geometry`: size and
  resolution from the printer), the settings (`Setup`, from `settings::print_setup`) and a
  `Measure` of the device's fonts (`Font`: `Mono` / `Sans`, size, bold, italic). `code_pages`:
  the Step 98 runs wrapped at blanks (a long word at characters), tabs expanded, line numbers in a
  gutter; `markdown_pages`: the render model's blocks as rows (headings, paragraphs of styled
  characters, lists, quotes with bars, code bands through the same code layout, tables with
  natural column widths scaled to the column, rules, footnotes last, images from
  `assets::resolve` sized from their header at 96 dpi). `paginate` puts rows on pages and adds the
  header and footer (`expand`: `left|center|right`, the variables). `describe` prints pages as
  text for golden tests. `Printers` is the backend interface (`list`, `print(job, layout)` — the
  layout runs on the device's measure); `NoPrinters` and `RecordingPrinters` (`FakeMeasure`, A4 at
  300 dpi) serve tests; `Printer::writes_a_file` (port `PORTPROMPT:`); `Job` (printer, output,
  the dialog's `DEVMODEW`, name, dry run).
- `jarak-core::print::gdi` (Windows): `printers` (`EnumPrintersW` level 2), `default_printer`, and
  `WinPrinters::print`: `CreateDCW` with the dialog's `DEVMODEW` (copied into aligned memory), the
  physical page and offsets from `GetDeviceCaps`, `Fonts` (Consolas / Segoe UI, Malgun Gothic for
  any character `GetGlyphIndicesW` says the face lacks; widths from `GetTextExtentPoint32W` and
  metrics cached), then `StartDocW` with `lpszOutput` for a file-writing printer (no dialog),
  `TextOutW` at baselines per font run, `FillRect`, `StretchDIBits` (images decoded within the
  limits, alpha over white), and for an output file a wait until the spooler has written and
  closed it (60 s at most). A dry run stops after the layout.
- `jarak print` / `jarak printers` / `jarak export --to pdf` (`print_document`): the print settings
  from `settings.toml`, the document's language, the printer checked (`printer_not_found`, a
  file-writing printer without `--output` refused), the overwrite rules of the exports.
- `jarak-gui::print_ui`: `Action::Print` (the print dialog, `Dialogs::print` → `PrintDlgW` with
  `PD_USEDEVMODECOPIESANDCOLLATE`, the printer from `DEVNAMES`, the `DEVMODEW` bytes, the range),
  `Action::PrintNow` (the default printer), `export_tool(5)` (Export as PDF); `output_for` asks
  the export Save dialog for a file-writing printer; `start_print` runs the job on a thread with
  the document's text as it is, the language, `prefs.print` and the variables; `print_frame`
  shows the result. `JarakApp::printers` is `NoPrinters` until `main` gives it `WinPrinters`.
  The manifest now also lists the commands without default keys (`defaults: []`), so *Print…*
  can be given Ctrl+P.

Tests (part 2): core print 5 (templates, wrapping, code pages numbered / wrapped / paginated /
black, Markdown blocks and images, the recorder's rules) and golden pages 1
(`sample.rs.pages.txt`, `note.pages.txt`); CLI 1 (options, `printer_not_found`, a file-writing
printer without `--output`, a dry run on the real Microsoft Print to PDF, `export_exists`,
`export_into_source`); window flows 4 (the dialog's printer, settings and selection range; the Save
dialog for a PDF printer, Print now, Export as PDF; no printers; the print settings live). e2e
`step106_jarak_export.rs` section 5: a real PDF through Microsoft Print to PDF, PASS. Real PDFs of
both samples were rendered with `Windows.Data.Pdf` and reviewed.

## 29. Headless command line and Kim interoperability (Step 107)

[JARAK_CLI](JARAK_CLI.md) is the user reference.

- `jarak` (CLI): `new` (`file::Format::from_name`, `file::encode` refusing unmappable characters,
  `create_new` so a file is never replaced), `convert` (`FileDocument` opened, its format changed,
  `Document::convert_eol`, the bytes encoded first so nothing lossy is written, then the atomic
  `FileDocument::save`), `session list` (`Session::load`, read-only), `assets add`
  (`assets::add_image`), `debug-info` (`about::windows_build`, the files' existence), and
  `open --language/--read-only` (passed to the window as tokens before the files). Global options
  are checked against their command before anything runs. A test checks that every command of the
  capabilities document has a golden JSON example.
- `jarak-core::assets::add_image`: the image checked by its first bytes and size (64 MiB), the
  document opened as the window would (a read-only one refused), the next free name planned
  (`plan_new_in`) for a dry run, then copied with `write_new_in` (`create_new`, never replacing)
  and a Markdown image line (alt text, then the path `assets/…`) inserted as a line of its own in the document's line endings; the
  document saved atomically; on a failed save only the copied image is removed.
- `jarak interop invoke` (`crates/jarak/src/interop.rs`): one `kim.interop.request` from stdin
  (64 KiB at most; schema, capability and interface major checked: `capability_missing`,
  `tool_incompatible`), then the matching command's own function, so the answer is that command's
  data and its rules hold (no file replaced without `overwrite`, no window in a dry run). A test
  keeps `PROVIDES` and `provides` in `capabilities.json` equal. Kim enforces consent (`write`,
  `opens_window`).
- `jarak-gui`: `open_args` reads `--language <name>`, `--read-only` and `--session <file>` tokens
  for the files after them (the first start and every hand-off), `choose_language_named`
  (`LangChoice::Named`, the user's languages first), read-only with its reason, and
  `open_session_file`; `main` takes `--no-session` (`SessionFiles { restore: false, save: false }`:
  the saved session and its untitled texts are left alone). Window → Copy debug info
  (`about::debug_text`).

Tests: core about 1, assets 1 (`add_image`: dry run, line endings, never replacing, refusals); CLI
7 more (new, convert, session list, assets add, interop, debug info, the examples); window flows 3
(language and read-only options, session options, debug info). e2e `step107_jarak_cli_interop.rs`:
the real commands, and the release `jarak` installed by the debug `kim` from a test-signed
loopback catalog, its five capabilities found and invoked with Kim's consent rules — PASS on its
first full run.

## 30. Extensions (Step 108)

[JARAK_EXTENSIONS](JARAK_EXTENSIONS.md) is the user reference; [JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)
the Notepad++ plugin audit.

- `jarak-core::extensions` (no UI toolkit): `Manifest` (serde, unknown fields refused), `check`
  (schema and version, id, name, version, every path inside the folder, language files through
  `syntax::parse_udl`, snippet files through `assist::parse_snippets`, commands: unique ids,
  `program_path` — an `.exe` in the folder or a System32 program by name, never a script — the
  timeout, and every permission a command `needs` declared), `load_all` (folders named by their
  id; the others reported, not used), `install` (a checked folder copied file by file — no links,
  200 files, 50 MiB — into a staging folder, then moved into place; an installed id only with
  `replace`), `remove`, `Grants` (`load`, atomic `save`, `covers`: the manifest's SHA-256 and the
  needed permissions; `grant`, `revoke`), `run` (the program without a window from a fresh empty
  temporary folder with `SystemRoot`, `TEMP`, `TMP` and `PATH` only; a Windows job with
  kill-on-close, 512 MiB per process and no crash dialogs; stdin written on a thread, stdout and
  stderr read with limits; killed at its timeout; a non-zero exit reported with stderr's first
  line; UTF-8 output only).
- `jarak extensions list/check/install/remove/grant/revoke` (`extensions_in`, testable with any
  folder); `Paths::extensions_dir`, `Paths::extension_grants_file`; schema `jarak-extension-v1`.
- `jarak-gui::ext_ui`: `with_extensions` (languages into `user_langs` — kept when the user's
  languages are read again — and snippets into the snippets), `extension_entries` (palette and
  Edit menu, `Run::Extension(n)`), `run_extension` (the grant question first:
  `extension_question_ui`, audit view `extension-question`; Allow grants and saves, Cancel runs
  nothing), `start_extension` (refused on a read-only document for document outputs, or without a
  selection for selection input; the document's text and selection remembered; the program on a
  thread), `extension_frame` (a message, a new document, or — only when the document is still as it
  was — the selection replaced or the output inserted, in the document's line endings, as one undo
  step).

Tests: core extensions 4 (the sample checks and is the contract example; checks and declared
permissions; install / replace / grants follow the manifest / revoke / remove; `sort.exe` with
input, a failing program, a timeout); CLI 1; window flows 3 (asked once, Cancel, the output undone in
one step, the count as a message; a document changed meanwhile keeps its change; the question under
the audit's rules). e2e `step108_jarak_extensions.rs`: PASS on its first run.

## 31. Large files, hardening, accessibility (Step 109)

[JARAK_PERFORMANCE](JARAK_PERFORMANCE.md), [JARAK_SECURITY](JARAK_SECURITY.md) and
[JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md) are the references.

- **A single 10 MB line** hung the window (8.7–11.6 s a keystroke; found by the new measurement).
  Four causes, each fixed:
  - `view::show` laid out the whole line every frame. A line of more than `LONG_LINE` (10 000)
    characters without wrap is now laid out only around the visible columns: `window_of` picks
    three screen widths around the scroll position, or around the caret when the view follows it.
    `VisualLine::build_from` keeps the tab stops from the window's start column; `Laid::skip`,
    `Laid::full` and `Laid::at` map line offsets into the laid part, so caret, selection, hits,
    symbols and the IME composition work as before; `drawn` keeps each line's column-0 origin.
  - Completion collected words from the whole long line: now at most 100 000 characters around
    the caret, and the prefix from at most 4 096 characters before it.
  - Bracket matching copied the caret's line: skipped on lines of more than 20 000 characters.
  - The structure was computed inline for documents of few lines however long: now only when
    they are also at most 2 MiB.
- **Large-file mode**: `JarakApp::is_large` (`prefs.large_file_bytes`, setting `large_file_mb`).
  `lang_for` gives none (no highlighting, folding or structure aids); wrap is refused and kept off
  each frame; the Document Map is not drawn; brace highlighting is off; a *Large file* status item.
- **Print layout**: quotes nested more than 16 deep draw 16 bars and lists stop indenting at 60 % of
  the column. The security tests found a `u8` overflow at 256 nested quotes.
- **Security tests** (`crates/jarak-core/tests/security.rs`, deterministic xorshift inputs): decoding
  (lossless means byte-exact), hostile Markdown through parse, HTML export and print layout,
  highlighting in every language, image bombs and garbage, asset references, extension manifests,
  corrupt state files.

Tests: security 7; window flows 2 (large-file mode; a 60 000-character line edited at both ends);
perf 4 more (a single 10 MB line within the keystroke budget, fifty tabs, Find All in 10 MB, a 10 MB
file in large-file mode). e2e `step109_jarak_hardening.rs`: PASS on its first run.
