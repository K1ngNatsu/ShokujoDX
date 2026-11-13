//! Drawing aids from drawing programs (owner request, 2026-11-13, ADR 0092):
//!
//! - Smart Fill: a click inside an area that lines (and the outlines of
//!   other objects) enclose makes a fill of that area (`emb_geom::linework::enclosed_area`).
//! - Trim Segment: a click on a line removes its part between the crossings
//!   on each side of the click (`emb_geom::linework::trim_piece`).
//! - Smart Drawing: a stroke drawn by hand that is near a line, a rectangle,
//!   a triangle, a circle or an ellipse becomes that shape (`emb_geom::recognize`).
//! - Copy Properties: the stitch settings and the color of a clicked object
//!   go to the selected objects of the same kind (the color to all of them).
//! - Envelope: bends the selected objects into a shape, as lettering art does
//!   for lettering (`emb_model::Envelope::warp_kind`).

use eframe::egui;
use emb_geom::linework::{enclosed_area, trim_piece};
use emb_geom::nodes::{NodePath, SubNodes};
use emb_geom::path::Polyline;
use emb_geom::recognize::{Recognized, recognize};
use emb_geom::simplify::{Simplify, simplified};
use emb_geom::{BBox, Path, Point, Transform};
use emb_model::{DesignObject, Envelope, EnvelopeShape, MergeKey, ObjectId, ObjectKind};

use crate::app::AppState;
use crate::i18n::tr;
use crate::manipulate::ViewMap;

/// Smart Fill: lines nearer than two times this close, in mm.
const SMART_FILL_HALF_MM: f64 = 0.15;
/// A click this near a line (screen points) is on it.
const PICK_PX: f64 = 6.0;

/// The kinds whose outlines are borders for Smart Fill and Trim Segment.
fn is_border(k: &ObjectKind) -> bool {
    matches!(
        k,
        ObjectKind::Running { .. }
            | ObjectKind::Vector { .. }
            | ObjectKind::Fill { .. }
            | ObjectKind::Satin { .. }
            | ObjectKind::Applique { .. }
    )
}

/// The kinds that Trim Segment cuts.
fn is_line(k: &ObjectKind) -> bool {
    matches!(k, ObjectKind::Running { .. } | ObjectKind::Vector { .. })
}

fn flat(p: &Path) -> Vec<Polyline> {
    p.flatten(0.05)
}

impl AppState {
    /// The outlines of the shown objects, but not of `skip`.
    fn border_lines(&self, skip: Option<ObjectId>) -> Vec<Polyline> {
        self.doc
            .design()
            .objects()
            .iter()
            .filter(|o| Some(o.id) != skip && self.view_filter.shows(o) && is_border(&o.kind))
            .flat_map(|o| o.kind.paths().flat_map(flat).collect::<Vec<_>>())
            .collect()
    }

    /// Smart Fill: a fill of the area round `p`.
    pub fn smart_fill(&mut self, p: Point) {
        let lines = self.border_lines(None);
        let Some(r) = enclosed_area(&lines, p, SMART_FILL_HALF_MM) else {
            self.message = tr(
                "Click inside an area that lines close all round. Gaps of 0.3 mm or less close; \
                 Close Gaps closes larger ones.",
            )
            .into();
            return;
        };
        let s = Simplify {
            tolerance_mm: 0.05,
            corner_deg: 40.0,
            curves: true,
        };
        let outer = simplified(&Path::polygon(&r.outer), &s);
        let holes = r.holes.iter().map(|h| simplified(&Path::polygon(h), &s)).collect();
        let params = self.doc.design().current.fill.clone();
        self.add_drawn(ObjectKind::Fill { outer, holes, params });
    }

    /// Trim Segment: the line under `p` (within `reach` mm) loses its part
    /// between the crossings on each side of `p`. One undo step.
    pub fn trim_segment(&mut self, p: Point, reach: f64) {
        let d = self.doc.design();
        // The nearest subpath of a line: object, path, subpath, distance.
        let mut best: Option<(ObjectId, usize, usize, f64)> = None;
        for o in d
            .objects()
            .iter()
            .filter(|o| self.view_filter.shows(o) && is_line(&o.kind))
        {
            for (k, path) in o.kind.paths().enumerate() {
                for (si, sub) in NodePath::from_path(path).subs.into_iter().enumerate() {
                    let dist = flat(&NodePath { subs: vec![sub] }.to_path())
                        .iter()
                        .flat_map(Polyline::segments)
                        .map(|(a, b)| p.distance_to_segment(a, b))
                        .fold(f64::INFINITY, f64::min);
                    if dist <= reach && best.is_none_or(|b| dist < b.3) {
                        best = Some((o.id, k, si, dist));
                    }
                }
            }
        }
        let Some((id, k, si, _)) = best else {
            self.message = tr("Click on a line (running stitch or vector) where a part must go.").into();
            return;
        };
        let Some(obj) = d.object(id).cloned() else { return };
        if self.refuse_linked(&[id]) {
            return;
        }
        let Some(path) = obj.kind.paths().nth(k).cloned() else {
            return;
        };
        let np = NodePath::from_path(&path);
        // The other lines: other objects and the other subpaths of this one.
        let mut others = self.border_lines(Some(id));
        for (j, q) in obj.kind.paths().enumerate() {
            for (sj, s) in NodePath::from_path(q).subs.into_iter().enumerate() {
                if (j, sj) != (k, si) {
                    others.extend(flat(&NodePath { subs: vec![s] }.to_path()));
                }
            }
        }
        let Some(keep) = trim_piece(&np.subs[si], &others, p, reach) else {
            return;
        };
        let mut subs: Vec<SubNodes> = np.subs.clone();
        subs.splice(si..=si, keep);
        let gone = subs.is_empty() && obj.kind.paths().count() == 1;
        let new_path = NodePath { subs }.to_path();
        self.doc.edit(tr("Trim"), |dd| {
            if gone {
                dd.remove_object(id);
            } else if let Some(o) = dd.object_mut(id)
                && let Some(q) = o.kind.paths_mut().into_iter().nth(k)
            {
                *q = new_path;
            }
        });
        if gone {
            self.selection.clear();
        }
        self.message = tr("Trimmed. Click the next part, or press Esc.").into();
    }

    /// Smart Drawing: the shape that the stroke `pts` is near, as a running
    /// line, or else the `smoothed` stroke. The message names the shape.
    pub fn smart_drawing(&mut self, pts: &[Point], smoothed: Vec<(Point, bool)>) {
        let params = self.doc.design().current.running.clone();
        let (path, name) = match recognize(pts) {
            Some(Recognized::Line(a, b)) => (Path::new().move_to(a).line_to(b), Some(tr("a line"))),
            Some(Recognized::Rectangle(v)) => (Path::polygon(&v), Some(tr("a rectangle"))),
            Some(Recognized::Polygon(v)) => {
                let name = if v.len() == 3 {
                    tr("a triangle")
                } else {
                    tr("a four-sided shape")
                };
                (Path::polygon(&v), Some(name))
            }
            Some(Recognized::Ellipse {
                center,
                rx,
                ry,
                angle_deg,
            }) => {
                let t = Transform::rotate_deg(angle_deg).then(Transform::translate(center.x, center.y));
                let name = if rx == ry { tr("a circle") } else { tr("an ellipse") };
                (Path::ellipse(Point::ZERO, rx, ry).transformed(&t), Some(name))
            }
            None => (Path::through_points(&smoothed, false), None),
        };
        self.add_drawn(ObjectKind::Running { path, params });
        if let Some(name) = name {
            self.message = crate::i18n::fill("Smart Drawing made {}. Ctrl+Z: back to the stroke.", &[&name]);
        }
    }

    /// Copy Properties: waits for a click on the object to copy from.
    pub fn start_copy_properties(&mut self) {
        if self.selection.is_empty() {
            self.message = tr("Select the objects that get the properties, then use Copy Properties.").into();
            return;
        }
        let pick = crate::lettering::Pick::PropertiesFrom;
        self.message = pick.prompt().into();
        self.pick = Some(pick);
    }

    /// Gives the selected objects the color of `src`, and its stitch
    /// settings where the kind is the same. One undo step.
    pub fn copy_properties_from(&mut self, src: ObjectId) {
        let d = self.doc.design();
        let Some(from) = d.object(src).cloned() else { return };
        let targets: Vec<DesignObject> = self
            .sel()
            .into_iter()
            .filter(|id| *id != src)
            .filter_map(|id| d.object(id).cloned())
            .collect();
        if targets.is_empty() {
            self.message = tr("Click another object than the selected ones.").into();
            return;
        }
        let mut changes: Vec<(ObjectId, Option<ObjectKind>)> = Vec::new();
        for o in &targets {
            let mut kind = copied(&from.kind, &o.kind);
            if let Some(ObjectKind::Lettering(l)) = &mut kind
                && !crate::lettering::relayout(self, l)
            {
                kind = None;
            }
            changes.push((o.id, kind));
        }
        let settings = changes.iter().filter(|(_, k)| k.is_some()).count();
        let thread = from.thread;
        self.doc.edit(tr("Copy Properties"), |dd| {
            for (id, kind) in changes {
                if let Some(o) = dd.object_mut(id) {
                    o.thread = thread;
                    if let Some(k) = kind {
                        o.kind = k;
                    }
                }
            }
        });
        self.message = crate::i18n::fill(
            "Copied: the color to {} objects, the stitch settings to {} of them.",
            &[&targets.len(), &settings],
        );
    }
}

/// `dst` with the stitch settings of `src`, or `None` if the kinds differ.
fn copied(src: &ObjectKind, dst: &ObjectKind) -> Option<ObjectKind> {
    let mut k = dst.clone();
    match (src, &mut k) {
        (ObjectKind::Running { params: a, .. }, ObjectKind::Running { params: b, .. }) => *b = a.clone(),
        (ObjectKind::Fill { params: a, .. }, ObjectKind::Fill { params: b, .. }) => *b = a.clone(),
        (ObjectKind::Satin { params: a, .. }, ObjectKind::Satin { params: b, .. }) => *b = a.clone(),
        (ObjectKind::Applique { params: a, .. }, ObjectKind::Applique { params: b, .. }) => *b = a.clone(),
        (ObjectKind::Vector { style: a, .. }, ObjectKind::Vector { style: b, .. }) => *b = *a,
        (ObjectKind::Lettering(a), ObjectKind::Lettering(b)) => {
            // The look of the letters; the text, the baseline, the envelope
            // and the place stay.
            b.font = a.font.clone();
            b.height_mm = a.height_mm;
            b.letter_spacing_mm = a.letter_spacing_mm;
            b.line_spacing = a.line_spacing;
            b.justify = a.justify;
            b.order = a.order;
            b.connection = a.connection;
            b.trim_over_mm = a.trim_over_mm;
            b.kerning = a.kerning;
            b.stitch = a.stitch;
            b.satin = a.satin.clone();
            b.fill = a.fill.clone();
            b.mitre_corners = a.mitre_corners;
        }
        _ => return None,
    }
    Some(k)
}

/// Canvas input of the click tools Smart Fill and Trim Segment.
pub fn tool_input(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if !response.clicked() {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let p = map.to_design(pos);
    match st.tool {
        crate::draw::Tool::SmartFill => st.smart_fill(p),
        crate::draw::Tool::TrimSegment => st.trim_segment(p, PICK_PX / st.view.scale()),
        _ => {}
    }
}

/// The Envelope window.
#[derive(Debug, Clone)]
pub struct EnvelopeDialog {
    pub envelope: Envelope,
    originals: Vec<DesignObject>,
    bbox: BBox,
    key: MergeKey,
    applied: Option<Envelope>,
}

/// The kinds that the envelope bends.
fn bends(k: &ObjectKind) -> bool {
    matches!(
        k,
        ObjectKind::Running { .. }
            | ObjectKind::Fill { .. }
            | ObjectKind::Satin { .. }
            | ObjectKind::Applique { .. }
            | ObjectKind::Vector { .. }
    )
}

/// `kind` bent by `e` in box `b`, with fewer nodes again.
pub fn bent(kind: &ObjectKind, e: &Envelope, b: &BBox) -> ObjectKind {
    let mut k = kind.clone();
    if e.is_none() || !bends(kind) {
        return k;
    }
    e.warp_kind(b, &mut k);
    let s = Simplify {
        tolerance_mm: 0.03,
        corner_deg: 30.0,
        curves: true,
    };
    for p in k.paths_mut() {
        *p = simplified(p, &s);
    }
    k
}

impl AppState {
    pub fn open_envelope(&mut self) {
        let d = self.doc.design();
        let originals: Vec<DesignObject> = self
            .sel()
            .into_iter()
            .filter_map(|id| d.object(id))
            .filter(|o| bends(&o.kind))
            .cloned()
            .collect();
        let ids: Vec<ObjectId> = originals.iter().map(|o| o.id).collect();
        let Some(bbox) = d.bbox_of(&ids) else {
            self.message = tr("Select lines, fills, satin columns, appliqué or vector objects.").into();
            return;
        };
        if self.refuse_linked(&ids) {
            return;
        }
        self.drag_counter += 1;
        self.envelope_dialog = Some(EnvelopeDialog {
            envelope: Envelope {
                shape: EnvelopeShape::ArchOver,
                ..Envelope::default()
            },
            originals,
            bbox,
            key: MergeKey::new(format!("envelope-{}", self.drag_counter)),
            applied: None,
        });
    }

    /// Shows the result of the window on the canvas.
    pub fn preview_envelope(&mut self, d: &mut EnvelopeDialog) {
        if d.applied == Some(d.envelope) {
            return;
        }
        let (originals, e, b) = (d.originals.clone(), d.envelope, d.bbox);
        self.doc.edit_merged(tr("Envelope"), d.key.clone(), |dd| {
            for o in &originals {
                if let Some(obj) = dd.object_mut(o.id) {
                    obj.kind = bent(&o.kind, &e, &b);
                }
            }
        });
        d.applied = Some(d.envelope);
    }

    /// OK (`keep`) or Cancel.
    pub fn close_envelope(&mut self, mut d: EnvelopeDialog, keep: bool) {
        if keep {
            self.preview_envelope(&mut d);
            self.doc.end_merge();
        } else if d.applied.is_some() && self.doc.is_merging(&d.key) {
            self.doc.end_merge();
            self.undo();
        }
    }
}

/// The Envelope window.
pub fn envelope_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.envelope_dialog.take() else {
        return;
    };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Envelope")).id(egui::Id::new("Envelope")),
        ctx,
        egui::Id::new("Envelope"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(280.0)
    .show(ctx, |ui| {
        ui.weak(tr("Bend the selected objects into a shape."));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for shape in EnvelopeShape::ALL.into_iter().skip(1) {
                let on = d.envelope.shape == shape;
                if crate::lettering::envelope_icon(ui, shape, on).clicked() {
                    d.envelope.shape = shape;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Amount"))
                .on_hover_text(tr("How far the edges move, as a part of the height of the selection."));
            let mut pct = d.envelope.amount * 100.0;
            if ui
                .add(
                    egui::Slider::new(&mut pct, 0.0..=emb_model::envelope::MAX_AMOUNT * 100.0)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix(" %")
                        .fixed_decimals(0),
                )
                .changed()
            {
                d.envelope.amount = pct / 100.0;
            }
        });
        ui.weak(tr("Lettering has its own envelope: Object Properties > Lettering art."));
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button(tr("OK")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
        });
    });
    if cancel || !open {
        st.close_envelope(d, false);
    } else if ok {
        st.close_envelope(d, true);
    } else {
        st.preview_envelope(&mut d);
        st.envelope_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{FillParams, RunningParams, SatinParams};

    use super::*;
    use crate::test_util::Harness;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(pts: &[Point]) -> ObjectKind {
        ObjectKind::Running {
            path: crate::actions::path_of(pts, false),
            params: RunningParams::default(),
        }
    }

    fn add(h: &mut Harness, kinds: Vec<ObjectKind>) -> Vec<ObjectId> {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            kinds.into_iter().map(|k| d.add_object(t, k)).collect()
        })
    }

    /// Four strokes that cross at the corners of a 20 mm square.
    fn hash(h: &mut Harness) -> Vec<ObjectId> {
        add(
            h,
            vec![
                line(&[p(-3.0, 0.0), p(23.0, 0.0)]),
                line(&[p(20.0, -3.0), p(20.0, 23.0)]),
                line(&[p(23.0, 20.0), p(-3.0, 20.0)]),
                line(&[p(0.0, 23.0), p(0.0, -3.0)]),
            ],
        )
    }

    #[test]
    fn smart_fill_fills_the_area_inside_crossing_strokes() {
        let mut h = Harness::empty();
        hash(&mut h);
        h.st.smart_fill(p(10.0, 10.0));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 5);
        let ObjectKind::Fill { outer, holes, .. } = &d.objects()[4].kind else {
            panic!()
        };
        let r = crate::actions::fill_regions(outer, holes);
        assert!((r[0].area() - 400.0).abs() < 2.0, "{}", r[0].area());
        // Outside: nothing.
        h.st.smart_fill(p(40.0, 10.0));
        assert_eq!(h.st.doc.design().objects().len(), 5);
        assert!(h.st.message.contains("close all round"));
    }

    #[test]
    fn trim_segment_removes_the_overshoots() {
        let mut h = Harness::empty();
        let ids = hash(&mut h);
        // The left end of the top stroke goes past the left stroke.
        h.st.trim_segment(p(-2.0, 0.0), 0.5);
        let ObjectKind::Running { path, .. } = &h.st.doc.design().object(ids[0]).unwrap().kind else {
            panic!()
        };
        let pts: Vec<Point> = path.points().collect();
        assert_eq!(pts, vec![p(0.0, 0.0), p(23.0, 0.0)]);
        assert_eq!(h.st.doc.undo_label(), Some("Trim"));
        // The middle of the top stroke: it splits in two parts.
        h.st.trim_segment(p(10.0, 0.0), 0.5);
        let ObjectKind::Running { path, .. } = &h.st.doc.design().object(ids[0]).unwrap().kind else {
            panic!()
        };
        assert_eq!(
            NodePath::from_path(path).subs.len(),
            1,
            "only the part past the right stroke"
        );
        // Nothing near: a message.
        h.st.trim_segment(p(10.0, 10.0), 0.5);
        assert!(h.st.message.starts_with("Click on a line"));
    }

    #[test]
    fn smart_drawing_makes_a_circle() {
        let mut h = Harness::empty();
        let pts: Vec<Point> = (0..=100)
            .map(|i| {
                let a = f64::from(i) / 100.0 * std::f64::consts::TAU;
                p(10.0 * a.cos() + 0.2 * (f64::from(i) * 1.3).sin(), 10.0 * a.sin())
            })
            .collect();
        let smoothed: Vec<(Point, bool)> = pts.iter().map(|q| (*q, false)).collect();
        h.st.smart_drawing(&pts, smoothed);
        let ObjectKind::Running { path, .. } = &h.st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(
            path.cmds
                .iter()
                .filter(|c| matches!(c, emb_geom::PathCmd::CubicTo(..)))
                .count(),
            4
        );
        assert!(h.st.message.contains("a circle"), "{}", h.st.message);
    }

    #[test]
    fn copy_properties_gives_settings_to_the_same_kind_and_the_color_to_all() {
        let mut h = Harness::empty();
        let fill = |density: f64| ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(0.0, 0.0), p(10.0, 10.0))),
            holes: vec![],
            params: FillParams {
                spacing_mm: density,
                ..FillParams::default()
            },
        };
        let ids = add(&mut h, vec![fill(0.4), fill(0.6), line(&[p(0.0, 20.0), p(10.0, 20.0)])]);
        let blue = h.st.doc.edit("Blue", |d| {
            let t = d.add_thread("Blue", emb_model::Rgb(0, 0, 200));
            d.object_mut(ids[0]).unwrap().thread = t;
            t
        });
        h.st.selection.set_all(vec![ids[1], ids[2]]);
        h.st.start_copy_properties();
        assert!(h.st.pick_clicked(Some(ids[0])));
        let d = h.st.doc.design();
        let ObjectKind::Fill { params, .. } = &d.object(ids[1]).unwrap().kind else {
            panic!()
        };
        assert_eq!(params.spacing_mm, 0.4);
        assert_eq!(d.object(ids[1]).unwrap().thread, blue);
        assert_eq!(d.object(ids[2]).unwrap().thread, blue, "the color goes to the line too");
        assert!(matches!(d.object(ids[2]).unwrap().kind, ObjectKind::Running { .. }));
        assert_eq!(h.st.doc.undo_label(), Some("Copy Properties"));
    }

    #[test]
    fn the_envelope_bends_a_column_and_cancel_goes_back() {
        let mut h = Harness::empty();
        let col = ObjectKind::Satin {
            geometry: emb_model::SatinGeometry::from_centerline(
                &crate::actions::path_of(&[p(0.0, 0.0), p(40.0, 0.0)], false),
                4.0,
            ),
            params: SatinParams::default(),
        };
        let ids = add(&mut h, vec![col.clone()]);
        h.st.selection.set(ids[0]);
        h.st.open_envelope();
        let mut d = h.st.envelope_dialog.take().unwrap();
        h.st.preview_envelope(&mut d);
        let bb = h.st.doc.design().bbox_of(&ids).unwrap();
        assert!(bb.height() > 5.0, "the middle rises: {bb:?}");
        h.st.close_envelope(d, false);
        assert_eq!(h.st.doc.design().object(ids[0]).unwrap().kind, col);
    }
}
