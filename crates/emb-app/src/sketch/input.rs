//! The keys and the canvas input of sketch mode: picks, snaps, clicks of
//! the tools and drags.

use super::*;

/// The part of the open sketch at screen position `pos`.
pub(super) fn pick_at(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<Pick> {
    let obj = st.sketch_obj()?;
    let s = obj.sketch();
    let marks = st.sketch.as_ref().is_none_or(|e| e.show_marks);
    for l in labels_with(obj, map, st.settings.units, marks) {
        if l.rect.expand(if l.dimension { 4.0 } else { 0.0 }).contains(pos) {
            return Some(Pick::Constraint(l.id));
        }
    }
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let point = s
        .points
        .iter()
        .map(|p| (p.id, to(p.pos).distance(pos)))
        .filter(|(_, d)| *d <= PICK_PX)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((id, _)) = point {
        return Some(Pick::Point(id));
    }
    let tol = f64::from(PICK_PX) / map.view.scale();
    let at = map.to_design(pos);
    obj.curve_paths()
        .iter()
        .find(|(_, path)| {
            path.flatten(tol / 4.0)
                .iter()
                .any(|pl| pl.segments().any(|(a, b)| at.distance_to_segment(a, b) <= tol))
        })
        .map(|(id, _)| Pick::Curve(*id))
}

/// The parts of the sketch inside screen rectangle `r`: the points, the
/// curves with all their points inside, and the labels of constraints.
fn box_picks(obj: &SketchObject, map: &ViewMap, units: crate::workspace::Units, r: Rect) -> Vec<Pick> {
    let s = obj.sketch();
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    let mut out: Vec<Pick> = Vec::new();
    for (id, path) in obj.curve_paths() {
        let all_in = path
            .flatten(tol)
            .iter()
            .all(|pl| pl.points.iter().all(|p| r.contains(map.to_screen(*p))));
        if all_in {
            out.push(Pick::Curve(*id));
        }
    }
    for p in &s.points {
        if r.contains(to(p.pos)) && !s.curves.iter().any(|c| c.points().contains(&p.id)) {
            out.push(Pick::Point(p.id));
        }
    }
    for l in labels(obj, map, units) {
        if r.contains_rect(l.rect) {
            out.push(Pick::Constraint(l.id));
        }
    }
    out
}

/// Relax mode: the driving dimensions are driven for a drag, so the drag
/// changes them. Returns them, for `restore_dims`.
pub(super) fn relax_dims(sk: &mut Sketch, on: bool) -> Vec<ConstraintId> {
    if !on {
        return Vec::new();
    }
    let dims: Vec<ConstraintId> = sk
        .constraints
        .iter()
        .filter(|c| c.constraint.is_dimension() && !c.driven)
        .map(|c| c.id)
        .collect();
    for c in &mut sk.constraints {
        if dims.contains(&c.id) {
            c.driven = true;
        }
    }
    dims
}

/// The dimensions of `relax_dims` drive again, with the sizes of now.
pub(super) fn restore_dims(sk: &mut Sketch, dims: &[ConstraintId]) {
    for d in dims {
        sk.set_driven(*d, false);
    }
}

/// The tolerance of a click in sketch mm.
pub(super) fn pick_mm(map: &ViewMap) -> f64 {
    f64::from(PICK_PX) / map.view.scale()
}

/// The snap of a click at screen position `pos`: a point or a curve of the
/// sketch near it. With `alt`, no snap.
pub(super) fn snap_at(st: &AppState, map: &ViewMap, pos: Pos2, alt: bool) -> Option<Snap> {
    let obj = st.sketch_obj()?;
    let at = obj.to_sketch(map.to_design(pos));
    if alt {
        return Some(Snap::free(at));
    }
    Some(infer::snap(obj.sketch(), at, pick_mm(map), &[]))
}

/// The keys of sketch mode (before the keys of the program): Esc ends the
/// command and goes back to Select, Delete and Backspace delete the
/// selection, Enter ends a chain of lines, Ctrl+A selects all. While the
/// value box of a dimension is open, it takes the keys (Esc closes it).
/// Returns true if the program must not use the keys of this frame.
pub fn handle_keys(ctx: &egui::Context, st: &mut AppState) -> bool {
    use egui::{Key, Modifiers};
    let Some(e) = &st.sketch else { return false };
    if e.value_edit.is_some() {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if let Some(e) = &mut st.sketch {
                e.value_edit = None;
            }
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        return true;
    }
    // Another text field (for example in the Properties panel).
    if ctx.memory(|m| m.focused()).is_some() {
        return false;
    }
    let (esc, del, enter, all, show, hide) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::Escape),
            i.consume_key(Modifiers::NONE, Key::Delete) | i.consume_key(Modifiers::NONE, Key::Backspace),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::COMMAND, Key::A),
            i.consume_key(Modifiers::NONE, Key::F8),
            i.consume_key(Modifiers::NONE, Key::F9),
        )
    });
    if (show || hide)
        && let Some(e) = &mut st.sketch
    {
        e.show_marks = show;
    }
    if esc {
        st.sketch_escape();
    }
    if del {
        st.delete_sketch_selection();
    }
    if enter && st.sketch.as_ref().is_some_and(|e| e.tool == SketchTool::Spline) {
        finish_spline(st, false);
    } else if enter && let Some(e) = &mut st.sketch {
        e.clicks.clear();
        e.chain_start = None;
    }
    if all {
        st.sketch_select_all();
    }
    false
}

/// The canvas input of sketch mode. The other objects take no clicks.
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    st.check_sketch();
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let tool = st.sketch.as_ref().map_or(SketchTool::Select, |e| e.tool);
    context_menu(response, st, map);
    let to_sketch = |pos: Pos2| obj.to_sketch(map.to_design(pos));
    let mods = ui.input(|i| i.modifiers);
    // The line that the pointer goes over while a line is drawn: the
    // parallel and perpendicular hints use it.
    if tool == SketchTool::Line
        && let Some(pos) = response.hover_pos()
        && let Some((c, _)) = obj.sketch().nearest_curve(to_sketch(pos), pick_mm(map), &[])
        && is_line(obj.sketch(), c)
        && let Some(e) = st.sketch.as_mut()
    {
        e.reference = Some(c);
    }
    let auto = st.sketch.as_ref().is_some_and(|e| e.auto);
    // Alt (or the panel setting off) stops the constraints of the drawing
    // help for this click.
    let no_help = mods.alt || !auto;

    // A double-click on a dimension: type its value.
    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(Pick::Constraint(id)) = pick_at(st, map, pos)
        && let Some(c) = obj.sketch().constraint(id).filter(|c| c.is_dimension()).copied()
        && !obj.sketch().is_driven(id)
    {
        let text = box_text(st.settings.units, obj.sketch(), id, &c);
        let e = st.sketch.as_mut().expect("in sketch mode");
        e.edit_value(id, text);
        return;
    }

    match tool {
        SketchTool::Select => {
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                let s = obj.sketch();
                let start = to_sketch(origin);
                let under = pick_at(st, map, origin);
                let sel = st.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
                let (points, radius) = match under {
                    // A drag of a dimension label moves the label.
                    Some(Pick::Constraint(id)) if s.constraint(id).is_some_and(|c| c.is_dimension()) => {
                        let e = st.sketch.as_mut().expect("in sketch mode");
                        e.drags += 1;
                        e.label_drag = Some((id, e.drags));
                        (Vec::new(), None)
                    }
                    // A drag of a part of the selection moves all of it.
                    Some(p) if sel.len() > 1 && sel.contains(&p) => {
                        let mut pts: Vec<PointId> = Vec::new();
                        for q in &sel {
                            match *q {
                                Pick::Point(id) => pts.push(id),
                                Pick::Curve(c) => pts.extend(s.curve(c).map(|c| c.points()).unwrap_or_default()),
                                Pick::Constraint(_) => {}
                            }
                        }
                        (s.with_handles(&pts), None)
                    }
                    // A point of a spline takes its handles with it.
                    Some(Pick::Point(p)) => (s.with_handles(&[p]), None),
                    Some(Pick::Curve(c)) => match s.curve(c).map(|c| c.geometry) {
                        Some(Geometry::Line { a, b }) => (vec![a, b], None),
                        Some(Geometry::Circle { .. }) => (Vec::new(), Some(c)),
                        Some(Geometry::Arc { center, start, end, .. }) => (vec![center, start, end], None),
                        Some(Geometry::Bezier { start, c1, c2, end }) => (vec![start, c1, c2, end], None),
                        None => (Vec::new(), None),
                    },
                    _ => (Vec::new(), None),
                };
                let e = st.sketch.as_mut().expect("in sketch mode");
                if !points.is_empty() || radius.is_some() {
                    e.drags += 1;
                    e.drag = Some(Drag {
                        points: points.iter().filter_map(|p| Some((*p, s.point(*p)?))).collect(),
                        start,
                        radius,
                        key: e.drags,
                    });
                } else if under.is_none() {
                    // A drag on an empty place: a box selection.
                    st.sketch.as_mut().expect("in sketch mode").boxsel = Some(origin);
                }
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some((id, key)) = st.sketch.as_ref().and_then(|e| e.label_drag)
            {
                let at = to_sketch(pos);
                st.edit_sketch(tr("Move Dimension"), Some(format!("sketch-label-{key}")), |sk| {
                    sk.label_at.insert(id, at);
                });
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(e) = st.sketch.as_mut()
                && e.label_drag.take().is_some()
            {
                st.doc.end_merge();
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(d) = st.sketch.as_ref().and_then(|e| e.drag.clone())
            {
                let now = to_sketch(pos);
                let delta = now - d.start;
                let key = Some(format!("sketch-drag-{}", d.key));
                let relax = st.sketch.as_ref().is_some_and(|e| e.relax_mode);
                if let Some(c) = d.radius {
                    st.edit_sketch(tr("Drag in Sketch"), key, |sk| {
                        let dims = relax_dims(sk, relax);
                        let center = match sk.curve(c).map(|c| c.geometry) {
                            Some(Geometry::Circle { center, .. }) => sk.point(center),
                            _ => None,
                        };
                        if let Some(cp) = center
                            && let Some(x) = sk.curves.iter_mut().find(|x| x.id == c)
                            && let Geometry::Circle { radius, .. } = &mut x.geometry
                        {
                            *radius = cp.distance(now).max(0.05);
                        }
                        sk.solve();
                        restore_dims(sk, &dims);
                    });
                } else {
                    let moves: Vec<(PointId, Point)> = d.points.iter().map(|(p, at)| (*p, *at + delta)).collect();
                    st.edit_sketch(tr("Drag in Sketch"), key, |sk| {
                        let dims = relax_dims(sk, relax);
                        sk.drag_points(&moves);
                        restore_dims(sk, &dims);
                    });
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(e) = st.sketch.as_mut()
                && e.drag.take().is_some()
            {
                st.doc.end_merge();
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(start) = st.sketch.as_mut().and_then(|e| e.boxsel.take())
                && let Some(end) = response.interact_pointer_pos()
            {
                let inside = box_picks(&obj, map, st.settings.units, Rect::from_two_pos(start, end));
                let e = st.sketch.as_mut().expect("in sketch mode");
                if !(mods.command || mods.shift) {
                    e.selection.clear();
                }
                for p in inside {
                    if !e.selection.contains(&p) {
                        e.selection.push(p);
                    }
                }
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let p = pick_at(st, map, pos);
                let e = st.sketch.as_mut().expect("in sketch mode");
                match p {
                    Some(p) if mods.command || mods.shift => {
                        if let Some(i) = e.selection.iter().position(|x| *x == p) {
                            e.selection.remove(i);
                        } else {
                            e.selection.push(p);
                        }
                    }
                    Some(p) => e.selection = vec![p],
                    None => e.selection.clear(),
                }
            }
        }
        SketchTool::Constrain(kind) => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(p) = pick_at(st, map, pos).filter(|p| !matches!(p, Pick::Constraint(_)))
            {
                let mut picks = st.sketch.as_ref().map(|e| e.picks.clone()).unwrap_or_default();
                picks.push(p);
                if !st.apply_constraint(kind, &picks)
                    && let Some(e) = &mut st.sketch
                {
                    e.picks = picks;
                }
            }
        }
        SketchTool::Dimension => {
            let hit = response
                .clicked()
                .then(|| response.interact_pointer_pos())
                .flatten()
                .map(|pos| pick_at(st, map, pos).filter(|p| !matches!(p, Pick::Constraint(_))));
            // The parts are picked: this click places the label.
            if hit.is_some()
                && let Some(c) = st.sketch.as_ref().and_then(|e| e.pending_dim)
            {
                let at = response.interact_pointer_pos().map(to_sketch);
                let locked = st.sketch.as_ref().is_some_and(|e| e.pending_locked);
                let c = match at {
                    Some(at) if !locked => orient(obj.sketch(), c, at),
                    _ => c,
                };
                if let Some(e) = st.sketch.as_mut() {
                    e.pending_dim = None;
                    e.pending_locked = false;
                }
                st.add_sketch_dimension(c, at);
                return;
            }
            if let Some(hit) = hit {
                let mut picks = st.sketch.as_ref().map(|e| e.picks.clone()).unwrap_or_default();
                let empty = hit.is_none();
                if empty && picks.is_empty() {
                    return;
                }
                picks.extend(hit);
                // A circle or an arc is a dimension at once; a point waits for
                // a point or a line, a line for the next click.
                match dimension_of(obj.sketch(), &picks, empty) {
                    Build::Done(c) if empty => {
                        // A line, then a place: the label goes there.
                        let at = response.interact_pointer_pos().map(to_sketch);
                        let c = at.map_or(c, |at| orient(obj.sketch(), c, at));
                        st.add_sketch_dimension(c, at);
                    }
                    Build::Done(c) => {
                        // All parts: the ghost follows the pointer until a
                        // click places it.
                        if let Some(e) = &mut st.sketch {
                            e.picks = picks;
                            e.pending_dim = Some(c);
                        }
                    }
                    Build::Wrong(why) => {
                        st.message = why.into();
                        if let Some(e) = &mut st.sketch {
                            e.picks.clear();
                        }
                    }
                    Build::More => {
                        if let Some(e) = &mut st.sketch {
                            e.picks = picks;
                        }
                    }
                }
            }
        }
        SketchTool::RectPattern => {}
        SketchTool::Regions => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                st.toggle_region(to_sketch(pos));
            }
        }
        SketchTool::Stretch => {
            // A drag: the box of the points.
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                && let Some(e) = st.sketch.as_mut()
            {
                e.boxsel = Some(origin);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(start) = st.sketch.as_mut().and_then(|e| e.boxsel.take())
                && let Some(end) = response.interact_pointer_pos()
            {
                let r = Rect::from_two_pos(start, end);
                let to = |p: Point| map.to_screen(obj.to_design(p));
                let pts: Vec<PointId> = obj
                    .sketch()
                    .points
                    .iter()
                    .filter(|p| r.contains(to(p.pos)))
                    .map(|p| p.id)
                    .collect();
                let e = st.sketch.as_mut().expect("in sketch mode");
                e.selection = pts.iter().map(|p| Pick::Point(*p)).collect();
                e.source.clear();
                e.source_points = pts;
                e.clicks.clear();
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let at = infer::snap(obj.sketch(), to_sketch(pos), pick_mm(map), &[]).at;
                let e = st.sketch.as_mut().expect("in sketch mode");
                if e.source_points.is_empty() {
                    st.message = tr("Drag a box round the points to stretch first.").into();
                } else if e.clicks.is_empty() {
                    e.clicks.push(Snap::free(at));
                } else {
                    let base = e.clicks[0].at;
                    e.clicks.clear();
                    st.apply_motion(SketchTool::Stretch, Motion::Shift(at - base), false);
                }
            }
        }
        t if t.is_edit() => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                st.edit_click(t, to_sketch(pos), pick_mm(map));
            }
        }
        _ => {
            // Right click ends a chain of lines, or a spline.
            if response.secondary_clicked() && tool == SketchTool::Spline {
                finish_spline(st, false);
            } else if response.secondary_clicked()
                && let Some(e) = &mut st.sketch
            {
                e.clicks.clear();
                e.chain_start = None;
            }
            // The Line tool: a drag from the end of a line makes a tangent arc.
            if tool == SketchTool::Line
                && response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                && let Some(from) = snap_at(st, map, origin, false)
                && let Some(p) = from.point
                && obj.sketch().curves.iter().any(|c| match c.geometry {
                    Geometry::Line { a, b } => a == p || b == p,
                    _ => false,
                })
                && let Some(e) = st.sketch.as_mut()
            {
                e.arc_drag = Some(from);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(from) = st.sketch.as_mut().and_then(|e| e.arc_drag.take())
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(to) = snap_at(st, map, pos, no_help)
            {
                let opts = st.sketch.as_ref().map(MakeOpts::of).expect("in sketch mode");
                let start = from.point.expect("an end");
                let end = st
                    .edit_sketch(tr("Sketch: Tangent Arc"), None, |sk| {
                        make(SketchTool::ArcTangent, sk, &[from, to], opts)?;
                        match sk.curves.last()?.geometry {
                            Geometry::Arc { start: a, end: b, .. } => Some(if a == start { b } else { a }),
                            _ => None,
                        }
                    })
                    .flatten();
                // The chain goes on from the end of the arc.
                if let Some(end) = end
                    && let Some(at) = st.sketch_obj().and_then(|o| o.sketch().point(end))
                    && let Some(e) = st.sketch.as_mut()
                {
                    e.clicks = vec![Snap::on_point(at, end)];
                    e.chain_start.get_or_insert(start);
                }
                return;
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(snap) = snap_at(st, map, pos, no_help)
            {
                // A click on a point uses it (also with Alt), so a chain closes.
                let snap = match (snap.point, snap_at(st, map, pos, false)) {
                    (None, Some(s)) if s.point.is_some() && no_help => s,
                    _ => snap,
                };
                // The tangent circle takes lines, not their points.
                let snap = if tool == SketchTool::TangentCircle {
                    match obj
                        .sketch()
                        .nearest_curve(snap.at, pick_mm(map), &[])
                        .filter(|(c, _)| is_line(obj.sketch(), *c))
                    {
                        Some((c, q)) => Snap::on_curve(q, c),
                        None => {
                            st.message = tr("Click a line.").into();
                            return;
                        }
                    }
                } else {
                    snap
                };
                let e = st.sketch.as_mut().expect("in sketch mode");
                if tool == SketchTool::ArcTangent && e.clicks.is_empty() && snap.point.is_none() {
                    st.message = tr("Click the end of a line.").into();
                    return;
                }
                if tool == SketchTool::Line {
                    line_click(st, snap, no_help);
                    return;
                }
                if tool == SketchTool::Spline {
                    spline_click(st, snap, pick_mm(map));
                    return;
                }
                e.clicks.push(snap);
                let clicks = e.clicks.clone();
                let opts = MakeOpts::of(e);
                if clicks.len() >= clicks_needed(tool, &opts) {
                    st.edit_sketch(tr("Sketch"), None, |sk| make(tool, sk, &clicks, opts));
                    if let Some(e) = &mut st.sketch {
                        e.clicks.clear();
                    }
                }
            }
        }
    }
}

/// A click of the spline tool: a new point of the spline. A click on the
/// last point ends it; a click on the first point (after three or more)
/// closes it.
pub(super) fn spline_click(st: &mut AppState, to: Snap, tol: f64) {
    let Some(e) = st.sketch.as_mut() else { return };
    if let Some(last) = e.clicks.last()
        && last.at.distance(to.at) <= tol
    {
        finish_spline(st, false);
        return;
    }
    if e.clicks.len() >= 3 && e.clicks[0].at.distance(to.at) <= tol {
        finish_spline(st, true);
        return;
    }
    e.clicks.push(to);
}

/// Makes the spline of the clicks of the spline tool (two or more). Its
/// ends join the points or the curves that the first and the last click
/// snapped to.
fn finish_spline(st: &mut AppState, closed: bool) {
    let Some(e) = st.sketch.as_mut() else { return };
    let clicks = std::mem::take(&mut e.clicks);
    let construction = e.construction;
    if clicks.len() < 2 {
        return;
    }
    st.edit_sketch(tr("Sketch: Spline"), None, |sk| {
        let pts: Vec<Point> = clicks.iter().map(|c| c.at).collect();
        let Ok(pieces) = sk.add_spline(&pts, closed) else {
            return;
        };
        let first = pieces
            .first()
            .and_then(|c| sk.curve(*c))
            .and_then(|c| c.ends())
            .map(|e| e.0);
        let last = pieces
            .last()
            .and_then(|c| sk.curve(*c))
            .and_then(|c| c.ends())
            .map(|e| e.1);
        let ends = if closed {
            vec![(clicks[0], first)]
        } else {
            vec![(clicks[0], first), (clicks[clicks.len() - 1], last)]
        };
        for (snap, end) in ends {
            let Some(end) = end else { continue };
            if let Some(q) = snap.point {
                let _ = sk.add_constraint(Constraint::Coincident { a: q, b: end });
            } else if let Some(c) = snap.curve {
                let _ = sk.add_constraint(Constraint::OnCurve { point: end, curve: c });
            }
        }
        for c in &pieces {
            sk.set_construction(*c, construction);
        }
    });
}

/// The end and the hints of a line from the last click of the chain to
/// `to` (no hints with `no_help`).
pub(super) fn line_end(st: &AppState, to: &Snap, no_help: bool) -> Option<(Snap, Vec<Hint>)> {
    let from = *st.sketch.as_ref()?.clicks.first()?;
    if no_help {
        return Some((*to, Vec::new()));
    }
    let reference = st.sketch.as_ref()?.reference;
    let (end, hints) = infer::line_hints_with(st.sketch_obj()?.sketch(), &from, to, reference);
    Some((Snap { at: end, ..*to }, hints))
}

/// A click of the line tool: the first click starts a chain; each next
/// click adds a line from the last point, with the constraints of the
/// drawing help. A click on the first point of the chain closes it.
pub(super) fn line_click(st: &mut AppState, to: Snap, no_help: bool) {
    let Some(e) = st.sketch.as_mut() else { return };
    let Some(&from) = e.clicks.first() else {
        let first = to;
        e.clicks = vec![first];
        e.chain_start = first.point;
        return;
    };
    let construction = e.construction;
    let Some((to, hints)) = line_end(st, &to, no_help) else {
        return;
    };
    let ends = st.edit_sketch(tr("Sketch: Line"), None, |sk| {
        let pa = infer::place(sk, &from);
        let pb = infer::place(sk, &to);
        if pa == pb {
            return None;
        }
        let l = sk.add_line_between(pa, pb).ok()?;
        sk.set_construction(l, construction);
        infer::apply_hints(sk, l, &hints);
        Some((pa, pb))
    });
    let Some(e) = st.sketch.as_mut() else { return };
    let Some(Some((pa, pb))) = ends else { return };
    let start = *e.chain_start.get_or_insert(pa);
    if pb == start {
        e.clicks.clear();
        e.chain_start = None;
    } else {
        let at = st.sketch_obj().and_then(|o| o.sketch().point(pb)).unwrap_or(to.at);
        let e = st.sketch.as_mut().expect("in sketch mode");
        e.clicks = vec![Snap::on_point(at, pb)];
    }
}
