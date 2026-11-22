# Jarak — security and threat model

Status: **current** (Step 109) · Requirements: `JARAK-SEC-001`…`JARAK-SEC-009`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Platform rules: [SECURITY](SECURITY.md) · Tests:
`crates/jarak-core/tests/security.rs` and the tests named below

## What is protected

1. **The user's documents.** They are never changed without an explicit action and never lost by
   a crash or a failed save.
2. **The user's machine.** Opening a hostile file never runs code, never reaches the network and
   never exhausts memory.
3. **Privacy.** Nothing leaves the machine. Clipboard and screen are read only on a user action.

## Who or what is hostile

- **A file the user opens:** malformed encodings, binary content, huge or deeply nested Markdown,
  images built to exhaust memory (decompression bombs), asset references that point outside the
  document's folder, links with `javascript:` targets, raw HTML.
- **State files on disk:** a damaged or newer `settings.toml`, `hotkeys.json`, `session.json`,
  recovery snapshots, user languages, snippets.
- **An extension** the user installed: its manifest, its files, the program it runs.
- **A command line or interop request** from a script or another tool.

## Threats, mitigations and the tests that check them

| Threat | Mitigation | Evidence |
| --- | --- | --- |
| A save corrupts or loses the document | Atomic save: a temporary file in the folder, flushed, then `ReplaceFileW`; a failure leaves the original (`JARAK-SEC-002`) | Step 87 `file::tests`, Step 104 recovery |
| A malformed or binary file is silently changed on save | Decoding is checked for losslessness; a lossy or binary file opens read-only and is never written back (`JARAK-SEC-001`) | `security.rs` `random_bytes_decode_safely_and_lossless_means_exact` (600 random buffers in four encodings: lossless means byte-exact) |
| Hostile Markdown crashes or hangs the preview, export or printing | `pulldown-cmark` with a native render model; raw HTML shown as text; exports keep only `http(s)`/`mailto`/`#`/relative links and carry a CSP | `security.rs` `hostile_markdown_renders_safely` (300 random documents, 500 nested quotes, 2000-column tables, 200 000-character lines, 20 000 brackets) |
| Highlighting breaks on odd input | Lexers are total: every character ends up in exactly one run | `security.rs` `random_text_highlights_in_every_language` (every built-in language) |
| An image exhausts memory (a decompression bomb) or crashes the decoder | Bytes (64–128 MiB), sides (16 384) and pixels (40 M) are checked before decoding; decoding runs off the UI thread under `catch_unwind` (`JARAK-SEC-004`) | `security.rs` `image_bombs_and_garbage_are_refused` (a 100 000 × 100 000 PNG header, 60 truncated or random images); Step 90 image tests |
| An asset reference reads or writes outside the document's folder | `assets::resolve` follows only relative references inside the folder: no `..`, absolute, drive-relative, UNC, percent-encoded traversal or remote paths; new assets get new names and never replace a file (`JARAK-SEC-005`) | `security.rs` `asset_references_never_leave_the_documents_folder`; Step 91 asset tests |
| A remote image or link reaches the network | Remote images are never fetched (links instead); links open only on an explicit click through the shell (`JARAK-SEC-003`, `JARAK-SEC-006`) | Step 91 preview tests, Step 106 export tests |
| An external command runs with attacker-chosen arguments | Run and saved commands are user-made; `$(…)` variables are quoted; nothing runs from a file's content (`JARAK-SEC-006`) | Step 102 `macros` tests |
| An extension runs code without consent | Commands need declared permissions and the user's grant for exactly that manifest (SHA-256); only `.exe` files of the extension or System32 programs, no scripts or shell; a Windows job, an empty folder, a minimal environment, time and output limits (`JARAK-SEC-007`) | `security.rs` `hostile_extension_manifests_are_refused`; Step 108 `extensions::tests` and window flows |
| A damaged or hostile state file crashes Jarak or is overwritten | Every state file is read with defaults, an error or safe mode; a damaged session falls back to its backup and is kept as `.broken`; reading never rewrites a file | `security.rs` `corrupt_state_files_are_read_safely` (120 random files each); Step 104 session tests; Step 105 hotkeys safe mode |
| Exports or prints replace the document or another file | Never the document itself, an existing output only with confirmation or `--overwrite`, temporary file then rename | Step 106 `export::tests`, CLI tests |
| Headless commands open windows or prompts | Only `jarak open` and `document.open` open a window; a file-writing printer needs `--output` (`KIM-FR-012`) | Step 106 and 107 CLI tests |
| Telemetry or update checks | None; nothing is downloaded (`KIM-SEC-001`, `JARAK-SEC-003`) | deps-check: no network crates in the headless packages |

## What Jarak does not protect against

- **A granted extension program** runs with the user's Windows rights. It can read and write the
  user's files and use the network; the grant is the user's decision
  ([JARAK_EXTENSIONS](JARAK_EXTENSIONS.md)).
- **Another program changing a file** while Jarak has it open. Jarak notices and asks (Step 104),
  but cannot prevent the change.
- **Malware already running as the user**, or an administrator, can read anything Jarak can.
- **A file shared on a network drive** is read like a local file. Jarak adds no network protocol
  of its own.
