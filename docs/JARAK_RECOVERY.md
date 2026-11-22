# Jarak — files changed on disk, recovery and the session

Status: **current** (Step 104) · Requirements: `JARAK-FR-120`, `JARAK-FR-121`, `JARAK-FR-122`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Architecture: [JARAK_ARCHITECTURE §26](JARAK_ARCHITECTURE.md) ·
Matrix: REL-02…05, JRK-16, VIEW-17, SET-14 ([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md))

Jarak never loses work it has seen: a file another program changes is noticed and reloaded only in
a way that can be undone; the text of documents with unsaved changes is kept aside while the window
runs, so a crash or a killed process does not take it; the session survives a damaged file. All of
it works only while Jarak runs — no service, no scheduled task, nothing left running.

## Files changed by other programs (`JARAK-FR-120`)

**When.** The open documents' files are compared with what each document last read or wrote (the
file's size and last-write time) when the window gets the focus, and every 2 seconds while it has
it (every second while a document is monitored). The comparison runs on a short-lived thread, so
a slow disk or a network share never holds the window.

| What happened | The document | What Jarak does |
| --- | --- | --- |
| The file changed | no unsaved changes | asks: **Reload** or **Keep this version**; with *Reload unmodified documents silently* on, reloads at once and says so |
| The file changed | unsaved changes | asks: **Keep mine** (this text stays; a save writes it over the other change), **Reload** (the file's text; Ctrl+Z brings yours back), **Compare** (the file's version opens read-only beside it, named `name (on disk)`) |
| The file was deleted or moved | either | the document stays open with its text, unsaved; **Save** writes the file again |
| A save while the file changed unseen | unsaved changes | the save does not happen; the question comes first |

- **A reload is one undo step.** Only the part that differs is replaced, so bookmarks and marks on
  unchanged lines stay, and **Ctrl+Z** after a reload brings back the text from before it
  (unsaved). Nothing is lost by choosing Reload. Esc on the question keeps what is on screen.
- **Monitoring (tail -f)** (`JARAK-FR-122`, View menu): the document follows its file, stays at its
  end and is read-only while it does; a file that only grew gets the new lines added. The same
  entry stops it (the document's own read-only state comes back). A document with unsaved changes
  or without a file cannot be monitored (Jarak says why).
- **Reload from disk** (File menu) reads the file again the same undoable way.

## Recovery snapshots (`JARAK-FR-121`)

- While a window runs, the documents with unsaved changes — untitled ones too — are copied every
  10 seconds (when their text changed) into a folder of the window's own:
  `%LOCALAPPDATA%\mouv\jarak\recovery\w-<process id>-<process start time>\`. The documents' own
  files are never written; nothing leaves the PC.
- The folder holds `manifest.json` (schema [`jarak-recovery-v1`](contracts/schemas/jarak-recovery-v1.schema.json):
  the window, and per document its file or *untitled*, its name, encoding, byte-order mark, line
  ending, caret, size, when it was kept, and what the file looked like then) and one UTF-8 text
  file per document. Texts are written first and the manifest last, each through a temporary file
  and a rename, so a crash in the middle never leaves a manifest pointing at half a text.
- A document that is saved, discarded or closed leaves the folder at once; a window with nothing
  unsaved has no folder; a normal exit removes it.
- Bounded: at most 100 documents and 512 MB of text per window; the largest ones beyond that are
  left out, and the window names them once.
- The writing happens on a short-lived thread with a cheap copy of the text, so typing never waits.

## After a crash

When Jarak starts and finds folders whose window no longer runs (its process is gone, or its
process id now belongs to another process — the start time tells them apart), it asks
**Recover unsaved documents?** and lists each document (name, file or *untitled*, size, when it was
kept):

- **Restore** opens them with the kept text, unsaved. A document with a file opens that file and
  gets the kept text as one undo step, so Ctrl+Z shows what is on disk; a file that is already
  open gets it in its tab. The files on disk stay as they are until you save. The old folder is
  removed once this window has kept the documents itself.
- **Not now** keeps them; they are offered again at the next start.
- **Discard…** asks again (**Discard for good**) and then removes them.
- A running window's folder is never offered or touched. Restoring or discarding first renames the
  folder for the window doing it, so two windows starting at once never both take it.

## The session file

`session.json` (Step 93) is written atomically. Since Step 104, before a save replaces a good
session file, that file is kept as `session.json.bak`. A damaged session file (unreadable, or not
a valid session) falls back to the backup, and the window says so; the damaged file is kept as
`session.json.broken` — never deleted, never overwritten by the next save. A session written by a
newer Jarak is left as it is.

## Settings

Palette (File and View menus) and `%APPDATA%\mouv\jarak\settings.toml`:

| Entry | Key | Default |
| --- | --- | --- |
| *Detect files changed by other programs: on/off* | `detect_changes` | on |
| *Reload unmodified documents silently: on/off* | `reload_unmodified_silently` | off (asks, as Notepad++) |
| *Recovery snapshots of unsaved documents: on/off* | `recovery_interval` (seconds; `0` off) | 10 |
| *Monitoring (tail -f): on/off* | — (per document) | off |
| *Reload from disk* | — | — |

## Verified

- Tests: the disk state and the undoable reload (`jarak-core` file tests), setting the text by its
  difference (document tests), the snapshot folder, its limits, claims and the golden example
  (`recovery` tests), a **killed child process** whose snapshots the next scan finds with the real
  Windows process check (`tests/recovery_kill.rs`), the session backup and fallback (`session`
  tests), and 12 window flows (asking and reloading, silent reload, keep mine / reload / compare,
  a deleted file, a save that refuses an unseen change, monitoring, the window's own checks,
  snapshots offered and restored, not now and discard, a saved document leaving its snapshot and a
  normal exit keeping none, a damaged session, the UI audit of both dialogs).
- e2e `step104_jarak_recovery.rs`: the release window offers a dead window's folder and touches
  nothing; a damaged session opens the backup and is kept as `.broken`; both dialogs under the
  audit's rules.
- Real desktop (2026-11-22): a test instance with real typing in a file and an untitled document
  was killed (TerminateProcess) after its snapshot; the next start offered both, **Restore**
  brought back the exact text, and the file saved from it holds it. Another program's change was
  noticed in under 1.1 s; Reload, Ctrl+Z and Save gave back the original text; Compare opened the
  file's version beside it; Keep mine saved mine; a deleted file was written again by Save.

## Not in this Step

- *Backup on save* (Notepad++'s simple and verbose backups of the previous file) — SET-14 stays
  partial.
- A Windows restart or power loss itself was not tested (it would restart the PC); the snapshots
  are on disk like after a killed process, so the next start offers them the same way.
- File-change notifications from the system (`ReadDirectoryChangesW`) are not used; checking on
  focus and every 2 seconds covers the same cases without a watcher thread per folder.
