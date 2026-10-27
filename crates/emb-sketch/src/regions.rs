//! The closed regions of a sketch (P14 Milestone 4, ADR 0075).
//!
//! The curves that are not construction curves cut each other where they
//! cross; the pieces between the cuts are the edges of a plane graph. The
//! edges that end at a point with no other edge go away (they close no
//! region). Each face of the graph is a walk that turns as far right as it
//! can at each point (the face is on its left). A walk with a positive area
//! is a region; a walk with a negative area is the outside of a group of
//! curves, and it is a hole of the smallest region round it (an island in
//! the region). The method is common practice for plane graphs, in our own
//! code.
//!
//! Milestone 5 (ADR 0076): a region has a key (the curves round it, each
//! with its direction, and a point in it), so the objects made from it can
//! find it again after a change of the sketch (`Sketch::find_region`).

use std::collections::BTreeSet;

use emb_geom::{Path, Point, Transform, math, region};
use serde::{Deserialize, Serialize};

use crate::shape::{Shape, crossings};
use crate::{CurveId, Sketch};

/// A closed region of a sketch: its outline, its holes, and the curves
/// round it.
#[derive(Debug, Clone, PartialEq)]
pub struct SketchRegion {
    pub outer: Path,
    pub holes: Vec<Path>,
    /// The area less the holes, in mm².
    pub area: f64,
    /// The curves of the outline and of the holes (sorted).
    pub curves: Vec<CurveId>,
    /// The pieces of curves round the region (outline and holes): each
    /// curve, and `true` if the walk round the region (the region on its
    /// left) goes along the curve. Sorted, a curve once for each piece.
    pub sides: Vec<(CurveId, bool)>,
}

/// What finds a region again after a change of the sketch: its sides and
/// a point in it (sketch coordinates).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionKey {
    pub sides: Vec<(CurveId, bool)>,
    pub at: Point,
}

impl SketchRegion {
    /// True if `p` is in the region (not in a hole).
    pub fn contains(&self, p: Point) -> bool {
        let ring = |path: &Path| -> Vec<Point> { path.flatten(FLAT).into_iter().flat_map(|pl| pl.points).collect() };
        region::ring_contains(&ring(&self.outer), p) && !self.holes.iter().any(|h| region::ring_contains(&ring(h), p))
    }

    /// The region moved by `t`.
    pub fn transformed(&self, t: &Transform) -> SketchRegion {
        SketchRegion {
            outer: self.outer.transformed(t),
            holes: self.holes.iter().map(|h| h.transformed(t)).collect(),
            area: self.area * (t.a * t.d - t.b * t.c).abs(),
            curves: self.curves.clone(),
            sides: self.sides.clone(),
        }
    }

    /// A point well in the region: the middle of the widest part of a
    /// horizontal line through the middle of its box.
    pub fn inner_point(&self) -> Point {
        let rings: Vec<Vec<Point>> = std::iter::once(&self.outer).chain(&self.holes).map(ring_of).collect();
        let first = rings[0].first().copied().unwrap_or(Point::new(0.0, 0.0));
        let (lo, hi) = rings[0]
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
        let mut best: Option<(f64, Point)> = None;
        // A few lines near the middle, in case one goes through a point.
        for k in [0.5, 0.37, 0.63, 0.25, 0.75] {
            let y = lo + (hi - lo) * k;
            let mut xs: Vec<f64> = Vec::new();
            for r in &rings {
                for (a, b) in r.iter().zip(r.iter().cycle().skip(1)) {
                    if (a.y > y) != (b.y > y) {
                        xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for w in xs.as_chunks::<2>().0 {
                let p = Point::new((w[0] + w[1]) / 2.0, y);
                let width = w[1] - w[0];
                if best.is_none_or(|(bw, _)| width > bw) && self.contains(p) {
                    best = Some((width, p));
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map_or(first, |(_, p)| p)
    }

    /// The key that finds this region again (in the coordinates of its
    /// paths).
    pub fn key(&self) -> RegionKey {
        RegionKey {
            sides: self.sides.clone(),
            at: self.inner_point(),
        }
    }
}

/// The tolerance to flatten curves for areas and tests, in mm.
const FLAT: f64 = 0.02;
/// Points nearer than this are one point of the graph, in mm.
const SAME: f64 = 1e-4;

/// A piece of a curve between two cuts.
#[derive(Debug, Clone, Copy)]
struct Edge {
    curve: CurveId,
    shape: Shape,
    t0: f64,
    t1: f64,
    v: [usize; 2],
}

impl Edge {
    fn len(&self) -> f64 {
        self.shape.length_between(self.t0, self.t1)
    }

    /// The parameter a short way from the start (forward) or the end.
    fn near_end(&self, forward: bool) -> f64 {
        let step_mm = (self.len() * 0.25).min(0.1);
        let k = match self.shape {
            Shape::Line { a, b } => step_mm / a.distance(b).max(1e-12),
            Shape::Circle { r, .. } | Shape::Arc { r, .. } => step_mm / r.max(1e-12),
            Shape::Cubic { .. } => step_mm / self.len().max(1e-12) * (self.t1 - self.t0),
        };
        if forward { self.t0 + k } else { self.t1 - k }
    }
}

/// Adds the piece of `e` to `path`, forward or back (no move to its start).
fn add_piece(path: Path, e: &Edge, forward: bool) -> Path {
    let (from, to) = if forward { (e.t0, e.t1) } else { (e.t1, e.t0) };
    match e.shape {
        Shape::Line { .. } => path.line_to(e.shape.at(to)),
        Shape::Cubic { p } => {
            let q = crate::shape::cubic_part(&p, from, to);
            path.cubic_to(q[1], q[2], q[3])
        }
        Shape::Circle { c, r } | Shape::Arc { c, r, .. } => {
            // The angles of the parameters (an arc counts from its start).
            let base = match e.shape {
                Shape::Arc { a0, .. } => a0,
                _ => 0.0,
            };
            arc_to(path, c, r, base + from, base + to)
        }
    }
}

/// Cubic curves of the circle round `c` from angle `a0` to `a1` (either
/// way), at most 90° each.
fn arc_to(mut path: Path, c: Point, r: f64, a0: f64, a1: f64) -> Path {
    let sweep = a1 - a0;
    let n = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * math::tan(step / 4.0);
    let at = |a: f64| Point::new(c.x + r * math::cos(a), c.y + r * math::sin(a));
    let tangent = |a: f64| Point::new(-math::sin(a), math::cos(a));
    for i in 0..n {
        let (s, e) = (a0 + step * i as f64, a0 + step * (i + 1) as f64);
        path = path.cubic_to(at(s) + tangent(s) * (k * r), at(e) - tangent(e) * (k * r), at(e));
    }
    path
}

fn ring_of(path: &Path) -> Vec<Point> {
    path.flatten(FLAT).into_iter().flat_map(|pl| pl.points).collect()
}

impl Sketch {
    /// The closed regions of the sketch (see the module text), the largest
    /// first.
    pub fn regions(&self) -> Vec<SketchRegion> {
        let curves: Vec<(CurveId, Shape)> = self
            .curves
            .iter()
            .filter(|c| !c.construction)
            .filter_map(|c| Some((c.id, self.shape(c.id)?)))
            .collect();
        // The cuts of each curve: its ends and the places where others cross.
        let mut points: Vec<Point> = Vec::new();
        let vertex = |p: Point, points: &mut Vec<Point>| -> usize {
            if let Some(i) = points.iter().position(|q| q.distance(p) <= SAME) {
                return i;
            }
            points.push(p);
            points.len() - 1
        };
        let mut edges: Vec<Edge> = Vec::new();
        for (i, (id, sh)) in curves.iter().enumerate() {
            let closed = matches!(sh, Shape::Circle { .. });
            let end = sh.end_param();
            let mut ts: Vec<f64> = if closed { Vec::new() } else { vec![0.0, end] };
            for (j, (_, other)) in curves.iter().enumerate() {
                if i == j {
                    continue;
                }
                for x in crossings(sh, other) {
                    let t = sh.param(x);
                    // A cut just before the start of an arc is at its start.
                    let t = if !closed && t > end {
                        if t > (end + std::f64::consts::TAU) / 2.0 {
                            0.0
                        } else {
                            end
                        }
                    } else {
                        t
                    };
                    ts.push(t);
                }
            }
            ts.sort_by(f64::total_cmp);
            let tol = SAME
                / match *sh {
                    Shape::Line { a, b } => a.distance(b).max(1e-9),
                    Shape::Circle { r, .. } | Shape::Arc { r, .. } => r.max(1e-9),
                    Shape::Cubic { .. } => sh.length().max(1e-9),
                };
            ts.dedup_by(|a, b| (*a - *b).abs() <= tol);
            if closed {
                if ts.is_empty() {
                    ts.push(0.0);
                }
                // Round the circle: the last cut to the first one again.
                let first = ts[0];
                if (first + std::f64::consts::TAU - ts[ts.len() - 1]).abs() > tol || ts.len() == 1 {
                    ts.push(first + std::f64::consts::TAU);
                }
            }
            for w in ts.windows(2) {
                let (t0, t1) = (w[0], w[1]);
                if t1 - t0 <= tol {
                    continue;
                }
                let v0 = vertex(sh.at(t0), &mut points);
                let v1 = vertex(sh.at(t1), &mut points);
                edges.push(Edge {
                    curve: *id,
                    shape: *sh,
                    t0,
                    t1,
                    v: [v0, v1],
                });
            }
        }
        // The edges with a free end close nothing.
        loop {
            let mut degree = vec![0usize; points.len()];
            for e in &edges {
                degree[e.v[0]] += 1;
                degree[e.v[1]] += 1;
            }
            let n = edges.len();
            edges.retain(|e| degree[e.v[0]] > 1 && degree[e.v[1]] > 1);
            if edges.len() == n {
                break;
            }
        }
        // Half edges: 2k from the first point of edge k, 2k+1 back.
        let half = edges.len() * 2;
        let start = |h: usize| {
            let e = &edges[h / 2];
            if h.is_multiple_of(2) { e.v[0] } else { e.v[1] }
        };
        let angle = |h: usize| {
            let e = &edges[h / 2];
            let forward = h.is_multiple_of(2);
            let from = points[start(h)];
            let q = e.shape.at(e.near_end(forward));
            math::atan2(q.y - from.y, q.x - from.x)
        };
        let mut out_of: Vec<Vec<usize>> = vec![Vec::new(); points.len()];
        for h in 0..half {
            out_of[start(h)].push(h);
        }
        for list in &mut out_of {
            list.sort_by(|a, b| angle(*a).total_cmp(&angle(*b)));
        }
        // The next half edge of a face: at the end point, the one before
        // the way back in the order round the point.
        let next = |h: usize| {
            let twin = h ^ 1;
            let v = start(twin);
            let list = &out_of[v];
            let i = list.iter().position(|x| *x == twin).expect("the way back starts here");
            list[(i + list.len() - 1) % list.len()]
        };
        let mut used = vec![false; half];
        type Walk = (Path, f64, Vec<CurveId>, Point, Vec<(CurveId, bool)>);
        let mut faces: Vec<Walk> = Vec::new();
        let mut outsides: Vec<Walk> = Vec::new();
        for h0 in 0..half {
            if used[h0] {
                continue;
            }
            let mut walk = Vec::new();
            let mut h = h0;
            while !used[h] {
                used[h] = true;
                walk.push(h);
                h = next(h);
                if walk.len() > half {
                    break;
                }
            }
            if h != h0 {
                continue;
            }
            let mut path = Path::new().move_to(points[start(walk[0])]);
            for &h in &walk {
                path = add_piece(path, &edges[h / 2], h.is_multiple_of(2));
            }
            let path = path.close();
            let area = region::ring_signed_area(&ring_of(&path));
            let sides: Vec<(CurveId, bool)> = walk.iter().map(|h| (edges[h / 2].curve, h.is_multiple_of(2))).collect();
            let mut ids: Vec<CurveId> = sides.iter().map(|s| s.0).collect();
            ids.sort();
            ids.dedup();
            // A point just on the left of the first edge (in the face).
            let e = &edges[walk[0] / 2];
            let forward = walk[0].is_multiple_of(2);
            let mid = (e.t0 + e.t1) / 2.0;
            let d = e.shape.tangent(mid);
            let d = if forward { d } else { d * -1.0 };
            let side = e.shape.at(mid) + d.normalized().unwrap_or(Point::new(1.0, 0.0)).perp() * 1e-3;
            if area > 1e-9 {
                faces.push((path, area, ids, side, sides));
            } else if area < -1e-9 {
                outsides.push((path, area, ids, side, sides));
            }
        }
        // Each outside is a hole of the smallest face round it.
        let rings: Vec<Vec<Point>> = faces.iter().map(|f| ring_of(&f.0)).collect();
        let mut holes: Vec<Vec<usize>> = vec![Vec::new(); faces.len()];
        for (k, o) in outsides.iter().enumerate() {
            let parent = (0..faces.len())
                .filter(|i| region::ring_contains(&rings[*i], o.3))
                .min_by(|a, b| faces[*a].1.total_cmp(&faces[*b].1));
            if let Some(i) = parent {
                holes[i].push(k);
            }
        }
        let mut out: Vec<SketchRegion> = faces
            .into_iter()
            .zip(holes)
            .map(|((outer, area, mut ids, _, mut sides), hs)| {
                let mut hole_paths = Vec::new();
                let mut hole_area = 0.0;
                for k in hs {
                    hole_paths.push(outsides[k].0.clone());
                    hole_area += -outsides[k].1;
                    ids.extend(outsides[k].2.iter().copied());
                    sides.extend(outsides[k].4.iter().copied());
                }
                let ids: BTreeSet<CurveId> = ids.into_iter().collect();
                sides.sort();
                SketchRegion {
                    outer,
                    holes: hole_paths,
                    area: area - hole_area,
                    curves: ids.into_iter().collect(),
                    sides,
                }
            })
            .collect();
        out.sort_by(|a, b| b.area.total_cmp(&a.area));
        out
    }

    /// The region at `p` (the smallest one that has `p` in it, not in a
    /// hole).
    pub fn region_at(&self, p: Point) -> Option<SketchRegion> {
        self.regions()
            .into_iter()
            .filter(|r| r.contains(p))
            .min_by(|a, b| a.area.total_cmp(&b.area))
    }

    /// The region of `key` after a change of the sketch: the region with
    /// the same sides (the nearest to the point of the key if there are
    /// more). Else, after a change of the curves (a fillet, a trim), the
    /// smallest region round the point of the key that has more than half
    /// of the curves of the key and of its own curves in common. `None` if
    /// the region is gone.
    pub fn find_region(&self, key: &RegionKey) -> Option<SketchRegion> {
        let all = self.regions();
        let same = all
            .iter()
            .filter(|r| r.sides == key.sides)
            .min_by(|a, b| {
                let d = |r: &SketchRegion| r.inner_point().distance(key.at);
                d(a).total_cmp(&d(b))
            })
            .cloned();
        if same.is_some() {
            return same;
        }
        let theirs: BTreeSet<CurveId> = key.sides.iter().map(|s| s.0).collect();
        all.into_iter()
            .filter(|r| {
                let ours: BTreeSet<CurveId> = r.curves.iter().copied().collect();
                let both = ours.intersection(&theirs).count();
                both * 2 > ours.union(&theirs).count() && r.contains(key.at)
            })
            .min_by(|a, b| a.area.total_cmp(&b.area))
    }

    /// The path of curve `id`, forward or back (for the rails of a satin).
    pub fn curve_path_dir(&self, id: CurveId, forward: bool) -> Option<Path> {
        let shape = self.shape(id)?;
        let e = Edge {
            curve: id,
            shape,
            t0: 0.0,
            t1: shape.end_param(),
            v: [0, 0],
        };
        let start = shape.at(if forward { e.t0 } else { e.t1 });
        Some(add_piece(Path::new().move_to(start), &e, forward))
    }
}
