# Ddalgak keyboard mappings: GUI-only setup and real Windows verification (ADR 0101)

Status: **current** (2026-11-16). Decision: [ADR 0101](adr/0101-ddalgak-capslock-modifier-and-keyboard-actions.md).
Scripts and raw results live outside the repository (the session's scratchpad `d101/`:
`phaseAB.ps1`, `phaseCDE.ps1`, `phaseZ.ps1`, `pgdn2.ps1`, `D101.cs`); this page records what
they measured.

## 1. How it was tested

- **Programs:** this build's debug `ddalgak-gui`, `ddalgak-editor` and `ddalgak` copied into a
  sandbox (`%APPDATA%`/`%LOCALAPPDATA%` in the scratchpad, a private `--endpoint`), the tray app
  started with `--test-physical-input` (§6 of the ADR). The maintainer's own Ddalgak, Noeul and
  Subak kept running and were never touched (checked by PID before and after every phase).
- **GUI:** real mouse clicks at the positions the editor's own geometry report gives
  (`KIM_UI_INSPECT`); text typed with real key events. No rule was written as JSON: the test read
  `rules.json` only as evidence.
- **Keys:** `SendInput` events tagged "DDTS" (counted as physical by the test switch), sent only
  while a test window had the foreground. Ddalgak's output went through its real `SendInput`.
- **Results read back from Windows:** Notepad's edit control (`WM_GETTEXT`, `EM_GETSEL`), a test
  list box (`LB_GETCURSEL`), CapsLock's toggle (`GetKeyState`), the master volume (Core Audio),
  and an observer low-level hook in the test process that records Ddalgak's injected keys
  (virtual key, scan code, extended flag) and swallows its media keys so the maintainer's player
  was never touched.
- **Restored:** CapsLock (off before and after), volume (34 %) and mute (off), checked after
  every run.

## 2. GUI-only setup from an empty profile (Phase A, B)

Phase A: Settings opened as its own window, a new profile *Keyboard test* made and switched to
(0 rules), the engine shown as off. Phase B: the 18 mappings one by one: New Rule → keys by
clicking in the key picker → action card → its settings → Save and enable (→ Allow for the
input-injection consent where needed). All 18 saved enabled on the first save, no validation
error, one mode (no hidden layer), `ddalgak rule check`: 0 errors, 0 warnings.

| # | Keys (stored) | Action (stored step) | Clicks before → after | Screens before → after | Text fields |
| --- | --- | --- | --- | --- | --- |
| 1–4 | `CapsLock+W/A/S/D` | `remap` to Up/Left/Down/Right | 12 → 10 | 4 → 3 | 0 |
| 5 | `RShift+Enter` | `send_keys` `MediaPlayPause` | 8 | 3 | 0 |
| 6–7 | `RShift+Left/Right` | `send_keys` `MediaPrevious`/`MediaNext` | 9 | 3 | 0 |
| 8–9 | `RShift+Up/Down` | `send_keys` `VolumeUp`/`VolumeDown`, `repeat: true` | 8 / 9 | 3 | 0 |
| 10–13 | `CapsLock+Q/E/Z/X` | `remap` to Home/End/PageDown/PageUp | 13 → 10 | 4 → 3 | 0 |
| 14 | `CapsLock+Backspace` | `engine` `toggle_pause` (no consent needed) | 7 | 2 | 0 |
| 15 | `CapsLock+code:Slash` | `engine` `open_settings` | 8 | 2 | 0 |
| 16–18 | `CapsLock+Shift+Comma` / `Shift+Period` / `M` | `type_text` `←` / `→` / `↔` | 10 | 3 | 1 |

The final run on the committed build needed 5–9 mouse-wheel notches per mapping (to reach the
action cards below the trigger in a 900 px tall window; the action's own settings now scroll into
view by themselves).

Screens: the editor, the key picker, the consent dialog (and before the fix a second picker for
the target key). Every mapping was found from the card titles without documentation; no
technical concept (modes, scan codes, virtual keys, JSON) appeared. The sentence above the form
said each rule in words, e.g. "When CapsLock is held and < is pressed, type “←”.".

**Problems the real run found and what changed** (before/after measured with the same script):
1. The target key needed a second picker (3 clicks, one more screen) → quick keys under
   *Acts as*: 12–13 → 10 clicks, 4 → 3 screens.
2. The chosen action's settings sat ~700 px below a 900 px window (the script needed up to 15
   wheel notches) → the editor scrolls them into view.
3. Three typed-text rules were all named "Type a phrase" → "Type a phrase: CapsLock + <".
4. The test script's own defects were fixed in the script, not the product: a shell toast holding
   the foreground, PowerShell's case-insensitive `$R`/`$r`, a focus helper that tapped Alt (Alt
   alone puts a window into menu mode, which swallowed the next click) and clicks sent without
   moving the pointer first right after a window switch (the first click was lost).

## 3. Real keys (Phase C) and negative cases (Phase E)

| Check | Result |
| --- | --- |
| CapsLock + W/A/S/D/Q/E in Notepad (caret at 9 in `abcde⏎fghij⏎klmno`) | 9→2 (Up), 9→8 (Left), 9→16 (Down), 9→10 (Right), 9→7 (Home), 9→12 (End); text unchanged (no `w`/`a`… leaked); CapsLock off after each |
| CapsLock + Z / X | Ddalgak sent VK 0x22 / 0x21 with scan 0x51 / 0x49 and the extended flag — byte for byte the key the test sent itself. Notepad moved for neither (Ddalgak's nor the test's own Page keys), so a test list box was used: 150→178 (PageDown) and 150→122 (PageUp), the same as the direct keys |
| CapsLock + Shift+Comma / Shift+Period / M | Notepad text `U+2190`, `U+2192`, `U+2194` exactly |
| Korean IME in Hangul mode | CapsLock + M typed `U+2194`; the next `r` composed `ㄱ` (`U+3131`) as usual |
| RShift + Enter / Left / Right | Ddalgak sent `MediaPlayPause` (B3), `MediaPrevious` (B1), `MediaNext` (B0), each one down + one up, with Right Shift lifted and restored around it; Notepad saw no Enter and the caret did not move. The observer swallowed the 6 media events (no media session was touched; whether a player reacts was not tested) |
| RShift + Up / Down | Volume 0.34 → 0.36 → 0.34 (one step per press); Up with two auto-repeats 0.34 → 0.40 (three steps); restored to 0.34 |
| LShift + Enter | A newline in Notepad; no media key sent |
| W A S D without CapsLock | `wasd` typed |
| CapsLock alone, three taps | CapsLock stayed off (`always_off`) |
| Four rapid CapsLock + W | caret up four lines, text unchanged |
| CapsLock + D held with three repeats | caret right four characters |
| CapsLock released before W | one Up, nothing leaked |
| Shift + CapsLock + A | selection 8..9 (Shift + Left) |
| After everything | no key logically held (Shift, Ctrl, Alt, CapsLock, Up, W) |
| CapsLock + Backspace | `running` → `paused`: Backspace not typed; CapsLock + W while paused typed `w` (no rule runs) and moved nothing; RShift + Enter sent no media key; CapsLock stayed off; again → `running`, CapsLock + W moves again |
| CapsLock + / | the Settings window opened (the running editor was asked); no `/` typed |
| Key picker: CapsLock + LWin | "the chord is reserved (Windows key shortcut)", Confirm disabled |
| A second rule on CapsLock + Backspace | Save disabled; the trigger says "these keys turn automation off and on (rule “…”)" |

## 4. Restart, persistence, emergency (Phase D)

- Editor and tray app closed normally and started again (the editor in Korean): profile
  *Keyboard test*, 18 rules, `settings.capslock: always_off` kept; the engine started **off**
  (`off_reason: default`, `engine_on_start` is off by default) and was turned on in Settings.
- CapsLock + W, CapsLock + M and RShift + Enter worked again; CapsLock + Backspace paused (CapsLock
  + W did not move the caret) and resumed (it did).
- Ctrl+Alt+Shift+Esc (test-physical) → `off`, `off_reason: emergency`. CapsLock + Backspace then
  did nothing to the engine (still off; with no hook, Windows toggled CapsLock and Notepad got the
  Backspace — the stated scope; CapsLock was put back). The next start stayed off with
  `emergency`; Settings (in Korean) said "긴급 끄기로 꺼졌습니다. 어떤 단축키도 다시 켜지 못하며…";
  only turning it on in Settings started it.

**Seen on this PC, outside Ddalgak:** after the test sent Ctrl+Alt+Shift+Esc with `SendInput`,
Windows kept Left Ctrl, Left Alt and Left Shift logically down — also with no Ddalgak test
instance running (reproduced). The test releases them afterwards; once it did not, and Noeul's
and Subak's window tests then failed until the keys were released (they pass since). The cause
was not investigated further, so as not to touch the maintainer's running programs.

## 5. Automated tests

- `cargo xtask ci`: EXIT 0 (fmt, clippy `-D warnings`, every workspace test, docs, schema,
  design, version and dependency checks).
- `ddalgak` lib: 380 (new: CapsLock as a held modifier that never toggles, always-off without a
  layer, CapsLock + Backspace while paused, Shift widening and the emergency chord with CapsLock
  held, Right Shift versus Left Shift, key reset; the 18 mappings load as 18 active rules;
  canonical CapsLock triggers; the trigger syntax and its reserved chords).
- `kim-ui` 12 (CapsLock and symbols in the picker), `ddalgak-editor` unit 44 (capture sides and
  CapsLock), flows 35 (new: the five kinds of mappings from clicks, Settings keeps CapsLock off),
  playground 3.
- e2e `adr0101_ddalgak_capslock_media.rs` (Daily workflow): the 18-mapping example through the
  real CLI (`rule check`, `ddalgak test`, paused, refused chords).

## 6. Not tested

- Physical keyboard presses by a person (the tests used tagged `SendInput`; the hook, core,
  output and applications were the real ones).
- A media player reacting to Play/Pause/Next/Previous (the keys were verified at dispatch and
  kept away from the maintainer's player on purpose).
- Lock and unlock (the lock screen is never automated) and the secure desktop; covered by unit
  tests of the key reset and by the unlock hook in code.
- An elevated foreground window, a second monitor, other keyboard layouts than Korean/US.
