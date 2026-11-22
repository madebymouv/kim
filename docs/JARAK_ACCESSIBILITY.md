# Jarak — accessibility

Status: **current** (Step 109, checklist reviewed 2026-11-22; contrast themes and the UI Automation tree 2026-11-23) · Requirement: `JARAK-NFR-007`
([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Design: [DESIGN_SYSTEM §4](DESIGN_SYSTEM.md),
[UI_QUALITY_PIPELINE](UI_QUALITY_PIPELINE.md)

This is the checklist `JARAK-NFR-007` asks for, with the evidence for each item and what is not
done or not tested.

## Checklist

| Item | State | Evidence |
| --- | --- | --- |
| **Every command reachable by keyboard** | Yes | The command palette (Ctrl+Shift+P) lists every command of the menus, the extensions and the languages (Step 92); the menu bar opens with Alt (Step 92/105); Settings → Keyboard can give any of the 163 commands keys (Steps 105–106); dialogs take Enter and Esc and focus their first field (Step 103 fix) |
| **Keyboard-only text work** | Yes | Selection, multi-caret, column selection, find and replace, folding, bookmarks, the Function List and Go to all have keys (`docs/JARAK_KEYS.md`) |
| **UI Automation names** | Yes, for egui widgets | AccessKit through eframe; the window tests find buttons, rows, fields, tabs and labels by their accessible names (more than 130 headless window tests use `get_by_label` / `query_by_label`); keycaps in Settings → Keyboard carry "command: keys" names (Step 105); the editor area itself is a custom-drawn surface: its text is not exposed to screen readers yet |
| **Design-system contrast** | Yes | kim-design's token pairs are checked by `cargo xtask design-check` in CI; the UI audit checks text contrast and target sizes in every view, at four zooms, in dark English and light Korean (`cargo xtask ui-audit --only jarak`; 275 runs, 0 failures at Step 105) |
| **Scaling** | Yes | Windows display scaling and Jarak's zoom (Ctrl+= / Ctrl+-) scale the whole window; audited at four zoom levels; the editor font size is a setting (8–40) |
| **Reduced motion** | Yes | "Show animations in Windows" off turns kim-ui animations off |
| **Korean input (IME)** | Yes | The composition is drawn inline and only the committed text is inserted (Step 88, window test `the_ime_composition_is_shown_and_only_the_commit_is_inserted`) |
| **High-contrast theme** | Yes (2026-11-23) | Under a Windows contrast theme Jarak draws with its system colours only (DESIGN_SYSTEM §4.3, kim-ui `follow_high_contrast`): window text on the window colour, the highlight for focus, selection, keywords and the primary action, no lilac accent. Checked: `contrast_theme_palettes_keep_their_pairs_readable` (every pair ≥ 7:1 text, ≥ 4.5:1 accent text, both High Contrast Black and White), window test `a_contrast_theme_draws_with_the_system_colours`, the UI audit's nine High Contrast Black runs (text contrast included), e2e `step109_contrast_themes_pass_the_audit`, and the release window shown with both themes' colours. Windows' own contrast theme was not switched on for the check (it re-themes the whole desktop, and switching back does not restore every personalisation exactly) |
| **Screen reader on the real desktop** | Partly (2026-11-23) | Windows' UI Automation tree of a real window, read with `UIAutomationClient` (what Narrator reads): the window's name (`note.md — Jarak`), the Settings button, the status bar items, and with the palette open its commands as named menu items (*Save*, *Save as…*, …). The editor is a focusable, focused Edit named *Text* with **no Value or Text pattern**: its text is not read (the gap below). Narrator itself was not run — its speech needs a listener |
| **Right-to-left text** | Not done | `VIEW-16`, planned for Step 110 |

## What the gaps mean for users

- People who rely on a Windows contrast theme see Jarak in their theme's colours (since
  2026-11-23); Kim Manager, Noeul's settings and the Ddalgak editor do not follow it yet.
- People who use a screen reader can operate the menus, dialogs, Settings and palette by name, but
  the document text is not read out from the editor area. The command line (`jarak info`,
  `jarak find`) and the Markdown preview's exports are the accessible routes for content today.
