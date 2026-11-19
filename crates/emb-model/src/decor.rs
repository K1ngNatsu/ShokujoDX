//! Decoration (P5 Milestone 7): color blends and multi blends, and
//! decorative borders (frames).
//!
//! - A blend is fill or satin layers with the same shape, one for each
//!   color. The density of each layer changes from one side to the other
//!   (accordion spacing), so the colors mix. A two-color blend has one
//!   layer from dense to open and one from open to dense. A multi blend
//!   keeps its layers in one group; `BlendRecord` marks the group.
//! - Buttonholes are in [`crate::buttonhole`].
//! - A frame is a decorative border round a box: a shape (rectangle,
//!   rounded rectangle, oval, circle) and a stitch (satin, runs, a column
//!   type, a motif). The built-in frames are our own designs.

use emb_geom::{BBox, Path, Point};
use serde::{Deserialize, Serialize};

use crate::motif::{Motif, MotifRun};
use crate::object::{Accordion, ColumnPattern, LineBorder, ObjectKind, RunningParams};

/// A multi blend: the fill or satin objects of `group` are its layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlendRecord {
    pub group: u32,
}

/// The least density of a blend layer, in percent.
pub const MIN_DENSITY_PCT: f64 = 5.0;

/// Points of a blend profile: the density changes evenly between them.
const BLEND_STEPS: usize = 20;

/// The accordion profile of a blend layer whose density goes evenly from
/// `start` to `end` percent of the set density. (The spacing factor is
/// 100 / density, so the profile needs points along the way: a straight
/// line of spacing would leave the middle too open.)
pub fn blend_profile(start: f64, end: f64) -> Accordion {
    let f = |pct: f64| 100.0 / pct.clamp(MIN_DENSITY_PCT, 100.0);
    Accordion {
        points: (0..=BLEND_STEPS)
            .map(|i| {
                let t = i as f64 / BLEND_STEPS as f64;
                (t, f(start + (end - start) * t))
            })
            .collect(),
    }
}

/// The start and end density of a blend layer, in percent (100 with no profile).
pub fn blend_densities(a: Option<&Accordion>) -> (f64, f64) {
    match a {
        Some(a) => (100.0 / a.factor(0.0), 100.0 / a.factor(1.0)),
        None => (100.0, 100.0),
    }
}

/// The shape of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameShape {
    Rectangle,
    Rounded,
    Oval,
    Circle,
    /// A diamond: its four corners are on the middle lines of the box.
    Diamond,
    /// A hexagon with a point on the left and on the right.
    Hexagon,
    /// A rectangle with its corners cut.
    Octagon,
}

impl FrameShape {
    pub const ALL: [FrameShape; 7] = [
        FrameShape::Rectangle,
        FrameShape::Rounded,
        FrameShape::Oval,
        FrameShape::Circle,
        FrameShape::Diamond,
        FrameShape::Hexagon,
        FrameShape::Octagon,
    ];

    /// The name of the shape (English; shown translated).
    pub fn name(self) -> &'static str {
        match self {
            FrameShape::Rectangle => "Rectangle",
            FrameShape::Rounded => "Rounded rectangle",
            FrameShape::Oval => "Oval",
            FrameShape::Circle => "Circle",
            FrameShape::Diamond => "Diamond",
            FrameShape::Hexagon => "Hexagon",
            FrameShape::Octagon => "Octagon",
        }
    }
}

/// The stitch of a frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameStitch {
    /// A column of `width_mm` of this column type (satin, E stitch, ...).
    Column { pattern: ColumnPattern, width_mm: f64 },
    /// A running line; `repeats` 3 is a triple run.
    Run { repeats: u32 },
    /// Two running lines `gap_mm` apart.
    Double { gap_mm: f64 },
    /// A motif run with this built-in motif.
    Motif {
        motif: String,
        length_mm: f64,
        height_mm: f64,
    },
}

/// A decorative border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub name: String,
    pub shape: FrameShape,
    pub stitch: FrameStitch,
}

impl Frame {
    /// The border with another shape (`None`: its own) and with its stitch
    /// `scale` times as large (the width of a column, the gap of two
    /// lines, the length and the height of a motif). One border and these
    /// two parameters give a family of borders (P17 Milestone 8, ADR 0118).
    pub fn with(&self, shape: Option<FrameShape>, scale: f64) -> Frame {
        let s = if scale.is_finite() { scale.clamp(0.25, 4.0) } else { 1.0 };
        let stitch = match &self.stitch {
            FrameStitch::Column { pattern, width_mm } => FrameStitch::Column {
                pattern: *pattern,
                width_mm: width_mm * s,
            },
            FrameStitch::Run { repeats } => FrameStitch::Run { repeats: *repeats },
            FrameStitch::Double { gap_mm } => FrameStitch::Double { gap_mm: gap_mm * s },
            FrameStitch::Motif {
                motif,
                length_mm,
                height_mm,
            } => FrameStitch::Motif {
                motif: motif.clone(),
                length_mm: length_mm * s,
                height_mm: height_mm * s,
            },
        };
        Frame {
            name: self.name.clone(),
            shape: shape.unwrap_or(self.shape),
            stitch,
        }
    }
}

/// The built-in frames, in menu order.
pub fn frames() -> Vec<Frame> {
    let f = |name: &str, shape, stitch| Frame {
        name: name.to_string(),
        shape,
        stitch,
    };
    let col = |pattern, width_mm| FrameStitch::Column { pattern, width_mm };
    let motif = |name: &str, length_mm, height_mm| FrameStitch::Motif {
        motif: name.to_string(),
        length_mm,
        height_mm,
    };
    vec![
        f("Satin box", FrameShape::Rectangle, col(ColumnPattern::Satin, 3.0)),
        f("Satin rounded", FrameShape::Rounded, col(ColumnPattern::Satin, 3.0)),
        f("Satin oval", FrameShape::Oval, col(ColumnPattern::Satin, 3.0)),
        f(
            "Raised circle",
            FrameShape::Circle,
            col(ColumnPattern::Raised { layers: 2 }, 3.5),
        ),
        f("Double line", FrameShape::Rounded, FrameStitch::Double { gap_mm: 2.0 }),
        f("Triple run circle", FrameShape::Circle, FrameStitch::Run { repeats: 3 }),
        f("Blanket", FrameShape::Rounded, col(ColumnPattern::EStitch, 3.0)),
        f(
            "Rope",
            FrameShape::Oval,
            col(ColumnPattern::String { chord_mm: 4.0 }, 3.0),
        ),
        f("Scallops", FrameShape::Oval, motif("Scallop", 6.0, 3.0)),
        f("Hearts", FrameShape::Rounded, motif("Heart", 7.0, 5.0)),
        f("Leaves", FrameShape::Oval, motif("Leaf", 8.0, 3.0)),
        f("Stars", FrameShape::Circle, motif("Star", 7.0, 5.0)),
        // P17 Milestone 8 (ADR 0118): more shapes, and the motifs of the
        // families.
        f("Satin circle", FrameShape::Circle, col(ColumnPattern::Satin, 3.0)),
        f("Satin diamond", FrameShape::Diamond, col(ColumnPattern::Satin, 3.0)),
        f("Satin hexagon", FrameShape::Hexagon, col(ColumnPattern::Satin, 3.0)),
        f("Satin octagon", FrameShape::Octagon, col(ColumnPattern::Satin, 3.0)),
        f("Wide satin box", FrameShape::Rectangle, col(ColumnPattern::Satin, 5.0)),
        f("Fine satin oval", FrameShape::Oval, col(ColumnPattern::Satin, 2.0)),
        f(
            "Raised oval",
            FrameShape::Oval,
            col(ColumnPattern::Raised { layers: 2 }, 3.5),
        ),
        f("Blanket circle", FrameShape::Circle, col(ColumnPattern::EStitch, 3.0)),
        f("Blanket box", FrameShape::Rectangle, col(ColumnPattern::EStitch, 3.0)),
        f(
            "Rope circle",
            FrameShape::Circle,
            col(ColumnPattern::String { chord_mm: 4.0 }, 3.0),
        ),
        f(
            "Rope box",
            FrameShape::Rounded,
            col(ColumnPattern::String { chord_mm: 4.0 }, 3.0),
        ),
        f("Triple run box", FrameShape::Rectangle, FrameStitch::Run { repeats: 3 }),
        f("Triple run oval", FrameShape::Oval, FrameStitch::Run { repeats: 3 }),
        f(
            "Double line box",
            FrameShape::Rectangle,
            FrameStitch::Double { gap_mm: 2.0 },
        ),
        f(
            "Double line circle",
            FrameShape::Circle,
            FrameStitch::Double { gap_mm: 2.5 },
        ),
        f(
            "Double line diamond",
            FrameShape::Diamond,
            FrameStitch::Double { gap_mm: 2.0 },
        ),
        f("Waves", FrameShape::Oval, motif("Wave", 6.0, 2.5)),
        f("Zigzag", FrameShape::Rectangle, motif("Zigzag", 6.0, 3.0)),
        f("Beads", FrameShape::Circle, motif("Bead", 5.0, 2.0)),
        f("Pearls", FrameShape::Oval, motif("Pearl", 4.0, 2.0)),
        f("Diamonds", FrameShape::Rounded, motif("Diamond", 6.0, 3.0)),
        f("Greek key", FrameShape::Rectangle, motif("Key", 6.0, 3.0)),
        f("Arcade", FrameShape::Rounded, motif("Arcade", 8.0, 3.0)),
        f("Loops", FrameShape::Oval, motif("Loop", 6.0, 3.0)),
        f("Flowers", FrameShape::Oval, motif("Flower", 9.0, 5.0)),
        f("Daisies", FrameShape::Circle, motif("Daisy", 9.0, 5.0)),
        f("Vine", FrameShape::Oval, motif("Vine", 10.0, 4.0)),
        f("Feather", FrameShape::Rounded, motif("Feather", 9.0, 3.0)),
        f("Crosses", FrameShape::Rectangle, motif("Cross stitch", 6.0, 3.0)),
        f("Battlement", FrameShape::Rectangle, motif("Battlement", 8.0, 3.0)),
        f("Small stars", FrameShape::Rounded, motif("Small star", 7.0, 4.0)),
        f("Shells", FrameShape::Circle, motif("Shell", 8.0, 4.0)),
        f("Stairs", FrameShape::Octagon, motif("Stairs", 8.0, 3.0)),
        f("Spirals", FrameShape::Oval, motif("Curl", 7.0, 4.0)),
        f("Squares", FrameShape::Rectangle, motif("Square", 6.0, 3.0)),
        f("Ladder", FrameShape::Rounded, motif("Ladder", 8.0, 3.0)),
        f("Snowflakes", FrameShape::Circle, motif("Snowflake", 8.0, 5.0)),
        f("Hexagon chain", FrameShape::Hexagon, motif("Hexagon", 7.0, 4.0)),
    ]
}

/// The outline of `shape` round `b` with `margin` mm space.
fn outline(shape: FrameShape, b: &BBox, margin: f64) -> Path {
    let c = b.center();
    let (w, h) = (b.width() + 2.0 * margin, b.height() + 2.0 * margin);
    match shape {
        FrameShape::Rectangle => Path::rect(BBox::from_center_size(c, w, h)),
        FrameShape::Rounded => Path::rounded_rect(BBox::from_center_size(c, w, h), 0.15 * w.min(h)),
        // An oval through the corners of the box (with its margin).
        FrameShape::Oval => Path::ellipse(
            c,
            w / 2.0 * std::f64::consts::SQRT_2,
            h / 2.0 * std::f64::consts::SQRT_2,
        ),
        FrameShape::Circle => {
            let r = (w * w + h * h).sqrt() / 2.0;
            Path::ellipse(c, r, r)
        }
        // The corners of the box are on the sides of the diamond.
        FrameShape::Diamond => Path::polygon(&[
            Point::new(c.x, c.y - h),
            Point::new(c.x + w, c.y),
            Point::new(c.x, c.y + h),
            Point::new(c.x - w, c.y),
        ]),
        FrameShape::Hexagon => {
            let tip = w / 2.0 + 0.29 * h;
            Path::polygon(&[
                Point::new(c.x - w / 2.0, c.y - h / 2.0),
                Point::new(c.x + w / 2.0, c.y - h / 2.0),
                Point::new(c.x + tip, c.y),
                Point::new(c.x + w / 2.0, c.y + h / 2.0),
                Point::new(c.x - w / 2.0, c.y + h / 2.0),
                Point::new(c.x - tip, c.y),
            ])
        }
        FrameShape::Octagon => {
            // A larger rectangle; each cut goes through a corner of the box.
            let e = 0.15 * w.min(h);
            let (x, y) = (w / 2.0 + e, h / 2.0 + e);
            let cut = 2.0 * e;
            Path::polygon(&[
                Point::new(c.x - x + cut, c.y - y),
                Point::new(c.x + x - cut, c.y - y),
                Point::new(c.x + x, c.y - y + cut),
                Point::new(c.x + x, c.y + y - cut),
                Point::new(c.x + x - cut, c.y + y),
                Point::new(c.x - x + cut, c.y + y),
                Point::new(c.x - x, c.y + y - cut),
                Point::new(c.x - x, c.y - y + cut),
            ])
        }
    }
}

/// The objects of `frame` round `b`, `margin` mm from it.
pub fn frame_objects(frame: &Frame, b: &BBox, margin: f64) -> Vec<ObjectKind> {
    let run = |path: Path, params: RunningParams| ObjectKind::Running { path, params };
    match &frame.stitch {
        FrameStitch::Column { pattern, width_mm } => {
            // The column is outside the margin, not on the box.
            let path = outline(frame.shape, b, margin + width_mm / 2.0);
            vec![run(
                path,
                RunningParams {
                    border: Some(LineBorder::new(*width_mm, *pattern)),
                    ..RunningParams::default()
                },
            )]
        }
        FrameStitch::Run { repeats } => vec![run(
            outline(frame.shape, b, margin),
            RunningParams {
                repeats: *repeats,
                ..RunningParams::default()
            },
        )],
        FrameStitch::Double { gap_mm } => vec![
            run(outline(frame.shape, b, margin), RunningParams::default()),
            run(outline(frame.shape, b, margin + gap_mm), RunningParams::default()),
        ],
        FrameStitch::Motif {
            motif,
            length_mm,
            height_mm,
        } => {
            let m = Motif::builtin(motif).unwrap_or_default();
            vec![run(
                outline(frame.shape, b, margin + height_mm / 2.0),
                RunningParams {
                    motif: Some(MotifRun {
                        length_mm: *length_mm,
                        height_mm: *height_mm,
                        ..MotifRun::new(m)
                    }),
                    ..RunningParams::default()
                },
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_geom::Point;

    #[test]
    fn blend_profiles_round_trip_densities() {
        let a = blend_profile(100.0, 20.0);
        assert_eq!(a.factor(0.0), 1.0);
        assert_eq!(a.factor(1.0), 5.0);
        let (s, e) = blend_densities(Some(&a));
        assert!((s - 100.0).abs() < 1e-9 && (e - 20.0).abs() < 1e-9);
        // Halfway, the density is halfway: 60 %.
        assert!((100.0 / a.factor(0.5) - 60.0).abs() < 1e-9);
        assert_eq!(blend_densities(None), (100.0, 100.0));
        assert_eq!(blend_profile(100.0, 0.0).factor(1.0), 100.0 / MIN_DENSITY_PCT);
    }

    #[test]
    fn frames_go_round_the_box() {
        let b = BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 20.0));
        let all = frames();
        assert!(all.len() >= 40, "{}", all.len());
        let mut names: Vec<&str> = all.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len(), "the names are different");
        for f in &all {
            // The motif of a border is a built-in motif.
            if let FrameStitch::Motif { motif, .. } = &f.stitch {
                assert!(Motif::builtin(motif).is_some(), "{}: no motif {motif}", f.name);
            }
            let objs = frame_objects(f, &b, 3.0);
            assert!(!objs.is_empty(), "{}", f.name);
            let fb = objs
                .iter()
                .filter_map(|k| k.control_bbox())
                .reduce(BBox::union)
                .unwrap();
            assert!(fb.min.x <= -3.0 + 1e-6 && fb.max.x >= 43.0 - 1e-6, "{}: {fb:?}", f.name);
            assert!(fb.min.y <= -3.0 + 1e-6 && fb.max.y >= 23.0 - 1e-6, "{}: {fb:?}", f.name);
        }
        // Each shape goes round the box, with the corners of the box in it
        // (a rounded rectangle cuts the corners).
        for shape in FrameShape::ALL.into_iter().filter(|s| *s != FrameShape::Rounded) {
            let path = outline(shape, &b, 0.0);
            let ring: Vec<Point> = path.flatten(0.05).into_iter().flat_map(|pl| pl.points).collect();
            let inside = |p: Point| {
                let mut n = 0;
                for k in 0..ring.len() {
                    let (s, e) = (ring[k], ring[(k + 1) % ring.len()]);
                    if (s.y > p.y) != (e.y > p.y) && p.x < s.x + (p.y - s.y) * (e.x - s.x) / (e.y - s.y) {
                        n += 1;
                    }
                }
                n % 2 == 1
            };
            for corner in [
                Point::new(0.5, 0.5),
                Point::new(39.5, 0.5),
                Point::new(39.5, 19.5),
                Point::new(0.5, 19.5),
            ] {
                assert!(inside(corner), "{shape:?}: {corner:?}");
            }
        }
    }

    #[test]
    fn a_border_with_another_shape_and_size() {
        let hearts = frames().into_iter().find(|f| f.name == "Hearts").unwrap();
        let same = hearts.with(None, 1.0);
        assert_eq!(same, hearts);
        let large = hearts.with(Some(FrameShape::Diamond), 2.0);
        assert_eq!(large.shape, FrameShape::Diamond);
        let (
            FrameStitch::Motif {
                length_mm: a,
                height_mm: h,
                ..
            },
            FrameStitch::Motif {
                length_mm: b,
                height_mm: k,
                ..
            },
        ) = (&hearts.stitch, &large.stitch)
        else {
            panic!()
        };
        assert!((b - 2.0 * a).abs() < 1e-9 && (k - 2.0 * h).abs() < 1e-9);
        let box_ = BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 20.0));
        assert!(!frame_objects(&large, &box_, 3.0).is_empty());
        // A size that is not a number is the size of the border.
        assert_eq!(hearts.with(None, f64::NAN), hearts);
        let satin = frames().remove(0).with(None, 0.5);
        assert!(matches!(satin.stitch, FrameStitch::Column { width_mm, .. } if (width_mm - 1.5).abs() < 1e-9));
    }
}
