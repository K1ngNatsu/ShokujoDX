//! Node editing of a [`Path`] (the Reshape tool).
//!
//! A [`NodePath`] is a path as nodes and segments: each subpath is a list of
//! nodes, and segment `i` goes from node `i` to node `i + 1` (in a closed
//! subpath, the last segment goes back to node 0). A segment is a line or a
//! cubic with two handles. Edit a `NodePath`, then make a `Path` again with
//! [`NodePath::to_path`].
//!
//! - A node is smooth if it has two handles on one line, on opposite sides.
//!   An end node of an open subpath is smooth if its handle has a length.
//!   All other nodes are corners.
//! - Moving a node moves its handles with it.
//! - Moving a handle of a smooth node turns the other handle, so the node stays smooth.
//! - A new node splits a segment without a change of shape (de Casteljau).
//! - Deleting a node joins its two segments. The outer handles keep their
//!   directions and get longer by the length of the other segment, so a node
//!   that was just inserted can be deleted with almost no change of shape.

use crate::{Path, PathCmd, Point};

/// Points nearer than this are the same point, in mm.
const EPS: f64 = 1e-9;
/// Handles at less than this angle from a straight line count as smooth, as a sine.
const SMOOTH_SIN: f64 = 1e-3;

/// A segment from one node to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Line,
    /// The handle of the start node and the handle of the end node.
    Cubic(Point, Point),
}

/// One subpath as nodes and segments.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubNodes {
    pub nodes: Vec<Point>,
    /// `nodes.len() - 1` segments if open, `nodes.len()` if closed.
    pub segs: Vec<Seg>,
    pub closed: bool,
}

/// A node: subpath index and node index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub sub: usize,
    pub idx: usize,
}

/// A segment: subpath index and segment index (from node `idx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegId {
    pub sub: usize,
    pub idx: usize,
}

/// Which handle of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The handle on the segment that ends at the node.
    In,
    /// The handle on the segment that starts at the node.
    Out,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodePath {
    pub subs: Vec<SubNodes>,
}

fn cubic_at(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    p0 * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

impl SubNodes {
    fn n(&self) -> usize {
        self.nodes.len()
    }

    /// The same subpath in the opposite direction. A closed subpath keeps
    /// its first node.
    pub fn reversed(&self) -> SubNodes {
        let swap = |g: &Seg| match *g {
            Seg::Line => Seg::Line,
            Seg::Cubic(h1, h2) => Seg::Cubic(h2, h1),
        };
        let k = self.n();
        if self.closed && k > 0 {
            SubNodes {
                nodes: (0..k).map(|j| self.nodes[(k - j) % k]).collect(),
                segs: (0..self.segs.len()).map(|j| swap(&self.segs[k - j - 1])).collect(),
                closed: true,
            }
        } else {
            SubNodes {
                nodes: self.nodes.iter().rev().copied().collect(),
                segs: self.segs.iter().rev().map(swap).collect(),
                closed: self.closed,
            }
        }
    }

    /// Closes an open subpath with a segment from its last node to its
    /// first node: a line, or (`curve`) a cubic that continues the
    /// directions of the two end segments. Returns `false` if the subpath
    /// is closed or has too few nodes (3 for a line, 2 for a curve).
    pub fn close_with(&mut self, curve: bool) -> bool {
        let k = self.n();
        if self.closed || k < 2 || (!curve && k < 3) {
            return false;
        }
        let (first, last) = (self.nodes[0], self.nodes[k - 1]);
        let chord = first.distance(last);
        if chord < EPS {
            // The ends are at the same place: drop the last node.
            if k < 4 {
                return false;
            }
            self.nodes.pop();
            self.closed = true;
            return true;
        }
        let seg = if curve {
            // The direction at the last node, out of the subpath.
            let before = match self.segs[k - 2] {
                Seg::Cubic(_, c2) if c2.distance(last) > EPS => c2,
                _ => self.nodes[k - 2],
            };
            // The direction at the first node, into the subpath.
            let after = match self.segs[0] {
                Seg::Cubic(c1, _) if c1.distance(first) > EPS => c1,
                _ => self.nodes[1],
            };
            let out = (last - before).normalized().unwrap_or(Point::ZERO);
            let into = (after - first).normalized().unwrap_or(Point::ZERO);
            Seg::Cubic(last + out * (chord / 3.0), first - into * (chord / 3.0))
        } else {
            Seg::Line
        };
        self.segs.push(seg);
        self.closed = true;
        true
    }

    /// Index of the segment that ends at node `i`.
    fn seg_in(&self, i: usize) -> Option<usize> {
        match (i, self.closed) {
            (0, false) => None,
            (0, true) => Some(self.n() - 1),
            _ => Some(i - 1),
        }
    }

    /// Index of the segment that starts at node `i`.
    fn seg_out(&self, i: usize) -> Option<usize> {
        (i < self.segs.len()).then_some(i)
    }

    fn next(&self, i: usize) -> usize {
        (i + 1) % self.n()
    }

    /// End points of segment `s`.
    fn ends(&self, s: usize) -> (Point, Point) {
        (self.nodes[s], self.nodes[self.next(s)])
    }

    /// Makes segment `s` a cubic with the same shape, if it is a line. The
    /// handles are on the end points, so the end nodes stay corners.
    fn make_cubic(&mut self, s: usize) {
        if self.segs[s] == Seg::Line {
            let (a, b) = self.ends(s);
            self.segs[s] = Seg::Cubic(a, b);
        }
    }

    /// Length of segment `s` (a polyline of 32 parts for a cubic).
    fn length(&self, s: usize) -> f64 {
        const PARTS: usize = 32;
        (0..PARTS)
            .map(|k| {
                let (t0, t1) = (k as f64 / PARTS as f64, (k + 1) as f64 / PARTS as f64);
                self.point_at(s, t0).distance(self.point_at(s, t1))
            })
            .sum()
    }

    /// Makes segment `s` a line if both handles are on its end points.
    fn simplify(&mut self, s: usize) {
        let (a, b) = self.ends(s);
        if let Seg::Cubic(c1, c2) = self.segs[s]
            && c1.distance(a) < EPS
            && c2.distance(b) < EPS
        {
            self.segs[s] = Seg::Line;
        }
    }

    fn point_at(&self, s: usize, t: f64) -> Point {
        let (a, b) = self.ends(s);
        match self.segs[s] {
            Seg::Line => a.lerp(b, t),
            Seg::Cubic(c1, c2) => cubic_at(a, c1, c2, b, t),
        }
    }
}

impl NodePath {
    pub fn from_path(path: &Path) -> NodePath {
        let mut subs = Vec::new();
        let mut cur: Option<SubNodes> = None;
        let finish = |cur: &mut Option<SubNodes>, subs: &mut Vec<SubNodes>, closed: bool| {
            if let Some(mut s) = cur.take() {
                if closed && s.n() >= 2 {
                    // An explicit last segment back to the start is the closing segment.
                    if s.nodes[s.n() - 1].distance(s.nodes[0]) < EPS {
                        s.nodes.pop();
                    } else {
                        s.segs.push(Seg::Line);
                    }
                    s.closed = s.n() >= 2;
                    if !s.closed {
                        s.segs.clear();
                    }
                }
                if !s.nodes.is_empty() {
                    subs.push(s);
                }
            }
        };
        for cmd in &path.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    finish(&mut cur, &mut subs, false);
                    cur = Some(SubNodes {
                        nodes: vec![p],
                        ..Default::default()
                    });
                }
                PathCmd::LineTo(p) => {
                    let s = cur.get_or_insert_with(|| SubNodes {
                        nodes: vec![Point::ZERO],
                        ..Default::default()
                    });
                    s.nodes.push(p);
                    s.segs.push(Seg::Line);
                }
                PathCmd::CubicTo(c1, c2, p) => {
                    let s = cur.get_or_insert_with(|| SubNodes {
                        nodes: vec![Point::ZERO],
                        ..Default::default()
                    });
                    s.nodes.push(p);
                    s.segs.push(Seg::Cubic(c1, c2));
                }
                PathCmd::Close => finish(&mut cur, &mut subs, true),
            }
        }
        finish(&mut cur, &mut subs, false);
        NodePath { subs }
    }

    pub fn to_path(&self) -> Path {
        let mut path = Path::new();
        for s in &self.subs {
            let Some(&first) = s.nodes.first() else { continue };
            path = path.move_to(first);
            for (i, seg) in s.segs.iter().enumerate() {
                let b = s.nodes[s.next(i)];
                path = match *seg {
                    Seg::Line => path.line_to(b),
                    Seg::Cubic(c1, c2) => path.cubic_to(c1, c2, b),
                };
            }
            if s.closed {
                path = path.close();
            }
        }
        path
    }

    /// All nodes with their positions.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, Point)> + '_ {
        self.subs.iter().enumerate().flat_map(|(sub, s)| {
            s.nodes
                .iter()
                .enumerate()
                .map(move |(idx, p)| (NodeId { sub, idx }, *p))
        })
    }

    pub fn node(&self, id: NodeId) -> Option<Point> {
        self.subs.get(id.sub)?.nodes.get(id.idx).copied()
    }

    /// The handle of the node on `side`, if that segment is a cubic.
    pub fn handle(&self, id: NodeId, side: Side) -> Option<Point> {
        let s = self.subs.get(id.sub)?;
        match side {
            Side::In => match s.segs[s.seg_in(id.idx)?] {
                Seg::Cubic(_, c2) => Some(c2),
                Seg::Line => None,
            },
            Side::Out => match s.segs[s.seg_out(id.idx)?] {
                Seg::Cubic(c1, _) => Some(c1),
                Seg::Line => None,
            },
        }
    }

    fn set_handle(&mut self, id: NodeId, side: Side, h: Point) {
        let s = &mut self.subs[id.sub];
        let seg = match side {
            Side::In => s.seg_in(id.idx),
            Side::Out => s.seg_out(id.idx),
        };
        let Some(k) = seg else { return };
        s.make_cubic(k);
        if let Seg::Cubic(c1, c2) = &mut s.segs[k] {
            match side {
                Side::In => *c2 = h,
                Side::Out => *c1 = h,
            }
        }
    }

    pub fn is_smooth(&self, id: NodeId) -> bool {
        let Some(p) = self.node(id) else { return false };
        let len = |h: Option<Point>| h.map_or(0.0, |h| h.distance(p));
        let (hi, ho) = (self.handle(id, Side::In), self.handle(id, Side::Out));
        let s = &self.subs[id.sub];
        let end = s.seg_in(id.idx).is_none() || s.seg_out(id.idx).is_none();
        if end {
            return len(hi) > EPS || len(ho) > EPS;
        }
        match (hi, ho) {
            (Some(a), Some(b)) if len(hi) > EPS && len(ho) > EPS => {
                let (u, v) = (a - p, b - p);
                u.dot(v) < 0.0 && (u.cross(v) / (u.length() * v.length())).abs() < SMOOTH_SIN
            }
            _ => false,
        }
    }

    /// Moves the node to `to`. Its handles move with it.
    pub fn move_node(&mut self, id: NodeId, to: Point) {
        let Some(p) = self.node(id) else { return };
        let d = to - p;
        for side in [Side::In, Side::Out] {
            if let Some(h) = self.handle(id, side) {
                self.set_handle(id, side, h + d);
            }
        }
        self.subs[id.sub].nodes[id.idx] = to;
    }

    /// Moves a handle to `to`. If `keep_smooth` and the node is smooth, the
    /// other handle turns to stay on the line (its length stays).
    pub fn move_handle(&mut self, id: NodeId, side: Side, to: Point, keep_smooth: bool) {
        let Some(p) = self.node(id) else { return };
        if self.handle(id, side).is_none() {
            return;
        }
        let smooth = keep_smooth && self.is_smooth(id);
        self.set_handle(id, side, to);
        let other = match side {
            Side::In => Side::Out,
            Side::Out => Side::In,
        };
        if smooth
            && let Some(h) = self.handle(id, other)
            && let Some(dir) = (p - to).normalized()
        {
            self.set_handle(id, other, p + dir * h.distance(p));
        }
    }

    /// Makes the node smooth (handles along the line between its neighbors)
    /// or a corner (handles on the node).
    pub fn set_smooth(&mut self, id: NodeId, smooth: bool) {
        let Some(p) = self.node(id) else { return };
        let s = &self.subs[id.sub];
        let (si, so) = (s.seg_in(id.idx), s.seg_out(id.idx));
        if !smooth {
            for side in [Side::In, Side::Out] {
                if self.handle(id, side).is_some() {
                    self.set_handle(id, side, p);
                }
            }
            let s = &mut self.subs[id.sub];
            for k in [si, so].into_iter().flatten() {
                s.simplify(k);
            }
            return;
        }
        let prev = si.map(|k| s.nodes[k]);
        let next = so.map(|k| s.nodes[s.next(k)]);
        let tangent = match (prev, next) {
            (Some(a), Some(b)) => (b - a) * 0.5,
            (None, Some(b)) => b - p,
            (Some(a), None) => p - a,
            (None, None) => return,
        };
        if si.is_some() {
            self.set_handle(id, Side::In, p - tangent * (1.0 / 3.0));
        }
        if so.is_some() {
            self.set_handle(id, Side::Out, p + tangent * (1.0 / 3.0));
        }
    }

    /// The point on the path nearest to `p`: segment, parameter `t` and distance.
    pub fn nearest(&self, p: Point) -> Option<(SegId, f64, f64)> {
        const STEPS: usize = 48;
        let mut best: Option<(SegId, f64, f64)> = None;
        for (sub, s) in self.subs.iter().enumerate() {
            for idx in 0..s.segs.len() {
                let id = SegId { sub, idx };
                let (t, d) = match s.segs[idx] {
                    Seg::Line => {
                        let (a, b) = s.ends(idx);
                        let ab = b - a;
                        let l2 = ab.length_sq();
                        let t = if l2 > 0.0 {
                            ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        (t, a.lerp(b, t).distance(p))
                    }
                    Seg::Cubic(..) => {
                        // Sample, then refine around the best sample (golden section).
                        let mut t0 = 0.0;
                        let mut d0 = f64::INFINITY;
                        for k in 0..=STEPS {
                            let t = k as f64 / STEPS as f64;
                            let d = s.point_at(idx, t).distance(p);
                            if d < d0 {
                                (t0, d0) = (t, d);
                            }
                        }
                        let (mut lo, mut hi) = ((t0 - 1.0 / STEPS as f64).max(0.0), (t0 + 1.0 / STEPS as f64).min(1.0));
                        let g = 0.618_033_988_749_894_9;
                        for _ in 0..40 {
                            let (m1, m2) = (hi - (hi - lo) * g, lo + (hi - lo) * g);
                            if s.point_at(idx, m1).distance(p) < s.point_at(idx, m2).distance(p) {
                                hi = m2;
                            } else {
                                lo = m1;
                            }
                        }
                        let t = (lo + hi) / 2.0;
                        (t, s.point_at(idx, t).distance(p))
                    }
                };
                if best.is_none_or(|b| d < b.2) {
                    best = Some((id, t, d));
                }
            }
        }
        best
    }

    /// Splits the segment at `t` with a new node. The shape does not change.
    pub fn insert_node(&mut self, seg: SegId, t: f64) -> Option<NodeId> {
        let s = self.subs.get_mut(seg.sub)?;
        if seg.idx >= s.segs.len() {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        let (a, b) = s.ends(seg.idx);
        let (left, right, mid) = match s.segs[seg.idx] {
            Seg::Line => (Seg::Line, Seg::Line, a.lerp(b, t)),
            Seg::Cubic(c1, c2) => {
                let (ab, bc, cd) = (a.lerp(c1, t), c1.lerp(c2, t), c2.lerp(b, t));
                let (abc, bcd) = (ab.lerp(bc, t), bc.lerp(cd, t));
                let m = abc.lerp(bcd, t);
                (Seg::Cubic(ab, abc), Seg::Cubic(bcd, cd), m)
            }
        };
        s.segs[seg.idx] = left;
        s.segs.insert(seg.idx + 1, right);
        s.nodes.insert(seg.idx + 1, mid);
        Some(NodeId {
            sub: seg.sub,
            idx: seg.idx + 1,
        })
    }

    /// Removes the node and joins its segments. An open subpath keeps two
    /// nodes and a closed one three; returns `false` if the node cannot go.
    pub fn delete_node(&mut self, id: NodeId) -> bool {
        let Some(s) = self.subs.get_mut(id.sub) else {
            return false;
        };
        let n = s.n();
        if id.idx >= n || (s.closed && n <= 3) || (!s.closed && n <= 2) {
            return false;
        }
        match (s.seg_in(id.idx), s.seg_out(id.idx)) {
            (None, _) => {
                s.nodes.remove(0);
                s.segs.remove(0);
            }
            (_, None) => {
                s.nodes.pop();
                s.segs.pop();
            }
            (Some(si), Some(so)) => {
                let (a, _) = s.ends(si);
                let (_, b) = s.ends(so);
                // Where the node was on the joined segment, by length.
                let (li, lo) = (s.length(si), s.length(so));
                let t = if li + lo > 0.0 { li / (li + lo) } else { 0.5 };
                let c1 = match s.segs[si] {
                    Seg::Cubic(c1, _) if t > EPS => Some(a + (c1 - a) * (1.0 / t)),
                    Seg::Cubic(c1, _) => Some(c1),
                    Seg::Line => None,
                };
                let c2 = match s.segs[so] {
                    Seg::Cubic(_, c2) if t < 1.0 - EPS => Some(b + (c2 - b) * (1.0 / (1.0 - t))),
                    Seg::Cubic(_, c2) => Some(c2),
                    Seg::Line => None,
                };
                s.segs[si] = match (c1, c2) {
                    (None, None) => Seg::Line,
                    (c1, c2) => Seg::Cubic(c1.unwrap_or(a), c2.unwrap_or(b)),
                };
                s.segs.remove(so);
                s.nodes.remove(id.idx);
            }
        }
        true
    }
}

impl Path {
    /// The path in the opposite direction: the subpaths in the opposite
    /// order, each one from its end to its start.
    pub fn reversed(&self) -> Path {
        let np = NodePath::from_path(self);
        NodePath {
            subs: np.subs.iter().rev().map(SubNodes::reversed).collect(),
        }
        .to_path()
    }

    /// Closes each open subpath (see [`SubNodes::close_with`]). Returns the
    /// new path and the number of subpaths that it closed.
    pub fn close_open(&self, curve: bool) -> (Path, usize) {
        let mut np = NodePath::from_path(self);
        let n = np
            .subs
            .iter_mut()
            .filter_map(|s| s.close_with(curve).then_some(()))
            .count();
        (np.to_path(), n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BBox;

    #[test]
    fn reversed_open_and_closed() {
        let open =
            Path::new()
                .move_to(p(0.0, 0.0))
                .line_to(p(10.0, 0.0))
                .cubic_to(p(12.0, 0.0), p(14.0, 2.0), p(14.0, 5.0));
        let r = open.reversed();
        assert_eq!(
            r,
            Path::new()
                .move_to(p(14.0, 5.0))
                .cubic_to(p(14.0, 2.0), p(12.0, 0.0), p(10.0, 0.0))
                .line_to(p(0.0, 0.0))
        );
        assert_eq!(r.reversed(), open);
        let square = Path::polygon(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)]);
        let r = square.reversed();
        let pts: Vec<Point> = r.flatten(0.01)[0].points.clone();
        assert_eq!(pts, vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        assert!(close(&r.reversed(), &square, 1e-9));
        // Two subpaths: the last one comes first.
        let two = Path::joined([&open, &square]);
        let r = two.reversed();
        assert_eq!(r.flatten(0.01)[0].points[0], p(0.0, 0.0));
        assert!(r.flatten(0.01)[0].closed && !r.flatten(0.01)[1].closed);
    }

    #[test]
    fn close_with_a_line_or_a_curve() {
        let u = Path::new()
            .move_to(p(0.0, 10.0))
            .line_to(p(0.0, 0.0))
            .line_to(p(10.0, 0.0))
            .line_to(p(10.0, 10.0));
        let (closed, n) = u.close_open(false);
        assert_eq!(n, 1);
        assert!(closed.flatten(0.01)[0].closed);
        assert!((closed.flatten(0.01)[0].length() - 40.0).abs() < 1e-9);
        // A curve goes on up from (10, 10) and comes down into (0, 10): it bulges up.
        let (curved, n) = u.close_open(true);
        assert_eq!(n, 1);
        let b = curved.control_bbox().unwrap();
        assert!(b.max.y > 12.0, "{b:?}");
        // Closed paths and single lines do not change.
        assert_eq!(closed.close_open(true).1, 0);
        let line = Path::new().move_to(p(0.0, 0.0)).line_to(p(5.0, 0.0));
        assert_eq!(line.close_open(false).1, 0);
        // Ends at the same place: the path closes there.
        let ring = Path::new()
            .move_to(p(0.0, 0.0))
            .line_to(p(5.0, 0.0))
            .line_to(p(5.0, 5.0))
            .line_to(p(0.0, 0.0));
        let (c, n) = ring.close_open(false);
        assert_eq!(n, 1);
        assert_eq!(c.flatten(0.01)[0].points.len(), 3);
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn close(a: &Path, b: &Path, tol: f64) -> bool {
        let (fa, fb) = (a.flatten(0.01), b.flatten(0.01));
        fa.len() == fb.len()
            && fa.iter().zip(&fb).all(|(x, y)| {
                // Each point of one is near the other polyline.
                x.points
                    .iter()
                    .all(|q| y.segments().any(|(s, e)| q.distance_to_segment(s, e) < tol))
            })
    }

    #[test]
    fn round_trip_open_closed_and_through_points() {
        let open =
            Path::new()
                .move_to(p(0.0, 0.0))
                .line_to(p(10.0, 0.0))
                .cubic_to(p(12.0, 0.0), p(14.0, 2.0), p(14.0, 5.0));
        assert_eq!(NodePath::from_path(&open).to_path(), open);

        let rect = Path::rect(BBox::new(p(0.0, 0.0), p(10.0, 5.0)));
        let np = NodePath::from_path(&rect);
        assert_eq!(np.subs[0].nodes.len(), 4);
        assert!(np.subs[0].closed);
        assert!(close(&np.to_path(), &rect, 1e-9));

        let pts = [(p(0.0, 0.0), true), (p(10.0, 0.0), true), (p(5.0, 8.0), false)];
        let curve = Path::through_points(&pts, true);
        let np = NodePath::from_path(&curve);
        assert_eq!(
            np.subs[0].nodes.len(),
            3,
            "the explicit last segment is the closing one"
        );
        assert_eq!(np.to_path(), curve);
        assert!(np.is_smooth(NodeId { sub: 0, idx: 0 }));
        assert!(!np.is_smooth(NodeId { sub: 0, idx: 2 }));
    }

    #[test]
    fn moving_a_node_moves_its_handles() {
        let pts = [(p(0.0, 0.0), false), (p(10.0, 0.0), true), (p(20.0, 0.0), false)];
        let mut np = NodePath::from_path(&Path::through_points(&pts, false));
        let id = NodeId { sub: 0, idx: 1 };
        let before = np.handle(id, Side::In).unwrap();
        np.move_node(id, p(10.0, 5.0));
        assert_eq!(np.node(id), Some(p(10.0, 5.0)));
        assert_eq!(np.handle(id, Side::In), Some(before + p(0.0, 5.0)));
        assert!(np.is_smooth(id));
    }

    #[test]
    fn smooth_handles_stay_on_a_line() {
        let pts = [(p(0.0, 0.0), false), (p(10.0, 0.0), true), (p(20.0, 0.0), false)];
        let mut np = NodePath::from_path(&Path::through_points(&pts, false));
        let id = NodeId { sub: 0, idx: 1 };
        let out_len = np.handle(id, Side::Out).unwrap().distance(p(10.0, 0.0));
        np.move_handle(id, Side::In, p(7.0, -3.0), true);
        let o = np.handle(id, Side::Out).unwrap();
        assert!((o.distance(p(10.0, 0.0)) - out_len).abs() < 1e-9);
        assert!(np.is_smooth(id));
        // Without keep_smooth the node becomes a corner.
        np.move_handle(id, Side::In, p(7.0, 3.0), false);
        assert!(!np.is_smooth(id));
    }

    #[test]
    fn corner_and_curve() {
        let mut np = NodePath::from_path(&Path::rect(BBox::new(p(0.0, 0.0), p(10.0, 10.0))));
        let id = NodeId { sub: 0, idx: 1 };
        assert!(!np.is_smooth(id));
        np.set_smooth(id, true);
        assert!(np.is_smooth(id));
        np.set_smooth(id, false);
        assert!(!np.is_smooth(id));
        // Back to lines: the same rectangle.
        assert!(np.subs[0].segs.iter().all(|s| *s == Seg::Line));
    }

    #[test]
    fn insert_keeps_the_shape_and_delete_joins() {
        let circle = Path::ellipse(p(0.0, 0.0), 10.0, 10.0);
        let mut np = NodePath::from_path(&circle);
        let (seg, t, d) = np.nearest(p(7.2, 7.2)).unwrap();
        assert!(d < 0.2, "{d}");
        let id = np.insert_node(seg, t).unwrap();
        assert_eq!(np.subs[0].nodes.len(), 5);
        assert!(close(&np.to_path(), &circle, 1e-6));
        assert!(np.delete_node(id));
        assert!(close(&np.to_path(), &circle, 0.02));
        // A closed subpath keeps three nodes.
        let mut tri = NodePath::from_path(&Path::polygon(&[p(0.0, 0.0), p(5.0, 0.0), p(0.0, 5.0)]));
        assert!(!tri.delete_node(NodeId { sub: 0, idx: 0 }));
        // Open ends can be deleted down to two nodes.
        let mut line = NodePath::from_path(
            &Path::new()
                .move_to(p(0.0, 0.0))
                .line_to(p(1.0, 0.0))
                .line_to(p(2.0, 0.0)),
        );
        assert!(line.delete_node(NodeId { sub: 0, idx: 0 }));
        assert_eq!(line.subs[0].nodes, vec![p(1.0, 0.0), p(2.0, 0.0)]);
        assert!(!line.delete_node(NodeId { sub: 0, idx: 0 }));
    }
}
