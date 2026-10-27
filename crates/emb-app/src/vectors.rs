//! Vector objects in the app (ADR 0024, P6 Milestone 1): drawing them on
//! the canvas, their Object Properties, Convert (vectors to embroidery and
//! back), import of SVG paths as vectors, and the export of the object
//! outlines as SVG.

use eframe::egui::{self, Pos2, Rect, Shape, Stroke};
use emb_geom::Path;
use emb_model::vector::{from_vector, to_vector};
use emb_model::{ConvertTo, ObjectId, ObjectKind, Rgb, VectorStyle};

use crate::app::AppState;
use crate::canvas::color32;
use crate::file::Notice;
use crate::i18n::tr;

/// How strong dimmed vectors show (View > Dim vectors).
const DIM: f32 = 0.35;

/// Draws the vector objects under the stitches: the fill (even-odd) and
/// the stroke at its real width.
pub fn draw(painter: &egui::Painter, rect: Rect, st: &AppState) {
    if !st.show_vectors {
        return;
    }
    let d = st.doc.design();
    let rot = d.rotation_transform();
    let view = &st.view;
    let alpha = if st.dim_vectors { DIM } else { 1.0 };
    for o in d.objects() {
        let ObjectKind::Vector { path, style } = &o.kind else {
            continue;
        };
        if !st.view_filter.shows(o) {
            continue;
        }
        let lines = path.flatten(view.tolerance());
        let screen = |pts: &[emb_geom::Point]| -> Vec<Pos2> {
            pts.iter().map(|p| view.to_screen(rect, rot.apply(*p))).collect()
        };
        if let Some(fill) = style.fill {
            // Open subpaths fill as if closed (as in SVG).
            let rings: Vec<Vec<Pos2>> = lines.iter().map(|pl| screen(&pl.points)).collect();
            painter.add(Shape::mesh(crate::applique::scanline_mesh(
                &rings,
                color32(fill).gamma_multiply(alpha),
                rect,
            )));
        }
        if let Some(stroke) = style.stroke {
            let width = (style.stroke_mm * view.scale()) as f32;
            let s = Stroke::new(width.max(1.0), color32(stroke).gamma_multiply(alpha));
            for pl in &lines {
                let pts = screen(&pl.points);
                painter.add(if pl.closed {
                    Shape::closed_line(pts, s)
                } else {
                    Shape::line(pts, s)
                });
            }
        }
    }
}

/// Object Properties of a vector object.
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, path: &Path, style: &VectorStyle) {
    ui.weak(tr(
        "Artwork: it sews nothing, and machine files leave it out. Convert makes embroidery from it.",
    ));
    let n = path.subpaths().len();
    ui.label(crate::i18n::fill("{} path{}", &[&n, &(if n == 1 { "" } else { "s" })]));
    let mut s = *style;
    ui.horizontal(|ui| {
        let mut on = s.fill.is_some();
        if ui.checkbox(&mut on, tr("Fill")).changed() {
            s.fill = on.then(|| style.stroke.unwrap_or(Rgb(128, 128, 128)));
        }
        if let Some(c) = s.fill.as_mut() {
            color_button(ui, c);
        }
    });
    ui.horizontal(|ui| {
        let mut on = s.stroke.is_some();
        if ui.checkbox(&mut on, tr("Line")).changed() {
            s.stroke = on.then(|| style.fill.unwrap_or(Rgb(0, 0, 0)));
        }
        if let Some(c) = s.stroke.as_mut() {
            color_button(ui, c);
            ui.add(
                egui::DragValue::new(&mut s.stroke_mm)
                    .range(0.05..=10.0)
                    .clamp_existing_to_range(false)
                    .speed(0.02)
                    .suffix(" mm"),
            )
            .on_hover_text(tr("The width of the line."));
        }
    });
    if s != *style {
        st.doc.edit_merged(
            "Change artwork",
            emb_model::MergeKey::new(format!("{}:vector", id.0)),
            |d| {
                if let Some(ObjectKind::Vector { style, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    *style = s;
                }
            },
        );
    }
    ui.separator();
    ui.label(tr("Convert to embroidery"));
    ui.horizontal_wrapped(|ui| {
        for to in ConvertTo::ALL {
            if ui.button(tr(to.name())).on_hover_text(convert_help(to)).clicked() {
                st.convert_vectors(to);
            }
        }
    });
}

/// A color swatch that opens a color picker.
fn color_button(ui: &mut egui::Ui, c: &mut Rgb) {
    let mut rgb = [c.0, c.1, c.2];
    if ui.color_edit_button_srgb(&mut rgb).changed() {
        *c = Rgb(rgb[0], rgb[1], rgb[2]);
    }
}

pub fn convert_help(to: ConvertTo) -> &'static str {
    match to {
        ConvertTo::Fill => tr("Each closed shape becomes a fill, with its holes."),
        ConvertTo::Satin => tr(
            "Each line becomes a satin column along it; each closed shape becomes a turning column along its length.",
        ),
        ConvertTo::Outline => tr("All paths become one running-stitch line."),
    }
}

/// How an SVG file comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkImport {
    /// Fills and lines.
    Objects,
    /// Every shape as a running-stitch line.
    Lines,
    /// Vector objects with no stitches.
    Vectors,
}

impl AppState {
    /// Makes embroidery objects from the selected vectors, in their place
    /// in the sew order. The new objects take the thread nearest to the
    /// vector color. One undo step.
    pub fn convert_vectors(&mut self, to: ConvertTo) {
        let design = self.doc.design();
        let current = design.current.clone();
        let width = self.settings.defaults.column_width_mm;
        let work: Vec<(ObjectId, Vec<ObjectKind>, Option<Rgb>)> = self
            .sel()
            .into_iter()
            .filter_map(|id| design.object(id))
            .filter_map(|o| match &o.kind {
                ObjectKind::Vector { path, style } => {
                    Some((o.id, from_vector(path, to, &current, width), to.color(style)))
                }
                _ => None,
            })
            .filter(|(_, kinds, _)| !kinds.is_empty())
            .collect();
        if work.is_empty() {
            self.message = match to {
                ConvertTo::Fill => "Select vectors with closed shapes first.".into(),
                _ => "Select vectors first.".into(),
            };
            return;
        }
        // A linked vector changes in place, so its link stays (one object
        // for one; see `Design::convert_linked`).
        let one = move |k: &ObjectKind| match k {
            ObjectKind::Vector { path, .. } => {
                let mut v = from_vector(path, to, &current, width);
                (v.len() == 1).then(|| v.remove(0))
            }
            _ => None,
        };
        let linked: Vec<ObjectId> = work.iter().map(|w| w.0).filter(|id| design.is_linked(*id)).collect();
        let mut probe = design.clone();
        for id in &linked {
            if probe.convert_linked(*id, None, &one).is_err() {
                self.message = crate::i18n::fill(
                    "A linked vector cannot become more than one {}: unlink it first (Artwork > Shaping > Unlink).",
                    &[&(to.name().to_lowercase())],
                );
                return;
            }
        }
        let new = self.doc.edit(format!("Convert to {}", to.name().to_lowercase()), |d| {
            let mut new = Vec::new();
            for (id, kinds, color) in work {
                let thread = color.map(|c| nearest_thread(d, c));
                if linked.contains(&id) {
                    let _ = d.convert_linked(id, thread, &one);
                    new.push(id);
                    continue;
                }
                let ids = crate::actions::replace_with(d, id, kinds);
                for nid in &ids {
                    if let (Some(t), Some(o)) = (thread, d.object_mut(*nid)) {
                        o.thread = t;
                    }
                }
                new.extend(ids);
            }
            new
        });
        self.message = crate::i18n::fill("{} embroidery objects made.", &[&(new.len())]);
        self.selection.set_all(new);
    }

    /// Recognize outlines: each selected stitch object becomes a fill of
    /// the area that its stitches cover (`emb_art::recognize`), in its
    /// thread, with the current fill settings. The stitch engine then makes
    /// it again at any size with its density. One undo step.
    pub fn recognize_outlines(&mut self) {
        let ids: Vec<ObjectId> = self.sel();
        if self.refuse_linked(&ids) {
            return;
        }
        let design = self.doc.design();
        let fill = design.current.fill.clone();
        let work: Vec<(ObjectId, ObjectKind)> = ids
            .iter()
            .filter_map(|id| design.object(*id))
            .filter_map(|o| {
                let ObjectKind::Stitches { path, .. } = &o.kind else {
                    return None;
                };
                let stitches: Vec<(emb_geom::Point, emb_geom::Point)> = path
                    .subpaths()
                    .iter()
                    .flat_map(|s| {
                        let pts: Vec<emb_geom::Point> = s.points().collect();
                        pts.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
                    })
                    .collect();
                let areas = emb_art::recognize::outlines(&stitches);
                if areas.is_empty() {
                    return None;
                }
                let outers: Vec<emb_geom::Path> = areas.iter().map(|a| a.outer.clone()).collect();
                Some((
                    o.id,
                    ObjectKind::Fill {
                        outer: emb_geom::Path::joined(&outers),
                        holes: areas.iter().flat_map(|a| a.holes.clone()).collect(),
                        params: fill.clone(),
                    },
                ))
            })
            .collect();
        if work.is_empty() {
            self.message = tr("Select stitch objects (from a machine file or a photo stitch) first.").into();
            return;
        }
        let new = self.doc.edit("Recognize Outlines", |d| {
            work.into_iter()
                .flat_map(|(id, k)| crate::actions::replace_with(d, id, vec![k]))
                .collect::<Vec<_>>()
        });
        self.message = crate::i18n::fill("{} stitch objects became fills.", &[&(new.len())]);
        self.selection.set_all(new);
    }

    /// Changes the selected embroidery objects into vectors (their outlines
    /// in the thread color). Linked objects change in place and stay
    /// linked. One undo step.
    pub fn convert_to_vectors(&mut self) {
        let design = self.doc.design();
        let work: Vec<(ObjectId, ObjectKind, Rgb)> = self
            .sel()
            .into_iter()
            .filter_map(|id| design.object(id))
            .filter_map(|o| {
                let color = design.thread(o.thread).map_or(Rgb(0, 0, 0), |t| t.color);
                to_vector(&o.kind, color).map(|k| (o.id, k, color))
            })
            .collect();
        if work.is_empty() {
            self.message = tr("Select embroidery objects first (not stitches from a machine file).").into();
            return;
        }
        let linked: Vec<ObjectId> = work.iter().map(|w| w.0).filter(|id| design.is_linked(*id)).collect();
        let mut probe = design.clone();
        for (id, _, color) in work.iter().filter(|w| linked.contains(&w.0)) {
            let c = *color;
            if probe
                .convert_linked(*id, None, &|k: &ObjectKind| to_vector(k, c))
                .is_err()
            {
                self.message =
                    tr("The border of a border trace cannot change: unlink it first (Artwork > Shaping > Unlink).")
                        .into();
                return;
            }
        }
        let new = self.doc.edit("Convert to vector", |d| {
            let mut new = Vec::new();
            for (id, k, color) in work {
                if linked.contains(&id) {
                    let _ = d.convert_linked(id, None, &|k: &ObjectKind| to_vector(k, color));
                    new.push(id);
                } else {
                    new.extend(crate::actions::replace_with(d, id, vec![k]));
                }
            }
            new
        });
        self.selection.set_all(new);
    }

    /// Runs a shaping command on the selection (one undo step) and selects
    /// the result. Nothing changes if the command cannot run.
    pub fn shape_selection(&mut self, op: emb_model::ShapeOp) {
        let ids = self.sel();
        let opts = self.shaping;
        // Try on a copy first, so a failed command makes no undo step.
        let mut probe = self.doc.design().clone();
        if let Err(e) = probe.shape(&ids, op, opts) {
            self.message = e.to_string();
            return;
        }
        let new = self
            .doc
            .edit(op.name(), |d| d.shape(&ids, op, opts).unwrap_or_default());
        self.message = crate::i18n::fill("{}: {} objects.", &[&tr(op.name()), &(new.len())]);
        self.selection.set_all(new);
    }

    /// Asks for a file name and writes the object outlines as SVG.
    pub fn export_outlines_dialog(&mut self) {
        let Some(svg) = emb_render::svg::outlines(self.doc.design()) else {
            self.message = tr("The design has no shapes to write.").into();
            return;
        };
        let name = format!("{} outlines.svg", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Export outlines (SVG or EMF)")
            .add_filter("SVG vector file", &["svg"])
            .add_filter("EMF Windows metafile", &["emf"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        let emf = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("emf"));
        let bytes = if emf {
            outlines_emf(self.doc.design())
        } else {
            svg.into_bytes()
        };
        match std::fs::write(&p, bytes) {
            Ok(()) => self.message = crate::i18n::fill("Outlines written to {}", &[&(p.display())]),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot write file".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                })
            }
        }
    }
}

/// The outlines of the objects as an EMF file (P10 Milestone 6): the same
/// shapes as the SVG outlines.
pub fn outlines_emf(design: &emb_model::Design) -> Vec<u8> {
    let shapes: Vec<emb_formats::emf::EmfShape> = emb_render::svg::outline_shapes(design)
        .into_iter()
        .map(|(path, style, _)| emb_formats::emf::EmfShape {
            path,
            fill: style.fill,
            stroke: style.stroke,
            stroke_mm: style.stroke_mm,
        })
        .collect();
    emb_formats::emf::write_emf(&shapes)
}

/// The thread of the design with the color nearest to `c`; a new thread
/// if none is near.
pub(crate) fn nearest_thread(d: &mut emb_model::Design, c: Rgb) -> emb_model::ThreadId {
    const NEAR: f64 = 12.0;
    let lab = emb_model::Lab::of(c);
    let best = d
        .threads
        .iter()
        .map(|t| (t.id, emb_model::Lab::of(t.color).distance(lab)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match best {
        Some((id, dist)) if dist <= NEAR => id,
        _ => d.thread_for_color(c),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn outlines_as_emf_read_back() {
        let d = emb_cli::corpus::build("two-color-patch").unwrap();
        let n = emb_render::svg::outline_shapes(&d).len();
        assert!(n > 0);
        let back = emb_formats::emf::import_emf(&super::outlines_emf(&d), true).unwrap();
        assert_eq!(back.objects.len(), n, "{:?}", back.warnings);
    }

    use eframe::egui::{Modifiers, PointerButton};
    use emb_geom::Point;

    use super::*;
    use crate::draw::Tool;
    use crate::test_util::Harness;

    const LOGO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="60mm" height="40mm" viewBox="0 0 60 40">
        <path id="ring" fill="#c00000" fill-rule="evenodd" d="M0,0 H40 V40 H0 Z M10,10 H30 V30 H10 Z"/>
        <path id="line" fill="none" stroke="#0000c0" stroke-width="1" d="M45,0 L60,40"/></svg>"##;

    fn vectors(h: &Harness) -> Vec<ObjectId> {
        h.st.doc
            .design()
            .objects()
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Vector { .. }))
            .map(|o| o.id)
            .collect()
    }

    #[test]
    fn import_as_vectors_then_convert_and_undo() {
        let mut h = Harness::empty();
        h.st.import_svg_as(LOGO, "logo.svg", ArtworkImport::Vectors);
        let ids = vectors(&h);
        assert_eq!(ids.len(), 2);
        assert!(h.st.doc.design().sewn_color_blocks().is_empty(), "artwork sews nothing");
        let start = h.st.doc.design().clone();
        // The ring to a fill with its hole, in the red thread.
        h.st.selection.set(ids[0]);
        h.st.convert_vectors(ConvertTo::Fill);
        let d = h.st.doc.design();
        let fill = d.object(h.st.selection.ids()[0]).unwrap();
        assert!(matches!(&fill.kind, ObjectKind::Fill { holes, .. } if holes.len() == 1));
        assert_eq!(d.thread(fill.thread).unwrap().color, Rgb(0xc0, 0, 0));
        assert_eq!(d.index_of(fill.id), Some(0), "in the place of the vector");
        // The line to a satin column, in the blue thread.
        h.st.selection.set(ids[1]);
        h.st.convert_vectors(ConvertTo::Satin);
        let d = h.st.doc.design();
        let col = d.object(h.st.selection.ids()[0]).unwrap();
        assert!(matches!(col.kind, ObjectKind::Satin { .. }));
        assert_eq!(d.thread(col.thread).unwrap().color, Rgb(0, 0, 0xc0));
        assert!(vectors(&h).is_empty());
        // And back: the fill becomes a vector again.
        h.st.selection.set(d.objects()[0].id);
        h.st.convert_to_vectors();
        assert_eq!(vectors(&h).len(), 1);
        for _ in 0..3 {
            h.st.undo();
        }
        assert!(h.st.doc.design().same_content(&start));
    }

    #[test]
    fn draw_artwork_makes_vectors() {
        let mut h = Harness::empty();
        h.st.draw_vectors = true;
        h.st.set_tool(Tool::Rectangle);
        let (a, b) = (h.screen(Point::new(-10.0, -10.0)), h.screen(Point::new(10.0, 10.0)));
        h.drag(PointerButton::Primary, a, b, Modifiers::NONE);
        let ids = vectors(&h);
        assert_eq!(ids.len(), 1);
        let d = h.st.doc.design();
        let Some(ObjectKind::Vector { style, .. }) = d.object(ids[0]).map(|o| &o.kind) else {
            panic!("not a vector")
        };
        assert!(style.fill.is_some());
        // Convert works on the shape that was drawn.
        h.st.convert_vectors(ConvertTo::Outline);
        assert!(matches!(
            h.st.doc.design().objects()[0].kind,
            ObjectKind::Running { .. }
        ));
    }

    #[test]
    fn shaping_is_one_undo_step_and_a_failure_makes_none() {
        let mut h = Harness::empty();
        let a = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            let sq = |x: f64| ObjectKind::Fill {
                outer: Path::rect(emb_geom::BBox::new(Point::new(x, 0.0), Point::new(x + 20.0, 20.0))),
                holes: Vec::new(),
                params: Default::default(),
            };
            vec![
                d.add_object(t, sq(0.0)),
                d.add_object(t, sq(10.0)),
                d.add_object(t, sq(100.0)),
            ]
        });
        let start = h.st.doc.design().clone();
        let steps = h.st.doc.undo_len();
        // The far square does not touch the others: nothing is left.
        h.st.selection.set_all([a[0], a[2]]);
        h.st.shape_selection(emb_model::ShapeOp::Intersect);
        assert_eq!(h.st.doc.undo_len(), steps);
        assert!(h.st.message.contains("Nothing is left"), "{}", h.st.message);
        h.st.selection.set_all([a[0], a[1]]);
        h.st.shape_selection(emb_model::ShapeOp::Weld);
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert_eq!(h.st.doc.design().objects().len(), 2);
        assert_eq!(h.st.selection.len(), 1);
        h.st.undo();
        assert!(h.st.doc.design().same_content(&start));
    }

    #[test]
    fn convert_needs_vectors() {
        let mut h = Harness::new();
        h.st.select_all();
        let n = h.st.doc.undo_len();
        h.st.convert_vectors(ConvertTo::Fill);
        assert_eq!(h.st.doc.undo_len(), n);
        assert!(!h.st.message.is_empty());
    }
}

#[cfg(test)]
mod timing {
    use eframe::egui::{Event, Modifiers, PointerButton};
    use emb_geom::Point;

    use super::*;
    use crate::test_util::Harness;

    /// An SVG like a logo: a large curved shape behind, 36 circles in front.
    fn logo() -> Vec<u8> {
        let mut circles = String::new();
        for i in 0..6 {
            for j in 0..6 {
                let (cx, cy) = (20.0 + 12.0 * f64::from(i), 20.0 + 12.0 * f64::from(j));
                circles.push_str(&format!("M{},{} a5,5 0 1,0 10,0 a5,5 0 1,0 -10,0 Z ", cx - 5.0, cy));
            }
        }
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
            <path fill="#2040a0" d="M5,50 C5,10 95,10 95,50 C95,90 5,90 5,50 Z"/>
            <path fill="#e0a020" d="{circles}"/></svg>"##
        )
        .into_bytes()
    }

    #[test]
    #[ignore]
    fn linked_drag_timing() {
        let mut h = Harness::empty();
        h.st.import_svg_as(&logo(), "logo.svg", ArtworkImport::Vectors);
        h.st.select_all();
        h.st.convert_vectors(ConvertTo::Fill);
        h.st.select_all();
        h.st.shape_selection(emb_model::ShapeOp::Flatten);
        let top = h.st.doc.design().objects()[1].id;
        h.st.selection.set(top);
        h.st.show_plan = true;
        let c = h.st.doc.design().bbox_of(&[top]).unwrap().center();
        let from = h.screen(c);
        h.frame(vec![Event::PointerMoved(from)]);
        h.frame(vec![Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        for i in 1..=5 {
            let t0 = std::time::Instant::now();
            h.frame(vec![Event::PointerMoved(
                from + eframe::egui::vec2(2.0 * i as f32, 0.0),
            )]);
            let frame = t0.elapsed();
            // The parts of the frame: the cut again, and the stitch plan.
            let d = h.st.doc.design().clone();
            let t1 = std::time::Instant::now();
            let mut d2 = d.clone();
            d2.linked_objects_moved(&[top], &emb_geom::Transform::translate(0.1, 0.0));
            let cut = t1.elapsed();
            let t2 = std::time::Instant::now();
            let _ = emb_stitch::build_plan(&d2, &Default::default(), &mut Default::default());
            let plan = t2.elapsed();
            println!("frame {i}: {frame:?} (cut {cut:?}, plan from scratch {plan:?})");
            if i == 5 {
                for o in d2.objects() {
                    let t3 = std::time::Instant::now();
                    let st = emb_stitch::stitch_object(o, Point::ZERO);
                    println!(
                        "  object {:?} {}: {:?}, {} runs",
                        o.id,
                        o.type_name(),
                        t3.elapsed(),
                        st.map_or(0, |s| s.runs.len())
                    );
                }
            }
        }
        let _ = Point::ZERO;
    }
}
