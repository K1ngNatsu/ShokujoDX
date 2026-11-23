//! Sketch mode (P14 Milestone 2, ADR 0069): Start Sketch makes a sketch
//! object (or opens the selected one) and shows the Sketch tab. The draw
//! tools add points, lines, arcs, circles, rectangles and polygons; the
//! constraint commands and the Dimension tool add constraints to the parts
//! that are selected or that the user clicks next. The solver
//! (`emb_sketch`) runs after each change and during a drag. The colors show
//! the state: free, fully constrained, or in a conflict. Finish Sketch goes
//! back to the other tabs.
//!
//! Each change is one undo step; a drag is one step.
//!
//! Milestone 3 (ADR 0070): the draw tools add constraints while the user
//! draws (`emb_sketch::infer`: coincident, on curve, horizontal, vertical,
//! tangent; Alt stops them), and the edit tools trim, extend, split,
//! fillet, chamfer, offset, mirror and copy in patterns
//! (`emb_sketch::edit`). Relax removes the constraints that conflict.
//!
//! The files: the types of the mode here; `state` (the start and the end
//! of the mode, `AppState::edit_sketch`, the tools), `edits` (the edit
//! tools and the changes of the selection), `constraints` (the constraint
//! commands and the dimensions), `embroidery` (regions and curves to
//! embroidery), `params` (the Parameters window), `make` (the geometry of
//! a draw tool), `input` (keys, picks, clicks and drags), `menu` (the
//! right-click menu), `labels` and `paint` (the drawing on the canvas) and
//! `panel` (the Properties panel). They see each other through
//! `use super::*`; an item that more than one file uses is `pub(super)`.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use emb_geom::{Point, math};
use emb_model::sketch::SketchObject;
use emb_model::sketch::{CurveTo, RegionTo};
use emb_model::{LinkSource, ObjectId, ObjectKind};
use emb_sketch::infer::{self, Hint, Snap};
use emb_sketch::{Analysis, Constraint, ConstraintId, CurveId, EditError, Geometry, PointId, Sketch, SketchRegion};

use crate::app::AppState;
use crate::i18n::{fill, tr};
use crate::manipulate::ViewMap;
use crate::ribbon::RibbonTab;

mod constraints;
mod edits;
mod embroidery;
mod input;
mod labels;
mod make;
mod menu;
mod paint;
mod panel;
mod params;
mod state;
#[cfg(test)]
mod tests;

pub use constraints::*;
pub use input::*;
use labels::*;
use make::*;
use menu::*;
pub use paint::*;
pub use panel::*;
pub use params::*;

/// A tool of sketch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchTool {
    /// Select and drag points, curves and constraints.
    Select,
    /// A chain of lines.
    Line,
    /// A smooth curve through the clicks (spline pieces with tangents,
    /// ADR 0079).
    Spline,
    Rectangle,
    /// A rectangle from its center and a corner.
    RectCenter,
    /// A rectangle at an angle: two corners of a side, then the height.
    Rect3,
    /// A slot (its kind is in the Properties panel).
    Slot,
    /// A rectangle at an angle from its center: the center, the middle of
    /// a side, then a corner.
    RectCenter3,
    /// A circle tangent to three lines.
    TangentCircle,
    /// The points in a box move; the curves that end at them stretch.
    Stretch,
    /// Click inside closed regions to pick them for embroidery (P14
    /// Milestone 4).
    Regions,
    Circle,
    /// An arc through three points: start, end, and a point on it.
    Arc3,
    /// An arc from its center, start and end.
    ArcCenter,
    /// An arc that goes on from the end of a line, tangent to it.
    ArcTangent,
    Polygon,
    Point,
    Dimension,
    /// A constraint that waits for the parts to click.
    Constrain(Kind),
    /// Click the piece of a curve to remove (between the curves that cut
    /// it).
    Trim,
    /// Click near the end of a line or an arc to extend it to the next
    /// curve.
    Extend,
    /// Click a line or an arc to cut it in two there.
    Split,
    /// Click the corner of two lines to round it.
    Fillet,
    /// Click the corner of two lines to cut it.
    Chamfer,
    /// Click a curve: its chain gets a copy at a distance.
    Offset,
    /// The selected curves wait for the line to mirror about.
    Mirror,
    /// The selected curves in rows and columns (the panel has the button).
    RectPattern,
    /// The selected curves round a center that the user clicks.
    CircularPattern,
    /// The selected parts: click a base point, then where it goes (or the
    /// values of the Properties panel).
    Move,
    /// As Move, with copies.
    Copy,
    /// The selected parts turn round the point that the user clicks.
    Rotate,
    /// The selected parts scale from the point that the user clicks.
    Scale,
}

impl SketchTool {
    /// The tools that change curves that the user clicks.
    fn is_edit(self) -> bool {
        matches!(
            self,
            SketchTool::Trim
                | SketchTool::Extend
                | SketchTool::Split
                | SketchTool::Fillet
                | SketchTool::Chamfer
                | SketchTool::Offset
                | SketchTool::Mirror
                | SketchTool::RectPattern
                | SketchTool::CircularPattern
                | SketchTool::Move
                | SketchTool::Copy
                | SketchTool::Rotate
                | SketchTool::Scale
                | SketchTool::Stretch
        )
    }

    /// The tools that move the selected parts.
    fn is_transform(self) -> bool {
        matches!(
            self,
            SketchTool::Move | SketchTool::Copy | SketchTool::Rotate | SketchTool::Scale
        )
    }
}

/// The kind of a slot (as in the other program).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// The two centers, then the width.
    CenterToCenter,
    /// The two ends, then the width.
    Overall,
    /// The middle, one center, then the width.
    CenterPoint,
    /// A curved slot through three points (the two centers and a point on
    /// the arc), then the width.
    Arc3,
    /// A curved slot from the center of its arc, the two centers, then the
    /// width.
    ArcCenter,
}

impl SlotKind {
    pub const ALL: [SlotKind; 5] = [
        SlotKind::CenterToCenter,
        SlotKind::Overall,
        SlotKind::CenterPoint,
        SlotKind::Arc3,
        SlotKind::ArcCenter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SlotKind::CenterToCenter => tr("Center to center"),
            SlotKind::Overall => tr("Overall"),
            SlotKind::CenterPoint => tr("Center point"),
            SlotKind::Arc3 => tr("Three-point arc"),
            SlotKind::ArcCenter => tr("Center point arc"),
        }
    }

    fn is_arc(self) -> bool {
        matches!(self, SlotKind::Arc3 | SlotKind::ArcCenter)
    }
}

/// A motion of the transform tools, in sketch coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    Shift(Point),
    /// Round a center, degrees counterclockwise on the screen (the y of
    /// the design goes down).
    Turn(Point, f64),
    /// From a base point, by a factor.
    Grow(Point, f64),
}

impl Motion {
    pub fn apply(self, p: Point) -> Point {
        match self {
            Motion::Shift(d) => p + d,
            Motion::Turn(c, deg) => {
                let (s, k) = math::sin_cos_deg(-deg);
                let v = p - c;
                c + Point::new(v.x * k - v.y * s, v.x * s + v.y * k)
            }
            Motion::Grow(c, f) => c + (p - c) * f,
        }
    }
}

/// The settings of the edit tools (in the Properties panel).
#[derive(Debug, Clone, PartialEq)]
pub struct EditValues {
    pub fillet_mm: f64,
    pub chamfer_mm: f64,
    /// Positive: outside of a closed shape, else on the left of the chain.
    pub offset_mm: f64,
    pub cols: u32,
    pub rows: u32,
    pub dx_mm: f64,
    pub dy_mm: f64,
    pub count: u32,
    pub angle_deg: f64,
    /// Move by values.
    pub move_dx_mm: f64,
    pub move_dy_mm: f64,
    /// Rotate: degrees counterclockwise on the screen.
    pub rotate_deg: f64,
    pub scale: f64,
    /// Rotate and Scale make copies.
    pub copy_parts: bool,
    /// The kind of a chamfer: 0 equal distances, 1 two distances, 2 a
    /// distance and an angle.
    pub chamfer_kind: u8,
    pub chamfer2_mm: f64,
    pub chamfer_deg: f64,
}

impl EditValues {
    /// The values in round numbers of `units`.
    pub fn for_units(units: crate::workspace::Units) -> EditValues {
        match units {
            crate::workspace::Units::Mm => EditValues::default(),
            crate::workspace::Units::Inch => {
                let inch = crate::workspace::MM_PER_INCH;
                EditValues {
                    fillet_mm: inch / 8.0,
                    chamfer_mm: inch / 16.0,
                    chamfer2_mm: inch / 8.0,
                    offset_mm: inch / 16.0,
                    dx_mm: inch,
                    dy_mm: inch,
                    move_dx_mm: inch / 2.0,
                    ..EditValues::default()
                }
            }
        }
    }
}

impl Default for EditValues {
    fn default() -> EditValues {
        EditValues {
            fillet_mm: 3.0,
            chamfer_mm: 2.0,
            offset_mm: 2.0,
            cols: 3,
            rows: 1,
            dx_mm: 20.0,
            dy_mm: 20.0,
            count: 6,
            angle_deg: 360.0,
            move_dx_mm: 10.0,
            move_dy_mm: 0.0,
            rotate_deg: 90.0,
            scale: 2.0,
            copy_parts: false,
            chamfer_kind: 0,
            chamfer2_mm: 4.0,
            chamfer_deg: 45.0,
        }
    }
}

/// The text of a refused edit, for the message line.
fn edit_message(e: &EditError) -> String {
    match e {
        EditError::Refused(why) => tr(why).to_string(),
        EditError::Missing => tr("The sketch has no such point or curve.").to_string(),
    }
}

/// A part of a sketch that the user picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Point(PointId),
    Curve(CurveId),
    Constraint(ConstraintId),
}

/// A drag of sketch points.
#[derive(Debug, Clone)]
struct Drag {
    /// The points and where they were at the start, in sketch coordinates.
    points: Vec<(PointId, Point)>,
    /// The press, in sketch coordinates.
    start: Point,
    /// A circle whose edge is dragged: its radius follows the pointer.
    radius: Option<CurveId>,
    key: u32,
}

/// The state of sketch mode.
#[derive(Debug, Clone)]
pub struct SketchEdit {
    pub object: ObjectId,
    pub tool: SketchTool,
    /// The clicks of the draw tool so far, in sketch coordinates, with the
    /// point or the curve of the sketch that a click snapped to.
    pub clicks: Vec<Snap>,
    /// Add constraints while drawing (Alt stops them for a click).
    pub auto: bool,
    pub values: EditValues,
    /// The curves that Mirror and Circular Pattern copy.
    pub source: Vec<CurveId>,
    /// The constraints that the last Relax removed (as text).
    pub relaxed: Vec<String>,
    /// The parts that a constraint command or the Dimension tool has.
    pub picks: Vec<Pick>,
    pub selection: Vec<Pick>,
    /// New curves are construction curves.
    pub construction: bool,
    pub polygon_sides: u32,
    /// A polygon round its circle (the sides touch it), else in it.
    pub polygon_outside: bool,
    pub slot_kind: SlotKind,
    /// A dimension whose value the user types, and the text.
    pub value_edit: Option<(ConstraintId, String)>,
    /// The first point of a chain of lines.
    pub chain_start: Option<PointId>,
    /// The value box opened in this frame: it takes the focus and selects
    /// its text.
    value_new: bool,
    drag: Option<Drag>,
    drags: u32,
    /// A box selection: where the drag started (screen).
    boxsel: Option<Pos2>,
    /// A dimension label that the user drags.
    label_drag: Option<(ConstraintId, u32)>,
    /// The Dimension tool has its parts: the next click places the label.
    pub pending_dim: Option<Constraint>,
    /// The user chose the kind of the pending dimension (right click): the
    /// place of the click does not change it.
    pending_locked: bool,
    /// A drag from the end of a line with the Line tool: a tangent arc.
    arc_drag: Option<Snap>,
    /// The points that the transform tools move (with `source`).
    source_points: Vec<PointId>,
    /// Relax mode: a drag changes the dimensions that hold the parts.
    pub relax_mode: bool,
    /// The picked regions: a point in each, in sketch coordinates.
    pub picked: Vec<Point>,
    /// The regions of the sketch (sketch coordinates) and the state of the
    /// design that they are of.
    regions_cache: Option<(u64, Vec<SketchRegion>)>,
    /// Show the marks of the constraints (F8 on, F9 off).
    pub show_marks: bool,
    /// The line that the pointer went over last while drawing (for
    /// parallel and perpendicular hints).
    reference: Option<CurveId>,
    /// Where the context menu opened: the sketch position and the click
    /// tolerance in mm.
    menu_at: Option<(Point, f64)>,
    analysis: Option<(u64, Analysis)>,
    tab_before: RibbonTab,
    /// The Parameters window (ADR 0078).
    pub params: ParamsUi,
}

/// The state of the Parameters window: the texts that the user types.
#[derive(Debug, Clone, Default)]
pub struct ParamsUi {
    pub open: bool,
    /// The typed names and expressions of the parameters, by name.
    names: std::collections::BTreeMap<String, String>,
    exprs: std::collections::BTreeMap<String, String>,
    pub new_name: String,
    pub new_expr: String,
    pub error: Option<String>,
}

/// A change in the Parameters window.
enum ParamAction {
    Set(String, String),
    Rename(String, String),
    Remove(String),
    Units(emb_sketch::Units),
}

impl SketchEdit {
    fn new(object: ObjectId, tab_before: RibbonTab) -> SketchEdit {
        SketchEdit {
            object,
            tool: SketchTool::Line,
            clicks: Vec::new(),
            auto: true,
            values: EditValues::default(),
            source: Vec::new(),
            relaxed: Vec::new(),
            picks: Vec::new(),
            selection: Vec::new(),
            construction: false,
            polygon_sides: 6,
            polygon_outside: false,
            slot_kind: SlotKind::CenterToCenter,
            value_edit: None,
            chain_start: None,
            value_new: false,
            drag: None,
            drags: 0,
            boxsel: None,
            label_drag: None,
            pending_dim: None,
            pending_locked: false,
            arc_drag: None,
            source_points: Vec::new(),
            relax_mode: false,
            picked: Vec::new(),
            regions_cache: None,
            show_marks: true,
            reference: None,
            menu_at: None,
            analysis: None,
            tab_before,
            params: ParamsUi::default(),
        }
    }

    /// Opens the value box of dimension `id` with `text` (selected).
    pub fn edit_value(&mut self, id: ConstraintId, text: String) {
        self.value_edit = Some((id, text));
        self.value_new = true;
    }
}

/// The screen distance (points) for a click on a point or a curve.
const PICK_PX: f32 = 7.0;
