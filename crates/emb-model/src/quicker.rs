//! Quicker to Sew (P22 Milestone 2, ADR 0169): changes that make a design
//! quicker to sew, as one step. Each part can be on or off:
//!
//! - merge the threads whose colors are almost the same: the thread that
//!   sews more stays (fewer threads and color changes);
//! - remove fills, satin columns and lines that are smaller than a size
//!   (small details that sew slowly and badly);
//! - join color blocks of the same thread where no object is in the way
//!   (`Design::optimize_color_changes`);
//! - the trim distance of the design (`Design::trim_gap_mm`): gaps up to it
//!   get a travel stitch, not a trim.
//!
//! Not a port. The idea of one window for these is from zigzag (an
//! open-source program under Apache-2.0); the code is our own.

use serde::{Deserialize, Serialize};

use crate::chart::Lab;
use crate::{Design, ObjectId, ObjectKind, ThreadId};

/// The largest trim distance of a design, mm.
pub const MAX_TRIM_GAP_MM: f64 = 50.0;

/// The settings of Quicker to Sew.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuickerToSew {
    /// Merge the threads whose colors differ by this or less (CIE76; about
    /// 2.3 is the smallest difference that a person can see).
    pub merge_threads: Option<f64>,
    /// Remove the fills, satin columns and lines whose box is smaller than
    /// this on its longer side, mm.
    pub small_details_mm: Option<f64>,
    /// Join color blocks of the same thread where no object is in the way.
    pub fewer_color_changes: bool,
    /// The trim distance of the design, mm.
    pub gap_mm: Option<f64>,
}

impl QuickerToSew {
    /// The settings when the window first opens.
    pub const STANDARD: QuickerToSew = QuickerToSew {
        merge_threads: Some(5.0),
        small_details_mm: Some(1.0),
        fewer_color_changes: true,
        gap_mm: Some(3.0),
    };
}

impl Default for QuickerToSew {
    fn default() -> Self {
        QuickerToSew::STANDARD
    }
}

/// What [`apply`] changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickerReport {
    /// Each thread that went, and the thread that took its place.
    pub merged: Vec<(ThreadId, ThreadId)>,
    /// The objects that were removed.
    pub removed: Vec<ObjectId>,
}

/// Makes `design` quicker to sew with the settings `s`. `sews(thread)` is
/// how much a thread sews (its stitches in the plan), to choose the thread
/// that stays when two merge.
pub fn apply(design: &mut Design, s: &QuickerToSew, sews: &dyn Fn(ThreadId) -> usize) -> QuickerReport {
    let mut report = QuickerReport::default();
    if let Some(max) = s.merge_threads {
        report.merged = merge_threads(design, max, sews);
    }
    if let Some(size) = s.small_details_mm {
        report.removed = remove_small(design, size);
    }
    if s.fewer_color_changes {
        design.optimize_color_changes();
    }
    design.trim_gap_mm = s.gap_mm.map(|g| g.clamp(0.0, MAX_TRIM_GAP_MM));
    report
}

/// Merges the used threads whose colors differ by `max` or less: the
/// thread that sews more stays. Returns (thread that went, thread that
/// stays).
fn merge_threads(design: &mut Design, max: f64, sews: &dyn Fn(ThreadId) -> usize) -> Vec<(ThreadId, ThreadId)> {
    let mut used: Vec<(ThreadId, Lab, usize)> = design
        .used_threads()
        .into_iter()
        .filter_map(|id| Some((id, Lab::of(design.thread(id)?.color), sews(id))))
        .collect();
    // The threads that sew most first; the same order on each run.
    used.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let mut merged: Vec<(ThreadId, ThreadId)> = Vec::new();
    for i in 0..used.len() {
        if merged.iter().any(|(gone, _)| *gone == used[i].0) {
            continue;
        }
        for j in i + 1..used.len() {
            let gone = used[j].0;
            if merged.iter().any(|(g, _)| *g == gone) {
                continue;
            }
            if used[i].1.distance(used[j].1) <= max {
                merged.push((gone, used[i].0));
            }
        }
    }
    if merged.is_empty() {
        return merged;
    }
    let ids: Vec<ObjectId> = design.objects().iter().map(|o| o.id).collect();
    for id in ids {
        let thread = design.object(id).map(|o| o.thread);
        if let Some(&(_, stays)) = merged.iter().find(|(gone, _)| Some(*gone) == thread)
            && let Some(o) = design.object_mut(id)
        {
            o.thread = stays;
        }
    }
    design.remove_unused_threads();
    merged
}

/// Removes the fills, satin columns and lines whose box is smaller than
/// `size` on its longer side. Objects in a group (monograms, blends) or in
/// a record of the design (border traces, linked shapes, sketches,
/// buttonholes, appliqué) stay.
fn remove_small(design: &mut Design, size: f64) -> Vec<ObjectId> {
    let small: Vec<ObjectId> = design
        .objects()
        .iter()
        .filter(|o| {
            matches!(
                o.kind,
                ObjectKind::Fill { .. } | ObjectKind::Satin { .. } | ObjectKind::Running { .. }
            ) && o.group.is_none()
                && !o.locked
                && o.control_bbox().is_some_and(|b| b.width().max(b.height()) < size)
        })
        .map(|o| o.id)
        .filter(|id| !in_a_record(design, *id))
        .collect();
    for id in &small {
        design.remove_object(*id);
    }
    small
}

/// A record of the design names the object.
fn in_a_record(d: &Design, id: ObjectId) -> bool {
    d.template_marks == Some(id)
        || d.buttonholes.iter().any(|b| b.boxes.contains(&id))
        || d.border_traces
            .iter()
            .any(|t| t.objects.contains(&id) || t.border == Some(id))
        || d.shape_links.iter().any(|l| l.cutters.contains(&id))
        || d.sketch_links.iter().any(|l| l.sketch == id || l.objects.contains(&id))
        || d.applique_origins
            .iter()
            .any(|a| a.applique == id || a.before.contains(&id))
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;
    use crate::{FillParams, Rgb};

    fn square(x: f64, size: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + size, size))),
            holes: Vec::new(),
            params: FillParams::default(),
        }
    }

    #[test]
    fn similar_threads_merge_into_the_one_that_sews_more() {
        let mut d = Design::new("merge");
        let red = d.add_thread("Red", Rgb(200, 30, 30));
        let red2 = d.add_thread("Red 2", Rgb(202, 32, 30));
        let blue = d.add_thread("Blue", Rgb(20, 40, 200));
        let a = d.add_object(red, square(0.0, 20.0));
        let b = d.add_object(red2, square(30.0, 20.0));
        let c = d.add_object(blue, square(60.0, 20.0));
        // Red 2 sews more: it stays.
        let sews = |t: ThreadId| if t == red2 { 900 } else { 100 };
        let s = QuickerToSew {
            merge_threads: Some(5.0),
            small_details_mm: None,
            fewer_color_changes: false,
            gap_mm: None,
        };
        let r = apply(&mut d, &s, &sews);
        assert_eq!(r.merged, vec![(red, red2)]);
        assert_eq!(d.object(a).unwrap().thread, red2);
        assert_eq!(d.object(b).unwrap().thread, red2);
        assert_eq!(d.object(c).unwrap().thread, blue);
        assert_eq!(d.threads.len(), 2, "the thread that went is removed");
        assert_eq!(d.trim_gap_mm, None);
    }

    #[test]
    fn small_objects_go_but_not_grouped_ones() {
        let mut d = Design::new("small");
        let t = d.add_thread("Red", Rgb(200, 30, 30));
        let big = d.add_object(t, square(0.0, 20.0));
        let speck = d.add_object(t, square(30.0, 0.6));
        let grouped = d.add_object(t, square(40.0, 0.6));
        d.object_mut(grouped).unwrap().group = Some(1);
        let s = QuickerToSew {
            merge_threads: None,
            small_details_mm: Some(1.0),
            fewer_color_changes: false,
            gap_mm: Some(4.0),
        };
        let r = apply(&mut d, &s, &|_| 0);
        assert_eq!(r.removed, vec![speck]);
        assert!(d.object(big).is_some() && d.object(grouped).is_some());
        assert_eq!(d.trim_gap_mm, Some(4.0));
    }

    #[test]
    fn a_trim_distance_out_of_range_is_refused() {
        let mut d = Design::new("gap");
        d.trim_gap_mm = Some(f64::NAN);
        assert!(crate::project::out_of_range(&d, None).is_some());
        d.trim_gap_mm = Some(80.0);
        assert!(crate::project::out_of_range(&d, None).is_some());
        d.trim_gap_mm = Some(4.0);
        assert!(crate::project::out_of_range(&d, None).is_none());
    }
}
