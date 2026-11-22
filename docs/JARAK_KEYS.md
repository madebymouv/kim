# Jarak — keyboard reference

Status: **current** (Step 92; tabs and views since Step 93; find and mark since Step 94) · Requirements: `JARAK-FR-031`, `JARAK-FR-054`, `JARAK-FR-070`…`JARAK-FR-072`
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

Marks, bookmarks of marked lines and the five token styles (*Style all occurrences of the token*,
*Style one token*, *Clear style*, *Jump to the next / previous styled text*, *Copy the styled
text*) are in the palette and the Search menu. A selected word highlights its other occurrences on
screen (smart highlighting).

## View and Markdown

| Keys | Command |
| --- | --- |
| Ctrl+E | Markdown source / preview |
| Ctrl+Shift+E | Source and preview side by side |
| Alt+Z | Word wrap |
| Ctrl+Plus, Ctrl+Minus, Ctrl+0 | Zoom in, zoom out, reset |
| Ctrl+Shift+P | Command palette |
| Alt (alone) | Show / hide the menu bar |

In the preview: Up / Down a block, PageUp / PageDown (Space) five blocks, Home / End, Esc back to
the source; Ctrl+click opens a web or mail link, Ctrl+click on a picture shows it in Explorer.

## Vim editing

With *Editing mode: Vim* (palette or menu), the keys of [JARAK_VIM](JARAK_VIM.md) apply in Normal,
Visual and Insert mode; every Ctrl key above that Vim does not use (Ctrl+S, Ctrl+E, Ctrl+Shift+P,
Ctrl+O, Ctrl+Tab, …) keeps working, and Ctrl+V pastes in Insert mode.
