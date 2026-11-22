//! Tests of sketch mode.

use super::*;
use crate::test_util::Harness;

/// A harness in sketch mode, with a sketch with no origin (as from an
/// older version), so points at (0, 0) do not snap to it.
fn sketching() -> Harness {
    let mut h = sketching_with_origin();
    h.st.edit_sketch("No origin", None, |sk| *sk = Sketch::new());
    h
}

/// A harness in sketch mode with a new sketch (it has its origin).
fn sketching_with_origin() -> Harness {
    let mut h = Harness::empty();
    h.st.start_sketch();
    assert!(h.st.sketch.is_some());
    h
}

fn click_tool(h: &mut Harness, tool: SketchTool, pts: &[Point]) {
    h.st.set_sketch_tool(tool);
    let id = h.st.sketch.as_ref().unwrap().object;
    for p in pts {
        let e = h.st.sketch.as_mut().unwrap();
        e.clicks.push(Snap::free(*p));
        let clicks = e.clicks.clone();
        if clicks.len() >= clicks_needed(tool, &MakeOpts::of(e)) {
            let opts = MakeOpts::of(e);
            h.st.edit_sketch("Sketch", None, |sk| make(tool, sk, &clicks, opts));
            h.st.sketch.as_mut().unwrap().clicks.clear();
        }
    }
    assert_eq!(h.st.sketch.as_ref().unwrap().object, id);
}

fn sk(h: &Harness) -> Sketch {
    h.st.sketch_obj().unwrap().sketch().clone()
}

#[test]
fn start_draw_and_finish() {
    let mut h = sketching();
    assert_eq!(h.st.settings.ribbon_tab, RibbonTab::Sketch);
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let s = sk(&h);
    assert_eq!((s.points.len(), s.curves.len()), (4, 4));
    assert_eq!(s.constraints.len(), 4, "horizontal and vertical sides");
    let a = h.st.sketch_analysis().unwrap();
    assert_eq!(a.free, 4, "width, height and the place");
    h.st.finish_sketch();
    assert!(h.st.sketch.is_none());
    let id = h.st.selection.single().unwrap();
    assert!(matches!(
        h.st.doc.design().object(id).unwrap().kind,
        ObjectKind::Sketch(_)
    ));
    assert_ne!(h.st.settings.ribbon_tab, RibbonTab::Sketch);
    // An empty sketch goes away.
    h.st.selection.clear();
    h.st.start_sketch();
    let n = h.st.doc.design().objects().len();
    h.st.finish_sketch();
    assert_eq!(h.st.doc.design().objects().len(), n - 1);
}

#[test]
fn constraints_from_the_selection_and_from_clicks() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 4.0)]);
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(0.0, 10.0), Point::new(30.0, 16.0)],
    );
    let s = sk(&h);
    let (l1, l2) = (s.curves[0].id, s.curves[1].id);
    // Select a line, then Horizontal: it is added at once.
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l1)];
    h.st.set_sketch_tool(SketchTool::Constrain(Kind::Horizontal));
    let s = sk(&h);
    assert!(matches!(s.constraints[0].constraint, Constraint::Horizontal { .. }));
    // Parallel with nothing selected waits for two clicks.
    h.st.set_sketch_tool(SketchTool::Constrain(Kind::Parallel));
    assert!(!h.st.apply_constraint(Kind::Parallel, &[Pick::Curve(l1)]));
    assert!(h.st.apply_constraint(Kind::Parallel, &[Pick::Curve(l1), Pick::Curve(l2)]));
    let s = sk(&h);
    let Geometry::Line { a, b } = s.curve(l2).unwrap().geometry else {
        unreachable!()
    };
    assert!(
        (s.point(a).unwrap().y - s.point(b).unwrap().y).abs() < 1e-9,
        "parallel to a horizontal line"
    );
    // A wrong pick gives a message and adds nothing.
    let n = sk(&h).constraints.len();
    assert!(h.st.apply_constraint(Kind::Concentric, &[Pick::Curve(l1), Pick::Curve(l2)]));
    assert_eq!(sk(&h).constraints.len(), n);
    // Undo takes back the last constraint.
    h.st.doc.undo();
    assert_eq!(sk(&h).constraints.len(), n - 1);
}

#[test]
fn dimensions_change_the_shape_and_conflicts_show() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let s = sk(&h);
    let bottom = s.curves[0].id;
    let Build::Done(c) = dimension_of(&s, &[Pick::Curve(bottom)], true) else {
        panic!("a length")
    };
    assert_eq!(c.value(), Some(60.0));
    let id =
        h.st.edit_sketch("Dimension", None, |sk| sk.add_constraint(c))
            .unwrap()
            .unwrap();
    h.st.set_dimension_text(id, "80");
    let s = sk(&h);
    let Geometry::Line { a, b } = s.curve(bottom).unwrap().geometry else {
        unreachable!()
    };
    assert!((s.point(a).unwrap().distance(s.point(b).unwrap()) - 80.0).abs() < 1e-6);
    // The top line 50 mm long: a conflict with the bottom line (80 mm).
    let top = s.curves[2].id;
    let c = Constraint::Length { line: top, mm: 50.0 };
    let bad =
        h.st.edit_sketch("Dimension", None, |sk| sk.add_constraint(c))
            .unwrap()
            .unwrap();
    let a = h.st.sketch_analysis().unwrap();
    assert_eq!(a.conflicting, vec![bad]);
    let (_, state) = h.st.sketch_status().unwrap();
    assert!(state.contains("conflict"), "{state}");
    // Delete it: no conflict.
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Constraint(bad)];
    h.st.delete_sketch_selection();
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
}

#[test]
fn arcs_circles_polygons_and_points() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Circle,
        &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
    );
    click_tool(
        &mut h,
        SketchTool::Arc3,
        &[Point::new(20.0, 0.0), Point::new(40.0, 0.0), Point::new(30.0, 10.0)],
    );
    click_tool(
        &mut h,
        SketchTool::ArcCenter,
        &[Point::new(60.0, 0.0), Point::new(70.0, 0.0), Point::new(60.0, 10.0)],
    );
    click_tool(
        &mut h,
        SketchTool::Polygon,
        &[Point::new(0.0, 50.0), Point::new(10.0, 50.0)],
    );
    click_tool(&mut h, SketchTool::Point, &[Point::new(90.0, 90.0)]);
    let s = sk(&h);
    let arcs = s
        .curves
        .iter()
        .filter(|c| matches!(c.geometry, Geometry::Arc { .. }))
        .count();
    assert_eq!(arcs, 2);
    // The arc through three points has its middle point on it.
    let arc = s
        .curves
        .iter()
        .find(|c| matches!(c.geometry, Geometry::Arc { .. }))
        .unwrap();
    let Geometry::Arc { center, radius, .. } = arc.geometry else {
        unreachable!()
    };
    assert!((s.point(center).unwrap().distance(Point::new(30.0, 10.0)) - radius).abs() < 1e-6);
    // The polygon: 6 equal sides and a construction circle.
    assert_eq!(s.curves.iter().filter(|c| c.construction).count(), 1);
    assert!(
        s.constraints
            .iter()
            .filter(|c| matches!(c.constraint, Constraint::Equal { .. }))
            .count()
            >= 5
    );
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
}

#[test]
fn a_tangent_arc_goes_on_from_a_line() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
    let s = sk(&h);
    let Geometry::Line { b, .. } = s.curves[0].geometry else {
        unreachable!()
    };
    h.st.set_sketch_tool(SketchTool::ArcTangent);
    let clicks = vec![
        Snap::on_point(Point::new(30.0, 0.0), b),
        Snap::free(Point::new(40.0, 10.0)),
    ];
    h.st.edit_sketch("Sketch", None, |sk| {
        make(
            SketchTool::ArcTangent,
            sk,
            &clicks,
            MakeOpts {
                sides: 6,
                outside: false,
                construction: false,
                slot: SlotKind::CenterToCenter,
            },
        )
    });
    let s = sk(&h);
    assert!(
        s.constraints
            .iter()
            .any(|c| matches!(c.constraint, Constraint::Tangent { .. }))
    );
    let arc = s
        .curves
        .iter()
        .find(|c| matches!(c.geometry, Geometry::Arc { .. }))
        .unwrap();
    let Geometry::Arc { center, radius, .. } = arc.geometry else {
        unreachable!()
    };
    // Tangent at (30, 0) to a horizontal line: the center is above it.
    let c = s.point(center).unwrap();
    assert!(
        (c.x - 30.0).abs() < 1e-6 && (c.y - radius).abs() < 1e-6,
        "{c:?} {radius}"
    );
}

#[test]
fn a_drag_moves_the_sketch_in_one_undo_step() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let s = sk(&h);
    let corner = s.points[2].id;
    for k in 1..=3 {
        let to = Point::new(60.0 + f64::from(k) * 5.0, 40.0 + f64::from(k) * 5.0);
        h.st.edit_sketch("Drag in Sketch", Some("sketch-drag-1".into()), |sk| sk.drag(corner, to));
    }
    h.st.doc.end_merge();
    let s = sk(&h);
    assert!(s.point(corner).unwrap().distance(Point::new(75.0, 55.0)) < 1e-6);
    h.st.doc.undo();
    assert!(sk(&h).point(corner).unwrap().distance(Point::new(60.0, 40.0)) < 1e-6);
}

fn count(s: &Sketch, f: impl Fn(&Constraint) -> bool) -> usize {
    s.constraints.iter().filter(|c| f(&c.constraint)).count()
}

#[test]
fn the_line_tool_adds_constraints_unless_alt() {
    let mut h = sketching();
    h.st.set_sketch_tool(SketchTool::Line);
    line_click(&mut h.st, Snap::free(Point::new(0.0, 0.0)), false);
    line_click(&mut h.st, Snap::free(Point::new(40.0, 1.5)), false);
    let s = sk(&h);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Horizontal { .. })), 1);
    assert!(s.points.iter().any(|p| p.pos.distance(Point::new(40.0, 0.0)) < 1e-9));
    // With Alt: no constraint, the end where the click was.
    line_click(&mut h.st, Snap::free(Point::new(41.0, 30.0)), true);
    let s = sk(&h);
    assert_eq!(s.constraints.len(), 1);
    assert!(s.points.iter().any(|p| p.pos.distance(Point::new(41.0, 30.0)) < 1e-9));
    // A click on a curve puts the point on it.
    let bottom = s.curves[0].id;
    let on = Snap::on_curve(Point::new(10.0, 0.0), bottom);
    line_click(&mut h.st, on, false);
    let s = sk(&h);
    assert_eq!(
        count(
            &s,
            |c| matches!(c, Constraint::OnCurve { curve, .. } if *curve == bottom)
        ),
        1
    );
}

#[test]
fn edit_tools_trim_fillet_and_refuse_without_an_undo_step() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    // A fillet at a corner: an arc and a radius to type.
    h.st.set_sketch_tool(SketchTool::Fillet);
    h.st.edit_click(SketchTool::Fillet, Point::new(60.0, 40.0), 0.5);
    let s = sk(&h);
    assert_eq!(s.curves.iter().filter(|c| !c.is_line()).count(), 1, "{}", h.st.message);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 2);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_some());
    // A refused edit: a message and no undo step.
    let undo = h.st.doc.undo_len();
    h.st.edit_click(SketchTool::Fillet, Point::new(200.0, 200.0), 0.5);
    h.st.edit_click(SketchTool::Extend, Point::new(30.0, 0.0), 0.5);
    assert_eq!(h.st.doc.undo_len(), undo);
    assert!(!h.st.message.is_empty());
    // A line across the rectangle, then trim its outer piece.
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(30.0, -10.0), Point::new(30.0, 20.0)],
    );
    let n = sk(&h).curves.len();
    h.st.edit_click(SketchTool::Trim, Point::new(30.0, -5.0), 0.5);
    let s = sk(&h);
    assert_eq!(s.curves.len(), n);
    assert!(!s.points.iter().any(|p| p.pos.distance(Point::new(30.0, -10.0)) < 1e-6));
    h.st.doc.undo();
    assert!(
        sk(&h)
            .points
            .iter()
            .any(|p| p.pos.distance(Point::new(30.0, -10.0)) < 1e-6)
    );
}

#[test]
fn mirror_offset_patterns_and_relax() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(0.0, -20.0), Point::new(0.0, 20.0)],
    );
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(-10.0, 0.0), Point::new(-5.0, 10.0)],
    );
    let s = sk(&h);
    let (axis, l) = (s.curves[0].id, s.curves[1].id);
    // Mirror needs a selection first.
    h.st.set_sketch_tool(SketchTool::Mirror);
    assert_ne!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Mirror);
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    h.st.set_sketch_tool(SketchTool::Mirror);
    h.st.edit_click(SketchTool::Mirror, Point::new(0.0, 5.0), 0.5);
    let s = sk(&h);
    assert_eq!(
        count(&s, |c| matches!(c, Constraint::Symmetric { line, .. } if *line == axis)),
        2
    );
    assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
    // Offset of the selection at once.
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    h.st.set_sketch_tool(SketchTool::Offset);
    assert_eq!(count(&sk(&h), |c| matches!(c, Constraint::LineDistance { .. })), 1);
    // A rectangular pattern of the line: 3 places.
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    let n = sk(&h).curves.len();
    h.st.rect_pattern();
    assert_eq!(sk(&h).curves.len(), n + 2);
    // A circular pattern round the origin.
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    h.st.set_sketch_tool(SketchTool::CircularPattern);
    let n = sk(&h).curves.len();
    h.st.edit_click(SketchTool::CircularPattern, Point::new(100.0, 100.0), 0.5);
    assert_eq!(sk(&h).curves.len(), n + 5);
    // Relax: a length that conflicts goes.
    let s = sk(&h);
    let a = h.st.edit_sketch("Dimension", None, |sk| {
        sk.add_constraint(Constraint::Length { line: l, mm: 10.0 })
    });
    assert!(a.unwrap().is_ok());
    h.st.edit_sketch("Dimension", None, |sk| {
        sk.add_constraint(Constraint::Length { line: l, mm: 20.0 })
    });
    assert!(!h.st.sketch_analysis().unwrap().conflicting.is_empty());
    h.st.relax_sketch();
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    assert_eq!(h.st.sketch.as_ref().unwrap().relaxed.len(), 1);
    assert_eq!(sk(&h).constraints.len(), s.constraints.len() + 1);
}

#[test]
fn the_dimension_tool_makes_a_distance_to_a_line() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
    click_tool(&mut h, SketchTool::Point, &[Point::new(10.0, 8.0)]);
    let s = sk(&h);
    let (l, q) = (s.curves[0].id, s.points[2].id);
    assert!(matches!(dimension_of(&s, &[Pick::Curve(l)], false), Build::More));
    let Build::Done(c) = dimension_of(&s, &[Pick::Point(q), Pick::Curve(l)], false) else {
        panic!("a distance to the line")
    };
    assert!((c.value().unwrap() - 8.0).abs() < 1e-9);
    assert_eq!(value_text(crate::workspace::Units::Mm, &c), "8.00");
}

/// The keys go through the keys of the program first, as in the app
/// (the harness runs `handle_shortcuts` before the canvas).
#[test]
fn esc_delete_and_the_value_box_work_with_the_keys_of_the_program() {
    use eframe::egui::{Event, Key, Modifiers, PointerButton};
    let mut h = sketching();
    h.st.settings.units = crate::workspace::Units::Mm;
    h.st.set_sketch_tool(SketchTool::Rectangle);
    let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
    h.click(PointerButton::Primary, a);
    h.click(PointerButton::Primary, b);
    assert_eq!(sk(&h).curves.len(), 4);
    // Esc: back to Select at once.
    h.st.set_sketch_tool(SketchTool::Line);
    let p = h.screen(Point::new(50.0, 0.0));
    h.click(PointerButton::Primary, p);
    h.key(Key::Escape);
    let e = h.st.sketch.as_ref().unwrap();
    assert_eq!(e.tool, SketchTool::Select);
    assert!(e.clicks.is_empty());
    // Delete: the selected line goes, the sketch object stays.
    let line = h.screen(Point::new(0.0, -20.0));
    h.click(PointerButton::Primary, line);
    assert!(matches!(
        h.st.sketch.as_ref().unwrap().selection.as_slice(),
        [Pick::Curve(_)]
    ));
    h.key(Key::Delete);
    assert!(h.st.sketch_obj().is_some(), "the sketch stays");
    assert_eq!(sk(&h).curves.len(), 3);
    h.st.doc.undo();
    assert_eq!(sk(&h).curves.len(), 4);
    // A dimension, then a value in inches into the box (its old text is
    // selected, so typing replaces it).
    h.st.set_sketch_tool(SketchTool::Dimension);
    h.click(PointerButton::Primary, line);
    let empty = h.screen(Point::new(0.0, -35.0));
    h.click(PointerButton::Primary, empty);
    let (id, _) = h.st.sketch.as_ref().unwrap().value_edit.clone().expect("the value box");
    h.frame(vec![]);
    h.frame(vec![Event::Text("2 in".into())]);
    h.key(Key::Enter);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
    let v = sk(&h).constraint(id).unwrap().value().unwrap();
    assert!((v - 50.8).abs() < 1e-9, "{v}");
    // Esc closes the box and keeps the value.
    let text = value_text(h.st.settings.units, sk(&h).constraint(id).unwrap());
    h.st.sketch.as_mut().unwrap().edit_value(id, text);
    h.frame(vec![]);
    h.frame(vec![Event::Text("99".into())]);
    h.key(Key::Escape);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
    assert!((sk(&h).constraint(id).unwrap().value().unwrap() - 50.8).abs() < 1e-9);
    // The Dimension tool is still on; Esc goes to Select.
    h.key(Key::Escape);
    assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
    // A box selection round all of it.
    let (from, to) = (h.screen(Point::new(-60.0, -60.0)), h.screen(Point::new(60.0, 40.0)));
    h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
    let sel = h.st.sketch.as_ref().unwrap().selection.clone();
    assert_eq!(sel.iter().filter(|p| matches!(p, Pick::Curve(_))).count(), 4, "{sel:?}");
    // Ctrl+A, then Esc clears the selection.
    h.key(Key::Escape);
    assert!(h.st.sketch.as_ref().unwrap().selection.is_empty());
    h.press(Modifiers::COMMAND, Key::A);
    assert!(h.st.sketch.as_ref().unwrap().selection.len() >= 4);
    // A right click selects the line under the pointer.
    h.key(Key::Escape);
    h.click(PointerButton::Secondary, line);
    assert!(matches!(
        h.st.sketch.as_ref().unwrap().selection.as_slice(),
        [Pick::Curve(_)]
    ));
    assert!(h.st.sketch.as_ref().unwrap().menu_at.is_some());
    // No undo step from all the drawing.
    assert!(h.st.sketch.is_some());
}

#[test]
fn delete_a_point_with_its_lines_and_the_constraints_of_a_part() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let s = sk(&h);
    let corner = s.points[0].id;
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Point(corner)];
    h.st.delete_sketch_selection();
    assert_eq!(sk(&h).curves.len(), 2, "the two lines of the corner go");
    h.st.doc.undo();
    let line = sk(&h).curves[0].id;
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(line)];
    h.st.remove_sketch_constraints_of_selection();
    let s = sk(&h);
    assert_eq!(s.curves.len(), 4);
    assert!(!s.constraints.iter().any(|c| c.constraint.curves().contains(&line)));
    // The tool values follow the units.
    let v = EditValues::for_units(crate::workspace::Units::Inch);
    assert!((v.fillet_mm - 3.175).abs() < 1e-9);
}

#[test]
fn rectangles_slots_and_polygons() {
    let mut h = sketching();
    // A center rectangle: the center is the middle of a diagonal.
    click_tool(
        &mut h,
        SketchTool::RectCenter,
        &[Point::new(0.0, 0.0), Point::new(20.0, 10.0)],
    );
    let s = sk(&h);
    assert!(s.points.iter().any(|p| p.pos.distance(Point::new(-20.0, -10.0)) < 1e-9));
    assert_eq!(count(&s, |c| matches!(c, Constraint::Midpoint { .. })), 1);
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    // A rectangle at an angle.
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rect3,
        &[Point::new(0.0, 0.0), Point::new(30.0, 17.0), Point::new(20.0, 40.0)],
    );
    let s = sk(&h);
    assert_eq!(s.curves.len(), 4);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Perpendicular { .. })), 1);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Parallel { .. })), 2);
    assert_eq!(h.st.sketch_analysis().unwrap().free, 5, "place, angle, two sizes");
    // A slot: two centers and a width; 5 free motions.
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Slot,
        &[Point::new(0.0, 0.0), Point::new(40.0, 1.0), Point::new(20.0, 8.0)],
    );
    let s = sk(&h);
    assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 4);
    let a = h.st.sketch_analysis().unwrap();
    assert!(a.conflicting.is_empty());
    assert_eq!(a.free, 4, "the centers (the axis is horizontal) and the width");
    // A hexagon round its circle: the sides touch the circle.
    let mut h = sketching();
    h.st.sketch.as_mut().unwrap().polygon_outside = true;
    click_tool(
        &mut h,
        SketchTool::Polygon,
        &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
    );
    let s = sk(&h);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 6);
    let far = s.points.iter().map(|p| p.pos.length()).fold(0.0, f64::max);
    assert!((far - 10.0 / (std::f64::consts::PI / 6.0).cos()).abs() < 1e-9);
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    // Collinear from the selection.
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(10.0, 3.0)]);
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(20.0, 5.0), Point::new(30.0, 4.0)],
    );
    let s = sk(&h);
    let sel = vec![Pick::Curve(s.curves[0].id), Pick::Curve(s.curves[1].id)];
    assert!(h.st.apply_constraint(Kind::Collinear, &sel));
    assert_eq!(count(&sk(&h), |c| matches!(c, Constraint::Collinear { .. })), 1);
}

#[test]
fn dimensions_go_where_the_click_is_and_repeated_ones_are_driven() {
    use eframe::egui::{Modifiers, PointerButton};
    let mut h = sketching();
    h.st.set_sketch_tool(SketchTool::Rectangle);
    let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
    h.click(PointerButton::Primary, a);
    h.click(PointerButton::Primary, b);
    // The bottom line, then a place: the label goes there.
    h.st.set_sketch_tool(SketchTool::Dimension);
    let bottom = h.screen(Point::new(0.0, -20.0));
    h.click(PointerButton::Primary, bottom);
    // The ghost draws while the pointer moves.
    let place = h.screen(Point::new(10.0, -40.0));
    h.frame(vec![eframe::egui::Event::PointerMoved(place)]);
    h.click(PointerButton::Primary, place);
    let s = sk(&h);
    let (id, at) = s.label_at.iter().next().map(|(i, p)| (*i, *p)).expect("a label place");
    assert!(at.distance(Point::new(10.0, -40.0)) < 0.5, "{at:?}");
    h.st.sketch.as_mut().unwrap().value_edit = None;
    // The top line has the same length: its dimension is driven.
    let top = h.screen(Point::new(0.0, 20.0));
    h.click(PointerButton::Primary, top);
    let over = h.screen(Point::new(0.0, 35.0));
    h.click(PointerButton::Primary, over);
    let s = sk(&h);
    let driven: Vec<ConstraintId> = s.constraints.iter().filter(|c| c.driven).map(|c| c.id).collect();
    assert_eq!(driven.len(), 1, "{}", h.st.message);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    // Driving again: it repeats the other one.
    h.st.toggle_driven(driven[0]);
    assert!(!sk(&h).is_driven(driven[0]));
    h.st.doc.undo();
    assert!(sk(&h).is_driven(driven[0]));
    // Drag the label of the first dimension: one undo step.
    h.key(eframe::egui::Key::Escape);
    assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
    let from = h.screen(at);
    let to = h.screen(Point::new(10.0, -55.0));
    let undo = h.st.doc.undo_len();
    h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
    assert!(sk(&h).label_at[&id].distance(Point::new(10.0, -55.0)) < 0.5);
    assert_eq!(h.st.doc.undo_len(), undo + 1);
    // F9 hides the marks: a click on a mark does not pick it.
    h.key(eframe::egui::Key::F9);
    assert!(!h.st.sketch.as_ref().unwrap().show_marks);
    h.key(eframe::egui::Key::F8);
    assert!(h.st.sketch.as_ref().unwrap().show_marks);
    // A point and a line: the dimension waits for a click that places it.
    h.st.set_sketch_tool(SketchTool::Dimension);
    let n = sk(&h).constraints.len();
    let corner = h.screen(Point::new(-30.0, 20.0));
    h.click(PointerButton::Primary, corner);
    h.click(PointerButton::Primary, bottom);
    assert!(h.st.sketch.as_ref().unwrap().pending_dim.is_some());
    assert_eq!(sk(&h).constraints.len(), n, "not yet");
    let place = h.screen(Point::new(-50.0, 0.0));
    h.click(PointerButton::Primary, place);
    let s = sk(&h);
    assert_eq!(s.constraints.len(), n + 1);
    let last = s.constraints.last().unwrap().id;
    assert!(s.label_at[&last].distance(Point::new(-50.0, 0.0)) < 0.5);
}

#[test]
fn a_drag_of_the_selection_moves_all_of_it() {
    use eframe::egui::{Modifiers, PointerButton};
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(20.0, 5.0)]);
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(0.0, 30.0), Point::new(20.0, 40.0)],
    );
    let s = sk(&h);
    let sel = vec![Pick::Curve(s.curves[0].id), Pick::Curve(s.curves[1].id)];
    h.st.set_sketch_tool(SketchTool::Select);
    h.st.sketch.as_mut().unwrap().selection = sel;
    let from = h.screen(Point::new(10.0, 2.5));
    let to = h.screen(Point::new(20.0, 12.5));
    h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
    let s = sk(&h);
    assert!(
        s.points.iter().any(|p| p.pos.distance(Point::new(10.0, 40.0)) < 0.5),
        "the second line moved"
    );
    assert!(s.points.iter().any(|p| p.pos.distance(Point::new(10.0, 10.0)) < 0.5));
}

#[test]
fn a_drag_from_the_end_of_a_line_makes_a_tangent_arc() {
    use eframe::egui::{Modifiers, PointerButton};
    let mut h = sketching();
    h.st.set_sketch_tool(SketchTool::Line);
    let (a, b) = (h.screen(Point::new(-30.0, 0.0)), h.screen(Point::new(0.0, 0.0)));
    h.click(PointerButton::Primary, a);
    h.click(PointerButton::Primary, b);
    // Drag from the end of the line.
    let to = h.screen(Point::new(15.0, -15.0));
    h.drag(PointerButton::Primary, b, to, Modifiers::NONE);
    let s = sk(&h);
    let arc = s.curves.iter().find(|c| !c.is_line()).expect("an arc");
    assert!(
        s.constraints
            .iter()
            .any(|c| matches!(c.constraint, Constraint::Tangent { b, .. } if b == arc.id))
    );
    // The chain goes on from the end of the arc.
    let e = h.st.sketch.as_ref().unwrap();
    assert_eq!(e.clicks.len(), 1);
    assert!(
        e.clicks[0].at.distance(Point::new(15.0, -15.0)) < 0.5,
        "{:?}",
        e.clicks[0]
    );
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
}

#[test]
fn move_rotate_and_scale_the_selection() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(10.0, 3.0)]);
    let s = sk(&h);
    let l = s.curves[0].id;
    // Nothing selected: refused.
    h.st.set_sketch_tool(SketchTool::Move);
    assert_ne!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Move);
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    h.st.set_sketch_tool(SketchTool::Move);
    // A base point, then where it goes.
    h.st.edit_click(SketchTool::Move, Point::new(0.0, 0.0), 0.5);
    h.st.edit_click(SketchTool::Move, Point::new(20.0, 20.0), 0.5);
    let s = sk(&h);
    assert!(
        s.points.iter().any(|p| p.pos.distance(Point::new(20.0, 20.0)) < 1e-6),
        "{}",
        h.st.message
    );
    // By values.
    h.st.apply_motion(SketchTool::Move, Motion::Shift(Point::new(-20.0, -20.0)), false);
    assert!(
        sk(&h)
            .points
            .iter()
            .any(|p| p.pos.distance(Point::new(0.0, 0.0)) < 1e-6)
    );
    // A turn of 90° round the origin: counterclockwise on the screen
    // (the y of the design goes down).
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
    h.st.set_sketch_tool(SketchTool::Rotate);
    h.st.edit_click(SketchTool::Rotate, Point::new(0.0, 0.0), 0.5);
    assert!(
        sk(&h)
            .points
            .iter()
            .any(|p| p.pos.distance(Point::new(3.0, -10.0)) < 1e-6)
    );
    // A scaled copy.
    h.st.sketch.as_mut().unwrap().values.copy_parts = true;
    h.st.set_sketch_tool(SketchTool::Scale);
    let n = sk(&h).curves.len();
    h.st.edit_click(SketchTool::Scale, Point::new(0.0, 0.0), 0.5);
    let s = sk(&h);
    assert_eq!(s.curves.len(), n + 1);
    assert!(s.points.iter().any(|p| p.pos.distance(Point::new(6.0, -20.0)) < 1e-6));
}

#[test]
fn the_place_of_a_dimension_chooses_its_kind() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(0.0, 0.0), Point::new(30.0, 20.0)],
    );
    let s = sk(&h);
    let l = s.curves[0].id;
    let len = Constraint::Length { line: l, mm: 0.0 };
    // Below the line (y bigger) and between its x: horizontal.
    assert!(matches!(
        orient(&s, len, Point::new(15.0, 40.0)),
        Constraint::HorizontalDistance { mm, .. } if (mm - 30.0).abs() < 1e-9
    ));
    assert!(matches!(
        orient(&s, len, Point::new(-10.0, 10.0)),
        Constraint::VerticalDistance { mm, .. } if (mm - 20.0).abs() < 1e-9
    ));
    assert!(matches!(
        orient(&s, len, Point::new(40.0, 30.0)),
        Constraint::Length { .. }
    ));
    // The kinds of an arc: radius, diameter, arc length.
    click_tool(
        &mut h,
        SketchTool::ArcCenter,
        &[Point::new(0.0, 50.0), Point::new(10.0, 50.0), Point::new(0.0, 60.0)],
    );
    let s = sk(&h);
    let arc = s.curves.iter().find(|c| !c.is_line()).unwrap().id;
    let kinds = dimension_kinds(&s, &Constraint::Radius { curve: arc, mm: 10.0 });
    assert_eq!(kinds.len(), 3);
    let Constraint::ArcLength { mm, .. } = kinds[2].1 else {
        panic!("an arc length")
    };
    assert!((mm - 5.0 * std::f64::consts::PI).abs() < 1e-6, "{mm}");
    h.st.add_sketch_dimension(kinds[2].1, None);
    assert!(
        sk(&h)
            .constraints
            .iter()
            .any(|c| matches!(c.constraint, Constraint::ArcLength { .. }))
    );
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
}

#[test]
fn centerlines_and_relax_mode() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(0.0, 40.0)]);
    click_tool(&mut h, SketchTool::Point, &[Point::new(-8.0, 20.0)]);
    let s = sk(&h);
    let (axis, q) = (s.curves[0].id, s.points[2].id);
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(axis)];
    h.st.toggle_centerline();
    assert!(sk(&h).is_centerline(axis));
    // A point and the centerline: a diameter of 16.
    let Build::Done(c) = dimension_of(&sk(&h), &[Pick::Point(q), Pick::Curve(axis)], false) else {
        panic!("a dimension")
    };
    assert!((c.value().unwrap().abs() - 16.0).abs() < 1e-9, "{c:?}");
    // Relax mode: a drag changes a dimension that holds the part.
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(20.0, 0.0)]);
    let l = sk(&h).curves[0].id;
    let d =
        h.st.edit_sketch("Dimension", None, |sk| {
            sk.add_constraint(Constraint::Length { line: l, mm: 20.0 })
        })
        .unwrap()
        .unwrap();
    let Geometry::Line { b, .. } = sk(&h).curve(l).unwrap().geometry else {
        unreachable!()
    };
    h.st.sketch.as_mut().unwrap().relax_mode = true;
    let moves = [(b, Point::new(30.0, 0.0))];
    h.st.edit_sketch("Drag in Sketch", None, |sk| {
        let dims = relax_dims(sk, true);
        sk.drag_points(&moves);
        restore_dims(sk, &dims);
    });
    let s = sk(&h);
    assert!((s.constraint(d).unwrap().value().unwrap() - 30.0).abs() < 1e-6);
    assert!(!s.is_driven(d), "it drives again");
}

/// Runs draw tool `tool` with the snaps `clicks`.
fn snap_tool(h: &mut Harness, tool: SketchTool, clicks: &[Snap]) {
    h.st.set_sketch_tool(tool);
    let opts = MakeOpts::of(h.st.sketch.as_ref().unwrap());
    let clicks = clicks.to_vec();
    let r = h.st.edit_sketch("Sketch", None, |sk| make(tool, sk, &clicks, opts));
    assert!(r.flatten().is_some(), "{tool:?} made nothing");
}

#[test]
fn slots_of_each_kind_and_a_center_rectangle_at_an_angle() {
    for kind in SlotKind::ALL {
        let mut h = sketching();
        h.st.sketch.as_mut().unwrap().slot_kind = kind;
        let pts: Vec<Point> = match kind {
            SlotKind::Arc3 => vec![
                Point::new(-30.0, 0.0),
                Point::new(30.0, 0.0),
                Point::new(0.0, -30.0),
                Point::new(0.0, -35.0),
            ],
            SlotKind::ArcCenter => vec![
                Point::new(0.0, 0.0),
                Point::new(30.0, 0.0),
                Point::new(0.0, 30.0),
                Point::new(25.0, 10.0),
            ],
            _ => vec![Point::new(0.0, 0.0), Point::new(40.0, 0.0), Point::new(20.0, 6.0)],
        };
        let clicks: Vec<Snap> = pts.iter().map(|q| Snap::free(*q)).collect();
        assert_eq!(
            clicks_needed(SketchTool::Slot, &MakeOpts::of(h.st.sketch.as_ref().unwrap())),
            clicks.len()
        );
        snap_tool(&mut h, SketchTool::Slot, &clicks);
        let s = sk(&h);
        assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4, "{kind:?}");
        let a = h.st.sketch_analysis().unwrap();
        assert!(a.conflicting.is_empty(), "{kind:?} {a:?}");
        if kind == SlotKind::Overall {
            // The ends are where the clicks were: the centers are a
            // half width (6) in.
            for x in [6.0, 34.0] {
                assert!(
                    s.points.iter().any(|p| p.pos.distance(Point::new(x, 0.0)) < 1e-6),
                    "{x}"
                );
            }
        }
    }
    // A center rectangle at an angle: the center is the middle.
    let mut h = sketching();
    let clicks = [Point::new(0.0, 0.0), Point::new(20.0, 10.0), Point::new(15.0, 30.0)].map(Snap::free);
    snap_tool(&mut h, SketchTool::RectCenter3, &clicks);
    let s = sk(&h);
    assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Midpoint { .. })), 1);
    // A corner: the center, a half side along (20, 10), a half height across.
    let d = Point::new(20.0, 10.0);
    let n = d.normalized().unwrap().perp();
    let h_ = (Point::new(15.0, 30.0) - d).dot(n);
    let corner = d + n * h_;
    assert!(s.points.iter().any(|p| p.pos.distance(corner) < 1e-6), "{corner:?}");
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
}

#[test]
fn a_tangent_circle_a_chamfer_with_an_angle_and_a_stretch() {
    let mut h = sketching();
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(30.0, 0.0), Point::new(0.0, 40.0)],
    );
    click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 40.0), Point::new(0.0, 0.0)]);
    let s = sk(&h);
    let l: Vec<CurveId> = s.curves.iter().map(|c| c.id).collect();
    let clicks = [
        Snap::on_curve(Point::new(10.0, 0.0), l[0]),
        Snap::on_curve(Point::new(15.0, 20.0), l[1]),
        Snap::on_curve(Point::new(0.0, 10.0), l[2]),
    ];
    snap_tool(&mut h, SketchTool::TangentCircle, &clicks);
    let s = sk(&h);
    let circle = s.curves.iter().find(|c| !c.is_line()).unwrap().id;
    assert!((s.radius(circle).unwrap() - 10.0).abs() < 1e-6);
    assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 3);
    // A chamfer with a distance and an angle.
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    {
        let v = &mut h.st.sketch.as_mut().unwrap().values;
        v.chamfer_kind = 2;
        v.chamfer_mm = 5.0;
        v.chamfer_deg = 30.0;
    }
    h.st.edit_click(SketchTool::Chamfer, Point::new(60.0, 40.0), 0.5);
    let s = sk(&h);
    assert_eq!(
        count(&s, |c| matches!(c, Constraint::Angle { .. })),
        1,
        "{}",
        h.st.message
    );
    assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    // Stretch: the two right corners move, the rectangle gets wider.
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let right: Vec<PointId> = sk(&h).points.iter().filter(|p| p.pos.x > 30.0).map(|p| p.id).collect();
    assert_eq!(right.len(), 2);
    h.st.sketch.as_mut().unwrap().source_points = right;
    h.st.apply_motion(SketchTool::Stretch, Motion::Shift(Point::new(20.0, 0.0)), false);
    let s = sk(&h);
    let xs: Vec<f64> = s.points.iter().map(|p| p.pos.x).collect();
    assert!(
        xs.iter().any(|x| (x - 80.0).abs() < 1e-6) && xs.iter().any(|x| x.abs() < 1e-6),
        "{xs:?}"
    );
}

#[test]
fn center_points_and_automatic_dimensions() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    click_tool(&mut h, SketchTool::Point, &[Point::new(30.0, 20.0)]);
    let s = sk(&h);
    let q = s.points.last().unwrap().id;
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Point(q)];
    h.st.toggle_center_point();
    assert!(sk(&h).is_center_point(q));
    h.st.sketch.as_mut().unwrap().selection.clear();
    let undo = h.st.doc.undo_len();
    h.st.auto_dimension();
    assert_eq!(h.st.doc.undo_len(), undo + 1, "one undo step");
    let s = sk(&h);
    assert!(
        count(&s, |c| matches!(c, Constraint::Length { .. })) >= 2,
        "{}",
        h.st.message
    );
    let a = h.st.sketch_analysis().unwrap();
    assert!(a.conflicting.is_empty() && a.redundant.is_empty());
    // The rectangle has its place free and the point both of its places.
    assert_eq!(a.free, 4, "{}", h.st.message);
}

#[test]
fn a_length_label_is_on_its_dimension_line() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(0.0, 0.0), Point::new(40.0, 30.0)],
    );
    let l = sk(&h).curves[0].id;
    h.st.edit_sketch("Dimension", None, |sk| {
        sk.add_constraint(Constraint::Length { line: l, mm: 50.0 })
    });
    let obj = h.st.sketch_obj().unwrap().clone();
    let map = h.map();
    let lab = labels(&obj, &map, crate::workspace::Units::Mm)
        .into_iter()
        .find(|x| x.dimension)
        .unwrap();
    let (p, q) = lab.leader.unwrap();
    let m = p.lerp(q, 0.5);
    assert!(lab.rect.center().distance(m) < 1.0, "{:?} {m:?}", lab.rect.center());
}

fn kinds(h: &Harness) -> Vec<&'static str> {
    h.st.doc
        .design()
        .objects()
        .iter()
        .map(|o| o.kind.type_name())
        .filter(|n| *n != "Sketch")
        .collect()
}

#[test]
fn vectors_and_dxf_drawings_become_sketches() {
    let mut h = Harness::empty();
    let path = emb_geom::Path::rounded_rect(emb_geom::BBox::new(Point::new(0.0, 0.0), Point::new(50.0, 30.0)), 5.0);
    let v = h.st.doc.edit("v", |d| {
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        d.add_object(
            t,
            ObjectKind::Vector {
                path,
                style: emb_model::VectorStyle::filled(emb_model::Rgb(200, 0, 0)),
            },
        )
    });
    h.st.selection.set(v);
    h.st.vectors_to_sketch();
    let d = h.st.doc.design();
    let vector = |d: &emb_model::Design| d.objects().iter().any(|o| matches!(o.kind, ObjectKind::Vector { .. }));
    assert!(!vector(d), "the vector goes");
    let id = h.st.selection.single().unwrap();
    let Some(ObjectKind::Sketch(s)) = d.object(id).map(|o| &o.kind) else {
        panic!("a sketch")
    };
    assert_eq!(s.sketch().curves.len(), 8);
    assert_eq!(s.regions().len(), 1);
    h.st.doc.undo();
    assert!(vector(h.st.doc.design()), "one undo step");
    // A DXF drawing.
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n12\n0\nENDSEC\n0\nEOF\n";
    h.st.import_dxf_bytes(dxf.as_bytes(), "ring.dxf");
    let d = h.st.doc.design();
    let id = h.st.selection.single().unwrap();
    let b = d.bbox_of(&[id]).unwrap();
    assert!((b.width() - 24.0).abs() < 1e-6);
    assert!(
        (b.center() - d.hoop.center).length() < 1e-6,
        "in the middle of the hoop"
    );
}

#[test]
fn the_spline_tool_draws_a_smooth_curve() {
    let mut h = sketching();
    h.st.set_sketch_tool(SketchTool::Spline);
    let tol = 0.5;
    for q in [Point::new(0.0, 0.0), Point::new(20.0, 15.0), Point::new(40.0, 0.0)] {
        spline_click(&mut h.st, Snap::free(q), tol);
    }
    assert_eq!(h.st.sketch.as_ref().unwrap().clicks.len(), 3);
    // A click on the last point ends it: two pieces and a tangent.
    spline_click(&mut h.st, Snap::free(Point::new(40.0, 0.2)), tol);
    let s = sk(&h);
    assert_eq!(s.curves.iter().filter(|c| c.is_spline()).count(), 2);
    assert!(
        s.constraints
            .iter()
            .any(|c| matches!(c.constraint, Constraint::Tangent { .. }))
    );
    assert!(h.st.sketch.as_ref().unwrap().clicks.is_empty());
    // A closed spline: a click on the first point.
    for q in [Point::new(0.0, 50.0), Point::new(30.0, 50.0), Point::new(30.0, 80.0)] {
        spline_click(&mut h.st, Snap::free(q), tol);
    }
    spline_click(&mut h.st, Snap::free(Point::new(0.2, 50.0)), tol);
    assert_eq!(sk(&h).curves.iter().filter(|c| c.is_spline()).count(), 5);
    assert_eq!(h.st.sketch_regions().len(), 1, "the closed spline is a region");
    // A dimension does not take a spline.
    let c = sk(&h).curves[0].id;
    assert!(matches!(
        dimension_of(&sk(&h), &[Pick::Curve(c)], true),
        Build::Wrong(_)
    ));
}

#[test]
fn typed_names_and_the_parameters_window_drive_the_dimensions() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let bottom = sk(&h).curves[0].id;
    let id =
        h.st.edit_sketch("Dimension", None, |s| {
            s.add_constraint(Constraint::Length { line: bottom, mm: 60.0 }).unwrap()
        })
        .unwrap();
    h.st.toggle_parameters();
    assert!(h.st.sketch.as_ref().unwrap().params.open);
    h.st.param_action(ParamAction::Set("width".into(), "70 mm".into()));
    h.st.set_dimension_text(id, "width + 10");
    let len = |h: &Harness| sk(h).constraint(id).unwrap().value().unwrap();
    assert!((len(&h) - 80.0).abs() < 1e-9);
    assert_eq!(sk(&h).dimension_expr(id), Some("width + 10"));
    // A change of the parameter: the dimension and the sketch follow.
    let steps = h.st.doc.undo_len();
    h.st.param_action(ParamAction::Set("width".into(), "50".into()));
    assert_eq!(h.st.doc.undo_len(), steps + 1);
    assert!((len(&h) - 60.0).abs() < 1e-9);
    // A loop is refused with a message and no undo step.
    h.st.param_action(ParamAction::Set("depth".into(), "width".into()));
    h.st.param_action(ParamAction::Set("width".into(), "depth".into()));
    let err = h.st.sketch.as_ref().unwrap().params.error.clone().unwrap();
    assert!(err.contains("loop"), "{err}");
    assert!((len(&h) - 60.0).abs() < 1e-9);
    // An unknown name in the value box.
    h.st.set_dimension_text(id, "height");
    assert!(h.st.message.contains("height"), "{}", h.st.message);
    // A plain number ends the expression.
    h.st.set_dimension_text(id, "45");
    assert_eq!(sk(&h).dimension_expr(id), None);
    assert!((len(&h) - 45.0).abs() < 1e-9);
}

#[test]
fn a_template_with_parameters_asks_for_their_values() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let bottom = sk(&h).curves[0].id;
    let id =
        h.st.edit_sketch("Dimension", None, |s| {
            s.add_constraint(Constraint::Length { line: bottom, mm: 60.0 }).unwrap()
        })
        .unwrap();
    h.st.param_action(ParamAction::Set("width".into(), "60 mm".into()));
    h.st.set_dimension_text(id, "width");
    h.st.make_picked_regions(RegionTo::Fill);
    h.st.finish_sketch();
    let dir = std::env::temp_dir().join(format!("emb-template-ask-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Badge.embtemplate");
    let t = emb_model::template::from_design(h.st.doc.design(), "Badge", true);
    emb_model::template::save(&t, &path).unwrap();
    h.st.new_from_template(Some(&path));
    std::fs::remove_dir_all(&dir).ok();
    let ask = h.st.template_ask.as_mut().expect("the template asks");
    assert_eq!(ask.rows.len(), 1);
    ask.rows[0].1 = "wide".into();
    assert!(!h.st.make_from_template_ask(), "an unknown name is refused");
    assert!(h.st.template_ask.as_ref().unwrap().error.is_some());
    h.st.template_ask.as_mut().unwrap().rows[0].1 = "90 mm".into();
    assert!(h.st.make_from_template_ask());
    let d = h.st.doc.design();
    let fill = d
        .objects()
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Fill { .. }))
        .unwrap()
        .id;
    assert!((d.bbox_of(&[fill]).unwrap().width() - 90.0).abs() < 1e-3);
}

#[test]
fn objects_made_from_a_sketch_follow_its_changes() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    let sketch = h.st.sketch.as_ref().unwrap().object;
    let bottom = sk(&h).curves[0].id;
    h.st.make_picked_regions(RegionTo::Fill);
    let fill = h.st.doc.design().objects().last().unwrap().id;
    let width = |h: &Harness| h.st.doc.design().bbox_of(&[fill]).unwrap().width();
    assert!((width(&h) - 60.0).abs() < 1e-3);
    // A length of 80 mm: the fill is made again, in one undo step.
    let steps = h.st.doc.undo_len();
    h.st.edit_sketch("Dimension", None, |s| {
        s.add_constraint(Constraint::Length { line: bottom, mm: 80.0 }).unwrap();
    });
    assert_eq!(h.st.doc.undo_len(), steps + 1);
    assert!((width(&h) - 80.0).abs() < 1e-3, "{}", width(&h));
    h.st.doc.undo();
    assert!((width(&h) - 60.0).abs() < 1e-3);
    // A line that is gone: the fill stays, with a warning.
    h.st.edit_sketch("Delete", None, |s| {
        s.remove_curve(bottom);
    });
    assert!(h.st.message.contains("Unlink"), "{}", h.st.message);
    assert!((width(&h) - 60.0).abs() < 1e-3);
    h.st.doc.undo();
    // After Finish Sketch, the fill opens its sketch; Unlink ends the link.
    h.st.finish_sketch();
    assert_eq!(h.st.doc.design().sketch_for(fill), Some(sketch));
    h.st.selection.set(fill);
    h.st.unlink_selection();
    assert!(!h.st.doc.design().is_linked(fill));
}

#[test]
fn a_sketch_and_its_fill_stay_together_until_break_apart() {
    let mut h = sketching_with_origin();
    // A new sketch has its origin, and Delete does not take it.
    let origin = sk(&h).origin.expect("the origin");
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Point(origin)];
    h.st.delete_sketch_selection();
    assert_eq!(sk(&h).origin, Some(origin));
    assert!(sk(&h).point(origin).is_some());
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(10.0, 10.0), Point::new(70.0, 50.0)],
    );
    h.st.set_sketch_tool(SketchTool::Regions);
    h.st.make_picked_regions(RegionTo::Fill);
    let sketch = h.st.sketch.as_ref().unwrap().object;
    h.st.finish_sketch();
    let d = h.st.doc.design();
    let fill = d.objects().iter().find(|o| o.kind.type_name() == "Fill").unwrap().id;
    let at = |h: &Harness, id| h.st.doc.design().bbox_of(&[id]).unwrap();
    // A move of the fill moves the sketch.
    let (sketch_at, fill_at) = (at(&h, sketch), at(&h, fill));
    h.st.selection.set(fill);
    h.st.nudge_selection(5.0, 0.0);
    assert!((at(&h, sketch).min.x - sketch_at.min.x - 5.0).abs() < 1e-6);
    assert!((at(&h, fill).min.x - fill_at.min.x - 5.0).abs() < 1e-6);
    // A scale does not change them, and says why.
    let before = at(&h, fill);
    h.st.transform_selection("Scale", |b| emb_model::edit::scale_about(b.center(), 2.0, 2.0));
    assert_eq!(at(&h, fill), before);
    assert!(h.st.message.contains("sketch mode"), "{}", h.st.message);
    assert!(h.st.refuse_sketch_scale());
    // Break Apart frees the fill: it moves alone now.
    h.st.break_apart();
    assert!(h.st.doc.design().sketch_link_of(fill).is_none());
    let sketch_at = at(&h, sketch);
    h.st.selection.set(fill);
    h.st.nudge_selection(5.0, 0.0);
    assert_eq!(at(&h, sketch), sketch_at);
    assert!(!h.st.refuse_sketch_scale());
}

#[test]
fn convert_makes_embroidery_of_a_selected_sketch() {
    let mut h = sketching_with_origin();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(10.0, 10.0), Point::new(70.0, 50.0)],
    );
    let sketch = h.st.sketch.as_ref().unwrap().object;
    h.st.finish_sketch();
    h.st.selection.set(sketch);
    let info = crate::ribbon::SelInfo::of(&h.st);
    assert!(h.st.can(crate::ribbon::Cmd::Convert(emb_model::ConvertTo::Fill), &info));
    h.st.convert_vectors(emb_model::ConvertTo::Fill);
    assert_eq!(kinds(&h), vec!["Fill"]);
    h.st.selection.set(sketch);
    h.st.convert_vectors(emb_model::ConvertTo::Outline);
    assert_eq!(kinds(&h), vec!["Fill", "Running", "Running", "Running", "Running"]);
    // They follow the sketch.
    let d = h.st.doc.design();
    assert!(
        d.objects()
            .iter()
            .filter(|o| o.id != sketch)
            .all(|o| d.sketch_link_of(o.id).is_some())
    );
}

#[test]
fn picked_regions_become_fills_and_curves_become_runs_and_satins() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    click_tool(
        &mut h,
        SketchTool::Line,
        &[Point::new(20.0, -5.0), Point::new(20.0, 45.0)],
    );
    assert_eq!(h.st.sketch_regions().len(), 2);
    // Pick the small region, then unpick and pick it again.
    h.st.set_sketch_tool(SketchTool::Regions);
    h.st.toggle_region(Point::new(10.0, 20.0));
    h.st.toggle_region(Point::new(12.0, 22.0));
    assert!(
        h.st.sketch.as_ref().unwrap().picked.is_empty(),
        "the same region unpicks"
    );
    h.st.toggle_region(Point::new(10.0, 20.0));
    let r = h.st.picked_regions();
    assert_eq!(r.len(), 1);
    assert!((r[0].area - 800.0).abs() < 1e-3);
    h.st.make_picked_regions(RegionTo::Fill);
    assert_eq!(kinds(&h), vec!["Fill"]);
    assert!(h.st.sketch.as_ref().unwrap().picked.is_empty());
    // Nothing picked: all regions.
    h.st.make_picked_regions(RegionTo::Applique);
    assert_eq!(kinds(&h), vec!["Fill", "Appliqué", "Appliqué"]);
    h.st.doc.undo();
    assert_eq!(kinds(&h), vec!["Fill"]);
    // Curves: a run and a satin line along the top line, a satin
    // column between the top and the bottom lines.
    let s = sk(&h);
    let (bottom, top) = (s.curves[0].id, s.curves[2].id);
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(top)];
    h.st.make_curves(CurveTo::Run);
    h.st.make_curves(CurveTo::SatinLine(3.0));
    h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(top), Pick::Curve(bottom)];
    h.st.make_satin_between();
    assert_eq!(kinds(&h), vec!["Fill", "Running", "Satin", "Satin"]);
    // The objects sew.
    for o in
        h.st.doc
            .design()
            .objects()
            .iter()
            .filter(|o| o.kind.type_name() != "Sketch")
    {
        let s = emb_stitch::stitch_object(o, Point::ZERO).unwrap();
        let n: usize = s.runs.iter().map(|r| r.points.len()).sum();
        assert!(n > 10, "{}: {n}", o.kind.type_name());
    }
}

#[test]
fn the_sketch_tab_and_the_panels_draw() {
    let mut h = sketching();
    click_tool(
        &mut h,
        SketchTool::Rectangle,
        &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
    );
    h.frame(vec![]);
    let before = h.st.doc.state_id();
    for tool in [
        SketchTool::Select,
        SketchTool::Move,
        SketchTool::Rotate,
        SketchTool::Scale,
        SketchTool::Fillet,
        SketchTool::Chamfer,
        SketchTool::Stretch,
        SketchTool::Offset,
        SketchTool::RectPattern,
        SketchTool::CircularPattern,
    ] {
        h.st.sketch.as_mut().unwrap().tool = tool;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| edit_panel(ui, st));
        });
        out.textures_delta.clear();
        assert!(h.st.sketch_status().is_some());
    }
    assert_eq!(h.st.doc.state_id(), before, "drawing the panel changes nothing");
    assert!(h.st.sketch_status().is_some());
    // An undo of Start Sketch ends sketch mode.
    while h.st.doc.can_undo() {
        h.st.doc.undo();
    }
    h.st.check_sketch();
    assert!(h.st.sketch.is_none());
}

#[test]
fn on_a_touch_screen_the_bar_makes_a_dimension_and_picks_are_wider() {
    use eframe::egui::PointerButton;
    let mut h = sketching();
    h.st.set_sketch_tool(SketchTool::Rectangle);
    let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
    h.click(PointerButton::Primary, a);
    h.click(PointerButton::Primary, b);
    h.st.touch_seen = true;
    h.st.set_sketch_tool(SketchTool::Dimension);
    // A finger 12 points away from the bottom line still picks it (a mouse
    // must be within 7 points).
    let bottom = h.screen(Point::new(0.0, -20.0)) + eframe::egui::Vec2::new(0.0, 12.0);
    h.click(PointerButton::Primary, bottom);
    assert!(
        matches!(h.st.sketch.as_ref().unwrap().picks.as_slice(), [Pick::Curve(_)]),
        "the line is picked"
    );
    // Length: the dimension of the line, with no second tap.
    let n = sk(&h).constraints.len();
    let length = h.widget("Length").expect("the touch bar shows Length");
    h.click(PointerButton::Primary, length.center());
    let s = sk(&h);
    assert_eq!(s.constraints.len(), n + 1);
    assert!(
        matches!(s.constraints.last().map(|c| c.constraint), Some(Constraint::Length { mm, .. }) if (mm - 60.0).abs() < 0.01),
        "{:?}",
        s.constraints.last()
    );
    // The value box opens for the value.
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_some());
    h.st.sketch.as_mut().unwrap().value_edit = None;
    // A circle: the dimension waits for a place; Add puts it in its own place.
    h.st.set_sketch_tool(SketchTool::Circle);
    let (c, r) = (h.screen(Point::new(60.0, 0.0)), h.screen(Point::new(70.0, 0.0)));
    h.click(PointerButton::Primary, c);
    h.click(PointerButton::Primary, r);
    h.st.set_sketch_tool(SketchTool::Dimension);
    h.click(PointerButton::Primary, r);
    assert!(h.st.sketch.as_ref().unwrap().pending_dim.is_some());
    let n = sk(&h).constraints.len();
    let add = h.widget("Add Dimension").expect("the touch bar shows Add");
    h.click(PointerButton::Primary, add.center());
    assert_eq!(sk(&h).constraints.len(), n + 1);
    assert!(h.st.sketch.as_ref().unwrap().pending_dim.is_none());
}
