//! Pictures of the windows of the program (P17 Milestone 2). Test code
//! only: nothing of this file is in the program that users get.
//!
//! `write_window_pictures` draws the whole program with no screen, in each
//! language: each ribbon tab, each panel, and each window that a command
//! opens. It writes the pictures, a page that shows them next to each
//! other (`index.html`) and a list of what it found (`report.txt`) to
//! `target/window-pictures` (or the folder in `EMB_PICTURES_OUT`). It
//! fails when a window gives an error. It does not compare pictures: a
//! person looks at them before a release.
//!
//! ```text
//! cargo test --release -p emb-app -- --ignored write_window_pictures --nocapture
//! ```
//!
//! The pictures come from a small software renderer for the triangles
//! that egui makes ([`Painter`]), so the test needs no graphics card and
//! no new dependency.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use emb_model::{HoopCatalog, MachineCatalog};

use crate::app::{AppState, EmbApp, Tab};
use crate::ribbon::{Cmd, RibbonTab, SelInfo};

/// A common laptop screen, in points: the windows must fit it.
const SMALL: Vec2 = Vec2::new(1366.0, 768.0);
/// A full HD screen.
const LARGE: Vec2 = Vec2::new(1920.0, 1080.0);

/// A texture of egui (the font atlas, or a picture of the program).
struct Texture {
    size: [usize; 2],
    /// Colors with the alpha multiplied in.
    pixels: Vec<Color32>,
    linear: bool,
}

impl Texture {
    fn texel(&self, x: isize, y: isize) -> [f32; 4] {
        let x = x.clamp(0, self.size[0] as isize - 1) as usize;
        let y = y.clamp(0, self.size[1] as isize - 1) as usize;
        self.pixels[y * self.size[0] + x].to_array().map(f32::from)
    }

    /// The color at a place of the texture (0 to 1 in each direction).
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let (w, h) = (self.size[0] as f32, self.size[1] as f32);
        if !self.linear {
            return self.texel((u * w).floor() as isize, (v * h).floor() as isize);
        }
        let (x, y) = (u * w - 0.5, v * h - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as isize, y0 as isize);
        let (a, b) = (self.texel(x0, y0), self.texel(x0 + 1, y0));
        let (c, d) = (self.texel(x0, y0 + 1), self.texel(x0 + 1, y0 + 1));
        std::array::from_fn(|i| {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bottom = c[i] + (d[i] - c[i]) * fx;
            top + (bottom - top) * fy
        })
    }
}

/// A picture: rows of pixels from the top, red, green, blue, alpha.
#[derive(Clone, PartialEq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl Picture {
    pub(crate) fn at(&self, x: usize, y: usize) -> [u8; 4] {
        self.pixels[y * self.width + x]
    }

    /// The part of the picture in `rect` (in pixels), cut at its edges.
    pub(crate) fn crop(&self, rect: Rect) -> Picture {
        let x0 = (rect.min.x.floor().max(0.0) as usize).min(self.width);
        let y0 = (rect.min.y.floor().max(0.0) as usize).min(self.height);
        let x1 = (rect.max.x.ceil().max(0.0) as usize).clamp(x0, self.width);
        let y1 = (rect.max.y.ceil().max(0.0) as usize).clamp(y0, self.height);
        let mut pixels = Vec::with_capacity((x1 - x0) * (y1 - y0));
        for y in y0..y1 {
            pixels.extend_from_slice(&self.pixels[y * self.width + x0..y * self.width + x1]);
        }
        Picture {
            width: x1 - x0,
            height: y1 - y0,
            pixels,
        }
    }

    pub(crate) fn save(&self, path: &Path) -> Result<(), String> {
        let bytes: Vec<u8> = self.pixels.iter().flatten().copied().collect();
        let image = image::RgbaImage::from_raw(self.width as u32, self.height as u32, bytes)
            .ok_or("the picture has a wrong size")?;
        image.save(path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// One corner of a triangle: its place in pixels, its place in the
/// texture and its color (0 to 1, with the alpha multiplied in).
#[derive(Clone, Copy)]
struct Corner {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    color: [f32; 4],
}

/// Twice the area of the triangle a, b, (x, y); its sign says on which
/// side of the line a to b the point is.
fn edge(a: &Corner, b: &Corner, x: f32, y: f32) -> f32 {
    (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}

/// True if a pixel center on the line a to b is a pixel of this triangle.
/// Of two triangles with the same edge, exactly one takes it, so no pixel
/// of a shape with alpha is painted two times.
fn owns(a: &Corner, b: &Corner) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dy > 0.0 || (dy == 0.0 && dx < 0.0)
}

/// Draws the triangles that egui makes, as its renderers do: the color of
/// a corner times the color of the texture, blended over the picture.
#[derive(Default)]
pub struct Painter {
    textures: HashMap<egui::TextureId, Texture>,
}

impl Painter {
    /// Takes the new and changed textures of a frame. Call it before
    /// [`Self::paint`] of that frame.
    pub fn set(&mut self, delta: &mut egui::TexturesDelta) {
        let changes: Vec<(egui::TextureId, egui::epaint::ImageDelta)> = delta
            .set
            .drain()
            .flat_map(|(id, list)| list.into_iter().map(move |d| (id, d)))
            .collect();
        for (id, d) in &changes {
            let egui::ImageData::Color(image) = &d.image;
            let [w, h] = image.size;
            match d.pos {
                None => {
                    self.textures.insert(
                        *id,
                        Texture {
                            size: image.size,
                            pixels: image.pixels.clone(),
                            linear: d.options.magnification == egui::TextureFilter::Linear,
                        },
                    );
                }
                Some([x, y]) => {
                    let Some(t) = self.textures.get_mut(id) else { continue };
                    for row in 0..h {
                        let to = (y + row) * t.size[0] + x;
                        if x + w <= t.size[0] && to + w <= t.pixels.len() {
                            t.pixels[to..to + w].copy_from_slice(&image.pixels[row * w..(row + 1) * w]);
                        }
                    }
                }
            }
        }
    }

    /// Drops the textures that a frame does not use any more. Call it after
    /// [`Self::paint`] of that frame.
    pub fn free(&mut self, delta: &mut egui::TexturesDelta) {
        for id in delta.free.drain() {
            self.textures.remove(&id);
        }
    }

    /// The picture of the triangles of one frame.
    pub fn paint(&self, primitives: &[egui::ClippedPrimitive], ppp: f32, size: [usize; 2], clear: Color32) -> Picture {
        let mut picture = Picture {
            width: size[0],
            height: size[1],
            pixels: vec![clear.to_array(); size[0] * size[1]],
        };
        for p in primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &p.primitive else {
                continue;
            };
            let Some(texture) = self.textures.get(&mesh.texture_id) else {
                continue;
            };
            let clip = [
                ((p.clip_rect.min.x * ppp).round().max(0.0) as usize).min(size[0]),
                ((p.clip_rect.min.y * ppp).round().max(0.0) as usize).min(size[1]),
                ((p.clip_rect.max.x * ppp).round().max(0.0) as usize).min(size[0]),
                ((p.clip_rect.max.y * ppp).round().max(0.0) as usize).min(size[1]),
            ];
            let corner = |i: u32| {
                let v = &mesh.vertices[i as usize];
                Corner {
                    x: v.pos.x * ppp,
                    y: v.pos.y * ppp,
                    u: v.uv.x,
                    v: v.uv.y,
                    color: v.color.to_array().map(|c| f32::from(c) / 255.0),
                }
            };
            for t in mesh.indices.as_chunks::<3>().0 {
                if t.iter().all(|i| (*i as usize) < mesh.vertices.len()) {
                    triangle(&mut picture, clip, [corner(t[0]), corner(t[1]), corner(t[2])], texture);
                }
            }
        }
        picture
    }
}

/// Paints one triangle inside `clip` (left, top, right, bottom in pixels).
fn triangle(picture: &mut Picture, clip: [usize; 4], mut t: [Corner; 3], texture: &Texture) {
    let mut area = edge(&t[0], &t[1], t[2].x, t[2].y);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    if area < 0.0 {
        t.swap(1, 2);
        area = -area;
    }
    let min = |f: fn(&Corner) -> f32| f(&t[0]).min(f(&t[1])).min(f(&t[2]));
    let max = |f: fn(&Corner) -> f32| f(&t[0]).max(f(&t[1])).max(f(&t[2]));
    let x0 = (min(|c| c.x).floor().max(0.0) as usize).max(clip[0]);
    let y0 = (min(|c| c.y).floor().max(0.0) as usize).max(clip[1]);
    let x1 = (max(|c| c.x).ceil().max(0.0) as usize).min(clip[2]);
    let y1 = (max(|c| c.y).ceil().max(0.0) as usize).min(clip[3]);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let tint = |texel: [f32; 4], color: [f32; 4]| -> [f32; 4] { std::array::from_fn(|i| texel[i] * color[i]) };
    // Most triangles have one color and one place in the texture (a plain
    // area): their color is the same for each pixel.
    let plain = t[1..]
        .iter()
        .all(|c| c.u == t[0].u && c.v == t[0].v && c.color == t[0].color);
    let plain = plain.then(|| tint(texture.sample(t[0].u, t[0].v), t[0].color));
    let inside = |w: f32, a: &Corner, b: &Corner| w > 0.0 || (w == 0.0 && owns(a, b));
    for py in y0..y1 {
        let y = py as f32 + 0.5;
        for px in x0..x1 {
            let x = px as f32 + 0.5;
            let w0 = edge(&t[1], &t[2], x, y);
            let w1 = edge(&t[2], &t[0], x, y);
            let w2 = edge(&t[0], &t[1], x, y);
            if !(inside(w0, &t[1], &t[2]) && inside(w1, &t[2], &t[0]) && inside(w2, &t[0], &t[1])) {
                continue;
            }
            let src = plain.unwrap_or_else(|| {
                let (a, b, c) = (w0 / area, w1 / area, w2 / area);
                let mix = |f: fn(&Corner) -> f32| a * f(&t[0]) + b * f(&t[1]) + c * f(&t[2]);
                let color = std::array::from_fn(|i| a * t[0].color[i] + b * t[1].color[i] + c * t[2].color[i]);
                tint(texture.sample(mix(|k| k.u), mix(|k| k.v)), color)
            });
            let dst = &mut picture.pixels[py * picture.width + px];
            let keep = 1.0 - src[3] / 255.0;
            for i in 0..4 {
                dst[i] = (src[i] + f32::from(dst[i]) * keep).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// A text of a frame: where it is, and the area that it can show in.
#[derive(Debug, Clone)]
struct Text {
    text: String,
    rect: Rect,
    clip: Rect,
    /// The program made the text shorter ("…") because it did not fit.
    elided: bool,
}

impl Text {
    /// True if the text shows only in part, because its right end is
    /// outside its area (and not because the user scrolled up or down).
    fn cut(&self) -> bool {
        !self.elided
            && !self.text.trim().is_empty()
            && self.rect.right() > self.clip.right() + 1.5
            && self.rect.left() < self.clip.right() - 6.0
            && self.rect.top() >= self.clip.top() - 1.0
            && self.rect.bottom() <= self.clip.bottom() + 1.0
    }
}

/// The texts of the shapes of a frame.
fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<Text> {
    fn walk(shape: &egui::Shape, clip: Rect, out: &mut Vec<Text>) {
        match shape {
            egui::Shape::Vec(list) => list.iter().for_each(|s| walk(s, clip, out)),
            egui::Shape::Text(t) => out.push(Text {
                text: t.galley.job.text.clone(),
                rect: t.galley.rect.translate(t.pos.to_vec2()),
                clip,
                elided: t.galley.elided,
            }),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for s in shapes {
        walk(&s.shape, s.clip_rect, &mut out);
    }
    out
}

/// One frame as a picture, with what the checks need.
pub(crate) struct Shot {
    pub(crate) picture: Picture,
    texts: Vec<Text>,
    /// The windows and menus that show, in points.
    pub(crate) windows: Vec<Rect>,
}

/// An egui context with the fonts of the program and a painter. One
/// studio draws all scenes of a language.
pub(crate) struct Studio {
    pub(crate) ctx: egui::Context,
    painter: Painter,
}

impl Studio {
    pub(crate) fn new() -> Self {
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        // No fade-in: a picture shows the window as it is at rest.
        ctx.all_styles_mut(|s| s.animation_time = 0.0);
        Studio {
            ctx,
            painter: Painter::default(),
        }
    }

    fn frame(&mut self, app: &mut EmbApp, size: Vec2) -> egui::FullOutput {
        self.frame_with(app, size, Vec::new())
    }

    fn frame_with(&mut self, app: &mut EmbApp, size: Vec2, events: Vec<egui::Event>) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let mut out = self.ctx.run_ui(input, |ui| app.draw(ui));
        self.painter.set(&mut out.textures_delta);
        out
    }

    /// One frame with the pointer and key `events`, as a user gives them
    /// (the tutorial steps, `tutorial_steps.rs`).
    pub(crate) fn input(&mut self, app: &mut EmbApp, size: Vec2, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut out = self.frame_with(app, size, events);
        self.painter.free(&mut out.textures_delta);
        out
    }

    /// Draws frames until the windows have their sizes and the background
    /// work of the program is done (or a short time went by), and gives
    /// the picture of the last frame.
    pub(crate) fn shot(&mut self, app: &mut EmbApp, size: Vec2) -> Shot {
        let mut out = self.frame(app, size);
        for i in 0..40 {
            let wait = out
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map_or(std::time::Duration::MAX, |v| v.repaint_delay);
            if i >= 3 {
                if wait > std::time::Duration::from_millis(300) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            self.painter.free(&mut out.textures_delta);
            out = self.frame(app, size);
        }
        let texts = texts(&out.shapes);
        let ppp = out.pixels_per_point;
        let primitives = self.ctx.tessellate(std::mem::take(&mut out.shapes), ppp);
        let pixels = [(size.x * ppp).round() as usize, (size.y * ppp).round() as usize];
        let clear = self.ctx.global_style().visuals.panel_fill;
        let picture = self.painter.paint(&primitives, ppp, pixels, clear);
        self.painter.free(&mut out.textures_delta);
        let windows = self.ctx.memory(|m| {
            let visible = m.areas().visible_layer_ids();
            m.layer_ids()
                .filter(|l| visible.contains(l) && matches!(l.order, egui::Order::Middle | egui::Order::Foreground))
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        });
        Shot {
            picture,
            texts,
            windows,
        }
    }
}

impl Studio {
    /// The picture of a piece of user interface that is not a window of
    /// the program (the picture lists that open as menus).
    fn paint(&mut self, size: Vec2, mut add: impl FnMut(&mut egui::Ui)) -> Picture {
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        };
        let mut out = self.ctx.run_ui(input(), &mut add);
        self.painter.set(&mut out.textures_delta);
        for _ in 0..3 {
            self.painter.free(&mut out.textures_delta);
            out = self.ctx.run_ui(input(), &mut add);
            self.painter.set(&mut out.textures_delta);
        }
        let ppp = out.pixels_per_point;
        let primitives = self.ctx.tessellate(std::mem::take(&mut out.shapes), ppp);
        let pixels = [(size.x * ppp).round() as usize, (size.y * ppp).round() as usize];
        let clear = self.ctx.global_style().visuals.panel_fill;
        let picture = self.painter.paint(&primitives, ppp, pixels, clear);
        self.painter.free(&mut out.textures_delta);
        picture
    }
}

/// What a scene shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The whole program, at the two screen sizes.
    Screen,
    /// The windows that are open, at the small screen size.
    Windows,
}

/// Puts the program in the state of a scene.
type Setup = Box<dyn Fn(&mut AppState, &egui::Context)>;

/// A state of the program to make a picture of.
struct Scene {
    /// An English name, the same in each language.
    name: String,
    view: View,
    setup: Setup,
}

impl Scene {
    fn new(name: impl Into<String>, view: View, setup: impl Fn(&mut AppState, &egui::Context) + 'static) -> Scene {
        Scene {
            name: name.into(),
            view,
            setup: Box::new(setup),
        }
    }
}

/// The program with the sample design, as a user of `lang` sees it.
pub(crate) fn base(lang: &str) -> AppState {
    let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
    st.start_on_machine("husqvarna_topaz20", None);
    st.add_sample_objects();
    st.selection.clear();
    st.settings.language = lang.into();
    st
}

/// Runs a command as a click on its button does: on all objects if it
/// needs a selection, else with no selection.
fn run_command(st: &mut AppState, ctx: &egui::Context, cmd: Cmd) {
    st.select_all();
    if !st.can(cmd, &SelInfo::of(st)) {
        st.selection.clear();
    }
    if st.can(cmd, &SelInfo::of(st)) {
        st.run(cmd, ctx);
    }
}

/// All scenes: the ribbon tabs, the panels, the object kinds in Object
/// Properties, the screens, and each command that can open a window.
fn scenes() -> Vec<Scene> {
    let mut v = Vec::new();
    for &tab in RibbonTab::visible(false) {
        v.push(Scene::new(format!("ribbon {tab:?}"), View::Screen, move |st, _| {
            st.settings.ribbon_tab = tab;
        }));
    }
    v.push(Scene::new("ribbon Sketch", View::Screen, |st, _| st.start_sketch()));
    for &tab in RibbonTab::visible(true) {
        v.push(Scene::new(
            format!("simple mode {tab:?}"),
            View::Screen,
            move |st, _| {
                st.settings.simple_mode = true;
                st.settings.ribbon_tab = tab;
            },
        ));
    }
    for tab in [
        Tab::Sequence,
        Tab::Threads,
        Tab::StitchList,
        Tab::DesignInfo,
        Tab::Styles,
        Tab::Library,
        Tab::Exports,
        Tab::Clipart,
    ] {
        v.push(Scene::new(format!("panel {tab:?}"), View::Screen, move |st, _| {
            st.select_all();
            st.focus_tab = Some(tab);
            // A fabric with a topping, so the Topping check box shows.
            if tab == Tab::Styles {
                let fabrics = st.fabrics.clone();
                st.doc.edit("Fabric", |d| {
                    d.set_fabric(&fabrics, Some("towel"), false);
                });
            }
        }));
    }
    v.push(Scene::new("start page", View::Screen, |st, _| st.show_start_page()));
    v.push(Scene::new("lettering kiosk", View::Screen, |st, _| st.open_kiosk()));
    let design = base("en");
    for (i, o) in design.doc.design().objects().iter().enumerate() {
        let name = format!("object properties {i:02} {}", o.kind.type_name());
        v.push(Scene::new(name, View::Screen, move |st, _| {
            let id = st.doc.design().objects()[i].id;
            st.selection.set(id);
            st.focus_tab = Some(Tab::Properties);
        }));
    }
    // An appliqué object (its fabric color and fabric surface, ADR 0128).
    v.push(Scene::new("object properties applique", View::Screen, |st, _| {
        let thread = st.doc.design().threads[0].id;
        let mut id = None;
        st.doc.edit("Appliqué", |d| {
            id = Some(d.add_object(
                thread,
                emb_model::ObjectKind::Applique {
                    pieces: vec![emb_model::AppliquePiece::new(
                        emb_geom::Path::ellipse(emb_geom::Point::new(0.0, 60.0), 15.0, 15.0),
                        Vec::new(),
                    )],
                    params: emb_model::AppliqueParams::default(),
                },
            ));
        });
        if let Some(id) = id {
            st.selection.set(id);
        }
        st.focus_tab = Some(Tab::Properties);
    }));
    // The windows that no command opens with the sample design.
    v.push(Scene::new("first start", View::Windows, |st, _| {
        st.ask_machine = Some(crate::first_start::MachineStep::new(st));
    }));
    v.push(Scene::new("update question", View::Windows, |st, _| {
        st.updates.ask = true
    }));
    for (name, status) in [
        (
            "update found",
            crate::update::Status::Found(crate::update::Release {
                version: "9.0.0".into(),
                url: crate::update::RELEASES_PAGE.into(),
                notes: "Tuned fonts.\nMore motifs.".into(),
            }),
        ),
        ("update newest", crate::update::Status::Newest),
        (
            "update failed",
            crate::update::Status::Failed(crate::update::Problem::NoCurl),
        ),
    ] {
        v.push(Scene::new(name, View::Windows, move |st, _| {
            st.updates.window = true;
            st.updates.status = status.clone();
        }));
    }
    // The windows that need a picture, a product or a place on the canvas.
    v.push(Scene::new("picture to design", View::Windows, |st, _| {
        let data = crate::product_photo::tests::shirt_png();
        let size = image::load_from_memory(&data).map_or((1, 1), |i| (i.width(), i.height()));
        st.doc.edit("Add backdrop", |d| {
            d.backdrop = Some(emb_model::Backdrop::new(
                "picture",
                data,
                size,
                emb_geom::Point::ZERO,
                80.0,
            ));
        });
        st.open_wizard();
    }));
    v.push(Scene::new("lettering window", View::Windows, |st, _| {
        let font = st.letter_font.clone();
        let dialog = crate::lettering::LetteringDialog::new(emb_geom::Point::ZERO, "Name", font, 12.0);
        st.open_lettering_dialog(dialog);
    }));
    // Text that needs shaping (ADR 0114): the text box shows the
    // characters, and the canvas shows the shaped lettering.
    v.push(Scene::new("lettering window arabic", View::Screen, |st, _| {
        let (family, style) = emb_letter::bundled::ARABIC;
        let font = emb_model::FontRef::new(family, style);
        let text = "\u{0633}\u{0644}\u{0627}\u{0645}\u{0020}\u{0639}\u{0644}\u{064A}\u{0643}\u{0645}";
        let dialog = crate::lettering::LetteringDialog::new(emb_geom::Point::ZERO, text, font, 20.0);
        st.open_lettering_dialog(dialog);
    }));
    v.push(Scene::new("lettering window devanagari", View::Screen, |st, _| {
        let (family, style) = emb_letter::bundled::DEVANAGARI;
        let font = emb_model::FontRef::new(family, style);
        let text = "\u{0928}\u{092E}\u{0938}\u{094D}\u{0924}\u{0947}";
        let dialog = crate::lettering::LetteringDialog::new(emb_geom::Point::ZERO, text, font, 20.0);
        st.open_lettering_dialog(dialog);
    }));
    v.push(Scene::new(
        "lettering window missing letters",
        View::Windows,
        |st, _| {
            let font = st.letter_font.clone();
            let text = "\u{0633}\u{0644}\u{0627}\u{0645}\u{0020}\u{0639}\u{0644}\u{064A}\u{0643}\u{0645}";
            let dialog = crate::lettering::LetteringDialog::new(emb_geom::Point::ZERO, text, font, 20.0);
            st.open_lettering_dialog(dialog);
        },
    ));
    // The Fonts window with the quality score of a font (ADR 0115), in the
    // order of the scores.
    v.push(Scene::new("fonts window quality", View::Windows, |st, _| {
        st.fonts_window.open = true;
        st.fonts_window.by_quality = true;
        st.fonts_window.select(emb_model::FontRef::embroidery("Brush Hand", ""));
    }));
    for kind in emb_model::family::Kind::ALL {
        let name = match kind {
            emb_model::family::Kind::Motif => "families window motif",
            emb_model::family::Kind::Split => "families window split pattern",
            emb_model::family::Kind::Stamp => "families window stamp",
            emb_model::family::Kind::Clipart => "families window clipart",
        };
        v.push(Scene::new(name, View::Windows, move |st, _| {
            st.families_window.show(kind);
            st.families_window.choose(1);
        }));
    }
    v.push(Scene::new("fonts window convert", View::Windows, |st, _| {
        st.fonts_window.open = true;
        st.fonts_window.open_convert(emb_model::FontRef::new("Lato", "Bold"));
    }));
    v.push(Scene::new("fonts window fill fonts", View::Windows, |st, _| {
        st.fonts_window.open = true;
        st.fonts_window.large_text = true;
        st.fonts_window
            .select(emb_model::FontRef::embroidery("Slab Heavy Fill", ""));
    }));
    v.push(Scene::new("product window with a product", View::Windows, |st, ctx| {
        st.set_product(emb_model::ProductKind::ALL[0]);
        run_command(st, ctx, Cmd::ProductWindow);
    }));
    v.push(Scene::new("turntable with a product", View::Windows, |st, ctx| {
        st.set_product(emb_model::ProductKind::ALL[0]);
        run_command(st, ctx, Cmd::Turntable);
    }));
    // The tilt view on the canvas and the checks of the surface (ADR 0129).
    v.push(Scene::new("tilt view on the canvas", View::Screen, |st, _| {
        st.stitch_tiles.sync = true;
        st.tilt.on_canvas = true;
    }));
    // The stitches by the kind of their part (ADR 0131).
    v.push(Scene::new("recognize objects", View::Windows, |st, _| {
        // The stitches of the sample design as a stitch object.
        let plan = st.plan_view().plan.clone();
        let d = emb_model::design_from_stitches(&plan.stitches, "sample");
        st.doc = emb_model::Document::new(d);
        st.select_all();
        st.open_recognize_objects();
    }));
    // With the outlines of the design (ADR 0137).
    v.push(Scene::new("recognize objects with outlines", View::Windows, |st, _| {
        let svg = emb_render::svg::outlines(st.doc.design()).unwrap_or_default();
        let plan = st.plan_view().plan.clone();
        let d = emb_model::design_from_stitches(&plan.stitches, "sample");
        st.doc = emb_model::Document::new(d);
        st.select_all();
        st.open_recognize_objects();
        if let (Some(w), Ok(read)) = (
            st.recognize_dialog.as_mut(),
            emb_formats::svg::import_svg_vectors(svg.as_bytes()),
        ) {
            w.set_outlines("sample outlines.svg", crate::recognize_objects::outlines_of(&read));
        }
    }));
    v.push(Scene::new("stitch parts", View::Screen, |st, _| {
        st.stitch_tiles.sync = true;
        st.parts_view.on = true;
    }));
    v.push(Scene::new("surface checks", View::Screen, |st, _| {
        st.stitch_tiles.sync = true;
        st.show_through = true;
        st.show_high_thread = true;
    }));
    v.push(Scene::new("product photo", View::Windows, |st, _| {
        let data = crate::product_photo::tests::shirt_png();
        if st.set_product_picture(data, "shirt.png", 500.0).is_ok() {
            st.photo_editor.open = true;
        }
    }));
    let mut seen: Vec<Cmd> = Vec::new();
    for (cmd, tab, _) in crate::ribbon::all_commands() {
        if cmd.opens_dialog() || seen.contains(&cmd) {
            continue;
        }
        seen.push(cmd);
        v.push(Scene::new(format!("{tab:?} {cmd:?}"), View::Windows, move |st, ctx| {
            run_command(st, ctx, cmd);
        }));
    }
    v
}

/// A file name from the name of a scene.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// What the run found for one scene in one language.
#[derive(Debug, Default, Clone)]
struct Found {
    /// The files (a label and the path from the output folder).
    files: Vec<(String, String)>,
    /// The size of the windows together, in points.
    size: Option<Vec2>,
    /// The windows do not fit the small screen.
    too_large: bool,
    /// Texts that show only in part.
    cut: Vec<String>,
    /// Texts that the program made shorter.
    elided: Vec<String>,
    /// The error of a scene that did not draw.
    error: Option<String>,
}

/// Draws one scene in one language and writes its pictures to `out` (no
/// files if `out` is `None`).
fn draw_scene(studio: &mut Studio, scene: &Scene, lang: &str, out: Option<&Path>) -> Found {
    let mut app = EmbApp::for_test(base(lang));
    (scene.setup)(app.state(), &studio.ctx);
    let mut result = Found::default();
    let file = slug(&scene.name);
    let save = |picture: &Picture, folder: &str, label: String, name: String, result: &mut Found| {
        let Some(out) = out else { return };
        let path = format!("{folder}/{name}");
        match picture.save(&out.join(&path)) {
            Ok(()) => result.files.push((label, path)),
            Err(e) => result.error = Some(e),
        }
    };
    let found = |shot: &Shot, area: Rect, result: &mut Found| {
        for t in shot.texts.iter().filter(|t| t.rect.intersects(area)) {
            if t.elided {
                result.elided.push(t.text.clone());
            } else if t.cut() {
                result.cut.push(t.text.clone());
            }
        }
    };
    match scene.view {
        View::Screen => {
            for size in [SMALL, LARGE] {
                let shot = studio.shot(&mut app, size);
                let label = format!("{} × {}", size.x, size.y);
                let name = format!("{file}.{lang}.{}x{}.png", size.x, size.y);
                save(&shot.picture, "screens", label, name, &mut result);
                if size == SMALL {
                    found(&shot, Rect::from_min_size(Pos2::ZERO, size), &mut result);
                }
            }
        }
        View::Windows => {
            let shot = studio.shot(&mut app, SMALL);
            let Some(all) = shot.windows.iter().copied().reduce(Rect::union) else {
                return result;
            };
            let screen = Rect::from_min_size(Pos2::ZERO, SMALL);
            result.too_large = shot.windows.iter().any(|w| !screen.expand(0.5).contains_rect(*w));
            result.size = Some(all.size());
            found(&shot, all, &mut result);
            let picture = shot.picture.crop(all.expand(8.0).intersect(screen));
            let label = format!("{:.0} × {:.0}", all.width(), all.height());
            save(&picture, "windows", label, format!("{file}.{lang}.png"), &mut result);
        }
    }
    result
}

/// Draws each scene in each language. A scene that panics gets its error
/// in the result, and the other scenes go on.
fn draw_all(scenes: &[Scene], langs: &[&str], out: Option<&Path>) -> Vec<Vec<Found>> {
    let mut all: Vec<Vec<Found>> = scenes.iter().map(|_| Vec::new()).collect();
    for lang in langs {
        crate::i18n::set_language(lang);
        let mut studio = Studio::new();
        for (i, scene) in scenes.iter().enumerate() {
            let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                draw_scene(&mut studio, scene, lang, out)
            }));
            all[i].push(drawn.unwrap_or_else(|e| {
                // A frame that did not end leaves the context in a bad state.
                studio = Studio::new();
                let text = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).to_string()))
                    .unwrap_or_else(|| "a panic".into());
                Found {
                    error: Some(text),
                    ..Default::default()
                }
            }));
        }
    }
    crate::i18n::set_language("en");
    all
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The page that shows the pictures of each scene next to each other.
fn index_page(scenes: &[Scene], langs: &[&str], results: &[Vec<Found>]) -> String {
    let mut page = String::from(
        "<!doctype html>\n<meta charset=\"utf-8\">\n<title>ShokujoDX: pictures of the windows</title>\n\
         <style>\nbody{font:14px sans-serif;background:#1e1e1e;color:#ddd;margin:16px}\n\
         .row{display:grid;gap:8px;margin-bottom:28px}\nimg{max-width:100%;border:1px solid #555}\n\
         figure{margin:0}\nfigcaption{color:#aaa;margin-top:2px}\na{color:#8cf}\n.note{color:#fa0}\n\
         .error{color:#f66}\n</style>\n<h1>ShokujoDX: pictures of the windows</h1>\n\
         <p>Each row is one screen or window in each language. See also <a href=\"report.txt\">report.txt</a>.</p>\n",
    );
    for view in [View::Screen, View::Windows] {
        page.push_str(if view == View::Screen {
            "<h2>Screens</h2>\n"
        } else {
            "<h2>Windows</h2>\n"
        });
        for (scene, row) in scenes.iter().zip(results).filter(|(s, _)| s.view == view) {
            if row.iter().all(|r| r.files.is_empty() && r.error.is_none()) {
                continue;
            }
            page.push_str(&format!(
                "<h3 id=\"{}\">{}</h3>\n<div class=\"row\" style=\"grid-template-columns:repeat({},1fr)\">\n",
                slug(&scene.name),
                escape(&scene.name),
                langs.len()
            ));
            for (lang, r) in langs.iter().zip(row) {
                page.push_str("<figure>");
                if let Some((_, first)) = r.files.first() {
                    page.push_str(&format!(
                        "<a href=\"{first}\"><img loading=\"lazy\" src=\"{first}\" alt=\"\"></a>"
                    ));
                }
                page.push_str(&format!("<figcaption>{lang}"));
                for (label, path) in &r.files {
                    page.push_str(&format!(" · <a href=\"{path}\">{}</a>", escape(label)));
                }
                let n = r.cut.len() + r.elided.len();
                if n > 0 {
                    page.push_str(&format!(" · <span class=\"note\">{n} texts do not fit</span>"));
                }
                if r.too_large {
                    page.push_str(" · <span class=\"note\">larger than the small screen</span>");
                }
                if let Some(e) = &r.error {
                    page.push_str(&format!(" · <span class=\"error\">{}</span>", escape(e)));
                }
                page.push_str("</figcaption></figure>\n");
            }
            page.push_str("</div>\n");
        }
    }
    page
}

/// The list of what the run found, as plain text.
fn report(scenes: &[Scene], langs: &[&str], results: &[Vec<Found>]) -> String {
    let mut errors = Vec::new();
    let mut large = Vec::new();
    let mut wide = Vec::new();
    let mut more = Vec::new();
    let mut all = Vec::new();
    for (scene, row) in scenes.iter().zip(results) {
        // The first language is English: the others are compared with it.
        let english = row.first();
        for (lang, r) in langs.iter().zip(row) {
            if let Some(e) = &r.error {
                errors.push(format!("{} [{lang}]: {e}", scene.name));
            }
            if r.too_large {
                let size = r.size.unwrap_or_default();
                large.push(format!("{} [{lang}]: {:.0} × {:.0}", scene.name, size.x, size.y));
            }
            if let (Some(size), Some(en)) = (r.size, english.and_then(|e| e.size))
                && size.x > en.x * 1.2 + 40.0
            {
                wide.push(format!(
                    "{} [{lang}]: {:.0} wide, {:.0} in English",
                    scene.name, size.x, en.x
                ));
            }
            let list = |texts: &[String], what: &str| -> Vec<String> {
                texts
                    .iter()
                    .map(|t| format!("    {what}: {}", t.replace('\n', " ")))
                    .collect()
            };
            let mut lines = list(&r.cut, "cut");
            lines.extend(list(&r.elided, "made shorter"));
            if lines.is_empty() {
                continue;
            }
            let block = format!("{} [{lang}]\n{}", scene.name, lines.join("\n"));
            let n_english = english.map_or(0, |e| e.cut.len() + e.elided.len());
            if lines.len() > n_english {
                more.push(block.clone());
            }
            all.push(block);
        }
    }
    let pictures: usize = results.iter().flatten().map(|r| r.files.len()).sum();
    let mut out = format!(
        "ShokujoDX: pictures of the windows\n{} scenes, {} languages, {pictures} pictures\n",
        scenes.len(),
        langs.len()
    );
    for (title, list) in [
        ("Errors (the test fails)", &errors),
        ("Windows that do not fit a screen of 1366 × 768", &large),
        ("Windows much wider than in English", &wide),
        ("Texts that do not fit: more than in English", &more),
        ("Texts that do not fit: all", &all),
    ] {
        out.push_str(&format!("\n== {title}: {}\n", list.len()));
        for line in list {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The folder for the pictures: `EMB_PICTURES_OUT`, or `target/window-pictures`.
/// The file with the hash of each picture of the last runs.
const HASHES: &str = "hashes.txt";

/// A hash of the bytes of a file (FNV-1a, the same in each run and on
/// each computer).
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Compares the pictures `files` (paths from `out`) with the hashes of
/// the runs before, and writes the new hashes (the hashes of pictures
/// that this run did not draw stay). Returns the pictures that are new
/// and the pictures that changed. So after a change of the program, only
/// these pictures need a look (the review of P17, lane 2).
fn changes(out: &Path, files: &[String]) -> (Vec<String>, Vec<String>) {
    let mut known: std::collections::BTreeMap<String, u64> = std::fs::read_to_string(out.join(HASHES))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (path, h) = l.rsplit_once('\t')?;
            Some((path.to_string(), u64::from_str_radix(h, 16).ok()?))
        })
        .collect();
    let (mut new, mut changed) = (Vec::new(), Vec::new());
    for path in files {
        let Ok(bytes) = std::fs::read(out.join(path)) else {
            continue;
        };
        let h = hash(&bytes);
        match known.insert(path.clone(), h) {
            None => new.push(path.clone()),
            Some(old) if old != h => changed.push(path.clone()),
            Some(_) => {}
        }
    }
    let text: String = known.iter().map(|(path, h)| format!("{path}\t{h:016x}\n")).collect();
    let _ = std::fs::write(out.join(HASHES), text);
    (new, changed)
}

fn out_dir() -> PathBuf {
    std::env::var_os("EMB_PICTURES_OUT").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target")
                .join("window-pictures")
        },
        PathBuf::from,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The painter gives the colors that egui asks for: a plain area, an
    /// area with alpha over it, and text.
    #[test]
    fn the_painter_draws_areas_and_text() {
        let ctx = egui::Context::default();
        let mut painter = Painter::default();
        let size = Vec2::new(200.0, 100.0);
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let p = ui.painter();
                p.rect_filled(
                    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(40.0)),
                    0.0,
                    Color32::RED,
                );
                p.rect_filled(
                    Rect::from_min_size(Pos2::new(30.0, 30.0), Vec2::splat(40.0)),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 0, 255, 128),
                );
                p.text(
                    Pos2::new(100.0, 20.0),
                    egui::Align2::LEFT_TOP,
                    "Text",
                    egui::FontId::proportional(20.0),
                    Color32::WHITE,
                );
            });
            painter.set(&mut out.textures_delta);
            out.textures_delta.clear();
            shapes = std::mem::take(&mut out.shapes);
        }
        let primitives = ctx.tessellate(shapes, 1.0);
        let picture = painter.paint(&primitives, 1.0, [200, 100], Color32::BLACK);
        assert_eq!(picture.at(20, 20), [255, 0, 0, 255], "the plain area");
        assert_eq!(picture.at(150, 80), [0, 0, 0, 255], "the background");
        // Blue with half alpha over red, and over the background.
        let [r, g, b, _] = picture.at(40, 40);
        assert!(
            (120..=135).contains(&r) && g == 0 && (120..=135).contains(&b),
            "{r} {g} {b}"
        );
        let [r, _, b, _] = picture.at(60, 60);
        assert!(r == 0 && (120..=135).contains(&b), "{r} {b}");
        // The text has white pixels and pixels with a part of the white.
        let area = picture.crop(Rect::from_min_size(Pos2::new(100.0, 20.0), Vec2::new(60.0, 24.0)));
        let white = area.pixels.iter().filter(|p| p[0] > 240).count();
        let soft = area.pixels.iter().filter(|p| p[0] > 40 && p[0] < 220).count();
        assert!(white > 20 && soft > 20, "{white} white, {soft} soft");
        assert!(white < area.pixels.len() / 2);
    }

    /// Two triangles with the same edge paint each pixel one time.
    #[test]
    fn an_edge_of_two_triangles_is_painted_one_time() {
        let mut picture = Picture {
            width: 8,
            height: 8,
            pixels: vec![[0, 0, 0, 255]; 64],
        };
        let texture = Texture {
            size: [1, 1],
            pixels: vec![Color32::WHITE],
            linear: true,
        };
        let half = [0.5, 0.5, 0.5, 0.5];
        let c = |x: f32, y: f32| Corner {
            x,
            y,
            u: 0.5,
            v: 0.5,
            color: half,
        };
        // A square of 8 pixels, cut along the diagonal through pixel centers.
        triangle(
            &mut picture,
            [0, 0, 8, 8],
            [c(0.0, 0.0), c(8.0, 0.0), c(8.0, 8.0)],
            &texture,
        );
        triangle(
            &mut picture,
            [0, 0, 8, 8],
            [c(0.0, 0.0), c(8.0, 8.0), c(0.0, 8.0)],
            &texture,
        );
        assert!(picture.pixels.iter().all(|p| *p == [128, 128, 128, 255]));
    }

    #[test]
    fn a_text_that_goes_out_of_its_area_is_found() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(100.0, 14.0));
        let text = |clip: Rect| Text {
            text: "Stichlänge".into(),
            rect,
            clip,
            elided: false,
        };
        assert!(!text(rect.expand(2.0)).cut());
        assert!(text(Rect::from_min_max(Pos2::ZERO, Pos2::new(80.0, 40.0))).cut());
        // Scrolled up, or all of it outside: not a text that does not fit.
        assert!(!text(Rect::from_min_max(Pos2::new(0.0, 15.0), Pos2::new(80.0, 40.0))).cut());
        assert!(!text(Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 40.0))).cut());
    }

    #[test]
    fn names_of_scenes_are_file_names() {
        assert_eq!(slug("Digitize Tool(Knife)"), "digitize-tool-knife");
        assert_eq!(slug("Help HelpTopic(\"whats-new\")"), "help-helptopic-whats-new");
        let all = scenes();
        let mut names: Vec<String> = all.iter().map(|s| slug(&s.name)).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "two scenes with the same file name");
        assert!(n > 100, "{n} scenes");
    }

    /// One screen and one window draw in English, with no files: the
    /// picture has the window, and its texts are found.
    #[test]
    fn a_screen_and_a_window_draw() {
        let all = scenes();
        let pick = |name: &str| all.iter().position(|s| s.name == name).unwrap();
        let mut studio = Studio::new();
        let mut app = EmbApp::for_test(base("en"));
        (all[pick("Help About")].setup)(app.state(), &studio.ctx);
        let shot = studio.shot(&mut app, SMALL);
        assert_eq!(shot.windows.len(), 1, "the About window");
        let window = shot.picture.crop(shot.windows[0]);
        assert!(window.width > 200 && window.height > 100);
        let mut colors: Vec<[u8; 4]> = window.pixels.clone();
        colors.sort_unstable();
        colors.dedup();
        assert!(colors.len() > 20, "the window has text: {} colors", colors.len());
        assert!(
            shot.texts
                .iter()
                .any(|t| t.text.contains("GNU General Public License") && shot.windows[0].contains_rect(t.rect))
        );
        let row = draw_all(&all[pick("ribbon Digitize")..=pick("ribbon Digitize")], &["en"], None);
        assert!(row[0][0].error.is_none() && row[0][0].files.is_empty());
    }

    /// Writes pictures of the picture lists of the patterns, the stamps and
    /// the borders (the lists open as menus, so no scene shows them):
    /// `cargo test --release -p emb-app -- --ignored write_list_pictures`.
    #[test]
    #[ignore = "writes pictures of the lists; run it by hand in a release build"]
    fn write_list_pictures() {
        use crate::item_pictures::{COLUMNS, FramePictures, border, cell, pattern, stamp};
        let dir = out_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mut studio = Studio::new();
        let mut frames = FramePictures::default();
        let grid = |ui: &mut egui::Ui, n: usize, one: &mut dyn FnMut(&mut egui::Ui, usize)| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::Grid::new("list").spacing([6.0, 6.0]).show(ui, |ui| {
                    for i in 0..n {
                        one(ui, i);
                        if i % COLUMNS == COLUMNS - 1 {
                            ui.end_row();
                        }
                    }
                });
            });
        };
        let patterns = emb_model::split::split_patterns();
        let stamps = emb_model::split::stamps();
        let borders = emb_model::decor::frames();
        let size = |n: usize| Vec2::new(390.0, 30.0 + 84.0 * n.div_ceil(COLUMNS) as f32);
        let picture = studio.paint(size(patterns.len()), |ui| {
            grid(ui, patterns.len(), &mut |ui, i| {
                let p = &patterns[i];
                cell(ui, i == 0, &p.name, |painter, rect, ink| {
                    pattern(painter, rect, p, 4.0, ink)
                });
            })
        });
        picture.save(&dir.join("list-patterns.png")).unwrap();
        let picture = studio.paint(size(stamps.len()), |ui| {
            grid(ui, stamps.len(), &mut |ui, i| {
                let s = &stamps[i];
                cell(ui, false, &s.name, |painter, rect, ink| stamp(painter, rect, s, ink));
            })
        });
        picture.save(&dir.join("list-stamps.png")).unwrap();
        let picture = studio.paint(size(borders.len()), |ui| {
            grid(ui, borders.len(), &mut |ui, i| {
                let lines = frames.make(&borders[i]);
                cell(ui, false, &borders[i].name, |painter, rect, ink| {
                    border(painter, rect, &lines, ink)
                });
            })
        });
        picture.save(&dir.join("list-borders.png")).unwrap();
    }

    #[test]
    fn the_pictures_that_changed_are_found() {
        let dir = std::env::temp_dir().join(format!("emb-picture-hashes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("windows")).unwrap();
        let file = |name: &str, bytes: &[u8]| std::fs::write(dir.join(name), bytes).unwrap();
        file("windows/a.png", b"one");
        file("windows/b.png", b"two");
        let all = vec!["windows/a.png".to_string(), "windows/b.png".to_string()];
        // The first run: all pictures are new.
        assert_eq!(changes(&dir, &all), (all.clone(), vec![]));
        // The same pictures again: no change.
        assert_eq!(changes(&dir, &all), (vec![], vec![]));
        // One picture changed. A run of a part of the pictures keeps the
        // hashes of the others.
        file("windows/b.png", b"three");
        assert_eq!(changes(&dir, &all[1..]), (vec![], vec![all[1].clone()]));
        file("windows/a.png", b"four");
        assert_eq!(changes(&dir, &all), (vec![], vec![all[0].clone()]));
        // A picture that is not there is not counted.
        assert_eq!(changes(&dir, &["windows/none.png".to_string()]), (vec![], vec![]));
        assert_ne!(hash(b"one"), hash(b"two"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The pictures of all windows in all languages. Run it by hand in a
    /// release build (see the top of this file).
    #[test]
    #[ignore = "writes the pictures of all windows; run it by hand in a release build"]
    fn write_window_pictures() {
        let out = out_dir();
        // `EMB_PICTURES_ONLY=word`: only the scenes with the word in their
        // name; `EMB_PICTURES_LANGS=en,ja`: only these languages.
        let only = std::env::var("EMB_PICTURES_ONLY").unwrap_or_default().to_lowercase();
        let wanted = std::env::var("EMB_PICTURES_LANGS").unwrap_or_default();
        let all: Vec<Scene> = scenes()
            .into_iter()
            .filter(|s| s.name.to_lowercase().contains(&only))
            .collect();
        // English first: the report compares the other languages with it.
        let langs: Vec<&str> = std::iter::once("en")
            .chain(crate::i18n::languages().iter().map(|l| l.code.as_str()))
            .filter(|l| wanted.is_empty() || wanted.split(',').any(|w| w == *l))
            .collect();
        for folder in ["screens", "windows"] {
            let dir = out.join(folder);
            // A run of all scenes starts with empty folders.
            if only.is_empty() && wanted.is_empty() {
                let _ = std::fs::remove_dir_all(&dir);
            }
            std::fs::create_dir_all(&dir).unwrap();
        }
        let start = std::time::Instant::now();
        let results = draw_all(&all, &langs, Some(&out));
        std::fs::write(out.join("index.html"), index_page(&all, &langs, &results)).unwrap();
        let mut text = report(&all, &langs, &results);
        // The pictures that are not the pictures of the run before.
        let files: Vec<String> = results
            .iter()
            .flatten()
            .flat_map(|r| r.files.iter().map(|(_, path)| path.clone()))
            .collect();
        let (new, changed) = changes(&out, &files);
        let list = |title: &str, paths: &[String]| {
            let mut s = format!("== {title}: {}\n", paths.len());
            for p in paths {
                s.push_str(&format!("{p}\n"));
            }
            s
        };
        std::fs::write(
            out.join("changed.txt"),
            format!(
                "{}\n{}",
                list("Pictures that changed since the run before", &changed),
                list("New pictures", &new)
            ),
        )
        .unwrap();
        text = format!(
            "== Pictures that changed since the run before: {} (new: {}; see changed.txt)\n\n{text}",
            changed.len(),
            new.len()
        );
        std::fs::write(out.join("report.txt"), &text).unwrap();
        let end = text.find("\n== Texts that do not fit: all").unwrap_or(text.len());
        println!("{}", &text[..end]);
        println!(
            "{:.0} s. Open {}",
            start.elapsed().as_secs_f64(),
            out.join("index.html").display()
        );
        let errors: Vec<&Found> = results.iter().flatten().filter(|r| r.error.is_some()).collect();
        assert!(
            errors.is_empty(),
            "{} scenes gave an error (see report.txt)",
            errors.len()
        );
    }
}
