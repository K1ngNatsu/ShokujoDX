//! Hoop placement (P7 Milestone 1, ADR 0032): the hoop angle, manual
//! placement or auto centering, auto hoop, My hoops (the hoops that the
//! user has, and their own hoops), auto start and end of the design, and
//! the hoop template.

use std::path::PathBuf;

use eframe::egui;
use emb_geom::Point;
use emb_model::{Anchor, HoopCatalog, HoopPreset, HoopSelection, HoopShape, MergeKey, Rotation};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// The step of the Turn hoop buttons, in degrees.
pub const HOOP_STEP_DEG: f64 = 15.0;

/// The user's own hoops: `hoops.toml` next to the settings file.
pub fn user_hoops_path() -> Option<PathBuf> {
    crate::workspace::settings_path()?
        .parent()
        .map(|d| d.join("hoops.toml"))
}

/// The state of the My hoops and Hoop options windows.
#[derive(Debug, Clone, Default)]
pub struct HoopWindows {
    pub my_hoops: bool,
    pub options: bool,
    /// The hoop in the form of the My hoops window, and the id of the user
    /// hoop that it changes (`None`: a new hoop).
    pub form: Option<(HoopPreset, Option<String>)>,
    pub form_error: Option<String>,
}

/// The places on the design box that a start or end point can use, with
/// their names.
pub(crate) const BOX_PLACES: [((f64, f64), &str); 9] = [
    ((0.0, 0.0), "Top left"),
    ((0.5, 0.0), "Top"),
    ((1.0, 0.0), "Top right"),
    ((0.0, 0.5), "Left"),
    ((0.5, 0.5), "Center of the design"),
    ((1.0, 0.5), "Right"),
    ((0.0, 1.0), "Bottom left"),
    ((0.5, 1.0), "Bottom"),
    ((1.0, 1.0), "Bottom right"),
];

fn anchor_name(a: Option<Anchor>) -> String {
    match a {
        None => tr("Off").into(),
        Some(Anchor::HoopCenter) => tr("Hoop center").into(),
        Some(Anchor::Point { .. }) => tr("A point").into(),
        Some(Anchor::DesignBox { x, y }) => BOX_PLACES
            .iter()
            .find(|((px, py), _)| *px == x && *py == y)
            .map_or_else(|| tr("A place on the design").into(), |(_, n)| tr(n).to_string()),
    }
}

/// A new hoop for the form.
fn blank_hoop() -> HoopPreset {
    HoopPreset {
        id: String::new(),
        name: "My hoop".into(),
        brand: None,
        width_mm: 150.0,
        height_mm: 150.0,
        shape: HoopShape::Rectangle,
        corner_radius_mm: None,
        max_stitch_length_mm: None,
        positions: Vec::new(),
        cap: false,
    }
}

impl AppState {
    /// Adds the user's own hoops to the hoop list.
    pub fn add_user_hoops(&mut self, user: HoopCatalog) {
        for h in &user.hoops {
            if self.hoops.get(&h.id).is_none() {
                self.hoops.hoops.push(h.clone());
            }
        }
        self.user_hoops = user;
    }

    /// Returns `true` if the hoop goes on the machine of the design: it is
    /// in the list of the machine, or it is one of the user's own hoops.
    pub fn hoop_fits_machine(&self, hoop: &str) -> bool {
        let machine = self.doc.design().machine.as_deref().and_then(|m| self.machines.get(m));
        machine.is_none_or(|m| m.hoops.iter().any(|h| h == hoop)) || self.user_hoops.get(hoop).is_some()
    }

    /// Turns the hoop by `delta` degrees (clockwise on the screen).
    pub fn turn_hoop(&mut self, delta: f64) {
        let a = self.doc.design().hoop.angle_deg + delta;
        self.set_hoop_angle(a, None);
    }

    /// Sets the hoop angle, from −180° to 180°. With `key`, one drag is one undo step.
    pub fn set_hoop_angle(&mut self, deg: f64, key: Option<MergeKey>) {
        let mut a = deg.rem_euclid(360.0);
        if a > 180.0 {
            a -= 360.0;
        }
        let a = (a * 10.0).round() / 10.0;
        match key {
            Some(k) => self.doc.edit_merged("Turn hoop", k, |d| d.hoop.angle_deg = a),
            None => self.doc.edit("Turn hoop", |d| d.hoop.angle_deg = a),
        }
        self.message = crate::i18n::fill("Hoop angle {}°", &[&a]);
    }

    /// The hoops that Auto hoop can choose: My hoops, else the hoops of the
    /// machine, else all hoops. Not multi-position hoops: one machine file
    /// sews in one position (they are for Hooping > Auto layout).
    pub(crate) fn auto_hoop_candidates(&self) -> Vec<&HoopPreset> {
        let single = |h: &&HoopPreset| h.positions.is_empty();
        let mine: Vec<&HoopPreset> = self
            .settings
            .my_hoops
            .iter()
            .filter_map(|id| self.hoops.get(id))
            .filter(single)
            .collect();
        if !mine.is_empty() {
            return mine;
        }
        let machine = self.doc.design().machine.as_deref().and_then(|m| self.machines.get(m));
        let of_machine: Vec<&HoopPreset> = machine
            .map(|m| {
                m.hoops
                    .iter()
                    .filter_map(|id| self.hoops.get(id))
                    .filter(single)
                    .collect()
            })
            .unwrap_or_default();
        if !of_machine.is_empty() {
            return of_machine;
        }
        self.hoops.hoops.iter().filter(single).collect()
    }

    /// Selects the smallest hoop that the design fits (inside the safety
    /// margin), with the hoop center at the center of the design. If the
    /// design fits a smaller hoop when it turns 90°, it turns.
    pub fn auto_hoop(&mut self) {
        let d = self.doc.design();
        let Some(e) = d.extents() else {
            self.message = tr("The design is empty.").into();
            return;
        };
        // The size of the design in the hoop frame, for a design rotation.
        let size = |r: Rotation| {
            let mut t = d.clone();
            t.rotation = r;
            t.hoop.center = e.center();
            let out = t.output_transform();
            let corners = [e.min, e.max, Point::new(e.min.x, e.max.y), Point::new(e.max.x, e.min.y)];
            emb_geom::BBox::from_points(corners.map(|p| out.apply(p))).map_or((0.0, 0.0), |b| (b.width(), b.height()))
        };
        // The rotation now, and the other way up (a quarter turn toward upright).
        let other = match d.rotation {
            Rotation::Deg0 => Rotation::Deg90,
            Rotation::Deg90 => Rotation::Deg0,
            Rotation::Deg180 => Rotation::Deg270,
            Rotation::Deg270 => Rotation::Deg180,
        };
        let turns = [d.rotation, other];
        let sizes = turns.map(size);
        let mut best: Option<(&HoopPreset, usize)> = None;
        for h in self.auto_hoop_candidates() {
            for (k, (w, ht)) in sizes.iter().enumerate() {
                if !h.fits(*w, *ht, self.margin_mm) {
                    continue;
                }
                let area = h.width_mm * h.height_mm;
                let better = best.is_none_or(|(b, bk)| {
                    let ba = b.width_mm * b.height_mm;
                    area < ba || (area == ba && k < bk)
                });
                if better {
                    best = Some((h, k));
                }
            }
        }
        let Some((hoop, k)) = best else {
            self.message = crate::i18n::fill(
                "The design ({} × {} mm) is larger than all hoops that Auto hoop can use.",
                &[&format!("{:.0}", sizes[0].0), &format!("{:.0}", sizes[0].1)],
            );
            return;
        };
        let (id, name) = (hoop.id.clone(), hoop.name.clone());
        let rotation = turns[k];
        self.doc.edit("Auto Hoop", |d| {
            d.hoop.selection = HoopSelection::Preset { id };
            d.hoop.center = e.center();
            d.rotation = rotation;
        });
        self.fit_requested = true;
        self.message = if k == 1 {
            format!("Auto hoop: {name}, with the design turned 90°")
        } else {
            format!("Auto hoop: {name}")
        };
    }

    /// Sets the start or the end point of the design.
    pub fn set_start_end(&mut self, end: bool, a: Option<Anchor>) {
        self.doc.edit(if end { "Auto end" } else { "Auto start" }, |d| {
            if end {
                d.start_end.end = a;
            } else {
                d.start_end.start = a;
            }
        });
    }

    /// The sheets of the hoop template. `None` in open-field mode.
    pub fn hoop_template_pages(&mut self) -> Option<Vec<emb_render::page::Page>> {
        let d = self.doc.design();
        let HoopSelection::Preset { id } = &d.hoop.selection else {
            return None;
        };
        let hoop = self.hoops.get(id)?.clone();
        let name = d.name.clone();
        let rings = |margin: f64| -> Vec<Vec<Point>> {
            hoop.outline(Point::ZERO, margin)
                .map(|p| p.flatten(0.2).into_iter().map(|pl| pl.points).collect())
                .unwrap_or_default()
        };
        let spec = emb_render::template::TemplateSpec {
            name,
            hoop_name: format!("{} ({} × {} mm)", hoop.name, hoop.width_mm, hoop.height_mm),
            field: rings(0.0),
            safe: rings(self.margin_mm),
            segments: emb_render::segments(&self.plan_view().plan.stitches),
            paper: self.settings.output.paper,
        };
        Some(emb_render::template::hoop_template(&spec))
    }

    /// Saves the hoop template as a PDF file.
    pub fn hoop_template_dialog(&mut self) {
        let Some(pages) = self.hoop_template_pages() else {
            self.message = tr("Select a hoop first: the template shows the hoop field.").into();
            return;
        };
        let title = format!("{} hoop template", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Print hoop template (PDF)")
            .add_filter("PDF", &["pdf"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{title}.pdf"))
            .save_file()
        else {
            return;
        };
        let n = pages.len();
        match std::fs::write(&p, emb_render::pdf::write(&pages, &title)) {
            Ok(()) => {
                self.message = crate::i18n::fill(
                    "Hoop template written to {} ({} sheet{}). Print it at 100 %.",
                    &[&(p.display()), &n, &(if n == 1 { "" } else { "s" })],
                )
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot write file".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                })
            }
        }
    }

    /// Writes the user's own hoops to their file.
    pub(crate) fn save_user_hoops(&mut self) {
        // Tests do not write to the settings folder.
        if !self.save_settings {
            return;
        }
        let Some(p) = user_hoops_path() else { return };
        let text = format!(
            "# My hoops (Design > My Hoops). Each size is the sewable field in mm.\n\n{}",
            self.user_hoops.to_toml_string()
        );
        let r = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&p, text));
        if let Err(e) = r {
            self.message = format!("{}: {e}", p.display());
        }
    }

    /// Adds or changes a hoop of the user. Returns the error if the hoop is not valid.
    pub fn save_user_hoop(&mut self, mut h: HoopPreset, replaces: Option<String>) -> Result<(), String> {
        if h.shape != HoopShape::RoundedRectangle {
            h.corner_radius_mm = None;
        }
        if h.shape == HoopShape::Circle {
            h.height_mm = h.width_mm;
        }
        h.name = h.name.trim().to_string();
        h.id = match &replaces {
            Some(id) => id.clone(),
            None => {
                let slug: String = h
                    .name
                    .to_ascii_lowercase()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                let base = format!("my_{}", slug.trim_matches('_'));
                let mut id = base.clone();
                let mut n = 2;
                while self.hoops.get(&id).is_some() {
                    id = format!("{base}_{n}");
                    n += 1;
                }
                id
            }
        };
        let mut user = self.user_hoops.clone();
        match user.hoops.iter_mut().find(|u| u.id == h.id) {
            Some(u) => *u = h.clone(),
            None => user.hoops.push(h.clone()),
        }
        user.validate().map_err(|e| e.to_string())?;
        match self.hoops.hoops.iter_mut().find(|u| u.id == h.id) {
            Some(u) => *u = h.clone(),
            None => self.hoops.hoops.push(h.clone()),
        }
        self.user_hoops = user;
        let mut s = self.settings.clone();
        if !s.my_hoops.contains(&h.id) {
            s.my_hoops.push(h.id.clone());
        }
        self.set_settings(s);
        self.save_user_hoops();
        self.message = crate::i18n::fill("Saved the hoop \"{}\"", &[&h.name]);
        Ok(())
    }

    /// Deletes a hoop of the user. A design that uses it shows no hoop.
    pub fn delete_user_hoop(&mut self, id: &str) {
        self.user_hoops.hoops.retain(|h| h.id != id);
        self.hoops.hoops.retain(|h| h.id != id);
        let mut s = self.settings.clone();
        s.my_hoops.retain(|h| h != id);
        self.set_settings(s);
        self.save_user_hoops();
    }

    /// Adds the hoop to My hoops, or takes it out.
    pub fn toggle_my_hoop(&mut self, id: &str) {
        let mut s = self.settings.clone();
        if let Some(i) = s.my_hoops.iter().position(|h| h == id) {
            s.my_hoops.remove(i);
        } else {
            s.my_hoops.push(id.to_string());
        }
        self.set_settings(s);
    }
}

/// The My hoops and Hoop options windows.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    my_hoops_window(ctx, st);
    options_window(ctx, st);
}

fn shape_name(s: HoopShape) -> &'static str {
    match s {
        HoopShape::Rectangle => tr("Rectangle"),
        HoopShape::RoundedRectangle => tr("Rounded rectangle"),
        HoopShape::Oval => tr("Oval"),
        HoopShape::Circle => tr("Circle"),
    }
}

fn my_hoops_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.hoop_windows.my_hoops;
    if !open {
        return;
    }
    let mut toggle: Option<String> = None;
    let mut edit: Option<HoopPreset> = None;
    let mut delete: Option<String> = None;
    crate::dock::docked(
        egui::Window::new(tr("My Hoops")).id(egui::Id::new("My Hoops")),
        ctx,
        egui::Id::new("My Hoops"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(true)
    .default_width(520.0)
    .show(ctx, |ui| {
        ui.label(tr(
            "Mark the hoops that you have. Auto hoop and the hoop list use them first.",
        ));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            egui::Grid::new("my_hoops").striped(true).num_columns(5).show(ui, |ui| {
                ui.strong(tr("I have it"));
                ui.strong(tr("Hoop"));
                ui.strong(tr("Field"));
                ui.strong(tr("Shape"));
                ui.end_row();
                for h in &st.hoops.hoops {
                    let mut mine = st.settings.my_hoops.contains(&h.id);
                    if ui.checkbox(&mut mine, "").changed() {
                        toggle = Some(h.id.clone());
                    }
                    ui.label(&h.name);
                    ui.label(crate::i18n::fill("{} × {} mm", &[&h.width_mm, &h.height_mm]));
                    ui.label(shape_name(h.shape));
                    if st.user_hoops.get(&h.id).is_some() {
                        ui.horizontal(|ui| {
                            if ui.small_button(tr("Edit")).clicked() {
                                edit = Some(h.clone());
                            }
                            if ui.small_button(tr("Delete")).clicked() {
                                delete = Some(h.id.clone());
                            }
                        });
                    } else {
                        ui.weak(tr("Built in"));
                    }
                    ui.end_row();
                }
            });
        });
        ui.separator();
        if st.hoop_windows.form.is_none() && ui.button(tr("New hoop…")).clicked() {
            st.hoop_windows.form = Some((blank_hoop(), None));
            st.hoop_windows.form_error = None;
        }
        hoop_form(ui, st);
    });
    if let Some(id) = toggle {
        st.toggle_my_hoop(&id);
    }
    if let Some(h) = edit {
        let id = h.id.clone();
        st.hoop_windows.form = Some((h, Some(id)));
        st.hoop_windows.form_error = None;
    }
    if let Some(id) = delete {
        st.delete_user_hoop(&id);
    }
    st.hoop_windows.my_hoops = open;
}

/// The form that adds or changes a hoop of the user.
fn hoop_form(ui: &mut egui::Ui, st: &mut AppState) {
    let Some((mut h, replaces)) = st.hoop_windows.form.clone() else {
        return;
    };
    ui.strong(if replaces.is_some() { "Change hoop" } else { "New hoop" });
    ui.label(tr(
        "The size is the sewable field (where the needle can go), not the frame.",
    ));
    let units = st.settings.units;
    egui::Grid::new("hoop_form").num_columns(2).show(ui, |ui| {
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut h.name);
        ui.end_row();
        ui.label(tr("Shape"));
        egui::ComboBox::from_id_salt("hoop_shape")
            .selected_text(shape_name(h.shape))
            .show_ui(ui, |ui| {
                for s in [
                    HoopShape::Rectangle,
                    HoopShape::RoundedRectangle,
                    HoopShape::Oval,
                    HoopShape::Circle,
                ] {
                    ui.selectable_value(&mut h.shape, s, shape_name(s));
                }
            });
        ui.end_row();
        ui.label(if h.shape == HoopShape::Circle {
            "Diameter"
        } else {
            "Width"
        });
        crate::workspace::length_value(ui, &mut h.width_mm, units, 10.0, 2000.0);
        ui.end_row();
        if h.shape != HoopShape::Circle {
            ui.label(tr("Height"));
            crate::workspace::length_value(ui, &mut h.height_mm, units, 10.0, 2000.0);
            ui.end_row();
        }
        if h.shape == HoopShape::RoundedRectangle {
            let mut r = h.corner_radius_mm.unwrap_or(10.0);
            ui.label(tr("Corner Radius"));
            crate::workspace::length_value(ui, &mut r, units, 0.5, 500.0);
            h.corner_radius_mm = Some(r);
            ui.end_row();
        }
    });
    // A multi-position hoop: the field of each position and its center from the hoop center.
    ui.label(tr("Positions (a multi-position hoop; none for a normal hoop):"));
    let mut remove = None;
    egui::Grid::new("hoop_positions").num_columns(5).show(ui, |ui| {
        if !h.positions.is_empty() {
            ui.weak(tr("Center right"));
            ui.weak(tr("Center down"));
            ui.weak(tr("Width"));
            ui.weak(tr("Height"));
            ui.end_row();
        }
        for (i, p) in h.positions.iter_mut().enumerate() {
            crate::workspace::length_value(ui, &mut p.x_mm, units, -1000.0, 1000.0);
            crate::workspace::length_value(ui, &mut p.y_mm, units, -1000.0, 1000.0);
            crate::workspace::length_value(ui, &mut p.width_mm, units, 10.0, 2000.0);
            crate::workspace::length_value(ui, &mut p.height_mm, units, 10.0, 2000.0);
            if ui.small_button(tr("Remove")).clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        h.positions.remove(i);
    }
    if ui.button(tr("Add position")).clicked() {
        let next = h.positions.last().copied().map_or(
            emb_model::HoopPosition {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: h.width_mm,
                height_mm: h.height_mm,
            },
            |p| emb_model::HoopPosition {
                x_mm: p.x_mm + p.width_mm / 2.0,
                ..p
            },
        );
        h.positions.push(next);
    }
    let mut close = false;
    ui.horizontal(|ui| {
        if ui.button(tr("Save")).clicked() {
            match st.save_user_hoop(h.clone(), replaces.clone()) {
                Ok(()) => close = true,
                Err(e) => st.hoop_windows.form_error = Some(e),
            }
        }
        if ui.button(tr("Cancel")).clicked() {
            close = true;
        }
    });
    if let Some(e) = &st.hoop_windows.form_error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    st.hoop_windows.form = if close { None } else { Some((h, replaces)) };
    if close {
        st.hoop_windows.form_error = None;
    }
}

fn options_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.hoop_windows.options;
    if !open {
        return;
    }
    crate::dock::docked(
        egui::Window::new(tr("Hoop Options")).id(egui::Id::new("Hoop Options")),
        ctx,
        egui::Id::new("Hoop Options"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| {
        let d = st.doc.design();
        let (auto_center, angle, se, units) = (d.hoop.auto_center, d.hoop.angle_deg, d.start_end, st.settings.units);
        ui.strong(tr("Placement"));
        let mut ac = auto_center;
        ui.radio_value(&mut ac, false, tr("Manual: the hoop stays where you put it"));
        ui.radio_value(
            &mut ac,
            true,
            tr("Auto centering: the hoop center follows the center of the design"),
        );
        if ac != auto_center {
            st.doc.edit("Hoop placement", |d| d.hoop.auto_center = ac);
        }
        ui.checkbox(&mut st.show_hoop, tr("Show the hoop"));

        ui.add_space(6.0);
        ui.strong(tr("Hoop angle"));
        ui.label(tr(
            "Turn the hoop to fit a design on a slant. The machine file turns the stitches back.",
        ));
        ui.horizontal(|ui| {
            if ui.button(format!("⟲ {HOOP_STEP_DEG}°")).clicked() {
                st.turn_hoop(-HOOP_STEP_DEG);
            }
            let mut a = angle;
            let r = ui.add(
                egui::DragValue::new(&mut a)
                    .range(-180.0..=180.0)
                    .clamp_existing_to_range(false)
                    .speed(0.5)
                    .suffix("°"),
            );
            if r.changed() {
                st.set_hoop_angle(a, Some(MergeKey::new("hoop_angle")));
            }
            if r.drag_stopped() || r.lost_focus() {
                st.doc.end_merge();
            }
            if ui.button(format!("⟳ {HOOP_STEP_DEG}°")).clicked() {
                st.turn_hoop(HOOP_STEP_DEG);
            }
            if ui
                .add_enabled(angle != 0.0, egui::Button::new(tr("Straight")))
                .clicked()
            {
                st.set_hoop_angle(0.0, None);
            }
        });

        ui.add_space(6.0);
        ui.strong(tr("Auto start and end"));
        ui.label(tr(
            "The needle goes to the start point before the first stitch, and to the end point \
                 after the last trim. No stitch sews there.",
        ));
        ui.weak(tr(
            "VP3 files have no jumps: there the start point sets only where the first object \
                 starts, and the needle stays at the last stitch.",
        ));
        egui::Grid::new("start_end").num_columns(2).show(ui, |ui| {
            for (end, a) in [(false, se.start), (true, se.end)] {
                ui.label(if end { "End" } else { "Start" });
                ui.horizontal(|ui| {
                    let mut pick: Option<Option<Anchor>> = None;
                    egui::ComboBox::from_id_salt(("start_end", end))
                        .selected_text(anchor_name(a))
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(a.is_none(), tr("Off")).clicked() {
                                pick = Some(None);
                            }
                            for ((x, y), name) in BOX_PLACES {
                                let v = Anchor::DesignBox { x, y };
                                if ui.selectable_label(a == Some(v), tr(name)).clicked() {
                                    pick = Some(Some(v));
                                }
                            }
                            if ui
                                .selectable_label(a == Some(Anchor::HoopCenter), tr("Hoop center"))
                                .clicked()
                            {
                                pick = Some(Some(Anchor::HoopCenter));
                            }
                            let is_point = matches!(a, Some(Anchor::Point { .. }));
                            if ui.selectable_label(is_point, tr("A point")).clicked() && !is_point {
                                let c = st.doc.design().hoop.center;
                                pick = Some(Some(Anchor::Point { point: c }));
                            }
                        });
                    // A point: its place from the hoop center, as the canvas
                    // shows it (the design rotation turns about the hoop center).
                    if let Some(Anchor::Point { point }) = a {
                        let d = st.doc.design();
                        let (c, rot) = (d.hoop.center, d.rotation_transform());
                        let v = rot.apply(point) - c;
                        let (mut x, mut y) = (v.x, v.y);
                        let rx = crate::workspace::length_value(ui, &mut x, units, -1000.0, 1000.0);
                        let ry = crate::workspace::length_value(ui, &mut y, units, -1000.0, 1000.0);
                        if rx.changed() || ry.changed() {
                            let inv = rot.inverse().unwrap_or(emb_geom::Transform::IDENTITY);
                            let p = Anchor::Point {
                                point: inv.apply(c + Point::new(x, y)),
                            };
                            let key = MergeKey::new(if end { "end_point" } else { "start_point" });
                            st.doc.edit_merged("Auto start and end", key, |d| {
                                if end {
                                    d.start_end.end = Some(p);
                                } else {
                                    d.start_end.start = Some(p);
                                }
                            });
                        }
                        ui.weak(tr("from the hoop center"));
                    }
                    if let Some(v) = pick {
                        st.set_start_end(end, v);
                    }
                });
                ui.end_row();
            }
        });
    });
    st.hoop_windows.options = open;
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path};
    use emb_model::{ObjectKind, Rgb};

    use super::*;
    use crate::test_util::Harness;

    fn add_box(h: &mut Harness, w: f64, ht: f64) {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::from_center_size(Point::new(500.0, 40.0), w, ht)),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            );
        });
    }

    #[test]
    fn auto_hoop_takes_the_smallest_hoop_and_turns_the_design() {
        let mut h = Harness::empty();
        // The Topaz 20 has the 260 × 200 (200 mm across) and the 120 × 120 hoops.
        add_box(&mut h, 100.0, 80.0);
        h.st.auto_hoop();
        let d = h.st.doc.design();
        assert_eq!(
            d.hoop.selection,
            HoopSelection::Preset {
                id: "topaz20_splendid_square".into()
            }
        );
        assert_eq!(d.hoop.center, Point::new(500.0, 40.0));
        // 240 × 180 fits the 200 × 260 field only turned 90°.
        let mut h = Harness::empty();
        add_box(&mut h, 240.0, 180.0);
        h.st.auto_hoop();
        let d = h.st.doc.design();
        assert_eq!(
            d.hoop.selection,
            HoopSelection::Preset {
                id: "topaz20_crown".into()
            }
        );
        assert_eq!(d.rotation, Rotation::Deg90);
        // My hoops win over the hoops of the machine.
        let mut s = h.st.settings.clone();
        s.my_hoops = vec!["extra_large_360x200".into()];
        h.st.set_settings(s);
        h.st.auto_hoop();
        let d = h.st.doc.design();
        assert_eq!(
            d.hoop.selection,
            HoopSelection::Preset {
                id: "extra_large_360x200".into()
            }
        );
        assert_eq!(d.rotation, Rotation::Deg0, "{:?}", d.rotation);
        // Too large for all of them.
        let mut h = Harness::empty();
        add_box(&mut h, 400.0, 400.0);
        h.st.auto_hoop();
        assert!(h.st.message.contains("larger than all hoops"), "{}", h.st.message);
    }

    #[test]
    fn auto_hoop_leaves_out_multi_position_hoops() {
        let mut h = Harness::empty();
        add_box(&mut h, 100.0, 80.0);
        h.st.add_user_hoops(emb_model::HoopCatalog {
            hoops: vec![emb_cli::corpus::test_position_hoop()],
        });
        let mut s = h.st.settings.clone();
        s.my_hoops = vec![emb_cli::corpus::TEST_POSITION_HOOP.into(), "square_200x200".into()];
        h.st.set_settings(s);
        h.st.auto_hoop();
        assert_eq!(
            h.st.doc.design().hoop.selection,
            HoopSelection::Preset {
                id: "square_200x200".into()
            }
        );
    }

    #[test]
    fn the_back_mark_turns_with_the_hoop() {
        let mut h = Harness::empty();
        let c = h.st.doc.design().hoop.center;
        // Upright: the back of the machine is at the top of the view.
        let (back, dir) = h.st.hoop_back().unwrap();
        assert!(back.distance(c + Point::new(0.0, -130.0)) < 1e-9, "{back:?}");
        assert!(dir.distance(Point::new(0.0, -1.0)) < 1e-9);
        // The top of the design sews toward the back mark: the design point
        // in that direction is at the back of the hoop frame.
        h.st.set_hoop_angle(90.0, None);
        let (back, dir) = h.st.hoop_back().unwrap();
        assert!((back.distance(c) - 130.0).abs() < 1e-9);
        let q =
            h.st.doc
                .design()
                .output_transform()
                .apply(c + Point::new(dir.x, dir.y) * 50.0);
        assert!(q.distance(Point::new(0.0, -50.0)) < 1e-9, "{q:?}");
    }

    #[test]
    fn a_turned_hoop_checks_the_turned_field() {
        let mut h = Harness::empty();
        add_box(&mut h, 250.0, 20.0);
        let id = h.st.doc.design().objects()[0].id;
        h.st.auto_hoop();
        assert!(!h.st.outside_margin(id));
        // At 45° the long box does not fit the hoop.
        h.st.set_hoop_angle(45.0, None);
        assert!(h.st.outside_margin(id));
        // Zoom to hoop shows all of the turned hoop.
        let b = h.st.hoop_box().unwrap();
        assert!(b.width() > 300.0 && b.height() > 300.0, "{b:?}");
        h.st.turn_hoop(-45.0);
        assert_eq!(h.st.doc.design().hoop.angle_deg, 0.0);
        // The angle stays from −180 to 180.
        h.st.set_hoop_angle(270.0, None);
        assert_eq!(h.st.doc.design().hoop.angle_deg, -90.0);
    }

    #[test]
    fn the_stitches_on_the_canvas_follow_the_turned_hoop() {
        let mut h = Harness::empty();
        add_box(&mut h, 40.0, 20.0);
        h.st.auto_hoop();
        let before = h.st.plan_view().segments.clone();
        h.st.set_hoop_angle(30.0, None);
        let plan = h.st.plan_view().plan.clone();
        // In the machine file the stitches are turned back by 30°.
        let ext = plan.stitches.extents().unwrap();
        assert!(ext.width() > 40.0 && ext.height() > 20.0, "{ext:?}");
        // On the canvas they stay where the objects are.
        let after = h.st.plan_view().segments.clone();
        let bbox = |s: &[(emb_render::Segment, egui::Color32)]| {
            BBox::from_points(s.iter().flat_map(|(g, _)| [g.a, g.b])).unwrap()
        };
        let (a, b) = (bbox(&before), bbox(&after));
        assert!(a.center().distance(b.center()) < 1.0, "{a:?} {b:?}");
        assert!((a.width() - b.width()).abs() < 1.0);
    }

    #[test]
    fn user_hoops_join_the_list_and_my_hoops() {
        let mut h = Harness::empty();
        let mut hoop = blank_hoop();
        hoop.name = "Magnetic 180".into();
        hoop.width_mm = 180.0;
        hoop.height_mm = 130.0;
        h.st.save_user_hoop(hoop, None).unwrap();
        let id = "my_magnetic_180";
        assert!(h.st.hoops.get(id).is_some());
        assert!(h.st.settings.my_hoops.contains(&id.to_string()));
        assert!(h.st.hoop_fits_machine(id));
        // A second hoop with the same name gets its own id.
        let mut again = blank_hoop();
        again.name = "Magnetic 180".into();
        h.st.save_user_hoop(again, None).unwrap();
        assert!(h.st.hoops.get("my_magnetic_180_2").is_some());
        // A bad size is refused.
        let mut bad = blank_hoop();
        bad.width_mm = 0.0;
        assert!(h.st.save_user_hoop(bad, None).is_err());
        // A multi-position hoop: positions inside the field are taken.
        let mut multi = blank_hoop();
        multi.name = "Two positions".into();
        multi.width_mm = 300.0;
        multi.positions = vec![
            emb_model::HoopPosition {
                x_mm: -75.0,
                y_mm: 0.0,
                width_mm: 150.0,
                height_mm: 150.0,
            },
            emb_model::HoopPosition {
                x_mm: 75.0,
                y_mm: 0.0,
                width_mm: 150.0,
                height_mm: 150.0,
            },
        ];
        h.st.save_user_hoop(multi.clone(), None).unwrap();
        assert_eq!(h.st.hoops.get("my_two_positions").unwrap().positions.len(), 2);
        multi.positions[1].x_mm = 100.0;
        assert!(h.st.save_user_hoop(multi, None).is_err());
        h.st.delete_user_hoop(id);
        assert!(h.st.hoops.get(id).is_none());
        assert!(!h.st.settings.my_hoops.contains(&id.to_string()));
    }

    #[test]
    fn the_template_has_the_hoop_at_full_size() {
        let mut h = Harness::empty();
        add_box(&mut h, 60.0, 40.0);
        h.st.auto_hoop();
        let pages = h.st.hoop_template_pages().unwrap();
        assert_eq!(pages.len(), 1);
        let has_stitches = pages[0]
            .ops
            .iter()
            .any(|o| matches!(o, emb_render::page::Op::Stitches { .. }));
        assert!(has_stitches);
    }

    #[test]
    fn the_windows_draw_and_do_not_change_the_design() {
        let mut h = Harness::new();
        h.st.doc.edit("Start", |d| {
            d.start_end.start = Some(Anchor::Point {
                point: Point::new(5.0, 5.0),
            });
            d.start_end.end = Some(Anchor::DESIGN_CENTER);
            d.hoop.angle_deg = 15.0;
        });
        h.st.hoop_windows.options = true;
        h.st.hoop_windows.my_hoops = true;
        h.st.hoop_windows.form = Some((blank_hoop(), None));
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| windows(ui.ctx(), &mut h.st));
            out.textures_delta.clear();
            h.frame(vec![]);
        }
        assert_eq!(h.st.doc.state_id(), state);
        assert!(h.st.hoop_windows.form.is_some());
    }
}
