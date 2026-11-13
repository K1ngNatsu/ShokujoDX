//! The commands of the ribbon: `Cmd`, `Flag` and `SketchCmd`, with the
//! icon, the name and the tooltip of each (`Cmd::meta`) and their keys.

use super::*;

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
pub(super) fn sketch_meta(c: SketchCmd) -> (Icon, &'static str, &'static str) {
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
    CopyProperties,
    Envelope,
    RepeatLast,
    CloseGaps,
    CleanUp,
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
    /// One appliqué object of the selected areas, with a selected border as
    /// its cover (ADR 0087).
    AppliqueFromSelection,
    /// Put back the objects that an appliqué from a selection replaced.
    BackToOriginal,
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
        Tool::ConstructionLine => g(ph::LINE_SEGMENT),
        Tool::ConstructionPoint => g(ph::MAP_PIN),
        Tool::SmartFill => g(ph::PAINT_BUCKET),
        Tool::TrimSegment => g(ph::ERASER),
        Tool::SmartDraw => g(ph::SCRIBBLE_LOOP),
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
                    tr(
                        "Points snap to the nodes, midpoints, centers and edges of other objects, and line up with the points that you placed.",
                    ),
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
                tr(
                    "Buttonholes and eyelets in seven shapes and in standard sizes: one, or a string along the selected line. With buttonholes selected, the window changes them.",
                ),
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
            Cmd::CopyProperties => (
                g(ph::EYEDROPPER),
                tr("Copy Properties"),
                tr(
                    "Click an object: the selected objects get its color, and its stitch settings where they \
                     are of the same kind.",
                ),
            ),
            Cmd::Envelope => (
                g(ph::FLAG_BANNER),
                tr("Envelope…"),
                tr(
                    "Bend the selected objects into a shape: arch, bulge, pennant, perspective and more. See \
                     the result before you keep it.",
                ),
            ),
            Cmd::RepeatLast => (
                g(ph::REPEAT),
                tr("Repeat"),
                tr("Do the last ribbon command again (Space)."),
            ),
            Cmd::CloseGaps => (
                g(ph::LINK_SIMPLE_HORIZONTAL),
                tr("Close Gaps…"),
                tr(
                    "Join the ends of the selected lines where they are near, so lines drawn one at a time become one line or a closed shape. See the result before you keep it.",
                ),
            ),
            Cmd::CleanUp => (
                g(ph::BANDAIDS),
                tr("Clean Up…"),
                tr(
                    "Remove small loops, overlaps, spikes and zig-zags from the selected lines, fills and vector objects (for example from an SVG file). See the result before you keep it.",
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
            Cmd::RemoveGuides => (
                g(ph::ERASER),
                tr("Remove Guides"),
                tr("Remove all guides, construction lines and construction points."),
            ),
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
            Cmd::AppliqueFromSelection => (
                g(ph::PUZZLE_PIECE),
                tr("Appliqué from Selection"),
                tr(
                    "Make one appliqué object of the selected fills and shapes. A selected satin border that covers an edge is the cover of that edge, and sews after the appliqué.",
                ),
            ),
            Cmd::BackToOriginal => (
                g(ph::ARROW_ARC_LEFT),
                tr("Back to Original"),
                tr("Put back the objects that the selected appliqué replaced, and remove the appliqué."),
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
    pub(super) fn keys(self, ctx: &egui::Context) -> Option<String> {
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
    pub(super) fn opens_dialog(self) -> bool {
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
