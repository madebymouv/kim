//! Automatic layout checks (ADR 0099): a geometry [`Report`] measured from a real program against
//! the design contract — the token sizes at the report's scale, containment, clipping, hit
//! targets, type sizes, consistent repeated components, even spacing, overlap, vertical centring
//! and colour contrast.
//!
//! Tolerances are per rule, never one number for everything: positions and sizes allow
//! [`TOL_PX`] (one physical pixel: token × scale is rounded once per edge), font sizes
//! [`TOL_FONT`]. Subjective qualities (balance, clutter) are not scored here; they are reviewed
//! on the screenshots the same run takes (docs/UI_QUALITY_PIPELINE.md).

use crate::inspect::{Component, Report};
use crate::{Theme, font, size};
use std::collections::BTreeMap;

/// Alignment and size tolerance: one physical pixel.
pub const TOL_PX: f32 = 1.0;
/// Font size tolerance in physical pixels (rasterisers round sizes to whole pixels).
pub const TOL_FONT: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Pass,
    Warn,
    Fail,
}

impl Level {
    pub fn word(self) -> &'static str {
        match self {
            Level::Pass => "PASS",
            Level::Warn => "WARN",
            Level::Fail => "FAIL",
        }
    }
}

/// One rule result about one component.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub level: Level,
    pub rule: &'static str,
    pub component: String,
    pub message: String,
    pub actual: String,
    pub expected: String,
}

/// What the contract says about a component type.
struct Expect {
    heights: &'static [f32],
    width: Option<(f32, f32)>,
    min_width: Option<f32>,
    centered: bool,
    uniform_h: bool,
    uniform_w: bool,
    even_gaps: bool,
}

const NONE: Expect = Expect {
    heights: &[],
    width: None,
    min_width: None,
    centered: false,
    uniform_h: false,
    uniform_w: false,
    even_gaps: false,
};

fn expect(kind: &str) -> Expect {
    match kind {
        "tabbar" => Expect {
            heights: &[size::TABBAR_HEIGHT, size::TABBAR_HEIGHT_COMPACT],
            ..NONE
        },
        "tab" => Expect {
            heights: &[
                size::TABBAR_HEIGHT - size::TAB_INSET_TOP,
                size::TABBAR_HEIGHT_COMPACT - size::TAB_INSET_TOP,
            ],
            width: Some((size::TAB_MIN_WIDTH, size::TAB_MAX_WIDTH)),
            centered: true,
            uniform_h: true,
            uniform_w: true,
            ..NONE
        },
        "button" => Expect {
            heights: &[size::CONTROL, size::CONTROL_COMPACT, size::CONTROL_LARGE],
            min_width: Some(size::BUTTON_MIN_WIDTH),
            centered: true,
            ..NONE
        },
        "switch" => Expect {
            heights: &[size::SWITCH_HEIGHT],
            width: Some((size::SWITCH_WIDTH, size::SWITCH_WIDTH)),
            ..NONE
        },
        "nav-item" => Expect {
            heights: &[size::NAV_ITEM],
            centered: true,
            uniform_h: true,
            uniform_w: true,
            even_gaps: true,
            ..NONE
        },
        "list-row" => Expect {
            heights: &[size::LIST_ROW],
            uniform_h: true,
            uniform_w: true,
            even_gaps: true,
            ..NONE
        },
        "menu-item" => Expect {
            heights: &[size::MENU_ITEM],
            centered: true,
            uniform_h: true,
            uniform_w: true,
            ..NONE
        },
        "text-input" | "dropdown" => Expect {
            heights: &[size::CONTROL, size::CONTROL_COMPACT],
            ..NONE
        },
        "segmented" => Expect {
            heights: &[size::CONTROL, size::CONTROL_COMPACT],
            ..NONE
        },
        "gear" => Expect {
            heights: &[size::GEAR],
            width: Some((size::GEAR, size::GEAR)),
            ..NONE
        },
        "tile" => Expect {
            uniform_h: true,
            uniform_w: true,
            even_gaps: true,
            ..NONE
        },
        "tab-new" | "icon-button" => Expect {
            heights: &[size::CONTROL, size::CONTROL_COMPACT, size::HIT_MIN],
            ..NONE
        },
        _ => NONE,
    }
}

/// The UI type scale (logical pixels) every UI text uses.
const TYPE_SCALE: [f32; 5] = [
    font::CAPTION,
    font::BODY,
    font::EMPHASIS,
    font::TITLE,
    font::DISPLAY,
];

fn px(v: f32) -> String {
    format!("{v:.1}px")
}

/// Runs every rule on `r`; one finding per failed rule and component, plus a `Pass` finding
/// for every interactive or structural component without failures.
pub fn audit(r: &Report) -> Vec<Finding> {
    let s = if r.scale > 0.0 { r.scale } else { 1.0 };
    let mut out = Vec::new();
    let by_id: BTreeMap<&str, &Component> =
        r.components.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut fail = |c: &Component, level: Level, rule: &'static str, msg: String, a, e| {
        out.push(Finding {
            level,
            rule,
            component: c.id.clone(),
            message: msg,
            actual: a,
            expected: e,
        });
    };
    let theme = if r.theme == "kim-light" {
        Theme::Light
    } else {
        Theme::Dark
    };
    let pal = theme.palette();
    for c in r.components.iter().filter(|c| c.visible) {
        let b = c.bounds;
        let ex = expect(&c.kind);
        // Containment: inside the parent (a clipped component is cut by a scroll area).
        if let Some(p) = c.parent.as_deref().and_then(|p| by_id.get(p))
            && !c.clipped
            && !p.bounds.contains(&b, TOL_PX)
        {
            fail(
                c,
                Level::Fail,
                "inside-parent",
                format!("extends beyond its parent `{}`", p.id),
                format!("{:.0},{:.0} {:.0}×{:.0}", b.x, b.y, b.w, b.h),
                format!(
                    "within {:.0},{:.0} {:.0}×{:.0}",
                    p.bounds.x, p.bounds.y, p.bounds.w, p.bounds.h
                ),
            );
        }
        // Inside the window.
        if r.window.0 > 0.0
            && !c.clipped
            && !crate::inspect::Rect::new(0.0, 0.0, r.window.0, r.window.1).contains(&b, TOL_PX)
        {
            fail(
                c,
                Level::Fail,
                "inside-window",
                "extends beyond the window".into(),
                format!("right {:.0} bottom {:.0}", b.right(), b.bottom()),
                format!("≤ {:.0} × {:.0}", r.window.0, r.window.1),
            );
        }
        // Token heights and widths.
        if !ex.heights.is_empty() && !c.has("custom-size") {
            let ok = ex
                .heights
                .iter()
                .any(|h| (b.h - (h * s).round()).abs() <= TOL_PX);
            if !ok {
                fail(
                    c,
                    Level::Fail,
                    "token-height",
                    format!(
                        "{} height differs from the design token at scale {s}",
                        c.kind
                    ),
                    px(b.h),
                    ex.heights
                        .iter()
                        .map(|h| px((h * s).round()))
                        .collect::<Vec<_>>()
                        .join(" or "),
                );
            }
        }
        if let Some((lo, hi)) = ex.width
            && !c.has("overflow")
            && !c.has("editing")
            && (b.w < (lo * s).round() - TOL_PX || b.w > (hi * s).round() + TOL_PX)
        {
            fail(
                c,
                Level::Fail,
                "token-width",
                format!("{} width outside the design range", c.kind),
                px(b.w),
                format!("{}–{}", px((lo * s).round()), px((hi * s).round())),
            );
        }
        let min_width = if c.has("compact") {
            ex.min_width.map(|_| size::BUTTON_MIN_WIDTH_COMPACT)
        } else {
            ex.min_width
        };
        if let Some(min) = min_width
            && !c.has("icon")
            && !c.has("ghost")
            && b.w < (min * s).round() - TOL_PX
        {
            fail(
                c,
                Level::Fail,
                "token-width",
                format!("{} narrower than its minimum", c.kind),
                px(b.w),
                format!("≥ {}", px((min * s).round())),
            );
        }
        // Hit targets.
        if c.interactive {
            let h = c.hit_rect();
            let min = (size::HIT_MIN * s).round() - TOL_PX;
            if h.w < min || h.h < min {
                fail(
                    c,
                    Level::Fail,
                    "hit-target",
                    "pointer target smaller than the minimum".into(),
                    format!("{:.0}×{:.0}", h.w, h.h),
                    format!("≥ {0:.0}×{0:.0}", (size::HIT_MIN * s).round()),
                );
            }
        }
        if let Some(t) = &c.text {
            // Text inside its component unless cut on purpose.
            if !t.truncated && !b.contains(&t.bounds, TOL_PX) {
                fail(
                    c,
                    Level::Fail,
                    "text-clipped",
                    format!("text `{}` does not fit its component", t.content),
                    format!(
                        "text {:.0},{:.0} {:.0}×{:.0}",
                        t.bounds.x, t.bounds.y, t.bounds.w, t.bounds.h
                    ),
                    format!("within {:.0},{:.0} {:.0}×{:.0}", b.x, b.y, b.w, b.h),
                );
            }
            if t.truncated && t.bounds.w > b.w + TOL_PX {
                fail(
                    c,
                    Level::Fail,
                    "text-clipped",
                    "truncated text still wider than its component".into(),
                    px(t.bounds.w),
                    format!("≤ {}", px(b.w)),
                );
            }
            // UI text uses the type scale.
            if t.role != "terminal"
                && t.role != "icon"
                && !TYPE_SCALE
                    .iter()
                    .any(|f| (t.font_px - f * s).abs() <= TOL_FONT)
            {
                fail(
                    c,
                    Level::Fail,
                    "type-scale",
                    "font size is not on the UI type scale".into(),
                    px(t.font_px),
                    TYPE_SCALE
                        .iter()
                        .map(|f| format!("{:.1}", f * s))
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
            if t.font_px > 0.0 && t.font_px < 10.0 * r.dpi_scale - TOL_FONT {
                fail(
                    c,
                    Level::Fail,
                    "min-readable",
                    "text smaller than 10 logical pixels".into(),
                    px(t.font_px),
                    format!("≥ {}", px(10.0 * r.dpi_scale)),
                );
            }
            // Vertical centring of the label in single-line controls.
            if ex.centered && (t.bounds.center_y() - b.center_y()).abs() > TOL_PX + 0.5 {
                fail(
                    c,
                    Level::Fail,
                    "centered",
                    "label not vertically centred".into(),
                    format!("centre y {:.1}", t.bounds.center_y()),
                    format!("{:.1} ±{:.1}", b.center_y(), TOL_PX + 0.5),
                );
            }
            // Contrast of text on its fill.
            if let (Some(fg), Some(bg)) = (c.fg, c.bg)
                && !c.has("disabled")
            {
                let cr = fg.contrast(bg);
                if cr < 4.5 {
                    fail(
                        c,
                        Level::Fail,
                        "contrast",
                        "text contrast below WCAG 4.5:1".into(),
                        format!("{cr:.2}:1 ({} on {})", fg.to_hex(), bg.to_hex()),
                        "≥ 4.5:1".into(),
                    );
                }
            }
        }
        // Colours from the palette (a mix while hovering or animating is a warning only).
        for (what, col) in [("fg", c.fg), ("bg", c.bg)] {
            if let Some(col) = col
                && !c.has("disabled")
                && !in_palette(pal, col)
            {
                fail(
                    c,
                    Level::Warn,
                    "palette",
                    format!("{what} colour is not a token of {}", r.theme),
                    col.to_hex(),
                    "a palette role".into(),
                );
            }
        }
        if c.has("disabled")
            && let Some(fg) = c.fg
            && (fg.contrast(pal.text_primary) < 1.4 || fg.contrast(pal.text_secondary) < 1.15)
        {
            fail(
                c,
                Level::Fail,
                "disabled-distinct",
                "disabled control drawn like an enabled one".into(),
                fg.to_hex(),
                pal.text_disabled.to_hex(),
            );
        }
    }
    // A bar's content starts at its inset: no unexplained gap before the first control.
    for bar in r
        .components
        .iter()
        .filter(|c| c.visible && c.kind == "tabbar")
    {
        let first = r
            .components
            .iter()
            .filter(|c| c.visible && c.interactive && c.parent.as_deref() == Some(bar.id.as_str()))
            .min_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x));
        if let Some(c) = first {
            let gap = c.bounds.x - bar.bounds.x;
            let max = (crate::space::S * s).round() + TOL_PX;
            if gap > max {
                fail(
                    c,
                    Level::Fail,
                    "leading-gap",
                    format!("unexpected gap before the first control of `{}`", bar.id),
                    px(gap),
                    format!("≤ {}", px(max - TOL_PX)),
                );
            }
        }
    }
    // Groups of repeated components: same parent, same type.
    let mut groups: BTreeMap<(String, String), Vec<&Component>> = BTreeMap::new();
    for c in r.components.iter().filter(|c| c.visible && !c.clipped) {
        groups
            .entry((c.parent.clone().unwrap_or_default(), c.kind.clone()))
            .or_default()
            .push(c);
    }
    for ((parent, kind), list) in &groups {
        if list.len() < 2 {
            continue;
        }
        let ex = expect(kind);
        let first = list[0];
        let regular: Vec<&&Component> = list
            .iter()
            .filter(|c| !c.has("editing") && !c.has("custom-size"))
            .collect();
        if ex.uniform_h
            && let Some(c) = regular
                .iter()
                .find(|c| (c.bounds.h - first.bounds.h).abs() > TOL_PX)
        {
            fail(
                c,
                Level::Fail,
                "uniform-size",
                format!("{kind} heights differ within `{parent}`"),
                px(c.bounds.h),
                px(first.bounds.h),
            );
        }
        if ex.uniform_w
            && let Some(c) = regular
                .iter()
                .find(|c| (c.bounds.w - regular[0].bounds.w).abs() > TOL_PX)
        {
            fail(
                c,
                Level::Fail,
                "uniform-size",
                format!("{kind} widths differ within `{parent}`"),
                px(c.bounds.w),
                px(regular[0].bounds.w),
            );
        }
        if ex.even_gaps && list.len() >= 3 {
            let column = list
                .iter()
                .all(|c| (c.bounds.x - first.bounds.x).abs() <= TOL_PX);
            let mut sorted: Vec<&Component> = list.clone();
            if column {
                sorted.sort_by(|a, b| a.bounds.y.total_cmp(&b.bounds.y));
            } else {
                sorted.sort_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x));
            }
            let gaps: Vec<f32> = sorted
                .windows(2)
                .filter(|w| column || (w[0].bounds.y - w[1].bounds.y).abs() <= TOL_PX)
                .map(|w| {
                    if column {
                        w[1].bounds.y - w[0].bounds.bottom()
                    } else {
                        w[1].bounds.x - w[0].bounds.right()
                    }
                })
                .collect();
            if let Some(g0) = gaps.first()
                && let Some((i, g)) = gaps
                    .iter()
                    .enumerate()
                    .find(|(_, g)| (*g - g0).abs() > TOL_PX)
            {
                fail(
                    sorted[i + 1],
                    Level::Fail,
                    "even-spacing",
                    format!("uneven gaps between {kind}s in `{parent}`"),
                    px(*g),
                    px(*g0),
                );
            }
        }
    }
    // Interactive siblings (any type, same parent) never overlap, unless one holds the other.
    let mut by_parent: BTreeMap<String, Vec<&Component>> = BTreeMap::new();
    for c in r
        .components
        .iter()
        .filter(|c| c.visible && !c.clipped && c.interactive)
    {
        by_parent
            .entry(c.parent.clone().unwrap_or_default())
            .or_default()
            .push(c);
    }
    for list in by_parent.values() {
        for (i, a) in list.iter().enumerate() {
            for b in &list[i + 1..] {
                let (ha, hb) = (a.hit_rect(), b.hit_rect());
                if ha.contains(&hb, TOL_PX) || hb.contains(&ha, TOL_PX) {
                    continue;
                }
                if ha.overlap(&hb) > TOL_PX * TOL_PX {
                    fail(
                        b,
                        Level::Fail,
                        "overlap",
                        format!("pointer areas of `{}` and `{}` overlap", a.id, b.id),
                        format!("{:.0} px²", a.hit_rect().overlap(&b.hit_rect())),
                        "0".into(),
                    );
                }
            }
        }
    }
    // A pass line for each checked component without failures.
    let failed: std::collections::BTreeSet<String> = out
        .iter()
        .filter(|f| f.level == Level::Fail)
        .map(|f| f.component.clone())
        .collect();
    for c in r.components.iter().filter(|c| c.visible) {
        if !failed.contains(&c.id)
            && (c.interactive || expect(&c.kind).heights.len() + usize::from(c.text.is_some()) > 0)
        {
            out.push(Finding {
                level: Level::Pass,
                rule: "all",
                component: c.id.clone(),
                message: "dimensions, text and states match the design contract".into(),
                actual: String::new(),
                expected: String::new(),
            });
        }
    }
    out
}

fn in_palette(p: &crate::Palette, c: crate::Rgb) -> bool {
    [
        p.bg_base,
        p.bg_raised,
        p.bg_overlay,
        p.bg_hover,
        p.bg_selected,
        p.border_subtle,
        p.border_strong,
        p.text_primary,
        p.text_secondary,
        p.text_muted,
        p.text_disabled,
        p.text_on_accent,
        p.accent,
        p.accent_hover,
        p.accent_pressed,
        p.accent_soft,
        p.focus,
        p.success,
        p.warning,
        p.danger,
        p.info,
        p.switch_off,
    ]
    .contains(&c)
}

/// The findings as the text report (`FAIL  subak.tabbar` + indented details); passes are
/// listed after the failures, one line each, when `passes` is set.
pub fn render(r: &Report, findings: &[Finding], passes: bool) -> String {
    let mut s = format!(
        "{} · {} · {} · window {:.0}×{:.0} · DPI {:.2} · zoom {:.2} · scale {:.2}\n",
        r.application, r.screen, r.fixture, r.window.0, r.window.1, r.dpi_scale, r.ui_zoom, r.scale
    );
    let mut sorted: Vec<&Finding> = findings.iter().collect();
    sorted.sort_by(|a, b| b.level.cmp(&a.level).then(a.component.cmp(&b.component)));
    for f in sorted {
        if f.level == Level::Pass {
            if passes {
                s.push_str(&format!("PASS  {}\n", f.component));
            }
            continue;
        }
        s.push_str(&format!(
            "{}  {}\n      {} [{}]: actual {}, expected {}\n",
            f.level.word(),
            f.component,
            f.message,
            f.rule,
            f.actual,
            f.expected
        ));
    }
    s
}

/// How many findings of each level.
pub fn counts(findings: &[Finding]) -> (usize, usize, usize) {
    let n = |l| findings.iter().filter(|f| f.level == l).count();
    (n(Level::Pass), n(Level::Warn), n(Level::Fail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect::{Rect, Text};

    fn report(scale: f32) -> Report {
        Report {
            application: "test".into(),
            screen: "bar".into(),
            theme: "kim-dark".into(),
            window: (800.0, 600.0),
            dpi_scale: 1.0,
            ui_zoom: scale,
            scale,
            ..Report::default()
        }
    }

    fn tab(i: usize, x: f32, w: f32, h: f32, text_y: f32) -> Component {
        Component::new(format!("tab[{i}]"), "tab", Rect::new(x, 4.0, w, h))
            .parent("bar")
            .interactive()
            .text(Text {
                content: format!("t{i}"),
                bounds: Rect::new(x + 12.0, text_y, 20.0, 16.0),
                font_px: 12.0,
                line_px: 16.0,
                truncated: false,
                role: "ui".into(),
            })
            .colors(crate::DARK.text_secondary, crate::DARK.bg_raised)
    }

    #[test]
    fn a_correct_bar_passes_every_rule() {
        let mut r = report(1.0);
        r.components.push(Component::new(
            "bar",
            "tabbar",
            Rect::new(0.0, 0.0, 800.0, 36.0),
        ));
        for i in 0..3 {
            r.components
                .push(tab(i, 6.0 + i as f32 * 150.0, 150.0, 32.0, 12.0));
        }
        let f = audit(&r);
        assert_eq!(counts(&f).2, 0, "{}", render(&r, &f, false));
    }

    #[test]
    fn measured_defects_are_found_with_actual_and_expected_values() {
        let mut r = report(1.0);
        // The 0.0.1 defect: a 56 px bar at 100 %, a 21 px font, uneven tabs, text off-centre.
        r.components.push(Component::new(
            "bar",
            "tabbar",
            Rect::new(0.0, 0.0, 800.0, 56.0),
        ));
        let mut t0 = tab(0, 6.0, 150.0, 52.0, 30.0);
        t0.text.as_mut().unwrap().font_px = 21.0;
        r.components.push(t0);
        r.components.push(tab(1, 156.0, 120.0, 52.0, 22.0));
        let mut close = Component::new("close", "tab-close", Rect::new(150.0, 20.0, 12.0, 12.0))
            .parent("tab[0]")
            .interactive();
        close.bg = Some(crate::Rgb::hex(0x123456));
        r.components.push(close);
        let f = audit(&r);
        let rules: Vec<(&str, &str)> = f
            .iter()
            .filter(|f| f.level == Level::Fail)
            .map(|f| (f.component.as_str(), f.rule))
            .collect();
        for want in [
            ("bar", "token-height"),
            ("tab[0]", "type-scale"),
            ("tab[0]", "centered"),
            ("tab[1]", "uniform-size"),
            ("close", "hit-target"),
            ("close", "inside-parent"),
        ] {
            assert!(rules.contains(&want), "{want:?} missing in {rules:?}");
        }
        assert!(
            f.iter()
                .any(|f| f.level == Level::Warn && f.rule == "palette")
        );
        let text = render(&r, &f, true);
        assert!(text.contains("FAIL  bar\n"), "{text}");
        assert!(
            text.contains("actual 56.0px, expected 36.0px or 28.0px"),
            "{text}"
        );
    }

    #[test]
    fn low_contrast_and_hidden_disabled_states_fail() {
        let mut r = report(1.0);
        let mut c = Component::new("b", "button", Rect::new(0.0, 0.0, 80.0, 28.0))
            .interactive()
            .text(Text {
                content: "Save".into(),
                bounds: Rect::new(20.0, 6.0, 30.0, 16.0),
                font_px: 12.0,
                line_px: 16.0,
                truncated: false,
                role: "ui".into(),
            })
            .colors(crate::DARK.text_disabled, crate::DARK.bg_base);
        r.components.push(c.clone());
        c.id = "d".into();
        c.state.push("disabled".into());
        c.fg = Some(crate::DARK.text_primary);
        r.components.push(c);
        let f = audit(&r);
        assert!(f.iter().any(|f| f.component == "b" && f.rule == "contrast"));
        assert!(
            f.iter()
                .any(|f| f.component == "d" && f.rule == "disabled-distinct")
        );
    }
}
