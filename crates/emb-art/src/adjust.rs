//! Brightness, contrast and sharpness of a picture.

use image::{Rgba, RgbaImage};

/// The settings of [`adjust`]. All zero: no change.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Adjust {
    /// -1 (black) to 1 (white).
    pub brightness: f64,
    /// -1 (flat gray) to 1 (strong).
    pub contrast: f64,
    /// 0 (none) to 2 (strong): an unsharp mask of the 3 × 3 neighborhood.
    pub sharpen: f64,
}

impl Adjust {
    pub fn is_none(&self) -> bool {
        *self == Adjust::default()
    }
}

/// The spread of the colors that count as one in [`smooth`] (RGB units).
const SMOOTH_COLOR_SIGMA: f64 = 20.0;
/// The spread of the distances in [`smooth`], pixels.
const SMOOTH_PLACE_SIGMA: f64 = 1.5;

/// Edge-preserving smoothing (P12 Milestone 3, a bilateral filter, `passes`
/// times): each opaque pixel takes the mean of the opaque pixels within 2
/// pixels, each weighted by its distance and by how near its color is. A
/// pixel across an edge has another color and counts little, so flat areas
/// get even (JPEG blocks and rings, the grain of a scan) and the edges stay
/// sharp. Clear pixels stay clear.
pub fn smooth(img: &RgbaImage, passes: u32) -> RgbaImage {
    const R: i64 = 2;
    // The weights as tables: of the place, and of the squared color distance.
    let place: Vec<f64> = (-R..=R)
        .flat_map(|dy| (-R..=R).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            emb_geom::math::exp(-((dx * dx + dy * dy) as f64) / (2.0 * SMOOTH_PLACE_SIGMA * SMOOTH_PLACE_SIGMA))
        })
        .collect();
    let far = (6.0 * SMOOTH_COLOR_SIGMA).powi(2) as usize;
    let color: Vec<f64> = (0..=far)
        .map(|d2| emb_geom::math::exp(-(d2 as f64) / (2.0 * SMOOTH_COLOR_SIGMA * SMOOTH_COLOR_SIGMA)))
        .collect();
    let (w, h) = img.dimensions();
    let mut cur = img.clone();
    for _ in 0..passes {
        let src = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let p = *src.get_pixel(x, y);
                if p[3] < 128 {
                    continue;
                }
                let (mut sum, mut total) = ([0.0; 3], 0.0);
                for dy in -R..=R {
                    for dx in -R..=R {
                        let (qx, qy) = (i64::from(x) + dx, i64::from(y) + dy);
                        if qx < 0 || qy < 0 || qx >= i64::from(w) || qy >= i64::from(h) {
                            continue;
                        }
                        let q = src.get_pixel(qx as u32, qy as u32);
                        if q[3] < 128 {
                            continue;
                        }
                        let d2: usize = (0..3)
                            .map(|c| (i64::from(p[c]) - i64::from(q[c])).pow(2) as usize)
                            .sum();
                        let Some(&wc) = color.get(d2) else { continue };
                        let wt = wc * place[((dy + R) * (2 * R + 1) + dx + R) as usize];
                        for c in 0..3 {
                            sum[c] += wt * f64::from(q[c]);
                        }
                        total += wt;
                    }
                }
                let out = cur.get_pixel_mut(x, y);
                for c in 0..3 {
                    out[c] = (sum[c] / total).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    cur
}

/// Noise reduction: each opaque pixel takes the median of each channel of
/// the opaque pixels within `radius` (a square window). Clear pixels stay
/// clear. A median keeps edges sharp, as a blur does not.
pub fn median(img: &RgbaImage, radius: u32) -> RgbaImage {
    if radius == 0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let r = radius as i64;
    let mut out = img.clone();
    let mut ch: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y)[3] < 128 {
                continue;
            }
            for c in &mut ch {
                c.clear();
            }
            for dy in -r..=r {
                for dx in -r..=r {
                    let (qx, qy) = (i64::from(x) + dx, i64::from(y) + dy);
                    if qx < 0 || qy < 0 || qx >= i64::from(w) || qy >= i64::from(h) {
                        continue;
                    }
                    let q = img.get_pixel(qx as u32, qy as u32);
                    if q[3] >= 128 {
                        for (c, v) in ch.iter_mut().zip(q.0) {
                            c.push(v);
                        }
                    }
                }
            }
            let p = out.get_pixel_mut(x, y);
            for (k, c) in ch.iter_mut().enumerate() {
                c.sort_unstable();
                p[k] = c[c.len() / 2];
            }
        }
    }
    out
}

/// The picture with the settings applied. Transparency stays.
pub fn adjust(img: &RgbaImage, a: &Adjust) -> RgbaImage {
    let mut out = if a.sharpen > 0.0 {
        sharpen(img, a.sharpen)
    } else {
        img.clone()
    };
    if a.brightness != 0.0 || a.contrast != 0.0 {
        let k = (1.0 + a.contrast.clamp(-1.0, 1.0)).max(0.0);
        let b = a.brightness.clamp(-1.0, 1.0);
        for p in out.pixels_mut() {
            for c in 0..3 {
                let v = f64::from(p[c]) / 255.0;
                let v = (v - 0.5) * k + 0.5 + b;
                p[c] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    out
}

/// Unsharp mask: each color plus `amount` times its difference from the
/// mean of its 3 × 3 neighborhood.
fn sharpen(img: &RgbaImage, amount: f64) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f64; 3];
            let mut n = 0.0;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                    if nx < 0 || ny < 0 || nx >= i64::from(w) || ny >= i64::from(h) {
                        continue;
                    }
                    let q = img.get_pixel(nx as u32, ny as u32);
                    for c in 0..3 {
                        sum[c] += f64::from(q[c]);
                    }
                    n += 1.0;
                }
            }
            let p = img.get_pixel(x, y);
            let mut q = *p;
            for c in 0..3 {
                let v = f64::from(p[c]);
                q[c] = (v + amount * (v - sum[c] / n)).round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(x, y, Rgba(q.0));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_removes_specks_and_keeps_edges() {
        // Left half black, right half white, with a white speck on the black.
        let mut img = RgbaImage::from_fn(10, 10, |x, _| {
            if x < 5 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        img.put_pixel(2, 5, Rgba([255, 255, 255, 255]));
        let m = median(&img, 1);
        assert_eq!(m.get_pixel(2, 5)[0], 0);
        assert_eq!(m.get_pixel(4, 5)[0], 0);
        assert_eq!(m.get_pixel(5, 5)[0], 255);
        assert_eq!(median(&img, 0), img);
    }

    fn gray(v: u8) -> RgbaImage {
        RgbaImage::from_pixel(4, 4, Rgba([v, v, v, 255]))
    }

    #[test]
    fn brightness_and_contrast() {
        let img = gray(100);
        assert_eq!(adjust(&img, &Adjust::default()), img);
        let brighter = adjust(
            &img,
            &Adjust {
                brightness: 0.2,
                ..Adjust::default()
            },
        );
        assert_eq!(brighter.get_pixel(0, 0)[0], 151);
        let flat = adjust(
            &img,
            &Adjust {
                contrast: -1.0,
                ..Adjust::default()
            },
        );
        assert_eq!(flat.get_pixel(0, 0)[0], 128, "no contrast: middle gray");
        assert_eq!(flat.get_pixel(0, 0)[3], 255, "alpha stays");
    }

    #[test]
    fn sharpen_makes_an_edge_stronger() {
        let mut img = gray(100);
        for y in 0..4 {
            for x in 2..4 {
                img.put_pixel(x, y, Rgba([200, 200, 200, 255]));
            }
        }
        let s = adjust(
            &img,
            &Adjust {
                sharpen: 1.0,
                ..Adjust::default()
            },
        );
        assert!(s.get_pixel(1, 1)[0] < 100, "the dark side of the edge is darker");
        assert!(s.get_pixel(2, 1)[0] > 200, "the light side is lighter");
        assert_eq!(s.get_pixel(0, 1)[0], 100, "far from the edge: no change");
    }

    #[test]
    fn smooth_makes_flat_areas_even_and_keeps_the_edge() {
        // Dark on the left, light on the right, both with grain; a clear row.
        let grain = |x: u32, y: u32| ((x * 7 + y * 13) % 9) as i32 * 3 - 12;
        let img = RgbaImage::from_fn(20, 20, |x, y| {
            if y == 0 {
                return Rgba([0, 0, 0, 0]);
            }
            let base = if x < 10 { 60 } else { 200 };
            let v = (base + grain(x, y)).clamp(0, 255) as u8;
            Rgba([v, v, v, 255])
        });
        let out = smooth(&img, 2);
        let spread = |im: &RgbaImage, x0: u32, x1: u32| {
            let v: Vec<i32> = (x0..x1)
                .flat_map(|x| (3..17).map(move |y| (x, y)))
                .map(|(x, y)| i32::from(im.get_pixel(x, y)[0]))
                .collect();
            v.iter().max().unwrap() - v.iter().min().unwrap()
        };
        assert!(
            spread(&out, 2, 8) * 2 < spread(&img, 2, 8),
            "{} {}",
            spread(&out, 2, 8),
            spread(&img, 2, 8)
        );
        // The edge: the pixels next to it keep their side.
        for y in 3..17 {
            assert!(out.get_pixel(9, y)[0] < 90 && out.get_pixel(10, y)[0] > 170);
        }
        assert_eq!(out.get_pixel(5, 0)[3], 0);
        assert_eq!(smooth(&img, 0), img);
    }
}
