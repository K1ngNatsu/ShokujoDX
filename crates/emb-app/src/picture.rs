//! Picture tools for the backdrop (P6 Milestone 5): crop (rectangle,
//! ellipse, polygon), adjust (brightness, contrast, sharpness), prepare
//! colors (color reduction), and the threads nearest to the colors. Each
//! tool changes the backdrop picture in one undo step. Milestone 6 adds
//! auto-trace (the color areas become vectors) and click-to-stitch (a click
//! on a color area makes a fill, a satin, a center line or an outline). The
//! engine work is in `emb_art`.

use std::sync::Arc;

use eframe::egui;
use emb_art::adjust::Adjust;
use emb_art::autotrace::{self, TraceSettings};
use emb_art::click::ClickStitch;
use emb_art::crop::CropShape;
use emb_art::reduce::{ReduceSpec, Reduced, match_threads, reduce};
use emb_geom::{BBox, Point, Transform};
use emb_model::{ObjectId, ObjectKind, Rgb, VectorStyle, colorway::chart_look};

use crate::app::AppState;
use crate::canvas::color32;
use crate::i18n::tr;

/// The values of the picture tools.
#[derive(Default)]
pub struct PictureTools {
    pub adjust: Adjust,
    pub reduce: ReduceSpec,
    /// The colors of the last color reduction, for thread matching.
    pub palette: Vec<Rgb>,
    pub trace: TraceSettings,
    /// Auto-trace also traces the color at the edge of the picture (the
    /// background); by default it is left out.
    pub keep_background: bool,
    /// What the Click to stitch tool makes.
    pub click: ClickStitch,
    /// The colors of the backdrop for tracing, with the picture data they
    /// are from (kept, so the data is not freed and its address not used again).
    colors: Option<(Arc<Vec<u8>>, Arc<Reduced>)>,
}

/// A picture with more colors than this is reduced before it is traced.
const TRACE_MAX_COLORS: usize = 32;
/// A click satin wider than this gets a note: a fill sews better.
const WIDE_SATIN_MM: f64 = 10.0;
/// Adjust, Prepare colors, Trace and Click to stitch work on the backdrop
/// at most this many pixels on its long side (as the canvas shows it), so a
/// large photo does not take gigabytes of memory (ADR 0077).
const TOOL_PX: u32 = crate::backdrop::MAX_TEXTURE_SIDE;

impl AppState {
    /// The backdrop picture, at most `max_side` pixels on its long side,
    /// with the memory budget of a decode; an error goes to the message.
    fn backdrop_image(&mut self, max_side: u32) -> Option<emb_art::RgbaImage> {
        let b = self.doc.design().backdrop.as_ref()?;
        match emb_art::decode_fit(&b.data, max_side, crate::resources::decode_budget()) {
            Ok(img) => Some(img),
            Err(e) => {
                self.message = crate::i18n::fill(
                    "Cannot read the backdrop picture: {}",
                    &[&crate::resources::decode_error(&e)],
                );
                None
            }
        }
    }

    /// Puts `img` in place of the backdrop picture. `offset` is the pixel
    /// position of its top-left corner in the picture that the tool worked
    /// on, which was `from_width` pixels wide; the scale stays.
    fn replace_backdrop_image(&mut self, label: &str, img: &emb_art::RgbaImage, offset: (u32, u32), from_width: u32) {
        let Ok(png) = emb_art::encode_png(img) else {
            self.message = tr("The picture could not be made.").into();
            return;
        };
        self.doc.edit(label, |d| {
            if let Some(b) = d.backdrop.as_mut() {
                let mm_per_px = b.width_mm / f64::from(from_width.max(1));
                b.origin += Point::new(f64::from(offset.0), f64::from(offset.1)) * mm_per_px;
                b.width_mm = f64::from(img.width()) * mm_per_px;
                b.pixels = img.dimensions();
                b.data = std::sync::Arc::new(png);
            }
        });
    }

    /// A point in design mm as a pixel position of the backdrop picture.
    fn to_backdrop_px(&self, p: Point) -> Option<Point> {
        let b = self.doc.design().backdrop.as_ref()?;
        let s = f64::from(b.pixels.0) / b.width_mm;
        let local = self.doc.design().rotation_transform().inverse()?.apply(p) - b.origin;
        Some(local * s)
    }

    /// Crops the backdrop to the shape that the crop tool clicked (design
    /// mm): two corners of a rectangle or of the box of an ellipse, or the
    /// corners of a polygon.
    pub fn crop_backdrop(&mut self, tool: crate::draw::Tool, pts: &[Point]) {
        use crate::draw::Tool;
        let shown = self.doc.design().backdrop.as_ref().is_some_and(|b| b.visible);
        if !shown {
            self.message = tr("Load a backdrop picture first, or show it (Backdrop panel).").into();
            return;
        }
        // At full size: the cut-out keeps its detail.
        let Some(img) = self.backdrop_image(u32::MAX) else {
            return;
        };
        let px: Vec<Point> = pts.iter().filter_map(|p| self.to_backdrop_px(*p)).collect();
        let shape = match tool {
            Tool::CropRect => BBox::from_points(px).map(CropShape::Rect),
            Tool::CropEllipse => BBox::from_points(px).map(CropShape::Ellipse),
            _ => Some(CropShape::Polygon(px)),
        };
        match shape.and_then(|s| emb_art::crop::crop(&img, &s)) {
            Some((out, at)) => {
                self.replace_backdrop_image("Crop picture", &out, at, img.width());
                self.message = crate::i18n::fill("Picture cropped to {} × {} px.", &[&(out.width()), &(out.height())]);
            }
            None => self.message = tr("The crop shape is not on the picture.").into(),
        }
    }

    /// Applies brightness, contrast and sharpness to the backdrop.
    pub fn adjust_backdrop(&mut self) {
        let a = self.picture.adjust;
        if a.is_none() {
            return;
        }
        let Some(img) = self.backdrop_image(TOOL_PX) else {
            return;
        };
        let out = emb_art::adjust::adjust(&img, &a);
        self.replace_backdrop_image("Adjust picture", &out, (0, 0), img.width());
        self.picture.adjust = Adjust::default();
    }

    /// Reduces the colors of the backdrop ("prepare colors").
    pub fn reduce_backdrop(&mut self) {
        let Some(img) = self.backdrop_image(TOOL_PX) else {
            return;
        };
        let r = reduce(&img, &self.picture.reduce);
        self.replace_backdrop_image("Prepare colors", &r.to_image(), (0, 0), img.width());
        self.message = crate::i18n::fill("The picture has {} colors now.", &[&(r.palette.len())]);
        self.picture.palette = r.palette;
    }

    /// The colors of the backdrop for tracing: its own colors if it has
    /// [`TRACE_MAX_COLORS`] or fewer (a prepared picture), else a color
    /// reduction with the Prepare colors settings (`true`).
    fn backdrop_colors(&mut self) -> Option<(Arc<Reduced>, bool)> {
        let data = self.doc.design().backdrop.as_ref().filter(|b| b.visible)?.data.clone();
        if let Some((d, r)) = &self.picture.colors
            && Arc::ptr_eq(d, &data)
        {
            return Some((r.clone(), false));
        }
        let img = self.backdrop_image(TOOL_PX)?;
        let (r, reduced) = match emb_art::reduce::exact(&img, TRACE_MAX_COLORS) {
            Some(r) => (r, false),
            None => (reduce(&img, &self.picture.reduce), true),
        };
        let r = Arc::new(r);
        self.picture.colors = Some((data, r.clone()));
        Some((r, reduced))
    }

    /// The transform from pixels of the backdrop picture (`width` pixels
    /// wide) to design mm.
    fn backdrop_to_design(&self, width: u32) -> Option<Transform> {
        let d = self.doc.design();
        let b = d.backdrop.as_ref()?;
        let s = b.width_mm / f64::from(width.max(1));
        Some(
            Transform::scale(s, s)
                .then(Transform::translate(b.origin.x, b.origin.y))
                .then(d.rotation_transform()),
        )
    }

    /// Auto-trace: each color area of the backdrop becomes a vector shape
    /// in its color (the background is left out unless `keep_background`).
    /// One undo step; the new vectors are selected.
    pub fn trace_backdrop(&mut self) {
        let Some((r, reduced)) = self.backdrop_colors() else {
            self.message = tr("Load a backdrop picture first, or show it (Backdrop panel).").into();
            return;
        };
        let Some(t) = self.backdrop_to_design(r.width) else {
            return;
        };
        let skip = if self.picture.keep_background {
            None
        } else {
            autotrace::background(&r)
        };
        let areas: Vec<autotrace::Area> = autotrace::trace(&r, &self.picture.trace)
            .into_iter()
            .filter(|a| Some(a.color) != skip)
            .collect();
        if areas.is_empty() {
            self.message = tr("Nothing to trace: the picture has no area apart from its background.").into();
            return;
        }
        let colors = {
            let mut c: Vec<u16> = areas.iter().map(|a| a.color).collect();
            c.dedup();
            c.len()
        };
        let ids: Vec<ObjectId> = self.doc.edit("Trace Picture", |d| {
            areas
                .iter()
                .map(|a| {
                    let color = r.palette[a.color as usize];
                    let thread = crate::vectors::nearest_thread(d, color);
                    let kind = ObjectKind::Vector {
                        path: a.transformed(&t).path(),
                        style: VectorStyle::filled(color),
                    };
                    d.add_object(thread, kind)
                })
                .collect()
        });
        self.show_vectors = true;
        self.message = crate::i18n::fill(
            "{} shapes traced in {} colors. Convert them (Artwork > Convert), or click areas to stitch.",
            &[&(ids.len()), &colors],
        );
        if reduced {
            self.message += &format!(
                " The picture had many colors: it was reduced to {} (Prepare colors gives more control).",
                r.palette.len()
            );
        }
        self.selection.set_all(ids);
    }

    /// Click to stitch: the color area of the backdrop under `p` (design
    /// mm) becomes embroidery, as `picture.click` sets. One undo step.
    pub fn click_stitch(&mut self, p: Point) {
        let Some((r, _)) = self.backdrop_colors() else {
            self.message = tr("Load a backdrop picture first, or show it (Backdrop panel).").into();
            return;
        };
        let (Some(t), Some(px)) = (self.backdrop_to_design(r.width), self.to_backdrop_px(p)) else {
            return;
        };
        // From pixels of the picture to pixels of the colors (at most `TOOL_PX`).
        let pixels = self.doc.design().backdrop.as_ref().map_or(r.width, |b| b.pixels.0);
        let px = px * (f64::from(r.width) / f64::from(pixels.max(1)));
        let area = (px.x >= 0.0 && px.y >= 0.0)
            .then(|| autotrace::trace_at(&r, px.x as u32, px.y as u32, &self.picture.trace))
            .flatten();
        let Some(area) = area.map(|a| a.transformed(&t)) else {
            self.message = tr("Click a color area of the backdrop picture.").into();
            return;
        };
        let how = self.picture.click;
        let kinds = emb_art::click::objects(&area.outer, &area.holes, how, &self.doc.design().current);
        if kinds.is_empty() {
            self.message = crate::i18n::fill("{}: nothing can be made of this area.", &[&tr(how.name())]);
            return;
        }
        let widest = kinds
            .iter()
            .filter_map(|k| match k {
                ObjectKind::Satin { geometry, .. } => {
                    geometry.rungs.iter().map(|(a, b)| a.distance(*b)).reduce(f64::max)
                }
                _ => None,
            })
            .fold(0.0, f64::max);
        let color = r.palette[area.color as usize];
        let ids: Vec<ObjectId> = self.doc.edit(how.name(), |d| {
            let thread = crate::vectors::nearest_thread(d, color);
            kinds.into_iter().map(|k| d.add_object(thread, k)).collect()
        });
        self.message = crate::i18n::fill(
            "{}: {} object{} added. Click the next area, or press Esc.",
            &[&tr(how.name()), &(ids.len()), &(if ids.len() == 1 { "" } else { "s" })],
        );
        if widest > WIDE_SATIN_MM {
            self.message += &format!(" The satin is up to {widest:.1} mm wide: a fill sews better.");
        }
        self.selection.set_all(ids);
    }

    /// Adds a design thread for each color of the last color reduction: the
    /// nearest thread of the chart of the Threads panel, or the color
    /// itself if there is no chart. Threads with the same color are not
    /// added twice. One undo step.
    pub fn add_palette_threads(&mut self) {
        let palette = self.picture.palette.clone();
        if palette.is_empty() {
            return;
        }
        let chart = self.charts.get(self.chart_index).cloned();
        let looks: Vec<emb_model::ThreadLook> = match &chart {
            Some(c) => match_threads(&palette, c)
                .into_iter()
                .zip(&palette)
                .map(|(m, rgb)| match m {
                    Some(i) => chart_look(c, i),
                    None => plain_look(*rgb),
                })
                .collect(),
            None => palette.iter().map(|c| plain_look(*c)).collect(),
        };
        let n = self.doc.edit("Threads from picture", |d| {
            let mut n = 0;
            for look in looks {
                if d.threads.iter().any(|t| t.color == look.color) {
                    continue;
                }
                let id = d.add_thread(look.name.clone(), look.color);
                if let Some(t) = d.threads.iter_mut().find(|t| t.id == id) {
                    t.brand = look.brand;
                    t.code = look.code;
                }
                n += 1;
            }
            n
        });
        self.message = crate::i18n::fill("{} threads added from the picture colors.", &[&n]);
    }
}

fn plain_look(c: Rgb) -> emb_model::ThreadLook {
    emb_model::ThreadLook {
        name: format!("Color {}", c.to_hex()),
        color: c,
        brand: None,
        code: None,
    }
}

/// The picture tools in the Backdrop panel.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    egui::CollapsingHeader::new(tr("Adjust picture"))
        .default_open(false)
        .show(ui, |ui| {
            let a = &mut st.picture.adjust;
            for (label, v, range) in [
                (tr("Brightness"), &mut a.brightness, -1.0..=1.0),
                (tr("Contrast"), &mut a.contrast, -1.0..=1.0),
                (tr("Sharpen"), &mut a.sharpen, 0.0..=2.0),
            ] {
                ui.add(
                    egui::Slider::new(v, range)
                        .clamping(egui::SliderClamping::Edits)
                        .text(label)
                        .fixed_decimals(2),
                );
            }
            ui.horizontal(|ui| {
                let some = !st.picture.adjust.is_none();
                if ui.add_enabled(some, egui::Button::new(tr("Apply"))).clicked() {
                    st.adjust_backdrop();
                }
                if ui.add_enabled(some, egui::Button::new(tr("Reset"))).clicked() {
                    st.picture.adjust = Adjust::default();
                }
            });
        });
    egui::CollapsingHeader::new(tr("Prepare colors"))
        .default_open(false)
        .show(ui, |ui| {
            ui.weak(tr(
                "Fewer colors, no specks, clean edges: ready to trace or to digitize by hand.",
            ));
            let r = &mut st.picture.reduce;
            ui.horizontal(|ui| {
                ui.label(tr("Colors"));
                ui.add(
                    egui::DragValue::new(&mut r.colors)
                        .range(2..=32)
                        .clamp_existing_to_range(false),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr("Remove specks under")).on_hover_text(tr(
                    "Areas of one color with fewer pixels than this take the color round them. 0: keep all.",
                ));
                ui.add(
                    egui::DragValue::new(&mut r.min_speck_px)
                        .range(0..=10_000)
                        .clamp_existing_to_range(false)
                        .suffix(" px"),
                );
            });
            ui.checkbox(&mut r.clean_edges, tr("Clean edges"));
            if ui.button(tr("Prepare colors")).clicked() {
                st.reduce_backdrop();
            }
            if !st.picture.palette.is_empty() {
                ui.separator();
                let chart = st.charts.get(st.chart_index);
                ui.label(match chart {
                    Some(c) => format!("Nearest threads ({}):", c.name),
                    None => "Colors:".to_string(),
                });
                let matched = chart.map(|c| match_threads(&st.picture.palette, c));
                for (i, c) in st.picture.palette.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color32(*c));
                        let thread = matched
                            .as_ref()
                            .and_then(|m| m[i])
                            .and_then(|k| chart.map(|ch| chart_look(ch, k)));
                        match thread {
                            Some(t) => {
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, color32(t.color));
                                ui.label(t.name);
                            }
                            None => {
                                ui.label(c.to_hex());
                            }
                        }
                    });
                }
                if ui
                    .button(tr("Add these threads"))
                    .on_hover_text(tr("Add a design thread for each color (the nearest chart thread)."))
                    .clicked()
                {
                    st.add_palette_threads();
                }
            }
        });
    egui::CollapsingHeader::new(tr("Trace and click to stitch"))
        .default_open(false)
        .show(ui, |ui| {
            ui.weak(tr(
                "Trace: each color area becomes a vector shape. Click to stitch: click an area to stitch it.",
            ));
            let t = &mut st.picture.trace;
            ui.add(
                egui::Slider::new(&mut t.tolerance_px, 0.2..=3.0)
                    .clamping(egui::SliderClamping::Edits)
                    .text(tr("Smoothing (px)")),
            )
            .on_hover_text(tr(
                "How far the curves can go from the pixel edges. More: fewer, smoother curves.",
            ));
            ui.add(
                egui::Slider::new(&mut t.corner_deg, 20.0..=150.0)
                    .clamping(egui::SliderClamping::Edits)
                    .text(tr("Corners (°)")),
            )
            .on_hover_text(tr("A turn larger than this stays a sharp corner."));
            ui.horizontal(|ui| {
                ui.label(tr("Leave out areas under"));
                ui.add(
                    egui::DragValue::new(&mut t.min_area_px)
                        .range(1..=10_000)
                        .clamp_existing_to_range(false)
                        .suffix(" px"),
                );
            });
            ui.checkbox(&mut st.picture.keep_background, tr("Trace the background too"))
                .on_hover_text(tr(
                    "The background is the most common color at the edge of the picture.",
                ));
            if ui.button(tr("Trace to vectors")).clicked() {
                st.trace_backdrop();
            }
            ui.separator();
            ui.label(tr("Click to stitch makes:"));
            for c in ClickStitch::ALL {
                if ui.radio(st.picture.click == c, tr(c.name())).clicked() {
                    st.picture.click = c;
                    st.set_tool(crate::draw::Tool::ClickStitch);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use emb_art::RgbaImage;
    use image::Rgba;

    use super::*;
    use crate::draw::Tool;
    use crate::test_util::Harness;

    /// A 100 × 50 px backdrop, 100 mm wide, at (0, 0): left half red,
    /// right half blue.
    fn with_backdrop() -> Harness {
        let mut h = Harness::empty();
        let img = RgbaImage::from_fn(100, 50, |x, _| {
            if x < 50 {
                Rgba([210, 20, 30, 255])
            } else {
                Rgba([20, 40, 200, 255])
            }
        });
        let png = emb_art::encode_png(&img).unwrap();
        h.st.doc.edit("Add backdrop", |d| {
            let mut b = emb_model::Backdrop::new("test.png", png, (100, 50), Point::new(50.0, 25.0), 100.0);
            b.origin = Point::ZERO;
            d.backdrop = Some(b);
            d.rotation = Default::default();
        });
        h
    }

    #[test]
    fn crop_keeps_the_scale_and_moves_the_origin() {
        let mut h = with_backdrop();
        let steps = h.st.doc.undo_len();
        h.st.crop_backdrop(Tool::CropRect, &[Point::new(10.0, 5.0), Point::new(40.0, 25.0)]);
        let b = h.st.doc.design().backdrop.clone().unwrap();
        assert_eq!(b.pixels, (30, 20));
        assert!(
            (b.width_mm - 30.0).abs() < 1e-9 && b.origin.distance(Point::new(10.0, 5.0)) < 1e-9,
            "{b:?}"
        );
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        h.st.undo();
        assert_eq!(h.st.doc.design().backdrop.as_ref().unwrap().pixels, (100, 50));
    }

    #[test]
    fn adjust_reduce_and_threads() {
        let mut h = with_backdrop();
        h.st.picture.adjust.brightness = 0.1;
        h.st.adjust_backdrop();
        assert!(h.st.picture.adjust.is_none(), "the values go back to zero");
        h.st.picture.reduce = ReduceSpec {
            colors: 2,
            ..ReduceSpec::default()
        };
        h.st.reduce_backdrop();
        assert_eq!(h.st.picture.palette.len(), 2);
        let before = h.st.doc.design().threads.len();
        h.st.add_palette_threads();
        assert_eq!(h.st.doc.design().threads.len(), before + 2);
        h.st.add_palette_threads();
        assert_eq!(h.st.doc.design().threads.len(), before + 2, "no thread twice");
    }

    /// A 120 × 60 px backdrop, 60 mm wide, at (0, 0) (0.5 mm a pixel):
    /// white, with a red square ring (40 px, hole 20 px) and a blue bar
    /// (80 × 8 px).
    fn with_logo() -> Harness {
        let mut h = Harness::empty();
        let img = RgbaImage::from_fn(120, 60, |x, y| {
            let ring = (5..45).contains(&x) && (10..50).contains(&y);
            let hole = (15..35).contains(&x) && (20..40).contains(&y);
            if ring && !hole {
                Rgba([200, 0, 0, 255])
            } else if (50..115).contains(&x) && (26..34).contains(&y) {
                Rgba([0, 0, 200, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let png = emb_art::encode_png(&img).unwrap();
        h.st.doc.edit("Add backdrop", |d| {
            let mut b = emb_model::Backdrop::new("logo.png", png, (120, 60), Point::new(30.0, 15.0), 60.0);
            b.origin = Point::ZERO;
            d.backdrop = Some(b);
            d.rotation = Default::default();
        });
        h
    }

    #[test]
    fn trace_makes_a_vector_for_each_area_but_the_background() {
        let mut h = with_logo();
        let steps = h.st.doc.undo_len();
        h.st.trace_backdrop();
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let d = h.st.doc.design();
        let vectors: Vec<(&emb_geom::Path, VectorStyle)> = d
            .objects()
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Vector { path, style } => Some((path, *style)),
                _ => None,
            })
            .collect();
        assert_eq!(vectors.len(), 2, "the ring and the bar, not the white");
        assert_eq!(h.st.selection.ids().len(), 2);
        // The ring has its hole and is 20 mm wide (0.5 mm a pixel).
        let (ring, style) = vectors.iter().find(|v| v.1.fill == Some(Rgb(200, 0, 0))).unwrap();
        assert_eq!(ring.subpaths().len(), 2);
        let bb = ring.control_bbox().unwrap();
        assert!(
            (bb.width() - 20.0).abs() < 1e-9 && (bb.min.x - 2.5).abs() < 1e-9,
            "{bb:?}"
        );
        assert_eq!(style.stroke, None);
        // With the background too: also the white round the shapes and in the ring.
        h.st.undo();
        h.st.picture.keep_background = true;
        h.st.trace_backdrop();
        assert_eq!(h.st.doc.design().objects().len(), 4);
    }

    #[test]
    fn click_to_stitch_makes_a_fill_a_satin_and_a_line() {
        let mut h = with_logo();
        h.st.set_tool(Tool::ClickStitch);
        // A click on the ring (pixel 8, 30 = 4 mm, 15 mm): a fill with its hole.
        h.st.picture.click = ClickStitch::Fill;
        let steps = h.st.doc.undo_len();
        h.st.click_stitch(Point::new(4.2, 15.2));
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let d = h.st.doc.design();
        let fill = &d.objects()[0];
        assert!(
            matches!(&fill.kind, ObjectKind::Fill { holes, .. } if holes.len() == 1),
            "{:?}",
            fill.kind
        );
        assert_eq!(d.thread(fill.thread).unwrap().color, Rgb(200, 0, 0));
        // A click on the bar: one satin column, 4 mm wide.
        h.st.picture.click = ClickStitch::Satin;
        h.st.click_stitch(Point::new(40.0, 15.0));
        let d = h.st.doc.design();
        let ObjectKind::Satin { geometry, .. } = &d.objects()[1].kind else {
            panic!("{:?}", d.objects()[1].kind);
        };
        let mid = geometry.rungs[geometry.rungs.len() / 2];
        assert!((mid.0.distance(mid.1) - 4.0).abs() < 0.4, "{mid:?}");
        // Its center line, and the outline of the ring.
        h.st.picture.click = ClickStitch::CenterLine;
        h.st.click_stitch(Point::new(40.0, 15.0));
        h.st.picture.click = ClickStitch::Outline;
        h.st.click_stitch(Point::new(4.2, 15.2));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 4);
        assert!(
            d.objects()[2..]
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::Running { .. }))
        );
        // All of them sew.
        let plan = emb_stitch::build_plan(d, &Default::default(), &mut Default::default());
        assert!(plan.stitches.records.len() > 200, "{}", plan.stitches.records.len());
        // A click off the picture does nothing.
        let steps = h.st.doc.undo_len();
        h.st.click_stitch(Point::new(-5.0, 5.0));
        assert_eq!(h.st.doc.undo_len(), steps);
        // A hidden backdrop takes no clicks.
        h.st.edit_backdrop("Show backdrop", None, |b| b.visible = false);
        let steps = h.st.doc.undo_len();
        h.st.click_stitch(Point::new(4.2, 15.2));
        h.st.trace_backdrop();
        assert_eq!(h.st.doc.undo_len(), steps);
        let s = crate::ribbon::SelInfo::of(&h.st);
        assert!(!h.st.can(crate::ribbon::Cmd::TracePicture, &s));
        h.st.edit_backdrop("Show backdrop", None, |b| b.visible = true);
        // The ribbon button sets the tool and what it makes.
        h.st.run(
            crate::ribbon::Cmd::ClickAs(ClickStitch::FillNoHoles),
            &eframe::egui::Context::default(),
        );
        assert_eq!(
            (h.st.tool, h.st.picture.click),
            (Tool::ClickStitch, ClickStitch::FillNoHoles)
        );
    }
}
