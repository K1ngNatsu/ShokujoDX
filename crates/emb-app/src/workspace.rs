//! Workspace: units, settings, grid and guide snap, rulers, guides, the
//! Measure and Zoom tools, the Options window and the property bar.
//! See `docs/design/p2-plan.md` (Milestone 2).

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use emb_geom::{BBox, Point, math};
use emb_model::{Guide, HoopSelection, MergeKey};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::canvas::CanvasView;
use crate::draw::Tool;
use crate::i18n::tr;
use crate::manipulate::{DragOp, ViewMap};

pub const MM_PER_INCH: f64 = 25.4;
/// Width of the rulers, in screen points.
pub const RULER: f32 = 18.0;
/// A guide can be picked this far from its line, in screen points.
const GUIDE_HIT: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Units {
    #[default]
    Mm,
    Inch,
}

impl Units {
    pub fn suffix(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::Inch => "in",
        }
    }

    pub fn of_mm(self, mm: f64) -> f64 {
        match self {
            Units::Mm => mm,
            Units::Inch => mm / MM_PER_INCH,
        }
    }

    pub fn to_mm(self, v: f64) -> f64 {
        match self {
            Units::Mm => v,
            Units::Inch => v * MM_PER_INCH,
        }
    }

    fn decimals(self) -> usize {
        match self {
            Units::Mm => 1,
            Units::Inch => 3,
        }
    }

    /// A length as text, for example "12.5 mm" or "0.492 in".
    pub fn fmt(self, mm: f64) -> String {
        format!("{:.*} {}", self.decimals(), self.of_mm(mm), self.suffix())
    }
}

/// A drag value for a length in mm, shown and typed in `units`.
pub fn length_value(ui: &mut egui::Ui, mm: &mut f64, units: Units, min_mm: f64, max_mm: f64) -> egui::Response {
    let d = units.decimals();
    let sfx = units.suffix();
    ui.add(
        egui::DragValue::new(mm)
            .range(min_mm..=max_mm)
            .clamp_existing_to_range(false)
            .speed(units.to_mm(if units == Units::Mm { 0.1 } else { 0.005 }))
            .custom_formatter(move |v, _| format!("{:.*} {sfx}", d, units.of_mm(v)))
            .custom_parser(move |s| parse_length(s, units)),
    )
}

/// The value of a number or a length that the user types.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Typed {
    Number(f64),
    /// A length in mm.
    Length(f64),
}

/// The mm of one unit that the user can type after a number.
fn unit_mm(unit: &str) -> Option<f64> {
    Some(match unit {
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => 1.0,
        "cm" => 10.0,
        "m" => 1000.0,
        "in" | "inch" | "inches" | "\"" | "''" => MM_PER_INCH,
        "ft" | "foot" | "feet" | "'" => 12.0 * MM_PER_INCH,
        "pt" => MM_PER_INCH / 72.0,
        _ => return None,
    })
}

/// A parser of typed values: numbers with optional units, `+ - * /` (also
/// `x`) and parentheses.
struct TypedParser<'a> {
    s: &'a [u8],
    i: usize,
    /// The units of a number with no unit in a sum with a length.
    units: Units,
}

impl TypedParser<'_> {
    fn space(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.space();
        self.s.get(self.i).copied()
    }

    fn as_length(&self, v: Typed) -> f64 {
        match v {
            Typed::Length(mm) => mm,
            Typed::Number(n) => self.units.to_mm(n),
        }
    }

    fn sum(&mut self) -> Option<Typed> {
        let mut v = self.product()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            let w = self.product()?;
            let k = if op == b'+' { 1.0 } else { -1.0 };
            v = match (v, w) {
                (Typed::Number(a), Typed::Number(b)) => Typed::Number(a + k * b),
                _ => Typed::Length(self.as_length(v) + k * self.as_length(w)),
            };
        }
        Some(v)
    }

    fn product(&mut self) -> Option<Typed> {
        let mut v = self.factor()?;
        while let Some(op @ (b'*' | b'x' | b'/')) = self.peek() {
            self.i += 1;
            let w = self.factor()?;
            v = match (op, v, w) {
                (b'/', _, Typed::Number(0.0)) => return None,
                (b'/', Typed::Number(a), Typed::Number(b)) => Typed::Number(a / b),
                (b'/', Typed::Length(a), Typed::Number(b)) => Typed::Length(a / b),
                // A length over a length is a number.
                (b'/', Typed::Length(a), Typed::Length(b)) if b != 0.0 => Typed::Number(a / b),
                (b'/', ..) => return None,
                (_, Typed::Number(a), Typed::Number(b)) => Typed::Number(a * b),
                (_, Typed::Length(a), Typed::Number(b)) | (_, Typed::Number(b), Typed::Length(a)) => {
                    Typed::Length(a * b)
                }
                _ => return None,
            };
        }
        Some(v)
    }

    fn factor(&mut self) -> Option<Typed> {
        match self.peek()? {
            b'-' => {
                self.i += 1;
                Some(match self.factor()? {
                    Typed::Number(n) => Typed::Number(-n),
                    Typed::Length(l) => Typed::Length(-l),
                })
            }
            b'+' => {
                self.i += 1;
                self.factor()
            }
            b'(' => {
                self.i += 1;
                let v = self.sum()?;
                if self.peek()? != b')' {
                    return None;
                }
                self.i += 1;
                self.with_unit(v)
            }
            _ => {
                let mut n = self.number()?;
                // A fraction with no spaces is one number: "3/4 in".
                if self.s.get(self.i) == Some(&b'/') && self.s.get(self.i + 1).is_some_and(|c| c.is_ascii_digit()) {
                    self.i += 1;
                    let d = self.number()?;
                    if d == 0.0 {
                        return None;
                    }
                    n /= d;
                    return self.with_unit(Typed::Number(n));
                }
                // A mixed fraction: "1 1/2" or "1-1/2".
                let save = self.i;
                let n = match self.fraction() {
                    Some(f) => n + f,
                    None => {
                        self.i = save;
                        n
                    }
                };
                self.with_unit(Typed::Number(n))
            }
        }
    }

    /// "a/b" right after a whole number and a space or a dash.
    fn fraction(&mut self) -> Option<f64> {
        if !self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b'-')
        {
            return None;
        }
        self.i += 1;
        self.space();
        let a = self.number()?;
        if self.s.get(self.i) != Some(&b'/') {
            return None;
        }
        self.i += 1;
        let b = self.number()?;
        (b != 0.0).then_some(a / b)
    }

    fn number(&mut self) -> Option<f64> {
        self.space();
        let start = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()
    }

    /// The unit after a value, if there is one.
    fn with_unit(&mut self, v: Typed) -> Option<Typed> {
        self.space();
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphabetic() || self.s[self.i] == b'"' || self.s[self.i] == b'\'')
        {
            // "x" right after a value is a product, not a unit.
            if self.i == start && self.s[self.i] == b'x' {
                break;
            }
            self.i += 1;
        }
        let unit = std::str::from_utf8(&self.s[start..self.i]).ok()?.to_ascii_lowercase();
        if unit.is_empty() {
            return Some(v);
        }
        let k = unit_mm(&unit)?;
        match v {
            Typed::Number(n) => Some(Typed::Length(n * k)),
            Typed::Length(_) => None,
        }
    }
}

fn parse_typed(text: &str, units: Units) -> Option<Typed> {
    let t = text
        .trim()
        .replace(',', ".")
        .replace('×', "x")
        .replace('″', "\"")
        .replace('′', "'");
    let mut p = TypedParser {
        s: t.as_bytes(),
        i: 0,
        units,
    };
    let v = p.sum()?;
    p.space();
    (p.i == p.s.len()).then_some(v).filter(|v| match v {
        Typed::Number(n) | Typed::Length(n) => n.is_finite(),
    })
}

/// A length that the user types, in mm: a number in `units`, or a number
/// with its own unit (`2 in`, `50mm`, `1 1/2"`, `3 cm`), with `+ - * /`
/// (`2 in + 3 mm`, `120/4`).
pub fn parse_length(text: &str, units: Units) -> Option<f64> {
    match parse_typed(text, units)? {
        Typed::Length(mm) => Some(mm),
        Typed::Number(n) => Some(units.to_mm(n)),
    }
}

/// A number or an angle in degrees that the user types (`°` or `deg` may
/// follow), with `+ - * /`.
pub fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_suffix('°').or_else(|| t.strip_suffix("deg")).unwrap_or(t);
    match parse_typed(t, Units::Mm)? {
        Typed::Number(n) => Some(n),
        Typed::Length(_) => None,
    }
}

/// Shape settings of new objects (the property bar edits them). The stitch
/// settings of new objects are the current properties of the design
/// (`Design::current`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    /// Width of a new Column C satin and of a new ring.
    pub column_width_mm: f64,
    /// Corner radius of a new rounded rectangle.
    pub corner_radius_mm: f64,
    /// Points of a new star.
    pub star_points: u32,
    /// Cell size of a new cross-stitch grid, in mm.
    pub cross_cell_mm: f64,
    /// The stone, the gap and the fill of new rhinestone objects.
    pub stone: emb_model::Stone,
    pub stone_gap_mm: f64,
    pub stone_fill: emb_model::BlingFill,
    /// The angle of the stitches to a new direction line, in degrees (90°:
    /// square to the line).
    pub direction_angle_deg: f64,
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults {
            column_width_mm: 3.0,
            corner_radius_mm: 5.0,
            star_points: 5,
            cross_cell_mm: emb_model::cross::DEFAULT_CELL_MM,
            stone: Default::default(),
            stone_gap_mm: emb_model::bling::DEFAULT_GAP_MM,
            stone_fill: Default::default(),
            direction_angle_deg: 90.0,
        }
    }
}

/// User settings. They are not part of a design; the app saves them in the
/// user's configuration folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub units: Units,
    pub show_grid: bool,
    /// Distance between grid lines, in mm.
    pub grid_mm: f64,
    /// Every n-th grid line is a major line.
    pub grid_major: u32,
    pub snap_grid: bool,
    pub show_rulers: bool,
    pub snap_guides: bool,
    /// Points snap to the nodes of other objects.
    pub snap_objects: bool,
    /// Snap distance to guides, in screen points.
    pub snap_px: f32,
    pub nudge_mm: f64,
    pub nudge_large_mm: f64,
    pub defaults: Defaults,
    /// The ribbon tab that shows, and whether the ribbon panels are hidden.
    /// The tab is not saved: the program starts on the Digitize tab (owner
    /// request of 2026-10-26).
    #[serde(skip)]
    pub ribbon_tab: crate::ribbon::RibbonTab,
    pub ribbon_collapsed: bool,
    /// Which files Output design makes.
    pub output: crate::output::OutputProfile,
    /// The template that New uses (a name in the templates folder). `None`:
    /// the built-in settings.
    pub template: Option<String>,
    /// Minutes between automatic backups. 0: no backups.
    pub backup_minutes: u32,
    /// Sew times that the user set, by machine id.
    pub sew_times: std::collections::BTreeMap<String, emb_model::SewTimes>,
    /// Screen calibration: real mm on screen per mm at 100 % zoom (1.0:
    /// the operating system value).
    pub screen_calibration: f64,
    /// Dark mode (light text on dark panels); `false`: light mode.
    pub dark_mode: bool,
    /// High contrast: black and white panels with strong lines (in the dark
    /// or the light mode).
    pub high_contrast: bool,
    /// The size of all text and controls (1.0: the size of the operating system).
    pub text_scale: f32,
    /// The language of the program: "en" or the code of a language file
    /// (`crate::i18n`).
    pub language: String,
    /// The designs and machine files that were opened or saved last, the
    /// last one first (File > Recent designs).
    pub recent: Vec<std::path::PathBuf>,
    /// The ids of the hoops that the user has (Design > My hoops).
    pub my_hoops: Vec<String>,
    /// The folders of the design library (the Library tab).
    pub library_folders: Vec<std::path::PathBuf>,
    /// The Library tab showed (not the canvas) when the program ended; it
    /// shows again at the next start.
    pub library_open: bool,
    /// Send to USB: the stick and folder of each machine (by machine id).
    pub send_targets: std::collections::BTreeMap<String, crate::send::SendTarget>,
    /// Paste, duplicate and Reshape settings.
    pub editing: crate::editing::EditSettings,
    /// The factor of Zoom in and Zoom out (see `viewtools::ZOOM_STEPS`).
    pub zoom_step: f64,
    /// The panels that the user closed (View > Panels opens them again).
    pub hidden_panels: Vec<crate::app::Tab>,
    /// Styles for all designs (Design > Favorites), at most
    /// `viewtools::MAX_FAVORITES`.
    pub favorite_styles: Vec<emb_model::Style>,
    /// The user's own colors of the grid, guides, hoop and selection.
    pub display: crate::viewtools::DisplayColors,
    /// The thread look: flat or 3D, the light and the fabric texture.
    pub look: crate::viewtools::LookSettings,
    /// The start page shows when the program starts.
    pub start_page: bool,
    /// Simple mode: fewer tools and larger buttons (P10 Milestone 5).
    pub simple_mode: bool,
    /// The trims of DST and TAP files that the user set, by machine id.
    pub trims: std::collections::BTreeMap<String, crate::setup::TrimSettings>,
    /// The needles of each multi-needle machine and the threads on them,
    /// by machine id (ADR 0063).
    pub needles: std::collections::BTreeMap<String, emb_model::NeedleSetup>,
    /// The user's own stones (size and color), for all designs.
    pub my_stones: Vec<emb_model::Stone>,
    /// The layers of thread over which the density map shows red and
    /// Design info lists the place (P11 Milestone 1).
    pub density_limit: f64,
    /// The prices of the thread, for the cost in Design info.
    pub thread_prices: emb_model::thread_use::ThreadPrices,
    /// The price lists of the Quote window (ADR 0064).
    pub price_lists: Vec<emb_model::quote::PriceList>,
    /// The threads of the shop (Design > Thread Stock, ADR 0066).
    pub thread_stock: emb_model::stock::ThreadStock,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            units: Units::Mm,
            show_grid: true,
            grid_mm: 10.0,
            grid_major: 5,
            snap_grid: false,
            show_rulers: true,
            snap_guides: true,
            snap_objects: false,
            snap_px: 8.0,
            nudge_mm: 0.5,
            nudge_large_mm: 5.0,
            defaults: Defaults::default(),
            ribbon_tab: Default::default(),
            ribbon_collapsed: false,
            output: Default::default(),
            template: None,
            backup_minutes: 2,
            sew_times: Default::default(),
            screen_calibration: 1.0,
            dark_mode: true,
            high_contrast: false,
            text_scale: 1.0,
            language: "en".into(),
            recent: Vec::new(),
            my_hoops: Vec::new(),
            library_folders: Vec::new(),
            library_open: false,
            send_targets: Default::default(),
            editing: Default::default(),
            zoom_step: 1.5,
            hidden_panels: Vec::new(),
            favorite_styles: Vec::new(),
            display: Default::default(),
            look: Default::default(),
            start_page: true,
            simple_mode: false,
            trims: Default::default(),
            needles: Default::default(),
            my_stones: Vec::new(),
            density_limit: emb_render::density::DEFAULT_LIMIT,
            thread_prices: Default::default(),
            price_lists: vec![Default::default()],
            thread_stock: Default::default(),
        }
    }
}

/// The text and control sizes that the Options offer.
pub const TEXT_SCALES: [f32; 6] = [0.9, 1.0, 1.15, 1.3, 1.5, 2.0];

/// The next text size of [`TEXT_SCALES`] up (`step` 1) or down (-1), or 100 % (0).
pub fn step_text_scale(now: f32, step: i32) -> f32 {
    match step {
        0 => 1.0,
        s if s > 0 => TEXT_SCALES.iter().copied().find(|k| *k > now + 1e-3).unwrap_or(now),
        _ => TEXT_SCALES
            .iter()
            .rev()
            .copied()
            .find(|k| *k < now - 1e-3)
            .unwrap_or(now),
    }
}

/// Sets the theme, the high contrast, the text size and `language` (the
/// language of the settings, or `--lang` on the command line), where they
/// are not set yet.
pub fn apply_theme_in(ctx: &egui::Context, s: &Settings, language: &str) {
    let want = if s.dark_mode {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    };
    if ctx.options(|o| o.theme_preference) != want {
        ctx.set_theme(want);
    }
    let id = egui::Id::new("high_contrast_applied");
    if ctx.data(|d| d.get_temp::<bool>(id)) != Some(s.high_contrast) {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let v = if s.high_contrast {
                high_contrast(theme)
            } else {
                theme.default_visuals()
            };
            ctx.set_visuals_of(theme, v);
        }
        ctx.data_mut(|d| d.insert_temp(id, s.high_contrast));
    }
    let scale = s.text_scale.clamp(0.5, 3.0);
    if (ctx.zoom_factor() - scale).abs() > 1e-4 {
        ctx.set_zoom_factor(scale);
    }
    if crate::i18n::current() != language {
        crate::i18n::set_language(language);
    }
}

/// High contrast colors: black and white panels, white or black text and
/// lines, and a yellow (dark) or blue (light) selection and focus.
pub fn high_contrast(theme: egui::Theme) -> egui::Visuals {
    use egui::{Color32, Stroke};
    let dark = theme == egui::Theme::Dark;
    let mut v = theme.default_visuals();
    let (bg, fg, mark) = if dark {
        (Color32::BLACK, Color32::WHITE, Color32::from_rgb(255, 230, 0))
    } else {
        (Color32::WHITE, Color32::BLACK, Color32::from_rgb(0, 60, 200))
    };
    let hover = if dark {
        Color32::from_gray(60)
    } else {
        Color32::from_gray(215)
    };
    v.override_text_color = Some(fg);
    v.panel_fill = bg;
    v.window_fill = bg;
    v.extreme_bg_color = bg;
    v.faint_bg_color = bg;
    v.code_bg_color = bg;
    v.window_stroke = Stroke::new(2.0, fg);
    v.hyperlink_color = mark;
    v.selection.bg_fill = mark;
    v.selection.stroke = Stroke::new(2.0, bg);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive] {
        w.bg_fill = bg;
        w.weak_bg_fill = bg;
        w.bg_stroke = Stroke::new(1.5, fg);
        w.fg_stroke = Stroke::new(1.5, fg);
    }
    for w in [&mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.bg_fill = hover;
        w.weak_bg_fill = hover;
        w.bg_stroke = Stroke::new(2.0, mark);
        w.fg_stroke = Stroke::new(2.0, fg);
    }
    v
}

/// The folder of the program in the user's configuration folder: `ShokujoDX`
/// (on Linux `shokujodx`, as the other Linux names).
const FOLDER: &str = if cfg!(any(windows, target_os = "macos")) {
    "ShokujoDX"
} else {
    "shokujodx"
};
/// The folders of the earlier names, newest first: Ariadne (ADR 0059) and
/// the versions before it.
const OLD_FOLDERS: [&str; 2] = [
    if cfg!(any(windows, target_os = "macos")) {
        "Ariadne"
    } else {
        "ariadne-embroidery"
    },
    "emb-digitizer",
];

/// True if the folder `dir` of an earlier name is ours. Other programs can
/// use the name Ariadne, so its folder must have our settings file; the
/// name `emb-digitizer` is ours only.
fn ours(dir: &std::path::Path) -> bool {
    dir.is_dir()
        && (dir.ends_with("emb-digitizer")
            || std::fs::read_to_string(dir.join("settings.json")).is_ok_and(|s| s.contains("\"nudge_large_mm\"")))
}

/// The folder of the program in the configuration folder `base`. The
/// folder of an earlier name moves to the new name once, with the
/// settings, backups, crash reports and the user's assets; if the move
/// fails, the old folder stays in use.
pub fn program_folder(base: &std::path::Path) -> std::path::PathBuf {
    let new = base.join(FOLDER);
    if new.exists() {
        return new;
    }
    if let Some(old) = OLD_FOLDERS.iter().map(|f| base.join(f)).find(|d| ours(d))
        && std::fs::rename(&old, &new).is_err()
    {
        return old;
    }
    new
}

/// The settings file: `settings.json` in the folder of the program
/// ([`program_folder`]) in the user's configuration folder.
pub fn settings_path() -> Option<std::path::PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
    }?;
    Some(program_folder(&base).join("settings.json"))
}

impl Settings {
    /// Reads the settings file. A missing or bad file gives the defaults.
    pub fn load() -> Settings {
        settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let p = settings_path().ok_or("no configuration folder")?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&p, s).map_err(|e| format!("{}: {e}", p.display()))
    }
}

impl AppState {
    /// Snaps a design point to the grid and the guides, as the settings say.
    /// Guides win over the grid.
    pub fn snap(&self, map: &ViewMap, p: Point) -> Point {
        let s = &self.settings;
        let d = self.doc.design();
        let rot = d.rotation_transform();
        let mut v = rot.apply(p);
        if s.snap_grid && s.grid_mm > 0.0 {
            let c = d.hoop.center;
            let g = s.grid_mm;
            v = Point::new(c.x + ((v.x - c.x) / g).round() * g, c.y + ((v.y - c.y) / g).round() * g);
        }
        if s.snap_objects {
            // The nearest node of an object that is not selected (a moved
            // object does not snap to itself).
            let tol = f64::from(s.snap_px) / map.view.scale();
            let raw = rot.apply(p);
            let near = d
                .objects()
                .iter()
                .filter(|o| !self.selection.contains(o.id))
                .flat_map(|o| o.kind.paths())
                .flat_map(|path| path.cmds.iter())
                .filter_map(|c| match *c {
                    emb_geom::path::PathCmd::MoveTo(q)
                    | emb_geom::path::PathCmd::LineTo(q)
                    | emb_geom::path::PathCmd::CubicTo(_, _, q) => Some(rot.apply(q)),
                    emb_geom::path::PathCmd::Close => None,
                })
                .map(|q| (q.distance(raw), q))
                .filter(|(dist, _)| *dist <= tol)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, q)) = near {
                return rot.inverse().map_or(p, |inv| inv.apply(q));
            }
        }
        if s.snap_guides {
            let tol = f64::from(s.snap_px) / map.view.scale();
            let raw = rot.apply(p);
            for g in &d.guides {
                if g.vertical && (raw.x - g.pos).abs() <= tol {
                    v.x = g.pos;
                } else if !g.vertical && (raw.y - g.pos).abs() <= tol {
                    v.y = g.pos;
                }
            }
        }
        rot.inverse().map_or(p, |inv| inv.apply(v))
    }

    /// The hoop field, or the design extents if there is no hoop.
    pub fn hoop_box(&self) -> Option<BBox> {
        match &self.doc.design().hoop.selection {
            HoopSelection::Preset { .. } => self.hoop_outline(0.0).and_then(|p| p.control_bbox()),
            HoopSelection::OpenField => self.fit_box(),
        }
    }

    pub fn zoom_by(&mut self, factor: f64) {
        self.view.zoom = (self.view.zoom * factor).clamp(crate::canvas::MIN_ZOOM, crate::canvas::MAX_ZOOM);
    }

    /// Changes a setting and saves the settings file.
    pub fn set_settings(&mut self, s: Settings) {
        if s != self.settings {
            crate::canvas::set_calibration(s.screen_calibration);
            if s.sew_times != self.settings.sew_times
                || s.density_limit != self.settings.density_limit
                || s.thread_prices != self.settings.thread_prices
                || s.needles != self.settings.needles
                || s.thread_stock != self.settings.thread_stock
            {
                // Design info shows the sew time (with the thread changes of
                // the needles), the places over the density limit and the cost
                // of the thread.
                self.info_cache = None;
            }
            self.settings = s;
            if self.save_settings
                && let Err(e) = self.settings.save()
            {
                self.message = crate::i18n::fill("Settings not saved: {}", &[&e]);
            }
        }
    }
}

/// Steps for ruler labels, in display units.
const STEPS: [f64; 12] = [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0];

/// The smallest step (display units) whose screen size is at least `min_px`.
fn ruler_step(units: Units, scale: f64, min_px: f64) -> f64 {
    for base in [1.0, 10.0, 100.0] {
        for s in STEPS {
            if units.to_mm(s * base) * scale >= min_px {
                return s * base;
            }
        }
    }
    1000.0
}

/// Draws the rulers along the top and left edges. Labels are measured from the hoop center.
pub fn draw_rulers(painter: &egui::Painter, rect: Rect, st: &AppState, dark: bool) {
    if !st.settings.show_rulers {
        return;
    }
    let units = st.settings.units;
    let view = st.view;
    let scale = view.scale();
    let (bg, ink) = if dark {
        (Color32::from_gray(40), Color32::from_gray(180))
    } else {
        (Color32::from_gray(236), Color32::from_gray(70))
    };
    let top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + RULER));
    let left = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + RULER, rect.max.y));
    painter.rect_filled(top, 0.0, bg);
    painter.rect_filled(left, 0.0, bg);
    let origin = st.doc.design().hoop.center;
    let step = ruler_step(units, scale, 60.0);
    let minor = step / 5.0;
    let font = FontId::monospace(9.0);
    let vis = view.visible(rect);
    let tick = |vertical_ruler: bool| {
        let (lo, hi, o) = if vertical_ruler {
            (vis.min.y, vis.max.y, origin.y)
        } else {
            (vis.min.x, vis.max.x, origin.x)
        };
        let (lo_u, hi_u) = (units.of_mm(lo - o), units.of_mm(hi - o));
        let mut k = (lo_u / minor).floor() as i64;
        while (k as f64) * minor <= hi_u {
            let u = k as f64 * minor;
            let mm = o + units.to_mm(u);
            let major = k.rem_euclid(5) == 0;
            let len = if major { RULER * 0.6 } else { RULER * 0.25 };
            if vertical_ruler {
                let y = view.to_screen(rect, Point::new(0.0, mm)).y;
                if y > rect.min.y + RULER {
                    painter.line_segment(
                        [Pos2::new(left.max.x - len, y), Pos2::new(left.max.x, y)],
                        Stroke::new(1.0, ink),
                    );
                    if major {
                        painter.text(
                            Pos2::new(left.min.x + 1.0, y + 1.0),
                            Align2::LEFT_TOP,
                            label(u),
                            font.clone(),
                            ink,
                        );
                    }
                }
            } else {
                let x = view.to_screen(rect, Point::new(mm, 0.0)).x;
                if x > rect.min.x + RULER {
                    painter.line_segment(
                        [Pos2::new(x, top.max.y - len), Pos2::new(x, top.max.y)],
                        Stroke::new(1.0, ink),
                    );
                    if major {
                        painter.text(
                            Pos2::new(x + 2.0, top.min.y),
                            Align2::LEFT_TOP,
                            label(u),
                            font.clone(),
                            ink,
                        );
                    }
                }
            }
            k += 1;
        }
    };
    tick(false);
    tick(true);
    painter.text(
        rect.min + Vec2::splat(2.0),
        Align2::LEFT_TOP,
        units.suffix(),
        FontId::proportional(9.0),
        ink,
    );
}

fn label(u: f64) -> String {
    let s = format!("{u:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Draws the grid with the spacing from the settings, relative to the hoop center.
pub fn draw_grid(painter: &egui::Painter, rect: Rect, st: &AppState, dark: bool) {
    let s = &st.settings;
    if !s.show_grid || s.grid_mm <= 0.0 {
        return;
    }
    let view = st.view;
    let scale = view.scale();
    let major_every = s.grid_major.max(1);
    // Too dense to see: show only the major lines, or nothing.
    let (step, every) = if s.grid_mm * scale >= 6.0 {
        (s.grid_mm, major_every)
    } else if s.grid_mm * f64::from(major_every) * scale >= 6.0 {
        (s.grid_mm * f64::from(major_every), 1)
    } else {
        return;
    };
    let (minor_c, major_c) = s.display.grid(dark);
    let o = st.doc.design().hoop.center;
    let vis = view.visible(rect);
    let mut k = ((vis.min.x - o.x) / step).floor() as i64;
    while o.x + k as f64 * step <= vis.max.x {
        let x = view.to_screen(rect, Point::new(o.x + k as f64 * step, 0.0)).x;
        let c = if k.rem_euclid(i64::from(every)) == 0 {
            major_c
        } else {
            minor_c
        };
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, c),
        );
        k += 1;
    }
    let mut k = ((vis.min.y - o.y) / step).floor() as i64;
    while o.y + k as f64 * step <= vis.max.y {
        let y = view.to_screen(rect, Point::new(0.0, o.y + k as f64 * step)).y;
        let c = if k.rem_euclid(i64::from(every)) == 0 {
            major_c
        } else {
            minor_c
        };
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, c),
        );
        k += 1;
    }
}

pub fn draw_guides(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let c = st.settings.display.guides();
    for g in &st.doc.design().guides {
        if g.vertical {
            let x = st.view.to_screen(rect, Point::new(g.pos, 0.0)).x;
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, c),
            );
        } else {
            let y = st.view.to_screen(rect, Point::new(0.0, g.pos)).y;
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, c),
            );
        }
    }
}

fn in_top_ruler(rect: Rect, p: Pos2) -> bool {
    p.y < rect.min.y + RULER && p.x > rect.min.x + RULER
}

fn in_left_ruler(rect: Rect, p: Pos2) -> bool {
    p.x < rect.min.x + RULER && p.y > rect.min.y + RULER
}

fn guide_at(st: &AppState, rect: Rect, p: Pos2) -> Option<usize> {
    st.doc.design().guides.iter().position(|g| {
        if g.vertical {
            (st.view.to_screen(rect, Point::new(g.pos, 0.0)).x - p.x).abs() <= GUIDE_HIT
        } else {
            (st.view.to_screen(rect, Point::new(0.0, g.pos)).y - p.y).abs() <= GUIDE_HIT
        }
    })
}

/// Rulers and guides: drag from a ruler to make a guide, drag a guide to
/// move it, drop it on a ruler to delete it. Returns `true` while this
/// input is in use, so the tool does not get it.
pub fn guide_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState) -> bool {
    let rect = response.rect;
    let view: CanvasView = st.view;
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(o) = ui.input(|i| i.pointer.press_origin())
    {
        let start = if st.settings.show_rulers && in_top_ruler(rect, o) {
            Some((None, false))
        } else if st.settings.show_rulers && in_left_ruler(rect, o) {
            Some((None, true))
        } else if matches!(st.tool, Tool::Select | Tool::Reshape) {
            guide_at(st, rect, o).map(|i| (Some(i), st.doc.design().guides[i].vertical))
        } else {
            None
        };
        if let Some((index, vertical)) = start {
            st.drag_counter += 1;
            let key = MergeKey::new(format!("guide-{}", st.drag_counter));
            let index = match index {
                Some(i) => i,
                None => {
                    let mm = view.to_mm(rect, o);
                    let pos = if vertical { mm.x } else { mm.y };
                    st.doc.edit_merged("Add guide", key.clone(), |d| {
                        d.guides.push(Guide { vertical, pos });
                        d.guides.len() - 1
                    })
                }
            };
            st.drag = Some(DragOp::Guide { index, key });
        }
    }
    let Some(DragOp::Guide { index, key }) = st.drag.clone() else {
        // A guide under the pointer shows a move cursor.
        if let Some(p) = response.hover_pos()
            && let Some(i) = guide_at(st, rect, p)
            && matches!(st.tool, Tool::Select | Tool::Reshape)
        {
            let v = st.doc.design().guides[i].vertical;
            ui.ctx().set_cursor_icon(if v {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::ResizeVertical
            });
        }
        return false;
    };
    if let Some(p) = response.interact_pointer_pos() {
        let mm = view.to_mm(rect, p);
        st.doc.edit_merged("Move guide", key.clone(), |d| {
            if let Some(g) = d.guides.get_mut(index) {
                g.pos = if g.vertical { mm.x } else { mm.y };
            }
        });
    }
    if response.drag_stopped() {
        let drop = response.interact_pointer_pos();
        st.drag = None;
        if let Some(p) = drop
            && (in_top_ruler(rect, p) || in_left_ruler(rect, p) || !rect.contains(p))
        {
            st.doc.edit_merged("Delete guide", key, |d| {
                if index < d.guides.len() {
                    d.guides.remove(index);
                }
            });
        }
        st.doc.end_merge();
    }
    true
}

/// The Measure tool: drag to measure a distance and an angle.
pub fn measure_input(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(p) = response.ctx.input(|i| i.pointer.press_origin())
    {
        let a = st.snap(map, map.to_design(p));
        st.measure = Some((a, a));
    }
    if response.dragged_by(egui::PointerButton::Primary)
        && let (Some(p), Some((a, _))) = (response.interact_pointer_pos(), st.measure)
    {
        let b = st.snap(map, map.to_design(p));
        st.measure = Some((a, b));
        st.message = measure_text(st.settings.units, a, b);
    }
}

pub fn measure_text(units: Units, a: Point, b: Point) -> String {
    let d = b - a;
    // Screen angle: 0° is to the right, positive is counterclockwise on screen (Y down).
    let angle = math::to_degrees(math::atan2(-d.y, d.x));
    format!(
        "{}  (dx {}, dy {}, {:.1}°)",
        units.fmt(d.length()),
        units.fmt(d.x.abs()),
        units.fmt(d.y.abs()),
        angle
    )
}

pub fn draw_measure(painter: &egui::Painter, map: &ViewMap, st: &AppState, color: Color32, text_bg: Color32) {
    let Some((a, b)) = st.measure else { return };
    let (sa, sb) = (map.to_screen(a), map.to_screen(b));
    painter.line_segment([sa, sb], Stroke::new(1.5, color));
    for p in [sa, sb] {
        painter.circle_stroke(p, 3.0, Stroke::new(1.5, color));
    }
    let text = measure_text(st.settings.units, a, b);
    let galley = painter.layout_no_wrap(text, FontId::proportional(12.0), color);
    let pos = sb + Vec2::new(10.0, 10.0);
    painter.rect_filled(Rect::from_min_size(pos, galley.size()).expand(3.0), 3.0, text_bg);
    painter.galley(pos, galley, color);
}

/// The Zoom tool: drag a box to zoom to it; click to zoom in, Shift+click to zoom out.
pub fn zoom_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState) {
    let rect = response.rect;
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(o) = ui.input(|i| i.pointer.press_origin())
    {
        st.drag = Some(DragOp::Marquee {
            start: o,
            current: o,
            additive: false,
        });
    }
    if let (Some(DragOp::Marquee { current, .. }), Some(p)) = (&mut st.drag, response.interact_pointer_pos()) {
        *current = p;
    }
    if response.drag_stopped()
        && let Some(DragOp::Marquee { start, current, .. }) = st.drag.take()
    {
        let (a, b) = (st.view.to_mm(rect, start), st.view.to_mm(rect, current));
        if let Some(bb) = BBox::from_points([a, b])
            && bb.width().max(bb.height()) * st.view.scale() > 4.0
        {
            st.push_view();
            st.view.fit(rect, bb);
        }
    }
    if response.clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        let f = if ui.input(|i| i.modifiers.shift) { 0.5 } else { 2.0 };
        st.push_view();
        st.view.zoom_at(rect, p, f);
    }
}

/// The Options window.
pub fn options_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_options {
        return;
    }
    let mut s = st.settings.clone();
    let mut open = true;
    egui::Window::new(tr("Options")).id(egui::Id::new("Options"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            egui::Grid::new("options")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr("Appearance"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.dark_mode, true, tr("Dark"));
                        ui.radio_value(&mut s.dark_mode, false, tr("Light"));
                        ui.checkbox(&mut s.high_contrast, tr("High contrast"))
                            .on_hover_text(tr("Black and white panels with strong lines, for low vision."));
                    });
                    ui.end_row();
                    ui.label(tr("Text size"));
                    egui::ComboBox::from_id_salt("text_scale")
                        .selected_text(format!("{:.0} %", s.text_scale * 100.0))
                        .show_ui(ui, |ui| {
                            for k in TEXT_SCALES {
                                ui.selectable_value(&mut s.text_scale, k, format!("{:.0} %", k * 100.0));
                            }
                        })
                        .response
                        .on_hover_text(tr("The size of all text and buttons. Ctrl+Plus and Ctrl+Minus change it too."));
                    ui.end_row();
                    ui.label(tr("Language"));
                    let name = |code: &str| {
                        crate::i18n::languages()
                            .iter()
                            .find(|l| l.code == code)
                            .map_or_else(|| "English".to_string(), |l| l.name.clone())
                    };
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(name(&s.language))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.language, "en".to_string(), tr("English"));
                            for l in crate::i18n::languages() {
                                ui.selectable_value(&mut s.language, l.code.clone(), l.name.as_str());
                            }
                        });
                    ui.end_row();
                    ui.label(tr("Tools"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.simple_mode, false, tr("All tools"));
                        ui.radio_value(&mut s.simple_mode, true, tr("Simple mode (fewer tools, larger buttons)"));
                    });
                    ui.end_row();
                    ui.label(tr("Units"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.units, Units::Mm, tr("Millimeters"));
                        ui.radio_value(&mut s.units, Units::Inch, tr("Inches"));
                    });
                    ui.end_row();
                    ui.label(tr("Grid"));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.show_grid, tr("Show"));
                        ui.checkbox(&mut s.snap_grid, tr("Snap to Grid"));
                    });
                    ui.end_row();
                    ui.label(tr("Grid spacing"));
                    length_value(ui, &mut s.grid_mm, s.units, 0.5, 100.0);
                    ui.end_row();
                    ui.label(tr("Major line every"));
                    ui.add(egui::DragValue::new(&mut s.grid_major).range(1..=20).clamp_existing_to_range(false).suffix(" lines"));
                    ui.end_row();
                    ui.label(tr("Rulers and guides"));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.show_rulers, tr("Show rulers"));
                        ui.checkbox(&mut s.snap_guides, tr("Snap to Guides"));
                    });
                    ui.end_row();
                    ui.label(tr("Snap distance"));
                    ui.add(egui::DragValue::new(&mut s.snap_px).range(2.0..=30.0).clamp_existing_to_range(false).suffix(" px"));
                    ui.end_row();
                    ui.label(tr("Arrow key nudge"));
                    length_value(ui, &mut s.nudge_mm, s.units, 0.1, 50.0);
                    ui.end_row();
                    ui.label(tr("Shift + arrow nudge"));
                    length_value(ui, &mut s.nudge_large_mm, s.units, 0.1, 100.0);
                    ui.end_row();
                    ui.label(tr("Automatic backup"));
                    ui.add(
                        egui::DragValue::new(&mut s.backup_minutes)
                            .range(0..=60).clamp_existing_to_range(false)
                            .custom_formatter(|v, _| {
                                if v < 0.5 {
                                    tr("off").into()
                                } else {
                                    crate::i18n::fill("every {} min", &[&format!("{v:.0}")])
                                }
                            }),
                    )
                    .on_hover_text(tr("Designs with unsaved changes are saved to a backup folder. After a crash the app offers to recover them."));
                    ui.end_row();
                    ui.label(tr("Hoop safety margin"));
                    let mut m = st.margin_mm;
                    if length_value(ui, &mut m, s.units, 0.0, 30.0).changed() {
                        st.margin_mm = m;
                    }
                    ui.end_row();
                });
            ui.separator();
            let units = s.units;
            crate::editing::options_ui(ui, &mut s.editing, units);
            ui.horizontal(|ui| {
                ui.label(tr("Zoom in and out by"));
                for f in crate::viewtools::ZOOM_STEPS {
                    ui.radio_value(&mut s.zoom_step, f, format!("{f}×"));
                }
            });
            ui.separator();
            crate::viewtools::display_colors_ui(ui, &mut s.display);
            ui.separator();
            crate::density::options_ui(ui, &mut s);
            ui.separator();
            crate::setup::calibration_ui(ui, &mut s);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("Reset to defaults")).clicked() {
                    let defaults = s.defaults.clone();
                    s = Settings::default();
                    s.defaults = defaults;
                }
                ui.weak(tr("Changes apply at once and are saved for the next start."));
            });
        });
    // A language chosen here wins over `--lang`.
    if s.language != st.settings.language {
        st.lang_override = None;
    }
    st.set_settings(s);
    if !open {
        st.show_options = false;
    }
}

/// Edits one length default; returns the new value.
fn bar_length(ui: &mut egui::Ui, label: &str, mm: f64, units: Units, min: f64, max: f64) -> f64 {
    let mut v = mm;
    ui.label(label);
    length_value(ui, &mut v, units, min, max);
    v
}

/// A length setting of new objects with its plain-language word.
fn bar_param(
    ui: &mut egui::Ui,
    label: &str,
    mm: f64,
    units: Units,
    range: (f64, f64),
    word: (fn(f64) -> &'static str, &str),
) -> f64 {
    let v = bar_length(ui, label, mm, units, range.0, range.1);
    ui.weak(word.0(v)).on_hover_text(word.1);
    v
}

/// The property bar: the settings of the active tool, for new objects.
/// The stitch settings are the current properties of the design; a change
/// is an undo step.
pub fn property_bar(ui: &mut egui::Ui, st: &mut AppState) {
    use crate::words;
    let mut s = st.settings.clone();
    let mut cur = st.doc.design().current.clone();
    let u = s.units;
    ui.horizontal(|ui| {
        ui.strong(st.tool.label());
        ui.separator();
        let d = &mut s.defaults;
        let p = &mut cur;
        match st.tool {
            Tool::Select | Tool::Reshape => selection_fields(ui, st),
            t @ (Tool::Stone | Tool::StoneRun | Tool::StoneFill) => {
                crate::bling::bar(
                    ui,
                    &mut d.stone,
                    &mut s.my_stones,
                    &mut d.stone_gap_mm,
                    &mut d.stone_fill,
                    t,
                );
                crate::bling::clearance_ui(ui, &mut s.output.stone_clearance_mm);
            }
            t @ (Tool::CrossStitch | Tool::CrossErase | Tool::Backstitch) => {
                let selected = st.selected_cross_mm();
                crate::cross::bar(ui, &mut st.cross, &mut d.cross_cell_mm, t, selected);
            }
            Tool::Measure => {
                ui.label(tr("Drag to measure. Snap to grid and guides applies."));
            }
            Tool::DirectionLine => {
                ui.label(tr("Stitch angle to the line")).on_hover_text(tr(
                    "90°: the stitches go square across the line. Other angles slant them.",
                ));
                ui.add(
                    egui::DragValue::new(&mut d.direction_angle_deg)
                        .range(1.0..=179.0)
                        .clamp_existing_to_range(false)
                        .suffix("°"),
                );
            }
            Tool::Zoom => {
                ui.label(tr("Drag a box to zoom to it. Click: zoom in. Shift+click: zoom out."));
            }
            t if t.makes_running() => {
                p.running.stitch_length_mm = bar_param(
                    ui,
                    tr("Stitch length"),
                    p.running.stitch_length_mm,
                    u,
                    (0.5, 12.0),
                    (words::length, tr(words::LENGTH_HELP)),
                );
                crate::draw::run_type_combo(ui, &mut p.running);
            }
            Tool::Applique => {
                p.applique.cover_width_mm = bar_length(ui, tr("Cover width"), p.applique.cover_width_mm, u, 1.0, 8.0);
                p.applique.cover_satin.spacing_mm = bar_param(
                    ui,
                    tr("Density"),
                    p.applique.cover_satin.spacing_mm,
                    u,
                    (0.2, 1.5),
                    (words::density, tr(words::DENSITY_HELP)),
                );
                ui.checkbox(&mut p.applique.placement, tr("Placement line"));
                ui.checkbox(&mut p.applique.trim_stop, tr("Stop to cut"));
            }
            t if t.makes_satin() => {
                p.satin.spacing_mm = bar_param(
                    ui,
                    tr("Density"),
                    p.satin.spacing_mm,
                    u,
                    (0.2, 1.5),
                    (words::density, tr(words::DENSITY_HELP)),
                );
                if matches!(t, Tool::ColumnC | Tool::Ring) {
                    d.column_width_mm = bar_length(ui, tr("Width"), d.column_width_mm, u, 0.5, 15.0);
                }
                crate::panels::underlay_combo(ui, "bar_satin_underlay", &mut p.satin.underlay);
            }
            t => {
                p.fill.spacing_mm = bar_param(
                    ui,
                    tr("Density"),
                    p.fill.spacing_mm,
                    u,
                    (0.2, 1.5),
                    (words::density, tr(words::DENSITY_HELP)),
                );
                p.fill.stitch_length_mm = bar_param(
                    ui,
                    tr("Stitch length"),
                    p.fill.stitch_length_mm,
                    u,
                    (1.0, 7.0),
                    (words::length, tr(words::LENGTH_HELP)),
                );
                ui.label(tr("Angle")).on_hover_text(tr(words::ANGLE_HELP));
                ui.add(
                    egui::DragValue::new(&mut p.fill.angle_deg)
                        .range(0.0..=179.0)
                        .clamp_existing_to_range(false)
                        .suffix("°"),
                );
                crate::panels::underlay_combo(ui, "bar_fill_underlay", &mut p.fill.underlay);
                if t == Tool::RoundRect {
                    d.corner_radius_mm = bar_length(ui, tr("Corner Radius"), d.corner_radius_mm, u, 0.0, 100.0);
                }
                if t == Tool::Star {
                    ui.label(tr("Points"));
                    ui.add(
                        egui::DragValue::new(&mut d.star_points)
                            .range(3..=24)
                            .clamp_existing_to_range(false),
                    );
                }
            }
        }
        ui.separator();
        ui.toggle_value(&mut s.snap_grid, tr("Snap grid"))
            .on_hover_text(tr("Points snap to the grid"));
        ui.toggle_value(&mut s.snap_guides, tr("Snap guides"))
            .on_hover_text(tr("Points snap to guides. Drag from a ruler to add a guide."));
        ui.toggle_value(&mut s.snap_objects, tr("Snap objects"))
            .on_hover_text(tr("Points snap to the nodes of other objects."));
    });
    st.set_settings(s);
    if cur != st.doc.design().current {
        st.doc
            .edit_merged("Change current properties", MergeKey::new("current-properties"), |d| {
                d.current = cur
            });
        if !ui.input(|i| i.pointer.any_down()) {
            st.doc.end_merge();
        }
    }
}

/// Position and size of the selection, in the property bar.
fn selection_fields(ui: &mut egui::Ui, st: &AppState) {
    let u = st.settings.units;
    let d = st.doc.design();
    let ids = st.selection.ids();
    let Some(b) = d.bbox_of(ids) else {
        ui.weak(tr("Nothing selected"));
        return;
    };
    let rot = d.rotation_transform();
    let corners = [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)].map(|p| rot.apply(p));
    let vb = BBox::from_points(corners).unwrap_or(b);
    let c = vb.center() - d.hoop.center;
    ui.label(format!(
        "X {}  Y {}  W {}  H {}",
        u.fmt(c.x),
        u.fmt(c.y),
        u.fmt(vb.width()),
        u.fmt(vb.height())
    ));
    ui.weak(crate::i18n::fill("({} selected)", &[&(ids.len())]));
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Modifiers, PointerButton};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn units_format_and_parse() {
        assert_eq!(Units::Mm.fmt(12.345), "12.3 mm");
        assert_eq!(Units::Inch.fmt(25.4), "1.000 in");
        assert!((Units::Inch.to_mm(2.0) - 50.8).abs() < 1e-12);
        assert_eq!(label(2.50), "2.5");
        assert_eq!(label(10.0), "10");
    }

    #[test]
    fn typed_lengths_take_their_units() {
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
        let (mm, inch) = (Units::Mm, Units::Inch);
        assert!(near(parse_length("12.5", mm), 12.5));
        assert!(near(parse_length("12,5", mm), 12.5));
        assert!(near(parse_length("2", inch), 50.8));
        assert!(near(parse_length("2 in", mm), 50.8));
        assert!(near(parse_length("2in", mm), 50.8));
        assert!(near(parse_length("2\"", mm), 50.8));
        assert!(near(parse_length("1 1/2 in", mm), 38.1));
        assert!(near(parse_length("1-1/2\"", mm), 38.1));
        assert!(near(parse_length("3/4 in", mm), 19.05));
        assert!(near(parse_length("25.4 mm", inch), 25.4));
        assert!(near(parse_length("3 cm", inch), 30.0));
        assert!(near(parse_length("1 ft", mm), 304.8));
        assert!(near(parse_length("2 in + 3 mm", mm), 53.8));
        assert!(
            near(parse_length("2 + 3 mm", inch), 53.8),
            "a number in a sum takes the units"
        );
        assert!(near(parse_length("120/4", mm), 30.0));
        assert!(near(parse_length("2 x 3 in", mm), 152.4));
        assert!(near(parse_length("(10 + 5) * 2", mm), 30.0));
        assert!(near(parse_length("-5", mm), -5.0));
        assert!(near(parse_length(" 4 IN ", mm), 101.6));
        for bad in ["", "abc", "2 parsecs", "1/0", "2 in in", "(3", "3 in * 2 in"] {
            assert_eq!(parse_length(bad, mm), None, "{bad}");
        }
        assert!(near(parse_number("45°"), 45.0));
        assert!(near(parse_number("90/2"), 45.0));
        assert_eq!(parse_number("2 in"), None);
    }

    /// The folder of an earlier name moves to the new name once, with its
    /// files (ADR 0059, 0061).
    #[test]
    fn the_old_settings_folder_moves_to_the_new_name() {
        for (n, old) in OLD_FOLDERS.iter().enumerate() {
            let base = std::env::temp_dir().join(format!("shokujodx-folder-test-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            std::fs::create_dir_all(base.join(old).join("crashes")).unwrap();
            std::fs::write(base.join(old).join("settings.json"), r#"{"nudge_large_mm":1.0}"#).unwrap();
            let folder = program_folder(&base);
            assert_eq!(folder, base.join(FOLDER));
            assert!(folder.join("settings.json").is_file() && folder.join("crashes").is_dir());
            assert!(!base.join(old).exists(), "the old folder moved");
            // Again: the new folder, and an old folder that comes back stays.
            std::fs::create_dir_all(base.join(old)).unwrap();
            assert_eq!(program_folder(&base), base.join(FOLDER));
            assert!(base.join(old).exists());
            std::fs::remove_dir_all(&base).unwrap();
        }
    }

    /// A folder named Ariadne of another program stays where it is.
    #[test]
    fn the_folder_of_another_program_does_not_move() {
        let base = std::env::temp_dir().join(format!("shokujodx-other-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join(OLD_FOLDERS[0])).unwrap();
        std::fs::write(base.join(OLD_FOLDERS[0]).join("settings.json"), r#"{"theme":"x"}"#).unwrap();
        assert_eq!(program_folder(&base), base.join(FOLDER));
        assert!(
            base.join(OLD_FOLDERS[0]).join("settings.json").is_file(),
            "the other folder stays"
        );
        assert!(!base.join(FOLDER).exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn settings_round_trip_and_tolerate_missing_fields() {
        let s = Settings {
            units: Units::Inch,
            defaults: Defaults {
                column_width_mm: 4.5,
                ..Defaults::default()
            },
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        let partial: Settings = serde_json::from_str(r#"{"units":"Inch"}"#).unwrap();
        assert_eq!(partial.grid_mm, Settings::default().grid_mm);
        assert!(partial.dark_mode, "dark mode is the default");
        // The program starts on the Digitize tab, also after the Edit tab
        // showed (and after a file of an older version with the tab).
        let edit = Settings {
            ribbon_tab: crate::ribbon::RibbonTab::Edit,
            ribbon_collapsed: true,
            ..Settings::default()
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&edit).unwrap()).unwrap();
        assert_eq!(back.ribbon_tab, crate::ribbon::RibbonTab::Digitize);
        assert!(back.ribbon_collapsed, "the folded ribbon stays");
        let old: Settings = serde_json::from_str(r#"{"ribbon_tab":"Edit"}"#).unwrap();
        assert_eq!(old.ribbon_tab, crate::ribbon::RibbonTab::Digitize);
    }

    #[test]
    fn dark_and_light_mode() {
        let mut h = Harness::empty();
        apply_theme_in(&h.ctx, &h.st.settings, &h.st.settings.language);
        assert_eq!(h.ctx.options(|o| o.theme_preference), egui::ThemePreference::Dark);
        h.st.run(
            crate::ribbon::Cmd::Toggle(crate::ribbon::Flag::DarkMode),
            &h.ctx.clone(),
        );
        assert!(!h.st.settings.dark_mode);
        apply_theme_in(&h.ctx, &h.st.settings, &h.st.settings.language);
        assert_eq!(h.ctx.options(|o| o.theme_preference), egui::ThemePreference::Light);
    }

    #[test]
    fn high_contrast_text_size_and_language() {
        let mut h = Harness::empty();
        let mut s = h.st.settings.clone();
        s.high_contrast = true;
        s.text_scale = 1.5;
        apply_theme_in(&h.ctx, &s, &s.language);
        let v = h.ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(v.panel_fill, egui::Color32::BLACK);
        assert_eq!(v.override_text_color, Some(egui::Color32::WHITE));
        let light = h.ctx.style_of(egui::Theme::Light).visuals.clone();
        assert_eq!(light.panel_fill, egui::Color32::WHITE);
        // The zoom changes at the next frame.
        h.frame(vec![]);
        assert_eq!(h.ctx.zoom_factor(), 1.5);
        // Off again: the normal colors.
        s.high_contrast = false;
        apply_theme_in(&h.ctx, &s, &s.language);
        assert_eq!(
            h.ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
            egui::Visuals::dark().panel_fill
        );
        // The steps of Ctrl+Plus and Ctrl+Minus.
        assert_eq!(step_text_scale(1.0, 1), 1.15);
        assert_eq!(step_text_scale(1.0, -1), 0.9);
        assert_eq!(step_text_scale(0.9, -1), 0.9);
        assert_eq!(step_text_scale(2.0, 1), 2.0);
        assert_eq!(step_text_scale(1.3, 0), 1.0);
        // An unknown language is English.
        s.language = "xx".into();
        apply_theme_in(&h.ctx, &s, &s.language);
        assert_eq!(crate::i18n::current(), "en");
    }

    #[test]
    fn snap_to_grid_and_guides() {
        let mut h = Harness::empty();
        let map = h.map();
        let c = h.st.doc.design().hoop.center;
        h.st.settings.snap_grid = true;
        h.st.settings.grid_mm = 5.0;
        let p = h.st.snap(&map, c + Point::new(6.8, -2.4));
        assert!(p.distance(c + Point::new(5.0, 0.0)) < 1e-9, "{p:?}");
        // A guide near the point wins over the grid.
        h.st.doc.edit("Add guide", |d| {
            d.guides.push(Guide {
                vertical: true,
                pos: c.x + 7.0,
            })
        });
        let p = h.st.snap(&map, c + Point::new(6.8, -2.4));
        assert!((p.x - (c.x + 7.0)).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn snap_to_objects() {
        let mut h = Harness::new();
        h.st.selection.clear();
        let map = h.map();
        let id = h.id(0);
        let node =
            h.st.doc
                .design()
                .object(id)
                .unwrap()
                .kind
                .paths()
                .next()
                .unwrap()
                .points()
                .next()
                .unwrap();
        let near = node + Point::new(0.5 / h.st.view.scale(), 0.0);
        assert_eq!(h.st.snap(&map, near), near, "off by default");
        h.st.settings.snap_objects = true;
        let p = h.st.snap(&map, near);
        assert!(p.distance(node) < 1e-9, "{p:?} {node:?}");
        // A selected object does not snap to itself: the point stays, or it
        // goes to a node of another object.
        h.st.selection.set(id);
        let p = h.st.snap(&map, near);
        let other_nodes: Vec<Point> =
            h.st.doc
                .design()
                .objects()
                .iter()
                .filter(|o| o.id != id)
                .flat_map(|o| o.kind.paths().flat_map(|path| path.points()).collect::<Vec<_>>())
                .collect();
        assert!(p == near || other_nodes.iter().any(|q| q.distance(p) < 1e-9), "{p:?}");
    }

    #[test]
    fn drag_from_the_ruler_makes_a_guide_and_back_deletes_it() {
        let mut h = Harness::empty();
        let from = Pos2::new(300.0, RULER / 2.0);
        let to = Pos2::new(300.0, 250.0);
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let guides = h.st.doc.design().guides.clone();
        assert_eq!(guides.len(), 1);
        assert!(!guides[0].vertical);
        let want =
            h.st.view
                .to_mm(Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN), to)
                .y;
        assert!((guides[0].pos - want).abs() < 1e-6);
        // One drag is one undo step.
        h.st.undo();
        assert!(h.st.doc.design().guides.is_empty());
        h.st.redo();
        // Drag the guide back onto the ruler: it is deleted.
        h.drag(PointerButton::Primary, to, from, Modifiers::NONE);
        assert!(h.st.doc.design().guides.is_empty());
    }

    #[test]
    fn measure_tool_measures() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Measure);
        let (a, b) = (Pos2::new(400.0, 300.0), Pos2::new(500.0, 300.0));
        h.drag(PointerButton::Primary, a, b, Modifiers::NONE);
        let (pa, pb) = h.st.measure.unwrap();
        let mm = 100.0 / h.st.view.scale();
        assert!((pa.distance(pb) - mm).abs() < 1e-3);
        assert!(h.st.message.starts_with(&Units::Mm.fmt(mm)));
    }

    #[test]
    fn zoom_tool_box_and_click() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Zoom);
        let z0 = h.st.view.zoom;
        h.drag(
            PointerButton::Primary,
            Pos2::new(400.0, 300.0),
            Pos2::new(500.0, 360.0),
            Modifiers::NONE,
        );
        assert!(h.st.view.zoom > z0 * 4.0, "{} vs {z0}", h.st.view.zoom);
        let z1 = h.st.view.zoom;
        h.click(PointerButton::Primary, Pos2::new(450.0, 330.0));
        assert!((h.st.view.zoom - (z1 * 2.0).min(crate::canvas::MAX_ZOOM)).abs() < 1e-9);
    }
}
