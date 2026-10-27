//! Appliqué in the app: the properties section, Convert to appliqué,
//! Combine appliqué and the fabric display. See `docs/design/p3-plan.md`
//! (Milestone 7) and `emb_model::applique`.

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Shape, Stroke};
use emb_geom::{Point, Transform};
use emb_model::{AppliqueParams, AppliquePiece, MergeKey, ObjectId, ObjectKind, Rgb, TackDown, outline_edges};

use crate::app::AppState;
use crate::canvas::{CanvasView, color32};
use crate::i18n::tr;
use crate::words;

use emb_model::DEFAULT_FABRIC_COLOR as DEFAULT_FABRIC;

impl AppState {
    /// Changes the selected fills and closed vectors into appliqué objects
    /// (the outline and the holes stay), with the current appliqué
    /// properties. A linked object stays linked (`Design::convert_linked`).
    pub fn convert_to_applique(&mut self) {
        let d = self.doc.design();
        let params = d.current.applique.clone();
        let current = d.current.clone();
        let to_applique = move |k: &ObjectKind| -> Option<ObjectKind> {
            let fill = match k {
                ObjectKind::Vector { path, .. } => {
                    emb_model::vector::from_vector(path, emb_model::ConvertTo::Fill, &current, 0.0)
                        .into_iter()
                        .next()?
                }
                other => other.clone(),
            };
            let ObjectKind::Fill { outer, holes, .. } = fill else {
                return None;
            };
            let pieces = outer
                .subpaths()
                .into_iter()
                .enumerate()
                .map(|(i, o)| AppliquePiece::new(o, if i == 0 { holes.clone() } else { Vec::new() }))
                .collect();
            Some(ObjectKind::Applique {
                pieces,
                params: params.clone(),
            })
        };
        let ids: Vec<ObjectId> = self
            .selection
            .in_sew_order(d)
            .into_iter()
            .filter(|id| d.object(*id).and_then(|o| to_applique(&o.kind)).is_some())
            .collect();
        if ids.is_empty() {
            self.message = tr("Select a fill or a closed vector to change into appliqué.").into();
            return;
        }
        let n = ids.len();
        self.doc.edit("Convert to appliqué", |d| {
            for id in ids {
                if d.is_linked(id) {
                    let _ = d.convert_linked(id, None, &to_applique);
                } else if let Some(o) = d.object_mut(id)
                    && let Some(k) = to_applique(&o.kind)
                {
                    o.kind = k;
                }
            }
        });
        self.message = crate::i18n::fill(
            "{} objects are appliqué now. Set the fabric color in Object Properties.",
            &[&n],
        );
    }

    /// Joins the selected appliqué objects into one, at the place of the
    /// first in sew order: all placement lines sew first, then all
    /// tack-downs, then all covers, with one stop for each step.
    pub fn combine_applique(&mut self) {
        let d = self.doc.design();
        let objs: Vec<_> = self
            .selection
            .in_sew_order(d)
            .into_iter()
            .filter_map(|id| d.object(id).cloned())
            .filter(|o| matches!(o.kind, ObjectKind::Applique { .. }))
            .collect();
        if objs.len() < 2 {
            self.message = tr("Select two or more appliqué objects to combine.").into();
            return;
        }
        let first = objs[0].id;
        let colors = objs.iter().any(|o| o.thread != objs[0].thread);
        let mut pieces = Vec::new();
        for o in &objs {
            if let ObjectKind::Applique { pieces: p, .. } = &o.kind {
                pieces.extend(p.iter().cloned());
            }
        }
        let others: Vec<ObjectId> = objs[1..].iter().map(|o| o.id).collect();
        let n = pieces.len();
        self.doc.edit("Combine Appliqué", |d| {
            if let Some(ObjectKind::Applique { pieces: p, .. }) = d.object_mut(first).map(|o| &mut o.kind) {
                *p = pieces;
            }
            for id in &others {
                d.remove_object(*id);
            }
        });
        self.selection.set(first);
        self.message = if colors {
            format!("{n} pieces in one appliqué object. It sews in the thread of the first object.")
        } else {
            format!("{n} pieces in one appliqué object: each step sews for all pieces, with one stop.")
        };
    }
}

/// A plain-language word for a value, and the help text of the setting.
pub(crate) type Word = (fn(f64) -> &'static str, &'static str);

/// The appliqué section of Object Properties. A drag of a slider is one undo step.
pub fn properties(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    pieces: &[AppliquePiece],
    params: &AppliqueParams,
) {
    let mut p = params.clone();
    let mut drag = false;
    let mut slider =
        |ui: &mut egui::Ui, label: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>, word: Option<Word>| {
            ui.horizontal(|ui| {
                let l = ui.label(label);
                if let Some((_, help)) = word {
                    l.on_hover_text(help);
                }
                let r = ui.add(
                    egui::Slider::new(v, range)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix(" mm")
                        .fixed_decimals(2),
                );
                if let Some((w, _)) = word {
                    ui.weak(format!("({})", w(*v)));
                }
                drag |= r.dragged();
            });
        };
    ui.strong(tr("Steps"));
    ui.checkbox(&mut p.placement, tr("1. Placement line, then stop"))
        .on_hover_text(tr(
            "A line that shows where to put the fabric. The machine stops after it.",
        ));
    ui.horizontal(|ui| {
        ui.label(tr("2. Tack-down"));
        egui::ComboBox::from_id_salt("tack_down")
            .selected_text(tack_name(p.tack_down))
            .show_ui(ui, |ui| {
                for t in [TackDown::Run, TackDown::Zigzag, TackDown::None] {
                    ui.selectable_value(&mut p.tack_down, t, tack_name(t));
                }
            })
            .response
            .on_hover_text(tr(
                "Stitches that hold the fabric. A zigzag holds fabric that frays better.",
            ));
    });
    if p.tack_down != TackDown::None {
        slider(ui, tr("Inside the outline"), &mut p.tack_inset_mm, 0.0..=3.0, None);
    }
    if p.tack_down == TackDown::Zigzag {
        slider(ui, tr("Zigzag width"), &mut p.tack_width_mm, 0.5..=3.0, None);
        slider(
            ui,
            tr("Zigzag spacing"),
            &mut p.tack_spacing_mm,
            0.5..=4.0,
            Some((words::density, tr(words::DENSITY_HELP))),
        );
    }
    ui.checkbox(&mut p.trim_stop, tr("Stop to cut the fabric"))
        .on_hover_text(tr(
            "The machine stops after the tack-down, so you can cut away the fabric outside it.",
        ));
    ui.checkbox(&mut p.cover, tr("3. Cover satin"))
        .on_hover_text(tr("A satin border over the cut edge of the fabric."));
    if p.cover {
        slider(ui, tr("Cover width"), &mut p.cover_width_mm, 1.0..=8.0, None);
        slider(ui, tr("Outside the outline"), &mut p.cover_outside_mm, 0.0..=3.0, None);
        slider(
            ui,
            tr("Density"),
            &mut p.cover_satin.spacing_mm,
            0.2..=1.0,
            Some((words::density, tr(words::DENSITY_HELP))),
        );
        slider(
            ui,
            tr("Pull compensation"),
            &mut p.cover_satin.pull_compensation_mm,
            0.0..=1.0,
            Some((words::pull, tr(words::PULL_HELP))),
        );
    }
    ui.horizontal(|ui| {
        ui.label(tr("Fabric Color"))
            .on_hover_text(tr("The color of the appliqué fabric, for the display."));
        let mut c = color32(p.fabric.unwrap_or(DEFAULT_FABRIC));
        if ui.color_edit_button_srgba(&mut c).changed() {
            p.fabric = Some(Rgb(c.r(), c.g(), c.b()));
        }
        if p.fabric.is_some() && ui.small_button(tr("None")).clicked() {
            p.fabric = None;
        }
    });

    let mut new_pieces = pieces.to_vec();
    // Layers of fabric and the color of each piece (P10 Milestone 6).
    if pieces.len() > 1 {
        ui.separator();
        ui.strong(tr("Layers")).on_hover_text(tr(
            "A piece on layer 2 or more goes on the fabric of the layers before it. Each layer sews its \
             placement line and tack-down in turn; the covers come last.",
        ));
        for (k, pc) in new_pieces.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(crate::i18n::fill("Piece {}", &[&(k + 1)]));
                let mut layer = pc.layer + 1;
                if ui
                    .add(
                        egui::DragValue::new(&mut layer)
                            .range(1..=emb_model::applique::MAX_LAYERS)
                            .clamp_existing_to_range(false)
                            .prefix("layer "),
                    )
                    .changed()
                {
                    pc.layer = layer.clamp(1, emb_model::applique::MAX_LAYERS) - 1;
                }
                let mut c = color32(pc.fabric_color(&p));
                if ui
                    .color_edit_button_srgba(&mut c)
                    .on_hover_text(tr("The fabric color of this piece"))
                    .changed()
                {
                    pc.fabric = Some(Rgb(c.r(), c.g(), c.b()));
                }
                if pc.fabric.is_some() && ui.small_button(tr("Object color")).clicked() {
                    pc.fabric = None;
                }
            });
        }
    }
    // Partial appliqué: edges with no cover.
    if p.cover {
        ui.separator();
        ui.strong(tr("Edges with cover")).on_hover_text(tr(
            "Clear an edge where another object covers the fabric edge (partial appliqué). \
             The edges are numbered from the first point of the outline.",
        ));
        for (k, pc) in new_pieces.iter_mut().enumerate() {
            let n = outline_edges(&pc.outer, 0.05).len();
            ui.horizontal_wrapped(|ui| {
                if pieces.len() > 1 {
                    ui.weak(crate::i18n::fill("Piece {}:", &[&(k + 1)]));
                }
                for e in 0..n {
                    let mut on = !pc.open_edges.contains(&e);
                    if ui.checkbox(&mut on, (e + 1).to_string()).changed() {
                        if on {
                            pc.open_edges.retain(|x| *x != e);
                        } else {
                            pc.open_edges.push(e);
                            pc.open_edges.sort_unstable();
                        }
                    }
                }
            });
        }
    }

    if p != *params || new_pieces != pieces {
        st.doc
            .edit_merged("Change appliqué", MergeKey::new(format!("{}:applique", id.0)), |d| {
                if let Some(ObjectKind::Applique { pieces, params }) = d.object_mut(id).map(|o| &mut o.kind) {
                    *params = p;
                    *pieces = new_pieces;
                }
            });
        if !drag {
            st.doc.end_merge();
        }
    }
}

fn tack_name(t: TackDown) -> &'static str {
    match t {
        TackDown::Run => tr("Running line"),
        TackDown::Zigzag => tr("Zigzag"),
        TackDown::None => tr("None"),
    }
}

/// Fills rings (even-odd) with `color` by horizontal screen lines, so any
/// shape with holes fills without a triangulation.
pub fn scanline_mesh(rings: &[Vec<Pos2>], color: Color32, clip: Rect) -> Mesh {
    let mut mesh = Mesh::default();
    let (mut top, mut bottom) = (f32::INFINITY, f32::NEG_INFINITY);
    for r in rings {
        for p in r {
            top = top.min(p.y);
            bottom = bottom.max(p.y);
        }
    }
    let (top, bottom) = (top.max(clip.top()), bottom.min(clip.bottom()));
    if top >= bottom {
        return mesh;
    }
    const STEP: f32 = 1.0;
    let mut y = top.floor();
    let mut xs: Vec<f32> = Vec::new();
    while y < bottom {
        let yc = y + STEP / 2.0;
        xs.clear();
        for r in rings {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a.y <= yc) != (b.y <= yc) {
                    xs.push(a.x + (yc - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            let (x0, x1) = (pair[0].max(clip.left()), pair[1].min(clip.right()));
            if x1 > x0 {
                mesh.add_colored_rect(Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + STEP)), color);
            }
        }
        y += STEP;
    }
    mesh
}

/// Draws the fabric of the appliqué objects under the stitches, and the
/// edges with no cover as dashed lines.
pub fn draw_fabric(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let d = st.doc.design();
    let rot = d.rotation_transform();
    let view: &CanvasView = &st.view;
    let to_screen = |p: Point, t: &Transform| view.to_screen(rect, t.apply(p));
    for o in d.objects() {
        let ObjectKind::Applique { pieces, params } = &o.kind else {
            continue;
        };
        if !st.view_filter.shows(o) {
            continue;
        }
        // Each piece in its color, the bottom layer first.
        let mut sorted: Vec<&emb_model::AppliquePiece> = pieces.iter().collect();
        sorted.sort_by_key(|pc| pc.layer);
        for pc in sorted {
            let fabric = color32(pc.fabric_color(params)).gamma_multiply(0.85);
            let mut rings: Vec<Vec<Pos2>> = Vec::new();
            for path in std::iter::once(&pc.outer).chain(pc.holes.iter()) {
                for pl in path.flatten(view.tolerance()) {
                    rings.push(pl.points.iter().map(|p| to_screen(*p, &rot)).collect());
                }
            }
            painter.add(Shape::mesh(scanline_mesh(&rings, fabric, rect)));
            if params.cover {
                let edges = outline_edges(&pc.outer, view.tolerance());
                for e in &pc.open_edges {
                    if let Some(pts) = edges.get(*e) {
                        let pts: Vec<Pos2> = pts.iter().map(|p| to_screen(*p, &rot)).collect();
                        painter.extend(Shape::dashed_line(
                            &pts,
                            Stroke::new(1.5, Color32::from_gray(90)),
                            5.0,
                            4.0,
                        ));
                    }
                }
            }
        }
    }
}

impl AppState {
    /// The appliqué pieces for the cutter file: of the selected appliqué
    /// objects, or of all of them when none is selected.
    pub fn cutter_pieces(&self) -> Vec<emb_formats::cutter::CutPiece> {
        let d = self.doc.design();
        let sel: Vec<_> = self
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|id| {
                d.object(*id)
                    .is_some_and(|o| matches!(o.kind, emb_model::ObjectKind::Applique { .. }))
            })
            .collect();
        emb_formats::cutter::pieces_of(d, (!sel.is_empty()).then_some(sel.as_slice()))
    }

    /// Writes the cutter file (SVG or DXF, by the extension of `path`).
    pub fn write_cutter_file(&self, path: &std::path::Path) -> Result<usize, String> {
        let pieces = self.cutter_pieces();
        if pieces.is_empty() {
            return Err("The design has no appliqué objects.".into());
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let text = match ext.as_str() {
            "dxf" => emb_formats::cutter::dxf(&pieces),
            _ => emb_formats::cutter::svg(&pieces),
        };
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(pieces.len())
    }

    /// Asks for a file and writes the cutter file of the appliqué pieces.
    pub fn cutter_file_dialog(&mut self) {
        let name = format!("{} cut.svg", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Appliqué cutter file")
            .add_filter("SVG (cutting machines)", &["svg"])
            .add_filter("DXF (cutter and laser programs)", &["dxf"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        match self.write_cutter_file(&p) {
            Ok(n) => {
                self.message = crate::i18n::fill(
                    "Wrote the cut lines of {} appliqué piece{} to {} (real size)",
                    &[&n, &(if n == 1 { "" } else { "s" }), &(p.display())],
                )
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Cutter file not written".into(),
                    lines: vec![e],
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path};

    use super::*;
    use crate::test_util::Harness;

    fn fill(h: &mut Harness, x: f64) -> ObjectId {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("red", Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + 20.0, 20.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        })
    }

    #[test]
    fn convert_and_combine() {
        let mut h = Harness::empty();
        let a = fill(&mut h, 0.0);
        let b = fill(&mut h, 40.0);
        h.st.select_all();
        h.st.convert_to_applique();
        assert!(
            h.st.doc
                .design()
                .objects()
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::Applique { .. }))
        );
        let steps = h.st.doc.undo_len();
        h.st.combine_applique();
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        assert_eq!(d.objects()[0].id, a);
        assert!(matches!(&d.objects()[0].kind, ObjectKind::Applique { pieces, .. } if pieces.len() == 2));
        assert!(d.object(b).is_none());
        // Two pieces: two stops in the plan, not four.
        let plan = h.st.plan_view().plan.clone();
        assert_eq!(plan.stops.len(), 2);
        // Break apart gives two objects again.
        h.st.select_all();
        h.st.break_apart();
        assert_eq!(h.st.doc.design().objects().len(), 2);
    }

    #[test]
    fn a_patch_with_a_hole_has_no_warnings() {
        let mut h = Harness::empty();
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("navy", Rgb(25, 35, 90));
            let c = d.hoop.center;
            d.add_object(
                t,
                ObjectKind::Applique {
                    pieces: vec![AppliquePiece::new(
                        Path::ellipse(c, 40.0, 28.0),
                        vec![Path::ellipse(c, 10.0, 10.0)],
                    )],
                    params: AppliqueParams::default(),
                },
            )
        });
        let info = h.st.design_info();
        assert!(info.issues.is_empty(), "{:?}", info.issues);
        assert_eq!(info.stops, 2);
    }

    #[test]
    fn scanlines_fill_a_ring_with_a_hole() {
        let outer = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        let hole = vec![
            Pos2::new(3.0, 3.0),
            Pos2::new(7.0, 3.0),
            Pos2::new(7.0, 7.0),
            Pos2::new(3.0, 7.0),
        ];
        let clip = Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0));
        let m = scanline_mesh(std::slice::from_ref(&outer), Color32::RED, clip);
        assert_eq!(m.vertices.len(), 10 * 4, "one rectangle for each line");
        let m = scanline_mesh(&[outer, hole], Color32::RED, clip);
        assert_eq!(m.vertices.len(), (6 + 4 * 2) * 4, "two rectangles where the hole is");
    }

    #[test]
    fn properties_draw_and_edit() {
        let mut h = Harness::empty();
        fill(&mut h, 0.0);
        h.st.select_all();
        h.st.convert_to_applique();
        let id = h.st.selection.single().unwrap();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
            let painter = ui.painter().clone();
            draw_fabric(&painter, Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN), st);
        });
        out.textures_delta.clear();
        assert_eq!(h.st.doc.design().object(id).unwrap().type_name(), "Appliqué");
    }

    #[test]
    fn vectors_and_linked_fills_become_applique_and_stay() {
        let mut h = Harness::empty();
        let style = emb_model::VectorStyle::filled(Rgb(0, 120, 0));
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("green", Rgb(0, 120, 0));
            let square = |x: f64| Path::rect(BBox::new(Point::new(x, 0.0), Point::new(x + 20.0, 20.0)));
            vec![
                d.add_object(
                    t,
                    ObjectKind::Vector {
                        path: square(0.0),
                        style,
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Fill {
                        outer: square(40.0),
                        holes: Vec::new(),
                        params: Default::default(),
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Fill {
                        outer: square(50.0),
                        holes: Vec::new(),
                        params: Default::default(),
                    },
                ),
            ]
        });
        // The two fills are flattened and linked: the back one is cut.
        h.st.doc.edit("Flatten", |d| {
            let opts = emb_model::ShapeOptions {
                linked: true,
                ..Default::default()
            };
            d.shape(&ids[1..], emb_model::ShapeOp::Flatten, opts).unwrap();
        });
        let d = h.st.doc.design();
        assert_eq!(d.shape_links.len(), 1);
        // The pieces of the cut: the back square and the front square.
        let piece = |orig: ObjectId| {
            d.shape_links[0]
                .cuts
                .iter()
                .find(|c| c.original.id == orig)
                .map(|c| c.pieces[0])
                .unwrap()
        };
        let (back, front) = (piece(ids[1]), piece(ids[2]));
        h.st.selection.set_all([ids[0], back]);
        h.st.convert_to_applique();
        let d = h.st.doc.design();
        assert!(matches!(d.object(ids[0]).unwrap().kind, ObjectKind::Applique { .. }));
        assert!(matches!(d.object(back).unwrap().kind, ObjectKind::Applique { .. }));
        // A move of the front square cuts again: the back piece stays appliqué.
        h.st.doc.edit("Move", |d| {
            d.transform_objects(&[front], &Transform::translate(5.0, 0.0));
        });
        let d = h.st.doc.design();
        assert_eq!(d.shape_links.len(), 1);
        assert!(
            matches!(d.object(back).unwrap().kind, ObjectKind::Applique { .. }),
            "{:?}",
            d.object(back).unwrap().kind
        );
    }

    #[test]
    fn the_cutter_file_has_the_pieces_of_the_selection_or_all() {
        let mut h = crate::test_util::Harness::new();
        let dir = std::env::temp_dir().join(format!("emb-cutter-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(
            h.st.write_cutter_file(&dir.join("none.svg")).is_err(),
            "no appliqué yet"
        );
        let ids: Vec<_> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        h.st.selection.set_all(vec![ids[0]]);
        h.st.convert_to_applique();
        assert_eq!(h.st.cutter_pieces().len(), 1);
        let n = h.st.write_cutter_file(&dir.join("cut.dxf")).unwrap();
        assert_eq!(n, 1);
        let text = std::fs::read_to_string(dir.join("cut.dxf")).unwrap();
        assert!(text.contains("POLYLINE"));
        h.st.write_cutter_file(&dir.join("cut.svg")).unwrap();
        assert!(std::fs::read_to_string(dir.join("cut.svg")).unwrap().contains("<path"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
