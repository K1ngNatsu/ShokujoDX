//! The surface of the embroidery (P18 view plan, Milestone 3, ADR 0124).
//!
//! Two steps make it:
//!
//! 1. [`lay`] lays the strands ([`crate::strand`]) in sew order on a grid
//!    of 0.1 mm. A stitch comes out of the fabric at its first needle
//!    point, goes up to its height in a short curve, and goes down into
//!    the fabric at its second needle point; a short stitch is an arch.
//!    Its section is a flat oval ([`THICKNESS`] of its width). It lies on
//!    what is under it as a thread under tension: a straight line from
//!    each needle point (on the fabric) over the highest things that it
//!    crosses (the upper hull of what is under it). So a column over
//!    underlay is round, a border on a fill is higher than the fill, and a
//!    puff column stands on its foam (the foam is a floor under the stitch
//!    from [`FOAM_EDGE_MM`] after its needle points). Thread presses
//!    thread: on many layers a thread adds less height ([`PRESS`]). A
//!    stitch does not lie on the stitch just before it: the two share a
//!    needle hole and lie side by side (the stitches of a satin and of a
//!    row of a fill).
//!    The result ([`Laid`]) keeps the height of the middle line of each
//!    stitch.
//! 2. [`Laid::surface`] makes the surface of an area at a zoom, in tiles
//!    of [`TILE`] × [`TILE`] pixels, on all processors: for each pixel its
//!    height, the color and the direction of the thread on top, the
//!    material and the part of the pixel that the threads cover. A pixel
//!    depends only on the laid stitches, so the same input gives the same
//!    bytes on each system and with each number of processors.
//!
//! A surface for a small zoom is made at [`DETAIL_PX_PER_MM`] or more and
//! halved ([`Surface::half`], the zoom levels of Milestone 4, ADR 0125).
//! [`crate::sheen::light`] makes its picture.
//!
//! The canvas (Milestone 5, ADR 0126) gets its tiles from [`Laid::block`]:
//! the pixels of a block on one grid of pixels for each zoom level, made
//! with an apron round them, so a pixel is the same in each block. After a
//! change of the design [`relay`] lays again only the stitches that the
//! change reaches, and tells where the surface changed ([`Changes`]).

use std::collections::HashSet;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use emb_geom::{BBox, Point};
use emb_model::{Rgb, ThreadKind};

use crate::raster::Areas;
use crate::strand::Strand;
use crate::thread3d::Lighting;

/// The cell of the grid on which the stitches are laid, mm.
pub const CELL_MM: f64 = 0.1;
/// The most cells of that grid: a larger design gets larger cells.
const MAX_CELLS: f64 = 16.0e6;
/// The thickness of a thread as a part of its width (a flat oval).
pub const THICKNESS: f64 = 0.55;
/// How far a thread goes from its needle point to its full height, mm.
pub const RISE_MM: f64 = 0.35;
/// The foam of puff is under a stitch from this far after its needle
/// points (the needle cuts the foam at the edge of the column), mm.
pub const FOAM_EDGE_MM: f64 = 0.3;
/// Thread presses thread: on `n` layers a thread adds its thickness
/// divided by `1 + PRESS × n`.
pub const PRESS: f64 = 0.15;
/// The thickness of a piece of appliqué fabric (and of a stone), mm.
pub const PIECE_MM: f64 = 0.3;
/// The side of a tile, pixels.
pub const TILE: u32 = 256;
/// A surface is made with at least this many pixels for each mm (a 40
/// weight thread is 6.4 pixels wide), and halved for a smaller zoom.
pub const DETAIL_PX_PER_MM: f64 = 16.0;
/// The most pixels of the surface that is halved (16 bytes each): a very
/// large design at a small zoom gets less detail.
const MAX_DETAIL_PIXELS: u64 = 8_000_000;
/// The apron round a block, mm: the light of a pixel depends on the heights
/// this far from it (its shadow, its gaps and its slope).
pub const APRON_MM: f64 = SHADOW_MAX_MM + GAP_RADIUS_MM + 0.2;
/// The longest side of a block that is made with its detail in one piece,
/// pixels of the detail (a larger block is made in parts).
const MAX_BLOCK_SIDE: u32 = 768;
/// A block is made with at most this many halvings (a block of a very
/// small zoom gets less detail).
const MAX_BLOCK_HALVINGS: u32 = 4;
/// The side of the squares of the index of the strands, mm.
const INDEX_MM: f64 = 2.0;
/// The side of the squares of the index with which [`relay`] finds the
/// heights under a stitch, mm.
const QUERY_MM: f64 = 1.0;
/// The side of the squares of [`Changes`], mm.
pub const CHANGE_MM: f64 = 2.0;
/// [`Changes`] with more squares than this are a change everywhere.
const MAX_CHANGE_SQUARES: usize = 1_000_000;
/// The small changes of each thread: of its width and of its brightness
/// (each from −1 to 1 times this, from the place of the stitch).
const VARY_WIDTH: f64 = 0.06;
const VARY_BRIGHTNESS: f64 = 0.04;
/// The length of one turn of the plies along a thread, mm.
const PLY_PITCH_MM: f64 = 0.35;
/// The gaps: a pixel lower than the mean of the heights round it (in this
/// radius, mm) is darker, up to `GAP_DARK` at `GAP_DEPTH_MM` lower.
const GAP_RADIUS_MM: f64 = 0.3;
const GAP_DARK: f64 = 0.35;
const GAP_DEPTH_MM: f64 = 0.15;

/// The materials of a pixel.
pub mod material {
    /// No thread and no piece.
    pub const FABRIC: u8 = 0;
    /// Appliqué fabric or a stone.
    pub const PIECE: u8 = 1;
    /// Thread: `THREAD` plus the index of its kind in `ThreadKind::ALL`.
    pub const THREAD: u8 = 2;
}

/// The material of a thread of `kind`.
pub fn thread_material(kind: ThreadKind) -> u8 {
    material::THREAD + ThreadKind::ALL.iter().position(|k| *k == kind).unwrap_or(0) as u8
}

/// The kind of thread of a material, if it is thread.
pub fn kind_of(material: u8) -> Option<ThreadKind> {
    material
        .checked_sub(material::THREAD)
        .and_then(|i| ThreadKind::ALL.get(i as usize).copied())
}

/// The heights of a thread under tension over `under` (heights at the
/// places `t`, in order, from one needle point to the other): the upper
/// hull of the points, through the first and the last point.
fn taut(t: &[f64], under: &[f64], out: &mut Vec<f64>) {
    out.clear();
    let mut hull: Vec<usize> = Vec::with_capacity(t.len());
    for k in 0..t.len() {
        while hull.len() >= 2 {
            let (o, a) = (hull[hull.len() - 2], hull[hull.len() - 1]);
            let cross = (t[a] - t[o]) * (under[k] - under[o]) - (under[a] - under[o]) * (t[k] - t[o]);
            if cross >= 0.0 {
                hull.pop();
            } else {
                break;
            }
        }
        hull.push(k);
    }
    let mut j = 0;
    for k in 0..t.len() {
        while j + 2 < hull.len() && t[hull[j + 1]] < t[k] {
            j += 1;
        }
        let (a, b) = (hull[j], hull[(j + 1).min(hull.len() - 1)]);
        let v = if b == a || t[b] <= t[a] {
            under[a]
        } else {
            under[a] + (under[b] - under[a]) * (t[k] - t[a]) / (t[b] - t[a])
        };
        out.push(v);
    }
}

/// The height profile of a thread along it at `d` mm from its nearer
/// needle point (a quarter circle: steep at the needle point, flat after
/// `rise`).
fn along(d: f64, rise: f64) -> f64 {
    if d >= rise {
        1.0
    } else {
        let x = 1.0 - d.max(0.0) / rise;
        (1.0 - x * x).max(0.0).sqrt()
    }
}

/// The grid on which the stitches are laid: the height of the top and the
/// number of layers of thread in each cell. The cell of a place `p` is
/// `floor(p / cell)`, so two grids with the same cell have the same cells
/// ([`relay`] uses this).
#[derive(Debug, Clone, PartialEq)]
struct Grid {
    /// The cell of the first column and of the first row.
    ox: i64,
    oy: i64,
    cell: f64,
    w: usize,
    h: usize,
    height: Vec<f32>,
    layers: Vec<u8>,
}

/// The center of the cell (`cx`, `cy`) of the cells of `cell` mm.
fn cell_center(cx: i64, cy: i64, cell: f64) -> Point {
    Point::new((cx as f64 + 0.5) * cell, (cy as f64 + 0.5) * cell)
}

/// The cells of `cell` mm between the coordinates `lo` and `hi`, from the
/// cell `o` of `n` cells.
fn cells_between(lo: f64, hi: f64, cell: f64, o: i64, n: usize) -> Range<usize> {
    let a = ((lo / cell).floor() as i64 - o).clamp(0, n as i64) as usize;
    let b = ((hi / cell).ceil() as i64 - o).clamp(0, n as i64) as usize;
    a.min(b)..b
}

impl Grid {
    fn new(b: BBox, cell: f64) -> Grid {
        let (ox, oy) = ((b.min.x / cell).floor() as i64, (b.min.y / cell).floor() as i64);
        let w = ((b.max.x / cell).ceil() as i64 - ox).max(1) as usize;
        let h = ((b.max.y / cell).ceil() as i64 - oy).max(1) as usize;
        Grid {
            ox,
            oy,
            cell,
            w,
            h,
            height: vec![0.0; w * h],
            layers: vec![0; w * h],
        }
    }

    fn index(&self, p: Point) -> Option<usize> {
        let x = (p.x / self.cell).floor() as i64 - self.ox;
        let y = (p.y / self.cell).floor() as i64 - self.oy;
        (x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h).then(|| y as usize * self.w + x as usize)
    }

    fn height_at(&self, p: Point) -> f64 {
        self.index(p).map_or(0.0, |i| f64::from(self.height[i]))
    }

    fn layers_at(&self, p: Point) -> u8 {
        self.index(p).map_or(0, |i| self.layers[i])
    }

    /// The columns of the cells between the x coordinates `lo` and `hi`.
    fn xs(&self, lo: f64, hi: f64) -> Range<usize> {
        cells_between(lo, hi, self.cell, self.ox, self.w)
    }

    /// The rows of the cells between the y coordinates `lo` and `hi`.
    fn ys(&self, lo: f64, hi: f64) -> Range<usize> {
        cells_between(lo, hi, self.cell, self.oy, self.h)
    }

    /// The center of the cell (`x`, `y`) of the grid.
    fn center(&self, x: usize, y: usize) -> Point {
        cell_center(self.ox + x as i64, self.oy + y as i64, self.cell)
    }

    /// A piece of fabric: the rings (even-odd) at the height `h`. Row by
    /// row, from the places where the edges cross the middle of the row:
    /// the same test as [`inside`] for the center of each cell.
    fn piece(&mut self, rings: &[Vec<Point>], h: f32) {
        let mut xs: Vec<f64> = Vec::new();
        for y in 0..self.h {
            let yc = self.center(0, y).y;
            xs.clear();
            for r in rings {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    if (a.y <= yc) != (b.y <= yc) {
                        xs.push(a.x + (yc - a.y) / (b.y - a.y) * (b.x - a.x));
                    }
                }
            }
            if xs.is_empty() {
                continue;
            }
            xs.sort_by(f64::total_cmp);
            // The crossings at the center of the cell or to its left.
            let mut left = 0;
            for x in 0..self.w {
                let xc = self.center(x, 0).x;
                while left < xs.len() && xs[left] <= xc {
                    left += 1;
                }
                if (xs.len() - left) % 2 == 1 {
                    let i = y * self.w + x;
                    self.height[i] = self.height[i].max(h);
                }
            }
        }
    }

    /// Lays a strand on the grid.
    fn stamp(&mut self, s: &LaidStrand, z: &[f32]) {
        let b = s.bbox;
        let (xs, ys) = (self.xs(b.min.x, b.max.x), self.ys(b.min.y, b.max.y));
        self.stamp_in(s, z, xs, ys);
    }

    /// Lays a strand on the cells `xs` × `ys` of the grid.
    fn stamp_in(&mut self, s: &LaidStrand, z: &[f32], xs: Range<usize>, ys: Range<usize>) {
        for y in ys {
            for x in xs.clone() {
                let q = self.center(x, y);
                let Some((h, middle)) = s.height_at(q, z) else { continue };
                let i = y * self.w + x;
                if h > self.height[i] {
                    self.height[i] = h;
                }
                if middle {
                    self.layers[i] = self.layers[i].saturating_add(1);
                }
            }
        }
    }
}

/// Two values from −1 to 1 from the place of a stitch: the same stitch
/// gets the same values on each system.
fn vary(a: Point, b: Point) -> [f32; 2] {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for v in [a.x, a.y, b.x, b.y] {
        h ^= (v * 100.0).round() as i64 as u64;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 31;
    }
    let one = |x: u64| ((x & 0xFFFF) as f32 / 65535.0) * 2.0 - 1.0;
    [one(h), one(h >> 16)]
}

/// `true` if `p` is inside the rings (even-odd).
fn inside(rings: &[Vec<Point>], p: Point) -> bool {
    let mut odd = false;
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            if (a.y <= p.y) != (b.y <= p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                odd = !odd;
            }
        }
    }
    odd
}

/// A stitch laid on the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LaidStrand {
    a: Point,
    dir: (f64, f64),
    len: f64,
    /// Half the width of the thread, mm.
    half: f64,
    /// The thickness that it adds (after the press).
    thick: f64,
    color: Rgb,
    material: u8,
    /// The direction of the thread: the vector of two times its angle,
    /// times 127.
    axis: [i8; 2],
    /// The small changes of its width and its brightness, −1 to 1.
    vary: [f32; 2],
    /// The heights of its middle line: `z[start..start + n]`, `step` mm apart.
    start: usize,
    n: usize,
    step: f64,
    /// The area that it covers, with its width.
    bbox: BBox,
    /// A number from the strand, not from its record ([`key`]): the same
    /// stitch has the same number.
    key: u64,
    /// The record of the plan at its end ([`Strand::record`]).
    record: u32,
}

impl LaidStrand {
    /// The height of the middle line at `t` mm along the stitch.
    fn middle(&self, z: &[f32], t: f64) -> f32 {
        if !(0.0..=self.len).contains(&t) {
            return 0.0;
        }
        let f = t / self.step;
        let k = (f as usize).min(self.n - 2);
        let r = (f - k as f64).clamp(0.0, 1.0) as f32;
        z[k] + (z[k + 1] - z[k]) * r
    }

    /// The height of the thread at `q` and `true` if `q` is near its middle
    /// line on the full height; `None` off the thread.
    fn height_at(&self, q: Point, z: &[f32]) -> Option<(f32, bool)> {
        let (rx, ry) = (q.x - self.a.x, q.y - self.a.y);
        let t = rx * self.dir.0 + ry * self.dir.1;
        if !(0.0..=self.len).contains(&t) {
            return None;
        }
        let s = (ry * self.dir.0 - rx * self.dir.1).abs() / self.half;
        if s >= 1.0 {
            return None;
        }
        let zc = self.middle(&z[self.start..self.start + self.n], t);
        let h = zc * ((1.0 - s * s).sqrt() as f32);
        Some((h, s < 0.7 && f64::from(zc) > 0.5 * self.thick))
    }
}

/// The stitches laid in sew order (step 1, see the module text).
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    strands: Vec<LaidStrand>,
    z: Vec<f32>,
    pieces: Vec<(Vec<Vec<Point>>, Rgb)>,
    grid: Grid,
    /// The strands in squares of [`INDEX_MM`] (for the blocks).
    index: Index,
    /// The highest point of the stitches and pieces, mm.
    top: f64,
}

/// The strands in squares: for each square the strands whose box meets
/// it, in sew order.
#[derive(Debug, Clone, PartialEq)]
struct Index {
    side: f64,
    /// The square of the first column and of the first row (the square of
    /// a place `p` is `floor(p / side)`).
    ox: i64,
    oy: i64,
    w: usize,
    h: usize,
    lists: Vec<Vec<u32>>,
}

impl Index {
    /// The index of the boxes of the strands, in their order, in the area
    /// `b`.
    fn new(boxes: impl Iterator<Item = BBox>, b: BBox, side: f64) -> Index {
        let (ox, oy) = ((b.min.x / side).floor() as i64, (b.min.y / side).floor() as i64);
        let w = ((b.max.x / side).floor() as i64 - ox + 1).max(1) as usize;
        let h = ((b.max.y / side).floor() as i64 - oy + 1).max(1) as usize;
        let mut index = Index {
            side,
            ox,
            oy,
            w,
            h,
            lists: vec![Vec::new(); w * h],
        };
        for (i, bb) in boxes.enumerate() {
            let (xs, ys) = index.squares(bb);
            for y in ys {
                for x in xs.clone() {
                    index.lists[y * w + x].push(i as u32);
                }
            }
        }
        index
    }

    /// The squares that `b` meets.
    fn squares(&self, b: BBox) -> (Range<usize>, Range<usize>) {
        let r = |lo: f64, hi: f64, o: i64, n: usize| {
            let a = ((lo / self.side).floor() as i64 - o).clamp(0, n as i64) as usize;
            let e = ((hi / self.side).floor() as i64 - o + 1).clamp(0, n as i64) as usize;
            a.min(e)..e
        };
        (
            r(b.min.x, b.max.x, self.ox, self.w),
            r(b.min.y, b.max.y, self.oy, self.h),
        )
    }

    /// The number of the square of `p`, if it is in the index.
    fn square(&self, p: Point) -> Option<usize> {
        let x = (p.x / self.side).floor() as i64 - self.ox;
        let y = (p.y / self.side).floor() as i64 - self.oy;
        (x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h).then(|| y as usize * self.w + x as usize)
    }

    /// The strands of the square of `p`.
    fn at(&self, p: Point) -> &[u32] {
        self.square(p).map_or(&[], |i| &self.lists[i])
    }

    /// Sets the squares that `b` meets.
    fn mark(&self, marks: &mut [bool], b: BBox) {
        let (xs, ys) = self.squares(b);
        for y in ys {
            marks[y * self.w + xs.start..y * self.w + xs.end].fill(true);
        }
    }

    /// `true` if a square that `b` meets is set.
    fn any(&self, marks: &[bool], b: BBox) -> bool {
        let (xs, ys) = self.squares(b);
        ys.into_iter()
            .any(|y| marks[y * self.w + xs.start..y * self.w + xs.end].contains(&true))
    }

    /// The strands whose box meets the squares of `b`, in sew order, each
    /// one time.
    fn near(&self, b: BBox) -> Vec<u32> {
        let (xs, ys) = self.squares(b);
        let mut out: Vec<u32> = Vec::new();
        for y in ys {
            for x in xs.clone() {
                out.extend_from_slice(&self.lists[y * self.w + x]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The size of the data, bytes.
    fn bytes(&self) -> usize {
        self.lists.iter().map(|l| l.len() * 4 + 24).sum()
    }
}

/// The areas where the surface changed: squares of [`CHANGE_MM`] (after a
/// [`relay`], or a change of the strands that show).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Changes {
    everywhere: bool,
    squares: HashSet<(i64, i64)>,
}

impl Changes {
    /// A change everywhere.
    pub fn everywhere() -> Changes {
        Changes {
            everywhere: true,
            squares: HashSet::new(),
        }
    }

    /// Adds the area `b`.
    pub fn add(&mut self, b: BBox) {
        if self.everywhere {
            return;
        }
        let s = CHANGE_MM;
        let (x0, x1) = ((b.min.x / s).floor() as i64, (b.max.x / s).floor() as i64);
        let (y0, y1) = ((b.min.y / s).floor() as i64, (b.max.y / s).floor() as i64);
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > MAX_CHANGE_SQUARES as i64 {
            *self = Changes::everywhere();
            return;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.squares.insert((x, y));
            }
        }
        if self.squares.len() > MAX_CHANGE_SQUARES {
            *self = Changes::everywhere();
        }
    }

    /// Adds the changes of `other`.
    pub fn merge(&mut self, other: &Changes) {
        if other.everywhere {
            *self = Changes::everywhere();
        } else if !self.everywhere {
            self.squares.extend(other.squares.iter().copied());
        }
    }

    /// `true` if nothing changed.
    pub fn is_empty(&self) -> bool {
        !self.everywhere && self.squares.is_empty()
    }

    /// `true` if all of the surface changed.
    pub fn is_everywhere(&self) -> bool {
        self.everywhere
    }

    /// The area of the changes in `b` (the squares that changed, cut to
    /// `b`), if something changed there.
    pub fn within(&self, b: BBox) -> Option<BBox> {
        if self.everywhere {
            return Some(b);
        }
        let s = CHANGE_MM;
        let (x0, x1) = ((b.min.x / s).floor() as i64, (b.max.x / s).floor() as i64);
        let (y0, y1) = ((b.min.y / s).floor() as i64, (b.max.y / s).floor() as i64);
        let square = |x: i64, y: i64| {
            BBox::new(
                Point::new(x as f64 * s, y as f64 * s),
                Point::new((x + 1) as f64 * s, (y + 1) as f64 * s),
            )
        };
        let mut out: Option<BBox> = None;
        let mut add = |x: i64, y: i64| {
            let q = square(x, y);
            out = Some(out.map_or(q, |o| o.union(q)));
        };
        let n = (x1 - x0 + 1).saturating_mul(y1 - y0 + 1);
        if n > self.squares.len() as i64 {
            for &(x, y) in &self.squares {
                if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
                    add(x, y);
                }
            }
        } else {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if self.squares.contains(&(x, y)) {
                        add(x, y);
                    }
                }
            }
        }
        out.map(|o| {
            BBox::new(
                Point::new(o.min.x.max(b.min.x), o.min.y.max(b.min.y)),
                Point::new(o.max.x.min(b.max.x), o.max.y.min(b.max.y)),
            )
        })
    }

    /// `true` if something changed in `b`.
    pub fn touches(&self, b: BBox) -> bool {
        if self.everywhere {
            return true;
        }
        if self.squares.is_empty() {
            return false;
        }
        let s = CHANGE_MM;
        let (x0, x1) = ((b.min.x / s).floor() as i64, (b.max.x / s).floor() as i64);
        let (y0, y1) = ((b.min.y / s).floor() as i64, (b.max.y / s).floor() as i64);
        let n = (x1 - x0 + 1).saturating_mul(y1 - y0 + 1);
        if n > self.squares.len() as i64 {
            self.squares
                .iter()
                .any(|&(x, y)| (x0..=x1).contains(&x) && (y0..=y1).contains(&y))
        } else {
            (y0..=y1).any(|y| (x0..=x1).any(|x| self.squares.contains(&(x, y))))
        }
    }
}

/// The length of a strand, mm.
fn length(s: &Strand) -> f64 {
    let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
    (dx * dx + dy * dy).sqrt()
}

/// `true` for a strand that is laid (a stitch of no length is not).
fn has_length(s: &Strand) -> bool {
    length(s) >= 1e-9
}

/// The area that a strand covers, with its width.
fn strand_box(s: &Strand) -> BBox {
    BBox::from_points([s.a, s.b])
        .expect("two points")
        .expand(emb_model::thread::width_mm(s.weight) / 2.0)
}

/// A number from all that a strand is, but its record: two strands with
/// the same number are the same stitch.
fn key(s: &Strand) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut add = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= h >> 29;
    };
    for v in [s.a.x, s.a.y, s.b.x, s.b.y, s.lift_mm] {
        add(v.to_bits());
    }
    add(u64::from(s.color.0)
        | (u64::from(s.color.1) << 8)
        | (u64::from(s.color.2) << 16)
        | (u64::from(s.weight) << 24)
        | (u64::from(thread_material(s.kind)) << 40));
    h
}

/// The area of the grid for `strands` and `areas`.
fn extent(strands: &[Strand], areas: &Areas) -> BBox {
    let points = strands
        .iter()
        .flat_map(|s| [s.a, s.b])
        .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied()));
    BBox::from_points(points)
        .unwrap_or(BBox::new(Point::ZERO, Point::ZERO))
        .expand(1.0)
}

/// The cell of the grid for the area `b`: [`CELL_MM`], or two times larger
/// as many times as a large area needs (at most [`MAX_CELLS`] cells). The
/// cell changes only by a factor of two, so a small change of a design
/// keeps it.
fn cell_for(b: BBox) -> f64 {
    let need = (b.width() * b.height() / MAX_CELLS).sqrt();
    let mut cell = CELL_MM;
    while cell < need {
        cell *= 2.0;
    }
    cell
}

/// The work space of [`lay_one`].
#[derive(Default)]
struct Scratch {
    under: Vec<f64>,
    ts: Vec<f64>,
    line: Vec<f64>,
    /// The heights of the middle line of the strand that was laid last.
    z: Vec<f32>,
}

/// Lays the strand `s` on what `under_at` gives (the height and the layers
/// of thread of the grid at a place), with cells of `cell` mm. The heights
/// of its middle line go to `sc.z`; `start` is their place in the heights
/// of all strands. `None` for a stitch of no length.
fn lay_one(
    s: &Strand,
    cell: f64,
    start: usize,
    under_at: impl Fn(Point) -> (f64, u8),
    sc: &mut Scratch,
) -> Option<LaidStrand> {
    let len = length(s);
    if len < 1e-9 {
        return None;
    }
    let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
    let dir = (dx / len, dy / len);
    let width = emb_model::thread::width_mm(s.weight);
    let half = width / 2.0;
    let n = ((len / cell).ceil() as usize).max(1) + 1;
    let step = len / (n - 1) as f64;
    // What is under the middle line of the stitch; the needle points are on
    // the fabric, and the foam is a floor. A thread beside it (less than
    // half a width into it, as the parts of a split satin or the rows of a
    // dense fill) is not under it: the two lie side by side.
    sc.under.clear();
    sc.ts.clear();
    let mut layers = 0u8;
    for k in 0..n {
        let t = step * k as f64;
        let p = Point::new(s.a.x + dir.0 * t, s.a.y + dir.1 * t);
        let (mut h, l) = under_at(p);
        if t.min(len - t) >= FOAM_EDGE_MM.min(len / 2.0) {
            h = h.max(s.lift_mm);
        }
        sc.ts.push(t);
        sc.under.push(if k == 0 || k == n - 1 { 0.0 } else { h });
        layers = layers.max(l);
    }
    // Thread under tension: a taut line over what is under it.
    taut(&sc.ts, &sc.under, &mut sc.line);
    let thick = THICKNESS * width / (1.0 + PRESS * f64::from(layers));
    sc.z.clear();
    for (k, u) in sc.line.iter().enumerate() {
        let t = sc.ts[k];
        sc.z.push((u + thick * along(t.min(len - t), RISE_MM)) as f32);
    }
    let axis = {
        let a2 = 2.0 * emb_geom::math::atan2(dir.1, dir.0);
        [emb_geom::math::cos(a2), emb_geom::math::sin(a2)].map(|v| (v * 127.0).round() as i8)
    };
    Some(LaidStrand {
        a: s.a,
        dir,
        len,
        half,
        thick,
        color: s.color,
        material: thread_material(s.kind),
        axis,
        vary: vary(s.a, s.b),
        start,
        n,
        step,
        bbox: BBox::from_points([s.a, s.b]).expect("two points").expand(half),
        key: key(s),
        record: s.record,
    })
}

/// Lays `strands` in sew order on the pieces of fabric `areas` (appliqué
/// fabric and stones, under all stitches).
pub fn lay(strands: &[Strand], areas: &Areas) -> Laid {
    let b = extent(strands, areas);
    let cell = cell_for(b);
    let mut grid = Grid::new(b, cell);
    for (rings, _) in areas {
        grid.piece(rings, PIECE_MM as f32);
    }
    let mut laid: Vec<LaidStrand> = Vec::with_capacity(strands.len());
    let mut z: Vec<f32> = Vec::new();
    let mut sc = Scratch::default();
    // The stitch just before is laid on the grid after the next one.
    let mut waiting: Option<usize> = None;
    for s in strands {
        let g = &grid;
        let Some(ls) = lay_one(s, cell, z.len(), |p| (g.height_at(p), g.layers_at(p)), &mut sc) else {
            continue;
        };
        z.extend_from_slice(&sc.z);
        laid.push(ls);
        if let Some(w) = waiting.replace(laid.len() - 1) {
            let ls = laid[w];
            grid.stamp(&ls, &z);
        }
    }
    if let Some(w) = waiting {
        let ls = laid[w];
        grid.stamp(&ls, &z);
    }
    Laid::new(laid, z, areas.to_vec(), grid, b)
}

/// The stitches `strands` laid again after a change of the design (ADR
/// 0126): the same result as [`lay`], but the stitches that the change
/// does not reach keep their heights from `old`. The stitches before the
/// first changed stitch are the same; a stitch after the change is laid
/// again only if a changed stitch is under it, and changes only if its
/// heights change. Also gives the areas where the laid stitches changed.
pub fn relay(old: &Laid, strands: &[Strand], areas: &Areas) -> (Laid, Changes) {
    let b = extent(strands, areas);
    let cell = cell_for(b);
    if cell != old.grid.cell || areas != old.pieces.as_slice() {
        return (lay(strands, areas), Changes::everywhere());
    }
    let list: Vec<&Strand> = strands.iter().filter(|s| has_length(s)).collect();
    let keys: Vec<u64> = list.iter().map(|s| key(s)).collect();
    let (n_old, n_new) = (old.strands.len(), list.len());
    // The stitches before and after the change.
    let mut p = 0;
    while p < n_old.min(n_new) && old.strands[p].key == keys[p] {
        p += 1;
    }
    let mut q = 0;
    while q < (n_old - p).min(n_new - p) && old.strands[n_old - 1 - q].key == keys[n_new - 1 - q] {
        q += 1;
    }
    let boxes: Vec<BBox> = list.iter().map(|s| strand_box(s)).collect();
    let index = Index::new(boxes.iter().copied(), b, QUERY_MM);
    // The squares where a stitch changed.
    let mut changed_at = vec![false; index.lists.len()];
    let mut changes = Changes::default();
    for s in &old.strands[p..n_old - q] {
        index.mark(&mut changed_at, s.bbox);
        changes.add(s.bbox);
    }
    // The pieces alone.
    let mut base = Grid::new(b, cell);
    for (rings, _) in areas {
        base.piece(rings, PIECE_MM as f32);
    }
    let mut laid: Vec<LaidStrand> = Vec::with_capacity(n_new);
    let mut z: Vec<f32> = Vec::with_capacity(old.z.len());
    if p > 0 {
        let last = old.strands[p - 1];
        z.extend_from_slice(&old.z[..last.start + last.n]);
    }
    laid.extend(old.strands[..p].iter().zip(&list).map(|(ls, s)| LaidStrand {
        record: s.record,
        ..*ls
    }));
    let mut sc = Scratch::default();
    for k in p..n_new {
        let s = list[k];
        let after = k >= n_new - q;
        if after && !index.any(&changed_at, boxes[k]) {
            // Nothing that changed is under it: as it was.
            let os = old.strands[k + n_old - n_new];
            let start = z.len();
            z.extend_from_slice(&old.z[os.start..os.start + os.n]);
            laid.push(LaidStrand {
                start,
                record: s.record,
                ..os
            });
            continue;
        }
        // On the stitches before it but the one just before it (as `lay`).
        let limit = k.saturating_sub(1);
        let ls = {
            let (laid, z) = (&laid, &z);
            lay_one(s, cell, z.len(), |at| under(&index, &base, laid, z, limit, at), &mut sc)
        };
        let Some(ls) = ls else { continue };
        let changed = !after || {
            let os = old.strands[k + n_old - n_new];
            os.thick != ls.thick || old.z[os.start..os.start + os.n] != sc.z[..]
        };
        z.extend_from_slice(&sc.z);
        if changed {
            index.mark(&mut changed_at, boxes[k]);
            changes.add(boxes[k]);
        }
        laid.push(ls);
    }
    // The grid: the cells that no change reaches as in the old grid, the
    // others from the pieces and the stitches again.
    let mut grid = base.clone();
    let og = &old.grid;
    for y in 0..grid.h {
        let oy = grid.oy + y as i64 - og.oy;
        let from = (og.ox - grid.ox).max(0);
        let to = (og.ox + og.w as i64 - grid.ox).min(grid.w as i64);
        if oy < 0 || oy >= og.h as i64 || from >= to {
            continue;
        }
        let (dst, n) = (y * grid.w + from as usize, (to - from) as usize);
        let src = oy as usize * og.w + (grid.ox + from - og.ox) as usize;
        grid.height[dst..dst + n].copy_from_slice(&og.height[src..src + n]);
        grid.layers[dst..dst + n].copy_from_slice(&og.layers[src..src + n]);
    }
    for (square, ids) in index.lists.iter().enumerate() {
        if !changed_at[square] {
            continue;
        }
        // The cells whose centers are in the square.
        let (sx, sy) = (
            index.ox + (square % index.w) as i64,
            index.oy + (square / index.w) as i64,
        );
        let near = |o: i64, sq: i64, n: usize| {
            let a = ((sq as f64 * index.side / cell).floor() as i64 - 1 - o).clamp(0, n as i64) as usize;
            let e = (((sq + 1) as f64 * index.side / cell).ceil() as i64 + 1 - o).clamp(0, n as i64) as usize;
            a.min(e)..e
        };
        let xs: Vec<usize> = near(grid.ox, sx, grid.w)
            .filter(|&x| (grid.center(x, 0).x / index.side).floor() as i64 == sx)
            .collect();
        let ys: Vec<usize> = near(grid.oy, sy, grid.h)
            .filter(|&y| (grid.center(0, y).y / index.side).floor() as i64 == sy)
            .collect();
        let (Some(&x0), Some(&y0)) = (xs.first(), ys.first()) else {
            continue;
        };
        let (cols, rows) = (x0..x0 + xs.len(), y0..y0 + ys.len());
        for y in rows.clone() {
            let i = y * grid.w;
            grid.height[i + cols.start..i + cols.end].copy_from_slice(&base.height[i + cols.start..i + cols.end]);
            grid.layers[i + cols.start..i + cols.end].fill(0);
        }
        for &j in ids {
            let s = laid[j as usize];
            let (sx, sy) = (grid.xs(s.bbox.min.x, s.bbox.max.x), grid.ys(s.bbox.min.y, s.bbox.max.y));
            let xs = sx.start.max(cols.start)..sx.end.min(cols.end);
            let ys = sy.start.max(rows.start)..sy.end.min(rows.end);
            grid.stamp_in(&s, &z, xs, ys);
        }
    }
    (Laid::new(laid, z, areas.to_vec(), grid, b), changes)
}

/// The height and the layers of thread of the grid at `p` when the strands
/// before `limit` are laid: the pieces and the strands of the square of
/// `p` (as the grid of [`lay`] has them).
fn under(index: &Index, base: &Grid, laid: &[LaidStrand], z: &[f32], limit: usize, p: Point) -> (f64, u8) {
    let cell = base.cell;
    let q = cell_center((p.x / cell).floor() as i64, (p.y / cell).floor() as i64, cell);
    let Some(i) = base.index(q) else {
        return (0.0, 0);
    };
    let mut h = base.height[i];
    let mut layers = 0u8;
    for &j in index.at(q) {
        let j = j as usize;
        if j >= limit {
            break;
        }
        if let Some((hj, middle)) = laid[j].height_at(q, z) {
            if hj > h {
                h = hj;
            }
            if middle {
                layers = layers.saturating_add(1);
            }
        }
    }
    (f64::from(h), layers)
}

impl Laid {
    fn new(strands: Vec<LaidStrand>, z: Vec<f32>, pieces: Vec<(Vec<Vec<Point>>, Rgb)>, grid: Grid, b: BBox) -> Laid {
        let index = Index::new(strands.iter().map(|s| s.bbox), b, INDEX_MM);
        let piece = if pieces.is_empty() { 0.0 } else { PIECE_MM };
        let top = z.iter().fold(piece, |m, v| m.max(f64::from(*v)));
        Laid {
            strands,
            z,
            pieces,
            grid,
            index,
            top,
        }
    }

    /// The highest point of the stitches and pieces, mm.
    pub fn top(&self) -> f64 {
        self.top
    }

    /// The area of the laid stitches and pieces, with a margin.
    pub fn bounds(&self) -> BBox {
        let g = &self.grid;
        BBox::new(
            Point::new(g.ox as f64 * g.cell, g.oy as f64 * g.cell),
            Point::new((g.ox + g.w as i64) as f64 * g.cell, (g.oy + g.h as i64) as f64 * g.cell),
        )
    }

    /// The areas of the strands whose record `f` keeps (for example the
    /// strands that show in one view and not in another).
    pub fn changes_where(&self, f: impl Fn(u32) -> bool) -> Changes {
        let mut c = Changes::default();
        for s in &self.strands {
            if f(s.record) {
                c.add(s.bbox);
            }
        }
        c
    }

    /// The height of the top at `p`, mm (from the grid of 0.1 mm).
    pub fn height_at(&self, p: Point) -> f64 {
        self.grid.height_at(p)
    }

    /// The heights of the cells in `b`.
    fn heights_in(&self, b: BBox) -> impl Iterator<Item = f64> + '_ {
        let g = &self.grid;
        let (xs, ys) = (g.xs(b.min.x, b.max.x), g.ys(b.min.y, b.max.y));
        ys.flat_map(move |y| xs.clone().map(move |x| f64::from(g.height[y * g.w + x])))
    }

    /// The highest point in `b`, mm.
    pub fn max_height_in(&self, b: BBox) -> f64 {
        self.heights_in(b).fold(0.0, f64::max)
    }

    /// The mean height in `b`, mm.
    pub fn mean_height_in(&self, b: BBox) -> f64 {
        let (sum, n) = self.heights_in(b).fold((0.0, 0usize), |(s, n), h| (s + h, n + 1));
        if n == 0 { 0.0 } else { sum / n as f64 }
    }

    /// The number of stitches laid.
    pub fn len(&self) -> usize {
        self.strands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strands.is_empty()
    }

    /// The size of the data, bytes.
    pub fn bytes(&self) -> usize {
        std::mem::size_of_val(self.strands.as_slice())
            + std::mem::size_of_val(self.z.as_slice())
            + self.grid.height.len() * 5
            + self.index.bytes()
    }

    /// The surface of the area `area` at `px_per_mm` (step 2). It is made
    /// at [`DETAIL_PX_PER_MM`] or more and halved to `px_per_mm` (the zoom
    /// levels, ADR 0125), so a pixel of a small zoom has the mean of the
    /// pixels under it.
    pub fn surface(&self, area: BBox, px_per_mm: f64) -> Surface {
        self.surface_in(area, px_per_mm, &Lighting::default())
    }

    /// As [`Laid::surface`], with the shadows of the threads for the light
    /// `lighting` (made at the detail and halved, as the other values).
    pub fn surface_in(&self, area: BBox, px_per_mm: f64, lighting: &Lighting) -> Surface {
        let width = ((area.width() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let height = ((area.height() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let mut k = 0u32;
        while px_per_mm * f64::from(1u32 << k) < DETAIL_PX_PER_MM - 1e-9
            && (width << (k + 1)) <= 16_384
            && (height << (k + 1)) <= 16_384
            && u64::from(width << (k + 1)) * u64::from(height << (k + 1)) <= MAX_DETAIL_PIXELS
        {
            k += 1;
        }
        let mut s = self.raster(
            area.min,
            (0, 0),
            px_per_mm * f64::from(1u32 << k),
            width << k,
            height << k,
            lighting,
            None,
            0,
        );
        for _ in 0..k {
            s = s.half();
        }
        s
    }

    /// The surface of the pixels `x0..x0 + w`, `y0..y0 + h` of the grid of
    /// pixels at `ppm` pixels for each mm, in which pixel `x` has its
    /// center at `(x + 0.5) / ppm` mm (the tiles of the canvas, ADR 0126),
    /// with the strands whose record `show` keeps, on this processor. It is
    /// made with its detail and halved as [`Laid::surface_in`] (at most
    /// [`MAX_BLOCK_HALVINGS`] times), with an apron of [`APRON_MM`] round
    /// it, so a pixel is the same in each block that has it. `None` if no
    /// shown thread and no piece is near.
    #[allow(clippy::too_many_arguments)]
    pub fn block(
        &self,
        ppm: f64,
        x0: i64,
        y0: i64,
        w: u32,
        h: u32,
        lighting: &Lighting,
        show: &dyn Fn(u32) -> bool,
    ) -> Option<Surface> {
        let mut k = 0u32;
        while ppm * f64::from(1u32 << k) < DETAIL_PX_PER_MM - 1e-9 && k < MAX_BLOCK_HALVINGS {
            k += 1;
        }
        self.block_at(ppm, k, x0, y0, w, h, lighting, show)
    }

    /// As [`Laid::block`], with `k` halvings; a block that is too large for
    /// its detail is made in parts.
    #[allow(clippy::too_many_arguments)]
    fn block_at(
        &self,
        ppm: f64,
        k: u32,
        x0: i64,
        y0: i64,
        w: u32,
        h: u32,
        lighting: &Lighting,
        show: &dyn Fn(u32) -> bool,
    ) -> Option<Surface> {
        let f = 1u32 << k;
        if w * f > MAX_BLOCK_SIDE || h * f > MAX_BLOCK_SIDE {
            let w1 = if w * f > MAX_BLOCK_SIDE { w / 2 } else { w };
            let h1 = if h * f > MAX_BLOCK_SIDE { h / 2 } else { h };
            let parts: Vec<(u32, u32, Option<Surface>)> = [
                (0, 0, w1, h1),
                (w1, 0, w - w1, h1),
                (0, h1, w1, h - h1),
                (w1, h1, w - w1, h - h1),
            ]
            .into_iter()
            .filter(|p| p.2 > 0 && p.3 > 0)
            .map(|(dx, dy, pw, ph)| {
                let part = self.block_at(ppm, k, x0 + i64::from(dx), y0 + i64::from(dy), pw, ph, lighting, show);
                (dx, dy, part)
            })
            .collect();
            if parts.iter().all(|p| p.2.is_none()) {
                return None;
            }
            let mut out = Surface::flat(ppm, x0, y0, w, h, lighting);
            for (dx, dy, part) in parts {
                if let Some(part) = part {
                    out.paste(&part, dx, dy);
                }
            }
            return Some(out);
        }
        // The block with its apron, in pixels of the level: as far as the
        // longest shadow of these stitches can fall, and the gaps.
        let apron_mm = shadow_reach(lighting, self.top) + GAP_RADIUS_MM + 0.2;
        let apron = (apron_mm * ppm).ceil().max(1.0) as i64;
        let (bx, by) = (x0 - apron, y0 - apron);
        let (bw, bh) = (i64::from(w) + 2 * apron, i64::from(h) + 2 * apron);
        let mm = BBox::new(
            Point::new(bx as f64 / ppm, by as f64 / ppm),
            Point::new((bx + bw) as f64 / ppm, (by + bh) as f64 / ppm),
        );
        let ids: Vec<u32> = self
            .index
            .near(mm)
            .into_iter()
            .filter(|&i| show(self.strands[i as usize].record))
            .collect();
        let pieces = self
            .pieces
            .iter()
            .any(|(rings, _)| BBox::from_points(rings.iter().flatten().copied()).is_some_and(|b| b.intersects(&mm)));
        if ids.is_empty() && !pieces {
            return None;
        }
        let f = i64::from(f);
        let mut s = self.raster(
            Point::ZERO,
            (bx * f, by * f),
            ppm * f as f64,
            (bw * f) as u32,
            (bh * f) as u32,
            lighting,
            Some(&ids),
            1,
        );
        for _ in 0..k {
            s = s.half_with(1);
        }
        let mut out = s.crop(apron as u32, apron as u32, w, h);
        out.area = grid_area(ppm, x0, y0, w, h);
        Some(out)
    }

    /// The surface at `ppm` with the size `width` × `height` pixels, with
    /// no halving: pixel (`x`, `y`) has its center at `base + (origin + (x,
    /// y) + 0.5) / ppm`. With `ids`, only these strands (in sew order). On
    /// `threads` processors (0: on all).
    #[allow(clippy::too_many_arguments)]
    fn raster(
        &self,
        base: Point,
        origin: (i64, i64),
        ppm: f64,
        width: u32,
        height: u32,
        lighting: &Lighting,
        ids: Option<&[u32]>,
        threads: usize,
    ) -> Surface {
        let (tx, ty) = (width.div_ceil(TILE), height.div_ceil(TILE));
        // The stitches of each tile, in sew order.
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); (tx * ty) as usize];
        let to_px = |v: f64, m: f64, o: i64| (v - m) * ppm - o as f64;
        let mut add = |i: u32| {
            let b = self.strands[i as usize].bbox.expand(1.0 / ppm);
            let (x0, x1) = (to_px(b.min.x, base.x, origin.0), to_px(b.max.x, base.x, origin.0));
            let (y0, y1) = (to_px(b.min.y, base.y, origin.1), to_px(b.max.y, base.y, origin.1));
            if x1 < 0.0 || y1 < 0.0 || x0 >= f64::from(width) || y0 >= f64::from(height) {
                return;
            }
            let t = f64::from(TILE);
            let (cx0, cx1) = ((x0.max(0.0) / t) as u32, ((x1 / t) as u32).min(tx - 1));
            let (cy0, cy1) = ((y0.max(0.0) / t) as u32, ((y1 / t) as u32).min(ty - 1));
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    lists[(cy * tx + cx) as usize].push(i);
                }
            }
        };
        match ids {
            Some(ids) => ids.iter().for_each(|&i| add(i)),
            None => (0..self.strands.len() as u32).for_each(add),
        }
        let tiles = in_parallel_with(threads, (tx * ty) as usize, |k| {
            let (cx, cy) = (k as u32 % tx, k as u32 / tx);
            let (x0, y0) = (cx * TILE, cy * TILE);
            let (w, h) = (TILE.min(width - x0), TILE.min(height - y0));
            self.tile(base, origin, ppm, x0, y0, w, h, &lists[k])
        });
        let min = Point::new(base.x + origin.0 as f64 / ppm, base.y + origin.1 as f64 / ppm);
        let area = BBox::new(
            min,
            Point::new(min.x + f64::from(width) / ppm, min.y + f64::from(height) / ppm),
        );
        let mut s = Surface {
            area,
            px_per_mm: ppm,
            width,
            height,
            tiles_x: tx,
            tiles,
            light: *lighting,
        };
        s.finish(threads);
        s
    }

    /// One tile: the pixels `x0..x0 + w`, `y0..y0 + h` of the surface (see
    /// [`Laid::raster`]).
    #[allow(clippy::too_many_arguments)]
    fn tile(&self, base: Point, origin: (i64, i64), ppm: f64, x0: u32, y0: u32, w: u32, h: u32, ids: &[u32]) -> Tile {
        let mut t = Tile::new(x0, y0, w, h);
        let center = |px: u32, py: u32| {
            Point::new(
                base.x + ((origin.0 + i64::from(px)) as f64 + 0.5) / ppm,
                base.y + ((origin.1 + i64::from(py)) as f64 + 0.5) / ppm,
            )
        };
        // The area of the tile, for the pieces.
        let (c0, c1) = (center(x0, y0), center(x0 + w, y0 + h));
        let area = BBox::new(c0, c1).expand(1.0 / ppm);
        for (rings, c) in &self.pieces {
            if BBox::from_points(rings.iter().flatten().copied()).is_none_or(|b| !b.intersects(&area)) {
                continue;
            }
            for py in 0..h {
                for px in 0..w {
                    if inside(rings, center(x0 + px, y0 + py)) {
                        let k = (py * w + px) as usize;
                        t.height[k] = PIECE_MM as f32;
                        t.color[k] = [c.0, c.1, c.2];
                        t.material[k] = material::PIECE;
                        t.cover[k] = 255;
                    }
                }
            }
        }
        for &i in ids {
            let s = &self.strands[i as usize];
            let z = &self.z[s.start..s.start + s.n];
            // Small changes of each thread (ADR 0125): its width and its
            // brightness, from its place.
            let half = s.half * (1.0 + VARY_WIDTH * f64::from(s.vary[0]));
            let shade = 1.0 + VARY_BRIGHTNESS * f64::from(s.vary[1]);
            let color =
                [s.color.0, s.color.1, s.color.2].map(|c| (f64::from(c) * shade).round().clamp(0.0, 255.0) as u8);
            let r_px = half * ppm;
            let b = s.bbox.expand(1.0 / ppm);
            let fx = |v: f64| (v - base.x) * ppm - origin.0 as f64;
            let fy = |v: f64| (v - base.y) * ppm - origin.1 as f64;
            let px0 = ((fx(b.min.x).floor().max(0.0) as u32).max(x0)).min(x0 + w) - x0;
            let px1 = ((fx(b.max.x).ceil().max(0.0)) as u32).min(x0 + w).max(x0) - x0;
            let py0 = ((fy(b.min.y).floor().max(0.0) as u32).max(y0)).min(y0 + h) - y0;
            let py1 = ((fy(b.max.y).ceil().max(0.0)) as u32).min(y0 + h).max(y0) - y0;
            for py in py0..py1 {
                for px in px0..px1 {
                    let q = center(x0 + px, y0 + py);
                    let (rx, ry) = (q.x - s.a.x, q.y - s.a.y);
                    let along = rx * s.dir.0 + ry * s.dir.1;
                    let tc = along.clamp(0.0, s.len);
                    let (ex, ey) = (rx - s.dir.0 * tc, ry - s.dir.1 * tc);
                    let d_px = (ex * ex + ey * ey).sqrt() * ppm;
                    let cover = (r_px + 0.5 - d_px).clamp(0.0, 1.0);
                    if cover <= 0.0 {
                        continue;
                    }
                    let side = (ry * s.dir.0 - rx * s.dir.1) / half;
                    let across = side.abs().min(1.0);
                    let zc = s.middle(z, along);
                    let hz = zc * ((1.0 - across * across).sqrt() as f32);
                    let k = (py * w + px) as usize;
                    // The union of the threads in the pixel (each one covers
                    // its part of the part that the others leave open).
                    let open = (1.0 - f64::from(t.cover[k]) / 255.0) * (1.0 - cover);
                    t.cover[k] = ((1.0 - open) * 255.0).round() as u8;
                    if hz >= t.height[k] {
                        t.height[k] = hz;
                        t.color[k] = color;
                        t.axis[k] = s.axis;
                        t.material[k] = s.material;
                        // The twist of the plies: a pattern along the
                        // thread, slanted across it.
                        let p = emb_geom::math::sin(std::f64::consts::TAU * (along / PLY_PITCH_MM + 0.6 * side));
                        t.ply[k] = (p * 127.0).round() as i8;
                    }
                }
            }
        }
        t
    }
}

/// How long a shadow can be, mm.
const SHADOW_MAX_MM: f64 = 4.0;

/// How far the shadow of a part `top` mm high falls in the light `l`, mm (at
/// most [`SHADOW_MAX_MM`]).
fn shadow_reach(l: &Lighting, top: f64) -> f64 {
    let v = l.vector();
    let flat = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if flat < 1e-6 || l.strength <= 0.0 {
        return 0.0;
    }
    (top * flat / v[2]).min(SHADOW_MAX_MM)
}
/// A part this far above the line to the light makes a full shadow, mm (a
/// softer shadow for less).
const SHADOW_SOFT_MM: f64 = 0.08;

/// The side of the squares of [`Tops`], pixels.
const TOP_SQUARE: usize = 16;

/// The highest point in each square of [`TOP_SQUARE`] pixels of a map of
/// heights: a line to the light that is above them all has no shadow.
struct Tops {
    w: usize,
    h: usize,
    across: usize,
    max: Vec<f32>,
}

impl Tops {
    fn new(heights: &[f32], w: usize, h: usize) -> Tops {
        let (across, down) = (w.div_ceil(TOP_SQUARE), h.div_ceil(TOP_SQUARE));
        let mut max = vec![0.0f32; across * down];
        for y in 0..h {
            let row = &heights[y * w..(y + 1) * w];
            let line = &mut max[(y / TOP_SQUARE) * across..(y / TOP_SQUARE + 1) * across];
            for (x, v) in row.iter().enumerate() {
                let m = &mut line[x / TOP_SQUARE];
                if *v > *m {
                    *m = *v;
                }
            }
        }
        Tops { w, h, across, max }
    }

    /// The highest point in the pixels `x0..=x1`, `y0..=y1` (or more).
    fn max_in(&self, x0: i64, y0: i64, x1: i64, y1: i64) -> f64 {
        let c = |v: i64, n: usize| (v.clamp(0, n as i64 - 1) as usize) / TOP_SQUARE;
        let (ax, bx) = (c(x0.min(x1), self.w), c(x0.max(x1), self.w));
        let (ay, by) = (c(y0.min(y1), self.h), c(y0.max(y1), self.h));
        let mut m = 0.0f32;
        for y in ay..=by {
            for v in &self.max[y * self.across + ax..=y * self.across + bx] {
                m = m.max(*v);
            }
        }
        f64::from(m)
    }
}

/// The shadow at the pixel (`x`, `y`) of heights `at` (`w` × `h` pixels at
/// `ppm`; the highest point `top`) in the light from the direction `v`
/// (`Lighting::vector`) with the strength `strength`: 0 in the light, 1 in
/// full shadow. The line from the pixel to the light goes up at the
/// height of the light; a part above it puts the pixel in its shadow. With
/// `tops`, the line stops where it is above all heights in front of it
/// (the same result, fewer steps).
#[allow(clippy::too_many_arguments)]
pub(crate) fn shadow_at(
    at: &impl Fn(i64, i64) -> f64,
    w: i64,
    h: i64,
    x: i64,
    y: i64,
    v: [f64; 3],
    strength: f64,
    ppm: f64,
    top: f64,
) -> f64 {
    shadow_with(at, None, w, h, x, y, v, strength, ppm, top)
}

/// As [`shadow_at`], with the highest points in squares.
#[allow(clippy::too_many_arguments)]
fn shadow_with(
    at: &impl Fn(i64, i64) -> f64,
    tops: Option<&Tops>,
    w: i64,
    h: i64,
    x: i64,
    y: i64,
    v: [f64; 3],
    strength: f64,
    ppm: f64,
    top: f64,
) -> f64 {
    let flat = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if flat < 1e-6 || strength <= 0.0 {
        return 0.0;
    }
    // A step of one pixel along the line, and its rise, mm.
    let (ux, uy) = (v[0] / flat, v[1] / flat);
    let rise = v[2] / flat / ppm;
    let h0 = at(x, y);
    let mut steps = ((top - h0) / rise).min(SHADOW_MAX_MM * ppm).ceil().max(0.0) as i64;
    if let Some(tops) = tops
        && steps > 0
    {
        // The highest point in front of the pixel: after it no part is
        // above the line.
        let (ex, ey) = (x as f64 + 0.5 + ux * steps as f64, y as f64 + 0.5 + uy * steps as f64);
        let m = tops.max_in(x, y, ex.floor() as i64, ey.floor() as i64);
        steps = steps.min(((m - h0) / rise).ceil().max(0.0) as i64);
    }
    let mut dark: f64 = 0.0;
    for k in 1..=steps {
        let (px, py) = (x as f64 + 0.5 + ux * k as f64, y as f64 + 0.5 + uy * k as f64);
        if px < 0.0 || py < 0.0 || px >= w as f64 || py >= h as f64 {
            break;
        }
        let over = at(px as i64, py as i64) - (h0 + rise * k as f64);
        dark = dark.max(over / SHADOW_SOFT_MM);
        if dark >= 1.0 {
            return 1.0;
        }
    }
    dark.clamp(0.0, 1.0)
}

/// The number of processors for work on `threads` processors (0: all).
fn processors(threads: usize) -> usize {
    if threads == 0 {
        std::thread::available_parallelism().map_or(1, usize::from)
    } else {
        threads
    }
}

/// Runs `f` for `0..count` on `threads` processors (0: all) and gives the
/// results in order.
fn in_parallel_with<T: Send>(threads: usize, count: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let workers = processors(threads).min(count).max(1);
    if workers == 1 {
        return (0..count).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut made: Vec<(usize, T)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        if k >= count {
                            break out;
                        }
                        out.push((k, f(k)));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a thread of the surface does not stop with an error"))
            .collect()
    });
    made.sort_by_key(|(k, _)| *k);
    made.into_iter().map(|(_, t)| t).collect()
}

/// The sum of the values of a row or a column in a window of `2r + 1`.
fn box_sum(src: &[f32], n: usize, stride: usize, r: usize, out: &mut [f32]) {
    let mut acc = 0.0f64;
    for k in 0..n.min(r + 1) {
        acc += f64::from(src[k * stride]);
    }
    for k in 0..n {
        out[k * stride] = acc as f32;
        if k + r + 1 < n {
            acc += f64::from(src[(k + r + 1) * stride]);
        }
        if k >= r {
            acc -= f64::from(src[(k - r) * stride]);
        }
    }
}

/// The mean of the values in a square of `2r + 1` (fewer at the edges).
fn box_mean(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut rows = vec![0.0f32; w * h];
    for y in 0..h {
        box_sum(&src[y * w..(y + 1) * w], w, 1, r, &mut rows[y * w..(y + 1) * w]);
    }
    let mut out = vec![0.0f32; w * h];
    for x in 0..w {
        box_sum(&rows[x..], h, w, r, &mut out[x..]);
    }
    for y in 0..h {
        let ny = (y + r + 1).min(h) - y.saturating_sub(r);
        for x in 0..w {
            let nx = (x + r + 1).min(w) - x.saturating_sub(r);
            out[y * w + x] /= (nx * ny) as f32;
        }
    }
    out
}

/// A tile of a surface: for each pixel, row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub x0: u32,
    pub y0: u32,
    pub w: u32,
    pub h: u32,
    /// The height of the top, mm.
    pub height: Vec<f32>,
    /// The mean direction of the surface (the normal, x, y and z times
    /// 127; at a small zoom shorter than 1).
    pub normal: Vec<[i8; 3]>,
    /// The color of the thread or the piece on top.
    pub color: Vec<[u8; 3]>,
    /// The direction of the thread on top, as the vector of two times its
    /// angle times 127: shorter where threads of more directions mix (the
    /// spread of the directions at a small zoom).
    pub axis: Vec<[i8; 2]>,
    /// What is on top ([`material`]).
    pub material: Vec<u8>,
    /// The part of the pixel that threads or pieces cover, 0 to 255 (the
    /// union of the parts of each thread).
    pub cover: Vec<u8>,
    /// The light that the gaps let in, 0 to 255 (a gap between stitches or
    /// a needle hole is darker).
    pub open: Vec<u8>,
    /// The twist of the plies at the pixel, −127 to 127.
    pub ply: Vec<i8>,
    /// The shadow at the pixel in the light of the surface, 0 (in the
    /// light) to 255 (in full shadow).
    pub shadow: Vec<u8>,
    /// The diffuse light at the pixel in the light of the surface, as a
    /// factor from 0 (0) to 2 (255).
    pub diffuse: Vec<u8>,
    /// The light of the fabric in the light of the surface (its gaps and
    /// its shadow), 0 to 255. At a small zoom it is the mean over the part
    /// that the threads do not cover; the values above are the means over
    /// the part that they cover.
    pub fabric_shade: Vec<u8>,
}

impl Tile {
    /// A tile with no thread and no piece in the light `lighting` (as
    /// [`Surface::finish`] makes it).
    fn flat(x0: u32, y0: u32, w: u32, h: u32, lighting: &Lighting) -> Tile {
        let mut t = Tile::new(x0, y0, w, h);
        let lit = crate::sheen::diffuse([0.0, 0.0, 1.0], lighting.vector());
        t.diffuse.fill((lit / 2.0 * 255.0).round().clamp(0.0, 255.0) as u8);
        t
    }

    /// Puts the pixel `i` of `src` in the pixel `k`.
    fn copy_pixel(&mut self, k: usize, src: &Tile, i: usize) {
        self.height[k] = src.height[i];
        self.normal[k] = src.normal[i];
        self.color[k] = src.color[i];
        self.axis[k] = src.axis[i];
        self.material[k] = src.material[i];
        self.cover[k] = src.cover[i];
        self.open[k] = src.open[i];
        self.ply[k] = src.ply[i];
        self.shadow[k] = src.shadow[i];
        self.diffuse[k] = src.diffuse[i];
        self.fabric_shade[k] = src.fabric_shade[i];
    }

    fn new(x0: u32, y0: u32, w: u32, h: u32) -> Tile {
        let n = (w * h) as usize;
        Tile {
            x0,
            y0,
            w,
            h,
            height: vec![0.0; n],
            normal: vec![[0, 0, 127]; n],
            color: vec![[0; 3]; n],
            axis: vec![[0; 2]; n],
            material: vec![material::FABRIC; n],
            cover: vec![0; n],
            open: vec![255; n],
            ply: vec![0; n],
            shadow: vec![0; n],
            diffuse: vec![128; n],
            fabric_shade: vec![255; n],
        }
    }
}

/// The surface of an area at a zoom: tiles of [`TILE`] pixels, row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    /// The area of the pixels (at least the area asked for).
    pub area: BBox,
    pub px_per_mm: f64,
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles: Vec<Tile>,
    /// The light of the shadows of the surface.
    pub light: Lighting,
}

/// The area of the pixels `x0..x0 + w`, `y0..y0 + h` of the grid of pixels
/// at `ppm` (see [`Laid::block`]).
fn grid_area(ppm: f64, x0: i64, y0: i64, w: u32, h: u32) -> BBox {
    BBox::new(
        Point::new(x0 as f64 / ppm, y0 as f64 / ppm),
        Point::new((x0 + i64::from(w)) as f64 / ppm, (y0 + i64::from(h)) as f64 / ppm),
    )
}

impl Surface {
    /// A surface with no thread and no piece in the light `lighting`: the
    /// pixels `x0..x0 + w`, `y0..y0 + h` of the grid of pixels at `ppm`.
    fn flat(ppm: f64, x0: i64, y0: i64, w: u32, h: u32, lighting: &Lighting) -> Surface {
        let (tx, ty) = (w.div_ceil(TILE), h.div_ceil(TILE));
        let mut tiles = Vec::with_capacity((tx * ty) as usize);
        for cy in 0..ty {
            for cx in 0..tx {
                let (x, y) = (cx * TILE, cy * TILE);
                tiles.push(Tile::flat(x, y, TILE.min(w - x), TILE.min(h - y), lighting));
            }
        }
        Surface {
            area: grid_area(ppm, x0, y0, w, h),
            px_per_mm: ppm,
            width: w,
            height: h,
            tiles_x: tx,
            tiles,
            light: *lighting,
        }
    }

    /// The pixels `x0..x0 + w`, `y0..y0 + h` of the surface.
    pub fn crop(&self, x0: u32, y0: u32, w: u32, h: u32) -> Surface {
        let ppm = self.px_per_mm;
        let mut out = Surface::flat(ppm, 0, 0, w, h, &self.light);
        out.paste_from(self, x0, y0);
        out.area = BBox::new(
            Point::new(
                self.area.min.x + f64::from(x0) / ppm,
                self.area.min.y + f64::from(y0) / ppm,
            ),
            Point::new(
                self.area.min.x + f64::from(x0 + w) / ppm,
                self.area.min.y + f64::from(y0 + h) / ppm,
            ),
        );
        out
    }

    /// Puts the pixels of `src` from its pixel (`x0`, `y0`) in all pixels of
    /// this surface.
    fn paste_from(&mut self, src: &Surface, x0: u32, y0: u32) {
        for t in &mut self.tiles {
            for row in 0..t.h {
                for col in 0..t.w {
                    let (s, i) = src.at(x0 + t.x0 + col, y0 + t.y0 + row);
                    t.copy_pixel((row * t.w + col) as usize, s, i);
                }
            }
        }
    }

    /// Puts all pixels of `src` in this surface from its pixel (`dx`, `dy`).
    fn paste(&mut self, src: &Surface, dx: u32, dy: u32) {
        for t in &src.tiles {
            for row in 0..t.h {
                for col in 0..t.w {
                    let (x, y) = (dx + t.x0 + col, dy + t.y0 + row);
                    let k = ((y / TILE) * self.tiles_x + x / TILE) as usize;
                    let d = &mut self.tiles[k];
                    let j = ((y - d.y0) * d.w + (x - d.x0)) as usize;
                    d.copy_pixel(j, t, (row * t.w + col) as usize);
                }
            }
        }
    }

    /// The tile of the pixel (`x`, `y`) and the index of the pixel in it.
    pub fn at(&self, x: u32, y: u32) -> (&Tile, usize) {
        let t = &self.tiles[((y / TILE) * self.tiles_x + x / TILE) as usize];
        (t, ((y - t.y0) * t.w + (x - t.x0)) as usize)
    }

    /// The height at the pixel (`x`, `y`), mm; the nearest pixel of the
    /// surface for a place outside it.
    pub fn height(&self, x: i64, y: i64) -> f32 {
        let x = x.clamp(0, i64::from(self.width) - 1) as u32;
        let y = y.clamp(0, i64::from(self.height) - 1) as u32;
        let (t, i) = self.at(x, y);
        t.height[i]
    }

    /// The highest point of the surface, mm.
    pub fn max_height(&self) -> f32 {
        self.tiles
            .iter()
            .flat_map(|t| t.height.iter().copied())
            .fold(0.0, f32::max)
    }

    /// The size of the data, bytes (20 for each pixel).
    pub fn bytes(&self) -> usize {
        self.tiles.iter().map(|t| t.height.len() * 20).sum()
    }

    /// The normals, the gaps and the shadows, from the heights (after the
    /// tiles), on `threads` processors (0: all).
    fn finish(&mut self, threads: usize) {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut flat = vec![0.0f32; w * h];
        for t in &self.tiles {
            for row in 0..t.h as usize {
                let src = &t.height[row * t.w as usize..(row + 1) * t.w as usize];
                let at = (t.y0 as usize + row) * w + t.x0 as usize;
                flat[at..at + t.w as usize].copy_from_slice(src);
            }
        }
        let r = ((GAP_RADIUS_MM * self.px_per_mm).round() as usize).max(1);
        let around = box_mean(&flat, w, h, r);
        let d = 2.0 / self.px_per_mm;
        let (flat, around) = (&flat, &around);
        let at = move |x: i64, y: i64| {
            f64::from(flat[y.clamp(0, h as i64 - 1) as usize * w + x.clamp(0, w as i64 - 1) as usize])
        };
        let top = f64::from(flat.iter().copied().fold(0.0, f32::max));
        let tops = Tops::new(flat, w, h);
        let tops = &tops;
        let (light, ppm) = (self.light, self.px_per_mm);
        // The direction of the light, one time (not for each pixel).
        let lv = light.vector();
        let work = move |chunk: &mut [Tile]| {
            for t in chunk {
                for row in 0..t.h {
                    for col in 0..t.w {
                        let (x, y) = (i64::from(t.x0 + col), i64::from(t.y0 + row));
                        let k = (row * t.w + col) as usize;
                        let dzdx = (at(x + 1, y) - at(x - 1, y)) / d;
                        let dzdy = (at(x, y + 1) - at(x, y - 1)) / d;
                        let len = (dzdx * dzdx + dzdy * dzdy + 1.0).sqrt();
                        t.normal[k] = [-dzdx / len, -dzdy / len, 1.0 / len].map(|v| (v * 127.0).round() as i8);
                        let depth = (f64::from(around[y as usize * w + x as usize]) - at(x, y)).max(0.0);
                        let open = 1.0 - GAP_DARK * (depth / GAP_DEPTH_MM).min(1.0);
                        t.open[k] = (open * 255.0).round() as u8;
                        let dark = shadow_with(&at, Some(tops), w as i64, h as i64, x, y, lv, light.strength, ppm, top);
                        t.shadow[k] = (dark * 255.0).round() as u8;
                        let shade = open * (1.0 - crate::sheen::SHADOW_DARK * dark);
                        t.fabric_shade[k] = (shade * 255.0).round() as u8;
                        let n = [-dzdx / len, -dzdy / len, 1.0 / len];
                        let lit = crate::sheen::diffuse(n, lv);
                        t.diffuse[k] = (lit / 2.0 * 255.0).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        };
        let workers = processors(threads).min(self.tiles.len()).max(1);
        if workers == 1 {
            work(&mut self.tiles);
            return;
        }
        let per = self.tiles.len().div_ceil(workers).max(1);
        std::thread::scope(|scope| {
            for chunk in self.tiles.chunks_mut(per) {
                scope.spawn(move || work(chunk));
            }
        });
    }

    /// The zoom level with half the pixels on each side: each pixel has the
    /// mean height, slope, gaps and plies of the four pixels under it, the
    /// mean color of their threads (in linear light), the part that they
    /// cover, the mean of their thread directions (shorter where they
    /// differ) and the material that covers most of them (ADR 0125).
    pub fn half(&self) -> Surface {
        self.half_with(0)
    }

    /// As [`Surface::half`], on `threads` processors (0: all).
    fn half_with(&self, threads: usize) -> Surface {
        let (width, height) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let (tx, ty) = (width.div_ceil(TILE), height.div_ceil(TILE));
        let tiles = in_parallel_with(threads, (tx * ty) as usize, |k| {
            let (cx, cy) = (k as u32 % tx, k as u32 / tx);
            let (x0, y0) = (cx * TILE, cy * TILE);
            let mut t = Tile::new(x0, y0, TILE.min(width - x0), TILE.min(height - y0));
            for row in 0..t.h {
                for col in 0..t.w {
                    let (x, y) = (2 * (x0 + col), 2 * (y0 + row));
                    let mut n = 0.0f64;
                    // The light of the threads is the mean over the part that
                    // they cover; the light of the fabric over the rest.
                    let (mut hs, mut cov, mut open, mut ply, mut dark) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
                    let (mut lit, mut bare, mut fabric) = (0.0f64, 0.0f64, 0.0f64);
                    let mut plain = [0.0f64; 5];
                    let mut normal = [0.0f64; 3];
                    let mut lin = [0.0f64; 3];
                    let mut axis = [0.0f64; 2];
                    let mut mats: [(u8, f64); 4] = [(0, 0.0); 4];
                    for (j, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
                        if x + dx >= self.width || y + dy >= self.height {
                            continue;
                        }
                        let (s, i) = self.at(x + dx, y + dy);
                        let c = f64::from(s.cover[i]) / 255.0;
                        n += 1.0;
                        hs += f64::from(s.height[i]);
                        cov += c;
                        let values = [s.open[i], s.shadow[i], s.diffuse[i], s.fabric_shade[i]].map(f64::from);
                        open += c * values[0];
                        dark += c * values[1];
                        lit += c * values[2];
                        ply += c * f64::from(s.ply[i]);
                        bare += 1.0 - c;
                        fabric += (1.0 - c) * values[3];
                        for (p, v) in plain.iter_mut().zip(values.iter().chain([f64::from(s.ply[i])].iter())) {
                            *p += v;
                        }
                        for (a, v) in normal.iter_mut().zip(s.normal[i]) {
                            *a += c * f64::from(v);
                        }
                        for (a, v) in lin.iter_mut().zip(s.color[i]) {
                            *a += c * crate::light::to_linear(v);
                        }
                        for (a, v) in axis.iter_mut().zip(s.axis[i]) {
                            *a += c * f64::from(v);
                        }
                        mats[j] = (s.material[i], c);
                    }
                    let i = (row * t.w + col) as usize;
                    t.height[i] = (hs / n) as f32;
                    t.cover[i] = (cov / n * 255.0).round() as u8;
                    if cov > 0.0 {
                        t.normal[i] = normal.map(|v| (v / cov).round() as i8);
                        t.open[i] = (open / cov).round() as u8;
                        t.shadow[i] = (dark / cov).round() as u8;
                        t.diffuse[i] = (lit / cov).round() as u8;
                        t.ply[i] = (ply / cov).round() as i8;
                    } else {
                        t.open[i] = (plain[0] / n).round() as u8;
                        t.shadow[i] = (plain[1] / n).round() as u8;
                        t.diffuse[i] = (plain[2] / n).round() as u8;
                        t.ply[i] = (plain[4] / n).round() as i8;
                    }
                    t.fabric_shade[i] = if bare > 0.0 { fabric / bare } else { plain[3] / n }.round() as u8;
                    if cov > 0.0 {
                        t.color[i] = lin.map(|v| crate::light::to_srgb(v / cov));
                        t.axis[i] = axis.map(|v| (v / cov).round() as i8);
                        // The material that covers most.
                        let mut best = (material::FABRIC, -1.0);
                        for (m, _) in mats {
                            let sum: f64 = mats.iter().filter(|(x, _)| *x == m).map(|(_, c)| c).sum();
                            if sum > best.1 {
                                best = (m, sum);
                            }
                        }
                        t.material[i] = best.0;
                    }
                }
            }
            t
        });
        Surface {
            area: self.area,
            px_per_mm: self.px_per_mm / 2.0,
            width,
            height,
            tiles_x: tx,
            tiles,
            light: self.light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strand(a: (f64, f64), b: (f64, f64)) -> Strand {
        Strand {
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            color: Rgb(120, 60, 60),
            kind: ThreadKind::Polyester,
            weight: 40,
            lift_mm: 0.0,
            record: 0,
        }
    }

    fn top() -> f64 {
        THICKNESS * emb_model::thread::WIDTH_40_MM
    }

    #[test]
    fn a_stitch_rises_from_its_needle_points() {
        let laid = lay(&[strand((0.0, 0.0), (5.0, 0.0))], &[]);
        let at = |x: f64, y: f64| laid.height_at(Point::new(x, y));
        assert!((at(2.5, 0.0) - top()).abs() < 0.01, "{}", at(2.5, 0.0));
        assert!(at(0.05, 0.0) < 0.6 * top(), "{}", at(0.05, 0.0));
        assert_eq!(at(2.5, 0.5), 0.0, "off the thread");
        // A short stitch is an arch, lower than a long one.
        let short = lay(&[strand((0.0, 0.0), (0.4, 0.0))], &[]);
        let h = short.height_at(Point::new(0.2, 0.0));
        assert!(h > 0.0 && h < 0.95 * top(), "{h}");
        // A thinner thread is lower.
        let fine = lay(
            &[Strand {
                weight: 60,
                ..strand((0.0, 0.0), (5.0, 0.0))
            }],
            &[],
        );
        assert!(fine.height_at(Point::new(2.5, 0.0)) < 0.85 * top());
    }

    #[test]
    fn a_stitch_lies_on_what_is_under_it() {
        // A stitch across the first one, and a third that is not the one
        // just before.
        let laid = lay(
            &[
                strand((0.0, 0.0), (6.0, 0.0)),
                strand((10.0, 5.0), (10.0, 6.0)),
                strand((3.0, -3.0), (3.0, 3.0)),
            ],
            &[],
        );
        let cross = laid.height_at(Point::new(3.0, 0.0));
        assert!(cross > 1.7 * top(), "{cross}");
        // A taut line from the needle points over the crossing.
        let near = laid.height_at(Point::new(3.0, 1.5));
        assert!(near > 1.2 * top() && near < cross, "{near}");
        assert!(laid.height_at(Point::new(3.0, 2.8)) < 1.1 * top());
    }

    #[test]
    fn a_stitch_lies_beside_the_stitch_before_it() {
        // Satin stitches: each shares a needle point with the one before;
        // the column has the height of one thread, in each sew direction.
        let mut zig = Vec::new();
        for k in 0..20 {
            let y = 0.2 * k as f64;
            let (a, b) = if k % 2 == 0 {
                ((0.0, y), (4.0, y + 0.2))
            } else {
                ((4.0, y), (0.0, y + 0.2))
            };
            zig.push(strand(a, b));
        }
        let up: Vec<Strand> = zig.iter().rev().map(|s| Strand { a: s.b, b: s.a, ..*s }).collect();
        let b = BBox::new(Point::new(1.0, 1.0), Point::new(3.0, 3.0));
        let (down_h, up_h) = (lay(&zig, &[]).max_height_in(b), lay(&up, &[]).max_height_in(b));
        assert!(down_h < 1.3 * top(), "{down_h}");
        assert!((down_h - up_h).abs() < 0.02, "{down_h} {up_h}");
    }

    #[test]
    fn a_taut_line_is_the_upper_hull() {
        let t = [0.0, 1.0, 2.0, 3.0, 4.0];
        let mut out = Vec::new();
        taut(&t, &[0.0, 0.0, 1.0, 0.0, 0.0], &mut out);
        assert_eq!(out, vec![0.0, 0.5, 1.0, 0.5, 0.0]);
        taut(&t, &[0.0, 1.0, 0.2, 1.0, 0.0], &mut out);
        assert_eq!(out, vec![0.0, 1.0, 1.0, 1.0, 0.0]);
        // A bump near a needle point lifts the middle less than its height.
        let t: Vec<f64> = (0..11).map(f64::from).collect();
        let mut u = vec![0.0; 11];
        u[1] = 1.0;
        taut(&t, &u, &mut out);
        assert!((out[5] - 0.5556).abs() < 1e-3, "{}", out[5]);
    }

    #[test]
    fn thread_presses_thread() {
        // Ten stitches over the same place: less than ten threads high.
        let s: Vec<Strand> = (0..10)
            .flat_map(|_| [strand((0.0, 0.0), (5.0, 0.0)), strand((9.0, 9.0), (9.5, 9.0))])
            .collect();
        let h = lay(&s, &[]).height_at(Point::new(2.5, 0.0));
        assert!(h > 3.0 * top() && h < 7.0 * top(), "{h}");
    }

    #[test]
    fn foam_and_pieces_lift_the_stitches() {
        let puff = lay(
            &[Strand {
                lift_mm: crate::strand::FOAM_MM,
                ..strand((0.0, 0.0), (5.0, 0.0))
            }],
            &[],
        );
        assert!(puff.height_at(Point::new(2.5, 0.0)) > crate::strand::FOAM_MM);
        let square = vec![vec![
            Point::new(-1.0, -1.0),
            Point::new(6.0, -1.0),
            Point::new(6.0, 1.0),
            Point::new(-1.0, 1.0),
        ]];
        let on = lay(&[strand((0.0, 0.0), (5.0, 0.0))], &[(square, Rgb(0, 0, 200))]);
        assert!((on.height_at(Point::new(2.5, 0.0)) - (PIECE_MM + top())).abs() < 0.02);
        assert!((on.height_at(Point::new(2.5, 0.8)) - PIECE_MM).abs() < 1e-6);
    }

    #[test]
    fn tiles_give_the_same_pixels_in_each_area() {
        let s: Vec<Strand> = (0..40)
            .map(|k| strand((0.0, 0.3 * k as f64), (30.0, 0.3 * k as f64 + 1.0)))
            .collect();
        let laid = lay(&s, &[]);
        // 30 mm at 20 px/mm: 600 pixels, three tiles across.
        let big = laid.surface(BBox::new(Point::new(0.0, 0.0), Point::new(30.0, 12.0)), 20.0);
        assert_eq!((big.width, big.tiles_x, big.tiles.len()), (600, 3, 3));
        // A part of it, from a pixel corner: the same heights.
        let part = laid.surface(BBox::new(Point::new(12.0, 2.0), Point::new(14.0, 4.0)), 20.0);
        for y in 0..40 {
            for x in 0..40 {
                assert_eq!(part.height(x, y), big.height(x + 240, y + 40), "{x} {y}");
            }
        }
        // The same again: the same bytes.
        assert_eq!(big, laid.surface(big.area, 20.0));
        assert!(big.bytes() == 600 * 240 * 20);
    }

    #[test]
    fn a_zoom_level_has_the_means_of_the_level_under_it() {
        let s: Vec<Strand> = (0..10)
            .map(|k| strand((0.0, 0.5 * k as f64), (6.0, 0.5 * k as f64 + 0.3)))
            .collect();
        let fine = lay(&s, &[]).surface(BBox::new(Point::new(0.0, 0.0), Point::new(6.0, 5.0)), 16.0);
        let half = fine.half();
        assert_eq!((half.width, half.height, half.px_per_mm), (48, 40, 8.0));
        let (t, i) = half.at(10, 10);
        let four: Vec<(&Tile, usize)> = [(20, 20), (21, 20), (20, 21), (21, 21)]
            .iter()
            .map(|&(x, y)| fine.at(x, y))
            .collect();
        let mean_h = four.iter().map(|(t, i)| t.height[*i]).sum::<f32>() / 4.0;
        assert!((t.height[i] - mean_h).abs() < 1e-6);
        let mean_cover = four.iter().map(|(t, i)| f64::from(t.cover[*i])).sum::<f64>() / 4.0;
        assert!((f64::from(t.cover[i]) - mean_cover).abs() <= 0.5);
        // All stitches go one way: the direction keeps its length.
        let a = t.axis[i];
        let len = (f64::from(a[0]).powi(2) + f64::from(a[1]).powi(2)).sqrt();
        assert!(len > 120.0, "{a:?}");
        // A smaller zoom from a surface that is too small for its detail.
        let small = lay(&s, &[]).surface(BBox::new(Point::new(0.0, 0.0), Point::new(6.0, 5.0)), 2.0);
        assert_eq!((small.width, small.height, small.px_per_mm), (12, 10, 2.0));
    }

    /// The strands of `parts` one after the other, with their records.
    fn design(parts: &[&[Strand]]) -> Vec<Strand> {
        parts
            .concat()
            .into_iter()
            .enumerate()
            .map(|(i, s)| Strand { record: i as u32, ..s })
            .collect()
    }

    /// A fill, a satin on it and runs far from them.
    fn objects() -> (Vec<Strand>, Vec<Strand>, Vec<Strand>) {
        let fill: Vec<Strand> = (0..30)
            .flat_map(|k| {
                let y = 0.4 * k as f64;
                [strand((0.0, y), (8.0, y + 0.1)), strand((8.0, y + 0.2), (0.0, y + 0.3))]
            })
            .collect();
        let satin: Vec<Strand> = (0..40)
            .map(|k| {
                let x = 1.0 + 0.15 * k as f64;
                if k % 2 == 0 {
                    strand((x, 2.0), (x + 0.15, 7.0))
                } else {
                    strand((x, 7.0), (x + 0.15, 2.0))
                }
            })
            .collect();
        let far: Vec<Strand> = (0..10)
            .map(|k| strand((40.0 + k as f64, 40.0), (41.0 + k as f64, 40.5)))
            .collect();
        (fill, satin, far)
    }

    #[test]
    fn a_relay_gives_the_same_as_a_new_lay() {
        let (fill, satin, far) = objects();
        let moved: Vec<Strand> = satin
            .iter()
            .map(|s| Strand {
                a: Point::new(s.a.x + 0.5, s.a.y + 0.5),
                b: Point::new(s.b.x + 0.5, s.b.y + 0.5),
                ..*s
            })
            .collect();
        let piece = vec![vec![
            Point::new(-1.0, -1.0),
            Point::new(9.0, -1.0),
            Point::new(9.0, 13.0),
            Point::new(-1.0, 13.0),
        ]];
        for areas in [vec![], vec![(piece, Rgb(0, 0, 200))]] {
            let old = lay(&design(&[&fill, &satin, &far]), &areas);
            for new in [
                design(&[&fill, &moved, &far]),
                design(&[&fill, &far]),
                design(&[&far[..2], &fill, &satin, &far]),
                design(&[&fill[..20], &satin, &far]),
                design(&[&fill, &satin, &far, &moved]),
                design(&[&fill, &satin]),
                design(&[&satin, &far]),
                design(&[&far]),
            ] {
                let (re, _) = relay(&old, &new, &areas);
                assert!(re == lay(&new, &areas), "{} strands", new.len());
            }
        }
        // Where it changed: the satin, not the runs far from it.
        let old = lay(&design(&[&fill, &satin, &far]), &[]);
        let (_, changes) = relay(&old, &design(&[&fill, &moved, &far]), &[]);
        let at = |x: f64, y: f64| BBox::new(Point::new(x, y), Point::new(x + 0.5, y + 0.5));
        assert!(changes.touches(at(3.0, 4.0)) && !changes.touches(at(45.0, 40.0)));
        // Nothing changed: no change.
        let (same, changes) = relay(&old, &design(&[&fill, &satin, &far]), &[]);
        assert!(changes.is_empty() && same == old);
        // Another cell (a much larger design): all of it changed.
        let huge = [strand((0.0, 0.0), (900.0, 900.0))];
        assert!(relay(&old, &design(&[&fill, &huge]), &[]).1.is_everywhere());
    }

    #[test]
    fn the_pieces_are_the_cells_inside_the_rings() {
        let star: Vec<Point> = (0..10)
            .map(|k| {
                let a = std::f64::consts::TAU * k as f64 / 10.0;
                let r = if k % 2 == 0 { 3.0 } else { 1.2 };
                Point::new(r * emb_geom::math::cos(a), r * emb_geom::math::sin(a))
            })
            .collect();
        let hole: Vec<Point> = vec![
            Point::new(-0.5, -0.5),
            Point::new(0.5, -0.5),
            Point::new(0.5, 0.5),
            Point::new(-0.5, 0.5),
        ];
        let rings = vec![star, hole];
        let mut g = Grid::new(BBox::new(Point::new(-4.0, -4.0), Point::new(4.0, 4.0)), CELL_MM);
        g.piece(&rings, 1.0);
        for y in 0..g.h {
            for x in 0..g.w {
                let want = inside(&rings, g.center(x, y));
                assert_eq!(g.height[y * g.w + x] > 0.0, want, "{x} {y}");
            }
        }
    }

    /// All values of the pixel (`x`, `y`).
    #[allow(clippy::type_complexity)]
    fn pixel(s: &Surface, x: u32, y: u32) -> (f32, [i8; 3], [u8; 3], [i8; 2], [u8; 8]) {
        let (t, i) = s.at(x, y);
        (
            t.height[i],
            t.normal[i],
            t.color[i],
            t.axis[i],
            [
                t.material[i],
                t.cover[i],
                t.open[i],
                t.ply[i] as u8,
                t.shadow[i],
                t.diffuse[i],
                t.fabric_shade[i],
                0,
            ],
        )
    }

    #[test]
    fn a_block_has_the_pixels_of_each_other_block() {
        let (fill, satin, far) = objects();
        let laid = lay(&design(&[&fill, &satin, &far]), &[]);
        let l = Lighting::default();
        let all = |_: u32| true;
        // At 8 and 32 pixels for each mm, and at 1 (made in parts).
        for (ppm, x0, y0, w, h) in [(8.0, -8, -4, 80, 104), (32.0, -16, -8, 300, 200), (1.0, -3, -3, 60, 50)] {
            let big = laid.block(ppm, x0, y0, w, h, &l, &all).expect("stitches");
            let (px, py, pw, ph) = (w / 10, h / 10, w / 3, h / 2);
            let part = laid
                .block(ppm, x0 + i64::from(px), y0 + i64::from(py), pw, ph, &l, &all)
                .expect("stitches");
            for y in 0..ph {
                for x in 0..pw {
                    assert_eq!(pixel(&part, x, y), pixel(&big, x + px, y + py), "{ppm}: {x} {y}");
                }
            }
            assert!((part.area.min.x - (x0 + i64::from(px)) as f64 / ppm).abs() < 1e-12);
        }
        // Far from the stitches, or with no stitch shown: nothing.
        assert!(laid.block(8.0, 4000, 4000, 16, 16, &l, &all).is_none());
        assert!(laid.block(8.0, -8, -4, 80, 104, &l, &|_| false).is_none());
        // Only the shown strands: the satin alone covers less.
        let first = fill.len() as u32;
        let satin_only = laid
            .block(8.0, -8, -4, 80, 104, &l, &|r| (first..first + 40).contains(&r))
            .expect("the satin");
        let all_of_it = laid.block(8.0, -8, -4, 80, 104, &l, &all).expect("stitches");
        let covered = |s: &Surface| s.tiles.iter().flat_map(|t| t.cover.iter()).filter(|c| **c > 0).count();
        assert!(covered(&satin_only) < covered(&all_of_it));
    }

    /// The time of the steps of a block of 258 pixels of the synthetic design
    /// of 100,000 stitches, on one processor (the place of most of the
    /// stitches of each zoom). Run with `cargo test --release -p emb-render
    /// -- --ignored block_steps --nocapture`.
    #[test]
    #[ignore]
    fn block_steps() {
        let s = crate::strand::strands(&crate::stress::make(100_000), &[]);
        let t = std::time::Instant::now();
        let laid = lay(&s, &[]);
        println!("lay {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        let l = Lighting::default();
        let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1000.0;
        for ppm in [4.0, 16.0, 32.0] {
            // The block with the most strands near its middle.
            let b = laid.bounds();
            let side = 258.0 / ppm;
            let mut best = (0, Point::ZERO);
            let mut y = b.min.y;
            while y < b.max.y {
                let mut x = b.min.x;
                while x < b.max.x {
                    let n = laid
                        .index
                        .near(BBox::new(Point::new(x, y), Point::new(x + side, y + side)))
                        .len();
                    if n > best.0 {
                        best = (n, Point::new(x, y));
                    }
                    x += side;
                }
                y += side;
            }
            let (x0, y0) = ((best.1.x * ppm) as i64, (best.1.y * ppm) as i64);
            let area = BBox::new(best.1, Point::new(best.1.x + side, best.1.y + side));
            let ids = laid.index.near(area);
            let f = (16.0 / ppm).max(1.0) as i64;
            let w = (258 * f) as u32;
            let t = std::time::Instant::now();
            let mut sf = laid.raster(Point::ZERO, (x0 * f, y0 * f), ppm * f as f64, w, w, &l, Some(&ids), 1);
            let raster = ms(t);
            let t = std::time::Instant::now();
            sf.finish(1);
            let finish = ms(t);
            let t = std::time::Instant::now();
            let mut k = f;
            while k > 1 {
                sf = sf.half_with(1);
                k /= 2;
            }
            let half = ms(t);
            let t = std::time::Instant::now();
            crate::sheen::overlay(&sf, &l);
            let light = ms(t);
            let t = std::time::Instant::now();
            laid.block(ppm, x0, y0, 258, 258, &l, &|_| true).expect("stitches");
            println!(
                "{ppm} px/mm: {} strands, {} pixels of the detail; the stitches and the finish {raster:.0} ms \
                 (the finish {finish:.0} ms), halved {half:.0} ms, light {light:.0} ms; the block {:.0} ms",
                ids.len(),
                w * w,
                ms(t)
            );
        }
    }

    /// The time of a relay after a change of 2,000 stitches in 200,000. Run
    /// with `cargo test --release -p emb-render -- --ignored relay_time
    /// --nocapture`.
    #[test]
    #[ignore]
    fn relay_time() {
        let s = crate::strand::strands(&crate::stress::make(200_000), &[]);
        let t = std::time::Instant::now();
        let old = lay(&s, &[]);
        println!("lay {:.0} ms, {} strands", t.elapsed().as_secs_f64() * 1000.0, s.len());
        let mut new = s.clone();
        for x in &mut new[80_000..82_000] {
            x.a.x += 0.3;
            x.b.x += 0.3;
        }
        let t = std::time::Instant::now();
        let (re, changes) = relay(&old, &new, &[]);
        println!(
            "relay {:.0} ms, {} squares",
            t.elapsed().as_secs_f64() * 1000.0,
            changes.squares.len()
        );
        let t = std::time::Instant::now();
        let b: Vec<BBox> = new.iter().map(strand_box).collect();
        println!("boxes {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
        let t = std::time::Instant::now();
        let ix = Index::new(b.iter().copied(), re.bounds(), QUERY_MM);
        println!("index 1 mm {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
        let t = std::time::Instant::now();
        let ix2 = Index::new(b.iter().copied(), re.bounds(), INDEX_MM);
        println!(
            "index 2 mm {:.1} ms ({} {})",
            t.elapsed().as_secs_f64() * 1000.0,
            ix.lists.len(),
            ix2.lists.len()
        );
        let t = std::time::Instant::now();
        let k: u64 = new.iter().map(key).fold(0, |a, b| a ^ b);
        println!("keys {:.1} ms {k}", t.elapsed().as_secs_f64() * 1000.0);
        let t = std::time::Instant::now();
        let g = re.grid.clone();
        println!(
            "grid clone {:.1} ms ({} cells)",
            t.elapsed().as_secs_f64() * 1000.0,
            g.height.len()
        );
    }

    #[test]
    fn changes_are_squares() {
        let mut c = Changes::default();
        assert!(c.is_empty());
        c.add(BBox::new(Point::new(1.0, 1.0), Point::new(5.0, 3.0)));
        let at = |x: f64, y: f64| BBox::new(Point::new(x, y), Point::new(x + 0.1, y + 0.1));
        assert!(c.touches(at(4.9, 2.9)) && c.touches(at(5.5, 1.0)) && !c.touches(at(6.5, 1.0)));
        assert!(c.touches(BBox::new(Point::new(-100.0, -100.0), Point::new(100.0, 100.0))));
        let mut d = Changes::default();
        d.merge(&c);
        assert_eq!(d, c);
        d.merge(&Changes::everywhere());
        assert!(d.is_everywhere() && d.touches(at(500.0, 500.0)));
    }

    #[test]
    fn the_gaps_are_darker() {
        // Two stitches with a gap between them.
        let s = [strand((0.0, 0.0), (6.0, 0.0)), strand((6.0, 0.5), (0.0, 0.5))];
        let surf = lay(&s, &[]).surface(BBox::new(Point::new(0.0, -1.0), Point::new(6.0, 1.5)), 32.0);
        let open = |y_mm: f64| {
            let (t, i) = surf.at(96, ((y_mm + 1.0) * 32.0) as u32);
            t.open[i]
        };
        assert!(open(0.25) < open(0.0), "{} {}", open(0.25), open(0.0));
        assert_eq!(open(-0.9), 255);
    }
}
