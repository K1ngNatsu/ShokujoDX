//! Vector objects (ADR 0024): artwork with no stitches, and the conversion
//! between vectors and embroidery objects.
//!
//! - To a fill: one fill object with all closed shapes and their holes
//!   (even-odd); open subpaths count as closed.
//! - To satin: an open subpath is the center line of a column. Each closed
//!   shape (with its holes, even-odd) becomes the satin that sews it with
//!   no stitch outside it and no stitch over another (owner report,
//!   2026-10-24: the old turning column along one straight axis went
//!   outside curved shapes, and a hole became a second column on top):
//!   - with no hole: a turning column along the spine of the shape
//!     (`emb_geom::spine`), with stitch angle lines square to the spine;
//!   - with one hole (an O): a ring column round the hole;
//!   - else, or when the column would not sew well (see `sews_well`): a
//!     fill with the satin stitch type, at the angle of the shortest
//!     stitches. The fill cuts the shape into sections.
//!
//!   The new objects go nearest first, so the needle does not go to and fro.
//! - To an outline: all subpaths as one running line.
//! - Back to a vector: the outline of an embroidery object, with its thread
//!   color as the fill (areas) or the stroke (lines).

use emb_geom::{Path, Point, region, spine};
use serde::{Deserialize, Serialize};

use crate::{ColumnPattern, CurrentProperties, FillPattern, ObjectKind, Rgb, SatinGeometry};

/// The stroke width of a new vector, in mm.
pub const DEFAULT_STROKE_MM: f64 = 0.35;
/// Tolerance to flatten outlines for the conversion, in mm.
const FLATTEN_TOL_MM: f64 = 0.05;
/// Distance between the stitch angle lines of a turning column made from a
/// closed shape, in mm.
const ANGLE_LINE_STEP_MM: f64 = 2.5;
/// Shapes smaller than this make no satin, in mm².
const MIN_AREA_MM2: f64 = 0.01;
/// [`sews_well`] tests a stitch this often along the column, in mm.
const CHECK_STEP_MM: f64 = 0.5;
/// [`sews_well`]: at most this part of the stitches can go outside.
const MAX_BAD: f64 = 0.01;
/// [`sews_well`]: the widest stitch is at most this many times the median one.
const MAX_WIDTH_RATIO: f64 = 3.0;
/// [`sews_well`]: a turning column covers at least this part of the shape.
const MIN_COVER: f64 = 0.9;

/// The look of a vector object. Its colors are not threads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorStyle {
    /// The fill of the closed subpaths (even-odd), if any.
    pub fill: Option<Rgb>,
    /// The color of the lines, if any.
    pub stroke: Option<Rgb>,
    /// The width of the lines, in mm.
    pub stroke_mm: f64,
}

impl Default for VectorStyle {
    fn default() -> Self {
        VectorStyle {
            fill: None,
            stroke: None,
            stroke_mm: DEFAULT_STROKE_MM,
        }
    }
}

impl VectorStyle {
    /// A style with a fill and no stroke.
    pub fn filled(color: Rgb) -> VectorStyle {
        VectorStyle {
            fill: Some(color),
            ..VectorStyle::default()
        }
    }

    /// A style with a stroke and no fill.
    pub fn stroked(color: Rgb) -> VectorStyle {
        VectorStyle {
            stroke: Some(color),
            ..VectorStyle::default()
        }
    }
}

/// What a vector becomes (Convert).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTo {
    Fill,
    Satin,
    Outline,
}

impl ConvertTo {
    pub const ALL: [ConvertTo; 3] = [ConvertTo::Fill, ConvertTo::Satin, ConvertTo::Outline];

    pub fn name(self) -> &'static str {
        match self {
            ConvertTo::Fill => "Fill",
            ConvertTo::Satin => "Satin column",
            ConvertTo::Outline => "Outline",
        }
    }

    /// The vector color that the new objects take: the fill for areas, the
    /// stroke for lines (the other one if the first is not set).
    pub fn color(self, style: &VectorStyle) -> Option<Rgb> {
        match self {
            ConvertTo::Fill | ConvertTo::Satin => style.fill.or(style.stroke),
            ConvertTo::Outline => style.stroke.or(style.fill),
        }
    }
}

/// The embroidery objects made from the subpaths of a vector, with the
/// current properties. `width_mm` is the width of a satin column made from
/// an open subpath. Empty if nothing can be made.
pub fn from_vector(path: &Path, to: ConvertTo, current: &CurrentProperties, width_mm: f64) -> Vec<ObjectKind> {
    let subs = path.subpaths();
    match to {
        ConvertTo::Fill => {
            let closed: Vec<Path> = subs.into_iter().map(Path::closed).collect();
            let groups = region::nest(&closed);
            if groups.is_empty() {
                return Vec::new();
            }
            let outers: Vec<Path> = groups.iter().map(|(o, _)| o.clone()).collect();
            vec![ObjectKind::Fill {
                outer: Path::joined(&outers),
                holes: groups.into_iter().flat_map(|(_, h)| h).collect(),
                params: current.fill.clone(),
            }]
        }
        ConvertTo::Outline if subs.is_empty() => Vec::new(),
        ConvertTo::Outline => vec![ObjectKind::Running {
            path: Path::joined(&subs),
            params: current.running.clone(),
        }],
        ConvertTo::Satin => {
            let (closed, open): (Vec<Path>, Vec<Path>) = subs.into_iter().partition(Path::is_closed);
            let mut out: Vec<ObjectKind> = open
                .iter()
                .map(|s| ObjectKind::Satin {
                    geometry: SatinGeometry::from_centerline(s, width_mm),
                    params: current.satin.clone(),
                })
                .collect();
            for (outer, holes) in region::nest(&closed) {
                out.extend(satin_area(&outer, &holes, current));
            }
            nearest_first(out)
        }
    }
}

/// The satin object for one closed shape and its holes (see the module
/// comment). `None` if the shape has no area.
fn satin_area(outer: &Path, holes: &[Path], current: &CurrentProperties) -> Option<ObjectKind> {
    let ring = |p: &Path| p.flatten(FLATTEN_TOL_MM).into_iter().next().map(|l| l.points);
    let outer_ring = ring(outer).filter(|r| r.len() >= 3)?;
    let hole_rings: Vec<Vec<Point>> = holes.iter().filter_map(ring).filter(|r| r.len() >= 3).collect();
    if region::ring_signed_area(&outer_ring).abs() < MIN_AREA_MM2 {
        return None;
    }
    let column = match hole_rings.as_slice() {
        [] => turning_column(outer, &outer_ring),
        [hole] => ring_column(&outer_ring, hole),
        _ => None,
    };
    if let Some(geometry) = column {
        return Some(ObjectKind::Satin {
            geometry,
            params: current.satin.clone(),
        });
    }
    let mut rings = vec![outer_ring];
    rings.extend(hole_rings);
    let mut params = current.fill.with_pattern(FillPattern::Columns {
        column: ColumnPattern::Satin,
    });
    params.angle_deg = shortest_angle(&rings);
    params.spacing_mm = current.satin.spacing_mm;
    params.pull_compensation_mm = current.satin.pull_compensation_mm;
    Some(ObjectKind::Fill {
        outer: outer.clone().closed(),
        holes: holes.iter().cloned().map(Path::closed).collect(),
        params,
    })
}

/// A turning satin column from a closed shape with no holes: stitch angle
/// lines square to its spine. `None` if the shape has no area or the
/// column would not sew well.
pub fn satin_from_shape(outline: &Path) -> Option<SatinGeometry> {
    let ring = outline.flatten(FLATTEN_TOL_MM).into_iter().next()?.points;
    if ring.len() < 3 || region::ring_signed_area(&ring).abs() < MIN_AREA_MM2 {
        return None;
    }
    turning_column(outline, &ring)
}

fn turning_column(outline: &Path, ring: &[Point]) -> Option<SatinGeometry> {
    let rings = [ring.to_vec()];
    let guide = spine::spine(ring)?;
    let lines = spine::cross_lines(&rings, &guide, 90.0, ANGLE_LINE_STEP_MM);
    if lines.len() < 2 {
        return None;
    }
    let g = SatinGeometry::from_outline(outline, &lines)?;
    let shape = region::ring_signed_area(ring).abs();
    sews_well(&g, &rings, Some(shape)).then_some(g)
}

/// A ring column round the hole of a shape with one hole: rail A is the
/// outline, rail B the hole, both the same way round, with stitch angle
/// lines to the nearest point of the hole. `None` if it would not sew well.
fn ring_column(outer: &[Point], hole: &[Point]) -> Option<SatinGeometry> {
    let mut hole = hole.to_vec();
    if region::ring_signed_area(outer).signum() != region::ring_signed_area(&hole).signum() {
        hole.reverse();
    }
    // Rail B starts at its point nearest to the start of rail A.
    let start = (0..hole.len())
        .min_by(|&i, &j| hole[i].distance(outer[0]).total_cmp(&hole[j].distance(outer[0])))
        .unwrap_or(0);
    hole.rotate_left(start);
    let nearest_on_hole = |p: Point| {
        (0..hole.len())
            .map(|i| {
                let (a, b) = (hole[i], hole[(i + 1) % hole.len()]);
                let d = b - a;
                let t = if d.length_sq() > 0.0 {
                    ((p - a).dot(d) / d.length_sq()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                a + d * t
            })
            .min_by(|x, y| x.distance(p).total_cmp(&y.distance(p)))
            .unwrap_or(hole[0])
    };
    let mut lines = Vec::new();
    let mut since = 0.0;
    for i in 1..outer.len() {
        since += outer[i].distance(outer[i - 1]);
        if since >= ANGLE_LINE_STEP_MM {
            lines.push((outer[i], nearest_on_hole(outer[i])));
            since = 0.0;
        }
    }
    let g = SatinGeometry {
        rail_a: Path::polygon(outer),
        rail_b: Path::polygon(&hole),
        rungs: lines,
    };
    sews_well(&g, &[outer.to_vec(), hole], None).then_some(g)
}

/// A rail as a polyline (a closed rail ends where it starts), with the
/// length along it at each point.
fn rail_points(path: &Path) -> Option<(Vec<Point>, Vec<f64>)> {
    let line = path.flatten(FLATTEN_TOL_MM).into_iter().next()?;
    let mut pts = line.points;
    if line.closed
        && let Some(&first) = pts.first()
    {
        pts.push(first);
    }
    pts.dedup();
    if pts.len() < 2 {
        return None;
    }
    let mut at = vec![0.0];
    for w in pts.windows(2) {
        at.push(at[at.len() - 1] + w[0].distance(w[1]));
    }
    Some((pts, at))
}

fn rail_at(pts: &[Point], at: &[f64], s: f64) -> Point {
    let s = s.clamp(0.0, at[at.len() - 1]);
    let i = at.partition_point(|&l| l <= s).clamp(1, pts.len() - 1);
    let seg = at[i] - at[i - 1];
    pts[i - 1].lerp(pts[i], if seg > 0.0 { (s - at[i - 1]) / seg } else { 0.0 })
}

fn rail_project(pts: &[Point], at: &[f64], p: Point) -> f64 {
    let mut best = (f64::INFINITY, 0.0);
    for (i, w) in pts.windows(2).enumerate() {
        let d = w[1] - w[0];
        let t = if d.length_sq() > 0.0 {
            ((p - w[0]).dot(d) / d.length_sq()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let dist = p.distance(w[0].lerp(w[1], t));
        if dist < best.0 {
            best = (dist, at[i] + t * (at[i + 1] - at[i]));
        }
    }
    best.1
}

/// Whether the column sews well: the stitches that the stitch engine makes
/// (points at the same fraction of the rails between two rungs) stay in the
/// shape (`rings`, even-odd), the widest stitch is at most [`MAX_WIDTH_RATIO`]
/// times the median one, and the column covers [`MIN_COVER`] of `area` (the
/// parts past the first and the last rung are not sewn).
fn sews_well(g: &SatinGeometry, rings: &[Vec<Point>], area: Option<f64>) -> bool {
    let (Some((a, at_a)), Some((b, at_b))) = (rail_points(&g.rail_a), rail_points(&g.rail_b)) else {
        return false;
    };
    let (la, lb) = (at_a[at_a.len() - 1], at_b[at_b.len() - 1]);
    // The rungs as lengths along the rails, as the stitch engine uses them.
    let mut inner: Vec<(f64, f64)> = g
        .rungs
        .iter()
        .map(|(pa, pb)| (rail_project(&a, &at_a, *pa), rail_project(&b, &at_b, *pb)))
        .collect();
    inner.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut rungs = vec![(0.0, 0.0)];
    for r in inner {
        let last = rungs[rungs.len() - 1];
        if r.0 > last.0 + 1e-6 && r.1 > last.1 + 1e-6 && r.0 < la - 1e-6 && r.1 < lb - 1e-6 {
            rungs.push(r);
        }
    }
    rungs.push((la, lb));
    let inside = |p: Point| rings.iter().filter(|r| region::ring_contains(r, p)).count() % 2 == 1;
    let crosses_edge = |p: Point, q: Point| {
        rings
            .iter()
            .any(|r| (0..r.len()).any(|i| spine::segments_cross(p, q, r[i], r[(i + 1) % r.len()])))
    };
    let (mut bad, mut total) = (0usize, 0usize);
    let mut widths: Vec<f64> = Vec::new();
    for w in rungs.windows(2) {
        let n = ((((w[1].0 - w[0].0) + (w[1].1 - w[0].1)) / 2.0 / CHECK_STEP_MM).ceil() as usize).max(1);
        for q in 0..n {
            let f = q as f64 / n as f64;
            let pa = rail_at(&a, &at_a, w[0].0 + f * (w[1].0 - w[0].0));
            let pb = rail_at(&b, &at_b, w[0].1 + f * (w[1].1 - w[0].1));
            total += 1;
            widths.push(pa.distance(pb));
            if pa.distance(pb) > 1e-6 && (!inside(pa.midpoint(pb)) || crosses_edge(pa, pb)) {
                bad += 1;
            }
        }
    }
    if total == 0 || bad as f64 > MAX_BAD * total as f64 {
        return false;
    }
    widths.sort_by(f64::total_cmp);
    let median = widths[widths.len() / 2];
    if widths[widths.len() - 1] > MAX_WIDTH_RATIO * median.max(0.5) {
        return false;
    }
    match area {
        Some(shape) => {
            let mut poly = a.clone();
            poly.extend(b.iter().rev());
            region::ring_signed_area(&poly).abs() >= MIN_COVER * shape
        }
        None => true,
    }
}

/// The fill angle of the shortest stitches across the shape (`rings`,
/// even-odd): the angle, in steps of 15°, with the shortest mean length of
/// the lines across the shape at that angle, by area (a long line counts
/// for the area that it covers).
fn shortest_angle(rings: &[Vec<Point>]) -> f64 {
    let mut best = (f64::INFINITY, 0.0);
    for k in 0..12 {
        let deg = 15.0 * f64::from(k);
        let (s, c) = emb_geom::math::sin_cos_deg(deg);
        // Turned so that the lines at `deg` are level: u along them, v across.
        let turned: Vec<Vec<(f64, f64)>> = rings
            .iter()
            .map(|r| r.iter().map(|p| (p.x * c + p.y * s, -p.x * s + p.y * c)).collect())
            .collect();
        let (lo, hi) = turned
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.1), hi.max(p.1))
            });
        let (mut sum, mut count) = (0.0, 0.0);
        let mut v = lo + 0.5;
        let mut xs: Vec<f64> = Vec::new();
        while v < hi {
            xs.clear();
            for r in &turned {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    if (a.1 > v) != (b.1 > v) {
                        xs.push(a.0 + (v - a.1) * (b.0 - a.0) / (b.1 - a.1));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for c in xs.as_chunks::<2>().0 {
                sum += (c[1] - c[0]) * (c[1] - c[0]);
                count += c[1] - c[0];
            }
            v += 1.0;
        }
        if count > 0.0 && sum / count < best.0 - 1e-9 {
            best = (sum / count, deg);
        }
    }
    best.1
}

/// The objects in an order where each one is the nearest (by the middle of
/// its box) to the one before, from the first one.
fn nearest_first(mut kinds: Vec<ObjectKind>) -> Vec<ObjectKind> {
    let middle = |k: &ObjectKind| k.control_bbox().map_or(Point::ZERO, |b| b.center());
    let mut out = Vec::with_capacity(kinds.len());
    let Some(first) = kinds.first() else {
        return out;
    };
    let mut at = middle(first);
    while !kinds.is_empty() {
        let i = (0..kinds.len())
            .min_by(|&i, &j| {
                middle(&kinds[i])
                    .distance(at)
                    .total_cmp(&middle(&kinds[j]).distance(at))
            })
            .unwrap_or(0);
        let k = kinds.remove(i);
        at = middle(&k);
        out.push(k);
    }
    out
}

/// The outline of an embroidery object as a vector, in `color`: areas get
/// a fill, lines a stroke. `None` for stitch objects and vectors.
pub fn to_vector(kind: &ObjectKind, color: Rgb) -> Option<ObjectKind> {
    let (path, style) = match kind {
        ObjectKind::Running { path, .. } => (path.clone(), VectorStyle::stroked(color)),
        ObjectKind::Lettering(_) | ObjectKind::Fill { .. } | ObjectKind::Satin { .. } | ObjectKind::Applique { .. } => {
            (area_outline(kind), VectorStyle::filled(color))
        }
        ObjectKind::Stitches { .. }
        | ObjectKind::Vector { .. }
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => {
            return None;
        }
    };
    (!path.subpaths().is_empty()).then_some(ObjectKind::Vector { path, style })
}

/// The outline of an area object as closed subpaths (even-odd).
pub(crate) fn area_outline(kind: &ObjectKind) -> Path {
    // Closed copies of the subpaths of the outlines and the holes.
    let closed = |paths: Vec<&Path>| -> Path {
        let subs: Vec<Path> = Path::joined(paths).subpaths().into_iter().map(Path::closed).collect();
        Path::joined(&subs)
    };
    match kind {
        ObjectKind::Fill { outer, holes, .. } => closed(std::iter::once(outer).chain(holes).collect()),
        ObjectKind::Applique { pieces, .. } => closed(
            pieces
                .iter()
                .flat_map(|pc| std::iter::once(&pc.outer).chain(&pc.holes))
                .collect(),
        ),
        ObjectKind::Satin { geometry, .. } => {
            let flat = |p: &Path| p.flatten(FLATTEN_TOL_MM).into_iter().next();
            match (flat(&geometry.rail_a), flat(&geometry.rail_b)) {
                // A ring column: two closed rails.
                (Some(a), Some(b)) if a.closed && b.closed => {
                    Path::joined([&Path::polygon(&a.points), &Path::polygon(&b.points)])
                }
                (Some(a), Some(b)) => {
                    let pts: Vec<Point> = a.points.iter().chain(b.points.iter().rev()).copied().collect();
                    Path::polygon(&pts)
                }
                _ => Path::new(),
            }
        }
        ObjectKind::Lettering(l) => {
            let parts: Vec<Path> = l.parts().map(area_outline).collect();
            Path::joined(&parts)
        }
        ObjectKind::Running { .. }
        | ObjectKind::Stitches { .. }
        | ObjectKind::Vector { .. }
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => Path::new(),
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn ring() -> Path {
        Path::joined([
            &Path::rect(BBox::new(p(0.0, 0.0), p(40.0, 40.0))),
            &Path::rect(BBox::new(p(10.0, 10.0), p(30.0, 30.0))),
        ])
    }

    #[test]
    fn closed_shapes_become_one_fill_with_holes() {
        let mut path = ring();
        // A second shape with no close command counts as closed.
        path.cmds.extend(
            Path::new()
                .move_to(p(50.0, 0.0))
                .line_to(p(60.0, 0.0))
                .line_to(p(60.0, 10.0))
                .cmds,
        );
        let kinds = from_vector(&path, ConvertTo::Fill, &CurrentProperties::default(), 3.0);
        assert_eq!(kinds.len(), 1, "one vector, one fill");
        let ObjectKind::Fill { outer, holes, .. } = &kinds[0] else {
            panic!()
        };
        assert_eq!(holes.len(), 1);
        let outers = outer.subpaths();
        assert_eq!(outers.len(), 2);
        assert!(outers.iter().all(Path::is_closed));
    }

    #[test]
    fn outline_is_one_line_and_open_paths_make_columns() {
        let line = Path::new().move_to(p(0.0, 0.0)).line_to(p(30.0, 0.0));
        let path = Path::joined([&ring(), &line]);
        let c = CurrentProperties::default();
        let out = from_vector(&path, ConvertTo::Outline, &c, 3.0);
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], ObjectKind::Running { path, .. } if path.subpaths().len() == 3));
        let satin = from_vector(&line, ConvertTo::Satin, &c, 4.0);
        let ObjectKind::Satin { geometry, .. } = &satin[0] else {
            panic!()
        };
        let b = geometry
            .rail_a
            .control_bbox()
            .unwrap()
            .union(geometry.rail_b.control_bbox().unwrap());
        assert!(
            (b.height() - 4.0).abs() < 1e-6 && (b.width() - 30.0).abs() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn a_closed_bar_becomes_a_turning_column_along_its_length() {
        let bar = Path::rect(BBox::new(p(0.0, 0.0), p(40.0, 6.0)));
        let kinds = from_vector(&bar, ConvertTo::Satin, &CurrentProperties::default(), 3.0);
        assert_eq!(kinds.len(), 1);
        let ObjectKind::Satin { geometry, .. } = &kinds[0] else {
            panic!()
        };
        assert!(geometry.rungs.len() >= 10);
        // The angle lines go across the bar (the short way).
        for (a, b) in &geometry.rungs {
            assert!((a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() > 5.0, "{a:?} {b:?}");
        }
        assert!(satin_from_shape(&Path::new().move_to(p(0.0, 0.0)).line_to(p(1.0, 0.0)).close()).is_none());
    }

    /// A C shape: a ring sector round (50, 50), radii 20 and 30, open on the right.
    fn c_shape() -> Path {
        let arc = |r: f64, from: f64, to: f64| -> Vec<Point> {
            (0..=56)
                .map(|i| {
                    let a = (from + (to - from) * f64::from(i) / 56.0).to_radians();
                    p(50.0 + r * a.cos(), 50.0 + r * a.sin())
                })
                .collect()
        };
        let mut pts = arc(30.0, 40.0, 320.0);
        pts.extend(arc(20.0, 320.0, 40.0));
        Path::polygon(&pts)
    }

    #[test]
    fn a_c_shape_becomes_a_turning_column_that_stays_inside() {
        let kinds = from_vector(&c_shape(), ConvertTo::Satin, &CurrentProperties::default(), 3.0);
        assert_eq!(kinds.len(), 1);
        let ObjectKind::Satin { geometry, .. } = &kinds[0] else {
            panic!("{:?}", kinds[0].type_name())
        };
        // The stitch angle lines go across the C (from the outer edge to
        // the inner edge), all round it.
        assert!(geometry.rungs.len() > 40, "{}", geometry.rungs.len());
        // (The lines at the very ends can meet the end edges.)
        let n = geometry.rungs.len();
        for (a, b) in &geometry.rungs[2..n - 2] {
            let r = |q: &Point| q.distance(p(50.0, 50.0));
            let (ra, rb) = (r(a), r(b));
            assert!(ra.max(rb) > 29.0 && ra.min(rb) < 21.0, "{a:?} {b:?}");
            let m = a.midpoint(*b);
            assert!((r(&m) - 25.0).abs() < 1.5, "{m:?}");
        }
    }

    #[test]
    fn a_shape_with_one_hole_becomes_a_ring_column() {
        // An O: two circles.
        let circle = |r: f64| {
            let pts: Vec<Point> = (0..72)
                .map(|i| {
                    let a = (5.0 * f64::from(i)).to_radians();
                    p(r * a.cos(), r * a.sin())
                })
                .collect();
            Path::polygon(&pts)
        };
        let o = Path::joined([&circle(30.0), &circle(20.0)]);
        let kinds = from_vector(&o, ConvertTo::Satin, &CurrentProperties::default(), 3.0);
        // One object: the hole is not sewn over by a second column.
        assert_eq!(kinds.len(), 1);
        let ObjectKind::Satin { geometry, .. } = &kinds[0] else {
            panic!()
        };
        assert!(geometry.rail_a.is_closed() && geometry.rail_b.is_closed());
        for (a, b) in &geometry.rungs {
            assert!(
                (a.length() - 30.0).abs() < 0.5 && (b.length() - 20.0).abs() < 0.5,
                "{a:?} {b:?}"
            );
        }
    }

    #[test]
    fn a_complex_shape_becomes_a_satin_fill() {
        // An E: no column sews it well.
        let e = Path::polygon(&[
            p(20.0, 15.0),
            p(80.0, 15.0),
            p(80.0, 30.0),
            p(40.0, 30.0),
            p(40.0, 42.0),
            p(70.0, 42.0),
            p(70.0, 57.0),
            p(40.0, 57.0),
            p(40.0, 70.0),
            p(80.0, 70.0),
            p(80.0, 85.0),
            p(20.0, 85.0),
        ]);
        let c = CurrentProperties::default();
        let kinds = from_vector(&e, ConvertTo::Satin, &c, 3.0);
        assert_eq!(kinds.len(), 1);
        let ObjectKind::Fill { params, .. } = &kinds[0] else {
            panic!("{}", kinds[0].type_name())
        };
        assert!(matches!(
            params.pattern,
            FillPattern::Columns {
                column: ColumnPattern::Satin
            }
        ));
        assert_eq!(params.spacing_mm, c.satin.spacing_mm);
    }

    #[test]
    fn new_objects_go_nearest_first() {
        let line = |x: f64| Path::new().move_to(p(x, 0.0)).line_to(p(x, 10.0));
        let path = Path::joined([&line(0.0), &line(50.0), &line(10.0), &line(40.0)]);
        let kinds = from_vector(&path, ConvertTo::Satin, &CurrentProperties::default(), 3.0);
        let xs: Vec<f64> = kinds
            .iter()
            .map(|k| k.control_bbox().unwrap().center().x.round())
            .collect();
        assert_eq!(xs, vec![0.0, 10.0, 40.0, 50.0]);
    }

    #[test]
    fn embroidery_back_to_vector() {
        let red = Rgb(200, 0, 0);
        let fill = from_vector(&ring(), ConvertTo::Fill, &CurrentProperties::default(), 3.0).remove(0);
        let Some(ObjectKind::Vector { path, style }) = to_vector(&fill, red) else {
            panic!()
        };
        assert_eq!(style, VectorStyle::filled(red));
        assert_eq!(path.subpaths().len(), 2);
        // And again to a fill: the same shape.
        let again = from_vector(&path, ConvertTo::Fill, &CurrentProperties::default(), 3.0);
        assert_eq!(again, vec![fill]);
        let line = ObjectKind::Running {
            path: Path::new().move_to(p(0.0, 0.0)).line_to(p(5.0, 0.0)),
            params: Default::default(),
        };
        assert!(
            matches!(to_vector(&line, red), Some(ObjectKind::Vector { style, .. }) if style == VectorStyle::stroked(red))
        );
        let column = from_vector(
            &Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)),
            ConvertTo::Satin,
            &CurrentProperties::default(),
            4.0,
        )
        .remove(0);
        let Some(ObjectKind::Vector { path, .. }) = to_vector(&column, red) else {
            panic!()
        };
        let b = path.control_bbox().unwrap();
        assert!((b.width() - 20.0).abs() < 1e-6 && (b.height() - 4.0).abs() < 1e-6);
    }
}
