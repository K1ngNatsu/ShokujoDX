//! View and workspace helpers (P10 Milestone 2).
//!
//! - Center at stitch: the view moves so the stitch cursor is at its center.
//! - Travel by function: Shift+Page Up and Page Down (and the player bar)
//!   go to the previous or next function of one kind (trim, color change,
//!   stop, jump, speed change), or of any kind.
//! - Zoom steps: the step of Zoom in and Zoom out (+ and -) is a setting
//!   (1.25×, 1.5× or 2×); Zoom in 2× and Zoom out 2× are commands.
//! - Panels: each dock panel can close (its × button) and open again
//!   (View > Panels). The settings keep which panels are closed.
//! - Display colors: the grid, the guides, the hoop and the selection can
//!   have the user's own colors (Options).
//! - Favorite styles: up to 10 styles that the user keeps for all designs.
//!   The ribbon applies them to the selection.
//! - The color wheel (Threads panel): the design colors on a wheel, and
//!   colors that go with the current thread, matched to the thread chart.
//! - The 3D thread look (P10 Milestone 3, `emb_render::thread3d`): View >
//!   Thread look has the 3D look, the light and the fabric texture. The
//!   texture shows under the stitches where the design has a background
//!   color, in the weave of the fabric of the design.

use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use emb_model::{Rgb, StitchKind, Style, chart};
use serde::{Deserialize, Serialize};

use crate::app::{AppState, Tab};
use crate::canvas::color32;
use crate::i18n::tr;

/// The kind of function that travel by function goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionTravel {
    /// Trims, color changes and stops.
    #[default]
    Any,
    Trim,
    ColorChange,
    Stop,
    Jump,
    /// Slow and normal speed.
    Speed,
}

impl FunctionTravel {
    pub const ALL: [FunctionTravel; 6] = [
        FunctionTravel::Any,
        FunctionTravel::Trim,
        FunctionTravel::ColorChange,
        FunctionTravel::Stop,
        FunctionTravel::Jump,
        FunctionTravel::Speed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FunctionTravel::Any => tr("Any function"),
            FunctionTravel::Trim => tr("Trims"),
            FunctionTravel::ColorChange => tr("Color changes"),
            FunctionTravel::Stop => tr("Stops"),
            FunctionTravel::Jump => tr("Jumps"),
            FunctionTravel::Speed => tr("Speed changes"),
        }
    }

    /// Records of this kind are stops of the travel.
    pub fn matches(self, k: StitchKind) -> bool {
        match self {
            FunctionTravel::Any => matches!(k, StitchKind::Trim | StitchKind::ColorChange | StitchKind::Stop),
            FunctionTravel::Trim => k == StitchKind::Trim,
            FunctionTravel::ColorChange => k == StitchKind::ColorChange,
            FunctionTravel::Stop => k == StitchKind::Stop,
            FunctionTravel::Jump => k == StitchKind::Jump,
            FunctionTravel::Speed => matches!(k, StitchKind::Slow | StitchKind::Fast),
        }
    }
}

/// The thread look settings (View > Thread look).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LookSettings {
    /// The thread look has light and shade (else the flat look).
    pub three_d: bool,
    pub light: emb_render::thread3d::Lighting,
    /// The texture of the fabric under the background color.
    pub fabric: bool,
    /// The 3D thread look shows the surface of the stitches on tiles (their
    /// heights, the sheen of the thread and the shadows; P18 view, ADR
    /// 0126). Else it shows the faster look of the stitches one by one (the
    /// quality Fast, ADR 0130).
    pub surface: bool,
    /// The short measure of the first start chose the quality (ADR 0130).
    pub measured: bool,
}

/// The quality of the 3D look of the canvas (ADR 0130).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// The look of the stitches one by one (no tiles).
    Fast,
    /// The surface on tiles, with no shadows from the heights.
    Good,
    /// All of the surface.
    Best,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Fast, Quality::Good, Quality::Best];

    /// The name (English; translated where it shows).
    pub fn label(self) -> &'static str {
        match self {
            Quality::Fast => "Fast",
            Quality::Good => "Good",
            Quality::Best => "Best",
        }
    }

    /// The tooltip.
    pub fn tooltip(self) -> &'static str {
        match self {
            Quality::Fast => "The stitches one by one, with light and shade: for a slow computer.",
            Quality::Good => "The surface of the stitches (heights, sheen, gaps), with no shadows from the heights.",
            Quality::Best => "All of the surface: also the shadows of high parts on the fabric and on lower stitches.",
        }
    }
}

/// The time of the measure of the first start ([`measure_ms`]) under which
/// the quality is Best, and under which it is Good, ms. The first full
/// view of a design of 100,000 stitches takes about 0.12 s for each ms of
/// the measure: the development computer measures 4.0 ms (0.44 s), on 4
/// of its cores 7.6 ms (0.98 s), on 2 cores 13.3 ms (1.59 s). Best: the
/// full view in about 2.5 s or less.
pub const BEST_UNDER_MS: f64 = 20.0;
pub const GOOD_UNDER_MS: f64 = 50.0;

/// The quality for a measure of `ms` (see [`measure_ms`]).
pub fn quality_for(ms: f64) -> Quality {
    if ms < BEST_UNDER_MS {
        Quality::Best
    } else if ms < GOOD_UNDER_MS {
        Quality::Good
    } else {
        Quality::Fast
    }
}

/// The short measure of the first start (ADR 0130): lays a synthetic
/// design of 20,000 stitches and makes two tiles of 256 × 256 pixels of it
/// for each thread of the tile cache, on those threads at one time (as the
/// canvas makes them, so the threads of a core, the memory and a laptop
/// that slows down count); the time of all of it for each tile, ms.
pub fn measure_ms() -> f64 {
    let stitches = emb_render::stress::make(20_000);
    let strands = emb_render::strand::strands(&stitches, &Default::default());
    let t0 = crate::clock::Instant::now();
    let laid = emb_render::surface::lay(&strands, &[]);
    let c = laid.bounds().center();
    let ppm = 8.0;
    let workers = crate::stitch_tiles::tile_workers();
    let count = 2 * workers;
    // The tiles in a square round the middle of the design.
    let side = (count as f64).sqrt().ceil() as i64;
    let next = std::sync::atomic::AtomicUsize::new(0);
    emb_geom::threads::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if k >= count {
                        break;
                    }
                    let (i, j) = ((k as i64) % side - side / 2, (k as i64) / side - side / 2);
                    let (x0, y0) = ((c.x * ppm) as i64 + 256 * i, (c.y * ppm) as i64 + 256 * j);
                    let _ = laid.block(
                        ppm,
                        x0,
                        y0,
                        256,
                        256,
                        &emb_render::thread3d::Lighting::default(),
                        None,
                        &|_| true,
                    );
                }
            });
        }
    });
    t0.elapsed().as_secs_f64() * 1000.0 / count as f64
}

impl Default for LookSettings {
    fn default() -> Self {
        LookSettings {
            three_d: true,
            light: Default::default(),
            fabric: true,
            // The tests of the program draw the faster look, unless they
            // ask for the surface.
            surface: !cfg!(test),
            measured: false,
        }
    }
}

impl LookSettings {
    /// The quality of the 3D look of the canvas.
    pub fn quality(&self) -> Quality {
        match (self.surface, self.light.shadows) {
            (false, _) => Quality::Fast,
            (true, false) => Quality::Good,
            (true, true) => Quality::Best,
        }
    }

    /// Sets the quality of the 3D look of the canvas.
    pub fn set_quality(&mut self, q: Quality) {
        self.surface = q != Quality::Fast;
        self.light.shadows = q != Quality::Good;
    }

    /// The quality of the first start (ADR 0130): Fast with the software
    /// renderer, else from the short measure. It is chosen one time; the
    /// user can change it.
    pub fn choose_quality(&mut self, software: bool) {
        // A debug build is too slow for the measure (and its settings are
        // those of the program).
        if self.measured || (cfg!(debug_assertions) && !software) {
            return;
        }
        self.measured = true;
        let q = if software {
            Quality::Fast
        } else {
            quality_for(measure_ms())
        };
        self.set_quality(q);
    }

    /// The look of the pictures (design picture, decoration) for a design
    /// on `fabric` (`AppState::fabric_look`).
    pub fn picture_look(&self, fabric: Option<emb_render::fabric::FabricLook>) -> emb_render::raster::Look {
        if !self.three_d {
            return emb_render::raster::Look::default();
        }
        // The pictures have all of the surface, at each quality.
        emb_render::raster::Look {
            lighting: Some(emb_render::thread3d::Lighting {
                shadows: true,
                ..self.light
            }),
            fabric: fabric.filter(|_| self.fabric),
        }
    }
}

/// The View options that stay between sessions (v0.3 plan, Stage 2): the
/// owner had to switch on Thread Look at each start. The program applies
/// them at the start and keeps them after each frame
/// ([`AppState::keep_view_options`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewOptions {
    pub artwork: bool,
    pub stitches: bool,
    pub jumps: bool,
    pub needle_points: bool,
    pub functions: bool,
    pub density: bool,
    pub thread_look: bool,
    pub overview: bool,
    pub applique_fabric: bool,
    pub stones: bool,
    pub split: bool,
    pub vectors: bool,
    pub dim_vectors: bool,
    pub draw_vectors: bool,
    pub hoop: bool,
}

impl Default for ViewOptions {
    /// The options of a new install.
    fn default() -> Self {
        ViewOptions {
            artwork: true,
            stitches: true,
            jumps: false,
            needle_points: false,
            functions: false,
            density: false,
            thread_look: false,
            overview: false,
            applique_fabric: true,
            stones: true,
            split: false,
            vectors: true,
            dim_vectors: false,
            draw_vectors: false,
            hoop: true,
        }
    }
}

impl AppState {
    /// The View options as they are now. While the stitch player shows,
    /// Stitches is the value from before the player (the player shows
    /// the stitches).
    pub fn view_options(&self) -> ViewOptions {
        ViewOptions {
            artwork: self.show_artwork,
            stitches: if self.player.shown {
                self.settings.view.stitches
            } else {
                self.show_plan
            },
            jumps: self.show_jumps,
            needle_points: self.show_needle_points,
            functions: self.show_functions,
            density: self.show_density,
            thread_look: self.show_thread_look,
            overview: self.show_overview,
            applique_fabric: self.show_applique_fabric,
            stones: self.show_stones,
            split: self.show_split,
            vectors: self.show_vectors,
            dim_vectors: self.dim_vectors,
            draw_vectors: self.draw_vectors,
            hoop: self.show_hoop,
        }
    }

    /// Sets the View options (at the start, from the settings).
    pub fn apply_view_options(&mut self, v: ViewOptions) {
        self.show_artwork = v.artwork;
        self.show_plan = v.stitches;
        self.show_jumps = v.jumps;
        self.show_needle_points = v.needle_points;
        self.show_functions = v.functions;
        self.show_density = v.density;
        self.show_thread_look = v.thread_look;
        self.show_overview = v.overview;
        self.show_applique_fabric = v.applique_fabric;
        self.show_stones = v.stones;
        self.show_split = v.split;
        self.show_vectors = v.vectors;
        self.dim_vectors = v.dim_vectors;
        self.draw_vectors = v.draw_vectors;
        self.show_hoop = v.hoop;
    }

    /// Keeps the View options in the settings if they changed (call it
    /// once a frame; the settings file is written only on a change).
    pub fn keep_view_options(&mut self) {
        // The kiosk sets its own View options (`open_kiosk`); they are not
        // the user's.
        if self.kiosk.is_some() {
            return;
        }
        let v = self.view_options();
        if v != self.settings.view {
            let mut s = self.settings.clone();
            s.view = v;
            self.set_settings(s);
        }
    }
}

/// The fabric of the canvas (ADR 0127): one repeat of its relief in the
/// light of the 3D look, and its texture at each zoom level that showed.
pub struct FabricTexture {
    pub key: (emb_render::fabric::FabricLook, Rgb, emb_render::thread3d::Lighting),
    shade: emb_render::fabric::Shade,
    textures: Vec<Option<egui::TextureHandle>>,
}

/// Draws the fabric of the design over the background color of the hoop
/// field (the whole canvas in open field), with the 3D thread look: its
/// relief in the light of the look, at the zoom level whose texels are as
/// small as the pixels of the screen (ADR 0127). Its mean color in the
/// default light is the background color.
pub fn draw_fabric(painter: &egui::Painter, ctx: &egui::Context, rect: egui::Rect, st: &mut AppState) {
    let look = st.settings.look;
    if !(st.show_plan && st.show_thread_look && look.three_d && look.fabric) {
        return;
    }
    let d = st.doc.design();
    let Some(bg) = d.background else { return };
    // Lace has no fabric: the stabilizer washes away (ADR 0128).
    if st.fabrics.of(d).is_lace() {
        return;
    }
    let fabric = st.fabric_look(d);
    let key = (fabric, bg, look.light);
    if st.fabric_tex.as_ref().is_none_or(|t| t.key != key) {
        let shade = fabric.fabric().shade(&look.light);
        let textures = vec![None; shade.levels()];
        st.fabric_tex = Some(FabricTexture { key, shade, textures });
    }
    let Some(tex) = st.fabric_tex.as_mut() else { return };
    let level = tex.shade.level_for(st.view.scale() * f64::from(ctx.pixels_per_point()));
    let repeat = tex.shade.repeat;
    let texture = match &tex.textures[level] {
        Some(t) => t.clone(),
        None => {
            let p = tex.shade.picture(level, bg, fabric.mix);
            let image = egui::ColorImage::from_rgba_unmultiplied([p.width as usize, p.height as usize], &p.rgba);
            let t = ctx.load_texture(format!("fabric-{level}"), image, egui::TextureOptions::LINEAR_REPEAT);
            tex.textures[level] = Some(t.clone());
            t
        }
    };
    let polygons: Vec<Vec<Pos2>> = match st.hoop_outline(0.0) {
        Some(path) => crate::canvas::screen_polylines(&path, &emb_geom::Transform::IDENTITY, rect, &st.view)
            .into_iter()
            .map(|(pts, _)| pts)
            .collect(),
        None => vec![vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ]],
    };
    let tint = Color32::WHITE;
    for pts in polygons {
        if pts.len() < 3 {
            continue;
        }
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for p in &pts {
            let mm = st.view.to_mm(rect, *p);
            mesh.vertices.push(egui::epaint::Vertex {
                pos: *p,
                uv: Pos2::new((mm.x / repeat.0) as f32, (mm.y / repeat.1) as f32),
                color: tint,
            });
        }
        // Hoop fields are convex.
        for k in 1..pts.len() as u32 - 1 {
            mesh.add_triangle(0, k, k + 1);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// The settings of the thread look (View > Thread look menu).
pub fn look_ui(ui: &mut egui::Ui, st: &mut AppState) {
    if ui.checkbox(&mut st.show_thread_look, tr("Thread Look")).changed() && st.show_thread_look {
        st.show_plan = true;
    }
    let mut s = st.settings.clone();
    let l = &mut s.look;
    ui.add_enabled_ui(st.show_thread_look, |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(&mut l.three_d, false, tr("Flat"));
            ui.radio_value(&mut l.three_d, true, tr("3D (light and shade)"));
        });
        ui.add_enabled_ui(l.three_d, |ui| {
            crate::viewer::light_ui(ui, &mut l.light);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr("Quality"));
                let now = l.quality();
                for q in Quality::ALL {
                    if ui
                        .radio(now == q, tr(q.label()))
                        .on_hover_text(tr(q.tooltip()))
                        .clicked()
                    {
                        l.set_quality(q);
                    }
                }
            });
            ui.checkbox(&mut l.fabric, tr("Fabric texture")).on_hover_text(tr(
                "The surface of the fabric of the design (its weave, loops or grain) in the background color (Color tab).",
            ));
            ui.add_enabled(l.surface, egui::Checkbox::new(&mut l.light.lights_off, tr("Lights off")))
                .on_hover_text(tr(
                    "A dark room: only glow-in-the-dark thread shows, in its own light (with the quality Good or Best).",
                ));
            if ui.button(tr("Default light")).clicked() {
                l.light = Default::default();
            }
        });
    });
    st.set_settings(s);
}

/// Zoom steps of Zoom in and Zoom out.
pub const ZOOM_STEPS: [f64; 3] = [1.25, 1.5, 2.0];

/// The panels that can close and open (not the canvas and the split view).
pub const PANELS: [(Tab, &str); 9] = [
    (Tab::Properties, "Object Properties"),
    (Tab::Styles, "Styles"),
    (Tab::DesignInfo, "Design Info"),
    (Tab::Clipart, "Clipart"),
    (Tab::Sequence, "Sequence"),
    (Tab::Threads, "Threads"),
    (Tab::StitchList, "Stitch List"),
    (Tab::Exports, "Exports"),
    (Tab::Library, "Library"),
];

/// The panels of simple mode.
pub const SIMPLE_PANELS: [Tab; 5] = [Tab::Properties, Tab::Sequence, Tab::Threads, Tab::Library, Tab::Exports];

/// The panels that go together when a closed panel opens again.
pub fn panel_group(tab: Tab) -> &'static [Tab] {
    if crate::app::SIDE_PANELS.contains(&tab) {
        &crate::app::SIDE_PANELS
    } else {
        &[Tab::Canvas, Tab::Library]
    }
}

/// The most favorite styles.
pub const MAX_FAVORITES: usize = 10;

/// The user's own colors for parts of the canvas. `None`: the color of the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayColors {
    pub grid: Option<Rgb>,
    pub guides: Option<Rgb>,
    pub hoop: Option<Rgb>,
    pub selection: Option<Rgb>,
}

impl DisplayColors {
    /// The minor and major grid lines.
    pub fn grid(&self, dark: bool) -> (Color32, Color32) {
        match self.grid {
            Some(c) => (color32(c).gamma_multiply(0.45), color32(c)),
            None if dark => (Color32::from_gray(44), Color32::from_gray(60)),
            None => (Color32::from_gray(232), Color32::from_gray(208)),
        }
    }

    pub fn guides(&self) -> Color32 {
        self.guides.map_or(Color32::from_rgb(40, 170, 220), color32)
    }

    pub fn hoop(&self, dark: bool) -> Color32 {
        match self.hoop {
            Some(c) => color32(c),
            None if dark => Color32::from_gray(200),
            None => Color32::from_gray(60),
        }
    }

    /// The selection, handles and the boxes of the tools; `theme` if not set.
    pub fn selection(&self, theme: Color32) -> Color32 {
        self.selection.map_or(theme, color32)
    }
}

/// Takes the closed panels out of the dock and puts the panels that open
/// again next to a panel of their group. A command that shows a closed
/// panel (Stitch list, Styles, …) opens it again.
/// `side_hidden`: the side panels are hidden (the user hid them, or the
/// Start page shows; P15 step 3).
pub fn sync_panels(dock: &mut egui_dock::DockState<Tab>, st: &mut AppState, side_hidden: bool) {
    for (flag, tab) in [
        (st.focus_properties, Tab::Properties),
        (st.focus_threads, Tab::Threads),
        (st.focus_stitch_list, Tab::StitchList),
        (st.focus_info, Tab::DesignInfo),
        (st.focus_styles, Tab::Styles),
        (st.focus_library, Tab::Library),
        (st.focus_clipart, Tab::Clipart),
    ] {
        if flag && !st.panel_shown(tab) {
            st.set_panel_shown(tab, true);
        }
        // A command that shows a side panel shows the side panels.
        if flag && crate::app::SIDE_PANELS.contains(&tab) {
            st.side_hidden = false;
        }
    }
    for (tab, _) in PANELS {
        // Simple mode shows the panels of the simple tasks only.
        let simple_hides = st.settings.simple_mode && !SIMPLE_PANELS.contains(&tab);
        // The Library is a tab next to the design, not a side panel.
        let side_hides = side_hidden && crate::app::SIDE_PANELS.contains(&tab);
        match (st.panel_shown(tab) && !simple_hides && !side_hides, dock.find_tab(&tab)) {
            (false, Some(p)) => {
                dock.remove_tab(p);
            }
            (true, None) => {
                let near = panel_group(tab)
                    .iter()
                    .filter(|t| **t != tab)
                    .find_map(|t| dock.find_tab(t));
                match near {
                    Some(p) => dock[p.surface][p.node].append_tab(tab),
                    None => {
                        if let Some(p) = dock.find_tab(&Tab::Canvas) {
                            if st.panels_below {
                                dock[p.surface].split_below(p.node, crate::app::CANVAS_PART_ABOVE, vec![tab]);
                            } else {
                                dock[p.surface].split_right(p.node, crate::app::CANVAS_PART, vec![tab]);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl AppState {
    /// Moves the view so the stitch cursor is at its center.
    pub fn center_at_cursor(&mut self) {
        let Some(i) = self.cursor else {
            self.message = tr("Set the stitch cursor first (the stitch player or the travel keys).").into();
            return;
        };
        let Some(pos) = self.plan_view().plan.stitches.records.get(i).map(|r| r.pos) else {
            return;
        };
        self.push_view();
        self.view.center = self.plan_to_canvas(pos);
    }

    /// Zoom in (`inward`) or out by the zoom step of the settings.
    pub fn zoom_step(&mut self, inward: bool, step: f64) {
        self.push_view();
        self.zoom_by(if inward { step } else { 1.0 / step });
    }

    /// The panel shows (it is not closed).
    pub fn panel_shown(&self, tab: Tab) -> bool {
        !self.settings.hidden_panels.contains(&tab)
    }

    /// Opens or closes a panel. Opening it also shows it in front.
    pub fn set_panel_shown(&mut self, tab: Tab, shown: bool) {
        let mut s = self.settings.clone();
        s.hidden_panels.retain(|t| *t != tab);
        if !shown {
            s.hidden_panels.push(tab);
        }
        self.set_settings(s);
        if shown {
            self.focus_tab = Some(tab);
        }
    }

    /// The favorite style with this name, if any.
    pub fn favorite_index(&self, name: &str) -> Option<usize> {
        self.settings.favorite_styles.iter().position(|s| s.name == name)
    }

    /// Adds the style to the favorites, or takes it out if it is one.
    pub fn toggle_favorite(&mut self, style: &Style) {
        let mut s = self.settings.clone();
        match s.favorite_styles.iter().position(|f| f.name == style.name) {
            Some(i) => {
                s.favorite_styles.remove(i);
                self.message = crate::i18n::fill("\"{}\" is not a favorite now", &[&style.name]);
            }
            None if s.favorite_styles.len() >= MAX_FAVORITES => {
                self.message =
                    crate::i18n::fill("There can be {} favorite styles. Remove one first.", &[&MAX_FAVORITES]);
                return;
            }
            None => {
                s.favorite_styles.push(style.clone());
                self.message = crate::i18n::fill("\"{}\" is a favorite style now (Design > Favorites)", &[&style.name]);
            }
        }
        self.set_settings(s);
    }

    /// Applies favorite style `i` to the selection. The design gets the
    /// style too (a style of the same name changes to the favorite).
    pub fn apply_favorite(&mut self, i: usize) {
        let Some(style) = self.settings.favorite_styles.get(i).cloned() else {
            return;
        };
        if self.selection.is_empty() {
            self.message = tr("Select objects first").into();
            return;
        }
        let ids = self.selection.in_sew_order(self.doc.design());
        let name = style.name.clone();
        let n = self.doc.edit(crate::i18n::fill("Apply style {}", &[&name]), |d| {
            let k = match d.style_index(&name) {
                Some(k) => {
                    d.styles[k] = style;
                    k
                }
                None => {
                    d.styles.push(style);
                    d.styles.len() - 1
                }
            };
            d.apply_style(k, &ids).len()
        });
        self.message = crate::i18n::fill("Style \"{}\" applied to {} objects", &[&name, &n]);
    }
}

/// The display colors in the Options window.
pub fn display_colors_ui(ui: &mut egui::Ui, c: &mut DisplayColors) {
    ui.strong(tr("Display colors"));
    egui::Grid::new("display_colors")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for (label, value, default) in [
                (tr("Grid"), &mut c.grid, Rgb(90, 90, 90)),
                (tr("Guides"), &mut c.guides, Rgb(40, 170, 220)),
                (tr("Hoop"), &mut c.hoop, Rgb(200, 200, 200)),
                (tr("Selection"), &mut c.selection, Rgb(80, 160, 255)),
            ] {
                ui.label(label);
                ui.horizontal(|ui| {
                    let mut own = value.is_some();
                    if ui.checkbox(&mut own, tr("Own color")).changed() {
                        *value = own.then_some(default);
                    }
                    if let Some(rgb) = value {
                        let mut a = [rgb.0, rgb.1, rgb.2];
                        if egui::color_picker::color_edit_button_srgb(ui, &mut a).changed() {
                            *rgb = Rgb(a[0], a[1], a[2]);
                        }
                    } else {
                        ui.weak(tr("the color of the theme"));
                    }
                });
                ui.end_row();
            }
        });
}

/// The point on a color wheel of `radius` at `center` for a color: the
/// hue is the angle (red at the right, counterclockwise), the saturation
/// the distance from the center.
fn wheel_point(center: Pos2, radius: f32, c: Rgb) -> Pos2 {
    let (h, s, _) = chart::hsv(c);
    let (sin, cos) = emb_geom::math::sin_cos_deg(h);
    center + Vec2::new(cos as f32, -sin as f32) * radius * s as f32
}

/// Draws a color wheel: hue round, saturation out, at full value.
fn draw_wheel(painter: &egui::Painter, center: Pos2, radius: f32) {
    const RINGS: usize = 6;
    const SECTORS: usize = 48;
    let mut mesh = egui::Mesh::default();
    for ring in 0..RINGS {
        let (s0, s1) = (ring as f64 / RINGS as f64, (ring + 1) as f64 / RINGS as f64);
        for k in 0..SECTORS {
            let (h0, h1) = (
                k as f64 * 360.0 / SECTORS as f64,
                (k + 1) as f64 * 360.0 / SECTORS as f64,
            );
            let at = |h: f64, s: f64| {
                let (sin, cos) = emb_geom::math::sin_cos_deg(h);
                center + Vec2::new(cos as f32, -sin as f32) * radius * s as f32
            };
            let col = |h: f64, s: f64| color32(chart::from_hsv(h, s, 1.0));
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(at(h0, s0), col(h0, s0));
            mesh.colored_vertex(at(h1, s0), col(h1, s0));
            mesh.colored_vertex(at(h1, s1), col(h1, s1));
            mesh.colored_vertex(at(h0, s1), col(h0, s1));
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// The color wheel of the Threads panel.
pub fn color_wheel(ui: &mut egui::Ui, st: &mut AppState) {
    let d = st.doc.design();
    let used = d.used_threads();
    let threads: Vec<(emb_model::ThreadId, Rgb)> = d
        .threads
        .iter()
        .filter(|t| used.contains(&t.id))
        .map(|t| (t.id, t.color))
        .collect();
    let current = st.current_thread_id();
    let size = 150.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter_at(rect);
    let (center, radius) = (rect.center(), size / 2.0 - 6.0);
    draw_wheel(&painter, center, radius);
    let ink = ui.visuals().text_color();
    let mut picked = None;
    for (id, c) in &threads {
        let p = wheel_point(center, radius, *c);
        let on = current == Some(*id);
        painter.circle_filled(p, if on { 6.0 } else { 4.5 }, color32(*c));
        painter.circle_stroke(
            p,
            if on { 6.0 } else { 4.5 },
            Stroke::new(if on { 2.0 } else { 1.0 }, ink),
        );
        if resp.clicked() && resp.interact_pointer_pos().is_some_and(|q| q.distance(p) <= 7.0) {
            picked = Some(*id);
        }
    }
    resp.on_hover_text(tr("The colors of the design. Click a dot: it is the current thread."));
    if let Some(id) = picked {
        st.current_thread = Some(id);
    }
    // Colors that go with the current thread, as the nearest chart threads.
    let Some(cur) = current
        .and_then(|id| st.doc.design().thread(id))
        .map(|t| (t.name.clone(), t.color))
    else {
        return;
    };
    let Some(chart) = st.chart() else { return };
    let rows: Vec<(&str, Rgb, usize, String)> = chart::harmonies(cur.1)
        .into_iter()
        .filter_map(|(kind, c)| {
            let k = chart.nearest(c)?;
            let t = &chart.threads[k];
            let text = match &t.code {
                Some(code) => format!("{code} {}", t.name),
                None => t.name.clone(),
            };
            Some((kind, t.color, k, text))
        })
        .collect();
    ui.weak(crate::i18n::fill(
        "Colors that go with \"{}\" in {}. Click: use for the current thread. Ctrl+click: add as a new thread.",
        &[&cur.0, &chart.name],
    ));
    let mut use_k = None;
    for (kind, color, k, text) in rows {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            ui.painter().rect_filled(r, 2.0, color32(color));
            if ui
                .add(egui::Button::new(format!("{}: {text}", tr(kind))).frame(false))
                .clicked()
            {
                use_k = Some((k, ui.input(|i| i.modifiers.command)));
            }
        });
    }
    if let Some((k, add)) = use_k {
        st.use_chart_thread(k, add);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{ObjectKind, RunningParams};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn view_options_stay_for_the_next_session() {
        let mut h = Harness::new();
        // A new install: the options of the state are the default options.
        assert_eq!(h.st.view_options(), ViewOptions::default());
        h.st.show_thread_look = true;
        h.st.show_jumps = true;
        h.st.keep_view_options();
        let kept = h.st.settings.view;
        assert!(kept.thread_look && kept.jumps && kept.stitches);
        // The settings go through the file as JSON.
        let text = serde_json::to_string(&h.st.settings).unwrap();
        let back: crate::workspace::Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back.view, kept);
        // A settings file of an older version has no View options.
        let old: crate::workspace::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.view, ViewOptions::default());
        // The next session starts with them.
        let mut next = Harness::new();
        next.st.apply_view_options(back.view);
        assert!(next.st.show_thread_look && next.st.show_jumps);
        // The stitch player shows the stitches, but does not change the option.
        next.st.show_plan = false;
        next.st.keep_view_options();
        next.st.toggle_player();
        next.st.keep_view_options();
        assert!(!next.st.settings.view.stitches);
        // The kiosk shows the thread look, but does not keep it.
        next.st.show_thread_look = false;
        next.st.keep_view_options();
        next.st.open_kiosk();
        assert!(next.st.show_thread_look);
        next.st.keep_view_options();
        next.st.close_kiosk();
        next.st.keep_view_options();
        assert!(!next.st.show_thread_look && !next.st.settings.view.thread_look);
    }

    fn two_lines(h: &mut Harness) {
        h.st.doc.edit("t", |d| {
            let a = d.add_thread("A", Rgb(200, 0, 0));
            let b = d.add_thread("B", Rgb(0, 0, 200));
            for (t, x) in [(a, 0.0), (b, 30.0)] {
                d.add_object(
                    t,
                    ObjectKind::Running {
                        path: Path::new()
                            .move_to(Point::new(x, 0.0))
                            .line_to(Point::new(x + 10.0, 0.0)),
                        params: RunningParams::default(),
                    },
                );
            }
        });
    }

    #[test]
    fn travel_by_function_kind() {
        let mut h = Harness::empty();
        two_lines(&mut h);
        let plan = h.st.plan_view().plan.clone();
        let recs = &plan.stitches.records;
        let color = recs.iter().position(|r| r.kind == StitchKind::ColorChange).unwrap();
        h.st.function_travel = FunctionTravel::ColorChange;
        h.st.cursor = Some(0);
        h.st.travel(crate::viewing::Travel::Function(true));
        assert_eq!(h.st.cursor, Some(color));
        // Jumps: the next jump after the cursor.
        h.st.function_travel = FunctionTravel::Jump;
        h.st.cursor = Some(0);
        h.st.travel(crate::viewing::Travel::Function(true));
        let i = h.st.cursor.unwrap();
        assert!(recs[i].kind == StitchKind::Jump || i == recs.len() - 1);
    }

    #[test]
    fn center_at_cursor_and_zoom_steps() {
        let mut h = Harness::empty();
        two_lines(&mut h);
        let last = h.st.plan_view().plan.stitches.records.len() - 1;
        h.st.cursor = Some(last);
        h.st.center_at_cursor();
        let pos = h.st.plan_view().plan.stitches.records[last].pos;
        assert!(h.st.view.center.distance(h.st.plan_to_canvas(pos)) < 1e-9);
        let z = h.st.view.zoom;
        h.st.zoom_step(true, 2.0);
        assert!((h.st.view.zoom - 2.0 * z).abs() < 1e-9);
        h.st.previous_view();
        assert!((h.st.view.zoom - z).abs() < 1e-9);
    }

    #[test]
    fn favorites_go_to_any_design() {
        let mut h = Harness::empty();
        h.st.save_settings = false;
        two_lines(&mut h);
        let mut style = Style::from_objects("Long", h.st.doc.design().objects().iter().map(|o| o.as_ref())).unwrap();
        style.running.as_mut().unwrap().stitch_length_mm = 4.0;
        h.st.toggle_favorite(&style);
        assert_eq!(h.st.favorite_index("Long"), Some(0));
        // A new design: the favorite applies and the design gets the style.
        h.st.new_design();
        two_lines(&mut h);
        let id = h.st.doc.design().objects()[0].id;
        h.st.selection.set(id);
        h.st.apply_favorite(0);
        let ObjectKind::Running { params, .. } = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert_eq!(params.stitch_length_mm, 4.0);
        assert!(h.st.doc.design().style_index("Long").is_some());
        h.st.toggle_favorite(&style);
        assert!(h.st.settings.favorite_styles.is_empty());
    }

    #[test]
    fn panels_close_and_open() {
        let mut h = Harness::empty();
        h.st.save_settings = false;
        let mut dock = crate::app::default_dock();
        // The side panels are one group at the start, and they are the
        // panels of View > Panels with no Library (it is next to the design).
        assert!(crate::test_util::one_side_group(&dock));
        let mut side = crate::app::SIDE_PANELS.to_vec();
        side.push(Tab::Library);
        assert!(PANELS.iter().all(|(t, _)| side.contains(t)) && side.len() == PANELS.len());
        h.st.set_panel_shown(Tab::Threads, false);
        assert!(!h.st.panel_shown(Tab::Threads));
        sync_panels(&mut dock, &mut h.st, false);
        assert!(dock.find_tab(&Tab::Threads).is_none());
        h.st.set_panel_shown(Tab::Threads, true);
        assert_eq!(h.st.focus_tab, Some(Tab::Threads));
        sync_panels(&mut dock, &mut h.st, false);
        // Back with its group, next to the Sequence panel.
        let (t, s) = (
            dock.find_tab(&Tab::Threads).unwrap(),
            dock.find_tab(&Tab::Sequence).unwrap(),
        );
        assert_eq!((t.surface, t.node), (s.surface, s.node));
        // All of the group closed: a panel opens on its own right of the canvas.
        for tab in panel_group(Tab::Properties) {
            h.st.set_panel_shown(*tab, false);
        }
        sync_panels(&mut dock, &mut h.st, false);
        assert!(dock.find_tab(&Tab::Properties).is_none());
        // A command that shows a closed panel opens it.
        h.st.focus_styles = true;
        sync_panels(&mut dock, &mut h.st, false);
        assert!(dock.find_tab(&Tab::Styles).is_some());
        assert!(h.st.panel_shown(Tab::Styles));
        h.st.set_panel_shown(Tab::Threads, true);
        sync_panels(&mut dock, &mut h.st, false);
        // Simple mode hides the panels that it does not need, and shows them again after.
        h.st.settings.simple_mode = true;
        sync_panels(&mut dock, &mut h.st, false);
        assert!(dock.find_tab(&Tab::Styles).is_none());
        assert!(dock.find_tab(&Tab::Threads).is_some());
        h.st.settings.simple_mode = false;
        sync_panels(&mut dock, &mut h.st, false);
        assert!(dock.find_tab(&Tab::Styles).is_some());
    }

    #[test]
    fn color_wheel_draws_and_picks_a_harmony() {
        let mut h = Harness::empty();
        two_lines(&mut h);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| color_wheel(ui, &mut h.st));
            });
            out.textures_delta.clear();
        }
        // The complementary color of the red thread, from the chart.
        let chart = h.st.chart().unwrap();
        let (_, c) = chart::harmonies(Rgb(200, 0, 0))[0];
        let k = chart.nearest(c).unwrap();
        let want = chart.threads[k].color;
        h.st.use_chart_thread(k, false);
        let cur = h.st.current_thread_id().unwrap();
        assert_eq!(h.st.doc.design().thread(cur).unwrap().color, want);
    }

    #[test]
    fn close_all_asks_for_each_unsaved_design() {
        let mut h = Harness::empty();
        two_lines(&mut h);
        h.st.new_design();
        two_lines(&mut h);
        h.st.new_design();
        let unsaved = h.st.unsaved_tabs().len();
        assert!(unsaved >= 2);
        h.st.request(crate::file::Pending::CloseAll);
        // Don't save, for each unsaved design.
        for _ in 0..unsaved {
            // (As the prompt does on Don't save.)
            assert_eq!(h.st.confirm.take(), Some(crate::file::Pending::CloseAll));
            h.st.perform(crate::file::Pending::CloseAll);
        }
        assert!(h.st.confirm.is_none());
        assert_eq!(h.st.tab_count(), 1);
        assert!(h.st.is_blank());
    }

    #[test]
    fn the_quality_sets_the_surface_and_the_shadows() {
        let mut l = LookSettings::default();
        for q in Quality::ALL {
            l.set_quality(q);
            assert_eq!(l.quality(), q);
        }
        l.set_quality(Quality::Good);
        assert!(l.surface && !l.light.shadows);
        // The pictures have the shadows at each quality.
        assert!(l.picture_look(None).lighting.unwrap().shadows);
        assert_eq!(quality_for(10.0), Quality::Best);
        assert_eq!(quality_for(BEST_UNDER_MS + 1.0), Quality::Good);
        assert_eq!(quality_for(GOOD_UNDER_MS + 1.0), Quality::Fast);
        // The software renderer starts with Fast, one time.
        let mut l = LookSettings::default();
        l.choose_quality(true);
        assert_eq!((l.quality(), l.measured), (Quality::Fast, true));
        l.set_quality(Quality::Best);
        l.choose_quality(true);
        assert_eq!(l.quality(), Quality::Best, "the choice of the user stays");
    }

    /// The measure of the first start on this computer. Run with `cargo test
    /// --release -p emb-app -- --ignored first_start_measure --nocapture`.
    #[test]
    #[ignore]
    fn first_start_measure() {
        let runs: Vec<f64> = (0..5).map(|_| measure_ms()).collect();
        println!("first start measure: {runs:?} ms -> {:?}", quality_for(runs[0]));
    }

    #[test]
    fn display_colors_default_to_the_theme() {
        let c = DisplayColors::default();
        assert_eq!(c.selection(Color32::RED), Color32::RED);
        let own = DisplayColors {
            selection: Some(Rgb(1, 2, 3)),
            ..c
        };
        assert_eq!(own.selection(Color32::RED), Color32::from_rgb(1, 2, 3));
    }
}
