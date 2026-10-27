//! Path simplification: fewer nodes for the same shape.
//!
//! The path is flattened, cut at its corners, and each part between two
//! corners gets the fewest cubic curves that stay within the tolerance.
//! The curve fit is the least-squares method with Newton reparameterization
//! that Philip J. Schneider published in "An Algorithm for Automatically
//! Fitting Digitized Curves" (Graphics Gems, 1990); this is our own
//! implementation of the published method.
//!
//! A result with more nodes than the original is not used: simplification
//! never adds nodes.

use crate::path::simplify as douglas_peucker;
use crate::{Path, PathCmd, Point, math};

/// Settings of [`simplified`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Simplify {
    /// The largest distance between the old and the new path, in mm.
    pub tolerance_mm: f64,
    /// A turn larger than this, in degrees, stays a sharp corner.
    pub corner_deg: f64,
    /// Curves between corners; `false` gives straight lines only.
    pub curves: bool,
}

impl Simplify {
    /// The settings of Quick simplify.
    pub const QUICK: Simplify = Simplify {
        tolerance_mm: 0.2,
        corner_deg: 40.0,
        curves: true,
    };
}

impl Default for Simplify {
    fn default() -> Self {
        Simplify::QUICK
    }
}

/// The number of nodes of a path (the end points of its segments).
pub fn node_count(path: &Path) -> usize {
    path.cmds.iter().filter(|c| !matches!(c, PathCmd::Close)).count()
}

/// A simpler path for `path`, or a copy of it if the result would have as
/// many nodes or more.
pub fn simplified(path: &Path, s: &Simplify) -> Path {
    let tol = s.tolerance_mm.max(0.001);
    let mut out = Path::new();
    for pl in path.flatten((tol / 10.0).min(0.02)) {
        let mut pts = pl.points.clone();
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if pl.closed && pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) < 1e-9 {
            pts.pop();
        }
        if pts.len() < 2 || (pl.closed && pts.len() < 3) {
            continue;
        }
        let sub = simplify_polyline(&pts, pl.closed, tol, s);
        out.cmds.extend(sub.cmds);
    }
    if out.cmds.is_empty() || node_count(&out) >= node_count(path) {
        return path.clone();
    }
    out
}

/// The turn at `b` from the direction `a`→`b` to `b`→`c`, in degrees.
fn turn_deg(a: Point, b: Point, c: Point) -> f64 {
    let (u, v) = (b - a, c - b);
    math::to_degrees(math::atan2(u.cross(v), u.dot(v))).abs()
}

fn simplify_polyline(pts: &[Point], closed: bool, tol: f64, s: &Simplify) -> Path {
    // Douglas-Peucker points (as indexes) find the corners.
    let ring: Vec<Point> = if closed {
        pts.iter().chain(std::iter::once(&pts[0])).copied().collect()
    } else {
        pts.to_vec()
    };
    let kept = keep_indexes(&ring, tol, closed);
    let n_kept = kept.len();
    let mut corners: Vec<usize> = Vec::new();
    for k in 0..n_kept {
        let i = kept[k];
        let (prev, next) = if closed {
            // `kept` ends with the closing index (= pts.len()); skip it.
            if k == n_kept - 1 {
                continue;
            }
            let prev = if k == 0 { kept[n_kept - 2] } else { kept[k - 1] };
            (prev, kept[k + 1])
        } else {
            if k == 0 || k == n_kept - 1 {
                corners.push(i);
                continue;
            }
            (kept[k - 1], kept[k + 1])
        };
        if !s.curves || turn_deg(ring[prev], ring[i], ring[next]) > s.corner_deg {
            corners.push(i);
        }
    }
    let n = pts.len();
    if !s.curves {
        let pts: Vec<(Point, bool)> = kept.iter().filter(|&&i| i < n).map(|&i| (pts[i], false)).collect();
        return Path::through_points(&pts, closed);
    }
    let at = |i: usize| pts[i % n];
    let mut path = Path::new();
    if closed && corners.is_empty() {
        // A smooth loop: one cut at the first point, with the same tangent on both sides.
        let t = (at(1) - at(n - 1)).normalized().unwrap_or(Point::new(1.0, 0.0));
        let seq: Vec<Point> = (0..=n).map(at).collect();
        path = path.move_to(seq[0]);
        fit(&seq, t, -t, tol, &mut path);
        return path.close();
    }
    // The parts between corners. For a closed path, the last part goes
    // round to the first corner.
    let mut cuts = corners.clone();
    if closed {
        cuts.push(corners[0] + n);
    }
    path = path.move_to(at(cuts[0]));
    for w in cuts.windows(2) {
        let seq: Vec<Point> = (w[0]..=w[1]).map(at).collect();
        let t1 = end_tangent(&seq, tol, false);
        let t2 = end_tangent(&seq, tol, true);
        fit(&seq, t1, t2, tol, &mut path);
    }
    if closed {
        path = path.close();
    }
    path
}

/// The indexes that Douglas-Peucker keeps. A closed ring (first point
/// repeated at the end) is cut at the point farthest from its start first.
fn keep_indexes(ring: &[Point], tol: f64, closed: bool) -> Vec<usize> {
    let index_of = |part: &[Point], offset: usize| -> Vec<usize> {
        let simple = douglas_peucker(part, tol);
        let mut out = Vec::new();
        let mut j = 0;
        for p in simple {
            while part[j] != p {
                j += 1;
            }
            out.push(offset + j);
        }
        out
    };
    if !closed {
        return index_of(ring, 0);
    }
    let far = (1..ring.len() - 1)
        .max_by(|&a, &b| ring[0].distance(ring[a]).total_cmp(&ring[0].distance(ring[b])))
        .unwrap_or(1);
    let mut a = index_of(&ring[..=far], 0);
    let b = index_of(&ring[far..], far);
    a.pop();
    a.extend(b);
    a
}

/// The direction at the start (or, with `end`, back from the end) of `seq`,
/// from the points within a few tolerances of it.
fn end_tangent(seq: &[Point], tol: f64, end: bool) -> Point {
    let n = seq.len();
    let p = |i: usize| if end { seq[n - 1 - i] } else { seq[i] };
    let reach = (3.0 * tol).max(0.05);
    let mut q = p(1);
    for i in 1..n {
        q = p(i);
        if q.distance(p(0)) >= reach {
            break;
        }
    }
    (q - p(0)).normalized().unwrap_or(Point::new(1.0, 0.0))
}

/// Adds the fewest cubic curves that stay within `tol` of the points `seq`
/// to `path` (its pen is at `seq[0]`). The end points stay; the end
/// tangents come from the points near each end. Auto-trace (`emb_art`)
/// uses this for the parts of an edge between two corners.
pub fn fit_points(seq: &[Point], tol: f64, path: &mut Path) {
    if seq.len() < 2 {
        return;
    }
    let t1 = end_tangent(seq, tol, false);
    let t2 = end_tangent(seq, tol, true);
    fit(seq, t1, t2, tol, path);
}

/// As [`fit_points`] for a smooth loop: `seq` ends with its first point,
/// and the curve has the same tangent on both sides of it.
pub fn fit_loop(seq: &[Point], tol: f64, path: &mut Path) {
    let n = seq.len();
    if n < 3 {
        return;
    }
    let t = (seq[1] - seq[n - 2]).normalized().unwrap_or(Point::new(1.0, 0.0));
    fit(seq, t, -t, tol, path);
}

/// Adds cubic curves through `seq` (its first point is the pen) with the
/// unit tangents `t1` at the start and `t2` back from the end. A part that
/// a curve does not fit is cut in two at its worst point. The parts wait on
/// a list, not on the call stack: a long, rough edge can need thousands of
/// cuts (ADR 0077).
fn fit(seq: &[Point], t1: Point, t2: Point, tol: f64, path: &mut Path) {
    // Parts (first and last index, tangents), the next part last.
    let mut todo = vec![(0, seq.len() - 1, t1, t2)];
    while let Some((i, j, t1, t2)) = todo.pop() {
        if let Some((split, tc)) = fit_part(&seq[i..=j], t1, t2, tol, path) {
            todo.push((i + split, j, -tc, t2));
            todo.push((i, i + split, t1, tc));
        }
    }
}

/// Adds one cubic curve through `seq` if it fits; else the index to cut
/// `seq` at and the tangent there.
fn fit_part(seq: &[Point], t1: Point, t2: Point, tol: f64, path: &mut Path) -> Option<(usize, Point)> {
    let n = seq.len();
    let (a, b) = (seq[0], seq[n - 1]);
    if n == 2 {
        let d = a.distance(b) / 3.0;
        push_cubic(path, [a, a + t1 * d, b + t2 * d, b]);
        return None;
    }
    let mut u = chord_params(seq);
    let mut bez = bezier(seq, &u, t1, t2);
    let (mut err, mut split) = max_error(seq, &bez, &u);
    if err <= tol {
        push_cubic(path, bez);
        return None;
    }
    if err <= 4.0 * tol {
        for _ in 0..4 {
            reparameterize(seq, &bez, &mut u);
            bez = bezier(seq, &u, t1, t2);
            (err, split) = max_error(seq, &bez, &u);
            if err <= tol {
                push_cubic(path, bez);
                return None;
            }
        }
    }
    let split = split.clamp(1, n - 2);
    let tc = (seq[split - 1] - seq[split + 1])
        .normalized()
        .unwrap_or_else(|| (seq[split - 1] - seq[split]).normalized().unwrap_or(t2));
    Some((split, tc))
}

fn push_cubic(path: &mut Path, [_, c1, c2, p]: [Point; 4]) {
    path.cmds.push(PathCmd::CubicTo(c1, c2, p));
}

/// Parameters from the lengths along the points, 0 to 1.
fn chord_params(seq: &[Point]) -> Vec<f64> {
    let mut u = vec![0.0; seq.len()];
    for i in 1..seq.len() {
        u[i] = u[i - 1] + seq[i].distance(seq[i - 1]);
    }
    let total = u[seq.len() - 1];
    if total > 0.0 {
        u.iter_mut().for_each(|x| *x /= total);
    }
    u
}

fn eval(b: &[Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    b[0] * (s * s * s) + b[1] * (3.0 * s * s * t) + b[2] * (3.0 * s * t * t) + b[3] * (t * t * t)
}

/// The least-squares cubic with the end points of `seq` and the tangents.
fn bezier(seq: &[Point], u: &[f64], t1: Point, t2: Point) -> [Point; 4] {
    let (a, b) = (seq[0], seq[seq.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, &t) in seq.iter().zip(u) {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        let (a1, a2) = (t1 * b1, t2 * b2);
        c00 += a1.dot(a1);
        c01 += a1.dot(a2);
        c11 += a2.dot(a2);
        let tmp = *p - (a * (b0 + b1) + b * (b2 + b3));
        x0 += a1.dot(tmp);
        x1 += a2.dot(tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let seg = a.distance(b);
    let eps = 1e-6 * seg;
    let (mut al, mut ar) = if det.abs() > 1e-12 {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (0.0, 0.0)
    };
    if al < eps || ar < eps {
        al = seg / 3.0;
        ar = seg / 3.0;
    }
    [a, a + t1 * al, b + t2 * ar, b]
}

/// The largest distance from the points to the curve, and where.
fn max_error(seq: &[Point], b: &[Point; 4], u: &[f64]) -> (f64, usize) {
    let mut best = (0.0, seq.len() / 2);
    for i in 1..seq.len() - 1 {
        let d = eval(b, u[i]).distance(seq[i]);
        if d > best.0 {
            best = (d, i);
        }
    }
    best
}

/// One Newton step for each parameter: the nearest point on the curve.
fn reparameterize(seq: &[Point], b: &[Point; 4], u: &mut [f64]) {
    let d1 = [(b[1] - b[0]) * 3.0, (b[2] - b[1]) * 3.0, (b[3] - b[2]) * 3.0];
    let d2 = [(d1[1] - d1[0]) * 2.0, (d1[2] - d1[1]) * 2.0];
    for (p, t) in seq.iter().zip(u.iter_mut()) {
        let s = 1.0 - *t;
        let q = eval(b, *t);
        let q1 = d1[0] * (s * s) + d1[1] * (2.0 * s * *t) + d1[2] * (*t * *t);
        let q2 = d2[0] * s + d2[1] * *t;
        let num = (q - *p).dot(q1);
        let den = q1.dot(q1) + (q - *p).dot(q2);
        if den.abs() > 1e-12 {
            *t = (*t - num / den).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The largest distance from the points of `a` to the polyline of `b`.
    fn deviation(a: &Path, b: &Path) -> f64 {
        let pb = b.flatten(0.005);
        a.flatten(0.005)
            .iter()
            .flat_map(|pl| pl.points.iter().copied())
            .map(|p| {
                pb.iter()
                    .flat_map(|pl| {
                        let n = pl.points.len();
                        let m = if pl.closed { n } else { n - 1 };
                        (0..m).map(move |i| p.distance_to_segment(pl.points[i], pl.points[(i + 1) % n]))
                    })
                    .fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_noisy_circle_becomes_a_few_curves() {
        let pts: Vec<Point> = (0..200)
            .map(|i| {
                let a = i as f64 / 200.0 * std::f64::consts::TAU;
                let r = 20.0 + 0.03 * math::sin(37.0 * a);
                Point::new(r * math::cos(a), r * math::sin(a))
            })
            .collect();
        let path = Path::polygon(&pts);
        let s = simplified(&path, &Simplify::QUICK);
        assert!(node_count(&s) <= 12, "{}", node_count(&s));
        assert!(matches!(s.cmds.last(), Some(PathCmd::Close)));
        let d = deviation(&path, &s).max(deviation(&s, &path));
        assert!(d < 0.25, "{d}");
    }

    #[test]
    fn corners_stay_sharp_and_lines_straight() {
        // A square with extra points on its sides.
        let mut pts = Vec::new();
        for (a, b) in [
            ((0.0, 0.0), (10.0, 0.0)),
            ((10.0, 0.0), (10.0, 10.0)),
            ((10.0, 10.0), (0.0, 10.0)),
            ((0.0, 10.0), (0.0, 0.0)),
        ] {
            for k in 0..5 {
                let t = k as f64 / 5.0;
                pts.push(Point::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        let path = Path::polygon(&pts);
        for curves in [true, false] {
            let s = simplified(
                &path,
                &Simplify {
                    curves,
                    ..Simplify::QUICK
                },
            );
            assert_eq!(node_count(&s), 5, "move and four sides: {s:?}");
            assert!(deviation(&path, &s) < 1e-6 && deviation(&s, &path) < 1e-6);
        }
    }

    #[test]
    fn open_paths_keep_their_ends_and_nothing_gets_more_nodes() {
        let pts: Vec<Point> = (0..=100)
            .map(|i| {
                let x = i as f64 * 0.5;
                Point::new(x, 5.0 * math::sin(x / 8.0))
            })
            .collect();
        let mut path = Path::new().move_to(pts[0]);
        for p in &pts[1..] {
            path = path.line_to(*p);
        }
        let s = simplified(&path, &Simplify::QUICK);
        assert!(node_count(&s) < 10, "{}", node_count(&s));
        let first = s.flatten(0.01)[0].points.clone();
        assert_eq!(first[0], pts[0]);
        assert_eq!(*first.last().unwrap(), pts[100]);
        assert!(deviation(&path, &s) < 0.25);
        // A clean ellipse (four curves) is already simple.
        let e = Path::ellipse(Point::ZERO, 20.0, 10.0);
        assert_eq!(simplified(&e, &Simplify::QUICK), e);
        // Larger tolerance: fewer or as many nodes.
        let loose = simplified(
            &path,
            &Simplify {
                tolerance_mm: 1.0,
                ..Simplify::QUICK
            },
        );
        assert!(node_count(&loose) <= node_count(&s));
    }

    #[test]
    fn a_long_rough_edge_fits_on_a_small_stack() {
        // A zigzag that gets larger to its end: the worst point is near the
        // end each time, so each cut takes off only a few points.
        let seq: Vec<Point> = (0..5000)
            .map(|i| {
                Point::new(
                    f64::from(i) * 0.1,
                    if i % 2 == 0 { 1.0 } else { -1.0 } * f64::from(i) / 100.0,
                )
            })
            .collect();
        let path = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || {
                let mut path = Path::new().move_to(seq[0]);
                fit_points(&seq, 0.01, &mut path);
                (path, seq)
            })
            .unwrap()
            .join();
        let (path, seq) = path.expect("no stack overflow");
        // The curves go through each point, in order.
        let ends: Vec<Point> = path
            .cmds
            .iter()
            .filter_map(|c| match c {
                PathCmd::CubicTo(_, _, p) => Some(*p),
                _ => None,
            })
            .collect();
        assert_eq!(ends.last(), seq.last());
        assert!(ends.windows(2).all(|w| w[1].x > w[0].x));
    }
}
