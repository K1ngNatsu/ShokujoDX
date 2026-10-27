//! Decoration (P5 Milestone 7): color blends and multi blends, buttonholes,
//! and decorative borders (frames).
//!
//! - A blend is fill or satin layers with the same shape, one for each
//!   color. The density of each layer changes from one side to the other
//!   (accordion spacing), so the colors mix. A two-color blend has one
//!   layer from dense to open and one from open to dense. A multi blend
//!   keeps its layers in one group; `BlendRecord` marks the group.
//! - A buttonhole is a running box (it holds the fabric), two narrow satin
//!   legs with a gap for the cut between them, and a bar tack across each
//!   end. A buttonhole string is buttonholes spaced evenly along a line.
//! - A frame is a decorative border round a box: a shape (rectangle,
//!   rounded rectangle, oval, circle) and a stitch (satin, runs, a column
//!   type, a motif). The built-in frames are our own designs.

use emb_geom::{BBox, Path, Point, Transform};
use serde::{Deserialize, Serialize};

use crate::motif::{Motif, MotifRun};
use crate::object::{
    Accordion, ColumnPattern, LineBorder, ObjectKind, RunningParams, SatinGeometry, SatinParams, Underlay,
};

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

/// The settings of a buttonhole, in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonholeSpec {
    /// The length of the opening (the cut), between the bar tacks.
    pub length_mm: f64,
    /// The width of each satin leg.
    pub leg_mm: f64,
    /// The gap between the legs, where the opening is cut.
    pub gap_mm: f64,
    /// The length of each bar tack along the buttonhole.
    pub bartack_mm: f64,
    /// Satin spacing of the legs and bar tacks.
    pub spacing_mm: f64,
}

impl Default for ButtonholeSpec {
    fn default() -> Self {
        ButtonholeSpec {
            length_mm: 18.0,
            leg_mm: 1.8,
            gap_mm: 0.6,
            bartack_mm: 1.5,
            spacing_mm: 0.3,
        }
    }
}

/// A straight centerline from `a` to `b`.
fn segment(a: Point, b: Point) -> Path {
    Path::new().move_to(a).line_to(b)
}

/// The objects of one buttonhole with its middle at `center`, its length
/// along `angle_deg` (0°: along +X), in sew order: a running box, the
/// first leg, a bar tack, the second leg, a bar tack.
pub fn buttonhole(spec: &ButtonholeSpec, center: Point, angle_deg: f64) -> Vec<ObjectKind> {
    let half_len = spec.length_mm.max(1.0) / 2.0;
    let leg = spec.leg_mm.max(0.5);
    let off = spec.gap_mm.max(0.0) / 2.0 + leg / 2.0;
    let width = 2.0 * leg + spec.gap_mm.max(0.0);
    let t = Transform::rotate_deg(angle_deg).then(Transform::translate(center.x, center.y));
    let at = |x: f64, y: f64| t.apply(Point::new(x, y));
    let satin = |center: Path, w: f64| ObjectKind::Satin {
        geometry: SatinGeometry::from_centerline(&center, w),
        params: SatinParams {
            spacing_mm: spec.spacing_mm,
            pull_compensation_mm: 0.0,
            short_stitches: false,
            underlay: Underlay::None,
            ..SatinParams::default()
        },
    };
    let bar = spec.bartack_mm.max(0.5);
    let end = half_len + bar / 2.0;
    let edge = width / 2.0;
    vec![
        // A running box round the buttonhole holds the fabric.
        ObjectKind::Running {
            path: Path::polygon(&[
                at(-end - bar / 2.0, -edge),
                at(end + bar / 2.0, -edge),
                at(end + bar / 2.0, edge),
                at(-end - bar / 2.0, edge),
            ]),
            params: RunningParams {
                stitch_length_mm: 1.5,
                ..RunningParams::default()
            },
        },
        satin(segment(at(-half_len, -off), at(half_len, -off)), leg),
        satin(segment(at(end, -edge), at(end, edge)), bar),
        satin(segment(at(half_len, off), at(-half_len, off)), leg),
        satin(segment(at(-end, edge), at(-end, -edge)), bar),
    ]
}

/// The middles and the angles of `count` buttonholes spaced evenly along
/// `line` (the first and the last at its ends). With `across`, each
/// buttonhole is across the line; else along it.
pub fn string_positions(line: &[Point], count: usize, across: bool) -> Vec<(Point, f64)> {
    let mut at = vec![0.0];
    for w in line.windows(2) {
        at.push(at[at.len() - 1] + w[0].distance(w[1]));
    }
    let total = at[at.len() - 1];
    if line.len() < 2 || count == 0 || total <= 0.0 {
        return Vec::new();
    }
    (0..count)
        .map(|k| {
            let s = if count == 1 {
                total / 2.0
            } else {
                total * k as f64 / (count - 1) as f64
            };
            let i = at.partition_point(|&l| l <= s).clamp(1, line.len() - 1);
            let seg = at[i] - at[i - 1];
            let f = if seg > 0.0 { (s - at[i - 1]) / seg } else { 0.0 };
            let p = line[i - 1].lerp(line[i], f);
            let d = line[i] - line[i - 1];
            let a = emb_geom::math::to_degrees(emb_geom::math::atan2(d.y, d.x));
            (p, if across { a + 90.0 } else { a })
        })
        .collect()
}

/// The shape of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameShape {
    Rectangle,
    Rounded,
    Oval,
    Circle,
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
    fn a_buttonhole_has_a_box_two_legs_and_two_bar_tacks() {
        let spec = ButtonholeSpec::default();
        let objs = buttonhole(&spec, Point::new(10.0, 10.0), 90.0);
        assert_eq!(objs.len(), 5);
        assert!(matches!(objs[0], ObjectKind::Running { .. }));
        let b = objs
            .iter()
            .filter_map(|k| k.control_bbox())
            .reduce(BBox::union)
            .unwrap();
        // Turned 90°: the length is along Y. Opening 18 mm + two bar tacks.
        assert!((b.height() - (18.0 + 2.0 * 1.5)).abs() < 0.01, "{b:?}");
        assert!((b.width() - (2.0 * 1.8 + 0.6)).abs() < 0.01, "{b:?}");
        assert!((b.center().distance(Point::new(10.0, 10.0))) < 1e-9);
        // The legs leave the gap free for the cut.
        let ObjectKind::Satin { geometry, .. } = &objs[1] else {
            panic!()
        };
        let xs: Vec<f64> = geometry
            .rail_a
            .points()
            .chain(geometry.rail_b.points())
            .map(|p| p.x)
            .collect();
        assert!(xs.iter().all(|x| (x - 10.0).abs() >= 0.3 - 1e-9));
    }

    #[test]
    fn a_string_spaces_buttonholes_evenly() {
        let line = [Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let p = string_positions(&line, 5, true);
        assert_eq!(p.len(), 5);
        assert_eq!(p[0].0, Point::new(0.0, 0.0));
        assert_eq!(p[4].0, Point::new(0.0, 100.0));
        assert!((p[1].0.y - 25.0).abs() < 1e-9);
        assert!((p[0].1 - 180.0).abs() < 1e-9, "across a line going down: {}", p[0].1);
        assert_eq!(string_positions(&line, 1, false)[0].0, Point::new(0.0, 50.0));
    }

    #[test]
    fn frames_go_round_the_box() {
        let b = BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 20.0));
        let all = frames();
        assert!(all.len() >= 12);
        for f in &all {
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
    }
}
