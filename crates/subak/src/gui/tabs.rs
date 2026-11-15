//! The tab bar (Step 17, DESIGN_SYSTEM §3.5, docs/SUBAK_GUI.md §2): one tab per Terminal of the
//! Host that this window shows, in order, with its state, its name, a close `×` that **hides**
//! the tab (a detach; the Terminal keeps running, ADR 0020 §7) and a `+` that creates a Terminal.
//!
//! ADR 0099: the bar is laid out in device pixels from the design tokens at the bar's scale and
//! drawn in the UI font (Segoe UI, Korean through Windows' font linking) and Windows' icon font,
//! not as a row of terminal cells: the cell grid forced the chrome into the monospace terminal
//! font and cell-quantised sizes. This module is pure: it measures text through [`Measure`],
//! scrolls the tabs when they do not fit (the active tab always stays visible, `‹`/`›` scroll),
//! and returns drawing operations, clickable rectangles and the parts of a geometry report, so
//! the layout is unit-tested without a window. State is never colour-only (DESIGN_SYSTEM §3.8):
//! every non-normal state has its own shape; the active tab is semibold with its own fill and a
//! lilac marker.

use super::screens::tokens::{ACCENT, DANGER, SUCCESS, TEXT_MUTED, WARNING};
#[cfg(test)]
use super::screens::tokens::{
    BG_BASE, BG_HOVER, BG_OVERLAY, BG_RAISED, BORDER_SUBTLE, TEXT_PRIMARY, TEXT_SECONDARY,
};
use subak_term::render::Rgb;

/// A tab's Terminal state, as the bar shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    Starting,
    Stopping,
    /// Kept by the Host without a process (`stopped`).
    Stopped,
    /// Kept after its start failed.
    Failed,
    /// No longer in the Host (exited or removed); only the shown tab stays, with its last screen.
    Gone,
    /// Not known while the window is disconnected from the Host.
    Unknown,
}

impl Status {
    /// The bar's status from a Host state (`terminal.list`, events).
    pub fn from_state(state: &str) -> Self {
        match state {
            "running" => Self::Running,
            "starting" => Self::Starting,
            "stopping" => Self::Stopping,
            "stopped" => Self::Stopped,
            "failed" => Self::Failed,
            "removed" | "exited" | "gone" => Self::Gone,
            _ => Self::Unknown,
        }
    }

    /// The shape (DESIGN_SYSTEM §3.1: shape and colour, never colour alone).
    pub fn glyph(self) -> char {
        match self {
            Self::Running => '●',
            Self::Starting | Self::Stopping => '◐',
            Self::Stopped => '■',
            Self::Failed => '▲',
            Self::Gone | Self::Unknown => '○',
        }
    }

    /// The state in words (test reports, the context menu).
    pub fn word(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Starting => "starting",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Gone => "gone",
            Self::Unknown => "unknown",
        }
    }

    /// The state's colour (always beside the shape).
    pub fn color(self) -> Rgb {
        match self {
            Self::Running => SUCCESS,
            Self::Starting | Self::Stopping => WARNING,
            Self::Failed => DANGER,
            Self::Stopped | Self::Gone | Self::Unknown => TEXT_MUTED,
        }
    }
}

/// One tab to lay out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item<'a> {
    pub name: &'a str,
    pub status: Status,
    /// Step 28: the Terminal's Claude Code state (`None`: no mark).
    pub claude: Claude,
    /// Step 44: the mark's current motion frame (`None`: the state's resting glyph).
    pub frame: Option<crate::gui::claude_motion::Frame>,
    /// #98 T3: output or a bell since the tab was last shown.
    pub alert: Option<crate::gui::model::Alert>,
    /// #98 T3: the progress the Terminal's program reports.
    pub progress: Option<subak_term::screen::Progress>,
    /// #98 T1 (`CORE-14`): pinned: a `◆` mark and no hide button.
    pub pinned: bool,
    /// #98 T1 (`CORE-10`, `tabs.close_buttons`): the tab has its `×` (hide) button.
    pub closable: bool,
    /// #98 T1 (`CORE-24`): the tab's colour (a bar at its start; the name stays the signal).
    pub color: Option<Rgb>,
}

/// #98 T1 (`CORE-24`): the tab colours, by name (mid tones that read on the dark and the light
/// theme).
pub const TAB_COLORS: [(&str, Rgb); 8] = [
    ("red", Rgb(0xE0, 0x4F, 0x5F)),
    ("orange", Rgb(0xE8, 0x8A, 0x3C)),
    ("yellow", Rgb(0xD9, 0xB4, 0x2C)),
    ("green", Rgb(0x4C, 0xB0, 0x6C)),
    ("teal", Rgb(0x2E, 0xA8, 0xA8)),
    ("blue", Rgb(0x4A, 0x86, 0xE0)),
    ("purple", Rgb(0x9A, 0x6C, 0xE0)),
    ("pink", Rgb(0xE0, 0x6C, 0xB4)),
];

/// The colour of a name of [`TAB_COLORS`].
pub fn color_named(name: &str) -> Option<Rgb> {
    TAB_COLORS.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

/// #98 T3 (`CORE-78`): a tab's progress: `42%` (accent), `42%!` (failed, danger), `‖42%`
/// (paused, warning) or `…` (busy without a value); text and shape, never colour alone.
pub fn progress_mark(p: Option<subak_term::screen::Progress>) -> Option<(String, Rgb)> {
    use subak_term::screen::ProgressState as S;
    let p = p?;
    Some(match p.state {
        S::Normal => (format!("{}%", p.value), ACCENT),
        S::Error => (format!("{}%!", p.value), DANGER),
        S::Paused => (format!("\u{2016}{}%", p.value), WARNING),
        S::Indeterminate => ("\u{2026}".to_string(), ACCENT),
    })
}

/// #98 T1 (`CORE-23`, `tabs.title`): a tab's text from the format: `{name}`, `{title}` (what
/// the program set), `{program}`, `{cwd}` (the folder's name). An empty result falls back to
/// the name, so a tab is never blank.
pub fn label(
    format: &str,
    name: &str,
    title: Option<&str>,
    program: &str,
    cwd: Option<&str>,
) -> String {
    let folder = cwd
        .map(|c| c.trim_end_matches(['\\', '/']))
        .and_then(|c| c.rsplit(['\\', '/']).next())
        .unwrap_or("");
    let text = format
        .replace("{name}", name)
        .replace("{title}", title.unwrap_or(""))
        .replace("{program}", program)
        .replace("{cwd}", folder);
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    let text = text.trim();
    if text.is_empty() {
        name.to_string()
    } else {
        text.to_string()
    }
}

/// #98 T3: the mark of a tab with unseen output (`•`) or a bell (`♪`), a shape each.
pub fn alert_mark(a: Option<crate::gui::model::Alert>) -> Option<(char, Rgb, &'static str)> {
    use crate::gui::model::Alert;
    Some(match a? {
        Alert::Output => ('•', ACCENT, "new output"),
        Alert::Bell => ('♪', WARNING, "bell"),
    })
}

pub use crate::gui::float::art::Claude;

/// Step 28: the tab's Claude mark — a shape and a colour per state (never colour alone,
/// DESIGN_SYSTEM §3.1) — and its words for the title, menus and tests.
pub fn claude_mark(c: Claude) -> Option<(char, Rgb, &'static str)> {
    Some(match c {
        Claude::NeedInput => ('!', WARNING, "Claude needs input"),
        Claude::Error => ('✕', DANGER, "Claude error"),
        Claude::Done => ('✓', SUCCESS, "Claude done"),
        Claude::InProgress => ('◔', ACCENT, "Claude working"),
        Claude::Unknown => ('?', TEXT_MUTED, "Claude state unknown"),
        Claude::Idle => ('◦', TEXT_MUTED, "Claude idle"),
        Claude::None => return None,
    })
}

/// What a click on the bar means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// The tab: select it (double click: rename; middle click: hide; right click: its menu).
    Tab(usize),
    /// The tab's `×`: hide the tab (a detach; the Terminal keeps running).
    Close(usize),
    /// `+`: a new Terminal.
    New,
    /// `‹` / `›`: scroll the tabs.
    Left,
    Right,
    /// ADR 0095: the gear at the right end of the bar: Subak's settings.
    Settings,
}

/// The rename field shown in place of the active tab's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit<'a> {
    pub text: &'a str,
    /// The caret, in characters from the start of `text`.
    pub cursor: usize,
}

/// A rectangle in device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Px {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Px {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Px {
        Px { x, y, w, h }
    }
    pub fn right(&self) -> i32 {
        self.x + self.w as i32
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h as i32
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    pub fn inspect(&self) -> kim_design::inspect::Rect {
        kim_design::inspect::Rect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }
}

/// ADR 0099: the bar's geometry at one scale, every value a design token × the scale, rounded
/// once. The label is a UI font line (never terminal cells), so sizes follow the type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// The bar's height; the tabs start `tab_top` below its top edge and reach its bottom.
    pub height: u32,
    pub tab_top: u32,
    /// Space before the first and after the last control.
    pub bar_pad: u32,
    /// A tab's inner horizontal padding, and the gap between its marks and its name.
    pub pad_x: u32,
    pub gap: u32,
    /// Uniform tabs are this narrow at least and this wide at most.
    pub min_w: u32,
    pub max_w: u32,
    /// The close button's square hit area and its glyph's size.
    pub close: u32,
    pub close_icon: u32,
    /// `+`, the gear and the scroll arrows: square buttons and their glyph size.
    pub button: u32,
    pub icon: u32,
    /// The label's font size (UI body).
    pub font: u32,
    /// Top corner radius of a filled tab, the active marker's thickness, the separator inset.
    pub radius: u32,
    pub marker: u32,
    pub sep_inset: u32,
    /// The colour bar of a coloured tab.
    pub color_bar: u32,
    /// The scale the values were made for.
    pub scale: f32,
}

impl Metrics {
    /// The tokens at `scale` (DPI × chrome zoom); `compact` is the lower bar (#98 T1).
    pub fn new(scale: f32, compact: bool) -> Metrics {
        use kim_design::{font, radius, size, space};
        let p = |v: f32| kim_design::px(v, scale).max(0) as u32;
        let height = if compact {
            size::TABBAR_HEIGHT_COMPACT
        } else {
            size::TABBAR_HEIGHT
        };
        Metrics {
            height: p(height),
            tab_top: p(size::TAB_INSET_TOP),
            bar_pad: p(space::S),
            pad_x: p(space::M),
            gap: p(6.0),
            min_w: p(size::TAB_MIN_WIDTH),
            max_w: p(size::TAB_MAX_WIDTH),
            close: p(size::HIT_MIN),
            close_icon: p(size::TAB_CLOSE_ICON),
            button: p(if compact {
                size::HIT_MIN
            } else {
                size::CONTROL
            }),
            icon: p(size::ICON_S),
            font: p(font::BODY),
            radius: p(radius::M),
            marker: p(2.0),
            sep_inset: p(space::S),
            color_bar: p(3.0),
            scale,
        }
    }

    /// The tabs' height.
    pub fn tab_h(&self) -> u32 {
        self.height.saturating_sub(self.tab_top)
    }
}

/// Text measuring: the real font's advance width of `text` at `px` pixels.
pub trait Measure {
    fn width(&mut self, text: &str, px: u32, semibold: bool) -> u32;
    /// The font's line height (ascent + descent) at `px`.
    fn line(&mut self, px: u32) -> u32;
}

/// A measure for tests: CJK and other wide characters twice as wide.
#[cfg(test)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ApproxMeasure;

#[cfg(test)]
impl Measure for ApproxMeasure {
    fn width(&mut self, text: &str, px: u32, semibold: bool) -> u32 {
        let units: u32 = text
            .chars()
            .map(|c| u32::from(subak_term::screen::char_width(c)).max(1))
            .sum();
        let per = px as f32 * if semibold { 0.58 } else { 0.55 };
        (units as f32 * per).round() as u32
    }
    fn line(&mut self, px: u32) -> u32 {
        (px as f32 * 1.33).round() as u32
    }
}

/// Which font a text is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// The UI family (kim_design::font::UI_FAMILY, Korean through Windows' font linking).
    Ui,
    /// Windows' icon font (Segoe MDL2 Assets / Segoe Fluent Icons): ×, +, gear, chevrons.
    Icon,
    /// Symbols for the state marks (Segoe UI Symbol).
    Symbol,
}

/// One drawing operation of the bar, in device pixels.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// A filled rectangle; `radius` rounds its top corners.
    Fill { rect: Px, color: Rgb, radius: u32 },
    /// Text in `rect`, vertically centred (horizontally too with `center`), cut with an
    /// ellipsis when it does not fit.
    Text {
        rect: Px,
        text: String,
        px: u32,
        semibold: bool,
        face: Face,
        color: Rgb,
        center: bool,
    },
}

/// The colours the bar is drawn with (the window maps the theme and High Contrast onto them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub bar: Rgb,
    pub active: Rgb,
    pub hover: Rgb,
    pub field: Rgb,
    pub border: Rgb,
    pub marker: Rgb,
    pub text: Rgb,
    pub text_active: Rgb,
    pub text_muted: Rgb,
}

#[cfg(test)]
impl Colors {
    /// `kim-dark`'s roles (the window passes its theme's).
    pub const DARK: Colors = Colors {
        bar: BG_RAISED,
        active: BG_BASE,
        hover: BG_HOVER,
        field: BG_OVERLAY,
        border: BORDER_SUBTLE,
        marker: ACCENT,
        text: TEXT_SECONDARY,
        text_active: TEXT_PRIMARY,
        text_muted: TEXT_MUTED,
    };
}

/// One laid-out part, for the geometry report (ADR 0099).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub kind: &'static str,
    pub parent: Option<String>,
    pub rect: Px,
    pub hit: Option<Px>,
    /// The text, its measured box, font size, line height, whether it was cut, its face.
    pub text: Option<(String, Px, u32, u32, bool, Face)>,
    pub state: Vec<&'static str>,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    pub interactive: bool,
}

/// A laid-out bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub ops: Vec<Op>,
    /// Clickable rectangles, tested in order (a close button before its tab).
    pub spans: Vec<(Px, Hit)>,
    /// The active tab's horizontal extent (the bar's bottom border is open under it).
    pub active: Option<(i32, i32)>,
    /// The rename caret's x and its height, while renaming.
    pub caret: Option<(i32, u32)>,
    /// The first visible tab.
    pub first: usize,
    /// The bar as text: what each visible tab shows, the arrows, `+` and the gear (tests,
    /// accessibility).
    pub text: String,
    pub parts: Vec<Part>,
    /// Not every tab fits: the arrows scroll.
    pub overflow: bool,
}

impl Bar {
    /// What a click at (`x`, `y`) (bar pixels) means.
    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        self.spans
            .iter()
            .find(|(r, _)| r.contains(x, y))
            .map(|s| s.1)
    }

    /// The bar's rectangle of `hit` (the first one).
    pub fn rect_of(&self, hit: Hit) -> Option<Px> {
        self.spans.iter().find(|s| s.1 == hit).map(|s| s.0)
    }

    /// An empty bar (before the first layout).
    pub fn empty() -> Bar {
        Bar {
            ops: Vec::new(),
            spans: Vec::new(),
            active: None,
            caret: None,
            first: 0,
            text: String::new(),
            parts: Vec::new(),
            overflow: false,
        }
    }
}

/// The marks of a tab before its name, in order, with their colours. A running Terminal is the
/// normal state and has no mark; every other state keeps its own shape.
fn marks(it: &Item<'_>) -> Vec<(String, Rgb)> {
    let mut out = Vec::new();
    if it.status != Status::Running {
        out.push((it.status.glyph().to_string(), it.status.color()));
    }
    if let Some((g, c, _)) = claude_mark(it.claude) {
        let g = it.frame.map_or(g, |f| f.glyph);
        out.push((g.to_string(), c));
    }
    if it.pinned {
        out.push(("\u{25c6}".to_string(), ACCENT));
    }
    if let Some((g, c, _)) = alert_mark(it.alert) {
        out.push((g.to_string(), c));
    }
    if let Some((t, c)) = progress_mark(it.progress) {
        out.push((t, c));
    }
    out
}

/// The icon font's glyphs (Segoe MDL2 Assets and Segoe Fluent Icons share the code points).
pub const ICON_CLOSE: &str = "\u{E8BB}";
pub const ICON_ADD: &str = "\u{E710}";
pub const ICON_SETTINGS: &str = "\u{E713}";
pub const ICON_LEFT: &str = "\u{E76B}";
pub const ICON_RIGHT: &str = "\u{E76C}";

/// `text` cut with `…` to at most `max` pixels (whole characters).
fn ellipsize(m: &mut dyn Measure, text: &str, px: u32, semibold: bool, max: u32) -> (String, bool) {
    if m.width(text, px, semibold) <= max {
        return (text.to_string(), false);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut n = chars.len();
    while n > 0 {
        let s: String = chars[..n].iter().collect::<String>() + "…";
        if m.width(&s, px, semibold) <= max {
            return (s, true);
        }
        n -= 1;
    }
    (String::new(), true)
}

fn sub(chars: &[char]) -> String {
    chars.iter().collect()
}

/// Lays out `items` on a bar `width` device pixels wide with metrics `mt` (ADR 0095, ADR 0099):
/// every tab the same width, sharing the bar within `min_w`–`max_w` before the tabs scroll
/// (the active tab always visible); the close button at each tab's right end; `+` after the
/// last tab; the gear at the bar's right end. `edit` is the rename field of the active tab,
/// `first` the first visible tab of the last layout, `hover` the tab under the pointer (its
/// fill only; nothing moves).
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::needless_range_loop
)]
pub fn layout(
    items: &[Item<'_>],
    active: Option<usize>,
    edit: Option<&Edit<'_>>,
    width: u32,
    first: usize,
    hover: Option<usize>,
    mt: &Metrics,
    colors: &Colors,
    m: &mut dyn Measure,
) -> Bar {
    let width = width as i32;
    let (h, top, th) = (mt.height, mt.tab_top as i32, mt.tab_h());
    let button = mt.button as i32;
    let gap = mt.gap as i32;
    let bcy = top + th as i32 / 2;
    let by = bcy - button / 2;
    let gear_x = width - mt.bar_pad as i32 - button;
    let left = mt.bar_pad as i32;
    // The room for the tabs (and the arrows) and `+`.
    let room = (gear_x - gap - left).max(0);
    let n = items.len() as i32;
    let plus_w = gap + button;
    let share = if n == 0 {
        mt.max_w as i32
    } else {
        ((room - plus_w) / n).clamp(mt.min_w as i32, mt.max_w as i32)
    };
    let overflow = n * share + plus_w > room;
    let mut first = if overflow {
        first.min(items.len().saturating_sub(1))
    } else {
        0
    };
    let (tabs_x0, visible_n) = if overflow {
        let avail = room - plus_w - 2 * button;
        let fit = (avail / share).max(1) as usize;
        if let Some(a) = active {
            if a < first {
                first = a;
            } else if a >= first + fit {
                first = a + 1 - fit;
            }
        }
        first = first.min(items.len().saturating_sub(fit));
        // The left arrow only when tabs are hidden on the left (no empty gap otherwise).
        let x0 = if first > 0 { left + button } else { left };
        (x0, fit.min(items.len() - first))
    } else {
        (left, items.len())
    };
    let last = first + visible_n;
    let mut ops = vec![Op::Fill {
        rect: Px::new(0, 0, width.max(0) as u32, h),
        color: colors.bar,
        radius: 0,
    }];
    let mut spans: Vec<(Px, Hit)> = Vec::new();
    let bar_id = "subak.tabbar".to_string();
    let mut parts = vec![Part {
        id: bar_id.clone(),
        kind: "tabbar",
        parent: None,
        rect: Px::new(0, 0, width.max(0) as u32, h),
        hit: None,
        text: None,
        state: if overflow {
            vec!["overflow"]
        } else {
            Vec::new()
        },
        fg: None,
        bg: Some(colors.bar),
        interactive: false,
    }];
    let mut text = String::new();
    let mut active_x = None;
    let mut caret = None;
    let line = m.line(mt.font);
    let mut icon_button = |ops: &mut Vec<Op>,
                           spans: &mut Vec<(Px, Hit)>,
                           x: i32,
                           glyph: &str,
                           hit: Hit,
                           id: &str,
                           kind: &'static str| {
        let r = Px::new(x, by, mt.button, mt.button);
        ops.push(Op::Text {
            rect: r,
            text: glyph.to_string(),
            px: mt.icon,
            semibold: false,
            face: Face::Icon,
            color: colors.text,
            center: true,
        });
        spans.push((r, hit));
        let g = Px::new(
            x + (button - mt.icon as i32) / 2,
            bcy - mt.icon as i32 / 2,
            mt.icon,
            mt.icon,
        );
        parts.push(Part {
            id: id.to_string(),
            kind,
            parent: Some(bar_id.clone()),
            rect: r,
            hit: Some(r),
            text: Some((glyph.to_string(), g, mt.icon, mt.icon, false, Face::Icon)),
            state: Vec::new(),
            fg: Some(colors.text),
            bg: Some(colors.bar),
            interactive: true,
        });
    };
    if overflow && first > 0 {
        icon_button(
            &mut ops,
            &mut spans,
            left,
            ICON_LEFT,
            Hit::Left,
            "subak.tabbar.left",
            "icon-button",
        );
        text.push_str("‹ ");
    }
    let mut tab_parts = Vec::new();
    let mut x = tabs_x0;
    let mut prev_filled = true;
    for i in first..last {
        let it = &items[i];
        let is_active = Some(i) == active;
        let hovered = !is_active && Some(i) == hover;
        let tab = Px::new(x, top, share as u32, th);
        let fill = if is_active {
            Some(colors.active)
        } else if hovered {
            Some(colors.hover)
        } else {
            None
        };
        if let Some(c) = fill {
            ops.push(Op::Fill {
                rect: tab,
                color: c,
                radius: mt.radius,
            });
        } else if !prev_filled {
            // A hairline between two plain tabs.
            ops.push(Op::Fill {
                rect: Px::new(
                    x,
                    top + mt.sep_inset as i32,
                    1,
                    th.saturating_sub(2 * mt.sep_inset),
                ),
                color: colors.border,
                radius: 0,
            });
        }
        prev_filled = fill.is_some();
        if is_active {
            ops.push(Op::Fill {
                rect: Px::new(
                    x + mt.radius as i32,
                    top,
                    (share as u32).saturating_sub(2 * mt.radius),
                    mt.marker,
                ),
                color: colors.marker,
                radius: 0,
            });
            active_x = Some((x, x + share));
        }
        if let Some(c) = it.color {
            ops.push(Op::Fill {
                rect: Px::new(
                    x + gap / 2,
                    top + mt.sep_inset as i32,
                    mt.color_bar,
                    th.saturating_sub(2 * mt.sep_inset),
                ),
                color: c,
                radius: 0,
            });
            text.push('\u{258e}');
        }
        let closable = it.closable && !it.pinned;
        let close_r = Px::new(
            x + share - gap / 2 - mt.close as i32,
            bcy - mt.close as i32 / 2,
            mt.close,
            mt.close,
        );
        let name_end = if closable {
            close_r.x
        } else {
            x + share - mt.pad_x as i32
        };
        let mut cx = x + mt.pad_x as i32;
        for (g, c) in marks(it) {
            let w = m.width(&g, mt.font, false).max(1);
            ops.push(Op::Text {
                rect: Px::new(cx, top, w, th),
                text: g.clone(),
                px: mt.font,
                semibold: false,
                face: Face::Symbol,
                color: c,
                center: false,
            });
            text.push_str(&g);
            text.push(' ');
            cx += w as i32 + gap;
        }
        let fg = if is_active {
            colors.text_active
        } else {
            colors.text
        };
        let bg = fill.unwrap_or(colors.bar);
        let room = (name_end - cx - gap / 2).max(0) as u32;
        let editing = edit.filter(|_| is_active);
        let (label, cut, tbox) = match editing {
            Some(e) => {
                // The rename field: the text around the caret, within the field.
                let chars: Vec<char> = e.text.chars().collect();
                let cursor = e.cursor.min(chars.len());
                let mut start = 0;
                while start < cursor
                    && m.width(&sub(&chars[start..cursor]), mt.font, false) + 2 > room
                {
                    start += 1;
                }
                let mut end = chars.len();
                while end > cursor && m.width(&sub(&chars[start..end]), mt.font, false) > room {
                    end -= 1;
                }
                let shown = sub(&chars[start..end]);
                let inset = kim_design::px(4.0, mt.scale).max(1);
                let field_h = line + 2 * inset as u32;
                ops.push(Op::Fill {
                    rect: Px::new(
                        cx - inset,
                        bcy - field_h as i32 / 2,
                        room + 2 * inset as u32,
                        field_h,
                    ),
                    color: colors.field,
                    radius: 0,
                });
                let at = m.width(&sub(&chars[start..cursor]), mt.font, false) as i32;
                caret = Some((cx + at, line));
                let w = m.width(&shown, mt.font, false);
                (shown, false, Px::new(cx, bcy - line as i32 / 2, w, line))
            }
            None => {
                let (s, cut) = ellipsize(m, it.name, mt.font, is_active, room);
                let w = m.width(&s, mt.font, is_active);
                (s, cut, Px::new(cx, bcy - line as i32 / 2, w, line))
            }
        };
        ops.push(Op::Text {
            rect: Px::new(cx, top, room, th),
            text: label.clone(),
            px: mt.font,
            semibold: is_active,
            face: Face::Ui,
            color: fg,
            center: false,
        });
        if let Some((cx, _)) = caret.filter(|_| editing.is_some()) {
            ops.push(Op::Fill {
                rect: Px::new(
                    cx,
                    bcy - line as i32 / 2,
                    kim_design::px(1.0, mt.scale).max(1) as u32,
                    line,
                ),
                color: colors.text_active,
                radius: 0,
            });
        }
        text.push_str(&label);
        let tab_id = format!("subak.tabbar.tab[{i}]");
        let mut state = Vec::new();
        if is_active {
            state.push("active");
        }
        if hovered {
            state.push("hover");
        }
        if overflow {
            state.push("overflow");
        }
        if editing.is_some() {
            state.push("editing");
        }
        tab_parts.push(Part {
            id: tab_id.clone(),
            kind: "tab",
            parent: Some(bar_id.clone()),
            rect: tab,
            hit: Some(tab),
            text: Some((label, tbox, mt.font, line, cut, Face::Ui)),
            state,
            fg: Some(fg),
            bg: Some(bg),
            interactive: true,
        });
        if closable {
            let dim = if is_active || hovered {
                colors.text
            } else {
                colors.text_muted
            };
            ops.push(Op::Text {
                rect: close_r,
                text: ICON_CLOSE.to_string(),
                px: mt.close_icon,
                semibold: false,
                face: Face::Icon,
                color: dim,
                center: true,
            });
            spans.push((close_r, Hit::Close(i)));
            let g = Px::new(
                close_r.x + (mt.close - mt.close_icon) as i32 / 2,
                bcy - mt.close_icon as i32 / 2,
                mt.close_icon,
                mt.close_icon,
            );
            tab_parts.push(Part {
                id: format!("{tab_id}.close"),
                kind: "tab-close",
                parent: Some(tab_id.clone()),
                rect: close_r,
                hit: Some(close_r),
                text: Some((
                    ICON_CLOSE.to_string(),
                    g,
                    mt.close_icon,
                    mt.close_icon,
                    false,
                    Face::Icon,
                )),
                state: Vec::new(),
                fg: Some(dim),
                bg: Some(bg),
                interactive: true,
            });
            text.push_str(" ×");
        }
        spans.push((tab, Hit::Tab(i)));
        text.push_str("  ");
        x += share;
    }
    if overflow && last < items.len() {
        icon_button(
            &mut ops,
            &mut spans,
            x,
            ICON_RIGHT,
            Hit::Right,
            "subak.tabbar.right",
            "icon-button",
        );
        x += button;
        text.push_str("› ");
    }
    if x > tabs_x0 {
        x += gap;
    }
    if x + button <= gear_x {
        icon_button(
            &mut ops,
            &mut spans,
            x,
            ICON_ADD,
            Hit::New,
            "subak.tabbar.new",
            "tab-new",
        );
        text.push_str("+ ");
    }
    icon_button(
        &mut ops,
        &mut spans,
        gear_x,
        ICON_SETTINGS,
        Hit::Settings,
        "subak.tabbar.settings",
        "icon-button",
    );
    text.push('\u{2699}');
    parts.extend(tab_parts);
    // The bar's bottom border, open under the active tab.
    let y = h as i32 - 1;
    let line_at = |a: i32, b: i32| Op::Fill {
        rect: Px::new(a, y, (b - a).max(0) as u32, 1),
        color: colors.border,
        radius: 0,
    };
    match active_x {
        Some((a, b)) => {
            ops.push(line_at(0, a));
            ops.push(line_at(b, width));
        }
        None => ops.push(line_at(0, width)),
    }
    Bar {
        ops,
        spans,
        active: active_x,
        caret,
        first,
        text: text.trim_end().to_string(),
        parts,
        overflow,
    }
}

/// ADR 0099: the bar's parts as a geometry report's components (window client pixels).
pub fn components(bar: &Bar) -> Vec<kim_design::inspect::Component> {
    use kim_design::inspect::{Component, Text};
    let rgb = |c: Rgb| kim_design::Rgb {
        r: c.0,
        g: c.1,
        b: c.2,
    };
    bar.parts
        .iter()
        .map(|p| {
            let mut c = Component::new(p.id.clone(), p.kind, p.rect.inspect());
            c.parent.clone_from(&p.parent);
            c.hit = p.hit.map(|h| h.inspect());
            c.interactive = p.interactive;
            c.state = p.state.iter().map(|s| (*s).to_string()).collect();
            c.fg = p.fg.map(rgb);
            c.bg = p.bg.map(rgb);
            c.text = p.text.as_ref().map(|(t, r, px, line, cut, face)| Text {
                content: t.clone(),
                bounds: r.inspect(),
                font_px: *px as f32,
                line_px: *line as f32,
                truncated: *cut,
                role: if *face == Face::Icon { "icon" } else { "ui" }.to_string(),
            });
            c
        })
        .collect()
}

/// A geometry report of the bar alone (unit tests).
#[cfg(test)]
pub fn report(bar: &Bar, width: u32, height: u32, scale: f32) -> kim_design::inspect::Report {
    kim_design::inspect::Report {
        application: "subak".into(),
        screen: "tabbar".into(),
        fixture: format!("scale={scale}"),
        theme: "kim-dark".into(),
        window: (width as f32, height as f32),
        dpi_scale: 1.0,
        ui_zoom: scale,
        scale,
        components: components(bar),
        ..kim_design::inspect::Report::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items<'a>(names: &[&'a str]) -> Vec<Item<'a>> {
        names
            .iter()
            .map(|n| Item {
                name: n,
                status: Status::Running,
                claude: Claude::None,
                frame: None,
                alert: None,
                progress: None,
                pinned: false,
                closable: true,
                color: None,
            })
            .collect()
    }

    fn lay(
        list: &[Item<'_>],
        active: Option<usize>,
        edit: Option<&Edit<'_>>,
        width: u32,
        hover: Option<usize>,
        mt: &Metrics,
    ) -> Bar {
        layout(
            list,
            active,
            edit,
            width,
            0,
            hover,
            mt,
            &Colors::DARK,
            &mut ApproxMeasure,
        )
    }

    fn bar(list: &[Item<'_>], active: Option<usize>, width: u32, scale: f32) -> Bar {
        lay(list, active, None, width, None, &Metrics::new(scale, false))
    }

    fn tab_rects(b: &Bar) -> Vec<Px> {
        b.spans
            .iter()
            .filter(|s| matches!(s.1, Hit::Tab(_)))
            .map(|s| s.0)
            .collect()
    }

    fn index(id: &str) -> usize {
        id[id.find('[').unwrap() + 1..id.find(']').unwrap()]
            .parse()
            .unwrap()
    }

    /// ADR 0099: every metric is a token × the scale, rounded once, at 100–200 %.
    #[test]
    fn metrics_are_the_tokens_at_the_scale() {
        use kim_design::size;
        for s in [1.0f32, 1.25, 1.5, 2.0] {
            let m = Metrics::new(s, false);
            let t = |v: f32| (v * s).round() as u32;
            assert_eq!(m.height, t(size::TABBAR_HEIGHT));
            assert_eq!(m.tab_h(), t(size::TABBAR_HEIGHT) - t(size::TAB_INSET_TOP));
            assert_eq!(m.font, t(kim_design::font::BODY));
            assert_eq!((m.min_w, m.max_w), (t(96.0), t(220.0)));
            assert!(m.close >= t(size::HIT_MIN), "close hit area");
            assert!(m.tab_h() >= m.close, "the close button fits the tab");
            let c = Metrics::new(s, true);
            assert_eq!(c.height, t(size::TABBAR_HEIGHT_COMPACT));
            assert!(c.tab_h() >= c.close, "compact still fits the close button");
        }
    }

    /// ADR 0095/0099: tabs are the same width, share the bar within 96–220 px × scale, grow with
    /// the window up to the maximum, and the gear ends the bar.
    #[test]
    fn tabs_share_the_bar_and_the_gear_ends_it() {
        for s in [1.0f32, 1.25, 1.5, 2.0] {
            let mt = Metrics::new(s, false);
            let list = items(&["claude", "server", "a much longer name of a terminal"]);
            let mut prev = 0;
            for w in [500u32, 900, 1400, 3000] {
                let w = (w as f32 * s) as u32;
                let b = bar(&list, Some(0), w, s);
                let tabs = tab_rects(&b);
                assert_eq!(tabs.len(), 3);
                assert!(tabs.iter().all(|t| t.w == tabs[0].w), "same width {tabs:?}");
                assert!(
                    (mt.min_w..=mt.max_w).contains(&tabs[0].w),
                    "{tabs:?} at {w}"
                );
                assert!(tabs[0].w >= prev, "wider window, wider tabs");
                prev = tabs[0].w;
                let gear = b.rect_of(Hit::Settings).unwrap();
                assert_eq!(
                    gear.right(),
                    w as i32 - mt.bar_pad as i32,
                    "the gear ends the bar"
                );
                let plus = b.rect_of(Hit::New).unwrap();
                assert!(plus.x >= tabs[2].right(), "+ follows the last tab");
                assert!(b.text.ends_with('\u{2699}'), "{}", b.text);
            }
        }
    }

    #[test]
    fn labels_are_centred_and_the_close_button_sits_at_the_right_end() {
        let list = items(&["main", "로그", "a-much-longer-terminal-name-than-fits"]);
        for s in [1.0f32, 1.5, 2.0] {
            let b = bar(&list, Some(0), (800.0 * s) as u32, s);
            for p in b.parts.iter().filter(|p| p.kind == "tab") {
                let (_, tb, ..) = p.text.as_ref().unwrap();
                let tc = tb.y * 2 + tb.h as i32;
                let pc = p.rect.y * 2 + p.rect.h as i32;
                assert!((tc - pc).abs() <= 2, "centred {p:?}");
                assert!(tb.right() <= p.rect.right(), "label inside its tab {p:?}");
            }
            for p in b.parts.iter().filter(|p| p.kind == "tab-close") {
                let tab = b
                    .parts
                    .iter()
                    .find(|t| Some(&t.id) == p.parent.as_ref())
                    .unwrap();
                assert!(
                    tab.rect.right() - p.rect.right() <= (8.0 * s) as i32,
                    "{p:?}"
                );
                assert!(p.rect.y >= tab.rect.y && p.rect.bottom() <= tab.rect.bottom());
            }
            let long = b.parts.iter().find(|p| p.id.ends_with("[2]")).unwrap();
            let t = long.text.as_ref().unwrap();
            assert!(t.4 && t.0.ends_with('…'), "the long name is cut: {t:?}");
        }
    }

    #[test]
    fn hit_testing_matches_the_painted_parts() {
        let list = items(&["one", "two"]);
        let b = bar(&list, Some(1), 900, 1.0);
        for p in &b.parts {
            let Some(h) = p.hit else { continue };
            let got = b.hit(h.x + h.w as i32 / 2, h.y + h.h as i32 / 2);
            let want = match p.kind {
                "tab" => Hit::Tab(index(&p.id)),
                "tab-close" => Hit::Close(index(&p.id)),
                "tab-new" => Hit::New,
                _ => Hit::Settings,
            };
            assert_eq!(got, Some(want), "{p:?}");
        }
        // Above a tab's close button but inside the tab: the tab.
        let c = b.rect_of(Hit::Close(0)).unwrap();
        let t = b.rect_of(Hit::Tab(0)).unwrap();
        assert_eq!(b.hit(c.x + 2, t.y + 1), Some(Hit::Tab(0)));
        assert_eq!(b.hit(-1, 5), None);
    }

    #[test]
    fn many_tabs_scroll_with_the_active_one_visible() {
        let names: Vec<String> = (0..30).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let list = items(&refs);
        let mt = Metrics::new(1.0, false);
        let b = layout(
            &list,
            Some(25),
            None,
            1000,
            0,
            None,
            &mt,
            &Colors::DARK,
            &mut ApproxMeasure,
        );
        assert!(b.overflow);
        assert!(
            b.spans.iter().any(|s| s.1 == Hit::Tab(25)),
            "active visible"
        );
        assert!(b.spans.iter().any(|s| s.1 == Hit::Left), "scrolls left");
        let tabs = tab_rects(&b);
        assert!(tabs.iter().all(|t| t.w == mt.min_w), "at the minimum");
        let gear = b.rect_of(Hit::Settings).unwrap();
        for (r, _) in &b.spans {
            if r != &gear {
                assert!(r.right() <= gear.x, "nothing under the gear: {r:?}");
            }
        }
        assert!(b.text.contains('‹'));
        // Scrolled to the start: no left arrow and no gap before the first tab.
        let b = layout(
            &list,
            Some(0),
            None,
            1000,
            0,
            None,
            &mt,
            &Colors::DARK,
            &mut ApproxMeasure,
        );
        assert!(b.rect_of(Hit::Left).is_none());
        assert_eq!(
            b.rect_of(Hit::Tab(0)).unwrap().x,
            mt.bar_pad as i32,
            "no gap"
        );
        assert!(b.rect_of(Hit::Right).is_some());
    }

    #[test]
    fn no_tabs_is_just_new_and_settings() {
        let b = bar(&[], None, 600, 1.0);
        assert_eq!(b.text, "+ \u{2699}");
        assert!(b.active.is_none());
    }

    #[test]
    fn marks_show_shapes_for_states_and_running_needs_none() {
        let mut list = items(&["run", "stop", "agent", "back", "build", "pin"]);
        list[1].status = Status::Stopped;
        list[2].claude = Claude::NeedInput;
        list[3].alert = Some(crate::gui::model::Alert::Output);
        list[4].progress = Some(subak_term::screen::Progress {
            state: subak_term::screen::ProgressState::Normal,
            value: 42,
        });
        list[5].pinned = true;
        let b = bar(&list, Some(0), 2400, 1.0);
        for want in [
            "run",
            "■ stop",
            "! agent",
            "• back",
            "42% build",
            "\u{25c6} pin",
        ] {
            assert!(b.text.contains(want), "{want} in {}", b.text);
        }
        assert!(
            !b.text.contains('●'),
            "running is the normal state: no mark"
        );
        assert!(
            b.rect_of(Hit::Close(5)).is_none(),
            "pinned: no close button"
        );
    }

    /// #98 T1 (`CORE-10`, `CORE-14`): a pinned tab has its ◆ and no close button; a tab without
    /// close buttons (`tabs.close_buttons = false`) keeps its width and has no close target.
    #[test]
    fn pinned_tabs_and_tabs_without_close_buttons() {
        let mut list = items(&["pinned", "plain", "noclose"]);
        list[0].pinned = true;
        list[2].closable = false;
        let b = bar(&list, Some(1), 1200, 1.0);
        assert!(b.text.contains("\u{25c6} pinned"), "{}", b.text);
        assert!(b.rect_of(Hit::Close(0)).is_none() && b.rect_of(Hit::Close(2)).is_none());
        assert!(b.rect_of(Hit::Close(1)).is_some());
        let w: Vec<u32> = tab_rects(&b).iter().map(|r| r.w).collect();
        assert!(w.iter().all(|x| *x == w[0]), "same width {w:?}");
    }

    /// #98 T3 (`CORE-78`): the program's progress is shown on its tab, in words and shape.
    #[test]
    fn progress_shows_on_the_tab() {
        use subak_term::screen::{Progress, ProgressState};
        let mut list = items(&["build", "copy", "fail"]);
        list[0].progress = Some(Progress {
            state: ProgressState::Normal,
            value: 42,
        });
        list[1].progress = Some(Progress {
            state: ProgressState::Indeterminate,
            value: 0,
        });
        list[2].progress = Some(Progress {
            state: ProgressState::Error,
            value: 7,
        });
        let b = bar(&list, Some(0), 1600, 1.0);
        for want in ["42% build", "\u{2026} copy", "7%! fail"] {
            assert!(b.text.contains(want), "{want} in {}", b.text);
        }
    }

    /// #98 T3 (`CORE-80`): unseen output (•) and a bell (♪) are marks beside the name.
    #[test]
    fn alert_marks_show_beside_the_status() {
        let mut list = items(&["front", "out", "bell"]);
        list[1].alert = Some(crate::gui::model::Alert::Output);
        list[2].alert = Some(crate::gui::model::Alert::Bell);
        let b = bar(&list, Some(0), 1600, 1.0);
        assert!(b.text.contains("\u{2022} out"), "{}", b.text);
        assert!(b.text.contains("\u{266a} bell"), "{}", b.text);
        let before = bar(&items(&["front", "out", "bell"]), Some(0), 1600, 1.0);
        assert_eq!(before.spans, b.spans, "marks never move the tabs");
    }

    #[test]
    fn a_motion_frame_replaces_the_glyph_without_moving_anything() {
        use crate::gui::claude_motion::{Frame, SPIN};
        let mut list = items(&["agent"]);
        list[0].claude = Claude::InProgress;
        let rest = bar(&list, Some(0), 800, 1.0);
        for g in SPIN {
            list[0].frame = Some(Frame {
                glyph: g,
                faint: false,
            });
            let b = bar(&list, Some(0), 800, 1.0);
            assert!(b.text.contains(g), "{}", b.text);
            assert_eq!(b.spans, rest.spans, "a frame never moves the tabs");
        }
    }

    #[test]
    fn a_hovered_tab_is_filled_without_moving_anything() {
        let list = items(&["one", "two", "three"]);
        let mt = Metrics::new(1.0, false);
        let plain = lay(&list, Some(0), None, 900, None, &mt);
        let hov = lay(&list, Some(0), None, 900, Some(1), &mt);
        assert_eq!(plain.spans, hov.spans);
        let t1 = hov.rect_of(Hit::Tab(1)).unwrap();
        assert!(hov.ops.iter().any(
            |o| matches!(o, Op::Fill { rect, color, .. } if *rect == t1 && *color == BG_HOVER)
        ));
        assert!(
            hov.parts
                .iter()
                .any(|p| p.id.ends_with("[1]") && p.state.contains(&"hover"))
        );
    }

    #[test]
    fn the_rename_field_shows_a_caret_inside_the_active_tab() {
        let list = items(&["one", "two"]);
        let mt = Metrics::new(1.0, false);
        let e = Edit {
            text: "renamed",
            cursor: 3,
        };
        let b = lay(&list, Some(1), Some(&e), 900, None, &mt);
        let (cx, _) = b.caret.unwrap();
        let t = b.rect_of(Hit::Tab(1)).unwrap();
        assert!(cx > t.x && cx < t.right(), "caret {cx} in {t:?}");
        assert!(b.text.contains("renamed"));
        let long = Edit {
            text: "a very long new name that does not fit into one tab at all",
            cursor: 58,
        };
        let b = lay(&list, Some(1), Some(&long), 900, None, &mt);
        let (cx, _) = b.caret.unwrap();
        assert!(cx < t.right(), "the caret stays visible at the end");
    }

    #[test]
    fn coloured_tabs_start_with_their_colour() {
        let mut list = items(&["build", "logs"]);
        let plain = bar(&list, Some(0), 900, 1.0);
        list[1].color = color_named("teal");
        let coloured = bar(&list, Some(0), 900, 1.0);
        assert_eq!(plain.spans, coloured.spans);
        assert!(coloured.text.contains("\u{258e}logs"), "{}", coloured.text);
        assert!(
            coloured
                .ops
                .iter()
                .any(|o| matches!(o, Op::Fill { color, .. } if *color == Rgb(0x2E, 0xA8, 0xA8)))
        );
        assert_eq!(color_named("nope"), None);
        assert_eq!(TAB_COLORS.len(), 8);
    }

    /// ADR 0099: the bar's parts pass the design audit at every scale and density.
    #[test]
    fn the_laid_out_bar_passes_the_design_audit() {
        use kim_design::audit;
        let list = items(&[
            "main",
            "build",
            "로그",
            "ssh-prod",
            "a-much-longer-terminal-name",
        ]);
        for s in [1.0f32, 1.25, 1.5, 2.0] {
            for compact in [false, true] {
                let mt = Metrics::new(s, compact);
                let w = (1400.0 * s) as u32;
                let b = lay(&list, Some(1), None, w, Some(3), &mt);
                let r = report(&b, w, mt.height, s);
                let f = audit::audit(&r);
                assert_eq!(audit::counts(&f).2, 0, "{}", audit::render(&r, &f, false));
            }
        }
    }

    #[test]
    fn tab_titles_follow_the_format() {
        assert_eq!(
            label("{name}", "build", Some("make"), "cmd.exe", None),
            "build"
        );
        assert_eq!(
            label(
                "{name}: {title}",
                "build",
                Some("make -j8"),
                "cmd.exe",
                None
            ),
            "build: make -j8"
        );
        assert_eq!(
            label(
                "{program} in {cwd}",
                "x",
                None,
                "pwsh.exe",
                Some(r"C:\work\kim\")
            ),
            "pwsh.exe in kim"
        );
        assert_eq!(
            label("{title}", "build", None, "cmd.exe", None),
            "build",
            "falls back"
        );
        assert_eq!(label("{title}", "b", Some("a\u{7}b"), "c", None), "ab");
    }
}
