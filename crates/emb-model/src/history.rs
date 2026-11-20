//! Undo and redo history. See ADR 0004.
//!
//! A [`Document`] owns a [`Design`]. The only way to change the design is
//! [`Document::edit`] (or its variants), so each change is in the history.
//! Each history entry is a full snapshot of the design. Snapshots are cheap
//! because objects are shared with `Arc` (see [`Design::object_mut`]).
//!
//! A change that puts the design out of the range of a project file is not
//! made (ADR 0139): the design stays as it was, the history gets no step,
//! and [`Document::take_refused`] says why. So a design in a document is
//! always one that the program can save and open again.

use crate::Design;

static NEXT_STATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// State ids are unique in the process, also across documents, so a cache
/// keyed by `Document::state_id` cannot mix up two documents.
fn fresh_state() -> u64 {
    NEXT_STATE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Default maximum number of undo steps.
pub const DEFAULT_UNDO_LIMIT: usize = 200;

/// Identifies a continuous edit, for example a slider drag on one parameter
/// of one object. Edits with the same key that follow each other become one undo step.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MergeKey(pub String);

impl MergeKey {
    pub fn new(key: impl Into<String>) -> Self {
        MergeKey(key.into())
    }
}

#[derive(Debug, Clone)]
struct Entry {
    label: String,
    design: Design,
    state: u64,
}

/// A design with its undo and redo history.
#[derive(Debug, Clone)]
pub struct Document {
    design: Design,
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    limit: usize,
    merge_key: Option<MergeKey>,
    /// Identifies the current design state. Each edit gives a new value.
    state: u64,
    saved_state: Option<u64>,
    /// Why the last change was not made (`take_refused`).
    refused: Option<String>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(Design::default())
    }
}

impl Document {
    /// Makes a document with an empty history. The design counts as saved.
    pub fn new(design: Design) -> Self {
        let state = fresh_state();
        Self {
            design,
            undo: Vec::new(),
            redo: Vec::new(),
            limit: DEFAULT_UNDO_LIMIT,
            merge_key: None,
            state,
            saved_state: Some(state),
            refused: None,
        }
    }

    pub fn design(&self) -> &Design {
        &self.design
    }

    /// Sets the maximum number of undo steps. The oldest steps are removed first.
    #[cfg(test)]
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        self.trim();
    }

    /// Applies a change as one undo step with the given label. A change
    /// that puts the design out of range is not made (`take_refused`).
    pub fn edit<R>(&mut self, label: impl Into<String>, f: impl FnOnce(&mut Design) -> R) -> R {
        self.merge_key = None;
        let before = self.design.clone();
        let r = f(&mut self.design);
        if let Some(before) = self.settle(before) {
            self.push_snapshot(label.into(), before);
        }
        r
    }

    /// The reason why the last change was not made, one time: the change
    /// put the hoop, an object or the product more than 1 km from the origin
    /// (`project::out_of_range`), or gave it a number that is not a real
    /// number. The design did not change, and the history has no new step.
    pub fn take_refused(&mut self) -> Option<String> {
        self.refused.take()
    }

    /// Ends a change of the design: makes the parts that follow the objects
    /// again. Returns the design of before, for the history. If the change
    /// put the design out of range, the design of before comes back, and
    /// the result is `None`.
    fn settle(&mut self, before: Design) -> Option<Design> {
        self.design.settle_knockouts();
        self.design.settle_hoop();
        match crate::project::out_of_range(&self.design, Some(&before)) {
            None => Some(before),
            Some(why) => {
                self.design = before;
                self.refused = Some(why);
                None
            }
        }
    }

    /// Applies a change that can fail. If `f` returns `Err`, the design does
    /// not change and the history does not get a new step.
    pub fn try_edit<R, E>(
        &mut self,
        label: impl Into<String>,
        f: impl FnOnce(&mut Design) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut work = self.design.clone();
        let r = f(&mut work)?;
        work.settle_knockouts();
        work.settle_hoop();
        if let Some(why) = crate::project::out_of_range(&work, Some(&self.design)) {
            self.refused = Some(why);
            return Ok(r);
        }
        self.merge_key = None;
        let before = std::mem::replace(&mut self.design, work);
        self.push_snapshot(label.into(), before);
        Ok(r)
    }

    /// Applies a change that merges with the previous edit if that edit had
    /// the same merge key. Use this for slider drags and node drags, then call
    /// [`Document::end_merge`] when the drag stops.
    pub fn edit_merged<R>(&mut self, label: impl Into<String>, key: MergeKey, f: impl FnOnce(&mut Design) -> R) -> R {
        let before = self.design.clone();
        let r = f(&mut self.design);
        let Some(before) = self.settle(before) else {
            return r;
        };
        if self.merge_key.as_ref() != Some(&key) || self.undo.is_empty() {
            self.push_snapshot(label.into(), before);
            self.merge_key = Some(key);
        } else {
            // Same continuous edit: keep the snapshot from before the first part.
            self.state = self.new_state();
        }
        r
    }

    /// Stops merging. The next `edit_merged` call makes a new undo step.
    pub fn end_merge(&mut self) {
        self.merge_key = None;
    }

    /// `true` while the edits with `key` make one undo step: the last undo
    /// step is theirs.
    pub fn is_merging(&self, key: &MergeKey) -> bool {
        self.merge_key.as_ref() == Some(key)
    }

    /// Stops merging only if the current merge has `key`. A panel uses this,
    /// so that it does not split a drag on the canvas into many steps.
    pub fn end_merge_of(&mut self, key: &MergeKey) {
        if self.merge_key.as_ref() == Some(key) {
            self.merge_key = None;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Label of the step that `undo` reverts.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    /// Label of the step that `redo` applies again.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }

    /// Reverts the last step. Returns its label, or `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        let entry = self.undo.pop()?;
        self.merge_key = None;
        let current = Entry {
            label: entry.label.clone(),
            design: std::mem::replace(&mut self.design, entry.design),
            state: self.state,
        };
        self.state = entry.state;
        self.redo.push(current);
        Some(entry.label)
    }

    /// Applies the last undone step again. Returns its label, or `None` if there is nothing to redo.
    pub fn redo(&mut self) -> Option<String> {
        let entry = self.redo.pop()?;
        self.merge_key = None;
        let current = Entry {
            label: entry.label.clone(),
            design: std::mem::replace(&mut self.design, entry.design),
            state: self.state,
        };
        self.state = entry.state;
        self.undo.push(current);
        Some(entry.label)
    }

    /// Records that the current state is saved to disk.
    pub fn mark_saved(&mut self) {
        self.saved_state = Some(self.state);
    }

    /// Records that no saved file has the current state (a design from a
    /// backup, for example).
    pub fn mark_unsaved(&mut self) {
        self.saved_state = None;
    }

    /// Returns `true` if the design is different from the last saved state.
    /// Undo back to the saved state makes this `false` again.
    pub fn is_modified(&self) -> bool {
        self.saved_state != Some(self.state)
    }

    /// Identifies the current design state. It changes on each edit, undo
    /// and redo, and comes back with undo. Use it to know when to rebuild
    /// data that depends on the design (for example the stitch plan).
    pub fn state_id(&self) -> u64 {
        self.state
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    fn new_state(&mut self) -> u64 {
        fresh_state()
    }

    /// Adds the design of before a change (`before`) to the history.
    fn push_snapshot(&mut self, label: String, before: Design) {
        let entry = Entry {
            label,
            design: before,
            state: self.state,
        };
        self.undo.push(entry);
        self.redo.clear();
        self.state = self.new_state();
        self.trim();
    }

    fn trim(&mut self) {
        if self.undo.len() > self.limit {
            let extra = self.undo.len() - self.limit;
            self.undo.drain(..extra);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::tests::line;
    use crate::{ObjectKind, ThreadId};

    fn doc_with_object() -> (Document, crate::ObjectId) {
        let mut doc = Document::default();
        let id = doc.edit("Add line", |d| d.add_object(ThreadId(0), line(0.0, 10.0)));
        (doc, id)
    }

    #[test]
    fn a_change_out_of_range_is_not_made() {
        use emb_geom::Transform;
        let (mut doc, id) = doc_with_object();
        let (state, steps) = (doc.state_id(), doc.undo_len());
        let far = Transform::translate(2.0 * crate::project::MAX_COORD_MM, 0.0);
        // One step, a step of a drag, and a change that can fail.
        assert!(doc.edit("Move", |d| d.transform_objects(&[id], &far)));
        assert!(doc.take_refused().is_some_and(|w| w.contains("out of range")));
        assert_eq!(doc.take_refused(), None, "the reason is given one time");
        doc.edit_merged("Drag", MergeKey::new("drag"), |d| d.transform_objects(&[id], &far));
        assert!(doc.take_refused().is_some());
        let r: Result<bool, ()> = doc.try_edit("Move", |d| Ok(d.transform_objects(&[id], &far)));
        assert_eq!(r, Ok(true));
        assert!(doc.take_refused().is_some());
        // A number that is not a real number.
        doc.edit("Move", |d| {
            d.transform_objects(&[id], &Transform::translate(f64::NAN, 0.0))
        });
        assert!(doc.take_refused().is_some());
        assert_eq!(
            (doc.state_id(), doc.undo_len()),
            (state, steps),
            "no step and the same design"
        );
        assert_eq!(crate::project::out_of_range(doc.design(), None), None);
        // The next change in range is made.
        doc.edit("Move", |d| d.transform_objects(&[id], &Transform::translate(5.0, 0.0)));
        assert_eq!(doc.take_refused(), None);
        assert_eq!(doc.undo_len(), steps + 1);
        // A drag that goes out of range and comes back is one step.
        let key = MergeKey::new("drag");
        doc.edit_merged("Drag", key.clone(), |d| {
            d.transform_objects(&[id], &Transform::translate(1.0, 0.0))
        });
        doc.edit_merged("Drag", key.clone(), |d| d.transform_objects(&[id], &far));
        doc.edit_merged("Drag", key, |d| {
            d.transform_objects(&[id], &Transform::translate(1.0, 0.0))
        });
        assert_eq!(doc.undo_len(), steps + 2);
    }

    #[test]
    fn a_refused_change_keeps_the_redo_steps() {
        use emb_geom::Transform;
        let (mut doc, id) = doc_with_object();
        doc.edit("Move", |d| d.transform_objects(&[id], &Transform::translate(5.0, 0.0)));
        doc.undo();
        assert!(doc.can_redo());
        let far = Transform::translate(0.0, -2.0 * crate::project::MAX_COORD_MM);
        doc.edit("Far", |d| d.transform_objects(&[id], &far));
        assert!(doc.take_refused().is_some());
        assert!(
            doc.can_redo(),
            "a change that was not made does not clear the redo steps"
        );
    }

    fn stitch_len(doc: &Document, id: crate::ObjectId) -> f64 {
        match &doc.design().object(id).unwrap().kind {
            ObjectKind::Running { params, .. } => params.stitch_length_mm,
            _ => unreachable!(),
        }
    }

    fn set_stitch_len(d: &mut Design, id: crate::ObjectId, v: f64) {
        if let ObjectKind::Running { params, .. } = &mut d.object_mut(id).unwrap().kind {
            params.stitch_length_mm = v;
        }
    }

    #[test]
    fn mark_unsaved() {
        let mut d = Document::new(Design::new("t"));
        assert!(!d.is_modified());
        d.mark_unsaved();
        assert!(d.is_modified());
        d.mark_saved();
        assert!(!d.is_modified());
    }

    #[test]
    fn undo_redo_round_trip() {
        let (mut doc, id) = doc_with_object();
        doc.edit("Rename", |d| d.object_mut(id).unwrap().name = Some("a".into()));
        assert_eq!(doc.undo_label(), Some("Rename"));

        assert_eq!(doc.undo().as_deref(), Some("Rename"));
        assert_eq!(doc.design().object(id).unwrap().name, None);
        assert_eq!(doc.redo().as_deref(), Some("Rename"));
        assert_eq!(doc.design().object(id).unwrap().name.as_deref(), Some("a"));

        doc.undo();
        doc.undo();
        assert!(doc.design().objects().is_empty());
        assert!(!doc.can_undo());
        assert_eq!(doc.undo(), None);
    }

    #[test]
    fn new_edit_clears_redo() {
        let (mut doc, _) = doc_with_object();
        doc.undo();
        assert!(doc.can_redo());
        doc.edit("Rename design", |d| d.name = "x".into());
        assert!(!doc.can_redo());
    }

    #[test]
    fn undo_restores_old_revision_for_cache_hits() {
        let (mut doc, id) = doc_with_object();
        let rev_before = doc.design().object(id).unwrap().revision;
        doc.edit("Length", |d| set_stitch_len(d, id, 3.0));
        assert_ne!(doc.design().object(id).unwrap().revision, rev_before);
        doc.undo();
        assert_eq!(doc.design().object(id).unwrap().revision, rev_before);
    }

    #[test]
    fn merged_edits_are_one_step() {
        let (mut doc, id) = doc_with_object();
        let key = MergeKey::new("len");
        for v in [2.6, 2.7, 2.8, 3.0] {
            doc.edit_merged("Stitch length", key.clone(), |d| set_stitch_len(d, id, v));
        }
        doc.end_merge();
        doc.edit_merged("Stitch length", key, |d| set_stitch_len(d, id, 4.0));

        assert_eq!(stitch_len(&doc, id), 4.0);
        doc.undo();
        assert_eq!(stitch_len(&doc, id), 3.0);
        doc.undo();
        assert_eq!(stitch_len(&doc, id), 2.5);
        assert_eq!(doc.undo_label(), Some("Add line"));
    }

    #[test]
    fn try_edit_failure_changes_nothing() {
        let (mut doc, id) = doc_with_object();
        let before = doc.undo_len();
        let r: Result<(), &str> = doc.try_edit("Bad", |d| {
            d.object_mut(id).unwrap().name = Some("half done".into());
            Err("rejected")
        });
        assert!(r.is_err());
        assert_eq!(doc.undo_len(), before);
        assert_eq!(doc.design().object(id).unwrap().name, None);
    }

    #[test]
    fn modified_flag_follows_saved_state() {
        let (mut doc, id) = doc_with_object();
        assert!(doc.is_modified());
        doc.mark_saved();
        assert!(!doc.is_modified());
        doc.edit("Rename", |d| d.object_mut(id).unwrap().name = Some("a".into()));
        assert!(doc.is_modified());
        doc.undo();
        assert!(!doc.is_modified());
        doc.redo();
        assert!(doc.is_modified());
    }

    #[test]
    fn modified_after_merged_edit_continues() {
        let (mut doc, id) = doc_with_object();
        let key = MergeKey::new("len");
        doc.edit_merged("Len", key.clone(), |d| set_stitch_len(d, id, 3.0));
        doc.mark_saved();
        doc.edit_merged("Len", key, |d| set_stitch_len(d, id, 3.5));
        assert!(doc.is_modified());
    }

    #[test]
    fn limit_drops_oldest() {
        let mut doc = Document::default();
        doc.set_limit(3);
        for i in 0..5 {
            doc.edit(format!("step {i}"), |d| d.name = format!("{i}"));
        }
        assert_eq!(doc.undo_len(), 3);
        while doc.undo().is_some() {}
        assert_eq!(doc.design().name, "1");
    }
}
