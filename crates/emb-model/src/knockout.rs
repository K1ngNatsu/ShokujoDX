//! Lettering knockout (ADR 0093): objects in front cut the stitches of a
//! lettering, and the lettering stays a lettering.
//!
//! A lettering with a knockout (`Lettering::knockout`) does not sew where
//! the objects that sew after it cover it. After each edit,
//! `Design::settle_knockouts` makes the cut area again from these objects.
//! The stitches (`emb_stitch::knockout`) leave out the cut area.

use std::collections::BTreeMap;

use emb_geom::BBox;
use emb_geom::region::{self, Region};

use crate::lettering::{CutArea, Knockout};
use crate::shaping::{cut_area_of, shrink};
use crate::{Design, ObjectId, ObjectKind};

/// What a cut area was made from: the objects in front with their
/// revisions, and the overlap (its bits).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Key {
    front: Vec<(ObjectId, u64)>,
    overlap: u64,
}

/// The key of each lettering with a knockout at the last settle.
pub(crate) type Seen = BTreeMap<ObjectId, Key>;

/// Returns `true` if objects of this kind cut a lettering under them: the
/// objects that sew an area. Vector objects do not sew, and lines,
/// stitches, cross stitch and stones have no area.
pub fn cuts_lettering(kind: &ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::Fill { .. } | ObjectKind::Satin { .. } | ObjectKind::Applique { .. } | ObjectKind::Lettering(_)
    )
}

impl Design {
    /// The objects that sew after the object at `at` and can cut a
    /// lettering in the box `b`, with their revisions.
    fn in_front(&self, at: usize, b: &BBox) -> Vec<(ObjectId, u64)> {
        self.objects[at + 1..]
            .iter()
            .filter(|o| cuts_lettering(&o.kind) && o.control_bbox().is_some_and(|ob| ob.intersects(b)))
            .map(|o| (o.id, o.revision))
            .collect()
    }

    /// Makes the cut area of each lettering with a knockout again (call
    /// it after each edit; `Document` does). A lettering changes only if
    /// its cut area changes.
    pub fn settle_knockouts(&mut self) {
        let jobs: Vec<(ObjectId, Key)> = self
            .objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| match &o.kind {
                ObjectKind::Lettering(l) => {
                    let k = l.knockout.as_ref()?;
                    let front = o.control_bbox().map_or_else(Vec::new, |b| self.in_front(i, &b));
                    Some((
                        o.id,
                        Key {
                            front,
                            overlap: k.overlap_mm.to_bits(),
                        },
                    ))
                }
                _ => None,
            })
            .collect();
        if jobs.is_empty() {
            self.knockout_seen.clear();
            return;
        }
        let mut seen = std::mem::take(&mut self.knockout_seen);
        seen.retain(|id, _| jobs.iter().any(|(j, ..)| j == id));
        for (id, key) in jobs {
            if seen.get(&id) == Some(&key) {
                continue;
            }
            let area = self.cut_area(&key.front, f64::from_bits(key.overlap));
            if let Some(o) = self.object(id)
                && let ObjectKind::Lettering(l) = &o.kind
                && l.knockout.as_ref().is_some_and(|k| k.area != area)
                && let Some(o) = self.object_mut(id)
                && let ObjectKind::Lettering(l) = &mut o.kind
                && let Some(k) = &mut l.knockout
            {
                k.area = area;
            }
            seen.insert(id, key);
        }
        self.knockout_seen = seen;
    }

    /// The area of the objects `front`, made smaller by `overlap_mm`.
    fn cut_area(&self, front: &[(ObjectId, u64)], overlap_mm: f64) -> Vec<CutArea> {
        let all = front
            .iter()
            .filter_map(|(c, _)| cut_area_of(&self.object(*c)?.kind))
            .fold(Vec::new(), |acc: Vec<Region>, a| region::union(&acc, &a));
        shrink(&all, overlap_mm).iter().map(CutArea::from_region).collect()
    }

    /// Returns `true` if objects that sew after object `id` cover a part of
    /// its area (for lettering, of its letters).
    pub fn covered_in_front(&self, id: ObjectId) -> bool {
        let Some(at) = self.index_of(id) else { return false };
        let o = &self.objects[at];
        let (Some(b), Some(area)) = (o.control_bbox(), cut_area_of(&o.kind)) else {
            return false;
        };
        self.in_front(at, &b).iter().any(|(c, _)| {
            self.object(*c)
                .and_then(|f| cut_area_of(&f.kind))
                .is_some_and(|a| crate::shaping::overlaps(&area, &a))
        })
    }

    /// Gives lettering `id` a knockout with the overlap `overlap_mm` (or
    /// sets the overlap of its knockout), and makes the cut area. Returns
    /// `false` (with no change) if `id` is not a lettering.
    pub fn add_knockout(&mut self, id: ObjectId, overlap_mm: f64) -> bool {
        let Some(o) = self.object_mut(id) else { return false };
        let ObjectKind::Lettering(l) = &mut o.kind else {
            return false;
        };
        let k = l.knockout.get_or_insert_with(|| Knockout {
            overlap_mm,
            area: Vec::new(),
        });
        k.overlap_mm = overlap_mm;
        // Make the area now (not only when the edit ends).
        self.knockout_seen.remove(&id);
        self.settle_knockouts();
        true
    }

    /// Removes the knockout of lettering `id`. Returns `false` if it has none.
    pub fn remove_knockout(&mut self, id: ObjectId) -> bool {
        let has = matches!(&self.object(id).map(|o| &o.kind), Some(ObjectKind::Lettering(l)) if l.knockout.is_some());
        if has
            && let Some(o) = self.object_mut(id)
            && let ObjectKind::Lettering(l) = &mut o.kind
        {
            l.knockout = None;
        }
        has
    }

    /// Sets the overlap of the knockout of lettering `id`.
    pub fn set_knockout_overlap(&mut self, id: ObjectId, overlap_mm: f64) {
        let has = matches!(&self.object(id).map(|o| &o.kind), Some(ObjectKind::Lettering(l)) if l.knockout.as_ref().is_some_and(|k| k.overlap_mm != overlap_mm));
        if has
            && let Some(o) = self.object_mut(id)
            && let ObjectKind::Lettering(l) = &mut o.kind
            && let Some(k) = &mut l.knockout
        {
            k.overlap_mm = overlap_mm;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;
    use crate::{FillParams, Rgb, SatinGeometry, SatinParams};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A lettering with one satin letter: a column from (0, 0) to (20, 0),
    /// 4 mm wide.
    pub(crate) fn lettering() -> ObjectKind {
        let mut l = crate::Lettering::new("I", crate::FontRef::default(), 10.0, Point::ZERO);
        let g = SatinGeometry::from_centerline(&Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)), 4.0);
        l.letters = vec![crate::Letter {
            text: "I".into(),
            line: 0,
            index: 0,
            frame: emb_geom::Transform::IDENTITY,
            center: Point::ZERO,
            parts: vec![ObjectKind::Satin {
                geometry: g,
                params: SatinParams::default(),
            }],
        }];
        ObjectKind::Lettering(Box::new(l))
    }

    pub(crate) fn square(x: f64, y: f64, s: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(x, y), p(x + s, y + s))),
            holes: Vec::new(),
            params: FillParams::default(),
        }
    }

    pub(crate) fn knockout(d: &Design, id: ObjectId) -> Option<Knockout> {
        match &d.object(id)?.kind {
            ObjectKind::Lettering(l) => l.knockout.clone(),
            _ => None,
        }
    }

    pub(crate) fn area(k: &Knockout) -> f64 {
        k.regions().iter().map(Region::area).sum()
    }

    #[test]
    fn the_cut_follows_the_object_in_front() {
        let mut d = Design::new("k");
        let t = d.add_thread("a", Rgb(0, 0, 0));
        let text = d.add_object(t, lettering());
        let front = d.add_object(t, square(5.0, -5.0, 10.0));
        assert!(d.add_knockout(text, 0.0));
        let k = knockout(&d, text).unwrap();
        assert!((area(&k) - 100.0).abs() < 0.5, "{}", area(&k));
        // A move of the front object moves the cut.
        if let Some(o) = d.object_mut(front) {
            o.kind = square(0.0, -5.0, 5.0);
        }
        d.settle_knockouts();
        let k = knockout(&d, text).unwrap();
        assert!((area(&k) - 25.0).abs() < 0.5);
        // The overlap makes the cut smaller.
        d.set_knockout_overlap(text, 1.0);
        d.settle_knockouts();
        assert!((area(&knockout(&d, text).unwrap()) - 9.0).abs() < 0.5);
    }

    #[test]
    fn only_objects_in_front_cut_and_new_ones_join() {
        let mut d = Design::new("k");
        let t = d.add_thread("a", Rgb(0, 0, 0));
        let back = d.add_object(t, square(5.0, -5.0, 10.0));
        let text = d.add_object(t, lettering());
        assert!(d.add_knockout(text, 0.0));
        assert!(!d.covered_in_front(text));
        assert!(
            knockout(&d, text).unwrap().area.is_empty(),
            "an object behind does not cut"
        );
        // An object that is added in front cuts with no command.
        let front = d.add_object(t, square(0.0, -1.5, 3.0));
        d.settle_knockouts();
        assert!(d.covered_in_front(text));
        assert!((area(&knockout(&d, text).unwrap()) - 9.0).abs() < 0.5);
        let before = d.object(text).unwrap().revision;
        // Nothing changed: the lettering keeps its revision.
        d.settle_knockouts();
        assert_eq!(d.object(text).unwrap().revision, before);
        // The object behind comes to the front: it cuts too.
        d.move_objects_to_end(&[back]);
        d.settle_knockouts();
        assert!((area(&knockout(&d, text).unwrap()) - 109.0).abs() < 1.0);
        d.remove_object(front);
        d.remove_object(back);
        d.settle_knockouts();
        let k = knockout(&d, text).unwrap();
        assert!(k.area.is_empty(), "the knockout stays, with nothing to cut");
    }

    #[test]
    fn lettering_in_front_cuts_by_its_letters() {
        let mut d = Design::new("k");
        let t = d.add_thread("a", Rgb(0, 0, 0));
        let text = d.add_object(t, lettering());
        d.add_object(t, lettering());
        assert!(d.add_knockout(text, 0.0));
        // The column is 20 x 4 mm.
        assert!((area(&knockout(&d, text).unwrap()) - 80.0).abs() < 1.0);
        assert!(d.remove_knockout(text));
        assert!(!d.remove_knockout(text));
    }
}
