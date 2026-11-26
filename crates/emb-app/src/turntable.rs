//! The turntable view (P16 Milestone 8, ADR 0108): the product with its
//! designs turns in steps of 15°, so the customer sees it from more sides.
//!
//! A background job makes the 24 views (`emb_render::turntable`) when the
//! design or the product changes; a turn only shows another finished
//! view, so it is fast on a slow PC too. The views show in the Turntable
//! window (the canvas stays flat: the design is the work there). Save GIF
//! writes the turn as an animated picture; Save Strip writes four views
//! next to each other.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, Color32, Rect, TextureHandle, TextureOptions};
use emb_geom::BBox;
use emb_model::{Design, ProductKind, ProductView, Side};
use emb_render::Segment;
use emb_render::raster::Picture;
use emb_render::turntable::{Body, STEPS};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::{fill, tr};

/// The degrees between two views.
const STEP_DEG: f64 = 360.0 / STEPS as f64;
/// The longest side of a view (pixels), and the most pixels for each mm.
const VIEW_SIDE: f64 = 720.0;
const MAX_PX_PER_MM: f64 = 2.5;
/// The time between two views when the turn plays, and in the GIF (ms).
const PLAY_MS: u64 = 150;
/// After a change, the views are made again when the design stays the
/// same for this long (a drag makes many changes).
const SETTLE: std::time::Duration = std::time::Duration::from_millis(300);
/// The views of the strip: front, a quarter turn, the back quarter, back.
const STRIP: [usize; 4] = [0, 3, 9, 12];

/// A message of the job that makes the views.
pub(crate) enum Msg {
    View(usize, Picture),
}

struct Job {
    rx: Receiver<Msg>,
    stop: Arc<AtomicBool>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The state of the Turntable window.
#[derive(Default)]
pub struct TurntableUi {
    pub open: bool,
    /// The view that shows (0: the front).
    pub step: usize,
    pub playing: bool,
    last_tick: f64,
    /// The design state that the views show.
    state: Option<u64>,
    /// A newer design state and when it came (the views wait for it to
    /// settle).
    changed: Option<(u64, std::time::Instant)>,
    views: Vec<Option<Arc<Picture>>>,
    job: Option<Job>,
    texture: Option<(usize, TextureHandle)>,
    /// A GIF that a thread writes now: its file and the thread.
    saving: Option<(PathBuf, std::thread::JoinHandle<Result<(), String>>)>,
}

impl TurntableUi {
    /// The number of finished views.
    pub fn ready(&self) -> usize {
        self.views.iter().filter(|v| v.is_some()).count()
    }

    /// All views, if all are finished.
    fn all(&self) -> Option<Vec<Arc<Picture>>> {
        self.views
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
            .filter(|v| v.len() == STEPS)
    }
}

/// True if the product of `d` can turn (a drawn product, not a photo).
pub fn can_turn(d: &Design) -> bool {
    d.product.as_ref().is_some_and(|v| v.picture.is_none())
}

/// The pixels for each mm of the views of `kind`.
fn view_px(kind: ProductKind) -> f64 {
    let b = kind.bbox();
    (VIEW_SIDE / b.width().max(b.height() + 20.0)).min(MAX_PX_PER_MM)
}

/// Makes the views of the product of `design` (with the stitches of `plan`)
/// and sends each one when it is finished. It stops when `stop` is set.
pub(crate) fn make_views(
    design: &Design,
    plan: &emb_stitch::StitchPlan,
    machines: &emb_model::MachineCatalog,
    look: &emb_render::raster::Look,
    send: &mut dyn FnMut(Msg) -> bool,
    stop: &AtomicBool,
) {
    let Some(v) = design.product.as_ref().filter(|v| v.picture.is_none()) else {
        return;
    };
    let front_kind = v.kind.front();
    let px = view_px(front_kind);
    let fine = px * 2.0;
    let back = design.from_hoop_transform();
    let open: Vec<Segment> = emb_render::segments(&plan.stitches)
        .into_iter()
        .map(|s| Segment {
            a: back.apply(s.a),
            b: back.apply(s.b),
            ..s
        })
        .collect();
    // A side of the product: its picture and the area that it shows.
    let side = |side: Side| -> (Picture, BBox) {
        let two = front_kind.turned().is_some();
        let mut segments: Vec<Segment> = Vec::new();
        if two || side == Side::Front {
            for e in v.extras.iter().filter(|e| e.visible && e.side == side) {
                let to = v.origin + e.at;
                segments.extend(crate::product::stitch_extra(e, machines).iter().map(|s| Segment {
                    a: s.a + to,
                    b: s.b + to,
                    ..*s
                }));
            }
            if side == v.side() {
                segments.extend_from_slice(&open);
            }
        }
        let view = ProductView {
            kind: front_kind.on(side),
            ..v.clone()
        };
        let area = emb_render::product::turn_area(&view);
        let pic = emb_render::product::turn_side(&view, &segments, fine, crate::product::THREAD_MM, look);
        (pic, area)
    };
    let (front, front_area) = side(Side::Front);
    if stop.load(Ordering::Relaxed) {
        return;
    }
    let (back_pic, back_area) = side(Side::Back);
    let body = Body::new(
        front_kind,
        v.origin,
        emb_render::turntable::Side {
            picture: &front,
            area: front_area,
        },
        emb_render::turntable::Side {
            picture: &back_pic,
            area: back_area,
        },
        px,
    );
    for i in 0..STEPS {
        if stop.load(Ordering::Relaxed) || !send(Msg::View(i, body.view(i as f64 * STEP_DEG))) {
            return;
        }
    }
}

/// The views of the turn as an animated GIF that plays again and again.
pub fn gif_bytes(views: &[Arc<Picture>]) -> Result<Vec<u8>, String> {
    use image::codecs::gif::{GifEncoder, Repeat};
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;
        for v in views {
            let img =
                image::RgbaImage::from_raw(v.width, v.height, v.rgba.clone()).ok_or_else(|| "bad view".to_string())?;
            let frame = image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(PLAY_MS as u32, 1));
            enc.encode_frame(frame).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

/// Four views next to each other: the front, a quarter turn, the back
/// quarter and the back.
pub fn strip(views: &[Arc<Picture>]) -> Option<Picture> {
    let mut out: Option<Picture> = None;
    for &i in &STRIP {
        let v = views.get(i)?;
        out = Some(match out {
            None => (**v).clone(),
            Some(p) => emb_render::product::side_by_side(&p, v, 20),
        });
    }
    out
}

impl AppState {
    /// Takes the finished views, and starts the job again when the design
    /// changed.
    fn poll_turntable(&mut self) {
        let state = self.doc.state_id();
        let first = self.turntable.state.is_none();
        let settled = self
            .turntable
            .changed
            .is_some_and(|(s, t)| s == state && t.elapsed() >= SETTLE);
        if self.turntable.state != Some(state) && !first && !settled {
            // The old views show until the design stays the same.
            if self.turntable.changed.is_none_or(|(s, _)| s != state) {
                self.turntable.changed = Some((state, std::time::Instant::now()));
            }
        } else if self.turntable.state != Some(state) {
            self.turntable.changed = None;
            self.turntable.job = None;
            self.turntable.views = vec![None; STEPS];
            self.turntable.texture = None;
            self.turntable.state = Some(state);
            if can_turn(self.doc.design()) {
                let design = self.doc.design().clone();
                let plan = self.plan_view().plan.clone();
                let machines = self.machines.clone();
                let look = crate::product::picture_look(&self.settings.look);
                let stop = Arc::new(AtomicBool::new(false));
                let (tx, rx) = channel();
                let flag = stop.clone();
                std::thread::spawn(move || {
                    make_views(&design, &plan, &machines, &look, &mut |m| tx.send(m).is_ok(), &flag);
                });
                self.turntable.job = Some(Job { rx, stop });
            }
        }
        if let Some(job) = &self.turntable.job {
            while let Ok(Msg::View(i, pic)) = job.rx.try_recv() {
                if let Some(v) = self.turntable.views.get_mut(i) {
                    *v = Some(Arc::new(pic));
                }
            }
        }
        if self.turntable.ready() == STEPS {
            self.turntable.job = None;
        }
    }

    /// The views of the turn for the design as it is now, made here (tests).
    #[cfg(test)]
    pub fn turntable_views_now(&mut self) -> Vec<Arc<Picture>> {
        let design = self.doc.design().clone();
        let plan = self.plan_view().plan.clone();
        let look = crate::product::picture_look(&self.settings.look);
        let mut out = vec![None; STEPS];
        make_views(
            &design,
            &plan,
            &self.machines,
            &look,
            &mut |Msg::View(i, p)| {
                out[i] = Some(Arc::new(p));
                true
            },
            &AtomicBool::new(false),
        );
        out.into_iter().flatten().collect()
    }

    fn save_turntable_gif(&mut self) {
        let Some(views) = self.turntable.all() else { return };
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Save the turn (GIF)"))
            .add_filter("GIF", &["gif"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{} turn.gif", self.doc.design().name))
            .save_file()
        else {
            return;
        };
        let p = path.clone();
        let handle = std::thread::spawn(move || {
            let bytes = gif_bytes(&views)?;
            std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))
        });
        self.turntable.saving = Some((path, handle));
        self.message = tr("Saving the turn…").into();
    }

    fn save_turntable_strip(&mut self) {
        let Some(pic) = self.turntable.all().and_then(|v| strip(&v)) else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Save four views (PNG)"))
            .add_filter("PNG", &["png"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{} views.png", self.doc.design().name))
            .save_file()
        else {
            return;
        };
        self.finish_save(
            &path,
            std::fs::write(&path, pic.png()).map_err(|e| format!("{}: {e}", path.display())),
        );
    }

    fn finish_save(&mut self, path: &Path, r: Result<(), String>) {
        match r {
            Ok(()) => self.message = fill("Saved {}", &[&path.display()]),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot save the picture".into(),
                    lines: vec![e],
                })
            }
        }
    }
}

/// The Turntable window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if let Some((_, h)) = &st.turntable.saving
        && h.is_finished()
        && let Some((path, h)) = st.turntable.saving.take()
    {
        let r = h.join().unwrap_or_else(|_| Err("the thread stopped".into()));
        st.finish_save(&path, r);
    }
    if !st.turntable.open {
        // No job runs while the window is closed.
        st.turntable.job = None;
        st.turntable.state = None;
        st.turntable.changed = None;
        return;
    }
    st.poll_turntable();
    if st.turntable.job.is_some()
        || st.turntable.changed.is_some()
        || st.turntable.playing
        || st.turntable.saving.is_some()
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    let mut open = true;
    crate::dock::docked(
        egui::Window::new(tr("Turntable")).id(egui::Id::new("Turntable")),
        ctx,
        egui::Id::new("Turntable"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| contents(ui, st));
    if !open {
        st.turntable.open = false;
        st.turntable.playing = false;
    }
}

fn contents(ui: &mut egui::Ui, st: &mut AppState) {
    if !can_turn(st.doc.design()) {
        ui.label(tr(
            "Choose a drawn product first (Design > Product). A photo of a product cannot turn.",
        ));
        return;
    }
    let t = &mut st.turntable;
    // Play: the next view each PLAY_MS, if it is finished.
    let now = ui.input(|i| i.time);
    if t.playing && now - t.last_tick >= PLAY_MS as f64 / 1000.0 {
        let next = (t.step + 1) % STEPS;
        if t.views.get(next).is_some_and(Option::is_some) {
            t.step = next;
        }
        t.last_tick = now;
    }
    let shown = t.views.get(t.step).cloned().flatten();
    let size = egui::vec2(480.0, 480.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let resp = crate::a11y::named(resp, tr("The product from the turn angle"));
    ui.painter().rect_filled(rect, 4.0, Color32::WHITE);
    match shown {
        Some(pic) => {
            if t.texture.as_ref().is_none_or(|(s, _)| *s != t.step) {
                let img =
                    egui::ColorImage::from_rgba_unmultiplied([pic.width as usize, pic.height as usize], &pic.rgba);
                let tex = crate::resources::load_texture(ui.ctx(), "turntable", img, TextureOptions::LINEAR);
                t.texture = Some((t.step, tex));
            }
            if let Some((_, tex)) = &t.texture {
                let k = (rect.width() / pic.width as f32).min(rect.height() / pic.height as f32);
                let r = Rect::from_center_size(rect.center(), egui::vec2(pic.width as f32 * k, pic.height as f32 * k));
                ui.painter().image(
                    tex.id(),
                    r,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        None => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Making the views…"),
                egui::FontId::proportional(16.0),
                Color32::DARK_GRAY,
            );
        }
    }
    // A drag across the picture turns the product.
    if resp.dragged() {
        let dx = resp.drag_delta().x;
        let acc = ui.memory_mut(|m| {
            let a = m.data.get_temp_mut_or_default::<f32>(egui::Id::new("turntable_drag"));
            *a += dx;
            let steps = (*a / 25.0).trunc();
            *a -= steps * 25.0;
            steps as i64
        });
        if acc != 0 {
            t.step = (t.step as i64 - acc).rem_euclid(STEPS as i64) as usize;
            t.playing = false;
        }
    }
    let ready = t.ready();
    ui.horizontal(|ui| {
        if ui
            .button("◀")
            .on_hover_text(tr("Turn the product 15° to the left."))
            .clicked()
        {
            t.step = (t.step + STEPS - 1) % STEPS;
            t.playing = false;
        }
        let mut deg = t.step as f64 * STEP_DEG;
        if ui
            .add(
                crate::workspace::slider(&mut deg, 0.0..=360.0 - STEP_DEG)
                    .step_by(STEP_DEG)
                    .suffix("°"),
            )
            .changed()
        {
            t.step = (deg / STEP_DEG).round() as usize % STEPS;
            t.playing = false;
        }
        if ui
            .button("▶")
            .on_hover_text(tr("Turn the product 15° to the right."))
            .clicked()
        {
            t.step = (t.step + 1) % STEPS;
            t.playing = false;
        }
        let label = if t.playing { tr("Stop") } else { tr("Play") };
        if ui
            .button(label)
            .on_hover_text(tr("Turn the product round and round."))
            .clicked()
        {
            t.playing = !t.playing;
            t.last_tick = now;
        }
    });
    if ready < STEPS {
        ui.weak(fill("Making the views: {} of {}.", &[&ready, &STEPS]));
    } else {
        ui.weak(tr("Drag the picture to turn the product."));
    }
    let saving = t.saving.is_some();
    ui.horizontal(|ui| {
        ui.add_enabled_ui(ready == STEPS && !saving, |ui| {
            if ui
                .button(tr("Save GIF…"))
                .on_hover_text(tr("Save the turn as an animated picture (GIF)."))
                .clicked()
            {
                st.save_turntable_gif();
            }
            if ui
                .button(tr("Save Four Views…"))
                .on_hover_text(tr(
                    "Save the front, two quarter turns and the back next to each other (PNG).",
                ))
                .clicked()
            {
                st.save_turntable_strip();
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use emb_model::Rgb;

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_t_shirt_with_a_design_turns_and_saves_as_a_gif() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::TShirt);
        h.st.edit_product("Color", None, |v| v.color = Rgb(30, 40, 70));
        let views = h.st.turntable_views_now();
        assert_eq!(views.len(), STEPS);
        // Each view has the same height; the side view is narrower in its
        // product than the front.
        assert!(views.iter().all(|v| v.height == views[0].height));
        let dark = |p: &Picture| p.rgba.as_chunks::<4>().0.iter().filter(|c| c[0] < 120).count();
        assert!(dark(&views[6]) * 3 < dark(&views[0]) * 2);
        let gif = gif_bytes(&views).unwrap();
        let frames = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&gif)).unwrap();
        use image::AnimationDecoder;
        assert_eq!(frames.into_frames().count(), STEPS);
        let s = strip(&views).unwrap();
        assert!(s.width > 4 * views[0].width);
        // A photo of a product cannot turn.
        assert!(can_turn(h.st.doc.design()));
    }

    /// Writes the views of some products as strips of eight views and
    /// GIFs: `EMB_PRODUCT_OUT=<folder> cargo test -p emb-app -- --ignored write_turntables`.
    #[test]
    #[ignore]
    fn write_turntables() {
        let dir = std::path::PathBuf::from(std::env::var("EMB_PRODUCT_OUT").expect("set EMB_PRODUCT_OUT"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::new();
        let print = crate::product::tests::small_project("back-print", Rgb(20, 90, 200));
        for (kind, color) in [
            (ProductKind::TShirt, Rgb(30, 40, 70)),
            (ProductKind::Hoodie, Rgb(170, 170, 172)),
            (ProductKind::Cap, Rgb(200, 30, 40)),
            (ProductKind::Beanie, Rgb(32, 70, 48)),
            (ProductKind::Tote, Rgb(238, 230, 211)),
        ] {
            h.st.doc.edit("None", |d| d.product = None);
            h.st.set_product(kind);
            if kind.turned().is_some() {
                h.st.turn_product();
                h.st.add_product_design(&print).unwrap();
                h.st.turn_product();
            }
            h.st.edit_product("Color", None, |v| v.color = color);
            let views = h.st.turntable_views_now();
            let mut row: Option<Picture> = None;
            for i in (0..STEPS).step_by(3) {
                row = Some(match row {
                    None => (*views[i]).clone(),
                    Some(p) => emb_render::product::side_by_side(&p, &views[i], 10),
                });
            }
            std::fs::write(dir.join(format!("turn-{kind:?}.png")), row.unwrap().png()).unwrap();
            std::fs::write(dir.join(format!("turn-{kind:?}.gif")), gif_bytes(&views).unwrap()).unwrap();
        }
    }

    #[test]
    fn the_window_makes_the_views_in_the_background() {
        let mut h = Harness::new();
        h.st.set_product(ProductKind::Cap);
        h.st.turntable.open = true;
        let ctx = egui::Context::default();
        let state = h.st.doc.state_id();
        let start = std::time::Instant::now();
        while h.st.turntable.ready() < STEPS && start.elapsed().as_secs() < 120 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
            out.textures_delta.clear();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(h.st.turntable.ready(), STEPS);
        assert_eq!(h.st.doc.state_id(), state);
        // A change of the design makes the views again when it settles;
        // the old views show until then.
        h.st.edit_product("Color", None, |v| v.color = Rgb(200, 30, 40));
        let new = h.st.doc.state_id();
        let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
        out.textures_delta.clear();
        assert_eq!(h.st.turntable.ready(), STEPS);
        let start = std::time::Instant::now();
        while h.st.turntable.state != Some(new) && start.elapsed().as_secs() < 5 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
            out.textures_delta.clear();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(h.st.turntable.state, Some(new));
        assert!(h.st.turntable.ready() < STEPS);
    }
}
