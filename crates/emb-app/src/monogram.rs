//! The Monogram window (P4 Milestone 5, ADR 0019): initials, a template,
//! the style, the font and size, the border, the ornaments and the colors,
//! with a preview. "Add" puts the monogram in the middle of the hoop as one
//! group; with a monogram selected, the window changes it.
//!
//! Templates: the built-in ones, and the user's in the `monograms` folder
//! next to the settings file (`.monogram`, JSON).

use std::path::PathBuf;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use emb_geom::{BBox, Point};
use emb_model::monogram::TEMPLATE_EXTENSION;
use emb_model::{
    BorderShape, BorderStitch, FontRef, MonogramSpec, MonogramStyle, MonogramTemplate, ObjectKind, OrnamentPlace,
    OrnamentShape, Rgb,
};

use crate::app::AppState;
use crate::i18n::tr;

/// The state of the Monogram window.
#[derive(Debug, Clone)]
pub struct MonogramWindow {
    pub open: bool,
    pub initials: String,
    pub spec: MonogramSpec,
    /// The group of the monogram that the window changes.
    pub replace: Option<u32>,
    /// The name for Save as template.
    pub template_name: String,
}

impl Default for MonogramWindow {
    fn default() -> Self {
        MonogramWindow {
            open: false,
            initials: "ABC".into(),
            spec: MonogramSpec::new(default_font()),
            replace: None,
            template_name: String::new(),
        }
    }
}

fn default_font() -> FontRef {
    FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, "")
}

/// The folder of the user's monogram templates.
pub fn templates_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("monograms"))
}

/// The built-in templates and the user's.
pub fn templates() -> Vec<MonogramTemplate> {
    let mut all = MonogramTemplate::builtin(&default_font());
    if let Some(Ok(rd)) = templates_dir().map(std::fs::read_dir) {
        let mut user: Vec<MonogramTemplate> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(TEMPLATE_EXTENSION))
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|s| MonogramTemplate::from_json(&s).ok())
            .collect();
        user.sort_by_key(|a| a.name.to_lowercase());
        all.extend(user);
    }
    all
}

impl AppState {
    /// Opens the Monogram window: for the selected monogram, or for a new one.
    pub fn open_monogram(&mut self) {
        self.fonts.start_scan();
        let d = self.doc.design();
        let selected = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id)?.group)
            .find_map(|g| d.monograms.iter().find(|r| r.group == g));
        let w = &mut self.monogram_window;
        match selected {
            Some(r) => {
                w.initials = r.initials.clone();
                w.spec = r.spec.clone();
                w.replace = Some(r.group);
            }
            None => w.replace = None,
        }
        w.open = true;
    }

    /// The objects of the monogram of the window, at `center` (`None` if the
    /// font is missing or has none of the initials).
    fn monogram_objects(&mut self, center: Point) -> Option<emb_letter::monogram::MonogramObjects> {
        let w = self.monogram_window.clone();
        let font = self.fonts.get(&w.spec.font)?;
        emb_letter::monogram::build(&w.initials, &w.spec, font.source(), center)
    }

    /// Adds the monogram of the window (or changes the selected one), as one
    /// undo step, and selects it.
    pub fn place_monogram(&mut self) {
        let w = self.monogram_window.clone();
        let replace = w
            .replace
            .and_then(|g| self.doc.design().monograms.iter().find(|r| r.group == g).cloned());
        let center = replace.as_ref().map_or(self.doc.design().hoop.center, |r| r.center);
        let Some(m) = self.monogram_objects(center) else {
            self.message = crate::i18n::fill(
                "{} is not on this computer or has none of these initials.",
                &[&(w.spec.font.label())],
            );
            return;
        };
        let label = if replace.is_some() {
            "Change monogram"
        } else {
            "Add monogram"
        };
        let group = replace.map(|r| r.group);
        let ids = self.doc.edit(label, |d| {
            emb_letter::monogram::add_to_design(d, &w.initials, &w.spec, m, center, group)
        });
        self.selection.set_all(ids.iter().copied());
        if let Some(g) = self.doc.design().object(ids[0]).and_then(|o| o.group) {
            self.monogram_window.replace = Some(g);
        }
    }

    /// Saves the spec of the window as a user template.
    pub fn save_monogram_template(&mut self) -> Result<PathBuf, String> {
        let name = self.monogram_window.template_name.trim().to_string();
        if name.is_empty() {
            return Err("Type a name for the template.".into());
        }
        let dir = templates_dir().ok_or("No folder for templates.")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = dir.join(format!("{file}.{TEMPLATE_EXTENSION}"));
        let t = MonogramTemplate {
            name,
            spec: self.monogram_window.spec.clone(),
        };
        std::fs::write(&path, t.to_json()).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

fn rgb_edit(ui: &mut egui::Ui, label: &str, c: &mut Rgb) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut v = [c.0, c.1, c.2];
        if ui.color_edit_button_srgb(&mut v).changed() {
            *c = Rgb(v[0], v[1], v[2]);
        }
    });
}

fn combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    salt: &str,
    label: &str,
    v: &mut T,
    all: &[T],
    name: fn(T) -> &'static str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(salt)
            .selected_text(name(*v))
            .show_ui(ui, |ui| {
                for x in all {
                    ui.selectable_value(v, *x, name(*x));
                }
            });
    });
}

/// Draws the objects, each in its color, fitted into `rect`.
fn draw_preview(painter: &egui::Painter, rect: Rect, objs: &[(Rgb, &ObjectKind)]) {
    let Some(bb) = objs.iter().filter_map(|(_, k)| k.control_bbox()).reduce(BBox::union) else {
        return;
    };
    let pad = 8.0;
    let s = ((rect.width() - 2.0 * pad) as f64 / bb.width().max(1e-6))
        .min((rect.height() - 2.0 * pad) as f64 / bb.height().max(1e-6));
    let off = Vec2::new(
        (rect.width() - (bb.width() * s) as f32) / 2.0,
        (rect.height() - (bb.height() * s) as f32) / 2.0,
    );
    let to = |p: Point| rect.min + off + Vec2::new(((p.x - bb.min.x) * s) as f32, ((p.y - bb.min.y) * s) as f32);
    for (c, k) in objs {
        let color = Color32::from_rgb(c.0, c.1, c.2);
        let parts: Vec<&ObjectKind> = match k {
            ObjectKind::Lettering(l) => l.parts().collect(),
            k => vec![*k],
        };
        for part in parts {
            for path in part.paths() {
                for pl in path.flatten(0.1) {
                    let mut pts: Vec<Pos2> = pl.points.into_iter().map(to).collect();
                    if pl.closed
                        && let Some(&f) = pts.first()
                    {
                        pts.push(f);
                    }
                    painter.add(egui::Shape::line(pts, Stroke::new(1.2, color)));
                }
            }
        }
    }
}

/// The Monogram window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.monogram_window.open {
        return;
    }
    let mut open = true;
    let (mut place, mut save) = (false, false);
    let mut w = st.monogram_window.clone();
    crate::dock::docked(
        egui::Window::new(if w.replace.is_some() {
            tr("Monogram (Selected)")
        } else {
            tr("Monogram")
        })
        .id(egui::Id::new("Monogram")),
        ctx,
        egui::Id::new("Monogram"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(420.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Initials"))
                .on_hover_text(tr("One to three letters, for example \"JMK\"."));
            ui.add(
                egui::TextEdit::singleline(&mut w.initials)
                    .desired_width(80.0)
                    .char_limit(5),
            );
        });
        ui.label(tr("Templates"));
        ui.horizontal_wrapped(|ui| {
            for t in templates() {
                if ui
                    .button(&t.name)
                    .on_hover_text(tr("Use the style, border, ornaments and colors of this template."))
                    .clicked()
                {
                    w.spec = t.spec.clone();
                }
            }
        });
        ui.separator();
        combo(
            ui,
            "mono_style",
            tr("Style"),
            &mut w.spec.style,
            &MonogramStyle::ALL,
            MonogramStyle::name,
        );
        crate::lettering::font_picker(ui, st, &mut w.spec.font, "monogram");
        ui.horizontal(|ui| {
            ui.label(tr("Height"))
                .on_hover_text(tr("The height of the largest initial."));
            ui.add(
                egui::DragValue::new(&mut w.spec.height_mm)
                    .range(5.0..=200.0)
                    .clamp_existing_to_range(false)
                    .speed(0.2)
                    .suffix(" mm"),
            );
        });
        ui.separator();
        ui.strong(tr("Border"));
        combo(
            ui,
            "mono_border",
            tr("Shape"),
            &mut w.spec.border.shape,
            &BorderShape::ALL,
            BorderShape::name,
        );
        if w.spec.border.shape != BorderShape::None {
            combo(
                ui,
                "mono_border_stitch",
                tr("Stitch"),
                &mut w.spec.border.stitch,
                &BorderStitch::ALL,
                BorderStitch::name,
            );
            ui.horizontal(|ui| {
                if w.spec.border.stitch == BorderStitch::Satin {
                    ui.label(tr("Width"));
                    ui.add(
                        egui::DragValue::new(&mut w.spec.border.width_mm)
                            .range(0.8..=10.0)
                            .clamp_existing_to_range(false)
                            .speed(0.05)
                            .suffix(" mm"),
                    );
                }
                ui.label(tr("Space"))
                    .on_hover_text(tr("The space between the initials and the border."));
                ui.add(
                    egui::DragValue::new(&mut w.spec.border.margin_mm)
                        .range(0.0..=30.0)
                        .clamp_existing_to_range(false)
                        .speed(0.1)
                        .suffix(" mm"),
                );
            });
        }
        ui.strong(tr("Ornaments"));
        combo(
            ui,
            "mono_orn",
            tr("Shape"),
            &mut w.spec.ornaments.shape,
            &OrnamentShape::ALL,
            OrnamentShape::name,
        );
        if w.spec.ornaments.shape != OrnamentShape::None {
            combo(
                ui,
                "mono_orn_place",
                tr("Place"),
                &mut w.spec.ornaments.place,
                &OrnamentPlace::ALL,
                OrnamentPlace::name,
            );
            ui.horizontal(|ui| {
                ui.label(tr("Size"));
                ui.add(
                    egui::DragValue::new(&mut w.spec.ornaments.size_mm)
                        .range(1.0..=60.0)
                        .clamp_existing_to_range(false)
                        .speed(0.1)
                        .suffix(" mm"),
                );
            });
        }
        ui.horizontal(|ui| {
            rgb_edit(ui, tr("Initials"), &mut w.spec.letter_color);
            rgb_edit(ui, tr("Border"), &mut w.spec.border_color);
            rgb_edit(ui, tr("Ornaments"), &mut w.spec.ornament_color);
        });
        // Preview.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(400.0, 180.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        st.monogram_window = w.clone();
        match st.monogram_objects(Point::ZERO) {
            Some(m) => {
                let letters = ObjectKind::Lettering(Box::new(m.letters));
                let mut objs: Vec<(Rgb, &ObjectKind)> = vec![(w.spec.letter_color, &letters)];
                objs.extend(m.ornaments.iter().map(|k| (w.spec.ornament_color, k)));
                objs.extend(m.borders.iter().map(|k| (w.spec.border_color, k)));
                draw_preview(ui.painter(), rect, &objs);
            }
            None => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Type initials, or choose a font that has them.",
                    egui::FontId::proportional(13.0),
                    ui.visuals().weak_text_color(),
                );
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            let text = if w.replace.is_some() {
                "Change the monogram"
            } else {
                "Add to the design"
            };
            place = ui
                .add_enabled(!w.initials.trim().is_empty(), egui::Button::new(text))
                .clicked();
            if w.replace.is_some() && ui.button(tr("Add as new")).clicked() {
                w.replace = None;
                place = true;
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut w.template_name)
                    .hint_text(tr("Template name"))
                    .desired_width(160.0),
            );
            save = ui.button(tr("Save as Template")).clicked();
        });
    });
    w.open = open;
    st.monogram_window = w;
    if place {
        st.place_monogram();
    }
    if save {
        st.message = match st.save_monogram_template() {
            Ok(p) => format!("Saved template {}", p.display()),
            Err(e) => e,
        };
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn a_three_letter_monogram_with_a_border_from_a_template() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.open_monogram();
        assert!(st.monogram_window.open && st.monogram_window.replace.is_none());
        let t = templates()
            .into_iter()
            .find(|t| t.name == "Classic in a circle")
            .unwrap();
        st.monogram_window.spec = t.spec;
        st.monogram_window.initials = "JMK".into();
        st.place_monogram();
        let d = st.doc.design();
        assert_eq!(d.objects().len(), 2, "initials and border");
        assert_eq!(d.monograms.len(), 1);
        assert_eq!(st.selection.ids().len(), 2);
        let plan = st.plan_view().plan.clone();
        assert!(plan.stitches.stitch_count() > 1000);
        assert_eq!(plan.stitches.color_count(), 2);
        // Select it again: the window changes it.
        st.monogram_window = MonogramWindow::default();
        st.open_monogram();
        assert_eq!(st.monogram_window.initials, "JMK");
        assert!(st.monogram_window.replace.is_some());
        st.monogram_window.spec.border.shape = BorderShape::DoubleCircle;
        st.place_monogram();
        assert_eq!(st.doc.design().objects().len(), 3);
        assert_eq!(st.doc.design().monograms.len(), 1);
        st.undo();
        assert_eq!(st.doc.design().objects().len(), 2);
        st.undo();
        assert!(st.doc.design().objects().is_empty() && st.doc.design().monograms.is_empty());
    }

    #[test]
    fn right_click_edit_opens_the_monogram_or_the_text() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.open_monogram();
        st.monogram_window.initials = "AB".into();
        st.place_monogram();
        st.monogram_window = MonogramWindow::default();
        // One object of the monogram is enough.
        let first = st.selection.ids()[0];
        st.selection.set(first);
        assert_eq!(st.edit_target(), Some(crate::lettering::EditTarget::Monogram));
        assert!(st.edit_selection());
        assert!(st.monogram_window.open && st.monogram_window.replace.is_some());
        assert_eq!(st.monogram_window.initials, "AB");
        // The menu shows the item (and draws with no edit).
        let before = st.doc.undo_len();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::commands::object_menu(ui, &mut st)
        });
        out.textures_delta.clear();
        assert_eq!(st.doc.undo_len(), before);
        // Plain lettering: the Lettering window with its text.
        st.add_lettering(crate::lettering::LetteringDialog::new(
            Point::new(0.0, 60.0),
            "Zoe",
            emb_model::FontRef::embroidery("Block Sans", ""),
            15.0,
        ));
        let id = st.selection.single().unwrap();
        assert_eq!(st.edit_target(), Some(crate::lettering::EditTarget::Lettering(id)));
        assert!(st.edit_selection());
        assert_eq!(st.lettering_dialog.as_ref().map(|d| d.text.as_str()), Some("Zoe"));
        // Nothing to edit for an empty selection.
        st.selection.clear();
        assert!(!st.edit_selection());
    }

    #[test]
    fn the_window_draws() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.open_monogram();
        st.monogram_window.spec = MonogramTemplate::builtin(&default_font())[4].spec.clone();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.monogram_window.open);
    }
}
