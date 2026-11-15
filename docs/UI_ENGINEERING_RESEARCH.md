# UI engineering research

Status: **current** · 2026-11-15 · feeds [ADR 0099](adr/0099-one-scale-model-measured-ui-audit.md)
and the [UI quality audit](UI_QUALITY_AUDIT.md)

How polished desktop applications and design systems get consistent, attractive interfaces, and
which techniques Kim adopts. Each entry separates **verified** facts (documentation or source
code) from **observed** ones (what the shipped product looks like) and **assumptions**. Nothing
here claims knowledge of a proprietary application's private implementation, and no asset or
layout was copied.

## 1. A spacing and size ramp on a base unit

- **What it achieves:** every gap, padding and control size is one of a few values, so repeated
  structures line up and nothing is "randomly sized".
- **Verified:** Fluent 2 defines a global spacing ramp applied to component and layout spacing
  ([Fluent 2 layout](https://fluent2.microsoft.design/layout)); third-party summaries describe a
  4 px base unit (not confirmed in the official page text). shadcn/ui derives its radius variants
  from one `--radius` variable ([shadcn/ui theming](https://ui.shadcn.com/docs/theming)).
- **Why it improves quality:** alignment errors become impossible to express in code that only
  uses the ramp; reviewers can check a value by name.
- **Kim:** already had `kim_design::space` (2/4/8/12/16/24/32) and `size`; the failure was that
  Subak's chrome and several components bypassed it. **Adopted (kept)**, plus an automatic
  check that the *measured* sizes are token × scale (`audit::token-height/width`).
- **Rust:** constants in `kim-design`, consumed by both the egui and the Win32 adapters.

## 2. A type ramp with one UI family and a separate content font

- **Verified:** Windows 11 uses Segoe UI Variable as the system font and a Fluent type ramp
  ([Typography in Windows 11](https://learn.microsoft.com/windows/apps/design/signature-experiences/typography));
  Fluent 2 web uses Segoe UI ([Fluent 2 typography](https://fluent2.microsoft.design/typography)).
- **Verified (Ghostty):** the terminal font size is its own setting (`font-size`), separate from
  window chrome, and new windows inherit the focused window's size by default
  (`window-inherit-font-size`) ([ghostty(5)](https://man.archlinux.org/man/ghostty.5.en)).
- **Observed:** Windows Terminal draws its tabs in the UI font while the content uses the
  configured monospace font.
- **Why:** chrome in the content font looks like content; a UI family with a fixed ramp keeps
  hierarchy consistent across programs.
- **Kim:** Subak's tab labels were drawn in the terminal's monospace font through the terminal
  renderer. **Adopted:** chrome text uses `font::UI_FAMILY` through a GDI adapter
  (`subak::gui::uitext`), the terminal keeps its own font and size. The audit checks that every UI
  text is on the ramp (11/12/13/15/20 × scale).

## 3. Logical pixels and per-monitor DPI, applied once

- **Verified:** Microsoft recommends per-monitor v2 awareness for desktop apps; with it the whole
  window tree is notified of DPI changes and non-client scaling is automatic
  ([High DPI desktop development](https://learn.microsoft.com/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows),
  [DPI_AWARENESS_CONTEXT](https://learn.microsoft.com/windows/win32/hidpi/dpi-awareness-context)).
- **Why:** sizes defined in logical units and multiplied once by the monitor's scale look the
  same everywhere; a second multiplication (an app zoom computed from *physical* pixels, or a
  zoom applied to some parts but not others) produces exactly the inconsistencies the audit found.
- **Kim:** **Adopted** as `kim_design::scale` — DPI scale × one shared UI zoom; the work area is
  converted to logical pixels before the zoom is chosen; Subak's chrome zoom is capped by its
  text's own enlargement. Terminal text scale is a separate, user-owned factor.

## 4. Tokens as executable rules, adapters per renderer

- **Verified:** shadcn/ui components read pairs of CSS variables (`--background` /
  `--foreground`, `--card`, `--popover`, `--radius`) so a theme is one set of values
  ([shadcn/ui theming](https://ui.shadcn.com/docs/theming)). Fluent publishes design tokens
  consumed by its component libraries ([Fluent 2](https://fluent2.microsoft.design/get-started/design)).
- **Observed:** Discord, Claude Desktop and ChatGPT Desktop look like refined web UIs; whatever
  their stacks, their consistency comes from one component library per product (assumption about
  process, not a claim about their code).
- **Kim:** one token crate (`kim-design`) with two adapters — `kim-ui` (egui) and the Win32
  painters (Subak's `uitext` + `tabs::Metrics`). **Adopted.** No web runtime (KIM philosophy).

## 5. Reusable components own their geometry and states

- **Verified:** egui widgets allocate a rectangle and return a `Response` with it (`rect`), the
  basis for measuring what was laid out; `egui_kittest` drives real egui frames headlessly
  ([egui_kittest](https://docs.rs/egui_kittest)).
- **Why:** if every row, button and field computes its own size, a fix in one place fixes every
  program. Kim's audit found components that did not meet the tokens (26 px text fields) — fixed
  once in `kim-ui`, corrected in Kim, Noeul and Ddalgak at the same time.
- **Kim:** **Adopted**; responsive behaviour moved into the component (a settings row stacks its
  control under its text when the text would get narrower than 160 px).

## 6. Measuring rendered geometry, not intentions

- **Verified:** egui exposes `Response::rect`, the clip rectangle (`Ui::clip_rect`) and painter
  text rectangles; GDI measures text with `DrawText(DT_CALCRECT)` and `GetTextMetrics`.
- **Why:** the 0.0.3 tab bar passed a test that checked a number in a dump; it failed the eye.
  Reading back real rectangles and text boxes makes "looks too small" a numeric failure (label ÷
  terminal text, clipping, overlap, centring).
- **Kim:** **Adopted:** `kim_design::inspect` (report schema), `kim_ui::inspect` (egui collector,
  off unless `KIM_UI_INSPECT`), Subak's `geometry()`; `kim_design::audit` checks them.

## 7. Screenshot regression with explicit tolerances and human review

- **Verified:** egui_kittest snapshots compare rendered images with a per-pixel threshold
  (default 0.6) and an optional failed-pixel count, write `.new.png` and `.diff.png`, and are
  updated only on request (`UPDATE_SNAPSHOTS=true`); its guidelines warn that image tests are slow
  and brittle and should be few ([egui_kittest snapshot](https://docs.rs/egui_kittest)). Playwright's
  `toHaveScreenshot` uses a per-pixel colour threshold and `maxDiffPixelRatio` / `maxDiffPixels`
  ([Playwright SnapshotAssertions](https://playwright.dev/docs/api/class-snapshotassertions)).
- **Kim:** **Adopted, adapted:** a small curated baseline set from the *real executables*
  (`design/ui-baselines/`), a per-pixel threshold (40 levels) and a changed-pixel ratio (0.5 %),
  a diff image for review, and `--bless` only by hand. egui_kittest's `wgpu` snapshot feature
  was **not adopted**: it would add a GPU stack to the test build and still not cover Subak's GDI
  window; the programs' own screenshots cover both backends.

## 8. Native chrome where it matters

- **Verified:** Windows Terminal paints its own title bar following "Custom Window Frame Using
  DWM" and puts WinUI's TabView into it ([microsoft/terminal #1375](https://github.com/microsoft/terminal/issues/1375)).
- **Kim:** **Deferred.** Integrating Subak's tabs into the caption would remove the second
  chrome row, but needs custom hit-testing, snap layouts and accessibility work; recorded as the
  next chrome step (UI_QUALITY_AUDIT §7).

## 9. Interaction states designed once

- **Verified:** Fluent and shadcn/ui define rest, hover, pressed, focus-visible, selected and
  disabled per component (the docs above).
- **Kim:** states are drawn by the `kim-ui` components and Subak's tab layout; the audit records
  `hover`, `active`, `selected`, `disabled` and checks contrast, palette membership and that a
  disabled control does not look enabled (with egui's disabled fade measured, not assumed).

## 10. Technology choice

| Option | Fidelity | Layout | Perf / idle | Windows integration | Testability | Cost | Verdict |
|---|---|---|---|---|---|---|---|
| Improve egui + Win32 (chosen) | good once components own geometry; GDI ClearType for Subak chrome | egui layouts + our row/stack rules | idle 0 % (no timers), small binaries | native windows, IME, DPI v2 | geometry reports + screenshots from both | low | **Adopt** |
| Stronger abstractions over egui | same | better reuse | same | same | same | medium | **Adopt incrementally** (inspect, responsive rows) |
| Replace Subak chrome with Direct2D/DirectWrite | slightly better text | same as GDI pixel layout | same | good | same | medium-high | Not now: GDI already uses the same fonts and ClearType; revisit with the custom caption |
| Another Rust UI toolkit (Slint, Iced, Xilem) | varies | strong | varies | weaker IME/accessibility on Windows in places | new harness | high (rewrite) | **Reject** for now: no evidence egui/Win32 block the goals |
| Electron / web runtime | high | high | heavy, always-on runtime | indirect | mature | — | **Rejected** by the product rules |

**Orca** (OrcaSlicer, a wxWidgets desktop app) was looked at as an example of a dense technical
UI: its consistency comes from a restricted widget set and fixed paddings (observation only).
