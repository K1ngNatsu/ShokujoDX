//! Lettering in the Reshape tool (P4 Milestone 3, ADR 0017): move, turn,
//! scale and kern single letters, change the shape of one letter, and
//! change the baseline.
//!
//! - Each letter has a diamond on its baseline. Drag it to move the letter.
//!   Shift+drag: change the space before the letter (kerning); the letters
//!   after it on the line move too. Click it to select the letter.
//! - The selected letter shows its box, a turn handle (a circle above the
//!   box), a size handle (a square at the top right corner) and the nodes
//!   of its parts (see `reshape`). A letter with changed nodes keeps its new
//!   shape; Object Properties can reset it.
//! - An arc baseline has a handle at the center of its circle, a circle
//!   baseline a handle on the circle: drag it to change the radius. A path
//!   baseline shows its nodes: drag them to change the path.
//!
//! Each drag is one undo step (see `manipulate` for the method).

use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Stroke, Vec2};
use emb_geom::nodes::{NodeId, NodePath};
use emb_geom::{Point, math};
use emb_model::baseline::MIN_RADIUS_MM;
use emb_model::{Baseline, LetterEdit, Lettering, MergeKey, ObjectId, ObjectKind};

use crate::app::AppState;
use crate::manipulate::{DragOp, ViewMap};

/// Hit distance for the handles, in screen points.
const HIT: f32 = 7.0;
const DIAMOND: f32 = 5.0;

/// What a drag on a letter changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterDrag {
    Move,
    Kern,
    Turn,
    Size,
}

/// What is under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    /// The diamond of a letter (index in `Lettering::letters`).
    Letter(usize),
    Turn(usize),
    Size(usize),
    /// The radius handle of an arc or circle baseline.
    Radius,
    /// A node of a path baseline.
    BaselineNode(NodeId),
}

/// Runs `f` with the selected lettering object.
fn with_target<R>(st: &AppState, f: impl FnOnce(ObjectId, &Lettering) -> R) -> Option<R> {
    let id = st.selection.single()?;
    match &st.doc.design().object(id)?.kind {
        ObjectKind::Lettering(l) => Some(f(id, l)),
        _ => None,
    }
}

/// The selected letter of the selected lettering object.
pub fn selected_letter(st: &AppState) -> Option<usize> {
    let (id, i) = st.reshape.letter?;
    (st.selection.single() == Some(id)).then_some(i)
}

/// The status bar text of the Reshape tool on lettering.
pub fn prompt(st: &AppState) -> Option<&'static str> {
    if st.tool != crate::draw::Tool::Reshape {
        return None;
    }
    with_target(st, |_, _| {
        if selected_letter(st).is_some() {
            "Drag the circle to turn the letter, the square to scale it, its nodes to change its \
             shape. Diamonds: drag to move a letter, Shift+drag to kern."
        } else {
            "Click a diamond to select a letter. Drag it to move the letter, Shift+drag to kern \
             (the letters after it move too)."
        }
    })
}

/// The diamond of letter `i`: its anchor on the baseline, in the design.
fn anchor(l: &Lettering, i: usize) -> Option<Point> {
    let c = l.letters.get(i)?.center;
    l.letter_point(i, Point::new(c.x, 0.0))
}

/// The corners of the box of letter `i`, in the design.
fn letter_box(l: &Lettering, i: usize) -> Option<[Point; 4]> {
    let c = l.letters.get(i)?.center;
    let (w, h) = (2.0 * c.x, 2.0 * c.y);
    let p = |x: f64, y: f64| l.letter_point(i, Point::new(x, y));
    Some([p(0.0, 0.0)?, p(w, 0.0)?, p(w, h)?, p(0.0, h)?])
}

fn turn_handle(l: &Lettering, i: usize) -> Option<Point> {
    let c = l.letters.get(i)?.center;
    l.letter_point(i, Point::new(c.x, 2.6 * c.y))
}

fn size_handle(l: &Lettering, i: usize) -> Option<Point> {
    letter_box(l, i).map(|b| b[2])
}

/// The radius handle: the center of an arc, a point on a circle.
fn radius_handle(l: &Lettering) -> Option<Point> {
    let p = match l.baseline {
        Baseline::Arc { .. } => l.baseline.center()?,
        Baseline::Circle { .. } => l.baseline.frame(0.0, 0.0, 0.5).0,
        _ => return None,
    };
    Some(l.placement.apply(p))
}

/// The path of a path baseline, in the design.
fn baseline_nodes(l: &Lettering) -> Option<NodePath> {
    match &l.baseline {
        Baseline::Path { path } => Some(NodePath::from_path(&path.transformed(&l.placement))),
        _ => None,
    }
}

fn hit(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<Hit> {
    let sel = selected_letter(st);
    with_target(st, |_, l| {
        let near = |p: Option<Point>| p.is_some_and(|p| map.to_screen(p).distance(pos) <= HIT);
        if let Some(i) = sel {
            if near(turn_handle(l, i)) {
                return Some(Hit::Turn(i));
            }
            if near(size_handle(l, i)) {
                return Some(Hit::Size(i));
            }
        }
        if near(radius_handle(l)) {
            return Some(Hit::Radius);
        }
        if let Some(np) = baseline_nodes(l)
            && let Some((n, _)) = np.nodes().find(|(_, p)| near(Some(*p)))
        {
            return Some(Hit::BaselineNode(n));
        }
        (0..l.letters.len())
            .rev()
            .find(|&i| near(anchor(l, i)))
            .map(Hit::Letter)
    })
    .flatten()
}

/// Handles canvas input on the handles of a selected lettering object.
/// Returns `true` if it used the input (the Reshape tool then does nothing
/// more with it).
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) -> bool {
    let Some(id) = with_target(st, |id, _| id) else {
        return false;
    };
    let mods = ui.input(|i| i.modifiers);
    let hover = response.hover_pos().and_then(|p| hit(st, map, p));
    if st.drag.is_none() && hover.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        && let Some(h) = hit(st, map, origin)
        && let Some(original) = with_target(st, |_, l| Box::new(l.clone()))
    {
        st.drag_counter += 1;
        let key = MergeKey::new(format!("letter-{}", st.drag_counter));
        let start = map.to_design(origin);
        let letter = |letter: usize, what: LetterDrag| DragOp::Letter {
            id,
            letter,
            what,
            start,
            original: original.clone(),
            key: key.clone(),
        };
        st.drag = Some(match h {
            Hit::Letter(i) => {
                st.reshape.letter = Some((id, i));
                st.reshape.node = None;
                letter(i, if mods.shift { LetterDrag::Kern } else { LetterDrag::Move })
            }
            Hit::Turn(i) => letter(i, LetterDrag::Turn),
            Hit::Size(i) => letter(i, LetterDrag::Size),
            Hit::Radius => DragOp::Baseline {
                id,
                node: None,
                original,
                key,
            },
            Hit::BaselineNode(n) => DragOp::Baseline {
                id,
                node: Some(n),
                original,
                key,
            },
        });
        return true;
    }
    let ours = matches!(st.drag, Some(DragOp::Letter { .. } | DragOp::Baseline { .. }));
    if ours {
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            st.drag_letter_to(map.to_design(pos), mods.shift);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            st.drag = None;
            st.doc.end_merge();
        }
        return true;
    }
    if let Some(h) = hover {
        if response.clicked()
            && let Hit::Letter(i) = h
        {
            st.reshape.letter = Some((id, i));
            st.reshape.node = None;
        }
        return response.clicked() || response.double_clicked();
    }
    false
}

/// The angle of `p` about `c`, in degrees.
fn angle(c: Point, p: Point) -> f64 {
    let d = p - c;
    math::to_degrees(math::atan2(d.y, d.x))
}

impl AppState {
    /// Applies a letter or baseline drag at design point `p`. Shift turns in
    /// steps of 15 degrees.
    fn drag_letter_to(&mut self, p: Point, shift: bool) {
        let (id, mut n, label, key, layout) = match self.drag.clone() {
            Some(DragOp::Letter {
                id,
                letter,
                what,
                start,
                original,
                key,
            }) => {
                let mut n = *original;
                let Some(l) = n.letters.get(letter).cloned() else {
                    return;
                };
                let Some(inv) = l.frame.then(n.placement).inverse() else {
                    return;
                };
                let e0 = n.edit(l.index).cloned().unwrap_or(LetterEdit::new(l.index));
                let Some(pivot) = n.letter_point(letter, l.center) else {
                    return;
                };
                let mirrored = n.placement.a * n.placement.d - n.placement.b * n.placement.c < 0.0;
                let label = match what {
                    LetterDrag::Move => {
                        let d = inv.apply_vector(p - start);
                        n.change_letter(letter, |e| e.offset = e0.offset + d);
                        "Move letter"
                    }
                    LetterDrag::Kern => {
                        let d = inv.apply_vector(p - start);
                        n.edit_mut(l.index).kern_mm = e0.kern_mm + d.x;
                        n.prune_edits();
                        "Kern letters"
                    }
                    LetterDrag::Turn => {
                        let mut delta = angle(pivot, p) - angle(pivot, start);
                        if mirrored {
                            delta = -delta;
                        }
                        let mut deg = (e0.rotate_deg + delta + 180.0).rem_euclid(360.0) - 180.0;
                        if shift {
                            deg = (deg / 15.0).round() * 15.0;
                        }
                        n.change_letter(letter, |e| e.rotate_deg = deg);
                        "Turn letter"
                    }
                    LetterDrag::Size => {
                        let d0 = start.distance(pivot);
                        if d0 < 1e-6 {
                            return;
                        }
                        let s = (e0.scale * p.distance(pivot) / d0).clamp(0.1, 10.0);
                        n.change_letter(letter, |e| e.scale = s);
                        "Scale letter"
                    }
                };
                (id, n, label, key, what == LetterDrag::Kern)
            }
            Some(DragOp::Baseline {
                id,
                node,
                original,
                key,
            }) => {
                let mut n = *original;
                let Some(inv) = n.placement.inverse() else { return };
                let q = inv.apply(p);
                match (&mut n.baseline, node) {
                    (Baseline::Arc { radius_mm, clockwise }, None) => {
                        *radius_mm = if *clockwise { q.y } else { -q.y }.max(MIN_RADIUS_MM);
                    }
                    (Baseline::Circle { radius_mm, .. }, None) => *radius_mm = q.length().max(MIN_RADIUS_MM),
                    (Baseline::Path { path }, Some(node)) => {
                        let mut np = NodePath::from_path(path);
                        np.move_node(node, q);
                        *path = np.to_path();
                    }
                    _ => return,
                }
                (id, n, "Change baseline", key, true)
            }
            _ => return,
        };
        if layout && !crate::lettering::relayout(self, &mut n) {
            self.message = crate::i18n::fill("{} is not on this computer: choose another font.", &[&(n.font.label())]);
            return;
        }
        self.doc.edit_merged(label, key, |d| {
            if let Some(o) = d.object_mut(id) {
                o.kind = ObjectKind::Lettering(Box::new(n));
            }
        });
    }
}

fn diamond(painter: &egui::Painter, at: Pos2, r: f32, fill: Color32, stroke: Stroke) {
    let pts = vec![
        at + Vec2::new(0.0, -r),
        at + Vec2::new(r, 0.0),
        at + Vec2::new(0.0, r),
        at + Vec2::new(-r, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
}

/// Draws the baseline, the letter diamonds and the handles of the selected letter.
pub fn draw_overlay(painter: &egui::Painter, map: &ViewMap, st: &AppState, accent: Color32) {
    let sel = selected_letter(st);
    with_target(st, |_, l| {
        let thin = Stroke::new(1.0, accent.gamma_multiply(0.7));
        let line = Stroke::new(1.5, accent);
        // The baseline.
        let ring = match &l.baseline {
            Baseline::Straight => None,
            Baseline::Arc { radius_mm, .. } | Baseline::Circle { radius_mm, .. } => {
                let r = radius_mm.max(MIN_RADIUS_MM);
                let half = std::f64::consts::PI * r;
                Some(l.baseline.polyline(-half, half, 0.0, 1.0))
            }
            Baseline::Path { path } => Some(path.flatten(0.05).into_iter().flat_map(|p| p.points).collect()),
        };
        if let Some(points) = ring {
            let pts: Vec<Pos2> = points.iter().map(|p| map.to_screen(l.placement.apply(*p))).collect();
            painter.add(egui::Shape::line(pts, thin));
        }
        if let Some(c) = l.baseline.center() {
            let c = map.to_screen(l.placement.apply(c));
            painter.line_segment([c - Vec2::X * 5.0, c + Vec2::X * 5.0], thin);
            painter.line_segment([c - Vec2::Y * 5.0, c + Vec2::Y * 5.0], thin);
        }
        if let Some(h) = radius_handle(l) {
            let s = map.to_screen(h);
            painter.circle_filled(s, 5.0, accent);
            painter.circle_stroke(s, 5.0, Stroke::new(1.5, Color32::WHITE));
        }
        if let Some(np) = baseline_nodes(l) {
            for (_, p) in np.nodes() {
                let r = Rect::from_center_size(map.to_screen(p), Vec2::splat(8.0));
                painter.rect_filled(r, 0.0, Color32::WHITE);
                painter.rect_stroke(r, 0.0, line, egui::StrokeKind::Inside);
            }
        }
        // The selected letter: box, turn and size handles.
        if let Some(i) = sel
            && let (Some(b), Some(t), Some(s)) = (letter_box(l, i), turn_handle(l, i), size_handle(l, i))
        {
            let pts: Vec<Pos2> = b.iter().map(|p| map.to_screen(*p)).collect();
            painter.add(egui::Shape::closed_line(pts, thin));
            let (t, top) = (map.to_screen(t), map.to_screen(b[2].midpoint(b[3])));
            painter.line_segment([top, t], thin);
            painter.circle_filled(t, 5.0, Color32::WHITE);
            painter.circle_stroke(t, 5.0, line);
            let r = Rect::from_center_size(map.to_screen(s), Vec2::splat(9.0));
            painter.rect_filled(r, 0.0, Color32::WHITE);
            painter.rect_stroke(r, 0.0, line, egui::StrokeKind::Inside);
        }
        for i in 0..l.letters.len() {
            if let Some(a) = anchor(l, i) {
                let fill = if sel == Some(i) { accent } else { Color32::WHITE };
                diamond(painter, map.to_screen(a), DIAMOND, fill, line);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Modifiers, PointerButton};
    use emb_geom::BBox;
    use emb_model::FontRef;

    use super::*;
    use crate::draw::Tool;
    use crate::lettering::LetteringDialog;
    use crate::test_util::Harness;

    /// "Anna" in Block Sans at 20 mm, selected, in the Reshape tool.
    fn anna() -> (Harness, ObjectId) {
        let mut h = Harness::empty();
        h.st.add_lettering(LetteringDialog::new(
            Point::new(-30.0, 10.0),
            "Anna",
            FontRef::embroidery("Block Sans", ""),
            20.0,
        ));
        let id = h.id(0);
        h.st.zoom_to = Some(BBox::new(Point::new(-40.0, -20.0), Point::new(40.0, 20.0)));
        h.frame(vec![]);
        h.st.set_tool(Tool::Reshape);
        h.st.selection.set(id);
        h.frame(vec![]);
        (h, id)
    }

    fn lettering(h: &Harness, id: ObjectId) -> Lettering {
        match &h.st.doc.design().object(id).unwrap().kind {
            ObjectKind::Lettering(l) => (**l).clone(),
            _ => panic!(),
        }
    }

    fn letter_center(l: &Lettering, i: usize) -> Point {
        l.letters[i]
            .parts
            .iter()
            .filter_map(|p| p.control_bbox())
            .reduce(BBox::union)
            .unwrap()
            .center()
    }

    #[test]
    fn drag_a_letter_moves_only_that_letter() {
        let (mut h, id) = anna();
        let before = lettering(&h, id);
        let from = h.screen(anchor(&before, 1).unwrap());
        h.drag(
            PointerButton::Primary,
            from,
            from + Vec2::new(0.0, 40.0),
            Modifiers::NONE,
        );
        let after = lettering(&h, id);
        assert_eq!(h.st.reshape.letter, Some((id, 1)), "the letter is selected");
        let dy = letter_center(&after, 1).y - letter_center(&before, 1).y;
        assert!(dy > 3.0, "{dy}");
        assert!(letter_center(&after, 2).distance(letter_center(&before, 2)) < 1e-6);
        assert!(after.edit(1).is_some_and(|e| e.offset.y > 3.0));
        h.st.undo();
        assert_eq!(lettering(&h, id), before, "one undo step");
    }

    #[test]
    fn shift_drag_kerns_the_letters_after() {
        let (mut h, id) = anna();
        let before = lettering(&h, id);
        let from = h.screen(anchor(&before, 2).unwrap());
        h.drag(
            PointerButton::Primary,
            from,
            from + Vec2::new(40.0, 0.0),
            Modifiers::SHIFT,
        );
        let after = lettering(&h, id);
        let k = after.edit(2).unwrap().kern_mm;
        assert!(k > 3.0, "{k}");
        for i in 2..4 {
            let dx = letter_center(&after, i).x - letter_center(&before, i).x;
            assert!((dx - k).abs() < 1e-6, "{i} {dx} {k}");
        }
        assert!(letter_center(&after, 1).distance(letter_center(&before, 1)) < 1e-6);
    }

    #[test]
    fn turn_and_size_handles() {
        let (mut h, id) = anna();
        h.st.reshape.letter = Some((id, 0));
        let l = lettering(&h, id);
        let pivot = l.letter_to_design(0).unwrap().apply(l.letters[0].center);
        // Turn: the handle above the letter to the right of the middle.
        let t = turn_handle(&l, 0).unwrap();
        let to = Point::new(pivot.x + pivot.distance(t), pivot.y);
        h.drag(PointerButton::Primary, h.screen(t), h.screen(to), Modifiers::NONE);
        let deg = lettering(&h, id).edit(0).unwrap().rotate_deg;
        assert!((deg - 90.0).abs() < 2.0, "{deg}");
        h.st.undo();
        // Size: the corner twice as far from the middle.
        let s = size_handle(&l, 0).unwrap();
        let to = pivot + (s - pivot) * 2.0;
        h.drag(PointerButton::Primary, h.screen(s), h.screen(to), Modifiers::NONE);
        let scale = lettering(&h, id).edit(0).unwrap().scale;
        assert!((scale - 2.0).abs() < 0.05, "{scale}");
    }

    #[test]
    fn reshape_one_letter_and_keep_it() {
        let (mut h, id) = anna();
        h.st.reshape.letter = Some((id, 1));
        h.frame(vec![]);
        let l = lettering(&h, id);
        let k = l.paths_of_letter(1).start;
        let path =
            h.st.doc
                .design()
                .object(id)
                .unwrap()
                .kind
                .paths()
                .nth(k)
                .unwrap()
                .clone();
        let (_, node) = NodePath::from_path(&path).nodes().next().unwrap();
        let from = h.screen(node);
        h.drag(
            PointerButton::Primary,
            from,
            from + Vec2::new(25.0, 0.0),
            Modifiers::NONE,
        );
        let after = lettering(&h, id);
        let moved = after.letters[1].parts.clone();
        assert_ne!(moved, l.letters[1].parts, "the node moved");
        assert_eq!(after.letters[0].parts, l.letters[0].parts);
        assert!(after.edit(1).is_some_and(|e| e.shape.is_some()));
        // A new layout (here: a change of the last letter) keeps the shape.
        let mut n = after.clone();
        n.retarget_edits("Anne");
        n.text = "Anne".into();
        assert!(crate::lettering::relayout(&mut h.st, &mut n));
        let near = n.letters[1].parts.iter().zip(&moved).all(|(a, b)| {
            a.paths()
                .flat_map(|p| p.points().collect::<Vec<_>>())
                .zip(b.paths().flat_map(|p| p.points().collect::<Vec<_>>()))
                .all(|(x, y)| x.distance(y) < 1e-6)
        });
        assert!(near, "the changed letter stays changed");
    }

    #[test]
    fn diamonds_follow_the_envelope() {
        let (mut h, id) = anna();
        let flat = lettering(&h, id);
        let mut l = flat.clone();
        l.envelope = emb_model::Envelope {
            shape: emb_model::EnvelopeShape::ArchOver,
            amount: 0.5,
        };
        assert!(crate::lettering::relayout(&mut h.st, &mut l));
        let b = l.envelope_box.unwrap();
        for i in 0..l.letters.len() {
            let (a, f) = (anchor(&l, i).unwrap(), anchor(&flat, i).unwrap());
            // Arch over moves the baseline up by half the height times sin(pi u).
            let u = (a.x - l.placement.e - b.min.x) / b.width();
            let up = 0.5 * 20.0 * (std::f64::consts::PI * u).sin();
            assert!((f.y - a.y - up).abs() < 1e-6, "letter {i}: {a:?} {f:?} {up}");
        }
        assert!(
            anchor(&l, 1).unwrap().y < anchor(&l, 0).unwrap().y - 2.0,
            "the middle bows up"
        );
    }

    #[test]
    fn circle_radius_handle() {
        let (mut h, id) = anna();
        h.st.doc.edit("Circle", |d| {
            if let ObjectKind::Lettering(l) = &mut d.object_mut(id).unwrap().kind {
                l.baseline = Baseline::Circle {
                    radius_mm: 15.0,
                    clockwise: true,
                };
            }
        });
        let mut l = lettering(&h, id);
        assert!(crate::lettering::relayout(&mut h.st, &mut l));
        h.st.doc.edit("Layout", |d| {
            d.object_mut(id).unwrap().kind = ObjectKind::Lettering(Box::new(l.clone()));
        });
        h.frame(vec![]);
        let from = radius_handle(&l).unwrap();
        let center = l.placement.apply(Point::ZERO);
        let to = center + (from - center) * 1.5;
        h.drag(PointerButton::Primary, h.screen(from), h.screen(to), Modifiers::NONE);
        let r = lettering(&h, id).baseline.radius().unwrap();
        assert!((r - 22.5).abs() < 0.5, "{r}");
    }
}
