//! Travel inside a filled shape, and a needle path that is built from
//! stitched pieces and travel.
//!
//! Not a port. Travel is the shortest path inside the shape: a straight
//! line if the shape contains it, else a path through the corners of the
//! shape moved inward by [`TRAVEL_INSET_MM`] (a visibility graph with A*
//! search). Fill and satin objects sew all their parts as one needle path,
//! and use travel to go from one part to the next. If no path inside the
//! shape exists (two separate areas), the needle path ends and a new one
//! starts. The stitch plan then connects them with a travel stitch or a trim.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use emb_geom::region::{self, Join, Region};
use emb_geom::{Point, Polyline};

use crate::running;

/// Travel goes this far inside the edge of the shape.
pub const TRAVEL_INSET_MM: f64 = 0.3;
/// The corners of the inset shape are simplified to this tolerance.
const SIMPLIFY_MM: f64 = 0.1;
/// Points nearer than this are the same point.
const EPS: f64 = 1e-7;

/// A filled shape as normalized regions: outer rings with holes.
#[derive(Debug, Clone, Default)]
pub struct Shape {
    pub regions: Vec<Region>,
}

impl Shape {
    /// Makes a shape from outer rings and holes. Overlaps and self
    /// intersections are resolved; holes are removed from all outer rings.
    pub fn new(outers: &[Polyline], holes: &[Polyline]) -> Shape {
        let ring = |p: &Polyline| Region::new(p.points.clone(), Vec::new());
        let outer: Vec<Region> = outers.iter().filter(|p| p.points.len() >= 3).map(ring).collect();
        let holes: Vec<Region> = holes.iter().filter(|p| p.points.len() >= 3).map(ring).collect();
        // An outer ring inside a hole is an island (one fill object with
        // several shapes, as Join and the shaping commands make): it stays.
        let in_hole = |o: &Region| {
            let area = o.area();
            o.outer.first().is_some_and(|p| {
                holes
                    .iter()
                    .any(|h| h.area() > area && region::ring_contains(&h.outer, *p))
            })
        };
        let (islands, outer): (Vec<Region>, Vec<Region>) = outer.into_iter().partition(in_hole);
        // The holes inside an island are its own holes.
        let (island_holes, holes): (Vec<Region>, Vec<Region>) = holes.into_iter().partition(|h| {
            h.outer
                .first()
                .is_some_and(|p| islands.iter().any(|i| region::ring_contains(&i.outer, *p)))
        });
        let mut regions = region::union(&outer, &[]);
        if !holes.is_empty() {
            regions = region::difference(&regions, &region::union(&holes, &[]));
        }
        if !islands.is_empty() {
            // An island is in a hole of the other areas, so it does not
            // overlap them: it is added as it is (a union would lose its holes).
            let mut own = region::union(&islands, &[]);
            if !island_holes.is_empty() {
                own = region::difference(&own, &region::union(&island_holes, &[]));
            }
            regions.extend(own);
        }
        regions.retain(|r| r.area() > 1e-6);
        Shape { regions }
    }

    /// All rings: outer rings and holes.
    pub fn rings(&self) -> impl Iterator<Item = &Vec<Point>> {
        self.regions
            .iter()
            .flat_map(|r| std::iter::once(&r.outer).chain(r.holes.iter()))
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// The shape with its edge moved inward by `d` mm (outward if `d` is negative).
    pub fn inset(&self, d: f64) -> Shape {
        let mut regions = region::offset(&self.regions, -d, Join::Miter);
        regions.retain(|r| r.area() > 1e-6);
        Shape { regions }
    }
}

/// The edges of the shape in a grid of cells, so a test of a line looks
/// only at the edges near it. An edge is in each cell that its box touches.
#[derive(Debug, Clone, Default)]
struct EdgeGrid {
    min: Point,
    cell: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

/// The grid has at most this many cells on a side.
const MAX_CELLS: usize = 256;

impl EdgeGrid {
    fn new(edges: &[(Point, Point)]) -> EdgeGrid {
        let pts = edges.iter().flat_map(|&(a, b)| [a, b]);
        let Some(b) = emb_geom::BBox::from_points(pts) else {
            return EdgeGrid::default();
        };
        let (w, h) = (b.width().max(1e-6), b.height().max(1e-6));
        // About one cell for each edge.
        let cell = ((w * h) / edges.len().max(1) as f64)
            .sqrt()
            .max(w.max(h) / MAX_CELLS as f64)
            .max(1e-3);
        let nx = ((w / cell).ceil() as usize).clamp(1, MAX_CELLS);
        let ny = ((h / cell).ceil() as usize).clamp(1, MAX_CELLS);
        let mut g = EdgeGrid {
            min: b.min,
            cell,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        };
        for (i, &(p, q)) in edges.iter().enumerate() {
            let (x0, y0) = g.at(Point::new(p.x.min(q.x) - 1e-6, p.y.min(q.y) - 1e-6));
            let (x1, y1) = g.at(Point::new(p.x.max(q.x) + 1e-6, p.y.max(q.y) + 1e-6));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    g.cells[y * nx + x].push(i as u32);
                }
            }
        }
        g
    }

    /// The cell of a point (clamped to the grid).
    fn at(&self, p: Point) -> (usize, usize) {
        let c = |v: f64, n: usize| ((v / self.cell).floor().max(0.0) as usize).min(n - 1);
        (c(p.x - self.min.x, self.nx), c(p.y - self.min.y, self.ny))
    }

    /// Calls `f` for each edge in the cells near the segment from `p` to
    /// `q` (each point of the segment is in one of these cells: samples
    /// every half cell, with the cells round each sample), each edge once,
    /// until `f` returns `true`. Returns `true` if it did.
    fn any_near(&self, p: Point, q: Point, marks: &mut Marks, mut f: impl FnMut(u32) -> bool) -> bool {
        if self.cells.is_empty() {
            return false;
        }
        marks.next(self.cells.len());
        let n = ((p.distance(q) / (self.cell * 0.5)).ceil() as usize).max(1);
        for k in 0..=n {
            let (cx, cy) = self.at(p.lerp(q, k as f64 / n as f64));
            for y in cy.saturating_sub(1)..=(cy + 1).min(self.ny - 1) {
                for x in cx.saturating_sub(1)..=(cx + 1).min(self.nx - 1) {
                    let c = y * self.nx + x;
                    if !marks.cell(c) {
                        continue;
                    }
                    for &e in &self.cells[c] {
                        if marks.edge(e as usize) && f(e) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Marks of the cells and edges that a test saw, by test number, so each
/// is looked at once.
#[derive(Debug, Clone, Default)]
struct Marks {
    test: u32,
    cells: Vec<u32>,
    edges: Vec<u32>,
}

impl Marks {
    fn next(&mut self, cells: usize) {
        self.test = self.test.wrapping_add(1);
        if self.test == 0 {
            self.cells.iter_mut().for_each(|m| *m = 0);
            self.edges.iter_mut().for_each(|m| *m = 0);
            self.test = 1;
        }
        if self.cells.len() < cells {
            self.cells.resize(cells, 0);
        }
    }

    /// Returns `true` the first time that this test sees cell `c`.
    fn cell(&mut self, c: usize) -> bool {
        let new = self.cells[c] != self.test;
        self.cells[c] = self.test;
        new
    }

    /// Returns `true` the first time that this test sees edge `e`.
    fn edge(&mut self, e: usize) -> bool {
        if self.edges.len() <= e {
            self.edges.resize(e + 1, 0);
        }
        let new = self.edges[e] != self.test;
        self.edges[e] = self.test;
        new
    }
}

/// Finds travel paths inside a shape.
#[derive(Debug, Clone)]
pub struct Travel {
    edges: Vec<(Point, Point)>,
    /// The edges in a grid, for fast tests.
    grid: EdgeGrid,
    rings: Vec<Vec<Point>>,
    /// The box of each ring: a point outside it is not inside the ring.
    ring_boxes: Vec<emb_geom::BBox>,
    /// The tests of `visible` between two nodes so far (a search tests the
    /// same pairs many times).
    seen: RefCell<HashMap<(u32, u32), bool>>,
    marks: RefCell<Marks>,
    /// Corners of the inset shape, ring by ring.
    nodes: Vec<Point>,
    /// For each node: the previous and the next node on its ring.
    ring_links: Vec<(usize, usize)>,
}

/// How much more a step across the inside of the shape costs than a step
/// along the edge, for [`Travel::edge_path`].
pub const INTERIOR_COST: f64 = 3.0;

impl Travel {
    pub fn new(shape: &Shape) -> Travel {
        let rings: Vec<Vec<Point>> = shape.rings().cloned().collect();
        let mut edges = Vec::new();
        for r in &rings {
            for i in 0..r.len() {
                edges.push((r[i], r[(i + 1) % r.len()]));
            }
        }
        // Short mitres: at a sharp inner corner (a cusp, such as where two
        // scallops meet) a long mitre goes far into the shape, and the
        // travel went out to its point and back.
        let mut regions = region::offset(&shape.regions, -TRAVEL_INSET_MM, Join::ShortMiter);
        regions.retain(|r| r.area() > 1e-6);
        let inset = Shape { regions };
        let source = if inset.is_empty() { shape } else { &inset };
        let mut nodes = Vec::new();
        let mut ring_links = Vec::new();
        for r in source.rings() {
            let ring = simplify_ring(r, SIMPLIFY_MM);
            let (base, n) = (nodes.len(), ring.len());
            for i in 0..n {
                ring_links.push((base + (i + n - 1) % n, base + (i + 1) % n));
            }
            nodes.extend(ring);
        }
        let ring_boxes = rings
            .iter()
            .map(|r| {
                emb_geom::BBox::from_points(r.iter().copied()).unwrap_or(emb_geom::BBox::new(Point::ZERO, Point::ZERO))
            })
            .collect();
        Travel {
            grid: EdgeGrid::new(&edges),
            edges,
            rings,
            ring_boxes,
            seen: RefCell::new(HashMap::new()),
            marks: RefCell::new(Marks::default()),
            nodes,
            ring_links,
        }
    }

    /// Even-odd test against all rings.
    fn contains(&self, p: Point) -> bool {
        self.rings
            .iter()
            .zip(&self.ring_boxes)
            .filter(|(r, b)| {
                p.x >= b.min.x && p.x <= b.max.x && p.y >= b.min.y && p.y <= b.max.y && region::ring_contains(r, p)
            })
            .count()
            % 2
            == 1
    }

    fn on_edge(&self, p: Point) -> bool {
        self.grid.any_near(p, p, &mut self.marks.borrow_mut(), |i| {
            let (a, b) = self.edges[i as usize];
            p.distance_to_segment(a, b) < 1e-6
        })
    }

    /// The straight line from `p` to `q` stays inside the shape (it can touch the edge).
    pub fn visible(&self, p: Point, q: Point) -> bool {
        if p.distance(q) < EPS {
            return true;
        }
        let (lo, hi) = (
            Point::new(p.x.min(q.x), p.y.min(q.y)),
            Point::new(p.x.max(q.x), p.y.max(q.y)),
        );
        let apart = |a: Point, b: Point| {
            a.x.max(b.x) < lo.x || a.x.min(b.x) > hi.x || a.y.max(b.y) < lo.y || a.y.min(b.y) > hi.y
        };
        let blocked = self.grid.any_near(p, q, &mut self.marks.borrow_mut(), |i| {
            let (a, b) = self.edges[i as usize];
            !apart(a, b) && crosses(p, q, a, b)
        });
        if blocked {
            return false;
        }
        let m = p.midpoint(q);
        self.contains(m) || self.on_edge(m)
    }

    /// The shortest path inside the shape from `a` to `b`, both included.
    /// `None` if `b` cannot be reached from `a` inside the shape.
    pub fn path(&self, a: Point, b: Point) -> Option<Vec<Point>> {
        if self.visible(a, b) {
            return Some(vec![a, b]);
        }
        self.search(a, b, 1.0)
    }

    /// A path inside the shape from `a` to `b` that goes along the edge
    /// where it can (a step across the inside costs [`INTERIOR_COST`] times
    /// its length). Travel along the edge is hidden better than travel across
    /// stitches that are already sewn.
    pub fn edge_path(&self, a: Point, b: Point) -> Option<Vec<Point>> {
        self.search(a, b, INTERIOR_COST)
    }

    /// `visible` between two points of a search; the result between two
    /// nodes of the shape is kept for the next searches.
    fn visible_nodes(&self, pts: &[Point], i: usize, j: usize) -> bool {
        if i < 2 || j < 2 {
            return self.visible(pts[i], pts[j]);
        }
        let key = (i.min(j) as u32, i.max(j) as u32);
        if let Some(v) = self.seen.borrow().get(&key) {
            return *v;
        }
        let v = self.visible(pts[i], pts[j]);
        self.seen.borrow_mut().insert(key, v);
        v
    }

    /// A* on the visibility graph of `a`, `b` and the nodes. Node 0 is `a`, node 1 is `b`.
    fn search(&self, a: Point, b: Point, interior_cost: f64) -> Option<Vec<Point>> {
        let mut pts = Vec::with_capacity(self.nodes.len() + 2);
        pts.push(a);
        pts.push(b);
        pts.extend(self.nodes.iter().copied());
        let n = pts.len();
        let along_edge = |i: usize, j: usize| {
            i >= 2 && j >= 2 && {
                let (prev, next) = self.ring_links[i - 2];
                prev + 2 == j || next + 2 == j
            }
        };
        let mut dist = vec![f64::INFINITY; n];
        let mut from = vec![usize::MAX; n];
        let mut done = vec![false; n];
        let mut heap = BinaryHeap::new();
        dist[0] = 0.0;
        heap.push(Entry {
            cost: pts[0].distance(b),
            node: 0,
        });
        while let Some(Entry { node, .. }) = heap.pop() {
            if done[node] {
                continue;
            }
            done[node] = true;
            if node == 1 {
                break;
            }
            for j in 1..n {
                if done[j] || j == node {
                    continue;
                }
                let step = pts[node].distance(pts[j]);
                // A path through `j` costs at least this (each step costs
                // its length or more). If that is more than the best path to
                // `b` so far, `j` cannot be on the result: no test needed.
                if dist[node] + step + pts[j].distance(b) > dist[1] + EPS {
                    continue;
                }
                let factor = if along_edge(node, j) { 1.0 } else { interior_cost };
                let d = dist[node] + step * factor;
                if d + EPS < dist[j] && self.visible_nodes(&pts, node, j) {
                    dist[j] = d;
                    from[j] = node;
                    heap.push(Entry {
                        cost: d + pts[j].distance(b),
                        node: j,
                    });
                }
            }
        }
        if !done[1] {
            return None;
        }
        let mut path = vec![b];
        let mut k = 1;
        while k != 0 {
            k = from[k];
            path.push(pts[k]);
        }
        path.reverse();
        Some(path)
    }
}

/// Heap entry for A*: lowest cost first, then the lowest node index, so the result is deterministic.
#[derive(Debug, PartialEq)]
struct Entry {
    cost: f64,
    node: usize,
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost.total_cmp(&self.cost).then_with(|| o.node.cmp(&self.node))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// `p`-`q` crosses `a`-`b` at one point inside both segments. Touching and
/// overlapping segments do not cross.
fn crosses(p: Point, q: Point, a: Point, b: Point) -> bool {
    let tol = 1e-9 * (1.0 + p.distance(q) + a.distance(b));
    let o1 = (q - p).cross(a - p);
    let o2 = (q - p).cross(b - p);
    let o3 = (b - a).cross(p - a);
    let o4 = (b - a).cross(q - a);
    ((o1 > tol && o2 < -tol) || (o1 < -tol && o2 > tol)) && ((o3 > tol && o4 < -tol) || (o3 < -tol && o4 > tol))
}

/// Douglas-Peucker simplification of a closed ring.
fn simplify_ring(ring: &[Point], tol: f64) -> Vec<Point> {
    if ring.len() <= 4 {
        return ring.to_vec();
    }
    let mut open = ring.to_vec();
    open.push(ring[0]);
    let mut keep = vec![false; open.len()];
    keep[0] = true;
    // Split at the point farthest from the first point, so the closed ring keeps its shape.
    let far = (1..open.len() - 1)
        .max_by(|&i, &j| open[i].distance(open[0]).total_cmp(&open[j].distance(open[0])))
        .unwrap_or(1);
    keep[far] = true;
    let mut stack = vec![(0, far), (far, open.len() - 1)];
    while let Some((s, e)) = stack.pop() {
        let mut best = (0.0, 0);
        for i in s + 1..e {
            let d = open[i].distance_to_segment(open[s], open[e]);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tol {
            keep[best.1] = true;
            stack.push((s, best.1));
            stack.push((best.1, e));
        }
    }
    open.pop();
    open.into_iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| p).collect()
}

/// A needle path made of stitched pieces, joined by travel inside a shape.
pub struct Sewer<'a> {
    travel: &'a Travel,
    travel_len: f64,
    pub runs: Vec<Vec<Point>>,
    /// Trapunto: all travel goes along the edge.
    pub edge_only: bool,
}

impl<'a> Sewer<'a> {
    pub fn new(travel: &'a Travel, travel_len: f64) -> Self {
        Sewer {
            travel,
            travel_len,
            runs: Vec::new(),
            edge_only: false,
        }
    }

    /// The needle position, if anything is sewn.
    pub fn needle(&self) -> Option<Point> {
        self.runs.last().and_then(|r| r.last().copied())
    }

    /// Goes to `p` with travel stitches inside the shape, along the edge
    /// where it can. If there is no path inside the shape, a new needle path starts at `p`.
    pub fn go_to(&mut self, p: Point) {
        self.travel_to(p, true);
    }

    /// Starts a new needle path at `p` (the plan joins it with a jump or a
    /// trim), so no travel lies on top of other stitches.
    pub fn jump_to(&mut self, p: Point) {
        match self.needle() {
            Some(cur) if cur.distance(p) < EPS => {}
            _ => self.runs.push(vec![p]),
        }
    }

    /// Goes to `p` the shortest way inside the shape (a straight stitch if it can).
    pub fn step_to(&mut self, p: Point) {
        self.travel_to(p, self.edge_only);
    }

    fn travel_to(&mut self, p: Point, along_edge: bool) {
        let Some(cur) = self.needle() else {
            self.runs.push(vec![p]);
            return;
        };
        if cur.distance(p) < EPS {
            return;
        }
        let path = if along_edge {
            self.travel.edge_path(cur, p)
        } else {
            self.travel.path(cur, p)
        };
        match path {
            Some(path) => {
                let line = Polyline {
                    points: path,
                    closed: false,
                };
                let pts = running::running_points(&line, self.travel_len);
                self.runs
                    .last_mut()
                    .expect("needle is set")
                    .extend(pts.into_iter().skip(1));
            }
            None => self.runs.push(vec![p]),
        }
    }

    /// Adds needle points directly after the needle position. The first
    /// point must be reachable with a straight stitch.
    pub fn stitch(&mut self, pts: impl IntoIterator<Item = Point>) {
        for p in pts {
            match self.runs.last_mut() {
                Some(r) if r.last() != Some(&p) => r.push(p),
                Some(_) => {}
                None => self.runs.push(vec![p]),
            }
        }
    }

    pub fn finish(self) -> Vec<Vec<Point>> {
        self.runs.into_iter().filter(|r| r.len() >= 2).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_island_in_a_hole_stays() {
        // A 40 mm square with a 20 mm hole, and a 10 mm island in the hole
        // that has a 4 mm hole of its own.
        let outers = [rect(0.0, 0.0, 40.0, 40.0), rect(15.0, 15.0, 25.0, 25.0)];
        let holes = [rect(10.0, 10.0, 30.0, 30.0), rect(18.0, 18.0, 22.0, 22.0)];
        let s = Shape::new(&outers, &holes);
        let area: f64 = s.regions.iter().map(Region::area).sum();
        assert!((area - (1600.0 - 400.0 + 100.0 - 16.0)).abs() < 1e-6, "{area}");
        assert_eq!(s.regions.len(), 2);
        // With no island, as before.
        let s = Shape::new(&outers[..1], &holes[..1]);
        assert!((s.regions.iter().map(Region::area).sum::<f64>() - 1200.0).abs() < 1e-6);
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polyline {
        Polyline {
            points: vec![
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
                Point::new(x0, y1),
            ],
            closed: true,
        }
    }

    #[test]
    fn straight_line_when_visible() {
        let t = Travel::new(&Shape::new(&[rect(0.0, 0.0, 10.0, 10.0)], &[]));
        let p = t.path(Point::new(1.0, 1.0), Point::new(9.0, 9.0)).unwrap();
        assert_eq!(p.len(), 2);
        // Along the edge is inside.
        assert!(t.visible(Point::new(0.0, 0.0), Point::new(10.0, 0.0)));
    }

    #[test]
    fn goes_around_a_hole() {
        let shape = Shape::new(&[rect(0.0, 0.0, 30.0, 10.0)], &[rect(10.0, 2.0, 20.0, 10.0)]);
        // The hole touches the bottom edge, so the shape is a U. Travel from the
        // bottom of the left leg to the bottom of the right leg goes over the top.
        let t = Travel::new(&shape);
        let (a, b) = (Point::new(5.0, 9.0), Point::new(25.0, 9.0));
        assert!(!t.visible(a, b));
        let p = t.path(a, b).unwrap();
        assert!(p.len() > 2);
        for w in p.windows(2) {
            assert!(t.visible(w[0], w[1]), "{w:?}");
        }
        assert!(p.iter().all(|q| q.y < 2.0 || q.x < 10.0 || q.x > 20.0));
    }

    #[test]
    fn edge_path_follows_the_edge() {
        let t = Travel::new(&Shape::new(&[rect(0.0, 0.0, 20.0, 20.0)], &[]));
        // From near one corner to near the opposite corner: along two sides,
        // not across the middle.
        let p = t.edge_path(Point::new(1.0, 0.0), Point::new(19.0, 20.0)).unwrap();
        assert!(p.len() > 2, "{p:?}");
        let mid = p
            .iter()
            .map(|q| q.distance(Point::new(10.0, 10.0)))
            .fold(f64::INFINITY, f64::min);
        assert!(mid > 9.0, "{p:?}");
    }

    #[test]
    fn separate_areas_have_no_path() {
        let shape = Shape::new(&[rect(0.0, 0.0, 10.0, 10.0), rect(20.0, 0.0, 30.0, 10.0)], &[]);
        let t = Travel::new(&shape);
        assert_eq!(t.path(Point::new(5.0, 5.0), Point::new(25.0, 5.0)), None);
        let mut s = Sewer::new(&t, 3.0);
        s.stitch([Point::new(5.0, 5.0), Point::new(6.0, 5.0)]);
        s.go_to(Point::new(25.0, 5.0));
        s.stitch([Point::new(26.0, 5.0)]);
        assert_eq!(s.finish().len(), 2);
    }

    #[test]
    fn simplify_keeps_corners() {
        let mut ring = Vec::new();
        for i in 0..10 {
            ring.push(Point::new(f64::from(i), 0.0));
        }
        ring.push(Point::new(10.0, 10.0));
        ring.push(Point::new(0.0, 10.0));
        let s = simplify_ring(&ring, 0.1);
        assert_eq!(s.len(), 4);
    }
}
