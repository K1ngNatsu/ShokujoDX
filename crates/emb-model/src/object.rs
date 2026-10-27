//! Design objects: geometry plus stitch parameters.

use std::sync::atomic::{AtomicU64, Ordering};

use emb_geom::{BBox, Path, Point};
use serde::{Deserialize, Serialize};

use crate::ThreadId;
use crate::applique::{AppliqueParams, AppliquePiece};
use crate::motif::{Motif, MotifRun};
use crate::split::{SplitPattern, StampPlacement};

/// Stable identifier of an object in a design. It does not change when the
/// object changes or moves in the sew order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u64);

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// Returns a revision number that is unique in this process.
///
/// Revisions are unique across all designs and all undo branches, so the
/// stitch cache key `(ObjectId, revision)` never refers to two different contents.
/// Revisions are not saved in project files; loading gives fresh revisions.
pub fn fresh_revision() -> u64 {
    NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// One object of a design. The position in [`crate::Design::objects`] is the sew order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignObject {
    pub id: ObjectId,
    /// Changes each time the object changes. Use `(id, revision)` as the stitch cache key.
    #[serde(skip, default = "fresh_revision")]
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub thread: ThreadId,
    /// Where the object starts to sew. If `None`, the engine selects the point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Point>,
    /// Where the object stops sewing. If `None`, the engine selects the point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Point>,
    /// Objects with the same group number are selected and moved together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    /// A locked object cannot be selected on the canvas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// Machine functions: lock stitches, trim and stop.
    #[serde(default, skip_serializing_if = "Functions::is_default")]
    pub functions: Functions,
    pub kind: ObjectKind,
}

/// What the machine does after an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrimMode {
    /// Trim when the next object of the same color is farther than the
    /// trim distance of the machine; else a travel stitch.
    #[default]
    Auto,
    /// Always trim after the object.
    Always,
    /// Never trim after the object (but at a color change); a long gap is a jump.
    Never,
}

/// The machine functions of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Functions {
    /// Lock stitches where the object starts after a trim.
    pub tie_in: bool,
    /// Lock stitches where the object ends before a trim.
    pub tie_off: bool,
    pub trim: TrimMode,
    /// The machine stops after the object (for example to put on appliqué
    /// fabric). A color change after the object stops the machine anyway.
    pub stop: bool,
    /// The machine sews the object slowly (for example small lettering or
    /// metallic thread). Only industrial formats with speed functions
    /// keep it (U01); the other formats sew at the normal speed.
    pub slow: bool,
}

impl Default for Functions {
    fn default() -> Self {
        Functions {
            tie_in: true,
            tie_off: true,
            trim: TrimMode::Auto,
            stop: false,
            slow: false,
        }
    }
}

impl Functions {
    pub fn is_default(&self) -> bool {
        *self == Functions::default()
    }
}

impl DesignObject {
    /// Compares all saved fields. The revision is not compared.
    pub fn same_content(&self, o: &DesignObject) -> bool {
        self.id == o.id
            && self.name == o.name
            && self.thread == o.thread
            && self.start == o.start
            && self.end == o.end
            && self.group == o.group
            && self.locked == o.locked
            && self.functions == o.functions
            && self.kind == o.kind
    }

    /// Bounding box of the geometry, control points included.
    pub fn control_bbox(&self) -> Option<BBox> {
        self.kind.control_bbox()
    }

    pub fn type_name(&self) -> &'static str {
        self.kind.type_name()
    }
}

/// The stitch type of an object, with the geometry and parameters for that type.
///
/// Geometry and parameters are in one enum, so an object cannot have a
/// geometry that does not match its stitch type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObjectKind {
    Running {
        path: Path,
        #[serde(default)]
        params: RunningParams,
    },
    Satin {
        geometry: SatinGeometry,
        #[serde(default)]
        params: SatinParams,
    },
    Fill {
        outer: Path,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<Path>,
        #[serde(default)]
        params: FillParams,
    },
    /// Stitches from a machine file, sewn as they are. Each subpath of `path`
    /// (straight lines only) is one sequence of needle points; a jump goes
    /// to the start of the next subpath. `trims` are the subpaths that start
    /// after a trim. Reshape moves, adds and deletes single needle points.
    Stitches {
        path: Path,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        trims: Vec<usize>,
    },
    /// Appliqué fabric: placement line, stop, tack-down, stop, cover (see
    /// `applique`). Several pieces sew each step together.
    Applique {
        pieces: Vec<AppliquePiece>,
        #[serde(default)]
        params: AppliqueParams,
    },
    /// Text in a font: letters made of satin and fill parts (ADR 0015).
    Lettering(Box<crate::Lettering>),
    /// Artwork with no stitches (ADR 0024): the subpaths of `path` with a
    /// fill (even-odd) and a stroke. Convert makes embroidery from it.
    Vector {
        path: Path,
        #[serde(default)]
        style: crate::vector::VectorStyle,
    },
    /// Crosses on a grid of cells (ADR 0038). It has no paths.
    CrossStitch(Box<crate::cross::CrossStitch>),
    /// Rhinestones (ADR 0039): no stitches.
    Bling(Box<crate::bling::Bling>),
    /// A parametric sketch (P14, ADR 0069): curves with constraints and
    /// dimensions, no stitches.
    Sketch(Box<crate::sketch::SketchObject>),
}

impl ObjectKind {
    /// Bounding box of the geometry, control points included.
    pub fn control_bbox(&self) -> Option<BBox> {
        match self {
            ObjectKind::CrossStitch(x) => return x.bbox(),
            ObjectKind::Bling(b) => return b.bbox(),
            _ => {}
        }
        self.paths().filter_map(Path::control_bbox).reduce(BBox::union)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ObjectKind::Running { .. } => "Running",
            ObjectKind::Satin { .. } => "Satin",
            ObjectKind::Fill { .. } => "Fill",
            ObjectKind::Stitches { .. } => "Stitches",
            ObjectKind::Applique { .. } => "Appliqué",
            ObjectKind::Lettering(_) => "Lettering",
            ObjectKind::Vector { .. } => "Vector",
            ObjectKind::CrossStitch(_) => "Cross stitch",
            ObjectKind::Bling(_) => "Rhinestones",
            ObjectKind::Sketch(_) => "Sketch",
        }
    }

    /// Returns `false` for objects that sew nothing (vectors, rhinestones).
    pub fn sews(&self) -> bool {
        !matches!(
            self,
            ObjectKind::Vector { .. } | ObjectKind::Bling(_) | ObjectKind::Sketch(_)
        )
    }

    /// All vector paths of the geometry.
    pub fn paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
        match self {
            ObjectKind::Running { path, .. } => Box::new(std::iter::once(path)),
            ObjectKind::Satin { geometry, .. } => Box::new([&geometry.rail_a, &geometry.rail_b].into_iter()),
            ObjectKind::Fill { outer, holes, .. } => Box::new(std::iter::once(outer).chain(holes.iter())),
            ObjectKind::Stitches { path, .. } => Box::new(std::iter::once(path)),
            ObjectKind::Applique { pieces, .. } => Box::new(
                pieces
                    .iter()
                    .flat_map(|p| std::iter::once(&p.outer).chain(p.holes.iter())),
            ),
            ObjectKind::Lettering(l) => Box::new(l.parts().flat_map(|p| p.paths())),
            ObjectKind::Vector { path, .. } => Box::new(std::iter::once(path)),
            ObjectKind::CrossStitch(_) => Box::new(std::iter::empty()),
            ObjectKind::Bling(b) => Box::new(b.paths().into_iter()),
            ObjectKind::Sketch(s) => Box::new(s.paths()),
        }
    }

    /// All vector paths of the geometry, in the same order as [`ObjectKind::paths`].
    pub fn paths_mut(&mut self) -> Vec<&mut Path> {
        match self {
            ObjectKind::Running { path, .. } => vec![path],
            ObjectKind::Satin { geometry, .. } => vec![&mut geometry.rail_a, &mut geometry.rail_b],
            ObjectKind::Fill { outer, holes, .. } => std::iter::once(outer).chain(holes.iter_mut()).collect(),
            ObjectKind::Stitches { path, .. } => vec![path],
            ObjectKind::Applique { pieces, .. } => pieces
                .iter_mut()
                .flat_map(|p| std::iter::once(&mut p.outer).chain(p.holes.iter_mut()))
                .collect(),
            ObjectKind::Lettering(l) => l.parts_mut().flat_map(|p| p.paths_mut()).collect(),
            ObjectKind::Vector { path, .. } => vec![path],
            ObjectKind::CrossStitch(_) => Vec::new(),
            ObjectKind::Bling(b) => b.paths_mut(),
            // The paths of a sketch come from its points (`sketch`).
            ObjectKind::Sketch(_) => Vec::new(),
        }
    }
}

/// Satin column geometry: two rails and optional rungs.
///
/// The stitches go from `rail_a` to `rail_b`. Each rung is a pair of points,
/// one near each rail, that sets the stitch direction at that place.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SatinGeometry {
    pub rail_a: Path,
    pub rail_b: Path,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rungs: Vec<(Point, Point)>,
}

/// Tolerance to flatten a centerline for [`SatinGeometry::from_centerline`], in mm.
const CENTERLINE_TOL_MM: f64 = 0.05;
/// At sharp corners the rails go no farther out than this many half widths.
const MAX_MITER: f64 = 2.0;
/// Corners sharper than this (the cosine of the turn) get a cap.
const CAP_COS: f64 = 0.707;
/// [`SatinGeometry::from_centerline`] puts a rung at least this often, so
/// the rails stay paired on curves.
const RUNG_STEP_MM: f64 = 2.0;

/// Points of a closed ring, and the length along it at each point.
struct Ring {
    pts: Vec<Point>,
    at: Vec<f64>,
}

impl Ring {
    fn new(mut pts: Vec<Point>) -> Ring {
        pts.dedup();
        if pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        let mut at = vec![0.0];
        for i in 0..pts.len() {
            let next = pts[(i + 1) % pts.len()];
            at.push(at[i] + pts[i].distance(next));
        }
        Ring { pts, at }
    }

    fn len(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    /// Length along the ring of the ring point nearest to `p`.
    fn project(&self, p: Point) -> f64 {
        let n = self.pts.len();
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..n {
            let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
            let d = b - a;
            let l2 = d.length_sq();
            let t = if l2 > 0.0 {
                ((p - a).dot(d) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dist = p.distance(a.lerp(b, t));
            if dist < best.0 {
                best = (dist, self.at[i] + t * (self.at[i + 1] - self.at[i]));
            }
        }
        best.1
    }

    fn at(&self, s: f64) -> Point {
        let n = self.pts.len();
        let s = s.rem_euclid(self.len());
        let i = self.at.partition_point(|&l| l <= s).clamp(1, n) - 1;
        let seg = self.at[i + 1] - self.at[i];
        let t = if seg > 0.0 { (s - self.at[i]) / seg } else { 0.0 };
        self.pts[i].lerp(self.pts[(i + 1) % n], t)
    }

    /// The ring from `from` to `to` (lengths along the ring), forward or backward.
    fn arc(&self, from: f64, to: f64, forward: bool) -> Vec<Point> {
        let l = self.len();
        let span = if forward {
            (to - from).rem_euclid(l)
        } else {
            (from - to).rem_euclid(l)
        };
        let mut out = vec![self.at(from)];
        // The ring points inside the arc, in order.
        let mut inner: Vec<(f64, Point)> = self
            .pts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let d = if forward {
                    (self.at[i] - from).rem_euclid(l)
                } else {
                    (from - self.at[i]).rem_euclid(l)
                };
                (d, *p)
            })
            .filter(|(d, _)| *d > 1e-9 && *d < span - 1e-9)
            .collect();
        inner.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.extend(inner.into_iter().map(|(_, p)| p));
        out.push(self.at(to));
        out
    }
}

fn polyline_path(pts: &[Point]) -> Path {
    pts[1..]
        .iter()
        .fold(Path::new().move_to(pts[0]), |path, q| path.line_to(*q))
}

impl SatinGeometry {
    /// A turning column from a closed outline and stitch angle lines
    /// ("complex turning"). The first and the last angle line cut the
    /// outline into the two rails; all angle lines become rungs. `None` if
    /// there are fewer than two angle lines or the outline has no area.
    pub fn from_outline(outline: &Path, angle_lines: &[(Point, Point)]) -> Option<SatinGeometry> {
        let line = outline.flatten(CENTERLINE_TOL_MM).into_iter().next()?;
        let ring = Ring::new(line.points);
        if ring.pts.len() < 3 || angle_lines.len() < 2 || ring.len() <= 0.0 {
            return None;
        }
        let (first, last) = (angle_lines[0], angle_lines[angle_lines.len() - 1]);
        let (a0, b0) = (ring.project(first.0), ring.project(first.1));
        let (a1, b1) = (ring.project(last.0), ring.project(last.1));
        let l = ring.len();
        // Rail A goes from a0 to a1 the way that does not pass b0.
        let between = |from: f64, to: f64, x: f64, forward: bool| {
            let (span, d) = if forward {
                ((to - from).rem_euclid(l), (x - from).rem_euclid(l))
            } else {
                ((from - to).rem_euclid(l), (from - x).rem_euclid(l))
            };
            d > 1e-9 && d < span - 1e-9
        };
        let fwd_a = !between(a0, a1, b0, true);
        let fwd_b = !between(b0, b1, a0, true);
        let rail_a = ring.arc(a0, a1, fwd_a);
        let rail_b = ring.arc(b0, b1, fwd_b);
        if rail_a.len() < 2 || rail_b.len() < 2 {
            return None;
        }
        Some(SatinGeometry {
            rail_a: polyline_path(&rail_a),
            rail_b: polyline_path(&rail_b),
            rungs: angle_lines.to_vec(),
        })
    }

    /// The column with new stitch angle lines from a direction line: lines
    /// at `angle_deg` to `line` (90°: square to it), about `step` apart
    /// along it, from rail to rail (`emb_geom::spine::cross_lines`). The
    /// rails stay. `None` if fewer than two lines cross the column.
    pub fn with_direction(&self, line: &[Point], angle_deg: f64, step: f64) -> Option<SatinGeometry> {
        let flat = |p: &Path| p.flatten(CENTERLINE_TOL_MM).into_iter().next();
        let (a, b) = (flat(&self.rail_a)?, flat(&self.rail_b)?);
        // The area of the column: a ring column has two closed rails.
        let rings: Vec<Vec<Point>> = if a.closed && b.closed {
            vec![a.points.clone(), b.points.clone()]
        } else {
            vec![a.points.iter().chain(b.points.iter().rev()).copied().collect()]
        };
        let near = |pts: &[Point], p: Point| {
            pts.windows(2)
                .map(|w| p.distance_to_segment(w[0], w[1]))
                .fold(f64::INFINITY, f64::min)
        };
        let lines: Vec<(Point, Point)> = emb_geom::spine::cross_lines(&rings, line, angle_deg, step)
            .into_iter()
            // The first point of a rung is on rail A.
            .map(|(l, r)| {
                if near(&a.points, l) <= near(&a.points, r) {
                    (l, r)
                } else {
                    (r, l)
                }
            })
            .collect();
        (lines.len() >= 2).then(|| SatinGeometry {
            rungs: lines,
            ..self.clone()
        })
    }

    /// As [`SatinGeometry::from_centerline`], but at corners sharper than
    /// 45° the outer rail is cut off straight (cap corner) and the inner
    /// rail comes to its point.
    pub fn from_centerline_capped(center: &Path, width: f64) -> SatinGeometry {
        let Some(line) = center.flatten(CENTERLINE_TOL_MM).into_iter().next() else {
            return SatinGeometry::default();
        };
        let mut pts = line.points;
        pts.dedup();
        let closed = line.closed && pts.len() > 2;
        let n = pts.len();
        if n < 2 {
            return SatinGeometry::default();
        }
        let half = width.abs() / 2.0;
        let seg = |i: usize| (pts[(i + 1) % n] - pts[i]).normalized().unwrap_or_default();
        let (mut a, mut b, mut rungs) = (Vec::new(), Vec::new(), Vec::new());
        for (i, &pt) in pts.iter().enumerate() {
            let end = !closed && (i == 0 || i == n - 1);
            let (d0, d1) = match (closed, i) {
                (false, 0) => (seg(0), seg(0)),
                (false, i) if i == n - 1 => (seg(n - 2), seg(n - 2)),
                (_, i) => (seg((i + n - 1) % n), seg(i)),
            };
            let (n0, n1) = (d0.perp(), d1.perp());
            if !end && d0.dot(d1) < CAP_COS {
                // The turn is toward rail B if the next direction goes to its side.
                let to_b = d1.dot(n0) > 0.0;
                let m = (n0 + n1).normalized().unwrap_or(n1);
                let inner = m * (half / m.dot(n1).max(1.0 / MAX_MITER));
                let (o0, o1) = (n0 * half, n1 * half);
                if to_b {
                    // Rail A is outside: two points; rail B: the inner point.
                    let bi = pt + inner;
                    for o in [o0, o1] {
                        a.push(pt - o);
                        rungs.push((pt - o, bi));
                    }
                    b.push(bi);
                } else {
                    let ai = pt - inner;
                    for o in [o0, o1] {
                        b.push(pt + o);
                        rungs.push((ai, pt + o));
                    }
                    a.push(ai);
                }
                continue;
            }
            let mm = (n0 + n1).normalized().unwrap_or(n1);
            let off = mm * (half / mm.dot(n1).max(1.0 / MAX_MITER));
            a.push(pt - off);
            b.push(pt + off);
        }
        if closed {
            a.push(a[0]);
            b.push(b[0]);
        }
        SatinGeometry {
            rail_a: polyline_path(&a),
            rail_b: polyline_path(&b),
            rungs,
        }
    }

    /// A column of fixed `width` along a centerline ("Column C"). Rail A is
    /// on the left of the line direction (on screen, Y down), rail B on the
    /// right. Rungs at the corners of the flattened line and every
    /// `RUNG_STEP_MM` keep the stitches straight across. Only the first subpath is used.
    pub fn from_centerline(center: &Path, width: f64) -> SatinGeometry {
        let Some(line) = center.flatten(CENTERLINE_TOL_MM).into_iter().next() else {
            return SatinGeometry::default();
        };
        let mut pts = line.points;
        pts.dedup();
        let closed = line.closed && pts.len() > 2;
        let n = pts.len();
        if n < 2 {
            return SatinGeometry::default();
        }
        let half = width.abs() / 2.0;
        // Unit normal of segment i (from point i to point i + 1), pointing to rail B.
        let seg_normal = |i: usize| -> Point {
            let d = pts[(i + 1) % n] - pts[i];
            d.perp().normalized().unwrap_or_default()
        };
        let mut a = Vec::with_capacity(n + 1);
        let mut b = Vec::with_capacity(n + 1);
        let mut rungs = Vec::new();
        let mut since_rung = 0.0;
        for i in 0..n {
            if i > 0 {
                since_rung += pts[i].distance(pts[i - 1]);
            }
            let (prev, next) = match (closed, i) {
                (false, 0) => (seg_normal(0), seg_normal(0)),
                (false, i) if i == n - 1 => (seg_normal(n - 2), seg_normal(n - 2)),
                (_, i) => (seg_normal((i + n - 1) % n), seg_normal(i)),
            };
            // Miter: the offset at a corner is longer, so the column keeps its width.
            let sum = prev + next;
            let m = sum.normalized().unwrap_or(next);
            let cos_half = m.dot(next).max(1.0 / MAX_MITER);
            let off = m * (half / cos_half);
            a.push(pts[i] - off);
            b.push(pts[i] + off);
            let end = !closed && (i == 0 || i == n - 1);
            if !end && (prev.dot(next) < 0.95 || since_rung >= RUNG_STEP_MM) {
                rungs.push((pts[i] - off, pts[i] + off));
                since_rung = 0.0;
            }
        }
        if closed {
            a.push(a[0]);
            b.push(b[0]);
        }
        let to_path = |p: &[Point]| {
            p[1..]
                .iter()
                .fold(Path::new().move_to(p[0]), |path, q| path.line_to(*q))
        };
        SatinGeometry {
            rail_a: to_path(&a),
            rail_b: to_path(&b),
            rungs,
        }
    }
}

/// Parameters of a running stitch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunningParams {
    pub stitch_length_mm: f64,
    /// 1 for a single run, 3 for a bean (triple) stitch.
    pub repeats: u32,
    pub style: RunStyle,
    pub pass: RunPass,
    /// With a border, the line sews as a column of fixed width along it (an
    /// outline stitch type such as satin, E stitch or coil). The run
    /// settings are not used then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<LineBorder>,
    /// With a motif, the line sews as the motif repeated along it (motif
    /// run). The run settings and the border are not used then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motif: Option<MotifRun>,
    /// Hand-stitch effect: needle points move at random by up to this
    /// much, so the line looks sewn by hand (0: off).
    #[serde(skip_serializing_if = "is_zero")]
    pub hand_mm: f64,
    /// Cording: a run to show where the cord goes, a stop to lay the cord,
    /// then a zigzag that holds the cord down.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cording: bool,
    /// With a ripple, the line sews as copies of itself that get smaller
    /// round a point or move along a guide (P11 Milestone 4). The border
    /// and the motif are not used then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ripple: Option<Ripple>,
    /// A cut line for a cutting needle (P11 Milestone 8, ADR 0051): the
    /// needle points are the cuts, with a trim between the parts (a travel
    /// stitch would cut the fabric too).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// A ripple stitch: lines that repeat the shape of a line.
///
/// A closed line makes rings that get smaller round `target` and are sewn
/// as one spiral. An open line makes copies that are sewn back and forth.
/// With a guide, the copies move along the guide (the first copy is the
/// line, the last is at the end of the guide).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ripple {
    /// The number of lines, the line itself included.
    pub lines: u32,
    /// The size of the last line, in percent of the line.
    pub end_percent: f64,
    /// The point that the lines get smaller round (design coordinates);
    /// with none, the middle of the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Point>,
    /// The guide line (design coordinates).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guide: Vec<Point>,
}

impl Default for Ripple {
    fn default() -> Self {
        Ripple {
            lines: 10,
            end_percent: 0.0,
            target: None,
            guide: Vec::new(),
        }
    }
}

impl Ripple {
    /// Most lines of a ripple.
    pub const MAX_LINES: u32 = 500;
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Default for RunningParams {
    fn default() -> Self {
        Self {
            stitch_length_mm: 2.5,
            repeats: 1,
            style: RunStyle::Run,
            pass: RunPass::Once,
            border: None,
            motif: None,
            hand_mm: 0.0,
            cording: false,
            ripple: None,
            cut: false,
        }
    }
}

/// A line sewn as a column of fixed width: the line is the middle of the column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineBorder {
    pub width_mm: f64,
    #[serde(default)]
    pub column: SatinParams,
    /// How the column goes round sharp corners of the line.
    #[serde(default, skip_serializing_if = "Corner::is_mitre")]
    pub corner: Corner,
}

/// How a column of fixed width goes round a sharp corner of its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    /// The outer edge comes to a point (cut off when the corner is very sharp).
    #[default]
    Mitre,
    /// The outer edge is cut off straight.
    Cap,
    /// The column stops at the corner and a new column starts over it.
    Lap,
}

impl Corner {
    pub fn is_mitre(&self) -> bool {
        *self == Corner::Mitre
    }
}

impl LineBorder {
    /// A border of `width_mm` with the column type `pattern` and its default spacing.
    pub fn new(width_mm: f64, pattern: ColumnPattern) -> LineBorder {
        LineBorder {
            width_mm,
            column: SatinParams::with_pattern(pattern),
            corner: Corner::Mitre,
        }
    }
}

/// How the needle points of a running stitch are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStyle {
    /// Even stitches along the line.
    #[default]
    Run,
    /// Two half steps forward, one back, like a hand backstitch.
    Backstitch,
    /// A backstitch with the forward stitches a little to one side, like a hand stem stitch.
    Stemstitch,
    /// The points of the path are the needle points (manual stitches).
    Manual,
    /// Triple stitches and single stitches in turn, so the line looks like
    /// a string of beads.
    Sculpture,
}

/// How many times, and in which order, the line is sewn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPass {
    #[default]
    Once,
    /// Forward, then back along the same line.
    Backtrack,
    /// Two times in the same direction (a closed line); an open line is backtracked.
    Repeat,
    /// All lines of the object sewn as a network: each line two times, and the
    /// object ends where it starts.
    Redwork,
    /// All lines of the object sewn as a network with the fewest lines sewn two times.
    Branch,
}

/// Parameters of a satin column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SatinParams {
    /// Distance between stitches, measured along the column.
    pub spacing_mm: f64,
    /// Extra width on each side.
    pub pull_compensation_mm: f64,
    /// Shorten every second stitch on the inside of tight curves.
    pub short_stitches: bool,
    pub underlay: Underlay,
    /// Stitches longer than this get needle points along them (auto split).
    /// A small value makes a turning fill.
    pub split_mm: f64,
    /// How the column is stitched. The settings above apply to the satin
    /// types; see each type for what `spacing_mm` sets.
    #[serde(skip_serializing_if = "ColumnPattern::is_satin")]
    pub pattern: ColumnPattern,
    /// Liquid effect: the stitch angle changes evenly along the column, also
    /// where the stitch angle lines change it sharply.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub liquid: bool,
    /// The motif of a motif column (the default motif if none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motif: Option<Motif>,
    /// The pattern of a program-split column (the default pattern if none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_pattern: Option<SplitPattern>,
    /// Auto jump: no splits; stitches longer than the machine allows go as
    /// jumps, so the thread lies long (for machines that cut long stitches).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub auto_jump: bool,
    /// Fractional spacing: where across the column the spacing is measured
    /// (0: rail A, 0.5: the middle, 1: rail B).
    #[serde(default = "half", skip_serializing_if = "is_half")]
    pub spacing_at: f64,
    /// Auto spacing: narrow parts of the column get more open spacing, wide
    /// parts denser spacing.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub auto_spacing: bool,
    /// Accordion spacing: the spacing changes along the column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accordion: Option<Accordion>,
    /// Jagged edge: needle points on the rails move in at random by up to
    /// this much (fur, grass); 0: off.
    #[serde(skip_serializing_if = "is_zero")]
    pub jagged_mm: f64,
    /// Hand-stitch effect: needle points move at random by up to this much.
    #[serde(skip_serializing_if = "is_zero")]
    pub hand_mm: f64,
    /// Pull compensation as a percent of the width, on each side, added to
    /// `pull_compensation_mm`.
    #[serde(skip_serializing_if = "is_zero")]
    pub pull_percent: f64,
    /// Pull compensation of rail B, if it is not the same as rail A.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_b_mm: Option<f64>,
    /// Omit the last stitch of the cover, so the column ends on the other
    /// rail (a smoother join with a column that starts there).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub omit_last: bool,
    /// Puff (3D foam, P11 Milestone 7): the column sews over foam. Its
    /// underlay has no rows across the column (they would press the foam
    /// flat), stitches go over each end to cut the foam there, and the
    /// machine stops before it to put the foam on (ADR 0050).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub puff: bool,
}

/// The satin spacing of puff columns, in mm: dense, so the foam does not show.
pub const PUFF_SPACING_MM: f64 = 0.25;

fn is_half(v: &f64) -> bool {
    *v == 0.5
}

/// Accordion spacing: the spacing as a factor (1: the set spacing, larger:
/// more open) at points from the start (0) to the end (1), straight in
/// between.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accordion {
    pub points: Vec<(f64, f64)>,
}

impl Accordion {
    /// The preset profiles, with their names.
    pub fn presets() -> Vec<(&'static str, Accordion)> {
        let a = |pts: &[(f64, f64)]| Accordion { points: pts.to_vec() };
        vec![
            ("Fade out", a(&[(0.0, 1.0), (1.0, 6.0)])),
            ("Fade in", a(&[(0.0, 6.0), (1.0, 1.0)])),
            ("Dense middle", a(&[(0.0, 6.0), (0.5, 1.0), (1.0, 6.0)])),
            ("Dense ends", a(&[(0.0, 1.0), (0.5, 6.0), (1.0, 1.0)])),
            (
                "Steps",
                a(&[
                    (0.0, 1.0),
                    (0.33, 1.0),
                    (0.34, 3.0),
                    (0.66, 3.0),
                    (0.67, 6.0),
                    (1.0, 6.0),
                ]),
            ),
        ]
    }

    /// The spacing factor at `t` (0 to 1), at least 1.
    pub fn factor(&self, t: f64) -> f64 {
        let pts = &self.points;
        let f = match pts.len() {
            0 => 1.0,
            1 => pts[0].1,
            _ => {
                let i = pts.partition_point(|p| p.0 <= t).clamp(1, pts.len() - 1);
                let (a, b) = (pts[i - 1], pts[i]);
                if b.0 > a.0 {
                    a.1 + (b.1 - a.1) * ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)
                } else {
                    b.1
                }
            }
        };
        f.max(1.0)
    }
}

/// The stitch type of a column (a satin column, or a line with a border).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ColumnPattern {
    /// Zigzag from rail to rail; `spacing_mm` is the distance between
    /// needle points on one side.
    #[default]
    Satin,
    /// Open satin: zigzag at the spacing, with no short stitches and no splits.
    Zigzag,
    /// Blanket stitch: a run along rail A, and a stitch across to rail B
    /// and back each `spacing_mm`.
    EStitch,
    /// Satin over lower satin layers, for a raised look. `layers` counts
    /// all layers, the top one too.
    Raised { layers: u32 },
    /// Rows across the column, each split into stitches of `stitch_mm` in
    /// a staggered pattern. `spacing_mm` is the distance of two rows.
    Tatami { stitch_mm: f64 },
    /// Square teeth: a stitch across, a step of `spacing_mm` along the
    /// rail, a stitch back, a step along the other rail.
    Square,
    /// Overlapping loops across the column, like a coiled spring seen from
    /// above. Each loop moves on by `spacing_mm`.
    Coil,
    /// Running lines along the column that follow its shape, about
    /// `spacing_mm` apart where the column has its mean width.
    Contour { stitch_mm: f64 },
    /// String: slanted stitches from rail A to rail B, each `chord_mm`
    /// farther on, which overlap like twisted string.
    String { chord_mm: f64 },
    /// The motif of `SatinParams::motif` repeated along the column, as high
    /// as the column is wide; each repeat is `aspect` times as long as high.
    Motif { aspect: f64 },
    /// Rows across the column split where the program-split pattern of
    /// `SatinParams::split_pattern` puts needle points; the pattern bends
    /// with the column (flexi split). `scale` sizes the pattern tile.
    ProgramSplit { scale: f64 },
}

impl ColumnPattern {
    /// All column types, in menu order, with their default settings.
    pub const ALL: [ColumnPattern; 11] = [
        ColumnPattern::Satin,
        ColumnPattern::Zigzag,
        ColumnPattern::EStitch,
        ColumnPattern::Raised { layers: 2 },
        ColumnPattern::Tatami { stitch_mm: 3.5 },
        ColumnPattern::Square,
        ColumnPattern::Coil,
        ColumnPattern::Contour { stitch_mm: 2.0 },
        ColumnPattern::String { chord_mm: 4.0 },
        ColumnPattern::Motif { aspect: 1.0 },
        ColumnPattern::ProgramSplit { scale: 1.0 },
    ];

    pub fn is_satin(&self) -> bool {
        *self == ColumnPattern::Satin
    }

    pub fn name(&self) -> &'static str {
        match self {
            ColumnPattern::Satin => "Satin",
            ColumnPattern::Zigzag => "Zigzag",
            ColumnPattern::EStitch => "E stitch",
            ColumnPattern::Raised { .. } => "Raised satin",
            ColumnPattern::Tatami { .. } => "Tatami",
            ColumnPattern::Square => "Square",
            ColumnPattern::Coil => "Coil",
            ColumnPattern::Contour { .. } => "Contour",
            ColumnPattern::String { .. } => "String",
            ColumnPattern::Motif { .. } => "Motif column",
            ColumnPattern::ProgramSplit { .. } => "Program split",
        }
    }

    /// The same type, without its settings (for menus).
    pub fn same_type(&self, o: &ColumnPattern) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(o)
    }

    /// A good `spacing_mm` for this type.
    pub fn default_spacing_mm(&self) -> f64 {
        match self {
            ColumnPattern::Satin => 0.4,
            ColumnPattern::Contour { .. } => 0.6,
            ColumnPattern::Raised { .. } => 0.5,
            ColumnPattern::Tatami { .. } => 0.8,
            ColumnPattern::Zigzag | ColumnPattern::Square => 1.5,
            ColumnPattern::EStitch => 2.0,
            ColumnPattern::Coil => 1.2,
            ColumnPattern::String { .. } => 0.8,
            ColumnPattern::Motif { .. } => 2.0,
            ColumnPattern::ProgramSplit { .. } => 0.4,
        }
    }

    /// Open types: no underlay by default (it would show).
    pub fn is_open(&self) -> bool {
        matches!(
            self,
            ColumnPattern::Zigzag
                | ColumnPattern::EStitch
                | ColumnPattern::Square
                | ColumnPattern::Coil
                | ColumnPattern::Contour { .. }
                | ColumnPattern::String { .. }
                | ColumnPattern::Motif { .. }
        )
    }
}

/// Auto split length of a normal satin column.
pub const SATIN_SPLIT_MM: f64 = 7.0;

impl Default for SatinParams {
    fn default() -> Self {
        Self {
            spacing_mm: 0.4,
            pull_compensation_mm: 0.2,
            short_stitches: true,
            underlay: Underlay::Auto,
            split_mm: SATIN_SPLIT_MM,
            pattern: ColumnPattern::Satin,
            liquid: false,
            motif: None,
            split_pattern: None,
            auto_jump: false,
            spacing_at: 0.5,
            auto_spacing: false,
            accordion: None,
            jagged_mm: 0.0,
            hand_mm: 0.0,
            pull_percent: 0.0,
            pull_b_mm: None,
            omit_last: false,
            puff: false,
        }
    }
}

impl SatinParams {
    /// Default settings for the column type `pattern`. The open types
    /// (zigzag, E stitch, square, coil, contour) have no underlay.
    pub fn with_pattern(pattern: ColumnPattern) -> Self {
        let open = pattern.is_open();
        Self {
            spacing_mm: pattern.default_spacing_mm(),
            short_stitches: pattern.is_satin(),
            underlay: if open { Underlay::None } else { Underlay::Auto },
            pattern,
            ..Self::default()
        }
    }

    /// Settings for a turning fill ("complex turning"): rows 0.4 mm apart,
    /// split into tatami-like stitches of 3.5 mm.
    pub fn turning() -> Self {
        Self {
            spacing_mm: 0.8,
            short_stitches: false,
            split_mm: 3.5,
            ..Self::default()
        }
    }
}

/// Parameters of a tatami fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FillParams {
    /// Distance between rows.
    pub spacing_mm: f64,
    /// Row direction in degrees. 0° is along +X; positive turns clockwise on screen.
    pub angle_deg: f64,
    pub stitch_length_mm: f64,
    /// Extra size at the ends of the rows.
    pub pull_compensation_mm: f64,
    pub underlay: Underlay,
    /// How the shape is filled. `spacing_mm` is the distance between the
    /// rows or lines of each type; the angle is used by tatami, contour and
    /// Florentine.
    #[serde(skip_serializing_if = "FillPattern::is_tatami")]
    pub pattern: FillPattern,
    /// Carving stamps pressed into the fill (tatami and program split).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stamps: Vec<StampPlacement>,
    /// Trapunto: travel goes along the edge, not across the inside, so the
    /// inside can be padded.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub trapunto: bool,
    /// Accordion spacing: the row spacing changes across the rows (tatami
    /// rows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accordion: Option<Accordion>,
    /// Jagged edge: row ends move in at random by up to this much; 0: off.
    #[serde(skip_serializing_if = "is_zero")]
    pub jagged_mm: f64,
    /// Hand-stitch effect: needle points move at random by up to this much.
    #[serde(skip_serializing_if = "is_zero")]
    pub hand_mm: f64,
    /// A direction line for the column types (satin and the others): the
    /// stitches go at its angle to the line at each place, so they turn
    /// with the line. Without it, they go at `angle_deg`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<DirectionLine>,
}

/// A stitch direction line (beta tester request, 2026-10-24): the user
/// draws a line through a shape, and the stitches go across it at
/// `angle_deg` (90°: square to the line) at each place along it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectionLine {
    /// The line, in design coordinates.
    pub points: Vec<Point>,
    /// The angle of the stitches to the line, in degrees (0° to 180°).
    #[serde(default = "right_angle")]
    pub angle_deg: f64,
}

impl Default for FillParams {
    fn default() -> Self {
        Self {
            spacing_mm: 0.4,
            angle_deg: 45.0,
            stitch_length_mm: 3.5,
            pull_compensation_mm: 0.0,
            underlay: Underlay::Auto,
            pattern: FillPattern::Tatami,
            stamps: Vec::new(),
            trapunto: false,
            accordion: None,
            jagged_mm: 0.0,
            hand_mm: 0.0,
            direction: None,
        }
    }
}

impl FillParams {
    /// Default settings for the fill type `pattern`, with the angle and the
    /// underlay of `self`. The open types have no underlay, because it
    /// would show between their lines.
    pub fn with_pattern(&self, pattern: FillPattern) -> Self {
        let (spacing_mm, stitch_length_mm) = pattern.defaults();
        let keeps_pull = matches!(
            pattern,
            FillPattern::Tatami
                | FillPattern::Florentine { .. }
                | FillPattern::Square
                | FillPattern::DoubleSquare { .. }
                | FillPattern::ProgramSplit { .. }
                | FillPattern::FlexiSplit { .. }
                | FillPattern::UserSplit { .. }
                | FillPattern::Tartan { .. }
                | FillPattern::Gradient { .. }
        ) || matches!(&pattern, FillPattern::Columns { column } if !column.is_open());
        Self {
            spacing_mm,
            stitch_length_mm,
            angle_deg: self.angle_deg,
            pull_compensation_mm: if keeps_pull { self.pull_compensation_mm } else { 0.0 },
            // A gradient alone leaves rows out, so its underlay would show
            // (the first fill of a gradient of several colors gets it back).
            underlay: if pattern.is_open() || matches!(pattern, FillPattern::Gradient { .. }) {
                Underlay::None
            } else {
                self.underlay.clone()
            },
            stamps: if pattern.takes_stamps() {
                self.stamps.clone()
            } else {
                Vec::new()
            },
            // The direction line is for the column types only.
            direction: if matches!(pattern, FillPattern::Columns { .. } | FillPattern::FlexiSplit { .. }) {
                self.direction.clone()
            } else {
                None
            },
            pattern,
            trapunto: self.trapunto,
            accordion: self.accordion.clone(),
            jagged_mm: self.jagged_mm,
            hand_mm: self.hand_mm,
        }
    }
}

/// The legs of a cross stitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossStyle {
    /// Two diagonal legs: X.
    #[default]
    Full,
    /// One diagonal leg: /.
    Diagonal,
    /// A level and an upright leg: +.
    Upright,
}

/// The stitch type of a fill.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FillPattern {
    /// Straight rows at the fill angle, split into stitches in a pattern.
    #[default]
    Tatami,
    /// Stitches across the shape at the fill angle, as on a column of this
    /// type (satin, zigzag, E stitch, raised satin, coil). The two sides of
    /// the column are the two halves of the outline. `stitch_length_mm` is
    /// the split length of satin.
    Columns { column: ColumnPattern },
    /// Straight rows at the fill angle with the needle points in line, so the
    /// rows make a grid of even stitches.
    Square,
    /// Two layers of square stitching; the second turned by `angle_deg`.
    DoubleSquare {
        #[serde(default = "right_angle")]
        angle_deg: f64,
    },
    /// Lines that follow the shape from one end to the other, along the fill
    /// angle: each line keeps a fraction of the way from one side to the other.
    Contour,
    /// Rings inside the edge, one row apart, from the edge to the middle.
    Offset,
    /// One line that turns from the edge to the middle, one row apart.
    Spiral,
    /// Rings of coil loops from the edge to the middle; `spacing_mm` is the
    /// width of each ring.
    IslandCoil,
    /// Lines from a center point to the edge. `center` is in design
    /// coordinates; with none, the middle of the shape.
    Radial {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        center: Option<Point>,
    },
    /// An open double spiral: to the middle, then back out between the rows.
    /// It never crosses itself and ends near its start.
    Maze,
    /// Rows that bend like the guide line (design coordinates). With no
    /// guide, the rows are straight (as tatami).
    Florentine {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        guide: Vec<Point>,
    },
    /// String art: straight lines from each point of the edge to a point a
    /// third of the way round, which overlap in curves. `spacing_mm` is the
    /// distance of the points on the edge.
    String,
    /// Crosses on a grid of `cell_mm` squares. The grid starts at the design
    /// origin, so crosses of all objects line up. Each leg is sewn `passes`
    /// times (1 or 3).
    CrossStitch {
        cell_mm: f64,
        #[serde(default)]
        style: CrossStyle,
        #[serde(default = "one")]
        passes: u32,
    },
    /// A meandering line that fills the shape and never crosses itself;
    /// `spacing_mm` is the distance of its rows. `style` is run, backstitch
    /// or stem stitch.
    Stipple {
        #[serde(default)]
        style: RunStyle,
    },
    /// Rows of motifs at the fill angle: each repeat `length_mm` long and
    /// `height_mm` high, rows `spacing_mm` apart, every second row moved on
    /// by `stagger` of a repeat. `gradient` is the size of the last row
    /// against the first (1: all the same); `globe` bends the rows like a
    /// ball (3D warp).
    Motif {
        motif: Motif,
        length_mm: f64,
        height_mm: f64,
        #[serde(default = "half")]
        stagger: f64,
        #[serde(default = "one_f")]
        gradient: f64,
        #[serde(default)]
        globe: bool,
    },
    /// Tatami rows with the needle points where the program-split pattern
    /// puts them, so they make a picture. `scale` sizes the pattern tile.
    ProgramSplit {
        #[serde(default)]
        pattern: SplitPattern,
        #[serde(default = "one_f")]
        scale: f64,
    },
    /// A program-split pattern that bends with the shape: stitches across
    /// the shape at the fill angle, as a column between the two halves of
    /// the outline, split by the pattern.
    FlexiSplit {
        #[serde(default)]
        pattern: SplitPattern,
        #[serde(default = "one_f")]
        scale: f64,
    },
    /// Rows at the fill angle with the needle points where they cross the
    /// split lines (design coordinates); longer stitches get needle points
    /// at the stitch length.
    UserSplit {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lines: Vec<Vec<Point>>,
    },
    /// A light fill under the design that holds down the pile of towels and
    /// fur: `layers` (1 or 2) of open rows, the second turned by `angle_deg`.
    Laydown {
        #[serde(default = "one")]
        layers: u32,
        #[serde(default = "right_angle")]
        angle_deg: f64,
    },
    /// Tartan (P11 Milestone 4, ADR 0047): tatami rows in the stripes of
    /// the sett color `color`, at the fill angle (the warp) and square to
    /// it (the weft), each direction `spacing_mm` apart. The sett is
    /// centered on the middle of the shape. A full tartan is a group of
    /// fills with the same sett, one for each color.
    Tartan {
        #[serde(default)]
        sett: crate::tartan::Sett,
        #[serde(default)]
        color: u32,
    },
    /// Linear gradient: tatami rows at the fill angle, each row of one color
    /// of the gradient. `stops` are the places of the colors across the
    /// rows (0 at the start, 1 at the end of `line`; with no line, from
    /// one side of the shape to the other). Between two stops, more and
    /// more rows go to the second color, so the colors mix and the density
    /// stays the same. The object sews the rows of color `color`; a full
    /// gradient is a group of fills, one for each color (ADR 0047).
    Gradient {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<(Point, Point)>,
        #[serde(default = "zero_one")]
        stops: Vec<f64>,
        #[serde(default)]
        color: u32,
    },
    /// Meander: one line along the edges of a tile pattern, `spacing_mm`
    /// from one corner of a tile to the next, that fills the shape and never
    /// crosses itself. The pattern turns with the fill angle.
    Meander {
        #[serde(default)]
        tile: MeanderTile,
    },
    /// Circular: rings round a center point (design coordinates; with
    /// none, the middle of the shape), `spacing_mm` apart. Where the rings
    /// are whole, they are one spiral.
    Circular {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        center: Option<Point>,
    },
    /// Guided: rows that follow guide lines (design coordinates). Between
    /// two guide lines the rows change from the shape of one to the shape
    /// of the other; past the outer guide lines the rows are copies of
    /// them. With no guide, the rows are straight at the fill angle.
    Guided {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        guides: Vec<Vec<Point>>,
    },
    /// Lace mesh (P11 Milestone 5): the net of free-standing lace. Open
    /// rows `spacing_mm` apart at the fill angle, and a second layer turned
    /// by `angle_deg`. Each row goes there and back on the same needle
    /// points, so it is two threads that lock where the rows cross.
    Mesh {
        #[serde(default = "right_angle")]
        angle_deg: f64,
    },
}

/// The tile pattern of a meander fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeanderTile {
    /// Squares: a line of square turns.
    #[default]
    Square,
    /// Triangles: a line of sharp turns.
    Triangle,
    /// Hexagons, as a honeycomb: a line of wide turns.
    Hexagon,
}

impl MeanderTile {
    pub const ALL: [MeanderTile; 3] = [MeanderTile::Square, MeanderTile::Triangle, MeanderTile::Hexagon];

    pub fn name(self) -> &'static str {
        match self {
            MeanderTile::Square => "Squares",
            MeanderTile::Triangle => "Triangles",
            MeanderTile::Hexagon => "Hexagons",
        }
    }
}

fn zero_one() -> Vec<f64> {
    vec![0.0, 1.0]
}

fn right_angle() -> f64 {
    90.0
}

fn one() -> u32 {
    1
}

fn one_f() -> f64 {
    1.0
}

fn half() -> f64 {
    0.5
}

impl FillPattern {
    /// All fill types, in menu order, with their default settings.
    pub fn all() -> Vec<FillPattern> {
        let mut v = vec![FillPattern::Tatami];
        v.extend(
            [
                ColumnPattern::Satin,
                ColumnPattern::Zigzag,
                ColumnPattern::EStitch,
                ColumnPattern::Raised { layers: 2 },
                ColumnPattern::Coil,
            ]
            .map(|column| FillPattern::Columns { column }),
        );
        v.extend([
            FillPattern::Square,
            FillPattern::DoubleSquare { angle_deg: 90.0 },
            FillPattern::Contour,
            FillPattern::Offset,
            FillPattern::Spiral,
            FillPattern::IslandCoil,
            FillPattern::Radial { center: None },
            FillPattern::Maze,
            FillPattern::Florentine { guide: Vec::new() },
            FillPattern::String,
            FillPattern::CrossStitch {
                cell_mm: 2.5,
                style: CrossStyle::Full,
                passes: 1,
            },
            FillPattern::Stipple { style: RunStyle::Run },
            FillPattern::Motif {
                motif: Motif::default(),
                length_mm: 6.0,
                height_mm: 4.0,
                stagger: 0.5,
                gradient: 1.0,
                globe: false,
            },
            FillPattern::ProgramSplit {
                pattern: SplitPattern::default(),
                scale: 1.0,
            },
            FillPattern::FlexiSplit {
                pattern: SplitPattern::default(),
                scale: 1.0,
            },
            FillPattern::UserSplit { lines: Vec::new() },
            FillPattern::Laydown {
                layers: 1,
                angle_deg: 90.0,
            },
            FillPattern::Tartan {
                sett: crate::tartan::Sett::default(),
                color: 0,
            },
            FillPattern::Gradient {
                line: None,
                stops: zero_one(),
                color: 0,
            },
            FillPattern::Meander {
                tile: MeanderTile::Square,
            },
            FillPattern::Circular { center: None },
            FillPattern::Guided { guides: Vec::new() },
            FillPattern::Mesh { angle_deg: 90.0 },
        ]);
        v
    }

    /// Fill types that carving stamps can be pressed into (rows of tatami).
    pub fn takes_stamps(&self) -> bool {
        matches!(self, FillPattern::Tatami | FillPattern::ProgramSplit { .. })
    }

    pub fn is_tatami(&self) -> bool {
        *self == FillPattern::Tatami
    }

    /// Open types: the fabric shows between the lines, so they have no underlay.
    pub fn is_open(&self) -> bool {
        match self {
            FillPattern::Columns { column } => column.is_open(),
            FillPattern::IslandCoil
            | FillPattern::Maze
            | FillPattern::String
            | FillPattern::CrossStitch { .. }
            | FillPattern::Stipple { .. }
            | FillPattern::Motif { .. }
            | FillPattern::Laydown { .. }
            | FillPattern::Meander { .. }
            | FillPattern::Mesh { .. } => true,
            _ => false,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FillPattern::Tatami => "Tatami",
            FillPattern::Columns { column } => column.name(),
            FillPattern::Square => "Square",
            FillPattern::DoubleSquare { .. } => "Double square",
            FillPattern::Contour => "Contour",
            FillPattern::Offset => "Offset",
            FillPattern::Spiral => "Spiral",
            FillPattern::IslandCoil => "Island coil",
            FillPattern::Radial { .. } => "Radial",
            FillPattern::Maze => "Maze",
            FillPattern::Florentine { .. } => "Florentine",
            FillPattern::String => "String",
            FillPattern::CrossStitch { .. } => "Cross stitch",
            FillPattern::Stipple { .. } => "Stipple",
            FillPattern::Motif { .. } => "Motif fill",
            FillPattern::ProgramSplit { .. } => "Program split",
            FillPattern::FlexiSplit { .. } => "Flexi split",
            FillPattern::UserSplit { .. } => "User split",
            FillPattern::Laydown { .. } => "Laydown",
            FillPattern::Tartan { .. } => "Tartan",
            FillPattern::Gradient { .. } => "Gradient",
            FillPattern::Meander { .. } => "Meander",
            FillPattern::Circular { .. } => "Circular",
            FillPattern::Guided { .. } => "Guided",
            FillPattern::Mesh { .. } => "Lace mesh",
        }
    }

    /// The same type, without its settings (for menus). The column fills
    /// are a type for each column type.
    pub fn same_type(&self, o: &FillPattern) -> bool {
        match (self, o) {
            (FillPattern::Columns { column: a }, FillPattern::Columns { column: b }) => a.same_type(b),
            _ => std::mem::discriminant(self) == std::mem::discriminant(o),
        }
    }

    /// A good row spacing and stitch length for this type, in mm.
    pub fn defaults(&self) -> (f64, f64) {
        match self {
            FillPattern::Tatami | FillPattern::Florentine { .. } => (0.4, 3.5),
            FillPattern::Columns { column } => (column.default_spacing_mm(), SATIN_SPLIT_MM),
            FillPattern::Square => (0.5, 3.0),
            FillPattern::DoubleSquare { .. } => (1.0, 3.0),
            FillPattern::Contour | FillPattern::Offset | FillPattern::Spiral => (0.4, 2.5),
            FillPattern::Radial { .. } => (0.4, 3.0),
            FillPattern::IslandCoil => (2.0, 1.5),
            FillPattern::Maze => (1.5, 2.0),
            FillPattern::String => (1.5, 4.0),
            FillPattern::CrossStitch { .. } => (2.5, 3.0),
            FillPattern::Stipple { .. } => (1.5, 1.5),
            FillPattern::Motif { .. } => (6.0, 2.0),
            FillPattern::ProgramSplit { .. } | FillPattern::FlexiSplit { .. } => (0.4, 7.0),
            FillPattern::UserSplit { .. } => (0.4, 5.0),
            FillPattern::Laydown { .. } => (2.0, 4.0),
            FillPattern::Tartan { .. } => (0.8, 3.0),
            FillPattern::Gradient { .. } => (0.4, 3.5),
            FillPattern::Meander { .. } => (2.0, 2.0),
            FillPattern::Circular { .. } | FillPattern::Guided { .. } => (0.4, 3.0),
            FillPattern::Mesh { .. } => (1.6, 2.0),
        }
    }
}

/// Underlay setting for satin and fill objects.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", content = "layers", rename_all = "snake_case")]
pub enum Underlay {
    /// The engine selects the underlay from the object type and size.
    #[default]
    Auto,
    /// As `Auto`, with fewer layers (for thin or delicate fabric).
    Light,
    /// As `Auto`, with more layers (for soft, stretchy or high-pile fabric).
    Heavy,
    /// Two crossing layers: two zigzag layers on columns, two tatami layers
    /// at ±45° to the top rows on fills.
    Double,
    /// For free-standing lace (P11 Milestone 5): a run along the edge two
    /// times, so its threads lock, then a zigzag (columns) or tatami rows
    /// (fills) across it.
    Lace,
    None,
    /// The layers sew in list order.
    Custom(Vec<UnderlayLayer>),
}

/// One underlay layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnderlayLayer {
    CenterWalk,
    EdgeWalk {
        inset_mm: f64,
    },
    Zigzag {
        inset_mm: f64,
        spacing_mm: f64,
    },
    Tatami {
        inset_mm: f64,
        spacing_mm: f64,
        angle_deg: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centerline_makes_rails_at_half_width() {
        let line = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(10.0, 0.0))
            .line_to(Point::new(10.0, 10.0));
        let g = SatinGeometry::from_centerline(&line, 4.0);
        let a: Vec<Point> = g.rail_a.points().collect();
        let b: Vec<Point> = g.rail_b.points().collect();
        // Going +X, the left side on screen (Y down) is -Y.
        assert_eq!(a[0], Point::new(0.0, -2.0));
        assert_eq!(b[0], Point::new(0.0, 2.0));
        // The corner is mitered: (12, -2) outside, (8, 2) inside.
        assert!(a[1].distance(Point::new(12.0, -2.0)) < 1e-9, "{:?}", a[1]);
        assert!(b[1].distance(Point::new(8.0, 2.0)) < 1e-9, "{:?}", b[1]);
        assert_eq!(g.rungs.len(), 1);
        assert_eq!(
            SatinGeometry::from_centerline(&Path::new(), 3.0),
            SatinGeometry::default()
        );
    }

    #[test]
    fn outline_and_angle_lines_make_a_turning_column() {
        // A 40 x 10 rectangle; angle lines across its two short ends.
        let outline = Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 10.0)));
        let lines = [
            (Point::new(1.0, 0.0), Point::new(1.0, 10.0)),
            (Point::new(20.0, 0.0), Point::new(20.0, 10.0)),
            (Point::new(39.0, 0.0), Point::new(39.0, 10.0)),
        ];
        let g = SatinGeometry::from_outline(&outline, &lines).unwrap();
        let a: Vec<Point> = g.rail_a.points().collect();
        let b: Vec<Point> = g.rail_b.points().collect();
        assert_eq!(a.first(), Some(&Point::new(1.0, 0.0)));
        assert_eq!(a.last(), Some(&Point::new(39.0, 0.0)));
        assert!(a.iter().all(|p| p.y == 0.0), "rail A is the top side: {a:?}");
        assert!(b.iter().all(|p| p.y == 10.0), "rail B is the bottom side: {b:?}");
        assert_eq!(g.rungs.len(), 3);
        assert!(SatinGeometry::from_outline(&outline, &lines[..1]).is_none());
    }

    #[test]
    fn running_params_default_when_new_fields_are_missing() {
        let p: RunningParams = serde_json::from_str(r#"{"stitch_length_mm":2.0,"repeats":3}"#).unwrap();
        assert_eq!((p.style, p.pass), (RunStyle::Run, RunPass::Once));
        let s: SatinParams = serde_json::from_str(r#"{"spacing_mm":0.5}"#).unwrap();
        assert_eq!(s.split_mm, SATIN_SPLIT_MM);
        assert_eq!(s.pattern, ColumnPattern::Satin);
        assert!(p.border.is_none());
    }

    #[test]
    fn fill_types_round_trip_and_tatami_is_not_written() {
        let json = serde_json::to_string(&FillParams::default()).unwrap();
        assert!(!json.contains("pattern"), "{json}");
        for pattern in FillPattern::all() {
            let p = FillParams::default().with_pattern(pattern);
            let back: FillParams = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
            assert_eq!(back, p);
        }
        let radial = r#"{"pattern":{"type":"radial","center":[1.0,2.0]}}"#;
        let p: FillParams = serde_json::from_str(radial).unwrap();
        assert_eq!(
            p.pattern,
            FillPattern::Radial {
                center: Some(Point::new(1.0, 2.0))
            }
        );
        let s: SatinParams = serde_json::from_str(r#"{"liquid":true}"#).unwrap();
        assert!(s.liquid);
        assert!(
            !serde_json::to_string(&SatinParams::default())
                .unwrap()
                .contains("liquid")
        );
    }

    #[test]
    fn effects_are_not_written_when_off_and_accordion_profiles() {
        let s = serde_json::to_string(&SatinParams::default()).unwrap();
        for key in [
            "auto_jump",
            "spacing_at",
            "accordion",
            "jagged",
            "hand",
            "pull_percent",
            "pull_b",
        ] {
            assert!(!s.contains(key), "{key} in {s}");
        }
        let f = serde_json::to_string(&FillParams::default()).unwrap();
        assert!(!f.contains("trapunto") && !f.contains("hand"), "{f}");
        let p: SatinParams = serde_json::from_str(r#"{"spacing_at":0.2,"auto_jump":true}"#).unwrap();
        assert_eq!((p.spacing_at, p.auto_jump), (0.2, true));
        let (_, fade) = &Accordion::presets()[0];
        assert_eq!(fade.factor(0.0), 1.0);
        assert_eq!(fade.factor(0.5), 3.5);
        assert_eq!(fade.factor(2.0), 6.0);
        assert_eq!(
            Accordion {
                points: vec![(0.0, 0.2)]
            }
            .factor(0.5),
            1.0,
            "never denser than set"
        );
        let u: Underlay = serde_json::from_str(r#"{"mode":"double"}"#).unwrap();
        assert_eq!(u, Underlay::Double);
    }

    #[test]
    fn column_types_round_trip_and_satin_is_not_written() {
        let json = serde_json::to_string(&SatinParams::default()).unwrap();
        assert!(!json.contains("pattern"), "{json}");
        let json = serde_json::to_string(&RunningParams::default()).unwrap();
        assert!(!json.contains("border"), "{json}");
        for pattern in ColumnPattern::ALL {
            let p = RunningParams {
                border: Some(LineBorder::new(3.0, pattern)),
                ..RunningParams::default()
            };
            let back: RunningParams = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
            assert_eq!(back, p);
        }
        let raised = r#"{"pattern":{"type":"raised","layers":3}}"#;
        let s: SatinParams = serde_json::from_str(raised).unwrap();
        assert_eq!(s.pattern, ColumnPattern::Raised { layers: 3 });
    }

    #[test]
    fn revisions_are_unique() {
        let a = fresh_revision();
        let b = fresh_revision();
        assert!(b > a);
    }

    #[test]
    fn params_default_when_missing() {
        let json = r#"{"id":1,"thread":0,"kind":{"type":"running","path":[{"M":[0,0]},{"L":[10,0]}]}}"#;
        let o: DesignObject = serde_json::from_str(json).unwrap();
        let ObjectKind::Running { params, .. } = &o.kind else {
            panic!()
        };
        assert_eq!(*params, RunningParams::default());
        assert_eq!(o.control_bbox().unwrap().width(), 10.0);
    }

    #[test]
    fn underlay_serialized_form() {
        let u = Underlay::Custom(vec![
            UnderlayLayer::CenterWalk,
            UnderlayLayer::EdgeWalk { inset_mm: 0.3 },
        ]);
        let json = serde_json::to_string(&u).unwrap();
        assert_eq!(
            json,
            r#"{"mode":"custom","layers":[{"type":"center_walk"},{"type":"edge_walk","inset_mm":0.3}]}"#
        );
        assert_eq!(serde_json::to_string(&Underlay::Auto).unwrap(), r#"{"mode":"auto"}"#);
        assert_eq!(serde_json::to_string(&Underlay::Heavy).unwrap(), r#"{"mode":"heavy"}"#);
    }
}
