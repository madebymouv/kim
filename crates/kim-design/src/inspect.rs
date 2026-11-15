//! UI geometry reports (ADR 0099, docs/UI_QUALITY_PIPELINE.md): what a program actually laid out
//! and painted, in physical pixels, so [`crate::audit`] can compare it with the design contract.
//!
//! A program fills a [`Report`] from its **real** layout code (egui `Response` rectangles in
//! `kim-ui`, the pixel layout of Subak's tab bar, measured GDI text) — never from the token values
//! it was supposed to use. Collection is off unless the developer asks for it
//! ([`requested_path`]); with it off nothing is recorded and nothing is written.

use crate::Rgb;
use serde_json::{Value, json};

/// The environment variable that turns collection on: the file a report is written to.
pub const ENV: &str = "KIM_UI_INSPECT";
/// The environment variable that asks a program for a screenshot of the same frame (PNG).
pub const ENV_SNAPSHOT: &str = "KIM_UI_SNAPSHOT";
/// The environment variable that names the view a program opens for an audit (`settings`, …).
pub const ENV_SCREEN: &str = "KIM_UI_SCREEN";
/// The schema name of a report.
pub const SCHEMA: &str = "kim.ui-geometry";

/// The report file the developer asked for (`KIM_UI_INSPECT`), read once.
pub fn requested_path() -> Option<&'static std::path::Path> {
    static P: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        std::env::var_os(ENV)
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
    })
    .as_deref()
}

/// Whether collection is on (one cached check; free when off).
pub fn enabled() -> bool {
    requested_path().is_some()
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn center_y(&self) -> f32 {
        self.y + self.h / 2.0
    }
    pub fn center_x(&self) -> f32 {
        self.x + self.w / 2.0
    }
    /// Whether `inner` lies inside this rectangle, `tol` pixels allowed on each side.
    pub fn contains(&self, inner: &Rect, tol: f32) -> bool {
        inner.x >= self.x - tol
            && inner.y >= self.y - tol
            && inner.right() <= self.right() + tol
            && inner.bottom() <= self.bottom() + tol
    }
    /// The overlapping area with `o` (0 when they only touch).
    pub fn overlap(&self, o: &Rect) -> f32 {
        let w = self.right().min(o.right()) - self.x.max(o.x);
        let h = self.bottom().min(o.bottom()) - self.y.max(o.y);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }
    fn json(&self) -> Value {
        json!({"x": r1(self.x), "y": r1(self.y), "width": r1(self.w), "height": r1(self.h)})
    }
    fn from_json(v: &Value) -> Option<Rect> {
        Some(Rect {
            x: v.get("x")?.as_f64()? as f32,
            y: v.get("y")?.as_f64()? as f32,
            w: v.get("width")?.as_f64()? as f32,
            h: v.get("height")?.as_f64()? as f32,
        })
    }
}

fn r1(v: f32) -> f64 {
    (f64::from(v) * 10.0).round() / 10.0
}

/// Text as laid out: its content, the box the text occupies (measured), its font size and line
/// height in physical pixels, and whether it was cut with an ellipsis.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
    pub content: String,
    pub bounds: Rect,
    pub font_px: f32,
    pub line_px: f32,
    pub truncated: bool,
    /// `ui` (the UI type scale applies) or `terminal` (the user's terminal size).
    pub role: String,
}

/// One laid-out component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Component {
    /// Stable id (`subak.tabbar.tab[2]`, `kim.settings.nav.general`).
    pub id: String,
    /// The component type: `tabbar`, `tab`, `tab-close`, `icon-button`, `button`, `switch`,
    /// `nav-item`, `setting-row`, `list-row`, `text-input`, `dropdown`, `menu-item`, `tile`,
    /// `label`, `region`, …
    pub kind: String,
    pub parent: Option<String>,
    pub bounds: Rect,
    /// The area that reacts to the pointer (when it differs from `bounds`).
    pub hit: Option<Rect>,
    pub text: Option<Text>,
    /// Inner padding: left, top, right, bottom.
    pub padding: Option<[f32; 4]>,
    pub visible: bool,
    /// Drawn only partly: cut by a scroll area or the window.
    pub clipped: bool,
    pub interactive: bool,
    /// `active`, `hover`, `selected`, `disabled`, `focused`, `editing`, `overflow`, …
    pub state: Vec<String>,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Component {
    /// A visible component of `kind` at `bounds`.
    pub fn new(id: impl Into<String>, kind: &str, bounds: Rect) -> Component {
        Component {
            id: id.into(),
            kind: kind.to_string(),
            bounds,
            visible: true,
            ..Component::default()
        }
    }
    pub fn parent(mut self, p: impl Into<String>) -> Component {
        self.parent = Some(p.into());
        self
    }
    pub fn interactive(mut self) -> Component {
        self.interactive = true;
        self
    }
    pub fn hit(mut self, r: Rect) -> Component {
        self.hit = Some(r);
        self
    }
    pub fn text(mut self, t: Text) -> Component {
        self.text = Some(t);
        self
    }
    pub fn colors(mut self, fg: Rgb, bg: Rgb) -> Component {
        self.fg = Some(fg);
        self.bg = Some(bg);
        self
    }
    pub fn state(mut self, s: &str) -> Component {
        self.state.push(s.to_string());
        self
    }
    pub fn has(&self, s: &str) -> bool {
        self.state.iter().any(|x| x == s)
    }
    /// The pointer area: `hit` or the bounds.
    pub fn hit_rect(&self) -> Rect {
        self.hit.unwrap_or(self.bounds)
    }

    fn json(&self) -> Value {
        let mut v = json!({
            "id": self.id,
            "type": self.kind,
            "bounds": self.bounds.json(),
            "visible": self.visible,
        });
        let o = v.as_object_mut().expect("object");
        if let Some(p) = &self.parent {
            o.insert("parent".into(), json!(p));
        }
        if let Some(h) = &self.hit {
            o.insert("hit".into(), h.json());
        }
        if let Some(t) = &self.text {
            o.insert(
                "text".into(),
                json!({"content": t.content, "bounds": t.bounds.json(), "font_px": r1(t.font_px),
                    "line_px": r1(t.line_px), "truncated": t.truncated, "role": t.role}),
            );
        }
        if let Some(p) = self.padding {
            o.insert("padding".into(), json!(p.map(r1)));
        }
        if self.clipped {
            o.insert("clipped".into(), json!(true));
        }
        if self.interactive {
            o.insert("interactive".into(), json!(true));
        }
        if !self.state.is_empty() {
            o.insert("state".into(), json!(self.state));
        }
        if let Some(c) = self.fg {
            o.insert("fg".into(), json!(c.to_hex()));
        }
        if let Some(c) = self.bg {
            o.insert("bg".into(), json!(c.to_hex()));
        }
        v
    }

    fn from_json(v: &Value) -> Option<Component> {
        let color = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .and_then(|s| u32::from_str_radix(s.trim_start_matches('#'), 16).ok())
                .map(Rgb::hex)
        };
        Some(Component {
            id: v.get("id")?.as_str()?.to_string(),
            kind: v.get("type")?.as_str()?.to_string(),
            parent: v.get("parent").and_then(Value::as_str).map(str::to_string),
            bounds: Rect::from_json(v.get("bounds")?)?,
            hit: v.get("hit").and_then(Rect::from_json),
            text: v.get("text").and_then(|t| {
                Some(Text {
                    content: t.get("content")?.as_str()?.to_string(),
                    bounds: Rect::from_json(t.get("bounds")?)?,
                    font_px: t.get("font_px")?.as_f64()? as f32,
                    line_px: t.get("line_px").and_then(Value::as_f64).unwrap_or(0.0) as f32,
                    truncated: t.get("truncated").and_then(Value::as_bool).unwrap_or(false),
                    role: t
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("ui")
                        .to_string(),
                })
            }),
            padding: v.get("padding").and_then(|p| {
                let a = p.as_array()?;
                let f = |i: usize| a.get(i).and_then(Value::as_f64).map(|x| x as f32);
                Some([f(0)?, f(1)?, f(2)?, f(3)?])
            }),
            visible: v.get("visible").and_then(Value::as_bool).unwrap_or(true),
            clipped: v.get("clipped").and_then(Value::as_bool).unwrap_or(false),
            interactive: v
                .get("interactive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            state: v
                .get("state")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            fg: color("fg"),
            bg: color("bg"),
        })
    }
}

/// A geometry report of one view of one program.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// `kim`, `subak`, `noeul`, `ddalgak`.
    pub application: String,
    /// The view: `launcher`, `settings`, `terminal`, `rules`, …
    pub screen: String,
    /// What the run was set up with (`zoom=1.5 theme=dark lang=ko`).
    pub fixture: String,
    /// The theme drawn (`kim-dark` / `kim-light`).
    pub theme: String,
    /// Client area in physical pixels.
    pub window: (f32, f32),
    /// Windows' DPI scale of the monitor.
    pub dpi_scale: f32,
    /// The UI zoom (ADR 0099).
    pub ui_zoom: f32,
    /// Physical pixels per token pixel for the components of this report.
    pub scale: f32,
    /// Further measured facts (`terminal_font_px`, `terminal_cell`, …).
    pub extra: serde_json::Map<String, Value>,
    pub components: Vec<Component>,
}

impl Report {
    pub fn find(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Component> + 'a {
        self.components.iter().filter(move |c| c.kind == kind)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "schema": SCHEMA,
            "schema_version": 1,
            "application": self.application,
            "screen": self.screen,
            "fixture": self.fixture,
            "theme": self.theme,
            "window": {"width": r1(self.window.0), "height": r1(self.window.1)},
            "dpi_scale": self.dpi_scale,
            "ui_zoom": self.ui_zoom,
            "scale": self.scale,
            "extra": self.extra,
            "components": self.components.iter().map(Component::json).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(v: &Value) -> Option<Report> {
        if v.get("schema")?.as_str()? != SCHEMA {
            return None;
        }
        let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(1.0) as f32;
        Some(Report {
            application: v.get("application")?.as_str()?.to_string(),
            screen: v.get("screen")?.as_str()?.to_string(),
            fixture: v
                .get("fixture")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            theme: v
                .get("theme")
                .and_then(Value::as_str)
                .unwrap_or("kim-dark")
                .to_string(),
            window: (
                v["window"]["width"].as_f64().unwrap_or(0.0) as f32,
                v["window"]["height"].as_f64().unwrap_or(0.0) as f32,
            ),
            dpi_scale: f("dpi_scale"),
            ui_zoom: f("ui_zoom"),
            scale: f("scale"),
            extra: v
                .get("extra")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
            components: v
                .get("components")?
                .as_array()?
                .iter()
                .filter_map(Component::from_json)
                .collect(),
        })
    }

    /// Writes the report as pretty JSON to `path`.
    pub fn write(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut s = serde_json::to_string_pretty(&self.to_json()).unwrap_or_default();
        s.push('\n');
        std::fs::write(path, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_round_trips_through_json() {
        let mut r = Report {
            application: "subak".into(),
            screen: "terminal".into(),
            fixture: "zoom=1.5".into(),
            theme: "kim-dark".into(),
            window: (1200.0, 800.0),
            dpi_scale: 1.0,
            ui_zoom: 1.5,
            scale: 1.5,
            ..Report::default()
        };
        r.extra.insert("terminal_font_px".into(), json!(20));
        r.components.push(
            Component::new("subak.tabbar", "tabbar", Rect::new(0.0, 0.0, 1200.0, 54.0))
                .colors(Rgb::hex(0xA9A6B8), Rgb::hex(0x15151B)),
        );
        r.components.push(
            Component::new("subak.tab[0]", "tab", Rect::new(6.0, 0.0, 200.0, 54.0))
                .parent("subak.tabbar")
                .interactive()
                .state("active")
                .text(Text {
                    content: "main".into(),
                    bounds: Rect::new(24.0, 16.0, 40.0, 22.0),
                    font_px: 18.0,
                    line_px: 24.0,
                    truncated: false,
                    role: "ui".into(),
                }),
        );
        let back = Report::from_json(&r.to_json()).unwrap();
        assert_eq!(back, r);
        assert!(Report::from_json(&json!({"schema": "other"})).is_none());
    }

    #[test]
    fn rectangles_contain_and_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(a.contains(&Rect::new(1.0, 1.0, 9.0, 9.0), 0.0));
        assert!(!a.contains(&Rect::new(1.0, 1.0, 10.0, 9.0), 0.0));
        assert!(a.contains(&Rect::new(1.0, 1.0, 10.0, 9.0), 1.0));
        assert_eq!(a.overlap(&Rect::new(10.0, 0.0, 5.0, 5.0)), 0.0, "touching");
        assert_eq!(a.overlap(&Rect::new(8.0, 8.0, 5.0, 5.0)), 4.0);
    }
}
