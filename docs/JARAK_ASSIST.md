# Jarak — editing assistance reference

Status: **current** (Step 100) · Requirements: `JARAK-FR-094`, `JARAK-FR-095`, `JARAK-FR-096`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Architecture: [JARAK_ARCHITECTURE §22](JARAK_ARCHITECTURE.md) ·
Keys: [JARAK_KEYS](JARAK_KEYS.md)

Completion, parameter hints, snippets, closing brackets, quotes and tags, indentation and the
matching-bracket highlight. Each one is a setting of its own (the command palette's Edit menu,
kept in `%APPDATA%\mouv\jarak\settings.toml`). Vim editing gets the indentation on Enter and the
bracket highlight; the popup, hints, closers, tags and snippets belong to Standard editing.

## Completion

| How | What it lists |
| --- | --- |
| While typing a word (from 2 characters; *Completion while typing: on/off*) | snippets, API functions and keywords, the language's keywords, types and constants, the document's words |
| Ctrl+Space | the same, for the word before the caret (one entry: inserted at once) |
| Ctrl+Enter | the document's words only |
| Ctrl+Alt+Space | file and folder names for the path before the caret (folders end with `\`) |

- The document's words come from 2 000 lines around the caret (at least two characters, not only
  digits; Korean and other scripts are words too), matched case-insensitively; one text appears
  once, the richer kind winning (snippet, function, keyword, word).
- **Keys in the popup:** Up/Down and Page Up/Down choose; a click takes an entry; Esc closes;
  Backspace follows the shorter word; any other key (a non-word character, a caret move) closes it.
- **Tab and Enter are never taken unasked** (`JARAK-FR-094`): a popup that opened by itself while
  typing lets Tab indent and Enter break the line, until an arrow key chose an entry. Settings:
  *Tab accepts a completion …* and *Enter accepts a completion …*: never / after choosing with the
  arrow keys (the default) / always. A popup asked for (Ctrl+Space, Ctrl+Enter, Ctrl+Alt+Space)
  takes Tab and Enter at once.
- **Paths:** relative to the document's folder (an untitled document has none), or absolute.
  Network paths (`\\server\…`, `//…`) are never read, as reading them would reach the network.

## Parameter hints

- Typing the language's opening character after a known function (`printf(`) shows its signature
  above the line, the current parameter in bold accent; commas inside strings and nested calls do
  not count. Ctrl+Shift+Space shows it on demand. Alt+Up / Alt+Down switch between overloads
  (`1/2`). Esc closes it; leaving the call closes it. *Parameter hints: on/off*.
- Functions come from **API files in Notepad++'s format** (`autoCompletion\<language>.xml`): put
  them in `%APPDATA%\mouv\jarak\autoCompletion\` (*Open the API files folder*), named or not after
  the language (the `language` attribute decides). Read: `<Environment ignoreCase startFunc
  stopFunc paramSeparator>`, every `<KeyWord name func>` with its `<Overload retVal descr>` and
  `<Param name>`. A user file replaces the built-in one of its language. Built in: a few common
  functions of C (also C++), Python and JavaScript. A broken file is named, the others still load
  (*Read the snippets and API files again*).

## Snippets

- `%APPDATA%\mouv\jarak\snippets.json` (*Edit the snippets file* creates a starting one, never
  replacing a file):

  ```json
  {
    "*":    { "sig": { "body": ["-- ", "${1:name}"], "description": "A signature" } },
    "Rust": { "fn":  { "body": ["fn ${1:name}(${2}) {", "\t$0", "}"], "description": "A function" } }
  }
  ```

  The key is a language name (as in the Language menu) or `*` for every one; `body` is a string or
  a list of lines.
- In a body: `$1`, `${1:placeholder}` are tab stops, the same number in several places is one
  field typed in all of them, `$0` is where the caret ends (else the end), `\$` a dollar sign, a
  `\t` at a line's start one indentation level (the document's own: a tab, or its run of spaces);
  every line after the first gets the indentation of the line the snippet is typed on.
- A snippet is offered in the popup when its prefix is typed (also the whole prefix); taking it
  expands it and selects the first stop. **Tab** goes to the next stop, **Shift+Tab** back, the last
  stop or **Esc** ends it. One undo step takes the expansion back. *Snippets in the completion
  list: on/off*.
- Built in: `todo` (every language); Rust `fn`, `test`, `match`; Python `def`, `ifmain`;
  JavaScript `fn`, `log`; Markdown `link`, `code`, `table`. The user's file adds to them; the same
  prefix for the same language replaces a built-in one.

## Typing aids

| Aid | Default | What it does |
| --- | --- | --- |
| *Close brackets and quotes* | off (as in Notepad++) | `(` `[` `{` and the language's quotes insert their closer before whitespace, a closer or the end; a quote does not close after a word (`don't`). Typing a closer it inserted steps over it; Backspace in an empty pair it inserted removes both. With a selection, the pair goes around it and the text stays selected. Every caret. |
| *Close HTML/XML tags* | off (as in Notepad++) | In HTML, XML and PHP, `>` after `<name …` adds `</name>` after the caret (not for void elements, `</…`, `<!…`, `<?…` or `/>`; a `>` inside an attribute value is not the end). |
| *Indentation on Enter: off / keep / smart* | smart | Keep: the line's indentation. Smart: one level more after an opening bracket (or a line ending in `:` in Python and YAML); Enter between a pair puts the closer on its own line; a closing bracket typed on a line of only indentation goes one level back. |
| *Highlight matching brackets* | on | The bracket at or before the caret and its partner (and in HTML/XML the tag name and its partner's) are highlighted; a bracket without a partner within 3 000 lines is marked in the danger colour. Brackets in strings and comments do not count. |

## Measured

Release, 10 MB of Rust (`assisted_typing_in_a_10_mb_rust_file`): typing words with the popup,
closing brackets and the highlight on — keystroke frames p50 0.36 ms, p95 1.30 ms. An open popup is
narrowed as the word grows (not found again), and a word that found nothing is not searched again
as it grows.

## Not in this Step

Completion from other open documents, fuzzy matching, ranking by use, snippet variables
(`$TM_FILENAME` …) and choices (`${1|a,b|}`), parameter hints from the language definitions
themselves (only API files), the popup, closers and snippets in Vim's Insert mode.
