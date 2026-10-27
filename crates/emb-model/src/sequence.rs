//! Sew-order operations for the Sequence panel.
//!
//! Sew order is also stacking order: an object that sews later lies on top
//! of the objects before it. "Forward" means later in the sew order (on top),
//! "backward" means earlier (below).
//!
//! All operations keep the relative order of the objects that they move.

use std::collections::BTreeSet;

use emb_geom::BBox;

use crate::{Design, ObjectId, ThreadId};

/// Direction for [`Design::step_objects`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// One place earlier in the sew order.
    Backward,
    /// One place later in the sew order.
    Forward,
}

impl Design {
    fn order(&self) -> Vec<ObjectId> {
        self.objects.iter().map(|o| o.id).collect()
    }

    /// Moves the objects in `ids` so that they sew just before `before`, or at
    /// the end if `before` is `None`. The moved objects keep their current
    /// relative order. If `before` is one of the moved objects, the target is
    /// the next object after it that does not move.
    ///
    /// Returns `true` if the sew order changed.
    pub fn move_objects_before(&mut self, ids: &[ObjectId], before: Option<ObjectId>) -> bool {
        let moving: BTreeSet<ObjectId> = ids.iter().copied().collect();
        let old = self.order();
        let target = before.and_then(|b| {
            let start = self.index_of(b)?;
            self.objects[start..]
                .iter()
                .map(|o| o.id)
                .find(|id| !moving.contains(id))
        });
        let (moved, mut rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.objects)
            .into_iter()
            .partition(|o| moving.contains(&o.id));
        let at = target
            .and_then(|t| rest.iter().position(|o| o.id == t))
            .unwrap_or(rest.len());
        rest.splice(at..at, moved);
        self.objects = rest;
        self.order() != old
    }

    /// Moves the objects to the start of the sew order (bottom of the stack).
    pub fn move_objects_to_start(&mut self, ids: &[ObjectId]) -> bool {
        let first = self.objects.first().map(|o| o.id);
        self.move_objects_before(ids, first)
    }

    /// Moves the objects to the end of the sew order (top of the stack).
    pub fn move_objects_to_end(&mut self, ids: &[ObjectId]) -> bool {
        self.move_objects_before(ids, None)
    }

    /// Moves each object one place past its neighbor that is not in `ids`.
    /// Returns `true` if the sew order changed.
    pub fn step_objects(&mut self, ids: &[ObjectId], step: Step) -> bool {
        let sel: BTreeSet<ObjectId> = ids.iter().copied().collect();
        let n = self.objects.len();
        let mut changed = false;
        match step {
            Step::Backward => {
                for i in 1..n {
                    if sel.contains(&self.objects[i].id) && !sel.contains(&self.objects[i - 1].id) {
                        self.objects.swap(i, i - 1);
                        changed = true;
                    }
                }
            }
            Step::Forward => {
                for i in (0..n.saturating_sub(1)).rev() {
                    if sel.contains(&self.objects[i].id) && !sel.contains(&self.objects[i + 1].id) {
                        self.objects.swap(i, i + 1);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Puts the objects in exactly the given order, at the position of the
    /// earliest of them. Use this for "sequence by click order".
    pub fn sequence_in_order(&mut self, ids: &[ObjectId]) -> bool {
        let wanted: Vec<ObjectId> = {
            let mut seen = BTreeSet::new();
            ids.iter()
                .copied()
                .filter(|id| self.index_of(*id).is_some() && seen.insert(*id))
                .collect()
        };
        if wanted.len() < 2 {
            return false;
        }
        let old = self.order();
        let set: BTreeSet<ObjectId> = wanted.iter().copied().collect();
        let first_pos = self.objects.iter().position(|o| set.contains(&o.id)).unwrap_or(0);
        let (mut moved, mut rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.objects)
            .into_iter()
            .partition(|o| set.contains(&o.id));
        moved.sort_by_key(|o| wanted.iter().position(|w| *w == o.id));
        // Objects before `first_pos` are all in `rest`, so the index is the same.
        rest.splice(first_pos..first_pos, moved);
        self.objects = rest;
        self.order() != old
    }

    /// Number of color changes that a perfect grouping by color could remove:
    /// color blocks minus distinct threads.
    pub fn avoidable_color_changes(&self) -> usize {
        let blocks = self.color_blocks();
        let distinct: BTreeSet<ThreadId> = blocks.iter().map(|(t, _)| *t).collect();
        blocks.len() - distinct.len()
    }

    /// Reduces color changes without changing what is on top of what.
    ///
    /// Each object moves back to join the last earlier block of the same
    /// thread, but only if its bounding box does not overlap any object that
    /// it would move under. Overlapping objects keep their layer order.
    /// The test uses bounding boxes, so it is conservative: it can keep a
    /// color change that is not necessary, but it never puts an object below
    /// one that it overlaps.
    pub fn optimize_color_changes(&mut self) -> bool {
        let old = self.order();
        let items: Vec<_> = std::mem::take(&mut self.objects);
        let mut out: Vec<std::sync::Arc<crate::DesignObject>> = Vec::with_capacity(items.len());
        let mut boxes: Vec<Option<BBox>> = Vec::with_capacity(items.len());

        for obj in items {
            let bb = obj.control_bbox();
            // Walk back from the end. Stop at the first overlap; join the
            // nearest same-thread object found before that.
            let mut insert_at = None;
            for j in (0..out.len()).rev() {
                if out[j].thread == obj.thread {
                    insert_at = Some(j + 1);
                    break;
                }
                let overlaps = matches!((&bb, &boxes[j]), (Some(a), Some(b)) if a.intersects(b));
                if overlaps {
                    break;
                }
            }
            match insert_at {
                Some(i) if i < out.len() => {
                    out.insert(i, obj);
                    boxes.insert(i, bb);
                }
                _ => {
                    out.push(obj);
                    boxes.push(bb);
                }
            }
        }
        self.objects = out;
        self.order() != old
    }

    /// Groups all objects by thread, in order of first use. This gives the
    /// fewest color changes, but it can put an object below an object that
    /// it overlapped before. Prefer [`Design::optimize_color_changes`].
    pub fn group_by_color(&mut self) -> bool {
        let old = self.order();
        let mut thread_order: Vec<ThreadId> = Vec::new();
        for o in &self.objects {
            if !thread_order.contains(&o.thread) {
                thread_order.push(o.thread);
            }
        }
        // A stable sort keeps the order inside each color.
        self.objects
            .sort_by_key(|o| thread_order.iter().position(|t| *t == o.thread));
        self.order() != old
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};

    use super::*;
    use crate::{ObjectKind, RunningParams};

    /// A short line at `x`, so objects at different `x` do not overlap.
    fn line_at(x: f64) -> ObjectKind {
        ObjectKind::Running {
            path: Path::new()
                .move_to(Point::new(x, 0.0))
                .line_to(Point::new(x + 1.0, 0.0)),
            params: RunningParams::default(),
        }
    }

    /// Makes a design with one object per entry: (thread, x position).
    fn design(spec: &[(u32, f64)]) -> (Design, Vec<ObjectId>) {
        let mut d = Design::new("t");
        let ids = spec
            .iter()
            .map(|&(t, x)| d.add_object(ThreadId(t), line_at(x)))
            .collect();
        (d, ids)
    }

    fn order(d: &Design) -> Vec<u64> {
        d.objects().iter().map(|o| o.id.0).collect()
    }

    fn threads(d: &Design) -> Vec<u32> {
        d.objects().iter().map(|o| o.thread.0).collect()
    }

    #[test]
    fn move_before_keeps_relative_order() {
        let (mut d, id) = design(&[(0, 0.0), (0, 2.0), (0, 4.0), (0, 6.0), (0, 8.0)]);
        // Move 4 and 2 (given out of order) before 1.
        assert!(d.move_objects_before(&[id[3], id[1]], Some(id[0])));
        assert_eq!(order(&d), vec![2, 4, 1, 3, 5]);
        // To the end.
        assert!(d.move_objects_before(&[id[0]], None));
        assert_eq!(order(&d), vec![2, 4, 3, 5, 1]);
        // No change gives false.
        assert!(!d.move_objects_before(&[id[0]], None));
    }

    #[test]
    fn move_before_a_moved_object_uses_next_unmoved() {
        let (mut d, id) = design(&[(0, 0.0), (0, 2.0), (0, 4.0), (0, 6.0)]);
        // Drop objects 1 and 2 "before 2": target is 3, so the order does not change.
        assert!(!d.move_objects_before(&[id[0], id[1]], Some(id[1])));
        assert_eq!(order(&d), vec![1, 2, 3, 4]);
        // Drop 1 and 3 before 3: target is 4.
        assert!(d.move_objects_before(&[id[0], id[2]], Some(id[2])));
        assert_eq!(order(&d), vec![2, 1, 3, 4]);
    }

    #[test]
    fn start_end_and_step() {
        let (mut d, id) = design(&[(0, 0.0), (0, 2.0), (0, 4.0), (0, 6.0)]);
        d.move_objects_to_start(&[id[2]]);
        assert_eq!(order(&d), vec![3, 1, 2, 4]);
        d.move_objects_to_end(&[id[2], id[0]]);
        assert_eq!(order(&d), vec![2, 4, 3, 1]);

        // Step two adjacent objects back together.
        assert!(d.step_objects(&[id[2], id[0]], Step::Backward));
        assert_eq!(order(&d), vec![2, 3, 1, 4]);
        assert!(d.step_objects(&[id[1]], Step::Forward));
        assert_eq!(order(&d), vec![3, 2, 1, 4]);
        // Already at the end.
        assert!(!d.step_objects(&[id[3]], Step::Forward));
    }

    #[test]
    fn sequence_in_click_order() {
        let (mut d, id) = design(&[(0, 0.0), (0, 2.0), (0, 4.0), (0, 6.0), (0, 8.0)]);
        // Click order 5, 2, 4: they go to the position of 2 (the earliest).
        assert!(d.sequence_in_order(&[id[4], id[1], id[3]]));
        assert_eq!(order(&d), vec![1, 5, 2, 4, 3]);
        assert!(!d.sequence_in_order(&[id[0]]));
    }

    #[test]
    fn optimize_merges_blocks_when_nothing_overlaps() {
        // Red, blue, red, blue, all apart.
        let (mut d, _) = design(&[(0, 0.0), (1, 10.0), (0, 20.0), (1, 30.0)]);
        assert_eq!(d.color_blocks().len(), 4);
        assert_eq!(d.avoidable_color_changes(), 2);
        assert!(d.optimize_color_changes());
        assert_eq!(threads(&d), vec![0, 0, 1, 1]);
        assert_eq!(d.avoidable_color_changes(), 0);
    }

    #[test]
    fn optimize_keeps_overlapping_layers() {
        // Red at 0, blue at 10, red at 10 (on top of the blue one).
        let (mut d, _) = design(&[(0, 0.0), (1, 10.0), (0, 10.5)]);
        assert!(!d.optimize_color_changes());
        assert_eq!(threads(&d), vec![0, 1, 0]);
        // group_by_color ignores layers.
        assert!(d.group_by_color());
        assert_eq!(threads(&d), vec![0, 0, 1]);
    }

    #[test]
    fn optimize_stops_at_first_overlap() {
        // Red A at 0; blue at 20; green at 40; red B at 40.5 overlaps green only.
        // Red B can not pass green, so it stays on top of green.
        let (mut d, _) = design(&[(0, 0.0), (1, 20.0), (2, 40.0), (0, 40.5)]);
        assert!(!d.optimize_color_changes());
        // Red B at 20.5 overlaps blue but not green: it passes green, stops at blue.
        let (mut d, _) = design(&[(0, 0.0), (1, 20.0), (2, 40.0), (0, 20.5)]);
        assert!(!d.optimize_color_changes());
        assert_eq!(threads(&d), vec![0, 1, 2, 0]);
        // Nothing overlaps: all three blue objects join the first blue block.
        let (mut d, _) = design(&[(1, 0.0), (0, 20.0), (1, 40.0), (2, 60.0), (1, 80.0)]);
        assert!(d.optimize_color_changes());
        assert_eq!(threads(&d), vec![1, 1, 1, 0, 2]);
        // Blue B overlaps green, so it can not reach blue A below green. It stays on top.
        let (mut d, _) = design(&[(1, 0.0), (2, 60.0), (0, 20.0), (1, 60.5)]);
        assert!(!d.optimize_color_changes());
        assert_eq!(threads(&d), vec![1, 2, 0, 1]);
    }

    #[test]
    fn operations_work_through_document_undo() {
        let (d, id) = design(&[(0, 0.0), (1, 2.0), (0, 4.0)]);
        let mut doc = crate::Document::new(d);
        doc.edit("Move", |d| d.move_objects_to_start(&[id[2]]));
        assert_eq!(order(doc.design()), vec![3, 1, 2]);
        doc.undo();
        assert_eq!(order(doc.design()), vec![1, 2, 3]);
    }
}
