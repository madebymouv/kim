# Jarak — settings

Status: **current** (Step 105) ·
Requirements: `JARAK-FR-130`, `JARAK-FR-131`, `JARAK-FR-132` ([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) ·
Architecture: [JARAK_ARCHITECTURE §27](JARAK_ARCHITECTURE.md) · Matrix: SET-01…26
([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)) · CLI: `jarak settings` ([CLI_CONTRACT](CLI_CONTRACT.md))

## The Settings window

- Opened by the **gear** at the right end of the status bar, **Ctrl+,**, or the palette's
  *Settings…* (File menu). It is a window of its own (*Jarak Settings*, ADR 0100): the editor
  stays as it is underneath; the gear brings it forward; **Esc** or the close button closes it.
  With the status bar turned off, Ctrl+, and the palette still open it.
- Sections: **General, Editor, Appearance, Markdown, Images, Vim, Keyboard, Files, Advanced** —
  every row a switch, a list, a number or a text field from one registry
  (`jarak_core::settings::SETTINGS`). Text fields are saved when they lose the focus (or on Enter).
- **Every change applies at once** and is written to `%APPDATA%\mouv\jarak\settings.toml`
  first: checked against the setting (a value it does not take is said beside the row and
  nothing is written), atomically (a temporary file, then a rename), keeping the file's other
  lines and comments. What the window shows is what the file says.
- **Keyboard** (the shortcut mapper, [HOTKEY_CONTRACT](HOTKEY_CONTRACT.md)): your changes first,
  then every command with its keys, searchable by name or keys. **Change** records the next keys
  (Esc cancels; recording stops when the window loses the focus), **Off** removes a command's keys,
  **Reset** gives its defaults back. Keys another command has by default go to this one (the window
  says which default they replace); keys another of *your* bindings has are refused and named,
  with **Take it** to move them. **Reset all shortcuts** asks twice and keeps the previous file as
  `hotkeys.json.bak`. The changes live in `%APPDATA%\mouv\jarak\hotkeys.json` (only what you
  changed; a broken or newer file is not used and never rewritten). The navigation and editing
  keys every Kim window reserves (arrows, Home, End, Page Up/Down, Tab, Enter, Backspace, Delete,
  Insert, Esc and their Shift forms) are the editor's own; a saved macro's or run command's
  shortcut is set where the macro or command is (Step 102).
- **Advanced**: *Show in Explorer* (the settings file), *Reset every setting* (asks twice; lines
  Jarak does not know stay).
- English and Korean for every string; **Language** follows Windows unless set (a `--lang` on
  the command line fixes it for that run). **Theme**: System follows Windows' app mode (read again
  when the window gets the focus), or dark / light (Kim's purple accent in both).

## The settings file and the command line

- `settings.toml` holds one `key = value` line per setting that differs from its default
  (booleans and numbers bare, text quoted). A missing line, or a value the setting does not take,
  is the default — reading never fails.
- `jarak hotkeys list | show <action> | set <action> <keys>... [--replace] | disable <action> |
  reset <action> | check` change the shortcuts the same way (the window reads `hotkeys.json` again
  when it gets the focus).
- `jarak settings list [<section>]`, `get <key>`, `set <key> <value> [--dry-run]`,
  `reset <key> [--dry-run]` read and write the same file (CLI_CONTRACT: `setting_not_found`,
  `setting_invalid`). A running window reads the file again when it gets the focus.
- The file as JSON is described by `docs/contracts/schemas/jarak-settings-v1.schema.json` (built
  from the registry; a test keeps it and its defaults example equal to the registry).

## Every setting

### General

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `language` | Language | `system` / `en` / `ko` | `system` |
| `theme` | Theme | `system` / `dark` / `light` | `system` |
| `menu_bar` | Menu bar | `alt` / `always` | `alt` |
| `status_bar` | Status bar | `true` / `false` | `true` |
| `tab_close_buttons` | Close buttons on tabs | `true` / `false` | `true` |
| `tab_double_click_close` | Double-click closes a tab | `true` / `false` | `false` |
| `single_window` | One window | `true` / `false` | `true` |
| `restore_session` | Restore the session at start | `true` / `false` | `true` |

### Editor

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `tab_width` | Tab width | 1–16 | `4` |
| `use_spaces` | Tab inserts spaces | `true` / `false` | `false` |
| `indent` | Indentation on Enter | `off` / `keep` / `smart` | `smart` |
| `word_wrap` | Word wrap | `true` / `false` | `false` |
| `highlight_current_line` | Highlight the current line | `true` / `false` | `true` |
| `caret_blink` | Blinking caret | `true` / `false` | `false` |
| `multi_editing` | Ctrl+click adds a caret | `true` / `false` | `true` |
| `virtual_space` | Virtual space | `true` / `false` | `false` |
| `copy_line` | Copy the line without a selection | `true` / `false` | `true` |
| `complete_auto` | Completion while typing | `true` / `false` | `true` |
| `complete_min` | From how many characters | 1–9 | `2` |
| `complete_tab` | Tab accepts a completion | `never` / `chosen` / `always` | `chosen` |
| `complete_enter` | Enter accepts a completion | `never` / `chosen` / `always` | `chosen` |
| `param_hints` | Parameter hints | `true` / `false` | `true` |
| `snippets` | Snippets in the completion list | `true` / `false` | `true` |
| `auto_close` | Close brackets and quotes | `true` / `false` | `false` |
| `close_tags` | Close HTML/XML tags | `true` / `false` | `false` |
| `brace_highlight` | Highlight matching brackets | `true` / `false` | `true` |

### Appearance

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `font_family` | Editor font | `default` / `consolas` / `cascadia-mono` / `d2coding` / `courier-new` | `default` |
| `font_size` | Editor font size | 8–40 | `13` |
| `line_numbers` | Line numbers | `true` / `false` | `true` |
| `edge_columns` | Edge lines at columns | columns 1–1000, comma separated | `(none)` |
| `show_spaces` | Show spaces and tabs | `true` / `false` | `false` |
| `show_eol` | Show the line endings | `true` / `false` | `false` |
| `show_controls` | Show control characters | `true` / `false` | `false` |
| `show_indent_guides` | Show the indent guides | `true` / `false` | `false` |
| `show_wrap_symbol` | Show the wrap symbol | `true` / `false` | `false` |

### Markdown

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `markdown_open_as` | Markdown files open as | `source` / `split` / `preview` | `source` |

### Images

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `image_folder` | Folder for pasted images | text (one line) | `assets` |

### Vim

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `editing_mode` | Editing | `standard` / `vim` | `standard` |

### Files

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `new_encoding` | Encoding | `utf-8` / `utf-8-bom` / `ansi` | `utf-8` |
| `new_eol` | Line endings | `crlf` / `lf` / `cr` | `crlf` |
| `recent_max` | Recent files kept | 0–50 | `20` |
| `detect_changes` | Detect files changed by other programs | `true` / `false` | `true` |
| `reload_unmodified_silently` | Reload unmodified documents silently | `true` / `false` | `false` |
| `recovery_interval` | Keep unsaved documents every (seconds) | 0–3600 | `10` |

### Advanced

| Key | Setting | Values | Default |
| --- | --- | --- | --- |
| `search_engine` | Search engine | text with `$(QUERY)` | `https://duckduckgo.com/?q=$(QUERY)` |
| `date_format` | Custom date format | text (one line) | `yyyy-MM-dd HH:mm:ss` |

## What each setting does

- **Menu bar** *With Alt*: shown while Alt is pressed (Alt again or Esc hides it); *Always*: it
  stays. **Status bar** off hides the line under the text and its gear.
- **Close buttons on tabs** off: only the active tab and the one under the pointer show the ×
  (a tab with unsaved changes keeps its dot). **Double-click closes a tab** asks about unsaved
  changes as any close does.
- **One window** off: every start of `jarak-gui` (or `jarak open`) opens a window of its own.
- **Tab width** and **Tab inserts spaces** apply to every open document; with spaces, Tab adds
  spaces up to the next tab stop and indenting lines adds a tab width of spaces.
- **Word wrap** applies to every open document when it changes and to each new one (View's *Word
  wrap* still changes one document). **Highlight the current line**, **Blinking caret** (off by
  default: an idle window never repaints; on, at most twice a second while it has the focus),
  **Ctrl+click adds a caret** (off: Ctrl+click places the caret; links and images still open).
- **Editor font**: *Built in* is the design system's monospace font; Consolas, Cascadia Mono,
  D2Coding and Courier New are used from the Windows or the user's fonts folder when the PC has
  them (Hangul and symbols the font lacks come from the built-in chain); a font the PC does not
  have is said and the built-in one is used. **Editor font size** is at 100 % zoom (Ctrl+Plus /
  Ctrl+Minus zoom on top of it). **Line numbers** off keeps the bookmark, change and fold margins.
- **Markdown files open as** source, split or preview (a restored session's documents keep
  theirs). **Folder for pasted images**: Ctrl+V writes images into this folder next to the
  document (a plain folder name; the orphan scan of *Assets* still looks at `assets`).
- **Files**: new documents' encoding and line ending; how many recent files are kept; detecting
  files changed by other programs, reloading unmodified documents silently and recovery
  snapshots (Step 104, [JARAK_RECOVERY](JARAK_RECOVERY.md)).

## Not in this Step

`jarak hotkeys enable/export/import`, shortcut profiles and key sequences; a default folder for
the Open dialog (SET-07);
smart highlighting and mark-all options (SET-11); searching options (SET-13); word delimiters
(SET-17); per-language tab sizes (SET-10, partial); multi-line tab bars (SET-02, partial);
scrolling beyond the last line (SET-03, partial); hiding the bookmark margin (SET-05, partial);
a default language for new documents (SET-06, partial); per-language token colours and imported
style themes (SET-22 partial, SET-24).
