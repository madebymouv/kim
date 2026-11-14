# ADR 0095: Subak's tabs follow the design tokens; shells for Linux commands

- Status: accepted
- Date: 2026-11-15
- Issue: maintainer request 2026-11-15 (§3 "Subak — Fix Terminal Tab Sizing", §4 "Support Common
  Linux Commands on Windows"); extends Step 17 (the tab bar), Step 20 (settings menu), Step 33
  (detected shells, ADR 0066), ADR 0091 (the bar grows with the screen) and ADR 0092

## Context

Subak's tabs were sized in *terminal cells*: each tab as wide as its name, clamped to 12–28 cells.
Cells are the bar font's width at the renderer's scale, so the tab width moved with the font and
the screen factor in uneven steps, tabs of one window had different widths, a wide window left
most of the bar empty and a narrow one scrolled early. The padding and the active tab's marker
were scaled by the DPI only, not by the screen factor of ADR 0091, so on a large screen the
text grew but the spacing around it did not. Settings were only in the tab bar's right-click
menu, and the shell of new tabs could be chosen only in `config.toml`.

The maintainer also asked for `ls`, `vim`, `cat`, `pwd`, `mkdir`, `mv`, `cp` and `grep`. Subak is
a terminal emulator: those commands belong to the shell a tab runs. Subak already detects the
shells on the PC (Step 33: PowerShell 7, Windows PowerShell, Command Prompt, Git Bash, one profile
per WSL distribution) without running or installing anything.

## Decision

1. **Uniform tabs from the tokens.** The bar computes its tab widths from the design tokens:
   `tab.min-width` 96 and `tab.max-width` 220 logical pixels at the bar's scale (DPI × the
   window's zoom), converted to cells of the bar font (`tabs::Sizing::from_tokens`). All tabs of
   a window get the same width: the bar's width (less `+` and the gear) shared by the tabs,
   within that range — they grow with the window, shrink as tabs are added, and scroll (`‹ ›`,
   the active tab always visible) only below the minimum. A tab always keeps room for its state,
   a few letters of its name (cut with `…`) and its `×`, which stays at the tab's right end.
   The rename field keeps its own width.
2. **One scale for every bar metric**: height (`tabbar.height` 32, compact 24) and at least the
   text's line plus 4 px above and below, the horizontal padding and the active tab's 2 px marker
   all use DPI × zoom, so DPI changes, the screen factor and a new font or theme lay the bar out
   again consistently. The terminal's own text size (`Ctrl+=`) does not change the bar: it is the
   window's chrome, not content (DESIGN_SYSTEM §1).
3. **The gear.** A small gear at the right end of the tab bar opens Subak's settings directly
   (the same menu the tab bar's right-click menu has). A gear in the lower-right corner, the
   design system's default, would cover terminal output; the bar is the terminal's chrome.
4. **Default shell** is the first item of the settings menu: *Windows default (first found)*
   and every configured and detected profile, the current one checked; the choice is
   `terminal.default_profile` in `config.toml` (saved like every menu choice).
5. **Linux commands come from a shell, not from Subak.** SUBAK_GUI §13.3.1 documents which
   detected shell provides which command: Git Bash all eight (GNU tools and `vim` of Git for
   Windows), WSL distributions their own tools, PowerShell aliases with PowerShell's own options
   (`ls` → `Get-ChildItem`, …; no `grep` — `Select-String`; no `vim`), Command Prompt none.
   Nothing is installed and no `PATH` or system setting is changed; no imitation of the commands
   is built into Subak.

## Consequences

- Tabs look alike, use the bar, and scale with the window and the screen; the bar's spacing and
  marker match its text on a large screen.
- `vim` and the other commands work in a Git Bash tab on this PC (checked on the real desktop,
  see the tests below); without Git for Windows or WSL they are not available, and the docs say
  where to get them as the user's own choice.
- Tested: `tabs` unit tests (uniform widths share the bar and stay within the token range at
  100 % and 175 % cells, a wider bar never gives narrower tabs, 30 tabs scroll with the active
  one visible, the gear's span ends the bar, the Step 17 layouts unchanged), the settings GUI
  test (Default shell is listed with exactly the current shell checked; a click on the bar's
  gear opens the settings), the whole Subak suite, and the real desktop (sandbox Host on a
  private endpoint): the gear opened the settings menu, *Default shell → Git Bash* wrote
  `default_profile = "Git Bash"`, `+` started Git Bash, and `pwd`, `mkdir`, `cat`, `cp`, `mv`,
  `ls -la`, `grep -n` ran; `vim` opened full screen, took typed text and `:wq` saved it
  (`which` showed all eight in `/usr/bin`); with six tabs the bar was shared evenly at a
  1576-pixel window, scrolled (`›`) at 884 pixels and grew to the maximum width at 2584 pixels
  with a long name cut by `…`.
