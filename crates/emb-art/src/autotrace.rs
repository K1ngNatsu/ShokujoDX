//! Auto-trace: the color areas of a reduced picture as vector shapes (P6
//! Milestone 6; ADR 0028). Our own method:
//!
//! 1. **Areas.** The pixels of one color that touch at a side are one
//!    area. Two pixels of one color that touch only at a corner, where the
//!    other two pixels have one other color, are one area if their color is
//!    the less common of the two (or the other pixels are transparent). So a
//!    thin diagonal line stays one area, and the color behind it is cut there.
//! 2. **Chains.** The pixel sides between two colors are the edges. A pixel
//!    corner where three or four edges meet is a node. The edges from one
//!    node to the next are a chain; an edge loop with no node is a closed
//!    chain.
//! 3. **Smoothing.** Each chain is smoothed once. A turn of more than
//!    `corner_deg` over a few pixels stays a sharp corner. Between two
//!    corners, the middle points of the pixel sides are averaged over a few
//!    pixels (this removes the steps of a slanted edge), and then the
//!    fewest cubic curves that stay within `tolerance_px` of them are fitted
//!    (the curve fit of `emb_geom::simplify`); a straight part becomes a line.
//! 4. **Rings.** Each area gets its outline and its holes from the smoothed
//!    chains. Two areas that touch use the same chain (in opposite
//!    directions), so traced areas have no gap and no overlap.
//!
//! Coordinates are in pixels: (0, 0) is the top-left corner of the
//! picture, and Y goes down. The result is the same each time (ADR 0003).

use std::collections::HashMap;

use emb_geom::simplify::{fit_loop, fit_points};
use emb_geom::{Path, PathCmd, Point, Transform, math};

use crate::reduce::{NONE, Reduced};

/// A turn is measured over this many pixel sides before and after a corner.
const CORNER_REACH: usize = 4;
/// The middle points are averaged over this many points on each side...
const SMOOTH_REACH: usize = 4;
/// ...this many times.
const SMOOTH_PASSES: usize = 2;
/// The area of a pixel that is in no area.
const NO_AREA: u32 = u32::MAX;

/// Settings of [`trace`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceSettings {
    /// The largest distance of the curves from the pixel edges, in pixels.
    pub tolerance_px: f64,
    /// A turn larger than this, in degrees, stays a sharp corner.
    pub corner_deg: f64,
    /// Areas and holes with fewer pixels than this are left out.
    pub min_area_px: usize,
}

impl Default for TraceSettings {
    fn default() -> Self {
        TraceSettings {
            tolerance_px: 0.8,
            corner_deg: 60.0,
            min_area_px: 4,
        }
    }
}

/// One traced area: its outline and its holes, in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    /// The index of the area in its [`AreaMap`].
    pub index: usize,
    /// The index of its color in the palette.
    pub color: u16,
    /// The number of pixels of the area.
    pub pixels: usize,
    /// The mean width of the area in pixels, from a distance map (see
    /// [`AreaMap::width`]). It stays in pixels when the area is transformed.
    pub width_px: f64,
    /// The center of the pixel of the area that is farthest from its edge:
    /// a point inside it. It moves with the area.
    pub inside: Point,
    pub outer: Path,
    pub holes: Vec<Path>,
    /// The area of each hole in pixels, from the pixel edges (before the
    /// smoothing, which makes a thin hole larger).
    pub hole_px: Vec<f64>,
}

impl Area {
    /// The outline and the holes in one path.
    pub fn path(&self) -> Path {
        Path::joined(std::iter::once(&self.outer).chain(&self.holes))
    }

    /// The area with each point moved by `t` (for example, from pixels to mm).
    pub fn transformed(&self, t: &Transform) -> Area {
        Area {
            index: self.index,
            color: self.color,
            pixels: self.pixels,
            width_px: self.width_px,
            inside: t.apply(self.inside),
            outer: self.outer.transformed(t),
            holes: self.holes.iter().map(|h| h.transformed(t)).collect(),
            hole_px: self.hole_px.clone(),
        }
    }
}

/// The color of a pixel; [`NONE`] outside the picture.
fn label(r: &Reduced, x: i64, y: i64) -> u16 {
    if x < 0 || y < 0 || x >= i64::from(r.width) || y >= i64::from(r.height) {
        return NONE;
    }
    r.labels[(y * i64::from(r.width) + x) as usize]
}

/// Two pixels of color `l` that touch only at a corner, where the two other
/// pixels have color `m`, are one area.
fn joins(l: u16, m: u16) -> bool {
    l != NONE && l != m && (m == NONE || l > m)
}

/// The connected areas of a reduced picture (step 1 of the module notes).
#[derive(Debug, Clone)]
pub struct AreaMap {
    width: u32,
    /// The area of each pixel, row by row ([`NO_AREA`] if transparent).
    of: Vec<u32>,
    colors: Vec<u16>,
    sizes: Vec<usize>,
    widths: Vec<f64>,
    /// The pixel of each area that is farthest from its edge.
    deepest: Vec<usize>,
}

impl AreaMap {
    pub fn new(r: &Reduced) -> AreaMap {
        let w = i64::from(r.width);
        let mut map = AreaMap {
            width: r.width,
            of: vec![NO_AREA; r.labels.len()],
            colors: Vec::new(),
            sizes: Vec::new(),
            widths: Vec::new(),
            deepest: Vec::new(),
        };
        let mut stack = Vec::new();
        for start in 0..r.labels.len() {
            let l = r.labels[start];
            if l == NONE || map.of[start] != NO_AREA {
                continue;
            }
            let id = map.colors.len() as u32;
            map.of[start] = id;
            stack.push(start);
            let mut size = 0;
            while let Some(k) = stack.pop() {
                size += 1;
                let (x, y) = (k as i64 % w, k as i64 / w);
                let mut visit = |qx: i64, qy: i64, of: &mut Vec<u32>| {
                    let q = (qy * w + qx) as usize;
                    if of[q] == NO_AREA {
                        of[q] = id;
                        stack.push(q);
                    }
                };
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if label(r, x + dx, y + dy) == l {
                        visit(x + dx, y + dy, &mut map.of);
                    }
                }
                for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                    let m = label(r, x + dx, y);
                    if label(r, x + dx, y + dy) == l && label(r, x, y + dy) == m && joins(l, m) {
                        visit(x + dx, y + dy, &mut map.of);
                    }
                }
            }
            map.colors.push(l);
            map.sizes.push(size);
        }
        (map.widths, map.deepest) = map.mean_widths(r.height);
        map
    }

    /// The mean width of each area (P12 Milestone 1). A distance map gives
    /// each pixel its distance to the nearest pixel of another area (1 at
    /// the edge; steps of 1 to the side and √2 on the diagonal); across a
    /// band `w` pixels wide the mean distance is about `(w + 2) / 4`, so
    /// the width is 4 × the mean distance, less 2. A ragged edge (grain of
    /// a photo) changes only the pixels near it, so a large area with a
    /// ragged edge stays wide; twice the area over the edge length (the
    /// measure before) made it narrow.
    /// Also gives the pixel of each area that is farthest from its edge.
    fn mean_widths(&self, height: u32) -> (Vec<f64>, Vec<usize>) {
        let (w, h) = (self.width as usize, height as usize);
        let at = |x: i64, y: i64| -> u32 {
            if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                NO_AREA
            } else {
                self.of[y as usize * w + x as usize]
            }
        };
        let mut d = vec![f64::INFINITY; self.of.len()];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let a = at(x, y);
                if a == NO_AREA {
                    continue;
                }
                if [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|&(dx, dy)| at(x + dx, y + dy) != a)
                {
                    d[y as usize * w + x as usize] = 1.0;
                }
            }
        }
        let s = std::f64::consts::SQRT_2;
        let relax = |d: &mut Vec<f64>, x: i64, y: i64, near: &[(i64, i64, f64)]| {
            let k = y as usize * w + x as usize;
            let a = self.of[k];
            if a == NO_AREA {
                return;
            }
            for &(dx, dy, step) in near {
                if at(x + dx, y + dy) == a {
                    let q = d[(y + dy) as usize * w + (x + dx) as usize] + step;
                    if q < d[k] {
                        d[k] = q;
                    }
                }
            }
        };
        let before = [(-1, 0, 1.0), (0, -1, 1.0), (-1, -1, s), (1, -1, s)];
        let after = [(1, 0, 1.0), (0, 1, 1.0), (1, 1, s), (-1, 1, s)];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                relax(&mut d, x, y, &before);
            }
        }
        for y in (0..h as i64).rev() {
            for x in (0..w as i64).rev() {
                relax(&mut d, x, y, &after);
            }
        }
        let n = self.colors.len();
        let mut sums = vec![0.0; n];
        let mut most = vec![0.0f64; n];
        for (k, &a) in self.of.iter().enumerate() {
            if a != NO_AREA {
                sums[a as usize] += d[k];
                most[a as usize] = most[a as usize].max(d[k]);
            }
        }
        // Of the pixels farthest from the edge, the one nearest to their
        // center: the middle of a thin line, not its end (an end pixel can
        // be outside the smoothed outline).
        let mut center = vec![(0.0, 0.0, 0.0); n];
        for (k, &a) in self.of.iter().enumerate() {
            if a != NO_AREA && d[k] == most[a as usize] {
                let c = &mut center[a as usize];
                *c = (c.0 + (k % w) as f64, c.1 + (k / w) as f64, c.2 + 1.0);
            }
        }
        let mut deepest = vec![usize::MAX; n];
        let mut best = vec![f64::INFINITY; n];
        for (k, &a) in self.of.iter().enumerate() {
            let a = a as usize;
            if a == NO_AREA as usize || d[k] != most[a] {
                continue;
            }
            let (cx, cy, m) = center[a];
            let e = ((k % w) as f64 - cx / m).powi(2) + ((k / w) as f64 - cy / m).powi(2);
            if e < best[a] {
                (best[a], deepest[a]) = (e, k);
            }
        }
        let widths = sums
            .iter()
            .zip(&self.sizes)
            .map(|(s, &n)| (4.0 * s / n.max(1) as f64 - 2.0).max(1.0))
            .collect();
        (widths, deepest)
    }

    /// The number of areas.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// The area of the pixel at `(x, y)`, if it has a color.
    pub fn at(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width {
            return None;
        }
        let k = y as usize * self.width as usize + x as usize;
        self.of.get(k).copied().filter(|&a| a != NO_AREA).map(|a| a as usize)
    }

    /// The palette index of the color of area `a`.
    pub fn color(&self, a: usize) -> u16 {
        self.colors[a]
    }

    /// The number of pixels of area `a`.
    pub fn pixels(&self, a: usize) -> usize {
        self.sizes[a]
    }

    /// The mean width of area `a` in pixels (see `mean_widths`).
    pub fn width(&self, a: usize) -> f64 {
        self.widths[a]
    }

    /// The center of the pixel of area `a` that is farthest from its edge
    /// (of those, the one nearest to their center), in pixels: a point well
    /// inside the area.
    pub fn inside(&self, a: usize) -> Point {
        let w = self.width as usize;
        let k = self.deepest[a];
        Point::new((k % w) as f64 + 0.5, (k / w) as f64 + 0.5)
    }

    /// The pairs of areas `(a, b)`, `a < b`, that touch at a pixel side,
    /// each pair one time, in order.
    pub fn touching(&self) -> Vec<(usize, usize)> {
        let w = self.width as usize;
        let mut pairs = std::collections::BTreeSet::new();
        for (k, &a) in self.of.iter().enumerate() {
            if a == NO_AREA {
                continue;
            }
            let right = (k % w + 1 < w).then(|| self.of[k + 1]);
            let below = self.of.get(k + w).copied();
            for b in [right, below].into_iter().flatten() {
                if b != NO_AREA && b != a {
                    pairs.insert((a.min(b) as usize, a.max(b) as usize));
                }
            }
        }
        pairs.into_iter().collect()
    }
}

/// All areas of `r` with `min_area_px` pixels or more, the most common
/// color first (it is usually behind the others).
pub fn trace(r: &Reduced, s: &TraceSettings) -> Vec<Area> {
    trace_map(r, s).1
}

/// As [`trace`], with the area map (P12 Milestone 2: which areas touch).
pub fn trace_map(r: &Reduced, s: &TraceSettings) -> (AreaMap, Vec<Area>) {
    let map = AreaMap::new(r);
    let which: Vec<usize> = (0..map.len()).filter(|&a| map.sizes[a] >= s.min_area_px).collect();
    let mut out = Tracer::new(r, &map, s).areas(&which);
    out.sort_by_key(|a| a.color);
    (map, out)
}

/// The area that holds the pixel at `(x, y)`, traced as [`trace`] traces
/// it. `None` if the pixel is transparent or outside the picture.
pub fn trace_at(r: &Reduced, x: u32, y: u32, s: &TraceSettings) -> Option<Area> {
    if y >= r.height {
        return None;
    }
    let map = AreaMap::new(r);
    let a = map.at(x, y)?;
    Tracer::new(r, &map, s).areas(&[a]).pop()
}

/// The most common color on the edge of the picture: usually the
/// background, which is not stitched.
pub fn background(r: &Reduced) -> Option<u16> {
    let (w, h) = (i64::from(r.width), i64::from(r.height));
    let mut count: HashMap<u16, usize> = HashMap::new();
    let edge = (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]));
    for (x, y) in edge {
        let l = label(r, x, y);
        if l != NONE {
            *count.entry(l).or_default() += 1;
        }
    }
    count
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(l, _)| l)
}

/// A lattice point: a pixel corner.
type V = (i64, i64);

fn pt(v: V) -> Point {
    Point::new(v.0 as f64, v.1 as f64)
}

/// One side of a pixel of an area, in the direction that has the area on
/// its right (clockwise round the pixel, as Y goes down).
#[derive(Debug, Clone, Copy)]
struct Side {
    from: V,
    to: V,
    pixel: (i64, i64),
}

/// A smoothed chain, in its own direction (see [`Tracer::canonical`]).
#[derive(Debug, Clone)]
struct Chain {
    start: Point,
    cmds: Vec<PathCmd>,
}

impl Chain {
    /// The commands in the other direction (from the end to the start).
    fn reversed(&self) -> Chain {
        let mut ends = vec![self.start];
        ends.extend(self.cmds.iter().filter_map(cmd_end));
        let mut cmds = Vec::with_capacity(self.cmds.len());
        for (k, c) in self.cmds.iter().enumerate().rev() {
            let back = ends[k];
            match *c {
                PathCmd::CubicTo(c1, c2, _) => cmds.push(PathCmd::CubicTo(c2, c1, back)),
                _ => cmds.push(PathCmd::LineTo(back)),
            }
        }
        Chain {
            start: *ends.last().unwrap_or(&self.start),
            cmds,
        }
    }
}

fn cmd_end(c: &PathCmd) -> Option<Point> {
    match *c {
        PathCmd::LineTo(p) | PathCmd::CubicTo(_, _, p) | PathCmd::MoveTo(p) => Some(p),
        PathCmd::Close => None,
    }
}

struct Tracer<'a> {
    r: &'a Reduced,
    map: &'a AreaMap,
    s: &'a TraceSettings,
    /// The smoothed chains by the key of their own direction.
    chains: HashMap<(u64, u64), Chain>,
}

impl<'a> Tracer<'a> {
    fn new(r: &'a Reduced, map: &'a AreaMap, s: &'a TraceSettings) -> Self {
        Tracer {
            r,
            map,
            s,
            chains: HashMap::new(),
        }
    }

    fn key(&self, v: V) -> u64 {
        (v.1 * (i64::from(self.r.width) + 1) + v.0) as u64
    }

    /// The key of the edge between two lattice points one step apart.
    fn edge(&self, a: V, b: V) -> u64 {
        2 * self.key(a).min(self.key(b)) + u64::from(a.0 == b.0)
    }

    /// The number of edges at a lattice point.
    fn degree(&self, (x, y): V) -> usize {
        let l = |x, y| label(self.r, x, y);
        [
            l(x, y - 1) != l(x, y),
            l(x - 1, y - 1) != l(x - 1, y),
            l(x - 1, y) != l(x, y),
            l(x - 1, y - 1) != l(x, y - 1),
        ]
        .iter()
        .filter(|&&e| e)
        .count()
    }

    /// The traced areas `which`, in that order.
    fn areas(&mut self, which: &[usize]) -> Vec<Area> {
        let mut slot = vec![usize::MAX; self.map.len()];
        for (i, &a) in which.iter().enumerate() {
            slot[a] = i;
        }
        let mut sides: Vec<Vec<Side>> = vec![Vec::new(); which.len()];
        let w = i64::from(self.r.width);
        for (k, &a) in self.map.of.iter().enumerate() {
            if a == NO_AREA || slot[a as usize] == usize::MAX {
                continue;
            }
            let (x, y) = (k as i64 % w, k as i64 / w);
            let l = self.r.labels[k];
            let list = &mut sides[slot[a as usize]];
            let mut add = |from: V, to: V| {
                list.push(Side {
                    from,
                    to,
                    pixel: (x, y),
                })
            };
            if label(self.r, x, y - 1) != l {
                add((x, y), (x + 1, y));
            }
            if label(self.r, x + 1, y) != l {
                add((x + 1, y), (x + 1, y + 1));
            }
            if label(self.r, x, y + 1) != l {
                add((x + 1, y + 1), (x, y + 1));
            }
            if label(self.r, x - 1, y) != l {
                add((x, y + 1), (x, y));
            }
        }
        which
            .iter()
            .zip(sides)
            .filter_map(|(&a, sides)| self.area(a, &sides))
            .collect()
    }

    fn area(&mut self, a: usize, sides: &[Side]) -> Option<Area> {
        let mut outer: Option<(f64, Path)> = None;
        let mut holes = Vec::new();
        let mut hole_px = Vec::new();
        for ring in self.rings(sides) {
            let n = ring.len();
            let twice: i64 = (0..n)
                .map(|i| {
                    let (p, q) = (ring[i], ring[(i + 1) % n]);
                    p.0 * q.1 - q.0 * p.1
                })
                .sum();
            let area = twice as f64 / 2.0;
            if area > 0.0 {
                if outer.as_ref().is_none_or(|o| area > o.0) {
                    outer = Some((area, self.ring_path(&ring)));
                }
            } else if -area >= self.s.min_area_px as f64 {
                holes.push(self.ring_path(&ring));
                hole_px.push(-area);
            }
        }
        Some(Area {
            index: a,
            inside: self.map.inside(a),
            color: self.map.colors[a],
            pixels: self.map.sizes[a],
            width_px: self.map.widths[a],
            outer: outer?.1,
            holes,
            hole_px,
        })
    }

    /// The closed rings of lattice points round the sides of one area.
    fn rings(&self, sides: &[Side]) -> Vec<Vec<V>> {
        let mut out_at: HashMap<V, Vec<usize>> = HashMap::new();
        for (i, s) in sides.iter().enumerate() {
            out_at.entry(s.from).or_default().push(i);
        }
        let mut used = vec![false; sides.len()];
        let mut rings = Vec::new();
        for start in 0..sides.len() {
            if used[start] {
                continue;
            }
            let mut ring = Vec::new();
            let mut cur = start;
            loop {
                used[cur] = true;
                ring.push(sides[cur].from);
                let v = sides[cur].to;
                let Some(next) = out_at.get(&v).and_then(|c| self.next_side(sides, c, cur, v)) else {
                    break;
                };
                if next == start || used[next] {
                    break;
                }
                cur = next;
            }
            rings.push(ring);
        }
        rings
    }

    /// The side that continues the ring after side `cur` at `v`. Two sides
    /// of the area start at `v` only where two of its pixels touch at the
    /// corner: the ring goes on to the other pixel if they are joined there
    /// (see [`joins`]), else it stays round its own pixel.
    fn next_side(&self, sides: &[Side], cands: &[usize], cur: usize, v: V) -> Option<usize> {
        if cands.len() < 2 {
            return cands.first().copied();
        }
        let own = sides[cur].pixel;
        let l = label(self.r, own.0, own.1);
        let m = [(-1, -1), (0, -1), (-1, 0), (0, 0)]
            .iter()
            .map(|(dx, dy)| label(self.r, v.0 + dx, v.1 + dy))
            .find(|&c| c != l)
            .unwrap_or(NONE);
        let joined = joins(l, m);
        cands.iter().copied().find(|&c| (sides[c].pixel != own) == joined)
    }

    /// The smoothed path of a ring of lattice points.
    fn ring_path(&mut self, ring: &[V]) -> Path {
        let n = ring.len();
        let nodes: Vec<usize> = (0..n).filter(|&i| self.degree(ring[i]) >= 3).collect();
        let mut path = Path::new();
        if nodes.is_empty() {
            let chain = self.chain(ring, true);
            path = path.move_to(chain.start);
            path.cmds.extend(chain.cmds);
            return path.close();
        }
        for (k, &a) in nodes.iter().enumerate() {
            let b = nodes.get(k + 1).copied().unwrap_or(nodes[0] + n);
            let piece: Vec<V> = (a..=b).map(|i| ring[i % n]).collect();
            let chain = self.chain(&piece, false);
            if k == 0 {
                path = path.move_to(chain.start);
            }
            path.cmds.extend(chain.cmds);
        }
        path.close()
    }

    /// The smoothed chain through `pts`, in this direction. Each chain is
    /// smoothed once, in its own direction: from the end with the lower key
    /// (a closed chain: from its lowest point, toward its lower neighbor).
    fn chain(&mut self, pts: &[V], closed: bool) -> Chain {
        let n = pts.len();
        let (canon, key): (Vec<V>, (u64, u64)) = if closed {
            let k0 = (0..n).min_by_key(|&i| self.key(pts[i])).unwrap_or(0);
            let mut rot: Vec<V> = pts[k0..].iter().chain(&pts[..k0]).copied().collect();
            if n > 2 && self.key(rot[n - 1]) < self.key(rot[1]) {
                rot[1..].reverse();
            }
            let key = (self.key(rot[0]), u64::MAX);
            (rot, key)
        } else {
            let fwd = (self.key(pts[0]), self.edge(pts[0], pts[1]));
            let back = (self.key(pts[n - 1]), self.edge(pts[n - 1], pts[n - 2]));
            if back < fwd {
                (pts.iter().rev().copied().collect(), back)
            } else {
                (pts.to_vec(), fwd)
            }
        };
        let s = *self.s;
        let chain = self
            .chains
            .entry(key)
            .or_insert_with(|| smooth(&canon, closed, &s))
            .clone();
        // Same direction as the stored chain?
        let same = if closed {
            n <= 2
                || pts
                    .iter()
                    .zip(pts.iter().cycle().skip(1))
                    .any(|(&a, &b)| a == canon[0] && b == canon[1])
        } else {
            pts[0] == canon[0] && pts[1] == canon[1]
        };
        if same { chain } else { chain.reversed() }
    }
}

/// The turn at `b` from the direction `a`→`b` to `b`→`c`, in degrees.
fn turn_deg(a: Point, b: Point, c: Point) -> f64 {
    let (u, v) = (b - a, c - b);
    math::to_degrees(math::atan2(u.cross(v), u.dot(v))).abs()
}

/// The corners of a chain of lattice points: the points where the turn
/// over [`CORNER_REACH`] sides on each side is larger than `deg` and
/// largest near them. The ends of an open chain are not counted.
fn corners(p: &[Point], closed: bool, deg: f64) -> Vec<usize> {
    let n = p.len();
    if n < 3 {
        return Vec::new();
    }
    let reach = if closed {
        CORNER_REACH.min((n - 1) / 2)
    } else {
        CORNER_REACH
    };
    let turn: Vec<f64> = (0..n)
        .map(|i| {
            if closed {
                turn_deg(p[(i + n - reach) % n], p[i], p[(i + reach) % n])
            } else if i == 0 || i == n - 1 {
                0.0
            } else {
                turn_deg(p[i.saturating_sub(reach)], p[i], p[(i + reach).min(n - 1)])
            }
        })
        .collect();
    (0..n)
        .filter(|&i| {
            turn[i] > deg
                && (1..=reach).all(|d| {
                    let near = if closed {
                        [Some((i + d) % n), Some((i + n - d) % n)]
                    } else {
                        [(i + d < n).then_some(i + d), i.checked_sub(d)]
                    };
                    near.iter().flatten().all(|&j| turn[j] <= turn[i])
                })
        })
        .collect()
}

/// Moving average of the points over [`SMOOTH_REACH`] points on each side,
/// [`SMOOTH_PASSES`] times. In an open sequence the ends stay and the
/// window gets smaller near them; a closed sequence (with no repeated end)
/// goes round.
fn average(seq: &mut [Point], closed: bool) {
    let n = seq.len();
    for _ in 0..SMOOTH_PASSES {
        let old = seq.to_vec();
        for (i, q) in seq.iter_mut().enumerate() {
            let k = if closed {
                SMOOTH_REACH.min((n - 1) / 2)
            } else {
                SMOOTH_REACH.min(i).min(n - 1 - i)
            };
            if k > 0 {
                let sum = (0..=2 * k).fold(Point::ZERO, |acc, j| acc + old[(i + n + j - k) % n]);
                *q = sum * (1.0 / (2 * k + 1) as f64);
            }
        }
    }
}

/// Smooths a chain of lattice points (step 3 of the module notes).
fn smooth(pts: &[V], closed: bool, s: &TraceSettings) -> Chain {
    let p: Vec<Point> = pts.iter().map(|&v| pt(v)).collect();
    let n = p.len();
    let tol = s.tolerance_px.max(0.05);
    let mid = |k: usize| p[k % n].lerp(p[(k + 1) % n], 0.5);
    let mut fixed = corners(&p, closed, s.corner_deg);
    let mut path = Path::new();
    if closed && fixed.is_empty() {
        // A smooth loop through the middle points of all sides.
        let mut seq: Vec<Point> = (0..n).map(mid).collect();
        average(&mut seq, true);
        seq.push(seq[0]);
        fit_loop(&seq, tol, &mut path);
        return Chain {
            start: seq[0],
            cmds: path.cmds,
        };
    }
    if closed {
        fixed.push(fixed[0] + n);
    } else {
        if fixed.first() != Some(&0) {
            fixed.insert(0, 0);
        }
        if fixed.last() != Some(&(n - 1)) {
            fixed.push(n - 1);
        }
    }
    for w in fixed.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut seq = vec![p[a % n]];
        seq.extend((a..b).map(mid));
        seq.push(p[b % n]);
        average(&mut seq, false);
        let (s0, s1) = (seq[0], seq[seq.len() - 1]);
        if seq.iter().all(|q| q.distance_to_segment(s0, s1) <= tol) {
            path.cmds.push(PathCmd::LineTo(s1));
        } else {
            fit_points(&seq, tol, &mut path);
        }
    }
    Chain {
        start: p[fixed[0] % n],
        cmds: path.cmds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_model::Rgb;

    /// A picture from a function of the pixel position (0 is the first
    /// color); [`NONE`] where it returns `None`.
    fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> Option<u16>) -> Reduced {
        Reduced {
            width: w,
            height: h,
            palette: vec![Rgb(255, 255, 255), Rgb(200, 0, 0), Rgb(0, 0, 200)],
            labels: (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| f(x, y).unwrap_or(NONE))
                .collect(),
        }
    }

    /// The area inside a path (holes count less), in pixels.
    fn area_of(p: &Path) -> f64 {
        p.flatten(0.01)
            .iter()
            .map(|pl| {
                let n = pl.points.len();
                (0..n).map(|i| pl.points[i].cross(pl.points[(i + 1) % n])).sum::<f64>() / 2.0
            })
            .sum()
    }

    fn area(a: &Area) -> f64 {
        area_of(&a.outer) + a.holes.iter().map(area_of).sum::<f64>()
    }

    #[test]
    fn the_mean_width_of_a_band_and_of_a_ragged_blob() {
        // A red band 10 px high and 80 px long, and a blue square of 60 px
        // with a ragged edge (every second pixel of its edge rows is white).
        let r = picture(200, 80, |x, y| {
            if (5..85).contains(&x) && (10..20).contains(&y) {
                Some(1)
            } else if (100..160).contains(&x) && (10..70).contains(&y) {
                let edge = !(103..157).contains(&x) || !(13..67).contains(&y);
                Some(if edge && (x + y) % 2 == 0 { 0 } else { 2 })
            } else {
                Some(0)
            }
        });
        let map = AreaMap::new(&r);
        let band = map.at(40, 15).unwrap();
        assert!((map.width(band) - 10.0).abs() < 0.6, "{}", map.width(band));
        let blob = map.at(130, 40).unwrap();
        let w = map.width(blob);
        // Twice the area over the edge length is small here: the ragged
        // edge is long. The distance map sees a wide area.
        assert!(w > 25.0, "{w}");
        let areas = trace(&r, &TraceSettings::default());
        let traced = areas.iter().find(|a| a.color == 2).unwrap();
        assert_eq!(traced.width_px, w);
    }

    #[test]
    fn a_square_keeps_its_corners_and_its_hole() {
        // A red 20 x 20 square with a 6 x 6 hole, on white.
        let r = picture(40, 30, |x, y| {
            let inside = (10..30).contains(&x) && (5..25).contains(&y);
            let hole = (17..23).contains(&x) && (12..18).contains(&y);
            Some(u16::from(inside && !hole))
        });
        let s = TraceSettings::default();
        let red = trace_at(&r, 12, 7, &s).expect("the red area");
        assert_eq!(red.color, 1);
        assert_eq!(red.pixels, 400 - 36);
        assert_eq!(red.holes.len(), 1);
        // Straight sides and sharp corners: the outline has the corners of the square.
        let pts: Vec<Point> = red.outer.points().collect();
        for c in [(10.0, 5.0), (30.0, 5.0), (30.0, 25.0), (10.0, 25.0)] {
            assert!(pts.contains(&Point::new(c.0, c.1)), "corner {c:?} in {pts:?}");
        }
        assert!(red.outer.cmds.iter().all(|c| !matches!(c, PathCmd::CubicTo(..))));
        assert!((area(&red) - 364.0).abs() < 1e-6, "{}", area(&red));
    }

    #[test]
    fn a_disk_becomes_a_few_smooth_curves() {
        let (cx, cy, rad) = (25.0, 25.0, 15.0);
        let r = picture(50, 50, |x, y| {
            let d = ((f64::from(x) + 0.5 - cx).powi(2) + (f64::from(y) + 0.5 - cy).powi(2)).sqrt();
            Some(u16::from(d < rad))
        });
        let a = trace_at(&r, 25, 25, &TraceSettings::default()).unwrap();
        let curves = a
            .outer
            .cmds
            .iter()
            .filter(|c| matches!(c, PathCmd::CubicTo(..)))
            .count();
        assert!((2..=12).contains(&curves), "{curves} curves");
        // The traced area is near the true area of the disk (the averaging
        // moves a curve this small about 0.3 px to its inside).
        let true_area = std::f64::consts::PI * rad * rad;
        assert!(
            (area(&a) - true_area).abs() < 0.05 * true_area,
            "{} {}",
            area(&a),
            true_area
        );
        // All points are near the circle.
        for p in a.outer.flatten(0.01).iter().flat_map(|pl| pl.points.clone()) {
            let d = p.distance(Point::new(cx, cy));
            assert!((d - rad).abs() < 1.0, "{p:?} at {d}");
        }
    }

    #[test]
    fn touching_areas_share_their_edges() {
        // Three colors that meet: white left, a red circle over a blue bar.
        let r = picture(60, 40, |x, y| {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            if ((fx - 30.0).powi(2) + (fy - 20.0).powi(2)).sqrt() < 12.0 {
                Some(1)
            } else if (15..50).contains(&x) && (14..26).contains(&y) {
                Some(2)
            } else {
                Some(0)
            }
        });
        let areas = trace(&r, &TraceSettings::default());
        // The areas cover the picture once: no gap and no overlap.
        let total: f64 = areas.iter().map(area).sum();
        assert!((total - 2400.0).abs() < 1e-6, "{total}");
        let pixels: usize = areas.iter().map(|a| a.pixels).sum();
        assert_eq!(pixels, 2400);
        // The most common color first.
        assert_eq!(areas[0].color, 0);
        // The same area alone is traced the same.
        let red = areas.iter().find(|a| a.color == 1).unwrap();
        assert_eq!(trace_at(&r, 30, 20, &TraceSettings::default()).as_ref(), Some(red));
        assert_eq!(background(&r), Some(0));
    }

    #[test]
    fn a_diagonal_line_of_the_less_common_color_stays_whole() {
        // A blue line one pixel wide from corner to corner, on white.
        let r = picture(20, 20, |x, y| Some(if x == y { 2 } else { 0 }));
        let map = AreaMap::new(&r);
        let blue = map.at(0, 0).unwrap();
        assert_eq!(map.pixels(blue), 20);
        // The white is cut in two by the line.
        assert_eq!(map.len(), 3);
        let a = trace_at(&r, 5, 5, &TraceSettings::default()).unwrap();
        assert_eq!(a.pixels, 20);
        assert!(a.holes.is_empty());
        // Transparent pixels are not traced; small areas are left out.
        let r = picture(10, 10, |x, y| (x == 4 && y == 4).then_some(1));
        assert!(trace(&r, &TraceSettings::default()).is_empty());
        assert!(trace_at(&r, 0, 0, &TraceSettings::default()).is_none());
    }

    #[test]
    fn a_reversed_chain_is_the_same_curve() {
        let c = Chain {
            start: Point::new(0.0, 0.0),
            cmds: vec![
                PathCmd::LineTo(Point::new(5.0, 0.0)),
                PathCmd::CubicTo(Point::new(6.0, 0.0), Point::new(7.0, 1.0), Point::new(7.0, 2.0)),
            ],
        };
        let r = c.reversed();
        assert_eq!(r.start, Point::new(7.0, 2.0));
        assert_eq!(
            r.cmds,
            vec![
                PathCmd::CubicTo(Point::new(7.0, 1.0), Point::new(6.0, 0.0), Point::new(5.0, 0.0)),
                PathCmd::LineTo(Point::new(0.0, 0.0)),
            ]
        );
        assert_eq!(r.reversed().cmds, c.cmds);
    }
}
