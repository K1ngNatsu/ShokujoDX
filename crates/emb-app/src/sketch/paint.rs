//! The drawing of sketches on the canvas: the sketch objects that are not
//! open, and the open sketch with its colors, labels and hints.

use super::*;

/// The colors of the parts of a sketch.
fn colors(dark: bool) -> (Color32, Color32, Color32, Color32) {
    let free = if dark {
        Color32::from_rgb(110, 180, 255)
    } else {
        Color32::from_rgb(20, 100, 210)
    };
    let fixed = if dark {
        Color32::from_rgb(120, 210, 120)
    } else {
        Color32::from_rgb(20, 140, 40)
    };
    let conflict = Color32::from_rgb(230, 60, 50);
    let construction = Color32::from_rgb(230, 150, 40);
    (free, fixed, conflict, construction)
}

/// Draws the sketch objects that are not open (thin lines; construction
/// curves dashed).
pub fn draw_objects(painter: &egui::Painter, map: &ViewMap, st: &AppState) {
    let open = st.sketch.as_ref().map(|e| e.object);
    let d = st.doc.design();
    for o in d.objects().iter().filter(|o| st.view_filter.shows(o)) {
        let ObjectKind::Sketch(s) = &o.kind else { continue };
        if Some(o.id) == open {
            continue;
        }
        let selected = st.selection.contains(o.id);
        let color = if selected {
            Color32::from_rgb(80, 160, 255)
        } else {
            Color32::from_gray(140)
        };
        for (id, path) in s.curve_paths() {
            let construction = s.sketch().curve(*id).is_some_and(|c| c.construction);
            draw_path(
                painter,
                map,
                path,
                Stroke::new(if selected { 1.8 } else { 1.0 }, color),
                construction,
            );
        }
    }
}

/// A centerline: long and short dashes.
fn draw_centerline(painter: &egui::Painter, map: &ViewMap, path: &emb_geom::Path, stroke: Stroke) {
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    for pl in path.flatten(tol) {
        let pts: Vec<Pos2> = pl.points.iter().map(|p| map.to_screen(*p)).collect();
        painter.extend(Shape::dashed_line_with_offset(
            &pts,
            stroke,
            &[14.0, 3.0],
            &[3.0, 3.0],
            0.0,
        ));
    }
}

fn draw_path(painter: &egui::Painter, map: &ViewMap, path: &emb_geom::Path, stroke: Stroke, dashed: bool) {
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    for pl in path.flatten(tol) {
        let mut pts: Vec<Pos2> = pl.points.iter().map(|p| map.to_screen(*p)).collect();
        if pl.closed
            && let Some(f) = pts.first().copied()
        {
            pts.push(f);
        }
        if dashed {
            painter.extend(Shape::dashed_line(&pts, stroke, 6.0, 4.0));
        } else {
            painter.add(Shape::line(pts, stroke));
        }
    }
}

/// Draws the open sketch: the other objects dim, the curves in the color
/// of their state, the points, the marks and dimensions, the preview of the
/// draw tool, and the value editor of a dimension.
pub fn draw_overlay(painter: &egui::Painter, ui: &egui::Ui, map: &ViewMap, st: &mut AppState) {
    // The object is shared (an `Arc`), so a hold of it is not a copy of the sketch.
    let Some(id) = st.sketch.as_ref().map(|e| e.object) else {
        return;
    };
    let Some(shared) = st.doc.design().objects().iter().find(|o| o.id == id).cloned() else {
        return;
    };
    let ObjectKind::Sketch(obj) = &shared.kind else { return };
    let Some(a) = st.sketch_analysis() else { return };
    let dark = ui.visuals().dark_mode;
    let bg = if dark {
        Color32::from_rgba_unmultiplied(32, 32, 32, 150)
    } else {
        Color32::from_rgba_unmultiplied(248, 248, 248, 150)
    };
    painter.rect_filled(map.rect, 0.0, bg);
    let (free, fixed, conflict, construction) = colors(dark);
    let s = obj.sketch();
    let e = st.sketch.clone().expect("in sketch mode");
    let selected = |p: Pick| e.selection.contains(&p) || e.picks.contains(&p);
    let accent = Color32::from_rgb(255, 200, 40);
    // The part under the pointer, for the tools that click parts.
    let picks_parts = matches!(
        e.tool,
        SketchTool::Select | SketchTool::Dimension | SketchTool::Constrain(_)
    ) || e.tool.is_edit();
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|p| map.rect.contains(*p) && picks_parts && e.drag.is_none())
        .and_then(|p| {
            if e.tool.is_edit() && !matches!(e.tool, SketchTool::Fillet | SketchTool::Chamfer) {
                let at = obj.to_sketch(map.to_design(p));
                s.nearest_curve(at, pick_mm(map), &[]).map(|(c, _)| Pick::Curve(c))
            } else {
                pick_at(st, map, p)
            }
        });
    // The picked regions, shaded; with the Regions tool, the region under
    // the pointer.
    {
        let regions = st.sketch_regions();
        let ring = |path: &emb_geom::Path| -> Vec<Pos2> {
            path.flatten(0.1)
                .into_iter()
                .flat_map(|pl| pl.points)
                .map(|q| map.to_screen(obj.to_design(q)))
                .collect()
        };
        let smallest = |p: Point| {
            regions
                .iter()
                .filter(|r| r.contains(p))
                .min_by(|a, b| a.area.total_cmp(&b.area))
        };
        for q in &e.picked {
            if let Some(r) = smallest(*q) {
                let mut rings = vec![ring(&r.outer)];
                rings.extend(r.holes.iter().map(ring));
                painter.add(crate::applique::scanline_mesh(
                    &rings,
                    accent.gamma_multiply(0.3),
                    map.rect,
                ));
            }
        }
        if e.tool == SketchTool::Regions
            && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
            && let Some(r) = smallest(obj.to_sketch(map.to_design(hover)))
        {
            let mut rings = vec![ring(&r.outer)];
            rings.extend(r.holes.iter().map(ring));
            painter.add(crate::applique::scanline_mesh(
                &rings,
                accent.gamma_multiply(0.12),
                map.rect,
            ));
            for path in std::iter::once(&r.outer).chain(r.holes.iter()) {
                let mut pts = ring(path);
                if let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                painter.add(Shape::line(pts, Stroke::new(3.0, accent.gamma_multiply(0.8))));
            }
        }
    }
    // The constraints of the part under the pointer (they show also when
    // the marks are off), and the parts of the constraint under it.
    let related: Vec<ConstraintId> = match hovered {
        Some(Pick::Curve(c)) => s
            .constraints
            .iter()
            .filter(|x| x.constraint.curves().contains(&c))
            .map(|x| x.id)
            .collect(),
        Some(Pick::Point(q)) => s
            .constraints
            .iter()
            .filter(|x| x.constraint.points().contains(&q))
            .map(|x| x.id)
            .collect(),
        _ => Vec::new(),
    };
    let (hot_curves, hot_points) = match hovered.and_then(|p| match p {
        Pick::Constraint(id) => s.constraint(id).copied(),
        _ => None,
    }) {
        Some(c) => (c.curves(), c.points()),
        None => (Vec::new(), Vec::new()),
    };
    // The curves of the constraints in conflict.
    let bad: Vec<CurveId> = s
        .constraints
        .iter()
        .filter(|c| a.conflicting.contains(&c.id))
        .flat_map(|c| c.constraint.curves())
        .collect();
    let bad_points: Vec<PointId> = s
        .constraints
        .iter()
        .filter(|c| a.conflicting.contains(&c.id))
        .flat_map(|c| c.constraint.points())
        .collect();
    for (id, path) in obj.curve_paths() {
        let c = s.curve(*id).expect("the path of a curve");
        let color = if bad.contains(id) {
            conflict
        } else if c.construction {
            construction
        } else if a.fixed_curves.contains(id) {
            fixed
        } else {
            free
        };
        let width = if selected(Pick::Curve(*id)) { 3.0 } else { 1.8 };
        if hovered == Some(Pick::Curve(*id)) || hot_curves.contains(id) {
            draw_path(
                painter,
                map,
                path,
                Stroke::new(width + 4.0, accent.gamma_multiply(0.3)),
                false,
            );
        }
        if selected(Pick::Curve(*id)) {
            draw_path(
                painter,
                map,
                path,
                Stroke::new(width + 3.0, accent.gamma_multiply(0.5)),
                false,
            );
        }
        if c.centerline {
            // A centerline: long and short dashes.
            draw_centerline(painter, map, path, Stroke::new(width, color.gamma_multiply(0.9)));
        } else {
            draw_path(painter, map, path, Stroke::new(width, color), c.construction);
        }
    }
    let to = |p: Point| map.to_screen(obj.to_design(p));
    // The handles of the spline pieces: thin lines from their ends.
    for c in &s.curves {
        if let Geometry::Bezier { start, c1, c2, end } = c.geometry {
            for (e, h) in [(start, c1), (end, c2)] {
                if let (Some(pe), Some(ph)) = (s.point(e), s.point(h)) {
                    painter.line_segment([to(pe), to(ph)], Stroke::new(1.0, construction.gamma_multiply(0.8)));
                }
            }
        }
    }
    for p in &s.points {
        let pos = to(p.pos);
        let color = if bad_points.contains(&p.id) {
            conflict
        } else if a.fixed_points.contains(&p.id) {
            fixed
        } else {
            free
        };
        let r = if selected(Pick::Point(p.id)) { 4.5 } else { 3.0 };
        if s.is_origin(p.id) {
            // The origin: a ring with a cross through it.
            let k = 8.0;
            let st = Stroke::new(1.4, fixed);
            painter.circle_stroke(pos, 5.0, st);
            painter.line_segment([pos + Vec2::new(-k, 0.0), pos + Vec2::new(k, 0.0)], st);
            painter.line_segment([pos + Vec2::new(0.0, -k), pos + Vec2::new(0.0, k)], st);
        }
        if p.center {
            // A center point: a cross.
            let k = 6.0;
            let st = Stroke::new(1.6, color);
            painter.line_segment([pos + Vec2::new(-k, 0.0), pos + Vec2::new(k, 0.0)], st);
            painter.line_segment([pos + Vec2::new(0.0, -k), pos + Vec2::new(0.0, k)], st);
        }
        if hovered == Some(Pick::Point(p.id)) || hot_points.contains(&p.id) {
            painter.circle_filled(pos, r + 4.0, accent.gamma_multiply(0.35));
        }
        if selected(Pick::Point(p.id)) {
            painter.circle_filled(pos, r + 2.5, accent.gamma_multiply(0.6));
        }
        painter.circle_filled(pos, r, color);
    }
    // Coincident points: a ring.
    for c in &s.constraints {
        if let Constraint::Coincident { a: pa, .. } = c.constraint
            && let Some(p) = s.point(pa)
        {
            painter.circle_stroke(to(p), 5.5, Stroke::new(1.2, free));
        }
    }
    // Marks and dimensions.
    let text_bg = ui.visuals().extreme_bg_color.gamma_multiply(0.9);
    for l in labels_with(obj, map, st.settings.units, true) {
        if !(e.show_marks || l.dimension || related.contains(&l.id)) {
            continue;
        }
        let color = if a.conflicting.contains(&l.id) {
            conflict
        } else if a.redundant.contains(&l.id) || s.is_driven(l.id) {
            Color32::from_gray(150)
        } else if l.dimension {
            ui.visuals().text_color()
        } else {
            free
        };
        let sel = selected(Pick::Constraint(l.id));
        let hot = hovered == Some(Pick::Constraint(l.id)) || related.contains(&l.id);
        let frame = if hot {
            Stroke::new(2.0, accent)
        } else {
            Stroke::new(1.0, color.gamma_multiply(0.6))
        };
        let (fill, text) = if sel {
            (accent, Color32::BLACK)
        } else {
            (text_bg, color)
        };
        draw_label(painter, &l, color, fill, text, frame, 1.0);
    }
    // The Dimension tool: the dimension that the next click makes, at the
    // pointer, as a ghost.
    if e.tool == SketchTool::Dimension
        && e.value_edit.is_none()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let under = hovered.filter(|p| !matches!(p, Pick::Constraint(_)));
        let mut picks = e.picks.clone();
        if e.pending_dim.is_none() {
            picks.extend(under);
        }
        let empty = under.is_none() || e.pending_dim.is_some();
        let candidate = match e
            .pending_dim
            .map(Build::Done)
            .unwrap_or_else(|| dimension_of(s, &picks, empty))
        {
            Build::Done(c) => Some(c),
            // A line alone: its length, placed where the pointer is.
            Build::More => match picks.as_slice() {
                [Pick::Curve(l)] if is_line(s, *l) => dimension_of(s, &picks, true).done(),
                _ => None,
            },
            Build::Wrong(_) => None,
        };
        let at = obj.to_sketch(map.to_design(hover));
        let candidate = candidate.map(|c| {
            if e.pending_locked || !empty {
                c
            } else {
                orient(s, c, at)
            }
        });
        if let Some(c) = candidate {
            let mut ghost = obj.clone();
            let id = ghost.edit(|sk| {
                let id = sk.add_constraint(c).ok()?;
                if empty {
                    sk.label_at.insert(id, at);
                }
                Some(id)
            });
            if let Some(l) = id.and_then(|id| labels(&ghost, map, st.settings.units).into_iter().find(|l| l.id == id)) {
                let color = ui.visuals().text_color();
                draw_label(painter, &l, color, text_bg, color, Stroke::new(1.0, accent), 0.55);
            }
        }
    }
    // The preview of the draw tool, with the snap and the constraints that
    // the drawing help adds.
    if let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p)) {
        let no_help = ui.input(|i| i.modifiers.alt) || !e.auto;
        let draws = !e.tool.is_edit()
            && !matches!(
                e.tool,
                SketchTool::Select | SketchTool::Dimension | SketchTool::Constrain(_)
            );
        let mut snap = snap_at(st, map, hover, no_help || !draws).unwrap_or(Snap::free(Point::new(0.0, 0.0)));
        if e.tool == SketchTool::TangentCircle
            && let Some((c, q)) = s
                .nearest_curve(obj.to_sketch(map.to_design(hover)), pick_mm(map), &[])
                .filter(|(c, _)| is_line(s, *c))
        {
            snap = Snap::on_curve(q, c);
        }
        let mut hints = Vec::new();
        if draws
            && e.tool == SketchTool::Line
            && !e.clicks.is_empty()
            && let Some((end, h)) = line_end(st, &snap, no_help)
        {
            snap = end;
            hints = h;
        }
        let pos = if draws { to(snap.at) } else { hover };
        let pts: Vec<Pos2> = e.clicks.iter().map(|c| to(c.at)).collect();
        let pv = Stroke::new(1.2, accent);
        let faint = Stroke::new(1.0, accent.gamma_multiply(0.5));
        if draws {
            // The lines to the points that the pointer is in line with.
            for (axis, p) in snap.align.iter().enumerate() {
                if let Some(q) = p.and_then(|p| s.point(p)) {
                    let from = to(q);
                    let dash = if axis == 0 {
                        [Pos2::new(from.x, from.y), Pos2::new(from.x, pos.y)]
                    } else {
                        [Pos2::new(from.x, from.y), Pos2::new(pos.x, from.y)]
                    };
                    painter.extend(Shape::dashed_line(
                        &dash,
                        Stroke::new(1.0, accent.gamma_multiply(0.7)),
                        4.0,
                        4.0,
                    ));
                }
            }
            // The place of a snap: a diamond at the middle of a line, an X
            // where two curves cross.
            let mark = Stroke::new(1.5, accent);
            if snap.midpoint {
                let k = 6.0;
                let d = [
                    pos + Vec2::new(0.0, -k),
                    pos + Vec2::new(k, 0.0),
                    pos + Vec2::new(0.0, k),
                    pos + Vec2::new(-k, 0.0),
                ];
                painter.add(Shape::closed_line(d.to_vec(), mark));
            } else if snap.also.is_some() {
                let k = 5.0;
                painter.line_segment([pos + Vec2::new(-k, -k), pos + Vec2::new(k, k)], mark);
                painter.line_segment([pos + Vec2::new(-k, k), pos + Vec2::new(k, -k)], mark);
            }
            // The clicks so far.
            for q in &pts {
                painter.circle_filled(*q, 3.5, accent);
            }
            // The shape that the next click makes (a drag from the end of a
            // line: its tangent arc).
            let (tool, clicks) = match e.arc_drag {
                Some(from) => (SketchTool::ArcTangent, vec![from]),
                None => (e.tool, e.clicks.clone()),
            };
            let need = clicks_needed(tool, &MakeOpts::of(&e));
            let mut all = clicks;
            all.push(snap);
            if tool == SketchTool::Line {
                if let [.., last] = pts.as_slice() {
                    painter.line_segment([*last, pos], pv);
                }
            } else if tool == SketchTool::Spline && all.len() >= 2 {
                // The spline through the clicks and the pointer.
                let mut ghost = Sketch::new();
                let at: Vec<Point> = all.iter().map(|c| c.at).collect();
                let t = obj.placement();
                for c in ghost.add_spline(&at, false).unwrap_or_default() {
                    if let Some(path) = ghost.curve_path(c) {
                        draw_path(painter, map, &path.transformed(&t), pv, false);
                    }
                }
            } else if all.len() >= need && need != usize::MAX {
                let mut ghost = s.clone();
                let before: std::collections::BTreeSet<CurveId> = s.curves.iter().map(|c| c.id).collect();
                make(tool, &mut ghost, &all, MakeOpts::of(&e));
                let t = obj.placement();
                for c in ghost.curves.iter().filter(|c| !before.contains(&c.id)) {
                    if let Some(path) = ghost.curve_path(c.id) {
                        let stroke = if c.construction { faint } else { pv };
                        draw_path(painter, map, &path.transformed(&t), stroke, c.construction);
                    }
                }
            } else if let [first, ..] = pts.as_slice() {
                // The first click of a tool of three clicks.
                match e.tool {
                    SketchTool::ArcCenter => {
                        painter.circle_stroke(*first, first.distance(pos), faint);
                        painter.line_segment([*first, pos], pv);
                    }
                    _ => {
                        painter.line_segment([*first, pos], pv);
                    }
                }
            }
        }
        if e.tool != SketchTool::Select
            && let Some(Pick::Point(p)) = pick_at(st, map, hover)
            && let Some(q) = s.point(p)
        {
            painter.circle_stroke(to(q), 7.0, Stroke::new(1.5, accent));
        }
        // The marks of the constraints that the click adds.
        if draws {
            let mut marks: Vec<&str> = hints.iter().map(|h| h.mark()).collect();
            marks.extend(snap.marks());
            if !marks.is_empty() {
                let text = marks.join(" ");
                let at = pos + Vec2::new(14.0, 14.0);
                let rect = Rect::from_min_size(at, Vec2::new(8.0 * text.chars().count() as f32 + 6.0, 16.0));
                painter.rect_filled(rect, 3.0, accent.gamma_multiply(0.8));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(12.0),
                    Color32::BLACK,
                );
            }
        }
    }
    // Stretch: the curves that the points take with them.
    if e.tool == SketchTool::Stretch
        && let Some(b) = e.clicks.first()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let at = infer::snap(s, obj.to_sketch(map.to_design(hover)), pick_mm(map), &[]).at;
        let d = at - b.at;
        let mut ghost = s.clone();
        let _ = ghost.transform_parts(&[], &e.source_points, &|p| p + d, 1.0, false, false);
        let t = obj.placement();
        painter.circle_filled(to(b.at), 3.5, accent);
        for c in ghost
            .curves
            .iter()
            .filter(|c| c.points().iter().any(|p| e.source_points.contains(p)))
        {
            if let Some(path) = ghost.curve_path(c.id) {
                draw_path(
                    painter,
                    map,
                    &path.transformed(&t),
                    Stroke::new(1.2, accent.gamma_multiply(0.7)),
                    true,
                );
            }
        }
    }
    // Move, Copy, Rotate and Scale: the parts where the click puts them.
    if e.tool.is_transform()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let at = infer::snap(s, obj.to_sketch(map.to_design(hover)), pick_mm(map), &[]).at;
        let v = &e.values;
        let m = match (e.tool, e.clicks.first()) {
            (SketchTool::Move | SketchTool::Copy, Some(b)) => Some(Motion::Shift(at - b.at)),
            (SketchTool::Rotate, _) => Some(Motion::Turn(at, v.rotate_deg)),
            (SketchTool::Scale, _) => Some(Motion::Grow(at, v.scale)),
            _ => None,
        };
        if let Some(b) = e.clicks.first() {
            painter.circle_filled(to(b.at), 3.5, accent);
            painter.line_segment([to(b.at), to(at)], Stroke::new(1.0, accent.gamma_multiply(0.6)));
        }
        if let Some(m) = m {
            let mut ghost = s.clone();
            let _ = ghost.transform_parts(&e.source, &e.source_points, &|p| m.apply(p), 1.0, false, true);
            let before: std::collections::BTreeSet<CurveId> = s.curves.iter().map(|c| c.id).collect();
            let t = obj.placement();
            for c in ghost.curves.iter().filter(|c| !before.contains(&c.id)) {
                if let Some(path) = ghost.curve_path(c.id) {
                    draw_path(
                        painter,
                        map,
                        &path.transformed(&t),
                        Stroke::new(1.2, accent.gamma_multiply(0.7)),
                        true,
                    );
                }
            }
        }
    }
    if let Some(start) = e.boxsel
        && let Some(now) = ui.ctx().pointer_hover_pos()
    {
        let r = Rect::from_two_pos(start, now);
        painter.rect_filled(r, 0.0, accent.gamma_multiply(0.08));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Middle);
    }
    value_editor(ui, map, st, obj);
    parameters_window(ui.ctx(), st);
}
