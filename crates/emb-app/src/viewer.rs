//! The viewer of the 3D look (P18 view plan, Milestone 8, ADR 0129):
//!
//! - **The light by hand**: hold L and drag on a view; the light comes from
//!   the place of the pointer (from straight above in the middle of the
//!   view to low at the edge). The light presets are in the Thread Look
//!   menu and on the View tab.
//! - **The magnifier**: a round window at the pointer with the thread at
//!   20 times its real size (the surface of the lay of the tiles).
//! - **Before and after**: the canvas keeps the stitches of the design as
//!   they were; a line that the user drags shows them on its left and the
//!   design now on its right.
//! - **Compare 3D**: the stitch view on the canvas and the 3D look in the
//!   split view, at the same place and zoom.

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point, math};
use emb_model::Design;
use emb_render::thread3d::{LightPreset, Lighting};
use emb_stitch::StitchPlan;

use crate::app::AppState;
use crate::canvas::CanvasView;
use crate::i18n::tr;

/// The key that moves the light while it is held.
pub const LIGHT_KEY: egui::Key = egui::Key::L;
/// The lowest light that a drag gives (at the edge of the view), degrees.
const LOWEST_DEG: f64 = 10.0;
/// The magnifier shows the thread this many times larger than its real size.
pub const MAGNIFY: f64 = 20.0;
/// The size of the magnifier, points.
const LENS_POINTS: f32 = 220.0;

/// The light that comes from the place `pos` of the view `rect`: its
/// direction from the middle of the view, and from straight above in the
/// middle to [`LOWEST_DEG`] at the edge of the largest circle in the view.
pub fn light_at(rect: Rect, pos: Pos2, l: &Lighting) -> Lighting {
    let d = pos - rect.center();
    let radius = f64::from(rect.width().min(rect.height()) / 2.0).max(1.0);
    let r = (f64::from(d.length()) / radius).min(1.0);
    let azimuth = math::atan2(f64::from(-d.y), f64::from(d.x))
        .to_degrees()
        .rem_euclid(360.0);
    Lighting {
        azimuth_deg: if r > 1e-3 { azimuth.round() } else { l.azimuth_deg },
        elevation_deg: (90.0 - (90.0 - LOWEST_DEG) * r).round(),
        ..*l
    }
}

/// The place in the view `rect` of the light `l` (see [`light_at`]).
fn light_place(rect: Rect, l: &Lighting) -> Pos2 {
    let radius = rect.width().min(rect.height()) / 2.0;
    let r = ((90.0 - l.elevation_deg) / (90.0 - LOWEST_DEG)).clamp(0.0, 1.0) as f32 * radius;
    let (s, c) = math::sin_cos_deg(l.azimuth_deg);
    rect.center() + egui::vec2(c as f32 * r, -s as f32 * r)
}

/// `true` while the light key is held and no text field takes the keys.
pub fn light_key_held(ctx: &egui::Context) -> bool {
    !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_down(LIGHT_KEY))
}

/// The light by hand: while L is held over the view `rect`, a drag (or a
/// click) puts the light at the pointer. `true` if it takes the pointer.
pub fn light_drag(ui: &egui::Ui, response: &egui::Response, rect: Rect, st: &mut AppState) -> bool {
    if !response.hovered() && !response.dragged() || !light_key_held(ui.ctx()) {
        return false;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    if (response.dragged_by(egui::PointerButton::Primary) || response.clicked())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let mut s = st.settings.clone();
        s.look.light = light_at(rect, pos, &s.look.light);
        let l = s.look.light;
        st.set_settings(s);
        st.message = crate::i18n::fill(
            "Light from {}°, {}° high.",
            &[&format!("{:.0}", l.azimuth_deg), &format!("{:.0}", l.elevation_deg)],
        );
    }
    true
}

/// While L is held over the view: the circle of the light places and the
/// light (a sun).
pub fn draw_light(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &AppState, hovered: bool) {
    if !hovered || !light_key_held(ctx) {
        return;
    }
    let radius = rect.width().min(rect.height()) / 2.0;
    let guide = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 90, 160));
    painter.circle_stroke(rect.center(), radius, guide);
    painter.circle_stroke(rect.center(), radius / 2.0, guide);
    let at = light_place(rect, &st.settings.look.light);
    painter.line_segment([rect.center(), at], guide);
    painter.circle_filled(at, 9.0, Color32::from_rgb(255, 210, 60));
    painter.circle_stroke(at, 9.0, Stroke::new(1.5, Color32::from_rgb(120, 80, 0)));
    for k in 0..8 {
        let (s, c) = math::sin_cos_deg(45.0 * f64::from(k));
        let u = egui::vec2(c as f32, s as f32);
        painter.line_segment(
            [at + u * 12.0, at + u * 17.0],
            Stroke::new(2.0, Color32::from_rgb(255, 210, 60)),
        );
    }
}

/// The light presets and the light (the Thread Look menu, the View tab).
pub fn light_ui(ui: &mut egui::Ui, l: &mut Lighting) {
    ui.horizontal_wrapped(|ui| {
        for p in LightPreset::ALL {
            if ui
                .selectable_label(LightPreset::of(l) == Some(p), tr(p.label()))
                .clicked()
            {
                *l = p.apply(l);
            }
        }
    });
    egui::Grid::new("light").num_columns(2).show(ui, |ui| {
        ui.label(tr("Light from"));
        ui.add(
            crate::workspace::slider(&mut l.azimuth_deg, 0.0..=360.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix("°")
                .fixed_decimals(0),
        )
        .on_hover_text(tr(
            "The direction of the light: 0° from the right, 90° from the top, 135° from the top left.",
        ));
        ui.end_row();
        ui.label(tr("Light height"));
        ui.add(
            crate::workspace::slider(&mut l.elevation_deg, 10.0..=90.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix("°")
                .fixed_decimals(0),
        )
        .on_hover_text(tr(
            "Low light gives long shadows and strong shade; 90° is straight above.",
        ));
        ui.end_row();
        ui.label(tr("Strength"));
        ui.add(
            crate::workspace::slider(&mut l.strength, 0.0..=1.0)
                .clamping(egui::SliderClamping::Edits)
                .fixed_decimals(2),
        );
        ui.end_row();
    });
    ui.checkbox(&mut l.two, tr("Two lights")).on_hover_text(tr(
        "A second, weaker light from the opposite side (as in a photo studio).",
    ));
    ui.label(egui::RichText::new(tr("Hold L and drag on the design to move the light.")).small());
}

/// The magnifier (see the module text).
#[derive(Default)]
pub struct Magnifier {
    pub on: bool,
    /// The picture of the lens, for its place, lay, light and size.
    texture: Option<(u64, TextureHandle)>,
}

/// Draws the magnifier at the pointer `pos` over the canvas `rect`.
pub fn draw_magnifier(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, pos: Pos2, st: &mut AppState) {
    if !st.magnifier.on || !rect.contains(pos) {
        return;
    }
    let Some((lay, laid)) = crate::stitch_tiles::laid_of(ctx, st) else {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        return;
    };
    let ppp = f64::from(ctx.pixels_per_point());
    let ppm = MAGNIFY * crate::canvas::points_per_mm() * ppp;
    let side_px = (f64::from(LENS_POINTS) * ppp).round().max(16.0);
    let side_mm = side_px / ppm;
    let center = st.view.to_mm(rect, pos);
    // The lens moves in steps of a pixel of the lens.
    let snap = |v: f64| (v * ppm).round() / ppm;
    let area = BBox::new(
        Point::new(snap(center.x - side_mm / 2.0), snap(center.y - side_mm / 2.0)),
        Point::new(
            snap(center.x - side_mm / 2.0) + side_mm,
            snap(center.y - side_mm / 2.0) + side_mm,
        ),
    );
    let d = st.doc.design();
    let ground = st.ground_of(d);
    let fabric = d
        .background
        .filter(|_| !st.fabrics.of(d).is_lace())
        .map(|bg| (bg, st.fabric_look(d)));
    let light = st.settings.look.light;
    let back = if st.settings.dark_mode {
        [32, 32, 32, 255]
    } else {
        [248, 248, 248, 255]
    };
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            lay,
            area.min.x.to_bits(),
            area.min.y.to_bits(),
            side_px.to_bits(),
            light.azimuth_deg.to_bits(),
            light.elevation_deg.to_bits(),
            light.strength.to_bits(),
            light.lights_off,
            light.two,
            format!("{fabric:?}"),
            ground.as_ref().map(emb_render::fabric::Ground::key),
            back,
        )
            .hash(&mut h);
        h.finish()
    };
    if st.magnifier.texture.as_ref().is_none_or(|(k, _)| *k != key) {
        let s = laid.surface_on(area, ppm, &light, ground.as_ref());
        let pic = emb_render::sheen::light(&s, &light, fabric);
        let (w, h) = (pic.width as usize, pic.height as usize);
        let mut rgba = pic.rgba;
        // Round: clear outside the circle, the background where nothing is.
        let r = w.min(h) as f64 / 2.0;
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let dist = ((x as f64 + 0.5 - r).powi(2) + (y as f64 + 0.5 - r).powi(2)).sqrt();
                let edge = (r - dist).clamp(0.0, 1.0);
                let a = f64::from(rgba[i + 3]) / 255.0;
                for k in 0..3 {
                    let v = f64::from(rgba[i + k]) * a + f64::from(back[k]) * (1.0 - a);
                    rgba[i + k] = v.round() as u8;
                }
                rgba[i + 3] = (edge * 255.0).round() as u8;
            }
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        let t = crate::resources::load_texture(ctx, "magnifier", image, TextureOptions::LINEAR);
        st.magnifier.texture = Some((key, t));
    }
    let Some((_, tex)) = &st.magnifier.texture else { return };
    let size = egui::vec2(LENS_POINTS, LENS_POINTS);
    let lens = Rect::from_center_size(pos, size);
    painter.image(
        tex.id(),
        lens,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.circle_stroke(pos, LENS_POINTS / 2.0, Stroke::new(3.0, Color32::from_gray(60)));
    painter.circle_stroke(pos, LENS_POINTS / 2.0 + 1.5, Stroke::new(1.0, Color32::WHITE));
    painter.text(
        lens.center_bottom() - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        format!("×{MAGNIFY:.0}"),
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );
}

/// The design as it was when Before and After started.
pub struct BeforeAfter {
    /// The id of the design before in the tile cache (not the id of an
    /// open design).
    doc: u64,
    state: u64,
    design: Arc<Design>,
    plan: Arc<StitchPlan>,
    segments: Arc<Vec<(emb_render::Segment, Color32)>>,
    /// The place of the line, as a part of the width of the canvas.
    pub split: f32,
}

impl AppState {
    /// Before and After on or off: on, the canvas keeps the stitches of the
    /// design as they are now.
    pub fn toggle_before_after(&mut self) {
        if let Some(b) = self.before_after.take() {
            self.stitch_tiles.forget(b.doc);
            return;
        }
        let pv = self.shown_plan();
        let (state, plan, segments) = (pv.state, pv.plan.clone(), pv.segments.clone());
        self.before_after = Some(BeforeAfter {
            doc: crate::tabs::fresh_doc_id(),
            state,
            design: Arc::new(self.doc.design().clone()),
            plan,
            segments,
            split: 0.5,
        });
        self.message = tr("Before and after: change a setting, then drag the line on the canvas to compare.").into();
    }

    /// Compare 3D: the stitch view on the canvas and the 3D look in the
    /// split view, at the same place and zoom.
    pub fn compare_3d(&mut self) {
        let mut s = self.settings.clone();
        s.look.three_d = true;
        s.look.surface = true;
        self.set_settings(s);
        self.show_plan = true;
        self.show_thread_look = false;
        self.show_split = true;
        self.split.thread_look = true;
        self.split.follow = true;
        self.split.view = self.view;
    }
}

/// The place of the line of Before and After on the canvas `rect`, if it
/// is on: the stitches of now show on its right.
pub fn before_after_line(rect: Rect, st: &AppState) -> Option<f32> {
    st.before_after.as_ref().map(|b| rect.left() + rect.width() * b.split)
}

/// Draws the design before on the left of the line of Before and After
/// (the canvas `rect` shows the stitches of now on its right), the line,
/// and the labels. The line follows a drag of its handle.
pub fn draw_before_after(ui: &mut egui::Ui, painter: &egui::Painter, rect: Rect, st: &mut AppState) {
    let Some(x) = before_after_line(rect, st) else {
        return;
    };
    let left = Rect::from_min_max(rect.min, Pos2::new(x, rect.max.y));
    let clip = painter.with_clip_rect(left.intersect(painter.clip_rect()));
    let view = st.view;
    let look = if !st.show_thread_look {
        crate::canvas::MeshLook::Stitches
    } else if st.settings.look.three_d {
        crate::canvas::MeshLook::Thread3d
    } else {
        crate::canvas::MeshLook::Thread
    };
    let table = emb_render::thread3d::ShadeTable::new(st.settings.look.light);
    let Some(b) = st.before_after.as_ref() else { return };
    let (doc, state, design, plan, segments) = (b.doc, b.state, b.design.clone(), b.plan.clone(), b.segments.clone());
    let mut drawn = false;
    if look == crate::canvas::MeshLook::Thread3d && st.settings.look.surface {
        let ground = st.ground_of(&design);
        let filter = crate::viewing::ViewFilter::default();
        let scene = crate::stitch_tiles::Scene {
            doc,
            state,
            plan: &plan,
            design: &design,
            lighting: st.settings.look.light,
            ground,
            filter: &filter,
            cursor: None,
            moving: &[],
            lead: false,
        };
        if let Some(frame) = st.stitch_tiles.draw(ui.ctx(), rect, &view, &scene) {
            if let Some(missing) = frame.missing.iter().copied().reduce(|a, b| a.union(b)) {
                let mesh = crate::canvas::fitted_mesh(rect, &view, &segments, false, look, &table).0;
                clip.with_clip_rect(missing.intersect(clip.clip_rect()))
                    .add(Shape::mesh(mesh));
            }
            clip.extend(frame.shapes);
            drawn = true;
        }
    }
    if !drawn {
        let mesh = crate::canvas::fitted_mesh(rect, &view, &segments, false, look, &table).0;
        clip.add(Shape::mesh(mesh));
    }
    // The line and its handle.
    let accent = Color32::from_rgb(255, 200, 40);
    painter.line_segment(
        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
        Stroke::new(2.0, accent),
    );
    let handle = Rect::from_center_size(Pos2::new(x, rect.center().y), egui::vec2(18.0, 44.0));
    let id = ui.id().with("before_after");
    let r = ui.interact(handle, id, egui::Sense::drag());
    if r.dragged()
        && let Some(p) = r.interact_pointer_pos()
        && let Some(b) = st.before_after.as_mut()
    {
        b.split = ((p.x - rect.left()) / rect.width()).clamp(0.02, 0.98);
    }
    if r.hovered() || r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    painter.rect_filled(handle, 6.0, accent);
    painter.text(
        handle.center(),
        egui::Align2::CENTER_CENTER,
        "⇔",
        egui::FontId::proportional(14.0),
        Color32::BLACK,
    );
    for (text, at, align) in [
        (
            tr("Before"),
            Pos2::new(x - 8.0, rect.top() + 8.0),
            egui::Align2::RIGHT_TOP,
        ),
        (
            tr("After"),
            Pos2::new(x + 8.0, rect.top() + 8.0),
            egui::Align2::LEFT_TOP,
        ),
    ] {
        let g = painter.layout_no_wrap(text.into(), egui::FontId::proportional(14.0), Color32::WHITE);
        let r = align.anchor_size(at, g.size());
        painter.rect_filled(r.expand(4.0), 4.0, Color32::from_black_alpha(170));
        painter.galley(r.min, g, Color32::WHITE);
    }
}

/// The split view (and the views of View > New view) at the place and the
/// zoom of the canvas: `follow` on.
pub fn same_place(view: &mut CanvasView, main: &CanvasView) {
    *view = *main;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_light_comes_from_the_place_of_the_pointer() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let l = Lighting::default();
        // The middle: straight above.
        assert_eq!(light_at(rect, rect.center(), &l).elevation_deg, 90.0);
        // At the left edge of the circle: from the left (180°), low.
        let left = light_at(rect, Pos2::new(50.0, 150.0), &l);
        assert_eq!((left.azimuth_deg, left.elevation_deg), (180.0, LOWEST_DEG));
        // Half way to the top: from the top (90°), half way down.
        let top = light_at(rect, Pos2::new(200.0, 75.0), &l);
        assert_eq!((top.azimuth_deg, top.elevation_deg), (90.0, 50.0));
        // The place of a light is where it came from.
        let p = light_place(rect, &top);
        assert!((p.x - 200.0).abs() < 0.5 && (p.y - 75.0).abs() < 0.5, "{p:?}");
    }

    #[test]
    fn hold_l_and_drag_moves_the_light() {
        let mut h = Harness::new();
        h.frame(Vec::new());
        let rect = h.st.canvas_rect.expect("the canvas");
        let at = Pos2::new(rect.left() + 10.0, rect.center().y);
        let state = h.st.doc.state_id();
        h.frame(vec![egui::Event::Key {
            key: LIGHT_KEY,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        h.drag(egui::PointerButton::Primary, rect.center(), at, egui::Modifiers::NONE);
        let l = h.st.settings.look.light;
        assert_eq!(l.azimuth_deg, 180.0, "{l:?}");
        assert!(l.elevation_deg < 30.0, "{l:?}");
        // The design did not change (the drag was for the light).
        assert_eq!(h.st.doc.state_id(), state);
    }

    #[test]
    fn before_and_after_keeps_the_design_before() {
        let mut h = Harness::new();
        h.st.settings.look.surface = true;
        h.st.stitch_tiles.sync = true;
        h.st.show_thread_look = true;
        h.frame(Vec::new());
        h.st.toggle_before_after();
        let before = h.st.before_after.as_ref().map(|b| b.state);
        h.st.selection.set(h.id(0));
        h.st.delete_selected();
        h.frame(Vec::new());
        h.frame(Vec::new());
        // The design changed; the stitches before stay.
        assert_ne!(h.st.shown_plan().state, before.unwrap());
        assert_eq!(h.st.before_after.as_ref().map(|b| b.state), before);
        h.st.toggle_before_after();
        assert!(h.st.before_after.is_none());
    }

    #[test]
    fn compare_3d_shows_the_two_looks_at_one_place() {
        let mut h = Harness::new();
        h.st.view.zoom = 3.0;
        h.st.compare_3d();
        assert!(h.st.show_split && h.st.split.follow && h.st.split.thread_look);
        assert!(!h.st.show_thread_look && h.st.show_plan);
        h.frame(Vec::new());
        h.frame(Vec::new());
        assert_eq!(h.st.split.view, h.st.view);
    }

    #[test]
    fn the_magnifier_shows_the_thread_at_the_pointer() {
        let mut h = Harness::new();
        h.st.stitch_tiles.sync = true;
        h.st.magnifier.on = true;
        h.frame(Vec::new());
        let rect = h.st.canvas_rect.expect("the canvas");
        h.frame(vec![egui::Event::PointerMoved(rect.center())]);
        h.frame(vec![egui::Event::PointerMoved(rect.center())]);
        assert!(h.st.magnifier.texture.is_some());
    }
}
