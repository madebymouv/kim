# Jarak — accessibility

Status: **current** (Step 109, checklist reviewed 2026-11-22) · Requirement: `JARAK-NFR-007`
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
| **High-contrast theme** | **Gap** | DESIGN_SYSTEM §4.3 asks every Kim window to switch to the Windows contrast theme's system colours; kim-ui does not do this yet. It needs a change in kim-ui for every tool and a check with a contrast theme switched on (a temporary display setting, to be done while the maintainer is away). Until then the dark and light themes meet the token contrast pairs |
| **Screen reader on the real desktop** | **Not tested** | Narrator was not run against Jarak (the maintainer was at the PC); the names are verified headlessly through AccessKit only |
| **Right-to-left text** | Not done | `VIEW-16`, planned for Step 110 |

## What the gaps mean for users

- People who rely on a Windows contrast theme see Jarak in its own dark or light colours, not
  their theme's.
- People who use a screen reader can operate the menus, dialogs, Settings and palette by name, but
  the document text is not read out from the editor area. The command line (`jarak info`,
  `jarak find`) and the Markdown preview's exports are the accessible routes for content today.
