//! Patterned stitching (P5 Milestone 5): program-split patterns and
//! carving stamps.
//!
//! - A program-split pattern is a tile of rows: for each row, where the
//!   needle goes in, as fractions of the tile width. Fill rows use the tile
//!   rows in turn, so the needle points make a picture in the fill
//!   (bricks, diamonds, waves, ...).
//! - A carving stamp is a set of lines in a unit box (from -0.5 to 0.5).
//!   Pressed into a fill, the rows get needle points where they cross the
//!   lines, so the lines show in the fill like a carving.
//!
//! The built-in patterns and stamps are our own designs.

use emb_geom::Point;
use emb_geom::math::{cos, sin};
use serde::{Deserialize, Serialize};

/// A program-split pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitPattern {
    pub name: String,
    /// The width of the tile at scale 1, in mm.
    pub width_mm: f64,
    /// For each row of the tile, the needle points as fractions (0 to 1) of
    /// the tile width.
    pub rows: Vec<Vec<f64>>,
}

impl Default for SplitPattern {
    fn default() -> Self {
        split_patterns().remove(0)
    }
}

impl SplitPattern {
    pub fn builtin(name: &str) -> Option<SplitPattern> {
        split_patterns().into_iter().find(|p| p.name == name)
    }
}

const TAU: f64 = std::f64::consts::TAU;

fn pattern(name: &str, width_mm: f64, rows: usize, f: impl Fn(usize, f64) -> Vec<f64>) -> SplitPattern {
    SplitPattern {
        name: name.to_string(),
        width_mm,
        rows: (0..rows)
            .map(|r| {
                let mut v: Vec<f64> = f(r, r as f64 / rows as f64)
                    .into_iter()
                    .map(|x| x.rem_euclid(1.0))
                    .collect();
                v.sort_by(f64::total_cmp);
                v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
                v
            })
            .collect(),
    }
}

/// Points every `1 / n` of the tile (a row of short stitches: a line).
fn dense(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64).collect()
}

/// The built-in program-split patterns, in menu order.
pub fn split_patterns() -> Vec<SplitPattern> {
    vec![
        pattern("Brick", 4.0, 16, |r, _| match r {
            0 | 8 => dense(8),
            r if r < 8 => vec![0.0],
            _ => vec![0.5],
        }),
        pattern("Diamond", 4.0, 20, |_, t| {
            let d = 0.5 * (1.0 - (2.0 * t - 1.0).abs());
            vec![0.5 - d, 0.5 + d]
        }),
        pattern("Wave", 3.0, 24, |_, t| vec![0.5 + 0.35 * sin(TAU * t)]),
        pattern("Zigzag", 3.0, 16, |_, t| vec![1.0 - (2.0 * t - 1.0).abs()]),
        pattern("Herringbone", 4.0, 16, |_, t| vec![0.5 * t, 1.0 - 0.5 * t]),
        pattern("Checker", 4.0, 20, |r, _| {
            let first: Vec<f64> = (0..4).map(|i| f64::from(i) * 0.125).collect();
            if r < 10 {
                let mut v = first;
                v.push(0.5);
                v
            } else {
                let mut v: Vec<f64> = first.into_iter().map(|x| x + 0.5).collect();
                v.push(0.0);
                v
            }
        }),
        pattern("Scales", 4.0, 16, |_, t| {
            let half = 0.5 * (1.0 - t * t).sqrt();
            vec![0.5 - half, 0.5 + half]
        }),
        pattern(
            "Basket",
            4.0,
            16,
            |r, _| if r < 8 { vec![0.0, 0.5] } else { vec![0.25, 0.75] },
        ),
        pattern("Ripple", 6.0, 30, |_, t| {
            let x = 0.5 + 0.2 * sin(TAU * t);
            vec![x, x + 0.5]
        }),
        pattern("Grid", 3.0, 15, |r, _| if r == 0 { dense(6) } else { vec![0.0] }),
    ]
}

/// A carving stamp: lines in a unit box, from -0.5 to 0.5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stamp {
    pub name: String,
    pub lines: Vec<Vec<Point>>,
}

impl Stamp {
    pub fn builtin(name: &str) -> Option<Stamp> {
        stamps().into_iter().find(|s| s.name == name)
    }
}

/// `n` points of a closed curve `f(t)` for t from 0 to 1, the first again at the end.
fn closed(n: usize, f: impl Fn(f64) -> (f64, f64)) -> Vec<Point> {
    (0..=n)
        .map(|i| {
            let (x, y) = f(i as f64 / n as f64);
            Point::new(x, y)
        })
        .collect()
}

/// The built-in carving stamps, in menu order.
pub fn stamps() -> Vec<Stamp> {
    let stamp = |name: &str, lines: Vec<Vec<Point>>| Stamp {
        name: name.to_string(),
        lines,
    };
    let star: Vec<Point> = (0..=10)
        .map(|k| {
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * f64::from(k) / 5.0;
            let r = if k % 2 == 0 { 0.5 } else { 0.2 };
            Point::new(r * cos(a), r * sin(a))
        })
        .collect();
    vec![
        stamp("Circle", vec![closed(48, |t| (0.5 * cos(TAU * t), 0.5 * sin(TAU * t)))]),
        stamp("Star", vec![star]),
        stamp(
            "Heart",
            vec![closed(48, |t| {
                let a = TAU * t;
                let x = 16.0 * sin(a) * sin(a) * sin(a);
                let y = 13.0 * cos(a) - 5.0 * cos(2.0 * a) - 2.0 * cos(3.0 * a) - cos(4.0 * a);
                (x / 34.0, -y / 34.0 - 0.05)
            })],
        ),
        stamp(
            "Leaf",
            vec![
                closed(40, |t| {
                    let a = TAU * t;
                    (0.5 * cos(a), 0.22 * sin(a) * (1.0 + 0.3 * cos(a)))
                }),
                vec![Point::new(-0.5, 0.0), Point::new(0.5, 0.0)],
            ],
        ),
        stamp(
            "Swirl",
            vec![
                (0..=60)
                    .map(|i| {
                        let t = f64::from(i) / 60.0;
                        let (a, r) = (TAU * 2.5 * t, 0.5 * t);
                        Point::new(r * cos(a), r * sin(a))
                    })
                    .collect(),
            ],
        ),
        stamp(
            "Flower",
            vec![closed(80, |t| {
                let a = TAU * t;
                let r = 0.3 + 0.2 * cos(5.0 * a);
                (r * cos(a), r * sin(a))
            })],
        ),
        stamp(
            "Diamond",
            vec![vec![
                Point::new(0.0, -0.5),
                Point::new(0.35, 0.0),
                Point::new(0.0, 0.5),
                Point::new(-0.35, 0.0),
                Point::new(0.0, -0.5),
            ]],
        ),
        stamp(
            "Waves",
            (0..3)
                .map(|k| {
                    let y0 = -0.3 + 0.3 * f64::from(k);
                    (0..=24)
                        .map(|i| {
                            let x = -0.5 + f64::from(i) / 24.0;
                            Point::new(x, y0 + 0.08 * sin(TAU * 2.0 * (x + 0.5)))
                        })
                        .collect()
                })
                .collect(),
        ),
    ]
}

/// A carving stamp on a fill: its middle, size and turn, in design coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StampPlacement {
    pub stamp: Stamp,
    pub center: Point,
    pub size_mm: f64,
    #[serde(default)]
    pub angle_deg: f64,
}

impl StampPlacement {
    /// The lines of the stamp in design coordinates.
    pub fn lines(&self) -> Vec<Vec<Point>> {
        let (s, c) = emb_geom::math::sin_cos_deg(self.angle_deg);
        self.stamp
            .lines
            .iter()
            .map(|l| {
                l.iter()
                    .map(|p| {
                        let (x, y) = (p.x * self.size_mm, p.y * self.size_mm);
                        self.center + Point::new(x * c - y * s, x * s + y * c)
                    })
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_patterns_and_stamps_are_well_formed() {
        let pats = split_patterns();
        assert!(pats.len() >= 10);
        for p in &pats {
            assert!(p.width_mm > 0.0 && p.rows.len() >= 10, "{}", p.name);
            for r in &p.rows {
                assert!(!r.is_empty(), "{}", p.name);
                assert!(r.iter().all(|x| (0.0..1.0).contains(x)), "{}", p.name);
                assert!(r.windows(2).all(|w| w[0] < w[1]), "{}", p.name);
            }
        }
        let st = stamps();
        assert!(st.len() >= 8);
        for s in &st {
            assert!(!s.lines.is_empty());
            for l in &s.lines {
                assert!(l.len() >= 2);
                assert!(
                    l.iter().all(|p| p.x.abs() <= 0.5 + 1e-9 && p.y.abs() <= 0.55),
                    "{}",
                    s.name
                );
            }
        }
    }

    #[test]
    fn a_stamp_placement_scales_and_turns() {
        let p = StampPlacement {
            stamp: Stamp::builtin("Diamond").unwrap(),
            center: Point::new(10.0, 10.0),
            size_mm: 20.0,
            angle_deg: 90.0,
        };
        let l = &p.lines()[0];
        // Turned 90° clockwise, the top point (0, -0.5) goes to the right: (10 + 10, 10).
        assert!(l[0].distance(Point::new(20.0, 10.0)) < 1e-9, "{:?}", l[0]);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<StampPlacement>(&json).unwrap(), p);
    }
}
