# Development checkpoint

Sequential workflow ([ADR 0055](docs/adr/0055-single-branch-sequential-development.md)): one
session, one folder, one branch (`main`), one Step at a time. Update this file at every Step
transition. Resume prompt: *"Continue the Kim sequential development mission from the latest
checkpoint."*

## Now

- Branch: `main` · last pushed commit: see `git log -1 origin/main`
- Current work (maintainer assignment 2026-11-22, ADR 0102): **the Jarak roadmap, Steps 84–111 = issues #103–#130** (registered 2026-11-22 from the roadmap; the prepared zip package was not on this PC, bodies rebuilt in the session scratchpad `jarak-issues/`). Jarak Steps come first; older issues stay open unchanged. Done: Step 84 (#103, closed, b5ec2f0: docs JARAK_PHILOSOPHY, JARAK_REQUIREMENTS, JARAK_NOTEPADPP_MATRIX checked by docs-check); Step 85 (#104: crates `jarak-core`/`jarak`/`jarak-gui`, CLI `version`/`capabilities`/`paths`/`open`, window shell, single-instance hand-off, catalog v11 + package + fourth Kim Manager tile, JARAK_ARCHITECTURE). Next: Step 86 (#105) rope text model, graphemes, selections, undo/redo in `jarak-core`.
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
