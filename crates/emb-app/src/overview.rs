//! The overview window (roadmap P3, Milestone 4): all of the design and the
//! hoop, small, with a frame around the part that the canvas shows. A click
//! or a drag in it moves the canvas view there.

use eframe::egui::{self, Color32, Rect, Sense, Shape, Stroke, Vec2};

use crate::app::AppState;
use crate::canvas::{CanvasView, stitch_mesh};
use crate::i18n::tr;

const SIZE: Vec2 = Vec2::new(240.0, 180.0);

pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_overview {
        return;
    }
    let mut open = true;
    egui::Window::new(tr("Overview"))
        .id(egui::Id::new("Overview"))
        .open(&mut open)
        .resizable(false)
        .collapsible(true)
        .default_pos(egui::pos2(ctx.content_rect().right() - SIZE.x - 380.0, 160.0))
        .show(ctx, |ui| contents(ui, st));
    if !open {
        st.show_overview = false;
    }
}

/// The view of the overview: everything that "zoom extents" shows.
fn overview_view(st: &AppState, rect: Rect) -> CanvasView {
    let mut v = CanvasView::default();
    if let Some(b) = st.fit_box() {
        v.fit(rect, b);
    }
    v
}

pub fn contents(ui: &mut egui::Ui, st: &mut AppState) {
    let (resp, painter) = ui.allocate_painter(SIZE, Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    let view = overview_view(st, rect);
    let segs = st.shown_plan().segments.clone();
    let (mesh, _) = stitch_mesh(rect, &view, &segs, false);
    painter.with_clip_rect(rect).add(Shape::mesh(mesh));

    // The hoop field.
    if let Some(field) = st.hoop_outline(0.0) {
        for pl in field.flatten(0.5) {
            let mut pts: Vec<egui::Pos2> = pl.points.iter().map(|p| view.to_screen(rect, *p)).collect();
            pts.extend(pts.first().copied());
            painter.add(Shape::line(pts, Stroke::new(1.0, Color32::from_gray(140))));
        }
    }
    // The part that the canvas shows.
    if let Some(canvas) = st.canvas_rect {
        let b = st.view.visible(canvas);
        let r = Rect::from_two_pos(view.to_screen(rect, b.min), view.to_screen(rect, b.max)).intersect(rect);
        let accent = ui.visuals().selection.stroke.color;
        painter.rect(
            r,
            0.0,
            accent.gamma_multiply(0.12),
            Stroke::new(1.5, accent),
            egui::StrokeKind::Middle,
        );
    }
    if (resp.clicked() || resp.dragged())
        && let Some(p) = resp.interact_pointer_pos()
    {
        st.view.center = view.to_mm(rect, p);
    }
    resp.on_hover_text(tr("Click or drag to move the view"));
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
    use emb_geom::Point;
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn a_click_moves_the_view() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st.canvas_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)));
        let ctx = egui::Context::default();
        // Runs one frame; returns the overview area.
        let frame = |st: &mut AppState, events: Vec<Event>| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
                events,
                ..Default::default()
            };
            let mut area = Rect::NOTHING;
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    area = Rect::from_min_size(ui.cursor().min, SIZE);
                    contents(ui, st);
                });
            });
            out.textures_delta.clear();
            area
        };
        let area = frame(&mut st, vec![]);
        let view = overview_view(&st, area);
        let target = Point::new(60.0, 10.0);
        let p = view.to_screen(area, target);
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame(&mut st, vec![Event::PointerMoved(p)]);
        frame(&mut st, vec![ev(true)]);
        frame(&mut st, vec![ev(false)]);
        assert!(st.view.center.distance(target) < 1.0, "{:?}", st.view.center);
    }
}
