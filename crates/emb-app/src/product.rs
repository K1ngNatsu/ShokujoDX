//! Product backdrops and the virtual decoration picture (P7 Milestone 5):
//! the Product window, the product under the design on the canvas, and
//! Save picture on product. The product does not sew.
//!
//! Other designs on the product (P16 Milestone 2): each `ProductExtra` is
//! stitched once (by its key) and drawn on the canvas as a picture over
//! the product; the saved picture has their stitches with the design.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point};
use emb_model::product::SWATCHES;
use emb_model::{MergeKey, ProductExtra, ProductKind, ProductPicture, ProductView, Rgb, Side, Trim};
use emb_render::Segment;
use emb_stitch::{PlanOptions, StitchCache, build_plan};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// The color, the places and the template of a photo of the user's
/// product (P16 Milestone 7).
fn photo_ui(ui: &mut egui::Ui, st: &mut AppState, v: &ProductView, p: &ProductPicture) {
    if p.mask.is_some() {
        ui.horizontal(|ui| {
            ui.label(tr("Color"));
            let mut c = [v.color.0, v.color.1, v.color.2];
            if ui.color_edit_button_srgb(&mut c).changed() {
                st.edit_product("Product color", Some("color"), |p| p.color = Rgb(c[0], c[1], c[2]));
            }
        });
        if let Some(c) = color_swatches(ui, v.color) {
            st.edit_product("Product color", None, |p| p.color = c);
        }
    }
    if ui
        .button(tr("Edit the Photo…"))
        .on_hover_text(tr(
            "Mark the part of the photo that takes the product color, and the usual places for a design.",
        ))
        .clicked()
    {
        st.photo_editor.open = true;
    }
    if !p.placements.is_empty() {
        ui.label(tr("Put the design at:"));
        let on = v.photo_placement_of(st.doc.design()).map(|q| q.name.clone());
        let mut put = None;
        ui.horizontal_wrapped(|ui| {
            for q in &p.placements {
                let tip = crate::i18n::fill(
                    "Usual largest design here: {} × {} mm",
                    &[&format!("{:.0}", q.max.0), &format!("{:.0}", q.max.1)],
                );
                if ui
                    .selectable_label(on.as_deref() == Some(q.name.as_str()), &q.name)
                    .on_hover_text(tip)
                    .clicked()
                {
                    put = Some(q.at);
                }
            }
        });
        if let Some(at) = put {
            st.put_design_at_photo(at);
        }
    }
    ui.horizontal(|ui| {
        let stem = p.name.rsplit_once('.').map_or(p.name.as_str(), |(s, _)| s).to_string();
        if ui
            .button(tr("Save in My Products"))
            .on_hover_text(tr(
                "Keep this photo with its real width, its mask and its places, to use it again for other designs.",
            ))
            .clicked()
        {
            st.save_product_template(&stem);
        }
        if ui
            .button(tr("Export…"))
            .on_hover_text(tr("Save this product as a file for another computer."))
            .clicked()
        {
            st.export_product_template();
        }
    });
}

/// The user's product templates: choose one, or import one.
fn templates_ui(ui: &mut egui::Ui, st: &mut AppState) {
    let templates = st.product_templates();
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(tr("My Products"));
        ui.add_enabled_ui(!templates.is_empty(), |ui| {
            egui::ComboBox::from_id_salt("product_templates")
                .selected_text(tr("Choose…"))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for t in &templates {
                        if ui.selectable_label(false, &t.name).clicked() {
                            chosen = Some(t.clone());
                        }
                    }
                });
        });
        if ui
            .button(tr("Import…"))
            .on_hover_text(tr("Add a product file from another computer to My Products."))
            .clicked()
        {
            st.import_product_template();
        }
    });
    if let Some(t) = chosen {
        st.use_product_template(&t);
    }
}

/// The trim color of the product and its trim parts (P16 Milestone 6).
fn trim_ui(ui: &mut egui::Ui, st: &mut AppState, v: &ProductView) {
    let names = v.kind.trims();
    if names.is_empty() {
        return;
    }
    let mut two = v.trim.is_some();
    if ui
        .checkbox(&mut two, tr("Trim color"))
        .on_hover_text(tr(
            "Give the trim parts of the product a second color, for example the collar of a polo or the front of a cap.",
        ))
        .changed()
    {
        // Black trim on a light product, white trim on a dark product.
        let luma = 0.299 * f64::from(v.color.0) + 0.587 * f64::from(v.color.1) + 0.114 * f64::from(v.color.2);
        let color = if luma < 128.0 { SWATCHES[0].1 } else { SWATCHES[5].1 };
        st.edit_product("Trim color", None, |p| {
            p.trim = two.then(|| Trim {
                color,
                off: Vec::new(),
            });
        });
    }
    let Some(t) = &v.trim else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(tr("Trim parts"));
        let mut c = [t.color.0, t.color.1, t.color.2];
        if ui.color_edit_button_srgb(&mut c).changed() {
            st.edit_product("Trim color", Some("trim"), |p| {
                if let Some(t) = p.trim.as_mut() {
                    t.color = Rgb(c[0], c[1], c[2]);
                }
            });
        }
    });
    if let Some(c) = color_swatches(ui, t.color) {
        st.edit_product("Trim color", None, |p| {
            if let Some(t) = p.trim.as_mut() {
                t.color = c;
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        for name in names {
            let mut on = v.trim_on(name);
            if ui.checkbox(&mut on, tr(name)).changed() {
                st.edit_product("Trim parts", None, |p| p.set_trim_on(name, on));
            }
        }
    });
}

/// A row of small swatches of the usual garment colors (`now` has a
/// ring). Returns the color that was clicked.
fn color_swatches(ui: &mut egui::Ui, now: Rgb) -> Option<Rgb> {
    let mut pick = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
        for (name, c) in SWATCHES {
            let (rect, resp) = ui.allocate_exact_size(egui::Vec2::splat(18.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(c.0, c.1, c.2));
            let ring = if c == now {
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
            } else {
                egui::Stroke::new(1.0, ui.visuals().weak_text_color())
            };
            ui.painter().rect_stroke(rect, 3.0, ring, egui::StrokeKind::Outside);
            let name = tr(name);
            if crate::a11y::named_toggle(resp, name, c == now)
                .on_hover_text(name)
                .clicked()
            {
                pick = Some(c);
            }
        }
    });
    pick
}

/// Pixels for each mm of the product texture on the canvas.
const TEXTURE_PX_PER_MM: f64 = 2.0;
/// Pixels for each mm of the canvas picture of another design (fewer for a
/// large design, so its side is at most `EXTRA_MAX_PX`).
const EXTRA_PX_PER_MM: f64 = 8.0;
const EXTRA_MAX_PX: f64 = 4096.0;
/// Turn Product: show the other side of the garment.
pub const TURN: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT), egui::Key::T);
/// The thread width (mm) of the pictures of the designs.
pub(crate) const THREAD_MM: f64 = 0.4;

/// The stitches and the canvas pictures of the other designs on the
/// product, by the key of each design.
#[derive(Default)]
pub struct Extras {
    looks: HashMap<u64, ExtraLook>,
}

struct ExtraLook {
    /// The stitches (view mm) with the center of the design at (0, 0).
    segments: Arc<Vec<Segment>>,
    /// The canvas picture, the area that it shows (same coordinates as the
    /// segments), and the look that it has.
    texture: Option<(TextureHandle, BBox, emb_render::raster::Look)>,
}
/// Pixels for each mm of the saved picture.
pub const DECORATION_PX_PER_MM: f64 = 4.0;
/// The default product color: white.
const WHITE: Rgb = Rgb(250, 250, 250);

/// The texture of the product, and what it shows. The shading of the
/// drawn product stays while only the color changes (P16 Milestone 1).
pub struct ProductTexture {
    key: TextureKey,
    texture: TextureHandle,
    shading: Option<Arc<emb_render::product::Shading>>,
    /// The decoded photo (by the address of its data), so a color change
    /// on a masked photo does not decode it again.
    photo: Option<(usize, Arc<image::RgbaImage>)>,
}

/// What the texture of the product shows: the kind, the colors, the photo
/// and its mask.
type TextureKey = (ProductKind, Rgb, Option<Trim>, usize, usize, usize);

fn key(v: &ProductView) -> TextureKey {
    let (ptr, len, mask) = v.picture.as_ref().map_or((0, 0, 0), |p| {
        (
            Arc::as_ptr(&p.data) as usize,
            p.data.len(),
            p.mask.as_ref().map_or(0, |m| Arc::as_ptr(&m.runs) as usize),
        )
    });
    (v.kind, v.color, v.trim.clone(), ptr, len, mask)
}

/// The longest side (pixels) of the photo of the product on the canvas.
const PHOTO_MAX_SIDE: u32 = 2048;

impl AppState {
    /// Changes the product as one undo step. With `merge`, a drag is one step.
    pub(crate) fn edit_product(&mut self, label: &str, merge: Option<&str>, f: impl FnOnce(&mut ProductView)) {
        let change = |d: &mut emb_model::Design| {
            if let Some(v) = d.product.as_mut() {
                f(v);
            }
        };
        match merge {
            Some(k) => self
                .doc
                .edit_merged(label, MergeKey::new(format!("product-{k}")), change),
            None => self.doc.edit(label, change),
        }
    }

    /// Puts a drawn product under the design, with the design at the first
    /// usual place of the product. The other designs stay (each at the
    /// place of the same name, if the product has it).
    pub fn set_product(&mut self, kind: ProductKind) {
        let old = self.doc.design().product.clone();
        let color = old.as_ref().map_or(WHITE, |v| v.color);
        let mut v = ProductView::new(kind, color);
        if let Some(old) = old {
            let mut moved = old;
            moved.change_kind(kind);
            v.extras = moved.extras;
            v.trim = moved.trim;
        }
        if let Some(p) = kind.placements().first() {
            v.put_design_at(self.doc.design(), p.at);
        }
        self.doc.edit(tr("Product"), |d| d.product = Some(v));
        self.fit_product = true;
    }

    /// Shows the other side of the garment (Turn Product). The open design
    /// goes to the same place on that side.
    pub fn turn_product(&mut self) {
        let Some(v) = self.doc.design().product.clone() else {
            return;
        };
        let Some(other) = v.kind.turned().filter(|_| v.picture.is_none()) else {
            self.message = tr("Only a garment has a front and a back.").into();
            return;
        };
        let design = self.doc.design().clone();
        self.edit_product("Turn product", None, |p| p.turn(&design));
        self.message = crate::i18n::fill("The product shows: {}.", &[&tr(other.name())]);
    }

    /// Moves the product so the design is at the usual place `i`.
    pub fn put_design_at_placement(&mut self, i: usize) {
        let Some(v) = self.doc.design().product.clone() else {
            return;
        };
        let Some(p) = v.kind.placements().get(i) else { return };
        let mut moved = v.clone();
        moved.put_design_at(self.doc.design(), p.at);
        let origin = moved.origin;
        self.edit_product("Product place", None, |v| v.origin = origin);
        self.message = crate::i18n::fill(
            "The design is at {} of the {}.",
            &[&(p.name.to_lowercase()), &tr(v.kind.name())],
        );
    }

    /// Puts a picture of the user's own product under the design, centered
    /// on the design, with its real width.
    pub fn set_product_picture(&mut self, bytes: Vec<u8>, name: &str, width_mm: f64) -> Result<(), String> {
        // A full decode (made small: less memory) checks the data.
        emb_art::decode_fit(&bytes, 64, crate::resources::decode_budget())
            .map_err(|e| format!("{name}: {}", crate::resources::decode_error(&e)))?;
        let pixels = emb_art::picture_size(&bytes).map_err(|e| format!("{name}: {e}"))?;
        let picture = ProductPicture {
            name: name.to_string(),
            data: Arc::new(bytes),
            pixels,
            width_mm,
            mask: None,
            placements: Vec::new(),
        };
        self.put_product_picture(picture);
        Ok(())
    }

    /// Puts the picture of the user's product `picture` under the design:
    /// the design at the first place marked on it, else at its middle. The
    /// product color and the other designs stay.
    pub fn put_product_picture(&mut self, picture: ProductPicture) {
        let size = Point::new(picture.width_mm, picture.height_mm());
        let c = self
            .doc
            .design()
            .view_extents()
            .map_or(self.doc.design().hoop.center, |b| b.center());
        let kind = self
            .doc
            .design()
            .product
            .as_ref()
            .map_or(ProductKind::TShirt, |v| v.kind);
        let color = self.doc.design().product.as_ref().map_or(WHITE, |v| v.color);
        let mut v = ProductView::new(kind, color);
        v.origin = picture.placements.first().map_or(c - size * 0.5, |p| c - p.at);
        v.picture = Some(picture);
        if let Some(old) = &self.doc.design().product {
            v.extras = old.extras.clone();
        }
        self.doc.edit(tr("Product picture"), |d| d.product = Some(v));
        self.fit_product = true;
    }

    pub fn product_picture_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title(tr("Picture of your product"))
            .add_filter(tr("Pictures (PNG, JPEG, BMP, TIFF, GIF)"), &crate::backdrop::FILE_TYPES)
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        let name = p
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let r = std::fs::read(&p)
            .map_err(|e| format!("{}: {e}", p.display()))
            .and_then(|b| self.set_product_picture(b, &name, 500.0));
        match r {
            Ok(()) => self.message = tr("Set the real width of the product in the Product window.").into(),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot use the picture".into(),
                    lines: vec![e],
                })
            }
        }
    }

    /// The picture of the design on the product (the virtual decoration).
    pub fn decoration_picture(&mut self) -> Option<emb_render::raster::Picture> {
        self.decoration_picture_at(DECORATION_PX_PER_MM)
    }

    /// As [`AppState::decoration_picture`], at `px_per_mm`.
    pub fn decoration_picture_at(&mut self, px_per_mm: f64) -> Option<emb_render::raster::Picture> {
        let v = self.doc.design().product.clone()?;
        let design: Vec<Segment> = self.plan_view().segments.iter().map(|(s, _)| *s).collect();
        let look = picture_look(&self.settings.look);
        Some(compose(&v, &design, &mut |e| self.extra_segments(e), &look, px_per_mm))
    }

    /// Writes the picture of the design on the product as PNG.
    pub fn save_decoration(&mut self, path: &Path) -> Result<(), String> {
        let pic = self
            .decoration_picture()
            .ok_or(tr("Choose a product first (Design > Product)."))?;
        std::fs::write(path, pic.png()).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save_decoration_dialog(&mut self) {
        if self.doc.design().product.is_none() {
            self.message = tr("Choose a product first (Design > Product).").into();
            return;
        }
        let Some(p) = rfd::FileDialog::new()
            .set_title(tr("Save picture on product (PNG)"))
            .add_filter("PNG", &["png"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{} on product.png", self.doc.design().name))
            .save_file()
        else {
            return;
        };
        match self.save_decoration(&p) {
            Ok(()) => self.message = crate::i18n::fill("Saved {}", &[&(p.display())]),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot save the picture".into(),
                    lines: vec![e],
                })
            }
        }
    }

    /// Shows the design of the file `path` (a project or a machine file) on
    /// the product, at a free place.
    pub fn add_product_design(&mut self, path: &Path) -> Result<(), String> {
        let Some(v) = self.doc.design().product.clone() else {
            return Err(tr("Choose a product first (Design > Product).").into());
        };
        let design = match crate::file::machine_ext(path) {
            Some(ext) => {
                let s = emb_library::read_machine_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
                if s.stitch_count() == 0 {
                    return Err(format!("{}: {}", path.display(), tr("The file has no stitches.")));
                }
                let stem = path
                    .file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                let mut d = emb_model::design_from_stitches(&s, &stem);
                let needles = std::fs::read(path)
                    .map(|b| emb_formats::read_needles(&ext, &b))
                    .unwrap_or_default();
                d.pin_block_needles(&needles);
                d
            }
            None => emb_model::project::load(path).map_err(|e| format!("{}: {e}", path.display()))?,
        };
        if design.view_extents().is_none() {
            return Err(format!("{}: {}", path.display(), tr("The design has no stitches.")));
        }
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let at = v.free_place(self.doc.design());
        let mut extra = ProductExtra::new(&name, design, at);
        extra.side = v.side();
        let place = v.placement_at(at).map(|p| p.name);
        self.edit_product("Add product design", None, |p| p.extras.push(extra));
        self.message = match place {
            Some(p) => crate::i18n::fill(
                "{} is at {} of the {}.",
                &[&name, &tr(p).to_lowercase(), &tr(v.kind.name())],
            ),
            None => crate::i18n::fill("{} is on the product.", &[&name]),
        };
        Ok(())
    }

    pub fn add_product_design_dialog(&mut self) {
        let mut exts: Vec<&str> = vec![emb_model::project::FILE_EXTENSION];
        exts.extend(emb_formats::READ_EXTENSIONS);
        let Some(p) = rfd::FileDialog::new()
            .set_title(tr("Design to show on the product"))
            .add_filter(tr("Designs (projects and machine files)"), &exts)
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        if let Err(e) = self.add_product_design(&p) {
            self.notice = Some(Notice {
                title: "Cannot use the design".into(),
                lines: vec![e],
            });
        }
    }

    #[cfg(test)]
    pub fn edit_product_for_test(&mut self, f: impl FnOnce(&mut ProductView)) {
        self.edit_product("Test", None, f);
    }

    /// Changes the other design `i` of the product as one undo step.
    fn edit_extra(&mut self, label: &str, merge: Option<&str>, i: usize, f: impl FnOnce(&mut ProductExtra)) {
        self.edit_product(label, merge, |v| {
            if let Some(e) = v.extras.get_mut(i) {
                f(e);
            }
        });
    }

    /// The stitches of the other design `e` (view mm, its center at (0, 0)),
    /// made once for each design.
    fn extra_segments(&mut self, e: &ProductExtra) -> Arc<Vec<Segment>> {
        if let Some(l) = self.product_extras.looks.get(&e.key) {
            return l.segments.clone();
        }
        let segments = Arc::new(stitch_extra(e, &self.machines));
        self.product_extras.looks.insert(
            e.key,
            ExtraLook {
                segments: segments.clone(),
                texture: None,
            },
        );
        segments
    }

    /// The canvas picture of the other design `e` and the area that it
    /// shows (view mm, its center at (0, 0)).
    fn extra_texture(&mut self, ctx: &egui::Context, e: &ProductExtra) -> Option<(TextureHandle, BBox)> {
        let look = picture_look(&self.settings.look);
        let segments = self.extra_segments(e);
        let l = self.product_extras.looks.get_mut(&e.key)?;
        if let Some((t, b, made)) = &l.texture
            && *made == look
        {
            return Some((t.clone(), *b));
        }
        let b = BBox::from_points(segments.iter().flat_map(|s| [s.a, s.b]))?.expand(1.0);
        let px = EXTRA_PX_PER_MM.min(EXTRA_MAX_PX / b.width().max(b.height()).max(1.0));
        let pic = emb_render::raster::picture_at_look(&segments, &[], b, px, THREAD_MM, None, &look);
        let image = egui::ColorImage::from_rgba_unmultiplied([pic.width as usize, pic.height as usize], &pic.rgba);
        let t = crate::resources::load_texture(ctx, format!("product-extra-{}", e.key), image, TextureOptions::LINEAR);
        l.texture = Some((t.clone(), b, look));
        Some((t, b))
    }

    fn product_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let v = self.doc.design().product.clone()?;
        let k = key(&v);
        // A change of the light (Options) makes the shading again.
        if let Some(t) = &self.product_tex
            && t.key == k
            && t.shading
                .as_ref()
                .is_none_or(|s| s.fits(v.kind, TEXTURE_PX_PER_MM, &self.settings.look.light))
        {
            return Some(t.texture.clone());
        }
        let light = self.settings.look.light;
        let mut shading = self
            .product_tex
            .as_ref()
            .and_then(|t| t.shading.clone())
            .filter(|s| s.fits(v.kind, TEXTURE_PX_PER_MM, &light));
        let mut photo = None;
        let image = match &v.picture {
            Some(p) => {
                let at = Arc::as_ptr(&p.data) as usize;
                let img = match self
                    .product_tex
                    .as_ref()
                    .and_then(|t| t.photo.clone())
                    .filter(|(a, _)| *a == at)
                {
                    Some((_, img)) => img,
                    None => {
                        let side = PHOTO_MAX_SIDE
                            .min(u32::try_from(crate::resources::max_texture_side(ctx)).unwrap_or(u32::MAX));
                        Arc::new(emb_art::decode_fit(&p.data, side, crate::resources::decode_budget()).ok()?)
                    }
                };
                photo = Some((at, img.clone()));
                let (w, h) = img.dimensions();
                let mut rgba = img.as_raw().clone();
                if let Some(m) = &p.mask {
                    emb_render::photo::recolor(&mut rgba, w, h, m, v.color);
                }
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
            }
            None => {
                let s = shading
                    .get_or_insert_with(|| Arc::new(emb_render::product::Shading::new(&v, TEXTURE_PX_PER_MM, &light)));
                let (pic, _) = emb_render::product::product_picture_with(&v, s);
                egui::ColorImage::from_rgba_unmultiplied([pic.width as usize, pic.height as usize], &pic.rgba)
            }
        };
        let texture = crate::resources::load_texture(ctx, "product", image, TextureOptions::LINEAR);
        self.product_tex = Some(ProductTexture {
            key: k,
            texture: texture.clone(),
            shading,
            photo,
        });
        Some(texture)
    }
}

/// Draws the product under the design (view coordinates; the product does
/// not turn with the design rotation).
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &mut AppState) {
    let Some(v) = st.doc.design().product.clone() else {
        return;
    };
    if !v.visible {
        return;
    }
    if st.fit_product {
        st.fit_product = false;
        let b = st.fit_box().map_or(v.bbox(), |f| f.union(v.bbox()));
        st.view.fit(rect, b);
    }
    let Some(tex) = st.product_texture(ctx) else { return };
    // The drawn product texture has 2 mm round the product.
    let b = if v.picture.is_some() {
        v.bbox()
    } else {
        v.bbox().expand(2.0)
    };
    picture(painter, rect, st, &tex, b);
    // The other designs, over the product and under the open design.
    let keys: Vec<u64> = v.extras.iter().map(|e| e.key).collect();
    st.product_extras.looks.retain(|k, _| keys.contains(k));
    for e in v.extras.iter().filter(|e| e.visible && e.side == v.side()) {
        if let Some((t, b)) = st.extra_texture(ctx, e) {
            let to = v.origin + e.at;
            picture(painter, rect, st, &t, BBox::new(b.min + to, b.max + to));
        }
    }
}

/// The look of the pictures of the product: the 3D look of the settings,
/// with no fabric texture (the product is under the design).
pub(crate) fn picture_look(settings: &crate::viewtools::LookSettings) -> emb_render::raster::Look {
    emb_render::raster::Look {
        fabric: None,
        ..settings.picture_look(None)
    }
}

/// The stitches of the other design `e` (view mm, its center at (0, 0)).
pub(crate) fn stitch_extra(e: &ProductExtra, machines: &emb_model::MachineCatalog) -> Vec<Segment> {
    let opts = e
        .design
        .machine
        .as_deref()
        .and_then(|id| machines.get(id))
        .map_or_else(PlanOptions::default, PlanOptions::for_machine);
    let plan = build_plan(&e.design, &opts, &mut StitchCache::default());
    let back = e.design.from_hoop_transform();
    let c = e.center();
    emb_render::segments(&plan.stitches)
        .into_iter()
        .map(|s| Segment {
            a: back.apply(s.a) - c,
            b: back.apply(s.b) - c,
            ..s
        })
        .collect()
}

/// The picture of the design (`design`: its stitches in view mm) on the
/// product `v` with its other designs (`extra` gives their stitches): the
/// side that shows, or both sides next to each other.
fn compose(
    v: &ProductView,
    design: &[Segment],
    extra: &mut dyn FnMut(&ProductExtra) -> Arc<Vec<Segment>>,
    look: &emb_render::raster::Look,
    px_per_mm: f64,
) -> emb_render::raster::Picture {
    let mut sides: Vec<emb_render::raster::Picture> = compose_sides(v, design, extra, look, px_per_mm)
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    if sides.len() == 2 {
        let gap = (40.0 * px_per_mm) as u32;
        return emb_render::product::side_by_side(&sides[0], &sides[1], gap);
    }
    sides.swap_remove(0)
}

/// As [`compose`], one picture for each side: the side that shows, or the
/// front and the back.
fn compose_sides(
    v: &ProductView,
    design: &[Segment],
    extra: &mut dyn FnMut(&ProductExtra) -> Arc<Vec<Segment>>,
    look: &emb_render::raster::Look,
    px_per_mm: f64,
) -> Vec<(Side, emb_render::raster::Picture)> {
    let photo = v.picture.as_ref().and_then(|p| {
        emb_art::decode_fit(
            &p.data,
            crate::backdrop::MAX_TEXTURE_SIDE,
            crate::resources::decode_budget(),
        )
        .ok()
    });
    // One side of the product: the other designs of that side, then the
    // open design if it is on that side.
    let mut picture_of = |side: Side| {
        let mut segments = Vec::new();
        for e in v.extras.iter().filter(|e| e.visible && e.side == side) {
            let to = v.origin + e.at;
            segments.extend(extra(e).iter().map(|s| Segment {
                a: s.a + to,
                b: s.b + to,
                ..*s
            }));
        }
        if side == v.side() {
            segments.extend_from_slice(design);
        }
        let kind = v.kind.on(side);
        // A cap curves the designs round its crown (not a photo).
        if v.picture.is_none() && kind == ProductKind::Cap {
            let warp = |q: Point| v.origin + kind.warp(q - v.origin);
            for s in &mut segments {
                (s.a, s.b) = (warp(s.a), warp(s.b));
            }
        }
        let view = ProductView { kind, ..v.clone() };
        emb_render::product::decoration(&view, photo.as_ref(), &segments, px_per_mm, THREAD_MM, look)
    };
    if v.both_sides && v.picture.is_none() && v.kind.turned().is_some() {
        return vec![
            (Side::Front, picture_of(Side::Front)),
            (Side::Back, picture_of(Side::Back)),
        ];
    }
    vec![(v.side(), picture_of(v.side()))]
}

/// The pictures of `design` on its product (one for each side that the
/// picture shows), with no app state (the approval sheet): `plan` is the
/// stitch plan of the design.
pub fn pictures_of_design(
    design: &emb_model::Design,
    plan: &emb_stitch::StitchPlan,
    machines: &emb_model::MachineCatalog,
    look: &crate::viewtools::LookSettings,
    px_per_mm: f64,
) -> Vec<(Side, emb_render::raster::Picture)> {
    let Some(v) = design.product.as_ref() else {
        return Vec::new();
    };
    let back = design.from_hoop_transform();
    let segments: Vec<Segment> = emb_render::segments(&plan.stitches)
        .into_iter()
        .map(|s| Segment {
            a: back.apply(s.a),
            b: back.apply(s.b),
            ..s
        })
        .collect();
    let mut extra = |e: &ProductExtra| Arc::new(stitch_extra(e, machines));
    compose_sides(v, &segments, &mut extra, &picture_look(look), px_per_mm)
}

/// Draws the texture `tex` on the area `b` (view mm) of the canvas.
fn picture(painter: &egui::Painter, rect: Rect, st: &AppState, tex: &TextureHandle, b: BBox) {
    let to = |p: Point| st.view.to_screen(rect, p);
    let corners = [b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)];
    let uvs = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    let mut mesh = Mesh::with_texture(tex.id());
    for (c, uv) in corners.iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: to(*c),
            uv,
            color: Color32::WHITE,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// The Product window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.product_window {
        return;
    }
    let mut open = true;
    crate::dock::docked(
        egui::Window::new(tr("Product")).id(egui::Id::new("Product")),
        ctx,
        egui::Id::new("Product"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| contents(ui, st));
    st.product_window = open;
}

fn contents(ui: &mut egui::Ui, st: &mut AppState) {
    ui.label(tr(
        "See the design on a product, at its real size. The product does not sew.",
    ));
    let current = st.doc.design().product.clone();
    let name = match &current {
        Some(v) if v.picture.is_some() => tr("My picture").to_string(),
        Some(v) => tr(v.kind.product_name()).to_string(),
        None => tr("None").to_string(),
    };
    let mut pick: Option<Option<ProductKind>> = None;
    ui.horizontal(|ui| {
        ui.label(tr("Product"));
        egui::ComboBox::from_id_salt("product_kind")
            .selected_text(name)
            .width(170.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current.is_none(), tr("None")).clicked() {
                    pick = Some(None);
                }
                for k in ProductKind::PRODUCTS {
                    let on = current
                        .as_ref()
                        .is_some_and(|v| v.picture.is_none() && v.kind.front() == k);
                    if ui.selectable_label(on, tr(k.product_name())).clicked() {
                        pick = Some(Some(k));
                    }
                }
            });
        if ui
            .button(tr("My picture…"))
            .on_hover_text(tr("A photo of your own product, at its real width."))
            .clicked()
        {
            st.product_picture_dialog();
        }
    });
    templates_ui(ui, st);
    match pick {
        Some(Some(k)) => st.set_product(k),
        Some(None) => {
            st.doc.edit(tr("No product"), |d| d.product = None);
        }
        None => {}
    }
    let Some(v) = st.doc.design().product.clone() else {
        return;
    };
    // The front and the back of a garment.
    if v.picture.is_none() && v.kind.turned().is_some() {
        let tip = crate::i18n::fill(
            "Turn the product ({}). The design goes to the same place on the other side.",
            &[&ui.ctx().format_shortcut(&TURN)],
        );
        ui.horizontal(|ui| {
            ui.label(tr("Side"));
            for (side, text) in [(Side::Front, tr("Front side")), (Side::Back, tr("Back side"))] {
                if ui
                    .selectable_label(v.side() == side, text)
                    .on_hover_text(&tip)
                    .clicked()
                    && v.side() != side
                {
                    st.turn_product();
                }
            }
        });
    }
    let mut visible = v.visible;
    if ui.checkbox(&mut visible, tr("Show the product")).changed() {
        st.edit_product("Show Product", None, |p| p.visible = visible);
    }
    if v.picture.is_none() {
        ui.horizontal(|ui| {
            ui.label(tr("Color"));
            let mut c = [v.color.0, v.color.1, v.color.2];
            if ui.color_edit_button_srgb(&mut c).changed() {
                st.edit_product("Product color", Some("color"), |p| p.color = Rgb(c[0], c[1], c[2]));
            }
        });
        if let Some(c) = color_swatches(ui, v.color) {
            st.edit_product("Product color", None, |p| p.color = c);
        }
        trim_ui(ui, st, &v);
        ui.add_space(4.0);
        ui.label(tr("Put the design at:"));
        let design = st.doc.design().clone();
        let on = v.placement_of(&design).map(|p| p.name);
        let size = design.view_extents().map(|b| (b.width(), b.height()));
        let mut put = None;
        ui.horizontal_wrapped(|ui| {
            for (i, p) in v.kind.placements().iter().enumerate() {
                let tip = crate::i18n::fill(
                    "Usual largest design here: {} × {} mm",
                    &[&format!("{:.0}", p.max.0), &format!("{:.0}", p.max.1)],
                );
                if ui
                    .selectable_label(on == Some(p.name), tr(p.name))
                    .on_hover_text(tip)
                    .clicked()
                {
                    put = Some(i);
                }
            }
        });
        if let Some(i) = put {
            st.put_design_at_placement(i);
        }
        if let (Some(p), Some((w, h))) = (v.placement_of(&design), size)
            && (w > p.max.0 + 0.5 || h > p.max.1 + 0.5)
        {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                crate::i18n::fill(
                    "The design ({} × {} mm) is larger than the usual size for {} ({} × {} mm).",
                    &[
                        &format!("{w:.0}"),
                        &format!("{h:.0}"),
                        &tr(p.name).to_lowercase(),
                        &format!("{:.0}", p.max.0),
                        &format!("{:.0}", p.max.1),
                    ],
                ),
            );
        }
    } else if let Some(p) = &v.picture {
        ui.horizontal(|ui| {
            ui.label(tr("Real width"));
            let mut w = p.width_mm;
            if crate::workspace::length_value(ui, &mut w, st.settings.units, 10.0, 3000.0).changed() {
                st.edit_product("Product width", Some("width"), |v| {
                    if let Some(p) = v.picture.as_mut() {
                        p.width_mm = w;
                    }
                });
            }
        });
        ui.weak(&p.name);
        photo_ui(ui, st, &v, p);
    }
    // Move the product under the design.
    ui.horizontal(|ui| {
        ui.label(tr("Move the product"));
        let units = st.settings.units;
        let (mut x, mut y) = (v.origin.x, v.origin.y);
        let rx = crate::workspace::length_value(ui, &mut x, units, -5000.0, 5000.0);
        let ry = crate::workspace::length_value(ui, &mut y, units, -5000.0, 5000.0);
        if rx.changed() || ry.changed() {
            st.edit_product("Move product", Some("origin"), |p| p.origin = Point::new(x, y));
        }
    });
    extras(ui, st, &v);
    ui.add_space(4.0);
    if v.picture.is_none() && v.kind.turned().is_some() {
        let mut both = v.both_sides;
        if ui
            .checkbox(&mut both, tr("Both sides in the picture"))
            .on_hover_text(tr("The saved picture shows the front and the back next to each other."))
            .changed()
        {
            st.edit_product("Both sides", None, |p| p.both_sides = both);
        }
    }
    ui.horizontal(|ui| {
        if ui.button(tr("Zoom to product")).clicked() {
            st.fit_product = true;
        }
        if ui.button(tr("Save picture on product…")).clicked() {
            st.save_decoration_dialog();
        }
    });
}

/// What a row of the list of other designs changes.
enum ExtraChange {
    Show(usize, bool),
    Side(usize, Side),
    Place(usize, Point),
    Move(usize, Point),
    Remove(usize),
}

/// The list of the other designs on the product.
fn extras(ui: &mut egui::Ui, st: &mut AppState, v: &ProductView) {
    ui.separator();
    ui.label(tr("Other designs on the product:"));
    let units = st.settings.units;
    let mut change = None;
    for (i, e) in v.extras.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut show = e.visible;
            if ui
                .checkbox(&mut show, &e.name)
                .on_hover_text(tr("Show this design"))
                .changed()
            {
                change = Some(ExtraChange::Show(i, show));
            }
            if ui.button(tr("Remove")).clicked() {
                change = Some(ExtraChange::Remove(i));
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let here = v.placement_of_extra(e);
            if v.picture.is_none() && v.kind.turned().is_some() {
                let name = |s: Side| match s {
                    Side::Front => tr("Front side"),
                    Side::Back => tr("Back side"),
                };
                egui::ComboBox::from_id_salt(("product_extra_side", i))
                    .selected_text(name(e.side))
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        for s in [Side::Front, Side::Back] {
                            if ui.selectable_label(e.side == s, name(s)).clicked() && e.side != s {
                                change = Some(ExtraChange::Side(i, s));
                            }
                        }
                    });
            }
            if v.picture.is_none() {
                let text = here.map_or_else(|| tr("Free place").to_string(), |p| tr(p.name).to_string());
                egui::ComboBox::from_id_salt(("product_extra_place", i))
                    .selected_text(text)
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for p in v.kind.on(e.side).placements() {
                            if ui
                                .selectable_label(here.is_some_and(|h| h.name == p.name), tr(p.name))
                                .clicked()
                            {
                                change = Some(ExtraChange::Place(i, p.at));
                            }
                        }
                    });
            }
            let (mut x, mut y) = (e.at.x, e.at.y);
            let rx = crate::workspace::length_value(ui, &mut x, units, -5000.0, 5000.0);
            let ry = crate::workspace::length_value(ui, &mut y, units, -5000.0, 5000.0);
            if rx.changed() || ry.changed() {
                change = Some(ExtraChange::Move(i, Point::new(x, y)));
            }
        });
        if let (Some(p), Some((w, h))) = (v.placement_of_extra(e), e.size())
            && (w > p.max.0 + 0.5 || h > p.max.1 + 0.5)
        {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                crate::i18n::fill(
                    "{} ({} × {} mm) is larger than the usual size for {} ({} × {} mm).",
                    &[
                        &e.name,
                        &format!("{w:.0}"),
                        &format!("{h:.0}"),
                        &tr(p.name).to_lowercase(),
                        &format!("{:.0}", p.max.0),
                        &format!("{:.0}", p.max.1),
                    ],
                ),
            );
        }
    }
    if ui
        .button(tr("Add a design…"))
        .on_hover_text(tr(
            "Show one more design on the product, for example a name on a sleeve. It does not sew and does not change this design.",
        ))
        .clicked()
    {
        st.add_product_design_dialog();
    }
    match change {
        Some(ExtraChange::Show(i, on)) => st.edit_extra("Show product design", None, i, |e| e.visible = on),
        Some(ExtraChange::Side(i, side)) => {
            let design = st.doc.design().clone();
            st.edit_product("Product design side", None, |p| p.set_extra_side(i, side, &design));
        }
        Some(ExtraChange::Place(i, at)) | Some(ExtraChange::Move(i, at)) => {
            st.edit_extra("Move product design", Some("extra"), i, |e| e.at = at);
        }
        Some(ExtraChange::Remove(i)) => st.edit_product("Remove product design", None, |p| {
            if i < p.extras.len() {
                p.extras.remove(i);
            }
        }),
        None => {}
    }
}

/// The box that shows the product, for Zoom.
pub fn product_box(st: &AppState) -> Option<BBox> {
    st.doc
        .design()
        .product
        .as_ref()
        .filter(|v| v.visible)
        .map(ProductView::bbox)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_product_puts_the_design_at_its_first_place_and_undoes() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::Polo);
        let d = h.st.doc.design();
        let v = d.product.clone().unwrap();
        assert_eq!(v.placement_of(d).map(|p| p.name), Some("Left chest"));
        h.st.put_design_at_placement(1);
        let d = h.st.doc.design();
        assert_eq!(
            d.product.as_ref().unwrap().placement_of(d).map(|p| p.name),
            Some("Right chest")
        );
        h.st.doc.undo();
        h.st.doc.undo();
        assert!(h.st.doc.design().product.is_none());
        // The product does not change the stitches.
        h.st.set_product(ProductKind::Towel);
        let with = h.st.plan_view().plan.stitches.clone();
        h.st.doc.edit("None", |d| d.product = None);
        assert_eq!(h.st.plan_view().plan.stitches.records, with.records);
    }

    /// A project file with a small square of `color`, in a new folder.
    pub(crate) fn small_project(name: &str, color: Rgb) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("emb-product-extra-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = emb_model::Design::new(name);
        let t = d.add_thread("Thread", color);
        d.add_object(
            t,
            emb_model::ObjectKind::Fill {
                outer: emb_geom::Path::rect(BBox::from_center_size(Point::new(40.0, -20.0), 30.0, 20.0)),
                holes: Vec::new(),
                params: Default::default(),
            },
        );
        let p = dir.join(format!("{name}.embproj"));
        emb_model::project::save(&d, &p).unwrap();
        p
    }

    #[test]
    fn other_designs_show_on_the_product_and_in_its_picture() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::Polo);
        let path = small_project("sleeve", Rgb(0, 0, 220));
        let objects = h.st.doc.design().objects().len();
        h.st.add_product_design(&path).unwrap();
        let v = h.st.doc.design().product.clone().unwrap();
        // The open design is at the left chest: the next place is free.
        assert_eq!(v.extras.len(), 1);
        assert_eq!(v.placement_at(v.extras[0].at).map(|p| p.name), Some("Right chest"));
        assert_eq!(v.extras[0].size(), Some((30.0, 20.0)));
        // Its stitches are centered on the place, in the picture on the product.
        let segs = h.st.extra_segments(&v.extras[0]);
        let b = BBox::from_points(segs.iter().flat_map(|s| [s.a, s.b])).unwrap();
        assert!(b.center().distance(Point::ZERO) < 1.0, "{b:?}");
        let with = h.st.decoration_picture().unwrap();
        h.st.edit_extra("Show product design", None, 0, |e| e.visible = false);
        let without = h.st.decoration_picture().unwrap();
        let blue = |p: &emb_render::raster::Picture| {
            p.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|c| c[2] > 150 && c[0] < 80 && c[1] < 80)
                .count()
        };
        assert!(
            blue(&with) > 1000 && blue(&without) == 0,
            "{} {}",
            blue(&with),
            blue(&without)
        );
        // The open design does not change.
        assert_eq!(h.st.doc.design().objects().len(), objects);
        // Another kind keeps the design; undo takes it off.
        h.st.edit_extra("Move product design", None, 0, |e| {
            e.at = ProductKind::Polo.placements()[2].at
        });
        h.st.set_product(ProductKind::TShirt);
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!(v.placement_at(v.extras[0].at).map(|p| p.name), Some("Sleeve"));
        for _ in 0..4 {
            h.st.doc.undo();
        }
        assert!(h.st.doc.design().product.as_ref().unwrap().extras.is_empty());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn a_garment_turns_and_its_picture_shows_both_sides() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::TShirt);
        // A back print, added while the back shows.
        assert!(h.st.can(crate::ribbon::Cmd::TurnProduct, &crate::ribbon::SelInfo::of(&h.st)));
        h.st.turn_product();
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!(v.kind, ProductKind::TShirtBack);
        assert_eq!(
            v.placement_of(h.st.doc.design()).map(|p| p.name),
            Some("Below the collar")
        );
        let path = small_project("back", Rgb(0, 0, 220));
        h.st.add_product_design(&path).unwrap();
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!(v.extras[0].side, Side::Back);
        assert_eq!(v.placement_of_extra(&v.extras[0]).map(|p| p.name), Some("Upper back"));
        // Turn again: the open design is at the left chest, the print is on the back.
        h.st.turn_product();
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!(v.kind, ProductKind::TShirt);
        assert_eq!(v.placement_of(h.st.doc.design()).map(|p| p.name), Some("Left chest"));
        let blue = |p: &emb_render::raster::Picture| {
            p.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|c| c[2] > 150 && c[0] < 80 && c[1] < 80)
                .count()
        };
        let front = h.st.decoration_picture().unwrap();
        assert_eq!(blue(&front), 0, "the back print is not on the front");
        h.st.edit_product("Both sides", None, |p| p.both_sides = true);
        let both = h.st.decoration_picture().unwrap();
        assert!(blue(&both) > 1000);
        assert!(both.width > 2 * front.width && both.height == front.height);
        // A cap has one side.
        h.st.set_product(ProductKind::Cap);
        assert!(
            !h.st
                .can(crate::ribbon::Cmd::TurnProduct, &crate::ribbon::SelInfo::of(&h.st))
        );
        let before = h.st.doc.state_id();
        h.st.turn_product();
        assert_eq!(h.st.doc.state_id(), before);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn a_file_with_no_design_is_not_added() {
        let mut h = Harness::new();
        let path = small_project("none", Rgb(0, 0, 0));
        assert!(h.st.add_product_design(&path).is_err(), "no product yet");
        h.st.set_product(ProductKind::Tote);
        let empty = path.with_file_name("empty.embproj");
        emb_model::project::save(&emb_model::Design::new("empty"), &empty).unwrap();
        assert!(h.st.add_product_design(&empty).is_err());
        assert!(h.st.add_product_design(&path.with_file_name("missing.dst")).is_err());
        assert!(h.st.doc.design().product.as_ref().unwrap().extras.is_empty());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn the_picture_on_the_product_is_saved_as_png() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::TShirt);
        let dir = std::env::temp_dir().join(format!("emb-product-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("on product.png");
        h.st.save_decoration(&p).unwrap();
        let img = image::open(&p).unwrap();
        // The T-shirt at 4 px/mm with 10 mm round it.
        let b = ProductKind::TShirt.bbox();
        assert!(f64::from(img.width()) >= (b.width() + 20.0) * DECORATION_PX_PER_MM - 1.0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_picture_of_my_product_has_its_real_width() {
        let mut h = Harness::new();
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(40, 20, image::Rgba([0, 90, 0, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        h.st.set_product_picture(png, "bag.png", 400.0).unwrap();
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!((v.bbox().width(), v.bbox().height()), (400.0, 200.0));
        assert!(
            h.st.set_product_picture(b"not a picture".to_vec(), "x.png", 400.0)
                .is_err()
        );
    }

    #[test]
    fn the_window_and_the_canvas_draw_with_no_change() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::Cap);
        let path = small_project("window", Rgb(0, 120, 0));
        h.st.add_product_design(&path).unwrap();
        h.st.product_window = true;
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
            out.textures_delta.clear();
            h.frame(vec![]);
        }
        assert_eq!(h.st.doc.state_id(), state);
        assert!(h.st.product_tex.is_some());
        // The other design has its canvas picture.
        assert!(h.st.product_extras.looks.values().all(|l| l.texture.is_some()));
        assert_eq!(h.st.product_extras.looks.len(), 1);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    /// Writes a picture of the sample design on each product:
    /// `EMB_PRODUCT_OUT=<folder> cargo test -p emb-app -- --ignored write_products`.
    #[test]
    #[ignore]
    fn write_products() {
        let dir = std::path::PathBuf::from(std::env::var("EMB_PRODUCT_OUT").expect("set EMB_PRODUCT_OUT"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::new();
        let sleeve = small_project("sleeve-name", Rgb(20, 90, 200));
        for (k, kind) in ProductKind::ALL.into_iter().enumerate() {
            h.st.doc.edit("None", |d| d.product = None);
            h.st.set_product(kind);
            // A design on the sleeve too (P16 Milestone 2).
            if let Some(i) = kind.placements().iter().position(|p| p.name == "Sleeve") {
                h.st.add_product_design(&sleeve).unwrap();
                h.st.edit_extra("Move", None, 0, |e| e.at = kind.placements()[i].at);
            }
            let c = [
                Rgb(250, 250, 250),
                Rgb(30, 40, 70),
                Rgb(200, 30, 40),
                Rgb(120, 120, 120),
            ][k % 4];
            h.st.edit_product("Color", None, |v| v.color = c);
            h.st.save_decoration(&dir.join(format!("{kind:?}.png"))).unwrap();
        }
        // A T-shirt with the design at the left chest and a print on the
        // back, both sides in one picture (P16 Milestone 3).
        h.st.doc.edit("None", |d| d.product = None);
        h.st.set_product(ProductKind::TShirt);
        let print = small_project("back-print", Rgb(20, 90, 200));
        h.st.turn_product();
        h.st.add_product_design(&print).unwrap();
        h.st.turn_product();
        h.st.edit_product("Both sides", None, |p| {
            p.both_sides = true;
            p.color = Rgb(30, 40, 70);
        });
        h.st.save_decoration(&dir.join("TShirtBothSides.png")).unwrap();
        // Two-tone products (P16 Milestone 6): each product with all its
        // trim parts on, then the exit test: a navy cap with a white front
        // and a gray polo with a black collar.
        let two_tone = |h: &mut Harness, kind: ProductKind, color: Rgb, trim: Rgb, off: &[&str]| {
            h.st.doc.edit("None", |d| d.product = None);
            h.st.set_product(kind);
            h.st.edit_product("Colors", None, |p| {
                p.color = color;
                p.trim = Some(Trim {
                    color: trim,
                    off: off.iter().map(|s| s.to_string()).collect(),
                });
            });
        };
        for kind in ProductKind::ALL.into_iter().filter(|k| !k.trims().is_empty()) {
            two_tone(&mut h, kind, Rgb(200, 30, 40), Rgb(250, 250, 250), &[]);
            h.st.save_decoration(&dir.join(format!("{kind:?}Trim.png"))).unwrap();
        }
        two_tone(
            &mut h,
            ProductKind::Cap,
            Rgb(30, 38, 66),
            Rgb(250, 250, 250),
            &["Brim", "Button"],
        );
        h.st.save_decoration(&dir.join("CapWhiteFront.png")).unwrap();
        two_tone(
            &mut h,
            ProductKind::Polo,
            Rgb(170, 170, 172),
            Rgb(24, 24, 26),
            &["Sleeve cuffs"],
        );
        h.st.save_decoration(&dir.join("PoloBlackCollar.png")).unwrap();
    }

    #[test]
    fn a_trim_color_stays_with_another_product() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::Polo);
        h.st.edit_product("Trim color", None, |p| {
            p.trim = Some(Trim {
                color: Rgb(24, 24, 26),
                off: Vec::new(),
            });
        });
        h.st.edit_product("Trim parts", None, |p| p.set_trim_on("Sleeve cuffs", false));
        let v = h.st.doc.design().product.clone().unwrap();
        assert!(v.trim_on("Collar") && !v.trim_on("Sleeve cuffs"));
        // Another product keeps the trim color; its parts have it.
        h.st.set_product(ProductKind::Cap);
        let v = h.st.doc.design().product.clone().unwrap();
        assert_eq!(v.trim.as_ref().map(|t| t.color), Some(Rgb(24, 24, 26)));
        assert!(v.trim_on("Front panels"));
        // Each change is one undo step.
        h.st.doc.undo();
        h.st.doc.undo();
        let v = h.st.doc.design().product.clone().unwrap();
        assert!(v.trim_on("Sleeve cuffs"));
        // The window draws a two-tone product with no change.
        h.st.product_window = true;
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
        out.textures_delta.clear();
        assert_eq!(h.st.doc.state_id(), state);
    }
}
