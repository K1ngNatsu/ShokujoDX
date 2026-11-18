//! The product under the design (P7 Milestone 5): the drawn products as
//! filled areas, and the picture of the design on the product (the
//! virtual decoration picture).
//!
//! The realistic look (P16 Milestone 1, ADR 0101): a height field of the
//! product (round edges, folds, seam grooves) lit by the light of the 3D
//! thread look, the fabric of the product, and a soft shadow under the
//! embroidery.

use emb_geom::region::{self, Join};
use emb_geom::{BBox, Point, math};
use emb_model::{ProductKind, ProductPart, ProductView, Rgb, Shade, product::shade};

use crate::Segment;
use crate::raster::{Areas, Look, Picture, picture_at, stitches_onto_look};
use crate::thread3d::{Lighting, Weave, fabric_value};

const FLATTEN_MM: f64 = 0.2;
/// The ambient and the diffuse part of the light on the product.
const AMBIENT: f64 = 0.45;
const DIFFUSE: f64 = 0.55;
/// The folds of the drawing are this many times higher in the light.
const FOLD_GAIN: f64 = 2.5;
/// Light colors are this much darker on flat cloth, so highlights show.
const HEADROOM: f64 = 0.07;
/// The shadow at the edges of the product, and its width (mm).
const EDGE_DARK: f64 = 0.10;
const EDGE_MM: f64 = 6.0;
/// A seam is a groove of this depth (mm), three times as wide as its line.
const SEAM_DEPTH: f64 = 0.5;
/// A line of this width (mm) or less is a seam.
const SEAM_MAX_MM: f64 = 3.0;
/// The soft shadow of the embroidery: its darkness, its blur and how far
/// it falls (mm).
const SHADOW_DARK: f64 = 0.32;
const SHADOW_BLUR_MM: f64 = 0.6;
const SHADOW_OFFSET_MM: f64 = 0.5;

/// The width (mm) and the height (mm) of the round edge of a product.
fn roundness(kind: ProductKind) -> (f64, f64) {
    match kind {
        ProductKind::TShirt
        | ProductKind::TShirtBack
        | ProductKind::LongSleeve
        | ProductKind::LongSleeveBack
        | ProductKind::Polo
        | ProductKind::PoloBack => (40.0, 6.0),
        ProductKind::Sweatshirt | ProductKind::SweatshirtBack | ProductKind::Hoodie | ProductKind::HoodieBack => {
            (45.0, 8.0)
        }
        ProductKind::Apron => (20.0, 3.0),
        ProductKind::Beanie => (70.0, 25.0),
        ProductKind::Messenger => (25.0, 6.0),
        ProductKind::Cap => (70.0, 20.0),
        ProductKind::Tote => (20.0, 3.0),
        ProductKind::Towel => (15.0, 3.0),
        ProductKind::Cushion => (110.0, 30.0),
    }
}

/// The fabric of a product.
pub fn weave(kind: ProductKind) -> Weave {
    match kind {
        ProductKind::TShirt
        | ProductKind::TShirtBack
        | ProductKind::LongSleeve
        | ProductKind::LongSleeveBack
        | ProductKind::Beanie => Weave::Knit,
        ProductKind::Polo | ProductKind::PoloBack => Weave::Pique,
        ProductKind::Sweatshirt | ProductKind::SweatshirtBack | ProductKind::Hoodie | ProductKind::HoodieBack => {
            Weave::Felt
        }
        ProductKind::Cap | ProductKind::Tote | ProductKind::Apron | ProductKind::Messenger => Weave::Twill,
        ProductKind::Towel => Weave::Terry,
        ProductKind::Cushion => Weave::Plain,
    }
}

/// The areas of a drawn product in view coordinates, in drawing order:
/// rings (even-odd) and a color. Lines become thin areas; a seam (a line
/// of `SEAM_MAX_MM` or less) stays in the areas of the product, so no seam
/// end goes past the edge. Wider lines (a handle, a piping) are parts.
///
/// A trim part (P16 Milestone 6) is cut at the edge of the product. On a
/// product with a trim color, each piece of a line is darker than the
/// color of the area under it, so a seam on a white cap front is light.
pub fn areas(view: &ProductView) -> Vec<(Vec<Vec<Point>>, Rgb)> {
    let o = view.origin;
    let mv = |pts: Vec<Point>| pts.into_iter().map(|p| p + o).collect::<Vec<_>>();
    let rings_of = |regions: Vec<region::Region>| -> Vec<Vec<Point>> {
        regions
            .into_iter()
            .flat_map(|r| std::iter::once(r.outer).chain(r.holes))
            .map(mv)
            .collect()
    };
    let regions_of = |path: &emb_geom::Path| -> Vec<region::Region> {
        path.flatten(FLATTEN_MM)
            .into_iter()
            .map(|pl| region::Region::new(pl.points, Vec::new()))
            .fold(Vec::new(), |a, r| region::union(&a, &[r]))
    };
    let parts = view.kind.parts();
    let mut silhouette: Vec<region::Region> = Vec::new();
    for part in &parts {
        if let ProductPart::Area(path, s) = part
            && !matches!(s, Shade::Trim { .. })
        {
            silhouette = region::union(&silhouette, &regions_of(path));
        }
    }
    let two_tone = view.trim.is_some();
    let dark = shade(view.color, Shade::Dark);
    // On a two-tone product: the areas drawn so far, their box and the
    // color of a line on them.
    let mut painted: Vec<(Vec<region::Region>, Option<BBox>, Rgb)> = Vec::new();
    let mut out = Vec::new();
    for part in parts {
        match part {
            ProductPart::Area(path, s) => {
                let Some(color) = view.part_color(s) else {
                    continue;
                };
                let trim = view.trim.as_ref().filter(|_| matches!(s, Shade::Trim { .. }));
                if !two_tone {
                    out.push((
                        path.flatten(FLATTEN_MM).into_iter().map(|pl| mv(pl.points)).collect(),
                        color,
                    ));
                    continue;
                }
                let mut regions = regions_of(&path);
                if trim.is_some() {
                    regions = region::intersection(&regions, &silhouette);
                }
                let line = trim.map_or(dark, |t| shade(t.color, Shade::Dark));
                let b = regions.iter().filter_map(region::Region::bbox).reduce(BBox::union);
                painted.push((regions.clone(), b, line));
                out.push((rings_of(regions), color));
            }
            ProductPart::Line(path, width) => {
                let band = region::offset_lines(&path.flatten(FLATTEN_MM), width / 2.0, Join::Round);
                let band = if width <= SEAM_MAX_MM {
                    region::intersection(&band, &silhouette)
                } else {
                    band
                };
                if !two_tone {
                    out.push((rings_of(band), dark));
                    continue;
                }
                // The top area under each piece of the line gives its color.
                let mut rest = band;
                for (regions, b, line) in painted.iter().rev() {
                    let Some(rb) = rest.iter().filter_map(region::Region::bbox).reduce(BBox::union) else {
                        break;
                    };
                    if !b.is_some_and(|b| rb.intersects(&b)) {
                        continue;
                    }
                    let piece = region::intersection(&rest, regions);
                    if !piece.is_empty() {
                        rest = region::difference(&rest, regions);
                        out.push((rings_of(piece), *line));
                    }
                }
                out.push((rings_of(rest), dark));
            }
            // A fold has no color (see `light_factors`).
            ProductPart::Fold { .. } => {}
        }
    }
    out.retain(|(rings, _): &(Vec<Vec<Point>>, Rgb)| !rings.is_empty());
    out
}

/// The light of a drawn product at a size: a brightness factor for each
/// pixel of its picture (see [`product_picture_with`]). It does not depend
/// on the color or the place of the product, so it can be kept.
pub struct Shading {
    kind: ProductKind,
    px_per_mm: f64,
    lighting: Lighting,
    factors: Vec<f32>,
}

impl Shading {
    pub fn new(view: &ProductView, px_per_mm: f64, lighting: &Lighting) -> Shading {
        let b = view.bbox().expand(2.0);
        let flat = picture_at(&[], &areas(view), b, px_per_mm, 0.0, None);
        Shading {
            kind: view.kind,
            px_per_mm,
            lighting: *lighting,
            factors: light_factors(view, &flat, b, px_per_mm, lighting),
        }
    }

    /// True if this shading is for products of `kind` at `px_per_mm` in this light.
    pub fn fits(&self, kind: ProductKind, px_per_mm: f64, lighting: &Lighting) -> bool {
        self.kind == kind && self.px_per_mm == px_per_mm && self.lighting == *lighting
    }
}

/// The picture of the drawn product alone, with its light and fabric, and
/// the area (view mm) that it shows.
pub fn product_picture(view: &ProductView, px_per_mm: f64) -> (Picture, BBox) {
    product_picture_with(view, &Shading::new(view, px_per_mm, &Lighting::default()))
}

/// As [`product_picture`], with a kept `shading` (of the same kind of product).
pub fn product_picture_with(view: &ProductView, shading: &Shading) -> (Picture, BBox) {
    let b = view.bbox().expand(2.0);
    let mut pic = picture_at(&[], &areas(view), b, shading.px_per_mm, 0.0, None);
    if pic.rgba.len() == shading.factors.len() * 4 {
        apply_factors(&mut pic, &shading.factors);
    }
    (pic, b)
}

/// Multiplies each colored pixel of `pic` by its factor. Light lifts dark
/// colors a little too, so a black product shows its shape.
fn apply_factors(pic: &mut Picture, factors: &[f32]) {
    for (px, &f) in pic.rgba.as_chunks_mut::<4>().0.iter_mut().zip(factors) {
        if px[3] == 0 {
            continue;
        }
        let f = f64::from(f);
        let luma = (0.299 * f64::from(px[0]) + 0.587 * f64::from(px[1]) + 0.114 * f64::from(px[2])) / 255.0;
        let base = 1.0 - HEADROOM * luma;
        for c in &mut px[..3] {
            *c = (f64::from(*c) * base * f + 45.0 * (f - 1.0)).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// A value from 0 to 1 for a cell of a grid (no repeat).
fn cell_hash(x: i64, y: i64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h & 0xFFFF) as f64 / 65535.0
}

/// Smooth value noise from 0 to 1 with cells of `cell` mm.
fn mottle(x: f64, y: f64, cell: f64) -> f64 {
    let (gx, gy) = (x / cell, y / cell);
    let (ix, iy) = (gx.floor() as i64, gy.floor() as i64);
    let smooth = |f: f64| f * f * (3.0 - 2.0 * f);
    let (sx, sy) = (smooth(gx - gx.floor()), smooth(gy - gy.floor()));
    let a = cell_hash(ix, iy) + (cell_hash(ix + 1, iy) - cell_hash(ix, iy)) * sx;
    let b = cell_hash(ix, iy + 1) + (cell_hash(ix + 1, iy + 1) - cell_hash(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

/// The distance (pixels) of each pixel of `mask` to the nearest pixel out
/// of the mask (chamfer distance, two passes). 0 out of the mask; out of
/// the picture is out of the mask.
fn distance_in(mask: &[bool], w: usize, h: usize) -> Vec<f32> {
    const D: f32 = std::f32::consts::SQRT_2;
    let mut d: Vec<f32> = mask.iter().map(|&m| if m { f32::MAX } else { 0.0 }).collect();
    let at = |x: usize, y: usize| y * w + x;
    for y in 0..h {
        for x in 0..w {
            let i = at(x, y);
            if d[i] == 0.0 {
                continue;
            }
            let mut v = if x == 0 || y == 0 || x + 1 == w { 1.0 } else { f32::MAX };
            if x > 0 {
                v = v.min(d[i - 1] + 1.0);
            }
            if y > 0 {
                v = v.min(d[at(x, y - 1)] + 1.0);
                if x > 0 {
                    v = v.min(d[at(x - 1, y - 1)] + D);
                }
                if x + 1 < w {
                    v = v.min(d[at(x + 1, y - 1)] + D);
                }
            }
            d[i] = d[i].min(v);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = at(x, y);
            if d[i] == 0.0 {
                continue;
            }
            let mut v = if x + 1 == w || y + 1 == h { 1.0 } else { f32::MAX };
            if x + 1 < w {
                v = v.min(d[i + 1] + 1.0);
            }
            if y + 1 < h {
                v = v.min(d[at(x, y + 1)] + 1.0);
                if x + 1 < w {
                    v = v.min(d[at(x + 1, y + 1)] + D);
                }
                if x > 0 {
                    v = v.min(d[at(x - 1, y + 1)] + D);
                }
            }
            d[i] = d[i].min(v);
        }
    }
    d
}

/// Adds a soft ridge (`height` > 0) or valley along `pts` (pixels) to the
/// height field `hf` (mm): `width` pixels wide, smooth across. With
/// `taper`, the fold is low at its two ends.
fn add_ridge(hf: &mut [f32], w: usize, h: usize, pts: &[(f64, f64)], width: f64, height: f64, taper: bool) {
    if pts.len() < 2 || width <= 0.0 {
        return;
    }
    let half = width / 2.0;
    let mut along = vec![0.0];
    for p in pts.windows(2) {
        let l = math::hypot(p[1].0 - p[0].0, p[1].1 - p[0].1);
        along.push(along.last().copied().unwrap_or(0.0) + l);
    }
    let total = along.last().copied().unwrap_or(0.0).max(1e-9);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in pts {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let xa = (x0 - half).floor().max(0.0) as usize;
    let ya = (y0 - half).floor().max(0.0) as usize;
    let xb = ((x1 + half).ceil().max(0.0) as usize).min(w);
    let yb = ((y1 + half).ceil().max(0.0) as usize).min(h);
    for py in ya..yb {
        for px in xa..xb {
            let (cx, cy) = (px as f64 + 0.5, py as f64 + 0.5);
            // The nearest point of the line: its distance and its place along.
            let (mut best, mut at) = (f64::MAX, 0.0);
            for (k, p) in pts.windows(2).enumerate() {
                let (dx, dy) = (p[1].0 - p[0].0, p[1].1 - p[0].1);
                let l2 = dx * dx + dy * dy;
                let t = if l2 > 0.0 {
                    (((cx - p[0].0) * dx + (cy - p[0].1) * dy) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = math::hypot(cx - (p[0].0 + t * dx), cy - (p[0].1 + t * dy));
                if d < best {
                    best = d;
                    at = along[k] + t * (along[k + 1] - along[k]);
                }
            }
            if best >= half {
                continue;
            }
            // cos² across the fold: smooth at the middle and at the sides.
            let c = math::cos(std::f64::consts::FRAC_PI_2 * best / half);
            let mut v = height * c * c;
            if taper {
                let e = (at.min(total - at) / (0.3 * total)).clamp(0.0, 1.0);
                v *= e * e * (3.0 - 2.0 * e);
            }
            hf[py * w + px] += v as f32;
        }
    }
}

/// The brightness factor of each pixel of `flat` (the drawn product on a
/// clear picture of the area `b`): the light on the height field of the
/// product, the shadow at its edges, its fabric and a soft mottle.
fn light_factors(view: &ProductView, flat: &Picture, b: BBox, px_per_mm: f64, lighting: &Lighting) -> Vec<f32> {
    light_factors_with(view, flat, b, px_per_mm, lighting, true)
}

/// As [`light_factors`]; with `round` false, the edges are not round and
/// not darker (a body in 3D has its own shape, see `turntable`).
fn light_factors_with(
    view: &ProductView,
    flat: &Picture,
    b: BBox,
    px_per_mm: f64,
    lighting: &Lighting,
    round: bool,
) -> Vec<f32> {
    let (w, h) = (flat.width as usize, flat.height as usize);
    let mask: Vec<bool> = flat.rgba.as_chunks::<4>().0.iter().map(|p| p[3] > 127).collect();
    let dist = distance_in(&mask, w, h);
    let (round_mm, round_h) = roundness(view.kind);
    let round_h = if round { round_h } else { 0.0 };
    let mut hf: Vec<f32> = dist
        .iter()
        .map(|&d| {
            let t = (f64::from(d) / px_per_mm / round_mm).min(1.0);
            (round_h * (1.0 - (1.0 - t) * (1.0 - t))) as f32
        })
        .collect();
    // The distance has creases where two edges meet (at corners); a blur
    // makes the round edge smooth there.
    blur(&mut hf, w, h, (0.15 * round_mm * px_per_mm).round() as usize);
    let to_px = |p: Point| {
        let q = p + view.origin;
        ((q.x - b.min.x) * px_per_mm, (q.y - b.min.y) * px_per_mm)
    };
    for part in view.kind.parts() {
        let (path, width, height, taper) = match part {
            ProductPart::Fold { path, width, height } => (path, width, height, true),
            ProductPart::Line(path, width) if width <= SEAM_MAX_MM => (path, 3.0 * width, -SEAM_DEPTH, false),
            _ => continue,
        };
        for pl in path.flatten(FLATTEN_MM * 5.0) {
            let pts: Vec<(f64, f64)> = pl.points.iter().map(|p| to_px(*p)).collect();
            add_ridge(&mut hf, w, h, &pts, width * px_per_mm, height * FOLD_GAIN, taper);
        }
    }
    let l = lighting.vector();
    let flat_light = AMBIENT + DIFFUSE * l[2];
    let strength = lighting.strength.clamp(0.0, 1.0);
    let weave = weave(view.kind);
    // The fine weave shows only with enough pixels for it.
    let fine = ((px_per_mm - 1.5) / 4.0).clamp(0.0, 1.0);
    let step = 2.0 / px_per_mm;
    let height_at = |x: usize, y: usize| f64::from(if mask[y * w + x] { hf[y * w + x] } else { 0.0 });
    let mut out = vec![1.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !mask[i] {
                continue;
            }
            let (xl, xr) = (x.saturating_sub(1), (x + 1).min(w - 1));
            let (yu, yd) = (y.saturating_sub(1), (y + 1).min(h - 1));
            let hx = (height_at(xr, y) - height_at(xl, y)) / step;
            let hy = (height_at(x, yd) - height_at(x, yu)) / step;
            let len = (hx * hx + hy * hy + 1.0).sqrt();
            let ndl = ((-hx * l[0] - hy * l[1] + l[2]) / len).max(0.0);
            let mut f = 1.0 + ((AMBIENT + DIFFUSE * ndl) / flat_light - 1.0) * strength;
            let e = f64::from(dist[i]) / px_per_mm / EDGE_MM;
            if round {
                f *= 1.0 - EDGE_DARK / (1.0 + e * e);
            }
            // Fabric and mottle in product coordinates: they move with the product.
            let (mx, my) = (
                b.min.x + (x as f64 + 0.5) / px_per_mm - view.origin.x,
                b.min.y + (y as f64 + 0.5) / px_per_mm - view.origin.y,
            );
            f *= 1.0 + (fabric_value(weave, mx, my) - 1.0) * fine;
            f *= 1.0 + 0.04 * (mottle(mx, my, 9.0) - 0.5) + 0.05 * (mottle(mx, my, 31.0) - 0.5);
            out[i] = f as f32;
        }
    }
    out
}

/// A box blur of `v` (`w` × `h`) with radius `r` pixels, three times (near
/// a Gaussian blur).
fn blur(v: &mut [f32], w: usize, h: usize, r: usize) {
    if r == 0 || w == 0 || h == 0 {
        return;
    }
    let mut tmp = vec![0.0f32; v.len()];
    let n = (2 * r + 1) as f32;
    for _ in 0..3 {
        for y in 0..h {
            let row = &v[y * w..(y + 1) * w];
            let at = |x: usize| row[x.min(w - 1)];
            let mut sum: f32 = (0..=r).map(at).sum::<f32>() + at(0) * r as f32;
            for x in 0..w {
                tmp[y * w + x] = sum / n;
                sum += at(x + r + 1) - at(x.saturating_sub(r));
            }
        }
        for x in 0..w {
            let at = |y: usize| tmp[y.min(h - 1) * w + x];
            let mut sum: f32 = (0..=r).map(at).sum::<f32>() + at(0) * r as f32;
            for y in 0..h {
                v[y * w + x] = sum / n;
                sum += at(y + r + 1) - at(y.saturating_sub(r));
            }
        }
    }
}

/// Darkens `pic` (the area `b`) under the stitches with a soft shadow that
/// falls away from the light: the embroidery sits on the cloth.
fn embroidery_shadow(
    pic: &mut Picture,
    b: BBox,
    segments: &[Segment],
    px_per_mm: f64,
    thread_mm: f64,
    lighting: &Lighting,
) {
    let cover = picture_at(segments, &[] as &Areas, b, px_per_mm, thread_mm, None);
    let (w, h) = (pic.width as usize, pic.height as usize);
    if cover.rgba.len() != pic.rgba.len() {
        return;
    }
    let mut v: Vec<f32> = cover
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| f32::from(p[3]) / 255.0)
        .collect();
    blur(&mut v, w, h, (SHADOW_BLUR_MM * px_per_mm).round() as usize);
    let (sx, sy) = lighting.shadow_dir();
    let off = SHADOW_OFFSET_MM * px_per_mm;
    let (ox, oy) = ((sx * off).round() as i64, (sy * off).round() as i64);
    let dark = SHADOW_DARK * lighting.strength.clamp(0.0, 1.0);
    for y in 0..h as i64 {
        let fy = y - oy;
        if !(0..h as i64).contains(&fy) {
            continue;
        }
        for x in 0..w as i64 {
            let fx = x - ox;
            if !(0..w as i64).contains(&fx) {
                continue;
            }
            let s = f64::from(v[fy as usize * w + fx as usize]);
            if s <= 0.0 {
                continue;
            }
            let i = (y as usize * w + x as usize) * 4;
            for c in &mut pic.rgba[i..i + 3] {
                *c = (f64::from(*c) * (1.0 - dark * s)).round() as u8;
            }
        }
    }
}

/// Stitches farther than this from the product do not make the picture
/// of the design on the product larger (mm): a design far away (a wrong
/// value in a file) must not make a picture of gigabytes.
const DECORATION_REACH_MM: f64 = 1000.0;

/// The area (view mm) that [`decoration`] shows: the product and the
/// stitches (to [`DECORATION_REACH_MM`] round the product), with 10 mm
/// round them.
pub fn decoration_area(view: &ProductView, segments: &[Segment]) -> BBox {
    let reach = view.bbox().expand(DECORATION_REACH_MM);
    let stitches = BBox::from_points(segments.iter().flat_map(|s| [s.a, s.b])).and_then(|s| {
        let min = Point::new(s.min.x.max(reach.min.x), s.min.y.max(reach.min.y));
        let max = Point::new(s.max.x.min(reach.max.x), s.max.y.min(reach.max.y));
        (min.x <= max.x && min.y <= max.y).then(|| BBox::new(min, max))
    });
    [Some(view.bbox()), stitches]
        .into_iter()
        .flatten()
        .reduce(BBox::union)
        .unwrap_or_else(|| view.bbox())
        .expand(10.0)
}

/// The picture of the design on the product: the product (drawn, or the
/// picture of the user's product decoded as `photo`), then the stitches
/// (view coordinates) with the thread look. The picture shows the product
/// and the design with 10 mm round them; the background is white.
pub fn decoration(
    view: &ProductView,
    photo: Option<&image::RgbaImage>,
    segments: &[Segment],
    px_per_mm: f64,
    thread_mm: f64,
    look: &Look,
) -> Picture {
    decoration_with(view, photo, segments, px_per_mm, thread_mm, look, true)
}

/// As [`decoration`] for a side of the turntable view (P16 Milestone 8):
/// the edges of the product are not round and not darker, and the picture
/// is clear (not white) round the product.
///
/// The picture shows [`turn_area`] only: stitches out of the product are
/// not on the body.
pub fn turn_side(view: &ProductView, segments: &[Segment], px_per_mm: f64, thread_mm: f64, look: &Look) -> Picture {
    decoration_with(view, None, segments, px_per_mm, thread_mm, look, false)
}

/// The area (view mm) of a picture of [`turn_side`]: the product with
/// 10 mm round it.
pub fn turn_area(view: &ProductView) -> BBox {
    view.bbox().expand(10.0)
}

fn decoration_with(
    view: &ProductView,
    photo: Option<&image::RgbaImage>,
    segments: &[Segment],
    px_per_mm: f64,
    thread_mm: f64,
    look: &Look,
    round: bool,
) -> Picture {
    let b = if round {
        decoration_area(view, segments)
    } else {
        turn_area(view)
    };
    let white = Some(Rgb(255, 255, 255));
    let lighting = look.lighting.unwrap_or_default();
    let mut pic = match (&view.picture, photo) {
        (Some(p), Some(img)) => {
            let mut pic = picture_at(&[], &[] as &Areas, b, px_per_mm, 0.0, white);
            // The product color on the masked part of the photo (P16 Milestone 7).
            let colored = p.mask.as_ref().filter(|m| !m.is_empty()).map(|m| {
                let mut c = img.clone();
                let (w, h) = c.dimensions();
                crate::photo::recolor(&mut c, w, h, m, view.color);
                c
            });
            let img = colored.as_ref().unwrap_or(img);
            paint_photo(&mut pic, b, px_per_mm, img, view.origin, p.width_mm, p.height_mm());
            pic
        }
        _ => {
            let mut pic = picture_at(&[], &areas(view), b, px_per_mm, 0.0, None);
            let factors = light_factors_with(view, &pic, b, px_per_mm, &lighting, round);
            apply_factors(&mut pic, &factors);
            if !round {
                embroidery_shadow(&mut pic, b, segments, px_per_mm, thread_mm, &lighting);
                stitches_onto_look(&mut pic, b, segments, px_per_mm, thread_mm, look.lighting.as_ref());
                return pic;
            }
            // White round the product.
            for px in pic.rgba.as_chunks_mut::<4>().0 {
                if px[3] == 0 {
                    *px = [255, 255, 255, 255];
                }
            }
            pic
        }
    };
    embroidery_shadow(&mut pic, b, segments, px_per_mm, thread_mm, &lighting);
    stitches_onto_look(&mut pic, b, segments, px_per_mm, thread_mm, look.lighting.as_ref());
    pic
}

/// The pictures `a` and `b` next to each other (`a` on the left) with a
/// gap of `gap` pixels. The tops align; the rest is white.
pub fn side_by_side(a: &Picture, b: &Picture, gap: u32) -> Picture {
    let width = a.width + gap + b.width;
    let height = a.height.max(b.height);
    let mut rgba = vec![255u8; width as usize * height as usize * 4];
    for (pic, x0) in [(a, 0), (b, a.width + gap)] {
        let row = pic.width as usize * 4;
        for y in 0..pic.height as usize {
            let at = (y * width as usize + x0 as usize) * 4;
            rgba[at..at + row].copy_from_slice(&pic.rgba[y * row..(y + 1) * row]);
        }
    }
    Picture { width, height, rgba }
}

/// Draws `img` over `pic` (which shows `b`) with its top left corner at
/// `at` and a size of `w` × `h` mm. Transparent pixels of `img` let the
/// white show.
fn paint_photo(pic: &mut Picture, b: BBox, px_per_mm: f64, img: &image::RgbaImage, at: Point, w: f64, h: f64) {
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    for py in 0..pic.height {
        let y = b.min.y + (f64::from(py) + 0.5) / px_per_mm;
        let v = (y - at.y) / h;
        if !(0.0..1.0).contains(&v) {
            continue;
        }
        let sy = ((v * f64::from(ih)) as u32).min(ih - 1);
        for px in 0..pic.width {
            let x = b.min.x + (f64::from(px) + 0.5) / px_per_mm;
            let u = (x - at.x) / w;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let sx = ((u * f64::from(iw)) as u32).min(iw - 1);
            let s = img.get_pixel(sx, sy).0;
            let a = f64::from(s[3]) / 255.0;
            let i = (py as usize * pic.width as usize + px as usize) * 4;
            for (dst, src) in pic.rgba[i..i + 3].iter_mut().zip(s) {
                *dst = (f64::from(src) * a + f64::from(*dst) * (1.0 - a)).round() as u8;
            }
            pic.rgba[i + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::ProductKind;

    use super::*;
    use crate::SegmentKind;

    fn pixel(p: &Picture, b: BBox, px_per_mm: f64, at: Point) -> [u8; 4] {
        let x = ((at.x - b.min.x) * px_per_mm) as usize;
        let y = ((at.y - b.min.y) * px_per_mm) as usize;
        let i = (y * p.width as usize + x) * 4;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2], p.rgba[i + 3]]
    }

    #[test]
    fn the_product_shows_its_color_and_the_inside_of_the_neck() {
        let mut v = ProductView::new(ProductKind::TShirt, Rgb(200, 30, 40));
        v.origin = Point::new(10.0, -100.0);
        let (pic, b) = product_picture(&v, 1.0);
        // The chest is near the product color (light and fabric change it a
        // little); the inside of the back neck is darker.
        let chest = pixel(&pic, b, 1.0, v.origin + Point::new(0.0, 300.0));
        assert!(near(chest, [200, 30, 40], 25), "{chest:?}");
        let neck = pixel(&pic, b, 1.0, v.origin + Point::new(0.0, 60.0));
        assert!(neck[0] < 200 && neck[3] == 255, "{neck:?}");
        // Outside the shirt is clear.
        assert_eq!(pixel(&pic, b, 1.0, v.origin + Point::new(320.0, 600.0))[3], 0);
    }

    #[test]
    fn no_seam_goes_past_the_edge_of_the_product() {
        for kind in ProductKind::ALL {
            let v = ProductView::new(kind, Rgb(128, 128, 128));
            let mut body: Vec<region::Region> = Vec::new();
            let mut drawn: Vec<region::Region> = Vec::new();
            for (part, (rings, _)) in kind
                .parts()
                .iter()
                .filter(|p| !matches!(p, ProductPart::Fold { .. } | ProductPart::Area(_, Shade::Trim { .. })))
                .zip(areas(&v))
            {
                let regions: Vec<region::Region> =
                    rings.into_iter().map(|r| region::Region::new(r, Vec::new())).collect();
                match part {
                    ProductPart::Line(_, w) if *w > SEAM_MAX_MM => body = region::union(&body, &regions),
                    ProductPart::Area(..) => body = region::union(&body, &regions),
                    _ => {}
                }
                drawn = region::union(&drawn, &regions);
            }
            let outside: f64 = region::difference(&drawn, &body).iter().map(region::Region::area).sum();
            assert!(outside < 1.0, "{kind:?}: {outside} mm² of seams outside");
        }
    }

    #[test]
    fn a_trim_color_goes_on_the_trim_parts_only() {
        // A navy cap with a white front, and a gray polo with a black collar.
        let px = 2.0;
        let mut cap = ProductView::new(ProductKind::Cap, Rgb(30, 38, 66));
        cap.trim = Some(emb_model::Trim {
            color: Rgb(250, 250, 250),
            off: vec!["Brim".into(), "Button".into()],
        });
        let (pic, b) = product_picture(&cap, px);
        let front = pixel(&pic, b, px, cap.origin + Point::new(30.0, -40.0));
        let side = pixel(&pic, b, px, cap.origin + Point::new(95.0, -40.0));
        let brim = pixel(&pic, b, px, cap.origin + Point::new(0.0, 40.0));
        assert!(front[0] > 200 && front[2] > 200, "{front:?}");
        assert!(side[0] < 60 && brim[0] < 60, "{side:?} {brim:?}");
        // The front seam on the white front is light, not navy.
        let seam = pixel(&pic, b, px, cap.origin + Point::new(0.0, -60.0));
        assert!(seam[0] > 130, "{seam:?}");

        let mut polo = ProductView::new(ProductKind::Polo, Rgb(170, 170, 172));
        polo.trim = Some(emb_model::Trim {
            color: Rgb(24, 24, 26),
            off: vec!["Sleeve cuffs".into()],
        });
        let (pic, b) = product_picture(&polo, px);
        let collar = pixel(&pic, b, px, polo.origin + Point::new(60.0, 60.0));
        let chest = pixel(&pic, b, px, polo.origin + Point::new(110.0, 300.0));
        let cuff = pixel(&pic, b, px, polo.origin + Point::new(395.0, 175.0));
        assert!(collar[0] < 70, "{collar:?}");
        assert!(chest[0] > 120 && cuff[0] > 120, "{chest:?} {cuff:?}");
        // With the cuffs on, the cuffs are black too.
        polo.set_trim_on("Sleeve cuffs", true);
        let (pic, b) = product_picture(&polo, px);
        assert!(pixel(&pic, b, px, polo.origin + Point::new(395.0, 175.0))[0] < 70);
    }

    #[test]
    fn no_trim_part_goes_past_the_edge_of_the_product() {
        for kind in ProductKind::ALL {
            let plain = ProductView::new(kind, Rgb(128, 128, 128));
            let mut two = plain.clone();
            two.trim = Some(emb_model::Trim {
                color: Rgb(20, 20, 20),
                off: Vec::new(),
            });
            let union = |v: &ProductView| {
                areas(v).into_iter().fold(Vec::new(), |a, (rings, _)| {
                    let r: Vec<region::Region> =
                        rings.into_iter().map(|r| region::Region::new(r, Vec::new())).collect();
                    region::union(&a, &r)
                })
            };
            let outside: f64 = region::difference(&union(&two), &union(&plain))
                .iter()
                .map(region::Region::area)
                .sum();
            assert!(outside < 1.0, "{kind:?}: {outside} mm² of trim outside");
            if !kind.trims().is_empty() {
                assert!(areas(&two).len() > areas(&plain).len(), "{kind:?} shows no trim");
            }
        }
    }

    #[test]
    fn a_design_far_away_does_not_make_a_huge_picture() {
        let v = ProductView::new(ProductKind::Tote, Rgb(240, 230, 200));
        let far = Segment {
            a: Point::new(900_000.0, 0.0),
            b: Point::new(900_010.0, 0.0),
            color: Rgb(0, 0, 200),
            kind: SegmentKind::Stitch,
        };
        let b = decoration_area(&v, &[far]);
        assert!(b.width() <= v.bbox().width() + 2020.0, "{b:?}");
    }

    #[test]
    fn the_decoration_has_the_stitches_on_the_product() {
        let v = ProductView::new(ProductKind::Tote, Rgb(240, 230, 200));
        let seg = Segment {
            a: Point::new(-20.0, 230.0),
            b: Point::new(20.0, 230.0),
            color: Rgb(0, 0, 200),
            kind: SegmentKind::Stitch,
        };
        let pic = decoration(&v, None, &[seg], 2.0, 0.6, &Look::default());
        let b = v.bbox().expand(10.0);
        // The blue stitch is on the bag (its soft shadow darkens the edges a little).
        let st = pixel(&pic, b, 2.0, Point::new(0.0, 230.0));
        assert!(st[0] < 100 && st[2] > 170, "{st:?}");
        let bag = pixel(&pic, b, 2.0, Point::new(0.0, 300.0));
        assert!(near(bag, [240, 230, 200], 25), "{bag:?}");
        // White round the product.
        assert_eq!(pixel(&pic, b, 2.0, Point::new(-199.0, 400.0)), [255, 255, 255, 255]);
    }

    fn near(p: [u8; 4], c: [u8; 3], tol: i32) -> bool {
        p[3] == 255 && (0..3).all(|k| (i32::from(p[k]) - i32::from(c[k])).abs() <= tol)
    }

    fn luma(p: [u8; 4]) -> f64 {
        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])
    }

    #[test]
    fn the_light_shapes_the_product() {
        // A cushion is round: its edge toward the light (top left) is
        // brighter than its edge away from the light.
        let v = ProductView::new(ProductKind::Cushion, Rgb(120, 120, 120));
        let (pic, b) = product_picture(&v, 1.0);
        let lit = luma(pixel(&pic, b, 1.0, Point::new(-190.0, 225.0)));
        let dark = luma(pixel(&pic, b, 1.0, Point::new(190.0, 225.0)));
        assert!(lit > dark + 15.0, "{lit} {dark}");
        // With no light strength, only the fabric and the edge shadow stay.
        let flat = Shading::new(
            &v,
            1.0,
            &Lighting {
                strength: 0.0,
                ..Lighting::default()
            },
        );
        let (pic, _) = product_picture_with(&v, &flat);
        let (l, d) = (
            luma(pixel(&pic, b, 1.0, Point::new(-190.0, 225.0))),
            luma(pixel(&pic, b, 1.0, Point::new(190.0, 225.0))),
        );
        assert!((l - d).abs() < 10.0, "{l} {d}");
    }

    #[test]
    fn a_fold_has_a_light_side_and_a_dark_side() {
        // The fold of a T-shirt below the right arm (valley, from the arm
        // down to the left): the light makes its two sides different.
        let v = ProductView::new(ProductKind::TShirt, Rgb(200, 200, 200));
        let (pic, b) = product_picture(&v, 2.0);
        let across: Vec<f64> = (0..24)
            .map(|k| luma(pixel(&pic, b, 2.0, Point::new(196.0 + f64::from(k), 330.0))))
            .collect();
        let (lo, hi) = across
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(hi - lo > 20.0, "{across:?}");
    }

    #[test]
    fn the_shading_is_kept_for_a_kind_and_a_light() {
        let v = ProductView::new(ProductKind::Tote, Rgb(10, 10, 10));
        let s = Shading::new(&v, 1.0, &Lighting::default());
        assert!(s.fits(ProductKind::Tote, 1.0, &Lighting::default()));
        assert!(!s.fits(ProductKind::Towel, 1.0, &Lighting::default()));
        assert!(!s.fits(ProductKind::Tote, 2.0, &Lighting::default()));
        // A kept shading gives the same picture for another color and place.
        let mut red = ProductView::new(ProductKind::Tote, Rgb(200, 0, 0));
        red.origin = Point::new(40.0, -7.0);
        let (kept, _) = product_picture_with(&red, &s);
        let (new, _) = product_picture(&red, 1.0);
        assert_eq!(kept.rgba, new.rgba);
    }

    #[test]
    fn the_embroidery_has_a_soft_shadow() {
        // Light from the top left: the shadow falls to the bottom right of
        // a satin block, a little out from its edge, and is soft.
        let v = ProductView::new(ProductKind::Cushion, Rgb(230, 230, 230));
        let segs: Vec<Segment> = (0..40)
            .map(|k| {
                let y = 200.0 + f64::from(k) * 0.4;
                Segment {
                    a: Point::new(-20.0, y),
                    b: Point::new(20.0, y),
                    color: Rgb(0, 0, 200),
                    kind: SegmentKind::Stitch,
                }
            })
            .collect();
        let pic = decoration(&v, None, &segs, 8.0, 0.4, &Look::default());
        let b = v.bbox().expand(10.0);
        let at = |x: f64, y: f64| luma(pixel(&pic, b, 8.0, Point::new(x, y)));
        // Just below the block (it ends near y = 215.8) and far below it.
        let (near_edge, far) = (at(0.0, 216.3), at(0.0, 225.0));
        assert!(near_edge < far - 10.0, "{near_edge} {far}");
        // Above the block (toward the light) there is less shadow.
        assert!(at(0.0, 199.2) > near_edge, "{} {near_edge}", at(0.0, 199.2));
    }

    #[test]
    fn the_decoration_has_the_3d_look() {
        let v = ProductView::new(ProductKind::Tote, Rgb(240, 230, 200));
        let seg = Segment {
            a: Point::new(-20.0, 230.0),
            b: Point::new(20.0, 230.0),
            color: Rgb(0, 0, 200),
            kind: SegmentKind::Stitch,
        };
        let flat = decoration(&v, None, &[seg], 8.0, 1.0, &Look::default());
        let lit = decoration(&v, None, &[seg], 8.0, 1.0, &Look::three_d(None));
        let b = v.bbox().expand(10.0);
        // The top edge of the thread is lighter than the bottom edge (light from the top left).
        let top = pixel(&lit, b, 8.0, Point::new(0.0, 229.75))[2];
        let bottom = pixel(&lit, b, 8.0, Point::new(0.0, 230.25))[2];
        let flat_top = pixel(&flat, b, 8.0, Point::new(0.0, 229.75))[2];
        let flat_bottom = pixel(&flat, b, 8.0, Point::new(0.0, 230.25))[2];
        assert!(
            i32::from(top) - i32::from(bottom) > (i32::from(flat_top) - i32::from(flat_bottom)).abs() + 20,
            "{top} {bottom} {flat_top} {flat_bottom}"
        );
    }

    #[test]
    fn a_photo_of_the_product_is_under_the_stitches() {
        let img = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 120, 0, 255]));
        let mut v = ProductView::new(ProductKind::Tote, Rgb(0, 0, 0));
        v.picture = Some(emb_model::ProductPicture {
            name: "bag.png".into(),
            data: std::sync::Arc::new(Vec::new()),
            pixels: (4, 2),
            width_mm: 200.0,
            mask: None,
            placements: Vec::new(),
        });
        let pic = decoration(&v, Some(&img), &[], 1.0, 0.4, &Look::default());
        let b = v.bbox().expand(10.0);
        assert_eq!(pixel(&pic, b, 1.0, Point::new(100.0, 50.0)), [0, 120, 0, 255]);
        assert_eq!(v.bbox().height(), 100.0);
    }
}
