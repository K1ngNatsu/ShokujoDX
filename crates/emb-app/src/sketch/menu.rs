//! The right-click menu of sketch mode.

use super::*;

/// The right click of sketch mode: it ends the clicks of a draw tool, or
/// it selects the part under the pointer and opens the menu of the
/// commands for the selection.
pub(super) fn context_menu(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let pending = st.sketch.as_ref().is_some_and(|e| e.pending_dim.is_some());
        let busy = st
            .sketch
            .as_ref()
            .is_some_and(|e| !e.clicks.is_empty() || !e.picks.is_empty());
        if busy && !pending {
            // A right click ends a chain of lines (or the clicks of a tool).
            if let Some(e) = &mut st.sketch {
                e.clicks.clear();
                e.picks.clear();
                e.chain_start = None;
            }
            return;
        }
        let pick = pick_at(st, map, pos);
        let at = st.sketch_obj().map(|o| o.to_sketch(map.to_design(pos)));
        let tol = pick_mm(st, map);
        if let Some(e) = &mut st.sketch {
            if let Some(p) = pick
                && !e.selection.contains(&p)
            {
                e.selection = vec![p];
            }
            e.menu_at = at.map(|a| (a, tol));
        }
    }
    response.context_menu(|ui| sketch_menu(ui, st));
}

/// The commands of the right-click menu for the selection.
fn sketch_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let s = obj.sketch();
    let Some(e) = &st.sketch else { return };
    // A dimension that waits for its place: its kinds.
    if let Some(c) = e.pending_dim {
        ui.label(tr("Dimension Type"));
        for (name, k) in dimension_kinds(s, &c) {
            if ui.button(name).clicked()
                && let Some(e) = &mut st.sketch
            {
                e.pending_dim = Some(k);
                e.pending_locked = true;
            }
        }
        return;
    }
    let sel = e.selection.clone();
    let tool = e.tool;
    let (at, tol) = e.menu_at.unwrap_or((Point::new(0.0, 0.0), 0.5));
    let parts: Vec<Pick> = sel
        .iter()
        .copied()
        .filter(|p| !matches!(p, Pick::Constraint(_)))
        .collect();
    let curves: Vec<CurveId> = sel
        .iter()
        .filter_map(|p| match p {
            Pick::Curve(c) => Some(*c),
            _ => None,
        })
        .collect();
    let units = st.settings.units;
    if let [Pick::Constraint(id)] = sel.as_slice()
        && let Some(c) = s.constraint(*id).filter(|c| c.is_dimension()).copied()
    {
        let driven = s.is_driven(*id);
        if !driven
            && ui.button(tr("Change Value")).clicked()
            && let Some(e) = &mut st.sketch
        {
            e.edit_value(*id, box_text(units, s, *id, &c));
        }
        let label = if driven { tr("Make Driving") } else { tr("Make Driven") };
        if ui
            .button(label)
            .on_hover_text(tr(
                "A driven dimension only shows the size (in parentheses); it does not change the sketch.",
            ))
            .clicked()
        {
            st.toggle_driven(*id);
        }
        if s.label_at.contains_key(id) && ui.button(tr("Reset Label Place")).clicked() {
            let id = *id;
            st.edit_sketch(tr("Move Dimension"), None, |sk| {
                sk.label_at.remove(&id);
            });
        }
    }
    if !parts.is_empty() {
        // The dimension and the constraints that fit the selection.
        if let Build::Done(c) = dimension_of(s, &parts, true)
            && ui.button(tr("Dimension")).clicked()
        {
            st.add_sketch_dimension(c, None);
            if let Some(e) = &mut st.sketch {
                e.selection.clear();
            }
        }
        // Not the constraints that the parts have already.
        let fits: Vec<Kind> = Kind::ALL
            .into_iter()
            .filter(|k| match build(*k, s, &parts) {
                Build::Done(c) => !s.constraints.iter().any(|x| same_constraint(&x.constraint, &c)),
                _ => false,
            })
            .collect();
        if !fits.is_empty() {
            ui.separator();
            for k in fits {
                if ui.button(k.name()).clicked() {
                    st.apply_constraint(k, &parts);
                }
            }
        }
        ui.separator();
    }
    // The edits of one curve, at the place of the click.
    if let [c] = curves.as_slice() {
        if ui.button(tr("Trim")).clicked() {
            st.edit_click(SketchTool::Trim, at, tol);
        }
        if ui.button(tr("Split")).clicked() {
            st.edit_click(SketchTool::Split, at, tol);
        }
        if !matches!(s.curve(*c).map(|c| c.geometry), Some(Geometry::Circle { .. }))
            && ui.button(tr("Extend")).clicked()
        {
            st.edit_click(SketchTool::Extend, at, tol);
        }
    }
    // The corner of two lines.
    if let [Pick::Point(p)] = sel.as_slice() {
        let joined = s.joined(*p);
        let lines = s
            .curves
            .iter()
            .filter(|c| c.is_line() && c.points().iter().any(|q| joined.contains(q)))
            .count();
        if lines == 2
            && let Some(pos) = s.point(*p)
        {
            if ui.button(tr("Fillet")).clicked() {
                st.edit_click(SketchTool::Fillet, pos, tol);
            }
            if ui.button(tr("Chamfer")).clicked() {
                st.edit_click(SketchTool::Chamfer, pos, tol);
            }
        }
    }
    let picked = st.sketch.as_ref().map_or(0, |e| e.picked.len());
    if picked > 0 || tool == SketchTool::Regions {
        ui.label(tr("Make Embroidery"));
        for to in RegionTo::ALL {
            if ui.button(tr(to.name())).clicked() {
                st.make_picked_regions(to);
            }
        }
        ui.separator();
    }
    if !curves.is_empty() {
        if ui.button(tr("Run")).clicked() {
            st.make_curves(CurveTo::Run);
        }
        if ui.button(tr("Satin Line")).clicked() {
            let w = st.settings.defaults.column_width_mm;
            st.make_curves(CurveTo::SatinLine(w));
        }
        if curves.len() == 2 && ui.button(tr("Satin Between")).clicked() {
            st.make_satin_between();
        }
        if ui.button(tr("Construction")).clicked() {
            st.toggle_construction();
        }
        if curves.iter().all(|c| is_line(s, *c)) && ui.button(tr("Centerline")).clicked() {
            st.toggle_centerline();
        }
        for (t, name) in [
            (SketchTool::Move, tr("Move")),
            (SketchTool::Copy, tr("Copy")),
            (SketchTool::Rotate, tr("Rotate")),
            (SketchTool::Scale, tr("Scale")),
        ] {
            if ui.button(name).clicked() {
                st.set_sketch_tool(t);
            }
        }
        if ui.button(tr("Offset")).clicked() {
            st.offset_curves(&curves);
        }
        if ui.button(tr("Mirror")).clicked() {
            st.set_sketch_tool(SketchTool::Mirror);
        }
        if ui.button(tr("Rectangular Pattern")).clicked() {
            st.set_sketch_tool(SketchTool::RectPattern);
        }
        if ui.button(tr("Circular Pattern")).clicked() {
            st.set_sketch_tool(SketchTool::CircularPattern);
        }
    }
    if sel.iter().any(|p| matches!(p, Pick::Point(_))) && ui.button(tr("Center Point")).clicked() {
        st.toggle_center_point();
    }
    if !parts.is_empty() && ui.button(tr("Delete Constraints")).clicked() {
        st.remove_sketch_constraints_of_selection();
    }
    if !sel.is_empty() {
        if ui.button(tr("Delete")).clicked() {
            st.delete_sketch_selection();
        }
        ui.separator();
    }
    if tool != SketchTool::Select && ui.button(tr("Select")).clicked() {
        st.sketch_escape();
    }
    if ui.button(tr("Select All")).clicked() {
        st.sketch_select_all();
    }
    if ui.button(tr("Finish Sketch")).clicked() {
        st.finish_sketch();
    }
}
