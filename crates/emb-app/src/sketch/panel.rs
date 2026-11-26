//! The Properties panel in sketch mode and of a sketch object, and the
//! value box of a dimension.

use super::*;

/// The text box of a dimension value, over its label.
pub(super) fn value_editor(ui: &egui::Ui, map: &ViewMap, st: &mut AppState, obj: &SketchObject) {
    let Some((id, mut text)) = st.sketch.as_ref().and_then(|e| e.value_edit.clone()) else {
        return;
    };
    let Some(l) = labels(obj, map, st.settings.units).into_iter().find(|l| l.id == id) else {
        st.sketch.as_mut().expect("in sketch mode").value_edit = None;
        return;
    };
    let unit = match obj.sketch().constraint(id) {
        Some(Constraint::Angle { .. }) => "°",
        _ => st.settings.units.suffix(),
    };
    let first = st.sketch.as_mut().is_some_and(|e| std::mem::take(&mut e.value_new));
    let mut done = None;
    egui::Area::new(egui::Id::new("sketch_value"))
        .fixed_pos(l.rect.left_bottom() + Vec2::new(0.0, 4.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let out = egui::TextEdit::singleline(&mut text)
                        .id(egui::Id::new(("sketch_value_text", id.0)))
                        .desired_width(90.0)
                        .show(ui);
                    let r = out.response.response.clone();
                    if first {
                        // The old value is selected: typing replaces it.
                        r.request_focus();
                        let mut state = out.state.clone();
                        state
                            .cursor
                            .set_char_range(Some(egui::text_selection::CCursorRange::select_all(&out.galley)));
                        state.store(ui.ctx(), r.id);
                    }
                    ui.label(unit);
                    crate::a11y::named(r.clone(), tr("Dimension value")).on_hover_text(tr(
                        "Type a value. You can add a unit, for example 2 in, 1 1/2\" or 50 mm, use + - * /, and the names of parameters.",
                    ));
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button(tr("OK")).clicked() || (r.lost_focus() && enter) {
                        done = Some(true);
                    } else if r.lost_focus() && !first {
                        // A click somewhere else keeps the value that is typed.
                        done = Some(true);
                    }
                });
            });
        });
    match done {
        Some(apply) => {
            st.sketch.as_mut().expect("in sketch mode").value_edit = None;
            if apply {
                st.set_dimension_text(id, &text);
            }
        }
        None => st.sketch.as_mut().expect("in sketch mode").value_edit = Some((id, text)),
    }
}

/// The Properties panel in sketch mode: the state, the selection, and the
/// constraints with their values.
pub fn edit_panel(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let Some(a) = st.sketch_analysis() else { return };
    let s = obj.sketch();
    ui.heading(tr("Sketch"));
    if s.is_empty() {
        ui.label(tr(
            "Draw with the tools of the Sketch tab: lines, rectangles, circles, arcs and polygons.",
        ));
    } else if !a.conflicting.is_empty() {
        ui.colored_label(
            Color32::from_rgb(230, 60, 50),
            fill(
                "{} constraints conflict with the others (red). Remove them, or change a value.",
                &[&a.conflicting.len()],
            ),
        );
    } else if a.free == 0 {
        ui.colored_label(
            Color32::from_rgb(40, 160, 60),
            tr("Fully constrained: nothing can move."),
        );
    } else {
        ui.label(fill(
            "{} free motions: add constraints or dimensions to fix the shape (green is fixed).",
            &[&a.free],
        ));
    }
    if !a.redundant.is_empty() {
        ui.weak(fill("{} constraints repeat others (gray).", &[&a.redundant.len()]));
    }
    ui.label(fill(
        "{} points, {} curves, {} constraints.",
        &[&s.points.len(), &s.curves.len(), &s.constraints.len()],
    ));
    make_embroidery_ui(ui, st);
    ui.separator();
    let mut construction = st.sketch.as_ref().is_some_and(|e| e.construction);
    if ui
        .checkbox(&mut construction, tr("New curves are construction curves"))
        .on_hover_text(tr("Construction curves help to draw. They do not become embroidery."))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.construction = construction;
    }
    // The units of the program (Options), also here.
    ui.horizontal(|ui| {
        ui.label(tr("Units"));
        let mut units = st.settings.units;
        let a = ui.radio_value(&mut units, crate::workspace::Units::Mm, "mm");
        let b = ui.radio_value(&mut units, crate::workspace::Units::Inch, "in");
        if (a.changed() || b.changed()) && units != st.settings.units {
            let mut s = st.settings.clone();
            s.units = units;
            st.set_settings(s);
            if let Some(e) = &mut st.sketch {
                e.values = EditValues::for_units(units);
            }
        }
    })
    .response
    .on_hover_text(tr(
        "The units of the program (also in Options). You can type a value in other units, for example 2 in or 50 mm.",
    ));
    let mut auto = st.sketch.as_ref().is_some_and(|e| e.auto);
    if ui
        .checkbox(&mut auto, tr("Add constraints while drawing"))
        .on_hover_text(tr(
            "Horizontal, vertical, tangent, on a curve and coincident. Hold Alt while you click to add none.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.auto = auto;
    }
    let mut marks = st.sketch.as_ref().is_some_and(|e| e.show_marks);
    if ui
        .checkbox(&mut marks, tr("Show constraints"))
        .on_hover_text(tr(
            "Show the marks of the constraints (F8 shows, F9 hides). The dimensions always show.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.show_marks = marks;
    }
    edit_values(ui, st);
    if let Some(e) = &st.sketch
        && !e.relaxed.is_empty()
    {
        ui.weak(fill("Relax removed: {}.", &[&e.relaxed.join(", ")]));
    }
    let mut outside = st.sketch.as_ref().is_some_and(|e| e.polygon_outside);
    if ui
        .checkbox(&mut outside, tr("Polygon round its circle"))
        .on_hover_text(tr(
            "On: the sides touch the circle (the click is the middle of a side). Off: the corners are on the circle.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.polygon_outside = outside;
    }
    let mut slot = st.sketch.as_ref().map_or(SlotKind::CenterToCenter, |e| e.slot_kind);
    ui.horizontal(|ui| {
        ui.label(tr("Slot"));
        egui::ComboBox::from_id_salt("sketch_slot_kind")
            .selected_text(slot.name())
            .show_ui(ui, |ui| {
                for k in SlotKind::ALL {
                    ui.selectable_value(&mut slot, k, k.name());
                }
            });
    });
    if let Some(e) = &mut st.sketch
        && e.slot_kind != slot
    {
        e.slot_kind = slot;
        e.clicks.clear();
    }
    let mut sides = st.sketch.as_ref().map_or(6, |e| e.polygon_sides);
    ui.horizontal(|ui| {
        ui.label(tr("Polygon sides"));
        if ui.add(crate::workspace::drag(&mut sides).range(3..=40)).changed()
            && let Some(e) = &mut st.sketch
        {
            e.polygon_sides = sides;
        }
    });
    ui.separator();
    ui.strong(tr("Constraints"));
    let mut remove = None;
    let mut select = None;
    let mut edit = None;
    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        for c in &s.constraints {
            ui.horizontal(|ui| {
                let color = if a.conflicting.contains(&c.id) {
                    Color32::from_rgb(230, 60, 50)
                } else if a.redundant.contains(&c.id) {
                    Color32::from_gray(150)
                } else {
                    ui.visuals().text_color()
                };
                let text = constraint_text(st.settings.units, &c.constraint);
                if ui
                    .selectable_label(false, egui::RichText::new(text).color(color))
                    .clicked()
                {
                    select = Some(c.id);
                }
                if c.constraint.is_dimension() && ui.small_button(tr("Change")).clicked() {
                    edit = Some((c.id, box_text(st.settings.units, s, c.id, &c.constraint)));
                }
                if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the constraint")).clicked()
                {
                    remove = Some(c.id);
                }
            });
        }
    });
    if let Some(id) = remove {
        st.edit_sketch(tr("Remove Constraint"), None, |sk| sk.remove_constraint(id));
    }
    if let Some(e) = &mut st.sketch {
        if let Some(id) = select {
            e.selection = vec![Pick::Constraint(id)];
        }
        if let Some((id, text)) = edit {
            e.edit_value(id, text);
        }
    }
}

/// The values of the edit tool that is on (fillet radius, chamfer and
/// offset distance, pattern).
fn edit_values(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(e) = &st.sketch else { return };
    let (tool, mut v) = (e.tool, e.values.clone());
    let units = st.settings.units;
    let max = emb_sketch::MAX_MM;
    let mut make_pattern = false;
    let mut motion: Option<Motion> = None;
    let row = |ui: &mut egui::Ui, label: &str, f: &mut dyn FnMut(&mut egui::Ui)| {
        ui.horizontal(|ui| {
            ui.label(label);
            f(ui);
        });
    };
    match tool {
        SketchTool::Fillet => row(ui, tr("Fillet radius"), &mut |ui| {
            crate::workspace::length_value(ui, &mut v.fillet_mm, units, 0.01, max);
        }),
        SketchTool::Chamfer => {
            row(ui, tr("Chamfer"), &mut |ui| {
                for (k, name) in [
                    (0, tr("Equal distances")),
                    (1, tr("Two distances")),
                    (2, tr("Distance and angle")),
                ] {
                    ui.radio_value(&mut v.chamfer_kind, k, name);
                }
            });
            row(ui, tr("Chamfer distance"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.chamfer_mm, units, 0.01, max);
            });
            match v.chamfer_kind {
                1 => row(ui, tr("Second distance"), &mut |ui| {
                    crate::workspace::length_value(ui, &mut v.chamfer2_mm, units, 0.01, max);
                }),
                2 => row(ui, tr("Angle"), &mut |ui| {
                    ui.add(
                        crate::workspace::drag(&mut v.chamfer_deg)
                            .range(1.0..=179.0)
                            .suffix("°"),
                    );
                }),
                _ => {}
            }
            if v.chamfer_kind != 0 {
                ui.weak(tr(
                    "The first distance is on the first line of the corner (the older line).",
                ));
            }
        }
        SketchTool::Offset => {
            row(ui, tr("Offset distance"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.offset_mm, units, -max, max);
            });
            ui.weak(tr(
                "Positive: outside of a closed shape, else on the left. Negative: the other side.",
            ));
        }
        SketchTool::RectPattern => {
            row(ui, tr("Number of columns"), &mut |ui| {
                ui.add(crate::workspace::drag(&mut v.cols).range(1..=100));
            });
            row(ui, tr("Number of rows"), &mut |ui| {
                ui.add(crate::workspace::drag(&mut v.rows).range(1..=100));
            });
            row(ui, tr("Column spacing"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.dx_mm, units, -max, max);
            });
            row(ui, tr("Row spacing"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.dy_mm, units, -max, max);
            });
            make_pattern = ui.button(tr("Make Pattern")).clicked();
        }
        SketchTool::CircularPattern => {
            row(ui, tr("Copies"), &mut |ui| {
                ui.add(crate::workspace::drag(&mut v.count).range(2..=400));
            });
            row(ui, tr("Angle"), &mut |ui| {
                ui.add(crate::workspace::drag(&mut v.angle_deg).range(1.0..=360.0).suffix("°"));
            });
        }
        SketchTool::Move | SketchTool::Copy => {
            ui.weak(tr("Click a base point, then where it goes. Or move by these values:"));
            row(ui, "X", &mut |ui| {
                crate::workspace::length_value(ui, &mut v.move_dx_mm, units, -max, max);
            });
            row(ui, "Y", &mut |ui| {
                crate::workspace::length_value(ui, &mut v.move_dy_mm, units, -max, max);
            });
            if ui.button(tr("Move by the Values")).clicked() {
                motion = Some(Motion::Shift(Point::new(v.move_dx_mm, v.move_dy_mm)));
            }
        }
        SketchTool::Rotate => {
            row(ui, tr("Angle"), &mut |ui| {
                ui.add(
                    crate::workspace::drag(&mut v.rotate_deg)
                        .range(-360.0..=360.0)
                        .suffix("°"),
                );
            });
            ui.checkbox(&mut v.copy_parts, tr("Copy"));
            ui.weak(tr(
                "Click the center of the turn (counterclockwise for a positive angle).",
            ));
        }
        SketchTool::Scale => {
            row(ui, tr("Factor"), &mut |ui| {
                ui.add(crate::workspace::drag(&mut v.scale).range(0.01..=100.0).speed(0.01));
            });
            ui.checkbox(&mut v.copy_parts, tr("Copy"));
            ui.weak(tr("Click the base point of the scale."));
        }
        _ => return,
    }
    if let Some(e) = &mut st.sketch {
        e.values = v;
    }
    if make_pattern {
        st.rect_pattern();
    }
    if let Some(m) = motion {
        st.apply_motion(tool, m, tool == SketchTool::Copy);
    }
}

/// The section of the Properties panel that makes embroidery from the
/// open sketch: its regions (the picked ones, else all) and its selected
/// curves.
fn make_embroidery_ui(ui: &mut egui::Ui, st: &mut AppState) {
    ui.separator();
    ui.strong(tr("Make Embroidery"));
    let regions = st.sketch_regions();
    let picked = st.sketch.as_ref().map_or(0, |e| e.picked.len());
    if regions.is_empty() {
        ui.weak(tr("The sketch has no closed regions yet."));
    } else if picked == 0 {
        ui.weak(fill(
            "{} closed regions. Pick some with the Regions tool, or make all:",
            &[&regions.len()],
        ));
    } else {
        ui.weak(fill("{} regions picked:", &[&picked]));
    }
    if !regions.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for to in RegionTo::ALL {
                if ui.button(tr(to.name())).clicked() {
                    st.make_picked_regions(to);
                }
            }
        });
    }
    let curves = st.selected_curves();
    if !curves.is_empty() {
        ui.weak(fill("{} curves selected:", &[&curves.len()]));
        let mut width = st.settings.defaults.column_width_mm;
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Run")).clicked() {
                st.make_curves(CurveTo::Run);
            }
            if ui.button(tr("Satin Line")).clicked() {
                st.make_curves(CurveTo::SatinLine(width));
            }
            if crate::workspace::length_value(ui, &mut width, st.settings.units, 0.5, 15.0).changed() {
                let mut s = st.settings.clone();
                s.defaults.column_width_mm = width;
                st.set_settings(s);
            }
            if curves.len() == 2 && ui.button(tr("Satin Between")).clicked() {
                st.make_satin_between();
            }
        });
    }
    ui.weak(tr(
        "The stitch settings are the current properties; the thread is the current thread.",
    ));
}

/// A short description of a sketch, for the Sequence panel.
pub fn detail(s: &SketchObject) -> String {
    let sk = s.sketch();
    fill(
        "Sketch · {} curves, {} constraints",
        &[&sk.curves.len(), &sk.constraints.len()],
    )
}

/// The Properties panel of a sketch object (not open).
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, s: &SketchObject) {
    ui.label(detail(s));
    let a = s.sketch().analyze();
    ui.weak(if a.is_fully_constrained() {
        tr("Fully constrained.").to_string()
    } else {
        fill("{} free motions.", &[&a.free])
    });
    if ui.button(tr("Edit Sketch")).clicked() {
        st.selection.set(id);
        st.open_sketch(id);
    }
    // The parameters (ADR 0078).
    let params = &s.sketch().parameters;
    if !params.is_empty() {
        ui.label(fill("{} parameters:", &[&params.len()]));
        for p in params {
            ui.weak(format!("{} = {}", p.name, p.expr));
        }
    }
    if ui.button(tr("Parameters")).clicked() {
        st.selection.set(id);
        st.open_sketch(id);
        st.toggle_parameters();
    }
    let regions = s.regions();
    ui.separator();
    ui.label(fill("Make embroidery of all {} regions:", &[&regions.len()]));
    ui.horizontal_wrapped(|ui| {
        for to in RegionTo::ALL {
            if ui
                .add_enabled(!regions.is_empty(), egui::Button::new(tr(to.name())))
                .clicked()
            {
                st.make_regions(id, regions.clone(), to);
            }
        }
    });
    if ui.button(tr("Runs Along the Curves")).clicked() {
        let sources = s
            .sketch()
            .curves
            .iter()
            .filter(|c| !c.construction)
            .map(|c| LinkSource::Curve {
                curve: c.id,
                to: CurveTo::Run,
            })
            .collect();
        let n = st.make_linked("Sketch to Stitches", id, sources).len();
        st.message = fill("{} objects made from the sketch.", &[&n]);
    }
    links_ui(ui, st, id);
}

/// The made objects of sketch `id` in the Properties panel of the sketch:
/// how many follow it, and those whose region or curve is gone (with
/// Unlink).
fn links_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId) {
    let d = st.doc.design();
    let links: Vec<&emb_model::SketchLink> = d.sketch_links.iter().filter(|l| l.sketch == id).collect();
    if links.is_empty() {
        return;
    }
    let n: usize = links.iter().map(|l| l.objects.len()).sum();
    let lost: Vec<u32> = links.iter().filter(|l| l.lost).map(|l| l.id).collect();
    ui.separator();
    ui.weak(fill("{} objects made from this sketch follow its changes.", &[&n]));
    if !lost.is_empty() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            fill("The region or curve of {} links is gone.", &[&lost.len()]),
        );
        if ui.button(tr("Unlink the Lost Objects")).clicked() {
            st.doc
                .edit(tr("Unlink"), |d| d.sketch_links.retain(|l| !lost.contains(&l.id)));
        }
    }
}

/// The Properties panel lines of an object made from a sketch: Edit
/// Sketch, Unlink, and a warning when its region or curve is gone.
pub fn made_from_sketch_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId) {
    let Some(link) = st.doc.design().sketch_link_of(id).cloned() else {
        return;
    };
    ui.separator();
    if link.lost {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            tr("Made from a sketch, but its region or curve is gone: the object stays as it is."),
        );
    } else {
        ui.weak(tr(
            "Made from a sketch: it changes and moves with the sketch, and does not scale or change its nodes alone. Break Apart or Unlink frees it.",
        ));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Edit Sketch")).clicked() {
            st.open_sketch(link.sketch);
        }
        if ui.button(tr("Unlink")).clicked() {
            st.doc.edit(tr("Unlink"), |d| d.unlink_sketch(link.id));
            st.message = tr("Unlinked: the objects stay as they are.").into();
        }
    });
}
