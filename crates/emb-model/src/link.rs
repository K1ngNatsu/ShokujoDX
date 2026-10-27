//! Linked shapes (ADR 0026): cuts that stay up to date.
//!
//! Flatten, Divide and Remove overlaps can keep a link: a record with each
//! cut object as it was before the cut and the pieces that the cut made of
//! it (and, for Remove overlaps, the objects that cut). When a linked
//! object moves (move, scale, turn, mirror), its stored original moves
//! with it, and the cut is made again from the originals. Border traces
//! (`trace`) work the same way.
//!
//! The pieces keep their object ids when they are made again (if there
//! are as many), so a selection and a drag on the canvas go on with them.
//! A change of the stitch settings or the thread of the first piece goes
//! back to its original. A change of the shape of a piece by hand does
//! not stay: unlink first.

use std::collections::BTreeMap;
use std::sync::Arc;

use emb_geom::Transform;
use emb_geom::region::{self, Region};
use serde::{Deserialize, Serialize};

use crate::shaping::{Part, ShapeError, area_of, clean, rebuild, shrink};
use crate::trace::TraceCut;
use crate::{Design, DesignObject, ObjectId, ObjectKind, ShapeOp};

/// The command that a link makes again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkOp {
    Flatten,
    Divide,
    RemoveOverlaps,
}

impl LinkOp {
    pub fn name(self) -> &'static str {
        match self {
            LinkOp::Flatten => "Flatten",
            LinkOp::Divide => "Divide",
            LinkOp::RemoveOverlaps => "Remove overlaps",
        }
    }
}

/// A linked shaping result in a design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeLink {
    pub id: u32,
    pub op: LinkOp,
    #[serde(default)]
    pub overlap_mm: f64,
    /// The objects that cut and stay as they are (Remove overlaps).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutters: Vec<ObjectId>,
    pub cuts: Vec<TraceCut>,
}

impl ShapeLink {
    /// Returns `true` if object `id` is part of the link.
    pub fn has(&self, id: ObjectId) -> bool {
        self.cutters.contains(&id) || self.cuts.iter().any(|c| c.pieces.contains(&id))
    }
}

impl Design {
    /// The shape link that object `id` is part of.
    pub fn link_of(&self, id: ObjectId) -> Option<&ShapeLink> {
        self.shape_links.iter().find(|l| l.has(id))
    }

    /// Returns `true` if object `id` is in a shape link, a border trace or
    /// a sketch link (an object made from a sketch).
    pub fn is_linked(&self, id: ObjectId) -> bool {
        self.link_of(id).is_some() || self.trace_of(id).is_some() || self.sketch_link_of(id).is_some()
    }

    /// Removes the link `id`: the pieces stay as plain objects.
    pub fn unlink(&mut self, id: u32) -> bool {
        let n = self.shape_links.len();
        self.shape_links.retain(|l| l.id != id);
        self.shape_links.len() != n
    }

    /// The ids of the originals that the records keep, so a new object
    /// never takes one of them (see `next_object_id`).
    pub(crate) fn kept_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        let traces = self.border_traces.iter().flat_map(|t| t.cuts.iter());
        let links = self.shape_links.iter().flat_map(|l| l.cuts.iter());
        traces.chain(links).map(|c| c.original.id)
    }

    /// Puts the cut objects back in the place of their first piece, with
    /// the thread and stitch settings of that piece, and removes the
    /// pieces. Returns the old pieces of each original, by its id.
    pub(crate) fn restore_cuts(&mut self, cuts: Vec<TraceCut>) -> BTreeMap<ObjectId, Vec<ObjectId>> {
        let mut old = BTreeMap::new();
        for cut in cuts.into_iter().rev() {
            let mut original = cut.original;
            let first = cut.pieces.iter().filter_map(|id| self.index_of(*id)).min();
            if let Some(piece) = cut.pieces.iter().find_map(|id| self.object(*id)) {
                let mut current = crate::CurrentProperties::default();
                if current.take_from(&piece.kind) {
                    current.apply_to(&mut original.kind);
                }
                original.thread = piece.thread;
                original.functions = piece.functions;
            }
            for id in &cut.pieces {
                self.remove_object(*id);
            }
            if self.object(original.id).is_none() {
                let at = first.unwrap_or(self.objects.len()).min(self.objects.len());
                original.revision = crate::object::fresh_revision();
                old.insert(original.id, cut.pieces);
                self.objects.insert(at, Arc::new(original));
            }
        }
        old
    }

    /// Replaces object `original` with objects of `kinds` in its place.
    /// The new objects take the ids of `reuse` while there are some (and
    /// they are free), its thread, group, functions and name. Returns the
    /// new ids in sew order.
    pub(crate) fn replace_with_kinds(
        &mut self,
        original: &DesignObject,
        kinds: Vec<ObjectKind>,
        reuse: &[ObjectId],
    ) -> Vec<ObjectId> {
        let at = self.index_of(original.id).unwrap_or(self.objects.len());
        self.remove_object(original.id);
        let free_ids: Vec<ObjectId> = reuse.iter().copied().filter(|id| self.object(*id).is_none()).collect();
        let mut free = free_ids.into_iter();
        let mut new = Vec::new();
        for (k, kind) in kinds.into_iter().enumerate() {
            let id = free.next().unwrap_or_else(|| self.next_object_id());
            let obj = DesignObject {
                id,
                revision: crate::object::fresh_revision(),
                kind,
                ..original.clone()
            };
            self.objects.insert((at + k).min(self.objects.len()), Arc::new(obj));
            new.push(id);
        }
        new
    }

    /// Makes a link of `op` on the objects `ids` (in sew order) and returns
    /// the new pieces. For Remove overlaps, `cutters` are the objects that
    /// cut and `ids` the objects under them.
    pub(crate) fn make_link(
        &mut self,
        op: LinkOp,
        ids: &[ObjectId],
        cutters: &[ObjectId],
        overlap_mm: f64,
    ) -> Result<Vec<ObjectId>, ShapeError> {
        if ids.iter().chain(cutters).any(|id| self.is_linked(*id)) {
            return Err(ShapeError::AlreadyLinked);
        }
        let originals: Vec<DesignObject> = ids.iter().filter_map(|id| self.object(*id).cloned()).collect();
        let link = ShapeLink {
            id: self.shape_links.iter().map(|l| l.id + 1).max().unwrap_or(1),
            op,
            overlap_mm,
            cutters: cutters.to_vec(),
            cuts: Vec::new(),
        };
        let mut link = link;
        let cuts = self.cut_link(&link, originals, &BTreeMap::new())?;
        let new: Vec<ObjectId> = cuts.iter().flat_map(|c| c.pieces.iter().copied()).collect();
        link.cuts = cuts;
        self.shape_links.push(link);
        Ok(new)
    }

    /// Cuts the originals (which are in the design) for `link`, and puts
    /// the pieces in their place.
    fn cut_link(
        &mut self,
        link: &ShapeLink,
        mut originals: Vec<DesignObject>,
        reuse: &BTreeMap<ObjectId, Vec<ObjectId>>,
    ) -> Result<Vec<TraceCut>, ShapeError> {
        originals.sort_by_key(|o| self.index_of(o.id));
        let parts: Vec<Part> = originals
            .iter()
            .filter_map(|o| {
                area_of(&o.kind).map(|area| Part {
                    id: o.id,
                    kind: o.kind.clone(),
                    area,
                })
            })
            .collect();
        let results: Vec<(usize, Vec<Region>)> = match link.op {
            LinkOp::Flatten => crate::shaping::op_results(&parts, ShapeOp::Flatten, link.overlap_mm),
            LinkOp::Divide => crate::shaping::op_results(&parts, ShapeOp::Divide, link.overlap_mm),
            LinkOp::RemoveOverlaps => {
                let cutters: Vec<(usize, Vec<Region>)> = link
                    .cutters
                    .iter()
                    .filter_map(|id| Some((self.index_of(*id)?, area_of(&self.object(*id)?.kind)?)))
                    .collect();
                parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let at = self.index_of(p.id).unwrap_or(0);
                        let cutter = cutters
                            .iter()
                            .filter(|(ci, _)| *ci > at)
                            .fold(Vec::new(), |acc, (_, a)| region::union(&acc, a));
                        (i, region::difference(&p.area, &shrink(&cutter, link.overlap_mm)))
                    })
                    .collect()
            }
        };
        let mut by_owner: Vec<Vec<Region>> = vec![Vec::new(); parts.len()];
        for (o, r) in results {
            by_owner[o].extend(clean(r));
        }
        let fill = self.current.fill.clone();
        let mut cuts = Vec::new();
        for (p, regions) in parts.iter().zip(by_owner) {
            let Some(original) = originals.iter().find(|o| o.id == p.id).cloned() else {
                continue;
            };
            // Divide makes an object of each piece; the other commands keep
            // each object one object.
            let kinds: Vec<ObjectKind> = match link.op {
                LinkOp::Divide => regions
                    .iter()
                    .flat_map(|r| rebuild(&p.kind, std::slice::from_ref(r), &fill))
                    .collect(),
                _ => rebuild(&p.kind, &regions, &fill),
            };
            let empty = Vec::new();
            let pieces = self.replace_with_kinds(&original, kinds, reuse.get(&original.id).unwrap_or(&empty));
            cuts.push(TraceCut { original, pieces });
        }
        if cuts.iter().all(|c| c.pieces.is_empty()) {
            return Err(ShapeError::NothingLeft);
        }
        Ok(cuts)
    }

    /// Makes the cut of link `id` again from its originals.
    pub fn refresh_link(&mut self, id: u32) {
        let Some(i) = self.shape_links.iter().position(|l| l.id == id) else {
            return;
        };
        let mut link = self.shape_links.remove(i);
        let reuse = self.restore_cuts(std::mem::take(&mut link.cuts));
        let originals: Vec<DesignObject> = reuse.keys().filter_map(|id| self.object(*id).cloned()).collect();
        if let Ok(cuts) = self.cut_link(&link, originals, &reuse) {
            link.cuts = cuts;
            self.shape_links.insert(i, link);
        }
    }

    /// Changes the kind of linked object `id` with `convert` and keeps its
    /// id (vector to fill, fill to vector and so on): the stored original
    /// that it was cut from changes the same way, and the link is made
    /// again, so it stays live. `thread` is its new thread. Fails (with no
    /// change) for the border of a trace, or if `convert` does not give one
    /// object for the object and for its original. An object made from a
    /// sketch keeps its link, which makes the new kind after a change of
    /// the sketch (`LinkSource::converted`).
    pub fn convert_linked(
        &mut self,
        id: ObjectId,
        thread: Option<crate::ThreadId>,
        convert: &dyn Fn(&ObjectKind) -> Option<ObjectKind>,
    ) -> Result<(), ShapeError> {
        if self.border_traces.iter().any(|t| t.border == Some(id)) {
            return Err(ShapeError::AlreadyLinked);
        }
        let kind = self
            .object(id)
            .and_then(|o| convert(&o.kind))
            .ok_or(ShapeError::AlreadyLinked)?;
        // An object made from a sketch: the link makes the new kind.
        let sketch_source = match self.sketch_link_of(id) {
            Some(l) => Some((l.id, l.source.converted(&kind).ok_or(ShapeError::AlreadyLinked)?)),
            None => None,
        };
        if let Some((link, source)) = sketch_source
            && let Some(l) = self.sketch_links.iter_mut().find(|l| l.id == link)
        {
            l.source = source;
        }
        // The originals of the cuts that this object is a piece of.
        let mut links = self.shape_links.clone();
        let mut traces = self.border_traces.clone();
        let cuts = links
            .iter_mut()
            .flat_map(|l| l.cuts.iter_mut())
            .chain(traces.iter_mut().flat_map(|t| t.cuts.iter_mut()))
            .filter(|c| c.pieces.contains(&id));
        for c in cuts {
            c.original.kind = convert(&c.original.kind).ok_or(ShapeError::AlreadyLinked)?;
            if let Some(t) = thread {
                c.original.thread = t;
            }
        }
        self.shape_links = links;
        self.border_traces = traces;
        if let Some(o) = self.object_mut(id) {
            o.kind = kind;
            if let Some(t) = thread {
                o.thread = t;
            }
        }
        // Make the cuts again with the new kind.
        let link_ids: Vec<u32> = self.shape_links.iter().filter(|l| l.has(id)).map(|l| l.id).collect();
        let trace_ids: Vec<(u32, crate::TraceSpec)> = self
            .border_traces
            .iter()
            .filter(|t| t.has(id))
            .map(|t| (t.id, t.spec))
            .collect();
        for l in link_ids {
            self.refresh_link(l);
        }
        for (t, spec) in trace_ids {
            let _ = self.update_border_trace(t, spec);
        }
        Ok(())
    }

    /// After objects `ids` moved by `delta`: the stored originals of moved
    /// pieces move too, and each link and border trace with a moved object
    /// is made again.
    pub fn linked_objects_moved(&mut self, ids: &[ObjectId], delta: &Transform) {
        if !self.sketch_links.is_empty() {
            self.sketch_objects_moved(ids, delta);
        }
        if self.shape_links.is_empty() && self.border_traces.is_empty() {
            return;
        }
        let moved = |cuts: &mut Vec<TraceCut>| {
            for c in cuts.iter_mut() {
                if c.pieces.iter().any(|p| ids.contains(p)) {
                    c.original.transform(delta);
                }
            }
        };
        let mut links = Vec::new();
        for l in &mut self.shape_links {
            if ids.iter().any(|id| l.has(*id)) {
                moved(&mut l.cuts);
                links.push(l.id);
            }
        }
        let mut traces = Vec::new();
        for t in &mut self.border_traces {
            if ids.iter().any(|id| t.has(*id)) {
                moved(&mut t.cuts);
                traces.push((t.id, t.spec));
            }
        }
        for id in links {
            self.refresh_link(id);
        }
        for (id, spec) in traces {
            let _ = self.update_border_trace(id, spec);
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;
    use crate::{FillParams, Rgb, ShapeOptions};

    fn square(x: f64, y: f64, s: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(Point::new(x, y), Point::new(x + s, y + s))),
            holes: Vec::new(),
            params: FillParams::default(),
        }
    }

    fn area(d: &Design, ids: &[ObjectId]) -> f64 {
        ids.iter()
            .filter_map(|id| d.object(*id))
            .filter_map(|o| area_of(&o.kind))
            .flatten()
            .map(|r| r.area())
            .sum()
    }

    fn linked() -> ShapeOptions {
        ShapeOptions {
            linked: true,
            ..ShapeOptions::default()
        }
    }

    #[test]
    fn flatten_link_cuts_again_when_the_front_moves() {
        let mut d = Design::new("l");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let back = d.add_object(t, square(0.0, 0.0, 20.0));
        let front = d.add_object(t, square(10.0, 10.0, 20.0));
        let new = d.shape(&[back, front], ShapeOp::Flatten, linked()).unwrap();
        assert_eq!(new.len(), 2);
        assert_eq!(d.shape_links.len(), 1);
        let (back_piece, front_piece) = (new[0], new[1]);
        assert!((area(&d, &[back_piece]) - 300.0).abs() < 0.01);
        // The front moves away: the back is whole again, with the same id.
        let t = Transform::translate(50.0, 0.0);
        d.transform_objects(&[front_piece], &t);
        assert!(
            (area(&d, &[back_piece]) - 400.0).abs() < 0.01,
            "{}",
            area(&d, &[back_piece])
        );
        // And back: cut again.
        d.transform_objects(&[front_piece], &t.inverse().unwrap());
        assert!((area(&d, &[back_piece]) - 300.0).abs() < 0.01);
        assert_eq!(d.objects().len(), 2);
        // Unlink: plain objects that stay as they are.
        let id = d.shape_links[0].id;
        assert!(d.unlink(id));
        d.transform_objects(&[front_piece], &t);
        assert!((area(&d, &[back_piece]) - 300.0).abs() < 0.01);
    }

    #[test]
    fn remove_overlaps_link_follows_the_cutter_and_the_piece() {
        let mut d = Design::new("r");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let under = d.add_object(t, square(0.0, 0.0, 20.0));
        let top = d.add_object(t, square(15.0, 15.0, 10.0));
        let new = d.remove_overlaps(&[top], 0.0, true).unwrap();
        assert!((area(&d, &new) - 375.0).abs() < 0.01);
        assert!(d.is_linked(top) && d.is_linked(new[0]));
        // Moving the piece moves its original: it is cut where it is now.
        d.transform_objects(&new, &Transform::translate(-10.0, -10.0));
        assert!((area(&d, &new) - 400.0).abs() < 0.01, "{}", area(&d, &new));
        assert!(d.object(under).is_none(), "the original is kept in the record only");
        // A copy of the design saves and loads the link.
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert!(back.same_content(&d));
        // A new object never takes the id of a kept original.
        let n = d.add_object(t, square(100.0, 100.0, 5.0));
        assert_ne!(n, under);
    }

    #[test]
    fn a_conversion_keeps_the_link() {
        use crate::vector::{from_vector, to_vector};
        use crate::{ConvertTo, CurrentProperties, VectorStyle};
        let mut d = Design::new("c");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let vector = |x: f64| ObjectKind::Vector {
            path: Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + 20.0, 20.0))),
            style: VectorStyle::filled(Rgb(200, 0, 0)),
        };
        let back = d.add_object(t, vector(0.0));
        let front = d.add_object(t, vector(10.0));
        let new = d.shape(&[back, front], ShapeOp::Flatten, linked()).unwrap();
        let to_fill = |k: &ObjectKind| match k {
            ObjectKind::Vector { path, .. } => {
                let mut v = from_vector(path, ConvertTo::Fill, &CurrentProperties::default(), 3.0);
                (v.len() == 1).then(|| v.remove(0))
            }
            _ => None,
        };
        for id in &new {
            d.convert_linked(*id, None, &to_fill).unwrap();
        }
        assert!(
            new.iter()
                .all(|id| matches!(d.object(*id).unwrap().kind, ObjectKind::Fill { .. }))
        );
        assert_eq!(d.shape_links.len(), 1, "still linked");
        // A move still cuts again, as fills.
        d.transform_objects(&[new[1]], &Transform::translate(50.0, 0.0));
        assert!((area(&d, &[new[0]]) - 400.0).abs() < 0.01);
        assert!(matches!(d.object(new[0]).unwrap().kind, ObjectKind::Fill { .. }));
        // And back to vectors.
        let to_vec = |k: &ObjectKind| to_vector(k, Rgb(200, 0, 0));
        d.convert_linked(new[0], None, &to_vec).unwrap();
        assert!(matches!(d.object(new[0]).unwrap().kind, ObjectKind::Vector { .. }));
        // A conversion that is not one object for one fails, with no change.
        let before = d.clone();
        assert_eq!(
            d.convert_linked(new[1], None, &|_| None),
            Err(ShapeError::AlreadyLinked)
        );
        assert!(d.same_content(&before));
    }

    #[test]
    fn linked_objects_cannot_be_linked_again() {
        let mut d = Design::new("x");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let a = d.add_object(t, square(0.0, 0.0, 20.0));
        let b = d.add_object(t, square(10.0, 10.0, 20.0));
        let new = d.shape(&[a, b], ShapeOp::Divide, linked()).unwrap();
        assert_eq!(new.len(), 3);
        assert_eq!(
            d.shape(&new, ShapeOp::Flatten, linked()),
            Err(ShapeError::AlreadyLinked)
        );
    }
}
