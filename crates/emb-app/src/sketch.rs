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

/// A constraint command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Coincident,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Collinear,
    Tangent,
    Equal,
    Concentric,
    Midpoint,
    OnCurve,
    Symmetric,
    Fix,
}

impl Kind {
    pub const ALL: [Kind; 13] = [
        Kind::Coincident,
        Kind::Horizontal,
        Kind::Vertical,
        Kind::Parallel,
        Kind::Perpendicular,
        Kind::Collinear,
        Kind::Tangent,
        Kind::Equal,
        Kind::Concentric,
        Kind::Midpoint,
        Kind::OnCurve,
        Kind::Symmetric,
        Kind::Fix,
    ];

    /// The name of the command (as on the ribbon).
    pub fn name(self) -> &'static str {
        match self {
            Kind::Coincident => tr("Coincident"),
            Kind::Horizontal => tr("Horizontal"),
            Kind::Vertical => tr("Vertical"),
            Kind::Parallel => tr("Parallel"),
            Kind::Perpendicular => tr("Perpendicular"),
            Kind::Collinear => tr("Collinear"),
            Kind::Tangent => tr("Tangent"),
            Kind::Equal => tr("Equal"),
            Kind::Concentric => tr("Concentric"),
            Kind::Midpoint => tr("Midpoint"),
            Kind::OnCurve => tr("On Curve"),
            Kind::Symmetric => tr("Symmetric"),
            Kind::Fix => tr("Fix"),
        }
    }

    /// What to click, for the status line.
    pub fn prompt(self) -> &'static str {
        match self {
            Kind::Coincident => tr("Click two points."),
            Kind::Horizontal | Kind::Vertical => tr("Click a line, or two points."),
            Kind::Parallel | Kind::Perpendicular | Kind::Collinear => tr("Click two lines."),
            Kind::Tangent => tr("Click a line and a circle or an arc, or two circles or arcs."),
            Kind::Equal => tr("Click two lines, or two circles or arcs."),
            Kind::Concentric => tr("Click two circles or arcs."),
            Kind::Midpoint => tr("Click a point and a line."),
            Kind::OnCurve => tr("Click a point and a curve."),
            Kind::Symmetric => tr("Click two points and the line of symmetry."),
            Kind::Fix => tr("Click a point."),
        }
    }
}

/// A part of a sketch that the user picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Point(PointId),
    Curve(CurveId),
    Constraint(ConstraintId),
}

/// The result of the picks of a constraint command.
enum Build {
    Done(Constraint),
    More,
    Wrong(&'static str),
}

impl Build {
    fn done(self) -> Option<Constraint> {
        match self {
            Build::Done(c) => Some(c),
            _ => None,
        }
    }
}

fn is_line(s: &Sketch, c: CurveId) -> bool {
    s.curve(c).is_some_and(|c| c.is_line())
}

fn ends(s: &Sketch, c: CurveId) -> Option<(PointId, PointId)> {
    match s.curve(c)?.geometry {
        Geometry::Line { a, b } => Some((a, b)),
        _ => None,
    }
}

/// The constraint of `kind` for `picks` (points and curves in click order).
fn build(kind: Kind, s: &Sketch, picks: &[Pick]) -> Build {
    let pts: Vec<PointId> = picks
        .iter()
        .filter_map(|p| match p {
            Pick::Point(p) => Some(*p),
            _ => None,
        })
        .collect();
    let cs: Vec<CurveId> = picks
        .iter()
        .filter_map(|p| match p {
            Pick::Curve(c) => Some(*c),
            _ => None,
        })
        .collect();
    let lines: Vec<CurveId> = cs.iter().copied().filter(|c| is_line(s, *c)).collect();
    let round: Vec<CurveId> = cs.iter().copied().filter(|c| !is_line(s, *c)).collect();
    let wrong = |why| Build::Wrong(why);
    match kind {
        Kind::Coincident => match pts.as_slice() {
            [a, b] => Build::Done(Constraint::Coincident { a: *a, b: *b }),
            [_] if cs.is_empty() => Build::More,
            [] if cs.is_empty() => Build::More,
            _ => wrong(tr("Coincident needs two points.")),
        },
        Kind::Horizontal | Kind::Vertical => {
            let h = kind == Kind::Horizontal;
            match (lines.as_slice(), pts.as_slice(), round.is_empty()) {
                ([l], [], true) => Build::Done(if h {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                }),
                // Two points: a distance of 0 across.
                ([], [a, b], true) => Build::Done(if h {
                    Constraint::VerticalDistance { a: *a, b: *b, mm: 0.0 }
                } else {
                    Constraint::HorizontalDistance { a: *a, b: *b, mm: 0.0 }
                }),
                ([], [_] | [], true) => Build::More,
                _ => wrong(tr("Horizontal and vertical need a line, or two points.")),
            }
        }
        Kind::Parallel | Kind::Perpendicular | Kind::Collinear => {
            match (lines.as_slice(), pts.is_empty() && round.is_empty()) {
                ([a, b], true) => Build::Done(match kind {
                    Kind::Parallel => Constraint::Parallel { a: *a, b: *b },
                    Kind::Perpendicular => Constraint::Perpendicular { a: *a, b: *b },
                    _ => Constraint::Collinear { a: *a, b: *b },
                }),
                ([_] | [], true) => Build::More,
                _ => wrong(tr("This constraint needs two lines.")),
            }
        }
        Kind::Tangent => match (cs.as_slice(), pts.is_empty()) {
            ([a, b], true) if !(is_line(s, *a) && is_line(s, *b)) => Build::Done(Constraint::Tangent {
                a: *a,
                b: *b,
                inside: tangent_inside(s, *a, *b),
            }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr(
                "Tangent needs a line and a circle or an arc, or two circles or arcs.",
            )),
        },
        Kind::Equal => match (cs.as_slice(), pts.is_empty()) {
            ([a, b], true) if is_line(s, *a) == is_line(s, *b) => Build::Done(Constraint::Equal { a: *a, b: *b }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr("Equal needs two lines, or two circles or arcs.")),
        },
        Kind::Concentric => match (round.as_slice(), pts.is_empty() && lines.is_empty()) {
            ([a, b], true) => Build::Done(Constraint::Concentric { a: *a, b: *b }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr("Concentric needs two circles or arcs.")),
        },
        Kind::Midpoint => match (pts.as_slice(), lines.as_slice(), round.is_empty()) {
            ([p], [l], true) => Build::Done(Constraint::Midpoint { point: *p, line: *l }),
            ([_] | [], [_] | [], true) => Build::More,
            _ => wrong(tr("Midpoint needs a point and a line.")),
        },
        Kind::OnCurve => match (pts.as_slice(), cs.as_slice()) {
            ([p], [c]) => Build::Done(Constraint::OnCurve { point: *p, curve: *c }),
            ([_] | [], [_] | []) => Build::More,
            _ => wrong(tr("On curve needs a point and a curve.")),
        },
        Kind::Symmetric => match (pts.as_slice(), lines.as_slice(), round.is_empty()) {
            ([a, b], [l], true) => Build::Done(Constraint::Symmetric { a: *a, b: *b, line: *l }),
            ([] | [_] | [_, _], [] | [_], true) => Build::More,
            _ => wrong(tr("Symmetric needs two points and a line.")),
        },
        Kind::Fix => match (pts.as_slice(), cs.is_empty()) {
            ([p], true) => match s.point(*p) {
                Some(at) => Build::Done(Constraint::Fix { point: *p, at }),
                None => wrong(tr("Fix needs a point.")),
            },
            ([], true) => Build::More,
            _ => wrong(tr("Fix needs a point.")),
        },
    }
}

/// True if two round curves touch with one inside the other (as they are
/// now), for a tangent constraint.
fn tangent_inside(s: &Sketch, a: CurveId, b: CurveId) -> bool {
    let center = |c: CurveId| match s.curve(c)?.geometry {
        Geometry::Circle { center, radius } | Geometry::Arc { center, radius, .. } => Some((s.point(center)?, radius)),
        Geometry::Line { .. } | Geometry::Bezier { .. } => None,
    };
    match (center(a), center(b)) {
        (Some((ca, ra)), Some((cb, rb))) => {
            let d = ca.distance(cb);
            (d - (ra - rb).abs()).abs() < (d - (ra + rb)).abs()
        }
        _ => false,
    }
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

impl AppState {
    /// The sketch object that is open in sketch mode.
    pub fn sketch_obj(&self) -> Option<&SketchObject> {
        let e = self.sketch.as_ref()?;
        match &self.doc.design().object(e.object)?.kind {
            ObjectKind::Sketch(s) => Some(s),
            _ => None,
        }
    }

    /// Start Sketch: opens the selected sketch, else makes a new one.
    pub fn start_sketch(&mut self) {
        if self.sketch.is_some() {
            return;
        }
        let d = self.doc.design();
        let selected = self
            .selection
            .single()
            .filter(|id| matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Sketch(_))));
        let id = match selected {
            Some(id) => id,
            None => {
                let thread = self.current_thread_id();
                self.doc.edit("Start Sketch", |d| {
                    let t = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
                    let id = d.add_object(t, ObjectKind::Sketch(Box::new(SketchObject::new(Sketch::new()))));
                    if let Some(o) = d.object_mut(id) {
                        o.name = Some("Sketch".into());
                    }
                    id
                })
            }
        };
        self.open_sketch(id);
    }

    /// Opens sketch object `id` in sketch mode.
    pub fn open_sketch(&mut self, id: ObjectId) {
        let tab = self.settings.ribbon_tab;
        let mut e = SketchEdit::new(
            id,
            if tab == RibbonTab::Sketch {
                RibbonTab::Digitize
            } else {
                tab
            },
        );
        e.values = EditValues::for_units(self.settings.units);
        self.sketch = Some(e);
        self.set_tool(crate::draw::Tool::Select);
        self.selection.set(id);
        let mut s = self.settings.clone();
        s.ribbon_tab = RibbonTab::Sketch;
        s.ribbon_collapsed = false;
        self.set_settings(s);
        self.message = tr("Sketch mode: draw, then add constraints and dimensions. Finish Sketch goes back.").into();
    }

    /// Finish Sketch: back to the other tabs. An empty sketch goes away.
    pub fn finish_sketch(&mut self) {
        let Some(e) = self.sketch.take() else { return };
        self.doc.end_merge();
        let empty = self.sketch_of(e.object).is_some_and(|s| s.sketch().points.is_empty());
        if empty {
            self.doc.edit("Remove Empty Sketch", |d| {
                d.remove_object(e.object);
            });
            self.selection.clear();
        } else {
            self.selection.set(e.object);
        }
        let mut s = self.settings.clone();
        s.ribbon_tab = e.tab_before;
        self.set_settings(s);
    }

    fn sketch_of(&self, id: ObjectId) -> Option<&SketchObject> {
        match &self.doc.design().object(id)?.kind {
            ObjectKind::Sketch(s) => Some(s),
            _ => None,
        }
    }

    /// Changes the open sketch and solves it: one undo step (with `merge`,
    /// the changes of one drag are one step).
    pub fn edit_sketch<R>(
        &mut self,
        label: &str,
        merge: Option<String>,
        f: impl FnOnce(&mut Sketch) -> R,
    ) -> Option<R> {
        let id = self.sketch.as_ref()?.object;
        self.sketch_of(id)?;
        let mut out = None;
        let mut lost = 0;
        let run = |d: &mut emb_model::Design| {
            if let Some(o) = d.object_mut(id)
                && let ObjectKind::Sketch(s) = &mut o.kind
            {
                out = Some(s.edit(|sk| {
                    let r = f(sk);
                    sk.solve();
                    r
                }));
            }
            // The objects made from the sketch follow it (ADR 0076).
            lost = d.refresh_sketch_links(id);
        };
        match merge {
            Some(k) => self.doc.edit_merged(label, emb_model::MergeKey::new(k), run),
            None => self.doc.edit(label, run),
        }
        self.warn_lost(lost);
        out
    }

    /// The message after a change of the sketch that took the region or
    /// the curve of `n` made objects away.
    fn warn_lost(&mut self, n: usize) {
        if n > 0 {
            self.message = fill(
                "The region or curve of {} made objects is gone. They stay as they are; Unlink ends their link.",
                &[&n],
            );
        }
    }

    /// The analysis of the open sketch (made again when it changes).
    pub fn sketch_analysis(&mut self) -> Option<Analysis> {
        let state = self.doc.state_id();
        if let Some((s, a)) = self.sketch.as_ref().and_then(|e| e.analysis.clone())
            && s == state
        {
            return Some(a);
        }
        let a = self.sketch_obj()?.sketch().analyze();
        if let Some(e) = &mut self.sketch {
            e.analysis = Some((state, a.clone()));
        }
        Some(a)
    }

    /// Leaves sketch mode if the open sketch is gone (for example after an
    /// undo of Start Sketch).
    pub fn check_sketch(&mut self) {
        if let Some(e) = &self.sketch
            && self.sketch_of(e.object).is_none()
        {
            let tab = e.tab_before;
            self.sketch = None;
            let mut s = self.settings.clone();
            s.ribbon_tab = tab;
            self.set_settings(s);
        }
    }

    /// Chooses a tool of sketch mode.
    pub fn set_sketch_tool(&mut self, tool: SketchTool) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        if let SketchTool::Constrain(kind) = tool
            && !sel.is_empty()
            && self.apply_constraint(kind, &sel)
        {
            return;
        }
        let curves: Vec<CurveId> = sel
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        match tool {
            // Offset of the selected curves at once.
            SketchTool::Offset if !curves.is_empty() => {
                self.offset_curves(&curves);
                return;
            }
            SketchTool::Mirror | SketchTool::CircularPattern if curves.is_empty() => {
                self.message = tr("Select the curves to copy first.").into();
                return;
            }
            t if t.is_transform() && sel.iter().all(|p| matches!(p, Pick::Constraint(_))) => {
                self.message = tr("Select the parts first.").into();
                return;
            }
            _ => {}
        }
        if let Some(e) = &mut self.sketch {
            e.tool = tool;
            e.clicks.clear();
            e.picks.clear();
            if matches!(tool, SketchTool::Mirror | SketchTool::CircularPattern) || tool.is_transform() {
                e.source = curves;
                e.source_points = sel
                    .iter()
                    .filter_map(|p| match p {
                        Pick::Point(p) => Some(*p),
                        _ => None,
                    })
                    .collect();
            }
        }
    }

    /// Changes the open sketch with an edit that can be refused: one undo
    /// step if it is done, none if it is refused.
    fn try_edit_sketch<R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Sketch) -> Result<R, EditError>,
    ) -> Option<Result<R, EditError>> {
        let id = self.sketch.as_ref()?.object;
        self.sketch_of(id)?;
        let mut lost = 0;
        let out = self.doc.try_edit(label, |d| {
            let Some(ObjectKind::Sketch(s)) = d.object_mut(id).map(|o| &mut o.kind) else {
                return Err(EditError::Missing);
            };
            let r = s.edit(|sk| {
                let r = f(sk);
                if r.is_ok() {
                    sk.solve();
                }
                r
            });
            if r.is_ok() {
                lost = d.refresh_sketch_links(id);
            }
            r
        });
        self.warn_lost(lost);
        Some(out)
    }

    /// Offset of `curves` at the distance of the panel: one undo step.
    fn offset_curves(&mut self, curves: &[CurveId]) {
        let mm = self.sketch.as_ref().map_or(2.0, |e| e.values.offset_mm);
        let r = self.try_edit_sketch("Offset", |sk| sk.offset(curves, mm));
        self.edit_done(r.map(|r| r.map(|made| fill("Offset: {} new curves.", &[&made.len()]))));
    }

    /// The message of an edit, and the end of its selection.
    fn edit_done(&mut self, r: Option<Result<String, EditError>>) {
        match r {
            Some(Ok(text)) => {
                self.message = text;
                if let Some(e) = &mut self.sketch {
                    e.selection.clear();
                }
            }
            Some(Err(e)) => self.message = edit_message(&e),
            None => {}
        }
    }

    /// A click of an edit tool at sketch position `at`.
    fn edit_click(&mut self, tool: SketchTool, at: Point, tol: f64) {
        let Some(obj) = self.sketch_obj() else { return };
        let s = obj.sketch();
        let curve = s.nearest_curve(at, tol, &[]).map(|(c, _)| c);
        let point = infer::snap(s, at, tol, &[]).point;
        let v = self.sketch.as_ref().map(|e| e.values.clone()).unwrap_or_default();
        let source = self.sketch.as_ref().map(|e| e.source.clone()).unwrap_or_default();
        let need_curve = tr("Click a curve.");
        let r: Option<Result<String, EditError>> = match tool {
            SketchTool::Trim => match curve {
                Some(c) => self
                    .try_edit_sketch("Trim", |sk| sk.trim(c, at))
                    .map(|r| r.map(|_| tr("Trimmed.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Extend => match curve {
                Some(c) => self
                    .try_edit_sketch("Extend", |sk| sk.extend(c, at))
                    .map(|r| r.map(|_| tr("Extended.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Split => match curve {
                Some(c) => self
                    .try_edit_sketch("Split", |sk| sk.split(c, at))
                    .map(|r| r.map(|_| tr("Split in two.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Fillet | SketchTool::Chamfer => {
                let Some(p) = point else {
                    self.message = tr("Click the corner of two lines.").into();
                    return;
                };
                let fillet = tool == SketchTool::Fillet;
                let r = if fillet {
                    self.try_edit_sketch("Fillet", |sk| sk.fillet(p, v.fillet_mm))
                } else {
                    let kind = match v.chamfer_kind {
                        1 => emb_sketch::edit::ChamferKind::Two(v.chamfer_mm, v.chamfer2_mm),
                        2 => emb_sketch::edit::ChamferKind::Angle(v.chamfer_mm, v.chamfer_deg),
                        _ => emb_sketch::edit::ChamferKind::Equal(v.chamfer_mm),
                    };
                    self.try_edit_sketch("Chamfer", |sk| sk.chamfer_with(p, kind))
                };
                // Type the value of the new dimension at once.
                if let Some(Ok((_, dim))) = &r
                    && let Some(c) = self.sketch_obj().and_then(|o| o.sketch().constraint(*dim).copied())
                {
                    let text = value_text(self.settings.units, &c);
                    if let Some(e) = &mut self.sketch {
                        e.edit_value(*dim, text);
                    }
                }
                r.map(|r| {
                    r.map(|_| {
                        if fillet {
                            tr("Fillet added: type its radius.").to_string()
                        } else {
                            tr("Chamfer added: type its distance.").to_string()
                        }
                    })
                })
            }
            SketchTool::Offset => match curve {
                Some(c) => {
                    let chain = s.connected_curves(c);
                    self.offset_curves(&chain);
                    return;
                }
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Mirror => match curve.filter(|c| is_line(s, *c)) {
                Some(axis) => self
                    .try_edit_sketch("Mirror", |sk| sk.mirror(&source, axis))
                    .map(|r| r.map(|made| fill("Mirror: {} new curves.", &[&made.len()]))),
                None => Some(Err(EditError::Refused(tr("Click the line to mirror about.")))),
            },
            SketchTool::CircularPattern => {
                let center = point.and_then(|p| s.point(p)).unwrap_or(at);
                let (n, deg) = (v.count as usize, v.angle_deg);
                self.try_edit_sketch("Circular Pattern", |sk| sk.circular_pattern(&source, center, n, deg))
                    .map(|r| r.map(|made| fill("Pattern: {} new curves.", &[&made.len()])))
            }
            t if t.is_transform() => {
                let at = infer::snap(s, at, tol, &[]).at;
                self.transform_click(t, at);
                return;
            }
            _ => None,
        };
        let done = matches!(r, Some(Ok(_)));
        self.edit_done(r);
        // A mirror or a pattern is done; the other tools stay for more clicks.
        if done
            && matches!(tool, SketchTool::Mirror | SketchTool::CircularPattern)
            && let Some(e) = &mut self.sketch
        {
            e.tool = SketchTool::Select;
            e.source.clear();
        }
    }

    /// A click of Move, Copy, Rotate or Scale at sketch position `at`.
    fn transform_click(&mut self, tool: SketchTool, at: Point) {
        let Some(e) = self.sketch.as_mut() else { return };
        if matches!(tool, SketchTool::Move | SketchTool::Copy) && e.clicks.is_empty() {
            e.clicks.push(Snap::free(at));
            self.message = tr("Click where the base point goes.").into();
            return;
        }
        let base = e.clicks.first().map(|c| c.at);
        e.clicks.clear();
        let v = e.values.clone();
        let f = match (tool, base) {
            (SketchTool::Move | SketchTool::Copy, Some(b)) => Motion::Shift(at - b),
            (SketchTool::Rotate, _) => Motion::Turn(at, v.rotate_deg),
            (SketchTool::Scale, _) => Motion::Grow(at, v.scale),
            _ => return,
        };
        let copy = tool == SketchTool::Copy || (matches!(tool, SketchTool::Rotate | SketchTool::Scale) && v.copy_parts);
        self.apply_motion(tool, f, copy);
    }

    /// Moves, turns or scales the parts of the transform tool (or copies).
    pub fn apply_motion(&mut self, tool: SketchTool, m: Motion, copy: bool) {
        let Some(e) = self.sketch.as_ref() else { return };
        let (curves, points) = (e.source.clone(), e.source_points.clone());
        let label = match tool {
            SketchTool::Stretch => "Stretch in Sketch",
            SketchTool::Rotate => "Rotate in Sketch",
            SketchTool::Scale => "Scale in Sketch",
            _ if copy => "Copy in Sketch",
            _ => "Move in Sketch",
        };
        let (scale, turned) = match m {
            Motion::Grow(_, k) => (k, false),
            Motion::Turn(..) => (1.0, true),
            Motion::Shift(_) => (1.0, false),
        };
        let r = self.try_edit_sketch(label, |sk| {
            sk.transform_parts(&curves, &points, &|p| m.apply(p), scale, turned, copy)
        });
        self.edit_done(r.map(|r| {
            r.map(|made| {
                if copy {
                    fill("{} new curves.", &[&made.len()])
                } else {
                    tr("Moved as near as the constraints let it.").to_string()
                }
            })
        }));
        // The parts stay selected for one more move.
        if let Some(e) = &mut self.sketch {
            e.selection = e
                .source
                .iter()
                .map(|c| Pick::Curve(*c))
                .chain(e.source_points.iter().map(|p| Pick::Point(*p)))
                .collect();
        }
    }

    /// Make Pattern (Rectangular Pattern): the selected curves in columns
    /// and rows.
    pub fn rect_pattern(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        if curves.is_empty() {
            self.message = tr("Select the curves to copy first.").into();
            return;
        }
        let v = e.values.clone();
        let r = self.try_edit_sketch("Rectangular Pattern", |sk| {
            sk.rect_pattern(&curves, v.cols as usize, v.rows as usize, v.dx_mm, v.dy_mm)
        });
        self.edit_done(r.map(|r| r.map(|made| fill("Pattern: {} new curves.", &[&made.len()]))));
    }

    /// Relax: removes the constraints that conflict (dimensions first), in
    /// one undo step, and lists them.
    pub fn relax_sketch(&mut self) {
        let has = self.sketch_analysis().is_some_and(|a| !a.conflicting.is_empty());
        if !has {
            self.message = tr("No constraints conflict.").into();
            return;
        }
        let units = self.settings.units;
        let removed = self.edit_sketch("Relax", None, |sk| sk.relax()).unwrap_or_default();
        let list: Vec<String> = removed.iter().map(|c| constraint_text(units, c)).collect();
        self.message = fill("Relax removed {} constraints: {}.", &[&list.len(), &list.join(", ")]);
        if let Some(e) = &mut self.sketch {
            e.relaxed = list;
        }
    }

    /// Adds the constraint of `kind` for `picks`. Returns `true` if it was
    /// added (or refused with a message), `false` if it needs more picks.
    fn apply_constraint(&mut self, kind: Kind, picks: &[Pick]) -> bool {
        let Some(obj) = self.sketch_obj() else { return true };
        match build(kind, obj.sketch(), picks) {
            Build::Done(c) => {
                let name = c.name();
                let r = self.edit_sketch(&format!("Constraint: {name}"), None, |sk| sk.add_constraint(c));
                match r {
                    Some(Ok(_)) => self.message = fill("{} added.", &[&tr(name)]),
                    Some(Err(e)) => self.message = e.to_string(),
                    None => {}
                }
                if let Some(e) = &mut self.sketch {
                    e.selection.clear();
                    e.picks.clear();
                }
                true
            }
            Build::More => false,
            Build::Wrong(why) => {
                self.message = why.into();
                if let Some(e) = &mut self.sketch {
                    e.picks.clear();
                }
                true
            }
        }
    }

    /// Delete in sketch mode: the selected constraints, curves and points
    /// (a point with the curves that use it), with their constraints.
    pub fn delete_sketch_selection(&mut self) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        if sel.is_empty() {
            self.message = tr("Select the parts to delete first.").into();
            return;
        }
        self.edit_sketch("Delete in Sketch", None, |sk| {
            for p in &sel {
                match *p {
                    Pick::Constraint(c) => {
                        sk.remove_constraint(c);
                    }
                    Pick::Curve(c) => {
                        sk.remove_curve(c);
                    }
                    Pick::Point(_) => {}
                }
            }
            for p in &sel {
                if let Pick::Point(id) = *p {
                    let users: Vec<CurveId> = sk
                        .curves
                        .iter()
                        .filter(|c| c.points().contains(&id))
                        .map(|c| c.id)
                        .collect();
                    for c in users {
                        sk.remove_curve(c);
                    }
                    sk.remove_point(id);
                }
            }
        });
        if let Some(e) = &mut self.sketch {
            e.selection.clear();
        }
        self.message = if sel.len() == 1 {
            tr("Deleted.").to_string()
        } else {
            fill("Deleted {} parts.", &[&sel.len()])
        };
    }

    /// Esc in sketch mode: ends the command and goes back to Select; in
    /// Select, clears the selection.
    pub fn sketch_escape(&mut self) {
        let Some(e) = &mut self.sketch else { return };
        e.clicks.clear();
        e.picks.clear();
        e.chain_start = None;
        e.source.clear();
        e.boxsel = None;
        e.pending_dim = None;
        e.pending_locked = false;
        e.arc_drag = None;
        e.source_points.clear();
        if e.tool != SketchTool::Select {
            e.tool = SketchTool::Select;
        } else {
            e.selection.clear();
            e.picked.clear();
        }
    }

    /// Adds dimension `c` with its label at `at` (else the program places
    /// it), and opens its value box. A dimension that the sketch sets
    /// already is driven: it shows the size only.
    pub fn add_sketch_dimension(&mut self, c: Constraint, at: Option<Point>) {
        let r = self.edit_sketch(&format!("Dimension: {}", c.name()), None, |sk| {
            let r = sk.add_dimension(c);
            if let (Ok((id, _)), Some(at)) = (&r, at) {
                sk.label_at.insert(*id, at);
            }
            r
        });
        let units = self.settings.units;
        let Some(e) = self.sketch.as_mut() else { return };
        e.picks.clear();
        match r {
            Some(Ok((id, false))) => e.edit_value(id, value_text(units, &c)),
            Some(Ok((_, true))) => {
                self.message = tr(
                    "The sketch sets this size already: the dimension is driven (in parentheses). Right-click it to make it driving.",
                )
                .into();
            }
            Some(Err(err)) => self.message = err.to_string(),
            None => {}
        }
    }

    /// Makes the selected dimension driven, or driving again.
    pub fn toggle_driven(&mut self, id: ConstraintId) {
        let on = !self.sketch_obj().is_some_and(|o| o.sketch().is_driven(id));
        self.edit_sketch(if on { "Driven Dimension" } else { "Driving Dimension" }, None, |sk| {
            sk.set_driven(id, on)
        });
        if !on
            && self
                .sketch_analysis()
                .is_some_and(|a| a.conflicting.contains(&id) || a.redundant.contains(&id))
        {
            self.message = tr("This dimension repeats or conflicts with others (gray or red).").into();
        }
    }

    /// The closed regions of the open sketch in sketch coordinates (made
    /// again when the design changes).
    pub fn sketch_regions(&mut self) -> Vec<SketchRegion> {
        let state = self.doc.state_id();
        if let Some((s, r)) = self.sketch.as_ref().and_then(|e| e.regions_cache.clone())
            && s == state
        {
            return r;
        }
        let r = self.sketch_obj().map(|o| o.sketch().regions()).unwrap_or_default();
        if let Some(e) = &mut self.sketch {
            e.regions_cache = Some((state, r.clone()));
        }
        r
    }

    /// The smallest region at sketch point `at`.
    fn sketch_region_at(&mut self, at: Point) -> Option<SketchRegion> {
        self.sketch_regions()
            .into_iter()
            .filter(|r| r.contains(at))
            .min_by(|a, b| a.area.total_cmp(&b.area))
    }

    /// A click of the Regions tool: picks the region at `at`, or unpicks it.
    fn toggle_region(&mut self, at: Point) {
        let Some(r) = self.sketch_region_at(at) else {
            self.message = tr("Click inside a closed region.").into();
            return;
        };
        let Some(e) = &mut self.sketch else { return };
        let before = e.picked.len();
        e.picked.retain(|p| !r.contains(*p));
        if e.picked.len() == before {
            e.picked.push(at);
        }
        let n = e.picked.len();
        self.message = fill("{} regions picked.", &[&n]);
    }

    /// The picked regions of the open sketch in design coordinates, else
    /// all its regions.
    fn picked_regions(&mut self) -> Vec<SketchRegion> {
        let all = self.sketch_regions();
        let picked = self.sketch.as_ref().map(|e| e.picked.clone()).unwrap_or_default();
        let Some(t) = self.sketch_obj().map(|o| o.placement()) else {
            return Vec::new();
        };
        let mut out: Vec<SketchRegion> = Vec::new();
        for r in &all {
            let smallest_at = |p: &Point| {
                all.iter()
                    .filter(|x| x.contains(*p))
                    .min_by(|a, b| a.area.total_cmp(&b.area))
                    .is_some_and(|x| x == r)
            };
            if picked.is_empty() || picked.iter().any(smallest_at) {
                out.push(r.transformed(&t));
            }
        }
        out
    }

    /// The color of the current thread (gray if there is none).
    fn current_color(&self) -> emb_model::Rgb {
        self.current_thread_id()
            .and_then(|t| self.doc.design().threads.iter().find(|x| x.id == t).map(|x| x.color))
            .unwrap_or(emb_model::Rgb(90, 90, 90))
    }

    /// Makes the objects of `sources` from sketch `sketch` with the current
    /// properties and the current thread, linked to the sketch (ADR 0076):
    /// one undo step. Returns the new objects.
    fn make_linked(&mut self, label: &str, sketch: ObjectId, sources: Vec<LinkSource>) -> Vec<ObjectId> {
        let current = self.doc.design().current.clone();
        let thread = self.current_thread_id();
        let color = self.current_color();
        self.doc.edit(label, |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Thread 1", color));
            sources
                .into_iter()
                .flat_map(|s| d.make_from_sketch(sketch, s, &current, t))
                .collect()
        })
    }

    /// Makes embroidery objects of the picked regions of the open sketch,
    /// else of all its regions.
    pub fn make_picked_regions(&mut self, to: RegionTo) {
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        let regions = self.picked_regions();
        self.make_regions(id, regions, to);
    }

    /// Makes embroidery objects of `regions` (design coordinates) of sketch
    /// `sketch` with the current properties and the current thread: one
    /// undo step.
    pub fn make_regions(&mut self, sketch: ObjectId, regions: Vec<SketchRegion>, to: RegionTo) {
        if regions.is_empty() {
            self.message = tr("The sketch has no closed regions.").into();
            return;
        }
        let Some(obj) = self.sketch_of(sketch) else { return };
        let width_mm = self.settings.defaults.column_width_mm;
        let color = self.current_color();
        let sources: Vec<LinkSource> = regions
            .iter()
            .map(|r| LinkSource::Region {
                key: obj.region_key(r),
                to,
                width_mm,
                color,
            })
            .collect();
        let made = self.make_linked(&format!("Sketch to {}", to.name()), sketch, sources);
        if let Some(e) = &mut self.sketch {
            e.picked.clear();
        }
        self.message = if made.is_empty() {
            tr("Nothing could be made from these regions.").to_string()
        } else {
            fill("{} objects made from the sketch.", &[&made.len()])
        };
    }

    /// Makes runs or satin lines along the selected curves of the open
    /// sketch (also construction curves).
    pub fn make_curves(&mut self, to: CurveTo) {
        let curves = self.selected_curves();
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        if curves.is_empty() {
            self.message = tr("Select the curves first.").into();
            return;
        }
        let sources = curves
            .into_iter()
            .map(|curve| LinkSource::Curve { curve, to })
            .collect();
        let n = self.make_linked("Sketch to Stitches", id, sources).len();
        self.message = fill("{} objects made from the sketch.", &[&n]);
    }

    /// A satin column between the two selected curves.
    pub fn make_satin_between(&mut self) {
        let curves = self.selected_curves();
        let [a, b] = curves.as_slice() else {
            self.message = tr("Select two curves for a satin column between them.").into();
            return;
        };
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        let source = LinkSource::SatinBetween { a: *a, b: *b };
        if self.make_linked("Sketch to Satin", id, vec![source]).is_empty() {
            return;
        }
        self.message = tr("Satin column made between the two curves.").into();
    }

    fn selected_curves(&self) -> Vec<CurveId> {
        self.sketch
            .as_ref()
            .map(|e| {
                e.selection
                    .iter()
                    .filter_map(|p| match p {
                        Pick::Curve(c) => Some(*c),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Removes the constraints of the selected points and curves.
    pub fn remove_sketch_constraints_of_selection(&mut self) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        let (points, curves): (Vec<PointId>, Vec<CurveId>) = (
            sel.iter()
                .filter_map(|p| match p {
                    Pick::Point(p) => Some(*p),
                    _ => None,
                })
                .collect(),
            sel.iter()
                .filter_map(|p| match p {
                    Pick::Curve(c) => Some(*c),
                    _ => None,
                })
                .collect(),
        );
        let n = self.edit_sketch("Delete Constraints", None, |sk| {
            let before = sk.constraints.len();
            sk.constraints.retain(|c| {
                !c.constraint.points().iter().any(|p| points.contains(p))
                    && !c.constraint.curves().iter().any(|x| curves.contains(x))
            });
            before - sk.constraints.len()
        });
        self.message = fill("Removed {} constraints.", &[&n.unwrap_or(0)]);
    }

    /// Selects all points, curves and constraints of the open sketch.
    pub fn sketch_select_all(&mut self) {
        let Some(obj) = self.sketch_obj() else { return };
        let s = obj.sketch();
        let all: Vec<Pick> = s
            .curves
            .iter()
            .map(|c| Pick::Curve(c.id))
            .chain(s.constraints.iter().map(|c| Pick::Constraint(c.id)))
            .collect();
        if let Some(e) = &mut self.sketch {
            e.selection = all;
        }
    }

    /// Makes the selected curves construction curves (or not), or turns
    /// construction on for new curves.
    pub fn toggle_construction(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        if curves.is_empty() {
            if let Some(e) = &mut self.sketch {
                e.construction = !e.construction;
            }
            return;
        }
        let on = self.sketch_obj().is_some_and(|o| {
            curves
                .iter()
                .any(|c| o.sketch().curve(*c).is_some_and(|c| !c.construction))
        });
        self.edit_sketch("Construction", None, |sk| {
            for c in &curves {
                sk.set_construction(*c, on);
            }
        });
    }

    /// Makes the selected points center points, or not.
    pub fn toggle_center_point(&mut self) {
        let Some(e) = &self.sketch else { return };
        let pts: Vec<PointId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Point(p) => Some(*p),
                _ => None,
            })
            .collect();
        if pts.is_empty() {
            self.message = tr("Select the points to make center points.").into();
            return;
        }
        let on = self
            .sketch_obj()
            .is_some_and(|o| pts.iter().any(|p| !o.sketch().is_center_point(*p)));
        self.edit_sketch("Center Point", None, |sk| {
            for p in &pts {
                sk.set_center_point(*p, on);
            }
        });
    }

    /// Automatic Dimensions: constraints and dimensions for the selected
    /// curves (all curves when none is selected), in one undo step.
    pub fn auto_dimension(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        let r = self.edit_sketch("Automatic Dimensions", None, |sk| sk.auto_dimension(&curves));
        let free = self.sketch_analysis().map_or(0, |a| a.free);
        if let Some((c, d)) = r {
            self.message = fill(
                "Added {} constraints and {} dimensions; {} free motions left (a Fix places the sketch).",
                &[&c, &d, &free],
            );
        }
    }

    /// Makes the selected lines centerlines, or not.
    pub fn toggle_centerline(&mut self) {
        let Some(e) = &self.sketch else { return };
        let lines: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .filter(|c| self.sketch_obj().is_some_and(|o| is_line(o.sketch(), *c)))
            .collect();
        if lines.is_empty() {
            self.message = tr("Select the lines to make centerlines.").into();
            return;
        }
        let on = self
            .sketch_obj()
            .is_some_and(|o| lines.iter().any(|l| !o.sketch().is_centerline(*l)));
        self.edit_sketch("Centerline", None, |sk| {
            for l in &lines {
                sk.set_centerline(*l, on);
            }
        });
    }

    /// Sets the value of dimension `id` from the text the user typed (in
    /// the units of the Options; an angle in degrees).
    fn set_dimension_text(&mut self, id: ConstraintId, text: &str) {
        let Some(old) = self.sketch_obj().and_then(|o| o.sketch().constraint(id).copied()) else {
            return;
        };
        // An expression with the names of parameters (ADR 0078).
        if emb_sketch::expr::parse(text).is_ok_and(|e| !e.names().is_empty()) {
            let units = self.settings.units;
            let r = self.edit_params("Dimension", |sk| {
                first_units(sk, units);
                sk.set_dimension_expr(id, text)
            });
            if let Err(e) = r {
                self.message = expr_message(&e);
            }
            return;
        }
        let typed = match old {
            Constraint::Angle { .. } => crate::workspace::parse_number(text),
            _ => crate::workspace::parse_length(text, self.settings.units),
        };
        let Some(typed) = typed else {
            self.message = tr("Type a number, for example 12.5, 2 in or 1 1/2\".").into();
            return;
        };
        let has_expr = self
            .sketch_obj()
            .is_some_and(|o| o.sketch().dimension_expr(id).is_some());
        if old.value() == Some(typed) && !has_expr {
            return;
        }
        let v = match old {
            Constraint::Angle { .. } => typed,
            // A horizontal or vertical distance keeps its direction.
            _ => typed.abs() * old.value().map_or(1.0, |o| if o < 0.0 { -1.0 } else { 1.0 }),
        };
        let r = self.edit_sketch("Dimension", None, |sk| {
            sk.clear_dimension_expr(id);
            sk.set_value(id, v)
        });
        match r {
            Some(Err(e)) => self.message = e.to_string(),
            _ => {
                if let Some(a) = self.sketch_analysis()
                    && a.conflicting.contains(&id)
                {
                    self.message = tr("This value conflicts with other constraints (red).").into();
                }
            }
        }
    }

    /// A change of the parameters or expressions of the open sketch: one
    /// undo step, and no step if it is refused.
    fn edit_params<R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Sketch) -> Result<R, emb_sketch::ExprError>,
    ) -> Result<R, emb_sketch::ExprError> {
        let mut err = None;
        let r = self.try_edit_sketch(label, |sk| {
            f(sk).map_err(|e| {
                err = Some(e);
                EditError::Missing
            })
        });
        match r {
            Some(Ok(v)) => Ok(v),
            _ => Err(err.unwrap_or(emb_sketch::ExprError::BadValue)),
        }
    }

    /// Vector to Sketch (ADR 0079): the selected vector objects become one
    /// sketch in their place, with the constraints that the program finds.
    /// One undo step.
    pub fn vectors_to_sketch(&mut self) {
        let d = self.doc.design();
        let vectors: Vec<(ObjectId, emb_geom::Path)> = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| match &d.object(*id)?.kind {
                ObjectKind::Vector { path, .. } => Some((*id, path.clone())),
                _ => None,
            })
            .collect();
        if vectors.is_empty() {
            self.message = tr("Select vector objects first.").into();
            return;
        }
        let paths: Vec<emb_geom::Path> = vectors.iter().map(|(_, p)| p.clone()).collect();
        let sketch = emb_sketch::import::from_paths(&paths, emb_sketch::import::TOL);
        let (curves, constraints) = (sketch.curves.len(), sketch.constraints.len());
        let ids: Vec<ObjectId> = vectors.iter().map(|(id, _)| *id).collect();
        let id = self.doc.edit("Vector to Sketch", |d| {
            let at = ids.iter().filter_map(|id| d.index_of(*id)).min().unwrap_or(0);
            let thread = d.object(ids[0]).map(|o| o.thread);
            for id in &ids {
                d.remove_object(*id);
            }
            let thread = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
            let new = d.add_object(thread, ObjectKind::Sketch(Box::new(SketchObject::new(sketch))));
            d.move_object(new, at);
            new
        });
        self.selection.set(id);
        self.message = fill(
            "A sketch with {} curves and {} constraints that the program found.",
            &[&curves, &constraints],
        );
    }

    /// Opens a DXF drawing as a new sketch, centered in the hoop (ADR 0079).
    pub fn import_dxf_bytes(&mut self, bytes: &[u8], source: &str) {
        let text = String::from_utf8_lossy(bytes);
        let sketch = match emb_sketch::import::from_dxf(&text) {
            Ok(s) => s,
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Cannot import the drawing".into(),
                    lines: vec![source.to_string(), e.to_string()],
                });
                return;
            }
        };
        let mut obj = SketchObject::new(sketch);
        let target = self.doc.design().hoop.center;
        let bbox = emb_geom::BBox::from_points(obj.paths().flat_map(|p| p.flatten(0.05)).flat_map(|pl| pl.points));
        if let Some(b) = bbox {
            let shift = target - b.center();
            obj.transform(&emb_geom::Transform::translate(shift.x, shift.y));
        }
        let (curves, constraints) = (obj.sketch().curves.len(), obj.sketch().constraints.len());
        let thread = self.current_thread_id();
        let id = self.doc.edit("Import DXF", |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
            d.add_object(t, ObjectKind::Sketch(Box::new(obj)))
        });
        self.selection.set(id);
        self.fit_requested = true;
        self.message = fill(
            "A sketch with {} curves and {} constraints that the program found.",
            &[&curves, &constraints],
        );
    }

    /// Opens or closes the Parameters window of the open sketch.
    pub fn toggle_parameters(&mut self) {
        if let Some(e) = &mut self.sketch {
            e.params.open = !e.params.open;
        }
    }

    fn param_action(&mut self, a: ParamAction) {
        let units = self.settings.units;
        let r = match a {
            ParamAction::Set(name, text) => self.edit_params("Parameter", |sk| {
                first_units(sk, units);
                sk.set_parameter(&name, &text)
            }),
            ParamAction::Rename(old, new) => self.edit_params("Rename Parameter", |sk| sk.rename_parameter(&old, &new)),
            ParamAction::Remove(name) => self.edit_params("Remove Parameter", |sk| sk.remove_parameter(&name)),
            ParamAction::Units(u) => self.edit_params("Parameter Units", |sk| {
                sk.units = u;
                sk.apply_parameters()
            }),
        };
        if let Some(e) = &mut self.sketch {
            e.params.error = r.err().map(|e| expr_message(&e));
            e.params.names.clear();
            e.params.exprs.clear();
        }
    }

    /// The text for the status line in sketch mode.
    pub fn sketch_status(&mut self) -> Option<(String, String)> {
        let tool = self.sketch.as_ref()?.tool;
        let clicks = self.sketch.as_ref()?.clicks.len();
        let picks_line = {
            let e = self.sketch.as_ref()?;
            matches!(e.picks.as_slice(), [Pick::Curve(c)] if self.sketch_obj().is_some_and(|o| is_line(o.sketch(), *c)))
        };
        let prompt = match tool {
            SketchTool::Select => {
                tr("Click a part to select it, drag it to move it, double-click a dimension to change it.")
            }
            SketchTool::Spline if clicks == 0 => tr("Click the first point of the spline."),
            SketchTool::Spline => tr(
                "Click the next point. Enter, a right click or a click on the last point ends the spline; click the first point to close it.",
            ),
            SketchTool::Line if clicks == 0 => tr("Click the start of the line."),
            SketchTool::Line => tr(
                "Click the next point (drag from the end for a tangent arc). Click the first point to close; Enter or Esc to end.",
            ),
            SketchTool::Rectangle if clicks == 0 => tr("Click a corner."),
            SketchTool::Rectangle => tr("Click the opposite corner."),
            SketchTool::RectCenter if clicks == 0 => tr("Click the center of the rectangle."),
            SketchTool::RectCenter => tr("Click a corner."),
            SketchTool::Rect3 => match clicks {
                0 => tr("Click the first corner of a side."),
                1 => tr("Click the second corner of the side."),
                _ => tr("Click to set the height."),
            },
            SketchTool::Slot => {
                let kind = self.sketch.as_ref().map_or(SlotKind::CenterToCenter, |e| e.slot_kind);
                match (kind, clicks) {
                    (SlotKind::Overall, 0) => tr("Click the first end of the slot."),
                    (SlotKind::Overall, 1) => tr("Click the second end."),
                    (SlotKind::CenterPoint, 0) => tr("Click the middle of the slot."),
                    (SlotKind::CenterPoint, 1) => tr("Click the center of an end."),
                    (SlotKind::Arc3, 0) => tr("Click the first center of the slot."),
                    (SlotKind::Arc3, 1) => tr("Click the second center."),
                    (SlotKind::Arc3, 2) => tr("Click a point on the arc of the slot."),
                    (SlotKind::ArcCenter, 0) => tr("Click the center of the arc of the slot."),
                    (SlotKind::ArcCenter, 1) => tr("Click the first center of the slot."),
                    (SlotKind::ArcCenter, 2) => tr("Click the second center (counterclockwise)."),
                    (_, 0) => tr("Click the first center of the slot."),
                    (_, 1) => tr("Click the second center."),
                    _ => tr("Click to set the width."),
                }
            }
            SketchTool::RectCenter3 => match clicks {
                0 => tr("Click the center of the rectangle."),
                1 => tr("Click the middle of a side."),
                _ => tr("Click a corner."),
            },
            SketchTool::TangentCircle => tr("Click three lines."),
            SketchTool::Regions => tr(
                "Click inside closed regions to pick them (again to unpick), then make embroidery in the Properties panel or with a right click.",
            ),
            SketchTool::Stretch => {
                tr("Drag a box round the points to stretch, then click a base point and where it goes.")
            }
            SketchTool::Circle if clicks == 0 => tr("Click the center."),
            SketchTool::Circle => tr("Click a point on the circle."),
            SketchTool::Arc3 => match clicks {
                0 => tr("Click the start of the arc."),
                1 => tr("Click the end of the arc."),
                _ => tr("Click a point on the arc."),
            },
            SketchTool::ArcCenter => match clicks {
                0 => tr("Click the center."),
                1 => tr("Click the start (the arc goes counterclockwise)."),
                _ => tr("Click the end."),
            },
            SketchTool::ArcTangent if clicks == 0 => tr("Click the end of a line."),
            SketchTool::ArcTangent => tr("Click the end of the arc."),
            SketchTool::Polygon if clicks == 0 => tr("Click the center."),
            SketchTool::Polygon => tr("Click a corner."),
            SketchTool::Point => tr("Click to add a point."),
            SketchTool::Dimension if self.sketch.as_ref().is_some_and(|e| e.pending_dim.is_some()) => {
                tr("Click to place the dimension (Esc cancels).")
            }
            SketchTool::Dimension if picks_line => tr(
                "Click another line (angle), a point (distance to the line), or an empty place (length of the line).",
            ),
            SketchTool::Dimension => tr(
                "Click a line, a circle or an arc (size), two points (distance), or a point and a line (distance to the line).",
            ),
            SketchTool::Constrain(k) => k.prompt(),
            SketchTool::Trim => tr("Click the piece of a curve to remove."),
            SketchTool::Extend => tr("Click near the end of a line or an arc to extend it."),
            SketchTool::Split => tr("Click a line or an arc where to cut it in two."),
            SketchTool::Fillet => {
                tr("Click the corner of two lines to round it (the radius is in the Properties panel).")
            }
            SketchTool::Chamfer => {
                tr("Click the corner of two lines to cut it (the distance is in the Properties panel).")
            }
            SketchTool::Offset => {
                tr("Click a curve: it and the curves joined to it get a copy at the distance of the Properties panel.")
            }
            SketchTool::Mirror => tr("Click the line to mirror about."),
            SketchTool::RectPattern => tr("Select the curves, set the columns and rows, then click Make Pattern."),
            SketchTool::CircularPattern => tr("Click the center of the pattern."),
            SketchTool::Move | SketchTool::Copy if clicks == 0 => tr("Click the base point."),
            SketchTool::Move | SketchTool::Copy => tr("Click where the base point goes."),
            SketchTool::Rotate => tr("Click the center of the turn."),
            SketchTool::Scale => tr("Click the base point of the scale."),
        };
        let prompt = if matches!(
            tool,
            SketchTool::Line
                | SketchTool::Rectangle
                | SketchTool::RectCenter
                | SketchTool::Rect3
                | SketchTool::RectCenter3
                | SketchTool::Slot
                | SketchTool::Circle
                | SketchTool::Arc3
                | SketchTool::ArcCenter
                | SketchTool::ArcTangent
                | SketchTool::Polygon
                | SketchTool::Point
        ) && self.sketch.as_ref().is_some_and(|e| e.auto)
        {
            format!("{} {}", prompt, tr("(Hold Alt: no constraints.)"))
        } else {
            prompt.to_string()
        };
        let a = self.sketch_analysis()?;
        let empty = self.sketch_obj()?.sketch().points.is_empty();
        let state = if empty {
            tr("empty").to_string()
        } else if !a.conflicting.is_empty() {
            fill("{} constraints in conflict (red)", &[&a.conflicting.len()])
        } else if a.free == 0 {
            tr("fully constrained").to_string()
        } else {
            fill("{} free motions", &[&a.free])
        };
        Some((prompt, state))
    }
}

/// A constraint as text: its name and its value.
fn constraint_text(units: crate::workspace::Units, c: &Constraint) -> String {
    match c.value() {
        Some(v) if matches!(c, Constraint::Angle { .. }) => format!("{} {v:.1}°", tr(c.name())),
        Some(v) => format!("{} {}", tr(c.name()), units.fmt(v.abs())),
        None => tr(c.name()).to_string(),
    }
}

/// The units of plain numbers in expressions: the units of the Options
/// when the sketch gets its first parameter or expression.
fn first_units(sk: &mut Sketch, units: crate::workspace::Units) {
    if sk.parameters.is_empty() && sk.exprs.is_empty() {
        sk.units = match units {
            crate::workspace::Units::Mm => emb_sketch::Units::Mm,
            crate::workspace::Units::Inch => emb_sketch::Units::Inch,
        };
    }
}

/// The message of an error of an expression, translated.
pub fn expr_message(e: &emb_sketch::ExprError) -> String {
    let (t, v) = e.parts();
    let text = match v {
        Some(v) => fill(t, &[&v]),
        None => tr(t).to_string(),
    };
    let mut c = text.chars();
    match c.next() {
        Some(f) => format!("{}{}.", f.to_uppercase(), c.as_str()),
        None => text,
    }
}

/// The text of a value box: the expression of the dimension, else its
/// value.
fn box_text(units: crate::workspace::Units, s: &Sketch, id: ConstraintId, c: &Constraint) -> String {
    match s.dimension_expr(id) {
        Some(e) => e.to_string(),
        None => value_text(units, c),
    }
}

/// A value of a parameter as text: a length in `units`, else a number.
fn quantity_text(q: emb_sketch::expr::Quantity, units: crate::workspace::Units, scale: f64) -> String {
    match q.length {
        1 => units.fmt(q.value * scale),
        0 => {
            let t = format!("{:.4}", q.value);
            t.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        n => format!("{:.2} mm^{n}", q.value),
    }
}

/// The Parameters window of the open sketch (ADR 0078): the named values
/// and their expressions.
fn parameters_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.sketch.as_ref().is_some_and(|e| e.params.open) {
        return;
    }
    let Some(s) = st.sketch_obj().map(|o| o.sketch().clone()) else {
        return;
    };
    let units = st.settings.units;
    let values = s.parameter_values().unwrap_or_default();
    let mut open = true;
    let mut action = None;
    let id = egui::Id::new("Sketch Parameters");
    let rect = st.canvas_rect;
    let Some(e) = st.sketch.as_mut() else { return };
    let ui_state = &mut e.params;
    crate::dock::docked(egui::Window::new(tr("Parameters")).id(id), ctx, id, rect)
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.weak(tr(
                "Named values for the dimensions. Type a name or an expression (for example width / 20) in the value box of a dimension.",
            ));
            egui::Grid::new("sketch_params").num_columns(4).striped(true).show(ui, |ui| {
                ui.strong(tr("Name"));
                ui.strong(tr("Expression"));
                ui.strong(tr("Value"));
                ui.end_row();
                for p in &s.parameters {
                    let name = ui_state.names.entry(p.name.clone()).or_insert_with(|| p.name.clone());
                    let r = ui.add(egui::TextEdit::singleline(name).desired_width(90.0));
                    if r.lost_focus() && *name != p.name {
                        action = Some(ParamAction::Rename(p.name.clone(), name.clone()));
                    }
                    let text = ui_state.exprs.entry(p.name.clone()).or_insert_with(|| p.expr.clone());
                    let r = ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                    if r.lost_focus() && *text != p.expr {
                        action = Some(ParamAction::Set(p.name.clone(), text.clone()));
                    }
                    ui.label(
                        values
                            .get(&p.name)
                            .map_or_else(|| "—".to_string(), |q| quantity_text(*q, units, s.expr_scale())),
                    );
                    if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the parameter")).clicked() {
                        action = Some(ParamAction::Remove(p.name.clone()));
                    }
                    ui.end_row();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut ui_state.new_name)
                        .hint_text(tr("width"))
                        .desired_width(90.0),
                );
                let r = ui.add(
                    egui::TextEdit::singleline(&mut ui_state.new_expr)
                        .hint_text(tr("80 mm"))
                        .desired_width(140.0),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button(tr("Add")).clicked() || enter) && !ui_state.new_name.trim().is_empty() {
                    action = Some(ParamAction::Set(
                        std::mem::take(&mut ui_state.new_name),
                        std::mem::take(&mut ui_state.new_expr),
                    ));
                }
                ui.end_row();
            });
            ui.horizontal(|ui| {
                ui.label(tr("Numbers with no unit are in"));
                for (u, name) in [(emb_sketch::Units::Mm, "mm"), (emb_sketch::Units::Inch, "in")] {
                    if ui.selectable_label(s.units == u, name).clicked() && s.units != u {
                        action = Some(ParamAction::Units(u));
                    }
                }
            });
            if (s.expr_scale() - 1.0).abs() > 1e-9 {
                ui.weak(fill(
                    "The sketch was scaled: the lengths of the expressions are × {}.",
                    &[&format!("{:.3}", s.expr_scale())],
                ));
            }
            if let Some(err) = &ui_state.error {
                ui.colored_label(ui.visuals().warn_fg_color, err);
            }
        });
    if !open && let Some(e) = &mut st.sketch {
        e.params.open = false;
    }
    if let Some(a) = action {
        st.param_action(a);
    }
}

/// The value of a dimension as text for the value box: an angle in
/// degrees, a size in the units of the Options.
fn value_text(units: crate::workspace::Units, c: &Constraint) -> String {
    let v = c.value().unwrap_or(0.0);
    match c {
        Constraint::Angle { .. } => format!("{v:.2}"),
        _ => {
            let x = units.of_mm(v.abs());
            if units == crate::workspace::Units::Mm {
                format!("{x:.2}")
            } else {
                format!("{x:.3}")
            }
        }
    }
}

/// The colors of the parts of a sketch.
fn colors(dark: bool) -> (Color32, Color32, Color32, Color32) {
    let free = if dark {
        Color32::from_rgb(110, 180, 255)
    } else {
        Color32::from_rgb(20, 100, 210)
    };
    let fixed = if dark {
        Color32::from_rgb(120, 210, 120)
    } else {
        Color32::from_rgb(20, 140, 40)
    };
    let conflict = Color32::from_rgb(230, 60, 50);
    let construction = Color32::from_rgb(230, 150, 40);
    (free, fixed, conflict, construction)
}

/// The label of a constraint on the canvas: where, the text, and the
/// points of its leader line (for a dimension).
struct Label {
    id: ConstraintId,
    rect: Rect,
    text: String,
    leader: Option<(Pos2, Pos2)>,
    /// The extension lines of a length or a distance.
    ext: Option<[(Pos2, Pos2); 2]>,
    dimension: bool,
}

/// The short mark of a constraint that is not a dimension.
fn mark(c: &Constraint) -> &'static str {
    match c {
        Constraint::Horizontal { .. } | Constraint::VerticalDistance { .. } => "H",
        Constraint::Vertical { .. } | Constraint::HorizontalDistance { .. } => "V",
        Constraint::Parallel { .. } => "//",
        Constraint::Perpendicular { .. } => "⊥",
        Constraint::Collinear { .. } => "C",
        Constraint::Tangent { .. } => "T",
        Constraint::Equal { .. } => "=",
        Constraint::Concentric { .. } => "◎",
        Constraint::Midpoint { .. } => "M",
        Constraint::OnCurve { .. } => "O",
        Constraint::Symmetric { .. } => "S",
        Constraint::Fix { .. } => "F",
        _ => "",
    }
}

/// The labels of the constraints of the open sketch.
fn labels(obj: &SketchObject, map: &ViewMap, units: crate::workspace::Units) -> Vec<Label> {
    labels_with(obj, map, units, true)
}

/// The labels; with `marks` false only the dimensions.
fn labels_with(obj: &SketchObject, map: &ViewMap, units: crate::workspace::Units, marks: bool) -> Vec<Label> {
    let s = obj.sketch();
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let pt = |id: PointId| s.point(id).map(to);
    let mid = |c: CurveId| -> Option<Pos2> {
        match s.curve(c)?.geometry {
            Geometry::Line { a, b } => Some(pt(a)?.lerp(pt(b)?, 0.5)),
            Geometry::Circle { center, radius } => Some(to(s.point(center)? + Point::new(radius, 0.0))),
            Geometry::Arc { start, .. } => pt(start),
            Geometry::Bezier { .. } => Some(to(s.shape(c)?.at(0.5))),
        }
    };
    let mut out = Vec::new();
    // Marks of one curve stack along it.
    let mut stack: std::collections::BTreeMap<CurveId, f32> = Default::default();
    let font_h = 14.0;
    // A length or a distance goes on the side away from the middle of the
    // sketch, so it does not cover the shape.
    let middle = {
        let n = s.points.len().max(1) as f32;
        let sum = s.points.iter().fold(Vec2::ZERO, |acc, p| acc + to(p.pos).to_vec2());
        Pos2::new(sum.x / n, sum.y / n)
    };
    let away = |pa: Pos2, pb: Pos2| -> Vec2 {
        let d = (pb - pa).normalized();
        let n = Vec2::new(-d.y, d.x);
        let m = pa.lerp(pb, 0.5);
        if n.dot(m - middle) < 0.0 { -n * 22.0 } else { n * 22.0 }
    };
    let mut ext = None;
    // A label that the user put somewhere.
    let placed = |id: ConstraintId| s.label_at.get(&id).map(|p| to(*p));
    // The offset of a length from its line, and the place of its label.
    let offset = |pa: Pos2, pb: Pos2, id: ConstraintId| -> (Vec2, Pos2) {
        let m = pa.lerp(pb, 0.5);
        match placed(id) {
            Some(at) => {
                let d = (pb - pa).normalized();
                let n = Vec2::new(-d.y, d.x);
                (n * n.dot(at - m), at)
            }
            None => {
                let n = away(pa, pb);
                (n, m + n)
            }
        }
    };
    for sc in &s.constraints {
        let c = &sc.constraint;
        if !marks && !c.is_dimension() {
            continue;
        }
        let (anchor, text, leader, dimension) = match *c {
            Constraint::Coincident { .. } => continue,
            Constraint::HorizontalDistance { a, b, mm } | Constraint::VerticalDistance { a, b, mm } if mm != 0.0 => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let across = matches!(c, Constraint::HorizontalDistance { .. });
                let m = pa.lerp(pb, 0.5);
                // The dimension line is horizontal (or vertical), on the
                // side away from the middle of the sketch, or at the label.
                let at = placed(sc.id).unwrap_or_else(|| {
                    if across {
                        let y = if middle.y > m.y {
                            pa.y.min(pb.y) - 22.0
                        } else {
                            pa.y.max(pb.y) + 22.0
                        };
                        Pos2::new(m.x, y)
                    } else {
                        let x = if middle.x > m.x {
                            pa.x.min(pb.x) - 22.0
                        } else {
                            pa.x.max(pb.x) + 22.0
                        };
                        Pos2::new(x, m.y)
                    }
                });
                let (qa, qb) = if across {
                    (Pos2::new(pa.x, at.y), Pos2::new(pb.x, at.y))
                } else {
                    (Pos2::new(at.x, pa.y), Pos2::new(at.x, pb.y))
                };
                ext = Some([(pa, qa), (pb, qb)]);
                (at, units.fmt(mm.abs()), Some((qa, qb)), true)
            }
            Constraint::ArcLength { curve, mm } => {
                let Some(sh @ emb_sketch::Shape::Arc { c: cp, sweep, .. }) = s.shape(curve) else {
                    continue;
                };
                let mid = to(sh.at(sweep / 2.0));
                let out = (mid - to(cp)).normalized();
                (
                    placed(sc.id).unwrap_or(mid + out * 24.0),
                    format!("⌒ {}", units.fmt(mm)),
                    None,
                    true,
                )
            }
            Constraint::Distance { a, b, mm } => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let (n, at) = offset(pa, pb, sc.id);
                ext = Some([(pa, pa + n * 1.15), (pb, pb + n * 1.15)]);
                (at, units.fmt(mm.abs()), Some((pa + n, pb + n)), true)
            }
            Constraint::LineDistance { point, line, mm } => {
                let (Some((a, b)), Some(q)) = (ends(s, line), s.point(point)) else {
                    continue;
                };
                let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
                    continue;
                };
                let d = pb - pa;
                let foot = pa + d * ((q - pa).dot(d) / d.length_sq().max(1e-12));
                let (sq, sf) = (to(q), to(foot));
                // To a centerline: a diameter.
                let text = if s.is_centerline(line) {
                    format!("⌀ {}", units.fmt(mm.abs()))
                } else {
                    units.fmt(mm.abs())
                };
                (
                    placed(sc.id).unwrap_or(sq.lerp(sf, 0.5) + Vec2::new(10.0, 0.0)),
                    text,
                    Some((sq, sf)),
                    true,
                )
            }
            Constraint::Length { line, mm } => {
                let Some((a, b)) = ends(s, line) else { continue };
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let (n, at) = offset(pa, pb, sc.id);
                ext = Some([(pa, pa + n * 1.15), (pb, pb + n * 1.15)]);
                (at, units.fmt(mm), Some((pa + n, pb + n)), true)
            }
            Constraint::Radius { curve, mm } | Constraint::Diameter { curve, mm } => {
                let Some((center, r)) = (match s.curve(curve).map(|c| c.geometry) {
                    Some(Geometry::Circle { center, radius } | Geometry::Arc { center, radius, .. }) => {
                        Some((center, radius))
                    }
                    _ => None,
                }) else {
                    continue;
                };
                let Some(cp) = s.point(center) else { continue };
                // An arc: the label at the middle of the arc, out of it.
                let dir = match s.shape(curve) {
                    Some(sh @ emb_sketch::Shape::Arc { sweep, .. }) => (sh.at(sweep / 2.0) - cp).normalized(),
                    _ => None,
                }
                .unwrap_or(Point::new(
                    std::f64::consts::FRAC_1_SQRT_2,
                    -std::f64::consts::FRAC_1_SQRT_2,
                ));
                let edge = to(cp + dir * r);
                let (edge, at) = match placed(sc.id) {
                    // The leader goes to the label.
                    Some(at) => {
                        let rs = edge.distance(to(cp));
                        (to(cp) + (at - to(cp)).normalized() * rs, at)
                    }
                    None => (edge, edge + (edge - to(cp)).normalized() * 26.0),
                };
                let prefix = if matches!(c, Constraint::Diameter { .. }) {
                    "⌀"
                } else {
                    "R"
                };
                (at, format!("{prefix} {}", units.fmt(mm)), Some((to(cp), edge)), true)
            }
            Constraint::Angle { a, b, deg } => {
                let (Some(ma), Some(mb)) = (mid(a), mid(b)) else {
                    continue;
                };
                (
                    placed(sc.id).unwrap_or(ma.lerp(mb, 0.5)),
                    format!("{deg:.1}°"),
                    None,
                    true,
                )
            }
            Constraint::Fix { point, .. } | Constraint::Midpoint { point, .. } | Constraint::OnCurve { point, .. } => {
                let Some(p) = pt(point) else { continue };
                (p + Vec2::new(10.0, -12.0), mark(c).to_string(), None, false)
            }
            Constraint::Symmetric { a, .. } => {
                let Some(p) = pt(a) else { continue };
                (p + Vec2::new(10.0, -12.0), mark(c).to_string(), None, false)
            }
            Constraint::HorizontalDistance { a, b, .. } | Constraint::VerticalDistance { a, b, .. } => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                (
                    pa.lerp(pb, 0.5) + Vec2::new(8.0, -10.0),
                    mark(c).to_string(),
                    None,
                    false,
                )
            }
            _ => {
                let curves = c.curves();
                let Some(&first) = curves.first() else { continue };
                let Some(m) = mid(first) else { continue };
                let k = stack.entry(first).or_insert(0.0);
                let at = m + Vec2::new(14.0 + *k, 14.0);
                *k += 16.0;
                (at, mark(c).to_string(), None, false)
            }
        };
        // A driven dimension in parentheses.
        let text = if sc.driven { format!("({text})") } else { text };
        // A dimension with an expression: its parameter, or ƒx (ADR 0078).
        let text = match s.dimension_expr(sc.id) {
            Some(e) if emb_sketch::expr::valid_name(e) => format!("{e} = {text}"),
            Some(_) => format!("ƒx {text}"),
            None => text,
        };
        let w = 8.0 * text.chars().count() as f32 + 6.0;
        let rect = Rect::from_center_size(anchor, Vec2::new(w, font_h + 2.0));
        out.push(Label {
            id: sc.id,
            rect,
            text,
            leader,
            ext: ext.take(),
            dimension,
        });
    }
    out
}

/// Draws a label: its leader and extension lines, its box and its text.
/// `alpha` below 1 draws a ghost (the dimension that a click makes).
#[allow(clippy::too_many_arguments)]
fn draw_label(
    painter: &egui::Painter,
    l: &Label,
    color: Color32,
    fill: Color32,
    text_color: Color32,
    frame: Stroke,
    alpha: f32,
) {
    let fade = |c: Color32| c.gamma_multiply(alpha);
    if let Some((p, q)) = l.leader {
        painter.line_segment([p, q], Stroke::new(1.0, fade(color.gamma_multiply(0.7))));
    }
    for (p, q) in l.ext.iter().flatten() {
        painter.line_segment([*p, *q], Stroke::new(0.8, fade(color.gamma_multiply(0.45))));
    }
    painter.rect_filled(l.rect, 3.0, fade(fill));
    painter.rect_stroke(
        l.rect,
        3.0,
        Stroke::new(frame.width, fade(frame.color)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        l.rect.center(),
        egui::Align2::CENTER_CENTER,
        &l.text,
        egui::FontId::proportional(13.0),
        fade(text_color),
    );
}

/// The part of the open sketch at screen position `pos`.
fn pick_at(st: &AppState, map: &ViewMap, pos: Pos2) -> Option<Pick> {
    let obj = st.sketch_obj()?;
    let s = obj.sketch();
    let marks = st.sketch.as_ref().is_none_or(|e| e.show_marks);
    for l in labels_with(obj, map, st.settings.units, marks) {
        if l.rect.expand(if l.dimension { 4.0 } else { 0.0 }).contains(pos) {
            return Some(Pick::Constraint(l.id));
        }
    }
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let point = s
        .points
        .iter()
        .map(|p| (p.id, to(p.pos).distance(pos)))
        .filter(|(_, d)| *d <= PICK_PX)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((id, _)) = point {
        return Some(Pick::Point(id));
    }
    let tol = f64::from(PICK_PX) / map.view.scale();
    let at = map.to_design(pos);
    obj.curve_paths()
        .iter()
        .find(|(_, path)| {
            path.flatten(tol / 4.0)
                .iter()
                .any(|pl| pl.segments().any(|(a, b)| at.distance_to_segment(a, b) <= tol))
        })
        .map(|(id, _)| Pick::Curve(*id))
}

/// The parts of the sketch inside screen rectangle `r`: the points, the
/// curves with all their points inside, and the labels of constraints.
fn box_picks(obj: &SketchObject, map: &ViewMap, units: crate::workspace::Units, r: Rect) -> Vec<Pick> {
    let s = obj.sketch();
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    let mut out: Vec<Pick> = Vec::new();
    for (id, path) in obj.curve_paths() {
        let all_in = path
            .flatten(tol)
            .iter()
            .all(|pl| pl.points.iter().all(|p| r.contains(map.to_screen(*p))));
        if all_in {
            out.push(Pick::Curve(*id));
        }
    }
    for p in &s.points {
        if r.contains(to(p.pos)) && !s.curves.iter().any(|c| c.points().contains(&p.id)) {
            out.push(Pick::Point(p.id));
        }
    }
    for l in labels(obj, map, units) {
        if r.contains_rect(l.rect) {
            out.push(Pick::Constraint(l.id));
        }
    }
    out
}

/// Relax mode: the driving dimensions are driven for a drag, so the drag
/// changes them. Returns them, for `restore_dims`.
fn relax_dims(sk: &mut Sketch, on: bool) -> Vec<ConstraintId> {
    if !on {
        return Vec::new();
    }
    let dims: Vec<ConstraintId> = sk
        .constraints
        .iter()
        .filter(|c| c.constraint.is_dimension() && !c.driven)
        .map(|c| c.id)
        .collect();
    for c in &mut sk.constraints {
        if dims.contains(&c.id) {
            c.driven = true;
        }
    }
    dims
}

/// The dimensions of `relax_dims` drive again, with the sizes of now.
fn restore_dims(sk: &mut Sketch, dims: &[ConstraintId]) {
    for d in dims {
        sk.set_driven(*d, false);
    }
}

/// The tolerance of a click in sketch mm.
fn pick_mm(map: &ViewMap) -> f64 {
    f64::from(PICK_PX) / map.view.scale()
}

/// The snap of a click at screen position `pos`: a point or a curve of the
/// sketch near it. With `alt`, no snap.
fn snap_at(st: &AppState, map: &ViewMap, pos: Pos2, alt: bool) -> Option<Snap> {
    let obj = st.sketch_obj()?;
    let at = obj.to_sketch(map.to_design(pos));
    if alt {
        return Some(Snap::free(at));
    }
    Some(infer::snap(obj.sketch(), at, pick_mm(map), &[]))
}

/// The center of the circle through three points.
fn circle_through(a: Point, b: Point, c: Point) -> Option<Point> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.length_sq(), b.length_sq(), c.length_sq());
    Some(Point::new(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    ))
}

/// The geometry that a draw tool makes from its clicks; `None` if it needs
/// more clicks. A click that snapped to a point uses it; a click on a curve
/// puts its new point on the curve. It returns the label of the undo step.
/// The options of the draw tools.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MakeOpts {
    sides: u32,
    /// A polygon round its circle (the sides touch it), else in it.
    outside: bool,
    construction: bool,
    slot: SlotKind,
}

impl MakeOpts {
    fn of(e: &SketchEdit) -> MakeOpts {
        MakeOpts {
            sides: e.polygon_sides,
            outside: e.polygon_outside,
            construction: e.construction,
            slot: e.slot_kind,
        }
    }
}

/// A straight slot round centers `c1` and `c2`, as wide as twice the
/// distance of `w` from the line between them: the two sides, the two half
/// circles, and the line between the centers (construction).
fn straight_slot(sk: &mut Sketch, c1: &Snap, c2: &Snap, w: Point) -> Option<(Vec<CurveId>, CurveId)> {
    let place = infer::place;
    let dir = (c2.at - c1.at).normalized()?;
    let n = dir.perp();
    let r = (w - c1.at).dot(n).abs();
    if r < 1e-6 {
        return None;
    }
    let a = place(sk, c1);
    let b = place(sk, c2);
    if a == b {
        return None;
    }
    let (pa, pb) = (sk.point(a)?, sk.point(b)?);
    let p1 = sk.add_point(pa + n * r);
    let p2 = sk.add_point(pb + n * r);
    let p3 = sk.add_point(pb - n * r);
    let p4 = sk.add_point(pa - n * r);
    let top = sk.add_line_between(p1, p2).ok()?;
    let bottom = sk.add_line_between(p3, p4).ok()?;
    // Round the far side of each center, counterclockwise.
    let end_b = sk.add_arc_between(b, p3, p2, r).ok()?;
    let end_a = sk.add_arc_between(a, p1, p4, r).ok()?;
    for (l, arc) in [(top, end_a), (top, end_b), (bottom, end_a), (bottom, end_b)] {
        let _ = sk.add_constraint(Constraint::Tangent {
            a: l,
            b: arc,
            inside: false,
        });
    }
    let _ = sk.add_constraint(Constraint::Equal { a: end_a, b: end_b });
    // The line between the centers (construction).
    let axis = sk.add_line_between(a, b).ok()?;
    sk.set_construction(axis, true);
    let (_, hints) = infer::line_hints(sk, &Snap::free(pa), &Snap::free(pb));
    infer::apply_hints(sk, axis, &hints);
    Some((vec![top, bottom, end_a, end_b], axis))
}

/// A curved slot: its path is the arc round `o` with radius `big_r` from
/// angle `a0` to `a1` (counterclockwise), `r` the half width. Two arcs on
/// the sides (round `o`), two half circles at the ends, tangent to them,
/// and the path as a construction arc.
fn arc_slot(sk: &mut Sketch, o: Point, big_r: f64, a0: f64, a1: f64, r: f64) -> Option<Vec<CurveId>> {
    if !(r > 1e-6 && r < big_r - 1e-6) {
        return None;
    }
    let u = |a: f64| Point::new(math::cos(a), math::sin(a));
    let oc = sk.add_point(o);
    let s = sk.add_point(o + u(a0) * big_r);
    let e = sk.add_point(o + u(a1) * big_r);
    let path = sk.add_arc_between(oc, s, e, big_r).ok()?;
    sk.set_construction(path, true);
    let po_s = sk.add_point(o + u(a0) * (big_r + r));
    let pi_s = sk.add_point(o + u(a0) * (big_r - r));
    let po_e = sk.add_point(o + u(a1) * (big_r + r));
    let pi_e = sk.add_point(o + u(a1) * (big_r - r));
    let outer = sk.add_arc_between(oc, po_s, po_e, big_r + r).ok()?;
    let inner = sk.add_arc_between(oc, pi_s, pi_e, big_r - r).ok()?;
    // The ends go round beyond the path.
    let cap_e = sk.add_arc_between(e, po_e, pi_e, r).ok()?;
    let cap_s = sk.add_arc_between(s, pi_s, po_s, r).ok()?;
    for (side, cap, inside) in [
        (outer, cap_e, true),
        (outer, cap_s, true),
        (inner, cap_e, false),
        (inner, cap_s, false),
    ] {
        let _ = sk.add_constraint(Constraint::Tangent {
            a: side,
            b: cap,
            inside,
        });
    }
    let _ = sk.add_constraint(Constraint::Equal { a: cap_s, b: cap_e });
    Some(vec![outer, inner, cap_s, cap_e])
}

/// The four sides of a rectangle with corners `ids` (in order), with the
/// constraints `c` of the sides 0 and 1 (the others are parallel).
fn rectangle_sides(sk: &mut Sketch, ids: [PointId; 4]) -> Vec<CurveId> {
    (0..4)
        .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % 4]).ok())
        .collect()
}

fn make(tool: SketchTool, sk: &mut Sketch, clicks: &[Snap], opts: MakeOpts) -> Option<&'static str> {
    let (sides, construction) = (opts.sides, opts.construction);
    let place = infer::place;
    let mut made: Vec<CurveId> = Vec::new();
    let label = match (tool, clicks) {
        (SketchTool::Point, [a]) => {
            place(sk, a);
            "Sketch: Point"
        }
        (SketchTool::Line, [.., a, b]) => {
            let pa = place(sk, a);
            let pb = place(sk, b);
            if pa == pb {
                return None;
            }
            made.extend(sk.add_line_between(pa, pb).ok());
            "Sketch: Line"
        }
        (SketchTool::Rectangle, [a, b]) => {
            let (pa, pb) = (a.at, b.at);
            if (pa.x - pb.x).abs() < 1e-6 || (pa.y - pb.y).abs() < 1e-6 {
                return None;
            }
            let p0 = place(sk, a);
            let p1 = sk.add_point(Point::new(pb.x, pa.y));
            let p2 = place(sk, b);
            let p3 = sk.add_point(Point::new(pa.x, pb.y));
            let ids = [p0, p1, p2, p3];
            let lines: Vec<CurveId> = (0..4)
                .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % 4]).ok())
                .collect();
            for (i, l) in lines.iter().enumerate() {
                let c = if i % 2 == 0 {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                };
                let _ = sk.add_constraint(c);
            }
            made.extend(lines);
            "Sketch: Rectangle"
        }
        (SketchTool::RectCenter, [c, k]) => {
            let (cp, kp) = (c.at, k.at);
            if (cp.x - kp.x).abs() < 1e-6 || (cp.y - kp.y).abs() < 1e-6 {
                return None;
            }
            let o = cp * 2.0 - kp;
            let p0 = place(sk, k);
            let ids = [
                p0,
                sk.add_point(Point::new(o.x, kp.y)),
                sk.add_point(o),
                sk.add_point(Point::new(kp.x, o.y)),
            ];
            let lines = rectangle_sides(sk, ids);
            for (i, l) in lines.iter().enumerate() {
                let c = if i % 2 == 0 {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                };
                let _ = sk.add_constraint(c);
            }
            // The center on the diagonals (construction lines).
            let center = place(sk, c);
            for (a, b) in [(ids[0], ids[2]), (ids[1], ids[3])] {
                if let Ok(d) = sk.add_line_between(a, b) {
                    sk.set_construction(d, true);
                    let _ = sk.add_constraint(Constraint::Midpoint { point: center, line: d });
                    break;
                }
            }
            if let Ok(d) = sk.add_line_between(ids[1], ids[3]) {
                sk.set_construction(d, true);
            }
            made.extend(lines);
            "Sketch: Center Rectangle"
        }
        (SketchTool::Rect3, [a, b, c]) => {
            let dir = (b.at - a.at).normalized()?;
            let n = dir.perp();
            let h = (c.at - b.at).dot(n);
            if h.abs() < 1e-6 {
                return None;
            }
            let p0 = place(sk, a);
            let p1 = place(sk, b);
            if p0 == p1 {
                return None;
            }
            let (q1, q0) = (sk.point(p1)?, sk.point(p0)?);
            let ids = [p0, p1, sk.add_point(q1 + n * h), sk.add_point(q0 + n * h)];
            let lines = rectangle_sides(sk, ids);
            if let [l0, l1, l2, l3] = lines.as_slice() {
                // The first side horizontal or vertical when it is almost.
                let (_, hints) = infer::line_hints(sk, &Snap::free(q0), &Snap::free(q1));
                infer::apply_hints(sk, *l0, &hints);
                for c in [
                    Constraint::Perpendicular { a: *l0, b: *l1 },
                    Constraint::Parallel { a: *l0, b: *l2 },
                    Constraint::Parallel { a: *l1, b: *l3 },
                ] {
                    let _ = sk.add_constraint(c);
                }
            }
            made.extend(lines);
            "Sketch: Three-Point Rectangle"
        }
        (SketchTool::Slot, [c1, c2, w]) if !opts.slot.is_arc() => {
            match opts.slot {
                SlotKind::Overall => {
                    // The ends: the centers are a half width in.
                    let dir = (c2.at - c1.at).normalized()?;
                    let r = (w.at - c1.at).dot(dir.perp()).abs();
                    if c1.at.distance(c2.at) <= 2.0 * r + 1e-6 {
                        return None;
                    }
                    let (a, b) = (Snap::free(c1.at + dir * r), Snap::free(c2.at - dir * r));
                    let (curves, _) = straight_slot(sk, &a, &b, w.at)?;
                    made.extend(curves);
                }
                SlotKind::CenterPoint => {
                    // The middle and one center: the other center is opposite.
                    let other = Snap::free(c1.at * 2.0 - c2.at);
                    let (curves, axis) = straight_slot(sk, &other, c2, w.at)?;
                    let mid = place(sk, c1);
                    let _ = sk.add_constraint(Constraint::Midpoint { point: mid, line: axis });
                    made.extend(curves);
                }
                _ => {
                    let (curves, _) = straight_slot(sk, c1, c2, w.at)?;
                    made.extend(curves);
                }
            }
            "Sketch: Slot"
        }
        (SketchTool::Slot, [a, b, m, w]) if opts.slot.is_arc() => {
            // The arc of the path, counterclockwise.
            let (o, s0, e0) = if opts.slot == SlotKind::Arc3 {
                let o = circle_through(a.at, b.at, m.at)?;
                let ccw = (b.at - a.at).cross(m.at - a.at) < 0.0;
                if ccw { (o, a.at, b.at) } else { (o, b.at, a.at) }
            } else {
                (a.at, b.at, m.at)
            };
            let big_r = o.distance(s0);
            let a0 = math::atan2(s0.y - o.y, s0.x - o.x);
            let mut a1 = math::atan2(e0.y - o.y, e0.x - o.x);
            while a1 <= a0 + 1e-9 {
                a1 += std::f64::consts::TAU;
            }
            let r = (w.at.distance(o) - big_r).abs();
            made.extend(arc_slot(sk, o, big_r, a0, a1, r)?);
            "Sketch: Slot"
        }
        (SketchTool::RectCenter3, [c, a, b]) => {
            let dir = (a.at - c.at).normalized()?;
            let half_w = c.at.distance(a.at);
            let n = dir.perp();
            let half_h = (b.at - c.at).dot(n).abs();
            if half_h < 1e-6 {
                return None;
            }
            let corners = [
                c.at + dir * half_w + n * half_h,
                c.at - dir * half_w + n * half_h,
                c.at - dir * half_w - n * half_h,
                c.at + dir * half_w - n * half_h,
            ];
            let ids = corners.map(|q| sk.add_point(q));
            let lines = rectangle_sides(sk, ids);
            if let [l0, l1, l2, l3] = lines.as_slice() {
                let (_, hints) = infer::line_hints(sk, &Snap::free(corners[0]), &Snap::free(corners[1]));
                infer::apply_hints(sk, *l0, &hints);
                for k in [
                    Constraint::Perpendicular { a: *l0, b: *l1 },
                    Constraint::Parallel { a: *l0, b: *l2 },
                    Constraint::Parallel { a: *l1, b: *l3 },
                ] {
                    let _ = sk.add_constraint(k);
                }
            }
            // The center on a diagonal (construction lines).
            let center = place(sk, c);
            if let Ok(d) = sk.add_line_between(ids[0], ids[2]) {
                sk.set_construction(d, true);
                let _ = sk.add_constraint(Constraint::Midpoint { point: center, line: d });
            }
            if let Ok(d) = sk.add_line_between(ids[1], ids[3]) {
                sk.set_construction(d, true);
            }
            made.extend(lines);
            "Sketch: Three-Point Center Rectangle"
        }
        (SketchTool::TangentCircle, [a, b, c]) => {
            let lines = [a.curve?, b.curve?, c.curve?];
            if lines.iter().any(|l| !is_line(sk, *l)) {
                return None;
            }
            let near = Point::new((a.at.x + b.at.x + c.at.x) / 3.0, (a.at.y + b.at.y + c.at.y) / 3.0);
            made.push(sk.add_tangent_circle(lines, near).ok()?);
            "Sketch: Tangent Circle"
        }
        (SketchTool::Circle, [c, e]) => {
            let r = c.at.distance(e.at);
            if r < 1e-6 {
                return None;
            }
            let center = place(sk, c);
            made.push(sk.add_circle_on(center, r).ok()?);
            "Sketch: Circle"
        }
        (SketchTool::Arc3, [a, b, m]) => {
            let c = circle_through(a.at, b.at, m.at)?;
            // Counterclockwise from start to end through the middle point.
            let ccw = (b.at - a.at).cross(m.at - a.at) < 0.0;
            let (s0, e0) = if ccw { (a, b) } else { (b, a) };
            let r = c.distance(s0.at);
            let ps = place(sk, s0);
            let pe = place(sk, e0);
            let center = sk.add_point(c);
            made.push(sk.add_arc_between(center, ps, pe, r).ok()?);
            "Sketch: Arc"
        }
        (SketchTool::ArcCenter, [c, a, b]) => {
            let r = c.at.distance(a.at);
            if r < 1e-6 {
                return None;
            }
            let center = place(sk, c);
            let ps = place(sk, a);
            // A free end goes on the circle.
            let end = if b.point.is_some() {
                *b
            } else {
                let dir = (b.at - c.at).normalized()?;
                Snap {
                    at: c.at + dir * r,
                    ..*b
                }
            };
            let pe = place(sk, &end);
            made.push(sk.add_arc_between(center, ps, pe, r).ok()?);
            "Sketch: Arc"
        }
        (SketchTool::ArcTangent, [a, b]) => {
            let from = a.point?;
            // The line that ends at the first point: the arc goes on along it.
            let line = sk.curves.iter().find(|c| match c.geometry {
                Geometry::Line { a, b } => a == from || b == from,
                _ => false,
            })?;
            let (la, lb) = match line.geometry {
                Geometry::Line { a, b } => (a, b),
                _ => return None,
            };
            let line_id = line.id;
            let other = if la == from { lb } else { la };
            let dir = (a.at - sk.point(other)?).normalized()?;
            let normal = dir.perp();
            // The center is on the normal at a: |c − a| = |c − b|.
            let ab = b.at - a.at;
            let t = ab.length_sq() / (2.0 * ab.dot(normal));
            if !t.is_finite() || t.abs() > emb_sketch::MAX_MM {
                return None;
            }
            let c = a.at + normal * t;
            let pb = place(sk, b);
            let (s0, e0) = if t > 0.0 { (from, pb) } else { (pb, from) };
            let center = sk.add_point(c);
            let id = sk.add_arc_between(center, s0, e0, t.abs()).ok()?;
            let _ = sk.add_constraint(Constraint::Tangent {
                a: line_id,
                b: id,
                inside: false,
            });
            made.push(id);
            "Sketch: Tangent Arc"
        }
        (SketchTool::Polygon, [c, v]) => {
            let n = sides.clamp(3, 40) as usize;
            let (cp, vp) = (c.at, v.at);
            let r = cp.distance(vp);
            if r < 1e-6 {
                return None;
            }
            // Round the circle: the click is the middle of a side.
            let half = std::f64::consts::PI / n as f64;
            let (corner_r, a0) = if opts.outside {
                (r / math::cos(half), math::atan2(vp.y - cp.y, vp.x - cp.x) + half)
            } else {
                (r, math::atan2(vp.y - cp.y, vp.x - cp.x))
            };
            let ids: Vec<PointId> = (0..n)
                .map(|i| {
                    let a = a0 + std::f64::consts::TAU * i as f64 / n as f64;
                    sk.add_point(Point::new(
                        cp.x + corner_r * math::cos(a),
                        cp.y + corner_r * math::sin(a),
                    ))
                })
                .collect();
            let lines: Vec<CurveId> = (0..n)
                .filter_map(|i| sk.add_line_between(ids[i], ids[(i + 1) % n]).ok())
                .collect();
            // The corners on a construction circle, the sides equal.
            let center = place(sk, c);
            if let Ok(ring) = sk.add_circle_on(center, r) {
                sk.set_construction(ring, true);
                if opts.outside {
                    for l in &lines {
                        let _ = sk.add_constraint(Constraint::Tangent {
                            a: *l,
                            b: ring,
                            inside: false,
                        });
                    }
                } else {
                    for p in &ids {
                        let _ = sk.add_constraint(Constraint::OnCurve { point: *p, curve: ring });
                    }
                }
            }
            for w in lines.windows(2) {
                let _ = sk.add_constraint(Constraint::Equal { a: w[0], b: w[1] });
            }
            made.extend(lines);
            "Sketch: Polygon"
        }
        _ => return None,
    };
    for c in made {
        sk.set_construction(c, construction);
    }
    Some(label)
}

/// The number of clicks that a draw tool needs.
fn clicks_needed(tool: SketchTool, opts: &MakeOpts) -> usize {
    match tool {
        SketchTool::Point => 1,
        SketchTool::Line
        | SketchTool::Rectangle
        | SketchTool::RectCenter
        | SketchTool::Circle
        | SketchTool::Polygon
        | SketchTool::ArcTangent => 2,
        SketchTool::Slot if opts.slot.is_arc() => 4,
        SketchTool::Arc3
        | SketchTool::ArcCenter
        | SketchTool::Rect3
        | SketchTool::Slot
        | SketchTool::RectCenter3
        | SketchTool::TangentCircle => 3,
        _ => usize::MAX,
    }
}

/// The kind of a length or a distance by the place of its label `at`: above
/// or below the two points a horizontal distance, left or right of them a
/// vertical distance, else along them.
fn orient(s: &Sketch, c: Constraint, at: Point) -> Constraint {
    let (a, b) = match c {
        Constraint::Distance { a, b, .. }
        | Constraint::HorizontalDistance { a, b, .. }
        | Constraint::VerticalDistance { a, b, .. } => (a, b),
        Constraint::Length { line, .. } => match ends(s, line) {
            Some(e) => e,
            None => return c,
        },
        _ => return c,
    };
    let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
        return c;
    };
    let d = pb - pa;
    // A line that is horizontal or vertical has one kind only.
    if d.x.abs() < 1e-6 || d.y.abs() < 1e-6 {
        return c;
    }
    let in_x = at.x > pa.x.min(pb.x) && at.x < pa.x.max(pb.x);
    let in_y = at.y > pa.y.min(pb.y) && at.y < pa.y.max(pb.y);
    match (in_x, in_y) {
        (true, false) => Constraint::HorizontalDistance { a, b, mm: d.x },
        (false, true) => Constraint::VerticalDistance { a, b, mm: d.y },
        _ => match c {
            Constraint::HorizontalDistance { .. } | Constraint::VerticalDistance { .. } => Constraint::Distance {
                a,
                b,
                mm: pa.distance(pb),
            },
            other => other,
        },
    }
}

/// The other kinds of a pending dimension (for the right-click menu): the
/// name and the dimension.
fn dimension_kinds(s: &Sketch, c: &Constraint) -> Vec<(&'static str, Constraint)> {
    match *c {
        Constraint::Radius { curve, .. } | Constraint::Diameter { curve, .. } | Constraint::ArcLength { curve, .. } => {
            let Some(r) = s.radius(curve) else { return Vec::new() };
            let mut out = vec![
                (tr("Radius"), Constraint::Radius { curve, mm: r }),
                (tr("Diameter"), Constraint::Diameter { curve, mm: 2.0 * r }),
            ];
            let arc = Constraint::ArcLength { curve, mm: 0.0 };
            if let Some(len) = s.measure(&arc) {
                out.push((tr("Arc Length"), arc.with_value(len)));
            }
            out
        }
        Constraint::Distance { a, b, .. }
        | Constraint::HorizontalDistance { a, b, .. }
        | Constraint::VerticalDistance { a, b, .. } => points_kinds(s, a, b),
        Constraint::Length { line, .. } => match ends(s, line) {
            Some((a, b)) => {
                let mut out = points_kinds(s, a, b);
                if let Some(l) = s.measure(c) {
                    out[0].1 = Constraint::Length { line, mm: l };
                }
                out
            }
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn points_kinds(s: &Sketch, a: PointId, b: PointId) -> Vec<(&'static str, Constraint)> {
    let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
        return Vec::new();
    };
    vec![
        (
            tr("Aligned"),
            Constraint::Distance {
                a,
                b,
                mm: pa.distance(pb),
            },
        ),
        (
            tr("Horizontal"),
            Constraint::HorizontalDistance { a, b, mm: pb.x - pa.x },
        ),
        (tr("Vertical"), Constraint::VerticalDistance { a, b, mm: pb.y - pa.y }),
    ]
}

/// The dimension for the picks of the Dimension tool, with the value as
/// the sketch is now. A line waits for the next click: another line (an
/// angle), a point (the distance to the line) or an empty place (`empty`:
/// its length).
fn dimension_of(s: &Sketch, picks: &[Pick], empty: bool) -> Build {
    let line_distance = |p: PointId, l: CurveId| match (ends(s, l), s.point(p)) {
        (Some((a, b)), Some(q)) => match (s.point(a), s.point(b)) {
            (Some(pa), Some(pb)) if pa.distance(pb) > 1e-9 => Build::Done(Constraint::LineDistance {
                point: p,
                line: l,
                mm: (pb - pa).cross(q - pa) / pa.distance(pb) * if s.is_centerline(l) { 2.0 } else { 1.0 },
            }),
            _ => Build::Wrong(tr("No such line.")),
        },
        _ => Build::Wrong(tr("No such line.")),
    };
    match picks {
        [Pick::Curve(c)] if is_line(s, *c) && !empty => Build::More,
        [Pick::Curve(c)] => match s.curve(*c).map(|c| c.geometry) {
            Some(Geometry::Line { a, b }) => match (s.point(a), s.point(b)) {
                (Some(pa), Some(pb)) => Build::Done(Constraint::Length {
                    line: *c,
                    mm: pa.distance(pb),
                }),
                _ => Build::Wrong(tr("No such line.")),
            },
            Some(Geometry::Circle { radius, .. }) => Build::Done(Constraint::Diameter {
                curve: *c,
                mm: 2.0 * radius,
            }),
            Some(Geometry::Arc { radius, .. }) => Build::Done(Constraint::Radius { curve: *c, mm: radius }),
            Some(Geometry::Bezier { .. }) => {
                Build::Wrong(tr("A spline takes no dimension: give dimensions to its points."))
            }
            None => Build::Wrong(tr("No such curve.")),
        },
        [Pick::Point(a), Pick::Point(b)] => match (s.point(*a), s.point(*b)) {
            (Some(pa), Some(pb)) => Build::Done(Constraint::Distance {
                a: *a,
                b: *b,
                mm: pa.distance(pb),
            }),
            _ => Build::Wrong(tr("No such point.")),
        },
        [Pick::Curve(a), Pick::Curve(b)] if is_line(s, *a) && is_line(s, *b) => {
            let dir = |c: CurveId| {
                let (p, q) = ends(s, c)?;
                Some(s.point(q)? - s.point(p)?)
            };
            match (dir(*a), dir(*b)) {
                (Some(da), Some(db)) => Build::Done(Constraint::Angle {
                    a: *a,
                    b: *b,
                    deg: math::to_degrees(math::atan2(da.cross(db), da.dot(db))),
                }),
                _ => Build::Wrong(tr("No such line.")),
            }
        }
        [Pick::Point(p), Pick::Curve(l)] | [Pick::Curve(l), Pick::Point(p)] if is_line(s, *l) => line_distance(*p, *l),
        [Pick::Point(_)] => Build::More,
        [Pick::Curve(c), _] if !is_line(s, *c) => Build::Wrong(tr("Click one circle or arc for its size.")),
        _ => Build::Wrong(tr("Click a line, a circle or an arc, two points or two lines.")),
    }
}

/// The right click of sketch mode: it ends the clicks of a draw tool, or
/// it selects the part under the pointer and opens the menu of the
/// commands for the selection.
fn context_menu(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let pending = st.sketch.as_ref().is_some_and(|e| e.pending_dim.is_some());
        let busy = st
            .sketch
            .as_ref()
            .is_some_and(|e| !e.clicks.is_empty() || !e.picks.is_empty());
        if busy && !pending {
            // A right click ends a chain of lines (or the clicks of a tool).
            if let Some(e) = &mut st.sketch {
                e.clicks.clear();
                e.picks.clear();
                e.chain_start = None;
            }
            return;
        }
        let pick = pick_at(st, map, pos);
        let at = st.sketch_obj().map(|o| o.to_sketch(map.to_design(pos)));
        if let Some(e) = &mut st.sketch {
            if let Some(p) = pick
                && !e.selection.contains(&p)
            {
                e.selection = vec![p];
            }
            e.menu_at = at.map(|a| (a, pick_mm(map)));
        }
    }
    response.context_menu(|ui| sketch_menu(ui, st));
}

/// True if two constraints are the same kind on the same parts (the
/// values and the fixed place do not count).
fn same_constraint(a: &Constraint, b: &Constraint) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b) && {
        let sorted = |c: &Constraint| {
            let (mut p, mut q) = (c.points(), c.curves());
            p.sort();
            q.sort();
            (p, q)
        };
        sorted(a) == sorted(b)
    }
}

/// The commands of the right-click menu for the selection.
fn sketch_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let s = obj.sketch();
    let Some(e) = &st.sketch else { return };
    // A dimension that waits for its place: its kinds.
    if let Some(c) = e.pending_dim {
        ui.label(tr("Dimension Type"));
        for (name, k) in dimension_kinds(s, &c) {
            if ui.button(name).clicked()
                && let Some(e) = &mut st.sketch
            {
                e.pending_dim = Some(k);
                e.pending_locked = true;
            }
        }
        return;
    }
    let sel = e.selection.clone();
    let tool = e.tool;
    let (at, tol) = e.menu_at.unwrap_or((Point::new(0.0, 0.0), 0.5));
    let parts: Vec<Pick> = sel
        .iter()
        .copied()
        .filter(|p| !matches!(p, Pick::Constraint(_)))
        .collect();
    let curves: Vec<CurveId> = sel
        .iter()
        .filter_map(|p| match p {
            Pick::Curve(c) => Some(*c),
            _ => None,
        })
        .collect();
    let units = st.settings.units;
    if let [Pick::Constraint(id)] = sel.as_slice()
        && let Some(c) = s.constraint(*id).filter(|c| c.is_dimension()).copied()
    {
        let driven = s.is_driven(*id);
        if !driven
            && ui.button(tr("Change Value")).clicked()
            && let Some(e) = &mut st.sketch
        {
            e.edit_value(*id, box_text(units, s, *id, &c));
        }
        let label = if driven { tr("Make Driving") } else { tr("Make Driven") };
        if ui
            .button(label)
            .on_hover_text(tr(
                "A driven dimension only shows the size (in parentheses); it does not change the sketch.",
            ))
            .clicked()
        {
            st.toggle_driven(*id);
        }
        if s.label_at.contains_key(id) && ui.button(tr("Reset Label Place")).clicked() {
            let id = *id;
            st.edit_sketch("Move Dimension", None, |sk| {
                sk.label_at.remove(&id);
            });
        }
    }
    if !parts.is_empty() {
        // The dimension and the constraints that fit the selection.
        if let Build::Done(c) = dimension_of(s, &parts, true)
            && ui.button(tr("Dimension")).clicked()
        {
            st.add_sketch_dimension(c, None);
            if let Some(e) = &mut st.sketch {
                e.selection.clear();
            }
        }
        // Not the constraints that the parts have already.
        let fits: Vec<Kind> = Kind::ALL
            .into_iter()
            .filter(|k| match build(*k, s, &parts) {
                Build::Done(c) => !s.constraints.iter().any(|x| same_constraint(&x.constraint, &c)),
                _ => false,
            })
            .collect();
        if !fits.is_empty() {
            ui.separator();
            for k in fits {
                if ui.button(k.name()).clicked() {
                    st.apply_constraint(k, &parts);
                }
            }
        }
        ui.separator();
    }
    // The edits of one curve, at the place of the click.
    if let [c] = curves.as_slice() {
        if ui.button(tr("Trim")).clicked() {
            st.edit_click(SketchTool::Trim, at, tol);
        }
        if ui.button(tr("Split")).clicked() {
            st.edit_click(SketchTool::Split, at, tol);
        }
        if !matches!(s.curve(*c).map(|c| c.geometry), Some(Geometry::Circle { .. }))
            && ui.button(tr("Extend")).clicked()
        {
            st.edit_click(SketchTool::Extend, at, tol);
        }
    }
    // The corner of two lines.
    if let [Pick::Point(p)] = sel.as_slice() {
        let joined = s.joined(*p);
        let lines = s
            .curves
            .iter()
            .filter(|c| c.is_line() && c.points().iter().any(|q| joined.contains(q)))
            .count();
        if lines == 2
            && let Some(pos) = s.point(*p)
        {
            if ui.button(tr("Fillet")).clicked() {
                st.edit_click(SketchTool::Fillet, pos, tol);
            }
            if ui.button(tr("Chamfer")).clicked() {
                st.edit_click(SketchTool::Chamfer, pos, tol);
            }
        }
    }
    let picked = st.sketch.as_ref().map_or(0, |e| e.picked.len());
    if picked > 0 || tool == SketchTool::Regions {
        ui.label(tr("Make Embroidery"));
        for to in RegionTo::ALL {
            if ui.button(tr(to.name())).clicked() {
                st.make_picked_regions(to);
            }
        }
        ui.separator();
    }
    if !curves.is_empty() {
        if ui.button(tr("Run")).clicked() {
            st.make_curves(CurveTo::Run);
        }
        if ui.button(tr("Satin Line")).clicked() {
            let w = st.settings.defaults.column_width_mm;
            st.make_curves(CurveTo::SatinLine(w));
        }
        if curves.len() == 2 && ui.button(tr("Satin Between")).clicked() {
            st.make_satin_between();
        }
        if ui.button(tr("Construction")).clicked() {
            st.toggle_construction();
        }
        if curves.iter().all(|c| is_line(s, *c)) && ui.button(tr("Centerline")).clicked() {
            st.toggle_centerline();
        }
        for (t, name) in [
            (SketchTool::Move, tr("Move")),
            (SketchTool::Copy, tr("Copy")),
            (SketchTool::Rotate, tr("Rotate")),
            (SketchTool::Scale, tr("Scale")),
        ] {
            if ui.button(name).clicked() {
                st.set_sketch_tool(t);
            }
        }
        if ui.button(tr("Offset")).clicked() {
            st.offset_curves(&curves);
        }
        if ui.button(tr("Mirror")).clicked() {
            st.set_sketch_tool(SketchTool::Mirror);
        }
        if ui.button(tr("Rectangular Pattern")).clicked() {
            st.set_sketch_tool(SketchTool::RectPattern);
        }
        if ui.button(tr("Circular Pattern")).clicked() {
            st.set_sketch_tool(SketchTool::CircularPattern);
        }
    }
    if sel.iter().any(|p| matches!(p, Pick::Point(_))) && ui.button(tr("Center Point")).clicked() {
        st.toggle_center_point();
    }
    if !parts.is_empty() && ui.button(tr("Delete Constraints")).clicked() {
        st.remove_sketch_constraints_of_selection();
    }
    if !sel.is_empty() {
        if ui.button(tr("Delete")).clicked() {
            st.delete_sketch_selection();
        }
        ui.separator();
    }
    if tool != SketchTool::Select && ui.button(tr("Select")).clicked() {
        st.sketch_escape();
    }
    if ui.button(tr("Select All")).clicked() {
        st.sketch_select_all();
    }
    if ui.button(tr("Finish Sketch")).clicked() {
        st.finish_sketch();
    }
}

/// The keys of sketch mode (before the keys of the program): Esc ends the
/// command and goes back to Select, Delete and Backspace delete the
/// selection, Enter ends a chain of lines, Ctrl+A selects all. While the
/// value box of a dimension is open, it takes the keys (Esc closes it).
/// Returns true if the program must not use the keys of this frame.
pub fn handle_keys(ctx: &egui::Context, st: &mut AppState) -> bool {
    use egui::{Key, Modifiers};
    let Some(e) = &st.sketch else { return false };
    if e.value_edit.is_some() {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if let Some(e) = &mut st.sketch {
                e.value_edit = None;
            }
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        return true;
    }
    // Another text field (for example in the Properties panel).
    if ctx.memory(|m| m.focused()).is_some() {
        return false;
    }
    let (esc, del, enter, all, show, hide) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::Escape),
            i.consume_key(Modifiers::NONE, Key::Delete) | i.consume_key(Modifiers::NONE, Key::Backspace),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::COMMAND, Key::A),
            i.consume_key(Modifiers::NONE, Key::F8),
            i.consume_key(Modifiers::NONE, Key::F9),
        )
    });
    if (show || hide)
        && let Some(e) = &mut st.sketch
    {
        e.show_marks = show;
    }
    if esc {
        st.sketch_escape();
    }
    if del {
        st.delete_sketch_selection();
    }
    if enter && st.sketch.as_ref().is_some_and(|e| e.tool == SketchTool::Spline) {
        finish_spline(st, false);
    } else if enter && let Some(e) = &mut st.sketch {
        e.clicks.clear();
        e.chain_start = None;
    }
    if all {
        st.sketch_select_all();
    }
    false
}

/// The canvas input of sketch mode. The other objects take no clicks.
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    st.check_sketch();
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let tool = st.sketch.as_ref().map_or(SketchTool::Select, |e| e.tool);
    context_menu(response, st, map);
    let to_sketch = |pos: Pos2| obj.to_sketch(map.to_design(pos));
    let mods = ui.input(|i| i.modifiers);
    // The line that the pointer goes over while a line is drawn: the
    // parallel and perpendicular hints use it.
    if tool == SketchTool::Line
        && let Some(pos) = response.hover_pos()
        && let Some((c, _)) = obj.sketch().nearest_curve(to_sketch(pos), pick_mm(map), &[])
        && is_line(obj.sketch(), c)
        && let Some(e) = st.sketch.as_mut()
    {
        e.reference = Some(c);
    }
    let auto = st.sketch.as_ref().is_some_and(|e| e.auto);
    // Alt (or the panel setting off) stops the constraints of the drawing
    // help for this click.
    let no_help = mods.alt || !auto;

    // A double-click on a dimension: type its value.
    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(Pick::Constraint(id)) = pick_at(st, map, pos)
        && let Some(c) = obj.sketch().constraint(id).filter(|c| c.is_dimension()).copied()
        && !obj.sketch().is_driven(id)
    {
        let text = box_text(st.settings.units, obj.sketch(), id, &c);
        let e = st.sketch.as_mut().expect("in sketch mode");
        e.edit_value(id, text);
        return;
    }

    match tool {
        SketchTool::Select => {
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                let s = obj.sketch();
                let start = to_sketch(origin);
                let under = pick_at(st, map, origin);
                let sel = st.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
                let (points, radius) = match under {
                    // A drag of a dimension label moves the label.
                    Some(Pick::Constraint(id)) if s.constraint(id).is_some_and(|c| c.is_dimension()) => {
                        let e = st.sketch.as_mut().expect("in sketch mode");
                        e.drags += 1;
                        e.label_drag = Some((id, e.drags));
                        (Vec::new(), None)
                    }
                    // A drag of a part of the selection moves all of it.
                    Some(p) if sel.len() > 1 && sel.contains(&p) => {
                        let mut pts: Vec<PointId> = Vec::new();
                        for q in &sel {
                            match *q {
                                Pick::Point(id) => pts.push(id),
                                Pick::Curve(c) => pts.extend(s.curve(c).map(|c| c.points()).unwrap_or_default()),
                                Pick::Constraint(_) => {}
                            }
                        }
                        (s.with_handles(&pts), None)
                    }
                    // A point of a spline takes its handles with it.
                    Some(Pick::Point(p)) => (s.with_handles(&[p]), None),
                    Some(Pick::Curve(c)) => match s.curve(c).map(|c| c.geometry) {
                        Some(Geometry::Line { a, b }) => (vec![a, b], None),
                        Some(Geometry::Circle { .. }) => (Vec::new(), Some(c)),
                        Some(Geometry::Arc { center, start, end, .. }) => (vec![center, start, end], None),
                        Some(Geometry::Bezier { start, c1, c2, end }) => (vec![start, c1, c2, end], None),
                        None => (Vec::new(), None),
                    },
                    _ => (Vec::new(), None),
                };
                let e = st.sketch.as_mut().expect("in sketch mode");
                if !points.is_empty() || radius.is_some() {
                    e.drags += 1;
                    e.drag = Some(Drag {
                        points: points.iter().filter_map(|p| Some((*p, s.point(*p)?))).collect(),
                        start,
                        radius,
                        key: e.drags,
                    });
                } else if under.is_none() {
                    // A drag on an empty place: a box selection.
                    st.sketch.as_mut().expect("in sketch mode").boxsel = Some(origin);
                }
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some((id, key)) = st.sketch.as_ref().and_then(|e| e.label_drag)
            {
                let at = to_sketch(pos);
                st.edit_sketch("Move Dimension", Some(format!("sketch-label-{key}")), |sk| {
                    sk.label_at.insert(id, at);
                });
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(e) = st.sketch.as_mut()
                && e.label_drag.take().is_some()
            {
                st.doc.end_merge();
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(d) = st.sketch.as_ref().and_then(|e| e.drag.clone())
            {
                let now = to_sketch(pos);
                let delta = now - d.start;
                let key = Some(format!("sketch-drag-{}", d.key));
                let relax = st.sketch.as_ref().is_some_and(|e| e.relax_mode);
                if let Some(c) = d.radius {
                    st.edit_sketch("Drag in Sketch", key, |sk| {
                        let dims = relax_dims(sk, relax);
                        let center = match sk.curve(c).map(|c| c.geometry) {
                            Some(Geometry::Circle { center, .. }) => sk.point(center),
                            _ => None,
                        };
                        if let Some(cp) = center
                            && let Some(x) = sk.curves.iter_mut().find(|x| x.id == c)
                            && let Geometry::Circle { radius, .. } = &mut x.geometry
                        {
                            *radius = cp.distance(now).max(0.05);
                        }
                        sk.solve();
                        restore_dims(sk, &dims);
                    });
                } else {
                    let moves: Vec<(PointId, Point)> = d.points.iter().map(|(p, at)| (*p, *at + delta)).collect();
                    st.edit_sketch("Drag in Sketch", key, |sk| {
                        let dims = relax_dims(sk, relax);
                        sk.drag_points(&moves);
                        restore_dims(sk, &dims);
                    });
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(e) = st.sketch.as_mut()
                && e.drag.take().is_some()
            {
                st.doc.end_merge();
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(start) = st.sketch.as_mut().and_then(|e| e.boxsel.take())
                && let Some(end) = response.interact_pointer_pos()
            {
                let inside = box_picks(&obj, map, st.settings.units, Rect::from_two_pos(start, end));
                let e = st.sketch.as_mut().expect("in sketch mode");
                if !(mods.command || mods.shift) {
                    e.selection.clear();
                }
                for p in inside {
                    if !e.selection.contains(&p) {
                        e.selection.push(p);
                    }
                }
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let p = pick_at(st, map, pos);
                let e = st.sketch.as_mut().expect("in sketch mode");
                match p {
                    Some(p) if mods.command || mods.shift => {
                        if let Some(i) = e.selection.iter().position(|x| *x == p) {
                            e.selection.remove(i);
                        } else {
                            e.selection.push(p);
                        }
                    }
                    Some(p) => e.selection = vec![p],
                    None => e.selection.clear(),
                }
            }
        }
        SketchTool::Constrain(kind) => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(p) = pick_at(st, map, pos).filter(|p| !matches!(p, Pick::Constraint(_)))
            {
                let mut picks = st.sketch.as_ref().map(|e| e.picks.clone()).unwrap_or_default();
                picks.push(p);
                if !st.apply_constraint(kind, &picks)
                    && let Some(e) = &mut st.sketch
                {
                    e.picks = picks;
                }
            }
        }
        SketchTool::Dimension => {
            let hit = response
                .clicked()
                .then(|| response.interact_pointer_pos())
                .flatten()
                .map(|pos| pick_at(st, map, pos).filter(|p| !matches!(p, Pick::Constraint(_))));
            // The parts are picked: this click places the label.
            if hit.is_some()
                && let Some(c) = st.sketch.as_ref().and_then(|e| e.pending_dim)
            {
                let at = response.interact_pointer_pos().map(to_sketch);
                let locked = st.sketch.as_ref().is_some_and(|e| e.pending_locked);
                let c = match at {
                    Some(at) if !locked => orient(obj.sketch(), c, at),
                    _ => c,
                };
                if let Some(e) = st.sketch.as_mut() {
                    e.pending_dim = None;
                    e.pending_locked = false;
                }
                st.add_sketch_dimension(c, at);
                return;
            }
            if let Some(hit) = hit {
                let mut picks = st.sketch.as_ref().map(|e| e.picks.clone()).unwrap_or_default();
                let empty = hit.is_none();
                if empty && picks.is_empty() {
                    return;
                }
                picks.extend(hit);
                // A circle or an arc is a dimension at once; a point waits for
                // a point or a line, a line for the next click.
                match dimension_of(obj.sketch(), &picks, empty) {
                    Build::Done(c) if empty => {
                        // A line, then a place: the label goes there.
                        let at = response.interact_pointer_pos().map(to_sketch);
                        let c = at.map_or(c, |at| orient(obj.sketch(), c, at));
                        st.add_sketch_dimension(c, at);
                    }
                    Build::Done(c) => {
                        // All parts: the ghost follows the pointer until a
                        // click places it.
                        if let Some(e) = &mut st.sketch {
                            e.picks = picks;
                            e.pending_dim = Some(c);
                        }
                    }
                    Build::Wrong(why) => {
                        st.message = why.into();
                        if let Some(e) = &mut st.sketch {
                            e.picks.clear();
                        }
                    }
                    Build::More => {
                        if let Some(e) = &mut st.sketch {
                            e.picks = picks;
                        }
                    }
                }
            }
        }
        SketchTool::RectPattern => {}
        SketchTool::Regions => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                st.toggle_region(to_sketch(pos));
            }
        }
        SketchTool::Stretch => {
            // A drag: the box of the points.
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                && let Some(e) = st.sketch.as_mut()
            {
                e.boxsel = Some(origin);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(start) = st.sketch.as_mut().and_then(|e| e.boxsel.take())
                && let Some(end) = response.interact_pointer_pos()
            {
                let r = Rect::from_two_pos(start, end);
                let to = |p: Point| map.to_screen(obj.to_design(p));
                let pts: Vec<PointId> = obj
                    .sketch()
                    .points
                    .iter()
                    .filter(|p| r.contains(to(p.pos)))
                    .map(|p| p.id)
                    .collect();
                let e = st.sketch.as_mut().expect("in sketch mode");
                e.selection = pts.iter().map(|p| Pick::Point(*p)).collect();
                e.source.clear();
                e.source_points = pts;
                e.clicks.clear();
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let at = infer::snap(obj.sketch(), to_sketch(pos), pick_mm(map), &[]).at;
                let e = st.sketch.as_mut().expect("in sketch mode");
                if e.source_points.is_empty() {
                    st.message = tr("Drag a box round the points to stretch first.").into();
                } else if e.clicks.is_empty() {
                    e.clicks.push(Snap::free(at));
                } else {
                    let base = e.clicks[0].at;
                    e.clicks.clear();
                    st.apply_motion(SketchTool::Stretch, Motion::Shift(at - base), false);
                }
            }
        }
        t if t.is_edit() => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                st.edit_click(t, to_sketch(pos), pick_mm(map));
            }
        }
        _ => {
            // Right click ends a chain of lines, or a spline.
            if response.secondary_clicked() && tool == SketchTool::Spline {
                finish_spline(st, false);
            } else if response.secondary_clicked()
                && let Some(e) = &mut st.sketch
            {
                e.clicks.clear();
                e.chain_start = None;
            }
            // The Line tool: a drag from the end of a line makes a tangent arc.
            if tool == SketchTool::Line
                && response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                && let Some(from) = snap_at(st, map, origin, false)
                && let Some(p) = from.point
                && obj.sketch().curves.iter().any(|c| match c.geometry {
                    Geometry::Line { a, b } => a == p || b == p,
                    _ => false,
                })
                && let Some(e) = st.sketch.as_mut()
            {
                e.arc_drag = Some(from);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary)
                && let Some(from) = st.sketch.as_mut().and_then(|e| e.arc_drag.take())
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(to) = snap_at(st, map, pos, no_help)
            {
                let opts = st.sketch.as_ref().map(MakeOpts::of).expect("in sketch mode");
                let start = from.point.expect("an end");
                let end = st
                    .edit_sketch("Sketch: Tangent Arc", None, |sk| {
                        make(SketchTool::ArcTangent, sk, &[from, to], opts)?;
                        match sk.curves.last()?.geometry {
                            Geometry::Arc { start: a, end: b, .. } => Some(if a == start { b } else { a }),
                            _ => None,
                        }
                    })
                    .flatten();
                // The chain goes on from the end of the arc.
                if let Some(end) = end
                    && let Some(at) = st.sketch_obj().and_then(|o| o.sketch().point(end))
                    && let Some(e) = st.sketch.as_mut()
                {
                    e.clicks = vec![Snap::on_point(at, end)];
                    e.chain_start.get_or_insert(start);
                }
                return;
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(snap) = snap_at(st, map, pos, no_help)
            {
                // A click on a point uses it (also with Alt), so a chain closes.
                let snap = match (snap.point, snap_at(st, map, pos, false)) {
                    (None, Some(s)) if s.point.is_some() && no_help => s,
                    _ => snap,
                };
                // The tangent circle takes lines, not their points.
                let snap = if tool == SketchTool::TangentCircle {
                    match obj
                        .sketch()
                        .nearest_curve(snap.at, pick_mm(map), &[])
                        .filter(|(c, _)| is_line(obj.sketch(), *c))
                    {
                        Some((c, q)) => Snap::on_curve(q, c),
                        None => {
                            st.message = tr("Click a line.").into();
                            return;
                        }
                    }
                } else {
                    snap
                };
                let e = st.sketch.as_mut().expect("in sketch mode");
                if tool == SketchTool::ArcTangent && e.clicks.is_empty() && snap.point.is_none() {
                    st.message = tr("Click the end of a line.").into();
                    return;
                }
                if tool == SketchTool::Line {
                    line_click(st, snap, no_help);
                    return;
                }
                if tool == SketchTool::Spline {
                    spline_click(st, snap, pick_mm(map));
                    return;
                }
                e.clicks.push(snap);
                let clicks = e.clicks.clone();
                let opts = MakeOpts::of(e);
                if clicks.len() >= clicks_needed(tool, &opts) {
                    st.edit_sketch("Sketch", None, |sk| make(tool, sk, &clicks, opts));
                    if let Some(e) = &mut st.sketch {
                        e.clicks.clear();
                    }
                }
            }
        }
    }
}

/// A click of the spline tool: a new point of the spline. A click on the
/// last point ends it; a click on the first point (after three or more)
/// closes it.
fn spline_click(st: &mut AppState, to: Snap, tol: f64) {
    let Some(e) = st.sketch.as_mut() else { return };
    if let Some(last) = e.clicks.last()
        && last.at.distance(to.at) <= tol
    {
        finish_spline(st, false);
        return;
    }
    if e.clicks.len() >= 3 && e.clicks[0].at.distance(to.at) <= tol {
        finish_spline(st, true);
        return;
    }
    e.clicks.push(to);
}

/// Makes the spline of the clicks of the spline tool (two or more). Its
/// ends join the points or the curves that the first and the last click
/// snapped to.
fn finish_spline(st: &mut AppState, closed: bool) {
    let Some(e) = st.sketch.as_mut() else { return };
    let clicks = std::mem::take(&mut e.clicks);
    let construction = e.construction;
    if clicks.len() < 2 {
        return;
    }
    st.edit_sketch("Sketch: Spline", None, |sk| {
        let pts: Vec<Point> = clicks.iter().map(|c| c.at).collect();
        let Ok(pieces) = sk.add_spline(&pts, closed) else {
            return;
        };
        let first = pieces
            .first()
            .and_then(|c| sk.curve(*c))
            .and_then(|c| c.ends())
            .map(|e| e.0);
        let last = pieces
            .last()
            .and_then(|c| sk.curve(*c))
            .and_then(|c| c.ends())
            .map(|e| e.1);
        let ends = if closed {
            vec![(clicks[0], first)]
        } else {
            vec![(clicks[0], first), (clicks[clicks.len() - 1], last)]
        };
        for (snap, end) in ends {
            let Some(end) = end else { continue };
            if let Some(q) = snap.point {
                let _ = sk.add_constraint(Constraint::Coincident { a: q, b: end });
            } else if let Some(c) = snap.curve {
                let _ = sk.add_constraint(Constraint::OnCurve { point: end, curve: c });
            }
        }
        for c in &pieces {
            sk.set_construction(*c, construction);
        }
    });
}

/// The end and the hints of a line from the last click of the chain to
/// `to` (no hints with `no_help`).
fn line_end(st: &AppState, to: &Snap, no_help: bool) -> Option<(Snap, Vec<Hint>)> {
    let from = *st.sketch.as_ref()?.clicks.first()?;
    if no_help {
        return Some((*to, Vec::new()));
    }
    let reference = st.sketch.as_ref()?.reference;
    let (end, hints) = infer::line_hints_with(st.sketch_obj()?.sketch(), &from, to, reference);
    Some((Snap { at: end, ..*to }, hints))
}

/// A click of the line tool: the first click starts a chain; each next
/// click adds a line from the last point, with the constraints of the
/// drawing help. A click on the first point of the chain closes it.
fn line_click(st: &mut AppState, to: Snap, no_help: bool) {
    let Some(e) = st.sketch.as_mut() else { return };
    let Some(&from) = e.clicks.first() else {
        let first = to;
        e.clicks = vec![first];
        e.chain_start = first.point;
        return;
    };
    let construction = e.construction;
    let Some((to, hints)) = line_end(st, &to, no_help) else {
        return;
    };
    let ends = st.edit_sketch("Sketch: Line", None, |sk| {
        let pa = infer::place(sk, &from);
        let pb = infer::place(sk, &to);
        if pa == pb {
            return None;
        }
        let l = sk.add_line_between(pa, pb).ok()?;
        sk.set_construction(l, construction);
        infer::apply_hints(sk, l, &hints);
        Some((pa, pb))
    });
    let Some(e) = st.sketch.as_mut() else { return };
    let Some(Some((pa, pb))) = ends else { return };
    let start = *e.chain_start.get_or_insert(pa);
    if pb == start {
        e.clicks.clear();
        e.chain_start = None;
    } else {
        let at = st.sketch_obj().and_then(|o| o.sketch().point(pb)).unwrap_or(to.at);
        let e = st.sketch.as_mut().expect("in sketch mode");
        e.clicks = vec![Snap::on_point(at, pb)];
    }
}

/// Draws the sketch objects that are not open (thin lines; construction
/// curves dashed).
pub fn draw_objects(painter: &egui::Painter, map: &ViewMap, st: &AppState) {
    let open = st.sketch.as_ref().map(|e| e.object);
    let d = st.doc.design();
    for o in d.objects().iter().filter(|o| st.view_filter.shows(o)) {
        let ObjectKind::Sketch(s) = &o.kind else { continue };
        if Some(o.id) == open {
            continue;
        }
        let selected = st.selection.contains(o.id);
        let color = if selected {
            Color32::from_rgb(80, 160, 255)
        } else {
            Color32::from_gray(140)
        };
        for (id, path) in s.curve_paths() {
            let construction = s.sketch().curve(*id).is_some_and(|c| c.construction);
            draw_path(
                painter,
                map,
                path,
                Stroke::new(if selected { 1.8 } else { 1.0 }, color),
                construction,
            );
        }
    }
}

/// A centerline: long and short dashes.
fn draw_centerline(painter: &egui::Painter, map: &ViewMap, path: &emb_geom::Path, stroke: Stroke) {
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    for pl in path.flatten(tol) {
        let pts: Vec<Pos2> = pl.points.iter().map(|p| map.to_screen(*p)).collect();
        painter.extend(Shape::dashed_line_with_offset(
            &pts,
            stroke,
            &[14.0, 3.0],
            &[3.0, 3.0],
            0.0,
        ));
    }
}

fn draw_path(painter: &egui::Painter, map: &ViewMap, path: &emb_geom::Path, stroke: Stroke, dashed: bool) {
    let tol = (0.25 / map.view.scale()).clamp(0.002, 0.5);
    for pl in path.flatten(tol) {
        let mut pts: Vec<Pos2> = pl.points.iter().map(|p| map.to_screen(*p)).collect();
        if pl.closed
            && let Some(f) = pts.first().copied()
        {
            pts.push(f);
        }
        if dashed {
            painter.extend(Shape::dashed_line(&pts, stroke, 6.0, 4.0));
        } else {
            painter.add(Shape::line(pts, stroke));
        }
    }
}

/// Draws the open sketch: the other objects dim, the curves in the color
/// of their state, the points, the marks and dimensions, the preview of the
/// draw tool, and the value editor of a dimension.
pub fn draw_overlay(painter: &egui::Painter, ui: &egui::Ui, map: &ViewMap, st: &mut AppState) {
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let Some(a) = st.sketch_analysis() else { return };
    let dark = ui.visuals().dark_mode;
    let bg = if dark {
        Color32::from_rgba_unmultiplied(32, 32, 32, 150)
    } else {
        Color32::from_rgba_unmultiplied(248, 248, 248, 150)
    };
    painter.rect_filled(map.rect, 0.0, bg);
    let (free, fixed, conflict, construction) = colors(dark);
    let s = obj.sketch();
    let e = st.sketch.clone().expect("in sketch mode");
    let selected = |p: Pick| e.selection.contains(&p) || e.picks.contains(&p);
    let accent = Color32::from_rgb(255, 200, 40);
    // The part under the pointer, for the tools that click parts.
    let picks_parts = matches!(
        e.tool,
        SketchTool::Select | SketchTool::Dimension | SketchTool::Constrain(_)
    ) || e.tool.is_edit();
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|p| map.rect.contains(*p) && picks_parts && e.drag.is_none())
        .and_then(|p| {
            if e.tool.is_edit() && !matches!(e.tool, SketchTool::Fillet | SketchTool::Chamfer) {
                let at = obj.to_sketch(map.to_design(p));
                s.nearest_curve(at, pick_mm(map), &[]).map(|(c, _)| Pick::Curve(c))
            } else {
                pick_at(st, map, p)
            }
        });
    // The picked regions, shaded; with the Regions tool, the region under
    // the pointer.
    {
        let regions = st.sketch_regions();
        let ring = |path: &emb_geom::Path| -> Vec<Pos2> {
            path.flatten(0.1)
                .into_iter()
                .flat_map(|pl| pl.points)
                .map(|q| map.to_screen(obj.to_design(q)))
                .collect()
        };
        let smallest = |p: Point| {
            regions
                .iter()
                .filter(|r| r.contains(p))
                .min_by(|a, b| a.area.total_cmp(&b.area))
        };
        for q in &e.picked {
            if let Some(r) = smallest(*q) {
                let mut rings = vec![ring(&r.outer)];
                rings.extend(r.holes.iter().map(ring));
                painter.add(crate::applique::scanline_mesh(
                    &rings,
                    accent.gamma_multiply(0.3),
                    map.rect,
                ));
            }
        }
        if e.tool == SketchTool::Regions
            && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
            && let Some(r) = smallest(obj.to_sketch(map.to_design(hover)))
        {
            let mut rings = vec![ring(&r.outer)];
            rings.extend(r.holes.iter().map(ring));
            painter.add(crate::applique::scanline_mesh(
                &rings,
                accent.gamma_multiply(0.12),
                map.rect,
            ));
            for path in std::iter::once(&r.outer).chain(r.holes.iter()) {
                let mut pts = ring(path);
                if let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                painter.add(Shape::line(pts, Stroke::new(3.0, accent.gamma_multiply(0.8))));
            }
        }
    }
    // The constraints of the part under the pointer (they show also when
    // the marks are off), and the parts of the constraint under it.
    let related: Vec<ConstraintId> = match hovered {
        Some(Pick::Curve(c)) => s
            .constraints
            .iter()
            .filter(|x| x.constraint.curves().contains(&c))
            .map(|x| x.id)
            .collect(),
        Some(Pick::Point(q)) => s
            .constraints
            .iter()
            .filter(|x| x.constraint.points().contains(&q))
            .map(|x| x.id)
            .collect(),
        _ => Vec::new(),
    };
    let (hot_curves, hot_points) = match hovered.and_then(|p| match p {
        Pick::Constraint(id) => s.constraint(id).copied(),
        _ => None,
    }) {
        Some(c) => (c.curves(), c.points()),
        None => (Vec::new(), Vec::new()),
    };
    // The curves of the constraints in conflict.
    let bad: Vec<CurveId> = s
        .constraints
        .iter()
        .filter(|c| a.conflicting.contains(&c.id))
        .flat_map(|c| c.constraint.curves())
        .collect();
    let bad_points: Vec<PointId> = s
        .constraints
        .iter()
        .filter(|c| a.conflicting.contains(&c.id))
        .flat_map(|c| c.constraint.points())
        .collect();
    for (id, path) in obj.curve_paths() {
        let c = s.curve(*id).expect("the path of a curve");
        let color = if bad.contains(id) {
            conflict
        } else if c.construction {
            construction
        } else if a.fixed_curves.contains(id) {
            fixed
        } else {
            free
        };
        let width = if selected(Pick::Curve(*id)) { 3.0 } else { 1.8 };
        if hovered == Some(Pick::Curve(*id)) || hot_curves.contains(id) {
            draw_path(
                painter,
                map,
                path,
                Stroke::new(width + 4.0, accent.gamma_multiply(0.3)),
                false,
            );
        }
        if selected(Pick::Curve(*id)) {
            draw_path(
                painter,
                map,
                path,
                Stroke::new(width + 3.0, accent.gamma_multiply(0.5)),
                false,
            );
        }
        if c.centerline {
            // A centerline: long and short dashes.
            draw_centerline(painter, map, path, Stroke::new(width, color.gamma_multiply(0.9)));
        } else {
            draw_path(painter, map, path, Stroke::new(width, color), c.construction);
        }
    }
    let to = |p: Point| map.to_screen(obj.to_design(p));
    // The handles of the spline pieces: thin lines from their ends.
    for c in &s.curves {
        if let Geometry::Bezier { start, c1, c2, end } = c.geometry {
            for (e, h) in [(start, c1), (end, c2)] {
                if let (Some(pe), Some(ph)) = (s.point(e), s.point(h)) {
                    painter.line_segment([to(pe), to(ph)], Stroke::new(1.0, construction.gamma_multiply(0.8)));
                }
            }
        }
    }
    for p in &s.points {
        let pos = to(p.pos);
        let color = if bad_points.contains(&p.id) {
            conflict
        } else if a.fixed_points.contains(&p.id) {
            fixed
        } else {
            free
        };
        let r = if selected(Pick::Point(p.id)) { 4.5 } else { 3.0 };
        if p.center {
            // A center point: a cross.
            let k = 6.0;
            let st = Stroke::new(1.6, color);
            painter.line_segment([pos + Vec2::new(-k, 0.0), pos + Vec2::new(k, 0.0)], st);
            painter.line_segment([pos + Vec2::new(0.0, -k), pos + Vec2::new(0.0, k)], st);
        }
        if hovered == Some(Pick::Point(p.id)) || hot_points.contains(&p.id) {
            painter.circle_filled(pos, r + 4.0, accent.gamma_multiply(0.35));
        }
        if selected(Pick::Point(p.id)) {
            painter.circle_filled(pos, r + 2.5, accent.gamma_multiply(0.6));
        }
        painter.circle_filled(pos, r, color);
    }
    // Coincident points: a ring.
    for c in &s.constraints {
        if let Constraint::Coincident { a: pa, .. } = c.constraint
            && let Some(p) = s.point(pa)
        {
            painter.circle_stroke(to(p), 5.5, Stroke::new(1.2, free));
        }
    }
    // Marks and dimensions.
    let text_bg = ui.visuals().extreme_bg_color.gamma_multiply(0.9);
    for l in labels_with(&obj, map, st.settings.units, true) {
        if !(e.show_marks || l.dimension || related.contains(&l.id)) {
            continue;
        }
        let color = if a.conflicting.contains(&l.id) {
            conflict
        } else if a.redundant.contains(&l.id) || s.is_driven(l.id) {
            Color32::from_gray(150)
        } else if l.dimension {
            ui.visuals().text_color()
        } else {
            free
        };
        let sel = selected(Pick::Constraint(l.id));
        let hot = hovered == Some(Pick::Constraint(l.id)) || related.contains(&l.id);
        let frame = if hot {
            Stroke::new(2.0, accent)
        } else {
            Stroke::new(1.0, color.gamma_multiply(0.6))
        };
        let (fill, text) = if sel {
            (accent, Color32::BLACK)
        } else {
            (text_bg, color)
        };
        draw_label(painter, &l, color, fill, text, frame, 1.0);
    }
    // The Dimension tool: the dimension that the next click makes, at the
    // pointer, as a ghost.
    if e.tool == SketchTool::Dimension
        && e.value_edit.is_none()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let under = hovered.filter(|p| !matches!(p, Pick::Constraint(_)));
        let mut picks = e.picks.clone();
        if e.pending_dim.is_none() {
            picks.extend(under);
        }
        let empty = under.is_none() || e.pending_dim.is_some();
        let candidate = match e
            .pending_dim
            .map(Build::Done)
            .unwrap_or_else(|| dimension_of(s, &picks, empty))
        {
            Build::Done(c) => Some(c),
            // A line alone: its length, placed where the pointer is.
            Build::More => match picks.as_slice() {
                [Pick::Curve(l)] if is_line(s, *l) => dimension_of(s, &picks, true).done(),
                _ => None,
            },
            Build::Wrong(_) => None,
        };
        let at = obj.to_sketch(map.to_design(hover));
        let candidate = candidate.map(|c| {
            if e.pending_locked || !empty {
                c
            } else {
                orient(s, c, at)
            }
        });
        if let Some(c) = candidate {
            let mut ghost = obj.clone();
            let id = ghost.edit(|sk| {
                let id = sk.add_constraint(c).ok()?;
                if empty {
                    sk.label_at.insert(id, at);
                }
                Some(id)
            });
            if let Some(l) = id.and_then(|id| labels(&ghost, map, st.settings.units).into_iter().find(|l| l.id == id)) {
                let color = ui.visuals().text_color();
                draw_label(painter, &l, color, text_bg, color, Stroke::new(1.0, accent), 0.55);
            }
        }
    }
    // The preview of the draw tool, with the snap and the constraints that
    // the drawing help adds.
    if let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p)) {
        let no_help = ui.input(|i| i.modifiers.alt) || !e.auto;
        let draws = !e.tool.is_edit()
            && !matches!(
                e.tool,
                SketchTool::Select | SketchTool::Dimension | SketchTool::Constrain(_)
            );
        let mut snap = snap_at(st, map, hover, no_help || !draws).unwrap_or(Snap::free(Point::new(0.0, 0.0)));
        if e.tool == SketchTool::TangentCircle
            && let Some((c, q)) = s
                .nearest_curve(obj.to_sketch(map.to_design(hover)), pick_mm(map), &[])
                .filter(|(c, _)| is_line(s, *c))
        {
            snap = Snap::on_curve(q, c);
        }
        let mut hints = Vec::new();
        if draws
            && e.tool == SketchTool::Line
            && !e.clicks.is_empty()
            && let Some((end, h)) = line_end(st, &snap, no_help)
        {
            snap = end;
            hints = h;
        }
        let pos = if draws { to(snap.at) } else { hover };
        let pts: Vec<Pos2> = e.clicks.iter().map(|c| to(c.at)).collect();
        let pv = Stroke::new(1.2, accent);
        let faint = Stroke::new(1.0, accent.gamma_multiply(0.5));
        if draws {
            // The lines to the points that the pointer is in line with.
            for (axis, p) in snap.align.iter().enumerate() {
                if let Some(q) = p.and_then(|p| s.point(p)) {
                    let from = to(q);
                    let dash = if axis == 0 {
                        [Pos2::new(from.x, from.y), Pos2::new(from.x, pos.y)]
                    } else {
                        [Pos2::new(from.x, from.y), Pos2::new(pos.x, from.y)]
                    };
                    painter.extend(Shape::dashed_line(
                        &dash,
                        Stroke::new(1.0, accent.gamma_multiply(0.7)),
                        4.0,
                        4.0,
                    ));
                }
            }
            // The place of a snap: a diamond at the middle of a line, an X
            // where two curves cross.
            let mark = Stroke::new(1.5, accent);
            if snap.midpoint {
                let k = 6.0;
                let d = [
                    pos + Vec2::new(0.0, -k),
                    pos + Vec2::new(k, 0.0),
                    pos + Vec2::new(0.0, k),
                    pos + Vec2::new(-k, 0.0),
                ];
                painter.add(Shape::closed_line(d.to_vec(), mark));
            } else if snap.also.is_some() {
                let k = 5.0;
                painter.line_segment([pos + Vec2::new(-k, -k), pos + Vec2::new(k, k)], mark);
                painter.line_segment([pos + Vec2::new(-k, k), pos + Vec2::new(k, -k)], mark);
            }
            // The clicks so far.
            for q in &pts {
                painter.circle_filled(*q, 3.5, accent);
            }
            // The shape that the next click makes (a drag from the end of a
            // line: its tangent arc).
            let (tool, clicks) = match e.arc_drag {
                Some(from) => (SketchTool::ArcTangent, vec![from]),
                None => (e.tool, e.clicks.clone()),
            };
            let need = clicks_needed(tool, &MakeOpts::of(&e));
            let mut all = clicks;
            all.push(snap);
            if tool == SketchTool::Line {
                if let [.., last] = pts.as_slice() {
                    painter.line_segment([*last, pos], pv);
                }
            } else if tool == SketchTool::Spline && all.len() >= 2 {
                // The spline through the clicks and the pointer.
                let mut ghost = Sketch::new();
                let at: Vec<Point> = all.iter().map(|c| c.at).collect();
                let t = obj.placement();
                for c in ghost.add_spline(&at, false).unwrap_or_default() {
                    if let Some(path) = ghost.curve_path(c) {
                        draw_path(painter, map, &path.transformed(&t), pv, false);
                    }
                }
            } else if all.len() >= need && need != usize::MAX {
                let mut ghost = s.clone();
                let before: std::collections::BTreeSet<CurveId> = s.curves.iter().map(|c| c.id).collect();
                make(tool, &mut ghost, &all, MakeOpts::of(&e));
                let t = obj.placement();
                for c in ghost.curves.iter().filter(|c| !before.contains(&c.id)) {
                    if let Some(path) = ghost.curve_path(c.id) {
                        let stroke = if c.construction { faint } else { pv };
                        draw_path(painter, map, &path.transformed(&t), stroke, c.construction);
                    }
                }
            } else if let [first, ..] = pts.as_slice() {
                // The first click of a tool of three clicks.
                match e.tool {
                    SketchTool::ArcCenter => {
                        painter.circle_stroke(*first, first.distance(pos), faint);
                        painter.line_segment([*first, pos], pv);
                    }
                    _ => {
                        painter.line_segment([*first, pos], pv);
                    }
                }
            }
        }
        if e.tool != SketchTool::Select
            && let Some(Pick::Point(p)) = pick_at(st, map, hover)
            && let Some(q) = s.point(p)
        {
            painter.circle_stroke(to(q), 7.0, Stroke::new(1.5, accent));
        }
        // The marks of the constraints that the click adds.
        if draws {
            let mut marks: Vec<&str> = hints.iter().map(|h| h.mark()).collect();
            marks.extend(snap.marks());
            if !marks.is_empty() {
                let text = marks.join(" ");
                let at = pos + Vec2::new(14.0, 14.0);
                let rect = Rect::from_min_size(at, Vec2::new(8.0 * text.chars().count() as f32 + 6.0, 16.0));
                painter.rect_filled(rect, 3.0, accent.gamma_multiply(0.8));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(12.0),
                    Color32::BLACK,
                );
            }
        }
    }
    // Stretch: the curves that the points take with them.
    if e.tool == SketchTool::Stretch
        && let Some(b) = e.clicks.first()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let at = infer::snap(s, obj.to_sketch(map.to_design(hover)), pick_mm(map), &[]).at;
        let d = at - b.at;
        let mut ghost = s.clone();
        let _ = ghost.transform_parts(&[], &e.source_points, &|p| p + d, 1.0, false, false);
        let t = obj.placement();
        painter.circle_filled(to(b.at), 3.5, accent);
        for c in ghost
            .curves
            .iter()
            .filter(|c| c.points().iter().any(|p| e.source_points.contains(p)))
        {
            if let Some(path) = ghost.curve_path(c.id) {
                draw_path(
                    painter,
                    map,
                    &path.transformed(&t),
                    Stroke::new(1.2, accent.gamma_multiply(0.7)),
                    true,
                );
            }
        }
    }
    // Move, Copy, Rotate and Scale: the parts where the click puts them.
    if e.tool.is_transform()
        && let Some(hover) = ui.ctx().pointer_hover_pos().filter(|p| map.rect.contains(*p))
    {
        let at = infer::snap(s, obj.to_sketch(map.to_design(hover)), pick_mm(map), &[]).at;
        let v = &e.values;
        let m = match (e.tool, e.clicks.first()) {
            (SketchTool::Move | SketchTool::Copy, Some(b)) => Some(Motion::Shift(at - b.at)),
            (SketchTool::Rotate, _) => Some(Motion::Turn(at, v.rotate_deg)),
            (SketchTool::Scale, _) => Some(Motion::Grow(at, v.scale)),
            _ => None,
        };
        if let Some(b) = e.clicks.first() {
            painter.circle_filled(to(b.at), 3.5, accent);
            painter.line_segment([to(b.at), to(at)], Stroke::new(1.0, accent.gamma_multiply(0.6)));
        }
        if let Some(m) = m {
            let mut ghost = s.clone();
            let _ = ghost.transform_parts(&e.source, &e.source_points, &|p| m.apply(p), 1.0, false, true);
            let before: std::collections::BTreeSet<CurveId> = s.curves.iter().map(|c| c.id).collect();
            let t = obj.placement();
            for c in ghost.curves.iter().filter(|c| !before.contains(&c.id)) {
                if let Some(path) = ghost.curve_path(c.id) {
                    draw_path(
                        painter,
                        map,
                        &path.transformed(&t),
                        Stroke::new(1.2, accent.gamma_multiply(0.7)),
                        true,
                    );
                }
            }
        }
    }
    if let Some(start) = e.boxsel
        && let Some(now) = ui.ctx().pointer_hover_pos()
    {
        let r = Rect::from_two_pos(start, now);
        painter.rect_filled(r, 0.0, accent.gamma_multiply(0.08));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Middle);
    }
    value_editor(ui, map, st, &obj);
    parameters_window(ui.ctx(), st);
}

/// The text box of a dimension value, over its label.
fn value_editor(ui: &egui::Ui, map: &ViewMap, st: &mut AppState, obj: &SketchObject) {
    let Some((id, mut text)) = st.sketch.as_ref().and_then(|e| e.value_edit.clone()) else {
        return;
    };
    let Some(l) = labels(obj, map, st.settings.units).into_iter().find(|l| l.id == id) else {
        st.sketch.as_mut().expect("in sketch mode").value_edit = None;
        return;
    };
    let unit = match obj.sketch().constraint(id) {
        Some(Constraint::Angle { .. }) => "°",
        _ => st.settings.units.suffix(),
    };
    let first = st.sketch.as_mut().is_some_and(|e| std::mem::take(&mut e.value_new));
    let mut done = None;
    egui::Area::new(egui::Id::new("sketch_value"))
        .fixed_pos(l.rect.left_bottom() + Vec2::new(0.0, 4.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let out = egui::TextEdit::singleline(&mut text)
                        .id(egui::Id::new(("sketch_value_text", id.0)))
                        .desired_width(90.0)
                        .show(ui);
                    let r = out.response.response.clone();
                    if first {
                        // The old value is selected: typing replaces it.
                        r.request_focus();
                        let mut state = out.state.clone();
                        state
                            .cursor
                            .set_char_range(Some(egui::text_selection::CCursorRange::select_all(&out.galley)));
                        state.store(ui.ctx(), r.id);
                    }
                    ui.label(unit);
                    crate::a11y::named(r.clone(), tr("Dimension value")).on_hover_text(tr(
                        "Type a value. You can add a unit, for example 2 in, 1 1/2\" or 50 mm, use + - * /, and the names of parameters.",
                    ));
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button(tr("OK")).clicked() || (r.lost_focus() && enter) {
                        done = Some(true);
                    } else if r.lost_focus() && !first {
                        // A click somewhere else keeps the value that is typed.
                        done = Some(true);
                    }
                });
            });
        });
    match done {
        Some(apply) => {
            st.sketch.as_mut().expect("in sketch mode").value_edit = None;
            if apply {
                st.set_dimension_text(id, &text);
            }
        }
        None => st.sketch.as_mut().expect("in sketch mode").value_edit = Some((id, text)),
    }
}

/// The Properties panel in sketch mode: the state, the selection, and the
/// constraints with their values.
pub fn edit_panel(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(obj) = st.sketch_obj().cloned() else { return };
    let Some(a) = st.sketch_analysis() else { return };
    let s = obj.sketch();
    ui.heading(tr("Sketch"));
    if s.points.is_empty() {
        ui.label(tr(
            "Draw with the tools of the Sketch tab: lines, rectangles, circles, arcs and polygons.",
        ));
    } else if !a.conflicting.is_empty() {
        ui.colored_label(
            Color32::from_rgb(230, 60, 50),
            fill(
                "{} constraints conflict with the others (red). Remove them, or change a value.",
                &[&a.conflicting.len()],
            ),
        );
    } else if a.free == 0 {
        ui.colored_label(
            Color32::from_rgb(40, 160, 60),
            tr("Fully constrained: nothing can move."),
        );
    } else {
        ui.label(fill(
            "{} free motions: add constraints or dimensions to fix the shape (green is fixed).",
            &[&a.free],
        ));
    }
    if !a.redundant.is_empty() {
        ui.weak(fill("{} constraints repeat others (gray).", &[&a.redundant.len()]));
    }
    ui.label(fill(
        "{} points, {} curves, {} constraints.",
        &[&s.points.len(), &s.curves.len(), &s.constraints.len()],
    ));
    make_embroidery_ui(ui, st);
    ui.separator();
    let mut construction = st.sketch.as_ref().is_some_and(|e| e.construction);
    if ui
        .checkbox(&mut construction, tr("New curves are construction curves"))
        .on_hover_text(tr("Construction curves help to draw. They do not become embroidery."))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.construction = construction;
    }
    // The units of the program (Options), also here.
    ui.horizontal(|ui| {
        ui.label(tr("Units"));
        let mut units = st.settings.units;
        let a = ui.radio_value(&mut units, crate::workspace::Units::Mm, "mm");
        let b = ui.radio_value(&mut units, crate::workspace::Units::Inch, "in");
        if (a.changed() || b.changed()) && units != st.settings.units {
            let mut s = st.settings.clone();
            s.units = units;
            st.set_settings(s);
            if let Some(e) = &mut st.sketch {
                e.values = EditValues::for_units(units);
            }
        }
    })
    .response
    .on_hover_text(tr(
        "The units of the program (also in Options). You can type a value in other units, for example 2 in or 50 mm.",
    ));
    let mut auto = st.sketch.as_ref().is_some_and(|e| e.auto);
    if ui
        .checkbox(&mut auto, tr("Add constraints while drawing"))
        .on_hover_text(tr(
            "Horizontal, vertical, tangent, on a curve and coincident. Hold Alt while you click to add none.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.auto = auto;
    }
    let mut marks = st.sketch.as_ref().is_some_and(|e| e.show_marks);
    if ui
        .checkbox(&mut marks, tr("Show constraints"))
        .on_hover_text(tr(
            "Show the marks of the constraints (F8 shows, F9 hides). The dimensions always show.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.show_marks = marks;
    }
    edit_values(ui, st);
    if let Some(e) = &st.sketch
        && !e.relaxed.is_empty()
    {
        ui.weak(fill("Relax removed: {}.", &[&e.relaxed.join(", ")]));
    }
    let mut outside = st.sketch.as_ref().is_some_and(|e| e.polygon_outside);
    if ui
        .checkbox(&mut outside, tr("Polygon round its circle"))
        .on_hover_text(tr(
            "On: the sides touch the circle (the click is the middle of a side). Off: the corners are on the circle.",
        ))
        .changed()
        && let Some(e) = &mut st.sketch
    {
        e.polygon_outside = outside;
    }
    let mut slot = st.sketch.as_ref().map_or(SlotKind::CenterToCenter, |e| e.slot_kind);
    ui.horizontal(|ui| {
        ui.label(tr("Slot"));
        egui::ComboBox::from_id_salt("sketch_slot_kind")
            .selected_text(slot.name())
            .show_ui(ui, |ui| {
                for k in SlotKind::ALL {
                    ui.selectable_value(&mut slot, k, k.name());
                }
            });
    });
    if let Some(e) = &mut st.sketch
        && e.slot_kind != slot
    {
        e.slot_kind = slot;
        e.clicks.clear();
    }
    let mut sides = st.sketch.as_ref().map_or(6, |e| e.polygon_sides);
    ui.horizontal(|ui| {
        ui.label(tr("Polygon sides"));
        if ui.add(egui::DragValue::new(&mut sides).range(3..=40)).changed()
            && let Some(e) = &mut st.sketch
        {
            e.polygon_sides = sides;
        }
    });
    ui.separator();
    ui.strong(tr("Constraints"));
    let mut remove = None;
    let mut select = None;
    let mut edit = None;
    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        for c in &s.constraints {
            ui.horizontal(|ui| {
                let color = if a.conflicting.contains(&c.id) {
                    Color32::from_rgb(230, 60, 50)
                } else if a.redundant.contains(&c.id) {
                    Color32::from_gray(150)
                } else {
                    ui.visuals().text_color()
                };
                let text = constraint_text(st.settings.units, &c.constraint);
                if ui
                    .selectable_label(false, egui::RichText::new(text).color(color))
                    .clicked()
                {
                    select = Some(c.id);
                }
                if c.constraint.is_dimension() && ui.small_button(tr("Change")).clicked() {
                    edit = Some((c.id, box_text(st.settings.units, s, c.id, &c.constraint)));
                }
                if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the constraint")).clicked()
                {
                    remove = Some(c.id);
                }
            });
        }
    });
    if let Some(id) = remove {
        st.edit_sketch("Remove Constraint", None, |sk| sk.remove_constraint(id));
    }
    if let Some(e) = &mut st.sketch {
        if let Some(id) = select {
            e.selection = vec![Pick::Constraint(id)];
        }
        if let Some((id, text)) = edit {
            e.edit_value(id, text);
        }
    }
}

/// The values of the edit tool that is on (fillet radius, chamfer and
/// offset distance, pattern).
fn edit_values(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(e) = &st.sketch else { return };
    let (tool, mut v) = (e.tool, e.values.clone());
    let units = st.settings.units;
    let max = emb_sketch::MAX_MM;
    let mut make_pattern = false;
    let mut motion: Option<Motion> = None;
    let row = |ui: &mut egui::Ui, label: &str, f: &mut dyn FnMut(&mut egui::Ui)| {
        ui.horizontal(|ui| {
            ui.label(label);
            f(ui);
        });
    };
    match tool {
        SketchTool::Fillet => row(ui, tr("Fillet radius"), &mut |ui| {
            crate::workspace::length_value(ui, &mut v.fillet_mm, units, 0.01, max);
        }),
        SketchTool::Chamfer => {
            row(ui, tr("Chamfer"), &mut |ui| {
                for (k, name) in [
                    (0, tr("Equal distances")),
                    (1, tr("Two distances")),
                    (2, tr("Distance and angle")),
                ] {
                    ui.radio_value(&mut v.chamfer_kind, k, name);
                }
            });
            row(ui, tr("Chamfer distance"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.chamfer_mm, units, 0.01, max);
            });
            match v.chamfer_kind {
                1 => row(ui, tr("Second distance"), &mut |ui| {
                    crate::workspace::length_value(ui, &mut v.chamfer2_mm, units, 0.01, max);
                }),
                2 => row(ui, tr("Angle"), &mut |ui| {
                    ui.add(egui::DragValue::new(&mut v.chamfer_deg).range(1.0..=179.0).suffix("°"));
                }),
                _ => {}
            }
            if v.chamfer_kind != 0 {
                ui.weak(tr(
                    "The first distance is on the first line of the corner (the older line).",
                ));
            }
        }
        SketchTool::Offset => {
            row(ui, tr("Offset distance"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.offset_mm, units, -max, max);
            });
            ui.weak(tr(
                "Positive: outside of a closed shape, else on the left. Negative: the other side.",
            ));
        }
        SketchTool::RectPattern => {
            row(ui, tr("Number of columns"), &mut |ui| {
                ui.add(egui::DragValue::new(&mut v.cols).range(1..=100));
            });
            row(ui, tr("Number of rows"), &mut |ui| {
                ui.add(egui::DragValue::new(&mut v.rows).range(1..=100));
            });
            row(ui, tr("Column spacing"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.dx_mm, units, -max, max);
            });
            row(ui, tr("Row spacing"), &mut |ui| {
                crate::workspace::length_value(ui, &mut v.dy_mm, units, -max, max);
            });
            make_pattern = ui.button(tr("Make Pattern")).clicked();
        }
        SketchTool::CircularPattern => {
            row(ui, tr("Copies"), &mut |ui| {
                ui.add(egui::DragValue::new(&mut v.count).range(2..=400));
            });
            row(ui, tr("Angle"), &mut |ui| {
                ui.add(egui::DragValue::new(&mut v.angle_deg).range(1.0..=360.0).suffix("°"));
            });
        }
        SketchTool::Move | SketchTool::Copy => {
            ui.weak(tr("Click a base point, then where it goes. Or move by these values:"));
            row(ui, "X", &mut |ui| {
                crate::workspace::length_value(ui, &mut v.move_dx_mm, units, -max, max);
            });
            row(ui, "Y", &mut |ui| {
                crate::workspace::length_value(ui, &mut v.move_dy_mm, units, -max, max);
            });
            if ui.button(tr("Move by the Values")).clicked() {
                motion = Some(Motion::Shift(Point::new(v.move_dx_mm, v.move_dy_mm)));
            }
        }
        SketchTool::Rotate => {
            row(ui, tr("Angle"), &mut |ui| {
                ui.add(
                    egui::DragValue::new(&mut v.rotate_deg)
                        .range(-360.0..=360.0)
                        .suffix("°"),
                );
            });
            ui.checkbox(&mut v.copy_parts, tr("Copy"));
            ui.weak(tr(
                "Click the center of the turn (counterclockwise for a positive angle).",
            ));
        }
        SketchTool::Scale => {
            row(ui, tr("Factor"), &mut |ui| {
                ui.add(egui::DragValue::new(&mut v.scale).range(0.01..=100.0).speed(0.01));
            });
            ui.checkbox(&mut v.copy_parts, tr("Copy"));
            ui.weak(tr("Click the base point of the scale."));
        }
        _ => return,
    }
    if let Some(e) = &mut st.sketch {
        e.values = v;
    }
    if make_pattern {
        st.rect_pattern();
    }
    if let Some(m) = motion {
        st.apply_motion(tool, m, tool == SketchTool::Copy);
    }
}

/// The section of the Properties panel that makes embroidery from the
/// open sketch: its regions (the picked ones, else all) and its selected
/// curves.
fn make_embroidery_ui(ui: &mut egui::Ui, st: &mut AppState) {
    ui.separator();
    ui.strong(tr("Make Embroidery"));
    let regions = st.sketch_regions();
    let picked = st.sketch.as_ref().map_or(0, |e| e.picked.len());
    if regions.is_empty() {
        ui.weak(tr("The sketch has no closed regions yet."));
    } else if picked == 0 {
        ui.weak(fill(
            "{} closed regions. Pick some with the Regions tool, or make all:",
            &[&regions.len()],
        ));
    } else {
        ui.weak(fill("{} regions picked:", &[&picked]));
    }
    if !regions.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for to in RegionTo::ALL {
                if ui.button(tr(to.name())).clicked() {
                    st.make_picked_regions(to);
                }
            }
        });
    }
    let curves = st.selected_curves();
    if !curves.is_empty() {
        ui.weak(fill("{} curves selected:", &[&curves.len()]));
        let mut width = st.settings.defaults.column_width_mm;
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Run")).clicked() {
                st.make_curves(CurveTo::Run);
            }
            if ui.button(tr("Satin Line")).clicked() {
                st.make_curves(CurveTo::SatinLine(width));
            }
            if crate::workspace::length_value(ui, &mut width, st.settings.units, 0.5, 15.0).changed() {
                let mut s = st.settings.clone();
                s.defaults.column_width_mm = width;
                st.set_settings(s);
            }
            if curves.len() == 2 && ui.button(tr("Satin Between")).clicked() {
                st.make_satin_between();
            }
        });
    }
    ui.weak(tr(
        "The stitch settings are the current properties; the thread is the current thread.",
    ));
}

/// A short description of a sketch, for the Sequence panel.
pub fn detail(s: &SketchObject) -> String {
    let sk = s.sketch();
    fill(
        "Sketch · {} curves, {} constraints",
        &[&sk.curves.len(), &sk.constraints.len()],
    )
}

/// The Properties panel of a sketch object (not open).
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, s: &SketchObject) {
    ui.label(detail(s));
    let a = s.sketch().analyze();
    ui.weak(if a.free == 0 && a.conflicting.is_empty() {
        tr("Fully constrained.").to_string()
    } else {
        fill("{} free motions.", &[&a.free])
    });
    if ui.button(tr("Edit Sketch")).clicked() {
        st.selection.set(id);
        st.open_sketch(id);
    }
    // The parameters (ADR 0078).
    let params = &s.sketch().parameters;
    if !params.is_empty() {
        ui.label(fill("{} parameters:", &[&params.len()]));
        for p in params {
            ui.weak(format!("{} = {}", p.name, p.expr));
        }
    }
    if ui.button(tr("Parameters")).clicked() {
        st.selection.set(id);
        st.open_sketch(id);
        st.toggle_parameters();
    }
    let regions = s.regions();
    ui.separator();
    ui.label(fill("Make embroidery of all {} regions:", &[&regions.len()]));
    ui.horizontal_wrapped(|ui| {
        for to in RegionTo::ALL {
            if ui
                .add_enabled(!regions.is_empty(), egui::Button::new(tr(to.name())))
                .clicked()
            {
                st.make_regions(id, regions.clone(), to);
            }
        }
    });
    if ui.button(tr("Runs Along the Curves")).clicked() {
        let sources = s
            .sketch()
            .curves
            .iter()
            .filter(|c| !c.construction)
            .map(|c| LinkSource::Curve {
                curve: c.id,
                to: CurveTo::Run,
            })
            .collect();
        let n = st.make_linked("Sketch to Stitches", id, sources).len();
        st.message = fill("{} objects made from the sketch.", &[&n]);
    }
    links_ui(ui, st, id);
}

/// The made objects of sketch `id` in the Properties panel of the sketch:
/// how many follow it, and those whose region or curve is gone (with
/// Unlink).
fn links_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId) {
    let d = st.doc.design();
    let links: Vec<&emb_model::SketchLink> = d.sketch_links.iter().filter(|l| l.sketch == id).collect();
    if links.is_empty() {
        return;
    }
    let n: usize = links.iter().map(|l| l.objects.len()).sum();
    let lost: Vec<u32> = links.iter().filter(|l| l.lost).map(|l| l.id).collect();
    ui.separator();
    ui.weak(fill("{} objects made from this sketch follow its changes.", &[&n]));
    if !lost.is_empty() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            fill("The region or curve of {} links is gone.", &[&lost.len()]),
        );
        if ui.button(tr("Unlink the Lost Objects")).clicked() {
            st.doc
                .edit("Unlink", |d| d.sketch_links.retain(|l| !lost.contains(&l.id)));
        }
    }
}

/// The Properties panel lines of an object made from a sketch: Edit
/// Sketch, Unlink, and a warning when its region or curve is gone.
pub fn made_from_sketch_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId) {
    let Some(link) = st.doc.design().sketch_link_of(id).cloned() else {
        return;
    };
    ui.separator();
    if link.lost {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            tr("Made from a sketch, but its region or curve is gone: the object stays as it is."),
        );
    } else {
        ui.weak(tr("Made from a sketch: it changes when the sketch changes."));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Edit Sketch")).clicked() {
            st.open_sketch(link.sketch);
        }
        if ui.button(tr("Unlink")).clicked() {
            st.doc.edit("Unlink", |d| d.unlink_sketch(link.id));
            st.message = tr("Unlinked: the objects stay as they are.").into();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn sketching() -> Harness {
        let mut h = Harness::empty();
        h.st.start_sketch();
        assert!(h.st.sketch.is_some());
        h
    }

    fn click_tool(h: &mut Harness, tool: SketchTool, pts: &[Point]) {
        h.st.set_sketch_tool(tool);
        let id = h.st.sketch.as_ref().unwrap().object;
        for p in pts {
            let e = h.st.sketch.as_mut().unwrap();
            e.clicks.push(Snap::free(*p));
            let clicks = e.clicks.clone();
            if clicks.len() >= clicks_needed(tool, &MakeOpts::of(e)) {
                let opts = MakeOpts::of(e);
                h.st.edit_sketch("Sketch", None, |sk| make(tool, sk, &clicks, opts));
                h.st.sketch.as_mut().unwrap().clicks.clear();
            }
        }
        assert_eq!(h.st.sketch.as_ref().unwrap().object, id);
    }

    fn sk(h: &Harness) -> Sketch {
        h.st.sketch_obj().unwrap().sketch().clone()
    }

    #[test]
    fn start_draw_and_finish() {
        let mut h = sketching();
        assert_eq!(h.st.settings.ribbon_tab, RibbonTab::Sketch);
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let s = sk(&h);
        assert_eq!((s.points.len(), s.curves.len()), (4, 4));
        assert_eq!(s.constraints.len(), 4, "horizontal and vertical sides");
        let a = h.st.sketch_analysis().unwrap();
        assert_eq!(a.free, 4, "width, height and the place");
        h.st.finish_sketch();
        assert!(h.st.sketch.is_none());
        let id = h.st.selection.single().unwrap();
        assert!(matches!(
            h.st.doc.design().object(id).unwrap().kind,
            ObjectKind::Sketch(_)
        ));
        assert_ne!(h.st.settings.ribbon_tab, RibbonTab::Sketch);
        // An empty sketch goes away.
        h.st.selection.clear();
        h.st.start_sketch();
        let n = h.st.doc.design().objects().len();
        h.st.finish_sketch();
        assert_eq!(h.st.doc.design().objects().len(), n - 1);
    }

    #[test]
    fn constraints_from_the_selection_and_from_clicks() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 4.0)]);
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(0.0, 10.0), Point::new(30.0, 16.0)],
        );
        let s = sk(&h);
        let (l1, l2) = (s.curves[0].id, s.curves[1].id);
        // Select a line, then Horizontal: it is added at once.
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l1)];
        h.st.set_sketch_tool(SketchTool::Constrain(Kind::Horizontal));
        let s = sk(&h);
        assert!(matches!(s.constraints[0].constraint, Constraint::Horizontal { .. }));
        // Parallel with nothing selected waits for two clicks.
        h.st.set_sketch_tool(SketchTool::Constrain(Kind::Parallel));
        assert!(!h.st.apply_constraint(Kind::Parallel, &[Pick::Curve(l1)]));
        assert!(h.st.apply_constraint(Kind::Parallel, &[Pick::Curve(l1), Pick::Curve(l2)]));
        let s = sk(&h);
        let Geometry::Line { a, b } = s.curve(l2).unwrap().geometry else {
            unreachable!()
        };
        assert!(
            (s.point(a).unwrap().y - s.point(b).unwrap().y).abs() < 1e-9,
            "parallel to a horizontal line"
        );
        // A wrong pick gives a message and adds nothing.
        let n = sk(&h).constraints.len();
        assert!(h.st.apply_constraint(Kind::Concentric, &[Pick::Curve(l1), Pick::Curve(l2)]));
        assert_eq!(sk(&h).constraints.len(), n);
        // Undo takes back the last constraint.
        h.st.doc.undo();
        assert_eq!(sk(&h).constraints.len(), n - 1);
    }

    #[test]
    fn dimensions_change_the_shape_and_conflicts_show() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let s = sk(&h);
        let bottom = s.curves[0].id;
        let Build::Done(c) = dimension_of(&s, &[Pick::Curve(bottom)], true) else {
            panic!("a length")
        };
        assert_eq!(c.value(), Some(60.0));
        let id =
            h.st.edit_sketch("Dimension", None, |sk| sk.add_constraint(c))
                .unwrap()
                .unwrap();
        h.st.set_dimension_text(id, "80");
        let s = sk(&h);
        let Geometry::Line { a, b } = s.curve(bottom).unwrap().geometry else {
            unreachable!()
        };
        assert!((s.point(a).unwrap().distance(s.point(b).unwrap()) - 80.0).abs() < 1e-6);
        // The top line 50 mm long: a conflict with the bottom line (80 mm).
        let top = s.curves[2].id;
        let c = Constraint::Length { line: top, mm: 50.0 };
        let bad =
            h.st.edit_sketch("Dimension", None, |sk| sk.add_constraint(c))
                .unwrap()
                .unwrap();
        let a = h.st.sketch_analysis().unwrap();
        assert_eq!(a.conflicting, vec![bad]);
        let (_, state) = h.st.sketch_status().unwrap();
        assert!(state.contains("conflict"), "{state}");
        // Delete it: no conflict.
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Constraint(bad)];
        h.st.delete_sketch_selection();
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    }

    #[test]
    fn arcs_circles_polygons_and_points() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Circle,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        );
        click_tool(
            &mut h,
            SketchTool::Arc3,
            &[Point::new(20.0, 0.0), Point::new(40.0, 0.0), Point::new(30.0, 10.0)],
        );
        click_tool(
            &mut h,
            SketchTool::ArcCenter,
            &[Point::new(60.0, 0.0), Point::new(70.0, 0.0), Point::new(60.0, 10.0)],
        );
        click_tool(
            &mut h,
            SketchTool::Polygon,
            &[Point::new(0.0, 50.0), Point::new(10.0, 50.0)],
        );
        click_tool(&mut h, SketchTool::Point, &[Point::new(90.0, 90.0)]);
        let s = sk(&h);
        let arcs = s
            .curves
            .iter()
            .filter(|c| matches!(c.geometry, Geometry::Arc { .. }))
            .count();
        assert_eq!(arcs, 2);
        // The arc through three points has its middle point on it.
        let arc = s
            .curves
            .iter()
            .find(|c| matches!(c.geometry, Geometry::Arc { .. }))
            .unwrap();
        let Geometry::Arc { center, radius, .. } = arc.geometry else {
            unreachable!()
        };
        assert!((s.point(center).unwrap().distance(Point::new(30.0, 10.0)) - radius).abs() < 1e-6);
        // The polygon: 6 equal sides and a construction circle.
        assert_eq!(s.curves.iter().filter(|c| c.construction).count(), 1);
        assert!(
            s.constraints
                .iter()
                .filter(|c| matches!(c.constraint, Constraint::Equal { .. }))
                .count()
                >= 5
        );
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    }

    #[test]
    fn a_tangent_arc_goes_on_from_a_line() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
        let s = sk(&h);
        let Geometry::Line { b, .. } = s.curves[0].geometry else {
            unreachable!()
        };
        h.st.set_sketch_tool(SketchTool::ArcTangent);
        let clicks = vec![
            Snap::on_point(Point::new(30.0, 0.0), b),
            Snap::free(Point::new(40.0, 10.0)),
        ];
        h.st.edit_sketch("Sketch", None, |sk| {
            make(
                SketchTool::ArcTangent,
                sk,
                &clicks,
                MakeOpts {
                    sides: 6,
                    outside: false,
                    construction: false,
                    slot: SlotKind::CenterToCenter,
                },
            )
        });
        let s = sk(&h);
        assert!(
            s.constraints
                .iter()
                .any(|c| matches!(c.constraint, Constraint::Tangent { .. }))
        );
        let arc = s
            .curves
            .iter()
            .find(|c| matches!(c.geometry, Geometry::Arc { .. }))
            .unwrap();
        let Geometry::Arc { center, radius, .. } = arc.geometry else {
            unreachable!()
        };
        // Tangent at (30, 0) to a horizontal line: the center is above it.
        let c = s.point(center).unwrap();
        assert!(
            (c.x - 30.0).abs() < 1e-6 && (c.y - radius).abs() < 1e-6,
            "{c:?} {radius}"
        );
    }

    #[test]
    fn a_drag_moves_the_sketch_in_one_undo_step() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let s = sk(&h);
        let corner = s.points[2].id;
        for k in 1..=3 {
            let to = Point::new(60.0 + f64::from(k) * 5.0, 40.0 + f64::from(k) * 5.0);
            h.st.edit_sketch("Drag in Sketch", Some("sketch-drag-1".into()), |sk| sk.drag(corner, to));
        }
        h.st.doc.end_merge();
        let s = sk(&h);
        assert!(s.point(corner).unwrap().distance(Point::new(75.0, 55.0)) < 1e-6);
        h.st.doc.undo();
        assert!(sk(&h).point(corner).unwrap().distance(Point::new(60.0, 40.0)) < 1e-6);
    }

    fn count(s: &Sketch, f: impl Fn(&Constraint) -> bool) -> usize {
        s.constraints.iter().filter(|c| f(&c.constraint)).count()
    }

    #[test]
    fn the_line_tool_adds_constraints_unless_alt() {
        let mut h = sketching();
        h.st.set_sketch_tool(SketchTool::Line);
        line_click(&mut h.st, Snap::free(Point::new(0.0, 0.0)), false);
        line_click(&mut h.st, Snap::free(Point::new(40.0, 1.5)), false);
        let s = sk(&h);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Horizontal { .. })), 1);
        assert!(s.points.iter().any(|p| p.pos.distance(Point::new(40.0, 0.0)) < 1e-9));
        // With Alt: no constraint, the end where the click was.
        line_click(&mut h.st, Snap::free(Point::new(41.0, 30.0)), true);
        let s = sk(&h);
        assert_eq!(s.constraints.len(), 1);
        assert!(s.points.iter().any(|p| p.pos.distance(Point::new(41.0, 30.0)) < 1e-9));
        // A click on a curve puts the point on it.
        let bottom = s.curves[0].id;
        let on = Snap::on_curve(Point::new(10.0, 0.0), bottom);
        line_click(&mut h.st, on, false);
        let s = sk(&h);
        assert_eq!(
            count(
                &s,
                |c| matches!(c, Constraint::OnCurve { curve, .. } if *curve == bottom)
            ),
            1
        );
    }

    #[test]
    fn edit_tools_trim_fillet_and_refuse_without_an_undo_step() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        // A fillet at a corner: an arc and a radius to type.
        h.st.set_sketch_tool(SketchTool::Fillet);
        h.st.edit_click(SketchTool::Fillet, Point::new(60.0, 40.0), 0.5);
        let s = sk(&h);
        assert_eq!(s.curves.iter().filter(|c| !c.is_line()).count(), 1, "{}", h.st.message);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 2);
        assert!(h.st.sketch.as_ref().unwrap().value_edit.is_some());
        // A refused edit: a message and no undo step.
        let undo = h.st.doc.undo_len();
        h.st.edit_click(SketchTool::Fillet, Point::new(200.0, 200.0), 0.5);
        h.st.edit_click(SketchTool::Extend, Point::new(30.0, 0.0), 0.5);
        assert_eq!(h.st.doc.undo_len(), undo);
        assert!(!h.st.message.is_empty());
        // A line across the rectangle, then trim its outer piece.
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(30.0, -10.0), Point::new(30.0, 20.0)],
        );
        let n = sk(&h).curves.len();
        h.st.edit_click(SketchTool::Trim, Point::new(30.0, -5.0), 0.5);
        let s = sk(&h);
        assert_eq!(s.curves.len(), n);
        assert!(!s.points.iter().any(|p| p.pos.distance(Point::new(30.0, -10.0)) < 1e-6));
        h.st.doc.undo();
        assert!(
            sk(&h)
                .points
                .iter()
                .any(|p| p.pos.distance(Point::new(30.0, -10.0)) < 1e-6)
        );
    }

    #[test]
    fn mirror_offset_patterns_and_relax() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(0.0, -20.0), Point::new(0.0, 20.0)],
        );
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(-10.0, 0.0), Point::new(-5.0, 10.0)],
        );
        let s = sk(&h);
        let (axis, l) = (s.curves[0].id, s.curves[1].id);
        // Mirror needs a selection first.
        h.st.set_sketch_tool(SketchTool::Mirror);
        assert_ne!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Mirror);
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        h.st.set_sketch_tool(SketchTool::Mirror);
        h.st.edit_click(SketchTool::Mirror, Point::new(0.0, 5.0), 0.5);
        let s = sk(&h);
        assert_eq!(
            count(&s, |c| matches!(c, Constraint::Symmetric { line, .. } if *line == axis)),
            2
        );
        assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
        // Offset of the selection at once.
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        h.st.set_sketch_tool(SketchTool::Offset);
        assert_eq!(count(&sk(&h), |c| matches!(c, Constraint::LineDistance { .. })), 1);
        // A rectangular pattern of the line: 3 places.
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        let n = sk(&h).curves.len();
        h.st.rect_pattern();
        assert_eq!(sk(&h).curves.len(), n + 2);
        // A circular pattern round the origin.
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        h.st.set_sketch_tool(SketchTool::CircularPattern);
        let n = sk(&h).curves.len();
        h.st.edit_click(SketchTool::CircularPattern, Point::new(100.0, 100.0), 0.5);
        assert_eq!(sk(&h).curves.len(), n + 5);
        // Relax: a length that conflicts goes.
        let s = sk(&h);
        let a = h.st.edit_sketch("Dimension", None, |sk| {
            sk.add_constraint(Constraint::Length { line: l, mm: 10.0 })
        });
        assert!(a.unwrap().is_ok());
        h.st.edit_sketch("Dimension", None, |sk| {
            sk.add_constraint(Constraint::Length { line: l, mm: 20.0 })
        });
        assert!(!h.st.sketch_analysis().unwrap().conflicting.is_empty());
        h.st.relax_sketch();
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
        assert_eq!(h.st.sketch.as_ref().unwrap().relaxed.len(), 1);
        assert_eq!(sk(&h).constraints.len(), s.constraints.len() + 1);
    }

    #[test]
    fn the_dimension_tool_makes_a_distance_to_a_line() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
        click_tool(&mut h, SketchTool::Point, &[Point::new(10.0, 8.0)]);
        let s = sk(&h);
        let (l, q) = (s.curves[0].id, s.points[2].id);
        assert!(matches!(dimension_of(&s, &[Pick::Curve(l)], false), Build::More));
        let Build::Done(c) = dimension_of(&s, &[Pick::Point(q), Pick::Curve(l)], false) else {
            panic!("a distance to the line")
        };
        assert!((c.value().unwrap() - 8.0).abs() < 1e-9);
        assert_eq!(value_text(crate::workspace::Units::Mm, &c), "8.00");
    }

    /// The keys go through the keys of the program first, as in the app
    /// (the harness runs `handle_shortcuts` before the canvas).
    #[test]
    fn esc_delete_and_the_value_box_work_with_the_keys_of_the_program() {
        use eframe::egui::{Event, Key, Modifiers, PointerButton};
        let mut h = sketching();
        h.st.settings.units = crate::workspace::Units::Mm;
        h.st.set_sketch_tool(SketchTool::Rectangle);
        let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
        h.click(PointerButton::Primary, a);
        h.click(PointerButton::Primary, b);
        assert_eq!(sk(&h).curves.len(), 4);
        // Esc: back to Select at once.
        h.st.set_sketch_tool(SketchTool::Line);
        let p = h.screen(Point::new(50.0, 0.0));
        h.click(PointerButton::Primary, p);
        h.key(Key::Escape);
        let e = h.st.sketch.as_ref().unwrap();
        assert_eq!(e.tool, SketchTool::Select);
        assert!(e.clicks.is_empty());
        // Delete: the selected line goes, the sketch object stays.
        let line = h.screen(Point::new(0.0, -20.0));
        h.click(PointerButton::Primary, line);
        assert!(matches!(
            h.st.sketch.as_ref().unwrap().selection.as_slice(),
            [Pick::Curve(_)]
        ));
        h.key(Key::Delete);
        assert!(h.st.sketch_obj().is_some(), "the sketch stays");
        assert_eq!(sk(&h).curves.len(), 3);
        h.st.doc.undo();
        assert_eq!(sk(&h).curves.len(), 4);
        // A dimension, then a value in inches into the box (its old text is
        // selected, so typing replaces it).
        h.st.set_sketch_tool(SketchTool::Dimension);
        h.click(PointerButton::Primary, line);
        let empty = h.screen(Point::new(0.0, -35.0));
        h.click(PointerButton::Primary, empty);
        let (id, _) = h.st.sketch.as_ref().unwrap().value_edit.clone().expect("the value box");
        h.frame(vec![]);
        h.frame(vec![Event::Text("2 in".into())]);
        h.key(Key::Enter);
        assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
        let v = sk(&h).constraint(id).unwrap().value().unwrap();
        assert!((v - 50.8).abs() < 1e-9, "{v}");
        // Esc closes the box and keeps the value.
        let text = value_text(h.st.settings.units, sk(&h).constraint(id).unwrap());
        h.st.sketch.as_mut().unwrap().edit_value(id, text);
        h.frame(vec![]);
        h.frame(vec![Event::Text("99".into())]);
        h.key(Key::Escape);
        assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
        assert!((sk(&h).constraint(id).unwrap().value().unwrap() - 50.8).abs() < 1e-9);
        // The Dimension tool is still on; Esc goes to Select.
        h.key(Key::Escape);
        assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
        // A box selection round all of it.
        let (from, to) = (h.screen(Point::new(-60.0, -60.0)), h.screen(Point::new(60.0, 40.0)));
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let sel = h.st.sketch.as_ref().unwrap().selection.clone();
        assert_eq!(sel.iter().filter(|p| matches!(p, Pick::Curve(_))).count(), 4, "{sel:?}");
        // Ctrl+A, then Esc clears the selection.
        h.key(Key::Escape);
        assert!(h.st.sketch.as_ref().unwrap().selection.is_empty());
        h.press(Modifiers::COMMAND, Key::A);
        assert!(h.st.sketch.as_ref().unwrap().selection.len() >= 4);
        // A right click selects the line under the pointer.
        h.key(Key::Escape);
        h.click(PointerButton::Secondary, line);
        assert!(matches!(
            h.st.sketch.as_ref().unwrap().selection.as_slice(),
            [Pick::Curve(_)]
        ));
        assert!(h.st.sketch.as_ref().unwrap().menu_at.is_some());
        // No undo step from all the drawing.
        assert!(h.st.sketch.is_some());
    }

    #[test]
    fn delete_a_point_with_its_lines_and_the_constraints_of_a_part() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let s = sk(&h);
        let corner = s.points[0].id;
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Point(corner)];
        h.st.delete_sketch_selection();
        assert_eq!(sk(&h).curves.len(), 2, "the two lines of the corner go");
        h.st.doc.undo();
        let line = sk(&h).curves[0].id;
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(line)];
        h.st.remove_sketch_constraints_of_selection();
        let s = sk(&h);
        assert_eq!(s.curves.len(), 4);
        assert!(!s.constraints.iter().any(|c| c.constraint.curves().contains(&line)));
        // The tool values follow the units.
        let v = EditValues::for_units(crate::workspace::Units::Inch);
        assert!((v.fillet_mm - 3.175).abs() < 1e-9);
    }

    #[test]
    fn rectangles_slots_and_polygons() {
        let mut h = sketching();
        // A center rectangle: the center is the middle of a diagonal.
        click_tool(
            &mut h,
            SketchTool::RectCenter,
            &[Point::new(0.0, 0.0), Point::new(20.0, 10.0)],
        );
        let s = sk(&h);
        assert!(s.points.iter().any(|p| p.pos.distance(Point::new(-20.0, -10.0)) < 1e-9));
        assert_eq!(count(&s, |c| matches!(c, Constraint::Midpoint { .. })), 1);
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
        // A rectangle at an angle.
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rect3,
            &[Point::new(0.0, 0.0), Point::new(30.0, 17.0), Point::new(20.0, 40.0)],
        );
        let s = sk(&h);
        assert_eq!(s.curves.len(), 4);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Perpendicular { .. })), 1);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Parallel { .. })), 2);
        assert_eq!(h.st.sketch_analysis().unwrap().free, 5, "place, angle, two sizes");
        // A slot: two centers and a width; 5 free motions.
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Slot,
            &[Point::new(0.0, 0.0), Point::new(40.0, 1.0), Point::new(20.0, 8.0)],
        );
        let s = sk(&h);
        assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 4);
        let a = h.st.sketch_analysis().unwrap();
        assert!(a.conflicting.is_empty());
        assert_eq!(a.free, 4, "the centers (the axis is horizontal) and the width");
        // A hexagon round its circle: the sides touch the circle.
        let mut h = sketching();
        h.st.sketch.as_mut().unwrap().polygon_outside = true;
        click_tool(
            &mut h,
            SketchTool::Polygon,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        );
        let s = sk(&h);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 6);
        let far = s.points.iter().map(|p| p.pos.length()).fold(0.0, f64::max);
        assert!((far - 10.0 / (std::f64::consts::PI / 6.0).cos()).abs() < 1e-9);
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
        // Collinear from the selection.
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(10.0, 3.0)]);
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(20.0, 5.0), Point::new(30.0, 4.0)],
        );
        let s = sk(&h);
        let sel = vec![Pick::Curve(s.curves[0].id), Pick::Curve(s.curves[1].id)];
        assert!(h.st.apply_constraint(Kind::Collinear, &sel));
        assert_eq!(count(&sk(&h), |c| matches!(c, Constraint::Collinear { .. })), 1);
    }

    #[test]
    fn dimensions_go_where_the_click_is_and_repeated_ones_are_driven() {
        use eframe::egui::{Modifiers, PointerButton};
        let mut h = sketching();
        h.st.set_sketch_tool(SketchTool::Rectangle);
        let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
        h.click(PointerButton::Primary, a);
        h.click(PointerButton::Primary, b);
        // The bottom line, then a place: the label goes there.
        h.st.set_sketch_tool(SketchTool::Dimension);
        let bottom = h.screen(Point::new(0.0, -20.0));
        h.click(PointerButton::Primary, bottom);
        // The ghost draws while the pointer moves.
        let place = h.screen(Point::new(10.0, -40.0));
        h.frame(vec![eframe::egui::Event::PointerMoved(place)]);
        h.click(PointerButton::Primary, place);
        let s = sk(&h);
        let (id, at) = s.label_at.iter().next().map(|(i, p)| (*i, *p)).expect("a label place");
        assert!(at.distance(Point::new(10.0, -40.0)) < 0.5, "{at:?}");
        h.st.sketch.as_mut().unwrap().value_edit = None;
        // The top line has the same length: its dimension is driven.
        let top = h.screen(Point::new(0.0, 20.0));
        h.click(PointerButton::Primary, top);
        let over = h.screen(Point::new(0.0, 35.0));
        h.click(PointerButton::Primary, over);
        let s = sk(&h);
        let driven: Vec<ConstraintId> = s.constraints.iter().filter(|c| c.driven).map(|c| c.id).collect();
        assert_eq!(driven.len(), 1, "{}", h.st.message);
        assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none());
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
        // Driving again: it repeats the other one.
        h.st.toggle_driven(driven[0]);
        assert!(!sk(&h).is_driven(driven[0]));
        h.st.doc.undo();
        assert!(sk(&h).is_driven(driven[0]));
        // Drag the label of the first dimension: one undo step.
        h.key(eframe::egui::Key::Escape);
        assert_eq!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Select);
        let from = h.screen(at);
        let to = h.screen(Point::new(10.0, -55.0));
        let undo = h.st.doc.undo_len();
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        assert!(sk(&h).label_at[&id].distance(Point::new(10.0, -55.0)) < 0.5);
        assert_eq!(h.st.doc.undo_len(), undo + 1);
        // F9 hides the marks: a click on a mark does not pick it.
        h.key(eframe::egui::Key::F9);
        assert!(!h.st.sketch.as_ref().unwrap().show_marks);
        h.key(eframe::egui::Key::F8);
        assert!(h.st.sketch.as_ref().unwrap().show_marks);
        // A point and a line: the dimension waits for a click that places it.
        h.st.set_sketch_tool(SketchTool::Dimension);
        let n = sk(&h).constraints.len();
        let corner = h.screen(Point::new(-30.0, 20.0));
        h.click(PointerButton::Primary, corner);
        h.click(PointerButton::Primary, bottom);
        assert!(h.st.sketch.as_ref().unwrap().pending_dim.is_some());
        assert_eq!(sk(&h).constraints.len(), n, "not yet");
        let place = h.screen(Point::new(-50.0, 0.0));
        h.click(PointerButton::Primary, place);
        let s = sk(&h);
        assert_eq!(s.constraints.len(), n + 1);
        let last = s.constraints.last().unwrap().id;
        assert!(s.label_at[&last].distance(Point::new(-50.0, 0.0)) < 0.5);
    }

    #[test]
    fn a_drag_of_the_selection_moves_all_of_it() {
        use eframe::egui::{Modifiers, PointerButton};
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(20.0, 5.0)]);
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(0.0, 30.0), Point::new(20.0, 40.0)],
        );
        let s = sk(&h);
        let sel = vec![Pick::Curve(s.curves[0].id), Pick::Curve(s.curves[1].id)];
        h.st.set_sketch_tool(SketchTool::Select);
        h.st.sketch.as_mut().unwrap().selection = sel;
        let from = h.screen(Point::new(10.0, 2.5));
        let to = h.screen(Point::new(20.0, 12.5));
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let s = sk(&h);
        assert!(
            s.points.iter().any(|p| p.pos.distance(Point::new(10.0, 40.0)) < 0.5),
            "the second line moved"
        );
        assert!(s.points.iter().any(|p| p.pos.distance(Point::new(10.0, 10.0)) < 0.5));
    }

    #[test]
    fn a_drag_from_the_end_of_a_line_makes_a_tangent_arc() {
        use eframe::egui::{Modifiers, PointerButton};
        let mut h = sketching();
        h.st.set_sketch_tool(SketchTool::Line);
        let (a, b) = (h.screen(Point::new(-30.0, 0.0)), h.screen(Point::new(0.0, 0.0)));
        h.click(PointerButton::Primary, a);
        h.click(PointerButton::Primary, b);
        // Drag from the end of the line.
        let to = h.screen(Point::new(15.0, -15.0));
        h.drag(PointerButton::Primary, b, to, Modifiers::NONE);
        let s = sk(&h);
        let arc = s.curves.iter().find(|c| !c.is_line()).expect("an arc");
        assert!(
            s.constraints
                .iter()
                .any(|c| matches!(c.constraint, Constraint::Tangent { b, .. } if b == arc.id))
        );
        // The chain goes on from the end of the arc.
        let e = h.st.sketch.as_ref().unwrap();
        assert_eq!(e.clicks.len(), 1);
        assert!(
            e.clicks[0].at.distance(Point::new(15.0, -15.0)) < 0.5,
            "{:?}",
            e.clicks[0]
        );
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    }

    #[test]
    fn move_rotate_and_scale_the_selection() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(10.0, 3.0)]);
        let s = sk(&h);
        let l = s.curves[0].id;
        // Nothing selected: refused.
        h.st.set_sketch_tool(SketchTool::Move);
        assert_ne!(h.st.sketch.as_ref().unwrap().tool, SketchTool::Move);
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        h.st.set_sketch_tool(SketchTool::Move);
        // A base point, then where it goes.
        h.st.edit_click(SketchTool::Move, Point::new(0.0, 0.0), 0.5);
        h.st.edit_click(SketchTool::Move, Point::new(20.0, 20.0), 0.5);
        let s = sk(&h);
        assert!(
            s.points.iter().any(|p| p.pos.distance(Point::new(20.0, 20.0)) < 1e-6),
            "{}",
            h.st.message
        );
        // By values.
        h.st.apply_motion(SketchTool::Move, Motion::Shift(Point::new(-20.0, -20.0)), false);
        assert!(
            sk(&h)
                .points
                .iter()
                .any(|p| p.pos.distance(Point::new(0.0, 0.0)) < 1e-6)
        );
        // A turn of 90° round the origin: counterclockwise on the screen
        // (the y of the design goes down).
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(l)];
        h.st.set_sketch_tool(SketchTool::Rotate);
        h.st.edit_click(SketchTool::Rotate, Point::new(0.0, 0.0), 0.5);
        assert!(
            sk(&h)
                .points
                .iter()
                .any(|p| p.pos.distance(Point::new(3.0, -10.0)) < 1e-6)
        );
        // A scaled copy.
        h.st.sketch.as_mut().unwrap().values.copy_parts = true;
        h.st.set_sketch_tool(SketchTool::Scale);
        let n = sk(&h).curves.len();
        h.st.edit_click(SketchTool::Scale, Point::new(0.0, 0.0), 0.5);
        let s = sk(&h);
        assert_eq!(s.curves.len(), n + 1);
        assert!(s.points.iter().any(|p| p.pos.distance(Point::new(6.0, -20.0)) < 1e-6));
    }

    #[test]
    fn the_place_of_a_dimension_chooses_its_kind() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(0.0, 0.0), Point::new(30.0, 20.0)],
        );
        let s = sk(&h);
        let l = s.curves[0].id;
        let len = Constraint::Length { line: l, mm: 0.0 };
        // Below the line (y bigger) and between its x: horizontal.
        assert!(matches!(
            orient(&s, len, Point::new(15.0, 40.0)),
            Constraint::HorizontalDistance { mm, .. } if (mm - 30.0).abs() < 1e-9
        ));
        assert!(matches!(
            orient(&s, len, Point::new(-10.0, 10.0)),
            Constraint::VerticalDistance { mm, .. } if (mm - 20.0).abs() < 1e-9
        ));
        assert!(matches!(
            orient(&s, len, Point::new(40.0, 30.0)),
            Constraint::Length { .. }
        ));
        // The kinds of an arc: radius, diameter, arc length.
        click_tool(
            &mut h,
            SketchTool::ArcCenter,
            &[Point::new(0.0, 50.0), Point::new(10.0, 50.0), Point::new(0.0, 60.0)],
        );
        let s = sk(&h);
        let arc = s.curves.iter().find(|c| !c.is_line()).unwrap().id;
        let kinds = dimension_kinds(&s, &Constraint::Radius { curve: arc, mm: 10.0 });
        assert_eq!(kinds.len(), 3);
        let Constraint::ArcLength { mm, .. } = kinds[2].1 else {
            panic!("an arc length")
        };
        assert!((mm - 5.0 * std::f64::consts::PI).abs() < 1e-6, "{mm}");
        h.st.add_sketch_dimension(kinds[2].1, None);
        assert!(
            sk(&h)
                .constraints
                .iter()
                .any(|c| matches!(c.constraint, Constraint::ArcLength { .. }))
        );
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    }

    #[test]
    fn centerlines_and_relax_mode() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(0.0, 40.0)]);
        click_tool(&mut h, SketchTool::Point, &[Point::new(-8.0, 20.0)]);
        let s = sk(&h);
        let (axis, q) = (s.curves[0].id, s.points[2].id);
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(axis)];
        h.st.toggle_centerline();
        assert!(sk(&h).is_centerline(axis));
        // A point and the centerline: a diameter of 16.
        let Build::Done(c) = dimension_of(&sk(&h), &[Pick::Point(q), Pick::Curve(axis)], false) else {
            panic!("a dimension")
        };
        assert!((c.value().unwrap().abs() - 16.0).abs() < 1e-9, "{c:?}");
        // Relax mode: a drag changes a dimension that holds the part.
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(20.0, 0.0)]);
        let l = sk(&h).curves[0].id;
        let d =
            h.st.edit_sketch("Dimension", None, |sk| {
                sk.add_constraint(Constraint::Length { line: l, mm: 20.0 })
            })
            .unwrap()
            .unwrap();
        let Geometry::Line { b, .. } = sk(&h).curve(l).unwrap().geometry else {
            unreachable!()
        };
        h.st.sketch.as_mut().unwrap().relax_mode = true;
        let moves = [(b, Point::new(30.0, 0.0))];
        h.st.edit_sketch("Drag in Sketch", None, |sk| {
            let dims = relax_dims(sk, true);
            sk.drag_points(&moves);
            restore_dims(sk, &dims);
        });
        let s = sk(&h);
        assert!((s.constraint(d).unwrap().value().unwrap() - 30.0).abs() < 1e-6);
        assert!(!s.is_driven(d), "it drives again");
    }

    /// Runs draw tool `tool` with the snaps `clicks`.
    fn snap_tool(h: &mut Harness, tool: SketchTool, clicks: &[Snap]) {
        h.st.set_sketch_tool(tool);
        let opts = MakeOpts::of(h.st.sketch.as_ref().unwrap());
        let clicks = clicks.to_vec();
        let r = h.st.edit_sketch("Sketch", None, |sk| make(tool, sk, &clicks, opts));
        assert!(r.flatten().is_some(), "{tool:?} made nothing");
    }

    #[test]
    fn slots_of_each_kind_and_a_center_rectangle_at_an_angle() {
        for kind in SlotKind::ALL {
            let mut h = sketching();
            h.st.sketch.as_mut().unwrap().slot_kind = kind;
            let pts: Vec<Point> = match kind {
                SlotKind::Arc3 => vec![
                    Point::new(-30.0, 0.0),
                    Point::new(30.0, 0.0),
                    Point::new(0.0, -30.0),
                    Point::new(0.0, -35.0),
                ],
                SlotKind::ArcCenter => vec![
                    Point::new(0.0, 0.0),
                    Point::new(30.0, 0.0),
                    Point::new(0.0, 30.0),
                    Point::new(25.0, 10.0),
                ],
                _ => vec![Point::new(0.0, 0.0), Point::new(40.0, 0.0), Point::new(20.0, 6.0)],
            };
            let clicks: Vec<Snap> = pts.iter().map(|q| Snap::free(*q)).collect();
            assert_eq!(
                clicks_needed(SketchTool::Slot, &MakeOpts::of(h.st.sketch.as_ref().unwrap())),
                clicks.len()
            );
            snap_tool(&mut h, SketchTool::Slot, &clicks);
            let s = sk(&h);
            assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4, "{kind:?}");
            let a = h.st.sketch_analysis().unwrap();
            assert!(a.conflicting.is_empty(), "{kind:?} {a:?}");
            if kind == SlotKind::Overall {
                // The ends are where the clicks were: the centers are a
                // half width (6) in.
                for x in [6.0, 34.0] {
                    assert!(
                        s.points.iter().any(|p| p.pos.distance(Point::new(x, 0.0)) < 1e-6),
                        "{x}"
                    );
                }
            }
        }
        // A center rectangle at an angle: the center is the middle.
        let mut h = sketching();
        let clicks = [Point::new(0.0, 0.0), Point::new(20.0, 10.0), Point::new(15.0, 30.0)].map(Snap::free);
        snap_tool(&mut h, SketchTool::RectCenter3, &clicks);
        let s = sk(&h);
        assert_eq!(s.curves.iter().filter(|c| !c.construction).count(), 4);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Midpoint { .. })), 1);
        // A corner: the center, a half side along (20, 10), a half height across.
        let d = Point::new(20.0, 10.0);
        let n = d.normalized().unwrap().perp();
        let h_ = (Point::new(15.0, 30.0) - d).dot(n);
        let corner = d + n * h_;
        assert!(s.points.iter().any(|p| p.pos.distance(corner) < 1e-6), "{corner:?}");
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
    }

    #[test]
    fn a_tangent_circle_a_chamfer_with_an_angle_and_a_stretch() {
        let mut h = sketching();
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(30.0, 0.0), Point::new(0.0, 40.0)],
        );
        click_tool(&mut h, SketchTool::Line, &[Point::new(0.0, 40.0), Point::new(0.0, 0.0)]);
        let s = sk(&h);
        let l: Vec<CurveId> = s.curves.iter().map(|c| c.id).collect();
        let clicks = [
            Snap::on_curve(Point::new(10.0, 0.0), l[0]),
            Snap::on_curve(Point::new(15.0, 20.0), l[1]),
            Snap::on_curve(Point::new(0.0, 10.0), l[2]),
        ];
        snap_tool(&mut h, SketchTool::TangentCircle, &clicks);
        let s = sk(&h);
        let circle = s.curves.iter().find(|c| !c.is_line()).unwrap().id;
        assert!((s.radius(circle).unwrap() - 10.0).abs() < 1e-6);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 3);
        // A chamfer with a distance and an angle.
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        {
            let v = &mut h.st.sketch.as_mut().unwrap().values;
            v.chamfer_kind = 2;
            v.chamfer_mm = 5.0;
            v.chamfer_deg = 30.0;
        }
        h.st.edit_click(SketchTool::Chamfer, Point::new(60.0, 40.0), 0.5);
        let s = sk(&h);
        assert_eq!(
            count(&s, |c| matches!(c, Constraint::Angle { .. })),
            1,
            "{}",
            h.st.message
        );
        assert!(h.st.sketch_analysis().unwrap().conflicting.is_empty());
        // Stretch: the two right corners move, the rectangle gets wider.
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let right: Vec<PointId> = sk(&h).points.iter().filter(|p| p.pos.x > 30.0).map(|p| p.id).collect();
        assert_eq!(right.len(), 2);
        h.st.sketch.as_mut().unwrap().source_points = right;
        h.st.apply_motion(SketchTool::Stretch, Motion::Shift(Point::new(20.0, 0.0)), false);
        let s = sk(&h);
        let xs: Vec<f64> = s.points.iter().map(|p| p.pos.x).collect();
        assert!(
            xs.iter().any(|x| (x - 80.0).abs() < 1e-6) && xs.iter().any(|x| x.abs() < 1e-6),
            "{xs:?}"
        );
    }

    #[test]
    fn center_points_and_automatic_dimensions() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        click_tool(&mut h, SketchTool::Point, &[Point::new(30.0, 20.0)]);
        let s = sk(&h);
        let q = s.points.last().unwrap().id;
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Point(q)];
        h.st.toggle_center_point();
        assert!(sk(&h).is_center_point(q));
        h.st.sketch.as_mut().unwrap().selection.clear();
        let undo = h.st.doc.undo_len();
        h.st.auto_dimension();
        assert_eq!(h.st.doc.undo_len(), undo + 1, "one undo step");
        let s = sk(&h);
        assert!(
            count(&s, |c| matches!(c, Constraint::Length { .. })) >= 2,
            "{}",
            h.st.message
        );
        let a = h.st.sketch_analysis().unwrap();
        assert!(a.conflicting.is_empty() && a.redundant.is_empty());
        // The rectangle has its place free and the point both of its places.
        assert_eq!(a.free, 4, "{}", h.st.message);
    }

    #[test]
    fn a_length_label_is_on_its_dimension_line() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(0.0, 0.0), Point::new(40.0, 30.0)],
        );
        let l = sk(&h).curves[0].id;
        h.st.edit_sketch("Dimension", None, |sk| {
            sk.add_constraint(Constraint::Length { line: l, mm: 50.0 })
        });
        let obj = h.st.sketch_obj().unwrap().clone();
        let map = h.map();
        let lab = labels(&obj, &map, crate::workspace::Units::Mm)
            .into_iter()
            .find(|x| x.dimension)
            .unwrap();
        let (p, q) = lab.leader.unwrap();
        let m = p.lerp(q, 0.5);
        assert!(lab.rect.center().distance(m) < 1.0, "{:?} {m:?}", lab.rect.center());
    }

    fn kinds(h: &Harness) -> Vec<&'static str> {
        h.st.doc
            .design()
            .objects()
            .iter()
            .map(|o| o.kind.type_name())
            .filter(|n| *n != "Sketch")
            .collect()
    }

    #[test]
    fn vectors_and_dxf_drawings_become_sketches() {
        let mut h = Harness::empty();
        let path = emb_geom::Path::rounded_rect(emb_geom::BBox::new(Point::new(0.0, 0.0), Point::new(50.0, 30.0)), 5.0);
        let v = h.st.doc.edit("v", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Vector {
                    path,
                    style: emb_model::VectorStyle::filled(emb_model::Rgb(200, 0, 0)),
                },
            )
        });
        h.st.selection.set(v);
        h.st.vectors_to_sketch();
        let d = h.st.doc.design();
        let vector = |d: &emb_model::Design| d.objects().iter().any(|o| matches!(o.kind, ObjectKind::Vector { .. }));
        assert!(!vector(d), "the vector goes");
        let id = h.st.selection.single().unwrap();
        let Some(ObjectKind::Sketch(s)) = d.object(id).map(|o| &o.kind) else {
            panic!("a sketch")
        };
        assert_eq!(s.sketch().curves.len(), 8);
        assert_eq!(s.regions().len(), 1);
        h.st.doc.undo();
        assert!(vector(h.st.doc.design()), "one undo step");
        // A DXF drawing.
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n12\n0\nENDSEC\n0\nEOF\n";
        h.st.import_dxf_bytes(dxf.as_bytes(), "ring.dxf");
        let d = h.st.doc.design();
        let id = h.st.selection.single().unwrap();
        let b = d.bbox_of(&[id]).unwrap();
        assert!((b.width() - 24.0).abs() < 1e-6);
        assert!(
            (b.center() - d.hoop.center).length() < 1e-6,
            "in the middle of the hoop"
        );
    }

    #[test]
    fn the_spline_tool_draws_a_smooth_curve() {
        let mut h = sketching();
        h.st.set_sketch_tool(SketchTool::Spline);
        let tol = 0.5;
        for q in [Point::new(0.0, 0.0), Point::new(20.0, 15.0), Point::new(40.0, 0.0)] {
            spline_click(&mut h.st, Snap::free(q), tol);
        }
        assert_eq!(h.st.sketch.as_ref().unwrap().clicks.len(), 3);
        // A click on the last point ends it: two pieces and a tangent.
        spline_click(&mut h.st, Snap::free(Point::new(40.0, 0.2)), tol);
        let s = sk(&h);
        assert_eq!(s.curves.iter().filter(|c| c.is_spline()).count(), 2);
        assert!(
            s.constraints
                .iter()
                .any(|c| matches!(c.constraint, Constraint::Tangent { .. }))
        );
        assert!(h.st.sketch.as_ref().unwrap().clicks.is_empty());
        // A closed spline: a click on the first point.
        for q in [Point::new(0.0, 50.0), Point::new(30.0, 50.0), Point::new(30.0, 80.0)] {
            spline_click(&mut h.st, Snap::free(q), tol);
        }
        spline_click(&mut h.st, Snap::free(Point::new(0.2, 50.0)), tol);
        assert_eq!(sk(&h).curves.iter().filter(|c| c.is_spline()).count(), 5);
        assert_eq!(h.st.sketch_regions().len(), 1, "the closed spline is a region");
        // A dimension does not take a spline.
        let c = sk(&h).curves[0].id;
        assert!(matches!(
            dimension_of(&sk(&h), &[Pick::Curve(c)], true),
            Build::Wrong(_)
        ));
    }

    #[test]
    fn typed_names_and_the_parameters_window_drive_the_dimensions() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let bottom = sk(&h).curves[0].id;
        let id =
            h.st.edit_sketch("Dimension", None, |s| {
                s.add_constraint(Constraint::Length { line: bottom, mm: 60.0 }).unwrap()
            })
            .unwrap();
        h.st.toggle_parameters();
        assert!(h.st.sketch.as_ref().unwrap().params.open);
        h.st.param_action(ParamAction::Set("width".into(), "70 mm".into()));
        h.st.set_dimension_text(id, "width + 10");
        let len = |h: &Harness| sk(h).constraint(id).unwrap().value().unwrap();
        assert!((len(&h) - 80.0).abs() < 1e-9);
        assert_eq!(sk(&h).dimension_expr(id), Some("width + 10"));
        // A change of the parameter: the dimension and the sketch follow.
        let steps = h.st.doc.undo_len();
        h.st.param_action(ParamAction::Set("width".into(), "50".into()));
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert!((len(&h) - 60.0).abs() < 1e-9);
        // A loop is refused with a message and no undo step.
        h.st.param_action(ParamAction::Set("depth".into(), "width".into()));
        h.st.param_action(ParamAction::Set("width".into(), "depth".into()));
        let err = h.st.sketch.as_ref().unwrap().params.error.clone().unwrap();
        assert!(err.contains("loop"), "{err}");
        assert!((len(&h) - 60.0).abs() < 1e-9);
        // An unknown name in the value box.
        h.st.set_dimension_text(id, "height");
        assert!(h.st.message.contains("height"), "{}", h.st.message);
        // A plain number ends the expression.
        h.st.set_dimension_text(id, "45");
        assert_eq!(sk(&h).dimension_expr(id), None);
        assert!((len(&h) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn a_template_with_parameters_asks_for_their_values() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let bottom = sk(&h).curves[0].id;
        let id =
            h.st.edit_sketch("Dimension", None, |s| {
                s.add_constraint(Constraint::Length { line: bottom, mm: 60.0 }).unwrap()
            })
            .unwrap();
        h.st.param_action(ParamAction::Set("width".into(), "60 mm".into()));
        h.st.set_dimension_text(id, "width");
        h.st.make_picked_regions(RegionTo::Fill);
        h.st.finish_sketch();
        let dir = std::env::temp_dir().join(format!("emb-template-ask-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Badge.embtemplate");
        let t = emb_model::template::from_design(h.st.doc.design(), "Badge", true);
        emb_model::template::save(&t, &path).unwrap();
        h.st.new_from_template(Some(&path));
        std::fs::remove_dir_all(&dir).ok();
        let ask = h.st.template_ask.as_mut().expect("the template asks");
        assert_eq!(ask.rows.len(), 1);
        ask.rows[0].1 = "wide".into();
        assert!(!h.st.make_from_template_ask(), "an unknown name is refused");
        assert!(h.st.template_ask.as_ref().unwrap().error.is_some());
        h.st.template_ask.as_mut().unwrap().rows[0].1 = "90 mm".into();
        assert!(h.st.make_from_template_ask());
        let d = h.st.doc.design();
        let fill = d
            .objects()
            .iter()
            .find(|o| matches!(o.kind, ObjectKind::Fill { .. }))
            .unwrap()
            .id;
        assert!((d.bbox_of(&[fill]).unwrap().width() - 90.0).abs() < 1e-3);
    }

    #[test]
    fn objects_made_from_a_sketch_follow_its_changes() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        let sketch = h.st.sketch.as_ref().unwrap().object;
        let bottom = sk(&h).curves[0].id;
        h.st.make_picked_regions(RegionTo::Fill);
        let fill = h.st.doc.design().objects().last().unwrap().id;
        let width = |h: &Harness| h.st.doc.design().bbox_of(&[fill]).unwrap().width();
        assert!((width(&h) - 60.0).abs() < 1e-3);
        // A length of 80 mm: the fill is made again, in one undo step.
        let steps = h.st.doc.undo_len();
        h.st.edit_sketch("Dimension", None, |s| {
            s.add_constraint(Constraint::Length { line: bottom, mm: 80.0 }).unwrap();
        });
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        assert!((width(&h) - 80.0).abs() < 1e-3, "{}", width(&h));
        h.st.doc.undo();
        assert!((width(&h) - 60.0).abs() < 1e-3);
        // A line that is gone: the fill stays, with a warning.
        h.st.edit_sketch("Delete", None, |s| {
            s.remove_curve(bottom);
        });
        assert!(h.st.message.contains("Unlink"), "{}", h.st.message);
        assert!((width(&h) - 60.0).abs() < 1e-3);
        h.st.doc.undo();
        // After Finish Sketch, the fill opens its sketch; Unlink ends the link.
        h.st.finish_sketch();
        assert_eq!(h.st.doc.design().sketch_for(fill), Some(sketch));
        h.st.selection.set(fill);
        h.st.unlink_selection();
        assert!(!h.st.doc.design().is_linked(fill));
    }

    #[test]
    fn picked_regions_become_fills_and_curves_become_runs_and_satins() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        click_tool(
            &mut h,
            SketchTool::Line,
            &[Point::new(20.0, -5.0), Point::new(20.0, 45.0)],
        );
        assert_eq!(h.st.sketch_regions().len(), 2);
        // Pick the small region, then unpick and pick it again.
        h.st.set_sketch_tool(SketchTool::Regions);
        h.st.toggle_region(Point::new(10.0, 20.0));
        h.st.toggle_region(Point::new(12.0, 22.0));
        assert!(
            h.st.sketch.as_ref().unwrap().picked.is_empty(),
            "the same region unpicks"
        );
        h.st.toggle_region(Point::new(10.0, 20.0));
        let r = h.st.picked_regions();
        assert_eq!(r.len(), 1);
        assert!((r[0].area - 800.0).abs() < 1e-3);
        h.st.make_picked_regions(RegionTo::Fill);
        assert_eq!(kinds(&h), vec!["Fill"]);
        assert!(h.st.sketch.as_ref().unwrap().picked.is_empty());
        // Nothing picked: all regions.
        h.st.make_picked_regions(RegionTo::Applique);
        assert_eq!(kinds(&h), vec!["Fill", "Appliqué", "Appliqué"]);
        h.st.doc.undo();
        assert_eq!(kinds(&h), vec!["Fill"]);
        // Curves: a run and a satin line along the top line, a satin
        // column between the top and the bottom lines.
        let s = sk(&h);
        let (bottom, top) = (s.curves[0].id, s.curves[2].id);
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(top)];
        h.st.make_curves(CurveTo::Run);
        h.st.make_curves(CurveTo::SatinLine(3.0));
        h.st.sketch.as_mut().unwrap().selection = vec![Pick::Curve(top), Pick::Curve(bottom)];
        h.st.make_satin_between();
        assert_eq!(kinds(&h), vec!["Fill", "Running", "Satin", "Satin"]);
        // The objects sew.
        for o in
            h.st.doc
                .design()
                .objects()
                .iter()
                .filter(|o| o.kind.type_name() != "Sketch")
        {
            let s = emb_stitch::stitch_object(o, Point::ZERO).unwrap();
            let n: usize = s.runs.iter().map(|r| r.points.len()).sum();
            assert!(n > 10, "{}: {n}", o.kind.type_name());
        }
    }

    #[test]
    fn the_sketch_tab_and_the_panels_draw() {
        let mut h = sketching();
        click_tool(
            &mut h,
            SketchTool::Rectangle,
            &[Point::new(0.0, 0.0), Point::new(60.0, 40.0)],
        );
        h.frame(vec![]);
        let before = h.st.doc.state_id();
        for tool in [
            SketchTool::Select,
            SketchTool::Move,
            SketchTool::Rotate,
            SketchTool::Scale,
            SketchTool::Fillet,
            SketchTool::Chamfer,
            SketchTool::Stretch,
            SketchTool::Offset,
            SketchTool::RectPattern,
            SketchTool::CircularPattern,
        ] {
            h.st.sketch.as_mut().unwrap().tool = tool;
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| edit_panel(ui, st));
            });
            out.textures_delta.clear();
            assert!(h.st.sketch_status().is_some());
        }
        assert_eq!(h.st.doc.state_id(), before, "drawing the panel changes nothing");
        assert!(h.st.sketch_status().is_some());
        // An undo of Start Sketch ends sketch mode.
        while h.st.doc.can_undo() {
            h.st.doc.undo();
        }
        h.st.check_sketch();
        assert!(h.st.sketch.is_none());
    }
}
