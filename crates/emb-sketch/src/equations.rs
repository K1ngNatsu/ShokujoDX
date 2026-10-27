//! The variables of a sketch and the equations of its constraints
//! (ADR 0068). Each equation is 0 when its constraint is true.

use std::collections::BTreeMap;

use emb_geom::{Point, math};

use crate::{Constraint, ConstraintId, CurveId, Geometry, PointId, Sketch};

/// What a variable is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Var {
    X(PointId),
    Y(PointId),
    Radius(CurveId),
}

/// The variables of a sketch: the x and y of each point, the radius of
/// each circle and arc.
pub(crate) struct Layout {
    pub x: Vec<f64>,
    pub vars: Vec<Var>,
    point: BTreeMap<PointId, usize>,
    radius: BTreeMap<CurveId, usize>,
}

impl Layout {
    pub fn of(s: &Sketch) -> Layout {
        let mut l = Layout {
            x: Vec::new(),
            vars: Vec::new(),
            point: BTreeMap::new(),
            radius: BTreeMap::new(),
        };
        for p in &s.points {
            l.point.insert(p.id, l.x.len());
            l.x.extend([p.pos.x, p.pos.y]);
            l.vars.extend([Var::X(p.id), Var::Y(p.id)]);
        }
        for c in &s.curves {
            if let Geometry::Circle { radius, .. } | Geometry::Arc { radius, .. } = c.geometry {
                l.radius.insert(c.id, l.x.len());
                l.x.push(radius);
                l.vars.push(Var::Radius(c.id));
            }
        }
        l
    }

    /// The index of the x of point `p` (y is the next one).
    pub fn point(&self, p: PointId) -> Option<usize> {
        self.point.get(&p).copied()
    }

    /// Writes the variables `x` back to the sketch. A radius stays positive.
    pub fn write_back(&self, x: &[f64], s: &mut Sketch) {
        for p in &mut s.points {
            if let Some(&i) = self.point.get(&p.id) {
                p.pos = Point::new(x[i], x[i + 1]);
            }
        }
        for c in &mut s.curves {
            if let (Some(&i), Geometry::Circle { radius, .. } | Geometry::Arc { radius, .. }) =
                (self.radius.get(&c.id), &mut c.geometry)
            {
                *radius = x[i].abs().max(1e-9);
            }
        }
    }
}

/// A residual function of the variables.
pub(crate) type Residual = Box<dyn Fn(&[f64]) -> f64>;

/// One equation: 0 when true.
pub(crate) struct Equation {
    /// The constraint of the equation (`None` for the equations of an arc).
    pub owner: Option<ConstraintId>,
    /// The variables that the equation uses.
    pub vars: Vec<usize>,
    pub f: Residual,
}

fn pt(x: &[f64], i: usize) -> Point {
    Point::new(x[i], x[i + 1])
}

/// A line as the variable indices of its two points.
#[derive(Clone, Copy)]
struct L(usize, usize);

impl L {
    fn dir(self, x: &[f64]) -> Point {
        pt(x, self.1) - pt(x, self.0)
    }
}

/// A circle or an arc as the variable indices of its center and radius.
#[derive(Clone, Copy)]
struct R(usize, usize);

/// The smallest length that a division uses, so a line of length 0 gives
/// no division by 0.
const TINY: f64 = 1e-9;

/// The angle in (−π, π].
fn wrap(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    let mut a = a % t;
    if a > std::f64::consts::PI {
        a -= t;
    } else if a <= -std::f64::consts::PI {
        a += t;
    }
    a
}

/// The equations of the sketch.
pub(crate) fn equations(s: &Sketch, l: &Layout) -> Vec<Equation> {
    let mut out: Vec<Equation> = Vec::new();
    let line = |id: CurveId| -> Option<L> {
        match s.curve(id)?.geometry {
            Geometry::Line { a, b } => Some(L(l.point(a)?, l.point(b)?)),
            _ => None,
        }
    };
    let round = |id: CurveId| -> Option<R> {
        match s.curve(id)?.geometry {
            Geometry::Circle { center, .. } | Geometry::Arc { center, .. } => {
                Some(R(l.point(center)?, *l.radius.get(&id)?))
            }
            Geometry::Line { .. } | Geometry::Bezier { .. } => None,
        }
    };
    let p = |id: PointId| l.point(id);
    let mut push = |owner: Option<ConstraintId>, vars: Vec<usize>, f: Residual| {
        // Each point gives its x and y.
        let mut v: Vec<usize> = Vec::new();
        for i in vars {
            if !v.contains(&i) {
                v.push(i);
            }
        }
        out.push(Equation { owner, vars: v, f });
    };
    let pv = |i: usize| [i, i + 1];
    // The point that a line and an arc, or two arcs, share at their ends
    // (the same point, or coincident points): a tangent there is an angle
    // of 90° at the point, which the solver finds well (the distance from
    // the center to the line is not exact enough, because the point can
    // slide along the line).
    let arc_ends = |id: CurveId| match s.curve(id).and_then(|c| c.ends()) {
        Some((a, b)) => vec![a, b],
        None => Vec::new(),
    };
    // The ends of two curves that are one point (or coincident points).
    let shared_pair = |a: CurveId, b: CurveId| -> Option<(PointId, PointId)> {
        let eb = arc_ends(b);
        arc_ends(a)
            .into_iter()
            .find_map(|e| eb.iter().find(|f| **f == e || s.joined(e).contains(f)).map(|f| (e, *f)))
    };
    // The direction of a curve away from its end `e`: two points (from,
    // to) for a line or a spline piece (its handle), else the center and
    // the end of an arc (the direction is square to them).
    enum Away {
        Seg(usize, usize),
        Radial(usize, usize),
    }
    let away = |id: CurveId, e: PointId| -> Option<Away> {
        match s.curve(id)?.geometry {
            Geometry::Line { a, b } => {
                let other = if a == e { b } else { a };
                Some(Away::Seg(l.point(e)?, l.point(other)?))
            }
            Geometry::Bezier { start, c1, c2, .. } => {
                let h = if start == e { c1 } else { c2 };
                Some(Away::Seg(l.point(e)?, l.point(h)?))
            }
            Geometry::Arc { center, .. } => Some(Away::Radial(l.point(center)?, l.point(e)?)),
            Geometry::Circle { .. } => None,
        }
    };
    let bez = |id: CurveId| -> Option<[usize; 4]> {
        match s.curve(id)?.geometry {
            Geometry::Bezier { start, c1, c2, end } => {
                Some([l.point(start)?, l.point(c1)?, l.point(c2)?, l.point(end)?])
            }
            _ => None,
        }
    };
    let shared = |a: CurveId, b: CurveId| -> Option<usize> {
        let eb = arc_ends(b);
        arc_ends(a)
            .into_iter()
            .find(|e| eb.iter().any(|f| f == e || s.joined(*e).contains(f)))
            .and_then(|e| l.point(e))
    };

    // The two equations of each arc: its ends are on its circle.
    for c in &s.curves {
        if let Geometry::Arc { center, start, end, .. } = c.geometry {
            let (Some(cv), Some(sv), Some(ev), Some(&rv)) = (p(center), p(start), p(end), l.radius.get(&c.id)) else {
                continue;
            };
            for q in [sv, ev] {
                let mut vars = pv(cv).to_vec();
                vars.extend(pv(q));
                vars.push(rv);
                push(None, vars, Box::new(move |x| pt(x, q).distance(pt(x, cv)) - x[rv]));
            }
        }
    }

    for sc in s.constraints.iter().filter(|c| !c.driven) {
        let o = Some(sc.id);
        match sc.constraint {
            Constraint::Coincident { a, b } => {
                let (Some(a), Some(b)) = (p(a), p(b)) else { continue };
                push(o, vec![a, b], Box::new(move |x| x[a] - x[b]));
                push(o, vec![a + 1, b + 1], Box::new(move |x| x[a + 1] - x[b + 1]));
            }
            Constraint::Horizontal { line: id } => {
                let Some(L(a, b)) = line(id) else { continue };
                push(o, vec![a + 1, b + 1], Box::new(move |x| x[a + 1] - x[b + 1]));
            }
            Constraint::Vertical { line: id } => {
                let Some(L(a, b)) = line(id) else { continue };
                push(o, vec![a, b], Box::new(move |x| x[a] - x[b]));
            }
            Constraint::Parallel { a, b } | Constraint::Perpendicular { a, b } => {
                let (Some(la), Some(lb)) = (line(a), line(b)) else {
                    continue;
                };
                let parallel = matches!(sc.constraint, Constraint::Parallel { .. });
                let mut vars = [pv(la.0), pv(la.1), pv(lb.0), pv(lb.1)].concat();
                vars.dedup();
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let (da, db) = (la.dir(x), lb.dir(x));
                        let n = (da.length() * db.length()).max(TINY);
                        if parallel { da.cross(db) / n } else { da.dot(db) / n }
                    }),
                );
            }
            Constraint::Collinear { a, b } => {
                let (Some(la), Some(lb)) = (line(a), line(b)) else {
                    continue;
                };
                for q in [lb.0, lb.1] {
                    let vars = [pv(q), pv(la.0), pv(la.1)].concat();
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let d = la.dir(x);
                            d.cross(pt(x, q) - pt(x, la.0)) / d.length().max(TINY)
                        }),
                    );
                }
            }
            Constraint::Equal { a, b } => match (line(a), line(b), round(a), round(b)) {
                (Some(la), Some(lb), ..) => {
                    let vars = [pv(la.0), pv(la.1), pv(lb.0), pv(lb.1)].concat();
                    push(o, vars, Box::new(move |x| la.dir(x).length() - lb.dir(x).length()));
                }
                (_, _, Some(ra), Some(rb)) => push(o, vec![ra.1, rb.1], Box::new(move |x| x[ra.1] - x[rb.1])),
                _ => continue,
            },
            // A spline piece: the directions at the shared end are on one
            // line (ADR 0079).
            Constraint::Tangent { a, b, .. } if bez(a).is_some() || bez(b).is_some() => {
                let Some((ea, eb)) = shared_pair(a, b) else { continue };
                let (Some(da), Some(db)) = (away(a, ea), away(b, eb)) else {
                    continue;
                };
                let idx = |d: &Away| match *d {
                    Away::Seg(f, t) | Away::Radial(f, t) => [pv(f), pv(t)].concat(),
                };
                let vars = [idx(&da), idx(&db)].concat();
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let dir = |d: &Away| match *d {
                            Away::Seg(f, t) => (pt(x, t) - pt(x, f), false),
                            Away::Radial(c, e) => (pt(x, e) - pt(x, c), true),
                        };
                        let ((u, ru), (v, rv)) = (dir(&da), dir(&db));
                        let n = (u.length() * v.length()).max(TINY);
                        // Square to the radius of an arc; else the two
                        // directions away from the end are opposite (the
                        // angle between them is 180°, not 0°: no cusp).
                        if ru != rv {
                            u.dot(v) / n
                        } else {
                            math::atan2(u.cross(v), -u.dot(v))
                        }
                    }),
                );
            }
            Constraint::Tangent { a, b, inside } => match (line(a), line(b), round(a), round(b)) {
                (Some(ln), _, _, Some(r)) | (_, Some(ln), Some(r), _) if shared(a, b).is_some() => {
                    let q = shared(a, b).expect("a shared point");
                    let mut vars = [pv(q), pv(ln.0), pv(ln.1), pv(r.0)].concat();
                    vars.push(r.1);
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let (d, radial) = (ln.dir(x), pt(x, q) - pt(x, r.0));
                            d.dot(radial) / (d.length() * radial.length()).max(TINY)
                        }),
                    );
                }
                (_, _, Some(ra), Some(rb)) if shared(a, b).is_some() => {
                    let q = shared(a, b).expect("a shared point");
                    let vars = [pv(q), pv(ra.0), pv(rb.0)].concat();
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let (u, v) = (pt(x, ra.0) - pt(x, q), pt(x, rb.0) - pt(x, q));
                            u.cross(v) / (u.length() * v.length()).max(TINY)
                        }),
                    );
                }
                (Some(ln), _, _, Some(r)) | (_, Some(ln), Some(r), _) => {
                    let mut vars = [pv(ln.0), pv(ln.1), pv(r.0)].concat();
                    vars.push(r.1);
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let d = ln.dir(x);
                            let len = d.length().max(TINY);
                            (d.cross(pt(x, r.0) - pt(x, ln.0)) / len).abs() - x[r.1]
                        }),
                    );
                }
                (_, _, Some(ra), Some(rb)) => {
                    let mut vars = [pv(ra.0), pv(rb.0)].concat();
                    vars.extend([ra.1, rb.1]);
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let d = pt(x, ra.0).distance(pt(x, rb.0));
                            if inside {
                                d - (x[ra.1] - x[rb.1]).abs()
                            } else {
                                d - (x[ra.1] + x[rb.1])
                            }
                        }),
                    );
                }
                _ => continue,
            },
            Constraint::Concentric { a, b } => {
                let (Some(ra), Some(rb)) = (round(a), round(b)) else {
                    continue;
                };
                let (a, b) = (ra.0, rb.0);
                push(o, vec![a, b], Box::new(move |x| x[a] - x[b]));
                push(o, vec![a + 1, b + 1], Box::new(move |x| x[a + 1] - x[b + 1]));
            }
            Constraint::Midpoint { point, line: id } => {
                let (Some(q), Some(L(a, b))) = (p(point), line(id)) else {
                    continue;
                };
                push(o, vec![q, a, b], Box::new(move |x| x[q] - (x[a] + x[b]) / 2.0));
                push(
                    o,
                    vec![q + 1, a + 1, b + 1],
                    Box::new(move |x| x[q + 1] - (x[a + 1] + x[b + 1]) / 2.0),
                );
            }
            Constraint::OnCurve { point, curve } => {
                let Some(q) = p(point) else { continue };
                if let Some(ln) = line(curve) {
                    let vars = [pv(q), pv(ln.0), pv(ln.1)].concat();
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let d = ln.dir(x);
                            d.cross(pt(x, q) - pt(x, ln.0)) / d.length().max(TINY)
                        }),
                    );
                } else if let Some(r) = round(curve) {
                    let mut vars = [pv(q), pv(r.0)].concat();
                    vars.push(r.1);
                    push(o, vars, Box::new(move |x| pt(x, q).distance(pt(x, r.0)) - x[r.1]));
                } else if let Some(c) = bez(curve) {
                    // The distance to the spline piece, on the left of it
                    // positive.
                    let vars = [pv(q), pv(c[0]), pv(c[1]), pv(c[2]), pv(c[3])].concat();
                    push(
                        o,
                        vars,
                        Box::new(move |x| {
                            let sh = crate::Shape::Cubic {
                                p: [pt(x, c[0]), pt(x, c[1]), pt(x, c[2]), pt(x, c[3])],
                            };
                            let at = pt(x, q);
                            let t = sh.param(at);
                            let d = sh.tangent(t);
                            d.cross(at - sh.at(t)) / d.length().max(TINY)
                        }),
                    );
                }
            }
            Constraint::Symmetric { a, b, line: id } => {
                let (Some(qa), Some(qb), Some(ln)) = (p(a), p(b), line(id)) else {
                    continue;
                };
                let vars = [pv(qa), pv(qb), pv(ln.0), pv(ln.1)].concat();
                push(
                    o,
                    vars.clone(),
                    Box::new(move |x| {
                        let d = ln.dir(x);
                        let m = pt(x, qa).midpoint(pt(x, qb));
                        d.cross(m - pt(x, ln.0)) / d.length().max(TINY)
                    }),
                );
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let d = ln.dir(x);
                        (pt(x, qb) - pt(x, qa)).dot(d) / d.length().max(TINY)
                    }),
                );
            }
            Constraint::Fix { point, at } => {
                let Some(q) = p(point) else { continue };
                push(o, vec![q], Box::new(move |x| x[q] - at.x));
                push(o, vec![q + 1], Box::new(move |x| x[q + 1] - at.y));
            }
            Constraint::Distance { a, b, mm } => {
                let (Some(a), Some(b)) = (p(a), p(b)) else { continue };
                push(
                    o,
                    [pv(a), pv(b)].concat(),
                    Box::new(move |x| pt(x, a).distance(pt(x, b)) - mm),
                );
            }
            Constraint::HorizontalDistance { a, b, mm } => {
                let (Some(a), Some(b)) = (p(a), p(b)) else { continue };
                push(o, vec![a, b], Box::new(move |x| x[b] - x[a] - mm));
            }
            Constraint::VerticalDistance { a, b, mm } => {
                let (Some(a), Some(b)) = (p(a), p(b)) else { continue };
                push(o, vec![a + 1, b + 1], Box::new(move |x| x[b + 1] - x[a + 1] - mm));
            }
            Constraint::LineDistance { point, line: id, mm } => {
                let (Some(q), Some(ln)) = (p(point), line(id)) else {
                    continue;
                };
                // To a centerline the value is a diameter.
                let mm = if s.is_centerline(id) { mm / 2.0 } else { mm };
                let vars = [pv(q), pv(ln.0), pv(ln.1)].concat();
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let d = ln.dir(x);
                        d.cross(pt(x, q) - pt(x, ln.0)) / d.length().max(TINY) - mm
                    }),
                );
            }
            Constraint::Length { line: id, mm } => {
                let Some(ln) = line(id) else { continue };
                push(
                    o,
                    [pv(ln.0), pv(ln.1)].concat(),
                    Box::new(move |x| ln.dir(x).length() - mm),
                );
            }
            Constraint::Angle { a, b, deg } => {
                let (Some(la), Some(lb)) = (line(a), line(b)) else {
                    continue;
                };
                let target = math::to_radians(deg);
                let vars = [pv(la.0), pv(la.1), pv(lb.0), pv(lb.1)].concat();
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let (da, db) = (la.dir(x), lb.dir(x));
                        wrap(math::atan2(da.cross(db), da.dot(db)) - target)
                    }),
                );
            }
            Constraint::ArcLength { curve, mm } => {
                let (Some(r), Some(Geometry::Arc { start, end, .. })) =
                    (round(curve), s.curve(curve).map(|c| c.geometry))
                else {
                    continue;
                };
                let (Some(sv), Some(ev)) = (p(start), p(end)) else {
                    continue;
                };
                let mut vars = [pv(r.0), pv(sv), pv(ev)].concat();
                vars.push(r.1);
                push(
                    o,
                    vars,
                    Box::new(move |x| {
                        let c = pt(x, r.0);
                        let (u, v) = (pt(x, sv) - c, pt(x, ev) - c);
                        let mut sweep = math::atan2(u.cross(v), u.dot(v));
                        if sweep <= 0.0 {
                            sweep += std::f64::consts::TAU;
                        }
                        x[r.1] * sweep - mm
                    }),
                );
            }
            Constraint::Radius { curve, mm } | Constraint::Diameter { curve, mm } => {
                let Some(r) = round(curve) else { continue };
                let k = if matches!(sc.constraint, Constraint::Diameter { .. }) {
                    2.0
                } else {
                    1.0
                };
                let rv = r.1;
                push(o, vec![rv], Box::new(move |x| k * x[rv] - mm));
            }
        }
    }
    out
}
