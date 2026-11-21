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
