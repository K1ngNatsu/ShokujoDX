//! Picture to cross stitch (P9 Milestone 2): a grid over the picture, the
//! color of each cell, and cross-stitch objects of a few colors.
//!
//! 1. Each cell takes 6 × 6 samples of the picture. Clear pixels (alpha
//!    below 128) and samples outside the picture count as empty.
//! 2. A cell with most of its samples filled gets a full cross. With
//!    partial crosses on, a cell at an edge whose filled samples are on one
//!    side of a diagonal gets a three-quarter cross toward that side.
//! 3. The mean colors of the cells are reduced to the number of colors
//!    (`reduce`; or kept as they are if the picture has that few). Small
//!    groups of cells of one color take the color round them.
//! 4. One cross-stitch object for each color, all on the grid.

use emb_geom::{Point, Transform};
use emb_model::Rgb;
use emb_model::cross::{Corner, CrossStitch, Part};
use image::{Rgba, RgbaImage};

use crate::reduce::{NONE, ReduceSpec, exact, reduce};

/// Samples along each side of a cell.
const SAMPLES: usize = 6;
/// A cell with at least this part filled gets a full cross (no partial
/// crosses).
const FULL_PART: f64 = 0.5;
/// With partial crosses: a full cross from this part, a three-quarter
/// cross from `PARTIAL_MIN` if its side of the diagonal is filled to
/// `SIDE_PART`.
const FULL_PART_EDGE: f64 = 0.75;
const PARTIAL_MIN: f64 = 0.2;
const SIDE_PART: f64 = 0.75;
/// No more cells than this in each direction.
const MAX_CELLS: i32 = 1000;

/// Settings of [`cross_stitch`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossSpec {
    /// The number of thread colors, 2 to 32.
    pub colors: usize,
    /// Three-quarter crosses at the edges of the shapes.
    pub partial: bool,
    /// Groups of cells of one color smaller than this take the color round
    /// them (fewer trims). 0 or 1: keep all.
    pub min_cells: usize,
}

impl Default for CrossSpec {
    fn default() -> Self {
        CrossSpec {
            colors: 6,
            partial: true,
            min_cells: 3,
        }
    }
}

/// The cells of one color.
#[derive(Debug, Clone)]
pub struct CrossColor {
    pub color: Rgb,
    pub cross: CrossStitch,
}

/// What the samples of one cell found.
#[derive(Debug, Clone, Copy, Default)]
struct CellSamples {
    filled: usize,
    sum: [f64; 3],
    /// Filled samples on the side of each corner (the half of the cell
    /// across the diagonal that does not touch the corner), in the order
    /// of `Corner::ALL`.
    side: [usize; 4],
}

/// The side of the corner that the sample at (`fx`, `fy`) (from 0 to 1 in
/// the cell) is on, for each corner.
fn sides(fx: f64, fy: f64) -> [bool; 4] {
    [fx + fy < 1.0, fx > fy, fx + fy > 1.0, fx < fy]
}

/// The cross stitch of `img` (pixels to mm by `to_mm`) on the grid of
/// `grid` (its cells are not used). The colors come most common first.
pub fn cross_stitch(img: &RgbaImage, to_mm: &Transform, grid: &CrossStitch, spec: &CrossSpec) -> Vec<CrossColor> {
    let (w, h) = img.dimensions();
    let Some(inv) = to_mm.inverse() else { return Vec::new() };
    if w == 0 || h == 0 {
        return Vec::new();
    }
    // The cells under the picture.
    let corners = [
        (0.0, 0.0),
        (f64::from(w), 0.0),
        (f64::from(w), f64::from(h)),
        (0.0, f64::from(h)),
    ]
    .map(|(x, y)| grid.to_grid(to_mm.apply(Point::new(x, y))));
    let Some(g) = corners.into_iter().collect::<Option<Vec<Point>>>() else {
        return Vec::new();
    };
    let lo = |f: fn(&Point) -> f64| g.iter().map(f).fold(f64::INFINITY, f64::min).floor() as i32;
    let hi = |f: fn(&Point) -> f64| g.iter().map(f).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    let (c0, c1, r0, r1) = (lo(|p| p.x), hi(|p| p.x), lo(|p| p.y), hi(|p| p.y));
    if c1 <= c0 || r1 <= r0 || c1 - c0 > MAX_CELLS || r1 - r0 > MAX_CELLS {
        return Vec::new();
    }
    let (cols, rows) = ((c1 - c0) as u32, (r1 - r0) as u32);
    // 1. Samples.
    let mut cells = vec![CellSamples::default(); (cols * rows) as usize];
    for r in r0..r1 {
        for c in c0..c1 {
            let s = &mut cells[((r - r0) as u32 * cols + (c - c0) as u32) as usize];
            for j in 0..SAMPLES {
                for i in 0..SAMPLES {
                    let (fx, fy) = ((i as f64 + 0.5) / SAMPLES as f64, (j as f64 + 0.5) / SAMPLES as f64);
                    let q = inv.apply(grid.at(f64::from(c) + fx, f64::from(r) + fy));
                    if q.x < 0.0 || q.y < 0.0 || q.x >= f64::from(w) || q.y >= f64::from(h) {
                        continue;
                    }
                    let p = img.get_pixel(q.x as u32, q.y as u32);
                    if p[3] < 128 {
                        continue;
                    }
                    s.filled += 1;
                    for k in 0..3 {
                        s.sum[k] += f64::from(p[k]);
                    }
                    for (k, on) in sides(fx, fy).into_iter().enumerate() {
                        s.side[k] += usize::from(on);
                    }
                }
            }
        }
    }
    // 2. The part of each cell.
    let all = (SAMPLES * SAMPLES) as f64;
    let half = all / 2.0;
    let part = |s: &CellSamples| -> Option<Part> {
        let filled = s.filled as f64 / all;
        if !spec.partial {
            return (filled >= FULL_PART).then_some(Part::Full);
        }
        if filled >= FULL_PART_EDGE {
            return Some(Part::Full);
        }
        if filled >= PARTIAL_MIN {
            let (k, n) = s.side.iter().enumerate().max_by_key(|(k, n)| (**n, 4 - *k))?;
            // The filled samples are mostly on that side.
            if *n as f64 / half >= SIDE_PART && (*n as f64) >= s.filled as f64 * 0.8 {
                return Some(Part::ThreeQuarter(Corner::ALL[k]));
            }
        }
        (filled >= FULL_PART).then_some(Part::Full)
    };
    let parts: Vec<Option<Part>> = cells.iter().map(part).collect();
    // 3. The colors: one pixel for each cell with a cross.
    let small = RgbaImage::from_fn(cols, rows, |x, y| {
        let i = (y * cols + x) as usize;
        let s = &cells[i];
        match parts[i] {
            Some(_) if s.filled > 0 => {
                let m = |k: usize| (s.sum[k] / s.filled as f64).round().clamp(0.0, 255.0) as u8;
                Rgba([m(0), m(1), m(2), 255])
            }
            _ => Rgba([0, 0, 0, 0]),
        }
    });
    let k = spec.colors.clamp(2, 32);
    let reduced = exact(&small, k).unwrap_or_else(|| {
        reduce(
            &small,
            &ReduceSpec {
                colors: k,
                min_speck_px: spec.min_cells,
                clean_edges: false,
            },
        )
    });
    // 4. One object for each color.
    let mut out: Vec<CrossColor> = reduced
        .palette
        .iter()
        .map(|c| CrossColor {
            color: *c,
            cross: grid.same_grid_empty(),
        })
        .collect();
    for (i, label) in reduced.labels.iter().enumerate() {
        if *label == NONE {
            continue;
        }
        let Some(p) = parts[i] else { continue };
        let (c, r) = (c0 + (i as u32 % cols) as i32, r0 + (i as u32 / cols) as i32);
        out[*label as usize].cross.set(c, r, p);
    }
    out.retain(|o| !o.cross.cells.is_empty());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 60 × 40 px picture: the left half red, the right half blue, with
    /// a clear band at the top; 1 px = 0.5 mm, from (0, 0).
    fn picture() -> (RgbaImage, Transform) {
        let img = RgbaImage::from_fn(60, 40, |x, _| {
            if x < 30 {
                Rgba([200, 20, 20, 255])
            } else {
                Rgba([20, 20, 200, 255])
            }
        });
        let mut img = img;
        for y in 0..10 {
            for x in 0..60 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        (img, Transform::scale(0.5, 0.5))
    }

    #[test]
    fn cells_take_the_colors_of_the_picture() {
        let (img, t) = picture();
        // 30 × 20 mm with 2.5 mm cells: 12 × 8 cells; the clear band is 5 mm (2 rows).
        let grid = CrossStitch::new(Point::ZERO, 2.5);
        let out = cross_stitch(
            &img,
            &t,
            &grid,
            &CrossSpec {
                colors: 4,
                partial: false,
                min_cells: 0,
            },
        );
        assert_eq!(out.len(), 2);
        let red = out.iter().find(|o| o.color == Rgb(200, 20, 20)).unwrap();
        let blue = out.iter().find(|o| o.color == Rgb(20, 20, 200)).unwrap();
        assert_eq!(red.cross.cells.len(), 6 * 6);
        assert_eq!(blue.cross.cells.len(), 6 * 6);
        assert!(red.cross.cells.iter().all(|c| c.col < 6 && c.row >= 2));
        assert!(out.iter().all(|o| o.cross.same_grid(&grid)));
    }

    #[test]
    fn a_diagonal_edge_gets_three_quarter_crosses() {
        // A triangle below the diagonal from the top left to the bottom right.
        let img = RgbaImage::from_fn(100, 100, |x, y| {
            if y > x {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let t = Transform::scale(0.25, 0.25);
        let grid = CrossStitch::new(Point::ZERO, 2.5);
        let with = cross_stitch(
            &img,
            &t,
            &grid,
            &CrossSpec {
                colors: 2,
                partial: true,
                min_cells: 0,
            },
        );
        let cells = &with[0].cross.cells;
        // The cells on the diagonal: three-quarter crosses toward the bottom left.
        let diag: Vec<Part> = cells.iter().filter(|c| c.col == c.row).map(|c| c.part).collect();
        assert_eq!(diag.len(), 10);
        assert!(
            diag.iter().all(|p| *p == Part::ThreeQuarter(Corner::BottomLeft)),
            "{diag:?}"
        );
        assert_eq!(cells.iter().filter(|c| c.part == Part::Full).count(), 45);
        // Without partial crosses: full crosses only, and no cells on the diagonal.
        let without = cross_stitch(
            &img,
            &t,
            &grid,
            &CrossSpec {
                colors: 2,
                partial: false,
                min_cells: 0,
            },
        );
        assert!(without[0].cross.cells.iter().all(|c| c.part == Part::Full));
        assert_eq!(without[0].cross.cells.len(), 45);
    }

    #[test]
    fn many_colors_are_reduced() {
        let img = RgbaImage::from_fn(64, 64, |x, y| Rgba([(x * 4) as u8, (y * 4) as u8, 128, 255]));
        let grid = CrossStitch::new(Point::ZERO, 2.0);
        let out = cross_stitch(
            &img,
            &Transform::scale(0.5, 0.5),
            &grid,
            &CrossSpec {
                colors: 5,
                partial: true,
                min_cells: 0,
            },
        );
        assert!(out.len() <= 5 && out.len() >= 4);
        let n: usize = out.iter().map(|o| o.cross.cells.len()).sum();
        assert_eq!(n, 16 * 16);
    }
}
