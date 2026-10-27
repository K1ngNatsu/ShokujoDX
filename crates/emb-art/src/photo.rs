//! Photo stitch (P6 Milestone 8; ADR 0030): a picture (a photo, a
//! portrait) as stitches whose density follows the picture, not as flat
//! color areas. Our own methods:
//!
//! - **Rows** (one thread): parallel rows across the picture. Along each
//!   row the needle makes a zigzag: as wide as the row spacing where the
//!   picture is dark, a plain run where it is light. The stitches are
//!   shorter where the picture is dark, so dark areas get more thread.
//! - **Color rows**: the picture reduced to a few thread colors; for each
//!   color, rows (at its own angle) whose zigzag width is the part of that
//!   color round each point. The light colors sew first, the dark colors
//!   on top.
//! - **Scribble** (one thread): short random stitches from point to point;
//!   the points are denser where the picture is dark.
//!
//! Clear pixels (a removed background) get no stitches; a gap up to
//! [`JOIN_MM`] is sewn over with one stitch, a longer gap is a jump with a
//! trim (VP3 files have no jump without a trim). The result is deterministic (ADR 0003): the random-looking points come from
//! a hash of their cell.

use emb_geom::{BBox, Path, Point, Transform, math};
use emb_model::{Lab, ObjectKind, Rgb};
use image::RgbaImage;

use crate::reduce::{NONE, ReduceSpec, reduce};

/// A gap up to this is sewn over with one stitch, in mm.
pub const JOIN_MM: f64 = 2.0;
/// A needle point nearer than this to the one before is left out, in mm
/// (a stitch this short sews badly, and a machine file can make it a
/// stitch of length 0).
pub const MIN_STITCH_MM: f64 = 0.3;
/// The zigzag of a row is at most this part of the row spacing.
const MAX_FILL: f64 = 0.92;
/// Color rows leave out the places where their color is less than this part.
const MIN_PART: f64 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhotoStyle {
    #[default]
    Rows,
    ColorRows,
    Scribble,
}

impl PhotoStyle {
    pub const ALL: [PhotoStyle; 3] = [PhotoStyle::Rows, PhotoStyle::ColorRows, PhotoStyle::Scribble];

    pub fn name(self) -> &'static str {
        match self {
            PhotoStyle::Rows => "Rows (one thread)",
            PhotoStyle::ColorRows => "Color rows",
            PhotoStyle::Scribble => "Scribble (one thread)",
        }
    }
}

/// Settings of [`photo`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoSpec {
    pub style: PhotoStyle,
    /// The distance between two rows (for Scribble: between two points), mm.
    pub row_mm: f64,
    /// The direction of the rows, degrees.
    pub angle_deg: f64,
    /// 1: as the picture. More: dark darker and light lighter.
    pub contrast: f64,
    /// -0.5 (darker) to 0.5 (lighter).
    pub brightness: f64,
    /// The thread of Rows and Scribble.
    pub thread: Rgb,
    /// The number of thread colors of Color rows.
    pub colors: usize,
    /// The shortest and the longest stitch along a row, mm.
    pub min_step_mm: f64,
    pub max_step_mm: f64,
}

impl Default for PhotoSpec {
    fn default() -> Self {
        PhotoSpec {
            style: PhotoStyle::Rows,
            row_mm: 1.2,
            angle_deg: 0.0,
            contrast: 1.2,
            brightness: 0.0,
            thread: Rgb(20, 20, 20),
            colors: 4,
            min_step_mm: 0.45,
            max_step_mm: 2.5,
        }
    }
}

/// One object of a photo stitch, with its thread color.
#[derive(Debug, Clone)]
pub struct PhotoObject {
    pub color: Rgb,
    pub kind: ObjectKind,
}

/// A value from 0 to 1 for each pixel, and which pixels are opaque.
struct Field {
    w: u32,
    h: u32,
    val: Vec<f32>,
    opaque: Vec<bool>,
    /// From mm to pixels.
    inv: Transform,
}

impl Field {
    /// The value at `p` (mm), bilinear; `None` where the picture is clear or ends.
    fn at(&self, p: Point) -> Option<f64> {
        let q = self.inv.apply(p);
        let (x, y) = (q.x - 0.5, q.y - 0.5);
        let (xi, yi) = (q.x.floor(), q.y.floor());
        if xi < 0.0 || yi < 0.0 || xi >= f64::from(self.w) || yi >= f64::from(self.h) {
            return None;
        }
        if !self.opaque[yi as usize * self.w as usize + xi as usize] {
            return None;
        }
        let cx = |i: f64| i.clamp(0.0, f64::from(self.w - 1)) as usize;
        let cy = |j: f64| j.clamp(0.0, f64::from(self.h - 1)) as usize;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let v = |i: f64, j: f64| f64::from(self.val[cy(j) * self.w as usize + cx(i)]);
        let top = v(x0, y0) * (1.0 - fx) + v(x0 + 1.0, y0) * fx;
        let bottom = v(x0, y0 + 1.0) * (1.0 - fx) + v(x0 + 1.0, y0 + 1.0) * fx;
        Some(top * (1.0 - fy) + bottom * fy)
    }
}

/// The photo stitch of `img` (pixels to mm by `to_mm`).
pub fn photo(img: &RgbaImage, to_mm: &Transform, spec: &PhotoSpec) -> Vec<PhotoObject> {
    let Some(inv) = to_mm.inverse() else { return Vec::new() };
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let opaque: Vec<bool> = img.pixels().map(|p| p[3] >= 128).collect();
    let corners = [
        (0.0, 0.0),
        (f64::from(w), 0.0),
        (0.0, f64::from(h)),
        (f64::from(w), f64::from(h)),
    ];
    let Some(bbox) = BBox::from_points(corners.map(|(x, y)| to_mm.apply(Point::new(x, y)))) else {
        return Vec::new();
    };
    let row = spec.row_mm.max(0.2);
    match spec.style {
        PhotoStyle::Rows | PhotoStyle::Scribble => {
            let val = img
                .pixels()
                .map(|p| {
                    let l = Lab::of(Rgb(p[0], p[1], p[2])).l / 100.0;
                    let d = ((1.0 - l) - 0.5) * spec.contrast + 0.5 - spec.brightness;
                    d.clamp(0.0, 1.0) as f32
                })
                .collect();
            let field = Field { w, h, val, opaque, inv };
            let kind = if spec.style == PhotoStyle::Rows {
                rows(&field, bbox, spec.angle_deg, row, spec, 0.0)
            } else {
                scribble(&field, bbox, row)
            };
            kind.map(|kind| PhotoObject {
                color: spec.thread,
                kind,
            })
            .into_iter()
            .collect()
        }
        PhotoStyle::ColorRows => {
            let r = reduce(
                img,
                &ReduceSpec {
                    colors: spec.colors,
                    min_speck_px: 0,
                    clean_edges: false,
                },
            );
            // The part of each color round each pixel: a box of about one row.
            let px_per_mm = 1.0 / to_mm.apply_vector(Point::new(1.0, 0.0)).length().max(1e-9);
            let radius = ((row * px_per_mm) / 2.0).round().max(1.0) as i64;
            let mut order: Vec<usize> = (0..r.palette.len()).collect();
            order.sort_by(|&a, &b| Lab::of(r.palette[b]).l.total_cmp(&Lab::of(r.palette[a]).l));
            let n = order.len().max(1);
            order
                .iter()
                .enumerate()
                .filter_map(|(k, &c)| {
                    let val = share(&r.labels, w, h, c as u16, radius);
                    let field = Field {
                        w,
                        h,
                        val,
                        opaque: opaque.clone(),
                        inv,
                    };
                    let angle = spec.angle_deg + 180.0 * k as f64 / n as f64;
                    rows(&field, bbox, angle, row, spec, MIN_PART).map(|kind| PhotoObject {
                        color: r.palette[c],
                        kind,
                    })
                })
                .collect()
        }
    }
}

/// For each pixel, the part of the pixels of color `c` in the square of
/// `radius` round it (summed-area table).
fn share(labels: &[u16], w: u32, h: u32, c: u16, radius: i64) -> Vec<f32> {
    let (w, h) = (w as usize, h as usize);
    let mut sum = vec![0u32; (w + 1) * (h + 1)];
    let mut all = vec![0u32; (w + 1) * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            let l = labels[y * w + x];
            let k = (y + 1) * (w + 1) + x + 1;
            sum[k] = u32::from(l == c) + sum[k - 1] + sum[k - w - 1] - sum[k - w - 2];
            all[k] = u32::from(l != NONE) + all[k - 1] + all[k - w - 1] - all[k - w - 2];
        }
    }
    let rect = |t: &[u32], x0: usize, y0: usize, x1: usize, y1: usize| {
        t[y1 * (w + 1) + x1] + t[y0 * (w + 1) + x0] - t[y0 * (w + 1) + x1] - t[y1 * (w + 1) + x0]
    };
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as i64 - radius).max(0) as usize;
            let y0 = (y as i64 - radius).max(0) as usize;
            let x1 = ((x as i64 + radius + 1) as usize).min(w);
            let y1 = ((y as i64 + radius + 1) as usize).min(h);
            let n = rect(&all, x0, y0, x1, y1);
            if n > 0 {
                out[y * w + x] = rect(&sum, x0, y0, x1, y1) as f32 / n as f32;
            }
        }
    }
    out
}

/// Needle points in runs, joined into one stitch object: a gap up to
/// [`JOIN_MM`] is one stitch, a longer gap a jump with a trim. Stitches
/// shorter than [`MIN_STITCH_MM`] are left out.
fn stitches(runs: Vec<Vec<Point>>) -> Option<ObjectKind> {
    let runs: Vec<Vec<Point>> = runs
        .into_iter()
        .map(|run| {
            let mut out: Vec<Point> = Vec::with_capacity(run.len());
            for p in run {
                if out.last().is_none_or(|q| q.distance(p) >= MIN_STITCH_MM) {
                    out.push(p);
                }
            }
            out
        })
        .collect();
    let mut path = Path::new();
    let mut trims = Vec::new();
    let mut subs = 0usize;
    let mut last: Option<Point> = None;
    for run in runs.into_iter().filter(|r| r.len() >= 2) {
        let first = run[0];
        match last {
            Some(l) if l.distance(first) <= JOIN_MM => {}
            Some(_) => {
                trims.push(subs);
                path = path.move_to(first);
                subs += 1;
            }
            None => {
                path = path.move_to(first);
                subs += 1;
            }
        }
        if let Some(l) = last
            && l.distance(first) <= JOIN_MM
            && l.distance(first) >= MIN_STITCH_MM
        {
            path = path.line_to(first);
        }
        for p in &run[1..] {
            path = path.line_to(*p);
        }
        last = run.last().copied();
    }
    (subs > 0).then_some(ObjectKind::Stitches { path, trims })
}

/// Zigzag rows over `field` (see the module notes). Where the value is
/// below `min`, there is a gap.
fn rows(field: &Field, bbox: BBox, angle_deg: f64, row: f64, spec: &PhotoSpec, min: f64) -> Option<ObjectKind> {
    let a = math::to_radians(angle_deg);
    let u = Point::new(math::cos(a), math::sin(a));
    let n = Point::new(-u.y, u.x);
    let corners = [
        bbox.min,
        Point::new(bbox.max.x, bbox.min.y),
        bbox.max,
        Point::new(bbox.min.x, bbox.max.y),
    ];
    let span = |d: Point| {
        corners
            .iter()
            .map(|c| c.dot(d))
            .fold((f64::MAX, f64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)))
    };
    let (n0, n1) = span(n);
    let (u0, u1) = span(u);
    let (min_step, max_step) = (spec.min_step_mm.max(0.2), spec.max_step_mm.max(spec.min_step_mm));
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let count = ((n1 - n0) / row).ceil() as usize;
    for k in 0..count {
        let t = n0 + row * (k as f64 + 0.5);
        let forward = k % 2 == 0;
        let mut run: Vec<Point> = Vec::new();
        let mut side = 1.0;
        let mut s = 0.0;
        let len = u1 - u0;
        while s <= len {
            let along = if forward { u0 + s } else { u1 - s };
            let p = u * along + n * t;
            match field.at(p).filter(|v| *v >= min) {
                Some(v) => {
                    let amp = v * row * MAX_FILL;
                    run.push(p + n * (side * amp / 2.0));
                    side = -side;
                    s += max_step - (max_step - min_step) * v;
                }
                None => {
                    if run.len() >= 2 {
                        runs.push(std::mem::take(&mut run));
                    } else {
                        run.clear();
                    }
                    s += min_step;
                }
            }
        }
        if run.len() >= 2 {
            runs.push(run);
        }
    }
    stitches(runs)
}

/// A number from 0 to 1 from a cell and a counter (splitmix64).
fn hash01(x: i64, y: i64, k: u64) -> f64 {
    let mut z = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ k;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Short random stitches between points that are denser where the value is
/// higher (see the module notes).
fn scribble(field: &Field, bbox: BBox, cell: f64) -> Option<ObjectKind> {
    let (nx, ny) = (
        (bbox.width() / cell).ceil() as i64,
        (bbox.height() / cell).ceil() as i64,
    );
    // Up to four points in each cell: four times the square of the value,
    // so light places get few points and dark places many.
    let mut points: Vec<Point> = Vec::new();
    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); (nx * ny).max(0) as usize];
    for j in 0..ny {
        for i in 0..nx {
            let c = bbox.min + Point::new((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
            let Some(v) = field.at(c) else { continue };
            let count = (4.0 * v * v + hash01(i, j, 0)).floor() as u64;
            for k in 0..count {
                let p = bbox.min
                    + Point::new(
                        (i as f64 + hash01(i, j, 2 * k + 1)) * cell,
                        (j as f64 + hash01(i, j, 2 * k + 2)) * cell,
                    );
                grid[(j * nx + i) as usize].push(points.len());
                points.push(p);
            }
        }
    }
    let mut used = vec![false; points.len()];
    let cell_of = |p: Point| {
        (
            (((p.x - bbox.min.x) / cell).floor() as i64).clamp(0, nx - 1),
            (((p.y - bbox.min.y) / cell).floor() as i64).clamp(0, ny - 1),
        )
    };
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let mut run: Vec<Point> = Vec::new();
    let mut next_start = 0usize;
    let mut cur: Option<usize> = None;
    loop {
        let found = cur.and_then(|c| {
            let (ci, cj) = cell_of(points[c]);
            let mut best: Option<(f64, usize)> = None;
            for r in 0..=3i64 {
                for j in (cj - r).max(0)..=(cj + r).min(ny - 1) {
                    for i in (ci - r).max(0)..=(ci + r).min(nx - 1) {
                        if (i - ci).abs() != r && (j - cj).abs() != r {
                            continue;
                        }
                        for &q in &grid[(j * nx + i) as usize] {
                            if !used[q] {
                                let d = points[q].distance(points[c]);
                                if best.is_none_or(|b| d < b.0) {
                                    best = Some((d, q));
                                }
                            }
                        }
                    }
                }
                // A point nearer than the next ring cannot be beaten.
                if best.is_some_and(|b| b.0 <= r as f64 * cell) {
                    break;
                }
            }
            best.filter(|b| b.0 <= 3.0 * cell).map(|b| b.1)
        });
        let q = match found {
            Some(q) => q,
            None => {
                if run.len() >= 2 {
                    runs.push(std::mem::take(&mut run));
                } else {
                    run.clear();
                }
                while next_start < points.len() && used[next_start] {
                    next_start += 1;
                }
                if next_start >= points.len() {
                    break;
                }
                next_start
            }
        };
        used[q] = true;
        run.push(points[q]);
        cur = Some(q);
    }
    if run.len() >= 2 {
        runs.push(run);
    }
    stitches(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A gradient from white (left) to black (right), 100 x 50 px, with a
    /// clear band in the middle (x 45 to 55).
    fn gradient() -> RgbaImage {
        RgbaImage::from_fn(100, 50, |x, _| {
            let g = 255 - (x * 255 / 99) as u8;
            if (45..55).contains(&x) {
                Rgba([0, 0, 0, 0])
            } else {
                Rgba([g, g, g, 255])
            }
        })
    }

    fn needle_points(kind: &ObjectKind) -> Vec<Point> {
        let ObjectKind::Stitches { path, .. } = kind else {
            panic!("{kind:?}")
        };
        path.points().collect()
    }

    #[test]
    fn rows_are_denser_and_wider_where_dark_and_skip_the_clear_band() {
        // 0.5 mm a pixel: 50 x 25 mm.
        let t = Transform::scale(0.5, 0.5);
        let spec = PhotoSpec {
            contrast: 1.0,
            ..PhotoSpec::default()
        };
        let objs = photo(&gradient(), &t, &spec);
        assert_eq!(objs.len(), 1);
        let pts = needle_points(&objs[0].kind);
        let light = pts.iter().filter(|p| p.x < 15.0).count();
        let dark = pts.iter().filter(|p| p.x > 35.0).count();
        assert!(dark > 2 * light, "{light} {dark}");
        // No needle point in the clear band (22.5 to 27.5 mm).
        assert!(pts.iter().all(|p| !(23.0..27.0).contains(&p.x)));
        // The same each time.
        let again = photo(&gradient(), &t, &spec);
        assert_eq!(needle_points(&again[0].kind), pts);
        let ObjectKind::Stitches { trims, .. } = &objs[0].kind else {
            unreachable!()
        };
        assert!(!trims.is_empty(), "the band is wider than a trim");
    }

    #[test]
    fn color_rows_make_a_layer_for_each_color_light_first() {
        // Left half red, right half navy.
        let img = RgbaImage::from_fn(60, 40, |x, _| {
            if x < 30 {
                Rgba([220, 40, 40, 255])
            } else {
                Rgba([20, 30, 90, 255])
            }
        });
        let spec = PhotoSpec {
            style: PhotoStyle::ColorRows,
            colors: 2,
            ..PhotoSpec::default()
        };
        let objs = photo(&img, &Transform::scale(0.5, 0.5), &spec);
        assert_eq!(objs.len(), 2);
        assert!(Lab::of(objs[0].color).l > Lab::of(objs[1].color).l);
        // The red layer stays on the left half (with the share of one row at the middle).
        let red = needle_points(&objs[0].kind);
        assert!(
            red.iter().all(|p| p.x < 16.5),
            "{:?}",
            red.iter().map(|p| p.x).fold(0.0, f64::max)
        );
    }

    #[test]
    fn scribble_is_denser_where_dark_with_short_stitches() {
        let spec = PhotoSpec {
            style: PhotoStyle::Scribble,
            row_mm: 1.0,
            contrast: 1.0,
            ..PhotoSpec::default()
        };
        let objs = photo(&gradient(), &Transform::scale(0.5, 0.5), &spec);
        let ObjectKind::Stitches { path, .. } = &objs[0].kind else {
            panic!()
        };
        let pts: Vec<Point> = path.points().collect();
        let light = pts.iter().filter(|p| p.x < 15.0).count();
        let dark = pts.iter().filter(|p| p.x > 35.0).count();
        assert!(dark > 2 * light, "{light} {dark}");
        // Within a run, no stitch is longer than three cells and a bit, or
        // shorter than the shortest stitch.
        for sub in path.subpaths() {
            let s: Vec<Point> = sub.points().collect();
            assert!(s.windows(2).all(|w| w[0].distance(w[1]) >= MIN_STITCH_MM - 1e-9));
            assert!(
                s.windows(2)
                    .all(|w| w[0].distance(w[1]) <= 3.0 * 1.0 + 1e-9 || w[0].distance(w[1]) <= JOIN_MM)
            );
        }
    }
}
