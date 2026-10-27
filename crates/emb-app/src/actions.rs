//! Edit commands of roadmap phase P2, Milestones 4 to 6: knife, holes,
//! stitch angles, break apart, branching and redwork, closest join,
//! selection tools, align and space, skew, mirror line, reference points,
//! group and lock, and color-block sequence steps.
//!
//! Each command is one undo step. Geometry is in design coordinates.

use emb_geom::region::{self, Region};
use emb_geom::{BBox, Path, Point, Polyline, Transform, math};
use emb_model::edit::scale_about;
use emb_model::{Design, DesignObject, ObjectId, ObjectKind, RunPass, SatinGeometry, ThreadId};
use emb_stitch::travel::Shape;

use crate::app::AppState;
use crate::i18n::tr;

const FLAT_MM: f64 = 0.05;
/// Stitch angle lines of a satin from a direction line are this far apart, in mm.
const DIRECTION_STEP_MM: f64 = 2.0;
/// A half plane is a square this large, in mm.
const FAR_MM: f64 = 1e4;

/// Alignment of the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
}

pub(crate) fn polylines(o: &DesignObject) -> Vec<Polyline> {
    o.kind.paths().flat_map(|p| p.flatten(FLAT_MM)).collect()
}

pub(crate) fn path_of(pts: &[Point], closed: bool) -> Path {
    if closed {
        return Path::polygon(pts);
    }
    pts[1..].iter().fold(Path::new().move_to(pts[0]), |p, q| p.line_to(*q))
}

/// Positive on one side of the line through `a` with direction `d`, negative on the other.
fn side(a: Point, d: Point, p: Point) -> f64 {
    d.cross(p - a)
}

/// Cuts a polyline where it crosses the line. Returns the pieces and their side.
fn cut_polyline(pts: &[Point], a: Point, d: Point) -> Vec<(Vec<Point>, bool)> {
    let mut out = Vec::new();
    let mut cur = vec![pts[0]];
    for w in pts.windows(2) {
        let (s0, s1) = (side(a, d, w[0]), side(a, d, w[1]));
        if s0 * s1 < 0.0 {
            let x = w[0].lerp(w[1], s0 / (s0 - s1));
            cur.push(x);
            out.push(std::mem::replace(&mut cur, vec![x]));
        }
        cur.push(w[1]);
    }
    out.push(cur);
    out.into_iter()
        .filter(|p| p.len() >= 2)
        .map(|p| {
            let m = p[0].midpoint(p[1]);
            let s = side(a, d, m);
            (p, s > 0.0)
        })
        .collect()
}

/// The half plane on the positive (`true`) or negative side of the line.
fn half_plane(a: Point, d: Point, positive: bool) -> Region {
    let n = Point::new(-d.y, d.x) * if positive { 1.0 } else { -1.0 };
    // `side` is positive where d.cross(p - a) > 0, which is the direction (-d.y, d.x).
    let (p0, p1) = (a - d * FAR_MM, a + d * FAR_MM);
    Region::new(vec![p0, p1, p1 + n * FAR_MM, p0 + n * FAR_MM], vec![])
}

pub(crate) fn fill_regions(outer: &Path, holes: &[Path]) -> Vec<Region> {
    let outers = outer.flatten(FLAT_MM);
    let holes: Vec<Polyline> = holes.iter().flat_map(|h| h.flatten(FLAT_MM)).collect();
    Shape::new(&outers, &holes).regions
}

fn fill_kind(r: &Region, params: &emb_model::FillParams) -> ObjectKind {
    ObjectKind::Fill {
        outer: Path::polygon(&r.outer),
        holes: r.holes.iter().map(|h| Path::polygon(h)).collect(),
        params: params.clone(),
    }
}

/// Splits one object by the line. `None` if the line does not cut it.
pub(crate) fn knife_object(o: &DesignObject, a: Point, d: Point, overlap_mm: f64) -> Option<Vec<ObjectKind>> {
    match &o.kind {
        // The knife does not cut appliqué (its pieces sew together),
        // lettering or vectors (vectors: shaping, P6 Milestone 2).
        ObjectKind::Applique { .. }
        | ObjectKind::Lettering(_)
        | ObjectKind::Vector { .. }
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => None,
        ObjectKind::Fill { outer, holes, params } => {
            let regions = fill_regions(outer, holes);
            let mut parts = Vec::new();
            for positive in [true, false] {
                // Automatic knife: each piece goes half the overlap past the
                // line, under the other piece.
                let n = Point::new(-d.y, d.x) * if positive { 1.0 } else { -1.0 };
                let at = a - n * (overlap_mm / 2.0);
                let cut = region::intersection(&regions, &[half_plane(at, d, positive)]);
                parts.extend(cut.iter().filter(|r| r.area() > 0.01).map(|r| fill_kind(r, params)));
            }
            (parts.len() > 1).then_some(parts)
        }
        ObjectKind::Running { params, .. } => {
            let mut sides: [Vec<Vec<Point>>; 2] = [Vec::new(), Vec::new()];
            for pl in polylines(o) {
                let mut pts = pl.points.clone();
                if pl.closed {
                    pts.push(pts[0]);
                }
                for (piece, positive) in cut_polyline(&pts, a, d) {
                    sides[usize::from(positive)].push(piece);
                }
            }
            if sides.iter().any(Vec::is_empty) {
                return None;
            }
            Some(
                sides
                    .iter()
                    .map(|pieces| {
                        let mut path = Path::new();
                        for p in pieces {
                            path.cmds.extend(path_of(p, false).cmds);
                        }
                        ObjectKind::Running {
                            path,
                            params: params.clone(),
                        }
                    })
                    .collect(),
            )
        }
        ObjectKind::Satin { geometry, params } => {
            let rail = |p: &Path| -> Vec<Point> { p.flatten(FLAT_MM).into_iter().flat_map(|l| l.points).collect() };
            let (ra, rb) = (rail(&geometry.rail_a), rail(&geometry.rail_b));
            let (ca, cb) = (cut_polyline(&ra, a, d), cut_polyline(&rb, a, d));
            if ca.len() != 2 || cb.len() != 2 {
                return None;
            }
            let cut_rung = (ca[0].0[ca[0].0.len() - 1], cb[0].0[cb[0].0.len() - 1]);
            let mut parts = Vec::new();
            for k in 0..2 {
                let positive = ca[k].1;
                let mut rungs: Vec<(Point, Point)> = geometry
                    .rungs
                    .iter()
                    .copied()
                    .filter(|(p, q)| (side(a, d, p.midpoint(*q)) > 0.0) == positive)
                    .collect();
                rungs.push(cut_rung);
                parts.push(ObjectKind::Satin {
                    geometry: SatinGeometry {
                        rail_a: path_of(&ca[k].0, false),
                        rail_b: path_of(&cb[k].0, false),
                        rungs,
                    },
                    params: params.clone(),
                });
            }
            Some(parts)
        }
        // Stitches from a machine file: use Split at needle or Break apart.
        ObjectKind::Stitches { .. } => None,
    }
}

/// A stitch object path from needle point lists (one subpath each).
pub(crate) fn stitches_path(subs: &[Vec<Point>]) -> Path {
    let mut path = Path::new();
    for s in subs {
        if let Some((&first, rest)) = s.split_first() {
            path = path.move_to(first);
            for &p in rest {
                path = path.line_to(p);
            }
        }
    }
    path
}

/// Replaces object `id` by `parts` at the same place in the sew order. Returns the new ids.
/// Puts `parts` in the place of object `id`. The parts keep its thread, group
/// and machine functions; only the last part keeps a stop.
pub(crate) fn replace_with(d: &mut Design, id: ObjectId, parts: Vec<ObjectKind>) -> Vec<ObjectId> {
    let Some(i) = d.index_of(id) else { return Vec::new() };
    let Some(old) = d.remove_object(id) else {
        return Vec::new();
    };
    let n = parts.len();
    parts
        .into_iter()
        .enumerate()
        .map(|(k, kind)| {
            let new = d.insert_object(i + k, old.thread, kind);
            if let Some(o) = d.object_mut(new) {
                o.group = old.group;
                o.functions = old.functions;
                o.functions.stop = old.functions.stop && k + 1 == n;
            }
            new
        })
        .collect()
}

fn segments_cross(p: Point, q: Point, a: Point, b: Point) -> bool {
    let d1 = (q - p).cross(a - p);
    let d2 = (q - p).cross(b - p);
    let d3 = (b - a).cross(p - a);
    let d4 = (b - a).cross(q - a);
    d1 * d2 <= 0.0 && d3 * d4 <= 0.0 && (d1 != 0.0 || d2 != 0.0)
}

fn object_crosses(o: &DesignObject, line: &[Point]) -> bool {
    polylines(o).iter().any(|pl| {
        pl.segments()
            .any(|(a, b)| line.windows(2).any(|w| segments_cross(w[0], w[1], a, b)))
    })
}

/// The point of `a` nearest to `b`, and the point of `b` nearest to it.
fn closest_points(a: &[Polyline], b: &[Polyline]) -> Option<(Point, Point)> {
    let mut best: Option<(f64, Point, Point)> = None;
    let mut test = |from: &[Polyline], to: &[Polyline], swap: bool| {
        for pl in from {
            for &p in &pl.points {
                for t in to {
                    for (s, e) in t.segments() {
                        let d = e - s;
                        let l2 = d.length_sq();
                        let u = if l2 > 0.0 {
                            ((p - s).dot(d) / l2).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let q = s.lerp(e, u);
                        let dist = p.distance(q);
                        if best.is_none_or(|x| dist < x.0) {
                            best = Some(if swap { (dist, q, p) } else { (dist, p, q) });
                        }
                    }
                }
            }
        }
    };
    test(a, b, false);
    test(b, a, true);
    best.map(|(_, p, q)| (p, q))
}

impl AppState {
    pub(crate) fn sel(&self) -> Vec<ObjectId> {
        self.selection.in_sew_order(self.doc.design())
    }

    /// Knife: cuts the selected objects (or all objects that the line
    /// crosses, if nothing is selected) along the line through `a` and `b`.
    pub fn knife(&mut self, a: Point, b: Point) {
        let Some(d) = (b - a).normalized() else { return };
        let design = self.doc.design();
        let mut ids = self.sel();
        if ids.is_empty() {
            ids = design
                .objects()
                .iter()
                .filter(|o| !o.locked && object_crosses(o, &[a, b]))
                .map(|o| o.id)
                .collect();
        }
        if ids.iter().any(|id| design.is_linked(*id)) {
            self.message = emb_model::ShapeError::AlreadyLinked.to_string();
            return;
        }
        let cuts: Vec<(ObjectId, Vec<ObjectKind>)> = ids
            .iter()
            .filter_map(|id| design.object(*id))
            .filter_map(|o| knife_object(o, a, d, self.shaping.overlap_mm).map(|p| (o.id, p)))
            .collect();
        if cuts.is_empty() {
            self.message = tr("The knife line does not cut the objects.").into();
            return;
        }
        let new = self.doc.edit("Knife", |d| {
            cuts.into_iter()
                .flat_map(|(id, parts)| replace_with(d, id, parts))
                .collect::<Vec<_>>()
        });
        self.message = crate::i18n::fill("Cut into {} objects.", &[&(new.len())]);
        self.selection.set_all(new);
    }

    /// Sets the stitch angle of the selected fill, or adds a stitch direction line to a satin.
    pub fn stitch_angle(&mut self, a: Point, b: Point) {
        let Some(id) = self.selection.single() else {
            self.message = tr("Select one fill or satin first.").into();
            return;
        };
        let v = b - a;
        if v.length() < 1e-6 {
            return;
        }
        // Fill angles are 0° to 180° (a row has no direction).
        let angle = math::to_degrees(math::atan2(v.y, v.x)).rem_euclid(180.0);
        let label = match self.doc.design().object(id).map(|o| &o.kind) {
            Some(ObjectKind::Fill { .. }) => "Set stitch angle",
            Some(ObjectKind::Satin { .. }) => "Add stitch angle",
            _ => {
                self.message = tr("Stitch angles are for fills and satins.").into();
                return;
            }
        };
        self.doc.edit(label, |d| match d.object_mut(id).map(|o| &mut o.kind) {
            Some(ObjectKind::Fill { params, .. }) => params.angle_deg = angle,
            Some(ObjectKind::Satin { geometry, .. }) => geometry.rungs.push((a, b)),
            _ => {}
        });
        self.message = label.into();
    }

    /// Direction line (beta tester request, 2026-10-24): the stitches of the
    /// selected object go across the drawn line at the angle in the property
    /// bar (90°: square to it), so they turn with the line.
    ///
    /// - A satin gets new stitch angle lines along the line; its rails stay.
    /// - A fill with a column stitch type (satin and the others) keeps the
    ///   line, so it still works when the shape changes.
    /// - Another fill takes the angle of the whole line as its fill angle.
    pub fn direction_line(&mut self, pts: &[Point]) {
        let Some(id) = self.selection.single() else {
            self.message = tr("Select one fill or satin first.").into();
            return;
        };
        if pts.len() < 2 || pts[0].distance(pts[pts.len() - 1]) < 1e-6 {
            return;
        }
        let angle = self.settings.defaults.direction_angle_deg;
        let Some(kind) = self.doc.design().object(id).map(|o| o.kind.clone()) else {
            return;
        };
        match kind {
            ObjectKind::Satin { geometry, .. } => {
                let Some(g) = geometry.with_direction(pts, angle, DIRECTION_STEP_MM) else {
                    self.message = tr("The direction line must go through the column.").into();
                    return;
                };
                self.doc.edit("Direction Line", |d| {
                    if let Some(ObjectKind::Satin { geometry, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        *geometry = g;
                    }
                });
                self.message = tr("Direction line: the stitches go across the line.").into();
            }
            ObjectKind::Fill { params, .. }
                if matches!(
                    params.pattern,
                    emb_model::FillPattern::Columns { .. } | emb_model::FillPattern::FlexiSplit { .. }
                ) =>
            {
                let dir = emb_model::DirectionLine {
                    points: pts.to_vec(),
                    angle_deg: angle,
                };
                self.doc.edit("Direction Line", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        params.direction = Some(dir);
                    }
                });
                self.message =
                    "Direction line: the stitches go across the line. Object Properties can change or remove it."
                        .into();
            }
            ObjectKind::Fill { .. } => {
                let v = pts[pts.len() - 1] - pts[0];
                let fill_angle = (math::to_degrees(math::atan2(v.y, v.x)) + angle).rem_euclid(180.0);
                self.doc.edit("Direction Line", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        params.angle_deg = fill_angle;
                    }
                });
                self.message = tr(
                    "The fill angle follows the line. For stitches that turn with the line, \
                                set the stitch type to Satin first.",
                )
                .into();
            }
            _ => self.message = tr("Direction lines are for fills and satins.").into(),
        }
    }

    /// Adds a hole to the selected fill.
    pub fn add_hole(&mut self, pts: &[Point]) {
        let Some(id) = self.selection.single() else {
            self.message = tr("Select one fill first.").into();
            return;
        };
        if !matches!(
            self.doc.design().object(id).map(|o| &o.kind),
            Some(ObjectKind::Fill { .. })
        ) {
            self.message = tr("Holes are for fills.").into();
            return;
        }
        let hole = Path::polygon(pts);
        self.doc.edit("Add Hole", |d| {
            if let Some(ObjectKind::Fill { holes, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                holes.push(hole);
            }
        });
    }

    /// Removes the holes of the selected fills.
    pub fn remove_holes(&mut self) {
        let ids = self.sel();
        self.doc.edit("Remove Holes", |d| {
            for id in &ids {
                if let Some(ObjectKind::Fill { holes, .. }) = d.object_mut(*id).map(|o| &mut o.kind) {
                    holes.clear();
                }
            }
        });
    }

    /// Makes a new fill object in each hole of the selected fills. The holes stay.
    pub fn fill_holes(&mut self) {
        let ids = self.sel();
        let new = self.doc.edit("Fill Holes", |d| {
            let mut new = Vec::new();
            for id in &ids {
                let Some(o) = d.object(*id).cloned() else { continue };
                let ObjectKind::Fill { holes, params, .. } = &o.kind else {
                    continue;
                };
                let at = d.index_of(*id).unwrap_or(0) + 1;
                for (k, h) in holes.iter().enumerate() {
                    let kind = ObjectKind::Fill {
                        outer: h.clone(),
                        holes: vec![],
                        params: params.clone(),
                    };
                    new.push(d.insert_object(at + k, o.thread, kind));
                }
            }
            new
        });
        if new.is_empty() {
            self.message = tr("The selection has no holes.").into();
        } else {
            self.selection.set_all(new);
        }
    }

    /// Splits objects with several parts into one object per part: lines
    /// per subpath, fills per separate area (with their holes).
    pub fn break_apart(&mut self) {
        if self.refuse_linked(&self.sel()) {
            return;
        }
        let design = self.doc.design();
        let parts: Vec<(ObjectId, Vec<ObjectKind>)> = self
            .sel()
            .into_iter()
            .filter_map(|id| design.object(id))
            .filter_map(|o| {
                let parts: Vec<ObjectKind> = match &o.kind {
                    ObjectKind::Running { path, params } => {
                        let subs = emb_geom::nodes::NodePath::from_path(path);
                        subs.subs
                            .iter()
                            .map(|s| ObjectKind::Running {
                                path: emb_geom::nodes::NodePath { subs: vec![s.clone()] }.to_path(),
                                params: params.clone(),
                            })
                            .collect()
                    }
                    ObjectKind::Fill { outer, holes, params } => fill_regions(outer, holes)
                        .iter()
                        .map(|r| fill_kind(r, params))
                        .collect(),
                    ObjectKind::Satin { .. } => Vec::new(),
                    // One appliqué object for each piece.
                    ObjectKind::Applique { pieces, params } => pieces
                        .iter()
                        .map(|p| ObjectKind::Applique {
                            pieces: vec![p.clone()],
                            params: params.clone(),
                        })
                        .collect(),
                    // Lettering becomes its satin and fill parts (no longer text).
                    ObjectKind::Lettering(l) => l.parts().cloned().collect(),
                    // A filled vector: one for each shape with its holes; else
                    // one for each path.
                    ObjectKind::Vector { path, style } => {
                        let pieces: Vec<Path> = if style.fill.is_some() {
                            let closed: Vec<Path> = path.subpaths().into_iter().map(Path::closed).collect();
                            region::nest(&closed)
                                .into_iter()
                                .map(|(o, hs)| Path::joined(std::iter::once(&o).chain(&hs)))
                                .collect()
                        } else {
                            path.subpaths()
                        };
                        pieces
                            .into_iter()
                            .map(|path| ObjectKind::Vector { path, style: *style })
                            .collect()
                    }
                    // One object for each group of cells that touch, and
                    // one for the backstitch lines.
                    ObjectKind::CrossStitch(x) => x
                        .components()
                        .into_iter()
                        .map(|c| ObjectKind::CrossStitch(Box::new(c)))
                        .collect(),
                    // A run: one object for each line.
                    ObjectKind::Bling(b) => match &b.layout {
                        emb_model::BlingLayout::Run { path } => path
                            .subpaths()
                            .into_iter()
                            .map(|path| {
                                ObjectKind::Bling(Box::new(emb_model::Bling {
                                    layout: emb_model::BlingLayout::Run { path },
                                    ..(**b).clone()
                                }))
                            })
                            .collect(),
                        _ => Vec::new(),
                    },
                    // A sketch stays one object (its regions become objects, P14 Milestone 4).
                    ObjectKind::Sketch(_) => Vec::new(),
                    // One object for each part between jumps.
                    ObjectKind::Stitches { path, .. } => emb_stitch::stitch_subpaths(path)
                        .iter()
                        .map(|s| ObjectKind::Stitches {
                            path: stitches_path(std::slice::from_ref(s)),
                            trims: Vec::new(),
                        })
                        .collect(),
                };
                (parts.len() > 1).then_some((o.id, parts))
            })
            .collect();
        // Grouped objects come out of their group too, so each one can be
        // selected alone (for example the result of Picture to design).
        let sel = self.sel();
        let grouped: Vec<ObjectId> = sel
            .iter()
            .copied()
            .filter(|id| design.object(*id).is_some_and(|o| o.group.is_some()))
            .collect();
        if parts.is_empty() && grouped.is_empty() {
            self.message = tr("Nothing to break apart: the selection has one part per object.").into();
            return;
        }
        let split: Vec<ObjectId> = parts.iter().map(|(id, _)| *id).collect();
        let (n_split, n_grouped) = (parts.len(), grouped.len());
        let new = self.doc.edit("Break Apart", |d| {
            for id in &grouped {
                if let Some(o) = d.object_mut(*id) {
                    o.group = None;
                }
            }
            parts
                .into_iter()
                .flat_map(|(id, p)| replace_with(d, id, p))
                .collect::<Vec<_>>()
        });
        let mut keep: Vec<ObjectId> = sel.into_iter().filter(|id| !split.contains(id)).collect();
        keep.extend(new);
        self.selection.set_all(keep);
        self.message = match (n_split, n_grouped) {
            (0, g) => format!("{g} objects are out of their group: click one to edit it alone."),
            (s, 0) => format!("Broke {s} objects apart."),
            (s, g) => format!("Broke {s} objects apart; {g} objects are out of their group."),
        };
    }

    /// Joins the selected running lines into one object sewn as a network
    /// (redwork or branching). The object takes the place of the first line.
    pub fn make_network(&mut self, pass: RunPass) {
        let design = self.doc.design();
        let lines: Vec<DesignObject> = self
            .sel()
            .into_iter()
            .filter_map(|id| design.object(id).cloned())
            .filter(|o| matches!(o.kind, ObjectKind::Running { .. }))
            .collect();
        let Some(first) = lines.first() else {
            self.message = tr("Select running lines first.").into();
            return;
        };
        let mut path = Path::new();
        for o in &lines {
            if let ObjectKind::Running { path: p, .. } = &o.kind {
                path.cmds.extend(p.cmds.iter().copied());
            }
        }
        let ObjectKind::Running { params, .. } = &first.kind else {
            return;
        };
        let mut params = params.clone();
        params.pass = pass;
        let kind = ObjectKind::Running { path, params };
        let (first_id, others): (ObjectId, Vec<ObjectId>) = (first.id, lines[1..].iter().map(|o| o.id).collect());
        let label = if pass == RunPass::Branch {
            "Branching"
        } else {
            "Redwork"
        };
        self.doc.edit(label, |d| {
            if let Some(o) = d.object_mut(first_id) {
                o.kind = kind;
                o.start = None;
                o.end = None;
            }
            for id in &others {
                d.remove_object(*id);
            }
        });
        self.selection.set(first_id);
    }

    /// Closest join: each object ends at the point nearest to the next one,
    /// and the next one starts there. For the selection in sew order, or for
    /// all objects if fewer than two are selected.
    pub fn closest_join(&mut self) {
        let design = self.doc.design();
        let mut ids = self.sel();
        if ids.len() < 2 {
            ids = design.objects().iter().map(|o| o.id).collect();
        }
        let objs: Vec<DesignObject> = ids.iter().filter_map(|id| design.object(*id).cloned()).collect();
        let mut points: Vec<(ObjectId, ObjectId, Point, Point)> = Vec::new();
        for w in objs.windows(2) {
            if let Some((p, q)) = closest_points(&polylines(&w[0]), &polylines(&w[1])) {
                points.push((w[0].id, w[1].id, p, q));
            }
        }
        self.doc.edit("Closest Join", |d| {
            for (a, b, p, q) in &points {
                if let Some(o) = d.object_mut(*a) {
                    o.end = Some(*p);
                }
                if let Some(o) = d.object_mut(*b) {
                    o.start = Some(*q);
                }
            }
        });
        self.message = crate::i18n::fill("Joined {} objects at their closest points.", &[&(objs.len())]);
    }

    /// Selects the objects whose outlines are fully inside the polygon.
    pub fn select_inside(&mut self, polygon: &[Point]) {
        let ids: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| !o.locked && self.view_filter.shows(o))
            .filter(|o| {
                polylines(o)
                    .iter()
                    .all(|pl| pl.points.iter().all(|p| region::ring_contains(polygon, *p)))
            })
            .map(|o| o.id)
            .collect();
        self.message = crate::i18n::fill("{} selected", &[&(ids.len())]);
        self.selection.set_all(ids);
    }

    /// Selects the objects that the polyline crosses.
    pub fn select_crossing(&mut self, line: &[Point]) {
        let ids: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| !o.locked && self.view_filter.shows(o) && object_crosses(o, line))
            .map(|o| o.id)
            .collect();
        self.message = crate::i18n::fill("{} selected", &[&(ids.len())]);
        self.selection.set_all(ids);
    }

    /// Selects all objects with this thread.
    pub fn select_by_thread(&mut self, t: ThreadId) {
        let ids: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| o.thread == t && !o.locked)
            .map(|o| o.id)
            .collect();
        self.selection.set_all(ids);
    }

    /// Selects all objects of a stitch type ("Running", "Satin", "Fill").
    pub fn select_by_type(&mut self, type_name: &str) {
        let ids: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| o.type_name() == type_name && !o.locked)
            .map(|o| o.id)
            .collect();
        self.selection.set_all(ids);
    }

    /// Selects the next (or the previous) object in sew order.
    pub fn select_step(&mut self, forward: bool) {
        let objs = self.doc.design().objects();
        if objs.is_empty() {
            return;
        }
        let n = objs.len();
        let cur = self.selection.primary().and_then(|id| self.doc.design().index_of(id));
        let i = match (cur, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        };
        let id = objs[i].id;
        self.selection.set(id);
    }

    pub fn align(&mut self, how: Align) {
        let ids = self.sel();
        let design = self.doc.design();
        let Some(all) = design.bbox_of(&ids) else { return };
        let moves: Vec<(ObjectId, Point)> = ids
            .iter()
            .filter_map(|id| design.bbox_of(&[*id]).map(|b| (*id, b)))
            .map(|(id, b)| {
                let d = match how {
                    Align::Left => Point::new(all.min.x - b.min.x, 0.0),
                    Align::Right => Point::new(all.max.x - b.max.x, 0.0),
                    Align::CenterX => Point::new(all.center().x - b.center().x, 0.0),
                    Align::Top => Point::new(0.0, all.min.y - b.min.y),
                    Align::Bottom => Point::new(0.0, all.max.y - b.max.y),
                    Align::CenterY => Point::new(0.0, all.center().y - b.center().y),
                };
                (id, d)
            })
            .collect();
        self.doc.edit("Align", |d| {
            for (id, m) in &moves {
                d.transform_objects(&[*id], &Transform::translate(m.x, m.y));
            }
        });
    }

    /// Spaces the selected objects evenly (equal gaps), left to right or top to bottom.
    pub fn space_evenly(&mut self, horizontal: bool) {
        let ids = self.sel();
        let design = self.doc.design();
        let mut boxes: Vec<(ObjectId, BBox)> = ids
            .iter()
            .filter_map(|id| design.bbox_of(&[*id]).map(|b| (*id, b)))
            .collect();
        if boxes.len() < 3 {
            self.message = tr("Select three or more objects to space them.").into();
            return;
        }
        let key = |b: &BBox| if horizontal { b.center().x } else { b.center().y };
        boxes.sort_by(|a, b| key(&a.1).total_cmp(&key(&b.1)));
        let size = |b: &BBox| if horizontal { b.width() } else { b.height() };
        let lo = |b: &BBox| if horizontal { b.min.x } else { b.min.y };
        let hi = |b: &BBox| if horizontal { b.max.x } else { b.max.y };
        let span = hi(&boxes[boxes.len() - 1].1) - lo(&boxes[0].1);
        let total: f64 = boxes.iter().map(|(_, b)| size(b)).sum();
        let gap = (span - total) / (boxes.len() - 1) as f64;
        let mut at = lo(&boxes[0].1);
        let mut moves = Vec::new();
        for (id, b) in &boxes {
            let d = at - lo(b);
            moves.push((
                *id,
                if horizontal {
                    Point::new(d, 0.0)
                } else {
                    Point::new(0.0, d)
                },
            ));
            at += size(b) + gap;
        }
        self.doc.edit("Space Evenly", |d| {
            for (id, m) in &moves {
                d.transform_objects(&[*id], &Transform::translate(m.x, m.y));
            }
        });
    }

    /// Scales each selected object about its center to the width and/or
    /// height of the first selected object.
    pub fn make_same_size(&mut self, width: bool, height: bool) {
        let ids = self.selection.ids().to_vec();
        let design = self.doc.design();
        let Some(r) = ids.first().and_then(|id| design.bbox_of(&[*id])) else {
            return;
        };
        let scales: Vec<(ObjectId, Transform)> = ids[1..]
            .iter()
            .filter_map(|id| design.bbox_of(&[*id]).map(|b| (*id, b)))
            .map(|(id, b)| {
                let sx = if width && b.width() > 1e-9 {
                    r.width() / b.width()
                } else {
                    1.0
                };
                let sy = if height && b.height() > 1e-9 {
                    r.height() / b.height()
                } else {
                    1.0
                };
                (id, scale_about(b.center(), sx, sy))
            })
            .collect();
        self.doc.edit("Make Same Size", |d| {
            for (id, t) in &scales {
                d.transform_objects(&[*id], t);
            }
        });
    }

    /// Skews the selection about its center: `x_deg` slants vertical lines, `y_deg` horizontal lines.
    pub fn skew_selection(&mut self, x_deg: f64, y_deg: f64) {
        let shear = Transform {
            a: 1.0,
            b: math::tan(math::to_radians(y_deg)),
            c: math::tan(math::to_radians(x_deg)),
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };
        self.transform_selection("Skew", |b| {
            let c = b.center();
            Transform::translate(-c.x, -c.y)
                .then(shear)
                .then(Transform::translate(c.x, c.y))
        });
    }

    /// Mirrors the selection about the line through `a` and `b`. With Ctrl
    /// held, a mirrored copy is made and the original stays.
    pub fn mirror_about_line(&mut self, a: Point, b: Point) {
        let Some(d) = (b - a).normalized() else { return };
        if self.selection.is_empty() {
            self.message = tr("Select objects first.").into();
            return;
        }
        let angle = math::to_degrees(math::atan2(d.y, d.x));
        let t = Transform::translate(-a.x, -a.y)
            .then(Transform::rotate_deg(-angle))
            .then(Transform::scale(1.0, -1.0))
            .then(Transform::rotate_deg(angle))
            .then(Transform::translate(a.x, a.y));
        if self.mods.command || self.reflect_copy {
            let ids = self.sel();
            let new = self.doc.edit("Mirror copy", |d| {
                let new = d.duplicate_objects(&ids, Point::ZERO);
                d.transform_objects(&new, &t);
                new
            });
            self.selection.set_all(new);
        } else {
            self.transform_selection("Mirror about line", |_| t);
        }
    }

    /// Moves, turns and scales the selection so that `p1` goes to `q1` and `p2` to `q2`.
    pub fn transform_by_points(&mut self, p1: Point, p2: Point, q1: Point, q2: Point) {
        let (u, v) = (p2 - p1, q2 - q1);
        if u.length() < 1e-6 || v.length() < 1e-6 {
            return;
        }
        let s = v.length() / u.length();
        let angle = math::to_degrees(math::atan2(v.y, v.x) - math::atan2(u.y, u.x));
        let t = Transform::translate(-p1.x, -p1.y)
            .then(Transform::rotate_deg(angle))
            .then(Transform::scale(s, s))
            .then(Transform::translate(q1.x, q1.y));
        self.transform_selection("Transform by points", |_| t);
    }

    /// Groups the selected objects.
    pub fn group_selection(&mut self) {
        let ids = self.sel();
        if ids.len() < 2 {
            self.message = tr("Select two or more objects to group them.").into();
            return;
        }
        self.doc.edit("Group", |d| {
            let g = d.new_group_id();
            for id in &ids {
                if let Some(o) = d.object_mut(*id) {
                    o.group = Some(g);
                }
            }
        });
    }

    pub fn ungroup_selection(&mut self) {
        let ids = self.sel();
        self.doc.edit("Ungroup", |d| {
            for id in &ids {
                if let Some(o) = d.object_mut(*id) {
                    o.group = None;
                }
            }
        });
    }

    /// Locks the selected objects. Locked objects cannot be selected on the canvas.
    pub fn lock_selection(&mut self) {
        let ids = self.sel();
        self.doc.edit("Lock", |d| {
            for id in &ids {
                if let Some(o) = d.object_mut(*id) {
                    o.locked = true;
                }
            }
        });
        self.selection.clear();
    }

    pub fn unlock_all(&mut self) {
        let locked: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| o.locked)
            .map(|o| o.id)
            .collect();
        if locked.is_empty() {
            return;
        }
        self.doc.edit("Unlock All", |d| {
            for id in &locked {
                if let Some(o) = d.object_mut(*id) {
                    o.locked = false;
                }
            }
        });
        self.selection.set_all(locked);
    }

    /// Adds the other members of selected groups to the selection, and
    /// removes locked and deleted objects from it. In a group that the user
    /// entered (a double-click), the objects stay one at a time while all
    /// the selected objects are in that group.
    pub fn apply_groups_and_locks(&mut self) {
        let d = self.doc.design();
        let ids = self.selection.ids().to_vec();
        let entered = self
            .selection
            .entered_group()
            .filter(|g| !ids.is_empty() && ids.iter().all(|id| d.object(*id).is_some_and(|o| o.group == Some(*g))));
        if entered.is_none() && self.selection.entered_group().is_some() {
            self.selection.leave_group();
        }
        let groups: Vec<u32> = ids
            .iter()
            .filter_map(|id| d.object(*id).and_then(|o| o.group))
            .filter(|g| Some(*g) != entered)
            .collect();
        let mut want: Vec<ObjectId> = ids
            .iter()
            .copied()
            .filter(|id| d.object(*id).is_some_and(|o| !o.locked))
            .collect();
        for o in d.objects() {
            if o.group.is_some_and(|g| groups.contains(&g)) && !want.contains(&o.id) && !o.locked {
                want.push(o.id);
            }
        }
        if want != ids {
            self.selection.set_all(want);
        }
    }

    /// Moves the selection in front of the color block before it (`back`),
    /// or after the color block after it.
    pub fn step_color(&mut self, back: bool) {
        let ids = self.sel();
        if ids.is_empty() {
            return;
        }
        let d = self.doc.design();
        let blocks = d.color_blocks();
        let block_of = |id: ObjectId| blocks.iter().position(|(_, b)| b.contains(&id));
        let target: Option<Option<ObjectId>> = if back {
            let k = block_of(ids[0]).unwrap_or(0);
            (k > 0).then(|| blocks[k - 1].1.first().copied())
        } else {
            let k = block_of(ids[ids.len() - 1]).unwrap_or(0);
            (k + 1 < blocks.len()).then(|| blocks.get(k + 2).and_then(|b| b.1.first().copied()))
        };
        let Some(before) = target else { return };
        let label = if back { "Back One Color" } else { "Forward One Color" };
        self.resequence(label, |d| d.move_objects_before(&ids, before));
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;
    use emb_model::{FillParams, Rgb, RunningParams};

    use super::*;
    use crate::test_util::Harness;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn add(h: &mut Harness, kind: ObjectKind) -> ObjectId {
        h.st.doc.edit("Add", |d| {
            let t = d.thread_for_color(Rgb(200, 0, 0));
            d.add_object(t, kind)
        })
    }

    fn square_fill(x: f64, y: f64, s: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(x, y), p(x + s, y + s))),
            holes: vec![],
            params: FillParams::default(),
        }
    }

    fn line(a: Point, b: Point) -> ObjectKind {
        ObjectKind::Running {
            path: Path::new().move_to(a).line_to(b),
            params: RunningParams::default(),
        }
    }

    #[test]
    fn knife_cuts_a_fill_and_a_line() {
        let mut h = Harness::empty();
        let f = add(&mut h, square_fill(0.0, 0.0, 20.0));
        let l = add(&mut h, line(p(0.0, 30.0), p(20.0, 30.0)));
        h.st.selection.set_all([f, l]);
        h.st.knife(p(10.0, -5.0), p(10.0, 40.0));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 4);
        let areas: Vec<f64> = d
            .objects()
            .iter()
            .filter(|o| o.type_name() == "Fill")
            .map(|o| o.control_bbox().unwrap().width())
            .collect();
        assert_eq!(areas.len(), 2);
        assert!(areas.iter().all(|w| (w - 10.0).abs() < 1e-6), "{areas:?}");
        h.st.undo();
        assert_eq!(h.st.doc.design().objects().len(), 2);
    }

    #[test]
    fn knife_cuts_a_satin_into_two_columns() {
        let mut h = Harness::empty();
        let geometry = SatinGeometry::from_centerline(&Path::new().move_to(p(0.0, 0.0)).line_to(p(40.0, 0.0)), 4.0);
        let s = add(
            &mut h,
            ObjectKind::Satin {
                geometry,
                params: Default::default(),
            },
        );
        h.st.selection.set(s);
        h.st.knife(p(20.0, -10.0), p(20.0, 10.0));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        for o in d.objects() {
            let b = o.control_bbox().unwrap();
            assert!((b.width() - 20.0).abs() < 1e-6, "{b:?}");
        }
    }

    #[test]
    fn holes_add_remove_fill() {
        let mut h = Harness::empty();
        let f = add(&mut h, square_fill(0.0, 0.0, 30.0));
        h.st.selection.set(f);
        h.st.add_hole(&[p(10.0, 10.0), p(20.0, 10.0), p(20.0, 20.0), p(10.0, 20.0)]);
        h.st.fill_holes();
        assert_eq!(h.st.doc.design().objects().len(), 2);
        h.st.selection.set(f);
        h.st.remove_holes();
        let ObjectKind::Fill { holes, .. } = &h.st.doc.design().object(f).unwrap().kind else {
            panic!()
        };
        assert!(holes.is_empty());
    }

    #[test]
    fn stitch_angle_sets_fill_angle() {
        let mut h = Harness::empty();
        let f = add(&mut h, square_fill(0.0, 0.0, 30.0));
        h.st.selection.set(f);
        h.st.stitch_angle(p(0.0, 0.0), p(10.0, 10.0));
        let ObjectKind::Fill { params, .. } = &h.st.doc.design().object(f).unwrap().kind else {
            panic!()
        };
        assert!((params.angle_deg - 45.0).abs() < 1e-9);
    }

    #[test]
    fn direction_line_turns_satins_and_satin_fills() {
        let mut h = Harness::empty();
        // A level bar as a satin column: the line goes along it.
        let bar = SatinGeometry {
            rail_a: Path::new().move_to(p(0.0, 0.0)).line_to(p(40.0, 0.0)),
            rail_b: Path::new().move_to(p(0.0, 6.0)).line_to(p(40.0, 6.0)),
            rungs: Vec::new(),
        };
        let s = add(
            &mut h,
            ObjectKind::Satin {
                geometry: bar,
                params: Default::default(),
            },
        );
        h.st.selection.set(s);
        h.st.direction_line(&[p(2.0, 3.0), p(38.0, 3.0)]);
        let Some(ObjectKind::Satin { geometry, .. }) = h.st.doc.design().object(s).map(|o| o.kind.clone()) else {
            panic!()
        };
        assert!(geometry.rungs.len() >= 15, "{}", geometry.rungs.len());
        // Square to the line, from rail A (y = 0) to rail B (y = 6).
        for (a, b) in &geometry.rungs {
            assert!(
                (a.x - b.x).abs() < 1e-6 && a.y.abs() < 1e-6 && (b.y - 6.0).abs() < 1e-6,
                "{a:?} {b:?}"
            );
        }
        // At 45°, the rungs lean.
        h.st.settings.defaults.direction_angle_deg = 45.0;
        h.st.direction_line(&[p(2.0, 3.0), p(38.0, 3.0)]);
        let Some(ObjectKind::Satin { geometry, .. }) = h.st.doc.design().object(s).map(|o| o.kind.clone()) else {
            panic!()
        };
        // (Near the ends, a leaning line meets the end of the column first.)
        let inner: Vec<_> = geometry
            .rungs
            .iter()
            .filter(|(a, b)| a.x.min(b.x) > 0.5 && a.x.max(b.x) < 39.5)
            .collect();
        assert!(inner.len() >= 10);
        assert!(inner.iter().all(|(a, b)| ((b.x - a.x).abs() - 6.0).abs() < 1e-6));
        h.st.settings.defaults.direction_angle_deg = 90.0;

        // A satin fill keeps the line; a tatami fill takes its angle.
        let mut satin_fill = square_fill(0.0, 20.0, 30.0);
        if let ObjectKind::Fill { params, .. } = &mut satin_fill {
            *params = params.with_pattern(emb_model::FillPattern::Columns {
                column: emb_model::ColumnPattern::Satin,
            });
        }
        let f = add(&mut h, satin_fill);
        h.st.selection.set(f);
        h.st.direction_line(&[p(0.0, 20.0), p(15.0, 35.0), p(30.0, 20.0)]);
        let Some(ObjectKind::Fill { params, .. }) = h.st.doc.design().object(f).map(|o| o.kind.clone()) else {
            panic!()
        };
        let dir = params.direction.expect("a direction line");
        assert_eq!(dir.points.len(), 3);
        assert_eq!(dir.angle_deg, 90.0);
        let t = add(&mut h, square_fill(40.0, 0.0, 30.0));
        h.st.selection.set(t);
        h.st.direction_line(&[p(40.0, 0.0), p(70.0, 0.0)]);
        let Some(ObjectKind::Fill { params, .. }) = h.st.doc.design().object(t).map(|o| o.kind.clone()) else {
            panic!()
        };
        assert!(params.direction.is_none());
        assert!((params.angle_deg - 90.0).abs() < 1e-9, "rows square to the line");
    }

    #[test]
    fn shortest_order_cuts_the_moves_and_undoes_in_one_step() {
        let mut h = Harness::empty();
        // Squares far apart, drawn to and fro.
        let ids: Vec<ObjectId> = [0.0, 80.0, 20.0, 60.0, 40.0]
            .iter()
            .map(|&x| add(&mut h, square_fill(x - 40.0, -5.0, 10.0)))
            .collect();
        let before: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(before, ids);
        h.st.optimize_sew_order();
        let after: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_ne!(after, before, "{}", h.st.message);
        assert!(h.st.message.starts_with("Shortest order:"), "{}", h.st.message);
        h.st.undo();
        let undone: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(undone, before);
    }

    #[test]
    fn a_double_click_selects_one_object_of_a_border_trace() {
        use emb_geom::{BBox, Path, Point};
        let mut h = Harness::empty();
        let fill = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_thread("Black", emb_model::Rgb(0, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 30.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        h.st.selection.set(fill);
        let spec = h.st.trace_spec();
        h.st.add_border_trace(spec);
        let group =
            h.st.doc
                .design()
                .object(fill)
                .unwrap()
                .group
                .expect("the trace makes a group");
        // A click selects the fill and its border.
        h.st.selection.set(fill);
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.len(), 2);
        // A double-click selects the fill alone, and its fill type changes.
        h.st.selection.enter(fill, group);
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.single(), Some(fill));
        h.st.doc.edit("Fill type", |d| {
            if let Some(o) = d.object_mut(fill)
                && let ObjectKind::Fill { params, .. } = &mut o.kind
            {
                params.pattern = emb_model::FillPattern::all()[1].clone();
            }
        });
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.single(), Some(fill), "the edit keeps the one object");
        let d = h.st.doc.design();
        let ObjectKind::Fill { params, .. } = &d.object(fill).unwrap().kind else {
            panic!("a fill")
        };
        assert_eq!(params.pattern, emb_model::FillPattern::all()[1]);
        assert!(d.trace_of(fill).is_some(), "the trace stays");
        // A click on another object leaves the group.
        h.st.selection.clear();
        h.st.apply_groups_and_locks();
        assert!(h.st.selection.entered_group().is_none());
    }

    #[test]
    fn break_apart_takes_grouped_objects_out_of_their_group() {
        let mut h = Harness::empty();
        let sq = |x: f64| ObjectKind::Fill {
            outer: Path::rect(emb_geom::BBox::new(p(x, 0.0), p(x + 10.0, 10.0))),
            holes: Vec::new(),
            params: Default::default(),
        };
        let a = add(&mut h, sq(0.0));
        let b = add(&mut h, sq(20.0));
        h.st.doc.edit("Group", |d| {
            let g = d.new_group_id();
            for id in [a, b] {
                d.object_mut(id).unwrap().group = Some(g);
            }
        });
        // A click on one selects the group.
        h.st.selection.set(a);
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.ids().len(), 2);
        h.st.break_apart();
        let d = h.st.doc.design();
        assert!(d.objects().iter().all(|o| o.group.is_none()));
        assert_eq!(d.objects().len(), 2, "one shape each: nothing to split");
        // Now one object can be selected alone.
        h.st.selection.set(b);
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.ids(), &[b]);
    }

    #[test]
    fn break_apart_and_redwork() {
        let mut h = Harness::empty();
        let a = add(&mut h, line(p(0.0, 0.0), p(10.0, 0.0)));
        let b = add(&mut h, line(p(5.0, -5.0), p(5.0, 5.0)));
        h.st.selection.set_all([a, b]);
        h.st.make_network(RunPass::Redwork);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let ObjectKind::Running { params, path } = &d.objects()[0].kind else {
            panic!()
        };
        assert_eq!(params.pass, RunPass::Redwork);
        assert_eq!(path.flatten(0.1).len(), 2);
        // The plan sews it in one piece that ends where it starts.
        let span = h.st.plan_view().plan.spans[0].clone();
        assert!(span.start.distance(span.end) < 1e-9);
        assert_eq!(h.st.plan_view().plan.stitches.trim_count(), 1);
        h.st.selection.set(a);
        h.st.break_apart();
        assert_eq!(h.st.doc.design().objects().len(), 2);
    }

    #[test]
    fn closest_join_sets_points() {
        let mut h = Harness::empty();
        let a = add(&mut h, square_fill(0.0, 0.0, 10.0));
        let b = add(&mut h, square_fill(15.0, 0.0, 10.0));
        h.st.selection.set_all([a, b]);
        h.st.closest_join();
        let d = h.st.doc.design();
        let (ea, sb) = (d.object(a).unwrap().end.unwrap(), d.object(b).unwrap().start.unwrap());
        assert!((ea.x - 10.0).abs() < 1e-9 && (sb.x - 15.0).abs() < 1e-9);
        assert!((ea.distance(sb) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn select_tools_align_space_and_size() {
        let mut h = Harness::empty();
        let a = add(&mut h, square_fill(0.0, 0.0, 10.0));
        let b = add(&mut h, square_fill(20.0, 5.0, 10.0));
        let c = add(&mut h, square_fill(50.0, 2.0, 20.0));
        h.st.select_inside(&[p(-1.0, -1.0), p(31.0, -1.0), p(31.0, 16.0), p(-1.0, 16.0)]);
        assert_eq!(h.st.selection.len(), 2);
        h.st.select_crossing(&[p(55.0, -10.0), p(55.0, 40.0)]);
        assert_eq!(h.st.selection.ids(), &[c]);
        h.st.selection.set_all([a, b, c]);
        h.st.align(Align::Top);
        let d = h.st.doc.design();
        assert!([a, b, c].iter().all(|id| d.bbox_of(&[*id]).unwrap().min.y.abs() < 1e-9));
        h.st.space_evenly(true);
        let d = h.st.doc.design();
        let (ba, bb, bc) = (
            d.bbox_of(&[a]).unwrap(),
            d.bbox_of(&[b]).unwrap(),
            d.bbox_of(&[c]).unwrap(),
        );
        assert!(((bb.min.x - ba.max.x) - (bc.min.x - bb.max.x)).abs() < 1e-9);
        h.st.selection.set_all([c, a]);
        h.st.make_same_size(true, true);
        assert!((h.st.doc.design().bbox_of(&[a]).unwrap().width() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn group_lock_and_reference_points() {
        let mut h = Harness::empty();
        let a = add(&mut h, square_fill(0.0, 0.0, 10.0));
        let b = add(&mut h, square_fill(20.0, 0.0, 10.0));
        h.st.selection.set_all([a, b]);
        h.st.group_selection();
        h.st.selection.set(a);
        h.st.apply_groups_and_locks();
        assert_eq!(h.st.selection.len(), 2, "a group is selected together");
        // Reference points: (0,0)->(0,0) and (10,0)->(0,20): turn 90° and scale 2.
        h.st.transform_by_points(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 0.0), p(0.0, 20.0));
        let bb = h.st.doc.design().bbox_of(&[a]).unwrap();
        assert!(
            (bb.width() - 20.0).abs() < 1e-6 && (bb.height() - 20.0).abs() < 1e-6,
            "{bb:?}"
        );
        h.st.lock_selection();
        assert!(h.st.selection.is_empty());
        h.st.selection.set(a);
        h.st.apply_groups_and_locks();
        assert!(h.st.selection.is_empty(), "locked objects do not stay selected");
        h.st.unlock_all();
        assert_eq!(h.st.selection.len(), 2);
    }

    #[test]
    fn color_steps() {
        let mut h = Harness::empty();
        let (r, g) = h.st.doc.edit("Threads", |d| {
            (d.add_thread("R", Rgb(255, 0, 0)), d.add_thread("G", Rgb(0, 255, 0)))
        });
        let a = h.st.doc.edit("A", |d| d.add_object(r, square_fill(0.0, 0.0, 5.0)));
        let b = h.st.doc.edit("B", |d| d.add_object(g, square_fill(10.0, 0.0, 5.0)));
        let c = h.st.doc.edit("C", |d| d.add_object(r, square_fill(20.0, 0.0, 5.0)));
        h.st.selection.set(c);
        h.st.step_color(true);
        let order: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(order, [a, c, b]);
        h.st.selection.set(a);
        h.st.step_color(false);
        let order: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(order, [c, b, a]);
    }
}
