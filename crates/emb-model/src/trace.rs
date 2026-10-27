//! Border trace (P6 Milestone 4, owner request of 2026-10-22; ADR 0025).
//!
//! One command on the selected objects: a border round them in a thread
//! that the user chooses, a gap round that (or round the objects), or
//! both, cut out of the objects that sew before them (the objects under
//! them), so no area sews twice. In a gap the fabric shows. The border is
//! a column of a column type (satin, zigzag, E stitch and the others), or
//! a run or triple run. It sews after the objects (fill then border) or
//! before them (border then fill). The traced objects and the border are
//! one group, so they move together. The border sits outside the edge of
//! the objects (a column goes a little over the edge), centered on the
//! edge, or inside it ([`BorderPlace`], owner request of 2026-10-26).
//!
//! A record (`Design::border_traces`) keeps the objects, the border and
//! each cut object as it was before the cut. A change of the settings, or
//! a move of the objects or of a cut piece, puts the cut objects back and
//! cuts them again, so the cut always fits (see `link`).

use std::collections::BTreeMap;

use emb_geom::region::{self, Join};
use serde::{Deserialize, Serialize};

use crate::shaping::{ShapeError, area_of, even_odd, takes_part};
use crate::{
    BorderPlace, ColumnPattern, Design, DesignObject, LineBorder, ObjectId, ObjectKind, RunningParams, ThreadId,
};

/// A column border goes this far over the edge of the objects, so no
/// fabric shows between them (pull of the fill), in mm.
pub const BORDER_INSET_MM: f64 = 0.4;
/// With no gap, the cut objects go this far under a column border, in mm.
pub const UNDER_BORDER_MM: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOrder {
    /// The objects sew first, then their border.
    #[default]
    FillThenBorder,
    /// The border sews first, then the objects.
    BorderThenFill,
}

/// The stitch of a border.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceStitch {
    /// A column of this type (a line with a width).
    Column { pattern: ColumnPattern },
    /// A running line in the middle of the border width.
    Run,
    /// A triple (bean) run in the middle of the border width.
    TripleRun,
}

impl Default for TraceStitch {
    fn default() -> Self {
        TraceStitch::Column {
            pattern: ColumnPattern::Satin,
        }
    }
}

impl TraceStitch {
    pub fn name(&self) -> &'static str {
        match self {
            TraceStitch::Column { pattern } => pattern.name(),
            TraceStitch::Run => "Run",
            TraceStitch::TripleRun => "Triple run",
        }
    }

    /// All choices: each column type, run and triple run.
    pub fn all() -> Vec<TraceStitch> {
        ColumnPattern::ALL
            .iter()
            .map(|p| TraceStitch::Column { pattern: *p })
            .chain([TraceStitch::Run, TraceStitch::TripleRun])
            .collect()
    }
}

/// The settings of a border trace. A border, a gap, or both.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "TraceSpecFile")]
pub struct TraceSpec {
    /// The width of the border; 0: no border.
    pub border_mm: f64,
    /// The width of the gap round the border (or round the objects with no
    /// border); 0: no gap.
    pub gap_mm: f64,
    #[serde(default)]
    pub stitch: TraceStitch,
    /// The thread of the border.
    pub thread: ThreadId,
    #[serde(default)]
    pub order: TraceOrder,
    /// Where the border sits on the edge of the objects.
    #[serde(default, skip_serializing_if = "BorderPlace::is_outside")]
    pub place: BorderPlace,
}

/// The fields of a trace in a file, also in the first form of
/// 2026-10-23 (`mode` border or gap, and one `width_mm`).
#[derive(Deserialize)]
struct TraceSpecFile {
    #[serde(default)]
    border_mm: Option<f64>,
    #[serde(default)]
    gap_mm: Option<f64>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    width_mm: Option<f64>,
    #[serde(default)]
    stitch: TraceStitch,
    thread: ThreadId,
    #[serde(default)]
    order: TraceOrder,
    #[serde(default)]
    place: BorderPlace,
}

impl From<TraceSpecFile> for TraceSpec {
    fn from(f: TraceSpecFile) -> TraceSpec {
        let w = f.width_mm.unwrap_or(0.0);
        let gap_mode = f.mode.as_deref() == Some("gap");
        TraceSpec {
            border_mm: f.border_mm.unwrap_or(if gap_mode { 0.0 } else { w }),
            gap_mm: f.gap_mm.unwrap_or(if gap_mode { w } else { 0.0 }),
            stitch: f.stitch,
            thread: f.thread,
            order: f.order,
            place: f.place,
        }
    }
}

/// An object under the traced objects (or of a shape link), as it was
/// before the cut, and the pieces that the cut made of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCut {
    pub original: DesignObject,
    pub pieces: Vec<ObjectId>,
}

impl PartialEq for TraceCut {
    fn eq(&self, o: &TraceCut) -> bool {
        self.original.same_content(&o.original) && self.pieces == o.pieces
    }
}

/// A border trace in a design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BorderTrace {
    pub id: u32,
    /// The traced objects.
    pub objects: Vec<ObjectId>,
    /// The border object (none with no border).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<ObjectId>,
    pub spec: TraceSpec,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<TraceCut>,
}

impl BorderTrace {
    /// Returns `true` if object `id` is part of the trace: a traced object,
    /// the border, or a cut piece.
    pub fn has(&self, id: ObjectId) -> bool {
        self.objects.contains(&id) || self.border == Some(id) || self.cuts.iter().any(|c| c.pieces.contains(&id))
    }
}

impl Design {
    /// The border trace that object `id` is part of.
    pub fn trace_of(&self, id: ObjectId) -> Option<&BorderTrace> {
        self.border_traces.iter().find(|t| t.has(id))
    }

    /// Adds a border trace round the objects `ids`. Returns its id.
    pub fn add_border_trace(&mut self, ids: &[ObjectId], spec: TraceSpec) -> Result<u32, ShapeError> {
        let objects: Vec<ObjectId> = ids.iter().copied().filter(|id| self.object(*id).is_some()).collect();
        if objects.iter().any(|id| self.is_linked(*id)) {
            return Err(ShapeError::AlreadyLinked);
        }
        let id = self.border_traces.iter().map(|t| t.id + 1).max().unwrap_or(1);
        let mut trace = BorderTrace {
            id,
            objects,
            border: None,
            spec,
            cuts: Vec::new(),
        };
        self.apply_trace(&mut trace, &BTreeMap::new(), None)?;
        // The objects and their border move together.
        let group = trace
            .objects
            .iter()
            .find_map(|id| self.object(*id).and_then(|o| o.group))
            .unwrap_or_else(|| self.new_group_id());
        for oid in trace.objects.iter().chain(&trace.border) {
            if let Some(o) = self.object_mut(*oid) {
                o.group = Some(group);
            }
        }
        self.border_traces.push(trace);
        Ok(id)
    }

    /// Changes the settings of border trace `id`, or makes it again after
    /// a move: the cut objects come back and are cut again, and the border
    /// is made again. The border and the pieces keep their ids.
    pub fn update_border_trace(&mut self, id: u32, spec: TraceSpec) -> Result<(), ShapeError> {
        let Some(i) = self.border_traces.iter().position(|t| t.id == id) else {
            return Err(ShapeError::NoOutline);
        };
        let mut trace = self.border_traces.remove(i);
        let old = trace.clone();
        let group = trace.border.and_then(|b| self.object(b)).and_then(|o| o.group);
        let border = trace.border.take();
        if let Some(b) = border {
            self.remove_object(b);
        }
        let reuse = self.restore_cuts(std::mem::take(&mut trace.cuts));
        trace.spec = spec;
        if let Err(e) = self.apply_trace(&mut trace, &reuse, border) {
            // Put the old trace back as it was.
            let mut back = old;
            back.border = None;
            back.cuts.clear();
            let _ = self.apply_trace(&mut back, &reuse, border);
            self.border_traces.insert(i, back);
            return Err(e);
        }
        if let Some(o) = trace.border.and_then(|b| self.object_mut(b)) {
            o.group = group;
        }
        self.border_traces.insert(i, trace);
        Ok(())
    }

    /// Removes border trace `id`: the border goes and the cut objects come
    /// back. The traced objects stay.
    pub fn remove_border_trace(&mut self, id: u32) -> bool {
        let Some(i) = self.border_traces.iter().position(|t| t.id == id) else {
            return false;
        };
        let mut trace = self.border_traces.remove(i);
        if let Some(b) = trace.border.take() {
            self.remove_object(b);
        }
        self.restore_cuts(std::mem::take(&mut trace.cuts));
        true
    }

    /// The area of the traced objects (lettering by its letters).
    fn traced_area(&self, ids: &[ObjectId]) -> Vec<region::Region> {
        ids.iter()
            .filter_map(|id| self.object(*id))
            .filter_map(|o| match &o.kind {
                ObjectKind::Lettering(_) => Some(even_odd(&crate::vector::area_outline(&o.kind))),
                k => area_of(k),
            })
            .fold(Vec::new(), |acc, a| region::union(&acc, &a))
    }

    /// Makes the border and the cut of `trace` (which has none). The pieces
    /// take the ids of `reuse` for their original, the border `border_id`.
    fn apply_trace(
        &mut self,
        trace: &mut BorderTrace,
        reuse: &BTreeMap<ObjectId, Vec<ObjectId>>,
        border_id: Option<ObjectId>,
    ) -> Result<(), ShapeError> {
        let spec = trace.spec;
        let (w, g) = (spec.border_mm.max(0.0), spec.gap_mm.max(0.0));
        if w <= 0.0 && g <= 0.0 {
            return Err(ShapeError::NothingToDo);
        }
        let area = self.traced_area(&trace.objects);
        if area.is_empty() {
            return Err(ShapeError::NoOutline);
        }
        let column = matches!(spec.stitch, TraceStitch::Column { .. });
        // The outer edge of the border, out from the edge of the objects
        // (outside, a column goes a little over the edge of the objects),
        // and the cut: the gap, or with no gap a little less, so the cut
        // objects go under a column border.
        let outer = match (w > 0.0, spec.place, column) {
            (false, ..) => 0.0,
            (true, BorderPlace::Outside, true) => w - BORDER_INSET_MM,
            (true, BorderPlace::Outside, false) => w,
            (true, BorderPlace::Center, _) => w / 2.0,
            (true, BorderPlace::Inside, _) => 0.0,
        };
        let cut = if g > 0.0 {
            outer + g
        } else if column {
            outer - UNDER_BORDER_MM
        } else {
            outer
        };
        let cutter = region::offset(&area, cut, Join::Round);
        let first = trace
            .objects
            .iter()
            .filter_map(|id| self.index_of(*id))
            .min()
            .ok_or(ShapeError::NoOutline)?;
        let under: Vec<DesignObject> = self.objects[..first]
            .iter()
            .filter(|o| !o.locked && takes_part(&o.kind) && !self.is_linked(o.id))
            .map(|o| (**o).clone())
            .collect();
        let empty = Vec::new();
        for o in under {
            let Some(a) = area_of(&o.kind) else { continue };
            // An object that was cut before is cut again (and keeps its
            // pieces) even if it does not overlap now.
            let was_cut = reuse.contains_key(&o.id);
            if !was_cut && region::intersection(&a, &cutter).iter().all(|r| r.area() < 0.05) {
                continue;
            }
            let rest = crate::shaping::clean(region::difference(&a, &cutter));
            let kinds = crate::shaping::rebuild(&o.kind, &rest, &self.current.fill);
            let pieces = self.replace_with_kinds(&o, kinds, reuse.get(&o.id).unwrap_or(&empty));
            trace.cuts.push(TraceCut { original: o, pieces });
        }
        if w > 0.0 {
            let middle = outer - w / 2.0;
            let path = crate::arrange::outline_path(&region::offset(&area, middle, Join::Round));
            if !path.is_empty() {
                let params = match spec.stitch {
                    TraceStitch::Column { pattern } => RunningParams {
                        border: Some(LineBorder {
                            width_mm: w,
                            column: crate::SatinParams {
                                pattern,
                                ..self.current.satin.clone()
                            },
                            corner: Default::default(),
                        }),
                        ..self.current.running.clone()
                    },
                    TraceStitch::Run => RunningParams {
                        repeats: 1,
                        ..self.current.running.clone()
                    },
                    TraceStitch::TripleRun => RunningParams {
                        repeats: 3,
                        ..self.current.running.clone()
                    },
                };
                let kind = ObjectKind::Running { path, params };
                let indices: Vec<usize> = trace.objects.iter().filter_map(|id| self.index_of(*id)).collect();
                let at = match spec.order {
                    TraceOrder::FillThenBorder => indices.iter().max().map_or(self.objects.len(), |i| i + 1),
                    TraceOrder::BorderThenFill => indices.iter().min().copied().unwrap_or(0),
                };
                let id = border_id
                    .filter(|b| self.object(*b).is_none())
                    .unwrap_or_else(|| self.next_object_id());
                let template = DesignObject {
                    id,
                    revision: crate::object::fresh_revision(),
                    name: None,
                    thread: spec.thread,
                    start: None,
                    end: None,
                    group: None,
                    locked: false,
                    functions: Default::default(),
                    kind,
                };
                self.objects
                    .insert(at.min(self.objects.len()), std::sync::Arc::new(template));
                trace.border = Some(id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point, Transform};

    use super::*;
    use crate::{FillParams, Rgb};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square(x: f64, y: f64, s: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(x, y), p(x + s, y + s))),
            holes: Vec::new(),
            params: FillParams::default(),
        }
    }

    /// A large square under a small square.
    fn design() -> (Design, ObjectId, ObjectId, ThreadId) {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let black = d.add_thread("Black", Rgb(0, 0, 0));
        let under = d.add_object(red, square(0.0, 0.0, 40.0));
        let top = d.add_object(red, square(10.0, 10.0, 10.0));
        (d, under, top, black)
    }

    fn area(d: &Design, id: ObjectId) -> f64 {
        area_of(&d.object(id).unwrap().kind)
            .unwrap()
            .iter()
            .map(|r| r.area())
            .sum()
    }

    /// The area of a square of side `s` grown by `r` with round corners.
    fn rounded_square(s: f64, r: f64) -> f64 {
        let side = s + 2.0 * r;
        side * side - (4.0 - std::f64::consts::PI) * r * r
    }

    fn spec(border: f64, gap: f64, thread: ThreadId, order: TraceOrder) -> TraceSpec {
        TraceSpec {
            border_mm: border,
            gap_mm: gap,
            stitch: TraceStitch::default(),
            thread,
            order,
            place: BorderPlace::Outside,
        }
    }

    /// The owner request of 2026-10-26: the border outside the edge of
    /// the objects, centered on it, or inside it.
    #[test]
    fn the_border_sits_outside_centered_or_inside_the_edge() {
        let (mut d, _, top, black) = design();
        let t = d
            .add_border_trace(&[top], spec(4.0, 0.0, black, TraceOrder::FillThenBorder))
            .unwrap();
        // (place, the middle of the border out from the 10 mm square, the
        // cut out from it)
        for (place, middle, cut) in [
            (
                BorderPlace::Outside,
                2.0 - BORDER_INSET_MM,
                4.0 - BORDER_INSET_MM - UNDER_BORDER_MM,
            ),
            (BorderPlace::Center, 0.0, 2.0 - UNDER_BORDER_MM),
            (BorderPlace::Inside, -2.0, -UNDER_BORDER_MM),
        ] {
            let s = TraceSpec {
                place,
                ..d.border_traces[0].spec
            };
            d.update_border_trace(t, s).unwrap();
            let trace = &d.border_traces[0];
            let b = d.object(trace.border.unwrap()).unwrap().control_bbox().unwrap();
            assert!((b.width() - (10.0 + 2.0 * middle)).abs() < 0.01, "{place:?}: {b:?}");
            let piece = trace.cuts[0].pieces[0];
            let hole = if cut > 0.0 {
                rounded_square(10.0, cut)
            } else {
                (10.0 + 2.0 * cut).powi(2)
            };
            assert!(
                (area(&d, piece) - (1600.0 - hole)).abs() < 0.5,
                "{place:?}: {} {}",
                area(&d, piece),
                1600.0 - hole
            );
        }
        // An inside border keeps its place in the project file; the
        // outside border (the first form) writes no field.
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert_eq!(back.border_traces[0].spec.place, BorderPlace::Inside);
        let s = TraceSpec {
            place: BorderPlace::Outside,
            ..d.border_traces[0].spec
        };
        d.update_border_trace(t, s).unwrap();
        assert!(!crate::project::to_string(&d).contains("\"place\""));
    }

    #[test]
    fn border_and_cut_in_both_sew_orders() {
        let (mut d, _, top, black) = design();
        let t = d
            .add_border_trace(&[top], spec(3.0, 0.0, black, TraceOrder::FillThenBorder))
            .unwrap();
        let trace = d.border_traces[0].clone();
        assert_eq!(trace.id, t);
        let border = trace.border.unwrap();
        assert_eq!(d.object(border).unwrap().thread, black);
        assert_eq!(
            d.index_of(border),
            Some(d.index_of(top).unwrap() + 1),
            "fill then border"
        );
        // One group: the object and its border move together.
        assert!(d.object(top).unwrap().group.is_some());
        assert_eq!(d.object(top).unwrap().group, d.object(border).unwrap().group);
        let piece = trace.cuts[0].pieces[0];
        let want = 1600.0 - rounded_square(10.0, 3.0 - BORDER_INSET_MM - UNDER_BORDER_MM);
        assert!((area(&d, piece) - want).abs() < 0.5);

        d.update_border_trace(t, spec(3.0, 0.0, black, TraceOrder::BorderThenFill))
            .unwrap();
        assert_eq!(d.border_traces[0].border, Some(border), "the border keeps its id");
        assert_eq!(
            d.index_of(border).unwrap() + 1,
            d.index_of(top).unwrap(),
            "border then fill"
        );
        assert_eq!(d.objects().len(), 3);
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert!(back.same_content(&d));
    }

    #[test]
    fn border_and_gap_together_and_other_stitches() {
        let (mut d, _, top, black) = design();
        d.add_border_trace(&[top], spec(3.0, 2.0, black, TraceOrder::FillThenBorder))
            .unwrap();
        let piece = d.border_traces[0].cuts[0].pieces[0];
        // The gap is round the border: the cut is the border and the gap.
        let want = 1600.0 - rounded_square(10.0, 3.0 - BORDER_INSET_MM + 2.0);
        assert!((area(&d, piece) - want).abs() < 0.5, "{} {want}", area(&d, piece));
        let t = d.border_traces[0].id;
        let s = TraceSpec {
            stitch: TraceStitch::TripleRun,
            ..d.border_traces[0].spec
        };
        d.update_border_trace(t, s).unwrap();
        let border = d.border_traces[0].border.unwrap();
        let ObjectKind::Running { params, .. } = &d.object(border).unwrap().kind else {
            panic!()
        };
        assert!(params.border.is_none() && params.repeats == 3);
        assert_eq!(
            d.update_border_trace(
                t,
                TraceSpec {
                    border_mm: 0.0,
                    gap_mm: 0.0,
                    ..s
                }
            ),
            Err(ShapeError::NothingToDo)
        );
        assert_eq!(d.border_traces[0].spec, s, "a failed change keeps the trace");
    }

    #[test]
    fn gap_only_updates_moves_and_removes() {
        let (mut d, under, top, black) = design();
        let start = d.clone();
        let t = d
            .add_border_trace(&[top], spec(0.0, 2.0, black, TraceOrder::FillThenBorder))
            .unwrap();
        assert!(d.border_traces[0].border.is_none());
        let piece = d.border_traces[0].cuts[0].pieces[0];
        let a = area(&d, piece);
        assert!((a - (1600.0 - rounded_square(10.0, 2.0))).abs() < 0.5);
        // Moving the object out of the square: the square is whole again,
        // and the piece keeps its id.
        d.transform_objects(&[top], &Transform::translate(100.0, 0.0));
        assert!((area(&d, piece) - 1600.0).abs() < 0.5, "{}", area(&d, piece));
        d.transform_objects(&[top], &Transform::translate(-100.0, 0.0));
        assert!((area(&d, piece) - a).abs() < 0.01);
        // A changed setting on the piece stays when the trace changes.
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(piece).map(|o| &mut o.kind) {
            params.angle_deg = 30.0;
        }
        assert!(d.remove_border_trace(t));
        let ObjectKind::Fill { params, .. } = &d.object(under).unwrap().kind else {
            panic!()
        };
        assert_eq!(params.angle_deg, 30.0);
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(under).map(|o| &mut o.kind) {
            params.angle_deg = FillParams::default().angle_deg;
        }
        if let Some(o) = d.object_mut(top) {
            o.group = None;
        }
        assert!(d.same_content(&start));
    }

    #[test]
    fn the_first_file_form_still_reads() {
        let old = r#"{"mode":"gap","width_mm":2.5,"thread":1,"order":"border_then_fill"}"#;
        let s: TraceSpec = serde_json::from_str(old).unwrap();
        assert_eq!((s.border_mm, s.gap_mm, s.order), (0.0, 2.5, TraceOrder::BorderThenFill));
        let old = r#"{"mode":"border","width_mm":3.0,"thread":1}"#;
        let s: TraceSpec = serde_json::from_str(old).unwrap();
        assert_eq!((s.border_mm, s.gap_mm), (3.0, 0.0));
        let new = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<TraceSpec>(&new).unwrap(), s);
    }

    #[test]
    fn errors_and_lookup() {
        let (mut d, under, top, black) = design();
        let s = spec(3.0, 0.0, black, TraceOrder::FillThenBorder);
        let t = d.add_border_trace(&[top], s).unwrap();
        assert_eq!(d.add_border_trace(&[top], s), Err(ShapeError::AlreadyLinked));
        let border = d.border_traces[0].border.unwrap();
        assert_eq!(d.trace_of(border).map(|t| t.id), Some(t));
        assert!(d.trace_of(under).is_none(), "the original is now a piece");
        let mut d = Design::new("e");
        let th = d.add_thread("Black", Rgb(0, 0, 0));
        let line = d.add_object(
            th,
            ObjectKind::Stitches {
                path: Path::new().move_to(p(0.0, 0.0)).line_to(p(5.0, 0.0)),
                trims: Vec::new(),
            },
        );
        assert_eq!(d.add_border_trace(&[line], s), Err(ShapeError::NoOutline));
    }
}
