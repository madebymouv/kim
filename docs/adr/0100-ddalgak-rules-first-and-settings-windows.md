# ADR 0100: Ddalgak's main window is for rules; Settings is a window of its own

- Status: accepted
- Date: 2026-11-16
- Issue: maintainer task "Kim — Ddalgak Complete UX Reconstruction and Unified Settings
  Architecture" (2026-11-16). Extends ADR 0092 (design system), ADR 0096 (Ddalgak's rail and
  the gear page, superseded here for the main window's content) and ADR 0099 (one scale model,
  the measured UI audit). Requirements: `DDALGAK-FR-050…053`, `DDALGAK-FR-004`,
  `DDALGAK-FR-030…032`, `DDALGAK-FR-061`, `DDALGAK-NFR-007`, `DDALGAK-SEC-013`, `KIM-FR-009`.
  Evidence: [DDALGAK_UI_AUDIT](../DDALGAK_UI_AUDIT.md).

## Context

The measured before-state ([DDALGAK_UI_AUDIT §1](../DDALGAK_UI_AUDIT.md)) showed why earlier
design-system work did not reach Ddalgak:

1. **The editor mostly did not use the design system.** 214 calls drew raw egui widgets
   (`egui::Button`, `TextEdit`, `ComboBox`, `checkbox`, `radio_value`, `DragValue`) against 37
   uses of `kim-ui` components. Raw widgets took egui's 24 px interact height and its own frames,
   and they recorded nothing for the UI audit — so the audit (ADR 0099) passed while it could
   not see most of the window. The Rules toolbar had a 24 px search field beside 28 px buttons.
2. **The main window carried everything**: the engine switch, Pause, profile and mode labels
   in the rail, Rules/Modes/Profiles/Playground pages, Settings as a page that replaced the
   content, and three always-visible text buttons on every rule row.
3. **The rule editor was a four-step wizard** (Trigger → Action → Details → Test) whose steps
   hid each other, with every key field doubled by a free-text field and technical options
   (on press/release, pass-through) next to the essentials.
4. **The key picker** was one flat, searchable list; left/right modifiers sat beside
   either-side ones, keys could not be removed one by one, and Confirm accepted chords the
   engine refuses (`LShift+RShift`, reserved `RAlt+Tab`).

## Decision

1. **Main window = rules.** The rail shows the name, the engine's state as a status (a dot and
   a word, never a switch), the profile and mode only when there is a choice, and two primary
   activities: Rules and Playground. Modes and Profiles leave the rail.
2. **The Rules page** is a title with the count, **one toolbar** — a search field that fills
   the row (magnifier, clear button, Esc clears), Filters (an icon button with an active-filter
   count opening a popover with the real filters), the primary **New Rule** — every control
   `size::CONTROL` high on one centre line, and the list: a row of `size::LIST_ROW_DETAIL` with
   the name and keycaps over a one-line summary, badges, the on/off switch and a `⋯` menu (Edit,
   Duplicate, Enable/Disable, Test in Playground, Delete). A click opens the rule;
   Ctrl/Shift+click selects; the drag handle and Alt+Up/Down keep the order.
3. **The rule editor is one page** reading *When I press [keys] → [action]*: the sentence at the
   top, then four numbered sections — **Trigger** (Keys / Typed text / Schedule; how the keys
   are pressed; the key field with record, the key picker and an optional *Type* text field;
   trigger options folded), **Action** (every action type as an equal-size choice card),
   **the action's settings** (only that action's fields; conditions, priority, description and
   macro steps under *Advanced*), **Test and save** (simulation, problems, conflicts,
   permissions) — and a fixed footer with Cancel and Save (new: *Save turned off* / *Save and
   enable*). Problems appear once the user changed something; Save stays off until they are
   solved. The data model and the store transaction are unchanged.
4. **The key picker** shows the chosen keys as removable chips with the chord in words, offers
   categories (Common — the left/right modifiers, CapsLock, Tab, Space, Enter, Escape,
   Backspace, Delete, arrows —, Modifiers, F1–F24, letters and digits, navigation, mouse, more)
   and a search across all keys, keeps left and right modifiers distinct (an either-side `Ctrl`
   replaces `LCtrl`/`RCtrl`), and asks the program before Confirm: Ddalgak's own trigger parser
   and its reserved-chord check (`ddalgak::trigger::Trigger::parse`), so a picked chord is one
   the engine can run. Recording and clicking coexist.
5. **Settings is a separate native window** (an egui immediate viewport titled *Ddalgak
   Settings*, `size::SETTINGS_WINDOW_WIDTH × HEIGHT`; an embedded window where the backend has
   one window, as in the headless tests). The gear in the main window's lower-right corner opens
   it or brings it forward; Esc or the close button closes it; the main window keeps its state
   underneath; dialogs opened from Settings open in it. Sections: **General** (Turn on Ddalgak
   and Pause automation as two switches that show the engine's answer — while a request is out
   they stay as they were, disabled, saying they wait; no answer within 5 s is reported with
   *Try again*; without the tray app both are disabled and say why —, the emergency chord,
   start at sign-in through the existing Run-key store), **Profiles**, **Modes** (each mode's
   switch, *Switch* through the app's `mode.set`), **Shortcuts** (Ddalgak's global shortcuts
   in `hotkeys.json` set with the key picker and checked by the hotkey contract; the fixed
   emergency chord; the rule list's keys), **Appearance**, **Advanced** (rules import/export,
   release held keys, the rules folder, keyboard help). The emergency chord stays a physical-key
   check in the engine: it never depends on the editor or Settings.
6. **Kim Manager follows the same convention**: Settings opens as *Kim Settings* in its own
   window (the launcher no longer turns into Settings and grows); Noeul's settings already are
   a separate program window; Subak keeps its native gear menu (no panel over the terminal).
7. **The audit sees every control.** New shared components (search field, icon buttons,
   checkbox, number field, text area, choice card, labelled field, form section, menu item,
   popover, status indicator, count badge) record their geometry; egui's own widgets take
   `size::CONTROL` as their interact height; new rules check toolbar height and alignment, the
   gear's corner, choice cards and the form column width; a Settings window is measured and
   photographed on its own (its real pixels copied from the screen during an audit only, since
   eframe takes no screenshot of a child viewport).

## Consequences

- Rules, the editor and Settings use no raw egui widgets; the playground's and some dialogs'
  remaining text fields and combo boxes are egui widgets at the design system's control height
  (listed in DDALGAK_UI_AUDIT §6).
- Shortcut changes in Settings apply when the tray app starts next (it reads `hotkeys.json`
  once); the page says so.
- `RAlt+Tab` and other reserved chords can no longer be picked; recording them shows the same
  problem in the form.
- Tests drive the new structure (Settings as an embedded window in the harness); the old page
  enum values `Modes`, `Profiles`, `Settings` are gone (`SettingsSection` replaces them).
