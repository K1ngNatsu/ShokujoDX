//! Motifs (P5 Milestone 4): small stitch patterns that repeat along a line
//! (motif run), in rows over a shape (motif fill) or along a column (motif
//! column).
//!
//! A motif is one line of needle points from its first reference point
//! (0, 0) to its second reference point (1, 0): the length of one repeat
//! is 1. Y is across the line (on screen, down). The engine scales the
//! length and the height of the motif to the sizes of the object, so the
//! points can have any height.
//!
//! The motifs of an object are kept in the object, so a design opens with
//! no motif library. The built-in motifs are our own designs.

use emb_geom::Point;
use emb_geom::math::{cos, sin};
use serde::{Deserialize, Serialize};

/// A motif: a name and its needle points from (0, 0) to (1, 0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Motif {
    pub name: String,
    pub points: Vec<Point>,
}

/// Motif files have this extension (JSON of one [`Motif`]).
pub const FILE_EXTENSION: &str = "embmotif";

impl Motif {
    /// The height of the motif across its line, in motif units (at least a
    /// small value, so a flat motif can scale).
    pub fn height(&self) -> f64 {
        let (lo, hi) = self
            .points
            .iter()
            .fold((0.0_f64, 0.0_f64), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
        (hi - lo).max(0.05)
    }

    /// The motif from needle points in design mm (for example the stitches
    /// of selected objects): the left middle and the right middle of their
    /// box become the reference points, so the motif repeats to the right.
    /// `None` if the points have no width.
    pub fn from_points(name: &str, pts: &[Point]) -> Option<Motif> {
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for p in pts {
            x0 = x0.min(p.x);
            x1 = x1.max(p.x);
            y0 = y0.min(p.y);
            y1 = y1.max(p.y);
        }
        let w = x1 - x0;
        if w.is_nan() || w <= 1e-6 {
            return None;
        }
        let mid = (y0 + y1) / 2.0;
        let unit = |p: &Point| Point::new((p.x - x0) / w, (p.y - mid) / w);
        let mut points = vec![Point::new(0.0, 0.0)];
        points.extend(pts.iter().map(unit));
        points.push(Point::new(1.0, 0.0));
        points.dedup();
        Some(Motif {
            name: name.to_string(),
            points,
        })
    }

    /// The built-in motif with this name.
    pub fn builtin(name: &str) -> Option<Motif> {
        builtin().into_iter().find(|m| m.name == name)
    }
}

impl Default for Motif {
    fn default() -> Self {
        builtin().remove(0)
    }
}

fn motif(name: &str, points: Vec<(f64, f64)>) -> Motif {
    Motif {
        name: name.to_string(),
        points: points.into_iter().map(|(x, y)| Point::new(x, y)).collect(),
    }
}

/// `n + 1` points of the curve `f(t)` for t from 0 to 1.
fn curve(n: usize, f: impl Fn(f64) -> (f64, f64)) -> Vec<(f64, f64)> {
    (0..=n).map(|i| f(i as f64 / n as f64)).collect()
}

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;

/// The built-in motifs, in menu order.
pub fn builtin() -> Vec<Motif> {
    let mut v = vec![
        motif("Peak", vec![(0.0, 0.0), (0.5, -0.5), (1.0, 0.0)]),
        motif(
            "Double peak",
            vec![(0.0, 0.0), (0.25, -0.4), (0.5, 0.0), (0.75, -0.4), (1.0, 0.0)],
        ),
        motif("Wave", curve(12, |t| (t, -0.35 * sin(TAU * t)))),
        motif("Scallop", curve(10, |t| (0.5 - 0.5 * cos(PI * t), -0.5 * sin(PI * t)))),
        motif(
            "Loop",
            curve(16, |t| (t - 0.3 * sin(TAU * t), -0.3 * (1.0 - cos(TAU * t)))),
        ),
        motif(
            "Crenel",
            vec![(0.0, 0.0), (0.0, -0.4), (0.5, -0.4), (0.5, 0.0), (1.0, 0.0)],
        ),
        motif(
            "Chevron",
            vec![
                (0.0, 0.0),
                (0.6, 0.0),
                (0.35, -0.3),
                (0.6, 0.0),
                (0.35, 0.3),
                (0.6, 0.0),
                (1.0, 0.0),
            ],
        ),
        motif(
            "Diamond",
            vec![
                (0.0, 0.0),
                (0.5, -0.35),
                (1.0, 0.0),
                (0.5, 0.35),
                (0.0, 0.0),
                (1.0, 0.0),
            ],
        ),
        motif(
            "Triangle",
            vec![(0.0, 0.0), (1.0, 0.0), (0.5, -0.45), (0.0, 0.0), (1.0, 0.0)],
        ),
    ];
    // A leaf: the upper edge, the lower edge back, and the middle vein.
    let mut leaf = curve(10, |t| (t, -0.3 * sin(PI * t)));
    leaf.extend(curve(10, |t| (1.0 - t, 0.3 * sin(PI * t))).into_iter().skip(1));
    leaf.push((1.0, 0.0));
    v.push(motif("Leaf", leaf));
    // A circle on the line.
    let mut circle = vec![(0.0, 0.0)];
    circle.extend(curve(16, |t| (0.5 - 0.3 * sin(TAU * t), -0.3 + 0.3 * cos(TAU * t))));
    circle.push((1.0, 0.0));
    v.push(motif("Circle", circle));
    // A bead: an oval on the line.
    let mut bead = vec![(0.0, 0.0)];
    bead.extend(curve(16, |t| (0.5 - 0.3 * cos(TAU * t), -0.2 * sin(TAU * t))));
    bead.push((1.0, 0.0));
    v.push(motif("Bead", bead));
    // A heart with its point on the line.
    let mut heart = vec![(0.0, 0.0)];
    heart.extend(curve(24, |t| {
        let a = TAU * t;
        let x = 16.0 * sin(a) * sin(a) * sin(a);
        let y = 13.0 * cos(a) - 5.0 * cos(2.0 * a) - 2.0 * cos(3.0 * a) - cos(4.0 * a);
        // The point of the heart (t = 0.5, y = -17) is at (0.5, 0).
        (0.5 + x / 40.0, -(y + 17.0) / 40.0)
    }));
    heart.push((1.0, 0.0));
    v.push(motif("Heart", heart));
    // A five-point star above the line.
    let mut star = vec![(0.0, 0.0)];
    for k in 0..=10 {
        let a = PI / 2.0 + PI * f64::from(k) / 5.0;
        let r = if k % 2 == 0 { 0.35 } else { 0.14 };
        star.push((0.5 + r * cos(a), -0.35 + r * sin(a)));
    }
    star.push((1.0, 0.0));
    v.push(motif("Star", star));
    // A flower: a five-petal rose curve round a center above the line.
    let mut flower = vec![(0.0, 0.0), (0.5, -0.3)];
    flower.extend(curve(40, |t| {
        let a = PI * t;
        let r = 0.25 * sin(5.0 * a);
        (0.5 + r * cos(a), -0.3 + r * sin(a))
    }));
    flower.push((1.0, 0.0));
    v.push(motif("Flower", flower));
    for m in &mut v {
        m.points.dedup();
    }
    v
}

/// A motif run: the motif repeats along a line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotifRun {
    pub motif: Motif,
    /// The length of one repeat along the line, in mm.
    pub length_mm: f64,
    /// The height of the motif across the line, in mm.
    pub height_mm: f64,
    /// A straight run between repeats, in mm.
    #[serde(default)]
    pub gap_mm: f64,
}

impl MotifRun {
    pub fn new(motif: Motif) -> MotifRun {
        MotifRun {
            motif,
            length_mm: 5.0,
            height_mm: 3.0,
            gap_mm: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_motifs_go_from_the_first_to_the_second_reference_point() {
        let all = builtin();
        assert!(all.len() >= 15);
        let mut names: Vec<&str> = all.iter().map(|m| m.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len(), "names are unique");
        for m in &all {
            let (first, last) = (m.points[0], m.points[m.points.len() - 1]);
            assert!(first.distance(Point::ZERO) < 1e-9, "{}", m.name);
            assert!(last.distance(Point::new(1.0, 0.0)) < 1e-9, "{}", m.name);
            assert!(m.height() > 0.1, "{}", m.name);
            assert!(m.points.iter().all(|p| p.x > -0.1 && p.x < 1.1), "{}", m.name);
        }
    }

    #[test]
    fn a_motif_from_points_and_back_from_json() {
        let pts = [Point::new(10.0, 5.0), Point::new(15.0, 0.0), Point::new(20.0, 5.0)];
        let m = Motif::from_points("Mine", &pts).unwrap();
        assert_eq!(m.points[0], Point::ZERO);
        assert_eq!(m.points[m.points.len() - 1], Point::new(1.0, 0.0));
        // The middle point is at half the length, half the height up.
        assert!(m.points.contains(&Point::new(0.5, -0.25)));
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Motif>(&json).unwrap(), m);
        assert!(Motif::from_points("Flat", &[Point::new(1.0, 1.0), Point::new(1.0, 5.0)]).is_none());
    }
}
