//! Sketch edits (P14 Milestone 3, ADR 0070): trim, extend, split, fillet,
//! chamfer, offset, mirror, rectangular and circular patterns, and relax.
//!
//! Each edit adds the constraints that keep its result true when the
//! sketch changes: a new end of a trimmed or extended curve stays on the
//! curve that cut it, the arc of a fillet stays tangent to its lines, a
//! mirrored point stays symmetric, an offset line stays parallel at its
//! distance. The corner of a fillet or a chamfer stays as a point on both
//! lines, so the dimensions to the corner stay. The methods are common CAD
//! practice in our own code.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

use emb_geom::{Point, math};

use crate::shape::{EPS, Shape, carrier_crossings, crossings};
use crate::{Constraint, ConstraintId, CurveId, Geometry, MAX_MM, PointId, Sketch};

/// Why an edit is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("the sketch has no such point or curve")]
    Missing,
    #[error("{0}")]
    Refused(&'static str),
}

fn no<T>(why: &'static str) -> Result<T, EditError> {
    Err(EditError::Refused(why))
}

/// The messages of refused edits (for the translations).
pub const MESSAGES: [&str; 32] = [
    "A circle has no ends to extend.",
    "A fillet or a chamfer needs a corner of two lines.",
    "A line has no length.",
    "A line of the corner has no length.",
    "A pattern needs 2 to 400 places.",
    "A tangent circle needs three different lines.",
    "A tangent circle needs three lines.",
    "Click inside the curve, not at an end.",
    "Mirror needs a line to mirror about.",
    "No circle touches these three lines.",
    "Select the curves to copy.",
    "Select the curves to mirror, then click the line.",
    "Select the curves to offset.",
    "Select the parts to move.",
    "Split needs a line or an arc (trim a circle).",
    "The angle must be more than 0° and at most 360°.",
    "The angle must be more than 0° and less than 180°.",
    "The chamfer does not meet the second line.",
    "The distance is too large for these lines.",
    "The distance must be more than 0.",
    "The extension of this curve meets no other curve.",
    "The line to mirror about has no length.",
    "The lines of the corner are in one line.",
    "The offset is larger than the radius of an arc.",
    "The offset is larger than the radius.",
    "The offset must be a distance more than 0.",
    "The radius is too large for these lines.",
    "The radius must be more than 0.",
    "The scale must be more than 0.",
    "The spacing must be a distance.",
    "This edit does not take splines yet.",
    "This end joins another curve.",
];

/// The kind of a chamfer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChamferKind {
    /// The same distance on both lines.
    Equal(f64),
    /// A distance on the first line and one on the second.
    Two(f64, f64),
    /// A distance on the first line and the angle (degrees) between the
    /// first line and the chamfer.
    Angle(f64, f64),
}

/// The largest number of copies of a pattern.
pub const MAX_COPIES: usize = 400;

/// A cut of a curve: its parameter, the place, and the curve that cuts.
#[derive(Debug, Clone, Copy)]
struct Cut {
    t: f64,
    at: Point,
    by: CurveId,
}

impl Sketch {
    /// The points that are one with `p` through coincident constraints
    /// (with `p`).
    pub fn joined(&self, p: PointId) -> BTreeSet<PointId> {
        let mut out = BTreeSet::from([p]);
        let mut todo = vec![p];
        while let Some(q) = todo.pop() {
            for c in &self.constraints {
                if let Constraint::Coincident { a, b } = c.constraint {
                    for (x, y) in [(a, b), (b, a)] {
                        if x == q && out.insert(y) {
                            todo.push(y);
                        }
                    }
                }
            }
        }
        out
    }

    fn set_geometry(&mut self, id: CurveId, g: Geometry) {
        if let Some(c) = self.curves.iter_mut().find(|c| c.id == id) {
            c.geometry = g;
        }
    }

    fn is_used(&self, p: PointId) -> bool {
        self.curves.iter().any(|c| c.points().contains(&p))
    }

    /// Removes point `p` if no curve uses it.
    fn release(&mut self, p: PointId) {
        if !self.is_used(p) {
            self.remove_point(p);
        }
    }

    /// Removes the constraints that a shorter or longer line makes false:
    /// its length, its midpoint and an equal length.
    fn drop_length(&mut self, line: CurveId) {
        self.constraints.retain(|c| match c.constraint {
            Constraint::Length { line: l, .. } | Constraint::Midpoint { line: l, .. } => l != line,
            Constraint::Equal { a, b } => a != line && b != line,
            _ => true,
        });
    }

    /// Removes the constraints that hold point `p` in its place (fix and
    /// distances), because an edit moves it.
    fn drop_place(&mut self, p: PointId) {
        self.constraints.retain(|c| match c.constraint {
            Constraint::Fix { point, .. } => point != p,
            Constraint::Distance { a, b, .. }
            | Constraint::HorizontalDistance { a, b, .. }
            | Constraint::VerticalDistance { a, b, .. } => a != p && b != p,
            _ => true,
        });
    }

    /// Puts the constraints of point `from` on point `into`.
    fn merge_point(&mut self, from: PointId, into: PointId) {
        let map = |q: PointId| if q == from { into } else { q };
        for c in &mut self.constraints {
            c.constraint = c.constraint.map_ids(map, |c| c);
        }
        self.constraints
            .retain(|c| !matches!(c.constraint, Constraint::Coincident { a, b } if a == b));
    }

    fn on_curve(&mut self, point: PointId, curve: CurveId) {
        let _ = self.add_constraint(Constraint::OnCurve { point, curve });
    }

    /// The places where the other curves cut curve `id`, not at its ends,
    /// in the order of the curve.
    fn cuts(&self, id: CurveId, shape: &Shape) -> Vec<Cut> {
        let len = match *shape {
            Shape::Line { a, b } => a.distance(b),
            Shape::Circle { r, .. } | Shape::Arc { r, .. } => r,
            Shape::Cubic { .. } => shape.length(),
        }
        .max(EPS);
        let tol = 1e-6 / len;
        let end = shape.end_param();
        let closed = matches!(shape, Shape::Circle { .. });
        let mut out: Vec<Cut> = Vec::new();
        for c in &self.curves {
            if c.id == id {
                continue;
            }
            let Some(other) = self.shape(c.id) else { continue };
            for at in crossings(shape, &other) {
                let t = shape.param(at);
                if !closed && (t <= tol || t >= end - tol) {
                    continue;
                }
                if out.iter().any(|x| (x.t - t).abs() <= tol) {
                    continue;
                }
                out.push(Cut { t, at, by: c.id });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /// Trim: removes the piece of curve `id` at `at` between the places
    /// where other curves cut it. A curve that nothing cuts goes away. The
    /// new ends stay on the curves that cut them.
    pub fn trim(&mut self, id: CurveId, at: Point) -> Result<(), EditError> {
        let shape = self.shape(id).ok_or(EditError::Missing)?;
        let curve = *self.curve(id).ok_or(EditError::Missing)?;
        let t = shape.nearest(at).1;
        let cuts = self.cuts(id, &shape);
        let lo = cuts.iter().rfind(|c| c.t < t).copied();
        let hi = cuts.iter().find(|c| c.t > t).copied();
        match curve.geometry {
            Geometry::Bezier { .. } => return no("This edit does not take splines yet."),
            Geometry::Line { a, b } => match (lo, hi) {
                (None, None) => {
                    self.remove_curve(id);
                }
                (None, Some(h)) => {
                    let q = self.add_point(h.at);
                    self.set_geometry(id, Geometry::Line { a: q, b });
                    self.on_curve(q, h.by);
                    self.release(a);
                    self.drop_length(id);
                }
                (Some(l), None) => {
                    let q = self.add_point(l.at);
                    self.set_geometry(id, Geometry::Line { a, b: q });
                    self.on_curve(q, l.by);
                    self.release(b);
                    self.drop_length(id);
                }
                (Some(l), Some(h)) => {
                    let (q1, q2) = (self.add_point(l.at), self.add_point(h.at));
                    self.set_geometry(id, Geometry::Line { a, b: q1 });
                    let new = self.add_curve(Geometry::Line { a: q2, b });
                    self.set_construction(new, curve.construction);
                    self.drop_length(id);
                    // The second piece stays in line with the first.
                    self.on_curve(q2, id);
                    self.on_curve(b, id);
                    self.on_curve(q1, l.by);
                    self.on_curve(q2, h.by);
                }
            },
            Geometry::Arc {
                center,
                start,
                end,
                radius,
            } => match (lo, hi) {
                (None, None) => {
                    self.remove_curve(id);
                }
                (None, Some(h)) => {
                    let q = self.add_point(h.at);
                    self.set_geometry(
                        id,
                        Geometry::Arc {
                            center,
                            start: q,
                            end,
                            radius,
                        },
                    );
                    self.on_curve(q, h.by);
                    self.release(start);
                }
                (Some(l), None) => {
                    let q = self.add_point(l.at);
                    self.set_geometry(
                        id,
                        Geometry::Arc {
                            center,
                            start,
                            end: q,
                            radius,
                        },
                    );
                    self.on_curve(q, l.by);
                    self.release(end);
                }
                (Some(l), Some(h)) => {
                    let (q1, q2) = (self.add_point(l.at), self.add_point(h.at));
                    self.set_geometry(
                        id,
                        Geometry::Arc {
                            center,
                            start,
                            end: q1,
                            radius,
                        },
                    );
                    let new = self.add_curve(Geometry::Arc {
                        center,
                        start: q2,
                        end,
                        radius,
                    });
                    self.set_construction(new, curve.construction);
                    let _ = self.add_constraint(Constraint::Equal { a: id, b: new });
                    self.on_curve(q1, l.by);
                    self.on_curve(q2, h.by);
                }
            },
            Geometry::Circle { center, radius } => {
                if cuts.len() < 2 {
                    self.remove_curve(id);
                    return Ok(());
                }
                // Round the circle: the cuts before and after the click.
                let lo = lo.unwrap_or(*cuts.last().expect("two cuts"));
                let hi = hi.unwrap_or(cuts[0]);
                let (q1, q2) = (self.add_point(hi.at), self.add_point(lo.at));
                self.set_geometry(
                    id,
                    Geometry::Arc {
                        center,
                        start: q1,
                        end: q2,
                        radius,
                    },
                );
                self.on_curve(q1, hi.by);
                self.on_curve(q2, lo.by);
            }
        }
        Ok(())
    }

    /// Extend: moves the end of curve `id` nearest to `at` to the nearest
    /// curve that its extension meets. The end stays on that curve.
    pub fn extend(&mut self, id: CurveId, at: Point) -> Result<(), EditError> {
        let shape = self.shape(id).ok_or(EditError::Missing)?;
        let curve = *self.curve(id).ok_or(EditError::Missing)?;
        let (end_point, from_end) = match curve.geometry {
            Geometry::Bezier { .. } => return no("This edit does not take splines yet."),
            Geometry::Line { a, b } => {
                let (pa, pb) = (
                    self.point(a).ok_or(EditError::Missing)?,
                    self.point(b).ok_or(EditError::Missing)?,
                );
                if at.distance(pb) <= at.distance(pa) {
                    (b, true)
                } else {
                    (a, false)
                }
            }
            Geometry::Arc { start, end, .. } => {
                let (ps, pe) = (
                    self.point(start).ok_or(EditError::Missing)?,
                    self.point(end).ok_or(EditError::Missing)?,
                );
                if at.distance(pe) <= at.distance(ps) {
                    (end, true)
                } else {
                    (start, false)
                }
            }
            Geometry::Circle { .. } => return no("A circle has no ends to extend."),
        };
        let joined = self.joined(end_point);
        let shared = self
            .curves
            .iter()
            .any(|c| c.id != id && c.points().iter().any(|p| joined.contains(p)));
        if shared {
            return no("This end joins another curve.");
        }
        let end = shape.end_param();
        let mut best: Option<(f64, Point, CurveId)> = None;
        for c in &self.curves {
            if c.id == id {
                continue;
            }
            let Some(other) = self.shape(c.id) else { continue };
            for x in carrier_crossings(&shape, &other) {
                if !other.has_param(other.param(x), 1e-6) {
                    continue;
                }
                let t = shape.param(x);
                // How far past the end, along the curve.
                let past = match (shape, from_end) {
                    (Shape::Line { .. }, true) => t - 1.0,
                    (Shape::Line { .. }, false) => -t,
                    (_, true) => t - end,
                    (_, false) => TAU - t,
                };
                let room = if matches!(shape, Shape::Line { .. }) {
                    f64::INFINITY
                } else {
                    TAU - end
                };
                if past > 1e-9 && past < room && best.is_none_or(|b| past < b.0) {
                    best = Some((past, x, c.id));
                }
            }
        }
        let Some((_, to, by)) = best else {
            return no("The extension of this curve meets no other curve.");
        };
        self.set_point(end_point, to);
        self.drop_place(end_point);
        if curve.is_line() {
            self.drop_length(id);
        }
        self.on_curve(end_point, by);
        Ok(())
    }

    /// Split: cuts curve `id` in two at the point nearest to `at`. The two
    /// pieces share the new point and stay one curve in shape (in line, or
    /// on one circle). Returns the new piece.
    pub fn split(&mut self, id: CurveId, at: Point) -> Result<CurveId, EditError> {
        let shape = self.shape(id).ok_or(EditError::Missing)?;
        let curve = *self.curve(id).ok_or(EditError::Missing)?;
        let (q, t) = shape.nearest(at);
        let len = match shape {
            Shape::Line { a, b } => a.distance(b),
            Shape::Circle { r, .. } | Shape::Arc { r, .. } => r,
            Shape::Cubic { .. } => shape.length(),
        };
        let tol = 1e-3 / len.max(EPS);
        if t <= tol || t >= shape.end_param() - tol {
            return no("Click inside the curve, not at an end.");
        }
        let new = match curve.geometry {
            Geometry::Bezier { .. } => return no("This edit does not take splines yet."),
            Geometry::Line { a, b } => {
                let m = self.add_point(q);
                self.set_geometry(id, Geometry::Line { a, b: m });
                let new = self.add_curve(Geometry::Line { a: m, b });
                self.drop_length(id);
                self.on_curve(b, id);
                new
            }
            Geometry::Arc {
                center,
                start,
                end,
                radius,
            } => {
                let m = self.add_point(q);
                self.set_geometry(
                    id,
                    Geometry::Arc {
                        center,
                        start,
                        end: m,
                        radius,
                    },
                );
                let new = self.add_curve(Geometry::Arc {
                    center,
                    start: m,
                    end,
                    radius,
                });
                let _ = self.add_constraint(Constraint::Equal { a: id, b: new });
                new
            }
            Geometry::Circle { .. } => return no("Split needs a line or an arc (trim a circle)."),
        };
        self.set_construction(new, curve.construction);
        Ok(new)
    }

    /// The two lines that meet at `corner` (directly or through coincident
    /// points): each line, its end at the corner and its other end.
    fn corner_lines(&self, corner: PointId) -> Result<[(CurveId, PointId, PointId); 2], EditError> {
        self.point(corner).ok_or(EditError::Missing)?;
        let joined = self.joined(corner);
        let mut lines = Vec::new();
        for c in &self.curves {
            match c.geometry {
                Geometry::Line { a, b } => {
                    if joined.contains(&a) {
                        lines.push((c.id, a, b));
                    } else if joined.contains(&b) {
                        lines.push((c.id, b, a));
                    }
                }
                Geometry::Arc { start, end, .. } if joined.contains(&start) || joined.contains(&end) => {
                    return no("A fillet or a chamfer needs a corner of two lines.");
                }
                _ => {}
            }
        }
        match lines.as_slice() {
            [x, y] => Ok([*x, *y]),
            _ => no("A fillet or a chamfer needs a corner of two lines."),
        }
    }

    /// Cuts the corner of two lines at `corner`: the new ends at distance
    /// `d1` and `d2` from the corner along each line. The corner stays as a
    /// point on both lines (a length of a line becomes a distance to the
    /// corner). Returns the two new ends.
    fn cut_corner(
        &mut self,
        corner: PointId,
        lines: [(CurveId, PointId, PointId); 2],
        d: [f64; 2],
    ) -> Result<[PointId; 2], EditError> {
        let p = self.point(corner).ok_or(EditError::Missing)?;
        let mut ends = [corner; 2];
        for (i, (line, at, other)) in lines.into_iter().enumerate() {
            let o = self.point(other).ok_or(EditError::Missing)?;
            let u = (o - p).normalized().ok_or(EditError::Missing)?;
            let q = self.add_point(p + u * d[i]);
            let g = match self.curve(line).map(|c| c.geometry) {
                Some(Geometry::Line { a, .. }) if a == at => Geometry::Line { a: q, b: other },
                _ => Geometry::Line { a: other, b: q },
            };
            self.set_geometry(line, g);
            // A length of the line becomes the distance to the corner.
            let mut lengths = Vec::new();
            self.constraints.retain(|c| match c.constraint {
                Constraint::Length { line: l, mm } if l == line => {
                    lengths.push(mm);
                    false
                }
                Constraint::Midpoint { line: l, .. } => l != line,
                _ => true,
            });
            for mm in lengths {
                let _ = self.add_constraint(Constraint::Distance {
                    a: other,
                    b: corner,
                    mm,
                });
            }
            ends[i] = q;
        }
        // The points at the corner that no curve uses now become one.
        for q in self.joined(corner) {
            if q != corner && !self.is_used(q) {
                self.merge_point(q, corner);
                self.remove_point(q);
            }
        }
        for (line, ..) in lines {
            self.on_curve(corner, line);
        }
        Ok(ends)
    }

    /// The directions from `corner` along the two lines, and the angle
    /// between them, for a fillet or a chamfer.
    fn corner_angle(
        &self,
        corner: PointId,
        lines: &[(CurveId, PointId, PointId); 2],
    ) -> Result<CornerAngle, EditError> {
        let p = self.point(corner).ok_or(EditError::Missing)?;
        let mut u = [Point::new(0.0, 0.0); 2];
        let mut len = [0.0; 2];
        for (i, (_, _, other)) in lines.iter().enumerate() {
            let o = self.point(*other).ok_or(EditError::Missing)?;
            len[i] = o.distance(p);
            u[i] = (o - p)
                .normalized()
                .ok_or(EditError::Refused("A line of the corner has no length."))?;
        }
        let theta = math::atan2(u[0].cross(u[1]).abs(), u[0].dot(u[1]));
        if !(1e-3..=PI - 1e-3).contains(&theta) {
            return no("The lines of the corner are in one line.");
        }
        Ok(CornerAngle { p, u, len, theta })
    }

    /// Fillet: rounds the corner of two lines at `corner` with an arc of
    /// `radius`, tangent to both lines. Returns the arc and its radius
    /// dimension.
    pub fn fillet(&mut self, corner: PointId, radius: f64) -> Result<(CurveId, ConstraintId), EditError> {
        if !(radius.is_finite() && radius > 0.0 && radius <= MAX_MM) {
            return no("The radius must be more than 0.");
        }
        let lines = self.corner_lines(corner)?;
        let k = self.corner_angle(corner, &lines)?;
        let d = radius / math::tan(k.theta / 2.0);
        if d >= k.len[0] - 1e-6 || d >= k.len[1] - 1e-6 {
            return no("The radius is too large for these lines.");
        }
        let bisector = (k.u[0] + k.u[1]).normalized().ok_or(EditError::Missing)?;
        let c = k.p + bisector * (radius / math::sin(k.theta / 2.0));
        let construction = lines
            .iter()
            .all(|(l, ..)| self.curve(*l).is_some_and(|c| c.construction));
        let [t1, t2] = self.cut_corner(corner, lines, [d, d])?;
        let center = self.add_point(c);
        let (p1, p2) = (
            self.point(t1).ok_or(EditError::Missing)?,
            self.point(t2).ok_or(EditError::Missing)?,
        );
        let (s, e) = if (p1 - c).cross(p2 - c) > 0.0 {
            (t1, t2)
        } else {
            (t2, t1)
        };
        let arc = self
            .add_arc_between(center, s, e, radius)
            .map_err(|_| EditError::Refused("The radius is too large for these lines."))?;
        self.set_construction(arc, construction);
        for (line, ..) in lines {
            let _ = self.add_constraint(Constraint::Tangent {
                a: line,
                b: arc,
                inside: false,
            });
        }
        let dim = self
            .add_constraint(Constraint::Radius { curve: arc, mm: radius })
            .map_err(|_| EditError::Missing)?;
        Ok((arc, dim))
    }

    /// Chamfer: cuts the corner of two lines at `corner` with a line, at
    /// distance `mm` from the corner along each line. Returns the new line
    /// and the dimension of the first distance.
    pub fn chamfer(&mut self, corner: PointId, mm: f64) -> Result<(CurveId, ConstraintId), EditError> {
        self.chamfer_with(corner, ChamferKind::Equal(mm))
    }

    /// Chamfer of a kind: equal distances, two distances (the first on the
    /// first line of the corner), or a distance and an angle.
    pub fn chamfer_with(&mut self, corner: PointId, kind: ChamferKind) -> Result<(CurveId, ConstraintId), EditError> {
        let ok = |mm: f64| mm.is_finite() && mm > 0.0 && mm <= MAX_MM;
        let lines = self.corner_lines(corner)?;
        let k = self.corner_angle(corner, &lines)?;
        let (d1, d2) = match kind {
            ChamferKind::Equal(d) => (d, d),
            ChamferKind::Two(a, b) => (a, b),
            ChamferKind::Angle(d, deg) => {
                if !(deg.is_finite() && deg > 0.5 && deg < 179.5) {
                    return no("The angle must be more than 0° and less than 180°.");
                }
                // From the end on the first line, at the angle to that line,
                // toward the second line.
                let t1 = k.p + k.u[0] * d;
                let back = k.u[0] * -1.0;
                let side = if k.u[0].cross(k.u[1]) > 0.0 { -1.0 } else { 1.0 };
                let (s, c) = math::sin_cos_deg(side * deg);
                let v = Point::new(back.x * c - back.y * s, back.x * s + back.y * c);
                // t1 + t·v = p + s2·u1.
                let den = v.cross(k.u[1]);
                if den.abs() < 1e-12 {
                    return no("The chamfer does not meet the second line.");
                }
                let s2 = (k.p - t1).cross(v) / den;
                (d, s2)
            }
        };
        if !(ok(d1) && ok(d2)) {
            return no("The distance must be more than 0.");
        }
        if d1 >= k.len[0] - 1e-6 || d2 >= k.len[1] - 1e-6 {
            return no("The distance is too large for these lines.");
        }
        let construction = lines
            .iter()
            .all(|(l, ..)| self.curve(*l).is_some_and(|c| c.construction));
        let first = lines[0].0;
        let [t1, t2] = self.cut_corner(corner, lines, [d1, d2])?;
        let line = self.add_curve(Geometry::Line { a: t1, b: t2 });
        self.set_construction(line, construction);
        let dim = self
            .add_constraint(Constraint::Distance {
                a: corner,
                b: t1,
                mm: d1,
            })
            .map_err(|_| EditError::Missing)?;
        let second = match kind {
            ChamferKind::Angle(..) => {
                let a = Constraint::Angle {
                    a: first,
                    b: line,
                    deg: 0.0,
                };
                a.with_value(self.measure(&a).unwrap_or(0.0))
            }
            _ => Constraint::Distance {
                a: corner,
                b: t2,
                mm: d2,
            },
        };
        let _ = self.add_constraint(second);
        Ok((line, dim))
    }

    /// A circle tangent to three lines (their extensions too): of the
    /// circles that touch all three, the one with its center nearest to
    /// `near`.
    pub fn add_tangent_circle(&mut self, lines: [CurveId; 3], near: Point) -> Result<CurveId, EditError> {
        let mut rows = Vec::new();
        for l in lines {
            let Some(Shape::Line { a, b }) = self.shape(l) else {
                return no("A tangent circle needs three lines.");
            };
            let n = (b - a)
                .normalized()
                .ok_or(EditError::Refused("A line has no length."))?
                .perp();
            rows.push((n, n.dot(a)));
        }
        if lines[0] == lines[1] || lines[1] == lines[2] || lines[0] == lines[2] {
            return no("A tangent circle needs three different lines.");
        }
        let mut best: Option<(f64, Point, f64)> = None;
        for sig in [[1.0, 1.0, 1.0], [1.0, 1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]] {
            // n·c − σ·r = d for each line: three equations in cx, cy, r.
            let m: Vec<[f64; 3]> = rows.iter().zip(sig).map(|((n, _), s)| [n.x, n.y, -s]).collect();
            let rhs: Vec<f64> = rows.iter().map(|(_, d)| *d).collect();
            let det3 = |m: &[[f64; 3]]| {
                m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                    + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
            };
            let det = det3(&m);
            if det.abs() < 1e-12 {
                continue;
            }
            let col = |k: usize| {
                let mut c = m.clone();
                for (i, row) in c.iter_mut().enumerate() {
                    row[k] = rhs[i];
                }
                det3(&c) / det
            };
            let (cx, cy, r) = (col(0), col(1), col(2));
            let (c, r) = (Point::new(cx, cy), r.abs());
            if !(r > 1e-6 && r <= MAX_MM && c.is_finite()) {
                continue;
            }
            let dist = c.distance(near);
            if best.is_none_or(|b| dist < b.0) {
                best = Some((dist, c, r));
            }
        }
        let Some((_, c, r)) = best else {
            return no("No circle touches these three lines.");
        };
        let circle = self.add_circle(c, r).map_err(|_| EditError::Missing)?;
        for l in lines {
            let _ = self.add_constraint(Constraint::Tangent {
                a: l,
                b: circle,
                inside: false,
            });
        }
        Ok(circle)
    }

    /// Adds `c` if it neither repeats nor conflicts with the constraints of
    /// the sketch. Returns true if it was added.
    fn add_if_new(&mut self, c: Constraint) -> bool {
        let mut test = self.clone();
        let Ok(id) = test.add_constraint(c) else {
            return false;
        };
        let a = test.analyze();
        if a.redundant.contains(&id) || a.conflicting.contains(&id) {
            return false;
        }
        self.add_constraint(c).is_ok()
    }

    /// Automatic dimensions and constraints of `curves` (all curves when it
    /// is empty): horizontal and vertical for lines that are almost so,
    /// then the lengths, the diameters of circles and the radii of arcs,
    /// then the angles between lines that meet, each with the size that the
    /// sketch has now, and only where the sketch can still move. Returns
    /// the number of constraints and of dimensions.
    pub fn auto_dimension(&mut self, curves: &[CurveId]) -> (usize, usize) {
        let list: Vec<CurveId> = if curves.is_empty() {
            self.curves.iter().map(|c| c.id).collect()
        } else {
            curves.iter().copied().filter(|c| self.curve(*c).is_some()).collect()
        };
        let (mut constraints, mut dimensions) = (0, 0);
        // Directions.
        for id in &list {
            let Some(Shape::Line { a, b }) = self.shape(*id) else {
                continue;
            };
            let held = self.constraints.iter().any(|c| {
                c.constraint.curves().contains(id)
                    && matches!(
                        c.constraint,
                        Constraint::Horizontal { .. }
                            | Constraint::Vertical { .. }
                            | Constraint::Parallel { .. }
                            | Constraint::Perpendicular { .. }
                            | Constraint::Collinear { .. }
                            | Constraint::Angle { .. }
                    )
            });
            if held {
                continue;
            }
            let d = b - a;
            let tol = math::tan(math::to_radians(0.5));
            let c = if d.y.abs() <= tol * d.x.abs() {
                Constraint::Horizontal { line: *id }
            } else if d.x.abs() <= tol * d.y.abs() {
                Constraint::Vertical { line: *id }
            } else {
                continue;
            };
            if self.add_if_new(c) {
                constraints += 1;
            }
        }
        // Sizes.
        for id in &list {
            let Some(curve) = self.curve(*id).copied() else {
                continue;
            };
            let c = match curve.geometry {
                Geometry::Line { .. } => Constraint::Length { line: *id, mm: 0.0 },
                Geometry::Circle { .. } => Constraint::Diameter { curve: *id, mm: 0.0 },
                Geometry::Arc { .. } => Constraint::Radius { curve: *id, mm: 0.0 },
                Geometry::Bezier { .. } => continue,
            };
            let Some(v) = self.measure(&c) else { continue };
            if v > 1e-9 && self.add_if_new(c.with_value(v)) {
                dimensions += 1;
            }
        }
        // Angles between lines that meet.
        let class = self.classes();
        let k = |p: PointId| class.get(&p).copied().unwrap_or(p);
        let lines: Vec<(CurveId, [PointId; 2])> = list
            .iter()
            .filter_map(|id| match self.curve(*id)?.geometry {
                Geometry::Line { a, b } => Some((*id, [k(a), k(b)])),
                _ => None,
            })
            .collect();
        for (i, (la, ea)) in lines.iter().enumerate() {
            for (lb, eb) in lines.iter().skip(i + 1) {
                if !ea.iter().any(|p| eb.contains(p)) {
                    continue;
                }
                let c = Constraint::Angle {
                    a: *la,
                    b: *lb,
                    deg: 0.0,
                };
                let Some(v) = self.measure(&c) else { continue };
                if self.add_if_new(c.with_value(v)) {
                    dimensions += 1;
                }
            }
        }
        (constraints, dimensions)
    }

    /// The point classes of the sketch: each point to the smallest point
    /// that is one with it (coincident).
    fn classes(&self) -> BTreeMap<PointId, PointId> {
        let mut out = BTreeMap::new();
        for p in &self.points {
            if out.contains_key(&p.id) {
                continue;
            }
            let j = self.joined(p.id);
            let first = *j.iter().next().expect("the point itself");
            for q in j {
                out.insert(q, first);
            }
        }
        out
    }

    /// The chains of `curves`: curves that follow on from each other at
    /// their ends, each with its direction (true: from its first point).
    fn chains(&self, curves: &[CurveId]) -> Vec<Chain> {
        let class = self.classes();
        let k = |p: PointId| class.get(&p).copied().unwrap_or(p);
        let mut ends: BTreeMap<CurveId, (PointId, PointId)> = BTreeMap::new();
        let mut at: BTreeMap<PointId, Vec<CurveId>> = BTreeMap::new();
        let mut out = Vec::new();
        for id in curves {
            match self.curve(*id).map(|c| c.geometry) {
                Some(Geometry::Line { a, b })
                | Some(Geometry::Arc { start: a, end: b, .. })
                | Some(Geometry::Bezier { start: a, end: b, .. }) => {
                    ends.insert(*id, (k(a), k(b)));
                    at.entry(k(a)).or_default().push(*id);
                    at.entry(k(b)).or_default().push(*id);
                }
                Some(Geometry::Circle { .. }) => out.push(Chain {
                    items: vec![(*id, true)],
                    closed: true,
                }),
                None => {}
            }
        }
        let deg = |p: PointId| at.get(&p).map_or(0, Vec::len);
        let mut done: BTreeSet<CurveId> = BTreeSet::new();
        let walk = |first: CurveId, forward: bool, done: &mut BTreeSet<CurveId>| {
            let mut items = Vec::new();
            let (mut cur, mut fwd) = (first, forward);
            let mut closed = false;
            loop {
                done.insert(cur);
                items.push((cur, fwd));
                let exit = if fwd { ends[&cur].1 } else { ends[&cur].0 };
                if deg(exit) != 2 {
                    break;
                }
                let Some(&next) = at[&exit].iter().find(|x| **x != cur) else {
                    break;
                };
                if next == first {
                    closed = true;
                    break;
                }
                if done.contains(&next) {
                    break;
                }
                fwd = ends[&next].0 == exit;
                cur = next;
            }
            Chain { items, closed }
        };
        let ids: Vec<CurveId> = ends.keys().copied().collect();
        for id in &ids {
            if done.contains(id) {
                continue;
            }
            let (s, e) = ends[id];
            if deg(s) != 2 {
                out.push(walk(*id, true, &mut done));
            } else if deg(e) != 2 {
                out.push(walk(*id, false, &mut done));
            }
        }
        for id in &ids {
            if !done.contains(id) {
                out.push(walk(*id, true, &mut done));
            }
        }
        out
    }

    /// The curves joined to curve `id` at their ends (with it), for example
    /// all the curves of a closed shape.
    pub fn connected_curves(&self, id: CurveId) -> Vec<CurveId> {
        let class = self.classes();
        let k = |p: PointId| class.get(&p).copied().unwrap_or(p);
        let ends = |c: &crate::Curve| match c.ends() {
            Some((a, b)) => vec![k(a), k(b)],
            None => Vec::new(),
        };
        let mut out = vec![id];
        let mut todo = vec![id];
        while let Some(c) = todo.pop() {
            let Some(curve) = self.curve(c) else { continue };
            let e = ends(curve);
            for other in &self.curves {
                if !out.contains(&other.id) && ends(other).iter().any(|p| e.contains(p)) {
                    out.push(other.id);
                    todo.push(other.id);
                }
            }
        }
        out
    }

    /// Offset: copies of `curves` at distance `mm` (positive: outside of a
    /// closed shape, else on the left of the chain). The copies join at
    /// their corners; an offset line stays parallel to its line at the
    /// distance, an offset arc keeps the center of its arc. Returns the new
    /// curves.
    pub fn offset(&mut self, curves: &[CurveId], mm: f64) -> Result<Vec<CurveId>, EditError> {
        if !(mm.is_finite() && mm.abs() > 1e-6 && mm.abs() <= MAX_MM) {
            return no("The offset must be a distance more than 0.");
        }
        let mut list: Vec<CurveId> = Vec::new();
        for c in curves {
            if self.curve(*c).is_some() && !list.contains(c) {
                list.push(*c);
            }
        }
        if list.is_empty() {
            return no("Select the curves to offset.");
        }
        let mut made = Vec::new();
        for chain in self.chains(&list) {
            made.extend(self.offset_chain(&chain, mm)?);
        }
        Ok(made)
    }

    fn offset_chain(&mut self, chain: &Chain, mm: f64) -> Result<Vec<CurveId>, EditError> {
        // One circle.
        if let [(id, _)] = chain.items.as_slice()
            && let Some(Geometry::Circle { center, radius }) = self.curve(*id).map(|c| c.geometry)
        {
            let r = radius + mm;
            if r <= 1e-6 {
                return no("The offset is larger than the radius.");
            }
            let construction = self.curve(*id).is_some_and(|c| c.construction);
            let new = self.add_circle_on(center, r).map_err(|_| EditError::Missing)?;
            self.set_construction(new, construction);
            let _ = self.add_constraint(Constraint::Radius { curve: new, mm: r });
            return Ok(vec![new]);
        }
        // The side: outside of a closed chain, the left of an open chain.
        let mut o = mm;
        if chain.closed {
            let mut pts = Vec::new();
            for (id, fwd) in &chain.items {
                let s = self.shape(*id).ok_or(EditError::Missing)?;
                let n = 8;
                for i in 0..n {
                    let f = i as f64 / n as f64;
                    let t = if *fwd { f } else { 1.0 - f } * s.end_param();
                    pts.push(s.at(t));
                }
            }
            let area: f64 = (0..pts.len()).map(|i| pts[i].cross(pts[(i + 1) % pts.len()])).sum();
            if area > 0.0 {
                o = -mm;
            }
        }
        // The offset of each curve: its carrier and its ends (in the order
        // of the chain).
        let mut offs: Vec<Off> = Vec::new();
        for (id, fwd) in &chain.items {
            let s = self.shape(*id).ok_or(EditError::Missing)?;
            let (p, q) = if *fwd {
                (s.at(0.0), s.at(s.end_param()))
            } else {
                (s.at(s.end_param()), s.at(0.0))
            };
            match (s, self.curve(*id).map(|c| c.geometry)) {
                (Shape::Line { .. }, _) => {
                    let n = (q - p).normalized().ok_or(EditError::Missing)?.perp();
                    let (a, b) = (p + n * o, q + n * o);
                    offs.push(Off {
                        carrier: Shape::Line { a, b },
                        ends: (a, b),
                        center: None,
                    });
                }
                (Shape::Arc { c, r, .. }, Some(Geometry::Arc { center, .. })) => {
                    let r2 = if *fwd { r - o } else { r + o };
                    if r2 <= 1e-6 {
                        return no("The offset is larger than the radius of an arc.");
                    }
                    let radial = |x: Point| c + (x - c).normalized().unwrap_or(Point::new(1.0, 0.0)) * r2;
                    offs.push(Off {
                        carrier: Shape::Circle { c, r: r2 },
                        ends: (radial(p), radial(q)),
                        center: Some((center, r2)),
                    });
                }
                _ => return Err(EditError::Missing),
            }
        }
        // The joints: where the carriers of curves that follow on cross.
        let n = offs.len();
        let joint = |i: usize, j: usize| -> Point {
            let near = offs[i].ends.1;
            let fallback = near.midpoint(offs[j].ends.0);
            carrier_crossings(&offs[i].carrier, &offs[j].carrier)
                .into_iter()
                .min_by(|a, b| a.distance(near).total_cmp(&b.distance(near)))
                .filter(|x| x.distance(near) <= 10.0 * mm.abs() + 1.0)
                .unwrap_or(fallback)
        };
        let mut joints: Vec<PointId> = Vec::new();
        if chain.closed {
            for i in 0..n {
                let x = joint(i, (i + 1) % n);
                joints.push(self.add_point(x));
            }
        } else {
            joints.push(self.add_point(offs[0].ends.0));
            for i in 0..n - 1 {
                let x = joint(i, i + 1);
                joints.push(self.add_point(x));
            }
            joints.push(self.add_point(offs[n - 1].ends.1));
        }
        // The ends of curve i: joints i−1 and i (closed), i and i+1 (open).
        let ends_of = |i: usize| -> (PointId, PointId) {
            if chain.closed {
                (joints[(i + n - 1) % n], joints[i])
            } else {
                (joints[i], joints[i + 1])
            }
        };
        let mut made = Vec::new();
        for (i, (id, fwd)) in chain.items.iter().enumerate() {
            let (pin, pout) = ends_of(i);
            let construction = self.curve(*id).is_some_and(|c| c.construction);
            let new = match offs[i].center {
                None => {
                    let new = self.add_curve(Geometry::Line { a: pin, b: pout });
                    let _ = self.add_constraint(Constraint::Parallel { a: new, b: *id });
                    let _ = self.add_constraint(Constraint::LineDistance {
                        point: pin,
                        line: *id,
                        mm: if *fwd { o } else { -o },
                    });
                    new
                }
                Some((center, r)) => {
                    let (s, e) = if *fwd { (pin, pout) } else { (pout, pin) };
                    self.add_arc_between(center, s, e, r).map_err(|_| EditError::Missing)?
                }
            };
            self.set_construction(new, construction);
            made.push(new);
        }
        // Tangent curves stay tangent; an arc with no tangent keeps its
        // radius as a dimension.
        let pairs: Vec<(usize, usize)> = if chain.closed {
            (0..n).map(|i| (i, (i + 1) % n)).collect()
        } else {
            (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect()
        };
        let mut tangent = vec![false; n];
        for (i, j) in pairs {
            let (a, b) = (chain.items[i].0, chain.items[j].0);
            let inside = self.constraints.iter().find_map(|c| match c.constraint {
                Constraint::Tangent { a: x, b: y, inside } if (x == a && y == b) || (x == b && y == a) => Some(inside),
                _ => None,
            });
            if let Some(inside) = inside
                && i != j
            {
                let _ = self.add_constraint(Constraint::Tangent {
                    a: made[i],
                    b: made[j],
                    inside,
                });
                tangent[i] = true;
                tangent[j] = true;
            }
        }
        for (i, off) in offs.iter().enumerate() {
            if let Some((_, r)) = off.center
                && !tangent[i]
            {
                let _ = self.add_constraint(Constraint::Radius { curve: made[i], mm: r });
            }
        }
        Ok(made)
    }

    /// Mirror: copies of `curves` mirrored about line `axis`. Each copied
    /// point is symmetric to its point about the line; a point on the line
    /// is shared. Returns the new curves.
    pub fn mirror(&mut self, curves: &[CurveId], axis: CurveId) -> Result<Vec<CurveId>, EditError> {
        let Some(Shape::Line { a, b }) = self.shape(axis) else {
            return no("Mirror needs a line to mirror about.");
        };
        let d = (b - a)
            .normalized()
            .ok_or(EditError::Refused("The line to mirror about has no length."))?;
        let reflect = |p: Point| {
            let v = p - a;
            a + d * (2.0 * v.dot(d)) - v
        };
        let on_axis = |p: Point| (p - a).cross(d).abs() < 1e-6;
        let list: Vec<CurveId> = curves
            .iter()
            .copied()
            .filter(|c| *c != axis && self.curve(*c).is_some())
            .collect();
        if list.is_empty() {
            return no("Select the curves to mirror, then click the line.");
        }
        let mut map: BTreeMap<PointId, PointId> = BTreeMap::new();
        for c in &list {
            for p in self.curve(*c).map(|c| c.points()).unwrap_or_default() {
                if map.contains_key(&p) {
                    continue;
                }
                let pos = self.point(p).ok_or(EditError::Missing)?;
                if on_axis(pos) {
                    map.insert(p, p);
                } else {
                    let q = self.add_point(reflect(pos));
                    map.insert(p, q);
                    let _ = self.add_constraint(Constraint::Symmetric { a: p, b: q, line: axis });
                }
            }
        }
        let m = |p: PointId| map.get(&p).copied().unwrap_or(p);
        let mut made = Vec::new();
        for c in list {
            let curve = *self.curve(c).ok_or(EditError::Missing)?;
            if curve.points().iter().all(|p| m(*p) == *p) {
                // On the line: it is its own mirror.
                continue;
            }
            let new = match curve.geometry {
                Geometry::Line { a, b } => self.add_curve(Geometry::Line { a: m(a), b: m(b) }),
                Geometry::Bezier { start, c1, c2, end } => self.add_curve(Geometry::Bezier {
                    start: m(start),
                    c1: m(c1),
                    c2: m(c2),
                    end: m(end),
                }),
                Geometry::Circle { center, radius } => {
                    let new = self.add_curve(Geometry::Circle {
                        center: m(center),
                        radius,
                    });
                    let _ = self.add_constraint(Constraint::Equal { a: c, b: new });
                    new
                }
                // A mirror turns the direction of an arc.
                Geometry::Arc {
                    center,
                    start,
                    end,
                    radius,
                } => self.add_curve(Geometry::Arc {
                    center: m(center),
                    start: m(end),
                    end: m(start),
                    radius,
                }),
            };
            self.set_construction(new, curve.construction);
            made.push(new);
        }
        Ok(made)
    }

    /// A copy of `curves` with their points moved by `f`, and with the
    /// constraints between them. `turned`: the copy is turned, so the
    /// horizontal and vertical constraints are not copied.
    fn copy_curves(&mut self, curves: &[CurveId], f: &dyn Fn(Point) -> Point, turned: bool) -> Vec<CurveId> {
        self.copy_curves_scaled(curves, f, turned, 1.0)
    }

    /// As `copy_curves`, with the sizes (radii, dimensions but angles)
    /// times `scale`.
    fn copy_curves_scaled(
        &mut self,
        curves: &[CurveId],
        f: &dyn Fn(Point) -> Point,
        turned: bool,
        scale: f64,
    ) -> Vec<CurveId> {
        let mut pmap: BTreeMap<PointId, PointId> = BTreeMap::new();
        let mut cmap: BTreeMap<CurveId, CurveId> = BTreeMap::new();
        for c in curves {
            let Some(curve) = self.curve(*c).copied() else { continue };
            for p in curve.points() {
                if !pmap.contains_key(&p)
                    && let Some(pos) = self.point(p)
                {
                    let q = self.add_point(f(pos));
                    pmap.insert(p, q);
                }
            }
            let m = |p: PointId| pmap[&p];
            let g = match curve.geometry {
                Geometry::Line { a, b } => Geometry::Line { a: m(a), b: m(b) },
                Geometry::Bezier { start, c1, c2, end } => Geometry::Bezier {
                    start: m(start),
                    c1: m(c1),
                    c2: m(c2),
                    end: m(end),
                },
                Geometry::Circle { center, radius } => Geometry::Circle {
                    center: m(center),
                    radius: radius * scale,
                },
                Geometry::Arc {
                    center,
                    start,
                    end,
                    radius,
                } => Geometry::Arc {
                    center: m(center),
                    start: m(start),
                    end: m(end),
                    radius: radius * scale,
                },
            };
            let new = self.add_curve(g);
            self.set_construction(new, curve.construction);
            if curve.centerline {
                self.set_centerline(new, true);
            }
            cmap.insert(*c, new);
        }
        let inside: Vec<Constraint> = self
            .constraints
            .iter()
            .filter(|c| !c.driven)
            .map(|c| c.constraint)
            .filter(|c| {
                c.points().iter().all(|p| pmap.contains_key(p)) && c.curves().iter().all(|x| cmap.contains_key(x))
            })
            .filter(|c| {
                !(turned
                    && matches!(
                        c,
                        Constraint::Horizontal { .. }
                            | Constraint::Vertical { .. }
                            | Constraint::HorizontalDistance { .. }
                            | Constraint::VerticalDistance { .. }
                    ))
            })
            .collect();
        for c in inside {
            let mut c = c.map_ids(|p| pmap[&p], |x| cmap[&x]);
            if let Constraint::Fix { at, .. } = &mut c {
                *at = f(*at);
            }
            if !matches!(c, Constraint::Angle { .. })
                && let Some(v) = c.value()
            {
                c = c.with_value(v * scale);
            }
            let _ = self.add_constraint(c);
        }
        cmap.values().copied().collect()
    }

    /// Move, rotate or scale parts (P14, ADR 0073): the points of `curves`
    /// and `points` go to `f` of their places, as near as the other
    /// constraints let them (as a drag). `scale` scales the radii of the
    /// circles and the dimensions between the parts (not the angles);
    /// `turned`: the parts turn (their horizontal and vertical constraints
    /// are not copied). With `copy`, copies of the curves go there, with the
    /// constraints between them, and the parts stay. Returns the copies.
    pub fn transform_parts(
        &mut self,
        curves: &[CurveId],
        points: &[PointId],
        f: &dyn Fn(Point) -> Point,
        scale: f64,
        turned: bool,
        copy: bool,
    ) -> Result<Vec<CurveId>, EditError> {
        if !(scale.is_finite() && scale > 1e-6 && scale < 1e6) {
            return no("The scale must be more than 0.");
        }
        let list: Vec<CurveId> = curves.iter().copied().filter(|c| self.curve(*c).is_some()).collect();
        if copy {
            if list.is_empty() {
                return no("Select the curves to copy.");
            }
            return Ok(self.copy_curves_scaled(&list, f, turned, scale));
        }
        let mut moved: BTreeSet<PointId> = points.iter().copied().filter(|p| self.point(*p).is_some()).collect();
        for c in &list {
            moved.extend(self.curve(*c).map(|c| c.points()).unwrap_or_default());
        }
        if moved.is_empty() {
            return no("Select the parts to move.");
        }
        if (scale - 1.0).abs() > 1e-12 {
            let inside: BTreeSet<CurveId> = list.iter().copied().collect();
            for c in &mut self.constraints {
                let k = &c.constraint;
                let all_in =
                    k.points().iter().all(|p| moved.contains(p)) && k.curves().iter().all(|x| inside.contains(x));
                if all_in
                    && !matches!(k, Constraint::Angle { .. })
                    && let Some(v) = k.value()
                {
                    c.constraint = k.with_value(v * scale);
                }
            }
            for c in &mut self.curves {
                if inside.contains(&c.id)
                    && let Geometry::Circle { radius, .. } = &mut c.geometry
                {
                    *radius *= scale;
                }
            }
        }
        let moves: Vec<(PointId, Point)> = moved.iter().filter_map(|p| Some((*p, f(self.point(*p)?)))).collect();
        self.drag_points(&moves);
        Ok(Vec::new())
    }

    fn pattern_curves(&self, curves: &[CurveId]) -> Result<Vec<CurveId>, EditError> {
        let list: Vec<CurveId> = curves.iter().copied().filter(|c| self.curve(*c).is_some()).collect();
        if list.is_empty() {
            return no("Select the curves to copy.");
        }
        Ok(list)
    }

    /// Rectangular pattern: `cols` × `rows` places of `curves` (the curves
    /// are the first), `dx` and `dy` mm apart. The copies have the
    /// constraints of the curves between them; their places are free.
    pub fn rect_pattern(
        &mut self,
        curves: &[CurveId],
        cols: usize,
        rows: usize,
        dx: f64,
        dy: f64,
    ) -> Result<Vec<CurveId>, EditError> {
        let list = self.pattern_curves(curves)?;
        if cols == 0 || rows == 0 || cols * rows < 2 || cols * rows > MAX_COPIES {
            return no("A pattern needs 2 to 400 places.");
        }
        if !(dx.is_finite() && dy.is_finite() && dx.abs() <= MAX_MM && dy.abs() <= MAX_MM) {
            return no("The spacing must be a distance.");
        }
        let mut made = Vec::new();
        for j in 0..rows {
            for i in 0..cols {
                if i == 0 && j == 0 {
                    continue;
                }
                let step = Point::new(dx * i as f64, dy * j as f64);
                made.extend(self.copy_curves(&list, &|p| p + step, false));
            }
        }
        Ok(made)
    }

    /// Circular pattern: `count` places of `curves` round `center` (the
    /// curves are the first), on `total_deg` degrees (360: all round). The
    /// copies have the constraints of the curves between them, but not
    /// horizontal and vertical; their places are free.
    pub fn circular_pattern(
        &mut self,
        curves: &[CurveId],
        center: Point,
        count: usize,
        total_deg: f64,
    ) -> Result<Vec<CurveId>, EditError> {
        let list = self.pattern_curves(curves)?;
        if !(2..=MAX_COPIES).contains(&count) {
            return no("A pattern needs 2 to 400 places.");
        }
        if !(total_deg.is_finite() && total_deg.abs() > 1e-6 && total_deg.abs() <= 360.0) {
            return no("The angle must be more than 0° and at most 360°.");
        }
        let full = (total_deg.abs() - 360.0).abs() < 1e-9;
        let step = if full {
            total_deg / count as f64
        } else {
            total_deg / (count - 1) as f64
        };
        let mut made = Vec::new();
        for k in 1..count {
            let (s, c) = math::sin_cos_deg(step * k as f64);
            let turn = move |p: Point| {
                let v = p - center;
                center + Point::new(v.x * c - v.y * s, v.x * s + v.y * c)
            };
            made.extend(self.copy_curves(&list, &turn, true));
        }
        Ok(made)
    }

    /// Relax: removes the constraints that conflict, dimensions first, until
    /// no conflict is left. Returns the removed constraints.
    pub fn relax(&mut self) -> Vec<Constraint> {
        let mut removed = Vec::new();
        for _ in 0..64 {
            self.solve();
            let a = self.analyze();
            if a.conflicting.is_empty() {
                break;
            }
            let dims: Vec<ConstraintId> = a
                .conflicting
                .iter()
                .copied()
                .filter(|id| self.constraint(*id).is_some_and(|c| c.is_dimension()))
                .collect();
            let take = if dims.is_empty() { a.conflicting } else { dims };
            for id in take {
                if let Some(c) = self.constraint(id).copied() {
                    removed.push(c);
                    self.remove_constraint(id);
                }
            }
        }
        self.solve();
        removed
    }
}

/// The corner of two lines.
struct CornerAngle {
    p: Point,
    /// The directions along the lines from the corner.
    u: [Point; 2],
    len: [f64; 2],
    /// The angle between the lines (radians, 0 to π).
    theta: f64,
}

/// A chain of curves.
#[derive(Debug, Clone)]
struct Chain {
    items: Vec<(CurveId, bool)>,
    closed: bool,
}

/// The offset of one curve of a chain.
struct Off {
    carrier: Shape,
    /// Its ends in the order of the chain.
    ends: (Point, Point),
    /// The center point and the new radius of an arc.
    center: Option<(PointId, f64)>,
}
