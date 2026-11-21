# Jarak–Notepad++ Feature Parity Matrix

Status: **current** (inventory, Step 84; every row `planned` or `not intended` — the Step that
implements a row updates it in the same change) · Reference: **Notepad++ 8.9.x** (8.9.7 released
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
| FILE-01 | New untitled document (Ctrl+N) | planned | `JARAK-FR-020` | 87 | T, M | — |
| FILE-02 | Open one or several files with a dialog (Ctrl+O) | planned | `JARAK-FR-020` | 87 | T, M | — |
| FILE-03 | Open files by drag and drop onto the window | planned | `JARAK-FR-032` | 88 | M | — |
| FILE-04 | Open containing folder in Explorer / in a command prompt | planned | `JARAK-FR-060` | 93 | T | — |
| FILE-05 | Open the file in the default viewer (shell association) | planned | `JARAK-FR-060` | 93 | T | — |
| FILE-06 | Open Folder as Workspace | planned | `JARAK-FR-063` | 93 | T, M | — |
| FILE-07 | Reload from disk | planned | `JARAK-FR-020` | 87 | T | — |
| FILE-08 | Save (Ctrl+S) | planned | `JARAK-FR-020` | 87 | T, M | — |
| FILE-09 | Save As | planned | `JARAK-FR-020` | 87 | T, M | — |
| FILE-10 | Save a Copy As (document keeps its path) | planned | `JARAK-FR-020` | 87 | T | — |
| FILE-11 | Save All | planned | `JARAK-FR-020` | 93 | T | — |
| FILE-12 | Rename the file of the document | planned | `JARAK-FR-020` | 93 | T | — |
| FILE-13 | Close (Ctrl+W) | planned | `JARAK-FR-020` | 87 | T, M | — |
| FILE-14 | Close All | planned | `JARAK-FR-020` | 93 | T | — |
| FILE-15 | Close more: all but active, to the left, to the right, all unchanged | planned | `JARAK-FR-020` | 93 | T | — |
| FILE-16 | Move the file to the Recycle Bin | planned | `JARAK-FR-020` | 93 | T | — |
| FILE-17 | Load Session from a file | planned | `JARAK-FR-062` | 93 | T | — |
| FILE-18 | Save Session to a file | planned | `JARAK-FR-062` | 93 | T | — |
| FILE-19 | Print (dialog) | planned | `JARAK-FR-140` | 106 | M | — |
| FILE-20 | Print Now (default printer, no dialog) | planned | `JARAK-FR-140` | 106 | M | — |
| FILE-21 | Recent files list | planned | `JARAK-FR-028` | 93 | T | — |
| FILE-22 | Restore recently closed file (Ctrl+Shift+T) | planned | `JARAK-FR-028` | 93 | T | — |
| FILE-23 | Open all recent files / empty the recent files list | planned | `JARAK-FR-028` | 93 | T | — |
| FILE-24 | Exit with unsaved-change prompts | planned | `JARAK-FR-024` | 87 | T, M | — |
| FILE-25 | Prompt Save / Don't save / Cancel when closing a dirty document | planned | `JARAK-FR-024` | 87 | T, M | — |

### EDIT — Edit menu: clipboard, selection and basic editing (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| EDIT-01 | Undo / Redo (Ctrl+Z, Ctrl+Y) | planned | `JARAK-FR-013` | 86 | T | — |
| EDIT-02 | Cut / Copy / Paste / Delete | planned | `JARAK-FR-031` | 88 | T, M | — |
| EDIT-03 | Select All | planned | `JARAK-FR-012` | 86 | T | — |
| EDIT-04 | Begin/End Select (stream) for keyboard-free range selection | planned | `JARAK-FR-012` | 96 | T | — |
| EDIT-05 | Begin/End Select in column mode | planned | `JARAK-FR-081` | 96 | T | — |
| EDIT-06 | Copy the current line when nothing is selected (setting) | planned | `JARAK-FR-031` | 97 | T | — |
| EDIT-07 | Insert date and time: short, long, customised format | planned | `JARAK-FR-087` | 97 | T | — |
| EDIT-08 | Copy to clipboard: full path, file name, directory of the document | planned | `JARAK-FR-087` | 97 | T | — |
| EDIT-09 | Copy to clipboard: names and paths of all open documents | planned | `JARAK-FR-087` | 97 | T | — |
| EDIT-10 | Increase / decrease line indent (Tab, Shift+Tab) | planned | `JARAK-FR-085` | 88 | T | — |
| EDIT-11 | Set read-only for the document / clear the file's read-only attribute | planned | `JARAK-FR-027` | 87 | T | — |
| EDIT-12 | Paste special: copy/cut/paste binary content (with NUL) | planned | `JARAK-FR-027` | 103 | T | — |
| EDIT-13 | Paste HTML / RTF clipboard content as source | planned | `JARAK-FR-087` | 110 | T | — |
| EDIT-14 | On selection: open the selected path as a file / its folder | planned | `JARAK-FR-103` | 101 | T | — |
| EDIT-15 | On selection: search on the internet (explicit, configurable engine) | planned | `JARAK-FR-111` | 102 | T | — |
| EDIT-16 | Character panel (insert a character by code) | planned | `JARAK-FR-087` | 110 | T | — |
| EDIT-17 | Clipboard history panel | planned | `JARAK-FR-087` | 110 | T | — |
| EDIT-18 | Drag and drop of selected text inside the document | planned | `JARAK-FR-032` | 88 | M | — |

### CASE — Edit → Convert Case to (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CASE-01 | UPPERCASE | planned | `JARAK-FR-084` | 97 | T | — |
| CASE-02 | lowercase | planned | `JARAK-FR-084` | 97 | T | — |
| CASE-03 | Proper Case and Proper Case (blend) | planned | `JARAK-FR-084` | 97 | T | — |
| CASE-04 | Sentence case and Sentence case (blend) | planned | `JARAK-FR-084` | 97 | T | — |
| CASE-05 | iNVERT cASE | planned | `JARAK-FR-084` | 97 | T | — |
| CASE-06 | ranDOm CasE | planned | `JARAK-FR-084` | 97 | T | — |

### LINE — Edit → Line Operations (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LINE-01 | Duplicate current line / selection (Ctrl+D) | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-02 | Delete current line | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-03 | Remove duplicate lines | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-04 | Remove consecutive duplicate lines | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-05 | Split lines at the edge or a width | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-06 | Join lines | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-07 | Move line up / down | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-08 | Remove empty lines | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-09 | Remove empty lines (containing blank characters) | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-10 | Insert blank line above / below current | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-11 | Reverse line order | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-12 | Randomize line order | planned | `JARAK-FR-082` | 97 | T | — |
| LINE-13 | Sort lines lexicographically ascending / descending | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-14 | Sort lines lexicographically ignoring case | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-15 | Sort lines in locale order | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-16 | Sort lines as integers ascending / descending | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-17 | Sort lines as decimals (comma / dot) ascending / descending | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-18 | Sort lines by length | planned | `JARAK-FR-083` | 97 | T | — |
| LINE-19 | Sort only the lines or the column of the selection | planned | `JARAK-FR-083` | 97 | T | — |

### CMT — Edit → Comment/Uncomment (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CMT-01 | Toggle single-line comment (Ctrl+Q) | planned | `JARAK-FR-086` | 98 | T | — |
| CMT-02 | Single-line comment / uncomment | planned | `JARAK-FR-086` | 98 | T | — |
| CMT-03 | Block comment / block uncomment | planned | `JARAK-FR-086` | 98 | T | — |

### BLANK — Edit → Blank Operations and EOL (sources U, E)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| BLANK-01 | Trim trailing space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-02 | Trim leading space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-03 | Trim leading and trailing space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-04 | EOL to space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-05 | Trim both and EOL to space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-06 | TAB to space | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-07 | Space to TAB (all / leading) | planned | `JARAK-FR-085` | 97 | T | — |
| BLANK-08 | EOL conversion: Windows (CR LF), Unix (LF), Macintosh (CR) | planned | `JARAK-FR-026` | 103 | T | — |

### MULTI — Multi-editing, column mode and the Column Editor (sources E, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MULTI-01 | Ctrl+click adds a caret / selection (multi-editing) | planned | `JARAK-FR-080` | 96 | T, M | — |
| MULTI-02 | Multi-select all occurrences (match case / whole word options) | planned | `JARAK-FR-080` | 96 | T | — |
| MULTI-03 | Multi-select next occurrence / skip current | planned | `JARAK-FR-080` | 96 | T | — |
| MULTI-04 | Column selection with Alt+drag | planned | `JARAK-FR-081` | 96 | M | — |
| MULTI-05 | Column selection with Alt+Shift+arrows | planned | `JARAK-FR-081` | 96 | T | — |
| MULTI-06 | Typing, deleting and pasting into every row of a column selection | planned | `JARAK-FR-081` | 96 | T | — |
| MULTI-07 | Column Editor: insert text into each row | planned | `JARAK-FR-081` | 96 | T | — |
| MULTI-08 | Column Editor: insert incrementing numbers (initial, increase, repeat, leading, base) | planned | `JARAK-FR-081` | 96 | T | — |
| MULTI-09 | Convert a column selection to a multi-selection (setting) | planned | `JARAK-FR-081` | 96 | T | — |
| MULTI-10 | Virtual space past line ends (setting) | planned | `JARAK-FR-081` | 96 | T | — |

### AUTO — Auto-completion and editing assistance (sources A, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| AUTO-01 | Word completion from the document | planned | `JARAK-FR-094` | 100 | T | — |
| AUTO-02 | Function / keyword completion from language definitions | planned | `JARAK-FR-094` | 100 | T | — |
| AUTO-03 | Function parameters hint (and previous/next hint) | planned | `JARAK-FR-094` | 100 | T | — |
| AUTO-04 | Path completion | planned | `JARAK-FR-094` | 100 | T | — |
| AUTO-05 | Auto-insert closing brackets and quotes (setting) | planned | `JARAK-FR-096` | 100 | T | — |
| AUTO-06 | HTML/XML closing tag auto-insertion | planned | `JARAK-FR-096` | 100 | T | — |
| AUTO-07 | Auto-indent (keep / smart per language) | planned | `JARAK-FR-096` | 100 | T | — |
| AUTO-08 | Brace highlighting | planned | `JARAK-FR-096` | 100 | T | — |
| AUTO-09 | HTML/XML tag matching highlight | planned | `JARAK-FR-096` | 100 | T | — |
| AUTO-10 | Smart highlighting of the selected word (with options) | planned | `JARAK-FR-071` | 94 | T | — |

### SRCH — Search menu: find and replace in the document (sources S, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| SRCH-01 | Find dialog (Ctrl+F) | planned | `JARAK-FR-070` | 94 | T, M | — |
| SRCH-02 | Replace dialog (Ctrl+H) | planned | `JARAK-FR-070` | 94 | T, M | — |
| SRCH-03 | Search mode Normal | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-04 | Search mode Extended (\n \r \t \0 \x \u escapes) | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-05 | Search mode Regular expression (and '. matches newline') | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-06 | Options: match case, whole word, wrap around, backward direction | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-07 | In selection | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-08 | Find Next / Find Previous (F3, Shift+F3) | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-09 | Select and Find Next / Previous (Ctrl+F3) | planned | `JARAK-FR-072` | 94 | T | — |
| SRCH-10 | Find (volatile) next / previous of the word under the caret | planned | `JARAK-FR-072` | 94 | T | — |
| SRCH-11 | Count matches | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-12 | Replace / Replace All (one undo step) | planned | `JARAK-FR-070` | 94 | T | — |
| SRCH-13 | Find All in current document (results panel) | planned | `JARAK-FR-073` | 95 | T | — |
| SRCH-14 | Find All / Replace All in all opened documents | planned | `JARAK-FR-074` | 95 | T | — |
| SRCH-15 | Incremental search bar (Ctrl+Alt+I) | planned | `JARAK-FR-072` | 94 | T | — |
| SRCH-16 | Search history (find/replace combo history) | planned | `JARAK-FR-072` | 94 | T | — |
| SRCH-17 | Find dialog transparency on losing focus | planned | `JARAK-FR-070` | 110 | M | — |
| SRCH-18 | Go to line / offset (Ctrl+G) | planned | `JARAK-FR-103` | 99 | T | — |
| SRCH-19 | Go to matching brace / select all between matching braces | planned | `JARAK-FR-096` | 99 | T | — |
| SRCH-20 | Find characters in range (e.g. non-ASCII) | planned | `JARAK-FR-070` | 110 | T | — |

### MARK — Search → Mark, style tokens and bookmarks (sources S, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MARK-01 | Mark All with the Mark dialog (optionally bookmark the lines) | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-02 | Purge marks before each search / clear all marks | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-03 | Copy marked text | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-04 | Style all occurrences of the token (five styles) | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-05 | Style one token | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-06 | Clear a style / all styles | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-07 | Jump up / down to the next styled occurrence | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-08 | Copy styled text | planned | `JARAK-FR-071` | 94 | T | — |
| MARK-09 | Toggle bookmark (Ctrl+F2) | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-10 | Next / previous bookmark (F2, Shift+F2) | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-11 | Clear all bookmarks | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-12 | Cut / copy / paste-replace / remove bookmarked lines | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-13 | Remove unmarked lines | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-14 | Inverse bookmarks | planned | `JARAK-FR-100` | 101 | T | — |
| MARK-15 | Change history: go to next / previous change, clear history | planned | `JARAK-FR-101` | 101 | T | — |

### FIF — Find in Files and the search results window (sources S)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FIF-01 | Find in Files: directory, filters, recursion, hidden folders | planned | `JARAK-FR-073` | 95 | T | — |
| FIF-02 | Filters with exclusions (!pattern, !+\folder) | planned | `JARAK-FR-073` | 95 | T | — |
| FIF-03 | Follow the current document's folder | planned | `JARAK-FR-073` | 95 | T | — |
| FIF-04 | Replace in Files (with confirmation) | planned | `JARAK-FR-073` | 95 | T, M | — |
| FIF-05 | Search results window grouped by file, double-click to open | planned | `JARAK-FR-073` | 95 | T, M | — |
| FIF-06 | Next / previous search result (F4, Shift+F4) | planned | `JARAK-FR-073` | 95 | T | — |
| FIF-07 | Copy selected / all result lines and paths | planned | `JARAK-FR-073` | 95 | T | — |
| FIF-08 | Find in Projects (project panels) | not intended | — | 110 | D | — |

### VIEW — View menu: window, symbols, zoom and wrap (sources V, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| VIEW-01 | Always on top | planned | `JARAK-FR-036` | 110 | T | — |
| VIEW-02 | Toggle full screen (F11) | planned | `JARAK-FR-036` | 110 | M | — |
| VIEW-03 | Post-it mode (F12: borderless window) | planned | `JARAK-FR-036` | 110 | M | — |
| VIEW-04 | Distraction-free mode | planned | `JARAK-FR-036` | 110 | M | — |
| VIEW-05 | View the current file in a browser (explicit) | planned | `JARAK-FR-111` | 102 | T | — |
| VIEW-06 | Show spaces and tabs | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-07 | Show end of line | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-08 | Show non-printing and control characters, Unicode EOL | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-09 | Show all characters | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-10 | Show indent guide | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-11 | Show wrap symbol | planned | `JARAK-FR-037` | 103 | T | — |
| VIEW-12 | Zoom in / out / restore (Ctrl+wheel, Ctrl+Num+/-/) | planned | `JARAK-FR-034` | 88 | T | — |
| VIEW-13 | Word wrap | planned | `JARAK-FR-030` | 88 | T | — |
| VIEW-14 | Hide lines | planned | `JARAK-FR-104` | 101 | T | — |
| VIEW-15 | Summary (characters, words, lines, bytes) | planned | `JARAK-FR-104` | 101 | T | — |
| VIEW-16 | Text direction right-to-left / left-to-right | planned | `JARAK-NFR-008` | 110 | M | — |
| VIEW-17 | Monitoring (tail -f) | planned | `JARAK-FR-122` | 104 | T | — |

### VIEWS — View → documents, views and panels (sources V, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| VIEWS-01 | Move current document to the other view | planned | `JARAK-FR-061` | 93 | T, M | — |
| VIEWS-02 | Clone current document to the other view | planned | `JARAK-FR-061` | 93 | T, M | — |
| VIEWS-03 | Move / open the document in a new instance | planned | `JARAK-FR-002` | 93 | T | — |
| VIEWS-04 | Focus on another view (F8) | planned | `JARAK-FR-061` | 93 | T | — |
| VIEWS-05 | Synchronize vertical / horizontal scrolling | planned | `JARAK-FR-061` | 93 | T | — |
| VIEWS-06 | Go to tab 1–9, next / previous tab | planned | `JARAK-FR-060` | 93 | T | — |
| VIEWS-07 | Move tab forward / backward | planned | `JARAK-FR-060` | 93 | T | — |
| VIEWS-08 | Tab colouring | planned | `JARAK-FR-060` | 110 | T | — |
| VIEWS-09 | Document Map panel | planned | `JARAK-FR-102` | 101 | T, M | — |
| VIEWS-10 | Document List panel | planned | `JARAK-FR-064` | 93 | T | — |
| VIEWS-11 | Function List panel | planned | `JARAK-FR-093` | 99 | T, M | — |
| VIEWS-12 | Folder as Workspace panel | planned | `JARAK-FR-063` | 93 | T, M | — |
| VIEWS-13 | Project panels (1–3, .workspace files) | not intended | — | 110 | D | — |

### FOLD — View → folding (sources V)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FOLD-01 | Fold all / unfold all | planned | `JARAK-FR-092` | 99 | T | — |
| FOLD-02 | Collapse / uncollapse current level | planned | `JARAK-FR-092` | 99 | T | — |
| FOLD-03 | Collapse / uncollapse level 1–8 | planned | `JARAK-FR-092` | 99 | T | — |
| FOLD-04 | Fold margin with mouse folding | planned | `JARAK-FR-092` | 99 | M | — |
| FOLD-05 | Folding by syntax for built-in languages | planned | `JARAK-FR-092` | 99 | T | — |
| FOLD-06 | Folding of user-defined languages by markers | planned | `JARAK-FR-092` | 99 | T | — |

### ENC — Encoding menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ENC-01 | Interpret as ANSI (system code page) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-02 | Interpret as UTF-8 | planned | `JARAK-FR-021` | 87 | T | — |
| ENC-03 | Interpret as UTF-8-BOM | planned | `JARAK-FR-021` | 87 | T | — |
| ENC-04 | Interpret as UTF-16 BE BOM / UTF-16 LE BOM | planned | `JARAK-FR-021` | 87 | T | — |
| ENC-05 | Character sets: Korean (EUC-KR / Windows-949) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-06 | Character sets: Japanese (Shift-JIS, EUC-JP) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-07 | Character sets: Chinese (GB2312/GBK, Big5) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-08 | Character sets: Cyrillic (Windows-1251, KOI8-R, KOI8-U, ISO 8859-5) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-09 | Character sets: Western and Central European (Windows-1252/1250, ISO 8859-1/2/15) | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-10 | Character sets: Greek, Turkish, Hebrew, Arabic, Baltic, Vietnamese, Thai | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-11 | Convert to ANSI | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-12 | Convert to UTF-8 / UTF-8-BOM | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-13 | Convert to UTF-16 BE BOM / UTF-16 LE BOM | planned | `JARAK-FR-025` | 103 | T | — |
| ENC-14 | Automatic encoding detection on open | planned | `JARAK-FR-021` | 87 | T | — |
| ENC-15 | Default encoding for new documents (setting) | planned | `JARAK-FR-021` | 87 | T | — |
| ENC-16 | Status-bar encoding and EOL indicators (clickable) | planned | `JARAK-FR-035` | 88 | T, M | — |

### LANG — Language menu and syntax highlighting (sources L, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LANG-01 | Built-in language highlighting (the JARAK-FR-090 list) | planned | `JARAK-FR-090` | 98 | T | — |
| LANG-02 | Further Notepad++ languages (Ada, ASM, AutoIt, COBOL, D, Fortran, Haskell, Lisp, Pascal, Perl, R, Scheme, Swift, VHDL, Verilog, …) | planned | `JARAK-FR-090` | 110 | T | — |
| LANG-03 | Language detection by extension | planned | `JARAK-FR-090` | 98 | T | — |
| LANG-04 | Language detection by first line / shebang | planned | `JARAK-FR-090` | 98 | T | — |
| LANG-05 | Manual language selection (Language menu) | planned | `JARAK-FR-090` | 98 | T | — |
| LANG-06 | Normal text (no highlighting) | planned | `JARAK-FR-090` | 98 | T | — |
| LANG-07 | User Defined Language: define keywords, comments, operators, delimiters, numbers | planned | `JARAK-FR-091` | 98 | T | — |
| LANG-08 | User Defined Language: folding markers | planned | `JARAK-FR-091` | 98 | T | — |
| LANG-09 | Import / export of user-defined languages | planned | `JARAK-FR-091` | 98 | T | — |
| LANG-10 | Import Notepad++ UDL XML files | planned | `JARAK-FR-091` | 98 | T | — |
| LANG-11 | Embedded languages (script and style inside HTML, PHP) | planned | `JARAK-FR-090` | 98 | T | — |

### SET — Settings menu (sources R, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| SET-01 | Preferences: General (localization, status bar, menu) | planned | `JARAK-FR-130` | 105 | T, M | — |
| SET-02 | Preferences: Tab bar options (close button, double-click to close, multi-line) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-03 | Preferences: Editing (caret, current line, line wrap, multi-editing, scrolling beyond last line) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-04 | Preferences: Dark mode | planned | `JARAK-FR-132` | 105 | T, M | — |
| SET-05 | Preferences: Margins, border, edge (line numbers, bookmark margin, edge column) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-06 | Preferences: New document (format, encoding, default language) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-07 | Preferences: Default directory | planned | `JARAK-FR-130` | 105 | T | — |
| SET-08 | Preferences: Recent files history size | planned | `JARAK-FR-130` | 105 | T | — |
| SET-09 | Preferences: File association (explicit, reversible) | planned | `JARAK-FR-130` | 111 | M | — |
| SET-10 | Preferences: Language (tab size per language, replace by space) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-11 | Preferences: Highlighting (smart highlight, mark-all options) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-12 | Preferences: Print (margins, header, footer, colour mode) | planned | `JARAK-FR-140` | 106 | T | — |
| SET-13 | Preferences: Searching (minimum size, fill find field, confirm replace all) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-14 | Preferences: Backup (session snapshot, periodic backup, backup on save) | planned | `JARAK-FR-121` | 104 | T | — |
| SET-15 | Preferences: Auto-completion | planned | `JARAK-FR-094` | 105 | T | — |
| SET-16 | Preferences: Multi-instance and date format | planned | `JARAK-FR-002` | 105 | T | — |
| SET-17 | Preferences: Delimiter (word characters, double-click delimiters) | planned | `JARAK-FR-130` | 105 | T | — |
| SET-18 | Preferences: Performance (large file restriction) | planned | `JARAK-NFR-004` | 109 | T | — |
| SET-19 | Preferences: Cloud settings path | not intended | — | 105 | D | — |
| SET-20 | Preferences: Search engine for 'search on internet' | planned | `JARAK-FR-111` | 105 | T | — |
| SET-21 | Preferences: MISC (file status auto-detection, document peeker, smooth font, clickable links) | planned | `JARAK-FR-120` | 105 | T | — |
| SET-22 | Style Configurator (themes, per-language token colours, global font) | planned | `JARAK-FR-132` | 105 | T, M | — |
| SET-23 | Shortcut Mapper (main menu, macros, run commands, editor commands; conflicts) | planned | `JARAK-FR-112` | 105 | T, M | — |
| SET-24 | Import style themes | planned | `JARAK-FR-132` | 105 | T | — |
| SET-25 | Edit the context menu | planned | `JARAK-FR-130` | 110 | T | — |
| SET-26 | Localization (Korean, English) | planned | `JARAK-FR-131` | 105 | T, M | — |

### TOOL — Tools menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| TOOL-01 | MD5: generate, from files, from selection to clipboard | planned | `JARAK-FR-088` | 97 | T | — |
| TOOL-02 | SHA-1: generate, from files, from selection | planned | `JARAK-FR-088` | 97 | T | — |
| TOOL-03 | SHA-256: generate, from files, from selection | planned | `JARAK-FR-088` | 97 | T | — |
| TOOL-04 | SHA-512: generate, from files, from selection | planned | `JARAK-FR-088` | 97 | T | — |

### MAC — Macro menu (sources M)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| MAC-01 | Start / stop recording (Ctrl+Shift+R) | planned | `JARAK-FR-110` | 102 | T, M | — |
| MAC-02 | Playback (Ctrl+Shift+P in Notepad++; Jarak keys configurable) | planned | `JARAK-FR-110` | 102 | T | — |
| MAC-03 | Save the recorded macro with a name and shortcut | planned | `JARAK-FR-110` | 102 | T | — |
| MAC-04 | Run a macro multiple times / until the end of the file | planned | `JARAK-FR-110` | 102 | T | — |
| MAC-05 | Built-in 'Trim trailing space and save' macro | planned | `JARAK-FR-110` | 102 | T | — |
| MAC-06 | Modify shortcut / delete macro | planned | `JARAK-FR-110` | 102 | T | — |
| MAC-07 | Search and replace actions are macro-recordable | planned | `JARAK-FR-110` | 102 | T | — |

### RUN — Run menu (sources X)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| RUN-01 | Run… a command with variables ($(FULL_CURRENT_PATH), $(CURRENT_WORD), …) | planned | `JARAK-FR-111` | 102 | T | — |
| RUN-02 | Save run commands with a name and shortcut | planned | `JARAK-FR-111` | 102 | T | — |
| RUN-03 | Modify shortcut / delete command | planned | `JARAK-FR-111` | 102 | T | — |
| RUN-04 | Launch in browsers / Wikipedia / PHP help entries | planned | `JARAK-FR-111` | 102 | T | — |
| RUN-05 | Open the file in another instance | planned | `JARAK-FR-002` | 93 | T | — |

### PLUG — Plugins menu and bundled plugin functions (sources G)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| PLUG-01 | Load third-party Notepad++ plugin DLLs | not intended | — | 108 | D | — |
| PLUG-02 | Plugins Admin (download and update plugins from a list) | not intended | — | 108 | D | — |
| PLUG-03 | MIME Tools: Base64 encode / decode (also with padding / URL-safe) | planned | `JARAK-FR-088` | 97 | T | — |
| PLUG-04 | MIME Tools: URL encode / decode | planned | `JARAK-FR-088` | 97 | T | — |
| PLUG-05 | MIME Tools: quoted-printable encode / decode | planned | `JARAK-FR-088` | 97 | T | — |
| PLUG-06 | Converter: ASCII to hex / hex to ASCII | planned | `JARAK-FR-088` | 97 | T | — |
| PLUG-07 | NppExport: export to RTF / HTML, copy as RTF / HTML | planned | `JARAK-FR-141` | 106 | T | — |
| PLUG-08 | Safe extension mechanism (Jarak's own) | planned | `JARAK-FR-160` | 108 | T | — |
| PLUG-09 | Plugin compatibility audit document | planned | `JARAK-FR-161` | 108 | D | — |

### WIN — Window menu (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| WIN-01 | Window list of open documents (activate) | planned | `JARAK-FR-064` | 93 | T | — |
| WIN-02 | Windows… dialog: sort, activate, save, close several | planned | `JARAK-FR-064` | 93 | T | — |

### HELP — ? menu and command line (sources C, U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| HELP-01 | Command line: open files | planned | `JARAK-FR-003` | 85 | T | — |
| HELP-02 | Command line: -n line / -c column / -p position | planned | `JARAK-FR-003` | 85 | T | — |
| HELP-03 | Command line: -l language / -udl | planned | `JARAK-FR-150` | 107 | T | — |
| HELP-04 | Command line: -ro read-only | planned | `JARAK-FR-150` | 107 | T | — |
| HELP-05 | Command line: -nosession / -openSession | planned | `JARAK-FR-150` | 107 | T | — |
| HELP-06 | Command line: -multiInst (separate instance) | planned | `JARAK-FR-002` | 85 | T | — |
| HELP-07 | Command line: -x / -y window position, -alwaysOnTop | planned | `JARAK-FR-150` | 110 | T | — |
| HELP-08 | Command line: -qt / -qf ghost typing | not intended | — | 107 | D | — |
| HELP-09 | Built-in updater (WinGUp) | not intended | — | 111 | D | — |
| HELP-10 | Debug info (versions, paths) | planned | `JARAK-FR-150` | 107 | T | — |
| HELP-11 | Help / about / user manual link | planned | `JARAK-FR-171` | 111 | D | — |

### REL — Sessions, backup and file status (sources U, P)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| REL-01 | Remember the session at exit and restore it at start | planned | `JARAK-FR-062` | 93 | T | — |
| REL-02 | Session snapshot and periodic backup of dirty documents | planned | `JARAK-FR-121` | 104 | T, M | — |
| REL-03 | File status auto-detection with reload prompt | planned | `JARAK-FR-120` | 104 | T, M | — |
| REL-04 | Update silently (auto-reload unmodified documents) | planned | `JARAK-FR-120` | 104 | T | — |
| REL-05 | Deleted file kept as an unsaved document | planned | `JARAK-FR-120` | 104 | T | — |
| REL-06 | Large-file mode (features turned off above a size) | planned | `JARAK-NFR-004` | 109 | T | — |

### UI — General user interface (sources U)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| UI-01 | Tab bar with dirty markers, close buttons and context menu | planned | `JARAK-FR-060` | 93 | T, M | — |
| UI-02 | Status bar: language, length/lines, position, EOL, encoding, insert/overwrite | planned | `JARAK-FR-035` | 88 | T, M | — |
| UI-03 | Overwrite mode (Insert key) | planned | `JARAK-FR-031` | 88 | T | — |
| UI-04 | Line number margin | planned | `JARAK-FR-030` | 88 | T, M | — |
| UI-05 | Current line highlight | planned | `JARAK-FR-030` | 88 | T | — |
| UI-06 | Vertical edge line(s) at configured columns | planned | `JARAK-FR-037` | 103 | T | — |
| UI-07 | Clickable links (Ctrl+click) | planned | `JARAK-FR-046` | 89 | T | — |
| UI-08 | Context menu in the text | planned | `JARAK-FR-031` | 88 | M | — |
| UI-09 | Toolbar with icons | not intended | — | 88 | D | — |
| UI-10 | Menu bar (Jarak: hidden by default, shown with Alt or from the palette) | planned | `JARAK-FR-054` | 92 | T | — |

## 2. Jarak additions (not Notepad++ features)

| ID | Feature (described in Jarak's words) | Status | Requirement | Step | Verify | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| JRK-01 | Markdown: GitHub-flavoured preview rendered natively | planned | `JARAK-FR-040` | 89 | T, M | — |
| JRK-02 | Markdown: instant Source / Preview / Split toggle keeping caret and scroll | planned | `JARAK-FR-041` | 89 | T, M | — |
| JRK-03 | Markdown: Ctrl+V image paste into portable assets with a relative reference | planned | `JARAK-FR-042` | 90 | T, M | — |
| JRK-04 | Markdown: image paste into an untitled document (explicit behaviour) | planned | `JARAK-FR-043` | 90 | T, M | — |
| JRK-05 | Markdown: inline image rendering, scaling, cache, placeholders | planned | `JARAK-FR-044` | 91 | T, M | — |
| JRK-06 | Markdown: asset navigation, orphans, rename with reference update | planned | `JARAK-FR-045` | 91 | T | — |
| JRK-07 | Vim: editing mode with a visible mode indicator | planned | `JARAK-FR-050` | 92 | T, M | — |
| JRK-08 | Vim: motions, operators, text objects, counts, repeat, registers | planned | `JARAK-FR-051` | 92 | T | — |
| JRK-09 | Vim: visual, visual-line, visual-block | planned | `JARAK-FR-052` | 92 | T | — |
| JRK-10 | Vim: : command line | planned | `JARAK-FR-053` | 92 | T | — |
| JRK-11 | Command palette listing every command with its keys | planned | `JARAK-FR-054` | 92 | T, M | — |
| JRK-12 | Snippets with tab stops | planned | `JARAK-FR-095` | 100 | T | — |
| JRK-13 | Quick open of files (Ctrl+P) and navigation history | planned | `JARAK-FR-103` | 101 | T | — |
| JRK-14 | Headless CLI with --json (info, find, replace --dry-run, convert, export) | planned | `JARAK-FR-150` | 107 | T | — |
| JRK-15 | Kim interoperability (Subak, Noeul, Ddalgak) | planned | `JARAK-FR-151` | 107 | T, M | — |
| JRK-16 | Recovery of untitled documents after a crash | planned | `JARAK-FR-121` | 104 | T, M | — |

## 3. Counts

| | Rows |
| --- | --- |
| Notepad++ built-in features | 285 |
| … `planned` | 277 |
| … `not intended` | 8 |
| Jarak additions | 16 |

## 4. Not intended, and why

- **Notepad++ plugin DLLs and Plugins Admin** (`PLUG-01`, `PLUG-02`): plugins are native DLLs
  bound to Scintilla's and Notepad++'s window-message APIs; Jarak is not built on Scintilla and
  does not run foreign native code inside its process. Plugin downloads would also contradict
  `KIM-SEC-001`. The common plugin functions are covered by built-in rows or by Jarak's own
  extensions (Step 108 audit).
- **Built-in updater** (`HELP-09`): Kim installs and updates tools only when the user asks
  (`KIM-SEC-001`, `KIM-SEC-002`).
- **Project panels and Find in Projects** (`VIEWS-13`, `FIF-08`): Folder as Workspace and Find in
  Files cover the use; `.workspace` project trees are not planned for v1 (revisited in Step 110).
- **Cloud settings path** (`SET-19`): no cloud features (`JARAK-SEC-003`); the settings folder is
  a plain folder the user may sync with tools of their choice.
- **Ghost typing** (`HELP-08`): typing a file's content into the editor as a demo is not an
  editing feature.
- **Toolbar** (`UI-09`): a deliberate design decision ([JARAK_PHILOSOPHY §1](JARAK_PHILOSOPHY.md));
  every toolbar command is in the palette, the menus and the keys.
