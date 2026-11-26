//! Test Swatches (P22 Milestone 3, ADR 0170): Design > Output > Test
//! Swatches… makes copies of the selected objects in a grid, with one
//! setting in steps from one value to another and the value under each
//! copy (`emb_letter::swatch`). The result is a new design tab, so the
//! design of the user does not change. A grid larger than the hoop goes on
//! more hoopings (ADR 0032).

use eframe::egui;
use emb_letter::sheet::LABEL_FONT;
use emb_letter::swatch::{self, SwatchSetting, SwatchSpec};
use emb_model::{FontRef, HoopSelection, ObjectId};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The Test Swatches window.
#[derive(Debug, Clone, Default)]
pub struct SwatchWindow {
    pub open: bool,
    pub setting: SwatchSetting,
    pub from: f64,
    pub to: f64,
    pub copies: usize,
    /// The grid of the last settings: (key, columns, rows, width, height, fits).
    preview: Option<(PreviewKey, usize, usize, f64, f64, bool)>,
}

type PreviewKey = (SwatchSetting, u64, u64, usize, u64, Vec<ObjectId>);

impl SwatchWindow {
    fn spec(&self, field: Option<(f64, f64)>) -> SwatchSpec {
        SwatchSpec {
            setting: self.setting,
            from: self.from,
            to: self.to,
            copies: self.copies,
            field,
        }
    }
}

/// The value of the first selected object that has the setting.
fn current(st: &AppState, setting: SwatchSetting) -> Option<f64> {
    let d = st.doc.design();
    st.sel()
        .into_iter()
        .filter_map(|id| d.object(id))
        .find_map(|o| setting.value_in(&o.kind))
}

/// Start values round the value `v` of the design.
fn start_values(setting: SwatchSetting, v: f64) -> (f64, f64) {
    let round = |x: f64| (x * 20.0).round() / 20.0;
    match setting {
        SwatchSetting::Angle => (v - 45.0, v + 45.0),
        SwatchSetting::Lean => (v, v + 15.0),
        SwatchSetting::BandOverlap => (0.0, 0.6),
        SwatchSetting::Underlay => (0.0, 4.0),
        SwatchSetting::PullCompensation => (0.0, round((v * 2.0).max(0.3))),
        _ => (round(v * 0.75), round(v * 1.25)),
    }
}

/// The settings that the selection has, in the order of the list.
fn settings_of_selection(st: &AppState) -> Vec<SwatchSetting> {
    SwatchSetting::ALL
        .into_iter()
        .filter(|s| current(st, *s).is_some())
        .collect()
}

impl AppState {
    pub fn open_test_swatches(&mut self) {
        let settings = settings_of_selection(self);
        let Some(&first) = settings.first() else {
            self.message = tr("Select fills, satin columns, lines or lettering.").into();
            return;
        };
        let w = &self.swatch_window;
        let setting = if settings.contains(&w.setting) {
            w.setting
        } else {
            first
        };
        self.set_swatch_setting(setting);
        self.swatch_window.copies = if self.swatch_window.copies == 0 {
            swatch::DEFAULT_COPIES
        } else {
            self.swatch_window.copies
        };
        self.swatch_window.open = true;
    }

    /// Chooses the setting, with start values round its value in the design.
    pub fn set_swatch_setting(&mut self, setting: SwatchSetting) {
        let v = current(self, setting).unwrap_or(0.0);
        let (from, to) = start_values(setting, v);
        let w = &mut self.swatch_window;
        w.setting = setting;
        w.from = from;
        w.to = to;
    }

    /// Makes the swatches as a new design tab.
    pub fn make_test_swatches(&mut self) -> Result<(), String> {
        let field = self.hoop_field();
        let spec = self.swatch_window.spec(field);
        let label = self.fonts.get(&FontRef::embroidery(LABEL_FONT, ""));
        let units = self.settings.units;
        let made = swatch::swatches(
            self.doc.design(),
            &self.sel(),
            &spec,
            label.as_ref().map(|l| l.source()),
            &|v| units.fmt_fine(v),
        )?;
        let mut d = made.design;
        let mut message = fill(
            "{} copies, {} × {}.",
            &[&made.values.len(), &units.fmt(made.size.0), &units.fmt(made.size.1)],
        );
        if !made.fits
            && let HoopSelection::Preset { id } = &d.hoop.selection
            && let Some(hoop) = self.hoops.get(id).cloned()
            && let Some(layout) =
                emb_model::HoopingLayout::auto(&d, &hoop, emb_model::hooping::DEFAULT_OVERLAP_MM, self.margin_mm)
        {
            message = fill(
                "{} copies, {} × {}: larger than the hoop, on {} hoopings (Hooping tab).",
                &[
                    &made.values.len(),
                    &units.fmt(made.size.0),
                    &units.fmt(made.size.1),
                    &layout.hoopings.len(),
                ],
            );
            d.hooping = Some(layout);
        }
        self.start_design(d);
        self.message = message;
        Ok(())
    }
}

/// The Test Swatches window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.swatch_window.open {
        return;
    }
    let settings = settings_of_selection(st);
    if settings.is_empty() {
        st.swatch_window.open = false;
        return;
    }
    // The grid of the settings now (no labels; their height is about the
    // same for each value).
    let field = st.hoop_field();
    let key: PreviewKey = {
        let w = &st.swatch_window;
        (
            w.setting,
            w.from.to_bits(),
            w.to.to_bits(),
            w.copies,
            st.doc.state_id(),
            st.sel(),
        )
    };
    if st.swatch_window.preview.as_ref().is_none_or(|p| p.0 != key) {
        let spec = st.swatch_window.spec(field);
        let grid = swatch::swatches(st.doc.design(), &st.sel(), &spec, None, &|v| format!("{v}"))
            .ok()
            .map(|s| (s.columns, s.rows, s.size.0, s.size.1, s.fits));
        st.swatch_window.preview = grid.map(|(c, r, w, h, f)| (key, c, r, w, h, f));
    }
    let units = st.settings.units;
    let mut open = true;
    let (mut make, mut close) = (false, false);
    let mut new_setting = None;
    crate::dock::docked(
        egui::Window::new(tr("Test Swatches")).id(egui::Id::new("Test Swatches")),
        ctx,
        egui::Id::new("Test Swatches"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let w = &mut st.swatch_window;
        egui::Grid::new("swatch_settings")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Setting"));
                egui::ComboBox::from_id_salt("swatch_setting")
                    .selected_text(tr(w.setting.name()))
                    .show_ui(ui, |ui| {
                        for s in &settings {
                            if ui.selectable_label(*s == w.setting, tr(s.name())).clicked() {
                                new_setting = Some(*s);
                            }
                        }
                    });
                ui.end_row();
                if w.setting != SwatchSetting::Underlay {
                    let (lo, hi) = w.setting.range();
                    let suffix = if w.setting.is_length() { " mm" } else { "°" };
                    for (name, v) in [(tr("From"), &mut w.from), (tr("To"), &mut w.to)] {
                        ui.label(name);
                        ui.add(
                            crate::workspace::drag(v)
                                .range(lo..=hi)
                                .speed(if w.setting.is_length() { 0.01 } else { 1.0 })
                                .fixed_decimals(if w.setting.is_length() { 2 } else { 0 })
                                .clamp_existing_to_range(false)
                                .suffix(suffix),
                        );
                        ui.end_row();
                    }
                    ui.label(tr("Copies"));
                    ui.add(crate::workspace::drag(&mut w.copies).range(2..=swatch::MAX_COPIES));
                    ui.end_row();
                }
            });
        let spec = w.spec(field);
        let labels: Vec<String> = spec
            .values()
            .iter()
            .map(|v| swatch::label_text(w.setting, *v, &|x| units.fmt_fine(x)))
            .collect();
        ui.label(labels.join(", "));
        match &w.preview {
            Some((_, c, r, gw, gh, fits)) => {
                ui.label(fill(
                    "A grid of {} × {} copies, about {} × {}.",
                    &[c, r, &units.fmt(*gw), &units.fmt(*gh)],
                ));
                if !fits {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        tr("Larger than the hoop: the swatches go on more hoopings."),
                    );
                }
            }
            None => {
                ui.label(tr("The selected objects have no size."));
            }
        }
        ui.weak(tr("The swatches are a new design: your design does not change."));
        ui.separator();
        ui.horizontal(|ui| {
            make = ui.button(tr("Make Swatches")).clicked();
            close = ui.button(tr("Close")).clicked();
        });
    });
    if let Some(s) = new_setting {
        st.set_swatch_setting(s);
    }
    if make {
        match st.make_test_swatches() {
            Ok(()) => st.swatch_window.open = false,
            Err(e) => st.message = e,
        }
    } else if close || !open {
        st.swatch_window.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_swatches_are_a_new_design_and_the_design_does_not_change() {
        let mut h = Harness::new();
        let d = h.st.doc.design().clone();
        let fill = d
            .objects()
            .iter()
            .find(|o| matches!(o.kind, emb_model::ObjectKind::Fill { .. }))
            .expect("the samples have a fill")
            .id;
        h.st.selection.set(fill);
        h.st.open_test_swatches();
        assert!(h.st.swatch_window.open);
        h.st.set_swatch_setting(SwatchSetting::Angle);
        h.st.swatch_window.copies = 3;
        let before = emb_model::project::to_string(h.st.doc.design());
        h.st.make_test_swatches().unwrap();
        // A new tab with three copies (and their labels).
        let fills: Vec<f64> =
            h.st.doc
                .design()
                .objects()
                .iter()
                .filter_map(|o| match &o.kind {
                    emb_model::ObjectKind::Fill { params, .. } => Some(params.angle_deg),
                    _ => None,
                })
                .collect();
        assert_eq!(fills.len(), 3, "{fills:?}");
        assert!(fills[0] < fills[1] && fills[1] < fills[2], "{fills:?}");
        // The design of before is in its tab, as it was.
        h.st.switch_tab(0);
        assert_eq!(emb_model::project::to_string(h.st.doc.design()), before);
    }
}
