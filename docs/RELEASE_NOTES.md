# Release notes

## Jarak 0.0.35 — large files and contrast themes (preview, not yet published)

- **Status:** draft (2026-11-23, #128).
- **Windows contrast themes:** with High Contrast on, Jarak draws with the theme's system colours
  only (text, borders, the highlight for focus, selection and keywords; no lilac accent) and
  switches when it is turned on or off ([JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md)).
- **Fixed:** an edit in a large plain-text document no longer starts a structure search that
  copied every line (1000 MiB: about 5 s of CPU after each pause in typing).
- **Measured** on the real desktop: 100 MiB opens in 0.6 s, 1000 MiB in 2.9 s, both at 2.3× the
  file in memory, idle at 0 % CPU ([JARAK_PERFORMANCE](JARAK_PERFORMANCE.md)).

## Jarak 0.0.34 — after the real-desktop acceptance run (preview, not yet published)

- **Status:** draft (2026-11-23). The checks that needed the real desktop were run with test
  windows and sandboxed profiles ([JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md)): installing through
  Kim Manager, the Open and Save As dialogs, a real file drop, window modes and `--position`, the
  context menu and dragging text, the real clipboard with WordPad, printing to PDF, the *Open
  with* offer and its exact removal, Help, the sample extension's grant, a Noeul capture through
  Kim into a note, `jarak open file:line` from a Subak terminal, and uninstalling.
- **Fixed** (found by that run): the Open dialog takes several files at once (Ctrl or Shift in
  its list); the Markdown preview — where the read-only user guide opens — runs the palette, tab,
  settings, printing and window-mode commands instead of refusing them; *Copy as RTF* pastes
  single-spaced (WordPad had added its own paragraph spacing); Settings → Keyboard centres the
  *Off* of a command without keys on its row.
- **Notepad++ parity:** 265 of 285 built-in features supported, 9 partial with their gaps named,
  11 not intended; the 16 Jarak additions all supported
  ([JARAK_PARITY_REPORT](JARAK_PARITY_REPORT.md)).

## Jarak 0.0.33 — the fourth Kim tool (preview, not yet published)

- **Status:** draft (Step 111, 2026-11-22). Installable from the local mirror through Kim
  Manager; published with the other tools once [RELEASE_READINESS](RELEASE_READINESS.md)'s
  blockers are cleared. Package: `cargo xtask package jarak` (programs, capabilities, licences,
  third-party notices, the user guide under `docs\`).
- **Platform:** Windows 10 22H2 x64 tested; Windows 11 not tested; no Linux.
- **What it is:** a native text editor for plain text, code and Markdown ([JARAK_GUIDE](JARAK_GUIDE.md)):
  tabs and two views, sessions and crash recovery, encodings without loss, Notepad++'s keys and
  line tools, several carets and columns, find and replace with regular expressions and Find in
  Files with an undo journal, 38 languages with folding and completion, Vim keys, the Markdown
  preview with pasted screenshots kept as local files, printing and PDF, HTML/RTF export,
  extensions with granted permissions, and `jarak.exe` — a headless CLI for scripts and agents.
- **Notepad++ parity:** 285 built-in features reviewed — 253 supported, 21 partial with their gaps
  named, 11 not intended with reasons ([JARAK_PARITY_REPORT](JARAK_PARITY_REPORT.md)). Notepad++
  plugin DLLs are not supported.
- **Safety:** no automatic install or update, no account, no telemetry; the source of a document is
  never rewritten by the preview or an export; *Open with* is offered only when asked, per user,
  never as a default program, and taken back exactly.
- **Known limitations:** real-desktop checks still to run with test windows (window modes,
  `--position`, Explorer's Open with list, the real clipboard's HTML/RTF, the print dialog —
  [JARAK_ACCEPTANCE](JARAK_ACCEPTANCE.md)); 100 MB / 1 GB measurements, High Contrast and Narrator
  ([JARAK_PERFORMANCE](JARAK_PERFORMANCE.md), [JARAK_ACCESSIBILITY](JARAK_ACCESSIBILITY.md));
  right-to-left text.

## Kim 0.0.1 and Subak 0.0.1 — preview (not yet published)

- **Status:** draft (Step 40, 2026-11-12). Nothing is published until the blockers of
  [RELEASE_READINESS §3](RELEASE_READINESS.md#3-blockers-and-known-limitations) are cleared;
  packages can be built with `cargo xtask package` ([RELEASING](RELEASING.md)).
- **Platform:** Windows 10 22H2 x64 tested; Windows 11 x64 not tested; no ARM64, no Linux yet.

### Kim 0.0.1

- A signed catalog of Kim tools (Ed25519, rollback protection) and an explicit installer:
  `install`, `update` (side by side; never stops running programs), **`rollback`** (new),
  `uninstall` (keeps user data unless `--purge-data`), all with `--dry-run` and `--json`.
- Never installs or updates by itself; offline mode; no telemetry.
- Tool discovery and capability interop: `tools list|describe|find|invoke`
  ([INTEROP](INTEROP.md)).
- Kim Manager window; Kim's own shortcuts (`hotkeys`).

### Subak 0.0.1

- A detached per-user Host owning Terminals, windows and panes (ConPTY); CLI and window
  clients; Terminals survive closing every window ([SUBAK_CLI](SUBAK_CLI.md), [SUBAK_GUI](SUBAK_GUI.md)).
- tmux-style commands, prefix keys, layouts, buffers, hooks ([TMUX_COMPATIBILITY](TMUX_COMPATIBILITY.md)).
- Claude Code status per pane from its hooks; crash recovery and cold restore with Claude resume
  ([RECOVERY](RECOVERY.md)).
- SSH through Windows' OpenSSH with secrets in Credential Manager, SFTP with jobs and resume,
  jump hosts, forwards, proxies; Telnet (off by default) and serial panes.
- An MCP server for agents, read-only unless granted ([MCP_GUIDE](MCP_GUIDE.md)).

### Known limitations

[RELEASE_READINESS §3](RELEASE_READINESS.md#3-blockers-and-known-limitations), the
[Tabby parity report](TABBY_PARITY_REPORT.md) (no full Tabby parity; follow-up #98) and the
unsupported rows of [TMUX_COMPATIBILITY](TMUX_COMPATIBILITY.md). Claude Code integration reads
Claude's hooks; Claude features beyond status and resume are not provided.
