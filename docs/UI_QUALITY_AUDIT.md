# UI quality audit

Status: **current** · 2026-11-15 · [ADR 0099](adr/0099-one-scale-model-measured-ui-audit.md) ·
How to reproduce: [UI_QUALITY_PIPELINE](UI_QUALITY_PIPELINE.md) (`cargo xtask ui-audit`) ·
Research behind the decisions: [UI_ENGINEERING_RESEARCH](UI_ENGINEERING_RESEARCH.md)

Why earlier UI work kept failing, with evidence from the real programs on this PC (one monitor,
5120 × 1440, work area 5120 × 1410, Windows display scaling 100 %, Windows 10 19045), and what was
changed. Every "before" value below was measured on a real executable — the installed Subak
0.0.1, the 0.0.3 release build of `main` at ab491c6, or the debug builds before the fix — never
taken from token tables.

## 1. Root causes

| # | Cause (from the task's list) | Evidence | Verdict |
|---|---|---|---|
| 1 | Incomplete component adoption | Subak's tab bar was not a design-system component at all but VT text in a one-row terminal grid (`tabs.rs` built `\x1b[…m` strings, `window.rs` rendered them with the terminal renderer at `BAR_FONT = 12`). | **Confirmed** |
| 2 | Independent layout calculations | Four zoom formulas on one screen: `kim_manager::auto_scale` 275 %, `noeul_settings::window_scale` 200 %, `ddalgak_editor::window_scale` 175 %, `subak::gui::layout::screen_zoom` 175 % (each "fit my own window into N % of the work area"). | **Confirmed**, the main cause of "inconsistent sizes" |
| 3 | Incorrect DPI or UI scaling | Subak 0.0.1: chrome × 1.75 while the user's chosen 13 px text stayed 13 px → a 56 px bar with 21 px tab text over 13 px terminal text. Subak 0.0.3 (ADR 0098): chrome × 1.0 while the default text was × 1.75 → 12 px tabs over 23 px text. Two opposite errors from treating UI scale and text scale as one factor, then as unrelated. | **Confirmed** |
| 4 | Hardcoded sizing values | `BAR_HEIGHT_COMPACT = CONTROL_COMPACT` (24, too low for a 24 px close target plus inset); text field margin `5` giving 26 px fields instead of the 28 px control height; `Button::compact` min 48 not in the tokens. | **Confirmed**, fixed |
| 5 | Inconsistent font metrics | The tab label used the terminal's monospace family (Cascadia Mono, bold for the active tab) while every other control uses Segoe UI. Widths were multiples of the 7–8 px cell, so tab widths could not hit 96–220 px exactly. | **Confirmed** |
| 6 | Different rendering backends | egui (glow) for Kim, Noeul settings, the Ddalgak editor; GDI/software frames for Subak. Each had its own idea of text. Not a problem in itself once both consume the same tokens and the same scale model; the problem was that the Subak chrome used the *terminal* backend. | Partly: fixed by a GDI UI-text adapter (`gui/uitext.rs`) |
| 7 | Missing responsive constraints | Kim Settings: a segmented choice drawn over its row's title when the content was narrow; `Done` drawn over the *About* section in a short window. | **Confirmed by the new audit**, fixed |
| 8 | Wrong window size assumptions | Each program sized its first window from its own base size × its own zoom; Subak's first window was 1400 × 1050 with 23 px text on a 1440-high screen. | **Confirmed** |
| 9 | Insufficient automated UI testing | 289 Subak unit tests, 47 window tests, kittest suites — none read back the laid-out geometry of a real window or compared pixels with a reviewed reference. | **Confirmed** |
| 10 | Testing internal calculations instead of rendered UI | ADR 0098's own test asserted the bar's height in a dump, not what was drawn; the 0.0.3 "tiny tabs" problem passed it. | **Confirmed** |
| 11 | Mock-ups not matching the application | `design/mockups/*.png` are generated from tokens, not from the programs; they showed proportions the programs did not have. | **Confirmed**; screenshots of the real programs replace them as evidence |
| 12 | Stale executables tested | The maintainer's running Subak is 0.0.1 (built 2026-11-14); fixes in 0.0.2/0.0.3 were never seen there. Every measurement here records the executable's path and SHA-256. | **Confirmed** (process, not code) |

## 2. Measurements before the change

Subak, sandboxed copy of each executable, private endpoint, first-window sizing, 5 Terminals
(`main`, `build`, `로그`, `ssh-prod`, `a-much-longer-terminal-name`). Pixel rows were read from
screenshots of the window only (PrintWindow).

| Value | 0.0.1 (installed, SHA-256 `FC366B64…`) | 0.0.3 (`main` ab491c6, `182FDA25…`) |
|---|---|---|
| Window / client | 1592 × 1050 / 1576 × 1011 | 1592 × 1050 / 1576 × 1011 |
| Native caption above the bar | 30 px | 30 px |
| Tab bar height (bg.raised band + border) | **56 px** | **32 px** |
| Tab label | Cascadia Mono 21 px (12 × 1.75), active bold | Cascadia Mono 12 px |
| Terminal text / row height | 23 px / 30 px (default × 1.75) | 23 px / 30 px |
| Tab label ÷ terminal text | 0.91 (and 1.6 with the user's 13 px text) | 0.52 |
| Tab width | up to 385 px (220 × 1.75) | 216–220 px, cell multiples |
| Marks per inactive tab | green ● + accent • (two dots) | same |
| UI zoom of the other programs on this screen | Kim 275 %, Noeul settings 200 %, Ddalgak 175 % | same |

The maintainer's real settings (read only): Subak `appearance.font_size = 13`, Kim
`scale = 150`. With 0.0.1 that produced the reported "Subak's tabs alone are huge": a 56 px bar,
21 px labels, over 13 px text.

## 3. Defect table

| Application | Screen / component | Observed defect | Code path | Root cause | Expected | Fix | Verification |
|---|---|---|---|---|---|---|---|
| Subak | Tab bar | 56 px bar, 21 px mono labels over 13 px text (0.0.1) | `window.rs::rebuild_bar` (scale × `screen_zoom`) | §1 #3 | bar proportional to text | `chrome_zoom = min(UI zoom, text/13)` | `the_tab_bar_follows_the_ui_zoom_but_never_outgrows_the_text`; audit run `subak-5tabs-zoom175-font13` |
| Subak | Tab bar | 12 px labels under 23 px text (0.0.3) | same, ADR 0098 | §1 #3 | same | same; default text = 13 × UI zoom | audit run `subak-5tabs-zoom150` |
| Subak | Tab labels | monospace, bold, cell-quantised widths | `tabs.rs` VT layout + terminal renderer | §1 #1, #5 | UI font, exact token widths | pixel layout + GDI UI text (`tabs.rs`, `uitext.rs`) | `tabs::tests::*`, `uitext::tests::*`, screenshots |
| Subak | Tab marks | two dots on every tab | `tabs.rs` status ● always drawn | design noise | marks only for non-normal states | running has no mark | `marks_show_shapes_for_states_and_running_needs_none` |
| Subak | Overflow | empty arrow-sized gap before the first tab when scrolled to the start | first pixel layout draft | layout bug found on the real screen | first tab at the inset | arrow only when tabs are hidden on the left; `leading-gap` audit rule | `many_tabs_scroll_with_the_active_one_visible`, audit |
| All | Window zoom | 275 / 200 / 175 / 175 % on one screen | four `*_scale` functions | §1 #2 | one zoom | `kim_design::scale::auto_zoom` / `ui.json` | `scale::tests::*`, Kim `the_size_follows_the_screen_or_the_choice_and_persists` |
| Kim | Settings → General | segmented control over its row's title in a narrow content area | `kim_ui::setting_row` | §1 #7 | control below the text | rows stack when the text would get < 160 px | audit `kim-settings-*` (FAIL → PASS) |
| Kim | Settings rail | `Done` covering *About* in a short window | `kim-manager::settings_view` | §1 #7 | never overlap | Done after the sections when there is no room | audit `overlap` rule (FAIL → PASS) |
| Noeul | Capture / Advanced | text fields 26 px (token 28) | `kim_ui::text_field` margin | §1 #4 | 28 px | `min_size(CONTROL)`, centred text | audit `token-height` (FAIL → PASS) |
| Noeul | Capture | "Apply" 48 px wide flagged against the 72 px minimum | `Button::compact` | rule not in the tokens | compact minimum as a token | `size::BUTTON_MIN_WIDTH_COMPACT = 48` | audit `token-width` |
| Ddalgak | New rule | a lone "—" under the steps; errors shown before any input | `app/view.rs::form_ui` | UX defect | quiet until the user acts | sentence only when it says something; no errors on an untouched new rule | screenshots, `flows` tests |
| Ddalgak | Rules (empty) | left-aligned sentence, plain buttons, unbalanced | `app/view.rs::empty_state` | not the design system's empty state | centred title, one sentence, quiet actions | redesigned | screenshot `ddalgak-rules-*` |
| Ddalgak | Engine switch row (disabled) | audit read the colour before egui's disabled fade | `kim_ui::inspect` | measurement error | record the painted colour | records the faded colour | audit `disabled-distinct` |

## 4. Measurements after the change

From `cargo xtask ui-audit` (geometry reports written by the programs' own layout code; Subak's
font sizes are GDI's, the line height `GetTextMetrics`' `tmHeight`).

| Fixture | Bar | Tab height | Label | Terminal text | Label ÷ text | Close target |
|---|---|---|---|---|---|---|
| 100 % (default text 13) | 36 px | 32 px | Segoe UI 12 px, line 16 | 13 px, cell 8 × 17 | 0.92 | 24 × 24 |
| DPI 125 % | 45 px | 40 px | 15 px | 16.25 px, cell 9 × 21 | 0.92 | 30 × 30 |
| DPI 150 % | 54 px | 48 px | 18 px | 19.5 px, cell 12 × 27 | 0.92 | 36 × 36 |
| DPI 200 % | 72 px | 64 px | 24 px | 26 px, cell 15 × 34 | 0.92 | 48 × 48 |
| UI zoom 150 % (this PC's Auto), default text 20 | 54 px | 48 px | 18 px | 20 px | 0.90 | 36 × 36 |
| UI zoom 175 %, user text 13 (the maintainer's config) | 36 px | 32 px | 12 px | 13 px | 0.92 | 24 × 24 |
| Compact | 28 px | 24 px | 12 px | 13 px | 0.92 | 24 × 24 |

All 88 runs of the full matrix (§6) pass the layout rules (0 failures, 0 warnings); the result file is
`target/ui-audit/report.txt`. Real-desktop screenshots of the rebuilt window (sandboxed, on
screen): the bar reads as part of the window at 150 % with 20 px text, and as a slim 36 px bar
over 13 px text.

## 5. What the tests did not see before, and do now

- Geometry is read back from the real layout (egui `Response` rectangles, painter text
  rectangles, GDI-measured text) — not recomputed from tokens.
- Each view is opened in the real executable with fixtures, and its screenshot is taken from the
  same frame (egui's own screenshot, Subak's composed client frame).
- The checks cover overlap and clipping across component types, not only token arithmetic.
- Curated baselines in `design/ui-baselines/` are compared pixel by pixel; changes need review.

## 6. Coverage

`cargo xtask ui-audit` (full): Kim — launcher, Settings General/Shortcuts/Catalog at 100, 125,
150, 200 % dark English plus 150 % light Korean; Noeul settings — all six sections, the same
fixtures; the Ddalgak editor — rules, new rule, profiles, settings, the same fixtures; Subak —
1/5/20 tabs at DPI 100/125/150/200 %, UI zoom 150 %, zoom 175 % with 13 px text, compact,
light, narrow (520 px) and wide (2400 px). `--quick` runs 100 % and 150 % only.

Not covered by fixtures yet (reviewed by hand or listed as remaining): egui context menus and
popovers, Kim's install/uninstall dialogs, Noeul's capture overlay and floating pins (native
layered windows; their own tests check geometry), Ddalgak's trigger picker dialog, physical
second monitors (this PC has one), Windows High Contrast on the real desktop.

## 7. Remaining issues

- Subak keeps the native caption above the tab bar (two chrome rows, 30 + 36 px at 100 %).
  Integrating tabs into the caption needs a custom DWM frame (hit-testing, snap layouts,
  accessibility); recorded as the next Subak chrome step, not attempted here.
- The UI zoom is read when a window opens; moving a window to a monitor with another work area
  does not re-zoom it (DPI changes are handled).
- The audit's egui runs place windows on the visible desktop for a moment (an off-screen GL
  window is not guaranteed to render); Subak runs off-screen.
- Subjective review (balance, clutter) is done on the screenshots by a person; the audit does not
  score beauty.
