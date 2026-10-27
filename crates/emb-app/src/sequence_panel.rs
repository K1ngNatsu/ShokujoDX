//! Sequence panel: color blocks and objects in sew order.
//!
//! - Each color block is a card with a header (color, thread, object count)
//!   and one row per object (thumbnail, number, stitch type).
//! - Click selects; Ctrl+click adds or removes; Shift+click selects a range.
//!   Click a block header to select the full block.
//! - Drag rows or block headers to change the sew order. A line shows where
//!   they will go. A drag of a selected row moves the full selection.
//! - The toolbar shows the color-change count and offers to remove the
//!   avoidable ones without changing what is on top of what.
//!
//! Each change is one undo step.

use eframe::egui::{self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use emb_geom::Transform;
use emb_model::{Design, DesignObject, ObjectId, ObjectKind, Step};

use crate::a11y::Named;
use crate::app::{AppState, SEQ_BACK, SEQ_END, SEQ_FORWARD, SEQ_START};
use crate::canvas::color32;
use crate::i18n::tr;

const HEADER_H: f32 = 26.0;
const ROW_H: f32 = 34.0;
const THUMB: f32 = 28.0;

/// The objects that a drag moves, in sew order.
#[derive(Debug, Clone)]
struct DragPayload(Vec<ObjectId>);

/// A place where a drop can go: the drop goes before `before` (or at the end
/// if `None`) when the pointer is above `y_mid`.
struct Slot {
    y_mid: f32,
    line_y: f32,
    before: Option<ObjectId>,
}

pub fn show(ui: &mut egui::Ui, st: &mut AppState) {
    // A clone of `Design` is cheap (objects are shared), and it lets the
    // rows read the design while the code changes `st`.
    let design = st.doc.design().clone();
    toolbar(ui, st, &design);
    ui.separator();

    if design.objects().is_empty() {
        ui.weak(tr(
            "No objects yet. Draw them with the Digitize tab, add lettering, or open a design.",
        ));
        return;
    }

    let mut slots: Vec<Slot> = Vec::new();
    let mut hovered_now = None;
    let blocks = design.color_blocks();
    let scroll = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (bi, (thread_id, ids)) in blocks.iter().enumerate() {
            let first = ids[0];
            let collapsed = st.seq_collapsed.contains(&first);
            // Other blocks with the same thread: a hint that a merge is possible.
            let same_thread: Vec<usize> = blocks
                .iter()
                .enumerate()
                .filter(|(j, (t, _))| *j != bi && t == thread_id)
                .map(|(j, _)| j + 1)
                .collect();

            let header = block_header(ui, st, &design, bi, *thread_id, ids, collapsed, &same_thread);
            slots.push(Slot {
                y_mid: header.center().y,
                line_y: header.top() - 1.0,
                before: Some(first),
            });
            if !collapsed {
                for id in ids {
                    let Some(index) = design.index_of(*id) else { continue };
                    let obj = &design.objects()[index];
                    let row = object_row(ui, st, &design, obj, index);
                    if row.1 {
                        hovered_now = Some(*id);
                    }
                    slots.push(Slot {
                        y_mid: row.0.center().y,
                        line_y: row.0.top() - 1.0,
                        before: Some(*id),
                    });
                }
            }
            ui.add_space(4.0);
        }
        let end = ui.min_rect().bottom();
        slots.push(Slot {
            y_mid: f32::INFINITY,
            line_y: end,
            before: None,
        });
    });
    st.hovered = hovered_now;

    handle_drop(ui, st, scroll.inner_rect, &slots);
}

fn toolbar(ui: &mut egui::Ui, st: &mut AppState, design: &Design) {
    let blocks = design.color_blocks().len();
    let avoidable = design.avoidable_color_changes();
    ui.horizontal_wrapped(|ui| {
        ui.label(crate::i18n::fill(
            "{} objects · {} color change{}",
            &[
                &(design.objects().len()),
                &(blocks.saturating_sub(1)),
                &(if blocks == 2 { "" } else { "s" }),
            ],
        ));
        if avoidable > 0
            && ui
                .button(crate::i18n::fill("Merge colors ({})", &[&avoidable]))
                .on_hover_text(tr("Same-color blocks can join. Optimize moves objects only where no \
                     overlapping object is in the way, so what is on top stays on top."))
                .clicked()
        {
            st.resequence("Optimize color changes", Design::optimize_color_changes);
        }
        if design.objects().len() > 1
            && ui
                .button(tr("Shortest Order"))
                .on_hover_text(tr(
                    "Sew the objects of each color nearest first: less travel and fewer trims. \
                     Keeps the colors and what is on top.",
                ))
                .clicked()
        {
            st.optimize_sew_order();
        }
    });
    ui.horizontal(|ui| {
        let has_sel = !st.selection.is_empty();
        let fmt = |s| ui.ctx().format_shortcut(s);
        let buttons = [
            ("⏮", tr("Move to start (sew first)"), fmt(&SEQ_START)),
            ("⏶", tr("Move Back One"), fmt(&SEQ_BACK)),
            ("⏷", tr("Move Forward One"), fmt(&SEQ_FORWARD)),
            ("⏭", tr("Move to end (sew last, on top)"), fmt(&SEQ_END)),
        ];
        for (i, (icon, tip, key)) in buttons.into_iter().enumerate() {
            let r = ui
                .add_enabled(has_sel, egui::Button::new(icon).min_size(Vec2::new(26.0, 20.0)))
                .on_hover_text(format!("{}  ({key})", tr(tip)))
                .named(tr(tip));
            if r.clicked() {
                match i {
                    0 => st.sequence_selected("Move to Start", |d, ids| d.move_objects_to_start(ids)),
                    1 => st.sequence_selected("Move Back One", |d, ids| d.step_objects(ids, Step::Backward)),
                    2 => st.sequence_selected("Move Forward One", |d, ids| d.step_objects(ids, Step::Forward)),
                    _ => st.sequence_selected("Move to End", |d, ids| d.move_objects_to_end(ids)),
                }
            }
        }
        if ui
            .add_enabled(st.selection.len() > 1, egui::Button::new(tr("Click order")))
            .on_hover_text(tr("Sew the selected objects in the order you Ctrl+clicked them"))
            .clicked()
        {
            let ids = st.selection.ids().to_vec();
            st.resequence("Sequence by Click Order", |d| d.sequence_in_order(&ids));
        }
    });
}

/// Handles a click on a row or header. `ids` are the objects that the row stands for.
fn click_select(ui: &egui::Ui, st: &mut AppState, design: &Design, ids: &[ObjectId]) {
    let mods = ui.input(|i| i.modifiers);
    if mods.command {
        let all_selected = ids.iter().all(|id| st.selection.contains(*id));
        for id in ids {
            if all_selected || !st.selection.contains(*id) {
                st.selection.toggle(*id);
            }
        }
    } else if mods.shift && ids.len() == 1 {
        st.selection.extend_to(design, ids[0]);
    } else {
        st.selection.set_all(ids.iter().copied());
    }
}

/// Starts a drag. A drag of a selected row moves the full selection.
fn start_drag(st: &mut AppState, design: &Design, ids: &[ObjectId], resp: &egui::Response) {
    if !resp.drag_started() {
        return;
    }
    let moving = if ids.iter().all(|id| st.selection.contains(*id)) {
        st.selection.in_sew_order(design)
    } else {
        st.selection.set_all(ids.iter().copied());
        ids.to_vec()
    };
    egui::DragAndDrop::set_payload(&resp.ctx, DragPayload(moving));
}

#[allow(clippy::too_many_arguments)]
fn block_header(
    ui: &mut egui::Ui,
    st: &mut AppState,
    design: &Design,
    block_index: usize,
    thread_id: emb_model::ThreadId,
    ids: &[ObjectId],
    collapsed: bool,
    same_thread: &[usize],
) -> Rect {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, HEADER_H), Sense::click_and_drag());
    let arrow_rect = Rect::from_min_size(rect.min, Vec2::new(22.0, HEADER_H));
    let arrow = ui.interact(arrow_rect, resp.id.with("arrow"), Sense::click());

    let thread = design.thread(thread_id);
    let all_selected = ids.iter().all(|id| st.selection.contains(*id));
    let v = ui.visuals();
    let bg = if all_selected {
        v.selection.bg_fill.gamma_multiply(0.6)
    } else if resp.hovered() {
        v.widgets.hovered.bg_fill
    } else {
        v.faint_bg_color
    };
    let p = ui.painter();
    p.rect_filled(rect, 4.0, bg);
    // Color bar at the left edge, the main visual key for the block.
    let color = thread.map(|t| color32(t.color)).unwrap_or(Color32::GRAY);
    p.rect_filled(Rect::from_min_size(rect.min, Vec2::new(4.0, HEADER_H)), 2.0, color);
    let text_color = v.text_color();
    p.text(
        arrow_rect.center() + Vec2::new(2.0, 0.0),
        Align2::CENTER_CENTER,
        if collapsed { "▶" } else { "▼" },
        FontId::proportional(10.0),
        text_color,
    );
    let swatch = Rect::from_center_size(Pos2::new(rect.left() + 32.0, rect.center().y), Vec2::splat(14.0));
    p.rect_filled(swatch, 3.0, color);
    p.rect_stroke(
        swatch,
        3.0,
        Stroke::new(1.0, text_color.gamma_multiply(0.4)),
        egui::StrokeKind::Outside,
    );
    p.text(
        Pos2::new(rect.left() + 46.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!(
            "{}. {}",
            block_index + 1,
            thread.map(|t| t.name.as_str()).unwrap_or("?")
        ),
        FontId::proportional(13.0),
        text_color,
    );
    let mut right = format!("{} obj", ids.len());
    if !same_thread.is_empty() {
        right = format!("↻ {right}");
    }
    p.text(
        Pos2::new(rect.right() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        right,
        FontId::proportional(12.0),
        v.weak_text_color(),
    );

    let resp = if same_thread.is_empty() {
        resp
    } else {
        let blocks: Vec<String> = same_thread.iter().map(|b| b.to_string()).collect();
        resp.on_hover_text(crate::i18n::fill(
            "This color also sews in block {}. Merge colors can join them when nothing overlaps.",
            &[&(blocks.join(", "))],
        ))
    };

    if arrow.clicked() {
        if collapsed {
            st.seq_collapsed.remove(&ids[0]);
        } else {
            st.seq_collapsed.insert(ids[0]);
        }
    } else if resp.clicked() {
        click_select(ui, st, design, ids);
    }
    start_drag(st, design, ids, &resp);
    // Right click: select the block (unless it is already selected), then the object menu.
    if resp.secondary_clicked() && !all_selected {
        st.selection.set_all(ids.iter().copied());
    }
    resp.context_menu(|ui| crate::commands::object_menu(ui, st));
    if resp.hovered() && !resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    rect
}

/// Draws one object row. Returns its rect and whether the pointer is over it.
fn object_row(ui: &mut egui::Ui, st: &mut AppState, design: &Design, obj: &DesignObject, index: usize) -> (Rect, bool) {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click_and_drag());
    let selected = st.selection.contains(obj.id);
    let v = ui.visuals();
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 3.0, v.selection.bg_fill);
    } else if resp.hovered() {
        p.rect_filled(rect, 3.0, v.widgets.hovered.bg_fill);
    }
    let color = design
        .thread(obj.thread)
        .map(|t| color32(t.color))
        .unwrap_or(Color32::GRAY);
    let thumb = Rect::from_min_size(
        Pos2::new(rect.left() + 18.0, rect.center().y - THUMB / 2.0),
        Vec2::splat(THUMB),
    );
    p.rect_filled(thumb, 3.0, v.extreme_bg_color);
    paint_thumbnail(p, thumb.shrink(3.0), obj, color);

    let text_color = if selected {
        v.selection.stroke.color
    } else {
        v.text_color()
    };
    p.text(
        Pos2::new(thumb.right() + 8.0, rect.center().y - 7.0),
        Align2::LEFT_CENTER,
        format!(
            "#{}  {}",
            index + 1,
            obj.name.clone().unwrap_or_else(|| obj.type_name().to_string())
        ),
        FontId::proportional(13.0),
        text_color,
    );
    p.text(
        Pos2::new(thumb.right() + 8.0, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        detail(obj),
        FontId::proportional(11.0),
        v.weak_text_color(),
    );
    // Hidden on screen (Alt+V, Hide others, view by color): it still sews.
    if !st.view_filter.shows(obj) {
        p.text(
            Pos2::new(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            egui_phosphor::regular::EYE_SLASH,
            FontId::proportional(14.0),
            v.weak_text_color(),
        );
    }

    if resp.clicked() {
        click_select(ui, st, design, &[obj.id]);
    }
    start_drag(st, design, &[obj.id], &resp);
    if resp.secondary_clicked() && !selected {
        st.selection.set(obj.id);
    }
    resp.context_menu(|ui| crate::commands::object_menu(ui, st));
    (rect, resp.hovered())
}

/// A short description of the stitch settings and machine functions, for
/// the second line of a row.
fn detail(obj: &DesignObject) -> String {
    let f = crate::functions::summary(&obj.functions);
    let s = stitch_detail(obj);
    if f.is_empty() { s } else { format!("{s} · {f}") }
}

fn stitch_detail(obj: &DesignObject) -> String {
    match &obj.kind {
        ObjectKind::Running { params, .. } => match (&params.border, &params.motif) {
            (Some(b), _) => format!("{} line · {:.1} mm wide", b.column.pattern.name(), b.width_mm),
            (None, Some(_)) => "Motif run".into(),
            (None, None) if params.cut => format!("Cut line · {:.1} mm", params.stitch_length_mm),
            (None, None) => {
                let kind = if params.repeats >= 3 { "Bean stitch" } else { "Running" };
                format!("{kind} · {:.1} mm", params.stitch_length_mm)
            }
        },
        ObjectKind::Satin { params, .. } => {
            format!("{} column · {:.2} mm spacing", params.pattern.name(), params.spacing_mm)
        }
        ObjectKind::Lettering(l) => format!("Lettering · {}", crate::lettering::summary(l)),
        ObjectKind::CrossStitch(x) => crate::cross::detail(x),
        ObjectKind::Bling(b) => crate::bling::detail(b),
        ObjectKind::Sketch(s) => crate::sketch::detail(s),
        ObjectKind::Fill { params, holes, .. } => {
            let h = if holes.is_empty() {
                String::new()
            } else {
                format!(" · {} hole{}", holes.len(), if holes.len() == 1 { "" } else { "s" })
            };
            // The curved fills have no row angle.
            use emb_model::FillPattern as F;
            let curved = matches!(
                params.pattern,
                F::Contour | F::Offset | F::Spiral | F::IslandCoil | F::Radial { .. } | F::Maze | F::Stipple { .. }
            );
            if curved {
                format!("{}{h}", params.pattern.name())
            } else {
                format!("{} · {:.0}°{h}", params.pattern.name(), params.angle_deg)
            }
        }
        ObjectKind::Stitches { path, .. } => {
            let n: usize = emb_stitch::stitch_subpaths(path).iter().map(Vec::len).sum();
            format!("Stitches · {n} points")
        }
        ObjectKind::Vector { path, .. } => {
            let n = path.subpaths().len();
            format!("Artwork · {n} path{} · no stitches", if n == 1 { "" } else { "s" })
        }
        ObjectKind::Applique { pieces, params } => {
            let n = pieces.len();
            let stops = usize::from(params.placement || params.tack_down != emb_model::TackDown::None)
                + usize::from(params.trim_stop && params.cover);
            format!(
                "Appliqué · {n} piece{} · {stops} stop{}",
                if n == 1 { "" } else { "s" },
                if stops == 1 { "" } else { "s" }
            )
        }
    }
}

/// The lines of the object geometry scaled to fit `area`, as segments.
/// Points nearer than half a pixel are one point. (A joined line of a
/// traced outline has very sharp turns, and egui draws a long point at
/// each sharp turn, out of the box.)
fn thumbnail_segments(area: Rect, obj: &DesignObject) -> Vec<[Pos2; 2]> {
    let Some(bb) = obj.control_bbox() else {
        return Vec::new();
    };
    let size = bb.width().max(bb.height()).max(1e-6);
    let scale = f64::from(area.width()) / size;
    let c = bb.center();
    let t = Transform::translate(-c.x, -c.y).then(Transform::scale(scale, scale));
    let center = area.center();
    let tol = size / 60.0;
    let mut out = Vec::new();
    for path in obj.kind.paths() {
        for pl in path.flatten(tol) {
            let mut pts: Vec<Pos2> = Vec::with_capacity(pl.points.len() + 1);
            let all = pl.points.iter().chain(pl.closed.then(|| &pl.points[0]));
            for q in all {
                let s = t.apply(*q);
                let p = area.clamp(Pos2::new(center.x + s.x as f32, center.y + s.y as f32));
                if pts.last().is_none_or(|l| l.distance(p) >= 0.5) {
                    pts.push(p);
                }
            }
            out.extend(pts.windows(2).map(|w| [w[0], w[1]]));
        }
    }
    out
}

/// Draws the object geometry scaled to fit `area`, clipped to it.
fn paint_thumbnail(p: &egui::Painter, area: Rect, obj: &DesignObject, color: Color32) {
    let p = p.with_clip_rect(area.expand(1.0));
    let stroke = Stroke::new(1.5, color);
    for seg in thumbnail_segments(area, obj) {
        p.line_segment(seg, stroke);
    }
}

/// Draws the drop line during a drag and applies the move on release.
fn handle_drop(ui: &mut egui::Ui, st: &mut AppState, area: Rect, slots: &[Slot]) {
    let ctx = ui.ctx().clone();
    let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(&ctx) else {
        return;
    };
    ctx.set_cursor_icon(CursorIcon::Grabbing);
    let Some(pos) = ctx.pointer_latest_pos() else { return };
    let released = ctx.input(|i| i.pointer.any_released());
    if !area.contains(pos) {
        if released {
            egui::DragAndDrop::clear_payload(&ctx);
        }
        return;
    }
    let Some(slot) = slots.iter().find(|s| pos.y < s.y_mid) else {
        return;
    };
    let accent = ui.visuals().selection.stroke.color;
    let p = ui.painter().with_clip_rect(area);
    p.line_segment(
        [
            Pos2::new(area.left() + 4.0, slot.line_y),
            Pos2::new(area.right() - 4.0, slot.line_y),
        ],
        Stroke::new(2.5, accent),
    );
    p.circle_filled(Pos2::new(area.left() + 6.0, slot.line_y), 4.0, accent);
    let n = payload.0.len();
    p.text(
        pos + Vec2::new(14.0, 10.0),
        Align2::LEFT_TOP,
        if n == 1 {
            "Move 1 object".to_string()
        } else {
            format!("Move {n} objects")
        },
        FontId::proportional(12.0),
        accent,
    );

    if released {
        egui::DragAndDrop::clear_payload(&ctx);
        let ids = payload.0.clone();
        let before = slot.before;
        st.resequence("Change sew order", |d| d.move_objects_before(&ids, before));
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Modifiers, PointerButton, RawInput};
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn thumbnails_stay_in_their_boxes() {
        // A traced outline with very sharp turns and points on top of each other.
        let mut pts = Vec::new();
        for i in 0..60 {
            let x = f64::from(i);
            pts.push(emb_geom::Point::new(x, 0.0));
            pts.push(emb_geom::Point::new(x + 0.001, if i % 2 == 0 { 40.0 } else { 39.999 }));
            pts.push(emb_geom::Point::new(x + 0.001, 40.0));
        }
        let mut d = Design::new("t");
        let id = d.add_object(
            emb_model::ThreadId(0),
            ObjectKind::Fill {
                outer: emb_geom::Path::polygon(&pts),
                holes: Vec::new(),
                params: Default::default(),
            },
        );
        let area = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::splat(28.0));
        let segs = thumbnail_segments(area, d.object(id).unwrap());
        assert!(segs.len() > 10);
        for [a, b] in &segs {
            assert!(area.contains(*a) && area.contains(*b), "{a:?} {b:?}");
            assert!(a.distance(*b) >= 0.5);
        }
    }

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn row_names_the_stitch_type_of_the_object() {
        let st = state();
        let d = st.doc.design();
        let fill = d
            .objects()
            .iter()
            .find(|o| matches!(o.kind, ObjectKind::Fill { .. }))
            .unwrap();
        let mut o = (**fill).clone();
        assert!(stitch_detail(&o).starts_with("Tatami · "), "{}", stitch_detail(&o));
        if let ObjectKind::Fill { params, .. } = &mut o.kind {
            *params = params.with_pattern(emb_model::FillPattern::Radial { center: None });
        }
        assert!(stitch_detail(&o).starts_with("Radial"), "{}", stitch_detail(&o));
        assert!(!stitch_detail(&o).contains('°'), "a radial fill has no row angle");
        if let ObjectKind::Fill { params, .. } = &mut o.kind {
            *params = params.with_pattern(emb_model::FillPattern::Square);
        }
        assert!(stitch_detail(&o).starts_with("Square · "), "{}", stitch_detail(&o));
        let satin = d
            .objects()
            .iter()
            .find(|o| matches!(o.kind, ObjectKind::Satin { .. }))
            .unwrap();
        let mut o = (**satin).clone();
        if let ObjectKind::Satin { params, .. } = &mut o.kind {
            params.pattern = emb_model::ColumnPattern::Zigzag;
        }
        assert!(stitch_detail(&o).starts_with("Zigzag column"), "{}", stitch_detail(&o));
    }

    /// Runs one headless frame. The texture output is cleared, as a real
    /// backend would apply it.
    fn run(ctx: &egui::Context, input: RawInput, ui_fn: impl FnMut(&mut egui::Ui)) {
        let mut out = ctx.run_ui(input, ui_fn);
        out.textures_delta.clear();
    }

    /// Runs one headless frame with the panel at the full screen.
    fn frame(ctx: &egui::Context, st: &mut AppState, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 900.0))),
            events,
            ..Default::default()
        };
        run(ctx, input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(ui, st));
        });
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    /// Moves the pointer down the panel and returns the y where `id` is hovered.
    fn find_row(ctx: &egui::Context, st: &mut AppState, id: ObjectId) -> f32 {
        let mut y = 0.0;
        while y < 900.0 {
            frame(ctx, st, vec![Event::PointerMoved(Pos2::new(200.0, y))]);
            if st.hovered == Some(id) {
                return y;
            }
            y += 4.0;
        }
        panic!("row for {id:?} not found");
    }

    #[test]
    fn renders_all_panels_without_panic() {
        let mut st = state();
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
                ..Default::default()
            };
            run(&ctx, input, |ui| {
                egui::Panel::right("seq").show(ui, |ui| show(ui, &mut st));
                egui::Panel::left("props").show(ui, |ui| crate::panels::properties(ui, &mut st));
                egui::CentralPanel::default().show(ui, |ui| crate::canvas::show(ui, &mut st));
            });
            // Multi-selection and collapsed blocks too.
            let ids: Vec<ObjectId> = st.doc.design().objects().iter().map(|o| o.id).collect();
            st.selection.set_all(ids.clone());
            st.seq_collapsed.insert(ids[0]);
        }
    }

    #[test]
    fn drag_row_to_top_changes_order_as_one_undo_step() {
        let mut st = state();
        let ctx = egui::Context::default();
        let order = |st: &AppState| st.doc.design().objects().iter().map(|o| o.id).collect::<Vec<_>>();
        let before = order(&st);
        let last = *before.last().unwrap();
        let first = before[0];
        let undo_steps = st.doc.undo_len();

        frame(&ctx, &mut st, vec![]);
        let y_last = find_row(&ctx, &mut st, last);
        let y_first = find_row(&ctx, &mut st, first);

        // Press on the last row, drag up above the first row (into the first block header), release.
        let start = Pos2::new(200.0, y_last);
        frame(&ctx, &mut st, vec![Event::PointerMoved(start), button(start, true)]);
        // `find_row` gives the top edge of a row. The block header is just above
        // the first row; aim for its upper half, so the drop goes before the block.
        let target = y_first - 20.0;
        let mut y = y_last;
        while y > target {
            y = (y - 10.0).max(target);
            frame(&ctx, &mut st, vec![Event::PointerMoved(Pos2::new(200.0, y))]);
        }
        let end = Pos2::new(200.0, target);
        frame(&ctx, &mut st, vec![button(end, false)]);
        frame(&ctx, &mut st, vec![]);

        let after = order(&st);
        assert_eq!(after[0], last, "dragged object sews first: {after:?}");
        assert_eq!(&after[1..], &before[..before.len() - 1]);
        assert_eq!(st.doc.undo_len(), undo_steps + 1);
        assert_eq!(st.doc.undo_label(), Some("Change sew order"));
        st.undo();
        assert_eq!(order(&st), before);
    }

    #[test]
    fn click_selects_and_ctrl_click_adds() {
        let mut st = state();
        let ctx = egui::Context::default();
        let ids: Vec<ObjectId> = st.doc.design().objects().iter().map(|o| o.id).collect();
        frame(&ctx, &mut st, vec![]);
        let y1 = find_row(&ctx, &mut st, ids[1]);
        let p = Pos2::new(200.0, y1);
        frame(&ctx, &mut st, vec![button(p, true)]);
        frame(&ctx, &mut st, vec![button(p, false)]);
        assert_eq!(st.selection.ids(), &[ids[1]]);

        let y3 = find_row(&ctx, &mut st, ids[3]);
        let p = Pos2::new(200.0, y3);
        let ctrl = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::COMMAND,
        };
        let input = |events| RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 900.0))),
            events,
            ..Default::default()
        };
        let held = Event::ModifiersChanged(Modifiers::COMMAND);
        for e in [vec![held.clone(), ctrl(true)], vec![held.clone(), ctrl(false)]] {
            run(&ctx, input(e), |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(ui, &mut st));
            });
        }
        assert_eq!(st.selection.ids(), &[ids[1], ids[3]]);
    }
}
