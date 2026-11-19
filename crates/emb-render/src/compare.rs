//! A compare of two pictures for the developer (P18 view plan, Milestone 1,
//! ADR 0122): a picture of the 3D view and the picture before a change, a
//! picture at another zoom, or a photo of a sew-out of the same design.
//!
//! [`compare`] puts the two pictures side by side with their difference,
//! and gives the brightness of each area of a grid in each picture. The
//! brightness is the relative luminance: linear light with the weights of
//! sRGB (0 is black, 1 is white). A mean of linear light is the brightness
//! that the eye gets from an area at a distance, so the numbers of two
//! zooms of one design can be compared.
//!
//! The second picture fills the frame of the first one, or it goes where a
//! [`Placement`] puts it (a photo, which the developer scales and turns by
//! hand). A pixel of the first picture that the second one does not cover
//! is not counted. When the second picture fills the frame (a picture of
//! the same area at another zoom), the brightness of each of its areas is
//! the mean of its own pixels there.

use emb_geom::math;

use crate::raster::Picture;

/// An area with less light than this is dark: its relative difference
/// is measured against this value.
const MIN_LUMINANCE: f64 = 0.01;
/// The space between the panels of the compare picture, pixels.
const GAP: u32 = 8;
/// An area within this difference is the same (green frame; the limit of
/// the exit test of Milestone 4 of the plan).
pub const SAME: f64 = 0.03;
/// An area within this difference is near (yellow frame); more is red.
pub const NEAR: f64 = 0.10;

/// Where the second picture goes on the first one: a pixel (u, v) of the
/// second picture goes to (x, y) + scale × (u, v) turned by `angle_deg`
/// (clockwise on the screen, as the y axis goes down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub angle_deg: f64,
}

impl Placement {
    /// Reads `x,y,scale` or `x,y,scale,angle` (pixels, a factor, degrees).
    pub fn parse(s: &str) -> Result<Placement, String> {
        let v: Vec<f64> = s
            .split(',')
            .map(|p| p.trim().parse::<f64>().map_err(|_| format!("{p:?} is not a number")))
            .collect::<Result<_, _>>()?;
        let (x, y, scale, angle_deg) = match v.as_slice() {
            [x, y, s] => (*x, *y, *s, 0.0),
            [x, y, s, a] => (*x, *y, *s, *a),
            _ => return Err(format!("{s:?}: give x,y,scale or x,y,scale,angle")),
        };
        if !(scale.is_finite() && scale > 0.0) || ![x, y, angle_deg].iter().all(|v| v.is_finite()) {
            return Err(format!("{s:?}: the scale must be more than 0, and all values finite"));
        }
        Ok(Placement { x, y, scale, angle_deg })
    }
}

/// The brightness of one area of the grid in each picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub col: u32,
    pub row: u32,
    /// The mean relative luminance of the first picture (0 to 1).
    pub a: f64,
    /// The mean relative luminance of the second picture (0 to 1).
    pub b: f64,
    /// The pixels that both pictures cover. An area with none has no values.
    pub pixels: u64,
}

impl Area {
    /// The difference of `b` to `a`, as a part of `a` (0.03: `b` is 3 %
    /// lighter).
    pub fn relative(&self) -> f64 {
        (self.b - self.a) / self.a.max(MIN_LUMINANCE)
    }
}

/// The result of [`compare`].
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// The first picture, the second one and their difference, side by side.
    pub picture: Picture,
    pub cols: u32,
    pub rows: u32,
    /// The areas row by row, from the top left.
    pub areas: Vec<Area>,
}

impl Comparison {
    /// The areas that both pictures cover.
    fn counted(&self) -> impl Iterator<Item = &Area> {
        self.areas.iter().filter(|a| a.pixels > 0)
    }

    /// The area with the largest difference ([`Area::relative`]).
    pub fn largest(&self) -> Option<&Area> {
        self.counted()
            .max_by(|a, b| a.relative().abs().total_cmp(&b.relative().abs()))
    }

    /// The mean of the differences of the areas, all made positive.
    pub fn mean_difference(&self) -> f64 {
        let (sum, n) = self
            .counted()
            .fold((0.0, 0usize), |(s, n), a| (s + a.relative().abs(), n + 1));
        if n == 0 { 0.0 } else { sum / n as f64 }
    }

    /// The numbers as text: the difference of each area in percent, row by
    /// row, then the mean and the largest difference.
    pub fn report(&self) -> String {
        let mut out = format!(
            "{} x {} areas: the difference of the brightness of the second picture to the first one, %\n",
            self.cols, self.rows
        );
        for row in self.areas.chunks(self.cols as usize) {
            let cells: Vec<String> = row
                .iter()
                .map(|a| {
                    if a.pixels == 0 {
                        format!("{:>7}", "-")
                    } else {
                        format!("{:>+7.1}", 100.0 * a.relative())
                    }
                })
                .collect();
            out.push_str(&cells.join(""));
            out.push('\n');
        }
        let (a, b, n) = self.counted().fold((0.0, 0.0, 0u64), |(sa, sb, n), x| {
            (sa + x.a * x.pixels as f64, sb + x.b * x.pixels as f64, n + x.pixels)
        });
        if n > 0 {
            out.push_str(&format!(
                "brightness of all areas: {:.1} % and {:.1} %\n",
                100.0 * a / n as f64,
                100.0 * b / n as f64
            ));
        }
        out.push_str(&format!("mean difference: {:.1} %\n", 100.0 * self.mean_difference()));
        if let Some(l) = self.largest() {
            out.push_str(&format!(
                "largest difference: {:+.1} % (column {}, row {})\n",
                100.0 * l.relative(),
                l.col + 1,
                l.row + 1
            ));
        }
        out
    }
}

/// `p` with `factor` times fewer pixels on each side: each pixel is the
/// mean of the light of the pixels under it (in linear light; the last
/// row and column take the pixels that are left). A picture of a design at
/// a large zoom, made smaller this way, has the brightness that a picture
/// at the small zoom must have (the exit test of Milestone 4 of the view).
pub fn smaller(p: &Picture, factor: u32) -> Picture {
    let f = factor.max(1);
    let (w, h) = (p.width.div_ceil(f), p.height.div_ceil(f));
    let lin = linear_table();
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f64; 4];
            let mut n = 0.0;
            for sy in y * f..((y + 1) * f).min(p.height) {
                for sx in x * f..((x + 1) * f).min(p.width) {
                    let i = ((sy * p.width + sx) * 4) as usize;
                    let a = f64::from(p.rgba[i + 3]) / 255.0;
                    for k in 0..3 {
                        sum[k] += lin[p.rgba[i + k] as usize] * a;
                    }
                    sum[3] += a;
                    n += 1.0;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                rgba[o + k] = if sum[3] > 0.0 {
                    crate::light::to_srgb(sum[k] / sum[3])
                } else {
                    0
                };
            }
            rgba[o + 3] = (sum[3] / n * 255.0).round() as u8;
        }
    }
    Picture {
        width: w,
        height: h,
        rgba,
    }
}

/// The linear value of each 8-bit sRGB value.
fn linear_table() -> [f64; 256] {
    let mut t = [0.0; 256];
    for (i, v) in t.iter_mut().enumerate() {
        let c = i as f64 / 255.0;
        *v = if c <= 0.04045 {
            c / 12.92
        } else {
            math::pow((c + 0.055) / 1.055, 2.4)
        };
    }
    t
}

/// The relative luminance of an RGBA pixel over a white background.
fn luminance(lin: &[f64; 256], px: [u8; 4]) -> f64 {
    let a = f64::from(px[3]) / 255.0;
    let y = 0.2126 * lin[px[0] as usize] + 0.7152 * lin[px[1] as usize] + 0.0722 * lin[px[2] as usize];
    y * a + (1.0 - a)
}

/// The color of the second picture at a place (pixels; bilinear), or
/// `None` outside it.
fn sample(p: &Picture, u: f64, v: f64) -> Option<[u8; 4]> {
    let (w, h) = (f64::from(p.width), f64::from(p.height));
    if !(0.0..w).contains(&u) || !(0.0..h).contains(&v) {
        return None;
    }
    let (x, y) = ((u - 0.5).max(0.0), (v - 0.5).max(0.0));
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(p.width - 1), (y0 + 1).min(p.height - 1));
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let at = |x: u32, y: u32| {
        let i = (y as usize * p.width as usize + x as usize) * 4;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2], p.rgba[i + 3]].map(f64::from)
    };
    let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
    let mut out = [0u8; 4];
    for k in 0..4 {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bottom = c[k] + (d[k] - c[k]) * fx;
        out[k] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
    }
    Some(out)
}

/// Compares `b` with `a` on a grid of `cols` × `rows` areas of `a`. With no
/// `place`, `b` is stretched to the size of `a`.
pub fn compare(a: &Picture, b: &Picture, place: Option<Placement>, cols: u32, rows: u32) -> Comparison {
    let (cols, rows) = (cols.clamp(1, a.width.max(1)), rows.clamp(1, a.height.max(1)));
    let (aw, ah) = (a.width as usize, a.height as usize);
    // From a pixel of `a` to a place in `b`.
    let to_b: Box<dyn Fn(f64, f64) -> (f64, f64)> = match place {
        None => {
            let (sx, sy) = (
                f64::from(b.width) / f64::from(a.width),
                f64::from(b.height) / f64::from(a.height),
            );
            Box::new(move |x, y| (x * sx, y * sy))
        }
        Some(pl) => {
            let (s, c) = math::sin_cos_deg(pl.angle_deg);
            Box::new(move |x, y| {
                let (dx, dy) = ((x - pl.x) / pl.scale, (y - pl.y) / pl.scale);
                (c * dx + s * dy, -s * dx + c * dy)
            })
        }
    };
    let lin = linear_table();
    let width = 3 * a.width + 2 * GAP;
    let mut out = vec![0u8; width as usize * ah * 4];
    let mut sums = vec![(0.0f64, 0.0f64, 0u64); (cols * rows) as usize];
    let area_of = |x: usize, y: usize| {
        let c = (x * cols as usize / aw).min(cols as usize - 1);
        let r = (y * rows as usize / ah).min(rows as usize - 1);
        r * cols as usize + c
    };
    for y in 0..ah {
        for x in 0..aw {
            let i = (y * aw + x) * 4;
            let pa = [a.rgba[i], a.rgba[i + 1], a.rgba[i + 2], a.rgba[i + 3]];
            let o = (y * width as usize + x) * 4;
            out[o..o + 4].copy_from_slice(&pa);
            let (u, v) = to_b(x as f64 + 0.5, y as f64 + 0.5);
            let o2 = o + (a.width + GAP) as usize * 4;
            let o3 = o + 2 * (a.width + GAP) as usize * 4;
            match sample(b, u, v) {
                Some(pb) => {
                    out[o2..o2 + 4].copy_from_slice(&pb);
                    let (ya, yb) = (luminance(&lin, pa), luminance(&lin, pb));
                    let s = &mut sums[area_of(x, y)];
                    *s = (s.0 + ya, s.1 + yb, s.2 + 1);
                    out[o3..o3 + 4].copy_from_slice(&difference_color(yb - ya));
                }
                None => {
                    // A pattern of squares where the second picture is not.
                    let v = if (x / 8 + y / 8) % 2 == 0 { 70 } else { 90 };
                    out[o2..o2 + 4].copy_from_slice(&[v, v, v, 255]);
                    out[o3..o3 + 4].copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
    }
    // With no placement, the second picture is the same area: the mean of
    // each area comes from its own pixels, not from samples between them
    // (a sample between a light and a dark pixel mixes the numbers of the
    // screen, which is darker than the mix of their light).
    let own: Option<Vec<(f64, u64)>> = place.is_none().then(|| {
        let mut own = vec![(0.0f64, 0u64); (cols * rows) as usize];
        let (bw, bh) = (b.width as usize, b.height as usize);
        for y in 0..bh {
            for x in 0..bw {
                let i = (y * bw + x) * 4;
                let px = [b.rgba[i], b.rgba[i + 1], b.rgba[i + 2], b.rgba[i + 3]];
                let c = (x * cols as usize / bw).min(cols as usize - 1);
                let r = (y * rows as usize / bh).min(rows as usize - 1);
                let s = &mut own[r * cols as usize + c];
                *s = (s.0 + luminance(&lin, px), s.1 + 1);
            }
        }
        own
    });
    let areas: Vec<Area> = sums
        .iter()
        .enumerate()
        .map(|(k, &(sa, sb, n))| Area {
            col: k as u32 % cols,
            row: k as u32 / cols,
            a: if n > 0 { sa / n as f64 } else { 0.0 },
            b: match &own {
                Some(own) if own[k].1 > 0 => own[k].0 / own[k].1 as f64,
                _ if n > 0 => sb / n as f64,
                _ => 0.0,
            },
            pixels: n,
        })
        .collect();
    let mut picture = Picture {
        width,
        height: a.height,
        rgba: out,
    };
    draw_grid(&mut picture, a.width, cols, rows, &areas);
    Comparison {
        picture,
        cols,
        rows,
        areas,
    }
}

/// The color of a difference of luminance `d` (the second picture minus
/// the first one): gray for none, red where the second picture is
/// lighter, blue where it is darker. A difference of 0.25 has the full
/// color.
fn difference_color(d: f64) -> [u8; 4] {
    let t = (d * 4.0).clamp(-1.0, 1.0);
    let to = if t >= 0.0 {
        [235.0, 70.0, 50.0]
    } else {
        [50.0, 110.0, 240.0]
    };
    let k = t.abs();
    let c = to.map(|v: f64| (128.0 + (v - 128.0) * k).round() as u8);
    [c[0], c[1], c[2], 255]
}

/// Draws the lines of the grid on the three panels, and a frame round each
/// area of the difference panel: green within [`SAME`], yellow within
/// [`NEAR`], red for more.
fn draw_grid(p: &mut Picture, panel: u32, cols: u32, rows: u32, areas: &[Area]) {
    let (w, h) = (panel, p.height);
    let put = |p: &mut Picture, x: u32, y: u32, c: [u8; 4]| {
        if x < p.width && y < p.height {
            let i = (y as usize * p.width as usize + x as usize) * 4;
            p.rgba[i..i + 4].copy_from_slice(&c);
        }
    };
    let line = [0, 0, 0, 255];
    for k in 0..3 {
        let x0 = k * (panel + GAP);
        for c in 1..cols {
            let x = x0 + c * w / cols;
            for y in 0..h {
                put(p, x, y, line);
            }
        }
        for r in 1..rows {
            let y = r * h / rows;
            for x in 0..w {
                put(p, x0 + x, y, line);
            }
        }
    }
    let x0 = 2 * (panel + GAP);
    for a in areas.iter().filter(|a| a.pixels > 0) {
        let d = a.relative().abs();
        let c = if d <= SAME {
            [40, 170, 70, 255]
        } else if d <= NEAR {
            [230, 190, 30, 255]
        } else {
            [210, 30, 30, 255]
        };
        let (l, r) = (a.col * w / cols, (a.col + 1) * w / cols);
        let (t, b) = (a.row * h / rows, (a.row + 1) * h / rows);
        for i in 1..3 {
            for x in l + i..r.saturating_sub(i) {
                put(p, x0 + x, t + i, c);
                put(p, x0 + x, b.saturating_sub(i + 1), c);
            }
            for y in t + i..b.saturating_sub(i) {
                put(p, x0 + l + i, y, c);
                put(p, x0 + r.saturating_sub(i + 1), y, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, c: [u8; 4]) -> Picture {
        Picture {
            width: w,
            height: h,
            rgba: c.repeat((w * h) as usize),
        }
    }

    #[test]
    fn the_same_picture_has_no_difference() {
        let mut a = flat(40, 30, [120, 80, 40, 255]);
        a.rgba[4 * 50..4 * 51].copy_from_slice(&[250, 250, 250, 255]);
        let c = compare(&a, &a, None, 4, 3);
        assert_eq!(c.areas.len(), 12);
        assert!(c.areas.iter().all(|x| x.pixels == 100 && x.relative() == 0.0));
        assert_eq!(c.mean_difference(), 0.0);
        assert_eq!(c.largest().unwrap().relative(), 0.0);
        assert_eq!((c.picture.width, c.picture.height), (3 * 40 + 2 * GAP, 30));
        assert!(c.report().contains("mean difference: 0.0 %"), "{}", c.report());
    }

    #[test]
    fn the_brightness_is_linear_light() {
        let lin = linear_table();
        assert_eq!(luminance(&lin, [0, 0, 0, 255]), 0.0);
        assert!((luminance(&lin, [255, 255, 255, 255]) - 1.0).abs() < 1e-12);
        // sRGB 188 is about half the light of white; transparent is white.
        assert!((luminance(&lin, [188, 188, 188, 255]) - 0.5).abs() < 0.01);
        assert!((luminance(&lin, [0, 0, 0, 0]) - 1.0).abs() < 1e-12);
        // Green is the brightest of the three colors.
        assert!(luminance(&lin, [0, 255, 0, 255]) > luminance(&lin, [255, 0, 0, 255]));
    }

    #[test]
    fn a_lighter_area_is_found() {
        let a = flat(40, 40, [118, 118, 118, 255]);
        let mut b = a.clone();
        // The bottom right area (20 × 20 pixels) is lighter.
        for y in 20..40 {
            for x in 20..40 {
                let i = (y * 40 + x) * 4;
                b.rgba[i..i + 3].copy_from_slice(&[160, 160, 160]);
            }
        }
        let c = compare(&a, &b, None, 2, 2);
        let l = c.largest().unwrap();
        assert_eq!((l.col, l.row), (1, 1));
        // sRGB 118 is 18 % of white, 160 is 35 %.
        assert!((l.relative() - 0.9).abs() < 0.1, "{}", l.relative());
        assert!(c.areas[..3].iter().all(|x| x.relative() == 0.0));
        // The difference panel is red there, gray elsewhere.
        let at = |x: u32, y: u32| {
            let i = ((y * c.picture.width + x) * 4) as usize;
            &c.picture.rgba[i..i + 3]
        };
        let x3 = 2 * (40 + GAP);
        assert_eq!(at(x3 + 10, 10), &[128, 128, 128]);
        assert!(at(x3 + 30, 30)[0] > 180, "{:?}", at(x3 + 30, 30));
        assert!(c.report().contains("largest difference: +"), "{}", c.report());
    }

    #[test]
    fn a_larger_picture_of_the_same_thing_has_no_difference() {
        // A picture with a dark left half, and the same at twice the size.
        let half = |w: u32| {
            let mut p = flat(w, w, [230, 230, 230, 255]);
            for y in 0..w {
                for x in 0..w / 2 {
                    let i = ((y * w + x) * 4) as usize;
                    p.rgba[i..i + 3].copy_from_slice(&[20, 20, 20]);
                }
            }
            p
        };
        let c = compare(&half(20), &half(40), None, 2, 2);
        assert!(c.mean_difference() < 1e-9, "{}", c.report());
    }

    #[test]
    fn a_photo_goes_where_the_placement_puts_it() {
        // The photo is half the size, turned 90°: its pixel (u, v) shows
        // the place (40 - 2v, 2u) of the picture.
        let mut a = flat(40, 40, [200, 200, 200, 255]);
        for y in 0..40 {
            for x in 0..10 {
                let i = (y * 40 + x) * 4;
                a.rgba[i..i + 3].copy_from_slice(&[30, 30, 30]);
            }
        }
        let mut photo = flat(20, 20, [200, 200, 200, 255]);
        for v in 15..20 {
            for u in 0..20 {
                let i = (v * 20 + u) * 4;
                photo.rgba[i..i + 3].copy_from_slice(&[30, 30, 30]);
            }
        }
        let place = Placement::parse("40, 0, 2, 90").unwrap();
        let c = compare(&a, &photo, Some(place), 4, 4);
        // The dark band is on the dark band: only the pixels at its edge
        // differ (the photo has half the pixels).
        for x in &c.areas {
            match x.col {
                0 => assert!(x.b < 0.025, "{}", c.report()),
                1 => assert!(x.relative().abs() < 0.05, "{}", c.report()),
                _ => assert_eq!(x.relative(), 0.0, "{}", c.report()),
            }
        }
        // Off the photo nothing is counted.
        let small = Placement::parse("0,0,1").unwrap();
        let c = compare(&a, &photo, Some(small), 4, 4);
        assert_eq!(c.areas.iter().filter(|x| x.pixels > 0).count(), 4);
        assert!(c.report().contains("      -"), "{}", c.report());
    }

    #[test]
    fn a_smaller_picture_keeps_the_light() {
        // Black and white stripes one pixel wide: half the light.
        let mut p = flat(4, 2, [0, 0, 0, 255]);
        for y in 0..2 {
            for x in [1, 3] {
                let i = (y * 4 + x) * 4;
                p.rgba[i..i + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let s = smaller(&p, 2);
        assert_eq!((s.width, s.height), (2, 1));
        assert_eq!(&s.rgba[..4], &[188, 188, 188, 255]);
        // A width that is not a multiple: the last column has one pixel.
        assert_eq!(smaller(&flat(5, 5, [9, 9, 9, 255]), 2).width, 3);
    }

    #[test]
    fn placements_are_read() {
        assert_eq!(
            Placement::parse("1,2,0.5").unwrap(),
            Placement {
                x: 1.0,
                y: 2.0,
                scale: 0.5,
                angle_deg: 0.0
            }
        );
        assert_eq!(Placement::parse("1, 2, 3, -4").unwrap().angle_deg, -4.0);
        for bad in ["", "1,2", "1,2,0", "1,2,-1", "a,b,c", "1,2,3,4,5", "1,2,inf"] {
            assert!(Placement::parse(bad).is_err(), "{bad}");
        }
    }
}
