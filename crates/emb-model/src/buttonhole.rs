//! Buttonholes (P5 Milestone 7; shapes, sizes and records in ADR 0085).
//!
//! A buttonhole is plain objects (ADR 0023), so the embroidery machine sews
//! it with the design: a running box that holds the fabric, satin legs with
//! a gap for the cut between them, and the ends of the shape (bar tacks, a
//! round end, an eye or a taper). An eyelet is a satin ring round a hole.
//!
//! - [`ButtonholeShape`]: square ends, round end, round ends, keyhole,
//!   tapered keyhole, stretch and eyelet.
//! - [`METRIC_BUTTONS`], [`INCH_BUTTONS`] and the eyelet holes: standard
//!   sizes. The opening of a buttonhole is the diameter of the button plus
//!   its thickness; [`ButtonholeSpec::scaled`] sets the legs, the gap, the
//!   bar tacks and the eye for the size.
//! - [`ButtonholeRecord`]: the buttonholes that the window can change. The
//!   first object of each buttonhole is its box; [`placement_of_box`] finds
//!   the middle and the angle of a buttonhole again from its box, after the
//!   user moves, turns or mirrors it.

use std::ops::Not;

use emb_geom::{BBox, Path, Point, Transform, math};
use serde::{Deserialize, Serialize};

use crate::mitre;
use crate::object::{ObjectId, ObjectKind, RunningParams, SatinGeometry, SatinParams, Underlay};

/// The shape of a buttonhole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonholeShape {
    /// Straight legs and a bar tack across each end.
    #[default]
    Square,
    /// A round end for the shank of the button, and a bar tack at the other end.
    RoundEnd,
    /// Two round ends.
    RoundEnds,
    /// A round eye at one end for the shank of a thick button, and a bar tack at the other end.
    Keyhole,
    /// A keyhole with a tapered end in place of the bar tack.
    TaperedKeyhole,
    /// Square ends and open zigzag legs that stretch.
    Stretch,
    /// A round hole with satin round it.
    Eyelet,
}

impl ButtonholeShape {
    pub const ALL: [ButtonholeShape; 7] = [
        ButtonholeShape::Square,
        ButtonholeShape::RoundEnd,
        ButtonholeShape::RoundEnds,
        ButtonholeShape::Keyhole,
        ButtonholeShape::TaperedKeyhole,
        ButtonholeShape::Stretch,
        ButtonholeShape::Eyelet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ButtonholeShape::Square => "Square ends",
            ButtonholeShape::RoundEnd => "Round end",
            ButtonholeShape::RoundEnds => "Round ends",
            ButtonholeShape::Keyhole => "Keyhole",
            ButtonholeShape::TaperedKeyhole => "Tapered keyhole",
            ButtonholeShape::Stretch => "Stretch (for knits)",
            ButtonholeShape::Eyelet => "Eyelet",
        }
    }

    /// When to use the shape (a tooltip).
    pub fn help(self) -> &'static str {
        match self {
            ButtonholeShape::Square => "The usual buttonhole: shirts, blouses and most woven fabrics.",
            ButtonholeShape::RoundEnd => "A round end for the shank of the button: blouses, dresses and trousers.",
            ButtonholeShape::RoundEnds => "Soft round ends: light, fine and sheer fabrics.",
            ButtonholeShape::Keyhole => "An eye for a thick button with a shank: coats, jackets and heavy fabrics.",
            ButtonholeShape::TaperedKeyhole => "A keyhole with a tapered end: tailored jackets and coats.",
            ButtonholeShape::Stretch => "Open zigzag legs that stretch with the fabric: knits and jersey.",
            ButtonholeShape::Eyelet => "A round hole for a cord, a lace, a belt or a drawstring.",
        }
    }

    /// The shape has an eye (a keyhole) or is a round hole (an eyelet).
    pub fn has_eye(self) -> bool {
        matches!(
            self,
            ButtonholeShape::Keyhole | ButtonholeShape::TaperedKeyhole | ButtonholeShape::Eyelet
        )
    }

    /// The legs and the bar tacks can sew as one ring with mitre corners
    /// ([`ButtonholeSpec::mitre`]).
    pub fn can_mitre(self) -> bool {
        matches!(
            self,
            ButtonholeShape::Square | ButtonholeShape::RoundEnd | ButtonholeShape::Keyhole
        )
    }

    /// The two ends of the shape are not the same, so it can turn end for end.
    pub fn has_ends(self) -> bool {
        matches!(
            self,
            ButtonholeShape::RoundEnd | ButtonholeShape::Keyhole | ButtonholeShape::TaperedKeyhole
        )
    }
}

/// The settings of a buttonhole, in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonholeSpec {
    #[serde(default)]
    pub shape: ButtonholeShape,
    /// The length of the opening (the cut): from the bar tack to the other
    /// end of the opening (for a keyhole, to the far side of the eye).
    pub length_mm: f64,
    /// The width of each satin leg.
    pub leg_mm: f64,
    /// The gap between the legs, where the opening is cut.
    pub gap_mm: f64,
    /// The length of each bar tack along the buttonhole.
    pub bartack_mm: f64,
    /// Satin spacing of the legs and bar tacks.
    pub spacing_mm: f64,
    /// The diameter of the eye of a keyhole, or of the hole of an eyelet.
    #[serde(default = "default_eye_mm")]
    pub eye_mm: f64,
    /// A run under each leg, as a cord: the buttonhole is stronger and
    /// stands up more.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub corded: bool,
    /// The shape turned end for end: the round end, the eye or the taper is
    /// at the start of the line.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub flip: bool,
    /// The legs and the bar tacks sew as one satin ring that turns at mitre
    /// corners, not as parts that stop and start (ADR 0086). Bar tacks hold
    /// the ends of the cut better; the ring has no stops at the corners.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub mitre: bool,
}

fn default_eye_mm() -> f64 {
    3.0
}

impl Default for ButtonholeSpec {
    fn default() -> Self {
        ButtonholeSpec {
            shape: ButtonholeShape::Square,
            length_mm: 18.0,
            leg_mm: 1.8,
            gap_mm: 0.6,
            bartack_mm: 1.5,
            spacing_mm: 0.3,
            eye_mm: default_eye_mm(),
            corded: false,
            flip: false,
            mitre: false,
        }
    }
}

/// The least and the largest opening of a buttonhole, in mm.
pub const OPENING_MM: (f64, f64) = (5.0, 60.0);
/// The least and the largest hole of an eyelet, in mm.
pub const EYELET_MM: (f64, f64) = (1.5, 15.0);
/// One ligne (the button size unit "L"): 1/40 in.
pub const LIGNE_MM: f64 = 25.4 / 40.0;
/// The thickness of a usual flat button, in mm.
pub const BUTTON_THICKNESS_MM: f64 = 2.0;

/// A standard size: a name and a diameter in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardSize {
    pub name: &'static str,
    pub mm: f64,
}

impl StandardSize {
    /// The name and the button size in lignes, for example "1/2 in (20L)".
    pub fn label(&self) -> String {
        format!("{} ({:.0}L)", self.name, self.mm / LIGNE_MM)
    }
}

const fn size(name: &'static str, mm: f64) -> StandardSize {
    StandardSize { name, mm }
}

const INCH: f64 = 25.4;

/// Button diameters in mm.
pub const METRIC_BUTTONS: [StandardSize; 13] = [
    size("9 mm", 9.0),
    size("10 mm", 10.0),
    size("11.5 mm", 11.5),
    size("12.5 mm", 12.5),
    size("15 mm", 15.0),
    size("18 mm", 18.0),
    size("20 mm", 20.0),
    size("23 mm", 23.0),
    size("25 mm", 25.0),
    size("28 mm", 28.0),
    size("30 mm", 30.0),
    size("34 mm", 34.0),
    size("38 mm", 38.0),
];

/// Button diameters in inches.
pub const INCH_BUTTONS: [StandardSize; 15] = [
    size("1/4 in", INCH / 4.0),
    size("5/16 in", INCH * 5.0 / 16.0),
    size("3/8 in", INCH * 3.0 / 8.0),
    size("7/16 in", INCH * 7.0 / 16.0),
    size("1/2 in", INCH / 2.0),
    size("9/16 in", INCH * 9.0 / 16.0),
    size("5/8 in", INCH * 5.0 / 8.0),
    size("11/16 in", INCH * 11.0 / 16.0),
    size("3/4 in", INCH * 3.0 / 4.0),
    size("7/8 in", INCH * 7.0 / 8.0),
    size("1 in", INCH),
    size("1 1/8 in", INCH * 9.0 / 8.0),
    size("1 1/4 in", INCH * 5.0 / 4.0),
    size("1 3/8 in", INCH * 11.0 / 8.0),
    size("1 1/2 in", INCH * 3.0 / 2.0),
];

/// Eyelet holes in mm.
pub const METRIC_EYELETS: [StandardSize; 7] = [
    size("2 mm", 2.0),
    size("3 mm", 3.0),
    size("4 mm", 4.0),
    size("5 mm", 5.0),
    size("6 mm", 6.0),
    size("8 mm", 8.0),
    size("10 mm", 10.0),
];

/// Eyelet holes in inches.
pub const INCH_EYELETS: [StandardSize; 6] = [
    size("1/8 in", INCH / 8.0),
    size("3/16 in", INCH * 3.0 / 16.0),
    size("1/4 in", INCH / 4.0),
    size("5/16 in", INCH * 5.0 / 16.0),
    size("3/8 in", INCH * 3.0 / 8.0),
    size("1/2 in", INCH / 2.0),
];

/// `v` rounded to 0.1 mm.
fn tenth(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

impl ButtonholeSpec {
    /// The opening for a button: its diameter and its thickness (a domed or
    /// a shank button is thicker than a flat one).
    pub fn opening_for(diameter_mm: f64, thickness_mm: f64) -> f64 {
        (diameter_mm + thickness_mm.max(0.0)).clamp(OPENING_MM.0, OPENING_MM.1)
    }

    /// The spec with the stitches set for its size: for a buttonhole the
    /// legs, the gap, the bar tacks and the eye follow the opening; for an
    /// eyelet the satin follows the hole. The shape, the density, the cord
    /// and the turn do not change.
    pub fn scaled(&self) -> ButtonholeSpec {
        let mut s = self.clone();
        if s.shape == ButtonholeShape::Eyelet {
            let d = s.eye_mm.clamp(EYELET_MM.0, EYELET_MM.1);
            s.leg_mm = tenth((0.8 + 0.2 * d).clamp(1.0, 2.5));
            return s;
        }
        let l = s.length_mm.clamp(OPENING_MM.0, OPENING_MM.1);
        s.leg_mm = tenth((1.0 + 0.045 * l).clamp(1.2, 2.6));
        s.gap_mm = tenth((0.4 + 0.012 * l).clamp(0.5, 1.0));
        s.bartack_mm = tenth((0.5 + 0.055 * l).clamp(1.0, 3.0));
        s.eye_mm = tenth((0.5 + 0.15 * l).clamp(2.0, 5.0));
        s
    }

    /// The size that the window shows: the opening, or the hole of an eyelet.
    pub fn size_mm(&self) -> f64 {
        if self.shape == ButtonholeShape::Eyelet {
            self.eye_mm
        } else {
            self.length_mm
        }
    }
}

/// A straight centerline from `a` to `b`.
fn segment(a: Point, b: Point) -> Path {
    Path::new().move_to(a).line_to(b)
}

/// Points on a circle round `c` with radius `r`, from `a0` to `a1` degrees
/// (Y down: 90° is +Y), not more than 10° apart; with both ends.
fn arc(c: Point, r: f64, a0: f64, a1: f64) -> Vec<Point> {
    let n = ((a1 - a0).abs() / 10.0).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let (s, co) = math::sin_cos_deg(a0 + (a1 - a0) * i as f64 / n as f64);
            Point::new(c.x + r * co, c.y + r * s)
        })
        .collect()
}

fn polyline(points: &[Point]) -> Path {
    let mut p = Path::new().move_to(points[0]);
    for &q in &points[1..] {
        p = p.line_to(q);
    }
    p
}

/// The sizes of a buttonhole in its own frame: the length along +X, the
/// middle at the origin.
struct Frame {
    /// Half the opening.
    h: f64,
    leg: f64,
    /// The distance of each leg's centerline from the middle line.
    off: f64,
    /// Half the width of the buttonhole.
    edge: f64,
    bar: f64,
    /// The box (the run under the edge of the satin) inside the shapes that
    /// are not square.
    inset_edge: f64,
}

impl Frame {
    fn of(spec: &ButtonholeSpec) -> Frame {
        let h = spec.length_mm.max(1.0) / 2.0;
        let leg = spec.leg_mm.max(0.5);
        let gap = spec.gap_mm.max(0.0);
        let edge = (2.0 * leg + gap) / 2.0;
        Frame {
            h,
            leg,
            off: gap / 2.0 + leg / 2.0,
            edge,
            bar: spec.bartack_mm.max(0.5),
            inset_edge: edge - (leg * 0.25).min(0.3),
        }
    }

    /// The radius of the hole of the eye of a keyhole, and the middle of the eye.
    fn eye(&self, spec: &ButtonholeSpec) -> (f64, Point) {
        let r = (spec.eye_mm / 2.0).clamp(self.edge - self.leg + 0.2, self.h * 0.8);
        (r, Point::new(self.h - r, 0.0))
    }

    /// The length of the tapered end of a tapered keyhole.
    fn taper(&self) -> f64 {
        (2.0 * self.bar).max(2.0)
    }
}

/// The local points where a line `y = ±e` meets a circle round `c` with
/// radius `r` (on the side of -X), and the angle of the point at `-e`.
fn meet(c: Point, r: f64, e: f64) -> (f64, f64) {
    let x = c.x - (r * r - e * e).max(0.0).sqrt();
    (x, math::to_degrees(math::atan2(-e, x - c.x)))
}

/// The box of a buttonhole in its own frame. Its first segment goes along
/// +X (see [`placement_of_box`]).
fn box_local(spec: &ButtonholeSpec, f: &Frame) -> Path {
    let (h, bar) = (f.h, f.bar);
    let e = f.inset_edge;
    match spec.shape {
        ButtonholeShape::Square | ButtonholeShape::Stretch => Path::polygon(&[
            Point::new(-h - bar, -f.edge),
            Point::new(h + bar, -f.edge),
            Point::new(h + bar, f.edge),
            Point::new(-h - bar, f.edge),
        ]),
        ButtonholeShape::RoundEnd => {
            let mut pts = vec![Point::new(-h - bar, -e)];
            pts.extend(arc(Point::new(h, 0.0), e, -90.0, 90.0));
            pts.push(Point::new(-h - bar, e));
            polyline(&pts).close()
        }
        ButtonholeShape::RoundEnds => {
            let mut pts = vec![Point::new(-h, -e)];
            pts.extend(arc(Point::new(h, 0.0), e, -90.0, 90.0));
            pts.extend(arc(Point::new(-h, 0.0), e, 90.0, 270.0));
            pts.pop();
            polyline(&pts).close()
        }
        ButtonholeShape::Keyhole | ButtonholeShape::TaperedKeyhole => {
            let (r, c) = f.eye(spec);
            let ro = r + f.leg - (f.edge - e);
            let (_, a) = meet(c, ro, e);
            let start = if spec.shape == ButtonholeShape::Keyhole {
                -h - bar
            } else {
                -h
            };
            let mut pts = vec![Point::new(start, -e)];
            pts.extend(arc(c, ro, a, -a));
            pts.push(Point::new(start, e));
            if spec.shape == ButtonholeShape::TaperedKeyhole {
                let t = f.taper() * 0.9;
                let tip = e * 0.15 + 0.15;
                pts.push(Point::new(-h - t, tip));
                pts.push(Point::new(-h - t, -tip));
            }
            polyline(&pts).close()
        }
        ButtonholeShape::Eyelet => {
            let r = spec.eye_mm.clamp(EYELET_MM.0, EYELET_MM.1) / 2.0 + f.leg - (f.edge - e);
            let mut pts = arc(Point::new(0.0, 0.0), r, -90.0, 270.0);
            pts.pop();
            polyline(&pts).close()
        }
    }
}

fn satin_params(spacing_mm: f64, short_stitches: bool) -> SatinParams {
    SatinParams {
        spacing_mm,
        pull_compensation_mm: 0.0,
        short_stitches,
        underlay: Underlay::None,
        ..SatinParams::default()
    }
}

fn satin(center: Path, width: f64, spacing_mm: f64, short: bool) -> ObjectKind {
    ObjectKind::Satin {
        geometry: SatinGeometry::from_centerline(&center, width),
        params: satin_params(spacing_mm, short),
    }
}

fn running(path: Path, stitch_length_mm: f64) -> ObjectKind {
    ObjectKind::Running {
        path,
        params: RunningParams {
            stitch_length_mm,
            ..RunningParams::default()
        },
    }
}

/// A satin part along `center`, after a run along it if the buttonhole is corded.
fn push_part(out: &mut Vec<ObjectKind>, spec: &ButtonholeSpec, center: Path, width: f64, spacing_mm: f64, short: bool) {
    if spec.corded {
        out.push(running(center.clone(), 2.0));
    }
    out.push(satin(center, width, spacing_mm, short));
}

/// The legs and the bar tacks of a buttonhole as one ring with mitre
/// corners: `parts` are the centerlines and widths in sew order. Before the
/// ring, a run along each leg if the buttonhole is corded. `false` (and no
/// objects) if the parts do not make a ring.
fn push_ring(out: &mut Vec<ObjectKind>, spec: &ButtonholeSpec, parts: &[(Path, f64, bool)]) -> bool {
    let cols: Vec<SatinGeometry> = parts
        .iter()
        .map(|(c, w, _)| SatinGeometry::from_centerline(c, *w))
        .collect();
    let ring = cols[1..]
        .iter()
        .try_fold(cols[0].clone(), |g, next| mitre::join(&g, next))
        .and_then(|g| mitre::close(&g));
    let Some(geometry) = ring else { return false };
    if spec.corded {
        out.extend(parts.iter().filter(|p| p.2).map(|(c, _, _)| running(c.clone(), 2.0)));
    }
    out.push(ObjectKind::Satin {
        geometry,
        params: satin_params(spec.spacing_mm, true),
    });
    true
}

/// The objects of one buttonhole with its middle at `center`, its length
/// along `angle_deg` (0°: along +X), in sew order. The first object is the
/// running box that holds the fabric; then, for each satin part, a run
/// along it if the buttonhole is corded and the satin.
pub fn buttonhole(spec: &ButtonholeSpec, center: Point, angle_deg: f64) -> Vec<ObjectKind> {
    let angle = if spec.flip && spec.shape.has_ends() {
        angle_deg + 180.0
    } else {
        angle_deg
    };
    let t = Transform::rotate_deg(angle).then(Transform::translate(center.x, center.y));
    let f = Frame::of(spec);
    let at = |x: f64, y: f64| t.apply(Point::new(x, y));
    let world = |pts: Vec<Point>| -> Vec<Point> { pts.into_iter().map(|p| t.apply(p)).collect() };
    let mut out = vec![running(box_local(spec, &f).transformed(&t), 1.5)];
    let (h, off, edge, bar, leg) = (f.h, f.off, f.edge, f.bar, f.leg);
    let end = h + bar / 2.0;
    let bar_at_end = segment(at(end, -edge), at(end, edge));
    let bar_at_start = segment(at(-end, edge), at(-end, -edge));
    match spec.shape {
        ButtonholeShape::Square | ButtonholeShape::Stretch => {
            let spacing = if spec.shape == ButtonholeShape::Stretch {
                (spec.spacing_mm * 3.0).max(0.9)
            } else {
                spec.spacing_mm
            };
            let (leg_a, leg_b) = (segment(at(-h, -off), at(h, -off)), segment(at(h, off), at(-h, off)));
            if spec.mitre
                && spec.shape.can_mitre()
                && push_ring(
                    &mut out,
                    spec,
                    &[
                        (leg_a.clone(), leg, true),
                        (bar_at_end.clone(), bar, false),
                        (leg_b.clone(), leg, true),
                        (bar_at_start.clone(), bar, false),
                    ],
                )
            {
                return out;
            }
            push_part(&mut out, spec, leg_a, leg, spacing, false);
            out.push(satin(bar_at_end, bar, spec.spacing_mm, false));
            push_part(&mut out, spec, leg_b, leg, spacing, false);
            out.push(satin(bar_at_start, bar, spec.spacing_mm, false));
        }
        ButtonholeShape::RoundEnd => {
            let mut pts = vec![Point::new(-h, -off)];
            pts.extend(arc(Point::new(h, 0.0), off, -90.0, 90.0));
            pts.push(Point::new(-h, off));
            let legs = polyline(&world(pts));
            if spec.mitre
                && push_ring(
                    &mut out,
                    spec,
                    &[(legs.clone(), leg, true), (bar_at_start.clone(), bar, false)],
                )
            {
                return out;
            }
            push_part(&mut out, spec, legs, leg, spec.spacing_mm, true);
            out.push(satin(bar_at_start, bar, spec.spacing_mm, false));
        }
        ButtonholeShape::RoundEnds => {
            let mut pts = vec![Point::new(-h, -off)];
            pts.extend(arc(Point::new(h, 0.0), off, -90.0, 90.0));
            pts.extend(arc(Point::new(-h, 0.0), off, 90.0, 270.0));
            pts.pop();
            push_part(
                &mut out,
                spec,
                polyline(&world(pts)).close(),
                leg,
                spec.spacing_mm,
                true,
            );
        }
        ButtonholeShape::Keyhole | ButtonholeShape::TaperedKeyhole => {
            let (r, c) = f.eye(spec);
            let rc = r + leg / 2.0;
            let (x, a) = meet(c, rc, off);
            let mut pts = vec![Point::new(-h, -off), Point::new(x, -off)];
            pts.extend(arc(c, rc, a, -a));
            pts.push(Point::new(-h, off));
            let legs = polyline(&world(pts));
            if spec.mitre
                && spec.shape == ButtonholeShape::Keyhole
                && push_ring(
                    &mut out,
                    spec,
                    &[(legs.clone(), leg, true), (bar_at_start.clone(), bar, false)],
                )
            {
                return out;
            }
            push_part(&mut out, spec, legs, leg, spec.spacing_mm, true);
            if spec.shape == ButtonholeShape::Keyhole {
                out.push(satin(bar_at_start, bar, spec.spacing_mm, false));
            } else {
                // The taper closes the end: stitches across, narrower to the tip.
                let tl = f.taper();
                let tip = 0.3;
                out.push(ObjectKind::Satin {
                    geometry: SatinGeometry {
                        rail_a: segment(at(-h, -edge), at(-h - tl, -tip)),
                        rail_b: segment(at(-h, edge), at(-h - tl, tip)),
                        rungs: Vec::new(),
                    },
                    params: satin_params(spec.spacing_mm, false),
                });
            }
        }
        ButtonholeShape::Eyelet => {
            let r = spec.eye_mm.clamp(EYELET_MM.0, EYELET_MM.1) / 2.0 + leg / 2.0;
            let mut pts = arc(Point::new(0.0, 0.0), r, -90.0, 270.0);
            pts.pop();
            push_part(
                &mut out,
                spec,
                polyline(&world(pts)).close(),
                leg,
                spec.spacing_mm,
                true,
            );
        }
    }
    out
}

/// The middle and the angle of a buttonhole of `spec` from its box (the
/// first object of [`buttonhole`]), after the user moved, turned or
/// mirrored it. `None` if `path` is not a box of this spec.
pub fn placement_of_box(spec: &ButtonholeSpec, path: &Path) -> Option<(Point, f64)> {
    let pts: Vec<Point> = path.flatten(0.05).into_iter().next()?.points;
    if pts.len() < 3 {
        return None;
    }
    let local = box_local(spec, &Frame::of(spec)).control_bbox()?;
    if spec.shape == ButtonholeShape::Eyelet {
        let n = (pts.len() - usize::from(pts.first() == pts.last())) as f64;
        let sum = pts.iter().take(n as usize).fold(Point::new(0.0, 0.0), |s, &p| s + p);
        return Some((sum * (1.0 / n), 0.0));
    }
    let u = (pts[1] - pts[0]).normalized()?;
    let v = u.perp();
    let (mut u0, mut u1, mut v0, mut v1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &p in &pts {
        let d = p - pts[0];
        u0 = u0.min(d.dot(u));
        u1 = u1.max(d.dot(u));
        v0 = v0.min(d.dot(v));
        v1 = v1.max(d.dot(v));
    }
    let lc = local.center();
    let center = pts[0] + u * ((u0 + u1) / 2.0 - lc.x) + v * ((v0 + v1) / 2.0);
    let mut angle = math::to_degrees(math::atan2(u.y, u.x));
    if spec.flip && spec.shape.has_ends() {
        angle -= 180.0;
    }
    Some((center, angle))
}

/// The middles and the angles of `count` buttonholes spaced evenly along
/// `line` (the first and the last at its ends). With `across`, each
/// buttonhole is across the line; else along it.
pub fn string_positions(line: &[Point], count: usize, across: bool) -> Vec<(Point, f64)> {
    let total = line_length(line);
    if line.len() < 2 || count == 0 || total <= 0.0 {
        return Vec::new();
    }
    let at: Vec<f64> = (0..count)
        .map(|k| {
            if count == 1 {
                total / 2.0
            } else {
                total * k as f64 / (count - 1) as f64
            }
        })
        .collect();
    positions_at(line, &at, across)
}

/// The middles and the angles of buttonholes `distance_mm` apart along
/// `line`, from its start (the last one is at the end of the line or before it).
pub fn string_every(line: &[Point], distance_mm: f64, across: bool) -> Vec<(Point, f64)> {
    let total = line_length(line);
    if line.len() < 2 || total <= 0.0 || distance_mm <= 0.0 {
        return Vec::new();
    }
    let count = ((total + 1e-6) / distance_mm).floor() as usize + 1;
    let at: Vec<f64> = (0..count.min(500)).map(|k| k as f64 * distance_mm).collect();
    positions_at(line, &at, across)
}

fn line_length(line: &[Point]) -> f64 {
    line.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The points and the angles at the distances `at` along `line`.
fn positions_at(line: &[Point], at: &[f64], across: bool) -> Vec<(Point, f64)> {
    let mut len = vec![0.0];
    for w in line.windows(2) {
        len.push(len[len.len() - 1] + w[0].distance(w[1]));
    }
    at.iter()
        .map(|&s| {
            let i = len.partition_point(|&l| l <= s).clamp(1, line.len() - 1);
            let seg = len[i] - len[i - 1];
            let f = if seg > 0.0 { (s - len[i - 1]) / seg } else { 0.0 };
            let p = line[i - 1].lerp(line[i], f);
            let d = line[i] - line[i - 1];
            let a = math::to_degrees(math::atan2(d.y, d.x));
            (p, if across { a + 90.0 } else { a })
        })
        .collect()
}

/// Buttonholes that the Buttonholes window can change: the group of their
/// objects, their spec, and the box (the first object) of each buttonhole
/// in sew order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonholeRecord {
    pub group: u32,
    pub spec: ButtonholeSpec,
    pub boxes: Vec<ObjectId>,
}

/// The size of the objects of a buttonhole of `spec` at the origin, along +X.
pub fn local_size(spec: &ButtonholeSpec) -> Option<BBox> {
    buttonhole(spec, Point::new(0.0, 0.0), 0.0)
        .iter()
        .filter_map(|k| k.control_bbox())
        .reduce(BBox::union)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bbox(objs: &[ObjectKind]) -> BBox {
        objs.iter()
            .filter_map(|k| k.control_bbox())
            .reduce(BBox::union)
            .unwrap()
    }

    #[test]
    fn a_buttonhole_has_a_box_two_legs_and_two_bar_tacks() {
        let spec = ButtonholeSpec::default();
        let objs = buttonhole(&spec, Point::new(10.0, 10.0), 90.0);
        assert_eq!(objs.len(), 5);
        assert!(matches!(objs[0], ObjectKind::Running { .. }));
        let b = bbox(&objs);
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
    fn each_shape_is_a_box_and_satin_in_its_size() {
        for shape in ButtonholeShape::ALL {
            let spec = ButtonholeSpec {
                shape,
                ..ButtonholeSpec::default()
            }
            .scaled();
            let objs = buttonhole(&spec, Point::new(0.0, 0.0), 0.0);
            assert!(matches!(objs[0], ObjectKind::Running { .. }), "{shape:?}");
            assert!(
                objs[1..].iter().all(|k| matches!(k, ObjectKind::Satin { .. })),
                "{shape:?}"
            );
            let b = bbox(&objs);
            assert!(b.min.is_finite() && b.max.is_finite(), "{shape:?}");
            let w = 2.0 * spec.leg_mm + spec.gap_mm;
            if shape == ButtonholeShape::Eyelet {
                let d = spec.eye_mm + 2.0 * spec.leg_mm;
                assert!((b.width() - d).abs() < 0.05 && (b.height() - d).abs() < 0.05, "{b:?}");
            } else {
                // The ends are outside the opening, not more than a few mm.
                assert!(
                    b.width() > spec.length_mm && b.width() < spec.length_mm + 2.0 * w + 4.0,
                    "{shape:?} {b:?}"
                );
                // A keyhole is wider at its eye.
                assert!(b.height() >= w - 1e-6 && b.height() <= w.max(spec.eye_mm + 2.0 * spec.leg_mm) + 1e-6);
            }
            // The box is under the satin: inside the box of the objects.
            let bb = objs[0].control_bbox().unwrap();
            assert!(b.contains_box(&bb), "{shape:?}: {bb:?} in {b:?}");
            // The gap is free for the cut: no satin crosses the middle line
            // inside the opening (but an eyelet and the ends).
            if !matches!(shape, ButtonholeShape::Eyelet) {
                for k in &objs[1..] {
                    let ObjectKind::Satin { geometry, .. } = k else {
                        continue;
                    };
                    for p in geometry.rail_a.points().chain(geometry.rail_b.points()) {
                        if p.x.abs() < spec.length_mm / 2.0 - spec.eye_mm - 0.5 {
                            assert!(p.y.abs() >= spec.gap_mm / 2.0 - 1e-6, "{shape:?}: {p:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_corded_buttonhole_has_a_run_under_each_leg() {
        for shape in ButtonholeShape::ALL {
            let plain = ButtonholeSpec {
                shape,
                ..ButtonholeSpec::default()
            };
            let corded = ButtonholeSpec {
                corded: true,
                ..plain.clone()
            };
            let a = buttonhole(&plain, Point::new(0.0, 0.0), 0.0);
            let b = buttonhole(&corded, Point::new(0.0, 0.0), 0.0);
            let legs = if matches!(shape, ButtonholeShape::Square | ButtonholeShape::Stretch) {
                2
            } else {
                1
            };
            assert_eq!(b.len(), a.len() + legs, "{shape:?}");
        }
    }

    #[test]
    fn mitre_corners_make_one_ring() {
        for shape in ButtonholeShape::ALL {
            for corded in [false, true] {
                let spec = ButtonholeSpec {
                    shape,
                    corded,
                    mitre: true,
                    ..ButtonholeSpec::default()
                };
                let objs = buttonhole(&spec, Point::new(0.0, 0.0), 0.0);
                let plain = buttonhole(
                    &ButtonholeSpec {
                        mitre: false,
                        ..spec.clone()
                    },
                    Point::new(0.0, 0.0),
                    0.0,
                );
                if !shape.can_mitre() {
                    assert_eq!(objs, plain, "{shape:?}: no mitre");
                    continue;
                }
                let satins: Vec<&SatinGeometry> = objs
                    .iter()
                    .filter_map(|o| match o {
                        ObjectKind::Satin { geometry, .. } => Some(geometry),
                        _ => None,
                    })
                    .collect();
                assert_eq!(satins.len(), 1, "{shape:?}: one ring");
                let runs = objs.len() - 2;
                let legs = if shape == ButtonholeShape::Square { 2 } else { 1 };
                assert_eq!(runs, if corded { legs } else { 0 }, "{shape:?}");
                let a = satins[0].rail_a.flatten(0.01).remove(0);
                assert!(a.closed, "{shape:?}");
                // The ring covers the box of the plain buttonhole.
                let bb = |k: &[ObjectKind]| {
                    k.iter()
                        .skip(1)
                        .filter_map(|o| match o {
                            ObjectKind::Satin { geometry, .. } => {
                                geometry.rail_a.control_bbox().zip(geometry.rail_b.control_bbox())
                            }
                            _ => None,
                        })
                        .map(|(x, y)| x.union(y))
                        .reduce(|x, y| x.union(y))
                        .unwrap()
                };
                let (m, q) = (bb(&objs), bb(&plain));
                assert!(
                    m.min.distance(q.min) < 1e-6 && m.max.distance(q.max) < 1e-6,
                    "{shape:?}: {m:?} {q:?}"
                );
            }
        }
    }

    #[test]
    fn the_stretch_legs_are_open() {
        let spec = ButtonholeSpec {
            shape: ButtonholeShape::Stretch,
            ..ButtonholeSpec::default()
        };
        let objs = buttonhole(&spec, Point::new(0.0, 0.0), 0.0);
        let ObjectKind::Satin { params, .. } = &objs[1] else {
            panic!()
        };
        assert!(params.spacing_mm >= 0.9);
        let ObjectKind::Satin { params, .. } = &objs[2] else {
            panic!()
        };
        assert_eq!(params.spacing_mm, spec.spacing_mm, "the bar tacks are dense");
    }

    #[test]
    fn a_turned_keyhole_has_its_eye_at_the_other_end() {
        let spec = ButtonholeSpec {
            shape: ButtonholeShape::Keyhole,
            ..ButtonholeSpec::default()
        };
        let eye_x = |s: &ButtonholeSpec| {
            let objs = buttonhole(s, Point::new(0.0, 0.0), 0.0);
            // The widest point of the satin is in the eye.
            let ObjectKind::Satin { geometry, .. } = &objs[1] else {
                panic!()
            };
            geometry
                .rail_a
                .points()
                .max_by(|a, b| a.y.abs().total_cmp(&b.y.abs()))
                .unwrap()
                .x
        };
        assert!(eye_x(&spec) > 0.0);
        assert!(eye_x(&ButtonholeSpec { flip: true, ..spec }) < 0.0);
    }

    #[test]
    fn the_box_gives_the_place_again() {
        for shape in ButtonholeShape::ALL {
            for flip in [false, true] {
                let spec = ButtonholeSpec {
                    shape,
                    flip,
                    ..ButtonholeSpec::default()
                }
                .scaled();
                let c = Point::new(12.0, -7.0);
                let objs = buttonhole(&spec, c, 30.0);
                let ObjectKind::Running { path, .. } = &objs[0] else {
                    panic!()
                };
                let (p, a) = placement_of_box(&spec, path).unwrap();
                assert!(p.distance(c) < 1e-6, "{shape:?} {flip}: {p:?}");
                if shape != ButtonholeShape::Eyelet {
                    assert!(
                        ((a - 30.0).rem_euclid(360.0)).min((30.0 - a).rem_euclid(360.0)) < 1e-6,
                        "{shape:?}: {a}"
                    );
                }
                // Mirrored across the buttonhole and across the other way:
                // the place is the same, and the angle sews the same shape.
                for m in [Transform::scale(1.0, -1.0), Transform::scale(-1.0, 1.0)] {
                    let t = Transform::translate(-c.x, -c.y)
                        .then(Transform::rotate_deg(-30.0))
                        .then(m)
                        .then(Transform::rotate_deg(30.0))
                        .then(Transform::translate(c.x, c.y));
                    let (p, a) = placement_of_box(&spec, &path.transformed(&t)).unwrap();
                    assert!(p.distance(c) < 1e-6, "{shape:?} {flip} mirrored: {p:?}");
                    let again = buttonhole(&spec, p, a);
                    let b0 = bbox(&objs[1..]);
                    let b1 = bbox(&again[1..]);
                    assert!(b0.center().distance(b1.center()) < 1e-6 || shape.has_ends());
                }
            }
        }
    }

    #[test]
    fn standard_sizes_grow_and_scale_the_stitches() {
        for list in [
            &METRIC_BUTTONS[..],
            &INCH_BUTTONS[..],
            &METRIC_EYELETS[..],
            &INCH_EYELETS[..],
        ] {
            assert!(list.windows(2).all(|w| w[0].mm < w[1].mm));
        }
        assert_eq!(INCH_BUTTONS[4].label(), "1/2 in (20L)");
        assert_eq!(METRIC_BUTTONS[1].label(), "10 mm (16L)");
        // A 1/2 in flat button: 12.7 + 2 mm.
        let open = ButtonholeSpec::opening_for(INCH / 2.0, BUTTON_THICKNESS_MM);
        assert!((open - 14.7).abs() < 1e-9);
        let small = ButtonholeSpec {
            length_mm: 10.0,
            ..ButtonholeSpec::default()
        }
        .scaled();
        let large = ButtonholeSpec {
            length_mm: 40.0,
            ..ButtonholeSpec::default()
        }
        .scaled();
        assert!(small.leg_mm < large.leg_mm && small.bartack_mm < large.bartack_mm);
        // The usual 18 mm buttonhole keeps the settings of the default.
        let usual = ButtonholeSpec::default().scaled();
        let d = ButtonholeSpec::default();
        assert_eq!(
            (usual.leg_mm, usual.gap_mm, usual.bartack_mm),
            (d.leg_mm, d.gap_mm, d.bartack_mm)
        );
        let eyelet = |mm| {
            ButtonholeSpec {
                shape: ButtonholeShape::Eyelet,
                eye_mm: mm,
                ..ButtonholeSpec::default()
            }
            .scaled()
            .leg_mm
        };
        assert!(eyelet(3.0) < eyelet(10.0));
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
    fn a_string_by_distance_starts_at_the_line_start() {
        let line = [Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let p = string_every(&line, 30.0, true);
        assert_eq!(p.len(), 4);
        assert!((p[3].0.y - 90.0).abs() < 1e-9);
        assert_eq!(string_every(&line, 25.0, true).len(), 5, "the last one at the end");
        assert!(string_every(&line, 0.0, true).is_empty());
    }
}
