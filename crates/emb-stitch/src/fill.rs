//! Tatami fill: rows of stitches at the fill angle, sewn back and forth.
//!
//! Not a port. The method is a scan-line fill with a boustrophedon cell
//! decomposition, which is common knowledge:
//!
//! 1. Turn the shape so that the rows are horizontal.
//! 2. Rows are on a grid at `(k + 0.5) * spacing` from the origin, so that the
//!    rows of two objects with the same settings line up. Each row is cut
//!    where it crosses the edge of the shape (even-odd rule).
//! 3. A row segment continues a section if it overlaps exactly one segment of
//!    the previous row, and that segment overlaps only it. Otherwise it starts
//!    a new section. In a section, each row connects to the next at one side.
//! 4. Sections are sewn back and forth. The next section is the one with the
//!    corner nearest to the needle. Travel to it goes inside the shape (`travel`).
//! 5. Needle points in a row are on a grid along the row. The grid moves by
//!    `1 / staggers` of the stitch length from one row to the next, so the
//!    needle points make the diagonal tatami pattern and do not line up.
//! 6. Pull compensation makes each row longer at both ends.
//! 7. Patterned stitching (P5 Milestone 5) changes step 5: a program-split
//!    pattern puts the needle points of row `k` where row `k` of its tile
//!    says (on a grid of tile widths, so all objects line up); split lines
//!    (user split, carving stamps) add a needle point where a row crosses
//!    them, and remove grid points nearer than [`CROSS_GAP_MM`]. With user
//!    split only the crossings are used, and longer stitches get needle
//!    points at the stitch length.

use emb_geom::{Point, Transform};

use crate::travel::{Sewer, Shape};

/// Settings of one tatami layer (top stitching or underlay).
#[derive(Debug, Clone, PartialEq)]
pub struct Tatami {
    pub spacing_mm: f64,
    /// Row direction in degrees. 0° is along +X; positive turns clockwise on screen.
    pub angle_deg: f64,
    pub stitch_len_mm: f64,
    /// Extra row length at each end.
    pub pull_comp_mm: f64,
    /// Rows until the needle point pattern repeats. 1 makes all needle points line up.
    pub staggers: u32,
    /// Patterned stitching: a program split or split lines.
    pub split: RowSplit,
}

/// Where the needle goes in along the rows, besides the row ends.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RowSplit {
    /// A program-split pattern: the rows of the tile (fractions of its
    /// width), and the tile width in mm.
    pub pattern: Option<(Vec<Vec<f64>>, f64)>,
    /// Lines (design coordinates) where the rows get needle points.
    pub lines: Vec<Vec<Point>>,
    /// Needle points only at the lines (user split).
    pub only_lines: bool,
    /// Jagged edge: row ends move in at random by up to this much.
    pub jagged_mm: f64,
    /// Accordion spacing: rows are left out so the spacing grows to this
    /// factor across the rows.
    pub accordion: Option<emb_model::Accordion>,
    /// Only some rows are sewn (a tartan stripe or a gradient color).
    pub keep: Option<RowKeep>,
    /// Each row goes there and back on the same needle points (a lace
    /// mesh), so the next row starts at the same side.
    pub double: bool,
}

/// Which rows a tatami layer sews. `v` is the place of a row across the
/// rows (turned coordinates).
#[derive(Debug, Clone, PartialEq)]
pub enum RowKeep {
    /// The rows in these bands `(v0, v1)`. The sections stop at the rows
    /// that are left out, and each section starts a new needle path, so no
    /// stitch or travel goes across another stripe.
    Bands(Vec<(f64, f64)>),
    /// The rows of color `own` of a gradient from `v0` (place 0) to `v1`
    /// (place 1); see [`gradient_color`]. The rows of a section that are
    /// left out are passed along the edge.
    Gradient {
        v0: f64,
        v1: f64,
        stops: Vec<f64>,
        own: usize,
    },
}

/// The fraction part of the golden ratio: the colors of rows `k` and
/// `k + 1` come from places far apart in [0, 1), so the rows of each color
/// spread evenly.
const GOLDEN: f64 = 0.618_033_988_749_894_9;

/// The color of row `k` at place `t` of a gradient whose color `i` is at
/// place `stops[i]`. Between two stops (in order of place) the share of
/// the second color goes from 0 to 1; a row gets the second color when
/// its golden-ratio number is below that share. Each row has one color.
pub fn gradient_color(k: i64, t: f64, stops: &[f64]) -> usize {
    if stops.is_empty() {
        return 0;
    }
    let mut order: Vec<usize> = (0..stops.len()).collect();
    order.sort_by(|&a, &b| stops[a].total_cmp(&stops[b]).then(a.cmp(&b)));
    let first = order[0];
    if t.is_nan() || t <= stops[first] {
        return first;
    }
    for w in order.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (sa, sb) = (stops[a], stops[b]);
        if t <= sb {
            let share = if sb > sa { (t - sa) / (sb - sa) } else { 1.0 };
            let x = (k.rem_euclid(1 << 30) as f64 * GOLDEN).fract();
            return if x < share { b } else { a };
        }
    }
    order[order.len() - 1]
}

/// Seed of the jagged row ends.
const JAGGED_SEED: u64 = 0x4A46;

/// Grid points nearer than this to a split line crossing are not used, in mm.
pub const CROSS_GAP_MM: f64 = 0.5;

/// Smallest row spacing that the generator accepts, in mm.
pub const MIN_SPACING_MM: f64 = 0.1;
/// Grid points nearer than this fraction of the stitch length to a row end are not used.
const END_GAP: f64 = 0.25;

/// One row segment, in turned coordinates: row index, `v`, and the ends `u0 < u1`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seg {
    pub k: i64,
    pub v: f64,
    pub u0: f64,
    pub u1: f64,
}

/// Row segments of the turned rings, row by row.
pub(crate) fn row_segments(rings: &[Vec<Point>], spacing: f64) -> Vec<Vec<Seg>> {
    let (mut vmin, mut vmax) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in rings.iter().flatten() {
        vmin = vmin.min(p.y);
        vmax = vmax.max(p.y);
    }
    if !vmin.is_finite() || vmin >= vmax {
        return Vec::new();
    }
    let k0 = (vmin / spacing - 0.5).floor() as i64;
    let k1 = (vmax / spacing - 0.5).ceil() as i64;
    let mut rows = Vec::new();
    let mut xs: Vec<f64> = Vec::new();
    for k in k0..=k1 {
        let v = (k as f64 + 0.5) * spacing;
        xs.clear();
        for r in rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                if (a.y > v) != (b.y > v) {
                    xs.push(a.x + (v - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        let segs: Vec<Seg> = xs
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|c| c[1] - c[0] > 1e-9)
            .map(|c| Seg {
                k,
                v,
                u0: c[0],
                u1: c[1],
            })
            .collect();
        rows.push(segs);
    }
    rows
}

/// Groups row segments into sections (see the module comment, step 3).
pub(crate) fn sections(rows: Vec<Vec<Seg>>) -> Vec<Vec<Seg>> {
    let mut out: Vec<Vec<Seg>> = Vec::new();
    // For each segment of the previous row: its section.
    let mut prev: Vec<(Seg, usize)> = Vec::new();
    for row in rows {
        let overlaps = |a: &Seg, b: &Seg| a.u0 < b.u1 && b.u0 < a.u1;
        let mut next = Vec::with_capacity(row.len());
        for s in &row {
            let above: Vec<usize> = (0..prev.len()).filter(|&i| overlaps(&prev[i].0, s)).collect();
            let section = match above.as_slice() {
                [i] if row.iter().filter(|t| overlaps(&prev[*i].0, t)).count() == 1 => prev[*i].1,
                _ => {
                    out.push(Vec::new());
                    out.len() - 1
                }
            };
            out[section].push(*s);
            next.push((*s, section));
        }
        prev = next;
    }
    out
}

/// Needle points of one row from `ua` to `ub` (`u` values; both ends
/// included). `lines` are the split lines in row coordinates.
fn row_points(seg: &Seg, ua: f64, ub: f64, t: &Tatami, lines: &[Vec<Point>], out: &mut Vec<Point>) {
    let len = t.stitch_len_mm;
    let gap = END_GAP * len;
    let (lo, hi) = (ua.min(ub) + gap, ua.max(ub) - gap);
    let mut us: Vec<f64> = Vec::new();
    if lo < hi && !t.split.only_lines {
        match &t.split.pattern {
            Some((rows, w)) if !rows.is_empty() && *w > 0.0 => {
                let row = &rows[seg.k.rem_euclid(rows.len() as i64) as usize];
                let (n0, n1) = ((lo / w).floor() as i64 - 1, (hi / w).ceil() as i64);
                for n in n0..=n1 {
                    us.extend(row.iter().map(|f| (n as f64 + f) * w).filter(|u| *u >= lo && *u <= hi));
                }
            }
            _ => {
                let phase = seg.k.rem_euclid(i64::from(t.staggers.max(1))) as f64 / f64::from(t.staggers.max(1));
                let n0 = (lo / len - phase).ceil() as i64;
                let n1 = (hi / len - phase).floor() as i64;
                us.extend((n0..=n1).map(|n| (n as f64 + phase) * len));
            }
        }
    }
    // Where the row crosses the split lines.
    let (a, b) = (ua.min(ub), ua.max(ub));
    let mut cross: Vec<f64> = Vec::new();
    for l in lines {
        for w in l.windows(2) {
            let (p, q) = (w[0], w[1]);
            if (p.y - seg.v) * (q.y - seg.v) <= 0.0 && p.y != q.y {
                let u = p.x + (seg.v - p.y) / (q.y - p.y) * (q.x - p.x);
                if u > a + CROSS_GAP_MM && u < b - CROSS_GAP_MM {
                    cross.push(u);
                }
            }
        }
    }
    if !cross.is_empty() {
        us.retain(|u| cross.iter().all(|c| (u - c).abs() > CROSS_GAP_MM));
        us.extend(cross);
    }
    us.sort_by(f64::total_cmp);
    us.dedup_by(|x, y| (*x - *y).abs() < 1e-6);
    if t.split.only_lines {
        // No stitch longer than the stitch length.
        let mut all = vec![a];
        all.extend(us.iter().copied());
        all.push(b);
        us.clear();
        for w in all.windows(2) {
            let n = ((w[1] - w[0]) / len).ceil().max(1.0) as usize;
            us.extend((1..n).map(|k| w[0] + (w[1] - w[0]) * k as f64 / n as f64));
            if w[1] < b {
                us.push(w[1]);
            }
        }
    }
    out.push(Point::new(ua, seg.v));
    let grid = us.into_iter().map(|u| Point::new(u, seg.v));
    if ua < ub {
        out.extend(grid);
    } else {
        out.extend(grid.rev());
    }
    out.push(Point::new(ub, seg.v));
}

/// Where a section starts if it is sewn from `last_row_first` and `right_first`.
fn corner(sec: &[Seg], last_row_first: bool, right_first: bool, comp: f64) -> Point {
    let s = if last_row_first { sec[sec.len() - 1] } else { sec[0] };
    Point::new(if right_first { s.u1 + comp } else { s.u0 - comp }, s.v)
}

/// Sews a tatami layer over `shape`. The first section starts at the corner
/// nearest to the needle (or to `entry` if nothing is sewn yet). If `exit` is
/// set, the section that can end nearest to it is sewn last, ending there.
pub fn sew_tatami(shape: &Shape, t: &Tatami, entry: Point, exit: Option<Point>, sewer: &mut Sewer) {
    let t = Tatami {
        spacing_mm: t.spacing_mm.max(MIN_SPACING_MM),
        stitch_len_mm: t.stitch_len_mm.max(crate::running::MIN_SETTING_MM),
        ..t.clone()
    };
    let to_rows = Transform::rotate_deg(-t.angle_deg);
    let from_rows = Transform::rotate_deg(t.angle_deg);
    let rings: Vec<Vec<Point>> = shape
        .rings()
        .map(|r| r.iter().map(|p| to_rows.apply(*p)).collect())
        .collect();
    let lines: Vec<Vec<Point>> = t
        .split
        .lines
        .iter()
        .map(|l| l.iter().map(|p| to_rows.apply(*p)).collect())
        .collect();
    let mut rows = row_segments(&rings, t.spacing_mm);
    match &t.split.keep {
        None => {}
        Some(RowKeep::Bands(bands)) => {
            for row in &mut rows {
                if let Some(v) = row.first().map(|s| s.v)
                    && !bands.iter().any(|(a, b)| v >= *a && v < *b)
                {
                    // An empty row stops the sections.
                    row.clear();
                }
            }
        }
        Some(RowKeep::Gradient { v0, v1, stops, own }) => {
            rows.retain(|row| {
                row.first().is_some_and(|s| {
                    let tt = if v1 != v0 { (s.v - v0) / (v1 - v0) } else { 0.0 };
                    gradient_color(s.k, tt, stops) == *own
                })
            });
        }
    }
    if let Some(a) = &t.split.accordion {
        // Keep a row when it is the accordion spacing from the row kept before.
        let (v0, v1) = rows
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
                (lo.min(s.v), hi.max(s.v))
            });
        let mut last = f64::NEG_INFINITY;
        rows.retain(|row| {
            let Some(v) = row.first().map(|s| s.v) else {
                return false;
            };
            let tt = if v1 > v0 { (v - v0) / (v1 - v0) } else { 0.0 };
            let keep = v - last >= t.spacing_mm * a.factor(tt) - 1e-9;
            if keep {
                last = v;
            }
            keep
        });
    }
    let mut todo = sections(rows);
    let comp = t.pull_comp_mm.max(0.0);
    // The rows to sew last, and how, if an exit is set: the row end nearest
    // to the exit. If that row is inside a section, the section is split
    // there, so the last part can end on that row.
    let mut last = None;
    if let Some(x) = exit.map(|x| to_rows.apply(x)) {
        let mut best = (f64::INFINITY, 0, 0, false);
        for (i, sec) in todo.iter().enumerate() {
            for (r, seg) in sec.iter().enumerate() {
                for right in [false, true] {
                    let end = Point::new(if right { seg.u1 + comp } else { seg.u0 - comp }, seg.v);
                    let d = end.distance(x);
                    if d < best.0 {
                        best = (d, i, r, right);
                    }
                }
            }
        }
        if best.0.is_finite() {
            let (_, i, r, ends_right) = best;
            let mut sec = todo.remove(i);
            // The larger part is sewn last and ends on row `r`; the rest is sewn before.
            let mut chunk = if 2 * r >= sec.len() {
                let rest = sec.split_off(r + 1);
                std::mem::replace(&mut sec, rest)
            } else {
                let mut chunk = sec.split_off(r);
                chunk.reverse();
                chunk
            };
            if !sec.is_empty() {
                todo.push(sec);
            }
            // Rows alternate, so the start side follows from the end side and the row count.
            let right_first = !ends_right != chunk.len().is_multiple_of(2);
            chunk.shrink_to_fit();
            last = Some((chunk, false, right_first));
        }
    }
    let mut buf: Vec<Point> = Vec::new();
    while !todo.is_empty() || last.is_some() {
        let (mut sec, last_first, mut right) = if todo.is_empty() {
            last.take().expect("checked above")
        } else {
            let at = to_rows.apply(sewer.needle().unwrap_or(entry));
            // The nearest corner of all sections. Ties go to the first, so the result is deterministic.
            let mut best = (f64::INFINITY, 0, false, false);
            for (i, sec) in todo.iter().enumerate() {
                for (last_first, right_first) in [(false, false), (false, true), (true, false), (true, true)] {
                    let d = corner(sec, last_first, right_first, comp).distance(at);
                    if d < best.0 {
                        best = (d, i, last_first, right_first);
                    }
                }
            }
            let (_, i, last_first, right) = best;
            (todo.remove(i), last_first, right)
        };
        if last_first {
            sec.reverse();
        }
        for seg in &sec {
            let (mut l, mut r) = (seg.u0 - comp, seg.u1 + comp);
            if t.split.jagged_mm > 0.0 {
                let j = t.split.jagged_mm.min((r - l) * 0.3);
                let key = (seg.k as u64).wrapping_mul(2654435761) ^ seg.u0.to_bits();
                l += j * crate::effects::hash01(JAGGED_SEED, key);
                r -= j * crate::effects::hash01(JAGGED_SEED, key ^ 1);
            }
            let (ua, ub) = if right { (r, l) } else { (l, r) };
            buf.clear();
            row_points(seg, ua, ub, &t, &lines, &mut buf);
            // The step from the previous row goes along the edge. Where the
            // edge bends inward, the step goes around it as travel.
            if std::ptr::eq(seg, &sec[0]) {
                if matches!(t.split.keep, Some(RowKeep::Bands(_))) {
                    sewer.jump_to(from_rows.apply(buf[0]));
                } else {
                    sewer.go_to(from_rows.apply(buf[0]));
                }
            } else {
                sewer.step_to(from_rows.apply(buf[0]));
            }
            sewer.stitch(buf[1..].iter().map(|p| from_rows.apply(*p)));
            if t.split.double {
                sewer.stitch(buf.iter().rev().skip(1).map(|p| from_rows.apply(*p)));
            } else {
                right = !right;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Polyline};

    use super::*;
    use crate::travel::Travel;

    fn tatami(angle: f64) -> Tatami {
        Tatami {
            spacing_mm: 0.4,
            angle_deg: angle,
            stitch_len_mm: 3.5,
            pull_comp_mm: 0.0,
            staggers: 4,
            split: RowSplit::default(),
        }
    }

    fn sew(shape: &Shape, t: &Tatami) -> Vec<Vec<Point>> {
        let travel = Travel::new(shape);
        let mut s = Sewer::new(&travel, 3.0);
        sew_tatami(shape, t, Point::new(-100.0, -100.0), None, &mut s);
        s.finish()
    }

    fn square(s: f64) -> Shape {
        Shape::new(
            &Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(s, s))).flatten(0.05),
            &[],
        )
    }

    #[test]
    fn square_rows_cover_the_shape() {
        let runs = sew(&square(10.0), &tatami(0.0));
        assert_eq!(runs.len(), 1);
        let pts = &runs[0];
        // 25 rows at 0.2, 0.6, ... 9.8.
        let mut ys: Vec<i64> = pts.iter().map(|p| (p.y * 10.0).round() as i64).collect();
        ys.dedup();
        assert_eq!(ys.len(), 25);
        assert_eq!(ys[0], 2);
        // Starts at the corner nearest to the entry. Grid points near a row
        // end are not used, so a stitch at a row end can be 1.25 times longer.
        assert_eq!(pts[0], Point::new(0.0, 0.2));
        for w in pts.windows(2) {
            assert!(w[0].distance(w[1]) <= 3.5 * (1.0 + END_GAP) + 1e-9, "{w:?}");
        }
        assert!(pts.iter().all(|p| p.x >= -1e-9 && p.x <= 10.0 + 1e-9));
    }

    #[test]
    fn needle_points_are_staggered() {
        let runs = sew(&square(20.0), &tatami(0.0));
        let row = |y: f64| -> Vec<f64> {
            runs[0]
                .iter()
                .filter(|p| (p.y - y).abs() < 1e-9 && p.x > 0.5 && p.x < 19.5)
                .map(|p| p.x)
                .collect()
        };
        let (a, b) = (row(0.2), row(0.6));
        assert!(!a.is_empty() && !b.is_empty());
        // Consecutive rows are shifted by a quarter of the stitch length.
        let shift = (b.iter().copied().fold(f64::INFINITY, f64::min) - a.iter().copied().fold(f64::INFINITY, f64::min))
            .rem_euclid(3.5);
        assert!(
            (shift - 0.875).abs() < 1e-9 || (shift - 3.5 + 0.875).abs() < 1e-9,
            "{shift}"
        );
    }

    #[test]
    fn angle_turns_the_rows() {
        let runs = sew(&square(10.0), &tatami(90.0));
        // Vertical rows: the needle points of a row have one x value.
        let xs: std::collections::BTreeSet<i64> = runs[0].iter().map(|p| (p.x * 10.0).round() as i64).collect();
        assert_eq!(xs.len(), 25);
    }

    #[test]
    fn a_hole_makes_sections_with_travel_inside() {
        let outer = Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 30.0))).flatten(0.05);
        let hole = Path::ellipse(Point::new(15.0, 15.0), 6.0, 6.0).flatten(0.05);
        let shape = Shape::new(&outer, &hole);
        let runs = sew(&shape, &tatami(0.0));
        assert_eq!(runs.len(), 1, "one needle path, joined by travel");
        // No needle point in the hole, and no stitch crosses it. (A step
        // along the edge of the hole can cut a corner of the flattened circle.)
        let c = Point::new(15.0, 15.0);
        for w in runs[0].windows(2) {
            assert!(w[1].distance(c) >= 6.0 - 0.06, "{:?}", w[1]);
            assert!(w[0].midpoint(w[1]).distance(c) >= 6.0 - 0.06, "{w:?}");
        }
    }

    #[test]
    fn exit_sets_where_the_fill_ends() {
        let shape = square(10.0);
        let travel = Travel::new(&shape);
        for (angle, exit) in [
            (0.0, Point::new(10.0, 10.0)),
            (0.0, Point::new(0.0, 10.0)),
            (0.0, Point::new(10.0, 0.0)),
            (45.0, Point::new(9.5, 9.5)),
            (45.0, Point::new(0.5, 9.5)),
        ] {
            let mut s = Sewer::new(&travel, 3.0);
            sew_tatami(&shape, &tatami(angle), Point::new(-1.0, -1.0), Some(exit), &mut s);
            let runs = s.finish();
            let end = *runs[0].last().unwrap();
            assert!(end.distance(exit) < 0.8, "{end:?} for {exit:?} at {angle}°");
        }
    }

    #[test]
    fn pull_compensation_extends_rows() {
        let mut t = tatami(0.0);
        t.pull_comp_mm = 0.3;
        let runs = sew(&square(10.0), &t);
        let (lo, hi) = runs[0]
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.x), b.max(p.x)));
        assert!((lo + 0.3).abs() < 1e-9 && (hi - 10.3).abs() < 1e-9, "{lo} {hi}");
    }

    #[test]
    fn separate_parts_are_separate_runs() {
        let a = Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(5.0, 5.0))).flatten(0.05);
        let b = Path::rect(emb_geom::BBox::new(Point::new(20.0, 0.0), Point::new(25.0, 5.0))).flatten(0.05);
        let outers: Vec<Polyline> = a.into_iter().chain(b).collect();
        let runs = sew(&Shape::new(&outers, &[]), &tatami(0.0));
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn deterministic() {
        let shape = Shape::new(&Path::ellipse(Point::new(3.0, 4.0), 25.0, 25.0).flatten(0.05), &[]);
        let t = tatami(37.0);
        assert_eq!(sew(&shape, &t), sew(&shape, &t));
    }

    /// Needle points that are not at a row end (x away from 0 and 20).
    fn inner(runs: &[Vec<Point>]) -> Vec<Point> {
        runs.iter()
            .flatten()
            .copied()
            .filter(|p| p.x > 0.2 && p.x < 19.8)
            .collect()
    }

    #[test]
    fn a_program_split_puts_needle_points_where_the_tile_says() {
        let mut t = tatami(0.0);
        // A tile 4 mm wide: row 0 at 0.25, row 1 at 0.75.
        t.split.pattern = Some((vec![vec![0.25], vec![0.75]], 4.0));
        let pts = inner(&sew(&square(20.0), &t));
        assert!(!pts.is_empty());
        for p in &pts {
            let k = (p.y / 0.4 - 0.5).round() as i64;
            let f = (p.x / 4.0).rem_euclid(1.0);
            let want = if k.rem_euclid(2) == 0 { 0.25 } else { 0.75 };
            assert!((f - want).abs() < 1e-6, "{p:?} row {k}");
        }
    }

    #[test]
    fn a_user_split_sews_to_the_lines_only() {
        let mut t = tatami(0.0);
        t.stitch_len_mm = 30.0;
        t.split.lines = vec![vec![Point::new(7.0, -1.0), Point::new(13.0, 21.0)]];
        t.split.only_lines = true;
        let pts = inner(&sew(&square(20.0), &t));
        // One needle point in each row, on the line x = 7 + 6 (y + 1) / 22.
        assert!(pts.len() > 40);
        for p in &pts {
            let x = 7.0 + 6.0 * (p.y + 1.0) / 22.0;
            assert!((p.x - x).abs() < 1e-6, "{p:?}");
        }
        // With a short stitch length, longer stitches get needle points too.
        t.stitch_len_mm = 3.0;
        let fine = inner(&sew(&square(20.0), &t));
        assert!(fine.len() > pts.len() * 4);
    }

    #[test]
    fn a_stamp_line_shows_as_needle_points_in_tatami() {
        let mut t = tatami(0.0);
        t.split.lines = vec![vec![Point::new(10.0, -1.0), Point::new(10.0, 21.0)]];
        let pts = inner(&sew(&square(20.0), &t));
        // Every row has a needle point on x = 10, and none within the gap of it.
        let on = pts.iter().filter(|p| (p.x - 10.0).abs() < 1e-9).count();
        let rows = (20.0 / 0.4) as usize;
        assert!(on >= rows - 1, "{on} of {rows}");
        assert!(
            pts.iter()
                .all(|p| (p.x - 10.0).abs() < 1e-9 || (p.x - 10.0).abs() > CROSS_GAP_MM)
        );
    }
}
