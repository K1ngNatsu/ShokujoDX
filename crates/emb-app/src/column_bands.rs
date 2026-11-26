//! The Bands window (P22 Milestone 4, ADR 0171): more than one thread
//! across a satin column.
//!
//! An object has one thread, so a column of bands is a group of satin
//! columns with the same rails, one for each band (`SatinParams::band`).
//! Each column sews only its band of the width; the first band also sews
//! the underlay of the whole column. This is the same plan as the tartan
//! and the gradient fills (ADR 0047).

use eframe::egui;
use emb_model::{ColumnBand, ObjectId, ObjectKind, SatinParams, ThreadId};

use crate::app::AppState;
use crate::i18n::tr;

/// The largest number of bands.
pub const MAX_BANDS: usize = 6;

/// The window state.
#[derive(Debug, Clone)]
pub struct ColumnBandsWindow {
    pub open: bool,
    /// The thread of each band, from rail A.
    pub threads: Vec<Option<ThreadId>>,
    /// How far each band goes into the band next to it, mm.
    pub overlap_mm: f64,
}

impl Default for ColumnBandsWindow {
    fn default() -> Self {
        ColumnBandsWindow {
            open: false,
            threads: vec![None, None],
            overlap_mm: 0.3,
        }
    }
}

/// The bands of the column of `id` (all the satin columns with a band in
/// its group), or `id` alone if it is not a band.
pub fn bands_of(d: &emb_model::Design, id: ObjectId) -> Vec<ObjectId> {
    let banded =
        |o: &emb_model::DesignObject| matches!(&o.kind, ObjectKind::Satin { params, .. } if params.band.is_some());
    match d.object(id).filter(|o| banded(o)).and_then(|o| o.group) {
        Some(g) => d
            .objects()
            .iter()
            .filter(|o| o.group == Some(g) && banded(o))
            .map(|o| o.id)
            .collect(),
        None => vec![id],
    }
}

impl AppState {
    /// The selected satin column, if the selection is one satin column or
    /// the bands of one column (then its first band).
    fn selected_satin(&self) -> Option<(ObjectId, SatinParams)> {
        let d = self.doc.design();
        let satin = |id: &ObjectId| {
            let o = d.object(*id)?;
            match &o.kind {
                ObjectKind::Satin { params, .. } => Some((o.id, o.group, params.clone())),
                _ => None,
            }
        };
        if let Some(id) = self.selection.single() {
            return satin(&id).map(|(id, _, p)| (id, p));
        }
        let all: Vec<_> = self.selection.ids().iter().map(satin).collect::<Option<_>>()?;
        let (_, group, _) = all.first()?;
        let one_column = group.is_some() && all.iter().all(|(_, g, p)| g == group && p.band.is_some());
        if !one_column {
            return None;
        }
        all.into_iter()
            .min_by_key(|(_, _, p)| p.band.map_or(0, |b| b.index))
            .map(|(id, _, p)| (id, p))
    }

    /// The other columns of the group of `id` that are bands.
    fn band_layers(&self, id: ObjectId) -> Vec<ObjectId> {
        bands_of(self.doc.design(), id)
            .into_iter()
            .filter(|x| *x != id)
            .collect()
    }

    /// Makes the selected satin column a column of bands: one band for
    /// each thread of `threads`, from rail A, each `overlap_mm` into the
    /// band next to it. The old bands of the column go. The bands are one
    /// group. One undo step.
    pub fn make_bands(&mut self, threads: &[ThreadId], overlap_mm: f64) -> bool {
        let Some((id, _)) = self.selected_satin() else {
            self.message = tr("Select one satin column.").into();
            return false;
        };
        if threads.len() < 2 {
            return false;
        }
        let old = self.band_layers(id);
        let Some(o) = self.doc.design().object(id).cloned() else {
            return false;
        };
        let count = threads.len() as u32;
        let overlap_mm = overlap_mm.max(0.0);
        let ids = self.doc.edit(tr("Bands"), |d| {
            for x in &old {
                d.remove_object(*x);
            }
            let g = d.new_group_id();
            let at = d.index_of(id).unwrap_or(d.objects().len());
            let mut ids = Vec::with_capacity(threads.len());
            for (i, thread) in threads.iter().enumerate() {
                let band = ColumnBand {
                    index: i as u32,
                    count,
                    overlap_mm,
                };
                let target = if i == 0 {
                    id
                } else {
                    d.insert_object(at + i, *thread, o.kind.clone())
                };
                if let Some(obj) = d.object_mut(target) {
                    obj.thread = *thread;
                    obj.group = Some(g);
                    if let ObjectKind::Satin { params, .. } = &mut obj.kind {
                        params.band = Some(band);
                    }
                }
                ids.push(target);
            }
            ids
        });
        self.selection.set_all(ids);
        true
    }

    /// Makes the selected column of bands one column of one thread again:
    /// the other bands go. One undo step.
    pub fn remove_bands(&mut self) -> bool {
        let Some((id, params)) = self.selected_satin() else {
            return false;
        };
        if params.band.is_none() {
            return false;
        }
        let old = self.band_layers(id);
        self.doc.edit(tr("Remove bands"), |d| {
            for x in &old {
                d.remove_object(*x);
            }
            if let Some(obj) = d.object_mut(id) {
                obj.group = None;
                if let ObjectKind::Satin { params, .. } = &mut obj.kind {
                    params.band = None;
                }
            }
        });
        self.selection.set(id);
        true
    }

    /// The bands of the selected column, in the window.
    fn load_bands(&mut self) {
        let Some((id, params)) = self.selected_satin() else {
            return;
        };
        let Some(band) = params.band else { return };
        let d = self.doc.design();
        let mut threads = vec![None; band.count.max(1) as usize];
        for x in std::iter::once(id).chain(self.band_layers(id)) {
            if let Some(o) = d.object(x)
                && let ObjectKind::Satin { params, .. } = &o.kind
                && let Some(b) = params.band
                && let Some(slot) = threads.get_mut(b.index as usize)
            {
                *slot = Some(o.thread);
            }
        }
        let w = &mut self.column_bands;
        w.threads = threads;
        w.overlap_mm = band.overlap_mm;
    }
}

/// The Bands window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.column_bands.open {
        return;
    }
    let mut open = true;
    let threads = st.doc.design().threads.clone();
    let selected = st.selected_satin();
    crate::dock::docked(
        egui::Window::new(tr("Bands")).id(egui::Id::new("ColumnBands")),
        ctx,
        egui::Id::new("ColumnBands"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(360.0)
    .show(ctx, |ui| {
        ui.label(tr(
            "More than one thread across a satin column: a band of the width for each thread, from the first side of the column.",
        ));
        if selected.as_ref().is_some_and(|(_, p)| p.band.is_some())
            && ui.button(tr("Use the bands of the selected column")).clicked()
        {
            st.load_bands();
        }
        ui.separator();
        bands_ui(ui, st, &threads, selected.as_ref());
    });
    st.column_bands.open = open;
}

fn bands_ui(
    ui: &mut egui::Ui,
    st: &mut AppState,
    threads: &[emb_model::Thread],
    selected: Option<&(ObjectId, SatinParams)>,
) {
    let selected_thread = selected
        .and_then(|(id, _)| st.doc.design().object(*id))
        .map(|o| o.thread);
    let w = &mut st.column_bands;
    // Bands with no thread yet get the thread of the column, then the next threads.
    for (i, t) in w.threads.iter_mut().enumerate() {
        if t.is_none() {
            *t = if i == 0 {
                selected_thread.or(threads.first().map(|t| t.id))
            } else {
                threads.get(i.min(threads.len().saturating_sub(1))).map(|t| t.id)
            };
        }
    }
    let mut remove = None;
    let n = w.threads.len();
    for (i, t) in w.threads.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(crate::i18n::fill("Band {}", &[&(i + 1)]));
            egui::ComboBox::from_id_salt(("band_thread", i))
                .selected_text(
                    t.and_then(|id| threads.iter().find(|x| x.id == id))
                        .map_or(tr("(choose)").to_string(), |x| x.name.clone()),
                )
                .show_ui(ui, |ui| {
                    for x in threads {
                        ui.selectable_value(t, Some(x.id), &x.name);
                    }
                });
            if n > 2 && ui.small_button(tr("Remove")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        w.threads.remove(i);
    }
    if w.threads.len() < MAX_BANDS && ui.button(tr("Add band")).clicked() {
        w.threads.push(None);
    }
    ui.horizontal(|ui| {
        ui.label(tr("Overlap")).on_hover_text(tr(
            "How far each band goes into the band next to it. A small overlap keeps the fabric from showing between the bands.",
        ));
        ui.add(
            crate::workspace::slider(&mut w.overlap_mm, 0.0..=1.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" mm")
                .fixed_decimals(2),
        );
    });
    ui.weak(tr(
        "The bands have equal widths. The first band sews the underlay of the whole column, and pull compensation goes only out at the two sides of the column.",
    ));
    let picks: Vec<ThreadId> = w.threads.iter().filter_map(|t| *t).collect();
    let overlap = w.overlap_mm;
    let ok = selected.is_some() && picks.len() == w.threads.len() && picks.len() >= 2;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(ok, egui::Button::new(tr("Make bands")))
            .on_hover_text(tr(
                "A satin column for each band, with the rails of the selected column.",
            ))
            .on_disabled_hover_text(tr("Select one satin column, and a thread for each band."))
            .clicked()
        {
            st.make_bands(&picks, overlap);
        }
        let banded = selected.is_some_and(|(_, p)| p.band.is_some());
        if ui
            .add_enabled(banded, egui::Button::new(tr("Remove bands")))
            .on_hover_text(tr("The selected column sews all of its width in one thread again."))
            .clicked()
        {
            st.remove_bands();
        }
    });
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::SatinGeometry;

    use super::*;
    use crate::test_util::Harness;

    fn one_column(h: &mut Harness) -> ObjectId {
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: SatinGeometry {
                        rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(40.0, 0.0)),
                        rail_b: Path::new().move_to(Point::new(0.0, 6.0)).line_to(Point::new(40.0, 6.0)),
                        rungs: Vec::new(),
                    },
                    params: SatinParams::default(),
                },
            )
        });
        h.st.selection.set(id);
        id
    }

    #[test]
    fn bands_are_a_group_of_columns_and_one_undo_step() {
        let mut h = Harness::empty();
        let id = one_column(&mut h);
        let red = h.st.doc.design().objects()[0].thread;
        let (blue, gold) = h.st.doc.edit("Threads", |d| {
            (
                d.add_thread("Blue", emb_model::Rgb(0, 0, 200)),
                d.add_thread("Gold", emb_model::Rgb(220, 180, 0)),
            )
        });
        assert!(h.st.make_bands(&[red, blue, gold], 0.3));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 3);
        let g = d.objects()[0].group;
        assert!(g.is_some() && d.objects().iter().all(|o| o.group == g));
        let mut stitched = Vec::new();
        for (i, o) in d.objects().iter().enumerate() {
            let ObjectKind::Satin { params, .. } = &o.kind else {
                panic!()
            };
            assert_eq!(params.band.map(|b| (b.index, b.count)), Some((i as u32, 3)));
            assert_eq!(o.thread, [red, blue, gold][i]);
            let s = emb_stitch::stitch_object(o, Point::ZERO).expect("sews");
            stitched.push(s);
        }
        // The middle band sews only the middle of the width.
        let ys: Vec<f64> = stitched[1].runs.iter().flat_map(|r| &r.points).map(|p| p.y).collect();
        let (lo, hi) = ys.iter().fold((f64::MAX, f64::MIN), |(a, b), &y| (a.min(y), b.max(y)));
        assert!(lo > 1.5 && hi < 4.5, "{lo} {hi}");
        // Make them again with two bands: the old bands go.
        h.st.selection.set(d.objects()[2].id);
        assert!(h.st.make_bands(&[gold, red], 0.0));
        assert_eq!(h.st.doc.design().objects().len(), 2);
        // Remove the bands: one column again.
        assert!(h.st.remove_bands());
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        assert!(matches!(&d.objects()[0].kind, ObjectKind::Satin { params, .. } if params.band.is_none()));
        // Undo: two bands, three bands, the column of one thread.
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 2);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 3);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 1);
        assert_eq!(h.st.doc.design().objects()[0].id, id);
    }

    #[test]
    fn the_lean_of_a_band_is_the_lean_of_all_the_bands() {
        let mut h = Harness::empty();
        let id = one_column(&mut h);
        let red = h.st.doc.design().objects()[0].thread;
        let blue =
            h.st.doc
                .edit("Thread", |d| d.add_thread("Blue", emb_model::Rgb(0, 0, 200)));
        // A column of one thread: only itself.
        assert_eq!(bands_of(h.st.doc.design(), id), [id]);
        assert!(h.st.make_bands(&[red, blue], 0.0));
        let all: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(bands_of(h.st.doc.design(), all[1]), all);
        // Another column in the same group is not a band of it.
        let other = h.st.doc.edit("Other", |d| {
            let g = d.object(id).and_then(|o| o.group);
            let o = d.add_object(
                red,
                ObjectKind::Satin {
                    geometry: SatinGeometry::default(),
                    params: SatinParams::default(),
                },
            );
            if let Some(x) = d.object_mut(o) {
                x.group = g;
            }
            o
        });
        assert!(!bands_of(h.st.doc.design(), id).contains(&other));
    }

    #[test]
    fn the_bands_load_back_into_the_window() {
        let mut h = Harness::empty();
        let id = one_column(&mut h);
        let red = h.st.doc.design().objects()[0].thread;
        let blue =
            h.st.doc
                .edit("Thread", |d| d.add_thread("Blue", emb_model::Rgb(0, 0, 200)));
        assert!(h.st.make_bands(&[blue, red], 0.45));
        h.st.column_bands = ColumnBandsWindow::default();
        h.st.selection.set(id);
        h.st.load_bands();
        assert_eq!(h.st.column_bands.threads, [Some(blue), Some(red)]);
        assert!((h.st.column_bands.overlap_mm - 0.45).abs() < 1e-12);
    }

    #[test]
    fn the_window_shows() {
        let mut h = Harness::empty();
        one_column(&mut h);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        h.st.column_bands.open = true;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| window(ui.ctx(), st));
        out.textures_delta.clear();
        assert!(h.st.column_bands.open);
        assert_eq!(h.st.column_bands.threads.len(), 2);
        assert!(h.st.column_bands.threads.iter().all(Option::is_some));
    }
}
