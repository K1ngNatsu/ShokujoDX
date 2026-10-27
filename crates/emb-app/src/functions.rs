//! Machine functions, split at needle and the stitch list (roadmap P3,
//! Milestone 2).
//!
//! - Each object has functions (`emb_model::Functions`): tie-in, tie-off,
//!   trim after (automatic, always, never) and stop after. The Object
//!   Properties panel and the Edit tab of the ribbon change them.
//! - Split at needle cuts the object under the stitch cursor in two, where
//!   the cursor is.
//! - The Stitch list panel shows each record of the stitch plan. A click on a
//!   row moves the stitch cursor there.

use eframe::egui::{self, RichText};
use emb_geom::{Point, math};
use emb_model::{Functions, ObjectId, ObjectKind, StitchKind, TrimMode};

use crate::actions::{knife_object, path_of, polylines, replace_with};
use crate::app::AppState;
use crate::i18n::tr;

/// A function that a ribbon button turns on and off for the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Stop,
    TrimAlways,
    TrimNever,
    TieIn,
    TieOff,
}

impl Func {
    fn get(self, f: &Functions) -> bool {
        match self {
            Func::Stop => f.stop,
            Func::TrimAlways => f.trim == TrimMode::Always,
            Func::TrimNever => f.trim == TrimMode::Never,
            Func::TieIn => f.tie_in,
            Func::TieOff => f.tie_off,
        }
    }

    fn set(self, f: &mut Functions, on: bool) {
        match self {
            Func::Stop => f.stop = on,
            Func::TrimAlways => f.trim = if on { TrimMode::Always } else { TrimMode::Auto },
            Func::TrimNever => f.trim = if on { TrimMode::Never } else { TrimMode::Auto },
            Func::TieIn => f.tie_in = on,
            Func::TieOff => f.tie_off = on,
        }
    }
}

impl AppState {
    /// Changes the functions of the selected objects as one undo step.
    pub fn edit_functions(&mut self, label: &str, f: impl Fn(&mut Functions)) {
        let ids = self.selection.ids().to_vec();
        if ids.is_empty() {
            return;
        }
        self.doc.edit(label, |d| {
            for id in ids {
                if let Some(o) = d.object_mut(id) {
                    f(&mut o.functions);
                }
            }
        });
    }

    /// `true` if all selected objects have the function.
    pub fn func_on(&self, func: Func) -> bool {
        let d = self.doc.design();
        let mut objs = self.selection.ids().iter().filter_map(|id| d.object(*id)).peekable();
        objs.peek().is_some() && objs.all(|o| func.get(&o.functions))
    }

    /// Turns the function on for all selected objects, or off if all have it.
    pub fn toggle_func(&mut self, func: Func) {
        let on = !self.func_on(func);
        let label = match func {
            Func::Stop => "Stop After",
            Func::TrimAlways => "Trim after",
            Func::TrimNever => "No trim after",
            Func::TieIn => "Tie-In",
            Func::TieOff => "Tie-Off",
        };
        self.edit_functions(label, |f| func.set(f, on));
    }

    /// The object under the stitch cursor and the needle position in design
    /// coordinates.
    fn needle_in_design(&mut self) -> Option<(ObjectId, Point)> {
        let i = self.cursor?;
        let id = self.owner_of(i)?;
        let pos = self.plan_view().plan.stitches.records.get(i)?.pos;
        let d = self.doc.design();
        Some((id, d.output_transform().inverse()?.apply(pos)))
    }

    /// Cuts the object under the stitch cursor in two where the cursor is.
    pub fn split_at_needle(&mut self) {
        let Some((id, p)) = self.needle_in_design() else {
            self.message = tr("Move the stitch cursor into an object first (View > Stitch Cursor).").into();
            return;
        };
        let Some(obj) = self.doc.design().object(id).cloned() else {
            return;
        };
        if self.refuse_linked(&[id]) {
            return;
        }
        let parts = match &obj.kind {
            ObjectKind::Applique { .. }
            | ObjectKind::Lettering(_)
            | ObjectKind::Vector { .. }
            | ObjectKind::CrossStitch(_)
            | ObjectKind::Bling(_)
            | ObjectKind::Sketch(_) => None,
            ObjectKind::Running { params, .. } => split_line(&obj, p).map(|pieces| {
                pieces
                    .iter()
                    .map(|pts| ObjectKind::Running {
                        path: path_of(pts, false),
                        params: params.clone(),
                    })
                    .collect()
            }),
            // Along the rows of the fill, through the needle.
            ObjectKind::Fill { params, .. } => {
                let (s, c) = math::sin_cos_deg(params.angle_deg);
                knife_object(&obj, p, Point::new(c, s), 0.0)
            }
            // Along the stitch direction there: from the nearest point of one
            // side to the nearest point of the other.
            ObjectKind::Satin { geometry, .. } => {
                let near = |path: &emb_geom::Path| nearest_on(&path.flatten(0.05), p);
                match (near(&geometry.rail_a), near(&geometry.rail_b)) {
                    (Some(a), Some(b)) => (b - a).normalized().and_then(|d| knife_object(&obj, a, d, 0.0)),
                    _ => None,
                }
            }
            ObjectKind::Stitches { path, trims } => split_stitches(path, trims, p),
        };
        let Some(parts) = parts.filter(|p| p.len() > 1) else {
            self.message = tr("Cannot split the object here. Move the stitch cursor into the object.").into();
            return;
        };
        let new = self.doc.edit("Split at Needle", |d| replace_with(d, id, parts));
        self.message = crate::i18n::fill("Split into {} objects.", &[&(new.len())]);
        self.selection.set_all(new);
    }
}

/// Cuts a stitch object at the needle point nearest to `p`: the first part
/// ends there, the second part starts there.
fn split_stitches(path: &emb_geom::Path, trims: &[usize], p: Point) -> Option<Vec<ObjectKind>> {
    let subs = emb_stitch::stitch_subpaths(path);
    let (si, pi) = subs
        .iter()
        .enumerate()
        .flat_map(|(si, s)| (0..s.len()).map(move |pi| (si, pi)))
        .min_by(|a, b| subs[a.0][a.1].distance(p).total_cmp(&subs[b.0][b.1].distance(p)))?;
    let mut first: Vec<Vec<Point>> = subs[..si].to_vec();
    first.push(subs[si][..=pi].to_vec());
    let mut second = vec![subs[si][pi..].to_vec()];
    second.extend_from_slice(&subs[si + 1..]);
    let count = |v: &Vec<Vec<Point>>| v.iter().map(Vec::len).sum::<usize>();
    if count(&first) < 2 || count(&second) < 2 {
        return None;
    }
    let first_trims: Vec<usize> = trims.iter().copied().filter(|&t| t <= si).collect();
    let second_trims: Vec<usize> = trims.iter().filter(|&&t| t > si).map(|t| t - si).collect();
    Some(vec![
        ObjectKind::Stitches {
            path: crate::actions::stitches_path(&first),
            trims: first_trims,
        },
        ObjectKind::Stitches {
            path: crate::actions::stitches_path(&second),
            trims: second_trims,
        },
    ])
}

/// The point of the polylines nearest to `p`.
fn nearest_on(lines: &[emb_geom::Polyline], p: Point) -> Option<Point> {
    lines
        .iter()
        .flat_map(|l| {
            let mut pts = l.points.clone();
            if l.closed && !pts.is_empty() {
                pts.push(pts[0]);
            }
            pts.windows(2)
                .map(|w| closest_on_segment(w[0], w[1], p))
                .collect::<Vec<_>>()
        })
        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
}

fn closest_on_segment(a: Point, b: Point, p: Point) -> Point {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 <= 0.0 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
}

/// Cuts an open line at the point nearest to `p`. A closed line cannot be
/// split at one point.
fn split_line(o: &emb_model::DesignObject, p: Point) -> Option<Vec<Vec<Point>>> {
    let lines = polylines(o);
    let [line] = lines.as_slice() else { return None };
    if line.closed || line.points.len() < 2 {
        return None;
    }
    let pts = &line.points;
    let (k, q) = (0..pts.len() - 1)
        .map(|k| (k, closest_on_segment(pts[k], pts[k + 1], p)))
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))?;
    let mut first: Vec<Point> = pts[..=k].to_vec();
    first.push(q);
    let mut second = vec![q];
    second.extend_from_slice(&pts[k + 1..]);
    let long = |v: &Vec<Point>| v.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() > 0.1;
    (long(&first) && long(&second)).then(|| vec![first, second])
}

/// The machine functions of the selection, in the Object Properties panel.
pub fn section(ui: &mut egui::Ui, st: &mut AppState) {
    ui.strong(tr("Machine Functions"));
    let mut f = {
        let d = st.doc.design();
        let first = st.selection.ids().first().and_then(|id| d.object(*id));
        first.map(|o| o.functions).unwrap_or_default()
    };
    let before = f;
    ui.horizontal(|ui| {
        ui.checkbox(&mut f.tie_in, tr("Tie-In"))
            .on_hover_text(tr("Lock stitches where the object starts after a trim"));
        ui.checkbox(&mut f.tie_off, tr("Tie-Off"))
            .on_hover_text(tr("Lock stitches where the object ends before a trim"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Trim after"));
        egui::ComboBox::from_id_salt("trim_mode")
            .selected_text(match f.trim {
                TrimMode::Auto => "Automatic",
                TrimMode::Always => "Always",
                TrimMode::Never => "Never",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut f.trim, TrimMode::Auto, tr("Automatic"))
                    .on_hover_text(tr("Trim when the next object is far; else a travel stitch"));
                ui.selectable_value(&mut f.trim, TrimMode::Always, tr("Always"));
                ui.selectable_value(&mut f.trim, TrimMode::Never, tr("Never"))
                    .on_hover_text(tr("A long gap becomes a jump; the thread is not cut"));
            });
    });
    ui.checkbox(&mut f.stop, tr("Stop After")).on_hover_text(tr(
        "The machine stops after this object, for example to put on appliqué fabric",
    ));
    ui.checkbox(&mut f.slow, tr("Sew slowly")).on_hover_text(tr(
        "The machine sews this object at its slow speed (small lettering, metallic thread). Only U01 files keep it.",
    ));
    if f != before {
        // Only the field that changed goes to all selected objects.
        st.edit_functions("Machine Functions", |g| {
            if f.tie_in != before.tie_in {
                g.tie_in = f.tie_in;
            }
            if f.tie_off != before.tie_off {
                g.tie_off = f.tie_off;
            }
            if f.trim != before.trim {
                g.trim = f.trim;
            }
            if f.stop != before.stop {
                g.stop = f.stop;
            }
            if f.slow != before.slow {
                g.slow = f.slow;
            }
        });
    }
}

/// A short text for the functions that are not the defaults, for the
/// Sequence panel.
pub fn summary(f: &Functions) -> String {
    let mut parts = Vec::new();
    match f.trim {
        TrimMode::Always => parts.push("trim"),
        TrimMode::Never => parts.push("no trim"),
        TrimMode::Auto => {}
    }
    if f.stop {
        parts.push("stop");
    }
    if f.slow {
        parts.push("slow");
    }
    if !f.tie_in || !f.tie_off {
        parts.push("no tie");
    }
    parts.join(", ")
}

fn kind_name(k: StitchKind) -> &'static str {
    match k {
        StitchKind::Stitch => tr("Stitch"),
        StitchKind::Jump => tr("Jump"),
        StitchKind::Trim => tr("Trim"),
        StitchKind::ColorChange => tr("Color change"),
        StitchKind::Stop => tr("Stop"),
        StitchKind::End => tr("End"),
        StitchKind::Slow => tr("Slow"),
        StitchKind::Fast => tr("Normal speed"),
    }
}

/// The Stitch list panel.
pub fn stitch_list(ui: &mut egui::Ui, st: &mut AppState) {
    let plan = st.shown_plan().plan.clone();
    let recs = &plan.stitches.records;
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.stitch_list_functions, tr("Functions only"))
            .on_hover_text(tr("Show only jumps, trims, color changes and stops"));
        ui.weak(crate::i18n::fill("{} records", &[&(recs.len())]));
    });
    // The color number of each record.
    let mut color = 1;
    let mut rows: Vec<(usize, usize)> = Vec::new();
    for (i, r) in recs.iter().enumerate() {
        if r.kind == StitchKind::ColorChange {
            color += 1;
        }
        if !st.stitch_list_functions || r.kind != StitchKind::Stitch {
            rows.push((i, color));
        }
    }
    let u = st.settings.units;
    let row_h = 18.0;
    let mut area = egui::ScrollArea::vertical().id_salt("stitch_list").auto_shrink(false);
    // Follow the stitch cursor when it moves.
    if st.cursor != st.stitch_list_shown
        && let Some(c) = st.cursor
        && let Some(row) = rows.iter().position(|&(i, _)| i >= c)
    {
        area = area.vertical_scroll_offset((row as f32 * row_h - 3.0 * row_h).max(0.0));
    }
    st.stitch_list_shown = st.cursor;
    let mut clicked = None;
    egui::Grid::new("stitch_list_head").num_columns(6).show(ui, |ui| {
        for h in ["#", "Kind", "X", "Y", "Length", "Color"] {
            ui.strong(h);
        }
    });
    area.show_rows(ui, row_h, rows.len(), |ui, range| {
        for &(i, c) in &rows[range] {
            let r = &recs[i];
            let len = if i > 0 { r.pos.distance(recs[i - 1].pos) } else { 0.0 };
            let text = format!(
                "{:>6}  {:<12} {:>9} {:>9} {:>8}  {}",
                i,
                kind_name(r.kind),
                u.fmt(r.pos.x),
                u.fmt(r.pos.y),
                if r.kind == StitchKind::Stitch {
                    u.fmt(len)
                } else {
                    String::new()
                },
                c
            );
            let mut rt = RichText::new(text).monospace();
            if r.kind != StitchKind::Stitch {
                rt = rt.strong();
            }
            if ui.selectable_label(st.cursor == Some(i), rt).clicked() {
                clicked = Some(i);
            }
        }
    });
    if let Some(i) = clicked {
        st.cursor = Some(i);
        st.stitch_list_shown = Some(i);
        st.show_plan = true;
        st.select_at_needle();
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn toggle_functions_on_the_selection() {
        let mut st = state();
        st.select_all();
        assert!(!st.func_on(Func::Stop));
        st.toggle_func(Func::Stop);
        assert!(st.func_on(Func::Stop));
        st.toggle_func(Func::TrimNever);
        assert!(st.func_on(Func::TrimNever) && !st.func_on(Func::TrimAlways));
        st.toggle_func(Func::TrimAlways);
        assert!(st.func_on(Func::TrimAlways));
        st.toggle_func(Func::Stop);
        assert!(!st.func_on(Func::Stop));
        // Each toggle is one undo step.
        for _ in 0..4 {
            st.undo();
        }
        assert!(st.doc.design().objects().iter().all(|o| o.functions.is_default()));
    }

    #[test]
    fn stop_shows_in_the_plan() {
        // Sample color blocks: red fill | white satin, white line | gold star | red ring.
        let mut st = state();
        let objs: Vec<ObjectId> = st.doc.design().objects().iter().map(|o| o.id).collect();
        // The fill is the last object of its block: a color change follows, so no stop.
        st.selection.set(objs[0]);
        st.toggle_func(Func::Stop);
        // The satin has the white line after it: one stop.
        st.selection.set(objs[1]);
        st.toggle_func(Func::Stop);
        let plan = st.plan_view().plan.clone();
        let stops = plan
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stop)
            .count();
        assert_eq!(stops, 1);
    }

    /// Puts the stitch cursor in the middle of the records of object `id`.
    fn cursor_into(st: &mut AppState, id: ObjectId) {
        let span = st
            .plan_view()
            .plan
            .spans
            .iter()
            .find(|s| s.object == id)
            .unwrap()
            .clone();
        st.cursor = Some((span.records.start + span.records.end) / 2);
    }

    #[test]
    fn split_each_kind_at_the_needle() {
        let mut st = state();
        // An open satin column (the satin of the sample is a ring).
        st.doc.edit("Add", |d| {
            let t = d.threads[0].id;
            let rail = |y: f64| {
                emb_geom::Path::new()
                    .move_to(Point::new(-40.0, y))
                    .line_to(Point::new(40.0, y))
            };
            d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: emb_model::SatinGeometry {
                        rail_a: rail(60.0),
                        rail_b: rail(64.0),
                        rungs: vec![],
                    },
                    params: Default::default(),
                },
            )
        });
        // The fill, the running line and the satin column.
        for k in [0, 2, 5] {
            let n = st.doc.design().objects().len();
            let id = st.doc.design().objects()[k].id;
            cursor_into(&mut st, id);
            st.split_at_needle();
            assert_eq!(st.doc.design().objects().len(), n + 1, "object {k}: {}", st.message);
            st.undo();
        }
        // No cursor: nothing happens.
        st.cursor = None;
        let n = st.doc.design().objects().len();
        st.split_at_needle();
        assert_eq!(st.doc.design().objects().len(), n);
    }

    #[test]
    fn split_line_keeps_the_ends() {
        let st = state();
        let line = st.doc.design().objects()[2].clone();
        let pieces = split_line(&line, Point::new(0.0, 0.0)).unwrap();
        let pl = &polylines(&line)[0].points;
        assert_eq!(pieces[0][0], pl[0]);
        assert_eq!(*pieces[1].last().unwrap(), *pl.last().unwrap());
        assert_eq!(pieces[0].last(), pieces[1].first());
    }

    #[test]
    fn panels_draw() {
        let mut st = state();
        st.select_all();
        st.stitch_list_functions = true;
        st.cursor = Some(10);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    section(ui, &mut st);
                    stitch_list(ui, &mut st);
                });
            });
            out.textures_delta.clear();
        }
    }
}
