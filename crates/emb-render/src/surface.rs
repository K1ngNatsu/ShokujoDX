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
//! [`light`] makes a picture of a surface with the light of the 3D thread
//! look of today on the directions of the surface (the first light of the
//! milestone). The light of Milestone 4 replaces it.

use std::sync::atomic::{AtomicUsize, Ordering};

use emb_geom::{BBox, Point};
use emb_model::{Rgb, ThreadKind};

use crate::raster::{Areas, Picture};
use crate::strand::Strand;
use crate::thread3d::{Lighting, Weave, fabric_value, lit, shine_of};

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
/// number of layers of thread in each cell.
#[derive(Debug, Clone)]
struct Grid {
    min: Point,
    cell: f64,
    w: usize,
    h: usize,
    height: Vec<f32>,
    layers: Vec<u8>,
}

impl Grid {
    fn new(b: BBox, cell: f64) -> Grid {
        let w = ((b.width() / cell).ceil() as usize).max(1);
        let h = ((b.height() / cell).ceil() as usize).max(1);
        Grid {
            min: b.min,
            cell,
            w,
            h,
            height: vec![0.0; w * h],
            layers: vec![0; w * h],
        }
    }

    fn index(&self, p: Point) -> Option<usize> {
        let (x, y) = ((p.x - self.min.x) / self.cell, (p.y - self.min.y) / self.cell);
        (x >= 0.0 && y >= 0.0 && (x as usize) < self.w && (y as usize) < self.h)
            .then(|| y as usize * self.w + x as usize)
    }

    fn height_at(&self, p: Point) -> f64 {
        self.index(p).map_or(0.0, |i| f64::from(self.height[i]))
    }

    fn layers_at(&self, p: Point) -> u8 {
        self.index(p).map_or(0, |i| self.layers[i])
    }

    /// The range of cells (columns or rows) between two coordinates.
    fn span(&self, lo: f64, hi: f64, min: f64, n: usize) -> std::ops::Range<usize> {
        let a = ((lo - min) / self.cell).floor().max(0.0) as usize;
        let b = (((hi - min) / self.cell).ceil().max(0.0) as usize).min(n);
        a.min(b)..b
    }

    /// The center of the cell (`x`, `y`).
    fn center(&self, x: usize, y: usize) -> Point {
        Point::new(
            self.min.x + (x as f64 + 0.5) * self.cell,
            self.min.y + (y as f64 + 0.5) * self.cell,
        )
    }

    /// A piece of fabric: the rings (even-odd) at the height `h`.
    fn piece(&mut self, rings: &[Vec<Point>], h: f32) {
        for y in 0..self.h {
            let yc = self.center(0, y).y;
            for x in 0..self.w {
                if inside(rings, Point::new(self.center(x, 0).x, yc)) {
                    let i = y * self.w + x;
                    self.height[i] = self.height[i].max(h);
                }
            }
        }
    }

    /// Lays a strand on the grid.
    fn stamp(&mut self, s: &LaidStrand, z: &[f32]) {
        let b = s.bbox;
        let (xs, ys) = (
            self.span(b.min.x, b.max.x, self.min.x, self.w),
            self.span(b.min.y, b.max.y, self.min.y, self.h),
        );
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
#[derive(Debug, Clone, Copy)]
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
    /// The direction of the thread, 0 to 255 for 0° to 180°.
    angle: u8,
    /// The heights of its middle line: `z[start..start + n]`, `step` mm apart.
    start: usize,
    n: usize,
    step: f64,
    /// The area that it covers, with its width.
    bbox: BBox,
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
#[derive(Debug, Clone)]
pub struct Laid {
    strands: Vec<LaidStrand>,
    z: Vec<f32>,
    pieces: Vec<(Vec<Vec<Point>>, Rgb)>,
    grid: Grid,
}

/// Lays `strands` in sew order on the pieces of fabric `areas` (appliqué
/// fabric and stones, under all stitches).
pub fn lay(strands: &[Strand], areas: &Areas) -> Laid {
    let points = strands
        .iter()
        .flat_map(|s| [s.a, s.b])
        .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied()));
    let b = BBox::from_points(points)
        .unwrap_or(BBox::new(Point::ZERO, Point::ZERO))
        .expand(1.0);
    let cell = CELL_MM.max((b.width() * b.height() / MAX_CELLS).sqrt());
    let mut grid = Grid::new(b, cell);
    for (rings, _) in areas {
        grid.piece(rings, PIECE_MM as f32);
    }
    let mut laid: Vec<LaidStrand> = Vec::with_capacity(strands.len());
    let mut z: Vec<f32> = Vec::new();
    let mut under: Vec<f64> = Vec::new();
    let mut ts: Vec<f64> = Vec::new();
    let mut line: Vec<f64> = Vec::new();
    // The stitch just before is laid on the grid after the next one.
    let mut waiting: Option<usize> = None;
    for s in strands {
        let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            continue;
        }
        let dir = (dx / len, dy / len);
        let width = emb_model::thread::width_mm(s.weight);
        let half = width / 2.0;
        let n = ((len / cell).ceil() as usize).max(1) + 1;
        let step = len / (n - 1) as f64;
        // What is under the middle line of the stitch; the needle points
        // are on the fabric, and the foam is a floor. A thread beside it
        // (less than half a width into it, as the parts of a split satin
        // or the rows of a dense fill) is not under it: the two lie side
        // by side.
        under.clear();
        ts.clear();
        let mut layers = 0u8;
        for k in 0..n {
            let t = step * k as f64;
            let p = Point::new(s.a.x + dir.0 * t, s.a.y + dir.1 * t);
            let mut h = grid.height_at(p);
            if t.min(len - t) >= FOAM_EDGE_MM.min(len / 2.0) {
                h = h.max(s.lift_mm);
            }
            ts.push(t);
            under.push(if k == 0 || k == n - 1 { 0.0 } else { h });
            layers = layers.max(grid.layers_at(p));
        }
        // Thread under tension: a taut line over what is under it.
        taut(&ts, &under, &mut line);
        let thick = THICKNESS * width / (1.0 + PRESS * f64::from(layers));
        let start = z.len();
        for (k, u) in line.iter().enumerate() {
            let t = ts[k];
            z.push((u + thick * along(t.min(len - t), RISE_MM)) as f32);
        }
        let angle = {
            let a = emb_geom::math::atan2(dir.1, dir.0).rem_euclid(std::f64::consts::PI);
            ((a / std::f64::consts::PI * 256.0) as u32).min(255) as u8
        };
        laid.push(LaidStrand {
            a: s.a,
            dir,
            len,
            half,
            thick,
            color: s.color,
            material: thread_material(s.kind),
            angle,
            start,
            n,
            step,
            bbox: BBox::from_points([s.a, s.b]).expect("two points").expand(half),
        });
        if let Some(w) = waiting.replace(laid.len() - 1) {
            let ls = laid[w];
            grid.stamp(&ls, &z);
        }
    }
    if let Some(w) = waiting {
        let ls = laid[w];
        grid.stamp(&ls, &z);
    }
    Laid {
        strands: laid,
        z,
        pieces: areas.to_vec(),
        grid,
    }
}

impl Laid {
    /// The height of the top at `p`, mm (from the grid of 0.1 mm).
    pub fn height_at(&self, p: Point) -> f64 {
        self.grid.height_at(p)
    }

    /// The heights of the cells in `b`.
    fn heights_in(&self, b: BBox) -> impl Iterator<Item = f64> + '_ {
        let g = &self.grid;
        let (xs, ys) = (
            g.span(b.min.x, b.max.x, g.min.x, g.w),
            g.span(b.min.y, b.max.y, g.min.y, g.h),
        );
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
    }

    /// The surface of the area `area` at `px_per_mm` (step 2).
    pub fn surface(&self, area: BBox, px_per_mm: f64) -> Surface {
        let width = ((area.width() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let height = ((area.height() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let (tx, ty) = (width.div_ceil(TILE), height.div_ceil(TILE));
        // The stitches of each tile, in sew order.
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); (tx * ty) as usize];
        let to_px = |v: f64, min: f64| (v - min) * px_per_mm;
        for (i, s) in self.strands.iter().enumerate() {
            let b = s.bbox.expand(1.0 / px_per_mm);
            let (x0, x1) = (to_px(b.min.x, area.min.x), to_px(b.max.x, area.min.x));
            let (y0, y1) = (to_px(b.min.y, area.min.y), to_px(b.max.y, area.min.y));
            if x1 < 0.0 || y1 < 0.0 || x0 >= f64::from(width) || y0 >= f64::from(height) {
                continue;
            }
            let t = f64::from(TILE);
            let (cx0, cx1) = ((x0.max(0.0) / t) as u32, ((x1 / t) as u32).min(tx - 1));
            let (cy0, cy1) = ((y0.max(0.0) / t) as u32, ((y1 / t) as u32).min(ty - 1));
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    lists[(cy * tx + cx) as usize].push(i as u32);
                }
            }
        }
        let count = (tx * ty) as usize;
        let next = AtomicUsize::new(0);
        let workers = std::thread::available_parallelism().map_or(1, usize::from).min(count);
        let mut made: Vec<(usize, Tile)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let k = next.fetch_add(1, Ordering::Relaxed);
                            if k >= count {
                                break out;
                            }
                            let (cx, cy) = (k as u32 % tx, k as u32 / tx);
                            let (x0, y0) = (cx * TILE, cy * TILE);
                            let (w, h) = (TILE.min(width - x0), TILE.min(height - y0));
                            out.push((k, self.tile(area.min, px_per_mm, x0, y0, w, h, &lists[k])));
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("a tile thread does not stop with an error"))
                .collect()
        });
        made.sort_by_key(|(k, _)| *k);
        Surface {
            area,
            px_per_mm,
            width,
            height,
            tiles_x: tx,
            tiles: made.into_iter().map(|(_, t)| t).collect(),
        }
    }

    /// One tile: the pixels `x0..x0 + w`, `y0..y0 + h` of the surface.
    #[allow(clippy::too_many_arguments)]
    fn tile(&self, min: Point, ppm: f64, x0: u32, y0: u32, w: u32, h: u32, ids: &[u32]) -> Tile {
        let n = (w * h) as usize;
        let mut t = Tile {
            x0,
            y0,
            w,
            h,
            height: vec![0.0; n],
            color: vec![[0; 3]; n],
            angle: vec![0; n],
            material: vec![material::FABRIC; n],
            cover: vec![0; n],
        };
        let center =
            |px: u32, py: u32| Point::new(min.x + (f64::from(px) + 0.5) / ppm, min.y + (f64::from(py) + 0.5) / ppm);
        for (rings, c) in &self.pieces {
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
            let r_px = s.half * ppm;
            let b = s.bbox.expand(1.0 / ppm);
            let px0 = (((b.min.x - min.x) * ppm).floor().max(0.0) as u32).max(x0) - x0;
            let px1 = ((((b.max.x - min.x) * ppm).ceil().max(0.0)) as u32).min(x0 + w).max(x0) - x0;
            let py0 = (((b.min.y - min.y) * ppm).floor().max(0.0) as u32).max(y0) - y0;
            let py1 = ((((b.max.y - min.y) * ppm).ceil().max(0.0)) as u32).min(y0 + h).max(y0) - y0;
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
                    let across = ((ry * s.dir.0 - rx * s.dir.1).abs() / s.half).min(1.0);
                    let zc = s.middle(z, along);
                    let hz = zc * ((1.0 - across * across).sqrt() as f32);
                    let k = (py * w + px) as usize;
                    // The union of the threads in the pixel (each one covers
                    // its part of the part that the others leave open).
                    let open = (1.0 - f64::from(t.cover[k]) / 255.0) * (1.0 - cover);
                    t.cover[k] = ((1.0 - open) * 255.0).round() as u8;
                    if hz >= t.height[k] {
                        t.height[k] = hz;
                        t.color[k] = [s.color.0, s.color.1, s.color.2];
                        t.angle[k] = s.angle;
                        t.material[k] = s.material;
                    }
                }
            }
        }
        t
    }
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
    /// The color of the thread or the piece on top.
    pub color: Vec<[u8; 3]>,
    /// The direction of the thread on top, 0 to 255 for 0° to 180°.
    pub angle: Vec<u8>,
    /// What is on top ([`material`]).
    pub material: Vec<u8>,
    /// The part of the pixel that threads or pieces cover, 0 to 255 (the
    /// union of the parts of each thread).
    pub cover: Vec<u8>,
}

/// The surface of an area at a zoom: tiles of [`TILE`] pixels, row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub area: BBox,
    pub px_per_mm: f64,
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles: Vec<Tile>,
}

impl Surface {
    /// The tile of the pixel (`x`, `y`) and the index of the pixel in it.
    fn at(&self, x: u32, y: u32) -> (&Tile, usize) {
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

    /// The size of the data, bytes (10 for each pixel).
    pub fn bytes(&self) -> usize {
        self.tiles.iter().map(|t| t.height.len() * 10).sum()
    }
}

/// The picture of a surface with the light of today on the directions of
/// the surface (ADR 0124): each pixel of thread has the light of its
/// normal and the highlight of its kind; the threads mix with the fabric
/// in linear light where they cover a part of the pixel. `fabric`: the
/// color and the weave under the threads, else a transparent background.
pub fn light(s: &Surface, lighting: &Lighting, fabric: Option<(Rgb, Weave)>) -> Picture {
    let (w, h) = (s.width as usize, s.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let d = 2.0 / s.px_per_mm;
    let rows_per = h
        .div_ceil(std::thread::available_parallelism().map_or(1, usize::from))
        .max(1);
    std::thread::scope(|scope| {
        for (band, chunk) in rgba.chunks_mut(rows_per * w * 4).enumerate() {
            scope.spawn(move || {
                for (r, row) in chunk.chunks_mut(w * 4).enumerate() {
                    let y = (band * rows_per + r) as i64;
                    for x in 0..w {
                        let px = &mut row[x * 4..x * 4 + 4];
                        let xi = x as i64;
                        let (t, i) = s.at(x as u32, y as u32);
                        let under = fabric.map(|(c, weave)| {
                            let (mx, my) = (
                                s.area.min.x + (x as f64 + 0.5) / s.px_per_mm,
                                s.area.min.y + (y as f64 + 0.5) / s.px_per_mm,
                            );
                            lit(c, (fabric_value(weave, mx, my), 0.0))
                        });
                        let cover = f64::from(t.cover[i]) / 255.0;
                        if cover <= 0.0 {
                            match under {
                                Some(c) => px.copy_from_slice(&[c[0], c[1], c[2], 255]),
                                None => px.copy_from_slice(&[0, 0, 0, 0]),
                            }
                            continue;
                        }
                        let dzdx = f64::from(s.height(xi + 1, y) - s.height(xi - 1, y)) / d;
                        let dzdy = f64::from(s.height(xi, y + 1) - s.height(xi, y - 1)) / d;
                        let len = (dzdx * dzdx + dzdy * dzdy + 1.0).sqrt();
                        let n = [-dzdx / len, -dzdy / len, 1.0 / len];
                        let (f, shine) = lighting.shade_normal(n);
                        let shine = kind_of(t.material[i]).map_or(0.0, |k| shine * shine_of(k));
                        let [r, g, b] = t.color[i];
                        let c = lit(Rgb(r, g, b), (f, shine));
                        match under {
                            Some(u) => {
                                let m = |k: usize| crate::light::mix(c[k], u[k], cover);
                                px.copy_from_slice(&[m(0), m(1), m(2), 255]);
                            }
                            None => {
                                px.copy_from_slice(&[c[0], c[1], c[2], (cover * 255.0).round() as u8]);
                            }
                        }
                    }
                }
            });
        }
    });
    Picture {
        width: s.width,
        height: s.height,
        rgba,
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
        assert!(big.bytes() == 600 * 240 * 10);
    }

    #[test]
    fn the_light_comes_from_the_top_left_and_matte_thread_has_no_highlight() {
        let one = |kind| {
            let laid = lay(
                &[Strand {
                    kind,
                    ..strand((0.0, 0.0), (10.0, 0.0))
                }],
                &[],
            );
            let s = laid.surface(BBox::new(Point::new(0.0, -1.0), Point::new(10.0, 1.0)), 40.0);
            light(&s, &Lighting::default(), Some((Rgb(250, 250, 250), Weave::Smooth)))
        };
        let p = one(ThreadKind::Polyester);
        let at = |p: &Picture, x: u32, y: u32| p.rgba[((y * p.width + x) * 4) as usize];
        // The thread is from y = 32 to 48 (pixels); its upper edge faces the light.
        assert!(
            at(&p, 200, 35) > at(&p, 200, 45) + 30,
            "{} {}",
            at(&p, 200, 35),
            at(&p, 200, 45)
        );
        assert_eq!(at(&p, 200, 5), at(&one(ThreadKind::Cotton), 200, 5), "the fabric");
        let green = |p: &Picture| p.rgba.chunks(4).map(|c| c[1]).filter(|&g| g < 200).max().unwrap();
        assert!(green(&p) > green(&one(ThreadKind::Cotton)) + 20);
    }
}
