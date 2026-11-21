# Jarak Architecture

Status: **current** (Step 85: crates, CLI foundation, window shell, single instance, Kim
integration; Step 86: the text model, §8) · Owner: mouv · Decision:
[ADR 0102](adr/0102-jarak-fourth-kim-tool.md) · Requirements:
[JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)

## 1. Crates and programs

| Crate | Program | Contents now | Never links |
| --- | --- | --- | --- |
| `jarak-core` | — (library) | `paths` (per-user folders), `target` (`file[:line[:column]]` open targets), the text model (`text`, `selection`, `history`, `document`, `command`; §8) | a GUI toolkit, windowing, the clipboard (`cargo xtask deps-check`) |
| `jarak` | `jarak.exe` | The headless CLI: `version`, `capabilities`, `paths`, `open` | a GUI toolkit (`deps-check`) |
| `jarak-gui` | `jarak-gui.exe` | The window shell on `kim-ui` (egui + glow), the single-instance hand-off | — |

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
always means the same text, and that undo-all/redo-all return the original and final text.
