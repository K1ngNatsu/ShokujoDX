//! The route of the needle through a letter (P17 Milestone 5, ADR 0113):
//! the order and the direction of its parts, and travel runs that go
//! from the end of one part to the start of the next one under the parts
//! that sew later, so the letter needs few trims.
//!
//! A person who digitizes a letter does the same: the travel stitches go
//! where a column covers them. The stitch engine alone can hide only a
//! straight travel from the needle to the next part; a routed letter has
//! its travel as a run that follows the middle of the columns.
//!
//! The search (over all orders and both directions of each column, as in
//! `emb_stitch` for the closest order, ADR 0018) takes the route with the
//! fewest trims, then the shortest travel. A routed letter is fixed
//! (`EmbLetter::fixed`): the stitch engine sews it as it is.

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::rc::Rc;

use emb_geom::{BBox, Path, Point};
use emb_model::{ObjectKind, PartTuning, RunningParams, SatinGeometry};

use crate::embfont::EmbLetter;
use crate::grid::{self, Grid};

/// The cells of the grid for each capital height.
const CELLS: f64 = 100.0;
/// A gap up to this (a part of the capital height) needs no run: the
/// stitch engine joins the parts with one stitch.
const NO_RUN: f64 = 0.012;
/// A run stays this far from the edge of the parts that hide it (a part
/// of the capital height), or a quarter of the half width of a narrow part.
const EDGE: f64 = 0.012;
/// An end of a part joins a run that starts at most this far from it (a
/// part of the capital height).
const SNAP: f64 = 0.12;
/// The stitch length of a travel run, in mm (at each size).
pub const RUN_STITCH_MM: f64 = 1.5;
/// Letters with more parts than this get no search of all orders: each
/// time the nearest part that needs no trim sews next.
const MAX_PARTS: usize = 8;
/// Letters with more parts than this keep their order.
const MOST_PARTS: usize = 24;
/// The number of steps along the end of a column where a run can start.
const EDGE_POINTS: usize = 8;
/// A letter goes on into the next letter if a stroke of it ends this near
/// to the end of its place in the line, or after it (a part of the capital
/// height).
const TOUCH: f64 = 0.03;
/// For a letter with a tail: a route that ends 1 mm nearer to the end of
/// the tail is as good as a route with this much less travel, in mm.
const TAIL_END: f64 = 12.0;
/// A route that starts 1 mm more to the left, or ends 1 mm more to the
/// right, is as good as a route with this much less travel, in mm: the
/// letter before is on the left and the next letter on the right, and
/// travel in the letter is under its columns.
const FLOW: f64 = 1.5;
/// A stroke that sews 1 mm against the direction of writing is as good as
/// a route with this much more travel, in mm. A pen makes a stroke down,
/// or to the right (a stroke that is more level than upright). The trims
/// come first: this is a choice between routes with the same trims.
const PEN: f64 = 0.5;

/// What [`route`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Routed {
    /// The trims in the letter.
    pub trims: usize,
    /// The travel runs in the letter.
    pub runs: usize,
    /// The columns that were cut at a junction.
    pub splits: usize,
}

/// The start or the end of a column: the ends of its two rails there. A
/// travel stitch along it lies on the end stitches of the column, so the
/// needle can leave or come to the column at each point of it.
type Edge = (Point, Point);

/// Points along an edge.
fn along(e: Edge) -> Vec<Point> {
    if e.0.distance(e.1) < 1e-9 {
        return vec![e.0];
    }
    (0..=EDGE_POINTS)
        .map(|i| e.0.lerp(e.1, i as f64 / EDGE_POINTS as f64))
        .collect()
}

/// One part of the letter on the grid.
struct Piece {
    /// The start and the end of a column or a run. `None`: a ring or a
    /// fill, which starts at the point of its outline nearest the needle.
    ends: Option<(Point, Point)>,
    /// The start edge and the end edge of a column or a run.
    edges: Option<(Edge, Edge)>,
    outline: Vec<Vec<Point>>,
    /// The cells of the part.
    mask: Vec<bool>,
    /// The distance of each cell to the edge of the part, in mm.
    dist: Vec<f64>,
    /// A run stays this far from the edge of this part, in mm.
    edge: f64,
    /// Half the width of the part, in mm.
    half: f64,
}

fn rings_of(path: &Path) -> Vec<Vec<Point>> {
    path.flatten(0.05).into_iter().map(|pl| pl.points).collect()
}

fn ends_of(path: &Path) -> Option<(Point, Point)> {
    let pts: Vec<Point> = path.points().collect();
    pts.first().copied().zip(pts.last().copied())
}

/// The start and the end of a part that has them.
fn part_ends(kind: &ObjectKind) -> Option<(Point, Point)> {
    match kind {
        ObjectKind::Satin { geometry, .. } => {
            let ((a0, a1), (b0, b1)) = (ends_of(&geometry.rail_a)?, ends_of(&geometry.rail_b)?);
            (a0.distance(a1) > 1e-6).then(|| (a0.midpoint(b0), a1.midpoint(b1)))
        }
        ObjectKind::Running { path, .. } => ends_of(path).filter(|(a, b)| a.distance(*b) > 1e-6),
        _ => None,
    }
}

/// The start edge and the end edge of a part that has them.
fn part_edges(kind: &ObjectKind) -> Option<(Edge, Edge)> {
    match kind {
        ObjectKind::Satin { geometry, .. } => {
            let ((a0, a1), (b0, b1)) = (ends_of(&geometry.rail_a)?, ends_of(&geometry.rail_b)?);
            (a0.distance(a1) > 1e-6).then_some(((a0, b0), (a1, b1)))
        }
        ObjectKind::Running { path, .. } => ends_of(path)
            .filter(|(a, b)| a.distance(*b) > 1e-6)
            .map(|(a, b)| ((a, a), (b, b))),
        _ => None,
    }
}

/// The area of a part: rings to fill with the nonzero rule, and rings to
/// take out of them (the hole of a ring column, the holes of a fill).
fn area_of(kind: &ObjectKind) -> (Vec<Vec<Point>>, Vec<Vec<Point>>) {
    match kind {
        ObjectKind::Satin { geometry, .. } => {
            let (a, b) = (geometry.rail_a.flatten(0.05), geometry.rail_b.flatten(0.05));
            match (a.first(), b.first()) {
                (Some(a), Some(b)) if a.closed && b.closed => {
                    let size =
                        |r: &[Point]| BBox::from_points(r.iter().copied()).map_or(0.0, |b| b.width() * b.height());
                    if size(&a.points) >= size(&b.points) {
                        (vec![a.points.clone()], vec![b.points.clone()])
                    } else {
                        (vec![b.points.clone()], vec![a.points.clone()])
                    }
                }
                (Some(a), Some(b)) => (
                    vec![a.points.iter().chain(b.points.iter().rev()).copied().collect()],
                    Vec::new(),
                ),
                _ => (Vec::new(), Vec::new()),
            }
        }
        ObjectKind::Fill { outer, holes, .. } => (rings_of(outer), holes.iter().flat_map(rings_of).collect()),
        _ => (Vec::new(), Vec::new()),
    }
}

/// The point of `rings` nearest `p`.
fn nearest_on(rings: &[Vec<Point>], p: Point) -> Point {
    let mut best = (f64::MAX, p);
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let ab = b - a;
            let t = if ab.length_sq() > 0.0 {
                ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let q = a + ab * t;
            if q.distance(p) < best.0 {
                best = (q.distance(p), q);
            }
        }
    }
    best.1
}

/// A way to sew a part: the part, and for a column `true` if it sews from
/// its end to its start.
type Mode = (usize, bool);

/// A connection from a point to the start of a part.
#[derive(Clone)]
struct Link {
    /// The run that hides the travel (`None`: the gap is short, or there
    /// is a trim).
    run: Option<Vec<Point>>,
    trim: bool,
    /// The length of the travel, or the gap with a trim.
    length: f64,
}

/// An edge as a key: its ends in steps of 0.001 mm.
type EdgeKey = [i64; 4];

fn edge_key(e: Edge) -> EdgeKey {
    let k = |v: f64| (v * 1000.0).round() as i64;
    [k(e.0.x), k(e.0.y), k(e.1.x), k(e.1.y)]
}

/// The passable cells that an edge can join: for each cell the length of
/// the join and the point of the edge.
type Joins = HashMap<usize, (f64, Point)>;

struct Router {
    grid: Grid,
    pieces: Vec<Piece>,
    cap: f64,
    /// The parts near each edge (see [`Router::near`]).
    near: RefCell<HashMap<EdgeKey, u32>>,
    /// The joins of each edge for each set of the parts near it that are
    /// not sewn.
    joins: RefCell<HashMap<(EdgeKey, u32), Rc<Joins>>>,
    links: HashMap<(usize, usize, usize, u32), Link>,
}

impl Router {
    fn new(parts: &[ObjectKind], cap: f64) -> Option<Router> {
        let bounds = parts.iter().filter_map(|p| p.control_bbox()).reduce(BBox::union)?;
        let cell = cap / CELLS;
        let grid = Grid::new(bounds, cell, 2);
        let pieces = parts
            .iter()
            .map(|kind| {
                let (fill, holes) = area_of(kind);
                let mut mask = vec![false; grid.w * grid.h];
                grid::fill_into(&grid, &fill, &mut mask);
                if !holes.is_empty() {
                    let mut out = vec![false; grid.w * grid.h];
                    grid::fill_into(&grid, &holes, &mut out);
                    for (m, o) in mask.iter_mut().zip(&out) {
                        *m &= !o;
                    }
                }
                let mut g = grid.clone();
                g.inside = mask.clone();
                let dist: Vec<f64> = grid::distance(&g).into_iter().map(|d| d * cell).collect();
                let half = dist.iter().copied().fold(0.0, f64::max);
                Piece {
                    ends: part_ends(kind),
                    edges: part_edges(kind),
                    outline: fill.into_iter().chain(holes).collect(),
                    mask,
                    dist,
                    edge: (EDGE * cap).min(0.25 * half).max(0.6 * cell),
                    half,
                }
            })
            .collect();
        Some(Router {
            grid,
            pieces,
            cap,
            near: RefCell::new(HashMap::new()),
            joins: RefCell::new(HashMap::new()),
            links: HashMap::new(),
        })
    }

    fn modes(&self) -> Vec<Mode> {
        self.pieces
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                if p.ends.is_some() {
                    vec![(i, false), (i, true)]
                } else {
                    vec![(i, false)]
                }
            })
            .collect()
    }

    /// Where the mode starts when the needle comes from `from`.
    fn start(&self, m: Mode, from: Point) -> Point {
        match self.pieces[m.0].ends {
            Some((a, b)) => {
                if m.1 {
                    b
                } else {
                    a
                }
            }
            None => nearest_on(&self.pieces[m.0].outline, from),
        }
    }

    /// The edge where the mode starts when the needle comes from `from`.
    fn start_edge(&self, m: Mode, from: Point) -> Edge {
        match self.pieces[m.0].edges {
            Some((a, b)) => {
                if m.1 {
                    b
                } else {
                    a
                }
            }
            None => {
                let p = self.start(m, from);
                (p, p)
            }
        }
    }

    /// The edge where the needle is after the mode that started at `start`.
    fn end_edge(&self, m: Mode, start: Point) -> Edge {
        match self.pieces[m.0].edges {
            Some((a, b)) => {
                if m.1 {
                    a
                } else {
                    b
                }
            }
            None => (start, start),
        }
    }

    /// Where the needle is after the mode that started at `start`.
    fn end(&self, m: Mode, start: Point) -> Point {
        match self.pieces[m.0].ends {
            Some((a, b)) => {
                if m.1 {
                    a
                } else {
                    b
                }
            }
            None => start,
        }
    }

    /// The cost of the direction of a mode (see [`PEN`]): how far the stroke
    /// goes up (Y is down), or to the left if it is more level than
    /// upright. A part with no ends has no direction.
    fn pen(&self, m: Mode) -> f64 {
        let Some((a, b)) = self.pieces[m.0].ends else {
            return 0.0;
        };
        let v = if m.1 { a - b } else { b - a };
        let against = if v.y.abs() >= v.x.abs() { -v.y } else { -v.x };
        PEN * against.max(0.0)
    }

    /// The cell is in a part of `unsewn`.
    fn covered(&self, idx: usize, unsewn: u32) -> bool {
        self.pieces
            .iter()
            .enumerate()
            .any(|(i, p)| unsewn & (1 << i) != 0 && p.mask[idx])
    }

    /// The middle of the cell in a part of `unsewn` that is nearest `p`,
    /// within [`SNAP`].
    fn nearest_covered(&self, p: Point, unsewn: u32) -> Option<Point> {
        let r = SNAP * self.cap;
        let n = (r / self.grid.cell).ceil() as isize;
        let (ci, cj) = self.grid.cell_of(p)?;
        let mut best: Option<(f64, Point)> = None;
        for dj in -n..=n {
            for di in -n..=n {
                let (i, j) = (ci as isize + di, cj as isize + dj);
                if i < 0 || j < 0 || i as usize >= self.grid.w || j as usize >= self.grid.h {
                    continue;
                }
                let idx = self.grid.idx(i as usize, j as usize);
                let c = self.grid.center_of(idx);
                let d = c.distance(p);
                if d <= r && self.covered(idx, unsewn) && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, c));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// A run can go through the cell: it is far enough from the edge of a
    /// part of `unsewn`. Returns how near the middle of that part it is
    /// (0: at the limit, 1: in the middle).
    fn passable(&self, idx: usize, unsewn: u32) -> Option<f64> {
        self.pieces
            .iter()
            .enumerate()
            .filter(|(i, p)| unsewn & (1 << i) != 0 && p.dist[idx] >= p.edge)
            .map(|(_, p)| (p.dist[idx] / (0.8 * p.half).max(1e-9)).min(1.0))
            .reduce(f64::max)
    }

    /// The straight line from `a` to `b` is under the parts of `unsewn`
    /// (but for its first and last bit).
    fn hidden(&self, a: Point, b: Point, unsewn: u32) -> bool {
        let free = NO_RUN * self.cap;
        let steps = (a.distance(b) / (0.5 * self.grid.cell)).ceil().max(1.0) as usize;
        (0..=steps).map(|i| a.lerp(b, i as f64 / steps as f64)).all(|p| {
            p.distance(a) <= free
                || p.distance(b) <= free
                || self
                    .grid
                    .cell_of(p)
                    .is_some_and(|(i, j)| self.covered(self.grid.idx(i, j), unsewn))
        })
    }

    /// The passable cells that a point of the edge `e` can join, each with
    /// the length of the join and the point of the edge: the cells within
    /// [`SNAP`] with a hidden line to the point.
    fn joins(&self, e: Edge, unsewn: u32) -> Rc<Joins> {
        // Only the parts near the edge change its joins.
        let key = (edge_key(e), unsewn & self.near(e));
        if let Some(j) = self.joins.borrow().get(&key) {
            return j.clone();
        }
        let mut out: Joins = HashMap::new();
        for p in along(e) {
            for (idx, d) in self.joins_of(p, key.1) {
                if out.get(&idx).is_none_or(|(best, _)| d < *best) {
                    out.insert(idx, (d, p));
                }
            }
        }
        let out = Rc::new(out);
        self.joins.borrow_mut().insert(key, out.clone());
        out
    }

    /// The parts with a cell within [`SNAP`] of a point of the edge `e`:
    /// only these can hide a join of the edge.
    fn near(&self, e: Edge) -> u32 {
        let key = edge_key(e);
        if let Some(n) = self.near.borrow().get(&key) {
            return *n;
        }
        let r = SNAP * self.cap;
        let n = (r / self.grid.cell).ceil() as isize + 1;
        let mut near = 0u32;
        for p in along(e) {
            let Some((ci, cj)) = self.grid.cell_of(p) else { continue };
            for dj in -n..=n {
                for di in -n..=n {
                    let (i, j) = (ci as isize + di, cj as isize + dj);
                    if i < 0 || j < 0 || i as usize >= self.grid.w || j as usize >= self.grid.h {
                        continue;
                    }
                    let idx = self.grid.idx(i as usize, j as usize);
                    for (k, piece) in self.pieces.iter().enumerate() {
                        if piece.mask[idx] {
                            near |= 1 << k;
                        }
                    }
                }
            }
        }
        self.near.borrow_mut().insert(key, near);
        near
    }

    fn joins_of(&self, p: Point, unsewn: u32) -> Vec<(usize, f64)> {
        let r = SNAP * self.cap;
        let n = (r / self.grid.cell).ceil() as isize;
        let Some((ci, cj)) = self.grid.cell_of(p) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for dj in -n..=n {
            for di in -n..=n {
                let (i, j) = (ci as isize + di, cj as isize + dj);
                if i < 0 || j < 0 || i as usize >= self.grid.w || j as usize >= self.grid.h {
                    continue;
                }
                let idx = self.grid.idx(i as usize, j as usize);
                let c = self.grid.center_of(idx);
                let d = c.distance(p);
                if d <= r && self.passable(idx, unsewn).is_some() && self.hidden(p, c, unsewn) {
                    out.push((idx, d));
                }
            }
        }
        out
    }

    /// The connection from the edge `from` to the edge `to` while the
    /// parts of `unsewn` (the part that starts at `to` with them) are not
    /// sewn.
    fn link(&self, from: Edge, to: Edge, unsewn: u32) -> Link {
        let (ps, qs) = (along(from), along(to));
        let pairs = || ps.iter().flat_map(|p| qs.iter().map(move |q| (*p, *q)));
        let gap = pairs().map(|(p, q)| p.distance(q)).fold(f64::MAX, f64::min);
        if gap <= NO_RUN * self.cap {
            return Link {
                run: None,
                trim: false,
                length: gap,
            };
        }
        let trim = Link {
            run: None,
            trim: true,
            length: gap,
        };
        // With no part near the first edge that sews later, nothing hides
        // the way from it.
        if unsewn & self.near(from) == 0 {
            return trim;
        }
        // A short straight run is the best one.
        let straight = pairs()
            .filter(|(p, q)| p.distance(*q) <= 2.0 * SNAP * self.cap && self.hidden(*p, *q, unsewn))
            .min_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)));
        if let Some((p, q)) = straight {
            return Link {
                run: Some(vec![p, q]),
                trim: false,
                length: p.distance(q),
            };
        }
        let starts = self.joins(from, unsewn);
        let goals = self.joins(to, unsewn);
        if starts.is_empty() || goals.is_empty() {
            return trim;
        }
        // The shortest way through the passable cells; a cell near the
        // edge of its part costs more, so the run goes down the middle.
        let size = self.grid.w * self.grid.h;
        let mut best = vec![f64::MAX; size];
        let mut prev = vec![usize::MAX; size];
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        let key = |c: f64| (c * 1e6) as u64;
        for (&idx, &(d, _)) in starts.iter() {
            if d < best[idx] {
                best[idx] = d;
                heap.push(Reverse((key(d), idx)));
            }
        }
        let mut found: Option<(f64, usize)> = None;
        while let Some(Reverse((k, idx))) = heap.pop() {
            if k > key(best[idx]) {
                continue;
            }
            if found.is_some_and(|(c, _)| best[idx] >= c) {
                break;
            }
            if let Some(&(d, _)) = goals.get(&idx) {
                let total = best[idx] + d;
                if found.is_none_or(|(c, _)| total < c) {
                    found = Some((total, idx));
                }
            }
            let here = self.grid.center_of(idx);
            for next in self.grid.neighbors(idx) {
                let Some(mid) = self.passable(next, unsewn) else {
                    continue;
                };
                let step = here.distance(self.grid.center_of(next));
                let cost = best[idx] + step * (1.0 + 4.0 * (1.0 - mid) * (1.0 - mid));
                if cost < best[next] {
                    best[next] = cost;
                    prev[next] = idx;
                    heap.push(Reverse((key(cost), next)));
                }
            }
        }
        let Some((_, mut idx)) = found else {
            return trim;
        };
        let mut cells = vec![goals[&idx].1, self.grid.center_of(idx)];
        while prev[idx] != usize::MAX {
            idx = prev[idx];
            cells.push(self.grid.center_of(idx));
        }
        cells.push(starts[&idx].1);
        cells.reverse();
        let run = self.simplify(&cells, unsewn);
        let length = run.windows(2).map(|w| w[0].distance(w[1])).sum();
        Link {
            run: Some(run),
            trim: false,
            length,
        }
    }

    /// Fewer points for the run `pts`: a point goes if the line between
    /// its neighbors stays in the passable cells.
    fn simplify(&self, pts: &[Point], unsewn: u32) -> Vec<Point> {
        let clear = |a: Point, b: Point| {
            let steps = (a.distance(b) / (0.5 * self.grid.cell)).ceil().max(1.0) as usize;
            (0..=steps).map(|i| a.lerp(b, i as f64 / steps as f64)).all(|p| {
                self.grid
                    .cell_of(p)
                    .is_some_and(|(i, j)| self.passable(self.grid.idx(i, j), unsewn).is_some())
            })
        };
        let last = pts.len() - 1;
        let mut out = vec![pts[0]];
        let mut at = 0;
        while at < last {
            // The farthest point that the line from `at` reaches. The first
            // and the last point join the run with a hidden line.
            let mut next = at + 1;
            for k in (at + 2..=last).rev() {
                let ok = if at == 0 || k == last {
                    self.hidden(pts[at], pts[k], unsewn) && k - at <= 3
                } else {
                    clear(pts[at], pts[k])
                };
                if ok {
                    next = k;
                    break;
                }
            }
            out.push(pts[next]);
            at = next;
        }
        out
    }

    /// The order of the modes for a letter with many parts: the part that
    /// starts nearest `entry` first, then each time the part that the
    /// needle comes to with no trim and the least travel (with a trim: the
    /// nearest part). Returns the order and the trims.
    fn nearest_next(&self, modes: &[Mode], all: u32, entry: Point) -> (Vec<usize>, usize) {
        let mut order: Vec<usize> = Vec::new();
        let (mut sewn, mut trims) = (0u32, 0usize);
        let mut at: Option<(Point, Edge)> = None;
        while sewn != all {
            let open = (0..modes.len()).filter(|j| sewn & (1 << modes[*j].0) == 0);
            let next = match at {
                None => open.min_by(|a, b| {
                    let d = |j: usize| self.start(modes[j], entry).distance(entry) + self.pen(modes[j]);
                    d(*a).total_cmp(&d(*b))
                }),
                Some((point, edge)) => open
                    .map(|j| (j, self.link(edge, self.start_edge(modes[j], point), all & !sewn)))
                    .min_by(|a, b| a.1.trim.cmp(&b.1.trim).then(a.1.length.total_cmp(&b.1.length)))
                    .map(|(j, link)| {
                        trims += usize::from(link.trim);
                        j
                    }),
            };
            let Some(j) = next else { break };
            let start = self.start(modes[j], at.map_or(entry, |a| a.0));
            order.push(j);
            sewn |= 1 << modes[j].0;
            at = Some((self.end(modes[j], start), self.end_edge(modes[j], start)));
        }
        (order, trims)
    }

    fn link_of(&mut self, from_key: usize, from: Edge, to: Mode, unsewn: u32) -> Link {
        let key = (from_key, to.0, usize::from(to.1), unsewn);
        if let Some(l) = self.links.get(&key) {
            return l.clone();
        }
        let start = self.start_edge(to, from.0.midpoint(from.1));
        let l = self.link(from, start, unsewn);
        self.links.insert(key, l.clone());
        l
    }
}

/// The letter with its parts in the order and the directions of the best
/// route, with travel runs, fixed. `cap_mm` is the capital height of the
/// letters of the font. A letter with more than [`MAX_PARTS`] parts gets a
/// simple route (the nearest part next). A letter with a part that is not a
/// column, a fill or a run stays as it is.
pub fn route(letter: &EmbLetter, cap_mm: f64) -> (EmbLetter, Routed) {
    let n = letter.parts.len();
    let keep = |trims: usize| {
        let mut l = letter.clone();
        l.fixed = true;
        (
            l,
            Routed {
                trims,
                ..Default::default()
            },
        )
    };
    let supported = letter.parts.iter().all(|p| {
        matches!(
            p,
            ObjectKind::Satin { .. } | ObjectKind::Fill { .. } | ObjectKind::Running { .. }
        )
    });
    if n == 0 || !supported {
        return (letter.clone(), Routed::default());
    }
    let Some(mut r) = Router::new(&letter.parts, cap_mm) else {
        return (letter.clone(), Routed::default());
    };
    if n > MOST_PARTS {
        return keep(n - 1);
    }
    let modes = r.modes();
    let all: u32 = (1 << n) - 1;
    // The left end of the baseline: where the first part starts if it has
    // no ends.
    let entry = Point::new(r.grid.origin.x, 0.0);
    // A letter of a script goes on into the next letter: its last stroke
    // (the tail) ends after its place in the line. The letter before it
    // does the same, so its tail ends in this letter, at about the same
    // height. The letter starts there (`lead`): a run goes from that place
    // to the first part, under the letter. And the letter ends at the end
    // of its tail, where the next letter starts.
    let tail = r
        .pieces
        .iter()
        .filter_map(|p| p.ends)
        .flat_map(|(a, b)| [a, b])
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .filter(|p| p.x >= letter.advance_mm - TOUCH * cap_mm);
    let lead = tail.and_then(|p| r.nearest_covered(Point::new(p.x - letter.advance_mm, p.y), all));
    let (order, trims): (Vec<usize>, usize) = if n > MAX_PARTS {
        // Too many parts for the search of all orders.
        r.nearest_next(&modes, all, lead.unwrap_or(entry))
    } else {
        let size = 1usize << n;
        // best[mask][mode] = (trims, travel, the mode before, the start point)
        type Step = (u32, f64, usize, Point);
        let mut best: Vec<Vec<Option<Step>>> = vec![vec![None; modes.len()]; size];
        for (j, m) in modes.iter().enumerate() {
            let start = r.start(*m, lead.unwrap_or(entry));
            // With a lead, the run from it to the first part is travel.
            let cost = match lead {
                Some(p) => p.distance(start),
                None => FLOW * (start.x - entry.x),
            };
            best[1 << m.0][j] = Some((0, cost + r.pen(*m), usize::MAX, start));
        }
        for mask in 1..size {
            for i in 0..modes.len() {
                let Some((t0, d0, _, s0)) = best[mask][i] else { continue };
                if mask & (1 << modes[i].0) == 0 {
                    continue;
                }
                let from = r.end(modes[i], s0);
                let from_edge = r.end_edge(modes[i], s0);
                let unsewn = all & !(mask as u32);
                for (j, m) in modes.iter().enumerate() {
                    if mask & (1 << m.0) != 0 {
                        continue;
                    }
                    // A part with no ends starts where the needle is: the key
                    // of its link has the place of the needle.
                    let from_key = if r.pieces[modes[i].0].ends.is_some() {
                        i
                    } else {
                        modes.len() + mask
                    };
                    let link = r.link_of(from_key, from_edge, *m, unsewn);
                    let cost = (t0 + u32::from(link.trim), d0 + link.length + r.pen(*m));
                    let next = mask | (1 << m.0);
                    let better = best[next][j].is_none_or(|b| cost.0 < b.0 || (cost.0 == b.0 && cost.1 < b.1 - 1e-9));
                    if better {
                        best[next][j] = Some((cost.0, cost.1, i, r.start(*m, from)));
                    }
                }
            }
        }
        // A route that ends on the right is better (the next letter is there).
        // A letter with a tail ends at the end of its tail if it can.
        let flow = |j: usize| {
            best[size - 1][j].map_or(0.0, |(_, _, _, s)| {
                let end = r.end(modes[j], s);
                match tail {
                    Some(t) => TAIL_END * end.distance(t),
                    None => -FLOW * end.x,
                }
            })
        };
        let Some(mut j) = (0..modes.len())
            .filter(|&j| best[size - 1][j].is_some())
            .min_by(|&a, &b| {
                let (ta, da, ..) = best[size - 1][a].unwrap_or((u32::MAX, 0.0, 0, entry));
                let (tb, db, ..) = best[size - 1][b].unwrap_or((u32::MAX, 0.0, 0, entry));
                ta.cmp(&tb).then((da + flow(a)).total_cmp(&(db + flow(b))))
            })
        else {
            return keep(n - 1);
        };
        let trims = best[size - 1][j].map_or(0, |b| b.0 as usize);
        let mut order: Vec<usize> = Vec::with_capacity(n);
        let mut mask = size - 1;
        loop {
            order.push(j);
            let Some((_, _, prev, _)) = best[mask][j] else { break };
            mask &= !(1 << modes[j].0);
            if prev == usize::MAX {
                break;
            }
            j = prev;
        }
        order.reverse();
        (order, trims)
    };
    // The parts in the order of the route, with the runs between them.
    let tuned = letter.tuning.len() == n;
    let mut parts = Vec::new();
    let mut tuning = Vec::new();
    let mut runs = 0;
    let mut sewn: u32 = 0;
    let mut needle: Option<(Point, Edge)> = lead.map(|p| (p, (p, p)));
    for &j in &order {
        let m = modes[j];
        let start = r.start(m, needle.map_or(entry, |n| n.0));
        if let Some((from, edge)) = needle
            && let Some(run) = r.link(edge, r.start_edge(m, from), all & !sewn).run
        {
            let mut path = Path::new().move_to(run[0]);
            for p in &run[1..] {
                path = path.line_to(*p);
            }
            parts.push(ObjectKind::Running {
                path,
                params: RunningParams {
                    stitch_length_mm: RUN_STITCH_MM,
                    ..Default::default()
                },
            });
            tuning.push(PartTuning::default());
            runs += 1;
        }
        let mut part = letter.parts[m.0].clone();
        if m.1 {
            emb_model::editing::reverse_kind(&mut part);
        }
        parts.push(part);
        tuning.push(if tuned {
            letter.tuning[m.0].clone()
        } else {
            PartTuning::default()
        });
        sewn |= 1 << m.0;
        needle = Some((r.end(m, start), r.end_edge(m, start)));
    }
    if tuning.iter().all(PartTuning::is_empty) {
        tuning.clear();
    }
    (
        EmbLetter {
            advance_mm: letter.advance_mm,
            parts,
            fixed: true,
            tuning,
        },
        Routed { trims, runs, splits: 0 },
    )
}

/// The two halves of a seam go this far past it (a part of the capital
/// height), so the seam has no gap.
const SEAM_OVERLAP: f64 = 0.01;
/// A column is cut only this far from its ends, as a factor of its width.
const SPLIT_FROM_END: f64 = 0.9;
/// At most this many cuts in one letter.
const MAX_SPLITS: usize = 2;

/// The index of the point of `pts` nearest `p`, and the point of the line
/// nearest `p` on the segment after it or before it.
fn nearest_at(pts: &[Point], p: Point) -> (usize, Point) {
    let mut best = (f64::MAX, 0, pts[0]);
    for i in 0..pts.len() - 1 {
        let (a, b) = (pts[i], pts[i + 1]);
        let ab = b - a;
        let t = if ab.length_sq() > 0.0 {
            ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + ab * t;
        if q.distance(p) < best.0 {
            best = (q.distance(p), i, q);
        }
    }
    (best.1, best.2)
}

fn line_path(pts: &[Point]) -> Path {
    let mut path = Path::new().move_to(pts[0]);
    for p in &pts[1..] {
        path = path.line_to(*p);
    }
    path
}

/// The column `g` as two columns that meet at the place nearest `at`: the
/// first one from the start of `g`, the second one to its end. Each goes
/// `overlap` past the cut.
fn split_satin(g: &SatinGeometry, at: Point, overlap: f64) -> Option<(SatinGeometry, SatinGeometry)> {
    let flat = |p: &Path| -> Vec<Point> { p.flatten(0.02).into_iter().flat_map(|pl| pl.points).collect() };
    let (ra, rb) = (flat(&g.rail_a), flat(&g.rail_b));
    if ra.len() < 2 || rb.len() < 2 {
        return None;
    }
    let cut = |pts: &[Point]| -> Option<(Vec<Point>, Vec<Point>)> {
        let (i, q) = nearest_at(pts, at);
        let d = (pts[i + 1] - pts[i]).normalized()?;
        let mut first: Vec<Point> = pts[..=i].iter().copied().filter(|p| p.distance(q) > 1e-6).collect();
        first.push(q + d * overlap);
        let mut second = vec![q - d * overlap];
        second.extend(pts[i + 1..].iter().copied().filter(|p| p.distance(q) > 1e-6));
        (first.len() >= 2 && second.len() >= 2).then_some((first, second))
    };
    let ((a1, a2), (b1, b2)) = (cut(&ra)?, cut(&rb)?);
    // Each rung goes to the half that its point on the first rail is in.
    let (ia, qa) = nearest_at(&ra, at);
    let before = |p: Point| {
        let (i, q) = nearest_at(&ra, p);
        i < ia || (i == ia && q.distance(ra[i]) < qa.distance(ra[i]))
    };
    let (r1, r2): (Vec<Edge>, Vec<Edge>) = g.rungs.iter().partition(|(p, _)| before(*p));
    Some((
        SatinGeometry {
            rail_a: line_path(&a1),
            rail_b: line_path(&b1),
            rungs: r1,
        },
        SatinGeometry {
            rail_a: line_path(&a2),
            rail_b: line_path(&b2),
            rungs: r2,
        },
    ))
}

/// The places where a column of `letter` can be cut: (part, point). A
/// column can be cut where another part lies on it, away from its ends: a
/// junction (the bar of "H" on its stem) or a crossing ("X").
fn split_places(letter: &EmbLetter, cap: f64) -> Vec<(usize, Point)> {
    let Some(r) = Router::new(&letter.parts, cap) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (i, x) in r.pieces.iter().enumerate() {
        let (Some((start, end)), true) = (x.ends, matches!(letter.parts[i], ObjectKind::Satin { .. })) else {
            continue;
        };
        for (j, y) in r.pieces.iter().enumerate() {
            if i == j {
                continue;
            }
            // The middle of the cells that the two parts share.
            let (mut n, mut sx, mut sy) = (0usize, 0.0, 0.0);
            for (idx, _) in x.mask.iter().zip(&y.mask).enumerate().filter(|(_, (a, b))| **a && **b) {
                let c = r.grid.center_of(idx);
                n += 1;
                sx += c.x;
                sy += c.y;
            }
            if n < 4 {
                continue;
            }
            let c = Point::new(sx / n as f64, sy / n as f64);
            let away = SPLIT_FROM_END * 2.0 * x.half;
            if c.distance(start) > away && c.distance(end) > away {
                out.push((i, c));
            }
        }
    }
    out
}

/// `letter` with the column `part` cut at `at`.
fn with_split(letter: &EmbLetter, part: usize, at: Point, cap: f64) -> Option<EmbLetter> {
    let ObjectKind::Satin { geometry, params } = &letter.parts[part] else {
        return None;
    };
    let (first, second) = split_satin(geometry, at, SEAM_OVERLAP * cap)?;
    let mut out = letter.clone();
    let half = |geometry: SatinGeometry| ObjectKind::Satin {
        geometry,
        params: params.clone(),
    };
    out.parts.splice(part..=part, [half(first), half(second)]);
    if out.tuning.len() == letter.parts.len() {
        let t = out.tuning[part].clone();
        out.tuning.insert(part, t);
    }
    Some(out)
}

/// [`route`], and with cuts: if the letter still has trims, a column is
/// cut at a junction or a crossing where that gives fewer trims (the stem
/// of "H" at its bar: each half sews to the bar). At most [`MAX_SPLITS`]
/// cuts.
pub fn route_with_splits(letter: &EmbLetter, cap_mm: f64) -> (EmbLetter, Routed) {
    let mut source = letter.clone();
    let (mut best, mut found) = route(&source, cap_mm);
    for _ in 0..MAX_SPLITS {
        if found.trims == 0 || source.parts.len() >= MAX_PARTS {
            break;
        }
        let mut better: Option<(EmbLetter, EmbLetter, Routed)> = None;
        for (part, at) in split_places(&source, cap_mm) {
            let Some(cut) = with_split(&source, part, at, cap_mm) else {
                continue;
            };
            let (routed, f) = route(&cut, cap_mm);
            if f.trims < better.as_ref().map_or(found.trims, |b| b.2.trims) {
                better = Some((cut, routed, f));
            }
        }
        let Some((cut, routed, f)) = better else { break };
        source = cut;
        best = routed;
        found = Routed {
            splits: found.splits + 1,
            ..f
        };
    }
    (best, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(from: (f64, f64), to: (f64, f64), width: f64) -> ObjectKind {
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(
                &Path::new()
                    .move_to(Point::new(from.0, from.1))
                    .line_to(Point::new(to.0, to.1)),
                width,
            ),
            params: Default::default(),
        }
    }

    fn letter(parts: Vec<ObjectKind>) -> EmbLetter {
        EmbLetter {
            advance_mm: 20.0,
            parts,
            fixed: false,
            tuning: Vec::new(),
        }
    }

    fn runs(l: &EmbLetter) -> Vec<Vec<Point>> {
        l.parts
            .iter()
            .filter_map(|p| match p {
                ObjectKind::Running { path, .. } => Some(path.points().collect()),
                _ => None,
            })
            .collect()
    }

    /// The stem and the three arms of an "E", 25 mm high, columns 4 mm
    /// wide. The arms go into the stem.
    fn e() -> EmbLetter {
        letter(vec![
            column((2.0, 0.0), (2.0, -25.0), 4.0),
            column((3.0, -23.0), (15.0, -23.0), 4.0),
            column((3.0, -12.5), (13.0, -12.5), 4.0),
            column((3.0, -2.0), (15.0, -2.0), 4.0),
        ])
    }

    #[test]
    fn an_e_sews_with_no_trim() {
        let before = e();
        let (after, found) = route(&before, 25.0);
        assert_eq!(found.trims, 0, "{found:?}");
        assert!(after.fixed);
        assert!(found.runs >= 1 && runs(&after).len() == found.runs);
        // All four columns are there, each one time.
        let columns: Vec<&ObjectKind> = after
            .parts
            .iter()
            .filter(|p| matches!(p, ObjectKind::Satin { .. }))
            .collect();
        assert_eq!(columns.len(), 4);
        // Each run is under columns that sew after it, away from their
        // edges: each point of it is in such a column.
        let area = |k: &ObjectKind, p: Point| {
            let (fill, _) = area_of(k);
            fill.iter().any(|r| emb_geom::region::ring_contains(r, p))
        };
        for (i, part) in after.parts.iter().enumerate() {
            let ObjectKind::Running { path, params } = part else {
                continue;
            };
            assert_eq!(params.stitch_length_mm, RUN_STITCH_MM);
            let pts: Vec<Point> = path.points().collect();
            for w in pts.windows(2) {
                for k in 1..10 {
                    let p = w[0].lerp(w[1], f64::from(k) / 10.0);
                    assert!(
                        after.parts[i + 1..].iter().any(|later| area(later, p)),
                        "the run point {p:?} is not under a later column"
                    );
                }
            }
            // The run starts on the end of the part before it, and ends on
            // the start of the next part.
            let on = |e: Edge, p: Point| {
                let ab = e.1 - e.0;
                let t = ((p - e.0).dot(ab) / ab.length_sq().max(1e-12)).clamp(0.0, 1.0);
                (e.0 + ab * t).distance(p) < 1e-6
            };
            let before_end = part_edges(&after.parts[i - 1]).unwrap().1;
            let next_start = part_edges(&after.parts[i + 1]).unwrap().0;
            assert!(on(before_end, pts[0]) && on(next_start, pts[pts.len() - 1]));
        }
        // The same letter gives the same route.
        assert_eq!(route(&before, 25.0).0, after);
    }

    #[test]
    fn a_column_is_cut_at_a_junction_if_that_saves_a_trim() {
        // An "H": with whole stems, the bar starts on a stem that is sewn.
        let h = letter(vec![
            column((2.0, 0.0), (2.0, -25.0), 4.0),
            column((3.0, -12.5), (15.0, -12.5), 4.0),
            column((16.0, 0.0), (16.0, -25.0), 4.0),
        ]);
        assert_eq!(route(&h, 25.0).1.trims, 1);
        let (after, found) = route_with_splits(&h, 25.0);
        assert_eq!((found.trims, found.splits), (0, 1), "{found:?}");
        let columns = |l: &EmbLetter| l.parts.iter().filter(|p| matches!(p, ObjectKind::Satin { .. })).count();
        assert_eq!(columns(&after), 4, "a stem is two columns");
        assert!(after.fixed);
        // The two halves of the stem cover the stem.
        let area: f64 = after
            .parts
            .iter()
            .filter_map(|p| p.control_bbox())
            .filter(|b| b.center().x < 5.0 && b.width() < 5.0)
            .map(|b| b.height())
            .sum();
        assert!(area >= 25.0, "{area}");
        // An "X": one stroke is cut where the other one crosses it.
        let x = letter(vec![
            column((0.0, 0.0), (16.0, -25.0), 4.0),
            column((16.0, 0.0), (0.0, -25.0), 4.0),
        ]);
        assert_eq!(route(&x, 25.0).1.trims, 1);
        let (after, found) = route_with_splits(&x, 25.0);
        assert_eq!((found.trims, found.splits), (0, 1), "{found:?}");
        assert_eq!(columns(&after), 3);
        // A letter with no trim has no cut.
        let (after, found) = route_with_splits(&e(), 25.0);
        assert_eq!((found.trims, found.splits), (0, 0));
        assert_eq!(columns(&after), 4);
    }

    #[test]
    fn parts_that_do_not_touch_need_a_trim() {
        // An "i": the dot is away from the stem.
        let i = letter(vec![
            column((2.0, 0.0), (2.0, -17.0), 4.0),
            column((2.0, -21.0), (2.0, -25.0), 4.0),
        ]);
        let (after, found) = route(&i, 25.0);
        assert_eq!(
            found,
            Routed {
                trims: 1,
                ..Default::default()
            }
        );
        assert_eq!(after.parts.len(), 2);
        // One part that sews down: no change but the fixed order.
        let one = letter(vec![column((2.0, -25.0), (2.0, 0.0), 4.0)]);
        let (after, found) = route(&one, 25.0);
        assert_eq!(found, Routed::default());
        assert!(after.fixed && after.parts == one.parts);
        // One part that sews up: it sews down, as a pen makes the stroke.
        let up = letter(vec![column((2.0, 0.0), (2.0, -25.0), 4.0)]);
        let (after, _) = route(&up, 25.0);
        let (start, end) = part_ends(&after.parts[0]).unwrap();
        assert!(start.y < end.y, "{start:?} {end:?}");
        // One part from the right to the left: it sews to the right, where
        // the next letter is.
        let back = letter(vec![column((18.0, -2.0), (2.0, -2.0), 4.0)]);
        let (after, _) = route(&back, 25.0);
        let (start, end) = part_ends(&after.parts[0]).unwrap();
        assert!(start.x < end.x, "{start:?} {end:?}");
        // No parts.
        assert_eq!(route(&letter(Vec::new()), 25.0).1, Routed::default());
    }

    #[test]
    fn the_settings_of_the_parts_go_with_them() {
        let mut l = e();
        l.tuning = vec![PartTuning::default(); 4];
        l.tuning[2].pull_mm = 0.1;
        let (after, _) = route(&l, 25.0);
        assert_eq!(after.tuning.len(), after.parts.len());
        let middle = after
            .parts
            .iter()
            .position(|p| {
                p.control_bbox()
                    .is_some_and(|b| (b.center().y + 12.5).abs() < 0.1 && b.height() < 5.0)
            })
            .unwrap();
        assert_eq!(after.tuning[middle].pull_mm, 0.1);
        assert!(
            after
                .tuning
                .iter()
                .enumerate()
                .all(|(i, t)| i == middle || t.is_empty())
        );
    }

    #[test]
    fn a_letter_of_a_font_gets_fewer_trims() {
        let source = crate::bundled::font("Lato", "Bold").unwrap();
        let chars: Vec<char> = "EFHKTMW".chars().collect();
        let f = crate::embfont::convert(&source, "Block Sans", "", emb_model::LetterStitch::Auto, &chars);
        let mut trims = 0;
        for c in "EFHKTMW".chars() {
            let (after, found) = route(&f.letters[&c], f.cap_height_mm);
            assert!(after.fixed);
            trims += found.trims;
        }
        // The stitch engine alone needs 17 trims for these letters.
        assert!(trims <= 6, "{trims} trims");
    }
}
