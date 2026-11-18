//! The photo of the user's own product, better (P16 Milestone 7, ADR
//! 0107): the window that marks the mask (the part of the photo that
//! takes the product color) and the usual places for a design, and the
//! user's product templates.
//!
//! The mask changes in the window (`MaskGrid`) while the user draws; one
//! click or one brush stroke is one undo step. A product template is a
//! `.embproduct` file in the `products` folder next to the settings: the
//! photo, its real width, its mask and its places.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, Rect, Sense, TextureHandle, TextureOptions};
use emb_geom::Point;
use emb_model::{PhotoPlacement, ProductPicture, product};
use emb_render::photo::MaskGrid;

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::{fill, tr};

/// The file extension of a product template.
pub const TEMPLATE_EXT: &str = "embproduct";
/// The largest side (pixels) of the photo in the window.
const VIEW_SIDE: f32 = 520.0;

/// What a click or a drag on the photo does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    /// Adds the area of the color at the click to the mask.
    #[default]
    Pick,
    /// Removes the area of the color at the click from the mask.
    Unpick,
    Brush,
    Eraser,
    /// Marks a usual place for a design.
    Place,
}

/// The state of the photo window.
pub struct PhotoEditor {
    pub open: bool,
    pub tool: Tool,
    /// The tolerance of a pick (0 to 100).
    pub tolerance: f64,
    /// The width of the brush in mm.
    pub brush_mm: f64,
    /// The photo at the size of the mask grid, and its texture (by the
    /// address of the photo data).
    photo: Option<(usize, Arc<image::RgbaImage>, TextureHandle)>,
    /// The cells of the mask, and the address of the runs of the mask of
    /// the design that they show (0: no mask).
    grid: Option<(usize, MaskGrid)>,
    overlay: Option<TextureHandle>,
    /// The overlay must be made again.
    dirty: bool,
    /// The last point (cells) of the brush stroke that is drawn now.
    stroke: Option<(f64, f64)>,
}

impl Default for PhotoEditor {
    fn default() -> Self {
        PhotoEditor {
            open: false,
            tool: Tool::Pick,
            tolerance: 30.0,
            brush_mm: 20.0,
            photo: None,
            grid: None,
            overlay: None,
            dirty: true,
            stroke: None,
        }
    }
}

/// A saved product template: its name (the file name) and its file.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub name: String,
    pub path: PathBuf,
}

/// The folder of the user's product templates, next to the settings file.
pub fn templates_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("products"))
}

/// The product templates in `dir`, by name.
pub fn list_templates(dir: &Path) -> Vec<Template> {
    let mut out: Vec<Template> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(TEMPLATE_EXT)))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(Template { name, path })
        })
        .collect();
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// A file name for a template named `name`: the name with no characters
/// that a file name cannot have.
fn file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() { "product".into() } else { s }
}

/// Saves `p` as the template `name` in `dir` (a template of the same name
/// is replaced). Returns its file.
pub fn save_template(dir: &Path, name: &str, p: &ProductPicture) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.{TEMPLATE_EXT}", file_name(name)));
    write_template(&path, p)?;
    Ok(path)
}

/// Writes `p` to the template file `path`.
pub fn write_template(path: &Path, p: &ProductPicture) -> Result<(), String> {
    std::fs::write(path, product::template_to_string(p)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads the template file `path` and checks its photo.
pub fn load_template(path: &Path) -> Result<ProductPicture, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let p = product::template_from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    emb_art::decode_fit(&p.data, 64, crate::resources::decode_budget())
        .map_err(|e| format!("{}: {}", path.display(), crate::resources::decode_error(&e)))?;
    Ok(p)
}

impl AppState {
    /// The user's product templates (read again after a change).
    pub fn product_templates(&mut self) -> Vec<Template> {
        if self.product_templates.is_none() {
            self.product_templates = Some(templates_dir().map(|d| list_templates(&d)).unwrap_or_default());
        }
        self.product_templates.clone().unwrap_or_default()
    }

    /// Puts the product of the template `t` under the design.
    pub fn use_product_template(&mut self, t: &Template) {
        match load_template(&t.path) {
            Ok(p) => {
                self.put_product_picture(p);
                self.message = fill("The product is {}.", &[&t.name]);
            }
            Err(e) => self.cannot("Cannot use the product template", e),
        }
    }

    /// Saves the photo of the product as a template of the user.
    pub fn save_product_template(&mut self, name: &str) {
        let Some(p) = self.doc.design().product.as_ref().and_then(|v| v.picture.clone()) else {
            return;
        };
        let r = templates_dir()
            .ok_or_else(|| tr("There is no folder for the settings.").to_string())
            .and_then(|d| save_template(&d, name, &p));
        match r {
            Ok(_) => {
                self.product_templates = None;
                self.message = fill("{} is in My Products.", &[&name]);
            }
            Err(e) => self.cannot("Cannot save the product template", e),
        }
    }

    /// Writes the photo of the product as a template file that the user
    /// chooses (to give it to another computer).
    pub fn export_product_template(&mut self) {
        let Some(p) = self.doc.design().product.as_ref().and_then(|v| v.picture.clone()) else {
            return;
        };
        let stem = p.name.rsplit_once('.').map_or(p.name.as_str(), |(s, _)| s).to_string();
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Export the product"))
            .add_filter(tr("Product templates"), &[TEMPLATE_EXT])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{}.{TEMPLATE_EXT}", file_name(&stem)))
            .save_file()
        else {
            return;
        };
        match write_template(&path, &p) {
            Ok(()) => self.message = fill("Saved {}.", &[&path.display().to_string()]),
            Err(e) => self.cannot("Cannot export the product", e),
        }
    }

    /// Reads a template file that the user chooses, adds it to My Products
    /// and puts it under the design.
    pub fn import_product_template(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Import a product"))
            .add_filter(tr("Product templates"), &[TEMPLATE_EXT])
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        let name = path
            .file_stem()
            .map_or_else(|| "product".to_string(), |s| s.to_string_lossy().into_owned());
        let r = load_template(&path).and_then(|p| {
            let dir = templates_dir().ok_or_else(|| tr("There is no folder for the settings.").to_string())?;
            save_template(&dir, &name, &p)?;
            Ok(p)
        });
        match r {
            Ok(p) => {
                self.product_templates = None;
                self.put_product_picture(p);
                self.message = fill("{} is in My Products.", &[&name]);
            }
            Err(e) => self.cannot("Cannot import the product", e),
        }
    }

    fn cannot(&mut self, title: &str, e: String) {
        self.notice = Some(Notice {
            title: title.into(),
            lines: vec![e],
        });
    }

    /// Changes the photo of the product (one undo step).
    fn edit_photo(&mut self, label: &str, merge: Option<&str>, f: impl FnOnce(&mut ProductPicture)) {
        self.edit_product(label, merge, |v| {
            if let Some(p) = v.picture.as_mut() {
                f(p);
            }
        });
    }

    /// Makes the mask of the photo the cells `g` (one undo step).
    fn set_photo_mask(&mut self, g: &MaskGrid) {
        let mask = g.to_mask();
        let mask = (!mask.is_empty()).then_some(mask);
        self.edit_photo("Photo mask", None, |p| p.mask = mask);
        let at = self.mask_address();
        if let Some((a, _)) = &mut self.photo_editor.grid {
            *a = at;
        }
    }

    /// The address of the runs of the mask of the photo (0: no mask).
    fn mask_address(&self) -> usize {
        self.doc
            .design()
            .product
            .as_ref()
            .and_then(|v| v.picture.as_ref())
            .and_then(|p| p.mask.as_ref())
            .map_or(0, |m| Arc::as_ptr(&m.runs) as usize)
    }

    /// The photo at the size of its mask grid (decoded once).
    fn editor_photo(
        &mut self,
        ctx: &egui::Context,
        p: &ProductPicture,
    ) -> Option<(Arc<image::RgbaImage>, TextureHandle)> {
        let at = Arc::as_ptr(&p.data) as usize;
        if let Some((a, img, t)) = &self.photo_editor.photo
            && *a == at
        {
            return Some((img.clone(), t.clone()));
        }
        let (gw, gh) = emb_model::PhotoMask::grid_for(p.pixels);
        let img = emb_art::decode_fit(&p.data, gw.max(gh), crate::resources::decode_budget()).ok()?;
        let img = if img.dimensions() == (gw, gh) {
            img
        } else {
            image::imageops::resize(&img, gw, gh, image::imageops::FilterType::Triangle)
        };
        let ci = egui::ColorImage::from_rgba_unmultiplied([gw as usize, gh as usize], img.as_raw());
        let t = crate::resources::load_texture(ctx, "product-photo-edit", ci, TextureOptions::LINEAR);
        let img = Arc::new(img);
        self.photo_editor.photo = Some((at, img.clone(), t.clone()));
        self.photo_editor.grid = None;
        Some((img, t))
    }

    /// The cells of the mask in the window: the mask of the design, unless
    /// the user draws now.
    fn editor_grid(&mut self, p: &ProductPicture) -> &mut MaskGrid {
        let at = self.mask_address();
        let drawing = self.photo_editor.stroke.is_some();
        let stale = self
            .photo_editor
            .grid
            .as_ref()
            .is_none_or(|(a, _)| *a != at && !drawing);
        if stale {
            let g = p
                .mask
                .as_ref()
                .map_or_else(|| MaskGrid::for_photo(p.pixels), MaskGrid::of);
            self.photo_editor.grid = Some((at, g));
            self.photo_editor.dirty = true;
        }
        &mut self.photo_editor.grid.as_mut().expect("set above").1
    }

    /// Adds (`on`) or removes the area of the color at the cell `cell` of
    /// the photo (one undo step).
    pub fn pick_photo_color(&mut self, ctx: &egui::Context, cell: (u32, u32), on: bool) {
        let Some(p) = self.doc.design().product.as_ref().and_then(|v| v.picture.clone()) else {
            return;
        };
        let Some((img, _)) = self.editor_photo(ctx, &p) else {
            return;
        };
        let tolerance = self.photo_editor.tolerance;
        let g = self.editor_grid(&p);
        g.pick(&img, cell, tolerance, on);
        let g = g.clone();
        self.photo_editor.dirty = true;
        self.set_photo_mask(&g);
    }
}

/// The texture that shows the mask over the photo.
fn overlay(ctx: &egui::Context, g: &MaskGrid) -> TextureHandle {
    let tint = Color32::from_rgba_unmultiplied(255, 0, 170, 120);
    let pixels: Vec<Color32> =
        g.on.iter()
            .map(|&c| if c { tint } else { Color32::TRANSPARENT })
            .collect();
    let ci = egui::ColorImage {
        size: [g.width as usize, g.height as usize],
        source_size: egui::vec2(g.width as f32, g.height as f32),
        pixels,
    };
    crate::resources::load_texture(ctx, "product-photo-mask", ci, TextureOptions::NEAREST)
}

/// The photo window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.photo_editor.open {
        return;
    }
    let has_photo = st.doc.design().product.as_ref().is_some_and(|v| v.picture.is_some());
    if !has_photo {
        st.photo_editor.open = false;
        return;
    }
    let mut open = true;
    egui::Window::new(tr("Product Photo"))
        .id(egui::Id::new("ProductPhoto"))
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| contents(ui, st));
    if !open {
        st.photo_editor.open = false;
        st.photo_editor.stroke = None;
    }
}

fn contents(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(v) = st.doc.design().product.clone() else {
        return;
    };
    let Some(p) = v.picture.clone() else { return };
    ui.label(tr(
        "Mark the part of the photo that takes the product color, and the usual places for a design.",
    ));
    ui.horizontal_wrapped(|ui| {
        for (tool, name, tip) in [
            (
                Tool::Pick,
                tr("Pick Color"),
                tr("Click the product: the area of that color takes the product color."),
            ),
            (
                Tool::Unpick,
                tr("Remove Color"),
                tr("Click an area: it keeps the colors of the photo."),
            ),
            (
                Tool::Brush,
                tr("Brush"),
                tr("Paint the part that takes the product color."),
            ),
            (
                Tool::Eraser,
                tr("Eraser"),
                tr("Paint the part that keeps the colors of the photo."),
            ),
            (
                Tool::Place,
                tr("Mark a Place"),
                tr("Click the usual place of a design."),
            ),
        ] {
            if ui
                .selectable_label(st.photo_editor.tool == tool, name)
                .on_hover_text(tip)
                .clicked()
            {
                st.photo_editor.tool = tool;
            }
        }
    });
    ui.horizontal(|ui| match st.photo_editor.tool {
        Tool::Pick | Tool::Unpick => {
            ui.label(tr("Tolerance"));
            ui.add(egui::Slider::new(&mut st.photo_editor.tolerance, 0.0..=100.0).max_decimals(0));
        }
        Tool::Brush | Tool::Eraser => {
            ui.label(tr("Brush size"));
            let units = st.settings.units;
            crate::workspace::length_value(ui, &mut st.photo_editor.brush_mm, units, 1.0, 300.0);
        }
        Tool::Place => {
            ui.weak(tr("Set the name and the size of each place in the list."));
        }
    });
    ui.horizontal(|ui| {
        if ui
            .button(tr("Clear the Mask"))
            .on_hover_text(tr("The whole photo keeps its colors."))
            .clicked()
        {
            st.photo_editor.grid = None;
            st.edit_photo("Photo mask", None, |p| p.mask = None);
        }
        if ui
            .button(tr("Invert the Mask"))
            .on_hover_text(tr(
                "The masked part keeps its colors and the rest takes the product color.",
            ))
            .clicked()
        {
            let g = st.editor_grid(&p);
            g.invert();
            let g = g.clone();
            st.photo_editor.dirty = true;
            st.set_photo_mask(&g);
        }
    });
    photo_area(ui, st, &p, v.origin);
    places(ui, st, &p);
}

/// The photo with the mask over it, where the user clicks and paints.
fn photo_area(ui: &mut egui::Ui, st: &mut AppState, p: &ProductPicture, origin: Point) {
    let ctx = ui.ctx().clone();
    let Some((img, photo_tex)) = st.editor_photo(&ctx, p) else {
        ui.colored_label(ui.visuals().warn_fg_color, tr("The photo cannot be read."));
        return;
    };
    let (gw, gh) = img.dimensions();
    let k = (VIEW_SIDE / gw.max(gh) as f32).min(ui.available_width() / gw as f32);
    let size = egui::vec2(gw as f32 * k, gh as f32 * k);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let resp = crate::a11y::named(resp, tr("Photo of the product"));
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    ui.painter().image(photo_tex.id(), rect, uv, Color32::WHITE);
    let to_cell = |pos: egui::Pos2| {
        (
            f64::from((pos.x - rect.min.x) / k).clamp(0.0, f64::from(gw) - 0.01),
            f64::from((pos.y - rect.min.y) / k).clamp(0.0, f64::from(gh) - 0.01),
        )
    };
    let mm_per_cell = p.width_mm / f64::from(gw);
    match st.photo_editor.tool {
        Tool::Pick | Tool::Unpick => {
            if resp.clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let (x, y) = to_cell(pos);
                st.pick_photo_color(&ctx, (x as u32, y as u32), st.photo_editor.tool == Tool::Pick);
            }
        }
        Tool::Brush | Tool::Eraser => {
            let on = st.photo_editor.tool == Tool::Brush;
            let radius = st.photo_editor.brush_mm / 2.0 / mm_per_cell;
            if (resp.dragged() || resp.drag_started() || resp.clicked())
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let c = to_cell(pos);
                let last = st.photo_editor.stroke.unwrap_or(c);
                st.editor_grid(p).brush_line(last, c, radius, on);
                st.photo_editor.stroke = Some(c);
                st.photo_editor.dirty = true;
            }
            if (resp.drag_stopped() || resp.clicked()) && st.photo_editor.stroke.take().is_some() {
                let g = st.editor_grid(p).clone();
                st.set_photo_mask(&g);
            }
            // The size of the brush at the pointer.
            if let Some(pos) = resp.hover_pos() {
                ui.painter().circle_stroke(
                    pos,
                    (radius as f32) * k,
                    egui::Stroke::new(1.0, ui.visuals().strong_text_color()),
                );
            }
        }
        Tool::Place => {
            if resp.clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let (x, y) = to_cell(pos);
                let at = Point::new(x * mm_per_cell, y * mm_per_cell);
                let name = fill("Place {}", &[&(p.placements.len() + 1).to_string()]);
                st.edit_photo("Mark a place", None, |q| {
                    q.placements.push(PhotoPlacement {
                        name,
                        at,
                        max: (100.0, 100.0),
                    });
                });
            }
        }
    }
    // The mask over the photo.
    let g = st.editor_grid(p).clone();
    if st.photo_editor.dirty || st.photo_editor.overlay.is_none() {
        st.photo_editor.overlay = Some(overlay(&ctx, &g));
        st.photo_editor.dirty = false;
    }
    if let Some(t) = &st.photo_editor.overlay {
        ui.painter().image(t.id(), rect, uv, Color32::WHITE);
    }
    // The places, and the design at its place.
    let to_screen = |q: Point| rect.min + egui::vec2((q.x / mm_per_cell) as f32 * k, (q.y / mm_per_cell) as f32 * k);
    let design = st.doc.design().view_extents().map(|b| b.center() - origin);
    for q in &p.placements {
        let half = Point::new(q.max.0 / 2.0, q.max.1 / 2.0);
        let r = Rect::from_min_max(to_screen(q.at - half), to_screen(q.at + half));
        let on = design.is_some_and(|c| c.distance(q.at) < 1.0);
        let color = if on {
            ui.visuals().selection.stroke.color
        } else {
            Color32::from_rgb(255, 200, 0)
        };
        ui.painter()
            .rect_stroke(r, 2.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Middle);
        ui.painter().text(
            r.left_top() + egui::vec2(3.0, 2.0),
            egui::Align2::LEFT_TOP,
            &q.name,
            egui::FontId::proportional(12.0),
            color,
        );
    }
}

/// The list of the places marked on the photo.
fn places(ui: &mut egui::Ui, st: &mut AppState, p: &ProductPicture) {
    if p.placements.is_empty() {
        return;
    }
    ui.add_space(4.0);
    ui.strong(tr("Places for a design"));
    let units = st.settings.units;
    let mut remove = None;
    let mut put = None;
    for (i, q) in p.placements.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut name = q.name.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut name).desired_width(110.0))
                .changed()
            {
                st.edit_photo("Place name", Some(&format!("place-name-{i}")), |p| {
                    if let Some(q) = p.placements.get_mut(i) {
                        q.name = name;
                    }
                });
            }
            ui.label(tr("Largest design"));
            let (mut w, mut h) = q.max;
            let rw = crate::workspace::length_value(ui, &mut w, units, 5.0, 1000.0);
            ui.label("×");
            let rh = crate::workspace::length_value(ui, &mut h, units, 5.0, 1000.0);
            if rw.changed() || rh.changed() {
                st.edit_photo("Place size", Some(&format!("place-size-{i}")), |p| {
                    if let Some(q) = p.placements.get_mut(i) {
                        q.max = (w, h);
                    }
                });
            }
            if ui
                .button(tr("Put the Design Here"))
                .on_hover_text(tr("The center of the design goes to this place."))
                .clicked()
            {
                put = Some(q.at);
            }
            if ui.button(tr("Remove")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(at) = put {
        st.put_design_at_photo(at);
    }
    if let Some(i) = remove {
        st.edit_photo("Remove place", None, |p| {
            if i < p.placements.len() {
                p.placements.remove(i);
            }
        });
    }
}

impl AppState {
    /// Moves the photo so that the place `at` (mm from its top left
    /// corner) is under the center of the design.
    pub fn put_design_at_photo(&mut self, at: Point) {
        let Some(v) = self.doc.design().product.clone() else {
            return;
        };
        let mut moved = v;
        moved.put_design_at(self.doc.design(), at);
        let origin = moved.origin;
        self.edit_product("Product place", None, |v| v.origin = origin);
    }
}

#[cfg(test)]
mod tests {
    use emb_model::Rgb;

    use super::*;
    use crate::test_util::Harness;

    /// A PNG of a white shirt (with a darker fold) on a blue background.
    fn shirt_png() -> Vec<u8> {
        let img = image::RgbaImage::from_fn(80, 60, |x, y| {
            if (16..64).contains(&x) && (10..50).contains(&y) {
                let v = if x == 40 { 170 } else { 235 };
                image::Rgba([v, v, v, 255])
            } else {
                image::Rgba([40, 80, 200, 255])
            }
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    fn picture(h: &Harness) -> ProductPicture {
        h.st.doc.design().product.as_ref().unwrap().picture.clone().unwrap()
    }

    #[test]
    fn a_click_masks_the_shirt_and_the_color_goes_on_it() {
        let mut h = Harness::new();
        h.st.set_product_picture(shirt_png(), "shirt.png", 400.0).unwrap();
        let ctx = egui::Context::default();
        h.st.pick_photo_color(&ctx, (20, 20), true);
        let m = picture(&h).mask.unwrap();
        let cells = m.cells();
        assert_eq!(cells.iter().filter(|&&c| c).count(), 48 * 40);
        // The product color goes on the shirt in the saved picture.
        h.st.edit_product("Color", None, |v| v.color = Rgb(200, 20, 30));
        let dir = std::env::temp_dir().join(format!("emb-photo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("on.png");
        h.st.save_decoration(&out).unwrap();
        let img = image::open(&out).unwrap().to_rgba8();
        let v = h.st.doc.design().product.clone().unwrap();
        let px = |q: Point| {
            let b = v.bbox();
            let at = q + v.origin;
            // The picture has 10 mm round the product.
            let k = f64::from(img.width()) / (b.width() + 20.0);
            img.get_pixel(
                ((at.x - b.min.x + 10.0) * k) as u32,
                ((at.y - b.min.y + 10.0) * k) as u32,
            )
            .0
        };
        let shirt = px(Point::new(100.0, 100.0));
        assert!(shirt[0] > 170 && shirt[1] < 50, "{shirt:?}");
        let background = px(Point::new(20.0, 20.0));
        assert!(background[2] > 150 && background[0] < 80, "{background:?}");
        // One undo step takes the mask away.
        h.st.doc.undo();
        h.st.doc.undo();
        assert!(picture(&h).mask.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_template_keeps_the_photo_its_mask_and_its_places() {
        let mut h = Harness::new();
        h.st.set_product_picture(shirt_png(), "shirt.png", 400.0).unwrap();
        let ctx = egui::Context::default();
        h.st.pick_photo_color(&ctx, (20, 20), true);
        h.st.edit_photo("Mark a place", None, |p| {
            p.placements.push(PhotoPlacement {
                name: "Left chest".into(),
                at: Point::new(250.0, 120.0),
                max: (100.0, 100.0),
            });
        });
        let dir = std::env::temp_dir().join(format!("emb-templates-{}", std::process::id()));
        let p = picture(&h);
        let path = save_template(&dir, "My: shirt", &p).unwrap();
        assert_eq!(path.file_name().unwrap(), "My_ shirt.embproduct");
        let list = list_templates(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "My_ shirt");
        let back = load_template(&list[0].path).unwrap();
        assert_eq!(back, p);
        // A new design on the template goes to its first place.
        h.st.doc.edit("None", |d| d.product = None);
        h.st.put_product_picture(back);
        let v = h.st.doc.design().product.clone().unwrap();
        let c = h.st.doc.design().view_extents().unwrap().center();
        assert!((c - v.origin).distance(Point::new(250.0, 120.0)) < 1e-6);
        assert_eq!(
            v.photo_placement_of(h.st.doc.design()).map(|q| q.name.as_str()),
            Some("Left chest")
        );
        // A file that is not a template is refused.
        let bad = dir.join("bad.embproduct");
        std::fs::write(&bad, "{\"format\": \"other\", \"product\": {}}").unwrap();
        assert!(load_template(&bad).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Writes a white T-shirt "photo" (the drawn T-shirt) with its mask,
    /// in three colors, with the design at the left chest:
    /// `EMB_PRODUCT_OUT=<folder> cargo test -p emb-app -- --ignored write_photo_colors`.
    #[test]
    #[ignore]
    fn write_photo_colors() {
        let dir = std::path::PathBuf::from(std::env::var("EMB_PRODUCT_OUT").expect("set EMB_PRODUCT_OUT"));
        std::fs::create_dir_all(&dir).unwrap();
        let shirt = emb_model::ProductView::new(emb_model::ProductKind::TShirt, Rgb(245, 245, 245));
        let (pic, b) = emb_render::product::product_picture(&shirt, 2.0);
        let mut rgba = pic.rgba.clone();
        // A gray backdrop, as on a photo.
        for px in rgba.as_chunks_mut::<4>().0 {
            if px[3] < 255 {
                let a = f64::from(px[3]) / 255.0;
                for c in &mut px[..3] {
                    *c = (f64::from(*c) * a + 120.0 * (1.0 - a)).round() as u8;
                }
                px[3] = 255;
            }
        }
        let img = image::RgbaImage::from_raw(pic.width, pic.height, rgba).unwrap();
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let mut h = Harness::new();
        h.st.set_product_picture(png, "white-shirt.png", b.width()).unwrap();
        let ctx = egui::Context::default();
        let p = picture(&h);
        let (gw, _) = emb_model::PhotoMask::grid_for(p.pixels);
        let cell = |q: Point| {
            let k = f64::from(gw) / b.width();
            (((q.x - b.min.x) * k) as u32, ((q.y - b.min.y) * k) as u32)
        };
        h.st.pick_photo_color(&ctx, cell(Point::new(-60.0, 400.0)), true);
        let left_chest = Point::new(110.0, 190.0) - b.min;
        h.st.edit_photo("Mark a place", None, |p| {
            p.placements.push(PhotoPlacement {
                name: "Left chest".into(),
                at: left_chest,
                max: (100.0, 100.0),
            });
        });
        h.st.put_design_at_photo(left_chest);
        img.save(dir.join("photo.png")).unwrap();
        for (name, c) in [
            ("navy", Rgb(30, 38, 66)),
            ("red", Rgb(200, 30, 40)),
            ("gold", Rgb(245, 180, 30)),
        ] {
            h.st.edit_product("Color", None, |v| v.color = c);
            h.st.save_decoration(&dir.join(format!("photo-{name}.png"))).unwrap();
        }
    }

    #[test]
    fn the_window_draws_with_no_change() {
        let mut h = Harness::new();
        h.st.set_product_picture(shirt_png(), "shirt.png", 400.0).unwrap();
        h.st.edit_photo("Mark a place", None, |p| {
            p.placements.push(PhotoPlacement {
                name: "Left chest".into(),
                at: Point::new(250.0, 120.0),
                max: (100.0, 100.0),
            });
        });
        h.st.photo_editor.open = true;
        h.st.product_window = true;
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                window(ui.ctx(), &mut h.st);
                crate::product::window(ui.ctx(), &mut h.st);
            });
            out.textures_delta.clear();
        }
        assert_eq!(h.st.doc.state_id(), state);
        assert!(h.st.photo_editor.overlay.is_some());
    }
}
