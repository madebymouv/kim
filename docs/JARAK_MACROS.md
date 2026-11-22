# Jarak — macros and Run reference

Status: **current** (Step 102) · Requirements: `JARAK-FR-110`, `JARAK-FR-111`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Architecture: [JARAK_ARCHITECTURE §24](JARAK_ARCHITECTURE.md) ·
Keys: [JARAK_KEYS](JARAK_KEYS.md)

## Macros (Notepad++'s Macro menu)

| What | How |
| --- | --- |
| Start / stop recording | Ctrl+Shift+R (the status bar says *● Recording*) |
| Play back the recorded macro | F9 — Notepad++ uses Ctrl+Shift+P, which is Jarak's command palette |
| Save the recorded macro… | a name and a shortcut (press the keys in the field; Backspace clears, Esc cancels) |
| Run a macro multiple times… | the recorded one or a saved one, a number of times or *until the end of the file* |
| Macros and run commands… | change a shortcut (click its field, press the keys), remove a macro or a command |
| A saved macro | the palette's *Macro: name*, or its shortcut |
| Built in | *Trim Trailing Space and Save* |

- **What is recorded:** the commands as they run, not the keys. These are the registry's commands
  (moving, selecting, line operations, case, comments, bookmarks …), typed text (a run of typing is
  one step), *paste* (of the clipboard at playback), *copy*, *cut*, *indent*, *outdent*, *save*, and
  Find Next / Replace / Replace All with the find panel's options at that moment. Not recorded:
  the mouse, completions and snippets taken from the popup, Vim's keys, dialogs, documents opened or
  switched.
- **Playback** runs the steps on the active document. Each run is one undo step. A Find or
  Replace step uses its recorded options and leaves the panel's own as they were. A read-only
  document refuses a macro that edits.
- **Until the end of the file** stops when the caret reaches the end, when a run changes neither
  the text nor the caret, when it went back up (a search that wrapped), after 100 000 runs, or
  after 20 seconds, and it says which.
- **Shortcuts** must be free. Refused are:
  - keys Windows or the input method reserve (the hotkey contract, via `kim-hotkeys`: Win+…,
    Ctrl+Alt+Del, Ctrl+Shift+Esc, the IME keys …);
  - Jarak's own keys (for example Ctrl+Alt+1 is Ctrl+1's: the first tab);
  - keys that type text;
  - another macro's or command's keys.

  The global keys of other Kim tools are checked by Kim's hotkey tools (`kim hotkeys check`), not
  here.
- **The file:** `%APPDATA%\mouv\jarak\macros.json` (`"format": "jarak.macros/1"`, the macros and
  the run commands). It is written atomically. A file that cannot be read is named and never
  written over.

  ```json
  {
    "format": "jarak.macros/1",
    "macros": [
      { "name": "Tidy", "keys": "Ctrl+Shift+1",
        "steps": [ { "cmd": "blank.trim-trailing" }, { "type": "!" }, { "action": "save" },
                   { "find": { "op": "replace_all", "find": "a+", "replace": "b", "mode": "regex" } } ] }
    ],
    "commands": [ { "name": "Notepad", "command": "notepad.exe \"$(FULL_CURRENT_PATH)\"" } ]
  }
  ```

## Run (Notepad++'s Run menu)

- **F5** (*Run…*) opens a dialog: type a command line with variables. The dialog shows the exact
  line that will run, with the variables replaced, and runs it only when **Run** is pressed. *Save*
  keeps it with a name and an optional shortcut. A saved command (*Run: name* in the palette, or
  its shortcut) opens the same dialog showing its exact line; nothing runs without the button.
- **Variables** (Notepad++'s names): `$(FULL_CURRENT_PATH)`, `$(CURRENT_DIRECTORY)`, `$(FILE_NAME)`,
  `$(NAME_PART)`, `$(EXT_PART)`, `$(CURRENT_WORD)` (the selection, or the word at the caret),
  `$(CURRENT_LINE)`, `$(CURRENT_COLUMN)`, `$(CURRENT_LINESTR)`. `$(JARAK_DIRECTORY)` and
  `$(JARAK_FULL_FILE_PATH)` also answer to `NPP_DIRECTORY` and `NPP_FULL_FILE_PATH`.
  `$(SYS.NAME)` is an environment variable (empty when unset). An unknown name stays as written.
  Values are not quoted: write the quotes yourself (`"$(FULL_CURRENT_PATH)"`).
- **What runs:**
  - a web address (`http://…`, `https://…`) opens in the default browser;
  - a file that is not a program opens with its default program (programs and scripts are
    refused there);
  - anything else is a program and its arguments: the first word, or the quoted text, is the
    program, found on `PATH`, and the rest is passed exactly as written. It starts in the
    document's folder, without a shell, and Jarak does not wait for it. For a shell, write it:
    `cmd /c …`.
- **Built in:** *Open the file with its default program*, *Wikipedia search for the current word*,
  *PHP help for the current word* (Notepad++'s *Launch in …* entries).
- **Search the selection on the internet** (Edit) opens the search engine with the selection or
  the word at the caret. The address is `search_engine` in the settings file (`$(QUERY)` the
  words), DuckDuckGo by default.
- **View the file in the default browser** (View) is for HTML, SVG and XML files. It opens the
  saved file and says so when the document has unsaved changes.

## Security notes

- Nothing runs by itself. Macros play only on a key, the palette or the dialog. A run command runs
  only after its exact line was shown and **Run** pressed. No command runs at start-up, on open,
  on save or from a document's content.
- **Never elevated.** Programs start with `CreateProcess` as the user. A program that asks for
  administrator rights is refused (*it needs administrator rights, and Jarak never elevates*); no
  UAC prompt comes from Jarak.
- **No shell unless written.** The arguments go to the program as written. `cmd /c`,
  `powershell -c` and the like run only when the line says so.
- **No network by itself.** Web addresses open in the user's browser only through Run, *Search on
  the internet* or a built-in entry, each an explicit action. Path completion never reads network
  paths (Step 100).
- **The file is the user's.** `macros.json` is never overwritten when it could not be read. Macros
  and commands are plain JSON the user can read and edit.
