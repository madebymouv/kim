# Jarak Architecture

Status: **current** (Step 85: crates, CLI foundation, window shell, single instance, Kim
integration; Step 86: the text model, §8; Step 87: files and encodings, §9; Step 88: the editor
window, §10; Step 89: Markdown, §11) · Owner: mouv · Decision:
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
