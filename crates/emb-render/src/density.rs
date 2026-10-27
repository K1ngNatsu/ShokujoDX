//! The density map (P11 Milestone 1): how many layers of thread lie on
//! each small square of the design, over all objects together.
//!
//! Where objects lie on top of each other, the layers add up. Too many
//! layers make the fabric stiff, break needles and threads, and stress the
//! machine (the owner report of 2026-10-24 on satin from SVG shapes). The
//! warnings of Design info look at one object at a time; the map shows the
//! build-up of all objects.
//!
//! Method: the design area is cut into cells of [`CELL_MM`]. The thread
//! of each stitch (from one needle point to the next) goes to the cells
//! that it crosses, by length. The layers of a cell are the thread length
//! times the thread width ([`THREAD_WIDTH_MM`]), divided by the cell area;
//! then each cell takes the mean of the 3 × 3 cells round it, so that a
//! row that falls on a cell border does not give a false peak. One even
//! layer of fill rows is about 1; underlay adds some.

use emb_geom::{BBox, Point};
use emb_model::{StitchDesign, StitchKind};

/// The side of a cell, in mm.
pub const CELL_MM: f64 = 0.5;
/// The width of a 40 wt embroidery thread on the fabric, in mm.
pub const THREAD_WIDTH_MM: f64 = 0.4;
/// The default limit, in layers. A fill with its underlay has about 1.5 to
/// 2, a satin column with its underlay 3 to 4 (its stitches go both ways),
/// a satin or a second fill on a fill 4 to 6. The corpus designs go over 5
/// where objects lie on top of each other with no knockout, at the joins
/// of small satin letters and at the center of radial fills.
pub const DEFAULT_LIMIT: f64 = 5.0;
/// Places over the limit that are smaller than this are left out of the
/// list (mm²): the stitches of a satin column come close together where it
/// ends in a point (2 to 3 mm² over the limit at the tips of a crescent).
pub const MIN_SPOT_MM2: f64 = 4.0;
/// The most cells of a map; a larger design gets larger cells.
const MAX_CELLS: usize = 4_000_000;

/// The layers of thread of each cell of a design.
#[derive(Debug, Clone, PartialEq)]
pub struct DensityMap {
    /// The corner of the first cell (the smallest x and y), in the
    /// coordinates of the stitches.
    pub origin: Point,
    pub cell_mm: f64,
    pub cols: usize,
    pub rows: usize,
    /// The layers of each cell, row after row.
    pub layers: Vec<f32>,
}

/// A place where the layers are more than the limit: connected cells.
#[derive(Debug, Clone, PartialEq)]
pub struct HotSpot {
    pub bbox: BBox,
    pub area_mm2: f64,
    pub max_layers: f64,
    /// The center of the cell with the most layers.
    pub peak: Point,
}

impl DensityMap {
    /// The map of the stitches of `s`. `None` for a design with no stitches.
    pub fn of(s: &StitchDesign) -> Option<DensityMap> {
        let recs = &s.records;
        let b = BBox::from_points(recs.iter().filter(|r| r.kind == StitchKind::Stitch).map(|r| r.pos))?;
        let mut cell = CELL_MM;
        let (cols, rows) = loop {
            // One empty cell round the design, and one more for the rounding.
            let cols = (b.width() / cell).ceil() as usize + 3;
            let rows = (b.height() / cell).ceil() as usize + 3;
            if cols.saturating_mul(rows) <= MAX_CELLS {
                break (cols, rows);
            }
            cell *= 2.0;
        };
        let origin = Point::new(b.min.x - cell, b.min.y - cell);
        let mut length = vec![0.0f64; cols * rows];
        let index = |p: Point| {
            let c = (((p.x - origin.x) / cell).floor().max(0.0) as usize).min(cols - 1);
            let r = (((p.y - origin.y) / cell).floor().max(0.0) as usize).min(rows - 1);
            r * cols + c
        };
        for w in recs.windows(2) {
            if w[0].kind != StitchKind::Stitch || w[1].kind != StitchKind::Stitch {
                continue;
            }
            let (a, z) = (w[0].pos, w[1].pos);
            let d = a.distance(z);
            if d < 1e-9 {
                continue;
            }
            // Samples a quarter of a cell apart, each with its part of the length.
            let n = (d / (cell / 4.0)).ceil().max(1.0) as usize;
            let step = d / n as f64;
            for i in 0..n {
                let t = (i as f64 + 0.5) / n as f64;
                length[index(Point::new(a.x + (z.x - a.x) * t, a.y + (z.y - a.y) * t))] += step;
            }
        }
        let k = THREAD_WIDTH_MM / (cell * cell);
        let mut layers = vec![0.0f32; cols * rows];
        for r in 0..rows {
            for c in 0..cols {
                let mut sum = 0.0;
                for rr in r.saturating_sub(1)..=(r + 1).min(rows - 1) {
                    for cc in c.saturating_sub(1)..=(c + 1).min(cols - 1) {
                        sum += length[rr * cols + cc];
                    }
                }
                layers[r * cols + c] = (sum / 9.0 * k) as f32;
            }
        }
        Some(DensityMap {
            origin,
            cell_mm: cell,
            cols,
            rows,
            layers,
        })
    }

    /// The layers of the cell at `p` (0 outside the map).
    pub fn layers_at(&self, p: Point) -> f64 {
        let c = ((p.x - self.origin.x) / self.cell_mm).floor();
        let r = ((p.y - self.origin.y) / self.cell_mm).floor();
        if c < 0.0 || r < 0.0 || c >= self.cols as f64 || r >= self.rows as f64 {
            return 0.0;
        }
        f64::from(self.layers[r as usize * self.cols + c as usize])
    }

    /// The most layers of a cell.
    pub fn max_layers(&self) -> f64 {
        f64::from(self.layers.iter().copied().fold(0.0f32, f32::max))
    }

    /// The center of the cell in column `c` and row `r`.
    pub fn cell_center(&self, c: usize, r: usize) -> Point {
        Point::new(
            self.origin.x + (c as f64 + 0.5) * self.cell_mm,
            self.origin.y + (r as f64 + 0.5) * self.cell_mm,
        )
    }

    /// The places with more layers than `limit` and an area of
    /// [`MIN_SPOT_MM2`] or more, the most layers first.
    pub fn hot_spots(&self, limit: f64) -> Vec<HotSpot> {
        let over = |i: usize| f64::from(self.layers[i]) > limit;
        let mut seen = vec![false; self.layers.len()];
        let mut spots = Vec::new();
        let cell_area = self.cell_mm * self.cell_mm;
        for start in 0..self.layers.len() {
            if seen[start] || !over(start) {
                continue;
            }
            // The cells of one place (4-connected), with a stack.
            seen[start] = true;
            let mut stack = vec![start];
            let (mut n, mut max, mut peak) = (0usize, 0.0f64, start);
            let mut bbox: Option<BBox> = None;
            while let Some(i) = stack.pop() {
                n += 1;
                let (c, r) = (i % self.cols, i / self.cols);
                let l = f64::from(self.layers[i]);
                if l > max {
                    max = l;
                    peak = i;
                }
                let p = self.cell_center(c, r);
                let h = self.cell_mm / 2.0;
                let cell_box = BBox::new(Point::new(p.x - h, p.y - h), Point::new(p.x + h, p.y + h));
                bbox = Some(bbox.map_or(cell_box, |b| b.union(cell_box)));
                let mut visit = |j: usize| {
                    if !seen[j] && over(j) {
                        seen[j] = true;
                        stack.push(j);
                    }
                };
                if c > 0 {
                    visit(i - 1);
                }
                if c + 1 < self.cols {
                    visit(i + 1);
                }
                if r > 0 {
                    visit(i - self.cols);
                }
                if r + 1 < self.rows {
                    visit(i + self.cols);
                }
            }
            let area = n as f64 * cell_area;
            if area >= MIN_SPOT_MM2
                && let Some(bbox) = bbox
            {
                spots.push(HotSpot {
                    bbox,
                    area_mm2: area,
                    max_layers: max,
                    peak: self.cell_center(peak % self.cols, peak / self.cols),
                });
            }
        }
        spots.sort_by(|a, b| b.max_layers.total_cmp(&a.max_layers));
        spots
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{StitchRecord, Thread, ThreadId};

    use super::*;

    /// Rows of running stitches `spacing` apart over a square of `size` mm
    /// at `at`, with stitches 2 mm long.
    fn rows(at: Point, size: f64, spacing: f64, out: &mut Vec<StitchRecord>) {
        let n = (size / spacing) as usize;
        for i in 0..=n {
            let y = at.y + i as f64 * spacing;
            let xs: Vec<f64> = (0..=(size / 2.0) as usize).map(|k| at.x + k as f64 * 2.0).collect();
            let xs: Vec<f64> = if i % 2 == 0 { xs } else { xs.into_iter().rev().collect() };
            for x in xs {
                out.push(StitchRecord::stitch(x, y));
            }
        }
    }

    fn design(records: Vec<StitchRecord>) -> StitchDesign {
        StitchDesign {
            threads: vec![Thread {
                id: ThreadId(0),
                name: "t".into(),
                color: emb_model::Rgb(0, 0, 0),
                brand: None,
                code: None,
            }],
            records,
        }
    }

    #[test]
    fn one_even_layer_is_about_one() {
        let mut r = Vec::new();
        rows(Point::ZERO, 20.0, 0.4, &mut r);
        let m = DensityMap::of(&design(r)).unwrap();
        let mid = m.layers_at(Point::new(10.0, 10.0));
        assert!((0.9..1.1).contains(&mid), "{mid}");
        assert!(m.hot_spots(DEFAULT_LIMIT).is_empty());
    }

    #[test]
    fn layers_on_top_of_each_other_add_up_and_show_as_a_place() {
        let mut r = Vec::new();
        rows(Point::ZERO, 20.0, 0.4, &mut r);
        // Five more layers on a 6 mm square in the middle.
        for _ in 0..5 {
            rows(Point::new(7.0, 7.0), 6.0, 0.4, &mut r);
        }
        let m = DensityMap::of(&design(r)).unwrap();
        assert!(
            m.layers_at(Point::new(10.0, 10.0)) > 5.2,
            "{}",
            m.layers_at(Point::new(10.0, 10.0))
        );
        assert!(m.layers_at(Point::new(2.0, 2.0)) < 1.2, "the rest stays one layer");
        let spots = m.hot_spots(DEFAULT_LIMIT);
        assert_eq!(spots.len(), 1);
        let s = &spots[0];
        assert!(s.bbox.contains(Point::new(10.0, 10.0)) && s.bbox.width() < 8.0, "{s:?}");
        assert!((20.0..50.0).contains(&s.area_mm2), "{s:?}");
        assert!(s.max_layers > DEFAULT_LIMIT);
    }

    #[test]
    fn jumps_are_not_thread_and_a_large_design_gets_larger_cells() {
        let mut r = vec![StitchRecord::stitch(0.0, 0.0)];
        r.push(StitchRecord::new(Point::new(5000.0, 3000.0), StitchKind::Jump));
        r.push(StitchRecord::stitch(5000.0, 3000.0));
        let m = DensityMap::of(&design(r)).unwrap();
        assert!(m.cols * m.rows <= MAX_CELLS && m.cell_mm > CELL_MM);
        assert!(m.max_layers() < 0.01, "the jump made no thread");
        assert!(DensityMap::of(&design(vec![])).is_none());
    }
}
