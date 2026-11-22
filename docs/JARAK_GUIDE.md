# Jarak — user guide

Status: **current** (Step 111) · Applies to: Jarak 0.0.x on Windows 10 and 11 · Reference:
[JARAK_KEYS](JARAK_KEYS.md), [JARAK_SETTINGS](JARAK_SETTINGS.md), [JARAK_CLI](JARAK_CLI.md) ·
Parity with Notepad++: [JARAK_PARITY_REPORT](JARAK_PARITY_REPORT.md)

Jarak (자락) is a native Windows text editor for plain text, code and Markdown. It opens in a
moment, keeps your files exactly as they are unless you change them, and works without an
account, a network or a background service. This guide covers the everyday tasks; the reference
documents linked above list every key, setting and command.

## 1. Install, start, remove

- **Install**: in Kim Manager choose **Jarak** → *Install*, or run `kim install jarak`. Nothing
  is installed or updated unless you ask (`KIM-SEC-001`).
- **Start**: from Kim Manager (the Jarak tile, *Open Jarak*) or `jarak-gui` in a terminal.
  `jarak-gui notes.md:12` opens a file at line 12. A second start hands its files to the running
  window (`--new-window` opens one of its own).
- **Open .txt or .md files with Jarak**: Help → *Offer Jarak in "Open with"…* adds Jarak to
  Windows' *Open with* list for the extensions you check, for your account only. It never changes
  your default program: you pick that in Windows (*Settings → Apps → Default apps*). Help →
  *Remove Jarak from "Open with"* takes back exactly what was added (also `jarak associate
  add md txt` and `jarak associate remove`; programs and scripts such as `.exe` or `.ps1` are
  never offered).
- **Remove**: Kim Manager → Jarak → *Uninstall* (or `kim uninstall jarak`). Your settings and
  sessions stay in `%APPDATA%\mouv\jarak` and `%LOCALAPPDATA%\mouv\jarak` until you delete them
  (`jarak paths` shows where).

## 2. The window

- There is no toolbar. Every command is in the **command palette** (Ctrl+Shift+P), with its keys,
  and in the **menu bar**, which Alt shows and hides.
- The **status bar** shows the caret's line and column, the selection, the document's length and
  lines, the encoding, the line endings, the language and INS/OVR. Clicking an item changes it,
  for example the encoding or the language. The gear opens **Settings** (Ctrl+,).
- **Tabs and two views**: Ctrl+Tab switches documents. A tab's context menu moves or clones it to
  the other view; F8 moves the focus between views.
- **Window modes** (View): Always on top, F11 full screen, F12 post-it (no frame or bars), and
  distraction-free (the text in a centred column). The palette works in every mode.
- **Languages**: Korean and English follow Windows; Settings → General can fix one.

## 3. Opening, saving, recovering

- Ctrl+O opens one or several files (Ctrl+click or Shift+click in the dialog's list), Ctrl+S saves, Ctrl+Shift+S saves as, and *Save a copy as…* writes a copy
  without changing the document's file. Saving is atomic: the old file stays whole until the new
  one is written.
- **Encodings** are detected (byte-order mark, UTF-8, UTF-16, then your Windows code page).
  A file Jarak cannot write back exactly opens **read-only**, with the reason in the status bar.
  The Encoding menu reinterprets or converts a document, and asks before anything lossy.
- **Changes made by other programs** are noticed. Jarak asks before reloading; *Compare* shows
  both versions.
- **Unsaved work is kept**: recovery snapshots are written every 10 seconds. After a crash, the
  next start offers to restore them, and nothing is overwritten without asking.
- **The session**: your documents, carets and views come back at the next start (Settings →
  General). `--no-session` starts clean.
- **Backups** (Settings → Files → Recovery): *Backup on save* keeps the previous file as
  `<name>.bak`, or as dated copies in `jarak-backup` beside it.

## 4. Editing

- Notepad++'s keys work: Ctrl+D duplicates a line, Ctrl+Shift+L deletes it, Ctrl+Shift+Up/Down moves
  it, Ctrl+Q comments, Ctrl+U/Ctrl+Shift+U changes case. The palette's *Line*, *Sort*, *Case* and
  *Blank* commands do the rest.
- **Several carets**: Ctrl+click adds one, Ctrl+Shift+D adds the next occurrence of the word, and
  Alt+drag or Alt+Shift+arrows select a column. Edit → *Column Editor* inserts text or numbers
  down a column.
- **Drag and drop**: drag a selection to move it; hold Ctrl when you drop it to copy it.
- **Right-click** the text for Undo, Cut, Copy, Paste, Select all and Copy as HTML (Settings →
  Advanced → *Context menu of the text* chooses the commands).
- **Completion**: words, the language's keywords and snippets are offered while you type; Tab
  or Enter accepts.
- **Vim**: Settings → Vim switches to Vim keys (Normal, Insert, Visual, `:` commands —
  [JARAK_VIM](JARAK_VIM.md)).
- **Insert characters**: Edit → *Character panel…* (by code, for example `U+00E9`, or from a
  table). Edit → *Paste from clipboard history…* pastes an earlier copy made in Jarak.

## 5. Finding

- Ctrl+F find, Ctrl+H replace, F3 / Shift+F3 next and previous. Options: case, whole word,
  extended (`\n`, `\t`) and regular expressions. *Mark* bookmarks every matching line.
- Ctrl+Shift+F searches a folder (Find in Files). A replace there shows a preview first, and the
  journal can undo it (`jarak journal undo`).
- Ctrl+Alt+I is incremental search. A selected word highlights its other occurrences (Settings
  → Editor → Highlighting).
- Search → *Find the next non-ASCII character* and *Find characters in a range…* find unusual
  characters.

## 6. Markdown and images

- A `.md` file opens as source. Ctrl+E switches to the preview and back; View → *Split* shows
  both. The preview never changes the file.
- **Paste a screenshot** with Ctrl+V into a Markdown document. Jarak saves it as a PNG in
  `assets` next to the document (Settings → Images) and inserts a Markdown image reference to it
  as one undo step. An untitled document is saved first, so the path is real.
- **Images stay local**: remote images are not downloaded; the preview shows their address.
  Edit → *Assets…* lists the document's images, renames one (its references follow) and shows
  orphans. Nothing is deleted without asking.
- Saving to another folder offers to copy the images along, and never replaces a different file.

## 7. Printing and exporting

- File → *Print…* uses the Windows print dialog. *Microsoft Print to PDF* makes a PDF.
- *Export* writes HTML or RTF of the code (with or without line numbers), or the rendered
  Markdown: embedded, linked, or published as a folder with its images. *Copy as HTML / RTF*
  puts the formatted text on the clipboard for Word or mail.

## 8. Settings, keys, extensions

- **Settings** (Ctrl+,) apply at once and are kept in `settings.toml`. The command line can set
  them too: `jarak settings set word_wrap true`.
- **Keys**: Settings → Keyboard changes any shortcut and warns about clashes, also with Kim's
  global hotkeys (`jarak hotkeys`).
- **Extensions** add languages, snippets and commands from a folder
  ([JARAK_EXTENSIONS](JARAK_EXTENSIONS.md)). A command runs only after you grant the permissions
  it declares, and the grant can be revoked. Notepad++ plugin DLLs are not supported
  ([JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)).

## 9. For scripts and agents

`jarak` (without `-gui`) is the headless command line. It never opens a window, every command
answers in `--json` with documented exit codes, and state-changing commands accept `--dry-run`
([JARAK_CLI](JARAK_CLI.md)). Kim and Ddalgak reach Jarak through its interop capabilities, for
example a Noeul screenshot inserted into a note.

## 10. Privacy

Jarak has no account, no telemetry, no ads and no network calls you did not ask for (for
example, *Search on the web* opens your browser with the selection). The clipboard is read only
when you paste. Logs and JSON output never contain your documents' text unless a command's
purpose is to return it.

## 11. Help

Help → *User guide* opens this guide in Jarak's preview. Help → *About Jarak* shows the version,
the programs and folders, and copies the debug information for a bug report (`jarak debug-info`
prints the same).
