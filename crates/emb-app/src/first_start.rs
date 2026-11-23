//! The first start (v0.3 plan, Stage 2): the program asks for the user's
//! machine and hoop. Until the user picks one, new designs use the
//! Generic machine. The choice goes in `Settings::machine` and
//! `Settings::hoop`; Design > Machine and Hoop changes it later. The step
//! also asks if the program looks for new versions at the start, when no
//! installer asked (`update.rs`, ADR 0109).

use eframe::egui;

use crate::app::{AppState, GENERIC_MACHINE};
use crate::dock::FitScreen;
use crate::i18n::tr;

/// The open "Which machine do you have?" step.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineStep {
    pub machine: String,
    pub hoop: Option<String>,
    /// Look for new versions at the start (the tick box of the step).
    pub updates: bool,
}

impl MachineStep {
    /// The step with the Generic machine and its first hoop selected.
    pub fn new(st: &AppState) -> MachineStep {
        let mut step = MachineStep {
            machine: GENERIC_MACHINE.into(),
            hoop: None,
            updates: true,
        };
        step.first_hoop(st);
        step
    }

    /// Selects the first hoop of the selected machine.
    fn first_hoop(&mut self, st: &AppState) {
        self.hoop = st.machines.get(&self.machine).and_then(|m| m.hoops.first().cloned());
    }
}

impl AppState {
    /// Keeps the machine and hoop of the step as the user's, and puts the
    /// design on them if it is the empty design of the start.
    pub fn finish_machine_step(&mut self, step: &MachineStep) {
        let mut s = self.settings.clone();
        s.machine = Some(step.machine.clone());
        s.hoop = step.hoop.clone();
        if s.updates.auto.is_none() {
            s.updates.auto = Some(step.updates);
        }
        self.set_settings(s);
        if self.doc.design().objects().is_empty() && self.doc.undo_label().is_none() {
            self.start_on_machine(&step.machine, step.hoop.as_deref());
            self.fit_requested = true;
        }
        self.ask_machine = None;
    }
}

/// The window of the step, if it is open.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut step) = st.ask_machine.clone() else { return };
    let mut done = false;
    let mut skip = false;
    let mut add_own = false;
    egui::Window::new(tr("Welcome to ShokujoDX"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fit_screen(ctx)
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.heading(tr("Which machine do you have?"));
            ui.label(tr(
                "ShokujoDX then writes the files that your machine reads, and shows your hoop. You can change them later in Design > Machine and Hoop.",
            ));
            ui.add_space(8.0);
            egui::Grid::new("first_start_machine").num_columns(2).show(ui, |ui| {
                ui.label(tr("Machine"));
                let name = |id: &str| st.machines.get(id).map_or_else(|| id.to_string(), label);
                let before = step.machine.clone();
                egui::ComboBox::from_id_salt("first_start_machine")
                    .selected_text(name(&step.machine))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        // The Generic machine first, then by brand and name.
                        let mut list: Vec<&emb_model::MachineProfile> = st.machines.machines.iter().collect();
                        list.sort_by_key(|m| (m.id != GENERIC_MACHINE, m.brand.clone().unwrap_or_default(), m.name.clone()));
                        for m in list {
                            ui.selectable_value(&mut step.machine, m.id.clone(), label(m));
                        }
                    });
                if step.machine != before {
                    step.first_hoop(st);
                }
                ui.end_row();
                ui.label(tr("Hoop"));
                let hoops: Vec<String> = st
                    .machines
                    .get(&step.machine)
                    .map(|m| m.hoops.clone())
                    .unwrap_or_default();
                let hoop_name = |id: &str| {
                    st.hoops
                        .get(id)
                        .map_or_else(|| id.to_string(), |h| format!("{}  ({} × {} mm)", h.name, h.width_mm, h.height_mm))
                };
                egui::ComboBox::from_id_salt("first_start_hoop")
                    .selected_text(step.hoop.as_deref().map(hoop_name).unwrap_or_default())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for id in &hoops {
                            ui.selectable_value(&mut step.hoop, Some(id.clone()), hoop_name(id));
                        }
                    });
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.weak(tr(
                "Is your machine not in the list? Use the Generic machine, or a machine of the same brand that reads the same files.",
            ));
            if ui
                .button(tr("Add My Machine…"))
                .on_hover_text(tr("Make a machine of your own: its file formats, hoops, needles and speed."))
                .clicked()
            {
                add_own = true;
            }
            // The browser version does not look for new versions.
            if st.settings.updates.auto.is_none() && crate::update::CHECK_AT_START {
                ui.separator();
                ui.checkbox(&mut step.updates, tr("Look for new versions at the start"));
                ui.weak(crate::update::what_it_does());
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                done = ui.button(tr("Use This Machine")).clicked();
                skip = ui
                    .button(tr("Skip"))
                    .on_hover_text(tr("Use the Generic machine. You can change it later."))
                    .clicked();
            });
        });
    if add_own {
        // The machine that the user saves becomes the choice of the step.
        st.machine_window.open = true;
        st.machine_window.form = Some((crate::my_machines::blank_machine(), None));
        st.machine_window.error = None;
    }
    if done {
        st.finish_machine_step(&step);
    } else if skip {
        // Skip is for the machine; the tick box counts as it shows.
        let mut generic = MachineStep::new(st);
        generic.updates = step.updates;
        st.finish_machine_step(&generic);
    } else {
        st.ask_machine = Some(step);
    }
}

/// "Brand · name", or the name of a machine with no brand.
fn label(m: &emb_model::MachineProfile) -> String {
    match &m.brand {
        Some(b) if !m.name.contains(b.as_str()) => format!("{b} · {}", m.name),
        _ => m.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, HoopSelection, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new())
    }

    #[test]
    fn a_new_install_is_on_the_generic_machine() {
        let st = state();
        assert_eq!(st.doc.design().machine.as_deref(), Some(GENERIC_MACHINE));
        let step = MachineStep::new(&st);
        assert_eq!(step.machine, GENERIC_MACHINE);
        let first = st.machines.get(GENERIC_MACHINE).unwrap().hoops[0].clone();
        assert_eq!(step.hoop, Some(first.clone()));
        assert_eq!(st.doc.design().hoop.selection, HoopSelection::Preset { id: first });
    }

    #[test]
    fn the_choice_is_kept_and_the_empty_design_uses_it() {
        let mut st = state();
        st.ask_machine = Some(MachineStep::new(&st));
        let step = MachineStep {
            machine: "husqvarna_topaz20".into(),
            hoop: Some("square_200x200".into()),
            updates: false,
        };
        st.finish_machine_step(&step);
        assert_eq!(st.settings.updates.auto, Some(false), "the tick box of the step");
        assert!(st.ask_machine.is_none());
        assert_eq!(st.settings.machine.as_deref(), Some("husqvarna_topaz20"));
        assert_eq!(st.settings.hoop.as_deref(), Some("square_200x200"));
        let d = st.doc.design();
        assert_eq!(d.machine.as_deref(), Some("husqvarna_topaz20"));
        assert_eq!(
            d.hoop.selection,
            HoopSelection::Preset {
                id: "square_200x200".into()
            }
        );
        // The next start uses the kept machine and hoop.
        let mut next = state();
        next.start_on_machine(st.settings.machine.as_deref().unwrap(), st.settings.hoop.as_deref());
        assert_eq!(next.doc.design().machine.as_deref(), Some("husqvarna_topaz20"));
        // A design with objects does not change.
        let mut busy = state();
        busy.add_sample_objects();
        // The choice of an installer stays.
        busy.settings.updates.auto = Some(true);
        busy.finish_machine_step(&step);
        assert_eq!(busy.settings.updates.auto, Some(true));
        assert_eq!(busy.doc.design().machine.as_deref(), Some(GENERIC_MACHINE));
        assert_eq!(busy.settings.machine.as_deref(), Some("husqvarna_topaz20"));
    }

    #[test]
    fn the_window_draws() {
        let mut st = state();
        st.ask_machine = Some(MachineStep::new(&st));
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.ask_machine.is_some(), "the step stays open until the user answers");
    }
}
