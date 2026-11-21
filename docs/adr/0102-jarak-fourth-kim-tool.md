# ADR 0102: Jarak, the fourth Kim tool — scope, crate boundaries, milestones and priority

- Status: accepted
- Date: 2026-11-22
- Issue: [#103](https://github.com/madebymouv/kim/issues/103) (Step 84) of the Jarak roadmap
  #103–#130 (Steps 84–111), maintainer assignment "Kim — Register and Complete the Entire Jarak
  Development Roadmap" (2026-11-22). Extends ADR 0002 (independent tool binaries), ADR 0055
  (sequential development on `main`), ADR 0092 (the design system in code) and ADR 0097 (one
  patch version per change). Requirements: `JARAK-FR-001`, `JARAK-SEC-001`, `JARAK-SEC-009`,
  `KIM-FR-012`, `KIM-SEC-001`.

## Context

The maintainer wants a fourth Kim tool, **Jarak (자락)**: a native Rust text editor with
Notepad++'s editing power, a minimal terminal-inspired look, optional Vim-style modal editing and
first-class Markdown with instant source/preview switching and Ctrl+V image paste
([JARAK_PHILOSOPHY](../JARAK_PHILOSOPHY.md)). The roadmap has 28 Steps (84–111) registered as
issues #103–#130. Until now the queue rule (ADR 0055, AGENTS.md §6) was "take the earliest
unfinished Step whose dependencies are met", which would put older unfinished Kim issues
(#8, #31, #44, #63, #71–#88, #97, #98) first.

## Decision

### 1. Jarak is an independent Kim tool

Jarak follows the tool pattern of Noeul and Ddalgak: its own packages, its own version group
(`cargo xtask bump`/`version-check`), its own catalog entry, capabilities document and
requirements (`JARAK-*`, milestones `J1`–`J5`, registered in `xtask/src/docs.rs`). It needs no
other Kim tool and no resident process.

### 2. Crate boundaries

| Crate | Kind | Contents | May depend on |
| --- | --- | --- | --- |
| `jarak-core` | library | Document model (rope, positions, graphemes, display width), selections and carets, transactions and undo, the command registry and editing commands, search, encodings and file persistence (atomic save), the Markdown render model (a toolkit-independent block/inline tree), image-asset naming and path rules | `ropey`, `unicode-segmentation`, `unicode-width`, `encoding_rs`, `regex`, `pulldown-cmark`, `serde` — **no GUI toolkit, no windowing, no clipboard** |
| `jarak` | binary `jarak.exe` | The headless CLI per the CLI contract | `jarak-core`; on the headless list of `cargo xtask deps-check` |
| `jarak-gui` | binary `jarak-gui.exe` | The window: text view and layout, tabs and views, the preview painter, the Windows clipboard and image decoding, the Vim key layer, panels, the Settings window, single-instance hand-off | `jarak-core`, `kim-ui`, `kim-design`, `kim-hotkeys`, `eframe`/`egui` (the same pin as the other Kim GUIs, ADR 0014), `image`, `windows-sys` |

Separation rules: the GUI translates input to commands and draws state; it never edits the rope
directly. The preview is a pure function of the text (`JARAK-SEC-001`). Clipboard and image
decoding live in the GUI crate behind a small interface, so `jarak-core` stays testable and
portable. Later Steps may split `jarak-core` modules into crates (for example syntax
highlighting, Step 98, or extensions, Step 108) when a dependency would otherwise reach the
headless CLI.

The text view is drawn by Jarak itself on egui's painter (galleys per visible line), not with
egui's `TextEdit`, which lays out the whole text every frame and has no rope, multiple carets,
column selection or folding.

Library choices recorded now (each Step re-checks maintenance and licence before adding it):
`ropey` (rope, MIT), `unicode-segmentation` (already in the workspace), `unicode-width`,
`encoding_rs` (MIT/Apache), `pulldown-cmark` (MIT), `regex` (already in the workspace),
`image` (already in the workspace). Syntax highlighting (tree-sitter versus TextMate grammars)
is decided in Step 98 with measurements.

### 3. Milestones

`J1` Foundation (Steps 84–88), `J2` first usable release (89–92; the MVP journey passes at Step
90), `J3` Notepad++ editing parity (93–104), `J4` settings, integrations and extensions
(105–108), `J5` hardening, parity and delivery (109–111).

### 4. Temporary priority: Jarak first

For this assignment the maintainer prioritises the Jarak Steps 84–111 over the older unfinished
Kim issues. The sequential rule of ADR 0055 stays (one session, one checkout, `main`, one Step at
a time, no parallel agents); only the queue order changes: the next Step is the earliest
unfinished **Jarak** Step whose dependencies are met. Older issues stay open as they are — none
is closed, relabelled or changed to make the queue look shorter. When the Jarak roadmap is done
(or the maintainer says so), the queue returns to the ADR 0055 order.

### 5. Parity is tracked row by row

[JARAK_NOTEPADPP_MATRIX](../JARAK_NOTEPADPP_MATRIX.md) lists every Notepad++ built-in feature
(285 rows from the 8.9.x user manual's menus) with an owner Step, a requirement and a
verification; `cargo xtask docs-check` refuses a row without them. A row is `supported` only with
its own evidence. Notepad++ plugin DLLs, Plugins Admin and the updater are `not intended`.

## Consequences

- AGENTS.md lists `JARAK` among the registered prefixes and records the temporary priority.
- The workspace gains three members in Step 85; `deps-check` keeps `jarak` headless.
- The Kim catalog lists Jarak as planned until Step 111 packages a validated release.
- Building a serious editor on egui's painter is more work than a widget, but it is the only way
  to meet the budgets and the Notepad++ editing features; the risk is tracked in Step 88's
  measurements.

## Rejected alternatives

- **Scintilla through FFI**: the fastest path to Notepad++ parity, but a large C++ dependency,
  Win32-only, at odds with the shared design system and Kim's rendering, and it would invite
  plugin-compatibility expectations Jarak does not meet.
- **A web view (WebView2) for the Markdown preview**: a heavy runtime and a second rendering
  model; the philosophy rules it out.
- **egui `TextEdit` as the editor**: no rope, no multiple carets or columns, whole-text layout
  per frame.
- **Starting the editor in an existing tool's crate** (e.g. Subak's renderer): couples
  independent tools (ADR 0002).
