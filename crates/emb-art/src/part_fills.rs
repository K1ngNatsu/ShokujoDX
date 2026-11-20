//! Fills from the parts of a stitch file (P18 Milestone 3, ADR 0133): the
//! rows of a fill become a fill object with its area, its holes, its
//! angle, its row spacing, its stitch length, the offset of its needle
//! points and its underlay.
//!
//! Not a port. The method:
//!
//! - **Rows**: in a part of rows, the legs at the mean angle of the part
//!   (within [`ROW_DEG`]) are rows; the other legs turn from one row to the
//!   next.
//! - **Settings**: the angle is the mean direction of the rows (long rows
//!   count more). The row spacing is the median distance between two rows
//!   one after the other. The stitch length is the median length of the
//!   stitches in the rows (not the first and the last of a row). The offset
//!   of the needle points: the stitch engine puts the needle points of row
//!   `k` at `(n + (k mod staggers) / staggers) · length`; the number of
//!   staggers that fits the needle points best gives a tatami fill (4) or
//!   a square fill (1).
//! - **Area**: the rows in the frame of the rows. Between two rows next to
//!   each other, the area goes from the ends of one row to the ends of the
//!   other; at a row with no row next to it, the area goes half a row
//!   spacing out. The area is drawn in cells and traced (as Recognize
//!   Outlines, ADR 0030), so the outline goes through the ends of the rows;
//!   a gap across the rows of about one row (a row that went with another
//!   part) is closed.
//!   The pull compensation is 0: the row ends have it already.
//! - **Groups**: parts of rows of one thread with the same angle (within
//!   [`SAME_DEG`]) and spacing (within [`SAME_SPACING`]) one after the other
//!   are one fill (the sections of a fill); a short run between them is
//!   travel, and a column part with rows at that angle and spacing is rows
//!   of the fill (a narrow section with rows of one stitch). A fill with
//!   rows in more than one direction becomes more than one fill. Each traced
//!   area is a fill object.
//! - **Underlay**: a piece of underlay along the outline (its legs stay at
//!   one distance from it) is an edge walk. In the other pieces, the legs
//!   across the inside are rows (in a narrow shape a row is one stitch or a
//!   few); the rows of one direction, over the pieces one after the other,
//!   are a tatami layer (its angle to the top rows, its spacing, and its
//!   inset: the distance of its row ends from the outline) if they are
//!   lighter than the top rows and cover 40 % of the area. Other pieces are
//!   travel. When the layers are the layers of an underlay setting
//!   (Auto, Light, Heavy, Double) of the shape, the fill gets that setting.

use std::ops::Range;

use emb_geom::{Path, Point, Polyline, Transform};
use emb_model::{FillParams, FillPattern, ObjectKind, StitchDesign, Underlay, UnderlayLayer};

use crate::autotrace::{self, TraceSettings};
use crate::part_objects::{Item, clean, points_of, run_item};
use crate::parts::{self, Part, PartKind};
use crate::reduce::{NONE, Reduced};

/// A leg within this angle of the mean angle of its part is a row, degrees.
pub const ROW_DEG: f64 = 12.0;
/// A row is at least this long, mm.
const MIN_ROW_MM: f64 = 0.5;
/// Parts of rows with angles within this, degrees, and spacings within
/// this part are the rows of one fill.
pub const SAME_DEG: f64 = 3.0;
pub const SAME_SPACING: f64 = 0.1;
/// A run of at most this many stitches between two parts of one fill is
/// travel.
const TRAVEL_STITCHES: usize = 40;
/// An angle this near to a whole degree is the whole degree, degrees.
const SNAP_DEG: f64 = 0.3;
/// The offset of the needle points is found if this part of the needle
/// points fit it (the length of the mean of their phases).
const STAGGER_FIT: f64 = 0.6;
/// The numbers of staggers that are tried.
const MAX_STAGGERS: u32 = 6;
/// The cells of the area are at most this size, mm, and at most this many.
const CELL_MM: f64 = 0.1;
const MAX_CELLS: f64 = 3.0e6;
/// A needle point of a row is on the line of the row within this part of
/// the row spacing.
const ON_ROW: f64 = 0.35;
/// A gap across the rows of the area of at most this many row spacings is
/// closed (a row that went with another part).
const SLIT: f64 = 1.2;
/// Areas smaller than this are left out, mm².
const MIN_AREA_MM2: f64 = 1.0;
/// An edge walk is this near to the outline (median), mm, and at least this
/// part of its length; this part of its length is in legs along the outline
/// (their ends and middle within [`EDGE_BAND_MM`] of the distance of the
/// run).
const EDGE_NEAR_MM: f64 = 1.5;
const EDGE_LENGTH: f64 = 0.7;
const ALONG: f64 = 0.8;
const EDGE_BAND_MM: f64 = 0.4;
/// Underlay rows: this part of them is at their mean angle; their spacing is
/// at most this, mm.
const ALONG_ROWS: f64 = 0.7;
const MAX_UNDERLAY_SPACING_MM: f64 = 6.0;
/// An underlay layer covers this part of the area inside its inset (its
/// rows, each a band of its spacing).
const UNDERLAY_COVER: f64 = 0.4;
/// Underlay layers that are the same: angles within this, degrees; spacing
/// and inset as for the columns.
const LAYER_DEG: f64 = 3.0;
const LAYER_SPACING: f64 = 0.15;
const LAYER_INSET_MM: f64 = 0.3;

/// What the rows of a fill show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowsShape {
    /// The direction of the rows (as the fill angle: 0° along +X, positive
    /// clockwise on screen), from −90° to 90°.
    pub angle_deg: f64,
    pub spacing_mm: f64,
    pub stitch_length_mm: f64,
    /// The number of rows until the offset of the needle points repeats
    /// (4: tatami, 1: square), if the needle points show one.
    pub staggers: Option<u32>,
}

impl RowsShape {
    /// The rows of `other` can be rows of the same fill.
    pub fn same_fill(&self, other: &RowsShape) -> bool {
        line_diff(self.angle_deg, other.angle_deg) <= SAME_DEG
            && (other.spacing_mm / self.spacing_mm - 1.0).abs() <= SAME_SPACING
    }

    /// The fill angle to keep: a whole degree if it is near one, else to
    /// 0.1°.
    pub fn angle_to_keep(&self) -> f64 {
        let whole = self.angle_deg.round();
        let a = if (self.angle_deg - whole).abs() <= SNAP_DEG {
            whole
        } else {
            (self.angle_deg * 10.0).round() / 10.0
        };
        if a <= -90.0 { a + 180.0 } else { a }
    }
}

/// The difference of two directions as lines (0° to 90°).
pub fn line_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

/// An angle as a line, from −90° (not included) to 90°.
fn as_line(a: f64) -> f64 {
    let a = a.rem_euclid(180.0);
    if a > 90.0 { a - 180.0 } else { a }
}

/// The angle of the vector `(x, y)`, degrees.
fn angle_of(x: f64, y: f64) -> f64 {
    emb_geom::math::to_degrees(emb_geom::math::atan2(y, x))
}

/// The mean direction of lines (as lines), with weights, degrees.
fn mean_line(lines: impl Iterator<Item = (f64, f64, f64)>) -> Option<f64> {
    let (mut c, mut s) = (0.0, 0.0);
    for (x, y, w) in lines {
        let a2 = 2.0 * emb_geom::math::atan2(y, x);
        c += w * emb_geom::math::cos(a2);
        s += w * emb_geom::math::sin(a2);
    }
    (c != 0.0 || s != 0.0).then(|| as_line(emb_geom::math::to_degrees(emb_geom::math::atan2(s, c)) / 2.0))
}

/// A layer of rows: rows in one direction, one after the other.
#[derive(Debug, Clone)]
pub struct RowLayer {
    /// The needle points of each row, in sew order.
    pub rows: Vec<Vec<Point>>,
    /// The stitches from the first row to the last (stitch `k` ends at
    /// point `k + 1` of the points).
    pub stitches: Range<usize>,
}

/// The rows of the needle points of a part of rows, in layers (see the
/// module text): a leg is a row if it is long enough and not a turn (a leg
/// between two rows that go back and forth); a new layer starts
/// where the direction of the rows changes and the next row keeps it (else
/// the leg is not a row).
pub fn row_layers(points: &[Point]) -> Vec<RowLayer> {
    let (pts, index) = clean(points);
    let legs = parts::legs(&pts);
    let n = legs.len();
    // A turn is between two rows that go back and forth (where the edge
    // is almost along the rows, a turn is longer than the rows).
    let turn = |k: usize| k > 0 && k + 1 < n && parts::dot(legs[k - 1].dir(), legs[k + 1].dir()) < parts::BACK;
    let rows: Vec<usize> = (0..n).filter(|&k| legs[k].len() >= MIN_ROW_MM && !turn(k)).collect();
    let angle = |k: usize| angle_of(legs[k].b.x - legs[k].a.x, legs[k].b.y - legs[k].a.y);
    let mut layers: Vec<Vec<usize>> = Vec::new();
    // The mean direction of the layer, as the sums of the doubled angles.
    let (mut c, mut s) = (0.0, 0.0);
    for (i, &k) in rows.iter().enumerate() {
        let a = angle(k);
        let mean = (c != 0.0 || s != 0.0).then(|| emb_geom::math::to_degrees(emb_geom::math::atan2(s, c)) / 2.0);
        let new_layer = match mean {
            None => true,
            Some(m) if line_diff(a, m) <= ROW_DEG => false,
            Some(_) => {
                // A new direction that the next row keeps, else not a row.
                if rows.get(i + 1).is_none_or(|&j| line_diff(angle(j), a) > ROW_DEG) {
                    continue;
                }
                true
            }
        };
        if new_layer {
            layers.push(Vec::new());
            (c, s) = (0.0, 0.0);
        }
        let a2 = 2.0 * emb_geom::math::to_radians(a);
        c += legs[k].len() * emb_geom::math::cos(a2);
        s += legs[k].len() * emb_geom::math::sin(a2);
        if let Some(l) = layers.last_mut() {
            l.push(k);
        }
    }
    layers
        .into_iter()
        .map(|ks| RowLayer {
            rows: ks.iter().map(|&k| pts[legs[k].start..=legs[k].end].to_vec()).collect(),
            stitches: index[legs[ks[0]].start]..index[legs[ks[ks.len() - 1]].end],
        })
        .collect()
}

/// All the rows of the needle points of a part of rows (all layers).
pub fn rows_of(points: &[Point]) -> Vec<Vec<Point>> {
    row_layers(points).into_iter().flat_map(|l| l.rows).collect()
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

/// The settings that rows show (see the module text); `None` for fewer
/// than three rows.
pub fn rows_shape(rows: &[Vec<Point>]) -> Option<RowsShape> {
    if rows.len() < 3 {
        return None;
    }
    let ends = |r: &Vec<Point>| (r[0], r[r.len() - 1]);
    let angle_deg = mean_line(rows.iter().map(|r| {
        let (a, b) = ends(r);
        (b.x - a.x, b.y - a.y, a.distance(b).powi(2))
    }))?;
    let to_rows = Transform::rotate_deg(-angle_deg);
    let v_of = |r: &Vec<Point>| row_line(&r.iter().map(|p| to_rows.apply(*p)).collect::<Vec<_>>());
    let vs: Vec<f64> = rows.iter().map(v_of).collect();
    let steps: Vec<f64> = vs
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .filter(|d| *d >= 0.05)
        .collect();
    let first = median(steps.clone())?;
    let near: Vec<f64> = steps
        .into_iter()
        .filter(|d| (0.7 * first..=1.3 * first).contains(d))
        .collect();
    let spacing_mm = near.iter().sum::<f64>() / near.len().max(1) as f64;
    if spacing_mm <= 0.0 {
        return None;
    }
    // The stitches inside the rows (a row starts and ends with a part of a
    // stitch).
    let inner: Vec<f64> = rows
        .iter()
        .filter(|r| r.len() > 3)
        .flat_map(|r| r[1..r.len() - 1].windows(2).map(|w| w[0].distance(w[1])))
        .collect();
    let all: Vec<f64> = rows
        .iter()
        .flat_map(|r| r.windows(2).map(|w| w[0].distance(w[1])))
        .collect();
    let length = median(inner).or_else(|| median(all))?;
    let stitch_length_mm = ((length * 10.0).round() / 10.0).max(0.5);
    let staggers = staggers(rows, &vs, &to_rows, spacing_mm, stitch_length_mm);
    Some(RowsShape {
        angle_deg,
        spacing_mm,
        stitch_length_mm,
        staggers,
    })
}

/// The number of staggers that fits the needle points inside the rows best
/// (see the module text), if it fits well.
fn staggers(rows: &[Vec<Point>], vs: &[f64], to_rows: &Transform, spacing: f64, len: f64) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for n in 1..=MAX_STAGGERS {
        let (mut c, mut s, mut count) = (0.0, 0.0, 0usize);
        for (r, v) in rows.iter().zip(vs) {
            if r.len() < 3 {
                continue;
            }
            let k = (v / spacing - 0.5).round() as i64;
            let phase = k.rem_euclid(i64::from(n)) as f64 / f64::from(n);
            for p in &r[1..r.len() - 1] {
                let f = to_rows.apply(*p).x / len - phase;
                let a = std::f64::consts::TAU * f;
                c += emb_geom::math::cos(a);
                s += emb_geom::math::sin(a);
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        let fit = (c * c + s * s).sqrt() / count as f64;
        if best.is_none_or(|b| fit > b.0 + 1e-9) {
            best = Some((fit, n));
        }
    }
    best.filter(|b| b.0 >= STAGGER_FIT).map(|b| b.1)
}

/// The area of rows at `angle_deg` and `spacing` (see the module text): the
/// outline and the holes of each part of it, in the coordinates of the
/// rows.
pub fn rows_area(rows: &[Vec<Point>], angle_deg: f64, spacing: f64) -> Vec<(Path, Vec<Path>)> {
    let to_rows = Transform::rotate_deg(-angle_deg);
    // Each row as (v, u0, u1) in the frame of the rows.
    let mut segs: Vec<(f64, f64, f64)> = rows
        .iter()
        .filter(|r| r.len() >= 2)
        .map(|r| {
            let t: Vec<Point> = r.iter().map(|p| to_rows.apply(*p)).collect();
            let v = row_line(&t);
            // The points on the line of the row (not a turn that the leg
            // has at its end, where the edge is almost along the rows).
            let on: Vec<&Point> = t.iter().filter(|p| (p.y - v).abs() <= ON_ROW * spacing).collect();
            let on = if on.len() >= 2 { on } else { t.iter().collect() };
            let u0 = on.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let u1 = on.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            (v, u0, u1)
        })
        .collect();
    if segs.is_empty() {
        return Vec::new();
    }
    segs.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Rows on one line (a row cut by a hole has more than one segment).
    let mut lines: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
    for (v, u0, u1) in segs {
        match lines.last_mut() {
            Some((lv, l)) if v - *lv < 0.5 * spacing => {
                *lv = (*lv * l.len() as f64 + v) / (l.len() + 1) as f64;
                l.push((u0, u1));
            }
            _ => lines.push((v, vec![(u0, u1)])),
        }
    }
    // The pieces of the area: between two rows next to each other, and half
    // a row out at a row with no row next to it.
    let overlap = |a: (f64, f64), b: (f64, f64)| a.0 < b.1 && b.0 < a.1;
    let next_to = |i: usize, j: usize| (lines[j].0 - lines[i].0).abs() <= 1.6 * spacing;
    let mut pieces: Vec<[Point; 4]> = Vec::new();
    let half = 0.5 * spacing;
    for i in 0..lines.len() {
        let (v, l) = &lines[i];
        for &a in l {
            let above = i > 0 && next_to(i - 1, i) && lines[i - 1].1.iter().any(|&b| overlap(a, b));
            let below = i + 1 < lines.len() && next_to(i, i + 1) && lines[i + 1].1.iter().any(|&b| overlap(a, b));
            if !above {
                pieces.push([
                    Point::new(a.0, v - half),
                    Point::new(a.1, v - half),
                    Point::new(a.1, *v),
                    Point::new(a.0, *v),
                ]);
            }
            if !below {
                pieces.push([
                    Point::new(a.0, *v),
                    Point::new(a.1, *v),
                    Point::new(a.1, v + half),
                    Point::new(a.0, v + half),
                ]);
            }
            if i + 1 < lines.len() && next_to(i, i + 1) {
                let (w, m) = &lines[i + 1];
                for &b in m.iter().filter(|&&b| overlap(a, b)) {
                    pieces.push([
                        Point::new(a.0, *v),
                        Point::new(a.1, *v),
                        Point::new(b.1, *w),
                        Point::new(b.0, *w),
                    ]);
                }
            }
        }
    }
    // The cells.
    let (mut lo, mut hi) = (
        Point::new(f64::INFINITY, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in pieces.iter().flatten() {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let margin = 2.0 * CELL_MM;
    let (bw, bh) = (hi.x - lo.x + 2.0 * margin, hi.y - lo.y + 2.0 * margin);
    let cell = CELL_MM
        .max((bw * bh / MAX_CELLS).sqrt())
        .min(0.25 * spacing.max(CELL_MM * 4.0));
    let cell = cell.max((bw * bh / MAX_CELLS).sqrt());
    let origin = Point::new(lo.x - margin, lo.y - margin);
    let w = (bw / cell).ceil() as usize + 1;
    let h = (bh / cell).ceil() as usize + 1;
    let mut on = vec![false; w * h];
    for q in &pieces {
        let (x0, x1) = (
            q.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            q.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
        );
        let (y0, y1) = (
            q.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            q.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
        );
        let cx0 = ((x0 - origin.x) / cell).floor().max(0.0) as usize;
        let cx1 = (((x1 - origin.x) / cell).ceil() as usize).min(w);
        let cy0 = ((y0 - origin.y) / cell).floor().max(0.0) as usize;
        let cy1 = (((y1 - origin.y) / cell).ceil() as usize).min(h);
        for y in cy0..cy1 {
            let py = origin.y + (y as f64 + 0.5) * cell;
            for x in cx0..cx1 {
                let px = origin.x + (x as f64 + 0.5) * cell;
                if inside(q, Point::new(px, py)) {
                    on[y * w + x] = true;
                }
            }
        }
    }
    // A gap across the rows no thicker than about a row is a row that is
    // not in the rows (it went with another part): close it.
    let gap = (SLIT * spacing / cell).round() as usize;
    for x in 0..w {
        let mut last_on: Option<usize> = None;
        for y in 0..h {
            if on[y * w + x] {
                if let Some(l) = last_on
                    && y - l > 1
                    && y - l - 1 <= gap
                {
                    for yy in l + 1..y {
                        on[yy * w + x] = true;
                    }
                }
                last_on = Some(y);
            }
        }
    }
    let r = Reduced {
        width: w as u32,
        height: h as u32,
        palette: vec![emb_model::Rgb(0, 0, 0)],
        labels: on.iter().map(|&c| if c { 0 } else { NONE }).collect(),
    };
    let s = TraceSettings {
        min_area_px: (MIN_AREA_MM2 / (cell * cell)) as usize,
        ..TraceSettings::default()
    };
    let back = Transform::scale(cell, cell)
        .then(Transform::translate(origin.x, origin.y))
        .then(Transform::rotate_deg(angle_deg));
    autotrace::trace(&r, &s)
        .into_iter()
        .map(|a| {
            let a = a.transformed(&back);
            (a.outer, a.holes)
        })
        .collect()
}

/// The line of a row in the frame of the rows: the median of its needle
/// points (a turn at the end of the leg does not move it).
fn row_line(t: &[Point]) -> f64 {
    median(t.iter().map(|p| p.y).collect()).unwrap_or(0.0)
}

/// `p` is inside the polygon `q` (even-odd).
fn inside(q: &[Point], p: Point) -> bool {
    let mut on = false;
    for i in 0..q.len() {
        let (a, b) = (q[i], q[(i + 1) % q.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            on = !on;
        }
    }
    on
}

/// The rings of an area (the outline first), flattened.
fn rings(outer: &Path, holes: &[Path]) -> Vec<Vec<Point>> {
    std::iter::once(outer)
        .chain(holes)
        .flat_map(|p| p.flatten(0.05))
        .map(|l| l.points)
        .filter(|r| r.len() >= 3)
        .collect()
}

/// `p` is in the area of the rings (even-odd).
fn in_rings(rings: &[Vec<Point>], p: Point) -> bool {
    rings.iter().filter(|r| inside(r, p)).count() % 2 == 1
}

/// The distance from `p` to the nearest ring.
fn to_rings(rings: &[Vec<Point>], p: Point) -> f64 {
    rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| p.distance_to_segment(r[i], r[(i + 1) % r.len()])))
        .fold(f64::INFINITY, f64::min)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The underlay layers of a fill of `rings` with the top rows `top` from the
/// needle points of its underlay (in sew order; see the module text): an
/// edge walk, or tatami layers from the legs across the inside, gathered
/// over the pieces of underlay while their direction stays the same.
fn layers(rings: &[Vec<Point>], top: &RowsShape, under: &[(Range<usize>, Vec<Point>)]) -> Vec<UnderlayLayer> {
    let perimeter: f64 = rings
        .iter()
        .map(|r| (0..r.len()).map(|i| r[i].distance(r[(i + 1) % r.len()])).sum::<f64>())
        .sum();
    let area = area_of_rings(rings);
    let mut out: Vec<UnderlayLayer> = Vec::new();
    // The rows of one direction, gathered over the pieces.
    let mut rows: Vec<Vec<Point>> = Vec::new();
    let flush = |rows: &mut Vec<Vec<Point>>, out: &mut Vec<UnderlayLayer>| {
        if let Some(layer) = tatami_of(rings, top, area, perimeter, rows) {
            // The sections of one tatami layer are one layer.
            if !out.last().is_some_and(|x| alike(x, &layer)) {
                out.push(layer);
            }
        }
        rows.clear();
    };
    for (_, pts) in under {
        let (pts, _) = clean(pts);
        let Some(d) = median(pts.iter().map(|p| to_rings(rings, *p)).collect()) else {
            continue;
        };
        // The legs along the outline: their ends and their middle are at
        // about the same distance from it as the needle points.
        let legs = parts::legs(&pts);
        let near = |p: Point| to_rings(rings, p) <= d + EDGE_BAND_MM;
        let along = |l: &parts::Leg| near(l.a) && near(l.b) && near(l.a.midpoint(l.b));
        let length: f64 = legs.iter().map(|l| l.len()).sum();
        let along_length: f64 = legs.iter().filter(|l| along(l)).map(|l| l.len()).sum();
        if along_length >= ALONG * length && d <= EDGE_NEAR_MM && length >= EDGE_LENGTH * perimeter {
            flush(&mut rows, &mut out);
            out.push(UnderlayLayer::EdgeWalk { inset_mm: round2(d) });
            continue;
        }
        // The legs across the inside are rows (in a narrow shape a row is
        // one stitch or a few); a new direction that the next row keeps
        // starts a new layer.
        let across: Vec<&parts::Leg> = legs.iter().filter(|l| !along(l) && l.len() >= MIN_ROW_MM).collect();
        let angle = |l: &parts::Leg| angle_of(l.b.x - l.a.x, l.b.y - l.a.y);
        for (i, l) in across.iter().enumerate() {
            let a = angle(l);
            let mean = mean_line(rows.iter().map(|r| {
                let (p, q) = (r[0], r[r.len() - 1]);
                (q.x - p.x, q.y - p.y, p.distance(q))
            }));
            if let Some(m) = mean
                && line_diff(a, m) > ROW_DEG
            {
                if across.get(i + 1).is_none_or(|n| line_diff(angle(n), a) > ROW_DEG) {
                    continue;
                }
                flush(&mut rows, &mut out);
            }
            rows.push(pts[l.start..=l.end].to_vec());
        }
    }
    flush(&mut rows, &mut out);
    out
}

/// The area of rings (even-odd: the holes less), mm².
fn area_of_rings(rings: &[Vec<Point>]) -> f64 {
    let ring = |r: &Vec<Point>| {
        (0..r.len())
            .map(|i| r[i].x * r[(i + 1) % r.len()].y - r[(i + 1) % r.len()].x * r[i].y)
            .sum::<f64>()
            .abs()
            / 2.0
    };
    let mut areas: Vec<f64> = rings.iter().map(ring).collect();
    areas.sort_by(|a, b| b.total_cmp(a));
    // The largest ring is the outline; a ring inside it is a hole.
    areas.first().copied().unwrap_or(0.0) - areas.iter().skip(1).sum::<f64>()
}

/// A tatami underlay layer of rows under a fill of `rings` (its `area` and
/// `perimeter`) with the top rows `top`: its angle to the top rows, its
/// spacing, and its inset (the distance of its row ends from the outline).
/// `None` if the rows do not show one: too few, not in one direction, not
/// lighter than the top rows, farther apart than
/// [`MAX_UNDERLAY_SPACING_MM`], or a piece that covers less than
/// [`UNDERLAY_COVER`] of the area inside the inset.
fn tatami_of(
    rings: &[Vec<Point>],
    top: &RowsShape,
    area: f64,
    perimeter: f64,
    rows: &[Vec<Point>],
) -> Option<UnderlayLayer> {
    let shape = rows_shape(rows)?;
    // Lighter than the top rows, and not travel far apart.
    if shape.spacing_mm * LIGHTER < top.spacing_mm || shape.spacing_mm > MAX_UNDERLAY_SPACING_MM {
        return None;
    }
    let at_angle = rows
        .iter()
        .filter(|r| {
            let (a, b) = (r[0], r[r.len() - 1]);
            line_diff(angle_of(b.x - a.x, b.y - a.y), shape.angle_deg) <= ROW_DEG
        })
        .count();
    if (at_angle as f64) < ALONG_ROWS * rows.len() as f64 {
        return None;
    }
    let ends: Vec<f64> = rows
        .iter()
        .flat_map(|r| [r[0], r[r.len() - 1]])
        .map(|p| to_rings(rings, p))
        .collect();
    let inset = median(ends)?;
    // The rows cover the area inside the inset (each row a band of the
    // spacing).
    let covered: f64 = rows
        .iter()
        .map(|r| r[0].distance(r[r.len() - 1]) * shape.spacing_mm)
        .sum();
    let inside = (area - perimeter * inset).max(0.25 * area);
    if covered < UNDERLAY_COVER * inside {
        return None;
    }
    let angle = as_line(shape.angle_deg - top.angle_deg).round();
    Some(UnderlayLayer::Tatami {
        inset_mm: round2(inset),
        spacing_mm: round2(shape.spacing_mm),
        angle_deg: if angle <= -90.0 { 90.0 } else { angle },
    })
}

/// Two underlay layers are the same (angles as lines).
fn alike(a: &UnderlayLayer, b: &UnderlayLayer) -> bool {
    match (a, b) {
        (UnderlayLayer::EdgeWalk { inset_mm: x }, UnderlayLayer::EdgeWalk { inset_mm: y }) => {
            (x - y).abs() <= LAYER_INSET_MM
        }
        (
            UnderlayLayer::Tatami {
                inset_mm: x,
                spacing_mm: s,
                angle_deg: p,
            },
            UnderlayLayer::Tatami {
                inset_mm: y,
                spacing_mm: t,
                angle_deg: q,
            },
        ) => (x - y).abs() <= LAYER_INSET_MM && (s / t - 1.0).abs() <= LAYER_SPACING && line_diff(*p, *q) <= LAYER_DEG,
        _ => false,
    }
}

/// The underlay setting of the layers for the shape of `rings`: the
/// setting of the engine that has these layers, else the layers.
fn underlay_setting(found: Vec<UnderlayLayer>, rings: &[Vec<Point>]) -> Underlay {
    use emb_stitch::object::{auto_fill_underlay, heavy_fill_underlay, light_fill_underlay};
    let polys: Vec<Polyline> = rings
        .iter()
        .map(|r| Polyline {
            points: r.clone(),
            closed: true,
        })
        .collect();
    let (outers, holes) = polys.split_at(1.min(polys.len()));
    let shape = emb_stitch::travel::Shape::new(outers, holes);
    let auto = auto_fill_underlay(&shape);
    let double: Vec<UnderlayLayer> = match auto.first() {
        Some(UnderlayLayer::Tatami {
            inset_mm, spacing_mm, ..
        }) => [45.0, -45.0]
            .iter()
            .map(|&angle_deg| UnderlayLayer::Tatami {
                inset_mm: *inset_mm,
                spacing_mm: *spacing_mm,
                angle_deg,
            })
            .collect(),
        _ => Vec::new(),
    };
    let same = |set: &[UnderlayLayer]| set.len() == found.len() && set.iter().zip(&found).all(|(a, b)| alike(a, b));
    for (setting, set) in [
        (Underlay::Auto, auto.clone()),
        (Underlay::Light, light_fill_underlay(&shape)),
        (Underlay::Heavy, heavy_fill_underlay(&shape)),
        (Underlay::Double, double),
    ] {
        if same(&set) {
            return setting;
        }
    }
    if found.is_empty() {
        Underlay::None
    } else {
        Underlay::Custom(found)
    }
}

/// The rows of a fill from one part, with the records that go with them
/// and the needle points of their underlay.
pub(crate) struct Member {
    rows: Vec<Vec<Point>>,
    records: Vec<Range<usize>>,
    under: Vec<(Range<usize>, Vec<Point>)>,
}

/// A group of layers of rows that are one fill.
pub(crate) struct FillGroup {
    pub thread: usize,
    pub shape: RowsShape,
    pub members: Vec<Member>,
    /// Short runs between them (travel), parts.
    pub travel: Vec<usize>,
}

/// A layer of rows is underlay if rows after it have less than this part
/// of its row spacing (as in `parts`).
const LIGHTER: f64 = 0.8;

/// The settings of a part of rows: of its densest layer.
pub(crate) fn part_shape(s: &StitchDesign, part: &Part) -> Option<RowsShape> {
    row_layers(&points_of(s, &part.records))
        .iter()
        .filter_map(|l| rows_shape(&l.rows))
        .min_by(|a, b| a.spacing_mm.total_cmp(&b.spacing_mm))
}

/// The fills of a part of rows `i`: each layer that no denser layer comes
/// after is the rows of a fill; the lighter layers before it, and for the
/// first the parts of the underlay of the part (`under`), are its underlay.
/// `None` if no layer shows its settings.
pub(crate) fn part_members(
    s: &StitchDesign,
    parts: &[Part],
    i: usize,
    under: &[usize],
) -> Option<Vec<(RowsShape, Member)>> {
    let part = &parts[i];
    let pts = points_of(s, &part.records);
    let layers = row_layers(&pts);
    let shapes: Vec<Option<RowsShape>> = layers.iter().map(|l| rows_shape(&l.rows)).collect();
    let base = part.records.start;
    // The records of each layer: from its first row to the first row of
    // the next layer (the first layer from the start of the part).
    let ranges: Vec<Range<usize>> = (0..layers.len())
        .map(|k| {
            let start = if k == 0 { base } else { base + layers[k].stitches.start };
            let end = match layers.get(k + 1) {
                Some(next) => base + next.stitches.start,
                None => part.records.end,
            };
            start..end.max(start)
        })
        .collect();
    let top = |k: usize| {
        shapes[k].is_some_and(|sk| {
            !shapes[k + 1..]
                .iter()
                .flatten()
                .any(|later| later.spacing_mm < LIGHTER * sk.spacing_mm)
        })
    };
    let mut out: Vec<(RowsShape, Member)> = Vec::new();
    let mut records: Vec<Range<usize>> = under.iter().map(|&u| parts[u].records.clone()).collect();
    let mut under_pts: Vec<(Range<usize>, Vec<Point>)> = under
        .iter()
        .map(|&u| (parts[u].records.clone(), points_of(s, &parts[u].records)))
        .collect();
    for (k, l) in layers.iter().enumerate() {
        records.push(ranges[k].clone());
        if let (true, Some(shape)) = (top(k), shapes[k]) {
            out.push((
                shape,
                Member {
                    rows: l.rows.clone(),
                    records: std::mem::take(&mut records),
                    under: std::mem::take(&mut under_pts),
                },
            ));
        } else {
            let end = (l.stitches.end + 1).min(pts.len());
            under_pts.push((ranges[k].clone(), pts[l.stitches.start.min(end)..end].to_vec()));
        }
    }
    let last = out.last_mut()?;
    // What comes after the last fill of the part.
    last.1.records.extend(records.into_iter().filter(|r| !r.is_empty()));
    if layers.is_empty() {
        return None;
    }
    Some(out)
}

/// A run part that can be travel between two parts of one fill.
pub(crate) fn can_be_travel(part: &Part) -> bool {
    part.kind == PartKind::Run && part.records.len() <= TRAVEL_STITCHES
}

/// The items of a group of rows: a fill object for each part of their
/// area, with the records of the rows, of their underlay and of the travel.
pub(crate) fn fill_items(s: &StitchDesign, parts: &[Part], g: &FillGroup, items: &mut Vec<Item>) {
    let all: Vec<Vec<Point>> = g.members.iter().flat_map(|m| m.rows.iter().cloned()).collect();
    let stitches = |items: &mut Vec<Item>| {
        let mut records: Vec<Range<usize>> = g.members.iter().flat_map(|m| m.records.iter().cloned()).collect();
        records.extend(g.travel.iter().map(|&t| parts[t].records.clone()));
        records.sort_by_key(|r| r.start);
        for r in records.into_iter().filter(|r| !r.is_empty()) {
            items.push(Item::Stitches(g.thread, r));
        }
    };
    let Some(shape) = rows_shape(&all) else {
        stitches(items);
        return;
    };
    // The rows at the fill angle in the runs between the parts (a section
    // that starts with a row or two before it turns) are a part of the area.
    let mut area_rows = all.clone();
    for &t in &g.travel {
        area_rows.extend(rows_of(&points_of(s, &parts[t].records)).into_iter().filter(|r| {
            let (a, b) = (r[0], r[r.len() - 1]);
            line_diff(angle_of(b.x - a.x, b.y - a.y), shape.angle_deg) <= ROW_DEG
        }));
    }
    let areas = rows_area(&area_rows, shape.angle_deg, shape.spacing_mm);
    if areas.is_empty() {
        stitches(items);
        return;
    }
    let area_rings: Vec<Vec<Vec<Point>>> = areas.iter().map(|(o, h)| rings(o, h)).collect();
    // The area of a point: the area that has it, else the nearest.
    let area_of = |p: Point| -> usize {
        (0..areas.len())
            .find(|&k| in_rings(&area_rings[k], p))
            .unwrap_or_else(|| {
                (0..areas.len())
                    .min_by(|&a, &b| to_rings(&area_rings[a], p).total_cmp(&to_rings(&area_rings[b], p)))
                    .unwrap_or(0)
            })
    };
    // The records and the underlay of each area.
    let mut records: Vec<Vec<Range<usize>>> = vec![Vec::new(); areas.len()];
    let mut under_pts: Vec<Vec<(Range<usize>, Vec<Point>)>> = vec![Vec::new(); areas.len()];
    for m in &g.members {
        let k = m.rows.first().map_or(0, |r| area_of(r[0].midpoint(r[r.len() - 1])));
        records[k].extend(m.records.iter().cloned());
        under_pts[k].extend(m.under.iter().cloned());
    }
    for &t in &g.travel {
        let pts = points_of(s, &parts[t].records);
        let mid = pts[pts.len() / 2];
        let k = area_of(mid);
        if in_rings(&area_rings[k], mid) {
            records[k].push(parts[t].records.clone());
        } else {
            run_item(s, &parts[t], items);
        }
    }
    let pattern = if shape.staggers == Some(1) {
        FillPattern::Square
    } else {
        FillPattern::Tatami
    };
    for (k, (outer, holes)) in areas.into_iter().enumerate() {
        let mut recs: Vec<Range<usize>> = std::mem::take(&mut records[k])
            .into_iter()
            .filter(|r| !r.is_empty())
            .collect();
        if recs.is_empty() {
            continue;
        }
        recs.sort_by_key(|r| r.start);
        let found = layers(&area_rings[k], &shape, &under_pts[k]);
        let params = FillParams {
            spacing_mm: round2(shape.spacing_mm),
            angle_deg: shape.angle_to_keep(),
            stitch_length_mm: shape.stitch_length_mm,
            pull_compensation_mm: 0.0,
            underlay: underlay_setting(found, &area_rings[k]),
            pattern: pattern.clone(),
            ..FillParams::default()
        };
        items.push(Item::Object(
            Box::new(ObjectKind::Fill { outer, holes, params }),
            g.thread,
            recs,
        ));
    }
}

#[cfg(test)]
mod tests {
    use emb_model::StitchRecord;

    use super::*;
    use crate::part_objects::objects;
    use crate::parts::parts;

    fn design(points: &[Point]) -> StitchDesign {
        StitchDesign {
            threads: vec![Default::default()],
            records: points.iter().map(|p| StitchRecord::stitch(p.x, p.y)).collect(),
        }
    }

    /// The needle points of a fill of the engine.
    fn engine_fill(outer: &Path, holes: &[Path], p: &FillParams) -> Vec<Point> {
        let d = {
            let mut d = emb_model::Design::new("");
            let t = d.add_thread("t", emb_model::Rgb(0, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: outer.clone(),
                    holes: holes.to_vec(),
                    params: p.clone(),
                },
            );
            d
        };
        let plan = emb_stitch::build_plan(&d, &Default::default(), &mut Default::default());
        plan.stitches.records.iter().map(|r| r.pos).collect()
    }

    fn the_fill(made: &[crate::part_objects::Made]) -> (&Path, &Vec<Path>, &FillParams) {
        let fills: Vec<_> = made
            .iter()
            .filter_map(|m| match &m.kind {
                ObjectKind::Fill { outer, holes, params } => Some((outer, holes, params)),
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 1, "{made:?}");
        fills[0]
    }

    fn area_of(p: &Path, holes: &[Path]) -> f64 {
        let ring_area = |r: &[Point]| {
            (0..r.len())
                .map(|i| r[i].x * r[(i + 1) % r.len()].y - r[(i + 1) % r.len()].x * r[i].y)
                .sum::<f64>()
                .abs()
                / 2.0
        };
        rings(p, &[]).iter().map(|r| ring_area(r)).sum::<f64>()
            - holes
                .iter()
                .flat_map(|h| rings(h, &[]))
                .map(|r| ring_area(&r))
                .sum::<f64>()
    }

    #[test]
    fn a_fill_of_the_engine_comes_back_with_its_settings_and_area() {
        let outer = Path::ellipse(Point::new(0.0, 0.0), 20.0, 12.0);
        let hole = Path::ellipse(Point::new(5.0, 0.0), 4.0, 4.0);
        let p = FillParams {
            angle_deg: 30.0,
            spacing_mm: 0.45,
            stitch_length_mm: 3.0,
            ..FillParams::default()
        };
        let pts = engine_fill(&outer, std::slice::from_ref(&hole), &p);
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let (o, holes, q) = the_fill(&made);
        assert_eq!(q.angle_deg, 30.0, "{q:?}");
        assert!((q.spacing_mm - 0.45).abs() < 0.02, "{q:?}");
        assert_eq!(q.stitch_length_mm, 3.0, "{q:?}");
        assert_eq!(q.pattern, FillPattern::Tatami);
        assert_eq!(q.underlay, Underlay::Auto, "{q:?}");
        assert_eq!(holes.len(), 1);
        // The area: the shape less the hole, within 4 %.
        let want = area_of(&outer, std::slice::from_ref(&hole));
        let got = area_of(o, holes);
        assert!((got / want - 1.0).abs() < 0.04, "{got} {want}");
        // No stitch is lost.
        let all: usize = made.iter().flat_map(|m| m.records.iter()).map(|r| r.len()).sum();
        assert_eq!(all, s.records.len());
    }

    #[test]
    fn a_square_fill_and_a_fill_with_no_underlay() {
        let outer = Path::rect(emb_geom::BBox::new(Point::new(0.0, 0.0), Point::new(30.0, 20.0)));
        let p = FillParams {
            angle_deg: -60.0,
            underlay: Underlay::None,
            ..FillParams::default().with_pattern(FillPattern::Square)
        };
        let pts = engine_fill(&outer, &[], &p);
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let (_, _, q) = the_fill(&made);
        assert_eq!(q.pattern, FillPattern::Square, "{q:?}");
        assert_eq!(q.angle_deg, -60.0);
        assert_eq!(q.underlay, Underlay::None, "{q:?}");
    }

    #[test]
    fn rows_in_two_directions_are_two_fills() {
        let a = Path::rect(emb_geom::BBox::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0)));
        let b = Path::rect(emb_geom::BBox::new(Point::new(20.0, 0.0), Point::new(40.0, 20.0)));
        let mut pts = engine_fill(&a, &[], &FillParams::default());
        pts.extend(engine_fill(
            &b,
            &[],
            &FillParams {
                angle_deg: -45.0,
                ..FillParams::default()
            },
        ));
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let angles: Vec<f64> = made
            .iter()
            .filter_map(|m| match &m.kind {
                ObjectKind::Fill { params, .. } => Some(params.angle_deg),
                _ => None,
            })
            .collect();
        assert_eq!(angles, vec![45.0, -45.0], "{made:?}");
    }
}
