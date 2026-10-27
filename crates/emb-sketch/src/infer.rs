//! Drawing help (P14 Milestone 3, ADR 0070 and 0072): the snaps of a click
//! and the constraints that the program adds while the user draws.
//!
//! A click snaps, in this order, to a point of the sketch (coincident), to
//! the middle of a line (midpoint), to the place where two curves cross
//! (on both curves), or to a curve (on curve). A free click takes the x or
//! the y of a point that it is in line with (no constraint: the place
//! only). A new line that is almost horizontal or vertical becomes so; one
//! that goes on from the end of an arc, or touches a circle or an arc in
//! its direction, becomes tangent; one that is almost at 90° to the line
//! before it in the chain, or parallel or at 90° to the line that the
//! pointer went over last, becomes so.

use emb_geom::{Point, math};

use crate::shape::{Shape, crossings};
use crate::{Constraint, CurveId, Geometry, PointId, Sketch};

/// Where a click goes: a point of the sketch, a curve, or free.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snap {
    pub at: Point,
    /// The point of the sketch at the click.
    pub point: Option<PointId>,
    /// The curve that the click is on (when it is not at a point).
    pub curve: Option<CurveId>,
    /// A second curve at the click: the place where two curves cross.
    pub also: Option<CurveId>,
    /// The click is at the middle of line `curve`.
    pub midpoint: bool,
    /// The points whose x and whose y the click took (the place only).
    pub align: [Option<PointId>; 2],
}

impl Snap {
    /// A click with no snap.
    pub fn free(at: Point) -> Snap {
        Snap {
            at,
            point: None,
            curve: None,
            also: None,
            midpoint: false,
            align: [None, None],
        }
    }

    /// A click on point `p` at `at`.
    pub fn on_point(at: Point, p: PointId) -> Snap {
        Snap {
            point: Some(p),
            ..Snap::free(at)
        }
    }

    /// A click on curve `c` at `at`.
    pub fn on_curve(at: Point, c: CurveId) -> Snap {
        Snap {
            curve: Some(c),
            ..Snap::free(at)
        }
    }

    /// True if the click is on a point or a curve (it does not move).
    pub fn is_held(&self) -> bool {
        self.point.is_some() || self.curve.is_some()
    }

    /// The marks of the snap, for the canvas.
    pub fn marks(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.midpoint {
            out.push("M");
        } else if self.also.is_some() {
            out.push("X");
        } else if self.curve.is_some() {
            out.push("O");
        }
        out
    }
}

/// A constraint that the program adds to a new line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hint {
    Horizontal,
    Vertical,
    /// Tangent to this circle or arc at the start or the end of the line.
    Tangent(CurveId),
    Parallel(CurveId),
    Perpendicular(CurveId),
}

impl Hint {
    /// The short mark of the hint (as the marks of constraints).
    pub fn mark(self) -> &'static str {
        match self {
            Hint::Horizontal => "H",
            Hint::Vertical => "V",
            Hint::Tangent(_) => "T",
            Hint::Parallel(_) => "//",
            Hint::Perpendicular(_) => "⊥",
        }
    }
}

/// The angle (degrees) within which a line becomes horizontal, vertical,
/// parallel, perpendicular or tangent.
pub const ANGLE_TOL_DEG: f64 = 4.0;

/// The snap of a click at `at` within `tol` mm (see the module text).
/// `skip` curves are not used.
pub fn snap(s: &Sketch, at: Point, tol: f64, skip: &[CurveId]) -> Snap {
    let nearest_point = s
        .points
        .iter()
        .map(|p| (p.id, p.pos, p.pos.distance(at)))
        .filter(|(_, _, d)| *d <= tol)
        .min_by(|a, b| a.2.total_cmp(&b.2));
    if let Some((id, pos, _)) = nearest_point {
        return Snap::on_point(pos, id);
    }
    let lines: Vec<(CurveId, Shape)> = s
        .curves
        .iter()
        .filter(|c| !skip.contains(&c.id))
        .filter_map(|c| Some((c.id, s.shape(c.id)?)))
        .collect();
    // The middle of a line.
    let mid = lines
        .iter()
        .filter_map(|(id, sh)| match sh {
            Shape::Line { a, b } => Some((*id, a.midpoint(*b))),
            _ => None,
        })
        .map(|(id, m)| (id, m, m.distance(at)))
        .filter(|(_, _, d)| *d <= tol)
        .min_by(|a, b| a.2.total_cmp(&b.2));
    if let Some((id, m, _)) = mid {
        return Snap {
            midpoint: true,
            ..Snap::on_curve(m, id)
        };
    }
    // Where two curves near the click cross.
    let near: Vec<&(CurveId, Shape)> = lines
        .iter()
        .filter(|(_, sh)| sh.nearest(at).0.distance(at) <= tol)
        .collect();
    let mut best: Option<(f64, Point, CurveId, CurveId)> = None;
    for (i, (a, sa)) in near.iter().enumerate() {
        for (b, sb) in near.iter().skip(i + 1) {
            for x in crossings(sa, sb) {
                let d = x.distance(at);
                if d <= tol && best.is_none_or(|b| d < b.0) {
                    best = Some((d, x, *a, *b));
                }
            }
        }
    }
    if let Some((_, x, a, b)) = best {
        return Snap {
            also: Some(b),
            ..Snap::on_curve(x, a)
        };
    }
    if let Some((id, q)) = s.nearest_curve(at, tol, skip) {
        return Snap::on_curve(q, id);
    }
    align(s, Snap::free(at), tol)
}

/// A free click takes the x and the y of the points that it is in line
/// with (within `tol`).
pub fn align(s: &Sketch, mut snap: Snap, tol: f64) -> Snap {
    if snap.is_held() {
        return snap;
    }
    let at = snap.at;
    let best = |axis: usize| {
        s.points
            .iter()
            .map(|p| {
                let d = if axis == 0 {
                    (p.pos.x - at.x).abs()
                } else {
                    (p.pos.y - at.y).abs()
                };
                (p.id, p.pos, d)
            })
            .filter(|(_, _, d)| *d <= tol)
            .min_by(|a, b| a.2.total_cmp(&b.2).then(a.1.distance(at).total_cmp(&b.1.distance(at))))
    };
    if let Some((id, pos, _)) = best(0) {
        snap.at.x = pos.x;
        snap.align[0] = Some(id);
    }
    if let Some((id, pos, _)) = best(1) {
        snap.at.y = pos.y;
        snap.align[1] = Some(id);
    }
    snap
}

/// The point of a click: the snapped point, else a new point with the
/// constraint of its snap (midpoint, on one or two curves).
pub fn place(s: &mut Sketch, snap: &Snap) -> PointId {
    if let Some(p) = snap.point.filter(|p| s.point(*p).is_some()) {
        return p;
    }
    let p = s.add_point(snap.at);
    if let Some(c) = snap.curve.filter(|c| s.curve(*c).is_some()) {
        let k = if snap.midpoint {
            Constraint::Midpoint { point: p, line: c }
        } else {
            Constraint::OnCurve { point: p, curve: c }
        };
        let _ = s.add_constraint(k);
    }
    if let Some(c) = snap.also.filter(|c| s.curve(*c).is_some()) {
        let _ = s.add_constraint(Constraint::OnCurve { point: p, curve: c });
    }
    p
}

/// The arcs that end at point `p`, with the direction of each arc there
/// (away from the arc).
fn arc_ends_at(s: &Sketch, p: PointId) -> Vec<(CurveId, Point)> {
    s.curves
        .iter()
        .filter_map(|c| {
            let Geometry::Arc { start, end, .. } = c.geometry else {
                return None;
            };
            let shape = s.shape(c.id)?;
            if start == p {
                Some((c.id, shape.tangent(0.0) * -1.0))
            } else if end == p {
                Some((c.id, shape.tangent(shape.end_param())))
            } else {
                None
            }
        })
        .collect()
}

/// The angle between two directions in degrees, in [0, 180].
fn angle_deg(a: Point, b: Point) -> f64 {
    math::to_degrees(math::atan2(a.cross(b).abs(), a.dot(b)))
}

/// The angle between two lines (directions in either sense), 0 to 90°.
fn line_angle_deg(a: Point, b: Point) -> f64 {
    let x = angle_deg(a, b);
    x.min(180.0 - x)
}

/// The direction of line `id`.
fn line_dir(s: &Sketch, id: CurveId) -> Option<Point> {
    match s.shape(id)? {
        Shape::Line { a, b } => (b - a).normalized(),
        _ => None,
    }
}

/// The hints for a line from `from` to `to` (see [`line_hints_with`]).
pub fn line_hints(s: &Sketch, from: &Snap, to: &Snap) -> (Point, Vec<Hint>) {
    line_hints_with(s, from, to, None)
}

/// The hints for a line from `from` to `to`, and the end where the line
/// goes with them (a horizontal line keeps the y of its start). The end of
/// a line that snapped to a point or a curve does not move. `reference` is
/// the line that the pointer went over last (for parallel and
/// perpendicular).
pub fn line_hints_with(s: &Sketch, from: &Snap, to: &Snap, reference: Option<CurveId>) -> (Point, Vec<Hint>) {
    let d = to.at - from.at;
    let mut end = to.at;
    let mut hints = Vec::new();
    if d.length() < 1e-9 {
        return (end, hints);
    }
    let free_end = !to.is_held();
    // Tangent to an arc at either end, in the direction of the arc.
    for (snap, dir) in [(from, d), (to, d * -1.0)] {
        if let Some(p) = snap.point {
            for (arc, away) in arc_ends_at(s, p) {
                if angle_deg(away, dir) <= ANGLE_TOL_DEG {
                    hints.push(Hint::Tangent(arc));
                }
            }
        }
        // Tangent to a circle or an arc that the end touches.
        if let (Some(c), false) = (snap.curve, snap.midpoint)
            && let Some(sh @ (Shape::Circle { .. } | Shape::Arc { .. })) = s.shape(c)
        {
            let t = sh.tangent(sh.param(snap.at));
            if line_angle_deg(t, d) <= ANGLE_TOL_DEG {
                hints.push(Hint::Tangent(c));
            }
        }
    }
    if !hints.is_empty() {
        return (end, hints);
    }
    // The direction that a hint gives: the end goes on it.
    let mut aim = |hint: Hint, dir: Point, hints: &mut Vec<Hint>| {
        hints.push(hint);
        if free_end {
            let along = d.dot(dir);
            end = from.at + dir * along;
        }
    };
    let h = line_angle_deg(d, Point::new(1.0, 0.0));
    let v = line_angle_deg(d, Point::new(0.0, 1.0));
    if h <= ANGLE_TOL_DEG {
        aim(Hint::Horizontal, Point::new(1.0, 0.0), &mut hints);
        // The end can still take the x of a point in line with it.
        if free_end && let Some(p) = to.align[0].and_then(|p| s.point(p)) {
            end.x = p.x;
        }
        return (end, hints);
    }
    if v <= ANGLE_TOL_DEG {
        aim(Hint::Vertical, Point::new(0.0, 1.0), &mut hints);
        if free_end && let Some(p) = to.align[1].and_then(|p| s.point(p)) {
            end.y = p.y;
        }
        return (end, hints);
    }
    // At 90° to the line before it in the chain.
    let before = from.point.and_then(|p| {
        s.curves
            .iter()
            .find(|c| c.is_line() && c.points().contains(&p))
            .map(|c| c.id)
    });
    if let Some(l) = before
        && let Some(ld) = line_dir(s, l)
        && line_angle_deg(ld, d) >= 90.0 - ANGLE_TOL_DEG
    {
        aim(Hint::Perpendicular(l), ld.perp(), &mut hints);
        return (end, hints);
    }
    // Parallel or at 90° to the line that the pointer went over last.
    if let Some(l) = reference.filter(|l| Some(*l) != before)
        && let Some(ld) = line_dir(s, l)
    {
        let x = line_angle_deg(ld, d);
        if x <= ANGLE_TOL_DEG {
            aim(Hint::Parallel(l), ld, &mut hints);
        } else if x >= 90.0 - ANGLE_TOL_DEG {
            aim(Hint::Perpendicular(l), ld.perp(), &mut hints);
        }
    }
    (end, hints)
}

/// Adds the constraints of `hints` to `line`.
pub fn apply_hints(s: &mut Sketch, line: CurveId, hints: &[Hint]) {
    for h in hints {
        let c = match *h {
            Hint::Horizontal => Constraint::Horizontal { line },
            Hint::Vertical => Constraint::Vertical { line },
            Hint::Tangent(arc) => Constraint::Tangent {
                a: line,
                b: arc,
                inside: false,
            },
            Hint::Parallel(l) => Constraint::Parallel { a: line, b: l },
            Hint::Perpendicular(l) => Constraint::Perpendicular { a: line, b: l },
        };
        let _ = s.add_constraint(c);
    }
}
