# ADR 0098: Subak's tab bar keeps the design system's size on a large screen

- Status: superseded by [ADR 0099](0099-one-scale-model-measured-ui-audit.md) (the bar follows the shared UI zoom, capped by the terminal text)
- Date: 2026-11-15
- Issue: maintainer report 2026-11-15 ("subak의 상단탭 크기가 아직도 혼자서 엄청 큰데?");
  supersedes the tab-bar sentence of ADR 0091 §Decision ("The tab bar grows by the same factor")

## Context

ADR 0091 made Subak's default text larger on a large screen (a zoom of 1.0–2.0 from the work
area: 1.75 on 5120 × 1410 at 100 %) and multiplied the tab bar by the same zoom. ADR 0095 kept
that multiplication while it moved the bar to the design tokens, so on the maintainer's monitor
the bar was 56 px high, its text 1.75× and each tab up to 385 px wide — larger than the tab bars
and controls of every other program on the same screen, Kim's own included.

## Decision

The tab bar (height, padding, tab widths 96–220 logical px, the gear, the active tab's marker
and the tab text) follows only the monitor's DPI, as every other Kim window's controls do. The
terminal text still grows on a large screen (ADR 0091 for the text is unchanged), and the
window's own zoom keys (`Ctrl` + `=` / `-` / `0`) still change only the terminal text.

## Consequences

- On every screen the bar is 32 logical px high and its tabs share the width between 96 and 220
  logical px each; on a large screen the bar is no longer the largest thing on the desktop.
- Tested off-screen: `gui_window::the_tab_bar_keeps_its_size_on_a_large_screen` starts the test
  window with Subak's screen zoom set to 1.0 and to 1.75 (debug builds only:
  `SUBAK_TEST_SCREEN_ZOOM`, honoured only with `--test-offscreen`): the bar's height and its text
  cell are equal, the terminal text is larger; with the old multiplication the test fails.
- Not tested on the real desktop: the maintainer was using the mouse, keyboard and screen.
