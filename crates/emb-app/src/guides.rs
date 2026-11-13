//! Guides and construction marks on the canvas (ADR 0089).
//!
//! - A guide is a line across the canvas from a ruler (drag from the
//!   ruler). Each guide has a marker on its ruler: drag the marker to move
//!   the guide, double-click it to type its place, right-click it for its
//!   menu.
//! - Construction lines (at any angle, through two points) and
//!   construction points come from the tools of the View tab. They do not
//!   sew.
//! - Each guide and mark is dashed (or solid), in the guide color of the
//!   Options or its own color, and can be locked (a drag does not move it).
//!   The menu (right click on a marker, or on the line with the Select
//!   tool) changes the place, the color, the line, the lock; it also
//!   duplicates and deletes. Points snap to the marks and to the places
//!   where they cross, with Snap to Guides.
//!
//! All places are view coordinates in mm (after the design rotation). The
//! values that show and that the user types are from the hoop center, as
//! the rulers show them.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use emb_geom::{Point, math};
use emb_model::{ConstructionMark, Design, Guide, GuideStyle, MarkShape, Rgb};

use crate::app::AppState;
use crate::canvas::color32;
use crate::i18n::tr;
use crate::workspace::{RULER, SnapKind};

/// The pointer must be this near a line or a point, in screen points.
pub const HIT: f32 = 4.0;
/// The pointer must be this near a marker on a ruler, in screen points.
const MARKER_HIT: f32 = 5.0;
/// Half the width of a ruler marker, in screen points.
const MARKER: f32 = 4.5;

/// A guide (by its index in `Design::guides`) or a construction mark (in
/// `Design::marks`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkRef {
    Guide(usize),
    Mark(usize),
}

/// The values of the edit window, as the user types them.
#[derive(Debug, Clone, PartialEq)]
pub struct GuideEdit {
    pub target: MarkRef,
    /// The place (a guide), X and Y (a point), or X, Y and the angle (a line).
    pub text: [String; 3],
}

/// The colors of the Color menu, with their names (the first is the guide
/// color of the Options).
fn colors() -> [(&'static str, Option<Rgb>); 7] {
    [
        (tr("Guide Color"), None),
        (tr("Red"), Some(Rgb(220, 50, 50))),
        (tr("Orange"), Some(Rgb(240, 140, 20))),
        (tr("Green"), Some(Rgb(40, 160, 70))),
        (tr("Blue"), Some(Rgb(40, 90, 220))),
        (tr("Magenta"), Some(Rgb(200, 50, 180))),
        (tr("Gray"), Some(Rgb(130, 130, 130))),
    ]
}

fn style_of(d: &Design, r: MarkRef) -> Option<GuideStyle> {
    match r {
        MarkRef::Guide(i) => d.guides.get(i).map(|g| g.style),
        MarkRef::Mark(i) => d.marks.get(i).map(|m| m.style),
    }
}

fn style_mut(d: &mut Design, r: MarkRef) -> Option<&mut GuideStyle> {
    match r {
        MarkRef::Guide(i) => d.guides.get_mut(i).map(|g| &mut g.style),
        MarkRef::Mark(i) => d.marks.get_mut(i).map(|m| &mut m.style),
    }
}

/// The color of a guide or a mark on the screen.
fn color_of(st: &AppState, style: &GuideStyle) -> Color32 {
    style.color.map_or_else(|| st.settings.display.guides(), color32)
}

/// The two ends of a line across the visible canvas, on the screen.
fn line_ends(st: &AppState, rect: Rect, p: Point, d: Point) -> [Pos2; 2] {
    let vis = st.view.visible(rect);
    let l = vis.min.distance(vis.max) + p.distance(vis.center()) + 1.0;
    [st.view.to_screen(rect, p - d * l), st.view.to_screen(rect, p + d * l)]
}

/// Draws a line, dashed or solid.
fn line(painter: &egui::Painter, ends: [Pos2; 2], stroke: Stroke, solid: bool) {
    if solid {
        painter.line_segment(ends, stroke);
    } else {
        painter.extend(Shape::dashed_line(&ends, stroke, 6.0, 4.0));
    }
}

/// Draws the guides and the construction marks.
pub fn draw(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let d = st.doc.design();
    for g in &d.guides {
        let stroke = Stroke::new(1.0, color_of(st, &g.style));
        let (p, dir) = if g.vertical {
            (Point::new(g.pos, 0.0), Point::new(0.0, 1.0))
        } else {
            (Point::new(0.0, g.pos), Point::new(1.0, 0.0))
        };
        line(painter, line_ends(st, rect, p, dir), stroke, g.style.solid);
    }
    for m in &d.marks {
        let c = color_of(st, &m.style);
        let stroke = Stroke::new(1.0, c);
        match m.shape {
            MarkShape::Line { .. } => {
                if let Some((p, dir)) = m.shape.line() {
                    line(painter, line_ends(st, rect, p, dir), stroke, m.style.solid);
                }
            }
            MarkShape::Point { at } => {
                let s = st.view.to_screen(rect, at);
                let r = 7.0;
                let (h, v) = ([s - Vec2::X * r, s + Vec2::X * r], [s - Vec2::Y * r, s + Vec2::Y * r]);
                line(painter, h, stroke, m.style.solid);
                line(painter, v, stroke, m.style.solid);
                painter.circle_stroke(s, 3.0, Stroke::new(1.5, c));
            }
        }
    }
}

/// The screen place of a guide on its ruler (X on the top ruler for a
/// vertical guide, Y on the left ruler for the others).
fn marker_place(st: &AppState, rect: Rect, g: &Guide) -> f32 {
    if g.vertical {
        st.view.to_screen(rect, Point::new(g.pos, 0.0)).x
    } else {
        st.view.to_screen(rect, Point::new(0.0, g.pos)).y
    }
}

/// Draws a marker on the ruler for each guide, and the place of the guide
/// that a drag moves now.
pub fn draw_ruler_markers(painter: &egui::Painter, rect: Rect, st: &AppState) {
    if !st.settings.show_rulers {
        return;
    }
    let d = st.doc.design();
    let dragged = match &st.drag {
        Some(crate::manipulate::DragOp::Guide { index, .. }) => Some(*index),
        _ => None,
    };
    for (i, g) in d.guides.iter().enumerate() {
        let c = color_of(st, &g.style);
        let at = marker_place(st, rect, g);
        let pts = if g.vertical {
            if at < rect.min.x + RULER || at > rect.max.x {
                continue;
            }
            let b = rect.min.y + RULER;
            vec![
                Pos2::new(at - MARKER, b - 2.0 * MARKER),
                Pos2::new(at + MARKER, b - 2.0 * MARKER),
                Pos2::new(at, b),
            ]
        } else {
            if at < rect.min.y + RULER || at > rect.max.y {
                continue;
            }
            let b = rect.min.x + RULER;
            vec![
                Pos2::new(b - 2.0 * MARKER, at - MARKER),
                Pos2::new(b - 2.0 * MARKER, at + MARKER),
                Pos2::new(b, at),
            ]
        };
        if g.style.locked {
            painter.add(Shape::closed_line(pts, Stroke::new(1.5, c)));
        } else {
            painter.add(Shape::convex_polygon(pts, c, Stroke::NONE));
        }
        if dragged == Some(i) {
            let text = position_text(st, g);
            let pos = if g.vertical {
                Pos2::new(at + 8.0, rect.min.y + RULER + 4.0)
            } else {
                Pos2::new(rect.min.x + RULER + 4.0, at + 6.0)
            };
            let galley = painter.layout_no_wrap(text, egui::FontId::proportional(12.0), c);
            let bg = painter
                .ctx()
                .global_style()
                .visuals
                .extreme_bg_color
                .gamma_multiply(0.85);
            painter.rect_filled(Rect::from_min_size(pos, galley.size()).expand(3.0), 3.0, bg);
            painter.galley(pos, galley, c);
        }
    }
}

/// The place of a guide from the hoop center, as text in the units.
fn position_text(st: &AppState, g: &Guide) -> String {
    let o = st.doc.design().hoop.center;
    let v = if g.vertical { g.pos - o.x } else { g.pos - o.y };
    let axis = if g.vertical { "X" } else { "Y" };
    format!("{axis} {}", st.settings.units.fmt(v))
}

/// The guide whose ruler marker is under the screen point `p`.
pub fn marker_at(st: &AppState, rect: Rect, p: Pos2) -> Option<usize> {
    if !st.settings.show_rulers {
        return None;
    }
    let top = p.y < rect.min.y + RULER && p.x > rect.min.x + RULER;
    let left = p.x < rect.min.x + RULER && p.y > rect.min.y + RULER;
    let d = st.doc.design();
    let near = |i: usize| {
        let g = &d.guides[i];
        let at = marker_place(st, rect, g);
        if g.vertical && top {
            Some((at - p.x).abs())
        } else if !g.vertical && left {
            Some((at - p.y).abs())
        } else {
            None
        }
    };
    (0..d.guides.len())
        .filter_map(|i| near(i).map(|dist| (dist, i)))
        .filter(|(dist, _)| *dist <= MARKER_HIT)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i)| i)
}

/// The guide or the mark under the screen point `p` (on the canvas, not on
/// a ruler). `movable`: only the ones that are not locked.
pub fn mark_at(st: &AppState, rect: Rect, p: Pos2, movable: bool) -> Option<MarkRef> {
    let d = st.doc.design();
    let free = |s: &GuideStyle| !(movable && s.locked);
    let mut best: Option<(f32, MarkRef)> = None;
    let mut keep = |dist: f32, r: MarkRef| {
        if dist <= HIT && best.is_none_or(|(b, _)| dist < b) {
            best = Some((dist, r));
        }
    };
    for (i, g) in d.guides.iter().enumerate().filter(|(_, g)| free(&g.style)) {
        let at = marker_place(st, rect, g);
        keep(
            if g.vertical { (at - p.x).abs() } else { (at - p.y).abs() },
            MarkRef::Guide(i),
        );
    }
    for (i, m) in d.marks.iter().enumerate().filter(|(_, m)| free(&m.style)) {
        let dist = match m.shape {
            MarkShape::Point { at } => st.view.to_screen(rect, at).distance(p),
            MarkShape::Line { .. } => {
                let Some((q, dir)) = m.shape.line() else { continue };
                let [a, b] = line_ends(st, rect, q, dir);
                let to = |v: Pos2| Point::new(f64::from(v.x), f64::from(v.y));
                to(p).distance_to_segment(to(a), to(b)) as f32
            }
        };
        // A point wins over a line through it.
        let dist = if matches!(m.shape, MarkShape::Point { .. }) {
            dist * 0.5
        } else {
            dist
        };
        keep(dist, MarkRef::Mark(i));
    }
    best.map(|(_, r)| r)
}

/// The points of the marks and the places where marks and guides cross,
/// near the view point `raw` (within `tol` mm): the nearest one.
pub fn snap_point(d: &Design, raw: Point, tol: f64) -> Option<(SnapKind, Point)> {
    let lines: Vec<(Point, Point)> = d.marks.iter().filter_map(|m| m.shape.line()).collect();
    let mut cands: Vec<(SnapKind, Point)> = d
        .marks
        .iter()
        .filter_map(|m| match m.shape {
            MarkShape::Point { at } => Some((SnapKind::Mark, at)),
            MarkShape::Line { .. } => None,
        })
        .collect();
    let cross = |a: Point, b: Point| a.x * b.y - a.y * b.x;
    for (i, &(p1, d1)) in lines.iter().enumerate() {
        for &(p2, d2) in &lines[i + 1..] {
            let c = cross(d1, d2);
            if c.abs() > 1e-9 {
                cands.push((SnapKind::Intersection, p1 + d1 * (cross(p2 - p1, d2) / c)));
            }
        }
        for g in &d.guides {
            let (k, num) = if g.vertical {
                (d1.x, g.pos - p1.x)
            } else {
                (d1.y, g.pos - p1.y)
            };
            if k.abs() > 1e-9 {
                cands.push((SnapKind::Intersection, p1 + d1 * (num / k)));
            }
        }
    }
    cands
        .into_iter()
        .map(|(k, q)| (q.distance(raw), k, q))
        .filter(|(dist, ..)| *dist <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, k, q)| (k, q))
}

/// The point of the nearest construction line within `tol` mm of the view
/// point `raw`.
pub fn snap_line(d: &Design, raw: Point, tol: f64) -> Option<Point> {
    d.marks
        .iter()
        .filter_map(|m| m.shape.line())
        .map(|(p, dir)| p + dir * (raw - p).dot(dir))
        .map(|q| (q.distance(raw), q))
        .filter(|(dist, _)| *dist <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, q)| q)
}

impl AppState {
    /// Adds a construction line through the design points `a` and `b`.
    pub fn add_construction_line(&mut self, a: Point, b: Point) {
        let rot = self.doc.design().rotation_transform();
        let (a, b) = (rot.apply(a), rot.apply(b));
        if a.distance(b) < 1e-6 {
            self.message = tr("Click two different points.").into();
            return;
        }
        let d = b - a;
        let angle_deg = math::to_degrees(math::atan2(-d.y, d.x));
        self.doc.edit(tr("Add Construction Line"), |d| {
            d.marks
                .push(ConstructionMark::new(MarkShape::Line { through: a, angle_deg }))
        });
        self.message = tr("Construction line added. Right-click it (Select tool) to change it.").into();
    }

    /// Adds a construction point at the design point `p`.
    pub fn add_construction_point(&mut self, p: Point) {
        let at = self.doc.design().rotation_transform().apply(p);
        self.doc.edit(tr("Add Construction Point"), |d| {
            d.marks.push(ConstructionMark::new(MarkShape::Point { at }))
        });
        self.message = tr("Construction point added. Right-click it (Select tool) to change it.").into();
    }

    /// Opens the edit window of a guide or a mark, with its values.
    pub fn edit_guide(&mut self, target: MarkRef) {
        let d = self.doc.design();
        let o = d.hoop.center;
        let u = self.settings.units;
        let num = |mm: f64| u.fmt_plain(mm);
        let text = match target {
            MarkRef::Guide(i) => {
                let Some(g) = d.guides.get(i) else { return };
                [
                    num(if g.vertical { g.pos - o.x } else { g.pos - o.y }),
                    String::new(),
                    String::new(),
                ]
            }
            MarkRef::Mark(i) => match d.marks.get(i).map(|m| m.shape) {
                Some(MarkShape::Point { at }) => [num(at.x - o.x), num(at.y - o.y), String::new()],
                Some(MarkShape::Line { through, angle_deg }) => {
                    [num(through.x - o.x), num(through.y - o.y), format!("{angle_deg:.1}")]
                }
                None => return,
            },
        };
        self.guide_edit = Some(GuideEdit { target, text });
    }

    /// Applies the values of the edit window. `false` (with a message) if a
    /// value is not a number.
    fn apply_guide_edit(&mut self, e: &GuideEdit) -> bool {
        let u = self.settings.units;
        let o = self.doc.design().hoop.center;
        let len = |t: &str| crate::workspace::parse_length(t, u);
        let bad = || tr("Type a number, for example 12.5, 2 in or 1 1/2\".");
        match e.target {
            MarkRef::Guide(i) => {
                let Some(v) = len(&e.text[0]) else {
                    self.message = bad().into();
                    return false;
                };
                self.doc.edit(tr("Move Guide"), |d| {
                    if let Some(g) = d.guides.get_mut(i) {
                        g.pos = if g.vertical { o.x + v } else { o.y + v };
                    }
                });
            }
            MarkRef::Mark(i) => {
                let (Some(x), Some(y)) = (len(&e.text[0]), len(&e.text[1])) else {
                    self.message = bad().into();
                    return false;
                };
                let p = o + Point::new(x, y);
                let angle = crate::workspace::parse_number(&e.text[2]);
                let is_line = matches!(
                    self.doc.design().marks.get(i).map(|m| m.shape),
                    Some(MarkShape::Line { .. })
                );
                if is_line && angle.is_none() {
                    self.message = tr("Type the angle in degrees, for example 45.").into();
                    return false;
                }
                self.doc.edit(tr("Move Construction Mark"), |d| {
                    if let Some(m) = d.marks.get_mut(i) {
                        m.shape = match m.shape {
                            MarkShape::Point { .. } => MarkShape::Point { at: p },
                            MarkShape::Line { angle_deg, .. } => MarkShape::Line {
                                through: p,
                                angle_deg: angle.unwrap_or(angle_deg),
                            },
                        };
                    }
                });
            }
        }
        true
    }

    /// Changes the style of a guide or a mark (one undo step).
    fn set_guide_style(&mut self, target: MarkRef, label: &str, f: impl FnOnce(&mut GuideStyle)) {
        self.doc.edit(label, |d| {
            if let Some(s) = style_mut(d, target) {
                f(s);
            }
        });
    }

    /// Moves a guide or a mark to the hoop center.
    fn guide_to_center(&mut self, target: MarkRef) {
        let o = self.doc.design().hoop.center;
        self.doc.edit(tr("Move to Hoop Center"), |d| match target {
            MarkRef::Guide(i) => {
                if let Some(g) = d.guides.get_mut(i) {
                    g.pos = if g.vertical { o.x } else { o.y };
                }
            }
            MarkRef::Mark(i) => {
                if let Some(m) = d.marks.get_mut(i) {
                    m.shape = match m.shape {
                        MarkShape::Point { .. } => MarkShape::Point { at: o },
                        MarkShape::Line { angle_deg, .. } => MarkShape::Line { through: o, angle_deg },
                    };
                }
            }
        });
    }

    /// A copy of a guide or a mark, one grid step away (right and down).
    fn duplicate_guide(&mut self, target: MarkRef) {
        let step = if self.settings.grid_mm > 0.0 {
            self.settings.grid_mm
        } else {
            10.0
        };
        self.doc.edit(tr("Duplicate"), |d| match target {
            MarkRef::Guide(i) => {
                if let Some(mut g) = d.guides.get(i).copied() {
                    g.pos += step;
                    g.style.locked = false;
                    d.guides.push(g);
                }
            }
            MarkRef::Mark(i) => {
                if let Some(mut m) = d.marks.get(i).copied() {
                    m.shape = match m.shape {
                        MarkShape::Point { at } => MarkShape::Point {
                            at: at + Point::new(step, step),
                        },
                        MarkShape::Line { through, angle_deg } => {
                            // Parallel, one step away.
                            let r = math::to_radians(angle_deg);
                            let n = Point::new(math::sin(r), math::cos(r));
                            MarkShape::Line {
                                through: through + n * step,
                                angle_deg,
                            }
                        }
                    };
                    m.style.locked = false;
                    d.marks.push(m);
                }
            }
        });
    }

    fn delete_guide(&mut self, target: MarkRef) {
        let label = match target {
            MarkRef::Guide(_) => tr("Delete Guide"),
            MarkRef::Mark(_) => tr("Delete Construction Mark"),
        };
        self.doc.edit(label, |d| match target {
            MarkRef::Guide(i) if i < d.guides.len() => {
                d.guides.remove(i);
            }
            MarkRef::Mark(i) if i < d.marks.len() => {
                d.marks.remove(i);
            }
            _ => {}
        });
        if self.guide_edit.as_ref().is_some_and(|e| e.target == target) {
            self.guide_edit = None;
        }
    }
}

/// The menu of a guide or a construction mark.
pub fn menu(ui: &mut egui::Ui, st: &mut AppState, target: MarkRef) {
    let Some(style) = style_of(st.doc.design(), target) else {
        ui.close();
        return;
    };
    ui.weak(match target {
        MarkRef::Guide(_) => tr("Guide"),
        MarkRef::Mark(i) => match st.doc.design().marks.get(i).map(|m| m.shape) {
            Some(MarkShape::Line { .. }) => tr("Construction Line"),
            _ => tr("Construction Point"),
        },
    });
    if ui.button(tr("Position…")).clicked() {
        st.edit_guide(target);
        ui.close();
    }
    if ui.button(tr("Move to Hoop Center")).clicked() {
        st.guide_to_center(target);
        ui.close();
    }
    ui.menu_button(tr("Color"), |ui| {
        for (name, c) in colors() {
            let swatch = c.map_or_else(|| st.settings.display.guides(), color32);
            let text = egui::RichText::new(format!("■  {name}")).color(swatch);
            if ui.selectable_label(style.color == c, text).clicked() {
                st.set_guide_style(target, tr("Guide Color"), |s| s.color = c);
                ui.close();
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            let mut c = color_of(st, &style);
            if ui.color_edit_button_srgba(&mut c).changed() {
                st.set_guide_style(target, tr("Guide Color"), |s| s.color = Some(Rgb(c.r(), c.g(), c.b())));
            }
            ui.label(tr("Other color"));
        });
    });
    let mut dashed = !style.solid;
    if ui.checkbox(&mut dashed, tr("Dashed")).clicked() {
        st.set_guide_style(target, tr("Dashed"), |s| s.solid = !dashed);
        ui.close();
    }
    let mut locked = style.locked;
    if ui
        .checkbox(&mut locked, tr("Locked"))
        .on_hover_text(tr("A locked guide does not move with a drag."))
        .clicked()
    {
        st.set_guide_style(target, tr("Lock Guide"), |s| s.locked = locked);
        ui.close();
    }
    if ui.button(tr("Duplicate")).clicked() {
        st.duplicate_guide(target);
        ui.close();
    }
    ui.separator();
    if ui.button(tr("Delete")).clicked() {
        st.delete_guide(target);
        ui.close();
    }
}

/// The edit window of a guide or a mark: its place (from the hoop center)
/// and the angle of a line.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut e) = st.guide_edit.clone() else { return };
    let d = st.doc.design();
    let angle = tr("Angle");
    let (title, rows): (&str, Vec<&str>) = match e.target {
        MarkRef::Guide(i) => match d.guides.get(i) {
            Some(g) if g.vertical => (tr("Guide"), vec!["X"]),
            Some(_) => (tr("Guide"), vec!["Y"]),
            None => {
                st.guide_edit = None;
                return;
            }
        },
        MarkRef::Mark(i) => match d.marks.get(i).map(|m| m.shape) {
            Some(MarkShape::Point { .. }) => (tr("Construction Point"), vec!["X", "Y"]),
            Some(MarkShape::Line { .. }) => (tr("Construction Line"), vec!["X", "Y", angle]),
            None => {
                st.guide_edit = None;
                return;
            }
        },
    };
    let id = egui::Id::new("Guide Position");
    let mut open = true;
    let mut close = false;
    let mut apply = false;
    crate::dock::docked(egui::Window::new(title).id(id), ctx, id, st.canvas_rect)
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.weak(tr("From the hoop center, in the units of the Options. Y goes down; the angle goes counterclockwise from the right."));
            egui::Grid::new("guide_edit").num_columns(2).show(ui, |ui| {
                for (k, row) in rows.iter().enumerate() {
                    ui.label(*row);
                    let r = ui.add(egui::TextEdit::singleline(&mut e.text[k]).desired_width(90.0));
                    if k == 0 && !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                        r.request_focus();
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        apply = true;
                    }
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button(tr("Apply")).clicked() {
                    apply = true;
                }
                if ui.button(tr("Close")).clicked() {
                    close = true;
                }
            });
        });
    if apply && st.apply_guide_edit(&e) {
        st.edit_guide(e.target);
        return;
    }
    st.guide_edit = (open && !close).then_some(e);
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Modifiers, PointerButton};

    use super::*;
    use crate::draw::Tool;
    use crate::test_util::Harness;

    #[test]
    fn construction_lines_cross_and_points_snap() {
        let mut d = Design::new("");
        d.marks.push(ConstructionMark::new(MarkShape::Line {
            through: Point::new(0.0, 0.0),
            angle_deg: 45.0,
        }));
        d.guides.push(Guide::new(true, 10.0));
        d.marks.push(ConstructionMark::new(MarkShape::Point {
            at: Point::new(30.0, 5.0),
        }));
        // The line goes up to the right (Y down): it meets x = 10 at y = -10.
        let (k, q) = snap_point(&d, Point::new(10.2, -9.9), 0.5).unwrap();
        assert_eq!(k, SnapKind::Intersection);
        assert!(q.distance(Point::new(10.0, -10.0)) < 1e-9, "{q:?}");
        let (k, q) = snap_point(&d, Point::new(30.1, 5.1), 0.5).unwrap();
        assert_eq!((k, q), (SnapKind::Mark, Point::new(30.0, 5.0)));
        assert!(snap_point(&d, Point::new(50.0, 50.0), 0.5).is_none());
        // Near the line: the point goes onto it.
        let q = snap_line(&d, Point::new(20.0, -19.7), 0.5).unwrap();
        assert!((q.x + q.y).abs() < 1e-9, "{q:?}");
    }

    #[test]
    fn the_tools_the_menu_actions_and_the_edit_window() {
        let mut h = Harness::empty();
        let c = h.st.doc.design().hoop.center;
        let at = |h: &Harness, x: f64, y: f64| h.screen(c + Point::new(x, y));
        // A construction line through two clicks, and a point.
        h.st.set_tool(Tool::ConstructionLine);
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        h.click(PointerButton::Primary, at(&h, 10.0, -10.0));
        h.st.set_tool(Tool::ConstructionPoint);
        h.click(PointerButton::Primary, at(&h, 20.0, 5.0));
        let d = h.st.doc.design();
        assert_eq!(d.marks.len(), 2);
        let MarkShape::Line { angle_deg, .. } = d.marks[0].shape else {
            panic!()
        };
        assert!((angle_deg - 45.0).abs() < 1.0, "{angle_deg}");
        assert!(d.objects().is_empty(), "marks do not sew");
        // Style, lock, duplicate, center and delete; each is one undo step.
        let m = MarkRef::Mark(1);
        let steps = h.st.doc.undo_len();
        h.st.set_guide_style(m, "Color", |s| s.color = Some(Rgb(1, 2, 3)));
        h.st.set_guide_style(m, "Dashed", |s| s.solid = true);
        h.st.set_guide_style(m, "Lock", |s| s.locked = true);
        h.st.duplicate_guide(m);
        h.st.guide_to_center(MarkRef::Mark(2));
        assert_eq!(h.st.doc.undo_len(), steps + 5);
        let d = h.st.doc.design();
        assert_eq!(d.marks[1].style.color, Some(Rgb(1, 2, 3)));
        assert!(d.marks[1].style.solid && d.marks[1].style.locked);
        assert!(!d.marks[2].style.locked, "a copy is not locked");
        assert_eq!(d.marks[2].shape, MarkShape::Point { at: c });
        h.st.delete_guide(MarkRef::Mark(2));
        assert_eq!(h.st.doc.design().marks.len(), 2);
        // The edit window: type a new place for the point.
        h.st.edit_guide(m);
        let e = h.st.guide_edit.as_mut().unwrap();
        assert_eq!(e.text[0], "20");
        e.text = ["-5".into(), "7.5".into(), String::new()];
        let e = e.clone();
        assert!(h.st.apply_guide_edit(&e));
        assert_eq!(
            h.st.doc.design().marks[1].shape,
            MarkShape::Point {
                at: c + Point::new(-5.0, 7.5)
            }
        );
        let mut bad = e.clone();
        bad.text[0] = "abc".into();
        assert!(!h.st.apply_guide_edit(&bad));
        h.frame(vec![]);
    }

    #[test]
    fn ruler_markers_move_guides_and_open_the_menu() {
        let mut h = Harness::empty();
        let c = h.st.doc.design().hoop.center;
        h.st.doc.edit("Add", |d| d.guides.push(Guide::new(true, c.x)));
        h.frame(vec![]);
        let rect = h.map().rect;
        let x = h.screen(c).x;
        let on_marker = Pos2::new(x, rect.min.y + RULER / 2.0);
        assert_eq!(marker_at(&h.st, rect, on_marker), Some(0));
        // A drag of the marker moves the guide (no new guide).
        let to = Pos2::new(x + 50.0, rect.min.y + RULER / 2.0);
        h.drag(PointerButton::Primary, on_marker, to, Modifiers::NONE);
        let d = h.st.doc.design();
        assert_eq!(d.guides.len(), 1);
        assert!(d.guides[0].pos > c.x + 1.0, "{}", d.guides[0].pos);
        // A locked guide does not move.
        let pos = d.guides[0].pos;
        h.st.set_guide_style(MarkRef::Guide(0), "Lock", |s| s.locked = true);
        let x = h.screen(Point::new(pos, c.y)).x;
        let on_marker = Pos2::new(x, rect.min.y + RULER / 2.0);
        h.drag(
            PointerButton::Primary,
            on_marker,
            on_marker + Vec2::new(40.0, 0.0),
            Modifiers::NONE,
        );
        assert_eq!(h.st.doc.design().guides[0].pos, pos);
        assert_eq!(
            h.st.doc.design().guides.len(),
            1,
            "a drag from a locked marker makes no guide"
        );
        // A double click on the marker opens the edit window; with a draw
        // tool, the clicks on the ruler place no point.
        h.st.set_tool(Tool::Line);
        for _ in 0..2 {
            h.click(PointerButton::Primary, on_marker);
        }
        assert!(h.st.guide_edit.as_ref().is_some_and(|e| e.target == MarkRef::Guide(0)));
        h.st.guide_edit = None;
        // A right click on the marker opens its menu, also with a draw tool.
        let ev = |pressed| Event::PointerButton {
            pos: on_marker,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![Event::PointerMoved(on_marker)]);
        h.frame(vec![ev(true)]);
        h.frame(vec![ev(false)]);
        assert_eq!(h.st.guide_menu, Some(MarkRef::Guide(0)));
        assert!(h.st.draw.points.is_empty(), "the clicks on the ruler place no point");
    }
}
