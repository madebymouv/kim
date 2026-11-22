# Jarak — export

Status: **current** (Step 106, part 1: HTML, RTF, publishing, Copy as HTML / RTF; printing and PDF
are the next part of Step 106 and are **not** in this build) ·
Requirements: `JARAK-FR-141`, `JARAK-SEC-001`, `JARAK-SEC-003`, `JARAK-SEC-004`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Architecture:
[JARAK_ARCHITECTURE §28](JARAK_ARCHITECTURE.md) · Matrix: `PLUG-07`
([JARAK_NOTEPADPP_MATRIX](JARAK_NOTEPADPP_MATRIX.md)) · CLI: `jarak export`
([CLI_CONTRACT](CLI_CONTRACT.md))

Jarak writes exports locally. Nothing is uploaded, nothing remote is fetched, and the document
itself is never changed.

## What each document exports as

| Document | HTML | RTF | Folder |
| --- | --- | --- | --- |
| Markdown (`.md`, `.markdown`, `.mdown`, `.mkd`, or the Markdown language chosen) | Rendered, as the preview shows it, images embedded | Its highlighted text | The rendered page and copies of its images |
| Anything else | Its highlighted text (the language of Step 98) | Its highlighted text | — |

- **Colours** are kim-design's: the light palette, and the dark one when the reader's system is
  dark (HTML only, `prefers-color-scheme`); printed HTML is light. Keywords, tags and headings take
  the purple accent, types the info colour, strings green, numbers and constants amber, comments
  muted — the window's colours (ADR 0103).
- **Line numbers** are an option of highlighted text (`--line-numbers`); a rendered Markdown page
  has none.
- **RTF** is Consolas 10 pt with the same colours; every non-ASCII character is written as `\uN?`,
  so Korean text survives any code page.

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
- PDF (`--to pdf`, `.pdf`) is refused in this build: it is printing to Microsoft Print to PDF and
  comes with the next part of Step 106.

## Not in this build

Printing (the Windows print dialog, Print Now), PDF, the print settings (margins, header, footer,
colours), Copy as HTML / RTF verified against the real Windows clipboard (the formats are tested;
pasting into another program is **not tested** yet).
