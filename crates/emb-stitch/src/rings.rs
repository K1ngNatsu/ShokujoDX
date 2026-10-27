//! Circular and guided fills (P11 Milestone 4, ADR 0047).
//!
//! Not a port. Ink/Stitch has fill types with these names; this follows
//! what its public documentation says they look like, in our own method:
//!
//! - Circular: one spiral round the center point, `spacing` from one turn
//!   to the next. The parts of the spiral inside the shape are sewn.
//! - Guided: rows that follow the guide lines. Between two guide lines,
//!   row `i` of `n` is the mix `i / n` of the two lines (point by point,
//!   at the same fraction of their lengths), with `n` so that no two rows
//!   are farther apart than the spacing. Past the outer guide lines the
//!   rows are copies of them, one spacing apart across the first guide.
//!   The rows go past the shape at both ends and are cut at its edge.
//!
//! Each row gets needle points at the stitch length along it, from a start
//! that moves by a quarter stitch from one row to the next (as tatami), so
//! the needle points do not make lines. The parts are sewn nearest first:
//! each part from its end nearest to the needle.

use emb_geom::math::{cos, sin};
use emb_geom::{BBox, Point};

use crate::curves::{clip, sew_line};
use crate::fill::{RowSplit, Tatami, sew_tatami};
use crate::object::FILL_STAGGERS;
use crate::travel::{Sewer, Shape};

/// Smallest spacing that the generator accepts, in mm.
const MIN_SPACING_MM: f64 = 0.2;
/// Most turns of a spiral and most rows of a guided fill. With more, the
/// spacing gets larger.
const MAX_ROWS: f64 = 4000.0;
/// Points of a spiral are at most this angle apart, so it stays round near
/// the center. (Not a whole part of a turn, so the needle points of one
/// turn do not line up with the turn before.)
const MAX_TURN_RAD: f64 = 0.5;
/// Guide lines are sampled this often, in mm.
const GUIDE_STEP_MM: f64 = 0.5;
/// Most points of a sampled guide line.
const MAX_GUIDE_POINTS: usize = 2000;
/// The nearest part is looked for among this many parts still to sew (in
/// row order), so a fill with very many parts stays fast.
const NEAREST_WINDOW: usize = 400;

/// Needle points along `line` at `len` apart from arc length `phase`, with
/// the first and the last point of the line.
fn phased(line: &[Point], len: f64, phase: f64) -> Vec<Point> {
    let mut out = Vec::new();
    let Some(&first) = line.first() else {
        return out;
    };
    out.push(first);
    let mut next = phase.rem_euclid(len);
    if next < 1e-9 {
        next += len;
    }
    let mut at = 0.0;
    for w in line.windows(2) {
        let d = w[0].distance(w[1]);
        while next <= at + d {
            let p = w[0].lerp(w[1], if d > 0.0 { (next - at) / d } else { 0.0 });
            if out.last().is_none_or(|q| q.distance(p) > 1e-9) {
                out.push(p);
            }
            next += len;
        }
        at += d;
    }
    let last = line[line.len() - 1];
    if out.last().is_none_or(|q| q.distance(last) > 1e-9) {
        out.push(last);
    }
    out
}

/// Sews the parts nearest first. A step to a part of at most `near` goes
/// straight (inside the shape); a longer one travels along the edge.
pub(crate) fn sew_nearest(mut parts: Vec<Vec<Point>>, entry: Point, near: f64, sewer: &mut Sewer) {
    parts.retain(|p| p.len() >= 2);
    let mut done = vec![false; parts.len()];
    let mut first_open = 0;
    let mut left = parts.len();
    while left > 0 {
        while done[first_open] {
            first_open += 1;
        }
        let at = sewer.needle().unwrap_or(entry);
        let mut best = (f64::INFINITY, first_open, false);
        let mut seen = 0;
        for i in first_open..parts.len() {
            if done[i] {
                continue;
            }
            for (rev, end) in [(false, parts[i][0]), (true, parts[i][parts[i].len() - 1])] {
                let d = end.distance(at);
                if d < best.0 {
                    best = (d, i, rev);
                }
            }
            seen += 1;
            if seen >= NEAREST_WINDOW {
                break;
            }
        }
        let (d, i, rev) = best;
        done[i] = true;
        left -= 1;
        let mut part = std::mem::take(&mut parts[i]);
        if rev {
            part.reverse();
        }
        if d > near && sewer.needle().is_some() {
            sewer.go_to(part[0]);
        }
        sew_line(sewer, &part);
    }
}

/// The box of the shape.
fn bbox(shape: &Shape) -> Option<BBox> {
    BBox::from_points(shape.rings().flatten().copied())
}

/// Sews a circular fill.
pub fn circular(shape: &Shape, center: Option<Point>, spacing: f64, len: f64, entry: Point, sewer: &mut Sewer) {
    let Some(b) = bbox(shape) else { return };
    let c = center.unwrap_or(b.center());
    let reach = shape.rings().flatten().map(|p| p.distance(c)).fold(0.0, f64::max);
    if reach <= 0.0 {
        return;
    }
    let spacing = spacing.max(MIN_SPACING_MM).max(reach / MAX_ROWS);
    // r = spacing · (θ / 2π + 0.5): the first turn starts half a spacing out.
    let k = spacing / std::f64::consts::TAU;
    let mut spiral = Vec::new();
    let mut th = 0.0;
    loop {
        let r = k * th + spacing / 2.0;
        spiral.push(c + Point::new(cos(th), sin(th)) * r);
        if r > reach + spacing {
            break;
        }
        th += (len / r).min(MAX_TURN_RAD);
    }
    let parts = clip(&spiral, shape);
    sew_nearest(parts, entry, 2.0 * spacing + len, sewer);
}

/// A guide line as points at the same fractions of its length.
fn sampled(guide: &[Point], n: usize) -> Vec<Point> {
    let mut at = vec![0.0];
    for w in guide.windows(2) {
        at.push(at[at.len() - 1] + w[0].distance(w[1]));
    }
    let total = at[at.len() - 1];
    (0..n)
        .map(|i| {
            let s = total * i as f64 / (n - 1) as f64;
            let j = at.partition_point(|&a| a <= s).clamp(1, guide.len() - 1);
            let d = at[j] - at[j - 1];
            guide[j - 1].lerp(guide[j], if d > 0.0 { (s - at[j - 1]) / d } else { 0.0 })
        })
        .collect()
}

/// The row with its ends made longer by `ext` along their directions.
fn extended(row: &[Point], ext: f64) -> Vec<Point> {
    let n = row.len();
    let mut out = Vec::with_capacity(n + 2);
    if let Some(d) = (row[0] - row[1]).normalized() {
        out.push(row[0] + d * ext);
    }
    out.extend_from_slice(row);
    if let Some(d) = (row[n - 1] - row[n - 2]).normalized() {
        out.push(row[n - 1] + d * ext);
    }
    out
}

/// Sews a guided fill. With no usable guide, the rows are straight tatami.
#[allow(clippy::too_many_arguments)]
pub fn guided(
    shape: &Shape,
    guides: &[Vec<Point>],
    angle_deg: f64,
    spacing: f64,
    len: f64,
    entry: Point,
    sewer: &mut Sewer,
) {
    let spacing = spacing.max(MIN_SPACING_MM);
    let usable: Vec<&Vec<Point>> = guides
        .iter()
        .filter(|g| g.len() >= 2 && g[0].distance(g[g.len() - 1]) > 1e-6)
        .collect();
    let (Some(b), Some(first)) = (bbox(shape), usable.first()) else {
        let t = Tatami {
            spacing_mm: spacing,
            angle_deg,
            stitch_len_mm: len,
            pull_comp_mm: 0.0,
            staggers: FILL_STAGGERS,
            split: RowSplit::default(),
        };
        sew_tatami(shape, &t, entry, None, sewer);
        return;
    };
    let along = (first[first.len() - 1] - first[0])
        .normalized()
        .unwrap_or(Point::new(1.0, 0.0));
    let side = Point::new(-along.y, along.x);
    let longest = usable
        .iter()
        .map(|g| g.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>())
        .fold(0.0, f64::max);
    let n = ((longest / GUIDE_STEP_MM).ceil() as usize).clamp(8, MAX_GUIDE_POINTS);
    // The guides in the same direction as the first, in order across it.
    let mut lines: Vec<Vec<Point>> = usable
        .iter()
        .map(|g| {
            let mut s = sampled(g, n);
            if (s[n - 1] - s[0]).dot(along) < 0.0 {
                s.reverse();
            }
            s
        })
        .collect();
    let across = |l: &[Point]| l.iter().map(|p| p.dot(side)).sum::<f64>() / l.len() as f64;
    lines.sort_by(|a, b| across(a).total_cmp(&across(b)));
    // How far the shape goes across, past the first and the last guide.
    let corners = [b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)];
    let reach = |l: &[Point], dir: f64| {
        let base = l.iter().map(|p| p.dot(side) * dir).fold(f64::INFINITY, f64::min);
        corners.iter().map(|c| c.dot(side) * dir - base).fold(0.0, f64::max)
    };
    let before = reach(&lines[0], -1.0);
    let after = reach(&lines[lines.len() - 1], 1.0);
    let mut between = 0.0;
    for w in lines.windows(2) {
        let d = w[0].iter().zip(&w[1]).map(|(p, q)| p.distance(*q)).fold(0.0, f64::max);
        between += (d / spacing).ceil();
    }
    let total = before / spacing + after / spacing + between;
    let spacing = if total > MAX_ROWS {
        spacing * total / MAX_ROWS
    } else {
        spacing
    };
    let shift = |l: &[Point], d: f64| l.iter().map(|p| *p + side * d).collect::<Vec<Point>>();
    let mut rows: Vec<Vec<Point>> = Vec::new();
    let n_before = (before / spacing).ceil() as usize;
    for i in (1..=n_before).rev() {
        rows.push(shift(&lines[0], -(i as f64) * spacing));
    }
    for w in lines.windows(2) {
        let d = w[0].iter().zip(&w[1]).map(|(p, q)| p.distance(*q)).fold(0.0, f64::max);
        let m = ((d / spacing).ceil() as usize).max(1);
        for i in 0..m {
            let f = i as f64 / m as f64;
            rows.push(w[0].iter().zip(&w[1]).map(|(p, q)| p.lerp(*q, f)).collect());
        }
    }
    let last = &lines[lines.len() - 1];
    let n_after = (after / spacing).ceil() as usize;
    for i in 0..=n_after {
        rows.push(shift(last, i as f64 * spacing));
    }
    let ext = Point::new(b.width(), b.height()).length() + len;
    let mut parts = Vec::new();
    for (k, row) in rows.iter().enumerate() {
        let phase = (k % FILL_STAGGERS as usize) as f64 / f64::from(FILL_STAGGERS) * len;
        let line = phased(&extended(row, ext), len, phase);
        parts.extend(clip(&line, shape));
    }
    sew_nearest(parts, entry, 2.0 * spacing + len, sewer);
}

#[cfg(test)]
mod tests {
    use emb_geom::Polyline;

    use super::*;
    use crate::travel::Travel;

    fn square(size: f64) -> Shape {
        Shape::new(
            &[Polyline {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(size, 0.0),
                    Point::new(size, size),
                    Point::new(0.0, size),
                ],
                closed: true,
            }],
            &[],
        )
    }

    fn sew(shape: &Shape, f: impl FnOnce(&mut Sewer)) -> Vec<Vec<Point>> {
        let travel = Travel::new(shape);
        let mut s = Sewer::new(&travel, 3.0);
        f(&mut s);
        s.finish()
    }

    fn inside(shape: &Shape, runs: &[Vec<Point>]) -> bool {
        let grown = shape.inset(-0.05);
        runs.iter()
            .flatten()
            .all(|q| grown.regions.iter().any(|r| r.contains(*q)))
    }

    #[test]
    fn phased_points_start_at_the_phase() {
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let xs: Vec<f64> = phased(&line, 3.0, 1.0).iter().map(|p| p.x).collect();
        assert_eq!(xs, [0.0, 1.0, 4.0, 7.0, 10.0]);
        let xs: Vec<f64> = phased(&line, 3.0, 0.0).iter().map(|p| p.x).collect();
        assert_eq!(xs, [0.0, 3.0, 6.0, 9.0, 10.0]);
    }

    #[test]
    fn a_circular_fill_is_rings_round_the_center() {
        let shape = square(20.0);
        let c = Point::new(10.0, 10.0);
        let runs = sew(&shape, |s| circular(&shape, Some(c), 0.5, 2.5, Point::ZERO, s));
        assert!(inside(&shape, &runs));
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        // About 400 mm² / 0.5 mm of line at 2.5 mm a stitch.
        assert!(pts.len() > 300, "{}", pts.len());
        // Needle points near the center and in the corners.
        assert!(pts.iter().any(|p| p.distance(c) < 0.6));
        assert!(pts.iter().any(|p| p.distance(Point::new(0.5, 0.5)) < 1.0));
        // Most stitches go round the center: square to the way out.
        let round = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| {
                let d = w[1] - w[0];
                let out = (w[0].midpoint(w[1]) - c).normalized();
                d.length() > 0.5 && out.is_some_and(|o| (d.dot(o) / d.length()).abs() < 0.3)
            })
            .count();
        let all = runs.iter().map(|r| r.len()).sum::<usize>();
        assert!(round * 10 > all * 7, "{round} of {all}");
    }

    #[test]
    fn guided_rows_follow_the_guides() {
        let shape = square(30.0);
        // Two guides: a straight line at the top and an arch below it.
        let top = vec![Point::new(0.0, 5.0), Point::new(30.0, 5.0)];
        let arch = vec![Point::new(0.0, 25.0), Point::new(15.0, 15.0), Point::new(30.0, 25.0)];
        let runs = sew(&shape, |s| {
            guided(&shape, &[top.clone(), arch.clone()], 0.0, 0.5, 3.0, Point::ZERO, s)
        });
        assert!(inside(&shape, &runs));
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        assert!(pts.len() > 500);
        // Near the top guide the stitches are level; near the arch they slope.
        let slope = |y0: f64, y1: f64, x0: f64, x1: f64| {
            let w: Vec<f64> = runs
                .iter()
                .flat_map(|r| r.windows(2))
                .filter(|w| {
                    let m = w[0].midpoint(w[1]);
                    m.y > y0 && m.y < y1 && m.x > x0 && m.x < x1 && w[0].distance(w[1]) > 1.0
                })
                .map(|w| ((w[1].y - w[0].y) / (w[1].x - w[0].x)).abs())
                .collect();
            w.iter().sum::<f64>() / w.len().max(1) as f64
        };
        assert!(slope(4.0, 6.0, 3.0, 12.0) < 0.05);
        assert!(slope(21.0, 25.0, 3.0, 12.0) > 0.4);
        // Below the arch the rows are copies of it.
        assert!(slope(26.0, 29.0, 3.0, 10.0) > 0.4);
    }

    #[test]
    fn guided_with_no_guide_is_straight_rows() {
        let shape = square(10.0);
        let runs = sew(&shape, |s| guided(&shape, &[], 0.0, 0.4, 3.0, Point::ZERO, s));
        let mut ys: Vec<i64> = runs.iter().flatten().map(|p| (p.y * 10.0).round() as i64).collect();
        ys.sort_unstable();
        ys.dedup();
        assert_eq!(ys.len(), 25);
    }

    #[test]
    fn nearest_parts_go_back_and_forth() {
        let shape = square(10.0);
        let parts: Vec<Vec<Point>> = (0..5)
            .map(|i| {
                let y = 1.0 + i as f64;
                vec![Point::new(1.0, y), Point::new(9.0, y)]
            })
            .collect();
        let runs = sew(&shape, |s| sew_nearest(parts, Point::ZERO, 2.0, s));
        assert_eq!(runs.len(), 1);
        let ends: Vec<f64> = runs[0].iter().map(|p| p.x).collect();
        assert_eq!(ends, [1.0, 9.0, 9.0, 1.0, 1.0, 9.0, 9.0, 1.0, 1.0, 9.0]);
    }
}
