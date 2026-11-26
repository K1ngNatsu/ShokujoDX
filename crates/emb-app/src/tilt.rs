//! The tilt view (P18 view plan, Milestone 8, ADR 0129): the embroidery
//! from an angle, with its real heights or with the heights made larger.
//! View > Tilt View shows it on the canvas (the tools wait until it is off);
//! View > Tilt Window shows it in a window of its own, next to the canvas.
//!
//! A thread makes the surface of the part of the design that the view can
//! show from the lay of the tiles (`stitch_tiles::laid_of`), at the detail
//! of the zoom. The picture comes from it in the light of the room
//! (`Lighting::turned`: the light stays when the design turns) and from the
//! point of view (`emb_render::tilt::project`). While the view moves, the
//! picture has half the pixels on each side.
//!
//! A left drag turns (across) and tilts (up and down), a right or middle
//! drag moves the view, the wheel zooms, a double-click fits the design,
//! and L with a drag moves the light.

use crate::clock::{Duration, Instant};
use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point, math};
use emb_render::surface::{Laid, Surface};
use emb_render::tilt::{Camera, MAX_LIFT, MAX_TILT_DEG, Relief};

use crate::app::AppState;
use crate::i18n::tr;

/// The most pixels of the surface of the tilt view.
const MAX_PIXELS: f64 = 4_000_000.0;
/// The view moved this recently: the picture has half the pixels.
const MOVING: Duration = Duration::from_millis(180);

/// A surface of the tilt view: the area, its detail and the lay and fabric
/// that it is made from.
struct Made {
    key: u64,
    area: BBox,
    ppm: f64,
    surface: Arc<Surface>,
}

/// The state of the tilt view (see the module text).
pub struct TiltView {
    pub camera: Camera,
    /// The tilt view is on the canvas.
    pub on_canvas: bool,
    /// The tilt view is in its window.
    pub window: bool,
    /// The lay that the camera was fitted to.
    fitted: Option<u64>,
    made: Option<Made>,
    job: Option<(u64, mpsc::Receiver<Option<Made>>)>,
    relief: Option<(u64, Arc<Relief>)>,
    texture: Option<(u64, TextureHandle)>,
    moved: Option<Instant>,
    /// The time of the last picture (its light and its projection), ms.
    pub frame_ms: f64,
}

impl Default for TiltView {
    fn default() -> Self {
        TiltView {
            camera: Camera::default(),
            on_canvas: false,
            window: false,
            fitted: None,
            made: None,
            job: None,
            relief: None,
            texture: None,
            moved: None,
            frame_ms: 0.0,
        }
    }
}

fn hash_of(v: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// The area and the detail of the surface for the camera `cam` in a view
/// of `w` × `h` pixels: the part of the design that the view can show (any
/// turn), at the detail of the zoom (a power of two), at most
/// [`MAX_PIXELS`].
pub fn wanted(bounds: BBox, cam: &Camera, w: f64, h: f64) -> Option<(BBox, f64)> {
    let ppm_screen = cam.px_per_mm.max(1e-3);
    let mut ppm = 2f64.powf(ppm_screen.clamp(1.0, 32.0).log2().ceil());
    let (_, cv) = math::sin_cos_deg(cam.tilt_deg.clamp(0.0, MAX_TILT_DEG));
    let reach = (w.hypot(h) / 2.0) / ppm_screen / cv.max(0.2) * 1.25;
    let view = BBox::new(
        Point::new(cam.center.x - reach, cam.center.y - reach),
        Point::new(cam.center.x + reach, cam.center.y + reach),
    );
    let min = Point::new(bounds.min.x.max(view.min.x), bounds.min.y.max(view.min.y));
    let max = Point::new(bounds.max.x.min(view.max.x), bounds.max.y.min(view.max.y));
    if min.x >= max.x || min.y >= max.y {
        return None;
    }
    let area = BBox::new(min, max);
    while area.width() * area.height() * ppm * ppm > MAX_PIXELS && ppm > 0.25 {
        ppm /= 2.0;
    }
    Some((area, ppm))
}

impl TiltView {
    /// Fits the design `bounds` in a view of `size` points.
    fn fit(&mut self, bounds: BBox, size: egui::Vec2) {
        self.camera.center = bounds.center();
        let side = bounds.width().max(bounds.height()).max(1.0);
        self.camera.px_per_mm = (f64::from(size.x.min(size.y)) * 0.8 / side).max(0.05);
    }
}

/// The input, the surface that shows and the picture of the tilt view in
/// `rect`.
fn view(ui: &mut egui::Ui, st: &mut AppState, rect: Rect) {
    let ctx = ui.ctx().clone();
    let resp = ui.interact(rect, ui.id().with("tilt-view"), egui::Sense::click_and_drag());
    let back = if ui.visuals().dark_mode {
        Color32::from_gray(32)
    } else {
        Color32::from_gray(248)
    };
    ui.painter().rect_filled(rect, 0.0, back);
    let Some((lay, laid)) = crate::stitch_tiles::laid_of(&ctx, st) else {
        message(ui, rect, tr("Laying the stitches…"));
        ctx.request_repaint_after(Duration::from_millis(50));
        return;
    };
    if laid.is_empty() {
        message(ui, rect, tr("The design has no stitches."));
        return;
    }
    let bounds = laid.bounds();
    if st.tilt.fitted.is_none() {
        st.tilt.fit(bounds, rect.size());
        st.tilt.fitted = Some(lay);
    }
    // Input first, so the picture shows the camera of this frame.
    input(ui, &resp, rect, st, bounds);
    let ppp = f64::from(ctx.pixels_per_point());
    let (w, h) = (f64::from(rect.width()) * ppp, f64::from(rect.height()) * ppp);
    let cam_px = Camera {
        px_per_mm: st.tilt.camera.px_per_mm * ppp,
        ..st.tilt.camera
    };
    let d = st.doc.design();
    let ground = st.ground_of(d);
    let fabric = d
        .background
        .filter(|_| !st.fabrics.of(d).is_lace())
        .map(|bg| (bg, st.fabric_look(d)));
    let light = st.settings.look.light;
    let key = hash_of((lay, ground.as_ref().map(emb_render::fabric::Ground::key)));
    request_surface(st, &ctx, &laid, key, &cam_px, w, h, ground, light);
    // The surface before shows until the new one is there.
    let Some(made) = st.tilt.made.as_ref() else {
        message(ui, rect, tr("Making the surface…"));
        ctx.request_repaint_after(Duration::from_millis(50));
        return;
    };
    // The light of the room on the turned design.
    let t0 = Instant::now();
    let turned = light.turned(st.tilt.camera.turn_deg);
    let rkey = hash_of((
        made.key,
        made.area.min.x.to_bits(),
        made.area.min.y.to_bits(),
        made.ppm.to_bits(),
        turned.azimuth_deg.to_bits(),
        turned.elevation_deg.to_bits(),
        turned.strength.to_bits(),
        turned.lights_off,
        turned.two,
        format!("{fabric:?}"),
    ));
    if st.tilt.relief.as_ref().is_none_or(|(k, _)| *k != rkey) {
        let r = Relief::of(&made.surface, &turned, fabric);
        st.tilt.relief = Some((rkey, Arc::new(r)));
    }
    let Some((_, relief)) = st.tilt.relief.clone() else {
        return;
    };
    let moving = st.tilt.moved.is_some_and(|t| t.elapsed() < MOVING);
    if moving {
        ctx.request_repaint_after(MOVING);
    }
    let f = if moving { 0.5 } else { 1.0 };
    let (pw, ph) = ((w * f).round().max(1.0) as u32, (h * f).round().max(1.0) as u32);
    let cam = Camera {
        px_per_mm: cam_px.px_per_mm * f,
        ..cam_px
    };
    let pkey = hash_of((
        rkey,
        cam.center.x.to_bits(),
        cam.center.y.to_bits(),
        cam.px_per_mm.to_bits(),
        cam.turn_deg.to_bits(),
        cam.tilt_deg.to_bits(),
        cam.lift.to_bits(),
        pw,
        ph,
        back,
    ));
    if st.tilt.texture.as_ref().is_none_or(|(k, _)| *k != pkey) {
        let pic = emb_render::tilt::project(&relief, &cam, pw, ph, back.to_array());
        let image = egui::ColorImage::from_rgba_unmultiplied([pw as usize, ph as usize], &pic.rgba);
        match st.tilt.texture.as_mut() {
            Some((k, t)) => {
                t.set(image, TextureOptions::LINEAR);
                *k = pkey;
            }
            None => {
                let t = crate::resources::load_texture(&ctx, "tilt-view", image, TextureOptions::LINEAR);
                st.tilt.texture = Some((pkey, t));
            }
        }
        st.tilt.frame_ms = t0.elapsed().as_secs_f64() * 1000.0;
    }
    if let Some((_, t)) = &st.tilt.texture {
        ui.painter().image(
            t.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    if st.tilt.job.is_some() {
        message(ui, rect, tr("Making the surface…"));
    }
    crate::viewer::draw_light(ui.painter(), &ctx, rect, st, resp.hovered());
}

/// Starts a thread that makes the surface that the camera needs, if the
/// surface that shows does not have it.
#[allow(clippy::too_many_arguments)]
fn request_surface(
    st: &mut AppState,
    ctx: &egui::Context,
    laid: &Arc<Laid>,
    key: u64,
    cam: &Camera,
    w: f64,
    h: f64,
    ground: Option<emb_render::fabric::Ground>,
    light: emb_render::thread3d::Lighting,
) {
    let t = &mut st.tilt;
    if let Some((_, rx)) = &t.job {
        match rx.try_recv() {
            Ok(m) => {
                if m.is_some() {
                    t.made = m;
                }
                t.job = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => t.job = None,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    let Some((area, ppm)) = wanted(laid.bounds(), cam, w, h) else {
        return;
    };
    let fits = t.made.as_ref().is_some_and(|m| {
        m.key == key
            && m.ppm == ppm
            && m.area.min.x <= area.min.x + 1e-6
            && m.area.min.y <= area.min.y + 1e-6
            && m.area.max.x >= area.max.x - 1e-6
            && m.area.max.y >= area.max.y - 1e-6
    });
    let jkey = hash_of((key, ppm.to_bits(), area.min.x.to_bits(), area.min.y.to_bits()));
    if fits || t.job.as_ref().is_some_and(|(k, _)| *k == jkey) {
        return;
    }
    // A margin, so a small move does not need a new surface.
    let bounds = laid.bounds();
    let grow = (area.width().max(area.height()) * 0.15).max(2.0);
    let area = BBox::new(
        Point::new(
            (area.min.x - grow).max(bounds.min.x),
            (area.min.y - grow).max(bounds.min.y),
        ),
        Point::new(
            (area.max.x + grow).min(bounds.max.x),
            (area.max.y + grow).min(bounds.max.y),
        ),
    );
    let laid = laid.clone();
    let (tx, rx) = mpsc::channel();
    let repaint = ctx.clone();
    let work = move || {
        let s = laid.surface_on(area, ppm, &light, ground.as_ref());
        let _ = tx.send(Some(Made {
            key,
            area,
            ppm,
            surface: Arc::new(s),
        }));
        repaint.request_repaint();
    };
    if st.stitch_tiles.sync {
        work();
    } else if std::thread::Builder::new()
        .name("tilt-surface".into())
        .stack_size(crate::resources::JOB_STACK)
        .spawn(work)
        .is_err()
    {
        return;
    }
    st.tilt.job = Some((jkey, rx));
    if st.stitch_tiles.sync
        && let Some((_, rx)) = st.tilt.job.take()
        && let Ok(m) = rx.try_recv()
    {
        st.tilt.made = m;
    }
}

/// The input of the tilt view (see the module text).
fn input(ui: &egui::Ui, resp: &egui::Response, rect: Rect, st: &mut AppState, bounds: BBox) {
    if crate::viewer::light_drag(ui, resp, rect, st) {
        return;
    }
    let cam = &mut st.tilt.camera;
    let mut moved = false;
    if resp.dragged_by(egui::PointerButton::Primary) {
        let d = resp.drag_delta();
        cam.turn_deg = (cam.turn_deg - f64::from(d.x) * 0.4 + 180.0).rem_euclid(360.0) - 180.0;
        cam.tilt_deg = (cam.tilt_deg + f64::from(d.y) * 0.3).clamp(0.0, MAX_TILT_DEG);
        moved = true;
    }
    if resp.dragged_by(egui::PointerButton::Middle) || resp.dragged_by(egui::PointerButton::Secondary) {
        // The move on the screen, back on the turned and tilted design.
        let d = resp.drag_delta();
        let (st_, ct) = math::sin_cos_deg(cam.turn_deg);
        let (_, cv) = math::sin_cos_deg(cam.tilt_deg);
        let (sx, sy) = (
            f64::from(d.x) / cam.px_per_mm,
            f64::from(d.y) / cam.px_per_mm / cv.max(0.2),
        );
        cam.center.x -= sx * ct - sy * st_;
        cam.center.y -= sx * st_ + sy * ct;
        moved = true;
    }
    if resp.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let mut factor = f64::from(pinch);
        if scroll != 0.0 {
            factor *= (f64::from(scroll) * 0.0015).exp();
        }
        if factor != 1.0 {
            cam.px_per_mm = (cam.px_per_mm * factor).clamp(0.05, 200.0);
            moved = true;
        }
    }
    if resp.double_clicked() {
        st.tilt.fit(bounds, rect.size());
        moved = true;
    }
    if moved {
        st.tilt.moved = Some(Instant::now());
    }
}

/// A line of text in the middle of the view.
fn message(ui: &egui::Ui, rect: Rect, text: &str) {
    let p = ui.painter();
    let g = p.layout_no_wrap(text.into(), egui::FontId::proportional(15.0), Color32::WHITE);
    let r = Rect::from_center_size(rect.center(), g.size());
    p.rect_filled(r.expand(6.0), 4.0, Color32::from_black_alpha(160));
    p.galley(r.min, g, Color32::WHITE);
}

/// The controls of the tilt view: tilt, turn, heights, fit and the light.
fn controls(ui: &mut egui::Ui, st: &mut AppState, in_window: bool) {
    ui.horizontal_wrapped(|ui| {
        let cam = &mut st.tilt.camera;
        let mut moved = false;
        ui.label(tr("Tilt"));
        moved |= ui
            .add(
                crate::workspace::slider(&mut cam.tilt_deg, 0.0..=MAX_TILT_DEG)
                    .suffix("°")
                    .fixed_decimals(0),
            )
            .on_hover_text(tr("0°: from straight above. Drag up and down on the view to tilt it."))
            .changed();
        ui.label(tr("Turn"));
        moved |= ui
            .add(
                crate::workspace::slider(&mut cam.turn_deg, -180.0..=180.0)
                    .suffix("°")
                    .fixed_decimals(0),
            )
            .on_hover_text(tr(
                "The design turns under the light, as a hoop that you turn under a lamp. Drag across the view to turn it.",
            ))
            .changed();
        ui.label(tr("Heights"));
        moved |= ui
            .add(
                crate::workspace::slider(&mut cam.lift, 1.0..=MAX_LIFT)
                    .prefix("×")
                    .fixed_decimals(1),
            )
            .on_hover_text(tr(
                "×1 shows the real heights of the thread; more makes them larger, so each height and each gap shows.",
            ))
            .changed();
        if moved {
            st.tilt.moved = Some(Instant::now());
        }
        if ui
            .button(tr("Fit"))
            .on_hover_text(tr("Show all of the design (also a double-click on the view)."))
            .clicked()
        {
            st.tilt.fitted = None;
        }
        let light = ui.button(tr("Lighting"));
        egui::Popup::menu(&light).show(|ui| {
            let mut s = st.settings.clone();
            crate::viewer::light_ui(ui, &mut s.look.light);
            st.set_settings(s);
        });
        if in_window {
            if ui.button(tr("On the Canvas")).clicked() {
                st.tilt.window = false;
                st.tilt.on_canvas = true;
            }
        } else {
            if ui.button(tr("In a Window")).clicked() {
                st.tilt.on_canvas = false;
                st.tilt.window = true;
            }
            if ui
                .button(tr("Close"))
                .on_hover_text(tr("Back to the canvas and its tools."))
                .clicked()
            {
                st.tilt.on_canvas = false;
            }
        }
    });
}

/// The tilt view on the canvas (View > Tilt View).
pub fn canvas(ui: &mut egui::Ui, st: &mut AppState) {
    controls(ui, st, false);
    let rect = ui.available_rect_before_wrap();
    ui.allocate_rect(rect, egui::Sense::hover());
    st.canvas_rect = Some(rect);
    view(ui, st, rect);
}

/// The window of the tilt view (View > Tilt Window).
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.tilt.window {
        return;
    }
    let mut open = true;
    crate::dock::docked(
        egui::Window::new(tr("Tilt View")).id(egui::Id::new("Tilt View")),
        ctx,
        egui::Id::new("Tilt View"),
        st.canvas_rect,
    )
    .open(&mut open)
    .default_size(egui::vec2(640.0, 480.0))
    .resizable(true)
    .show(ctx, |ui| {
        controls(ui, st, true);
        let rect = ui.available_rect_before_wrap();
        let rect = Rect::from_min_size(rect.min, rect.size().max(egui::vec2(320.0, 240.0)));
        ui.allocate_rect(rect, egui::Sense::hover());
        view(ui, st, rect);
    });
    if !open {
        st.tilt.window = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_surface_covers_what_the_view_can_show() {
        let b = BBox::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
        let cam = Camera {
            center: Point::ZERO,
            px_per_mm: 4.0,
            ..Camera::default()
        };
        // All of a small design, at the detail of the zoom.
        let (area, ppm) = wanted(b, &cam, 1000.0, 800.0).unwrap();
        assert_eq!(area, b);
        assert_eq!(ppm, 4.0);
        // Near: a part of it, with more detail.
        let near = Camera { px_per_mm: 40.0, ..cam };
        let (area, ppm) = wanted(b, &near, 1000.0, 800.0).unwrap();
        assert!(area.width() < 100.0 && ppm >= 16.0, "{area:?} {ppm}");
        assert!(area.width() * area.height() * ppm * ppm <= MAX_PIXELS);
        // Away from the design: nothing.
        let away = Camera {
            center: Point::new(500.0, 0.0),
            ..cam
        };
        assert!(wanted(b, &away, 1000.0, 800.0).is_none());
    }

    /// The exit test of Milestone 8 (speed): the tilt view of a design of
    /// 100,000 stitches moves at 20 frames in a second or more on the
    /// development computer. Run with
    /// `cargo test --release -p emb-app -- --ignored tilt_budget --nocapture`.
    #[test]
    #[ignore]
    fn tilt_budget() {
        let plan = crate::stitch_tiles::stress_plan(100_000);
        let strands = emb_render::strand::strands(&plan.stitches, &Default::default());
        let t0 = Instant::now();
        let laid = emb_render::surface::lay(&strands, &[]);
        let lay_ms = t0.elapsed().as_secs_f64() * 1000.0;
        // A view of 1280 × 800 pixels that shows all of the design.
        let (w, h) = (1280.0, 800.0);
        let b = laid.bounds();
        let mut cam = Camera {
            center: b.center(),
            px_per_mm: h * 0.8 / b.width().max(b.height()),
            ..Camera::default()
        };
        let (area, ppm) = wanted(b, &cam, w, h).expect("the design shows");
        let light = emb_render::thread3d::Lighting::default();
        let t0 = Instant::now();
        let s = laid.surface_on(area, ppm, &light, None);
        let surface_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let fabric = Some((
            emb_model::Rgb(235, 235, 230),
            emb_render::fabric::FabricLook::new(emb_render::fabric::Weave::Twill),
        ));
        // Moving: a new turn in each frame (the light of the room on the
        // turned design, then the picture with half the pixels).
        let frames = 30;
        let t0 = Instant::now();
        for k in 0..frames {
            cam.turn_deg = 3.0 * f64::from(k);
            cam.tilt_deg = 30.0 + f64::from(k);
            let relief = Relief::of(&s, &light.turned(cam.turn_deg), fabric);
            let half = Camera {
                px_per_mm: cam.px_per_mm * 0.5,
                ..cam
            };
            let _ = emb_render::tilt::project(&relief, &half, (w / 2.0) as u32, (h / 2.0) as u32, [32, 32, 32, 255]);
        }
        let turn_ms = t0.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        // Tilt only (the same light): the picture alone.
        let relief = Relief::of(&s, &light, fabric);
        let t0 = Instant::now();
        for k in 0..frames {
            cam.tilt_deg = 20.0 + 1.5 * f64::from(k);
            let half = Camera {
                px_per_mm: cam.px_per_mm * 0.5,
                ..cam
            };
            let _ = emb_render::tilt::project(&relief, &half, (w / 2.0) as u32, (h / 2.0) as u32, [32, 32, 32, 255]);
        }
        let tilt_ms = t0.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        // Still: the picture with all of its pixels.
        let t0 = Instant::now();
        let _ = emb_render::tilt::project(&relief, &cam, w as u32, h as u32, [32, 32, 32, 255]);
        let still_ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!(
            "tilt view, 100,000 stitches: lay {lay_ms:.0} ms; surface {} x {} px at {ppm} px/mm {surface_ms:.0} ms; turn {turn_ms:.1} ms/frame ({:.0} fps); tilt {tilt_ms:.1} ms/frame ({:.0} fps); still {still_ms:.1} ms",
            s.width,
            s.height,
            1000.0 / turn_ms,
            1000.0 / tilt_ms,
        );
        assert!(turn_ms < 50.0 && tilt_ms < 50.0, "{turn_ms} {tilt_ms}");
    }

    #[test]
    fn the_tilt_view_shows_the_design_and_turns_it() {
        let mut h = Harness::new();
        h.st.stitch_tiles.sync = true;
        h.st.tilt.on_canvas = true;
        let frame = |h: &mut Harness| {
            let st = &mut h.st;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
                ..Default::default()
            };
            let mut out = h.ctx.run_ui(input, |ui| {
                egui::CentralPanel::no_frame().show(ui, |ui| canvas(ui, st));
            });
            out.textures_delta.clear();
        };
        frame(&mut h);
        frame(&mut h);
        assert!(h.st.tilt.texture.is_some(), "a picture");
        let first = h.st.tilt.texture.as_ref().map(|(k, _)| *k);
        h.st.tilt.camera.turn_deg = 90.0;
        frame(&mut h);
        assert_ne!(h.st.tilt.texture.as_ref().map(|(k, _)| *k), first);
        // The window shows the same view.
        h.st.tilt.on_canvas = false;
        h.st.tilt.window = true;
        let st = &mut h.st;
        let mut out = h
            .ctx
            .run_ui(egui::RawInput::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
    }
}
