//! App state, layout and commands.

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use emb_geom::{BBox, Path, Point};
use emb_model::{Design, Document, HoopCatalog, HoopSelection, MachineCatalog, ObjectId, ObjectKind, Rgb, Step};

use crate::a11y::Named;
use crate::canvas::{self, CanvasView};
use crate::commands::{self, Clipboard};
use crate::draw::{DrawState, Tool};
use crate::file::{Notice, Pending};
use crate::i18n::tr;
use crate::manipulate::DragOp;
use crate::selection::Selection;
use crate::{panels, sequence_panel};

const CTRL: Modifiers = Modifiers::COMMAND;
const CTRL_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
pub const UNDO: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::Z);
pub const REDO: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::Y);
const REDO_ALT: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::Z);
pub const DELETE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
pub const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::A);
const DESELECT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
/// Zoom extents. Double-click the middle mouse button does the same.
pub const ZOOM_EXTENTS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Home);
// Sew order = stacking order, so these follow the usual "bring forward / send backward" keys.
pub const SEQ_BACK: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::OpenBracket);
pub const SEQ_FORWARD: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::CloseBracket);
pub const SEQ_START: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::OpenBracket);
pub const SEQ_END: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::CloseBracket);
pub const NEW: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::N);
pub const OPEN: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::O);
pub const SAVE: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::S);
pub const SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(CTRL_SHIFT, Key::S);
pub const IMPORT: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::I);
pub const EXPORT: KeyboardShortcut = KeyboardShortcut::new(CTRL, Key::E);

/// Default safety margin inside the hoop field, in mm.
pub const DEFAULT_MARGIN_MM: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    Canvas,
    Properties,
    Sequence,
    Threads,
    StitchList,
    DesignInfo,
    Styles,
    /// The second view of the design (View > Split view).
    Split,
    /// The start page (P10 Milestone 4).
    Start,
    /// A view of View > New view (its slot in `AppState::views`).
    View(u8),
    /// Another open design (View > Other design).
    Other,
    /// The design library (P8).
    Library,
    /// The export queue and its history (P8).
    Exports,
    /// Embroidery clipart (P8).
    Clipart,
}

/// Everything except the dock layout. The dock layout borrows this as its tab viewer.
pub struct AppState {
    pub doc: Document,
    pub hoops: HoopCatalog,
    pub machines: MachineCatalog,
    pub view: CanvasView,
    pub selection: Selection,
    /// Object under the pointer in the Sequence panel; the canvas highlights it.
    pub hovered: Option<ObjectId>,
    /// Collapsed color blocks in the Sequence panel, by the id of their first object.
    pub seq_collapsed: std::collections::BTreeSet<ObjectId>,
    pub margin_mm: f64,
    pub show_artwork: bool,
    pub show_jumps: bool,
    /// View > Density map (P11 Milestone 1).
    pub show_density: bool,
    pub density_cache: Option<crate::density::DensityCache>,
    pub cursor_mm: Option<Point>,
    pub fit_requested: bool,
    pub message: String,
    /// The drag in progress on the canvas.
    pub drag: Option<DragOp>,
    /// Makes a new merge key for each drag.
    pub drag_counter: u64,
    pub clipboard: Option<Clipboard>,
    /// Set by commands; the canvas fits this box on its next frame.
    pub zoom_to: Option<BBox>,
    /// Set by "Properties…"; the dock shows the Properties tab.
    pub focus_properties: bool,
    /// The project file of the design, once saved or opened.
    pub file_path: Option<std::path::PathBuf>,
    /// An action that waits for the unsaved-changes prompt.
    pub confirm: Option<Pending>,
    pub notice: Option<Notice>,
    /// Set when the window may close now (after the prompt).
    pub close_now: bool,
    /// Thread for new objects.
    pub current_thread: Option<emb_model::ThreadId>,
    /// The cross-stitch tools (P9).
    pub cross: crate::cross::CrossUi,
    /// The stones of the design (rhinestones, P9).
    pub stone_cache: crate::bling::StoneCache,
    pub tool: Tool,
    pub draw: DrawState,
    /// Per-object stitches, keyed by (ObjectId, revision).
    pub stitch_cache: emb_stitch::StitchCache,
    /// The stitch plan of the current design state.
    pub plan_view: Option<crate::preview::PlanView>,
    /// The plan is older than the design (a slow plan during a drag): the
    /// canvas asks for another frame soon.
    pub plan_waits: bool,
    /// The last design state that the plan saw, and when it changed.
    pub plan_change: (u64, std::time::Instant),
    /// Show the stitches of the design (the Stitches view).
    pub show_plan: bool,
    pub export: Option<crate::export::ExportDialog>,
    /// The texture of the backdrop picture.
    pub backdrop_tex: Option<crate::backdrop::BackdropTexture>,
    pub reshape: crate::reshape::ReshapeState,
    /// What the last right click of the Reshape tool was on.
    pub reshape_menu_hit: Option<crate::reshape::MenuHit>,
    pub settings: crate::workspace::Settings,
    /// Save the settings file when a setting changes (off in tests).
    pub save_settings: bool,
    pub show_options: bool,
    /// The Measure tool line, in design coordinates.
    pub measure: Option<(Point, Point)>,
    /// Keyboard modifiers of the current frame on the canvas.
    pub mods: Modifiers,
    /// Values of the Skew and Rotate dialog: skew X, skew Y, rotate (degrees).
    pub arrange_values: (f64, f64, f64),
    /// Thread or stitch-type filter of the canvas (View > Show only).
    pub view_filter: crate::viewing::ViewFilter,
    /// The stitch cursor (an index into the stitch plan records).
    pub cursor: Option<usize>,
    pub show_needle_points: bool,
    pub show_functions: bool,
    /// Thread charts: the built-in ones and the ones the user imported.
    pub charts: Vec<emb_model::ThreadChart>,
    /// The chart that the Threads panel shows.
    pub chart_index: usize,
    /// The search text of the Threads panel.
    pub chart_query: String,
    /// Set by commands; the dock shows the Threads tab.
    pub focus_threads: bool,
    /// Set by commands; the dock shows the Stitch list tab.
    pub focus_stitch_list: bool,
    /// The Stitch list shows only functions (not stitches).
    pub stitch_list_functions: bool,
    /// The stitch cursor that the Stitch list scrolled to last.
    pub stitch_list_shown: Option<usize>,
    /// Design information for a document state and hoop margin.
    pub info_cache: Option<(u64, f64, std::sync::Arc<crate::info::DesignInfo>)>,
    /// Set by commands; the dock shows the Design info tab.
    pub focus_info: bool,
    pub player: crate::player::Player,
    /// Draw stitches as shaded threads (the Thread preview view).
    pub show_thread_look: bool,
    /// The overview window shows.
    pub show_overview: bool,
    /// The canvas area of the last frame, for the overview window.
    pub canvas_rect: Option<egui::Rect>,
    /// The open Output design dialog.
    pub output: Option<crate::output::OutputDialog>,
    /// The open print preview.
    pub preview: Option<crate::output::Preview>,
    pub fabrics: emb_model::FabricCatalog,
    pub fabric_dialog: Option<crate::styles::FabricDialog>,
    pub template_dialog: Option<crate::styles::TemplateDialog>,
    /// The values for the parameters of a template (ADR 0078).
    pub template_ask: Option<crate::styles::TemplateAsk>,
    /// Show the Styles tab in the next frame.
    pub focus_styles: bool,
    /// Set by commands; the dock shows the Design (canvas) tab or the Library tab.
    pub focus_canvas: bool,
    pub focus_library: bool,
    pub focus_clipart: bool,
    /// A panel to show in front (View > Panels).
    pub focus_tab: Option<Tab>,
    /// The kind of function that Shift+Page Up and Page Down go to.
    pub function_travel: crate::viewtools::FunctionTravel,
    /// The rhinestones show on the canvas.
    pub show_stones: bool,
    /// The fabric texture of the 3D thread look.
    pub fabric_tex: Option<crate::viewtools::FabricTexture>,
    /// The Help window.
    pub help: crate::help::HelpState,
    /// A question for the help search, to answer when the window shows.
    pub help_ask_pending: bool,
    /// The ribbon command under the pointer (for F1).
    pub hover_cmd: Option<crate::ribbon::Cmd>,
    /// The start page shows as a tab; `focus_start` brings it to the front.
    pub show_start: bool,
    pub focus_start: bool,
    pub start: crate::start::StartState,
    /// The lettering kiosk shows in place of the normal window.
    pub kiosk: Option<crate::kiosk::Kiosk>,
    /// The backdrop picture that another program edits.
    pub outside_edit: Option<crate::share::OutsideEdit>,
    /// The My assets window.
    pub assets_window: crate::assets::AssetsWindow,
    /// The views of View > New view (`None`: closed).
    pub views: Vec<Option<crate::tabs::SplitView>>,
    /// The other design view (View > Other design).
    pub other_design: Option<crate::views::OtherDesign>,
    /// The Clipart tab.
    pub clipart: crate::clipart::ClipartUi,
    /// The measured widths of the ribbon panels (open and folded), so the
    /// panels that do not fit fold into one button.
    pub ribbon_widths: std::collections::HashMap<(crate::ribbon::RibbonTab, usize, bool), f32>,
    /// The tab of the canvas leaf that drew in this frame: `true` for the Library tab.
    pub library_drawn: Option<bool>,
    /// The Library tab.
    pub library: crate::library::LibraryUi,
    /// The Send to USB window.
    pub send_dialog: Option<crate::send::SendDialog>,
    /// The export queue (the Exports tab).
    pub jobs: crate::jobs::Queue,
    /// The removable drives that tests give (no real drive is used in tests).
    #[cfg(test)]
    pub test_drives: Vec<crate::send::Drive>,
    /// Show the fabric of appliqué objects on the canvas.
    pub show_applique_fabric: bool,
    /// Show the vector objects (artwork with no stitches).
    pub show_vectors: bool,
    /// Show the hoop field and the safety margin on the canvas.
    pub show_hoop: bool,
    /// The user's own hoops (also in `hoops`), from their `hoops.toml`.
    pub user_hoops: HoopCatalog,
    /// The My hoops and Hoop options windows.
    pub hoop_windows: crate::hoops::HoopWindows,
    /// Hooping mode and Preview hoopings (see `hooping`).
    pub hoopings_ui: crate::hooping::HoopingUi,
    /// The Product window is open.
    pub product_window: bool,
    /// The texture of the product under the design.
    pub product_tex: Option<crate::product::ProductTexture>,
    /// The next frame shows all of the product.
    pub fit_product: bool,
    /// The Repeats window is open.
    pub repeats_window: bool,
    /// The About window is open.
    pub about_window: bool,
    /// Show the vector objects pale, so the stitches stand out.
    pub dim_vectors: bool,
    /// The draw tools make vector objects (artwork) in place of embroidery.
    pub draw_vectors: bool,
    /// Keep originals and overlap amount of the shaping commands.
    pub shaping: emb_model::ShapeOptions,
    /// The Offsets and Arrangements windows.
    pub artwork: crate::artwork::ArtworkWindows,
    /// The values of the picture tools of the backdrop.
    pub picture: crate::picture::PictureTools,
    /// The Picture to design wizard, while it is open.
    pub wizard: Option<Box<crate::wizard::Wizard>>,
    /// The Mirror line tool makes a mirror copy (Reflect).
    pub reflect_copy: bool,
    /// The transform of the drag with this merge key at the last frame, so
    /// linked cuts can follow the step of each frame.
    pub drag_step: Option<(emb_model::MergeKey, emb_geom::Transform)>,
    /// The open designs. The active one is `None`: its state is in the fields above.
    pub tabs: Vec<Option<crate::tabs::DocSlot>>,
    pub active_tab: usize,
    /// The id of the active design for its backup file.
    pub doc_id: u64,
    /// The views before the last zoom or pan command (View > Previous view).
    pub view_history: Vec<CanvasView>,
    /// The second view of the design (View > Split view).
    pub split: crate::tabs::SplitView,
    pub show_split: bool,
    /// Tabs that Quit still has to ask about.
    pub quit_queue: Vec<usize>,
    pub backups: crate::backup::Backups,
    /// Designs of a crashed session that wait for Recover or Discard.
    pub recovery: Vec<crate::backup::Recovered>,
    /// The Sew time window shows.
    pub show_sew_times: bool,
    /// The Needles window (ADR 0063).
    pub show_needles: bool,
    /// The Quote window (P13 Milestone 1, ADR 0064) and the Order Details
    /// window (P13 Milestone 2).
    pub show_quote: bool,
    pub show_order: bool,
    /// The Thread Stock window (P13 Milestone 3, ADR 0066), its state, and
    /// the chart that showed before Only threads in stock.
    pub show_stock: bool,
    pub stock_window: crate::stock::StockWindow,
    pub chart_before_stock: usize,
    /// The Scripts window (P13 Milestone 4, ADR 0067) and its state.
    pub show_scripts: bool,
    pub scripts: crate::scripts::ScriptsWindow,
    /// Sketch mode (P14 Milestone 2, ADR 0069): the open sketch.
    pub sketch: Option<crate::sketch::SketchEdit>,
    pub corner_dialog: Option<crate::corners::CornerDialog>,
    pub hole_dialog: Option<crate::corners::HoleDialog>,
    pub simplify_dialog: Option<crate::simplify::SimplifyDialog>,
    /// The Paste special window.
    pub paste_special: Option<crate::editing::PasteSpecial>,
    /// The bundled fonts and the fonts of the computer.
    pub fonts: crate::lettering::FontLibrary,
    pub lettering_dialog: Option<crate::lettering::LetteringDialog>,
    /// The Find a command window, if it is open (Ctrl+K).
    pub palette: Option<crate::palette::Palette>,
    /// The language of this run from `--lang <code>`, else the settings.
    pub lang_override: Option<String>,
    /// A command waits for a click on an object (Select tool).
    pub pick: Option<crate::lettering::Pick>,
    /// The last ribbon command, for Space (repeat).
    pub last_cmd: Option<crate::ribbon::Cmd>,
    /// The font and height of the last lettering, for the next one.
    pub letter_font: emb_model::FontRef,
    pub letter_height: f64,
    /// The search text of the font list.
    pub font_filter: String,
    pub fonts_window: crate::fonts::FontsWindow,
    pub monogram_window: crate::monogram::MonogramWindow,
    pub team_window: crate::teamnames::TeamNamesWindow,
    pub keyboard_window: crate::keyboard::KeyboardWindow,
    pub motifs_window: crate::motifs::MotifsWindow,
    pub decor: crate::decor::DecorWindows,
    pub color_fills: crate::color_fills::ColorFillsWindow,
    pub quilting: crate::quilting::QuiltingWindow,
    pub cutwork: crate::cutwork::CutworkWindow,
    pub alphabets_window: crate::alphabets::AlphabetsWindow,
    /// The folder of the user's alphabets, if not the `fonts` folder (tests).
    pub alphabets_dir_override: Option<std::path::PathBuf>,
}

pub struct EmbApp {
    state: AppState,
    dock: DockState<Tab>,
    /// The window title that was sent last.
    last_title: String,
    /// False until the first frame, which maximizes the window.
    started: bool,
    /// The window size of the last frame.
    last_size: egui::Vec2,
}

/// Loads a data file from `data/` next to the executable or in the working
/// directory, so users can edit presets without a rebuild. Falls back to the
/// built-in copy.
fn load_catalogs() -> (HoopCatalog, MachineCatalog, emb_model::FabricCatalog, String) {
    let dirs: Vec<std::path::PathBuf> = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("data"))),
        Some(std::path::PathBuf::from("data")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut notes = Vec::new();

    let hoops = dirs
        .iter()
        .map(|d| d.join("hoops.toml"))
        .find(|p| p.is_file())
        .and_then(|p| match HoopCatalog::load(&p) {
            Ok(c) => Some(c),
            Err(e) => {
                notes.push(format!("{e}; using built-in hoops"));
                None
            }
        })
        .unwrap_or_else(HoopCatalog::builtin);

    let machines = dirs
        .iter()
        .map(|d| d.join("machines.toml"))
        .find(|p| p.is_file())
        .and_then(
            |p| match MachineCatalog::load(&p).and_then(|m| m.validate_hoops(&hoops).map(|_| m)) {
                Ok(c) => Some(c),
                Err(e) => {
                    notes.push(format!("{e}; using built-in machines"));
                    None
                }
            },
        )
        .unwrap_or_else(MachineCatalog::builtin);

    let fabrics = dirs
        .iter()
        .map(|d| d.join("fabrics.toml"))
        .find(|p| p.is_file())
        .and_then(|p| {
            match std::fs::read_to_string(&p)
                .map_err(|e| format!("{}: {e}", p.display()))
                .and_then(|s| emb_model::FabricCatalog::from_toml_str(&s).map_err(|e| e.to_string()))
            {
                Ok(c) => Some(c),
                Err(e) => {
                    notes.push(format!("{e}; using built-in fabrics"));
                    None
                }
            }
        })
        .unwrap_or_else(emb_model::FabricCatalog::builtin);

    (hoops, machines, fabrics, notes.join(" | "))
}

/// The dock layout at the start: the canvas and the Library on the left,
/// two groups of panels on the right.
pub fn default_dock() -> DockState<Tab> {
    let mut dock = DockState::new(vec![Tab::Canvas, Tab::Library]);
    let surface = dock.main_surface_mut();
    let [_, right] = surface.split_right(
        NodeIndex::root(),
        0.74,
        vec![Tab::Properties, Tab::Styles, Tab::DesignInfo, Tab::Clipart],
    );
    surface.split_below(
        right,
        0.5,
        vec![Tab::Sequence, Tab::Threads, Tab::StitchList, Tab::Exports],
    );
    // The Library tab is next to the canvas; the canvas shows first.
    if let Some(p) = dock.find_tab(&Tab::Canvas) {
        let _ = dock.set_active_tab(p);
    }
    dock
}

impl EmbApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::icons::install(&cc.egui_ctx);
        let (hoops, machines, fabrics, message) = load_catalogs();
        let dock = default_dock();

        let mut state = AppState::new(hoops, machines, message);
        state.settings = crate::workspace::Settings::load();
        state.start_library(crate::library::library_dir());
        state.focus_library = state.settings.library_open;
        // The start page, unless a file is opened from the command line.
        if state.settings.start_page && std::env::args_os().nth(1).is_none() {
            state.show_start_page();
            state.focus_library = false;
        }
        state.jobs = crate::jobs::Queue::load(crate::about::user_folder().map(|d| d.join("exports.json")));
        // The user's own hoops (Design > My hoops).
        if let Some(p) = crate::hoops::user_hoops_path().filter(|p| p.is_file()) {
            match HoopCatalog::load(&p) {
                Ok(user) => state.add_user_hoops(user),
                Err(e) => state.message = crate::i18n::fill("{}; your own hoops are not loaded", &[&e]),
            }
        }
        crate::canvas::set_calibration(state.settings.screen_calibration);
        state.save_settings = true;
        state.fabrics = fabrics;
        state.backups = crate::backup::Backups::new(crate::backup::backups_dir());
        if let Some(root) = state.backups.root.clone() {
            state.recovery = crate::backup::find_recoverable(
                &root,
                state.backups.session_dir().as_deref(),
                crate::backup::unix_now(),
            );
        }
        if state.settings.template.is_some() {
            // The start design comes from the template for New.
            state.new_design();
        }
        // A crash report from the last time: say where it is.
        if let Some(dir) = crate::crash::crashes_dir() {
            let reports = crate::crash::take_unseen(&dir);
            if !reports.is_empty() && state.notice.is_none() {
                let mut lines = vec![
                    "The program stopped because of an error the last time it ran. Your work is in the \
                     automatic backups."
                        .to_string(),
                    "Please send this report with your feedback:".to_string(),
                ];
                lines.extend(reports.iter().map(|p| p.display().to_string()));
                state.notice = Some(crate::file::Notice {
                    title: "Crash report".into(),
                    lines,
                });
            }
        }
        let (charts, errors) = crate::threads::load_charts();
        state.charts = charts;
        state.refresh_stock_chart();
        if !errors.is_empty() {
            state.message = crate::i18n::fill("Thread charts not loaded: {}", &[&(errors.join(" | "))]);
        }
        // A file name on the command line (for example from a double-click in Explorer).
        // `--kiosk`: the program starts in the lettering kiosk (for a shop).
        // `--lang <code>` first: the language for this run (the settings keep theirs).
        let mut args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
        if args.len() >= 2 && args[0] == "--lang" {
            state.lang_override = Some(args[1].to_string_lossy().into_owned());
            args.drain(..2);
        }
        let arg = args.into_iter().next();
        if arg.as_deref() == Some(std::ffi::OsStr::new("--kiosk")) {
            state.show_start = false;
            state.open_kiosk();
        } else if let Some(arg) = arg {
            state.open_any(std::path::Path::new(&arg));
        }
        Self {
            state,
            dock,
            last_title: String::new(),
            started: false,
            last_size: egui::Vec2::ZERO,
        }
    }
}

impl AppState {
    /// Makes the state with an empty design on the Topaz 20 profile (if it exists).
    pub fn new(hoops: HoopCatalog, machines: MachineCatalog, message: String) -> Self {
        let mut design = Design::new("Untitled");
        if let Some(m) = machines.get("husqvarna_topaz20") {
            design.machine = Some(m.id.clone());
            if let Some(h) = m.hoops.first() {
                design.hoop.selection = HoopSelection::Preset { id: h.clone() };
            }
        }
        Self {
            doc: Document::new(design),
            hoops,
            machines,
            view: CanvasView::default(),
            selection: Selection::default(),
            hovered: None,
            seq_collapsed: Default::default(),
            margin_mm: DEFAULT_MARGIN_MM,
            show_artwork: true,
            show_jumps: false,
            show_density: false,
            density_cache: None,
            cursor_mm: None,
            fit_requested: true,
            message,
            drag: None,
            drag_counter: 0,
            clipboard: None,
            zoom_to: None,
            focus_properties: false,
            file_path: None,
            confirm: None,
            notice: None,
            close_now: false,
            current_thread: None,
            cross: Default::default(),
            stone_cache: Default::default(),
            tool: Tool::Select,
            draw: DrawState::default(),
            stitch_cache: emb_stitch::StitchCache::default(),
            plan_view: None,
            plan_waits: false,
            plan_change: (0, std::time::Instant::now()),
            show_plan: true,
            export: None,
            backdrop_tex: None,
            reshape: Default::default(),
            reshape_menu_hit: None,
            settings: crate::workspace::Settings::default(),
            save_settings: false,
            show_options: false,
            measure: None,
            mods: Modifiers::NONE,
            arrange_values: (15.0, 0.0, 45.0),
            view_filter: Default::default(),
            cursor: None,
            show_needle_points: false,
            show_functions: false,
            charts: emb_model::ThreadChart::builtin(),
            chart_index: 0,
            chart_query: String::new(),
            focus_threads: false,
            focus_stitch_list: false,
            stitch_list_functions: false,
            stitch_list_shown: None,
            info_cache: None,
            focus_info: false,
            player: Default::default(),
            show_thread_look: false,
            show_overview: false,
            canvas_rect: None,
            output: None,
            preview: None,
            fabrics: emb_model::FabricCatalog::builtin(),
            fabric_dialog: None,
            template_dialog: None,
            template_ask: None,
            focus_styles: false,
            focus_canvas: false,
            focus_library: false,
            focus_clipart: false,
            focus_tab: None,
            function_travel: Default::default(),
            show_stones: true,
            fabric_tex: None,
            help: Default::default(),
            help_ask_pending: false,
            hover_cmd: None,
            show_start: false,
            focus_start: false,
            start: Default::default(),
            kiosk: None,
            outside_edit: None,
            assets_window: Default::default(),
            views: Vec::new(),
            other_design: None,
            clipart: Default::default(),
            ribbon_widths: Default::default(),
            library_drawn: None,
            library: Default::default(),
            send_dialog: None,
            jobs: Default::default(),
            #[cfg(test)]
            test_drives: Vec::new(),
            show_applique_fabric: true,
            show_vectors: true,
            show_hoop: true,
            user_hoops: HoopCatalog::default(),
            hoop_windows: Default::default(),
            hoopings_ui: Default::default(),
            product_window: false,
            product_tex: None,
            fit_product: false,
            repeats_window: false,
            about_window: false,
            dim_vectors: false,
            draw_vectors: false,
            shaping: emb_model::ShapeOptions {
                linked: true,
                ..Default::default()
            },
            artwork: Default::default(),
            picture: Default::default(),
            wizard: None,
            reflect_copy: false,
            drag_step: None,
            tabs: vec![None],
            active_tab: 0,
            doc_id: crate::tabs::fresh_doc_id(),
            view_history: Vec::new(),
            split: Default::default(),
            show_split: false,
            quit_queue: Vec::new(),
            backups: crate::backup::Backups::new(None),
            recovery: Vec::new(),
            show_sew_times: false,
            show_needles: false,
            show_quote: false,
            show_order: false,
            show_stock: false,
            stock_window: Default::default(),
            chart_before_stock: 0,
            show_scripts: false,
            scripts: Default::default(),
            sketch: None,
            corner_dialog: None,
            hole_dialog: None,
            simplify_dialog: None,
            paste_special: None,
            fonts: crate::lettering::FontLibrary::default(),
            lettering_dialog: None,
            palette: None,
            lang_override: None,
            pick: None,
            last_cmd: None,
            letter_font: emb_model::FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, ""),
            letter_height: emb_model::lettering::DEFAULT_HEIGHT_MM,
            font_filter: String::new(),
            fonts_window: crate::fonts::FontsWindow::default(),
            monogram_window: crate::monogram::MonogramWindow::default(),
            team_window: Default::default(),
            keyboard_window: Default::default(),
            motifs_window: Default::default(),
            decor: Default::default(),
            color_fills: Default::default(),
            quilting: Default::default(),
            cutwork: Default::default(),
            alphabets_window: Default::default(),
            alphabets_dir_override: None,
        }
    }

    pub fn undo(&mut self) {
        if let Some(label) = self.doc.undo() {
            self.message = crate::i18n::fill("Undo: {}", &[&label]);
            self.fix_selection();
        }
    }

    pub fn redo(&mut self) {
        if let Some(label) = self.doc.redo() {
            self.message = crate::i18n::fill("Redo: {}", &[&label]);
            self.fix_selection();
        }
    }

    /// Removes deleted objects from the selection.
    fn fix_selection(&mut self) {
        self.selection.retain_existing(self.doc.design());
    }

    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let ids = self.selection.ids().to_vec();
        let label = if ids.len() == 1 {
            "Delete object"
        } else {
            "Delete objects"
        };
        self.doc.edit(label, |d| {
            for id in ids {
                d.remove_object(id);
            }
        });
        self.selection.clear();
    }

    /// Applies a sew-order operation to the design as one undo step.
    /// Does not add an undo step if the order does not change.
    pub fn resequence(&mut self, label: &str, op: impl FnOnce(&mut Design) -> bool) {
        let mut probe = self.doc.design().clone();
        if !op(&mut probe) {
            self.message = crate::i18n::fill("{}: no change", &[&label]);
            return;
        }
        let _ = self.doc.try_edit(label, |d| {
            *d = probe;
            Ok::<(), ()>(())
        });
        self.message = label.to_string();
    }

    /// Shortest order (owner report, 2026-10-24): each color block in
    /// the order with the least travel (`emb_stitch::order`). The new order
    /// is used only if it has fewer trims, or as many and a shorter way
    /// between the objects. One undo step.
    pub fn optimize_sew_order(&mut self) {
        let opts = self.plan_options();
        let design = self.doc.design().clone();
        let order = emb_stitch::order::optimized_order(&design, &mut self.stitch_cache);
        let mut probe = design.clone();
        if !probe.sequence_in_order(&order) {
            self.message = tr("Shortest order: the order is already the shortest.").into();
            return;
        }
        // Trims, and the length of the moves between the objects.
        let cost = |d: &Design, cache: &mut emb_stitch::StitchCache| {
            let plan = emb_stitch::build_plan(d, &opts, cache);
            let mut moves = 0.0;
            let mut at: Option<emb_geom::Point> = None;
            for r in &plan.stitches.records {
                if r.kind == emb_model::StitchKind::Jump
                    && let Some(p) = at
                {
                    moves += p.distance(r.pos);
                }
                at = Some(r.pos);
            }
            (plan.stitches.trim_count(), moves)
        };
        let (before, after) = (
            cost(&design, &mut self.stitch_cache),
            cost(&probe, &mut self.stitch_cache),
        );
        if after.0 > before.0 || (after.0 == before.0 && after.1 >= before.1 - 1e-6) {
            self.message = tr("Shortest order: no shorter order found.").into();
            return;
        }
        let _ = self.doc.try_edit("Shortest Order", |d| {
            *d = probe;
            Ok::<(), ()>(())
        });
        self.message = crate::i18n::fill(
            "Shortest order: {} trims and {} mm of moves (before: {} and {} mm).",
            &[
                &after.0,
                &format!("{:.0}", after.1),
                &before.0,
                &format!("{:.0}", before.1),
            ],
        );
    }

    /// Cap order (ADR 0050): each color from the center out and from the
    /// bottom up, and lettering from the middle letter out. One undo step.
    pub fn cap_sew_order(&mut self) {
        let order = emb_stitch::order::cap_order(self.doc.design());
        let mut probe = self.doc.design().clone();
        let moved = probe.sequence_in_order(&order);
        let lettering: Vec<ObjectId> = probe
            .objects()
            .iter()
            .filter(|o| matches!(&o.kind, ObjectKind::Lettering(l) if l.order != emb_model::LetterOrder::CenterOut))
            .map(|o| o.id)
            .collect();
        let letters = lettering.len();
        for id in lettering {
            if let Some(ObjectKind::Lettering(l)) = probe.object_mut(id).map(|o| &mut o.kind) {
                l.order = emb_model::LetterOrder::CenterOut;
            }
        }
        if !moved && letters == 0 {
            self.message = tr("Cap order: the order is already from the center out.").into();
            return;
        }
        let _ = self.doc.try_edit("Cap Order", |d| {
            *d = probe;
            Ok::<(), ()>(())
        });
        self.message = tr("Cap order: from the center out and from the bottom up.").into();
    }

    /// Runs a sew-order command on the selection (in sew order).
    pub fn sequence_selected(&mut self, label: &str, op: impl FnOnce(&mut Design, &[ObjectId]) -> bool) {
        let ids = self.selection.in_sew_order(self.doc.design());
        if ids.is_empty() {
            self.message = tr("Select objects first").into();
            return;
        }
        self.resequence(label, |d| op(d, &ids));
    }

    /// Box for "zoom extents": the hoop field and the design (with its rotation).
    pub fn fit_box(&self) -> Option<BBox> {
        let d = self.doc.design();
        let hoop = self.hoop_outline(0.0).and_then(|p| p.control_bbox());
        let t = d.rotation_transform();
        let design = d.extents().and_then(|b| {
            BBox::from_points(
                [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)].map(|p| t.apply(p)),
            )
        });
        [hoop, design, crate::product::product_box(self)]
            .into_iter()
            .flatten()
            .reduce(BBox::union)
            .or(Some(BBox::from_center_size(Point::ZERO, 100.0, 100.0)))
    }

    /// The outline of the hoop field made smaller by `margin`, in view
    /// coordinates (the hoop angle applied). `None` in open-field mode.
    pub fn hoop_outline(&self, margin: f64) -> Option<Path> {
        let d = self.doc.design();
        let HoopSelection::Preset { id } = &d.hoop.selection else {
            return None;
        };
        let field = self.hoops.get(id)?.outline(Point::ZERO, margin)?;
        Some(field.transformed(&d.from_hoop_transform()))
    }

    /// The middle of the back edge of the hoop field (the edge at the back
    /// of the machine) and the unit direction from the hoop center to it,
    /// in view coordinates. `None` in open-field mode.
    pub fn hoop_back(&self) -> Option<(Point, Point)> {
        let d = self.doc.design();
        let HoopSelection::Preset { id } = &d.hoop.selection else {
            return None;
        };
        let hoop = self.hoops.get(id)?;
        let back = d.from_hoop_transform().apply(Point::new(0.0, -hoop.height_mm / 2.0));
        let dir = back - d.hoop.center;
        let len = dir.length();
        (len > 0.0).then(|| (back, Point::new(dir.x / len, dir.y / len)))
    }

    /// Returns `true` if the object goes outside the hoop safety margin.
    pub fn outside_margin(&self, id: ObjectId) -> bool {
        let d = self.doc.design();
        let HoopSelection::Preset { id: hoop_id } = &d.hoop.selection else {
            return false;
        };
        let (Some(hoop), Some(obj)) = (self.hoops.get(hoop_id), d.object(id)) else {
            return false;
        };
        let t = d.output_transform();
        let tol = 0.1;
        obj.kind.paths().any(|p| {
            p.flatten(tol)
                .iter()
                .flat_map(|pl| pl.points.iter())
                .any(|pt| !hoop.contains(t.apply(*pt), self.margin_mm))
        })
    }

    /// Adds a small two-color patch, for the tests.
    #[cfg(test)]
    pub fn add_sample_objects(&mut self) {
        use emb_model::{FillParams, RunningParams, SatinGeometry, SatinParams};
        let ids = self.doc.edit("Add Sample Objects", |d| {
            let red = d
                .threads
                .iter()
                .find(|t| t.name == "Red")
                .map(|t| t.id)
                .unwrap_or_else(|| d.add_thread("Red", Rgb(200, 30, 45)));
            let white = d
                .threads
                .iter()
                .find(|t| t.name == "White")
                .map(|t| t.id)
                .unwrap_or_else(|| d.add_thread("White", Rgb(240, 240, 240)));
            let c = d.hoop.center;
            let fill = d.add_object(
                red,
                ObjectKind::Fill {
                    outer: Path::ellipse(c, 30.0, 30.0),
                    holes: vec![],
                    params: FillParams::default(),
                },
            );
            let satin = d.add_object(
                white,
                ObjectKind::Satin {
                    geometry: SatinGeometry {
                        rail_a: Path::ellipse(c, 33.0, 33.0),
                        rail_b: Path::ellipse(c, 29.0, 29.0),
                        rungs: vec![],
                    },
                    params: SatinParams::default(),
                },
            );
            let run = d.add_object(
                white,
                ObjectKind::Running {
                    path: Path::new().move_to(c + Point::new(-18.0, 6.0)).cubic_to(
                        c + Point::new(-8.0, -14.0),
                        c + Point::new(8.0, 20.0),
                        c + Point::new(18.0, -6.0),
                    ),
                    params: RunningParams::default(),
                },
            );
            // A second color patch to the right, so the Sequence panel has
            // blocks to merge and reorder.
            let gold = d
                .threads
                .iter()
                .find(|t| t.name == "Gold")
                .map(|t| t.id)
                .unwrap_or_else(|| d.add_thread("Gold", Rgb(230, 170, 30)));
            let c2 = c + Point::new(64.0, 0.0);
            let star = d.add_object(
                gold,
                ObjectKind::Fill {
                    outer: Path::polygon(
                        &(0..10)
                            .map(|i| {
                                let r = if i % 2 == 0 { 25.0 } else { 11.0 };
                                let a = emb_geom::math::to_radians(f64::from(i) * 36.0 - 90.0);
                                c2 + Point::new(r * emb_geom::math::cos(a), r * emb_geom::math::sin(a))
                            })
                            .collect::<Vec<_>>(),
                    ),
                    holes: vec![],
                    params: FillParams::default(),
                },
            );
            let ring = d.add_object(
                red,
                ObjectKind::Running {
                    path: Path::ellipse(c2, 29.0, 29.0),
                    params: RunningParams {
                        repeats: 3,
                        ..RunningParams::default()
                    },
                },
            );
            [fill, satin, run, star, ring]
        });
        self.selection.set(ids[0]);
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // F1: the help of the button under the pointer (P10 Milestone 4).
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1)) {
            self.help_f1(crate::help::window_topic(ctx));
        }
        // F6: the keyboard focus goes to the ribbon (P10 Milestone 7).
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F6)) && crate::ribbon::focus_ribbon(ctx) {
            self.message =
                tr("Ribbon: Tab and Shift+Tab go from button to button; Enter runs one; Esc goes back.").into();
        }
        // Ctrl+K: Find a command (P10 Milestone 7), also from a text field.
        if ctx.input_mut(|i| i.consume_shortcut(&crate::palette::FIND_COMMAND)) {
            self.open_palette();
        }
        // The Find a command window takes the arrow keys, Enter and Esc.
        if self.palette.is_some() {
            return;
        }
        // Esc closes the Lettering window, also when its text box has the focus.
        if self.lettering_dialog.is_some() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.lettering_dialog = None;
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            return;
        }
        // Ctrl+Z in the Background step of the Picture to design wizard
        // undoes the last change of the picture there, not of the design.
        if let Some(w) = self.wizard.as_mut()
            && w.step == crate::wizard::Step::Background
            && ctx.input_mut(|i| i.consume_shortcut(&UNDO))
        {
            w.undo();
            return;
        }
        // Sketch mode takes Esc, Delete, Backspace, Enter and Ctrl+A; the
        // value box of a dimension takes all keys.
        if self.sketch.is_some() && crate::sketch::handle_keys(ctx, self) {
            return;
        }
        // Do not take keys from a focused text field. Esc takes the keyboard
        // focus away from a button (after F6), back to the canvas.
        if let Some(id) = ctx.memory(|m| m.focused()) {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
            return;
        }
        // Hooping mode takes Enter, Esc, Delete and Backspace.
        if self.hoopings_ui.mode && crate::hooping::handle_keys(ctx, self) {
            return;
        }
        // `consume_shortcut` ignores an extra Shift, so the Shift variants must be
        // consumed first. With Shift held, the logical key of '[' is '{' on most
        // layouts, so accept both forms.
        // A dialog is open: it takes the keys.
        if self.confirm.is_some() || self.notice.is_some() || self.export.is_some() {
            return;
        }
        // File commands (Shift variant first).
        let (save_as, save, new, open, import, export) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&SAVE_AS),
                i.consume_shortcut(&SAVE),
                i.consume_shortcut(&NEW),
                i.consume_shortcut(&OPEN),
                i.consume_shortcut(&IMPORT),
                i.consume_shortcut(&EXPORT),
            )
        });
        if export {
            self.open_export();
        }
        if save_as {
            self.save_as();
        } else if save {
            self.save();
        }
        if new {
            self.request(Pending::New);
        }
        if open {
            self.request(Pending::Open(None));
        }
        if import {
            self.import_artwork_dialog(crate::vectors::ArtworkImport::Objects);
        }
        // Alt+V before V (mirror up-down).
        if ctx.input_mut(|i| i.consume_shortcut(&commands::HIDE)) {
            self.hide_selection();
        }
        // Draw tools take Enter, Backspace and Esc first; Reshape also takes
        // Delete while a node is selected.
        if self.tool != Tool::Select {
            let node = self.tool == Tool::Reshape && self.reshape.node.is_some();
            let keys = ctx.input_mut(|i| {
                [Key::Enter, Key::Backspace, Key::Escape, Key::Delete].map(|k| {
                    let take = k != Key::Delete || node;
                    (k, take && i.consume_key(Modifiers::NONE, k))
                })
            });
            for (k, pressed) in keys {
                if pressed {
                    self.draw_key(k);
                }
            }
        }
        let k = ctx.input_mut(|i| {
            let redo = i.consume_shortcut(&REDO_ALT) || i.consume_shortcut(&REDO);
            let start = i.consume_key(CTRL, Key::OpenCurlyBracket) || i.consume_shortcut(&SEQ_START);
            let end = i.consume_key(CTRL, Key::CloseCurlyBracket) || i.consume_shortcut(&SEQ_END);
            [
                i.consume_shortcut(&UNDO),
                redo,
                i.consume_shortcut(&DELETE),
                i.consume_shortcut(&ZOOM_EXTENTS),
                i.consume_shortcut(&SELECT_ALL),
                i.consume_shortcut(&DESELECT),
                start,
                end,
                i.consume_shortcut(&SEQ_BACK),
                i.consume_shortcut(&SEQ_FORWARD),
            ]
        });
        let [undo, redo, del, extents, all, mut none, start, end, back, forward] = k;
        // Esc first cancels a command that waits for a click.
        if none && self.pick.take().is_some() {
            self.message = tr("Canceled.").into();
            none = false;
        }
        // Space repeats the last ribbon command (as in CAD programs).
        let space = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space));
        let drawing = self.tool != Tool::Select && !self.draw.points.is_empty();
        if space
            && self.drag.is_none()
            && !drawing
            && let Some(cmd) = self.last_cmd
        {
            if self.can(cmd, &crate::ribbon::SelInfo::of(self)) {
                self.run(cmd, ctx);
            } else {
                self.message = crate::i18n::fill("{} cannot run now.", &[&(cmd.meta().1)]);
            }
        }
        // View and selection keys of roadmap P2.
        let (zoom_in, zoom_out, next, prev, travel) = ctx.input_mut(|i| {
            let prev = i.consume_key(Modifiers::SHIFT, Key::Tab);
            let next = i.consume_key(Modifiers::NONE, Key::Tab);
            let zoom_in = i.consume_key(Modifiers::NONE, Key::Plus) || i.consume_key(Modifiers::NONE, Key::Equals);
            let zoom_out = i.consume_key(Modifiers::NONE, Key::Minus);
            (zoom_in, zoom_out, next, prev, crate::viewing::travel_keys(i))
        });
        if zoom_in {
            self.zoom_by(1.5);
        }
        if zoom_out {
            self.zoom_by(1.0 / 1.5);
        }
        if next {
            self.select_step(true);
        }
        if prev {
            self.select_step(false);
        }
        if let Some(t) = travel {
            self.travel(t);
        }

        // Object commands. Copy, cut and paste arrive as their own events, not as keys.
        let (copy, cut, paste_text) = ctx.input(|i| {
            let mut paste = None;
            let (mut copy, mut cut) = (false, false);
            for e in &i.events {
                match e {
                    egui::Event::Copy => copy = true,
                    egui::Event::Cut => cut = true,
                    egui::Event::Paste(text) => paste = Some(text.clone()),
                    _ => {}
                }
            }
            (copy, cut, paste)
        });
        let o = ctx.input_mut(|i| {
            let rot_ccw = i.consume_shortcut(&commands::ROTATE_CCW);
            let rot_cw = i.consume_shortcut(&commands::ROTATE_CW);
            let big = i.modifiers.shift;
            let mut nudge = (0.0, 0.0);
            for (key, dx, dy) in [
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowUp, 0.0, -1.0),
                (Key::ArrowDown, 0.0, 1.0),
            ] {
                let n = i.count_and_consume_key(Modifiers::NONE, key) as f64;
                nudge.0 += dx * n;
                nudge.1 += dy * n;
            }
            let step = if big {
                self.settings.nudge_large_mm
            } else {
                self.settings.nudge_mm
            };
            (
                (
                    i.consume_shortcut(&commands::DUPLICATE),
                    i.consume_shortcut(&crate::join::JOIN),
                ),
                rot_cw,
                rot_ccw,
                i.consume_shortcut(&commands::MIRROR_H),
                i.consume_shortcut(&commands::MIRROR_V),
                (nudge.0 * step, nudge.1 * step),
            )
        });
        let ((duplicate, join), rot_cw, rot_ccw, mirror_h, mirror_v, nudge) = o;
        if copy {
            self.copy_selection(ctx);
        }
        if cut {
            self.cut_selection(ctx);
        }
        if let Some(text) = paste_text {
            // Paste our own clipboard data; ignore plain text.
            match Clipboard::from_json(&text) {
                Some(clip) => self.paste(Some(clip)),
                None if self.clipboard.is_some() => self.paste(None),
                None => {}
            }
        }
        if duplicate {
            self.duplicate_selection();
        }
        if join && self.selection.len() > 1 {
            self.join_selection();
        }
        if rot_cw && !self.selection.is_empty() {
            self.rotate_selection(90.0);
        }
        if rot_ccw && !self.selection.is_empty() {
            self.rotate_selection(-90.0);
        }
        if mirror_h && !self.selection.is_empty() {
            self.mirror_selection(true);
        }
        if mirror_v && !self.selection.is_empty() {
            self.mirror_selection(false);
        }
        if nudge != (0.0, 0.0) {
            self.nudge_selection(nudge.0, nudge.1);
        }

        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if del {
            self.delete_selected();
        }
        if extents {
            self.fit_requested = true;
        }
        if all {
            let ids: Vec<ObjectId> = self.doc.design().objects().iter().map(|o| o.id).collect();
            self.selection.set_all(ids);
        }
        if none {
            self.selection.clear();
        }
        if start {
            self.sequence_selected("Move to Start", |d, ids| d.move_objects_to_start(ids));
        }
        if end {
            self.sequence_selected("Move to End", |d, ids| d.move_objects_to_end(ids));
        }
        if back {
            self.sequence_selected("Move Back One", |d, ids| d.step_objects(ids, Step::Backward));
        }
        if forward {
            self.sequence_selected("Move Forward One", |d, ids| d.step_objects(ids, Step::Forward));
        }
    }

    /// Design threads as swatches. Click: thread for new objects, and for
    /// the selection if there is one. "+": add a thread with a color picker.
    pub(crate) fn thread_strip(&mut self, ui: &mut egui::Ui) {
        ui.label(tr("Threads"));
        let threads = self.doc.design().threads.clone();
        let current = self
            .current_thread
            .filter(|t| threads.iter().any(|x| x.id == *t))
            .or_else(|| threads.first().map(|t| t.id));
        for t in &threads {
            let size = egui::Vec2::splat(18.0);
            let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, 3.0, canvas::color32(t.color));
            let ring = if Some(t.id) == current {
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
            } else {
                egui::Stroke::new(1.0, ui.visuals().weak_text_color())
            };
            p.rect_stroke(rect, 3.0, ring, egui::StrokeKind::Outside);
            let resp = resp.on_hover_text(crate::i18n::fill(
                "{}\nClick: use for new objects{}",
                &[
                    &t.name,
                    &(if self.selection.is_empty() {
                        ""
                    } else {
                        " and the selection"
                    }),
                ],
            ));
            if resp.clicked() {
                self.current_thread = Some(t.id);
                if !self.selection.is_empty() {
                    self.set_thread(t.id);
                }
            }
        }
        let popup_id = ui.make_persistent_id("add_thread_color");
        let resp = ui
            .small_button("+")
            .on_hover_text(tr("Add a thread color"))
            .named(tr("Add a thread color"));
        egui::Popup::from_toggle_button_response(&resp)
            .id(popup_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                // The picked color lives in temporary UI memory until "Add thread".
                let mut color = ui.data_mut(|d| *d.get_temp_mut_or(popup_id, egui::Color32::from_rgb(200, 30, 45)));
                egui::color_picker::color_picker_color32(ui, &mut color, egui::color_picker::Alpha::Opaque);
                ui.data_mut(|d| d.insert_temp(popup_id, color));
                if ui.button(tr("Add thread")).clicked() {
                    let rgb = Rgb(color.r(), color.g(), color.b());
                    let id = self.doc.edit("Add thread", |d| crate::file::thread_for_color(d, rgb));
                    self.current_thread = Some(id);
                    ui.close();
                }
            });
    }

    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let info = self.design_info();
        let mut open_info = false;
        // A command that waits for a click says what to click.
        if let Some(pick) = self.pick {
            ui.horizontal(|ui| {
                ui.strong(tr("Lettering on line:"));
                ui.label(pick.prompt());
            });
        }
        // In sketch mode: what to do next, and the state of the sketch.
        if let Some((prompt, state)) = self.sketch_status() {
            ui.horizontal(|ui| {
                ui.strong(tr("Sketch:"));
                ui.label(prompt);
                ui.weak(format!("· {state} · Esc: cancel"));
            });
        }
        // While drawing, the first line tells the user what to do next.
        if self.tool != Tool::Select {
            ui.horizontal(|ui| {
                ui.strong(format!("{}:", self.tool.label()));
                let prompt = crate::letter_edit::prompt(self)
                    .unwrap_or_else(|| self.tool.prompt(self.draw.points.len(), self.draw.stages.len()));
                ui.label(prompt);
                if self.tool.uses_points() {
                    ui.weak(tr("Enter: finish · Backspace: remove point · Esc: cancel"));
                }
            });
        }
        if let Some(line) = self.cursor_text() {
            ui.horizontal(|ui| {
                ui.strong(tr("Stitch cursor:"));
                ui.label(line);
                ui.weak(tr(crate::viewing::KEYS_HELP));
            });
        }
        ui.horizontal(|ui| {
            let d = self.doc.design();
            let size = d
                .extents()
                .map(|b| {
                    format!(
                        "{} × {}",
                        self.settings.units.fmt(b.width()),
                        self.settings.units.fmt(b.height())
                    )
                })
                .unwrap_or_else(|| tr("empty").into());
            ui.label(crate::i18n::fill("Objects: {}", &[&(d.objects().len())]));
            ui.separator();
            ui.label(crate::i18n::fill("Colors: {}", &[&(d.sewn_color_blocks().len())]));
            if let Some(pv) = &self.plan_view {
                let s = &pv.plan.stitches;
                ui.separator();
                ui.label(crate::i18n::fill(
                    "Stitches: {}  Trims: {}",
                    &[&(s.stitch_count()), &(s.trim_count())],
                ))
                .on_hover_text(crate::i18n::fill(
                    "Stitch plan built in {} ms",
                    &[&format!("{:.1}", pv.build_ms)],
                ));
            }
            if let Some(m) = info.minutes {
                ui.separator();
                ui.label(crate::i18n::fill("Sew time: about {}", &[&(crate::info::time_text(m))]))
                    .on_hover_text(tr(
                        "Stitches at the speed of the machine; trims and color changes add time.",
                    ));
            }
            if !info.issues.is_empty() {
                ui.separator();
                let (text, color) = if info.errors() > 0 {
                    (
                        format!("⛔ {} problems", info.issues.len()),
                        egui::Color32::from_rgb(220, 60, 50),
                    )
                } else {
                    (
                        format!("⚠ {} warnings", info.issues.len()),
                        egui::Color32::from_rgb(230, 150, 20),
                    )
                };
                let r = ui
                    .add(egui::Button::new(egui::RichText::new(text).color(color)).frame(false))
                    .on_hover_text(tr("Show the warnings in the Design info panel"));
                if r.clicked() {
                    open_info = true;
                }
            }
            ui.separator();
            ui.label(crate::i18n::fill("Size: {}", &[&size]));
            ui.separator();
            ui.label(crate::i18n::fill(
                "Zoom: {} %",
                &[&format!("{:.0}", self.view.zoom * 100.0)],
            ));
            if let Some(p) = self.cursor_mm {
                ui.separator();
                let u = self.settings.units;
                let c = p - d.hoop.center;
                ui.label(format!("X {}  Y {}", u.fmt(c.x), u.fmt(c.y)));
            }
            if self.doc.is_modified() {
                ui.separator();
                ui.label("●").on_hover_text(tr("Changed since last save"));
            }
            if !self.message.is_empty() {
                ui.separator();
                ui.label(&self.message);
            }
        });
        if open_info {
            self.focus_info = true;
        }
    }
}

impl TabViewer for AppState {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::Canvas => tr("Design"),
            Tab::Properties => tr("Object Properties"),
            Tab::Sequence => tr("Sequence"),
            Tab::Threads => tr("Threads"),
            Tab::StitchList => tr("Stitch List"),
            Tab::DesignInfo => tr("Design Info"),
            Tab::Styles => tr("Styles"),
            Tab::Split => tr("Split View"),
            Tab::Library => tr("Library"),
            Tab::Clipart => tr("Clipart"),
            Tab::Start => tr("Start"),
            Tab::View(i) => return crate::i18n::fill("View {}", &[&(*i + 3)]).into(),
            Tab::Other => {
                return match &self.other_design {
                    Some(o) => crate::i18n::fill("Other: {}", &[&self.tab_title(o.tab).0]).into(),
                    None => tr("Other Design").into(),
                };
            }
            Tab::Exports => {
                return match self.jobs.active() {
                    0 => tr("Exports").into(),
                    n => crate::i18n::fill("Exports ({})", &[&n]).into(),
                };
            }
        }
        .into()
    }

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("tab", *tab))
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::Canvas => {
                self.library_drawn = Some(false);
                canvas::show(ui, self)
            }
            Tab::Properties => panels::properties(ui, self),
            Tab::Sequence => sequence_panel::show(ui, self),
            Tab::Threads => crate::threads::panel(ui, self),
            Tab::StitchList => crate::functions::stitch_list(ui, self),
            Tab::DesignInfo => crate::info::panel(ui, self),
            Tab::Styles => crate::styles::panel(ui, self),
            Tab::Split => crate::tabs::split_canvas(ui, self),
            Tab::Library => {
                self.library_drawn = Some(true);
                crate::library::panel(ui, self)
            }
            Tab::Exports => crate::jobs::panel(ui, self),
            Tab::Clipart => crate::clipart::panel(ui, self),
            Tab::Start => crate::start::page(ui, self),
            Tab::View(i) => crate::views::view_canvas(ui, self, *i as usize),
            Tab::Other => crate::views::other_design(ui, self),
        }
    }

    /// The panels close with their × button; View > Panels opens them again.
    fn is_closeable(&self, tab: &Tab) -> bool {
        matches!(tab, Tab::Start | Tab::View(_) | Tab::Other) || crate::viewtools::PANELS.iter().any(|(t, _)| t == tab)
    }

    fn on_close(&mut self, tab: &mut Tab) -> egui_dock::widgets::tab_viewer::OnCloseResponse {
        match tab {
            Tab::Start => self.show_start = false,
            Tab::View(i) => {
                if let Some(v) = self.views.get_mut(*i as usize) {
                    *v = None;
                }
            }
            Tab::Other => self.other_design = None,
            _ => self.set_panel_shown(*tab, false),
        }
        egui_dock::widgets::tab_viewer::OnCloseResponse::Close
    }

    fn clear_background(&self, tab: &Tab) -> bool {
        !matches!(tab, Tab::Canvas | Tab::Split)
    }
}

impl eframe::App for EmbApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Ctrl+Plus, Ctrl+Minus and Ctrl+0: the text size of the settings
        // (egui's own keyboard zoom is off, so the setting keeps it).
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let step = ctx.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::Plus) || i.consume_key(Modifiers::COMMAND, Key::Equals) {
                Some(1)
            } else if i.consume_key(Modifiers::COMMAND, Key::Minus) {
                Some(-1)
            } else if i.consume_key(Modifiers::COMMAND, Key::Num0) {
                Some(0)
            } else {
                None
            }
        });
        if let Some(step) = step {
            let mut s = self.state.settings.clone();
            s.text_scale = crate::workspace::step_text_scale(s.text_scale, step);
            self.state.message = crate::i18n::fill("Text size: {} %", &[&format!("{:.0}", s.text_scale * 100.0)]);
            self.state.set_settings(s);
        }
        let language = self
            .state
            .lang_override
            .as_deref()
            .unwrap_or(&self.state.settings.language);
        crate::workspace::apply_theme_in(&ctx, &self.state.settings, language);
        if !self.started {
            // The ribbon needs the width of a full screen to show all panels.
            // (`ViewportBuilder::with_maximized` keeps the small size on Windows.)
            self.started = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        // Zoom to extents again when the window gets its full size at the start.
        let (size, time) = ctx.input(|i| (i.content_rect().size(), i.time));
        if size != self.last_size {
            self.last_size = size;
            if time < 2.0 {
                self.state.fit_requested = true;
            }
        }
        // Closing the window with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.state.close_now {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.state.request(Pending::Quit);
        }
        if self.state.close_now {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let title = self.state.window_title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        self.state.handle_shortcuts(&ctx);
        // Files dragged onto the window open as on the command line.
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            self.state.open_any(f.path());
        }
        self.state.backups_tick(std::time::Instant::now());
        ctx.request_repaint_after(crate::backup::HEARTBEAT);
        crate::backup::recovery_window(&ctx, &mut self.state);
        crate::file::dialogs(&ctx, &mut self.state);
        crate::export::dialog(&ctx, &mut self.state);

        let st = &mut self.state;
        let kiosk = st.kiosk.is_some();
        if kiosk {
            egui::CentralPanel::default().show(ui, |ui| crate::kiosk::screen(ui, st));
        } else {
            egui::Panel::top("ribbon").show(ui, |ui| crate::ribbon::show(ui, st));
            egui::Panel::top("property_bar").show(ui, |ui| crate::workspace::property_bar(ui, st));
            egui::Panel::top("design_tabs").show(ui, |ui| crate::tabs::strip(ui, st));
        }
        crate::workspace::options_window(&ctx, st);
        if !kiosk {
            egui::Panel::bottom("status").show(ui, |ui| st.status_bar(ui));
            egui::Panel::bottom("threads").show(ui, |ui| ui.horizontal(|ui| st.thread_strip(ui)));
            if st.player.shown {
                egui::Panel::bottom("player").show(ui, |ui| crate::player::bar(ui, st));
            }
        }
        let dt = f64::from(ctx.input(|i| i.unstable_dt));
        if st.player_tick(dt) {
            ctx.request_repaint();
        }
        crate::overview::window(&ctx, st);
        crate::output::windows(&ctx, st);
        crate::hoops::windows(&ctx, st);
        crate::hooping::preview_window(&ctx, st);
        crate::product::window(&ctx, st);
        crate::repeats::window(&ctx, st);
        crate::about::window(&ctx, st);
        crate::styles::windows(&ctx, st);
        crate::setup::sew_time_window(&ctx, st);
        crate::needles::window(&ctx, st);
        crate::quote::window(&ctx, st);
        crate::order::window(&ctx, st);
        crate::stock::window(&ctx, st);
        crate::scripts::window(&ctx, st);
        crate::corners::windows(&ctx, st);
        crate::simplify::window(&ctx, st);
        crate::editing::paste_special_window(&ctx, st);
        crate::help::window(&ctx, st);
        crate::palette::window(&ctx, st);
        crate::assets::window(&ctx, st);
        crate::lettering::windows(&ctx, st);
        crate::fonts::window(&ctx, st);
        crate::monogram::window(&ctx, st);
        crate::motifs::window(&ctx, st);
        crate::decor::windows(&ctx, st);
        crate::color_fills::window(&ctx, st);
        crate::quilting::window(&ctx, st);
        crate::cutwork::window(&ctx, st);
        crate::artwork::windows(&ctx, st);
        crate::wizard::window(&ctx, st);
        crate::teamnames::window(&ctx, st);
        crate::keyboard::window(&ctx, st);
        crate::alphabets::window(&ctx, st);
        crate::send::window(&ctx, st);
        crate::clipart::window(&ctx, st);
        // Jobs and the library scan go on while their tabs are hidden.
        if st.poll_jobs() | st.poll_library() | st.poll_outside_edit() {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
        // The split view is a dock tab right of the canvas while it is on.
        match (st.show_split, self.dock.find_tab(&Tab::Split)) {
            (true, None) => {
                if let Some(p) = self.dock.find_tab(&Tab::Canvas) {
                    self.dock[p.surface].split_right(p.node, 0.5, vec![Tab::Split]);
                }
            }
            (false, Some(p)) => {
                self.dock.remove_tab(p);
            }
            _ => {}
        }
        crate::viewtools::sync_panels(&mut self.dock, st);
        crate::views::sync_views(&mut self.dock, st);
        // The start page is a tab before the design.
        match (st.show_start, self.dock.find_tab(&Tab::Start)) {
            (true, None) => {
                if let Some(p) = self.dock.find_tab(&Tab::Canvas) {
                    self.dock[p.surface][p.node].insert_tab(0.into(), Tab::Start);
                }
            }
            (false, Some(p)) => {
                self.dock.remove_tab(p);
            }
            _ => {}
        }
        if !kiosk {
            egui::CentralPanel::no_frame().show(ui, |ui| {
                DockArea::new(&mut self.dock)
                    .style(Style::from_egui(ui.style().as_ref()))
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_inside(ui, st);
            });
        }
        // The program starts with the tab (Design or Library) that showed last.
        if let Some(shown) = st.library_drawn.take()
            && shown != st.settings.library_open
            && !st.focus_library
            && !st.focus_canvas
        {
            let mut s = st.settings.clone();
            s.library_open = shown;
            st.set_settings(s);
        }
        for (flag, tab) in [
            (&mut st.focus_properties, Tab::Properties),
            (&mut st.focus_threads, Tab::Threads),
            (&mut st.focus_stitch_list, Tab::StitchList),
            (&mut st.focus_info, Tab::DesignInfo),
            (&mut st.focus_styles, Tab::Styles),
            (&mut st.focus_canvas, Tab::Canvas),
            (&mut st.focus_library, Tab::Library),
            (&mut st.focus_clipart, Tab::Clipart),
        ] {
            if std::mem::take(flag)
                && let Some(path) = self.dock.find_tab(&tab)
            {
                let _ = self.dock.set_active_tab(path);
            }
        }
        if std::mem::take(&mut st.focus_start)
            && let Some(path) = self.dock.find_tab(&Tab::Start)
        {
            let _ = self.dock.set_active_tab(path);
        }
        if let Some(tab) = st.focus_tab.take()
            && let Some(path) = self.dock.find_tab(&tab)
        {
            let _ = self.dock.set_active_tab(path);
        }
    }
}
