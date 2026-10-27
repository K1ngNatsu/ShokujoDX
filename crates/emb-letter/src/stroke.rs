//! Satin columns from the strokes of a glyph shape (our own method; see
//! ADR 0015 and `docs/design/p4-plan.md`).
//!
//! 1. Fill a grid of about [`GRID_CELLS`] cells for the capital height, and
//!    find the distance of each cell to the edge.
//! 2. Thin the shape to its middle lines and make a graph: nodes at line ends
//!    and junctions, edges along the lines.
//! 3. Remove short side branches (from corners and serifs), join the nodes
//!    that are very near, and at each junction pair the two edges that
//!    continue most straight. Paired edges make one stroke.
//! 4. Smooth each stroke, cut it at sharp corners, and extend free ends to
//!    the edge of the shape. A stroke that ends where another stroke goes
//!    through (the stem of a "T") ends a little inside the edge of that
//!    stroke, not at its middle, so the thread does not build up at the
//!    join (P11 Milestone 2; the density map of ADR 0044 showed 6 to 9
//!    layers there).
//! 5. At points along each stroke, lines across the stroke meet the outline:
//!    these points are the rails of a satin column.
//!
//! A part of the shape with almost no middle line (a dot) gets one stroke
//! along its long axis.

use std::collections::HashMap;

use emb_geom::{BBox, Path, Point};
use emb_model::SatinGeometry;

use crate::grid::{Grid, distance, fill_into, parts, thin};

/// Grid cells for the capital height.
pub const GRID_CELLS: f64 = 200.0;
/// A branch from a line end to a junction is removed when it is shorter
/// than this many times the stroke half-width at the junction.
const PRUNE: f64 = 1.6;
/// A branch is a side branch only if it stays inside this many
/// half-widths of the junction (counting its own half-width): a branch to
/// a corner does, a short thick arm (the bar of a condensed "T") does not.
const SPUR_REACH: f64 = 1.5;
/// Junctions nearer than this many half-widths are one junction.
const JOIN: f64 = 0.8;
/// Two edges at a junction continue as one stroke when the angle between
/// them is at least this (degrees).
const STRAIGHT_DEG: f64 = 140.0;
/// A stroke is cut where it turns more than this (degrees) over about
/// three half-widths.
const CORNER_DEG: f64 = 70.0;
/// Rail points are at most this many half-widths from the middle line.
const MAX_REACH: f64 = 1.6;
/// A stroke that ends where another stroke goes through goes into it this
/// part of its half-width (from its edge), so there is no gap at the join.
const JOIN_OVERLAP: f64 = 0.3;

const NONE: usize = usize::MAX;

/// Satin columns for a glyph, with checks of how well they fit it.
#[derive(Debug, Clone, Default)]
pub struct Strokes {
    pub satins: Vec<SatinGeometry>,
    /// The part of the shape that the columns cover, 0 to 1.
    pub coverage: f64,
    /// The widest column, in font units.
    pub max_width: f64,
    /// The middle width of the columns (over their rungs), in font units.
    pub typical_width: f64,
}

#[derive(Debug, Clone)]
struct Node {
    pos: Point,
    /// Half-width of the shape at the node.
    r: f64,
}

#[derive(Debug, Clone)]
struct Edge {
    /// Node at the start and at the end (`NONE` for a closed edge).
    ends: [usize; 2],
    pts: Vec<Point>,
    closed: bool,
    alive: bool,
}

/// One end of an edge: (edge, 0 for the start or 1 for the end).
type Half = (usize, usize);

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

fn length(p: &[Point]) -> f64 {
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

impl Graph {
    fn build(g: &Grid, skel: &[bool], radius: &dyn Fn(usize) -> f64) -> Graph {
        let count: Vec<usize> = (0..skel.len())
            .map(|k| {
                if skel[k] {
                    g.neighbors(k).filter(|&q| skel[q]).count()
                } else {
                    0
                }
            })
            .collect();
        let is_node = |k: usize| skel[k] && count[k] != 2;
        // Nodes: line ends alone; junction cells that touch are one node.
        let mut node_of = vec![NONE; skel.len()];
        let mut nodes: Vec<Node> = Vec::new();
        for k in 0..skel.len() {
            if !is_node(k) || node_of[k] != NONE {
                continue;
            }
            let id = nodes.len();
            let mut cluster = vec![k];
            node_of[k] = id;
            if count[k] >= 3 {
                let mut i = 0;
                while i < cluster.len() {
                    let c = cluster[i];
                    for q in g.neighbors(c) {
                        if is_node(q) && count[q] >= 3 && node_of[q] == NONE {
                            node_of[q] = id;
                            cluster.push(q);
                        }
                    }
                    i += 1;
                }
            }
            let n = cluster.len() as f64;
            let pos = cluster.iter().fold(Point::new(0.0, 0.0), |a, &c| a + g.center_of(c)) * (1.0 / n);
            let r = cluster.iter().map(|&c| radius(c)).fold(0.0, f64::max);
            nodes.push(Node { pos, r });
        }
        let mut edges = Vec::new();
        let mut visited = vec![false; skel.len()];
        let mut direct: Vec<(usize, usize)> = Vec::new();
        for k in 0..skel.len() {
            if !is_node(k) {
                continue;
            }
            for q in g.neighbors(k).collect::<Vec<_>>() {
                if !skel[q] {
                    continue;
                }
                if is_node(q) {
                    let (a, b) = (node_of[k], node_of[q]);
                    let key = (a.min(b), a.max(b));
                    if a != b && !direct.contains(&key) {
                        direct.push(key);
                        edges.push(Edge {
                            ends: [a, b],
                            pts: vec![g.center_of(k), g.center_of(q)],
                            closed: false,
                            alive: true,
                        });
                    }
                    continue;
                }
                if visited[q] {
                    continue;
                }
                let mut pts = vec![g.center_of(k), g.center_of(q)];
                visited[q] = true;
                let (mut prev, mut cur) = (k, q);
                let mut end = NONE;
                loop {
                    let next = g
                        .neighbors(cur)
                        .find(|&x| skel[x] && x != prev && (is_node(x) || !visited[x]));
                    match next {
                        Some(x) if is_node(x) => {
                            pts.push(g.center_of(x));
                            end = node_of[x];
                            break;
                        }
                        Some(x) => {
                            visited[x] = true;
                            pts.push(g.center_of(x));
                            (prev, cur) = (cur, x);
                        }
                        None => break,
                    }
                }
                if end == NONE {
                    // A dead end (should not occur): the line ends here.
                    end = nodes.len();
                    nodes.push(Node {
                        pos: *pts.last().unwrap_or(&pts[0]),
                        r: radius(cur),
                    });
                }
                edges.push(Edge {
                    ends: [node_of[k], end],
                    pts,
                    closed: false,
                    alive: true,
                });
            }
        }
        // Loops with no node.
        for k in 0..skel.len() {
            if !skel[k] || is_node(k) || visited[k] {
                continue;
            }
            visited[k] = true;
            let mut pts = vec![g.center_of(k)];
            let (mut prev, mut cur) = (NONE, k);
            while let Some(x) = g.neighbors(cur).find(|&x| skel[x] && x != prev && !visited[x]) {
                visited[x] = true;
                pts.push(g.center_of(x));
                (prev, cur) = (cur, x);
            }
            if pts.len() >= 3 {
                edges.push(Edge {
                    ends: [NONE, NONE],
                    pts,
                    closed: true,
                    alive: true,
                });
            }
        }
        Graph { nodes, edges }
    }

    fn halves(&self, n: usize) -> Vec<Half> {
        let mut v = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            if !e.alive || e.closed {
                continue;
            }
            for k in 0..2 {
                if e.ends[k] == n {
                    v.push((i, k));
                }
            }
        }
        v
    }

    fn degree(&self, n: usize) -> usize {
        self.halves(n).len()
    }

    /// The points of an edge from end `k` to the other end.
    fn points_from(&self, (e, k): Half) -> Vec<Point> {
        let mut p = self.edges[e].pts.clone();
        if k == 1 {
            p.reverse();
        }
        p
    }

    /// Removes short branches from line ends to junctions.
    fn prune(&mut self, radius_at: &dyn Fn(Point) -> f64) -> bool {
        let mut changed = false;
        for i in 0..self.edges.len() {
            let e = &self.edges[i];
            if !e.alive || e.closed {
                continue;
            }
            let (a, b) = (e.ends[0], e.ends[1]);
            let (da, db) = (self.degree(a), self.degree(b));
            let junction = match (da, db) {
                (1, d) if d >= 3 => b,
                (d, 1) if d >= 3 => a,
                _ => continue,
            };
            let (pos, r) = (self.nodes[junction].pos, self.nodes[junction].r);
            let reach = self.edges[i]
                .pts
                .iter()
                .map(|&p| p.distance(pos) + radius_at(p))
                .fold(0.0, f64::max);
            if length(&self.edges[i].pts) < PRUNE * r && reach < SPUR_REACH * r {
                self.edges[i].alive = false;
                changed = true;
            }
        }
        changed
    }

    /// Joins the two edges at each node with two edges.
    fn merge(&mut self) {
        loop {
            let Some((_, h)) = (0..self.nodes.len())
                .map(|n| (n, self.halves(n)))
                .find(|(_, h)| h.len() == 2)
            else {
                return;
            };
            let ((e1, k1), (e2, k2)) = (h[0], h[1]);
            if e1 == e2 {
                // A loop through this node only.
                let e = &mut self.edges[e1];
                e.closed = true;
                e.ends = [NONE, NONE];
                if e.pts.len() > 2 && e.pts.first().unwrap().distance(*e.pts.last().unwrap()) < 1e-9 {
                    e.pts.pop();
                }
                continue;
            }
            let mut pts = self.points_from((e1, k1));
            pts.reverse();
            pts.extend(self.points_from((e2, k2)).into_iter().skip(1));
            let ends = [self.edges[e1].ends[1 - k1], self.edges[e2].ends[1 - k2]];
            self.edges[e1].alive = false;
            self.edges[e2].alive = false;
            self.edges.push(Edge {
                ends,
                pts,
                closed: false,
                alive: true,
            });
        }
    }

    /// Makes one junction of two junctions that a short edge joins.
    fn join_near(&mut self) -> bool {
        for i in 0..self.edges.len() {
            let e = &self.edges[i];
            if !e.alive || e.closed || e.ends[0] == e.ends[1] {
                continue;
            }
            let (a, b) = (e.ends[0], e.ends[1]);
            if self.degree(a) < 3 || self.degree(b) < 3 {
                continue;
            }
            let r = self.nodes[a].r.max(self.nodes[b].r);
            if length(&e.pts) < JOIN * r {
                self.edges[i].alive = false;
                self.nodes[a].pos = self.nodes[a].pos.midpoint(self.nodes[b].pos);
                self.nodes[a].r = r;
                for e in self.edges.iter_mut().filter(|e| e.alive) {
                    for end in &mut e.ends {
                        if *end == b {
                            *end = a;
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    /// The direction in which a half-edge leaves its node.
    fn direction(&self, h: Half) -> Option<Point> {
        let pts = self.points_from(h);
        let node = &self.nodes[self.edges[h.0].ends[h.1]];
        let want = (2.0 * node.r).min(0.5 * length(&pts)).max(1e-9);
        let mut d = 0.0;
        let mut at = *pts.last()?;
        for w in pts.windows(2) {
            d += w[0].distance(w[1]);
            if d >= want {
                at = w[1];
                break;
            }
        }
        (at - node.pos).normalized().or_else(|| (at - pts[0]).normalized())
    }

    /// At each junction, links the pairs of half-edges that continue most straight.
    fn links(&self) -> HashMap<Half, Half> {
        let mut link = HashMap::new();
        let cos_min = emb_geom::math::cos(emb_geom::math::to_radians(STRAIGHT_DEG));
        for n in 0..self.nodes.len() {
            let hs = self.halves(n);
            if hs.len() < 3 {
                continue;
            }
            let dirs: Vec<Option<Point>> = hs.iter().map(|&h| self.direction(h)).collect();
            let mut pairs = Vec::new();
            for i in 0..hs.len() {
                for j in i + 1..hs.len() {
                    if let (Some(a), Some(b)) = (dirs[i], dirs[j]) {
                        let c = a.dot(b);
                        if c <= cos_min {
                            pairs.push((c, i, j));
                        }
                    }
                }
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut used = vec![false; hs.len()];
            for (_, i, j) in pairs {
                if !used[i] && !used[j] && hs[i].0 != hs[j].0 {
                    used[i] = true;
                    used[j] = true;
                    link.insert(hs[i], hs[j]);
                    link.insert(hs[j], hs[i]);
                }
            }
        }
        link
    }

    /// The strokes: chains of linked edges, with a flag for each end that is
    /// a free line end (not a junction).
    fn chains(&self) -> Vec<Raw> {
        let link = self.links();
        let mut done = vec![false; self.edges.len()];
        let mut out = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            if !e.alive || done[i] {
                continue;
            }
            if e.closed {
                done[i] = true;
                out.push(Raw {
                    pts: e.pts.clone(),
                    closed: true,
                    free: [false, false],
                    meets: [false, false],
                    r_fix: None,
                });
                continue;
            }
            // Walk back from the start of edge i to the start of the chain.
            let mut h: Half = (i, 0);
            let mut closed = false;
            let mut guard = 0;
            while let Some(&(f, kf)) = link.get(&h) {
                if f == i {
                    closed = true;
                    break;
                }
                h = (f, 1 - kf);
                guard += 1;
                if guard > self.edges.len() {
                    break;
                }
            }
            // `h` is the free end of the chain: enter there.
            let (mut e, mut k) = if closed { (i, 0) } else { h };
            let start_node = self.edges[e].ends[k];
            let mut pts: Vec<Point> = Vec::new();
            let mut end_node;
            loop {
                done[e] = true;
                let p = self.points_from((e, k));
                if pts.is_empty() {
                    pts = p;
                } else {
                    pts.extend(p.into_iter().skip(1));
                }
                end_node = self.edges[e].ends[1 - k];
                match link.get(&(e, 1 - k)) {
                    Some(&(f, kf)) if !done[f] => (e, k) = (f, kf),
                    _ => break,
                }
            }
            let free = |n: usize| n != NONE && self.degree(n) == 1;
            // Another stroke goes through the node: two of its edges are linked there.
            let through = |n: usize| n != NONE && link.keys().any(|&(f, kf)| self.edges[f].ends[kf] == n);
            out.push(Raw {
                closed,
                free: [!closed && free(start_node), !closed && free(end_node)],
                meets: [!closed && through(start_node), !closed && through(end_node)],
                pts,
                r_fix: None,
            });
        }
        out
    }
}

/// A stroke before smoothing.
#[derive(Debug, Clone)]
struct Raw {
    pts: Vec<Point>,
    closed: bool,
    /// The start and the end are free line ends (to extend to the edge).
    free: [bool; 2],
    /// The start and the end are where another stroke goes through.
    meets: [bool; 2],
    /// A fixed half-width (for the long axis of a dot).
    r_fix: Option<f64>,
}

/// The point of a line nearest to `p`, and its distance.
fn nearest(line: &[Point], p: Point) -> (Point, f64) {
    let mut best = (line[0], line[0].distance(p));
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ab = b - a;
        let len2 = ab.dot(ab);
        let t = if len2 > 0.0 {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + ab * t;
        let d = q.distance(p);
        if d < best.1 {
            best = (q, d);
        }
    }
    best
}

/// Cuts the start of a stroke that lies in other strokes, up to the point
/// where it is out of them but [`JOIN_OVERLAP`] of their half-width: a point
/// is in a stroke while it is nearer to its middle line than
/// `(1 − JOIN_OVERLAP)` times its half-width (the one given, else the
/// half-width at the nearest point). The cut is after the last point in the
/// first half of the stroke that is in another stroke (an extended end can
/// go across the other stroke and out again), so a short stroke keeps most
/// of its length. Returns the number of points that went.
fn trim_start_out_of(
    pts: &mut Vec<Point>,
    others: &[(&[Point], Option<f64>)],
    radius_at: &dyn Fn(Point) -> f64,
    step: f64,
) -> usize {
    if others.is_empty() || pts.len() < 2 {
        return 0;
    }
    let inside = |p: Point| {
        others.iter().any(|(l, r)| {
            let (q, d) = nearest(l, p);
            d < (1.0 - JOIN_OVERLAP) * r.unwrap_or_else(|| radius_at(q))
        })
    };
    let half = 0.5 * length(pts);
    let step = step.max(1e-9);
    // Samples of the first half: the segment, the point and whether it is in.
    let mut samples: Vec<(usize, Point, bool)> = vec![(0, pts[0], inside(pts[0]))];
    let mut walked = 0.0;
    'walk: for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let d = a.distance(b);
        let n = (d / step).ceil().max(1.0) as usize;
        for s in 1..=n {
            let t = s as f64 / n as f64;
            if walked + d * t > half {
                break 'walk;
            }
            let p = a.lerp(b, t);
            samples.push((k, p, inside(p)));
        }
        walked += d;
    }
    let Some(last_in) = samples.iter().rposition(|s| s.2) else {
        return 0;
    };
    let Some(&(k, p, _)) = samples.get(last_in + 1) else {
        // In other strokes up to the middle: keep the stroke.
        return 0;
    };
    pts.drain(..=k);
    pts.insert(0, p);
    k
}

/// Moving average over `win` points on each side; open lines keep their ends.
fn smooth(p: &[Point], closed: bool, win: usize) -> Vec<Point> {
    let n = p.len();
    if n < 3 || win == 0 {
        return p.to_vec();
    }
    (0..n)
        .map(|i| {
            if !closed && (i == 0 || i == n - 1) {
                return p[i];
            }
            let mut sum = Point::new(0.0, 0.0);
            let mut c = 0.0;
            for d in -(win as isize)..=win as isize {
                let j = i as isize + d;
                let j = if closed {
                    j.rem_euclid(n as isize) as usize
                } else if j < 0 || j >= n as isize {
                    continue;
                } else {
                    j as usize
                };
                sum += p[j];
                c += 1.0;
            }
            sum * (1.0 / c)
        })
        .collect()
}

/// Points about `step` apart along the line (the ends stay).
fn resample(p: &[Point], closed: bool, step: f64) -> Vec<Point> {
    let mut pts = p.to_vec();
    if closed && let Some(&f) = pts.first() {
        pts.push(f);
    }
    let total = length(&pts);
    if total < 1e-9 || pts.len() < 2 {
        return p.to_vec();
    }
    let n = (total / step).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(n + 1);
    let (mut seg, mut acc) = (0usize, 0.0);
    for q in 0..=n {
        let want = total * q as f64 / n as f64;
        while seg + 1 < pts.len() - 1 && acc + pts[seg].distance(pts[seg + 1]) < want {
            acc += pts[seg].distance(pts[seg + 1]);
            seg += 1;
        }
        let l = pts[seg].distance(pts[seg + 1]);
        let t = if l > 0.0 {
            ((want - acc) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(pts[seg].lerp(pts[seg + 1], t));
    }
    if closed {
        out.pop();
    }
    out
}

/// The distance along `dir` from `o` to the nearest outline segment, up to `max`.
fn ray(o: Point, dir: Point, max: f64, segs: &[(Point, Point)]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for &(a, b) in segs {
        let ab = b - a;
        let den = dir.cross(ab);
        if den.abs() < 1e-12 {
            continue;
        }
        let ao = a - o;
        let t = ao.cross(ab) / den;
        let u = ao.cross(dir) / den;
        if t > 1e-9 && t <= max && (-1e-9..=1.0 + 1e-9).contains(&u) && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    }
    best
}

/// One stroke after smoothing and cutting, ready for rails.
struct Piece {
    pts: Vec<Point>,
    closed: bool,
    /// Points from this index on (and before `ext[0]`) are extensions past
    /// the middle line: they take the width of the last point on it.
    ext: [usize; 2],
    /// Near an end at a junction, the width is held for this length (the
    /// lines across the stroke would reach into the other stroke there).
    hold: [f64; 2],
}

/// Cuts a stroke where it turns more than [`CORNER_DEG`].
/// Returns the point indices of each piece.
fn cut_corners(p: &[Point], closed: bool, free: [bool; 2], reach: f64) -> Vec<(Vec<usize>, bool, [bool; 2])> {
    let n = p.len();
    if n < 5 {
        return vec![((0..n).collect(), closed, free)];
    }
    let spacing = length(p) / (n - 1) as f64;
    let k = ((reach / spacing.max(1e-9)).round() as usize).clamp(1, n / 4 + 1);
    let cos_max = emb_geom::math::cos(emb_geom::math::to_radians(CORNER_DEG));
    let at = |i: isize| -> Point { p[i.rem_euclid(n as isize) as usize] };
    let mut turn = vec![1.0f64; n];
    for (i, t) in turn.iter_mut().enumerate() {
        let i = i as isize;
        if !closed && (i < k as isize || i + (k as isize) >= n as isize) {
            continue;
        }
        let (a, b) = (at(i) - at(i - k as isize), at(i + k as isize) - at(i));
        if let (Some(a), Some(b)) = (a.normalized(), b.normalized()) {
            *t = a.dot(b);
        }
    }
    // Local minima of the cosine below the limit, at least 2k apart.
    let mut cuts: Vec<usize> = Vec::new();
    let mut order: Vec<usize> = (0..n).filter(|&i| turn[i] < cos_max).collect();
    order.sort_by(|&a, &b| turn[a].total_cmp(&turn[b]));
    for i in order {
        let near = cuts.iter().any(|&c| {
            let d = c.abs_diff(i);
            let d = if closed { d.min(n - d) } else { d };
            d < 2 * k
        });
        if !near {
            cuts.push(i);
        }
    }
    if cuts.is_empty() {
        return vec![((0..n).collect(), closed, free)];
    }
    cuts.sort_unstable();
    let mut out = Vec::new();
    if closed {
        for w in 0..cuts.len() {
            let (s, e) = (cuts[w], cuts[(w + 1) % cuts.len()]);
            let mut seg = Vec::new();
            let mut i = s;
            loop {
                seg.push(i);
                if i == e && seg.len() > 1 {
                    break;
                }
                i = (i + 1) % n;
            }
            out.push((seg, false, [true, true]));
        }
    } else {
        let mut s = 0;
        for (w, &c) in cuts.iter().enumerate() {
            out.push(((s..=c).collect(), false, [if w == 0 { free[0] } else { true }, true]));
            s = c;
        }
        out.push(((s..n).collect(), false, [true, free[1]]));
    }
    out
}

/// Extends free ends along their direction to the edge of the shape.
fn extend(mut pts: Vec<Point>, free: [bool; 2], g: &Grid, reach: f64) -> Piece {
    let n0 = pts.len();
    let step = g.cell * 0.5;
    let back = |p: &[Point], from_end: bool| -> Option<Point> {
        let n = p.len();
        if n < 2 {
            return None;
        }
        let k = (n / 3).max(1);
        if from_end {
            (p[n - 1] - p[n - 1 - k]).normalized()
        } else {
            (p[0] - p[k]).normalized()
        }
    };
    let walk = |from: Point, dir: Point| -> Vec<Point> {
        let mut v = Vec::new();
        let mut s = step;
        while s < 4.0 * reach && g.is_inside(from + dir * s) {
            s += step;
        }
        let last = from + dir * (s - 0.5 * step);
        if last.distance(from) > step {
            v.push(last);
        }
        v
    };
    let mut ext = [0, n0];
    if free[1]
        && let Some(d) = back(&pts, true)
    {
        let add = walk(pts[n0 - 1], d);
        pts.extend(add);
    }
    if free[0]
        && let Some(d) = back(&pts[..n0], false)
    {
        let add = walk(pts[0], d);
        ext = [add.len(), ext[1] + add.len()];
        let mut v = add;
        v.reverse();
        v.extend(pts);
        pts = v;
    }
    Piece {
        pts,
        closed: false,
        ext,
        hold: [0.0; 2],
    }
}

/// Satin columns for a glyph shape (rings in font units, Y down).
pub fn strokes(rings: &[Vec<Point>], cap_height: f64) -> Strokes {
    let Some(bb) = BBox::from_points(rings.iter().flatten().copied()) else {
        return Strokes::default();
    };
    let cell = (cap_height / GRID_CELLS).max(1e-6);
    let mut g = Grid::new(bb, cell, 3);
    g.fill(rings);
    let inside_count = g.inside.iter().filter(|&&v| v).count();
    if inside_count == 0 {
        return Strokes::default();
    }
    let dist = distance(&g);
    // Half-width at a cell: the distance to the edge, not to the next outside center.
    let radius = |k: usize| (dist[k] - 0.5).max(0.5) * cell;
    let radius_at = |p: Point| g.cell_of(p).map_or(0.5 * cell, |(i, j)| radius(g.idx(i, j)));
    let skel = thin(&g);
    let mut graph = Graph::build(&g, &skel, &radius);
    loop {
        let pruned = graph.prune(&radius_at);
        graph.merge();
        let joined = graph.join_near();
        if !pruned && !joined {
            break;
        }
    }
    let raws = graph.chains();

    // Parts of the shape with almost no middle line get a stroke along their long axis.
    let (label, count) = parts(&g);
    let mut part_len = vec![0.0; count];
    let mut part_r = vec![0.0f64; count];
    for (k, &l) in label.iter().enumerate() {
        if l != NONE {
            part_r[l] = part_r[l].max(radius(k));
        }
    }
    let part_of = |p: Point| g.cell_of(p).map(|(i, j)| label[g.idx(i, j)]).filter(|&l| l != NONE);
    for r in &raws {
        if let Some(l) = r.pts.iter().find_map(|&p| part_of(p)) {
            part_len[l] += length(&r.pts);
        }
    }
    let mut raws: Vec<Raw> = raws
        .into_iter()
        .filter(|r| {
            r.pts
                .iter()
                .find_map(|&p| part_of(p))
                .is_none_or(|l| part_len[l] >= 1.2 * part_r[l])
        })
        .collect();
    for l in 0..count {
        if part_len[l] < 1.2 * part_r[l] {
            raws.push(long_axis(&g, &label, l, part_r[l]));
        }
    }

    let segs: Vec<(Point, Point)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |k| (r[k], r[(k + 1) % r.len()])))
        .collect();
    let mut out = Strokes::default();
    let mut cover = vec![false; g.inside.len()];
    let lines: Vec<Vec<Point>> = raws.iter().map(|r| r.pts.clone()).collect();
    // The pieces of all strokes: the stroke, its fixed half-width, its
    // middle half-width and the piece.
    let mut all: Vec<(usize, Option<f64>, f64, Piece)> = Vec::new();
    for (i, mut raw) in raws.into_iter().enumerate() {
        // An end where another stroke goes through: from the middle of that
        // stroke back to a little inside its edge.
        for end in 0..2 {
            if !raw.meets[end] || raw.r_fix.is_some() {
                continue;
            }
            let at = if end == 0 {
                raw.pts[0]
            } else {
                raw.pts[raw.pts.len() - 1]
            };
            let r = radius_at(at);
            // The strokes that go through the node (not the ones that end there).
            let through: Vec<(&[Point], Option<f64>)> = lines
                .iter()
                .enumerate()
                .filter(|(j, l)| {
                    *j != i
                        && l.len() >= 2
                        && nearest(l, at).1 < r
                        && l[0].distance(at) > r
                        && l[l.len() - 1].distance(at) > r
                })
                .map(|(_, l)| (l.as_slice(), None))
                .collect();
            if end == 1 {
                raw.pts.reverse();
            }
            trim_start_out_of(&mut raw.pts, &through, &radius_at, cell);
            if end == 1 {
                raw.pts.reverse();
            }
        }
        let fixed = |_: Point| raw.r_fix.unwrap_or(0.0);
        let width_at: &dyn Fn(Point) -> f64 = if raw.r_fix.is_some() { &fixed } else { &radius_at };
        let mut sorted: Vec<f64> = raw.pts.iter().map(|&p| width_at(p)).collect();
        sorted.sort_by(f64::total_cmp);
        let r_med = sorted.get(sorted.len() / 2).copied().unwrap_or(cell).max(cell);
        let win = ((0.6 * r_med / cell).round() as usize).max(1);
        // Find the corners on a smoothed line, then smooth each piece alone,
        // so that the pieces stay straight up to the corner.
        let smoothed = smooth(&raw.pts, raw.closed, win);
        let mut pieces: Vec<Piece> = cut_corners(&smoothed, raw.closed, raw.free, 1.5 * r_med)
            .into_iter()
            .map(|(idx, closed, free)| {
                let p: Vec<Point> = idx.iter().map(|&i| raw.pts[i]).collect();
                let p = smooth(&p, closed, win);
                if closed {
                    let n = p.len();
                    Piece {
                        pts: p,
                        closed,
                        ext: [0, n],
                        hold: [0.0; 2],
                    }
                } else {
                    let mut piece = extend(p, free, &g, r_med);
                    for (hold, free) in piece.hold.iter_mut().zip(free) {
                        if !free && raw.r_fix.is_none() {
                            *hold = 1.5 * r_med;
                        }
                    }
                    piece
                }
            })
            .collect();
        // At a corner the piece before keeps the corner, and this piece
        // starts where it comes out of it (the apex of an "A").
        let n = pieces.len();
        if raw.r_fix.is_none() && n >= 2 {
            let first = if raw.closed { 0 } else { 1 };
            for k in first..n {
                let before = pieces[(k + n - 1) % n].pts.clone();
                let piece = &mut pieces[k];
                if piece.closed || piece.ext[0] > 0 {
                    continue;
                }
                let gone = trim_start_out_of(&mut piece.pts, &[(&before, None)], &radius_at, cell);
                piece.ext[1] = piece.ext[1].saturating_sub(gone).max(1);
            }
        }
        all.extend(pieces.into_iter().map(|p| (i, raw.r_fix, r_med, p)));
    }
    // An end that was extended to the edge of the shape into an earlier
    // piece (the two legs of an "A" with a flat top): it stops a little
    // inside the edge of that piece.
    for k in 0..all.len() {
        let (_, fixed, _, ref piece) = all[k];
        if fixed.is_some() || piece.closed {
            continue;
        }
        // The pieces before this one (also of the same stroke: the legs of
        // an "A" are two pieces cut at the apex). The extended parts are
        // near the edge of the shape, where the half-width is small: the
        // middle half-width of the stroke counts.
        let earlier: Vec<(Vec<Point>, f64)> = all[..k]
            .iter()
            .filter(|(_, f, _, p)| f.is_none() && p.pts.len() >= 2)
            .map(|(_, _, r, p)| (p.pts.clone(), *r))
            .collect();
        let earlier: Vec<(&[Point], Option<f64>)> = earlier.iter().map(|(l, r)| (l.as_slice(), Some(*r))).collect();
        let piece = &mut all[k].3;
        if piece.ext[0] > 0 {
            let gone = trim_start_out_of(&mut piece.pts, &earlier, &radius_at, cell);
            piece.ext[0] = piece.ext[0].saturating_sub(gone);
            piece.ext[1] = piece.ext[1].saturating_sub(gone).max(piece.ext[0] + 1);
        }
        let n = piece.pts.len();
        if piece.ext[1] < n {
            piece.pts.reverse();
            let gone = trim_start_out_of(&mut piece.pts, &earlier, &radius_at, cell);
            piece.pts.reverse();
            // The tail extension had `n - ext[1]` points; `gone` of them went.
            let tail = (n - piece.ext[1]).saturating_sub(gone);
            piece.ext[1] = piece.pts.len().saturating_sub(tail).max(piece.ext[0] + 1);
        }
    }
    for (_, fixed, r_med, piece) in &all {
        let fixed_at = |_: Point| fixed.unwrap_or(0.0);
        let width_at: &dyn Fn(Point) -> f64 = if fixed.is_some() { &fixed_at } else { &radius_at };
        if let Some(s) = rails(piece, *r_med, width_at, &segs, cell) {
            out.max_width = out.max_width.max(s.1);
            for q in &s.2 {
                fill_into(&g, std::slice::from_ref(q), &mut cover);
            }
            out.satins.push(s.0);
        }
    }
    let covered = (0..cover.len()).filter(|&k| cover[k] && g.inside[k]).count();
    out.coverage = covered as f64 / inside_count as f64;
    let mut widths: Vec<f64> = out
        .satins
        .iter()
        .flat_map(|s| s.rungs.iter().map(|(a, b)| a.distance(*b)))
        .collect();
    widths.sort_by(f64::total_cmp);
    out.typical_width = widths.get(widths.len() / 2).copied().unwrap_or(out.max_width);
    out
}

/// A stroke through a part of the shape along its long axis (for dots).
fn long_axis(g: &Grid, label: &[usize], l: usize, r: f64) -> Raw {
    let cells: Vec<Point> = (0..label.len())
        .filter(|&k| label[k] == l)
        .map(|k| g.center_of(k))
        .collect();
    let n = cells.len() as f64;
    let c = cells.iter().fold(Point::new(0.0, 0.0), |a, &p| a + p) * (1.0 / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in &cells {
        let d = *p - c;
        sxx += d.x * d.x;
        syy += d.y * d.y;
        sxy += d.x * d.y;
    }
    let angle = 0.5 * emb_geom::math::atan2(2.0 * sxy, sxx - syy);
    let u = Point::new(emb_geom::math::cos(angle), emb_geom::math::sin(angle));
    let (lo, hi) = cells
        .iter()
        .map(|p| (*p - c).dot(u))
        .fold((f64::MAX, f64::MIN), |(a, b), t| (a.min(t), b.max(t)));
    let (a, b) = (c + u * lo, c + u * hi);
    let steps = ((hi - lo) / g.cell).ceil().max(2.0) as usize;
    Raw {
        pts: (0..=steps).map(|i| a.lerp(b, i as f64 / steps as f64)).collect(),
        closed: false,
        free: [false, false],
        meets: [false, false],
        r_fix: Some(r),
    }
}

/// The satin column of a piece, its widest width and the quads it covers.
fn rails(
    piece: &Piece,
    r_med: f64,
    radius_at: &dyn Fn(Point) -> f64,
    segs: &[(Point, Point)],
    cell: f64,
) -> Option<(SatinGeometry, f64, Vec<Vec<Point>>)> {
    let step = (0.7 * r_med).max(cell);
    // Resample the middle part and the extensions apart, so the extension
    // flags stay right.
    let mid = &piece.pts[piece.ext[0]..piece.ext[1].max(piece.ext[0])];
    if mid.len() < 2 && piece.pts.len() < 2 {
        return None;
    }
    let mut pts = Vec::new();
    let mut is_ext = Vec::new();
    if piece.ext[0] > 0 {
        let head = resample(&piece.pts[..=piece.ext[0]], false, step);
        let k = head.len() - 1;
        pts.extend_from_slice(&head[..k]);
        is_ext.extend(std::iter::repeat_n(true, k));
    }
    let body = if mid.len() >= 2 {
        resample(mid, piece.closed, step)
    } else {
        mid.to_vec()
    };
    let nb = body.len();
    pts.extend_from_slice(&body);
    is_ext.extend(std::iter::repeat_n(false, nb));
    if piece.ext[1] < piece.pts.len() {
        let tail = resample(&piece.pts[piece.ext[1] - 1..], false, step);
        pts.extend_from_slice(&tail[1..]);
        is_ext.extend(std::iter::repeat_n(true, tail.len() - 1));
    }
    let n = pts.len();
    if n < 2 {
        return None;
    }
    // Points near an end at a junction take the width of the first point
    // past the hold length (as extensions do).
    let mut along = vec![0.0; n];
    for i in 1..n {
        along[i] = along[i - 1] + pts[i - 1].distance(pts[i]);
    }
    let total = along[n - 1];
    let held: Vec<bool> = (0..n)
        .map(|i| along[i] < piece.hold[0] || total - along[i] < piece.hold[1])
        .collect();
    if (0..n).all(|i| is_ext[i] || held[i]) {
        // A short stroke: measure at the middle point only.
        let mid = n / 2;
        for (i, e) in is_ext.iter_mut().enumerate() {
            *e = i != mid;
        }
    } else {
        for i in 0..n {
            is_ext[i] |= held[i];
        }
    }
    let closed = piece.closed && n >= 3;
    let tangent = |i: usize| -> Point {
        let (a, b) = if closed {
            (pts[(i + n - 1) % n], pts[(i + 1) % n])
        } else {
            (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)])
        };
        (b - a).normalized().unwrap_or(Point::new(1.0, 0.0))
    };
    let cap = 1.3 * r_med;
    let mut half: Vec<(f64, f64)> = vec![(0.0, 0.0); n];
    for i in 0..n {
        if is_ext[i] {
            continue;
        }
        let nrm = tangent(i).perp();
        let r = radius_at(pts[i]).min(cap);
        let reach = MAX_REACH * r + cell;
        let hp = ray(pts[i], nrm, reach, segs).unwrap_or(r);
        let hm = ray(pts[i], -nrm, reach, segs).unwrap_or(r);
        half[i] = (hp.max(0.25 * cell), hm.max(0.25 * cell));
    }
    // Extensions take the width of the nearest middle point.
    let first = (0..n).find(|&i| !is_ext[i])?;
    let last = (0..n).rev().find(|&i| !is_ext[i]).unwrap_or(first);
    for i in 0..n {
        if is_ext[i] {
            // The held width, but not past the outline (a pointed end narrows).
            let (hp, hm) = if i < first { half[first] } else { half[last] };
            let nrm = tangent(i).perp();
            let hp = ray(pts[i], nrm, hp, segs).unwrap_or(hp);
            let hm = ray(pts[i], -nrm, hm, segs).unwrap_or(hm);
            half[i] = (hp.max(0.25 * cell), hm.max(0.25 * cell));
        }
    }
    let a: Vec<Point> = (0..n).map(|i| pts[i] + tangent(i).perp() * half[i].0).collect();
    let b: Vec<Point> = (0..n).map(|i| pts[i] - tangent(i).perp() * half[i].1).collect();
    let width = half.iter().map(|h| h.0 + h.1).fold(0.0, f64::max);
    let mut quads = Vec::new();
    let m = if closed { n } else { n - 1 };
    for i in 0..m {
        let j = (i + 1) % n;
        quads.push(vec![a[i], a[j], b[j], b[i]]);
    }
    let path = |p: &[Point]| {
        if closed {
            Path::polygon(p)
        } else {
            let mut path = Path::new().move_to(p[0]);
            for &q in &p[1..] {
                path = path.line_to(q);
            }
            path
        }
    };
    // A rung at every second point keeps the stitches across the stroke.
    let rungs = (1..n.saturating_sub(1)).step_by(2).map(|i| (a[i], b[i])).collect();
    Some((
        SatinGeometry {
            rail_a: path(&a),
            rail_b: path(&b),
            rungs,
        },
        width,
        quads,
    ))
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
    fn a_bar_is_one_column_that_covers_it() {
        // A vertical bar 100 wide and 700 high (an "I").
        let s = strokes(&[rect(0.0, -700.0, 100.0, 0.0)], 700.0);
        assert_eq!(s.satins.len(), 1, "{:?}", s.satins.len());
        assert!(s.coverage > 0.95, "{}", s.coverage);
        assert!((s.max_width - 100.0).abs() < 10.0, "{}", s.max_width);
        assert!((s.typical_width - 100.0).abs() < 5.0, "{}", s.typical_width);
    }

    #[test]
    fn a_t_is_two_columns() {
        let s = strokes(
            &[vec![
                Point::new(0.0, -700.0),
                Point::new(500.0, -700.0),
                Point::new(500.0, -610.0),
                Point::new(295.0, -610.0),
                Point::new(295.0, 0.0),
                Point::new(205.0, 0.0),
                Point::new(205.0, -610.0),
                Point::new(0.0, -610.0),
            ]],
            700.0,
        );
        assert_eq!(s.satins.len(), 2);
        assert!(s.coverage > 0.9, "{}", s.coverage);
        // The stem ends a little inside the bar (bar from y -700 to -610,
        // its middle at -655), not at the middle of the bar.
        let ys = |g: &SatinGeometry| -> Vec<f64> {
            g.rail_a
                .flatten(1.0)
                .iter()
                .chain(g.rail_b.flatten(1.0).iter())
                .flat_map(|l| l.points.iter().map(|p| p.y).collect::<Vec<_>>())
                .collect()
        };
        let height = |g: &SatinGeometry| {
            let y = ys(g);
            y.iter().copied().fold(f64::MIN, f64::max) - y.iter().copied().fold(f64::MAX, f64::min)
        };
        let stem = s.satins.iter().max_by(|a, b| height(a).total_cmp(&height(b))).unwrap();
        let top = ys(stem).into_iter().fold(f64::MAX, f64::min);
        assert!((-640.0..-610.0).contains(&top), "the stem ends at y {top}");
    }

    #[test]
    fn a_ring_is_a_closed_column() {
        let circle = |r: f64, rev: bool| {
            let mut v: Vec<Point> = (0..96)
                .map(|i| {
                    let a = i as f64 / 96.0 * std::f64::consts::TAU;
                    Point::new(350.0 + r * emb_geom::math::cos(a), -350.0 + r * emb_geom::math::sin(a))
                })
                .collect();
            if rev {
                v.reverse();
            }
            v
        };
        let s = strokes(&[circle(350.0, false), circle(260.0, true)], 700.0);
        assert_eq!(s.satins.len(), 1);
        let pl = s.satins[0].rail_a.flatten(0.1);
        assert!(pl[0].closed);
        assert!(s.coverage > 0.9, "{}", s.coverage);
    }

    #[test]
    fn an_l_is_cut_at_the_corner_and_a_dot_gets_a_stroke() {
        let l = vec![
            Point::new(0.0, -700.0),
            Point::new(100.0, -700.0),
            Point::new(100.0, -100.0),
            Point::new(450.0, -100.0),
            Point::new(450.0, 0.0),
            Point::new(0.0, 0.0),
        ];
        let s = strokes(&[l], 700.0);
        assert_eq!(s.satins.len(), 2);
        assert!(s.coverage > 0.9, "{}", s.coverage);
        let dot = strokes(&[rect(0.0, -120.0, 120.0, 0.0)], 700.0);
        assert_eq!(dot.satins.len(), 1);
        assert!(dot.coverage > 0.8, "{}", dot.coverage);
    }
}
