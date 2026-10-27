//! Ripple stitch (P11 Milestone 4, ADR 0047): lines that repeat the shape
//! of a line.
//!
//! Not a port. Ink/Stitch has a ripple stitch; this follows what its
//! public documentation says it looks like, in our own method:
//!
//! - The line is sampled at the same fractions of its length, and each
//!   copy is the line scaled round the target (from 100 % to the size of
//!   the last line) and, with a guide, moved along the guide.
//! - A closed line: the rings are sewn as one spiral. Each point of the
//!   spiral is between the same point of two rings, nearer to the next
//!   ring the farther it is round, so the spiral goes from each ring to the
//!   next in one turn. The last ring is sewn whole.
//! - An open line: the copies are sewn back and forth, each one from the
//!   end where the one before stopped.

use emb_geom::{BBox, Point, Polyline};
use emb_model::Ripple;

/// Most needle points of all lines of a ripple (the lines get fewer points
/// when there are many).
const MAX_POINTS: usize = 400_000;
/// The line is sampled this often, in mm.
const STEP_MM: f64 = 0.5;

/// Points at the same fractions of the length of `pts`: `n` of them, the
/// first and the last included.
fn sampled(pts: &[Point], n: usize) -> Vec<Point> {
    let mut at = vec![0.0];
    for w in pts.windows(2) {
        at.push(at[at.len() - 1] + w[0].distance(w[1]));
    }
    let total = at[at.len() - 1];
    (0..n)
        .map(|i| {
            let s = total * i as f64 / (n - 1) as f64;
            let j = at.partition_point(|&a| a <= s).clamp(1, pts.len() - 1);
            let d = at[j] - at[j - 1];
            pts[j - 1].lerp(pts[j], if d > 0.0 { (s - at[j - 1]) / d } else { 0.0 })
        })
        .collect()
}

/// The point at fraction `f` of the length of the guide.
fn along(guide: &[Point], f: f64) -> Point {
    let total: f64 = guide.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * f.clamp(0.0, 1.0);
    for w in guide.windows(2) {
        let d = w[0].distance(w[1]);
        if left <= d {
            return w[0].lerp(w[1], if d > 0.0 { left / d } else { 0.0 });
        }
        left -= d;
    }
    guide[guide.len() - 1]
}

/// The line of a ripple of `line`: the copies, joined, as one line
/// (before the needle points are set at the stitch length).
pub fn ripple_line(line: &Polyline, r: &Ripple) -> Vec<Point> {
    let mut pts = line.points.clone();
    pts.dedup();
    if line.closed && pts.len() > 2 {
        pts.push(pts[0]);
    }
    if pts.len() < 2 {
        return pts;
    }
    let lines = r.lines.clamp(1, Ripple::MAX_LINES) as usize;
    let end = (r.end_percent / 100.0).clamp(0.0, 10.0);
    let guide: Option<&[Point]> = (r.guide.len() >= 2).then_some(r.guide.as_slice());
    let target = r.target.unwrap_or_else(|| {
        BBox::from_points(pts.iter().copied())
            .map(|b| b.center())
            .unwrap_or(pts[0])
    });
    let length: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let n = ((length / STEP_MM).ceil() as usize)
        .clamp(8, 4000)
        .min((MAX_POINTS / lines).max(8));
    let base = sampled(&pts, n);
    let copy = |i: usize| -> Vec<Point> {
        let f = if lines > 1 { i as f64 / (lines - 1) as f64 } else { 0.0 };
        let scale = 1.0 + (end - 1.0) * f;
        let shift = guide.map_or(Point::ZERO, |g| along(g, f) - g[0]);
        base.iter().map(|p| target + (*p - target) * scale + shift).collect()
    };
    let mut out = Vec::new();
    if line.closed {
        // A spiral from each ring to the next.
        let m = n - 1;
        for i in 0..lines.saturating_sub(1) {
            let (a, b) = (copy(i), copy(i + 1));
            out.extend((0..m).map(|j| a[j].lerp(b[j], j as f64 / m as f64)));
        }
        let last = copy(lines - 1);
        // A last ring of size 0 is the target point.
        if end > 0.0 || guide.is_some() || lines == 1 {
            out.extend(last);
        } else {
            out.push(last[0]);
        }
    } else {
        for i in 0..lines {
            let mut c = copy(i);
            if i % 2 == 1 {
                c.reverse();
            }
            out.extend(c);
        }
    }
    out.dedup_by(|a, b| a.distance(*b) < 1e-9);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(r: f64) -> Polyline {
        let points = (0..64)
            .map(|i| {
                let t = std::f64::consts::TAU * i as f64 / 64.0;
                Point::new(emb_geom::math::cos(t) * r, emb_geom::math::sin(t) * r)
            })
            .collect();
        Polyline { points, closed: true }
    }

    #[test]
    fn a_closed_line_ripples_to_its_middle_as_a_spiral() {
        let r = Ripple {
            lines: 5,
            ..Ripple::default()
        };
        let line = ripple_line(&ring(10.0), &r);
        // Starts on the ring, ends at the middle, and always gets nearer.
        assert!((line[0].length() - 10.0).abs() < 0.1);
        assert!(line[line.len() - 1].length() < 1e-9);
        let d: Vec<f64> = line.iter().map(|p| p.length()).collect();
        assert!(d.windows(2).all(|w| w[1] <= w[0] + 0.05), "the spiral goes in");
        // Four turns from the ring to the middle: points near 7.5, 5 and 2.5 mm.
        for want in [7.5, 5.0, 2.5] {
            assert!(d.iter().any(|x| (x - want).abs() < 0.1), "{want}");
        }
    }

    #[test]
    fn an_open_line_ripples_back_and_forth_along_a_guide() {
        let line = Polyline {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            closed: false,
        };
        let r = Ripple {
            lines: 3,
            end_percent: 100.0,
            target: None,
            guide: vec![Point::new(0.0, 0.0), Point::new(0.0, 8.0)],
        };
        let pts = ripple_line(&line, &r);
        let first = pts[0];
        let last = pts[pts.len() - 1];
        // Three copies at y = 0, 4 and 8: forward, back, forward.
        assert_eq!(first, Point::new(0.0, 0.0));
        assert!(last.distance(Point::new(10.0, 8.0)) < 1e-9, "{last:?}");
        let ys: std::collections::BTreeSet<i64> = pts.iter().map(|p| (p.y * 10.0).round() as i64).collect();
        assert_eq!(ys.into_iter().collect::<Vec<_>>(), [0, 40, 80]);
        // One line is the line itself.
        let one = ripple_line(
            &line,
            &Ripple {
                lines: 1,
                ..Ripple::default()
            },
        );
        assert_eq!((one[0], one[one.len() - 1]), (line.points[0], line.points[1]));
    }

    #[test]
    fn the_target_sets_where_the_lines_go() {
        let r = Ripple {
            lines: 4,
            end_percent: 0.0,
            target: Some(Point::new(3.0, -2.0)),
            guide: Vec::new(),
        };
        let line = ripple_line(&ring(10.0), &r);
        assert!(line[line.len() - 1].distance(Point::new(3.0, -2.0)) < 1e-9);
        // Many lines on a long line keep the number of points down.
        let many = Ripple {
            lines: Ripple::MAX_LINES,
            ..Ripple::default()
        };
        assert!(ripple_line(&ring(1000.0), &many).len() <= MAX_POINTS + 4000);
    }
}
