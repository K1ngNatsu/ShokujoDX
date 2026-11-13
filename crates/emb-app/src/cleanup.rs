//! Clean Up (owner request, 2026-11-13): smooth lines and outlines that
//! have small loops, overlaps, spikes or zig-zags, as from an SVG file or a
//! trace (`emb_geom::cleanup`).
//!
//! - Lines and vector objects: each subpath is cleaned.
//! - Fills: the outline and the holes are cleaned, then the areas are
//!   made again, so parts of the outline that go over each other become
//!   one area. Areas and holes smaller than the square of the detail size go.
//! - Satin columns, appliqué, lettering and stitch objects do not change.
//!
//! The window sets the detail size. The canvas shows the result while the
//! values change; OK keeps it as one undo step, Cancel goes back.

use eframe::egui;
use emb_geom::cleanup::{Cleanup, clean_polylines, cleaned, curves};
use emb_geom::region::{self, Region};
use emb_geom::simplify::node_count;
use emb_geom::{Path, Point};
use emb_model::{DesignObject, MergeKey, ObjectKind};

use crate::app::AppState;
use crate::i18n::tr;

/// The Clean Up window.
#[derive(Debug, Clone)]
pub struct CleanupDialog {
    pub settings: Cleanup,
    /// The objects before the change.
    originals: Vec<DesignObject>,
    key: MergeKey,
    /// The last settings applied to the design.
    applied: Option<Cleanup>,
}

fn can_clean(k: &ObjectKind) -> bool {
    matches!(
        k,
        ObjectKind::Running { .. } | ObjectKind::Vector { .. } | ObjectKind::Fill { .. }
    )
}

/// The areas of a cleaned fill, or `None` if nothing stays.
fn clean_fill(outer: &Path, holes: &[Path], c: &Cleanup) -> Option<(Path, Vec<Path>)> {
    let rings = |p: &Path| -> Vec<Vec<Point>> {
        clean_polylines(p, c)
            .into_iter()
            .filter(|(pts, _)| pts.len() >= 3)
            .map(|(pts, _)| pts)
            .collect()
    };
    let outers: Vec<Path> = rings(outer).iter().map(|r| Path::polygon(r)).collect();
    let o = Path::joined(&outers);
    let h: Vec<Path> = holes.iter().flat_map(rings).map(|r| Path::polygon(&r)).collect();
    // Parts of the outline that go over each other become one area.
    let regions: Vec<Region> = region::union(&crate::actions::fill_regions(&o, &h), &[]);
    let min_area = c.detail_mm * c.detail_mm;
    let big = |r: &[Point]| !c.tiny || region::ring_signed_area(r).abs() >= min_area;
    let mut out = Path::new();
    let mut out_holes = Vec::new();
    for r in regions.iter().filter(|r| big(&r.outer)) {
        out.cmds.extend(curves(&r.outer, true, c).cmds);
        out_holes.extend(r.holes.iter().filter(|h| big(h)).map(|h| curves(h, true, c)));
    }
    (!out.cmds.is_empty()).then_some((out, out_holes))
}

/// The cleaned kind, or `None` for kinds that do not change and when
/// nothing would stay.
pub fn clean_kind(kind: &ObjectKind, c: &Cleanup) -> Option<ObjectKind> {
    match kind {
        ObjectKind::Running { path, params } => Some(ObjectKind::Running {
            path: cleaned(path, c)?,
            params: params.clone(),
        }),
        ObjectKind::Vector { path, style } => Some(ObjectKind::Vector {
            path: cleaned(path, c)?,
            style: *style,
        }),
        ObjectKind::Fill { outer, holes, params } => {
            let (outer, holes) = clean_fill(outer, holes, c)?;
            Some(ObjectKind::Fill {
                outer,
                holes,
                params: params.clone(),
            })
        }
        _ => None,
    }
}

fn nodes(kinds: impl Iterator<Item = ObjectKind>) -> usize {
    kinds.map(|k| k.paths().map(node_count).sum::<usize>()).sum()
}

impl AppState {
    /// The selected objects that Clean Up can change.
    fn cleanup_targets(&self) -> Vec<DesignObject> {
        let d = self.doc.design();
        self.sel()
            .into_iter()
            .filter_map(|id| d.object(id))
            .filter(|o| can_clean(&o.kind))
            .cloned()
            .collect()
    }

    pub fn open_cleanup(&mut self) {
        let originals = self.cleanup_targets();
        if originals.is_empty() {
            self.message = tr("Select lines, fills or vector objects.").into();
            return;
        }
        self.drag_counter += 1;
        self.cleanup_dialog = Some(CleanupDialog {
            settings: self.cleanup_settings,
            originals,
            key: MergeKey::new(format!("cleanup-{}", self.drag_counter)),
            applied: None,
        });
    }

    /// Shows the result of the current settings on the canvas.
    pub fn preview_cleanup(&mut self, d: &mut CleanupDialog) {
        if d.applied == Some(d.settings) {
            return;
        }
        let (originals, s) = (d.originals.clone(), d.settings);
        self.doc.edit_merged(tr("Clean Up"), d.key.clone(), |dd| {
            for o in &originals {
                let k = clean_kind(&o.kind, &s).unwrap_or_else(|| o.kind.clone());
                if let Some(obj) = dd.object_mut(o.id) {
                    obj.kind = k;
                }
            }
        });
        d.applied = Some(d.settings);
    }

    /// OK (`keep`) or Cancel.
    pub fn close_cleanup(&mut self, mut d: CleanupDialog, keep: bool) {
        self.cleanup_settings = d.settings;
        if keep {
            self.preview_cleanup(&mut d);
            self.doc.end_merge();
        } else if d.applied.is_some() {
            self.doc.end_merge();
            self.undo();
        }
    }
}

/// The Clean Up window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.cleanup_dialog.take() else {
        return;
    };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Clean Up")).id(egui::Id::new("Clean Up")),
        ctx,
        egui::Id::new("Clean Up"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let s = &mut d.settings;
        egui::Grid::new("cleanup")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Detail size")).on_hover_text(tr(
                    "Loops, spikes and zig-zags smaller than this go. Larger: smoother lines, fewer details.",
                ));
                ui.add(
                    egui::Slider::new(&mut s.detail_mm, 0.1..=5.0)
                        .clamping(egui::SliderClamping::Edits)
                        .logarithmic(true)
                        .suffix(" mm")
                        .fixed_decimals(2),
                );
                ui.end_row();
            });
        ui.checkbox(&mut s.loops, tr("Remove small loops and spikes"))
            .on_hover_text(tr(
                "Where a line crosses itself or goes out and back, the small part goes.",
            ));
        ui.checkbox(&mut s.smooth, tr("Smooth zig-zags"))
            .on_hover_text(tr("Smooths the line over the detail size. Corners stay sharp."));
        ui.checkbox(&mut s.tiny, tr("Remove tiny parts")).on_hover_text(tr(
            "Lines shorter than the detail size, and areas and holes smaller than its square, go.",
        ));
        let before = nodes(d.originals.iter().map(|o| o.kind.clone()));
        let after = nodes(
            d.originals
                .iter()
                .map(|o| clean_kind(&o.kind, &d.settings).unwrap_or_else(|| o.kind.clone())),
        );
        ui.label(crate::i18n::fill("Nodes: {} → {}", &[&before, &after]));
        ui.weak(tr("Satin columns, appliqué and lettering do not change."));
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button(tr("OK")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
            if ui
                .button(tr("Standard"))
                .on_hover_text(tr("The settings when the window first opens."))
                .clicked()
            {
                d.settings = Cleanup::STANDARD;
            }
        });
    });
    if cancel || !open {
        st.close_cleanup(d, false);
    } else if ok {
        st.close_cleanup(d, true);
    } else {
        st.preview_cleanup(&mut d);
        st.cleanup_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_fill_that_goes_over_itself_becomes_one_clean_area() {
        let mut h = Harness::empty();
        // A 20 mm square with a zig-zag top, a spike and a tiny second part.
        let mut pts: Vec<Point> = (0..=100)
            .map(|i| p(f64::from(i) * 0.2, if i % 2 == 0 { 0.1 } else { -0.1 }))
            .collect();
        pts.extend([p(20.0, 20.0), p(10.1, 20.0), p(10.0, 20.8), p(9.9, 20.0), p(0.0, 20.0)]);
        let outer = Path::joined([
            &Path::polygon(&pts),
            &Path::polygon(&[p(30.0, 0.0), p(30.2, 0.0), p(30.2, 0.2)]),
        ]);
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer,
                    holes: vec![],
                    params: Default::default(),
                },
            )
        });
        h.st.selection.set(id);
        let before = h.st.doc.undo_len();
        h.st.open_cleanup();
        let d = h.st.cleanup_dialog.take().unwrap();
        h.st.close_cleanup(d, true);
        assert_eq!(h.st.doc.undo_len(), before + 1);
        assert_eq!(h.st.doc.undo_label(), Some("Clean Up"));
        let ObjectKind::Fill { outer, holes, .. } = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        let regions = crate::actions::fill_regions(outer, holes);
        assert_eq!(regions.len(), 1, "the tiny part went");
        let bb = regions[0].bbox().unwrap();
        assert!(bb.max.y < 20.1 && bb.min.y > -0.15, "the spike went: {bb:?}");
        assert!((regions[0].area() - 400.0).abs() < 4.0, "{}", regions[0].area());
        assert!(node_count(outer) < 20, "{}", node_count(outer));
    }

    #[test]
    fn cancel_goes_back_and_columns_do_not_change() {
        let mut h = Harness::empty();
        let pts: Vec<Point> = (0..=50)
            .map(|i| p(f64::from(i) * 0.2, if i % 2 == 0 { 0.1 } else { -0.1 }))
            .collect();
        let line = ObjectKind::Running {
            path: crate::actions::path_of(&pts, false),
            params: Default::default(),
        };
        let col = ObjectKind::Satin {
            geometry: emb_model::SatinGeometry::from_centerline(
                &crate::actions::path_of(&[p(0.0, 10.0), p(20.0, 10.0)], false),
                3.0,
            ),
            params: Default::default(),
        };
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            vec![d.add_object(t, line.clone()), d.add_object(t, col.clone())]
        });
        h.st.select_all();
        h.st.open_cleanup();
        let mut d = h.st.cleanup_dialog.take().unwrap();
        h.st.preview_cleanup(&mut d);
        let design = h.st.doc.design();
        assert_ne!(design.object(ids[0]).unwrap().kind, line);
        assert_eq!(design.object(ids[1]).unwrap().kind, col);
        d.settings.detail_mm = 1.0;
        h.st.preview_cleanup(&mut d);
        h.st.close_cleanup(d, false);
        assert_eq!(h.st.doc.design().object(ids[0]).unwrap().kind, line);
    }
}
