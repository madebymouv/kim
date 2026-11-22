# Jarak — command line

Status: **current** (Step 107) · Requirements: `JARAK-FR-150`, `JARAK-FR-151`, `KIM-FR-012`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Contract: [CLI_CONTRACT](CLI_CONTRACT.md) (the
envelope, exit codes, error registry) · Interop: [INTEROP](INTEROP.md) · Architecture:
[JARAK_ARCHITECTURE §29](JARAK_ARCHITECTURE.md) · Matrix: `JRK-14`, `JRK-15`, `HELP-03`…`HELP-10`
([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md))

`jarak` is Jarak's headless command line, for people, scripts and agents. It never opens a window
or a dialog, except `jarak open`, whose job is to open the editor window. Every command prints
human text, or exactly one JSON document with `--json`
(`{"ok", "command", "schema_version": 1, "data" | "error", "warnings"}`). Data goes to stdout and
diagnostics to stderr.

## Rules every command follows

- **Exit codes** ([CLI_CONTRACT §4](CLI_CONTRACT.md)):
  - 0: done
  - 1: an I/O or system failure
  - 2: usage (unknown command or option, an option on the wrong command, a bad value)
  - 3: not found (file, setting, action, printer)
  - 4: an invalid stored file
  - 5: incompatible
  - 6: unavailable
  - 7: refused to protect something: an existing file, a read-only document, a clash, a
    character an encoding cannot hold, or a destructive step without `--yes`
- **Dry runs.** Every command that changes something takes `--dry-run`, which checks everything
  and writes nothing. Destructive commands also need `--yes`, because the command line never asks.
- **Documents are never changed silently.** Only these commands write a document:
  - `replace` (with `--yes`)
  - `journal undo` (with `--yes`)
  - `convert`
  - `assets add`
  - `assets rename`

  Each writes atomically, in the document's own encoding and line endings. `export` and `print`
  never change the document.
- **Nothing is overwritten without being asked.**
  - `new` refuses an existing file.
  - `export` and `print` need `--overwrite` to replace an existing output.
  - `assets add` always picks a new image name.
- **No network** and no telemetry. Remote images are never fetched.
- Settings come from `%APPDATA%\mouv\jarak\settings.toml` ([JARAK_SETTINGS](JARAK_SETTINGS.md)),
  as the window uses them.

## Commands

| Command | Effect | What it does |
| --- | --- | --- |
| `jarak version` | read | Jarak's version, the CLI contract and JSON schema versions |
| `jarak capabilities` | read | The capabilities document: every command, its effect, dry run, consent, window; and the interop capabilities Jarak `provides` |
| `jarak paths` | read | Where Jarak keeps settings, shortcuts, session, recovery, languages, the journal |
| `jarak debug-info` | read | Version, programs, folders and files (whether they exist), the Windows build, the number of printers — for a bug report; never a file's contents (HELP-10; the window's *Window → Copy debug info* puts the same on the clipboard) |
| `jarak info <file> [--fallback <encoding>]` | read | Encoding, byte-order mark, line endings and their counts, lines, characters, language, binary or read-only |
| `jarak find <file\|folder> <text> [--extended\|--regex] [--match-case] [--whole-word] [--dot-all] [--count] [--filters …] [--no-subfolders] [--hidden]` | read | Every hit with its line and column; a folder is Find in Files |
| `jarak replace <file\|folder> <text> <with> [same options] [--dry-run] [--yes]` | destructive | Replace in Files: the plan with `--dry-run`, the change with `--yes` (originals journaled) |
| `jarak journal list` / `undo <batch> [--dry-run] [--yes]` | read / destructive | The Replace in Files batches and undoing one |
| `jarak new <file> [--encoding <name>] [--eol crlf\|lf\|cr] [--text <text>] [--dry-run]` | write | A new text file (never replacing one) in the encoding and line endings asked for (the new-document settings by default), with an optional first text |
| `jarak convert <file> [--encoding <name>] [--eol crlf\|lf\|cr] [--dry-run]` | write | The file saved again in another encoding and/or line endings, atomically; a character the encoding cannot hold is refused and named (`text_unencodable`); a file Jarak opens read-only is refused |
| `jarak export <file> --output <path> [--to html\|rtf\|folder\|pdf] [--line-numbers] [--linked] [--source] [--overwrite] [--dry-run]` | write | HTML, RTF, a published folder or PDF ([JARAK_EXPORT](JARAK_EXPORT.md)) |
| `jarak print <file> [--printer <name>] [--output <file>] [--pages <a-b>] [--overwrite] [--dry-run]` / `jarak printers` | write / read | Printing with the print settings; the printers |
| `jarak assets list <doc.md>` | read | A Markdown document's images and orphans |
| `jarak assets add <doc.md> <image> [--line <n>] [--alt <text>] [--dry-run]` | write | Copies a local image into the document's image folder under a new name and inserts a Markdown image line (alt text, then the path `assets/…`) on a line of its own before line *n* or at the end |
| `jarak assets rename <doc.md> <asset> <new-name> [--dry-run]` | write | Renames an image and its references |
| `jarak session list` | read | The saved session: documents (path or untitled, active, how shown, caret), the second view, the workspace, recent files |
| `jarak settings list/get/set/reset` | read / write | The settings ([JARAK_SETTINGS](JARAK_SETTINGS.md)) |
| `jarak hotkeys list/show/set/disable/reset/check` | read / write | The shortcuts ([HOTKEY_CONTRACT](HOTKEY_CONTRACT.md)) |
| `jarak interop invoke` | as the capability | One Kim interop capability from a `kim.interop.request` on stdin ([below](#kim-interoperability)) |
| `jarak open [<file>[:<line>[:<column>]]...] [--language <name>] [--read-only] [--new-window] [--dry-run]` | write, opens a window | Opens the files in the editor window; a running window takes them |

`--encoding` names: `utf-8`, `utf-8-bom`, `utf-16le`, `utf-16be` (both with a byte-order mark),
`ansi` (the system's code page), or a label such as `euc-kr`, `windows-1252` or `shift_jis`.

## Opening files in the window

`jarak open` and `jarak-gui` take Notepad++'s command-line options under Jarak's names:

| Notepad++ | Jarak | Effect |
| --- | --- | --- |
| `file -nLine -cColumn` | `file:line:column` | The caret at that line and column |
| `-lLanguage` | `--language <name>` | The files after it open in that language (any case: `rust`, `Markdown`, the user's languages; `text` for none) — HELP-03 |
| `-udl="name"` | `--language <name>` | A user-defined language by its name |
| `-ro` | `--read-only` | The files after it open read-only — HELP-04 |
| `-nosession` | `--no-session` (`jarak-gui`) | Starts without the last session and does not save one at exit — HELP-05 |
| `-openSession file` | `--session <file>` (`jarak-gui`) | Opens a saved session's documents — HELP-05 |
| `-multiInst` | `--new-window` | A window of its own instead of the running one |

The options travel to a running window through the hand-off, so `jarak open --read-only
notes.md:12` from a Subak terminal shows *notes.md* read-only at line 12 in the window already
open. `--dry-run` says what would open and starts nothing.

## Kim interoperability

Jarak provides five capabilities to Kim and the other tools ([INTEROP](INTEROP.md); interface
1.0.0). Kim runs `jarak interop invoke --json --no-input` with the request on stdin, and the
answer is the matching command's data:

| Capability | Effect | Params | Same as |
| --- | --- | --- | --- |
| `document.info` | read | `path` | `jarak info` |
| `text.find` | read | `path`, `text`, `regex`, `match_case`, `whole_word` | `jarak find` |
| `document.export` | write | `path`, `output`, `format`, `overwrite` | `jarak export` |
| `markdown.insert-image` | write | `document`, `image`, `line`, `alt` | `jarak assets add` |
| `document.open` | write, opens a window | `path`, `line`, `column` | `jarak open` |

Consent is Kim's:
- `write` needs `--allow-write`, given only as the direct result of a user action.
- `document.open` also needs `--allow-window`.
- `--dry-run` asks what would happen.

Jarak still applies its own rules: never replacing a file without `overwrite`, never the document
itself, local images only.

Workflows:

- **A Noeul screenshot into a Markdown note.** Noeul's `screen.capture-file` writes a PNG, then
  `markdown.insert-image` puts it into the note:

  ```powershell
  kim tools invoke screen.capture-file --allow-write --params '{"monitor":1,"output":"C:\\Users\\me\\Pictures\\shot.png"}' --json
  kim tools invoke markdown.insert-image --allow-write --params '{"document":"C:\\Users\\me\\notes\\day.md","image":"C:\\Users\\me\\Pictures\\shot.png","alt":"screen"}' --json
  ```

  In the window, Ctrl+V pastes a Noeul capture from the clipboard ([JARAK_ARCHITECTURE
  §5](JARAK_ARCHITECTURE.md)).
- **From a Subak terminal to the running editor at a line.** Run `jarak open src\main.rs:42` in the
  terminal, or `kim tools invoke document.open --allow-write --allow-window --params
  '{"path":"…","line":42}'`.
- **A Ddalgak shortcut.** A rule's `tool` step names a capability and its params, for example
  `markdown.insert-image` after `screen.capture-file`, or `document.open` for a notes file
  ([DDALGAK_RULE_MODEL §5.17](DDALGAK_RULE_MODEL.md)). Ddalgak calls through Kim with the same
  consent rules.

## Examples

Golden JSON examples of every command live in `docs/contracts/examples/cli-envelope-v1/jarak-*.json`.
A test checks that each command in the capabilities document has at least one example.
