//! Product backdrops and the virtual decoration picture (P7 Milestone 5):
//! the Product window, the product under the design on the canvas, and
//! Save picture on product. The product does not sew.

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point};
use emb_model::{MergeKey, ProductKind, ProductPicture, ProductView, Rgb};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// Pixels for each mm of the product texture on the canvas.
const TEXTURE_PX_PER_MM: f64 = 2.0;
/// Pixels for each mm of the saved picture.
pub const DECORATION_PX_PER_MM: f64 = 4.0;
/// The default product color: white.
const WHITE: Rgb = Rgb(250, 250, 250);

/// The texture of the product, and what it shows.
pub struct ProductTexture {
    key: (ProductKind, Rgb, usize, usize),
    texture: TextureHandle,
}

fn key(v: &ProductView) -> (ProductKind, Rgb, usize, usize) {
    let (ptr, len) = v
        .picture
        .as_ref()
        .map_or((0, 0), |p| (Arc::as_ptr(&p.data) as usize, p.data.len()));
    (v.kind, v.color, ptr, len)
}

impl AppState {
    /// Changes the product as one undo step. With `merge`, a drag is one step.
    fn edit_product(&mut self, label: &str, merge: Option<&str>, f: impl FnOnce(&mut ProductView)) {
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
    /// usual place of the product.
    pub fn set_product(&mut self, kind: ProductKind) {
        let color = self.doc.design().product.as_ref().map_or(WHITE, |v| v.color);
        let mut v = ProductView::new(kind, color);
        if let Some(p) = kind.placements().first() {
            v.put_design_at(self.doc.design(), p.at);
        }
        self.doc.edit("Product", |d| d.product = Some(v));
        self.fit_product = true;
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
        };
        let size = Point::new(width_mm, picture.height_mm());
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
        let mut v = ProductView::new(kind, WHITE);
        v.origin = c - size * 0.5;
        v.picture = Some(picture);
        self.doc.edit("Product picture", |d| d.product = Some(v));
        self.fit_product = true;
        Ok(())
    }

    pub fn product_picture_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title("Picture of your product")
            .add_filter("Pictures (PNG, JPEG, BMP, TIFF, GIF)", &crate::backdrop::FILE_TYPES)
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
        let v = self.doc.design().product.clone()?;
        let photo = v.picture.as_ref().and_then(|p| {
            emb_art::decode_fit(
                &p.data,
                crate::backdrop::MAX_TEXTURE_SIDE,
                crate::resources::decode_budget(),
            )
            .ok()
        });
        let segments: Vec<emb_render::Segment> = self.plan_view().segments.iter().map(|(s, _)| *s).collect();
        // The 3D look, with no fabric texture: the product is under the design.
        let look = emb_render::raster::Look {
            fabric: None,
            ..self.settings.look.picture_look(None)
        };
        Some(emb_render::product::decoration(
            &v,
            photo.as_ref(),
            &segments,
            DECORATION_PX_PER_MM,
            0.4,
            &look,
        ))
    }

    /// Writes the picture of the design on the product as PNG.
    pub fn save_decoration(&mut self, path: &Path) -> Result<(), String> {
        let pic = self
            .decoration_picture()
            .ok_or("Choose a product first (Design > Product).")?;
        std::fs::write(path, pic.png()).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save_decoration_dialog(&mut self) {
        if self.doc.design().product.is_none() {
            self.message = tr("Choose a product first (Design > Product).").into();
            return;
        }
        let Some(p) = rfd::FileDialog::new()
            .set_title("Save picture on product (PNG)")
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

    fn product_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let v = self.doc.design().product.clone()?;
        let k = key(&v);
        if let Some(t) = &self.product_tex
            && t.key == k
        {
            return Some(t.texture.clone());
        }
        let image = match &v.picture {
            Some(p) => crate::backdrop::decode(&p.data, crate::resources::max_texture_side(ctx)).ok()?,
            None => {
                let (pic, _) = emb_render::product::product_picture(&v, TEXTURE_PX_PER_MM);
                egui::ColorImage::from_rgba_unmultiplied([pic.width as usize, pic.height as usize], &pic.rgba)
            }
        };
        let texture = crate::resources::load_texture(ctx, "product", image, TextureOptions::LINEAR);
        self.product_tex = Some(ProductTexture {
            key: k,
            texture: texture.clone(),
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
        Some(v) if v.picture.is_some() => "My picture".to_string(),
        Some(v) => v.kind.name().to_string(),
        None => "None".to_string(),
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
                for k in ProductKind::ALL {
                    let on = current.as_ref().is_some_and(|v| v.picture.is_none() && v.kind == k);
                    if ui.selectable_label(on, tr(k.name())).clicked() {
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
    match pick {
        Some(Some(k)) => st.set_product(k),
        Some(None) => {
            st.doc.edit("No product", |d| d.product = None);
        }
        None => {}
    }
    let Some(v) = st.doc.design().product.clone() else {
        return;
    };
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
        ui.add_space(4.0);
        ui.label(tr("Put the design at:"));
        let design = st.doc.design().clone();
        let on = v.placement_of(&design).map(|p| p.name);
        let size = design.view_extents().map(|b| (b.width(), b.height()));
        let mut put = None;
        ui.horizontal_wrapped(|ui| {
            for (i, p) in v.kind.placements().iter().enumerate() {
                let tip = format!("Usual largest design here: {:.0} × {:.0} mm", p.max.0, p.max.1);
                if ui
                    .selectable_label(on == Some(p.name), p.name)
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
                format!(
                    "The design ({w:.0} × {h:.0} mm) is larger than the usual size for {} ({:.0} × {:.0} mm).",
                    p.name.to_lowercase(),
                    p.max.0,
                    p.max.1
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
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(tr("Zoom to product")).clicked() {
            st.fit_product = true;
        }
        if ui.button(tr("Save picture on product…")).clicked() {
            st.save_decoration_dialog();
        }
    });
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
mod tests {
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
    }

    /// Writes a picture of the sample design on each product:
    /// `EMB_PRODUCT_OUT=<folder> cargo test -p emb-app -- --ignored write_products`.
    #[test]
    #[ignore]
    fn write_products() {
        let dir = std::path::PathBuf::from(std::env::var("EMB_PRODUCT_OUT").expect("set EMB_PRODUCT_OUT"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::new();
        for (k, kind) in ProductKind::ALL.into_iter().enumerate() {
            h.st.set_product(kind);
            let c = [
                Rgb(250, 250, 250),
                Rgb(30, 40, 70),
                Rgb(200, 30, 40),
                Rgb(120, 120, 120),
            ][k % 4];
            h.st.edit_product("Color", None, |v| v.color = c);
            h.st.save_decoration(&dir.join(format!("{kind:?}.png"))).unwrap();
        }
    }
}
