//! The geometry that a draw tool makes from its clicks, with the
//! constraints that keep its shape.

use super::*;

/// The center of the circle through three points.
fn circle_through(a: Point, b: Point, c: Point) -> Option<Point> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.length_sq(), b.length_sq(), c.length_sq());
    Some(Point::new(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    ))
}

/// The options of the draw tools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MakeOpts {
    pub(super) sides: u32,
    /// A polygon round its circle (the sides touch it), else in it.
    pub(super) outside: bool,
    pub(super) construction: bool,
    pub(super) slot: SlotKind,
}

impl MakeOpts {
    pub(super) fn of(e: &SketchEdit) -> MakeOpts {
        MakeOpts {
            sides: e.polygon_sides,
            outside: e.polygon_outside,
            construction: e.construction,
            slot: e.slot_kind,
        }
    }
}

/// A straight slot round centers `c1` and `c2`, as wide as twice the
/// distance of `w` from the line between them: the two sides, the two half
/// circles, and the line between the centers (construction).
fn straight_slot(sk: &mut Sketch, c1: &Snap, c2: &Snap, w: Point) -> Option<(Vec<CurveId>, CurveId)> {
    let place = infer::place;
    let dir = (c2.at - c1.at).normalized()?;
    let n = dir.perp();
    let r = (w - c1.at).dot(n).abs();
    if r < 1e-6 {
        return None;
    }
    let a = place(sk, c1);
    let b = place(sk, c2);
    if a == b {
        return None;
    }
    let (pa, pb) = (sk.point(a)?, sk.point(b)?);
    let p1 = sk.add_point(pa + n * r);
    let p2 = sk.add_point(pb + n * r);
    let p3 = sk.add_point(pb - n * r);
    let p4 = sk.add_point(pa - n * r);
    let top = sk.add_line_between(p1, p2).ok()?;
    let bottom = sk.add_line_between(p3, p4).ok()?;
    // Round the far side of each center, counterclockwise.
    let end_b = sk.add_arc_between(b, p3, p2, r).ok()?;
    let end_a = sk.add_arc_between(a, p1, p4, r).ok()?;
    for (l, arc) in [(top, end_a), (top, end_b), (bottom, end_a), (bottom, end_b)] {
        let _ = sk.add_constraint(Constraint::Tangent {
            a: l,
            b: arc,
            inside: false,
        });
    }
    let _ = sk.add_constraint(Constraint::Equal { a: end_a, b: end_b });
    // The line between the centers (construction).
    let axis = sk.add_line_between(a, b).ok()?;
    sk.set_construction(axis, true);
    let (_, hints) = infer::line_hints(sk, &Snap::free(pa), &Snap::free(pb));
    infer::apply_hints(sk, axis, &hints);
    Some((vec![top, bottom, end_a, end_b], axis))
}

/// A curved slot: its path is the arc round `o` with radius `big_r` from
/// angle `a0` to `a1` (counterclockwise), `r` the half width. Two arcs on
/// the sides (round `o`), two half circles at the ends, tangent to them,
/// and the path as a construction arc.
fn arc_slot(sk: &mut Sketch, o: Point, big_r: f64, a0: f64, a1: f64, r: f64) -> Option<Vec<CurveId>> {
    if !(r > 1e-6 && r < big_r - 1e-6) {
        return None;
    }
    let u = |a: f64| Point::new(math::cos(a), math::sin(a));
    let oc = sk.add_point(o);
    let s = sk.add_point(o + u(a0) * big_r);
    let e = sk.add_point(o + u(a1) * big_r);
    let path = sk.add_arc_between(oc, s, e, big_r).ok()?;
    sk.set_construction(path, true);
    let po_s = sk.add_point(o + u(a0) * (big_r + r));
    let pi_s = sk.add_point(o + u(a0) * (big_r - r));
    let po_e = sk.add_point(o + u(a1) * (big_r + r));
    let pi_e = sk.add_point(o + u(a1) * (big_r - r));
    let outer = sk.add_arc_between(oc, po_s, po_e, big_r + r).ok()?;
    let inner = sk.add_arc_between(oc, pi_s, pi_e, big_r - r).ok()?;
    // The ends go round beyond the path.
    let cap_e = sk.add_arc_between(e, po_e, pi_e, r).ok()?;
    let cap_s = sk.add_arc_between(s, pi_s, po_s, r).ok()?;
    for (side, cap, inside) in [
        (outer, cap_e, true),
        (outer, cap_s, true),
        (inner, cap_e, false),
        (inner, cap_s, false),
    ] {
        let _ = sk.add_constraint(Constraint::Tangent {
            a: side,
            b: cap,
            inside,
        });
    }
    let _ = sk.add_constraint(Constraint::Equal { a: cap_s, b: cap_e });
    Some(vec![outer, inner, cap_s, cap_e])
}

/// The four sides of a rectangle with corners `ids` (in order), with the
/// constraints `c` of the sides 0 and 1 (the others are parallel).
fn rectangle_sides(sk: &mut Sketch, ids: [PointId; 4]) -> Vec<CurveId> {
    (0..4)
        .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % 4]).ok())
        .collect()
}

/// The geometry that a draw tool makes from its clicks; `None` if it needs
/// more clicks. A click that snapped to a point uses it; a click on a curve
/// puts its new point on the curve. It returns the label of the undo step.
pub(super) fn make(tool: SketchTool, sk: &mut Sketch, clicks: &[Snap], opts: MakeOpts) -> Option<&'static str> {
    let (sides, construction) = (opts.sides, opts.construction);
    let place = infer::place;
    let mut made: Vec<CurveId> = Vec::new();
    let label = match (tool, clicks) {
        (SketchTool::Point, [a]) => {
            place(sk, a);
            "Sketch: Point"
        }
        (SketchTool::Line, [.., a, b]) => {
            let pa = place(sk, a);
            let pb = place(sk, b);
            if pa == pb {
                return None;
            }
            made.extend(sk.add_line_between(pa, pb).ok());
            "Sketch: Line"
        }
        (SketchTool::Rectangle, [a, b]) => {
            let (pa, pb) = (a.at, b.at);
            if (pa.x - pb.x).abs() < 1e-6 || (pa.y - pb.y).abs() < 1e-6 {
                return None;
            }
            let p0 = place(sk, a);
            let p1 = sk.add_point(Point::new(pb.x, pa.y));
            let p2 = place(sk, b);
            let p3 = sk.add_point(Point::new(pa.x, pb.y));
            let ids = [p0, p1, p2, p3];
            let lines: Vec<CurveId> = (0..4)
                .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % 4]).ok())
                .collect();
            for (i, l) in lines.iter().enumerate() {
                let c = if i % 2 == 0 {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                };
                let _ = sk.add_constraint(c);
            }
            made.extend(lines);
            "Sketch: Rectangle"
        }
        (SketchTool::RectCenter, [c, k]) => {
            let (cp, kp) = (c.at, k.at);
            if (cp.x - kp.x).abs() < 1e-6 || (cp.y - kp.y).abs() < 1e-6 {
                return None;
            }
            let o = cp * 2.0 - kp;
            let p0 = place(sk, k);
            let ids = [
                p0,
                sk.add_point(Point::new(o.x, kp.y)),
                sk.add_point(o),
                sk.add_point(Point::new(kp.x, o.y)),
            ];
            let lines = rectangle_sides(sk, ids);
            for (i, l) in lines.iter().enumerate() {
                let c = if i % 2 == 0 {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                };
                let _ = sk.add_constraint(c);
            }
            // The center on the diagonals (construction lines).
            let center = place(sk, c);
            for (a, b) in [(ids[0], ids[2]), (ids[1], ids[3])] {
                if let Ok(d) = sk.add_line_between(a, b) {
                    sk.set_construction(d, true);
                    let _ = sk.add_constraint(Constraint::Midpoint { point: center, line: d });
                    break;
                }
            }
            if let Ok(d) = sk.add_line_between(ids[1], ids[3]) {
                sk.set_construction(d, true);
            }
            made.extend(lines);
            "Sketch: Center Rectangle"
        }
        (SketchTool::Rect3, [a, b, c]) => {
            let dir = (b.at - a.at).normalized()?;
            let n = dir.perp();
            let h = (c.at - b.at).dot(n);
            if h.abs() < 1e-6 {
                return None;
            }
            let p0 = place(sk, a);
            let p1 = place(sk, b);
            if p0 == p1 {
                return None;
            }
            let (q1, q0) = (sk.point(p1)?, sk.point(p0)?);
            let ids = [p0, p1, sk.add_point(q1 + n * h), sk.add_point(q0 + n * h)];
            let lines = rectangle_sides(sk, ids);
            if let [l0, l1, l2, l3] = lines.as_slice() {
                // The first side horizontal or vertical when it is almost.
                let (_, hints) = infer::line_hints(sk, &Snap::free(q0), &Snap::free(q1));
                infer::apply_hints(sk, *l0, &hints);
                for c in [
                    Constraint::Perpendicular { a: *l0, b: *l1 },
                    Constraint::Parallel { a: *l0, b: *l2 },
                    Constraint::Parallel { a: *l1, b: *l3 },
                ] {
                    let _ = sk.add_constraint(c);
                }
            }
            made.extend(lines);
            "Sketch: Three-Point Rectangle"
        }
        (SketchTool::Slot, [c1, c2, w]) if !opts.slot.is_arc() => {
            match opts.slot {
                SlotKind::Overall => {
                    // The ends: the centers are a half width in.
                    let dir = (c2.at - c1.at).normalized()?;
                    let r = (w.at - c1.at).dot(dir.perp()).abs();
                    if c1.at.distance(c2.at) <= 2.0 * r + 1e-6 {
                        return None;
                    }
                    let (a, b) = (Snap::free(c1.at + dir * r), Snap::free(c2.at - dir * r));
                    let (curves, _) = straight_slot(sk, &a, &b, w.at)?;
                    made.extend(curves);
                }
                SlotKind::CenterPoint => {
                    // The middle and one center: the other center is opposite.
                    let other = Snap::free(c1.at * 2.0 - c2.at);
                    let (curves, axis) = straight_slot(sk, &other, c2, w.at)?;
                    let mid = place(sk, c1);
                    let _ = sk.add_constraint(Constraint::Midpoint { point: mid, line: axis });
                    made.extend(curves);
                }
                _ => {
                    let (curves, _) = straight_slot(sk, c1, c2, w.at)?;
                    made.extend(curves);
                }
            }
            "Sketch: Slot"
        }
        (SketchTool::Slot, [a, b, m, w]) if opts.slot.is_arc() => {
            // The arc of the path, counterclockwise.
            let (o, s0, e0) = if opts.slot == SlotKind::Arc3 {
                let o = circle_through(a.at, b.at, m.at)?;
                let ccw = (b.at - a.at).cross(m.at - a.at) < 0.0;
                if ccw { (o, a.at, b.at) } else { (o, b.at, a.at) }
            } else {
                (a.at, b.at, m.at)
            };
            let big_r = o.distance(s0);
            let a0 = math::atan2(s0.y - o.y, s0.x - o.x);
            let mut a1 = math::atan2(e0.y - o.y, e0.x - o.x);
            while a1 <= a0 + 1e-9 {
                a1 += std::f64::consts::TAU;
            }
            let r = (w.at.distance(o) - big_r).abs();
            made.extend(arc_slot(sk, o, big_r, a0, a1, r)?);
            "Sketch: Slot"
        }
        (SketchTool::RectCenter3, [c, a, b]) => {
            let dir = (a.at - c.at).normalized()?;
            let half_w = c.at.distance(a.at);
            let n = dir.perp();
            let half_h = (b.at - c.at).dot(n).abs();
            if half_h < 1e-6 {
                return None;
            }
            let corners = [
                c.at + dir * half_w + n * half_h,
                c.at - dir * half_w + n * half_h,
                c.at - dir * half_w - n * half_h,
                c.at + dir * half_w - n * half_h,
            ];
            let ids = corners.map(|q| sk.add_point(q));
            let lines = rectangle_sides(sk, ids);
            if let [l0, l1, l2, l3] = lines.as_slice() {
                let (_, hints) = infer::line_hints(sk, &Snap::free(corners[0]), &Snap::free(corners[1]));
                infer::apply_hints(sk, *l0, &hints);
                for k in [
                    Constraint::Perpendicular { a: *l0, b: *l1 },
                    Constraint::Parallel { a: *l0, b: *l2 },
                    Constraint::Parallel { a: *l1, b: *l3 },
                ] {
                    let _ = sk.add_constraint(k);
                }
            }
            // The center on a diagonal (construction lines).
            let center = place(sk, c);
            if let Ok(d) = sk.add_line_between(ids[0], ids[2]) {
                sk.set_construction(d, true);
                let _ = sk.add_constraint(Constraint::Midpoint { point: center, line: d });
            }
            if let Ok(d) = sk.add_line_between(ids[1], ids[3]) {
                sk.set_construction(d, true);
            }
            made.extend(lines);
            "Sketch: Three-Point Center Rectangle"
        }
        (SketchTool::TangentCircle, [a, b, c]) => {
            let lines = [a.curve?, b.curve?, c.curve?];
            if lines.iter().any(|l| !is_line(sk, *l)) {
                return None;
            }
            let near = Point::new((a.at.x + b.at.x + c.at.x) / 3.0, (a.at.y + b.at.y + c.at.y) / 3.0);
            made.push(sk.add_tangent_circle(lines, near).ok()?);
            "Sketch: Tangent Circle"
        }
        (SketchTool::Circle, [c, e]) => {
            let r = c.at.distance(e.at);
            if r < 1e-6 {
                return None;
            }
            let center = place(sk, c);
            made.push(sk.add_circle_on(center, r).ok()?);
            "Sketch: Circle"
        }
        (SketchTool::Arc3, [a, b, m]) => {
            let c = circle_through(a.at, b.at, m.at)?;
            // Counterclockwise from start to end through the middle point.
            let ccw = (b.at - a.at).cross(m.at - a.at) < 0.0;
            let (s0, e0) = if ccw { (a, b) } else { (b, a) };
            let r = c.distance(s0.at);
            let ps = place(sk, s0);
            let pe = place(sk, e0);
            let center = sk.add_point(c);
            made.push(sk.add_arc_between(center, ps, pe, r).ok()?);
            "Sketch: Arc"
        }
        (SketchTool::ArcCenter, [c, a, b]) => {
            let r = c.at.distance(a.at);
            if r < 1e-6 {
                return None;
            }
            let center = place(sk, c);
            let ps = place(sk, a);
            // A free end goes on the circle.
            let end = if b.point.is_some() {
                *b
            } else {
                let dir = (b.at - c.at).normalized()?;
                Snap {
                    at: c.at + dir * r,
                    ..*b
                }
            };
            let pe = place(sk, &end);
            made.push(sk.add_arc_between(center, ps, pe, r).ok()?);
            "Sketch: Arc"
        }
        (SketchTool::ArcTangent, [a, b]) => {
            let from = a.point?;
            // The line that ends at the first point: the arc goes on along it.
            let line = sk.curves.iter().find(|c| match c.geometry {
                Geometry::Line { a, b } => a == from || b == from,
                _ => false,
            })?;
            let (la, lb) = match line.geometry {
                Geometry::Line { a, b } => (a, b),
                _ => return None,
            };
            let line_id = line.id;
            let other = if la == from { lb } else { la };
            let dir = (a.at - sk.point(other)?).normalized()?;
            let normal = dir.perp();
            // The center is on the normal at a: |c − a| = |c − b|.
            let ab = b.at - a.at;
            let t = ab.length_sq() / (2.0 * ab.dot(normal));
            if !t.is_finite() || t.abs() > emb_sketch::MAX_MM {
                return None;
            }
            let c = a.at + normal * t;
            let pb = place(sk, b);
            let (s0, e0) = if t > 0.0 { (from, pb) } else { (pb, from) };
            let center = sk.add_point(c);
            let id = sk.add_arc_between(center, s0, e0, t.abs()).ok()?;
            let _ = sk.add_constraint(Constraint::Tangent {
                a: line_id,
                b: id,
                inside: false,
            });
            made.push(id);
            "Sketch: Tangent Arc"
        }
        (SketchTool::Polygon, [c, v]) => {
            let n = sides.clamp(3, 40) as usize;
            let (cp, vp) = (c.at, v.at);
            let r = cp.distance(vp);
            if r < 1e-6 {
                return None;
            }
            // Round the circle: the click is the middle of a side.
            let half = std::f64::consts::PI / n as f64;
            let (corner_r, a0) = if opts.outside {
                (r / math::cos(half), math::atan2(vp.y - cp.y, vp.x - cp.x) + half)
            } else {
                (r, math::atan2(vp.y - cp.y, vp.x - cp.x))
            };
            let ids: Vec<PointId> = (0..n)
                .map(|i| {
                    let a = a0 + std::f64::consts::TAU * i as f64 / n as f64;
                    sk.add_point(Point::new(
                        cp.x + corner_r * math::cos(a),
                        cp.y + corner_r * math::sin(a),
                    ))
                })
                .collect();
            let lines: Vec<CurveId> = (0..n)
                .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % n]).ok())
                .collect();
            // The corners on a construction circle, the sides equal.
            let center = place(sk, c);
            if let Ok(ring) = sk.add_circle_on(center, r) {
                sk.set_construction(ring, true);
                if opts.outside {
                    for l in &lines {
                        let _ = sk.add_constraint(Constraint::Tangent {
                            a: *l,
                            b: ring,
                            inside: false,
                        });
                    }
                } else {
                    for p in &ids {
                        let _ = sk.add_constraint(Constraint::OnCurve { point: *p, curve: ring });
                    }
                }
            }
            for w in lines.windows(2) {
                let _ = sk.add_constraint(Constraint::Equal { a: w[0], b: w[1] });
            }
            made.extend(lines);
            "Sketch: Polygon"
        }
        _ => return None,
    };
    for c in made {
        sk.set_construction(c, construction);
    }
    Some(label)
}

/// The number of clicks that a draw tool needs.
pub(super) fn clicks_needed(tool: SketchTool, opts: &MakeOpts) -> usize {
    match tool {
        SketchTool::Point => 1,
        SketchTool::Line
        | SketchTool::Rectangle
        | SketchTool::RectCenter
        | SketchTool::Circle
        | SketchTool::Polygon
        | SketchTool::ArcTangent => 2,
        SketchTool::Slot if opts.slot.is_arc() => 4,
        SketchTool::Arc3
        | SketchTool::ArcCenter
        | SketchTool::Rect3
        | SketchTool::Slot
        | SketchTool::RectCenter3
        | SketchTool::TangentCircle => 3,
        _ => usize::MAX,
    }
}
