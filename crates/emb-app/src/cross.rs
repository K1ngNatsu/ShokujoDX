//! Cross-stitch objects in the app (P9 Milestone 1, ADR 0038): the paint,
//! erase and backstitch tools, the grid on the canvas, Object Properties
//! and Convert to cross stitch.
//!
//! The tools work on one grid: the grid of the selected cross-stitch
//! object; else the grid of the last cross-stitch object with the cell size
//! of the property bar; else a new grid with a cell corner at the hoop
//! center.

use eframe::egui::{self, Color32, Pos2, Shape, Stroke};
use emb_geom::Point;
use emb_model::cross::{CrossOrder, CrossStitch, MAX_CELL_MM, MIN_CELL_MM, Part, aida_cell_mm};
use emb_model::{DesignObject, MergeKey, ObjectId, ObjectKind};

use crate::app::AppState;
use crate::draw::{Tool, thread_for_new};
use crate::i18n::tr;
use crate::manipulate::ViewMap;

/// Aida fabric counts in the cell size menu (cells to the inch).
const AIDA_COUNTS: [u32; 5] = [8, 11, 14, 16, 18];
/// Machine cell sizes in the cell size menu, in mm.
const MACHINE_CELLS_MM: [f64; 4] = [2.0, 2.5, 3.0, 4.0];
/// The grid shows only when its cells are at least this large on screen.
const MIN_GRID_PX: f32 = 4.0;
/// No more grid lines than this in each direction.
const MAX_GRID_LINES: i32 = 400;

/// The state of the cross-stitch tools.
#[derive(Debug, Clone, Default)]
pub struct CrossUi {
    /// The part that the cross-stitch tool puts into cells. Quarter and
    /// three-quarter crosses take the corner of the cell nearest to the
    /// pointer.
    pub part: Part,
    /// The drag that paints or erases now: its number (one undo step for
    /// each drag) and the last grid point it was at.
    stroke: Option<(u64, Point)>,
    strokes: u64,
}

fn cross_of(o: &DesignObject) -> Option<&CrossStitch> {
    match &o.kind {
        ObjectKind::CrossStitch(x) => Some(x),
        _ => None,
    }
}

impl AppState {
    /// The selected cross-stitch object with the grid that the tools use.
    fn selected_cross(&self) -> Option<&CrossStitch> {
        let d = self.doc.design();
        self.selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id))
            .find_map(cross_of)
    }

    /// The cell size of the selected cross-stitch object.
    pub fn selected_cross_mm(&self) -> Option<f64> {
        self.selected_cross().map(CrossStitch::cell_mm)
    }

    /// The grid that the cross-stitch tools work on (see the module comment).
    pub fn cross_grid(&self) -> CrossStitch {
        let d = self.doc.design();
        let cell = self.settings.defaults.cross_cell_mm;
        self.selected_cross()
            .or_else(|| {
                d.objects()
                    .iter()
                    .rev()
                    .filter_map(|o| cross_of(o))
                    .find(|x| (x.cell_mm() - cell).abs() < 1e-6)
            })
            .map(CrossStitch::same_grid_empty)
            .unwrap_or_else(|| CrossStitch::new(d.hoop.center, cell))
    }

    /// Puts the part of the tool into the cells on the line from `a` to
    /// `b` (grid units), or empties them. One undo step for each drag.
    fn cross_stroke(&mut self, grid: &CrossStitch, a: Point, b: Point, erase: bool, key: u64) {
        // The cells along the line, a quarter of a cell apart.
        let steps = ((b - a).length() * 4.0).ceil().max(1.0) as usize;
        let mut cells: Vec<(i32, i32, Part)> = Vec::new();
        for k in 0..=steps {
            let g = a.lerp(b, k as f64 / steps as f64);
            let Some((c, r, corner)) = grid.cell_at(grid.at(g.x, g.y)) else {
                continue;
            };
            if !cells.iter().any(|&(cc, rr, _)| (cc, rr) == (c, r)) {
                cells.push((c, r, self.cross.part.at_corner(corner)));
            }
        }
        let current = self.current_thread;
        let label = if erase { "Erase Cells" } else { "Paint cells" };
        let at: Vec<(i32, i32)> = cells.iter().map(|&(c, r, _)| (c, r)).collect();
        // Test on a copy first: a click that changes nothing makes no undo step.
        let mut test = self.doc.design().clone();
        let changes = if erase {
            test.cross_erase(grid, &at)
        } else {
            let t = thread_for_new(&mut test, current);
            test.cross_paint(grid, t, &cells).is_some()
        };
        if !changes {
            return;
        }
        let key = MergeKey::new(format!("cross-stroke-{key}"));
        let id = self.doc.edit_merged(label, key, |d| {
            if erase {
                d.cross_erase(grid, &at);
                None
            } else {
                let t = thread_for_new(d, current);
                d.cross_paint(grid, t, &cells)
            }
        });
        if let Some(id) = id {
            self.current_thread = self.doc.design().object(id).map(|o| o.thread);
            self.selection.set(id);
        }
    }

    /// Changes the selected fills, appliqué, satin columns and closed
    /// vectors into cross-stitch objects: each cell whose center is inside.
    pub fn convert_to_cross(&mut self) {
        let d = self.doc.design();
        let ids: Vec<ObjectId> = self
            .selection
            .in_sew_order(d)
            .into_iter()
            .filter(|id| d.object(*id).is_some_and(|o| emb_model::shaping::takes_part(&o.kind)))
            .collect();
        if ids.is_empty() {
            self.message =
                "Select a fill, an appliqué, a satin column or a closed vector to change into cross stitch.".into();
            return;
        }
        if self.refuse_linked(&ids) {
            return;
        }
        let grid = self.cross_grid();
        let mut made = Vec::new();
        let mut cells = 0;
        self.doc.edit("Convert to cross stitch", |d| {
            for id in ids {
                let Some(o) = d.object(id).cloned() else { continue };
                let Some(regions) = emb_model::shaping::area_of(&o.kind) else {
                    continue;
                };
                let rings: Vec<Vec<Point>> = regions
                    .iter()
                    .flat_map(|r| std::iter::once(r.outer.clone()).chain(r.holes.iter().cloned()))
                    .collect();
                let inside: Vec<(i32, i32, Part)> = grid
                    .cells_inside(&rings)
                    .into_iter()
                    .map(|(c, r)| (c, r, Part::Full))
                    .collect();
                let index = d.index_of(id).unwrap_or(0);
                d.remove_object(id);
                let before: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
                cells += inside.len();
                if let Some(x) = d.cross_paint(&grid, o.thread, &inside) {
                    // A new object takes the place of the old one in the sew order.
                    if !before.contains(&x) {
                        d.move_object(x, index);
                        if let Some(n) = d.object_mut(x) {
                            n.name = o.name.clone();
                            n.functions = o.functions;
                        }
                    }
                    if !made.contains(&x) {
                        made.push(x);
                    }
                }
            }
        });
        self.selection.set_all(made.iter().copied());
        self.message = if made.is_empty() {
            "The shapes are smaller than one cell: no cross stitch. Use a smaller cell size.".into()
        } else {
            format!(
                "{cells} cells in {} cross-stitch object{} ({:.1} mm cells).",
                made.len(),
                if made.len() == 1 { "" } else { "s" },
                grid.cell_mm()
            )
        };
    }
}

impl AppState {
    /// The chart pages of the cross-stitch objects on the grid of the
    /// selected one (or of the first one), on the paper of the output
    /// profile. `None` if the design has none.
    pub fn cross_chart_pages(&self) -> Option<Vec<emb_render::page::Page>> {
        let d = self.doc.design();
        let (colors, cell_mm) = emb_render::cross_chart::colors_of(d, self.selected_cross())?;
        let spec = emb_render::cross_chart::ChartSpec {
            title: if d.name.is_empty() {
                "Design".into()
            } else {
                d.name.clone()
            },
            paper: self.settings.output.paper,
            cell_mm,
        };
        Some(emb_render::cross_chart::chart(&colors, &spec))
    }

    /// Asks for a file name and writes the cross-stitch chart (PDF).
    pub fn cross_chart_dialog(&mut self) {
        let Some(pages) = self.cross_chart_pages() else {
            self.message = tr("The design has no cross stitch. Draw some with Specialty > Cross Stitch first.").into();
            return;
        };
        let name = format!("{} chart.pdf", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Cross-stitch chart")
            .add_filter("PDF", &["pdf"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        let title = self.doc.design().name.clone();
        match std::fs::write(&p, emb_render::pdf::write(&pages, &title)) {
            Ok(()) => {
                self.message = crate::i18n::fill("Wrote the chart ({} pages) to {}", &[&(pages.len()), &(p.display())]);
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Chart not written".into(),
                    lines: vec![format!("{}: {e}", p.display())],
                })
            }
        }
    }
}

/// Canvas input of the cross-stitch tools.
pub fn tool_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if st.tool == Tool::Backstitch {
        return line_input(response, st, map);
    }
    let grid = st.cross_grid();
    let (primary, shift) = ui.input(|i| (i.pointer.primary_down(), i.modifiers.shift));
    let down = response.is_pointer_button_down_on() && primary;
    let pos = response.interact_pointer_pos().filter(|_| down);
    let Some(g) = pos.and_then(|p| grid.to_grid(map.to_design(p))) else {
        if st.cross.stroke.take().is_some() {
            st.doc.end_merge();
        }
        return;
    };
    let key = match st.cross.stroke {
        Some((key, _)) => key,
        None => {
            st.cross.strokes += 1;
            st.cross.strokes
        }
    };
    let from = st.cross.stroke.map_or(g, |(_, last)| last);
    // The erase tool, or Shift with the cross-stitch tool, erases. (A right
    // drag pans the view, as with all tools.)
    let erase = st.tool == Tool::CrossErase || shift;
    st.cross_stroke(&grid, from, g, erase, key);
    if erase && let Some(p) = pos {
        let tol = 4.0 / st.view.scale();
        let q = map.to_design(p);
        let mut test = st.doc.design().clone();
        if test.cross_erase_lines(&grid, q, tol) {
            st.doc
                .edit_merged("Erase Cells", MergeKey::new(format!("cross-stroke-{key}")), |d| {
                    d.cross_erase_lines(&grid, q, tol)
                });
        }
    }
    st.cross.stroke = Some((key, g));
}

/// The backstitch tool: each click adds a line from the last point to the
/// nearest corner or cell center.
fn line_input(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if response.secondary_clicked() {
        st.draw = Default::default();
        return;
    }
    if !response.clicked() {
        return;
    }
    let grid = st.cross_grid();
    let Some(h) = response
        .interact_pointer_pos()
        .and_then(|p| grid.half_point_near(map.to_design(p)))
    else {
        return;
    };
    let q = grid.half_point(h);
    if let Some(&(last, _)) = st.draw.points.last()
        && let Some(a) = grid.half_point_near(last)
        && a != h
    {
        let current = st.current_thread;
        let mut test = st.doc.design().clone();
        let t = thread_for_new(&mut test, current);
        if test.cross_add_line(&grid, t, a, h).is_some() {
            let id = st.doc.edit("Add backstitch line", |d| {
                let t = thread_for_new(d, current);
                d.cross_add_line(&grid, t, a, h)
            });
            if let Some(id) = id {
                st.current_thread = st.doc.design().object(id).map(|o| o.thread);
                st.selection.set(id);
            }
        }
    }
    st.draw.points.push((q, false));
}

/// The grid, the cell under the pointer and the backstitch line in progress.
pub fn draw_overlay(painter: &egui::Painter, ui: &egui::Ui, st: &AppState, map: &ViewMap, color: Color32) {
    let grid = st.cross_grid();
    let cell_px = grid.cell_mm() as f32 * map.view.scale() as f32;
    let dark = ui.visuals().dark_mode;
    let ink = if dark {
        Color32::from_gray(200)
    } else {
        Color32::from_gray(60)
    };
    if cell_px >= MIN_GRID_PX {
        draw_grid(painter, map, &grid, ink);
    }
    let hover = ui
        .input(|i| i.pointer.hover_pos())
        .filter(|p| map.rect.contains(*p))
        .map(|p| map.to_design(p));
    let Some(h) = hover else { return };
    let s = |x: f64, y: f64| map.to_screen(grid.at(x, y));
    match st.tool {
        Tool::Backstitch => {
            let Some(hp) = grid.half_point_near(h) else { return };
            let q = map.to_screen(grid.half_point(hp));
            painter.circle_stroke(q, 4.0, Stroke::new(1.5, color));
            if let Some(&(last, _)) = st.draw.points.last() {
                painter.line_segment([map.to_screen(last), q], Stroke::new(2.0, color));
            }
        }
        _ => {
            let Some((c, r, corner)) = grid.cell_at(h) else { return };
            let (x, y) = (f64::from(c), f64::from(r));
            let erase = st.tool == Tool::CrossErase;
            let edge = if erase { Color32::from_rgb(220, 60, 50) } else { color };
            let ring = vec![s(x, y), s(x + 1.0, y), s(x + 1.0, y + 1.0), s(x, y + 1.0)];
            painter.add(Shape::closed_line(ring, Stroke::new(1.5, edge)));
            if !erase {
                let thread = st
                    .current_thread
                    .and_then(|t| st.doc.design().thread(t))
                    .map_or(color, |t| crate::canvas::color32(t.color));
                for ((ax, ay), (bx, by)) in st.cross.part.at_corner(corner).legs() {
                    painter.line_segment([s(x + ax, y + ay), s(x + bx, y + by)], Stroke::new(2.0, thread));
                }
            }
        }
    }
}

/// The grid lines in the view; each tenth line is darker, as on a chart.
fn draw_grid(painter: &egui::Painter, map: &ViewMap, grid: &CrossStitch, ink: Color32) {
    let r = map.rect;
    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
    let Some(g) = corners
        .iter()
        .map(|p| grid.to_grid(map.to_design(*p)))
        .collect::<Option<Vec<Point>>>()
    else {
        return;
    };
    let lo = |f: fn(&Point) -> f64| g.iter().map(f).fold(f64::INFINITY, f64::min).floor() as i32;
    let hi = |f: fn(&Point) -> f64| g.iter().map(f).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    let (c0, c1, r0, r1) = (lo(|p| p.x), hi(|p| p.x), lo(|p| p.y), hi(|p| p.y));
    if c1 - c0 > MAX_GRID_LINES || r1 - r0 > MAX_GRID_LINES {
        return;
    }
    let s = |x: i32, y: i32| -> Pos2 { map.to_screen(grid.at(f64::from(x), f64::from(y))) };
    let minor = Stroke::new(0.5, ink.gamma_multiply(0.25));
    let major = Stroke::new(1.0, ink.gamma_multiply(0.45));
    for c in c0..=c1 {
        let w = if c.rem_euclid(10) == 0 { major } else { minor };
        painter.line_segment([s(c, r0), s(c, r1)], w);
    }
    for r in r0..=r1 {
        let w = if r.rem_euclid(10) == 0 { major } else { minor };
        painter.line_segment([s(c0, r), s(c1, r)], w);
    }
}

/// The property bar of the cross-stitch tools: the part and the cell size.
pub fn bar(ui: &mut egui::Ui, cross: &mut CrossUi, cell_mm: &mut f64, tool: Tool, selected: Option<f64>) {
    if tool == Tool::CrossStitch {
        egui::ComboBox::from_id_salt("cross_part")
            .selected_text(tr(cross.part.name()))
            .show_ui(ui, |ui| {
                for p in [
                    Part::Full,
                    Part::HalfUp,
                    Part::HalfDown,
                    Part::Quarter(emb_model::cross::Corner::TopLeft),
                    Part::ThreeQuarter(emb_model::cross::Corner::TopLeft),
                ] {
                    let on = std::mem::discriminant(&cross.part) == std::mem::discriminant(&p);
                    if ui.selectable_label(on, tr(p.name())).clicked() {
                        cross.part = p;
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "Quarter and three-quarter crosses go to the corner of the cell nearest to the pointer.",
            ));
    }
    match selected {
        Some(mm) => {
            ui.label(crate::i18n::fill(
                "Grid of the selected object: {} mm cells",
                &[&format!("{:.2}", mm)],
            ))
            .on_hover_text(tr(
                "Click an empty place first to use the cell size of the property bar.",
            ));
        }
        None => {
            ui.label(tr("Cell size"));
            ui.add(
                egui::DragValue::new(cell_mm)
                    .range(MIN_CELL_MM..=MAX_CELL_MM)
                    .speed(0.05)
                    .max_decimals(2)
                    .clamp_existing_to_range(false)
                    .suffix(" mm"),
            )
            .on_hover_text(tr("A new cell size starts a new grid."));
            cell_menu(ui, cell_mm);
        }
    }
    let hint = match tool {
        Tool::CrossStitch => "Drag over cells to put crosses in them. Shift+drag: erase.",
        Tool::CrossErase => "Drag over cells and lines to remove them.",
        _ => "Click corners or cell centers. Right click or Esc: end the line.",
    };
    ui.weak(hint);
}

/// A menu of the usual cell sizes: machine sizes and Aida counts.
pub(crate) fn cell_menu(ui: &mut egui::Ui, cell_mm: &mut f64) -> bool {
    let mut changed = false;
    ui.menu_button(egui_phosphor::regular::CARET_DOWN, |ui| {
        for mm in MACHINE_CELLS_MM {
            if ui
                .button(crate::i18n::fill("{} mm", &[&format!("{:.1}", mm)]))
                .clicked()
            {
                *cell_mm = mm;
                changed = true;
            }
        }
        ui.separator();
        for n in AIDA_COUNTS {
            if ui
                .button(crate::i18n::fill(
                    "Aida {} ({} mm)",
                    &[&n, &format!("{:.2}", aida_cell_mm(n))],
                ))
                .clicked()
            {
                *cell_mm = aida_cell_mm(n);
                changed = true;
            }
        }
    })
    .response
    .on_hover_text(tr("Usual cell sizes"));
    changed
}

/// Object Properties of a cross-stitch object.
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, x: &CrossStitch) {
    let [full, half, quarter, three] = x.counts();
    ui.label(crate::i18n::fill(
        "{} full, {} half, {} quarter and {} three-quarter crosses; {} backstitch lines",
        &[&full, &half, &quarter, &three, &(x.lines.len())],
    ));
    let (w, h) = x
        .bbox()
        .map_or((0.0, 0.0), |b| (b.width() / x.cell_mm(), b.height() / x.cell_mm()));
    ui.weak(crate::i18n::fill(
        "{} × {} cells",
        &[&format!("{:.0}", w), &format!("{:.0}", h)],
    ));
    ui.separator();
    // The cell size is the size of the grid: all objects on it change.
    let mut mm = x.cell_mm();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("Cell size"));
        changed |= ui
            .add(
                egui::DragValue::new(&mut mm)
                    .range(MIN_CELL_MM..=MAX_CELL_MM)
                    .speed(0.05)
                    .max_decimals(2)
                    .clamp_existing_to_range(false)
                    .suffix(" mm"),
            )
            .on_hover_text(tr(
                "The size of the cells of the grid. All cross-stitch objects on this grid change.",
            ))
            .changed();
        changed |= cell_menu(ui, &mut mm);
    });
    if changed && (mm - x.cell_mm()).abs() > 1e-9 {
        let grid = x.same_grid_empty();
        st.doc
            .edit_merged("Change cell size", MergeKey::new(format!("{}:cross-cell", id.0)), |d| {
                for oid in d.cross_objects_on(&grid) {
                    if let Some(ObjectKind::CrossStitch(y)) = d.object_mut(oid).map(|o| &mut o.kind) {
                        y.set_cell_mm(mm);
                    }
                }
            });
        st.settings.defaults.cross_cell_mm = mm.clamp(MIN_CELL_MM, MAX_CELL_MM);
    }
    let mut p = x.params;
    ui.horizontal(|ui| {
        ui.label(tr("Each leg"));
        for (n, name) in [(1, tr("Once")), (3, tr("3 times"))] {
            ui.radio_value(&mut p.passes, n, name);
        }
    })
    .response
    .on_hover_text(tr("3 times: a heavier cross, for thin thread or a textured fabric."));
    ui.horizontal(|ui| {
        ui.label(tr("Order"));
        ui.radio_value(&mut p.order, CrossOrder::Rows, tr("Rows"))
            .on_hover_text(tr(
                "All top legs go the same way. Travel stitches go along the cell sides.",
            ));
        ui.radio_value(&mut p.order, CrossOrder::Zigzag, tr("Zigzag"))
            .on_hover_text(tr("No travel stitches, but the top leg changes from cell to cell."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Backstitch"));
        for (n, name) in [(1, tr("Once")), (3, tr("3 times"))] {
            ui.radio_value(&mut p.line_repeats, n, name);
        }
    });
    if p != x.params {
        st.doc.edit("Change cross stitch", |d| {
            if let Some(ObjectKind::CrossStitch(y)) = d.object_mut(id).map(|o| &mut o.kind) {
                y.params = p;
            }
        });
    }
    ui.separator();
    ui.weak(tr("Edit the cells with the tools of Specialty > Cross Stitch."));
}

/// A short description for the Sequence panel.
pub fn detail(x: &CrossStitch) -> String {
    format!(
        "Cross stitch · {} cells · {:.1} mm{}",
        x.cells.len(),
        x.cell_mm(),
        if x.lines.is_empty() {
            String::new()
        } else {
            format!(" · {} lines", x.lines.len())
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn convert_makes_cells_on_the_grid_in_the_place_of_the_fill() {
        let mut h = Harness::empty();
        let red = h.st.doc.edit("t", |d| d.add_thread("Red", emb_model::Rgb(200, 0, 0)));
        let fill = h.st.doc.edit("t", |d| {
            let a = d.add_object(
                red,
                ObjectKind::Fill {
                    outer: emb_geom::Path::rect(emb_geom::BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 5.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            );
            d.add_object(
                red,
                ObjectKind::Running {
                    path: emb_geom::Path::new().move_to(Point::ZERO).line_to(Point::new(5.0, 5.0)),
                    params: Default::default(),
                },
            );
            a
        });
        h.st.doc.edit("t", |d| d.hoop.center = Point::ZERO);
        h.st.settings.defaults.cross_cell_mm = 2.5;
        h.st.selection.set(fill);
        h.st.convert_to_cross();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        let x = cross_of(&d.objects()[0]).expect("first in the sew order");
        assert_eq!(x.cells.len(), 4 * 2);
        assert_eq!(d.objects()[0].thread, red);
        // The stitches are crosses on the 2.5 mm grid.
        let s = emb_stitch::stitch_object(&d.objects()[0], Point::ZERO).unwrap();
        assert!(s.runs.iter().flat_map(|r| &r.points).all(|p| {
            let (a, b) = (p.x / 2.5, p.y / 2.5);
            (a - a.round()).abs() < 1e-9 && (b - b.round()).abs() < 1e-9
        }));
    }

    #[test]
    fn a_drag_paints_a_line_of_cells_in_one_undo_step() {
        let mut h = Harness::empty();
        h.st.doc.edit("t", |d| d.hoop.center = Point::ZERO);
        h.st.settings.defaults.cross_cell_mm = 2.0;
        h.st.set_tool(Tool::CrossStitch);
        let grid = h.st.cross_grid();
        let undo = h.st.doc.undo_len();
        h.st.cross_stroke(&grid, Point::new(0.5, 0.5), Point::new(0.5, 0.5), false, 1);
        h.st.cross_stroke(&grid, Point::new(0.5, 0.5), Point::new(5.5, 0.5), false, 1);
        h.st.doc.end_merge();
        assert_eq!(h.st.doc.undo_len(), undo + 1);
        let d = h.st.doc.design();
        let x = cross_of(&d.objects()[0]).unwrap();
        assert_eq!(x.cells.len(), 6, "no gaps in a fast drag");
        // The selected object gives the grid now; a click on a painted cell
        // changes nothing and makes no undo step.
        let grid = h.st.cross_grid();
        h.st.cross_stroke(&grid, Point::new(2.5, 0.5), Point::new(2.5, 0.5), false, 2);
        assert_eq!(h.st.doc.undo_len(), undo + 1);
        // Erase two cells.
        h.st.cross_stroke(&grid, Point::new(1.5, 0.5), Point::new(2.5, 0.5), true, 3);
        let d = h.st.doc.design();
        assert_eq!(cross_of(&d.objects()[0]).unwrap().cells.len(), 4);
    }

    /// Real pointer events on the canvas: a drag paints a row of cells in
    /// one undo step, Shift+drag erases, and the backstitch tool adds a
    /// line for each click after the first.
    #[test]
    fn canvas_drags_paint_and_erase_and_clicks_draw_lines() {
        use eframe::egui::{Key, Modifiers, PointerButton};
        let mut h = Harness::empty();
        h.st.settings.defaults.cross_cell_mm = 3.0;
        h.st.set_tool(Tool::CrossStitch);
        let grid = h.st.cross_grid();
        let map = h.map();
        let cell = |c: f64, r: f64| map.to_screen(grid.at(c + 0.5, r + 0.5));
        let (a, b) = (cell(0.0, 0.0), cell(7.0, 0.0));
        let undo = h.st.doc.undo_len();
        h.drag(PointerButton::Primary, a, b, Modifiers::NONE);
        assert_eq!(h.st.doc.undo_len(), undo + 1, "one step");
        let x = cross_of(&h.st.doc.design().objects()[0]).unwrap().clone();
        assert_eq!(x.cells.len(), 8);
        assert!(x.cells.iter().all(|c| c.part == Part::Full && c.row == 0));
        // Shift+drag over three cells erases them.
        let (a, b) = (cell(2.0, 0.0), cell(4.0, 0.0));
        h.drag(PointerButton::Primary, a, b, Modifiers::SHIFT);
        let left: Vec<i32> = cross_of(&h.st.doc.design().objects()[0])
            .unwrap()
            .cells
            .iter()
            .map(|c| c.col)
            .collect();
        assert_eq!(left, vec![0, 1, 5, 6, 7]);
        // Backstitch: three clicks on cell corners make two lines.
        h.st.set_tool(Tool::Backstitch);
        for (c, r) in [(0.0, 2.0), (3.0, 2.0), (3.0, 4.0)] {
            h.click(PointerButton::Primary, map.to_screen(grid.at(c, r)));
        }
        h.key(Key::Enter);
        let lines: usize =
            h.st.doc
                .design()
                .objects()
                .iter()
                .filter_map(|o| cross_of(o))
                .map(|x| x.lines.len())
                .sum();
        assert_eq!(lines, 2);
        assert!(h.st.draw.points.is_empty(), "Enter ends the line");
    }

    #[test]
    fn the_chart_has_the_threads_of_the_grid() {
        let mut h = Harness::empty();
        h.st.settings.defaults.cross_cell_mm = 2.0;
        assert!(h.st.cross_chart_pages().is_none());
        let grid = h.st.cross_grid();
        h.st.cross_stroke(&grid, Point::new(0.5, 0.5), Point::new(4.5, 0.5), false, 1);
        let pages = h.st.cross_chart_pages().expect("a chart");
        assert_eq!(pages.len(), 2, "the key page and one chart page");
        let texts: Vec<&str> = pages[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                emb_render::page::Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.starts_with("5 × 1 cells, 5 stitches, 1 color")),
            "{texts:?}"
        );
    }

    #[test]
    fn a_new_cell_size_starts_a_new_grid() {
        let mut h = Harness::empty();
        h.st.doc.edit("t", |d| d.hoop.center = Point::new(1.0, 1.0));
        h.st.settings.defaults.cross_cell_mm = 2.0;
        let a = h.st.cross_grid();
        assert_eq!(a.origin, Point::new(1.0, 1.0));
        h.st.cross_stroke(&a, Point::new(0.5, 0.5), Point::new(0.5, 0.5), false, 1);
        h.st.selection.clear();
        h.st.settings.defaults.cross_cell_mm = 3.0;
        let b = h.st.cross_grid();
        assert!(!b.same_grid(&a));
        h.st.settings.defaults.cross_cell_mm = 2.0;
        assert!(h.st.cross_grid().same_grid(&a), "back to the grid of the object");
    }
}
