//! Lettering baselines: the line that the letters stand on (P4 Milestone 3,
//! ADR 0017).
//!
//! Layout first puts the letters on a straight line in lettering space (see
//! `lettering`). A baseline then moves each letter as one rigid shape: the
//! anchor of the letter (the middle of its advance, on its line) goes to the
//! point at the same distance along the baseline, and the letter turns with
//! the baseline there. The letters do not bend.

use emb_geom::{Path, Point, Transform, math};
use serde::{Deserialize, Serialize};

/// The smallest radius of an arc or circle line, in mm.
pub const MIN_RADIUS_MM: f64 = 2.0;

/// The line that the letters stand on, in lettering space.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Baseline {
    /// The X axis.
    #[default]
    Straight,
    /// A circle arc through the origin. Clockwise: the center is
    /// `radius_mm` below the origin and the letters stand on the outside
    /// (text over the top of the circle). Counter-clockwise: the center is
    /// above the origin and the letters hang on the inside (text along the
    /// bottom of the circle).
    Arc { radius_mm: f64, clockwise: bool },
    /// As `Arc`, but the center of the circle is the origin: the text starts
    /// at the top (clockwise) or at the bottom (counter-clockwise). A change
    /// of the radius keeps the center where it is.
    Circle { radius_mm: f64, clockwise: bool },
    /// A path drawn by the user, from its first point. Letters past the
    /// ends continue in a straight line.
    Path { path: Path },
}

impl Baseline {
    pub fn is_straight(&self) -> bool {
        *self == Baseline::Straight
    }

    /// The radius of an arc or circle.
    pub fn radius(&self) -> Option<f64> {
        match self {
            Baseline::Arc { radius_mm, .. } | Baseline::Circle { radius_mm, .. } => Some(*radius_mm),
            _ => None,
        }
    }

    /// The center of an arc or circle, in lettering space.
    pub fn center(&self) -> Option<Point> {
        match *self {
            Baseline::Arc { radius_mm, clockwise } => {
                Some(Point::new(0.0, if clockwise { radius_mm } else { -radius_mm }))
            }
            Baseline::Circle { .. } => Some(Point::ZERO),
            _ => None,
        }
    }

    /// The length of a path baseline. Justification uses it: left starts
    /// the text at the start of the path, center puts the middle of the
    /// text at the middle of the path, right ends it at the end.
    pub fn path_length(&self) -> Option<f64> {
        match self {
            Baseline::Path { path } => Some(Track::new(path).length()),
            _ => None,
        }
    }

    /// The point at distance `x` along the line `y` mm below the baseline
    /// (lines go down), and the direction of the line there in degrees
    /// (clockwise on screen, 0 = to the right). On a path, the direction is
    /// that of the chord from `x - half` to `x + half`.
    pub fn frame(&self, x: f64, y: f64, half: f64) -> (Point, f64) {
        match self {
            Baseline::Straight => (Point::new(x, y), 0.0),
            Baseline::Arc { radius_mm, clockwise } => arc_frame(*radius_mm, *clockwise, x, y),
            Baseline::Circle { radius_mm, clockwise } => {
                let (p, deg) = arc_frame(*radius_mm, *clockwise, x, y);
                let r = radius_mm.max(MIN_RADIUS_MM);
                let dy = if *clockwise { -r } else { r };
                (Point::new(p.x, p.y + dy), deg)
            }
            Baseline::Path { path } => Track::new(path).frame(x, y, half),
        }
    }

    /// Moves a letter from the straight layout onto the baseline: its anchor
    /// `(x, y)` goes to [`Baseline::frame`] and it turns with the line.
    /// `half` is half the advance of the letter.
    pub fn letter_transform(&self, x: f64, y: f64, half: f64) -> Transform {
        if self.is_straight() {
            return Transform::IDENTITY;
        }
        let (p, deg) = self.frame(x, y, half);
        Transform::translate(-x, -y)
            .then(Transform::rotate_deg(deg))
            .then(Transform::translate(p.x, p.y))
    }

    /// Points on the line of `y`, from distance `from` to `to`, at most
    /// `step` mm apart (to draw the baseline).
    pub fn polyline(&self, from: f64, to: f64, y: f64, step: f64) -> Vec<Point> {
        let n = (((to - from).abs() / step.max(0.01)).ceil() as usize).clamp(1, 2000);
        (0..=n)
            .map(|i| self.frame(from + (to - from) * i as f64 / n as f64, y, 0.5).0)
            .collect()
    }
}

/// Arc baseline through the origin (see [`Baseline::Arc`]).
fn arc_frame(radius: f64, clockwise: bool, x: f64, y: f64) -> (Point, f64) {
    let r = radius.max(MIN_RADIUS_MM);
    if clockwise {
        // Lower lines are nearer the center.
        let rl = (r - y).max(MIN_RADIUS_MM);
        let phi = x / rl;
        (
            Point::new(rl * math::sin(phi), r - rl * math::cos(phi)),
            math::to_degrees(phi),
        )
    } else {
        let rl = (r + y).max(MIN_RADIUS_MM);
        let phi = x / rl;
        (
            Point::new(rl * math::sin(phi), rl * math::cos(phi) - r),
            -math::to_degrees(phi),
        )
    }
}

/// A path as a polyline with distances along it.
struct Track {
    points: Vec<Point>,
    /// Distance from the start to each point.
    at: Vec<f64>,
}

impl Track {
    fn new(path: &Path) -> Track {
        let mut points: Vec<Point> = Vec::new();
        for pl in path.flatten(0.02) {
            for p in pl.points {
                if points.last().is_none_or(|q| q.distance(p) > 1e-9) {
                    points.push(p);
                }
            }
        }
        let mut at = Vec::with_capacity(points.len());
        let mut d = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                d += points[i - 1].distance(*p);
            }
            at.push(d);
        }
        Track { points, at }
    }

    fn length(&self) -> f64 {
        self.at.last().copied().unwrap_or(0.0)
    }

    /// The point at distance `s`, past the ends on the end directions.
    fn point(&self, s: f64) -> Point {
        let n = self.points.len();
        if n == 0 {
            return Point::new(s, 0.0);
        }
        if n == 1 {
            return self.points[0] + Point::new(s, 0.0);
        }
        let i = match self.at.partition_point(|&d| d <= s) {
            0 => 1,
            i if i >= n => n - 1,
            i => i,
        };
        let (a, b) = (self.points[i - 1], self.points[i]);
        let len = self.at[i] - self.at[i - 1];
        a.lerp(b, (s - self.at[i - 1]) / len)
    }

    fn frame(&self, x: f64, y: f64, half: f64) -> (Point, f64) {
        let half = half.max(0.25);
        let d = self.point(x + half) - self.point(x - half);
        let deg = math::to_degrees(math::atan2(d.y, d.x));
        let (s, c) = math::sin_cos_deg(deg);
        // Down from the line: the direction turned a quarter clockwise.
        (self.point(x) + Point::new(-s, c) * y, deg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        a.distance(b) < 1e-6
    }

    #[test]
    fn straight_is_the_identity() {
        let b = Baseline::Straight;
        assert_eq!(b.frame(3.0, 4.0, 1.0), (Point::new(3.0, 4.0), 0.0));
        assert_eq!(b.letter_transform(3.0, 4.0, 1.0), Transform::IDENTITY);
    }

    #[test]
    fn arcs_go_round_their_centers() {
        let r = 20.0;
        let quarter = std::f64::consts::FRAC_PI_2 * r;
        let cw = Baseline::Arc {
            radius_mm: r,
            clockwise: true,
        };
        assert!(close(cw.frame(0.0, 0.0, 1.0).0, Point::ZERO));
        let (p, deg) = cw.frame(quarter, 0.0, 1.0);
        assert!(close(p, Point::new(r, r)), "{p:?}");
        assert!((deg - 90.0).abs() < 1e-9);
        // A lower line is on a smaller circle, at the same angle for the
        // same distance along it.
        let (p, _) = cw.frame(std::f64::consts::FRAC_PI_2 * 15.0, 5.0, 1.0);
        assert!(close(p, Point::new(15.0, r)), "{p:?}");
        let ccw = Baseline::Arc {
            radius_mm: r,
            clockwise: false,
        };
        let (p, deg) = ccw.frame(quarter, 0.0, 1.0);
        assert!(close(p, Point::new(r, -r)), "{p:?}");
        assert!((deg + 90.0).abs() < 1e-9);
        assert_eq!(ccw.center(), Some(Point::new(0.0, -r)));
        // A letter on top of the clockwise arc turns with it: its top
        // points away from the center.
        let t = cw.letter_transform(quarter, 0.0, 1.0);
        let up = t.apply(Point::new(quarter, -1.0)) - t.apply(Point::new(quarter, 0.0));
        assert!(close(up, Point::new(1.0, 0.0)), "{up:?}");
    }

    #[test]
    fn circles_keep_the_center_at_the_origin() {
        let top = Baseline::Circle {
            radius_mm: 30.0,
            clockwise: true,
        };
        assert!(close(top.frame(0.0, 0.0, 1.0).0, Point::new(0.0, -30.0)));
        let bottom = Baseline::Circle {
            radius_mm: 30.0,
            clockwise: false,
        };
        assert!(close(bottom.frame(0.0, 0.0, 1.0).0, Point::new(0.0, 30.0)));
        let ring = top.polyline(0.0, 2.0 * std::f64::consts::PI * 30.0, 0.0, 1.0);
        assert!(ring.iter().all(|p| (p.length() - 30.0).abs() < 1e-6));
    }

    #[test]
    fn paths_and_their_ends() {
        // An L: 10 mm to the right, then 10 mm down.
        let b = Baseline::Path {
            path: Path::new()
                .move_to(Point::new(0.0, 0.0))
                .line_to(Point::new(10.0, 0.0))
                .line_to(Point::new(10.0, 10.0)),
        };
        assert_eq!(b.path_length(), Some(20.0));
        let (p, deg) = b.frame(5.0, 0.0, 1.0);
        assert!(close(p, Point::new(5.0, 0.0)) && deg.abs() < 1e-9);
        let (p, deg) = b.frame(15.0, 2.0, 1.0);
        // Down from a line that goes down is to the left.
        assert!(close(p, Point::new(8.0, 5.0)), "{p:?}");
        assert!((deg - 90.0).abs() < 1e-9);
        // Past the ends: straight on.
        assert!(close(b.frame(-3.0, 0.0, 1.0).0, Point::new(-3.0, 0.0)));
        assert!(close(b.frame(25.0, 0.0, 1.0).0, Point::new(10.0, 15.0)));
        // At the corner, the letter takes the chord direction.
        assert!((b.frame(10.0, 0.0, 2.0).1 - 45.0).abs() < 1e-9);
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(serde_json::from_str::<Baseline>(&json).unwrap(), b);
    }
}
