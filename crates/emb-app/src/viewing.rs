//! Viewing and travel (roadmap P2, Milestone 7): the stitch cursor, travel
//! through the stitches, view filters (by color, hide others), needle
//! points and function symbols.
//!
//! The stitch cursor is an index into the records of the stitch plan.
//! Stitches before the cursor show at full strength, the rest dimmed, so
//! the canvas shows how far the machine has sewn.
//!
//! Travel keys (when the Stitches view is on):
//! - `,` and `.`: one stitch back or forward. Shift: 10. Ctrl: 100. Ctrl+Shift: 1000.
//! - Page Up and Page Down: previous or next object.
//! - Ctrl+Page Up and Page Down: previous or next color.
//! - Shift+Page Up and Page Down: previous or next function (trim, color
//!   change, stop; or one kind of function, see `viewtools::FunctionTravel`).
//! - Alt+Page Up and Page Down: previous or next segment (a run between jumps).
//!
//! View > Stitch cursor shows and hides the cursor.

use std::collections::BTreeSet;

use eframe::egui::{self, Color32, InputState, Key, Modifiers, Pos2, Rect, Stroke};
use emb_geom::Point;
use emb_model::{DesignObject, ObjectId, StitchKind, ThreadId};
use emb_render::{Segment, SegmentKind};

use crate::app::AppState;

pub const KEYS_HELP: &str =
    ", .: stitch (Shift 10, Ctrl 100) · PgUp/PgDn: object (Ctrl: color, Shift: function, Alt: segment)";

/// Which objects the canvas shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ViewFilter {
    /// Show only objects with these threads ("view by color").
    pub threads: Option<BTreeSet<ThreadId>>,
    /// Show only these objects ("hide others").
    pub objects: Option<BTreeSet<ObjectId>>,
    /// Do not show these objects ("hide selected", Alt+V). They still sew.
    pub hidden: BTreeSet<ObjectId>,
}

impl ViewFilter {
    pub fn shows(&self, o: &DesignObject) -> bool {
        self.threads.as_ref().is_none_or(|t| t.contains(&o.thread))
            && self.objects.as_ref().is_none_or(|ids| ids.contains(&o.id))
            && !self.hidden.contains(&o.id)
    }

    pub fn is_active(&self) -> bool {
        self.threads.is_some() || self.objects.is_some() || !self.hidden.is_empty()
    }
}

/// A travel step of the stitch cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Travel {
    Stitches(i64),
    Object(bool),
    Color(bool),
    Function(bool),
    Segment(bool),
    Hide,
}

/// Reads the travel keys (see the module comment).
pub fn travel_keys(i: &mut InputState) -> Option<Travel> {
    let steps = [
        (Modifiers::COMMAND.plus(Modifiers::SHIFT), 1000),
        (Modifiers::COMMAND, 100),
        (Modifiers::SHIFT, 10),
        (Modifiers::NONE, 1),
    ];
    for (m, n) in steps {
        if i.consume_key(m, Key::Comma) {
            return Some(Travel::Stitches(-n));
        }
        if i.consume_key(m, Key::Period) {
            return Some(Travel::Stitches(n));
        }
    }
    let pages = [
        (Modifiers::COMMAND, Travel::Color as fn(bool) -> Travel),
        (Modifiers::SHIFT, Travel::Function),
        (Modifiers::ALT, Travel::Segment),
        (Modifiers::NONE, Travel::Object),
    ];
    for (m, make) in pages {
        if i.consume_key(m, Key::PageUp) {
            return Some(make(false));
        }
        if i.consume_key(m, Key::PageDown) {
            return Some(make(true));
        }
    }
    None
}

impl AppState {
    /// The object that owns record `i` of the plan.
    pub fn owner_of(&mut self, i: usize) -> Option<ObjectId> {
        self.plan_view()
            .plan
            .spans
            .iter()
            .find(|s| s.records.contains(&i))
            .map(|s| s.object)
    }

    /// Moves the stitch cursor. The Stitches view turns on.
    pub fn travel(&mut self, t: Travel) {
        if t == Travel::Hide {
            self.cursor = None;
            return;
        }
        self.show_plan = true;
        let plan = self.plan_view().plan.clone();
        let recs = &plan.stitches.records;
        if recs.is_empty() {
            return;
        }
        let last = recs.len() - 1;
        let cur = self.cursor.unwrap_or(0).min(last);
        let find = |fwd: bool, ok: &dyn Fn(usize) -> bool| -> usize {
            if fwd {
                (cur + 1..=last).find(|&i| ok(i)).unwrap_or(last)
            } else {
                (0..cur).rev().find(|&i| ok(i)).unwrap_or(0)
            }
        };
        let kind = |i: usize| recs[i].kind;
        let new = match t {
            Travel::Stitches(n) => (cur as i64 + n).clamp(0, last as i64) as usize,
            Travel::Object(fwd) => {
                let starts: Vec<usize> = plan.spans.iter().map(|s| s.records.start).collect();
                if fwd {
                    starts.iter().copied().find(|&s| s > cur).unwrap_or(last)
                } else {
                    starts.iter().copied().rev().find(|&s| s < cur).unwrap_or(0)
                }
            }
            Travel::Color(fwd) => find(fwd, &|i| kind(i) == StitchKind::ColorChange),
            Travel::Function(fwd) => {
                let f = self.function_travel;
                find(fwd, &|i| f.matches(kind(i)))
            }
            Travel::Segment(fwd) => find(fwd, &|i| {
                kind(i) == StitchKind::Stitch && i > 0 && matches!(kind(i - 1), StitchKind::Jump)
            }),
            Travel::Hide => unreachable!(),
        };
        self.cursor = Some(new);
    }

    /// Selects the object at the stitch cursor.
    pub fn select_at_needle(&mut self) {
        if let Some(i) = self.cursor
            && let Some(id) = self.owner_of(i)
        {
            self.selection.set(id);
        }
    }

    /// The status line of the stitch cursor.
    pub fn cursor_text(&self) -> Option<String> {
        let i = self.cursor?;
        let pv = self.plan_view.as_ref()?;
        let recs = &pv.plan.stitches.records;
        let r = recs.get(i)?;
        let color = recs[..=i].iter().filter(|r| r.kind == StitchKind::ColorChange).count() + 1;
        let stitch = recs[..=i].iter().filter(|r| r.kind == StitchKind::Stitch).count();
        let owner = pv.plan.spans.iter().find(|s| s.records.contains(&i)).map(|s| s.object);
        let what = owner
            .and_then(|id| self.doc.design().object(id))
            .map(|o| o.name.clone().unwrap_or_else(|| o.type_name().to_string()))
            .unwrap_or_else(|| "connection".into());
        let u = self.settings.units;
        Some(format!(
            "record {} of {} · stitch {} · color {} · {:?} at X {} Y {} · {}",
            i + 1,
            recs.len(),
            stitch,
            color,
            r.kind,
            u.fmt(r.pos.x),
            u.fmt(r.pos.y),
            what
        ))
    }
}

/// The plan segments to draw: filtered by the view filter, and dimmed after the cursor.
pub fn visible_segments(st: &mut AppState) -> Vec<(Segment, Color32)> {
    let pv = st.shown_plan();
    let segs = pv.segments.clone();
    let plan = pv.plan.clone();
    if !st.view_filter.is_active() && st.cursor.is_none() {
        return segs.as_ref().clone();
    }
    let back = st.doc.design().from_hoop_transform();
    // Owner of each record, from the object spans.
    let mut owner: Vec<Option<ObjectId>> = vec![None; plan.stitches.records.len()];
    for s in &plan.spans {
        for o in &mut owner[s.records.clone()] {
            *o = Some(s.object);
        }
    }
    let design = st.doc.design();
    let shown = |id: Option<ObjectId>| {
        id.and_then(|id| design.object(id))
            .is_none_or(|o| st.view_filter.shows(o))
            && (id.is_some() || !st.view_filter.is_active())
    };
    let cursor = st.cursor.unwrap_or(usize::MAX);
    let mut out = Vec::with_capacity(segs.len());
    let mut thread = 0usize;
    let mut last: Option<Point> = None;
    let mut cut = false;
    let recs = &plan.stitches.records;
    for (i, r) in recs.iter().enumerate() {
        match r.kind {
            StitchKind::Stitch | StitchKind::Jump => {
                if let Some(a) = last
                    && a != r.pos
                    && shown(owner[i])
                {
                    let kind = if r.kind == StitchKind::Jump || cut {
                        SegmentKind::Jump
                    } else {
                        SegmentKind::Stitch
                    };
                    let rgb = plan
                        .stitches
                        .threads
                        .get(thread)
                        .map_or(emb_render::FALLBACK_COLOR, |t| t.color);
                    let mut color = crate::canvas::color32(rgb);
                    if i > cursor {
                        color = color.gamma_multiply(0.18);
                    }
                    out.push((
                        Segment {
                            a: back.apply(a),
                            b: back.apply(r.pos),
                            color: rgb,
                            kind,
                        },
                        color,
                    ));
                }
                if r.kind == StitchKind::Stitch {
                    cut = false;
                }
                last = Some(r.pos);
            }
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::Trim => cut = true,
            StitchKind::ColorChange => {
                thread += 1;
                cut = true;
            }
            StitchKind::Stop | StitchKind::End => {}
        }
    }
    out
}

/// Draws the stitch cursor, needle points and function symbols.
pub fn draw_overlay(painter: &egui::Painter, rect: Rect, st: &mut AppState) {
    if st.cursor.is_none() && !st.show_needle_points && !st.show_functions {
        return;
    }
    let plan = st.shown_plan().plan.clone();
    let back = st.doc.design().from_hoop_transform();
    let view = st.view;
    let to = |p: Point| view.to_screen(rect, back.apply(p));
    let vis = view.visible(rect).expand(1.0);
    let recs = &plan.stitches.records;
    if st.show_needle_points {
        let dot = Color32::from_rgb(20, 20, 20);
        for r in recs.iter().filter(|r| r.kind == StitchKind::Stitch) {
            if vis.contains(back.apply(r.pos)) {
                painter.circle_filled(to(r.pos), 1.5, dot);
            }
        }
    }
    if st.show_functions {
        let font = egui::FontId::proportional(11.0);
        for r in recs {
            let (text, color) = match r.kind {
                StitchKind::Trim => ("✂", Color32::from_rgb(200, 60, 60)),
                StitchKind::ColorChange => ("C", Color32::from_rgb(40, 120, 200)),
                StitchKind::Stop => ("S", Color32::from_rgb(200, 140, 20)),
                _ => continue,
            };
            let p = to(r.pos);
            painter.circle_filled(p, 7.0, Color32::WHITE.gamma_multiply(0.85));
            painter.text(p, egui::Align2::CENTER_CENTER, text, font.clone(), color);
        }
    }
    if let Some(i) = st.cursor
        && let Some(r) = recs.get(i)
    {
        let p = to(r.pos);
        let s = Stroke::new(1.5, Color32::from_rgb(255, 60, 160));
        painter.circle_stroke(p, 6.0, s);
        painter.line_segment([p - egui::vec2(10.0, 0.0), p + egui::vec2(10.0, 0.0)], s);
        painter.line_segment([p - egui::vec2(0.0, 10.0), p + egui::vec2(0.0, 10.0)], s);
        let _ = Pos2::ZERO;
    }
}

/// View menu items of this module.
#[cfg(test)]
mod tests {
    use eframe::egui::Event;
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn spans_cover_the_stitches_of_each_object() {
        let mut st = state();
        let plan = st.plan_view().plan.clone();
        let n_objects = st.doc.design().objects().len();
        assert_eq!(plan.spans.len(), n_objects);
        for w in plan.spans.windows(2) {
            assert!(
                w[0].records.end <= w[1].records.start,
                "{:?} {:?}",
                w[0].records,
                w[1].records
            );
        }
        for s in &plan.spans {
            let first = &plan.stitches.records[s.records.start];
            assert_eq!(first.kind, StitchKind::Stitch);
        }
    }

    #[test]
    fn travel_by_stitch_object_color_and_function() {
        let mut st = state();
        st.travel(Travel::Stitches(0));
        assert_eq!(st.cursor, Some(0));
        st.travel(Travel::Stitches(10));
        assert_eq!(st.cursor, Some(10));
        st.travel(Travel::Stitches(-100));
        assert_eq!(st.cursor, Some(0));
        st.travel(Travel::Object(true));
        let plan = st.plan_view().plan.clone();
        assert_eq!(st.cursor, Some(plan.spans[0].records.start));
        st.travel(Travel::Color(true));
        assert_eq!(plan.stitches.records[st.cursor.unwrap()].kind, StitchKind::ColorChange);
        st.travel(Travel::Function(false));
        let k = plan.stitches.records[st.cursor.unwrap()].kind;
        assert!(matches!(k, StitchKind::Trim | StitchKind::ColorChange), "{k:?}");
        st.select_at_needle();
        st.travel(Travel::Object(true));
        st.select_at_needle();
        assert_eq!(st.selection.len(), 1);
        assert!(st.cursor_text().unwrap().contains("record"));
        st.travel(Travel::Hide);
        assert!(st.cursor.is_none());
    }

    #[test]
    fn filters_hide_objects_and_dim_after_the_cursor() {
        let mut st = state();
        let all = visible_segments(&mut st).len();
        let first = st.doc.design().objects()[0].clone();
        st.view_filter.objects = Some([first.id].into_iter().collect());
        let some = visible_segments(&mut st).len();
        assert!(some > 0 && some < all, "{some} of {all}");
        assert!(!st.view_filter.shows(&st.doc.design().objects()[1]));
        st.view_filter = ViewFilter::default();
        st.cursor = Some(5);
        let segs = visible_segments(&mut st);
        let dim = segs.iter().filter(|(_, c)| c.a() < 255).count();
        assert!(dim > all / 2);
    }

    #[test]
    fn alt_v_hides_the_selection_on_screen_only() {
        use crate::test_util::Harness;
        let mut h = Harness::new();
        let first = h.id(0);
        let at = h.screen(h.center_of(first));
        let before = h.st.doc.design().clone();
        let segs = visible_segments(&mut h.st).len();
        h.st.selection.set(first);
        let alt_v = |pressed| Event::Key {
            key: Key::V,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::ALT,
        };
        h.frame(vec![alt_v(true)]);
        h.frame(vec![alt_v(false)]);
        // Hidden: not drawn, not picked, not selected; the design is the same
        // (no mirror, no undo step).
        assert!(h.st.view_filter.hidden.contains(&first));
        assert!(h.st.selection.is_empty());
        assert!(visible_segments(&mut h.st).len() < segs);
        assert_ne!(crate::canvas::pick(&h.st, h.map().to_design(at)), Some(first));
        h.st.select_all();
        assert!(!h.st.selection.contains(first));
        assert!(h.st.doc.design().same_content(&before));
        // Alt+V with nothing selected shows all again.
        h.st.selection.clear();
        h.frame(vec![alt_v(true)]);
        h.frame(vec![alt_v(false)]);
        assert!(!h.st.view_filter.is_active());
        assert_eq!(visible_segments(&mut h.st).len(), segs);
    }

    #[test]
    fn a_new_design_shows_all_and_closes_the_wizard() {
        use crate::test_util::Harness;
        let mut h = Harness::new();
        let first = h.id(0);
        h.st.selection.set(first);
        h.st.hide_selection();
        assert!(h.st.view_filter.is_active());
        h.st.new_design();
        assert!(!h.st.view_filter.is_active());
        assert!(h.st.wizard.is_none());
    }
}
