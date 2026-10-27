//! The curves of a sketch as shapes in place (P14 Milestone 3): the
//! nearest point of a curve, the place of a point along a curve, and the
//! points where two curves cross. The edits (trim, extend, split) and the
//! drawing help use them.
//!
//! A spline piece (P14 Milestone 6, ADR 0079) is a cubic Bézier curve: its
//! nearest point and its crossings are found on a polyline of it, then
//! made exact with Newton steps.

use std::f64::consts::TAU;

use emb_geom::{Point, math};

use crate::{CurveId, Geometry, Sketch};

/// The tolerance for "the same place" in mm.
pub const EPS: f64 = 1e-7;

/// A curve with the positions of its points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// From `a` to `b`. The parameter is 0 at `a` and 1 at `b`.
    Line { a: Point, b: Point },
    /// The parameter is the angle from the +x direction, in [0, 2π).
    Circle { c: Point, r: f64 },
    /// Counterclockwise from the angle `a0` through `sweep` (radians). The
    /// parameter is the angle from the start, in [0, sweep].
    Arc { c: Point, r: f64, a0: f64, sweep: f64 },
    /// A cubic Bézier curve from `p[0]` to `p[3]` with the handles `p[1]`
    /// and `p[2]`. The parameter is in [0, 1].
    Cubic { p: [Point; 4] },
}

/// The point of a cubic Bézier curve at `t`.
pub fn cubic_at(p: &[Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    p[0] * (u * u * u) + p[1] * (3.0 * u * u * t) + p[2] * (3.0 * u * t * t) + p[3] * (t * t * t)
}

/// The first derivative of a cubic Bézier curve at `t`.
pub fn cubic_d1(p: &[Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    (p[1] - p[0]) * (3.0 * u * u) + (p[2] - p[1]) * (6.0 * u * t) + (p[3] - p[2]) * (3.0 * t * t)
}

fn cubic_d2(p: &[Point; 4], t: f64) -> Point {
    (p[2] - p[1] * 2.0 + p[0]) * (6.0 * (1.0 - t)) + (p[3] - p[2] * 2.0 + p[1]) * (6.0 * t)
}

/// The part of a cubic Bézier curve from `t0` to `t1` (de Casteljau).
pub fn cubic_part(p: &[Point; 4], t0: f64, t1: f64) -> [Point; 4] {
    let (a, b) = (cubic_at(p, t0), cubic_at(p, t1));
    let k = (t1 - t0) / 3.0;
    [a, a + cubic_d1(p, t0) * k, b - cubic_d1(p, t1) * k, b]
}

/// The length of a cubic Bézier curve from `t0` to `t1` (a polyline of
/// 16 pieces).
fn cubic_len(p: &[Point; 4], t0: f64, t1: f64) -> f64 {
    let n = 16;
    (0..n)
        .map(|i| {
            let (a, b) = (
                t0 + (t1 - t0) * i as f64 / n as f64,
                t0 + (t1 - t0) * (i + 1) as f64 / n as f64,
            );
            cubic_at(p, a).distance(cubic_at(p, b))
        })
        .sum()
}

/// The number of pieces of the polyline of a cubic for crossings.
const PIECES: usize = 48;

/// The angle in [0, 2π).
pub fn wrap_tau(a: f64) -> f64 {
    let a = a % TAU;
    if a < 0.0 { a + TAU } else { a }
}

impl Shape {
    /// The point at parameter `t`.
    pub fn at(&self, t: f64) -> Point {
        match *self {
            Shape::Line { a, b } => a.lerp(b, t),
            Shape::Circle { c, r } => c + Point::new(math::cos(t), math::sin(t)) * r,
            Shape::Arc { c, r, a0, .. } => c + Point::new(math::cos(a0 + t), math::sin(a0 + t)) * r,
            Shape::Cubic { p } => cubic_at(&p, t),
        }
    }

    /// The length of the curve (an estimate for a cubic).
    pub fn length(&self) -> f64 {
        match *self {
            Shape::Line { a, b } => a.distance(b),
            Shape::Circle { r, .. } => r * TAU,
            Shape::Arc { r, sweep, .. } => r * sweep,
            Shape::Cubic { p } => cubic_len(&p, 0.0, 1.0),
        }
    }

    /// The length of the curve from parameter `t0` to `t1`.
    pub fn length_between(&self, t0: f64, t1: f64) -> f64 {
        match *self {
            Shape::Line { a, b } => a.distance(b) * (t1 - t0).abs(),
            Shape::Circle { r, .. } | Shape::Arc { r, .. } => r * (t1 - t0).abs(),
            Shape::Cubic { p } => cubic_len(&p, t0.min(t1), t0.max(t1)),
        }
    }

    /// The parameter of the point of the carrier (the full line or the
    /// full circle) nearest to `p`. For a line it is not clamped; for an arc
    /// it is the angle from the start in [0, 2π).
    pub fn param(&self, p: Point) -> f64 {
        match *self {
            Shape::Line { a, b } => {
                let d = b - a;
                (p - a).dot(d) / d.length_sq().max(EPS * EPS)
            }
            Shape::Circle { c, .. } => wrap_tau(math::atan2(p.y - c.y, p.x - c.x)),
            Shape::Arc { c, a0, .. } => wrap_tau(math::atan2(p.y - c.y, p.x - c.x) - a0),
            Shape::Cubic { p: q } => {
                // The nearest of samples, then Newton steps on the slope of
                // the distance.
                let mut t = (0..=32)
                    .map(|i| i as f64 / 32.0)
                    .min_by(|a, b| cubic_at(&q, *a).distance(p).total_cmp(&cubic_at(&q, *b).distance(p)))
                    .unwrap_or(0.0);
                for _ in 0..10 {
                    let (b, d1, d2) = (cubic_at(&q, t) - p, cubic_d1(&q, t), cubic_d2(&q, t));
                    let g = b.dot(d1);
                    let h = d1.dot(d1) + b.dot(d2);
                    if h.abs() < 1e-18 {
                        break;
                    }
                    t = (t - g / h).clamp(0.0, 1.0);
                }
                t
            }
        }
    }

    /// The largest parameter: 1 for a line, 2π for a circle, the sweep of
    /// an arc.
    pub fn end_param(&self) -> f64 {
        match *self {
            Shape::Line { .. } => 1.0,
            Shape::Circle { .. } => TAU,
            Shape::Arc { sweep, .. } => sweep,
            Shape::Cubic { .. } => 1.0,
        }
    }

    /// True if parameter `t` is on the curve (not only on its carrier).
    pub fn has_param(&self, t: f64, tol: f64) -> bool {
        match *self {
            Shape::Line { a, b } => {
                let len = a.distance(b).max(EPS);
                t >= -tol / len && t <= 1.0 + tol / len
            }
            Shape::Circle { .. } => true,
            Shape::Arc { r, sweep, .. } => {
                let k = tol / r.max(EPS);
                t <= sweep + k || t >= TAU - k
            }
            Shape::Cubic { .. } => {
                let k = tol / self.length().max(EPS);
                t >= -k && t <= 1.0 + k
            }
        }
    }

    /// The point of the curve nearest to `p`, and its parameter.
    pub fn nearest(&self, p: Point) -> (Point, f64) {
        match *self {
            Shape::Line { .. } => {
                let t = self.param(p).clamp(0.0, 1.0);
                (self.at(t), t)
            }
            Shape::Circle { .. } => {
                let t = self.param(p);
                (self.at(t), t)
            }
            Shape::Arc { sweep, .. } => {
                let t = self.param(p);
                if t <= sweep {
                    return (self.at(t), t);
                }
                // Past the end: the nearer end.
                let (s, e) = (self.at(0.0), self.at(sweep));
                if p.distance(s) <= p.distance(e) {
                    (s, 0.0)
                } else {
                    (e, sweep)
                }
            }
            Shape::Cubic { .. } => {
                let t = self.param(p);
                (self.at(t), t)
            }
        }
    }

    /// The direction of the curve at parameter `t` (not normalized for a
    /// line).
    pub fn tangent(&self, t: f64) -> Point {
        match *self {
            Shape::Line { a, b } => b - a,
            Shape::Circle { .. } => Point::new(-math::sin(t), math::cos(t)),
            Shape::Arc { a0, .. } => Point::new(-math::sin(a0 + t), math::cos(a0 + t)),
            Shape::Cubic { p } => cubic_d1(&p, t),
        }
    }

    /// The signed distance from `x` to the carrier (for a cubic: to the
    /// curve, on the left of its direction positive).
    fn side(&self, x: Point) -> f64 {
        match *self {
            Shape::Line { a, b } => (b - a).cross(x - a) / a.distance(b).max(EPS),
            Shape::Circle { c, r } | Shape::Arc { c, r, .. } => x.distance(c) - r,
            Shape::Cubic { .. } => {
                let t = self.param(x);
                let d = self.tangent(t);
                d.cross(x - self.at(t)) / d.length().max(EPS)
            }
        }
    }
}

/// The points of a polyline of cubic `p` and their parameters.
fn cubic_polyline(p: &[Point; 4]) -> Vec<(f64, Point)> {
    (0..=PIECES)
        .map(|i| {
            let t = i as f64 / PIECES as f64;
            (t, cubic_at(p, t))
        })
        .collect()
}

/// The crossing of the segments `a`–`b` and `c`–`d`: the parameters on
/// each.
fn segments(a: Point, b: Point, c: Point, d: Point) -> Option<(f64, f64)> {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() < 1e-18 {
        return None;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    let k = 1e-9;
    (t >= -k && t <= 1.0 + k && u >= -k && u <= 1.0 + k).then_some((t, u))
}

/// The points where cubic `p` crosses the carrier of `q` (a line, a circle
/// or another cubic).
fn cubic_crossings(p: &[Point; 4], q: &Shape) -> Vec<Point> {
    let poly = cubic_polyline(p);
    let mut ts: Vec<f64> = Vec::new();
    let mut both: Vec<(f64, f64)> = Vec::new();
    for w in poly.windows(2) {
        let ((t0, a), (t1, b)) = (w[0], w[1]);
        match *q {
            Shape::Cubic { p: r } => {
                let other = cubic_polyline(&r);
                for v in other.windows(2) {
                    if let Some((s, u)) = segments(a, b, v[0].1, v[1].1) {
                        both.push((t0 + (t1 - t0) * s, v[0].0 + (v[1].0 - v[0].0) * u));
                    }
                }
            }
            _ => {
                let (fa, fb) = (q.side(a), q.side(b));
                if fa == 0.0 || (fa < 0.0) != (fb < 0.0) || fb == 0.0 {
                    let k = if (fa - fb).abs() > 1e-18 { fa / (fa - fb) } else { 0.0 };
                    ts.push(t0 + (t1 - t0) * k);
                }
            }
        }
    }
    let mut out: Vec<Point> = Vec::new();
    let mut add = |x: Point| {
        if !out.iter().any(|y| y.distance(x) < 1e-6) {
            out.push(x);
        }
    };
    // Newton steps on the parameter of the cubic: its point on the carrier.
    for mut t in ts {
        for _ in 0..12 {
            let f = q.side(cubic_at(p, t));
            let h = 1e-7;
            let df = (q.side(cubic_at(p, t + h)) - q.side(cubic_at(p, t - h))) / (2.0 * h);
            if df.abs() < 1e-14 || f.abs() < 1e-12 {
                break;
            }
            t = (t - f / df).clamp(0.0, 1.0);
        }
        add(cubic_at(p, t));
    }
    // Two cubics: Newton steps on both parameters.
    if let Shape::Cubic { p: r } = *q {
        for (mut t, mut u) in both {
            for _ in 0..12 {
                let f = cubic_at(p, t) - cubic_at(&r, u);
                if f.length() < 1e-12 {
                    break;
                }
                let (a, b) = (cubic_d1(p, t), cubic_d1(&r, u) * -1.0);
                let det = a.cross(b);
                if det.abs() < 1e-18 {
                    break;
                }
                // Solve a·dt + b·du = −f.
                let dt = -(f.cross(b)) / det;
                let du = -(a.cross(f)) / det;
                t = (t + dt).clamp(0.0, 1.0);
                u = (u + du).clamp(0.0, 1.0);
            }
            add(cubic_at(p, t));
        }
    }
    out
}

/// The points where the carriers of two shapes cross: the full lines and
/// the full circles.
pub fn carrier_crossings(p: &Shape, q: &Shape) -> Vec<Point> {
    let round = |s: &Shape| match *s {
        Shape::Circle { c, r } | Shape::Arc { c, r, .. } => Some((c, r)),
        Shape::Line { .. } | Shape::Cubic { .. } => None,
    };
    // A cubic: its carrier is the curve itself; an arc's is its circle.
    let carrier = |s: &Shape| match *s {
        Shape::Arc { c, r, .. } => Shape::Circle { c, r },
        other => other,
    };
    match (*p, *q) {
        (Shape::Cubic { p: a }, other) | (other, Shape::Cubic { p: a }) => {
            return cubic_crossings(&a, &carrier(&other));
        }
        _ => {}
    }
    match (*p, *q, round(p), round(q)) {
        (Shape::Line { a, b }, Shape::Line { a: c, b: d }, ..) => {
            let (r, s) = (b - a, d - c);
            let den = r.cross(s);
            if den.abs() < 1e-12 * r.length().max(1.0) * s.length().max(1.0) {
                return Vec::new();
            }
            let t = (c - a).cross(s) / den;
            vec![a + r * t]
        }
        (Shape::Line { a, b }, _, _, Some((c, r))) | (_, Shape::Line { a, b }, Some((c, r)), _) => {
            line_circle(a, b, c, r)
        }
        (_, _, Some((c1, r1)), Some((c2, r2))) => circle_circle(c1, r1, c2, r2),
        _ => Vec::new(),
    }
}

fn line_circle(a: Point, b: Point, c: Point, r: f64) -> Vec<Point> {
    let Some(d) = (b - a).normalized() else {
        return Vec::new();
    };
    let t0 = (c - a).dot(d);
    let foot = a + d * t0;
    let h2 = r * r - foot.distance(c).powi(2);
    if h2 < -EPS * r.max(1.0) {
        return Vec::new();
    }
    let h = h2.max(0.0).sqrt();
    if h < EPS {
        return vec![foot];
    }
    vec![foot - d * h, foot + d * h]
}

fn circle_circle(c1: Point, r1: f64, c2: Point, r2: f64) -> Vec<Point> {
    let d = c1.distance(c2);
    if d < EPS || d > r1 + r2 + EPS || d < (r1 - r2).abs() - EPS {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let u = (c2 - c1) * (1.0 / d);
    let m = c1 + u * a;
    if h < EPS {
        return vec![m];
    }
    vec![m + u.perp() * h, m - u.perp() * h]
}

/// The points where two shapes cross (on both curves, not only on their
/// carriers).
pub fn crossings(p: &Shape, q: &Shape) -> Vec<Point> {
    let tol = 1e-6;
    carrier_crossings(p, q)
        .into_iter()
        .filter(|x| p.has_param(p.param(*x), tol) && q.has_param(q.param(*x), tol))
        .collect()
}

impl Sketch {
    /// The shape of curve `id` in place.
    pub fn shape(&self, id: CurveId) -> Option<Shape> {
        let c = self.curve(id)?;
        let p = |id| self.point(id);
        Some(match c.geometry {
            Geometry::Line { a, b } => Shape::Line { a: p(a)?, b: p(b)? },
            Geometry::Circle { center, radius } => Shape::Circle {
                c: p(center)?,
                r: radius,
            },
            Geometry::Arc {
                center,
                start,
                end,
                radius,
            } => {
                let c = p(center)?;
                let (s, e) = (p(start)?, p(end)?);
                let a0 = math::atan2(s.y - c.y, s.x - c.x);
                let mut sweep = wrap_tau(math::atan2(e.y - c.y, e.x - c.x) - a0);
                if sweep < 1e-12 {
                    sweep = TAU;
                }
                Shape::Arc {
                    c,
                    r: radius,
                    a0,
                    sweep,
                }
            }
            Geometry::Bezier { start, c1, c2, end } => Shape::Cubic {
                p: [p(start)?, p(c1)?, p(c2)?, p(end)?],
            },
        })
    }

    /// The curve nearest to `at` within `tol` mm, and its nearest point.
    /// `skip` curves are not used.
    pub fn nearest_curve(&self, at: Point, tol: f64, skip: &[CurveId]) -> Option<(CurveId, Point)> {
        self.curves
            .iter()
            .filter(|c| !skip.contains(&c.id))
            .filter_map(|c| {
                let (q, _) = self.shape(c.id)?.nearest(at);
                let d = q.distance(at);
                (d <= tol).then_some((c.id, q, d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, q, _)| (id, q))
    }
}
