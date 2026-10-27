//! Object selection. It is view state, not part of the undo history.

use emb_model::{Design, ObjectId};

/// Selected objects in click order. The click order is used by
/// "Sequence by Click Order"; the last clicked object is the primary one.
///
/// A click on an object of a group selects the whole group
/// (`AppState::apply_groups_and_locks`). After [`Selection::enter`] (a
/// double-click), the objects of that group are selected one at a time,
/// until the selection has an object that is not in the group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    ids: Vec<ObjectId>,
    entered: Option<u32>,
}

impl Selection {
    /// Selects only object `id` of group `group`: the other objects of the
    /// group stay out of the selection.
    pub fn enter(&mut self, id: ObjectId, group: u32) {
        self.ids = vec![id];
        self.entered = Some(group);
    }

    /// The group whose objects are selected one at a time, if any.
    pub fn entered_group(&self) -> Option<u32> {
        self.entered
    }

    /// Selects groups as a whole again.
    pub fn leave_group(&mut self) {
        self.entered = None;
    }

    pub fn ids(&self) -> &[ObjectId] {
        &self.ids
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn contains(&self, id: ObjectId) -> bool {
        self.ids.contains(&id)
    }

    /// The only selected object, if exactly one is selected.
    pub fn single(&self) -> Option<ObjectId> {
        match self.ids.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }

    /// The last clicked object.
    pub fn primary(&self) -> Option<ObjectId> {
        self.ids.last().copied()
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    /// Selects only this object.
    pub fn set(&mut self, id: ObjectId) {
        self.ids = vec![id];
    }

    pub fn set_all(&mut self, ids: impl IntoIterator<Item = ObjectId>) {
        self.ids = ids.into_iter().collect();
    }

    /// Adds the object, or removes it if it is already selected (Ctrl+click).
    pub fn toggle(&mut self, id: ObjectId) {
        if let Some(i) = self.ids.iter().position(|x| *x == id) {
            self.ids.remove(i);
        } else {
            self.ids.push(id);
        }
    }

    /// Selects all objects in sew order from the primary object to `id` (Shift+click).
    pub fn extend_to(&mut self, design: &Design, id: ObjectId) {
        let anchor = self.primary().and_then(|a| design.index_of(a));
        let (Some(a), Some(b)) = (anchor, design.index_of(id)) else {
            self.set(id);
            return;
        };
        let (lo, hi) = (a.min(b), a.max(b));
        let range = design.objects()[lo..=hi].iter().map(|o| o.id);
        let anchor_id = design.objects()[a].id;
        // Keep the anchor as primary, so a second Shift+click starts from the same place.
        self.ids = range
            .filter(|x| *x != anchor_id)
            .chain(std::iter::once(anchor_id))
            .collect();
    }

    /// Removes objects that are not in the design (after undo or delete).
    pub fn retain_existing(&mut self, design: &Design) {
        self.ids.retain(|id| design.object(*id).is_some());
    }

    /// The selected ids in sew order.
    pub fn in_sew_order(&self, design: &Design) -> Vec<ObjectId> {
        design
            .objects()
            .iter()
            .map(|o| o.id)
            .filter(|id| self.contains(*id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{ObjectKind, RunningParams, ThreadId};

    use super::*;

    fn design(n: usize) -> (Design, Vec<ObjectId>) {
        let mut d = Design::new("t");
        let ids = (0..n)
            .map(|i| {
                d.add_object(
                    ThreadId(0),
                    ObjectKind::Running {
                        path: Path::new()
                            .move_to(Point::new(i as f64, 0.0))
                            .line_to(Point::new(i as f64, 1.0)),
                        params: RunningParams::default(),
                    },
                )
            })
            .collect();
        (d, ids)
    }

    #[test]
    fn toggle_and_click_order() {
        let (_, id) = design(3);
        let mut s = Selection::default();
        s.toggle(id[2]);
        s.toggle(id[0]);
        assert_eq!(s.ids(), &[id[2], id[0]]);
        assert_eq!(s.primary(), Some(id[0]));
        s.toggle(id[2]);
        assert_eq!(s.single(), Some(id[0]));
    }

    #[test]
    fn shift_range_in_sew_order() {
        let (d, id) = design(5);
        let mut s = Selection::default();
        s.set(id[3]);
        s.extend_to(&d, id[1]);
        assert_eq!(s.in_sew_order(&d), vec![id[1], id[2], id[3]]);
        assert_eq!(s.primary(), Some(id[3]));
        s.extend_to(&d, id[4]);
        assert_eq!(s.in_sew_order(&d), vec![id[3], id[4]]);
    }
}
