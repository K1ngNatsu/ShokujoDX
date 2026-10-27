//! Tests of the drawing help and the sketch edits (P14 Milestone 3).

use emb_geom::Point;

use super::infer::{Hint, Snap, apply_hints, line_hints, line_hints_with, place, snap};
use super::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-6
}

fn ends(s: &Sketch, l: CurveId) -> (Point, Point) {
    match s.curve(l).unwrap().geometry {
        Geometry::Line { a, b } => (s.point(a).unwrap(), s.point(b).unwrap()),
        _ => panic!("not a line"),
    }
}

/// The distance from the center of an arc to the carrier of a line.
fn line_to_center(s: &Sketch, l: CurveId, arc: CurveId) -> f64 {
    let (a, b) = ends(s, l);
    let Geometry::Arc { center, .. } = s.curve(arc).unwrap().geometry else {
        panic!("not an arc")
    };
    let c = s.point(center).unwrap();
    ((b - a).cross(c - a) / a.distance(b)).abs()
}

/// A rectangle 60 × 40 with its first corner fixed at (0, 0), horizontal
/// and vertical sides, and the width and the height as lengths.
fn rectangle() -> (Sketch, [CurveId; 4], [PointId; 4], [ConstraintId; 2]) {
    let mut s = Sketch::new();
    let c = [p(0.0, 0.0), p(60.0, 0.0), p(60.0, 40.0), p(0.0, 40.0)].map(|q| s.add_point(q));
    let l = [0, 1, 2, 3].map(|i| s.add_line_between(c[i], c[(i + 1) % 4]).unwrap());
    for i in [0, 2] {
        s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
    }
    for i in [1, 3] {
        s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
    }
    let w = s.add_constraint(Constraint::Length { line: l[0], mm: 60.0 }).unwrap();
    let h = s.add_constraint(Constraint::Length { line: l[1], mm: 40.0 }).unwrap();
    s.add_constraint(Constraint::Fix {
        point: c[0],
        at: p(0.0, 0.0),
    })
    .unwrap();
    assert!(s.solve().converged);
    (s, l, c, [w, h])
}

#[test]
fn a_click_snaps_to_points_then_curves() {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(20.0, 0.0));
    let (a, _) = (s.curve(l).unwrap().points()[0], ());
    let sn = snap(&s, p(0.3, 0.2), 1.0, &[]);
    assert_eq!(sn.point, Some(a));
    let sn = snap(&s, p(5.0, 0.4), 1.0, &[]);
    assert_eq!((sn.point, sn.curve), (None, Some(l)));
    assert!(near(sn.at, p(5.0, 0.0)));
    // A new point on the curve has an on-curve constraint.
    let q = place(&mut s, &sn);
    assert!(
        s.constraints
            .iter()
            .any(|c| c.constraint == Constraint::OnCurve { point: q, curve: l })
    );
    assert_eq!(snap(&s, p(10.0, 5.0), 1.0, &[]).curve, None);
}

#[test]
fn an_almost_horizontal_line_becomes_horizontal() {
    let s = Sketch::new();
    let (end, hints) = line_hints(&s, &Snap::free(p(0.0, 0.0)), &Snap::free(p(30.0, 1.0)));
    assert_eq!(hints, vec![Hint::Horizontal]);
    assert!(near(end, p(30.0, 0.0)));
    let (end, hints) = line_hints(&s, &Snap::free(p(0.0, 0.0)), &Snap::free(p(-1.0, -25.0)));
    assert_eq!(hints, vec![Hint::Vertical]);
    assert!(near(end, p(0.0, -25.0)));
    let (_, hints) = line_hints(&s, &Snap::free(p(0.0, 0.0)), &Snap::free(p(20.0, 10.0)));
    assert!(hints.is_empty());
}

#[test]
fn a_line_from_the_end_of_an_arc_becomes_tangent() {
    let mut s = Sketch::new();
    // A quarter circle from (10, 0) to (0, 10) round (0, 0): at its end it
    // goes in −x.
    let arc = s.add_arc(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)).unwrap();
    let Geometry::Arc { end, .. } = s.curve(arc).unwrap().geometry else {
        unreachable!()
    };
    let from = Snap::on_point(p(0.0, 10.0), end);
    let (_, hints) = line_hints(&s, &from, &Snap::free(p(-20.0, 10.5)));
    assert_eq!(hints, vec![Hint::Tangent(arc)]);
    let a = place(&mut s, &from);
    let b = s.add_point(p(-20.0, 11.0));
    let l = s.add_line_between(a, b).unwrap();
    apply_hints(&mut s, l, &hints);
    assert!(s.solve().converged);
    assert!((line_to_center(&s, l, arc) - s.radius(arc).unwrap()).abs() < 1e-6);
}

#[test]
fn trim_removes_the_piece_between_the_cuts() {
    // A horizontal line through two vertical lines.
    let mut s = Sketch::new();
    let h = s.add_line(p(0.0, 0.0), p(30.0, 0.0));
    let v1 = s.add_line(p(10.0, -5.0), p(10.0, 5.0));
    let v2 = s.add_line(p(20.0, -5.0), p(20.0, 5.0));
    s.add_constraint(Constraint::Horizontal { line: h }).unwrap();
    // The middle piece: two pieces in line.
    s.trim(h, p(15.0, 0.0)).unwrap();
    let lines: Vec<CurveId> = s.curves.iter().filter(|c| c.is_line()).map(|c| c.id).collect();
    assert_eq!(lines.len(), 4);
    let (a, b) = ends(&s, h);
    assert!(near(a, p(0.0, 0.0)) && near(b, p(10.0, 0.0)), "{a:?} {b:?}");
    let piece = *lines.iter().find(|l| ![h, v1, v2].contains(l)).unwrap();
    let (c, d) = ends(&s, piece);
    assert!(near(c, p(20.0, 0.0)) && near(d, p(30.0, 0.0)));
    // A drag of the far end keeps the pieces in line and the cuts on the
    // vertical lines.
    let Geometry::Line { b: far, .. } = s.curve(piece).unwrap().geometry else {
        unreachable!()
    };
    assert!(s.drag(far, p(30.0, 3.0)).converged);
    let (c, d) = ends(&s, piece);
    assert!((c.y - d.y).abs() < 1e-6 && (c.x - 20.0).abs() < 1e-6);
    // The end piece of a line through one line: the line ends at the cut.
    let h2 = s.add_line(p(0.0, 100.0), p(30.0, 100.0));
    s.add_line(p(10.0, 90.0), p(10.0, 110.0));
    s.trim(h2, p(2.0, 100.0)).unwrap();
    let (a, b) = ends(&s, h2);
    assert!(near(a, p(10.0, 100.0)) && near(b, p(30.0, 100.0)), "{a:?} {b:?}");
    // A line that nothing cuts goes away.
    let free = s.add_line(p(0.0, 50.0), p(10.0, 50.0));
    s.trim(free, p(5.0, 50.0)).unwrap();
    assert!(s.curve(free).is_none());
}

#[test]
fn trim_makes_an_arc_of_a_circle() {
    let mut s = Sketch::new();
    let c = s.add_circle(p(0.0, 0.0), 10.0).unwrap();
    let l = s.add_line(p(-20.0, 0.0), p(20.0, 0.0));
    s.trim(c, p(0.0, -10.0)).unwrap();
    let Some(Geometry::Arc { start, end, .. }) = s.curve(c).map(|c| c.geometry) else {
        panic!("an arc")
    };
    // The lower half is gone: counterclockwise from (10, 0) to (−10, 0).
    assert!(near(s.point(start).unwrap(), p(10.0, 0.0)));
    assert!(near(s.point(end).unwrap(), p(-10.0, 0.0)));
    assert!(s.solve().converged);
    // Then trim the line inside the arc ends: three pieces of the line
    // need two cuts.
    s.trim(l, p(0.0, 0.0)).unwrap();
    assert_eq!(s.curves.iter().filter(|c| c.is_line()).count(), 2);
}

#[test]
fn extend_goes_to_the_next_curve() {
    let mut s = Sketch::new();
    let h = s.add_line(p(0.0, 0.0), p(10.0, 0.0));
    let v = s.add_line(p(25.0, -5.0), p(25.0, 5.0));
    s.add_constraint(Constraint::Length { line: h, mm: 10.0 }).unwrap();
    s.extend(h, p(9.0, 0.0)).unwrap();
    let (_, b) = ends(&s, h);
    assert!(near(b, p(25.0, 0.0)), "{b:?}");
    assert!(
        !s.constraints
            .iter()
            .any(|c| matches!(c.constraint, Constraint::Length { .. })),
        "the length goes"
    );
    // The end stays on the vertical line when it moves.
    let Geometry::Line { a: va, b: vb } = s.curve(v).unwrap().geometry else {
        unreachable!()
    };
    s.drag_points(&[(va, p(30.0, -5.0)), (vb, p(30.0, 5.0))]);
    assert!((ends(&s, h).1.x - 30.0).abs() < 1e-6);
    // Nothing to meet.
    let lone = s.add_line(p(0.0, 50.0), p(10.0, 50.0));
    assert!(s.extend(lone, p(10.0, 50.0)).is_err());
}

#[test]
fn split_makes_two_pieces_in_line() {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(20.0, 0.0));
    s.add_constraint(Constraint::Horizontal { line: l }).unwrap();
    let new = s.split(l, p(8.0, 0.1)).unwrap();
    let (a, m) = ends(&s, l);
    let (m2, b) = ends(&s, new);
    assert!(near(a, p(0.0, 0.0)) && near(m, p(8.0, 0.0)) && near(m, m2) && near(b, p(20.0, 0.0)));
    // The far end stays in line with the first piece.
    let Geometry::Line { b: far, .. } = s.curve(new).unwrap().geometry else {
        unreachable!()
    };
    s.drag(far, p(25.0, 4.0));
    let (a, m) = ends(&s, l);
    let b = ends(&s, new).1;
    assert!((b.y - 4.0).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 && (m.y - b.y).abs() < 1e-6);
    assert!(s.split(l, p(0.0, 0.0)).is_err(), "not at an end");
    let arc = s.add_arc(p(0.0, 30.0), p(10.0, 30.0), p(-10.0, 30.0)).unwrap();
    let piece = s.split(arc, p(0.0, 40.0)).unwrap();
    assert!(
        s.constraints
            .iter()
            .any(|c| c.constraint == Constraint::Equal { a: arc, b: piece })
    );
}

#[test]
fn a_fillet_stays_tangent_after_a_change_of_a_dimension() {
    let (mut s, l, c, [w, _]) = rectangle();
    let (arc, dim) = s.fillet(c[1], 5.0).unwrap();
    assert!(s.solve().converged);
    for line in [l[0], l[1]] {
        assert!((line_to_center(&s, line, arc) - 5.0).abs() < 1e-6);
    }
    assert!(s.analyze().conflicting.is_empty());
    // The width was the length of the bottom line: now it is the distance
    // to the corner, which stays as a point on both lines.
    assert!(s.constraint(w).is_none());
    let width = s
        .constraints
        .iter()
        .find(|x| matches!(x.constraint, Constraint::Distance { mm, .. } if (mm - 60.0).abs() < 1e-9))
        .unwrap()
        .id;
    s.set_value(width, 80.0).unwrap();
    s.set_value(dim, 8.0).unwrap();
    let r = s.solve();
    assert!(r.converged, "{r:?}");
    for line in [l[0], l[1]] {
        assert!((line_to_center(&s, line, arc) - 8.0).abs() < 1e-6, "tangent");
    }
    // The arc ends are the ends of the lines.
    let Geometry::Arc { start, end, .. } = s.curve(arc).unwrap().geometry else {
        unreachable!()
    };
    let (_, b0) = ends(&s, l[0]);
    let (a1, _) = ends(&s, l[1]);
    let arc_ends = [s.point(start).unwrap(), s.point(end).unwrap()];
    assert!(arc_ends.iter().any(|q| near(*q, b0)) && arc_ends.iter().any(|q| near(*q, a1)));
    assert!(near(s.point(c[1]).unwrap(), p(80.0, 0.0)), "the corner");
    assert!((b0.x - 72.0).abs() < 1e-6, "{b0:?}");
    assert!(s.analyze().conflicting.is_empty());
    // A radius too large for the lines is refused.
    assert!(s.fillet(c[3], 50.0).is_err());
}

#[test]
fn a_chamfer_cuts_the_corner() {
    let (mut s, l, c, _) = rectangle();
    let (line, dim) = s.chamfer(c[2], 4.0).unwrap();
    assert!(s.solve().converged);
    let (a, b) = ends(&s, line);
    assert!(near(a, p(60.0, 36.0)) || near(a, p(56.0, 40.0)), "{a:?}");
    assert!((a.distance(b) - 32.0_f64.sqrt()).abs() < 1e-6);
    s.set_value(dim, 6.0).unwrap();
    assert!(s.solve().converged);
    assert!(near(ends(&s, l[1]).1, p(60.0, 34.0)));
}

#[test]
fn an_offset_of_a_rectangle_follows_it() {
    let (mut s, l, _, [w, _]) = rectangle();
    let made = s.offset(&l, 2.0).unwrap();
    assert_eq!(made.len(), 4);
    assert!(s.solve().converged);
    let pts: Vec<Point> = made.iter().flat_map(|m| [ends(&s, *m).0, ends(&s, *m).1]).collect();
    let min_x = pts.iter().map(|q| q.x).fold(f64::MAX, f64::min);
    let max_x = pts.iter().map(|q| q.x).fold(f64::MIN, f64::max);
    let max_y = pts.iter().map(|q| q.y).fold(f64::MIN, f64::max);
    assert!((min_x + 2.0).abs() < 1e-6 && (max_x - 62.0).abs() < 1e-6 && (max_y - 42.0).abs() < 1e-6);
    // A wider rectangle: the offset stays 2 mm outside.
    s.set_value(w, 90.0).unwrap();
    assert!(s.solve().converged);
    let max_x = made
        .iter()
        .flat_map(|m| [ends(&s, *m).0.x, ends(&s, *m).1.x])
        .fold(f64::MIN, f64::max);
    assert!((max_x - 92.0).abs() < 1e-6, "{max_x}");
    assert!(s.analyze().conflicting.is_empty());
    // Inside with a negative distance.
    let (mut s, l, _, _) = rectangle();
    let made = s.offset(&l, -3.0).unwrap();
    s.solve();
    let min_y = made
        .iter()
        .flat_map(|m| [ends(&s, *m).0.y, ends(&s, *m).1.y])
        .fold(f64::MAX, f64::min);
    assert!((min_y - 3.0).abs() < 1e-6);
}

#[test]
fn an_offset_of_a_line_and_a_tangent_arc_and_a_circle() {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(20.0, 0.0));
    let Geometry::Line { b, .. } = s.curve(l).unwrap().geometry else {
        unreachable!()
    };
    // A half circle up from (20, 0) round (20, 10).
    let c = s.add_point(p(20.0, 10.0));
    let e = s.add_point(p(20.0, 20.0));
    let arc = s.add_arc_between(c, b, e, 10.0).unwrap();
    s.add_constraint(Constraint::Tangent {
        a: l,
        b: arc,
        inside: false,
    })
    .unwrap();
    assert!(s.solve().converged);
    // Positive: on the left of the chain, so inside the arc.
    let made = s.offset(&[l, arc], 2.0).unwrap();
    assert_eq!(made.len(), 2);
    assert!(s.solve().converged);
    assert!((s.radius(made[1]).unwrap() - 8.0).abs() < 1e-6);
    let (a0, a1) = ends(&s, made[0]);
    assert!((a0.y - 2.0).abs() < 1e-6 && (a1.y - 2.0).abs() < 1e-6 && (a1.x - 20.0).abs() < 1e-6);
    assert!(s.analyze().conflicting.is_empty());
    // A circle grows.
    let ring = s.add_circle(p(0.0, 50.0), 5.0).unwrap();
    let made = s.offset(&[ring], 1.5).unwrap();
    assert!((s.radius(made[0]).unwrap() - 6.5).abs() < 1e-9);
    assert!(s.offset(&[ring], -6.0).is_err());
}

#[test]
fn a_mirror_copy_stays_symmetric() {
    let mut s = Sketch::new();
    let axis = s.add_line(p(0.0, -10.0), p(0.0, 30.0));
    s.set_construction(axis, true);
    for q in s.curve(axis).unwrap().points() {
        let at = s.point(q).unwrap();
        s.add_constraint(Constraint::Fix { point: q, at }).unwrap();
    }
    let l = s.add_line(p(-10.0, 0.0), p(-5.0, 10.0));
    let arc = s.add_arc(p(-5.0, 20.0), p(-2.0, 20.0), p(-8.0, 20.0)).unwrap();
    let made = s.mirror(&[l, arc, axis], axis).unwrap();
    assert_eq!(made.len(), 2);
    let (a, b) = ends(&s, made[0]);
    assert!(near(a, p(10.0, 0.0)) && near(b, p(5.0, 10.0)));
    assert!(s.solve().converged);
    // The arc turns: it is still an upper half circle.
    let Shape::Arc { c, a0, .. } = s.shape(made[1]).unwrap() else {
        panic!("an arc")
    };
    assert!(near(c, p(5.0, 20.0)));
    assert!(near(s.shape(made[1]).unwrap().at(0.0), p(8.0, 20.0)), "{a0}");
    // A drag of the first point: the copy follows.
    let Geometry::Line { a: pa, .. } = s.curve(l).unwrap().geometry else {
        unreachable!()
    };
    s.drag(pa, p(-12.0, 3.0));
    assert!(near(ends(&s, made[0]).0, p(12.0, 3.0)));
    assert!(s.mirror(&[l], l).is_err());
}

#[test]
fn patterns_copy_the_shape_and_its_constraints() {
    let (mut s, l, _, _) = rectangle();
    let n = s.constraints.len();
    let made = s.rect_pattern(&l, 3, 2, 70.0, 50.0).unwrap();
    assert_eq!(made.len(), 4 * 5);
    // Each copy has the 4 directions and 2 lengths; its fix moves with it.
    assert_eq!(s.constraints.len(), n * 6);
    assert!(s.solve().converged);
    assert!(s.analyze().conflicting.is_empty());
    assert!(s.points.iter().any(|q| near(q.pos, p(140.0, 50.0))));
    let mut s = Sketch::new();
    let c = s.add_circle(p(20.0, 0.0), 3.0).unwrap();
    s.add_constraint(Constraint::Diameter { curve: c, mm: 6.0 }).unwrap();
    let made = s.circular_pattern(&[c], p(0.0, 0.0), 6, 360.0).unwrap();
    assert_eq!(made.len(), 5);
    assert!(s.points.iter().any(|q| near(q.pos, p(-20.0, 0.0))));
    assert_eq!(
        s.constraints
            .iter()
            .filter(|c| matches!(c.constraint, Constraint::Diameter { .. }))
            .count(),
        6
    );
    assert!(s.rect_pattern(&[c], 1, 1, 10.0, 10.0).is_err());
}

#[test]
fn relax_removes_the_dimensions_that_conflict() {
    let (mut s, l, _, _) = rectangle();
    s.add_constraint(Constraint::Length { line: l[2], mm: 50.0 }).unwrap();
    s.solve();
    assert!(!s.analyze().conflicting.is_empty());
    let removed = s.relax();
    assert_eq!(removed, vec![Constraint::Length { line: l[2], mm: 50.0 }]);
    assert!(s.analyze().conflicting.is_empty());
    assert!(s.relax().is_empty());
}

#[test]
fn a_distance_to_a_line_solves() {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(20.0, 0.0));
    let q = s.add_point(p(5.0, 3.0));
    s.add_constraint(Constraint::LineDistance {
        point: q,
        line: l,
        mm: 7.5,
    })
    .unwrap();
    assert!(s.solve().converged);
    let (a, b) = ends(&s, l);
    assert!(((b - a).cross(s.point(q).unwrap() - a) / a.distance(b) - 7.5).abs() < 1e-6);
    assert!(
        s.add_constraint(Constraint::LineDistance {
            point: q,
            line: l,
            mm: f64::NAN
        })
        .is_err()
    );
}

#[test]
fn each_refusal_is_in_the_messages() {
    let src = include_str!("edit.rs");
    let mut n = 0;
    for part in src.split("no(\"").skip(1).chain(src.split("Refused(\"").skip(1)) {
        let text = &part[..part.find('"').unwrap()];
        assert!(edit::MESSAGES.contains(&text), "{text}");
        n += 1;
    }
    assert!(n >= edit::MESSAGES.len());
}

#[test]
fn snaps_to_the_middle_to_crossings_and_in_line() {
    let mut s = Sketch::new();
    let h = s.add_line(p(0.0, 0.0), p(20.0, 0.0));
    let v = s.add_line(p(5.0, -10.0), p(5.0, 20.0));
    // The middle of the horizontal line: a midpoint constraint.
    let sn = snap(&s, p(10.2, 0.3), 1.0, &[]);
    assert!(sn.midpoint && sn.curve == Some(h) && near(sn.at, p(10.0, 0.0)));
    let q = place(&mut s, &sn);
    assert!(
        s.constraints
            .iter()
            .any(|c| c.constraint == Constraint::Midpoint { point: q, line: h })
    );
    // Where the lines cross: on both.
    let sn = snap(&s, p(5.3, 0.4), 1.0, &[]);
    assert!(near(sn.at, p(5.0, 0.0)) && sn.also.is_some(), "{sn:?}");
    let q = place(&mut s, &sn);
    let on: Vec<CurveId> = s
        .constraints
        .iter()
        .filter_map(|c| match c.constraint {
            Constraint::OnCurve { point, curve } if point == q => Some(curve),
            _ => None,
        })
        .collect();
    assert!(on.contains(&h) && on.contains(&v));
    // A free click in line with the end of the vertical line takes its x.
    let sn = snap(&s, p(5.4, 40.0), 1.0, &[]);
    assert!(sn.align[0].is_some() && (sn.at.x - 5.0).abs() < 1e-12 && sn.curve.is_none());
}

#[test]
fn a_line_becomes_perpendicular_parallel_or_tangent() {
    let mut s = Sketch::new();
    // A line at 30°.
    let l = s.add_line(p(0.0, 0.0), p(17.320508, 10.0));
    let Geometry::Line { b, .. } = s.curve(l).unwrap().geometry else {
        unreachable!()
    };
    let from = Snap::on_point(p(17.320508, 10.0), b);
    // Almost at 90° to it: perpendicular, and the end goes there.
    let (end, hints) = line_hints(&s, &from, &Snap::free(p(17.320508 - 5.3, 10.0 + 8.4)));
    assert_eq!(hints, vec![Hint::Perpendicular(l)]);
    let d = end - p(17.320508, 10.0);
    assert!(d.dot(p(0.866_025_4, 0.5)).abs() < 1e-5);
    // Parallel to a line that the pointer went over.
    let start = Snap::free(p(0.0, 30.0));
    let (_, hints) = line_hints_with(&s, &start, &Snap::free(p(17.0, 40.2)), Some(l));
    assert_eq!(hints, vec![Hint::Parallel(l)]);
    // Tangent to a circle that the end touches.
    let c = s.add_circle(p(0.0, -40.0), 10.0).unwrap();
    let touch = Snap::on_curve(p(0.0, -30.0), c);
    let (_, hints) = line_hints(&s, &Snap::free(p(-20.0, -30.5)), &touch);
    assert_eq!(hints, vec![Hint::Tangent(c)]);
}

#[test]
fn collinear_lines_and_driven_dimensions() {
    let mut s = Sketch::new();
    let a = s.add_line(p(0.0, 0.0), p(10.0, 0.0));
    let b = s.add_line(p(15.0, 2.0), p(25.0, 3.0));
    s.add_constraint(Constraint::Collinear { a, b }).unwrap();
    assert!(s.solve().converged);
    let (q, r) = ends(&s, a);
    let (u, w) = ends(&s, b);
    for x in [u, w] {
        assert!(((r - q).cross(x - q) / q.distance(r)).abs() < 1e-6);
    }
    // A rectangle with its width: a second width is driven, not a conflict.
    let (mut s, l, _, _) = rectangle();
    let (id, driven) = s.add_dimension(Constraint::Length { line: l[2], mm: 60.0 }).unwrap();
    assert!(driven && s.is_driven(id));
    assert!(s.analyze().conflicting.is_empty() && s.analyze().redundant.is_empty());
    // It follows the sketch.
    let w = s
        .constraints
        .iter()
        .find(|c| c.constraint == Constraint::Length { line: l[0], mm: 60.0 })
        .unwrap()
        .id;
    s.set_value(w, 75.0).unwrap();
    assert!(s.solve().converged);
    assert!((s.constraint(id).unwrap().value().unwrap() - 75.0).abs() < 1e-6);
    // A dimension that is new is driving.
    let (mut s, l, _, _) = rough();
    let (_, driven) = s.add_dimension(Constraint::Length { line: l, mm: 12.0 }).unwrap();
    assert!(!driven);
    // Driving again: the value of now.
    let (mut s, l, _, _) = rectangle();
    let (id, _) = s.add_dimension(Constraint::Length { line: l[2], mm: 60.0 }).unwrap();
    s.set_driven(id, false);
    assert!(!s.is_driven(id));
    assert!(!s.analyze().redundant.is_empty());
}

fn rough() -> (Sketch, CurveId, (), ()) {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(12.0, 0.0));
    (s, l, (), ())
}

#[test]
fn an_arc_length_solves() {
    let mut s = Sketch::new();
    let arc = s.add_arc(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)).unwrap();
    let Geometry::Arc { center, .. } = s.curve(arc).unwrap().geometry else {
        unreachable!()
    };
    s.add_constraint(Constraint::Fix {
        point: center,
        at: p(0.0, 0.0),
    })
    .unwrap();
    s.add_constraint(Constraint::Radius { curve: arc, mm: 10.0 }).unwrap();
    // A half circle: 10π.
    let id = s
        .add_constraint(Constraint::ArcLength {
            curve: arc,
            mm: 10.0 * std::f64::consts::PI,
        })
        .unwrap();
    assert!(s.solve().converged);
    let Shape::Arc { sweep, .. } = s.shape(arc).unwrap() else {
        panic!("an arc")
    };
    assert!((sweep - std::f64::consts::PI).abs() < 1e-6, "{sweep}");
    assert!((s.measure(s.constraint(id).unwrap()).unwrap() - 10.0 * std::f64::consts::PI).abs() < 1e-6);
    let line = s.add_line(p(0.0, 0.0), p(1.0, 0.0));
    assert!(
        s.add_constraint(Constraint::ArcLength { curve: line, mm: 1.0 })
            .is_err()
    );
    s.set_centerline(line, true);
    assert!(s.is_centerline(line) && s.curve(line).unwrap().construction);
}

#[test]
fn parts_move_turn_scale_and_copy() {
    let (mut s, l, c, [w, _]) = rectangle();
    // The rectangle has a fixed corner: take the fix away to move it.
    s.constraints
        .retain(|x| !matches!(x.constraint, Constraint::Fix { .. }));
    let n = s.curves.len();
    s.transform_parts(&l, &[], &|q| q + p(5.0, 7.0), 1.0, false, false)
        .unwrap();
    assert!(near(s.point(c[0]).unwrap(), p(5.0, 7.0)));
    // A copy turned by 90° round the origin: 4 new lines, no H or V.
    let turn = |q: Point| p(-q.y, q.x);
    let made = s.transform_parts(&l, &[], &turn, 1.0, true, true).unwrap();
    assert_eq!((made.len(), s.curves.len()), (4, n + 4));
    assert!(s.solve().converged);
    assert!(s.points.iter().any(|q| near(q.pos, p(-7.0, 5.0))));
    // Scale 2 round the first corner: the width dimension doubles.
    let base = s.point(c[0]).unwrap();
    s.transform_parts(&l, &[], &|q| base + (q - base) * 2.0, 2.0, false, false)
        .unwrap();
    assert!((s.constraint(w).unwrap().value().unwrap() - 120.0).abs() < 1e-9);
    assert!(s.analyze().conflicting.is_empty());
    let (a, b) = ends(&s, l[0]);
    assert!((a.distance(b) - 120.0).abs() < 1e-6);
    assert!(s.transform_parts(&[], &[], &|q| q, 1.0, false, false).is_err());
}

#[test]
fn a_distance_to_a_centerline_is_a_diameter() {
    let mut s = Sketch::new();
    let axis = s.add_line(p(0.0, 0.0), p(0.0, 40.0));
    for q in s.curve(axis).unwrap().points() {
        let at = s.point(q).unwrap();
        s.add_constraint(Constraint::Fix { point: q, at }).unwrap();
    }
    let q = s.add_point(p(-6.0, 10.0));
    let d = s
        .add_constraint(Constraint::LineDistance {
            point: q,
            line: axis,
            mm: 6.0,
        })
        .unwrap();
    s.set_centerline(axis, true);
    assert_eq!(
        s.constraint(d).unwrap().value(),
        Some(12.0),
        "the value is the diameter"
    );
    assert!(s.solve().converged);
    assert!((s.point(q).unwrap().x.abs() - 6.0).abs() < 1e-6, "the geometry stays");
    s.set_value(d, 20.0).unwrap();
    assert!(s.solve().converged);
    assert!((s.point(q).unwrap().x.abs() - 10.0).abs() < 1e-6);
    assert!((s.measure(s.constraint(d).unwrap()).unwrap().abs() - 20.0).abs() < 1e-6);
}

#[test]
fn a_line_from_the_end_of_a_tangent_arc_is_tangent() {
    // A line to (30, 0), then an arc to (40, -10) round (30, -10).
    let mut s = Sketch::new();
    let a = s.add_point(p(0.0, 0.0));
    let b = s.add_point(p(30.0, 0.0));
    s.add_line_between(a, b).unwrap();
    let c = s.add_point(p(30.0, -10.0));
    let e = s.add_point(p(40.0, -10.0));
    // Counterclockwise in the sketch: from (40, -10) to (30, 0) or back.
    let (start, end) = if (p(40.0, -10.0) - p(30.0, -10.0)).cross(p(30.0, 0.0) - p(30.0, -10.0)) > 0.0 {
        (e, b)
    } else {
        (b, e)
    };
    let arc = s.add_arc_between(c, start, end, 10.0).unwrap();
    let from = Snap::on_point(p(40.0, -10.0), e);
    let (_, hints) = line_hints(&s, &from, &Snap::free(p(40.2, -40.0)));
    assert_eq!(hints, vec![Hint::Tangent(arc)]);
}

#[test]
fn a_circle_tangent_to_three_lines() {
    // A triangle 0,0 - 30,0 - 0,40: its inner circle has radius 10 at (10, 10).
    let mut s = Sketch::new();
    let c = [p(0.0, 0.0), p(30.0, 0.0), p(0.0, 40.0)].map(|q| s.add_point(q));
    let l = [0, 1, 2].map(|i| s.add_line_between(c[i], c[(i + 1) % 3]).unwrap());
    let circle = s.add_tangent_circle(l, p(8.0, 9.0)).unwrap();
    assert!((s.radius(circle).unwrap() - 10.0).abs() < 1e-9);
    let Geometry::Circle { center, .. } = s.curve(circle).unwrap().geometry else {
        unreachable!()
    };
    assert!(near(s.point(center).unwrap(), p(10.0, 10.0)));
    // A click outside the side on the x axis: the circle on that side.
    let other = s.add_tangent_circle(l, p(15.0, -20.0)).unwrap();
    let Geometry::Circle { center, .. } = s.curve(other).unwrap().geometry else {
        unreachable!()
    };
    assert!(s.point(center).unwrap().y < 0.0);
    assert!(s.solve().converged);
    assert!(s.add_tangent_circle([l[0], l[0], l[1]], p(0.0, 0.0)).is_err());
}

#[test]
fn chamfers_with_two_distances_and_with_an_angle() {
    let (mut s, l, c, _) = rectangle();
    let (line, _) = s.chamfer_with(c[2], edit::ChamferKind::Two(4.0, 8.0)).unwrap();
    assert!(s.solve().converged);
    let (a, b) = ends(&s, line);
    let d: Vec<f64> = [a, b].iter().map(|q| q.distance(p(60.0, 40.0))).collect();
    assert!(
        d.iter().any(|x| (x - 4.0).abs() < 1e-6) && d.iter().any(|x| (x - 8.0).abs() < 1e-6),
        "{d:?}"
    );
    // 4 mm at 45°: the other end 4 mm from the corner too.
    let (mut s, _, c, _) = rectangle();
    let (line, _) = s.chamfer_with(c[2], edit::ChamferKind::Angle(4.0, 45.0)).unwrap();
    assert!(s.solve().converged);
    let (a, b) = ends(&s, line);
    for q in [a, b] {
        assert!((q.distance(p(60.0, 40.0)) - 4.0).abs() < 1e-6, "{q:?}");
    }
    assert!(s.analyze().conflicting.is_empty());
    assert_eq!(l.len(), 4);
    let (mut s, _, c, _) = rectangle();
    assert!(s.chamfer_with(c[2], edit::ChamferKind::Angle(4.0, 0.0)).is_err());
}

#[test]
fn automatic_dimensions_fix_the_shape_and_leave_the_place() {
    // Four lines, almost a rectangle, no constraints.
    let mut s = Sketch::new();
    let c = [p(0.0, 0.0), p(50.0, 0.1), p(50.1, 30.0), p(0.0, 30.0)].map(|q| s.add_point(q));
    for i in 0..4 {
        s.add_line_between(c[i], c[(i + 1) % 4]).unwrap();
    }
    let (constraints, dimensions) = s.auto_dimension(&[]);
    assert!(constraints >= 3, "{constraints}");
    assert!(dimensions >= 2, "{dimensions}");
    assert!(s.solve().converged);
    let a = s.analyze();
    assert!(a.conflicting.is_empty() && a.redundant.is_empty(), "{a:?}");
    assert_eq!(a.free, 2, "only the place is free");
    // Again: nothing more.
    assert_eq!(s.auto_dimension(&[]), (0, 0));
    // Center points.
    s.set_center_point(c[0], true);
    assert!(s.is_center_point(c[0]) && !s.is_center_point(c[1]));
}

mod regions {
    use super::*;
    use std::f64::consts::PI;

    fn square(s: &mut Sketch, x0: f64, y0: f64, w: f64) {
        let c = [p(x0, y0), p(x0 + w, y0), p(x0 + w, y0 + w), p(x0, y0 + w)].map(|q| s.add_point(q));
        for i in 0..4 {
            s.add_line_between(c[i], c[(i + 1) % 4]).unwrap();
        }
    }

    fn areas(s: &Sketch) -> Vec<f64> {
        s.regions().iter().map(|r| r.area).collect()
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn a_rectangle_and_a_line_across_it() {
        let (mut s, ..) = rectangle();
        assert_eq!(areas(&s).len(), 1);
        assert!(close(areas(&s)[0], 2400.0, 1e-3));
        // A line across, past both sides: two regions, the ends go.
        s.add_line(p(20.0, -5.0), p(20.0, 45.0));
        let a = areas(&s);
        assert_eq!(a.len(), 2, "{a:?}");
        assert!(close(a[0], 1600.0, 1e-3) && close(a[1], 800.0, 1e-3), "{a:?}");
        // Construction curves and open chains make no region.
        let mut t = Sketch::new();
        t.add_line(p(0.0, 0.0), p(10.0, 0.0));
        let c = t.add_circle(p(0.0, 0.0), 5.0).unwrap();
        t.set_construction(c, true);
        assert!(t.regions().is_empty());
    }

    #[test]
    fn rings_islands_and_islands_in_islands() {
        // A ring: two circles round one center.
        let mut s = Sketch::new();
        s.add_circle(p(0.0, 0.0), 40.0).unwrap();
        s.add_circle(p(0.0, 0.0), 30.0).unwrap();
        let r = s.regions();
        assert_eq!(r.len(), 2);
        // The largest first: the disc (900π), then the ring (700π).
        assert_eq!(r[1].holes.len(), 1);
        assert!(close(r[1].area, PI * 700.0, 0.5), "{}", r[1].area);
        assert!(close(r[0].area, PI * 900.0, 0.5));
        assert!(r[1].contains(p(35.0, 0.0)) && !r[1].contains(p(0.0, 0.0)));
        assert!(close(s.region_at(p(0.0, 0.0)).unwrap().area, PI * 900.0, 0.5));
        // Squares in squares: three regions, each with the next as a hole.
        let mut s = Sketch::new();
        square(&mut s, 0.0, 0.0, 100.0);
        square(&mut s, 20.0, 20.0, 60.0);
        square(&mut s, 40.0, 40.0, 20.0);
        let r = s.regions();
        let a: Vec<f64> = r.iter().map(|r| r.area).collect();
        assert_eq!(a.len(), 3, "{a:?}");
        assert!(close(a[0], 10000.0 - 3600.0, 1e-3) && close(a[1], 3600.0 - 400.0, 1e-3) && close(a[2], 400.0, 1e-3));
        assert_eq!(r.iter().map(|r| r.holes.len()).collect::<Vec<_>>(), vec![1, 1, 0]);
        // A disc with a hexagon in it.
        let mut s = Sketch::new();
        s.add_circle(p(0.0, 0.0), 40.0).unwrap();
        let hex: Vec<PointId> = (0..6)
            .map(|i| {
                let a = PI / 3.0 * f64::from(i);
                s.add_point(p(10.0 * a.cos(), 10.0 * a.sin()))
            })
            .collect();
        for i in 0..6 {
            s.add_line_between(hex[i], hex[(i + 1) % 6]).unwrap();
        }
        let r = s.regions();
        let hex_area = 1.5 * 3.0_f64.sqrt() * 100.0;
        assert_eq!(r.len(), 2);
        assert!(close(r[0].area, PI * 1600.0 - hex_area, 1.0), "{}", r[0].area);
        assert!(close(r[1].area, hex_area, 1e-3));
    }

    #[test]
    fn overlapping_circles_fillets_and_a_circle_in_a_triangle() {
        // Two circles that cross: three regions.
        let mut s = Sketch::new();
        s.add_circle(p(0.0, 0.0), 10.0).unwrap();
        s.add_circle(p(10.0, 0.0), 10.0).unwrap();
        let a = areas(&s);
        assert_eq!(a.len(), 3, "{a:?}");
        let total: f64 = a.iter().sum();
        let lens = 2.0 * 100.0 * (PI / 3.0) - 2.0 * (0.5 * 100.0 * (2.0 * PI / 3.0).sin());
        assert!(close(total, 2.0 * PI * 100.0 - lens, 0.5), "{total}");
        // A rectangle with a fillet: the corner is round.
        let (mut s, _, c, _) = rectangle();
        s.fillet(c[2], 10.0).unwrap();
        s.solve();
        let a = areas(&s);
        assert_eq!(a.len(), 1);
        assert!(close(a[0], 2400.0 - (100.0 - PI * 25.0), 0.2), "{a:?}");
        // A circle that touches the three sides of a triangle: the circle
        // and the three corners.
        let mut s = Sketch::new();
        let c = [p(0.0, 0.0), p(30.0, 0.0), p(0.0, 40.0)].map(|q| s.add_point(q));
        let l = [0, 1, 2].map(|i| s.add_line_between(c[i], c[(i + 1) % 3]).unwrap());
        s.add_tangent_circle(l, p(10.0, 10.0)).unwrap();
        assert!(s.solve().converged);
        let a = areas(&s);
        assert_eq!(a.len(), 4, "{a:?}");
        assert!(close(a.iter().sum::<f64>(), 600.0, 0.3), "{a:?}");
    }

    #[test]
    fn a_region_is_found_again_after_a_change() {
        // A rectangle cut by a line: two regions with different keys.
        let (mut s, l, _, [w, _]) = rectangle();
        s.add_line(p(20.0, -5.0), p(20.0, 45.0));
        let r = s.regions();
        assert_ne!(r[0].sides, r[1].sides);
        let (big, small) = (r[0].key(), r[1].key());
        assert!(r[0].contains(big.at) && r[1].contains(small.at));
        // A width of 80 mm: the same regions, now larger.
        s.set_value(w, 80.0).unwrap();
        assert!(s.solve().converged);
        let big2 = s.find_region(&big).unwrap();
        assert!(close(big2.area, 2400.0, 1e-3), "{}", big2.area);
        assert!(close(s.find_region(&small).unwrap().area, 800.0, 1e-3));
        // A fillet at a corner of the large region changes its curves: it
        // is found by its point and the curves that it keeps.
        let corner = s.curve(l[1]).unwrap().points()[0];
        s.fillet(corner, 5.0).unwrap();
        assert!(s.solve().converged);
        let f = s.find_region(&big).unwrap();
        assert!(f.area < 2400.0 && f.area > 2300.0, "{}", f.area);
        // With the right side gone, the large region is gone.
        s.remove_curve(l[1]);
        assert!(s.find_region(&big).is_none());
        // A ring and its disc have the same curves, other sides.
        let mut s = Sketch::new();
        s.add_circle(p(0.0, 0.0), 40.0).unwrap();
        s.add_circle(p(0.0, 0.0), 30.0).unwrap();
        let r = s.regions();
        assert_ne!(r[0].sides, r[1].sides);
        assert_eq!(s.find_region(&r[1].key()).unwrap().holes.len(), 1);
    }
}

mod splines {
    use super::*;

    /// The directions of two spline pieces (or a line) away from the
    /// point that they share: on one line when they are tangent.
    fn bend(s: &Sketch, a: CurveId, b: CurveId) -> f64 {
        let away = |id: CurveId, shared: PointId| -> Point {
            let pt = |p| s.point(p).unwrap();
            match s.curve(id).unwrap().geometry {
                Geometry::Bezier { start, c1, c2, .. } => {
                    if start == shared {
                        pt(c1) - pt(start)
                    } else {
                        pt(c2) - pt(shared)
                    }
                }
                Geometry::Line { a, b } => {
                    let o = if a == shared { b } else { a };
                    pt(o) - pt(shared)
                }
                _ => unreachable!(),
            }
        };
        let (ea, eb) = (s.curve(a).unwrap().ends().unwrap(), s.curve(b).unwrap().ends().unwrap());
        let shared = [ea.0, ea.1]
            .into_iter()
            .find(|p| [eb.0, eb.1].contains(p) || s.joined(*p).iter().any(|q| [eb.0, eb.1].contains(q)))
            .unwrap();
        let other = if [eb.0, eb.1].contains(&shared) {
            shared
        } else {
            *s.joined(shared).iter().find(|q| [eb.0, eb.1].contains(q)).unwrap()
        };
        let (u, v) = (away(a, shared), away(b, other));
        // Smooth: the directions away from the joint are opposite.
        assert!(u.dot(v) < 0.0, "a cusp at the joint");
        u.cross(v) / (u.length() * v.length())
    }

    #[test]
    fn a_spline_goes_through_its_points_and_stays_smooth() {
        let mut s = Sketch::new();
        let pts = [p(0.0, 0.0), p(20.0, 15.0), p(40.0, 0.0), p(60.0, 10.0)];
        let pieces = s.add_spline(&pts, false).unwrap();
        assert_eq!(pieces.len(), 3);
        assert_eq!(s.constraints.len(), 2, "a tangent at each joint");
        for (i, c) in pieces.iter().enumerate() {
            let sh = s.shape(*c).unwrap();
            assert!(sh.at(0.0).distance(pts[i]) < 1e-12 && sh.at(1.0).distance(pts[i + 1]) < 1e-12);
        }
        assert!(s.solve().converged);
        // Move a point far (its handles stay) and solve: the joints stay
        // smooth, with no cusp.
        let mid = s.curve(pieces[1]).unwrap().ends().unwrap().0;
        s.add_constraint(Constraint::Fix {
            point: mid,
            at: p(20.0, 45.0),
        })
        .unwrap();
        assert!(s.solve().converged);
        assert!(s.point(mid).unwrap().distance(p(20.0, 45.0)) < 1e-6);
        for w in pieces.windows(2) {
            assert!(bend(&s, w[0], w[1]).abs() < 1e-6, "{}", bend(&s, w[0], w[1]));
        }
        // A drag of a point takes its handles.
        assert_eq!(s.with_handles(&[mid]).len(), 3, "the point and its two handles");
        // Saved and loaded.
        let back: Sketch = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_line_tangent_to_a_spline_and_a_point_on_it() {
        let mut s = Sketch::new();
        let pieces = s
            .add_spline(&[p(0.0, 0.0), p(20.0, 20.0), p(40.0, 0.0)], false)
            .unwrap();
        let start = s.curve(pieces[0]).unwrap().ends().unwrap().0;
        let far = s.add_point(p(-20.0, -5.0));
        let line = s.add_line_between(far, start).unwrap();
        s.add_constraint(Constraint::Tangent {
            a: line,
            b: pieces[0],
            inside: false,
        })
        .unwrap();
        assert!(s.solve().converged);
        assert!(bend(&s, line, pieces[0]).abs() < 1e-6);
        // A tangent needs a shared end.
        let lone = s.add_line(p(0.0, 40.0), p(10.0, 40.0));
        assert!(
            s.add_constraint(Constraint::Tangent {
                a: lone,
                b: pieces[1],
                inside: false
            })
            .is_err()
        );
        // A point on the spline.
        let q = s.add_point(p(30.0, 30.0));
        s.add_constraint(Constraint::OnCurve {
            point: q,
            curve: pieces[1],
        })
        .unwrap();
        assert!(s.solve().converged);
        let sh = s.shape(pieces[1]).unwrap();
        let at = s.point(q).unwrap();
        assert!(sh.nearest(at).0.distance(at) < 1e-6);
        // Trim does not take splines yet.
        assert!(s.trim(pieces[1], p(30.0, 10.0)).is_err());
    }

    #[test]
    fn a_circle_region_has_no_closing_segment_of_length_zero() {
        // The outline of a circle ends at 360°, a rounding error away from
        // 0°. A zero-length closing segment drew a spike on the canvas.
        let mut s = Sketch::new();
        s.add_circle(p(3.0, 4.0), 12.5).unwrap();
        let r = s.regions();
        assert_eq!(r.len(), 1);
        for line in r[0].outer.flatten(0.01) {
            assert!(line.closed);
            assert!(
                line.segments().all(|(a, b)| a.distance(b) > 1e-6),
                "{:?}",
                line.points.last()
            );
        }
    }

    #[test]
    fn a_closed_spline_makes_regions() {
        let mut s = Sketch::new();
        let pts = [p(0.0, 0.0), p(40.0, 0.0), p(40.0, 30.0), p(0.0, 30.0)];
        let pieces = s.add_spline(&pts, true).unwrap();
        assert_eq!(pieces.len(), 4);
        assert_eq!(s.constraints.len(), 4);
        let r = s.regions();
        assert_eq!(r.len(), 1);
        // A spline through the corners bulges out of the 40 × 30 box.
        assert!(r[0].area > 1200.0 && r[0].area < 1800.0, "{}", r[0].area);
        // A line across: two regions with the same area in all.
        s.add_line(p(20.0, -20.0), p(20.0, 50.0));
        let cut = s.regions();
        assert_eq!(cut.len(), 2, "{:?}", cut.iter().map(|r| r.area).collect::<Vec<_>>());
        let sum: f64 = cut.iter().map(|r| r.area).sum();
        assert!((sum - r[0].area).abs() < 0.5, "{sum} {}", r[0].area);
        // Two crossing splines.
        let mut t = Sketch::new();
        t.add_spline(&[p(0.0, 0.0), p(20.0, 20.0), p(40.0, 0.0)], false)
            .unwrap();
        t.add_spline(&[p(0.0, 10.0), p(20.0, -10.0), p(40.0, 10.0)], false)
            .unwrap();
        let a = t.shape(t.curves[0].id).unwrap();
        let b = t.shape(t.curves[2].id).unwrap();
        let x = crate::shape::crossings(&a, &b);
        assert_eq!(x.len(), 1, "{x:?}");
        assert!(a.nearest(x[0]).0.distance(x[0]) < 1e-8 && b.nearest(x[0]).0.distance(x[0]) < 1e-8);
    }
}
