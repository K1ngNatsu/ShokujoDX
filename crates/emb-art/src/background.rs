//! Background removal: pixels of one color become transparent, so the
//! color reduction, the trace and the picture-to-design steps leave them
//! out (P6, owner request of 2026-10-23).
//!
//! - [`remove_at`]: the color under a click, within a tolerance: only the
//!   area that touches the clicked pixel (as a magic wand), or that color
//!   everywhere in the picture.
//!
//! The area that touches a pixel follows a color that changes slowly (a
//! wall that is lighter on one side of a photo), and stops at the edge of
//! the subject, also in a photo with grain:
//!
//! 1. The colors are averaged over a few pixels first, so the grain does
//!    not count.
//! 2. Where the averaged color changes fast from pixel to pixel, there is
//!    an edge: the removal does not go through it.
//! 3. Each removed pixel gives its neighbors a reference color that moves
//!    a little toward its own averaged color; a neighbor goes if it is
//!    within the tolerance of that reference.
//! 4. Last, the band of a few pixels along the edge, where the averaged
//!    colors mix the subject and the background, is checked pixel by pixel
//!    against the reference.
//! - [`remove_edges`]: the most common color at the edge of the picture,
//!   in the areas that touch the edge (the usual background of a logo or
//!   of a photo on a plain wall). The top edge counts most and the bottom
//!   edge least: a person or a thing in a photo usually stands on the
//!   bottom edge.
//! - [`trim`]: cuts the transparent edges off.
//! - [`brush`]: clears the pixels under a round brush, or brings them back
//!   from the original picture (for the parts that a color click cannot
//!   separate, such as hair against a busy background).
//! - [`shrink_edge`]: clears the edge of the cut-out (the band of mixed
//!   colors between the subject and the old background).
//! - [`clean`]: removes small bits that stay after a removal, and fills
//!   small holes in the subject from the original picture.
//! - [`erase_shape`]: clears (or brings back) all pixels inside or outside
//!   a box or a lasso shape, for large parts that do not belong.
//!
//! Colors are compared in Lab space (the distance is about how different
//! two colors look; 2 is just visible, 10 is clear).

use std::collections::BTreeMap;

use emb_model::{Lab, Rgb};
use image::RgbaImage;

/// The default tolerance (Lab distance).
pub const DEFAULT_TOLERANCE: f64 = 12.0;
/// The reference color moves this part of the way to each removed pixel.
const FOLLOW: f64 = 0.1;
/// The reference color stays within this many tolerances of the first
/// color (with the default tolerance, 30: a wall that is clearly lighter
/// on one side; not the dark hair at the edge of a light background).
const DRIFT: f64 = 2.5;
/// Colors are averaged over a square of this radius (pixels).
const SMOOTH_PX: i64 = 2;
/// An averaged color that changes more than this a pixel is an edge
/// (Lab distance; at least this, or a part of the tolerance).
const EDGE_MIN: f64 = 3.0;
const EDGE_PART: f64 = 0.35;

fn mix(a: Lab, b: Lab, t: f64) -> Lab {
    Lab {
        l: a.l + (b.l - a.l) * t,
        a: a.a + (b.a - a.a) * t,
        b: a.b + (b.b - a.b) * t,
    }
}

fn lab(p: &image::Rgba<u8>) -> Lab {
    Lab::of(Rgb(p[0], p[1], p[2]))
}

fn opaque(p: &image::Rgba<u8>) -> bool {
    p[3] >= 128
}

/// The Lab color of each opaque pixel averaged over the opaque pixels in a
/// square of [`SMOOTH_PX`] round it (summed-area tables).
fn smoothed(img: &RgbaImage) -> Vec<Lab> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut t = vec![[0.0f64; 4]; (w + 1) * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x as u32, y as u32);
            let v = if opaque(p) {
                let l = lab(p);
                [l.l, l.a, l.b, 1.0]
            } else {
                [0.0; 4]
            };
            let k = (y + 1) * (w + 1) + x + 1;
            for c in 0..4 {
                t[k][c] = v[c] + t[k - 1][c] + t[k - w - 1][c] - t[k - w - 2][c];
            }
        }
    }
    let mut out = vec![Lab { l: 0.0, a: 0.0, b: 0.0 }; w * h];
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as i64 - SMOOTH_PX).max(0) as usize;
            let y0 = (y as i64 - SMOOTH_PX).max(0) as usize;
            let x1 = ((x as i64 + SMOOTH_PX + 1) as usize).min(w);
            let y1 = ((y as i64 + SMOOTH_PX + 1) as usize).min(h);
            let s = |c: usize| {
                t[y1 * (w + 1) + x1][c] + t[y0 * (w + 1) + x0][c] - t[y0 * (w + 1) + x1][c] - t[y1 * (w + 1) + x0][c]
            };
            let n = s(3).max(1.0);
            out[y * w + x] = Lab {
                l: s(0) / n,
                a: s(1) / n,
                b: s(2) / n,
            };
        }
    }
    out
}

/// How fast the averaged color changes at each pixel (Lab distance a
/// pixel, the larger of across and down).
fn edges(s: &[Lab], w: usize, h: usize) -> Vec<f64> {
    let at = |x: usize, y: usize| s[y * w + x];
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let (xa, xb) = (x.saturating_sub(1), (x + 1).min(w - 1));
            let (ya, yb) = (y.saturating_sub(1), (y + 1).min(h - 1));
            let dx = at(xa, y).distance(at(xb, y)) / (xb - xa).max(1) as f64;
            let dy = at(x, ya).distance(at(x, yb)) / (yb - ya).max(1) as f64;
            out[y * w + x] = dx.max(dy);
        }
    }
    out
}

/// Makes the pixels from the seeds transparent (see the module notes).
/// Returns the number of pixels removed.
fn flood(img: &mut RgbaImage, seeds: &[(u32, u32)], color: Lab, tolerance: f64) -> usize {
    let (w, h) = img.dimensions();
    let (wu, hu) = (w as usize, h as usize);
    let smooth = smoothed(img);
    let edge = edges(&smooth, wu, hu);
    let edge_max = EDGE_MIN.max(EDGE_PART * tolerance);
    let mut reference: Vec<Option<Lab>> = vec![None; wu * hu];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for &(x, y) in seeds {
        if x >= w || y >= h {
            continue;
        }
        let k = (y * w + x) as usize;
        let p = img.get_pixel(x, y);
        // A seed counts by its own color or by its averaged color (a small
        // spot, or grain).
        let near = lab(p).distance(color) <= tolerance || smooth[k].distance(color) <= tolerance;
        if reference[k].is_none() && opaque(p) && near {
            reference[k] = Some(mix(color, smooth[k], FOLLOW));
            stack.push((x, y));
        }
    }
    let next = |x: u32, y: u32| [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
    let mut n = 0;
    let mut removed = Vec::new();
    while let Some((x, y)) = stack.pop() {
        let r = reference[(y * w + x) as usize].unwrap_or(color);
        removed.push((x, y));
        n += 1;
        for (qx, qy) in next(x, y) {
            if qx >= w || qy >= h {
                continue;
            }
            let k = (qy * w + qx) as usize;
            if reference[k].is_some() || !opaque(img.get_pixel(qx, qy)) {
                continue;
            }
            let s = smooth[k];
            if edge[k] <= edge_max && s.distance(r) <= tolerance && s.distance(color) <= DRIFT * tolerance {
                reference[k] = Some(mix(r, s, FOLLOW));
                stack.push((qx, qy));
            }
        }
    }
    // The band along the edge: each pixel by its own color.
    let mut front = removed.clone();
    for _ in 0..=SMOOTH_PX {
        let mut grown = Vec::new();
        for &(x, y) in &front {
            let r = reference[(y * w + x) as usize].unwrap_or(color);
            for (qx, qy) in next(x, y) {
                if qx >= w || qy >= h {
                    continue;
                }
                let k = (qy * w + qx) as usize;
                let q = img.get_pixel(qx, qy);
                let lq = lab(q);
                if reference[k].is_none()
                    && opaque(q)
                    && lq.distance(r) <= tolerance
                    && lq.distance(color) <= DRIFT * tolerance
                {
                    reference[k] = Some(r);
                    grown.push((qx, qy));
                }
            }
        }
        n += grown.len();
        removed.extend_from_slice(&grown);
        front = grown;
    }
    for (x, y) in removed {
        img.get_pixel_mut(x, y)[3] = 0;
    }
    n
}

/// Removes the color of the pixel at `(x, y)`, within `tolerance`: the
/// area that touches it (`connected`), or the color everywhere. Returns the
/// number of pixels removed (0 if the pixel is already transparent).
pub fn remove_at(img: &mut RgbaImage, x: u32, y: u32, tolerance: f64, connected: bool) -> usize {
    if x >= img.width() || y >= img.height() || !opaque(img.get_pixel(x, y)) {
        return 0;
    }
    let color = lab(img.get_pixel(x, y));
    if connected {
        return flood(img, &[(x, y)], color, tolerance);
    }
    let mut n = 0;
    for p in img.pixels_mut() {
        if opaque(p) && lab(p).distance(color) <= tolerance {
            p[3] = 0;
            n += 1;
        }
    }
    n
}

/// Removes the most common color at the edge of the picture, within
/// `tolerance`, in the areas that touch the edge. Returns the number of
/// pixels removed.
pub fn remove_edges(img: &mut RgbaImage, tolerance: f64) -> usize {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 0;
    }
    let edge: Vec<(u32, u32)> = (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]))
        .collect();
    // The most common edge color (4 bits for each channel, so grain does
    // not split a color), as the mean of its pixels. Top 4, sides 2, bottom 1.
    let mut bins: BTreeMap<u16, ([u64; 3], u64)> = BTreeMap::new();
    for &(x, y) in &edge {
        let p = img.get_pixel(x, y);
        if opaque(p) {
            let weight = if y == 0 {
                4
            } else if y == h - 1 {
                1
            } else {
                2
            };
            let key = (u16::from(p[0] >> 4) << 8) | (u16::from(p[1] >> 4) << 4) | u16::from(p[2] >> 4);
            let b = bins.entry(key).or_default();
            for c in 0..3 {
                b.0[c] += weight * u64::from(p[c]);
            }
            b.1 += weight;
        }
    }
    let Some((sum, count)) = bins.values().max_by_key(|b| b.1).copied() else {
        return 0;
    };
    let mean = |c: usize| (sum[c] / count) as u8;
    flood(img, &edge, Lab::of(Rgb(mean(0), mean(1), mean(2))), tolerance)
}

/// Brush: clears the pixels within `radius` of `(x, y)` (pixels), or with
/// `restore` brings them back from `source` (the same size).
pub fn brush(img: &mut RgbaImage, source: &RgbaImage, x: f64, y: f64, radius: f64, restore: bool) {
    let (w, h) = img.dimensions();
    let r = radius.max(0.5);
    let x0 = (x - r).floor().max(0.0) as u32;
    let y0 = (y - r).floor().max(0.0) as u32;
    let x1 = ((x + r).ceil().max(0.0) as u32).min(w);
    let y1 = ((y + r).ceil().max(0.0) as u32).min(h);
    for py in y0..y1 {
        for px in x0..x1 {
            let (dx, dy) = (f64::from(px) + 0.5 - x, f64::from(py) + 0.5 - y);
            if dx * dx + dy * dy <= r * r {
                if restore {
                    if px < source.width() && py < source.height() {
                        img.put_pixel(px, py, *source.get_pixel(px, py));
                    }
                } else {
                    img.get_pixel_mut(px, py)[3] = 0;
                }
            }
        }
    }
}

/// Clears the opaque pixels within `px` pixels (4 neighbors, step by step)
/// of a clear pixel. Returns the number of pixels cleared.
pub fn shrink_edge(img: &mut RgbaImage, px: u32) -> usize {
    let (w, h) = img.dimensions();
    let mut n = 0;
    for _ in 0..px {
        let edge: Vec<(u32, u32)> = img
            .enumerate_pixels()
            .filter(|(x, y, p)| {
                opaque(p)
                    && [
                        (x.wrapping_sub(1), *y),
                        (x + 1, *y),
                        (*x, y.wrapping_sub(1)),
                        (*x, y + 1),
                    ]
                    .iter()
                    .any(|&(qx, qy)| qx < w && qy < h && !opaque(img.get_pixel(qx, qy)))
            })
            .map(|(x, y, _)| (x, y))
            .collect();
        if edge.is_empty() {
            break;
        }
        for (x, y) in edge {
            img.get_pixel_mut(x, y)[3] = 0;
            n += 1;
        }
    }
    n
}

/// The connected parts (8 neighbors) of the pixels where `inside` is true:
/// for each part, its pixels.
pub(crate) fn parts(w: u32, h: u32, inside: &dyn Fn(u32, u32) -> bool) -> Vec<Vec<(u32, u32)>> {
    let mut seen = vec![false; (w * h) as usize];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for y0 in 0..h {
        for x0 in 0..w {
            if seen[(y0 * w + x0) as usize] || !inside(x0, y0) {
                continue;
            }
            seen[(y0 * w + x0) as usize] = true;
            stack.push((x0, y0));
            let mut part = Vec::new();
            while let Some((x, y)) = stack.pop() {
                part.push((x, y));
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (qx, qy) = (i64::from(x) + dx, i64::from(y) + dy);
                        if qx < 0 || qy < 0 || qx >= i64::from(w) || qy >= i64::from(h) {
                            continue;
                        }
                        let (qx, qy) = (qx as u32, qy as u32);
                        let k = (qy * w + qx) as usize;
                        if !seen[k] && inside(qx, qy) {
                            seen[k] = true;
                            stack.push((qx, qy));
                        }
                    }
                }
            }
            out.push(part);
        }
    }
    out
}

/// Removes the opaque bits with fewer than `min_px` pixels, and fills the
/// clear holes with fewer than `min_px` pixels that do not touch the edge
/// of the picture, from `source`. Returns the number of pixels changed.
pub fn clean(img: &mut RgbaImage, source: &RgbaImage, min_px: usize) -> usize {
    if min_px == 0 {
        return 0;
    }
    let (w, h) = img.dimensions();
    let mut n = 0;
    let bits = parts(w, h, &|x, y| opaque(img.get_pixel(x, y)));
    for part in bits.iter().filter(|p| p.len() < min_px) {
        for &(x, y) in part {
            img.get_pixel_mut(x, y)[3] = 0;
            n += 1;
        }
    }
    let holes = parts(w, h, &|x, y| !opaque(img.get_pixel(x, y)));
    for part in holes.iter().filter(|p| p.len() < min_px) {
        let edge = part.iter().any(|&(x, y)| x == 0 || y == 0 || x + 1 == w || y + 1 == h);
        if edge {
            continue;
        }
        for &(x, y) in part {
            if x < source.width() && y < source.height() {
                let mut p = *source.get_pixel(x, y);
                p[3] = 255;
                img.put_pixel(x, y, p);
                n += 1;
            }
        }
    }
    n
}

/// What [`erase_shape`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeAction {
    EraseInside,
    /// Keeps only the inside.
    EraseOutside,
    RestoreInside,
}

/// The pixels whose centers are inside the closed `polygon` (pixel
/// coordinates; even-odd rule), row by row.
pub fn inside_polygon(w: u32, h: u32, polygon: &[(f64, f64)]) -> Vec<bool> {
    let mut mask = vec![false; (w * h) as usize];
    let n = polygon.len();
    if n < 3 {
        return mask;
    }
    let mut xs: Vec<f64> = Vec::new();
    for y in 0..h {
        let yc = f64::from(y) + 0.5;
        xs.clear();
        for k in 0..n {
            let (a, b) = (polygon[k], polygon[(k + 1) % n]);
            if (a.1 <= yc) != (b.1 <= yc) {
                xs.push(a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1));
            }
        }
        xs.sort_by(f64::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            let x0 = (pair[0] - 0.5).ceil().clamp(0.0, f64::from(w)) as u32;
            let x1 = (pair[1] - 0.5).ceil().clamp(0.0, f64::from(w)) as u32;
            for x in x0..x1 {
                mask[(y * w + x) as usize] = true;
            }
        }
    }
    mask
}

/// Box and lasso erase: clears the pixels inside `polygon` (pixels), or
/// all outside it, or brings the inside back from `source` (the same
/// size). Returns the number of pixels changed.
pub fn erase_shape(img: &mut RgbaImage, source: &RgbaImage, polygon: &[(f64, f64)], action: ShapeAction) -> usize {
    let (w, h) = img.dimensions();
    let inside = inside_polygon(w, h, polygon);
    let mut n = 0;
    for y in 0..h {
        for x in 0..w {
            let k = (y * w + x) as usize;
            let p = img.get_pixel_mut(x, y);
            match action {
                ShapeAction::EraseInside if inside[k] && p[3] != 0 => {
                    p[3] = 0;
                    n += 1;
                }
                ShapeAction::EraseOutside if !inside[k] && p[3] != 0 => {
                    p[3] = 0;
                    n += 1;
                }
                ShapeAction::RestoreInside if inside[k] && x < source.width() && y < source.height() => {
                    let s = *source.get_pixel(x, y);
                    if *p != s {
                        *p = s;
                        n += 1;
                    }
                }
                _ => {}
            }
        }
    }
    n
}

/// The picture cut to the box of its opaque pixels, and the position of
/// that box in the picture. `None` if all pixels are transparent.
pub fn trim(img: &RgbaImage) -> Option<(RgbaImage, (u32, u32))> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if opaque(p) {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    (x1 > x0).then(|| {
        (
            image::imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image(),
            (x0, y0),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A gray background (with a little noise), a red square in the middle
    /// and a gray dot in the square (the same gray as the background).
    fn picture() -> RgbaImage {
        RgbaImage::from_fn(40, 30, |x, y| {
            let noise = ((x * 7 + y * 13) % 5) as u8;
            if (19..21).contains(&x) && (14..16).contains(&y) {
                Rgba([200, 200, 200, 255])
            } else if (10..30).contains(&x) && (5..25).contains(&y) {
                Rgba([200, 20, 30, 255])
            } else {
                Rgba([198 + noise, 200, 200 - noise, 255])
            }
        })
    }

    fn clear(img: &RgbaImage) -> usize {
        img.pixels().filter(|p| p[3] == 0).count()
    }

    #[test]
    fn edges_remove_the_background_but_not_the_same_color_inside() {
        let mut img = picture();
        let n = remove_edges(&mut img, DEFAULT_TOLERANCE);
        assert_eq!(n, 40 * 30 - 20 * 20);
        assert_eq!(clear(&img), n);
        // The gray dot inside the square stays.
        assert_eq!(img.get_pixel(20, 15)[3], 255);
        // Trim: the red square with its dot.
        let (t, at) = trim(&img).unwrap();
        assert_eq!((t.dimensions(), at), ((20, 20), (10, 5)));
    }

    #[test]
    fn a_background_that_shades_slowly_goes_too() {
        // A wall from mid gray (left) to light gray (right), with noise,
        // and a red square.
        let mut img = RgbaImage::from_fn(200, 60, |x, y| {
            let noise = ((x * 7 + y * 13) % 7) as u8;
            if (90..110).contains(&x) && (20..40).contains(&y) {
                Rgba([200, 20, 30, 255])
            } else {
                let g = 150 + (x * 70 / 200) as u8 + noise;
                Rgba([g, g, g - 4, 255])
            }
        });
        let n = remove_edges(&mut img, DEFAULT_TOLERANCE);
        assert_eq!(n, 200 * 60 - 400);
        assert_eq!(img.get_pixel(100, 30)[3], 255);
    }

    #[test]
    fn a_noisy_busy_background_goes_and_the_face_stays() {
        // Soft blotches of green and gray with grain, and a skin-colored
        // disk with a soft edge (a face) in the middle.
        let img = RgbaImage::from_fn(160, 120, |x, y| {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let grain = (((x * 37 + y * 91) % 23) as f64 - 11.0) * 1.1;
            let blotch = 20.0 * ((fx / 17.0).sin() * (fy / 13.0).cos());
            let r = (fx - 80.0).hypot(fy - 60.0);
            // A soft edge: two pixels from background to skin.
            let t = ((32.0 - r) / 2.0).clamp(0.0, 1.0);
            let bg = [120.0 + blotch, 140.0 + blotch, 115.0 + blotch];
            let skin = [225.0, 180.0, 150.0];
            let c = |i: usize| (bg[i] * (1.0 - t) + skin[i] * t + grain).clamp(0.0, 255.0) as u8;
            Rgba([c(0), c(1), c(2), 255])
        });
        let mut cut = img.clone();
        remove_edges(&mut cut, DEFAULT_TOLERANCE);
        let gone = |x0: u32, y0: u32, x1: u32, y1: u32| {
            let mut n = 0;
            let mut all = 0;
            for y in y0..y1 {
                for x in x0..x1 {
                    all += 1;
                    n += u32::from(cut.get_pixel(x, y)[3] == 0);
                }
            }
            f64::from(n) / f64::from(all)
        };
        // The face (r < 28) stays whole; nearly all the background goes.
        for y in 0..120 {
            for x in 0..160 {
                if (f64::from(x) - 80.0).hypot(f64::from(y) - 60.0) < 28.0 {
                    assert_eq!(cut.get_pixel(x, y)[3], 255, "face pixel {x} {y} removed");
                }
            }
        }
        assert!(gone(0, 0, 160, 20) > 0.97, "{}", gone(0, 0, 160, 20));
        assert!(gone(0, 0, 30, 120) > 0.97, "{}", gone(0, 0, 30, 120));
    }

    #[test]
    fn the_removal_does_not_drift_into_dark_hair() {
        // A green wall; dark hair whose strands fan out over the wall (a
        // band of stripes), then solid hair.
        let img = RgbaImage::from_fn(120, 80, |x, y| {
            let grain = (((x * 37 + y * 91) % 23) as f64 - 11.0) * 0.8;
            let wall = [120.0, 140.0, 115.0];
            let hair = [50.0, 35.0, 25.0];
            let c = if y >= 50 || (y >= 30 && x % 3 == 0) { hair } else { wall };
            let v = |i: usize| (c[i] + grain).clamp(0.0, 255.0) as u8;
            Rgba([v(0), v(1), v(2), 255])
        });
        let mut cut = img.clone();
        remove_edges(&mut cut, DEFAULT_TOLERANCE);
        // The solid hair stays.
        for y in 52..80 {
            for x in 2..118 {
                assert_eq!(cut.get_pixel(x, y)[3], 255, "hair pixel {x} {y} removed");
            }
        }
        // The wall above the strands goes.
        assert!((0..120).all(|x| cut.get_pixel(x, 10)[3] == 0));
    }

    #[test]
    fn box_and_lasso_erase() {
        let source = picture();
        let mut img = source.clone();
        // A box over the left quarter: 10 x 30 pixels go.
        let rect = [(0.0, 0.0), (10.0, 0.0), (10.0, 30.0), (0.0, 30.0)];
        assert_eq!(erase_shape(&mut img, &source, &rect, ShapeAction::EraseInside), 300);
        assert_eq!(img.get_pixel(5, 5)[3], 0);
        assert_eq!(img.get_pixel(15, 5)[3], 255);
        // Restore half of it.
        let half = [(0.0, 0.0), (5.0, 0.0), (5.0, 30.0), (0.0, 30.0)];
        assert_eq!(erase_shape(&mut img, &source, &half, ShapeAction::RestoreInside), 150);
        // Keep only a triangle round the square: the outside goes.
        let mut img = source.clone();
        let tri = [(0.0, -5.0), (40.0, -5.0), (20.0, 40.0)];
        erase_shape(&mut img, &source, &tri, ShapeAction::EraseOutside);
        assert_eq!(img.get_pixel(20, 15)[3], 255);
        assert_eq!(img.get_pixel(1, 28)[3], 0);
        assert_eq!(erase_shape(&mut img, &source, &tri[..2], ShapeAction::EraseInside), 0);
    }

    #[test]
    fn brush_shrink_and_clean() {
        let source = picture();
        let mut img = source.clone();
        remove_edges(&mut img, DEFAULT_TOLERANCE);
        // A brush clears a round spot in the square; a restore brush brings it back.
        brush(&mut img, &source, 15.0, 10.0, 2.0, false);
        assert_eq!(img.get_pixel(15, 10)[3], 0);
        brush(&mut img, &source, 15.0, 10.0, 3.0, true);
        assert_eq!(*img.get_pixel(15, 10), *source.get_pixel(15, 10));
        // A speck of background left on the wall, and a hole in the square.
        img.get_pixel_mut(2, 2)[3] = 255;
        img.get_pixel_mut(15, 10)[3] = 0;
        assert_eq!(clean(&mut img, &source, 5), 2);
        assert_eq!(img.get_pixel(2, 2)[3], 0);
        assert_eq!(img.get_pixel(15, 10)[3], 255);
        // Shrink: one pixel off each side of the 20 x 20 square.
        assert_eq!(shrink_edge(&mut img, 1), 20 * 20 - 18 * 18);
        assert_eq!(trim(&img).unwrap().0.dimensions(), (18, 18));
    }

    #[test]
    fn a_click_removes_the_area_or_the_color_everywhere() {
        let mut img = picture();
        // The dot alone (connected).
        assert_eq!(remove_at(&mut img, 20, 15, DEFAULT_TOLERANCE, true), 4);
        assert_eq!(img.get_pixel(0, 0)[3], 255);
        // The gray everywhere (not connected).
        let mut img = picture();
        assert_eq!(remove_at(&mut img, 0, 0, DEFAULT_TOLERANCE, false), 40 * 30 - 400 + 4);
        // A click on a clear pixel, or off the picture, does nothing.
        assert_eq!(remove_at(&mut img, 0, 0, DEFAULT_TOLERANCE, true), 0);
        assert_eq!(remove_at(&mut img, 99, 0, DEFAULT_TOLERANCE, true), 0);
        assert!(trim(&RgbaImage::new(3, 3)).is_none());
    }
}
