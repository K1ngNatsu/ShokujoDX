//! Rhinestone (bling) objects (ADR 0039): stones of one size and color,
//! as single stones, a run along a path, or a fill of a closed shape.

use emb_geom::region::{self, Join, Region};
use emb_geom::{BBox, Path, Point, Transform, math};
use serde::{Deserialize, Serialize};

use crate::{Design, ObjectId, ObjectKind, Rgb};

/// The stone sizes: the SS number and the diameter in mm.
pub const STONE_SIZES: [(u32, f64); 8] = [
    (6, 2.0),
    (8, 2.4),
    (10, 2.8),
    (12, 3.1),
    (16, 3.9),
    (20, 4.7),
    (30, 6.4),
    (34, 7.1),
];

/// Common stone colors.
pub const STONE_COLORS: [(&str, Rgb); 12] = [
    ("Crystal", Rgb(225, 232, 238)),
    ("Black", Rgb(30, 30, 35)),
    ("Gold", Rgb(212, 170, 60)),
    ("Hematite", Rgb(110, 112, 118)),
    ("Red", Rgb(190, 20, 40)),
    ("Pink", Rgb(235, 130, 170)),
    ("Blue", Rgb(30, 70, 170)),
    ("Light blue", Rgb(120, 175, 225)),
    ("Green", Rgb(30, 140, 70)),
    ("Purple", Rgb(110, 50, 150)),
    ("Orange", Rgb(240, 130, 30)),
    ("Yellow", Rgb(245, 215, 50)),
];

/// The gap between the edges of two stones next to each other, by default.
pub const DEFAULT_GAP_MM: f64 = 0.5;
/// Two stones whose edges are nearer than this overlap.
pub const MIN_CLEAR_MM: f64 = 0.2;
/// Flattening tolerance of the paths, mm.
const FLAT_MM: f64 = 0.05;

/// One kind of stone: a size and a color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stone {
    /// The SS number of the size.
    pub ss: u32,
    pub color: Rgb,
    pub name: String,
}

impl Default for Stone {
    fn default() -> Self {
        Stone {
            ss: 10,
            color: STONE_COLORS[0].1,
            name: STONE_COLORS[0].0.into(),
        }
    }
}

impl Stone {
    /// The diameter in mm: from the table, or about 0.2 mm for each SS step
    /// for a size that is not in it.
    pub fn diameter_mm(&self) -> f64 {
        STONE_SIZES
            .iter()
            .find(|(ss, _)| *ss == self.ss)
            .map_or(1.0 + 0.19 * f64::from(self.ss), |s| s.1)
    }

    /// "SS10 Crystal".
    pub fn label(&self) -> String {
        format!("SS{} {}", self.ss, self.name)
    }
}

/// How a fill puts its stones.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pattern", rename_all = "snake_case")]
pub enum BlingFill {
    /// Rows at an angle; each other row moves half a step (a honeycomb).
    Rows { angle_deg: f64 },
    /// Rings that follow the outline, from the outside in, a stone apart.
    Rings,
}

impl Default for BlingFill {
    fn default() -> Self {
        BlingFill::Rows { angle_deg: 0.0 }
    }
}

/// Where the stones are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "layout", rename_all = "snake_case")]
pub enum BlingLayout {
    /// Single stones at these centers.
    Stones { points: Vec<Point> },
    /// Stones along the subpaths, from end to end (evenly round a closed
    /// subpath).
    Run { path: Path },
    /// Stones inside a closed shape (even-odd: the inner subpaths and the
    /// holes are holes).
    Fill {
        outer: Path,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<Path>,
        #[serde(default)]
        fill: BlingFill,
    },
}

/// A rhinestone object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bling {
    pub stone: Stone,
    pub layout: BlingLayout,
    /// The gap between the edges of two stones next to each other, mm.
    #[serde(default = "default_gap")]
    pub gap_mm: f64,
    /// Leave out the stones that overlap stones placed before them.
    #[serde(default = "yes")]
    pub remove_overlaps: bool,
}

fn default_gap() -> f64 {
    DEFAULT_GAP_MM
}

fn yes() -> bool {
    true
}

/// The point at length `s` along the polyline `pts`.
fn at_length(pts: &[Point], s: f64) -> Point {
    let mut left = s.max(0.0);
    for w in pts.windows(2) {
        let d = w[0].distance(w[1]);
        if left <= d {
            return w[0].lerp(w[1], if d > 0.0 { left / d } else { 0.0 });
        }
        left -= d;
    }
    pts[pts.len() - 1]
}

/// Stones `step` apart along a polyline: from end to end, or evenly round
/// a closed one.
fn along(points: &[Point], closed: bool, step: f64, out: &mut Vec<Point>) {
    let mut pts = points.to_vec();
    if closed && pts.len() > 1 && pts.first() != pts.last() {
        pts.push(pts[0]);
    }
    let len: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    if pts.is_empty() {
        return;
    }
    if closed {
        let n = (len / step).floor() as usize;
        if n < 2 {
            return;
        }
        out.extend((0..n).map(|k| at_length(&pts, len * k as f64 / n as f64)));
    } else {
        let n = (len / step).floor() as usize + 1;
        if n == 1 {
            out.push(pts[0]);
            return;
        }
        out.extend((0..n).map(|k| at_length(&pts, len * k as f64 / (n - 1) as f64)));
    }
}

/// The area of a closed shape and its holes (even-odd) as regions.
pub fn fill_regions(outer: &Path, holes: &[Path]) -> Vec<Region> {
    let subs: Vec<Path> = Path::joined(std::iter::once(outer).chain(holes))
        .subpaths()
        .into_iter()
        .map(Path::closed)
        .collect();
    let flat = |p: &Path| -> Vec<Point> { p.flatten(FLAT_MM).into_iter().flat_map(|pl| pl.points).collect() };
    region::nest(&subs)
        .iter()
        .map(|(o, hs)| Region::new(flat(o), hs.iter().map(flat).collect()))
        .collect()
}

impl Bling {
    pub fn new(stone: Stone, layout: BlingLayout) -> Bling {
        Bling {
            stone,
            layout,
            gap_mm: DEFAULT_GAP_MM,
            remove_overlaps: true,
        }
    }

    pub fn radius(&self) -> f64 {
        self.stone.diameter_mm() / 2.0
    }

    /// The distance between the centers of two stones next to each other.
    pub fn step(&self) -> f64 {
        self.stone.diameter_mm() + self.gap_mm.max(0.0)
    }

    /// The centers of the stones of the object, before the overlap removal
    /// of [`Design::stones`].
    pub fn centers(&self) -> Vec<Point> {
        let step = self.step();
        let mut out = Vec::new();
        match &self.layout {
            BlingLayout::Stones { points } => out.extend(points.iter().copied()),
            BlingLayout::Run { path } => {
                for pl in path.flatten(FLAT_MM) {
                    along(&pl.points, pl.closed, step, &mut out);
                }
            }
            BlingLayout::Fill { outer, holes, fill } => {
                let area = fill_regions(outer, holes);
                // The centers stay a radius inside the edge.
                let inside = region::offset(&area, -self.radius(), Join::Round);
                match *fill {
                    BlingFill::Rows { angle_deg } => rows(&inside, angle_deg, step, &mut out),
                    BlingFill::Rings => {
                        // Rings a step apart, so the stones of two rings do
                        // not touch; a small middle part gets one stone.
                        let mut ring = inside;
                        let mut k = 0;
                        while !ring.is_empty() && k < 500 {
                            for r in &ring {
                                let before = out.len();
                                along(&r.outer, true, step, &mut out);
                                if out.len() == before
                                    && let Some(c) = BBox::from_points(r.outer.iter().copied()).map(|b| b.center())
                                    && r.contains(c)
                                {
                                    out.push(c);
                                }
                                for h in &r.holes {
                                    along(h, true, step, &mut out);
                                }
                            }
                            ring = region::offset(&ring, -step, Join::Round);
                            k += 1;
                        }
                    }
                }
            }
        }
        out
    }

    /// The paths of the layout (none for single stones).
    pub fn paths(&self) -> Vec<&Path> {
        match &self.layout {
            BlingLayout::Stones { .. } => Vec::new(),
            BlingLayout::Run { path } => vec![path],
            BlingLayout::Fill { outer, holes, .. } => std::iter::once(outer).chain(holes.iter()).collect(),
        }
    }

    pub fn paths_mut(&mut self) -> Vec<&mut Path> {
        match &mut self.layout {
            BlingLayout::Stones { .. } => Vec::new(),
            BlingLayout::Run { path } => vec![path],
            BlingLayout::Fill { outer, holes, .. } => std::iter::once(outer).chain(holes.iter_mut()).collect(),
        }
    }

    /// Applies the transform to the layout. The stones keep their size.
    pub fn transform(&mut self, t: &Transform) {
        match &mut self.layout {
            BlingLayout::Stones { points } => {
                for p in points {
                    *p = t.apply(*p);
                }
            }
            BlingLayout::Run { path } => *path = path.transformed(t),
            BlingLayout::Fill { outer, holes, fill } => {
                *outer = outer.transformed(t);
                for h in holes {
                    *h = h.transformed(t);
                }
                if let BlingFill::Rows { angle_deg } = fill {
                    *angle_deg = crate::edit::transform_angle(*angle_deg, t);
                }
            }
        }
    }

    /// The box of the stones and the paths.
    pub fn bbox(&self) -> Option<BBox> {
        let r = self.radius();
        let stones = BBox::from_points(self.centers()).map(|b| b.expand(r));
        let paths = self
            .paths()
            .into_iter()
            .filter_map(Path::control_bbox)
            .reduce(BBox::union);
        match (stones, paths) {
            (Some(a), Some(b)) => Some(a.union(b)),
            (a, b) => a.or(b),
        }
    }

    /// `true` if `p` is on a stone, near a path, or inside a fill.
    pub fn hit_test(&self, p: Point, tol: f64) -> bool {
        let r = self.radius();
        if self.centers().iter().any(|c| c.distance(p) <= r + tol) {
            return true;
        }
        match &self.layout {
            BlingLayout::Stones { .. } => false,
            BlingLayout::Run { path } => path
                .flatten(FLAT_MM)
                .iter()
                .any(|pl| pl.segments().any(|(a, b)| p.distance_to_segment(a, b) <= tol)),
            BlingLayout::Fill { outer, holes, .. } => fill_regions(outer, holes).iter().any(|r| r.contains(p)),
        }
    }
}

/// Rows of points `step` apart at `angle_deg` inside the regions; each
/// other row moves half a step, and the rows are `step`·√3/2 apart.
fn rows(area: &[Region], angle_deg: f64, step: f64, out: &mut Vec<Point>) {
    let (s, c) = math::sin_cos_deg(angle_deg);
    let (u, v) = (Point::new(c, s), Point::new(-s, c));
    let Some(b) = BBox::from_points(
        area.iter()
            .flat_map(|r| r.outer.iter().map(|p| Point::new(p.dot(u), p.dot(v)))),
    ) else {
        return;
    };
    let h = step * 3f64.sqrt() / 2.0;
    let (j0, j1) = ((b.min.y / h).floor() as i64, (b.max.y / h).ceil() as i64);
    for j in j0..=j1 {
        let shift = if j.rem_euclid(2) == 1 { step / 2.0 } else { 0.0 };
        let (i0, i1) = (
            ((b.min.x - shift) / step).floor() as i64,
            ((b.max.x - shift) / step).ceil() as i64,
        );
        for i in i0..=i1 {
            let (x, y) = (i as f64 * step + shift, j as f64 * h);
            let p = u * x + v * y;
            if area.iter().any(|r| r.contains(p)) {
                out.push(p);
            }
        }
    }
}

/// One stone in the design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedStone {
    pub at: Point,
    pub radius: f64,
    pub object: ObjectId,
    pub color: Rgb,
    /// It overlaps another stone (its object keeps overlaps).
    pub overlaps: bool,
}

/// A grid of buckets for the overlap search.
struct Buckets {
    cell: f64,
    map: std::collections::HashMap<(i64, i64), Vec<usize>>,
}

impl Buckets {
    fn key(&self, p: Point) -> (i64, i64) {
        ((p.x / self.cell).floor() as i64, (p.y / self.cell).floor() as i64)
    }

    fn near(&self, p: Point) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.key(p);
        (-1..=1)
            .flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
            .filter_map(|k| self.map.get(&k))
            .flatten()
            .copied()
    }
}

impl Design {
    /// All stones of the rhinestone objects, in sew order (ADR 0039). A
    /// stone of an object with `remove_overlaps` that overlaps a stone
    /// before it is left out; other overlaps are marked.
    pub fn stones(&self) -> Vec<PlacedStone> {
        let max_d = self
            .objects()
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Bling(b) => Some(b.stone.diameter_mm()),
                _ => None,
            })
            .fold(0.0, f64::max);
        if max_d == 0.0 {
            return Vec::new();
        }
        let mut b = Buckets {
            cell: max_d + MIN_CLEAR_MM,
            map: Default::default(),
        };
        let mut out: Vec<PlacedStone> = Vec::new();
        for o in self.objects() {
            let ObjectKind::Bling(bl) = &o.kind else { continue };
            let r = bl.radius();
            for at in bl.centers() {
                let hits: Vec<usize> = b
                    .near(at)
                    .filter(|&i| out[i].at.distance(at) < out[i].radius + r + MIN_CLEAR_MM - 1e-9)
                    .collect();
                if !hits.is_empty() && bl.remove_overlaps {
                    continue;
                }
                for &i in &hits {
                    out[i].overlaps = true;
                }
                let k = b.key(at);
                b.map.entry(k).or_default().push(out.len());
                out.push(PlacedStone {
                    at,
                    radius: r,
                    object: o.id,
                    color: bl.stone.color,
                    overlaps: !hits.is_empty(),
                });
            }
        }
        out
    }
}

/// The hole of a template is this much larger than the stone, by default
/// (mm, on the diameter), so the stone falls in and stays.
pub const DEFAULT_CLEARANCE_MM: f64 = 0.3;
/// The registration marks are this far outside the box of the stones, mm.
pub const MARK_MARGIN_MM: f64 = 8.0;
/// The half length of the arms of a registration mark, mm.
pub const MARK_ARM_MM: f64 = 3.0;

/// One hole of a rhinestone template.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateHole {
    pub at: Point,
    pub diameter_mm: f64,
    /// The stone that goes in it.
    pub stone: Stone,
}

/// A rhinestone template (P9 Milestone 4): a hole for each stone, and
/// registration marks (crosses) at the corners, in design mm (the design
/// rotation applied, as on the canvas; Y down).
#[derive(Debug, Clone, PartialEq)]
pub struct StoneTemplate {
    pub holes: Vec<TemplateHole>,
    /// The centers of the registration crosses.
    pub marks: Vec<Point>,
}

impl StoneTemplate {
    /// The box of the holes and the marks.
    pub fn bbox(&self) -> Option<BBox> {
        let holes = self
            .holes
            .iter()
            .map(|h| BBox::from_center_size(h.at, h.diameter_mm, h.diameter_mm));
        let marks = self
            .marks
            .iter()
            .map(|m| BBox::from_center_size(*m, 2.0 * MARK_ARM_MM, 2.0 * MARK_ARM_MM));
        holes.chain(marks).reduce(BBox::union)
    }

    /// The kinds of stone in the template, in the order they first come,
    /// with the number of holes of each.
    pub fn counts(&self) -> Vec<(Stone, usize)> {
        let mut out: Vec<(Stone, usize)> = Vec::new();
        for h in &self.holes {
            match out.iter_mut().find(|(s, _)| *s == h.stone) {
                Some((_, n)) => *n += 1,
                None => out.push((h.stone.clone(), 1)),
            }
        }
        out
    }
}

/// The thread of the sewn template marks: a color that is easy to see and
/// to pull out.
pub const MARK_THREAD: (&str, Rgb) = ("Template marks (pull out)", Rgb(230, 0, 120));

/// A registration cross centered on `c`, as one line that sews with no
/// trim: across, back to the middle, then up and down.
pub fn mark_path(c: Point) -> Path {
    let a = MARK_ARM_MM;
    Path::new()
        .move_to(Point::new(c.x - a, c.y))
        .line_to(Point::new(c.x + a, c.y))
        .line_to(c)
        .line_to(Point::new(c.x, c.y - a))
        .line_to(Point::new(c.x, c.y + a))
}

/// The centers of the crosses of a marks path: the middle of the box of
/// each subpath.
pub fn mark_centers(path: &Path) -> Vec<Point> {
    path.subpaths()
        .iter()
        .filter_map(|p| p.control_bbox())
        .map(|b| b.center())
        .collect()
}

impl Design {
    /// The centers of the sewn template marks (design mm, no rotation), if
    /// the design has its marks object.
    pub fn sewn_mark_centers(&self) -> Option<Vec<Point>> {
        let o = self.object(self.template_marks?)?;
        match &o.kind {
            ObjectKind::Running { path, .. } => Some(mark_centers(path)).filter(|c| !c.is_empty()),
            _ => None,
        }
    }

    /// Adds the object that sews the template marks: a running-stitch
    /// cross at each corner, [`MARK_MARGIN_MM`] outside the box of the
    /// stones, at the end of the sew order, in its own thread. If the
    /// design has its marks object, the crosses go to the new corners.
    /// Returns the object, or `None` if the design has no stones.
    pub fn add_template_marks(&mut self) -> Option<ObjectId> {
        let b = self
            .stones()
            .iter()
            .map(|s| BBox::from_center_size(s.at, 2.0 * s.radius, 2.0 * s.radius))
            .reduce(BBox::union)?
            .expand(MARK_MARGIN_MM);
        let corners = [b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)];
        let path = Path::joined(&corners.map(mark_path));
        if let Some(id) = self.template_marks
            && let Some(ObjectKind::Running { path: p, .. }) = self.object_mut(id).map(|o| &mut o.kind)
        {
            *p = path;
            return Some(id);
        }
        let thread = match self.threads.iter().find(|t| t.name == MARK_THREAD.0) {
            Some(t) => t.id,
            None => self.add_thread(MARK_THREAD.0, MARK_THREAD.1),
        };
        let id = self.add_object(
            thread,
            ObjectKind::Running {
                path,
                params: crate::RunningParams {
                    stitch_length_mm: 1.5,
                    ..crate::RunningParams::default()
                },
            },
        );
        if let Some(o) = self.object_mut(id) {
            o.name = Some("Template marks".into());
        }
        self.template_marks = Some(id);
        Some(id)
    }

    /// The rhinestone template of the design: a hole `clearance_mm` larger
    /// than each stone of [`Design::stones`], and the registration marks
    /// (the sewn marks if the design has them). `None` if the design has
    /// no stones.
    pub fn stone_template(&self, clearance_mm: f64) -> Option<StoneTemplate> {
        let rot = self.rotation_transform();
        let stones = self.stones();
        let holes: Vec<TemplateHole> = stones
            .iter()
            .filter_map(|s| {
                let ObjectKind::Bling(b) = &self.object(s.object)?.kind else {
                    return None;
                };
                Some(TemplateHole {
                    at: rot.apply(s.at),
                    diameter_mm: 2.0 * s.radius + clearance_mm.max(0.0),
                    stone: b.stone.clone(),
                })
            })
            .collect();
        let b = holes
            .iter()
            .map(|h| BBox::from_center_size(h.at, h.diameter_mm, h.diameter_mm))
            .reduce(BBox::union)?
            .expand(MARK_MARGIN_MM);
        let marks = match self.sewn_mark_centers() {
            Some(c) => c.into_iter().map(|p| rot.apply(p)).collect(),
            None => vec![b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)],
        };
        Some(StoneTemplate { holes, marks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn the_template_has_a_larger_hole_for_each_stone() {
        let mut d = Design::new("t");
        let th = d.add_thread("Stones", Rgb(0, 0, 0));
        let red = Stone {
            ss: 20,
            name: "Red".into(),
            ..Stone::default()
        };
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Stones {
                    points: vec![p(0.0, 0.0), p(10.0, 0.0)],
                },
            ))),
        );
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(
                red.clone(),
                BlingLayout::Stones {
                    points: vec![p(0.0, 20.0)],
                },
            ))),
        );
        let t = d.stone_template(0.3).unwrap();
        assert_eq!(t.holes.len(), 3);
        assert!((t.holes[0].diameter_mm - 3.1).abs() < 1e-9);
        assert!((t.holes[2].diameter_mm - 5.0).abs() < 1e-9);
        assert_eq!(t.counts(), vec![(Stone::default(), 2), (red, 1)]);
        // The marks: 8 mm outside the box of the holes (the large hole at
        // x = 0 sets the left side).
        assert_eq!(t.marks.len(), 4);
        assert!(
            t.marks[0].distance(p(-2.5 - 8.0, -1.55 - 8.0)) < 1e-9,
            "{:?}",
            t.marks[0]
        );
        assert!(Design::new("e").stone_template(0.3).is_none());
    }

    #[test]
    fn sewn_marks_and_template_marks_are_the_same_points() {
        let mut d = Design::new("t");
        let th = d.add_thread("Stones", Rgb(0, 0, 0));
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Stones {
                    points: vec![p(0.0, 0.0), p(20.0, 10.0)],
                },
            ))),
        );
        d.rotation = crate::Rotation::Deg90;
        let id = d.add_template_marks().unwrap();
        assert_eq!(d.objects().last().map(|o| o.id), Some(id), "the marks sew last");
        let sewn = d.sewn_mark_centers().unwrap();
        assert_eq!(sewn.len(), 4);
        assert!(sewn[0].distance(p(-1.4 - 8.0, -1.4 - 8.0)) < 1e-9, "{:?}", sewn[0]);
        // The template has the sewn marks (with the design rotation).
        let t = d.stone_template(0.3).unwrap();
        let rot = d.rotation_transform();
        for (a, b) in t.marks.iter().zip(&sewn) {
            assert!(a.distance(rot.apply(*b)) < 1e-9);
        }
        // A moved mark moves the template mark too.
        let moved = Path::joined(&[
            mark_path(p(-30.0, -30.0)),
            mark_path(p(40.0, -9.4)),
            mark_path(p(40.0, 30.0)),
            mark_path(p(-9.4, 30.0)),
        ]);
        if let Some(ObjectKind::Running { path, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
            *path = moved;
        }
        let t = d.stone_template(0.3).unwrap();
        assert!(t.marks[0].distance(rot.apply(p(-30.0, -30.0))) < 1e-9);
        // Adding them again puts the crosses back at the corners; one object, one thread.
        assert_eq!(d.add_template_marks(), Some(id));
        assert_eq!(d.objects().len(), 2);
        assert_eq!(d.threads.iter().filter(|t| t.name == MARK_THREAD.0).count(), 1);
        assert!(d.sewn_mark_centers().unwrap()[0].distance(p(-9.4, -9.4)) < 1e-9);
    }

    #[test]
    fn sizes_and_labels() {
        let s = Stone::default();
        assert!((s.diameter_mm() - 2.8).abs() < 1e-9);
        assert_eq!(s.label(), "SS10 Crystal");
    }

    #[test]
    fn a_run_goes_from_end_to_end() {
        // A 30 mm line, SS10 (2.8 mm) with a 0.5 mm gap: a step of 3.3 mm,
        // so 10 stones, the first and last at the ends.
        let b = Bling::new(
            Stone::default(),
            BlingLayout::Run {
                path: Path::new().move_to(p(0.0, 0.0)).line_to(p(30.0, 0.0)),
            },
        );
        let c = b.centers();
        assert_eq!(c.len(), 10);
        assert_eq!(c[0], p(0.0, 0.0));
        assert!(c[9].distance(p(30.0, 0.0)) < 1e-9);
        let gap = c[1].distance(c[0]) - 2.8;
        assert!(gap >= 0.5 - 1e-9, "{gap}");
    }

    #[test]
    fn a_closed_run_is_even_all_round() {
        let b = Bling::new(
            Stone::default(),
            BlingLayout::Run {
                path: Path::ellipse(p(0.0, 0.0), 20.0, 20.0),
            },
        );
        let c = b.centers();
        let d: Vec<f64> = (0..c.len()).map(|i| c[i].distance(c[(i + 1) % c.len()])).collect();
        let (lo, hi) = d.iter().fold((f64::MAX, 0.0_f64), |(a, b), x| (a.min(*x), b.max(*x)));
        assert!(hi - lo < 0.05, "{lo} {hi}");
        assert!(lo >= 3.3 - 0.05);
    }

    #[test]
    fn a_fill_keeps_stones_inside_and_apart() {
        for fill in [BlingFill::Rows { angle_deg: 15.0 }, BlingFill::Rings] {
            let outer = Path::rect(BBox::new(p(0.0, 0.0), p(40.0, 30.0)));
            let hole = Path::ellipse(p(20.0, 15.0), 6.0, 6.0);
            let b = Bling::new(
                Stone::default(),
                BlingLayout::Fill {
                    outer,
                    holes: vec![hole],
                    fill,
                },
            );
            let c = b.centers();
            assert!(c.len() > 60, "{fill:?}: {}", c.len());
            let r = b.radius();
            for q in &c {
                // Inside the box and out of the hole, with the whole stone.
                assert!(q.x >= r - 0.05 && q.x <= 40.0 - r + 0.05 && q.y >= r - 0.05 && q.y <= 30.0 - r + 0.05);
                assert!(q.distance(p(20.0, 15.0)) >= 6.0 + r - 0.05, "{fill:?} {q:?}");
            }
            if matches!(fill, BlingFill::Rows { .. }) {
                // Rows: no two stones nearer than a step.
                for i in 0..c.len() {
                    for j in i + 1..c.len() {
                        assert!(c[i].distance(c[j]) >= b.step() - 1e-6);
                    }
                }
            }
        }
    }

    #[test]
    fn overlaps_are_removed_or_marked() {
        let mut d = Design::new("t");
        let t = d.add_thread("Stones", Rgb(0, 0, 0));
        let line = |y: f64| BlingLayout::Run {
            path: Path::new().move_to(p(0.0, y)).line_to(p(30.0, y)),
        };
        let a = d.add_object(t, ObjectKind::Bling(Box::new(Bling::new(Stone::default(), line(0.0)))));
        // A second run 1 mm lower: all its stones overlap the first run.
        let mut second = Bling::new(Stone::default(), line(1.0));
        d.add_object(t, ObjectKind::Bling(Box::new(second.clone())));
        let s = d.stones();
        assert_eq!(s.len(), 10, "the second run is left out");
        assert!(s.iter().all(|x| x.object == a && !x.overlaps));
        // With overlaps kept: 20 stones, all marked.
        second.remove_overlaps = false;
        let id = d.objects()[1].id;
        d.object_mut(id).unwrap().kind = ObjectKind::Bling(Box::new(second));
        let s = d.stones();
        assert_eq!(s.len(), 20);
        assert!(s.iter().all(|x| x.overlaps));
    }

    #[test]
    fn transforms_keep_the_stone_size() {
        let mut b = Bling::new(
            Stone::default(),
            BlingLayout::Run {
                path: Path::new().move_to(p(0.0, 0.0)).line_to(p(10.0, 0.0)),
            },
        );
        b.transform(&Transform::scale(3.0, 3.0));
        assert!((b.radius() - 1.4).abs() < 1e-9);
        assert_eq!(b.centers().len(), 10);
        assert!(b.hit_test(p(30.0, 0.5), 0.1));
    }

    #[test]
    fn saved_form_reads_back() {
        let b = Bling::new(
            Stone::default(),
            BlingLayout::Fill {
                outer: Path::rect(BBox::new(p(0.0, 0.0), p(10.0, 10.0))),
                holes: Vec::new(),
                fill: BlingFill::Rings,
            },
        );
        let json = serde_json::to_string(&b).unwrap();
        assert!(
            json.contains(r#""layout":"fill""#) && json.contains(r#""pattern":"rings""#),
            "{json}"
        );
        let back: Bling = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
