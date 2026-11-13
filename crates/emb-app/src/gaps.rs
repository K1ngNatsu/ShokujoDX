//! Close Gaps (owner request, 2026-11-13): lines that were drawn one at a
//! time (Freehand, Line) join where their ends are near
//! (`emb_geom::gaps`).
//!
//! - Lines (running stitch) join with lines of the same color, and vector
//!   lines with vector lines of the same color.
//! - Each new part is one object, in the place of its first line. Lines
//!   that do not change stay as they are.
//! - The window sets the gap. The canvas shows the result while the value
//!   changes; OK keeps it as one undo step, Cancel goes back.

use eframe::egui;
use emb_geom::Path;
use emb_geom::gaps::{GapResult, Gaps, close_gaps, suggested_gap};
use emb_geom::nodes::{NodePath, SubNodes};
use emb_model::{Design, ObjectId, ObjectKind, ThreadId};

use crate::app::AppState;
use crate::i18n::tr;

/// The window starts with a gap that joins the ends nearer than this to
/// another end.
const SUGGEST_MAX_MM: f64 = 10.0;

/// The Close Gaps window.
#[derive(Debug, Clone)]
pub struct GapsDialog {
    pub settings: Gaps,
    /// The selected lines when the window opened.
    ids: Vec<ObjectId>,
    /// The settings that the canvas shows, and the length of the undo list
    /// after the change (`None`: nothing changed).
    applied: Option<(Gaps, Option<usize>)>,
    /// The gaps that closed and the shapes that closed.
    result: (usize, usize),
}

/// The changes of the objects: each object and its new kinds (none: it goes).
type Plan = Vec<(ObjectId, Vec<ObjectKind>)>;

fn is_line(k: &ObjectKind) -> bool {
    matches!(k, ObjectKind::Running { .. } | ObjectKind::Vector { .. })
}

/// `kind` with the path `path`.
fn with_path(kind: &ObjectKind, path: Path) -> ObjectKind {
    let mut k = kind.clone();
    match &mut k {
        ObjectKind::Running { path: p, .. } | ObjectKind::Vector { path: p, .. } => *p = path,
        _ => {}
    }
    k
}

fn path_of(subs: Vec<SubNodes>) -> Path {
    NodePath { subs }.to_path()
}

/// The changes that close the gaps of the lines `ids`, and the gaps and
/// shapes that closed.
fn plan(d: &Design, ids: &[ObjectId], g: &Gaps) -> (Plan, usize, usize) {
    let objs: Vec<&emb_model::DesignObject> = ids
        .iter()
        .filter_map(|id| d.object(*id))
        .filter(|o| is_line(&o.kind))
        .collect();
    let mut groups: Vec<((bool, ThreadId), Vec<&emb_model::DesignObject>)> = Vec::new();
    for o in objs {
        let key = (matches!(o.kind, ObjectKind::Vector { .. }), o.thread);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(o),
            None => groups.push((key, vec![o])),
        }
    }
    let (mut out, mut joins, mut closed) = (Plan::new(), 0, 0);
    for (_, group) in groups {
        // All subpaths of the group, and the object of each one.
        let mut subs = Vec::new();
        let mut owner = Vec::new();
        for (k, o) in group.iter().enumerate() {
            for p in o.kind.paths() {
                for s in NodePath::from_path(p).subs {
                    subs.push(s);
                    owner.push(k);
                }
            }
        }
        let GapResult {
            parts,
            joins: j,
            closed: c,
        } = close_gaps(&subs, g);
        if parts.iter().all(|p| !p.changed) {
            continue;
        }
        joins += j;
        closed += c;
        let mut keep: Vec<Vec<SubNodes>> = vec![Vec::new(); group.len()];
        let mut new: Vec<Vec<SubNodes>> = vec![Vec::new(); group.len()];
        let mut touched = vec![false; group.len()];
        for part in parts {
            let first = owner[part.from[0]];
            if part.changed {
                for &i in &part.from {
                    touched[owner[i]] = true;
                }
                new[first].push(part.sub);
            } else {
                keep[first].push(part.sub);
            }
        }
        for (k, o) in group.iter().enumerate() {
            if !touched[k] {
                continue;
            }
            let mut kinds: Vec<ObjectKind> = std::mem::take(&mut new[k])
                .into_iter()
                .map(|s| with_path(&o.kind, path_of(vec![s])))
                .collect();
            let rest = std::mem::take(&mut keep[k]);
            if !rest.is_empty() {
                kinds.push(with_path(&o.kind, path_of(rest)));
            }
            out.push((o.id, kinds));
        }
    }
    (out, joins, closed)
}

impl AppState {
    /// The selected lines.
    fn gap_targets(&self) -> Vec<ObjectId> {
        let d = self.doc.design();
        self.sel()
            .into_iter()
            .filter(|id| d.object(*id).is_some_and(|o| is_line(&o.kind)))
            .collect()
    }

    pub fn open_close_gaps(&mut self) {
        let ids = self.gap_targets();
        if ids.is_empty() {
            self.message = tr("Select the lines whose ends must join.").into();
            return;
        }
        if self.refuse_linked(&ids) {
            return;
        }
        // Start with a gap that joins the ends that are near each other.
        let d = self.doc.design();
        let subs: Vec<SubNodes> = ids
            .iter()
            .filter_map(|id| d.object(*id))
            .flat_map(|o| {
                o.kind
                    .paths()
                    .flat_map(|p| NodePath::from_path(p).subs)
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut settings = self.gap_settings;
        if let Some(g) = suggested_gap(&subs, SUGGEST_MAX_MM) {
            settings.gap_mm = g;
        }
        self.gaps_dialog = Some(GapsDialog {
            settings,
            ids,
            applied: None,
            result: (0, 0),
        });
    }

    /// Takes back the change of the window, if it is the last undo step.
    /// Returns `false` if other changes came after it.
    fn undo_gaps(&mut self, d: &mut GapsDialog) -> bool {
        let Some((_, len)) = d.applied.take() else {
            return true;
        };
        match len {
            None => true,
            Some(len) if self.doc.undo_len() == len && self.doc.undo_label() == Some(tr("Close Gaps")) => {
                self.doc.undo();
                true
            }
            Some(_) => false,
        }
    }

    /// Shows the result of the current settings on the canvas.
    pub fn preview_close_gaps(&mut self, d: &mut GapsDialog) {
        if d.applied.as_ref().is_some_and(|(g, _)| *g == d.settings) {
            return;
        }
        if !self.undo_gaps(d) {
            // The design changed after the preview: start from it.
            d.ids = self.gap_targets();
        }
        let (plan, joins, closed) = plan(self.doc.design(), &d.ids, &d.settings);
        d.result = (joins, closed);
        if plan.is_empty() {
            d.applied = Some((d.settings, None));
            self.selection.set_all(d.ids.clone());
            return;
        }
        let changed: Vec<ObjectId> = plan.iter().map(|(id, _)| *id).collect();
        let mut selected: Vec<ObjectId> = d.ids.iter().copied().filter(|id| !changed.contains(id)).collect();
        selected.extend(self.doc.edit(tr("Close Gaps"), |dd| {
            let mut new = Vec::new();
            for (id, kinds) in plan {
                if kinds.is_empty() {
                    dd.remove_object(id);
                } else {
                    new.extend(crate::actions::replace_with(dd, id, kinds));
                }
            }
            new
        }));
        d.applied = Some((d.settings, Some(self.doc.undo_len())));
        self.selection.set_all(selected);
    }

    /// OK (`keep`) or Cancel.
    pub fn close_close_gaps(&mut self, mut d: GapsDialog, keep: bool) {
        self.gap_settings = d.settings;
        if keep {
            self.preview_close_gaps(&mut d);
            self.message = crate::i18n::fill("Gaps closed: {}. Closed shapes: {}.", &[&d.result.0, &d.result.1]);
        } else if self.undo_gaps(&mut d) {
            self.selection.set_all(d.ids);
        }
    }
}

/// The Close Gaps window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.gaps_dialog.take() else {
        return;
    };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Close Gaps")).id(egui::Id::new("Close Gaps")),
        ctx,
        egui::Id::new("Close Gaps"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let s = &mut d.settings;
        egui::Grid::new("close_gaps")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Gap"))
                    .on_hover_text(tr("Ends nearer than this join. The nearest ends join first."));
                ui.add(
                    egui::Slider::new(&mut s.gap_mm, 0.1..=20.0)
                        .clamping(egui::SliderClamping::Edits)
                        .logarithmic(true)
                        .suffix(" mm")
                        .fixed_decimals(1),
                );
                ui.end_row();
            });
        ui.checkbox(&mut s.overshoots, tr("Cut overshoots")).on_hover_text(tr(
            "Lines that go across each other near their ends are cut where they cross, and join there.",
        ));
        ui.label(crate::i18n::fill(
            "Gaps closed: {}. Closed shapes: {}.",
            &[&d.result.0, &d.result.1],
        ));
        ui.weak(tr(
            "Ends join only with ends of lines of the same color. A closed shape can become a fill: Convert.",
        ));
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button(tr("OK")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
        });
    });
    if cancel || !open {
        st.close_close_gaps(d, false);
    } else if ok {
        st.close_close_gaps(d, true);
    } else {
        st.preview_close_gaps(&mut d);
        st.gaps_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::RunningParams;

    use super::*;
    use crate::test_util::Harness;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(pts: &[Point]) -> ObjectKind {
        ObjectKind::Running {
            path: crate::actions::path_of(pts, false),
            params: RunningParams::default(),
        }
    }

    fn strokes(h: &mut Harness) -> Vec<ObjectId> {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            [
                line(&[p(0.0, 0.0), p(10.0, 0.0)]),
                line(&[p(10.3, 0.4), p(5.0, 9.0)]),
                line(&[p(0.2, 0.3), p(4.8, 8.7)]),
                line(&[p(30.0, 0.0), p(40.0, 0.0)]),
            ]
            .into_iter()
            .map(|k| d.add_object(t, k))
            .collect()
        })
    }

    #[test]
    fn three_strokes_become_one_closed_line_and_cancel_goes_back() {
        let mut h = Harness::empty();
        let ids = strokes(&mut h);
        h.st.select_all();
        let before = h.st.doc.undo_len();
        h.st.open_close_gaps();
        let mut d = h.st.gaps_dialog.take().unwrap();
        h.st.preview_close_gaps(&mut d);
        let objs = h.st.doc.design().objects();
        assert_eq!(objs.len(), 2, "the triangle and the far line");
        let ObjectKind::Running { path, .. } = &objs[0].kind else {
            panic!()
        };
        let n = NodePath::from_path(path);
        assert!(n.subs.len() == 1 && n.subs[0].closed);
        assert_eq!(objs[1].id, ids[3], "the far line did not change");
        assert_eq!(h.st.selection.ids().len(), 2);
        // Another gap changes the same undo step.
        d.settings.gap_mm = 0.2;
        h.st.preview_close_gaps(&mut d);
        assert_eq!(h.st.doc.design().objects().len(), 4);
        assert_eq!(h.st.doc.undo_len(), before);
        d.settings.gap_mm = 1.0;
        h.st.preview_close_gaps(&mut d);
        assert_eq!(h.st.doc.undo_len(), before + 1);
        h.st.close_close_gaps(d.clone(), false);
        assert_eq!(h.st.doc.design().objects().len(), 4, "Cancel goes back");
        assert_eq!(h.st.doc.undo_len(), before);
        // OK keeps it as one undo step.
        h.st.select_all();
        h.st.open_close_gaps();
        let d = h.st.gaps_dialog.take().unwrap();
        h.st.close_close_gaps(d, true);
        assert_eq!(h.st.doc.design().objects().len(), 2);
        assert_eq!(h.st.doc.undo_label(), Some("Close Gaps"));
        assert!(h.st.message.contains("Gaps closed: 3"), "{}", h.st.message);
        h.st.undo();
        assert_eq!(h.st.doc.design().objects().len(), 4);
    }

    #[test]
    fn other_colors_do_not_join() {
        let mut h = Harness::empty();
        let ids = strokes(&mut h);
        h.st.doc.edit("Color", |d| {
            let t = d.add_thread("Blue", emb_model::Rgb(0, 0, 200));
            d.object_mut(ids[1]).unwrap().thread = t;
        });
        h.st.select_all();
        h.st.open_close_gaps();
        let mut d = h.st.gaps_dialog.take().unwrap();
        h.st.preview_close_gaps(&mut d);
        // Only the red strokes 0 and 2 join (at one end).
        assert_eq!(d.result, (1, 0));
        assert_eq!(h.st.doc.design().objects().len(), 3);
    }
}
