# UI quality pipeline

Status: **current** · [ADR 0099](adr/0099-one-scale-model-measured-ui-audit.md) · the loop every
UI change in Kim, Subak, Noeul and Ddalgak follows. Findings so far:
[UI_QUALITY_AUDIT](UI_QUALITY_AUDIT.md).

```
inspect the real UI → capture screenshots + geometry → find measurable defects
  → compare with the design system → fix the shared component / layout
  → build → cargo xtask ui-audit → review screenshots → fix the rest → verify again → commit
```

## 1. One command

```
cargo xtask ui-audit            # the full matrix (≈ 10 min)
cargo xtask ui-audit --quick    # 100 % and 150 % only
cargo xtask ui-audit --only subak|kim|noeul|ddalgak
cargo xtask ui-audit --bless    # record reviewed screenshots as baselines (never automatic)
```

It builds the debug programs, opens each audited view of the real executables, and writes to
`target/ui-audit/`:

- `report.txt` — `FAIL`/`WARN` per component with actual and expected values and the rule, then a
  `VISUAL` line per baseline comparison;
- `summary.json` — one entry per run (fixture, window, scale, counts, the program's measured facts
  such as `bar_height`, `tab_font_px`, `terminal_cell`, `bar_to_terminal_text`);
- `<run>.json` — the geometry report (schema `kim.ui-geometry`), `<run>.png` — the screenshot of
  the same frame, `<run>.diff.png` when it differs from its baseline.

A layout failure fails the command. A changed screenshot does not: look at the PNG and the diff,
decide whether the change is intended and better, then `--bless` (and commit the new baseline) or
fix the code. Keep the baseline set small (`design/ui-baselines/`, one or two views per program).

## 2. Safety

- Sandboxed `%APPDATA%` / `%LOCALAPPDATA%` per run, private endpoints (`--endpoint`), no hotkeys
  (`--no-hotkeys`), `KIM_NO_INPUT=1`; the user's running programs and files are never touched.
- Subak's window runs off-screen in its test mode. Kim, Noeul settings and the Ddalgak editor open
  briefly at (40, 40) on the visible desktop (an off-screen GL window is not guaranteed to
  render): do not run the audit while someone is working at the PC unless they agree.
- Nothing is installed; nothing changes Windows settings.

## 3. Fixtures (what the variables do)

| Variable | Read by | Effect |
|---|---|---|
| `KIM_UI_INSPECT=<file>` | every program | write the geometry report of the current view (off when unset) |
| `KIM_UI_SNAPSHOT=<file>` | every program | screenshot of the same frame (egui: PNG; Subak: BMP, converted) |
| `KIM_UI_EXIT=1` | egui programs | close once report and screenshot are written |
| `KIM_UI_ZOOM=<1.0…2.0>` | every program | the UI zoom instead of `ui.json` / Auto |
| `KIM_UI_SCREEN=<view>` | egui programs | open a view: Kim `launcher`, `settings`, `settings-shortcuts`, `settings-catalog`, `settings-about`; Noeul `general` … `advanced`; Ddalgak `rules`, `new-rule`, `modes`, `profiles`, `playground`, `settings` |
| `KIM_UI_THEME=dark|light`, `KIM_UI_LANG=en|ko` | all `kim-ui` / `kim-design` users | theme and language of the run |
| `KIM_UI_POS=x,y` | egui programs | window position |
| `SUBAK_TEST_SCREEN_ZOOM`, `SUBAK_TEST_DPI` | `subak-gui --test-offscreen`, debug builds | UI zoom and DPI fixtures |

The report is written from the program's own layout: egui `Response` rectangles and painter text
rectangles (`kim_ui::inspect`), Subak's pixel tab layout and GDI-measured text
(`subak::gui::tabs::components`, `Window::geometry`). Do not add components to a report by
computing where they *should* be.

## 4. Rules (`kim_design::audit`)

`token-height`, `token-width` (token × the report's scale, ±1 px), `inside-parent`,
`inside-window`, `hit-target` (≥ 24 × scale), `text-clipped`, `type-scale` (11/12/13/15/20 ×
scale, ±0.6 px; icons and terminal text exempt), `min-readable` (≥ 10 logical px), `centered`
(label centre within 1.5 px of the control's), `uniform-size` (repeated tabs, nav items, list rows,
menu items, tiles), `even-spacing`, `leading-gap`, `overlap` (pointer areas of interactive
siblings of any type), `contrast` (WCAG 4.5:1 text), `palette` (warning), `disabled-distinct`.

To cover a new component: record it in its `kim-ui` widget with `inspect::record` (or in the
Win32 layout's parts), give it a `kind` the rules know (or add an `expect()` entry in
`audit.rs` with its tokens), add a fixture to `xtask/src/ui_audit.rs` if it needs a view of its
own, and add a unit test in `kim-design` for any new rule.

## 5. Reviewing by eye

The numbers cannot judge balance or clutter. For every UI change, open the PNGs of the changed
views at 100 % and 150 % (dark and light) and check: one accent per view, no stray marks or
separators, text hierarchy readable at a glance, nothing cramped against an edge, Korean as tidy
as English. Record anything left as a remaining issue in UI_QUALITY_AUDIT §7.

## 6. Before committing

Run the program's unit and UI tests, `cargo xtask ui-audit --quick` (no `FAIL`), look at the
screenshots, `cargo xtask bump --changed`, `cargo xtask ci`, then rebuild the root `kim.exe`
and the local mirror (AGENTS.md §4.5–4.7). Real-desktop checks follow ADR 0088.
