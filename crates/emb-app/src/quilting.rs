//! The Quilting window (P11 Milestone 6, ADR 0049): echo quilting round
//! the selected shapes, a stipple or meander fill of the background round
//! them, and edge-to-edge quilt blocks that sew in hoopings. The engine
//! work is in `emb_model::quilt` and `emb_stitch::meander`.

use eframe::egui;
use emb_geom::{BBox, Path, Point, Polyline};
use emb_model::quilt::{EchoSpec, block_lines, fill_of};
use emb_model::{Design, FillParams, FillPattern, MeanderTile, ObjectId, ObjectKind, RunStyle, RunningParams};

use crate::app::AppState;
use crate::artwork::{count, number};
use crate::i18n::tr;

/// The part of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuiltPart {
    Echo,
    Background,
    Block,
}

/// The window state.
#[derive(Debug, Clone)]
pub struct QuiltingWindow {
    pub open: bool,
    pub part: QuiltPart,
    pub echo: EchoSpec,
    /// Echo lines as triple (bean) stitch.
    pub triple: bool,
    /// The background: the gap round the shapes, the margin of the box when
    /// there is no border shape, the tiles (`None`: stipple) and spacing.
    pub gap_mm: f64,
    pub margin_mm: f64,
    pub tiles: Option<MeanderTile>,
    pub spacing_mm: f64,
    /// The quilt block: size, the gaps of the pattern, a meander instead of
    /// the selected lines, and an outline round the block.
    pub block_w_mm: f64,
    pub block_h_mm: f64,
    pub gap_x_mm: f64,
    pub gap_y_mm: f64,
    pub block_meander: Option<MeanderTile>,
    /// The tile size of a meander block.
    pub block_spacing_mm: f64,
    pub outline: bool,
}

impl Default for QuiltingWindow {
    fn default() -> Self {
        QuiltingWindow {
            open: false,
            part: QuiltPart::Echo,
            echo: EchoSpec::default(),
            triple: false,
            gap_mm: 1.5,
            margin_mm: 15.0,
            tiles: None,
            spacing_mm: 3.0,
            block_w_mm: 300.0,
            block_h_mm: 300.0,
            gap_x_mm: 0.0,
            gap_y_mm: 5.0,
            block_meander: None,
            block_spacing_mm: 10.0,
            outline: true,
        }
    }
}

/// Stitch length of quilting lines, in mm.
const QUILT_LEN_MM: f64 = 3.0;

fn quilt_line(triple: bool) -> RunningParams {
    RunningParams {
        stitch_length_mm: QUILT_LEN_MM,
        repeats: if triple { 3 } else { 1 },
        ..RunningParams::default()
    }
}

/// A path of open lines.
fn path_of(lines: &[Polyline]) -> Path {
    let parts: Vec<Path> = lines
        .iter()
        .filter(|l| l.points.len() >= 2)
        .map(|l| {
            let p = l.points[1..]
                .iter()
                .fold(Path::new().move_to(l.points[0]), |p, q| p.line_to(*q));
            if l.closed { p.close() } else { p }
        })
        .collect();
    Path::joined(&parts)
}

impl AppState {
    /// Adds echo quilting round the selection: one line after it. One undo step.
    pub fn add_echo(&mut self) -> Option<ObjectId> {
        let ids = self.sel();
        let spec = self.quilting.echo;
        let d = self.doc.design();
        let path = match d.echo_path(&ids, &spec) {
            Ok(p) => p,
            Err(e) => {
                self.message = e.to_string();
                return None;
            }
        };
        let thread = ids.iter().filter_map(|id| d.object(*id)).map(|o| o.thread).next()?;
        let at = ids.iter().filter_map(|id| d.index_of(*id)).max().map_or(0, |i| i + 1);
        let params = quilt_line(self.quilting.triple);
        let id = self.doc.edit("Echo quilting", |d| {
            let id = d.insert_object(at, thread, ObjectKind::Running { path, params });
            if let Some(o) = d.object_mut(id) {
                o.name = Some("Echo quilting".into());
            }
            id
        });
        self.selection.set(id);
        self.message = crate::i18n::fill("{} echo lines added.", &[&spec.rows]);
        Some(id)
    }

    /// Fills the background round the selection (stipple or meander), after
    /// it. One undo step.
    pub fn add_background(&mut self) -> Option<ObjectId> {
        let ids = self.sel();
        let w = self.quilting.clone();
        let d = self.doc.design();
        let area = match d.background_area(&ids, w.gap_mm, w.margin_mm) {
            Ok(a) => a,
            Err(e) => {
                self.message = e.to_string();
                return None;
            }
        };
        let thread = ids.iter().filter_map(|id| d.object(*id)).map(|o| o.thread).next()?;
        let at = ids.iter().filter_map(|id| d.index_of(*id)).max().map_or(0, |i| i + 1);
        let pattern = match w.tiles {
            Some(tile) => FillPattern::Meander { tile },
            None => FillPattern::Stipple { style: RunStyle::Run },
        };
        let params = FillParams {
            spacing_mm: w.spacing_mm,
            stitch_length_mm: QUILT_LEN_MM,
            ..FillParams::default().with_pattern(pattern)
        };
        let kind = fill_of(&area, params);
        let id = self.doc.edit("Background quilting", |d| {
            let id = d.insert_object(at, thread, kind);
            if let Some(o) = d.object_mut(id) {
                o.name = Some("Background quilting".into());
            }
            id
        });
        self.selection.set(id);
        self.message = tr("Background quilting added round the selection.").into();
        Some(id)
    }

    /// Makes a quilt block: the selected lines repeated over the block (or
    /// a meander over it), and the hoopings it needs. One undo step.
    pub fn make_quilt_block(&mut self) -> bool {
        let w = self.quilting.clone();
        let ids = self.sel();
        let d = self.doc.design();
        let lines: Vec<(ObjectId, Vec<Polyline>)> = ids
            .iter()
            .filter_map(|id| match d.object(*id).map(|o| &o.kind) {
                Some(ObjectKind::Running { path, .. }) => Some((*id, path.flatten(0.05))),
                _ => None,
            })
            .collect();
        let all: Vec<Polyline> = lines.iter().flat_map(|(_, l)| l.iter().cloned()).collect();
        let center =
            BBox::from_points(all.iter().flat_map(|l| l.points.iter().copied())).map_or(Point::ZERO, |b| b.center());
        let (block, thread, params, used) = match w.block_meander {
            Some(tile) => {
                let rect = Path::rect(BBox::from_center_size(center, w.block_w_mm, w.block_h_mm));
                let shape = emb_stitch::travel::Shape::new(&rect.flatten(0.05), &[]);
                let pts = emb_stitch::meander::meander_lines(&shape, tile, 0.0, w.block_spacing_mm.max(2.0), center);
                let block: Vec<Polyline> = pts
                    .into_iter()
                    .map(|points| Polyline { points, closed: false })
                    .collect();
                let thread = d.threads.first().map(|t| t.id);
                (block, thread, quilt_line(w.triple), Vec::new())
            }
            None => {
                if all.is_empty() {
                    self.message = tr("Select the lines of the pattern, or choose a meander for the block.").into();
                    return false;
                }
                let block = block_lines(
                    &all,
                    center,
                    w.block_w_mm,
                    w.block_h_mm,
                    Point::new(w.gap_x_mm, w.gap_y_mm),
                );
                let first = d.object(lines[0].0);
                let params = match first.map(|o| &o.kind) {
                    Some(ObjectKind::Running { params, .. }) => params.clone(),
                    _ => quilt_line(w.triple),
                };
                (
                    block,
                    first.map(|o| o.thread),
                    params,
                    lines.iter().map(|(id, _)| *id).collect(),
                )
            }
        };
        let Some(thread) = thread.or_else(|| d.threads.first().map(|t| t.id)) else {
            return false;
        };
        if block.is_empty() {
            self.message = tr("The pattern does not make a block (it is too small or too large).").into();
            return false;
        }
        let rect = BBox::from_center_size(center, w.block_w_mm, w.block_h_mm);
        let apply = |d: &mut Design| -> Vec<ObjectId> {
            for id in &used {
                d.remove_object(*id);
            }
            let mut new = vec![d.add_object(
                thread,
                ObjectKind::Running {
                    path: path_of(&block),
                    params: params.clone(),
                },
            )];
            if w.outline {
                new.push(d.add_object(
                    thread,
                    ObjectKind::Running {
                        path: Path::rect(rect),
                        params: quilt_line(false),
                    },
                ));
            }
            new
        };
        // The hoopings of the block, if it is larger than the hoop.
        let layout = self.layout_hoop().and_then(|hoop| {
            let mut probe = self.doc.design().clone();
            probe.hooping = None;
            apply(&mut probe);
            emb_model::HoopingLayout::auto(&probe, &hoop, emb_model::hooping::DEFAULT_OVERLAP_MM, self.margin_mm)
                .filter(|l| l.hoopings.len() > 1)
        });
        let n = layout.as_ref().map_or(1, |l| l.hoopings.len());
        let new = self.doc.edit("Quilt block", |d| {
            let new = apply(d);
            d.hooping = layout;
            new
        });
        self.selection.set_all(new);
        if n > 1 {
            self.hoopings_ui.mode = true;
        }
        self.message = crate::i18n::fill(
            "Quilt block of {} × {} mm in {} hoopings.",
            &[&format!("{:.0}", w.block_w_mm), &format!("{:.0}", w.block_h_mm), &n],
        );
        true
    }
}

fn tiles_choice(ui: &mut egui::Ui, salt: &str, v: &mut Option<MeanderTile>, none: &str) {
    ui.horizontal(|ui| {
        ui.label(tr("Pattern"));
        egui::ComboBox::from_id_salt(salt)
            .selected_text(v.map_or(tr(none), |t| tr(t.name())))
            .show_ui(ui, |ui| {
                ui.selectable_value(v, None, tr(none));
                for t in MeanderTile::ALL {
                    ui.selectable_value(v, Some(t), crate::i18n::fill("Meander: {}", &[&tr(t.name())]));
                }
            });
    });
}

/// The Quilting window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.quilting.open {
        return;
    }
    let mut open = true;
    let mut run: Option<QuiltPart> = None;
    let has = !st.selection.is_empty();
    crate::dock::docked(egui::Window::new(tr("Quilting"))
        .id(egui::Id::new("Quilting")), ctx, egui::Id::new("Quilting"), st.canvas_rect)
        .open(&mut open)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            let w = &mut st.quilting;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut w.part, QuiltPart::Echo, tr("Echo"));
                ui.selectable_value(&mut w.part, QuiltPart::Background, tr("Background"));
                ui.selectable_value(&mut w.part, QuiltPart::Block, tr("Quilt block"));
            });
            ui.separator();
            match w.part {
                QuiltPart::Echo => {
                    ui.weak(tr(
                        "Lines round the selected shapes, as one line. Select a border shape too, and the lines stop at its edge.",
                    ));
                    number(
                        ui,
                        tr("Distance"),
                        tr("The distance of the first line from the shapes."),
                        &mut w.echo.distance_mm,
                        0.0..=50.0,
                    );
                    number(
                        ui,
                        tr("Spacing"),
                        tr("The distance between two lines."),
                        &mut w.echo.spacing_mm,
                        1.0..=50.0,
                    );
                    count(ui, tr("Lines"), &mut w.echo.rows, 1..=emb_model::quilt::MAX_ECHOES);
                    ui.checkbox(&mut w.triple, tr("Triple stitch"))
                        .on_hover_text(tr("Each stitch three times, so the line shows more."));
                    if ui.add_enabled(has, egui::Button::new(tr("Add echo lines"))).clicked() {
                        run = Some(QuiltPart::Echo);
                    }
                }
                QuiltPart::Background => {
                    ui.weak(tr(
                        "A fill of the background round the selected shapes and appliqué, so they stand up. Select a border shape too, or the fill is a box round them.",
                    ));
                    tiles_choice(ui, "bg_tiles", &mut w.tiles, tr("Stipple"));
                    number(
                        ui,
                        tr("Spacing"),
                        tr("The distance between the lines of the fill."),
                        &mut w.spacing_mm,
                        1.0..=20.0,
                    );
                    number(
                        ui,
                        tr("Gap"),
                        tr("The space between the shapes and the fill."),
                        &mut w.gap_mm,
                        0.0..=20.0,
                    );
                    number(
                        ui,
                        tr("Margin"),
                        tr("How far the box goes past the shapes, with no border shape."),
                        &mut w.margin_mm,
                        0.0..=100.0,
                    );
                    if ui.add_enabled(has, egui::Button::new(tr("Add background"))).clicked() {
                        run = Some(QuiltPart::Background);
                    }
                }
                QuiltPart::Block => {
                    ui.weak(tr(
                        "The selected lines repeated over a quilt block, or a meander over it. A block larger than the hoop sews in hoopings, and the lines of two hoopings meet.",
                    ));
                    tiles_choice(ui, "block_tiles", &mut w.block_meander, tr("Selected lines"));
                    number(ui, tr("Width"), tr("The width of the block."), &mut w.block_w_mm, 20.0..=3000.0);
                    number(ui, tr("Height"), tr("The height of the block."), &mut w.block_h_mm, 20.0..=3000.0);
                    if w.block_meander.is_some() {
                        number(
                            ui,
                            tr("Spacing"),
                            tr("The distance between the lines of the meander."),
                            &mut w.block_spacing_mm,
                            2.0..=50.0,
                        );
                    } else {
                        number(
                            ui,
                            tr("Gap across"),
                            tr("The space between two copies side by side (0: they join)."),
                            &mut w.gap_x_mm,
                            -50.0..=100.0,
                        );
                        number(
                            ui,
                            tr("Gap down"),
                            tr("The space between two rows of copies."),
                            &mut w.gap_y_mm,
                            -50.0..=100.0,
                        );
                    }
                    ui.checkbox(&mut w.outline, tr("Line round the block"));
                    let can = has || w.block_meander.is_some();
                    if ui.add_enabled(can, egui::Button::new(tr("Make quilt block"))).clicked() {
                        run = Some(QuiltPart::Block);
                    }
                }
            }
        });
    st.quilting.open = open;
    match run {
        Some(QuiltPart::Echo) => {
            st.add_echo();
        }
        Some(QuiltPart::Background) => {
            st.add_background();
        }
        Some(QuiltPart::Block) => {
            st.make_quilt_block();
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use emb_model::Rgb;

    use super::*;
    use crate::test_util::Harness;

    fn heart_and_border(h: &mut Harness) -> (ObjectId, ObjectId) {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            let shape = d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::ellipse(Point::ZERO, 15.0, 15.0),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            );
            let border = d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(-40.0, -40.0), Point::new(40.0, 40.0))),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            );
            (shape, border)
        })
    }

    #[test]
    fn echo_and_background_are_one_undo_step_each() {
        let mut h = Harness::empty();
        let (shape, border) = heart_and_border(&mut h);
        h.st.selection.set_all(vec![shape, border]);
        let echo = h.st.add_echo().expect("echo lines");
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 3);
        let Some(ObjectKind::Running { path, .. }) = d.object(echo).map(|o| &o.kind) else {
            panic!()
        };
        assert_eq!(path.subpaths().len(), 1, "one line");
        assert!(emb_stitch::stitch_object(d.object(echo).unwrap(), Point::ZERO).is_ok());
        h.st.selection.set_all(vec![shape, border]);
        let bg = h.st.add_background().expect("a background");
        let d = h.st.doc.design();
        let Some(ObjectKind::Fill { params, holes, .. }) = d.object(bg).map(|o| &o.kind) else {
            panic!()
        };
        assert!(matches!(params.pattern, FillPattern::Stipple { .. }));
        assert_eq!(holes.len(), 1, "a hole for the shape");
        assert!(emb_stitch::stitch_object(d.object(bg).unwrap(), Point::ZERO).is_ok());
        h.st.doc.undo();
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 2);
    }

    #[test]
    fn a_quilt_block_larger_than_the_hoop_sews_in_hoopings_with_joins_that_meet() {
        let mut h = Harness::empty();
        h.st.doc.edit("Hoop", |d| {
            d.machine = Some("husqvarna_topaz20".into());
            d.hoop.selection = emb_model::HoopSelection::Preset {
                id: "topaz20_crown".into(),
            };
        });
        let wave = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Blue", Rgb(0, 0, 200));
            let path = (1..=4).fold(Path::new().move_to(Point::ZERO), |p, i| {
                let x = 10.0 * f64::from(i);
                p.line_to(Point::new(x, if i % 2 == 1 { -5.0 } else { 0.0 }))
            });
            d.add_object(
                t,
                ObjectKind::Running {
                    path,
                    params: RunningParams::default(),
                },
            )
        });
        h.st.selection.set(wave);
        assert!(h.st.make_quilt_block());
        let d = h.st.doc.design();
        let layout = d.hooping.clone().expect("hoopings");
        assert_eq!(layout.hoopings.len(), 4, "300 × 300 mm in the 200 × 260 mm hoop");
        let hoop = h.st.layout_hoop().unwrap();
        let outs = d.hoopings(&hoop);
        assert_eq!(outs.len(), 4);
        // Each end of a line that is on a split line is the end of a line
        // in another hooping: the joins meet.
        let ends: Vec<(usize, Point)> = outs
            .iter()
            .enumerate()
            .flat_map(|(i, o)| {
                o.design
                    .objects()
                    .iter()
                    .filter(|x| matches!(x.kind, ObjectKind::Running { .. }) && x.thread == d.objects()[0].thread)
                    .flat_map(|x| x.kind.paths().flat_map(|p| p.flatten(0.05)).collect::<Vec<_>>())
                    .flat_map(move |l| [(i, l.points[0]), (i, l.points[l.points.len() - 1])])
                    .collect::<Vec<_>>()
            })
            .collect();
        let on_split = |p: Point| {
            layout.splits.iter().any(|s| {
                let (a, b) = (s[0], s[s.len() - 1]);
                p.distance_to_segment(a, b) < 1e-6
            })
        };
        let mut joins = 0;
        for (i, p) in &ends {
            if on_split(*p) {
                assert!(
                    ends.iter().any(|(j, q)| j != i && q.distance(*p) < 1e-6),
                    "{p:?} of hooping {i} meets no line"
                );
                joins += 1;
            }
        }
        assert!(joins > 20, "{joins}");
    }

    #[test]
    fn a_meander_block_needs_no_selection() {
        let mut h = Harness::empty();
        h.st.doc.edit("Thread", |d| d.add_thread("White", Rgb(250, 250, 250)));
        h.st.quilting.block_meander = Some(MeanderTile::Square);
        h.st.quilting.block_w_mm = 60.0;
        h.st.quilting.block_h_mm = 40.0;
        assert!(h.st.make_quilt_block());
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2, "the meander and the line round it");
        assert!(emb_stitch::stitch_object(&d.objects()[0], Point::ZERO).is_ok());
    }
}
