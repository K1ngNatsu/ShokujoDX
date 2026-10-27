//! Simplify: fewer nodes for the selected objects (`emb_geom::simplify`).
//!
//! - Quick simplify uses the standard settings, as one undo step.
//! - Simplify opens a window with the tolerance, the corner angle and
//!   curves or straight lines. The canvas shows the result while the values
//!   change; OK keeps it, Cancel goes back.
//!
//! Lettering and stitch objects are not changed (lettering: reshape one
//! letter; stitch objects have no nodes).

use eframe::egui;
use emb_geom::simplify::{Simplify, node_count, simplified};
use emb_model::{DesignObject, MergeKey, ObjectKind};

use crate::app::AppState;
use crate::i18n::tr;

/// The Simplify window.
#[derive(Debug, Clone)]
pub struct SimplifyDialog {
    pub settings: Simplify,
    /// The objects before the change.
    originals: Vec<DesignObject>,
    key: MergeKey,
    /// The last settings applied to the design.
    applied: Option<Simplify>,
}

/// The simplified kind, or `None` for kinds that are not simplified.
pub fn simplify_kind(kind: &ObjectKind, s: &Simplify) -> Option<ObjectKind> {
    if matches!(kind, ObjectKind::Stitches { .. } | ObjectKind::Lettering(_)) {
        return None;
    }
    let mut k = kind.clone();
    for p in k.paths_mut() {
        *p = simplified(p, s);
    }
    Some(k)
}

fn nodes(objs: &[DesignObject]) -> usize {
    objs.iter().map(|o| o.kind.paths().map(node_count).sum::<usize>()).sum()
}

impl AppState {
    /// The selected objects that simplify can change.
    fn simplify_targets(&self) -> Vec<DesignObject> {
        let d = self.doc.design();
        self.sel()
            .into_iter()
            .filter_map(|id| d.object(id))
            .filter(|o| !matches!(o.kind, ObjectKind::Stitches { .. } | ObjectKind::Lettering(_)))
            .cloned()
            .collect()
    }

    /// Sets the simplified objects of `originals` with `s`.
    fn apply_simplify(&mut self, originals: &[DesignObject], s: &Simplify, key: Option<MergeKey>) {
        let change = |d: &mut emb_model::Design| {
            for o in originals {
                if let (Some(k), Some(obj)) = (simplify_kind(&o.kind, s), d.object_mut(o.id)) {
                    obj.kind = k;
                }
            }
        };
        match key {
            Some(key) => self.doc.edit_merged("Simplify", key, change),
            None => self.doc.edit("Simplify", change),
        }
    }

    /// Quick simplify of the selection.
    pub fn quick_simplify(&mut self) {
        let objs = self.simplify_targets();
        if objs.is_empty() {
            self.message = tr("Select objects with nodes (not lettering or stitch objects).").into();
            return;
        }
        let before = nodes(&objs);
        self.apply_simplify(&objs, &Simplify::QUICK, None);
        let after = nodes(&self.simplify_targets());
        self.message = if after < before {
            format!("Simplified: {before} nodes to {after}.")
        } else {
            "These objects are already simple.".into()
        };
    }

    pub fn open_simplify(&mut self) {
        let originals = self.simplify_targets();
        if originals.is_empty() {
            self.message = tr("Select objects with nodes (not lettering or stitch objects).").into();
            return;
        }
        self.drag_counter += 1;
        self.simplify_dialog = Some(SimplifyDialog {
            settings: Simplify::QUICK,
            originals,
            key: MergeKey::new(format!("simplify-{}", self.drag_counter)),
            applied: None,
        });
    }
}

/// The Simplify window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.simplify_dialog.take() else {
        return;
    };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Simplify")).id(egui::Id::new("Simplify")),
        ctx,
        egui::Id::new("Simplify"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let s = &mut d.settings;
        egui::Grid::new("simplify")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Tolerance")).on_hover_text(tr(
                    "How far the new outline can be from the old one. Larger: fewer nodes.",
                ));
                ui.add(
                    egui::Slider::new(&mut s.tolerance_mm, 0.02..=3.0)
                        .clamping(egui::SliderClamping::Edits)
                        .logarithmic(true)
                        .suffix(" mm")
                        .fixed_decimals(2),
                );
                ui.end_row();
                ui.label(tr("Corners"))
                    .on_hover_text(tr("A turn sharper than this stays a corner. Smaller: more corners."));
                ui.add(
                    egui::Slider::new(&mut s.corner_deg, 5.0..=170.0)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix("°")
                        .fixed_decimals(0),
                );
                ui.end_row();
                ui.label(tr("Between corners"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.curves, true, tr("Curves"));
                    ui.radio_value(&mut s.curves, false, tr("Straight lines"));
                });
                ui.end_row();
            });
        let before = nodes(&d.originals);
        let after: usize = d
            .originals
            .iter()
            .map(|o| {
                simplify_kind(&o.kind, &d.settings).map_or_else(
                    || o.kind.paths().map(node_count).sum::<usize>(),
                    |k| k.paths().map(node_count).sum(),
                )
            })
            .sum();
        ui.label(crate::i18n::fill("Nodes: {} → {}", &[&before, &after]));
        ui.weak(tr(
            "Simplify never adds nodes: a shape that is already simple stays as it is.",
        ));
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button(tr("OK")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
            if ui
                .button(tr("Standard"))
                .on_hover_text(tr("The settings of Quick simplify."))
                .clicked()
            {
                d.settings = Simplify::QUICK;
            }
        });
    });
    if cancel || !open {
        st.close_simplify(d, false);
    } else if ok {
        st.close_simplify(d, true);
    } else {
        st.preview_simplify(&mut d);
        st.simplify_dialog = Some(d);
    }
}

impl AppState {
    /// Shows the result of the current settings on the canvas.
    pub fn preview_simplify(&mut self, d: &mut SimplifyDialog) {
        if d.applied != Some(d.settings) {
            let originals = d.originals.clone();
            self.apply_simplify(&originals, &d.settings, Some(d.key.clone()));
            d.applied = Some(d.settings);
        }
    }

    /// OK (`keep`) or Cancel.
    pub fn close_simplify(&mut self, mut d: SimplifyDialog, keep: bool) {
        if keep {
            self.preview_simplify(&mut d);
            self.doc.end_merge();
        } else if d.applied.is_some() {
            self.doc.end_merge();
            self.undo();
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};

    use super::*;
    use crate::test_util::Harness;

    fn wobbly(h: &mut Harness) -> emb_model::ObjectId {
        let pts: Vec<Point> = (0..120)
            .map(|i| {
                let a = i as f64 / 120.0 * std::f64::consts::TAU;
                Point::new(15.0 * a.cos(), 10.0 * a.sin())
            })
            .collect();
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::polygon(&pts),
                    holes: vec![],
                    params: Default::default(),
                },
            )
        })
    }

    fn node_total(h: &Harness, id: emb_model::ObjectId) -> usize {
        h.st.doc.design().object(id).unwrap().kind.paths().map(node_count).sum()
    }

    #[test]
    fn quick_simplify_is_one_undo_step() {
        let mut h = Harness::empty();
        let id = wobbly(&mut h);
        h.st.selection.set(id);
        assert_eq!(node_total(&h, id), 120);
        h.st.quick_simplify();
        let n = node_total(&h, id);
        assert!(n < 15, "{n}");
        assert!(h.st.message.contains("120 nodes"), "{}", h.st.message);
        h.st.undo();
        assert_eq!(node_total(&h, id), 120);
    }

    #[test]
    fn the_window_shows_the_result_and_cancel_goes_back() {
        let mut h = Harness::empty();
        let id = wobbly(&mut h);
        h.st.selection.set(id);
        h.st.open_simplify();
        let mut d = h.st.simplify_dialog.take().unwrap();
        h.st.preview_simplify(&mut d);
        let quick = node_total(&h, id);
        assert!(quick < 120);
        // Other settings change the same undo step.
        d.settings.curves = false;
        d.settings.tolerance_mm = 2.0;
        h.st.preview_simplify(&mut d);
        assert!(node_total(&h, id) < 120);
        h.st.close_simplify(d.clone(), false);
        assert_eq!(node_total(&h, id), 120, "Cancel goes back");
        // OK keeps the result as one undo step.
        d.applied = None;
        h.st.preview_simplify(&mut d);
        let kept = node_total(&h, id);
        h.st.close_simplify(d, true);
        assert_eq!(node_total(&h, id), kept);
        h.st.undo();
        assert_eq!(node_total(&h, id), 120);
    }
}
