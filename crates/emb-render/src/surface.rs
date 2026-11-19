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
        let axis = {
            let a2 = 2.0 * emb_geom::math::atan2(dir.1, dir.0);
            [emb_geom::math::cos(a2), emb_geom::math::sin(a2)].map(|v| (v * 127.0).round() as i8)
        };
        laid.push(LaidStrand {
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
            px_per_mm * f64::from(1u32 << k),
            width << k,
            height << k,
            lighting,
        );
        for _ in 0..k {
            s = s.half();
        }
        s
    }

    /// The surface at `ppm` with the size `width` × `height` pixels from
    /// `min`, with no halving.
    fn raster(&self, min: Point, ppm: f64, width: u32, height: u32, lighting: &Lighting) -> Surface {
        let (tx, ty) = (width.div_ceil(TILE), height.div_ceil(TILE));
        // The stitches of each tile, in sew order.
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); (tx * ty) as usize];
        let to_px = |v: f64, m: f64| (v - m) * ppm;
        for (i, s) in self.strands.iter().enumerate() {
            let b = s.bbox.expand(1.0 / ppm);
            let (x0, x1) = (to_px(b.min.x, min.x), to_px(b.max.x, min.x));
            let (y0, y1) = (to_px(b.min.y, min.y), to_px(b.max.y, min.y));
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
        let tiles = in_parallel((tx * ty) as usize, |k| {
            let (cx, cy) = (k as u32 % tx, k as u32 / tx);
            let (x0, y0) = (cx * TILE, cy * TILE);
            let (w, h) = (TILE.min(width - x0), TILE.min(height - y0));
            self.tile(min, ppm, x0, y0, w, h, &lists[k])
        });
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
        s.finish();
        s
    }

    /// One tile: the pixels `x0..x0 + w`, `y0..y0 + h` of the surface.
    #[allow(clippy::too_many_arguments)]
    fn tile(&self, min: Point, ppm: f64, x0: u32, y0: u32, w: u32, h: u32, ids: &[u32]) -> Tile {
        let mut t = Tile::new(x0, y0, w, h);
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
            // Small changes of each thread (ADR 0125): its width and its
            // brightness, from its place.
            let half = s.half * (1.0 + VARY_WIDTH * f64::from(s.vary[0]));
            let shade = 1.0 + VARY_BRIGHTNESS * f64::from(s.vary[1]);
            let color =
                [s.color.0, s.color.1, s.color.2].map(|c| (f64::from(c) * shade).round().clamp(0.0, 255.0) as u8);
            let r_px = half * ppm;
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
/// A part this far above the line to the light makes a full shadow, mm (a
/// softer shadow for less).
const SHADOW_SOFT_MM: f64 = 0.08;

/// The shadow at the pixel (`x`, `y`) of heights `at` (`w` × `h` pixels at
/// `ppm`; the highest point `top`) in the light `l`: 0 in the light, 1 in
/// full shadow. The line from the pixel to the light goes up at the
/// height of the light; a part above it puts the pixel in its shadow.
#[allow(clippy::too_many_arguments)]
pub(crate) fn shadow_at(
    at: &impl Fn(i64, i64) -> f64,
    w: i64,
    h: i64,
    x: i64,
    y: i64,
    l: &Lighting,
    ppm: f64,
    top: f64,
) -> f64 {
    let v = l.vector();
    let flat = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if flat < 1e-6 || l.strength <= 0.0 {
        return 0.0;
    }
    let (ux, uy) = (v[0] / flat, v[1] / flat);
    let rise = v[2] / flat / ppm;
    let h0 = at(x, y);
    let steps = ((top - h0) / rise).min(SHADOW_MAX_MM * ppm).ceil().max(0.0) as i64;
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

/// Runs `f` for `0..count` on all processors and gives the results in
/// order.
fn in_parallel<T: Send>(count: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(count)
        .max(1);
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

impl Surface {
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
    /// tiles), on all processors.
    fn finish(&mut self) {
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
        let (light, ppm) = (self.light, self.px_per_mm);
        let per = self
            .tiles
            .len()
            .div_ceil(std::thread::available_parallelism().map_or(1, usize::from))
            .max(1);
        std::thread::scope(|scope| {
            for chunk in self.tiles.chunks_mut(per) {
                scope.spawn(move || {
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
                                let dark = shadow_at(&at, w as i64, h as i64, x, y, &light, ppm, top);
                                t.shadow[k] = (dark * 255.0).round() as u8;
                                let shade = open * (1.0 - crate::sheen::SHADOW_DARK * dark);
                                t.fabric_shade[k] = (shade * 255.0).round() as u8;
                                let n = [-dzdx / len, -dzdy / len, 1.0 / len];
                                let lit = crate::sheen::diffuse(n, light.vector());
                                t.diffuse[k] = (lit / 2.0 * 255.0).round().clamp(0.0, 255.0) as u8;
                            }
                        }
                    }
                });
            }
        });
    }

    /// The zoom level with half the pixels on each side: each pixel has the
    /// mean height, slope, gaps and plies of the four pixels under it, the
    /// mean color of their threads (in linear light), the part that they
    /// cover, the mean of their thread directions (shorter where they
    /// differ) and the material that covers most of them (ADR 0125).
    pub fn half(&self) -> Surface {
        let (width, height) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let (tx, ty) = (width.div_ceil(TILE), height.div_ceil(TILE));
        let tiles = in_parallel((tx * ty) as usize, |k| {
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
