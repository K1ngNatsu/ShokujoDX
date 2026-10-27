//! The backdrop bitmap: import, texture, drawing and the Backdrop panel.
//! See `docs/design/p2-plan.md` (Milestone 1 decisions).

use std::sync::Arc;

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point};
use emb_model::{Backdrop, HoopSelection, MergeKey, backdrop::MIN_WIDTH_MM};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;
use crate::manipulate::ViewMap;

/// Images larger than this (pixels per side) are made smaller for the texture.
pub const MAX_TEXTURE_SIDE: u32 = 4096;
/// A new backdrop fills this part of the hoop field.
const FIT_FRACTION: f64 = 0.8;
/// Width of a new backdrop when the design has no hoop, in mm.
const OPEN_FIELD_WIDTH_MM: f64 = 100.0;
/// The picture files that the program reads (TIFF and GIF: P10 Milestone 6).
pub const FILE_TYPES: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "gif"];

/// The texture of the current backdrop, made again when the image bytes change.
pub struct BackdropTexture {
    data: Arc<Vec<u8>>,
    pub texture: TextureHandle,
}

/// Decodes an image file to RGBA pixels, upright (the EXIF orientation of
/// a photo applied), at most [`MAX_TEXTURE_SIDE`] and `max_side` pixels on
/// its long side, with the memory budget of a decode (ADR 0077).
pub fn decode(bytes: &[u8], max_side: usize) -> Result<egui::ColorImage, String> {
    let side = MAX_TEXTURE_SIDE.min(u32::try_from(max_side).unwrap_or(u32::MAX));
    let rgba = emb_art::decode_fit(bytes, side, crate::resources::decode_budget())
        .map_err(|e| crate::resources::decode_error(&e))?;
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Pixel size of an image file, upright, without a full decode.
fn pixel_size(bytes: &[u8]) -> Result<(u32, u32), String> {
    emb_art::picture_size(bytes)
}

/// Gives the backdrop and the product picture of a design that was saved
/// before P12 Milestone 1 their upright pixel size: then the program did
/// not apply the EXIF orientation, so a turned photo has its width and
/// height the wrong way round. The backdrop keeps its width and center.
pub fn upright(d: &mut emb_model::Design) {
    let turned = |data: &[u8], pixels: (u32, u32)| {
        pixel_size(data)
            .ok()
            .filter(|&p| p != pixels && p == (pixels.1, pixels.0))
    };
    if let Some(b) = d.backdrop.as_mut()
        && let Some(p) = turned(&b.data, b.pixels)
    {
        let c = b.center();
        b.pixels = p;
        b.set_center(c);
    }
    if let Some(p) = d.product.as_mut().and_then(|v| v.picture.as_mut())
        && let Some(px) = turned(&p.data, p.pixels)
    {
        p.pixels = px;
    }
}

impl AppState {
    /// The box that a new backdrop fits in: the hoop field, or an open field at the hoop center.
    fn backdrop_target(&self) -> BBox {
        let d = self.doc.design();
        let c = d.hoop.center;
        match &d.hoop.selection {
            HoopSelection::Preset { id } => match self.hoops.get(id) {
                Some(h) => h.field_bbox(c),
                None => BBox::from_center_size(c, OPEN_FIELD_WIDTH_MM, OPEN_FIELD_WIDTH_MM),
            },
            HoopSelection::OpenField => BBox::from_center_size(c, OPEN_FIELD_WIDTH_MM, OPEN_FIELD_WIDTH_MM),
        }
    }

    /// Puts the image under the design, fitted to the hoop, as one undo step.
    /// It replaces the current backdrop.
    pub fn import_bitmap_bytes(&mut self, bytes: Vec<u8>, name: &str) {
        let pixels = match pixel_size(&bytes).and_then(|p| {
            if p.0 > 0 && p.1 > 0 {
                Ok(p)
            } else {
                Err("empty image".into())
            }
        }) {
            Ok(p) => p,
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot load picture".into(),
                    lines: vec![name.to_string(), e],
                });
                return;
            }
        };
        let target = self.backdrop_target();
        let aspect = f64::from(pixels.1) / f64::from(pixels.0);
        let width = (target.width() * FIT_FRACTION).min(target.height() * FIT_FRACTION / aspect);
        let file = std::path::Path::new(name)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.to_string());
        let backdrop = Backdrop::new(file, bytes, pixels, target.center(), width);
        self.doc.edit("Load backdrop", |d| d.backdrop = Some(backdrop));
        self.message = tr("Backdrop loaded and locked. Unlock it in the Backdrop panel to move it.").into();
    }

    /// Asks for a picture file and loads it as the backdrop.
    pub fn import_bitmap_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title("Load backdrop picture")
            .add_filter("Pictures (PNG, JPEG, BMP, TIFF, GIF)", &FILE_TYPES)
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&p) {
            Ok(bytes) => self.import_bitmap_bytes(bytes, &p.display().to_string()),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot read file".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                })
            }
        }
    }

    /// Changes the backdrop as one undo step. `merge` joins the steps of one slider drag.
    pub(crate) fn edit_backdrop(&mut self, label: &str, merge: Option<&str>, f: impl FnOnce(&mut Backdrop)) {
        let change = |d: &mut emb_model::Design| {
            if let Some(b) = d.backdrop.as_mut() {
                f(b);
            }
        };
        match merge {
            Some(k) => self
                .doc
                .edit_merged(label, MergeKey::new(format!("backdrop-{k}")), change),
            None => self.doc.edit(label, change),
        };
    }

    /// The texture for the backdrop; made on first use and when the image changes.
    pub fn backdrop_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let data = self.doc.design().backdrop.as_ref()?.data.clone();
        if let Some(t) = &self.backdrop_tex
            && Arc::ptr_eq(&t.data, &data)
        {
            return Some(t.texture.clone());
        }
        let texture = match decode(&data, crate::resources::max_texture_side(ctx)) {
            Ok(img) => ctx.load_texture("backdrop", img, TextureOptions::LINEAR),
            Err(e) => {
                self.message = crate::i18n::fill("Cannot show the backdrop: {}", &[&e]);
                return None;
            }
        };
        self.backdrop_tex = Some(BackdropTexture {
            data,
            texture: texture.clone(),
        });
        Some(texture)
    }
}

/// Draws the backdrop under the design, with the design rotation.
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, map: &ViewMap, st: &mut AppState) {
    let Some(b) = st.doc.design().backdrop.clone() else {
        return;
    };
    if !b.visible {
        return;
    }
    let Some(tex) = st.backdrop_texture(ctx) else { return };
    let bb = b.bbox();
    let corners = [
        Point::new(bb.min.x, bb.min.y),
        Point::new(bb.max.x, bb.min.y),
        Point::new(bb.max.x, bb.max.y),
        Point::new(bb.min.x, bb.max.y),
    ];
    let uvs = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    let tint = Color32::WHITE.gamma_multiply(b.opacity.clamp(0.0, 1.0) as f32);
    let mut mesh = Mesh::with_texture(tex.id());
    for (c, uv) in corners.iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: map.to_screen(*c),
            uv,
            color: tint,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
    if !b.locked {
        let r = Rect::from_points(&corners.map(|c| map.to_screen(c)));
        painter.rect_stroke(
            r,
            0.0,
            egui::Stroke::new(1.0, Color32::from_rgb(80, 160, 255)),
            egui::StrokeKind::Outside,
        );
    }
}

/// The point (design coordinates) is on an unlocked, visible backdrop.
pub fn grabs(st: &AppState, p: Point) -> bool {
    st.doc
        .design()
        .backdrop
        .as_ref()
        .is_some_and(|b| b.visible && !b.locked && b.bbox().contains(p))
}

/// The Backdrop panel: shown in the Properties tab when the design has a backdrop.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(b) = st.doc.design().backdrop.clone() else {
        if ui.button(tr("Load Backdrop Picture…")).clicked() {
            st.import_bitmap_dialog();
        }
        return;
    };
    egui::CollapsingHeader::new(format!("Backdrop: {}", b.name))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut visible = b.visible;
                if ui.checkbox(&mut visible, tr("Show")).changed() {
                    st.edit_backdrop("Show backdrop", None, |b| b.visible = visible);
                    // The picture tools work on a shown picture only.
                    if !visible && st.tool.uses_backdrop() {
                        st.set_tool(crate::draw::Tool::Select);
                    }
                }
                let mut locked = b.locked;
                if ui
                    .checkbox(&mut locked, tr("Lock"))
                    .on_hover_text(tr("Unlock to drag the picture on the canvas with the Select tool."))
                    .changed()
                {
                    st.edit_backdrop(if locked { "Lock backdrop" } else { "Unlock backdrop" }, None, |b| {
                        b.locked = locked
                    });
                }
            });
            let mut opacity = b.opacity * 100.0;
            let r = ui.add(
                egui::Slider::new(&mut opacity, 10.0..=100.0)
                    .clamping(egui::SliderClamping::Edits)
                    .text("Strength")
                    .suffix(" %"),
            );
            if r.changed() {
                st.edit_backdrop("Dim backdrop", Some("opacity"), |b| b.opacity = opacity / 100.0);
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                st.doc.end_merge();
            }
            let mut width = b.width_mm;
            let r = ui.horizontal(|ui| {
                ui.label(tr("Width"));
                ui.add(
                    egui::DragValue::new(&mut width)
                        .range(MIN_WIDTH_MM..=2000.0)
                        .clamp_existing_to_range(false)
                        .speed(0.5)
                        .suffix(" mm"),
                )
            });
            ui.weak(crate::i18n::fill(
                "Height {} mm ({} × {} px)",
                &[&format!("{:.1}", b.height_mm()), &b.pixels.0, &b.pixels.1],
            ));
            crate::picture::panel(ui, st);
            if r.inner.changed() {
                st.edit_backdrop("Resize backdrop", Some("width"), |b| b.set_width(width));
            }
            if r.inner.drag_stopped() || r.inner.lost_focus() {
                st.doc.end_merge();
            }
            let c = b.center() - st.doc.design().hoop.center;
            let (mut x, mut y) = (c.x, c.y);
            let r = ui.horizontal(|ui| {
                ui.label(tr("Center X"));
                let a = ui.add(egui::DragValue::new(&mut x).speed(0.5).suffix(" mm"));
                ui.label(tr("Y"));
                let b = ui.add(egui::DragValue::new(&mut y).speed(0.5).suffix(" mm"));
                a | b
            });
            if r.inner.changed() {
                let hc = st.doc.design().hoop.center;
                st.edit_backdrop("Move backdrop", Some("center"), |b| b.set_center(hc + Point::new(x, y)));
            }
            if r.inner.drag_stopped() || r.inner.lost_focus() {
                st.doc.end_merge();
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Replace…")).clicked() {
                    st.import_bitmap_dialog();
                }
                if ui
                    .button(tr("Edit in Another Program"))
                    .on_hover_text(tr("Save the picture in the other program: the backdrop changes here."))
                    .clicked()
                {
                    st.edit_backdrop_outside();
                }
                if ui.button(tr("Remove")).clicked() {
                    st.doc.edit("Remove backdrop", |d| d.backdrop = None);
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn tiff_and_gif_pictures_decode() {
        let img = image::RgbaImage::from_fn(12, 8, |x, y| image::Rgba([(x * 20) as u8, (y * 30) as u8, 90, 255]));
        for format in [image::ImageFormat::Tiff, image::ImageFormat::Gif] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(img.clone())
                .write_to(&mut bytes, format)
                .unwrap();
            let bytes = bytes.into_inner();
            let pic = decode(&bytes, 4096).unwrap();
            assert_eq!(pic.size, [12, 8], "{format:?}");
            assert_eq!(pixel_size(&bytes).unwrap(), (12, 8));
        }
    }

    /// A 40 × 20 px JPEG that a phone took turned: its EXIF orientation is
    /// 6 (turn 90° clockwise to see it upright).
    fn turned_jpeg() -> Vec<u8> {
        use image::ImageEncoder;
        let exif = [
            b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        let img = image::RgbImage::from_pixel(40, 20, image::Rgb([200, 30, 40]));
        let mut out = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new(&mut out);
        enc.set_exif_metadata(exif.to_vec()).unwrap();
        enc.write_image(img.as_raw(), 40, 20, image::ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    #[test]
    fn a_turned_photo_is_upright_also_in_an_old_project() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.import_bitmap_bytes(turned_jpeg(), "phone.jpg");
        let b = st.doc.design().backdrop.clone().unwrap();
        assert_eq!(b.pixels, (20, 40));
        assert_eq!(decode(&b.data, 4096).unwrap().size, [20, 40]);
        assert!(b.height_mm() > b.width_mm);
        // An old project kept the size of the pixels in the file.
        let mut d = emb_model::Design::new("");
        d.backdrop = Some(Backdrop::new(
            "phone.jpg",
            turned_jpeg(),
            (40, 20),
            Point::new(5.0, 5.0),
            60.0,
        ));
        st.start_design(d);
        let b = st.doc.design().backdrop.clone().unwrap();
        assert_eq!(b.pixels, (20, 40));
        assert!((b.width_mm - 60.0).abs() < 1e-9 && b.center().distance(Point::new(5.0, 5.0)) < 1e-9);
        assert!(!st.doc.can_undo(), "the fix is not an undo step");
    }

    /// A 4 × 2 pixel PNG.
    fn png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(4, 2, image::Rgba([200, 30, 40, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn import_fits_the_hoop_and_undoes() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.import_bitmap_bytes(png(), "C:/pictures/logo.png");
        let b = st.doc.design().backdrop.clone().unwrap();
        assert_eq!(b.name, "logo.png");
        assert_eq!(b.pixels, (4, 2));
        assert!(b.locked && b.visible);
        let field = st.backdrop_target();
        assert!(field.contains_box(&b.bbox()));
        assert!((b.center().distance(field.center())) < 1e-9);
        assert!(decode(&b.data, 4096).unwrap().size == [4, 2]);
        st.undo();
        assert!(st.doc.design().backdrop.is_none());
    }

    #[test]
    fn bad_data_is_a_notice() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.import_bitmap_bytes(b"not a picture".to_vec(), "x.png");
        assert!(st.notice.is_some());
        assert!(st.doc.design().backdrop.is_none());
    }

    #[test]
    fn unlocked_backdrop_drags_and_locked_does_not() {
        use eframe::egui::{Modifiers, PointerButton};

        let mut h = crate::test_util::Harness::empty();
        h.st.import_bitmap_bytes(png(), "logo.png");
        h.frame(vec![]);
        let start = h.st.doc.design().backdrop.clone().unwrap();
        let from = h.screen(start.center());
        let to = from + egui::Vec2::new(40.0, 0.0);
        // Locked: the drag is a box select.
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        assert_eq!(h.st.doc.design().backdrop.as_ref().unwrap().origin, start.origin);

        h.st.edit_backdrop("Unlock backdrop", None, |b| b.locked = false);
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let moved = h.st.doc.design().backdrop.clone().unwrap();
        let dx_mm = 40.0 / h.st.view.scale();
        assert!(
            (moved.origin.x - start.origin.x - dx_mm).abs() < 1e-3,
            "{:?}",
            moved.origin
        );
        // One drag is one undo step.
        h.st.undo();
        assert_eq!(h.st.doc.design().backdrop.as_ref().unwrap().origin, start.origin);
    }

    #[test]
    fn panel_and_canvas_draw_the_backdrop() {
        let mut h = crate::test_util::Harness::empty();
        h.st.import_bitmap_bytes(png(), "logo.png");
        h.frame(vec![]);
        assert!(h.st.backdrop_tex.is_some(), "the canvas made the texture");
        let ctx = h.ctx.clone();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, &mut h.st));
        });
        out.textures_delta.clear();
        // The panel can hide the backdrop; the canvas then draws no texture.
        h.st.edit_backdrop("Show backdrop", None, |b| b.visible = false);
        h.frame(vec![]);
        assert!(!h.st.doc.design().backdrop.as_ref().unwrap().visible);
    }

    #[test]
    fn backdrop_is_saved_in_the_project() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.import_bitmap_bytes(png(), "logo.png");
        let s = emb_model::project::to_string(st.doc.design());
        let back = emb_model::project::from_str(&s).unwrap();
        assert_eq!(back.backdrop, st.doc.design().backdrop);
    }
}
