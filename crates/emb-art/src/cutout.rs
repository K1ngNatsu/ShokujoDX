//! Cut out the subject (ADR 0031): the user drags a box round the
//! subject, and marks parts to keep or to remove if needed; the rest is
//! found from the colors and the edges of the picture.
//!
//! The method is GrabCut (Rother, Kolmogorov and Blake, "GrabCut:
//! Interactive Foreground Extraction using Iterated Graph Cuts", SIGGRAPH
//! 2004), our own implementation of the published method:
//!
//! 1. The pixels outside the box, the pixels marked "remove" and clear
//!    pixels are background for sure; the pixels marked "keep" are subject
//!    for sure; the others start as subject.
//! 2. Each of the subject and the background gets a color model: a mix of
//!    `COMPONENTS` Gaussians in RGB.
//! 3. A minimum graph cut (`maxflow`) labels each pixel: a pixel costs
//!    less on the side whose model fits its color, and neighbors with
//!    similar colors cost more to separate, so the cut follows edges.
//! 4. Steps 2 and 3 are done `ROUNDS` times.
//!
//! The work is done on the picture at most [`WORK_PX`] pixels on its long
//! side, so it stays fast; then at the full size, the pixels near the edge
//! of the result are decided by the color models one by one. Small
//! islands and holes are cleaned up. Deterministic (ADR 0003).

use emb_geom::math;
use image::RgbaImage;

use crate::maxflow::Graph;

/// The work picture has at most this many pixels on its long side.
pub const WORK_PX: u32 = 320;
const COMPONENTS: usize = 5;
const ROUNDS: usize = 5;
/// The weight of the smoothness (edges) against the colors.
const GAMMA: f64 = 50.0;
/// The cost that makes a sure pixel stay as it is.
const SURE: f64 = 1e9;
/// Islands and holes smaller than this part of the picture are cleaned up.
const ISLAND_PART: f64 = 0.001;

/// A mark of the user on a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    Keep,
    Remove,
}

#[derive(Debug, Clone, Copy)]
struct Gauss {
    /// The weight divided by the normalization of the density.
    k: f64,
    mean: [f64; 3],
    inv: [[f64; 3]; 3],
}

#[derive(Debug, Clone, Default)]
struct Gmm(Vec<Gauss>);

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn inv3(m: &[[f64; 3]; 3], det: f64) -> [[f64; 3]; 3] {
    let c = |r0: usize, c0: usize, r1: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    [
        [c(1, 1, 2, 2) / det, -c(0, 1, 2, 2) / det, c(0, 1, 1, 2) / det],
        [-c(1, 0, 2, 2) / det, c(0, 0, 2, 2) / det, -c(0, 0, 1, 2) / det],
        [c(1, 0, 2, 1) / det, -c(0, 0, 2, 1) / det, c(0, 0, 1, 1) / det],
    ]
}

impl Gauss {
    fn density(&self, z: [f64; 3]) -> f64 {
        let d = [z[0] - self.mean[0], z[1] - self.mean[1], z[2] - self.mean[2]];
        let mut q = 0.0;
        for r in 0..3 {
            for c in 0..3 {
                q += d[r] * self.inv[r][c] * d[c];
            }
        }
        self.k * math::exp(-0.5 * q)
    }
}

impl Gmm {
    /// The model of the colors `z[i]` with the component `comp[i]` of each.
    fn fit(z: &[[f64; 3]], comp: &[usize]) -> Gmm {
        let total = z.len().max(1) as f64;
        let mut out = Vec::new();
        for c in 0..COMPONENTS {
            let pts: Vec<[f64; 3]> = z.iter().zip(comp).filter(|(_, k)| **k == c).map(|(p, _)| *p).collect();
            if pts.is_empty() {
                continue;
            }
            let n = pts.len() as f64;
            let mut mean = [0.0; 3];
            for p in &pts {
                for i in 0..3 {
                    mean[i] += p[i] / n;
                }
            }
            let mut cov = [[0.0; 3]; 3];
            for p in &pts {
                for r in 0..3 {
                    for s in 0..3 {
                        cov[r][s] += (p[r] - mean[r]) * (p[s] - mean[s]) / n;
                    }
                }
            }
            for (i, row) in cov.iter_mut().enumerate() {
                row[i] += 1.0;
            }
            let det = det3(&cov).max(1e-9);
            out.push(Gauss {
                k: (n / total) / ((2.0 * std::f64::consts::PI).powi(3) * det).sqrt(),
                mean,
                inv: inv3(&cov, det),
            });
        }
        Gmm(out)
    }

    fn prob(&self, z: [f64; 3]) -> f64 {
        self.0.iter().map(|g| g.density(z)).sum()
    }

    /// The component that fits `z` best.
    fn best(&self, z: [f64; 3]) -> usize {
        (0..self.0.len())
            .max_by(|&a, &b| self.0[a].density(z).total_cmp(&self.0[b].density(z)))
            .unwrap_or(0)
    }

    /// The cost of a color in the model.
    fn cost(&self, z: [f64; 3]) -> f64 {
        -math::ln(self.prob(z).max(1e-300))
    }
}

/// K-means start of the components: the first center is the mean, then
/// each time the color farthest from the centers so far.
fn kmeans(z: &[[f64; 3]]) -> Vec<usize> {
    if z.is_empty() {
        return Vec::new();
    }
    let d2 = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>();
    let n = z.len() as f64;
    let mut centers = vec![[0, 1, 2].map(|i| z.iter().map(|p| p[i]).sum::<f64>() / n)];
    while centers.len() < COMPONENTS {
        let far = z
            .iter()
            .copied()
            .max_by(|a, b| {
                let da = centers.iter().map(|c| d2(*a, *c)).fold(f64::MAX, f64::min);
                let db = centers.iter().map(|c| d2(*b, *c)).fold(f64::MAX, f64::min);
                da.total_cmp(&db)
            })
            .unwrap_or(centers[0]);
        centers.push(far);
    }
    let mut comp = vec![0; z.len()];
    for _ in 0..8 {
        for (k, p) in z.iter().enumerate() {
            comp[k] = (0..centers.len())
                .min_by(|&a, &b| d2(*p, centers[a]).total_cmp(&d2(*p, centers[b])))
                .unwrap_or(0);
        }
        for (c, center) in centers.iter_mut().enumerate() {
            let (mut s, mut m) = ([0.0; 3], 0.0);
            for (p, _) in z.iter().zip(&comp).filter(|(_, k)| **k == c) {
                for i in 0..3 {
                    s[i] += p[i];
                }
                m += 1.0;
            }
            if m > 0.0 {
                *center = s.map(|v| v / m);
            }
        }
    }
    comp
}

fn rgb(p: &image::Rgba<u8>) -> [f64; 3] {
    [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]
}

/// The subject of `img`: `true` for each pixel of it (row by row). `rect`
/// is the box round the subject, `[x0, y0, x1, y1]` in pixels (`None`: the
/// whole picture but its edge pixels); `marks` has a mark for each pixel,
/// or is empty. `None` if there is nothing to cut (no subject or no
/// background).
pub fn cutout(img: &RgbaImage, rect: Option<[f64; 4]>, marks: &[Mark]) -> Option<Vec<bool>> {
    let (w, h) = img.dimensions();
    if w < 2 || h < 2 {
        return None;
    }
    let rect = rect.unwrap_or([1.0, 1.0, f64::from(w) - 1.0, f64::from(h) - 1.0]);
    let mark = |x: u32, y: u32| marks.get((y * w + x) as usize).copied().unwrap_or_default();
    // The work picture.
    let f = (f64::from(w.max(h)) / f64::from(WORK_PX)).max(1.0);
    let (sw, sh) = (
        ((f64::from(w) / f).ceil() as u32).max(2),
        ((f64::from(h) / f).ceil() as u32).max(2),
    );
    let n = (sw * sh) as usize;
    // Mean colors, sure labels (1 subject, -1 background) of each work pixel.
    let mut sum = vec![[0.0f64; 4]; n];
    let mut keep = vec![false; n];
    let mut remove = vec![false; n];
    let cell = |x: u32, y: u32| {
        let cx = ((f64::from(x) / f) as u32).min(sw - 1);
        let cy = ((f64::from(y) / f) as u32).min(sh - 1);
        (cy * sw + cx) as usize
    };
    for y in 0..h {
        for x in 0..w {
            let k = cell(x, y);
            let p = img.get_pixel(x, y);
            let m = mark(x, y);
            if p[3] >= 128 || m == Mark::Keep {
                for i in 0..3 {
                    sum[k][i] += f64::from(p[i]);
                }
                sum[k][3] += 1.0;
            }
            match m {
                Mark::Keep => keep[k] = true,
                Mark::Remove => remove[k] = true,
                Mark::None => {}
            }
        }
    }
    let z: Vec<[f64; 3]> = sum
        .iter()
        .map(|s| {
            if s[3] > 0.0 {
                [s[0] / s[3], s[1] / s[3], s[2] / s[3]]
            } else {
                [0.0; 3]
            }
        })
        .collect();
    let in_rect = |k: usize| {
        let (x, y) = ((k as u32 % sw) as f64 + 0.5, (k as u32 / sw) as f64 + 0.5);
        let (x, y) = (x * f, y * f);
        x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
    };
    // 1: subject for sure, -1: background for sure, 0: to find.
    let sure: Vec<i8> = (0..n)
        .map(|k| {
            if keep[k] && !remove[k] {
                1
            } else if remove[k] || sum[k][3] == 0.0 || !in_rect(k) {
                -1
            } else {
                0
            }
        })
        .collect();
    let mut fg: Vec<bool> = sure.iter().map(|&s| s >= 0).collect();
    if !fg.iter().any(|&v| v) || fg.iter().all(|&v| v) {
        return None;
    }

    // The edge weights between neighbors (right, down and both diagonals).
    let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
    for y in 0..sh {
        for x in 0..sw {
            let k = (y * sw + x) as usize;
            let mut add = |qx: i64, qy: i64, dist: f64| {
                if qx >= 0 && qy >= 0 && (qx as u32) < sw && (qy as u32) < sh {
                    pairs.push((k, (qy as u32 * sw + qx as u32) as usize, dist));
                }
            };
            let (xi, yi) = (i64::from(x), i64::from(y));
            add(xi + 1, yi, 1.0);
            add(xi, yi + 1, 1.0);
            add(xi + 1, yi + 1, std::f64::consts::SQRT_2);
            add(xi - 1, yi + 1, std::f64::consts::SQRT_2);
        }
    }
    let d2 = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>();
    let mean_d2 = pairs.iter().map(|&(a, b, _)| d2(z[a], z[b])).sum::<f64>() / pairs.len().max(1) as f64;
    let beta = if mean_d2 > 0.0 { 1.0 / (2.0 * mean_d2) } else { 0.0 };
    let weights: Vec<f64> = pairs
        .iter()
        .map(|&(a, b, dist)| GAMMA * math::exp(-beta * d2(z[a], z[b])) / dist)
        .collect();

    let split = |fg: &[bool], side: bool| -> (Vec<usize>, Vec<[f64; 3]>) {
        let idx: Vec<usize> = (0..n).filter(|&k| fg[k] == side && sum[k][3] > 0.0).collect();
        let pts = idx.iter().map(|&k| z[k]).collect();
        (idx, pts)
    };
    let (_, fz) = split(&fg, true);
    let (_, bz) = split(&fg, false);
    if fz.is_empty() || bz.is_empty() {
        return None;
    }
    let mut fgm = Gmm::fit(&fz, &kmeans(&fz));
    let mut bgm = Gmm::fit(&bz, &kmeans(&bz));
    for _ in 0..ROUNDS {
        // The cut.
        let mut g = Graph::new(n);
        for k in 0..n {
            match sure[k] {
                1 => g.add_terminal(k, SURE, 0.0),
                -1 => g.add_terminal(k, 0.0, SURE),
                _ => g.add_terminal(k, bgm.cost(z[k]), fgm.cost(z[k])),
            }
        }
        for (&(a, b, _), &wt) in pairs.iter().zip(&weights) {
            g.add_edge(a, b, wt, wt);
        }
        g.maxflow();
        for (k, v) in fg.iter_mut().enumerate() {
            *v = g.on_source_side(k);
        }
        // New models from the new labels.
        let (_, fz) = split(&fg, true);
        let (_, bz) = split(&fg, false);
        if fz.is_empty() || bz.is_empty() {
            break;
        }
        let fc: Vec<usize> = fz.iter().map(|p| fgm.best(*p)).collect();
        let bc: Vec<usize> = bz.iter().map(|p| bgm.best(*p)).collect();
        fgm = Gmm::fit(&fz, &fc);
        bgm = Gmm::fit(&bz, &bc);
    }

    // The full size: the work label, and near its edge the colors decide.
    let at = |x: i64, y: i64| -> f64 {
        let (x, y) = (x.clamp(0, i64::from(sw) - 1), y.clamp(0, i64::from(sh) - 1));
        if fg[(y as u32 * sw + x as u32) as usize] {
            1.0
        } else {
            0.0
        }
    };
    let mut out = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let k = (y * w + x) as usize;
            let p = img.get_pixel(x, y);
            let m = mark(x, y);
            if m != Mark::None {
                out[k] = m == Mark::Keep;
                continue;
            }
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            if p[3] < 128 || fx < rect[0] || fx > rect[2] || fy < rect[1] || fy > rect[3] {
                continue;
            }
            // Bilinear value of the work labels.
            let (gx, gy) = (fx / f - 0.5, fy / f - 0.5);
            let (x0, y0) = (gx.floor(), gy.floor());
            let (tx, ty) = (gx - x0, gy - y0);
            let (xi, yi) = (x0 as i64, y0 as i64);
            let soft = (at(xi, yi) * (1.0 - tx) + at(xi + 1, yi) * tx) * (1.0 - ty)
                + (at(xi, yi + 1) * (1.0 - tx) + at(xi + 1, yi + 1) * tx) * ty;
            out[k] = if soft > 0.95 {
                true
            } else if soft < 0.05 {
                false
            } else {
                let c = rgb(p);
                fgm.prob(c) * soft > bgm.prob(c) * (1.0 - soft)
            };
        }
    }
    // Clean up small islands of subject and small holes in it.
    let min = ((f64::from(w) * f64::from(h)) * ISLAND_PART).ceil() as usize;
    for (value, parts) in [
        (true, crate::background::parts(w, h, &|x, y| out[(y * w + x) as usize])),
        (
            false,
            crate::background::parts(w, h, &|x, y| !out[(y * w + x) as usize]),
        ),
    ] {
        for part in parts.iter().filter(|p| p.len() < min) {
            let marked = part.iter().any(|&(x, y)| mark(x, y) != Mark::None);
            let edge = part.iter().any(|&(x, y)| x == 0 || y == 0 || x + 1 == w || y + 1 == h);
            if marked || (!value && edge) {
                continue;
            }
            for &(x, y) in part {
                out[(y * w + x) as usize] = !value;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A busy background of soft green and gray blotches with grain, and a
    /// skin-colored disk (r = 25) with dark "hair" on top.
    fn person() -> RgbaImage {
        RgbaImage::from_fn(140, 110, |x, y| {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let grain = (((x * 37 + y * 91) % 23) as f64 - 11.0) * 1.0;
            let r = (fx - 70.0).hypot(fy - 60.0);
            let c = if r < 25.0 && fy < 45.0 {
                [55.0, 38.0, 28.0]
            } else if r < 25.0 {
                [225.0, 180.0, 150.0]
            } else {
                let b = 25.0 * ((fx / 9.0).sin() * (fy / 7.0).cos());
                [120.0 + b, 145.0 + b, 110.0 + b]
            };
            let v = |i: usize| (c[i] + grain).clamp(0.0, 255.0) as u8;
            Rgba([v(0), v(1), v(2), 255])
        })
    }

    fn truth(x: u32, y: u32) -> bool {
        (f64::from(x) + 0.5 - 70.0).hypot(f64::from(y) + 0.5 - 60.0) < 25.0
    }

    /// The part of the pixels that the mask labels as `truth` says.
    fn right(mask: &[bool], w: u32, h: u32) -> f64 {
        let ok = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| mask[(y * w + x) as usize] == truth(x, y))
            .count();
        ok as f64 / f64::from(w * h)
    }

    #[test]
    fn a_box_round_the_person_cuts_it_out() {
        let img = person();
        let mask = cutout(&img, Some([35.0, 25.0, 105.0, 95.0]), &[]).unwrap();
        let r = right(&mask, 140, 110);
        assert!(r > 0.985, "{r}");
        // The hair and the face are both kept.
        assert!(mask[(40 * 140 + 70) as usize] && mask[(65 * 140 + 70) as usize]);
        // The same each time.
        assert_eq!(cutout(&img, Some([35.0, 25.0, 105.0, 95.0]), &[]).unwrap(), mask);
    }

    #[test]
    fn marks_correct_the_cut() {
        let img = person();
        // Remove marks on the face: the face goes, the hair stays.
        let mut marks = vec![Mark::None; 140 * 110];
        for y in 55..75 {
            for x in 55..85 {
                if truth(x, y) {
                    marks[(y * 140 + x) as usize] = Mark::Remove;
                }
            }
        }
        let mask = cutout(&img, Some([35.0, 25.0, 105.0, 95.0]), &marks).unwrap();
        assert!(!mask[(65 * 140 + 70) as usize]);
        assert!(mask[(40 * 140 + 70) as usize]);
        // A keep mark outside the box is kept.
        let mut marks = vec![Mark::None; 140 * 110];
        marks[(5 * 140 + 5) as usize] = Mark::Keep;
        let mask = cutout(&img, Some([35.0, 25.0, 105.0, 95.0]), &marks).unwrap();
        assert!(mask[(5 * 140 + 5) as usize]);
        // No room for a background: nothing to cut.
        assert!(cutout(&img, Some([-10.0, -10.0, 200.0, 200.0]), &[]).is_none());
    }

    #[test]
    fn a_large_picture_works_on_a_smaller_copy() {
        // Twice the size: the work picture is smaller than the picture.
        let small = person();
        let img = RgbaImage::from_fn(560, 440, |x, y| *small.get_pixel(x / 4, y / 4));
        let mask = cutout(&img, Some([140.0, 100.0, 420.0, 380.0]), &[]).unwrap();
        let ok = (0..440u32)
            .flat_map(|y| (0..560u32).map(move |x| (x, y)))
            .filter(|&(x, y)| mask[(y * 560 + x) as usize] == truth(x / 4, y / 4))
            .count();
        assert!(ok as f64 / (560.0 * 440.0) > 0.98, "{}", ok as f64 / (560.0 * 440.0));
    }
}
