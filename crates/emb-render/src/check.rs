//! The check views of the surface (P18 view plan, Milestone 8, ADR 0129):
//!
//! - **The fabric shows through**: in an area of stitches (a fill or a
//!   satin), the part of each place that no thread covers. It comes from the
//!   laid stitches (their widths, their heights and the gaps between them),
//!   not only from the density of the object: a fill with its rows too far
//!   apart for the thread, a fill that pulls open at a turn, the pile of a
//!   towel through thin stitches.
//! - **The thread is high**: the height of the thread over what is not
//!   thread under it (the fabric, a piece, padding or foam). Many layers of
//!   thread are a risk of thread breaks and broken needles.
//!
//! The map has cells of [`CELL_MM`]: a surface of the design at this size
//! (made with the detail of the surface and halved, so a cell has the mean
//! of its pixels). The edges of an area are not checked ([`EDGE_MM`]): the
//! stitches of a fill end at its outline.

use emb_geom::{BBox, Point};
use emb_model::{Design, FillParams, FillPattern, ObjectKind};

use crate::fabric::Ground;
use crate::surface::Laid;
use crate::thread3d::Lighting;

/// The side of a cell, mm.
pub const CELL_MM: f64 = 0.25;
/// The default limit of the part of the fabric that shows through an area.
pub const DEFAULT_SHOW_LIMIT: f64 = 0.25;
/// The default limit of the height of the thread, mm (about six layers of
/// 40 weight thread).
pub const DEFAULT_HEIGHT_MM: f64 = 1.2;
/// Rows this far apart or more are a texture: the fabric shows by design,
/// mm.
pub const OPEN_SPACING_MM: f64 = 1.0;
/// A place is at least this large, mm².
pub const MIN_PLACE_MM2: f64 = 0.5;
/// The part of an area this near its edge is not checked, mm.
pub const EDGE_MM: f64 = 0.5;
/// The fabric that shows is the mean in a square of this many cells round
/// a cell (one place is about 0.75 mm wide).
const WINDOW: usize = 1;
/// The most cells of a map; a larger design gets larger cells.
const MAX_CELLS: usize = 4_000_000;

/// The checks of the surface of a design.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckMap {
    /// The corner of the first cell (the coordinates of the stitches).
    pub origin: Point,
    pub cell_mm: f64,
    pub cols: usize,
    pub rows: usize,
    /// The part of the fabric that shows through an area, 0 to 1; −1 where
    /// no area is checked.
    pub open: Vec<f32>,
    /// The height of the thread over what is not thread, mm.
    pub thread: Vec<f32>,
}

/// A place of a check: connected cells over the limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub bbox: BBox,
    pub area_mm2: f64,
    /// The largest value in it (the part that shows, or mm).
    pub worst: f64,
    /// The center of the cell with the largest value.
    pub peak: Point,
}

/// The areas of the objects of `design` that must cover the fabric (in the
/// coordinates of the stitches): fills of rows and satins. Fills that are
/// open by design are not checked: motifs, stipple, cross stitch, a
/// tartan or a gradient, accordion spacing (a blend), a jagged edge, the
/// hand-stitch effect, and rows [`OPEN_SPACING_MM`] or more apart (a
/// texture).
pub fn areas(design: &Design) -> Vec<Vec<Vec<Point>>> {
    let to_out = design.output_transform();
    let rings = |p: &emb_geom::Path| -> Vec<Vec<Point>> {
        p.flatten(0.1)
            .into_iter()
            .map(|pl| pl.points.iter().map(|q| to_out.apply(*q)).collect())
            .collect()
    };
    let mut out = Vec::new();
    for o in design.objects() {
        match &o.kind {
            ObjectKind::Fill { outer, holes, params } if covers(params) => {
                let mut r = rings(outer);
                for h in holes {
                    r.extend(rings(h));
                }
                out.push(r);
            }
            ObjectKind::Satin { geometry, params }
                if params.pattern.is_satin() && params.accordion.is_none() && params.spacing_mm < OPEN_SPACING_MM =>
            {
                let side = |p: &emb_geom::Path| -> Vec<Point> { rings(p).into_iter().flatten().collect() };
                let mut ring = side(&geometry.rail_a);
                ring.extend(side(&geometry.rail_b).into_iter().rev());
                out.push(vec![ring]);
            }
            _ => {}
        }
    }
    out
}

/// A fill whose rows must cover all of its shape.
fn covers(p: &FillParams) -> bool {
    let rows = matches!(
        p.pattern,
        FillPattern::Tatami
            | FillPattern::Square
            | FillPattern::DoubleSquare { .. }
            | FillPattern::Contour
            | FillPattern::Offset
            | FillPattern::Spiral
            | FillPattern::Florentine { .. }
            | FillPattern::ProgramSplit { .. }
            | FillPattern::FlexiSplit { .. }
            | FillPattern::UserSplit { .. }
    );
    let satin = matches!(p.pattern, FillPattern::Columns { column } if column.is_satin());
    (rows || satin) && p.accordion.is_none() && p.jagged_mm <= 0.0 && p.hand_mm <= 0.0 && p.spacing_mm < OPEN_SPACING_MM
}

impl CheckMap {
    /// The checks of the laid stitches `laid` on `ground`, in the areas
    /// `areas` ([`areas`]). `None` if nothing is laid.
    pub fn of(laid: &Laid, areas: &[Vec<Vec<Point>>], ground: Option<&Ground>) -> Option<CheckMap> {
        if laid.is_empty() {
            return None;
        }
        let b = laid.bounds();
        let mut cell = CELL_MM;
        while (b.width() / cell).ceil() * (b.height() / cell).ceil() > MAX_CELLS as f64 {
            cell *= 2.0;
        }
        let ppm = 1.0 / cell;
        let s = laid.surface_on(b, ppm, &Lighting::default(), ground);
        let (cols, rows) = (s.width as usize, s.height as usize);
        let floor = laid.floor_in(s.area, ppm, s.width, s.height);
        let mut bare = vec![0.0f32; cols * rows];
        let mut thread = vec![0.0f32; cols * rows];
        for t in &s.tiles {
            for row in 0..t.h as usize {
                for col in 0..t.w as usize {
                    let k = row * t.w as usize + col;
                    let i = (t.y0 as usize + row) * cols + t.x0 as usize + col;
                    // A hole of cutwork is not fabric that shows.
                    let open = 1.0 - f32::from(t.cover[k]) / 255.0;
                    bare[i] = open * (1.0 - f32::from(t.behind[k]) / 255.0);
                    if t.cover[k] > 0 {
                        thread[i] = (t.height[k] - floor[i]).max(0.0);
                    }
                }
            }
        }
        let inside = inside(areas, s.area.min, cell, cols, rows);
        let edge = (EDGE_MM / cell).round() as usize;
        let checked = erode(&inside, cols, rows, edge);
        let mean = mean_in(&bare, cols, rows, WINDOW);
        let open = (0..cols * rows)
            .map(|i| if checked[i] { mean[i] } else { -1.0 })
            .collect();
        Some(CheckMap {
            origin: s.area.min,
            cell_mm: cell,
            cols,
            rows,
            open,
            thread,
        })
    }

    /// The center of the cell in column `c` and row `r`.
    pub fn cell_center(&self, c: usize, r: usize) -> Point {
        Point::new(
            self.origin.x + (c as f64 + 0.5) * self.cell_mm,
            self.origin.y + (r as f64 + 0.5) * self.cell_mm,
        )
    }

    /// The index of the cell at `p`, if it is in the map.
    fn index(&self, p: Point) -> Option<usize> {
        let c = ((p.x - self.origin.x) / self.cell_mm).floor();
        let r = ((p.y - self.origin.y) / self.cell_mm).floor();
        (c >= 0.0 && r >= 0.0 && c < self.cols as f64 && r < self.rows as f64)
            .then(|| r as usize * self.cols + c as usize)
    }

    /// The part of the fabric that shows at `p` (−1 where no area is
    /// checked).
    pub fn open_at(&self, p: Point) -> f64 {
        self.index(p).map_or(-1.0, |i| f64::from(self.open[i]))
    }

    /// The height of the thread at `p`, mm.
    pub fn thread_at(&self, p: Point) -> f64 {
        self.index(p).map_or(0.0, |i| f64::from(self.thread[i]))
    }

    /// The highest thread, mm.
    pub fn max_thread(&self) -> f64 {
        f64::from(self.thread.iter().copied().fold(0.0f32, f32::max))
    }

    /// The places where more than `limit` of the fabric shows through an
    /// area, the largest part first.
    pub fn show_through(&self, limit: f64) -> Vec<Place> {
        self.places(&self.open, limit)
    }

    /// The places where the thread is higher than `limit_mm`, the highest
    /// first.
    pub fn high(&self, limit_mm: f64) -> Vec<Place> {
        self.places(&self.thread, limit_mm)
    }

    /// The places of connected cells (4-connected) whose `values` are over
    /// `limit`, of [`MIN_PLACE_MM2`] or more, the largest value first.
    fn places(&self, values: &[f32], limit: f64) -> Vec<Place> {
        let over = |i: usize| f64::from(values[i]) > limit;
        let mut seen = vec![false; values.len()];
        let mut out = Vec::new();
        let area = self.cell_mm * self.cell_mm;
        for start in 0..values.len() {
            if seen[start] || !over(start) {
                continue;
            }
            seen[start] = true;
            let mut stack = vec![start];
            let (mut n, mut worst, mut peak) = (0usize, f64::MIN, start);
            let mut b: Option<BBox> = None;
            while let Some(i) = stack.pop() {
                n += 1;
                let v = f64::from(values[i]);
                if v > worst {
                    (worst, peak) = (v, i);
                }
                let (c, r) = (i % self.cols, i / self.cols);
                let p = self.cell_center(c, r);
                let cb = BBox::new(p, p).expand(self.cell_mm / 2.0);
                b = Some(b.map_or(cb, |b| b.union(cb)));
                let mut push = |j: usize| {
                    if !seen[j] && over(j) {
                        seen[j] = true;
                        stack.push(j);
                    }
                };
                if c > 0 {
                    push(i - 1);
                }
                if c + 1 < self.cols {
                    push(i + 1);
                }
                if r > 0 {
                    push(i - self.cols);
                }
                if r + 1 < self.rows {
                    push(i + self.cols);
                }
            }
            let a = n as f64 * area;
            if a >= MIN_PLACE_MM2
                && let Some(bbox) = b
            {
                out.push(Place {
                    bbox,
                    area_mm2: a,
                    worst,
                    peak: self.cell_center(peak % self.cols, peak / self.cols),
                });
            }
        }
        out.sort_by(|a, b| b.worst.total_cmp(&a.worst));
        out
    }
}

/// The cells (`cols` × `rows` of `cell` mm from `origin`) whose centers are
/// inside one of `areas` (each area even-odd).
fn inside(areas: &[Vec<Vec<Point>>], origin: Point, cell: f64, cols: usize, rows: usize) -> Vec<bool> {
    let mut out = vec![false; cols * rows];
    let mut cross: Vec<f64> = Vec::new();
    for rings in areas {
        let Some(b) = BBox::from_points(rings.iter().flatten().copied()) else {
            continue;
        };
        let r0 = (((b.min.y - origin.y) / cell).floor().max(0.0) as usize).min(rows);
        let r1 = ((((b.max.y - origin.y) / cell).ceil()).max(0.0) as usize).min(rows);
        for row in r0..r1 {
            let yc = origin.y + (row as f64 + 0.5) * cell;
            cross.clear();
            for r in rings {
                for i in 0..r.len() {
                    let (a, c) = (r[i], r[(i + 1) % r.len()]);
                    if (a.y <= yc) != (c.y <= yc) {
                        cross.push(a.x + (yc - a.y) / (c.y - a.y) * (c.x - a.x));
                    }
                }
            }
            cross.sort_by(f64::total_cmp);
            for [x0, x1] in cross.as_chunks::<2>().0 {
                // The cells whose centers are between the two crossings.
                let c0 = ((x0 - origin.x) / cell - 0.5).ceil().max(0.0) as usize;
                let c1 = (((x1 - origin.x) / cell - 0.5).floor() + 1.0).max(0.0) as usize;
                for c in c0.min(cols)..c1.min(cols) {
                    out[row * cols + c] = true;
                }
            }
        }
    }
    out
}

/// The cells of `mask` whose square of `r` cells round them is all in it.
fn erode(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    if r == 0 {
        return mask.to_vec();
    }
    // Along the rows, then along the columns.
    let pass = |src: &[bool], n: usize, stride: usize, count: usize, step: usize| -> Vec<bool> {
        let mut out = vec![false; src.len()];
        for line in 0..count {
            let base = line * step;
            let mut run = 0usize;
            // The length of the run of set cells that ends at each cell, from
            // both sides.
            let mut left = vec![0usize; n];
            for k in 0..n {
                run = if src[base + k * stride] { run + 1 } else { 0 };
                left[k] = run;
            }
            run = 0;
            for k in (0..n).rev() {
                run = if src[base + k * stride] { run + 1 } else { 0 };
                out[base + k * stride] = left[k] > r && run > r;
            }
        }
        out
    };
    let rows = pass(mask, w, 1, h, w);
    pass(&rows, h, w, w, 1)
}

/// The mean of `v` in a square of `2r + 1` cells round each cell (fewer at
/// the edges).
fn mean_in(v: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let (mut sum, mut n) = (0.0f32, 0.0f32);
            for yy in y.saturating_sub(r)..=(y + r).min(h - 1) {
                for xx in x.saturating_sub(r)..=(x + r).min(w - 1) {
                    sum += v[yy * w + xx];
                    n += 1.0;
                }
            }
            out[y * w + x] = sum / n;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use emb_model::{Rgb, ThreadKind};

    use super::*;
    use crate::strand::Strand;
    use crate::surface::lay;

    /// Rows of a fill over a square of 10 mm, `spacing` mm apart.
    fn rows(spacing: f64) -> Vec<Strand> {
        let mut out = Vec::new();
        let mut y = 0.0;
        let mut k = 0;
        while y <= 10.0 {
            let (a, b): (f64, f64) = if k % 2 == 0 { (0.0, 10.0) } else { (10.0, 0.0) };
            // Stitches of 3 mm along each row.
            let mut x = a;
            while (b - x).abs() > 1e-9 {
                let nx = if b > a { (x + 3.0).min(b) } else { (x - 3.0).max(b) };
                out.push(Strand {
                    a: Point::new(x, y),
                    b: Point::new(nx, y),
                    color: Rgb(30, 60, 160),
                    kind: ThreadKind::Polyester,
                    weight: 40,
                    lift_mm: 0.0,
                    record: out.len() as u32,
                });
                x = nx;
            }
            y += spacing;
            k += 1;
        }
        out
    }

    fn square() -> Vec<Vec<Vec<Point>>> {
        vec![vec![vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ]]]
    }

    #[test]
    fn the_fabric_shows_through_an_open_fill_and_not_a_dense_one() {
        let open = CheckMap::of(&lay(&rows(0.9), &[]), &square(), None).unwrap();
        let dense = CheckMap::of(&lay(&rows(0.35), &[]), &square(), None).unwrap();
        let middle = Point::new(5.0, 5.0);
        assert!(open.open_at(middle) > 0.3, "{}", open.open_at(middle));
        assert!(dense.open_at(middle) < 0.1, "{}", dense.open_at(middle));
        let places = open.show_through(DEFAULT_SHOW_LIMIT);
        assert!(!places.is_empty());
        assert!(places[0].area_mm2 > 20.0, "{:?}", places[0]);
        assert!(dense.show_through(DEFAULT_SHOW_LIMIT).is_empty());
        // The edge of the area and the fabric round it are not checked.
        assert_eq!(open.open_at(Point::new(0.2, 5.0)), -1.0);
        assert_eq!(open.open_at(Point::new(-1.0, 5.0)), -1.0);
    }

    #[test]
    fn many_layers_are_high_thread() {
        // The same rows sewn twelve times on top of each other.
        let one = rows(0.35);
        let mut many = Vec::new();
        for _ in 0..12 {
            many.extend(one.iter().cloned());
        }
        let low = CheckMap::of(&lay(&one, &[]), &square(), None).unwrap();
        let high = CheckMap::of(&lay(&many, &[]), &square(), None).unwrap();
        assert!(low.max_thread() < DEFAULT_HEIGHT_MM, "{}", low.max_thread());
        assert!(high.max_thread() > DEFAULT_HEIGHT_MM, "{}", high.max_thread());
        assert!(low.high(DEFAULT_HEIGHT_MM).is_empty());
        assert!(!high.high(DEFAULT_HEIGHT_MM).is_empty());
    }

    #[test]
    fn foam_is_not_thread() {
        let puff: Vec<Strand> = rows(0.35)
            .into_iter()
            .map(|s| Strand {
                lift_mm: crate::strand::FOAM_MM,
                ..s
            })
            .collect();
        let m = CheckMap::of(&lay(&puff, &[]), &square(), None).unwrap();
        // The stitches are high over the fabric, but the thread on the foam
        // is not.
        assert!(lay(&puff, &[]).top() > 2.0);
        assert!(m.max_thread() < 0.6, "{}", m.max_thread());
    }

    #[test]
    fn erode_takes_the_edge_away() {
        let mut mask = vec![false; 10 * 10];
        for y in 2..8 {
            for x in 2..8 {
                mask[y * 10 + x] = true;
            }
        }
        let e = erode(&mask, 10, 10, 1);
        assert!(e[5 * 10 + 5] && e[3 * 10 + 3]);
        assert!(!e[2 * 10 + 5] && !e[5 * 10 + 7]);
    }
}
