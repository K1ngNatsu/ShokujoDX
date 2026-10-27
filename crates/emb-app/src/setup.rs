//! Machine and screen setup: sew times for each machine, the trims of DST
//! and TAP files for each machine (P10 Milestone 6), and the screen
//! calibration (so that 100 % zoom shows the real size). See
//! `docs/design/p3-plan.md` (Milestone 8).

use std::collections::BTreeMap;

use eframe::egui::{self, Color32, Sense, Stroke, Vec2};
use emb_model::{MachineProfile, SewTimes};
use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// The user's trim code for a machine (Machine settings): the number of
/// jumps that make a trim and the move of each, for DST and TAP files.
/// `None`: the value of the machine profile.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrimSettings {
    pub jumps: Option<u32>,
    pub jump_mm: Option<f64>,
}

/// The machine profile with the trim code that the user set for it.
pub fn machine_with(m: &MachineProfile, trims: &BTreeMap<String, TrimSettings>) -> MachineProfile {
    let mut m = m.clone();
    if let Some(t) = trims.get(&m.id) {
        if t.jumps.is_some() {
            m.trim_jumps = t.jumps;
        }
        if t.jump_mm.is_some() {
            m.trim_jump_mm = t.jump_mm;
        }
    }
    m
}

use crate::app::AppState;
use crate::workspace::Settings;

impl AppState {
    /// The machine profile for machine files: the profile of `machine` with
    /// the user's trim code.
    pub fn machine_for_files(&self, machine: Option<&str>) -> Option<MachineProfile> {
        Some(machine_with(self.machines.get(machine?)?, &self.settings.trims))
    }

    /// The sew times of a machine: the ones the user set, else the machine
    /// defaults. `None` for no machine or an unknown one.
    pub fn sew_times_of(&self, machine: Option<&str>) -> Option<SewTimes> {
        let m = self.machines.get(machine?)?;
        Some(
            self.settings
                .sew_times
                .get(&m.id)
                .copied()
                .unwrap_or_else(|| m.sew_times()),
        )
    }
}

/// The Machine settings window: the sew times of the machine of the design,
/// and the trims of its DST and TAP files.
pub fn sew_time_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_sew_times {
        return;
    }
    let mut open = true;
    let machine = st
        .doc
        .design()
        .machine
        .as_deref()
        .and_then(|m| st.machines.get(m))
        .cloned();
    let mut s = st.settings.clone();
    crate::dock::docked(
        egui::Window::new(tr("Machine Settings")).id(egui::Id::new("Machine Settings")),
        ctx,
        egui::Id::new("Machine Settings"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let Some(m) = machine else {
            ui.label(tr("Select a machine for the design first (Design > Machine and Hoop)."));
            return;
        };
        ui.label(crate::i18n::fill(
            "Times for {}. The sew time on the worksheet and in Design info, and the run time of a quote, use them.",
            &[&m.name],
        ));
        let mut t = s.sew_times.get(&m.id).copied().unwrap_or_else(|| m.sew_times());
        egui::Grid::new("sew_times")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Speed"));
                ui.add(
                    egui::DragValue::new(&mut t.stitches_per_minute)
                        .range(50..=2000)
                        .clamp_existing_to_range(false)
                        .suffix(" stitches/min"),
                );
                ui.end_row();
                for (label, v, help) in [
                    (
                        tr("Color change"),
                        &mut t.color_change_s,
                        tr("Time to change the thread. A multi-needle machine needs only a few seconds."),
                    ),
                    (tr("Trim"), &mut t.trim_s, tr("Time for one trim.")),
                    (
                        tr("Stop"),
                        &mut t.stop_s,
                        tr("Time for a stop, for example to put on appliqué fabric."),
                    ),
                ] {
                    ui.label(label).on_hover_text(help);
                    ui.add(
                        egui::DragValue::new(v)
                            .range(0.0..=600.0)
                            .clamp_existing_to_range(false)
                            .speed(0.5)
                            .suffix(" s"),
                    );
                    ui.end_row();
                }
                // The run time of an order (Design > Quote, ADR 0064).
                ui.label(tr("Heads"))
                    .on_hover_text(tr("The pieces that the machine sews at a time."));
                ui.add(
                    egui::DragValue::new(&mut t.heads)
                        .range(1..=emb_model::machine::MAX_HEADS)
                        .clamp_existing_to_range(false),
                );
                ui.end_row();
                for (label, v, help) in [
                    (
                        tr("Hoop one piece"),
                        &mut t.hoop_s,
                        tr("Time to hoop one piece, and to take it out of the hoop after it sews."),
                    ),
                    (
                        tr("Frame change"),
                        &mut t.frame_change_s,
                        tr("Time to take the hoops off the machine and put the next hoops on."),
                    ),
                ] {
                    ui.label(label).on_hover_text(help);
                    ui.add(
                        egui::DragValue::new(v)
                            .range(0.0..=3600.0)
                            .clamp_existing_to_range(false)
                            .speed(0.5)
                            .suffix(" s"),
                    );
                    ui.end_row();
                }
            });
        ui.checkbox(
            &mut t.hoop_while_sewing,
            tr("Hoop the next pieces while the machine sews"),
        )
        .on_hover_text(tr(
            "You have a second set of hoops. The run time of an order uses this.",
        ));
        if t != m.sew_times() {
            s.sew_times.insert(m.id.clone(), t);
        } else {
            s.sew_times.remove(&m.id);
        }
        if s.sew_times.contains_key(&m.id) && ui.button(tr("Reset the times to the machine defaults")).clicked() {
            s.sew_times.remove(&m.id);
        }
        ui.separator();
        ui.strong(tr("Trims in DST and TAP files"));
        ui.label(tr(
            "These formats have no trim code: a few short jumps tell the machine to cut the thread. \
                 Change them only if your machine does not cut, or cuts too often.",
        ));
        let base = machine_with(&m, &BTreeMap::new());
        let mut t = s.trims.get(&m.id).copied().unwrap_or_default();
        let mut jumps = t.jumps.or(base.trim_jumps).unwrap_or(3);
        let mut jump_mm = t.jump_mm.or(base.trim_jump_mm).unwrap_or(0.4);
        egui::Grid::new("trims")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Jumps for a trim"));
                if ui
                    .add(
                        egui::DragValue::new(&mut jumps)
                            .range(emb_model::machine::TRIM_JUMPS_RANGE)
                            .clamp_existing_to_range(false),
                    )
                    .changed()
                {
                    t.jumps = Some(jumps);
                }
                ui.end_row();
                ui.label(tr("Length of each jump"));
                if ui
                    .add(
                        egui::DragValue::new(&mut jump_mm)
                            .range(emb_model::machine::TRIM_JUMP_MM_RANGE)
                            .clamp_existing_to_range(false)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .suffix(" mm"),
                    )
                    .changed()
                {
                    t.jump_mm = Some(jump_mm);
                }
                ui.end_row();
            });
        if t == TrimSettings::default() {
            s.trims.remove(&m.id);
        } else {
            s.trims.insert(m.id.clone(), t);
        }
        if s.trims.contains_key(&m.id) && ui.button(tr("Reset the trims to the machine defaults")).clicked() {
            s.trims.remove(&m.id);
        }
    });
    st.set_settings(s);
    if !open {
        st.show_sew_times = false;
    }
}

/// The screen calibration part of the Options window: a line that must be
/// 100 mm long on the screen.
pub fn calibration_ui(ui: &mut egui::Ui, s: &mut Settings) {
    ui.strong(tr("Screen calibration"));
    ui.label(tr(
        "Hold a ruler to the screen. Change the value until the line is 100 mm long, or type the length you measure.",
    ));
    let points = 100.0 * crate::canvas::POINTS_PER_MM * s.screen_calibration;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(points as f32 + 2.0, 18.0), Sense::hover());
    let y = rect.center().y;
    let color = ui.visuals().text_color();
    let (a, b) = (rect.left() + 1.0, rect.left() + 1.0 + points as f32);
    ui.painter()
        .line_segment([egui::pos2(a, y), egui::pos2(b, y)], Stroke::new(2.0, color));
    for x in [a, b] {
        ui.painter().line_segment(
            [egui::pos2(x, y - 7.0), egui::pos2(x, y + 7.0)],
            Stroke::new(1.5, color),
        );
    }
    ui.painter().text(
        rect.center() - Vec2::new(0.0, 2.0),
        egui::Align2::CENTER_BOTTOM,
        "100 mm",
        egui::FontId::proportional(11.0),
        Color32::GRAY,
    );
    ui.horizontal(|ui| {
        ui.label(tr("Scale"));
        ui.add(
            egui::DragValue::new(&mut s.screen_calibration)
                .range(0.5..=2.0)
                .clamp_existing_to_range(false)
                .speed(0.002)
                .fixed_decimals(3),
        );
        let id = ui.id().with("measured");
        let mut measured: f64 = ui.data_mut(|d| *d.get_temp_mut_or(id, 100.0));
        ui.label(tr("Measured"));
        ui.add(
            egui::DragValue::new(&mut measured)
                .range(20.0..=400.0)
                .clamp_existing_to_range(false)
                .suffix(" mm"),
        );
        ui.data_mut(|d| d.insert_temp(id, measured));
        if ui.button(tr("Apply")).clicked() {
            s.screen_calibration = calibrated(s.screen_calibration, measured);
            ui.data_mut(|d| d.insert_temp(id, 100.0));
        }
        if ui.button(tr("Reset")).clicked() {
            s.screen_calibration = 1.0;
        }
    });
}

/// The new scale when the 100 mm line measures `measured` mm.
pub fn calibrated(scale: f64, measured: f64) -> f64 {
    (scale * 100.0 / measured.max(1.0)).clamp(0.5, 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn sew_times_from_the_machine_or_the_user() {
        let mut h = Harness::new();
        let id = h.st.doc.design().machine.clone();
        let t = h.st.sew_times_of(id.as_deref()).unwrap();
        assert_eq!(t.stitches_per_minute, 800, "the Topaz 20 speed");
        let before = h.st.design_info().minutes.unwrap();
        let mut s = h.st.settings.clone();
        s.sew_times.insert(
            id.clone().unwrap(),
            SewTimes {
                stitches_per_minute: 400,
                ..t
            },
        );
        h.st.set_settings(s);
        assert_eq!(h.st.sew_times_of(id.as_deref()).unwrap().stitches_per_minute, 400);
        assert!(h.st.sew_times_of(None).is_none());
        // The info uses the user times at once; it is longer at half speed.
        assert!(h.st.design_info().minutes.unwrap() > before);
    }

    #[test]
    fn calibration() {
        assert!((calibrated(1.0, 90.0) - 100.0 / 90.0).abs() < 1e-12);
        assert_eq!(calibrated(1.0, 10.0), 2.0);
        crate::canvas::set_calibration(1.25);
        let v = crate::canvas::CanvasView::default();
        assert!((v.scale() - 1.25 * crate::canvas::POINTS_PER_MM).abs() < 1e-12);
        crate::canvas::set_calibration(1.0);
    }

    #[test]
    fn windows_draw() {
        let mut h = Harness::new();
        h.st.show_sew_times = true;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| {
            sew_time_window(&ui.ctx().clone(), st);
            let mut s = st.settings.clone();
            calibration_ui(ui, &mut s);
        });
        out.textures_delta.clear();
        assert!(h.st.show_sew_times);
    }
}
