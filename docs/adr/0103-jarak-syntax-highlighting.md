# ADR 0103: Jarak's syntax highlighting — its own data-driven line-state lexers

- Status: accepted
- Date: 2026-11-22
- Issue: [#117](https://github.com/madebymouv/kim/issues/117) (Step 98). Extends ADR 0102 (Jarak's
  crate boundaries) and ADR 0092 (the design system in code). Requirements: `JARAK-FR-090`,
  `JARAK-FR-091`, `JARAK-FR-086`, `JARAK-NFR-001`.

## Context

Step 98 asks for fast, incremental highlighting of at least 23 languages (C, C++, C#, Rust, Go,
Java, JavaScript/TypeScript, Python, PowerShell, Batch, Shell, HTML, CSS, JSON, YAML, TOML, XML,
SQL, Markdown with fenced code, INI, Lua, PHP, Ruby), detection by extension and shebang, a
Language menu, user-defined languages (keywords, comments, operators, delimiters, folding markers)
and the import of Notepad++ UDL files, with the evaluation of tree-sitter against syntect/TextMate
grammars recorded.

The candidates, for size, speed, licence and incremental behaviour:

- **tree-sitter**: exact parse trees and incremental re-parsing — but each grammar is generated C
  (a C compiler in the build; this workspace builds with the GNU toolchain and avoids C builds),
  0.2–1 MB per grammar (23 grammars: several MB in one binary that is 6 MB today), grammars of
  mixed licences, and it cannot serve user-defined languages: a UDL would need a second engine.
- **syntect (TextMate/Sublime grammars)**: wide coverage — but a regex engine (onig in C, or
  fancy-regex), binary grammar packs of 1–3 MB, a known cost of tens of ms per thousand lines on
  long or regex-heavy lines, and again no fit for Notepad++'s UDL model.
- **Own data-driven lexers**: every language is a declaration — comments (line, line-start,
  block), strings (escape, multi-line, line-start, maximum length), keyword/type/constant lists,
  operators, variables, a preprocessor prefix, key/section rules — run by three lexer styles (code,
  markup with embedded languages, Markdown with fenced code). The state a line ends in (inside a
  block comment, a multi-line string, a tag, a script, a fence) is all that crosses lines. Notepad++
  itself works this way for its UDLs, so a UDL maps onto the same structure.

## Decision

Jarak highlights with its own lexers in `jarak-core::syntax`:

1. A **`Lang`** is plain data (`serde`); the built-in languages are declared in code, user-defined
   languages are the same structure as JSON in `%APPDATA%\mouv\jarak\languages\`. Notepad++ UDL
   2.x XML is converted into it (name, extensions, case, comments, the eight keyword lists, the
   operators, the delimiters with escapes, the code folding markers; not styles, prefixes, number
   definitions or comment folding — documented in the importer).
2. **Incremental by line states**: the window keeps each document's line-start states as far as
   they were needed (`Highlighter`), lexes only the lines on screen each frame, and drops the states
   from an edit's first line (the document's edit log says where) — a state that comes out the same
   stops the re-lexing.
3. **Colours from kim-design's existing tokens** (no new palette entries): keywords, tags, headings
   and links in the purple accent; types, attributes and variables in `info`; strings and code in
   `success`; numbers, constants and preprocessor lines in `warning`; comments in `text_muted`;
   operators in `text_secondary`; the text itself stays `text_primary` (monochrome base). Wide
   characters are laid out exactly two cells wide so columns line up (Step 96).
4. **No new dependency**: the lexers, the UDL reader and the XML subset it needs are Jarak's own.

## Consequences

- Measured (release): a 10 MB Rust file of 415 280 lines — lexing to its end the first time
  164 ms, a screenful 0.02 ms, re-lexing after an edit 100 lines up 0.04 ms; opening it at its end
  in the window 514 ms; a keystroke frame p95 0.29 ms.
- Highlighting is lexical, not a parse: no semantic colouring (a type used as a value), no Rust raw
  strings `r#"…"#`, no JavaScript regex literals, no YAML block scalars or PowerShell sub-expressions
  inside strings; these show as plain text or as the nearest token. Golden tests pin the constructs
  that are recognised for every language.
- A later Step may add a grammar engine behind the same `Kind` tokens if exactness is needed; the
  view, the colours and the UDL format would not change.

## Rejected

- tree-sitter and syntect for the reasons above (size, a C build or a regex engine, a second engine
  for UDLs).
- New colour tokens in kim-design: they would change every tool's version (ADR 0097) for colours
  the existing tokens already give.
