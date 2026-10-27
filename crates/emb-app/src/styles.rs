//! Fabrics, current properties, styles and templates. See
//! `docs/design/p3-plan.md` (Milestone 6).
//!
//! - The Fabric window sets the fabric of the design. It changes the
//!   current properties and, if the user wants, the satin and fill objects,
//!   as one undo step.
//! - The Styles panel shows the current properties and the styles of the
//!   design: make a style from the selection, apply it, update it, delete it.
//! - Templates are files in the `templates` folder next to the settings
//!   file. "New from Template" starts a design from one; "Save as
//!   template" makes one from the design.

use std::path::{Path, PathBuf};

use eframe::egui;
use emb_model::{Document, Fabric, ObjectKind, Style, template};

use crate::a11y::Named;
use crate::app::AppState;
use crate::file::Pending;
use crate::i18n::tr;
use crate::words;

/// The Fabric window.
#[derive(Debug, Clone, PartialEq)]
pub struct FabricDialog {
    /// The fabric id; `None` is no fabric (the base settings).
    pub choice: Option<String>,
    /// Change the satin and fill objects too, not only new objects.
    pub objects: bool,
}

/// The template windows.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateDialog {
    /// Pick a template for a new design.
    New { list: Vec<(String, PathBuf)> },
    /// Save the design as a template.
    Save { name: String, objects: bool },
}

/// A new design from a template with parameters: the values that the
/// user types for them (ADR 0078).
#[derive(Debug, Clone)]
pub struct TemplateAsk {
    pub design: emb_model::Design,
    pub name: String,
    /// Each parameter and the text of its value.
    pub rows: Vec<(emb_model::template::TemplateParameter, String)>,
    pub error: Option<String>,
}

/// The folder of the user's templates.
pub fn templates_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("templates"))
}

/// The templates in `dir` by name, sorted.
pub fn list_templates(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(template::FILE_EXTENSION))
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().into_owned(), p)))
        .collect();
    v.sort_by_key(|(n, _)| n.to_lowercase());
    v
}

/// A template name that is safe as a file name.
pub(crate) fn valid_name(name: &str) -> Option<&str> {
    let n = name.trim();
    let bad = n.is_empty() || n.starts_with('.') || n.chars().any(|c| "/\\:*?\"<>|".contains(c) || c.is_control());
    (!bad).then_some(n)
}

/// The change of fabric as text, for example "10 % denser, pull
/// compensation +0.20 mm, heavy underlay".
pub fn change_text(from: &Fabric, to: &Fabric) -> String {
    let mut v = Vec::new();
    let s = to.spacing_factor / from.spacing_factor;
    if (s - 1.0).abs() > 1e-6 {
        let pct = ((1.0 - s).abs() * 100.0).round();
        v.push(if s < 1.0 {
            format!("{pct} % denser")
        } else {
            format!("{pct} % lighter")
        });
    }
    let l = to.stitch_length_factor / from.stitch_length_factor;
    if (l - 1.0).abs() > 1e-6 {
        let pct = ((l - 1.0).abs() * 100.0).round();
        v.push(if l > 1.0 {
            format!("fill stitches {pct} % longer")
        } else {
            format!("fill stitches {pct} % shorter")
        });
    }
    let p = to.pull_mm - from.pull_mm;
    if p.abs() > 1e-6 {
        v.push(format!("pull compensation {p:+.2} mm"));
    }
    if to.underlay != from.underlay {
        v.push(format!(
            "{} underlay",
            words::underlay_weight(to.underlay).to_lowercase()
        ));
    }
    if v.is_empty() {
        "no change to the stitch settings".into()
    } else {
        v.join(", ")
    }
}

impl AppState {
    pub fn open_fabric_dialog(&mut self) {
        self.fabric_dialog = Some(FabricDialog {
            choice: self.doc.design().fabric.clone(),
            objects: true,
        });
    }

    /// The name of the fabric of the design, or "Not set".
    pub fn fabric_name(&self) -> String {
        self.fabrics.of(self.doc.design()).name
    }

    /// Sets the fabric as one undo step. With `objects`, the satin and fill
    /// objects change too.
    pub fn apply_fabric(&mut self, id: Option<&str>, objects: bool) {
        let fabrics = self.fabrics.clone();
        let name = id
            .and_then(|i| fabrics.get(i))
            .map_or("no fabric".to_string(), |f| f.name.clone());
        let changed = self
            .doc
            .edit(format!("Fabric: {name}"), |d| d.set_fabric(&fabrics, id, objects));
        self.message = match changed.len() {
            0 => format!("Fabric: {name}. New objects get the fabric settings."),
            n => format!("Fabric: {name}. {n} objects changed."),
        };
    }

    /// Copies the settings of the selected objects to the current properties.
    pub fn make_properties_current(&mut self) {
        let ids = self.selection.in_sew_order(self.doc.design());
        let mut test = self.doc.design().clone();
        if test.make_current(&ids) == 0 {
            self.message = tr("The current properties are the same already").into();
            return;
        }
        self.doc.edit("Make properties current", |d| d.make_current(&ids));
        self.message = tr("New objects get the settings of the selection").into();
    }

    /// Gives the selected objects the current properties.
    pub fn apply_current_properties(&mut self) {
        let ids = self.selection.in_sew_order(self.doc.design());
        let mut test = self.doc.design().clone();
        if test.apply_current(&ids).is_empty() {
            self.message = tr("The selection has the current properties already").into();
            return;
        }
        let n = self
            .doc
            .edit("Apply current properties", |d| d.apply_current(&ids).len());
        self.message = crate::i18n::fill("Current properties applied to {} objects", &[&n]);
    }

    /// Makes a style from the selected objects. A style with the same name is replaced.
    pub fn new_style(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.message = tr("Type a name for the style").into();
            return;
        }
        let d = self.doc.design();
        let ids = self.selection.in_sew_order(d);
        let Some(style) = Style::from_objects(name, ids.iter().filter_map(|i| d.object(*i))) else {
            self.message = tr("Select a running, satin or fill object to make a style").into();
            return;
        };
        self.message = crate::i18n::fill("Style \"{}\" for {}", &[&name, &(style.types())]);
        self.doc
            .edit(format!("New style {name}"), |d| match d.style_index(&style.name) {
                Some(i) => d.styles[i] = style,
                None => d.styles.push(style),
            });
    }

    /// Gives the selected objects the settings of style `i`.
    pub fn apply_style(&mut self, i: usize) {
        let ids = self.selection.in_sew_order(self.doc.design());
        let Some(name) = self.doc.design().styles.get(i).map(|s| s.name.clone()) else {
            return;
        };
        let mut test = self.doc.design().clone();
        if test.apply_style(i, &ids).is_empty() {
            self.message = crate::i18n::fill(
                "The selection has the settings of \"{}\" already, or no object of its types",
                &[&name],
            );
            return;
        }
        let n = self
            .doc
            .edit(format!("Apply style {name}"), |d| d.apply_style(i, &ids).len());
        self.message = crate::i18n::fill("Style \"{}\" applied to {} objects", &[&name, &n]);
    }

    /// Takes the settings of the selected objects into style `i`.
    pub fn update_style(&mut self, i: usize) {
        let d = self.doc.design();
        let ids = self.selection.in_sew_order(d);
        let Some(mut style) = d.styles.get(i).cloned() else {
            return;
        };
        style.update_from(ids.iter().filter_map(|id| d.object(*id)));
        if d.styles[i] == style {
            self.message = tr("The style has the settings of the selection already").into();
            return;
        }
        let name = style.name.clone();
        self.doc.edit(format!("Update style {name}"), |d| d.styles[i] = style);
        self.message = crate::i18n::fill(
            "Style \"{}\" updated. Objects keep their settings until you apply it.",
            &[&name],
        );
    }

    pub fn delete_style(&mut self, i: usize) {
        let Some(name) = self.doc.design().styles.get(i).map(|s| s.name.clone()) else {
            return;
        };
        self.doc.edit(format!("Delete style {name}"), |d| d.styles.remove(i));
    }

    pub fn rename_style(&mut self, i: usize, name: &str) {
        let name = name.trim().to_string();
        let d = self.doc.design();
        if name.is_empty() || d.styles.get(i).is_none_or(|s| s.name == name) {
            return;
        }
        if d.style_index(&name).is_some() {
            self.message = crate::i18n::fill("A style \"{}\" exists already", &[&name]);
            return;
        }
        self.doc.edit("Rename style", |d| d.styles[i].name = name);
    }

    /// Writes the design as a template `name` in `dir`.
    pub fn save_template(&mut self, dir: &Path, name: &str, objects: bool) -> Result<PathBuf, String> {
        let name = valid_name(name).ok_or("Type a name without / \\ : * ? \" < > |")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(format!("{name}.{}", template::FILE_EXTENSION));
        let t = template::from_design(self.doc.design(), name, objects);
        template::save(&t, &path).map_err(|e| e.to_string())?;
        self.message = crate::i18n::fill("Template \"{}\" saved", &[&name]);
        Ok(path)
    }

    /// Starts a new design from a template file, or from the built-in
    /// template (`None`).
    pub fn new_from_template(&mut self, path: Option<&Path>) {
        let Some(path) = path else {
            self.new_builtin_design();
            return;
        };
        match template::load(path) {
            Ok(t) => {
                let asks = template::parameters(&t);
                if asks.is_empty() {
                    self.start_design(template::new_design(&t));
                    self.message = crate::i18n::fill("New design from template \"{}\"", &[&t.name]);
                } else {
                    // The template asks for the values of its parameters.
                    self.template_ask = Some(TemplateAsk {
                        design: template::new_design(&t),
                        name: t.name.clone(),
                        rows: asks
                            .into_iter()
                            .map(|p| {
                                let text = p.expr.clone();
                                (p, text)
                            })
                            .collect(),
                        error: None,
                    });
                }
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Template not opened".into(),
                    lines: vec![format!("{}: {e}", path.display())],
                });
            }
        }
    }

    /// Shows a new, unsaved design: in the active tab if it is blank, else
    /// in a new tab.
    pub fn start_design(&mut self, mut d: emb_model::Design) {
        crate::backdrop::upright(&mut d);
        self.open_tab();
        self.doc = Document::new(d);
        self.file_path = None;
        self.selection.clear();
        // The objects that were hidden and the wizard are of the old design.
        self.view_filter = Default::default();
        self.wizard = None;
        self.hoopings_ui = Default::default();
        self.current_thread = None;
        self.fit_requested = true;
    }

    /// Makes the new design of the template that asks for parameters, with
    /// the values that the user typed. Returns false (with the error in
    /// the window) if a value is refused.
    pub fn make_from_template_ask(&mut self) -> bool {
        let Some(ask) = self.template_ask.as_mut() else {
            return false;
        };
        let mut d = ask.design.clone();
        for (p, text) in &ask.rows {
            if text.trim() == p.expr {
                continue;
            }
            if let Err(e) = d.set_sketch_parameter(p.sketch, &p.name, text) {
                ask.error = Some(format!("{}: {}", p.name, crate::sketch::expr_message(&e)));
                return false;
            }
        }
        let name = ask.name.clone();
        self.template_ask = None;
        self.start_design(d);
        self.message = crate::i18n::fill("New design from template \"{}\"", &[&name]);
        true
    }

    pub fn open_template_dialog(&mut self, save: bool) {
        self.template_dialog = Some(if save {
            TemplateDialog::Save {
                name: self.doc.design().name.clone(),
                objects: false,
            }
        } else {
            TemplateDialog::New {
                list: templates_dir().map(|d| list_templates(&d)).unwrap_or_default(),
            }
        });
    }
}

/// One line of the current properties.
fn current_line(ui: &mut egui::Ui, kind: &str, text: String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(kind).strong());
        ui.label(text);
    });
}

/// The Styles panel (a dock tab).
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    let has_sel = !st.selection.is_empty();
    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Fabric:"));
            ui.strong(st.fabric_name());
            if ui.button(tr("Change…")).clicked() {
                st.open_fabric_dialog();
            }
        });
        if let Some(f) = st.doc.design().fabric.as_deref().and_then(|id| st.fabrics.get(id)) {
            ui.weak(crate::i18n::fill("Backing: {}", &[&f.backing]));
            if let Some(t) = &f.topping {
                ui.weak(crate::i18n::fill("Topping: {}", &[&t]));
            }
        }
        ui.separator();

        ui.strong(tr("Current properties")).on_hover_text(tr(
            "The stitch settings that new objects get. The property bar changes them while a drawing tool is active.",
        ));
        let c = st.doc.design().current.clone();
        let u = st.settings.units;
        current_line(
            ui,
            "Running",
            format!(
                "stitch length {} ({})",
                u.fmt(c.running.stitch_length_mm),
                words::length(c.running.stitch_length_mm)
            ),
        );
        current_line(
            ui,
            "Satin",
            format!(
                "density {} ({}), underlay {}",
                u.fmt(c.satin.spacing_mm),
                words::density(c.satin.spacing_mm),
                words::underlay(&c.satin.underlay).to_lowercase()
            ),
        );
        current_line(
            ui,
            "Fill",
            format!(
                "density {} ({}), stitch length {}, underlay {}",
                u.fmt(c.fill.spacing_mm),
                words::density(c.fill.spacing_mm),
                u.fmt(c.fill.stitch_length_mm),
                words::underlay(&c.fill.underlay).to_lowercase()
            ),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(has_sel, egui::Button::new(tr("Make Current")))
                .on_hover_text(tr("New objects get the settings of the selected objects."))
                .clicked()
            {
                st.make_properties_current();
            }
            if ui
                .add_enabled(has_sel, egui::Button::new(tr("Apply Current")))
                .on_hover_text(tr("The selected objects get the current properties."))
                .clicked()
            {
                st.apply_current_properties();
            }
        });
        ui.separator();

        ui.strong(tr("Styles")).on_hover_text(tr(
            "Named sets of stitch settings. Apply a style to give objects the same settings.",
        ));
        let styles = st.doc.design().styles.clone();
        if styles.is_empty() {
            ui.weak(tr("No styles. Select an object, type a name and click New style."));
        }
        let mut action: Option<(usize, u8)> = None;
        for (i, s) in styles.iter().enumerate() {
            ui.horizontal(|ui| {
                let fav = st.favorite_index(&s.name).is_some();
                let star = if fav {
                    egui::RichText::new(egui_phosphor::regular::STAR)
                        .color(egui::Color32::from_rgb(240, 180, 40))
                        .strong()
                } else {
                    egui::RichText::new(egui_phosphor::regular::STAR).weak()
                };
                if ui
                    .add(egui::Button::new(star).frame(false))
                    .on_hover_text(if fav {
                        tr("A favorite style (Design > Favorites, in all designs). Click: not a favorite.")
                    } else {
                        tr("Make a favorite style: Design > Favorites applies it in all designs.")
                    })
                    .named(if fav {
                        tr("Favorite style")
                    } else {
                        tr("Make a favorite style")
                    })
                    .clicked()
                {
                    action = Some((i, 3));
                }
                let id = ui.id().with(("style_name", i));
                let mut name = ui
                    .data_mut(|d| d.get_temp::<String>(id))
                    .unwrap_or_else(|| s.name.clone());
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(110.0));
                if r.changed() {
                    ui.data_mut(|d| d.insert_temp(id, name.clone()));
                }
                if r.lost_focus() {
                    ui.data_mut(|d| d.remove::<String>(id));
                    st.rename_style(i, &name);
                }
                ui.weak(s.types()).on_hover_text(style_text(s, st.settings.units));
                if ui
                    .add_enabled(has_sel, egui::Button::new(tr("Apply")).small())
                    .on_hover_text(tr("Give the selected objects the settings of this style."))
                    .clicked()
                {
                    action = Some((i, 0));
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new(tr("Update")).small())
                    .on_hover_text(tr("Take the settings of the selected objects into this style."))
                    .clicked()
                {
                    action = Some((i, 1));
                }
                if ui.small_button(tr("Delete")).clicked() {
                    action = Some((i, 2));
                }
            });
        }
        match action {
            Some((i, 0)) => st.apply_style(i),
            Some((i, 1)) => st.update_style(i),
            Some((i, 3)) => st.toggle_favorite(&styles[i]),
            Some((i, _)) => st.delete_style(i),
            None => {}
        }
        ui.horizontal(|ui| {
            let id = ui.id().with("new_style_name");
            let mut name = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_default();
            ui.add(
                egui::TextEdit::singleline(&mut name)
                    .hint_text(tr("Style name"))
                    .desired_width(110.0),
            );
            ui.data_mut(|d| d.insert_temp(id, name.clone()));
            if ui
                .add_enabled(has_sel, egui::Button::new(tr("New style")))
                .on_hover_text(tr("Make a style from the settings of the selected objects."))
                .clicked()
            {
                st.new_style(&name);
                if st.doc.design().style_index(name.trim()).is_some() {
                    ui.data_mut(|d| d.remove::<String>(id));
                }
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("New from Template…")).clicked() {
                st.open_template_dialog(false);
            }
            if ui.button(tr("Save as Template…")).clicked() {
                st.open_template_dialog(true);
            }
        });
    });
}

/// The settings of a style, for a tooltip.
fn style_text(s: &Style, u: crate::workspace::Units) -> String {
    let mut v = Vec::new();
    if let Some(r) = &s.running {
        v.push(format!("Running: stitch length {}", u.fmt(r.stitch_length_mm)));
    }
    if let Some(p) = &s.satin {
        v.push(format!(
            "Satin: density {} ({}), pull compensation {}, underlay {}",
            u.fmt(p.spacing_mm),
            words::density(p.spacing_mm),
            u.fmt(p.pull_compensation_mm),
            words::underlay(&p.underlay).to_lowercase()
        ));
    }
    if let Some(p) = &s.fill {
        v.push(format!(
            "Fill: density {} ({}), stitch length {}, angle {:.0}°, underlay {}",
            u.fmt(p.spacing_mm),
            words::density(p.spacing_mm),
            u.fmt(p.stitch_length_mm),
            p.angle_deg,
            words::underlay(&p.underlay).to_lowercase()
        ));
    }
    v.join("\n")
}

/// The Fabric and template windows.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    fabric_window(ctx, st);
    template_window(ctx, st);
    template_ask_window(ctx, st);
}

/// The values for the parameters of a template, before the new design.
fn template_ask_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(ask) = st.template_ask.as_mut() else {
        return;
    };
    let mut open = true;
    let mut make = false;
    let id = egui::Id::new("Template Parameters");
    let several = ask
        .rows
        .iter()
        .any(|(p, _)| ask.rows.first().is_some_and(|(f, _)| f.sketch != p.sketch));
    crate::dock::docked(
        egui::Window::new(tr("Template Parameters")).id(id),
        ctx,
        id,
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        ui.label(crate::i18n::fill(
            "The template \"{}\" has parameters. Type the values for the new design:",
            &[&ask.name],
        ));
        egui::Grid::new("template_ask").num_columns(2).show(ui, |ui| {
            for (p, text) in &mut ask.rows {
                let label = match (&p.sketch_name, several) {
                    (Some(s), true) => format!("{s}: {}", p.name),
                    _ => p.name.clone(),
                };
                ui.label(label);
                ui.add(egui::TextEdit::singleline(text).desired_width(160.0));
                ui.end_row();
            }
        });
        ui.weak(tr(
            "You can type a unit (80 mm, 3 in) or an expression with the other names.",
        ));
        if let Some(e) = &ask.error {
            ui.colored_label(ui.visuals().warn_fg_color, e);
        }
        make = ui.button(tr("Make Design")).clicked();
    });
    if make {
        st.make_from_template_ask();
    } else if !open {
        st.template_ask = None;
    }
}

fn fabric_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.fabric_dialog.clone() else {
        return;
    };
    let mut open = true;
    let mut apply = false;
    let from = st.fabrics.of(st.doc.design());
    let n_objects = st
        .doc
        .design()
        .objects()
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Satin { .. } | ObjectKind::Fill { .. }))
        .count();
    crate::dock::docked(
        egui::Window::new(tr("Fabric")).id(egui::Id::new("Fabric")),
        ctx,
        egui::Id::new("Fabric"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(420.0)
    .show(ctx, |ui| {
        ui.label(tr(
            "The fabric sets the density, pull compensation and underlay of satin and fill stitches.",
        ));
        ui.add_space(4.0);
        let to = dlg
            .choice
            .as_deref()
            .and_then(|id| st.fabrics.get(id))
            .cloned()
            .unwrap_or_else(Fabric::base);
        egui::ComboBox::from_label(tr("Fabric"))
            .width(260.0)
            .selected_text(&to.name)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut dlg.choice, None, tr("Not set (base settings)"));
                for f in &st.fabrics.fabrics {
                    ui.selectable_value(&mut dlg.choice, Some(f.id.clone()), &f.name)
                        .on_hover_text(&f.description);
                }
            });
        if !to.description.is_empty() {
            ui.label(&to.description);
        }
        egui::Grid::new("fabric_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.weak(tr("Change"));
                ui.label(change_text(&from, &to));
                ui.end_row();
                if !to.backing.is_empty() {
                    ui.weak(tr("Backing"));
                    ui.label(&to.backing);
                    ui.end_row();
                }
                if let Some(t) = &to.topping {
                    ui.weak(tr("Topping"));
                    ui.label(t);
                    ui.end_row();
                }
            });
        ui.add_space(4.0);
        ui.checkbox(
            &mut dlg.objects,
            format!("Change the satin and fill objects of the design ({n_objects})"),
        )
        .on_hover_text(tr(
            "Each object keeps its own settings, changed by the difference between the fabrics. \
                 Objects with no underlay or custom underlay keep their underlay.",
        ));
        ui.weak(tr(
            "New objects always get the fabric settings. The worksheet shows the fabric, backing and topping.",
        ));
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr("Apply")).clicked() {
                apply = true;
            }
            if ui.button(tr("Cancel")).clicked() {
                st.fabric_dialog = None;
            }
        });
    });
    if apply {
        st.apply_fabric(dlg.choice.as_deref(), dlg.objects);
        st.fabric_dialog = None;
    } else if !open {
        st.fabric_dialog = None;
    } else if st.fabric_dialog.is_some() {
        st.fabric_dialog = Some(dlg);
    }
}

fn template_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.template_dialog.clone() else {
        return;
    };
    let mut open = true;
    let mut close = false;
    let dir = templates_dir();
    let title = match dlg {
        TemplateDialog::New { .. } => tr("New from Template"),
        TemplateDialog::Save { .. } => tr("Save as Template"),
    };
    crate::dock::docked(
        egui::Window::new(title).id(egui::Id::new("Template")),
        ctx,
        egui::Id::new("Template"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(380.0)
    .show(ctx, |ui| match &mut dlg {
        TemplateDialog::New { list } => {
            ui.label(tr(
                "A template gives a new design its machine, hoop, fabric, current properties, styles and threads.",
            ));
            let mut default = st.settings.template.clone();
            let mut pick: Option<Option<PathBuf>> = None;
            egui::Grid::new("template_list")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    if ui
                        .button(tr("Normal"))
                        .on_hover_text(tr("The built-in settings"))
                        .clicked()
                    {
                        pick = Some(None);
                    }
                    ui.radio_value(&mut default, None, tr("Use for New"));
                    ui.end_row();
                    for (name, path) in list.iter() {
                        if ui.button(name.as_str()).clicked() {
                            pick = Some(Some(path.clone()));
                        }
                        ui.radio_value(&mut default, Some(name.clone()), tr("Use for New"));
                        ui.end_row();
                    }
                });
            if list.is_empty() {
                ui.weak(tr("No templates of your own yet. Use Save as template to make one."));
            }
            ui.separator();
            ui.label(tr("Free-standing lace, on water-soluble stabilizer:"));
            ui.horizontal(|ui| {
                for (i, (name, _)) in emb_model::lace::TEMPLATES.iter().enumerate() {
                    if ui.button(tr(name)).clicked() {
                        st.request(Pending::NewLace(i));
                        close = true;
                    }
                }
            });
            if default != st.settings.template {
                let mut s = st.settings.clone();
                s.template = default;
                st.set_settings(s);
            }
            if let Some(d) = &dir {
                ui.weak(crate::i18n::fill("Folder: {}", &[&(d.display())]));
            }
            if let Some(p) = pick {
                st.request(Pending::NewFrom(p));
                close = true;
            }
        }
        TemplateDialog::Save { name, objects } => {
            ui.label(tr(
                "Saves the machine, hoop, fabric, current properties, styles, threads and notes.",
            ));
            ui.horizontal(|ui| {
                ui.label(tr("Name"));
                ui.text_edit_singleline(name);
            });
            ui.checkbox(objects, tr("Keep the objects too")).on_hover_text(tr(
                "For example a frame or a name badge that each new design starts with.",
            ));
            let exists = dir
                .as_ref()
                .zip(valid_name(name))
                .is_some_and(|(d, n)| d.join(format!("{n}.{}", template::FILE_EXTENSION)).is_file());
            if exists {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "A template with this name exists. Save replaces it.",
                );
            }
            ui.horizontal(|ui| {
                let ok = valid_name(name).is_some() && dir.is_some();
                if ui.add_enabled(ok, egui::Button::new(tr("Save"))).clicked()
                    && let Some(d) = &dir
                {
                    match st.save_template(d, name, *objects) {
                        Ok(_) => close = true,
                        Err(e) => st.message = e,
                    }
                }
                if ui.button(tr("Cancel")).clicked() {
                    close = true;
                }
            });
        }
    });
    if close || !open {
        st.template_dialog = None;
    } else {
        st.template_dialog = Some(dlg);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{FillParams, SatinGeometry, SatinParams, Underlay};

    use super::*;
    use crate::test_util::Harness;

    /// Three satin columns and a fill, all selected.
    fn harness() -> Harness {
        let mut h = Harness::empty();
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("red", emb_model::Rgb(200, 0, 0));
            for i in 0..3 {
                let y = f64::from(i) * 10.0;
                let line = Path::new().move_to(Point::new(0.0, y)).line_to(Point::new(30.0, y));
                d.add_object(
                    t,
                    ObjectKind::Satin {
                        geometry: SatinGeometry::from_centerline(&line, 3.0),
                        params: SatinParams::default(),
                    },
                );
            }
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::new(0.0, 40.0), Point::new(20.0, 60.0))),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            );
        });
        h.st.select_all();
        h
    }

    fn satins(st: &AppState) -> Vec<SatinParams> {
        st.doc
            .design()
            .objects()
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Satin { params, .. } => Some(params.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn cotton_to_towel_in_one_step() {
        let mut h = harness();
        h.st.apply_fabric(Some("cotton"), true);
        let steps = h.st.doc.undo_len();
        let before = satins(&h.st);
        h.st.apply_fabric(Some("towel"), true);
        assert_eq!(h.st.doc.undo_len(), steps + 1, "one undo step");
        assert!(h.st.message.contains("4 objects"), "{}", h.st.message);
        let after = satins(&h.st);
        assert!(
            after
                .iter()
                .all(|p| p.underlay == Underlay::Heavy && p.spacing_mm < before[0].spacing_mm)
        );
        assert_eq!(h.st.fabric_name(), "Towel (terry)");
        let data = h.st.current_sheet_data();
        assert_eq!(data.fabric.as_deref(), Some("Towel (terry)"));
        assert!(data.topping.is_some());
        h.st.undo();
        assert_eq!(satins(&h.st), before);
        assert_eq!(h.st.doc.design().fabric.as_deref(), Some("cotton"));
    }

    #[test]
    fn style_to_three_objects() {
        let mut h = harness();
        let ids = h.st.selection.in_sew_order(h.st.doc.design());
        // A style from the first satin, with new settings.
        h.st.doc.edit("Tune", |d| {
            if let Some(ObjectKind::Satin { params, .. }) = d.object_mut(ids[0]).map(|o| &mut o.kind) {
                params.spacing_mm = 0.3;
                params.underlay = Underlay::None;
            }
        });
        h.st.selection.set(ids[0]);
        h.st.new_style("Lettering");
        assert_eq!(h.st.doc.design().styles.len(), 1);
        assert!(h.st.message.contains("satin"), "{}", h.st.message);
        h.st.select_all();
        let steps = h.st.doc.undo_len();
        h.st.apply_style(0);
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert!(
            h.st.message.contains("2 objects"),
            "the first has the settings already: {}",
            h.st.message
        );
        assert!(
            satins(&h.st)
                .iter()
                .all(|p| p.spacing_mm == 0.3 && p.underlay == Underlay::None)
        );
        h.st.rename_style(0, "Small text");
        assert_eq!(h.st.doc.design().styles[0].name, "Small text");
        h.st.delete_style(0);
        assert!(h.st.doc.design().styles.is_empty());
    }

    #[test]
    fn current_properties_round_trip() {
        let mut h = harness();
        let ids = h.st.selection.in_sew_order(h.st.doc.design());
        h.st.doc.edit("Tune", |d| {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(ids[3]).map(|o| &mut o.kind) {
                params.angle_deg = 10.0;
            }
        });
        h.st.selection.set(ids[3]);
        h.st.make_properties_current();
        assert_eq!(h.st.doc.design().current.fill.angle_deg, 10.0);
        // A new fill gets the current properties.
        let kind = crate::draw::build_box_with(
            crate::draw::Tool::Rectangle,
            emb_geom::BBox::new(Point::ZERO, Point::new(10.0, 10.0)),
            &h.st.doc.design().current,
            &h.st.settings.defaults,
        )
        .unwrap();
        assert!(matches!(kind, ObjectKind::Fill { params, .. } if params.angle_deg == 10.0));
        h.st.doc.edit("Current", |d| d.current.satin.spacing_mm = 0.5);
        h.st.select_all();
        h.st.apply_current_properties();
        assert!(satins(&h.st).iter().all(|p| p.spacing_mm == 0.5));
        let steps = h.st.doc.undo_len();
        h.st.apply_current_properties();
        assert_eq!(h.st.doc.undo_len(), steps, "nothing to change: no undo step");
    }

    #[test]
    fn templates_save_list_and_start_a_design() {
        let mut h = harness();
        h.st.apply_fabric(Some("fleece"), true);
        h.st.new_style("Outline");
        let dir = std::env::temp_dir().join(format!("emb-app-templates-{}", std::process::id()));
        assert!(h.st.save_template(&dir, "bad/name", false).is_err());
        let path = h.st.save_template(&dir, "Fleece jackets", false).unwrap();
        let list = list_templates(&dir);
        assert_eq!(list, vec![("Fleece jackets".to_string(), path.clone())]);
        h.st.doc.mark_saved();
        h.st.request(Pending::NewFrom(Some(path)));
        std::fs::remove_dir_all(&dir).ok();
        let d = h.st.doc.design();
        assert!(d.objects().is_empty());
        assert_eq!(d.name, "Untitled");
        assert_eq!(d.fabric.as_deref(), Some("fleece"));
        assert_eq!(d.styles.len(), 1);
        assert!(h.st.file_path.is_none());
        h.st.request(Pending::NewFrom(None));
        assert!(h.st.doc.design().fabric.is_none());
    }

    #[test]
    fn change_texts() {
        let c = emb_model::FabricCatalog::builtin();
        let base = Fabric::base();
        assert_eq!(
            change_text(&base, c.get("cotton").unwrap()),
            "no change to the stitch settings"
        );
        assert_eq!(
            change_text(c.get("cotton").unwrap(), c.get("towel").unwrap()),
            "10 % denser, pull compensation +0.20 mm, heavy underlay"
        );
        assert!(change_text(&base, c.get("leather").unwrap()).contains("fill stitches 15 % longer"));
    }

    /// One frame with the Styles panel and the windows.
    fn draw(h: &mut Harness) {
        let st = &mut h.st;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let mut out = h.ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            egui::CentralPanel::default().show(ui, |ui| panel(ui, st));
            windows(&ctx, st);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn panel_and_windows_draw() {
        let mut h = harness();
        h.st.new_style("Outline");
        h.st.apply_fabric(Some("towel"), false);
        h.st.open_template_dialog(true);
        h.st.open_fabric_dialog();
        draw(&mut h);
        draw(&mut h);
        assert!(h.st.template_dialog.is_some() && h.st.fabric_dialog.is_some());
        h.st.open_template_dialog(false);
        draw(&mut h);
        assert!(matches!(h.st.template_dialog, Some(TemplateDialog::New { .. })));
    }
}
