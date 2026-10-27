//! Decoration windows (P5 Milestone 7): Blends (two-color blend and multi
//! blend layers), Buttonholes (one, or a string along a line) and Borders
//! (decorative frames round the selection or the design).

use eframe::egui;
use emb_geom::BBox;
use emb_model::decor::{blend_densities, blend_profile, buttonhole, frame_objects, frames, string_positions};
use emb_model::{BlendRecord, ButtonholeSpec, ObjectId, ObjectKind, ThreadId};

use crate::app::AppState;
use crate::i18n::tr;
use crate::stitch_types::column_params;

/// The window states.
#[derive(Debug, Clone)]
pub struct DecorWindows {
    pub blends: bool,
    pub buttonholes: bool,
    pub borders: bool,
    /// The thread of the second color of a new blend.
    pub blend_thread: Option<ThreadId>,
    pub buttonhole: ButtonholeSpec,
    pub buttonhole_count: usize,
    pub buttonhole_across: bool,
    /// The index of the frame in the list.
    pub frame: usize,
    pub frame_margin_mm: f64,
}

impl Default for DecorWindows {
    fn default() -> Self {
        DecorWindows {
            blends: false,
            buttonholes: false,
            borders: false,
            blend_thread: None,
            buttonhole: ButtonholeSpec::default(),
            buttonhole_count: 5,
            buttonhole_across: true,
            frame: 0,
            frame_margin_mm: 4.0,
        }
    }
}

/// The accordion profile slot of a fill or a satin column.
fn profile_of(k: &mut ObjectKind) -> Option<&mut Option<emb_model::Accordion>> {
    if let ObjectKind::Fill { params, .. } = k {
        return Some(&mut params.accordion);
    }
    match k {
        ObjectKind::Satin { .. } => column_params(k).map(|c| &mut c.accordion),
        _ => None,
    }
}

fn blendable(k: &ObjectKind) -> bool {
    matches!(k, ObjectKind::Fill { .. } | ObjectKind::Satin { .. })
}

impl AppState {
    /// The multi blend that the selection is in: its group and its layers
    /// in sew order.
    pub fn selected_blend(&self) -> Option<(u32, Vec<ObjectId>)> {
        let d = self.doc.design();
        let g = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id)?.group)
            .find(|g| d.blends.iter().any(|b| b.group == *g))?;
        let layers = d
            .objects()
            .iter()
            .filter(|o| o.group == Some(g))
            .map(|o| o.id)
            .collect();
        Some((g, layers))
    }

    /// Makes a two-color blend of the selected fill or satin column: the
    /// object goes from dense to open, and a copy in `second` above it from
    /// open to dense. The two are one group (a multi blend). One undo step.
    pub fn make_blend(&mut self, second: ThreadId) -> bool {
        let d = self.doc.design();
        let Some(o) = self
            .selection
            .single()
            .and_then(|id| d.object(id))
            .filter(|o| blendable(&o.kind))
            .cloned()
        else {
            self.message = tr("Select one fill or satin column to blend.").into();
            return false;
        };
        let ids = self.doc.edit("Color blend", |d| {
            let g = d.new_group_id();
            let i = d.index_of(o.id).unwrap_or(d.objects().len());
            let mut top = o.kind.clone();
            if let Some(slot) = profile_of(&mut top) {
                *slot = Some(blend_profile(10.0, 100.0));
            }
            // The top layer has no underlay: the bottom layer has it.
            match &mut top {
                ObjectKind::Fill { params, .. } => params.underlay = emb_model::Underlay::None,
                ObjectKind::Satin { params, .. } => params.underlay = emb_model::Underlay::None,
                _ => {}
            }
            let new = d.insert_object(i + 1, second, top);
            if let Some(obj) = d.object_mut(o.id) {
                obj.group = Some(g);
                if let Some(slot) = profile_of(&mut obj.kind) {
                    *slot = Some(blend_profile(100.0, 10.0));
                }
            }
            if let Some(obj) = d.object_mut(new) {
                obj.group = Some(g);
            }
            d.blends.push(BlendRecord { group: g });
            vec![o.id, new]
        });
        self.selection.set_all(ids);
        true
    }

    /// Adds a layer on top of the selected blend: a copy of its top layer,
    /// from open to dense. One undo step.
    pub fn add_blend_layer(&mut self) {
        let Some((g, layers)) = self.selected_blend() else {
            return;
        };
        let Some(&top) = layers.last() else { return };
        let Some(o) = self.doc.design().object(top).cloned() else {
            return;
        };
        let new = self.doc.edit("Add blend layer", |d| {
            let i = d.index_of(top).map_or(d.objects().len(), |i| i + 1);
            let mut kind = o.kind.clone();
            if let Some(slot) = profile_of(&mut kind) {
                *slot = Some(blend_profile(10.0, 60.0));
            }
            let id = d.insert_object(i, o.thread, kind);
            if let Some(obj) = d.object_mut(id) {
                obj.group = Some(g);
            }
            id
        });
        self.selection.set(new);
    }
}

/// The Blends window: make a two-color blend, or edit the layers of the
/// selected blend (color, density at the start and the end, order).
fn blends_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.decor.blends;
    let blend = st.selected_blend();
    let threads = st.doc.design().threads.clone();
    crate::dock::docked(
        egui::Window::new(tr("Blends")).id(egui::Id::new("Blends")),
        ctx,
        egui::Id::new("Blends"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(360.0)
    .show(ctx, |ui| {
        let Some((g, layers)) = blend else {
            ui.label(tr("Select one fill or satin column to blend two colors."))
                .on_hover_text(tr(crate::words::BLEND_HELP));
            let mut second = st.decor.blend_thread.or_else(|| threads.get(1).map(|t| t.id));
            ui.horizontal(|ui| {
                ui.label(tr("Second color"));
                egui::ComboBox::from_id_salt("blend_second")
                    .selected_text(
                        second
                            .and_then(|id| threads.iter().find(|t| t.id == id))
                            .map_or("(choose)".to_string(), |t| t.name.clone()),
                    )
                    .show_ui(ui, |ui| {
                        for t in &threads {
                            ui.selectable_value(&mut second, Some(t.id), &t.name);
                        }
                    });
            });
            st.decor.blend_thread = second;
            let can = second.is_some()
                && st
                    .selection
                    .single()
                    .and_then(|id| st.doc.design().object(id))
                    .is_some_and(|o| blendable(&o.kind));
            if ui
                .add_enabled(can, egui::Button::new(tr("Make a color blend")))
                .clicked()
                && let Some(t) = second
            {
                st.make_blend(t);
            }
            return;
        };
        ui.label(crate::i18n::fill(
            "Blend with {} layers (first sews first).",
            &[&(layers.len())],
        ));
        let mut action: Option<(usize, &str)> = None;
        for (i, id) in layers.iter().enumerate() {
            let Some(o) = st.doc.design().object(*id).cloned() else {
                continue;
            };
            let mut kind = o.kind.clone();
            let (a0, b0) = blend_densities(profile_of(&mut kind).and_then(|s| s.as_ref()));
            let (mut a, mut b) = (a0, b0);
            let mut thread = o.thread;
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong(crate::i18n::fill("Layer {}", &[&(i + 1)]));
                egui::ComboBox::from_id_salt(("blend_thread", i))
                    .selected_text(threads.iter().find(|t| t.id == thread).map_or("", |t| t.name.as_str()))
                    .show_ui(ui, |ui| {
                        for t in &threads {
                            ui.selectable_value(&mut thread, t.id, &t.name);
                        }
                    });
                if i > 0 && ui.small_button(tr("Up")).clicked() {
                    action = Some((i, "up"));
                }
                if i + 1 < layers.len() && ui.small_button(tr("Down")).clicked() {
                    action = Some((i, "down"));
                }
                if layers.len() > 2 && ui.small_button(tr("Remove")).clicked() {
                    action = Some((i, "remove"));
                }
            });
            let (ra, rb) = ui
                .horizontal(|ui| {
                    ui.label(tr("Density from"));
                    let ra = ui.add(
                        egui::Slider::new(&mut a, 5.0..=100.0)
                            .clamping(egui::SliderClamping::Edits)
                            .suffix(" %")
                            .fixed_decimals(0),
                    );
                    ui.label(tr("to"));
                    let rb = ui.add(
                        egui::Slider::new(&mut b, 5.0..=100.0)
                            .clamping(egui::SliderClamping::Edits)
                            .suffix(" %")
                            .fixed_decimals(0),
                    );
                    (ra, rb)
                })
                .inner;
            if thread != o.thread {
                st.doc.edit("Change blend color", |d| {
                    if let Some(obj) = d.object_mut(*id) {
                        obj.thread = thread;
                    }
                });
            }
            // Only a real change is an edit (a slider can report a change
            // when it first shows a value).
            if (ra.changed() || rb.changed()) && ((a - a0).abs() > 1e-6 || (b - b0).abs() > 1e-6) {
                st.doc.edit_merged(
                    "Change blend density",
                    emb_model::MergeKey::new(format!("{}:blend", id.0)),
                    |d| {
                        if let Some(slot) = d.object_mut(*id).and_then(|o| profile_of(&mut o.kind)) {
                            *slot = Some(blend_profile(a, b));
                        }
                    },
                );
            }
            if ra.drag_stopped() || rb.drag_stopped() || ra.lost_focus() || rb.lost_focus() {
                st.doc.end_merge();
            }
        }
        ui.separator();
        if ui.button(tr("Add layer")).clicked() {
            action = Some((0, "add"));
        }
        if let Some((i, what)) = action {
            match what {
                "add" => st.add_blend_layer(),
                "remove" => {
                    let id = layers[i];
                    st.doc.edit("Remove blend layer", |d| {
                        d.remove_object(id);
                    });
                }
                _ => {
                    let j = if what == "up" { i - 1 } else { i + 1 };
                    let (a, b) = (layers[i], layers[j]);
                    st.doc.edit("Move blend layer", |d| {
                        // Next to each other: moving one to the other's place swaps them.
                        if let Some(ib) = d.index_of(b) {
                            d.move_object(a, ib);
                        }
                    });
                }
            }
            let _ = g;
        }
    });
    st.decor.blends = open;
}

impl AppState {
    /// Adds buttonholes: a string along the selected line, or one in the
    /// middle of the hoop. One undo step; the buttonholes are one group.
    pub fn add_buttonholes(&mut self) -> usize {
        let w = self.decor.clone();
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
        let (positions, thread, used) = match line {
            Some((id, t, pts)) => (
                string_positions(&pts, w.buttonhole_count.max(1), w.buttonhole_across),
                Some(t),
                Some(id),
            ),
            None => (vec![(d.hoop.center, 90.0)], d.threads.first().map(|t| t.id), None),
        };
        let n = positions.len();
        let ids = self.doc.edit("Buttonholes", |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Thread", emb_model::Rgb(30, 30, 30)));
            let g = d.new_group_id();
            if let Some(id) = used {
                d.remove_object(id);
            }
            let mut ids = Vec::new();
            for (c, a) in positions {
                for kind in buttonhole(&w.buttonhole, c, a) {
                    let id = d.add_object(t, kind);
                    if let Some(o) = d.object_mut(id) {
                        o.group = Some(g);
                        o.name = Some("Buttonhole".into());
                    }
                    ids.push(id);
                }
            }
            ids
        });
        self.selection.set_all(ids);
        self.message = crate::i18n::fill(
            "{} buttonholes added. Cut each opening between the legs after sewing.",
            &[&n],
        );
        n
    }

    /// Adds the frame of the Borders window round the selection (or the
    /// whole design). It sews last. One undo step; the frame is one group.
    pub fn add_frame(&mut self) -> bool {
        let w = self.decor.clone();
        let all = frames();
        let Some(frame) = all.get(w.frame).cloned() else {
            return false;
        };
        let d = self.doc.design();
        let ids = self.sel();
        let boxed: Vec<BBox> = if ids.is_empty() {
            d.objects().iter().filter_map(|o| o.control_bbox()).collect()
        } else {
            ids.iter().filter_map(|id| d.object(*id)?.control_bbox()).collect()
        };
        let b = boxed
            .into_iter()
            .reduce(BBox::union)
            .unwrap_or_else(|| BBox::from_center_size(d.hoop.center, 60.0, 40.0));
        let thread = ids
            .first()
            .and_then(|id| d.object(*id))
            .map(|o| o.thread)
            .or_else(|| d.threads.first().map(|t| t.id));
        let kinds = frame_objects(&frame, &b, w.frame_margin_mm);
        let new = self.doc.edit(format!("Border {}", frame.name), |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Thread", emb_model::Rgb(30, 30, 30)));
            let g = d.new_group_id();
            kinds
                .into_iter()
                .map(|k| {
                    let id = d.add_object(t, k);
                    if let Some(o) = d.object_mut(id) {
                        o.group = Some(g);
                        o.name = Some(frame.name.clone());
                    }
                    id
                })
                .collect::<Vec<_>>()
        });
        self.selection.set_all(new);
        true
    }
}

fn buttonholes_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.decor.buttonholes;
    let mut add = false;
    let on_line = st
        .selection
        .single()
        .and_then(|id| st.doc.design().object(id))
        .is_some_and(|o| matches!(o.kind, ObjectKind::Running { .. }));
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
        let w = &mut st.decor;
        let s = &mut w.buttonhole;
        for (label, v, range) in [
            (tr("Opening"), &mut s.length_mm, 5.0..=50.0),
            (tr("Leg width"), &mut s.leg_mm, 0.8..=4.0),
            (tr("Gap for the cut"), &mut s.gap_mm, 0.2..=2.0),
            (tr("Bar tack"), &mut s.bartack_mm, 0.5..=4.0),
            (tr("Density"), &mut s.spacing_mm, 0.2..=0.6),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::Slider::new(v, range)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix(" mm")
                        .fixed_decimals(1),
                );
            });
        }
        ui.separator();
        if on_line {
            ui.label(tr(
                "A string of buttonholes along the selected line (the line is used up).",
            ));
            ui.horizontal(|ui| {
                ui.label(tr("Number"));
                ui.add(egui::Slider::new(&mut w.buttonhole_count, 1..=20).clamping(egui::SliderClamping::Edits));
            });
            ui.checkbox(&mut w.buttonhole_across, tr("Across the line (else along it)"));
        } else {
            ui.weak(tr("Select a line to place a string of buttonholes along it."));
        }
        add = ui
            .button(if on_line {
                "Place buttonholes"
            } else {
                "Place one buttonhole"
            })
            .clicked();
    });
    st.decor.buttonholes = open;
    if add {
        st.add_buttonholes();
    }
}

fn borders_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.decor.borders;
    let mut add = false;
    let all = frames();
    crate::dock::docked(
        egui::Window::new(tr("Borders")).id(egui::Id::new("Borders")),
        ctx,
        egui::Id::new("Borders"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(300.0)
    .show(ctx, |ui| {
        let w = &mut st.decor;
        egui::Grid::new("frames").spacing([6.0, 4.0]).show(ui, |ui| {
            for (i, f) in all.iter().enumerate() {
                ui.selectable_value(&mut w.frame, i, &f.name);
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Space round the objects"));
            ui.add(
                egui::Slider::new(&mut w.frame_margin_mm, 0.0..=20.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix(" mm"),
            );
        });
        add = ui
            .button(tr("Add the border"))
            .on_hover_text(tr(
                "Round the selection, or round the whole design if nothing is selected.",
            ))
            .clicked();
    });
    st.decor.borders = open;
    if add {
        st.add_frame();
    }
}

/// All decoration windows.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    if st.decor.blends {
        blends_window(ctx, st);
    }
    if st.decor.buttonholes {
        buttonholes_window(ctx, st);
    }
    if st.decor.borders {
        borders_window(ctx, st);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::FillParams;

    use super::*;
    use crate::test_util::Harness;

    fn fill(h: &mut Harness) -> (ObjectId, ThreadId) {
        h.st.doc.edit("Add", |d| {
            let a = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let b = d.add_thread("Gold", emb_model::Rgb(230, 170, 30));
            let id = d.add_object(
                a,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::ZERO, Point::new(40.0, 30.0))),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            );
            (id, b)
        })
    }

    fn draw(h: &mut Harness) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| windows(ui.ctx(), st));
        out.textures_delta.clear();
    }

    #[test]
    fn a_two_color_blend_and_more_layers() {
        let mut h = Harness::empty();
        let (id, second) = fill(&mut h);
        h.st.selection.set(id);
        assert!(h.st.make_blend(second));
        let (_, layers) = h.st.selected_blend().unwrap();
        assert_eq!(layers.len(), 2);
        let d = h.st.doc.design();
        let dens = |id: ObjectId| {
            let mut k = d.object(id).unwrap().kind.clone();
            blend_densities(profile_of(&mut k).and_then(|s| s.as_ref()))
        };
        assert_eq!(dens(layers[0]), (100.0, 10.0));
        assert_eq!(dens(layers[1]), (10.0, 100.0));
        assert_eq!(d.object(layers[1]).unwrap().thread, second);
        assert_eq!(d.blends.len(), 1);
        h.st.add_blend_layer();
        assert_eq!(h.st.selected_blend().unwrap().1.len(), 3);
        // The window draws for a blend and for a plain fill, with no edit.
        h.st.decor.blends = true;
        let before = h.st.doc.undo_len();
        draw(&mut h);
        assert_eq!(h.st.doc.undo_len(), before, "{:?}", h.st.doc.undo_label());
        h.st.selection.clear();
        draw(&mut h);
        assert_eq!(h.st.doc.undo_len(), before);
        let mut labels = Vec::new();
        while let Some(l) = h.st.doc.undo_label().map(str::to_string) {
            labels.push(l);
            h.st.doc.undo();
        }
        assert_eq!(labels.len(), 3, "{labels:?}");
    }

    #[test]
    fn a_string_of_buttonholes_along_a_line() {
        let mut h = Harness::empty();
        let line = h.st.doc.edit("Add", |d| {
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
        });
        h.st.selection.set(line);
        h.st.decor.buttonhole_count = 4;
        assert_eq!(h.st.add_buttonholes(), 4);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 4 * 5, "the line is used up");
        assert!(d.objects().iter().all(|o| o.group == d.objects()[0].group));
        h.st.decor.buttonholes = true;
        draw(&mut h);
    }

    #[test]
    fn a_border_round_the_selection() {
        let mut h = Harness::empty();
        let (id, _) = fill(&mut h);
        h.st.selection.set(id);
        h.st.decor.frame = frames().iter().position(|f| f.name == "Hearts").unwrap();
        assert!(h.st.add_frame());
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        let b = d.objects()[1].control_bbox().unwrap();
        assert!(b.min.x < -4.0 && b.max.x > 44.0, "{b:?}");
        h.st.decor.borders = true;
        draw(&mut h);
    }
}
