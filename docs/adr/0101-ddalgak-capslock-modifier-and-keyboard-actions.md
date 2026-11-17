# ADR 0101: CapsLock as a modifier, the always-off policy, and keyboard actions in the editor

- Status: accepted
- Date: 2026-11-16
- Issue: maintainer task "Ddalgak — Implement and Verify 12 Real-World Keyboard Automation
  Requirements" (2026-11-16). Extends ADR 0022 (rule model and safety), ADR 0031 (remap
  execution), ADR 0043 (modes and layers), ADR 0096 ("Hold + press" layers in the editor) and
  ADR 0100 (the rule editor and the Settings window). Requirements: `DDALGAK-FR-077`,
  `DDALGAK-FR-078` (new), `DDALGAK-FR-004`, `DDALGAK-SEC-003`, `DDALGAK-SEC-005`,
  `DDALGAK-SEC-009`, `DDALGAK-SEC-011`. Evidence: [DDALGAK_KEYBOARD_TASK](../DDALGAK_KEYBOARD_TASK.md).

## Context

The maintainer wants 18 concrete mappings, made only in the editor: CapsLock + W/A/S/D/Q/E/Z/X
as arrows, Home/End and Page Down/Up, Right Shift + Enter/Left/Right/Up/Down as media and
volume keys, CapsLock + Backspace to turn the automation off and on, CapsLock + / for Settings,
and CapsLock + `<` `>` `M` typing `←` `→` `↔`; CapsLock itself must stay off.

What Ddalgak had before this change (read from the code, then confirmed on the real desktop):

1. **CapsLock was only a key.** `CapsLock` could be a trigger or have modifiers in front
   (`Shift+CapsLock`), but nothing could be *held with* CapsLock. The editor's "Hold + press"
   (ADR 0096) made a hidden mode layer: a `mode hold` rule on CapsLock plus rules limited to
   the layer.
2. **That layer broke the off/on shortcut.** While paused only engine-control rules run, so the
   `mode hold` rule on CapsLock stopped: CapsLock went back to Windows (it toggled capital
   letters) and a CapsLock + Backspace rule inside the layer could never resume the engine.
3. **Nothing kept CapsLock off.** A tap of a layer key did nothing, but CapsLock already on
   (or toggled on the lock screen, or by any key Ddalgak did not own) stayed on.
4. **No "open Settings" action** and **no media or volume action** in the editor: media keys
   were reachable only as a raw `send_keys` text (`MediaPlayPause`), volume keys as remaps that
   repeat with the key; `/`, `<` and `>` were not in the key picker; the picker could not hold
   CapsLock with another key; the capture box could not tell Right Shift from Left Shift.
5. **Real-desktop input tests could not reach the rules.** The engine ignores input other
   programs inject (`injected: ignore`), so a test driver's `SendInput` keys never triggered a
   rule, and the debug test host never injects its output.

## Decision

### 1. `CapsLock+<chord>` is a chord with CapsLock as its modifier

Trigger syntax (DDALGAK_RULE_MODEL §3.1): `CapsLock+` before one contract chord —
`CapsLock+W`, `CapsLock+Shift+Comma` (`<` on US-style and Korean layouts), `CapsLock+code:Slash`
(`/`), `CapsLock+RShift+Up`. It is a whole trigger (never a sequence step); its key is a named
or `code:` key (not a lock key, a bare modifier or a mouse button). The chord after CapsLock is
checked like any chord: `CapsLock+Alt+F4`, `CapsLock+Win+…` and the emergency chord stay
reserved. The JSON schema's `trigger_syntax` and `trigger_keys` say the same.

**Matching** (the core, `crates/ddalgak/src/core.rs`): while an active rule has a CapsLock chord,
the engine owns CapsLock: its down, repeats and up are consumed (Windows never sees them, so it
never toggles), and the core remembers that it is held. A chord with CapsLock matches only while
CapsLock is held; a chord without it only while CapsLock is up. Other modifiers keep their rules:
`CapsLock+W` → `Up` is a remap whose trigger has no other modifier, so it widens (§3.2) and
Shift + CapsLock + W selects (Shift + Up); an exact `CapsLock+Shift+Comma` wins over
`CapsLock+Comma`. With CapsLock held, keys without a CapsLock rule reach the application
unchanged. This is done **before** the pause check, so a paused engine still owns CapsLock and
still matches its engine-control rules: CapsLock + Backspace resumes. A key reset (another
desktop, an unlock) forgets that CapsLock is held; its up stays consumed. The emergency chord is
checked first, as before; after it everything passes until the hook is gone.

A modes-based layer (ADR 0096) stays for other held keys (`Tab`, `Space`); the editor saves
"Hold + press" with CapsLock as the native chord instead, without a mode.

### 2. `settings.capslock`: `default` or `always_off`

- `default` (absent): CapsLock toggles as Windows does. While a CapsLock chord rule is active, a
  tap of CapsLock alone (no other key while it was held) is sent on as a tap, so CapsLock alone
  keeps working.
- `always_off`: while the engine is on (running or paused) CapsLock never reaches Windows — the
  layer swallows its tap, and without a layer a CapsLock no rule took is consumed with its up.
  When the engine starts or resumes, after an unlock and when a reload chooses the policy, the
  tray app reads the toggle (`GetKeyState`, checked on this PC from a process without focus) and
  turns it off with one marked tap; the time and outcome are in `status` (`engine.capslock`).

**Scope, stated in Settings and here:** nothing is promised while the engine is off, after the
emergency chord, while Ddalgak is not running or crashed, on the secure desktop (the hook sees
nothing there; CapsLock pressed on the lock screen is turned off after the unlock), or for an
elevated foreground window that refuses the tap (UIPI).

### 3. "Automation off" is the existing pause

The engine already has a control-only state: `paused` keeps the hook, releases everything held
and runs only engine-control rules (rules whose steps are all `engine` steps). The editor's
action **Automation on/off** saves `engine` `toggle_pause`, so CapsLock + Backspace turns the
automation off and on. No second engine, hook or process was added. What stays active while the
automation is off: CapsLock as the policy says, the engine-control rules (listed in Settings →
General → *Works while automation is off*) and the emergency chord.

The safety rules stay: no rule turns the engine **on** (`DDALGAK-FR-004`), so after the
emergency chord, a lost hook or a crash the engine is off and CapsLock + Backspace does nothing
(verified on the real desktop: the chord reached Windows, the engine stayed off with
`off_reason: emergency`, the next start stayed off until the user turned it on in Settings). A
pause is not remembered across restarts; a turn-off by the user is (Step 73).

**The shortcut cannot be taken away by accident.** The editor refuses to save another enabled
rule on the same keys (same `on`, modes, no condition) and names the rule it would collide with
("these keys turn automation off and on (rule “…”)"); validation's `duplicate_trigger` makes a
later duplicate inactive in any case.

### 4. `engine` `open_settings`

A new `engine` op opens the rule editor's Settings window: the tray app starts
`ddalgak-editor.exe --settings` (a debug build on its own `--endpoint` passes it on); a running
editor is told through a second named event and opens Settings on its next frame. Like every
`engine` step it needs no consent and works while paused. The debug test host opens no window.

### 5. Editor

- **Actions:** new cards *Media control* (Play/Pause, Next, Previous, Stop → `send_keys` of one
  media key, once per press), *Volume* (Up/Down repeat while held → `trigger.repeat: true`; Mute
  once), *Open Settings*; *Pause Ddalgak* is renamed *Automation on/off* with "Turn on/off" as
  its default. A loaded rule maps back to these cards from its step.
- **Key picker:** CapsLock with another key is held as a modifier (`CapsLock+W`), alone it is the
  key; a *Symbols* tab with `, . / ; ' [ ] \ ` - =` and `< > ?` (Shift + the key, removed again
  when another symbol is chosen). Keycaps and readable names show `CapsLock + <`, `CapsLock + /`.
- **Capture box:** asks Windows which side is held when a key arrives: a right-side modifier
  alone is recorded as `RShift`/`RCtrl`/`RAlt`, a held CapsLock makes `CapsLock+…`, `/` and
  other punctuation become `code:` keys. When Ddalgak itself owns CapsLock, Windows never sees it
  held, so such chords are picked by clicking.
- **Sentences:** the rule in words in the window's language — "When CapsLock is held and W is
  pressed, send Up arrow." / "CapsLock을 누른 채 W 키를 누르면 Up 화살표 키를 보냅니다." — in the
  editor, and (Korean) in the rule list, where typed text is counted, never shown
  (`DDALGAK-SEC-011`).
- **Measured fixes from the real-desktop run:** a chosen action scrolls its settings into view
  (they were ~700 px below a 900 px window); quick target keys under *Acts as* (arrows, Home,
  End, Page Up/Down, Escape, Enter) — a CapsLock + W → Up rule went from 12–13 clicks and four
  screens to 10 clicks and three; a typed-text rule's automatic name says its keys (three rules
  were all called "Type a phrase").
- **Settings → General:** *Keep CapsLock off*, *CapsLock as a modifier* (how many rules, what a
  tap does, CapsLock's state now), *Works while automation is off*, and the engine's state with
  why it is off (the emergency chord, a lost hook).

### 6. Real-desktop input tests (debug builds only)

`ddalgak-gui --test-physical-input` (compiled out of release builds) makes key events tagged
with `hook::TEST_INPUT_MARKER` ("DDTS") in `dwExtraInfo` count as physical keys. Everything
else is the real path: the hook, the core, the worker, `SendInput` with Ddalgak's marker and the
target application. A release build has no such switch, so no program can make its injected
keys trigger rules that ignore injected input.

## Consequences

- A profile can mix CapsLock chords with a rule on CapsLock itself; the CapsLock rule then never
  fires (the layer takes CapsLock first). Validation says so: `trigger_shadowed` (a warning, with
  rebind, disable and keep offers) names the rule that holds CapsLock as a modifier (2026-11-17).
- `<` and `>` are stored as `Shift+Comma` / `Shift+Period`: the key, not the character. On a
  layout where `<` is another key (German `IntlBackslash`) the rule still fires on Shift+Comma
  and the readable name assumes a US-style layout.
- Pausing from CapsLock + Backspace lets CapsLock-held keys through as plain keys (W types `w`):
  no rule runs while paused, by design.
- The status contract gained `engine.capslock`; the rules schema gained `settings.capslock`,
  the `open_settings` op and the CapsLock trigger form.
