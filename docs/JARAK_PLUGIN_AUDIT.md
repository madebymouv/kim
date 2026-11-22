# Jarak — Notepad++ plugin compatibility audit

Status: **current** (Step 108, 2026-11-22) · Requirement: `JARAK-FR-161`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Matrix: `PLUG-01`, `PLUG-02`, `PLUG-09`
([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)) · Extensions:
[JARAK_EXTENSIONS](JARAK_EXTENSIONS.md)

## Binary compatibility: none

Notepad++ plugins are Windows DLLs built against Notepad++'s plugin interface: the
`NppData`/`SCNotification` structures and the `NPPM_*` and Scintilla `SCI_*` messages sent to
Notepad++'s and Scintilla's window handles. Jarak is not Notepad++, has no Scintilla component and
no such window messages, so **no Notepad++ plugin DLL can run in Jarak**, and Jarak never loads one
(`PLUG-01`: not intended). Jarak also has no Plugins Admin: it downloads nothing and updates
nothing (`PLUG-02`: not intended; `KIM-SEC-001`).

What Jarak offers instead:

1. Built-in features that cover what the plugin did.
2. Its own extensions: languages, snippets and commands that run a program with explicit,
   revocable permissions ([JARAK_EXTENSIONS](JARAK_EXTENSIONS.md)).

## How to read the table

- **Built in**: Jarak does it without the plugin; the matrix rows and Steps are the evidence.
- **Partly**: Jarak covers the main use; the rest is named.
- **By extension**: a Jarak extension command can do it with a program the user provides; nothing
  ships in Jarak.
- **Not covered**: Jarak has no equivalent today.
- **Not intended**: against Jarak's rules (local-first, no network, no embedded scripting
  runtime, no plugin DLLs).

## The plugins

The list covers the plugins most often installed through Plugins Admin and the ones bundled with
Notepad++.

| Plugin | What it does | Jarak | How |
| --- | --- | --- | --- |
| MIME Tools (bundled) | Base64, URL, quoted-printable | Built in | Step 97: `PLUG-03`…`PLUG-05` |
| Converter (bundled) | ASCII ↔ hex | Built in | Step 97: `PLUG-06` |
| NppExport (bundled) | HTML / RTF export and copy | Built in | Step 106: `PLUG-07` (pasting from the real clipboard still to be checked) |
| MarkdownViewer++ | Markdown preview | Built in | Steps 89–91: a native preview with local images, split view, scroll sync (`JRK-01`…`JRK-06`) |
| Explorer / LightExplorer | Folder tree panel | Built in | Step 93: the workspace folder panel (`FILE-04`, `VIEWS-*`) |
| SessionMgr | Named sessions | Built in | Step 93: save and open session files (`FILE-17`, `FILE-18`), `jarak session list` |
| Location Navigate | Back / forward through edit positions | Built in | Step 101: navigation history, change history (`MARK-09`…`MARK-15`) |
| Document Monitor | Follow a growing file | Built in | Step 104: Monitoring (tail -f) (`VIEW-17`) |
| AutoSave | Saves documents by itself | Partly | Step 104: unsaved documents are kept as recovery snapshots every 10 s and offered back after a crash; Jarak never writes a document without an explicit save (`JARAK-SEC-001`), so saving into the files by itself is not intended |
| TextFX | Line, case, sort and whitespace tools | Built in | Steps 96–97: line operations, sorting, case, blanks, Column Editor (`LINE-*`, `CASE-*`, `BLANK-*`, `MULTI-*`) |
| QuickText / FingerText / Snippets | Expanding snippets | Built in | Step 100: snippets with tab stops and mirrors (`JRK-12`); extensions can add snippet files |
| NppMenuSearch | Searching the menus | Built in | Step 92: the command palette (Ctrl+Shift+P) |
| Customize Toolbar | Toolbar buttons | Not applicable | Jarak has no toolbar; the palette and Settings → Keyboard (any command can get keys, Step 105) take its place |
| NppExec | Run programs and scripts, show their output in a console | Partly | Step 102: Run with variables, saved commands with shortcuts (`RUN-*`); Step 108: an extension command runs a program with the document or selection on stdin and puts its output into the document, a new document or the status bar. Missing: an interactive console panel and NppExec's script language |
| Python Script / LuaScript / jN | Scripting Notepad++ in a language | Not intended | No embedded scripting runtime; an extension command can run a program the extension ships (an `.exe`), with permissions |
| Compare / ComparePlus | Line diff of two documents | Not covered | Two views side by side with synchronized scrolling (Step 93) and *Compare with the file on disk* (Step 104) open documents next to each other, but nothing marks differences |
| JSON Viewer / JSTool | JSON tree, format, minify | Not covered / By extension | No tree view; formatting by an extension command that runs a formatter program the user provides (input: selection, output: replace) |
| XML Tools | Pretty print, validate, XPath | Partly / By extension | Built in: tag matching and auto-close (`AUTO-06`, `AUTO-09`), XML highlighting and folding; pretty print or validation by an extension command with a program the user provides; no XPath |
| DSpellCheck / Spell Checker | Spell checking as you type | Not covered | No spell checker in Jarak's roadmap |
| Hex Editor | Edit bytes in hex | Not covered | Binary files open read-only with their control characters shown (Step 103, `ENC-*`); no hex view |
| NppFTP | Edit files over FTP/SFTP | Not intended | No network access (`JARAK-SEC-003`); files on a mounted drive open like any file |
| MultiClipboard / ClipboardHelper | Clipboard history | Not covered | Planned for Step 110 (`EDIT-17`) |
| AnalysePlugin | Many searches at once with highlighting | Partly | Find All, Mark All with five styles, Find in Files (Steps 94–95); no saved search sets |
| CSV Lint / CSVQuery | CSV columns, checks, queries | Not covered | Column selection and the Column Editor work on CSV text (Step 96) |
| Function List (built into Notepad++ since 7.x) | Symbols of the document | Built in | Step 99 (`SRCH-18`, `SRCH-19`) |
| Elastic Tabstops / Code Alignment | Aligning columns | Not covered | — |

## Extensions instead of plugins

A Jarak extension is a folder with a manifest:
- languages (Step 98's JSON format, also imported from Notepad++ UDL XML)
- snippets
- commands, each running a program from the extension or from Windows' System32

A command is used only after the user granted the permissions its manifest names. The program
runs without a window, from an empty folder, with a time and memory limit.

[JARAK_EXTENSIONS](JARAK_EXTENSIONS.md) explains it, with the sample extension
`examples/jarak-extensions/mouv.sample-tools`, which adds a language, snippets and two commands:
sorting selected lines with Windows' `sort.exe`, and counting a document's lines with `find.exe`.
