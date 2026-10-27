//! Column stitch types: the top stitching of a satin column (or of a line
//! with a border) for each [`ColumnPattern`].
//!
//! Not a port. Each type follows what public embroidery documentation
//! says it looks like, and common digitizing practice:
//!
//! - Satin and zigzag: needle points on the two rails in turn. Zigzag has
//!   no short stitches and no splits, so its open stitches stay straight.
//! - Raised satin: lower satin layers, each a little narrower than the
//!   layer above, then the top layer. The layers go forward and back, so
//!   the top layer starts at the start.
//! - Tatami: satin rows split into short stitches; the split points move
//!   from row to row.
//! - E stitch (blanket stitch): a run along rail A; at each step a stitch
//!   goes across to rail B and back.
//! - Square: a square wave: across, along rail B, back, along rail A.
//! - Coil: loops that go across the column and back, each a little
//!   farther on, like a coiled spring seen from above.
//! - String: slanted stitches from rail A to rail B a chord length
//!   farther on, which overlap like twisted string.
//! - Motif column: the motif repeated along the column, as high as the
//!   column is wide (see `motifs`).
//! - Program split (flexi split): satin rows with needle points where the
//!   program-split pattern puts them, measured from rail A, so the pattern
//!   bends with the column.
//! - Contour: running lines along the column at even fractions of its
//!   width, sewn forward and back in turn. The spacing sets how many lines
//!   (an odd number, so the column ends at its end); where the column is
//!   narrow, the lines come closer.
//!
//! Pull compensation makes the column wider on both sides for all types.
//! Only `sqrt`, `sin` and `cos` from `emb_geom::math` are used (ADR 0003).

use emb_geom::Point;
use emb_geom::math::{cos, sin};
use emb_model::{ColumnPattern, SatinParams};

use crate::satin::{Column, inset_pairs, satin_top};

/// Smallest spacing that the generator accepts, in mm.
const MIN_SPACING_MM: f64 = 0.1;
/// Each lower layer of raised satin is this much narrower on each side.
const RAISED_INSET_MM: f64 = 0.2;
/// Most layers of raised satin.
const MAX_RAISED_LAYERS: u32 = 5;
/// Longest stitch along a rail (E stitch, square) and of a contour line, in mm.
const EDGE_LEN_MM: f64 = 2.5;
/// Coil: needle points on a loop are about this far apart.
const COIL_STITCH_MM: f64 = 1.5;
/// Coil: a loop reaches this fraction of the column width forward and back.
const COIL_REACH: f64 = 0.35;
/// Most lines of a contour column.
const MAX_CONTOUR_LINES: u32 = 61;
/// Step of the point pairs that coil and contour interpolate.
const FINE_STEP_MM: f64 = 0.2;

/// The top stitching of the column, from its start to its end.
pub fn top(col: &Column, p: &SatinParams) -> Vec<Point> {
    let spacing = p.spacing_mm.max(MIN_SPACING_MM);
    let half = |p: &SatinParams| crate::effects::spaced_pairs(col, p, spacing / 2.0);
    match p.pattern {
        ColumnPattern::Satin => satin_top(&half(p), p),
        ColumnPattern::Zigzag => {
            let open = SatinParams {
                short_stitches: false,
                split_mm: f64::INFINITY,
                ..p.clone()
            };
            satin_top(&half(&open), &open)
        }
        ColumnPattern::Tatami { stitch_mm } => {
            let rows = SatinParams {
                short_stitches: false,
                split_mm: stitch_mm,
                ..p.clone()
            };
            satin_top(&half(&rows), &rows)
        }
        ColumnPattern::Raised { layers } => raised(col, p, layers),
        ColumnPattern::EStitch => e_stitch(&widen(&col.pairs(spacing), p)),
        ColumnPattern::Square => square(&widen(&col.pairs(spacing), p), p.split_mm),
        ColumnPattern::Coil => coil(&widen(&col.pairs(FINE_STEP_MM), p), spacing),
        ColumnPattern::String { chord_mm } => string(&widen(&col.pairs(spacing), p), chord_mm / spacing),
        ColumnPattern::Motif { aspect } => {
            let default = emb_model::Motif::default();
            let m = p.motif.as_ref().unwrap_or(&default);
            crate::motifs::column(&widen(&col.pairs(FINE_STEP_MM), p), m, aspect)
        }
        ColumnPattern::ProgramSplit { scale } => {
            let default = emb_model::SplitPattern::default();
            let pat = p.split_pattern.as_ref().unwrap_or(&default);
            split_column(&widen(&half(p), p), pat, scale)
        }
        ColumnPattern::Contour { stitch_mm } => contour(&widen(&col.pairs(FINE_STEP_MM), p), spacing, stitch_mm),
    }
}

/// Moves the points of each pair apart by the pull compensation of `p`:
/// rail A and rail B each by their own amount, plus the percent of the width.
fn widen(pairs: &[(Point, Point)], p: &SatinParams) -> Vec<(Point, Point)> {
    let (ca, cb, pct) = (
        p.pull_compensation_mm,
        p.pull_b_mm.unwrap_or(p.pull_compensation_mm),
        p.pull_percent / 100.0,
    );
    if ca == 0.0 && cb == 0.0 && pct == 0.0 {
        return pairs.to_vec();
    }
    pairs
        .iter()
        .map(|&(a, b)| match (b - a).normalized() {
            Some(u) => {
                let w = a.distance(b);
                (a - u * (ca + w * pct), b + u * (cb + w * pct))
            }
            None => (a, b),
        })
        .collect()
}

/// Points from `a` to `b` (without `a`), no more than `max` apart.
fn straight(a: Point, b: Point, max: f64, out: &mut Vec<Point>) {
    let n = ((a.distance(b) / max) - 1e-9).ceil().max(1.0) as usize;
    out.extend((1..=n).map(|k| a.lerp(b, k as f64 / n as f64)));
}

fn raised(col: &Column, p: &SatinParams, layers: u32) -> Vec<Point> {
    let spacing = p.spacing_mm.max(MIN_SPACING_MM);
    let lower = layers.clamp(1, MAX_RAISED_LAYERS) - 1;
    let pairs = crate::effects::spaced_pairs(col, p, spacing / 2.0);
    let under = SatinParams {
        short_stitches: false,
        ..p.clone()
    };
    let mut out = Vec::new();
    for k in 0..lower {
        // The lowest layer is the narrowest.
        let mut layer = inset_pairs(&pairs, RAISED_INSET_MM * f64::from(lower - k));
        if k % 2 == 1 {
            layer.reverse();
        }
        out.extend(satin_top(&layer, &under));
    }
    if lower % 2 == 1 {
        // An odd number of lower layers ends at the far end: walk back in the middle.
        let mids: Vec<Point> = pairs.iter().rev().map(|(a, b)| a.midpoint(*b)).collect();
        out.extend(crate::satin::walk(mids));
    }
    out.extend(satin_top(&pairs, p));
    out
}

fn e_stitch(pairs: &[(Point, Point)]) -> Vec<Point> {
    let mut out = vec![pairs[0].0];
    for (i, &(a, b)) in pairs.iter().enumerate() {
        if i > 0 {
            straight(pairs[i - 1].0, a, EDGE_LEN_MM, &mut out);
        }
        out.push(b);
        out.push(a);
    }
    out
}

fn square(pairs: &[(Point, Point)], split_mm: f64) -> Vec<Point> {
    let split = split_mm.max(crate::satin::MIN_SPLIT_MM);
    let mut out = vec![pairs[0].0];
    for (i, &(a, b)) in pairs.iter().enumerate() {
        let (near, far) = if i % 2 == 0 { (a, b) } else { (b, a) };
        if i > 0 {
            let last = *out.last().expect("not empty");
            straight(last, near, EDGE_LEN_MM, &mut out);
        }
        straight(near, far, split, &mut out);
    }
    out
}

/// A program split across the column (flexi split): satin rows from rail
/// to rail; stitch `j` gets needle points where row `j` of the pattern tile
/// says, measured from rail A, so the pattern bends with the column.
fn split_column(pairs: &[(Point, Point)], pat: &emb_model::SplitPattern, scale: f64) -> Vec<Point> {
    let w = (pat.width_mm * scale).max(0.5);
    let mut out = vec![pairs[0].0];
    for (j, &(a, b)) in pairs.iter().enumerate().skip(1) {
        let from = out[out.len() - 1];
        let to = if j % 2 == 1 { b } else { a };
        // The ends of this stitch on rail A and on rail B.
        let (ea, eb) = if j % 2 == 1 { (from, to) } else { (to, from) };
        let len = ea.distance(eb);
        let row = &pat.rows[j % pat.rows.len().max(1)];
        let mut ds: Vec<f64> = (0..=(len / w).ceil() as i64)
            .flat_map(|n| row.iter().map(move |f| (n as f64 + f) * w))
            .filter(|d| *d > 0.3 && *d < len - 0.3)
            .collect();
        if j % 2 == 0 {
            ds.reverse();
        }
        out.extend(ds.into_iter().map(|d| ea.lerp(eb, d / len)));
        out.push(to);
    }
    out
}

/// String: from each point on rail A to the point on rail B `ahead` pairs
/// farther on (the last pair at the end of the column).
fn string(pairs: &[(Point, Point)], ahead: f64) -> Vec<Point> {
    let k = (ahead.round() as usize).max(1);
    let last = pairs.len() - 1;
    let mut out = Vec::with_capacity(2 * pairs.len());
    for (i, &(a, _)) in pairs.iter().enumerate() {
        out.push(a);
        out.push(pairs[(i + k).min(last)].1);
    }
    out.push(pairs[last].0);
    out
}

/// The point at length `s` along the middle and fraction `f` across, from
/// `pairs` with the middle lengths `at`.
fn at(pairs: &[(Point, Point)], at: &[f64], s: f64, f: f64) -> Point {
    let s = s.clamp(0.0, at[at.len() - 1]);
    let i = at.partition_point(|&l| l <= s).clamp(1, pairs.len() - 1);
    let seg = at[i] - at[i - 1];
    let t = if seg > 0.0 { (s - at[i - 1]) / seg } else { 0.0 };
    let a = pairs[i - 1].0.lerp(pairs[i].0, t);
    let b = pairs[i - 1].1.lerp(pairs[i].1, t);
    a.lerp(b, f)
}

/// Lengths along the middle of the column at each pair.
fn middle_lengths(pairs: &[(Point, Point)]) -> Vec<f64> {
    let mut at = vec![0.0];
    for w in pairs.windows(2) {
        let d = w[0].0.midpoint(w[0].1).distance(w[1].0.midpoint(w[1].1));
        at.push(at[at.len() - 1] + d);
    }
    at
}

fn coil(pairs: &[(Point, Point)], spacing: f64) -> Vec<Point> {
    let lens = middle_lengths(pairs);
    let total = lens[lens.len() - 1];
    let width = pairs.iter().map(|(a, b)| a.distance(*b)).sum::<f64>() / pairs.len() as f64;
    if total <= 0.0 {
        return vec![pairs[0].0, pairs[0].1];
    }
    let turns = (total / spacing).round().max(1.0);
    let advance = total / turns;
    let reach = COIL_REACH * width;
    // The length of one loop, about: an ellipse with half axes width / 2 and reach.
    let loop_len = std::f64::consts::TAU * ((width * width / 4.0 + reach * reach) / 2.0).sqrt();
    let per_turn = (loop_len / COIL_STITCH_MM).ceil().max(6.0);
    let n = (turns * per_turn) as usize;
    (0..=n)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / per_turn;
            let s = advance * t / std::f64::consts::TAU - reach * sin(t);
            at(pairs, &lens, s, (1.0 - cos(t)) / 2.0)
        })
        .collect()
}

fn contour(pairs: &[(Point, Point)], spacing: f64, stitch_mm: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for (i, mut line) in contour_lines(pairs, spacing, stitch_mm).into_iter().enumerate() {
        if i % 2 == 1 {
            line.reverse();
        }
        out.extend(line);
    }
    out
}

/// The lines of a contour column, each from the start of the column to its end.
pub(crate) fn contour_lines(pairs: &[(Point, Point)], spacing: f64, stitch_mm: f64) -> Vec<Vec<Point>> {
    let width = pairs.iter().map(|(a, b)| a.distance(*b)).sum::<f64>() / pairs.len() as f64;
    // An odd number of lines, so the column ends at its end.
    let n = ((width / spacing).round() as u32 + 1).clamp(2, MAX_CONTOUR_LINES) | 1;
    let lens = middle_lengths(pairs);
    let total = lens[lens.len() - 1];
    let steps = ((total / stitch_mm.clamp(0.5, EDGE_LEN_MM * 4.0)) - 1e-9)
        .ceil()
        .max(1.0) as usize;
    (0..n)
        .map(|i| {
            let f = f64::from(i) / f64::from(n - 1);
            (0..=steps)
                .map(|k| at(pairs, &lens, total * k as f64 / steps as f64, f))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{SatinGeometry, Underlay};

    use super::*;
    use crate::satin::satin;

    fn straight_col(w: f64, l: f64) -> SatinGeometry {
        SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(l, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, w)).line_to(Point::new(l, w)),
            rungs: Vec::new(),
        }
    }

    fn sew(pattern: ColumnPattern) -> Vec<Point> {
        let p = SatinParams {
            underlay: Underlay::None,
            pull_compensation_mm: 0.0,
            ..SatinParams::with_pattern(pattern)
        };
        satin(&straight_col(4.0, 20.0), &p, Point::new(-5.0, 0.0), None).unwrap()
    }

    fn in_column(pts: &[Point]) -> bool {
        pts.iter()
            .all(|q| q.x >= -1e-6 && q.x <= 20.0 + 1e-6 && q.y >= -1e-6 && q.y <= 4.0 + 1e-6)
    }

    #[test]
    fn every_type_stays_in_the_column_and_ends_at_the_far_end() {
        for pattern in ColumnPattern::ALL {
            let pts = sew(pattern);
            assert!(pts.len() > 10, "{pattern:?}");
            assert!(in_column(&pts), "{pattern:?}");
            assert!(pts[0].x.abs() < 1e-6, "{pattern:?} starts at the start");
            assert!(
                (pts[pts.len() - 1].x - 20.0).abs() < 1e-6,
                "{pattern:?} ends at the end"
            );
        }
    }

    #[test]
    fn zigzag_is_open_satin_with_no_splits() {
        let pts = sew(ColumnPattern::Zigzag);
        // 20 mm at 0.75 mm a needle point: 27 points, all on a rail.
        assert_eq!(pts.len(), 28);
        assert!(pts.iter().all(|q| q.y == 0.0 || q.y == 4.0));
    }

    #[test]
    fn raised_satin_has_lower_layers_inside_the_top() {
        let one = sew(ColumnPattern::Raised { layers: 1 });
        let three = sew(ColumnPattern::Raised { layers: 3 });
        assert!(three.len() > 2 * one.len());
        // The lower layers are narrower: their needle points are inside the rails.
        let lower = &three[..three.len() - one.len()];
        assert!(lower.iter().all(|q| q.y > 0.1 && q.y < 3.9));
        assert_eq!(&three[three.len() - one.len()..], &one[..]);
        // Two layers: one lower layer forward, a walk back, then the top.
        let two = sew(ColumnPattern::Raised { layers: 2 });
        assert!(two.len() > one.len() * 2);
    }

    #[test]
    fn e_stitch_has_a_spine_on_rail_a_and_prongs_to_rail_b() {
        let pts = sew(ColumnPattern::EStitch);
        let prongs = pts.iter().filter(|q| q.y == 4.0).count();
        // 20 mm at 2 mm: 11 prongs.
        assert_eq!(prongs, 11);
        assert!(
            pts.windows(2).all(|w| w[0].y == 0.0 || w[1].y == 0.0),
            "each prong comes back"
        );
    }

    #[test]
    fn square_makes_teeth() {
        let pts = sew(ColumnPattern::Square);
        let along_b = pts.windows(2).filter(|w| w[0].y == 4.0 && w[1].y == 4.0).count();
        let along_a = pts.windows(2).filter(|w| w[0].y == 0.0 && w[1].y == 0.0).count();
        assert!(along_b > 4 && along_a > 4, "{along_a} {along_b}");
    }

    #[test]
    fn coil_goes_back_and_forth() {
        let pts = sew(ColumnPattern::Coil);
        // Loops: the needle moves backward along the column at times.
        let back = pts.windows(2).filter(|w| w[1].x < w[0].x - 0.05).count();
        assert!(back > 10, "{back}");
        assert!(pts.iter().any(|q| (q.y - 4.0).abs() < 0.05));
    }

    #[test]
    fn contour_lines_follow_the_column() {
        let p = SatinParams {
            spacing_mm: 2.0,
            underlay: Underlay::None,
            pull_compensation_mm: 0.0,
            ..SatinParams::with_pattern(ColumnPattern::Contour { stitch_mm: 2.0 })
        };
        let pts = satin(&straight_col(4.0, 20.0), &p, Point::new(-5.0, 0.0), None).unwrap();
        let ys: std::collections::BTreeSet<i64> = pts.iter().map(|q| (q.y * 100.0).round() as i64).collect();
        assert_eq!(ys.into_iter().collect::<Vec<_>>(), vec![0, 200, 400]);
        // 3 lines of 10 stitches.
        assert_eq!(pts.len(), 33);
        // The default spacing fills the column with more lines.
        assert!(sew(ColumnPattern::Contour { stitch_mm: 2.0 }).len() > 60);
    }
}
