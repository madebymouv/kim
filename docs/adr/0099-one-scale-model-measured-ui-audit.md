# ADR 0099: One scale model, a pixel tab bar in the UI font, and a measured UI audit

- Status: accepted
- Date: 2026-11-15
- Issue: maintainer task "Kim — Complete UI/UX Reconstruction with Measurable Visual Quality"
  (2026-11-15); supersedes ADR 0098 (the Subak tab bar at DPI only), the per-program zoom rules
  of ADR 0091, and the "one row of terminal cells" tab bar of Step 17 / ADR 0095. Requirements:
  `KIM-FR-009`, `KIM-NFR-012`, `KIM-NFR-013`, `SUBAK-FR-031`, `SUBAK-FR-048`, `DDALGAK-FR-050`. Evidence:
  [UI_QUALITY_AUDIT](../UI_QUALITY_AUDIT.md), research:
  [UI_ENGINEERING_RESEARCH](../UI_ENGINEERING_RESEARCH.md), workflow:
  [UI_QUALITY_PIPELINE](../UI_QUALITY_PIPELINE.md).

## Context

The design system (ADR 0092) existed in tokens, docs and tests, yet the programs did not look
consistent, and Subak's tab bar was wrong twice in a row: 56 px with a 21 px monospace label in
0.0.1 ("혼자서 엄청 큰"), 32 px with a 12 px monospace label under 23 px terminal text in 0.0.3.
Measuring the real windows ([UI_QUALITY_AUDIT §2](../UI_QUALITY_AUDIT.md)) found the causes:

1. **Four independent zoom formulas** (ADR 0091): Kim 275 %, Noeul settings 200 %, the Ddalgak
   editor 175 %, Subak 175 % for its text — on the same 5120 × 1410 screen. No shared scale.
2. **Scale applied to the wrong things.** Subak multiplied its chrome by the screen zoom while
   the user's own 13 px text did not grow (0.0.1); ADR 0098 then removed the zoom from the
   chrome while the *default* text kept it (0.0.3). Terminal text scale and UI scale were treated
   as one factor, then as unrelated ones.
3. **The wrong rendering backend for chrome.** The tab bar was a one-row terminal grid: the
   monospace terminal font, widths quantised to cells, the line height of the terminal font —
   no way to use the design system's UI typography.
4. **Tests measured intentions, not output.** Unit tests checked token arithmetic; nothing read
   back the geometry a window really laid out, and nothing compared rendered pixels with a
   reviewed reference. Defects such as Kim's settings rows overlapping their text in a short
   window, a `Done` button covering a section, 26 px text fields and premature validation errors
   in the Ddalgak editor were invisible to the test suite.

## Decision

1. **One scale model** (`kim_design::scale`, DESIGN_SYSTEM §2.7). Physical = logical × DPI scale
   × zoom, each factor applied once:
   - *DPI scale*: Windows' per-monitor setting; never changed by Kim.
   - *UI zoom*: one value for every program — the user's choice in `%APPDATA%\mouv\ui.json`
     (Kim → Settings → General → Size writes it: Auto, 100/125/150/175/200 %) or
     `auto_zoom(work area in logical px)` = height ÷ 900, limited by width ÷ 1200, in 25 % steps
     between 100 % and 200 % (150 % on this PC, 100 % on a 1080p screen). The work area is
     converted to logical pixels first (`system::work_area_logical`), so a 200 % monitor is never
     zoomed twice.
   - *Terminal text*: Subak's `appearance.font_size` (the user's), default `13 × UI zoom`.
   - *Chrome cap*: Subak's tab bar uses `chrome_zoom = min(UI zoom, text ÷ 13)` in whole steps,
     so the bar is proportional to the text it frames: never larger than the text (the 0.0.1
     defect), never tiny under large text (the 0.0.3 defect).
2. **Subak's tab bar is laid out in pixels and drawn in the UI font** (`gui/tabs.rs`,
   `gui/uitext.rs`): GDI Segoe UI (Variable when installed; Korean through Windows' font
   linking), Windows' icon font for ×, +, the gear and the chevrons, measured with the same GDI
   fonts that draw it. Tokens: bar 36 (compact 28), tabs 32 high below a 4 px inset, label body
   12, padding 12, close target 24 with a 10 px glyph, `+` and gear 28, tabs 96–220 wide and all
   the same width, rounded top corners (radius 6) on the active and hovered tab, a 2 px accent
   marker. A running Terminal (the normal state) has no mark; every other state keeps its shape.
   Hit testing uses the same rectangles that are painted (2-D, the close button inside its tab).
3. **Geometry inspection** (`kim_design::inspect`, `kim_ui::inspect`, Subak's `geometry()`):
   a development-only report (schema `kim.ui-geometry`) of what each view really laid out —
   bounds, hit areas, measured text boxes, font sizes, clip state, colours, states — written to
   `KIM_UI_INSPECT` with a screenshot of the same frame in `KIM_UI_SNAPSHOT`. Off by default: one
   cached environment check, no allocation, no file.
4. **Automatic checks** (`kim_design::audit`): token heights and widths at the report's scale,
   containment in parent and window, hit-target minimum, text clipping, type scale, minimum
   readable size, vertical centring, uniform repeated components, even spacing, leading gaps,
   overlapping pointer areas (any type, same parent), WCAG contrast, palette membership,
   disabled-state distinctness — each with its own tolerance (1 physical px for geometry, 0.6 px
   for font sizes).
5. **`cargo xtask ui-audit`** opens every audited view of the real debug executables (sandboxed
   profiles, private endpoints, no hotkeys; Subak off-screen in its test mode, with real
   Terminals of a private Host) over a fixture matrix of zoom/DPI 100–200 %, dark and light, English
   and Korean, narrow and wide windows, 1/5/20 tabs and compact density; checks each report; and
   compares curated screenshots with `design/ui-baselines/` (≤ 0.5 % of pixels may differ by more
   than 40 levels; a larger change is reported for review and never approved automatically,
   `--bless` records a reviewed one). Layout failures fail the command.
6. **Components follow the measured contract**: text fields 28 high; compact buttons ≥ 48 wide
   (a token now); a settings row whose control would leave its text less than 160 px stacks the
   control below the text; Kim's settings rail never puts `Done` over a section; the Ddalgak
   editor shows the rule's sentence only once it says something, no validation errors on an
   untouched new rule, and the design system's empty state.

## Consequences

- One number decides how large Kim looks; changing it in Kim changes every program at its next
  window. On this PC every program opens at 150 % instead of 175–275 %.
- Subak's bar matches the other programs' typography and stays proportional to the terminal
  text in every case measured (§5 of the audit). ADR 0098's test was replaced by
  `the_tab_bar_follows_the_ui_zoom_but_never_outgrows_the_text`.
- UI changes now have an objective loop (inspect → measure → fix → re-measure) that future
  sessions can run with one command; the audit found and fixed four real defects the existing
  tests missed.
- Not done: integrating Subak's tabs into the window caption (a custom DWM frame), automatic
  per-monitor re-zoom when a window moves to another monitor, and egui menus/popovers in the
  audit (context menus are not opened by the fixtures). These are listed in the audit's
  remaining-work section.
