//! The Keyboard designs window (P4 Milestone 6, ADR 0020): type with a
//! keyboard design collection (a design for each key), and map the
//! selected objects to a key of your own collection.
//!
//! Collections: the built-in "Shadow" one (from Block Sans), and the user's
//! `.embkdc` files in the `keyboard` folder next to the settings file.

use std::path::PathBuf;

use eframe::egui;
use emb_geom::Point;
use emb_model::keyboard::FILE_EXTENSION;
use emb_model::{KeyboardCollection, ObjectKind, Rgb};

use crate::app::AppState;
use crate::i18n::tr;

#[derive(Debug, Clone)]
pub struct KeyboardWindow {
    pub open: bool,
    /// The index of the collection in the list.
    pub selected: usize,
    pub text: String,
    pub height_mm: f64,
    pub spacing_mm: f64,
    /// The key for Map selection.
    pub key: String,
    /// The name of a new collection.
    pub new_name: String,
}

impl Default for KeyboardWindow {
    fn default() -> Self {
        KeyboardWindow {
            open: false,
            selected: 0,
            text: "ABC".into(),
            height_mm: 25.0,
            spacing_mm: 1.0,
            key: "A".into(),
            new_name: String::new(),
        }
    }
}

/// The folder of the user's collections.
pub fn collections_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("keyboard"))
}

/// The built-in collection and the user's (with their files).
pub fn collections() -> Vec<(KeyboardCollection, Option<PathBuf>)> {
    let mut all = Vec::new();
    if let Some(f) = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY) {
        all.push((
            emb_letter::keyboard::shadow_collection(&f, "Shadow (built in)", Rgb(25, 35, 90), Rgb(180, 140, 40)),
            None,
        ));
    }
    if let Some(Ok(rd)) = collections_dir().map(std::fs::read_dir) {
        let mut user: Vec<(KeyboardCollection, Option<PathBuf>)> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(FILE_EXTENSION))
            .filter_map(|p| {
                let k = KeyboardCollection::from_json(&std::fs::read_to_string(&p).ok()?).ok()?;
                Some((k, Some(p)))
            })
            .collect();
        user.sort_by_key(|(k, _)| k.name.to_lowercase());
        all.extend(user);
    }
    all
}

impl AppState {
    /// Adds the typed text of the window in the middle of the hoop, as one
    /// group (one undo step).
    pub fn type_keyboard(&mut self, k: &KeyboardCollection) {
        let w = self.keyboard_window.clone();
        let (objs, missing) = emb_letter::keyboard::type_text(k, &w.text, w.height_mm, w.spacing_mm, Point::ZERO);
        if objs.is_empty() {
            self.message = crate::i18n::fill("{} has none of these keys.", &[&k.name]);
            return;
        }
        let b = objs
            .iter()
            .filter_map(|(_, o)| o.control_bbox())
            .reduce(emb_geom::BBox::union)
            .unwrap_or(emb_geom::BBox::new(Point::ZERO, Point::ZERO));
        let c = self.doc.design().hoop.center;
        let t = emb_geom::Transform::translate(c.x - b.center().x, c.y - b.center().y);
        let ids = self.doc.edit("Keyboard Designs", |d| {
            let g = d.new_group_id();
            objs.into_iter()
                .map(|(color, mut kind)| {
                    kind.transform(&t);
                    let thread = d.thread_for_color(color);
                    let id = d.add_object(thread, kind);
                    if let Some(o) = d.object_mut(id) {
                        o.group = Some(g);
                    }
                    id
                })
                .collect::<Vec<_>>()
        });
        self.selection.set_all(ids);
        if !missing.is_empty() {
            self.message = crate::i18n::fill(
                "{} has no design for: {}",
                &[
                    &k.name,
                    &(missing.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")),
                ],
            );
        }
    }

    /// Maps the selected objects (with their colors) to `key` in the
    /// collection file `path`, and saves it.
    pub fn map_selection_to_key(&mut self, path: &std::path::Path, key: char) -> Result<(), String> {
        let d = self.doc.design();
        let objs: Vec<(Rgb, ObjectKind)> = self
            .sel()
            .iter()
            .filter_map(|id| d.object(*id))
            .map(|o| (d.thread(o.thread).map_or(Rgb(0, 0, 0), |t| t.color), o.kind.clone()))
            .collect();
        if objs.is_empty() {
            return Err("Select the objects of the key design first.".into());
        }
        let mut k = KeyboardCollection::from_json(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?;
        let design = emb_letter::keyboard::key_from_objects(&objs, k.height_mm).ok_or("The selection has no size.")?;
        k.keys.insert(key.to_string(), design);
        std::fs::write(path, k.to_json()).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// A new empty collection file. Returns its path.
    pub fn new_keyboard_collection(&mut self, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Type a name for the collection.".into());
        }
        let dir = collections_dir().ok_or("No folder for collections.")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " -_".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = dir.join(format!("{file}.{FILE_EXTENSION}"));
        std::fs::write(&path, KeyboardCollection::new(name, 25.0).to_json()).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// The Keyboard designs window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.keyboard_window.open {
        return;
    }
    let all = collections();
    let mut w = st.keyboard_window.clone();
    w.selected = w.selected.min(all.len().saturating_sub(1));
    let mut open = true;
    let (mut add, mut map, mut create) = (false, false, false);
    crate::dock::docked(
        egui::Window::new(tr("Keyboard Designs")).id(egui::Id::new("Keyboard Designs")),
        ctx,
        egui::Id::new("Keyboard Designs"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(380.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Collection"));
            egui::ComboBox::from_id_salt("kdc")
                .selected_text(all.get(w.selected).map_or("", |(k, _)| k.name.as_str()))
                .show_ui(ui, |ui| {
                    for (i, (k, _)) in all.iter().enumerate() {
                        ui.selectable_value(&mut w.selected, i, format!("{} ({} keys)", k.name, k.keys.len()));
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label(tr("Text"));
            ui.add(egui::TextEdit::singleline(&mut w.text).desired_width(200.0));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Height"));
            ui.add(
                egui::DragValue::new(&mut w.height_mm)
                    .range(5.0..=200.0)
                    .clamp_existing_to_range(false)
                    .speed(0.2)
                    .suffix(" mm"),
            );
            ui.label(tr("Spacing"));
            ui.add(
                egui::DragValue::new(&mut w.spacing_mm)
                    .range(-10.0..=30.0)
                    .clamp_existing_to_range(false)
                    .speed(0.05)
                    .suffix(" mm"),
            );
        });
        add = ui
            .add_enabled(!w.text.trim().is_empty(), egui::Button::new(tr("Add to the design")))
            .clicked();
        ui.separator();
        ui.strong(tr("Your collections"));
        let editable = all.get(w.selected).is_some_and(|(_, p)| p.is_some());
        ui.horizontal(|ui| {
            ui.label(tr("Key"));
            ui.add(egui::TextEdit::singleline(&mut w.key).desired_width(30.0).char_limit(1));
            map = ui
                .add_enabled(
                    editable && !w.key.is_empty(),
                    egui::Button::new(tr("Map the selection to this key")),
                )
                .on_hover_text(tr(
                    "The selected objects, in their colors, become the design of this key.",
                ))
                .on_disabled_hover_text(tr("Choose one of your collections (or make a new one) first."))
                .clicked();
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut w.new_name)
                    .hint_text(tr("New collection name"))
                    .desired_width(160.0),
            );
            create = ui.button(tr("New collection")).clicked();
        });
    });
    w.open = open;
    st.keyboard_window = w.clone();
    if add && let Some((k, _)) = all.get(w.selected) {
        st.type_keyboard(k);
    }
    if map && let (Some((_, Some(path))), Some(c)) = (all.get(w.selected), w.key.chars().next()) {
        st.message = match st.map_selection_to_key(path, c) {
            Ok(()) => format!("The selection is now the design of \"{c}\"."),
            Err(e) => e,
        };
    }
    if create {
        match st.new_keyboard_collection(&w.new_name) {
            Ok(p) => {
                let i = collections()
                    .iter()
                    .position(|(_, q)| q.as_deref() == Some(p.as_path()));
                st.keyboard_window.selected = i.unwrap_or(0);
                st.keyboard_window.new_name.clear();
                st.message = crate::i18n::fill("New collection {}", &[&(p.display())]);
            }
            Err(e) => st.message = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn type_with_the_built_in_collection() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let all = collections();
        st.keyboard_window.text = "AB".into();
        st.type_keyboard(&all[0].0);
        let d = st.doc.design();
        assert!(d.objects().len() >= 4);
        assert_eq!(d.threads.len(), 2, "letter and shadow");
        let g = d.objects()[0].group;
        assert!(g.is_some() && d.objects().iter().all(|o| o.group == g));
        st.undo();
        assert!(st.doc.design().objects().is_empty());
    }

    #[test]
    fn map_objects_to_a_key() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let dir = std::env::temp_dir().join(format!("emb-kdc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mine.embkdc");
        std::fs::write(&path, KeyboardCollection::new("Mine", 20.0).to_json()).unwrap();
        st.keyboard_window.text = "A".into();
        st.type_keyboard(&collections()[0].0);
        st.select_all();
        st.map_selection_to_key(&path, 'Z').unwrap();
        let k = KeyboardCollection::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let z = k.key('Z').unwrap();
        assert!(z.objects.len() >= 2 && z.advance_mm > 10.0);
        let colors: std::collections::BTreeSet<_> = z.objects.iter().map(|o| o.color.to_hex()).collect();
        assert_eq!(colors.len(), 2, "the key keeps both colors");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_window_draws() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.keyboard_window.open = true;
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.keyboard_window.open);
    }
}
