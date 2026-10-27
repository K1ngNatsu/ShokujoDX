//! Offsets and arrangements in the app (P6 Milestone 3): the Offsets
//! window (simple, inside, open and outline offsets, add border), the
//! Arrangements window (mirror-merge, array, wreath, kaleidoscope), and
//! Remove overlaps. The Border trace window (P6 Milestone 4, owner
//! request). The engine work is in `emb_model::arrange`,
//! `emb_model::shaping` and `emb_model::trace`.

use eframe::egui;
use emb_geom::region::Join;
use emb_model::{
    Arrangement, BorderPlace, MergeKey, OffsetMake, OffsetSpec, ThreadId, TraceOrder, TraceSpec, TraceStitch,
};

use crate::app::AppState;
use crate::i18n::tr;

/// The state of the Offsets and Arrangements windows.
pub struct ArtworkWindows {
    pub offsets: bool,
    pub spec: OffsetSpec,
    /// The width of a border, kept when the outline is a line or a vector.
    pub border_mm: f64,
    /// Where a border sits on its outline, kept as `border_mm`.
    pub border_place: BorderPlace,
    pub arrangements: bool,
    pub mirror_gap_mm: f64,
    pub rows: u32,
    pub columns: u32,
    pub gap_x_mm: f64,
    pub gap_y_mm: f64,
    pub wreath_count: u32,
    pub radius_mm: f64,
    pub trace: bool,
    pub trace_border_mm: f64,
    pub trace_gap_mm: f64,
    pub trace_stitch: TraceStitch,
    pub trace_order: TraceOrder,
    pub trace_place: BorderPlace,
    /// The border thread; `None`: a thread that differs from the objects.
    pub trace_thread: Option<ThreadId>,
}

impl Default for ArtworkWindows {
    fn default() -> Self {
        ArtworkWindows {
            offsets: false,
            spec: OffsetSpec::default(),
            border_mm: 3.0,
            border_place: BorderPlace::Outside,
            arrangements: false,
            mirror_gap_mm: 2.0,
            rows: 2,
            columns: 3,
            gap_x_mm: 5.0,
            gap_y_mm: 5.0,
            wreath_count: 6,
            radius_mm: 30.0,
            trace: false,
            trace_border_mm: 3.0,
            trace_gap_mm: 0.0,
            trace_stitch: TraceStitch::default(),
            trace_order: TraceOrder::FillThenBorder,
            trace_place: BorderPlace::Outside,
            trace_thread: None,
        }
    }
}

pub(crate) fn number(ui: &mut egui::Ui, label: &str, help: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>) {
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(help);
        ui.add(
            egui::DragValue::new(v)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(0.1)
                .fixed_decimals(1)
                .suffix(" mm"),
        );
    });
}

pub(crate) fn count(ui: &mut egui::Ui, label: &str, v: &mut u32, range: std::ops::RangeInclusive<u32>) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(v).range(range).clamp_existing_to_range(false));
    });
}

/// The choice of where a border sits on its line (`line` names the line
/// in the help text).
fn place_ui(ui: &mut egui::Ui, place: &mut BorderPlace, help: &str) {
    ui.horizontal(|ui| {
        ui.label(tr("Border position")).on_hover_text(help);
        for p in BorderPlace::ALL {
            ui.selectable_value(place, p, tr(p.name())).on_hover_text(help);
        }
    });
}

/// Shows the Offsets and Arrangements windows when they are open.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    offsets_window(ctx, st);
    arrangements_window(ctx, st);
    trace_window(ctx, st);
}

/// What the Border trace window does for the selection.
enum TraceAction {
    Add(TraceSpec),
    Change(u32, TraceSpec),
    Remove(u32),
}

fn trace_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.artwork.trace;
    let mut action = None;
    let existing = st.selected_trace();
    crate::dock::docked(egui::Window::new(tr("Border Trace")).id(egui::Id::new("Border Trace")), ctx, egui::Id::new("Border Trace"), st.canvas_rect)
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.weak(
                tr("A border round the selected objects, a gap round them, or both, cut out of the objects \
                 under them. In the gap the fabric shows."),
            );
            let d = st.doc.design();
            // The values of a trace of the selection, else the window values.
            let mut spec = existing.map_or_else(|| st.trace_spec(), |(_, s)| s);
            number(
                ui,
                tr("Border width"),
                tr("The width of the border. 0: no border."),
                &mut spec.border_mm,
                0.0..=20.0,
            );
            if spec.border_mm > 0.0 {
                ui.horizontal(|ui| {
                    ui.label(tr("Stitch"));
                    egui::ComboBox::from_id_salt("trace_stitch")
                        .selected_text(tr(spec.stitch.name()))
                        .show_ui(ui, |ui| {
                            for s in TraceStitch::all() {
                                let on = s == spec.stitch
                                    || matches!((s, spec.stitch), (TraceStitch::Column { pattern: a }, TraceStitch::Column { pattern: b }) if a.same_type(&b));
                                if ui.selectable_label(on, tr(s.name())).clicked() && !on {
                                    spec.stitch = s;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Thread"));
                    let name = d.thread(spec.thread).map_or("—".to_string(), |t| t.name.clone());
                    egui::ComboBox::from_id_salt("trace_thread")
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for t in &d.threads {
                                ui.selectable_value(&mut spec.thread, t.id, &t.name);
                            }
                        });
                });
                place_ui(
                    ui,
                    &mut spec.place,
                    tr("Where the border sits on the edge of the objects. Outside: it goes round the objects and \
                     covers only a little of their edge. Centered: its middle is on the edge. Inside: it covers \
                     the edge of the objects, and the design does not get larger."),
                );
                ui.horizontal(|ui| {
                    ui.label(tr("Sew Order"));
                    ui.selectable_value(&mut spec.order, TraceOrder::FillThenBorder, tr("Objects, then border"));
                    ui.selectable_value(&mut spec.order, TraceOrder::BorderThenFill, tr("Border, then objects"));
                });
            }
            number(
                ui,
                tr("Gap width"),
                tr("The width of the gap round the border (or round the objects with no border), where the \
                 fabric shows. 0: no gap."),
                &mut spec.gap_mm,
                0.0..=20.0,
            );
            ui.weak(tr("The trace stays linked: it is cut again when its objects move."));
            ui.separator();
            match existing {
                // A trace of the selection changes as the values change.
                Some((id, old)) => {
                    if spec != old {
                        action = Some(TraceAction::Change(id, spec));
                    }
                    if ui
                        .button(tr("Remove border trace"))
                        .on_hover_text(tr("Remove the border; the cut objects come back."))
                        .clicked()
                    {
                        action = Some(TraceAction::Remove(id));
                    }
                }
                None => {
                    st.artwork.trace_border_mm = spec.border_mm;
                    st.artwork.trace_gap_mm = spec.gap_mm;
                    st.artwork.trace_stitch = spec.stitch;
                    st.artwork.trace_order = spec.order;
                    st.artwork.trace_place = spec.place;
                    st.artwork.trace_thread = Some(spec.thread);
                    let ok = ui
                        .add_enabled(!st.selection.is_empty(), egui::Button::new(tr("Add border trace")))
                        .clicked();
                    if ok {
                        action = Some(TraceAction::Add(spec));
                    }
                }
            }
        });
    st.artwork.trace = open;
    match action {
        Some(TraceAction::Add(spec)) => st.add_border_trace(spec),
        Some(TraceAction::Change(id, spec)) => st.change_border_trace(id, spec),
        Some(TraceAction::Remove(id)) => st.remove_border_trace(id),
        None => {}
    }
}

fn offsets_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.artwork.offsets;
    let mut apply = false;
    crate::dock::docked(
        egui::Window::new(tr("Offsets")).id(egui::Id::new("Offsets")),
        ctx,
        egui::Id::new("Offsets"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| {
        let w = &mut st.artwork;
        ui.weak(tr(
            "Outlines at a distance round the selected objects (outside), or inside them.",
        ));
        let mut outside = w.spec.distance_mm >= 0.0;
        let mut dist = w.spec.distance_mm.abs();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut outside, true, tr("Outside"));
            ui.selectable_value(&mut outside, false, tr("Inside"));
        });
        number(
            ui,
            tr("Distance"),
            tr("The distance of the first outline from the objects."),
            &mut dist,
            0.0..=50.0,
        );
        w.spec.distance_mm = if outside { dist } else { -dist };
        count(ui, tr("Outlines"), &mut w.spec.count, 1..=20);
        if w.spec.count > 1 {
            number(
                ui,
                tr("Spacing"),
                tr("The distance between two outlines."),
                &mut w.spec.spacing_mm,
                0.1..=50.0,
            );
        }
        ui.horizontal(|ui| {
            ui.label(tr("Corners"));
            for (j, name) in [
                (Join::Round, tr("Round")),
                (Join::Miter, tr("Sharp")),
                (Join::Bevel, tr("Cut")),
            ] {
                ui.selectable_value(&mut w.spec.corners, j, name);
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Make"));
            let border = OffsetMake::Border {
                width_mm: w.border_mm,
                place: w.border_place,
            };
            for (m, name) in [
                (OffsetMake::Line, tr("Line")),
                (OffsetMake::Vector, tr("Vector")),
                (border, tr("Border")),
            ] {
                let on = std::mem::discriminant(&w.spec.make) == std::mem::discriminant(&m);
                if ui.selectable_label(on, name).clicked() {
                    w.spec.make = m;
                }
            }
        });
        if let OffsetMake::Border { width_mm, place } = &mut w.spec.make {
            number(
                ui,
                tr("Border width"),
                tr("The width of the satin border."),
                width_mm,
                0.5..=12.0,
            );
            place_ui(
                ui,
                place,
                tr(
                    "Where the border sits on the outline. Outside: away from the middle of the objects, so a \
                     border at distance 0 does not cover them. Centered: its middle is on the outline. Inside: \
                     toward the middle of the objects.",
                ),
            );
            w.border_mm = *width_mm;
            w.border_place = *place;
        }
        ui.separator();
        apply = ui
            .add_enabled(!st.selection.is_empty(), egui::Button::new(tr("Add outlines")))
            .clicked();
    });
    st.artwork.offsets = open;
    if apply {
        st.add_offsets();
    }
}

fn arrangements_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.artwork.arrangements;
    let mut run: Option<Arrangement> = None;
    let has = !st.selection.is_empty();
    crate::dock::docked(
        egui::Window::new(tr("Arrangements")).id(egui::Id::new("Arrangements")),
        ctx,
        egui::Id::new("Arrangements"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| {
        let w = &mut st.artwork;
        ui.strong(tr("Mirror-merge"));
        number(
            ui,
            tr("Gap"),
            tr("The space between the selection and its mirror copy."),
            &mut w.mirror_gap_mm,
            0.0..=100.0,
        );
        ui.horizontal(|ui| {
            for (h, v, name) in [
                (true, false, tr("Right")),
                (false, true, tr("Below")),
                (true, true, tr("Both")),
            ] {
                if ui.add_enabled(has, egui::Button::new(name)).clicked() {
                    run = Some(Arrangement::MirrorMerge {
                        horizontal: h,
                        vertical: v,
                        gap_mm: w.mirror_gap_mm,
                    });
                }
            }
        });
        ui.separator();
        ui.strong(tr("Array"));
        count(ui, tr("Rows"), &mut w.rows, 1..=50);
        count(ui, tr("Columns"), &mut w.columns, 1..=50);
        number(
            ui,
            tr("Gap across"),
            tr("The space between two columns."),
            &mut w.gap_x_mm,
            -100.0..=100.0,
        );
        number(
            ui,
            tr("Gap down"),
            tr("The space between two rows."),
            &mut w.gap_y_mm,
            -100.0..=100.0,
        );
        if ui.add_enabled(has, egui::Button::new(tr("Make array"))).clicked() {
            run = Some(Arrangement::Array {
                rows: w.rows,
                columns: w.columns,
                gap_x_mm: w.gap_x_mm,
                gap_y_mm: w.gap_y_mm,
            });
        }
        ui.separator();
        ui.strong(tr("Wreath and kaleidoscope"));
        count(ui, tr("Copies"), &mut w.wreath_count, 2..=72);
        number(
            ui,
            tr("Radius"),
            tr("The distance from the middle of the selection to the center, which is below it."),
            &mut w.radius_mm,
            0.0..=200.0,
        );
        ui.horizontal(|ui| {
            if ui.add_enabled(has, egui::Button::new(tr("Make wreath"))).clicked() {
                run = Some(Arrangement::Wreath {
                    count: w.wreath_count,
                    radius_mm: w.radius_mm,
                });
            }
            if ui
                .add_enabled(has, egui::Button::new(tr("Make kaleidoscope")))
                .on_hover_text(tr("A wreath with a mirror copy in each part."))
                .clicked()
            {
                run = Some(Arrangement::Kaleidoscope {
                    count: w.wreath_count,
                    radius_mm: w.radius_mm,
                });
            }
        });
    });
    st.artwork.arrangements = open;
    if let Some(a) = run {
        st.arrange_selection(&a);
    }
}

impl AppState {
    /// Adds the offset outlines of the Offsets window round the selection.
    /// One undo step.
    pub fn add_offsets(&mut self) {
        let ids = self.sel();
        let spec = self.artwork.spec;
        let mut probe = self.doc.design().clone();
        if let Err(e) = probe.add_offsets(&ids, &spec) {
            self.message = e.to_string();
            return;
        }
        let new = self
            .doc
            .edit("Offsets", |d| d.add_offsets(&ids, &spec).unwrap_or_default());
        self.message = crate::i18n::fill("{} outlines added.", &[&(new.len())]);
        self.selection.set_all(new);
    }

    /// Adds the copies of an arrangement of the selection. One undo step.
    pub fn arrange_selection(&mut self, a: &Arrangement) {
        let ids = self.sel();
        if ids.is_empty() {
            self.message = tr("Select objects first.").into();
            return;
        }
        let new = self.doc.edit("Arrange copies", |d| d.arrange(&ids, a));
        let mut all = ids;
        all.extend(&new);
        self.message = crate::i18n::fill("{} copies added.", &[&(new.len())]);
        self.selection.set_all(all);
    }

    /// Cuts the selection out of the objects under it. One undo step.
    pub fn remove_overlaps(&mut self) {
        let ids = self.sel();
        let overlap = self.shaping.overlap_mm;
        let mut probe = self.doc.design().clone();
        let linked = self.shaping.linked;
        if let Err(e) = probe.remove_overlaps(&ids, overlap, linked) {
            self.message = e.to_string();
            return;
        }
        let n = self.doc.edit("Remove Overlaps", |d| {
            d.remove_overlaps(&ids, overlap, linked).map_or(0, |v| v.len())
        });
        self.message = crate::i18n::fill("Overlaps removed: {} pieces left under the selection.", &[&n]);
    }

    /// The border trace of the selection, with its settings.
    pub fn selected_trace(&self) -> Option<(u32, TraceSpec)> {
        let d = self.doc.design();
        self.selection
            .ids()
            .iter()
            .find_map(|id| d.trace_of(*id))
            .map(|t| (t.id, t.spec))
    }

    /// The settings of a new border trace from the window values. The
    /// thread is the chosen one, else a thread that the selection does not
    /// use.
    pub fn trace_spec(&self) -> TraceSpec {
        let d = self.doc.design();
        let used: Vec<ThreadId> = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id))
            .map(|o| o.thread)
            .collect();
        let thread = self
            .artwork
            .trace_thread
            .filter(|t| d.thread(*t).is_some())
            .or_else(|| d.threads.iter().map(|t| t.id).find(|t| !used.contains(t)))
            .or_else(|| d.threads.first().map(|t| t.id))
            .unwrap_or(ThreadId(0));
        TraceSpec {
            border_mm: self.artwork.trace_border_mm,
            gap_mm: self.artwork.trace_gap_mm,
            stitch: self.artwork.trace_stitch,
            thread,
            order: self.artwork.trace_order,
            place: self.artwork.trace_place,
        }
    }

    /// Adds a border trace round the selection. One undo step.
    pub fn add_border_trace(&mut self, spec: TraceSpec) {
        let ids = self.sel();
        let mut probe = self.doc.design().clone();
        if let Err(e) = probe.add_border_trace(&ids, spec) {
            self.message = e.to_string();
            return;
        }
        self.doc.edit("Border Trace", |d| {
            let _ = d.add_border_trace(&ids, spec);
        });
        self.message = tr("Border trace added. Change it in this window while its objects are selected.").into();
    }

    /// Changes border trace `id`. Changes in a row (a drag of a value) make
    /// one undo step.
    pub fn change_border_trace(&mut self, id: u32, spec: TraceSpec) {
        let mut probe = self.doc.design().clone();
        if let Err(e) = probe.update_border_trace(id, spec) {
            self.message = e.to_string();
            return;
        }
        self.doc
            .edit_merged("Change border trace", MergeKey::new(format!("trace-{id}")), |d| {
                let _ = d.update_border_trace(id, spec);
            });
    }

    /// Removes border trace `id`; the cut objects come back.
    pub fn remove_border_trace(&mut self, id: u32) {
        self.doc.edit("Remove border trace", |d| d.remove_border_trace(id));
    }

    /// Returns `true` (with a message) if one of `ids` is linked (a border
    /// trace or linked shapes). Commands that replace objects with new ones
    /// (join, break apart, knife, split at the needle) refuse such objects:
    /// the link would lose them.
    pub fn refuse_linked(&mut self, ids: &[emb_model::ObjectId]) -> bool {
        let d = self.doc.design();
        if ids.iter().any(|id| d.is_linked(*id)) {
            self.message = emb_model::ShapeError::AlreadyLinked.to_string();
            return true;
        }
        false
    }

    /// Unlink: the border traces, shape links and sketch links of the
    /// selection (also of a selected sketch) end; their objects stay as
    /// they are now. One undo step.
    pub fn unlink_selection(&mut self) {
        let ids = self.sel();
        let d = self.doc.design();
        let links: Vec<u32> = d
            .shape_links
            .iter()
            .filter(|l| ids.iter().any(|id| l.has(*id)))
            .map(|l| l.id)
            .collect();
        let traces: Vec<u32> = d
            .border_traces
            .iter()
            .filter(|t| ids.iter().any(|id| t.has(*id)))
            .map(|t| t.id)
            .collect();
        let sketch_links: Vec<u32> = d
            .sketch_links
            .iter()
            .filter(|l| ids.iter().any(|id| l.has(*id) || l.sketch == *id))
            .map(|l| l.id)
            .collect();
        if links.is_empty() && traces.is_empty() && sketch_links.is_empty() {
            self.message = tr("The selection has no links.").into();
            return;
        }
        self.doc.edit("Unlink", |d| {
            d.shape_links.retain(|l| !links.contains(&l.id));
            d.border_traces.retain(|t| !traces.contains(&t.id));
            d.sketch_links.retain(|l| !sketch_links.contains(&l.id));
        });
        self.message = tr("Unlinked: the objects stay as they are.").into();
    }

    /// Reflect: the Mirror line tool, which makes a mirror copy.
    pub fn start_reflect(&mut self) {
        self.set_tool(crate::draw::Tool::MirrorLine);
        self.reflect_copy = true;
        self.message = tr("Click two points of the mirror line. The mirror copy is added.").into();
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};
    use emb_model::{ObjectId, ObjectKind, Rgb};

    use super::*;
    use crate::test_util::Harness;

    fn two_squares(h: &mut Harness) -> Vec<ObjectId> {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            let sq = |x: f64| ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + 20.0, 20.0))),
                holes: Vec::new(),
                params: Default::default(),
            };
            vec![d.add_object(t, sq(0.0)), d.add_object(t, sq(10.0))]
        })
    }

    #[test]
    fn offsets_arrangements_and_remove_overlaps_are_one_step_each() {
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        let start = h.st.doc.design().clone();
        let steps = h.st.doc.undo_len();

        h.st.selection.set_all(ids.clone());
        h.st.artwork.spec.count = 2;
        h.st.add_offsets();
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert_eq!(h.st.doc.design().objects().len(), 4);
        h.st.undo();

        h.st.selection.set(ids[0]);
        h.st.arrange_selection(&Arrangement::Wreath {
            count: 4,
            radius_mm: 30.0,
        });
        assert_eq!(h.st.doc.design().objects().len(), 5);
        assert_eq!(h.st.selection.len(), 4, "the selection and its copies");
        h.st.undo();

        h.st.selection.set(ids[1]);
        h.st.remove_overlaps();
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        h.st.undo();
        assert!(h.st.doc.design().same_content(&start));

        // Nothing under the first square: no step, and a message.
        h.st.selection.set(ids[0]);
        h.st.remove_overlaps();
        assert_eq!(h.st.doc.undo_len(), steps);
        assert_eq!(h.st.message, emb_model::ShapeError::NothingToCut.to_string());
    }

    #[test]
    fn automatic_knife_cuts_with_an_overlap() {
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        h.st.selection.set(ids[0]);
        h.st.shaping.overlap_mm = 1.0;
        h.st.knife(Point::new(10.0, -5.0), Point::new(10.0, 25.0));
        let d = h.st.doc.design();
        let widths: Vec<f64> = d
            .objects()
            .iter()
            .filter(|o| o.id != ids[1])
            .map(|o| o.control_bbox().unwrap().width())
            .collect();
        // Each piece goes 0.5 mm past the cut, under the other piece.
        assert_eq!(widths.len(), 2);
        assert!(widths.iter().all(|w| (w - 10.5).abs() < 1e-6), "{widths:?}");
    }

    /// The Border trace window has the Border position choice (owner
    /// report of 2026-10-26), and it opens docked at the canvas corner.
    #[test]
    fn the_border_trace_window_shows_the_border_position() {
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        h.st.selection.set(ids[1]);
        h.st.artwork.trace = true;
        let canvas = egui::Rect::from_min_size(egui::pos2(200.0, 150.0), egui::vec2(800.0, 600.0));
        h.st.canvas_rect = Some(canvas);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut names = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))),
                ..Default::default()
            };
            let st = &mut h.st;
            let mut out = ctx.run_ui(input, |ui| windows(&ui.ctx().clone(), st));
            out.textures_delta.clear();
            let update = out.platform_output.accesskit_update.expect("the screen reader tree");
            names = update
                .nodes
                .iter()
                .filter_map(|(_, n)| n.label().map(str::to_string))
                .collect();
        }
        for want in ["Outside the line", "Centered on the line", "Inside the line"] {
            assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
        }
        let r = ctx.memory(|m| m.area_rect(egui::Id::new("Border Trace"))).unwrap();
        assert_eq!(
            r.left_top(),
            canvas.left_top() + egui::vec2(crate::dock::MARGIN, crate::dock::MARGIN)
        );
    }

    #[test]
    fn border_trace_add_change_remove() {
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        let start = h.st.doc.design().clone();
        let steps = h.st.doc.undo_len();
        h.st.selection.set(ids[1]);
        let spec = h.st.trace_spec();
        h.st.add_border_trace(spec);
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let (id, got) = h.st.selected_trace().expect("a trace of the selection");
        assert_eq!(got, spec);
        // A change of the width in two steps is one undo step.
        h.st.change_border_trace(id, TraceSpec { border_mm: 4.0, ..spec });
        h.st.change_border_trace(id, TraceSpec { border_mm: 5.0, ..spec });
        assert_eq!(h.st.doc.undo_len(), steps + 2);
        h.st.doc.end_merge();
        h.st.remove_border_trace(id);
        assert!(h.st.doc.design().border_traces.is_empty());
        for _ in 0..3 {
            h.st.undo();
        }
        assert!(h.st.doc.design().same_content(&start));
        // A trace on a trace is refused with a message and no step.
        h.st.selection.set(ids[1]);
        h.st.add_border_trace(spec);
        let n = h.st.doc.undo_len();
        h.st.add_border_trace(spec);
        assert_eq!(h.st.doc.undo_len(), n);
        assert_eq!(h.st.message, emb_model::shaping::ShapeError::AlreadyLinked.to_string());
        // The windows keep the place of a new border (owner request of
        // 2026-10-26), and Add border takes it.
        h.st.artwork.trace_place = BorderPlace::Inside;
        assert_eq!(h.st.trace_spec().place, BorderPlace::Inside);
        h.st.artwork.border_place = BorderPlace::Center;
        h.st.run(crate::ribbon::Cmd::AddBorder, &egui::Context::default());
        assert!(matches!(
            h.st.artwork.spec.make,
            OffsetMake::Border {
                place: BorderPlace::Center,
                ..
            }
        ));
    }

    #[test]
    fn linked_flatten_follows_a_drag_and_unlinks() {
        use eframe::egui::{Modifiers, PointerButton};
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        h.st.selection.set_all(ids.clone());
        h.st.shape_selection(emb_model::ShapeOp::Flatten);
        assert_eq!(h.st.doc.design().shape_links.len(), 1, "linked by default");
        let d = h.st.doc.design();
        let back = d.objects()[0].id;
        let front = d.objects()[1].id;
        let area = |h: &Harness, id: ObjectId| h.st.doc.design().object(id).unwrap().control_bbox().unwrap().width();
        assert!((area(&h, back) - 10.0).abs() < 1e-6, "the back is cut");
        // Drag the front away on the canvas: the back is whole again.
        h.st.selection.set(front);
        let c = h.st.doc.design().bbox_of(&[front]).unwrap().center();
        let (from, to) = (h.screen(c), h.screen(c + Point::new(40.0, 0.0)));
        let steps = h.st.doc.undo_len();
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        assert!((area(&h, back) - 20.0).abs() < 1e-6, "{}", area(&h, back));
        assert_eq!(h.st.doc.undo_len(), steps + 1, "one undo step");
        assert!(h.st.selection.contains(front), "the dragged piece keeps its id");
        h.st.undo();
        assert!((area(&h, back) - 10.0).abs() < 1e-6);
        h.st.selection.set(front);
        h.st.unlink_selection();
        assert!(h.st.doc.design().shape_links.is_empty());
    }

    #[test]
    fn linked_pieces_stay_whole_convert_and_refuse_join() {
        let mut h = Harness::empty();
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            // A vector with two squares (joined artwork), and a bar across
            // its middle, in front.
            let two = Path::joined([
                &Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0))),
                &Path::rect(BBox::new(Point::new(30.0, 0.0), Point::new(50.0, 20.0))),
            ]);
            let style = emb_model::VectorStyle::filled(Rgb(200, 0, 0));
            vec![
                d.add_object(t, ObjectKind::Vector { path: two, style }),
                d.add_object(
                    t,
                    ObjectKind::Vector {
                        path: Path::rect(BBox::new(Point::new(-5.0, 8.0), Point::new(55.0, 12.0))),
                        style,
                    },
                ),
            ]
        });
        h.st.selection.set_all(ids.clone());
        h.st.shape_selection(emb_model::ShapeOp::Flatten);
        // One object for each original: the back vector is cut in four
        // areas but stays one object.
        assert_eq!(h.st.doc.design().objects().len(), 2);
        let pieces: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        // To fill keeps the objects and the link.
        h.st.selection.set_all(pieces.clone());
        h.st.convert_vectors(emb_model::ConvertTo::Fill);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        assert!(
            pieces
                .iter()
                .all(|id| matches!(d.object(*id).unwrap().kind, ObjectKind::Fill { .. }))
        );
        assert_eq!(d.shape_links.len(), 1);
        // Join, break apart and the knife refuse linked objects.
        let steps = h.st.doc.undo_len();
        h.st.join_selection();
        h.st.break_apart();
        h.st.knife(Point::new(25.0, -10.0), Point::new(25.0, 30.0));
        assert_eq!(h.st.doc.undo_len(), steps);
        assert_eq!(h.st.message, emb_model::ShapeError::AlreadyLinked.to_string());
        // To vector keeps the link too.
        h.st.convert_to_vectors();
        assert_eq!(h.st.doc.design().shape_links.len(), 1);
        assert!(matches!(
            h.st.doc.design().object(pieces[0]).unwrap().kind,
            ObjectKind::Vector { .. }
        ));
    }

    #[test]
    fn reflect_adds_a_mirror_copy() {
        let mut h = Harness::empty();
        let ids = two_squares(&mut h);
        h.st.selection.set(ids[0]);
        h.st.start_reflect();
        h.st.mirror_about_line(Point::new(30.0, -10.0), Point::new(30.0, 10.0));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 3);
        let b = d.objects()[2].control_bbox().unwrap();
        assert!((b.min.x - 40.0).abs() < 1e-9 && (b.max.x - 60.0).abs() < 1e-9, "{b:?}");
    }
}
