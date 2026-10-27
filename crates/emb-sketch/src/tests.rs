use emb_geom::Point;
use proptest::prelude::*;

use super::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-6
}

/// A rectangle of four lines with shared corners, a little out of shape.
/// Returns the sketch, the four lines (bottom, right, top, left) and the
/// four corners.
fn rough_rectangle() -> (Sketch, [CurveId; 4], [PointId; 4]) {
    let mut s = Sketch::new();
    let c = [p(0.0, 0.0), p(52.0, 1.0), p(49.0, 31.0), p(-1.0, 29.0)].map(|q| s.add_point(q));
    let l = [0, 1, 2, 3].map(|i| s.add_line_between(c[i], c[(i + 1) % 4]).unwrap());
    (s, l, c)
}

/// The rectangle with horizontal and vertical sides, 60 × 40, the first
/// corner fixed at (10, 10).
fn rectangle() -> (Sketch, [CurveId; 4], [PointId; 4]) {
    let (mut s, l, c) = rough_rectangle();
    for i in [0, 2] {
        s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
    }
    for i in [1, 3] {
        s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
    }
    s.add_constraint(Constraint::Length { line: l[0], mm: 60.0 }).unwrap();
    s.add_constraint(Constraint::Length { line: l[1], mm: 40.0 }).unwrap();
    s.add_constraint(Constraint::Fix {
        point: c[0],
        at: p(10.0, 10.0),
    })
    .unwrap();
    (s, l, c)
}

#[test]
fn a_rectangle_solves_and_is_fully_constrained() {
    let (mut s, _, c) = rectangle();
    let r = s.solve();
    assert!(r.converged, "{r:?}");
    assert!(near(s.point(c[0]).unwrap(), p(10.0, 10.0)));
    assert!(near(s.point(c[1]).unwrap(), p(70.0, 10.0)));
    assert!(near(s.point(c[2]).unwrap(), p(70.0, 50.0)));
    assert!(near(s.point(c[3]).unwrap(), p(10.0, 50.0)));
    let a = s.analyze();
    assert_eq!(a.free, 0, "{a:?}");
    assert!(a.is_fully_constrained());
    assert_eq!(a.fixed_points.len(), 4);
    assert_eq!(a.fixed_curves.len(), 4);
    assert!(a.redundant.is_empty() && a.conflicting.is_empty());
}

#[test]
fn free_motions_without_a_fixed_point() {
    let (mut s, l, _) = rough_rectangle();
    for i in [0, 2] {
        s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
    }
    for i in [1, 3] {
        s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
    }
    assert!(s.solve().converged);
    // 8 variables, 4 equations: width, height and the place.
    assert_eq!(s.analyze().free, 4);
    s.add_constraint(Constraint::Length { line: l[0], mm: 60.0 }).unwrap();
    s.add_constraint(Constraint::Length { line: l[1], mm: 40.0 }).unwrap();
    assert!(s.solve().converged);
    let a = s.analyze();
    assert_eq!(a.free, 2, "it can still move as a whole");
    assert!(a.fixed_points.is_empty());
}

#[test]
fn a_repeated_constraint_is_redundant() {
    let (mut s, l, _) = rectangle();
    let extra = s.add_constraint(Constraint::Parallel { a: l[0], b: l[2] }).unwrap();
    assert!(s.solve().converged);
    let a = s.analyze();
    assert_eq!(a.redundant, vec![extra]);
    assert!(a.conflicting.is_empty());
}

#[test]
fn a_conflict_is_named() {
    let (mut s, l, _) = rectangle();
    let bad = s.add_constraint(Constraint::Length { line: l[2], mm: 75.0 }).unwrap();
    let r = s.solve();
    assert!(!r.converged);
    let a = s.analyze();
    assert_eq!(a.conflicting, vec![bad], "{a:?}");
    // Remove it, and the sketch solves again.
    assert!(s.remove_constraint(bad));
    assert!(s.solve().converged);
    assert!(s.analyze().conflicting.is_empty());
}

#[test]
fn a_change_of_a_dimension_moves_the_sketch() {
    let (mut s, _, c) = rectangle();
    s.solve();
    let width = s
        .constraints
        .iter()
        .find(|c| c.constraint.value() == Some(60.0))
        .unwrap()
        .id;
    s.set_value(width, 80.0).unwrap();
    assert!(s.solve().converged);
    assert!(near(s.point(c[2]).unwrap(), p(90.0, 50.0)));
    assert_eq!(s.set_value(width, -1.0), Err(SketchError::BadValue));
    assert_eq!(s.set_value(width, f64::NAN), Err(SketchError::BadValue));
}

#[test]
fn circles_arcs_and_tangents() {
    let mut s = Sketch::new();
    let base = s.add_line(p(0.0, 0.0), p(100.0, 0.5));
    let circle = s.add_circle(p(40.0, 12.0), 8.0).unwrap();
    let ring = s.add_circle(p(41.0, 13.0), 3.0).unwrap();
    let arc = s.add_arc(p(80.0, 30.0), p(90.0, 30.0), p(80.0, 41.0)).unwrap();
    s.add_constraint(Constraint::Horizontal { line: base }).unwrap();
    s.add_constraint(Constraint::Tangent {
        a: base,
        b: circle,
        inside: false,
    })
    .unwrap();
    s.add_constraint(Constraint::Radius {
        curve: circle,
        mm: 10.0,
    })
    .unwrap();
    s.add_constraint(Constraint::Concentric { a: circle, b: ring }).unwrap();
    s.add_constraint(Constraint::Diameter { curve: ring, mm: 4.0 }).unwrap();
    s.add_constraint(Constraint::Equal { a: arc, b: circle }).unwrap();
    s.add_constraint(Constraint::Tangent {
        a: circle,
        b: arc,
        inside: false,
    })
    .unwrap();
    let r = s.solve();
    assert!(r.converged, "{r:?}");
    let Geometry::Line { a, b } = s.curve(base).unwrap().geometry else {
        panic!("a line")
    };
    let y = s.point(a).unwrap().y;
    assert!((s.point(b).unwrap().y - y).abs() < 1e-9);
    let Geometry::Circle { center, radius } = s.curve(circle).unwrap().geometry else {
        panic!("a circle")
    };
    let cc = s.point(center).unwrap();
    assert!((radius - 10.0).abs() < 1e-9);
    assert!(((cc.y - y).abs() - 10.0).abs() < 1e-9, "tangent to the line");
    assert!((s.radius(ring).unwrap() - 2.0).abs() < 1e-9);
    let Geometry::Arc {
        center: ac,
        start,
        end,
        radius: ar,
    } = s.curve(arc).unwrap().geometry
    else {
        panic!("an arc")
    };
    assert!((ar - 10.0).abs() < 1e-9, "equal to the circle");
    let acp = s.point(ac).unwrap();
    assert!((acp.distance(cc) - 20.0).abs() < 1e-9, "tangent circles");
    for q in [start, end] {
        assert!(
            (s.point(q).unwrap().distance(acp) - 10.0).abs() < 1e-9,
            "the ends are on the arc"
        );
    }
    // The path of the arc goes from its start to its end.
    let path = s.curve_path(arc).unwrap();
    let pts: Vec<Point> = path.points().collect();
    assert!(near(pts[0], s.point(start).unwrap()));
    assert!(near(*pts.last().unwrap(), s.point(end).unwrap()));
}

#[test]
fn points_on_curves_midpoints_symmetry_and_angles() {
    let mut s = Sketch::new();
    let axis = s.add_line(p(0.0, -20.0), p(0.5, 20.0));
    let l1 = s.add_line(p(-10.0, 0.0), p(-3.0, 8.0));
    let l2 = s.add_line(p(11.0, 1.0), p(4.0, 7.0));
    let mid = s.add_point(p(3.0, 3.0));
    let on = s.add_point(p(30.0, 30.0));
    let circle = s.add_circle(p(0.0, 40.0), 5.0).unwrap();
    s.add_constraint(Constraint::Vertical { line: axis }).unwrap();
    let ends = |s: &Sketch, l: CurveId| match s.curve(l).unwrap().geometry {
        Geometry::Line { a, b } => (a, b),
        _ => unreachable!(),
    };
    let (a1, b1) = ends(&s, l1);
    let (a2, b2) = ends(&s, l2);
    s.add_constraint(Constraint::Symmetric {
        a: a1,
        b: a2,
        line: axis,
    })
    .unwrap();
    s.add_constraint(Constraint::Symmetric {
        a: b1,
        b: b2,
        line: axis,
    })
    .unwrap();
    s.add_constraint(Constraint::Midpoint { point: mid, line: l2 }).unwrap();
    s.add_constraint(Constraint::OnCurve {
        point: on,
        curve: circle,
    })
    .unwrap();
    s.add_constraint(Constraint::Angle {
        a: axis,
        b: l1,
        deg: 30.0,
    })
    .unwrap();
    s.add_constraint(Constraint::Distance { a: a1, b: b1, mm: 12.0 })
        .unwrap();
    let r = s.solve();
    assert!(r.converged, "{r:?}");
    let q = |id| s.point(id).unwrap();
    let ax = q(ends(&s, axis).0).x;
    assert!((q(a1).x + q(a2).x - 2.0 * ax).abs() < 1e-9 && (q(a1).y - q(a2).y).abs() < 1e-9);
    assert!(near(q(mid), q(a2).midpoint(q(b2))));
    let Geometry::Circle { center, radius } = s.curve(circle).unwrap().geometry else {
        unreachable!()
    };
    assert!((q(on).distance(q(center)) - radius).abs() < 1e-9);
    assert!((q(a1).distance(q(b1)) - 12.0).abs() < 1e-9);
    let (da, db) = (q(ends(&s, axis).1) - q(ends(&s, axis).0), q(b1) - q(a1));
    let angle = math::to_degrees(math::atan2(da.cross(db), da.dot(db)));
    assert!((angle - 30.0).abs() < 1e-6, "{angle}");
}

#[test]
fn a_drag_keeps_the_constraints() {
    // A free point goes to the pointer.
    let mut s = Sketch::new();
    let free = s.add_point(p(0.0, 0.0));
    assert!(s.drag(free, p(5.0, 7.0)).converged);
    assert!(near(s.point(free).unwrap(), p(5.0, 7.0)));
    // A line 30 mm long from a fixed point: its end goes toward the pointer.
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(30.0, 0.0));
    let Geometry::Line { a, b } = s.curve(l).unwrap().geometry else {
        unreachable!()
    };
    s.add_constraint(Constraint::Fix {
        point: a,
        at: p(0.0, 0.0),
    })
    .unwrap();
    s.add_constraint(Constraint::Length { line: l, mm: 30.0 }).unwrap();
    let r = s.drag(b, p(0.0, 50.0));
    assert!(r.converged, "{r:?}");
    assert!(near(s.point(a).unwrap(), p(0.0, 0.0)));
    assert!(near(s.point(b).unwrap(), p(0.0, 30.0)), "{:?}", s.point(b));
    // In a rectangle with free motions, the dragged corner follows the pointer.
    let (mut s, l, c) = rough_rectangle();
    for i in [0, 2] {
        s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
    }
    for i in [1, 3] {
        s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
    }
    s.solve();
    assert!(s.drag(c[2], p(80.0, 60.0)).converged);
    assert!(s.point(c[2]).unwrap().distance(p(80.0, 60.0)) < 1e-3);
}

#[test]
fn a_line_drags_with_both_ends() {
    let (mut s, l, c) = rough_rectangle();
    for i in [0, 2] {
        s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
    }
    for i in [1, 3] {
        s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
    }
    s.solve();
    // The top line goes 10 mm up: the rectangle grows, the bottom stays.
    let (a, b) = (s.point(c[2]).unwrap(), s.point(c[3]).unwrap());
    let bottom = s.point(c[0]).unwrap();
    let up = p(0.0, 10.0);
    assert!(s.drag_points(&[(c[2], a + up), (c[3], b + up)]).converged);
    assert!(near(s.point(c[2]).unwrap(), a + up));
    assert!(near(s.point(c[0]).unwrap(), bottom));
}

#[test]
fn wrong_curves_and_missing_parts_are_refused() {
    let mut s = Sketch::new();
    let l = s.add_line(p(0.0, 0.0), p(10.0, 0.0));
    let c = s.add_circle(p(0.0, 10.0), 3.0).unwrap();
    assert!(matches!(
        s.add_constraint(Constraint::Horizontal { line: c }),
        Err(SketchError::WrongCurves(_))
    ));
    assert!(matches!(
        s.add_constraint(Constraint::Radius { curve: l, mm: 2.0 }),
        Err(SketchError::WrongCurves(_))
    ));
    assert!(matches!(
        s.add_constraint(Constraint::Equal { a: l, b: c }),
        Err(SketchError::WrongCurves(_))
    ));
    assert!(matches!(
        s.add_constraint(Constraint::Parallel { a: l, b: l }),
        Err(SketchError::WrongCurves(_))
    ));
    assert_eq!(
        s.add_constraint(Constraint::Horizontal { line: CurveId(999) }),
        Err(SketchError::Missing)
    );
    assert_eq!(s.add_circle(p(0.0, 0.0), 0.0), Err(SketchError::BadRadius));
    assert_eq!(
        s.add_constraint(Constraint::Length { line: l, mm: 1e9 }),
        Err(SketchError::BadValue)
    );
}

#[test]
fn removing_a_curve_removes_its_constraints_and_points() {
    let (mut s, l, c) = rectangle();
    let n_points = s.points.len();
    assert!(s.remove_curve(l[0]));
    // The corners stay (the other lines use them); the length of the line goes.
    assert_eq!(s.points.len(), n_points);
    assert!(!s.constraints.iter().any(|x| x.constraint.curves().contains(&l[0])));
    assert!(s.remove_curve(l[1]));
    // Corner 1 has no line now.
    assert!(s.point(c[1]).is_none());
    assert!(s.point(c[0]).is_some());
}

#[test]
fn json_round_trip() {
    let (mut s, _, _) = rectangle();
    let arc = s.add_arc(p(0.0, 0.0), p(5.0, 0.0), p(0.0, 5.0)).unwrap();
    s.set_construction(arc, true);
    s.solve();
    let text = serde_json::to_string(&s).unwrap();
    let back: Sketch = serde_json::from_str(&text).unwrap();
    assert_eq!(back, s);
    // New ids do not repeat old ones.
    let mut back = back;
    let id = back.add_point(p(1.0, 1.0));
    assert!(s.points.iter().all(|q| q.id != id));
}

/// A chain of `n` lines with shared ends, each 10 mm long, every other one
/// horizontal (the others at 30°), and the first point fixed: as drawn,
/// so the constraints are true.
fn chain(n: usize) -> (Sketch, PointId) {
    let mut s = Sketch::new();
    let mut prev = s.add_point(p(0.0, 0.0));
    let first = prev;
    let mut at = p(0.0, 0.0);
    for i in 0..n {
        let (sin, cos) = if i % 2 == 0 { (0.0, 1.0) } else { (0.5, 0.75f64.sqrt()) };
        at += p(10.0 * cos, 10.0 * sin);
        let q = s.add_point(at);
        let l = s.add_line_between(prev, q).unwrap();
        s.add_constraint(Constraint::Length { line: l, mm: 10.0 }).unwrap();
        if i % 2 == 0 {
            s.add_constraint(Constraint::Horizontal { line: l }).unwrap();
        }
        prev = q;
    }
    s.add_constraint(Constraint::Fix {
        point: first,
        at: p(0.0, 0.0),
    })
    .unwrap();
    s.solve();
    (s, prev)
}

#[test]
fn two_hundred_curves_solve_fast_enough_for_a_drag() {
    let (mut s, last) = chain(200);
    let to = s.point(last).unwrap() + p(-15.0, 20.0);
    let t = std::time::Instant::now();
    let r = s.drag(last, to);
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(r.converged, "{r:?}");
    eprintln!("drag of a chain of 200 lines: {ms:.2} ms");
    // The exit test of the milestone: under 20 ms (release build).
    let limit = if cfg!(debug_assertions) { 2000.0 } else { 20.0 };
    assert!(ms < limit, "{ms:.1} ms");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// A random polygon with constraints made from its own shape always
    /// solves after its points move a little.
    #[test]
    fn solvable_sketches_solve(
        pts in prop::collection::vec((-50.0f64..50.0, -50.0f64..50.0), 3..8),
        kinds in prop::collection::vec(0u8..4, 8),
        noise in prop::collection::vec((-1.0f64..1.0, -1.0f64..1.0), 8),
    ) {
        let mut s = Sketch::new();
        let ids: Vec<PointId> = pts.iter().map(|(x, y)| s.add_point(p(*x, *y))).collect();
        let n = ids.len();
        // No two points at one place (the shape would have no direction).
        for i in 0..n {
            for j in i + 1..n {
                prop_assume!(s.point(ids[i]).unwrap().distance(s.point(ids[j]).unwrap()) > 1.0);
            }
        }
        let lines: Vec<CurveId> = (0..n).map(|i| s.add_line_between(ids[i], ids[(i + 1) % n]).unwrap()).collect();
        let len = |s: &Sketch, l: CurveId| match s.curve(l).unwrap().geometry {
            Geometry::Line { a, b } => s.point(a).unwrap().distance(s.point(b).unwrap()),
            _ => unreachable!(),
        };
        for (i, l) in lines.iter().enumerate() {
            match kinds[i % kinds.len()] {
                0 => { s.add_constraint(Constraint::Length { line: *l, mm: len(&s, *l) }).unwrap(); }
                1 => {
                    let (a, b) = (ids[i], ids[(i + 1) % n]);
                    let d = s.point(b).unwrap() - s.point(a).unwrap();
                    s.add_constraint(Constraint::HorizontalDistance { a, b, mm: d.x }).unwrap();
                }
                2 => {
                    let next = lines[(i + 1) % n];
                    let (da, db) = match (s.curve(*l).unwrap().geometry, s.curve(next).unwrap().geometry) {
                        (Geometry::Line { a, b }, Geometry::Line { a: c, b: d }) => (
                            s.point(b).unwrap() - s.point(a).unwrap(),
                            s.point(d).unwrap() - s.point(c).unwrap(),
                        ),
                        _ => unreachable!(),
                    };
                    let deg = math::to_degrees(math::atan2(da.cross(db), da.dot(db)));
                    s.add_constraint(Constraint::Angle { a: *l, b: next, deg }).unwrap();
                }
                _ => {}
            }
        }
        let first = s.point(ids[0]).unwrap();
        s.add_constraint(Constraint::Fix { point: ids[0], at: first }).unwrap();
        for (k, id) in ids.iter().enumerate() {
            let q = s.point(*id).unwrap();
            let (dx, dy) = noise[k % noise.len()];
            s.set_point(*id, q + p(dx, dy));
        }
        let r = s.solve();
        prop_assert!(r.converged, "{:?}", r);
        prop_assert!(r.max_error < 1e-6);
    }
}
