# Development checkpoint

Sequential workflow ([ADR 0055](docs/adr/0055-single-branch-sequential-development.md)): one
session, one folder, one branch (`main`), one Step at a time. Update this file at every Step
transition. Resume prompt: *"Continue the Kim sequential development mission from the latest
checkpoint."*

## Now

- Branch: `main` · last pushed commit: see `git log -1 origin/main`
- Current work (maintainer assignment 2026-11-22, ADR 0102): **the Jarak roadmap, Steps 84–111 = issues #103–#130** (registered 2026-11-22 from the roadmap; the prepared zip package was not on this PC, bodies rebuilt in the session scratchpad `jarak-issues/`). Jarak Steps come first; older issues stay open unchanged. Done: Step 84 (#103, closed, b5ec2f0: docs JARAK_PHILOSOPHY, JARAK_REQUIREMENTS, JARAK_NOTEPADPP_MATRIX checked by docs-check); Step 85 (#104: crates `jarak-core`/`jarak`/`jarak-gui`, CLI `version`/`capabilities`/`paths`/`open`, window shell, single-instance hand-off, catalog v11 + package + fourth Kim Manager tile, JARAK_ARCHITECTURE). Step 86 (#105: the text model in `jarak-core`, b093acd); Step 87 (#106: files, encodings, atomic save, `jarak info`; stays open until Step 88 shows the window's dialogs and the unsaved-changes prompt). Step 88 (#107: the editor window; #106 closes with it). Step 89 (#108: Markdown preview). Step 90 (#109, closed, d6ba078: Ctrl+V images, the MVP journey passes on the real desktop). Step 91 (#110, closed, 8036701: pictures, assets). Step 92 (#111, closed, 4155986 + fa00926: Vim, the palette, the menu bar). Step 93 (#112, closed, f467f76: tabs, two views, the session, recent files, file operations, the workspace). Step 94 (#113, closed, 92146b0 + 294c3ca + 88ca2fa: find, replace, regular expressions, marks, styles, incremental search, history, `jarak find`). Step 95 (#114, closed, 682be93: Find and Replace in Files with the journal, the results panel, Find All in documents, `jarak replace`/`journal`). Step 96 (#115, closed, 8c03f81: multiple carets, occurrences, column selections, virtual space, the Column Editor). Step 97 (#116, closed, 5e4f48e: line operations, sorting, case, blanks, encodings, digests, dates, paths). Step 98 (#117, 86a2b86: syntax highlighting, languages, user languages, comments; ADR 0103) Step 99 (#118, b2f9a4b: folding, the Function List, Go to, matching brackets) Step 100 (#119, 75a1203: completion, hints, snippets, typing aids; also the Step 99 one-worker fix) Step 101 (#120, cd50943: bookmarks, change history, hidden lines, Document Map, navigation history, quick open, Summary) Step 102 (#121, 4a2dd1e: macros, Run, the browser entries) and Step 103 (#122, 6068a6a: the Encoding menu, EOL conversion, Show Symbol, binary content) — pushed; their real-desktop checks, e2e first runs and the jarak UI audit ran 2026-11-22 while the maintainer was away (see the verification log), with two fixes (the Save/Run dialogs' focus; the Function List title and Go to position recorded for the audit). Steps 98–103 closed after those checks (56af7bd). Step 104 (#123: files changed on disk, recovery snapshots and the offer after a crash, the session's backup) — real kill-and-recover and external-change checks passed on the desktop. 2026-11-22 12:38: the PC blue-screened (0x0A, a kernel driver fault; no low-memory or WHEA events) during a full-load release build; since then every heavy command runs through `../tools/memory/limit.py` (12 GB, CPU 50 %, below normal, one at a time). limit.py now runs without a CPU hard cap and at normal priority (both made a Subak GUI responsiveness test time out), 8 compilers and 8 test threads at once. Step 104 closed (4b278bc). Now: Step 105 (#124) — part 1 (the Settings window, live settings, `jarak settings`, schema; 5938aab) and part 2 (the shortcut mapper on kim-hotkeys, `jarak hotkeys`) pushed; #124 stays OPEN for one real check: the Keyboard section's screenshots after part 2 (visible windows; the maintainer is back at the PC since ~16:45).
- Previous work: maintainer task 2026-11-16 "Ddalgak — Implement and Verify 12 Real-World Keyboard Automation Requirements" (ADR 0101: CapsLock as a modifier and kept off, CapsLock+Backspace pauses/resumes, CapsLock+/ opens Settings, media/volume/Settings actions, symbols in the key picker, sentences; 18 mappings made in the GUI from an empty profile and verified with real input, docs/DDALGAK_KEYBOARD_TASK.md). Before it: ADR 0100 (rules-first main window, Settings windows) and ADR 0099.
- Real-UI checks (ADR 0088, user 2026-11-13: full real desktop testing with save/restore of settings and clipboard; never the user's open windows): driver and keep-awake helper in the session scratchpad. Closed with real evidence: #99, #55, #56, #58, #14, #15, #16, #18, #19, #20, #57, #61, #62, #64, #66, #68, #77, #79, #80, #81, #82, #83; open with exact blockers: #8, #31 (sleep/sign-out/reboot), #63 (second monitor, HDR). Fixed on the way: Ctrl+click selection, snapping edges, zoom drift, pin quick-save format (Noeul); term_bench lone Alt, tray double toggle, clipboard open retry, mark-mode wide steps, question width (Subak); smoke scripts (tab process, glued rows). Next: Ddalgak issues still open only for physical-input or environment blockers (reported on each issue): #71/#78 emergency chord (+ reboot), #73 remaps, #74 hotstrings, #75 website in the maintainer's browser, #76 held button + 2nd monitor, #84 elevated window + sleep; #87 waits on those. #99 CLOSED (all 28 planned rows supported or partial with gaps named; APP-03 is Linux, #69). Now #98 (Tabby parity): T2 done (f2980b3), T3 done (ba75330, a8327c2, 4cd5ae1); T1 batches e42fbc8, eef87a9, 06ebf16, 187fc08, dd1ddda, 8141171; next rest of T1, then T4, T5, T6; then #97, #44 Windows part
- Last delivered Step: Step 82 (#87, Ddalgak hardening; OPEN for DDALGAK_ACCEPTANCE and the release). Step 31 (#31)
  possible, left OPEN: lock/sleep/hibernate/reboot/power rows need the maintainer
- Uncommitted changes: none expected between Steps
- Root `kim.exe` (ADR 0089): at wrap-up also copy `kim-manager.exe` to the root and run `cargo xtask local-mirror --key <catalog key>` so the double-click works with the current tools
- Next action: #98 T4 rest (CORE-38, SHELL-14, SHELL-18, SHELL-23, SHELL-24, SHELL-26, CFG-07, CFG-08, PLUG-05), T1 rest (CORE-09, CORE-28, CORE-47), then T5, T6; since 2026-11-14 NO real mouse/keyboard/screen (the user is at the PC): off-screen windows and tests only, real-desktop checks are written as not tested; then #97 (tmux) and #44. Real-desktop drags must check the end point (memory real-drag-end-point). Maintainer-check and Linux issues stay open. The repository moved to madebymouv/kim (old URL redirects).
- Loop directive (user, 2026-11-11): keep implementing until `gh api --paginate
  repos/madebymouv/kim/issues?state=open` (no PRs) is 0; re-query after every issue

## Verification log (latest first)

- 2026-11-22: Step 105 (#124) part 2 — jarak-core 165 tests (hotkeys 2: manifest valid without conflicting defaults; set, default replaced, clash refused and taken, dry run, disable, reset, reset all, invalid file never rewritten); jarak CLI 4 (hotkeys list/show/set/disable/reset/check with codes); jarak-gui unit 33 (key names, shipped manifest = commands) and window flows 117 (3 new: remapped command, Keyboard capture / refusal / Take it / Reset all, CLI change on focus); schema-check PASS (manifest example `kim-hotkey-manifest-v1/jarak.json`); e2e step105 extended (hotkeys CLI + Keyboard) first run FAIL (Reset button below the visible list → user's changes listed first) then PASS. Activity detected ~16:45: real input and visible windows stopped; pending: the Keyboard section's real screenshots.

- 2026-11-22: Step 105 (#124) part 1 — jarak-core 163 tests (settings 6: registry, values, the file kept, schema `jarak-settings-v1` + example, the reference; spaces; image folder); jarak CLI 3 (settings list/get/set/reset, dry run, codes); jarak-gui window flows 114 (10 new for Settings); ui-audit 275 runs 0 failures (9 Settings sections; screenshots reviewed en dark / ko light); e2e step105 first runs FAIL (test assumption; then the line numbers at the user's font size measured against the UI scale → recorded as the user's size) then PASS; schema-check 23 schemas PASS. Not done yet: remapping shortcuts (part 2).

- 2026-11-22: Step 104 (#123) — jarak-core 156 tests + `recovery_kill.rs` (a real child process killed after writing snapshots, found by the next scan with the real Windows process check); jarak-gui window flows 105 (12 new: asking and undoable reload, silent reload, keep mine / reload / compare, deleted file, a save refusing an unseen change, monitoring, the window's own checks, snapshots offered and restored, not now and discard twice, a saved document leaving its snapshot and a normal exit keeping none, a damaged session, the UI audit of both dialogs); schema `jarak-recovery-v1` (schema-check 22 schemas PASS); e2e step104 first run FAIL on the test's assumption (one document hides the tab bar), then PASS; real desktop: a test instance killed after real typing (a file and an untitled document) → the next start offered both and Restore gave back the exact text (its own snapshot and the saved file hold it); another program's change noticed in 0.7 s / 1.1 s, Reload + Ctrl+Z + Save gave back the original, Compare opened `a.txt (on disk)`, Keep mine saved mine, a deleted file written again. A first real run right after ~28 minutes of idle saw the window stop after one typed character (the display was likely asleep); it did not recur in four runs once the display was awake. Not done: backup on save (SET-14 partial); a Windows restart itself not tested.

- 2026-11-22: Steps 98–103 real desktop, e2e first runs, UI audit (the maintainer away, real input allowed). Real desktop (test-started windows, sandboxed profiles, scripts in the session scratchpad `j88r/real98.ps1`, `real102.ps1`, `real103.ps1`, `real103b.ps1`): 98 Rust colours, status → 26 `Language:` entries, Python and back; 99 Alt+0 13 → 7 drawn lines, Alt+Shift+0 back, Function List click → line 6; 100 `ap` popup, Down+Enter inserts `apricot`, Esc closes, Ctrl+Space single answer inserted; 101 Ctrl+F2 / F2, Document Map, Ctrl+P open, Alt+Left back; 102 macro recorded with real keys, F9 ×2, one Ctrl+Z = one run, until the end stops (Played: 3), saved by typing its name, Run line expanded, nothing run on Esc; 103 Shift-JIS opens read-only as EUC-KR, Reinterpret as Shift-JIS (bytes untouched), status → `Encoding:` entries, UTF-8 / UTF-16 LE BOM / LF saved as those bytes, `LF` symbol; binary content through the real Windows clipboard (`Jarak Binary Text`, pasted into another document and saved with the NUL; the maintainer's clipboard text saved and put back). Fixed: the macro Save and Run dialogs now focus their first field (typing went nowhere); the Function List title and the Go to position line are recorded in the geometry report (`functions:title`, `goto:where`; the step99 e2e first run failed on them). e2e first runs step98–103: PASS (step99 after the fix). `cargo xtask ui-audit --only jarak`: 220 runs, 0 failures, 0 warnings. Found on the way: the maintainer's AutoHotkey script (Bepo.ahk) takes LShift+R/X/C/V/T/Q chords even when injected; the test's Ctrl+Shift+R presses had changed its window-pin flag, restored with its own reset (LShift+Q); real tests now send Shift+letters with Right Shift.

- 2026-11-22: Step 103 (#122) — jarak-core 145 tests pass (+2 ignored; file 4 new: every character set round trip, BOM reinterpretation, lossy detection, Reinterpret from disk; document EOL conversion); jarak-gui lib 31 (control names) and window flows 93 (5 new, one under the UI audit's rules headless; one older test now expects LF where its text held only LF); the real `jarak info --fallback` reads Python-encoded Shift-JIS, EUC-KR, KOI8-R, Windows-874, GB18030, Windows-1256 losslessly; ui-audit views symbols/encoding-question added, NOT run; e2e step103 compiled; the real Windows clipboard (binary content) not tested.

- 2026-11-22: Step 102 (#121) — jarak-core 142 tests (macros 2); jarak-gui window flows 88 (7 new: recording and playback with per-run undo and a safe stop; find and replace recorded with options; shortcut conflicts and the save dialog; Run showing the exact line and running only on the button, the browser and search entries, the elevation refusal; the real runner; the manager; the dialogs under the UI audit's rules, headless); ui-audit views run/macros added, NOT run (visible windows); e2e step102 compiled, first run pending; target/ cleaned at 16.9 GB after Step 101.

- 2026-11-22: Step 101 (#120) — jarak-core 138 tests (navigate 4); jarak-gui window flows 81 (5 new, one of them the UI audit's rules on the map, quick open and the Summary, headless; two older tests now count only find marks and styles, not the new change-history markers); 10 MB Document Map: first frame 0.8 ms, keystroke p95 0.85 ms, scroll p95 0.40 ms; ui-audit views docmap/quick-open/summary added, NOT run (visible windows); e2e step101 compiled, first run pending.

- 2026-11-22: Step 100 (#119) — jarak-core 134 tests (assist 7); jarak-gui window flows 76 (8 new: popup never taking Tab/Enter unasked, snippets, closers/tags/indent, hints/highlight, user snippets and API files, paths/tags/overloads, headless UI audit of the popup and the hint, one structure worker at a time); 10 MB measurements under limit.py: assisted typing p95 1.30 ms, Step 98 highlighted keystroke p95 1.10 ms, Step 99 keystroke p95 0.59 ms / typing with all folded p95 10.22 ms; the first assisted measurement showed p95 526 ms — the Step 99 worker storm, fixed; ui-audit views completion/hint added, NOT run (visible windows); e2e step100 compiled, first run pending.

- 2026-11-22: Step 99 (#118) — jarak-core 129 tests (structure 5: folds per language family with levels, symbols of every family, brackets, folded markers and moves); jarak-gui window flows 68 (5 new, one of them the UI audit's rules on the Function List, the fold margin and the Go to dialog, headless); 10 MB Rust measurement: 249 663 folds, first structure 452 ms on the worker, keystroke p95 0.35 ms, fold all 74 ms, folded moves p95 1.41 ms, typing with all folded p95 9.23 ms; ui-audit views functions/goto added but NOT run (visible windows); e2e step99 compiled, first run pending; real desktop pending (the maintainer is at the PC).

- 2026-11-22: Step 98 (#117) — jarak-core 124 tests (syntax 11 golden: 23 languages, detection, JSON UDL, Notepad++ UDL import, folding, state cache; comments; edit log) + 10 MB measurement (164 ms to lex to the end, a screen 0.02 ms); jarak-gui window flows 63 (3 new) + highlighted keystroke frame p95 0.29 ms (opening 10 MB at its end 514 ms); ui-audit 165 runs 0 failures (syntax, palette-language; run before the maintainer came back); e2e step98 compiled, first run pending; real desktop pending (the maintainer is at the PC).

- 2026-11-22: Step 97 (#116) — jarak-core 110 tests (lineops 14, every operation with Unicode; MD5/SHA-1 test vectors); jarak-gui unit 30 + window flows 60 (3 new); ui-audit 155 runs 0 failures (hash, palette-lines); e2e step97 focused PASS (the release window ran registry commands on a CRLF/Hangul file and saved the expected bytes); real desktop r97 checked on disk (Ctrl+D, Ctrl+Shift+L, Ctrl+Shift+Down, palette dedupe and sort, Ctrl+Shift+U, Alt+U, Base64 encode/decode, Ctrl+J, Ctrl+Alt+Enter with Korean typing, custom date; CRLF kept). Registry ids may hold digits; registry commands without default keys are palette-only by design; read-only now refuses every non-movement command.

- 2026-11-22: Step 96 (#115) — jarak-core 96 tests (multi 5: carets, occurrences, Hangul/emoji block alignment, virtual space, Column Editor, numbers); jarak-gui window flows 57 (8 new, real pointer events for Ctrl+click and Alt+drag); perf: keystroke frame 10 MB p95 0.23 ms (wide characters laid out two cells wide); ui-audit 145 runs 0 failures (multi, column, column-editor; status items now recorded); e2e step96 focused PASS; real desktop r96 checked on disk (real click + two Ctrl+clicks, real Alt+drag, Alt+Shift+arrows over Hangul, undo, Column Editor via real clicks inserting 08..12). Found by the real runs: Enter swallowed by a focused dialog button, unlabelled number fields (fixed); a first-run Ctrl+click miss that did not recur in three runs and a probe. Decision: Ctrl+D stays Duplicate line (Notepad++, Step 97); next occurrence is Ctrl+Shift+D.

- 2026-11-22: Step 95 (#114) — jarak-core infiles 6 (fixture tree, byte-level Replace in Files across UTF-8 LF/CRLF/mixed, UTF-16 LE BOM, CP949; changed-after-preview and unencodable skipped; undo; 20 batches); jarak-gui unit 30 (palette ranking) + window flows 50 (7 new); jarak CLI 2 (capability invariants: destructive needs --yes); CLI smoke in a sandbox; perf (release): `crates/` 483 files 13 MB 11 049 matches 85 ms, `docs/` 830 files 118 ms; ui-audit 130 runs 0 failures (files, results, replace-preview; rows 24 pt `tree-row`, the panel hidden under the preview modal); schema-check (6 new golden examples); e2e step95 focused PASS; real desktop r95 (Ctrl+Shift+F, Alt+W, Enter, a real double-click on a result row, palette next/previous result, Replace in Files via palette + preview + Enter with CP949/CRLF kept and the open b.md reloaded, undo restoring every byte — all checked by hex on disk). Found by the real runs: window shortcuts did not reach the find panel's boxes; the palette ranked a mid-label match first (both fixed with tests).

- 2026-11-22: Step 94 (#113) — jarak-core 84 tests + 1 ignored (search 9 + the 50 MB measurement); jarak-gui window flows 44 (9 new); perf (release): 50 MB, 832 204 matches — Count 125 ms, Replace All 347 ms (one undo step, undone in 87 ms), regex Replace All 609 ms; ui-audit 115 runs 0 failures (find, replace, mark, incremental); schema-check (jarak-find golden, session history fields); e2e step94 focused PASS; real desktop r94 byte-checked on disk (Ctrl+F/Enter/Shift+Enter/Esc, Tab to the replace box, Alt+W/Alt+A whole-word Replace All, Ctrl+Z/Ctrl+Y, Korean search, Mark page + Alt+M, Ctrl+Alt+I, select-and-find-next; r94b: regex, extended, match case, backward, wrap off, in selection, `.` matches newline via the Alt mnemonics, each checked on disk). Found by the real run: Alt+letter also typed its letter into the boxes (fixed for the whole window, AltGr still types). Incident: one F3 sent by the real test triggered the maintainer's Noeul (`paste.float`) and made pin-24; closed by its ID (`noeul pin close pin-24 --yes`, created_at matched the test); F1/F3/F8 are never sent again.

- 2026-11-22: Step 93 (#112) — jarak-core 75 tests (4 session); jarak-gui unit 29 (tabs 2, docops 1, workspace 1) + window flows 35 (8 new); schema-check with jarak-session-v1 (2 valid, 3 invalid examples); ui-audit 95 runs 0 failures (tabs, views, side, documents; fixed tab overlap at fractional widths and the invisible focus dimming → accent on the focused view's tab); e2e step93 focused PASS (session across a restart). Found by tests: stale egui focus switched the views back; a clone's other caret drifted (now line/column).

- 2026-11-22: Step 92 (#111) — jarak-core 71 tests (18 vim); jarak-gui unit 27 (keys label check, palette 2) + window flows 27 (7 new: Vim session, search/noh, read-only, settings, palette, Alt menu bar, assets dialog); perf: Vim keys 10 MB p95 4.82 ms; ui-audit 75 runs 0 failures (6 new views; fixed the palette selection over its text, the assets dialog's orphans); e2e step92 focused PASS; real desktop: palette → Vim, real keys w dw, Korean IME 한글 in Insert, j dd with Hangul on, V-LINE, :wq (file `one 한글 three⏎six⏎`, exit 0); Alt alone menu bar (found: egui reports the Alt key itself; fixed). Found by tests: Esc dropped the editor's focus before Vim saw it.

- 2026-11-22: Step 91 (#110) — jarak-core 53 tests (6 assets); jarak unit 2 + CLI smoke; jarak-gui unit 22 (9 images, 3 media) + window flows 20 (reveal through the pointer, Save As copy, end of document); perf (release): 50 × 1080p scroll p95 3.12 ms, textures peak 126.6 MB of 128, split keystroke p95 0.41 ms, 1 MB preview switch 46.7 ms; ui-audit 45 runs 0 failures (images view); schema-check PASS (3 new golden examples); e2e step91 + step90 focused PASS; real desktop: 50 pictures to the end and back, WS 121 MB (peak 143), idle CPU 0, Ctrl+click → one Explorer window (closed by the test), 100 %/150 % UI scale screenshots. Found and fixed: the preview could scroll past the end (empty page), hover text at the bottom of the preview, the audit measured before pictures decoded.

- 2026-11-22: Step 90 (#109) — jarak-core 50 tests (3 assets); jarak-gui unit 17 (7 images) + window flows 16 (MVP journey, untitled, Markdown-only/text wins); ui-audit 40 runs 0 failures (paste-image added; found a reference glued to a paragraph drawn as a placeholder → images inside paragraphs drawn); e2e step90 focused PASS; real desktop MVP journey with the real clipboard (asset 360×180, reopen shows it; found and fixed Windows' synthesized CF_DIBV5 mask offset, 3-pixel strip).

- 2026-11-22: Step 89 (#108) — jarak-core 47 tests (7 markdown); jarak-gui unit 12 + window flows 13; perf: first preview switch of 1 MB Markdown 38.4 ms (budget 100), cached 0.11 ms; ui-audit --only jarak 35 runs 0 failures (preview, split added); e2e step89 + step88 focused PASS; real desktop: Ctrl+E/typing/Down/PageDown/Esc/Ctrl+Shift+E on the fixture, file byte-identical, exit 0. Added dependency pulldown-cmark 0.13 (cargo deny ok).

- 2026-11-22: Step 88 (#107) — jarak-gui unit 8 + window flows 10 + perf (ignored, release: keystroke frame p95 0.13 ms on 10 MB after caching the EOL count, was 12.1 ms); ui-audit --only jarak 25 runs 0 failures (found: gear 24→28, audit close blocked by the question); e2e step88 focused PASS; real desktop: typing, Korean IME composition, save CRLF, undo to clean, copy, paste LF→CRLF, delete+undo, close question (Esc/Enter), click/double click/drag/wheel; release: window 247/224/307/577 ms for empty/1/10/100 MB, idle CPU 0, WS 91/107/212 MB. Found and fixed: eframe without its clipboard feature made Copy a no-op and swallowed Ctrl+V (Jarak now owns the Windows clipboard).

- 2026-11-22: Step 87 (#106) — jarak-core 38 tests (13 file: corpus round trip, detection, lossy/binary read-only, unmappable refused, EOL counts, fault injection at 4 stages, read-only refused, hidden kept, FileDocument) + the rope-vs-String model test for #105; `jarak info` on this PC (ACP 65001 → the locale's 949: a CP949 file reads as EUC-KR, lossless); e2e step87 focused run PASS.

- 2026-11-22: Step 86 (#105) — jarak-core 26 tests (graphemes incl. across chunks, words with Hangul, columns, selections, grouping, save point, three carets, moves, registry, 40-seed random consistency); 100 MB benchmark (release): edit p50 2.7 µs / max 79 µs, undo 0.7 µs/step, record docs/perf/jarak-text-model.json; `cargo deny check bans licenses sources` ok (ropey 1.6.1 and unicode-width added).

- 2026-11-22: Step 85 (#104) — jarak-core 4 tests, jarak 2, jarak-gui 2 + window flows 4; kim 74 + cli_contract (catalog-list re-blessed: Jarak row, catalog v11) + contract_examples; kim-manager ui 20 (four tiles, launcher 464 × 224); e2e step85 focused run PASS 1/1; real desktop (sandbox, `--instance`): screenshot + geometry of the shell, hand-off of `todo.txt:4` to the running window, `--new-window`; release: jarak.exe 0.5 MB, jarak-gui.exe 6.0 MB, window handle 61–103 ms, idle CPU 0 ms/10 s, memory = the shared egui+glow baseline (114.5 MB private / 87.9 MB WS vs Kim Manager 110.1 / 89.6 at zoom 1; JARAK-NFR-003 corrected with this evidence); `cargo xtask package jarak` 3.3 MB zip.

- 2026-11-22: Step 84 (#103) — 28 Jarak issues created (#103–#130) and read back; docs JARAK_PHILOSOPHY / JARAK_REQUIREMENTS (J1–J5) / JARAK_NOTEPADPP_MATRIX (285 Notepad++ 8.9.x rows, 8 not intended, 16 additions), ADR 0102; `xtask` docs-check validates `JARAK-*` IDs and every matrix row (owner Step 84–111, defined requirement, evidence for partial/supported; unit test). `cargo xtask docs-check` PASS.

- 2026-11-17: ADR 0101 follow-up — validation warns `trigger_shadowed` when a rule on CapsLock itself (`CapsLock`, `Shift+CapsLock`) sits next to a CapsLock chord rule (the engine takes CapsLock first in every mode); unit test; ddalgak/ddalgak-editor 0.0.6. The maintainer asked to keep the 18 mappings set up for daily use on their own Ddalgak: `kim update ddalgak` 0.0.4 → 0.0.6 (local mirror, catalog cache v11), the old 0.0.3 tray app ended with `ddalgak quit`, the 18 rules brought in with `ddalgak profile import` (profile *키보드 단축키*, active) and turned on with `rule enable --allow input-injection` (the screen was locked, so not in Settings), `settings.capslock: always_off` and `engine_on_start: on`; the 0.0.6 tray app runs: engine running, 18/18 active, CapsLock off, `rule check` 0/0/0, `ddalgak test` fires each of the 18 (simulation). Noeul and Subak untouched.

- 2026-11-16: ADR 0101 — real desktop (sandbox, debug tray app with `--test-physical-input`, real editor clicks): all 18 mappings created from an empty profile in the GUI (first save each, 0 rule-check errors/warnings); real keys verified in Notepad/a test list box (arrows, Home/End, Page Down/Up, ←→↔ incl. Korean IME, media keys dispatched, volume ±1 step and restored, pause/resume by CapsLock+Backspace, CapsLock+/ opens Settings, restart keeps everything, emergency off not bypassed); 3 GUI problems fixed (quick target keys 12–13→10 clicks, auto-scroll to the action's settings, distinct typed-text names). Tests: ddalgak lib 380, kim-ui 12, editor 44 + flows 35 + playground 3. CapsLock/volume/mute restored after every run.

- 2026-11-16: ADR 0100 — `cargo xtask ui-audit`: Ddalgak 70 runs 0 failures 0 warnings (14 screens incl. six Settings sections measured in the Settings window), full matrix 133 runs (Kim, Noeul, Subak 0 failures); new rules toolbar-height/align/gap, gear-corner, form-width with unit tests; Ddalgak editor unit 43, flows 34 (+9 new), playground 3; kim-manager ui 20; kim-design 18; kim-ui 11; real desktop `ddux.ps1` r7 (sandbox, test host): every journey of DDALGAK_UI_AUDIT §4 passed; 8 defects found on the desktop and fixed (mode added to the wrong profile, theme switching the language, Esc in search, Filters under the search, keycaps under the switch, a new rule opening scrolled, the empty-key message, a worker race). Versions: kim 0.0.4, subak 0.0.5, noeul 0.0.4, ddalgak 0.0.4, kim-ui 0.0.3, kim-design 0.0.3.

- 2026-11-15: ADR 0099 — `cargo xtask ui-audit` full matrix 88 runs: 0 layout failures, 0 warnings (found and fixed: Kim settings row overlap, Done over a section, 26 px text fields, compact-button rule, Ddalgak lone dash / premature errors / empty state, the overflow gap); 9 curated baselines match at 0.000 %; Subak lib 289, gui_window 47, gui_settings 12, all other Subak suites pass; kim-manager ui 20, Ddalgak editor 43 + flows 25 + playground 3, kim-design 17, kim-ui 9, noeul-settings 4; e2e adr0099 focused run PASS; real desktop: before/after screenshots of Subak 0.0.1, 0.0.3 and the new build (sandboxed). Versions: kim 0.0.3, subak 0.0.4, noeul 0.0.3, ddalgak 0.0.3.

- 2026-11-15: ADR 0098 — Subak's tab bar keeps the design-system size on a large screen (was ×1.75 on 5120 × 1410); off-screen test with `SUBAK_TEST_SCREEN_ZOOM` (fails with the old code); subak 0.0.3; real desktop not tested (the maintainer was using it). `cargo xtask ci` EXIT 0.

- 2026-11-15: ADR 0097 — `cargo xtask bump --changed` / `version-check` (in ci); kim, subak, noeul, ddalgak raised to 0.0.2 (Cargo.toml, path requirements, capabilities/hotkey manifests, contract examples, Kim golden files re-blessed: version lines only); dev profile without incremental cache and dependency debug info: `target/` 111 GB → 5.9 GB after a full `cargo xtask ci` (EXIT 0).

- 2026-11-15: ADR 0095 (Subak: uniform tabs from tokens, gear, Default shell; real Git Bash Linux commands) and ADR 0096 (Ddalgak editor: rail with the real engine switch, Settings behind the gear, one-line rule list, rule page Trigger → Action → Details → Test, Together / One after another / Hold + press saved as a hold layer, key picker). Editor unit 43, flows 25, playground 3; e2e scenario adr0096 compiled and its commands checked by hand; real desktop (sandbox ddalgak-ds): Hold CapsLock + J → Down built with the picker and real keys, consent dialog, saved; Settings, light theme saved in editor.json.

- 2026-11-15: user request "Unified Design System and Complete UX Refinement" (ADR 0092–0094, in progress): `kim-design` + `kim-ui` crates; Kim Manager on them (settings rail, switch rows, language en/ko, dialogs; real desktop dark/light/Korean, menu fits); `noeul-settings.exe` (sections, IPC `hotkeys.pause`, real clicks wrote config.toml); new pins drag at once (real: first move 62–76 ms after a press 0–200 ms after the pin appeared), double-click closes only that pin, zoom read-out (real: 125 % shown, gone after 1.6 s), triple/quadruple click makes one pin. Next: Subak tabs/shells/Linux commands, Ddalgak redesign, review. `cargo xtask ci` per increment.

- 2026-11-14: ADR 0091 follow-ups — Subak tab bar grows with the screen factor (real clicks switch tabs); Kim right-click *Reinstall…* (same version again, verified, data kept; ui test); e2e leftovers from release40 stopped and removed; a real display-scaling change to 150 % (saved, restored to 100 %, registry values checked): Kim Auto 175 %, Settings on screen; Subak 1.0 by design. kim-manager ui 19/19; `cargo xtask ci` EXIT 0.

- 2026-11-14: user request (ADR 0091) — windows open larger on large screens: Kim zoom (Auto 275 % on 5120 × 1410; Settings → General → Size, saved in manager.json; a grown window stays on screen and moves back), Subak first window 1400 × 1050 and default text 23 px (config value wins; Ctrl+0 returns to it), Ddalgak editor 175 %, Noeul settings 150 % (+ its dark background, a pre-existing gap). Unit tests per rule, kim-manager ui 18/18, subak all suites; real desktop (sandbox `kim-big`): Kim install of all three via clicks at 275 %, Settings Size 150 % ↔ Auto (resize, on-screen, restore), Subak size/text/zoom keys, Ddalgak editor 1855 × 1190, Noeul settings 900 × 1110 through an isolated `--endpoint` instance (the maintainer's own Noeul and Ddalgak in the real profile were running and were not touched). Found by real runs: Subak's grid sizing overrode the first size, Kim's resize sent before the zoom applied, egui's stale window position. `cargo xtask ci` EXIT 0; `cargo deny` bans/licenses/sources ok.

- 2026-11-14: user request (ADR 0090) — Kim Manager redesigned as a three-icon launcher (Subak, Noeul, Ddalgak; gear → Settings; right-click menus; state from the inventory with the program on disk); Noeul/Ddalgak icons (assets/noeul, assets/ddalgak, linked into their exes). kim-manager ui 17/17 (incl. 150/200 % scale); real desktop with the release build in a sandboxed profile: launcher, menus, install of all three from kim-local, launch (Subak shell, Noeul F1 overlay + settings window, Ddalgak tray + editor), close/reopen, Settings (theme, shortcut Change/Reset via kim hotkeys, catalog refresh), uninstall refused while Subak ran, uninstall/reinstall with data kept, repair, resize; root kim.exe double-click (ShellExecute) in the real profile opens the launcher (nothing installed there). Fixed from real runs: dialog overflow in the compact window, repair refused (now reinstall), uninstall dialog width/name, info dialog width, status line clipping, Ddalgak click opening nothing visible. Real display-scaling change not tested. `cargo xtask ci` EXIT 0.

- 2026-11-14: user request (ADR 0089) — root `kim.exe` usable by double-click: hand-over to Kim Manager (only no-arg, lone console with a window, console stdio, no KIM_NO_INPUT), `cargo xtask local-mirror` (signed `file://` catalog in `kim-local/`, session-only, never cached), Open buttons, icons + version info via `kim-winres` (kim, kim-manager, kim-setup: Kim; subak, subak-gui: watermelon). Tests: kim-winres 4/4, start 3/3, store mirror, double_click (hidden console + job: baseline 2, double-click 3), kim-manager ui mirror/open/icon, program_icon, subak icon_assets; sandboxed real install of subak/noeul/ddalgak from the generated mirror (release kim); `cargo deny` bans/licenses/sources ok; real double-click and on-screen icons **not tested** (no real input while the user is present).

- 2026-11-13: Noeul #99 SNIP-12 (typed size / ratio, T) and DET-03 (wheel to parent element) verified on the real desktop; `cargo xtask ci` EXIT 0. Driver note: the key name is `esc` (not `escape`); F1 needs `RegisteredKey(1)` because `Hotkey` only sends F13-F24.

- 2026-11-13: Ddalgak real-desktop pass 2 (modes, contexts, macros, window/clipboard/timers, text, launches); fixes 82ae844 (live header, mode row), 77f0271 (timer target), 91a13d8 (screen map ratio), 9aaab54 (clipboard restore after paste); `cargo xtask ci` EXIT 0 each.

- 2026-11-13: Ddalgak editor on the real desktop (CRUD, profiles, playground, own shortcuts); fixes 2b8804d (Korean UI strings, stale Undo toast, profile-per-program rows); results in DDALGAK_ACCEPTANCE; `cargo xtask ci` EXIT 0.

- 2026-11-13: Noeul #66/#68 and Ddalgak checks on the real desktop; fixes 3df6de9 (settings theme), 1781540 (Ddalgak password field, schema memory 26.9 -> 3.4 MB); `cargo xtask ci` EXIT 0.

- 2026-11-13: real-desktop runs of SUBAK_GUI 9/11/12, TERMINAL_EMULATOR 9, MILESTONE_A 6, CONPTY_PROOF 7.1 and NOEUL_UX_SPEC 4.5-4.7 (F1, clipboard, 150 %, double-click speed); `cargo xtask ci` EXIT 0 at ae349d3.

- 2026-11-13: real-UI §4.5-4.7 on the visible desktop (release, 5120×1440 100 %): first frame p50 111 ms / p95 115 ms; found and fixed Ctrl+click selection, snapping edges, wheel zoom drift, pin quick-save format; `cargo xtask ci` EXIT 0 at fce5582.

- 2026-11-13: #102, #101, #100 closed; #99 batches (output, pins, tray, Space drag) with e2e issue99/100/101/102 passing; `cargo xtask ci` EXIT 0 at a20f86d.
- 2026-11-12: Step 82 — `cargo xtask bench-ddalgak --record`: 8/8 budgets (0 ms CPU and 0
  wakeups / 10 s at 100 and 1000 rules, hook p99 ≤ 100 µs, 1000 rules checked in 78 ms, profile
  switch 510 ms after the reload fix, CLI 4.9 MiB; debug-host memory 31 MiB informational);
  e2e step82 1/1; `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 81 — `mcp_cmd::tests` 3/3, e2e step81 1/1 (Kim installs, discovers and
  invokes Ddalgak; MCP session with grants, refusals and audit); `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 80 — `ahk::tests` 4/4 (fixtures and goldens), editor wizard flow test,
  e2e step80 1/1; `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 79 — `actions::tests` 2/2, loader, worker and Windows desktop tests, editor
  form test; e2e step79 2/2 (a 1-minute schedule fired in the running engine); `cargo xtask ci`
  EXIT 0.
- 2026-11-12: Step 78 — worker macro tests 5/5, loader tests updated (mixes are macros), editor
  recorder 2/2 and macro flow test, CLI tests 31/31; e2e step78 A 1/1, B passed its ordered-run
  part before the desktop locked (cancel part then skipped); `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 77 — `context::tests` 5/5, core condition tests 4/4 (per-app precedence,
  200 rapid focus switches, unknown fails safe, one probe per event, hotstrings), loader
  precedence test, Windows reader test (own window unknown, explorer by path), editor form
  round-trip; e2e step77 2/2; `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 76 — `simulate::tests` 4/4 (all 23 `ddalgak-simulation-v1` examples
  reproduced and schema-valid), editor `tests/playground.rs` 2/2 (FakeLauncher: nothing opened
  before *Confirm and open*), e2e step76 1/1, `cargo xtask ci` EXIT 0.
- 2026-11-12: Step 63 — `cargo xtask bench-noeul --record`: 9/9 budgets (5.0 MiB, ready 25 ms,
  0 ms idle CPU / 10 s, 1.8 MiB private, overlay p95 33.5 / 83.8 ms, pin p95 17.2 / 60.2 ms);
  fuzz tests 3/3 (images 4200 mutations, HTML 2000, DIB 3000); e2e step63 1/1; parity matrix
  recounted (164 rows: 11 implemented, 101 partial, 28 planned, 24 not intended), #99 opened.

- 2026-11-12: Step 62 — privacy_tests 3/3 (notify/allow/deny, agents off by default and never the
  clipboard, capture to a PNG file, no overwrite, no path echoed), mcp 2/2 + interop 1/1 unit,
  mcp_adapter.rs 1/1 real instance (read only, two consents, pin file, audit without paths),
  e2e step62 1/1 (kim install noeul from a test-signed catalog, tools find/invoke, uninstall),
  a real Claude Code run against `noeul mcp serve` (status ok, capture refused).

- 2026-11-12: Step 61 — prefs model 4/4, settings_tests 2/2 (opens only when asked, recording
  suspends hotkeys), settings_window.rs 2/2 on the real off-screen window (record, Esc, shadowed
  default, refused conflict + Use anyway, disable, reset, reset all in two presses, gesture and
  disk-history switches to config.toml, live re-registration of a global key and back), all noeul
  tests, e2e step61 2/2.

- 2026-11-12: Step 60 — noeul lib 334/334 (store 4, settings, selection, keeping_tests 4: what was
  output kept with redactions drawn in, private excluded, disk history bounded/restart/damage,
  backup offered/restored once/no duplicates/damaged skipped/off-screen fixed), history_backup.rs
  1/1 real instance across a restart, all noeul integration tests (memory checks now count the
  history copy explicitly), e2e step60 2/2.

- 2026-11-12: Step 44 — claude_motion unit tests 6/6, art/tabs frame tests, icon_assets 3/3
  (byte-exact assets, SVG colours, legibility 16–48 px on 4 surfaces), gui_window motion test
  (turn, blink, minimized 0 frames, static, settle, new window; 0.0 ms CPU over 10 s turning),
  gui_float badge test (no tray update per frame), e2e step44 1/1.

- 2026-11-12: Step 43 (Windows) — kim-setup unit tests 2/2, setup.rs 1/1 (install only Kim,
  refuse/repair/upgrade/downgrade, damaged installer, running refusal, uninstall keeping data;
  test registry key), e2e step43 1/1 with the release Kim programs; `xtask package kim` writes
  Kim-Setup-0.0.1-x86_64.exe (6.3 MB); the double-click window path is the maintainer's check.

- 2026-11-12: Step 40 — `cargo xtask package kim|subak` (deterministic, re-pack same SHA-256,
  notices), `kim rollback` (rollback.rs 1/1, contract goldens), `package --signed` refuses
  unsigned programs, e2e step40 1/1 (install → running → side-by-side update → rollback →
  offline → uninstall keeping data); release.yml not run (no certificate).

- 2026-11-12: Step 39 — `cargo xtask bench --record`/`--check` (22 measurements, budgets in
  docs/perf/baseline.json), found and pinned the Windows 10 inbox ConPTY handle per pane
  (existing conpty tests: Subak adds none), `subak host stats`, e2e step39 1/1; SECURITY,
  PERFORMANCE, RELEASE_READINESS docs.

- 2026-11-11: Step 38 — kim interop/redact unit tests, kim interop 1/1 (fixture: consent per
  effect, dry run, versions, missing/old/hung/garbage/failing provider, nothing installed),
  contract goldens (tools-find, tools-invoke), subak interop_provider 1/1, e2e step38 1/1
  (release kim → release subak).

- 2026-11-11: Step 37 — mcp unit tests (grants, annotations, handshake, refusals), mcp_server 1/1
  (read-only with no Host, redacted token, ungranted tools refused, cancelled wait, create/type/
  wait/split/keys, run required for a program, kill preview then confirm, audit without text),
  e2e step37 1/1, real Claude Code 2.1.322 `claude -p --mcp-config` called list_terminals.

- 2026-11-11: Step 36 — remote/ssh/config unit tests (Telnet state machine, serial newlines and
  DCB bits, ssh config hosts, profile groups), telnet_serial 1/1 (opt-in, negotiation, login,
  hang-up, missing COM port, ssh hosts), e2e step36 1/1; idle resources: Host 7.5 MiB / 0 ms CPU
  with 9 panes, Telnet bridge 5.3 MiB / 0 ms; parity report + follow-up #98; not tested: real
  serial hardware.

- 2026-11-11: Step 35 — ssh/sftp unit tests (hostile SFTP listing, per-host askpass, forwards,
  login scripts), ssh_transfers 1/1 against three real sshd servers (tree, 64 MiB, overwrite,
  cancel+resume, server killed+resume, jumps 1 and 2 hops, L/R/D, SOCKS5/HTTP proxy, port
  collision, guards), e2e step34+35 2/2 on a hidden desktop (no window); not tested: real X11,
  agent forwarding with a running agent.

- 2026-11-11: Step 34 — ssh unit tests 6/6, ssh_sessions 1/1 against a real local sshd (first
  trust, saved passphrase, wrong secret, changed key, offline, SSH profiles, leak scan), e2e
  step34 1/1 (focused local run, LEDGER); not tested: password login, 2FA, the ssh-agent.
- 2026-11-11: Step 33 — shells/integration/term unit tests, profiles_integration 2/2 (detected
  profiles, title, missing shell, PowerShell and cmd folder reports, state keeps it), e2e step33
  1/1 (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 32 — Tabby matrix audited (39 rows reclassified, checklist, ADR 0065),
  docs-check validates the matrix (209 rows, 83 evidence refs); `cargo xtask ci` before push.
- 2026-11-11: Step 31 — milestone_c 1/1 (Host crash → intact state → cold rebuild 99 ms, no
  re-run, no duplicate); recovery-matrix.ps1 smoke: check-live PASS, check-cold after a Host
  kill PASS (79 ms); power transitions NOT TESTED (maintainer).
- 2026-11-11: Step 30 — recovery 2/2 (topology, sizes, id, no rerun, confirm, duplicate, auto,
  Claude resume decisions), a real Claude 2.1.322 conversation resumed by `subak recover
  --claude-resume`, e2e step30 1/1 (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 29 — state unit tests 5/5, state_persistence 3/3 (clean stop, no relaunch,
  killed Host, torn file, newer version, rollback, secrets, private Hosts), e2e step29 1/1
  (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 28 — claude/tabs/model/float unit tests, claude_status 1/1 (all states,
  lost and stale signals, repeated states), gui_window claude tab test (off-screen), a real
  `claude -p` (2.1.322) with the 8-event hook set, e2e step27+28 2/2 (focused local run,
  LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 27 — claude unit tests 3/3, claude_hooks 2/2, a real `claude -p` (2.1.322)
  in a Subak pane reported SessionStart/UserPromptSubmit/Stop/SessionEnd, e2e step27 1/1
  (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 26 — xtask tmux matrix check (606 rows, 302 evidence refs), tmux_journeys
  1/1, e2e step26 1/1 (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 25 — layout/tmuxcmd/session/screen unit tests, tmux_advanced 2/2, full
  `cargo test -p subak -p subak-term` green, e2e step25 1/1 (focused local run, LEDGER);
  `cargo xtask ci` before push.
- 2026-11-11: Step 24 — tmuxcmd unit tests, tmux_commands 2/2, gui prompt/copy-mode test, e2e
  step24 2/2 (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 23 — keys unit tests, gui prefix test, CLI detach keys test, e2e step23
  2/2 (focused local run, LEDGER); `cargo xtask ci` before push.
- 2026-11-11: Step 22 — `cargo xtask ci` green (after fixing hotkey scenario 13); pane_layout 5/5,
  gui split test, e2e step22 2/2 (focused local run, LEDGER).
- 2026-11-11: Daily e2e had failed 7× (stale `step17`/`step18` scenarios); fixed, focused local
  run 4/4 PASS; recorded in `e2e/LEDGER.md`.

## Queue (Step number in the issue title; reconstruct with
`gh issue list --repo madebymouv/kim --state open --limit 200`)

1. Subak: 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 43
   (Windows part), 44
2. Noeul: 60, 61, 62, 63
3. Ddalgak: 76, 77, 78, 79, 80, 81, 82

## Blocked

- **Step 31 (#31)**: Milestone C needs real lock/unlock, sleep, hibernate, reboot and power-loss
  runs by the maintainer (rows 10–16 of MILESTONE_C_RECOVERY_VALIDATION); everything automatable
  passes.
- **Linux Steps 41, 42, 64, 83 and the Linux half of 43 (#44)**: AGENTS.md §3 forbids Linux work on
  this Windows PC (maintainer decision 2026-11-04); they wait for the Linux server phase.
- **Maintainer manual checks** (implemented and merged; issues stay open until the visible-window /
  real-input / real-clipboard / multi-monitor checks named in each issue's report are done):
  #8, #14, #15, #16, #18, #19, #20, #55–#58, #61–#64, #66, #68, #71, #73–#84, #87.
