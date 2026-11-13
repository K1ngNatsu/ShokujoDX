//! The Buttonholes window (P5 Milestone 7; shapes, sizes and changes in
//! ADR 0085): one buttonhole or eyelet, or a string along the selected
//! line, in a shape and in the size of a standard button. With buttonholes
//! selected, the window changes them in their places.

use eframe::egui;
use emb_geom::Point;
use emb_model::buttonhole::{
    BUTTON_THICKNESS_MM, EYELET_MM, INCH_BUTTONS, INCH_EYELETS, METRIC_BUTTONS, METRIC_EYELETS, OPENING_MM,
    StandardSize, buttonhole, placement_of_box, string_every, string_positions,
};
use emb_model::{ButtonholeRecord, ButtonholeShape, ButtonholeSpec, Design, ObjectId, ObjectKind, ThreadId};

use crate::app::AppState;
use crate::i18n::{fill, tr};
use crate::workspace::{Units, length_value};

/// The settings of the Buttonholes window.
#[derive(Debug, Clone)]
pub struct ButtonholeWindow {
    pub spec: ButtonholeSpec,
    /// The diameter of the button of the last standard size, so that a new
    /// thickness changes the opening.
    pub button_mm: Option<f64>,
    /// The thickness of the button: the opening is its diameter and its
    /// thickness.
    pub thickness_mm: f64,
    /// The legs, the gap, the bar tacks and the eye follow the size.
    pub scale: bool,
    pub count: usize,
    /// A string is spaced by a distance, else by the number.
    pub by_distance: bool,
    pub distance_mm: f64,
    pub across: bool,
    /// One buttonhole is upright, else level.
    pub upright: bool,
}

impl Default for ButtonholeWindow {
    fn default() -> Self {
        ButtonholeWindow {
            spec: ButtonholeSpec::default(),
            button_mm: None,
            thickness_mm: BUTTON_THICKNESS_MM,
            scale: true,
            count: 5,
            by_distance: false,
            distance_mm: 80.0,
            across: true,
            upright: true,
        }
    }
}

/// Adds buttonholes of `spec` at `places` to `d`, in the group `g`, at
/// `index` in the sew order (else last), and keeps their record. Returns
/// the new objects.
fn place(
    d: &mut Design,
    spec: &ButtonholeSpec,
    places: &[(Point, f64)],
    thread: ThreadId,
    g: u32,
    mut index: Option<usize>,
) -> Vec<ObjectId> {
    let name = if spec.shape == ButtonholeShape::Eyelet {
        "Eyelet"
    } else {
        "Buttonhole"
    };
    let (mut ids, mut boxes) = (Vec::new(), Vec::new());
    for &(c, a) in places {
        for (k, kind) in buttonhole(spec, c, a).into_iter().enumerate() {
            let id = match index {
                Some(i) => {
                    index = Some(i + 1);
                    d.insert_object(i, thread, kind)
                }
                None => d.add_object(thread, kind),
            };
            if let Some(o) = d.object_mut(id) {
                o.group = Some(g);
                o.name = Some(name.into());
            }
            if k == 0 {
                boxes.push(id);
            }
            ids.push(id);
        }
    }
    d.buttonholes.retain(|r| r.group != g);
    d.buttonholes.push(ButtonholeRecord {
        group: g,
        spec: spec.clone(),
        boxes,
    });
    ids
}

impl AppState {
    /// The record of the buttonholes that the selection is in.
    pub fn selected_buttonholes(&self) -> Option<ButtonholeRecord> {
        let d = self.doc.design();
        self.selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id)?.group)
            .find_map(|g| d.buttonholes.iter().find(|r| r.group == g))
            .cloned()
    }

    /// Opens the Buttonholes window, with the settings of the selected
    /// buttonholes if there are some.
    pub fn open_buttonholes(&mut self) {
        if let Some(r) = self.selected_buttonholes() {
            let w = &mut self.decor.buttonhole;
            w.spec = r.spec;
            w.button_mm = None;
        }
        self.decor.buttonholes = true;
    }

    /// Adds buttonholes: a string along the selected line, or one in the
    /// middle of the hoop. One undo step; the buttonholes are one group.
    pub fn add_buttonholes(&mut self) -> usize {
        let w = self.decor.buttonhole.clone();
        let d = self.doc.design();
        let line = self
            .selection
            .single()
            .and_then(|id| d.object(id))
            .and_then(|o| match &o.kind {
                ObjectKind::Running { path, .. } => {
                    Some((o.id, o.thread, path.flatten(0.1).into_iter().next()?.points))
                }
                _ => None,
            });
        let (places, thread, used) = match line {
            Some((id, t, pts)) => {
                let places = if w.by_distance {
                    string_every(&pts, w.distance_mm, w.across)
                } else {
                    string_positions(&pts, w.count.max(1), w.across)
                };
                (places, Some(t), Some(id))
            }
            None => {
                let angle = if w.upright { 90.0 } else { 0.0 };
                (vec![(d.hoop.center, angle)], d.threads.first().map(|t| t.id), None)
            }
        };
        let n = places.len();
        if n == 0 {
            return 0;
        }
        let ids = self.doc.edit(tr("Buttonholes"), |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Thread", emb_model::Rgb(30, 30, 30)));
            let g = d.new_group_id();
            if let Some(id) = used {
                d.remove_object(id);
            }
            place(d, &w.spec, &places, t, g, None)
        });
        self.selection.set_all(ids);
        self.message = if w.spec.shape == ButtonholeShape::Eyelet {
            fill("{} eyelets added. Cut or punch each hole after sewing.", &[&n])
        } else {
            fill(
                "{} buttonholes added. Cut each opening between the legs after sewing.",
                &[&n],
            )
        };
        n
    }

    /// Makes the selected buttonholes again with the settings of the
    /// window, each in its place (also after a move, a turn or a mirror).
    /// One undo step. Returns the number of buttonholes.
    pub fn change_buttonholes(&mut self) -> usize {
        let Some(r) = self.selected_buttonholes() else {
            return 0;
        };
        let spec = self.decor.buttonhole.spec.clone();
        let d = self.doc.design();
        let boxes: Vec<_> = r.boxes.iter().filter_map(|id| d.object(*id)).collect();
        let places: Vec<(Point, f64)> = boxes
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Running { path, .. } => placement_of_box(&r.spec, path),
                _ => None,
            })
            .collect();
        let (Some(thread), false) = (boxes.first().map(|o| o.thread), places.is_empty()) else {
            self.message =
                tr("The boxes of these buttonholes are gone, so they cannot change. Place them again.").into();
            return 0;
        };
        let index = d.objects().iter().position(|o| o.group == Some(r.group));
        let ids = self.doc.edit(tr("Change buttonholes"), |d| {
            let old: Vec<ObjectId> = d
                .objects()
                .iter()
                .filter(|o| o.group == Some(r.group))
                .map(|o| o.id)
                .collect();
            for id in old {
                d.remove_object(id);
            }
            place(d, &spec, &places, thread, r.group, index)
        });
        self.selection.set_all(ids);
        self.message = fill("{} buttonholes changed.", &[&places.len()]);
        places.len()
    }
}

/// The standard sizes of the shape, in the units of the program first.
fn sizes(shape: ButtonholeShape, units: Units) -> [(&'static str, &'static [StandardSize]); 2] {
    let (metric, inch): (&[StandardSize], &[StandardSize]) = if shape == ButtonholeShape::Eyelet {
        (&METRIC_EYELETS, &INCH_EYELETS)
    } else {
        (&METRIC_BUTTONS, &INCH_BUTTONS)
    };
    match units {
        Units::Inch => [("Inches", inch), ("Millimeters", metric)],
        Units::Mm => [("Millimeters", metric), ("Inches", inch)],
    }
}

/// A slider of a stitch setting in mm.
fn mm_slider(ui: &mut egui::Ui, label: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::Slider::new(v, range)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" mm")
                .fixed_decimals(1),
        )
        .changed()
    })
    .inner
}

/// The size, the shape and the stitches of the window. Returns true when
/// the size or the shape changed (then a scaled spec follows).
fn spec_ui(ui: &mut egui::Ui, w: &mut ButtonholeWindow, units: Units) -> bool {
    let mut resize = false;
    let s = &mut w.spec;
    ui.horizontal(|ui| {
        ui.label(tr("Shape"));
        egui::ComboBox::from_id_salt("buttonhole_shape")
            .selected_text(tr(s.shape.name()))
            .width(170.0)
            .show_ui(ui, |ui| {
                for shape in ButtonholeShape::ALL {
                    resize |= ui
                        .selectable_value(&mut s.shape, shape, tr(shape.name()))
                        .on_hover_text(tr(shape.help()))
                        .changed();
                }
            });
    });
    ui.weak(tr(s.shape.help()));
    let eyelet = s.shape == ButtonholeShape::Eyelet;
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.label(if eyelet { tr("Hole") } else { tr("Button") });
        egui::ComboBox::from_id_salt("buttonhole_size")
            .selected_text(tr("Standard sizes"))
            .width(170.0)
            .show_ui(ui, |ui| {
                for (title, list) in sizes(s.shape, units) {
                    ui.label(egui::RichText::new(tr(title)).strong());
                    for size in list {
                        let text = if eyelet { size.name.to_string() } else { size.label() };
                        if ui.selectable_label(false, text).clicked() {
                            picked = Some(size.mm);
                        }
                    }
                    ui.separator();
                }
            })
            .response
            .on_hover_text(if eyelet {
                tr("The diameter of the hole.")
            } else {
                tr("The diameter of the button. The size in lignes (L) is in brackets.")
            });
    });
    if eyelet {
        if let Some(mm) = picked {
            s.eye_mm = mm;
            resize = true;
        }
        ui.horizontal(|ui| {
            ui.label(tr("Hole"));
            resize |= length_value(ui, &mut s.eye_mm, units, EYELET_MM.0, EYELET_MM.1).changed();
        });
    } else {
        if let Some(mm) = picked {
            w.button_mm = Some(mm);
        }
        ui.horizontal(|ui| {
            ui.label(tr("Button thickness"));
            let changed = length_value(ui, &mut w.thickness_mm, units, 0.0, 15.0)
                .on_hover_text(tr("A domed button or a button with a shank needs a longer opening."))
                .changed();
            if (changed || picked.is_some())
                && let Some(b) = w.button_mm
            {
                s.length_mm = ButtonholeSpec::opening_for(b, w.thickness_mm);
                resize = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Opening"));
            let r = length_value(ui, &mut s.length_mm, units, OPENING_MM.0, OPENING_MM.1).on_hover_text(tr(
                "The length of the cut: the diameter of the button and its thickness.",
            ));
            if r.changed() {
                w.button_mm = None;
                resize = true;
            }
        });
    }
    ui.checkbox(&mut w.scale, tr("Scale the stitches with the size"))
        .on_hover_text(tr("The legs, the gap, the bar tacks and the eye follow the size."));
    egui::CollapsingHeader::new(tr("Stitches"))
        .id_salt("buttonhole_stitches")
        .show(ui, |ui| {
            let s = &mut w.spec;
            mm_slider(
                ui,
                if eyelet { tr("Satin width") } else { tr("Leg width") },
                &mut s.leg_mm,
                0.8..=4.0,
            );
            if !eyelet {
                mm_slider(ui, tr("Gap for the cut"), &mut s.gap_mm, 0.2..=2.0);
            }
            if !matches!(s.shape, ButtonholeShape::RoundEnds | ButtonholeShape::Eyelet) {
                mm_slider(ui, tr("Bar tack"), &mut s.bartack_mm, 0.5..=4.0);
            }
            if matches!(s.shape, ButtonholeShape::Keyhole | ButtonholeShape::TaperedKeyhole) {
                mm_slider(ui, tr("Eye"), &mut s.eye_mm, 1.5..=8.0);
            }
            mm_slider(ui, tr("Density"), &mut s.spacing_mm, 0.2..=0.6);
        });
    let s = &mut w.spec;
    ui.checkbox(&mut s.corded, tr("Corded")).on_hover_text(tr(
        "A run under each leg, as a cord: the buttonhole is stronger and stands up more.",
    ));
    if s.shape.can_mitre() {
        ui.checkbox(&mut s.mitre, tr("Mitre corners")).on_hover_text(tr(
            "The legs and the bar tacks sew as one column that turns at the corners. Off: bar tacks, which hold the ends of the cut better.",
        ));
    }
    if s.shape.has_ends() {
        ui.checkbox(&mut s.flip, tr("Turn end for end"))
            .on_hover_text(tr("Put the round end, the eye or the taper at the other end."));
    }
    resize
}

/// A small picture of the buttonhole of `spec`.
fn preview(ui: &mut egui::Ui, spec: &ButtonholeSpec) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(300.0), 64.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let objs = buttonhole(spec, Point::new(0.0, 0.0), 0.0);
    let color = ui.visuals().strong_text_color();
    let rgb = emb_model::Rgb(color.r(), color.g(), color.b());
    let list: Vec<(emb_model::Rgb, &ObjectKind)> = objs.iter().map(|k| (rgb, k)).collect();
    crate::monogram::draw_preview(ui.painter(), rect, &list);
}

pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.decor.buttonholes;
    let (mut add, mut change) = (false, false);
    let on_line = st
        .selection
        .single()
        .and_then(|id| st.doc.design().object(id))
        .is_some_and(|o| matches!(o.kind, ObjectKind::Running { .. }));
    let selected = st.selected_buttonholes().is_some();
    let units = st.settings.units;
    crate::dock::docked(
        egui::Window::new(tr("Buttonholes")).id(egui::Id::new("Buttonholes")),
        ctx,
        egui::Id::new("Buttonholes"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(320.0)
    .show(ctx, |ui| {
        let w = &mut st.decor.buttonhole;
        if spec_ui(ui, w, units) && w.scale {
            w.spec = w.spec.scaled();
        }
        preview(ui, &w.spec);
        ui.separator();
        let eyelet = w.spec.shape == ButtonholeShape::Eyelet;
        if on_line {
            ui.label(tr(
                "A string of buttonholes along the selected line (the line is used up).",
            ));
            ui.horizontal(|ui| {
                ui.radio_value(&mut w.by_distance, false, tr("Number"));
                ui.add_enabled(
                    !w.by_distance,
                    egui::Slider::new(&mut w.count, 1..=40).clamping(egui::SliderClamping::Edits),
                );
            });
            ui.horizontal(|ui| {
                ui.radio_value(&mut w.by_distance, true, tr("Distance apart"));
                ui.add_enabled_ui(w.by_distance, |ui| {
                    length_value(ui, &mut w.distance_mm, units, 5.0, 500.0)
                });
            });
            if !eyelet {
                ui.checkbox(&mut w.across, tr("Across the line (else along it)"));
            }
        } else {
            if !eyelet {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut w.upright, true, tr("Upright"));
                    ui.radio_value(&mut w.upright, false, tr("Level"));
                });
            }
            ui.weak(tr("Select a line to place a string of buttonholes along it."));
        }
        let text = match (on_line, eyelet) {
            (true, false) => tr("Place buttonholes"),
            (false, false) => tr("Place one buttonhole"),
            (true, true) => tr("Place eyelets"),
            (false, true) => tr("Place one eyelet"),
        };
        ui.horizontal(|ui| {
            add = ui.button(text).clicked();
            if selected {
                change = ui
                    .button(tr("Change the selected buttonholes"))
                    .on_hover_text(tr(
                        "Make the selected buttonholes again with these settings, each in its place.",
                    ))
                    .clicked();
            }
        });
    });
    st.decor.buttonholes = open;
    if add {
        st.add_buttonholes();
    } else if change {
        st.change_buttonholes();
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Transform};

    use super::*;
    use crate::test_util::Harness;

    fn draw(h: &mut Harness) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| window(ui.ctx(), st));
        out.textures_delta.clear();
    }

    fn line(h: &mut Harness) -> ObjectId {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Black", emb_model::Rgb(0, 0, 0));
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(0.0, -40.0))
                        .line_to(Point::new(0.0, 40.0)),
                    params: Default::default(),
                },
            )
        })
    }

    #[test]
    fn a_string_of_buttonholes_along_a_line() {
        let mut h = Harness::empty();
        let id = line(&mut h);
        h.st.selection.set(id);
        h.st.decor.buttonhole.count = 4;
        assert_eq!(h.st.add_buttonholes(), 4);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 4 * 5, "the line is used up");
        assert!(d.objects().iter().all(|o| o.group == d.objects()[0].group));
        assert_eq!(d.buttonholes.len(), 1);
        assert_eq!(d.buttonholes[0].boxes.len(), 4);
        h.st.decor.buttonholes = true;
        draw(&mut h);
    }

    #[test]
    fn a_string_by_distance() {
        let mut h = Harness::empty();
        let id = line(&mut h);
        h.st.selection.set(id);
        let w = &mut h.st.decor.buttonhole;
        w.by_distance = true;
        w.distance_mm = 25.4 * 3.0;
        w.spec = ButtonholeSpec {
            shape: ButtonholeShape::Eyelet,
            ..ButtonholeSpec::default()
        }
        .scaled();
        assert_eq!(h.st.add_buttonholes(), 2, "80 mm of line, 3 in apart");
        assert!(h.st.message.contains("eyelets"), "{}", h.st.message);
    }

    #[test]
    fn the_selected_buttonholes_change_in_their_places() {
        let mut h = Harness::empty();
        let id = line(&mut h);
        h.st.selection.set(id);
        h.st.decor.buttonhole.count = 3;
        h.st.add_buttonholes();
        // Move and turn the buttonholes, then make them keyholes.
        let ids: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        let t = Transform::rotate_deg(30.0).then(Transform::translate(15.0, 5.0));
        h.st.doc.edit("Move", |d| d.transform_objects(&ids, &t));
        let before = h.st.doc.design().buttonholes[0].clone();
        let places = |st: &AppState| -> Vec<(Point, f64)> {
            let d = st.doc.design();
            let r = &d.buttonholes[0];
            r.boxes
                .iter()
                .map(|id| match &d.object(*id).unwrap().kind {
                    ObjectKind::Running { path, .. } => placement_of_box(&r.spec, path).unwrap(),
                    _ => panic!(),
                })
                .collect()
        };
        let moved = places(&h.st);
        h.st.selection.set(ids[3]);
        h.st.open_buttonholes();
        assert_eq!(h.st.decor.buttonhole.spec, before.spec);
        h.st.decor.buttonhole.spec.shape = ButtonholeShape::Keyhole;
        h.st.decor.buttonhole.spec.corded = true;
        assert_eq!(h.st.change_buttonholes(), 3);
        let d = h.st.doc.design();
        assert_eq!(d.buttonholes.len(), 1);
        assert_eq!(d.buttonholes[0].spec.shape, ButtonholeShape::Keyhole);
        // Box, cord, satin and bar tack for each.
        assert_eq!(d.objects().len(), 3 * 4);
        for ((p0, a0), (p1, a1)) in moved.iter().zip(places(&h.st)) {
            assert!(
                p0.distance(p1) < 1e-6 && (a0 - a1).abs() < 1e-6,
                "{p0:?} {a0} {p1:?} {a1}"
            );
        }
        assert_eq!(h.st.doc.undo_label(), Some("Change buttonholes"));
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().buttonholes[0], before);
        // Mitre corners: the box and one ring for each.
        h.st.selection.set(ids[3]);
        h.st.open_buttonholes();
        h.st.decor.buttonhole.spec.mitre = true;
        assert_eq!(h.st.change_buttonholes(), 3);
        assert_eq!(h.st.doc.design().objects().len(), 3 * 2);
        h.st.decor.buttonholes = true;
        h.st.selection.set(ids[0]);
        draw(&mut h);
    }

    #[test]
    fn a_standard_size_sets_the_opening() {
        let mut h = Harness::empty();
        let w = &mut h.st.decor.buttonhole;
        w.button_mm = Some(INCH_BUTTONS[6].mm);
        w.thickness_mm = 3.0;
        w.spec.length_mm = ButtonholeSpec::opening_for(INCH_BUTTONS[6].mm, w.thickness_mm);
        w.spec = w.spec.scaled();
        assert!((w.spec.length_mm - (15.875 + 3.0)).abs() < 1e-9);
        h.st.decor.buttonholes = true;
        for shape in ButtonholeShape::ALL {
            h.st.decor.buttonhole.spec.shape = shape;
            draw(&mut h);
        }
    }
}
