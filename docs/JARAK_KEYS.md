# Jarak — keyboard reference

Status: **current** (Step 92; tabs and views since Step 93; find and mark since Step 94; find in files since Step 95; carets and columns since Step 96; line and case keys since Step 97; comment keys since Step 98; folding and Go to since Step 99; completion since Step 100; bookmarks and navigation since Step 101; macros and Run since Step 102) · Requirements: `JARAK-FR-031`, `JARAK-FR-054`, `JARAK-FR-070`…`JARAK-FR-072`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Vim editing: [JARAK_VIM](JARAK_VIM.md)

Every command is in the **command palette** (Ctrl+Shift+P) with its keys, and in the **menu bar**
(hidden by default; Alt alone shows and hides it). The keys below are the defaults; Step 105
makes them configurable. The palette's list and these tables come from the same bindings (a test
checks each label against the key handling, `crates/jarak-gui/src/keys.rs`).

## Files and documents

| Keys | Command |
| --- | --- |
| Ctrl+N | New document |
| Ctrl+O | Open… |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save as… (to another folder: offers to copy the document's images) |
| Ctrl+W | Close the document |
| Ctrl+Tab / Ctrl+Shift+Tab | Next / previous document (in the focused view) |
| Ctrl+1 … Ctrl+9 | The tab of that number (Ctrl+9: the last) |
| Ctrl+Shift+PageUp / PageDown | Move the tab left / right |
| Ctrl+Shift+T | Reopen the last closed file |
| F8 | The other view takes the focus |

## Editing

| Keys | Command |
| --- | --- |
| Ctrl+Z, Ctrl+Y (Ctrl+Shift+Z) | Undo, redo |
| Ctrl+X, Ctrl+C, Ctrl+V | Cut, copy (the line without a selection), paste — text, or an image into a Markdown document |
| Shift+Delete, Ctrl+Insert, Shift+Insert | Cut, copy, paste |
| Backspace, Delete | Delete the character before / after the caret |
| Ctrl+Backspace, Ctrl+Delete | Delete to the start of the word / of the next word |
| Enter | New line (keeps the indentation) |
| Tab, Shift+Tab | Indent, outdent (several selected lines: each line) |
| Insert | Overwrite mode |
| Ctrl+A | Select all |

## Moving and selecting

| Keys | Moves | With Shift: selects |
| --- | --- | --- |
| Left, Right, Up, Down | a character, a line | ✓ |
| Ctrl+Left, Ctrl+Right | a word | ✓ |
| Home, End | the line start / end | ✓ |
| Ctrl+Home, Ctrl+End | the document start / end | ✓ |
| PageUp, PageDown | a page | ✓ |

The mouse: click, drag, double-click (word), triple-click (line), the wheel; Ctrl+wheel zooms.
Ctrl+click on an image reference shows the image in Explorer.

## Find, replace and mark

| Keys | Command |
| --- | --- |
| Ctrl+F | Find (the find panel; a selected text on one line becomes the text to find) |
| Ctrl+H | Replace |
| F3 / Shift+F3 | Find next / previous (also in the panel's boxes) |
| Enter / Shift+Enter | In the find box: find next / previous (the box keeps the focus) |
| Ctrl+F3 / Ctrl+Shift+F3 | Select the word at the caret (or take the selection) and find the next / previous one |
| Ctrl+Alt+F3 / Ctrl+Alt+Shift+F3 | Find the word under the caret (match case, whole word), the find box untouched |
| Ctrl+Alt+I | Incremental search bar (Enter / Shift+Enter next / previous, Esc closes) |
| Up / Down | In the find box: the find history |
| Esc | Close the find panel |
| Alt+A, Alt+R | In the panel: Replace All, Replace (Replace page) |
| Alt+T, Alt+M | In the panel: Count, Mark All (Mark page) |
| Alt+C, Alt+W | In the panel: match case, whole word |
| Alt+P, Alt+B, Alt+I | In the panel: wrap around, backward, in selection |
| Alt+N, Alt+X, Alt+G | In the panel: Normal, Extended, Regular expression mode |
| Alt+D | In the panel: `.` matches newline |
| Ctrl+Shift+F | Find in Files (the panel's Files page: folder, filters, sub-folders, hidden folders) |
| F4 / Shift+F4 | Next / previous search result (opens it with the match selected) |
| F7 | Show / hide the search results panel |

In the results panel a double-click opens a match; a row's menu (right click) opens it or copies
the line, every line or the paths. The Replace in Files preview applies the checked changes with
Enter and closes with Esc. While a box of the find panel has the keyboard, Ctrl+Shift+P, Ctrl+S,
Ctrl+Tab and the other window shortcuts still work.

Marks, bookmarks of marked lines and the five token styles (*Style all occurrences of the token*,
*Style one token*, *Clear style*, *Jump to the next / previous styled text*, *Copy the styled
text*) are in the palette and the Search menu. A selected word highlights its other occurrences on
screen (smart highlighting).

## Lines and case

| Keys | Command |
| --- | --- |
| Ctrl+D | Duplicate the line (or the selection) |
| Ctrl+Shift+L | Delete the line |
| Ctrl+Shift+Up / Down | Move the line up / down |
| Ctrl+J | Join lines |
| Ctrl+I | Split lines at the window's width |
| Ctrl+Alt+Enter / Ctrl+Alt+Shift+Enter | Insert a blank line above / below |
| Ctrl+Shift+U / Ctrl+U | UPPERCASE / lowercase |
| Alt+U / Alt+Shift+U | Proper Case / Proper Case (blend) |
| Ctrl+Alt+U / Ctrl+Alt+Shift+U | Sentence case / Sentence case (blend) |
| Ctrl+Q | Toggle the line comment (in the document's language) |
| Ctrl+K / Ctrl+Shift+K | Comment / uncomment the lines |
| Ctrl+Shift+Q | Block comment (block uncomment: the palette) |
| Alt+0 / Alt+Shift+0 | Fold all / unfold all |
| Alt+1…8 / Alt+Shift+1…8 | Fold / unfold level 1–8 |
| Ctrl+Alt+F / Ctrl+Alt+Shift+F | Fold / unfold the current level (the innermost fold around the caret) |
| Click the fold margin | Fold or unfold that fold (the caret stays) |
| Ctrl+G | Go to a line or a character offset |
| Ctrl+B / Ctrl+Alt+B | Go to the matching bracket / select everything between them |
| Ctrl+Space | Complete the word (snippets, functions, keywords, words) |
| Ctrl+Enter | Complete from the document's words |
| Ctrl+Alt+Space | Complete a path |
| Ctrl+Shift+Space | Parameter hint (Alt+Up / Alt+Down: the previous / next overload) |
| In the completion popup | Up/Down choose; Tab / Enter take the chosen entry (a popup that opened by itself: only after choosing, unless set otherwise — [JARAK_ASSIST](JARAK_ASSIST.md)); Esc closes |
| In a snippet | Tab / Shift+Tab: the next / previous tab stop; Esc ends it |
| Ctrl+F2 | Toggle the bookmark (every caret's line) |
| F2 / Shift+F2 | Next / previous bookmark |
| Alt+H | Hide the selected lines (a click on the mark above shows them again) |
| Alt+Left / Alt+Right | Back / forward in the navigation history |
| Ctrl+P | Go to file (open documents and the workspace's files) — Notepad++ prints with it; Jarak's *Print…* and *Print now* (Step 106) have no default keys: Settings → Keyboard can give *Print…* Ctrl+P (Go to file then loses it) |
| Ctrl+Shift+R | Start / stop recording a macro |
| F9 | Play back the recorded macro (Notepad++'s Ctrl+Shift+P is Jarak's command palette) |
| F5 | Run… (a command line, shown exactly before it runs — [JARAK_MACROS](JARAK_MACROS.md)) |
| F11 | Full screen on/off (Step 110) |
| F12 | Post-it mode on/off: no window frame, menu, tab or status bar (Step 110; the palette, Ctrl+Shift+P, still works) |

The palette and the Edit menu have the rest: removing duplicate, consecutive duplicate and empty
lines, reverse, randomize, the sorts (lexicographic, ignoring case, locale, integers, decimals,
length; ascending or descending), iNVERT and ranDOm case, the blank operations, Base64 / URL /
quoted-printable / hex, the digests, the date and time, and copying paths.

## Carets and columns

| Keys | Command |
| --- | --- |
| Ctrl+click | Add a caret (on an existing one of several: remove it); Ctrl+drag adds a selection |
| Ctrl+Shift+D | Select the word at the caret, then add its next occurrence each time |
| Alt+F3 | Select every occurrence |
| Esc | Back to one caret |
| Alt+drag | Column selection |
| Alt+Shift+Up / Down / Left / Right | Column selection from the caret |
| Alt+C | Column Editor (text or incrementing numbers into every row) |

The palette adds Notepad++'s four multi-select variants (ignore case and whole word, match case
only, whole word only, match case and whole word) for *next* and *all*, *skip this occurrence*,
*Begin/End Select* (also in column mode) and *Virtual space*. Ctrl+D is *Duplicate line*
(Step 97), as in Notepad++.

## View and Markdown

| Keys | Command |
| --- | --- |
| Ctrl+E | Markdown source / preview |
| Ctrl+Shift+E | Source and preview side by side |
| Alt+Z | Word wrap |
| Ctrl+Plus, Ctrl+Minus, Ctrl+0 | Zoom in, zoom out, reset |
| Ctrl+Shift+P | Command palette |
| Ctrl+, | Settings (a window of its own; Step 105) |
| Alt (alone) | Show / hide the menu bar |

In the preview: Up / Down a block, PageUp / PageDown (Space) five blocks, Home / End, Esc back to
the source; Ctrl+click opens a web or mail link, Ctrl+click on a picture shows it in Explorer.

## Vim editing

With *Editing mode: Vim* (palette or menu), the keys of [JARAK_VIM](JARAK_VIM.md) apply in Normal,
Visual and Insert mode; every Ctrl key above that Vim does not use (Ctrl+S, Ctrl+E, Ctrl+Shift+P,
Ctrl+O, Ctrl+Tab, …) keeps working, and Ctrl+V pastes in Insert mode.
