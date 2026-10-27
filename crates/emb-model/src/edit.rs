//! Object edits: move, rotate, scale, mirror, duplicate and copy.
//!
//! A transform changes the geometry and also every direction parameter
//! (fill angle, tatami underlay angle), so the stitches turn with the shape.
//! Spacing and stitch length do not scale: they are thread properties, not
//! shape properties.

use std::sync::Arc;

use emb_geom::{BBox, Point, Transform, math};

use crate::object::fresh_revision;
use crate::{Design, DesignObject, FillPattern, ObjectId, ObjectKind, Underlay, UnderlayLayer};

/// Returns the stitch angle after the transform, in degrees in [0, 180).
///
/// Stitch rows have no direction, so 30° and 210° are the same angle.
pub fn transform_angle(deg: f64, t: &Transform) -> f64 {
    let (s, c) = math::sin_cos_deg(deg);
    let v = t.apply_vector(Point::new(c, s));
    if v.length_sq() == 0.0 {
        return deg;
    }
    let a = math::to_degrees(math::atan2(v.y, v.x)).rem_euclid(180.0);
    // Remove rounding noise near whole degrees, for clean values in the UI.
    let r = a.round();
    let a = if (a - r).abs() < 1e-9 { r } else { a };
    if a >= 180.0 { a - 180.0 } else { a }
}

fn transform_underlay(u: &mut Underlay, t: &Transform) {
    if let Underlay::Custom(layers) = u {
        for layer in layers {
            if let UnderlayLayer::Tatami { angle_deg, .. } = layer {
                *angle_deg = transform_angle(*angle_deg, t);
            }
        }
    }
}

impl ObjectKind {
    /// Applies the transform to the geometry and to the direction parameters.
    pub fn transform(&mut self, t: &Transform) {
        match self {
            ObjectKind::Running { path, params } => {
                *path = path.transformed(t);
                if let Some(r) = &mut params.ripple {
                    r.target = r.target.map(|p| t.apply(p));
                    for p in r.guide.iter_mut() {
                        *p = t.apply(*p);
                    }
                }
            }
            ObjectKind::Satin { geometry, params } => {
                geometry.rail_a = geometry.rail_a.transformed(t);
                geometry.rail_b = geometry.rail_b.transformed(t);
                for (a, b) in &mut geometry.rungs {
                    *a = t.apply(*a);
                    *b = t.apply(*b);
                }
                transform_underlay(&mut params.underlay, t);
            }
            ObjectKind::Fill { outer, holes, params } => {
                *outer = outer.transformed(t);
                for h in holes.iter_mut() {
                    *h = h.transformed(t);
                }
                params.angle_deg = transform_angle(params.angle_deg, t);
                transform_underlay(&mut params.underlay, t);
                match &mut params.pattern {
                    FillPattern::Radial { center: Some(c) } | FillPattern::Circular { center: Some(c) } => {
                        *c = t.apply(*c)
                    }
                    FillPattern::Gradient { line: Some((a, b)), .. } => {
                        *a = t.apply(*a);
                        *b = t.apply(*b);
                    }
                    FillPattern::Guided { guides } => {
                        for p in guides.iter_mut().flatten() {
                            *p = t.apply(*p);
                        }
                    }
                    FillPattern::Florentine { guide } => {
                        for p in guide.iter_mut() {
                            *p = t.apply(*p);
                        }
                    }
                    FillPattern::UserSplit { lines } => {
                        for p in lines.iter_mut().flatten() {
                            *p = t.apply(*p);
                        }
                    }
                    _ => {}
                }
                if let Some(dir) = &mut params.direction {
                    for p in dir.points.iter_mut() {
                        *p = t.apply(*p);
                    }
                    // A mirror turns the angle to the line the other way.
                    let (x, y) = (
                        t.apply_vector(Point::new(1.0, 0.0)),
                        t.apply_vector(Point::new(0.0, 1.0)),
                    );
                    if x.cross(y) < 0.0 {
                        dir.angle_deg = 180.0 - dir.angle_deg;
                    }
                }
                // Stamps move, turn and scale with the fill.
                let scale = t.apply_vector(Point::new(1.0, 0.0)).length();
                for s in &mut params.stamps {
                    s.center = t.apply(s.center);
                    s.size_mm *= scale;
                    let (sin, cos) = math::sin_cos_deg(s.angle_deg);
                    let v = t.apply_vector(Point::new(cos, sin));
                    s.angle_deg = math::to_degrees(math::atan2(v.y, v.x));
                }
            }
            ObjectKind::Stitches { path, .. } => *path = path.transformed(t),
            ObjectKind::Vector { path, style } => {
                *path = path.transformed(t);
                style.stroke_mm *= t.apply_vector(Point::new(1.0, 0.0)).length();
            }
            ObjectKind::Applique { pieces, .. } => {
                for p in pieces {
                    p.outer = p.outer.transformed(t);
                    for h in p.holes.iter_mut() {
                        *h = h.transformed(t);
                    }
                }
            }
            ObjectKind::Lettering(l) => {
                l.placement = l.placement.then(*t);
                l.fill.angle_deg = transform_angle(l.fill.angle_deg, t);
                for part in l.parts_mut() {
                    part.transform(t);
                }
                // A scale changes the height (and the size warning), not
                // only the placement.
                l.scale_into_height();
            }
            ObjectKind::CrossStitch(x) => x.transform(t),
            ObjectKind::Bling(b) => b.transform(t),
            ObjectKind::Sketch(s) => s.transform(t),
        }
    }
}

impl DesignObject {
    /// Applies the transform to the geometry, the direction parameters and the
    /// start and end points.
    pub fn transform(&mut self, t: &Transform) {
        self.kind.transform(t);
        self.start = self.start.map(|p| t.apply(p));
        self.end = self.end.map(|p| t.apply(p));
    }

    pub fn transformed(&self, t: &Transform) -> DesignObject {
        let mut o = self.clone();
        o.transform(t);
        o
    }

    /// Returns `true` if `p` is on the object: inside a fill or a satin
    /// column, or within `tol` mm of a line.
    pub fn hit_test(&self, p: Point, tol: f64) -> bool {
        self.control_bbox().is_some_and(|b| b.expand(tol).contains(p)) && self.kind.hit_test(p, tol)
    }
}

impl ObjectKind {
    /// As [`DesignObject::hit_test`], with no bounding-box check first.
    pub fn hit_test(&self, p: Point, tol: f64) -> bool {
        let flat_tol = (tol / 4.0).clamp(0.01, 0.5);
        let near = |path: &emb_geom::Path| {
            path.flatten(flat_tol)
                .iter()
                .any(|pl| pl.segments().any(|(a, b)| p.distance_to_segment(a, b) <= tol))
        };
        let ring = |path: &emb_geom::Path| -> Vec<Point> {
            path.flatten(flat_tol).into_iter().flat_map(|pl| pl.points).collect()
        };
        match self {
            ObjectKind::Lettering(l) => l.parts().any(|part| part.hit_test(p, tol)),
            ObjectKind::CrossStitch(x) => x.hit_test(p, tol),
            ObjectKind::Bling(b) => b.hit_test(p, tol),
            ObjectKind::Sketch(s) => s.paths().any(near),
            ObjectKind::Running { path, .. } | ObjectKind::Stitches { path, .. } => near(path),
            ObjectKind::Vector { path, style } => {
                // Inside the fill (even-odd), or on a subpath.
                let inside = style.fill.is_some()
                    && path
                        .flatten(flat_tol)
                        .iter()
                        .filter(|pl| pl.closed && emb_geom::region::ring_contains(&pl.points, p))
                        .count()
                        % 2
                        == 1;
                inside || near(path)
            }
            ObjectKind::Fill { outer, holes, .. } => {
                let inside = emb_geom::region::ring_contains(&ring(outer), p)
                    && !holes.iter().any(|h| emb_geom::region::ring_contains(&ring(h), p));
                inside || near(outer) || holes.iter().any(near)
            }
            ObjectKind::Applique { pieces, .. } => pieces.iter().any(|pc| {
                let inside = emb_geom::region::ring_contains(&ring(&pc.outer), p)
                    && !pc.holes.iter().any(|h| emb_geom::region::ring_contains(&ring(h), p));
                inside || near(&pc.outer) || pc.holes.iter().any(near)
            }),
            ObjectKind::Satin { geometry, .. } => {
                let (a, b) = (&geometry.rail_a, &geometry.rail_b);
                if near(a) || near(b) {
                    return true;
                }
                let closed = |path: &emb_geom::Path| path.flatten(flat_tol).first().is_some_and(|pl| pl.closed);
                if closed(a) && closed(b) {
                    // A ring: between the two closed rails.
                    emb_geom::region::ring_contains(&ring(a), p) != emb_geom::region::ring_contains(&ring(b), p)
                } else {
                    // A column: inside the outline made of rail A and rail B reversed.
                    let mut outline = ring(a);
                    outline.extend(ring(b).into_iter().rev());
                    emb_geom::region::ring_contains(&outline, p)
                }
            }
        }
    }
}

/// Transform that rotates by `deg` about `center`. Positive turns clockwise on screen.
pub fn rotate_about(center: Point, deg: f64) -> Transform {
    Transform::translate(-center.x, -center.y)
        .then(Transform::rotate_deg(deg))
        .then(Transform::translate(center.x, center.y))
}

/// Transform that scales by `sx`, `sy` with `anchor` fixed.
pub fn scale_about(anchor: Point, sx: f64, sy: f64) -> Transform {
    Transform::translate(-anchor.x, -anchor.y)
        .then(Transform::scale(sx, sy))
        .then(Transform::translate(anchor.x, anchor.y))
}

impl Design {
    /// Bounding box of the objects (control points included).
    pub fn bbox_of(&self, ids: &[ObjectId]) -> Option<BBox> {
        ids.iter()
            .filter_map(|id| self.object(*id))
            .filter_map(DesignObject::control_bbox)
            .reduce(BBox::union)
    }

    /// Applies the transform to each object. Returns `true` if one or more objects changed.
    pub fn transform_objects(&mut self, ids: &[ObjectId], t: &Transform) -> bool {
        if *t == Transform::IDENTITY {
            return false;
        }
        let mut changed = false;
        for id in ids {
            if let Some(o) = self.object_mut(*id) {
                o.transform(t);
                changed = true;
            }
        }
        // Linked cuts follow (see `link`).
        self.linked_objects_moved(ids, t);
        changed
    }

    /// Adds a copy of `obj` at the end of the sew order with a new id. The
    /// thread must be in the palette; see [`Design::paste_objects`] for objects
    /// from another design.
    pub fn add_object_copy(&mut self, obj: &DesignObject) -> ObjectId {
        let id = self.next_object_id();
        let mut copy = obj.clone();
        copy.id = id;
        copy.revision = fresh_revision();
        self.objects.push(Arc::new(copy));
        id
    }

    /// Copies the objects (in sew order) to the end of the sew order, moved
    /// by `offset`. Returns the new ids.
    pub fn duplicate_objects(&mut self, ids: &[ObjectId], offset: Point) -> Vec<ObjectId> {
        let t = Transform::translate(offset.x, offset.y);
        let originals: Vec<DesignObject> = self
            .objects
            .iter()
            .filter(|o| ids.contains(&o.id))
            .map(|o| o.transformed(&t))
            .collect();
        let ids: Vec<ObjectId> = originals.iter().map(|o| self.add_object_copy(o)).collect();
        self.regroup(&ids);
        ids
    }

    /// A group number that no object has.
    pub fn new_group_id(&self) -> u32 {
        self.objects.iter().filter_map(|o| o.group).max().map_or(1, |g| g + 1)
    }

    /// Gives the copies `ids` new group numbers, so they do not join the groups of the originals.
    fn regroup(&mut self, ids: &[ObjectId]) {
        let mut map = std::collections::BTreeMap::new();
        let mut next = self.new_group_id();
        for &id in ids {
            if let Some(g) = self.object(id).and_then(|o| o.group) {
                let new = *map.entry(g).or_insert_with(|| {
                    next += 1;
                    next - 1
                });
                if let Some(o) = self.object_mut(id) {
                    o.group = Some(new);
                }
            }
        }
    }

    /// Adds copies of objects from a clipboard. Threads that are not in this
    /// palette (by id and color) are added. Returns the new ids.
    pub fn paste_objects(&mut self, objects: &[DesignObject], threads: &[crate::Thread]) -> Vec<ObjectId> {
        let mut ids = Vec::with_capacity(objects.len());
        for obj in objects {
            let mut o = obj.clone();
            let src = threads.iter().find(|t| t.id == obj.thread);
            let same = |t: &crate::Thread| src.is_some_and(|s| s.color == t.color && s.name == t.name);
            o.thread = match self.threads.iter().find(|t| same(t)) {
                Some(t) => t.id,
                None => match src {
                    Some(s) => {
                        let id = self.add_thread(s.name.clone(), s.color);
                        if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
                            t.brand = s.brand.clone();
                            t.code = s.code.clone();
                        }
                        id
                    }
                    // Unknown source thread: keep the id if it exists here, else use the first thread.
                    None if self.thread(o.thread).is_some() => o.thread,
                    None => match self.threads.first() {
                        Some(t) => t.id,
                        None => self.add_thread("Thread 1", crate::Rgb(0, 0, 0)),
                    },
                },
            };
            ids.push(self.add_object_copy(&o));
        }
        self.regroup(&ids);
        ids
    }

    /// Moves the objects so that the center of their bounding box is at `target`.
    pub fn center_objects_at(&mut self, ids: &[ObjectId], target: Point) -> bool {
        let Some(b) = self.bbox_of(ids) else { return false };
        let d = target - b.center();
        self.transform_objects(ids, &Transform::translate(d.x, d.y))
    }
}

#[cfg(test)]
mod tests {
    /// The topmost object (last in sew order) at `p`, as the canvas picks.
    fn object_at(d: &Design, p: Point, tol: f64) -> Option<ObjectId> {
        d.objects().iter().rev().find(|o| o.hit_test(p, tol)).map(|o| o.id)
    }
    use emb_geom::Path;

    use super::*;
    use crate::{FillParams, Rgb, ThreadId};

    #[test]
    fn a_direction_line_moves_and_mirrors_with_its_fill() {
        let mut k = ObjectKind::Fill {
            outer: emb_geom::Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0))),
            holes: Vec::new(),
            params: FillParams {
                direction: Some(crate::DirectionLine {
                    points: vec![Point::new(0.0, 5.0), Point::new(10.0, 5.0)],
                    angle_deg: 60.0,
                }),
                ..Default::default()
            },
        };
        k.transform(&Transform::translate(5.0, 0.0));
        let dir = |k: &ObjectKind| match k {
            ObjectKind::Fill { params, .. } => params.direction.clone().unwrap(),
            _ => panic!(),
        };
        assert_eq!(dir(&k).points[0], Point::new(5.0, 5.0));
        assert_eq!(dir(&k).angle_deg, 60.0);
        // A mirror turns the stitches the other way to the line.
        k.transform(&Transform::scale(-1.0, 1.0));
        assert_eq!(dir(&k).points[1], Point::new(-15.0, 5.0));
        assert_eq!(dir(&k).angle_deg, 120.0);
    }

    fn square_fill(d: &mut Design, x: f64, angle: f64) -> ObjectId {
        let t = d
            .threads
            .first()
            .map(|t| t.id)
            .unwrap_or_else(|| d.add_thread("Red", Rgb(255, 0, 0)));
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + 10.0, 10.0))),
                holes: vec![],
                params: FillParams {
                    angle_deg: angle,
                    ..FillParams::default()
                },
            },
        )
    }

    fn angle(d: &Design, id: ObjectId) -> f64 {
        match &d.object(id).unwrap().kind {
            ObjectKind::Fill { params, .. } => params.angle_deg,
            _ => unreachable!(),
        }
    }

    #[test]
    fn angle_follows_rotation_and_mirror() {
        assert_eq!(transform_angle(45.0, &Transform::rotate_deg(90.0)), 135.0);
        assert_eq!(transform_angle(170.0, &Transform::rotate_deg(30.0)), 20.0);
        // Mirror left-right: 30° becomes 150°.
        assert_eq!(transform_angle(30.0, &Transform::scale(-1.0, 1.0)), 150.0);
        // Stretching in X makes a 45° angle flatter.
        assert!(transform_angle(45.0, &Transform::scale(2.0, 1.0)) < 45.0);
        // Pure move does not change the angle.
        assert_eq!(transform_angle(45.0, &Transform::translate(5.0, 5.0)), 45.0);
    }

    #[test]
    fn rotate_about_center_keeps_bbox_center() {
        let mut d = Design::new("t");
        let id = square_fill(&mut d, 0.0, 45.0);
        let c = d.bbox_of(&[id]).unwrap().center();
        assert!(d.transform_objects(&[id], &rotate_about(c, 90.0)));
        let b = d.bbox_of(&[id]).unwrap();
        assert!(b.center().distance(c) < 1e-9);
        assert_eq!(angle(&d, id), 135.0);
    }

    #[test]
    fn scale_keeps_anchor_and_start_point_follows() {
        let mut d = Design::new("t");
        let id = square_fill(&mut d, 0.0, 0.0);
        d.object_mut(id).unwrap().start = Some(Point::new(10.0, 10.0));
        d.transform_objects(&[id], &scale_about(Point::ZERO, 2.0, 3.0));
        let b = d.bbox_of(&[id]).unwrap();
        assert_eq!((b.min, b.max), (Point::ZERO, Point::new(20.0, 30.0)));
        assert_eq!(d.object(id).unwrap().start, Some(Point::new(20.0, 30.0)));
        assert!(!d.transform_objects(&[id], &Transform::IDENTITY));
    }

    #[test]
    fn duplicate_goes_to_end_with_offset() {
        let mut d = Design::new("t");
        let a = square_fill(&mut d, 0.0, 10.0);
        let b = square_fill(&mut d, 20.0, 20.0);
        let new = d.duplicate_objects(&[b, a], Point::new(5.0, 5.0));
        assert_eq!(new.len(), 2);
        let order: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(order, vec![a, b, new[0], new[1]]);
        // Copies keep sew order of the originals (a before b) and their settings.
        assert_eq!(angle(&d, new[0]), 10.0);
        assert_eq!(d.bbox_of(&[new[0]]).unwrap().min, Point::new(5.0, 5.0));
    }

    #[test]
    fn paste_maps_threads_by_color_and_name() {
        let mut src = Design::new("src");
        let id = square_fill(&mut src, 0.0, 0.0);
        let gold = src.add_thread("Gold", Rgb(230, 170, 30));
        src.object_mut(id).unwrap().thread = gold;
        let clip = vec![src.object(id).unwrap().clone()];

        let mut dst = Design::new("dst");
        dst.add_thread("Black", Rgb(0, 0, 0));
        let new = dst.paste_objects(&clip, &src.threads);
        let t = dst.object(new[0]).unwrap().thread;
        assert_eq!(dst.thread(t).unwrap().name, "Gold");
        // A second paste reuses the thread.
        dst.paste_objects(&clip, &src.threads);
        assert_eq!(dst.threads.len(), 2);
        assert_ne!(t, ThreadId(0));
    }

    #[test]
    fn hit_test_by_shape() {
        use crate::{RunningParams, SatinGeometry, SatinParams};
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        // A satin ring between radius 29 and 33, and a short line inside it, on top.
        let ring = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::ellipse(Point::ZERO, 33.0, 33.0),
                    rail_b: Path::ellipse(Point::ZERO, 29.0, 29.0),
                    rungs: vec![],
                },
                params: SatinParams::default(),
            },
        );
        let line = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(-10.0, 0.0))
                    .line_to(Point::new(10.0, 0.0)),
                params: RunningParams::default(),
            },
        );
        assert_eq!(object_at(&d, Point::new(31.0, 0.0), 0.2), Some(ring));
        assert_eq!(object_at(&d, Point::new(0.0, 0.1), 0.2), Some(line));
        // Inside the ring hole, away from the line: nothing (a box test would give the ring).
        assert_eq!(object_at(&d, Point::new(0.0, 15.0), 0.2), None);

        // Fill with a hole, and an open satin column.
        let fill = square_fill(&mut d, 100.0, 0.0);
        if let Some(ObjectKind::Fill { holes, .. }) = d.object_mut(fill).map(|o| &mut o.kind) {
            holes.push(Path::rect(BBox::new(Point::new(103.0, 3.0), Point::new(107.0, 7.0))));
        }
        assert_eq!(object_at(&d, Point::new(101.0, 1.0), 0.1), Some(fill));
        assert_eq!(object_at(&d, Point::new(105.0, 5.0), 0.1), None);
        let col = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::new()
                        .move_to(Point::new(200.0, 0.0))
                        .line_to(Point::new(200.0, 50.0)),
                    rail_b: Path::new()
                        .move_to(Point::new(203.0, 0.0))
                        .line_to(Point::new(203.0, 50.0)),
                    rungs: vec![],
                },
                params: SatinParams::default(),
            },
        );
        assert_eq!(object_at(&d, Point::new(201.5, 25.0), 0.1), Some(col));
        assert_eq!(object_at(&d, Point::new(210.0, 25.0), 0.1), None);
    }

    #[test]
    fn center_objects() {
        let mut d = Design::new("t");
        let a = square_fill(&mut d, 30.0, 0.0);
        assert!(d.center_objects_at(&[a], Point::ZERO));
        assert!(d.bbox_of(&[a]).unwrap().center().distance(Point::ZERO) < 1e-9);
        assert!(!d.center_objects_at(&[a], Point::ZERO));
    }
}
