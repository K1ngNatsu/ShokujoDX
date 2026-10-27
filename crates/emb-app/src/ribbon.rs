//! The ribbon: tabs at the top of the window with the commands in panels,
//! as in CAD programs. See ADR 0010.
//!
//! - The File button opens the application menu (new, open, save, import,
//!   export). Next to it are New, Open, Save, Undo and Redo.
//! - Each tab has panels. A panel has large buttons (icon above the name)
//!   and columns of up to three small buttons (icon left of the name). The
//!   panel name is below its buttons.
//! - Double-click a tab, or click the arrow at the right, to hide the panels
//!   and get more space for the design. Click a tab to show them again.
//! - Commands that need a selection are dimmed when there is none.

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Vec2, vec2};
use egui_phosphor::regular as ph;
use emb_model::{HoopSelection, ObjectKind, RunPass, Step};
use serde::{Deserialize, Serialize};

use crate::actions::Align;
use crate::app::AppState;
use crate::draw::Tool;
use crate::file::Pending;
use crate::functions::Func;
use crate::i18n::tr;
use crate::icons::{self, Drawing, Icon};
use crate::vectors::ArtworkImport;
use crate::viewing::{Travel, ViewFilter};

/// The tabs of the ribbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RibbonTab {
    #[default]
    Digitize,
    /// Lettering and personalization (monograms, team names).
    Lettering,
    Artwork,
    /// Specialty decoration (P9): cross stitch and rhinestones.
    Specialty,
    Edit,
    Arrange,
    Color,
    View,
    Design,
    Hooping,
    /// Help, tutorials and the start page (P10 Milestone 4).
    Help,
    /// Sketch mode (P14 Milestone 2): it shows only while a sketch is open.
    Sketch,
}

impl RibbonTab {
    pub const ALL: [RibbonTab; 12] = [
        RibbonTab::Digitize,
        RibbonTab::Lettering,
        RibbonTab::Artwork,
        RibbonTab::Specialty,
        RibbonTab::Edit,
        RibbonTab::Arrange,
        RibbonTab::Color,
        RibbonTab::View,
        RibbonTab::Design,
        RibbonTab::Hooping,
        RibbonTab::Help,
        RibbonTab::Sketch,
    ];

    /// The tabs that show: all, or (simple mode) the tabs of the simple
    /// tasks. The Sketch tab shows only in sketch mode (`show`).
    pub fn visible(simple: bool) -> &'static [RibbonTab] {
        const NORMAL: [RibbonTab; 11] = [
            RibbonTab::Digitize,
            RibbonTab::Lettering,
            RibbonTab::Artwork,
            RibbonTab::Specialty,
            RibbonTab::Edit,
            RibbonTab::Arrange,
            RibbonTab::Color,
            RibbonTab::View,
            RibbonTab::Design,
            RibbonTab::Hooping,
            RibbonTab::Help,
        ];
        const SIMPLE: [RibbonTab; 6] = [
            RibbonTab::Digitize,
            RibbonTab::Lettering,
            RibbonTab::Edit,
            RibbonTab::View,
            RibbonTab::Design,
            RibbonTab::Help,
        ];
        if simple { &SIMPLE } else { &NORMAL }
    }

    /// The panels of the tab in simple mode (large buttons only), or all.
    fn panels_in(self, simple: bool) -> &'static [Panel] {
        if !simple {
            return self.panels();
        }
        match self {
            RibbonTab::Digitize => S_DIGITIZE,
            RibbonTab::Lettering => S_LETTERING,
            RibbonTab::Edit => S_EDIT,
            RibbonTab::View => S_VIEW,
            RibbonTab::Design => S_DESIGN,
            _ => self.panels(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RibbonTab::Digitize => tr("Digitize"),
            RibbonTab::Lettering => tr("Lettering"),
            RibbonTab::Artwork => tr("Artwork"),
            RibbonTab::Specialty => tr("Specialty"),
            RibbonTab::Edit => tr("Edit"),
            RibbonTab::Arrange => tr("Arrange"),
            RibbonTab::Color => tr("Color"),
            RibbonTab::View => tr("View"),
            RibbonTab::Design => tr("Design"),
            RibbonTab::Hooping => tr("Hooping"),
            RibbonTab::Help => tr("Help"),
            RibbonTab::Sketch => tr("Sketch"),
        }
    }

    fn panels(self) -> &'static [Panel] {
        match self {
            RibbonTab::Digitize => DIGITIZE,
            RibbonTab::Lettering => LETTERING,
            RibbonTab::Artwork => ARTWORK,
            RibbonTab::Specialty => SPECIALTY,
            RibbonTab::Edit => EDIT,
            RibbonTab::Arrange => ARRANGE,
            RibbonTab::Color => COLOR,
            RibbonTab::View => VIEW,
            RibbonTab::Design => DESIGN,
            RibbonTab::Hooping => HOOPING,
            RibbonTab::Help => HELP,
            RibbonTab::Sketch => SKETCH,
        }
    }
}

/// A command of the Sketch tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchCmd {
    Finish,
    Tool(crate::sketch::SketchTool),
    Construction,
    /// Remove the constraints that conflict (P14 Milestone 3).
    Relax,
    /// Relax mode: a drag changes the dimensions (ADR 0073).
    RelaxMode,
    /// Make the selected lines centerlines (ADR 0073).
    Centerline,
    /// Make the selected points center points (ADR 0074).
    CenterPoint,
    /// Automatic dimensions and constraints (ADR 0074).
    AutoDimension,
    /// The Parameters window (ADR 0078).
    Parameters,
}

/// Icon, name and help text of a command of the Sketch tab.
fn sketch_meta(c: SketchCmd) -> (Icon, &'static str, &'static str) {
    use crate::sketch::{Kind, SketchTool as T};
    let g = Icon::Glyph;
    match c {
        SketchCmd::Finish => (
            g(ph::CHECK_CIRCLE),
            tr("Finish Sketch"),
            tr("Close the sketch and go back to the other tabs."),
        ),
        SketchCmd::Construction => (
            g(ph::CIRCLE_DASHED),
            tr("Construction"),
            tr(
                "Make the selected curves construction curves (they help to draw and sew nothing), or back. With nothing selected: new curves are construction curves.",
            ),
        ),
        SketchCmd::Relax => (
            g(ph::BROOM),
            tr("Relax"),
            tr("Remove the constraints that conflict with the others (red), dimensions first, and list them."),
        ),
        SketchCmd::RelaxMode => (
            g(ph::HAND_GRABBING),
            tr("Relax Mode"),
            tr("While it is on, a drag changes the dimensions that hold the parts (the other constraints stay)."),
        ),
        SketchCmd::Centerline => (
            g(ph::LINE_VERTICAL),
            tr("Centerline"),
            tr(
                "Make the selected lines centerlines (construction lines). A dimension from a point to a centerline is a diameter.",
            ),
        ),
        SketchCmd::CenterPoint => (
            g(ph::CROSSHAIR),
            tr("Center Point"),
            tr("Make the selected points center points (a cross), for example the places of holes."),
        ),
        SketchCmd::Parameters => (
            g(ph::FUNCTION),
            tr("Parameters"),
            tr(
                "Named values (for example width = 80 mm) and expressions (border = width / 20) that the dimensions use.",
            ),
        ),
        SketchCmd::AutoDimension => (
            g(ph::MAGIC_WAND),
            tr("Automatic Dimensions"),
            tr(
                "Add the dimensions and constraints that the selected curves (or all curves) need, with their sizes of now.",
            ),
        ),
        SketchCmd::Tool(t) => match t {
            T::Regions => (
                g(ph::PAINT_BUCKET),
                tr("Regions"),
                tr(
                    "Click inside closed regions to pick them, then make fills, satins, appliqué or vectors of them (Properties panel or right click).",
                ),
            ),
            T::RectCenter3 => (
                g(ph::SQUARE_HALF),
                tr("Three-Point Center Rectangle"),
                tr("Draw a rectangle at an angle from its center: the center, the middle of a side, then a corner."),
            ),
            T::TangentCircle => (
                g(ph::CIRCLE_NOTCH),
                tr("Tangent Circle"),
                tr("Draw a circle tangent to three lines: click the three lines."),
            ),
            T::Stretch => (
                g(ph::RESIZE),
                tr("Stretch"),
                tr(
                    "Drag a box round points, then click a base point and where it goes: the curves that end there stretch.",
                ),
            ),
            T::Move => (
                g(ph::ARROWS_OUT_CARDINAL),
                tr("Move"),
                tr("Move the selected parts: click a base point, then where it goes, or type the values."),
            ),
            T::Copy => (
                g(ph::COPY),
                tr("Copy"),
                tr("Copy the selected curves: click a base point, then where the copy goes."),
            ),
            T::Rotate => (
                g(ph::ARROW_CLOCKWISE),
                tr("Rotate"),
                tr("Turn the selected parts round the point you click, by the angle in the Properties panel."),
            ),
            T::Scale => (
                g(ph::ARROWS_OUT),
                tr("Scale"),
                tr("Scale the selected parts from the point you click, by the factor in the Properties panel."),
            ),
            T::Trim => (
                g(ph::SCISSORS),
                tr("Trim"),
                tr("Click the piece of a curve to remove, between the curves that cut it."),
            ),
            T::Extend => (
                g(ph::ARROW_LINE_RIGHT),
                tr("Extend"),
                tr("Click near the end of a line or an arc to extend it to the next curve."),
            ),
            T::Split => (
                g(ph::SPLIT_HORIZONTAL),
                tr("Split"),
                tr("Click a line or an arc to cut it in two there."),
            ),
            T::Fillet => (
                g(ph::CORNERS_OUT),
                tr("Fillet"),
                tr("Click the corner of two lines to round it with an arc tangent to both."),
            ),
            T::Chamfer => (
                g(ph::KNIFE),
                tr("Chamfer"),
                tr("Click the corner of two lines to cut it with a line."),
            ),
            T::Offset => (
                g(ph::RECTANGLE_DASHED),
                tr("Offset"),
                tr(
                    "Make a copy of the selected curves, or of the curves joined to the curve you click, at a distance.",
                ),
            ),
            T::Mirror => (
                g(ph::FLIP_VERTICAL),
                tr("Mirror"),
                tr("Select the curves, click Mirror, then click the line to mirror about. The copy stays symmetric."),
            ),
            T::RectPattern => (
                g(ph::GRID_FOUR),
                tr("Rectangular Pattern"),
                tr("Copy the selected curves in columns and rows (set them in the Properties panel)."),
            ),
            T::CircularPattern => (
                g(ph::ARROWS_CLOCKWISE),
                tr("Circular Pattern"),
                tr("Select the curves, click Circular Pattern, then click the center: copies round it."),
            ),
            T::Select => (
                g(ph::CURSOR),
                tr("Select"),
                tr("Select and drag points, curves and dimensions. Double-click a dimension to change it."),
            ),
            T::Line => (
                g(ph::LINE_SEGMENTS),
                tr("Line"),
                tr("Draw a chain of lines. Click the first point to close it; Enter or Esc ends it."),
            ),
            T::Rectangle => (
                g(ph::RECTANGLE),
                tr("Rectangle"),
                tr("Draw a rectangle from two corners, with horizontal and vertical sides."),
            ),
            T::RectCenter => (
                g(ph::SELECTION),
                tr("Center Rectangle"),
                tr("Draw a rectangle from its center and a corner, with horizontal and vertical sides."),
            ),
            T::Rect3 => (
                g(ph::PARALLELOGRAM),
                tr("Three-Point Rectangle"),
                tr("Draw a rectangle at an angle: two corners of a side, then the height."),
            ),
            T::Slot => (
                g(ph::PILL),
                tr("Slot"),
                tr("Draw a slot: the two centers, then the width. The ends are half circles, tangent to the sides."),
            ),
            T::Circle => (
                g(ph::CIRCLE),
                tr("Circle"),
                tr("Draw a circle from its center and a point on it."),
            ),
            T::Arc3 => (
                g(ph::PEN),
                tr("Three-Point Arc"),
                tr("Draw an arc through its start, its end and a point on it."),
            ),
            T::ArcCenter => (
                g(ph::COMPASS),
                tr("Center Arc"),
                tr("Draw an arc from its center, its start and its end (counterclockwise)."),
            ),
            T::ArcTangent => (
                g(ph::BEZIER_CURVE),
                tr("Tangent Arc"),
                tr("Draw an arc that goes on from the end of a line, tangent to it."),
            ),
            T::Polygon => (
                g(ph::POLYGON),
                tr("Polygon"),
                tr(
                    "Draw a regular polygon from its center and a corner (the number of sides is in the Properties panel).",
                ),
            ),
            T::Point => (
                g(ph::DOT_OUTLINE),
                tr("Point"),
                tr("Add a point, for example to put a constraint on."),
            ),
            T::Spline => (
                g(ph::WAVE_SINE),
                tr("Spline"),
                tr(
                    "Draw a smooth curve through the points that you click. Enter, a right click or a click on the last point ends it; a click on the first point closes it. Drag its points and handles to shape it.",
                ),
            ),
            T::Dimension => (
                g(ph::RULER),
                tr("Dimension"),
                tr("Click a line, a circle or an arc, two points or two lines, then type the value."),
            ),
            T::Constrain(k) => match k {
                Kind::Coincident => (g(ph::GIT_COMMIT), tr("Coincident"), tr("Two points at the same place.")),
                Kind::Horizontal => (
                    g(ph::ARROWS_LEFT_RIGHT),
                    tr("Horizontal"),
                    tr("A line, or two points, horizontal."),
                ),
                Kind::Vertical => (
                    g(ph::ARROWS_DOWN_UP),
                    tr("Vertical"),
                    tr("A line, or two points, vertical."),
                ),
                Kind::Parallel => (g(ph::VECTOR_TWO), tr("Parallel"), tr("Two lines parallel.")),
                Kind::Perpendicular => (g(ph::ANGLE), tr("Perpendicular"), tr("Two lines at 90°.")),
                Kind::Collinear => (
                    g(ph::LINE_SEGMENT),
                    tr("Collinear"),
                    tr("Two lines on one straight line."),
                ),
                Kind::Tangent => (
                    g(ph::CIRCLE_HALF),
                    tr("Tangent"),
                    tr("A line and a circle or an arc, or two circles or arcs, that touch."),
                ),
                Kind::Equal => (
                    g(ph::EQUALS),
                    tr("Equal"),
                    tr("Two lines of the same length, or two circles or arcs of the same size."),
                ),
                Kind::Concentric => (
                    g(ph::TARGET),
                    tr("Concentric"),
                    tr("Two circles or arcs with the same center."),
                ),
                Kind::Midpoint => (
                    g(ph::TEXT_ALIGN_CENTER),
                    tr("Midpoint"),
                    tr("A point at the middle of a line."),
                ),
                Kind::OnCurve => (
                    g(ph::DOTS_THREE_CIRCLE),
                    tr("On Curve"),
                    tr("A point on a line, a circle or an arc."),
                ),
                Kind::Symmetric => (
                    g(ph::FLIP_HORIZONTAL),
                    tr("Symmetric"),
                    tr("Two points symmetric about a line."),
                ),
                Kind::Fix => (g(ph::PUSH_PIN), tr("Fix"), tr("A point that does not move.")),
            },
        },
    }
}

/// A view setting that a ribbon button turns on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Artwork,
    Stitches,
    Jumps,
    NeedlePoints,
    Functions,
    /// The density map over the stitches (P11 Milestone 1).
    Density,
    StitchCursor,
    Grid,
    Rulers,
    SnapGrid,
    SnapGuides,
    SnapObjects,
    ThreadLook,
    Player,
    Overview,
    AppliqueFabric,
    SplitView,
    DarkMode,
    Vectors,
    DimVectors,
    DrawVectors,
    /// The rhinestones on the canvas.
    Stones,
    /// Simple mode: fewer tools, larger buttons (P10 Milestone 5).
    SimpleMode,
    Hoop,
    HoopingMode,
    Product,
    Repeats,
}

/// A ribbon command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Tool(Tool),
    Toggle(Flag),
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Export,
    ImportSvg,
    ImportSvgLines,
    ImportSvgVectors,
    /// Make embroidery from the selected vectors.
    Convert(emb_model::ConvertTo),
    /// Make vectors from the selected embroidery objects.
    ToVector,
    /// Vector to Sketch (P14 Milestone 6, ADR 0079).
    VectorToSketch,
    /// Stitch objects become fills of the area that their stitches cover.
    RecognizeOutlines,
    /// A shaping command on the selection.
    Shape(emb_model::ShapeOp),
    /// The Offsets window.
    Offsets,
    /// The Quilting window: echo, background, quilt block.
    Quilting,
    /// The Cutwork window: lines for cutting needles.
    Cutwork,
    /// The Offsets window, set to make a border.
    AddBorder,
    RemoveOverlaps,
    /// The Arrangements window.
    Arrangements,
    /// A mirror copy to the right (first), below (second), or both.
    MirrorMerge(bool, bool),
    Reflect,
    /// The Border trace window.
    BorderTrace,
    /// The picture tools of the backdrop (Object Properties).
    PictureTools,
    /// Auto-trace the backdrop picture to vectors.
    TracePicture,
    /// The Picture to design wizard.
    PictureWizard,
    /// Click to stitch, making this.
    ClickAs(emb_art::click::ClickStitch),
    /// End the links of the selection (border traces, shape links).
    Unlink,
    ExportOutlines,
    LoadPicture,
    Cut,
    Copy,
    /// The lettering kiosk (P10 Milestone 5).
    Kiosk,
    /// Edit the backdrop picture in another program (P10 Milestone 6).
    EditPictureOutside,
    /// The My assets window (P10 Milestone 6).
    Assets,
    /// One more view of the design (P10 Milestone 6).
    NewView,
    /// Another open design next to the active one (P10 Milestone 6).
    OtherDesign,
    /// The design files in a zip file, for an email (P10 Milestone 6).
    SendByEmail,
    /// The Help window at its contents.
    HelpContents,
    /// The Help window at a topic (its id).
    HelpTopic(&'static str),
    /// The keyboard shortcuts (a help topic).
    KeyboardShortcuts,
    /// Show the start page.
    StartPage,
    /// Close the design tab.
    Close,
    /// Close all design tabs (each unsaved design asks first).
    CloseAll,
    /// Move the view so the stitch cursor is at its center.
    CenterAtCursor,
    /// Zoom in (true) or out by 2×.
    ZoomTwice(bool),
    Paste,
    /// Paste with the copies just after the selection in the sew order.
    PasteAfter,
    /// The Paste special window.
    PasteSpecial,
    Duplicate,
    /// Close open lines, vectors and stone runs: with a line, or (true) a curve.
    CloseShape(bool),
    /// Reverse the direction of lines, columns, vectors and stone runs.
    Reverse,
    /// Swap the sides of satin columns.
    SwapSides,
    /// Stone runs and fills to single stones.
    ToSingleStones,
    Delete,
    SelectAll,
    SelectNext,
    SelectPrev,
    SelectAtCursor,
    BreakApart,
    Join,
    FlorentineGuide,
    Borders,
    Buttonholes,
    Blends,
    SplitLines,
    /// The Tartan and Gradient window, at its tartan part.
    Tartan,
    /// The Tartan and Gradient window, at its gradient part.
    Gradient,
    /// A fill and lines: the lines become guides of a guided fill.
    GuideLines,
    /// Two lines: the second becomes the guide of a ripple of the first.
    RippleGuide,
    Motifs,
    Laydown,
    ClosestJoin,
    RemoveHoles,
    FillHoles,
    ContinueLine,
    Redwork,
    Branching,
    RotateCw,
    RotateCcw,
    Rotate180,
    MirrorH,
    MirrorV,
    CenterInHoop,
    Align(Align),
    /// Space evenly: across (true) or down (false).
    Space(bool),
    /// Make same size: width, height.
    SameSize(bool, bool),
    Group,
    Ungroup,
    Lock,
    UnlockAll,
    ToStart,
    BackOne,
    ForwardOne,
    ToEnd,
    BackColor,
    ForwardColor,
    ClickOrder,
    OptimizeColors,
    /// Find a command by its name (Ctrl+K, `palette.rs`).
    FindCommand,
    /// The sew order of each color with the least travel (`emb_stitch::order`).
    OptimizeOrder,
    /// Sew order for caps: from the center out, from the bottom up.
    CapOrder,
    /// Puff (3D foam) on the selected columns and lettering, or off.
    Puff,
    GroupByColor,
    ZoomExtents,
    Zoom100,
    ZoomIn,
    ZoomOut,
    ZoomHoop,
    ZoomSelection,
    ShowSelectedColors,
    HideOthers,
    /// Hide the selected objects on screen (Alt+V).
    HideSelected,
    ShowAll,
    RemoveGuides,
    Options,
    RotateDesign,
    /// The smallest hoop that the design fits.
    AutoHoop,
    /// Turn the hoop 15° counterclockwise or clockwise.
    TurnHoopLeft,
    TurnHoopRight,
    HoopOptions,
    MyHoops,
    /// Save the hoop template as PDF.
    HoopTemplate,
    /// The fewest hoopings that cover the design.
    AutoLayout,
    AddHooping,
    DeleteHooping,
    TurnHooping,
    DrawSplit,
    DeleteSplit,
    ClearHoopings,
    PreviewHoopings,
    OutputHoopings,
    /// The Product window.
    ProductWindow,
    /// Save the picture of the design on the product.
    SaveDecoration,
    /// The Repeats window.
    RepeatsWindow,
    /// The About window: version, build, folders.
    About,
    /// The Library tab (the design library).
    Library,
    /// The Clipart tab.
    ClipartPanel,
    /// Write the cut lines of the appliqué pieces (SVG or DXF).
    CutterFile,
    /// Keep the selected objects as a clipart item.
    SaveClipart,
    /// Send the design to the USB stick of the machine (one action).
    SendToUsb,
    /// Choose the USB stick and the folder, then send.
    SendToUsbChoose,
    /// Make the repeats as objects.
    MakeRepeats,
    ThreadsPanel,
    MatchChart,
    ImportChart,
    RemoveUnused,
    CycleColors,
    SplitThread,
    NewColorway,
    DeleteColorway,
    /// A machine function on the selection.
    Func(Func),
    /// Start Sketch (P14): open the selected sketch, or make a new one.
    StartSketch,
    /// A command of sketch mode.
    Sketch(SketchCmd),
    SplitAtNeedle,
    StitchListPanel,
    InsertMachineFile,
    OutputDesign,
    PrintPreview,
    BatchOutput,
    Fabric,
    SewTimes,
    Needles,
    Quote,
    OrderDetails,
    ThreadStock,
    Scripts,
    Fonts,
    LetteringOnLine,
    Monogram,
    TeamNames,
    Alphabets,
    KeyboardDesigns,
    QuickSimplify,
    ExportSvg,
    Simplify,
    CornerRadius,
    HoleFromOutline,
    PreviousView,
    ConvertToApplique,
    /// Change the selected shapes into cross-stitch objects.
    ConvertToCross,
    /// The chart of the cross stitch (PDF).
    CrossChart,
    /// The Picture to design wizard, set to make cross stitch.
    PictureToCross,
    /// Change the selection into rhinestones: a run along the outlines.
    ToStonesRun,
    /// Change the selection into rhinestones: a fill of the areas.
    ToStonesFill,
    /// Change the selection into rhinestones: a run along the center lines.
    ToStonesCenter,
    /// The rhinestone template (SVG, DXF or PDF).
    StoneTemplate,
    /// Sewn marks to put the rhinestone template in place.
    TemplateMarks,
    CombineApplique,
    MakeCurrent,
    ApplyCurrent,
    StylesPanel,
    NewFromTemplate,
    SaveAsTemplate,
}

/// The icon of a tool.
pub fn tool_icon(t: Tool) -> Icon {
    let g = Icon::Glyph;
    let d = Icon::Drawn;
    match t {
        Tool::Select => g(ph::CURSOR),
        Tool::Reshape => g(ph::BEZIER_CURVE),
        Tool::Lasso => g(ph::LASSO),
        Tool::PolySelect => g(ph::POLYGON),
        Tool::LineSelect => g(ph::LINE_SEGMENTS),
        Tool::Line => d(Drawing::Run),
        Tool::Manual => d(Drawing::Manual),
        Tool::Freehand => d(Drawing::Freehand),
        Tool::Shape => d(Drawing::Shape),
        Tool::ComplexFill => d(Drawing::ComplexFill),
        Tool::FreeShape => d(Drawing::FreeShape),
        Tool::Rectangle => d(Drawing::Rectangle),
        Tool::RoundRect => d(Drawing::RoundRect),
        Tool::Ellipse => d(Drawing::Ellipse),
        Tool::Star => d(Drawing::Star),
        // Column draws two sides (the icon with the sides); Column Pairs
        // draws pairs of points.
        Tool::Column => d(Drawing::ColumnB),
        Tool::ColumnB => d(Drawing::Column),
        Tool::ColumnC => d(Drawing::ColumnC),
        Tool::Ring => d(Drawing::Ring),
        Tool::Turning => d(Drawing::Turning),
        Tool::Applique => d(Drawing::Applique),
        Tool::Lettering => g(ph::TEXT_T),
        Tool::Knife => g(ph::KNIFE),
        Tool::AddHole => d(Drawing::AddHole),
        Tool::StitchAngle => g(ph::ANGLE),
        Tool::DirectionLine => g(ph::FLOW_ARROW),
        Tool::MirrorLine => g(ph::SQUARE_HALF),
        Tool::CropRect => g(ph::CROP),
        Tool::CropEllipse => g(ph::CIRCLE_DASHED),
        Tool::CropPolygon => g(ph::POLYGON),
        Tool::ClickStitch => g(ph::CURSOR_CLICK),
        Tool::RefPoints => g(ph::MAP_PIN_SIMPLE_LINE),
        Tool::Measure => g(ph::PENCIL_RULER),
        Tool::Zoom => g(ph::MAGNIFYING_GLASS),
        Tool::CrossStitch => g(ph::X_SQUARE),
        Tool::CrossErase => g(ph::ERASER),
        Tool::Backstitch => g(ph::LINE_SEGMENTS),
        Tool::Stone => g(ph::DIAMOND),
        Tool::StoneRun => g(ph::DOTS_THREE_OUTLINE),
        Tool::StoneFill => g(ph::DOTS_NINE),
    }
}

impl Cmd {
    /// Icon, name on the button, and help text.
    pub fn meta(self) -> (Icon, &'static str, &'static str) {
        let g = Icon::Glyph;
        match self {
            Cmd::Tool(t) => (tool_icon(t), t.label(), t.tooltip()),
            Cmd::Toggle(f) => match f {
                Flag::Artwork => (g(ph::PEN_NIB), tr("Artwork"), tr("Show the object outlines.")),
                Flag::Stitches => (
                    g(ph::WAVE_TRIANGLE),
                    tr("Stitches"),
                    tr("Show the stitches that export writes."),
                ),
                Flag::Jumps => (
                    g(ph::ARROW_BEND_UP_RIGHT),
                    tr("Jumps"),
                    tr("Show jumps and travel lines."),
                ),
                Flag::NeedlePoints => (
                    g(ph::DOTS_NINE),
                    tr("Needle Points"),
                    tr("Show a dot at each needle point."),
                ),
                Flag::Functions => (
                    g(ph::FLAG),
                    tr("Trims and Colors"),
                    tr("Show a symbol at each trim and color change."),
                ),
                Flag::Density => (
                    g(ph::THERMOMETER_HOT),
                    tr("Density Map"),
                    tr(
                        "Color the stitches by the layers of thread: red where objects on top of each other build up (more than the limit in Options). Design info lists these places.",
                    ),
                ),
                Flag::StitchCursor => (g(ph::NEEDLE), tr("Stitch Cursor"), tr(crate::viewing::KEYS_HELP)),
                Flag::Grid => (
                    g(ph::GRID_FOUR),
                    tr("Grid"),
                    tr("Show the grid. Set the spacing in Options."),
                ),
                Flag::DarkMode => (
                    g(ph::MOON),
                    tr("Dark Mode"),
                    tr("Light text on dark panels. Off: light mode. Also in Options."),
                ),
                Flag::Rulers => (
                    g(ph::RULER),
                    tr("Rulers"),
                    tr("Show the rulers. Drag from a ruler to make a guide."),
                ),
                Flag::SnapGrid => (
                    g(ph::MAGNET),
                    tr("Snap to Grid"),
                    tr("Points and moves snap to the grid."),
                ),
                Flag::SnapGuides => (
                    g(ph::MAGNET_STRAIGHT),
                    tr("Snap to Guides"),
                    tr("Points and moves snap to guides."),
                ),
                Flag::SnapObjects => (
                    g(ph::CROSSHAIR_SIMPLE),
                    tr("Snap to Objects"),
                    tr("Points snap to the nodes of other objects."),
                ),
                Flag::ThreadLook => (
                    g(ph::SPIRAL),
                    tr("Thread Look"),
                    tr(
                        "Show the stitches as threads, as they look on the fabric. The menu has the 3D look (light and shade, shadows, the fabric texture) and the light.",
                    ),
                ),
                Flag::Player => (
                    g(ph::PLAY_CIRCLE),
                    tr("Stitch Player"),
                    tr("Play the sew-out: the bar under the canvas has play, pause, speed and position."),
                ),
                Flag::Overview => (
                    g(ph::PICTURE_IN_PICTURE),
                    tr("Overview"),
                    tr("A small window with all of the design. Drag in it to move the view."),
                ),
                Flag::SplitView => (
                    g(ph::COLUMNS),
                    tr("Split View"),
                    tr("A second view of the design next to the first, with its own zoom and the thread look."),
                ),
                Flag::AppliqueFabric => (
                    Icon::Drawn(Drawing::Applique),
                    tr("Appliqué Fabric"),
                    tr("Show the fabric of the appliqué objects under the stitches."),
                ),
                Flag::SimpleMode => (
                    g(ph::SQUARES_FOUR),
                    tr("Simple Mode"),
                    tr(
                        "Fewer tools and larger buttons: the tools to draw a design, letter it and send it to the \
                     machine. Click again for all the tools.",
                    ),
                ),
                Flag::Stones => (
                    g(ph::DIAMONDS_FOUR),
                    tr("Rhinestones"),
                    tr("Show the rhinestones on the canvas. Hidden stones still go in the template and the worksheet."),
                ),
                Flag::Vectors => (
                    g(ph::VECTOR_TWO),
                    tr("Vectors"),
                    tr("Show the vector artwork (shapes with no stitches)."),
                ),
                Flag::DimVectors => (
                    g(ph::CIRCLE_HALF),
                    tr("Dim Vectors"),
                    tr("Show the vector artwork pale, so the stitches stand out."),
                ),
                Flag::DrawVectors => (
                    g(ph::PEN_NIB_STRAIGHT),
                    tr("Draw Artwork"),
                    tr("The draw tools make vector artwork (no stitches) in place of embroidery objects."),
                ),
                Flag::Hoop => (
                    g(ph::SQUARE),
                    tr("Show Hoop"),
                    tr("Show the hoop field and the safety margin on the canvas."),
                ),
                Flag::Product => (
                    g(ph::COAT_HANGER),
                    tr("Show Product"),
                    tr("Show the product under the design (Product window)."),
                ),
                Flag::Repeats => (
                    g(ph::COPY_SIMPLE),
                    tr("Show Repeats"),
                    tr("Show the repeats of the design (pale; they do not sew)."),
                ),
                Flag::HoopingMode => (
                    g(ph::FRAME_CORNERS),
                    tr("Hooping Mode"),
                    tr(
                        "Show and change the hoopings of a design that is larger than the hoop: drag a \
                     field to move its hooping, drag a split line or its nodes. Esc ends it.",
                    ),
                ),
            },
            Cmd::New => (g(ph::FILE_PLUS), tr("New"), tr("Start a new design.")),
            Cmd::Open => (
                g(ph::FOLDER_OPEN),
                tr("Open"),
                tr("Open a design, or a machine file as stitch objects."),
            ),
            Cmd::Save => (g(ph::FLOPPY_DISK), tr("Save"), tr("Save the design.")),
            Cmd::SaveAs => (
                g(ph::FLOPPY_DISK),
                tr("Save As"),
                tr("Save the design with a new name."),
            ),
            Cmd::Undo => (g(ph::ARROW_U_UP_LEFT), tr("Undo"), tr("Undo the last change.")),
            Cmd::Redo => (g(ph::ARROW_U_UP_RIGHT), tr("Redo"), tr("Redo the change.")),
            Cmd::Export => (
                g(ph::EXPORT),
                tr("Export"),
                tr("VP3 (Husqvarna Viking, Pfaff), DST (Tajima) or PES (Brother)."),
            ),
            Cmd::ImportSvg => (
                g(ph::FILE_SVG),
                tr("Import Artwork"),
                tr("Make objects from vector artwork: SVG, PDF, AI, EMF or WMF."),
            ),
            Cmd::ImportSvgLines => (
                g(ph::PATH),
                tr("Import as Lines"),
                tr("Make running-stitch lines from an SVG file: the outline of each shape and each stroke."),
            ),
            Cmd::ImportSvgVectors => (
                g(ph::FILE_ARROW_DOWN),
                tr("Import as Vectors"),
                tr("Add the paths of an SVG file as vector artwork with no stitches. Convert them later."),
            ),
            Cmd::Convert(to) => (
                match to {
                    emb_model::ConvertTo::Fill => g(ph::PAINT_ROLLER),
                    emb_model::ConvertTo::Satin => g(ph::WAVE_SQUARE),
                    emb_model::ConvertTo::Outline => g(ph::HEXAGON),
                },
                match to {
                    emb_model::ConvertTo::Fill => tr("To Fill"),
                    emb_model::ConvertTo::Satin => tr("To Satin"),
                    emb_model::ConvertTo::Outline => tr("To Outline"),
                },
                crate::vectors::convert_help(to),
            ),
            Cmd::Shape(op) => {
                use emb_model::ShapeOp as S;
                let (icon, name, help) = match op {
                    S::Weld => (ph::UNITE, tr("Weld"), tr("Make one shape of the selected shapes.")),
                    S::Intersect => (
                        ph::INTERSECT,
                        tr("Intersect"),
                        tr("Keep only the area that all selected shapes cover."),
                    ),
                    S::Exclude => (
                        ph::EXCLUDE,
                        tr("Exclude"),
                        tr("Keep the areas that the shapes do not share."),
                    ),
                    S::FrontMinusBack => (
                        ph::SUBTRACT,
                        tr("Front Minus Back"),
                        tr("Keep the front shape (the one that sews last), with the others cut out of it."),
                    ),
                    S::BackMinusFront => (
                        ph::SUBTRACT_SQUARE,
                        tr("Back Minus Front"),
                        tr("Keep the back shape (the one that sews first), with the others cut out of it."),
                    ),
                    S::Flatten => (
                        ph::STACK_SIMPLE,
                        tr("Flatten"),
                        tr(
                            "Cut out of each shape the parts that the shapes in front of it hide, so nothing sews twice.",
                        ),
                    ),
                    S::Divide => (
                        ph::SQUARE_SPLIT_HORIZONTAL,
                        tr("Divide"),
                        tr("Cut the shapes into pieces where they overlap; each overlap goes to the front shape."),
                    ),
                    S::Combine => (
                        ph::STACK,
                        tr("Combine"),
                        tr("Make one object of the selected vectors, or of the selected appliqué objects."),
                    ),
                };
                (g(icon), name, help)
            }
            Cmd::Offsets => (
                g(ph::SELECTION_BACKGROUND),
                tr("Offsets"),
                tr("Add outlines at a distance round the selected objects, or inside them."),
            ),
            Cmd::Cutwork => (
                g(ph::NEEDLE),
                tr("Cutwork"),
                tr(
                    "Make the selected lines cut lines for the four cutting needles of a cutwork kit (Bernina, Brother, Husqvarna Viking, Janome): each needle is a color.",
                ),
            ),
            Cmd::Quilting => (
                g(ph::GRID_NINE),
                tr("Quilting"),
                tr(
                    "Echo lines round the selected shapes, a stipple or meander background round them, or a quilt block that sews in hoopings.",
                ),
            ),
            Cmd::AddBorder => (
                g(ph::FRAME_CORNERS),
                tr("Add Border"),
                tr("Add a satin border round the selected objects."),
            ),
            Cmd::RemoveOverlaps => (
                g(ph::STACK_MINUS),
                tr("Remove Overlaps"),
                tr(
                    "Cut the selected shapes out of the objects under them, so no area sews twice. \
                 The overlap amount (Shaping) keeps a little under the cut.",
                ),
            ),
            Cmd::Arrangements => (
                g(ph::CARDS),
                tr("Arrangements"),
                tr("Copies of the selection: mirror-merge, array, wreath and kaleidoscope."),
            ),
            Cmd::MirrorMerge(h, v) => match (h, v) {
                (true, false) => (
                    g(ph::SQUARE_SPLIT_HORIZONTAL),
                    tr("Mirror Right"),
                    tr("Add a mirror copy to the right."),
                ),
                (false, true) => (
                    g(ph::SQUARE_SPLIT_VERTICAL),
                    tr("Mirror Below"),
                    tr("Add a mirror copy below."),
                ),
                _ => (
                    g(ph::GRID_FOUR),
                    tr("Mirror Both"),
                    tr("Add mirror copies to the right and below, and a turned copy in the corner."),
                ),
            },
            Cmd::BorderTrace => (
                g(ph::SELECTION_PLUS),
                tr("Border Trace"),
                tr(
                    "A border round the selected objects in a thread you choose, or a gap with no stitches, \
                 cut out of the objects under them.",
                ),
            ),
            Cmd::PictureTools => (
                g(ph::SLIDERS),
                tr("Picture Tools"),
                tr("Adjust the backdrop picture and prepare its colors (fewer colors, no specks) for tracing."),
            ),
            Cmd::PictureWizard => (
                g(ph::MAGIC_WAND),
                tr("Picture to Design"),
                tr(
                    "Turn a picture into embroidery step by step: choose it, remove the background, \
                 choose the detail and the colors, the stitch of each color, then see the stitches.",
                ),
            ),
            Cmd::RecognizeOutlines => (
                g(ph::SHAPES),
                tr("Recognize Outlines"),
                tr(
                    "Selected stitch objects (from a machine file or a photo stitch) become fills of the area \
                 that their stitches cover, so they can be scaled and keep their density.",
                ),
            ),
            Cmd::TracePicture => (
                g(ph::SCAN),
                tr("Trace Picture"),
                tr(
                    "Each color area of the backdrop picture becomes a vector shape in its color \
                 (not the background). Prepare colors first for a picture with many colors.",
                ),
            ),
            Cmd::ClickAs(c) => {
                use emb_art::click::ClickStitch as C;
                match c {
                    C::Fill => (
                        g(ph::PAINT_BUCKET),
                        tr("Click Fill"),
                        tr("Click a color area of the picture: a fill with its holes."),
                    ),
                    C::FillNoHoles => (
                        g(ph::DROP_HALF_BOTTOM),
                        tr("Fill, No Holes"),
                        tr("Click a color area of the picture: a fill with no holes (the holes sew over)."),
                    ),
                    C::Satin => (
                        g(ph::WAVE_SINE),
                        tr("Click Satin"),
                        tr("Click a narrow color area of the picture: satin columns along its middle."),
                    ),
                    C::CenterLine => (
                        g(ph::LINE_SEGMENTS),
                        tr("Center Line"),
                        tr("Click a thin color area of the picture: a running line along its middle."),
                    ),
                    C::Outline => (
                        g(ph::SCRIBBLE_LOOP),
                        tr("Click Outline"),
                        tr("Click a color area of the picture: a running line round it and its holes."),
                    ),
                }
            }
            Cmd::Unlink => (
                g(ph::LINK_BREAK),
                tr("Unlink"),
                tr(
                    "End the links of the selection (border traces, linked flatten, divide and remove overlaps). \
                 The objects stay as they are and are no longer cut again when they move.",
                ),
            ),
            Cmd::Reflect => (
                g(ph::SQUARE_HALF_BOTTOM),
                tr("Reflect"),
                tr("Click two points of a line: a mirror copy of the selection is added."),
            ),
            Cmd::VectorToSketch => (
                g(ph::COMPASS_TOOL),
                tr("Vector to Sketch"),
                tr(
                    "Change the selected vector objects into a sketch: lines, arcs, circles and splines, with the horizontal, vertical, tangent and equal constraints that the program finds.",
                ),
            ),
            Cmd::ToVector => (
                g(ph::VECTOR_THREE),
                tr("To Vector"),
                tr("Change the selected embroidery objects into vector artwork (their outlines, no stitches)."),
            ),
            Cmd::ExportOutlines => (
                g(ph::FILE_ARROW_UP),
                tr("Export Outlines"),
                tr("Write the shapes of the objects as SVG or EMF paths, for vector programs and cutters."),
            ),
            Cmd::LoadPicture => (
                g(ph::IMAGE),
                tr("Backdrop Picture"),
                tr("A PNG, JPEG, BMP, TIFF or GIF picture under the design, to trace."),
            ),
            Cmd::Cut => (g(ph::SCISSORS), tr("Cut"), tr("Move the selection to the clipboard.")),
            Cmd::Copy => (g(ph::COPY), tr("Copy"), tr("Copy the selection to the clipboard.")),
            Cmd::Paste => (
                g(ph::CLIPBOARD),
                tr("Paste"),
                tr("Paste the objects from the clipboard (where Options > Editing says)."),
            ),
            Cmd::PasteAfter => (
                g(ph::CLIPBOARD_TEXT),
                tr("Paste After Selected"),
                tr("Paste the objects just after the selected objects in the sew order, not at the end."),
            ),
            Cmd::PasteSpecial => (
                g(ph::SLIDERS_HORIZONTAL),
                tr("Paste Special…"),
                tr(
                    "Choose where the pasted objects go: the same place, moved, on the hoop center or at the stitch cursor.",
                ),
            ),
            Cmd::CloseShape(false) => (
                g(ph::POLYGON),
                tr("Close Shape"),
                tr("Close the open lines, vectors and stone runs with a straight line from the end to the start."),
            ),
            Cmd::CloseShape(true) => (
                g(ph::CIRCLE_DASHED),
                tr("Close with Curve"),
                tr("Close the open lines, vectors and stone runs with a curve that continues the ends."),
            ),
            Cmd::Reverse => (
                g(ph::ARROWS_LEFT_RIGHT),
                tr("Reverse Direction"),
                tr(
                    "The selected lines, columns, vectors and stone runs go the other way. The start and end points change places; motifs, borders and stone runs start at the other end.",
                ),
            ),
            Cmd::SwapSides => (
                g(ph::SWAP),
                tr("Swap Sides"),
                tr("The two sides of the selected satin columns change places, with the settings of each side."),
            ),
            Cmd::ToSingleStones => (
                g(ph::DOTS_SIX),
                tr("To Single Stones"),
                tr(
                    "The selected stone runs and fills become single stones at the same places. Reshape then moves each stone, and Delete in Reshape removes one.",
                ),
            ),
            Cmd::Duplicate => (
                g(ph::COPY_SIMPLE),
                tr("Duplicate"),
                tr("Make a copy of the selection, a little to the side."),
            ),
            Cmd::Delete => (g(ph::TRASH), tr("Delete"), tr("Delete the selection.")),
            Cmd::SelectAll => (g(ph::SELECTION_ALL), tr("Select All"), tr("Select all objects.")),
            Cmd::SelectNext => (
                g(ph::ARROW_LINE_RIGHT),
                tr("Next"),
                tr("Select the next object in sew order."),
            ),
            Cmd::SelectPrev => (
                g(ph::ARROW_LINE_LEFT),
                tr("Previous"),
                tr("Select the previous object in sew order."),
            ),
            Cmd::SelectAtCursor => (
                g(ph::CROSSHAIR),
                tr("At Stitch Cursor"),
                tr("Select the object that the stitch cursor is in."),
            ),
            Cmd::BreakApart => (
                g(ph::ARROWS_SPLIT),
                tr("Break Apart"),
                tr("One object for each separate part."),
            ),
            Cmd::Laydown => (
                g(ph::TOWEL),
                tr("Laydown"),
                tr(
                    "A light fill under the selected objects (or the whole design) that holds down the pile of \
                 towels and fur. It sews first, in a thread of its own.",
                ),
            ),
            Cmd::Motifs => (
                g(ph::FLOWER),
                tr("Motifs"),
                tr(
                    "The motif library: use a motif for the selected lines, columns or fills, or make a motif \
                 from the selected objects.",
                ),
            ),
            Cmd::SplitLines => (
                g(ph::LINE_SEGMENTS),
                tr("Split Lines"),
                tr(
                    "Select a fill and one or more lines: the fill rows are split where they cross the lines (user split). \
                 The lines are used up.",
                ),
            ),
            Cmd::Borders => (
                g(ph::FRAME_CORNERS),
                tr("Borders"),
                tr("Decorative borders (satin, runs, blanket, rope, motifs) round the selection or the whole design."),
            ),
            Cmd::Buttonholes => (
                g(ph::DOTS_SIX_VERTICAL),
                tr("Buttonholes"),
                tr("One buttonhole, or a string of buttonholes spaced along the selected line."),
            ),
            Cmd::Blends => (
                g(ph::GRADIENT),
                tr("Blends"),
                tr(
                    "Blend two or more colors in a fill or satin column: layers whose density changes from one side to the other.",
                ),
            ),
            Cmd::FlorentineGuide => (
                g(ph::WAVE_SINE),
                tr("Guide Line"),
                tr("Select a fill and a line: the fill rows bend like the line (Florentine). The line is used up."),
            ),
            Cmd::Tartan => (
                g(ph::CHECKERBOARD),
                tr("Tartan"),
                tr(
                    "Make a tartan from the selected fill: stripes of color in two directions, from a sett such as K4 R24 K24 Y4.",
                ),
            ),
            Cmd::Gradient => (
                g(ph::RAINBOW),
                tr("Gradient"),
                tr(
                    "Make a gradient from the selected fill: rows that change from one color to the next across the shape.",
                ),
            ),
            Cmd::GuideLines => (
                g(ph::WAVES),
                tr("Guide Lines"),
                tr(
                    "Select a fill and one or more lines: the fill rows follow the lines and change from one to the next (guided fill). \
                 The lines are used up.",
                ),
            ),
            Cmd::RippleGuide => (
                g(ph::PATH),
                tr("Ripple Guide"),
                tr(
                    "Select two lines: copies of the first line move along the second (ripple stitch). The second line is used up.",
                ),
            ),
            Cmd::Join => (
                g(ph::ARROWS_MERGE),
                tr("Join"),
                tr(
                    "Join the selected objects of one kind and color into one object: lines and columns whose \
                 ends touch, fills that touch. It puts back together what Break apart or the knife separated.",
                ),
            ),
            Cmd::ClosestJoin => (
                g(ph::ARROWS_MERGE),
                tr("Closest Join"),
                tr("Each object ends at the point nearest to the next one, which starts there."),
            ),
            Cmd::RemoveHoles => (
                Icon::Drawn(Drawing::RemoveHoles),
                tr("Remove Holes"),
                tr("Fill the holes of the fill."),
            ),
            Cmd::FillHoles => (
                Icon::Drawn(Drawing::FillHoles),
                tr("Fill Holes"),
                tr("A new fill object in each hole."),
            ),
            Cmd::ContinueLine => (
                g(ph::PENCIL_LINE),
                tr("Continue Line"),
                tr("Add points to the end of the line, or to its start: click near the start first."),
            ),
            Cmd::Redwork => (
                g(ph::ARROWS_COUNTER_CLOCKWISE),
                tr("Redwork"),
                tr("Join the lines into one object. Each line is sewn two times and it ends where it starts."),
            ),
            Cmd::Branching => (
                g(ph::TREE_STRUCTURE),
                tr("Branching"),
                tr("Join the lines into one object, sewn with the fewest lines sewn two times."),
            ),
            Cmd::RotateCw => (
                g(ph::ARROW_CLOCKWISE),
                tr("Rotate 90° Right"),
                tr("Turn the selection clockwise."),
            ),
            Cmd::RotateCcw => (
                g(ph::ARROW_COUNTER_CLOCKWISE),
                tr("Rotate 90° Left"),
                tr("Turn the selection counterclockwise."),
            ),
            Cmd::Rotate180 => (
                g(ph::ARROWS_CLOCKWISE),
                tr("Rotate 180°"),
                tr("Turn the selection half a turn."),
            ),
            Cmd::MirrorH => (
                g(ph::FLIP_HORIZONTAL),
                tr("Mirror Left-Right"),
                tr("Mirror the selection left to right."),
            ),
            Cmd::MirrorV => (
                g(ph::FLIP_VERTICAL),
                tr("Mirror Up-Down"),
                tr("Mirror the selection top to bottom."),
            ),
            Cmd::CenterInHoop => (
                g(ph::TARGET),
                tr("Center in Hoop"),
                tr("Move the selection to the hoop center."),
            ),
            Cmd::Align(a) => match a {
                Align::Left => (g(ph::ALIGN_LEFT), tr("Left"), tr("Align the left edges.")),
                Align::CenterX => (
                    g(ph::ALIGN_CENTER_HORIZONTAL),
                    tr("Center"),
                    tr("Align the centers, left to right."),
                ),
                Align::Right => (g(ph::ALIGN_RIGHT), tr("Right"), tr("Align the right edges.")),
                Align::Top => (g(ph::ALIGN_TOP), tr("Top"), tr("Align the top edges.")),
                Align::CenterY => (
                    g(ph::ALIGN_CENTER_VERTICAL),
                    tr("Middle"),
                    tr("Align the centers, top to bottom."),
                ),
                Align::Bottom => (g(ph::ALIGN_BOTTOM), tr("Bottom"), tr("Align the bottom edges.")),
            },
            Cmd::Space(true) => (
                g(ph::COLUMNS),
                tr("Across"),
                tr("Space the selection evenly from left to right."),
            ),
            Cmd::Space(false) => (
                g(ph::ROWS),
                tr("Down"),
                tr("Space the selection evenly from top to bottom."),
            ),
            Cmd::SameSize(true, false) => (
                g(ph::ARROWS_OUT_LINE_HORIZONTAL),
                tr("Same Width"),
                tr("Give the selection the width of the first selected object."),
            ),
            Cmd::SameSize(false, true) => (
                g(ph::ARROWS_OUT_LINE_VERTICAL),
                tr("Same Height"),
                tr("Give the selection the height of the first selected object."),
            ),
            Cmd::SameSize(..) => (
                g(ph::ARROWS_OUT),
                tr("Same Size"),
                tr("Give the selection the size of the first selected object."),
            ),
            Cmd::Group => (
                g(ph::BOUNDING_BOX),
                tr("Group"),
                tr("A click on one object selects all of the group."),
            ),
            Cmd::Ungroup => (
                g(ph::SELECTION_SLASH),
                tr("Ungroup"),
                tr("Remove the selection from its groups."),
            ),
            Cmd::Lock => (
                g(ph::LOCK),
                tr("Lock"),
                tr("Locked objects cannot be selected or changed."),
            ),
            Cmd::UnlockAll => (g(ph::LOCK_OPEN), tr("Unlock All"), tr("Unlock all locked objects.")),
            Cmd::ToStart => (g(ph::SKIP_BACK), tr("To Start"), tr("Sew the selection first.")),
            Cmd::BackOne => (
                g(ph::CARET_LEFT),
                tr("Back One"),
                tr("Sew the selection one object earlier."),
            ),
            Cmd::ForwardOne => (
                g(ph::CARET_RIGHT),
                tr("Forward One"),
                tr("Sew the selection one object later."),
            ),
            Cmd::ToEnd => (g(ph::SKIP_FORWARD), tr("To End"), tr("Sew the selection last.")),
            Cmd::BackColor => (
                g(ph::CARET_DOUBLE_LEFT),
                tr("Back One Color"),
                tr("Sew the selection one color earlier."),
            ),
            Cmd::ForwardColor => (
                g(ph::CARET_DOUBLE_RIGHT),
                tr("Forward One Color"),
                tr("Sew the selection one color later."),
            ),
            Cmd::ClickOrder => (
                g(ph::LIST_NUMBERS),
                tr("By Click Order"),
                tr("Sew the selected objects in the order you Ctrl+clicked them."),
            ),
            Cmd::OptimizeColors => (
                g(ph::MAGIC_WAND),
                tr("Fewer Color Changes"),
                tr("Merge color blocks where no overlapping object is in the way. Keeps what is on top."),
            ),
            Cmd::FindCommand => (
                g(ph::MAGNIFYING_GLASS),
                tr("Find a Command"),
                tr(
                    "Type a part of a command name and push Enter to run it. All ribbon commands, with the keyboard only.",
                ),
            ),
            Cmd::OptimizeOrder => (
                g(ph::PATH),
                tr("Shortest Order"),
                tr(
                    "Sew the objects of each color nearest first, so the needle does not go to and fro: less travel and fewer trims. Keeps the colors and what is on top.",
                ),
            ),
            Cmd::CapOrder => (
                g(ph::BASEBALL_CAP),
                tr("Cap Order"),
                tr(
                    "Sew the objects of each color from the center out and from the bottom up, and lettering from the middle letter out, so the cap fabric does not move. Keeps the colors and what is on top.",
                ),
            ),
            Cmd::Puff => (
                g(ph::CLOUD),
                tr("Puff"),
                tr(
                    "Puff (3D foam) on the selected satin columns and lettering, or off: dense satin, no underlay rows across, closed ends and a stop to put the foam on.",
                ),
            ),
            Cmd::GroupByColor => (
                g(ph::SWATCHES),
                tr("Group by Color"),
                tr("Fewest color changes. Can put an object below one that covered it."),
            ),
            Cmd::ZoomExtents => (
                g(ph::CORNERS_OUT),
                tr("Zoom Extents"),
                tr("Show the hoop and all of the design. Also: double-click the mouse wheel."),
            ),
            Cmd::Zoom100 => (
                g(ph::NUMBER_SQUARE_ONE),
                tr("Zoom 100 %"),
                tr("Show the design at its real size."),
            ),
            Cmd::ZoomIn => (
                g(ph::MAGNIFYING_GLASS_PLUS),
                tr("Zoom In"),
                tr("Zoom in by the zoom step of Options (1.5× at the start)."),
            ),
            Cmd::ZoomOut => (
                g(ph::MAGNIFYING_GLASS_MINUS),
                tr("Zoom Out"),
                tr("Zoom out by the zoom step of Options (1.5× at the start)."),
            ),
            Cmd::ZoomTwice(true) => (
                g(ph::MAGNIFYING_GLASS_PLUS),
                tr("Zoom In 2×"),
                tr("Zoom in to twice the size."),
            ),
            Cmd::ZoomTwice(false) => (
                g(ph::MAGNIFYING_GLASS_MINUS),
                tr("Zoom Out 2×"),
                tr("Zoom out to half the size."),
            ),
            Cmd::CenterAtCursor => (
                g(ph::CROSSHAIR),
                tr("Center at Stitch"),
                tr("Move the view so that the stitch cursor is at its center."),
            ),
            Cmd::HelpContents => (
                g(ph::QUESTION),
                tr("Help"),
                tr("The help topics, and a search that answers questions from them. F1 on a button opens its help."),
            ),
            Cmd::HelpTopic(id) => {
                let (icon, label, tip) = match id {
                    "tutorial-patch" => (
                        ph::GRADUATION_CAP,
                        tr("A Patch from a Picture"),
                        tr("Tutorial: make a two-color patch from a logo picture and write the machine file."),
                    ),
                    "tutorial-lettering" => (
                        ph::GRADUATION_CAP,
                        tr("A Name on an Arc"),
                        tr("Tutorial: lettering on an arc, the font, the size and the spacing."),
                    ),
                    "tutorial-output" => (
                        ph::GRADUATION_CAP,
                        tr("Production Output"),
                        tr("Tutorial: the machine file, the worksheet and the picture in one step."),
                    ),
                    "tutorial-applique" => (
                        ph::GRADUATION_CAP,
                        tr("An Appliqué"),
                        tr("Tutorial: an appliqué piece with its placement line, tack-down and cover satin."),
                    ),
                    "tutorial-towel" => (
                        ph::GRADUATION_CAP,
                        tr("A Monogram on a Towel"),
                        tr("Tutorial: a monogram on a towel, with the towel fabric and a laydown."),
                    ),
                    "tutorial-hooping" => (
                        ph::GRADUATION_CAP,
                        tr("Two Hoopings"),
                        tr("Tutorial: a design larger than the hoop, sewn in two hoopings."),
                    ),
                    "tutorial-lace" => (
                        ph::GRADUATION_CAP,
                        tr("A Lace Ornament"),
                        tr("Tutorial: a free-standing lace ornament on water-soluble stabilizer."),
                    ),
                    "troubleshooting" => (
                        ph::LIFEBUOY,
                        tr("Questions and Problems"),
                        tr("Answers to common problems: puckers, gaps, trims, small text, files."),
                    ),
                    _ => (ph::BOOK_OPEN, tr("Help topic"), tr("Open this help topic.")),
                };
                (g(icon), label, tip)
            }
            Cmd::KeyboardShortcuts => (
                g(ph::KEYBOARD),
                tr("Keyboard Shortcuts"),
                tr("All keyboard shortcuts of the program."),
            ),
            Cmd::StartPage => (
                g(ph::HOUSE),
                tr("Start Page"),
                tr("Recent designs, new designs, samples and tutorials."),
            ),
            Cmd::Close => (
                g(ph::X),
                tr("Close"),
                tr("Close this design. If it has changes, the program asks first."),
            ),
            Cmd::CloseAll => (
                g(ph::X_SQUARE),
                tr("Close All"),
                tr("Close all designs. For each design with changes, the program asks first."),
            ),
            Cmd::ZoomHoop => (g(ph::FRAME_CORNERS), tr("Zoom to Hoop"), tr("Show all of the hoop.")),
            Cmd::ZoomSelection => (
                g(ph::SELECTION),
                tr("Zoom to Selection"),
                tr("Show all of the selection."),
            ),
            Cmd::ShowSelectedColors => (
                g(ph::EYEDROPPER),
                tr("Only These Colors"),
                tr("Show only the colors of the selection."),
            ),
            Cmd::HideOthers => (
                g(ph::EYE_SLASH),
                tr("Hide Others"),
                tr("Show only the selected objects."),
            ),
            Cmd::HideSelected => (
                g(ph::EYE_CLOSED),
                tr("Hide Selected"),
                tr(
                    "Hide the selected objects on screen, to see what is under them. They still sew. \
                 With nothing selected, the key shows all objects again.",
                ),
            ),
            Cmd::ShowAll => (g(ph::EYE), tr("Show All"), tr("Show all objects and colors again.")),
            Cmd::RemoveGuides => (g(ph::ERASER), tr("Remove Guides"), tr("Remove all guides.")),
            Cmd::Options => (
                g(ph::GEAR),
                tr("Options"),
                tr("Units, grid, snap, nudge steps and hoop margin."),
            ),
            Cmd::RotateDesign => (
                g(ph::ARROW_ARC_RIGHT),
                tr("Rotate Design"),
                tr("Turn all of the design 90° clockwise in the hoop."),
            ),
            Cmd::AutoHoop => (
                g(ph::MAGIC_WAND),
                tr("Auto Hoop"),
                tr(
                    "Select the smallest hoop that the design fits, from My hoops (else the hoops of the machine). The design turns 90° if it fits a smaller hoop that way.",
                ),
            ),
            Cmd::TurnHoopLeft => (
                g(ph::ARROW_COUNTER_CLOCKWISE),
                tr("Turn Hoop Left"),
                tr(
                    "Turn the hoop 15° counterclockwise, to fit a design on a slant. The machine file turns the stitches back.",
                ),
            ),
            Cmd::TurnHoopRight => (
                g(ph::ARROW_CLOCKWISE),
                tr("Turn Hoop Right"),
                tr(
                    "Turn the hoop 15° clockwise, to fit a design on a slant. The machine file turns the stitches back.",
                ),
            ),
            Cmd::HoopOptions => (
                g(ph::SLIDERS),
                tr("Hoop Options"),
                tr("Hoop placement (manual or auto centering), the hoop angle, and auto start and end of the design."),
            ),
            Cmd::MyHoops => (
                g(ph::LIST_CHECKS),
                tr("My Hoops"),
                tr("Mark the hoops that you have, and add your own hoops."),
            ),
            Cmd::ProductWindow => (
                g(ph::T_SHIRT),
                tr("Product"),
                tr(
                    "See the design at its real size on a T-shirt, polo, sweatshirt, cap, bag, towel or \
                 cushion, or on a picture of your own product. The product does not sew.",
                ),
            ),
            Cmd::SaveDecoration => (
                g(ph::IMAGE_SQUARE),
                tr("Picture on Product"),
                tr("Save a picture (PNG) of the design with the thread look on the product."),
            ),
            Cmd::SendToUsb => (
                g(ph::USB),
                tr("Send to USB"),
                tr(
                    "Write the design in the format of the machine (VP3 for the Topaz 20) to the USB stick of the machine, and read it back to check it. The first time, choose the stick.",
                ),
            ),
            Cmd::SendToUsbChoose => (
                g(ph::HARD_DRIVES),
                tr("Send To…"),
                tr("Choose the USB stick and the folder on it, then send the design."),
            ),
            Cmd::CutterFile => (
                g(ph::SCISSORS),
                tr("Cutter File"),
                tr(
                    "Write the cut lines of the appliqué pieces at real size, as SVG or DXF, for a fabric \
                 cutting machine. The selected appliqué objects, or all of them.",
                ),
            ),
            Cmd::ClipartPanel => (
                g(ph::SHAPES),
                tr("Clipart"),
                tr(
                    "Show the Clipart tab: design pieces to use again. Double-click an item, or drag it \
                 onto the design.",
                ),
            ),
            Cmd::SaveClipart => (
                g(ph::FLOPPY_DISK_BACK),
                tr("Save as Clipart"),
                tr("Keep the selected objects as a clipart item, with a name and keywords."),
            ),
            Cmd::Library => (
                g(ph::BOOKS),
                tr("Design Library"),
                tr(
                    "The designs in your design folders: thumbnails, search, sort and group. Open a design, make a new design from one, print a catalog.",
                ),
            ),
            Cmd::About => (
                g(ph::INFO),
                tr("About"),
                tr("The version and build of the program, and the folder of the settings, backups and crash reports."),
            ),
            Cmd::RepeatsWindow => (
                g(ph::DOTS_THREE_OUTLINE),
                tr("Repeats"),
                tr(
                    "Show the design again in columns and rows, to check the join of a continuous \
                 border or of designs that overlap.",
                ),
            ),
            Cmd::MakeRepeats => (
                g(ph::COPY),
                tr("Make Repeats"),
                tr("Make the repeats as objects, after the design in the sew order."),
            ),
            Cmd::AutoLayout => (
                g(ph::MAGIC_WAND),
                tr("Auto Layout"),
                tr(
                    "Make the fewest hoopings of the hoop that cover the design, with split lines \
                 between them. A multi-position hoop gets a hooping for each of its positions.",
                ),
            ),
            Cmd::AddHooping => (
                g(ph::PLUS_SQUARE),
                tr("Add Hooping"),
                tr("Add a hooping at the middle of the canvas."),
            ),
            Cmd::DeleteHooping => (
                g(ph::MINUS_SQUARE),
                tr("Delete"),
                tr("Delete the selected hooping or split line (Delete)."),
            ),
            Cmd::TurnHooping => (
                g(ph::ARROW_CLOCKWISE),
                tr("Turn Hooping"),
                tr("Turn the selected hooping 90° (the hoop goes on the fabric the other way)."),
            ),
            Cmd::DrawSplit => (
                g(ph::LINE_SEGMENTS),
                tr("Split Line"),
                tr(
                    "Draw a split line: click its points, then Enter or a double-click. Objects that \
                 do not fit one hooping are cut there.",
                ),
            ),
            Cmd::DeleteSplit => (
                g(ph::SCISSORS),
                tr("Remove Split Lines"),
                tr("Remove all split lines. Each zone then goes to one hooping only."),
            ),
            Cmd::ClearHoopings => (
                g(ph::TRASH),
                tr("Remove Hoopings"),
                tr("Remove all hoopings of the design."),
            ),
            Cmd::PreviewHoopings => (
                g(ph::IMAGES),
                tr("Preview Hoopings"),
                tr("A picture of each hooping with its stitches, colors and warnings."),
            ),
            Cmd::OutputHoopings => (
                g(ph::EXPORT),
                tr("Output Hoopings"),
                tr(
                    "Write a machine file for each hooping (name_1, name_2, ...), with the files of \
                 Output design, and a hooping sheet (PDF) with the order and the places.",
                ),
            ),
            Cmd::HoopTemplate => (
                g(ph::FILE_PDF),
                tr("Hoop Template"),
                tr(
                    "Save the hoop field at full size as PDF, with the center lines and the design, to put the design in the correct place on the fabric.",
                ),
            ),
            Cmd::ThreadsPanel => (
                g(ph::PALETTE),
                tr("Threads"),
                tr("Show the Threads panel: thread names and colors, charts and colorways."),
            ),
            Cmd::MatchChart => (
                g(ph::EYEDROPPER_SAMPLE),
                tr("Match to Chart"),
                tr("Each used thread gets the nearest color of the chart in the Threads panel."),
            ),
            Cmd::ImportChart => (
                g(ph::FILE_ARROW_DOWN),
                tr("Import Chart"),
                tr("Import a thread chart: a GIMP palette (.gpl) or a CSV file (code,name,#rrggbb)."),
            ),
            Cmd::RemoveUnused => (
                g(ph::BROOM),
                tr("Remove Unused"),
                tr("Remove the threads that no object uses."),
            ),
            Cmd::CycleColors => (
                g(ph::ARROWS_CLOCKWISE),
                tr("Cycle Colors"),
                tr("Each used thread takes the color of the next one. Do it again to try more combinations."),
            ),
            Cmd::SplitThread => (
                g(ph::DROP_HALF),
                tr("Split Thread"),
                tr("Give the selection its own thread, so its color can change alone."),
            ),
            Cmd::NewColorway => (
                g(ph::SWATCHES),
                tr("New Colorway"),
                tr("A copy of the current colors, to change. Each colorway is one color version of the design."),
            ),
            Cmd::DeleteColorway => (g(ph::TRASH), tr("Delete Colorway"), tr("Delete the current colorway.")),
            Cmd::StartSketch => (
                g(ph::SELECTION_PLUS),
                tr("Start Sketch"),
                tr(
                    "Draw with exact sizes: lines, arcs and circles with constraints and dimensions, as in CAD programs. With a sketch selected, open it.",
                ),
            ),
            Cmd::Sketch(c) => sketch_meta(c),
            Cmd::Func(f) => match f {
                Func::Stop => (
                    g(ph::HAND_PALM),
                    tr("Stop After"),
                    tr("The machine stops after the selected objects, for example to put on appliqué fabric."),
                ),
                Func::TrimAlways => (
                    g(ph::LINK_BREAK),
                    tr("Always Trim"),
                    tr("Always trim after the selected objects."),
                ),
                Func::TrimNever => (
                    g(ph::LINK),
                    tr("Never Trim"),
                    tr("Do not trim after the selected objects. A long gap becomes a jump."),
                ),
                Func::TieIn => (
                    g(ph::ANCHOR_SIMPLE),
                    tr("Tie-In"),
                    tr("Lock stitches where the selected objects start after a trim."),
                ),
                Func::TieOff => (
                    g(ph::ANCHOR),
                    tr("Tie-Off"),
                    tr("Lock stitches where the selected objects end before a trim."),
                ),
            },
            Cmd::SplitAtNeedle => (
                g(ph::DIVIDE),
                tr("Split at Needle"),
                tr("Cut the object under the stitch cursor in two, where the cursor is."),
            ),
            Cmd::OutputDesign => (
                g(ph::PACKAGE),
                tr("Output Design"),
                tr("Make the machine files, the project file, the worksheet, the picture and more, in one step."),
            ),
            Cmd::PrintPreview => (
                g(ph::PRINTER),
                tr("Print Preview"),
                tr("The production worksheet and the approval sheet as they print. Save them as PDF."),
            ),
            Cmd::BatchOutput => (
                g(ph::FILES),
                tr("Batch Output"),
                tr("Output design for many project and machine files at once."),
            ),
            Cmd::InsertMachineFile => (
                g(ph::STACK_PLUS),
                tr("Insert Stitches"),
                tr(
                    "Add the stitches of a machine file (DST, VP3, PES, EXP, JEF, XXX, TBF, TAP and others) to the design.",
                ),
            ),
            Cmd::ConvertToApplique => (
                g(ph::ARROW_SQUARE_IN),
                tr("To Appliqué"),
                tr("Change the selected fills into appliqué objects, with the same outline and holes."),
            ),
            Cmd::ConvertToCross => (
                g(ph::GRID_FOUR),
                tr("To Cross Stitch"),
                tr(
                    "Change the selected fills, appliqué, satin columns and closed vectors into cross stitch: \
                 a full cross in each cell of the grid whose center is inside the shape.",
                ),
            ),
            Cmd::ToStonesRun => (
                g(ph::CIRCLES_THREE),
                tr("Outline to Stones"),
                tr(
                    "Change the selected shapes, lines and lettering into a run of rhinestones along \
                 their outlines (rhinestone lettering).",
                ),
            ),
            Cmd::TemplateMarks => (
                g(ph::CROSSHAIR_SIMPLE),
                tr("Template Marks"),
                tr(
                    "Add four crosses that sew last, in their own color, outside the stones: put the \
                 rhinestone template (and the transfer tape) on them. Pull them out after the press. \
                 Again: the crosses go back to the corners of the stones.",
                ),
            ),
            Cmd::StoneTemplate => (
                g(ph::SCISSORS),
                tr("Template"),
                tr(
                    "The rhinestone template: a hole for each stone, a little larger than the stone, and \
                 registration marks, at real size. SVG or DXF for a cutter, or PDF on paper.",
                ),
            ),
            Cmd::ToStonesCenter => (
                g(ph::DOTS_THREE),
                tr("Center Line to Stones"),
                tr(
                    "Change the selected narrow shapes and lettering into one line of rhinestones along \
                 the middle of each stroke.",
                ),
            ),
            Cmd::ToStonesFill => (
                g(ph::CIRCLES_FOUR),
                tr("Fill with Stones"),
                tr("Change the selected shapes and lettering into rhinestones that fill their areas."),
            ),
            Cmd::PictureToCross => (
                g(ph::MAGIC_WAND),
                tr("Picture to Cross Stitch"),
                tr(
                    "Turn a picture into cross stitch: choose it, remove the background, then the cell size and the number of thread colors.",
                ),
            ),
            Cmd::CrossChart => (
                g(ph::FILE_PDF),
                tr("Chart"),
                tr(
                    "The chart of the cross stitch as PDF: a symbol for each color in each cell, the key \
                 with the threads and the number of stitches, and the size on Aida fabric.",
                ),
            ),
            Cmd::CombineApplique => (
                g(ph::STACK),
                tr("Combine Appliqué"),
                tr(
                    "Join the selected appliqué objects: all placement lines first, then all tack-downs, \
                 then all covers, with one stop for each step.",
                ),
            ),
            Cmd::CornerRadius => (
                g(ph::ARROW_ELBOW_RIGHT_DOWN),
                tr("Corner Radius"),
                tr("Set the radius of each corner of the selected fill or appliqué."),
            ),
            Cmd::HoleFromOutline => (
                g(ph::FRAME_CORNERS),
                tr("Hole from Outline"),
                tr("Add a hole with the shape of the outline, at a distance inside it (a frame)."),
            ),
            Cmd::PreviousView => (
                g(ph::ARROW_U_UP_LEFT),
                tr("Previous View"),
                tr("Go back to the view before the last zoom."),
            ),
            Cmd::Fonts => (
                g(ph::TEXT_AA),
                tr("Fonts"),
                tr("Find fonts, see a preview and the size range, and convert a TrueType font to an embroidery font."),
            ),
            Cmd::ExportSvg => (
                g(ph::FILE_SVG),
                tr("Export SVG"),
                tr("Save the design picture as SVG: the stitches as lines in the thread colors, at real size."),
            ),
            Cmd::QuickSimplify => (
                g(ph::BEZIER_CURVE),
                tr("Quick Simplify"),
                tr("Fewer nodes for the selected objects, with the standard settings. The shape stays within 0.2 mm."),
            ),
            Cmd::Simplify => (
                g(ph::FADERS_HORIZONTAL),
                tr("Simplify…"),
                tr(
                    "Fewer nodes, with your settings: tolerance, corners, curves or straight lines. See the result before you keep it.",
                ),
            ),
            Cmd::Alphabets => (
                g(ph::TEXT_A_UNDERLINE),
                tr("Alphabets"),
                tr(
                    "Make your own alphabets: letters from objects you draw, between guides. Rename, delete, import (also bitmap fonts: .bdf, .fon) and export alphabets.",
                ),
            ),
            Cmd::NewView => (
                g(ph::SQUARE_SPLIT_HORIZONTAL),
                tr("New View"),
                tr("One more view of the design, with its own zoom and thread look (up to three more)."),
            ),
            Cmd::OtherDesign => (
                g(ph::BROWSERS),
                tr("Other Design"),
                tr(
                    "Show another open design next to this one, with its own zoom. Edit this design makes it the \
                 active one.",
                ),
            ),
            Cmd::Assets => (
                g(ph::ARCHIVE),
                tr("My Assets"),
                tr(
                    "Your own templates, fonts, motifs, thread charts, clipart and hoops: import, export and share them as one package.",
                ),
            ),
            Cmd::EditPictureOutside => (
                g(ph::PAINT_BRUSH),
                tr("Edit in Another Program"),
                tr(
                    "Open the backdrop picture in the picture program of the computer. Save it there: the backdrop changes here (same place and width).",
                ),
            ),
            Cmd::SendByEmail => (
                g(ph::ENVELOPE),
                tr("Send by Email"),
                tr(
                    "Put the machine file, the project and a picture in a zip file next to the design, and open the mail program. Attach the zip file to the message.",
                ),
            ),
            Cmd::Kiosk => (
                g(ph::STOREFRONT),
                tr("Lettering Kiosk"),
                tr(
                    "A full-window screen for a shop: a product, a name, a font and a color, then save, output or \
                 send to USB.",
                ),
            ),
            Cmd::TeamNames => (
                g(ph::USERS_THREE),
                tr("Team Names"),
                tr(
                    "One design, many names: a list of names (and numbers), saved as one file for each name or all in one design.",
                ),
            ),
            Cmd::KeyboardDesigns => (
                g(ph::KEYBOARD),
                tr("Keyboard Designs"),
                tr(
                    "Type with a collection of designs, one for each key; map selected objects to a key of your own collection.",
                ),
            ),
            Cmd::Monogram => (
                g(ph::SEAL),
                tr("Monogram"),
                tr(
                    "One to three initials in a style, with a border and ornaments, from a template or your own settings. With a monogram selected: change it.",
                ),
            ),
            Cmd::LetteringOnLine => (
                g(ph::PATH),
                tr("Lettering on Line"),
                tr(
                    "Put lettering on a line: select the lettering, use this, then click the line \
                 (or select both first). The line becomes the baseline.",
                ),
            ),
            Cmd::SewTimes => (
                g(ph::TIMER),
                tr("Machine Settings"),
                tr(
                    "The speed of the machine and the time of color changes, trims and stops (for the sew time), \
                 and how DST and TAP files tell the machine to trim.",
                ),
            ),
            Cmd::Needles => (
                g(ph::NEEDLE),
                tr("Needles"),
                tr(
                    "The needles of a multi-needle machine: the number of needles, the needle of each color, and the threads to put on before the start.",
                ),
            ),
            Cmd::Quote => (
                g(ph::RECEIPT),
                tr("Quote"),
                tr(
                    "The price of an order from your price lists, the run time on your machine, and a quote sheet for the customer.",
                ),
            ),
            Cmd::OrderDetails => (
                g(ph::NOTEPAD),
                tr("Order Details"),
                tr(
                    "The customer, the order number, the garment, the sizes, the placement and the due date of this design, for the worksheet and the approval sheet.",
                ),
            ),
            Cmd::ThreadStock => (
                g(ph::WAREHOUSE),
                tr("Thread Stock"),
                tr(
                    "The threads that you have: color matching with only them, the nearest thread in stock for each thread of the design, and the threads to buy for the order.",
                ),
            ),
            Cmd::Scripts => (
                g(ph::CODE),
                tr("Scripts"),
                tr(
                    "Run a script: a small tool that changes the design, for example an outline round each fill. Write your own in the scripts folder.",
                ),
            ),
            Cmd::Fabric => (
                g(ph::SHIRT_FOLDED),
                tr("Fabric"),
                tr(
                    "Set the fabric. It changes the density, pull compensation and underlay of satin and fill stitches.",
                ),
            ),
            Cmd::MakeCurrent => (
                g(ph::ARROW_FAT_LINE_DOWN),
                tr("Make Current"),
                tr("New objects get the stitch settings of the selected objects."),
            ),
            Cmd::ApplyCurrent => (
                g(ph::PAINT_BRUSH),
                tr("Apply Current"),
                tr("Give the selected objects the current properties (the settings of new objects)."),
            ),
            Cmd::StylesPanel => (
                g(ph::STAMP),
                tr("Styles"),
                tr("Show the Styles panel: the current properties, and named sets of stitch settings to apply."),
            ),
            Cmd::NewFromTemplate => (
                g(ph::FILE_DASHED),
                tr("New from Template"),
                tr("Start a new design from a template: machine, hoop, fabric, settings, styles and threads."),
            ),
            Cmd::SaveAsTemplate => (
                g(ph::LAYOUT),
                tr("Save as Template"),
                tr("Save the settings of this design as a template for new designs."),
            ),
            Cmd::StitchListPanel => (
                g(ph::LIST_DASHES),
                tr("Stitch List"),
                tr("Show the Stitch list panel: every stitch, jump, trim, stop and color change."),
            ),
        }
    }

    /// The keyboard shortcut, as text, if the command has one.
    pub(crate) fn shortcut_text(self, ctx: &egui::Context) -> Option<String> {
        self.keys(ctx)
    }

    /// The keyboard shortcut, as text, if the command has one.
    fn keys(self, ctx: &egui::Context) -> Option<String> {
        use crate::app as a;
        use crate::commands as c;
        let ctrl = |k| ctx.format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, k));
        let s = match self {
            Cmd::New => a::NEW,
            Cmd::FindCommand => crate::palette::FIND_COMMAND,
            Cmd::Open => a::OPEN,
            Cmd::Save => a::SAVE,
            Cmd::SaveAs => a::SAVE_AS,
            Cmd::Undo => a::UNDO,
            Cmd::Redo => a::REDO,
            Cmd::Export => a::EXPORT,
            Cmd::ImportSvg => a::IMPORT,
            Cmd::SelectAll => a::SELECT_ALL,
            Cmd::Duplicate => c::DUPLICATE,
            Cmd::Join => crate::join::JOIN,
            Cmd::RotateCw => c::ROTATE_CW,
            Cmd::RotateCcw => c::ROTATE_CCW,
            Cmd::MirrorH => c::MIRROR_H,
            Cmd::MirrorV => c::MIRROR_V,
            Cmd::HideSelected => c::HIDE,
            Cmd::ToStart => a::SEQ_START,
            Cmd::BackOne => a::SEQ_BACK,
            Cmd::ForwardOne => a::SEQ_FORWARD,
            Cmd::ToEnd => a::SEQ_END,
            Cmd::ZoomExtents => a::ZOOM_EXTENTS,
            Cmd::Cut => return Some(ctrl(egui::Key::X)),
            Cmd::Copy => return Some(ctrl(egui::Key::C)),
            Cmd::Paste => return Some(ctrl(egui::Key::V)),
            Cmd::Delete => return Some("Del".into()),
            Cmd::SelectNext => return Some("Tab".into()),
            Cmd::SelectPrev => return Some("Shift+Tab".into()),
            Cmd::ZoomIn => return Some("+".into()),
            Cmd::ZoomOut => return Some("-".into()),
            _ => return None,
        };
        Some(ctx.format_shortcut(&s))
    }

    /// Commands that open a file dialog or a new design (tests do not run them).
    #[cfg(test)]
    fn opens_dialog(self) -> bool {
        matches!(
            self,
            Cmd::New
                | Cmd::Open
                | Cmd::Save
                | Cmd::SaveAs
                | Cmd::Export
                | Cmd::ImportSvg
                | Cmd::ImportSvgLines
                | Cmd::ImportSvgVectors
                | Cmd::ExportOutlines
                | Cmd::CutterFile
                | Cmd::CrossChart
                | Cmd::StoneTemplate
                | Cmd::PictureToCross
                | Cmd::LoadPicture
                | Cmd::PictureWizard
                | Cmd::ImportChart
                | Cmd::InsertMachineFile
                | Cmd::BatchOutput
                | Cmd::ExportSvg
                | Cmd::HoopTemplate
                | Cmd::OutputHoopings
                | Cmd::SaveDecoration
                | Cmd::Kiosk
                | Cmd::EditPictureOutside
                | Cmd::SendByEmail
        )
    }
}

/// What the selection holds. It sets which commands are available.
pub struct SelInfo {
    pub n: usize,
    pub fills: usize,
    pub holes: bool,
    pub lines: usize,
    pub single_line: bool,
    /// One object that is not a line (a fill or a satin).
    pub single_shape: bool,
    pub appliques: usize,
    pub letterings: usize,
    /// Satin columns and lines with a border: objects with a column type.
    pub columns: usize,
    /// Vector objects (artwork).
    pub vectors: usize,
    /// Objects that take part in shaping (objects with an area).
    pub shapes: usize,
    /// Stitch objects (from a machine file or a photo stitch).
    pub stitches: usize,
}

impl SelInfo {
    pub fn of(st: &AppState) -> Self {
        let d = st.doc.design();
        let objs: Vec<&emb_model::DesignObject> = st.selection.ids().iter().filter_map(|id| d.object(*id)).collect();
        let count = |f: fn(&ObjectKind) -> bool| objs.iter().filter(|o| f(&o.kind)).count();
        let fills = count(|k| matches!(k, ObjectKind::Fill { .. }));
        let lines = count(|k| matches!(k, ObjectKind::Running { .. }));
        let vectors = count(|k| matches!(k, ObjectKind::Vector { .. }));
        SelInfo {
            vectors,
            shapes: count(emb_model::shaping::takes_part),
            n: objs.len(),
            fills,
            holes: count(|k| matches!(k, ObjectKind::Fill { holes, .. } if !holes.is_empty())) > 0,
            lines,
            single_line: objs.len() == 1 && lines == 1,
            single_shape: objs.len() == 1 && lines == 0 && vectors == 0,
            appliques: count(|k| matches!(k, ObjectKind::Applique { .. })),
            stitches: count(|k| matches!(k, ObjectKind::Stitches { .. })),
            letterings: count(|k| matches!(k, ObjectKind::Lettering(_))),
            columns: count(|k| match k {
                ObjectKind::Satin { .. } => true,
                ObjectKind::Running { params, .. } => params.border.is_some(),
                _ => false,
            }),
        }
    }
}

impl AppState {
    /// Returns `true` if the command can run now.
    pub fn can(&self, cmd: Cmd, s: &SelInfo) -> bool {
        let d = self.doc.design();
        match cmd {
            Cmd::Tool(Tool::Knife | Tool::MirrorLine | Tool::RefPoints) => s.n > 0,
            Cmd::Tool(Tool::CropRect | Tool::CropEllipse | Tool::CropPolygon | Tool::ClickStitch) => {
                d.backdrop.as_ref().is_some_and(|b| b.visible)
            }
            Cmd::Tool(Tool::AddHole) => s.n == 1 && s.fills == 1,
            Cmd::Tool(Tool::StitchAngle | Tool::DirectionLine) => s.single_shape,
            Cmd::StartSketch => self.sketch.is_none(),
            Cmd::Sketch(_) => self.sketch.is_some(),
            Cmd::Undo => self.doc.can_undo(),
            Cmd::Redo => self.doc.can_redo(),
            Cmd::Paste | Cmd::PasteSpecial => self.clipboard.is_some(),
            Cmd::PasteAfter => self.clipboard.is_some() && s.n > 0,
            Cmd::CloseShape(_) => self.selected_count(crate::editing::has_open_part) > 0,
            Cmd::Reverse => self.selected_count(crate::editing::reversible) > 0,
            Cmd::SwapSides => self.selected_count(|k| matches!(k, ObjectKind::Satin { .. })) > 0,
            Cmd::ToSingleStones => self.selected_count(crate::editing::stone_pattern) > 0,
            Cmd::SelectAtCursor | Cmd::CenterAtCursor => self.cursor.is_some(),
            Cmd::RemoveHoles | Cmd::FillHoles => s.holes,
            Cmd::ContinueLine => s.single_line,
            Cmd::Convert(_) => s.vectors > 0,
            Cmd::Shape(emb_model::ShapeOp::Combine) => s.vectors >= 2 || s.appliques >= 2,
            Cmd::Shape(_) => s.shapes >= 2,
            Cmd::Offsets | Cmd::AddBorder | Cmd::Arrangements | Cmd::Quilting | Cmd::Cutwork => true,
            Cmd::RemoveOverlaps => s.shapes > 0,
            Cmd::MirrorMerge(..) | Cmd::Reflect => s.n > 0,
            Cmd::BorderTrace => true,
            Cmd::PictureTools | Cmd::EditPictureOutside => d.backdrop.is_some(),
            Cmd::SendByEmail => !d.objects().is_empty(),
            Cmd::NewView => self.views.iter().filter(|v| v.is_some()).count() < crate::views::MAX_VIEWS,
            Cmd::OtherDesign => self.tab_count() > 1,
            Cmd::PictureWizard => true,
            Cmd::TracePicture | Cmd::ClickAs(_) => d.backdrop.as_ref().is_some_and(|b| b.visible),
            Cmd::Unlink => {
                let d = self.doc.design();
                let made = |id: &emb_model::ObjectId| d.sketch_links.iter().any(|l| l.sketch == *id);
                self.selection.ids().iter().any(|id| d.is_linked(*id) || made(id))
            }
            Cmd::ToVector => s.n > s.vectors,
            Cmd::VectorToSketch => s.vectors > 0,
            Cmd::RecognizeOutlines => s.stitches > 0,
            Cmd::ExportOutlines => !d.objects().is_empty(),
            Cmd::FlorentineGuide => s.n == 2 && s.fills == 1 && s.lines == 1,
            Cmd::Laydown => !d.objects().is_empty(),
            Cmd::Motifs | Cmd::Borders | Cmd::Buttonholes | Cmd::Blends => true,
            Cmd::SplitLines | Cmd::GuideLines => s.fills == 1 && s.lines > 0 && s.n == 1 + s.lines,
            Cmd::Tartan | Cmd::Gradient => true,
            Cmd::RippleGuide => s.n == 2 && s.lines == 2,
            Cmd::Redwork | Cmd::Branching => s.lines > 0,
            Cmd::LetteringOnLine => {
                (s.n == 2 && s.lines == 1 && s.letterings == 1) || (s.n == 1 && (s.lines == 1 || s.letterings == 1))
            }
            Cmd::QuickSimplify | Cmd::Simplify => s.n > s.letterings,
            Cmd::Align(_) | Cmd::SameSize(..) | Cmd::Group | Cmd::ClosestJoin | Cmd::ClickOrder | Cmd::Join => s.n > 1,
            Cmd::OptimizeOrder | Cmd::CapOrder => d.objects().len() > 1,
            Cmd::Puff => self.selection.ids().iter().any(|id| {
                matches!(
                    d.object(*id).map(|o| &o.kind),
                    Some(ObjectKind::Satin { .. } | ObjectKind::Lettering(_))
                ) || matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Running { params, .. }) if params.border.is_some())
            }),
            Cmd::Space(_) => s.n > 2,
            Cmd::UnlockAll => d.objects().iter().any(|o| o.locked),
            Cmd::ShowAll => self.view_filter.is_active(),
            Cmd::RemoveGuides => !d.guides.is_empty(),
            Cmd::Cut
            | Cmd::Copy
            | Cmd::Duplicate
            | Cmd::Delete
            | Cmd::BreakApart
            | Cmd::RotateCw
            | Cmd::RotateCcw
            | Cmd::Rotate180
            | Cmd::MirrorH
            | Cmd::MirrorV
            | Cmd::CenterInHoop
            | Cmd::Ungroup
            | Cmd::Lock
            | Cmd::ToStart
            | Cmd::BackOne
            | Cmd::ForwardOne
            | Cmd::ToEnd
            | Cmd::BackColor
            | Cmd::ForwardColor
            | Cmd::ZoomSelection
            | Cmd::ShowSelectedColors
            | Cmd::HideOthers
            | Cmd::HideSelected
            | Cmd::SplitThread => s.n > 0,
            Cmd::DeleteColorway => d.colorways.len() > 1,
            Cmd::Func(_) => s.n > 0,
            Cmd::SplitAtNeedle => self.cursor.is_some(),
            Cmd::CycleColors => d.used_threads().len() > 1,
            Cmd::MatchChart => !d.objects().is_empty(),
            Cmd::OutputDesign | Cmd::PrintPreview | Cmd::SendToUsb | Cmd::SendToUsbChoose => !d.objects().is_empty(),
            Cmd::MakeCurrent | Cmd::ApplyCurrent => s.n > 0,
            Cmd::CornerRadius | Cmd::HoleFromOutline => s.n == 1 && (s.fills == 1 || s.appliques == 1),
            Cmd::PreviousView => !self.view_history.is_empty(),
            Cmd::ConvertToApplique => s.fills + s.vectors > 0,
            Cmd::ConvertToCross => s.shapes > 0,
            Cmd::PictureToCross => true,
            Cmd::ToStonesRun => s.n > 0,
            Cmd::ToStonesFill | Cmd::ToStonesCenter => s.shapes + s.letterings > 0,
            Cmd::StoneTemplate | Cmd::TemplateMarks => {
                d.objects().iter().any(|o| matches!(o.kind, ObjectKind::Bling(_)))
            }
            Cmd::CrossChart => d.objects().iter().any(|o| matches!(o.kind, ObjectKind::CrossStitch(_))),
            Cmd::AutoHoop => !d.objects().is_empty(),
            Cmd::SaveDecoration => d.product.is_some(),
            Cmd::MakeRepeats => d.repeat.is_some() && !d.objects().is_empty(),
            Cmd::AutoLayout => !d.objects().is_empty() && self.layout_hoop().is_some(),
            Cmd::AddHooping => d.hooping.is_some() && self.layout_hoop().is_some_and(|h| h.positions.is_empty()),
            Cmd::DrawSplit | Cmd::ClearHoopings => d.hooping.is_some(),
            Cmd::DeleteHooping => self.hoopings_ui.selected.is_some(),
            Cmd::TurnHooping => matches!(self.hoopings_ui.selected, Some(crate::hooping::HoopingPick::Hooping(_))),
            Cmd::DeleteSplit => d.hooping.as_ref().is_some_and(|l| !l.splits.is_empty()),
            Cmd::PreviewHoopings | Cmd::OutputHoopings => d.hooping.as_ref().is_some_and(|l| !l.hoopings.is_empty()),
            Cmd::TurnHoopLeft | Cmd::TurnHoopRight | Cmd::HoopTemplate => {
                matches!(d.hoop.selection, HoopSelection::Preset { .. })
            }
            Cmd::CombineApplique => s.appliques > 1,
            Cmd::SaveClipart => s.n > 0,
            Cmd::CutterFile => d
                .objects()
                .iter()
                .any(|o| matches!(o.kind, emb_model::ObjectKind::Applique { .. })),
            _ => true,
        }
    }

    /// Returns `true` if the button of the command shows as on.
    pub fn is_on(&self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Tool(t) => self.tool == t,
            Cmd::ClickAs(c) => self.tool == Tool::ClickStitch && self.picture.click == c,
            Cmd::Toggle(f) => self.flag(f),
            Cmd::Func(f) => self.func_on(f),
            Cmd::Sketch(SketchCmd::Tool(t)) => self.sketch.as_ref().is_some_and(|e| e.tool == t),
            Cmd::Sketch(SketchCmd::Construction) => self.sketch.as_ref().is_some_and(|e| e.construction),
            Cmd::Sketch(SketchCmd::RelaxMode) => self.sketch.as_ref().is_some_and(|e| e.relax_mode),
            Cmd::Sketch(SketchCmd::Parameters) => self.sketch.as_ref().is_some_and(|e| e.params.open),
            Cmd::DrawSplit => self.hoopings_ui.drawing.is_some(),
            Cmd::PreviewHoopings => self.hoopings_ui.preview,
            _ => false,
        }
    }

    fn flag(&self, f: Flag) -> bool {
        let s = &self.settings;
        match f {
            Flag::Artwork => self.show_artwork,
            Flag::Stitches => self.show_plan,
            Flag::Jumps => self.show_jumps,
            Flag::NeedlePoints => self.show_needle_points,
            Flag::Functions => self.show_functions,
            Flag::Density => self.show_density,
            Flag::StitchCursor => self.cursor.is_some(),
            Flag::Grid => s.show_grid,
            Flag::Rulers => s.show_rulers,
            Flag::SnapGrid => s.snap_grid,
            Flag::SnapGuides => s.snap_guides,
            Flag::SnapObjects => s.snap_objects,
            Flag::ThreadLook => self.show_thread_look,
            Flag::Player => self.player.shown,
            Flag::Overview => self.show_overview,
            Flag::AppliqueFabric => self.show_applique_fabric,
            Flag::Stones => self.show_stones,
            Flag::SimpleMode => s.simple_mode,
            Flag::SplitView => self.show_split,
            Flag::DarkMode => s.dark_mode,
            Flag::Vectors => self.show_vectors,
            Flag::DimVectors => self.dim_vectors,
            Flag::DrawVectors => self.draw_vectors,
            Flag::Hoop => self.show_hoop,
            Flag::HoopingMode => self.hoopings_ui.mode,
            Flag::Product => self.doc.design().product.as_ref().is_some_and(|v| v.visible),
            Flag::Repeats => self.doc.design().repeat.is_some_and(|r| r.show),
        }
    }

    fn toggle(&mut self, f: Flag) {
        let mut s = self.settings.clone();
        match f {
            Flag::Artwork => self.show_artwork = !self.show_artwork,
            Flag::Stitches => self.show_plan = !self.show_plan,
            Flag::Jumps => self.show_jumps = !self.show_jumps,
            Flag::NeedlePoints => self.show_needle_points = !self.show_needle_points,
            Flag::Functions => self.show_functions = !self.show_functions,
            Flag::Density => self.show_density = !self.show_density,
            Flag::StitchCursor if self.cursor.is_some() => self.cursor = None,
            Flag::StitchCursor => self.travel(Travel::Stitches(0)),
            Flag::Grid => s.show_grid = !s.show_grid,
            Flag::Rulers => s.show_rulers = !s.show_rulers,
            Flag::DarkMode => s.dark_mode = !s.dark_mode,
            Flag::SnapGrid => s.snap_grid = !s.snap_grid,
            Flag::SnapGuides => s.snap_guides = !s.snap_guides,
            Flag::SnapObjects => s.snap_objects = !s.snap_objects,
            Flag::ThreadLook => {
                self.show_thread_look = !self.show_thread_look;
                self.show_plan |= self.show_thread_look;
            }
            Flag::Player => self.toggle_player(),
            Flag::Overview => self.show_overview = !self.show_overview,
            Flag::AppliqueFabric => self.show_applique_fabric = !self.show_applique_fabric,
            Flag::Stones => self.show_stones = !self.show_stones,
            Flag::SimpleMode => {
                s.simple_mode = !s.simple_mode;
                self.ribbon_widths.clear();
                if !RibbonTab::visible(s.simple_mode).contains(&s.ribbon_tab) {
                    s.ribbon_tab = RibbonTab::Digitize;
                }
            }
            Flag::Vectors => self.show_vectors = !self.show_vectors,
            Flag::DimVectors => {
                self.dim_vectors = !self.dim_vectors;
                self.show_vectors |= self.dim_vectors;
            }
            Flag::DrawVectors => self.draw_vectors = !self.draw_vectors,
            Flag::Hoop => self.show_hoop = !self.show_hoop,
            Flag::HoopingMode => self.set_hooping_mode(!self.hoopings_ui.mode),
            Flag::Product => match self.doc.design().product.as_ref().map(|v| v.visible) {
                Some(shown) => {
                    self.doc.edit("Show Product", |d| {
                        if let Some(v) = d.product.as_mut() {
                            v.visible = !shown;
                        }
                    });
                }
                None => self.product_window = true,
            },
            Flag::Repeats => match self.doc.design().repeat.map(|r| r.show) {
                Some(shown) => {
                    self.doc.edit("Show Repeats", |d| {
                        if let Some(r) = d.repeat.as_mut() {
                            r.show = !shown;
                        }
                    });
                }
                None => self.open_repeats(),
            },
            Flag::SplitView => {
                self.show_split = !self.show_split;
                self.split.fit_requested = true;
            }
        }
        self.set_settings(s);
    }

    /// Runs a ribbon command.
    pub fn run(&mut self, cmd: Cmd, ctx: &egui::Context) {
        let label = cmd.meta().1;
        // Space repeats it (not undo, redo and the view switches).
        if !matches!(cmd, Cmd::Undo | Cmd::Redo | Cmd::Toggle(_)) {
            self.last_cmd = Some(cmd);
        }
        match cmd {
            Cmd::Tool(t) => self.set_tool(t),
            Cmd::Toggle(f) => self.toggle(f),
            Cmd::New => self.request(Pending::New),
            Cmd::Open => self.request(Pending::Open(None)),
            Cmd::Save => {
                self.save();
            }
            Cmd::SaveAs => {
                self.save_as();
            }
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::Export => self.open_export(),
            Cmd::ImportSvg => self.import_artwork_dialog(ArtworkImport::Objects),
            Cmd::ImportSvgLines => self.import_artwork_dialog(ArtworkImport::Lines),
            Cmd::ImportSvgVectors => self.import_artwork_dialog(ArtworkImport::Vectors),
            Cmd::Convert(to) => self.convert_vectors(to),
            Cmd::Shape(op) => self.shape_selection(op),
            Cmd::Offsets => self.artwork.offsets = true,
            Cmd::Quilting => self.quilting.open = true,
            Cmd::Cutwork => self.cutwork.open = true,
            Cmd::AddBorder => {
                self.artwork.spec.make = emb_model::OffsetMake::Border {
                    width_mm: self.artwork.border_mm,
                    place: self.artwork.border_place,
                };
                self.artwork.spec.distance_mm = self.artwork.spec.distance_mm.abs().min(1.0);
                self.artwork.spec.count = 1;
                self.artwork.offsets = true;
            }
            Cmd::RemoveOverlaps => self.remove_overlaps(),
            Cmd::Arrangements => self.artwork.arrangements = true,
            Cmd::MirrorMerge(h, v) => self.arrange_selection(&emb_model::Arrangement::MirrorMerge {
                horizontal: h,
                vertical: v,
                gap_mm: self.artwork.mirror_gap_mm,
            }),
            Cmd::Reflect => self.start_reflect(),
            Cmd::BorderTrace => self.artwork.trace = true,
            Cmd::PictureTools => {
                self.selection.clear();
                self.focus_properties = true;
            }
            Cmd::TracePicture => self.trace_backdrop(),
            Cmd::PictureWizard => self.picture_to_design(),
            Cmd::ClickAs(c) => {
                self.picture.click = c;
                self.set_tool(Tool::ClickStitch);
            }
            Cmd::Unlink => self.unlink_selection(),
            Cmd::ToVector => self.convert_to_vectors(),
            Cmd::VectorToSketch => self.vectors_to_sketch(),
            Cmd::RecognizeOutlines => self.recognize_outlines(),
            Cmd::ExportOutlines => self.export_outlines_dialog(),
            Cmd::LoadPicture => self.import_bitmap_dialog(),
            Cmd::Cut => self.cut_selection(ctx),
            Cmd::Copy => self.copy_selection(ctx),
            Cmd::Paste => self.paste(None),
            Cmd::PasteAfter => self.paste_after_selected(),
            Cmd::PasteSpecial => self.open_paste_special(),
            Cmd::CloseShape(curve) => self.close_shapes(curve),
            Cmd::Reverse => self.reverse_selection(),
            Cmd::SwapSides => self.swap_sides(),
            Cmd::ToSingleStones => self.make_single_stones(),
            Cmd::Duplicate => self.duplicate_selection(),
            Cmd::Delete if self.sketch.is_some() => self.delete_sketch_selection(),
            Cmd::Delete => self.delete_selected(),
            Cmd::SelectAll => self.select_all(),
            Cmd::SelectNext => self.select_step(true),
            Cmd::SelectPrev => self.select_step(false),
            Cmd::SelectAtCursor => self.select_at_needle(),
            Cmd::BreakApart => self.break_apart(),
            Cmd::Join => self.join_selection(),
            Cmd::FlorentineGuide => {
                crate::stitch_types::florentine_guide(self);
            }
            Cmd::Laydown => {
                crate::stitch_types::add_laydown(self);
            }
            Cmd::Motifs => self.motifs_window.open = true,
            Cmd::Borders => self.decor.borders = true,
            Cmd::Buttonholes => self.decor.buttonholes = true,
            Cmd::Blends => self.decor.blends = true,
            Cmd::SplitLines => {
                crate::stitch_types::split_lines(self);
            }
            Cmd::Tartan | Cmd::Gradient => {
                self.color_fills.open = true;
                self.color_fills.gradient = cmd == Cmd::Gradient;
            }
            Cmd::GuideLines => {
                crate::stitch_types::guide_lines(self);
            }
            Cmd::RippleGuide => {
                crate::stitch_types::ripple_guide(self);
            }
            Cmd::ClosestJoin => self.closest_join(),
            Cmd::RemoveHoles => self.remove_holes(),
            Cmd::FillHoles => self.fill_holes(),
            Cmd::ContinueLine => self.continue_line(),
            Cmd::Redwork => self.make_network(RunPass::Redwork),
            Cmd::Branching => self.make_network(RunPass::Branch),
            Cmd::RotateCw => self.rotate_selection(90.0),
            Cmd::RotateCcw => self.rotate_selection(-90.0),
            Cmd::Rotate180 => self.rotate_selection(180.0),
            Cmd::MirrorH => self.mirror_selection(true),
            Cmd::MirrorV => self.mirror_selection(false),
            Cmd::CenterInHoop => self.center_in_hoop(),
            Cmd::Align(a) => self.align(a),
            Cmd::Space(across) => self.space_evenly(across),
            Cmd::SameSize(w, h) => self.make_same_size(w, h),
            Cmd::Group => self.group_selection(),
            Cmd::Ungroup => self.ungroup_selection(),
            Cmd::Lock => self.lock_selection(),
            Cmd::UnlockAll => self.unlock_all(),
            Cmd::ToStart => self.sequence_selected(label, |d, ids| d.move_objects_to_start(ids)),
            Cmd::BackOne => self.sequence_selected(label, |d, ids| d.step_objects(ids, Step::Backward)),
            Cmd::ForwardOne => self.sequence_selected(label, |d, ids| d.step_objects(ids, Step::Forward)),
            Cmd::ToEnd => self.sequence_selected(label, |d, ids| d.move_objects_to_end(ids)),
            Cmd::BackColor => self.step_color(true),
            Cmd::ForwardColor => self.step_color(false),
            Cmd::ClickOrder => {
                let ids = self.selection.ids().to_vec();
                self.resequence("Sequence by Click Order", |d| d.sequence_in_order(&ids));
            }
            Cmd::OptimizeColors => self.resequence("Optimize color changes", emb_model::Design::optimize_color_changes),
            Cmd::OptimizeOrder => self.optimize_sew_order(),
            Cmd::CapOrder => self.cap_sew_order(),
            Cmd::Puff => {
                let n = crate::stitch_types::toggle_puff(self);
                self.message = crate::i18n::fill("Puff changed on {} objects.", &[&n]);
            }
            Cmd::FindCommand => self.open_palette(),
            Cmd::GroupByColor => self.resequence("Group by Color", emb_model::Design::group_by_color),
            Cmd::ZoomExtents => {
                self.push_view();
                self.fit_requested = true;
            }
            Cmd::Zoom100 => self.set_zoom(1.0),
            Cmd::ZoomIn => self.zoom_step(true, self.settings.zoom_step),
            Cmd::ZoomOut => self.zoom_step(false, self.settings.zoom_step),
            Cmd::ZoomTwice(inward) => self.zoom_step(inward, 2.0),
            Cmd::CenterAtCursor => self.center_at_cursor(),
            Cmd::Kiosk => self.open_kiosk(),
            Cmd::EditPictureOutside => self.edit_backdrop_outside(),
            Cmd::Assets => self.assets_window.open = true,
            Cmd::NewView => {
                self.new_view();
            }
            Cmd::OtherDesign => self.show_other_design(),
            Cmd::SendByEmail => self.send_by_email(),
            Cmd::HelpContents => self.open_help("getting-started", None),
            Cmd::HelpTopic(id) => self.open_help(id, None),
            Cmd::KeyboardShortcuts => self.open_help(crate::help::KEYBOARD_ID, None),
            Cmd::StartPage => self.show_start_page(),
            Cmd::Close => self.request(Pending::CloseTab),
            Cmd::CloseAll => self.request(Pending::CloseAll),
            Cmd::ZoomHoop => {
                self.push_view();
                self.zoom_to = self.hoop_box();
            }
            Cmd::ZoomSelection => {
                self.push_view();
                self.zoom_to_selection();
            }
            Cmd::ShowSelectedColors => {
                let d = self.doc.design();
                let threads = self
                    .selection
                    .ids()
                    .iter()
                    .filter_map(|id| d.object(*id))
                    .map(|o| o.thread)
                    .collect();
                self.view_filter.threads = Some(threads);
            }
            Cmd::HideOthers => self.view_filter.objects = Some(self.selection.ids().iter().copied().collect()),
            Cmd::HideSelected => self.hide_selection(),
            Cmd::ShowAll => self.view_filter = ViewFilter::default(),
            Cmd::RemoveGuides => {
                self.doc.edit("Remove Guides", |d| d.guides.clear());
            }
            Cmd::Options => self.show_options = true,
            Cmd::RotateDesign => {
                self.doc.edit("Rotate Design", |d| d.rotation = d.rotation.turned_cw());
            }
            Cmd::AutoHoop => self.auto_hoop(),
            Cmd::TurnHoopLeft => self.turn_hoop(-crate::hoops::HOOP_STEP_DEG),
            Cmd::TurnHoopRight => self.turn_hoop(crate::hoops::HOOP_STEP_DEG),
            Cmd::HoopOptions => self.hoop_windows.options = true,
            Cmd::MyHoops => self.hoop_windows.my_hoops = true,
            Cmd::HoopTemplate => self.hoop_template_dialog(),
            Cmd::AutoLayout => self.auto_layout(),
            Cmd::ProductWindow => self.product_window = true,
            Cmd::SaveDecoration => self.save_decoration_dialog(),
            Cmd::RepeatsWindow => self.open_repeats(),
            Cmd::About => self.about_window = true,
            Cmd::Library => self.focus_library = true,
            Cmd::ClipartPanel => self.focus_clipart = true,
            Cmd::CutterFile => self.cutter_file_dialog(),
            Cmd::SaveClipart => self.open_save_clipart(),
            Cmd::SendToUsb => self.send_to_usb(),
            Cmd::SendToUsbChoose => self.open_send_dialog(None),
            Cmd::MakeRepeats => self.make_repeats(),
            Cmd::AddHooping => self.add_hooping(),
            Cmd::DeleteHooping => self.delete_hooping_pick(),
            Cmd::TurnHooping => self.turn_hooping(),
            Cmd::DrawSplit => self.toggle_draw_split(),
            Cmd::DeleteSplit => {
                self.doc.edit("Remove Split Lines", |d| {
                    if let Some(l) = d.hooping.as_mut() {
                        l.splits.clear();
                    }
                });
                self.hoopings_ui.selected = None;
            }
            Cmd::ClearHoopings => self.clear_hoopings(),
            Cmd::PreviewHoopings => self.hoopings_ui.preview = !self.hoopings_ui.preview,
            Cmd::OutputHoopings => self.output_hoopings_dialog(),
            Cmd::ThreadsPanel => self.focus_threads = true,
            Cmd::MatchChart => self.match_to_chart(),
            Cmd::ImportChart => self.import_chart_dialog(),
            Cmd::RemoveUnused => self.remove_unused_threads(),
            Cmd::CycleColors => self.cycle_colors(),
            Cmd::SplitThread => self.split_thread(),
            Cmd::NewColorway => self.new_colorway(),
            Cmd::DeleteColorway => self.delete_colorway(),
            Cmd::Func(f) => self.toggle_func(f),
            Cmd::StartSketch => self.start_sketch(),
            Cmd::Sketch(SketchCmd::Finish) => self.finish_sketch(),
            Cmd::Sketch(SketchCmd::Construction) => self.toggle_construction(),
            Cmd::Sketch(SketchCmd::Relax) => self.relax_sketch(),
            Cmd::Sketch(SketchCmd::Centerline) => self.toggle_centerline(),
            Cmd::Sketch(SketchCmd::CenterPoint) => self.toggle_center_point(),
            Cmd::Sketch(SketchCmd::AutoDimension) => self.auto_dimension(),
            Cmd::Sketch(SketchCmd::Parameters) => self.toggle_parameters(),
            Cmd::Sketch(SketchCmd::RelaxMode) => {
                if let Some(e) = &mut self.sketch {
                    e.relax_mode = !e.relax_mode;
                }
            }
            Cmd::Sketch(SketchCmd::Tool(t)) => self.set_sketch_tool(t),
            Cmd::SplitAtNeedle => self.split_at_needle(),
            Cmd::StitchListPanel => self.focus_stitch_list = true,
            Cmd::Fabric => self.open_fabric_dialog(),
            Cmd::SewTimes => self.show_sew_times = true,
            Cmd::Needles => self.show_needles = true,
            Cmd::Quote => self.show_quote = true,
            Cmd::OrderDetails => self.show_order = true,
            Cmd::ThreadStock => self.show_stock = true,
            Cmd::Scripts => self.show_scripts = true,
            Cmd::Fonts => self.fonts_window.open = true,
            Cmd::LetteringOnLine => self.lettering_on_line(),
            Cmd::Monogram => self.open_monogram(),
            Cmd::TeamNames => self.open_team_names(),
            Cmd::Alphabets => self.alphabets_window.open = true,
            Cmd::KeyboardDesigns => self.keyboard_window.open = true,
            Cmd::QuickSimplify => self.quick_simplify(),
            Cmd::ExportSvg => self.export_svg_dialog(),
            Cmd::Simplify => self.open_simplify(),
            Cmd::CornerRadius => self.open_corner_dialog(),
            Cmd::HoleFromOutline => self.open_hole_dialog(),
            Cmd::PreviousView => self.previous_view(),
            Cmd::ConvertToApplique => self.convert_to_applique(),
            Cmd::ConvertToCross => self.convert_to_cross(),
            Cmd::CrossChart => self.cross_chart_dialog(),
            Cmd::ToStonesRun => self.convert_to_stones(crate::bling::ToStones::Outline),
            Cmd::ToStonesCenter => self.convert_to_stones(crate::bling::ToStones::CenterLine),
            Cmd::StoneTemplate => self.stone_template_dialog(),
            Cmd::TemplateMarks => self.add_template_marks(),
            Cmd::ToStonesFill => self.convert_to_stones(crate::bling::ToStones::Fill),
            Cmd::PictureToCross => {
                self.picture_to_design();
                if let Some(w) = self.wizard.as_mut() {
                    w.make = crate::wizard::Make::Cross;
                }
            }
            Cmd::CombineApplique => self.combine_applique(),
            Cmd::MakeCurrent => self.make_properties_current(),
            Cmd::ApplyCurrent => self.apply_current_properties(),
            Cmd::StylesPanel => self.focus_styles = true,
            Cmd::NewFromTemplate => self.open_template_dialog(false),
            Cmd::SaveAsTemplate => self.open_template_dialog(true),
            Cmd::InsertMachineFile => self.insert_machine_file_dialog(),
            Cmd::OutputDesign => self.open_output_dialog(),
            Cmd::PrintPreview => self.preview = Some(Default::default()),
            Cmd::BatchOutput => self.batch_output_dialog(),
        }
    }
}

// ---- Layout of the tabs ----

/// One part of a panel.
enum Item {
    /// A large button: icon above the name.
    Large(Cmd),
    /// Up to three small buttons, one above the other.
    Small(&'static [Cmd]),
    /// Controls that are not plain buttons, and the commands that they
    /// show (for the help and the tests).
    Custom(fn(&mut egui::Ui, &mut AppState, &SelInfo), &'static [Cmd]),
}

struct Panel {
    title: &'static str,
    items: &'static [Item],
}

use Item::{Custom, Large, Small};

const fn t(tool: Tool) -> Cmd {
    Cmd::Tool(tool)
}

const fn f(flag: Flag) -> Cmd {
    Cmd::Toggle(flag)
}

const DIGITIZE: &[Panel] = &[
    Panel {
        title: "Select",
        items: &[
            Large(t(Tool::Select)),
            Large(t(Tool::Reshape)),
            Small(&[t(Tool::Lasso), t(Tool::PolySelect), t(Tool::LineSelect)]),
        ],
    },
    Panel {
        title: "Lines",
        items: &[Large(t(Tool::Line)), Small(&[t(Tool::Manual), t(Tool::Freehand)])],
    },
    Panel {
        title: "Fills",
        items: &[
            Large(t(Tool::Shape)),
            Small(&[t(Tool::ComplexFill), t(Tool::FreeShape), t(Tool::Turning)]),
        ],
    },
    Panel {
        title: "Columns",
        items: &[
            Large(t(Tool::Column)),
            Small(&[t(Tool::ColumnB), t(Tool::ColumnC), t(Tool::Ring)]),
        ],
    },
    Panel {
        title: "Shapes",
        items: &[
            Small(&[t(Tool::Rectangle), t(Tool::RoundRect), t(Tool::Ellipse)]),
            Small(&[t(Tool::Star), Cmd::StartSketch]),
        ],
    },
    Panel {
        title: "Decorate",
        items: &[Large(Cmd::Borders), Small(&[Cmd::Buttonholes, Cmd::Blends])],
    },
    Panel {
        title: "Appliqué",
        items: &[
            Large(t(Tool::Applique)),
            Small(&[Cmd::ConvertToApplique, Cmd::CombineApplique, Cmd::CutterFile]),
        ],
    },
];

/// Lettering and personalization.
const LETTERING: &[Panel] = &[
    Panel {
        title: "Lettering",
        items: &[
            Large(t(Tool::Lettering)),
            Small(&[Cmd::Fonts, Cmd::Alphabets, Cmd::LetteringOnLine]),
        ],
    },
    Panel {
        title: "Personalize",
        items: &[
            Large(Cmd::Monogram),
            Small(&[Cmd::TeamNames, Cmd::KeyboardDesigns]),
            Large(Cmd::Kiosk),
        ],
    },
];

/// Specialty decoration (P9, P11): cross stitch, rhinestones, cutwork,
/// quilting, puff and laydown. Each has its own panel.
const SPECIALTY: &[Panel] = &[
    Panel {
        title: "Cross Stitch",
        items: &[
            Large(t(Tool::CrossStitch)),
            Small(&[t(Tool::CrossErase), t(Tool::Backstitch), Cmd::ConvertToCross]),
            Large(Cmd::PictureToCross),
            Large(Cmd::CrossChart),
        ],
    },
    Panel {
        title: "Rhinestones",
        items: &[
            Large(t(Tool::StoneRun)),
            Small(&[t(Tool::Stone), t(Tool::StoneFill)]),
            Small(&[Cmd::ToStonesRun, Cmd::ToStonesCenter, Cmd::ToStonesFill]),
            Small(&[Cmd::ToSingleStones, f(Flag::Stones)]),
            Large(Cmd::StoneTemplate),
            Small(&[Cmd::TemplateMarks]),
        ],
    },
    Panel {
        title: "Cutwork",
        items: &[Large(Cmd::Cutwork)],
    },
    Panel {
        title: "Quilting",
        items: &[Large(Cmd::Quilting)],
    },
    Panel {
        title: "Puff and Laydown",
        items: &[Large(Cmd::Puff), Large(Cmd::Laydown)],
    },
];

/// Pictures and vector artwork (P6): import, picture tools, trace, draw,
/// convert, shaping, borders and offsets, and export. The copies of the
/// selection (arrangements) are on the Arrange tab, quilting is on the
/// Specialty tab (ADR 0055).
const fn sk(t: crate::sketch::SketchTool) -> Cmd {
    Cmd::Sketch(SketchCmd::Tool(t))
}

const fn kc(k: crate::sketch::Kind) -> Cmd {
    Cmd::Sketch(SketchCmd::Tool(crate::sketch::SketchTool::Constrain(k)))
}

/// The constraint commands of the Sketch tab, as a grid of icons (as the
/// other program shows them; the names are in the tooltips).
const SKETCH_CONSTRAINTS: [Cmd; 13] = {
    use crate::sketch::Kind as K;
    [
        kc(K::Coincident),
        kc(K::Collinear),
        kc(K::Concentric),
        kc(K::Fix),
        kc(K::Parallel),
        kc(K::Perpendicular),
        kc(K::Horizontal),
        kc(K::Vertical),
        kc(K::Tangent),
        kc(K::Equal),
        kc(K::Midpoint),
        kc(K::OnCurve),
        kc(K::Symmetric),
    ]
};

/// The grid of the constraint commands: three rows of icons.
fn constraint_icons(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    for row in SKETCH_CONSTRAINTS.chunks(5) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for cmd in row {
                icon_cmd(ui, st, s, *cmd);
            }
        });
    }
}

/// A command as an icon only (the name is the tooltip and the screen
/// reader name).
fn icon_cmd(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, cmd: Cmd) {
    let (icon, label, _) = cmd.meta();
    let enabled = st.can(cmd, s);
    let on = st.is_on(cmd);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SMALL_H), sense(enabled));
    let resp = crate::a11y::named(resp, label);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let (fg, bg) = colors(ui, enabled);
    icons::paint(ui.painter(), rect.shrink(3.0), icon, fg, bg);
    let ctx = ui.ctx().clone();
    let resp = tooltip(resp, &ctx, cmd);
    if resp.hovered() {
        st.hover_cmd = Some(cmd);
    }
    if resp.clicked() {
        st.run(cmd, &ctx);
    }
}

/// The Sketch tab (sketch mode, P14 Milestone 2).
const SKETCH: &[Panel] = {
    use crate::sketch::SketchTool as T;
    &[
        Panel {
            title: "Sketch",
            items: &[
                Large(Cmd::Sketch(SketchCmd::Finish)),
                Large(sk(T::Select)),
                Large(sk(T::Regions)),
            ],
        },
        Panel {
            title: "Draw",
            items: &[
                Large(sk(T::Line)),
                Large(sk(T::Spline)),
                Small(&[sk(T::Rectangle), sk(T::RectCenter), sk(T::Rect3)]),
                Small(&[sk(T::RectCenter3), sk(T::Slot), sk(T::Polygon)]),
                Small(&[sk(T::Circle), sk(T::TangentCircle), sk(T::Point)]),
                Small(&[sk(T::Arc3), sk(T::ArcCenter), sk(T::ArcTangent)]),
                Small(&[
                    Cmd::Sketch(SketchCmd::Construction),
                    Cmd::Sketch(SketchCmd::Centerline),
                    Cmd::Sketch(SketchCmd::CenterPoint),
                ]),
            ],
        },
        Panel {
            title: "Constrain",
            items: &[Custom(constraint_icons, &SKETCH_CONSTRAINTS)],
        },
        Panel {
            title: "Dimension",
            items: &[
                Large(sk(T::Dimension)),
                Large(Cmd::Sketch(SketchCmd::Parameters)),
                Small(&[
                    Cmd::Sketch(SketchCmd::AutoDimension),
                    Cmd::Sketch(SketchCmd::Relax),
                    Cmd::Sketch(SketchCmd::RelaxMode),
                ]),
            ],
        },
        Panel {
            title: "Modify",
            items: &[
                Small(&[sk(T::Trim), sk(T::Extend), sk(T::Split)]),
                Small(&[sk(T::Fillet), sk(T::Chamfer), sk(T::Offset)]),
                Small(&[sk(T::Move), sk(T::Copy), sk(T::Rotate)]),
                Small(&[sk(T::Scale), sk(T::Stretch)]),
            ],
        },
        Panel {
            title: "Copy",
            items: &[Small(&[sk(T::Mirror), sk(T::RectPattern), sk(T::CircularPattern)])],
        },
    ]
};

const ARTWORK: &[Panel] = &[
    Panel {
        title: "Import",
        items: &[
            Large(Cmd::PictureWizard),
            Large(Cmd::ImportSvgVectors),
            Small(&[Cmd::LoadPicture, Cmd::ImportSvg, Cmd::ImportSvgLines]),
            Small(&[Cmd::InsertMachineFile]),
        ],
    },
    Panel {
        title: "Picture",
        items: &[
            Large(Cmd::PictureTools),
            Small(&[t(Tool::CropRect), t(Tool::CropEllipse), t(Tool::CropPolygon)]),
            Small(&[Cmd::EditPictureOutside]),
        ],
    },
    Panel {
        title: "Trace",
        items: &[
            Large(Cmd::TracePicture),
            Large(t(Tool::ClickStitch)),
            Small(&[
                Cmd::ClickAs(emb_art::click::ClickStitch::Fill),
                Cmd::ClickAs(emb_art::click::ClickStitch::FillNoHoles),
                Cmd::ClickAs(emb_art::click::ClickStitch::Satin),
            ]),
            Small(&[
                Cmd::ClickAs(emb_art::click::ClickStitch::CenterLine),
                Cmd::ClickAs(emb_art::click::ClickStitch::Outline),
            ]),
        ],
    },
    Panel {
        title: "Draw",
        items: &[
            Large(f(Flag::DrawVectors)),
            Small(&[t(Tool::Shape), t(Tool::Line), t(Tool::Freehand)]),
            Small(&[t(Tool::Rectangle), t(Tool::Ellipse), t(Tool::Star)]),
            Small(&[f(Flag::Vectors), f(Flag::DimVectors)]),
        ],
    },
    Panel {
        title: "Convert",
        items: &[
            Large(Cmd::Convert(emb_model::ConvertTo::Fill)),
            Small(&[
                Cmd::Convert(emb_model::ConvertTo::Satin),
                Cmd::Convert(emb_model::ConvertTo::Outline),
                Cmd::ToVector,
            ]),
            Small(&[Cmd::RecognizeOutlines, Cmd::VectorToSketch]),
        ],
    },
    Panel {
        title: "Shaping",
        items: &[
            Large(Cmd::Shape(emb_model::ShapeOp::Weld)),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::Intersect),
                Cmd::Shape(emb_model::ShapeOp::Exclude),
                Cmd::Shape(emb_model::ShapeOp::Combine),
            ]),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::FrontMinusBack),
                Cmd::Shape(emb_model::ShapeOp::BackMinusFront),
            ]),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::Flatten),
                Cmd::Shape(emb_model::ShapeOp::Divide),
                Cmd::RemoveOverlaps,
            ]),
            Small(&[Cmd::Unlink]),
            Custom(shaping_options, &[]),
        ],
    },
    Panel {
        title: "Borders and Offsets",
        items: &[Large(Cmd::BorderTrace), Large(Cmd::Offsets), Small(&[Cmd::AddBorder])],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::ExportOutlines)],
    },
];

const EDIT: &[Panel] = &[
    Panel {
        title: "Clipboard",
        items: &[
            Large(Cmd::Paste),
            Small(&[Cmd::Cut, Cmd::Copy, Cmd::Duplicate]),
            Small(&[Cmd::PasteAfter, Cmd::PasteSpecial, Cmd::Delete]),
        ],
    },
    Panel {
        title: "Clipart",
        items: &[Large(Cmd::ClipartPanel), Small(&[Cmd::SaveClipart])],
    },
    Panel {
        title: "Select",
        items: &[
            Large(Cmd::SelectAll),
            Small(&[Cmd::SelectNext, Cmd::SelectPrev, Cmd::SelectAtCursor]),
            Custom(select_by, &[]),
        ],
    },
    Panel {
        title: "Split and Join",
        items: &[
            Large(t(Tool::Knife)),
            Small(&[Cmd::BreakApart, Cmd::Join, Cmd::SplitAtNeedle]),
        ],
    },
    Panel {
        title: "Shape",
        items: &[
            Large(t(Tool::StitchAngle)),
            Small(&[t(Tool::AddHole), Cmd::RemoveHoles, Cmd::FillHoles]),
            Small(&[t(Tool::DirectionLine), Cmd::CornerRadius, Cmd::HoleFromOutline]),
            Small(&[Cmd::QuickSimplify, Cmd::Simplify]),
        ],
    },
    Panel {
        title: "Lines and Columns",
        items: &[
            Small(&[Cmd::ContinueLine, Cmd::Redwork, Cmd::Branching]),
            Small(&[Cmd::CloseShape(false), Cmd::CloseShape(true), Cmd::Reverse]),
            Small(&[Cmd::SwapSides]),
        ],
    },
    Panel {
        title: "Stitch Types",
        items: &[
            Custom(stitch_types, &[]),
            Small(&[Cmd::Motifs, Cmd::FlorentineGuide, Cmd::SplitLines]),
            Small(&[Cmd::Tartan, Cmd::Gradient, Cmd::GuideLines]),
            Small(&[Cmd::RippleGuide]),
        ],
    },
    Panel {
        title: "Machine Functions",
        items: &[
            Small(&[
                Cmd::Func(Func::Stop),
                Cmd::Func(Func::TrimAlways),
                Cmd::Func(Func::TrimNever),
            ]),
            Small(&[Cmd::Func(Func::TieIn), Cmd::Func(Func::TieOff)]),
        ],
    },
    Panel {
        title: "Scripts",
        items: &[Large(Cmd::Scripts)],
    },
];

const ARRANGE: &[Panel] = &[
    Panel {
        title: "Transform",
        items: &[
            Small(&[Cmd::RotateCcw, Cmd::RotateCw, Cmd::Rotate180]),
            Small(&[Cmd::MirrorH, Cmd::MirrorV, Cmd::CenterInHoop]),
            Small(&[t(Tool::MirrorLine), t(Tool::RefPoints)]),
            Custom(rotate_skew, &[]),
        ],
    },
    Panel {
        title: "Align",
        items: &[
            Small(&[
                Cmd::Align(Align::Left),
                Cmd::Align(Align::CenterX),
                Cmd::Align(Align::Right),
            ]),
            Small(&[
                Cmd::Align(Align::Top),
                Cmd::Align(Align::CenterY),
                Cmd::Align(Align::Bottom),
            ]),
        ],
    },
    Panel {
        title: "Space and Size",
        items: &[
            Small(&[Cmd::Space(true), Cmd::Space(false)]),
            Small(&[
                Cmd::SameSize(true, false),
                Cmd::SameSize(false, true),
                Cmd::SameSize(true, true),
            ]),
        ],
    },
    Panel {
        title: "Copies",
        items: &[
            Large(Cmd::Arrangements),
            Small(&[
                Cmd::MirrorMerge(true, false),
                Cmd::MirrorMerge(false, true),
                Cmd::MirrorMerge(true, true),
            ]),
            Small(&[Cmd::Reflect]),
        ],
    },
    Panel {
        title: "Group and Lock",
        items: &[Small(&[Cmd::Group, Cmd::Ungroup]), Small(&[Cmd::Lock, Cmd::UnlockAll])],
    },
    Panel {
        title: "Sew Order",
        items: &[
            Small(&[Cmd::ToStart, Cmd::BackOne, Cmd::ForwardOne]),
            Small(&[Cmd::ToEnd, Cmd::BackColor, Cmd::ForwardColor]),
            Small(&[Cmd::ClickOrder, Cmd::OptimizeColors, Cmd::GroupByColor]),
            Small(&[Cmd::OptimizeOrder, Cmd::CapOrder, Cmd::ClosestJoin]),
        ],
    },
];

const COLOR: &[Panel] = &[
    Panel {
        title: "Threads",
        items: &[
            Large(Cmd::ThreadsPanel),
            Small(&[Cmd::RemoveUnused, Cmd::CycleColors, Cmd::SplitThread]),
        ],
    },
    Panel {
        title: "Thread Chart",
        items: &[Custom(chart_pick, &[]), Small(&[Cmd::MatchChart, Cmd::ImportChart])],
    },
    Panel {
        title: "Colorways",
        items: &[
            Custom(colorway_pick, &[]),
            Small(&[Cmd::NewColorway, Cmd::DeleteColorway]),
        ],
    },
    Panel {
        title: "Fabric Color",
        items: &[Custom(background_pick, &[])],
    },
];

const VIEW: &[Panel] = &[
    Panel {
        title: "Show",
        items: &[
            Custom(look_column, &[f(Flag::Artwork), f(Flag::Stitches), f(Flag::ThreadLook)]),
            Small(&[f(Flag::Jumps), f(Flag::NeedlePoints), f(Flag::Functions)]),
            Small(&[f(Flag::AppliqueFabric), f(Flag::Vectors), f(Flag::DimVectors)]),
            Small(&[f(Flag::Density)]),
        ],
    },
    Panel {
        title: "Stitch Cursor",
        items: &[
            Large(f(Flag::Player)),
            Large(f(Flag::StitchCursor)),
            Large(Cmd::StitchListPanel),
            Custom(cursor_tools, &[Cmd::CenterAtCursor]),
        ],
    },
    Panel {
        title: "Filter",
        items: &[
            Small(&[Cmd::ShowSelectedColors, Cmd::HideOthers, Cmd::HideSelected]),
            Custom(show_colors, &[Cmd::ShowAll]),
        ],
    },
    Panel {
        title: "Zoom",
        items: &[
            Large(Cmd::ZoomExtents),
            Small(&[Cmd::ZoomIn, Cmd::ZoomOut, Cmd::Zoom100]),
            Small(&[Cmd::ZoomHoop, Cmd::ZoomSelection, t(Tool::Zoom)]),
            Custom(
                views_column,
                &[
                    f(Flag::Overview),
                    f(Flag::SplitView),
                    Cmd::NewView,
                    Cmd::OtherDesign,
                    Cmd::PreviousView,
                ],
            ),
            Custom(zoom_factor, &[Cmd::ZoomTwice(false), Cmd::ZoomTwice(true)]),
        ],
    },
    Panel {
        title: "Workspace",
        items: &[
            Small(&[f(Flag::Grid), f(Flag::Rulers), t(Tool::Measure)]),
            Small(&[f(Flag::SnapGrid), f(Flag::SnapGuides), f(Flag::SnapObjects)]),
            Custom(workspace_more, &[Cmd::RemoveGuides, f(Flag::DarkMode)]),
            Large(Cmd::Options),
        ],
    },
];

const DESIGN: &[Panel] = &[
    Panel {
        title: "Machine and Hoop",
        items: &[Custom(machine_hoop, &[]), Small(&[Cmd::SewTimes, Cmd::Needles])],
    },
    Panel {
        title: "Hoop",
        items: &[
            Large(Cmd::AutoHoop),
            Large(Cmd::RotateDesign),
            Small(&[Cmd::TurnHoopLeft, Cmd::TurnHoopRight, f(Flag::Hoop)]),
            Small(&[Cmd::MyHoops, Cmd::HoopOptions, Cmd::HoopTemplate]),
        ],
    },
    Panel {
        title: "Fabric and Styles",
        items: &[
            Large(Cmd::Fabric),
            Small(&[Cmd::MakeCurrent, Cmd::ApplyCurrent, Cmd::StylesPanel]),
            Custom(style_picker, &[]),
            Custom(favorites, &[]),
        ],
    },
    Panel {
        title: "Library and Templates",
        items: &[
            Large(Cmd::Library),
            Small(&[Cmd::NewFromTemplate, Cmd::SaveAsTemplate, Cmd::Assets]),
        ],
    },
    Panel {
        title: "Repeats",
        items: &[Large(Cmd::RepeatsWindow), Small(&[f(Flag::Repeats), Cmd::MakeRepeats])],
    },
    Panel {
        title: "Product",
        items: &[
            Large(Cmd::ProductWindow),
            Small(&[f(Flag::Product), Cmd::SaveDecoration]),
        ],
    },
    Panel {
        title: "Orders",
        items: &[Large(Cmd::OrderDetails), Large(Cmd::Quote), Large(Cmd::ThreadStock)],
    },
    Panel {
        title: "Output",
        items: &[
            Large(Cmd::OutputDesign),
            Large(Cmd::Export),
            Large(Cmd::SendToUsb),
            Small(&[Cmd::PrintPreview, Cmd::BatchOutput, Cmd::ExportSvg]),
            Small(&[Cmd::SendToUsbChoose, Cmd::SendByEmail]),
        ],
    },
];

const HOOPING: &[Panel] = &[
    Panel {
        title: "Hooping",
        items: &[Large(f(Flag::HoopingMode)), Large(Cmd::AutoLayout)],
    },
    Panel {
        title: "Hoopings and Split Lines",
        items: &[
            Small(&[Cmd::AddHooping, Cmd::TurnHooping, Cmd::DeleteHooping]),
            Small(&[Cmd::DrawSplit, Cmd::DeleteSplit, Cmd::ClearHoopings]),
        ],
    },
    Panel {
        title: "Options",
        items: &[Custom(hooping_options, &[])],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::PreviewHoopings), Large(Cmd::OutputHoopings)],
    },
];

fn hooping_options(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    crate::hooping::options(ui, st);
}

// ---- Simple mode (P10 Milestone 5): the tools of the patch and lettering tasks, large. ----

const S_DIGITIZE: &[Panel] = &[
    Panel {
        title: "Select",
        items: &[Large(t(Tool::Select)), Large(t(Tool::Reshape))],
    },
    Panel {
        title: "Draw",
        items: &[
            Large(t(Tool::Line)),
            Large(t(Tool::Shape)),
            Large(t(Tool::Column)),
            Large(t(Tool::Rectangle)),
            Large(t(Tool::Ellipse)),
            Large(t(Tool::Applique)),
        ],
    },
    Panel {
        title: "Picture",
        items: &[Large(Cmd::PictureWizard), Large(Cmd::LoadPicture)],
    },
];

const S_LETTERING: &[Panel] = &[
    Panel {
        title: "Lettering",
        items: &[Large(t(Tool::Lettering)), Large(Cmd::Fonts)],
    },
    Panel {
        title: "Personalize",
        items: &[Large(Cmd::Monogram), Large(Cmd::TeamNames), Large(Cmd::Kiosk)],
    },
];

const S_EDIT: &[Panel] = &[
    Panel {
        title: "Clipboard",
        items: &[
            Large(Cmd::Paste),
            Large(Cmd::Copy),
            Large(Cmd::Cut),
            Large(Cmd::Duplicate),
            Large(Cmd::Delete),
        ],
    },
    Panel {
        title: "Arrange",
        items: &[
            Large(Cmd::RotateCw),
            Large(Cmd::MirrorH),
            Large(Cmd::CenterInHoop),
            Large(Cmd::SelectAll),
        ],
    },
];

const S_VIEW: &[Panel] = &[
    Panel {
        title: "Show",
        items: &[Large(f(Flag::ThreadLook)), Large(f(Flag::Player))],
    },
    Panel {
        title: "Zoom",
        items: &[
            Large(Cmd::ZoomExtents),
            Large(Cmd::ZoomHoop),
            Large(Cmd::ZoomIn),
            Large(Cmd::ZoomOut),
        ],
    },
    Panel {
        title: "Mode",
        items: &[Large(f(Flag::SimpleMode))],
    },
];

const S_DESIGN: &[Panel] = &[
    Panel {
        title: "Machine and Hoop",
        items: &[Custom(machine_hoop, &[]), Large(Cmd::AutoHoop)],
    },
    Panel {
        title: "Fabric",
        items: &[Large(Cmd::Fabric)],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::Export), Large(Cmd::OutputDesign), Large(Cmd::SendToUsb)],
    },
];

const HELP: &[Panel] = &[
    Panel {
        title: "Help",
        items: &[
            Large(Cmd::HelpContents),
            Large(Cmd::FindCommand),
            Large(Cmd::StartPage),
            Small(&[Cmd::KeyboardShortcuts, Cmd::HelpTopic("troubleshooting"), Cmd::About]),
        ],
    },
    Panel {
        title: "Mode",
        items: &[Large(f(Flag::SimpleMode))],
    },
    Panel {
        title: "Tutorials",
        items: &[
            Small(&[
                Cmd::HelpTopic("tutorial-patch"),
                Cmd::HelpTopic("tutorial-lettering"),
                Cmd::HelpTopic("tutorial-applique"),
            ]),
            Small(&[
                Cmd::HelpTopic("tutorial-towel"),
                Cmd::HelpTopic("tutorial-hooping"),
                Cmd::HelpTopic("tutorial-lace"),
            ]),
            Small(&[Cmd::HelpTopic("tutorial-output")]),
        ],
    },
];

/// All commands of the full ribbon (also in simple mode), with their tab
/// and panel title, in ribbon order (a command can be in two panels).
pub fn all_commands() -> Vec<(Cmd, RibbonTab, &'static str)> {
    // The quick-access buttons next to the File button first.
    let mut out: Vec<(Cmd, RibbonTab, &'static str)> = QUICK
        .iter()
        .map(|c| (*c, RibbonTab::Digitize, tr("Quick access")))
        .collect();
    for tab in RibbonTab::ALL {
        for p in tab.panels() {
            for item in p.items {
                match item {
                    Large(c) => out.push((*c, tab, p.title)),
                    Small(cs) | Custom(_, cs) => out.extend(cs.iter().map(|c| (*c, tab, p.title))),
                }
            }
        }
    }
    out
}

/// The commands next to the File button.
const QUICK: [Cmd; 5] = [Cmd::New, Cmd::Open, Cmd::Save, Cmd::Undo, Cmd::Redo];

// ---- Widgets ----

const LARGE: Vec2 = vec2(60.0, 68.0);
const SMALL_H: f32 = 22.0;
const ROW_GAP: f32 = 1.0;
const CONTENT_H: f32 = 3.0 * SMALL_H + 2.0 * ROW_GAP;
const TITLE_H: f32 = 15.0;

#[cfg(test)]
thread_local! {
    /// Button rectangles by name, so tests can click them.
    pub static RECTS: std::cell::RefCell<Vec<(String, Rect)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// The width of the panels of the tab that showed last.
    pub static PANELS_W: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

fn note_rect(_label: &str, _rect: Rect) {
    #[cfg(test)]
    RECTS.with(|r| r.borrow_mut().push((_label.to_string(), _rect)));
}

/// Paints the background of a button for its state.
fn frame(ui: &egui::Ui, rect: Rect, resp: &egui::Response, on: bool, enabled: bool) {
    let v = ui.visuals();
    let fill = if on {
        v.selection.bg_fill.gamma_multiply(0.45)
    } else if enabled && resp.hovered() {
        v.widgets.hovered.weak_bg_fill
    } else {
        return;
    };
    let stroke = if on {
        Stroke::new(1.0, v.selection.stroke.color.gamma_multiply(0.6))
    } else {
        Stroke::NONE
    };
    ui.painter().rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
}

fn colors(ui: &egui::Ui, enabled: bool) -> (Color32, Color32) {
    let v = ui.visuals();
    let fg = if enabled {
        v.text_color()
    } else {
        v.weak_text_color().gamma_multiply(0.6)
    };
    (fg, v.panel_fill)
}

fn tooltip(resp: egui::Response, ctx: &egui::Context, cmd: Cmd) -> egui::Response {
    let (_, name, tip) = cmd.meta();
    let text = match cmd.keys(ctx) {
        Some(k) => format!("{name} ({k})\n{tip}\nF1: help"),
        None => format!("{name}\n{tip}\nF1: help"),
    };
    resp.on_hover_text(&text).on_disabled_hover_text(text)
}

/// The id of the first ribbon button that can take the keyboard focus (F6).
const FIRST_BUTTON: &str = "ribbon_first_button";

/// Keeps the id of the first button of the frame that can take the focus.
fn note_focus(ui: &egui::Ui, resp: &egui::Response) {
    if resp.sense.senses_click() {
        let key = egui::Id::new(FIRST_BUTTON);
        let pass = ui.ctx().cumulative_pass_nr();
        ui.ctx().data_mut(|d| {
            let have = d.get_temp::<(u64, egui::Id)>(key);
            if have.is_none_or(|(p, _)| p != pass) {
                d.insert_temp(key, (pass, resp.id));
            }
        });
    }
}

/// F6: the keyboard focus goes to the first ribbon button. Then Tab and
/// Shift+Tab go from button to button, and Enter or Space runs one.
pub fn focus_ribbon(ctx: &egui::Context) -> bool {
    let id = ctx.data(|d| d.get_temp::<(u64, egui::Id)>(egui::Id::new(FIRST_BUTTON)));
    match id {
        Some((_, id)) => {
            ctx.memory_mut(|m| m.request_focus(id));
            true
        }
        None => false,
    }
}

fn sense(enabled: bool) -> Sense {
    if enabled { Sense::click() } else { Sense::hover() }
}

/// The widest that a large button gets, so that its name has two lines at
/// most.
const LARGE_MAX_W: f32 = 120.0;

/// The width of a large button and the font size of its name: two lines
/// at most (the button gets wider for a long name, also of a folded panel),
/// and no word wider than the button (a long word of a translation makes
/// the text smaller), so no word breaks in its middle.
fn large_fit(ui: &egui::Ui, label: &str) -> (f32, f32) {
    let painter = ui.painter();
    let color = ui.visuals().text_color();
    let rows = |size: f32, w: f32| {
        painter
            .layout(label.to_string(), FontId::proportional(size), color, w - 4.0)
            .rows
            .len()
    };
    let widest = |size: f32| {
        label
            .split_whitespace()
            .map(|w| {
                painter
                    .layout_no_wrap(w.to_string(), FontId::proportional(size), color)
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
    };
    let mut size = 11.5;
    while size > 8.5 && widest(size) > LARGE.x - 4.0 {
        size -= 0.5;
    }
    let mut w = LARGE.x.max((widest(size) + 4.0).min(LARGE_MAX_W));
    while w < LARGE_MAX_W && rows(size, w) > 2 {
        w += 4.0;
    }
    (w.ceil(), size)
}

/// A large button: the icon above the name (two lines at most).
fn large(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool, enabled: bool) -> egui::Response {
    let (w, size) = large_fit(ui, label);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, LARGE.y), sense(enabled));
    let resp = crate::a11y::named_toggle(resp, label, on);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let (fg, bg) = colors(ui, enabled);
    let icon_rect = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 20.0), Vec2::splat(28.0));
    icons::paint(ui.painter(), icon_rect, icon, fg, bg);
    let galley = ui
        .painter()
        .layout(label.to_string(), FontId::proportional(size), fg, w - 4.0);
    let pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 38.0);
    ui.painter().galley(pos, galley, fg);
    resp
}

/// A small button: the icon left of the name. `menu` adds a down arrow.
fn small(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool, enabled: bool, menu: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let (fg, bg) = colors(ui, enabled);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let arrow = if menu { 12.0 } else { 0.0 };
    let size = vec2(4.0 + 17.0 + 5.0 + galley.size().x + arrow + 6.0, SMALL_H);
    let (rect, resp) = ui.allocate_exact_size(size, sense(enabled));
    let resp = crate::a11y::named_toggle(resp, label, on);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let icon_rect = Rect::from_center_size(egui::pos2(rect.left() + 12.5, rect.center().y), Vec2::splat(16.0));
    icons::paint(ui.painter(), icon_rect, icon, fg, bg);
    let text_pos = egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, fg);
    if menu {
        ui.painter().text(
            egui::pos2(rect.right() - 9.0, rect.center().y),
            Align2::CENTER_CENTER,
            ph::CARET_DOWN,
            FontId::proportional(10.0),
            fg,
        );
    }
    resp
}

/// A button with only an icon (the quick-access buttons).
fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SMALL_H), sense(enabled));
    let resp = crate::a11y::named(resp, label);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, false, enabled);
    let (fg, bg) = colors(ui, enabled);
    icons::paint(ui.painter(), rect.shrink(3.0), icon, fg, bg);
    resp
}

fn cmd_button(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, cmd: Cmd, big: bool) {
    let (icon, label, _) = cmd.meta();
    let enabled = st.can(cmd, s);
    let on = st.is_on(cmd);
    let resp = if big {
        large(ui, icon, label, on, enabled)
    } else {
        small(ui, icon, label, on, enabled, false)
    };
    let ctx = ui.ctx().clone();
    let resp = tooltip(resp, &ctx, cmd);
    if resp.hovered() {
        st.hover_cmd = Some(cmd);
    }
    if resp.clicked() {
        st.run(cmd, &ctx);
    }
}

/// A small button that opens a menu.
fn dropdown(
    ui: &mut egui::Ui,
    icon: &'static str,
    label: &str,
    tip: &str,
    enabled: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let resp = small(ui, Icon::Glyph(icon), label, false, enabled, true).on_hover_text(tip);
    egui::Popup::menu(&resp).show(content);
}

/// A column of controls with the height of three small buttons.
fn column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        add(ui);
    });
}

/// The panels of `tab` that fold into one button, so that the panels fit
/// in `width` (the panels on the right fold first). Before the widths of
/// all panels are known, none folds.
fn folded_panels(st: &AppState, tab: RibbonTab, width: f32) -> Vec<bool> {
    let n = tab.panels_in(st.settings.simple_mode).len();
    let w = |i: usize, f: bool| st.ribbon_widths.get(&(tab, i, f)).copied();
    let mut folded = vec![false; n];
    let Some(mut total) = (0..n).map(|i| w(i, false)).sum::<Option<f32>>() else {
        return folded;
    };
    for i in (0..n).rev() {
        if total <= width {
            break;
        }
        total -= w(i, false).unwrap_or(0.0) - w(i, true).unwrap_or(FOLDED_W);
        folded[i] = true;
    }
    folded
}

/// The width of a folded panel before it is measured.
const FOLDED_W: f32 = LARGE.x + 13.0;

/// A command as the help shows it.
pub struct HelpCommand {
    pub label: &'static str,
    pub tip: &'static str,
    pub keys: Option<String>,
}

/// A ribbon panel as the help shows it.
pub struct HelpPanel {
    pub title: &'static str,
    pub commands: Vec<HelpCommand>,
}

/// The panels and commands of a ribbon tab, for its help topic.
pub fn help_panels(tab: RibbonTab, ctx: &egui::Context) -> Vec<HelpPanel> {
    tab.panels()
        .iter()
        .map(|p| HelpPanel {
            title: tr(p.title),
            commands: p
                .items
                .iter()
                .flat_map(|item| match item {
                    Large(c) => std::slice::from_ref(c),
                    Small(cs) | Custom(_, cs) => cs,
                })
                .map(|&cmd| {
                    let (_, label, tip) = cmd.meta();
                    HelpCommand {
                        label,
                        tip,
                        keys: cmd.keys(ctx),
                    }
                })
                .collect(),
        })
        .collect()
}

/// The ribbon tab that has the command.
pub fn tab_of(cmd: Cmd) -> Option<RibbonTab> {
    RibbonTab::ALL.into_iter().find(|t| {
        t.panels().iter().any(|p| {
            p.items.iter().any(|item| match item {
                Large(c) => *c == cmd,
                Small(cs) | Custom(_, cs) => cs.contains(&cmd),
            })
        })
    })
}

/// The icon of a panel: the icon of its first command.
fn panel_icon(p: &Panel) -> Icon {
    p.items
        .iter()
        .find_map(|item| match item {
            Large(c) => Some(c.meta().0),
            Small(cs) => cs.first().map(|c| c.meta().0),
            Custom(..) => None,
        })
        .unwrap_or(Icon::Glyph(ph::DOTS_THREE))
}

/// The label of a folded panel: its title and a caret.
fn folded_label(p: &Panel) -> String {
    format!("{} {}", tr(p.title), ph::CARET_DOWN)
}

/// A panel that does not fit: one large button with the panel title; a
/// click shows the panel under it.
fn folded_panel(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, p: &Panel) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let resp = ui
            .horizontal_top(|ui| {
                ui.set_min_height(LARGE.y.max(CONTENT_H));
                large(ui, panel_icon(p), &folded_label(p), false, true)
            })
            .inner
            .on_hover_text(crate::i18n::fill(
                "The {} tools (the window is too narrow to show them all)",
                &[&tr(p.title)],
            ));
        egui::Popup::menu(&resp)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .layout(egui::Layout::left_to_right(egui::Align::Min))
            .show(|ui| panel(ui, st, s, p));
        let color = ui.visuals().weak_text_color();
        let galley = ui
            .painter()
            .layout_no_wrap(tr(p.title).to_string(), FontId::proportional(11.0), color);
        let (r, _) = ui.allocate_exact_size(vec2(LARGE.x.max(galley.size().x), TITLE_H), Sense::hover());
        ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
    });
    let (r, _) = ui.allocate_exact_size(vec2(11.0, LARGE.y + TITLE_H), Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().vline(r.center().x, r.y_range().shrink(3.0), stroke);
}

fn panel(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, p: &Panel) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let content = ui
            .horizontal_top(|ui| {
                ui.set_min_height(LARGE.y.max(CONTENT_H));
                for item in p.items {
                    match item {
                        Large(cmd) => cmd_button(ui, st, s, *cmd, true),
                        Small(cmds) => column(ui, |ui| {
                            for cmd in *cmds {
                                cmd_button(ui, st, s, *cmd, false);
                            }
                        }),
                        Custom(add, _) => column(ui, |ui| add(ui, st, s)),
                    }
                }
            })
            .response
            .rect;
        let color = ui.visuals().weak_text_color();
        let galley = ui
            .painter()
            .layout_no_wrap(tr(p.title).to_string(), FontId::proportional(11.0), color);
        let (r, _) = ui.allocate_exact_size(vec2(content.width().max(galley.size().x), TITLE_H), Sense::hover());
        ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
    });
    // A line between the panels.
    let (r, _) = ui.allocate_exact_size(vec2(11.0, LARGE.y + TITLE_H), Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().vline(r.center().x, r.y_range().shrink(3.0), stroke);
}

// ---- Custom controls ----

fn select_by(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    dropdown(
        ui,
        ph::PALETTE,
        tr("By Color"),
        tr("Select all objects of a color."),
        true,
        |ui| {
            let threads = st.doc.design().threads.clone();
            for t in threads {
                if ui.button(&t.name).clicked() {
                    st.select_by_thread(t.id);
                }
            }
        },
    );
    dropdown(
        ui,
        ph::SHAPES,
        tr("By Type"),
        tr("Select all objects of a stitch type."),
        true,
        |ui| {
            for (label, name) in [
                (tr("Running lines"), "Running"),
                (tr("Satin columns"), "Satin"),
                (tr("Fills"), "Fill"),
            ] {
                if ui.button(label).clicked() {
                    st.select_by_type(name);
                }
            }
        },
    );
}

/// The outline type of the selected lines and the column type of the
/// selected columns, each one undo step.
fn stitch_types(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    use emb_model::ColumnPattern;
    dropdown(
        ui,
        ph::WAVE_TRIANGLE,
        tr("Outline Type"),
        tr("Sew the selected lines as a plain line, or as a column of a stitch type along the line."),
        s.lines > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            let mut pick = None;
            if ui
                .button(tr("Line"))
                .on_hover_text(tr(crate::words::OUTLINE_HELP))
                .clicked()
            {
                pick = Some(None);
            }
            for p in ColumnPattern::ALL {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::column_help(&p))
                    .clicked()
                {
                    pick = Some(Some(p));
                }
            }
            if let Some(p) = pick {
                let n = crate::stitch_types::set_outline(st, &ids, p);
                st.message = crate::i18n::fill(
                    "Outline type of {} lines: {}.",
                    &[&n, &(p.map_or(tr("Line"), |p| tr(p.name())))],
                );
            }
        },
    );
    dropdown(
        ui,
        ph::SPIRAL,
        tr("Fill Type"),
        tr("The stitch type of the selected fills."),
        s.fills > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            for p in emb_model::FillPattern::all() {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::fill_help(&p))
                    .clicked()
                {
                    let n = crate::stitch_types::set_fill(st, &ids, p.clone());
                    st.message = crate::i18n::fill("Fill type of {} fills: {}.", &[&n, &tr(p.name())]);
                }
            }
        },
    );
    dropdown(
        ui,
        ph::WAVE_SQUARE,
        tr("Column Type"),
        tr("The stitch type of the selected satin columns and of lines with a width."),
        s.columns > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            for p in ColumnPattern::ALL {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::column_help(&p))
                    .clicked()
                {
                    let n = crate::stitch_types::set_column(st, &ids, p);
                    st.message = crate::i18n::fill("Column type of {} objects: {}.", &[&n, &tr(p.name())]);
                }
            }
        },
    );
}

/// Keep originals and the overlap amount of the shaping commands.
fn shaping_options(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    ui.vertical(|ui| {
        ui.checkbox(&mut st.shaping.keep_originals, tr("Keep originals"))
            .on_hover_text(tr("The selected objects stay; the result comes after them."));
        ui.checkbox(&mut st.shaping.linked, tr("Keep linked")).on_hover_text(tr(
            "Flatten, Divide and Remove overlaps are cut again when their objects move. Unlink ends it.",
        ));
        ui.horizontal(|ui| {
            ui.label(tr("Overlap")).on_hover_text(tr(
                "How far a cut piece goes under the shape that cut it, so no fabric shows between the pieces. \
                 Also for Remove overlaps and the knife.",
            ));
            ui.add(
                egui::DragValue::new(&mut st.shaping.overlap_mm)
                    .range(0.0..=5.0)
                    .clamp_existing_to_range(false)
                    .speed(0.05)
                    .fixed_decimals(1)
                    .suffix(" mm"),
            );
        });
    });
}

fn rotate_skew(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let (mut sx, mut sy, mut rot) = st.arrange_values;
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut rot)
                .range(-360.0..=360.0)
                .clamp_existing_to_range(false)
                .suffix("°"),
        );
        let r = small(
            ui,
            Icon::Glyph(ph::ARROW_CLOCKWISE),
            tr("Rotate"),
            false,
            s.n > 0,
            false,
        )
        .on_hover_text(tr("Turn the selection by this angle (clockwise)."));
        if r.clicked() {
            st.rotate_selection(rot);
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut sx)
                .range(-80.0..=80.0)
                .clamp_existing_to_range(false)
                .suffix("° X"),
        );
        ui.add(
            egui::DragValue::new(&mut sy)
                .range(-80.0..=80.0)
                .clamp_existing_to_range(false)
                .suffix("° Y"),
        );
    });
    let r = small(ui, Icon::Glyph(ph::PARALLELOGRAM), tr("Skew"), false, s.n > 0, false)
        .on_hover_text(tr("Slant the selection by these angles."));
    if r.clicked() {
        st.skew_selection(sx, sy);
    }
    st.arrange_values = (sx, sy, rot);
}

/// Show all and the Show colors menu, in one column.
fn show_colors(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::ShowAll, false);
    dropdown(
        ui,
        ph::PALETTE,
        tr("Show Colors"),
        tr("Show or hide each color."),
        true,
        |ui| {
            let threads = st.doc.design().threads.clone();
            for t in threads {
                let mut on = st.view_filter.threads.as_ref().is_none_or(|s| s.contains(&t.id));
                if ui.checkbox(&mut on, &t.name).changed() {
                    let all: std::collections::BTreeSet<_> = st.doc.design().threads.iter().map(|t| t.id).collect();
                    let set = st.view_filter.threads.get_or_insert(all.clone());
                    if on {
                        set.insert(t.id);
                    } else {
                        set.remove(&t.id);
                    }
                    if *set == all {
                        st.view_filter.threads = None;
                    }
                }
            }
        },
    );
}

/// Center at stitch and travel by function, in one column.
fn cursor_tools(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::CenterAtCursor, false);
    function_pick(ui, st, s);
}

/// Remove guides, dark mode and the Panels menu, in one column.
fn workspace_more(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::RemoveGuides, false);
    cmd_button(ui, st, s, Cmd::Toggle(Flag::DarkMode), false);
    panels_menu(ui, st, s);
}

/// Artwork, Stitches and the Thread look menu (flat or 3D, the light, the
/// fabric texture), in one column.
fn look_column(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Artwork), false);
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Stitches), false);
    let (icon, label, tip) = Cmd::Toggle(Flag::ThreadLook).meta();
    let resp = small(ui, icon, label, st.show_thread_look, true, true).on_hover_text(tip);
    egui::Popup::menu(&resp).show(|ui| crate::viewtools::look_ui(ui, st));
}

/// Overview, the Views menu (split view, new view, other design) and
/// Previous view, in one column.
fn views_column(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Overview), false);
    let ctx = ui.ctx().clone();
    let resp = small(
        ui,
        Icon::Glyph(ph::COLUMNS),
        tr("Views"),
        st.show_split || st.other_design.is_some(),
        true,
        true,
    )
    .on_hover_text(tr(
        "More views: the split view, new views of the design, and another design next to it.",
    ));
    egui::Popup::menu(&resp).show(|ui| {
        for cmd in [Cmd::Toggle(Flag::SplitView), Cmd::NewView, Cmd::OtherDesign] {
            let (_, label, tip) = cmd.meta();
            let text = if st.is_on(cmd) {
                format!("✔ {label}")
            } else {
                label.to_string()
            };
            let r = ui
                .add_enabled(st.can(cmd, s), egui::Button::new(text))
                .on_hover_text(tip);
            if r.hovered() {
                st.hover_cmd = Some(cmd);
            }
            if r.clicked() {
                st.run(cmd, &ctx);
                ui.close();
            }
        }
    });
    cmd_button(ui, st, s, Cmd::PreviousView, false);
}

/// View > Panels: open and close each panel.
fn panels_menu(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    dropdown(
        ui,
        ph::SIDEBAR_SIMPLE,
        tr("Panels"),
        tr("Open and close the panels."),
        true,
        |ui| {
            for (tab, name) in crate::viewtools::PANELS {
                let mut on = st.panel_shown(tab);
                if ui.checkbox(&mut on, tr(name)).changed() {
                    st.set_panel_shown(tab, on);
                }
            }
        },
    );
}

/// The kind of function that travel by function goes to.
fn function_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let tip = crate::i18n::fill(
        "Travel by function: the functions that Shift+Page Up and Shift+Page Down go to. Now: {}.",
        &[&st.function_travel.label()],
    );
    dropdown(ui, ph::FLOW_ARROW, tr("Functions"), &tip, true, |ui| {
        for f in crate::viewtools::FunctionTravel::ALL {
            if ui.selectable_label(st.function_travel == f, f.label()).clicked() {
                st.function_travel = f;
            }
        }
    });
}

/// Design > Favorites: apply a favorite style to the selection.
fn favorites(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let names: Vec<String> = st.settings.favorite_styles.iter().map(|f| f.name.clone()).collect();
    let mut apply = None;
    dropdown(
        ui,
        ph::STAR,
        tr("Favorites"),
        tr("Your favorite styles, for all designs. Make a style a favorite with its star in the Styles panel."),
        true,
        |ui| {
            if names.is_empty() {
                ui.weak(tr("No favorites. Click the star of a style in the Styles panel."));
            }
            for (i, name) in names.iter().enumerate() {
                if ui
                    .add_enabled(s.n > 0, egui::Button::new(format!("{}  {name}", i + 1)))
                    .on_disabled_hover_text(tr("Select objects first"))
                    .clicked()
                {
                    apply = Some(i);
                    ui.close();
                }
            }
        },
    );
    if let Some(i) = apply {
        st.apply_favorite(i);
    }
}

fn chart_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    ui.label(tr("Chart"));
    let shown = st.chart().map(|c| c.name.clone()).unwrap_or_default();
    let names: Vec<String> = st.charts.iter().map(|c| c.name.clone()).collect();
    egui::ComboBox::from_id_salt("ribbon_chart")
        .selected_text(shown)
        .width(150.0)
        .show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut st.chart_index, i, n);
            }
        });
}

fn colorway_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let d = st.doc.design();
    let names: Vec<String> = d.colorways.iter().map(|c| c.name.clone()).collect();
    let current = d.colorway;
    ui.label(tr("Current"));
    let shown = names
        .get(current)
        .cloned()
        .unwrap_or_else(|| tr("Design colors").into());
    egui::ComboBox::from_id_salt("ribbon_colorway")
        .selected_text(shown)
        .width(150.0)
        .show_ui(ui, |ui| {
            if names.is_empty() {
                ui.weak(tr("Only the design colors. Click New colorway to add one."));
            }
            for (i, n) in names.iter().enumerate() {
                if ui.selectable_label(i == current, n).clicked() {
                    st.switch_colorway(i);
                }
            }
        });
}

fn background_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let bg = st.doc.design().background;
    ui.label(tr("Background"));
    let mut c = bg.map(crate::canvas::color32).unwrap_or(Color32::WHITE);
    let r = egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque)
        .on_hover_text(tr("The fabric color inside the hoop. It changes with the colorway."));
    if r.changed() {
        let rgb = emb_model::Rgb(c.r(), c.g(), c.b());
        st.doc
            .edit_merged("Background color", emb_model::MergeKey::new("background"), |d| {
                d.background = Some(rgb)
            });
    }
    let enabled = bg.is_some();
    let r = small(ui, Icon::Glyph(ph::X), tr("No Background"), false, enabled, false)
        .on_hover_text(tr("No fabric color: the canvas shows the design on white."));
    if r.clicked() {
        st.doc.edit("Background color", |d| d.background = None);
    }
}

/// The machine and hoop of the design.
/// The zoom factor list.
fn zoom_factor(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    ui.vertical(|ui| {
        ui.label(tr("Zoom"));
        crate::corners::zoom_combo(ui, st);
        ui.horizontal(|ui| {
            for (cmd, text) in [(Cmd::ZoomTwice(false), "½×"), (Cmd::ZoomTwice(true), "2×")] {
                let (_, _, tip) = cmd.meta();
                if ui.small_button(text).on_hover_text(tip).clicked() && st.can(cmd, s) {
                    st.run(cmd, &ui.ctx().clone());
                }
            }
        });
    });
}

/// The fabric of the design, and a list to apply a style to the selection.
fn style_picker(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let styles: Vec<String> = st.doc.design().styles.iter().map(|x| x.name.clone()).collect();
    let mut apply = None;
    egui::Grid::new("style_picker")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            ui.label(tr("Fabric"));
            ui.label(st.fabric_name());
            ui.end_row();
            ui.label(tr("Style"));
            ui.add_enabled_ui(s.n > 0 && !styles.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("apply_style")
                    .selected_text(if styles.is_empty() {
                        tr("No styles")
                    } else {
                        tr("Apply to selection")
                    })
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in styles.iter().enumerate() {
                            if ui.selectable_label(false, name).clicked() {
                                apply = Some(i);
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr("Give the selected objects the settings of a style."));
            });
            ui.end_row();
        });
    if let Some(i) = apply {
        st.apply_style(i);
    }
}

fn machine_hoop(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let design = st.doc.design();
    let machine = design.machine.clone();
    let hoop = design.hoop.selection.clone();
    let machine_name = machine
        .as_deref()
        .and_then(|id| st.machines.get(id))
        .map(|m| m.name.clone())
        .unwrap_or_else(|| tr("None").into());
    let hoop_name = match &hoop {
        HoopSelection::OpenField => tr("Open field").to_string(),
        HoopSelection::Preset { id } => st.hoops.get(id).map(|h| h.name.clone()).unwrap_or_else(|| id.clone()),
    };
    let (mut new_machine, mut new_hoop) = (None, None);
    egui::Grid::new("machine_hoop")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            ui.label(tr("Machine"));
            egui::ComboBox::from_id_salt("machine")
                .selected_text(machine_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for m in &st.machines.machines {
                        if ui
                            .selectable_label(machine.as_deref() == Some(m.id.as_str()), &m.name)
                            .clicked()
                        {
                            new_machine = Some(m.id.clone());
                        }
                    }
                });
            ui.end_row();
            ui.label(tr("Hoop"));
            egui::ComboBox::from_id_salt("hoop")
                .selected_text(hoop_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(hoop == HoopSelection::OpenField, tr("Open field"))
                        .clicked()
                    {
                        new_hoop = Some(HoopSelection::OpenField);
                    }
                    // My hoops first.
                    let mine = &st.settings.my_hoops;
                    let (first, rest): (Vec<_>, Vec<_>) = st.hoops.hoops.iter().partition(|h| mine.contains(&h.id));
                    for (k, list) in [first, rest].into_iter().enumerate() {
                        if list.is_empty() {
                            continue;
                        }
                        if !mine.is_empty() {
                            ui.separator();
                            ui.weak(if k == 0 { tr("My Hoops") } else { tr("Other hoops") });
                        }
                        for h in list {
                            let sel = HoopSelection::Preset { id: h.id.clone() };
                            let mut text = format!("{}  ({} × {} mm)", h.name, h.width_mm, h.height_mm);
                            if !st.hoop_fits_machine(&h.id) {
                                text.push_str(&format!("  ⚠ {}", tr("not for this machine")));
                            }
                            if ui.selectable_label(hoop == sel, text).clicked() {
                                new_hoop = Some(sel);
                            }
                        }
                    }
                });
            ui.end_row();
        });
    if let Some(m) = new_machine {
        st.doc.edit("Change machine", |d| d.machine = Some(m));
    }
    if let Some(sel) = new_hoop {
        st.doc.edit("Change hoop", |d| {
            // In a hoop, the hoop center goes to the center of the design. Object coordinates do not change.
            if d.hoop.selection == HoopSelection::OpenField
                && let Some(e) = d.extents()
            {
                d.hoop.center = e.center();
            }
            d.hoop.selection = sel;
        });
        st.fit_requested = true;
    }
}

// ---- The ribbon ----

/// File > Recent designs.
fn recent_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let recent = st.settings.recent.clone();
    ui.add_enabled_ui(!recent.is_empty(), |ui| {
        ui.menu_button(
            format!("{}  {}", ph::CLOCK_COUNTER_CLOCKWISE, tr("Recent designs")),
            |ui| {
                for path in &recent {
                    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
                    if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                        st.open_recent(path);
                        ui.close();
                    }
                }
            },
        );
    });
}

/// The File menu.
fn file_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let ctx = ui.ctx().clone();
    let s = SelInfo::of(st);
    let item = |ui: &mut egui::Ui, st: &mut AppState, cmd: Cmd, text: &str| {
        let (icon, _, tip) = cmd.meta();
        let glyph = match icon {
            Icon::Glyph(g) => g,
            Icon::Drawn(_) => "",
        };
        let b = egui::Button::new(format!("{glyph}  {text}")).shortcut_text(cmd.keys(&ctx).unwrap_or_default());
        if ui.add_enabled(st.can(cmd, &s), b).on_hover_text(tip).clicked() {
            st.run(cmd, &ctx);
        }
    };
    item(ui, st, Cmd::New, tr("New"));
    item(ui, st, Cmd::NewFromTemplate, tr("New from Template…"));
    item(ui, st, Cmd::Open, tr("Open…"));
    recent_menu(ui, st);
    item(ui, st, Cmd::Library, tr("Design Library"));
    item(ui, st, Cmd::Save, tr("Save"));
    item(ui, st, Cmd::SaveAs, tr("Save As…"));
    item(ui, st, Cmd::SaveAsTemplate, tr("Save as Template…"));
    item(ui, st, Cmd::Close, tr("Close"));
    item(ui, st, Cmd::CloseAll, tr("Close All"));
    ui.separator();
    item(ui, st, Cmd::LoadPicture, tr("Load Backdrop Picture…"));
    item(ui, st, Cmd::ImportSvg, tr("Import Artwork (SVG)…"));
    item(ui, st, Cmd::ImportSvgLines, tr("Import Artwork as Lines (SVG)…"));
    item(ui, st, Cmd::ImportSvgVectors, tr("Import Artwork as Vectors (SVG)…"));
    item(ui, st, Cmd::InsertMachineFile, tr("Insert Machine File…"));
    item(ui, st, Cmd::Export, tr("Export Machine File…"));
    item(ui, st, Cmd::SendToUsb, tr("Send to USB"));
    item(ui, st, Cmd::SendToUsbChoose, tr("Send to USB Stick…"));
    item(ui, st, Cmd::SendByEmail, tr("Send by Email…"));
    item(ui, st, Cmd::ExportSvg, tr("Export Design Picture (SVG)…"));
    item(ui, st, Cmd::ExportOutlines, tr("Export Outlines (SVG, EMF)…"));
    item(ui, st, Cmd::CutterFile, tr("Appliqué Cutter File (SVG, DXF)…"));
    item(ui, st, Cmd::OutputDesign, tr("Output Design…"));
    item(ui, st, Cmd::PrintPreview, tr("Print Preview…"));
    item(ui, st, Cmd::BatchOutput, tr("Batch Output…"));
    ui.separator();
    item(ui, st, Cmd::Assets, tr("My Assets…"));
    item(ui, st, Cmd::Options, tr("Options…"));
    item(ui, st, Cmd::About, tr("About…"));
    ui.separator();
    if ui.button(crate::i18n::fill("{}  Exit", &[&(ph::SIGN_OUT)])).clicked() {
        st.request(Pending::Quit);
    }
}

/// Shows the ribbon: the tab row and the panels of the current tab.
pub fn show(ui: &mut egui::Ui, st: &mut AppState) {
    #[cfg(test)]
    RECTS.with(|r| r.borrow_mut().clear());
    let s = SelInfo::of(st);
    let ctx = ui.ctx().clone();
    // The command under the pointer, for F1 (set again while the buttons draw).
    st.hover_cmd = None;
    let mut settings = st.settings.clone();
    let (tab_before, collapsed_before) = (settings.ribbon_tab, settings.ribbon_collapsed);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let accent = ui.visuals().selection.bg_fill;
        let file = egui::Button::new(RichText::new(tr("File")).strong().color(Color32::WHITE))
            .fill(accent)
            .min_size(vec2(52.0, SMALL_H));
        let resp = ui.add(file);
        note_rect("File", resp.rect);
        egui::Popup::menu(&resp).show(|ui| file_menu(ui, st));
        ui.add_space(6.0);
        for cmd in QUICK {
            let (icon, label, _) = cmd.meta();
            let r = icon_button(ui, icon, label, st.can(cmd, &s));
            if tooltip(r, &ctx, cmd).clicked() {
                st.run(cmd, &ctx);
            }
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        let sketching = st.sketch.is_some();
        if !sketching && settings.ribbon_tab == RibbonTab::Sketch {
            settings.ribbon_tab = RibbonTab::Digitize;
        }
        let mut tabs = RibbonTab::visible(settings.simple_mode).to_vec();
        if sketching {
            tabs.push(RibbonTab::Sketch);
        }
        for tab in tabs {
            let active = settings.ribbon_tab == tab && !settings.ribbon_collapsed;
            let text = RichText::new(tab.label()).size(13.5);
            let text = if active { text.strong() } else { text };
            let r = ui.add(egui::Button::selectable(active, text).min_size(vec2(64.0, SMALL_H)));
            note_rect(tab.label(), r.rect);
            if active {
                let y = r.rect.bottom() - 1.0;
                let stroke = Stroke::new(2.0, ui.visuals().selection.stroke.color);
                ui.painter().hline(r.rect.x_range().shrink(6.0), y, stroke);
            }
            if r.double_clicked() {
                settings.ribbon_tab = tab;
                settings.ribbon_collapsed = !settings.ribbon_collapsed;
            } else if r.clicked() {
                settings.ribbon_tab = tab;
                settings.ribbon_collapsed = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (glyph, tip) = if settings.ribbon_collapsed {
                (ph::CARET_DOWN, tr("Show the ribbon"))
            } else {
                (ph::CARET_UP, tr("Hide the ribbon (or double-click a tab)"))
            };
            if icon_button(ui, Icon::Glyph(glyph), tr("Collapse"), true)
                .on_hover_text(tip)
                .clicked()
            {
                settings.ribbon_collapsed = !settings.ribbon_collapsed;
            }
        });
    });
    if !settings.ribbon_collapsed {
        ui.add_space(2.0);
        // The width of the window, outside the scroll area (inside it the
        // width has no end).
        let width = ui.available_width();
        egui::ScrollArea::horizontal()
            .id_salt("ribbon_panels")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                let tab = settings.ribbon_tab;
                let folded = folded_panels(st, tab, width);
                let _row = ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (i, p) in tab.panels_in(settings.simple_mode).iter().enumerate() {
                        let x0 = ui.cursor().min.x;
                        if folded[i] {
                            folded_panel(ui, st, &s, p);
                        } else {
                            panel(ui, st, &s, p);
                        }
                        let w = ui.cursor().min.x - x0;
                        if st.ribbon_widths.insert((tab, i, folded[i]), w) != Some(w) {
                            ui.ctx().request_repaint();
                        }
                    }
                });
                #[cfg(test)]
                PANELS_W.with(|w| w.set(_row.response.rect.width()));
            });
    }
    // The ribbon state is a user setting; the folded ribbon stays after a
    // restart, the tab does not (the program starts on Digitize).
    // Only a change of the tab here: a command of the ribbon (for example
    // Start Sketch) can change the tab too.
    let mut new = st.settings.clone();
    if settings.ribbon_tab != tab_before {
        new.ribbon_tab = settings.ribbon_tab;
    }
    if settings.ribbon_collapsed != collapsed_before {
        new.ribbon_collapsed = settings.ribbon_collapsed;
    }
    st.set_settings(new);
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn all_cmds() -> Vec<Cmd> {
        let mut v = Vec::new();
        for tab in RibbonTab::ALL {
            for p in tab.panels() {
                for item in p.items {
                    match item {
                        Large(c) => v.push(*c),
                        Small(cs) | Custom(_, cs) => v.extend_from_slice(cs),
                    }
                }
            }
        }
        v.extend(QUICK);
        v
    }

    struct Rib {
        ctx: egui::Context,
        st: AppState,
        /// A very wide screen (no panel folds), else `width`.
        wide: bool,
        width: f32,
    }

    impl Rib {
        fn new() -> Self {
            let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
            st.add_sample_objects();
            let ctx = egui::Context::default();
            icons::install(&ctx);
            Rib {
                ctx,
                st,
                wide: true,
                width: 1600.0,
            }
        }

        fn frame(&mut self, events: Vec<Event>) {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    vec2(if self.wide { 4000.0 } else { self.width }, 900.0),
                )),
                events,
                ..Default::default()
            };
            let st = &mut self.st;
            let mut out = self.ctx.run_ui(input, |ui| {
                egui::Panel::top("ribbon").show(ui, |ui| show(ui, st));
            });
            out.textures_delta.clear();
        }

        fn rect(&self, label: &str) -> Rect {
            RECTS.with(|r| {
                r.borrow()
                    .iter()
                    .find(|(l, _)| l == label)
                    .map(|(_, r)| *r)
                    .unwrap_or_else(|| panic!("no button {label}"))
            })
        }

        fn click(&mut self, label: &str) {
            let p = self.rect(label).center();
            let ev = |pressed| Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![Event::PointerMoved(p)]);
            self.frame(vec![ev(true)]);
            self.frame(vec![ev(false)]);
        }
    }

    /// No ribbon tab needs the scroll bar in a window 1366 points wide (a
    /// common laptop screen): the panels that do not fit fold into one
    /// button each, and a click on it shows the panel.
    #[test]
    fn simple_mode_has_fewer_tabs_and_large_buttons() {
        let mut r = Rib::new();
        r.st.save_settings = false;
        r.st.settings.ribbon_tab = RibbonTab::Artwork;
        r.frame(vec![]);
        r.click("Help");
        r.frame(vec![]);
        r.click("Simple Mode");
        assert!(r.st.settings.simple_mode);
        // A tab that simple mode does not have: the first tab shows.
        r.st.settings.ribbon_tab = RibbonTab::Artwork;
        r.st.run(Cmd::Toggle(Flag::SimpleMode), &r.ctx.clone());
        r.st.run(Cmd::Toggle(Flag::SimpleMode), &r.ctx.clone());
        assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Digitize);
        r.wide = false;
        r.width = 1366.0;
        for &tab in RibbonTab::visible(true) {
            r.st.settings.ribbon_tab = tab;
            for _ in 0..3 {
                r.frame(vec![]);
            }
            let w = PANELS_W.with(|w| w.get());
            assert!(w <= 1366.0, "the simple {} tab is {w} points wide", tab.label());
            // Only large buttons, and each is on the full ribbon too (for the help).
            for p in tab.panels_in(true) {
                for item in p.items {
                    match item {
                        Large(c) => assert!(tab_of(*c).is_some(), "{c:?}"),
                        // The Help tab is the same in both modes.
                        Small(cs) if tab != RibbonTab::Help => panic!("small buttons in simple mode: {cs:?}"),
                        Small(_) => {}
                        Custom(..) => {}
                    }
                }
            }
        }
        // The tab row has the simple tabs only.
        r.frame(vec![]);
        assert!(RECTS.with(|x| x.borrow().iter().all(|(l, _)| l != "Artwork")));
        r.click("View");
        r.frame(vec![]);
        r.click("Simple Mode");
        assert!(!r.st.settings.simple_mode);
    }

    /// The names that the screen reader gets in one frame.
    fn reader_names(r: &mut Rib) -> Vec<String> {
        r.ctx.enable_accesskit();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(4000.0, 900.0))),
            ..Default::default()
        };
        let st = &mut r.st;
        let mut out = r.ctx.run_ui(input, |ui| {
            egui::Panel::top("ribbon").show(ui, |ui| show(ui, st));
            egui::CentralPanel::default().show(ui, |ui| crate::canvas::show(ui, st));
        });
        out.textures_delta.clear();
        let update = out.platform_output.accesskit_update.expect("the screen reader tree");
        update
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().map(str::to_string))
            .collect()
    }

    #[test]
    fn a_screen_reader_reads_the_ribbon_and_the_canvas() {
        let mut r = Rib::new();
        r.frame(vec![]);
        let names = reader_names(&mut r);
        // The tabs, the quick buttons and the buttons of the Digitize tab,
        // which the program paints itself.
        for want in ["Digitize", "Lettering", "Undo", "Save"] {
            assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
        }
        for (cmd, tab, _) in all_commands() {
            if tab == RibbonTab::Digitize {
                let label = cmd.meta().1;
                assert!(names.iter().any(|n| n == label), "{label}");
            }
        }
        assert!(
            names.iter().any(|n| n.starts_with("Design canvas:")),
            "the canvas has a name"
        );
    }

    #[test]
    fn f6_puts_the_keyboard_focus_on_the_ribbon() {
        let mut r = Rib::new();
        r.frame(vec![]);
        assert!(focus_ribbon(&r.ctx));
        r.frame(vec![]);
        let first = r.ctx.memory(|m| m.focused()).expect("a ribbon button has the focus");
        // Tab goes to the next button.
        let tab = |pressed| Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        r.frame(vec![tab(true), tab(false)]);
        r.frame(vec![]);
        let next = r.ctx.memory(|m| m.focused()).expect("the focus stays in the ribbon");
        assert_ne!(first, next);
    }

    #[test]
    fn every_tab_fits_a_laptop_screen_in_every_language() {
        // Longer German labels fold more panels, but every tab still fits.
        for code in ["de", "es", "ja"] {
            crate::i18n::set_language(code);
            let mut r = Rib::new();
            r.wide = false;
            r.width = 1366.0;
            for tab in RibbonTab::ALL {
                r.st.settings.ribbon_tab = tab;
                for _ in 0..3 {
                    r.frame(vec![]);
                }
                let w = PANELS_W.with(|w| w.get());
                assert!(w <= 1366.0, "{code}: the {} tab is {w} points wide", tab.label());
            }
        }
        crate::i18n::set_language("en");
    }

    #[test]
    fn the_sketch_tab_buttons_take_clicks_at_every_width() {
        for width in [1366.0, 1456.0, 1600.0, 1920.0] {
            let mut r = Rib::new();
            r.st.save_settings = false;
            r.wide = false;
            r.width = width;
            r.st.selection.clear();
            r.st.start_sketch();
            for _ in 0..3 {
                r.frame(vec![]);
            }
            let label = tr("Trim");
            if RECTS.with(|x| x.borrow().iter().any(|(l, _)| l == label)) {
                r.click(label);
                assert_eq!(
                    r.st.sketch.as_ref().unwrap().tool,
                    crate::sketch::SketchTool::Trim,
                    "width {width}"
                );
            }
        }
    }

    #[test]
    fn every_tab_fits_a_laptop_screen() {
        let mut r = Rib::new();
        r.wide = false;
        r.width = 1366.0;
        for tab in RibbonTab::ALL {
            r.st.settings.ribbon_tab = tab;
            // The first frame measures the panels; the next ones fold.
            for _ in 0..3 {
                r.frame(vec![]);
            }
            let w = PANELS_W.with(|w| w.get());
            assert!(w <= 1366.0, "the {} tab is {w} points wide", tab.label());
        }
        // A folded panel opens and its buttons work.
        r.st.settings.ribbon_tab = RibbonTab::Artwork;
        for _ in 0..3 {
            r.frame(vec![]);
        }
        let folded = folded_panels(&r.st, RibbonTab::Artwork, 1366.0);
        let k = folded
            .iter()
            .rposition(|f| *f)
            .expect("a panel of the Artwork tab folds");
        r.click(&folded_label(&ARTWORK[k]));
        let (_, label, _) = match ARTWORK[k].items[0] {
            Large(c) => c.meta(),
            Small(cs) => cs[0].meta(),
            Custom(..) => unreachable!(),
        };
        r.frame(vec![]);
        assert!(
            RECTS.with(|x| x.borrow().iter().any(|(l, _)| l == label)),
            "{label} shows in the open panel"
        );
        // A wide window folds nothing.
        assert!(folded_panels(&r.st, RibbonTab::Artwork, 4000.0).iter().all(|f| !f));
    }

    /// Two commands of one tab with the same icon look like one command.
    /// The tutorials share the icon of a tutorial; the 2× zoom buttons show
    /// text, not their icon.
    #[test]
    fn the_commands_of_a_tab_have_different_icons() {
        let mut same = Vec::new();
        for tab in RibbonTab::ALL {
            let mut seen: Vec<(Icon, Cmd)> = Vec::new();
            for (cmd, t, _) in all_commands() {
                if t != tab
                    || seen.iter().any(|(_, c)| *c == cmd)
                    || matches!(cmd, Cmd::HelpTopic(id) if id.starts_with("tutorial-"))
                    || matches!(cmd, Cmd::ZoomTwice(_))
                {
                    continue;
                }
                let icon = cmd.meta().0;
                if let Some((_, other)) = seen.iter().find(|(i, _)| *i == icon) {
                    same.push(format!("{}: {:?} and {:?}", tab.label(), other, cmd));
                }
                seen.push((icon, cmd));
            }
        }
        assert!(same.is_empty(), "{}", same.join("\n"));
    }

    /// The name of a large button (also of a folded panel) has two lines at
    /// most in each language, so it does not go over the panel title (owner
    /// report of 2026-10-26).
    #[test]
    fn large_button_names_have_two_lines_at_most() {
        let ctx = egui::Context::default();
        for code in ["en", "de", "es", "ja"] {
            crate::i18n::set_language(code);
            let mut long = Vec::new();
            let mut out = ctx.run_ui(Default::default(), |ui| {
                let mut labels: Vec<String> = Vec::new();
                for tab in RibbonTab::ALL {
                    for simple in [false, true] {
                        for p in tab.panels_in(simple) {
                            labels.push(folded_label(p));
                            for item in p.items {
                                if let Large(c) = item {
                                    labels.push(c.meta().1.to_string());
                                }
                            }
                        }
                    }
                }
                for label in labels {
                    let (w, size) = large_fit(ui, &label);
                    let rows = ui
                        .painter()
                        .layout(label.clone(), FontId::proportional(size), Color32::WHITE, w - 4.0)
                        .rows
                        .len();
                    if rows > 2 {
                        long.push(format!("{code}: {label} ({rows} lines at {w})"));
                    }
                }
            });
            out.textures_delta.clear();
            assert!(
                long.is_empty(),
                "{}",
                long.join(
                    "
"
                )
            );
        }
        crate::i18n::set_language("en");
    }

    #[test]
    fn every_tool_is_on_the_ribbon() {
        let cmds = all_cmds();
        for tool in Tool::ALL {
            assert!(cmds.contains(&Cmd::Tool(tool)), "{tool:?} is not on the ribbon");
        }
    }

    #[test]
    fn every_command_runs_on_a_selection() {
        let mut r = Rib::new();
        let ctx = r.ctx.clone();
        let (start, steps) = (r.st.doc.design().clone(), r.st.doc.undo_len());
        let mut changed = 0;
        for cmd in all_cmds() {
            if cmd.opens_dialog() {
                continue;
            }
            r.st.select_all();
            let s = SelInfo::of(&r.st);
            if r.st.can(cmd, &s) {
                r.st.run(cmd, &ctx);
            }
            r.st.set_tool(Tool::Select);
            // Each change is one undo step, and undo gives the start design again.
            if r.st.doc.undo_len() > steps {
                changed += 1;
                assert_eq!(r.st.doc.undo_len(), steps + 1, "{cmd:?}");
                r.st.undo();
                assert!(r.st.doc.design().same_content(&start), "{cmd:?}");
            }
        }
        assert!(changed > 20, "only {changed} commands changed the design");
    }

    #[test]
    fn every_tab_draws() {
        let mut r = Rib::new();
        r.st.select_all();
        for tab in RibbonTab::ALL {
            // The Sketch tab shows only in sketch mode.
            if tab == RibbonTab::Sketch {
                r.st.start_sketch();
            }
            r.st.settings.ribbon_tab = tab;
            r.frame(vec![]);
            r.frame(vec![]);
            let (_, first, _) = match tab.panels()[0].items[0] {
                Large(c) => c.meta(),
                Small(cs) => cs[0].meta(),
                Custom(..) => continue,
            };
            r.rect(first);
        }
    }

    #[test]
    fn click_a_tab_and_a_tool() {
        let mut r = Rib::new();
        r.frame(vec![]);
        r.click("Arrange");
        assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Arrange);
        r.click("Digitize");
        r.click("Ellipse");
        assert_eq!(r.st.tool, Tool::Ellipse);
        // A disabled command does nothing: no selection, so no Knife.
        r.st.selection.clear();
        r.click("Edit");
        r.click("Knife");
        assert_eq!(r.st.tool, Tool::Ellipse);
    }

    #[test]
    fn view_buttons_toggle() {
        let mut r = Rib::new();
        r.frame(vec![]);
        r.click("View");
        let grid = r.st.settings.show_grid;
        r.click("Grid");
        assert_eq!(r.st.settings.show_grid, !grid);
        r.click("Stitch Cursor");
        assert!(r.st.cursor.is_some());
        r.click("Stitch Cursor");
        assert!(r.st.cursor.is_none());
    }

    #[test]
    fn collapse_and_expand() {
        let mut r = Rib::new();
        r.frame(vec![]);
        r.click("Collapse");
        assert!(r.st.settings.ribbon_collapsed);
        // The panels are gone; a click on a tab brings them back.
        assert!(RECTS.with(|x| x.borrow().iter().all(|(l, _)| l != "Select All")));
        r.click("Edit");
        assert!(!r.st.settings.ribbon_collapsed);
        assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Edit);
        r.frame(vec![]);
        r.rect("Select All");
    }

    #[test]
    fn undo_from_the_quick_access_buttons() {
        let mut r = Rib::new();
        r.frame(vec![]);
        let n = r.st.doc.design().objects().len();
        r.click("Undo");
        assert_eq!(r.st.doc.design().objects().len(), 0, "sample objects are one undo step");
        r.click("Redo");
        assert_eq!(r.st.doc.design().objects().len(), n);
    }
}
