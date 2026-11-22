# Jarak — extensions

Status: **current** (Step 108) · Requirements: `JARAK-FR-160`, `JARAK-SEC-007`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Matrix: `PLUG-08` · Architecture:
[JARAK_ARCHITECTURE §30](JARAK_ARCHITECTURE.md) · Plugins: [JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)
· CLI: `jarak extensions` ([JARAK_CLI](JARAK_CLI.md))

Jarak's extensions are folders. They add languages, snippets and commands. They are installed
only when you ask, never downloaded or updated, and a command that runs a program needs your
explicit, revocable grant. Notepad++ plugin DLLs are not supported
([JARAK_PLUGIN_AUDIT](JARAK_PLUGIN_AUDIT.md)).

## An extension

A folder with `extension.json` (schema `jarak-extension-v1`) and the files it names:

```json
{
  "schema": "jarak.extension",
  "schema_version": 1,
  "id": "mouv.sample-tools",
  "name": "Sample tools",
  "version": "1.0.0",
  "description": "…",
  "contributes": {
    "languages": ["todo-list.json"],
    "snippets": ["snippets.json"],
    "commands": [
      { "id": "sort-lines", "title": "Sort the selected lines (Windows sort)",
        "program": "sort.exe", "input": "selection", "output": "replace-selection" }
    ]
  },
  "permissions": ["run-process", "read-document", "write-document"]
}
```

- **id**: lower-case letters and digits joined by `.` or `-` (64 at most). An installed extension
  lives in `%APPDATA%\mouv\jarak\extensions\<id>`.
- **languages**: user-language files in Step 98's JSON format ([JARAK_ARCHITECTURE
  §19](JARAK_ARCHITECTURE.md); a Notepad++ UDL XML file can be converted with *Import a Notepad++
  UDL*).
- **snippets**: files in the `snippets.json` format of Step 100 (`{ "<language or *>": { "<prefix>":
  { "body": …, "description": … } } }`).
- **commands**: each runs one program:
  - `program`: an `.exe` inside the extension (a relative path), or a Windows program's name such as
    `sort.exe`, found only in System32. Scripts (`.cmd`, `.bat`, `.ps1`) and shell command lines are
    not run.
  - `args`: the arguments, passed as they are. There is no shell.
  - `input`: `none`, `selection` or `document`, sent to the program's stdin as UTF-8.
  - `output`: `message` (the first line in the status bar), `replace-selection`, `insert` (at the
    caret) or `new-document`. A document change is one undo step.
  - `timeout_ms`: 10 000 by default, 60 000 at most.
- **permissions**: what the commands need. `run-process` for every command, `read-document` when
  it reads the selection or document, `write-document` when its output goes into the document. A
  command that needs a permission the manifest does not name is refused when the extension is
  checked.

Every path stays inside the folder: `..`, drives and absolute paths are refused. An extension
has at most 200 files and 50 MiB.

## Permissions and grants

- Languages and snippets are data: they work as soon as the extension is installed.
- A command runs only when you have **granted** the extension the permissions its manifest names.
  - In the window, the first run of one of its commands asks: *Allow "Sample tools" to run
    `sort.exe`? It gets the selected text and replaces it.* **Allow** remembers it; **Cancel** runs
    nothing.
  - On the command line: `jarak extensions grant <id>`.
- A grant is for **exactly this manifest** (its SHA-256). If the extension changes (another
  program, other arguments, more permissions), it asks again.
- `jarak extensions revoke <id>` ends a grant. Grants live in
  `%APPDATA%\mouv\jarak\extension-grants.json`.

## How a command's program runs

- Without a window, from a new empty temporary folder (deleted afterwards), with only `SystemRoot`,
  `TEMP`/`TMP` and `PATH` (System32) in its environment.
- In a Windows job with 512 MiB of memory; the job, and with it the program and anything it
  started, ends when the command ends.
- Stopped at its timeout. At most 1 MiB of output, which must be UTF-8 text. A non-zero exit code
  is reported with the first line of what it wrote to stderr; nothing changes in the document then.
- **The honest limit:** the program runs with your Windows account's rights. Jarak limits time,
  memory, environment and windows, but it cannot stop the program from reading or writing files or
  using the network. The grant is that permission, so grant only programs you trust. Windows'
  own `sort.exe` and `find.exe` in the sample are such programs.

## Installing, listing, removing

```text
jarak extensions list                       (read-only: installed, problems, grants)
jarak extensions check <folder>             (read-only: every problem of a folder)
jarak extensions install <folder> [--replace] [--dry-run]
jarak extensions remove <id> [--dry-run]    (also ends its grant)
jarak extensions grant <id> [--dry-run]
jarak extensions revoke <id> [--dry-run]
```

- `install` copies a local folder that checks. An installed extension with the same id is replaced
  only with `--replace`; its grant then ends unless the manifest is unchanged.
- A folder in the extensions folder that does not check is listed with its problems and not used.
- The window reads the extensions when it starts.

## The sample extension

`examples/jarak-extensions/mouv.sample-tools` adds:
- the *TODO list* language (`.todo` files: TODO, DOING, DONE, WAIT keywords, `@contexts`, `!priority`)
- two snippets (`todo`, `done`)
- two commands:
  - *Sort the selected lines (Windows sort)*: `sort.exe`, selection in, selection replaced
  - *Count the document's lines (Windows find)*: `find.exe /c /v ""`, document in, the count in
    the status bar

Install it with `jarak extensions install examples\jarak-extensions\mouv.sample-tools`.

## Not in Jarak

- Notepad++ plugin DLLs
- a plugin catalog or automatic updates
- an embedded scripting language
- extensions that draw their own panels or dialogs
- themes (Jarak's colours come from kim-design; token colours are `SET-22`)
