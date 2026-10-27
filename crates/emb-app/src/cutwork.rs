//! The Cutwork window (P11 Milestone 8, ADR 0051): the selected lines
//! become cut lines for the four cutting needles of a kit. The engine work
//! is in `emb_model::cutwork`.

use eframe::egui;
use emb_model::ObjectKind;
use emb_model::cutwork::{CutBrand, DEFAULT_PENETRATION_MM, needle_name};

use crate::app::AppState;
use crate::i18n::tr;

/// The window state.
#[derive(Debug, Clone)]
pub struct CutworkWindow {
    pub open: bool,
    /// The kit; `None`: from the brand of the machine (else Bernina).
    pub brand: Option<CutBrand>,
    pub penetration_mm: f64,
}

impl Default for CutworkWindow {
    fn default() -> Self {
        CutworkWindow {
            open: false,
            brand: None,
            penetration_mm: DEFAULT_PENETRATION_MM,
        }
    }
}

impl AppState {
    /// The cutwork kit: the one set in the window, or the one of the brand
    /// of the machine of the design, or Bernina.
    pub fn cutwork_brand(&self) -> CutBrand {
        self.cutwork
            .brand
            .or_else(|| {
                let m = self.doc.design().machine.as_deref()?;
                let brand = self.machines.get(m)?.brand.clone()?;
                CutBrand::of_machine(&brand)
            })
            .unwrap_or(CutBrand::Bernina)
    }

    /// Makes cut lines of the selected lines. One undo step.
    pub fn make_cutwork(&mut self) -> usize {
        let d = self.doc.design();
        let ids: Vec<_> = self
            .selection
            .in_sew_order(d)
            .into_iter()
            .filter(|id| matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Running { .. })))
            .collect();
        if ids.is_empty() {
            self.message = tr("Select the lines to cut.").into();
            return 0;
        }
        let (brand, pen) = (self.cutwork_brand(), self.cutwork.penetration_mm);
        let new = self.doc.edit("Cutwork", |d| d.make_cutwork(&ids, brand, pen));
        self.message = crate::i18n::fill("Cut lines for {} needles.", &[&new.len()]);
        let n = new.len();
        self.selection.set_all(new);
        n
    }
}

/// The Cutwork window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.cutwork.open {
        return;
    }
    let mut open = true;
    let mut run = false;
    let brand = st.cutwork_brand();
    let has = !st.selection.is_empty();
    crate::dock::docked(egui::Window::new(tr("Cutwork"))
        .id(egui::Id::new("Cutwork")), ctx, egui::Id::new("Cutwork"), st.canvas_rect)
        .open(&mut open)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.weak(tr(
                "The selected lines become cut lines for the cutting needles of a cutwork kit. Each part of a line goes to the needle whose blade has its direction; each needle is a color, so the machine stops to change the needle.",
            ));
            let w = &mut st.cutwork;
            let mut pick = brand;
            ui.horizontal(|ui| {
                ui.label(tr("Kit"));
                egui::ComboBox::from_id_salt("cut_brand")
                    .selected_text(pick.name())
                    .show_ui(ui, |ui| {
                        for b in CutBrand::ALL {
                            ui.selectable_value(&mut pick, b, b.name());
                        }
                    });
            });
            if pick != brand {
                w.brand = Some(pick);
            }
            for (i, n) in pick.needles().iter().enumerate() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, crate::canvas::color32(n.color));
                    ui.label(needle_name(i, n));
                });
            }
            crate::artwork::number(
                ui,
                tr("Cut spacing"),
                tr("The distance between two cuts of the needle along the line."),
                &mut w.penetration_mm,
                0.3..=3.0,
            );
            ui.weak(tr(
                "Not tried on a machine yet: how each machine finds the cutting needles in its file is not public. Test the file on scrap fabric first.",
            ));
            run = ui
                .add_enabled(has, egui::Button::new(tr("Make cut lines")))
                .clicked();
        });
    st.cutwork.open = open;
    if run {
        st.make_cutwork();
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{Rgb, RunningParams};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_kit_follows_the_machine_and_the_lines_become_cut_lines() {
        let mut h = Harness::empty();
        let circle = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Black", Rgb(0, 0, 0));
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::ellipse(Point::ZERO, 15.0, 15.0),
                    params: RunningParams::default(),
                },
            )
        });
        // The Topaz 20 is a Husqvarna Viking.
        h.st.doc
            .edit("Machine", |d| d.machine = Some("husqvarna_topaz20".into()));
        assert_eq!(h.st.cutwork_brand(), CutBrand::Husqvarna);
        h.st.cutwork.brand = Some(CutBrand::Brother);
        assert_eq!(h.st.cutwork_brand(), CutBrand::Brother);
        h.st.selection.set(circle);
        assert_eq!(h.st.make_cutwork(), 4);
        assert_eq!(h.st.doc.design().objects().len(), 4);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 1);
    }
}
