//! Appliqué from a selection (ADR 0087).
//!
//! One command makes one appliqué object from the selected objects:
//!
//! - The fabric is the area of the selected fills, closed vectors and
//!   appliqué objects, all in one. A closed border round them (a satin
//!   ring, or a closed line with a border) adds the area inside its middle
//!   line, so the cut edge of the fabric is under the border. With no
//!   areas in the selection, the area inside each closed border is the
//!   fabric.
//! - A selected border that covers an edge of the fabric (the outline and
//!   the tack-down line) is the cover of that edge: the edge gets no cover
//!   of its own, and the border sews after the appliqué. If the borders
//!   cover all edges, the appliqué has no cover of its own.
//! - The objects that the fabric replaces go into a record
//!   (`Design::applique_origins`) with the border traces that they were
//!   part of. Back to Original puts them back and removes the appliqué.

use std::sync::Arc;

use emb_geom::region::{self, Join, Region};
use emb_geom::{Path, Point, text};
use serde::{Deserialize, Serialize};

use crate::applique::{AppliqueParams, AppliquePiece, TackDown, outline_edges};
use crate::shaping::{area_of, even_odd};
use crate::trace::BorderTrace;
use crate::{Design, DesignObject, ObjectId, ObjectKind, Rgb};

/// Tolerance to flatten the outlines, in mm.
const FLATTEN_TOL_MM: f64 = 0.05;
/// The points of an edge that are checked are this far apart, in mm.
const SAMPLE_MM: f64 = 0.5;
/// A point is under a border if it is in the border or nearer to it than
/// this, in mm.
const UNDER_MM: f64 = 0.2;
/// An edge is covered if this part of its points is under a border.
const COVERED: f64 = 0.9;
/// A closed border adds the area inside it if the area of the selection
/// covers this part of it (or of the area of the selection, if smaller).
const OVERLAP: f64 = 0.5;
/// Pieces smaller than this are left out, in mm².
const MIN_AREA_MM2: f64 = 0.05;

/// The objects that an appliqué made from a selection replaced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliqueOrigin {
    /// The appliqué object.
    pub applique: ObjectId,
    /// The replaced objects, in sew order.
    pub originals: Vec<DesignObject>,
    /// The borders that sewed before the originals. They go back before
    /// them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<ObjectId>,
    /// The border traces that the replaced objects or the borders were
    /// part of. They end with the command and come back with the originals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<BorderTrace>,
}

impl PartialEq for AppliqueOrigin {
    fn eq(&self, o: &AppliqueOrigin) -> bool {
        self.applique == o.applique
            && self.originals.len() == o.originals.len()
            && self.originals.iter().zip(&o.originals).all(|(a, b)| a.same_content(b))
            && self.before == o.before
            && self.traces == o.traces
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppliqueError {
    #[error(
        "{}",
        text::tr("Select fills, closed shapes or appliqué, or a closed satin border, to make appliqué.")
    )]
    NoFabric,
    #[error(
        "{}",
        text::tr("A selected shape is a linked shape or comes from a sketch. Remove the link first.")
    )]
    Linked,
}

/// A selected border.
struct Border {
    id: ObjectId,
    index: usize,
    /// The area of the border, made larger by [`UNDER_MM`].
    under: Vec<Region>,
    /// The area inside the middle line of a closed border (else empty).
    inside: Vec<Region>,
}

/// The area of a border kind, and the area inside its middle line if it is
/// closed. `None` if the kind is not a border.
fn border_areas(kind: &ObjectKind) -> Option<(Vec<Region>, Vec<Region>)> {
    match kind {
        ObjectKind::Satin { .. } => {
            let area = area_of(kind)?;
            let inside = ring_inside(&area);
            Some((area, inside))
        }
        ObjectKind::Running { path, params } => {
            let b = params.border.as_ref()?;
            let lines = path.flatten(FLATTEN_TOL_MM);
            let area = region::offset_lines(&lines, b.width_mm / 2.0, Join::Round);
            let closed = !lines.is_empty()
                && lines
                    .iter()
                    .all(|l| l.closed || (l.points.len() > 2 && l.points.first() == l.points.last()));
            let inside = if closed { even_odd(path) } else { Vec::new() };
            (!area.is_empty()).then_some((area, inside))
        }
        _ => None,
    }
}

fn perimeter(ring: &[Point]) -> f64 {
    (0..ring.len())
        .map(|i| ring[i].distance(ring[(i + 1) % ring.len()]))
        .sum()
}

/// The area inside the middle line of a ring column: the hole of the ring,
/// made larger by half the mean width of the ring.
fn ring_inside(area: &[Region]) -> Vec<Region> {
    area.iter()
        .filter(|r| r.holes.len() == 1)
        .map(|r| {
            let hole = Region::new(r.holes[0].clone(), Vec::new());
            let w = 2.0 * r.area() / (perimeter(&r.outer) + perimeter(&r.holes[0])).max(1e-9);
            region::offset(&[hole], w / 2.0, Join::Round)
        })
        .fold(Vec::new(), |acc, r| region::union(&acc, &r))
}

fn total_area(rs: &[Region]) -> f64 {
    rs.iter().map(Region::area).sum()
}

fn in_any(rs: &[Region], p: Point) -> bool {
    rs.iter().any(|r| r.contains(p))
}

/// The borders under the edge `pts` of `fabric` if the edge is covered: if
/// [`COVERED`] of its points, and the points `inset` into the fabric (the
/// tack-down line), are under a border.
fn edge_cover(pts: &[Point], fabric: &Region, inset: f64, borders: &[Border]) -> Option<Vec<usize>> {
    let mut hits = vec![false; borders.len()];
    let (mut n, mut good) = (0usize, 0usize);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let Some(dir) = (b - a).normalized() else { continue };
        let mut nrm = dir.perp();
        if !fabric.contains(a.midpoint(b) + nrm * 1e-3) {
            nrm = -nrm;
        }
        let k = (a.distance(b) / SAMPLE_MM).ceil().max(1.0) as usize;
        for j in 0..k {
            let p = a.lerp(b, (j as f64 + 0.5) / k as f64);
            let q = p + nrm * inset;
            n += 1;
            let at_p: Vec<usize> = (0..borders.len()).filter(|&i| borders[i].under_has(p)).collect();
            if !at_p.is_empty() && borders.iter().any(|b| b.under_has(q)) {
                good += 1;
                for i in at_p {
                    hits[i] = true;
                }
            }
        }
    }
    (n > 0 && good as f64 >= COVERED * n as f64).then(|| (0..hits.len()).filter(|&i| hits[i]).collect())
}

impl Border {
    fn under_has(&self, p: Point) -> bool {
        in_any(&self.under, p)
    }
}

/// How far the tack-down goes into the fabric from the outline.
fn tack_depth(p: &AppliqueParams) -> f64 {
    match p.tack_down {
        TackDown::Run => p.tack_inset_mm,
        TackDown::Zigzag => p.tack_inset_mm + p.tack_width_mm / 2.0,
        TackDown::None => 0.0,
    }
}

impl Design {
    /// Makes one appliqué object from the objects `ids` with the settings
    /// `params` (see the module notes) and returns its id. The objects that
    /// are not areas or borders stay as they are.
    pub fn applique_from(&mut self, ids: &[ObjectId], params: &AppliqueParams) -> Result<ObjectId, AppliqueError> {
        let mut sel: Vec<(usize, ObjectId)> = ids.iter().filter_map(|id| Some((self.index_of(*id)?, *id))).collect();
        sel.sort_unstable();
        sel.dedup();
        let mut sources: Vec<(usize, ObjectId, Vec<Region>)> = Vec::new();
        let mut borders: Vec<Border> = Vec::new();
        for &(index, id) in &sel {
            let Some(o) = self.object(id) else { continue };
            match &o.kind {
                ObjectKind::Fill { .. } | ObjectKind::Vector { .. } | ObjectKind::Applique { .. } => {
                    let Some(area) = area_of(&o.kind) else { continue };
                    if self.link_of(id).is_some() || self.sketch_link_of(id).is_some() {
                        return Err(AppliqueError::Linked);
                    }
                    sources.push((index, id, area));
                }
                kind => {
                    if let Some((area, inside)) = border_areas(kind) {
                        borders.push(Border {
                            id,
                            index,
                            under: region::offset(&area, UNDER_MM, Join::Round),
                            inside,
                        });
                    }
                }
            }
        }

        // The fabric.
        let area = sources.iter().fold(Vec::new(), |acc, s| region::union(&acc, &s.2));
        let mut fabric = area.clone();
        for b in borders.iter().filter(|b| !b.inside.is_empty()) {
            let common = total_area(&region::intersection(&area, &b.inside));
            let smaller = if area.is_empty() {
                0.0
            } else {
                total_area(&area).min(total_area(&b.inside))
            };
            if area.is_empty() || common >= OVERLAP * smaller {
                fabric = region::union(&fabric, &b.inside);
            }
        }
        let fabric: Vec<Region> = fabric.into_iter().filter(|r| r.area() >= MIN_AREA_MM2).collect();
        if fabric.is_empty() {
            return Err(AppliqueError::NoFabric);
        }
        // A border inside the fabric is no border (borders only with no areas).
        if sources.is_empty() {
            borders.retain(|b| !b.inside.is_empty());
        }

        // The edges that the borders cover.
        let depth = tack_depth(params);
        let mut used = vec![false; borders.len()];
        let mut all_covered = true;
        let mut pieces = Vec::new();
        for r in &fabric {
            let outer = Path::polygon(&r.outer);
            let holes: Vec<Path> = r.holes.iter().map(|h| Path::polygon(h)).collect();
            let mut piece = AppliquePiece::new(outer.clone(), holes.clone());
            for (e, pts) in outline_edges(&outer, FLATTEN_TOL_MM).iter().enumerate() {
                match edge_cover(pts, r, depth, &borders) {
                    Some(bs) => {
                        piece.open_edges.push(e);
                        for i in bs {
                            used[i] = true;
                        }
                    }
                    None => all_covered = false,
                }
            }
            for h in &holes {
                for pts in outline_edges(h, FLATTEN_TOL_MM) {
                    match edge_cover(&pts, r, depth, &borders) {
                        Some(bs) => {
                            for i in bs {
                                used[i] = true;
                            }
                        }
                        None => all_covered = false,
                    }
                }
            }
            pieces.push(piece);
        }
        let used: Vec<&Border> = borders.iter().zip(&used).filter(|(_, u)| **u).map(|(b, _)| b).collect();
        let mut p = params.clone();
        if all_covered && !used.is_empty() {
            p.cover = false;
            for pc in &mut pieces {
                pc.open_edges.clear();
            }
        }
        if p.fabric.is_none() {
            p.fabric = sources.iter().find_map(|s| self.fabric_color(s.1));
        }

        // The thread of the placement and the tack-down: that of the
        // border, so that the border hides them.
        let thread = used
            .first()
            .map(|b| b.id)
            .or(sources.first().map(|s| s.1))
            .and_then(|id| self.object(id))
            .map(|o| o.thread)
            .ok_or(AppliqueError::NoFabric)?;
        let first = sources
            .iter()
            .map(|s| s.0)
            .chain(used.iter().map(|b| b.index))
            .min()
            .ok_or(AppliqueError::NoFabric)?;
        let first_source = sources.iter().map(|s| s.0).min();
        let before: Vec<ObjectId> = used
            .iter()
            .filter(|b| first_source.is_some_and(|f| b.index < f))
            .map(|b| b.id)
            .collect();
        let moved: Vec<ObjectId> = used.iter().map(|b| b.id).collect();
        let source_ids: Vec<ObjectId> = sources.iter().map(|s| s.1).collect();

        // The border traces end (the record keeps them).
        let (traces, kept): (Vec<BorderTrace>, Vec<BorderTrace>) = std::mem::take(&mut self.border_traces)
            .into_iter()
            .partition(|t| source_ids.iter().chain(&moved).any(|id| t.has(*id)));
        self.border_traces = kept;

        // The appliqué goes first, so that it does not take an id of an original.
        let id = self.insert_object(first, thread, ObjectKind::Applique { pieces, params: p });
        let originals: Vec<DesignObject> = source_ids.iter().filter_map(|s| self.object(*s).cloned()).collect();
        for s in &source_ids {
            self.remove_object(*s);
        }
        // The borders sew after the appliqué, in their order.
        let mut placed = 0;
        for b in &moved {
            let (Some(i), Some(at)) = (self.index_of(*b), self.index_of(id)) else {
                continue;
            };
            if i < at {
                self.move_object(*b, at + placed);
                placed += 1;
            }
        }
        self.applique_origins.push(AppliqueOrigin {
            applique: id,
            originals,
            before,
            traces,
        });
        Ok(id)
    }

    /// The fabric color for an appliqué that replaces object `id`: the
    /// color of a fill, of a vector, or of appliqué fabric.
    fn fabric_color(&self, id: ObjectId) -> Option<Rgb> {
        let o = self.object(id)?;
        match &o.kind {
            ObjectKind::Fill { .. } => self.thread(o.thread).map(|t| t.color),
            ObjectKind::Vector { style, .. } => style.fill,
            ObjectKind::Applique { pieces, params } => pieces.iter().find_map(|pc| pc.fabric).or(params.fabric),
            _ => None,
        }
    }

    /// The record of appliqué object `id`, if it was made from a selection.
    pub fn applique_origin(&self, id: ObjectId) -> Option<&AppliqueOrigin> {
        self.applique_origins.iter().find(|o| o.applique == id)
    }

    /// Puts back the objects that appliqué object `id` replaced, in its
    /// place, with their border traces, and removes the appliqué. The
    /// borders that sewed before them go back before them. Returns `false`
    /// if `id` was not made from a selection.
    pub fn back_to_original(&mut self, id: ObjectId) -> bool {
        let (Some(k), Some(at)) = (
            self.applique_origins.iter().position(|o| o.applique == id),
            self.index_of(id),
        ) else {
            return false;
        };
        let origin = self.applique_origins.remove(k);
        self.remove_object(id);
        let n = origin.originals.len();
        for (j, mut o) in origin.originals.into_iter().enumerate() {
            if self.object(o.id).is_some() {
                o.id = self.next_object_id();
            }
            o.revision = crate::object::fresh_revision();
            self.objects.insert((at + j).min(self.objects.len()), Arc::new(o));
        }
        let mut placed = 0;
        for b in &origin.before {
            if self.index_of(*b).is_some_and(|i| i >= at + n) {
                self.move_object(*b, at + placed);
                placed += 1;
            }
        }
        for t in origin.traces {
            let present = t.objects.iter().chain(&t.border).all(|o| self.object(*o).is_some());
            if present && !self.border_traces.iter().any(|x| x.id == t.id) {
                self.border_traces.push(t);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;
    use crate::{FillParams, LineBorder, RunningParams, SatinGeometry};

    fn square(c: f64, half: f64) -> Path {
        Path::rect(BBox::new(
            Point::new(c - half, c - half),
            Point::new(c + half, c + half),
        ))
    }

    fn fill(d: &mut Design, outer: Path) -> ObjectId {
        let t = d.add_thread("Red", Rgb(200, 30, 30));
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params: FillParams::default(),
            },
        )
    }

    /// A closed line with a satin border of `width` on `path`.
    fn border(d: &mut Design, path: Path, width: f64) -> ObjectId {
        let t = d.add_thread("Navy", Rgb(20, 30, 90));
        d.add_object(
            t,
            ObjectKind::Running {
                path,
                params: RunningParams {
                    border: Some(LineBorder {
                        width_mm: width,
                        column: Default::default(),
                        corner: Default::default(),
                    }),
                    ..RunningParams::default()
                },
            },
        )
    }

    fn applique(d: &Design, id: ObjectId) -> (Vec<AppliquePiece>, AppliqueParams) {
        match &d.object(id).unwrap().kind {
            ObjectKind::Applique { pieces, params } => (pieces.clone(), params.clone()),
            k => panic!("not appliqué: {}", k.type_name()),
        }
    }

    #[test]
    fn overlapping_fills_become_one_piece_in_their_place() {
        let mut d = Design::new("");
        let before = fill(&mut d, square(-30.0, 2.0));
        let a = fill(&mut d, square(0.0, 10.0));
        let b = fill(&mut d, square(8.0, 10.0));
        let id = d.applique_from(&[a, b], &AppliqueParams::default()).unwrap();
        let (pieces, params) = applique(&d, id);
        assert_eq!(pieces.len(), 1, "one piece of the welded area");
        assert!(params.cover, "no border: the appliqué has its own cover");
        assert_eq!(
            params.fabric,
            Some(Rgb(200, 30, 30)),
            "the fabric shows the color of the fill"
        );
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![before, id]);
        assert!(d.object(a).is_none() && d.object(b).is_none());
    }

    #[test]
    fn a_border_round_the_fill_is_the_cover_and_sews_after() {
        let mut d = Design::new("");
        let line = border(&mut d, square(0.0, 10.0), 3.0);
        // The fill stops at the inner edge of the border.
        let f = fill(&mut d, square(0.0, 8.5));
        let id = d.applique_from(&[f, line], &AppliqueParams::default()).unwrap();
        let (pieces, params) = applique(&d, id);
        assert!(!params.cover, "the border covers all edges");
        assert_eq!(pieces.len(), 1);
        // The fabric goes to the middle of the border, so the border hides the cut edge.
        let bb = pieces[0].outer.control_bbox().unwrap();
        assert!((bb.width() - 20.0).abs() < 0.2, "fabric width {}", bb.width());
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![id, line], "the border sews after the appliqué");
        assert_eq!(d.object(id).unwrap().thread, d.object(line).unwrap().thread);

        assert!(d.back_to_original(id));
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![line, f], "the order of before comes back");
        assert!(d.applique_origins.is_empty());
        assert!(!d.back_to_original(f));
    }

    #[test]
    fn a_traced_border_is_the_cover_and_its_trace_comes_back() {
        use crate::{BorderPlace, TraceOrder, TraceSpec, TraceStitch};
        let mut d = Design::new("");
        let f = fill(&mut d, square(0.0, 10.0));
        let black = d.add_thread("Black", Rgb(0, 0, 0));
        let spec = TraceSpec {
            border_mm: 4.0,
            gap_mm: 0.0,
            stitch: TraceStitch::default(),
            thread: black,
            order: TraceOrder::FillThenBorder,
            place: BorderPlace::Outside,
        };
        let t = d.add_border_trace(&[f], spec).unwrap();
        let line = d.border_traces[0].border.unwrap();
        let id = d.applique_from(&[f, line], &AppliqueParams::default()).unwrap();
        let (pieces, params) = applique(&d, id);
        assert!(!params.cover, "the traced border covers all edges");
        assert_eq!(pieces.len(), 1);
        assert!(d.border_traces.is_empty(), "the trace ends");
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![id, line]);
        assert!(d.back_to_original(id));
        assert_eq!(d.border_traces.len(), 1, "the trace comes back");
        assert_eq!(d.border_traces[0].id, t);
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![f, line]);
    }

    #[test]
    fn a_border_on_some_edges_leaves_the_others_their_cover() {
        let mut d = Design::new("");
        let f = fill(&mut d, square(0.0, 10.0));
        // A straight border on the left edge only.
        let left = Path::new()
            .move_to(Point::new(-10.0, -10.0))
            .line_to(Point::new(-10.0, 10.0));
        let line = border(&mut d, left, 3.0);
        let id = d.applique_from(&[f, line], &AppliqueParams::default()).unwrap();
        let (pieces, params) = applique(&d, id);
        assert!(params.cover);
        assert_eq!(pieces[0].open_edges.len(), 1, "only the left edge is open");
        let e = outline_edges(&pieces[0].outer, 0.05)[pieces[0].open_edges[0]].clone();
        assert!(
            e.iter().all(|p| (p.x + 10.0).abs() < 1e-6),
            "the open edge is the left one"
        );
    }

    #[test]
    fn a_closed_border_alone_becomes_appliqué_inside_it() {
        let mut d = Design::new("");
        let ring = SatinGeometry::from_centerline(&square(0.0, 10.0), 3.0);
        let t = d.add_thread("Gold", Rgb(220, 180, 40));
        let s = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: ring,
                params: Default::default(),
            },
        );
        let id = d.applique_from(&[s], &AppliqueParams::default()).unwrap();
        let (pieces, params) = applique(&d, id);
        assert!(!params.cover);
        let bb = pieces[0].outer.control_bbox().unwrap();
        assert!(
            (bb.width() - 20.0).abs() < 0.5,
            "fabric to the middle of the ring: {}",
            bb.width()
        );
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(ids, vec![id, s]);
        assert!(d.applique_origin(id).is_some_and(|o| o.originals.is_empty()));
    }

    #[test]
    fn nothing_to_make_and_moves_of_the_appliqué() {
        let mut d = Design::new("");
        let line = border(
            &mut d,
            Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(9.0, 0.0)),
            2.0,
        );
        assert_eq!(
            d.applique_from(&[line], &AppliqueParams::default()),
            Err(AppliqueError::NoFabric)
        );
        let f = fill(&mut d, square(0.0, 5.0));
        let id = d.applique_from(&[f], &AppliqueParams::default()).unwrap();
        // The original moves with the appliqué.
        d.transform_objects(&[id], &emb_geom::Transform::translate(5.0, 0.0));
        assert!(d.back_to_original(id));
        let bb = d.object(f).unwrap().control_bbox().unwrap();
        assert!((bb.min.x - 0.0).abs() < 1e-9, "moved with the appliqué: {:?}", bb);
    }
}
