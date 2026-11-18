//! The photo of the user's own product (P16 Milestone 7, ADR 0107): the
//! mask (the part of the photo that takes the product color) and the
//! color on the photo.
//!
//! The mask is a grid of cells over the photo ([`PhotoMask`]). A click
//! picks the cells round it that have a color near the color at the click
//! (a flood fill); a brush adds or removes cells. The product color goes
//! on the masked cloth with the light and the shade of the photo, so one
//! photo of a white product shows the product in any color.

use emb_model::{PhotoMask, Rgb};
use image::RgbaImage;

/// The cells of a mask, to change them.
#[derive(Debug, Clone, PartialEq)]
pub struct MaskGrid {
    pub width: u32,
    pub height: u32,
    /// Row by row.
    pub on: Vec<bool>,
}

impl MaskGrid {
    /// An empty grid for a photo of `pixels`.
    pub fn for_photo(pixels: (u32, u32)) -> Self {
        let (width, height) = PhotoMask::grid_for(pixels);
        MaskGrid {
            width,
            height,
            on: vec![false; width as usize * height as usize],
        }
    }

    pub fn of(mask: &PhotoMask) -> Self {
        MaskGrid {
            width: mask.width,
            height: mask.height,
            on: mask.cells(),
        }
    }

    pub fn to_mask(&self) -> PhotoMask {
        PhotoMask::from_cells(self.width, self.height, &self.on)
    }

    /// Sets each cell within `radius` cells of `center` (cell coordinates)
    /// to `on`.
    pub fn brush(&mut self, center: (f64, f64), radius: f64, on: bool) {
        let r = radius.max(0.5);
        let y0 = (center.1 - r).floor().max(0.0) as u32;
        let y1 = ((center.1 + r).ceil().max(0.0) as u32).min(self.height);
        let x0 = (center.0 - r).floor().max(0.0) as u32;
        let x1 = ((center.0 + r).ceil().max(0.0) as u32).min(self.width);
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = (f64::from(x) + 0.5 - center.0, f64::from(y) + 0.5 - center.1);
                if dx * dx + dy * dy <= r * r {
                    self.on[(y * self.width + x) as usize] = on;
                }
            }
        }
    }

    /// As [`MaskGrid::brush`], along the line from `a` to `b`.
    pub fn brush_line(&mut self, a: (f64, f64), b: (f64, f64), radius: f64, on: bool) {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let steps = (len / (radius.max(0.5) * 0.5)).ceil().max(1.0) as u32;
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps);
            self.brush((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), radius, on);
        }
    }

    /// Turns each cell on or off.
    pub fn invert(&mut self) {
        for c in &mut self.on {
            *c = !*c;
        }
    }

    /// Adds (`on`) or removes the cells that touch `seed` (cell
    /// coordinates) through cells with a color near the color at the seed.
    /// Thin lines of another color (seams, stitches) do not stop it and
    /// are part of the area. `photo` is the photo at the size of the grid.
    /// `tolerance` is from 0 (the same color only) to 100.
    pub fn pick(&mut self, photo: &RgbaImage, seed: (u32, u32), tolerance: f64, on: bool) {
        if photo.dimensions() != (self.width, self.height) || seed.0 >= self.width || seed.1 >= self.height {
            return;
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let target = ycc(photo.get_pixel(seed.0, seed.1).0);
        let limit = tolerance.clamp(0.0, 100.0) * 1.2;
        let near: Vec<bool> = photo
            .pixels()
            .map(|p| p.0[3] > 0 && distance(ycc(p.0), target) <= limit)
            .collect();
        let near = close(&near, w, h, CLOSE_CELLS);
        let mut seen = vec![false; w * h];
        let mut stack = vec![(seed.0 as usize, seed.1 as usize)];
        seen[seed.1 as usize * w + seed.0 as usize] = true;
        while let Some((x, y)) = stack.pop() {
            self.on[y * w + x] = on;
            let mut visit = |nx: usize, ny: usize| {
                let i = ny * w + nx;
                if !seen[i] && near[i] {
                    seen[i] = true;
                    stack.push((nx, ny));
                }
            };
            if x > 0 {
                visit(x - 1, y);
            }
            if x + 1 < w {
                visit(x + 1, y);
            }
            if y > 0 {
                visit(x, y - 1);
            }
            if y + 1 < h {
                visit(x, y + 1);
            }
        }
    }
}

/// A pick goes over lines up to twice this many cells wide.
const CLOSE_CELLS: usize = 3;

/// The closing of `cells` (`w` × `h`) by a square of `r` cells: it fills
/// gaps and holes narrower than about 2 × `r` cells; the outline stays.
fn close(cells: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let grown = spread(cells, w, h, r);
    let inverted: Vec<bool> = grown.iter().map(|&c| !c).collect();
    // Cells out of the grid do not count, so an area at the edge of the
    // photo stays at the edge.
    spread(&inverted, w, h, r).iter().map(|&c| !c).collect()
}

/// Each cell is on if a cell within `r` cells (a square) is on (rows,
/// then columns).
fn spread(cells: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let value = true;
    let mut rows = vec![!value; w * h];
    for y in 0..h {
        let row = &cells[y * w..(y + 1) * w];
        let mut last: Option<usize> = None;
        // The nearest `value` cell on the left and on the right.
        let mut left = vec![usize::MAX; w];
        for x in 0..w {
            if row[x] == value {
                last = Some(x);
            }
            left[x] = last.map_or(usize::MAX, |l| x - l);
        }
        last = None;
        for x in (0..w).rev() {
            if row[x] == value {
                last = Some(x);
            }
            let right = last.map_or(usize::MAX, |l| l - x);
            if left[x].min(right) <= r {
                rows[y * w + x] = value;
            }
        }
    }
    let mut out = vec![!value; w * h];
    for x in 0..w {
        let mut last: Option<usize> = None;
        let mut up = vec![usize::MAX; h];
        for y in 0..h {
            if rows[y * w + x] == value {
                last = Some(y);
            }
            up[y] = last.map_or(usize::MAX, |l| y - l);
        }
        last = None;
        for y in (0..h).rev() {
            if rows[y * w + x] == value {
                last = Some(y);
            }
            let down = last.map_or(usize::MAX, |l| l - y);
            if up[y].min(down) <= r {
                out[y * w + x] = value;
            }
        }
    }
    out
}

/// Brightness and color of a pixel: luma and two color differences.
fn ycc(p: [u8; 4]) -> [f64; 3] {
    let (r, g, b) = (f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    [y, b - y, r - y]
}

/// The distance of two colors. A change of brightness counts less than a
/// change of color: the light and the folds make the cloth brighter and
/// darker, but its color stays.
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dc = ((a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    dc + 0.4 * (a[0] - b[0]).abs()
}

/// The value (0 to 1) of the mask at the point (`u`, `v`) of the photo (0
/// to 1 across it), with a soft edge one cell wide.
fn mask_at(on: &[bool], w: u32, h: u32, u: f64, v: f64) -> f64 {
    let (fx, fy) = (u * f64::from(w) - 0.5, v * f64::from(h) - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let cell = |x: f64, y: f64| {
        let x = x.clamp(0.0, f64::from(w - 1)) as usize;
        let y = y.clamp(0.0, f64::from(h - 1)) as usize;
        if on[y * w as usize + x] { 1.0 } else { 0.0 }
    };
    let top = cell(x0, y0) * (1.0 - tx) + cell(x0 + 1.0, y0) * tx;
    let bottom = cell(x0, y0 + 1.0) * (1.0 - tx) + cell(x0 + 1.0, y0 + 1.0) * tx;
    top * (1.0 - ty) + bottom * ty
}

/// Puts `color` on the masked part of `rgba` (a photo of `w` × `h` pixels,
/// any size) with the light and the shade of the photo: the bright cloth
/// gets the color, and folds and shadows are darker by the same ratio as
/// in the photo. Highlights brighter than the cloth go to white.
pub fn recolor(rgba: &mut [u8], w: u32, h: u32, mask: &PhotoMask, color: Rgb) {
    if w == 0 || h == 0 || mask.width == 0 || mask.height == 0 || mask.is_empty() {
        return;
    }
    let on = mask.cells();
    let at = |x: u32, y: u32| {
        mask_at(
            &on,
            mask.width,
            mask.height,
            (f64::from(x) + 0.5) / f64::from(w),
            (f64::from(y) + 0.5) / f64::from(h),
        )
    };
    // The brightness of the cloth: a high part of the brightness of the
    // masked pixels (so most of the cloth is at the color or darker).
    let mut lumas: Vec<u8> = Vec::new();
    let step = ((u64::from(w) * u64::from(h) / 200_000).max(1) as f64).sqrt().ceil() as u32;
    for y in (0..h).step_by(step as usize) {
        for x in (0..w).step_by(step as usize) {
            if at(x, y) > 0.99 {
                let i = (y as usize * w as usize + x as usize) * 4;
                lumas.push(ycc([rgba[i], rgba[i + 1], rgba[i + 2], 255])[0].round() as u8);
            }
        }
    }
    if lumas.is_empty() {
        return;
    }
    lumas.sort_unstable();
    let reference = f64::from(lumas[(lumas.len() - 1) * 85 / 100]).max(8.0);
    let c = [f64::from(color.0), f64::from(color.1), f64::from(color.2)];
    for y in 0..h {
        for x in 0..w {
            let m = at(x, y);
            if m <= 0.0 {
                continue;
            }
            let i = (y as usize * w as usize + x as usize) * 4;
            let s = ycc([rgba[i], rgba[i + 1], rgba[i + 2], 255])[0] / reference;
            for k in 0..3 {
                let v = if s <= 1.0 {
                    c[k] * s
                } else {
                    c[k] + (255.0 - c[k]) * (s - 1.0).min(1.0)
                };
                let old = f64::from(rgba[i + k]);
                rgba[i + k] = (old + (v - old) * m).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A white shirt (with a darker fold) on a blue background.
    fn shirt() -> RgbaImage {
        RgbaImage::from_fn(40, 30, |x, y| {
            if (8..32).contains(&x) && (5..25).contains(&y) {
                let v = if x == 20 { 170 } else { 235 };
                image::Rgba([v, v, v, 255])
            } else {
                image::Rgba([40, 80, 200, 255])
            }
        })
    }

    #[test]
    fn a_pick_takes_the_cloth_and_its_folds_and_not_the_background() {
        let photo = shirt();
        let mut g = MaskGrid::for_photo((40, 30));
        g.pick(&photo, (10, 10), 30.0, true);
        let at = |g: &MaskGrid, x: u32, y: u32| g.on[(y * 40 + x) as usize];
        assert!(at(&g, 10, 10) && at(&g, 20, 10) && at(&g, 31, 24));
        assert!(!at(&g, 2, 2) && !at(&g, 35, 10));
        assert_eq!(g.on.iter().filter(|&&c| c).count(), 24 * 20);
        // A pick with "off" removes it again; the brush adds a round spot.
        g.pick(&photo, (10, 10), 30.0, false);
        assert!(g.on.iter().all(|&c| !c));
        g.brush((5.0, 5.0), 2.0, true);
        assert!(at(&g, 4, 4) && at(&g, 5, 5) && !at(&g, 9, 9));
        let m = g.to_mask();
        assert_eq!(MaskGrid::of(&m), g);
    }

    #[test]
    fn a_pick_goes_over_a_seam() {
        // A dark seam two cells wide across the shirt.
        let mut photo = shirt();
        for x in 8..32 {
            for y in 14..16 {
                photo.put_pixel(x, y, image::Rgba([90, 90, 90, 255]));
            }
        }
        let mut g = MaskGrid::for_photo((40, 30));
        g.pick(&photo, (10, 10), 30.0, true);
        assert_eq!(g.on.iter().filter(|&&c| c).count(), 24 * 20);
    }

    #[test]
    fn the_color_goes_on_with_the_shade_of_the_photo() {
        let photo = shirt();
        let mut g = MaskGrid::for_photo((40, 30));
        g.pick(&photo, (10, 10), 30.0, true);
        // The photo at twice the size of the mask.
        let mut big = image::imageops::resize(&photo, 80, 60, image::imageops::FilterType::Nearest);
        recolor(big.as_mut(), 80, 60, &g.to_mask(), Rgb(200, 20, 30));
        let px = |x: u32, y: u32| big.get_pixel(x, y).0;
        assert_eq!(&px(24, 20)[..3], &[200, 20, 30]);
        // The fold is darker in the same ratio as in the photo.
        let fold = px(41, 20);
        assert!((140..150).contains(&fold[0]) && fold[1] < 20, "{fold:?}");
        // The background stays.
        assert_eq!(&px(4, 4)[..3], &[40, 80, 200]);
    }

    #[test]
    fn a_large_photo_has_a_smaller_grid_of_its_shape() {
        assert_eq!(PhotoMask::grid_for((4000, 3000)), (1024, 768));
        assert_eq!(PhotoMask::grid_for((300, 200)), (300, 200));
        let m = PhotoMask::from_cells(3, 2, &[true, true, false, false, true, true]);
        assert_eq!(*m.runs, vec![0, 2, 2, 2]);
        assert!(!m.is_empty());
        assert!(PhotoMask::from_cells(2, 1, &[false, false]).is_empty());
    }
}
