//! ADR 0099: the geometry the components really laid out, for the UI audit.
//!
//! Every component of [`crate::widgets`] records its allocated rectangle (egui's `Response`
//! rectangle), the box of the text it painted (the laid-out galley or the painter's returned
//! text rectangle), its font size, its clip state and its colours — in physical pixels — while
//! collection is on. Collection is on when the developer set `KIM_UI_INSPECT` (a program then
//! writes the report with [`finish_frame`]) or a test called [`force`]; otherwise
//! [`active`] is one cached check and nothing is recorded.

use egui::{Color32, Rect, Ui};
use kim_design::inspect::{Component, Rect as R, Report, Text};
use std::cell::{Cell, RefCell};

thread_local! {
    static FORCED: Cell<bool> = const { Cell::new(false) };
    static ITEMS: RefCell<Vec<Component>> = const { RefCell::new(Vec::new()) };
    static SCOPES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FRAMES: Cell<u64> = const { Cell::new(0) };
    static SHOT_ASKED: Cell<bool> = const { Cell::new(false) };
    static SETTLE: Cell<(egui::Vec2, u32)> = const { Cell::new((egui::Vec2::ZERO, 0)) };
}

/// Whether components record their geometry now.
pub fn active() -> bool {
    FORCED.with(Cell::get) || kim_design::inspect::enabled()
}

/// Turns collection on or off for this thread (tests).
pub fn force(on: bool) {
    FORCED.with(|f| f.set(on));
    clear();
}

/// Forgets what was recorded (the start of a frame).
pub fn clear() {
    ITEMS.with(|i| i.borrow_mut().clear());
    SCOPES.with(|s| s.borrow_mut().clear());
}

/// What was recorded since [`clear`].
pub fn take() -> Vec<Component> {
    ITEMS.with(|i| std::mem::take(&mut *i.borrow_mut()))
}

/// Puts components back (a second window measured inside the first one's frame hands the first
/// one's components back afterwards).
pub fn restore(items: Vec<Component>) {
    ITEMS.with(|i| {
        let mut list = i.borrow_mut();
        let mine = std::mem::take(&mut *list);
        *list = items;
        list.extend(mine);
    });
}

/// Runs `f` with `name` as the parent of the components it records (a settings group, a rail).
pub fn scope<R>(name: &str, f: impl FnOnce() -> R) -> R {
    if !active() {
        return f();
    }
    SCOPES.with(|s| s.borrow_mut().push(name.to_string()));
    let r = f();
    SCOPES.with(|s| {
        s.borrow_mut().pop();
    });
    r
}

/// The current parent.
fn parent() -> Option<String> {
    SCOPES.with(|s| s.borrow().last().cloned())
}

fn phys(ppp: f32, r: Rect) -> R {
    R::new(
        r.min.x * ppp,
        r.min.y * ppp,
        r.width() * ppp,
        r.height() * ppp,
    )
}

/// An sRGB token colour of an egui colour (alpha ignored; fully transparent is `None`).
fn rgb(c: Color32) -> Option<kim_design::Rgb> {
    (c.a() > 0).then(|| kim_design::Rgb {
        r: c.r(),
        g: c.g(),
        b: c.b(),
    })
}

/// What a component painted as text: its content, its box (points) and its font size (points).
#[derive(Debug, Clone)]
pub struct Painted<'a> {
    pub content: &'a str,
    pub rect: Rect,
    pub size: f32,
    pub truncated: bool,
}

/// Records one component laid out in `rect` (points) by `ui`.
#[allow(clippy::too_many_arguments)]
pub fn record(
    ui: &Ui,
    id: &str,
    kind: &str,
    rect: Rect,
    text: Option<Painted<'_>>,
    interactive: bool,
    states: &[&str],
    fg: Option<Color32>,
    bg: Option<Color32>,
) {
    if !active() {
        return;
    }
    let ppp = ui.ctx().pixels_per_point();
    let clip = ui.clip_rect();
    let mut c = Component::new(id, kind, phys(ppp, rect));
    c.parent = parent();
    c.interactive = interactive;
    c.visible = clip.intersects(rect);
    c.clipped = !clip.expand(0.5).contains_rect(rect);
    c.state = states.iter().map(|s| (*s).to_string()).collect();
    c.fg = fg.and_then(rgb);
    c.bg = bg.and_then(rgb);
    if !ui.is_enabled() {
        c.state.push("disabled".into());
        // egui paints a disabled control faded towards the surface
        // (`state::DISABLED_OPACITY`): record the colour actually drawn.
        let under =
            c.bg.unwrap_or_else(|| kim_design::Theme::palette(crate::theme_of(ui.ctx())).bg_base);
        c.fg =
            c.fg.map(|f| under.mix(f, kim_design::state::DISABLED_OPACITY));
    }
    c.text = text.map(|t| Text {
        content: t.content.to_string(),
        bounds: phys(ppp, t.rect),
        font_px: t.size * ppp,
        line_px: t.rect.height() * ppp,
        truncated: t.truncated,
        role: "ui".into(),
    });
    ITEMS.with(|i| {
        let mut items = i.borrow_mut();
        // Ids stay unique (two rows with the same title).
        let n = items
            .iter()
            .filter(|x| x.id == c.id || x.id.starts_with(&format!("{}#", c.id)))
            .count();
        if n > 0 {
            c.id = format!("{}#{n}", c.id);
        }
        items.push(c);
    });
}

/// The text of the component recorded last is drawn at the user's own text size (an editor's
/// or a terminal's font setting), not on the UI type scale: the audit's type-scale rule skips it,
/// as it does for a terminal (role `terminal`).
pub fn last_text_is_users_size() {
    if !active() {
        return;
    }
    ITEMS.with(|i| {
        if let Some(t) = i.borrow_mut().last_mut().and_then(|c| c.text.as_mut()) {
            t.role = "terminal".into();
        }
    });
}

/// A report of what was recorded this frame (the window's size and scales from `ctx`).
pub fn report(ctx: &egui::Context, application: &str, screen: &str) -> Report {
    let ppp = ctx.pixels_per_point();
    let zoom = ctx.zoom_factor();
    let content = ctx.content_rect();
    let mut components = vec![Component::new(
        format!("{application}.window"),
        "region",
        phys(ppp, content),
    )];
    components.extend(take());
    Report {
        application: application.into(),
        screen: screen.into(),
        fixture: std::env::var("KIM_UI_FIXTURE").unwrap_or_default(),
        theme: crate::theme_of(ctx).token_name().into(),
        window: (content.width() * ppp, content.height() * ppp),
        dpi_scale: ppp / zoom,
        ui_zoom: zoom,
        scale: ppp,
        components,
        ..Report::default()
    }
}

/// The end of a frame of a real program: while `KIM_UI_INSPECT` is set, writes the frame's
/// report there (once the layout has settled, frame 4 onwards), asks egui for a screenshot when
/// `KIM_UI_SNAPSHOT` names a file, and — with `KIM_UI_EXIT=1` — closes the window once both are
/// written. Without the variable it only clears (nothing was recorded).
pub fn finish_frame(ctx: &egui::Context, application: &str, screen: &str) {
    finish_frame_with(ctx, application, screen, None);
}

/// [`finish_frame`] for a window egui cannot photograph itself (a child viewport drawn
/// immediately: eframe paints it but runs no screenshot for it). `capture` writes the window's
/// real pixels (the program grabs them from the screen) and returns whether it did.
pub fn finish_frame_with(
    ctx: &egui::Context,
    application: &str,
    screen: &str,
    capture: Option<&dyn Fn(&std::path::Path) -> bool>,
) {
    let Some(path) = kim_design::inspect::requested_path() else {
        return;
    };
    let frame = FRAMES.with(|f| {
        f.set(f.get() + 1);
        f.get()
    });
    // Keep frames coming until the report and the screenshot are written.
    ctx.request_repaint();
    // Measure once the window has settled: at least 6 frames, the same size for 4 frames (a
    // program may still be growing its window for the view it opened).
    let size = ctx.content_rect().size();
    let stable = SETTLE.with(|s| {
        let (last, n) = s.get();
        let n = if (last - size).length() < 0.5 {
            n + 1
        } else {
            0
        };
        s.set((size, n));
        n
    });
    if frame < 20 || stable < 6 {
        clear();
        return;
    }
    let report = report(ctx, application, screen);
    let _ = report.write(path);
    clear();
    let shot = std::env::var_os(kim_design::inspect::ENV_SNAPSHOT);
    let exit = std::env::var("KIM_UI_EXIT").is_ok_and(|v| v == "1");
    if let (Some(shot), Some(capture)) = (&shot, capture) {
        // Two more settled frames on screen before the grab, then the grab, then the end.
        if frame > 24 && !SHOT_DONE.load(std::sync::atomic::Ordering::SeqCst) {
            if capture(std::path::Path::new(shot)) {
                SHOT_DONE.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        } else if exit && SHOT_DONE.load(std::sync::atomic::Ordering::SeqCst) {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
        return;
    }
    if let Some(shot) = shot {
        if !SHOT_ASKED.with(Cell::get) {
            SHOT_ASKED.with(|s| s.set(true));
            let path = std::path::PathBuf::from(shot);
            let c = ctx.clone();
            ctx.request_screenshot(move |img| {
                let _ = write_png(&path, &img);
                SHOT_DONE.store(true, std::sync::atomic::Ordering::SeqCst);
                c.request_repaint();
            });
        }
        if exit && SHOT_DONE.load(std::sync::atomic::Ordering::SeqCst) {
            // The whole program ends (a Settings window's audit runs in a child window).
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    } else if exit && frame > 6 {
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    }
}

/// The screenshot asked for by [`finish_frame`] was written.
static SHOT_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// ADR 0100: writes the real pixels of this process's window titled `title` (copied from the
/// screen) as a PNG — the photograph of a Settings window, which egui does not take for a child
/// viewport. `false` when there is no such window.
pub fn capture_window_png(title: &str, path: &std::path::Path) -> bool {
    let Some((w, h, rgba)) = kim_design::system::grab_window(title) else {
        return false;
    };
    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
    write_png(path, &img).is_ok()
}

/// Writes an egui screenshot as an 8-bit RGB PNG.
pub fn write_png(path: &std::path::Path, img: &egui::ColorImage) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let [w, h] = img.size;
    let mut enc = png::Encoder::new(file, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(std::io::Error::other)?;
    let data: Vec<u8> = img
        .pixels
        .iter()
        .flat_map(|p| [p.r(), p.g(), p.b()])
        .collect();
    wr.write_image_data(&data).map_err(std::io::Error::other)
}

/// The environment variable that places an audited window (`x,y` in screen points), so a UI
/// audit can open real windows away from the user's work.
pub const ENV_POS: &str = "KIM_UI_POS";

/// Where an audited window opens (`KIM_UI_POS`), if anywhere particular.
pub fn audit_position() -> Option<[f32; 2]> {
    let v = std::env::var(ENV_POS).ok()?;
    let (x, y) = v.split_once(',')?;
    Some([x.trim().parse().ok()?, y.trim().parse().ok()?])
}

/// `vb` placed at [`audit_position`] when an audit asks for it.
pub fn audit_viewport(vb: egui::ViewportBuilder) -> egui::ViewportBuilder {
    match audit_position() {
        Some(p) => vb.with_position(p),
        None => vb,
    }
}
