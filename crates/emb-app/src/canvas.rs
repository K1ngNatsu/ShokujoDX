//! The design canvas: pan, zoom, grid, hoop, artwork and stitches.

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use emb_geom::{BBox, Path, Point, Transform};
use emb_model::{ObjectId, Rgb, ThreadKind};
use emb_render::{Segment, SegmentKind};

use crate::app::AppState;
use crate::commands;
use crate::draw::{self, Tool};
use crate::i18n::tr;
use crate::manipulate::{self, ViewMap};

/// Screen points per mm at 100 % zoom (96 points per inch).
/// Screen points per mm at 100 % zoom, before the calibration.
pub const POINTS_PER_MM: f64 = 96.0 / 25.4;

thread_local! {
    /// The screen calibration of the user (Options), for this UI thread.
    static CALIBRATION: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Sets the screen calibration: 100 % zoom shows `factor` × the operating
/// system size.
pub fn set_calibration(factor: f64) {
    CALIBRATION.with(|c| c.set(factor.clamp(0.5, 2.0)));
}

/// Screen points for each mm at 100 % (the real size).
pub fn points_per_mm() -> f64 {
    POINTS_PER_MM * CALIBRATION.with(std::cell::Cell::get)
}
pub const MIN_ZOOM: f64 = 0.1;
pub const MAX_ZOOM: f64 = 32.0;
/// Visual thread width in mm for the stitch view.
const THREAD_WIDTH_MM: f64 = 0.35;

/// The view transform of the canvas. It is not part of the undo history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasView {
    /// The design point (mm) at the center of the canvas.
    pub center: Point,
    /// 1.0 is 100 %.
    pub zoom: f64,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            center: Point::ZERO,
            zoom: 1.0,
        }
    }
}

impl CanvasView {
    /// Screen points per mm.
    pub fn scale(self) -> f64 {
        self.zoom * points_per_mm()
    }

    pub fn to_screen(self, rect: Rect, p: Point) -> Pos2 {
        let s = self.scale();
        let c = rect.center();
        Pos2::new(
            (f64::from(c.x) + (p.x - self.center.x) * s) as f32,
            (f64::from(c.y) + (p.y - self.center.y) * s) as f32,
        )
    }

    pub fn to_mm(self, rect: Rect, pos: Pos2) -> Point {
        let s = self.scale();
        let c = rect.center();
        Point::new(
            self.center.x + f64::from(pos.x - c.x) / s,
            self.center.y + f64::from(pos.y - c.y) / s,
        )
    }

    /// The visible area in mm.
    pub fn visible(self, rect: Rect) -> BBox {
        BBox::new(self.to_mm(rect, rect.min), self.to_mm(rect, rect.max))
    }

    /// Zooms by `factor` and keeps the design point under `pos` in the same place.
    pub fn zoom_at(&mut self, rect: Rect, pos: Pos2, factor: f64) {
        let before = self.to_mm(rect, pos);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.to_mm(rect, pos);
        self.center += before - after;
    }

    /// Shows the full box with a small border.
    pub fn fit(&mut self, rect: Rect, b: BBox) {
        let w = b.width().max(1.0);
        let h = b.height().max(1.0);
        let sx = f64::from(rect.width()) / (w * points_per_mm());
        let sy = f64::from(rect.height()) / (h * points_per_mm());
        self.zoom = (sx.min(sy) * 0.9).clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = b.center();
    }

    /// Flatten tolerance in mm that gives about a quarter point of error on screen.
    pub fn tolerance(self) -> f64 {
        0.25 / self.scale()
    }
}

pub fn color32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

/// The distance from an edge of the canvas, in points, at which the
/// endless pan makes the pointer jump.
const WRAP_EDGE: f32 = 3.0;
/// The frames in which the jump of the pointer can come.
const JUMP_FRAMES: u8 = 10;

/// Endless pan: where the pointer at `pos` goes if it is at an edge of
/// `rect` (on the other side, a little inside), else `None`.
pub fn pan_wrap(rect: Rect, pos: Pos2) -> Option<Pos2> {
    let inner = rect.shrink(WRAP_EDGE);
    if inner.width() <= WRAP_EDGE * 4.0 || inner.height() <= WRAP_EDGE * 4.0 {
        return None;
    }
    let mut to = pos;
    let gap = WRAP_EDGE * 2.0;
    if pos.x <= inner.left() {
        to.x = inner.right() - gap;
    } else if pos.x >= inner.right() {
        to.x = inner.left() + gap;
    }
    if pos.y <= inner.top() {
        to.y = inner.bottom() - gap;
    } else if pos.y >= inner.bottom() {
        to.y = inner.top() + gap;
    }
    (to != pos).then_some(to)
}

/// The pan movement `delta` of this frame without the jump of the pointer
/// that the endless pan asked for: the jump is taken out in the frame
/// where it comes (the movement is near the jump). A jump that does not come
/// in [`JUMP_FRAMES`] frames is forgotten.
pub fn take_jump(delta: Vec2, jump: &mut Option<(Vec2, u8)>) -> Vec2 {
    let Some((j, left)) = *jump else { return delta };
    if (delta - j).length() < j.length() * 0.5 {
        *jump = None;
        return delta - j;
    }
    *jump = left.checked_sub(1).filter(|l| *l > 0).map(|l| (j, l));
    delta
}

/// Shows the canvas and handles its input.
pub fn show(ui: &mut egui::Ui, st: &mut AppState) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    {
        let d = st.doc.design();
        let name = crate::i18n::fill(
            "Design canvas: {} objects, {} selected",
            &[&d.objects().len(), &st.selection.len()],
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &name));
    }
    let rect = response.rect;
    st.canvas_rect = Some(rect);
    let visuals = ui.visuals().clone();
    painter.rect_filled(
        rect,
        0.0,
        if visuals.dark_mode {
            Color32::from_gray(32)
        } else {
            Color32::from_gray(248)
        },
    );

    // Input: wheel zooms about the cursor, middle or right drag pans,
    // double-click on the wheel zooms to extents (as in CAD programs),
    // left click selects (Ctrl toggles, Shift selects a sew-order range).
    if response.hovered() && ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Middle)) {
        st.fit_requested = true;
    }
    if st.fit_requested {
        st.fit_requested = false;
        if let Some(b) = st.fit_box() {
            st.view.fit(rect, b);
        }
    }

    // Touch screens (P15 step 3): two fingers pan, and pinch zooms about
    // the middle between them. From the second finger until all fingers are
    // up, the tools take no input, so a pinch does not move or select.
    let (multi, touching) = ui.input(|i| (i.multi_touch(), i.any_touches()));
    if ui.input(|i| i.has_touch_screen()) {
        st.touch_seen = true;
    }
    match multi.filter(|t| rect.contains(t.center_pos)) {
        Some(t) => {
            if !st.touch_gesture {
                st.touch_gesture = true;
                // A drag of one finger before the second one: it ends here.
                if st.drag.is_some() {
                    let map = ViewMap::new(rect, st.view, st.doc.design());
                    manipulate::end_drag(st, &map);
                }
            }
            let s = st.view.scale();
            let d = t.translation_delta;
            st.view.center -= Point::new(f64::from(d.x) / s, f64::from(d.y) / s);
            if t.zoom_delta != 1.0 {
                st.view.zoom_at(rect, t.center_pos, f64::from(t.zoom_delta));
            }
        }
        None if !touching => st.touch_gesture = false,
        None => {}
    }

    if let Some(pos) = response.hover_pos() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        // A pinch on a touch screen zooms above.
        let pinch = if multi.is_some() { 1.0 } else { pinch };
        let mut factor = f64::from(pinch);
        if scroll != 0.0 {
            factor *= (f64::from(scroll) * 0.0015).exp();
        }
        if factor != 1.0 {
            st.view.zoom_at(rect, pos, factor);
        }
        st.cursor_mm = Some(st.view.to_mm(rect, pos));
    } else {
        st.cursor_mm = None;
    }
    if st.zoom_to.is_some()
        && let Some(b) = st.zoom_to.take()
    {
        st.view.fit(rect, b);
    }
    if response.dragged_by(egui::PointerButton::Middle) || response.dragged_by(egui::PointerButton::Secondary) {
        let d = take_jump(response.drag_delta(), &mut st.pan_jump);
        let s = st.view.scale();
        st.view.center -= Point::new(f64::from(d.x) / s, f64::from(d.y) / s);
        // Endless pan (v0.3 plan, Stage 3): at an edge the pointer goes to
        // the other side. Touch input cannot jump.
        if st.settings.endless_pan
            && !ui.input(|i| i.any_touches())
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(to) = pan_wrap(rect, pos)
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CursorPosition(to));
            st.pan_jump = Some((to - pos, JUMP_FRAMES));
        }
    } else {
        st.pan_jump = None;
    }

    let map = ViewMap::new(rect, st.view, st.doc.design());
    // A clipart item dropped from the Clipart tab goes where it is dropped.
    if let Some(drag) = response.dnd_release_payload::<crate::clipart::ClipartDrag>()
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
    {
        st.insert_clipart(&drag.0, Some(map.to_design(pos)));
    }
    let mods = ui.input(|i| i.modifiers);
    st.mods = mods;
    // A draw tool takes the left and right clicks (see `draw`).
    let drawing = !matches!(st.tool, Tool::Select | Tool::Reshape);
    // Two fingers on a touch screen pan and zoom (above): no tool input.
    let gesture = st.touch_gesture;
    // L held: a drag moves the light (ADR 0129), not a tool.
    let lighting = !gesture && crate::viewer::light_drag(ui, &response, rect, st);
    // Rulers and guides first: a drag from a ruler or on a guide is not for the tool.
    let by_guides = !gesture
        && !lighting
        && st.sketch.is_none()
        && !st.hoopings_ui.mode
        && crate::workspace::guide_input(ui, &response, st);
    if gesture || lighting {
    } else if st.sketch.is_some() {
        crate::sketch::handle_input(ui, &response, st, &map);
    } else if st.hoopings_ui.mode {
        crate::hooping::handle_input(ui, &response, st, &map);
    } else if by_guides {
    } else if st.tool == Tool::Reshape {
        crate::reshape::handle_input(ui, &response, st, &map);
    } else if drawing {
        draw::handle_input(ui, &response, st, &map);
    } else {
        // Select tool. Left drag: move, scale, rotate or box-select (see `manipulate`).
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            manipulate::begin_drag(st, &map, origin, mods);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            manipulate::update_drag(st, &map, pos, mods);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            manipulate::end_drag(st, &map);
        }
        if st.drag.is_none()
            && let Some(pos) = response.hover_pos()
            && let Some(icon) = manipulate::hover_cursor(st, &map, pos)
        {
            ui.ctx().set_cursor_icon(icon);
        }

        // Right click: select what is under the pointer, then show the menu.
        // A right drag still pans (above), because a drag is not a click.
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            st.guide_menu = None;
            match pick(st, st.view.to_mm(rect, pos)) {
                Some(id) if !st.selection.contains(id) => st.selection.set(id),
                Some(_) => {}
                None => {
                    st.selection.clear();
                    // A guide or a construction mark there: its menu.
                    st.guide_menu = crate::guides::mark_at(st, rect, pos, false);
                }
            }
        }
        response.context_menu(|ui| match st.guide_menu {
            Some(target) => crate::guides::menu(ui, st, target),
            None => commands::canvas_menu(ui, st),
        });
        // Double-click on a guide or a mark (no object there): type its place.
        if response.double_clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && pick(st, st.view.to_mm(rect, pos)).is_none()
            && let Some(target) = crate::guides::mark_at(st, rect, pos, false)
        {
            st.edit_guide(target);
        }

        // A click for a command that waits for an object.
        let picked = st.pick.is_some()
            && response.clicked()
            && response.interact_pointer_pos().is_some_and(|pos| {
                let id = pick(st, st.view.to_mm(rect, pos));
                st.pick_clicked(id)
            });
        // Double-click on lettering: edit its text.
        if response.double_clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(id) = pick(st, st.view.to_mm(rect, pos))
            && matches!(
                st.doc.design().object(id).map(|o| &o.kind),
                Some(emb_model::ObjectKind::Lettering(_))
            )
        {
            st.selection.set(id);
            st.edit_lettering(id);
        } else if response.double_clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(id) = pick(st, st.view.to_mm(rect, pos))
            && let Some(sketch) = st.doc.design().sketch_for(id)
        {
            // Double-click on a sketch, or on an object made from one: open
            // the sketch (ADR 0076).
            st.open_sketch(sketch);
        } else if response.double_clicked()
            && st.pick.is_none()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(id) = pick(st, st.view.to_mm(rect, pos))
            && let Some(g) = st.doc.design().object(id).and_then(|o| o.group)
        {
            // Double-click on an object of a group: select it alone, for
            // example to change the fill of a border trace.
            st.selection.enter(id, g);
        }
        // A click on a handle is not a selection click.
        let on_handle = response
            .interact_pointer_pos()
            .is_some_and(|p| manipulate::hit_handle(st, &map, p).is_some());
        if response.clicked()
            && !picked
            && !on_handle
            && let Some(pos) = response.interact_pointer_pos()
        {
            match pick(st, st.view.to_mm(rect, pos)) {
                Some(id) if mods.command => st.selection.toggle(id),
                Some(id) if mods.shift => st.selection.extend_to(st.doc.design(), id),
                Some(id) => st.selection.set(id),
                // A click on empty canvas clears the selection, unless Ctrl or Shift is held.
                None if !(mods.command || mods.shift) => st.selection.clear(),
                None => {}
            }
        }
    }
    // The menu of a guide (from its ruler marker) with Reshape or a draw
    // tool; the Select tool shows it in place of its own menu.
    if st.sketch.is_none()
        && !st.hoopings_ui.mode
        && (by_guides || st.tool != Tool::Select)
        && let Some(target) = st.guide_menu
        && response
            .context_menu(|ui| crate::guides::menu(ui, st, target))
            .is_none()
        && !by_guides
    {
        st.guide_menu = None;
    }

    // Groups are selected together; locked objects cannot be selected.
    st.apply_groups_and_locks();

    let painter = painter.with_clip_rect(rect);
    draw_background(&painter, rect, st);
    crate::viewtools::draw_fabric(&painter, ui.ctx(), rect, st);
    crate::product::draw(&painter, ui.ctx(), rect, st);
    crate::workspace::draw_grid(&painter, rect, st, visuals.dark_mode);
    crate::backdrop::draw(&painter, ui.ctx(), &map, st);
    draw_hoop(&painter, rect, st, visuals.dark_mode);

    crate::vectors::draw(&painter, rect, st);
    crate::sketch::draw_objects(&painter, &map, st);
    if st.show_applique_fabric {
        crate::applique::draw_fabric(&painter, rect, st);
    }
    st.paint_plan_in(ui.ctx());
    if st.show_plan {
        // The stitches that export writes, from the cached stitch plan.
        let segs = crate::viewing::visible_segments(st);
        // The last plan shows while a thread makes the new one (ADR 0077).
        let waits = st.plan_waits;
        if waits {
            ui.ctx().request_repaint_after(crate::clock::Duration::from_millis(40));
        }
        let look = if !st.show_thread_look {
            MeshLook::Stitches
        } else if st.settings.look.three_d {
            MeshLook::Thread3d
        } else {
            MeshLook::Thread
        };
        let table = emb_render::thread3d::ShadeTable::new(st.settings.look.light);
        // The surface on tiles (ADR 0126), else the meshes of the stitches.
        let tiles = if look == MeshLook::Thread3d && st.settings.look.surface {
            let view = st.view;
            crate::stitch_tiles::tiles_of(ui.ctx(), rect, &view, st, true)
        } else {
            None
        };
        // Before and After: the stitches of now on the right of its line.
        let now = match crate::viewer::before_after_line(rect, st) {
            Some(x) => painter
                .with_clip_rect(Rect::from_min_max(Pos2::new(x, rect.top()), rect.max).intersect(painter.clip_rect())),
            None => painter.clone(),
        };
        match tiles {
            Some(frame) => crate::stitch_tiles::paint(&now, rect, st, frame, &segs, look, &table),
            None => {
                let mesh = fitted_mesh(rect, &st.view, &segs, st.show_jumps, look, &table).0;
                now.add(Shape::mesh(mesh));
            }
        }
        // The stitches before a change, on the left of the line (ADR 0129).
        crate::viewer::draw_before_after(ui, &painter, rect, st);
        if st.show_density {
            crate::density::draw(&painter, ui.ctx(), rect, st);
        }
        crate::checks::draw(&painter, ui.ctx(), rect, st);
        crate::parts_view::draw(&painter, ui.ctx(), rect, st);
        crate::repeats::draw(&painter, rect, st);
        crate::viewing::draw_overlay(&painter, rect, st);
        if waits {
            let at = rect.left_top() + egui::vec2(8.0, 8.0);
            let text = painter.layout_no_wrap(tr("Working…").into(), egui::FontId::proportional(14.0), Color32::WHITE);
            painter.rect_filled(
                Rect::from_min_size(at, text.size()).expand(4.0),
                4.0,
                Color32::from_black_alpha(160),
            );
            painter.galley(at, text, Color32::WHITE);
        }
    }
    // Rhinestones go on after the embroidery.
    if st.show_stones {
        crate::bling::draw(&painter, rect, st);
    }
    if st.show_artwork {
        draw_artwork(&painter, rect, st);
    }
    if st.hoopings_ui.mode {
        crate::hooping::draw(&painter, rect, st);
    }
    let accent = st.settings.display.selection(visuals.selection.stroke.color);
    let text_bg = visuals.extreme_bg_color.gamma_multiply(0.85);
    if st.sketch.is_some() {
        crate::sketch::draw_overlay(&painter, ui, &map, st);
    } else if st.tool == Tool::Reshape {
        crate::reshape::draw_overlay(&painter, &map, st, accent);
    } else if st.tool == Tool::Measure {
        crate::workspace::draw_measure(&painter, &map, st, accent, text_bg);
    } else if st.tool == Tool::Zoom {
        manipulate::draw_overlay(&painter, &map, st, accent, text_bg);
    } else if drawing {
        draw::draw_preview(&painter, ui, st, &map, accent);
    } else {
        manipulate::draw_overlay(&painter, &map, st, accent, text_bg);
    }
    // The direction line of the selected fill, dashed.
    if let Some(id) = st.selection.single()
        && let Some(emb_model::ObjectKind::Fill { params, .. }) = st.doc.design().object(id).map(|o| &o.kind)
        && let Some(dir) = &params.direction
    {
        let pts: Vec<Pos2> = dir.points.iter().map(|p| map.to_screen(*p)).collect();
        painter.extend(Shape::dashed_line(&pts, Stroke::new(1.5, accent), 6.0, 4.0));
    }
    crate::guides::draw(&painter, rect, st);
    crate::workspace::draw_rulers(&painter, rect, st, visuals.dark_mode);
    crate::guides::draw_ruler_markers(&painter, rect, st);
    // The magnifier and the light by hand (ADR 0129), over all.
    if let Some(pos) = response.hover_pos() {
        crate::viewer::draw_magnifier(&painter, ui.ctx(), rect, pos, st);
    }
    crate::viewer::draw_light(&painter, ui.ctx(), rect, st, response.hovered());
    touch_bar(ui, rect, st, drawing);
    side_handle(ui, rect, st);
}

/// The button at the right edge of the canvas that hides and shows the side
/// panels (P15 step 3): the canvas gets the width, and one tap brings them
/// back. Larger on a touch screen.
fn side_handle(ui: &mut egui::Ui, rect: Rect, st: &mut AppState) {
    let size = if st.touch_seen {
        Vec2::new(26.0, 64.0)
    } else {
        Vec2::new(16.0, 44.0)
    };
    let at = Rect::from_min_size(
        Pos2::new(rect.right() - size.x - 2.0, rect.center().y - size.y / 2.0),
        size,
    );
    let (glyph, tip) = if st.side_hidden {
        (egui_phosphor::regular::CARET_LEFT, tr("Show the side panels"))
    } else {
        (egui_phosphor::regular::CARET_RIGHT, tr("Hide the side panels"))
    };
    let resp = ui.put(at, egui::Button::new(egui::RichText::new(glyph).size(14.0)));
    let resp = crate::a11y::named(resp, tip).on_hover_text(tip);
    if resp.clicked() {
        st.side_hidden = !st.side_hidden;
    }
}

/// On a touch screen with no keyboard, the buttons for the keys of the
/// draw tools (Enter, Backspace, Esc) at the bottom of the canvas while a
/// shape has points (P15 step 3). It shows only after a touch, so a computer
/// with a mouse does not get it.
fn touch_bar(ui: &mut egui::Ui, rect: Rect, st: &mut AppState, drawing: bool) {
    let has_points = !st.draw.points.is_empty() || !st.draw.stages.is_empty();
    if !drawing || !has_points || !ui.input(|i| i.has_touch_screen()) {
        return;
    }
    egui::Area::new(egui::Id::new("touch_draw_bar"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.center_bottom() - Vec2::new(150.0, 64.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.spacing_mut().button_padding = Vec2::new(14.0, 10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("Finish")).clicked() {
                        st.draw_key(egui::Key::Enter);
                    }
                    if ui.button(tr("Remove Point")).clicked() {
                        st.draw_key(egui::Key::Backspace);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        st.draw_key(egui::Key::Escape);
                    }
                });
            });
        });
}

/// Returns the topmost object at the point (view mm), by its real shape.
/// The tolerance is 4 screen points.
pub fn pick(st: &AppState, p_mm: Point) -> Option<ObjectId> {
    let design = st.doc.design();
    let inv = design.rotation_transform().inverse()?;
    let (p, tol) = (inv.apply(p_mm), 4.0 / st.view.scale());
    // Locked and hidden objects cannot be picked.
    design
        .objects()
        .iter()
        .rev()
        .filter(|o| !o.locked && st.view_filter.shows(o))
        .find(|o| o.hit_test(p, tol))
        .map(|o| o.id)
}

pub(crate) fn screen_polylines(path: &Path, t: &Transform, rect: Rect, view: &CanvasView) -> Vec<(Vec<Pos2>, bool)> {
    path.flatten(view.tolerance())
        .into_iter()
        .map(|pl| {
            let pts = pl.points.iter().map(|p| view.to_screen(rect, t.apply(*p))).collect();
            (pts, pl.closed)
        })
        .collect()
}

/// The background (fabric) color of the colorway: inside the hoop field, or
/// the whole canvas in open-field mode.
fn draw_background(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let design = st.doc.design();
    let Some(bg) = design.background else { return };
    let fill = color32(bg);
    match st.hoop_outline(0.0) {
        Some(path) => {
            for (pts, _) in screen_polylines(&path, &Transform::IDENTITY, rect, &st.view) {
                // Hoop fields are convex (rectangles, ovals, circles).
                painter.add(Shape::convex_polygon(pts, fill, Stroke::NONE));
            }
        }
        None => {
            painter.rect_filled(rect, 0.0, fill);
        }
    }
}

/// The brightness of a color, 0 (black) to 1 (white).
fn brightness(c: Color32) -> f32 {
    (0.299 * f32::from(c.r()) + 0.587 * f32::from(c.g()) + 0.114 * f32::from(c.b())) / 255.0
}

/// The colors of the hoop lines: the line, and a halo under it if a
/// product or a fabric color is under the hoop. The line then contrasts
/// with that color (if the user did not set a hoop color), and the halo
/// keeps the line clear on a picture and on the canvas round the product.
fn hoop_inks(st: &AppState, dark: bool) -> (Color32, Option<Color32>) {
    let design = st.doc.design();
    let theme = st.settings.display.hoop(dark);
    // Some(Some(color)): a product or a fabric color; Some(None): a picture.
    let under = match &design.product {
        Some(v) if v.visible => Some(v.picture.is_none().then_some(v.color)),
        _ => design.background.map(Some),
    };
    let Some(under) = under else {
        return (theme, None);
    };
    let ink = match (st.settings.display.hoop, under) {
        (None, Some(c)) if brightness(color32(c)) > 0.5 => Color32::from_gray(35),
        (None, Some(_)) => Color32::from_gray(235),
        _ => theme,
    };
    let halo = if brightness(ink) > 0.5 {
        Color32::from_black_alpha(150)
    } else {
        Color32::from_white_alpha(170)
    };
    (ink, Some(halo))
}

/// Draws a line of the hoop, with its halo first.
fn hoop_line(painter: &egui::Painter, pts: Vec<Pos2>, width: f32, ink: Color32, halo: Option<Color32>, dash: bool) {
    let add = |w: f32, c: Color32| {
        if dash {
            painter.extend(Shape::dashed_line(&pts, Stroke::new(w, c), 6.0, 4.0));
        } else {
            painter.add(Shape::line(pts.clone(), Stroke::new(w, c)));
        }
    };
    if let Some(h) = halo {
        add(width + 2.5, h);
    }
    add(width, ink);
}

fn draw_hoop(painter: &egui::Painter, rect: Rect, st: &AppState, dark: bool) {
    let design = st.doc.design();
    let center = design.hoop.center;
    let (ink, halo) = hoop_inks(st, dark);

    // Hoop center cross, along the axes of the hoop.
    let c = st.view.to_screen(rect, center);
    let (s, co) = emb_geom::math::sin_cos_deg(design.hoop.angle_deg);
    let (ax, ay) = (Vec2::new(co as f32, s as f32), Vec2::new(-s as f32, co as f32));
    for a in [ax, ay] {
        hoop_line(painter, vec![c - a * 8.0, c + a * 8.0], 1.0, ink, halo, false);
    }

    // The auto start and end points (ADR 0032).
    let se = design.start_end;
    let rot = design.rotation_transform();
    for (a, end) in [(se.start, false), (se.end, true)] {
        let Some(p) = a.and_then(|a| design.anchor_point(a)) else {
            continue;
        };
        let s = st.view.to_screen(rect, rot.apply(p));
        let color = if end {
            Color32::from_rgb(210, 70, 60)
        } else {
            Color32::from_rgb(40, 170, 80)
        };
        painter.circle_stroke(s, 6.0, Stroke::new(2.0, color));
        painter.text(
            s + Vec2::new(8.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            if end { "End" } else { "Start" },
            egui::FontId::proportional(11.0),
            color,
        );
    }

    // In hooping mode each hooping shows its own field.
    if !st.show_hoop || st.hoopings_ui.mode {
        return;
    }
    let id_t = Transform::IDENTITY;
    if let Some(field) = st.hoop_outline(0.0) {
        for (mut pts, _) in screen_polylines(&field, &id_t, rect, &st.view) {
            pts.push(pts[0]);
            hoop_line(painter, pts, 1.5, ink, halo, false);
        }
    }
    if let Some(safe) = st.hoop_outline(st.margin_mm) {
        // With a halo, the dashes need the full color to show.
        let dash_ink = if halo.is_some() { ink } else { ink.gamma_multiply(0.6) };
        for (mut pts, _) in screen_polylines(&safe, &id_t, rect, &st.view) {
            pts.push(pts[0]);
            hoop_line(painter, pts, 1.0, dash_ink, halo, true);
        }
    }
    // The back of the machine: a small arrow out of the back edge. The top
    // of the design sews toward it.
    if let Some((back, dir)) = st.hoop_back() {
        let b = st.view.to_screen(rect, back);
        let (out, side) = (
            Vec2::new(dir.x as f32, dir.y as f32),
            Vec2::new(-dir.y as f32, dir.x as f32),
        );
        painter.add(Shape::convex_polygon(
            vec![b + out * 10.0, b + side * 6.0 + out * 2.0, b - side * 6.0 + out * 2.0],
            ink,
            halo.map_or(Stroke::NONE, |h| Stroke::new(1.5, h)),
        ));
        let text = painter.layout_no_wrap(tr("Back").into(), egui::FontId::proportional(11.0), ink);
        let at = b + out * 20.0 - text.size() * 0.5;
        if let Some(h) = halo {
            painter.rect_filled(Rect::from_min_size(at, text.size()).expand(2.0), 3.0, h);
        }
        painter.galley(at, text, ink);
    }
}

fn draw_artwork(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let design = st.doc.design();
    let t = design.rotation_transform();
    for obj in design.objects().iter().filter(|o| st.view_filter.shows(o)) {
        let color = design
            .thread(obj.thread)
            .map(|t| color32(t.color))
            .unwrap_or(Color32::GRAY);
        let selected = st.selection.contains(obj.id);
        let hovered = st.hovered == Some(obj.id);
        let outside = st.outside_margin(obj.id);
        let width = if selected { 2.5 } else { 1.5 };
        for path in obj.kind.paths() {
            for (pts, closed) in screen_polylines(path, &t, rect, &st.view) {
                if selected || hovered {
                    let alpha = if selected { 110 } else { 70 };
                    let sel = st.settings.display.selection(Color32::from_rgb(80, 160, 255));
                    let halo = Stroke::new(
                        width + 3.0,
                        Color32::from_rgba_unmultiplied(sel.r(), sel.g(), sel.b(), alpha),
                    );
                    painter.add(if closed {
                        Shape::closed_line(pts.clone(), halo)
                    } else {
                        Shape::line(pts.clone(), halo)
                    });
                }
                if outside {
                    let warn = Stroke::new(width + 2.0, Color32::from_rgb(240, 140, 0));
                    painter.add(if closed {
                        Shape::closed_line(pts.clone(), warn)
                    } else {
                        Shape::line(pts.clone(), warn)
                    });
                }
                // A vector shows in its own colors (`vectors::draw`).
                if matches!(obj.kind, emb_model::ObjectKind::Vector { .. }) {
                    continue;
                }
                let stroke = Stroke::new(width, color);
                painter.add(if closed {
                    Shape::closed_line(pts, stroke)
                } else {
                    Shape::line(pts, stroke)
                });
            }
        }
    }
}

/// Builds one mesh of thin quads for all visible segments.
/// Returns the mesh and the number of segments drawn.
pub fn stitch_mesh(rect: Rect, view: &CanvasView, segments: &[(Segment, Color32)], show_jumps: bool) -> (Mesh, usize) {
    let s = view.scale();
    let vis = view.visible(rect).expand(THREAD_WIDTH_MM);
    let stitch_half = ((THREAD_WIDTH_MM * s) as f32 / 2.0).clamp(0.5, 12.0);
    let jump_half = 0.5f32;
    let mut mesh = Mesh::default();
    mesh.reserve_vertices(segments.len() * 4);
    mesh.reserve_triangles(segments.len() * 2);
    let mut drawn = 0;

    for (seg, color) in segments {
        let (half, color) = match seg.kind {
            SegmentKind::Stitch => (stitch_half, *color),
            SegmentKind::Jump if show_jumps => (jump_half, color.gamma_multiply(0.4)),
            SegmentKind::Jump => continue,
        };
        // Cull segments that are fully outside the visible area.
        let (a, b) = (seg.a, seg.b);
        if (a.x < vis.min.x && b.x < vis.min.x)
            || (a.x > vis.max.x && b.x > vis.max.x)
            || (a.y < vis.min.y && b.y < vis.min.y)
            || (a.y > vis.max.y && b.y > vis.max.y)
        {
            continue;
        }
        let pa = view.to_screen(rect, a);
        let pb = view.to_screen(rect, b);
        let d = pb - pa;
        let len = d.length();
        if len < 0.01 {
            continue;
        }
        let n = Vec2::new(-d.y, d.x) / len * half;
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(pa + n, color);
        mesh.colored_vertex(pa - n, color);
        mesh.colored_vertex(pb - n, color);
        mesh.colored_vertex(pb + n, color);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
        drawn += 1;
    }
    (mesh, drawn)
}

/// Width of a thread in the Thread look view (mm).
const THREAD_LOOK_MM: f64 = 0.45;

/// The most vertices of the stitch mesh of one frame. egui-wgpu makes its
/// vertex buffer two times larger when a frame needs more, and the
/// graphics library allows at most 256 MB for one buffer (20 bytes for each
/// vertex), so a mesh of more than 128 MB can stop the program. The view
/// bench found this with 800,000 stitches in the 3D thread look (ADR 0122).
pub const MAX_MESH_VERTICES: usize = 6_000_000;

/// The meshes of the stitches, from the heaviest to the lightest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshLook {
    /// [`thread_mesh_3d`].
    Thread3d,
    /// [`thread_mesh`].
    Thread,
    /// [`stitch_mesh`].
    Stitches,
}

impl MeshLook {
    /// The most vertices of one stitch with its shadow.
    fn vertices(self) -> usize {
        match self {
            MeshLook::Thread3d => 19,
            MeshLook::Thread => 10,
            MeshLook::Stitches => 4,
        }
    }

    fn lighter(self) -> Option<MeshLook> {
        match self {
            MeshLook::Thread3d => Some(MeshLook::Thread),
            MeshLook::Thread => Some(MeshLook::Stitches),
            MeshLook::Stitches => None,
        }
    }
}

/// The mesh of the stitches in `look`, or in a lighter look if the mesh
/// can have more than [`MAX_MESH_VERTICES`] vertices. If the stitches that
/// show are too many for the lightest look too, it shows only each n-th
/// stitch. Returns the mesh and its look.
pub fn fitted_mesh(
    rect: Rect,
    view: &CanvasView,
    segments: &[(Segment, Color32)],
    show_jumps: bool,
    look: MeshLook,
    table: &emb_render::thread3d::ShadeTable,
) -> (Mesh, MeshLook) {
    fitted_mesh_with(rect, view, segments, show_jumps, look, table, MAX_MESH_VERTICES)
}

/// As [`fitted_mesh`], with a limit of `max` vertices.
fn fitted_mesh_with(
    rect: Rect,
    view: &CanvasView,
    segments: &[(Segment, Color32)],
    show_jumps: bool,
    mut look: MeshLook,
    table: &emb_render::thread3d::ShadeTable,
    max: usize,
) -> (Mesh, MeshLook) {
    let vis = view.visible(rect).expand(THREAD_LOOK_MM * 3.0);
    let shows = |(s, _): &&(Segment, Color32)| {
        !((s.a.x < vis.min.x && s.b.x < vis.min.x)
            || (s.a.x > vis.max.x && s.b.x > vis.max.x)
            || (s.a.y < vis.min.y && s.b.y < vis.min.y)
            || (s.a.y > vis.max.y && s.b.y > vis.max.y))
    };
    let visible = segments.iter().filter(shows).count();
    while visible * look.vertices() > max
        && let Some(l) = look.lighter()
    {
        look = l;
    }
    let fewer: Vec<(Segment, Color32)>;
    let segments = if visible * look.vertices() > max {
        let step = (visible * look.vertices()).div_ceil(max);
        fewer = segments.iter().filter(shows).step_by(step).copied().collect();
        &fewer[..]
    } else {
        segments
    };
    let mesh = match look {
        MeshLook::Thread3d => thread_mesh_3d(rect, view, segments, show_jumps, table),
        MeshLook::Thread => thread_mesh(rect, view, segments, show_jumps),
        MeshLook::Stitches => stitch_mesh(rect, view, segments, show_jumps).0,
    };
    (mesh, look)
}

/// The Thread look view (spec UX rule 4, "Thread preview"): each stitch is a
/// thread with a light middle and dark edges, over a soft shadow. When a
/// thread is narrower than 2 screen points, this is the normal stitch view.
pub fn thread_mesh(rect: Rect, view: &CanvasView, segments: &[(Segment, Color32)], show_jumps: bool) -> Mesh {
    let s = view.scale() as f32;
    let half = (THREAD_LOOK_MM as f32 * s) / 2.0;
    if half < 1.0 {
        return stitch_mesh(rect, view, segments, show_jumps).0;
    }
    let vis = view.visible(rect).expand(THREAD_LOOK_MM);
    let mut mesh = Mesh::default();
    mesh.reserve_vertices(segments.len() * 10);
    mesh.reserve_triangles(segments.len() * 6);
    let shadow_offset = Vec2::new(0.35, 0.5) * half;
    let base = half;
    for (seg, color) in segments {
        // The width of the weight of the thread (ADR 0123).
        let half = base * seg.width_factor() as f32;
        let (a, b) = (seg.a, seg.b);
        if (a.x < vis.min.x && b.x < vis.min.x)
            || (a.x > vis.max.x && b.x > vis.max.x)
            || (a.y < vis.min.y && b.y < vis.min.y)
            || (a.y > vis.max.y && b.y > vis.max.y)
        {
            continue;
        }
        let (pa, pb) = (view.to_screen(rect, a), view.to_screen(rect, b));
        let d = pb - pa;
        let len = d.length();
        if len < 0.01 {
            continue;
        }
        let u = d / len;
        let n = Vec2::new(-u.y, u.x);
        if seg.kind == SegmentKind::Jump {
            if show_jumps {
                let c = color.gamma_multiply(0.4);
                let i = mesh.vertices.len() as u32;
                for p in [pa + n * 0.5, pa - n * 0.5, pb - n * 0.5, pb + n * 0.5] {
                    mesh.colored_vertex(p, c);
                }
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i, i + 2, i + 3);
            }
            continue;
        }
        // The thread goes into the fabric at the ends: round them a little.
        let (pa, pb) = (pa + u * half * 0.3, pb - u * half * 0.3);
        let [r, g, bl, al] = color.to_array();
        let shade = |k: f32| {
            Color32::from_rgba_premultiplied((r as f32 * k) as u8, (g as f32 * k) as u8, (bl as f32 * k) as u8, al)
        };
        let light = {
            let up = |v: u8| (v as f32 + (al as f32 - v as f32) * 0.35) as u8;
            Color32::from_rgba_premultiplied(up(r), up(g), up(bl), al)
        };
        let edge = shade(0.55);
        let shadow = Color32::from_black_alpha((al as f32 * 0.3) as u8);
        // Shadow under the thread.
        let i = mesh.vertices.len() as u32;
        for p in [pa + n * half, pa - n * half, pb - n * half, pb + n * half] {
            mesh.colored_vertex(p + shadow_offset, shadow);
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
        // The thread: edge, middle, edge at each end.
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(pa + n * half, edge);
        mesh.colored_vertex(pa, light);
        mesh.colored_vertex(pa - n * half, edge);
        mesh.colored_vertex(pb + n * half, edge);
        mesh.colored_vertex(pb, light);
        mesh.colored_vertex(pb - n * half, edge);
        mesh.add_triangle(i, i + 1, i + 4);
        mesh.add_triangle(i, i + 4, i + 3);
        mesh.add_triangle(i + 1, i + 2, i + 5);
        mesh.add_triangle(i + 1, i + 5, i + 4);
    }
    mesh
}

/// A color with a brightness factor and a highlight, for a premultiplied
/// color: the highlight goes toward the alpha, not toward white.
fn lit_premultiplied(c: [u8; 4], (factor, shine): (f32, f32)) -> Color32 {
    let a = f32::from(c[3]);
    let v = |x: u8| {
        let x = (f32::from(x) * factor).min(a);
        (x + (a - x) * shine).round().clamp(0.0, a) as u8
    };
    Color32::from_rgba_premultiplied(v(c[0]), v(c[1]), v(c[2]), c[3])
}

/// Half thread widths (screen points) from which the 3D mesh has three
/// rows of points along each stitch, so the thread is darker where it
/// goes into the fabric; below it, one row (the middle shading) at each end.
const ALONG_ROWS_FROM: f32 = 6.0;

/// The 3D thread look (P10 Milestone 3): each stitch as a round thread with
/// light and shade from `table` (5 points across; 3 along when the zoom is
/// large, see [`ALONG_ROWS_FROM`]), and its shadow away from the light.
/// Small zoom: as [`thread_mesh`].
pub fn thread_mesh_3d(
    rect: Rect,
    view: &CanvasView,
    segments: &[(Segment, Color32)],
    show_jumps: bool,
    table: &emb_render::thread3d::ShadeTable,
) -> Mesh {
    use emb_render::thread3d::{ACROSS, ALONG};
    let s = view.scale() as f32;
    let half = (THREAD_LOOK_MM as f32 * s) / 2.0;
    if half < 1.0 {
        return thread_mesh(rect, view, segments, show_jumps);
    }
    let light = table.lighting;
    let vis = view.visible(rect).expand(THREAD_LOOK_MM * 3.0);
    let mut mesh = Mesh::default();
    mesh.reserve_vertices(segments.len() * 19);
    mesh.reserve_triangles(segments.len() * 18);
    let (sx, sy) = light.shadow_dir();
    let shadow_offset = Vec2::new(sx as f32, sy as f32) * (light.shadow_length() as f32 * half);
    let shadow_alpha = 0.16 * light.strength.clamp(0.0, 1.0) as f32;
    // The lit colors of the mesh points for each direction, for the color
    // of the stitches before (stitches come in runs of one color).
    const POINTS: usize = ACROSS.len() * ALONG.len();
    let mut cache: Vec<Option<[Color32; POINTS]>> = vec![None; emb_render::thread3d::ShadeTable::DIRECTIONS];
    let mut cache_color = (Color32::TRANSPARENT, ThreadKind::default());
    let base = half;
    for (seg, color) in segments {
        // The width of the weight of the thread and the highlight of its
        // kind (ADR 0123).
        let half = base * seg.width_factor() as f32;
        let shine = emb_render::thread3d::shine_of(seg.thread_kind) as f32;
        let (a, b) = (seg.a, seg.b);
        if (a.x < vis.min.x && b.x < vis.min.x)
            || (a.x > vis.max.x && b.x > vis.max.x)
            || (a.y < vis.min.y && b.y < vis.min.y)
            || (a.y > vis.max.y && b.y > vis.max.y)
        {
            continue;
        }
        let (pa, pb) = (view.to_screen(rect, a), view.to_screen(rect, b));
        let d = pb - pa;
        let len = d.length();
        if len < 0.01 {
            continue;
        }
        let u = d / len;
        // The side of positive `s` in the shading (left of the direction).
        let n = Vec2::new(u.y, -u.x);
        if seg.kind == SegmentKind::Jump {
            if show_jumps {
                let c = color.gamma_multiply(0.4);
                let i = mesh.vertices.len() as u32;
                for p in [pa + n * 0.5, pa - n * 0.5, pb - n * 0.5, pb + n * 0.5] {
                    mesh.colored_vertex(p, c);
                }
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i, i + 2, i + 3);
            }
            continue;
        }
        let rgba = color.to_array();
        // The shadow on what is under the thread.
        let shadow = Color32::from_black_alpha((f32::from(rgba[3]) * shadow_alpha) as u8);
        let i = mesh.vertices.len() as u32;
        for p in [pa + n * half, pa - n * half, pb - n * half, pb + n * half] {
            mesh.colored_vertex(p + shadow_offset, shadow);
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
        // The thread goes into the fabric at the ends: round them a little.
        let (pa, pb) = (pa + u * half * 0.3, pb - u * half * 0.3);
        if (*color, seg.thread_kind) != cache_color {
            cache.iter_mut().for_each(|c| *c = None);
            cache_color = (*color, seg.thread_kind);
        }
        let k = table.index(d.x, d.y);
        let lit = *cache[k].get_or_insert_with(|| {
            let shade = table.cell(k);
            std::array::from_fn(|j| lit_premultiplied(rgba, (shade[j].0, shade[j].1 * shine)))
        });
        let i = mesh.vertices.len() as u32;
        // The rows: their place along the stitch and their row of the table.
        let rows: &[(f32, usize)] = if half >= ALONG_ROWS_FROM {
            &[(ALONG[0] as f32, 0), (ALONG[1] as f32, 1), (ALONG[2] as f32, 2)]
        } else {
            &[(0.0, 1), (1.0, 1)]
        };
        for &(t, r) in rows {
            let p = pa + (pb - pa) * t;
            for (c, &sv) in ACROSS.iter().enumerate() {
                mesh.colored_vertex(p + n * (sv as f32 * half), lit[r * ACROSS.len() + c]);
            }
        }
        let w = ACROSS.len() as u32;
        for r in 0..rows.len() as u32 - 1 {
            for c in 0..w - 1 {
                let k = i + r * w + c;
                mesh.add_triangle(k, k + 1, k + w + 1);
                mesh.add_triangle(k, k + w + 1, k + w);
            }
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_fingers_zoom_about_their_middle_pan_and_move_nothing() {
        // P15 step 3: a pinch that starts on an object does not move it.
        let mut h = crate::test_util::Harness::new();
        let id = h.id(0);
        let object = h.center_of(id);
        let steps = h.st.doc.undo_len();
        let zoom = h.st.view.zoom;
        let a = h.screen(object);
        let b = a + Vec2::new(120.0, 0.0);
        let middle = h.st.view.to_mm(crate::test_util::screen_rect(), a + (b - a) * 0.5);
        // Spread from 120 to 240 points, and move the middle 50 points down.
        let shift = Vec2::new(0.0, 50.0);
        h.pinch(
            (a, a - Vec2::new(60.0, 0.0) + shift),
            (b, b + Vec2::new(60.0, 0.0) + shift),
        );
        assert!(
            (h.st.view.zoom / zoom - 2.0).abs() < 0.05,
            "{} {}",
            zoom,
            h.st.view.zoom
        );
        // The point that was between the fingers is between them again.
        let now = h.st.view.to_screen(crate::test_util::screen_rect(), middle);
        let want = a + (b - a) * 0.5 + shift;
        assert!(now.distance(want) < 3.0, "{now:?} {want:?}");
        assert_eq!(h.center_of(id), object, "the object did not move");
        assert_eq!(h.st.doc.undo_len(), steps, "no undo step");
        assert!(h.st.drag.is_none() && !h.st.touch_gesture);
    }

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))
    }

    #[test]
    fn the_hoop_lines_contrast_with_the_product() {
        let mut h = crate::test_util::Harness::new();
        // No product and no fabric color: the theme color, no halo.
        assert_eq!(hoop_inks(&h.st, true), (Color32::from_gray(200), None));
        // A white shirt in the dark theme: dark lines with a light halo.
        h.st.set_product(emb_model::ProductKind::TShirt);
        let (ink, halo) = hoop_inks(&h.st, true);
        assert!(brightness(ink) < 0.2 && halo.is_some_and(|c| brightness(c) > 0.5));
        // A black shirt: light lines with a dark halo.
        h.st.doc
            .edit("Color", |d| d.product.as_mut().unwrap().color = Rgb(20, 20, 25));
        let (ink, halo) = hoop_inks(&h.st, false);
        assert!(brightness(ink) > 0.8 && halo.is_some_and(|c| c.r() < 50));
        // A hoop color of the user stays, with a halo.
        h.st.settings.display.hoop = Some(Rgb(255, 0, 0));
        assert_eq!(hoop_inks(&h.st, false).0, Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn screen_mm_round_trip() {
        let v = CanvasView {
            center: Point::new(12.0, -7.0),
            zoom: 3.7,
        };
        let p = Point::new(30.0, 5.0);
        let q = v.to_mm(rect(), v.to_screen(rect(), p));
        assert!(p.distance(q) < 1e-3);
    }

    #[test]
    fn zoom_keeps_point_under_cursor() {
        let mut v = CanvasView::default();
        let pos = Pos2::new(600.0, 100.0);
        let before = v.to_mm(rect(), pos);
        v.zoom_at(rect(), pos, 4.0);
        assert!(before.distance(v.to_mm(rect(), pos)) < 1e-6);
        v.zoom_at(rect(), pos, 1e9);
        assert_eq!(v.zoom, MAX_ZOOM);
    }

    #[test]
    fn thread_look_mesh() {
        let mut v = CanvasView {
            zoom: 8.0,
            ..Default::default()
        };
        let seg = |x: f64, kind| {
            (
                Segment {
                    a: Point::new(x, 0.0),
                    b: Point::new(x + 2.0, 0.0),
                    color: Rgb(200, 0, 0),
                    kind,
                    ..Default::default()
                },
                Color32::from_rgb(200, 0, 0),
            )
        };
        let segs = [
            seg(0.0, SegmentKind::Stitch),
            seg(5.0, SegmentKind::Jump),
            seg(9000.0, SegmentKind::Stitch),
        ];
        let m = thread_mesh(rect(), &v, &segs, false);
        // One stitch: a shadow (4 points) and a thread (6 points); the jump is hidden, the far stitch culled.
        assert_eq!(m.vertices.len(), 10);
        assert_eq!(m.indices.len(), 6 * 3);
        assert_eq!(thread_mesh(rect(), &v, &segs, true).vertices.len(), 14);
        // Zoomed out: the normal stitch view.
        v.zoom = 0.2;
        assert_eq!(thread_mesh(rect(), &v, &segs[..1], false).vertices.len(), 4);
    }

    #[test]
    fn mesh_culls_offscreen_segments() {
        let v = CanvasView::default();
        let seg = |x: f64| {
            (
                Segment {
                    a: Point::new(x, 0.0),
                    b: Point::new(x + 1.0, 0.0),
                    color: Rgb(0, 0, 0),
                    kind: SegmentKind::Stitch,
                    ..Default::default()
                },
                Color32::BLACK,
            )
        };
        let (_, drawn) = stitch_mesh(rect(), &v, &[seg(0.0), seg(10_000.0)], false);
        assert_eq!(drawn, 1);
    }

    /// Speed check for the canvas. Run with:
    /// `cargo test --release -p emb-app -- --ignored --nocapture mesh_build_timing`
    #[test]
    #[ignore]
    fn mesh_build_timing() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0));
        for n in [10_000, 100_000, 500_000, 1_000_000] {
            let segments: Vec<(Segment, Color32)> = emb_render::segments(&emb_render::stress::make(n))
                .into_iter()
                .map(|s| (s, color32(s.color)))
                .collect();
            let mut v = CanvasView::default();
            v.fit(
                r,
                emb_render::bounds(&segments.iter().map(|s| s.0).collect::<Vec<_>>()).unwrap(),
            );
            let runs = 10;
            let t0 = crate::clock::Instant::now();
            let mut drawn = 0;
            for _ in 0..runs {
                drawn = stitch_mesh(r, &v, &segments, false).1;
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0 / f64::from(runs);
            println!("{n:>8} stitches: {drawn:>8} drawn, mesh build {ms:.2} ms");
        }
    }

    /// The exit test of P10 Milestone 3: the 3D thread look builds its mesh
    /// in about the time of the 2D thread look (±50 %) at 1:1 and 4:1 zoom.
    /// Run with `cargo test --release -- --ignored thread_look_timing --nocapture`.
    #[test]
    #[ignore]
    fn thread_look_timing() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0));
        let table = emb_render::thread3d::ShadeTable::new(Default::default());
        for n in [10_000, 100_000] {
            let segments: Vec<(Segment, Color32)> = emb_render::segments(&emb_render::stress::make(n))
                .into_iter()
                .map(|s| (s, color32(s.color)))
                .collect();
            for zoom in [1.0, 4.0] {
                let v = CanvasView {
                    center: emb_geom::Point::new(-55.0, -45.0),
                    zoom,
                };
                let time = |f: &dyn Fn() -> Mesh| {
                    let t0 = crate::clock::Instant::now();
                    let mut tris = 0;
                    for _ in 0..10 {
                        tris = f().indices.len() / 3;
                    }
                    (t0.elapsed().as_secs_f64() * 100.0, tris)
                };
                let (t2, n2) = time(&|| thread_mesh(r, &v, &segments, false));
                let (t3, n3) = time(&|| thread_mesh_3d(r, &v, &segments, false, &table));
                println!(
                    "{n:>7} stitches at {zoom}:1: 2D {t2:.2} ms ({n2} triangles), 3D {t3:.2} ms ({n3} triangles), {:.0} %",
                    100.0 * t3 / t2
                );
            }
        }
    }

    #[test]
    fn the_weight_gives_the_width_and_the_kind_the_highlight() {
        let r = rect();
        let v = CanvasView {
            center: Point::ZERO,
            zoom: 16.0,
        };
        let table = emb_render::thread3d::ShadeTable::new(Default::default());
        let seg = |kind: emb_model::ThreadKind, weight: u16| Segment {
            a: Point::new(-4.0, 0.0),
            b: Point::new(4.0, 0.0),
            color: Rgb(120, 60, 60),
            kind: SegmentKind::Stitch,
            thread_kind: kind,
            thread_weight: weight,
        };
        let span = |m: &Mesh| {
            // The thread (not its shadow): the vertices after the first four.
            let ys: Vec<f32> = m.vertices[4..].iter().map(|v| v.pos.y).collect();
            ys.iter().copied().fold(f32::MIN, f32::max) - ys.iter().copied().fold(f32::MAX, f32::min)
        };
        let mesh = |kind, weight| {
            let s = seg(kind, weight);
            thread_mesh_3d(r, &v, &[(s, color32(s.color))], false, &table)
        };
        let (w40, w60) = (
            span(&mesh(ThreadKind::Polyester, 40)),
            span(&mesh(ThreadKind::Polyester, 60)),
        );
        assert!((w60 / w40 - 0.816).abs() < 0.01, "{w40} {w60}");
        let flat = |weight| {
            let s = seg(ThreadKind::Polyester, weight);
            span(&thread_mesh(r, &v, &[(s, color32(s.color))], false))
        };
        assert!((flat(60) / flat(40) - 0.816).abs() < 0.01);
        // A matte thread has no highlight: no point is lighter than the
        // color with the light of the shading alone.
        let brightest = |m: &Mesh| m.vertices[4..].iter().map(|v| v.color.r()).max().unwrap();
        let (shiny, matte) = (
            brightest(&mesh(ThreadKind::Polyester, 40)),
            brightest(&mesh(ThreadKind::Cotton, 40)),
        );
        assert!(shiny > matte + 5, "{shiny} {matte}");
        let most = table
            .cell(table.index(1.0, 0.0))
            .iter()
            .map(|&(f, _)| f)
            .fold(0.0f32, f32::max);
        assert!(f32::from(matte) <= (120.0 * most).ceil(), "{matte} {most}");
        assert!(brightest(&mesh(ThreadKind::Metallic, 40)) > shiny);
    }

    #[test]
    fn a_mesh_that_is_too_large_gets_a_lighter_look() {
        let r = rect();
        let v = CanvasView {
            center: Point::ZERO,
            zoom: 8.0,
        };
        let table = emb_render::thread3d::ShadeTable::new(Default::default());
        let seg = |x: f64| {
            (
                Segment {
                    a: Point::new(x, -2.0),
                    b: Point::new(x, 2.0),
                    color: Rgb(120, 120, 120),
                    kind: SegmentKind::Stitch,
                    ..Default::default()
                },
                Color32::GRAY,
            )
        };
        // 100 stitches that show, and 100 far away.
        let segs: Vec<_> = (0..100)
            .map(|i| seg(-20.0 + 0.4 * f64::from(i)))
            .chain((0..100).map(|i| seg(5000.0 + f64::from(i))))
            .collect();
        let fit = |max: usize| fitted_mesh_with(r, &v, &segs, false, MeshLook::Thread3d, &table, max);
        let (m, look) = fit(10_000);
        assert_eq!(look, MeshLook::Thread3d);
        assert!(m.vertices.len() <= 1900, "{}", m.vertices.len());
        assert_eq!(fit(1900).1, MeshLook::Thread3d, "the stitches far away do not count");
        assert_eq!(fit(1000).1, MeshLook::Thread);
        let (all, look) = fit(400);
        assert_eq!(look, MeshLook::Stitches);
        assert!((100..=400).contains(&all.vertices.len()), "{}", all.vertices.len());
        // Too many for the lightest look: each second stitch.
        let (m, look) = fit(200);
        assert_eq!(look, MeshLook::Stitches);
        assert!(m.vertices.len() <= 200 && m.vertices.len() < all.vertices.len());
        assert!(
            fitted_mesh(r, &v, &segs, false, MeshLook::Thread3d, &table)
                .0
                .vertices
                .len()
                > 1000
        );
    }

    #[test]
    fn the_3d_mesh_is_lit_from_the_light() {
        let r = rect();
        let v = CanvasView {
            center: Point::ZERO,
            zoom: 8.0,
        };
        let seg = Segment {
            a: Point::new(-2.0, 0.0),
            b: Point::new(2.0, 0.0),
            color: Rgb(120, 120, 120),
            kind: SegmentKind::Stitch,
            ..Default::default()
        };
        let table = emb_render::thread3d::ShadeTable::new(Default::default());
        let mesh = thread_mesh_3d(r, &v, &[(seg, color32(seg.color))], false, &table);
        // A shadow quad and 15 thread vertices.
        assert_eq!(mesh.vertices.len(), 4 + 15);
        // The middle row: the top edge (s = 1, up on the screen) is lit
        // (the light is at the top left), the bottom edge is in shade.
        let row = &mesh.vertices[4 + 5..4 + 10];
        assert!(row[4].pos.y < row[0].pos.y, "s = 1 is up");
        assert!(row[4].color.r() > row[0].color.r() + 20);
        // At a small zoom: the 2D look.
        let far = CanvasView {
            center: Point::ZERO,
            zoom: 0.05,
        };
        let m = thread_mesh_3d(r, &far, &[(seg, color32(seg.color))], false, &table);
        assert_eq!(
            m.vertices.len(),
            thread_mesh(r, &far, &[(seg, color32(seg.color))], false).vertices.len()
        );
    }

    #[test]
    fn pan_wrap_goes_to_the_other_side() {
        let r = Rect::from_min_max(Pos2::new(100.0, 50.0), Pos2::new(900.0, 650.0));
        assert_eq!(pan_wrap(r, Pos2::new(500.0, 300.0)), None);
        let to = pan_wrap(r, Pos2::new(101.0, 300.0)).unwrap();
        assert!(to.x > 880.0 && to.x < 900.0 && to.y == 300.0, "{to:?}");
        let to = pan_wrap(r, Pos2::new(500.0, 649.0)).unwrap();
        assert!(to.y > 50.0 && to.y < 70.0 && to.x == 500.0, "{to:?}");
        // A corner: both.
        let to = pan_wrap(r, Pos2::new(899.0, 51.0)).unwrap();
        assert!(to.x < 120.0 && to.y > 630.0, "{to:?}");
        // The jump is taken out of the movement once; a small move stays.
        let mut jump = Some((Vec2::new(780.0, 0.0), JUMP_FRAMES));
        assert_eq!(take_jump(Vec2::new(-4.0, 1.0), &mut jump), Vec2::new(-4.0, 1.0));
        assert!(jump.is_some());
        assert_eq!(take_jump(Vec2::new(774.0, 1.0), &mut jump), Vec2::new(-6.0, 1.0));
        assert!(jump.is_none());
        // A jump that does not come is forgotten.
        let mut jump = Some((Vec2::new(780.0, 0.0), 2));
        take_jump(Vec2::ZERO, &mut jump);
        take_jump(Vec2::ZERO, &mut jump);
        assert!(jump.is_none());
    }

    #[test]
    fn endless_pan_goes_on_past_the_edge() {
        use eframe::egui::{Event, Modifiers, PointerButton, RawInput, ViewportCommand};
        use emb_model::{HoopCatalog, MachineCatalog};

        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
        // Runs a frame and returns the pointer jump that the frame asked for.
        let run = |st: &mut AppState, events: Vec<Event>| -> Option<Pos2> {
            let input = RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(ui, st));
            });
            out.textures_delta.clear();
            out.viewport_output
                .values()
                .flat_map(|v| v.commands.iter())
                .find_map(|c| match c {
                    ViewportCommand::CursorPosition(p) => Some(*p),
                    _ => None,
                })
        };
        run(&mut st, vec![]);
        let canvas = st.canvas_rect.unwrap();
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let start = canvas.center();
        run(&mut st, vec![Event::PointerMoved(start)]);
        run(&mut st, vec![button(start, true)]);
        let before = st.view.center;
        let scale = st.view.scale();
        // Drag left to the edge, in steps.
        let mut x = start.x;
        let mut jump = None;
        while jump.is_none() && x > canvas.left() - 10.0 {
            x -= 25.0;
            jump = run(
                &mut st,
                vec![Event::PointerMoved(Pos2::new(x.max(canvas.left()), start.y))],
            );
        }
        let to = jump.expect("the pointer jumps at the left edge");
        assert!(to.x > canvas.right() - 20.0, "{to:?}");
        let moved = start.x - x.max(canvas.left());
        // The jump comes (as the window system sends it) with 5 more points
        // of movement to the left: the view goes on, with no lurch.
        run(&mut st, vec![Event::PointerMoved(Pos2::new(to.x - 5.0, to.y))]);
        let panned = (st.view.center.x - before.x) * scale;
        assert!(
            (panned - f64::from(moved + 5.0)).abs() < 1.0,
            "{panned} vs {}",
            moved + 5.0
        );
        run(&mut st, vec![button(Pos2::new(to.x - 5.0, to.y), false)]);
        assert!(st.pan_jump.is_none());
        // With the option off, the pointer does not jump.
        st.settings.endless_pan = false;
        run(&mut st, vec![Event::PointerMoved(start), button(start, true)]);
        let mut x = start.x;
        while x > canvas.left() {
            x -= 25.0;
            assert!(
                run(
                    &mut st,
                    vec![Event::PointerMoved(Pos2::new(x.max(canvas.left()), start.y))]
                )
                .is_none()
            );
        }
    }

    #[test]
    fn double_middle_click_zooms_to_extents() {
        use eframe::egui::{Event, Modifiers, PointerButton, RawInput};
        use emb_model::{HoopCatalog, MachineCatalog};

        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        // The test is on the reference machine.
        st.start_on_machine("husqvarna_topaz20", None);
        st.add_sample_objects();
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let run = |st: &mut AppState, events: Vec<Event>| {
            let input = RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(ui, st));
            });
            out.textures_delta.clear();
        };
        // The first frame fits the view.
        run(&mut st, vec![]);
        let fitted = st.view;
        st.view.zoom = 7.0;
        st.view.center = Point::new(500.0, 500.0);

        let p = Pos2::new(500.0, 350.0);
        let click = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        };
        run(&mut st, vec![Event::PointerMoved(p)]);
        run(&mut st, vec![click(true)]);
        run(&mut st, vec![click(false)]);
        run(&mut st, vec![click(true)]);
        run(&mut st, vec![click(false)]);

        assert!(
            (st.view.zoom - fitted.zoom).abs() < 1e-9,
            "{} vs {}",
            st.view.zoom,
            fitted.zoom
        );
        assert!(st.view.center.distance(fitted.center) < 1e-9);
        // The extents include the hoop field (260 mm front to back).
        assert!(st.fit_box().unwrap().height() >= 260.0);
    }
}
