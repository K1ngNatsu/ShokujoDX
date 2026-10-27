//! Links from a sketch to the objects made from it (P14 Milestone 5,
//! ADR 0076).
//!
//! Each make from a sketch (a region to a fill, a curve to a run, two
//! curves to a satin column) keeps a record: the sketch object, what the
//! objects came from (the key of a region, or the ids of curves), how they
//! were made, and the ids of the objects. After a change of the sketch,
//! `Design::refresh_sketch_links` makes the objects again in their places:
//! they keep their ids, sew order, thread, functions and stitch settings.
//! A region or a curve that is gone makes no change: the objects stay as
//! they are and the record is lost (a warning; Unlink ends it). When the
//! region comes back, the record makes the objects again.
//!
//! A move of the sketch moves its objects. A move of the objects without
//! the sketch is kept in the record (`offset`), so they stay where the
//! user put them after a change of the sketch.

use emb_geom::Transform;
use emb_sketch::{CurveId, RegionKey};
use serde::{Deserialize, Serialize};

use crate::sketch::{CurveTo, RegionTo, SketchObject, curve_objects, region_objects};
use crate::{CurrentProperties, Design, ObjectId, ObjectKind, Rgb, ThreadId};

/// What the objects of a link came from, and how they were made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case")]
pub enum LinkSource {
    /// A closed region. `width_mm` is the width of a satin column (Satin),
    /// `color` the color of a vector.
    Region {
        key: RegionKey,
        to: RegionTo,
        #[serde(default)]
        width_mm: f64,
        color: Rgb,
    },
    /// One curve, to a run or a satin line.
    Curve { curve: CurveId, to: CurveTo },
    /// A satin column with two curves as its sides.
    SatinBetween { a: CurveId, b: CurveId },
}

impl LinkSource {
    /// The objects of the source in sketch `s` (design coordinates), and
    /// the source with its region key brought up to date. `None` if the
    /// region or a curve is not in the sketch.
    pub fn make(&self, s: &SketchObject, current: &CurrentProperties) -> Option<(Vec<ObjectKind>, LinkSource)> {
        match self {
            LinkSource::Region {
                key,
                to,
                width_mm,
                color,
            } => {
                let r = s.find_region(key)?;
                let kinds = region_objects(&r, *to, current, *width_mm, *color);
                let new = LinkSource::Region {
                    key: s.region_key(&r),
                    to: *to,
                    width_mm: *width_mm,
                    color: *color,
                };
                Some((kinds, new))
            }
            LinkSource::Curve { curve, to } => {
                let path = s.curve_paths().iter().find(|(id, _)| id == curve)?.1.clone();
                Some((curve_objects(&[path], *to, current), self.clone()))
            }
            LinkSource::SatinBetween { a, b } => Some((vec![s.satin_between(*a, *b, current)?], self.clone())),
        }
    }

    /// The source after a conversion of its objects to `kind` (To fill, To
    /// satin and so on), so the next change of the sketch makes that kind.
    /// `None` if the source cannot make it.
    pub fn converted(&self, kind: &ObjectKind) -> Option<LinkSource> {
        let mut s = self.clone();
        match (&mut s, kind) {
            (LinkSource::Region { to, .. }, k) => {
                *to = match k {
                    ObjectKind::Fill { .. } => RegionTo::Fill,
                    ObjectKind::Satin { .. } => RegionTo::Satin,
                    ObjectKind::Applique { .. } => RegionTo::Applique,
                    ObjectKind::Vector { .. } => RegionTo::Vector,
                    _ => return None,
                }
            }
            (LinkSource::Curve { to, .. }, ObjectKind::Running { .. }) => *to = CurveTo::Run,
            (LinkSource::Curve { to, .. }, ObjectKind::Satin { .. }) => {
                if !matches!(to, CurveTo::SatinLine(_)) {
                    *to = CurveTo::SatinLine(3.0);
                }
            }
            (LinkSource::SatinBetween { .. }, ObjectKind::Satin { .. }) => {}
            _ => return None,
        }
        Some(s)
    }
}

/// The record of the objects made from one source of a sketch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SketchLink {
    pub id: u32,
    /// The sketch object.
    pub sketch: ObjectId,
    pub source: LinkSource,
    /// The objects, in sew order when they were made.
    pub objects: Vec<ObjectId>,
    /// A move of the objects without their sketch (design coordinates,
    /// after the sketch).
    #[serde(default = "identity", skip_serializing_if = "is_identity")]
    pub offset: Transform,
    /// The source is not in the sketch now; the objects stay as they were.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lost: bool,
}

fn identity() -> Transform {
    Transform::IDENTITY
}

fn is_identity(t: &Transform) -> bool {
    *t == Transform::IDENTITY
}

impl SketchLink {
    pub fn has(&self, id: ObjectId) -> bool {
        self.objects.contains(&id)
    }
}

impl Design {
    /// The sketch link of object `id` (an object made from a sketch).
    pub fn sketch_link_of(&self, id: ObjectId) -> Option<&SketchLink> {
        self.sketch_links.iter().find(|l| l.has(id))
    }

    /// The sketch object of `id`: the sketch itself, or the sketch that
    /// object `id` was made from.
    pub fn sketch_for(&self, id: ObjectId) -> Option<ObjectId> {
        match self.object(id).map(|o| &o.kind) {
            Some(ObjectKind::Sketch(_)) => Some(id),
            _ => self.sketch_link_of(id).map(|l| l.sketch),
        }
    }

    /// Makes the objects of `source` from sketch `sketch` with the
    /// settings `current` and thread `thread`, at the end of the sew order,
    /// linked to the sketch. Returns the new objects.
    pub fn make_from_sketch(
        &mut self,
        sketch: ObjectId,
        source: LinkSource,
        current: &CurrentProperties,
        thread: ThreadId,
    ) -> Vec<ObjectId> {
        let Some(ObjectKind::Sketch(s)) = self.object(sketch).map(|o| &o.kind) else {
            return Vec::new();
        };
        let Some((kinds, source)) = source.make(s, current) else {
            return Vec::new();
        };
        let objects: Vec<ObjectId> = kinds.into_iter().map(|k| self.add_object(thread, k)).collect();
        if !objects.is_empty() {
            let id = self.sketch_links.iter().map(|l| l.id + 1).max().unwrap_or(1);
            self.sketch_links.push(SketchLink {
                id,
                sketch,
                source,
                objects: objects.clone(),
                offset: Transform::IDENTITY,
                lost: false,
            });
        }
        objects
    }

    /// Gives parameter `name` of sketch `sketch` the expression `text`
    /// (ADR 0078), solves the sketch and makes its linked objects again.
    /// Returns the number of objects whose source is gone now. On an error
    /// the design does not change.
    pub fn set_sketch_parameter(
        &mut self,
        sketch: ObjectId,
        name: &str,
        text: &str,
    ) -> Result<usize, emb_sketch::ExprError> {
        let Some(ObjectKind::Sketch(s)) = self.object(sketch).map(|o| &o.kind) else {
            return Err(emb_sketch::ExprError::Unknown(name.to_string()));
        };
        let mut s = s.clone();
        s.edit(|sk| {
            sk.set_parameter(name, text)?;
            sk.solve();
            Ok::<_, emb_sketch::ExprError>(())
        })?;
        if let Some(o) = self.object_mut(sketch) {
            o.kind = ObjectKind::Sketch(s);
        }
        Ok(self.refresh_sketch_links(sketch))
    }

    /// Removes sketch link `id`: its objects stay as they are.
    pub fn unlink_sketch(&mut self, id: u32) -> bool {
        let n = self.sketch_links.len();
        self.sketch_links.retain(|l| l.id != id);
        self.sketch_links.len() != n
    }

    /// After a change of sketch `sketch`: makes the objects of its links
    /// again. Records of objects that are all deleted, or of a sketch that
    /// is deleted, end. Returns the number of objects whose source is gone
    /// now (they were not lost before; they stay as they are).
    pub fn refresh_sketch_links(&mut self, sketch: ObjectId) -> usize {
        let mut links = std::mem::take(&mut self.sketch_links);
        let Some(ObjectKind::Sketch(s)) = self.object(sketch).map(|o| o.kind.clone()) else {
            links.retain(|l| l.sketch != sketch);
            self.sketch_links = links;
            return 0;
        };
        let mut lost = 0;
        links.retain_mut(|l| {
            l.objects.retain(|id| self.object(*id).is_some());
            if l.objects.is_empty() {
                return false;
            }
            if l.sketch != sketch {
                return true;
            }
            let old: Vec<crate::DesignObject> = l.objects.iter().filter_map(|id| self.object(*id).cloned()).collect();
            // The settings of the objects as they are now, for each type.
            let mut current = self.current.clone();
            for o in &old {
                current.take_from(&o.kind);
            }
            match l.source.make(&s, &current) {
                Some((mut kinds, source)) if !kinds.is_empty() => {
                    for (i, k) in kinds.iter_mut().enumerate() {
                        if l.offset != Transform::IDENTITY {
                            k.transform(&l.offset);
                        }
                        keep_settings(k, &old[i.min(old.len() - 1)].kind);
                    }
                    l.objects = self.replace_linked(&old, kinds);
                    l.source = source;
                    l.lost = false;
                }
                _ => {
                    if !l.lost {
                        lost += l.objects.len();
                    }
                    l.lost = true;
                }
            }
            true
        });
        self.sketch_links = links;
        lost
    }

    /// Puts `kinds` in the place of objects `old`: in place one for one
    /// when there are as many, else all in the place of the first one. The
    /// new objects take the ids of the old ones while there are some.
    fn replace_linked(&mut self, old: &[crate::DesignObject], kinds: Vec<ObjectKind>) -> Vec<ObjectId> {
        if old.len() == kinds.len() {
            for (o, k) in old.iter().zip(kinds) {
                if let Some(m) = self.object_mut(o.id) {
                    m.kind = k;
                }
            }
            return old.iter().map(|o| o.id).collect();
        }
        let mut old: Vec<&crate::DesignObject> = old.iter().collect();
        old.sort_by_key(|o| self.index_of(o.id));
        for o in &old[1..] {
            self.remove_object(o.id);
        }
        let reuse: Vec<ObjectId> = old.iter().map(|o| o.id).collect();
        self.replace_with_kinds(old[0], kinds, &reuse)
    }

    /// After objects `ids` moved by `delta`: the objects of a moved sketch
    /// move with it; a move of linked objects without their sketch goes
    /// into the offset of their link (and the other objects of the link
    /// move with them).
    pub(crate) fn sketch_objects_moved(&mut self, ids: &[ObjectId], delta: &Transform) {
        let mut follow: Vec<ObjectId> = Vec::new();
        for l in &mut self.sketch_links {
            let others = l.objects.iter().filter(|id| !ids.contains(id));
            if ids.contains(&l.sketch) {
                // The offset stays the same move after the sketch.
                if let Some(inv) = delta.inverse() {
                    l.offset = inv.then(l.offset).then(*delta);
                }
                follow.extend(others);
            } else if l.objects.iter().any(|id| ids.contains(id)) {
                l.offset = l.offset.then(*delta);
                follow.extend(others);
            }
        }
        for id in follow {
            if let Some(o) = self.object_mut(id) {
                o.transform(delta);
            }
        }
    }
}

/// Gives the new kind `k` the settings of the old object `old` when they
/// are of one type: the stitch settings, and the style of a vector.
fn keep_settings(k: &mut ObjectKind, old: &ObjectKind) {
    if std::mem::discriminant(k) != std::mem::discriminant(old) {
        return;
    }
    let mut c = CurrentProperties::default();
    if c.take_from(old) {
        c.apply_to(k);
    }
    match (k, old) {
        (ObjectKind::Vector { style, .. }, ObjectKind::Vector { style: s, .. }) => *style = *s,
        (ObjectKind::Applique { pieces, .. }, ObjectKind::Applique { pieces: p, .. }) if pieces.len() == p.len() => {
            for (n, o) in pieces.iter_mut().zip(p) {
                n.layer = o.layer;
                n.fabric = o.fabric;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Point};
    use emb_sketch::{Constraint, ConstraintId, Sketch};

    use super::*;
    use crate::{Document, ShapeError, ShapeOp, ShapeOptions};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A frame: a rectangle of 60 × 40 mm and an offset of it 5 mm away.
    /// Returns the sketch and the width dimension.
    fn frame() -> (Sketch, ConstraintId) {
        let mut s = Sketch::new();
        let c = [p(0.0, 0.0), p(60.0, 0.0), p(60.0, 40.0), p(0.0, 40.0)].map(|q| s.add_point(q));
        let l = [0, 1, 2, 3].map(|i| s.add_line_between(c[i], c[(i + 1) % 4]).unwrap());
        for i in [0, 2] {
            s.add_constraint(Constraint::Horizontal { line: l[i] }).unwrap();
        }
        for i in [1, 3] {
            s.add_constraint(Constraint::Vertical { line: l[i] }).unwrap();
        }
        let w = s.add_constraint(Constraint::Length { line: l[0], mm: 60.0 }).unwrap();
        s.add_constraint(Constraint::Length { line: l[1], mm: 40.0 }).unwrap();
        s.add_constraint(Constraint::Fix {
            point: c[0],
            at: p(0.0, 0.0),
        })
        .unwrap();
        s.offset(&l, 5.0).unwrap();
        assert!(s.solve().converged);
        assert_eq!(s.regions().len(), 2);
        (s, w)
    }

    fn bbox(d: &Design, id: ObjectId) -> BBox {
        d.bbox_of(&[id]).unwrap()
    }

    /// A design with the frame: the inner region as a fill and the ring as
    /// a satin border. Returns the design, the sketch, the fill, the
    /// border and the width dimension.
    fn design() -> (Design, ObjectId, ObjectId, Vec<ObjectId>, ConstraintId) {
        let (s, w) = frame();
        let mut d = Design::new("frame");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let sk = d.add_object(t, ObjectKind::Sketch(Box::new(SketchObject::new(s))));
        let ObjectKind::Sketch(o) = d.object(sk).unwrap().kind.clone() else {
            unreachable!()
        };
        let regions = o.regions();
        let ring = regions.iter().position(|r| !r.holes.is_empty()).unwrap();
        let inner = 1 - ring;
        let source = |i: usize, to: RegionTo| LinkSource::Region {
            key: o.region_key(&regions[i]),
            to,
            width_mm: 6.0,
            color: Rgb(0, 0, 0),
        };
        let current = d.current.clone();
        let fill = d.make_from_sketch(sk, source(inner, RegionTo::Fill), &current, t);
        let border = d.make_from_sketch(sk, source(ring, RegionTo::Satin), &current, t);
        assert_eq!(fill.len(), 1);
        assert!(!border.is_empty());
        assert_eq!(d.sketch_links.len(), 2);
        (d, sk, fill[0], border, w)
    }

    fn change(d: &mut Design, sk: ObjectId, f: impl FnOnce(&mut Sketch)) -> usize {
        if let Some(ObjectKind::Sketch(s)) = d.object_mut(sk).map(|o| &mut o.kind) {
            s.edit(|sk| {
                f(sk);
                sk.solve();
            });
        }
        d.refresh_sketch_links(sk)
    }

    #[test]
    fn a_wider_sketch_makes_the_fill_and_the_border_again_with_their_settings() {
        let (mut d, sk, fill, border, w) = design();
        let w0 = bbox(&d, fill).width();
        // The user changes the fill and the thread of the border.
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(fill).map(|o| &mut o.kind) {
            params.angle_deg = 30.0;
            params.spacing_mm = 0.3;
        }
        d.object_mut(border[0]).unwrap().thread = blue;
        let order: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        let mut doc = Document::new(d);
        let before = doc.design().clone();
        let lost = doc.edit("Width", |d| change(d, sk, |s| s.set_value(w, 80.0).unwrap()));
        assert_eq!(lost, 0);
        let d = doc.design();
        assert!(
            (bbox(d, fill).width() - (w0 + 20.0)).abs() < 1e-3,
            "{}",
            bbox(d, fill).width()
        );
        let Some(ObjectKind::Fill { params, .. }) = d.object(fill).map(|o| &o.kind) else {
            panic!("still a fill")
        };
        assert_eq!((params.angle_deg, params.spacing_mm), (30.0, 0.3));
        assert_eq!(d.object(border[0]).unwrap().thread, blue);
        assert_eq!(
            d.objects().iter().map(|o| o.id).collect::<Vec<_>>(),
            order,
            "the same sew order"
        );
        assert!(bbox(d, border[0]).width() > 80.0 - 1e-6);
        // One undo takes it all back.
        doc.undo();
        assert!(doc.design().same_content(&before));
    }

    #[test]
    fn the_objects_follow_a_move_of_the_sketch_and_keep_their_own_move() {
        let (mut d, sk, fill, _, w) = design();
        let at = bbox(&d, fill).min;
        d.transform_objects(&[sk], &Transform::translate(100.0, 0.0));
        assert!((bbox(&d, fill).min - at - p(100.0, 0.0)).length() < 1e-6);
        // The fill alone moves 10 mm down; after a change it stays there.
        d.transform_objects(&[fill], &Transform::translate(0.0, 10.0));
        change(&mut d, sk, |s| s.set_value(w, 70.0).unwrap());
        assert!(
            (bbox(&d, fill).min - at - p(100.0, 10.0)).length() < 1e-6,
            "{:?}",
            bbox(&d, fill)
        );
        // A turn of the sketch turns the fill with it, also after a change.
        d.transform_objects(&[sk], &Transform::rotate_deg(90.0));
        let b = bbox(&d, fill);
        change(&mut d, sk, |s| s.set_value(w, 70.0).unwrap());
        let c = bbox(&d, fill);
        assert!(
            (b.min - c.min).length() < 1e-6 && (b.max - c.max).length() < 1e-6,
            "{b:?} {c:?}"
        );
        assert!(c.height() > c.width(), "the long side is upright now");
    }

    #[test]
    fn a_region_that_is_gone_leaves_the_objects_and_comes_back() {
        let (mut d, sk, fill, border, _) = design();
        let shape = d.object(border[0]).unwrap().kind.clone();
        // A line of the outside of the ring (not round the inner region).
        let sides = |id: ObjectId| match &d.sketch_link_of(id).unwrap().source {
            LinkSource::Region { key, .. } => key.sides.iter().map(|s| s.0).collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        let first = *sides(border[0]).iter().find(|c| !sides(fill).contains(c)).unwrap();
        // Without that line, the ring is gone; the inner region stays.
        let mut gone = d.clone();
        let removed = gone.clone();
        let lost = change(&mut gone, sk, |s| {
            s.remove_curve(first);
        });
        assert_eq!(lost, border.len(), "the ring is gone");
        assert_eq!(change(&mut gone, sk, |_| {}), 0, "lost once");
        assert!(gone.sketch_link_of(border[0]).unwrap().lost);
        assert_eq!(gone.object(border[0]).unwrap().kind, shape, "the border is not changed");
        // The sketch as it was: the ring comes back.
        let ObjectKind::Sketch(s) = removed.object(sk).unwrap().kind.clone() else {
            unreachable!()
        };
        gone.object_mut(sk).unwrap().kind = ObjectKind::Sketch(s);
        assert_eq!(gone.refresh_sketch_links(sk), 0);
        assert!(!gone.sketch_link_of(border[0]).unwrap().lost);
        // Unlink ends the record; the objects stay.
        let id = gone.sketch_link_of(border[0]).unwrap().id;
        assert!(gone.unlink_sketch(id));
        assert!(gone.sketch_link_of(border[0]).is_none() && gone.object(border[0]).is_some());
        // A deleted object leaves its link; a deleted sketch ends all links.
        d.remove_object(fill);
        assert_eq!(d.sketch_links.len(), 1);
        d.remove_object(sk);
        assert!(d.sketch_links.is_empty() && d.object(border[0]).is_some());
    }

    #[test]
    fn a_conversion_stays_after_a_change_of_the_sketch() {
        let (mut d, sk, fill, _, w) = design();
        let to_vector = |k: &ObjectKind| crate::vector::to_vector(k, Rgb(1, 2, 3));
        d.convert_linked(fill, None, &to_vector).unwrap();
        change(&mut d, sk, |s| s.set_value(w, 75.0).unwrap());
        assert!(matches!(d.object(fill).unwrap().kind, ObjectKind::Vector { .. }));
        assert!((bbox(&d, fill).width() - 75.0).abs() < 10.0 + 1e-3);
        // A conversion that a region cannot make fails with no change.
        assert!(d.convert_linked(fill, None, &|_| None).is_err());
    }

    #[test]
    fn links_save_and_shaping_refuses_linked_objects() {
        let (d, sk, fill, border, _) = design();
        assert!(d.is_linked(fill));
        assert_eq!(d.sketch_for(fill), Some(sk));
        assert_eq!(d.sketch_for(sk), Some(sk));
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert!(back.same_content(&d));
        let mut d = d;
        assert_eq!(
            d.shape(&[fill, border[0]], ShapeOp::Flatten, ShapeOptions::default()),
            Err(ShapeError::AlreadyLinked)
        );
    }
}
