//! Tartan and linear gradient fills (P11 Milestone 4, ADR 0047): tatami
//! rows of one color of several.
//!
//! Not a port. Ink/Stitch has tartan and linear gradient fills; this
//! follows what its public documentation says they look like, in our own
//! method. An object has one thread, so a tartan or a gradient of several
//! colors is a group of fills, and each fill sews the rows of its color
//! (`fill::RowKeep`):
//!
//! - Tartan: the stripes of the sett, centered on the middle of the shape,
//!   go across the rows at the fill angle (the warp) and across the rows
//!   square to them (the weft). A fill sews the rows that are in the
//!   stripes of its color, first the warp, then the weft.
//! - Gradient: each row of one tatami layer has one color, from the place
//!   of the row between the start and the end of the gradient
//!   (`fill::gradient_color`), so the rows of all colors together have the
//!   set density.

use emb_geom::{BBox, Point, Transform};
use emb_model::{FillParams, Sett};

use crate::fill::{RowKeep, RowSplit, Tatami, sew_tatami};
use crate::object::FILL_STAGGERS;
use crate::travel::{Sewer, Shape};

/// Most stripes across a shape. With more (a very small count), there is
/// no tartan.
const MAX_BANDS: usize = 100_000;

/// The middle of the box of the shape.
fn middle(shape: &Shape) -> Option<Point> {
    BBox::from_points(shape.rings().flatten().copied()).map(|b| b.center())
}

/// The lowest and highest `v` (across the rows at `angle_deg`) of the shape.
fn span(shape: &Shape, to_rows: &Transform) -> (f64, f64) {
    shape
        .rings()
        .flatten()
        .map(|p| to_rows.apply(*p).y)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)))
}

/// The bands of color `color` across `lo`–`hi`, with the middle of the
/// first stripe of the sett at `mid`.
pub(crate) fn bands(sett: &Sett, color: u32, mid: f64, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let repeat = sett.repeat();
    let width: f64 = repeat.iter().map(|(_, w)| w).sum();
    if repeat.is_empty() || width.is_nan() || width <= 0.0 || !lo.is_finite() || !hi.is_finite() {
        return Vec::new();
    }
    if (hi - lo) / width * repeat.len() as f64 > MAX_BANDS as f64 {
        return Vec::new();
    }
    let start = mid - repeat[0].1 / 2.0;
    let (n0, n1) = (
        ((lo - start) / width).floor() as i64 - 1,
        ((hi - start) / width).ceil() as i64,
    );
    let mut out: Vec<(f64, f64)> = Vec::new();
    for n in n0..=n1 {
        let mut at = start + n as f64 * width;
        for (c, w) in &repeat {
            if *c == color && at + w > lo && at < hi {
                match out.last_mut() {
                    // Two stripes of the same color next to each other are one band.
                    Some((_, b)) if (*b - at).abs() < 1e-9 => *b = at + w,
                    _ => out.push((at, at + w)),
                }
            }
            at += w;
        }
    }
    out
}

/// Sews the stripes of color `color` of a tartan.
pub fn tartan(shape: &Shape, p: &FillParams, sett: &Sett, color: u32, entry: Point, sewer: &mut Sewer) {
    let Some(mid) = middle(shape) else { return };
    for angle in [p.angle_deg, p.angle_deg + 90.0] {
        let to_rows = Transform::rotate_deg(-angle);
        let (lo, hi) = span(shape, &to_rows);
        let b = bands(sett, color, to_rows.apply(mid).y, lo, hi);
        if b.is_empty() {
            continue;
        }
        let t = Tatami {
            spacing_mm: p.spacing_mm,
            angle_deg: angle,
            stitch_len_mm: p.stitch_length_mm,
            pull_comp_mm: p.pull_compensation_mm,
            staggers: FILL_STAGGERS,
            split: RowSplit {
                keep: Some(RowKeep::Bands(b)),
                jagged_mm: p.jagged_mm,
                ..RowSplit::default()
            },
        };
        let at = sewer.needle().unwrap_or(entry);
        sew_tatami(shape, &t, at, None, sewer);
    }
}

/// The rows of a gradient fill that its color sews.
pub(crate) fn gradient_keep(
    shape: &Shape,
    angle_deg: f64,
    line: Option<(Point, Point)>,
    stops: &[f64],
    color: u32,
) -> RowKeep {
    let to_rows = Transform::rotate_deg(-angle_deg);
    let (v0, v1) = match line {
        Some((a, b)) => (to_rows.apply(a).y, to_rows.apply(b).y),
        None => span(shape, &to_rows),
    };
    RowKeep::Gradient {
        v0,
        v1,
        stops: stops.to_vec(),
        own: color as usize,
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Polyline;
    use emb_model::FillPattern;

    use super::*;
    use crate::fill::gradient_color;
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

    #[test]
    fn bands_repeat_the_sett_round_the_middle() {
        // K4 W4 (0.5 mm a count): 2 mm black, 2 mm white.
        let sett = Sett::parse("K4 W4", 0.5, false).unwrap();
        let k = bands(&sett, 0, 10.0, 0.0, 20.0);
        // Black at 9–11, 13–15, ...; and 5–7, 1–3; and the parts at the ends.
        assert!(k.contains(&(9.0, 11.0)) && k.contains(&(13.0, 15.0)) && k.contains(&(5.0, 7.0)));
        let w = bands(&sett, 1, 10.0, 0.0, 20.0);
        assert!(w.contains(&(11.0, 13.0)));
        // A color that is not in the sett has no bands.
        assert!(bands(&sett, 5, 10.0, 0.0, 20.0).is_empty());
        // A count so small that the stripes would be too many: no bands.
        let tiny = Sett { count_mm: 1e-6, ..sett };
        assert!(bands(&tiny, 0, 0.0, 0.0, 100_000.0).is_empty());
    }

    #[test]
    fn the_colors_of_a_tartan_sew_their_stripes_in_two_directions() {
        let shape = square(24.0);
        let sett = Sett::parse("K8 W8", 0.5, false).unwrap();
        let p = FillParams::default().with_pattern(FillPattern::Tartan {
            sett: sett.clone(),
            color: 0,
        });
        let p = FillParams { angle_deg: 0.0, ..p };
        let sew = |color: u32| {
            let travel = Travel::new(&shape);
            let mut s = Sewer::new(&travel, 3.0);
            tartan(&shape, &p, &sett, color, Point::ZERO, &mut s);
            s.finish()
        };
        let (k, w) = (sew(0), sew(1));
        assert!(!k.is_empty() && !w.is_empty());
        // Level stitches of black are in the black stripes (y in 10–14 and
        // the repeats 4 mm away), and upright ones in x the same. (The
        // steps from row to row along the edge are not counted.)
        let away = |m: Point| m.x > 0.5 && m.x < 23.5 && m.y > 0.5 && m.y < 23.5;
        let level = |runs: &[Vec<Point>]| -> Vec<Point> {
            runs.iter()
                .flat_map(|r| r.windows(2))
                .filter(|w| (w[0].y - w[1].y).abs() < 1e-9 && (w[0].x - w[1].x).abs() > 1.0)
                .map(|w| w[0].midpoint(w[1]))
                .filter(|m| away(*m))
                .collect()
        };
        // A row exactly on the edge of a stripe can go to either color.
        let in_black = |v: f64| ((v - 10.0 + 1e-6).rem_euclid(8.0)) <= 4.0 + 2e-6;
        let in_white = |v: f64| ((v - 14.0 + 1e-6).rem_euclid(8.0)) <= 4.0 + 2e-6;
        let kl = level(&k);
        assert!(kl.len() > 20);
        assert!(kl.iter().all(|m| in_black(m.y)), "black rows in black stripes");
        assert!(level(&w).iter().all(|m| in_white(m.y)), "white rows in white stripes");
        let upright = k
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| (w[0].x - w[1].x).abs() < 1e-9 && (w[0].y - w[1].y).abs() > 1.0)
            .map(|w| w[0].midpoint(w[1]))
            .filter(|m| away(*m))
            .collect::<Vec<_>>();
        assert!(upright.len() > 20);
        let bad: Vec<&Point> = upright.iter().filter(|m| !in_black(m.x)).collect();
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn gradient_rows_have_one_color_each_and_mix() {
        let stops = [0.0, 1.0];
        // At the start all rows are color 0, at the end all color 1.
        assert!((0..50).all(|k| gradient_color(k, 0.0, &stops) == 0));
        assert!((0..50).all(|k| gradient_color(k, 1.0, &stops) == 1));
        // In the middle about half each, and well mixed.
        let mid: Vec<usize> = (0..100).map(|k| gradient_color(k, 0.5, &stops)).collect();
        let ones = mid.iter().filter(|c| **c == 1).count();
        assert!((40..=60).contains(&ones), "{ones}");
        assert!(mid.windows(4).all(|w| w.contains(&0) && w.contains(&1)));
        // Three colors with the stops in any order.
        let three = [1.0, 0.0, 0.5];
        assert_eq!(gradient_color(3, 0.0, &three), 1);
        assert_eq!(gradient_color(3, 0.5, &three), 2);
        assert_eq!(gradient_color(3, 1.0, &three), 0);
        assert_eq!(gradient_color(3, f64::NAN, &three), 1);
    }
}
