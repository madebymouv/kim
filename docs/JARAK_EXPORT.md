# Jarak — export

Status: **current** (Step 106) ·
Requirements: `JARAK-FR-140`, `JARAK-FR-141`, `JARAK-SEC-001`, `JARAK-SEC-003`, `JARAK-SEC-004`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Architecture:
[JARAK_ARCHITECTURE §28](JARAK_ARCHITECTURE.md) · Matrix: `PLUG-07`, `FILE-19`, `FILE-20`, `SET-12`
([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)) · CLI: `jarak export`, `jarak print`,
`jarak printers` ([CLI_CONTRACT](CLI_CONTRACT.md))

Jarak writes exports locally. Nothing is uploaded, nothing remote is fetched, and the document
itself is never changed.

## What each document exports as

| Document | HTML | RTF | Folder | Print and PDF |
| --- | --- | --- | --- | --- |
| Markdown (`.md`, `.markdown`, `.mdown`, `.mkd`, or the Markdown language chosen) | Rendered, as the preview shows it, images embedded | Its highlighted text | The rendered page and copies of its images | Rendered, with its local images |
| Anything else | Its highlighted text (the language of Step 98) | Its highlighted text | — | Its highlighted text |

- **Colours** are kim-design's: the light palette, and the dark one when the reader's system is
  dark (HTML only, `prefers-color-scheme`); printed HTML is light. Keywords, tags and headings take
  the purple accent, types the info colour, strings green, numbers and constants amber, comments
  muted — the window's colours (ADR 0103).
- **Line numbers** are an option of highlighted text (`--line-numbers`); a rendered Markdown page
  has none.
- **RTF** is Consolas 10 pt with the same colours, single-spaced with no space after the lines
  (`\pard\sa0\sl240\slmult1`, so WordPad and Word do not add their own paragraph spacing); every
  non-ASCII character is written as `\uN?`, so Korean text survives any code page.

## Images in a Markdown export

- Only images **inside the document's folder** are used, as in the preview (`..` and absolute
  paths are not followed). An untitled document has no folder, so it has no images.
- An image is read only when its first bytes say PNG, JPEG, GIF, BMP or WebP and the file has at
  most 64 MiB; one export embeds or copies at most 256 MiB of images. Images are not decoded.
- **Embedded** (the default): the page holds the image as a `data:` URI and works anywhere on its
  own.
- **Linked** (`--linked`): the page refers to the image by its path relative to the output file
  (`file:///` when they are on different drives).
- **Published** (`--to folder`, or *Publish Markdown to a folder…*): the folder gets
  `<document>.html` and `assets/<image>`. Two different images with the same name become
  `name.png` and `name-2.png`; an image used twice is copied once.
- A **remote** image (`http://`, `https://`) is never fetched: it becomes a link that says
  *remote image, not loaded*. A **missing** image says *image not found*; a refused one says why.

## Safety of the page

- Raw HTML in the document stays text, as in the preview (`<script>` is shown, not run).
- Links keep only `http://`, `https://`, `mailto:`, `#…` and relative targets; `javascript:`,
  `file:` and `data:` links keep their text and lose the link.
- The page carries a Content-Security-Policy (`default-src 'none'; style-src 'unsafe-inline';
  img-src data: file: 'self'`): it loads nothing from the network, even if opened online.

## Never replacing what exists

- The window's **Save dialog** asks before it replaces a file. Publishing into a folder that already
  holds the page or one of its images asks first (*Replace files?*: **Replace** or **Cancel**).
- `jarak export` refuses an existing output (`export_exists`, exit 7, the paths in `details`) unless
  `--overwrite` is given; `--dry-run` checks and writes nothing.
- The document itself is never an output, even with `--overwrite`, and a document's own folder is
  never a publishing target (its images would be written onto themselves): `export_into_source`
  (exit 2).
- Each file is written to a temporary file in its folder, flushed, then moved into place.

## In the window

- **File → Export as HTML…**, **Export as RTF…**, **Publish Markdown to a folder…** (also in the
  command palette). The export takes the text as it is in the window, saved or not; the status bar
  says where it went and what happened to the images.
- **Edit → Copy as HTML** and **Copy as RTF** put the selection (or the whole document) on the
  clipboard with its colours (`HTML Format` or `Rich Text Format`) beside the plain text, as
  Notepad++'s NppExport does. They have no default keys; Settings → Keyboard does not list them
  (only commands with keys are remappable).

## Printing and PDF

- **File → Print…** opens the Windows print dialog: the printer, its settings (copies, paper,
  orientation), all pages, the selection or a page range. **File → Print now** prints on the
  default printer without a dialog. **File → Export as PDF…** prints to *Microsoft Print to PDF*
  into the file the Save dialog names. None of them has a default key (Ctrl+P is Go to file, as
  in Step 101); Settings → Keyboard can give them one — Ctrl+P too, which Go to file then loses.
- A printer that **writes a file** (Microsoft Print to PDF, the XPS writer: port `PORTPROMPT:`)
  gets its file from Jarak's Save dialog, which asks before replacing one — the driver never asks
  on its own, and the command line refuses such a printer without `--output`.
- The job prints the text as it is in the window, saved or not, on a thread; the status bar says
  *Printed n pages on …* or *Saved … (n pages)*, or why it failed.
- **Layout** (`jarak-core::print`, the same for the window and the command line): the page's
  size and resolution come from the printer. Highlighted text is Consolas at the print size, with
  line numbers on a line's first row, tabs expanded, long lines wrapped at a blank (a long word at
  a character). Rendered Markdown uses Segoe UI one point larger: headings (with a rule under the
  first two levels), paragraphs with bold, italic, struck, code and link text, lists and task
  lists, quotes with bars, code blocks on a grey band, tables with their alignment and a header
  band, rules, raw HTML as monospace text, footnotes at the end, and local images at their screen
  size (96 dpi), narrower than the column and shorter than a page. Korean and any character the
  font lacks print in Malgun Gothic.
- **Header and footer**: `left|center|right` templates with `{file}`, `{path}`, `{page}`, `{pages}`,
  `{date}` and `{time}`; defaults `{file}||{date} {time}` and `||{page} / {pages}`; a line
  separates each from the text.
- **Print settings** (Settings → Files → Printing, `print_*` in `settings.toml`): header, footer,
  line numbers, colours (the window's or black on white), the text size (6–24 pt), and the four
  margins (0–50 mm). They apply to the next job.
- Images are decoded only for printing, bounded like the preview (64 MiB of data, 16 384 pixels a
  side, 40 million pixels); one that fails prints as a thin line, the rest of the page prints.

## On the command line

```text
jarak export <file> --output <path> [--to html|rtf|folder] [--line-numbers] [--linked] [--source]
                    [--overwrite] [--dry-run] [--json]
```

- The format follows the output's extension (`.html`, `.htm`, `.rtf`) or `--to`; a folder needs
  `--to folder`.
- `--source` exports a Markdown document's highlighted text instead of rendering it.
- Options that do not fit are refused before anything is read (`usage`, exit 2): `--linked` outside
  a rendered HTML export, `--line-numbers` on a rendered page, `--source` on a non-Markdown file,
  `--to folder` for a non-Markdown file.
- The JSON data names the source, format, output, whether it was rendered, the language, the files
  (path and bytes) and each image's use (`embedded`, `linked`, `copied` with its `published` path,
  `remote`, `missing`, `refused` with its `reason`). Examples:
  `docs/contracts/examples/cli-envelope-v1/jarak-export.json`, `jarak-export-exists.json`,
  `jarak-export-folder-dry-run.json`.
- PDF (`--to pdf`, or a `.pdf` output) prints the document to Microsoft Print to PDF with the
  print settings, into the output file (the same overwrite rules); a machine without that printer
  says `printer_not_found` (exit 3).

```text
jarak print <file> [--printer <name>] [--output <file>] [--pages <a-b>] [--overwrite] [--dry-run]
jarak printers
```

- `jarak print` uses the default printer unless `--printer` names one (`printer_not_found`, exit 3);
  a printer that writes a file needs `--output` (`usage`, exit 2 — it would ask otherwise); an
  existing output needs `--overwrite` (`export_exists`); `--pages 2-3` prints a range; `--dry-run`
  lays the pages out on the real printer and prints nothing. A failure of the spooler is
  `print_failed` (exit 1). No dialog ever opens (`KIM-FR-012`).
- `jarak printers` lists every printer with its port, whether it is the default and whether it
  writes a file. Examples: `jarak-printers.json`, `jarak-print-dry-run.json`,
  `jarak-printer-not-found.json`.

## Verified, and not tested

- Real PDFs printed through Microsoft Print to PDF by `jarak export --to pdf` (a Rust file and the
  rendered Markdown sample) were rendered with Windows' own PDF renderer and reviewed: colours,
  line numbers, Korean through Malgun Gothic, header and footer, headings, table, lists, quote,
  code band, footnote.
- Real desktop, 2026-11-23 ([JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md)): File → Print… opened the
  Windows print dialog; Microsoft Print to PDF asked for a file name and wrote a one-page PDF with
  the header, footer, line numbers, colours and Korean text. *Copy as RTF* pasted into WordPad as
  coloured Consolas code; *Copy as HTML* was read from the real clipboard by another process
  (StartFragment offsets, 14 coloured spans). The first paste showed WordPad's own paragraph
  spacing between the lines, so the RTF now states single spacing.
- **Not tested**: a paper printer (this PC has none).
- Not in Jarak: a print preview, header and footer fonts, printing in two columns.
