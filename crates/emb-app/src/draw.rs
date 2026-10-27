//! Draw and edit tools on the canvas (all tools except Select and Reshape).
//!
//! Point tools (Line, Manual, Shape, Complex fill, Column, Column B, Column C,
//! Turning, and the edit tools Add hole, Polygon and Line select):
//! - Left click places a corner point; right click places a curve point.
//!   (A right drag still pans the view.)
//! - Enter finishes (or ends one part of a tool with parts); Backspace
//!   removes the last point; Esc cancels, and a second Esc goes back to Select.
//! - A closed outline also finishes when you click its first point again.
//! - Column: click one side, then the other side, in pairs. Each pair sets
//!   the stitch direction there.
//! - Complex fill: the outline, Enter, then each hole, Enter; Enter again finishes.
//! - Column B: side A, Enter, side B, Enter.
//! - Turning: the outline, Enter, then stitch angle lines (two clicks each), Enter.
//!
//! Tools with a fixed number of points finish by themselves: Knife, Stitch
//! angle and Mirror line take two points, Reference points four.
//!
//! Drag tools: Freehand line and shape, Lasso; the box tools (Rectangle,
//! Rounded rectangle, Ellipse, Star, Ring): drag from corner to corner. Ctrl
//! makes equal sides; Alt draws from the center.
//!
//! Points snap to the grid and guides when snap is on. A finished object is
//! one undo step. The tool stays active, so you can draw the next object.

use eframe::egui::{self, Color32, CursorIcon, Modifiers, Pos2, Rect, Shape, Stroke, Vec2};
use emb_geom::{BBox, Path, Point, math};
use emb_model::{
    AppliquePiece, CurrentProperties, Design, ObjectId, ObjectKind, Rgb, RunPass, RunStyle, RunningParams,
    SatinGeometry, SatinParams, ThreadId,
};

use crate::app::AppState;
use crate::i18n::tr;
use crate::manipulate::ViewMap;
use crate::workspace::Defaults;

/// Snap distance to close a shape on its first point, in screen points.
const CLOSE_SNAP: f32 = 8.0;
/// Color of the first thread in a new design.
const FIRST_THREAD: Rgb = Rgb(30, 60, 140);
/// Freehand: a new point when the pointer moved this far, in screen points.
const FREEHAND_STEP: f32 = 3.0;
/// Freehand: points nearer than this to the smoothed line are dropped, in mm.
const FREEHAND_TOL_MM: f64 = 0.3;
/// Inner radius of a star, as a fraction of the outer radius.
const STAR_INNER: f64 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Reshape,
    Lasso,
    PolySelect,
    LineSelect,
    Line,
    Manual,
    Freehand,
    Shape,
    ComplexFill,
    FreeShape,
    Rectangle,
    RoundRect,
    Ellipse,
    Star,
    /// A satin column: side A, Enter, side B, Enter (owner request of
    /// 2026-10-26; before, pairs of points).
    Column,
    /// Column Pairs: a satin column from pairs of points, side A then B.
    ColumnB,
    ColumnC,
    Ring,
    Turning,
    Applique,
    Lettering,
    Knife,
    AddHole,
    StitchAngle,
    /// A stitch direction line through a satin or a satin-type fill: the
    /// stitches go across it (beta tester request, 2026-10-24).
    DirectionLine,
    MirrorLine,
    RefPoints,
    Measure,
    Zoom,
    /// Crop the backdrop picture (P6 Milestone 5).
    CropRect,
    CropEllipse,
    CropPolygon,
    /// Click a color area of the backdrop to stitch it (P6 Milestone 6).
    ClickStitch,
    /// Paint, erase and draw lines on a cross-stitch grid (P9 Milestone 1).
    CrossStitch,
    CrossErase,
    Backstitch,
    /// Rhinestones: single stones, a run along a line, a fill (P9 Milestone 3).
    Stone,
    StoneRun,
    StoneFill,
}

impl Tool {
    /// All tools. The ribbon test checks that each one has a button.
    #[cfg(test)]
    pub const ALL: [Tool; 40] = [
        Tool::Select,
        Tool::Reshape,
        Tool::Lasso,
        Tool::PolySelect,
        Tool::LineSelect,
        Tool::Line,
        Tool::Manual,
        Tool::Freehand,
        Tool::Shape,
        Tool::ComplexFill,
        Tool::FreeShape,
        Tool::Rectangle,
        Tool::RoundRect,
        Tool::Ellipse,
        Tool::Star,
        Tool::Column,
        Tool::ColumnB,
        Tool::ColumnC,
        Tool::Ring,
        Tool::Turning,
        Tool::Applique,
        Tool::Lettering,
        Tool::Knife,
        Tool::AddHole,
        Tool::StitchAngle,
        Tool::DirectionLine,
        Tool::MirrorLine,
        Tool::RefPoints,
        Tool::Measure,
        Tool::Zoom,
        Tool::CropRect,
        Tool::CropEllipse,
        Tool::CropPolygon,
        Tool::ClickStitch,
        Tool::CrossStitch,
        Tool::CrossErase,
        Tool::Backstitch,
        Tool::Stone,
        Tool::StoneRun,
        Tool::StoneFill,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => tr("Select"),
            Tool::Reshape => tr("Reshape"),
            Tool::Lasso => tr("Lasso"),
            Tool::PolySelect => tr("Poly Select"),
            Tool::LineSelect => tr("Line Select"),
            Tool::Line => tr("Line"),
            Tool::Manual => tr("Manual"),
            Tool::Freehand => tr("Freehand"),
            Tool::Shape => tr("Shape"),
            Tool::ComplexFill => tr("Complex Fill"),
            Tool::FreeShape => tr("Free Shape"),
            Tool::Rectangle => tr("Rectangle"),
            Tool::RoundRect => tr("Round Rect"),
            Tool::Ellipse => tr("Ellipse"),
            Tool::Star => tr("Star"),
            Tool::Column => tr("Column"),
            Tool::ColumnB => tr("Column Pairs"),
            Tool::ColumnC => tr("Column C"),
            Tool::Ring => tr("Ring"),
            Tool::Turning => tr("Turning"),
            Tool::Applique => tr("Appliqué"),
            Tool::Lettering => tr("Lettering"),
            Tool::Knife => tr("Knife"),
            Tool::AddHole => tr("Add Hole"),
            Tool::StitchAngle => tr("Angle"),
            Tool::DirectionLine => tr("Direction Line"),
            Tool::MirrorLine => tr("Mirror Line"),
            Tool::RefPoints => tr("Ref Points"),
            Tool::Measure => tr("Measure"),
            Tool::Zoom => tr("Zoom"),
            Tool::CropRect => tr("Crop Box"),
            Tool::CropEllipse => tr("Crop Oval"),
            Tool::CropPolygon => tr("Crop Shape"),
            Tool::ClickStitch => tr("Click to Stitch"),
            Tool::CrossStitch => tr("Cross Stitch"),
            Tool::CrossErase => tr("Erase Cells"),
            Tool::Backstitch => tr("Backstitch"),
            Tool::Stone => tr("Stone"),
            Tool::StoneRun => tr("Stone Run"),
            Tool::StoneFill => tr("Stone Fill"),
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Tool::Select => tr(
                "Select, move, scale and rotate objects. Drag a box to the right: the objects all inside it. \
                 Drag a box to the left: the objects that it touches.",
            ),
            Tool::Reshape => tr("Edit the nodes of an object, and its start and end points. \
                 Double-click: add a node, or change corner and curve. Del: remove a node."),
            Tool::Lasso => tr("Select the objects inside a freehand line. Drag around them."),
            Tool::PolySelect => tr("Select the objects inside a polygon. Click its corners, Enter."),
            Tool::LineSelect => tr("Select the objects that a line crosses. Click its points, Enter."),
            Tool::Line => tr("Running stitch line. Left click: corner. Right click: curve point. Enter: finish."),
            Tool::Manual => tr("Manual stitches: each click is one needle point. Enter: finish."),
            Tool::Freehand => tr("Running stitch line drawn by hand. Drag; the line is smoothed."),
            Tool::Shape => tr("Filled shape (tatami). Left click: corner. Right click: curve point. \
                 Enter or click the first point: finish."),
            Tool::ComplexFill => tr("Fill with holes: the outline, Enter, then each hole, Enter. Enter again: finish."),
            Tool::FreeShape => tr("Filled shape drawn by hand. Drag around it; the outline is smoothed."),
            Tool::Rectangle => tr("Filled rectangle. Drag corner to corner. Ctrl: square. Alt: from center."),
            Tool::RoundRect => tr("Filled rectangle with round corners. Drag corner to corner."),
            Tool::Ellipse => tr("Filled ellipse. Drag corner to corner. Ctrl: circle. Alt: from center."),
            Tool::Star => tr("Filled star. Drag corner to corner. Set the number of points in the property bar."),
            Tool::Column => tr(
                "Satin column: click one side, Enter, then the other side, Enter. The satin fills \
                 the space between them. Right click: curve point.",
            ),
            Tool::ColumnB => tr(
                "Satin column by pairs: click a point on one side, then the matching point on the \
                 other, pair by pair. Right click for curve points. Enter: finish.",
            ),
            Tool::ColumnC => tr("Satin column of fixed width along a line. Left click: corner. \
                 Right click: curve point. Enter: finish."),
            Tool::Ring => tr("Satin ring. Drag corner to corner. Set the width in the property bar."),
            Tool::Turning => tr("Turning fill: the outline, Enter, then stitch angle lines across it \
                 (two clicks each), Enter."),
            Tool::Applique => tr(
                "Appliqué: placement line, stop, tack-down, stop, cover satin. Click the outline, \
                 Enter; then holes, if any, each with Enter. Enter again: finish.",
            ),
            Tool::Lettering => tr("Lettering: click where the text starts, then type the text and choose the font."),
            Tool::Knife => tr("Cut the selected objects along a line. Click two points."),
            Tool::AddHole => tr("Add a hole to the selected fill. Click its outline, Enter."),
            Tool::StitchAngle => tr(
                "Stitch angle: click two points across the selected object. A fill gets this \
                 angle; a satin gets a new stitch direction line.",
            ),
            Tool::DirectionLine => tr(
                "Direction line: click points of a line through the selected satin or fill, Enter. \
                 The stitches go across the line at the angle in the property bar (90°: square to it), \
                 so they turn with the line. A fill with the satin stitch type keeps the line.",
            ),
            Tool::MirrorLine => tr("Mirror the selection about a line. Click two points. Ctrl: keep the original."),
            Tool::RefPoints => tr(
                "Move, turn and scale the selection by two points: click two points on it, \
                 then where they go.",
            ),
            Tool::Measure => tr("Measure a distance and an angle. Drag."),
            Tool::Zoom => tr("Drag a box to zoom to it. Click: zoom in. Shift+click: zoom out."),
            Tool::CropRect => tr("Crop the backdrop picture to a box: click two corners."),
            Tool::CropEllipse => tr("Crop the backdrop picture to an oval: click two corners of its box."),
            Tool::CropPolygon => tr("Crop the backdrop picture to a shape: click its corners, Enter."),
            Tool::ClickStitch => tr(
                "Click a color area of the backdrop picture: it becomes a fill, a satin, a center line \
                 or an outline (choose with the buttons next to this one).",
            ),
            Tool::CrossStitch => tr(
                "Cross stitch: click or drag over the cells of the grid to put crosses in them, in the \
                 current color. Shift+drag: erase. Choose full, half, quarter or three-quarter crosses \
                 in the property bar.",
            ),
            Tool::CrossErase => tr("Erase cross-stitch cells and backstitch lines: click or drag over them."),
            Tool::Backstitch => tr(
                "Backstitch lines on the cross-stitch grid: click cell corners or cell centers. \
                 Right click or Esc: end the line.",
            ),
            Tool::Stone => {
                tr("Rhinestones one at a time: each click puts a stone. Choose the stone in the property bar.")
            }
            Tool::StoneRun => tr(
                "A line of rhinestones. Left click: corner. Right click: curve point. Enter: finish. \
                 The stones go from end to end.",
            ),
            Tool::StoneFill => tr(
                "A shape filled with rhinestones. Left click: corner. Right click: curve point. \
                 Enter or click the first point: finish.",
            ),
        }
    }

    /// Instruction for the status bar, for the current state.
    pub fn prompt(self, points: usize, stages: usize) -> &'static str {
        match (self, points, stages) {
            (Tool::Select, ..) => "",
            (Tool::Reshape, ..) => tr(
                "Drag nodes or handles (Alt: corner). Double-click the outline to add a node. \
                 Drag the green or red point to set the start or end.",
            ),
            (Tool::ColumnB, n, _) if n % 2 == 0 && n < 4 => tr("Click a point on side A."),
            (Tool::ColumnB, n, _) if n % 2 == 1 => tr("Click the matching point on side B."),
            (Tool::ColumnB, ..) => tr("Click the next pair (side A, then B), or press Enter to finish."),
            (Tool::ComplexFill | Tool::Applique, _, 0) => {
                tr("Click the outline. Enter or click the first point: next, the holes.")
            }
            (Tool::ComplexFill | Tool::Applique, 0, _) => tr("Click a hole, or press Enter to finish."),
            (Tool::ComplexFill | Tool::Applique, ..) => {
                tr("Click the hole outline. Enter or click its first point: done.")
            }
            (Tool::Column, _, 0) => tr("Click side A, then press Enter."),
            (Tool::Column, ..) => tr("Click side B, then press Enter. The satin fills the space between them."),
            (Tool::Turning, _, 0) => tr("Click the outline. Enter or click the first point: next, the angle lines."),
            (Tool::Turning, n, _) if n % 2 == 1 => tr("Click the other end of the stitch angle line."),
            (Tool::Turning, ..) => tr("Click stitch angle lines across the shape (two or more). Enter: finish."),
            (Tool::Knife | Tool::StitchAngle | Tool::MirrorLine, 0, _) => tr("Click the first point of the line."),
            (Tool::Knife | Tool::StitchAngle | Tool::MirrorLine, ..) => tr("Click the second point."),
            (Tool::DirectionLine, 0, _) => tr("Click the first point of the direction line, at one end of the shape."),
            (Tool::DirectionLine, ..) => tr("Click the next point along the shape. Enter: finish."),
            (Tool::CropRect | Tool::CropEllipse, 0, _) => tr("Click a corner of the box on the picture."),
            (Tool::CropRect | Tool::CropEllipse, ..) => tr("Click the opposite corner."),
            (Tool::ClickStitch, ..) => tr("Click a color area of the backdrop picture."),
            (Tool::CrossStitch, ..) => tr("Drag over cells to put crosses in them. Shift+drag: erase."),
            (Tool::CrossErase, ..) => tr("Drag over cells and lines to remove them."),
            (Tool::Stone, ..) => tr("Click where a stone goes."),
            (Tool::Backstitch, 0, _) => tr("Click a cell corner or a cell center where the line starts."),
            (Tool::Backstitch, ..) => tr("Click the next point. Right click or Esc: end the line."),
            (Tool::RefPoints, 0, _) => tr("Click the first reference point on the selection."),
            (Tool::RefPoints, 1, _) => tr("Click the second reference point on the selection."),
            (Tool::RefPoints, 2, _) => tr("Click where the first point goes."),
            (Tool::RefPoints, ..) => tr("Click where the second point goes."),
            (t, ..) if t.is_box() => tr("Drag from corner to corner. Ctrl: equal sides. Alt: from center."),
            (Tool::Freehand | Tool::FreeShape | Tool::Lasso, ..) => tr("Drag to draw."),
            (Tool::Measure, ..) => tr("Drag to measure."),
            (Tool::Lettering, ..) => tr("Click where the baseline of the first line starts."),
            (Tool::Zoom, ..) => tr("Drag a box, or click to zoom in (Shift: out)."),
            (_, 0, _) => tr("Click the first point (left: corner, right: curve)."),
            (Tool::Shape | Tool::AddHole | Tool::PolySelect, 1 | 2, _) | (_, 1, _) => tr("Click the next point."),
            (Tool::Shape | Tool::AddHole, ..) => tr("Click more points. Enter or click the first point to finish."),
            _ => tr("Click more points, or press Enter to finish. Backspace: remove last point."),
        }
    }

    /// Click tools: each click places a point.
    pub fn uses_points(self) -> bool {
        matches!(
            self,
            Tool::Line
                | Tool::Manual
                | Tool::StoneRun
                | Tool::StoneFill
                | Tool::Shape
                | Tool::ComplexFill
                | Tool::Applique
                | Tool::Column
                | Tool::ColumnB
                | Tool::ColumnC
                | Tool::Turning
                | Tool::Knife
                | Tool::AddHole
                | Tool::StitchAngle
                | Tool::DirectionLine
                | Tool::MirrorLine
                | Tool::RefPoints
                | Tool::PolySelect
                | Tool::LineSelect
                | Tool::CropRect
                | Tool::CropEllipse
                | Tool::CropPolygon
                | Tool::ClickStitch
        )
    }

    /// Tools that work on the backdrop picture (it must be shown).
    pub fn uses_backdrop(self) -> bool {
        matches!(
            self,
            Tool::CropRect | Tool::CropEllipse | Tool::CropPolygon | Tool::ClickStitch
        )
    }

    pub fn is_box(self) -> bool {
        matches!(
            self,
            Tool::Rectangle | Tool::RoundRect | Tool::Ellipse | Tool::Star | Tool::Ring
        )
    }

    pub fn is_freehand(self) -> bool {
        matches!(self, Tool::Freehand | Tool::FreeShape | Tool::Lasso)
    }

    /// The outline of the current part is closed (clicking the first point closes it).
    fn closes(self, stages: usize) -> bool {
        matches!(
            self,
            Tool::Shape
                | Tool::StoneFill
                | Tool::AddHole
                | Tool::PolySelect
                | Tool::ComplexFill
                | Tool::Applique
                | Tool::CropPolygon
        ) || (self == Tool::Turning && stages == 0)
    }

    /// Tools that finish by themselves after this many points.
    fn fixed_points(self) -> Option<usize> {
        match self {
            Tool::ClickStitch => Some(1),
            Tool::Knife | Tool::StitchAngle | Tool::MirrorLine | Tool::CropRect | Tool::CropEllipse => Some(2),
            Tool::RefPoints => Some(4),
            _ => None,
        }
    }

    pub fn makes_running(self) -> bool {
        matches!(self, Tool::Line | Tool::Manual | Tool::Freehand)
    }

    pub fn makes_satin(self) -> bool {
        matches!(
            self,
            Tool::Column | Tool::ColumnB | Tool::ColumnC | Tool::Ring | Tool::Turning
        )
    }
}

/// Run type and pass of running stitches, as in the property bar and the properties.
pub fn run_type_combo(ui: &mut egui::Ui, p: &mut RunningParams) -> bool {
    let mut changed = false;
    let name = |s: RunStyle| match s {
        RunStyle::Run => tr("Run"),
        RunStyle::Backstitch => tr("Backstitch"),
        RunStyle::Stemstitch => tr("Stem stitch"),
        RunStyle::Manual => tr("Manual"),
        RunStyle::Sculpture => tr("Sculpture run"),
    };
    egui::ComboBox::from_id_salt(ui.id().with("run_style"))
        .selected_text(name(p.style))
        .show_ui(ui, |ui| {
            for s in [
                RunStyle::Run,
                RunStyle::Backstitch,
                RunStyle::Stemstitch,
                RunStyle::Sculpture,
                RunStyle::Manual,
            ] {
                changed |= ui.selectable_value(&mut p.style, s, name(s)).changed();
            }
        });
    let mut triple = p.repeats >= 3;
    if ui
        .checkbox(&mut triple, tr("Triple"))
        .on_hover_text(tr("Bean stitch: each stitch three times"))
        .changed()
    {
        p.repeats = if triple { 3 } else { 1 };
        changed = true;
    }
    let pass_name = |s: RunPass| match s {
        RunPass::Once => tr("Once"),
        RunPass::Backtrack => tr("Backtrack"),
        RunPass::Repeat => tr("Repeat"),
        RunPass::Redwork => tr("Redwork"),
        RunPass::Branch => tr("Branching"),
    };
    egui::ComboBox::from_id_salt(ui.id().with("run_pass"))
        .selected_text(pass_name(p.pass))
        .show_ui(ui, |ui| {
            for s in [
                RunPass::Once,
                RunPass::Backtrack,
                RunPass::Repeat,
                RunPass::Redwork,
                RunPass::Branch,
            ] {
                changed |= ui.selectable_value(&mut p.pass, s, pass_name(s)).changed();
            }
        })
        .response
        .on_hover_text(tr(
            "Backtrack: forward and back. Repeat: two times. Redwork and branching sew all \
             lines of the object as one network.",
        ));
    changed
}

/// The object being drawn. Points are in design coordinates.
#[derive(Debug, Clone, Default)]
pub struct DrawState {
    /// (point, is curve point) of the current part.
    pub points: Vec<(Point, bool)>,
    /// Finished parts of a tool with parts (Complex fill, Column B, Turning).
    pub stages: Vec<Vec<(Point, bool)>>,
    /// Box tools: the drag start.
    pub box_start: Option<Point>,
    /// Freehand tools: the points of the drag.
    pub freehand: Vec<Point>,
    /// Line tool: add the new points to the end of this object ("continue digitizing").
    pub continue_from: Option<ObjectId>,
}

/// The thread for new objects: the current one, else the first, else a new one.
pub fn thread_for_new(d: &mut Design, current: Option<ThreadId>) -> ThreadId {
    if let Some(t) = current.filter(|t| d.thread(*t).is_some()) {
        return t;
    }
    if let Some(t) = d.threads.first() {
        return t.id;
    }
    d.add_thread("Thread 1", FIRST_THREAD)
}

/// Makes the object from the points with the default settings, or `None` if there are too few.
#[cfg(test)]
pub fn build(tool: Tool, points: &[(Point, bool)]) -> Option<ObjectKind> {
    build_with(tool, points, &[], &CurrentProperties::default(), &Defaults::default())
}

/// Makes the object from the points of the current part and the finished parts.
pub fn build_with(
    tool: Tool,
    points: &[(Point, bool)],
    stages: &[Vec<(Point, bool)>],
    p: &CurrentProperties,
    d: &Defaults,
) -> Option<ObjectKind> {
    let corners = |pts: &[(Point, bool)]| -> Vec<(Point, bool)> { pts.iter().map(|(p, _)| (*p, false)).collect() };
    match tool {
        Tool::Line if points.len() >= 2 => Some(ObjectKind::Running {
            path: Path::through_points(points, false),
            params: p.running.clone(),
        }),
        Tool::Manual if points.len() >= 2 => Some(ObjectKind::Running {
            path: Path::through_points(&corners(points), false),
            params: RunningParams {
                style: RunStyle::Manual,
                ..p.running.clone()
            },
        }),
        Tool::StoneRun if points.len() >= 2 => Some(ObjectKind::Bling(Box::new(emb_model::Bling {
            gap_mm: d.stone_gap_mm,
            ..emb_model::Bling::new(
                d.stone.clone(),
                emb_model::BlingLayout::Run {
                    path: Path::through_points(points, false),
                },
            )
        }))),
        Tool::StoneFill if points.len() >= 3 => Some(ObjectKind::Bling(Box::new(emb_model::Bling {
            gap_mm: d.stone_gap_mm,
            ..emb_model::Bling::new(
                d.stone.clone(),
                emb_model::BlingLayout::Fill {
                    outer: Path::through_points(points, true),
                    holes: Vec::new(),
                    fill: d.stone_fill,
                },
            )
        }))),
        Tool::Shape if points.len() >= 3 => Some(ObjectKind::Fill {
            outer: Path::through_points(points, true),
            holes: vec![],
            params: p.fill.clone(),
        }),
        Tool::ComplexFill => {
            let mut parts: Vec<&[(Point, bool)]> = stages.iter().map(Vec::as_slice).collect();
            if points.len() >= 3 {
                parts.push(points);
            }
            let (outer, holes) = parts.split_first()?;
            (outer.len() >= 3).then(|| ObjectKind::Fill {
                outer: Path::through_points(outer, true),
                holes: holes
                    .iter()
                    .filter(|h| h.len() >= 3)
                    .map(|h| Path::through_points(h, true))
                    .collect(),
                params: p.fill.clone(),
            })
        }
        Tool::Applique => {
            let mut parts: Vec<&[(Point, bool)]> = stages.iter().map(Vec::as_slice).collect();
            if points.len() >= 3 {
                parts.push(points);
            }
            let (outer, holes) = parts.split_first()?;
            (outer.len() >= 3).then(|| ObjectKind::Applique {
                pieces: vec![AppliquePiece::new(
                    Path::through_points(outer, true),
                    holes
                        .iter()
                        .filter(|h| h.len() >= 3)
                        .map(|h| Path::through_points(h, true))
                        .collect(),
                )],
                params: p.applique.clone(),
            })
        }
        Tool::ColumnB if points.len() >= 4 => {
            let pairs = points.len() / 2;
            let a: Vec<(Point, bool)> = (0..pairs).map(|i| points[2 * i]).collect();
            let b: Vec<(Point, bool)> = (0..pairs).map(|i| points[2 * i + 1]).collect();
            Some(ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::through_points(&a, false),
                    rail_b: Path::through_points(&b, false),
                    rungs: (0..pairs).map(|i| (a[i].0, b[i].0)).collect(),
                },
                params: p.satin.clone(),
            })
        }
        Tool::Column => {
            let a = stages.first()?;
            let b = stages.get(1).map(Vec::as_slice).unwrap_or(points);
            (a.len() >= 2 && b.len() >= 2).then(|| ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::through_points(a, false),
                    rail_b: Path::through_points(b, false),
                    rungs: vec![],
                },
                params: p.satin.clone(),
            })
        }
        Tool::ColumnC if points.len() >= 2 => Some(ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&Path::through_points(points, false), d.column_width_mm),
            params: p.satin.clone(),
        }),
        Tool::Turning => {
            let outline = stages.first()?;
            let lines: Vec<(Point, Point)> = points.as_chunks::<2>().0.iter().map(|c| (c[0].0, c[1].0)).collect();
            if outline.len() < 3 {
                return None;
            }
            let geometry = SatinGeometry::from_outline(&Path::through_points(outline, true), &lines)?;
            Some(ObjectKind::Satin {
                geometry,
                params: SatinParams {
                    spacing_mm: p.fill.spacing_mm * 2.0,
                    ..SatinParams::turning()
                },
            })
        }
        _ => None,
    }
}

/// The box from a drag, with the Ctrl (equal sides) and Alt (from center) options.
pub fn drag_box(start: Point, end: Point, mods: Modifiers) -> BBox {
    let mut d = end - start;
    if mods.command {
        let s = d.x.abs().max(d.y.abs());
        d = Point::new(s * d.x.signum(), s * d.y.signum());
    }
    if mods.alt {
        BBox::from_points([start - d, start + d]).unwrap_or(BBox::new(start, start))
    } else {
        BBox::from_points([start, start + d]).unwrap_or(BBox::new(start, start))
    }
}

#[cfg(test)]
pub fn build_box(tool: Tool, b: BBox) -> Option<ObjectKind> {
    build_box_with(tool, b, &CurrentProperties::default(), &Defaults::default())
}

/// A star with `n` points inside the box.
pub fn star(b: BBox, n: u32) -> Path {
    let n = n.max(3);
    let c = b.center();
    let (rx, ry) = (b.width() / 2.0, b.height() / 2.0);
    let pts: Vec<Point> = (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { STAR_INNER };
            let (s, co) = math::sin_cos_deg(-90.0 + 180.0 * f64::from(i) / f64::from(n));
            Point::new(c.x + rx * r * co, c.y + ry * r * s)
        })
        .collect();
    Path::polygon(&pts)
}

pub fn build_box_with(tool: Tool, b: BBox, p: &CurrentProperties, d: &Defaults) -> Option<ObjectKind> {
    if b.width() < 0.5 || b.height() < 0.5 {
        return None;
    }
    let outer = match tool {
        Tool::Rectangle => Path::rect(b),
        Tool::RoundRect => Path::rounded_rect(b, d.corner_radius_mm),
        Tool::Ellipse => Path::ellipse(b.center(), b.width() / 2.0, b.height() / 2.0),
        Tool::Star => star(b, d.star_points),
        Tool::Ring => {
            let w = d.column_width_mm;
            let (rx, ry) = (b.width() / 2.0, b.height() / 2.0);
            if rx <= w || ry <= w {
                return None;
            }
            return Some(ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::ellipse(b.center(), rx, ry),
                    rail_b: Path::ellipse(b.center(), rx - w, ry - w),
                    rungs: vec![],
                },
                params: p.satin.clone(),
            });
        }
        _ => return None,
    };
    Some(ObjectKind::Fill {
        outer,
        holes: vec![],
        params: p.fill.clone(),
    })
}

/// A freehand line: simplified, with smooth points (the ends are corners).
fn freehand_points(pts: &[Point], closed: bool) -> Vec<(Point, bool)> {
    let mut s = emb_geom::path::simplify(pts, FREEHAND_TOL_MM);
    if closed && s.len() > 3 && s[0].distance(s[s.len() - 1]) < 1e-9 {
        s.pop();
    }
    let n = s.len();
    s.into_iter()
        .enumerate()
        .map(|(i, p)| (p, closed || (i > 0 && i + 1 < n)))
        .collect()
}

impl AppState {
    pub fn set_tool(&mut self, tool: Tool) {
        // A tool other than Select ends hooping mode.
        if tool != Tool::Select {
            self.hoopings_ui.mode = false;
            self.hoopings_ui.drawing = None;
        }
        self.tool = tool;
        self.reflect_copy = false;
        self.draw = DrawState::default();
        self.reshape = Default::default();
        self.measure = None;
        // Tools that work on the selection keep it; draw tools start with no selection.
        let keeps = matches!(
            tool,
            Tool::Select
                | Tool::Reshape
                | Tool::Knife
                | Tool::AddHole
                | Tool::StitchAngle
                | Tool::DirectionLine
                | Tool::MirrorLine
                | Tool::RefPoints
                | Tool::Measure
                | Tool::Zoom
                | Tool::CropRect
                | Tool::CropEllipse
                | Tool::CropPolygon
                | Tool::Lasso
                | Tool::PolySelect
                | Tool::LineSelect
                | Tool::CrossStitch
                | Tool::CrossErase
                | Tool::Backstitch
                | Tool::Stone
        );
        if !keeps {
            self.selection.clear();
        }
    }

    /// Starts the Line tool so that new points continue the selected line.
    pub fn continue_line(&mut self) {
        let Some(id) = self.selection.single() else { return };
        if !matches!(
            self.doc.design().object(id).map(|o| &o.kind),
            Some(ObjectKind::Running { .. })
        ) {
            return;
        }
        self.set_tool(Tool::Line);
        self.selection.set(id);
        self.draw.continue_from = Some(id);
        self.message =
            "Click points to continue the line from its end (or from its start: click near the start first). Enter: finish."
                .into();
    }

    /// Ends one part of a tool with parts, or finishes. Returns `true` if something happened.
    pub fn finish_drawing(&mut self) -> bool {
        let pts = self.draw.points.clone();
        match self.tool {
            Tool::ComplexFill | Tool::Applique | Tool::Column | Tool::Turning => {
                let stage_done = match self.tool {
                    Tool::ComplexFill | Tool::Applique => pts.len() >= 3,
                    Tool::Column => pts.len() >= 2 && self.draw.stages.is_empty(),
                    _ => pts.len() >= 3 && self.draw.stages.is_empty(),
                };
                if stage_done {
                    self.draw.stages.push(pts);
                    self.draw.points.clear();
                    // A column finishes after side B.
                    if !(self.tool == Tool::Column && self.draw.stages.len() == 2) {
                        return true;
                    }
                }
            }
            Tool::Knife => return self.finish_action(|st, p| st.knife(p[0], p[1])),
            Tool::StitchAngle => return self.finish_action(|st, p| st.stitch_angle(p[0], p[1])),
            Tool::DirectionLine => return self.finish_action(|st, p| st.direction_line(p)),
            Tool::MirrorLine => return self.finish_action(|st, p| st.mirror_about_line(p[0], p[1])),
            t @ (Tool::CropRect | Tool::CropEllipse | Tool::CropPolygon) => {
                return self.finish_action(|st, p| st.crop_backdrop(t, p));
            }
            Tool::ClickStitch => return self.finish_action(|st, p| st.click_stitch(p[0])),
            Tool::RefPoints => return self.finish_action(|st, p| st.transform_by_points(p[0], p[1], p[2], p[3])),
            Tool::AddHole => return self.finish_action(|st, p| st.add_hole(p)),
            Tool::PolySelect => return self.finish_action(|st, p| st.select_inside(p)),
            Tool::LineSelect => return self.finish_action(|st, p| st.select_crossing(p)),
            // The lines are in the design already; Enter ends the line.
            Tool::Backstitch => {
                self.draw = DrawState::default();
                return true;
            }
            Tool::CrossStitch | Tool::CrossErase => return false,
            _ => {}
        }
        if self.tool == Tool::Line
            && let Some(id) = self.draw.continue_from
        {
            return self.append_to_line(id);
        }
        let stages = self.draw.stages.clone();
        let Some(kind) = build_with(
            self.tool,
            &self.draw.points,
            &stages,
            &self.doc.design().current,
            &self.settings.defaults,
        ) else {
            self.message = tr("Not enough points for this object").into();
            return false;
        };
        self.add_drawn(kind);
        true
    }

    /// Runs an edit tool action with the points, then clears them.
    fn finish_action(&mut self, f: impl FnOnce(&mut AppState, &[Point])) -> bool {
        let pts: Vec<Point> = self.draw.points.iter().map(|(p, _)| *p).collect();
        let need = self
            .tool
            .fixed_points()
            .unwrap_or(if matches!(self.tool, Tool::LineSelect | Tool::DirectionLine) {
                2
            } else {
                3
            });
        if pts.len() < need {
            self.message = tr("Not enough points").into();
            return false;
        }
        self.draw = DrawState::default();
        f(self, &pts);
        true
    }

    /// Adds the drawn points to the end of a running line, or before its
    /// start when the first new point is nearer to the start.
    fn append_to_line(&mut self, id: ObjectId) -> bool {
        let Some(ObjectKind::Running { path, .. }) = self.doc.design().object(id).map(|o| o.kind.clone()) else {
            return false;
        };
        let nodes = emb_geom::nodes::NodePath::from_path(&path);
        let (Some(start), Some(end)) = (
            nodes.subs.first().and_then(|s| s.nodes.first().copied()),
            nodes.subs.last().and_then(|s| s.nodes.last().copied()),
        ) else {
            return false;
        };
        let Some(first) = self.draw.points.first().map(|(p, _)| *p) else {
            return false;
        };
        let at_start = first.distance(start) < first.distance(end);
        if at_start {
            // The new points, last one first, then the old start.
            let mut pts: Vec<(Point, bool)> = self.draw.points.iter().rev().copied().collect();
            pts.push((start, false));
            let extra = Path::through_points(&pts, false);
            self.doc.edit("Continue Line", |d| {
                if let Some(ObjectKind::Running { path, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    let mut cmds = extra.cmds;
                    cmds.extend(path.cmds.drain(..).skip(1));
                    path.cmds = cmds;
                }
            });
        } else {
            let mut pts = vec![(end, false)];
            pts.extend(self.draw.points.iter().copied());
            let extra = Path::through_points(&pts, false);
            self.doc.edit("Continue Line", |d| {
                if let Some(ObjectKind::Running { path, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    path.cmds.extend(extra.cmds.into_iter().skip(1));
                }
            });
        }
        self.draw = DrawState::default();
        self.selection.set(id);
        self.message = tr("Line continued.").into();
        true
    }

    pub(crate) fn add_drawn(&mut self, kind: ObjectKind) {
        let label = format!("Draw {}", kind.type_name().to_lowercase());
        let current = self.current_thread;
        let as_vector = self.draw_vectors;
        let id = self.doc.edit(label, |d| {
            let t = thread_for_new(d, current);
            // Draw artwork: the outline of the shape, in the thread color.
            let kind = match as_vector {
                true => {
                    let color = d.thread(t).map_or(emb_model::Rgb(0, 0, 0), |th| th.color);
                    emb_model::vector::to_vector(&kind, color).unwrap_or(kind)
                }
                false => kind,
            };
            d.add_object(t, kind)
        });
        self.current_thread = self.doc.design().object(id).map(|o| o.thread);
        self.selection.set(id);
        self.draw = DrawState::default();
        self.message = crate::i18n::fill("{} added. Draw the next one, or press Esc.", &[&(self.tool.label())]);
    }

    /// Enter, Backspace and Esc while a tool other than Select is active.
    /// Returns `true` if the key was used.
    pub fn draw_key(&mut self, key: egui::Key) -> bool {
        if self.tool == Tool::Select {
            return false;
        }
        if self.tool == Tool::Reshape {
            return match key {
                egui::Key::Delete | egui::Key::Backspace => self.delete_node(),
                egui::Key::Escape if self.reshape.node.is_some() => {
                    self.reshape.node = None;
                    true
                }
                egui::Key::Escape => {
                    self.set_tool(Tool::Select);
                    true
                }
                _ => false,
            };
        }
        match key {
            egui::Key::Enter => {
                self.finish_drawing();
                true
            }
            egui::Key::Backspace => {
                if self.draw.points.pop().is_none()
                    && let Some(last) = self.draw.stages.pop()
                {
                    // Back into the last finished part.
                    self.draw.points = last;
                }
                true
            }
            egui::Key::Escape => {
                let idle = self.draw.points.is_empty()
                    && self.draw.stages.is_empty()
                    && self.draw.box_start.is_none()
                    && self.measure.is_none();
                if idle {
                    self.set_tool(Tool::Select);
                } else {
                    let keep = self.draw.continue_from;
                    self.draw = DrawState::default();
                    self.measure = None;
                    self.draw.continue_from = keep;
                }
                true
            }
            _ => false,
        }
    }
}

/// Handles canvas input for a draw or edit tool.
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    let mods = ui.input(|i| i.modifiers);
    match st.tool {
        Tool::Measure => return crate::workspace::measure_input(response, st, map),
        Tool::Zoom => return crate::workspace::zoom_input(ui, response, st),
        Tool::Lettering => return crate::lettering::tool_input(response, st, map),
        Tool::CrossStitch | Tool::CrossErase | Tool::Backstitch => {
            return crate::cross::tool_input(ui, response, st, map);
        }
        Tool::Stone => return crate::bling::tool_input(response, st, map),
        _ => {}
    }
    if st.tool.uses_points() {
        let left = response.clicked();
        let right = response.secondary_clicked();
        if (left || right)
            && let Some(pos) = response.interact_pointer_pos()
        {
            // A closed outline: a click on its first point closes it.
            if st.tool.closes(st.draw.stages.len())
                && st.draw.points.len() >= 3
                && map.to_screen(st.draw.points[0].0).distance(pos) <= CLOSE_SNAP
            {
                st.finish_drawing();
                return;
            }
            let p = st.snap(map, map.to_design(pos));
            // Ignore a second click on the same place (for example a double click).
            if st.draw.points.last().is_none_or(|(q, _)| q.distance(p) > 1e-6) {
                st.draw.points.push((p, right));
            }
            if st.tool.fixed_points() == Some(st.draw.points.len()) {
                st.finish_drawing();
            }
        }
        return;
    }
    if st.tool.is_freehand() {
        if response.drag_started_by(egui::PointerButton::Primary) {
            st.draw.freehand.clear();
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
            && st
                .draw
                .freehand
                .last()
                .is_none_or(|q| map.to_screen(*q).distance(pos) >= FREEHAND_STEP)
        {
            st.draw.freehand.push(map.to_design(pos));
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            let pts = std::mem::take(&mut st.draw.freehand);
            finish_freehand(st, &pts);
        }
        return;
    }
    // Box tools.
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        st.draw.box_start = Some(st.snap(map, map.to_design(origin)));
    }
    if response.drag_stopped_by(egui::PointerButton::Primary)
        && let (Some(start), Some(pos)) = (st.draw.box_start.take(), ui.input(|i| i.pointer.latest_pos()))
    {
        let b = drag_box(start, st.snap(map, map.to_design(pos)), mods);
        if let Some(kind) = build_box_with(st.tool, b, &st.doc.design().current, &st.settings.defaults) {
            st.add_drawn(kind);
        }
    }
}

/// Makes the object (or the lasso selection) from a freehand drag.
fn finish_freehand(st: &mut AppState, pts: &[Point]) {
    match st.tool {
        Tool::Lasso if pts.len() >= 3 => st.select_inside(pts),
        Tool::Freehand if pts.len() >= 2 => {
            let p = freehand_points(pts, false);
            if let Some(kind) = build_with(Tool::Line, &p, &[], &st.doc.design().current, &st.settings.defaults) {
                st.add_drawn(kind);
            }
        }
        Tool::FreeShape if pts.len() >= 3 => {
            let p = freehand_points(pts, true);
            if let Some(kind) = build_with(Tool::Shape, &p, &[], &st.doc.design().current, &st.settings.defaults) {
                st.add_drawn(kind);
            }
        }
        _ => {}
    }
}

/// Draws the object being made, with a line to the pointer.
pub fn draw_preview(painter: &egui::Painter, ui: &egui::Ui, st: &AppState, map: &ViewMap, color: Color32) {
    if matches!(st.tool, Tool::CrossStitch | Tool::CrossErase | Tool::Backstitch) {
        return crate::cross::draw_overlay(painter, ui, st, map, color);
    }
    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|p| map.rect.contains(*p));
    let mods = ui.input(|i| i.modifiers);
    let stroke = Stroke::new(1.5, color);
    let screen = |path: &Path| -> Vec<(Vec<Pos2>, bool)> {
        path.flatten(0.5 / map.view.scale())
            .into_iter()
            .map(|pl| (pl.points.iter().map(|p| map.to_screen(*p)).collect(), pl.closed))
            .collect()
    };
    let paint = |kind: &ObjectKind, stroke: Stroke| {
        for path in kind.paths() {
            for (pts, closed) in screen(path) {
                painter.add(if closed {
                    Shape::closed_line(pts, stroke)
                } else {
                    Shape::line(pts, stroke)
                });
            }
        }
    };
    let polyline = |pts: &[(Point, bool)], closed: bool, stroke: Stroke| {
        let mut line: Vec<Pos2> = pts.iter().map(|(p, _)| map.to_screen(*p)).collect();
        if closed && line.len() > 2 {
            line.push(line[0]);
        }
        if line.len() >= 2 {
            painter.add(Shape::line(line, stroke));
        }
    };

    if st.tool.is_freehand() {
        let line: Vec<Pos2> = st.draw.freehand.iter().map(|p| map.to_screen(*p)).collect();
        if line.len() >= 2 {
            painter.add(Shape::line(line, stroke));
        }
        return;
    }

    if st.tool.uses_points() {
        // Finished parts of a tool with parts.
        for (i, part) in st.draw.stages.iter().enumerate() {
            let closed = matches!(st.tool, Tool::ComplexFill | Tool::Applique) || (st.tool == Tool::Turning && i == 0);
            let path = Path::through_points(part, closed);
            for (pts, closed) in screen(&path) {
                painter.add(if closed {
                    Shape::closed_line(pts, stroke)
                } else {
                    Shape::line(pts, stroke)
                });
            }
        }
        // The object as it would be with the pointer as the next corner.
        let mut pts = st.draw.points.clone();
        if let Some(h) = hover {
            pts.push((st.snap(map, map.to_design(h)), false));
        }
        let object_tool = st.tool.makes_running() || st.tool.makes_satin() || matches!(st.tool, Tool::Shape);
        let built = if object_tool && st.draw.continue_from.is_none() {
            build_with(
                st.tool,
                &pts,
                &st.draw.stages,
                &st.doc.design().current,
                &st.settings.defaults,
            )
        } else {
            None
        };
        match &built {
            Some(kind) if st.tool != Tool::Turning => paint(kind, stroke),
            _ => polyline(&pts, false, stroke),
        }
        // Column: while side B is drawn, the satin fills the space between
        // the two sides.
        if st.tool == Tool::Column
            && let Some(ObjectKind::Satin { geometry, .. }) = &built
        {
            let line = |path: &Path| -> Vec<Pos2> {
                path.flatten(0.5 / map.view.scale())
                    .into_iter()
                    .flat_map(|pl| pl.points)
                    .map(|p| map.to_screen(p))
                    .collect()
            };
            let a = line(&geometry.rail_a);
            let mut b = line(&geometry.rail_b);
            if a.len() >= 2 && b.len() >= 2 {
                // Side B can go the other way.
                let (a0, a1, b0, b1) = (a[0], a[a.len() - 1], b[0], b[b.len() - 1]);
                if a0.distance(b0) + a1.distance(b1) > a0.distance(b1) + a1.distance(b0) {
                    b.reverse();
                }
                let mut ring = a;
                ring.extend(b.into_iter().rev());
                painter.add(crate::applique::scanline_mesh(
                    &[ring],
                    color.gamma_multiply(0.25),
                    map.rect,
                ));
            }
        }
        if let Some(ObjectKind::Satin { geometry, .. }) = &built {
            for (a, b) in &geometry.rungs {
                painter.line_segment(
                    [map.to_screen(*a), map.to_screen(*b)],
                    Stroke::new(1.0, color.gamma_multiply(0.6)),
                );
            }
        }
        // Turning: angle lines of the current part.
        if st.tool == Tool::Turning && !st.draw.stages.is_empty() {
            for c in pts.as_chunks::<2>().0 {
                painter.line_segment([map.to_screen(c[0].0), map.to_screen(c[1].0)], stroke);
            }
        }
        // Placed points: squares for corners, circles for curve points.
        for (i, (p, curve)) in st.draw.points.iter().enumerate() {
            let s = map.to_screen(*p);
            let fill = if i == 0 { color } else { Color32::WHITE };
            if *curve {
                painter.circle_filled(s, 4.0, fill);
                painter.circle_stroke(s, 4.0, Stroke::new(1.5, color));
            } else {
                let r = Rect::from_center_size(s, Vec2::splat(7.0));
                painter.rect_filled(r, 0.0, fill);
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, color), egui::StrokeKind::Inside);
            }
        }
        // Close hint on the first point of a closed outline.
        if st.tool.closes(st.draw.stages.len())
            && st.draw.points.len() >= 3
            && let Some(h) = hover
        {
            let first = map.to_screen(st.draw.points[0].0);
            if first.distance(h) <= CLOSE_SNAP {
                painter.circle_stroke(first, CLOSE_SNAP, Stroke::new(1.5, color));
            }
        }
        return;
    }

    if let (Some(start), Some(h)) = (st.draw.box_start, hover) {
        let b = drag_box(start, st.snap(map, map.to_design(h)), mods);
        if let Some(kind) = build_box_with(st.tool, b, &st.doc.design().current, &st.settings.defaults) {
            paint(&kind, stroke);
        }
        let u = st.settings.units;
        let label = format!("{} × {}", u.fmt(b.width()), u.fmt(b.height()));
        painter.text(
            h + Vec2::new(12.0, 12.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(12.0),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, curve: bool) -> (Point, bool) {
        (Point::new(x, y), curve)
    }

    #[test]
    fn column_c_makes_a_satin_of_fixed_width() {
        assert!(build(Tool::ColumnC, &[p(0.0, 0.0, false)]).is_none());
        let Some(ObjectKind::Satin { geometry, .. }) = build(Tool::ColumnC, &[p(0.0, 0.0, false), p(20.0, 0.0, false)])
        else {
            panic!("a satin")
        };
        let a = geometry.rail_a.control_bbox().unwrap();
        let b = geometry.rail_b.control_bbox().unwrap();
        assert!((b.min.y - a.min.y - Defaults::default().column_width_mm).abs() < 1e-9);
    }

    #[test]
    fn build_needs_enough_points() {
        assert!(build(Tool::Line, &[p(0.0, 0.0, false)]).is_none());
        assert!(build(Tool::Line, &[p(0.0, 0.0, false), p(1.0, 0.0, false)]).is_some());
        assert!(build(Tool::Shape, &[p(0.0, 0.0, false), p(1.0, 0.0, false)]).is_none());
        assert!(
            build(
                Tool::ColumnB,
                &[p(0.0, 0.0, false), p(3.0, 0.0, false), p(0.0, 5.0, false)]
            )
            .is_none()
        );
    }

    #[test]
    fn column_pairs_make_rails_and_rungs() {
        // A then B, three times. The fifth click (odd) is ignored.
        let pts = [
            p(0.0, 0.0, false),
            p(3.0, 0.0, false),
            p(0.0, 10.0, true),
            p(3.0, 10.0, true),
            p(0.0, 20.0, false),
            p(3.0, 20.0, false),
            p(9.0, 9.0, false),
        ];
        let Some(ObjectKind::Satin { geometry, .. }) = build(Tool::ColumnB, &pts) else {
            panic!()
        };
        assert_eq!(geometry.rungs.len(), 3);
        assert_eq!(geometry.rungs[1], (Point::new(0.0, 10.0), Point::new(3.0, 10.0)));
        let a = geometry.rail_a.control_bbox().unwrap();
        assert_eq!((a.min.x, a.max.y), (0.0, 20.0));
    }

    #[test]
    fn drag_box_modifiers() {
        let s = Point::new(10.0, 10.0);
        let e = Point::new(20.0, 14.0);
        let b = drag_box(s, e, Modifiers::NONE);
        assert_eq!((b.width(), b.height()), (10.0, 4.0));
        let sq = drag_box(s, e, Modifiers::COMMAND);
        assert_eq!((sq.width(), sq.height()), (10.0, 10.0));
        let c = drag_box(s, e, Modifiers::ALT);
        assert_eq!(c.center(), s);
        assert_eq!(c.width(), 20.0);
        assert!(build_box(Tool::Ellipse, BBox::new(s, s)).is_none(), "too small");
    }

    #[test]
    fn first_object_creates_a_thread() {
        let mut d = Design::new("t");
        let t = thread_for_new(&mut d, None);
        assert_eq!(d.threads.len(), 1);
        assert_eq!(thread_for_new(&mut d, Some(ThreadId(99))), t);
    }

    // ---- Headless canvas tests with real pointer and key events ----

    use eframe::egui::{Key, PointerButton};

    use crate::test_util::Harness;

    /// Screen position of a design point relative to the hoop center.
    fn at(h: &Harness, x: f64, y: f64) -> Pos2 {
        let c = h.st.doc.design().hoop.center;
        h.screen(c + Point::new(x, y))
    }

    #[test]
    fn line_tool_corner_and_curve_points_then_enter() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Line);
        h.click(PointerButton::Primary, at(&h, -20.0, 0.0));
        h.click(PointerButton::Secondary, at(&h, 0.0, -15.0));
        h.click(PointerButton::Primary, at(&h, 20.0, 0.0));
        assert_eq!(h.st.draw.points.len(), 3);
        assert!(h.st.draw.points[1].1, "right click is a curve point");
        h.key(Key::Enter);

        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let ObjectKind::Running { path, .. } = &d.objects()[0].kind else {
            panic!()
        };
        assert!(path.cmds.iter().any(|c| matches!(c, emb_geom::PathCmd::CubicTo(..))));
        assert_eq!(d.threads.len(), 1, "the first object makes a thread");
        assert_eq!(h.st.selection.single(), Some(d.objects()[0].id));
        assert_eq!(h.st.tool, Tool::Line, "the tool stays active");
        assert!(h.st.draw.points.is_empty());
        assert_eq!(h.st.doc.undo_label(), Some("Draw running"));
    }

    #[test]
    fn shape_closes_on_first_point() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Shape);
        for (x, y) in [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.click(PointerButton::Primary, at(&h, -10.0, -10.0));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let b = d.bbox_of(&[d.objects()[0].id]).unwrap();
        assert!(
            (b.width() - 20.0).abs() < 0.5 && (b.height() - 20.0).abs() < 0.5,
            "{b:?}"
        );
        assert!(matches!(d.objects()[0].kind, ObjectKind::Fill { .. }));
    }

    #[test]
    fn column_pairs_tool_makes_satin_from_pairs() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::ColumnB);
        for (x, y) in [(-2.0, -20.0), (2.0, -20.0), (-2.0, 20.0), (2.0, 20.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.key(Key::Enter);
        let d = h.st.doc.design();
        let ObjectKind::Satin { geometry, .. } = &d.objects()[0].kind else {
            panic!()
        };
        assert_eq!(geometry.rungs.len(), 2);
    }

    #[test]
    fn rectangle_drag_with_ctrl_is_square() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Rectangle);
        h.drag(
            PointerButton::Primary,
            at(&h, 0.0, 0.0),
            at(&h, 30.0, 10.0),
            Modifiers::COMMAND,
        );
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let b = d.bbox_of(&[d.objects()[0].id]).unwrap();
        assert!(
            (b.width() - 30.0).abs() < 0.5 && (b.height() - 30.0).abs() < 0.5,
            "{b:?}"
        );
    }

    #[test]
    fn backspace_and_escape() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Line);
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        h.click(PointerButton::Primary, at(&h, 10.0, 0.0));
        h.key(Key::Backspace);
        assert_eq!(h.st.draw.points.len(), 1);
        h.key(Key::Escape);
        assert!(h.st.draw.points.is_empty());
        assert_eq!(h.st.tool, Tool::Line);
        h.key(Key::Escape);
        assert_eq!(h.st.tool, Tool::Select);
        assert!(h.st.doc.design().objects().is_empty());
    }

    fn kinds(h: &Harness) -> Vec<&'static str> {
        h.st.doc.design().objects().iter().map(|o| o.type_name()).collect()
    }

    #[test]
    fn manual_tool_keeps_each_click_as_a_needle_point() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Manual);
        for (x, y) in [(0.0, 0.0), (8.0, 0.0), (8.0, 8.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.key(Key::Enter);
        let ObjectKind::Running { params, .. } = &h.st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(params.style, RunStyle::Manual);
        // 8 mm stitches stay 8 mm (no 2.5 mm running stitch).
        let plan = h.st.plan_view().plan.clone();
        let longest = plan
            .stitches
            .records
            .windows(2)
            .filter(|w| w[0].kind == emb_model::StitchKind::Stitch && w[1].kind == emb_model::StitchKind::Stitch)
            .map(|w| w[0].pos.distance(w[1].pos))
            .fold(0.0, f64::max);
        assert!((longest - 8.0).abs() < 0.01, "{longest}");
    }

    #[test]
    fn complex_fill_outline_then_hole() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::ComplexFill);
        for (x, y) in [(-20.0, -20.0), (20.0, -20.0), (20.0, 20.0), (-20.0, 20.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.key(Key::Enter);
        assert_eq!(h.st.draw.stages.len(), 1);
        for (x, y) in [(-5.0, -5.0), (5.0, -5.0), (0.0, 5.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.key(Key::Enter);
        h.key(Key::Enter);
        let ObjectKind::Fill { holes, .. } = &h.st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(holes.len(), 1);
    }

    #[test]
    fn column_tool_draws_one_side_then_the_other() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Column);
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        h.click(PointerButton::Primary, at(&h, 30.0, 0.0));
        h.key(Key::Enter);
        assert_eq!(h.st.draw.stages.len(), 1, "side A is done");
        assert!(kinds(&h).is_empty(), "no object after one side");
        h.click(PointerButton::Primary, at(&h, 0.0, 4.0));
        h.click(PointerButton::Primary, at(&h, 30.0, 4.0));
        h.key(Key::Enter);
        assert_eq!(kinds(&h), ["Satin"]);
        let d = h.st.doc.design();
        let ObjectKind::Satin { geometry, .. } = &d.objects()[0].kind else {
            panic!()
        };
        assert!(geometry.rungs.is_empty(), "two sides, no pairs");
    }

    #[test]
    fn turning_fill_from_outline_and_angle_lines() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Turning);
        for (x, y) in [(0.0, 0.0), (40.0, 0.0), (40.0, 12.0), (0.0, 12.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        assert_eq!(h.st.draw.stages.len(), 1, "clicking the first point closes the outline");
        for (x, y) in [(1.0, 0.0), (1.0, 12.0), (39.0, 0.0), (39.0, 12.0)] {
            h.click(PointerButton::Primary, at(&h, x, y));
        }
        h.key(Key::Enter);
        let ObjectKind::Satin { params, geometry } = &h.st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert!(params.split_mm < 5.0);
        assert_eq!(geometry.rungs.len(), 2);
        assert!(h.st.plan_view().plan.warnings.is_empty());
    }

    #[test]
    fn box_tools_ring_star_round_rect() {
        let mut h = Harness::empty();
        for tool in [Tool::Ring, Tool::Star, Tool::RoundRect] {
            h.st.set_tool(tool);
            h.drag(
                PointerButton::Primary,
                at(&h, -15.0, -15.0),
                at(&h, 15.0, 15.0),
                Modifiers::NONE,
            );
        }
        assert_eq!(kinds(&h), ["Satin", "Fill", "Fill"]);
        let ObjectKind::Fill { outer, .. } = &h.st.doc.design().objects()[1].kind else {
            panic!()
        };
        assert_eq!(
            outer.flatten(0.1)[0].points.len(),
            10,
            "a five-point star has ten corners"
        );
    }

    #[test]
    fn freehand_line_and_lasso() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Freehand);
        h.drag(
            PointerButton::Primary,
            at(&h, -20.0, 0.0),
            at(&h, 20.0, 10.0),
            Modifiers::NONE,
        );
        assert_eq!(kinds(&h), ["Running"]);
        let id = h.st.doc.design().objects()[0].id;
        h.st.selection.clear();
        // Lasso around the line: a freehand loop drawn with several drags is not
        // possible here, so use the polygon select with the same code path.
        h.st.select_inside(&[
            Point::new(-1000.0, -1000.0),
            Point::new(1000.0, -1000.0),
            Point::new(1000.0, 1000.0),
            Point::new(-1000.0, 1000.0),
        ]);
        assert_eq!(h.st.selection.ids(), &[id]);
    }

    #[test]
    fn continue_line_adds_points_to_the_end() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Line);
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        h.click(PointerButton::Primary, at(&h, 10.0, 0.0));
        h.key(Key::Enter);
        h.st.continue_line();
        h.click(PointerButton::Primary, at(&h, 10.0, 10.0));
        h.key(Key::Enter);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let b = d.bbox_of(&[d.objects()[0].id]).unwrap();
        assert!((b.height() - 10.0).abs() < 0.5, "{b:?}");
    }

    #[test]
    fn continue_line_from_its_start() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Line);
        h.click(PointerButton::Primary, at(&h, 0.0, 0.0));
        h.click(PointerButton::Primary, at(&h, 10.0, 0.0));
        h.key(Key::Enter);
        h.st.continue_line();
        h.click(PointerButton::Primary, at(&h, -1.0, 5.0));
        h.click(PointerButton::Primary, at(&h, -1.0, 10.0));
        h.key(Key::Enter);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let Some(ObjectKind::Running { path, .. }) = d.objects().first().map(|o| &o.kind) else {
            panic!("not a line");
        };
        let pts = path.flatten(0.1);
        let pts = &pts[0].points;
        // The line now starts at the last new point and ends where it ended.
        assert!(pts[0].distance(Point::new(-1.0, 10.0)) < 0.5, "{pts:?}");
        assert!(pts[pts.len() - 1].distance(Point::new(10.0, 0.0)) < 0.5, "{pts:?}");
    }

    #[test]
    fn knife_tool_finishes_after_two_points() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Rectangle);
        h.drag(
            PointerButton::Primary,
            at(&h, -10.0, -10.0),
            at(&h, 10.0, 10.0),
            Modifiers::NONE,
        );
        h.st.set_tool(Tool::Knife);
        assert_eq!(h.st.selection.len(), 1, "the knife keeps the selection");
        h.click(PointerButton::Primary, at(&h, 0.0, -20.0));
        h.click(PointerButton::Primary, at(&h, 0.0, 20.0));
        assert_eq!(kinds(&h), ["Fill", "Fill"]);
    }

    #[test]
    fn right_drag_pans_and_adds_no_point() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Shape);
        let before = h.st.view.center;
        h.drag(
            PointerButton::Secondary,
            at(&h, 0.0, 0.0),
            at(&h, 20.0, 0.0),
            Modifiers::NONE,
        );
        assert!(h.st.draw.points.is_empty());
        assert!(h.st.view.center.distance(before) > 1.0);
    }
}
