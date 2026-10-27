//! Items moved from P2 (see `docs/design/p2-plan.md`): the corner radius
//! of each corner in a dialog, a hole at a distance inside the outline,
//! the previous view and zoom factors. Snap to objects is in `workspace`.

use eframe::egui;
use emb_geom::path::PathCmd;
use emb_geom::region::{self, Join, Region};
use emb_geom::{Path, Point};
use emb_model::{ObjectId, ObjectKind};

use crate::app::AppState;
use crate::canvas::{MAX_ZOOM, MIN_ZOOM};
use crate::i18n::tr;

/// The corners of an outline made of straight sides, with a rounded corner
/// (a curve between two sides) counted as one corner with its radius.
#[derive(Debug, Clone, PartialEq)]
pub struct Corners {
    /// The corner points: where the sides meet.
    pub points: Vec<Point>,
    pub radii: Vec<f64>,
}

/// The point where the lines `a + s·u` and `b + t·v` cross.
fn cross(a: Point, u: Point, b: Point, v: Point) -> Option<Point> {
    let den = u.cross(v);
    if den.abs() < 1e-12 {
        return None;
    }
    Some(a + u * ((b - a).cross(v) / den))
}

/// Half the angle between the two sides at a corner, as tan(θ/2), where θ
/// is the inner angle; the tangent length of radius r is r / tan(θ/2).
fn half_tan(prev: Point, at: Point, next: Point) -> f64 {
    let (Some(u), Some(v)) = ((prev - at).normalized(), (next - at).normalized()) else {
        return 1.0;
    };
    let cos = u.dot(v).clamp(-1.0, 1.0);
    let theta = cos.acos();
    (theta / 2.0).tan().max(1e-6)
}

impl Corners {
    /// Reads a closed outline of lines, and of curves between lines. `None`
    /// if the outline has other curves (an ellipse) or fewer than 3 corners.
    pub fn of(path: &Path) -> Option<Corners> {
        // The outline as a list of items: a line end, or a curve.
        enum Item {
            Line(Point),
            Curve(Point, Point),
        }
        let mut start = None;
        let mut items = Vec::new();
        let mut pen = Point::ZERO;
        for c in &path.cmds {
            match *c {
                PathCmd::MoveTo(p) => {
                    if start.is_some() {
                        return None;
                    }
                    start = Some(p);
                    pen = p;
                }
                PathCmd::LineTo(p) => {
                    if p != pen {
                        items.push(Item::Line(p));
                    }
                    pen = p;
                }
                PathCmd::CubicTo(_, _, p) => {
                    items.push(Item::Curve(pen, p));
                    pen = p;
                }
                PathCmd::Close => {}
            }
        }
        let s = start?;
        if pen != s {
            items.push(Item::Line(s));
        }
        // Each curve must be between two lines. A corner of lines is where
        // two lines meet; a rounded corner is where the lines next to a
        // curve would meet.
        let n = items.len();
        let mut points = Vec::new();
        let mut radii = Vec::new();
        let end_of = |i: usize| match items[i % n] {
            Item::Line(p) => p,
            Item::Curve(_, p) => p,
        };
        let start_of = |i: usize| end_of(i + n - 1);
        for i in 0..n {
            match items[i] {
                Item::Line(p) => {
                    // A point where the outline goes straight on is not a corner.
                    let straight = (p - start_of(i)).cross(end_of(i + 1) - p).abs() < 1e-9;
                    if matches!(items[(i + 1) % n], Item::Line(_)) && !straight {
                        points.push(p);
                        radii.push(0.0);
                    }
                }
                Item::Curve(a, b) => {
                    let (Item::Line(_), Item::Line(_)) = (&items[(i + n - 1) % n], &items[(i + 1) % n]) else {
                        return None;
                    };
                    let u = a - start_of(i + n - 1);
                    let v = b - end_of(i + 1);
                    let c = cross(a, u, b, v)?;
                    let r = a.distance(c) * half_tan(start_of(i + n - 1), c, end_of(i + 1));
                    points.push(c);
                    radii.push(r);
                }
            }
        }
        (points.len() >= 3).then_some(Corners { points, radii })
    }

    /// The outline with each corner rounded by its radius. A radius is
    /// made smaller where the sides are too short for it.
    pub fn to_path(&self) -> Path {
        let n = self.points.len();
        let p = &self.points;
        // The tangent length at each corner, limited to half of each side.
        let tangent: Vec<f64> = (0..n)
            .map(|i| {
                let (prev, next) = (p[(i + n - 1) % n], p[(i + 1) % n]);
                let t = self.radii[i].max(0.0) / half_tan(prev, p[i], next);
                // A little less than half of each side, so that a short line
                // stays between two corners and the outline reads back.
                t.min(p[i].distance(prev) * 0.499).min(p[i].distance(next) * 0.499)
            })
            .collect();
        // Start in the middle of the last side, so that corner 0 is the first
        // corner again when the outline is read back.
        let mut path = Path::new().move_to(p[n - 1].midpoint(p[0]));
        for i in 0..n {
            let (prev, at, next) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
            let t = tangent[i];
            if t <= 1e-9 {
                path = path.line_to(at);
                continue;
            }
            let (Some(u), Some(v)) = ((prev - at).normalized(), (next - at).normalized()) else {
                continue;
            };
            let (a, b) = (at + u * t, at + v * t);
            path = path.line_to(a);
            // A cubic near a circle arc: the control points are 0.55 of the
            // way to the corner for a right angle (Path::rounded_rect uses it too).
            let k = 0.552_284_75;
            path = path.cubic_to(a + (at - a) * k, b + (at - b) * k, b);
        }
        path.close()
    }
}

/// The outline of the object that the corner dialog edits.
fn outline(kind: &ObjectKind) -> Option<&Path> {
    match kind {
        ObjectKind::Fill { outer, .. } => Some(outer),
        ObjectKind::Applique { pieces, .. } if pieces.len() == 1 => Some(&pieces[0].outer),
        _ => None,
    }
}

fn outline_mut(kind: &mut ObjectKind) -> Option<&mut Path> {
    match kind {
        ObjectKind::Fill { outer, .. } => Some(outer),
        ObjectKind::Applique { pieces, .. } if pieces.len() == 1 => Some(&mut pieces[0].outer),
        _ => None,
    }
}

/// The open corner dialog: the object and its corners.
#[derive(Debug, Clone, PartialEq)]
pub struct CornerDialog {
    pub id: ObjectId,
    pub corners: Corners,
    pub all: f64,
}

/// The open "Hole Inside the Outline" dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleDialog {
    pub id: ObjectId,
    pub distance_mm: f64,
}

impl AppState {
    pub fn open_corner_dialog(&mut self) {
        let Some(id) = self.selection.single() else {
            self.message = tr("Select one fill or appliqué.").into();
            return;
        };
        match self
            .doc
            .design()
            .object(id)
            .and_then(|o| outline(&o.kind))
            .and_then(Corners::of)
        {
            Some(corners) => {
                let all = corners.radii.first().copied().unwrap_or(0.0);
                self.corner_dialog = Some(CornerDialog { id, corners, all });
            }
            None => {
                self.message = tr("Corner radius is for outlines with straight sides (and rounded corners).").into()
            }
        }
    }

    /// Gives the object the corners. A drag in the dialog is one undo step.
    pub fn set_corners(&mut self, id: ObjectId, c: &Corners, merge: bool) {
        let path = c.to_path();
        let key = emb_model::MergeKey::new(format!("{}:corners", id.0));
        self.doc.edit_merged("Corner Radius", key, |d| {
            if let Some(o) = d.object_mut(id).and_then(|o| outline_mut(&mut o.kind)) {
                *o = path;
            }
        });
        if !merge {
            self.doc.end_merge();
        }
    }

    pub fn open_hole_dialog(&mut self) {
        match self.selection.single() {
            Some(id)
                if matches!(
                    self.doc.design().object(id).map(|o| &o.kind),
                    Some(ObjectKind::Fill { .. } | ObjectKind::Applique { .. })
                ) =>
            {
                self.hole_dialog = Some(HoleDialog { id, distance_mm: 5.0 });
            }
            _ => self.message = tr("Select one fill or appliqué.").into(),
        }
    }

    /// Adds a hole: the outline moved `d` mm inward. Returns `false` if
    /// nothing is left of the outline at that distance.
    pub fn hole_from_outline(&mut self, id: ObjectId, d: f64) -> bool {
        let Some(o) = self.doc.design().object(id) else {
            return false;
        };
        let Some(outer) = outline(&o.kind).or(match &o.kind {
            ObjectKind::Applique { pieces, .. } => pieces.first().map(|p| &p.outer),
            _ => None,
        }) else {
            return false;
        };
        let rings: Vec<Region> = outer
            .flatten(0.05)
            .iter()
            .map(|pl| Region::from_polylines(pl, &[]))
            .collect();
        let inner = region::offset(&rings, -d.abs(), Join::Round);
        let holes: Vec<Path> = inner.iter().map(|r| Path::polygon(&r.outer)).collect();
        if holes.is_empty() {
            self.message = crate::i18n::fill(
                "The outline is too small for a hole {} mm inside it.",
                &[&format!("{:.1}", d)],
            );
            return false;
        }
        self.doc.edit("Hole Inside the Outline", |dsg| {
            match dsg.object_mut(id).map(|o| &mut o.kind) {
                Some(ObjectKind::Fill { holes: h, .. }) => h.extend(holes),
                Some(ObjectKind::Applique { pieces, .. }) => {
                    if let Some(p) = pieces.first_mut() {
                        p.holes.extend(holes);
                    }
                }
                _ => {}
            }
        });
        true
    }

    /// Keeps the view before a zoom or pan command, for Previous view.
    pub fn push_view(&mut self) {
        if self.view_history.last() != Some(&self.view) {
            self.view_history.push(self.view);
            if self.view_history.len() > 50 {
                self.view_history.remove(0);
            }
        }
    }

    pub fn previous_view(&mut self) {
        if let Some(v) = self.view_history.pop() {
            self.view = v;
        }
    }

    /// Sets the zoom (1.0 is 100 %) and keeps the center.
    pub fn set_zoom(&mut self, zoom: f64) {
        self.push_view();
        self.view.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

/// Zoom factors for the zoom list.
pub const ZOOM_FACTORS: [f64; 9] = [0.25, 0.5, 1.0, 1.25, 2.0, 4.0, 8.0, 16.0, 32.0];

/// The zoom list: the zoom now, and factors to pick.
pub fn zoom_combo(ui: &mut egui::Ui, st: &mut AppState) {
    let mut pick = None;
    egui::ComboBox::from_id_salt("zoom_factor")
        .selected_text(format!("{:.0} %", st.view.zoom * 100.0))
        .width(70.0)
        .show_ui(ui, |ui| {
            for f in ZOOM_FACTORS {
                if ui
                    .selectable_label((st.view.zoom - f).abs() < 1e-6, format!("{:.0} %", f * 100.0))
                    .clicked()
                {
                    pick = Some(f);
                }
            }
        })
        .response
        .on_hover_text(tr(
            "Zoom factor. 100 % is the real size (calibrate the screen in Options).",
        ));
    if let Some(f) = pick {
        st.set_zoom(f);
    }
}

/// The corner radius and hole dialogs.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    if let Some(mut dlg) = st.corner_dialog.clone() {
        let mut open = true;
        let mut changed = false;
        let mut dragging = false;
        crate::dock::docked(egui::Window::new(tr("Corner Radius")).id(egui::Id::new("Corner Radius")), ctx, egui::Id::new("Corner Radius"), st.canvas_rect)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let u = st.settings.units;
                ui.horizontal(|ui| {
                    ui.label(tr("All corners"));
                    let r = crate::workspace::length_value(ui, &mut dlg.all, u, 0.0, 200.0);
                    if r.changed() {
                        dlg.corners.radii.iter_mut().for_each(|x| *x = dlg.all);
                        changed = true;
                        dragging |= r.dragged();
                    }
                });
                ui.separator();
                egui::Grid::new("corners").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                    for i in 0..dlg.corners.points.len() {
                        ui.label(crate::i18n::fill("Corner {}", &[&(i + 1)]));
                        let r = crate::workspace::length_value(ui, &mut dlg.corners.radii[i], u, 0.0, 200.0);
                        changed |= r.changed();
                        dragging |= r.dragged();
                        if i % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
                ui.weak(tr("Corners are numbered from the first point of the outline. A radius is smaller where a side is too short."));
            });
        if changed {
            st.set_corners(dlg.id, &dlg.corners, dragging);
        }
        st.corner_dialog = open.then_some(dlg);
    }
    if let Some(mut dlg) = st.hole_dialog.clone() {
        let mut open = true;
        let mut apply = false;
        crate::dock::docked(
            egui::Window::new(tr("Hole Inside the Outline")).id(egui::Id::new("Hole Inside the Outline")),
            ctx,
            egui::Id::new("Hole Inside the Outline"),
            st.canvas_rect,
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(tr(
                "Adds a hole with the shape of the outline, at a distance inside it. Use it for a frame or a border.",
            ));
            ui.horizontal(|ui| {
                ui.label(tr("Distance"));
                crate::workspace::length_value(ui, &mut dlg.distance_mm, st.settings.units, 0.5, 200.0);
            });
            apply = ui.button(tr("Add Hole")).clicked();
        });
        if apply && st.hole_from_outline(dlg.id, dlg.distance_mm) {
            open = false;
        }
        st.hole_dialog = open.then_some(dlg);
    }
}

/// The previous view, for tests of the zoom commands.
#[cfg(test)]
pub fn last_view(st: &AppState) -> Option<crate::canvas::CanvasView> {
    st.view_history.last().copied()
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn corners_of_a_rectangle_and_a_rounded_one() {
        let r = Path::rect(BBox::new(Point::ZERO, Point::new(40.0, 20.0)));
        let c = Corners::of(&r).unwrap();
        assert_eq!(c.points.len(), 4);
        assert!(c.radii.iter().all(|x| *x == 0.0));
        let rr = Path::rounded_rect(BBox::new(Point::ZERO, Point::new(40.0, 20.0)), 5.0);
        let c = Corners::of(&rr).unwrap();
        assert_eq!(c.points.len(), 4);
        for (p, r) in c.points.iter().zip(&c.radii) {
            assert!((r - 5.0).abs() < 1e-6, "{r}");
            assert!(p.x.abs() < 1e-6 || (p.x - 40.0).abs() < 1e-6, "{p:?}");
        }
        assert!(Corners::of(&Path::ellipse(Point::ZERO, 5.0, 3.0)).is_none());
        // Rounded again, the path is the same shape (round trip).
        let back = Corners::of(&c.to_path()).unwrap();
        for (a, b) in back.radii.iter().zip(&c.radii) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn one_corner_and_too_large_radii() {
        let r = Path::rect(BBox::new(Point::ZERO, Point::new(40.0, 20.0)));
        let mut c = Corners::of(&r).unwrap();
        c.radii[2] = 8.0;
        c.radii[0] = 100.0;
        let back = Corners::of(&c.to_path()).unwrap();
        assert!((back.radii[2] - 8.0).abs() < 1e-6);
        assert!(
            (back.radii[0] - 9.98).abs() < 1e-6,
            "about half the short side: {}",
            back.radii[0]
        );
        assert_eq!(back.radii[1], 0.0);
    }

    #[test]
    fn dialog_edits_one_undo_step_and_hole_from_outline() {
        let mut h = Harness::empty();
        let c = h.st.doc.design().hoop.center;
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::from_center_size(c, 40.0, 30.0)),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        h.st.selection.set(id);
        h.st.open_corner_dialog();
        let mut dlg = h.st.corner_dialog.clone().unwrap();
        dlg.corners.radii[1] = 6.0;
        let steps = h.st.doc.undo_len();
        h.st.set_corners(id, &dlg.corners, true);
        dlg.corners.radii[1] = 7.0;
        h.st.set_corners(id, &dlg.corners, false);
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let outer = match &h.st.doc.design().object(id).unwrap().kind {
            ObjectKind::Fill { outer, .. } => outer.clone(),
            _ => panic!(),
        };
        assert!((Corners::of(&outer).unwrap().radii[1] - 7.0).abs() < 1e-6);

        assert!(h.st.hole_from_outline(id, 5.0));
        let holes = match &h.st.doc.design().object(id).unwrap().kind {
            ObjectKind::Fill { holes, .. } => holes.clone(),
            _ => panic!(),
        };
        assert_eq!(holes.len(), 1);
        let b = holes[0].control_bbox().unwrap();
        assert!(
            (b.width() - 30.0).abs() < 0.2 && (b.height() - 20.0).abs() < 0.2,
            "{b:?}"
        );
        assert!(!h.st.hole_from_outline(id, 50.0), "too small");
        // The windows draw.
        h.st.open_hole_dialog();
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| windows(&ui.ctx().clone(), st));
        out.textures_delta.clear();
    }

    #[test]
    fn previous_view_and_zoom_factor() {
        let mut h = Harness::new();
        let v0 = h.st.view;
        h.st.set_zoom(4.0);
        assert_eq!(h.st.view.zoom, 4.0);
        assert_eq!(last_view(&h.st), Some(v0));
        h.st.set_zoom(8.0);
        h.st.previous_view();
        assert_eq!(h.st.view.zoom, 4.0);
        h.st.previous_view();
        assert_eq!(h.st.view, v0);
        h.st.previous_view();
        assert_eq!(h.st.view, v0, "no more history");
    }
}
