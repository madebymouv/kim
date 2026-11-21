# Jarak Philosophy

Status: **current** (Step 84, specification; nothing is implemented yet) · Owner: mouv ·
Applies to: Jarak (자락), the fourth Kim tool · Decision: [ADR 0102](adr/0102-jarak-fourth-kim-tool.md) ·
Requirements: [JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md) · Parity target:
[JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)

*Jarak* (자락) is the hem of a garment, the trailing edge of a long cloth: the part that is
always with you. Jarak is the text editor that is always at hand — for a note, a log file, a
config, a source file or a Markdown document with screenshots in it.

## 1. What Jarak is

A **small, fast, native Rust text editor** for plain text, source code and Markdown that
combines:

1. **The editing power of Notepad++** — tabs and two views, Notepad++-class search and replace
   (regular expressions, mark-all, find in files), multiple carets and column mode, line
   operations, encodings and line endings, syntax highlighting with user-defined languages,
   folding, a function list, macros, sessions and recovery. The target is written down row by
   row in [JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md).
2. **A minimal, terminal-inspired look** — a monospace text area that fills the window, no
   toolbar, a monochrome palette with a subtle purple accent where it means something (the
   caret, the current match, the active mode), quiet chrome, the Settings gear in the bottom-right
   corner of the status bar, Settings in a window of its own (as in Ddalgak and Kim Manager,
   ADR 0100).
3. **Optional Vim-style modal editing** — normal, insert and visual modes, motions, operators,
   counts, registers, `.` repeat and an ex command line, always discoverable (a command palette
   lists every command with its keys) and never a trap: the default editing mode is the
   standard one every Windows user knows.
4. **First-class Markdown** — GitHub-flavoured Markdown rendered natively (no web view), one key
   to switch between the editable source and the rendered preview, an optional split, and
   **Ctrl+V of a screenshot** that saves the image next to the document and inserts a portable
   relative reference to it (an image reference to `assets/ideas-20261122-001.png` for
   `ideas.md`).

Terminal-inspired means the *look and the keyboard*: Jarak is a native GUI application that
renders images, tables and code blocks and uses the mouse naturally. It is not a text-mode
program.

## 2. Principles

1. **The user's files are the user's.** Jarak changes a file only when the user saves or
   explicitly replaces in it. Opening and saving without an edit writes identical bytes:
   encoding, byte-order mark, line endings, a missing final newline and trailing whitespace are
   kept. Rendering a preview, highlighting, folding or indexing never writes anything
   (`JARAK-SEC-001`, NON-NEGOTIABLE).
2. **Never lose work.** Saves are atomic (a failed save never truncates the original), unsaved
   changes are asked about, recovery snapshots survive a crash, an external change is detected
   and offered as a choice, not applied silently (`JARAK-SEC-002`, `JARAK-FR-120…121`).
3. **Fast and frugal.** One native process; nothing runs in the background when Jarak is
   closed; no repaint when nothing changes; only the visible lines are laid out and highlighted;
   large files stay responsive (budgets in [JARAK_REQUIREMENTS §3](JARAK_REQUIREMENTS.md)).
4. **Local-first and private.** No account, no cloud, no telemetry, no ads. Pasted images stay
   on the disk next to the document. Remote images in a preview are not fetched unless the user
   opts in. Nothing is uploaded, ever, by default (`JARAK-SEC-003`, `KIM-SEC-010…012`).
5. **Keyboard first, mouse friendly.** Every command has a name, a palette entry and a
   configurable key (the Kim hotkey contract); the mouse does what a Windows user expects
   (click, drag, double/triple click, Alt+drag for a column, Ctrl+click for another caret).
6. **Discoverable power.** Advanced features are reachable without memorising: the palette, the
   status bar (encoding, line ending, language, mode are clickable), the context menu and
   Settings. Vim is an editing mode you turn on, with a visible mode indicator.
7. **One design system.** Every surface uses `kim-design` tokens and `kim-ui` components; the
   UI is measured with the same audit pipeline as the other Kim tools
   ([UI_QUALITY_PIPELINE](UI_QUALITY_PIPELINE.md)).
8. **Honest parity.** A Notepad++ feature is `supported` only when Jarak has its own test or
   real-window evidence for that exact feature; "something similar exists" is not support.
   Binary compatibility with Notepad++ plugin DLLs is **not** claimed (§4).
9. **Headless means headless.** `jarak.exe` (the CLI) never opens a window; it inspects,
   searches, converts and exports with `--json` and `--dry-run` per the
   [CLI contract](CLI_CONTRACT.md) (`KIM-FR-012`). The window is `jarak-gui.exe`.
10. **Windows first.** Windows 10/11 is the platform now; the document model, editing engine,
    persistence and Markdown rendering are platform-neutral so Linux can follow later
    (AGENTS.md: no Linux work on this PC).

## 3. Who it is for

- The maintainer and developers who want Notepad++'s power with a calmer, modern look.
- Vim users who want modal editing in a native Windows editor without a terminal.
- Anyone who writes Markdown notes with screenshots: copy a screenshot (for example from
  Noeul), Ctrl+V, done; the note and its images stay together and portable.
- Scripts and agents that need a predictable headless editor CLI (`jarak info`, `find`,
  `replace --dry-run`, `convert`, `export`).

## 4. Non-goals (v1)

- **No Electron and no web view runtime.** Markdown is rendered by Jarak's own native renderer.
- **No Notepad++ plugin DLL compatibility.** Notepad++ plugins are Win32 DLLs written against
  Scintilla's and Notepad++'s message APIs; Jarak is not Scintilla-based and will not load them.
  The most-used plugins' functions are covered by built-in features or by Jarak's own safe
  extension model (Step 108), and the audit says which.
- **No auto-updater, no plugin store downloads.** Kim installs and updates Jarak only when the
  user asks (`KIM-SEC-001`, `KIM-SEC-002`).
- **No IDE.** No language servers, debuggers or build systems in v1; Run commands are explicit,
  shown before they run and never elevated.
- **No cloud sync, accounts or collaboration.**
- **No WYSIWYG Markdown editor.** The source is the document; the preview is a view of it.

## 5. Shape of the product

| Part | What it is |
| --- | --- |
| `jarak-core` | The document model and editing engine: rope buffer, positions and graphemes, selections and carets, transactions and undo, commands, search, encodings and file persistence, the Markdown render model. No GUI toolkit. |
| `jarak.exe` | The headless CLI (`--json`, exit codes, `--dry-run`). Links no GUI toolkit (`cargo xtask deps-check`). |
| `jarak-gui.exe` | The editor window (egui/eframe through `kim-ui`, like the other Kim GUIs): text view, tabs and views, preview, image paste, Vim layer, panels, the Settings window. One process per user session; a second start hands its files to the running window. |

Files: settings in `%APPDATA%\mouv\jarak\`, session and recovery snapshots in
`%LOCALAPPDATA%\mouv\jarak\`. Documents stay wherever the user keeps them.

## 6. License and IP boundary

Jarak is MIT OR Apache-2.0 like the rest of Kim. Notepad++ is described in Jarak's own words for
comparison only; no Notepad++ or Scintilla code, text, icons, themes or configuration files are
copied. Importing a user's own Notepad++ files (user-defined languages, sessions) is a
compatibility feature that reads the user's data; it ships no Notepad++ content. "Notepad++" is
the product of its respective owner; this project is not affiliated with it.
