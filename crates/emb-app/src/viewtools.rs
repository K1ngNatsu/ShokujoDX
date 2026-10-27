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
}

impl Default for LookSettings {
    fn default() -> Self {
        LookSettings {
            three_d: true,
            light: Default::default(),
            fabric: true,
        }
    }
}

impl LookSettings {
    /// The look of the pictures (design picture, decoration) for a design
    /// on `fabric` (a fabric id).
    pub fn picture_look(&self, fabric: Option<&str>) -> emb_render::raster::Look {
        if !self.three_d {
            return emb_render::raster::Look::default();
        }
        emb_render::raster::Look {
            lighting: Some(self.light),
            fabric: self.fabric.then(|| emb_render::thread3d::Weave::for_fabric(fabric)),
        }
    }
}

/// The fabric texture of the canvas: its weave and color, and the texture.
pub struct FabricTexture {
    pub key: (emb_render::thread3d::Weave, Rgb),
    pub texture: egui::TextureHandle,
}

/// Draws the fabric texture over the background color of the hoop field
/// (the whole canvas in open field), with the 3D thread look. The texture
/// fades out when its threads are smaller than the screen pixels.
pub fn draw_fabric(painter: &egui::Painter, ctx: &egui::Context, rect: egui::Rect, st: &mut AppState) {
    use emb_render::thread3d::{TILE_MM, Weave, fabric_tile};
    let look = st.settings.look;
    if !(st.show_plan && st.show_thread_look && look.three_d && look.fabric) {
        return;
    }
    let d = st.doc.design();
    let Some(bg) = d.background else { return };
    let fade = ((st.view.scale() as f32 - 2.0) / 4.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    let key = (Weave::for_fabric(d.fabric.as_deref()), bg);
    let texture = match &st.fabric_tex {
        Some(t) if t.key == key => t.texture.clone(),
        _ => {
            let tile = fabric_tile(key.0, key.1, 32.0);
            let image =
                egui::ColorImage::from_rgba_unmultiplied([tile.width as usize, tile.height as usize], &tile.rgba);
            let texture = ctx.load_texture("fabric", image, egui::TextureOptions::LINEAR_REPEAT);
            st.fabric_tex = Some(FabricTexture {
                key,
                texture: texture.clone(),
            });
            texture
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
    let a = (255.0 * fade) as u8;
    let tint = Color32::from_rgba_premultiplied(a, a, a, a);
    for pts in polygons {
        if pts.len() < 3 {
            continue;
        }
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for p in &pts {
            let mm = st.view.to_mm(rect, *p);
            mesh.vertices.push(egui::epaint::Vertex {
                pos: *p,
                uv: Pos2::new((mm.x / TILE_MM) as f32, (mm.y / TILE_MM) as f32),
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
            egui::Grid::new("look").num_columns(2).show(ui, |ui| {
                ui.label(tr("Light from"));
                ui.add(
                    egui::Slider::new(&mut l.light.azimuth_deg, 0.0..=360.0)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix("°")
                        .fixed_decimals(0),
                )
                .on_hover_text(tr(
                    "The direction of the light: 0° from the right, 90° from the top, 135° from the top left.",
                ));
                ui.end_row();
                ui.label(tr("Light height"));
                ui.add(
                    egui::Slider::new(&mut l.light.elevation_deg, 10.0..=90.0)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix("°")
                        .fixed_decimals(0),
                )
                .on_hover_text(tr(
                    "Low light gives long shadows and strong shade; 90° is straight above.",
                ));
                ui.end_row();
                ui.label(tr("Strength"));
                ui.add(
                    egui::Slider::new(&mut l.light.strength, 0.0..=1.0)
                        .clamping(egui::SliderClamping::Edits)
                        .fixed_decimals(2),
                );
                ui.end_row();
            });
            ui.checkbox(&mut l.fabric, tr("Fabric texture")).on_hover_text(tr(
                "The weave of the fabric of the design under the background color (Color tab).",
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
    match tab {
        Tab::Properties | Tab::Styles | Tab::DesignInfo | Tab::Clipart => {
            &[Tab::Properties, Tab::Styles, Tab::DesignInfo, Tab::Clipart]
        }
        Tab::Sequence | Tab::Threads | Tab::StitchList | Tab::Exports => {
            &[Tab::Sequence, Tab::Threads, Tab::StitchList, Tab::Exports]
        }
        _ => &[Tab::Canvas, Tab::Library],
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
pub fn sync_panels(dock: &mut egui_dock::DockState<Tab>, st: &mut AppState) {
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
    }
    for (tab, _) in PANELS {
        // Simple mode shows the panels of the simple tasks only.
        let simple_hides = st.settings.simple_mode && !SIMPLE_PANELS.contains(&tab);
        match (st.panel_shown(tab) && !simple_hides, dock.find_tab(&tab)) {
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
                            dock[p.surface].split_right(p.node, 0.74, vec![tab]);
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
        let n = self.doc.edit(format!("Apply style {name}"), |d| {
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
                .add(egui::Button::new(format!("{kind}: {text}")).frame(false))
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
        h.st.set_panel_shown(Tab::Threads, false);
        assert!(!h.st.panel_shown(Tab::Threads));
        sync_panels(&mut dock, &mut h.st);
        assert!(dock.find_tab(&Tab::Threads).is_none());
        h.st.set_panel_shown(Tab::Threads, true);
        assert_eq!(h.st.focus_tab, Some(Tab::Threads));
        sync_panels(&mut dock, &mut h.st);
        // Back with its group, next to the Sequence panel.
        let (t, s) = (
            dock.find_tab(&Tab::Threads).unwrap(),
            dock.find_tab(&Tab::Sequence).unwrap(),
        );
        assert_eq!((t.surface, t.node), (s.surface, s.node));
        // All of a group closed: a panel opens on its own right of the canvas.
        for tab in panel_group(Tab::Properties) {
            h.st.set_panel_shown(*tab, false);
        }
        sync_panels(&mut dock, &mut h.st);
        assert!(dock.find_tab(&Tab::Properties).is_none());
        // A command that shows a closed panel opens it.
        h.st.focus_styles = true;
        sync_panels(&mut dock, &mut h.st);
        assert!(dock.find_tab(&Tab::Styles).is_some());
        assert!(h.st.panel_shown(Tab::Styles));
        // Simple mode hides the panels that it does not need, and shows them again after.
        h.st.settings.simple_mode = true;
        sync_panels(&mut dock, &mut h.st);
        assert!(dock.find_tab(&Tab::Styles).is_none());
        assert!(dock.find_tab(&Tab::Threads).is_some());
        h.st.settings.simple_mode = false;
        sync_panels(&mut dock, &mut h.st);
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
