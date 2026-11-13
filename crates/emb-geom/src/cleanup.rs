//! Clean up: smooth lines and outlines that have small faults, as from a
//! scan, a trace or an SVG file (owner request, 2026-11-13).
//!
//! Each subpath is flattened, then:
//!
//! 1. Small loops, overlaps and spikes. Where the line crosses itself, or
//!    goes out and comes back to less than half the detail size from where
//!    it went out, and the part between is shorter than four detail sizes,
//!    that part goes. So the tip of a sharp point (less than about 45°)
//!    that is narrower than half the detail size goes too.
//! 2. Zig-zags. The line is smoothed over the detail size. Corners (turns
//!    of more than 55° over two detail sizes) stay sharp.
//! 3. Tiny parts. Open subpaths shorter than the detail size, and closed
//!    subpaths with less area than the square of the detail size, go.
//!
//! Last, curves go through the points (`simplify`, with a tolerance of a
//! quarter of the detail size), and a curve that is straight becomes a line.

use crate::simplify::{Simplify, simplified, turn_deg};
use crate::{Path, PathCmd, Point, region};

/// Settings of [`cleaned`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cleanup {
    /// Faults smaller than this go, in mm.
    pub detail_mm: f64,
    /// Remove small loops and spikes.
    pub loops: bool,
    /// Smooth zig-zags.
    pub smooth: bool,
    /// Remove tiny parts.
    pub tiny: bool,
}

impl Cleanup {
    /// The settings when the window opens.
    pub const STANDARD: Cleanup = Cleanup {
        detail_mm: 0.5,
        loops: true,
        smooth: true,
        tiny: true,
    };

    fn size(&self) -> f64 {
        self.detail_mm.max(0.01)
    }

    /// The tolerance of the curves.
    fn tolerance(&self) -> f64 {
        (self.size() / 4.0).clamp(0.02, 0.5)
    }
}

impl Default for Cleanup {
    fn default() -> Self {
        Cleanup::STANDARD
    }
}

/// Removes the small loops and spikes of a point sequence (see the module
/// comment). The first and the last point stay.
fn remove_loops(pts: &mut Vec<Point>, size: f64) {
    let (window, near) = (4.0 * size, size / 2.0);
    let mut i = 0;
    while i + 2 < pts.len() {
        let n = pts.len();
        let mut arc = pts[i].distance(pts[i + 1]);
        // The end of the part that goes, and the point in its place.
        let mut cut: Option<(usize, Option<Point>)> = None;
        let mut j = i + 2;
        while j < pts.len() {
            arc += pts[j - 1].distance(pts[j]);
            if arc > window {
                break;
            }
            let d = pts[i].distance(pts[j]);
            // Never all of the line.
            if d <= near && arc > 2.5 * d && (i, j) != (0, n - 1) {
                // A spike or a loop that comes back near: the farthest one.
                cut = Some((j, None));
            }
            // The last line of a closed ring ends where the first one starts.
            if j + 1 < n
                && (i, j + 1) != (0, n - 1)
                && let Some((u, _)) = Point::crossing(pts[i], pts[i + 1], pts[j], pts[j + 1])
            {
                // The line crosses itself: the first crossing.
                cut = Some((j, Some(pts[i].lerp(pts[i + 1], u))));
                break;
            }
            j += 1;
        }
        match cut {
            Some((j, Some(x))) => {
                pts.splice(i + 1..=j, [x]);
            }
            Some((j, None)) if j > i + 1 => {
                pts.drain(i + 1..j);
            }
            _ => i += 1,
        }
    }
    pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
}

/// Points along `pts` at `step` from each other; the end points stay.
fn resample(pts: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![pts[0]];
    let mut left = step;
    for w in pts.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        let mut seg = a.distance(b);
        while seg >= left {
            a = a.lerp(b, left / seg);
            out.push(a);
            seg -= left;
            left = step;
        }
        left -= seg;
    }
    let last = pts[pts.len() - 1];
    if out.last().is_some_and(|p| p.distance(last) < step / 2.0) && out.len() > 1 {
        out.pop();
    }
    out.push(last);
    out
}

/// Three passes of a moving average over the points `run`; the end points
/// stay, and the window gets smaller near them.
fn smooth_run(run: &mut [Point]) {
    const HALF: usize = 2;
    let n = run.len();
    for _ in 0..3 {
        let old = run.to_vec();
        for (i, p) in run.iter_mut().enumerate().take(n.saturating_sub(1)).skip(1) {
            let m = HALF.min(i).min(n - 1 - i);
            let sum = old[i - m..=i + m].iter().fold(Point::ZERO, |s, q| s + *q);
            *p = sum * (1.0 / (2 * m + 1) as f64);
        }
    }
}

/// As [`smooth_run`] for a closed ring with no corners.
fn smooth_ring(ring: &mut [Point]) {
    const HALF: usize = 2;
    let n = ring.len();
    if n < 2 * HALF + 2 {
        return;
    }
    for _ in 0..3 {
        let old = ring.to_vec();
        for (i, p) in ring.iter_mut().enumerate() {
            let sum = (0..=2 * HALF).fold(Point::ZERO, |s, k| s + old[(i + n + k - HALF) % n]);
            *p = sum * (1.0 / (2 * HALF + 1) as f64);
        }
    }
}

/// The corners of resampled points: turns of more than 55° over `k`
/// points on each side. Of the corners near each other, the sharpest stays.
fn corners(pts: &[Point], closed: bool, k: usize) -> Vec<usize> {
    let n = pts.len();
    if k == 0 || n < 2 * k + 1 {
        return Vec::new();
    }
    let idx = |i: isize| -> usize {
        if closed {
            i.rem_euclid(n as isize) as usize
        } else {
            i.clamp(0, n as isize - 1) as usize
        }
    };
    let turns: Vec<f64> = (0..n)
        .map(|i| {
            if !closed && (i == 0 || i == n - 1) {
                return 0.0;
            }
            let (a, c) = (pts[idx(i as isize - k as isize)], pts[idx(i as isize + k as isize)]);
            if a == pts[i] || c == pts[i] {
                0.0
            } else {
                turn_deg(a, pts[i], c)
            }
        })
        .collect();
    (0..n)
        .filter(|&i| {
            turns[i] > 55.0
                && (1..=k as isize).all(|d| {
                    let (a, b) = (idx(i as isize - d), idx(i as isize + d));
                    // Equal turns: the first point is the corner.
                    (a == i || turns[i] > turns[a]) && (b == i || turns[i] >= turns[b])
                })
        })
        .collect()
}

/// Points at `step` from each other along `pts`. The end points and the
/// points where the line turns more than 30° stay.
fn resample_keep(pts: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![pts[0]];
    let mut piece = vec![pts[0]];
    for i in 1..pts.len() {
        piece.push(pts[i]);
        let sharp = i + 1 < pts.len() && turn_deg(pts[i - 1], pts[i], pts[i + 1]) > 30.0;
        if sharp || i + 1 == pts.len() {
            out.extend(resample(&piece, step).into_iter().skip(1));
            piece = vec![pts[i]];
        }
    }
    out
}

/// Smooths the zig-zags of `pts` over the detail size; corners stay.
fn smooth(pts: &[Point], closed: bool, size: f64) -> Vec<Point> {
    let mut seq = pts.to_vec();
    if closed {
        seq.push(pts[0]);
    }
    let mut r = resample_keep(&seq, size / 4.0);
    if closed {
        r.pop();
    }
    let n = r.len();
    if n < 3 {
        return pts.to_vec();
    }
    // Two detail sizes on each side.
    let cs = corners(&r, closed, 8);
    if !closed {
        let cuts: Vec<usize> = std::iter::once(0).chain(cs).chain([n - 1]).collect();
        for w in cuts.windows(2) {
            smooth_run(&mut r[w[0]..=w[1]]);
        }
        return r;
    }
    if cs.is_empty() {
        smooth_ring(&mut r);
        return r;
    }
    r.rotate_left(cs[0]);
    let cuts: Vec<usize> = cs.iter().map(|c| c - cs[0]).chain([n]).collect();
    r.push(r[0]);
    for w in cuts.windows(2) {
        smooth_run(&mut r[w[0]..=w[1]]);
    }
    r.pop();
    r
}

fn is_tiny(pts: &[Point], closed: bool, size: f64) -> bool {
    if closed {
        region::ring_signed_area(pts).abs() < size * size
    } else {
        pts.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() < size
    }
}

/// The cleaned points of each subpath of `path` (see the module comment),
/// with `true` for a closed subpath. A closed subpath does not repeat its
/// first point.
pub fn clean_polylines(path: &Path, c: &Cleanup) -> Vec<(Vec<Point>, bool)> {
    let size = c.size();
    let mut out = Vec::new();
    for pl in path.flatten((size / 20.0).clamp(0.005, 0.02)) {
        let closed = pl.closed;
        let mut pts = pl.points;
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if closed && pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) < 1e-9 {
            pts.pop();
        }
        if pts.len() < 2 {
            continue;
        }
        if closed {
            // Start at the point farthest from the middle: no small loop
            // goes round the start.
            let mid = pts.iter().fold(Point::ZERO, |s, p| s + *p) * (1.0 / pts.len() as f64);
            let far = (0..pts.len())
                .max_by(|&a, &b| pts[a].distance(mid).total_cmp(&pts[b].distance(mid)))
                .unwrap_or(0);
            pts.rotate_left(far);
        }
        if c.loops {
            if closed {
                pts.push(pts[0]);
            }
            remove_loops(&mut pts, size);
            if closed {
                pts.pop();
            }
        }
        if c.smooth && pts.len() >= 3 {
            pts = smooth(&pts, closed, size);
        }
        if pts.len() < 2 || (closed && pts.len() < 3) || (c.tiny && is_tiny(&pts, closed, size)) {
            continue;
        }
        out.push((pts, closed));
    }
    out
}

/// A cubic whose handles are on the line between its ends becomes a line.
fn straighten(path: &Path, eps: f64) -> Path {
    let mut out = Path::new();
    let (mut pen, mut start) = (Point::ZERO, Point::ZERO);
    for cmd in &path.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => {
                (pen, start) = (p, p);
                out.cmds.push(*cmd);
            }
            PathCmd::LineTo(p) => {
                pen = p;
                out.cmds.push(*cmd);
            }
            PathCmd::CubicTo(c1, c2, p) => {
                if c1.distance_to_segment(pen, p) <= eps && c2.distance_to_segment(pen, p) <= eps {
                    out.cmds.push(PathCmd::LineTo(p));
                } else {
                    out.cmds.push(*cmd);
                }
                pen = p;
            }
            PathCmd::Close => {
                pen = start;
                out.cmds.push(*cmd);
            }
        }
    }
    out
}

/// Curves through the cleaned points of one subpath.
pub fn curves(pts: &[Point], closed: bool, c: &Cleanup) -> Path {
    let raw = if closed {
        Path::polygon(pts)
    } else {
        pts[1..].iter().fold(Path::new().move_to(pts[0]), |p, q| p.line_to(*q))
    };
    let s = Simplify {
        tolerance_mm: c.tolerance(),
        corner_deg: 40.0,
        curves: true,
    };
    straighten(&simplified(&raw, &s), c.tolerance() / 10.0)
}

/// The cleaned path, or `None` if no part of it stays.
pub fn cleaned(path: &Path, c: &Cleanup) -> Option<Path> {
    let parts = clean_polylines(path, c);
    if parts.is_empty() {
        return None;
    }
    let mut out = Path::new();
    for (pts, closed) in &parts {
        out.cmds.extend(curves(pts, *closed, c).cmds);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn open(pts: &[Point]) -> Path {
        pts[1..].iter().fold(Path::new().move_to(pts[0]), |a, q| a.line_to(*q))
    }

    /// The largest distance from the points of `a` to the lines of `b`.
    fn deviation(a: &Path, b: &Path) -> f64 {
        let pb = b.flatten(0.005);
        a.flatten(0.005)
            .iter()
            .flat_map(|pl| pl.points.iter().copied())
            .map(|q| {
                pb.iter()
                    .flat_map(|pl| {
                        let n = pl.points.len();
                        let m = if pl.closed { n } else { n - 1 };
                        (0..m).map(move |i| q.distance_to_segment(pl.points[i], pl.points[(i + 1) % n]))
                    })
                    .fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_zigzag_line_becomes_straight() {
        let pts: Vec<Point> = (0..=200)
            .map(|i| p(f64::from(i) * 0.2, if i % 2 == 0 { 0.1 } else { -0.1 }))
            .collect();
        let path = open(&pts);
        let c = cleaned(&path, &Cleanup::STANDARD).unwrap();
        let q: Vec<Point> = c.flatten(0.01).into_iter().flat_map(|pl| pl.points).collect();
        assert!(q.iter().all(|x| x.y.abs() < 0.11), "{c:?}");
        // Near the ends (which stay) the curves can be a tolerance from the
        // smoothed points.
        assert!(q.iter().filter(|x| x.x > 5.0 && x.x < 35.0).all(|x| x.y.abs() < 0.01));
        assert_eq!(q[0], pts[0]);
        assert_eq!(q[q.len() - 1], pts[200]);
        assert!(c.cmds.len() < 10, "{}", c.cmds.len());
    }

    #[test]
    fn small_loops_and_spikes_go_and_corners_stay() {
        // A 10 mm square. Its top side has a small loop (it comes back over
        // itself) and a spike of 0.6 mm.
        let pts = vec![
            p(0.0, 0.0),
            p(3.6, 0.0),
            p(3.6, -0.3),
            p(3.3, -0.3),
            p(3.3, 0.0),
            p(6.0, 0.0),
            p(6.1, -0.6),
            p(6.2, 0.0),
            p(10.0, 0.0),
            p(10.0, 10.0),
            p(0.0, 10.0),
        ];
        let path = Path::polygon(&pts);
        let c = cleaned(&path, &Cleanup::STANDARD).unwrap();
        let square = Path::polygon(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)]);
        assert!(deviation(&c, &square) < 0.05, "{}", deviation(&c, &square));
        assert!(deviation(&square, &c) < 0.05, "{}", deviation(&square, &c));
        // Four lines: the sides stay straight and the corners sharp.
        assert!(c.cmds.iter().all(|x| !matches!(x, PathCmd::CubicTo(..))), "{c:?}");
        assert!(c.cmds.len() <= 6, "{c:?}");
    }

    #[test]
    fn a_rough_circle_stays_round() {
        let pts: Vec<Point> = (0..400)
            .map(|i| {
                let a = f64::from(i) / 400.0 * std::f64::consts::TAU;
                let r = 10.0 + if i % 2 == 0 { 0.08 } else { -0.08 };
                p(r * math::cos(a), r * math::sin(a))
            })
            .collect();
        let path = Path::polygon(&pts);
        let c = cleaned(&path, &Cleanup::STANDARD).unwrap();
        let circle = Path::ellipse(Point::ZERO, 10.0, 10.0);
        assert!(deviation(&c, &circle) < 0.08, "{}", deviation(&c, &circle));
        assert!(c.cmds.len() < 14, "{}", c.cmds.len());
    }

    #[test]
    fn tiny_parts_go() {
        let big = Path::polygon(&[p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(0.0, 5.0)]);
        let dot = Path::polygon(&[p(8.0, 0.0), p(8.3, 0.0), p(8.3, 0.3)]);
        let dash = open(&[p(10.0, 0.0), p(10.3, 0.0)]);
        let path = Path::joined([&big, &dot, &dash]);
        let parts = clean_polylines(&path, &Cleanup::STANDARD);
        assert_eq!(parts.len(), 1);
        let keep = Cleanup {
            tiny: false,
            ..Cleanup::STANDARD
        };
        let kept = clean_polylines(&path, &keep);
        assert_eq!(kept.len(), 3, "{kept:?}");
        assert!(cleaned(&dash, &Cleanup::STANDARD).is_none());
    }
}
