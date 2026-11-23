//! The middle line of a shape from the Voronoi diagram of the segments of
//! its outline (P21 Milestone 7, a trial; ADR 0151).
//!
//! `boostvoronoi` (the Voronoi diagram of Boost.Polygon in pure Rust,
//! BSL-1.0) makes the diagram. Each edge of the diagram has the points that
//! are at the same distance from two parts of the outline (two segments, a
//! segment and a corner, or two corners). The edges inside the shape are
//! its middle line, with no grid: no point is a cell off the middle, and
//! the half-width (the distance to the outline) is exact.
//!
//! An edge between two parts of the outline that meet at a corner goes from
//! that corner into the shape: it is not a middle line, and it goes. The
//! other side branches (from round corners and bumps of the outline) stay
//! for the prune of the stroke finder, as on the grid. (The trial also
//! measured two other ways: all of the edge from a corner where the outline
//! turns 40 degrees or more, and the half of it nearer to the middle line.
//! Both were worse; `docs/design/p21-plan.md` has the numbers.)

use std::collections::HashMap;

use boostvoronoi::prelude::{Builder, Cell, SourceCategory};
use emb_geom::{BBox, Point};

/// The larger side of the shape is this many units of the integer
/// coordinates of the diagram (2 to the power 22).
const UNITS: f64 = 4_194_304.0;
/// A curved edge (between a corner and a segment) has at most this many
/// points.
const CURVE_POINTS_MAX: usize = 1000;

/// The node at an end of a closed edge.
pub(crate) const NONE: usize = usize::MAX;

/// The middle line of a shape: nodes (line ends and junctions) and edges.
#[derive(Debug, Clone, Default)]
pub(crate) struct Medial {
    /// The position and the half-width of each node.
    pub nodes: Vec<(Point, f64)>,
    pub edges: Vec<MedialEdge>,
}

/// An edge of the middle line, from node to node or closed.
#[derive(Debug, Clone)]
pub(crate) struct MedialEdge {
    /// The nodes at the start and at the end ([`NONE`] for a closed edge).
    pub ends: [usize; 2],
    /// The points; a closed edge does not repeat its first point.
    pub pts: Vec<Point>,
    pub closed: bool,
}

/// A part of the outline that makes a cell of the diagram.
#[derive(Debug, Clone, Copy)]
enum Site {
    /// A segment (its index).
    Segment(usize),
    /// A corner (the index of the point).
    Corner(usize),
}

/// The middle line of the shape in `rings` (nonzero winding), with points
/// at most about `step` apart on curved edges. `None` if the rings have
/// segments that cross or overlap (the diagram needs segments that meet
/// only at their ends), or if the diagram cannot be made.
pub(crate) fn middle_line(rings: &[Vec<Point>], step: f64) -> Option<Medial> {
    let bb = BBox::from_points(rings.iter().flatten().copied())?;
    let size = bb.width().max(bb.height());
    if size.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        || step.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
    {
        return None;
    }
    let c = bb.center();
    let scale = UNITS / size;
    let to_int = |p: Point| -> [i32; 2] {
        [
            ((p.x - c.x) * scale).round() as i32,
            ((p.y - c.y) * scale).round() as i32,
        ]
    };
    let to_font = |p: Point| Point::new(p.x / scale + c.x, p.y / scale + c.y);

    // The points (each one time) and the segments of the rings.
    let mut point_of: HashMap<[i32; 2], usize> = HashMap::new();
    let mut points: Vec<[i32; 2]> = Vec::new();
    let mut segs: Vec<[usize; 2]> = Vec::new();
    let mut int_rings: Vec<Vec<[i32; 2]>> = Vec::new();
    for ring in rings {
        let mut r: Vec<[i32; 2]> = Vec::with_capacity(ring.len());
        for &p in ring {
            let q = to_int(p);
            if r.last() != Some(&q) {
                r.push(q);
            }
        }
        while r.len() > 1 && r.first() == r.last() {
            r.pop();
        }
        if r.len() < 3 {
            continue;
        }
        let ids: Vec<usize> = r
            .iter()
            .map(|q| {
                *point_of.entry(*q).or_insert_with(|| {
                    points.push(*q);
                    points.len() - 1
                })
            })
            .collect();
        for k in 0..ids.len() {
            segs.push([ids[k], ids[(k + 1) % ids.len()]]);
        }
        int_rings.push(r);
    }
    if segs.len() < 3 || crosses(&points, &segs) {
        return None;
    }
    let lines: Vec<[i32; 4]> = segs
        .iter()
        .map(|s| {
            let (a, b) = (points[s[0]], points[s[1]]);
            [a[0], a[1], b[0], b[1]]
        })
        .collect();
    let diagram = Builder::<i32>::default()
        .with_segments(lines.iter())
        .ok()?
        .build()
        .ok()?;

    let pt = |i: usize| Point::new(f64::from(points[i][0]), f64::from(points[i][1]));
    let site = |cell: &Cell| -> Site {
        let s = cell.source_index().usize();
        match cell.source_category() {
            SourceCategory::SegmentStart => Site::Corner(segs[s][0]),
            SourceCategory::SegmentEnd => Site::Corner(segs[s][1]),
            SourceCategory::SinglePoint | SourceCategory::Segment => Site::Segment(s),
        }
    };
    let ends_of = |s: Site| -> [usize; 2] {
        match s {
            Site::Segment(i) => segs[i],
            Site::Corner(i) => [i, i],
        }
    };
    // Two parts of the outline that meet at a corner.
    let touch = |a: Site, b: Site| {
        let (a, b) = (ends_of(a), ends_of(b));
        a.iter().any(|x| b.contains(x))
    };
    let distance_to = |s: Site, p: Point| -> f64 {
        match s {
            Site::Corner(i) => p.distance(pt(i)),
            Site::Segment(i) => segment_distance(p, pt(segs[i][0]), pt(segs[i][1])),
        }
    };

    let edges = diagram.edges();
    let cells = diagram.cells();
    let vertices = diagram.vertices();
    let vertex = |v: usize| Point::new(vertices[v].x(), vertices[v].y());
    // The edges of the middle line: the vertices at the ends and the points
    // (in the coordinates of the diagram).
    let mut kept: Vec<([usize; 2], Vec<Point>)> = Vec::new();
    let step_units = step * scale;
    for e in edges {
        let id = e.id().usize();
        let Ok(twin) = e.twin() else { continue };
        if twin.usize() < id || !e.is_primary() {
            continue;
        }
        let (Some(v0), Ok(Some(v1))) = (e.vertex0(), diagram.edge_get_vertex1(e.id())) else {
            continue;
        };
        let (Ok(c0), Ok(c1)) = (e.cell(), edges[twin.usize()].cell()) else {
            continue;
        };
        let (s0, s1) = (site(&cells[c0.usize()]), site(&cells[c1.usize()]));
        let (a, b) = (vertex(v0.usize()), vertex(v1.usize()));
        if touch(s0, s1) {
            continue;
        }
        let pts = match (e.is_curved(), s0, s1) {
            (true, Site::Corner(p), Site::Segment(s)) | (true, Site::Segment(s), Site::Corner(p)) => {
                parabola(pt(p), pt(segs[s][0]), pt(segs[s][1]), a, b, step_units)
            }
            _ => vec![a, b],
        };
        let mid = if pts.len() > 2 {
            pts[pts.len() / 2]
        } else {
            a.midpoint(b)
        };
        if winding(&int_rings, mid) == 0 {
            continue;
        }
        kept.push(([v0.usize(), v1.usize()], pts));
    }

    // The half-width at each vertex of the kept edges.
    let mut half_width: HashMap<usize, f64> = HashMap::new();
    for e in edges {
        let Some(v) = e.vertex0() else { continue };
        if half_width.contains_key(&v.usize()) {
            continue;
        }
        if let Ok(c) = e.cell() {
            half_width.insert(
                v.usize(),
                distance_to(site(&cells[c.usize()]), vertex(v.usize())) / scale,
            );
        }
    }
    let node = |v: usize| (to_font(vertex(v)), half_width.get(&v).copied().unwrap_or(0.0));
    Some(graph(&kept, &node, &to_font))
}

/// The nodes and edges of the middle line from the kept edges of the
/// diagram: a node at each vertex with one edge or more than two, and the
/// edges joined through the vertices with two.
fn graph(
    kept: &[([usize; 2], Vec<Point>)],
    node: &dyn Fn(usize) -> (Point, f64),
    to_font: &dyn Fn(Point) -> Point,
) -> Medial {
    let mut at: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, (ends, _)) in kept.iter().enumerate() {
        at.entry(ends[0]).or_default().push(i);
        at.entry(ends[1]).or_default().push(i);
    }
    let degree = |v: usize| at.get(&v).map_or(0, Vec::len);
    let mut out = Medial::default();
    let mut node_of: HashMap<usize, usize> = HashMap::new();
    let mut vs: Vec<usize> = at.keys().copied().filter(|&v| degree(v) != 2).collect();
    vs.sort_unstable();
    for v in vs {
        node_of.insert(v, out.nodes.len());
        out.nodes.push(node(v));
    }
    // The points of kept edge `i` from its vertex `v`.
    let from = |i: usize, v: usize| -> Vec<Point> {
        let (ends, pts) = &kept[i];
        let mut p = pts.clone();
        if ends[0] != v {
            p.reverse();
        }
        p
    };
    let other = |i: usize, v: usize| if kept[i].0[0] == v { kept[i].0[1] } else { kept[i].0[0] };
    let mut used = vec![false; kept.len()];
    let mut starts: Vec<(usize, usize)> = node_of
        .keys()
        .flat_map(|&v| at[&v].iter().map(move |&i| (v, i)))
        .collect();
    starts.sort_unstable();
    for (v, first) in starts {
        if used[first] {
            continue;
        }
        let (mut cur, mut i) = (v, first);
        let mut pts: Vec<Point> = Vec::new();
        loop {
            used[i] = true;
            let p = from(i, cur);
            if pts.is_empty() {
                pts = p;
            } else {
                pts.extend(p.into_iter().skip(1));
            }
            cur = other(i, cur);
            if node_of.contains_key(&cur) {
                break;
            }
            match at[&cur].iter().copied().find(|&j| !used[j]) {
                Some(j) => i = j,
                None => break,
            }
        }
        let end = node_of.get(&cur).copied().unwrap_or(NONE);
        out.edges.push(MedialEdge {
            ends: [node_of[&v], end],
            pts: pts.into_iter().map(to_font).collect(),
            closed: false,
        });
    }
    // Loops with no node (the middle line of an "O").
    for first in 0..kept.len() {
        if used[first] {
            continue;
        }
        let start = kept[first].0[0];
        let (mut cur, mut i) = (start, first);
        let mut pts: Vec<Point> = Vec::new();
        loop {
            used[i] = true;
            let p = from(i, cur);
            if pts.is_empty() {
                pts = p;
            } else {
                pts.extend(p.into_iter().skip(1));
            }
            cur = other(i, cur);
            if cur == start {
                break;
            }
            match at[&cur].iter().copied().find(|&j| !used[j]) {
                Some(j) => i = j,
                None => break,
            }
        }
        if cur == start && pts.len() > 1 {
            pts.pop();
        }
        if pts.len() >= 3 {
            out.edges.push(MedialEdge {
                ends: [NONE, NONE],
                pts: pts.into_iter().map(to_font).collect(),
                closed: true,
            });
        }
    }
    out
}

/// The points of the curved edge from `q0` to `q1` between the corner `p`
/// and the segment from `a` to `b` (a parabola), about `step` apart.
fn parabola(p: Point, a: Point, b: Point, q0: Point, q1: Point, step: f64) -> Vec<Point> {
    let Some(u) = (b - a).normalized() else {
        return vec![q0, q1];
    };
    let n = u.perp();
    let (px, py) = ((p - a).dot(u), (p - a).dot(n));
    if py.abs() < 1e-9 {
        return vec![q0, q1];
    }
    let (x0, x1) = ((q0 - a).dot(u), (q1 - a).dot(u));
    // The parabola is longer than the line between its ends.
    let k = ((1.5 * q0.distance(q1) / step).ceil() as usize).clamp(1, CURVE_POINTS_MAX);
    let mut out = Vec::with_capacity(k + 1);
    out.push(q0);
    for i in 1..k {
        let x = x0 + (x1 - x0) * i as f64 / k as f64;
        let y = ((x - px) * (x - px) + py * py) / (2.0 * py);
        out.push(a + u * x + n * y);
    }
    out.push(q1);
    out
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let d = b - a;
    let l = d.length_sq();
    if l <= 0.0 {
        return p.distance(a);
    }
    let t = ((p - a).dot(d) / l).clamp(0.0, 1.0);
    p.distance(a + d * t)
}

/// The winding number of the rings round `p`.
fn winding(rings: &[Vec<[i32; 2]>], p: Point) -> i32 {
    let mut w = 0;
    for r in rings {
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            let (ax, ay, bx, by) = (f64::from(a[0]), f64::from(a[1]), f64::from(b[0]), f64::from(b[1]));
            let left = (bx - ax) * (p.y - ay) - (p.x - ax) * (by - ay);
            if ay <= p.y {
                if by > p.y && left > 0.0 {
                    w += 1;
                }
            } else if by <= p.y && left < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

/// True if two segments cross, touch or overlap anywhere but at a shared
/// end point.
fn crosses(points: &[[i32; 2]], segs: &[[usize; 2]]) -> bool {
    let x = |i: usize| points[i][0];
    let lo = |s: usize| x(segs[s][0]).min(x(segs[s][1]));
    let hi = |s: usize| x(segs[s][0]).max(x(segs[s][1]));
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by_key(|&s| (lo(s), s));
    let mut active: Vec<usize> = Vec::new();
    for s in order {
        let start = lo(s);
        active.retain(|&t| hi(t) >= start);
        if active.iter().any(|&t| meet(points, segs[s], segs[t])) {
            return true;
        }
        active.push(s);
    }
    false
}

fn orient(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> i64 {
    let (ax, ay) = (i64::from(a[0]), i64::from(a[1]));
    let (bx, by) = (i64::from(b[0]), i64::from(b[1]));
    let (cx, cy) = (i64::from(c[0]), i64::from(c[1]));
    ((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)).signum()
}

/// `c` is on the segment from `a` to `b` (it is on the line through them).
fn within(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> bool {
    a[0].min(b[0]) <= c[0] && c[0] <= a[0].max(b[0]) && a[1].min(b[1]) <= c[1] && c[1] <= a[1].max(b[1])
}

/// True if two segments meet anywhere but at a shared end point.
fn meet(points: &[[i32; 2]], s: [usize; 2], t: [usize; 2]) -> bool {
    let shared: Vec<usize> = s.iter().copied().filter(|i| t.contains(i)).collect();
    let (a, b, c, d) = (points[s[0]], points[s[1]], points[t[0]], points[t[1]]);
    match shared.len() {
        0 => {
            let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
            if o1 != o2 && o3 != o4 && o1 * o2 <= 0 && o3 * o4 <= 0 && (o1 != 0 || o2 != 0) {
                return true;
            }
            (o1 == 0 && within(a, b, c))
                || (o2 == 0 && within(a, b, d))
                || (o3 == 0 && within(c, d, a))
                || (o4 == 0 && within(c, d, b))
        }
        1 => {
            // Collinear and on the same side of the shared point: they overlap.
            let o = shared[0];
            let p = if s[0] == o { b } else { a };
            let q = if t[0] == o { d } else { c };
            let m = points[o];
            orient(m, p, q) == 0
                && i64::from(p[0] - m[0]) * i64::from(q[0] - m[0]) + i64::from(p[1] - m[1]) * i64::from(q[1] - m[1]) > 0
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    #[test]
    fn a_bar_has_one_middle_line_on_its_middle() {
        let m = middle_line(&[rect(0.0, 0.0, 1000.0, 100.0)], 5.0).expect("a diagram");
        assert_eq!(m.edges.len(), 1, "{m:?}");
        let e = &m.edges[0];
        assert!(!e.closed);
        for p in &e.pts {
            assert!((p.y - 50.0).abs() < 1e-3, "{p:?}");
        }
        // The middle line of a bar ends where the lines from its corners
        // meet: a half-width from each end.
        let xs: Vec<f64> = e.pts.iter().map(|p| p.x).collect();
        let (lo, hi) = (
            xs.iter().copied().fold(f64::MAX, f64::min),
            xs.iter().copied().fold(f64::MIN, f64::max),
        );
        assert!((lo - 50.0).abs() < 1e-3 && (hi - 950.0).abs() < 1e-3, "{lo} {hi}");
        for &(_, r) in &m.nodes {
            assert!((r - 50.0).abs() < 1e-3, "{r}");
        }
    }

    #[test]
    fn a_ring_has_a_closed_middle_line() {
        let circle = |r: f64, hole: bool| -> Vec<Point> {
            let mut v: Vec<Point> = (0..96)
                .map(|k| {
                    let a = emb_geom::math::to_radians(f64::from(k) * 3.75);
                    Point::new(r * emb_geom::math::cos(a), r * emb_geom::math::sin(a))
                })
                .collect();
            if hole {
                v.reverse();
            }
            v
        };
        let m = middle_line(&[circle(350.0, false), circle(250.0, true)], 5.0).expect("a diagram");
        let closed: Vec<&MedialEdge> = m.edges.iter().filter(|e| e.closed).collect();
        assert_eq!(closed.len(), 1, "{m:?}");
        for p in &closed[0].pts {
            assert!((p.length() - 300.0).abs() < 2.0, "{p:?}");
        }
    }

    #[test]
    fn a_cross_has_a_junction_with_four_edges() {
        // A plus sign: one ring.
        let ring = vec![
            Point::new(450.0, 0.0),
            Point::new(550.0, 0.0),
            Point::new(550.0, 450.0),
            Point::new(1000.0, 450.0),
            Point::new(1000.0, 550.0),
            Point::new(550.0, 550.0),
            Point::new(550.0, 1000.0),
            Point::new(450.0, 1000.0),
            Point::new(450.0, 550.0),
            Point::new(0.0, 550.0),
            Point::new(0.0, 450.0),
            Point::new(450.0, 450.0),
        ];
        let m = middle_line(&[ring], 5.0).expect("a diagram");
        let mut count: HashMap<usize, usize> = HashMap::new();
        for n in m.edges.iter().flat_map(|e| e.ends).filter(|&n| n != NONE) {
            *count.entry(n).or_default() += 1;
        }
        // Four line ends, and the junction at the middle (one node, or
        // nodes at the same place).
        assert_eq!(count.values().filter(|&&c| c == 1).count(), 4, "{m:?}");
        let middle = Point::new(500.0, 500.0);
        let junction: Vec<usize> = count
            .keys()
            .copied()
            .filter(|&n| count[&n] >= 3 && m.nodes[n].0.distance(middle) < 100.0)
            .collect();
        assert!(!junction.is_empty(), "{m:?}");
        for n in junction {
            let (p, r) = m.nodes[n];
            assert!(
                p.distance(middle) < 1e-3 && (r - 50.0 * std::f64::consts::SQRT_2).abs() < 1e-3,
                "{p:?} {r}"
            );
        }
    }

    #[test]
    fn rings_that_cross_have_no_diagram() {
        assert!(middle_line(&[rect(0.0, 0.0, 100.0, 100.0), rect(50.0, 50.0, 150.0, 150.0)], 5.0).is_none());
    }
}
