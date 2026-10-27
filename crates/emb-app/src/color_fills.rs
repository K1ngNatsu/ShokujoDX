//! The Tartan and Gradient window (P11 Milestone 4, ADR 0047): make a
//! tartan or a linear gradient of several colors from the selected fill.
//!
//! An object has one thread, so a tartan or a gradient is a group of
//! fills with the same shape, one for each color; each fill sews the rows
//! of its color (`FillPattern::Tartan`, `FillPattern::Gradient`).

use eframe::egui;
use emb_model::tartan::{Sett, code_color, gradient_layers, tartan_layers};
use emb_model::{FillParams, FillPattern, ObjectId, ObjectKind, ThreadId};

use crate::app::AppState;
use crate::i18n::tr;

/// Setts of our own for a start.
pub const SETTS: &[(&str, &str, bool)] = &[
    ("Red and black check", "R16 K16", false),
    ("Blue gingham", "W8 B8", false),
    ("Green and navy", "G16 K2 DB16 K2", true),
    ("Highland", "K4 R24 K4 G16 Y2 G16", true),
    ("Autumn", "T12 GD2 T12 DR6 LT4", true),
];

/// The window state.
#[derive(Debug, Clone)]
pub struct ColorFillsWindow {
    pub open: bool,
    /// The gradient part (else the tartan part).
    pub gradient: bool,
    pub sett_text: String,
    /// The width of one count, in mm.
    pub count_mm: f64,
    pub mirror: bool,
    /// The colors of a gradient: the thread and the place in percent.
    pub colors: Vec<(Option<ThreadId>, f64)>,
}

impl Default for ColorFillsWindow {
    fn default() -> Self {
        ColorFillsWindow {
            open: false,
            gradient: false,
            sett_text: SETTS[3].1.into(),
            count_mm: 0.5,
            mirror: true,
            colors: vec![(None, 0.0), (None, 100.0)],
        }
    }
}

/// The thread of a layer: a thread of the design, or a new thread of a
/// color (unless the design has a thread of that color).
#[derive(Debug, Clone)]
enum Pick {
    Thread(ThreadId),
    Color(String, emb_model::Rgb),
}

/// The kind of color fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tartan,
    Gradient,
}

fn kind_of(p: &FillPattern) -> Option<Kind> {
    match p {
        FillPattern::Tartan { .. } => Some(Kind::Tartan),
        FillPattern::Gradient { .. } => Some(Kind::Gradient),
        _ => None,
    }
}

impl AppState {
    /// The selected fill, if the selection is one fill.
    fn selected_fill(&self) -> Option<(ObjectId, FillParams)> {
        let d = self.doc.design();
        let o = d.object(self.selection.single()?)?;
        match &o.kind {
            ObjectKind::Fill { params, .. } => Some((o.id, params.clone())),
            _ => None,
        }
    }

    /// The other fills of the group of `id` that are layers of the same kind
    /// of color fill.
    fn color_fill_layers(&self, id: ObjectId) -> Vec<ObjectId> {
        let d = self.doc.design();
        let Some(o) = d.object(id) else { return Vec::new() };
        let (Some(g), ObjectKind::Fill { params, .. }) = (o.group, &o.kind) else {
            return Vec::new();
        };
        let Some(kind) = kind_of(&params.pattern) else {
            return Vec::new();
        };
        d.objects()
            .iter()
            .filter(|x| x.id != id && x.group == Some(g))
            .filter(|x| matches!(&x.kind, ObjectKind::Fill { params, .. } if kind_of(&params.pattern) == Some(kind)))
            .map(|x| x.id)
            .collect()
    }

    /// Makes the selected fill a color fill of the layers `layers` (thread
    /// and settings, in sew order). The old layers of a color fill of the
    /// same kind go. The layers are one group. One undo step.
    fn make_color_fill(&mut self, label: &str, layers: Vec<(Pick, FillParams)>) -> bool {
        let Some((id, _)) = self.selected_fill() else {
            self.message = tr("Select one fill.").into();
            return false;
        };
        if layers.is_empty() {
            return false;
        }
        let old = self.color_fill_layers(id);
        let Some(o) = self.doc.design().object(id).cloned() else {
            return false;
        };
        let ids = self.doc.edit(label, |d| {
            for x in &old {
                d.remove_object(*x);
            }
            let g = d.new_group_id();
            let mut ids = vec![id];
            let at = d.index_of(id).unwrap_or(d.objects().len());
            for (i, (pick, params)) in layers.iter().enumerate() {
                let thread = &match pick {
                    Pick::Thread(t) => *t,
                    Pick::Color(name, rgb) => match d.threads.iter().find(|t| t.color == *rgb) {
                        Some(t) => t.id,
                        None => d.add_thread(name.clone(), *rgb),
                    },
                };
                if i == 0 {
                    if let Some(obj) = d.object_mut(id) {
                        obj.thread = *thread;
                        obj.group = Some(g);
                        if let ObjectKind::Fill { params: p, .. } = &mut obj.kind {
                            *p = params.clone();
                        }
                    }
                    continue;
                }
                let ObjectKind::Fill { outer, holes, .. } = &o.kind else {
                    continue;
                };
                let new = d.insert_object(
                    at + i,
                    *thread,
                    ObjectKind::Fill {
                        outer: outer.clone(),
                        holes: holes.clone(),
                        params: params.clone(),
                    },
                );
                if let Some(obj) = d.object_mut(new) {
                    obj.group = Some(g);
                }
                ids.push(new);
            }
            ids
        });
        self.selection.set_all(ids);
        true
    }

    /// Makes a tartan of `sett` from the selected fill: a fill for each
    /// color, with a thread of the color of its code. One undo step.
    pub fn make_tartan(&mut self, sett: &Sett) -> bool {
        let Some((_, base)) = self.selected_fill() else {
            self.message = tr("Select one fill.").into();
            return false;
        };
        let params = tartan_layers(&base, sett);
        let picks = sett.colors.iter().map(|code| {
            let (name, rgb) = code_color(code);
            Pick::Color(name, rgb)
        });
        self.make_color_fill("Tartan", picks.zip(params).collect())
    }

    /// Makes a gradient from the selected fill: the colors `colors`
    /// (thread, place 0 to 1) across the rows. One undo step.
    pub fn make_gradient(&mut self, colors: &[(ThreadId, f64)]) -> bool {
        let Some((_, base)) = self.selected_fill() else {
            self.message = tr("Select one fill.").into();
            return false;
        };
        let stops: Vec<f64> = colors.iter().map(|(_, s)| s.clamp(0.0, 1.0)).collect();
        let params = gradient_layers(&base, None, &stops);
        self.make_color_fill(
            "Gradient",
            colors.iter().map(|(t, _)| Pick::Thread(*t)).zip(params).collect(),
        )
    }

    /// The tartan or gradient of the selected fill, in the window.
    fn load_color_fill(&mut self) {
        let Some((id, params)) = self.selected_fill() else {
            return;
        };
        match &params.pattern {
            FillPattern::Tartan { sett, .. } => {
                let w = &mut self.color_fills;
                w.gradient = false;
                w.sett_text = sett.text();
                w.count_mm = sett.count_mm;
                w.mirror = sett.mirror;
            }
            FillPattern::Gradient { stops, .. } => {
                let d = self.doc.design();
                let mut colors = vec![(None, 0.0); stops.len()];
                for x in std::iter::once(id).chain(self.color_fill_layers(id)) {
                    if let Some(o) = d.object(x)
                        && let ObjectKind::Fill { params, .. } = &o.kind
                        && let FillPattern::Gradient { color, .. } = params.pattern
                        && let Some(slot) = colors.get_mut(color as usize)
                    {
                        *slot = (Some(o.thread), stops[color as usize] * 100.0);
                    }
                }
                let w = &mut self.color_fills;
                w.gradient = true;
                w.colors = colors;
            }
            _ => {}
        }
    }
}

/// The Tartan and Gradient window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.color_fills.open {
        return;
    }
    let mut open = true;
    let threads = st.doc.design().threads.clone();
    let selected = st.selected_fill();
    crate::dock::docked(
        egui::Window::new(tr("Tartan and Gradient")).id(egui::Id::new("ColorFills")),
        ctx,
        egui::Id::new("ColorFills"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(380.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut st.color_fills.gradient, false, tr("Tartan"));
            ui.selectable_value(&mut st.color_fills.gradient, true, tr("Gradient"));
        });
        if selected.as_ref().is_some_and(|(_, p)| kind_of(&p.pattern).is_some())
            && ui.button(tr("Use the settings of the selected fill")).clicked()
        {
            st.load_color_fill();
        }
        ui.separator();
        if st.color_fills.gradient {
            gradient_ui(ui, st, &threads, selected.is_some());
        } else {
            tartan_ui(ui, st, selected.is_some());
        }
    });
    st.color_fills.open = open;
}

fn tartan_ui(ui: &mut egui::Ui, st: &mut AppState, can: bool) {
    let w = &mut st.color_fills;
    ui.horizontal(|ui| {
        ui.label(tr("Sett")).on_hover_text(tr(
            "The stripes of one repeat: a color code and a count for each, for example K4 R24 K24 Y4.",
        ));
        ui.add(egui::TextEdit::singleline(&mut w.sett_text).desired_width(240.0));
    });
    let mut pick = None;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("sett_presets")
            .selected_text(tr("Setts…"))
            .show_ui(ui, |ui| {
                for (name, text, mirror) in SETTS {
                    if ui.selectable_label(false, tr(name)).clicked() {
                        pick = Some((*text, *mirror));
                    }
                }
            });
        ui.checkbox(&mut w.mirror, tr("Mirrored")).on_hover_text(tr(
            "The stripes go out from the first stripe to the last and back, as in most tartans.",
        ));
    });
    if let Some((text, mirror)) = pick {
        w.sett_text = text.into();
        w.mirror = mirror;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Width of one count"));
        ui.add(
            egui::Slider::new(&mut w.count_mm, 0.1..=3.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" mm")
                .fixed_decimals(2),
        );
    });
    let sett = Sett::parse(&w.sett_text, w.count_mm, w.mirror);
    match &sett {
        Ok(s) => {
            ui.horizontal_wrapped(|ui| {
                for code in &s.colors {
                    let (name, rgb) = code_color(code);
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, crate::canvas::color32(rgb));
                    ui.label(format!("{code} {}", tr(&name)));
                }
            });
            ui.weak(crate::i18n::fill(
                "{} colors; one repeat is {} mm wide.",
                &[&s.colors.len(), &format!("{:.0}", s.width_mm())],
            ));
        }
        Err(e) => {
            ui.colored_label(ui.visuals().warn_fg_color, e.to_string());
        }
    }
    ui.weak(tr(
        "Codes: K black, W white, R red, DR dark red, C crimson, M maroon, O orange, Y yellow, GD gold, G green, DG dark green, LG light green, B blue, DB dark blue, LB light blue, A azure, P purple, N grey, T brown, LT tan.",
    ));
    let ok = sett.is_ok() && can;
    if ui
        .add_enabled(ok, egui::Button::new(tr("Make tartan")))
        .on_hover_text(tr(
            "A fill for each color of the sett, with the shape of the selected fill.",
        ))
        .on_disabled_hover_text(tr("Select one fill, and type a sett."))
        .clicked()
        && let Ok(s) = sett
    {
        st.make_tartan(&s);
    }
}

fn gradient_ui(ui: &mut egui::Ui, st: &mut AppState, threads: &[emb_model::Thread], can: bool) {
    let selected_thread = st
        .selection
        .single()
        .and_then(|id| st.doc.design().object(id))
        .map(|o| o.thread);
    let w = &mut st.color_fills;
    // Colors with no thread yet get the thread of the fill, then the next threads.
    for (i, (t, _)) in w.colors.iter_mut().enumerate() {
        if t.is_none() {
            *t = if i == 0 {
                selected_thread.or(threads.first().map(|t| t.id))
            } else {
                threads.get(i.min(threads.len().saturating_sub(1))).map(|t| t.id)
            };
        }
    }
    let mut remove = None;
    let n = w.colors.len();
    for (i, (t, place)) in w.colors.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(crate::i18n::fill("Color {}", &[&(i + 1)]));
            egui::ComboBox::from_id_salt(("gradient_thread", i))
                .selected_text(
                    t.and_then(|id| threads.iter().find(|x| x.id == id))
                        .map_or("(choose)".to_string(), |x| x.name.clone()),
                )
                .show_ui(ui, |ui| {
                    for x in threads {
                        ui.selectable_value(t, Some(x.id), &x.name);
                    }
                });
            ui.add(
                egui::Slider::new(place, 0.0..=100.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix(" %")
                    .fixed_decimals(0),
            );
            if n > 2 && ui.small_button(tr("Remove")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        w.colors.remove(i);
    }
    if w.colors.len() < 8 && ui.button(tr("Add color")).clicked() {
        w.colors.push((None, 100.0));
    }
    ui.weak(tr(
        "The colors change across the rows, from 0 % at one side of the shape to 100 % at the other. The fill angle sets the direction of the rows.",
    ));
    let colors: Vec<(ThreadId, f64)> = w
        .colors
        .iter()
        .filter_map(|(t, p)| t.map(|t| (t, *p / 100.0)))
        .collect();
    let ok = can && colors.len() == w.colors.len() && colors.len() >= 2;
    if ui
        .add_enabled(ok, egui::Button::new(tr("Make gradient")))
        .on_hover_text(tr("A fill for each color, with the shape of the selected fill."))
        .on_disabled_hover_text(tr("Select one fill, and a thread for each color."))
        .clicked()
    {
        st.make_gradient(&colors);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;
    use crate::test_util::Harness;

    fn one_fill(h: &mut Harness) -> ObjectId {
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::ZERO, Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            )
        });
        h.st.selection.set(id);
        id
    }

    #[test]
    fn a_tartan_is_a_group_of_fills_and_one_undo_step() {
        let mut h = Harness::empty();
        one_fill(&mut h);
        let sett = Sett::parse("K4 R8 G4", 0.5, true).unwrap();
        assert!(h.st.make_tartan(&sett));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 3);
        let g = d.objects()[0].group;
        assert!(g.is_some() && d.objects().iter().all(|o| o.group == g));
        let colors: Vec<emb_model::Rgb> = d.objects().iter().map(|o| d.thread(o.thread).unwrap().color).collect();
        assert_eq!(colors, [code_color("K").1, code_color("R").1, code_color("G").1]);
        // Each layer sews.
        for o in d.objects() {
            assert!(emb_stitch::stitch_object(o, Point::ZERO).is_ok());
        }
        // Make it again with another sett: the old layers go.
        h.st.selection.set(d.objects()[1].id);
        let two = Sett::parse("W8 B8", 0.5, false).unwrap();
        assert!(h.st.make_tartan(&two));
        assert_eq!(h.st.doc.design().objects().len(), 2);
        // Undo: the first tartan, then the plain fill (threads too).
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 3);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 1);
        assert_eq!(h.st.doc.design().threads.len(), 1);
    }

    #[test]
    fn a_gradient_loads_back_into_the_window() {
        let mut h = Harness::empty();
        let id = one_fill(&mut h);
        let blue =
            h.st.doc
                .edit("Thread", |d| d.add_thread("Blue", emb_model::Rgb(0, 0, 200)));
        let red = h.st.doc.design().objects()[0].thread;
        assert!(h.st.make_gradient(&[(red, 0.0), (blue, 0.8)]));
        assert_eq!(h.st.doc.design().objects().len(), 2);
        h.st.color_fills = ColorFillsWindow::default();
        h.st.selection.set(id);
        h.st.load_color_fill();
        assert!(h.st.color_fills.gradient);
        assert_eq!(h.st.color_fills.colors, [(Some(red), 0.0), (Some(blue), 80.0)]);
    }

    #[test]
    fn the_window_shows() {
        let mut h = Harness::empty();
        one_fill(&mut h);
        let draw = |h: &mut Harness| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
                ..Default::default()
            };
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(input, |ui| window(ui.ctx(), st));
            out.textures_delta.clear();
        };
        h.st.color_fills.open = true;
        draw(&mut h);
        h.st.color_fills.gradient = true;
        draw(&mut h);
        assert!(h.st.color_fills.open);
    }

    #[test]
    fn the_setts_read() {
        for (name, text, mirror) in SETTS {
            assert!(Sett::parse(text, 0.5, *mirror).is_ok(), "{name}");
        }
    }
}
