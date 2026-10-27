//! Picture to design: a wizard that turns a picture into embroidery step
//! by step (P6 Milestones 7 and 8; owner requests of 2026-10-23; ADR 0029,
//! ADR 0030).
//!
//! 1. Picture: the picture (the backdrop), and what to make: color areas
//!    (a logo, a drawing) or a photo stitch (a portrait, a photo).
//! 2. Background: cut out the subject (a box round it, then marks to keep
//!    or remove parts), click colors to remove them, remove the edge color,
//!    paint with the erase and restore brushes, or erase large parts with a
//!    box or a lasso; shrink the edge, clean up small bits. The preview
//!    zooms (scroll) and pans (right or middle drag).
//! 3. Color areas: Simplify (design width, detail as pixels, colors,
//!    specks, noise), Colors (keep, leave out, merge), Stitches (stitch type
//!    and sew order). Photo stitch: Photo (style, size, rows, contrast).
//! 4. Finish: a picture of the stitches; Finish adds the objects in one
//!    undo step.
//!
//! Each step shows its result on the right. The work is in
//! `emb_art::background`, `emb_art::smart` and `emb_art::photo`; the wizard
//! keeps the results of each step, so a change makes only the later steps
//! again.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, mpsc};

use eframe::egui::{
    self, Color32, ColorImage, PointerButton, Rect, Sense, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use emb_art::RgbaImage;
use emb_art::background::{self, ShapeAction};
use emb_art::cross::{CrossColor, CrossSpec};
use emb_art::cutout::{self, Mark};
use emb_art::photo::{self, PhotoSpec, PhotoStyle};
use emb_art::reduce::{Reduced, match_threads};
use emb_art::smart::{self, ColorChoice, SimplifySpec, SmartObject, SmartSpec, SmartStitch};
use emb_geom::{Point, Transform};
use emb_model::cross::CrossStitch;
use emb_model::{
    CurrentProperties, Design, Lab, ObjectId, ObjectKind, Rgb, StitchKind, ThreadChart, colorway::chart_look,
};

use crate::app::AppState;
use crate::canvas::color32;
use crate::i18n::tr;

/// The stitch picture of the Finish step is about this many pixels wide.
const RESULT_PX: f64 = 700.0;
/// Undo steps of the Background step (each keeps a copy of the picture).
const HISTORY_MAX: usize = 40;
/// Checks of the Finish step (P12 Milestone 2): more trims than this in
/// the design are slow to sew.
const MANY_TRIMS: usize = 30;
/// An object smaller than this on each side sews as a knot, mm.
const TINY_MM: f64 = 1.0;
/// A stitch longer than this can catch, mm.
const LONG_MM: f64 = 7.0;
/// Two kept colors nearer than this (ΔE) are almost the same.
const NEAR_DE: f64 = 8.0;

/// A check of the result on the Finish step, with its fix if one exists
/// (P12 Milestone 2).
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub text: String,
    pub fix: Option<Fix>,
}

/// A change of the settings that a check proposes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fix {
    /// Specks smaller than this go into the color round them, mm².
    Specks(f64),
    /// Less detail: pixels of this size, mm.
    Detail(f64),
    /// The first color goes into the second.
    Merge(u16, u16),
    /// Less overlap: this much, mm.
    Overlap(f64),
    /// Each detail is a hole (no details on top).
    NoLayers,
    /// Auto satin up to this width, mm, and Auto for the satin colors.
    SatinMax(f64),
}

impl Fix {
    /// The text of its button.
    pub fn label(self) -> String {
        match self {
            Fix::Specks(v) => crate::i18n::fill("Remove specks under {} mm²", &[&format!("{v:.1}")]),
            Fix::Detail(v) => crate::i18n::fill("Less detail ({} mm a pixel)", &[&format!("{v:.2}")]),
            Fix::Merge(..) => tr("Merge them").into(),
            Fix::Overlap(v) => crate::i18n::fill("Overlap {} mm", &[&format!("{v:.2}")]),
            Fix::NoLayers => tr("No details on top").into(),
            Fix::SatinMax(v) => crate::i18n::fill("Auto satin up to {} mm", &[&format!("{v:.0}")]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Picture,
    Background,
    Simplify,
    Colors,
    Stitches,
    Photo,
    Cross,
    Finish,
}

/// The steps of each kind of design.
const AREA_STEPS: [Step; 6] = [
    Step::Picture,
    Step::Background,
    Step::Simplify,
    Step::Colors,
    Step::Stitches,
    Step::Finish,
];
const PHOTO_STEPS: [Step; 4] = [Step::Picture, Step::Background, Step::Photo, Step::Finish];
const CROSS_STEPS: [Step; 4] = [Step::Picture, Step::Background, Step::Cross, Step::Finish];

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Picture => tr("Picture"),
            Step::Background => tr("Background"),
            Step::Simplify => tr("Simplify"),
            Step::Colors => tr("Colors"),
            Step::Stitches => tr("Stitches"),
            Step::Photo => tr("Photo stitch"),
            Step::Cross => tr("Cross Stitch"),
            Step::Finish => tr("Finish"),
        }
    }
}

/// What the wizard makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Make {
    /// Fills, satins and outlines of flat color areas (a logo, a drawing).
    Areas,
    /// Rows or scribbles whose density follows the picture (a photo).
    Photo,
    /// Cross-stitch objects on a grid (P9 Milestone 2).
    Cross,
}

/// What a press on the picture does in the Background step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgTool {
    /// A box round the subject: the rest goes (`emb_art::cutout`).
    Subject,
    /// Marks parts of the subject to keep, then cuts again.
    MarkKeep,
    /// Marks parts to remove, then cuts again.
    MarkRemove,
    /// Removes the color under the click.
    Click,
    /// Clears the pixels under the brush.
    Erase,
    /// Brings the pixels under the brush back.
    Restore,
    /// A box: erases (or keeps, or restores) what is in it.
    Box,
    /// A freehand shape: as the box.
    Lasso,
}

/// A mark stroke of the cut-out: points in work pixels, a radius, and keep
/// or remove.
#[derive(Debug, Clone)]
struct MarkStroke {
    keep: bool,
    radius: f64,
    points: Vec<(f64, f64)>,
}

/// The cut-out of the subject: the picture before it, the box and the
/// marks. A new box starts a new cut-out.
#[derive(Debug, Clone)]
struct Subject {
    base: Arc<RgbaImage>,
    rect: Option<[f64; 4]>,
    strokes: Vec<MarkStroke>,
    /// `Wizard::rev` after the last cut: another change makes a new base.
    rev: u64,
}

/// The state of the wizard: the state of the window, and the work state
/// (the picture, the settings and the results of the steps), which a
/// background job can copy and make the previews with (P12 Milestone 1).
/// The wizard derefs to its work state, so the step controls read and
/// change the settings as fields of the wizard.
pub struct Wizard {
    pub step: Step,
    /// The backdrop picture data that the wizard started from.
    data: Arc<Vec<u8>>,
    /// Earlier states of `cut` (and of the cut-out), for Undo.
    history: Vec<(RgbaImage, Option<Subject>)>,
    subject: Option<Subject>,
    /// A mark stroke that goes on.
    mark: Option<MarkStroke>,
    /// What a box or a lasso does.
    pub shape_action: ShapeAction,
    /// The points of a box or a lasso that is dragged (work pixels).
    drag: Vec<(f64, f64)>,
    /// The preview: zoom (1: the picture fits), pan (screen points) and the
    /// step they are for.
    zoom: f32,
    pan: Vec2,
    view_step: Step,
    /// Show the removed parts faintly in the Background step.
    pub show_removed: bool,
    source_tex: Option<TextureHandle>,
    /// Background: the tool, the color tolerance, and only the clicked
    /// area (`true`) or the color everywhere.
    pub tool: BgTool,
    pub tolerance: f64,
    pub connected: bool,
    /// The brush diameter, mm.
    pub brush_mm: f64,
    /// A brush stroke goes on: the last point (work pixels).
    stroke: Option<(f64, f64)>,
    pub group: bool,
    pub hide_backdrop: bool,
    /// The preview picture on the screen and its key.
    texture: Option<(u64, TextureHandle)>,
    /// The background job that makes a preview, if one runs.
    job: Option<Job>,
    /// The key of a preview whose job failed: it is not tried again.
    failed: Option<u64>,
    /// Finish was clicked while a job ran: finish when it ends.
    finish: bool,
    pub note: String,
    work: Work,
}

/// The picture, the settings and the results of the steps (each result
/// with a key of its inputs, so a change makes only the later steps again).
/// A clone is cheap but for `cut` (one copy of the picture).
#[derive(Clone)]
pub struct Work {
    pub make: Make,
    source: Arc<RgbaImage>,
    /// The picture with the background removals so far.
    pub cut: RgbaImage,
    /// Changes each time `cut` changes.
    rev: u64,
    /// Clear this many pixels at the edge of the cut-out.
    pub shrink_px: u32,
    /// Remove bits and fill holes smaller than this, mm².
    pub bits_mm2: f64,
    /// Cut the clear edges off.
    pub trim: bool,
    /// The design width in mm; `None`: as on the backdrop.
    pub width_mm: Option<f64>,
    /// The detail: the size of one pixel of the simplified picture, in mm.
    pub pixel_mm: f64,
    pub colors: usize,
    /// Specks smaller than this take the color round them, in mm².
    pub speck_mm2: f64,
    pub clean_edges: bool,
    /// Noise reduction: the radius of a median, in pixels of the simplified
    /// picture (0: none).
    pub denoise: u32,
    /// Edge-preserving smoothing: passes before the color reduction (0: none).
    pub smooth: u32,
    /// The colors of the thread chart that the color reduction uses (with a
    /// hash of them, for the keys); `None`: the colors of the picture.
    pub chart: Option<(u64, Arc<Vec<Rgb>>)>,
    pub choices: Vec<ColorChoice>,
    /// The sew order of the colors (palette indexes).
    pub order: Vec<u16>,
    pub spec: SmartSpec,
    pub photo: PhotoSpec,
    pub cross: CrossSpec,
    /// The grid of the cross stitch (its cells are not used).
    pub cross_grid: CrossStitch,
    /// Where the backdrop is: its top-left corner, mm per work pixel, and
    /// the design rotation.
    origin: Point,
    mm_per_px: f64,
    rotation: Transform,
    cleaned: Option<(u64, Arc<RgbaImage>)>,
    content: Option<(u64, Arc<Content>)>,
    reduced: Option<(u64, Arc<Reduced>)>,
    result: Option<(u64, Arc<Vec<SmartObject>>)>,
    photo_result: Option<(u64, Arc<Vec<photo::PhotoObject>>)>,
    cross_result: Option<(u64, Arc<Vec<CrossColor>>)>,
    /// The checks of the color-area result (key: the key of the objects).
    checks: Option<(u64, Arc<Vec<Check>>)>,
    /// The density limit of the Options (layers of thread), for the checks.
    pub density_limit: f64,
}

/// A background job that makes the preview picture of a step.
struct Job {
    /// The preview key when the job started.
    key: u64,
    rx: mpsc::Receiver<JobDone>,
}

/// What a job gives back: its work state (with the results it made), the
/// preview picture and its key, and whether to show its pixels sharp.
struct JobDone {
    work: Work,
    key: u64,
    picture: RgbaImage,
    sharp: bool,
}

impl std::ops::Deref for Wizard {
    type Target = Work;
    fn deref(&self) -> &Work {
        &self.work
    }
}

impl std::ops::DerefMut for Wizard {
    fn deref_mut(&mut self) -> &mut Work {
        &mut self.work
    }
}

/// The picture after the background step, and the work pixel of its
/// top-left corner.
type Content = (RgbaImage, (u32, u32));
/// Makes the picture that a step before Simplify shows.
type MakePicture = Box<dyn FnOnce(&mut Work) -> RgbaImage>;

fn hash(v: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

impl Wizard {
    /// A wizard for the backdrop of `d`; an error if there is no backdrop
    /// or it cannot be read. The picture is decoded to the work size with
    /// the memory budget of a decode (ADR 0077).
    pub fn new(d: &Design) -> Result<Wizard, String> {
        let Some(b) = d.backdrop.as_ref() else {
            return Err(tr("Load a backdrop picture first (Artwork > Import > Backdrop Picture).").into());
        };
        let source = smart::decode_work(&b.data, crate::resources::decode_budget()).map_err(|e| {
            crate::i18n::fill(
                "Cannot read the backdrop picture: {}",
                &[&crate::resources::decode_error(&e)],
            )
        })?;
        let mm_per_px = b.width_mm / f64::from(source.width().max(1));
        Ok(Wizard {
            step: Step::Picture,
            data: b.data.clone(),
            history: Vec::new(),
            subject: None,
            mark: None,
            shape_action: ShapeAction::EraseInside,
            drag: Vec::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            view_step: Step::Picture,
            show_removed: true,
            source_tex: None,
            tool: BgTool::Subject,
            tolerance: background::DEFAULT_TOLERANCE,
            connected: true,
            brush_mm: 4.0,
            stroke: None,
            group: true,
            hide_backdrop: true,
            texture: None,
            job: None,
            failed: None,
            finish: false,
            note: String::new(),
            work: Work {
                make: Make::Areas,
                cut: source.clone(),
                source: Arc::new(source),
                rev: 0,
                shrink_px: 0,
                bits_mm2: 0.0,
                trim: true,
                width_mm: None,
                pixel_mm: 0.4,
                colors: 4,
                speck_mm2: 2.0,
                clean_edges: true,
                denoise: 0,
                smooth: 1,
                chart: None,
                choices: Vec::new(),
                order: Vec::new(),
                spec: SmartSpec::default(),
                photo: PhotoSpec::default(),
                cross: CrossSpec::default(),
                cross_grid: CrossStitch::new(d.hoop.center, emb_model::cross::DEFAULT_CELL_MM),
                origin: b.origin,
                mm_per_px,
                rotation: d.rotation_transform(),
                cleaned: None,
                content: None,
                reduced: None,
                result: None,
                photo_result: None,
                cross_result: None,
                checks: None,
                density_limit: emb_render::density::DEFAULT_LIMIT,
            },
        })
    }

    /// The steps for what the wizard makes.
    pub fn steps(&self) -> &'static [Step] {
        match self.work.make {
            Make::Areas => &AREA_STEPS,
            Make::Photo => &PHOTO_STEPS,
            Make::Cross => &CROSS_STEPS,
        }
    }

    fn index(&self) -> usize {
        self.steps().iter().position(|s| *s == self.step).unwrap_or(0)
    }

    fn changed(&mut self, before: RgbaImage) {
        self.remember((before, self.subject.clone()));
        self.work.rev += 1;
    }

    /// Adds an undo step; the oldest go when there are too many.
    fn remember(&mut self, state: (RgbaImage, Option<Subject>)) {
        self.history.push(state);
        if self.history.len() > HISTORY_MAX {
            self.history.remove(0);
        }
    }

    /// Cut out the subject in the box `[x0, y0, x1, y1]` (work pixels): a
    /// new cut-out of the picture as it is now.
    pub fn cut_out(&mut self, rect: [f64; 4]) {
        let r = [
            rect[0].min(rect[2]),
            rect[1].min(rect[3]),
            rect[0].max(rect[2]),
            rect[1].max(rect[3]),
        ];
        if r[2] - r[0] < 4.0 || r[3] - r[1] < 4.0 {
            self.note = "Drag a box round the subject.".into();
            return;
        }
        let before = (self.work.cut.clone(), self.subject.clone());
        self.subject = Some(Subject {
            base: Arc::new(self.work.cut.clone()),
            rect: Some(r),
            strokes: Vec::new(),
            rev: self.work.rev,
        });
        self.run_cutout(before);
    }

    /// Cuts again with the box and the marks of the cut-out; `before` is
    /// the state for Undo.
    fn run_cutout(&mut self, before: (RgbaImage, Option<Subject>)) {
        let Some(s) = self.subject.as_mut() else { return };
        // Another change since the last cut: it is the new base.
        if s.rev != self.work.rev {
            s.base = Arc::new(self.work.cut.clone());
        }
        let base = s.base.clone();
        let (w, h) = base.dimensions();
        let mut marks = vec![Mark::None; (w * h) as usize];
        for st in &s.strokes {
            stamp_marks(&mut marks, w, h, st);
        }
        let Some(mask) = cutout::cutout(&base, s.rect, &marks) else {
            self.note = "The cut found no subject: drag a box round it, with some background in the box.".into();
            return;
        };
        let mut out = (*base).clone();
        let mut kept = 0;
        for (k, p) in out.pixels_mut().enumerate() {
            if mask[k] {
                kept += 1;
                if p[3] < 128 {
                    // A keep mark on a removed pixel brings it back.
                    let (x, y) = (k as u32 % w, k as u32 / w);
                    *p = *self.work.source.get_pixel(x, y);
                }
            } else {
                p[3] = 0;
            }
        }
        self.remember(before);
        self.work.cut = out;
        self.work.rev += 1;
        if let Some(s) = self.subject.as_mut() {
            s.rev = self.work.rev;
        }
        self.note = format!(
            "Subject cut out ({:.0}% of the picture). Mark parts to keep or remove if the edge is wrong.",
            100.0 * kept as f64 / f64::from(w * h)
        );
    }

    /// A mark stroke to the work pixel `(x, y)`.
    pub fn mark_to(&mut self, x: f64, y: f64) {
        let keep = self.tool == BgTool::MarkKeep;
        let radius = self.brush_radius_px();
        self.mark
            .get_or_insert_with(|| MarkStroke {
                keep,
                radius,
                points: Vec::new(),
            })
            .points
            .push((x, y));
    }

    /// Ends a mark stroke and cuts again with it.
    pub fn end_mark(&mut self) {
        let Some(stroke) = self.mark.take() else { return };
        let before = (self.work.cut.clone(), self.subject.clone());
        let rev = self.work.rev;
        let base = Arc::new(self.work.cut.clone());
        let s = self.subject.get_or_insert_with(|| Subject {
            base,
            rect: None,
            strokes: Vec::new(),
            rev,
        });
        s.strokes.push(stroke);
        self.run_cutout(before);
    }

    /// A point of a box or a lasso that is dragged (work pixels).
    pub fn shape_to(&mut self, x: f64, y: f64) {
        if self.tool == BgTool::Box && self.drag.len() == 2 {
            self.drag[1] = (x, y);
        } else {
            self.drag.push((x, y));
        }
    }

    /// Ends a box or a lasso: erases, keeps or restores as `shape_action`
    /// says; a box of the Subject tool cuts out the subject.
    pub fn end_shape(&mut self) {
        let pts = std::mem::take(&mut self.drag);
        if self.tool == BgTool::Subject {
            if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
                self.cut_out([a.0, a.1, b.0, b.1]);
            }
            return;
        }
        let polygon: Vec<(f64, f64)> = match (self.tool, pts.first(), pts.last()) {
            (BgTool::Box, Some(&a), Some(&b)) => vec![a, (b.0, a.1), b, (a.0, b.1)],
            _ => pts,
        };
        let before = self.work.cut.clone();
        let n = background::erase_shape(&mut self.work.cut, &self.work.source, &polygon, self.shape_action);
        self.note = if n == 0 {
            "Nothing changed (draw a larger shape).".into()
        } else {
            self.changed(before);
            format!("{n} pixels changed.")
        };
    }

    /// Background: removes the color at the work pixel `(x, y)`.
    pub fn remove_at(&mut self, x: u32, y: u32) {
        let before = self.work.cut.clone();
        let n = background::remove_at(&mut self.work.cut, x, y, self.tolerance, self.connected);
        self.note = if n == 0 {
            "Nothing removed there (clear already, or off the picture).".into()
        } else {
            self.changed(before);
            format!("{n} pixels removed.")
        };
    }

    /// Background: removes the most common color at the edges.
    pub fn remove_edges(&mut self) {
        let before = self.work.cut.clone();
        let n = background::remove_edges(&mut self.work.cut, self.tolerance);
        self.note = if n == 0 {
            "No background found at the edges.".into()
        } else {
            self.changed(before);
            format!("{n} pixels of the edge color removed.")
        };
    }

    /// The brush radius in work pixels.
    fn brush_radius_px(&self) -> f64 {
        (self.brush_mm / 2.0 / self.work.mm_per_px.max(1e-6)).max(0.5)
    }

    /// Background: a brush stroke to the work pixel `(x, y)` (the first
    /// point starts the stroke, one undo step for each stroke).
    pub fn brush_to(&mut self, x: f64, y: f64) {
        let restore = self.tool == BgTool::Restore;
        let r = self.brush_radius_px();
        let from = match self.stroke {
            Some(p) => p,
            None => {
                let before = self.work.cut.clone();
                self.changed(before);
                (x, y)
            }
        };
        // Stamps along the line from the last point, half a radius apart.
        let d = ((x - from.0).powi(2) + (y - from.1).powi(2)).sqrt();
        let n = (d / (r / 2.0)).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f64 / n as f64;
            let (px, py) = (from.0 + (x - from.0) * t, from.1 + (y - from.1) * t);
            background::brush(&mut self.work.cut, &self.work.source, px, py, r, restore);
        }
        self.stroke = Some((x, y));
        self.work.rev += 1;
    }

    /// Ends a brush stroke.
    pub fn end_stroke(&mut self) {
        if self.stroke.take().is_some() {
            self.note = "Brush stroke done (Undo takes it back).".into();
        }
    }

    pub fn undo(&mut self) {
        if let Some((img, subject)) = self.history.pop() {
            self.work.cut = img;
            self.work.rev += 1;
            self.subject = subject;
            // The cut-out goes on from the state that comes back.
            if let Some(s) = self.subject.as_mut() {
                s.rev = self.work.rev;
            }
            self.note = "Last change undone.".into();
        }
    }

    pub fn start_again(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.history.clear();
        self.subject = None;
        self.work.cut = (*self.work.source).clone();
        self.work.rev += 1;
        self.note = "Back to the full picture.".into();
    }

    /// Makes the texture of a preview picture.
    fn set_texture(&mut self, ctx: &egui::Context, key: u64, img: &RgbaImage, sharp: bool) -> TextureHandle {
        let color = ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
        let opts = if sharp {
            TextureOptions::NEAREST
        } else {
            TextureOptions::LINEAR
        };
        let t = crate::resources::load_texture(ctx, "picture-wizard", color, opts);
        self.texture = Some((key, t.clone()));
        t
    }

    /// The preview of the current step. The steps up to Background are
    /// quick and are made here; the later steps are made by a background
    /// job, and the last picture stays on the screen until it is done.
    fn texture(&mut self, ctx: &egui::Context, current: &CurrentProperties) -> TextureHandle {
        self.poll_job(ctx);
        let quick: Option<(u64, MakePicture)> = match self.step {
            Step::Picture => Some((
                hash(("source", Arc::as_ptr(&self.data) as usize)),
                Box::new(|w| (*w.source).clone()),
            )),
            // While a brush stroke goes on, the cut picture (no clean-up), so it is quick.
            Step::Background if self.stroke.is_some() => {
                Some((hash(("cut", self.work.rev)), Box::new(|w| w.cut.clone())))
            }
            Step::Background => Some((
                hash(("cleaned", self.work.content_key())),
                Box::new(|w| (*w.cleaned()).clone()),
            )),
            _ => None,
        };
        let key = match &quick {
            Some((k, _)) => *k,
            None => self.work.preview_key(self.step, current),
        };
        if let Some((k, t)) = &self.texture
            && *k == key
        {
            return t.clone();
        }
        if let Some((_, make)) = quick {
            let img = make(&mut self.work);
            return self.set_texture(ctx, key, &img, false);
        }
        // One job at a time: when it ends, a job for the newest settings starts.
        if self.job.is_none() && self.failed != Some(key) {
            self.start_job(ctx, key, current);
        }
        if self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
        match &self.texture {
            Some((_, t)) => t.clone(),
            None => {
                let img = self.work.cut.clone();
                self.set_texture(ctx, 0, &img, false)
            }
        }
    }

    /// Starts a background job for the preview of the current step.
    fn start_job(&mut self, ctx: &egui::Context, key: u64, current: &CurrentProperties) {
        let (tx, rx) = mpsc::channel();
        let mut work = self.work.clone();
        let (step, cur) = (self.step, current.clone());
        let job = move || {
            let (picture, sharp) = work.preview(step, &cur);
            let key = work.preview_key(step, &cur);
            let _ = tx.send(JobDone {
                work,
                key,
                picture,
                sharp,
            });
        };
        let thread = std::thread::Builder::new()
            .name("picture-wizard".into())
            .stack_size(crate::resources::JOB_STACK);
        match thread.spawn(job) {
            Ok(_) => self.job = Some(Job { key, rx }),
            Err(_) => {
                // No thread: make it here.
                let (img, sharp) = self.work.preview(self.step, current);
                let key = self.work.preview_key(self.step, current);
                let _ = self.set_texture(ctx, key, &img, sharp);
            }
        }
    }

    /// Takes the result of the job if it is done.
    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(got) = self.job.as_ref().map(|j| j.rx.try_recv()) else {
            return;
        };
        match got {
            Ok(done) => {
                self.job = None;
                self.work.adopt(done.work);
                let _ = self.set_texture(ctx, done.key, &done.picture, done.sharp);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.failed = self.job.take().map(|j| j.key);
                self.note = tr("The preview failed with these settings. Change a setting to try again.").into();
            }
        }
    }

    /// A background job makes a preview now.
    pub fn working(&self) -> bool {
        self.job.is_some()
    }

    /// Waits for the job and takes its results (Finish uses them).
    pub fn wait_for_job(&mut self) {
        if let Some(job) = self.job.take()
            && let Ok(done) = job.rx.recv()
        {
            self.work.adopt(done.work);
        }
    }
}

/// The result in `slot` if it has the key `key`.
fn ready<T>(slot: &Option<(u64, Arc<T>)>, key: u64) -> Option<Arc<T>> {
    slot.as_ref().filter(|(k, _)| *k == key).map(|(_, v)| v.clone())
}

impl Work {
    /// The cut picture with the edge shrunk and small bits cleaned up.
    fn cleaned(&mut self) -> Arc<RgbaImage> {
        let key = hash((self.rev, self.shrink_px, self.bits_mm2.to_bits()));
        if let Some((k, c)) = &self.cleaned
            && *k == key
        {
            return c.clone();
        }
        let mut img = self.cut.clone();
        background::shrink_edge(&mut img, self.shrink_px);
        let min_px = (self.bits_mm2 / (self.mm_per_px * self.mm_per_px)).round() as usize;
        background::clean(&mut img, &self.source, min_px);
        let c = Arc::new(img);
        self.cleaned = Some((key, c.clone()));
        c
    }

    fn content_key(&self) -> u64 {
        hash((self.rev, self.shrink_px, self.bits_mm2.to_bits(), self.trim))
    }

    /// The picture after the background step (trimmed if asked), and the
    /// work pixel of its top-left corner.
    fn content(&mut self) -> Arc<Content> {
        let key = self.content_key();
        if let Some((k, c)) = &self.content
            && *k == key
        {
            return c.clone();
        }
        let img = self.cleaned();
        let c = if self.trim {
            background::trim(&img).unwrap_or_else(|| ((*img).clone(), (0, 0)))
        } else {
            ((*img).clone(), (0, 0))
        };
        let c = Arc::new(c);
        self.content = Some((key, c.clone()));
        c
    }

    /// The width of the picture after the background step, as on the backdrop.
    pub fn natural_width_mm(&mut self) -> f64 {
        f64::from(self.content().0.width()) * self.mm_per_px
    }

    /// The design width: as set or as on the backdrop, with the long side
    /// at most `smart::MAX_SIDE_MM`.
    pub fn design_width_mm(&mut self) -> f64 {
        let w = match self.width_mm {
            Some(w) => w,
            None => self.natural_width_mm(),
        };
        let (cw, ch) = self.content().0.dimensions();
        smart::fit_width(w, cw, ch)
    }

    /// The size of the simplified picture in pixels.
    pub fn detail_px(&mut self) -> (u32, u32) {
        let c = self.content();
        let (w, h) = c.0.dimensions();
        smart::detail_px(w, h, self.design_width_mm(), self.pixel_mm)
    }

    fn reduce_key(&self) -> u64 {
        hash((
            self.content_key(),
            self.width_mm.map(f64::to_bits),
            self.pixel_mm.to_bits(),
            self.colors,
            self.speck_mm2.to_bits(),
            self.clean_edges,
            self.denoise,
            self.smooth,
            self.chart.as_ref().map(|c| c.0),
        ))
    }

    /// The simplified picture (resampled, smoothed and color-reduced). New
    /// choices and order when its colors change.
    pub fn reduced(&mut self) -> Arc<Reduced> {
        let key = self.reduce_key();
        if let Some((k, r)) = &self.reduced
            && *k == key
        {
            return r.clone();
        }
        let spec = SimplifySpec {
            pixel_mm: self.pixel_mm,
            colors: self.colors,
            speck_mm2: self.speck_mm2,
            clean_edges: self.clean_edges,
            denoise: self.denoise,
            smooth: self.smooth,
        };
        let width_mm = self.design_width_mm();
        let chart = self.chart.as_ref().map(|c| c.1.clone());
        let r = smart::simplify(&self.content().0, width_mm, &spec, chart.as_deref().map(Vec::as_slice));
        let same = self.reduced.as_ref().is_some_and(|(_, old)| old.palette == r.palette);
        if !same {
            self.choices = vec![ColorChoice::default(); r.palette.len()];
            self.order = (0..r.palette.len() as u16).collect();
        }
        let r = Arc::new(r);
        self.reduced = Some((key, r.clone()));
        r
    }

    /// The colors that sew, in sew order: kept and not merged into another
    /// (the choices are for the last simplified picture).
    pub fn final_order(&self) -> Vec<u16> {
        let to = smart::final_colors(&self.choices);
        self.order
            .iter()
            .copied()
            .filter(|&c| to.get(c as usize) == Some(&Some(c)))
            .collect()
    }

    /// From pixels of the picture after the background step to design mm.
    fn content_to_mm(&mut self) -> Transform {
        let c = self.content();
        let (cw, ch) = c.0.dimensions();
        let (ox, oy) = (f64::from(c.1.0), f64::from(c.1.1));
        let m = self.mm_per_px;
        let f = self.design_width_mm() / (f64::from(cw.max(1)) * m);
        let center = self.origin + Point::new(ox + f64::from(cw) / 2.0, oy + f64::from(ch) / 2.0) * m;
        Transform::translate(ox, oy)
            .then(Transform::scale(m, m))
            .then(Transform::translate(self.origin.x - center.x, self.origin.y - center.y))
            .then(Transform::scale(f, f))
            .then(Transform::translate(center.x, center.y))
            .then(self.rotation)
    }

    /// From pixels of the simplified picture to design mm.
    pub fn pixels_to_mm(&mut self) -> Transform {
        let r = self.reduced();
        let cw = self.content().0.width();
        let k = f64::from(cw) / f64::from(r.width.max(1));
        Transform::scale(k, k).then(self.content_to_mm())
    }

    fn objects_key(&self, current: &CurrentProperties) -> u64 {
        hash((
            self.reduce_key(),
            format!("{:?}", self.choices),
            self.order.clone(),
            format!("{:?}", self.spec),
            format!("{current:?}"),
        ))
    }

    /// The color-area objects (made again when a setting changes).
    pub fn objects(&mut self, current: &CurrentProperties) -> Arc<Vec<SmartObject>> {
        // New colors reset the choices first.
        let r = self.reduced();
        let key = self.objects_key(current);
        if let Some((k, o)) = &self.result
            && *k == key
        {
            return o.clone();
        }
        let order = self.final_order();
        let t = self.pixels_to_mm();
        let objs = Arc::new(smart::digitize(&r, &t, &self.choices, &order, &self.spec, current));
        self.result = Some((key, objs.clone()));
        objs
    }

    fn photo_key(&self) -> u64 {
        hash((
            self.content_key(),
            format!("{:?}", self.photo),
            self.width_mm.map(f64::to_bits),
            self.denoise,
        ))
    }

    /// The photo stitch objects (made again when a setting changes).
    pub fn photo_objects(&mut self) -> Arc<Vec<photo::PhotoObject>> {
        let key = self.photo_key();
        if let Some((k, o)) = &self.photo_result
            && *k == key
        {
            return o.clone();
        }
        let img = emb_art::adjust::median(&self.content().0, self.denoise);
        let t = self.content_to_mm();
        let objs = Arc::new(photo::photo(&img, &t, &self.photo));
        self.photo_result = Some((key, objs.clone()));
        objs
    }

    fn cross_key(&self) -> u64 {
        let g = &self.cross_grid;
        hash((
            self.content_key(),
            format!("{:?} {:?} {:?} {:?}", self.cross, g.origin, g.u, g.v),
            self.width_mm.map(f64::to_bits),
            self.denoise,
        ))
    }

    /// The cross-stitch objects (made again when a setting changes).
    pub fn cross_objects(&mut self) -> Arc<Vec<CrossColor>> {
        let key = self.cross_key();
        if let Some((k, o)) = &self.cross_result
            && *k == key
        {
            return o.clone();
        }
        let img = emb_art::adjust::median(&self.content().0, self.denoise);
        let t = self.content_to_mm();
        let objs = Arc::new(emb_art::cross::cross_stitch(&img, &t, &self.cross_grid, &self.cross));
        self.cross_result = Some((key, objs.clone()));
        objs
    }

    /// The objects that Finish adds, with their thread colors.
    pub fn result(&mut self, current: &CurrentProperties) -> Vec<(Rgb, ObjectKind)> {
        match self.make {
            Make::Areas => {
                let palette = self.reduced().palette.clone();
                self.objects(current)
                    .iter()
                    .map(|o| (palette[o.color as usize], o.kind.clone()))
                    .collect()
            }
            Make::Photo => self.photo_objects().iter().map(|o| (o.color, o.kind.clone())).collect(),
            Make::Cross => self
                .cross_objects()
                .iter()
                .map(|o| (o.color, ObjectKind::CrossStitch(Box::new(o.cross.clone()))))
                .collect(),
        }
    }

    /// The key of the preview picture of `step` (a step after Background):
    /// it changes when the picture changes. Quick: it makes nothing.
    fn preview_key(&self, step: Step, current: &CurrentProperties) -> u64 {
        match step {
            Step::Simplify => hash(("reduced", self.reduce_key())),
            Step::Colors | Step::Stitches => hash(("merged", self.reduce_key(), format!("{:?}", self.choices))),
            _ => match self.make {
                Make::Areas => hash(("result", self.objects_key(current))),
                Make::Photo => hash(("result", self.photo_key())),
                Make::Cross => hash(("result", self.cross_key())),
            },
        }
    }

    /// The preview picture of `step` (a step after Background), and whether
    /// to show its pixels sharp.
    fn preview(&mut self, step: Step, current: &CurrentProperties) -> (RgbaImage, bool) {
        match step {
            Step::Simplify => (self.reduced().to_image(), true),
            Step::Colors | Step::Stitches => {
                let r = self.reduced();
                (smart::merge_colors(&r, &self.choices).to_image(), true)
            }
            _ => {
                let objs = self.result(current);
                let (picture, stitches) = self.stitch_picture(&objs);
                if self.make == Make::Areas {
                    let key = self.objects_key(current);
                    let checks = self.checks_of(&objs, &stitches);
                    self.checks = Some((key, Arc::new(checks)));
                }
                (picture, false)
            }
        }
    }

    /// The simplified picture, if it is made for the settings as they are
    /// (else a job makes it).
    pub fn ready_reduced(&self) -> Option<Arc<Reduced>> {
        ready(&self.reduced, self.reduce_key())
    }

    /// The photo stitch objects, if they are made for the settings as they are.
    pub fn ready_photo(&self) -> Option<Arc<Vec<photo::PhotoObject>>> {
        ready(&self.photo_result, self.photo_key())
    }

    /// The cross-stitch objects, if they are made for the settings as they are.
    pub fn ready_cross(&self) -> Option<Arc<Vec<CrossColor>>> {
        ready(&self.cross_result, self.cross_key())
    }

    /// The number of objects and of colors that Finish adds, if they are
    /// made for the settings as they are.
    pub fn ready_count(&self, current: &CurrentProperties) -> Option<(usize, usize)> {
        fn count(mut c: Vec<Rgb>) -> usize {
            c.sort_by_key(|c| (c.0, c.1, c.2));
            c.dedup();
            c.len()
        }
        match self.make {
            Make::Areas => {
                let palette = &self.reduced.as_ref()?.1.palette;
                ready(&self.result, self.objects_key(current)).map(|o| {
                    let colors = o
                        .iter()
                        .filter_map(|o| palette.get(o.color as usize).copied())
                        .collect();
                    (o.len(), count(colors))
                })
            }
            Make::Photo => self
                .ready_photo()
                .map(|o| (o.len(), count(o.iter().map(|o| o.color).collect()))),
            Make::Cross => self
                .ready_cross()
                .map(|o| (o.len(), count(o.iter().map(|o| o.color).collect()))),
        }
    }

    /// Takes the results that a job made (`done` is the work state of the
    /// job). The settings stay as they are now: they can have changed while
    /// the job ran. New colors in the simplified picture reset the color
    /// choices and the order, as [`Work::reduced`] does.
    fn adopt(&mut self, done: Work) {
        fn take<T>(slot: &mut Option<(u64, T)>, new: Option<(u64, T)>) {
            if new.is_some() {
                *slot = new;
            }
        }
        let new_colors = done
            .reduced
            .as_ref()
            .is_some_and(|(_, r)| self.reduced.as_ref().is_none_or(|(_, o)| o.palette != r.palette));
        if new_colors {
            self.choices = done.choices;
            self.order = done.order;
        }
        take(&mut self.cleaned, done.cleaned);
        take(&mut self.content, done.content);
        take(&mut self.reduced, done.reduced);
        take(&mut self.result, done.result);
        take(&mut self.photo_result, done.photo_result);
        take(&mut self.cross_result, done.cross_result);
        take(&mut self.checks, done.checks);
    }

    /// The checks of the color-area result, if they are made for the
    /// settings as they are.
    pub fn ready_checks(&self, current: &CurrentProperties) -> Option<Arc<Vec<Check>>> {
        match self.make {
            Make::Areas => ready(&self.checks, self.objects_key(current)),
            Make::Photo | Make::Cross => None,
        }
    }

    /// Changes the settings as `fix` says.
    pub fn apply(&mut self, fix: Fix) {
        match fix {
            Fix::Specks(v) => self.speck_mm2 = v,
            Fix::Detail(v) => self.pixel_mm = v,
            Fix::Merge(a, b) => {
                if let Some(c) = self.choices.get_mut(a as usize) {
                    c.merge_into = Some(b);
                }
            }
            Fix::Overlap(v) => self.spec.overlap_mm = v,
            Fix::NoLayers => self.spec.layer_max_mm2 = 0.0,
            Fix::SatinMax(v) => {
                self.spec.satin_max_mm = v;
                for c in &mut self.choices {
                    if c.stitch == SmartStitch::Satin {
                        c.stitch = SmartStitch::Auto;
                    }
                }
            }
        }
    }

    /// The checks of the color-area result: build-up of thread, trims,
    /// very small objects, long stitches and colors that are almost the
    /// same, each with a fix where one exists.
    fn checks_of(&mut self, objs: &[(Rgb, ObjectKind)], s: &emb_model::StitchDesign) -> Vec<Check> {
        let mut out = Vec::new();
        if let Some(m) = emb_render::density::DensityMap::of(s) {
            let spots = m.hot_spots(self.density_limit);
            if !spots.is_empty() {
                let fix = if self.spec.overlap_mm > 0.1 {
                    Some(Fix::Overlap(0.1))
                } else if self.spec.layer_max_mm2 > 0.0 {
                    Some(Fix::NoLayers)
                } else {
                    None
                };
                out.push(Check {
                    text: crate::i18n::fill(
                        "Thread builds up in {} places (more than {} layers).",
                        &[&spots.len(), &format!("{:.0}", self.density_limit)],
                    ),
                    fix,
                });
            }
        }
        let trims = s.trim_count();
        if trims > MANY_TRIMS {
            let coarser = self.pixel_mm * 1.5;
            out.push(Check {
                text: crate::i18n::fill("{} trims: slow to sew. Less detail makes fewer areas.", &[&trims]),
                fix: (coarser <= 3.0).then_some(Fix::Detail(coarser)),
            });
        }
        let mut d = Design::new("");
        let t = d.add_thread("t", Rgb(0, 0, 0));
        let tiny = objs
            .iter()
            .filter(|(_, k)| {
                let id = d.add_object(t, k.clone());
                d.bbox_of(&[id])
                    .is_some_and(|b| b.width() < TINY_MM && b.height() < TINY_MM)
            })
            .count();
        if tiny > 0 {
            out.push(Check {
                text: crate::i18n::fill(
                    "{} objects smaller than {} mm: they sew as knots.",
                    &[&tiny, &format!("{TINY_MM:.0}")],
                ),
                fix: Some(Fix::Specks((self.speck_mm2 * 2.0).max(2.0))),
            });
        }
        let long = s
            .records
            .windows(2)
            .filter(|w| w[0].kind == StitchKind::Stitch && w[1].kind == StitchKind::Stitch)
            .filter(|w| w[0].pos.distance(w[1].pos) > LONG_MM)
            .count();
        if long > 0 {
            let satins = self.choices.iter().any(|c| c.stitch == SmartStitch::Satin);
            out.push(Check {
                text: crate::i18n::fill(
                    "{} stitches longer than {} mm: they can catch.",
                    &[&long, &format!("{LONG_MM:.0}")],
                ),
                fix: (satins || self.spec.satin_max_mm > 5.0).then_some(Fix::SatinMax(5.0)),
            });
        }
        let r = self.reduced();
        let counts = r.counts();
        let kept = self.final_order();
        for (i, &a) in kept.iter().enumerate() {
            for &b in &kept[i + 1..] {
                let (ca, cb) = (r.palette[a as usize], r.palette[b as usize]);
                if Lab::of(ca).distance(Lab::of(cb)) < NEAR_DE {
                    // The less common color goes into the more common one.
                    let (from, to) = if counts[a as usize] < counts[b as usize] {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    out.push(Check {
                        text: crate::i18n::fill(
                            "Two colors are almost the same ({} and {}).",
                            &[&ca.to_hex(), &cb.to_hex()],
                        ),
                        fix: Some(Fix::Merge(from, to)),
                    });
                }
            }
        }
        out
    }

    /// A picture of the stitches of `objs`.
    /// Also gives the stitches.
    fn stitch_picture(&mut self, objs: &[(Rgb, ObjectKind)]) -> (RgbaImage, emb_model::StitchDesign) {
        let mut d = Design::new("");
        for (color, kind) in objs {
            let t = d.thread_for_color(*color);
            d.add_object(t, kind.clone());
        }
        let plan = emb_stitch::build_plan(&d, &Default::default(), &mut Default::default());
        let segs = emb_render::segments(&plan.stitches);
        let width = self.design_width_mm().max(1.0);
        let px_per_mm = (RESULT_PX / width).clamp(2.0, 20.0);
        let p = emb_render::raster::picture(&segs, &[], px_per_mm, 0.35, 1.0, Some(Rgb(255, 255, 255)));
        let img = RgbaImage::from_raw(p.width, p.height, p.rgba).unwrap_or_else(|| RgbaImage::new(1, 1));
        (img, plan.stitches)
    }
}

impl AppState {
    /// Picture to design: asks for a picture if the design has no backdrop
    /// yet (it becomes the backdrop), then opens the wizard on it.
    pub fn picture_to_design(&mut self) {
        if self.doc.design().backdrop.is_none() {
            self.import_bitmap_dialog();
            if self.doc.design().backdrop.is_none() {
                return;
            }
        }
        self.open_wizard();
    }

    /// Opens the Picture to design wizard on the backdrop picture.
    pub fn open_wizard(&mut self) {
        match Wizard::new(self.doc.design()) {
            Ok(mut w) => {
                // Cross stitch goes on the grid of the cross-stitch tools.
                w.cross_grid = self.cross_grid();
                w.density_limit = self.settings.density_limit;
                self.wizard = Some(Box::new(w));
            }
            Err(e) => self.message = e,
        }
    }

    /// Adds the objects of the wizard to the design in one undo step, with
    /// threads for their colors (the nearest chart threads when the Threads
    /// panel has a chart).
    pub fn finish_wizard(&mut self, w: &mut Wizard) -> usize {
        w.wait_for_job();
        let current = self.doc.design().current.clone();
        let objs = w.result(&current);
        if objs.is_empty() {
            self.message = tr("Nothing to add: no color is kept, or the picture is all clear.").into();
            return 0;
        }
        let chart: Option<ThreadChart> = self.charts.get(self.chart_index).cloned();
        let (group, hide) = (w.group, w.hide_backdrop);
        let ids: Vec<ObjectId> = self.doc.edit("Picture to Design", |d| {
            let mut threads = std::collections::HashMap::new();
            let mut ids = Vec::new();
            for (color, kind) in objs {
                let t = *threads.entry(color).or_insert_with(|| {
                    let look = chart.as_ref().and_then(|c| c.nearest(color).map(|k| chart_look(c, k)));
                    match look {
                        Some(look) => match d.threads.iter().find(|t| t.color == look.color) {
                            Some(t) => t.id,
                            None => {
                                let id = d.add_thread(look.name.clone(), look.color);
                                if let Some(t) = d.threads.iter_mut().find(|t| t.id == id) {
                                    t.brand = look.brand;
                                    t.code = look.code;
                                }
                                id
                            }
                        },
                        None => crate::vectors::nearest_thread(d, color),
                    }
                });
                ids.push(d.add_object(t, kind));
            }
            if group && ids.len() > 1 {
                let g = d.new_group_id();
                for id in &ids {
                    if let Some(o) = d.object_mut(*id) {
                        o.group = Some(g);
                    }
                }
            }
            if hide && let Some(b) = d.backdrop.as_mut() {
                b.visible = false;
            }
            ids
        });
        self.show_vectors = true;
        let n_ids = ids.len();
        self.message = crate::i18n::fill(
            "Picture to design: {} objects added{}{}. Undo takes them back.",
            &[
                &(ids.len()),
                &(if group && n_ids > 1 {
                    " as a group (Break apart or Ungroup to edit them one by one)"
                } else {
                    ""
                }),
                &(if hide {
                    "; the backdrop is hidden (Backdrop panel: Show)"
                } else {
                    ""
                }),
            ],
        );
        let n = ids.len();
        self.selection.set_all(ids);
        n
    }
}

/// Sets the marks under a mark stroke (round stamps along it).
fn stamp_marks(marks: &mut [Mark], w: u32, h: u32, st: &MarkStroke) {
    let m = if st.keep { Mark::Keep } else { Mark::Remove };
    let r = st.radius.max(0.5);
    let mut stamp = |x: f64, y: f64| {
        let (x0, y0) = ((x - r).floor().max(0.0) as u32, (y - r).floor().max(0.0) as u32);
        let (x1, y1) = (
            ((x + r).ceil().max(0.0) as u32).min(w),
            ((y + r).ceil().max(0.0) as u32).min(h),
        );
        for py in y0..y1 {
            for px in x0..x1 {
                let (dx, dy) = (f64::from(px) + 0.5 - x, f64::from(py) + 0.5 - y);
                if dx * dx + dy * dy <= r * r {
                    marks[(py * w + px) as usize] = m;
                }
            }
        }
    };
    let pts = &st.points;
    if let Some(&(x, y)) = pts.first() {
        stamp(x, y);
    }
    for seg in pts.windows(2) {
        let ((ax, ay), (bx, by)) = (seg[0], seg[1]);
        let n = ((bx - ax).hypot(by - ay) / (r / 2.0)).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f64 / n as f64;
            stamp(ax + (bx - ax) * t, ay + (by - ay) * t);
        }
    }
}

/// A light and dark checker under a picture with clear parts (only the
/// part of `rect` in `clip` is drawn).
fn checker(painter: &egui::Painter, rect: Rect, clip: Rect) {
    let s = 10.0;
    let shown = rect.intersect(clip);
    if !shown.is_positive() {
        return;
    }
    painter.rect_filled(shown, 0.0, Color32::from_gray(235));
    let (i0, j0) = (
        ((shown.min.x - rect.min.x) / s).floor() as i32,
        ((shown.min.y - rect.min.y) / s).floor() as i32,
    );
    let (i1, j1) = (
        ((shown.max.x - rect.min.x) / s).ceil() as i32,
        ((shown.max.y - rect.min.y) / s).ceil() as i32,
    );
    for j in j0..j1 {
        for i in i0..i1 {
            if (i + j) % 2 == 1 {
                let r = Rect::from_min_size(rect.min + vec2(i as f32 * s, j as f32 * s), vec2(s, s)).intersect(shown);
                painter.rect_filled(r, 0.0, Color32::from_gray(200));
            }
        }
    }
}

fn swatch(ui: &mut egui::Ui, c: Rgb) {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color32(c));
    ui.painter().rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
}

/// What the buttons of the wizard did.
enum Done {
    No,
    Cancel,
    Finish,
}

/// Shows the wizard window while it is open. An error in the wizard
/// closes it with a message; the program goes on (ADR 0077).
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if st.wizard.is_none() {
        return;
    }
    guarded(st, |st| show_window(ctx, st));
}

/// Runs `f`; if it panics, the wizard closes with a message.
fn guarded(st: &mut AppState, f: impl FnOnce(&mut AppState)) {
    let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut *st)));
    if shown.is_err() {
        st.wizard = None;
        st.message =
            tr("The Picture to Design wizard stopped because of an error. Try less detail or a smaller picture.")
                .into();
    }
}

fn show_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut w) = st.wizard.take() else { return };
    // Another picture was loaded: start again with it.
    let data = st.doc.design().backdrop.as_ref().map(|b| b.data.clone());
    if data.is_some_and(|d| !Arc::ptr_eq(&d, &w.data)) {
        match Wizard::new(st.doc.design()) {
            Ok(n) => w = Box::new(n),
            Err(e) => {
                st.message = e;
                return;
            }
        }
    }
    let current = st.doc.design().current.clone();
    let mut open = true;
    let mut done = Done::No;
    egui::Window::new(tr("Picture to Design"))
        .id(egui::Id::new("Picture to Design"))
        .open(&mut open)
        .default_size([1000.0, 660.0])
        .min_size([860.0, 560.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            // The steps.
            ui.horizontal(|ui| {
                let steps = w.steps();
                for (i, &s) in steps.iter().enumerate() {
                    let label = format!("{} {}", i + 1, s.name());
                    if ui.selectable_label(w.step == s, label).clicked() {
                        w.step = s;
                    }
                    if i + 1 < steps.len() {
                        ui.weak("›");
                    }
                }
            });
            ui.separator();
            let avail = ui.available_size();
            let bottom = 36.0;
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(290.0, (avail.y - bottom).max(200.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.set_width(280.0);
                            controls(ui, st, &mut w, &current);
                        });
                    },
                );
                ui.separator();
                let size = vec2(ui.available_width(), (avail.y - bottom).max(200.0));
                preview(ui, &mut w, size, &current);
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("Cancel")).clicked() {
                    done = Done::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (i, steps) = (w.index(), w.steps());
                    if w.step == Step::Finish {
                        if ui.add_enabled(!w.finish, egui::Button::new(tr("Finish"))).clicked() {
                            w.finish = true;
                        }
                    } else if ui.button(tr("Next ›")).clicked() {
                        w.step = steps[(i + 1).min(steps.len() - 1)];
                    }
                    if i > 0 && ui.button(tr("‹ Back")).clicked() {
                        w.step = steps[i - 1];
                    }
                    ui.weak(&w.note);
                });
            });
        });
    // Finish waits for the job in the background: the window stays usable.
    if w.finish && w.step == Step::Finish {
        if w.working() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        } else {
            w.finish = false;
            done = Done::Finish;
        }
    } else {
        w.finish = false;
    }
    match done {
        Done::Cancel => return,
        Done::Finish => {
            if st.finish_wizard(&mut w) > 0 {
                return;
            }
        }
        Done::No => {}
    }
    if open {
        st.wizard = Some(w);
    }
}

/// A slider for a design value, as the other windows have them.
fn slider<'a>(v: &'a mut f64, range: std::ops::RangeInclusive<f64>, text: &str) -> egui::Slider<'a> {
    egui::Slider::new(v, range)
        .clamping(egui::SliderClamping::Edits)
        .text(text)
}

/// The design width (shared by the color-area and photo steps).
fn width_row(ui: &mut egui::Ui, w: &mut Wizard) {
    let natural = w.natural_width_mm();
    let mut width = w.design_width_mm();
    ui.horizontal(|ui| {
        ui.label(tr("Design width"));
        if ui
            .add(
                egui::DragValue::new(&mut width)
                    .range(5.0..=1000.0)
                    .clamp_existing_to_range(false)
                    .speed(0.5)
                    .fixed_decimals(1)
                    .suffix(" mm"),
            )
            .changed()
        {
            w.width_mm = Some(width);
        }
    });
    let (cw, ch) = w.content().0.dimensions();
    ui.weak(crate::i18n::fill(
        "Height {} mm. As on the backdrop: {} mm.",
        &[
            &format!("{:.1}", width * f64::from(ch) / f64::from(cw.max(1))),
            &format!("{:.1}", natural),
        ],
    ));
}

/// The settings of the current step.
fn controls(ui: &mut egui::Ui, st: &mut AppState, w: &mut Wizard, current: &CurrentProperties) {
    match w.step {
        Step::Picture => {
            ui.label(tr(
                "This wizard turns the picture into embroidery, step by step. Each step shows its result on the right.",
            ));
            ui.add_space(6.0);
            let (pw, ph) = w.source.dimensions();
            let width = w.mm_per_px * f64::from(pw);
            ui.label(crate::i18n::fill(
                "Picture: {} × {} px, {} mm wide.",
                &[&pw, &ph, &format!("{:.0}", width)],
            ));
            if ui.button(tr("Choose another picture…")).clicked() {
                st.import_bitmap_dialog();
            }
            ui.add_space(8.0);
            ui.strong(tr("Make"));
            ui.radio_value(&mut w.make, Make::Areas, tr("Color areas (a logo, a drawing)"))
                .on_hover_text(tr("Flat areas of a few colors: fills, satins and outlines."));
            ui.radio_value(&mut w.make, Make::Photo, tr("Photo stitch (a portrait, a photo)"))
                .on_hover_text(tr(
                    "Rows or scribbles whose density follows the light and dark of the picture.",
                ));
            ui.radio_value(&mut w.make, Make::Cross, tr("Cross stitch (a chart of crosses)"))
                .on_hover_text(tr(
                    "A cross in each cell of a grid, in a few thread colors. Print the chart with Specialty > Chart.",
                ));
            ui.add_space(6.0);
            ui.weak(tr("Then remove the background. Crop the picture first (Artwork > Picture) if only a part of it is wanted."));
        }
        Step::Background => {
            ui.strong(tr("Smart"));
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut w.tool, BgTool::Subject, tr("Cut out subject"))
                    .on_hover_text(tr("Drag a box round the subject: the background goes."));
                ui.selectable_value(&mut w.tool, BgTool::MarkKeep, tr("Mark keep"))
                    .on_hover_text(tr("Paint over parts of the subject that went: they come back."));
                ui.selectable_value(&mut w.tool, BgTool::MarkRemove, tr("Mark remove"))
                    .on_hover_text(tr(
                        "Paint over background that stayed: it goes, and so do colors like it next to it.",
                    ));
            });
            ui.strong(tr("By Color"));
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut w.tool, BgTool::Click, tr("Click a color"));
            });
            ui.strong(tr("By hand"));
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut w.tool, BgTool::Erase, tr("Erase brush"));
                ui.selectable_value(&mut w.tool, BgTool::Restore, tr("Restore brush"));
                ui.selectable_value(&mut w.tool, BgTool::Box, tr("Box"));
                ui.selectable_value(&mut w.tool, BgTool::Lasso, tr("Lasso"));
            });
            ui.separator();
            match w.tool {
                BgTool::Subject => {
                    ui.label(tr(
                        "Drag a box round the subject, with a little background all round it.",
                    ));
                    ui.weak(tr(
                        "Then use Mark keep and Mark remove where the edge is wrong: each mark cuts again.",
                    ));
                }
                BgTool::MarkKeep | BgTool::MarkRemove => {
                    ui.label(if w.tool == BgTool::MarkKeep {
                        "Paint a line over parts of the subject that went. Keep marks stay."
                    } else {
                        "Paint a line over background that stayed. Remove marks go."
                    });
                    ui.add(slider(&mut w.brush_mm, 0.5..=40.0, tr("Brush (mm)")).logarithmic(true));
                }
                BgTool::Box | BgTool::Lasso => {
                    ui.label(if w.tool == BgTool::Box {
                        "Drag a box over the picture."
                    } else {
                        "Drag round a part of the picture."
                    });
                    ui.radio_value(&mut w.shape_action, ShapeAction::EraseInside, tr("Erase inside"));
                    ui.radio_value(&mut w.shape_action, ShapeAction::EraseOutside, tr("Keep only inside"));
                    ui.radio_value(&mut w.shape_action, ShapeAction::RestoreInside, tr("Restore inside"));
                }
                BgTool::Click => {
                    ui.label(tr("Click the picture to remove a color (it shows as a checker)."));
                    ui.radio_value(&mut w.connected, true, tr("Only the area I click"));
                    ui.radio_value(&mut w.connected, false, tr("This color everywhere"));
                    ui.add(slider(&mut w.tolerance, 1.0..=60.0, tr("Tolerance")))
                        .on_hover_text(tr(
                            "How different a color can be and still go. More: also the shades of the color.",
                        ));
                }
                BgTool::Erase | BgTool::Restore => {
                    ui.label(if w.tool == BgTool::Erase {
                        "Paint over the parts to remove (hair edges, noise round a person)."
                    } else {
                        "Paint over the parts to bring back from the picture."
                    });
                    ui.add(slider(&mut w.brush_mm, 0.5..=40.0, tr("Brush (mm)")).logarithmic(true));
                }
            }
            if ui
                .button(tr("Remove the edge color"))
                .on_hover_text(tr(
                    "Removes the most common color at the edges of the picture, where it touches the edges. \
                     A wall that shades slowly goes too.",
                ))
                .clicked()
            {
                w.remove_edges();
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!w.history.is_empty(), egui::Button::new(tr("Undo")))
                    .clicked()
                {
                    w.undo();
                }
                if ui
                    .add_enabled(!w.history.is_empty(), egui::Button::new(tr("Start again")))
                    .clicked()
                {
                    w.start_again();
                }
            });
            ui.separator();
            ui.strong(tr("Clean up"));
            ui.horizontal(|ui| {
                ui.label(tr("Shrink the edge by"));
                ui.add(
                    egui::DragValue::new(&mut w.shrink_px)
                        .range(0..=10)
                        .clamp_existing_to_range(false)
                        .suffix(" px"),
                )
                .on_hover_text(tr(
                    "Clears a band at the edge of the cut-out: the mixed colors of the old background.",
                ));
            });
            ui.horizontal(|ui| {
                ui.label(tr("Remove bits under"));
                ui.add(
                    egui::DragValue::new(&mut w.bits_mm2)
                        .range(0.0..=500.0)
                        .clamp_existing_to_range(false)
                        .speed(0.2)
                        .fixed_decimals(1)
                        .suffix(" mm²"),
                )
                .on_hover_text(tr(
                    "Removes small bits of background that stay, and fills small holes in the subject.",
                ));
            });
            ui.checkbox(&mut w.trim, tr("Cut off clear edges"));
            ui.separator();
            ui.checkbox(&mut w.show_removed, tr("Show removed parts faintly"));
            ui.horizontal(|ui| {
                if ui.button(tr("Fit")).clicked() {
                    w.zoom = 1.0;
                    w.pan = Vec2::ZERO;
                }
                ui.weak(tr("Scroll: zoom. Right or middle drag: pan. Ctrl+Z: undo."));
            });
        }
        Step::Simplify => {
            width_row(ui, w);
            ui.add(
                slider(&mut w.pixel_mm, 0.1..=3.0, tr("Detail (mm a pixel)"))
                    .logarithmic(true)
                    .fixed_decimals(2),
            )
            .on_hover_text(tr("Smaller: more detail. Larger: simpler shapes, fewer small areas."));
            let (px, py) = w.detail_px();
            ui.weak(crate::i18n::fill(
                "The picture is {} × {} pixels at this detail.",
                &[&px, &py],
            ));
            ui.horizontal(|ui| {
                ui.label(tr("Colors"));
                ui.add(
                    egui::DragValue::new(&mut w.colors)
                        .range(2..=16)
                        .clamp_existing_to_range(false),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr("Remove specks under"));
                ui.add(
                    egui::DragValue::new(&mut w.speck_mm2)
                        .range(0.0..=100.0)
                        .clamp_existing_to_range(false)
                        .speed(0.1)
                        .fixed_decimals(1)
                        .suffix(" mm²"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr("Smooth noise"));
                ui.add(
                    egui::DragValue::new(&mut w.denoise)
                        .range(0..=3)
                        .clamp_existing_to_range(false),
                )
                .on_hover_text(tr(
                    "A median of the pixels round each pixel: removes grain and keeps edges. 0: off.",
                ));
            });
            ui.horizontal(|ui| {
                ui.label(tr("Even out"));
                ui.add(
                    egui::DragValue::new(&mut w.smooth)
                        .range(0..=3)
                        .clamp_existing_to_range(false),
                )
                .on_hover_text(tr(
                    "Edge-preserving smoothing before the colors are found: flat areas get even (JPEG blocks, the grain of a scan) and the edges stay sharp. 0: off.",
                ));
            });
            ui.checkbox(&mut w.clean_edges, tr("Clean edges"));
            let chart = st.charts.get(st.chart_index);
            let mut use_chart = w.chart.is_some();
            let label = match chart {
                Some(c) => crate::i18n::fill("Colors from the thread chart ({})", &[&c.name]),
                None => tr("Colors from the thread chart").into(),
            };
            if ui
                .add_enabled(chart.is_some(), egui::Checkbox::new(&mut use_chart, label))
                .on_hover_text(tr(
                    "The colors are threads of the chart of the Threads panel. Two colors that go to one thread are one color.",
                ))
                .changed()
            {
                w.chart = chart.filter(|_| use_chart).map(|c| {
                    let colors: Vec<Rgb> = c.threads.iter().map(|t| t.color).collect();
                    (hash(&colors), Arc::new(colors))
                });
            }
        }
        Step::Colors => {
            ui.label(tr(
                "Untick a color to leave it out (no stitches), or merge it into another color.",
            ));
            let Some(r) = w.ready_reduced() else {
                ui.weak(tr("Working…"));
                return;
            };
            let counts = r.counts();
            let total = counts.iter().sum::<usize>().max(1);
            let chart = st.charts.get(st.chart_index);
            let matched = chart.map(|c| match_threads(&r.palette, c));
            let n = r.palette.len().min(w.choices.len());
            // The name of each color: the nearest chart thread, or its hex code.
            let names: Vec<String> = (0..n)
                .map(|i| {
                    matched
                        .as_ref()
                        .and_then(|m| m[i])
                        .zip(chart)
                        .map_or(r.palette[i].to_hex(), |(k, ch)| chart_look(ch, k).name)
                })
                .collect();
            for i in 0..n {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut w.choices[i].keep, "");
                    swatch(ui, r.palette[i]);
                    ui.label(format!("{:.0}%", 100.0 * counts[i] as f64 / total as f64));
                    ui.label(&names[i]);
                });
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(tr("Merge into"));
                    let sel = w.choices[i].merge_into;
                    egui::ComboBox::from_id_salt(("wizard_merge", i))
                        .selected_text(sel.map_or("—".to_string(), |t| names[t as usize].clone()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut w.choices[i].merge_into, None, "—");
                            for t in (0..n as u16).filter(|t| *t as usize != i) {
                                ui.horizontal(|ui| {
                                    swatch(ui, r.palette[t as usize]);
                                    ui.selectable_value(&mut w.choices[i].merge_into, Some(t), &names[t as usize]);
                                });
                            }
                        });
                });
            }
        }
        Step::Stitches => {
            ui.label(tr(
                "The stitch of each color, in sew order (the first sews first, under the others).",
            ));
            let Some(r) = w.ready_reduced() else {
                ui.weak(tr("Working…"));
                return;
            };
            let order = w.final_order();
            for (k, &c) in order.iter().enumerate() {
                ui.horizontal(|ui| {
                    swatch(ui, r.palette[c as usize]);
                    let choice = &mut w.choices[c as usize];
                    egui::ComboBox::from_id_salt(("wizard_stitch", c))
                        .selected_text(tr(choice.stitch.name()))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for s in SmartStitch::ALL {
                                ui.selectable_value(&mut choice.stitch, s, s.name());
                            }
                        });
                    let up = egui::Button::new(egui_phosphor::regular::ARROW_UP).small();
                    if ui.add_enabled(k > 0, up).on_hover_text(tr("Sew earlier")).clicked() {
                        move_color(&mut w.order, c, order[k - 1]);
                    }
                    let down = egui::Button::new(egui_phosphor::regular::ARROW_DOWN).small();
                    if ui
                        .add_enabled(k + 1 < order.len(), down)
                        .on_hover_text(tr("Sew later"))
                        .clicked()
                    {
                        move_color(&mut w.order, c, order[k + 1]);
                    }
                });
            }
            ui.separator();
            let s = &mut w.spec;
            ui.add(slider(&mut s.overlap_mm, 0.0..=1.5, tr("Overlap (mm)")).fixed_decimals(2))
                .on_hover_text(tr(
                    "Fills grow by half of this, so colors that touch overlap and no fabric shows between them.",
                ));
            ui.add(slider(&mut s.satin_max_mm, 1.0..=10.0, tr("Auto satin up to (mm)")).fixed_decimals(1))
                .on_hover_text(tr(
                    "Auto makes a satin of a long area narrower than this; other areas get a fill.",
                ));
            ui.add(slider(&mut s.layer_max_mm2, 0.0..=200.0, tr("Details on top up to (mm²)")).fixed_decimals(0))
                .on_hover_text(tr(
                    "A detail inside an area up to this size sews on top of the area, and the area has no hole there. 0: each detail is a hole.",
                ));
            ui.add(slider(&mut s.small_mm2, 0.0..=20.0, tr("Small areas under (mm²)")).fixed_decimals(1))
                .on_hover_text(tr(
                    "Auto makes an area smaller than this a satin, or a run if it is very narrow; a fill this small has no underlay.",
                ));
            ui.checkbox(&mut s.fill_angles, tr("Fill along each area"))
                .on_hover_text(tr(
                    "Each fill sews along the long side of its area, and fills that touch get angles 30° or more apart. Off: the fill angle of the current settings.",
                ));
            ui.add(slider(&mut s.trace.tolerance_px, 0.2..=3.0, tr("Smoothing (px)")));
            ui.add(slider(&mut s.trace.corner_deg, 20.0..=150.0, tr("Corners (°)")));
        }
        Step::Photo => {
            width_row(ui, w);
            let p = &mut w.photo;
            egui::ComboBox::from_label(tr("Style"))
                .selected_text(tr(p.style.name()))
                .show_ui(ui, |ui| {
                    for s in PhotoStyle::ALL {
                        ui.selectable_value(&mut p.style, s, s.name());
                    }
                });
            let spacing = if p.style == PhotoStyle::Scribble {
                tr("Point spacing (mm)")
            } else {
                tr("Row spacing (mm)")
            };
            ui.add(slider(&mut p.row_mm, 0.5..=4.0, spacing).fixed_decimals(2))
                .on_hover_text(tr("Smaller: more detail and more stitches."));
            if p.style != PhotoStyle::Scribble {
                ui.add(slider(&mut p.angle_deg, -90.0..=90.0, tr("Row angle (°)")).fixed_decimals(0));
            }
            ui.add(slider(&mut p.contrast, 0.5..=3.0, tr("Contrast")).fixed_decimals(2));
            ui.add(slider(&mut p.brightness, -0.5..=0.5, tr("Brightness")).fixed_decimals(2));
            match p.style {
                PhotoStyle::ColorRows => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Thread colors"));
                        ui.add(
                            egui::DragValue::new(&mut p.colors)
                                .range(2..=8)
                                .clamp_existing_to_range(false),
                        );
                    });
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Thread"));
                        let mut c = [p.thread.0, p.thread.1, p.thread.2];
                        if ui.color_edit_button_srgb(&mut c).changed() {
                            p.thread = Rgb(c[0], c[1], c[2]);
                        }
                    });
                }
            }
            ui.horizontal(|ui| {
                ui.label(tr("Smooth noise"));
                ui.add(
                    egui::DragValue::new(&mut w.denoise)
                        .range(0..=3)
                        .clamp_existing_to_range(false),
                );
            });
            let Some(objs) = w.ready_photo() else {
                ui.weak(tr("Working…"));
                return;
            };
            let n: usize = objs
                .iter()
                .map(|o| match &o.kind {
                    ObjectKind::Stitches { path, .. } => path.points().count(),
                    _ => 0,
                })
                .sum();
            let k = objs.len();
            ui.weak(crate::i18n::fill(
                "About {} stitches in {} color{}.",
                &[&n, &k, &(if k == 1 { "" } else { "s" })],
            ));
        }
        Step::Cross => {
            width_row(ui, w);
            let mut mm = w.cross_grid.cell_mm();
            ui.horizontal(|ui| {
                ui.label(tr("Cell size"));
                let changed = ui
                    .add(
                        egui::DragValue::new(&mut mm)
                            .range(emb_model::cross::MIN_CELL_MM..=emb_model::cross::MAX_CELL_MM)
                            .speed(0.05)
                            .max_decimals(2)
                            .clamp_existing_to_range(false)
                            .suffix(" mm"),
                    )
                    .changed();
                if (changed | crate::cross::cell_menu(ui, &mut mm)) && (mm - w.cross_grid.cell_mm()).abs() > 1e-9 {
                    w.cross_grid.set_cell_mm(mm);
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Thread colors"));
                ui.add(
                    egui::DragValue::new(&mut w.cross.colors)
                        .range(2..=32)
                        .clamp_existing_to_range(false),
                );
            });
            ui.checkbox(&mut w.cross.partial, tr("Three-quarter crosses at the edges"))
                .on_hover_text(tr(
                    "Cells at a slanted edge get a three-quarter cross, so the edge is smoother.",
                ));
            ui.horizontal(|ui| {
                ui.label(tr("Smooth noise"));
                ui.add(
                    egui::DragValue::new(&mut w.denoise)
                        .range(0..=3)
                        .clamp_existing_to_range(false),
                );
            });
            let Some(objs) = w.ready_cross() else {
                ui.weak(tr("Working…"));
                return;
            };
            let cells: usize = objs.iter().map(|o| o.cross.cells.len()).sum();
            let (cw, ch) = objs
                .iter()
                .filter_map(|o| o.cross.bbox())
                .reduce(emb_geom::BBox::union)
                .map_or((0.0, 0.0), |b| (b.width() / mm, b.height() / mm));
            ui.weak(crate::i18n::fill(
                "{} × {} cells: {} crosses in {} color{}.",
                &[
                    &format!("{:.0}", cw),
                    &format!("{:.0}", ch),
                    &cells,
                    &(objs.len()),
                    &(if objs.len() == 1 { "" } else { "s" }),
                ],
            ));
        }
        Step::Finish => {
            match w.ready_count(current) {
                Some((objects, colors)) => {
                    ui.label(crate::i18n::fill("{} objects in {} colors.", &[&objects, &colors]));
                }
                None => {
                    ui.weak(tr("Working…"));
                }
            }
            ui.weak(tr("The picture shows the stitches. Go back to change a step."));
            if let Some(checks) = w.ready_checks(current) {
                ui.add_space(6.0);
                if checks.is_empty() {
                    ui.label(tr("The checks found no problem."));
                } else {
                    ui.strong(tr("Checks"));
                }
                let mut fix = None;
                for (i, c) in checks.iter().enumerate() {
                    ui.label(format!("{} {}", egui_phosphor::regular::WARNING, c.text));
                    if let Some(f) = c.fix
                        && ui
                            .push_id(("wizard_fix", i), |ui| ui.small_button(f.label()))
                            .inner
                            .clicked()
                    {
                        fix = Some(f);
                    }
                }
                if let Some(f) = fix {
                    w.apply(f);
                    w.note = tr("Settings changed: the stitches are made again.").into();
                }
                ui.add_space(6.0);
            }
            ui.checkbox(&mut w.group, tr("Group the objects"));
            ui.checkbox(&mut w.hide_backdrop, tr("Hide the backdrop picture"));
        }
    }
}

/// Moves color `c` to the place of color `other` in `order` (they swap).
fn move_color(order: &mut [u16], c: u16, other: u16) {
    let (Some(i), Some(j)) = (
        order.iter().position(|&x| x == c),
        order.iter().position(|&x| x == other),
    ) else {
        return;
    };
    order.swap(i, j);
}

/// The picture of the current step, fitted in `size`, with zoom and pan.
/// In the Background step a press on the picture uses the tool.
fn preview(ui: &mut egui::Ui, w: &mut Wizard, size: egui::Vec2, current: &CurrentProperties) {
    let tex = w.texture(ui.ctx(), current);
    let [tw, th] = tex.size();
    let img = vec2(tw as f32, th as f32);
    let (outer, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    if w.view_step != w.step {
        (w.zoom, w.pan, w.view_step) = (1.0, Vec2::ZERO, w.step);
    }
    // Scroll zooms about the pointer; a right or middle drag pans.
    if let Some(p) = resp.hover_pos() {
        let s = ui.input(|i| i.smooth_scroll_delta.y);
        if s != 0.0 {
            let old = w.zoom;
            w.zoom = (w.zoom * (s * 0.003).exp()).clamp(1.0, 40.0);
            let c = outer.center() + w.pan;
            w.pan += (p - c) * (1.0 - w.zoom / old);
        }
    }
    if resp.dragged_by(PointerButton::Secondary) || resp.dragged_by(PointerButton::Middle) {
        w.pan += resp.drag_delta();
    }
    let fit = (outer.width() / img.x).min(outer.height() / img.y).min(8.0);
    let rect = Rect::from_center_size(outer.center() + w.pan, img * fit * w.zoom);
    let painter = ui.painter_at(outer);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    checker(&painter, rect, outer);
    if w.step == Step::Background && w.show_removed {
        let s = w.work.source.clone();
        let src = w
            .source_tex
            .get_or_insert_with(|| {
                let c = ColorImage::from_rgba_unmultiplied([s.width() as usize, s.height() as usize], s.as_raw());
                crate::resources::load_texture(ui.ctx(), "picture-wizard-source", c, TextureOptions::LINEAR)
            })
            .clone();
        painter.image(src.id(), rect, uv, Color32::from_white_alpha(70));
    }
    painter.image(tex.id(), rect, uv, Color32::WHITE);
    if w.working() {
        let at = outer.left_top() + vec2(8.0, 8.0);
        let text = painter.layout_no_wrap(tr("Working…").into(), egui::FontId::proportional(14.0), Color32::WHITE);
        let back = Rect::from_min_size(at, text.size()).expand(4.0);
        painter.rect_filled(back, 4.0, Color32::from_black_alpha(160));
        painter.galley(at, text, Color32::WHITE);
    }
    if w.step != Step::Background {
        return;
    }
    let (cw, ch) = w.cut.dimensions();
    let to_px = |p: egui::Pos2| {
        (
            f64::from((p.x - rect.min.x) / rect.width()) * f64::from(cw),
            f64::from((p.y - rect.min.y) / rect.height()) * f64::from(ch),
        )
    };
    let to_screen = |(x, y): (f64, f64)| {
        pos2(
            rect.min.x + (x / f64::from(cw)) as f32 * rect.width(),
            rect.min.y + (y / f64::from(ch)) as f32 * rect.height(),
        )
    };
    let px_to_screen = rect.width() / cw as f32;
    // The marks of the cut-out.
    let strokes = w.subject.iter().flat_map(|s| s.strokes.iter()).chain(w.mark.iter());
    for st in strokes {
        let color = if st.keep {
            Color32::from_rgba_unmultiplied(40, 200, 60, 140)
        } else {
            Color32::from_rgba_unmultiplied(230, 40, 40, 140)
        };
        let pts: Vec<egui::Pos2> = st.points.iter().map(|p| to_screen(*p)).collect();
        let width = (2.0 * st.radius) as f32 * px_to_screen;
        if pts.len() == 1 {
            painter.circle_filled(pts[0], width / 2.0, color);
        } else {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(width, color)));
        }
    }
    // The box of the cut-out.
    if let Some(r) = w.subject.as_ref().and_then(|s| s.rect) {
        let b = Rect::from_two_pos(to_screen((r[0], r[1])), to_screen((r[2], r[3])));
        painter.rect_stroke(
            b,
            0.0,
            egui::Stroke::new(1.0, Color32::from_rgb(80, 160, 255)),
            egui::StrokeKind::Middle,
        );
    }
    let primary = |r: &egui::Response| r.dragged_by(PointerButton::Primary);
    // A drag starts after the pointer moves a little: its first point is
    // where the button went down.
    let origin = resp
        .drag_started_by(PointerButton::Primary)
        .then(|| ui.input(|i| i.pointer.press_origin()))
        .flatten();
    match w.tool {
        BgTool::Click => {
            let resp = resp.on_hover_cursor(egui::CursorIcon::Crosshair);
            if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
                && rect.contains(p)
            {
                let (x, y) = to_px(p);
                w.remove_at((x as u32).min(cw - 1), (y as u32).min(ch - 1));
            }
        }
        BgTool::Erase | BgTool::Restore | BgTool::MarkKeep | BgTool::MarkRemove => {
            let mark = matches!(w.tool, BgTool::MarkKeep | BgTool::MarkRemove);
            // The brush outline under the pointer.
            if let Some(p) = resp.hover_pos() {
                let r = w.brush_radius_px() as f32 * px_to_screen;
                let c = match w.tool {
                    BgTool::MarkKeep => Color32::from_rgb(40, 200, 60),
                    BgTool::Restore => Color32::from_rgb(80, 160, 255),
                    _ => Color32::from_rgb(255, 80, 80),
                };
                painter.circle_stroke(p, r, egui::Stroke::new(1.5, c));
            }
            if let Some(o) = origin {
                let (x, y) = to_px(o);
                if mark { w.mark_to(x, y) } else { w.brush_to(x, y) }
            }
            if (primary(&resp) || resp.clicked())
                && let Some(p) = resp.interact_pointer_pos()
            {
                let (x, y) = to_px(p);
                if mark { w.mark_to(x, y) } else { w.brush_to(x, y) }
            }
            if resp.drag_stopped_by(PointerButton::Primary) || resp.clicked() {
                if mark { w.end_mark() } else { w.end_stroke() }
            }
        }
        BgTool::Subject | BgTool::Box | BgTool::Lasso => {
            let resp = resp.on_hover_cursor(egui::CursorIcon::Crosshair);
            if let Some(o) = origin {
                w.drag.clear();
                w.drag.push(to_px(o));
            }
            if primary(&resp)
                && let Some(p) = resp.interact_pointer_pos()
            {
                let (x, y) = to_px(p);
                if w.tool == BgTool::Subject && w.drag.len() == 2 {
                    w.drag[1] = (x, y);
                } else if w.tool == BgTool::Subject {
                    w.drag.push((x, y));
                } else {
                    w.shape_to(x, y);
                }
            }
            let line = egui::Stroke::new(1.5, Color32::from_rgb(255, 200, 0));
            match (w.tool, w.drag.first(), w.drag.last()) {
                (BgTool::Lasso, Some(_), _) => {
                    let mut pts: Vec<egui::Pos2> = w.drag.iter().map(|p| to_screen(*p)).collect();
                    pts.push(pts[0]);
                    painter.add(egui::Shape::line(pts, line));
                }
                (_, Some(&a), Some(&b)) => {
                    painter.rect_stroke(
                        Rect::from_two_pos(to_screen(a), to_screen(b)),
                        0.0,
                        line,
                        egui::StrokeKind::Middle,
                    );
                }
                _ => {}
            }
            if resp.drag_stopped_by(PointerButton::Primary) {
                w.end_shape();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_art::RgbaImage;
    use image::Rgba;

    use super::*;
    use crate::test_util::Harness;

    /// A 200 × 100 px backdrop, 100 mm wide, at (0, 0): a gray wall, a
    /// red disk and a blue bar.
    fn with_photo() -> Harness {
        let mut h = Harness::empty();
        let img = RgbaImage::from_fn(200, 100, |x, y| {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let noise = ((x * 7 + y * 13) % 6) as u8;
            if (fx - 50.0).hypot(fy - 50.0) < 35.0 {
                Rgba([200, 20 + noise, 30, 255])
            } else if (110..190).contains(&x) && (46..54).contains(&y) {
                Rgba([20, 40, 200 - noise, 255])
            } else {
                Rgba([180 + noise, 180, 175, 255])
            }
        });
        let png = emb_art::encode_png(&img).unwrap();
        h.st.doc.edit("Add backdrop", |d| {
            let mut b = emb_model::Backdrop::new("photo.png", png, (200, 100), Point::new(50.0, 25.0), 100.0);
            b.origin = Point::ZERO;
            d.backdrop = Some(b);
            d.rotation = Default::default();
        });
        h
    }

    #[test]
    fn the_steps_make_a_design_in_one_undo_step() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        // Background: the gray wall goes; the content is cut to the shapes.
        w.remove_edges();
        let (img, at) = &*w.content();
        assert_eq!(at.0, 15, "the disk starts at x = 15 px");
        assert!(img.width() < 200);
        // Simplify: 0.5 mm a pixel, the colors of the picture.
        w.pixel_mm = 0.5;
        w.colors = 3;
        let r = w.reduced();
        assert!(r.width < 200 && r.palette.len() >= 2, "{} {:?}", r.width, r.palette);
        // Every color kept: a fill for the disk, a satin for the bar.
        let steps = h.st.doc.undo_len();
        assert!(h.st.finish_wizard(&mut w) >= 2);
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let d = h.st.doc.design();
        assert!(
            d.objects()
                .iter()
                .any(|o| matches!(o.kind, emb_model::ObjectKind::Fill { .. }))
        );
        assert!(
            d.objects()
                .iter()
                .any(|o| matches!(o.kind, emb_model::ObjectKind::Satin { .. }))
        );
        // Grouped, the backdrop hidden, and the disk where it was on the backdrop.
        assert!(d.objects().iter().all(|o| o.group.is_some()));
        assert!(!d.backdrop.as_ref().unwrap().visible);
        let fill = d
            .objects()
            .iter()
            .find(|o| matches!(o.kind, emb_model::ObjectKind::Fill { .. }))
            .unwrap();
        let c = d.bbox_of(&[fill.id]).unwrap().center();
        assert!(c.distance(Point::new(25.0, 25.0)) < 1.0, "{c:?}");
    }

    #[test]
    fn left_out_and_merged_colors_and_a_new_width() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.remove_edges();
        w.colors = 2;
        let r = w.reduced();
        assert_eq!(r.palette.len(), 2);
        // Leave out the first color: one color sews.
        w.choices[0].keep = false;
        assert_eq!(w.final_order().len(), 1);
        // Merge it into the second: one color, which has all areas.
        w.choices[0].keep = true;
        w.choices[0].merge_into = Some(1);
        assert_eq!(w.final_order(), vec![1]);
        // Twice as wide: the objects are twice as wide.
        let natural = w.natural_width_mm();
        let current = CurrentProperties::default();
        let one = w.objects(&current);
        let bb = |objs: &[SmartObject]| {
            let mut d = Design::new("");
            let t = d.add_thread("t", Rgb(0, 0, 0));
            let ids: Vec<_> = objs.iter().map(|o| d.add_object(t, o.kind.clone())).collect();
            d.bbox_of(&ids).unwrap()
        };
        let b1 = bb(&one);
        w.width_mm = Some(2.0 * natural);
        let two = w.objects(&current);
        let b2 = bb(&two);
        assert!(
            (b2.width() / b1.width() - 2.0).abs() < 0.1,
            "{} {}",
            b1.width(),
            b2.width()
        );
        assert!(b1.center().distance(b2.center()) < 1.0);
    }

    #[test]
    fn each_step_draws_and_the_buttons_go_through_them() {
        let mut h = with_photo();
        h.st.open_wizard();
        let ctx = egui::Context::default();
        let frame = |st: &mut AppState, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1400.0, 900.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| window(ui.ctx(), st));
            out.textures_delta.clear();
        };
        for make in [Make::Areas, Make::Photo, Make::Cross] {
            h.st.wizard.as_mut().unwrap().make = make;
            let steps = h.st.wizard.as_ref().unwrap().steps();
            for &s in steps {
                h.st.wizard.as_mut().unwrap().step = s;
                frame(&mut h.st, vec![]);
                frame(&mut h.st, vec![]);
                assert_eq!(h.st.wizard.as_ref().map(|w| w.step), Some(s));
            }
        }
        // Drawing changed nothing in the design.
        assert_eq!(h.st.doc.undo_len(), 1);
    }

    /// Draws one frame of the wizard window.
    fn draw(ctx: &egui::Context, st: &mut AppState) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1400.0, 900.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| window(ui.ctx(), st));
        out.textures_delta.clear();
    }

    /// Draws frames until the background job is done. The limit is a time,
    /// not a number of frames: a slow computer (a CI runner with two cores
    /// and all the other tests) needs more frames.
    fn draw_until_done(ctx: &egui::Context, st: &mut AppState) {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while std::time::Instant::now() < end {
            draw(ctx, st);
            if !st.wizard.as_ref().unwrap().working() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the job did not end");
    }

    #[test]
    fn the_previews_are_made_in_the_background() {
        let mut h = with_photo();
        h.st.open_wizard();
        let ctx = egui::Context::default();
        let w = h.st.wizard.as_mut().unwrap();
        w.remove_edges();
        w.colors = 3;
        w.step = Step::Colors;
        // The first frame starts a job and does not wait for it.
        draw(&ctx, &mut h.st);
        let w = h.st.wizard.as_ref().unwrap();
        assert!(w.working() && w.ready_reduced().is_none());
        draw_until_done(&ctx, &mut h.st);
        let w = h.st.wizard.as_mut().unwrap();
        let n = w
            .ready_reduced()
            .expect("the job made the simplified picture")
            .palette
            .len();
        assert_eq!(w.choices.len(), n);
        // A new setting starts a new job; its new colors reset the choices.
        w.colors = 2;
        w.choices[0].keep = false;
        draw(&ctx, &mut h.st);
        assert!(h.st.wizard.as_ref().unwrap().working());
        draw_until_done(&ctx, &mut h.st);
        let w = h.st.wizard.as_mut().unwrap();
        assert_eq!(w.ready_reduced().unwrap().palette.len(), 2);
        assert_eq!(w.choices.len(), 2);
        assert!(w.choices.iter().all(|c| c.keep));
        // A choice that changes no colors stays when the job ends.
        w.choices[1].stitch = SmartStitch::Fill;
        w.step = Step::Finish;
        draw(&ctx, &mut h.st);
        assert!(h.st.wizard.as_ref().unwrap().working());
        // Finish waits for the job.
        let mut w = h.st.wizard.take().unwrap();
        assert!(h.st.finish_wizard(&mut w) >= 1);
        assert_eq!(w.choices[1].stitch, SmartStitch::Fill);
        assert!(!w.working());
    }

    #[test]
    fn the_finish_step_checks_the_result_and_fixes_it() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        let current = CurrentProperties::default();
        // The wall goes; four colors for a red disk and a blue bar: the
        // grain gives reds or blues that are almost the same.
        w.remove_edges();
        w.colors = 4;
        w.step = Step::Finish;
        assert!(w.ready_checks(&current).is_none());
        w.work.preview(Step::Finish, &current);
        let checks = w.ready_checks(&current).expect("made with the stitch picture");
        let near = checks
            .iter()
            .find_map(|c| match c.fix {
                Some(Fix::Merge(a, b)) => Some((a, b)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{checks:?}"));
        // The fix merges the less common color into the other; the checks are
        // made again for the new settings.
        w.apply(Fix::Merge(near.0, near.1));
        assert_eq!(w.choices[near.0 as usize].merge_into, Some(near.1));
        assert!(w.ready_checks(&current).is_none());
        w.work.preview(Step::Finish, &current);
        let again = w.ready_checks(&current).unwrap();
        assert!(
            !again.iter().any(|c| c.fix == Some(Fix::Merge(near.0, near.1))),
            "{again:?}"
        );
        // The other fixes change their settings.
        w.apply(Fix::Specks(3.0));
        w.apply(Fix::Detail(0.6));
        w.apply(Fix::Overlap(0.1));
        w.apply(Fix::NoLayers);
        w.choices[0].stitch = SmartStitch::Satin;
        w.apply(Fix::SatinMax(5.0));
        assert_eq!((w.speck_mm2, w.pixel_mm, w.spec.overlap_mm), (3.0, 0.6, 0.1));
        assert_eq!(w.spec.layer_max_mm2, 0.0);
        assert_eq!(w.choices[0].stitch, SmartStitch::Auto);
        // Photo stitch has no checks.
        w.make = Make::Photo;
        assert!(w.ready_checks(&current).is_none());
    }

    #[test]
    fn the_colors_can_come_from_the_thread_chart() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.remove_edges();
        w.colors = 4;
        let own = w.reduced().palette.clone();
        let chart = h.st.charts.first().expect("a built-in chart").clone();
        let colors: Vec<Rgb> = chart.threads.iter().map(|t| t.color).collect();
        w.chart = Some((hash(&colors), Arc::new(colors.clone())));
        let r = w.reduced();
        assert!(r.palette.iter().all(|c| colors.contains(c)), "{:?}", r.palette);
        assert!(r.palette.len() <= own.len());
        // New colors: new choices for them.
        assert_eq!(w.choices.len(), r.palette.len());
    }

    #[test]
    fn cross_stitch_adds_cross_stitch_objects_on_the_grid() {
        let mut h = with_photo();
        h.st.settings.defaults.cross_cell_mm = 2.0;
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.make = Make::Cross;
        w.remove_edges();
        w.cross.colors = 3;
        let objs = w.cross_objects();
        assert!(!objs.is_empty() && objs.len() <= 3);
        let grid = h.st.cross_grid();
        assert!(objs.iter().all(|o| o.cross.same_grid(&grid)));
        // A larger cell: fewer cells.
        let before: usize = objs.iter().map(|o| o.cross.cells.len()).sum();
        w.cross_grid.set_cell_mm(4.0);
        let after: usize = w.cross_objects().iter().map(|o| o.cross.cells.len()).sum();
        assert!(after * 3 < before, "{before} then {after}");
        let n = h.st.finish_wizard(&mut w);
        assert_eq!(n, h.st.doc.design().objects().len());
        assert!(
            h.st.doc
                .design()
                .objects()
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::CrossStitch(_)))
        );
    }

    #[test]
    fn a_photo_stitch_adds_stitch_objects() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.make = Make::Photo;
        w.remove_edges();
        // Rows: one stitch object in the thread color.
        let objs = w.photo_objects();
        assert_eq!(objs.len(), 1);
        assert!(matches!(objs[0].kind, ObjectKind::Stitches { .. }));
        // Color rows: one object for each thread color.
        w.photo.style = PhotoStyle::ColorRows;
        w.photo.colors = 2;
        assert_eq!(w.photo_objects().len(), 2);
        assert_eq!(h.st.finish_wizard(&mut w), 2);
        let d = h.st.doc.design();
        assert!(
            d.objects()
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::Stitches { .. }))
        );
        // The stitches are over the shapes of the picture (the disk starts at 7.5 mm).
        let b = d
            .bbox_of(&d.objects().iter().map(|o| o.id).collect::<Vec<_>>())
            .unwrap();
        assert!(
            b.min.x > 7.0 && b.max.x < 98.0 && b.min.y > 7.0 && b.max.y < 43.0,
            "{b:?}"
        );
    }

    #[test]
    fn recognize_outlines_makes_a_fill_of_a_stitch_object() {
        let mut h = Harness::empty();
        // Rows of stitches over a 20 x 10 mm block.
        let mut path = emb_geom::Path::new().move_to(Point::new(0.0, 0.0));
        for k in 0..=25 {
            let y = f64::from(k) * 0.4;
            let (a, b) = if k % 2 == 0 { (0.0, 20.0) } else { (20.0, 0.0) };
            path = path.line_to(Point::new(a, y)).line_to(Point::new(b, y));
        }
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Black", Rgb(0, 0, 0));
            d.add_object(
                t,
                ObjectKind::Stitches {
                    path,
                    trims: Vec::new(),
                },
            )
        });
        h.st.selection.set(id);
        let steps = h.st.doc.undo_len();
        h.st.run(crate::ribbon::Cmd::RecognizeOutlines, &egui::Context::default());
        assert_eq!(h.st.doc.undo_len(), steps + 1);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let o = &d.objects()[0];
        assert!(matches!(o.kind, ObjectKind::Fill { .. }), "{:?}", o.kind);
        let b = d.bbox_of(&[o.id]).unwrap();
        assert!(
            (b.width() - 20.4).abs() < 0.6 && (b.height() - 10.4).abs() < 0.6,
            "{b:?}"
        );
    }

    #[test]
    fn cut_out_marks_box_lasso_and_undo() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        // A box round the disk (center 50, 50, r 35): the wall and the bar go.
        w.tool = BgTool::Subject;
        w.shape_to(10.0, 10.0);
        w.shape_to(90.0, 90.0);
        w.end_shape();
        assert_eq!(w.cut.get_pixel(50, 50)[3], 255, "{}", w.note);
        assert_eq!(w.cut.get_pixel(150, 50)[3], 0);
        assert_eq!(w.cut.get_pixel(12, 12)[3], 0);
        // A remove mark on the middle of the disk cuts it away there.
        w.tool = BgTool::MarkRemove;
        w.brush_mm = 6.0;
        w.mark_to(50.0, 50.0);
        w.mark_to(52.0, 50.0);
        w.end_mark();
        assert_eq!(w.cut.get_pixel(51, 50)[3], 0);
        // Undo brings the disk back and the mark goes.
        w.undo();
        assert_eq!(w.cut.get_pixel(51, 50)[3], 255);
        assert!(w.subject.as_ref().unwrap().strokes.is_empty());
        // A box erase clears the left half of the disk.
        w.tool = BgTool::Box;
        w.shape_action = ShapeAction::EraseInside;
        w.shape_to(0.0, 0.0);
        w.shape_to(50.0, 100.0);
        w.end_shape();
        assert_eq!(w.cut.get_pixel(30, 50)[3], 0);
        assert_eq!(w.cut.get_pixel(70, 50)[3], 255);
        // A lasso "keep only inside" round the right part keeps only it.
        w.start_again();
        w.tool = BgTool::Lasso;
        w.shape_action = ShapeAction::EraseOutside;
        for (x, y) in [(60.0, 20.0), (95.0, 20.0), (95.0, 80.0), (60.0, 80.0)] {
            w.shape_to(x, y);
        }
        w.end_shape();
        assert_eq!(w.cut.get_pixel(70, 50)[3], 255);
        assert_eq!(w.cut.get_pixel(30, 50)[3], 0);
        assert_eq!(w.cut.get_pixel(150, 50)[3], 0);
    }

    #[test]
    fn picture_to_design_opens_the_wizard_on_the_backdrop() {
        let mut h = with_photo();
        h.st.run(crate::ribbon::Cmd::PictureWizard, &egui::Context::default());
        assert!(h.st.wizard.is_some());
    }

    #[test]
    fn picture_to_cross_stitch_opens_the_wizard_for_cross_stitch() {
        let mut h = with_photo();
        h.st.run(crate::ribbon::Cmd::PictureToCross, &egui::Context::default());
        assert_eq!(h.st.wizard.as_ref().map(|w| w.make), Some(Make::Cross));
    }

    #[test]
    fn brushes_and_clean_up() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.remove_edges();
        let before = w.cut.clone();
        // An erase stroke through the disk, one undo step.
        w.tool = BgTool::Erase;
        w.brush_mm = 4.0;
        w.brush_to(30.0, 50.0);
        w.brush_to(70.0, 50.0);
        w.end_stroke();
        assert_eq!(w.cut.get_pixel(50, 50)[3], 0);
        w.undo();
        assert_eq!(w.cut, before);
        // Restore brings the wall back where it paints.
        w.tool = BgTool::Restore;
        w.brush_to(5.0, 5.0);
        w.end_stroke();
        assert_eq!(w.cut.get_pixel(5, 5)[3], 255);
        // Clean up removes that bit of wall again (under 20 mm²).
        w.bits_mm2 = 20.0;
        assert_eq!(w.cleaned().get_pixel(5, 5)[3], 0);
        // Shrink: the disk loses a pixel at its edge.
        let wide = w.content().0.width();
        w.shrink_px = 1;
        assert_eq!(w.content().0.width(), wide - 2);
    }

    #[test]
    fn a_click_removes_a_color_and_undo_brings_it_back() {
        let mut h = with_photo();
        h.st.open_wizard();
        let mut w = h.st.wizard.take().unwrap();
        w.connected = false;
        w.remove_at(5, 5);
        assert_eq!(w.cut.get_pixel(5, 5)[3], 0);
        assert_eq!(w.cut.get_pixel(50, 50)[3], 255);
        w.undo();
        assert_eq!(w.cut.get_pixel(5, 5)[3], 255);
        // No backdrop: the wizard does not open.
        let mut e = Harness::empty();
        e.st.open_wizard();
        assert!(e.st.wizard.is_none());
    }

    /// A harness with `data` (a picture file of `pixels`) as the backdrop,
    /// `width_mm` wide.
    fn with_backdrop(data: Vec<u8>, pixels: (u32, u32), width_mm: f64) -> Harness {
        let mut h = Harness::empty();
        h.st.doc.edit("Add backdrop", |d| {
            let mut b = emb_model::Backdrop::new("picture", data, pixels, Point::ZERO, width_mm);
            b.origin = Point::ZERO;
            d.backdrop = Some(b);
            d.rotation = Default::default();
        });
        h
    }

    /// Goes through each step of each kind of design with frames of the
    /// window. The wizard stays open (an error would close it), and each
    /// preview texture is as large as the graphics card takes, or smaller.
    fn through_the_steps(h: &mut Harness) {
        let ctx = egui::Context::default();
        let side = crate::resources::max_texture_side(&ctx);
        for make in [Make::Areas, Make::Photo, Make::Cross] {
            h.st.wizard.as_mut().unwrap().make = make;
            let steps = h.st.wizard.as_ref().unwrap().steps();
            for &s in steps {
                h.st.wizard.as_mut().unwrap().step = s;
                draw_until_done(&ctx, &mut h.st);
                draw(&ctx, &mut h.st);
                let w = h.st.wizard.as_ref().expect("the wizard is open");
                assert_eq!(w.step, s);
                let [tw, th] = w.texture.as_ref().unwrap().1.size();
                assert!(tw <= side && th <= side, "{make:?} {s:?}: {tw} x {th}");
            }
        }
    }

    #[test]
    fn a_very_large_photo_opens_at_the_work_size_and_makes_a_design() {
        use image::ImageEncoder;
        // A 6000 x 4000 photo (as a 24 MP camera makes): a red disk on a light wall.
        let (w, h) = (6000, 4000);
        let rgb = image::RgbImage::from_fn(w, h, |x, y| {
            let (fx, fy) = (f64::from(x) - 3000.0, f64::from(y) - 2000.0);
            if fx.hypot(fy) < 1500.0 {
                image::Rgb([200, 30, 40])
            } else {
                image::Rgb([235, 235, 228])
            }
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
            .write_image(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .unwrap();
        drop(rgb);
        let mut hs = with_backdrop(jpeg, (w, h), 100.0);
        hs.st.open_wizard();
        let wz = hs.st.wizard.as_mut().expect("the wizard opens");
        assert_eq!(wz.source.dimensions(), (smart::WORK_PX, 800));
        wz.remove_edges();
        through_the_steps(&mut hs);
        hs.st.wizard.as_mut().unwrap().make = Make::Areas;
        let mut wz = hs.st.wizard.take().unwrap();
        assert!(hs.st.finish_wizard(&mut wz) >= 1);
    }

    #[test]
    fn odd_pictures_do_not_stop_the_program() {
        let png = |w: u32, h: u32, f: &dyn Fn(u32, u32) -> Rgba<u8>| {
            (emb_art::encode_png(&RgbaImage::from_fn(w, h, f)).unwrap(), (w, h))
        };
        let dot = |x: u32, y: u32| {
            if (x + y) % 7 < 3 {
                Rgba([20, 30, 200, 255])
            } else {
                Rgba([240, 240, 240, 255])
            }
        };
        let pictures = [
            png(1, 1, &|_, _| Rgba([200, 0, 0, 255])),
            // Very thin: as wide as 1 pixel, or 1 pixel high.
            png(1, 3000, &dot),
            png(3000, 1, &dot),
            // All clear.
            png(40, 30, &|_, _| Rgba([0, 0, 0, 0])),
            // Tall: the stitch picture of Finish is higher than a texture can be.
            png(50, 1200, &dot),
        ];
        for (data, pixels) in pictures {
            // 10 mm wide: the tall picture is 240 mm high, 4800 px in the stitch picture.
            let mut h = with_backdrop(data, pixels, 10.0);
            h.st.open_wizard();
            assert!(h.st.wizard.is_some(), "{pixels:?}: {}", h.st.message);
            through_the_steps(&mut h);
        }
        // Data that is not a picture, or only part of one: a message.
        let (full, pixels) = png(300, 200, &dot);
        for data in [b"not a picture".to_vec(), full[..full.len() / 2].to_vec()] {
            let mut h = with_backdrop(data, pixels, 40.0);
            h.st.open_wizard();
            assert!(h.st.wizard.is_none());
            assert!(
                h.st.message.contains("Cannot read the backdrop picture"),
                "{}",
                h.st.message
            );
        }
    }

    #[test]
    fn an_error_in_the_wizard_closes_it_and_the_program_goes_on() {
        let mut h = with_photo();
        h.st.open_wizard();
        guarded(&mut h.st, |st| {
            assert!(st.wizard.is_some());
            panic!("an error in a step (this test makes it)");
        });
        assert!(h.st.wizard.is_none());
        assert!(h.st.message.contains("stopped because of an error"));
        // No error: the wizard stays.
        h.st.open_wizard();
        guarded(&mut h.st, |_| {});
        assert!(h.st.wizard.is_some());
    }
}
