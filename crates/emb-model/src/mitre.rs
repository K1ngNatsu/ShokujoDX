//! Mitre corners of satin columns (ADR 0086).
//!
//! Two columns that meet at a corner sew as a stop and a start: the first
//! column ends, and the second column starts over its end (a lap). A mitre
//! corner makes them one column that turns: the rails meet on the line
//! from the outer corner to the inner corner, and the stitches fan round
//! the inner corner. The thread does not build up at the corner, and there
//! is no stop and start.
//!
//! - [`join`]: the end of one column and the start of the next as one column.
//! - [`close`]: a column whose end meets its start as a ring.
//! - [`join_parts`]: all the corners of a list of satin parts (the parts of
//!   a letter, or the selected columns).

use emb_geom::region::ring_contains;
use emb_geom::{Path, Point, math};

use crate::object::{ObjectKind, SatinGeometry};

/// The ends of two columns meet if their middles are nearer than this many
/// widths of the wider end.
const MEET: f64 = 0.9;
/// The least turn at a corner, in degrees: a smaller turn is not a corner.
pub const MIN_TURN_DEG: f64 = 30.0;
/// The largest turn at a mitre corner, in degrees: a sharper corner (the
/// apex of an "A") makes a mitre that is too long, so it stays a lap.
pub const MAX_TURN_DEG: f64 = 120.0;
/// The line from the outer to the inner corner is at most this many widths.
const MAX_MITRE: f64 = 2.2;
/// The stitches turn over this many widths before and after the inner
/// corner (a rung there): they tilt a little at a time and do not all meet
/// at the inner corner.
const TURN: f64 = 0.5;
/// The turn starts at least this distance from the inner corner, in mm, if
/// the column is too short for [`TURN`].
const EDGE_MM: f64 = 0.01;
/// The corner is the part of the column at most this many widths from the
/// corner points: the turn rungs are found there, and only the rungs there
/// can be in the way of the turn (the same column can go past the corner
/// again, as the legs of a buttonhole do).
const REACH: f64 = 3.0;
/// A last segment of a rail of at least this many widths is the line that
/// the rail follows at its end (a drawn column, a buttonhole).
const LONG: f64 = 1.0;
/// Else the line goes through the points of the rail this many widths and
/// [`FAR`] widths from the end, in the body of the stroke. The last part of
/// a rail can be ragged (the ends of font strokes narrow and spread for one
/// to two widths), so it does not count.
const NEAR: f64 = 2.0;
const FAR: f64 = 3.0;
/// The inner edge of a mitre corner stays in the area of the two columns,
/// to this many widths (at least [`FOOTPRINT_MM`]); else the corner stays a
/// lap. So the stitches do not go into the open part of a letter.
const FOOTPRINT: f64 = 0.05;
const FOOTPRINT_MM: f64 = 0.1;
/// The edges are checked at points this far apart, in mm.
const CHECK_STEP_MM: f64 = 0.1;
const EPS: f64 = 1e-6;
const FLATTEN_MM: f64 = 0.05;

/// A column as point lists. Each rung has its first point on rail A.
#[derive(Debug, Clone)]
struct Col {
    a: Vec<Point>,
    b: Vec<Point>,
    rungs: Vec<(Point, Point)>,
}

impl Col {
    /// `None` for a ring or a rail with fewer than two points.
    fn of(g: &SatinGeometry) -> Option<Col> {
        let rail = |p: &Path| -> Option<Vec<Point>> {
            let mut parts = p.flatten(FLATTEN_MM);
            if parts.len() != 1 || parts[0].closed {
                return None;
            }
            let mut pts = parts.remove(0).points;
            pts.dedup();
            (pts.len() >= 2).then_some(pts)
        };
        Some(Col {
            a: rail(&g.rail_a)?,
            b: rail(&g.rail_b)?,
            rungs: g.rungs.clone(),
        })
    }

    fn geometry(self, closed: bool) -> SatinGeometry {
        let path = |p: &[Point]| {
            if closed {
                Path::polygon(p)
            } else {
                p[1..]
                    .iter()
                    .fold(Path::new().move_to(p[0]), |path, &q| path.line_to(q))
            }
        };
        SatinGeometry {
            rail_a: path(&self.a),
            rail_b: path(&self.b),
            rungs: self.rungs,
        }
    }

    fn reversed(mut self) -> Col {
        self.a.reverse();
        self.b.reverse();
        self.rungs.reverse();
        self
    }

    fn swapped(self) -> Col {
        Col {
            a: self.b,
            b: self.a,
            rungs: self.rungs.into_iter().map(|(p, q)| (q, p)).collect(),
        }
    }

    fn last(&self) -> (Point, Point) {
        (self.a[self.a.len() - 1], self.b[self.b.len() - 1])
    }

    fn first(&self) -> (Point, Point) {
        (self.a[0], self.b[0])
    }

    /// The width of the column near an end, past the ragged part.
    fn end_width(&self, at_end: bool) -> f64 {
        let (a, b) = if at_end { self.last() } else { self.first() };
        let w0 = a.distance(b);
        let s = w0.min(0.4 * rail_len(&self.a).min(rail_len(&self.b)));
        w0.max(along(&self.a, s, at_end).distance(along(&self.b, s, at_end)))
    }

    /// The lines that rail A and rail B follow near an end (see [`NEAR`]),
    /// and the direction of the column there, out of the column.
    fn end_lines(&self, at_end: bool) -> Option<EndLines> {
        let w = self.end_width(at_end);
        let a = end_line(&self.a, at_end, w)?;
        let b = end_line(&self.b, at_end, w)?;
        Some(EndLines {
            a,
            b,
            out: (a.1 + b.1).normalized()?,
            width: w,
        })
    }

    /// The area of the column: rail A, then rail B back.
    fn outline(&self) -> Vec<Point> {
        self.a.iter().chain(self.b.iter().rev()).copied().collect()
    }
}

/// The lines (a point and a direction) of the rails near an end of a column.
#[derive(Debug, Clone, Copy)]
struct EndLines {
    a: (Point, Point),
    b: (Point, Point),
    /// The direction of the column at the end, out of the column.
    out: Point,
    width: f64,
}

fn rail_len(rail: &[Point]) -> f64 {
    rail.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point `s` along a rail from its end, or from its start.
fn along(rail: &[Point], s: f64, from_end: bool) -> Point {
    let pts: Vec<Point> = if from_end {
        rail.iter().rev().copied().collect()
    } else {
        rail.to_vec()
    };
    let mut left = s.max(0.0);
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if left <= l && l > 0.0 {
            return w[0].lerp(w[1], left / l);
        }
        left -= l;
    }
    pts[pts.len() - 1]
}

/// The line that a rail follows near its end (or its start), with its
/// direction out of the column (see [`LONG`] and [`NEAR`]). A short rail
/// uses its whole length.
fn end_line(rail: &[Point], from_end: bool, w: f64) -> Option<(Point, Point)> {
    let n = rail.len();
    let (p, q) = if from_end {
        (rail[n - 2], rail[n - 1])
    } else {
        (rail[1], rail[0])
    };
    let last = Some((q, (q - p).normalized()?));
    if p.distance(q) >= LONG * w {
        return last;
    }
    let len = rail_len(rail);
    let (s0, s1) = ((NEAR * w).min(0.5 * len), (FAR * w).min(0.9 * len));
    let (p0, p1) = (along(rail, s0, from_end), along(rail, s1, from_end));
    (p0 - p1).normalized().map(|d| (p0, d)).or(last)
}

/// `true` if each point of `pts` is in one of the areas `outlines`, or
/// nearer than `tol` to its edge.
fn in_footprint(pts: &[Point], outlines: &[Vec<Point>], tol: f64) -> bool {
    let near_edge =
        |p: Point, o: &[Point]| (0..o.len()).any(|i| p.distance_to_segment(o[i], o[(i + 1) % o.len()]) <= tol);
    pts.iter()
        .all(|&p| outlines.iter().any(|o| ring_contains(o, p) || near_edge(p, o)))
}

/// Points along the segments of `pts` that are near the corner (within
/// `reach` of `ma` or `mb`), not more than [`CHECK_STEP_MM`] apart.
fn corner_samples(pts: &[Point], ma: Point, mb: Point, reach: f64) -> Vec<Point> {
    let near = |p: Point| p.distance(ma).min(p.distance(mb)) <= reach;
    let mut out = Vec::new();
    for w in pts.windows(2) {
        if !near(w[0]) && !near(w[1]) {
            continue;
        }
        let n = (w[0].distance(w[1]) / CHECK_STEP_MM).ceil().max(1.0) as usize;
        out.extend((0..=n).map(|k| w[0].lerp(w[1], k as f64 / n as f64)));
    }
    out
}

/// `true` if the inner edge of a mitre corner (the rail `inner` near its
/// corner point `m`) stays in the area of the columns `outlines` (see
/// [`FOOTPRINT`]).
fn stays_inside(inner: &[Point], m: Point, outlines: &[Vec<Point>], w: f64) -> bool {
    in_footprint(&corner_samples(inner, m, m, REACH * w), outlines, tolerance(w))
}

fn tolerance(w: f64) -> f64 {
    (FOOTPRINT * w).max(FOOTPRINT_MM)
}

/// The outer corner point. The outer edges meet at `m`; if `m` is out of
/// the columns `outlines` (a corner that turns more than 90°, as at the
/// top of a "7"), the corner is cut across from `last` (the last outer
/// point before it) to `next` (the first after it), and the point is the
/// middle of the cut. `None` if the cut is out of the columns too.
fn outer_point(m: Point, last: Option<&Point>, next: Option<&Point>, outlines: &[Vec<Point>], w: f64) -> Option<Point> {
    let tol = tolerance(w);
    if in_footprint(&[m], outlines, tol) {
        return Some(m);
    }
    let (p, q) = (*last?, *next?);
    in_footprint(&corner_samples(&[p, q], p, q, f64::INFINITY), outlines, tol).then(|| p.midpoint(q))
}

/// `true` if the corner point `mb` of rail B comes before `ma` along `d1`
/// (the direction of the first column): rail B is on the inside.
fn b_is_inner(ma: Point, mb: Point, d1: Point) -> bool {
    mb.dot(d1) <= ma.dot(d1)
}

/// The point where the line through `p` along `u` meets the line through
/// `q` along `v`.
fn meet(p: Point, u: Point, q: Point, v: Point) -> Option<Point> {
    let den = u.cross(v);
    if den.abs() < 1e-9 {
        return None;
    }
    Some(p + u * ((q - p).cross(v) / den))
}

/// `true` if rail A is on the left of the direction `d`.
fn a_on_left(d: Point, a: Point, b: Point) -> bool {
    d.cross(a - b) < 0.0
}

/// The point of `pts` where the length along `d` is `t`: the first such
/// point from the start, or from the end with `from_end`, not more than
/// `reach` along the rail.
fn at_level(pts: &[Point], d: Point, t: f64, from_end: bool, reach: f64) -> Option<Point> {
    let n = pts.len();
    let mut walked = 0.0;
    let mut seg = |i: usize| -> Option<Option<Point>> {
        if walked > reach {
            return Some(None);
        }
        walked += pts[i].distance(pts[i + 1]);
        let (p, q) = (pts[i], pts[i + 1]);
        let (sp, sq) = (p.dot(d) - t, q.dot(d) - t);
        if (sp <= 0.0 && sq >= 0.0) || (sp >= 0.0 && sq <= 0.0) {
            let den = sp - sq;
            let f = if den.abs() > 1e-12 { sp / den } else { 0.0 };
            Some(Some(p.lerp(q, f)))
        } else {
            None
        }
    };
    if from_end {
        (0..n.saturating_sub(1)).rev().find_map(&mut seg).flatten()
    } else {
        (0..n.saturating_sub(1)).find_map(&mut seg).flatten()
    }
}

/// The two corner points, rail A first: where the line of each rail at
/// the end meets the line of the same rail at the start (see [`NEAR`]).
fn corner_points(end: &EndLines, start: &EndLines) -> Option<(Point, Point)> {
    let (d1, d2) = (end.out, -start.out);
    let cos = d1.dot(d2);
    let (lo, hi) = (
        math::cos(math::to_radians(MAX_TURN_DEG)),
        math::cos(math::to_radians(MIN_TURN_DEG)),
    );
    if !(lo..=hi).contains(&cos) {
        return None;
    }
    let ma = meet(end.a.0, end.a.1, start.a.0, start.a.1)?;
    let mb = meet(end.b.0, end.b.1, start.b.0, start.b.1)?;
    let w = end.width.max(start.width).max(EPS);
    let c = end.a.0.midpoint(end.b.0);
    if ma.distance(mb) > MAX_MITRE * w || ma.distance(c) > 3.0 * w || mb.distance(c) > 3.0 * w {
        return None;
    }
    Some((ma, mb))
}

/// The points of a rail before the corner point `m` (along `d`).
fn before(rail: &[Point], d: Point, m: Point) -> Vec<Point> {
    let t = m.dot(d);
    let mut v = rail.to_vec();
    while v.last().is_some_and(|p| p.dot(d) > t - EPS) {
        v.pop();
    }
    v
}

/// The points of a rail after the corner point `m` (along `d`).
fn after(rail: &[Point], d: Point, m: Point) -> Vec<Point> {
    let t = m.dot(d);
    let k = rail.iter().take_while(|p| p.dot(d) < t + EPS).count();
    rail[k..].to_vec()
}

/// The rung across a column where the length along `d` is `t`, on the
/// rails `a` and `b`.
fn rung_at(a: &[Point], b: &[Point], d: Point, t: f64, from_end: bool, reach: f64) -> Option<(Point, Point)> {
    Some((at_level(a, d, t, from_end, reach)?, at_level(b, d, t, from_end, reach)?))
}

/// The rung where the turn round a corner starts (`by` < 0, before the
/// corner at `t` along `d`) or ends (`by` > 0), and its length along `d`:
/// `by` from the corner, or [`EDGE_MM`] if the rails are too short. The
/// rung is at most `reach` along each rail from its end or its start.
fn turn_rung(
    a: &[Point],
    b: &[Point],
    d: Point,
    t: f64,
    by: f64,
    from_end: bool,
    reach: f64,
) -> (f64, Option<(Point, Point)>) {
    let edge = EDGE_MM.copysign(by);
    for at in [t + by, t + edge] {
        if let Some(r) = rung_at(a, b, d, at, from_end, reach) {
            return (at, Some(r));
        }
    }
    (t + edge, None)
}

/// The end of `first` and the start of `second` as one column with a mitre
/// corner. `None` if they do not make a corner (the turn is less than
/// [`MIN_TURN_DEG`] or more than [`MAX_TURN_DEG`], or the mitre is too long).
pub fn join(first: &SatinGeometry, second: &SatinGeometry) -> Option<SatinGeometry> {
    join_cols(Col::of(first)?, Col::of(second)?).map(|c| c.geometry(false))
}

fn join_cols(first: Col, second: Col) -> Option<Col> {
    let e1 = first.end_lines(true)?;
    let e2 = second.end_lines(false)?;
    let (d1, d2) = (e1.out, -e2.out);
    let second = if a_on_left(d1, e1.a.0, e1.b.0) == a_on_left(d2, e2.a.0, e2.b.0) {
        second
    } else {
        second.swapped()
    };
    let e2 = second.end_lines(false)?;
    let (ma, mb) = corner_points(&e1, &e2)?;
    let w = e1.width.max(e2.width);
    let outlines = [first.outline(), second.outline()];
    let mut a = before(&first.a, d1, ma);
    let mut b = before(&first.b, d1, mb);
    let (a2, b2) = (after(&second.a, d2, ma), after(&second.b, d2, mb));
    let inner_b = b_is_inner(ma, mb, d1);
    let (ma, mb) = if inner_b {
        (outer_point(ma, a.last(), a2.first(), &outlines, w)?, mb)
    } else {
        (ma, outer_point(mb, b.last(), b2.first(), &outlines, w)?)
    };
    a.push(ma);
    b.push(mb);
    // The stitches are straight across up to the corner, then turn round it.
    let (turn, reach) = (TURN * w, REACH * w);
    let near = |p: &Point, q: &Point| [p, q].iter().any(|x| x.distance(ma).min(x.distance(mb)) <= reach);
    let with = |m: Point, rest: &[Point]| std::iter::once(m).chain(rest.iter().copied()).collect::<Vec<_>>();
    let (ra, rb) = (with(ma, &a2), with(mb, &b2));
    let (t1, r1) = turn_rung(&a, &b, d1, ma.dot(d1).min(mb.dot(d1)), -turn, true, reach);
    let (t2, r2) = turn_rung(&ra, &rb, d2, ma.dot(d2).max(mb.dot(d2)), turn, false, reach);
    let mut rungs: Vec<(Point, Point)> = first
        .rungs
        .iter()
        .filter(|(p, q)| !near(p, q) || (p.dot(d1) < t1 - EPS && q.dot(d1) < t1 - EPS))
        .copied()
        .collect();
    rungs.extend(r1);
    rungs.push((ma, mb));
    rungs.extend(r2);
    rungs.extend(
        second
            .rungs
            .iter()
            .filter(|(p, q)| !near(p, q) || (p.dot(d2) > t2 + EPS && q.dot(d2) > t2 + EPS))
            .copied(),
    );
    a.extend(a2);
    b.extend(b2);
    let inside = if inner_b {
        stays_inside(&b, mb, &outlines, w)
    } else {
        stays_inside(&a, ma, &outlines, w)
    };
    (a.len() >= 2 && b.len() >= 2 && inside).then_some(Col { a, b, rungs })
}

/// A column whose end meets its start as a ring with a mitre corner there.
pub fn close(g: &SatinGeometry) -> Option<SatinGeometry> {
    let c = Col::of(g)?;
    let (e1, e2) = (c.end_lines(true)?, c.end_lines(false)?);
    let (d1, d2) = (e1.out, -e2.out);
    if a_on_left(d1, e1.a.0, e1.b.0) != a_on_left(d2, e2.a.0, e2.b.0) {
        return None;
    }
    let (ma, mb) = corner_points(&e1, &e2)?;
    let trim = |rail: &[Point], m: Point| before(&after(rail, d2, m), d1, m);
    let (a, b) = (trim(&c.a, ma), trim(&c.b, mb));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let w = e1.width.max(e2.width);
    let outlines = [c.outline()];
    let inner_b = b_is_inner(ma, mb, d1);
    let (ma, mb) = if inner_b {
        (outer_point(ma, a.last(), a.first(), &outlines, w)?, mb)
    } else {
        (ma, outer_point(mb, b.last(), b.first(), &outlines, w)?)
    };
    // The rails start and end at the corner.
    let ring = |m: Point, mid: &[Point]| -> Vec<Point> {
        std::iter::once(m)
            .chain(mid.iter().copied())
            .chain(std::iter::once(m))
            .collect()
    };
    let (ra, rb) = (ring(ma, &a), ring(mb, &b));
    let (turn, reach) = (TURN * w, REACH * w);
    let near = |p: &Point, q: &Point| [p, q].iter().any(|x| x.distance(ma).min(x.distance(mb)) <= reach);
    let (t1, r1) = turn_rung(&ra, &rb, d1, ma.dot(d1).min(mb.dot(d1)), -turn, true, reach);
    let (t2, r2) = turn_rung(&ra, &rb, d2, ma.dot(d2).max(mb.dot(d2)), turn, false, reach);
    let mut rungs: Vec<(Point, Point)> = r2.into_iter().collect();
    rungs.extend(
        c.rungs
            .iter()
            .filter(|(p, q)| !near(p, q) || (p.dot(d2) > t2 + EPS && q.dot(d2) > t2 + EPS))
            .filter(|(p, q)| !near(p, q) || (p.dot(d1) < t1 - EPS && q.dot(d1) < t1 - EPS))
            .copied(),
    );
    rungs.extend(r1);
    let inside = if inner_b {
        stays_inside(&rb, mb, &outlines, w)
    } else {
        stays_inside(&ra, ma, &outlines, w)
    };
    if !inside {
        return None;
    }
    let mut a = ra;
    let mut b = rb;
    // A closed path ends where it starts.
    a.pop();
    b.pop();
    Some(Col { a, b, rungs }.geometry(true))
}

/// The middle and the width of an end of a column.
fn end_of(c: &Col, at_end: bool) -> (Point, f64) {
    let (a, b) = if at_end { c.last() } else { c.first() };
    (a.midpoint(b), c.end_width(at_end))
}

/// Joins the satin parts of `parts` that meet at corners, with mitre
/// corners. A joined part takes the place and the settings of the part
/// that sews first; a column whose ends then meet becomes a ring. The other
/// parts do not change.
pub fn join_parts(parts: &mut Vec<ObjectKind>) {
    let col = |k: &ObjectKind| match k {
        ObjectKind::Satin { geometry, .. } => Col::of(geometry),
        _ => None,
    };
    let mut tried: Vec<(usize, usize, bool, bool)> = Vec::new();
    loop {
        let cols: Vec<Option<Col>> = parts.iter().map(col).collect();
        // The nearest pair of ends that meet and were not tried.
        let mut best: Option<(f64, usize, usize, bool, bool)> = None;
        for (i, ci) in cols.iter().enumerate() {
            let Some(ci) = ci else { continue };
            for (j, cj) in cols.iter().enumerate().skip(i + 1) {
                let Some(cj) = cj else { continue };
                for ei in [false, true] {
                    for ej in [false, true] {
                        let ((pi, wi), (pj, wj)) = (end_of(ci, ei), end_of(cj, ej));
                        let d = pi.distance(pj);
                        if d <= MEET * wi.max(wj) && !tried.contains(&(i, j, ei, ej)) && best.is_none_or(|b| d < b.0) {
                            best = Some((d, i, j, ei, ej));
                        }
                    }
                }
            }
        }
        let Some((_, i, j, ei, ej)) = best else { break };
        let (Some(ci), Some(cj)) = (cols[i].clone(), cols[j].clone()) else {
            break;
        };
        // The first part ends at the corner and the second starts there.
        let first = if ei { ci } else { ci.reversed() };
        let second = if ej { cj.reversed() } else { cj };
        match join_cols(first, second) {
            Some(joined) => {
                if let ObjectKind::Satin { geometry, .. } = &mut parts[i] {
                    *geometry = joined.geometry(false);
                }
                parts.remove(j);
                tried.clear();
            }
            None => tried.push((i, j, ei, ej)),
        }
    }
    for part in parts.iter_mut() {
        let ObjectKind::Satin { geometry, .. } = part else {
            continue;
        };
        let Some(c) = Col::of(geometry) else { continue };
        let ((p, wp), (q, wq)) = (end_of(&c, false), end_of(&c, true));
        if p.distance(q) <= MEET * wp.max(wq)
            && let Some(ring) = close(geometry)
        {
            *geometry = ring;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::SatinParams;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn column(a0: Point, a1: Point, b0: Point, b1: Point) -> SatinGeometry {
        SatinGeometry {
            rail_a: Path::new().move_to(a0).line_to(a1),
            rail_b: Path::new().move_to(b0).line_to(b1),
            rungs: Vec::new(),
        }
    }

    /// The stem and the foot of an "L", 2 mm wide, each to the outer edge
    /// of the other (a lap).
    fn l_shape() -> (SatinGeometry, SatinGeometry) {
        let stem = column(p(0.0, 0.0), p(0.0, 20.0), p(2.0, 0.0), p(2.0, 20.0));
        let foot = column(p(0.0, 20.0), p(12.0, 20.0), p(0.0, 18.0), p(12.0, 18.0));
        (stem, foot)
    }

    fn rail_points(path: &Path) -> Vec<Point> {
        path.flatten(0.01).remove(0).points
    }

    #[test]
    fn an_l_becomes_one_column_round_the_corner() {
        let (stem, foot) = l_shape();
        let g = join(&stem, &foot).expect("a corner");
        let (a, b) = (rail_points(&g.rail_a), rail_points(&g.rail_b));
        // Rail A goes round the outer corner, rail B through the inner one.
        assert!(a.iter().any(|q| q.distance(p(0.0, 20.0)) < 1e-6), "{a:?}");
        assert!(b.iter().any(|q| q.distance(p(2.0, 18.0)) < 1e-6), "{b:?}");
        assert_eq!(a[0], p(0.0, 0.0));
        assert_eq!(*a.last().unwrap(), p(12.0, 20.0));
        // No rail point goes past the inner corner.
        assert!(b.iter().all(|q| q.y <= 18.0 + 1e-6 || q.x >= 2.0 - 1e-6), "{b:?}");
        // The mitre rung goes from the outer to the inner corner.
        assert!(
            g.rungs
                .iter()
                .any(|(x, y)| x.distance(p(0.0, 20.0)) < 1e-6 && y.distance(p(2.0, 18.0)) < 1e-6)
        );
    }

    #[test]
    fn a_rail_on_the_other_side_and_a_reversed_column_join_the_same() {
        let (stem, foot) = l_shape();
        let flipped = SatinGeometry {
            rail_a: foot.rail_b.clone(),
            rail_b: foot.rail_a.clone(),
            rungs: Vec::new(),
        };
        let a = join(&stem, &foot).unwrap();
        let b = join(&stem, &flipped).unwrap();
        assert_eq!(rail_points(&a.rail_a), rail_points(&b.rail_a));
        let mut parts = vec![
            ObjectKind::Satin {
                geometry: foot,
                params: SatinParams::default(),
            },
            ObjectKind::Satin {
                geometry: stem,
                params: SatinParams::default(),
            },
        ];
        join_parts(&mut parts);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn a_sharp_corner_and_a_straight_joint_stay_apart() {
        // The legs of an "A": a turn of about 140°.
        let left = column(p(0.0, 20.0), p(7.0, 0.0), p(2.0, 20.0), p(9.0, 0.0));
        let right = column(p(7.0, 0.0), p(14.0, 20.0), p(9.0, 0.0), p(16.0, 20.0));
        assert!(join(&left, &right).is_none());
        // Two parts in a straight line: no corner.
        let one = column(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 2.0), p(10.0, 2.0));
        let two = column(p(10.0, 0.0), p(20.0, 0.0), p(10.0, 2.0), p(20.0, 2.0));
        assert!(join(&one, &two).is_none());
    }

    #[test]
    fn a_t_is_not_joined() {
        // The stem ends inside the bar, far from the ends of the bar.
        let bar = column(p(0.0, 0.0), p(20.0, 0.0), p(0.0, 2.0), p(20.0, 2.0));
        let stem = column(p(9.0, 1.5), p(9.0, 20.0), p(11.0, 1.5), p(11.0, 20.0));
        let mut parts: Vec<ObjectKind> = [bar, stem]
            .into_iter()
            .map(|geometry| ObjectKind::Satin {
                geometry,
                params: SatinParams::default(),
            })
            .collect();
        join_parts(&mut parts);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn four_sides_become_a_ring() {
        let w = 2.0;
        let sides = [
            column(p(0.0, 0.0), p(20.0, 0.0), p(0.0, w), p(20.0, w)),
            column(p(20.0, 0.0), p(20.0, 20.0), p(20.0 - w, 0.0), p(20.0 - w, 20.0)),
            column(p(20.0, 20.0), p(0.0, 20.0), p(20.0, 20.0 - w), p(0.0, 20.0 - w)),
            column(p(0.0, 20.0), p(0.0, 0.0), p(w, 20.0), p(w, 0.0)),
        ];
        let mut parts: Vec<ObjectKind> = sides
            .into_iter()
            .map(|geometry| ObjectKind::Satin {
                geometry,
                params: SatinParams::default(),
            })
            .collect();
        join_parts(&mut parts);
        assert_eq!(parts.len(), 1);
        let ObjectKind::Satin { geometry, .. } = &parts[0] else {
            unreachable!()
        };
        let a = geometry.rail_a.flatten(0.01).remove(0);
        assert!(a.closed, "a ring");
        // The outer rail has the four outer corners.
        for c in [p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0), p(0.0, 20.0)] {
            let near = a.points.iter().any(|q| q.distance(c) < 1e-6);
            let b = rail_points(&geometry.rail_b);
            assert!(near || b.iter().any(|q| q.distance(c) < 1e-6), "{c:?}");
        }
    }

    fn polyline(pts: &[Point]) -> Path {
        pts[1..]
            .iter()
            .fold(Path::new().move_to(pts[0]), |path, &q| path.line_to(q))
    }

    #[test]
    fn a_ragged_end_does_not_move_the_inner_corner() {
        // The top bar of an "E" from a font: its rails narrow at the left end.
        let bar = SatinGeometry {
            rail_a: polyline(&[
                p(20.0, 0.0),
                p(16.0, 0.0),
                p(12.0, 0.0),
                p(8.0, 0.0),
                p(4.0, 0.0),
                p(1.0, 0.1),
                p(0.0, 0.15),
            ]),
            rail_b: polyline(&[
                p(20.0, 3.0),
                p(16.0, 3.0),
                p(12.0, 3.0),
                p(8.0, 3.0),
                p(5.0, 3.05),
                p(4.0, 2.6),
                p(3.0, 2.5),
                p(0.0, 4.5),
            ]),
            rungs: Vec::new(),
        };
        // The stem, from inside the bar down, with points 1 mm apart.
        let stem = SatinGeometry {
            rail_a: polyline(&(0..=28).map(|k| p(0.0, 2.5 + k as f64)).collect::<Vec<_>>()),
            rail_b: polyline(&(0..=28).map(|k| p(3.5, 2.5 + k as f64)).collect::<Vec<_>>()),
            rungs: Vec::new(),
        };
        let g = join(&bar, &stem).expect("a corner");
        let b = rail_points(&g.rail_b);
        assert!(
            b.iter().any(|q| q.distance(p(3.5, 3.0)) < 0.15),
            "the inner corner: {b:?}"
        );
        // No point of the inner edge is in the open part under the bar.
        assert!(b.iter().all(|q| !(q.x > 3.65 && q.y > 3.15)), "{b:?}");
        let a = rail_points(&g.rail_a);
        assert!(
            a.iter().any(|q| q.distance(p(0.0, 0.0)) < 0.2),
            "the outer corner: {a:?}"
        );
    }

    #[test]
    fn a_corner_of_more_than_90_degrees_is_cut_inside_the_columns() {
        // The bar and the stroke of a "7": a turn of about 117°. The outer
        // edges meet past the end of the bar, out of both columns.
        let bar = column(p(0.0, 0.0), p(20.0, 0.0), p(0.0, 3.0), p(20.0, 3.0));
        let u = Point::new(-0.45, 0.893).normalized().unwrap();
        let n = Point::new(-u.y, u.x) * -1.5;
        let (c0, c1) = (p(18.5, 1.0), p(18.5, 1.0) + u * 20.0);
        let stroke = column(c0 + n, c1 + n, c0 - n, c1 - n);
        let g = join(&bar, &stroke).expect("a corner");
        let a = rail_points(&g.rail_a);
        assert!(
            a.iter().all(|q| !(q.x > 20.05 && q.y < 0.5)),
            "no point past the bar: {a:?}"
        );
        let outlines = [Col::of(&bar).unwrap().outline(), Col::of(&stroke).unwrap().outline()];
        let pts = corner_samples(&a, p(20.0, 0.0), p(20.0, 0.0), 4.0);
        assert!(in_footprint(&pts, &outlines, tolerance(3.0)), "{a:?}");
    }
}
