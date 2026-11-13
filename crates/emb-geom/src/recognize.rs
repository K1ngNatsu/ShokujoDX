//! Shape recognition for the Smart Drawing tool (owner request,
//! 2026-11-13): a stroke drawn by hand that is near a straight line, an
//! ellipse, a triangle or a four-sided shape becomes that clean shape.
//!
//! - An open stroke is a line if no point is farther than 4 % of its length
//!   from the line between its ends.
//! - A stroke is closed if its ends are nearer than 15 % of its length.
//!   - Few corners (3 or 4 points after Douglas-Peucker at 6 % of the
//!     size) and a small mean distance: a polygon. Four corners near 90°
//!     make a rectangle; a side within 6° of level or upright makes it level.
//!   - Else an ellipse from the extents along the main axes, if the points
//!     are near it (mean error under 8 % of the radius). Radii within 10 % of
//!     each other make a circle; an axis within 6° of level makes it level.
//! - Else no shape: the tool keeps the stroke.

use crate::path::simplify as douglas_peucker;
use crate::simplify::turn_deg;
use crate::{Point, math};

/// A recognized shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognized {
    Line(Point, Point),
    /// A closed polygon of three or four corners.
    Polygon(Vec<Point>),
    /// A rectangle (its four corners).
    Rectangle(Vec<Point>),
    /// An ellipse: the center, the radii and the angle of the first axis
    /// (degrees, clockwise on screen).
    Ellipse {
        center: Point,
        rx: f64,
        ry: f64,
        angle_deg: f64,
    },
}

/// Snaps to level or upright within this many degrees.
const SNAP_DEG: f64 = 6.0;

fn length(pts: &[Point]) -> f64 {
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Mean distance from the points to the closed polygon `poly`.
fn mean_distance(pts: &[Point], poly: &[Point]) -> f64 {
    let n = poly.len();
    let sum: f64 = pts
        .iter()
        .map(|q| {
            (0..n)
                .map(|i| q.distance_to_segment(poly[i], poly[(i + 1) % n]))
                .fold(f64::INFINITY, f64::min)
        })
        .sum();
    sum / pts.len().max(1) as f64
}

/// An angle (degrees) snapped to a multiple of 90° if it is near one.
fn snap_angle(deg: f64) -> f64 {
    let q = (deg / 90.0).round() * 90.0;
    if (deg - q).abs() <= SNAP_DEG { q } else { deg }
}

/// The rectangle round the points, with its sides at `angle_deg`.
fn rectangle(pts: &[Point], angle_deg: f64) -> Vec<Point> {
    let (s, c) = math::sin_cos_deg(angle_deg);
    let (u, v) = (Point::new(c, s), Point::new(-s, c));
    let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in pts {
        let (a, b) = (p.dot(u), p.dot(v));
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    // The mean of the extents and of the points: a hand-drawn side wobbles.
    [(a0, b0), (a1, b0), (a1, b1), (a0, b1)]
        .into_iter()
        .map(|(a, b)| u * a + v * b)
        .collect()
}

fn polygon(pts: &[Point], size: f64) -> Option<Recognized> {
    let mut ring = pts.to_vec();
    ring.push(pts[0]);
    // Cut the ring at the point farthest from its start, so the start is
    // not taken as a corner only because the stroke starts there.
    let far = (0..pts.len())
        .max_by(|&a, &b| pts[0].distance(pts[a]).total_cmp(&pts[0].distance(pts[b])))
        .unwrap_or(0);
    let tol = 0.06 * size;
    let mut corners = douglas_peucker(&ring[..=far], tol);
    corners.pop();
    corners.extend(douglas_peucker(&ring[far..], tol));
    corners.pop();
    // Drop points that are not corners (almost straight).
    let mut changed = true;
    while changed && corners.len() > 3 {
        changed = false;
        let n = corners.len();
        if let Some(i) = (0..n).find(|&i| turn_deg(corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]) < 25.0) {
            corners.remove(i);
            changed = true;
        }
    }
    if !(3..=4).contains(&corners.len()) {
        return None;
    }
    let n = corners.len();
    let turns: Vec<f64> = (0..n)
        .map(|i| turn_deg(corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]))
        .collect();
    if turns.iter().any(|t| *t < 30.0) {
        return None;
    }
    let square = n == 4 && turns.iter().all(|t| (t - 90.0).abs() < 15.0);
    let poly = if square {
        // A rectangle: its sides at the mean angle of the sides.
        let mean = (0..4)
            .map(|i| {
                let d = corners[(i + 1) % 4] - corners[i];
                let a = math::to_degrees(math::atan2(d.y, d.x));
                // The same angle for all four sides: modulo 90°.
                a.rem_euclid(90.0)
            })
            .fold((0.0, 0.0), |(x, y), a| {
                let (s, c) = math::sin_cos_deg(4.0 * a);
                (x + c, y + s)
            });
        let angle = math::to_degrees(math::atan2(mean.1, mean.0)) / 4.0;
        rectangle(pts, snap_angle(angle))
    } else {
        corners
    };
    if mean_distance(pts, &poly) >= 0.04 * size {
        return None;
    }
    Some(if square {
        Recognized::Rectangle(poly)
    } else {
        Recognized::Polygon(poly)
    })
}

fn ellipse(pts: &[Point]) -> Option<Recognized> {
    let n = pts.len() as f64;
    let m = pts.iter().fold(Point::ZERO, |s, p| s + *p) * (1.0 / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in pts {
        let d = *p - m;
        sxx += d.x * d.x;
        syy += d.y * d.y;
        sxy += d.x * d.y;
    }
    let angle = snap_angle(math::to_degrees(0.5 * math::atan2(2.0 * sxy, sxx - syy)));
    let (s, c) = math::sin_cos_deg(angle);
    let (u, v) = (Point::new(c, s), Point::new(-s, c));
    let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in pts {
        let (a, b) = (p.dot(u), p.dot(v));
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    let (mut rx, mut ry) = ((a1 - a0) / 2.0, (b1 - b0) / 2.0);
    if rx < 1e-6 || ry < 1e-6 {
        return None;
    }
    let center = u * ((a0 + a1) / 2.0) + v * ((b0 + b1) / 2.0);
    let err: f64 = pts
        .iter()
        .map(|p| {
            let d = *p - center;
            let (x, y) = (d.dot(u) / rx, d.dot(v) / ry);
            ((x * x + y * y).sqrt() - 1.0).abs()
        })
        .sum::<f64>()
        / n;
    if err > 0.08 {
        return None;
    }
    let mut angle_deg = angle;
    if (rx - ry).abs() <= 0.1 * rx.max(ry) {
        let r = (rx + ry) / 2.0;
        (rx, ry, angle_deg) = (r, r, 0.0);
    }
    Some(Recognized::Ellipse {
        center,
        rx,
        ry,
        angle_deg,
    })
}

/// The shape that the stroke `pts` is near, if there is one.
pub fn recognize(pts: &[Point]) -> Option<Recognized> {
    if pts.len() < 2 {
        return None;
    }
    let len = length(pts);
    if len < 1.0 {
        return None;
    }
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    if first.distance(last) > 0.15 * len {
        let far = pts
            .iter()
            .map(|q| q.distance_to_segment(first, last))
            .fold(0.0, f64::max);
        if far > 0.04 * len {
            return None;
        }
        // Level or upright if it is near.
        let d = last - first;
        let a = math::to_degrees(math::atan2(d.y, d.x));
        let snapped = snap_angle(a);
        if snapped == a {
            return Some(Recognized::Line(first, last));
        }
        let (s, c) = math::sin_cos_deg(snapped);
        let mid = first.midpoint(last);
        let h = Point::new(c, s) * (d.length() / 2.0);
        return Some(Recognized::Line(mid - h, mid + h));
    }
    if pts.len() < 8 {
        return None;
    }
    let bb = pts.iter().fold(
        (Point::new(f64::MAX, f64::MAX), Point::new(f64::MIN, f64::MIN)),
        |(a, b), p| {
            (
                Point::new(a.x.min(p.x), a.y.min(p.y)),
                Point::new(b.x.max(p.x), b.y.max(p.y)),
            )
        },
    );
    let size = bb.0.distance(bb.1);
    polygon(pts, size).or_else(|| ellipse(pts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A wobble of up to `a` mm, the same each run.
    fn wobble(i: usize, a: f64) -> f64 {
        a * math::sin(i as f64 * 1.7) * math::cos(i as f64 * 0.37)
    }

    fn along(corners: &[Point], per_side: usize, a: f64) -> Vec<Point> {
        let mut out = Vec::new();
        let n = corners.len();
        for i in 0..n {
            for k in 0..per_side {
                let q = corners[i].lerp(corners[(i + 1) % n], k as f64 / per_side as f64);
                out.push(q + p(wobble(out.len(), a), wobble(out.len() + 7, a)));
            }
        }
        out.push(corners[0] + p(0.5, 0.3));
        out
    }

    #[test]
    fn a_wobbly_line_becomes_straight_and_level() {
        let pts: Vec<Point> = (0..=50)
            .map(|i| p(f64::from(i), 0.03 * f64::from(i) + wobble(i as usize, 0.3)))
            .collect();
        let Some(Recognized::Line(a, b)) = recognize(&pts) else {
            panic!()
        };
        assert!((a.y - b.y).abs() < 1e-9, "level: {a:?} {b:?}");
        // A curve is not a line.
        let arc: Vec<Point> = (0..=50)
            .map(|i| p(f64::from(i), 0.02 * f64::from((i - 25) * (i - 25))))
            .collect();
        assert_eq!(recognize(&arc), None);
    }

    #[test]
    fn a_hand_drawn_rectangle_becomes_level() {
        let pts = along(&[p(0.0, 0.0), p(40.0, 1.0), p(39.0, 21.0), p(0.5, 20.0)], 30, 0.4);
        let Some(Recognized::Rectangle(r)) = recognize(&pts) else {
            panic!("{:?}", recognize(&pts))
        };
        assert_eq!(r.len(), 4);
        assert!(
            (r[0].y - r[1].y).abs() < 1e-9 && (r[1].x - r[2].x).abs() < 1e-9,
            "{r:?}"
        );
    }

    #[test]
    fn a_triangle_keeps_its_corners() {
        let pts = along(&[p(0.0, 30.0), p(15.0, 0.0), p(30.0, 30.0)], 30, 0.3);
        let Some(Recognized::Polygon(t)) = recognize(&pts) else {
            panic!("{:?}", recognize(&pts))
        };
        assert_eq!(t.len(), 3);
        assert!(t.iter().any(|q| q.distance(p(15.0, 0.0)) < 1.5));
    }

    #[test]
    fn a_rough_circle_and_an_ellipse() {
        let ring = |rx: f64, ry: f64| -> Vec<Point> {
            (0..=120)
                .map(|i| {
                    let a = f64::from(i) / 120.0 * std::f64::consts::TAU;
                    p(
                        50.0 + rx * math::cos(a) + wobble(i as usize, 0.4),
                        30.0 + ry * math::sin(a),
                    )
                })
                .collect()
        };
        let Some(Recognized::Ellipse { center, rx, ry, .. }) = recognize(&ring(20.0, 19.0)) else {
            panic!()
        };
        assert!(center.distance(p(50.0, 30.0)) < 1.0);
        assert_eq!(rx, ry, "a circle");
        let Some(Recognized::Ellipse { rx, ry, angle_deg, .. }) = recognize(&ring(30.0, 12.0)) else {
            panic!()
        };
        assert!(
            (rx - 30.0).abs() < 1.0 && (ry - 12.0).abs() < 1.0 && angle_deg == 0.0,
            "{rx} {ry} {angle_deg}"
        );
    }

    #[test]
    fn a_scribble_is_not_a_shape() {
        let pts: Vec<Point> = (0..=80)
            .map(|i| {
                let t = f64::from(i) / 10.0;
                p(t * 5.0 * math::cos(t), t * 5.0 * math::sin(t))
            })
            .collect();
        assert_eq!(recognize(&pts), None);
    }
}
