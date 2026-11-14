# ADR 0096: The Ddalgak rule editor: no top bar, a clean rule list, a rule page, key combinations

- Status: accepted
- Date: 2026-11-15
- Issue: maintainer request 2026-11-15 (§9 remove the top bar and move settings, §10 simplify the
  Rules screen, §11 multi-key triggers and special keys, §12 redesign rule creation); extends
  ADR 0049 (the rule editor), Step 72 (modes and hold layers) and ADR 0092 (design system)

## Context

The editor opened on a top bar holding the engine's state, its buttons, the profile, the mode,
a language switch and help; the rules page had a search, New, Import and Export and three
filter dropdowns always visible; every row carried a checkbox, the keys and summary as one
string, its state and badges; selecting a rule opened a detail panel on the right, and the rule
form lived in that panel, with radio buttons, check boxes and technical options all shown at
once. Navigation, buttons and tiles were translated into Korean, unlike the other Kim programs.
Triggers already supported left/right modifiers, `CapsLock`, sequences and mouse buttons, but
only by typing them or through a row of "special keys" buttons; a non-modifier key held with
another (AutoHotkey's `a & b`) was not a trigger (DDALGAK_AHK_MATRIX 1.10: modes `hold` cover it).

## Decision

1. **No top bar.** A left rail (the design system's navigation) shows *Ddalgak* with one switch
   that turns the engine on or off — it always shows the engine's real state (`engine.status`
   of the running tray app); while the tray app does not run the switch is disabled and says
   why — a Pause/Resume button while it runs, the profile and mode, and the sections Rules,
   Modes, Profiles, Playground. The status line stays at the bottom and ends with the settings
   gear in the window's lower-right corner.
2. **Settings** (the gear): Theme (system, dark, light), Language (system, English, Korean;
   saved with the theme in `editor.json` next to `rules.json`), the engine notes (the emergency
   chord), Import and Export of rules, keyboard help.
3. **The rule list** is one line per rule: its switch (enabled), its name, its keys as keycaps,
   the effect in a few words, problem marks with their text in a tooltip, and Edit, Duplicate,
   Delete. A click opens the rule; `Ctrl`/`Shift`+click selects for the bulk bar. The search and
   *New rule* are always there; the filters (state, profile, mode, app, grouping, text search)
   appear behind *Filters* or while one is set. Empty list: the starters as before.
4. **The rule page.** A rule opens in the page itself (← Rules returns), with its four steps as
   one control: **Trigger → Action → Details → Test**. The trigger step first asks *what*
   starts the rule (keys, a typed text, a schedule), then *how* keys combine:
   - **Together**: a chord (`Ctrl+Shift+K`, `RCtrl+CapsLock`: left and right keys stay apart);
   - **One after another**: a sequence (`Ctrl+K U`), never confused with a chord;
   - **Hold + press**: one key held (any key, e.g. `CapsLock`, `Space`, `Tab`) while another
     is pressed. Saved as the engine's tested hold layer: a mode `hold-<key>` titled *Hold
     <key>*, a rule that holds it while the key is down, and this rule limited to that mode,
     all in one commit; the list shows it as `CapsLock + J`. The held key then opens the layer
     and no longer acts alone — the step says so.
   Keys are recorded by pressing them in the field (as before) or chosen with **Pick…**, a key
   picker with left/right modifiers, `CapsLock`, `Tab`, `Space`, the arrows, the mouse buttons
   and every contract key, searchable, with Clear, Cancel and Confirm. Release/hold timing and
   passing the key through are under *Advanced*. The action step's tiles and the details step's
   fields are unchanged (the existing data models validate them); duplicate triggers, reserved
   chords (`Win` chords, `Alt+Tab`, the emergency chord) and other problems are reported inline
   by the engine's own validation, unsaved changes are protected, and the test step simulates
   before anything is saved.
5. **Names stay English** in both languages (sections, buttons, steps, action tiles);
   descriptions, hints and messages follow the language (ADR 0092 §3).

## Consequences

- The main screen is a list that can be scanned; configuration lives in Settings; a rule gets
  the whole page while it is edited.
- Hold + press works without a new hook path: the engine's hold layers are already tested
  through the real hook (Step 72). The cost is that the held key loses its own function; a
  dual-role (tap) prefix is the remap `tap` option, documented, not part of this choice.
- Saved rules, modes and profiles are unchanged; rules made before keep working and open in the
  new page (a rule limited to a hold layer is shown as *Hold + press*).
- Tested: the editor's flow tests (25: among them the engine switch following and driving the
  app's state, Hold + press saved as a layer with keys picked — `RCtrl+J` kept side-specific —
  and read back as Hold + press, the list's row buttons, the rule page's Back asking before
  discarding, filters behind *Filters*, Import and Export in Settings, the language and theme
  saved in `editor.json`, names English and explanations Korean), its unit tests (43), the e2e
  scenario `adr0096_ddalgak_hold_press` (the engine's simulation: J alone reaches the app,
  CapsLock opens the layer, J in it becomes Down; checked by hand with the same commands), and
  the real desktop (sandbox profile, private endpoint): the new rail, list, Settings and rule
  page; Hold + press built with the key picker (CapsLock) and a real J press, the remap to Down,
  the consent dialog (Cancel focused), saved as the layer, its hold rule and the rule; the
  CLI's `rule list` reported both rules with no problem.
