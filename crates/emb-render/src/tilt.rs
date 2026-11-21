//! The tilt view (P18 view plan, Milestone 8, ADR 0129): the embroidery
//! seen from an angle, with its real heights or with the heights made
//! larger, made with the processor.
//!
//! A [`Relief`] is the picture of a surface in a light (`sheen::light`)
//! and the height of each of its pixels. [`project`] shows it from a
//! [`Camera`]: the design turns on the screen, then the view tilts toward
//! the back of the design (the bottom of the screen comes nearer). The
//! projection is parallel (no perspective), so a column of the picture is
//! one line across the relief. Each column is made from the front to the
//! back: a point of the relief fills the rows from its place down to the
//! highest row that a point in front of it filled (at most to the fabric
//! under it), so a high part hides what is behind it. The rows under the
//! top of a point (its front face) are a little darker.
//!
//! The light stays in the room when the design turns: the app makes the
//! relief again in the light `Lighting::turned` (the sheen of a satin
//! changes as on a hoop that turns under a lamp).

use emb_geom::{BBox, Point, math};
use emb_model::Rgb;

use crate::fabric::FabricLook;
use crate::raster::Picture;
use crate::surface::Surface;
use crate::thread3d::Lighting;

/// The largest tilt, degrees from straight above.
pub const MAX_TILT_DEG: f64 = 75.0;
/// The most that the heights can be made larger.
pub const MAX_LIFT: f64 = 10.0;
/// The light of the front face of a point (the rows under its top), as a
/// part of the light of its top.
const FACE: f64 = 0.72;

/// The picture of a surface in a light and the heights of its pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Relief {
    /// The area of the pixels, mm.
    pub area: BBox,
    pub px_per_mm: f64,
    pub width: u32,
    pub height: u32,
    /// The colors of the screen, with alpha (0: nothing there), row by row.
    pub rgba: Vec<[u8; 4]>,
    /// The height of each pixel, mm.
    pub heights: Vec<f32>,
    /// The highest point, mm.
    pub top: f32,
}

impl Relief {
    /// The relief of the surface `s` in the light `lighting` on `fabric`
    /// (else the pixels with no thread are clear: lace, or no fabric).
    pub fn of(s: &Surface, lighting: &Lighting, fabric: Option<(Rgb, FabricLook)>) -> Relief {
        let pic = crate::sheen::light(s, lighting, fabric);
        let (w, h) = (s.width, s.height);
        let mut heights = vec![0.0f32; (w * h) as usize];
        for t in &s.tiles {
            for row in 0..t.h {
                let at = ((t.y0 + row) * w + t.x0) as usize;
                let from = (row * t.w) as usize;
                heights[at..at + t.w as usize].copy_from_slice(&t.height[from..from + t.w as usize]);
            }
        }
        Relief {
            area: s.area,
            px_per_mm: s.px_per_mm,
            width: w,
            height: h,
            rgba: pic.rgba.as_chunks::<4>().0.to_vec(),
            top: heights.iter().copied().fold(0.0, f32::max),
            heights,
        }
    }
}

/// The point of view of the tilt view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The point of the design at the middle of the picture, mm.
    pub center: Point,
    /// Pixels for each mm of the design (across the view).
    pub px_per_mm: f64,
    /// The turn of the design on the screen, degrees counterclockwise.
    pub turn_deg: f64,
    /// The tilt, degrees from straight above (0 to [`MAX_TILT_DEG`]).
    pub tilt_deg: f64,
    /// The heights are this many times larger (1: the real heights).
    pub lift: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            center: Point::ZERO,
            px_per_mm: 4.0,
            turn_deg: 0.0,
            tilt_deg: 50.0,
            lift: 3.0,
        }
    }
}

impl Camera {
    /// The place on the picture (`w` × `h` pixels) of the point `p` at the
    /// height `z` mm (pixels, from the top left corner).
    pub fn to_picture(&self, p: Point, z: f64, w: u32, h: u32) -> (f64, f64) {
        let (st, ct) = math::sin_cos_deg(self.turn_deg);
        let (sv, cv) = math::sin_cos_deg(self.tilt_deg.clamp(0.0, MAX_TILT_DEG));
        let (dx, dy) = (p.x - self.center.x, p.y - self.center.y);
        let xr = dx * ct + dy * st;
        let yr = -dx * st + dy * ct;
        (
            f64::from(w) / 2.0 + self.px_per_mm * xr,
            f64::from(h) / 2.0 + self.px_per_mm * (yr * cv - z * self.lift * sv),
        )
    }
}

/// The picture of `relief` from `camera`, `w` × `h` pixels, with
/// `background` where the relief is not (see the module text), on all
/// processors.
pub fn project(relief: &Relief, camera: &Camera, w: u32, h: u32, background: [u8; 4]) -> Picture {
    let (w, h) = (w.max(1), h.max(1));
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(w as usize)
        .max(1);
    let per = (w as usize).div_ceil(workers);
    // Each thread makes a band of columns, column by column.
    let bands: Vec<Vec<[u8; 4]>> = emb_geom::threads::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|k| {
                let cols = (k * per) as u32..(((k + 1) * per) as u32).min(w);
                scope.spawn(move || {
                    let mut out = vec![background; cols.len() * h as usize];
                    for (i, x) in cols.enumerate() {
                        column(relief, camera, w, h, x, &mut out[i * h as usize..(i + 1) * h as usize]);
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|t| t.join().expect("a thread of the tilt view does not stop with an error"))
            .collect()
    });
    let mut rgba = vec![0u8; (w * h) as usize * 4];
    for (k, band) in bands.iter().enumerate() {
        let x0 = k * per;
        for (i, col) in band.chunks(h as usize).enumerate() {
            for (y, px) in col.iter().enumerate() {
                let at = (y * w as usize + x0 + i) * 4;
                rgba[at..at + 4].copy_from_slice(px);
            }
        }
    }
    Picture {
        width: w,
        height: h,
        rgba,
    }
}

/// The column `x` of the picture (`out`, row by row).
fn column(relief: &Relief, cam: &Camera, w: u32, h: u32, x: u32, out: &mut [[u8; 4]]) {
    let s = cam.px_per_mm.max(1e-6);
    let (st, ct) = math::sin_cos_deg(cam.turn_deg);
    let (sv, cv) = math::sin_cos_deg(cam.tilt_deg.clamp(0.0, MAX_TILT_DEG));
    let lift = cam.lift.max(0.0) * sv;
    let (hw, hh) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let xr = (f64::from(x) + 0.5 - hw) / s;
    // The range of the line across the turned design: the corners of the
    // relief.
    let a = relief.area;
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for c in [a.min, Point::new(a.max.x, a.min.y), a.max, Point::new(a.min.x, a.max.y)] {
        let (dx, dy) = (c.x - cam.center.x, c.y - cam.center.y);
        let yr = -dx * st + dy * ct;
        lo = lo.min(yr);
        hi = hi.max(yr);
    }
    // Points behind the top of the picture and in front of its bottom do
    // not show (a height only lifts a point up the picture).
    let cv = cv.max(1e-6);
    let top = f64::from(relief.top) * lift;
    hi = hi.min((hh + top * s) / (s * cv) + 1.0 / s);
    lo = lo.max(-hh / (s * cv) - 1.0 / s);
    if lo >= hi {
        return;
    }
    // A step of one row on a flat part, and not more than one pixel of the
    // relief.
    let step = (1.0 / (s * cv)).min(1.0 / relief.px_per_mm);
    let ppm = relief.px_per_mm;
    let (rw, rh) = (i64::from(relief.width), i64::from(relief.height));
    // The place in the pixels of the relief at `yr`, and its change for each
    // step.
    let at = |yr: f64| {
        let px = cam.center.x + xr * ct - yr * st;
        let py = cam.center.y + xr * st + yr * ct;
        ((px - a.min.x) * ppm, (py - a.min.y) * ppm)
    };
    let (mut u, mut v) = at(hi);
    let (du, dv) = (step * st * ppm, -step * ct * ppm);
    // The highest row that a point in front filled (rows from it up are
    // free).
    let mut horizon = f64::from(h);
    let n = ((hi - lo) / step).ceil() as usize;
    for k in 0..=n {
        let yr = hi - step * k as f64;
        let (iu, iv) = (u.floor() as i64, v.floor() as i64);
        u += du;
        v += dv;
        if iu < 0 || iv < 0 || iu >= rw || iv >= rh {
            continue;
        }
        let i = (iv * rw + iu) as usize;
        let c = relief.rgba[i];
        if c[3] < 128 {
            continue;
        }
        let z = f64::from(relief.heights[i]);
        let sy = hh + s * (yr * cv - z * lift);
        if sy >= horizon {
            continue;
        }
        // The front face goes down to the fabric under the point, not
        // past it (at the front edge of the relief and at a clear pixel
        // nothing is in front).
        let base = hh + s * yr * cv + 1.0;
        let first = (sy - 0.5).ceil().max(0.0) as i64;
        let last = ((horizon.min(base) - 0.5).ceil() as i64 - 1).min(i64::from(h) - 1);
        for row in first..=last {
            out[row as usize] = if row == first {
                c
            } else {
                let f = |v: u8| (f64::from(v) * FACE).round() as u8;
                [f(c[0]), f(c[1]), f(c[2]), c[3]]
            };
        }
        horizon = horizon.min(sy);
        if horizon <= 0.0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A relief of 20 × 20 mm at 4 pixels for each mm: gray fabric, and a
    /// red block 1 mm high in the middle (8 to 12 mm).
    fn block() -> Relief {
        let (w, h) = (80u32, 80u32);
        let mut rgba = vec![[128, 128, 128, 255]; (w * h) as usize];
        let mut heights = vec![0.0f32; (w * h) as usize];
        for y in 32..48 {
            for x in 32..48 {
                rgba[(y * w + x) as usize] = [200, 30, 30, 255];
                heights[(y * w + x) as usize] = 1.0;
            }
        }
        Relief {
            area: BBox::new(Point::ZERO, Point::new(20.0, 20.0)),
            px_per_mm: 4.0,
            width: w,
            height: h,
            rgba,
            heights,
            top: 1.0,
        }
    }

    fn px(p: &Picture, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * p.width + x) * 4) as usize;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2], p.rgba[i + 3]]
    }

    #[test]
    fn straight_from_above_is_the_picture() {
        let r = block();
        let cam = Camera {
            center: Point::new(10.0, 10.0),
            px_per_mm: 4.0,
            tilt_deg: 0.0,
            turn_deg: 0.0,
            lift: 5.0,
        };
        let p = project(&r, &cam, 80, 80, [0, 0, 0, 255]);
        for (x, y) in [(5, 5), (40, 40), (33, 47), (70, 20)] {
            assert_eq!(px(&p, x, y), r.rgba[(y * 80 + x) as usize], "{x} {y}");
        }
    }

    #[test]
    fn a_high_part_hides_what_is_behind_it_and_shows_its_front() {
        let r = block();
        let flat = Camera {
            center: Point::new(10.0, 10.0),
            px_per_mm: 4.0,
            tilt_deg: 60.0,
            turn_deg: 0.0,
            lift: 0.0,
        };
        let high = Camera { lift: 4.0, ..flat };
        let (a, b) = (
            project(&r, &flat, 80, 80, [0, 0, 0, 255]),
            project(&r, &high, 80, 80, [0, 0, 0, 255]),
        );
        let red = |c: [u8; 4]| c[0] > c[1] + 50;
        let red_rows = |p: &Picture| (0..80).filter(|y| red(px(p, 40, *y))).count();
        // Lifted, the block is higher on the picture and covers more rows (its
        // top and its front face); the fabric behind it is hidden.
        assert!(red_rows(&b) > red_rows(&a) + 4, "{} {}", red_rows(&b), red_rows(&a));
        // Its front face is darker than its top.
        let reds: Vec<u8> = (0..80)
            .map(|y| px(&b, 40, y))
            .filter(|c| red(*c))
            .map(|c| c[0])
            .collect();
        assert!(reds.iter().any(|v| *v < 190) && reds.contains(&200), "{reds:?}");
        // Under the front edge of the relief (at 40 + 4 × 10 × cos 60° = 60
        // pixels) the background shows: no face goes past the fabric.
        assert_eq!(px(&b, 40, 70), [0, 0, 0, 255]);
        assert_eq!(px(&b, 40, 58), [128, 128, 128, 255]);
        // The point at the middle of the top is where the camera says.
        let (sx, sy) = high.to_picture(Point::new(10.0, 10.0), 1.0, 80, 80);
        assert_eq!(px(&b, sx as u32, sy as u32)[0], 200);
    }

    #[test]
    fn a_turn_turns_the_design_counterclockwise() {
        // A relief with a red mark at the right of the middle.
        let mut r = block();
        r.rgba.iter_mut().for_each(|c| *c = [128, 128, 128, 255]);
        r.heights.iter_mut().for_each(|z| *z = 0.0);
        for y in 38..42 {
            for x in 64..72 {
                r.rgba[(y * 80 + x) as usize] = [200, 30, 30, 255];
            }
        }
        let cam = Camera {
            center: Point::new(10.0, 10.0),
            px_per_mm: 4.0,
            tilt_deg: 0.0,
            turn_deg: 90.0,
            lift: 1.0,
        };
        let p = project(&r, &cam, 80, 80, [0, 0, 0, 255]);
        // A quarter turn counterclockwise: the mark is above the middle.
        assert!(px(&p, 40, 12)[0] > 150, "{:?}", px(&p, 40, 12));
        assert!(px(&p, 68, 40)[0] < 150);
    }

    #[test]
    fn clear_pixels_and_places_off_the_relief_show_the_background() {
        let mut r = block();
        for y in 0..2 {
            for x in 0..2 {
                r.rgba[y * 80 + x] = [0, 0, 0, 0];
            }
        }
        let cam = Camera {
            center: Point::new(10.0, 10.0),
            px_per_mm: 2.0,
            tilt_deg: 0.0,
            turn_deg: 0.0,
            lift: 1.0,
        };
        let p = project(&r, &cam, 80, 80, [1, 2, 3, 255]);
        // The relief is 40 pixels wide in the middle of 80.
        assert_eq!(px(&p, 5, 5), [1, 2, 3, 255]);
        assert_eq!(px(&p, 20, 20), [1, 2, 3, 255]);
        assert_eq!(px(&p, 21, 21), [128, 128, 128, 255]);
    }
}
