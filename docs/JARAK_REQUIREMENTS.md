# Jarak Requirements

Status: **current** (Step 84, specification; Step 85 implemented the CLI foundation and the
window shell — `JARAK-FR-001`, `JARAK-FR-002`, the CLI half of `JARAK-FR-003`, `JARAK-SEC-009`,
measured `JARAK-NFR-006` and corrected `JARAK-NFR-003` with measurements, see
[JARAK_ARCHITECTURE §6](JARAK_ARCHITECTURE.md); Step 86 implemented the text model —
`JARAK-FR-010…014` and `JARAK-NFR-008` in `jarak-core`, the edit half of `JARAK-NFR-002`
measured (max 79 µs on 100 MB), [JARAK_ARCHITECTURE §8](JARAK_ARCHITECTURE.md); the keys reach it
in Step 88; each implementing Step updates the rows it delivers) · Owner: mouv · Derived from [JARAK_PHILOSOPHY](JARAK_PHILOSOPHY.md) ·
Decision: [ADR 0102](adr/0102-jarak-fourth-kim-tool.md) · Parity:
[JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)

## 0. How to read this document

- **IDs** are unique and permanent: `JARAK-FR-ddd` (functional), `JARAK-NFR-ddd`
  (non-functional), `JARAK-SEC-ddd` (security, privacy and data safety). IDs are never
  renumbered or reused; a dropped requirement is marked *withdrawn*. `cargo xtask docs-check`
  validates the rows, every reference and the parity matrix.
- **Platform requirements still apply.** Every `KIM-*` requirement in
  [PRODUCT_REQUIREMENTS](PRODUCT_REQUIREMENTS.md) binds Jarak too: no automatic installs or
  updates (`KIM-SEC-001`, `KIM-SEC-002`), headless means headless (`KIM-FR-012`), no account or
  telemetry (`KIM-SEC-010…012`), the hotkey contract (`KIM-FR-040…046`).
- **Priority:** `P0` must ship in its milestone · `P1` should ship · `P2` tracked, not promised.
- **Verify:** `T` automated test · `M` manual or driven check on real Windows · `I` inspection ·
  `D` documentation exists and is accurate.
- **NON-NEGOTIABLE** requirements may not be relaxed by any ADR, setting, default or test
  shortcut.

## 1. Milestones

| Milestone | Goal | Steps | Done when |
| --- | --- | --- | --- |
| **J1 – Foundation** | Philosophy, requirements, matrix; the crates, CLI and window shell; the text model; files and encodings; the editor window | 84–88 | Text can be typed, edited and saved safely in the real window; budgets of §3 measured |
| **J2 – First usable release (MVP)** | Markdown preview and toggle, Ctrl+V image paste with portable assets, image rendering, Vim mode | 89–92 | The MVP journey of §4 passes on real Windows (Step 90), refined by 91 and 92 |
| **J3 – Notepad++ editing parity** | Tabs, views, sessions; search, find in files; multiple carets and columns; line operations; syntax, folding, function list, completion; bookmarks and map; macros; encodings; monitoring and recovery | 93–104 | Each matrix row of these Steps passes its own test or real-window check |
| **J4 – Settings, integrations, extensions** | Settings window, localization, themes, keys; print and export; headless CLI and Kim interop; safe extensions and the plugin audit | 105–108 | Per row |
| **J5 – Hardening, parity and delivery** | Large files, performance, security, accessibility; closing parity gaps; packaging and delivery through Kim | 109–111 | Budgets met, parity report final, Jarak installable from Kim |

Until J5 ships, the Kim catalog lists Jarak as *planned* or *preview* (`KIM-FR-051`).

## 2. Functional requirements

### 2.1 Product and process

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-001 | Jarak is one independently installable package with two binaries: `jarak.exe` (headless CLI, links no GUI toolkit) and `jarak-gui.exe` (the editor window). It needs no Electron, web view runtime, resident Kim process, account, cloud service or other Kim tool. | P0 | J1 | I, T |
| JARAK-FR-002 | One editor process per user session: starting `jarak-gui` again with files hands them to the running window, which opens them and comes to the front; `--new-window` starts a separate process. | P1 | J1 | T, M |
| JARAK-FR-003 | `jarak open <file>[:line[:column]]` and `jarak-gui <file>…` open files (creating a new untitled document when none is given); line and column place the caret. | P0 | J1 | T |
| JARAK-FR-004 | Kim lists Jarak in its catalog and installs, updates and removes it only on explicit request; Kim Manager starts it. | P0 | J5 | T, M |

### 2.2 Text model and editing engine

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-010 | The document is a rope with logarithmic edits and line lookups, independent of any UI toolkit (`jarak-core`). | P0 | J1 | T |
| JARAK-FR-011 | Caret movement, selection extension, Backspace and Delete work on grapheme clusters (Hangul, combining marks, emoji ZWJ sequences, CRLF) and never split one; display columns account for East Asian wide characters. | P0 | J1 | T |
| JARAK-FR-012 | Selections have an anchor and a head; select word, line and all; the selection set holds several ranges for multiple carets. | P0 | J1 | T |
| JARAK-FR-013 | Edits are transactions: undo and redo restore the text and the selections, typed runs are coalesced into one step, and the save point marks the document clean or dirty. | P0 | J1 | T |
| JARAK-FR-014 | Every user-visible action is a named command in one registry (palette, keys, menus, macros and the Vim layer use the same commands). | P0 | J1 | T |

### 2.3 Files, encodings and line endings

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-020 | New, Open, Save, Save As, Save a Copy As, Save All, Close, Close All and the close variants (others, left, right, unchanged), Rename, Move to Recycle Bin and Reload from disk. | P0 | J1 | T, M |
| JARAK-FR-021 | Opening detects UTF-8 (with or without BOM), UTF-16 LE/BE (BOM) and a legacy code page (the system ANSI code page by default) and keeps the detected encoding and BOM on save. | P0 | J1 | T |
| JARAK-FR-022 | Line endings CRLF, LF and CR (and mixed) are detected and preserved on save; new documents use the configured default. | P0 | J1 | T |
| JARAK-FR-023 | Opening and saving without an edit writes byte-identical content. | P0 | J1 | T |
| JARAK-FR-024 | Closing a dirty document or quitting asks Save / Don't save / Cancel; nothing dirty is discarded silently. | P0 | J1 | T, M |
| JARAK-FR-025 | Encoding menu: reinterpret the file in another encoding (no data change) or convert it (data change on save) between UTF-8, UTF-8-BOM, UTF-16 LE/BE BOM, ANSI and the common character sets (Korean, Japanese, Chinese, Cyrillic, Western, Central European, …); a lossy conversion is refused or warned before save. | P1 | J3 | T |
| JARAK-FR-026 | EOL conversion (Windows, Unix, Macintosh classic) and indicators in the status bar. | P1 | J3 | T |
| JARAK-FR-027 | Binary or NUL-containing files are detected and opened read-only (or refused) with an explanation; read-only files and documents marked read-only cannot be modified by accident. | P1 | J1 | T |
| JARAK-FR-028 | Recent files list, reopen the last closed document, empty the list. | P1 | J3 | T |

### 2.4 The editor window

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-030 | A text view that renders only the visible lines with a monospace font and fallbacks for Korean/CJK and emoji: caret, selections, current-line highlight, line-number gutter, horizontal and vertical scrolling, word wrap. | P0 | J1 | T, M |
| JARAK-FR-031 | The standard Windows editing keys (arrows, Home/End, Ctrl+arrows, Page keys, Shift to select, Ctrl+A/C/X/V/Z/Y/S/O/N/W, Tab/Shift+Tab indentation, Enter with auto-indent). | P0 | J1 | T, M |
| JARAK-FR-032 | Mouse: click, drag selection, double-click word, triple-click line, Shift+click, wheel, drag and drop of text and of files to open. | P0 | J1 | T, M |
| JARAK-FR-033 | IME: Korean (and other) composition is shown inline at the caret and committed once. | P0 | J1 | M |
| JARAK-FR-034 | Per-monitor DPI scaling through the kim-ui scale model; zoom in, out and reset of the text. | P0 | J1 | T, M |
| JARAK-FR-035 | No toolbar; a status bar with line:column, selection size, encoding, EOL, language, editing mode and the Settings gear at its right end; encoding, EOL, language and mode are clickable. | P0 | J1 | T, M |
| JARAK-FR-036 | Always on top, full screen, distraction-free mode. | P2 | J3 | T |
| JARAK-FR-037 | Show whitespace, EOL and non-printing characters, indent guides, wrap symbol and a vertical edge line. | P1 | J3 | T |

### 2.5 Markdown and images

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-040 | GitHub-flavoured Markdown (headings, emphasis, lists, task lists, block quotes, code blocks, tables, strikethrough, footnotes, autolinks, rules, images) is parsed and rendered natively, without a web view. | P0 | J2 | T, M |
| JARAK-FR-041 | One key switches between Source and Preview (and an optional Split); the switch keeps the document byte-identical and its dirty state, keeps the caret and maps the scroll position to the nearest block and back. | P0 | J2 | T, M |
| JARAK-FR-042 | Ctrl+V of a clipboard image (bitmap, PNG, an image file) in a Markdown document saves it as a PNG asset in a folder next to the document (`assets/` by default) under a unique name and inserts a relative reference at the caret as one undoable edit. | P0 | J2 | T, M |
| JARAK-FR-043 | Pasting an image into an untitled document asks to save the document first (or keeps the image in a private staging folder until the first save, then moves it and rewrites the reference once); the behaviour is stated in the UI. | P0 | J2 | T, M |
| JARAK-FR-044 | The preview shows local PNG, JPEG, GIF (first frame), BMP and WebP images scaled to the column with their aspect ratio, decoded off the UI thread into a bounded cache; a missing or broken image shows a placeholder with its path. | P0 | J2 | T, M |
| JARAK-FR-045 | Assets: open or reveal an image from its reference, list the document's assets and the orphaned ones, rename an asset and update its references on explicit request; Save As to another folder offers to copy the assets. | P1 | J2 | T |
| JARAK-FR-046 | Links in the preview open only on an explicit action (Ctrl+click or the context menu), never on a plain click or hover. | P0 | J2 | T |

### 2.6 Keyboard, Vim and commands

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-050 | Editing mode setting: Standard (default) or Vim; the current Vim mode is always shown in the status bar and Esc never leaves a Standard user stuck. | P0 | J2 | T, M |
| JARAK-FR-051 | Vim normal mode: motions (`h j k l w b e W B E 0 ^ $ gg G f F t T ; , % { } H M L`), operators (`d c y > < g~ gu gU`) with motions and text objects (`iw aw iW aW i" a" i' a' i( a( i[ a[ i{ a{ it at ip ap`), counts, `.` repeat, `u`/Ctrl+R undo/redo, registers (unnamed, `a`–`z`, `0`, `+` the system clipboard), `p`/`P`, `x`, `r`, `J`, `o`/`O`, `i a I A`, `/` and `?` search with `n`/`N`, `*`/`#`. | P0 | J2 | T |
| JARAK-FR-052 | Vim visual, visual-line and visual-block modes; visual-block hands over to column selection. | P0 | J2 | T |
| JARAK-FR-053 | The `:` command line: `:w`, `:q`, `:wq`, `:x`, `:q!`, `:e`, `:s` and `:%s` with flags, `:noh`, `:set` of common options, `:<line>`. | P1 | J2 | T |
| JARAK-FR-054 | A command palette (Ctrl+Shift+P) lists every command with its current keys and runs it. | P0 | J2 | T, M |

### 2.7 Documents, tabs, views and sessions

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-060 | Tabs with a dirty marker: switch (Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1…9), reorder, close (Ctrl+W, middle click), the tab context menu (close variants, copy path, open the folder). | P0 | J3 | T, M |
| JARAK-FR-061 | Two views side by side (vertical or horizontal): move or clone a document to the other view; a clone edits the same buffer; synchronized scrolling optional. | P1 | J3 | T, M |
| JARAK-FR-062 | The session (open files, active tab, carets, scroll, views, folds) is restored on start; sessions can be saved to and loaded from a file. | P0 | J3 | T |
| JARAK-FR-063 | Folder as Workspace: an optional sidebar showing folders with open-on-click, refresh and filter. | P1 | J3 | T, M |
| JARAK-FR-064 | Document list and window list (sort, activate, close several). | P2 | J3 | T |

### 2.8 Search and replace

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-070 | Find and Replace in the document: Normal, Extended (`\n \r \t \0 \xNN \uNNNN`) and Regular expression modes; match case, whole word, wrap around, direction, in selection, `.` matches newline; Find next/previous, Count, Replace, Replace All as one undo step. | P0 | J3 | T, M |
| JARAK-FR-071 | Mark All with up to five styles and optional bookmarks on the marked lines, clear marks, jump between styled occurrences, copy styled text, smart highlighting of the selected word. | P1 | J3 | T |
| JARAK-FR-072 | Incremental search, search history, select-and-find next/previous, find the word under the caret. | P1 | J3 | T |
| JARAK-FR-073 | Find in Files and Replace in Files: folder, include/exclude filters, recursion, hidden files, every mode and option of `JARAK-FR-070`; a results panel grouped by file with navigation; replacements previewed and confirmed, saved atomically with each file's encoding and EOL, with a batch summary. | P0 | J3 | T, M |
| JARAK-FR-074 | Find All and Replace All in all open documents. | P1 | J3 | T |

### 2.9 Editing power tools

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-080 | Multiple carets and selections: Ctrl+click adds a caret, add the next occurrence, select all occurrences, Esc returns to one caret; every edit command applies to all carets and undo restores them. | P0 | J3 | T, M |
| JARAK-FR-081 | Column (rectangular) selection with Alt+drag and Alt+Shift+arrows aligned by display column, typing/deleting/pasting into each line, virtual space; the Column Editor inserts text or incrementing numbers. | P0 | J3 | T, M |
| JARAK-FR-082 | Line operations: duplicate, delete, move up/down, join, split, insert a blank line above/below, remove duplicate and consecutive duplicate lines, remove empty lines (with or without blanks), reverse and randomize line order. | P0 | J3 | T |
| JARAK-FR-083 | Sort lines lexicographically (case-sensitive or not), by locale, as integers, as decimals (dot or comma), by length, ascending or descending, in the selection or the document. | P1 | J3 | T |
| JARAK-FR-084 | Case conversion: UPPER, lower, Proper, Sentence (also the "blend" variants), iNVERT, random. | P1 | J3 | T |
| JARAK-FR-085 | Blank operations: trim trailing, leading or both, EOL to space, Tab to space, space to Tab (all or leading); increase/decrease indentation. | P1 | J3 | T |
| JARAK-FR-086 | Comment and uncomment per language: toggle line comment, block comment, block uncomment. | P1 | J3 | T |
| JARAK-FR-087 | Insert date and time (short, long, custom format); copy the full path, file name or directory (also of all open documents); a character panel; a clipboard history. | P2 | J3 | T |
| JARAK-FR-088 | Selection tools: Base64, URL and quoted-printable encode/decode, ASCII↔hex, MD5/SHA-1/SHA-256/SHA-512 of the selection or of files to the clipboard. | P2 | J3 | T |

### 2.10 Languages and code assistance

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-090 | Incremental syntax highlighting for at least C, C++, C#, Rust, Go, Java, JavaScript, TypeScript, Python, PowerShell, Batch, shell, HTML, CSS, JSON, YAML, TOML, XML, SQL, Markdown (with fenced code), INI, Lua, PHP, Ruby and Diff; detection by extension, shebang and first line; the Language menu to override. | P0 | J3 | T, M |
| JARAK-FR-091 | User-defined languages (keywords, comments, operators, delimiters, numbers, folding markers) in Jarak's own file format, and import of Notepad++ UDL XML files with the documented limits. | P1 | J3 | T |
| JARAK-FR-092 | Code folding by syntax, indentation and Markdown headings: fold/unfold current, all, by level 1–8; a fold margin. | P1 | J3 | T, M |
| JARAK-FR-093 | Function List / outline: symbols per language and Markdown headings, filtered, with jump. | P1 | J3 | T, M |
| JARAK-FR-094 | Auto-completion from document words and language keywords, function parameter hints, path completion; never steals Enter or Tab without the setting. | P1 | J3 | T |
| JARAK-FR-095 | Snippets with tab stops and placeholders in a user-editable file. | P2 | J3 | T |
| JARAK-FR-096 | Brace and quote matching highlight, auto-close (setting), go to and select to the matching brace, HTML/XML tag matching and auto-close, smart indentation per language. | P1 | J3 | T |

### 2.11 Navigation

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-100 | Bookmarks: toggle, next, previous, clear; cut, copy, delete and replace bookmarked lines, remove unbookmarked lines, inverse bookmarks; a bookmark margin. | P1 | J3 | T |
| JARAK-FR-101 | Change history: modified and saved lines marked in the margin, go to the next/previous change, clear. | P1 | J3 | T |
| JARAK-FR-102 | Document Map (a miniature of the document with the viewport) with click and drag navigation. | P2 | J3 | T, M |
| JARAK-FR-103 | Go to line or offset, navigation history back/forward, quick open of workspace files and open documents. | P1 | J3 | T |
| JARAK-FR-104 | Summary of the document (characters, words, lines, bytes, selection) and Hide Lines. | P2 | J3 | T |

### 2.12 Automation and commands

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-110 | Macros: record and stop, play back, run N times or to the end of the file, save named macros with keys; macros record commands from the registry, replay deterministically and undo as one step per run. | P1 | J3 | T, M |
| JARAK-FR-111 | Run: user-defined external commands with path variables, shown before they run, started only by an explicit action, never elevated, without a shell unless the user wrote one. | P2 | J3 | T |
| JARAK-FR-112 | Every command's keys are configurable through `hotkeys.json` and the Settings window with conflict detection ([HOTKEY_CONTRACT](HOTKEY_CONTRACT.md)). | P1 | J4 | T |

### 2.13 Reliability and recovery

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-120 | An external change to an open file is detected; an unmodified document reloads (setting) or asks; a modified one offers keep mine / reload / compare; a deleted file is kept open as unsaved. | P0 | J3 | T, M |
| JARAK-FR-121 | Dirty documents (including untitled ones) are snapshotted periodically to private recovery files; after a crash Jarak offers to restore them with their content intact; snapshots are removed after a save or an explicit discard. | P0 | J3 | T, M |
| JARAK-FR-122 | Monitoring (tail -f): a document follows a growing file and scrolls to its end. | P2 | J3 | T |

### 2.14 Settings, output, integration, extensions

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-FR-130 | A Settings window of its own, opened from the gear: General, Editor, Appearance, Markdown, Images, Vim, Keyboard, Files, Advanced; every setting persists atomically and applies live. | P0 | J4 | T, M |
| JARAK-FR-131 | English and Korean for every user-visible string; the language follows Windows by default. | P0 | J4 | T, M |
| JARAK-FR-132 | Dark and light themes from `kim-design` with the purple accent; editor font family and size; a style configurator for syntax colours. | P0 | J4 | T, M |
| JARAK-FR-140 | Print through the Windows print dialog with header, footer, line numbers and highlighting. | P2 | J4 | T, M |
| JARAK-FR-141 | Export to HTML (self-contained option with embedded images) and RTF; Markdown to HTML/PDF with its assets into a folder; nothing is uploaded and nothing is overwritten without confirmation. | P1 | J4 | T |
| JARAK-FR-150 | Headless CLI: `info`, `open`, `new`, `find`, `replace --dry-run`, `convert`, `export`, `session`, `capabilities`, each with `--json` and documented exit codes ([CLI_CONTRACT](CLI_CONTRACT.md)). | P0 | J4 | T |
| JARAK-FR-151 | Kim interoperability: open a file at a line from Subak, insert a Noeul capture into a document, Jarak commands as Ddalgak tool actions, Kim capabilities. | P1 | J4 | T, M |
| JARAK-FR-160 | Extensions (languages, themes, snippets, commands; optional sandboxed scripts) with explicit, revocable permissions and no automatic download. | P2 | J4 | T |
| JARAK-FR-161 | A plugin audit states, for the widely used Notepad++ plugins, whether Jarak covers them built in, by an extension, or not at all; Notepad++ plugin DLLs are not loaded. | P1 | J4 | D |
| JARAK-FR-170 | A parity report gives the final status and evidence of every matrix row. | P0 | J5 | D |
| JARAK-FR-171 | A user guide, release notes and an acceptance script with real Windows evidence ship with the first release. | P0 | J5 | D, M |

## 3. Non-functional requirements (budgets measured on the maintainer's PC, release build)

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-NFR-001 | Cold start to the first painted frame with an empty document ≤ 400 ms; a 1 MB file ≤ 600 ms. | P0 | J1 | T |
| JARAK-NFR-002 | Keystroke to painted frame ≤ 16 ms (p95) in a 10 MB document; the edit itself ≤ 1 ms. | P0 | J1 | T |
| JARAK-NFR-003 | Idle: no repaint and ≈ 0 % CPU when nothing changes (caret blink may be turned off; at most two wake-ups per second with it on); working set ≤ 100 MB with an empty document. Step 85: the unmeasured "≤ 80 MB private" was corrected to the measured egui + glow baseline every Kim window shares on this PC (Jarak 114.5 MB private / 87.9 MB working set, Kim Manager 110.1 / 89.6 MB at UI zoom 1); idle CPU 0 ms over 10 s. Step 109 tries to lower it. | P0 | J1 | T |
| JARAK-NFR-004 | A 100 MB file opens in ≤ 3 s with memory ≤ 3× its size; scrolling stays at frame rate; a single 10 MB line does not hang the window (features degrade with a notice). | P1 | J5 | T |
| JARAK-NFR-005 | Source ↔ Preview switch of a 1 MB Markdown document ≤ 100 ms; re-render in split mode debounced and ≤ 50 ms for typical notes. | P0 | J2 | T |
| JARAK-NFR-006 | `jarak-gui.exe` ≤ 15 MB, `jarak.exe` ≤ 8 MB (release, stripped). | P1 | J1 | T |
| JARAK-NFR-007 | Accessibility: every command reachable by keyboard, UI Automation names through AccessKit, design-system contrast, a high-contrast theme, text scaling. | P1 | J5 | T, M |
| JARAK-NFR-008 | Unicode correctness: graphemes, wide characters, combining marks and right-to-left text display without corrupting the buffer. | P0 | J1 | T |
| JARAK-NFR-009 | Replace All of 100 000 matches in a 50 MB document ≤ 3 s and one undo step; Find in Files is cancellable and keeps the window responsive. | P1 | J3 | T |
| JARAK-NFR-010 | Highlighting cost scales with the visible text: scrolling a 10 MB source file stays at frame rate. | P1 | J3 | T |

## 4. Security, privacy and data safety

| ID | Requirement | Pri | MS | Verify |
| --- | --- | --- | --- | --- |
| JARAK-SEC-001 | **NON-NEGOTIABLE.** Jarak writes a user's document only on an explicit save, save-as, replace-in-files confirmation, conversion or asset action by the user; preview, highlighting, folding, indexing, search and session restore never modify a document. | P0 | J1 | T, I |
| JARAK-SEC-002 | **NON-NEGOTIABLE.** A save writes a temporary file in the target folder and replaces the original only after the new content is completely written and flushed; a failure leaves the original untouched and says why. | P0 | J1 | T |
| JARAK-SEC-003 | No network access by default: no telemetry, no update checks of its own, remote images not fetched without opt-in, nothing uploaded. | P0 | J2 | T, I |
| JARAK-SEC-004 | Image decoding is bounded (bytes and pixels) and off the UI thread; decompression bombs and malformed images fail safely. | P0 | J2 | T |
| JARAK-SEC-005 | Pasted assets are written only inside the document's asset folder (no path traversal), never overwrite an existing file and are never deleted without an explicit request. | P0 | J2 | T |
| JARAK-SEC-006 | Links, external commands and "open with" actions run only on an explicit user action, show what will run, are never elevated and never run from a file's content alone. | P0 | J2 | T |
| JARAK-SEC-007 | Extensions run with explicit, revocable permissions (no file system or network access without a grant) and are never downloaded or updated automatically. | P0 | J4 | T |
| JARAK-SEC-008 | Session and recovery files live under `%LOCALAPPDATA%\mouv\jarak` (not roaming), are private to the user and are removed when no longer needed. | P1 | J3 | T |
| JARAK-SEC-009 | `jarak.exe` never opens a window, tray icon or prompt (`KIM-FR-012`); GUI-opening commands are explicit (`jarak open`). | P0 | J1 | T |

## 5. The MVP journey (Step 90)

1. Start Jarak. 2. Create a Markdown document. 3. Type and edit text (English and Korean).
4. Save it. 5. Switch to the preview instantly. 6. Copy an image or screenshot. 7. Paste it with
Ctrl+V. 8. See the image in the preview. 9. Close and reopen the document: text and image intact.
