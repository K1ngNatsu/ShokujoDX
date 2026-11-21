//! My Machines (v0.3 plan, Stage 6, ADR 0099): the user's own machine
//! profiles, for a machine that is not in the library.
//!
//! The user sets the name, the brand, the file formats that the machine
//! reads (and the one for new files), the hoops, the needles, the speed
//! and the longest stitch. The profiles are in `machines.toml` next to the
//! settings file, and the app adds them to the machine catalog at the start.
//! Export writes one machine with the user's hoops that it uses to a file;
//! Import reads such a file (from a friend, for example).

use std::path::PathBuf;

use eframe::egui;
use emb_model::{HoopCatalog, HoopPreset, MachineCatalog, MachineProfile};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The formats that a machine of the user can list: the formats that the
/// program writes.
pub const FORMATS: [&str; 9] = ["pes", "jef", "vp3", "exp", "xxx", "dst", "tbf", "tap", "u01"];

/// The user's own machines: `machines.toml` next to the settings file.
pub fn user_machines_path() -> Option<PathBuf> {
    crate::workspace::settings_path()?
        .parent()
        .map(|d| d.join("machines.toml"))
}

/// The file of an exported machine: the machine and the user's hoops that
/// it uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineShare {
    #[serde(rename = "machine", default)]
    pub machines: Vec<MachineProfile>,
    #[serde(rename = "hoop", default, skip_serializing_if = "Vec::is_empty")]
    pub hoops: Vec<HoopPreset>,
}

/// The state of the My Machines window.
#[derive(Debug, Clone, Default)]
pub struct MachineWindow {
    pub open: bool,
    /// The machine in the form, and the id of the user machine that it
    /// changes (`None`: a new machine).
    pub form: Option<(MachineProfile, Option<String>)>,
    pub error: Option<String>,
}

/// A new machine for the form: a PES machine in a 130 × 180 mm hoop.
pub fn blank_machine() -> MachineProfile {
    MachineProfile {
        id: String::new(),
        name: String::new(),
        brand: None,
        default_format: "pes".into(),
        formats: vec!["pes".into()],
        hoops: vec!["medium_130x180".into()],
        max_stitch_length_mm: 12.0,
        min_stitch_length_mm: 0.3,
        stitches_per_minute: 700,
        trim_gap_mm: None,
        trim_jumps: None,
        trim_jump_mm: None,
        needles: None,
        heads: None,
    }
}

/// A new id for a machine named `name`: "my_" and the name in a-z, 0-9
/// and '_', different from the ids in `taken`.
pub fn new_id(name: &str, taken: &MachineCatalog) -> String {
    let mut slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    slug = slug.trim_matches('_').to_string();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    if slug.is_empty() {
        slug = "machine".into();
    }
    let base = format!("my_{slug}");
    let mut id = base.clone();
    let mut n = 2;
    while taken.get(&id).is_some() {
        id = format!("{base}_{n}");
        n += 1;
    }
    id
}

impl AppState {
    /// Adds the user's machines to the catalog (the ones whose id is not
    /// in it already) and keeps them as the user's.
    pub fn add_user_machines(&mut self, user: MachineCatalog) {
        for m in &user.machines {
            if self.machines.get(&m.id).is_none() {
                self.machines.machines.push(m.clone());
            }
        }
        self.user_machines = user;
    }

    /// Checks machine `m` of the user: a name, formats that the program
    /// writes with the format for new files among them, and hoops of the
    /// catalog.
    fn check_machine(&self, m: &MachineProfile) -> Result<(), String> {
        if m.name.trim().is_empty() {
            return Err(tr("Give the machine a name.").into());
        }
        if m.formats.is_empty() {
            return Err(tr("Select the file formats that the machine reads.").into());
        }
        if !m.formats.contains(&m.default_format) {
            return Err(tr("The format for new files must be one of the formats of the machine.").into());
        }
        if m.hoops.is_empty() {
            return Err(tr("Select the hoops of the machine.").into());
        }
        if let Some(h) = m.hoops.iter().find(|h| self.hoops.get(h).is_none()) {
            return Err(fill("Hoop {} is not in the hoop list.", &[h]));
        }
        MachineCatalog {
            machines: vec![m.clone()],
        }
        .validate()
        .map_err(|e| e.to_string())
    }

    /// Adds machine `m` of the user, or changes the user machine `old`.
    /// Returns its id.
    pub fn save_user_machine(&mut self, mut m: MachineProfile, old: Option<&str>) -> Result<String, String> {
        if m.id.is_empty() {
            m.id = new_id(&m.name, &self.machines);
        }
        self.check_machine(&m)?;
        if let Some(old) = old {
            self.user_machines.machines.retain(|x| x.id != old);
            self.machines.machines.retain(|x| x.id != old);
        } else if self.machines.get(&m.id).is_some() {
            m.id = new_id(&m.name, &self.machines);
        }
        let id = m.id.clone();
        self.user_machines.machines.push(m.clone());
        self.machines.machines.push(m);
        self.write_user_machines();
        Ok(id)
    }

    /// Removes the user machine `id`.
    pub fn delete_user_machine(&mut self, id: &str) {
        self.user_machines.machines.retain(|x| x.id != id);
        self.machines.machines.retain(|x| x.id != id);
        self.write_user_machines();
    }

    /// Writes the user's machines to their file.
    fn write_user_machines(&mut self) {
        // Tests do not write to the settings folder.
        if !self.save_settings {
            return;
        }
        let Some(p) = user_machines_path() else { return };
        let body = toml::to_string(&self.user_machines).unwrap_or_default();
        let text = format!("# My machines (Design > My Machines).\n\n{body}");
        let r = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&p, text));
        if let Err(e) = r {
            self.message = format!("{}: {e}", p.display());
        }
    }

    /// The text of the file that exports user machine `id` with the user's
    /// hoops that it uses.
    pub fn export_machine_text(&self, id: &str) -> Option<String> {
        let m = self.user_machines.get(id)?.clone();
        let hoops = m.hoops.iter().filter_map(|h| self.user_hoops.get(h).cloned()).collect();
        let share = MachineShare {
            machines: vec![m],
            hoops,
        };
        let body = toml::to_string(&share).ok()?;
        Some(format!(
            "# A ShokujoDX machine (Design > My Machines > Import).\n\n{body}"
        ))
    }

    /// Adds the machines and hoops of an exported file. A machine or a
    /// hoop with an id that is in use gets a new id. Returns the number of
    /// machines.
    pub fn import_machines_text(&mut self, text: &str) -> Result<usize, String> {
        let share: MachineShare = toml::from_str(text).map_err(|e| e.to_string())?;
        if share.machines.is_empty() {
            return Err(tr("The file has no machine.").into());
        }
        let mut hoops = self.user_hoops.clone();
        let mut renamed: Vec<(String, String)> = Vec::new();
        for mut h in share.hoops {
            let old = h.id.clone();
            if self.hoops.get(&h.id).is_some() {
                let mut n = 2;
                while self.hoops.get(&format!("{old}_{n}")).is_some() || hoops.get(&format!("{old}_{n}")).is_some() {
                    n += 1;
                }
                h.id = format!("{old}_{n}");
                renamed.push((old, h.id.clone()));
            }
            hoops.hoops.push(h);
        }
        HoopCatalog::from_toml_str(&hoops.to_toml_string()).map_err(|e| e.to_string())?;
        let added_hoops = hoops.hoops.len() != self.user_hoops.hoops.len();
        self.add_user_hoops(hoops);
        if added_hoops {
            self.save_user_hoops();
        }
        let mut n = 0;
        for mut m in share.machines {
            for h in &mut m.hoops {
                if let Some((_, new)) = renamed.iter().find(|(old, _)| old == h) {
                    *h = new.clone();
                }
            }
            m.id = String::new();
            self.save_user_machine(m, None)?;
            n += 1;
        }
        Ok(n)
    }
}

/// The My Machines window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.machine_window.open;
    if !open {
        return;
    }
    let mut edit: Option<MachineProfile> = None;
    let mut delete: Option<String> = None;
    let mut export: Option<String> = None;
    let mut import = false;
    crate::dock::docked(
        egui::Window::new(tr("My Machines")).id(egui::Id::new("My Machines")),
        ctx,
        egui::Id::new("My Machines"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(true)
    .default_width(520.0)
    .show(ctx, |ui| {
        ui.label(tr(
            "Add your own machine when it is not in the list: its file formats, hoops, needles and speed. It shows in the machine list of Design > Machine and Hoop.",
        ));
        ui.add_space(4.0);
        if st.user_machines.machines.is_empty() {
            ui.weak(tr("You have not added a machine."));
        }
        egui::Grid::new("my_machines").striped(true).num_columns(3).show(ui, |ui| {
            for m in &st.user_machines.machines {
                ui.label(&m.name);
                ui.weak(m.default_format.to_uppercase());
                ui.horizontal(|ui| {
                    if ui.small_button(tr("Edit")).clicked() {
                        edit = Some(m.clone());
                    }
                    if ui.small_button(tr("Export…")).clicked() {
                        export = Some(m.id.clone());
                    }
                    if ui.small_button(tr("Delete")).clicked() {
                        delete = Some(m.id.clone());
                    }
                });
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            if st.machine_window.form.is_none() && ui.button(tr("Add My Machine…")).clicked() {
                st.machine_window.form = Some((blank_machine(), None));
                st.machine_window.error = None;
            }
            if ui.button(tr("Import…")).clicked() {
                import = true;
            }
        });
        ui.separator();
        machine_form(ui, st);
    });
    if let Some(m) = edit {
        let id = m.id.clone();
        st.machine_window.form = Some((m, Some(id)));
        st.machine_window.error = None;
    }
    if let Some(id) = delete {
        st.delete_user_machine(&id);
    }
    if let Some(id) = export {
        export_dialog(st, &id);
    }
    if import {
        import_dialog(st);
    }
    st.machine_window.open = open;
}

/// The form that adds or changes a machine of the user.
fn machine_form(ui: &mut egui::Ui, st: &mut AppState) {
    let Some((mut m, replaces)) = st.machine_window.form.clone() else {
        return;
    };
    ui.strong(if replaces.is_some() {
        tr("Change Machine")
    } else {
        tr("New Machine")
    });
    egui::Grid::new("machine_form").num_columns(2).show(ui, |ui| {
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut m.name);
        ui.end_row();
        ui.label(tr("Brand"));
        let mut brand = m.brand.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut brand).changed() {
            m.brand = (!brand.trim().is_empty()).then(|| brand.trim().to_string());
        }
        ui.end_row();
        ui.label(tr("File formats"))
            .on_hover_text(tr("The formats that the machine reads. See its manual."));
        ui.horizontal_wrapped(|ui| {
            for f in FORMATS {
                let mut on = m.formats.iter().any(|x| x == f);
                if ui.checkbox(&mut on, f.to_uppercase()).changed() {
                    if on {
                        m.formats.push(f.to_string());
                    } else {
                        m.formats.retain(|x| x != f);
                    }
                }
            }
        });
        // The format for new files is one of the formats of the machine.
        if !m.formats.contains(&m.default_format)
            && let Some(f) = m.formats.first()
        {
            m.default_format = f.clone();
        }
        ui.end_row();
        ui.label(tr("Format for new files"));
        egui::ComboBox::from_id_salt("machine_default_format")
            .selected_text(m.default_format.to_uppercase())
            .show_ui(ui, |ui| {
                for f in m.formats.clone() {
                    ui.selectable_value(&mut m.default_format, f.clone(), f.to_uppercase());
                }
            });
        ui.end_row();
        ui.label(tr("Needles"));
        let mut needles = u32::from(m.needles.unwrap_or(1));
        if ui
            .add(
                egui::DragValue::new(&mut needles)
                    .range(1..=15)
                    .clamp_existing_to_range(false),
            )
            .changed()
        {
            m.needles = (needles > 1).then(|| needles.min(15) as u8);
        }
        ui.end_row();
        ui.label(tr("Speed"));
        ui.add(
            egui::DragValue::new(&mut m.stitches_per_minute)
                .range(100..=2000)
                .clamp_existing_to_range(false)
                .suffix(tr(" stitches/min")),
        );
        ui.end_row();
        ui.label(tr("Longest stitch"));
        ui.add(
            egui::DragValue::new(&mut m.max_stitch_length_mm)
                .range(1.0..=12.7)
                .speed(0.1)
                .clamp_existing_to_range(false)
                .suffix(" mm"),
        );
        ui.end_row();
    });
    ui.label(tr("Hoops (the sewable field of each hoop that the machine takes):"));
    egui::ScrollArea::vertical()
        .max_height(180.0)
        .id_salt("machine_hoops")
        .show(ui, |ui| {
            for h in &st.hoops.hoops {
                let mut on = m.hoops.contains(&h.id);
                let text = format!("{}  ({} × {} mm)", h.name, h.width_mm, h.height_mm);
                if ui.checkbox(&mut on, text).changed() {
                    if on {
                        m.hoops.push(h.id.clone());
                    } else {
                        m.hoops.retain(|x| x != &h.id);
                    }
                }
            }
        });
    ui.weak(tr("A hoop that is not in the list: add it in Design > My Hoops first."));
    if let Some(e) = &st.machine_window.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    let (mut save, mut cancel) = (false, false);
    ui.horizontal(|ui| {
        save = ui.button(tr("Save")).clicked();
        cancel = ui.button(tr("Cancel")).clicked();
    });
    if cancel {
        st.machine_window.form = None;
        return;
    }
    if save {
        match st.save_user_machine(m.clone(), replaces.as_deref()) {
            Ok(id) => {
                st.machine_window.form = None;
                st.machine_window.error = None;
                st.message = fill("Machine {} saved. Pick it in Design > Machine and Hoop.", &[&m.name]);
                // The first-start step shows the new machine.
                if let Some(step) = &mut st.ask_machine {
                    step.machine = id;
                    step.hoop = m.hoops.first().cloned();
                }
            }
            Err(e) => st.machine_window.error = Some(e),
        }
        return;
    }
    st.machine_window.form = Some((m, replaces));
}

fn export_dialog(st: &mut AppState, id: &str) {
    let Some(text) = st.export_machine_text(id) else { return };
    let name = st.user_machines.get(id).map_or("machine".into(), |m| m.name.clone());
    let Some(p) = crate::dialog::FileDialog::new()
        .set_title(tr("Export Machine"))
        .add_filter(tr("ShokujoDX machine"), &["toml"])
        .set_file_name(format!("{name}.toml"))
        .save_file()
    else {
        return;
    };
    st.message = match std::fs::write(&p, text) {
        Ok(()) => fill("Machine written to {}.", &[&p.display()]),
        Err(e) => format!("{}: {e}", p.display()),
    };
}

fn import_dialog(st: &mut AppState) {
    let Some(p) = crate::dialog::FileDialog::new()
        .set_title(tr("Import Machine"))
        .add_filter(tr("ShokujoDX machine"), &["toml"])
        .pick_file()
    else {
        return;
    };
    let result = std::fs::read_to_string(&p)
        .map_err(|e| e.to_string())
        .and_then(|t| st.import_machines_text(&t));
    st.message = match result {
        Ok(n) => fill("{} machines added from {}.", &[&n, &p.display()]),
        Err(e) => fill("Cannot import {}: {}", &[&p.display(), &e]),
    };
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new())
    }

    #[test]
    fn a_machine_of_the_user_is_added_changed_and_removed() {
        let mut st = state();
        let mut m = blank_machine();
        assert!(st.save_user_machine(m.clone(), None).is_err(), "no name");
        m.name = "Brother SE700".into();
        let id = st.save_user_machine(m.clone(), None).unwrap();
        assert_eq!(id, "my_brother_se700");
        assert!(st.machines.get(&id).is_some());
        // The same name again: a new id.
        let id2 = st.save_user_machine(m.clone(), None).unwrap();
        assert_eq!(id2, "my_brother_se700_2");
        // A change keeps the id.
        let mut changed = st.machines.get(&id).unwrap().clone();
        changed.stitches_per_minute = 710;
        assert_eq!(st.save_user_machine(changed, Some(&id)).unwrap(), id);
        assert_eq!(st.machines.get(&id).unwrap().stitches_per_minute, 710);
        // Bad values are refused.
        let mut bad = st.machines.get(&id).unwrap().clone();
        bad.default_format = "jef".into();
        assert!(st.save_user_machine(bad, Some(&id)).is_err());
        let mut bad = st.machines.get(&id).unwrap().clone();
        bad.hoops = vec!["no_such_hoop".into()];
        assert!(st.save_user_machine(bad, Some(&id)).is_err());
        st.delete_user_machine(&id);
        assert!(st.machines.get(&id).is_none());
        assert_eq!(st.user_machines.machines.len(), 1);
    }

    #[test]
    fn an_exported_machine_comes_back_with_its_hoops() {
        let mut st = state();
        let hoop = HoopPreset {
            id: "my_hoop".into(),
            name: "My hoop 150 x 150".into(),
            width_mm: 150.0,
            height_mm: 150.0,
            ..crate::hoops::blank_hoop()
        };
        st.add_user_hoops(HoopCatalog { hoops: vec![hoop] });
        let mut m = blank_machine();
        m.name = "Club machine".into();
        m.hoops = vec!["my_hoop".into(), "medium_130x180".into()];
        m.needles = Some(6);
        let id = st.save_user_machine(m, None).unwrap();
        let text = st.export_machine_text(&id).unwrap();
        // A friend with no such hoop imports it.
        let mut friend = state();
        assert_eq!(friend.import_machines_text(&text).unwrap(), 1);
        let got = friend.user_machines.machines[0].clone();
        assert_eq!(got.name, "Club machine");
        assert_eq!(got.needles, Some(6));
        assert!(friend.hoops.get("my_hoop").is_some());
        // Again: the hoop id is in use, so the copy gets a new id.
        assert_eq!(friend.import_machines_text(&text).unwrap(), 1);
        let second = friend.user_machines.machines[1].clone();
        assert_eq!(second.hoops[0], "my_hoop_2");
        assert!(friend.import_machines_text("not toml [").is_err());
    }

    #[test]
    fn the_window_draws_with_a_form() {
        let mut st = state();
        st.machine_window.open = true;
        st.machine_window.form = Some((blank_machine(), None));
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.machine_window.open);
    }
}
