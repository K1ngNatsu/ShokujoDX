//! Running stitch along a polyline.
//!
//! Not a port: even spacing along a line is common knowledge.
//!
//! - Real corners always get a needle point, so the stitched line keeps its
//!   shape. A corner is a vertex where the line turns more than 30°.
//! - Between corners, stitches have equal length, not more than the set length.
//! - Bean stitch sews each stitch forward, back and forward again.
//!
//! Only `sqrt` and basic arithmetic are used, so the output is the same on
//! all platforms (ADR 0003).

use emb_geom::{Point, Polyline};

/// cos(30°). A vertex where the direction changes by more than 30° is a corner.
const COS_CORNER: f64 = 0.866_025_403_784_438_6;
/// Shortest stitch length that the generator accepts as a setting, in mm.
pub const MIN_SETTING_MM: f64 = 0.5;

/// Needle points along the polyline. For a closed polyline the last point is
/// the first point again.
pub fn running_points(line: &Polyline, stitch_len: f64) -> Vec<Point> {
    let stitch_len = if stitch_len.is_finite() {
        stitch_len.max(MIN_SETTING_MM)
    } else {
        MIN_SETTING_MM
    };
    let mut pts: Vec<Point> = Vec::with_capacity(line.points.len() + 1);
    for p in &line.points {
        if pts.last() != Some(p) {
            pts.push(*p);
        }
    }
    if line.closed && pts.len() > 2 && pts.first() != pts.last() {
        pts.push(pts[0]);
    }
    if pts.len() < 2 {
        return pts;
    }

    // Indices of points that must be needle points.
    let mut corners = vec![0];
    for i in 1..pts.len() - 1 {
        let a = pts[i] - pts[i - 1];
        let b = pts[i + 1] - pts[i];
        let cos = a.dot(b) / (a.length() * b.length());
        if cos < COS_CORNER {
            corners.push(i);
        }
    }
    corners.push(pts.len() - 1);

    let mut out = vec![pts[0]];
    for w in corners.windows(2) {
        let section = &pts[w[0]..=w[1]];
        let lengths: Vec<f64> = section.windows(2).map(|s| s[0].distance(s[1])).collect();
        let total: f64 = lengths.iter().sum();
        // A small tolerance, so an exact multiple does not get an extra stitch.
        let n = ((total / stitch_len) - 1e-9).ceil().max(1.0) as usize;
        let step = total / n as f64;
        // Walk the section and place points at step, 2*step, ...
        let mut seg = 0;
        let mut seg_start = 0.0;
        for k in 1..n {
            let target = step * k as f64;
            while seg + 1 < lengths.len() && seg_start + lengths[seg] < target {
                seg_start += lengths[seg];
                seg += 1;
            }
            let t = if lengths[seg] > 0.0 {
                ((target - seg_start) / lengths[seg]).clamp(0.0, 1.0)
            } else {
                0.0
            };
            out.push(section[seg].lerp(section[seg + 1], t));
        }
        out.push(*section.last().expect("a section has two or more points"));
    }
    out
}

/// Side offset of the forward stitches of a stem stitch, in mm.
pub const STEM_OFFSET_MM: f64 = 0.4;

/// Backstitch from points at half the stitch length: two steps forward, one
/// back (q0, q2, q1, q3, q2, ... , qn). Each forward stitch is one stitch
/// length, and each part of the line is sewn two times.
pub fn backstitch(q: &[Point]) -> Vec<Point> {
    if q.len() < 3 {
        return q.to_vec();
    }
    let mut out = vec![q[0]];
    for k in 0..q.len() - 2 {
        out.push(q[k + 2]);
        if k + 2 < q.len() - 1 {
            out.push(q[k + 1]);
        }
    }
    out
}

/// Stem stitch: a backstitch whose forward stitches end a little to the
/// left of the line, so the stitches lie at an angle like a twisted cord.
pub fn stemstitch(q: &[Point]) -> Vec<Point> {
    if q.len() < 3 {
        return q.to_vec();
    }
    let side = |i: usize| -> Point {
        let a = q[i.saturating_sub(1)];
        let b = q[(i + 1).min(q.len() - 1)];
        // Left of the direction on screen (Y down) is (d.y, -d.x).
        (b - a)
            .normalized()
            .map_or(Point::ZERO, |d| Point::new(d.y, -d.x) * STEM_OFFSET_MM)
    };
    let mut out = vec![q[0]];
    for k in 0..q.len() - 2 {
        let last = k + 2 == q.len() - 1;
        // The last needle point is on the line, so the next object starts there.
        out.push(if last { q[k + 2] } else { q[k + 2] + side(k + 2) });
        if !last {
            out.push(q[k + 1]);
        }
    }
    out
}

/// Sculpture run: the stitches are triple and single in turn (forward,
/// back and forward on the first stitch, once on the second, ...), so the
/// line looks like a string of beads.
pub fn sculpture(points: &[Point]) -> Vec<Point> {
    let mut out = Vec::with_capacity(points.len() * 2);
    out.extend(points.first());
    for (k, w) in points.windows(2).enumerate() {
        out.push(w[1]);
        if k % 2 == 0 {
            out.push(w[0]);
            out.push(w[1]);
        }
    }
    out
}

/// The most times a bean stitch sews each stitch: a wrong value in a project
/// file (10⁹) made the stitch list too large for the memory (audit of P11
/// and P12).
pub const MAX_REPEATS: u32 = 9;

/// Bean stitch: each stitch is sewn `repeats` times (forward, back, forward, ...).
/// An even value is rounded up, so the needle always ends at the far end.
/// `repeats` of 0 or 1 returns the points unchanged; more than
/// [`MAX_REPEATS`] is [`MAX_REPEATS`].
pub fn bean(points: &[Point], repeats: u32) -> Vec<Point> {
    if repeats <= 1 || points.len() < 2 {
        return points.to_vec();
    }
    let back_and_forth = (repeats.min(MAX_REPEATS) | 1) / 2;
    let mut out = Vec::with_capacity(points.len() * (2 * back_and_forth as usize + 1));
    out.push(points[0]);
    for w in points.windows(2) {
        out.push(w[1]);
        for _ in 0..back_and_forth {
            out.push(w[0]);
            out.push(w[1]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn a_wrong_bean_count_stays_small() {
        let pts = [Point::new(0.0, 0.0), Point::new(3.0, 0.0)];
        assert_eq!(bean(&pts, 1_000_000_000), bean(&pts, MAX_REPEATS));
        assert_eq!(bean(&pts, MAX_REPEATS).len(), 1 + MAX_REPEATS as usize);
    }

    fn line(pts: &[(f64, f64)], closed: bool) -> Polyline {
        Polyline {
            points: pts.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            closed,
        }
    }

    fn gaps(pts: &[Point]) -> Vec<f64> {
        pts.windows(2).map(|w| w[0].distance(w[1])).collect()
    }

    #[test]
    fn straight_line_even_spacing() {
        let p = running_points(&line(&[(0.0, 0.0), (10.0, 0.0)], false), 2.5);
        assert_eq!(p.len(), 5);
        assert!(gaps(&p).iter().all(|g| (g - 2.5).abs() < 1e-12));
        // 10 mm at 3 mm: four stitches of 2.5 mm (equal, not more than 3).
        let p = running_points(&line(&[(0.0, 0.0), (10.0, 0.0)], false), 3.0);
        assert_eq!(p.len(), 5);
    }

    #[test]
    fn corners_are_kept_and_loop_closes() {
        let sq = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], true);
        let p = running_points(&sq, 3.0);
        for c in [(10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
            assert!(p.contains(&Point::new(c.0, c.1)), "corner {c:?} missing");
        }
        assert_eq!(p.first(), p.last());
        // Each side: 10 mm at 3 mm -> 4 stitches. 4 sides -> 16 stitches.
        assert_eq!(p.len(), 17);
    }

    #[test]
    fn gentle_curve_is_not_cut_at_every_vertex() {
        // A flattened arc has many vertices with small turns.
        let pts: Vec<(f64, f64)> = (0..=40)
            .map(|i| {
                let a = f64::from(i) / 40.0 * std::f64::consts::FRAC_PI_2;
                (20.0 * a.cos(), 20.0 * a.sin())
            })
            .collect();
        let p = running_points(&line(&pts, false), 2.5);
        // Arc length 31.4 mm at 2.5 mm: 13 stitches, not 40.
        assert_eq!(p.len(), 14);
    }

    #[test]
    fn backstitch_and_stemstitch_patterns() {
        let q: Vec<Point> = (0..5).map(|i| Point::new(f64::from(i), 0.0)).collect();
        let b = backstitch(&q);
        let xs: Vec<f64> = b.iter().map(|p| p.x).collect();
        assert_eq!(xs, [0.0, 2.0, 1.0, 3.0, 2.0, 4.0]);
        let s = stemstitch(&q);
        assert_eq!(s.len(), b.len());
        assert!(
            (s[1].y + STEM_OFFSET_MM).abs() < 1e-9,
            "forward stitch ends to the left: {:?}",
            s[1]
        );
        assert_eq!(s[2], q[1]);
        assert_eq!(*s.last().unwrap(), q[4]);
    }

    #[test]
    fn sculpture_pattern() {
        let p: Vec<Point> = (0..5).map(|i| Point::new(f64::from(i), 0.0)).collect();
        let x: Vec<f64> = sculpture(&p).iter().map(|q| q.x).collect();
        assert_eq!(x, vec![0.0, 1.0, 0.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0]);
        assert_eq!(sculpture(&p[..1]), p[..1].to_vec());
    }

    #[test]
    fn bean_pattern() {
        let pts = [Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(2.0, 0.0)];
        let b = bean(&pts, 3);
        let xs: Vec<f64> = b.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![0.0, 1.0, 0.0, 1.0, 2.0, 1.0, 2.0]);
        assert_eq!(bean(&pts, 1), pts.to_vec());
        assert_eq!(bean(&pts, 2).len(), b.len(), "even rounds up to odd");
    }

    #[test]
    fn degenerate_input() {
        assert!(running_points(&line(&[], false), 2.5).is_empty());
        assert_eq!(running_points(&line(&[(1.0, 1.0), (1.0, 1.0)], false), 2.5).len(), 1);
        // A silly setting does not make millions of points.
        assert_eq!(running_points(&line(&[(0.0, 0.0), (1.0, 0.0)], false), 0.0).len(), 3);
    }

    fn pt() -> impl Strategy<Value = (f64, f64)> {
        (-100.0..100.0f64, -100.0..100.0f64)
    }

    proptest! {
        #[test]
        fn invariants(pts in prop::collection::vec(pt(), 2..12), len in 0.5..8.0f64, closed: bool) {
            let l = line(&pts, closed);
            let out = running_points(&l, len);
            // No stitch longer than the setting, no NaN, ends kept.
            for g in gaps(&out) {
                prop_assert!(g <= len + 1e-9, "{g} > {len}");
            }
            prop_assert!(out.iter().all(|p| p.is_finite()));
            prop_assert_eq!(out.first().copied(), Some(l.points[0]));
            // Every needle point is on the polyline.
            let mut segs: Vec<(Point, Point)> = l.points.windows(2).map(|w| (w[0], w[1])).collect();
            if closed {
                segs.push((*l.points.last().unwrap(), l.points[0]));
            }
            for p in &out {
                let d = segs.iter().map(|(a, b)| p.distance_to_segment(*a, *b)).fold(f64::INFINITY, f64::min);
                prop_assert!(d < 1e-6, "{p:?} is {d} mm off the line");
            }
            // Deterministic.
            prop_assert_eq!(out, running_points(&l, len));
        }
    }
}
