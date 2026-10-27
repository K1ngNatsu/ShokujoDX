//! The lettering kiosk (P10 Milestone 5): one full-window screen for a
//! shop, to make a name on a product in a few steps.
//!
//! 1. The product: its kind, its color and the place of the design (the
//!    product sets the fabric of the design, so the stitches suit it).
//! 2. The text, its size and its line (straight, arc over, arc under).
//! 3. The font (the embroidery fonts first).
//! 4. The thread color (matched to the thread chart).
//!
//! The kiosk works on a design of its own (a new design tab): the first
//! lettering object of the design is the text. A template from the
//! templates folder can be the start (its first lettering object is the
//! text then). The preview shows the design on the product with the 3D
//! thread look. The buttons save, output, send to USB or save a picture,
//! with the normal commands. Exit kiosk goes back to the normal window with
//! the design.

use eframe::egui::{self, Color32, RichText, Vec2};
use emb_model::product::{ProductKind, ProductView};
use emb_model::{Design, FontKind, FontRef, Justify, Lettering, MergeKey, ObjectKind, Rgb, ThreadLook};

use crate::app::AppState;
use crate::canvas::color32;
use crate::i18n::tr;
use crate::lettering::BaselineKind;
use crate::ribbon::Cmd;

/// The thread colors of the kiosk (matched to the thread chart).
pub const THREADS: [(&str, Rgb); 14] = [
    ("White", Rgb(250, 250, 250)),
    ("Black", Rgb(20, 20, 20)),
    ("Red", Rgb(200, 25, 40)),
    ("Navy", Rgb(25, 35, 90)),
    ("Royal blue", Rgb(30, 70, 180)),
    ("Light blue", Rgb(120, 180, 230)),
    ("Green", Rgb(30, 130, 60)),
    ("Yellow", Rgb(250, 215, 40)),
    ("Gold", Rgb(200, 150, 40)),
    ("Orange", Rgb(240, 120, 30)),
    ("Pink", Rgb(240, 130, 170)),
    ("Purple", Rgb(110, 50, 140)),
    ("Gray", Rgb(140, 140, 140)),
    ("Brown", Rgb(110, 70, 40)),
];

/// The product colors of the kiosk.
pub const PRODUCT_COLORS: [(&str, Rgb); 8] = [
    ("White", Rgb(245, 245, 242)),
    ("Black", Rgb(30, 30, 32)),
    ("Navy", Rgb(30, 40, 75)),
    ("Gray", Rgb(160, 160, 165)),
    ("Red", Rgb(180, 30, 40)),
    ("Pink", Rgb(240, 180, 200)),
    ("Light blue", Rgb(170, 205, 235)),
    ("Green", Rgb(40, 110, 70)),
];

/// The fabric (an id of `data/fabrics.toml`) of a product.
pub fn fabric_of(kind: ProductKind) -> &'static str {
    match kind {
        ProductKind::TShirt | ProductKind::TShirtBack => "knit",
        ProductKind::Polo => "pique",
        ProductKind::Sweatshirt => "fleece",
        ProductKind::Cap => "cap",
        ProductKind::Tote => "denim",
        ProductKind::Towel => "towel",
        ProductKind::Cushion => "cotton",
    }
}

/// The choices of the kiosk.
#[derive(Debug, Clone, PartialEq)]
pub struct KioskChoice {
    pub text: String,
    pub height_mm: f64,
    pub line: BaselineKind,
    pub font: FontRef,
    /// An index into [`THREADS`].
    pub thread: usize,
    pub product: ProductKind,
    /// An index into [`PRODUCT_COLORS`].
    pub product_color: usize,
    /// An index into the placements of the product.
    pub placement: usize,
}

/// The kiosk screen.
#[derive(Debug, Clone)]
pub struct Kiosk {
    pub choice: KioskChoice,
    /// The choice that the design has now.
    applied: Option<KioskChoice>,
    /// The templates of the templates folder (name, file).
    templates: Vec<(String, std::path::PathBuf)>,
}

impl AppState {
    /// Opens the kiosk with a new design.
    pub fn open_kiosk(&mut self) {
        self.new_builtin_design();
        let font = self.letter_font.clone();
        let templates = crate::styles::templates_dir()
            .map(|d| crate::styles::list_templates(&d))
            .unwrap_or_default();
        self.kiosk = Some(Kiosk {
            choice: KioskChoice {
                text: String::new(),
                height_mm: 20.0,
                line: BaselineKind::Straight,
                font,
                thread: 3,
                product: ProductKind::Towel,
                product_color: 0,
                placement: 0,
            },
            applied: None,
            templates,
        });
        self.selection.clear();
        self.show_thread_look = true;
        self.show_plan = true;
        self.message = tr("Lettering Kiosk").into();
    }

    /// Starts the kiosk design from a template.
    fn kiosk_template(&mut self, path: &std::path::Path) {
        self.new_from_template(Some(path));
        if let Some(k) = &mut self.kiosk {
            k.applied = None;
            if let Some(l) = self.doc.design().objects().iter().find_map(|o| match &o.kind {
                ObjectKind::Lettering(l) => Some(l),
                _ => None,
            }) {
                k.choice.text = l.text.clone();
                k.choice.font = l.font.clone();
                k.choice.height_mm = l.height_mm;
            }
        }
    }

    /// Makes the design of the kiosk choice, if it changed: the first
    /// lettering object, its thread, the product and the fabric. One undo
    /// step for all kiosk changes in a row.
    pub fn apply_kiosk(&mut self) {
        let Some(k) = &self.kiosk else { return };
        if k.applied.as_ref() == Some(&k.choice) {
            return;
        }
        let c = k.choice.clone();
        let fabric = fabric_of(c.product);
        // The lettering, with the stitch settings of the fabric.
        let font = self.fonts.get(&c.font).or_else(|| self.fonts.default_font());
        let lettering = match font {
            Some(font) if !c.text.trim().is_empty() => {
                let mut l = Lettering::new(&c.text, font.source().font_ref(), c.height_mm, emb_geom::Point::ZERO);
                l.justify = Justify::Center;
                emb_letter::relayout(&mut l, font.source());
                if c.line != BaselineKind::Straight {
                    crate::lettering::set_baseline(&mut l, c.line);
                    emb_letter::relayout(&mut l, font.source());
                }
                let mut tmp = Design::new("kiosk");
                let t = tmp.add_thread("Thread", Rgb(0, 0, 0));
                let id = tmp.add_object(t, ObjectKind::Lettering(Box::new(l)));
                tmp.set_fabric(&self.fabrics, Some(fabric), true);
                tmp.object(id).map(|o| o.kind.clone())
            }
            _ => None,
        };
        let (name, color) = THREADS[c.thread.min(THREADS.len() - 1)];
        let look = self
            .chart()
            .and_then(|ch| ch.nearest(color).map(|k| emb_model::colorway::chart_look(ch, k)))
            .unwrap_or(ThreadLook {
                name: name.into(),
                color,
                brand: None,
                code: None,
            });
        let product_color = PRODUCT_COLORS[c.product_color.min(PRODUCT_COLORS.len() - 1)].1;
        let fabrics = self.fabrics.clone();
        self.doc.edit_merged("Kiosk", MergeKey::new("kiosk"), |d| {
            let existing = d.objects().iter().find_map(|o| match o.kind {
                ObjectKind::Lettering(_) => Some((o.id, o.thread)),
                _ => None,
            });
            let thread = match (&lettering, existing) {
                (Some(kind), Some((id, t))) => {
                    if let Some(o) = d.object_mut(id) {
                        // Keep the place of a template's lettering.
                        let mut kind = kind.clone();
                        if let (ObjectKind::Lettering(new), ObjectKind::Lettering(old)) = (&mut kind, &o.kind) {
                            new.placement = old.placement;
                        }
                        o.kind = kind;
                    }
                    Some(t)
                }
                (Some(kind), None) => {
                    let t = d.add_thread(look.name.clone(), look.color);
                    let id = d.add_object(t, kind.clone());
                    let c = d.hoop.center;
                    d.center_objects_at(&[id], c);
                    Some(t)
                }
                (None, Some((id, _))) => {
                    d.remove_object(id);
                    None
                }
                (None, None) => None,
            };
            if let Some(t) = thread {
                let i = d.colorway;
                d.set_colorway_look(i, t, look.clone());
            }
            d.set_fabric(&fabrics, Some(fabric), false);
            d.background = Some(product_color);
            let mut v = ProductView::new(c.product, product_color);
            if let Some(p) = c.product.placements().get(c.placement) {
                v.put_design_at(d, p.at);
            }
            d.product = Some(v);
        });
        if let Some(k) = &mut self.kiosk {
            k.applied = Some(c);
        }
    }

    /// Leaves the kiosk; the design stays in its tab.
    pub fn close_kiosk(&mut self) {
        self.kiosk = None;
        self.doc.end_merge();
        self.fit_requested = true;
        self.focus_canvas = true;
    }
}

/// A large text for the kiosk.
fn big(text: &str, size: f32) -> RichText {
    RichText::new(text).size(size)
}

/// A step heading.
fn step(ui: &mut egui::Ui, n: u32, title: &str) {
    ui.add_space(10.0);
    ui.label(big(&format!("{n}  {title}"), 22.0).strong());
    ui.add_space(4.0);
}

/// A row of color swatches. Returns the index that was clicked.
fn swatches(ui: &mut egui::Ui, colors: &[(&str, Rgb)], on: usize) -> Option<usize> {
    let mut pick = None;
    ui.horizontal_wrapped(|ui| {
        for (i, (name, c)) in colors.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 6.0, color32(*c));
            let ring = if i == on {
                egui::Stroke::new(3.0, ui.visuals().selection.stroke.color)
            } else {
                egui::Stroke::new(1.0, ui.visuals().weak_text_color())
            };
            ui.painter().rect_stroke(rect, 6.0, ring, egui::StrokeKind::Outside);
            if resp.on_hover_text(tr(name)).clicked() {
                pick = Some(i);
            }
        }
    });
    pick
}

/// The design on the product, with the 3D thread look.
fn preview(ui: &mut egui::Ui, st: &mut AppState, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::from_gray(235));
    let design = st.doc.design();
    let Some(v) = design.product.clone() else { return };
    // The view: the place of the design on the product, and the design.
    let place = v
        .kind
        .placements()
        .get(st.kiosk.as_ref().map_or(0, |k| k.choice.placement))
        .map(|p| {
            let (w, h) = p.max;
            let c = p.at + v.origin;
            emb_geom::BBox::new(
                c - emb_geom::Point::new(w / 2.0, h / 2.0),
                c + emb_geom::Point::new(w / 2.0, h / 2.0),
            )
        });
    let b = [place, design.view_extents()]
        .into_iter()
        .flatten()
        .reduce(emb_geom::BBox::union)
        .unwrap_or_else(|| v.bbox())
        .expand(25.0);
    let saved = st.view;
    st.view.fit(rect, b);
    let ctx = ui.ctx().clone();
    crate::product::draw(&painter, &ctx, rect, st);
    let segs = crate::viewing::visible_segments(st);
    let table = emb_render::thread3d::ShadeTable::new(st.settings.look.light);
    painter.add(egui::Shape::mesh(crate::canvas::thread_mesh_3d(
        rect, &st.view, &segs, false, &table,
    )));
    if st.show_stones {
        crate::bling::draw(&painter, rect, st);
    }
    st.view = saved;
}

/// The kiosk screen (the whole window).
pub fn screen(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(mut k) = st.kiosk.clone() else { return };
    let ctx = ui.ctx().clone();
    let mut action: Option<Cmd> = None;
    let mut template: Option<std::path::PathBuf> = None;
    let mut exit = false;
    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
    ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
    ui.horizontal(|ui| {
        ui.label(big(tr("Lettering Kiosk"), 28.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(big(tr("Exit kiosk"), 18.0)).clicked() {
                exit = true;
            }
        });
    });
    ui.separator();
    let left_w = 460.0f32.min(ui.available_width() * 0.45);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(left_w);
            egui::ScrollArea::vertical().id_salt("kiosk_steps").show(ui, |ui| {
                let c = &mut k.choice;
                step(ui, 1, tr("Product"));
                ui.horizontal_wrapped(|ui| {
                    for kind in ProductKind::ALL {
                        if ui
                            .selectable_label(c.product == kind, big(tr(kind.name()), 17.0))
                            .clicked()
                        {
                            c.product = kind;
                            c.placement = 0;
                        }
                    }
                });
                if let Some(i) = swatches(ui, &PRODUCT_COLORS, c.product_color) {
                    c.product_color = i;
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, p) in c.product.placements().iter().enumerate() {
                        if ui.selectable_label(c.placement == i, big(p.name, 16.0)).clicked() {
                            c.placement = i;
                        }
                    }
                });
                if !k.templates.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(big(tr("Or start from a template:"), 16.0));
                        egui::ComboBox::from_id_salt("kiosk_template")
                            .selected_text(tr("Templates"))
                            .show_ui(ui, |ui| {
                                for (name, p) in &k.templates {
                                    if ui.selectable_label(false, name).clicked() {
                                        template = Some(p.clone());
                                    }
                                }
                            });
                    });
                }
                step(ui, 2, tr("Text"));
                ui.add(
                    egui::TextEdit::singleline(&mut c.text)
                        .font(egui::FontId::proportional(26.0))
                        .hint_text(tr("Type the name"))
                        .desired_width(left_w - 20.0),
                );
                ui.horizontal(|ui| {
                    ui.label(big(tr("Size"), 17.0));
                    ui.add(
                        egui::Slider::new(&mut c.height_mm, 6.0..=80.0)
                            .clamping(egui::SliderClamping::Edits)
                            .suffix(" mm")
                            .fixed_decimals(0),
                    );
                });
                ui.horizontal(|ui| {
                    for (line, text) in [
                        (BaselineKind::Straight, tr("Straight")),
                        (BaselineKind::ArcOver, tr("Arc over")),
                        (BaselineKind::ArcUnder, tr("Arc under")),
                    ] {
                        if ui.selectable_label(c.line == line, big(text, 17.0)).clicked() {
                            c.line = line;
                        }
                    }
                });
                step(ui, 3, tr("Font"));
                let families = st.fonts.families();
                ui.horizontal_wrapped(|ui| {
                    for (family, kind) in families.iter().filter(|(_, k)| *k == FontKind::Embroidery) {
                        let on = c.font.kind == *kind && c.font.family.eq_ignore_ascii_case(family);
                        if ui.selectable_label(on, big(family, 17.0)).clicked() {
                            let styles = st.fonts.styles(family, *kind);
                            c.font = FontRef {
                                family: family.clone(),
                                style: styles.first().cloned().unwrap_or_default(),
                                kind: *kind,
                            };
                        }
                    }
                });
                step(ui, 4, tr("Thread color"));
                if let Some(i) = swatches(ui, &THREADS, c.thread) {
                    c.thread = i;
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            let h = (ui.available_height() - 90.0).max(200.0);
            let size = Vec2::new(ui.available_width(), h);
            preview(ui, st, size);
            let d = st.doc.design();
            let info = match d.view_extents() {
                Some(b) => {
                    let pv = st.shown_plan();
                    format!(
                        "{} × {} mm · {} stitches",
                        b.width().round(),
                        b.height().round(),
                        pv.plan.stitches.stitch_count()
                    )
                }
                None => "Type a name to start.".into(),
            };
            ui.label(big(&info, 16.0));
            // A design larger than the place on the product.
            let d = st.doc.design();
            if let (Some(v), Some(b)) = (&d.product, d.view_extents())
                && let Some(p) = v.kind.placements().get(k.choice.placement)
                && (b.width() > p.max.0 || b.height() > p.max.1)
            {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    big(
                        &format!(
                            "The name is larger than the place ({} × {} mm). Make it smaller.",
                            p.max.0, p.max.1
                        ),
                        16.0,
                    ),
                );
            }
            ui.horizontal_wrapped(|ui| {
                for (cmd, text) in [
                    (Cmd::SaveAs, tr("Save…")),
                    (Cmd::OutputDesign, tr("Output Design…")),
                    (Cmd::SendToUsb, tr("Send to USB")),
                    (Cmd::SaveDecoration, tr("Save picture…")),
                ] {
                    if ui.button(big(text, 18.0)).clicked() {
                        action = Some(cmd);
                    }
                }
                if ui.button(big(tr("New name"), 18.0)).clicked() {
                    k.choice.text.clear();
                }
            });
        });
    });
    if let Some(kk) = &mut st.kiosk {
        kk.choice = k.choice;
    }
    if let Some(p) = template {
        st.kiosk_template(&p);
    }
    st.apply_kiosk();
    if let Some(cmd) = action {
        st.doc.end_merge();
        st.run(cmd, &ctx);
    }
    if exit {
        st.close_kiosk();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_name_on_a_towel() {
        let mut h = Harness::empty();
        h.st.open_kiosk();
        let k = h.st.kiosk.as_mut().unwrap();
        k.choice.text = "Anna".into();
        k.choice.height_mm = 30.0;
        k.choice.thread = 2;
        h.st.apply_kiosk();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let o = &d.objects()[0];
        let ObjectKind::Lettering(l) = &o.kind else { panic!() };
        assert_eq!(l.text, "Anna");
        assert!(!l.letters.is_empty());
        assert_eq!(d.fabric.as_deref(), Some("towel"));
        assert_eq!(d.product.as_ref().unwrap().kind, ProductKind::Towel);
        // The design is on the first place of the towel.
        assert_eq!(
            d.product.as_ref().unwrap().placement_of(d).map(|p| p.name),
            Some("On the border")
        );
        // The thread is near red.
        let t = d.thread(o.thread).unwrap();
        assert!(t.color.0 > 150 && t.color.1 < 100, "{:?}", t.color);
        // A change of the text changes the same object, in the same undo step.
        let steps = h.st.doc.undo_len();
        h.st.kiosk.as_mut().unwrap().choice.text = "Anna Lee".into();
        h.st.kiosk.as_mut().unwrap().choice.line = BaselineKind::ArcOver;
        h.st.apply_kiosk();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        assert_eq!(h.st.doc.undo_len(), steps);
        let ObjectKind::Lettering(l) = &d.objects()[0].kind else {
            panic!()
        };
        assert!(!l.baseline.is_straight());
        // No text: no object.
        h.st.kiosk.as_mut().unwrap().choice.text.clear();
        h.st.apply_kiosk();
        assert!(h.st.doc.design().objects().is_empty());
        h.st.close_kiosk();
        assert!(h.st.kiosk.is_none());
    }

    #[test]
    fn the_screen_draws() {
        let mut h = Harness::empty();
        h.st.open_kiosk();
        h.st.kiosk.as_mut().unwrap().choice.text = "Joshua".into();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| screen(ui, &mut h.st));
            });
            out.textures_delta.clear();
        }
        assert_eq!(h.st.doc.design().objects().len(), 1);
    }
}
