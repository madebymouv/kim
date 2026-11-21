# Jarak — Vim editing

Status: **current** (Step 92, [ADR 0102](adr/0102-jarak-fourth-kim-tool.md)) · Requirements:
`JARAK-FR-050`…`JARAK-FR-054` ([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Design:
[JARAK_ARCHITECTURE §14](JARAK_ARCHITECTURE.md) · Keys: [JARAK_KEYS](JARAK_KEYS.md)

Jarak edits like a Windows editor by default (**Standard editing**). **Vim editing** is an option
for people who know Vim: modal editing with motions, operators, text objects, counts, registers,
`.` and a `:` command line. It is a layer over the same editing commands Standard editing uses,
so undo, saving, the Markdown preview and pasted images work the same in both.

## Turning it on and off

- **On:** Ctrl+Shift+P → *Editing mode: Vim* (or the menu bar: Alt → Edit → *Editing mode:
  Vim*). Jarak remembers it in `%APPDATA%\mouv\jarak\settings.toml` (`editing_mode = "vim"`).
- **Off:** Ctrl+Shift+P → *Editing mode: Standard*, or click the mode word at the left of the
  status bar (`NORMAL`, `INSERT`, …).
- In Standard editing Esc does nothing special and there is no mode: nobody gets stuck.

## Modes

The status bar starts with the mode, followed by the keys of an unfinished command (`NORMAL  2d`).

| Mode | Shown | How you get there |
| --- | --- | --- |
| Normal | `NORMAL` | Esc from any other mode; a block caret |
| Insert | `INSERT` | `i a I A o O`, `c` with a motion, `s S C` |
| Replace | `REPLACE` | `R` |
| Visual | `VISUAL` | `v` |
| Visual line | `V-LINE` | `V` |
| Visual block | `V-BLOCK` | Ctrl+V (one range per line; full column editing is Step 96) |
| Command line | `COMMAND` / `SEARCH` | `:`, `/`, `?` (typed at the left of the status bar) |

**Korean and other IMEs.** In Insert and Replace mode the IME composes as usual (한글 typed with
the 2-set keys, committed with Space or a non-letter). In Normal and Visual mode the IME is
switched off for the window, so `j` moves down even when the Hangul key is on — no need to switch
to English to give a command (checked on the real desktop, [JARAK_ARCHITECTURE §14](JARAK_ARCHITECTURE.md)).

## Normal mode

**Motions** (all take a count): `h j k l` (and the arrow keys), `w b e ge` (words), `W B E gE`
(WORDs, anything between blanks), `0 ^ $`, `gg G` (`5G` line 5), `f F t T` + a character and
`; ,` to repeat, `%` (matching bracket), `{ }` (paragraphs), `H M L` (top, middle, bottom of the
screen), `n N` (the last search), `* #` (the word under the cursor), Ctrl+D / Ctrl+U (half a
page), Ctrl+F / Ctrl+B (a page).

**Operators** with a motion, a text object, or doubled for whole lines (`dd`, `cc`, `yy`, `>>`,
`<<`, `g~~`, `guu`, `gUU`):

| Operator | Does |
| --- | --- |
| `d` | delete |
| `c` | change (delete and Insert) — `cw` on a word changes to its end, as in Vim |
| `y` | yank (copy into a register) |
| `>` `<` | indent / outdent the lines by one tab |
| `g~` `gu` `gU` | toggle case, lower case, upper case |

**Text objects** (`i` inner, `a` with the surroundings): `iw aw` word, `iW aW` WORD, `i" a"`
`i' a'` `` i` a` `` quotes, `i( a(` / `ib ab`, `i[ a[`, `i{ a{` / `iB aB`, `i< a<` brackets,
`it at` HTML/XML tags, `ip ap` paragraphs. Counts work: `d2w`, `2dd`, `3x`, `2ci(`.

**Other commands:** `x X D C s S Y`, `r` + a character (`3rx`), `R`, `J` (join lines), `~`,
`p P` (put after / before; lines go below / above), `o O`, `i a I A`, `u` undo, Ctrl+R redo, `.`
repeat the last change (`3.` with a new count).

**Undo:** every Vim change is one undo step — `cw`, the typed word and Esc undo together; `3dd`
is one step.

## Registers

`"` + a letter before a command: `"ayy`, `"ap`.

| Register | Holds |
| --- | --- |
| `""` (no prefix) | the last delete, change or yank |
| `"0` | the last yank |
| `"a`–`"z` | named registers; `"A`–`"Z` append to them |
| `"_` | nothing (a delete that keeps the registers) |
| `"+` and `"*` | the Windows clipboard (a text ending in a line break pastes as lines) |

The unnamed register is not the clipboard (as in Vim); Ctrl+C / Ctrl+V keep working on the
clipboard in every mode, and Ctrl+V pastes in Insert mode.

## Visual modes

Move with any motion; `o` goes to the other end. Then `d x y c s > < ~ u U J p r`+char; `D X Y C
S R` on whole lines; `iw`, `i(` … extend the selection to a text object; `:` starts a command for
the selected lines (`:'<,'>`). In Visual block mode `I` and `A` type on every line at the block's
left or right edge (one undo step), `c` changes the block on every line.

## The command line

| Command | Does |
| --- | --- |
| `:w` | save (`:w name` writes a copy; an untitled document is saved under that name) |
| `:q` / `:q!` | close the document (refused with unsaved changes unless `!`); the last one closes the window |
| `:wq`, `:x` | save and close |
| `:e name` | open a file (in a new tab); `:e!` reloads the document from disk, giving up changes |
| `:s/old/new/[g][i][I][n]` | substitute on the line; with a range `:%s`, `:2,5s`, `:'<,'>s`, `.`, `$`; any delimiter (`:s#a/b#c#`) |
| `:noh` | hide the search highlight until the next search |
| `:set wrap` / `nowrap`, `ts=4`, `ic` / `noic`, `scs` / `noscs`, `hls` / `nohls` | options |
| `:42` | go to line 42 |
| `/text`, `?text` | search forward / backward (the matches are highlighted); `n N` repeat |

**Patterns** use Vim's "magic" syntax — `\(…\)` groups, `\|` alternatives, `\+ \? \=`, `\{n,m}`,
`\<…\>` word boundaries, `.` `*` `[…]` `^` `$` `\d \w \s` — or `\v` "very magic" with the
groups and alternatives unescaped. `\c` anywhere in a pattern ignores case for it. In the replacement, `&` is the
match, `\1`…`\9` the groups, `\r` a line break. They are translated to Jarak's regular
expressions, so a few rare things are not available: back references inside the pattern
(`\(a\)\1`), `\%V`, `\zs` and the confirm flag `c` (refused with a message).

## What is not there (yet)

Macros (`q`), marks (`m`, `` ` ``), `:g`, `:normal`, folds, the jump list, Vim's own `:help`, and a
`vimrc` are not part of Step 92. The command palette (Ctrl+Shift+P) lists every Jarak command with
its keys in both editing modes.
