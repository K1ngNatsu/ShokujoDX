//! Spines and stitch angle lines of shapes.
//!
//! Not a port. The methods are common geometry:
//!
//! - [`spine`]: a line along the middle of a long shape, from one end to
//!   the other. The shape is drawn on a grid of at most [`GRID`] cells on a
//!   side. The two cells farthest apart by paths inside the shape are the
//!   ends, also when the shape bends (a C or a swoosh). The outline is cut
//!   into two sides at the ends, and the middles of the points at the same
//!   fraction of each side make a first line. The middle part of that line
//!   goes on straight to the outline: those two points are the ends of the
//!   second pass. So the spine of a bar ends in the middle of its short
//!   edges, not at its corners.
//! - [`cross_lines`]: lines across a shape at an angle to a guide line (a
//!   spine or a line that the user draws). Each line goes from its point on
//!   the guide to the nearest edge on each side, holes included. A line
//!   that would cross the line before it (on the inside of a tight bend) is
//!   left out.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::Point;
use crate::math::sin_cos_deg;
use crate::region::ring_contains;

/// Most grid cells on one side of the shape for [`spine`].
pub const GRID: usize = 160;
/// The ends of the second pass go on from the point of the first line
/// that is this many widths along it from its end.
const END_REACH: f64 = 1.0;
/// Most points of a spine.
const MAX_POINTS: usize = 2000;
const SQRT_2: f64 = std::f64::consts::SQRT_2;

/// The spine of the closed `ring` (see the module comment), from one end
/// to the other. `None` if the ring has no area.
pub fn spine(ring: &[Point]) -> Option<Vec<Point>> {
    if ring.len() < 3 {
        return None;
    }
    let (a, b) = far_ends(ring)?;
    let rings = [ring.to_vec()];
    let first = middles(ring, a, b)?;
    let Some(line) = trimmed(&first, &rings) else {
        return Some(smooth(&first.iter().map(|p| p.0).collect::<Vec<_>>()));
    };
    let (ring2, ia, ib) = with_points(ring, line[0], line[line.len() - 1])?;
    let second = middles(&ring2, ia, ib)?;
    let line = trimmed(&second, &rings).unwrap_or_else(|| second.iter().map(|p| p.0).collect());
    Some(smooth(&line))
}

/// A line of middles (with the widths) without its ends: near each end,
/// the line bends along the end edge, because the two sides start there
/// at the same point. One width (the distance of the two side points) along
/// the line from its end, it is good: from there it goes on as it bends
/// there (an arc) to the outline. `None` if an end does not reach the outline.
fn trimmed(pts: &[(Point, f64)], rings: &[Vec<Point>]) -> Option<Vec<Point>> {
    // The index where the good part starts, and the way from there to the outline.
    let end = |pts: &[(Point, f64)]| -> Option<(usize, Vec<Point>)> {
        let mut along = 0.0;
        for k in 1..pts.len() / 2 {
            along += pts[k].0.distance(pts[k - 1].0);
            if along >= END_REACH * pts[k].1 {
                let step = (k / 4).max(1);
                let (p0, p1, p2) = (
                    pts[k].0,
                    pts[(k + step).min(pts.len() - 1)].0,
                    pts[(k + 2 * step).min(pts.len() - 1)].0,
                );
                let (d0, d1) = (p0 - p1, p1 - p2);
                let dir = d0.normalized()?;
                let turn = if d0.length() > 0.0 && d1.length() > 0.0 {
                    crate::math::atan2(d1.cross(d0), d1.dot(d0)) / d0.length()
                } else {
                    0.0
                };
                let way = arc_to_edge(rings, p0, dir, turn, 3.0 * pts[k].1 + 1.0)
                    .or_else(|| ray(rings, p0, dir).map(|q| vec![q]))?;
                return Some((k, way));
            }
        }
        None
    };
    let n = pts.len();
    let rev: Vec<(Point, f64)> = pts.iter().rev().copied().collect();
    let ((k0, way0), (k1, way1)) = (end(pts)?, end(&rev)?);
    let k1 = n - 1 - k1;
    if k0 >= k1 {
        return None;
    }
    let mut out: Vec<Point> = way0.into_iter().rev().collect();
    out.extend(pts[k0..=k1].iter().map(|p| p.0));
    out.extend(way1);
    out.dedup();
    Some(out)
}

/// Lines across the shape (`rings`, even-odd) at `angle_deg` to the
/// `guide`, about `step` apart along it (see the module comment). 90° is
/// square to the guide. The guide goes on straight from its ends to the
/// outline first. Each line goes from the left side of the guide to the right.
pub fn cross_lines(rings: &[Vec<Point>], guide: &[Point], angle_deg: f64, step: f64) -> Vec<(Point, Point)> {
    let mut g: Vec<Point> = guide.to_vec();
    g.dedup();
    if g.len() < 2 || step <= 0.0 {
        return Vec::new();
    }
    let inside = |p: Point| rings.iter().filter(|r| ring_contains(r, p)).count() % 2 == 1;
    // On to the outline at both ends.
    if inside(g[0])
        && let Some(d) = (g[0] - g[1]).normalized()
        && let Some(p) = ray(rings, g[0], d)
    {
        g.insert(0, p);
    }
    let n = g.len();
    if inside(g[n - 1])
        && let Some(d) = (g[n - 1] - g[n - 2]).normalized()
        && let Some(p) = ray(rings, g[n - 1], d)
    {
        g.push(p);
    }
    let mut at = vec![0.0];
    for w in g.windows(2) {
        at.push(at[at.len() - 1] + w[0].distance(w[1]));
    }
    let total = at[at.len() - 1];
    if total <= 0.0 {
        return Vec::new();
    }
    // Points a little in from the ends, then every `step`.
    let inset = (step / 2.0).min(0.3).min(total / 4.0);
    let count = (((total - 2.0 * inset) / step).round() as usize).max(1);
    let (s, c) = sin_cos_deg(angle_deg);
    let mut out: Vec<(Point, Point)> = Vec::new();
    for q in 0..=count {
        let len = inset + (total - 2.0 * inset) * q as f64 / count as f64;
        let i = at.partition_point(|&l| l <= len).clamp(1, g.len() - 1);
        let seg = at[i] - at[i - 1];
        let t = if seg > 0.0 { (len - at[i - 1]) / seg } else { 0.0 };
        let p = g[i - 1].lerp(g[i], t);
        // The direction of the guide here: over about one step.
        let back = g[i - 1].lerp(g[i], t) - point_at(&g, &at, (len - step / 2.0).max(0.0));
        let ahead = point_at(&g, &at, (len + step / 2.0).min(total)) - p;
        let Some(tan) = (back + ahead).normalized() else {
            continue;
        };
        // Turn the guide direction by the angle; 90° points to the left side.
        let d = Point::new(tan.x * c + tan.y * s, -tan.x * s + tan.y * c);
        if !inside(p) {
            continue;
        }
        let (Some(l), Some(r)) = (ray(rings, p, d), ray(rings, p, -d)) else {
            continue;
        };
        if let Some(&(pl, pr)) = out.last()
            && segments_cross(pl, pr, l, r)
        {
            continue;
        }
        out.push((l, r));
    }
    out
}

fn point_at(g: &[Point], at: &[f64], len: f64) -> Point {
    let i = at.partition_point(|&l| l <= len).clamp(1, g.len() - 1);
    let seg = at[i] - at[i - 1];
    let t = if seg > 0.0 { (len - at[i - 1]) / seg } else { 0.0 };
    g[i - 1].lerp(g[i], t)
}

/// The way along an arc from `p` (not included) in direction `d`, which
/// turns by `turn` radians for each mm, to where it first meets an edge of
/// the rings (the last point), within `max_len` mm.
fn arc_to_edge(rings: &[Vec<Point>], p: Point, d: Point, turn: f64, max_len: f64) -> Option<Vec<Point>> {
    const STEP: f64 = 0.25;
    let (s, c) = (crate::math::sin(turn * STEP), crate::math::cos(turn * STEP));
    let (mut at, mut dir, mut gone) = (p, d, 0.0);
    let mut way = Vec::new();
    while gone < max_len {
        let next = at + dir * STEP;
        let mut best: Option<(f64, Point)> = None;
        for r in rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                if let Some(t) = cross_at(at, next, a, b)
                    && best.is_none_or(|x| t < x.0)
                {
                    best = Some((t, at.lerp(next, t)));
                }
            }
        }
        if let Some((_, q)) = best {
            way.push(q);
            return Some(way);
        }
        // Every mm a point, so the way keeps its bend.
        if ((gone + STEP) / 1.0).floor() > (gone / 1.0).floor() {
            way.push(next);
        }
        at = next;
        dir = Point::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
        gone += STEP;
    }
    None
}

/// Where segment `p`-`q` crosses segment `a`-`b`, as a fraction of `p`-`q`.
fn cross_at(p: Point, q: Point, a: Point, b: Point) -> Option<f64> {
    let (r, e) = (q - p, b - a);
    let den = r.cross(e);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (a - p).cross(e) / den;
    let u = (a - p).cross(r) / den;
    (t > 1e-9 && t <= 1.0 && (0.0..=1.0).contains(&u)).then_some(t)
}

/// Where the ray from `p` in direction `d` first meets an edge of the rings.
pub fn ray(rings: &[Vec<Point>], p: Point, d: Point) -> Option<Point> {
    let mut best = f64::INFINITY;
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let e = b - a;
            let den = d.cross(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let w = a - p;
            let s = w.cross(e) / den;
            let u = w.cross(d) / den;
            if s > 1e-9 && (0.0..=1.0).contains(&u) && s < best {
                best = s;
            }
        }
    }
    best.is_finite().then(|| p + d * best)
}

/// The segments `a`-`b` and `c`-`d` cross at a point inside both.
pub fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return false;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    t > 1e-6 && t < 1.0 - 1e-6 && u > 1e-6 && u < 1.0 - 1e-6
}

/// A cell of the grid and its path length from the start, for the search.
#[derive(PartialEq)]
struct Cell(f64, usize);

impl Eq for Cell {}

impl Ord for Cell {
    fn cmp(&self, o: &Self) -> Ordering {
        // The smallest length first; ties by index, so the result is deterministic.
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
    }
}

impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// The indices of the two ring points nearest to the two grid cells that
/// are farthest apart by paths inside the ring.
fn far_ends(ring: &[Point]) -> Option<(usize, usize)> {
    let (mut lo, mut hi) = (
        Point::new(f64::INFINITY, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in ring {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let size = (hi.x - lo.x).max(hi.y - lo.y);
    if size.is_nan() || size <= 0.0 {
        return None;
    }
    let cell = size / GRID as f64;
    let (nx, ny) = (
        ((hi.x - lo.x) / cell).ceil() as usize + 1,
        ((hi.y - lo.y) / cell).ceil() as usize + 1,
    );
    let center = |i: usize| {
        Point::new(
            lo.x + ((i % nx) as f64 + 0.5) * cell,
            lo.y + ((i / nx) as f64 + 0.5) * cell,
        )
    };
    // Cells inside: scan lines through the cell centers.
    let mut inside = vec![false; nx * ny];
    let mut xs: Vec<f64> = Vec::new();
    for j in 0..ny {
        let y = lo.y + (j as f64 + 0.5) * cell;
        xs.clear();
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            if (a.y > y) != (b.y > y) {
                xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
        xs.sort_by(f64::total_cmp);
        for c in xs.as_chunks::<2>().0 {
            let i0 = ((c[0] - lo.x) / cell - 0.5).ceil().max(0.0) as usize;
            let i1 = ((c[1] - lo.x) / cell - 0.5).floor();
            if i1 < 0.0 {
                continue;
            }
            for i in i0..=(i1 as usize).min(nx - 1) {
                inside[j * nx + i] = true;
            }
        }
    }
    let start = inside.iter().position(|&x| x)?;
    let farthest = |from: usize| -> usize {
        let mut dist = vec![f64::INFINITY; nx * ny];
        let mut heap = BinaryHeap::new();
        dist[from] = 0.0;
        heap.push(Cell(0.0, from));
        let mut best = (0.0, from);
        while let Some(Cell(d, i)) = heap.pop() {
            if d > dist[i] {
                continue;
            }
            if d > best.0 {
                best = (d, i);
            }
            let (x, y) = ((i % nx) as i64, (i / nx) as i64);
            for (dx, dy, w) in [
                (1, 0, 1.0),
                (-1, 0, 1.0),
                (0, 1, 1.0),
                (0, -1, 1.0),
                (1, 1, SQRT_2),
                (1, -1, SQRT_2),
                (-1, 1, SQRT_2),
                (-1, -1, SQRT_2),
            ] {
                let (x2, y2) = (x + dx, y + dy);
                if x2 < 0 || y2 < 0 || x2 >= nx as i64 || y2 >= ny as i64 {
                    continue;
                }
                let k = y2 as usize * nx + x2 as usize;
                if inside[k] && d + w < dist[k] {
                    dist[k] = d + w;
                    heap.push(Cell(d + w, k));
                }
            }
        }
        best.1
    };
    let a = farthest(start);
    let b = farthest(a);
    let nearest = |p: Point| {
        (0..ring.len())
            .min_by(|&i, &j| ring[i].distance(p).total_cmp(&ring[j].distance(p)))
            .unwrap_or(0)
    };
    let (ia, ib) = (nearest(center(a)), nearest(center(b)));
    (ia != ib).then_some((ia, ib))
}

/// The ring with `a` and `b` put in on their nearest edges, and their indices.
fn with_points(ring: &[Point], a: Point, b: Point) -> Option<(Vec<Point>, usize, usize)> {
    let edge = |p: Point| {
        (0..ring.len())
            .min_by(|&i, &j| {
                let d = |k: usize| p.distance_to_segment(ring[k], ring[(k + 1) % ring.len()]);
                d(i).total_cmp(&d(j))
            })
            .unwrap_or(0)
    };
    let (ea, eb) = (edge(a), edge(b));
    let mut out = Vec::with_capacity(ring.len() + 2);
    let (mut ia, mut ib) = (None, None);
    for (i, p) in ring.iter().enumerate() {
        out.push(*p);
        // Both points on one edge: the nearer one to the edge start first.
        let mut here: Vec<(Point, bool)> = Vec::new();
        if i == ea {
            here.push((a, true));
        }
        if i == eb {
            here.push((b, false));
        }
        here.sort_by(|x, y| x.0.distance(*p).total_cmp(&y.0.distance(*p)));
        for (q, is_a) in here {
            if out.last() != Some(&q) {
                out.push(q);
            }
            if is_a {
                ia = Some(out.len() - 1);
            } else {
                ib = Some(out.len() - 1);
            }
        }
    }
    let (ia, ib) = (ia?, ib?);
    (ia != ib).then_some((out, ia, ib))
}

/// The middles of the points at the same fraction of the two sides of the
/// ring from `ia` to `ib`, from `ring[ia]` to `ring[ib]`, with the distance
/// of the two points.
fn middles(ring: &[Point], ia: usize, ib: usize) -> Option<Vec<(Point, f64)>> {
    let n = ring.len();
    let side = |forward: bool| -> Vec<Point> {
        let mut v = vec![ring[ia]];
        let mut i = ia;
        while i != ib {
            i = if forward { (i + 1) % n } else { (i + n - 1) % n };
            v.push(ring[i]);
        }
        v
    };
    let (s1, s2) = (side(true), side(false));
    let len = |s: &[Point]| s.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
    let (l1, l2) = (len(&s1), len(&s2));
    if l1 <= 0.0 || l2 <= 0.0 {
        return None;
    }
    // About one point for each mm of the longer side.
    let count = (l1.max(l2).round() as usize).clamp(16, MAX_POINTS);
    let at = |s: &[Point], l: f64, f: f64| -> Point {
        let mut left = l * f;
        for w in s.windows(2) {
            let d = w[0].distance(w[1]);
            if left <= d {
                return w[0].lerp(w[1], if d > 0.0 { left / d } else { 0.0 });
            }
            left -= d;
        }
        s[s.len() - 1]
    };
    Some(
        (0..=count)
            .map(|q| {
                let f = q as f64 / count as f64;
                let (p1, p2) = (at(&s1, l1, f), at(&s2, l2, f));
                (p1.midpoint(p2), p1.distance(p2))
            })
            .collect(),
    )
}

/// The points, with a moving mean over five points, two times; the ends stay.
fn smooth(pts: &[Point]) -> Vec<Point> {
    let mut cur: Vec<Point> = pts.to_vec();
    for _ in 0..2 {
        let prev = cur.clone();
        for i in 2..prev.len().saturating_sub(2) {
            let sum = prev[i - 2] + prev[i - 1] + prev[i] + prev[i + 1] + prev[i + 2];
            cur[i] = sum * 0.2;
        }
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn rect(w: f64, h: f64) -> Vec<Point> {
        vec![p(0.0, 0.0), p(w, 0.0), p(w, h), p(0.0, h)]
    }

    /// A C shape: a ring sector round (0, 0), radii 20 and 30, open on the right.
    fn c_shape() -> Vec<Point> {
        let arc = |r: f64, from: f64, to: f64| -> Vec<Point> {
            (0..=60)
                .map(|i| {
                    let a = (from + (to - from) * f64::from(i) / 60.0).to_radians();
                    p(r * a.cos(), r * a.sin())
                })
                .collect()
        };
        let mut ring = arc(30.0, 40.0, 320.0);
        ring.extend(arc(20.0, 320.0, 40.0));
        ring
    }

    #[test]
    fn a_bar_has_its_spine_in_the_middle_from_end_to_end() {
        let s = spine(&rect(80.0, 10.0)).unwrap();
        let (first, last) = (s[0], s[s.len() - 1]);
        let (a, b) = if first.x < last.x { (first, last) } else { (last, first) };
        assert!(a.x < 0.5 && (a.y - 5.0).abs() < 0.5, "{a:?}");
        assert!(b.x > 79.5 && (b.y - 5.0).abs() < 0.5, "{b:?}");
        assert!(s.iter().all(|q| (q.y - 5.0).abs() < 0.6), "{s:?}");
    }

    #[test]
    fn a_c_shape_has_its_spine_round_the_bend() {
        let ring = c_shape();
        let s = spine(&ring).unwrap();
        // All points near the middle radius, inside the shape.
        for q in &s {
            let r = q.length();
            assert!((r - 25.0).abs() < 2.0, "{q:?} at {r}");
        }
        // From one open end to the other: round the left side.
        assert!(s.iter().any(|q| q.x < -24.0));
        let ends = (s[0], s[s.len() - 1]);
        assert!(ends.0.x > 15.0 && ends.1.x > 15.0, "{ends:?}");
    }

    #[test]
    fn cross_lines_go_across_the_guide_to_the_edges() {
        let rings = [rect(40.0, 10.0)];
        let guide = [p(5.0, 5.0), p(35.0, 5.0)];
        let lines = cross_lines(&rings, &guide, 90.0, 2.0);
        // The guide goes on to x = 0 and x = 40: about 20 lines.
        assert!(lines.len() >= 19, "{}", lines.len());
        for (l, r) in &lines {
            assert!((l.x - r.x).abs() < 1e-9);
            assert!((l.y - r.y).abs() > 9.99);
        }
        assert!(lines[0].0.x < 1.0 && lines[lines.len() - 1].0.x > 39.0);
        // At 45° the lines lean, and they stay in the shape.
        for (l, r) in cross_lines(&rings, &guide, 45.0, 2.0) {
            let d = r - l;
            assert!((d.x.abs() - d.y.abs()).abs() < 1e-6, "{l:?} {r:?}");
        }
    }

    #[test]
    fn cross_lines_stop_at_holes_and_do_not_cross() {
        let hole = vec![p(15.0, 3.0), p(25.0, 3.0), p(25.0, 7.0), p(15.0, 7.0)];
        let rings = [rect(40.0, 10.0), hole];
        let lines = cross_lines(&rings, &[p(1.0, 1.5), p(39.0, 1.5)], 90.0, 1.0);
        for (l, r) in &lines {
            if l.x > 15.5 && l.x < 24.5 {
                // From y = 0 to the hole at y = 3.
                assert!((l.y.max(r.y) - 3.0).abs() < 1e-9, "{l:?} {r:?}");
            }
        }
        // A tight bend: no two lines cross.
        let ring = c_shape();
        let guide = spine(&ring).unwrap();
        let lines = cross_lines(&[ring], &guide, 90.0, 0.5);
        for w in lines.windows(2) {
            assert!(!segments_cross(w[0].0, w[0].1, w[1].0, w[1].1));
        }
        assert!(lines.len() > 100);
    }
}
