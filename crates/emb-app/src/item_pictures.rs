//! Pictures of the library items in the lists (the review of P17, lane 3):
//! a program-split pattern as its needle points, a carving stamp as its
//! lines, and a decorative border as its stitches round a small box. A
//! list of 48 patterns is hard to use by names only.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use emb_geom::{BBox, Point};
use emb_model::{Design, Frame, Rgb, SplitPattern, Stamp, StitchKind};

/// The size of the picture of a cell in a list, in screen points.
pub const CELL: Vec2 = Vec2::new(84.0, 52.0);
/// The cells in a row of a list.
pub const COLUMNS: usize = 4;

/// Draws a split pattern in `rect`: the needle points of its rows, at
/// 0.4 mm from one row to the next, `px_per_mm` large.
pub fn pattern(painter: &egui::Painter, rect: Rect, p: &SplitPattern, px_per_mm: f32, ink: Color32) {
    if p.rows.is_empty() || p.width_mm <= 0.0 {
        return;
    }
    let (tile, step) = (p.width_mm as f32 * px_per_mm, 0.4 * px_per_mm);
    let radius = (0.16 * px_per_mm).clamp(0.7, 1.4);
    let mut row = 0;
    while (row as f32) * step < rect.height() {
        let y = rect.top() + row as f32 * step;
        let points = &p.rows[row % p.rows.len()];
        let mut k = 0.0_f32;
        while k * tile < rect.width() {
            for f in points {
                let x = rect.left() + (k + *f as f32) * tile;
                if x <= rect.right() {
                    painter.circle_filled(Pos2::new(x, y), radius, ink);
                }
            }
            k += 1.0;
        }
        row += 1;
    }
}

/// Draws the lines of a stamp in the middle of `rect`.
pub fn stamp(painter: &egui::Painter, rect: Rect, s: &Stamp, ink: Color32) {
    let side = rect.width().min(rect.height());
    let at = |q: &Point| rect.center() + Vec2::new(q.x as f32 * side, q.y as f32 * side);
    for line in &s.lines {
        painter.add(egui::Shape::line(line.iter().map(at).collect(), Stroke::new(1.3, ink)));
    }
}

/// The stitches of a border as lines (one for each run of stitches), in mm.
pub type Lines = Arc<Vec<Vec<Point>>>;

/// The stitches of the borders, made one time for each border.
#[derive(Debug, Clone, Default)]
pub struct FramePictures {
    made: HashMap<String, Lines>,
}

impl FramePictures {
    /// The size of the box that the border of a picture goes round, in mm.
    const BOX_MM: (f64, f64) = (34.0, 20.0);
    /// The borders that are made in one frame of the program, so the list
    /// opens at once.
    pub const IN_A_FRAME: usize = 6;

    /// The stitches of `frame` round a small box, if they are made.
    pub fn get(&self, frame: &Frame) -> Option<Lines> {
        self.made.get(&frame.name).cloned()
    }

    /// Makes the stitches of `frame` round a small box (the stitch engine
    /// sews the objects of the border).
    pub fn make(&mut self, frame: &Frame) -> Lines {
        if let Some(lines) = self.made.get(&frame.name) {
            return lines.clone();
        }
        let mut d = Design::new("");
        let t = d.add_thread("Border", Rgb(0, 0, 0));
        let b = BBox::from_center_size(Point::ZERO, Self::BOX_MM.0, Self::BOX_MM.1);
        for kind in emb_model::decor::frame_objects(frame, &b, 2.0) {
            d.add_object(t, kind);
        }
        let plan = emb_stitch::build_plan(&d, &emb_stitch::PlanOptions::default(), &mut Default::default());
        let mut lines: Vec<Vec<Point>> = vec![Vec::new()];
        for r in &plan.stitches.records {
            if r.kind == StitchKind::Stitch {
                lines.last_mut().expect("one line").push(r.pos);
            } else if lines.last().is_some_and(|l| !l.is_empty()) {
                lines.push(Vec::new());
            }
        }
        lines.retain(|l| l.len() >= 2);
        let lines = Arc::new(lines);
        self.made.insert(frame.name.clone(), lines.clone());
        lines
    }
}

/// Draws the stitches of a border in `rect`.
pub fn border(painter: &egui::Painter, rect: Rect, lines: &[Vec<Point>], ink: Color32) {
    let Some(b) = BBox::from_points(lines.iter().flatten().copied()) else {
        return;
    };
    let scale = (f64::from(rect.width()) / b.width().max(1.0)).min(f64::from(rect.height()) / b.height().max(1.0));
    let c = b.center();
    let at = |q: &Point| rect.center() + Vec2::new(((q.x - c.x) * scale) as f32, ((q.y - c.y) * scale) as f32);
    for line in lines {
        painter.add(egui::Shape::line(line.iter().map(at).collect(), Stroke::new(0.8, ink)));
    }
}

/// One cell of a picture list: the picture that `draw` paints (in the
/// rectangle and the color that it gets), and the name under it. A click
/// on the picture or on the name is a click on the cell.
pub fn cell(
    ui: &mut egui::Ui,
    selected: bool,
    name: &str,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    ui.vertical(|ui| {
        ui.set_max_width(CELL.x);
        let (rect, picture) = ui.allocate_exact_size(CELL, Sense::click());
        let visuals = ui.visuals();
        let (accent, ink) = (visuals.selection.bg_fill, visuals.text_color());
        ui.painter().rect_filled(rect, 3.0, visuals.extreme_bg_color);
        if selected || picture.hovered() {
            let width = if selected { 2.0 } else { 1.0 };
            ui.painter()
                .rect_stroke(rect, 3.0, Stroke::new(width, accent), egui::StrokeKind::Inside);
        }
        draw(&ui.painter().with_clip_rect(rect), rect.shrink(5.0), ink);
        let picture = crate::a11y::named(picture, name);
        let label = ui.add(egui::Button::selectable(selected, egui::RichText::new(name).small()).truncate());
        picture.union(label)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stitches_of_each_border_are_made_one_time() {
        let mut pictures = FramePictures::default();
        for f in emb_model::decor::frames() {
            assert!(pictures.get(&f).is_none());
            let lines = pictures.make(&f);
            assert!(!lines.is_empty(), "{}", f.name);
            // The border goes round the box.
            let b = BBox::from_points(lines.iter().flatten().copied()).unwrap();
            assert!(b.width() >= 34.0 && b.height() >= 20.0, "{}: {b:?}", f.name);
            assert!(b.width() < 90.0 && b.height() < 80.0, "{}: {b:?}", f.name);
            // The second time it is the same list.
            assert!(Arc::ptr_eq(&lines, &pictures.make(&f)));
            assert!(pictures.get(&f).is_some());
        }
    }

    #[test]
    fn the_pictures_draw() {
        let ctx = crate::test_util::ready_context();
        let mut pictures = FramePictures::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                for p in emb_model::split::split_patterns() {
                    cell(ui, false, &p.name, |painter, rect, ink| {
                        pattern(painter, rect, &p, 4.0, ink)
                    });
                }
                for s in emb_model::split::stamps() {
                    cell(ui, true, &s.name, |painter, rect, ink| stamp(painter, rect, &s, ink));
                }
                for f in emb_model::decor::frames() {
                    let lines = pictures.make(&f);
                    cell(ui, false, &f.name, |painter, rect, ink| {
                        border(painter, rect, &lines, ink)
                    });
                }
            });
        });
        out.textures_delta.clear();
        assert!(!out.shapes.is_empty());
    }
}
