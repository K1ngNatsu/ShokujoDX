//! Motif stitching (P5 Milestone 4): motif run, motif column and motif
//! fill, with gradient motifs and the 3D warp of a motif fill.
//!
//! Not a port. The method follows what public embroidery documentation
//! says the stitches look like, and common practice:
//!
//! - A motif goes from its first reference point (0, 0) to its second
//!   (1, 0). To place it between two points `a` and `b`, its length is
//!   scaled to `|b - a|` and its height to the height setting; it turns
//!   with the line from `a` to `b`. Long stitches get needle points.
//! - Motif run: the line is cut into equal lengths, as near to the motif
//!   length (and the gap) as fits, so the motifs fill the line from end to
//!   end. Each motif goes from one cut to the next.
//! - Motif column: each motif is as high as the column is wide where it
//!   starts, and `aspect` times as long. The lengths are then scaled so the
//!   motifs fill the column from end to end.
//! - Motif fill: rows at the fill angle, one row spacing apart; every
//!   second row moves on by the stagger. A gradient changes the size of the
//!   motifs (and the row spacing) evenly from the first row to the last.
//!   The parts of motifs outside the shape are cut off.
//! - 3D warp ("globe"): the rows are laid over the circle round the shape,
//!   then each point moves along its radius as on a ball seen from the
//!   front (`r' = R · sin(π/2 · r/R)`), so the middle grows and the edge
//!   gets smaller.
//!
//! Only `sqrt`, `sin` and `cos` from `emb_geom::math` are used (ADR 0003).

use emb_geom::math::sin;
use emb_geom::{Point, Polyline};
use emb_model::{FillParams, Motif, MotifRun};

use crate::curves::{clip, sew_line};
use crate::running;
use crate::travel::{Sewer, Shape};

/// Shortest motif repeat that the generator accepts, in mm.
const MIN_REPEAT_MM: f64 = 0.5;
/// Most motifs in one row, column or run.
const MAX_REPEATS: usize = 20_000;
/// Longest stitch in a motif column, in mm.
const COLUMN_STITCH_MM: f64 = 2.5;

/// The motif placed from `a` to `b`, `h` high across the line, as needle
/// points no more than `len` apart (from `a` to `b`).
pub fn place(m: &Motif, a: Point, b: Point, h: f64, len: f64) -> Vec<Point> {
    place_at(m, a, b, h, len, false)
}

/// As [`place`]; with `centered`, the middle of the motif height is on the
/// line from `a` to `b` (for columns and rows, where the motif must stay
/// inside a band), else its reference line is.
fn place_at(m: &Motif, a: Point, b: Point, h: f64, len: f64, centered: bool) -> Vec<Point> {
    let d = b - a;
    let Some(u) = d.normalized() else {
        return vec![a];
    };
    let n = Point::new(-u.y, u.x);
    let l = d.length();
    let scale = h / m.height();
    let shift = if centered {
        let (lo, hi) = m.points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        -(lo + hi) / 2.0
    } else {
        0.0
    };
    let pts: Vec<Point> = m
        .points
        .iter()
        .map(|p| a + u * (p.x * l) + n * ((p.y + shift) * scale))
        .collect();
    running::running_points(
        &Polyline {
            points: pts,
            closed: false,
        },
        len,
    )
}

/// A polyline with the length along it at each point.
struct Walk {
    pts: Vec<Point>,
    at: Vec<f64>,
}

impl Walk {
    fn new(line: &Polyline) -> Walk {
        let mut pts = line.points.clone();
        if line.closed && pts.len() > 2 && pts.first() != pts.last() {
            pts.push(pts[0]);
        }
        pts.dedup();
        let mut at = vec![0.0];
        for w in pts.windows(2) {
            at.push(at[at.len() - 1] + w[0].distance(w[1]));
        }
        Walk { pts, at }
    }

    fn len(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    fn point(&self, s: f64) -> Point {
        let s = s.clamp(0.0, self.len());
        let i = self.at.partition_point(|&l| l <= s).clamp(1, self.pts.len() - 1);
        let seg = self.at[i] - self.at[i - 1];
        let t = if seg > 0.0 { (s - self.at[i - 1]) / seg } else { 0.0 };
        self.pts[i - 1].lerp(self.pts[i], t)
    }

    /// The part of the line from `s0` to `s1`.
    fn part(&self, s0: f64, s1: f64) -> Vec<Point> {
        let mut v = vec![self.point(s0)];
        v.extend(
            self.pts
                .iter()
                .zip(&self.at)
                .filter(|(_, l)| **l > s0 + 1e-9 && **l < s1 - 1e-9)
                .map(|(p, _)| *p),
        );
        v.push(self.point(s1));
        v
    }
}

/// Needle points of a motif run along `line`.
pub fn run(line: &Polyline, r: &MotifRun, len: f64) -> Vec<Point> {
    let w = Walk::new(line);
    let total = w.len();
    if w.pts.len() < 2 || total <= 0.0 {
        return w.pts;
    }
    let repeat = r.length_mm.max(MIN_REPEAT_MM) + r.gap_mm.max(0.0);
    let n = ((total / repeat).round() as usize).clamp(1, MAX_REPEATS);
    // Scale the motif and the gap so that `n` repeats fill the line.
    let f = total / (n as f64 * repeat);
    let (ml, gap) = (r.length_mm.max(MIN_REPEAT_MM) * f, r.gap_mm.max(0.0) * f);
    let mut out = vec![w.point(0.0)];
    let mut s = 0.0;
    for _ in 0..n {
        let (a, b) = (w.point(s), w.point(s + ml));
        out.extend(place(&r.motif, a, b, r.height_mm, len).into_iter().skip(1));
        s += ml;
        if gap > 0.0 {
            let part = Polyline {
                points: w.part(s, s + gap),
                closed: false,
            };
            out.extend(running::running_points(&part, len).into_iter().skip(1));
            s += gap;
        }
    }
    out.dedup();
    out
}

/// Needle points of a motif column through `pairs` (point pairs across the
/// column from its start to its end): each motif as high as the column is
/// wide, `aspect` times as long.
pub fn column(pairs: &[(Point, Point)], m: &Motif, aspect: f64) -> Vec<Point> {
    let mids: Vec<Point> = pairs.iter().map(|(a, b)| a.midpoint(*b)).collect();
    let widths: Vec<f64> = pairs.iter().map(|(a, b)| a.distance(*b)).collect();
    let w = Walk::new(&Polyline {
        points: mids.clone(),
        closed: false,
    });
    let total = w.len();
    if mids.len() < 2 || total <= 0.0 {
        return mids;
    }
    let width_at = |s: f64| {
        let i =
            w.at.partition_point(|&l| l <= s)
                .clamp(1, widths.len().min(w.at.len()) - 1);
        widths[i.min(widths.len() - 1)]
    };
    // The cuts: each motif as long as `aspect` times the width at its start.
    let mut cuts = vec![0.0];
    while cuts[cuts.len() - 1] < total && cuts.len() < MAX_REPEATS {
        let s = cuts[cuts.len() - 1];
        cuts.push(s + (aspect.max(0.1) * width_at(s)).max(MIN_REPEAT_MM));
    }
    // Drop a last cut that is less than half a motif on, then fit the rest.
    if cuts.len() > 2 {
        let (prev, last) = (cuts[cuts.len() - 2], cuts[cuts.len() - 1]);
        if total - prev < (last - prev) / 2.0 {
            cuts.pop();
        }
    }
    let f = total / cuts[cuts.len() - 1];
    let mut out = vec![w.point(0.0)];
    for c in cuts.windows(2) {
        let (s0, s1) = (c[0] * f, c[1] * f);
        let h = width_at((s0 + s1) / 2.0);
        out.extend(
            place_at(m, w.point(s0), w.point(s1), h, COLUMN_STITCH_MM, true)
                .into_iter()
                .skip(1),
        );
    }
    out.dedup();
    out
}

/// Moves `p` as on a ball of radius `r` round `c` (3D warp).
fn globe(p: Point, c: Point, r: f64) -> Point {
    let d = p - c;
    let dist = d.length();
    if dist <= 0.0 || r <= 0.0 {
        return p;
    }
    let t = (dist / r).min(1.0);
    c + d * (r * sin(std::f64::consts::FRAC_PI_2 * t) / dist)
}

/// The settings of a motif fill.
pub struct MotifFill<'a> {
    pub motif: &'a Motif,
    pub length_mm: f64,
    pub height_mm: f64,
    pub stagger: f64,
    pub gradient: f64,
    pub globe: bool,
}

/// Sews a motif fill.
pub fn fill(shape: &Shape, p: &FillParams, f: &MotifFill, sewer: &mut Sewer) {
    let pts: Vec<Point> = shape.rings().flatten().copied().collect();
    let Some(bbox) = emb_geom::BBox::from_points(pts.iter().copied()) else {
        return;
    };
    let center = bbox.center();
    let reach = pts.iter().map(|q| q.distance(center)).fold(0.0, f64::max);
    let (s, c) = emb_geom::math::sin_cos_deg(p.angle_deg);
    let (dir, n) = (Point::new(c, s), Point::new(-s, c));
    // The area to lay rows over, in row coordinates (u along, v across).
    let corners: Vec<Point> = if f.globe {
        vec![
            center + Point::new(-reach, -reach),
            center + Point::new(reach, -reach),
            center + Point::new(reach, reach),
            center + Point::new(-reach, reach),
        ]
    } else {
        pts
    };
    let (mut u0, mut u1, mut v0, mut v1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for q in &corners {
        u0 = u0.min(q.dot(dir));
        u1 = u1.max(q.dot(dir));
        v0 = v0.min(q.dot(n));
        v1 = v1.max(q.dot(n));
    }
    let spacing = p.spacing_mm.max(MIN_REPEAT_MM);
    let size_at = |v: f64| {
        let t = if v1 > v0 {
            ((v - v0) / (v1 - v0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // The panel gives 0.25 to 4; a wrong value in a file stays in a range that sews.
        1.0 + (f.gradient.clamp(0.05, 20.0) - 1.0) * t
    };
    let len = p.stitch_length_mm.max(running::MIN_SETTING_MM);
    let mut v = v0 + spacing * size_at(v0) / 2.0;
    let mut row = 0;
    while v < v1 + spacing && row < MAX_REPEATS {
        let g = size_at(v);
        let l = (f.length_mm * g).max(MIN_REPEAT_MM);
        let h = f.height_mm * g;
        let shift = if row % 2 == 1 { f.stagger * l } else { 0.0 };
        let mut pieces: Vec<Vec<Point>> = Vec::new();
        let mut u = u0 - l + shift.rem_euclid(l);
        let mut k = 0;
        while u < u1 && k < MAX_REPEATS {
            let a = dir * u + n * v;
            let mut motif = place_at(f.motif, a, a + dir * l, h, len, true);
            if f.globe {
                for q in &mut motif {
                    *q = globe(*q, center, reach);
                }
            }
            pieces.extend(clip(&motif, shape));
            u += l;
            k += 1;
        }
        if row % 2 == 1 {
            pieces.reverse();
            for piece in &mut pieces {
                piece.reverse();
            }
        }
        for piece in pieces {
            sew_line(sewer, &piece);
        }
        v += spacing * g;
        row += 1;
    }
}

#[cfg(test)]
mod tests {
    use emb_model::FillPattern;

    use super::*;
    use crate::travel::Travel;

    fn peak() -> Motif {
        Motif::builtin("Peak").unwrap()
    }

    #[test]
    fn a_motif_turns_and_scales_with_its_points() {
        let pts = place(&peak(), Point::new(0.0, 0.0), Point::new(0.0, 10.0), 4.0, 100.0);
        // Going down the screen, the peak points to +x (the motif's up is to the left of the line).
        assert_eq!(pts.len(), 3);
        assert!(pts[1].distance(Point::new(4.0, 5.0)) < 1e-9, "{:?}", pts[1]);
        let fine = place(&peak(), Point::ZERO, Point::new(10.0, 0.0), 4.0, 1.0);
        assert!(fine.windows(2).all(|w| w[0].distance(w[1]) <= 1.0 + 1e-9));
    }

    #[test]
    fn a_motif_run_fills_the_line_from_end_to_end() {
        let line = Polyline {
            points: vec![Point::ZERO, Point::new(23.0, 0.0)],
            closed: false,
        };
        let r = MotifRun::new(peak());
        let pts = run(&line, &r, 10.0);
        // 23 mm at about 5 mm: 5 peaks of 4.6 mm.
        let tops = pts.iter().filter(|q| (q.y + 3.0).abs() < 1e-9).count();
        assert_eq!(tops, 5);
        assert_eq!(pts[0], Point::ZERO);
        assert!(pts[pts.len() - 1].distance(Point::new(23.0, 0.0)) < 1e-9);
        // A gap adds straight runs between the motifs.
        let gapped = run(&line, &MotifRun { gap_mm: 2.0, ..r }, 10.0);
        let tops = gapped.iter().filter(|q| (q.y + 3.0).abs() < 1e-9).count();
        assert_eq!(tops, 3);
    }

    #[test]
    fn a_motif_column_follows_the_width() {
        // A column 2 mm wide at the start and 6 mm at the end.
        let pairs: Vec<(Point, Point)> = (0..=100)
            .map(|i| {
                let x = f64::from(i) * 0.3;
                let w = 2.0 + 4.0 * f64::from(i) / 100.0;
                (Point::new(x, -w / 2.0), Point::new(x, w / 2.0))
            })
            .collect();
        let pts = column(&pairs, &peak(), 1.0);
        assert!(pts.iter().all(|q| q.y >= -3.0 - 1e-6 && q.y <= 3.0 + 1e-6));
        // Narrow motifs at the start, wide ones at the end.
        let near = pts.iter().filter(|q| q.x < 5.0).count();
        let far = pts.iter().filter(|q| q.x > 25.0).count();
        assert!(near > far, "{near} {far}");
        assert!((pts[pts.len() - 1].x - 30.0).abs() < 1e-6, "it ends at the end");
    }

    fn fill_runs(pattern: FillPattern) -> Vec<Vec<Point>> {
        let shape = Shape::new(
            &[Polyline {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(40.0, 0.0),
                    Point::new(40.0, 40.0),
                    Point::new(0.0, 40.0),
                ],
                closed: true,
            }],
            &[],
        );
        let travel = Travel::new(&shape);
        let mut sewer = Sewer::new(&travel, 3.0);
        let p = FillParams {
            angle_deg: 0.0,
            ..FillParams::default().with_pattern(pattern)
        };
        crate::curves::sew(&shape, &p, Point::ZERO, &mut sewer);
        sewer.finish()
    }

    fn motif_fill(gradient: f64, globe: bool) -> FillPattern {
        FillPattern::Motif {
            motif: peak(),
            length_mm: 6.0,
            height_mm: 4.0,
            stagger: 0.5,
            gradient,
            globe,
        }
    }

    #[test]
    fn motif_fill_rows_cover_the_shape_and_stay_inside() {
        let runs = fill_runs(motif_fill(1.0, false));
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        assert!(
            pts.iter()
                .all(|q| q.x > -0.05 && q.x < 40.05 && q.y > -0.05 && q.y < 40.05)
        );
        // 6 or 7 rows of peaks, 6 mm apart.
        let tops = pts.iter().filter(|q| q.y > 35.0).count();
        assert!(tops > 5);
        // A gradient makes the last rows larger: fewer needle points there.
        let grad: Vec<Point> = fill_runs(motif_fill(2.0, false)).into_iter().flatten().collect();
        let (top, bottom) = (
            grad.iter().filter(|q| q.y < 10.0).count(),
            grad.iter().filter(|q| q.y > 30.0).count(),
        );
        assert!(top > bottom, "{top} {bottom}");
    }

    #[test]
    fn the_globe_makes_the_middle_larger() {
        let flat: Vec<Point> = fill_runs(motif_fill(1.0, false)).into_iter().flatten().collect();
        let ball: Vec<Point> = fill_runs(motif_fill(1.0, true)).into_iter().flatten().collect();
        let middle = |v: &[Point]| v.iter().filter(|q| q.distance(Point::new(20.0, 20.0)) < 8.0).count();
        let edge = |v: &[Point]| v.iter().filter(|q| q.distance(Point::new(20.0, 20.0)) > 16.0).count();
        // Fewer, larger motifs in the middle; more, smaller ones at the edge.
        assert!(middle(&ball) < middle(&flat), "{} {}", middle(&ball), middle(&flat));
        assert!(edge(&ball) > edge(&flat), "{} {}", edge(&ball), edge(&flat));
    }
}
