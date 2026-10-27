//! Direct manipulation on the canvas: move, box select, scale and rotate.
//!
//! - Drag a selected or unselected object to move the selection.
//!   Shift keeps the move horizontal or vertical.
//! - Drag on empty canvas to box-select objects that are fully inside.
//!   Ctrl or Shift adds to the selection.
//! - Drag a corner handle to scale in proportion; Shift scales freely.
//!   Drag a side handle to scale in one direction. Alt scales about the center.
//! - Drag the round handle above the box to rotate; Shift snaps to 15°.
//!
//! The math is in design coordinates, so it also works when the design has
//! a design-level rotation. Each drag is one undo step: each frame replaces
//! the objects with their state at drag start plus the total transform,
//! through `Document::edit_merged`, so there is no drift.

use eframe::egui::{self, Align2, Color32, CursorIcon, FontId, Modifiers, Pos2, Rect, Stroke, Vec2};
use emb_geom::{BBox, Point, Transform, math};
use emb_model::edit::{rotate_about, scale_about};
use emb_model::{Design, DesignObject, MergeKey, ObjectId};

use crate::app::AppState;
use crate::canvas::{self, CanvasView};

/// Half size of a scale handle, in screen points.
const HANDLE: f32 = 4.0;
/// Distance of the rotate handle above the selection box, in screen points.
const ROTATE_OFFSET: f32 = 24.0;
/// Smallest size that scaling can make, in mm.
const MIN_SIZE_MM: f64 = 0.5;
const ROTATE_SNAP_DEG: f64 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// `fx`, `fy` are -1, 0 or 1: the side of the box in design coordinates.
    Scale {
        fx: i8,
        fy: i8,
    },
    Rotate,
}

/// The drag in progress on the canvas.
#[derive(Debug, Clone)]
pub enum DragOp {
    Move {
        originals: Vec<DesignObject>,
        start: Point,
        key: MergeKey,
    },
    Scale {
        originals: Vec<DesignObject>,
        bbox: BBox,
        fx: i8,
        fy: i8,
        key: MergeKey,
    },
    Rotate {
        originals: Vec<DesignObject>,
        center: Point,
        start_angle: f64,
        key: MergeKey,
        angle: f64,
    },
    Marquee {
        start: Pos2,
        current: Pos2,
        additive: bool,
    },
    /// Moves the unlocked backdrop picture.
    Backdrop {
        start: Point,
        origin: Point,
        key: MergeKey,
    },
    /// Reshape tool: moves a node (`handle` is `None`) or one of its handles.
    ReshapeNode {
        id: ObjectId,
        /// Path index in the object (see `ObjectKind::paths`).
        path: usize,
        node: emb_geom::nodes::NodeId,
        handle: Option<emb_geom::nodes::Side>,
        /// The path at drag start.
        original: emb_geom::Path,
        key: MergeKey,
    },
    /// Moves a guide (index in `Design::guides`).
    Guide {
        index: usize,
        key: MergeKey,
    },
    /// Reshape tool: moves one end of a satin stitch angle line (rung).
    ReshapeRung {
        id: ObjectId,
        rung: usize,
        /// The end on rail B (else rail A).
        b_end: bool,
        key: MergeKey,
    },
    /// Reshape tool: moves the start or end point.
    ReshapeMarker {
        id: ObjectId,
        which: crate::reshape::Marker,
        key: MergeKey,
    },
    /// Reshape tool on lettering: moves, kerns, turns or scales one letter.
    Letter {
        id: ObjectId,
        /// Index in `Lettering::letters`.
        letter: usize,
        what: crate::letter_edit::LetterDrag,
        start: Point,
        /// The object at drag start.
        original: Box<emb_model::Lettering>,
        key: MergeKey,
    },
    /// Reshape tool on lettering: the radius handle of an arc or circle
    /// baseline (`node` is `None`), or a node of a path baseline.
    Baseline {
        id: ObjectId,
        node: Option<emb_geom::nodes::NodeId>,
        original: Box<emb_model::Lettering>,
        key: MergeKey,
    },
}

/// Maps between screen points and design coordinates (mm, before the
/// design-level rotation).
#[derive(Debug, Clone, Copy)]
pub struct ViewMap {
    pub rect: Rect,
    pub view: CanvasView,
    rot: Transform,
    inv: Transform,
}

impl ViewMap {
    pub fn new(rect: Rect, view: CanvasView, design: &Design) -> Self {
        let rot = design.rotation_transform();
        Self {
            rect,
            view,
            rot,
            inv: rot.inverse().unwrap_or(Transform::IDENTITY),
        }
    }

    pub fn to_design(self, pos: Pos2) -> Point {
        self.inv.apply(self.view.to_mm(self.rect, pos))
    }

    pub fn to_screen(self, p: Point) -> Pos2 {
        self.view.to_screen(self.rect, self.rot.apply(p))
    }

    /// The box `b` (design coordinates) on screen.
    pub fn screen_box(&self, b: BBox) -> Rect {
        let corners = [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)];
        let pts = corners.map(|p| self.to_screen(p));
        Rect::from_points(&pts)
    }
}

fn selection_box(st: &AppState) -> Option<BBox> {
    st.doc.design().bbox_of(st.selection.ids())
}

/// Screen positions of the handles for box `b`.
fn handles(map: &ViewMap, b: BBox) -> Vec<(Handle, Pos2)> {
    let c = b.center();
    let (hw, hh) = (b.width() / 2.0, b.height() / 2.0);
    let mut v = Vec::with_capacity(9);
    for fy in -1i8..=1 {
        for fx in -1i8..=1 {
            if fx == 0 && fy == 0 {
                continue;
            }
            // A line has no height (or width): no handles to scale it in that direction.
            if (fx != 0 && b.width() < 1e-9) || (fy != 0 && b.height() < 1e-9) {
                continue;
            }
            let p = Point::new(c.x + f64::from(fx) * hw, c.y + f64::from(fy) * hh);
            v.push((Handle::Scale { fx, fy }, map.to_screen(p)));
        }
    }
    let sb = map.screen_box(b);
    v.push((Handle::Rotate, Pos2::new(sb.center().x, sb.top() - ROTATE_OFFSET)));
    v
}

/// Returns the handle under `pos`, if a selection exists.
pub fn hit_handle(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<Handle> {
    let b = selection_box(st)?;
    handles(map, b)
        .into_iter()
        .find(|(_, p)| p.distance(pos) <= HANDLE + 4.0)
        .map(|(h, _)| h)
}

fn originals(st: &AppState) -> Vec<DesignObject> {
    let d = st.doc.design();
    st.selection
        .in_sew_order(d)
        .into_iter()
        .filter_map(|id| d.object(id).cloned())
        .collect()
}

fn new_key(st: &mut AppState, what: &str) -> MergeKey {
    st.drag_counter += 1;
    MergeKey::new(format!("canvas-{what}-{}", st.drag_counter))
}

/// Starts a drag that began at `press` (screen).
pub fn begin_drag(st: &mut AppState, map: &ViewMap, press: Pos2, mods: Modifiers) {
    if let Some(h) = hit_handle(st, map, press) {
        let Some(bbox) = selection_box(st) else { return };
        let originals = originals(st);
        st.drag = Some(match h {
            Handle::Scale { fx, fy } => DragOp::Scale {
                originals,
                bbox,
                fx,
                fy,
                key: new_key(st, "scale"),
            },
            Handle::Rotate => {
                let c = bbox.center();
                let p = map.to_design(press) - c;
                DragOp::Rotate {
                    originals,
                    center: c,
                    start_angle: math::to_degrees(math::atan2(p.y, p.x)),
                    key: new_key(st, "rotate"),
                    angle: 0.0,
                }
            }
        });
        return;
    }
    match canvas::pick(st, map.view.to_mm(map.rect, press)) {
        Some(id) => {
            if !st.selection.contains(id) {
                if mods.command || mods.shift {
                    st.selection.toggle(id);
                } else {
                    st.selection.set(id);
                }
            }
            st.drag = Some(DragOp::Move {
                originals: originals(st),
                start: map.to_design(press),
                key: new_key(st, "move"),
            });
        }
        None if crate::backdrop::grabs(st, map.to_design(press)) => {
            let origin = st.doc.design().backdrop.as_ref().map_or(Point::ZERO, |b| b.origin);
            st.drag = Some(DragOp::Backdrop {
                start: map.to_design(press),
                origin,
                key: new_key(st, "backdrop"),
            });
        }
        None => {
            st.drag = Some(DragOp::Marquee {
                start: press,
                current: press,
                additive: mods.command || mods.shift,
            });
        }
    }
}

/// Replaces each object with its state at drag start plus `t`, as one merged undo step.
/// Linked cuts (border traces, shape links) follow the step from the last frame.
fn apply(st: &mut AppState, label: &str, key: &MergeKey, originals: &[DesignObject], t: &Transform) {
    let last = match &st.drag_step {
        Some((k, last)) if k == key => *last,
        _ => Transform::IDENTITY,
    };
    let step = last.inverse().map_or(*t, |inv| inv.then(*t));
    st.drag_step = Some((key.clone(), *t));
    st.doc.edit_merged(label, key.clone(), |d| {
        let mut ids = Vec::new();
        for o in originals {
            if let Some(m) = d.object_mut(o.id) {
                // `object_mut` gave a new revision; keep it so the stitch cache sees the change.
                let rev = m.revision;
                *m = o.transformed(t);
                m.revision = rev;
                ids.push(o.id);
            }
        }
        d.linked_objects_moved(&ids, &step);
    });
}

/// Scale factors for a handle drag. `p` is the pointer in design coordinates.
fn scale_factors(bbox: BBox, fx: i8, fy: i8, p: Point, mods: Modifiers) -> (Point, f64, f64) {
    let c = bbox.center();
    let (hw, hh) = (bbox.width() / 2.0, bbox.height() / 2.0);
    let handle = Point::new(c.x + f64::from(fx) * hw, c.y + f64::from(fy) * hh);
    let anchor = if mods.alt {
        c
    } else {
        Point::new(c.x - f64::from(fx) * hw, c.y - f64::from(fy) * hh)
    };
    let factor = |p: f64, h: f64, a: f64, size: f64| {
        let span = h - a;
        if span.abs() < 1e-12 {
            return 1.0;
        }
        let s = (p - a) / span;
        let min = MIN_SIZE_MM / size.max(1e-9);
        s.max(min)
    };
    let mut sx = if fx != 0 {
        factor(p.x, handle.x, anchor.x, bbox.width())
    } else {
        1.0
    };
    let mut sy = if fy != 0 {
        factor(p.y, handle.y, anchor.y, bbox.height())
    } else {
        1.0
    };
    if fx != 0 && fy != 0 && !mods.shift {
        // Corner: same factor on both axes, from the projection on the diagonal.
        let diag = handle - anchor;
        let s =
            ((p - anchor).dot(diag) / diag.length_sq()).max(MIN_SIZE_MM / bbox.width().min(bbox.height()).max(1e-9));
        sx = s;
        sy = s;
    }
    (anchor, sx, sy)
}

/// Updates the drag for pointer `pos` (screen).
pub fn update_drag(st: &mut AppState, map: &ViewMap, pos: Pos2, mods: Modifiers) {
    let Some(mut op) = st.drag.take() else { return };
    let p = map.to_design(pos);
    match &mut op {
        DragOp::Move { originals, start, key } => {
            // With snap on, the move goes in grid steps and stops on guides.
            let mut d = st.snap(map, p) - st.snap(map, *start);
            if mods.shift {
                if d.x.abs() >= d.y.abs() {
                    d.y = 0.0;
                } else {
                    d.x = 0.0;
                }
            }
            apply(st, "Move", key, originals, &Transform::translate(d.x, d.y));
        }
        DragOp::Scale {
            originals,
            bbox,
            fx,
            fy,
            key,
        } => {
            let (anchor, sx, sy) = scale_factors(*bbox, *fx, *fy, p, mods);
            apply(st, "Scale", key, originals, &scale_about(anchor, sx, sy));
        }
        DragOp::Rotate {
            originals,
            center,
            start_angle,
            key,
            angle,
        } => {
            let v = p - *center;
            let mut a = math::to_degrees(math::atan2(v.y, v.x)) - *start_angle;
            a = (a + 180.0).rem_euclid(360.0) - 180.0;
            if mods.shift {
                a = (a / ROTATE_SNAP_DEG).round() * ROTATE_SNAP_DEG;
            }
            *angle = a;
            apply(st, "Rotate", key, originals, &rotate_about(*center, a));
        }
        DragOp::Marquee { current, .. } => *current = pos,
        DragOp::ReshapeNode { .. }
        | DragOp::ReshapeMarker { .. }
        | DragOp::ReshapeRung { .. }
        | DragOp::Letter { .. }
        | DragOp::Baseline { .. }
        | DragOp::Guide { .. } => {}
        DragOp::Backdrop { start, origin, key } => {
            let to = *origin + (p - *start);
            st.doc.edit_merged("Move backdrop", key.clone(), |d| {
                if let Some(b) = d.backdrop.as_mut() {
                    b.origin = to;
                }
            });
        }
    }
    st.drag = Some(op);
}

/// Ends the drag.
pub fn end_drag(st: &mut AppState, map: &ViewMap) {
    let Some(op) = st.drag.take() else { return };
    match op {
        DragOp::Move { .. }
        | DragOp::Scale { .. }
        | DragOp::Rotate { .. }
        | DragOp::Backdrop { .. }
        | DragOp::ReshapeNode { .. }
        | DragOp::ReshapeMarker { .. }
        | DragOp::ReshapeRung { .. }
        | DragOp::Letter { .. }
        | DragOp::Baseline { .. }
        | DragOp::Guide { .. } => st.doc.end_merge(),
        DragOp::Marquee {
            start,
            current,
            additive,
        } => {
            let a = map.to_design(start);
            let b = map.to_design(current);
            let Some(area) = BBox::from_points([a, b]) else { return };
            // As in CAD programs: a box dragged to the right selects the
            // objects that are all inside it; a box dragged to the left
            // (a crossing box) also selects the objects that it touches.
            let crossing = is_crossing(start, current);
            let inside: Vec<ObjectId> = st
                .doc
                .design()
                .objects()
                .iter()
                .filter(|o| !o.locked && st.view_filter.shows(o))
                .filter(|o| match crossing {
                    true => touches_box(o, &area),
                    false => o.control_bbox().is_some_and(|ob| area.contains_box(&ob)),
                })
                .map(|o| o.id)
                .collect();
            if !additive {
                st.selection.clear();
            }
            for id in inside {
                if !st.selection.contains(id) {
                    st.selection.toggle(id);
                }
            }
        }
    }
}

/// A selection box dragged from right to left is a crossing box.
fn is_crossing(start: Pos2, current: Pos2) -> bool {
    current.x < start.x
}

/// Returns `true` if the object touches the box: a line of it goes into
/// the box, or the box is inside its area.
fn touches_box(o: &DesignObject, area: &BBox) -> bool {
    let Some(ob) = o.control_bbox() else { return false };
    if !ob.intersects(area) {
        return false;
    }
    if area.contains_box(&ob) || o.hit_test(area.center(), 0.0) {
        return true;
    }
    o.kind.paths().any(|path| {
        path.flatten(0.05)
            .iter()
            .any(|pl| pl.segments().any(|(p, q)| segment_touches_box(p, q, area)))
    })
}

/// Returns `true` if the segment from `p` to `q` has a point in the box
/// (Liang-Barsky clipping).
fn segment_touches_box(p: Point, q: Point, b: &BBox) -> bool {
    let d = q - p;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (den, num) in [
        (-d.x, p.x - b.min.x),
        (d.x, b.max.x - p.x),
        (-d.y, p.y - b.min.y),
        (d.y, b.max.y - p.y),
    ] {
        if den == 0.0 {
            if num < 0.0 {
                return false;
            }
        } else {
            let t = num / den;
            if den < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Sets the mouse cursor for the pointer position when no drag is active.
pub fn hover_cursor(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<CursorIcon> {
    if let Some(h) = hit_handle(st, map, pos) {
        let b = selection_box(st)?;
        return Some(match h {
            Handle::Rotate => CursorIcon::Alias,
            Handle::Scale { .. } => {
                // Use the screen direction from the box center, so the cursor is right for a rotated design.
                let c = map.to_screen(b.center());
                let d = handles(map, b).into_iter().find(|(hh, _)| *hh == h)?.1 - c;
                if d.x.abs() < 1.0 {
                    CursorIcon::ResizeVertical
                } else if d.y.abs() < 1.0 {
                    CursorIcon::ResizeHorizontal
                } else if (d.x > 0.0) == (d.y > 0.0) {
                    CursorIcon::ResizeNwSe
                } else {
                    CursorIcon::ResizeNeSw
                }
            }
        });
    }
    let Some(id) = canvas::pick(st, map.view.to_mm(map.rect, pos)) else {
        return crate::backdrop::grabs(st, map.to_design(pos)).then_some(CursorIcon::Grab);
    };
    st.selection.contains(id).then_some(CursorIcon::Move)
}

/// Draws the selection box, its handles, the marquee and the live values.
pub fn draw_overlay(painter: &egui::Painter, map: &ViewMap, st: &AppState, accent: Color32, text_bg: Color32) {
    if let Some(DragOp::Marquee { start, current, .. }) = &st.drag {
        let r = Rect::from_two_pos(*start, *current);
        if st.tool != crate::draw::Tool::Zoom && is_crossing(*start, *current) {
            // A crossing box: green and dashed, as in CAD programs.
            let green = Color32::from_rgb(60, 170, 90);
            painter.rect_filled(r, 0.0, green.gamma_multiply(0.08));
            let pts = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            painter.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.0, green), 5.0, 3.0));
        } else {
            painter.rect_filled(r, 0.0, accent.gamma_multiply(0.08));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Inside);
        }
        return;
    }
    let Some(b) = selection_box(st) else { return };
    let sb = map.screen_box(b).expand(2.0);
    let pts = [
        sb.left_top(),
        sb.right_top(),
        sb.right_bottom(),
        sb.left_bottom(),
        sb.left_top(),
    ];
    painter.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.0, accent), 5.0, 3.0));

    let hs = handles(map, b);
    if let Some((_, rot)) = hs.iter().find(|(h, _)| *h == Handle::Rotate) {
        painter.line_segment([Pos2::new(rot.x, sb.top()), *rot], Stroke::new(1.0, accent));
        painter.circle_filled(*rot, HANDLE + 1.0, Color32::WHITE);
        painter.circle_stroke(*rot, HANDLE + 1.0, Stroke::new(1.5, accent));
    }
    for (h, p) in &hs {
        if let Handle::Scale { .. } = h {
            let r = Rect::from_center_size(*p, Vec2::splat(HANDLE * 2.0));
            painter.rect_filled(r, 1.0, Color32::WHITE);
            painter.rect_stroke(r, 1.0, Stroke::new(1.5, accent), egui::StrokeKind::Inside);
        }
    }

    // Live values while dragging, and the size at rest.
    let text = match &st.drag {
        Some(DragOp::Rotate { angle, .. }) => format!("{angle:+.1}°"),
        Some(DragOp::Move { originals, .. }) => {
            let ids: Vec<ObjectId> = originals.iter().map(|o| o.id).collect();
            let start = originals
                .iter()
                .filter_map(DesignObject::control_bbox)
                .reduce(BBox::union);
            match (start, st.doc.design().bbox_of(&ids)) {
                (Some(s), Some(n)) => {
                    let d = n.center() - s.center();
                    format!("Δ {:+.1}, {:+.1} mm", d.x, d.y)
                }
                _ => String::new(),
            }
        }
        _ => format!("{:.1} × {:.1} mm", b.width(), b.height()),
    };
    if !text.is_empty() {
        let galley = painter.layout_no_wrap(text, FontId::proportional(12.0), accent);
        let r = Align2::CENTER_TOP.anchor_size(Pos2::new(sb.center().x, sb.bottom() + 6.0), galley.size());
        painter.rect_filled(r.expand(3.0), 3.0, text_bg);
        painter.galley(r.min, galley, accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bb() -> BBox {
        BBox::new(Point::ZERO, Point::new(10.0, 20.0))
    }

    #[test]
    fn side_handle_scales_one_axis_from_opposite_side() {
        let (anchor, sx, sy) = scale_factors(bb(), 1, 0, Point::new(20.0, 5.0), Modifiers::NONE);
        assert_eq!(anchor, Point::new(0.0, 10.0));
        assert_eq!((sx, sy), (2.0, 1.0));
    }

    #[test]
    fn corner_is_proportional_unless_shift() {
        let (_, sx, sy) = scale_factors(bb(), 1, 1, Point::new(20.0, 40.0), Modifiers::NONE);
        assert!((sx - 2.0).abs() < 1e-12 && (sy - 2.0).abs() < 1e-12);
        let (_, sx, sy) = scale_factors(bb(), 1, 1, Point::new(20.0, 30.0), Modifiers::SHIFT);
        assert_eq!((sx, sy), (2.0, 1.5));
    }

    #[test]
    fn alt_scales_about_center_and_size_has_a_minimum() {
        let (anchor, sx, _) = scale_factors(bb(), 1, 0, Point::new(15.0, 0.0), Modifiers::ALT);
        assert_eq!(anchor, Point::new(5.0, 10.0));
        assert_eq!(sx, 2.0);
        // Dragging past the anchor does not flip or collapse the shape.
        let (_, sx, _) = scale_factors(bb(), 1, 0, Point::new(-50.0, 0.0), Modifiers::NONE);
        assert!((sx * 10.0 - MIN_SIZE_MM).abs() < 1e-12);
    }

    // ---- Headless canvas tests with real pointer events ----

    use eframe::egui::PointerButton;
    use emb_model::ObjectKind;

    use crate::test_util::Harness;

    #[test]
    fn drag_an_unselected_object_moves_it_as_one_undo_step() {
        let mut h = Harness::new();
        let fill = h.id(0);
        let c = h.st.doc.design().hoop.center;
        // Inside the red fill, away from the running line and the satin ring.
        let from = h.screen(c + Point::new(0.0, 25.0));
        let before = h.center_of(fill);
        let steps = h.st.doc.undo_len();
        h.drag(
            PointerButton::Primary,
            from,
            from + Vec2::new(100.0, 0.0),
            Modifiers::NONE,
        );

        assert_eq!(h.st.selection.single(), Some(fill));
        let moved = h.center_of(fill) - before;
        let expected = 100.0 / h.st.view.scale();
        assert!((moved.x - expected).abs() < 0.05 && moved.y.abs() < 1e-9, "{moved:?}");
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert_eq!(h.st.doc.undo_label(), Some("Move"));
        h.st.undo();
        assert!(h.center_of(fill).distance(before) < 1e-9);
    }

    #[test]
    fn drag_on_empty_canvas_box_selects() {
        let mut h = Harness::new();
        let c = h.st.doc.design().hoop.center;
        let from = h.screen(c + Point::new(-40.0, -40.0));
        let to = h.screen(c + Point::new(38.0, 40.0));
        let steps = h.st.doc.undo_len();
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        // The first patch (fill, satin ring, running line) is fully inside; the second is not.
        let sel = h.st.selection.in_sew_order(h.st.doc.design());
        assert_eq!(sel, vec![h.id(0), h.id(1), h.id(2)]);
        assert_eq!(h.st.doc.undo_len(), steps, "box select is not an edit");
    }

    #[test]
    fn drag_to_the_left_selects_what_the_box_touches() {
        let mut h = Harness::new();
        let (fill, ring) = (h.id(0), h.id(1));
        let c = h.st.doc.design().hoop.center;
        // A box over the upper right edge of the round fill and its ring.
        // Both drags start on empty canvas.
        let at = |x: f64, y: f64| h.screen(c + Point::new(x, y));
        let (top_left, bottom_right) = (at(10.0, -40.0), at(40.0, -10.0));
        let (top_right, bottom_left) = (at(40.0, -40.0), at(10.0, -10.0));
        // Left to right: nothing is all inside the box.
        h.drag(PointerButton::Primary, top_left, bottom_right, Modifiers::NONE);
        assert!(h.st.selection.is_empty(), "{:?}", h.st.selection.ids());
        // Right to left: the box touches the fill and the ring.
        h.drag(PointerButton::Primary, top_right, bottom_left, Modifiers::NONE);
        let sel = h.st.selection.in_sew_order(h.st.doc.design());
        assert!(sel.contains(&fill) && sel.contains(&ring), "{sel:?}");
    }

    #[test]
    fn crossing_test_of_one_object() {
        let h = Harness::new();
        let fill = h.st.doc.design().object(h.id(0)).cloned().unwrap();
        let b = fill.control_bbox().unwrap();
        // A small box in the middle of the fill touches it (its area).
        assert!(touches_box(&fill, &BBox::from_center_size(b.center(), 1.0, 1.0)));
        // A box that only crosses the edge touches it.
        let edge = BBox::new(
            Point::new(b.min.x - 1.0, b.center().y),
            Point::new(b.min.x + 1.0, b.center().y + 1.0),
        );
        assert!(touches_box(&fill, &edge));
        // A box outside it does not.
        let away = BBox::new(
            Point::new(b.max.x + 5.0, b.max.y + 5.0),
            Point::new(b.max.x + 6.0, b.max.y + 6.0),
        );
        assert!(!touches_box(&fill, &away));
        assert!(segment_touches_box(
            Point::new(-5.0, 0.5),
            Point::new(5.0, 0.5),
            &BBox::new(Point::ZERO, Point::new(1.0, 1.0))
        ));
        assert!(!segment_touches_box(
            Point::new(-5.0, 2.0),
            Point::new(5.0, 2.0),
            &BBox::new(Point::ZERO, Point::new(1.0, 1.0))
        ));
    }

    #[test]
    fn right_click_selects_the_object_under_the_pointer() {
        let mut h = Harness::new();
        let star = h.id(3);
        let star_center = h.center_of(star);
        h.st.selection.set(h.id(0));
        h.click(PointerButton::Secondary, h.screen(star_center));
        assert_eq!(h.st.selection.single(), Some(star));
        // Right click on empty canvas clears the selection.
        let empty = h.screen(h.st.doc.design().hoop.center + Point::new(-100.0, -80.0));
        h.click(PointerButton::Secondary, empty);
        assert!(h.st.selection.is_empty());
    }

    #[test]
    fn rotate_handle_turns_shape_and_fill_angle() {
        let mut h = Harness::new();
        let fill = h.id(0);
        h.st.selection.set(fill);
        h.frame(vec![]);
        let map = h.map();
        let b = h.st.doc.design().bbox_of(&[fill]).unwrap();
        let (_, handle) = handles(&map, b)
            .into_iter()
            .find(|(k, _)| *k == Handle::Rotate)
            .unwrap();
        let c = map.to_screen(b.center());
        let r = handle.distance(c);
        // From straight up to straight right: a quarter turn clockwise. Shift snaps to 15°.
        h.drag(PointerButton::Primary, handle, c + Vec2::new(r, 0.0), Modifiers::SHIFT);
        let ObjectKind::Fill { params, .. } = &h.st.doc.design().object(fill).unwrap().kind else {
            panic!()
        };
        assert_eq!(params.angle_deg, 135.0);
        assert_eq!(h.st.doc.undo_label(), Some("Rotate"));
        assert!(h.center_of(fill).distance(b.center()) < 1e-9);
    }

    #[test]
    fn side_handle_drag_scales_width_only() {
        let mut h = Harness::new();
        let star = h.id(3);
        h.st.selection.set(star);
        h.frame(vec![]);
        let map = h.map();
        let b = h.st.doc.design().bbox_of(&[star]).unwrap();
        let (_, right) = handles(&map, b)
            .into_iter()
            .find(|(k, _)| *k == Handle::Scale { fx: 1, fy: 0 })
            .unwrap();
        let grow_px = (b.width() * h.st.view.scale()) as f32;
        h.drag(
            PointerButton::Primary,
            right,
            right + Vec2::new(grow_px, 0.0),
            Modifiers::NONE,
        );
        let nb = h.st.doc.design().bbox_of(&[star]).unwrap();
        assert!(
            (nb.width() - 2.0 * b.width()).abs() < 0.05,
            "{} vs {}",
            nb.width(),
            b.width()
        );
        assert!((nb.height() - b.height()).abs() < 1e-9);
        assert!((nb.min.x - b.min.x).abs() < 1e-9, "left side stays");
    }

    #[test]
    fn scaling_lettering_changes_its_height() {
        let mut h = Harness::empty();
        h.st.add_lettering(crate::lettering::LetteringDialog::new(
            Point::new(-20.0, 5.0),
            "Anna",
            emb_model::FontRef::embroidery("Block Sans", ""),
            10.0,
        ));
        let id = h.id(0);
        h.st.selection.set(id);
        h.st.zoom_to = Some(BBox::new(Point::new(-40.0, -30.0), Point::new(40.0, 30.0)));
        h.frame(vec![]);
        let map = h.map();
        let b = h.st.doc.design().bbox_of(&[id]).unwrap();
        let (_, corner) = handles(&map, b)
            .into_iter()
            .find(|(k, _)| *k == Handle::Scale { fx: 1, fy: 1 })
            .unwrap();
        let grow = Vec2::new(
            (b.width() * h.st.view.scale()) as f32,
            (b.height() * h.st.view.scale()) as f32,
        );
        h.drag(PointerButton::Primary, corner, corner + grow, Modifiers::NONE);
        let emb_model::ObjectKind::Lettering(l) = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert!((l.height_mm - 20.0).abs() < 0.1, "twice the size: {}", l.height_mm);
        h.st.undo();
        let emb_model::ObjectKind::Lettering(l) = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert_eq!(l.height_mm, 10.0);
    }
}
