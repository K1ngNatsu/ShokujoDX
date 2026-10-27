//! Hooping mode (P7 Milestone 3, ADR 0032): the hoopings of a large design
//! on the canvas, the Hooping ribbon tab, Preview hoopings, and output of
//! one machine file for each hooping with a hooping sheet.
//!
//! In hooping mode the canvas shows the field of each hooping with its
//! number, and the split lines. A drag in a field moves that hooping; a drag
//! on a split line moves it, a drag on a node of the selected line moves the
//! node. Draw split line: click the points, then Enter, a double-click or a
//! right-click; Esc stops. Delete removes the selected hooping or line.
//! All changes are design edits (undo works).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use emb_geom::{BBox, Point};
use emb_model::hooping::{DEFAULT_JOIN_MM, DEFAULT_OVERLAP_MM};
use emb_model::{HoopPreset, HoopSelection, Hooping, HoopingLayout, HoopingOut, MergeKey};
use emb_render::Segment;

use crate::app::AppState;
use crate::canvas::{CanvasView, color32, stitch_mesh};
use crate::file::Notice;
use crate::i18n::tr;
use crate::manipulate::ViewMap;

/// A press this near a line or a node (screen points) takes it.
const HIT: f32 = 6.0;

/// What is selected in hooping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoopingPick {
    Hooping(usize),
    Split(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// A hooping; its center is this far from the pointer.
    Hooping {
        k: usize,
        grab: Point,
    },
    /// A split line, from where the pointer was last.
    Split {
        k: usize,
        last: Point,
    },
    Node {
        k: usize,
        node: usize,
    },
}

/// One hooping, made from the layout, with its stitch plan.
pub struct HoopingView {
    pub out: HoopingOut,
    pub plan: emb_stitch::StitchPlan,
    pub segments: Vec<(Segment, Color32)>,
}

/// The state of hooping mode (not part of the design).
#[derive(Default)]
pub struct HoopingUi {
    pub mode: bool,
    pub selected: Option<HoopingPick>,
    /// The points of a split line that is being drawn (view coordinates).
    pub drawing: Option<Vec<Point>>,
    drag: Option<Drag>,
    pub preview: bool,
    /// The hoopings of a design state and a hoop.
    cache: Option<(u64, Option<HoopPreset>, Arc<Vec<HoopingView>>)>,
}

/// The color of hooping `k`.
pub fn hooping_color(k: usize) -> Color32 {
    let c = emb_render::hooping_sheet::HOOPING_COLORS;
    color32(c[k % c.len()])
}

/// The distance from `p` to the segment `a`–`b`.
fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

impl AppState {
    /// The hoop of the layout, or the hoop of the design.
    pub fn layout_hoop(&self) -> Option<HoopPreset> {
        let d = self.doc.design();
        match (&d.hooping, &d.hoop.selection) {
            (Some(l), _) => self.hoops.get(&l.hoop).cloned(),
            (None, HoopSelection::Preset { id }) => self.hoops.get(id).cloned(),
            (None, HoopSelection::OpenField) => None,
        }
    }

    /// Turns hooping mode on or off. On with no layout: Auto layout.
    pub fn set_hooping_mode(&mut self, on: bool) {
        self.hoopings_ui.mode = on;
        self.hoopings_ui.drawing = None;
        self.hoopings_ui.drag = None;
        if on {
            self.set_tool(crate::draw::Tool::Select);
            if self.doc.design().hooping.is_none() {
                self.auto_layout();
            }
        }
    }

    /// Makes the fewest hoopings that cover the design (the values of the
    /// layout, or the defaults).
    pub fn auto_layout(&mut self) {
        let Some(hoop) = self.layout_hoop() else {
            self.message = tr("Select a hoop first (Design > Machine and Hoop).").into();
            return;
        };
        let old = self.doc.design().hooping.clone();
        let overlap = old.as_ref().map_or(DEFAULT_OVERLAP_MM, |l| l.overlap_mm);
        let margin = old.as_ref().map_or(self.margin_mm, |l| l.margin_mm);
        let Some(mut layout) = HoopingLayout::auto(self.doc.design(), &hoop, overlap, margin) else {
            self.message = tr("The design has no stitches to put in hoopings.").into();
            return;
        };
        if let Some(o) = old {
            layout.join_mm = o.join_mm;
            layout.marks = o.marks;
        }
        let n = layout.hoopings.len();
        self.doc.edit("Auto Layout", |d| d.hooping = Some(layout));
        self.hoopings_ui.mode = true;
        self.hoopings_ui.selected = None;
        self.message = crate::i18n::fill(
            "{} hooping{} of the {}",
            &[&n, &(if n == 1 { "" } else { "s" }), &hoop.name],
        );
    }

    /// Changes the layout as one undo step. With `key`, a drag is one step.
    fn edit_layout(&mut self, label: &str, key: Option<&str>, f: impl FnOnce(&mut HoopingLayout)) {
        let edit = |d: &mut emb_model::Design| {
            if let Some(l) = d.hooping.as_mut() {
                f(l);
            }
        };
        match key {
            Some(k) => self.doc.edit_merged(label, MergeKey::new(k), edit),
            None => self.doc.edit(label, edit),
        }
    }

    /// Adds a hooping at the middle of the canvas.
    pub fn add_hooping(&mut self) {
        let center = self.view.center;
        let turned = self
            .doc
            .design()
            .hooping
            .as_ref()
            .and_then(|l| l.hoopings.last())
            .is_some_and(|h| h.turned);
        self.edit_layout("Add Hooping", None, |l| {
            l.hoopings.push(Hooping {
                center,
                turned,
                position: None,
            })
        });
        let n = self.doc.design().hooping.as_ref().map_or(0, |l| l.hoopings.len());
        self.hoopings_ui.selected = n.checked_sub(1).map(HoopingPick::Hooping);
    }

    /// Deletes the selected hooping or split line.
    pub fn delete_hooping_pick(&mut self) {
        match self.hoopings_ui.selected.take() {
            Some(HoopingPick::Hooping(k)) => self.edit_layout("Delete hooping", None, |l| {
                if k < l.hoopings.len() {
                    l.hoopings.remove(k);
                }
            }),
            Some(HoopingPick::Split(k)) => self.edit_layout("Delete split line", None, |l| {
                if k < l.splits.len() {
                    l.splits.remove(k);
                }
            }),
            None => self.message = tr("Select a hooping or a split line first.").into(),
        }
    }

    /// Turns the selected hooping 90° (or back). The positions of a
    /// multi-position hoop turn together.
    pub fn turn_hooping(&mut self) {
        let Some(HoopingPick::Hooping(k)) = self.hoopings_ui.selected else {
            return;
        };
        let hoop = self.layout_hoop();
        self.edit_layout("Turn Hooping", None, |l| {
            match (l.hoopings.get(k).and_then(|h| h.position), hoop) {
                (Some(_), Some(hoop)) => l.turn_positions(&hoop),
                _ => {
                    if let Some(h) = l.hoopings.get_mut(k) {
                        h.turned = !h.turned;
                    }
                }
            }
        });
    }

    /// Starts or stops drawing a split line.
    pub fn toggle_draw_split(&mut self) {
        let ui = &mut self.hoopings_ui;
        ui.drawing = if ui.drawing.is_some() { None } else { Some(Vec::new()) };
        ui.mode = true;
        self.message = tr("Click the points of the split line; Enter or a double-click ends it.").into();
    }

    /// Ends the split line that is being drawn (two points or more).
    fn finish_split(&mut self) {
        let Some(pts) = self.hoopings_ui.drawing.take() else {
            return;
        };
        let mut pts = pts;
        pts.dedup_by(|a, b| a.distance(*b) < 0.5);
        if pts.len() < 2 {
            return;
        }
        self.edit_layout("Add split line", None, |l| l.splits.push(pts));
        let n = self.doc.design().hooping.as_ref().map_or(0, |l| l.splits.len());
        self.hoopings_ui.selected = n.checked_sub(1).map(HoopingPick::Split);
    }

    /// Removes the layout.
    pub fn clear_hoopings(&mut self) {
        self.doc.edit("Remove Hoopings", |d| d.hooping = None);
        self.hoopings_ui.selected = None;
        self.hoopings_ui.drawing = None;
    }

    /// The hoopings of the current design, with their stitch plans. While a
    /// drag goes on, the hoopings of the state before the drag.
    pub fn hooping_views(&mut self) -> Arc<Vec<HoopingView>> {
        let state = self.doc.state_id();
        let hoop = self.layout_hoop();
        if let Some((s, h, v)) = &self.hoopings_ui.cache
            && ((*s == state && *h == hoop) || self.hoopings_ui.drag.is_some())
        {
            return v.clone();
        }
        let views = match hoop.clone() {
            Some(hoop) => {
                let opts = self.plan_options();
                self.doc
                    .design()
                    .hoopings(&hoop)
                    .into_iter()
                    .map(|out| {
                        let plan = emb_stitch::build_plan(&out.design, &opts, &mut Default::default());
                        let back = out.design.from_hoop_transform();
                        let segments = emb_render::segments(&plan.stitches)
                            .into_iter()
                            .map(|mut s| {
                                s.a = back.apply(s.a);
                                s.b = back.apply(s.b);
                                (s, color32(s.color))
                            })
                            .collect();
                        HoopingView { out, plan, segments }
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        let v = Arc::new(views);
        self.hoopings_ui.cache = Some((state, hoop, v.clone()));
        v
    }

    /// Writes one machine file (and the files of the Output design
    /// profile) for each hooping to `folder` as `name_1.*`, `name_2.*`, …,
    /// and the hooping sheet `name hoopings.pdf`. Returns the files.
    pub fn output_hoopings(&mut self, folder: &Path, name: &str) -> Result<Vec<PathBuf>, String> {
        let views = self.hooping_views();
        if views.is_empty() {
            return Err("The design has no hoopings. Use Auto layout first.".into());
        }
        let hoop = self
            .layout_hoop()
            .ok_or("The hoop of the layout is not in the hoop list.")?;
        let profile = self.settings.output.clone();
        let mut written = Vec::new();
        let mut rows = Vec::new();
        let first = views[0].out.design.hoop.center;
        let layout = self.doc.design().hooping.clone().unwrap_or_else(|| HoopingLayout {
            hoop: hoop.id.clone(),
            hoopings: Vec::new(),
            splits: Vec::new(),
            overlap_mm: DEFAULT_OVERLAP_MM,
            join_mm: DEFAULT_JOIN_MM,
            margin_mm: self.margin_mm,
            marks: true,
        });
        for (k, v) in views.iter().enumerate() {
            let file = format!("{name}_{}", k + 1);
            let files = crate::output::write_files(&v.out.design, &v.plan, self.catalogs(), &profile, folder, &file)?;
            let machine: Vec<String> = files
                .iter()
                .filter(|p| {
                    p.extension()
                        .is_some_and(|e| emb_formats::WRITE_EXTENSIONS.contains(&&*e.to_string_lossy()))
                })
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect();
            let is_mark = |o: &&std::sync::Arc<emb_model::DesignObject>| {
                o.name.as_deref().is_some_and(|n| n.starts_with("Registration mark"))
            };
            let objs = v.out.design.objects();
            let rot = self.doc.design().rotation_transform();
            rows.push(emb_render::hooping_sheet::HoopingRow {
                files: machine.join(", "),
                stitches: v.plan.stitches.stitch_count(),
                colors: v.plan.stitches.color_count(),
                turned: layout.hoopings.get(k).is_some_and(|h| h.turned),
                offset: rot.apply(v.out.design.hoop.center) - rot.apply(first),
                marks_first: objs.first().is_some_and(|o| is_mark(&o)),
                marks_last: objs.last().is_some_and(|o| is_mark(&o)),
                position: layout.hoopings.get(k).and_then(|h| h.position),
            });
            written.extend(files);
        }
        let sheet = emb_render::hooping_sheet::HoopingSheet {
            positions: !hoop.positions.is_empty(),
            name: name.to_string(),
            hoop_name: hoop.name.clone(),
            date: crate::output::today(),
            segments: self.plan_view().segments.iter().map(|(s, _)| *s).collect(),
            fields: layout
                .hoopings
                .iter()
                .map(|h| h.field(&hoop, layout.margin_mm))
                .collect(),
            rows,
            warnings: views.iter().flat_map(|v| v.out.warnings.clone()).collect(),
        };
        let pdf = emb_render::pdf::write(&[emb_render::hooping_sheet::overview(&sheet, profile.paper)], name);
        let p = folder.join(format!("{name} hoopings.pdf"));
        std::fs::write(&p, pdf).map_err(|e| format!("{}: {e}", p.display()))?;
        written.push(p);
        Ok(written)
    }

    /// Asks for a folder, then writes the hoopings there.
    pub fn output_hoopings_dialog(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Output hoopings: choose a folder")
            .set_directory(self.dialog_dir())
            .pick_folder()
        else {
            return;
        };
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.doc.design().name.clone());
        match self.output_hoopings(&folder, &name) {
            Ok(files) => {
                self.message = crate::i18n::fill("Wrote {} files to {}", &[&(files.len()), &(folder.display())]);
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Output Hoopings".into(),
                    lines: vec![e],
                })
            }
        }
    }
}

/// Takes Enter, Esc, Backspace and Delete in hooping mode. Returns `true`
/// if it used a key.
pub fn handle_keys(ctx: &egui::Context, st: &mut AppState) -> bool {
    let [enter, esc, back, del] = ctx.input_mut(|i| {
        [Key::Enter, Key::Escape, Key::Backspace, Key::Delete].map(|k| i.consume_key(Modifiers::NONE, k))
    });
    if st.hoopings_ui.drawing.is_some() {
        if enter {
            st.finish_split();
        } else if esc {
            st.hoopings_ui.drawing = None;
        } else if (back || del)
            && let Some(pts) = st.hoopings_ui.drawing.as_mut()
        {
            pts.pop();
        }
        return enter || esc || back || del;
    }
    if esc {
        if st.hoopings_ui.selected.is_some() {
            st.hoopings_ui.selected = None;
        } else {
            st.set_hooping_mode(false);
        }
        return true;
    }
    if (back || del) && st.hoopings_ui.selected.is_some() {
        st.delete_hooping_pick();
        return true;
    }
    false
}

/// What is under the pointer: a node of the selected line, a line, a field.
fn hit(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<(HoopingPick, Option<usize>)> {
    let layout = st.doc.design().hooping.as_ref()?;
    let to = |p: Point| map.view.to_screen(map.rect, p);
    if let Some(HoopingPick::Split(k)) = st.hoopings_ui.selected
        && let Some(line) = layout.splits.get(k)
        && let Some(n) = line.iter().position(|p| to(*p).distance(pos) <= HIT)
    {
        return Some((HoopingPick::Split(k), Some(n)));
    }
    for (k, line) in layout.splits.iter().enumerate() {
        if line
            .windows(2)
            .any(|w| segment_distance(pos, to(w[0]), to(w[1])) <= HIT)
        {
            return Some((HoopingPick::Split(k), None));
        }
    }
    let hoop = st.layout_hoop()?;
    let p = map.view.to_mm(map.rect, pos);
    layout
        .hoopings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.field(&hoop, 0.0).contains(p))
        .min_by(|a, b| a.1.center.distance(p).total_cmp(&b.1.center.distance(p)))
        .map(|(k, _)| (HoopingPick::Hooping(k), None))
}

/// The canvas input in hooping mode (left button; right and middle pan).
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    let to_mm = |pos: Pos2| map.view.to_mm(map.rect, pos);
    if st.hoopings_ui.drawing.is_some() {
        // A click adds a point (the second click of a double-click adds
        // its point too, if it is a new place), then a double-click ends.
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(pts) = st.hoopings_ui.drawing.as_mut()
        {
            let p = to_mm(pos);
            if pts.last().is_none_or(|q| q.distance(p) > 0.5) {
                pts.push(p);
            }
        }
        if response.double_clicked() || response.secondary_clicked() {
            st.finish_split();
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        return;
    }
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        let h = hit(st, map, origin);
        st.hoopings_ui.selected = h.map(|(p, _)| p);
        let at = to_mm(origin);
        let layout = st.doc.design().hooping.clone();
        st.hoopings_ui.drag = match (h, layout) {
            (Some((HoopingPick::Split(k), Some(node))), _) => Some(Drag::Node { k, node }),
            (Some((HoopingPick::Split(k), None)), _) => Some(Drag::Split { k, last: at }),
            (Some((HoopingPick::Hooping(k), _)), Some(l)) => {
                l.hoopings.get(k).map(|h| Drag::Hooping { k, grab: h.center - at })
            }
            _ => None,
        };
    }
    if response.dragged_by(egui::PointerButton::Primary)
        && let (Some(drag), Some(pos)) = (st.hoopings_ui.drag, response.interact_pointer_pos())
    {
        let at = to_mm(pos);
        match drag {
            Drag::Hooping { k, grab } => st.edit_layout("Move hooping", Some("hooping_drag"), |l| {
                let Some(h) = l.hoopings.get(k).copied() else { return };
                if h.position.is_some() {
                    // The positions of a multi-position hoop move together.
                    l.move_positions(at + grab - h.center);
                } else if let Some(h) = l.hoopings.get_mut(k) {
                    h.center = at + grab;
                }
            }),
            Drag::Split { k, last } => {
                let d = at - last;
                st.edit_layout("Move split line", Some("hooping_drag"), |l| {
                    if let Some(line) = l.splits.get_mut(k) {
                        for p in line {
                            *p += d;
                        }
                    }
                });
                st.hoopings_ui.drag = Some(Drag::Split { k, last: at });
            }
            Drag::Node { k, node } => st.edit_layout("Move split line", Some("hooping_drag"), |l| {
                if let Some(p) = l.splits.get_mut(k).and_then(|line| line.get_mut(node)) {
                    *p = at;
                }
            }),
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        st.hoopings_ui.drag = None;
        st.doc.end_merge();
    }
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        st.hoopings_ui.selected = hit(st, map, pos).map(|(p, _)| p);
    }
    if let Some(pos) = response.hover_pos() {
        match hit(st, map, pos) {
            Some((HoopingPick::Split(_), _)) => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
            Some((HoopingPick::Hooping(_), _)) => ui.ctx().set_cursor_icon(egui::CursorIcon::Move),
            None => {}
        }
    }
}

/// Draws the hoopings and the split lines.
pub fn draw(painter: &egui::Painter, rect: Rect, st: &mut AppState) {
    let Some(layout) = st.doc.design().hooping.clone() else {
        return;
    };
    let Some(hoop) = st.layout_hoop() else { return };
    let view = st.view;
    let to = |p: Point| view.to_screen(rect, p);
    for (k, h) in layout.hoopings.iter().enumerate() {
        let color = hooping_color(k);
        let selected = st.hoopings_ui.selected == Some(HoopingPick::Hooping(k));
        let width = if selected { 3.0 } else { 1.5 };
        let turn =
            emb_geom::Transform::rotate_deg(h.angle_deg()).then(emb_geom::Transform::translate(h.center.x, h.center.y));
        if let Some(field) = h.hoop_of(&hoop).outline(Point::ZERO, 0.0) {
            for pl in field.transformed(&turn).flatten(view.tolerance()) {
                let mut pts: Vec<Pos2> = pl.points.iter().map(|p| to(*p)).collect();
                pts.extend(pts.first().copied());
                painter.add(Shape::line(pts, Stroke::new(width, color)));
            }
        }
        let usable = h.field(&hoop, layout.margin_mm);
        let r = Rect::from_two_pos(to(usable.min), to(usable.max));
        painter.extend(Shape::dashed_line(
            &[
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ],
            Stroke::new(1.0, color.gamma_multiply(0.7)),
            6.0,
            4.0,
        ));
        if selected {
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.06));
        }
        let corner = to(h.field(&hoop, 0.0).min) + Vec2::new(6.0, 4.0);
        painter.text(
            corner,
            egui::Align2::LEFT_TOP,
            format!("{}", k + 1),
            egui::FontId::proportional(22.0),
            color,
        );
        let c = to(h.center);
        painter.line_segment([c - Vec2::X * 6.0, c + Vec2::X * 6.0], Stroke::new(1.0, color));
        painter.line_segment([c - Vec2::Y * 6.0, c + Vec2::Y * 6.0], Stroke::new(1.0, color));
    }
    let split_color = Color32::from_rgb(255, 140, 0);
    for (k, line) in layout.splits.iter().enumerate() {
        let selected = st.hoopings_ui.selected == Some(HoopingPick::Split(k));
        let pts: Vec<Pos2> = line.iter().map(|p| to(*p)).collect();
        painter.add(Shape::line(
            pts.clone(),
            Stroke::new(if selected { 3.0 } else { 2.0 }, split_color),
        ));
        if selected {
            for p in pts {
                painter.rect_filled(Rect::from_center_size(p, Vec2::splat(7.0)), 1.0, split_color);
            }
        }
    }
    if let Some(pts) = &st.hoopings_ui.drawing {
        let mut screen: Vec<Pos2> = pts.iter().map(|p| to(*p)).collect();
        if let Some(c) = st.cursor_mm {
            screen.push(to(c));
        }
        if screen.len() >= 2 {
            painter.extend(Shape::dashed_line(&screen, Stroke::new(2.0, split_color), 8.0, 4.0));
        }
        for p in pts {
            painter.circle_filled(to(*p), 3.5, split_color);
        }
    }
}

/// The hooping options in the ribbon: the hoop, the overlaps, the margin, the marks.
pub fn options(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(layout) = st.doc.design().hooping.clone() else {
        ui.weak(tr("Use Auto layout to make hoopings."));
        return;
    };
    let units = st.settings.units;
    let mut l = layout.clone();
    egui::Grid::new("hooping_options").num_columns(4).spacing([6.0, 3.0]).show(ui, |ui| {
        ui.label(tr("Hoop"));
        let name = st.hoops.get(&l.hoop).map_or(l.hoop.clone(), |h| h.name.clone());
        egui::ComboBox::from_id_salt("layout_hoop")
            .selected_text(name)
            .width(200.0)
            .show_ui(ui, |ui| {
                for h in &st.hoops.hoops {
                    ui.selectable_value(&mut l.hoop, h.id.clone(), format!("{} ({} × {} mm)", h.name, h.width_mm, h.height_mm));
                }
            });
        ui.label(tr("Margin"));
        crate::workspace::length_value(ui, &mut l.margin_mm, units, 0.0, 30.0)
            .on_hover_text(tr("The space between the field edge and the stitches."));
        ui.end_row();
        ui.label(tr("Field overlap"));
        crate::workspace::length_value(ui, &mut l.overlap_mm, units, 0.0, 100.0)
            .on_hover_text(tr("How far the fields of two hoopings overlap at least (for Auto layout). The registration marks go there."));
        ui.label(tr("Join overlap"));
        crate::workspace::length_value(ui, &mut l.join_mm, units, 0.0, 5.0)
            .on_hover_text(tr("How far a cut fill goes into the next hooping, so no fabric shows at the join."));
        ui.end_row();
        ui.checkbox(&mut l.marks, tr("Registration marks"))
            .on_hover_text(tr("Crosses where two hoopings meet: the first hooping sews them last, the next one first."));
        ui.end_row();
    });
    if l != layout {
        st.edit_layout("Hooping options", Some("hooping_options"), |x| *x = l);
    }
}

/// Size of one picture in Preview hoopings.
const THUMB: Vec2 = Vec2::new(230.0, 175.0);

/// The Preview hoopings window: a picture of each hooping, its numbers and warnings.
pub fn preview_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.hoopings_ui.preview {
        return;
    }
    let mut open = true;
    let views = st.hooping_views();
    let hoop = st.layout_hoop();
    let mut pick = None;
    egui::Window::new(tr("Preview Hoopings"))
        .id(egui::Id::new("Preview Hoopings"))
        .open(&mut open)
        .resizable(true)
        .default_width(2.0 * THUMB.x + 40.0)
        .show(ctx, |ui| {
            if views.is_empty() {
                ui.label(tr("The design has no hoopings. Use Hooping > Auto Layout."));
                return;
            }
            let total: usize = views.iter().map(|v| v.plan.stitches.stitch_count()).sum();
            ui.label(crate::i18n::fill(
                "{} hoopings, {} stitches. They sew in the order of their numbers.",
                &[&(views.len()), &total],
            ));
            egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (k, v) in views.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_width(THUMB.x);
                            let (resp, painter) = ui.allocate_painter(THUMB, Sense::click());
                            let rect = resp.rect;
                            painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
                            let field = hoop.as_ref().and_then(|h| {
                                st.doc
                                    .design()
                                    .hooping
                                    .as_ref()
                                    .and_then(|l| l.hoopings.get(k))
                                    .map(|g| g.field(h, 0.0))
                            });
                            let mut view = CanvasView::default();
                            if let Some(b) =
                                field.or_else(|| BBox::from_points(v.segments.iter().flat_map(|(s, _)| [s.a, s.b])))
                            {
                                view.fit(rect, b);
                            }
                            let (mesh, _) = stitch_mesh(rect, &view, &v.segments, false);
                            painter.with_clip_rect(rect).add(Shape::mesh(mesh));
                            if let Some(f) = field {
                                let r = Rect::from_two_pos(view.to_screen(rect, f.min), view.to_screen(rect, f.max));
                                painter.rect_stroke(
                                    r,
                                    0.0,
                                    Stroke::new(1.5, hooping_color(k)),
                                    egui::StrokeKind::Middle,
                                );
                            }
                            if resp.clicked() {
                                pick = Some(k);
                            }
                            let position = st
                                .doc
                                .design()
                                .hooping
                                .as_ref()
                                .and_then(|l| l.hoopings.get(k))
                                .and_then(|h| h.position);
                            let title = match position {
                                Some(p) => format!("Hooping {} (position {})", k + 1, p + 1),
                                None => format!("Hooping {}", k + 1),
                            };
                            ui.colored_label(hooping_color(k), title);
                            ui.label(crate::i18n::fill(
                                "{} stitches, {} colors",
                                &[&(v.plan.stitches.stitch_count()), &(v.plan.stitches.color_count())],
                            ));
                            for w in &v.out.warnings {
                                ui.colored_label(ui.visuals().warn_fg_color, w);
                            }
                        });
                    }
                });
            });
        });
    if let Some(k) = pick {
        st.hoopings_ui.mode = true;
        st.hoopings_ui.selected = Some(HoopingPick::Hooping(k));
    }
    st.hoopings_ui.preview = open;
}

#[cfg(test)]
mod tests {
    use emb_model::ObjectKind;

    use super::*;
    use crate::test_util::Harness;

    /// The corpus design in four hoopings, in the app.
    fn big() -> Harness {
        let mut h = Harness::empty();
        let d = emb_cli::corpus::build("multi-hoop").unwrap();
        h.st.doc = emb_model::Document::new(d);
        h.st.fit_requested = true;
        h.frame(vec![]);
        h
    }

    #[test]
    fn mode_on_makes_a_layout_and_auto_layout_undoes() {
        let mut h = big();
        h.st.doc.edit("No layout", |d| d.hooping = None);
        h.st.set_hooping_mode(true);
        let l = h.st.doc.design().hooping.clone().unwrap();
        assert_eq!(l.hoopings.len(), 4);
        h.st.doc.undo();
        assert!(h.st.doc.design().hooping.is_none());
    }

    #[test]
    fn a_drag_in_a_field_moves_the_hooping_in_one_step() {
        let mut h = big();
        h.st.set_hooping_mode(true);
        let before = h.st.doc.design().hooping.clone().unwrap().hoopings[0].center;
        let undo = h.st.doc.undo_len();
        // Press near the center of hooping 1 (no split line there) and drag 20 mm right.
        let map = h.map();
        let p = before + Point::new(-30.0, -30.0);
        let from = map.view.to_screen(map.rect, p);
        let to = map.view.to_screen(map.rect, p + Point::new(20.0, 0.0));
        h.drag(egui::PointerButton::Primary, from, to, Modifiers::NONE);
        let after = h.st.doc.design().hooping.clone().unwrap().hoopings[0].center;
        assert!(
            (after - before).distance(Point::new(20.0, 0.0)) < 0.5,
            "{before:?} {after:?}"
        );
        assert_eq!(h.st.doc.undo_len(), undo + 1);
        assert_eq!(h.st.hoopings_ui.selected, Some(HoopingPick::Hooping(0)));
    }

    #[test]
    fn draw_a_split_line_then_delete_it() {
        let mut h = big();
        h.st.set_hooping_mode(true);
        let n = h.st.doc.design().hooping.as_ref().unwrap().splits.len();
        h.st.toggle_draw_split();
        let map = h.map();
        for p in [Point::new(-60.0, -120.0), Point::new(-60.0, 120.0)] {
            let pos = map.view.to_screen(map.rect, p);
            h.click(egui::PointerButton::Primary, pos);
        }
        h.key(Key::Enter);
        let l = h.st.doc.design().hooping.clone().unwrap();
        assert_eq!(l.splits.len(), n + 1);
        assert_eq!(h.st.hoopings_ui.selected, Some(HoopingPick::Split(n)));
        h.key(Key::Delete);
        assert_eq!(h.st.doc.design().hooping.as_ref().unwrap().splits.len(), n);
        // The objects are not deleted.
        assert!(!h.st.doc.design().objects().is_empty());
    }

    #[test]
    fn add_turn_and_delete_a_hooping() {
        let mut h = big();
        h.st.set_hooping_mode(true);
        h.st.add_hooping();
        let l = h.st.doc.design().hooping.clone().unwrap();
        assert_eq!(l.hoopings.len(), 5);
        // The new hooping has the orientation of the last one.
        assert_eq!(l.hoopings[4].turned, l.hoopings[3].turned);
        h.st.turn_hooping();
        assert_ne!(
            h.st.doc.design().hooping.as_ref().unwrap().hoopings[4].turned,
            l.hoopings[4].turned
        );
        h.st.delete_hooping_pick();
        assert_eq!(h.st.doc.design().hooping.as_ref().unwrap().hoopings.len(), 4);
        h.st.clear_hoopings();
        assert!(h.st.doc.design().hooping.is_none());
    }

    #[test]
    fn output_writes_a_file_for_each_hooping_and_the_sheet() {
        let mut h = big();
        let dir = std::env::temp_dir().join(format!("emb-hoopings-{}", std::process::id()));
        let files = h.st.output_hoopings(&dir, "big").unwrap();
        let names: Vec<String> = files
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        for k in 1..=4 {
            assert!(names.contains(&format!("big_{k}.vp3")), "{names:?}");
        }
        assert!(names.contains(&"big hoopings.pdf".to_string()));
        // The VP3 of hooping 1 reads back with the stitches of its plan.
        let bytes = std::fs::read(dir.join("big_1.vp3")).unwrap();
        let back = emb_formats::vp3::read(&bytes).unwrap();
        let views = h.st.hooping_views();
        assert_eq!(back.stitch_count(), views[0].plan.stitches.stitch_count());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_preview_and_the_ribbon_options_draw_without_changes() {
        let mut h = big();
        h.st.set_hooping_mode(true);
        h.st.hoopings_ui.preview = true;
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                preview_window(ui.ctx(), &mut h.st);
                options(ui, &mut h.st);
            });
            out.textures_delta.clear();
            h.frame(vec![]);
        }
        assert_eq!(h.st.doc.state_id(), state);
        let views = h.st.hooping_views();
        assert_eq!(views.len(), 4);
        assert!(views.iter().all(|v| v.out.warnings.is_empty()));
        // A hooping keeps the cut pieces of the frame.
        let fills = views[0]
            .out
            .design
            .objects()
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Fill { .. }))
            .count();
        assert_eq!(fills, 2);
    }

    #[test]
    fn export_points_to_output_hoopings_and_a_closed_tab_ends_the_mode() {
        let mut h = big();
        let r = h.st.export_report();
        assert!(
            r.warnings.iter().any(|w| w.text.contains("Output Hoopings")),
            "{:?}",
            r.warnings
        );
        h.st.set_hooping_mode(true);
        h.st.hoopings_ui.selected = Some(HoopingPick::Hooping(2));
        h.st.close_tab();
        assert!(!h.st.hoopings_ui.mode);
        assert!(h.st.hoopings_ui.selected.is_none());
    }

    #[test]
    fn the_positions_of_a_hoop_move_and_turn_together() {
        let mut h = Harness::empty();
        h.st.add_user_hoops(emb_model::HoopCatalog {
            hoops: vec![emb_cli::corpus::test_position_hoop()],
        });
        h.st.doc = emb_model::Document::new(emb_cli::corpus::build("multi-position").unwrap());
        h.st.fit_requested = true;
        h.frame(vec![]);
        h.st.set_hooping_mode(true);
        let before = h.st.doc.design().hooping.clone().unwrap();
        assert_eq!(before.hoopings.len(), 2);
        // A drag in the field of position 1 moves both positions and the split line.
        let map = h.map();
        let p = before.hoopings[0].center + Point::new(-30.0, -30.0);
        let from = map.view.to_screen(map.rect, p);
        // 8 mm: the banner stays inside the safety margin of the fields.
        let to = map.view.to_screen(map.rect, p + Point::new(0.0, 8.0));
        h.drag(egui::PointerButton::Primary, from, to, Modifiers::NONE);
        let after = h.st.doc.design().hooping.clone().unwrap();
        for k in 0..2 {
            let d = after.hoopings[k].center - before.hoopings[k].center;
            assert!(d.distance(Point::new(0.0, 8.0)) < 0.5, "{k}: {d:?}");
        }
        assert!((after.splits[0][0].y - before.splits[0][0].y - 8.0).abs() < 0.5);
        // Turn: both positions turn about the hoop center.
        h.st.hoopings_ui.selected = Some(HoopingPick::Hooping(1));
        h.st.turn_hooping();
        let turned = h.st.doc.design().hooping.clone().unwrap();
        assert!(turned.hoopings.iter().all(|g| g.turned));
        assert!((turned.hoopings[0].center.x - turned.hoopings[1].center.x).abs() < 1e-6);
        // The hoopings are the two positions, with the fields of the positions.
        h.st.doc.undo();
        let views = h.st.hooping_views();
        assert_eq!(views.len(), 2);
        assert!(
            views.iter().all(|v| v.out.warnings.is_empty()),
            "{:?}",
            views[0].out.warnings
        );
    }

    /// Writes the P7 exit test kit: `EMB_KIT_OUT=out/p7-sewout cargo test
    /// -p emb-app -- --ignored write_hooping_kit`.
    #[test]
    #[ignore]
    fn write_hooping_kit() {
        let dir = PathBuf::from(std::env::var("EMB_KIT_OUT").expect("set EMB_KIT_OUT"));
        let mut h = big();
        let mut s = h.st.settings.clone();
        s.output.worksheet = true;
        s.output.picture = true;
        h.st.settings = s;
        let files = h.st.output_hoopings(&dir.join("multi-hoop"), "multi-hoop").unwrap();
        for f in files {
            println!("{}", f.display());
        }
    }
}
