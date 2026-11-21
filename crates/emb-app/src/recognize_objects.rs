//! Recognize Objects (P18 Milestone 4, ADR 0134): the selected stitch
//! objects (from a machine file) become objects with stitch types: running
//! objects, satin columns and fills (`emb_art::part_objects::recognize`).
//!
//! The window shows the stitches of the file and the objects sewn again
//! next to each other, with the stitch counts and the places where the two
//! differ (red). The list has each object that the stitches become: the
//! user keeps single objects as stitches. Accept makes the change as one
//! undo step; stitches kept one after the other are one stitch object.
//! With Options > Recognize objects when a machine file opens, Open does it
//! at once (`recognize_all`).
//!
//! Outlines (P18 Milestone 7, ADR 0137): the user opens the outlines of
//! the same design (a vector file). The program puts them on the stitches
//! by their sizes, or by two points that the user clicks (an outline point,
//! then the same point of the stitches, two times). Each outline takes the
//! kind and the settings of the stitches inside it
//! (`emb_art::part_outlines`); the window draws the outlines on the stitches
//! of the file.

use std::collections::HashMap;
use std::ops::Range;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};
use emb_art::part_objects::{Made, recognize, stitches_of};
use emb_art::part_outlines::{self, Outline};
use emb_geom::{BBox, Point, Transform};
use emb_model::{Design, ObjectId, ObjectKind, Rgb, StitchDesign, StitchKind};
use emb_render::{Segment, SegmentKind};
use emb_stitch::PlanOptions;

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The size of each picture, screen points.
const PICTURE: Vec2 = Vec2::new(340.0, 260.0);
/// The pictures are compared in cells of at least this size, mm, and at
/// most this many cells on the long side.
const DIFF_CELL_MM: f64 = 1.0;
const DIFF_CELLS: f64 = 80.0;
/// A cell differs if one side has at least this much thread in it (mm of
/// thread for each mm of the cell side) and the other side less than this
/// part of it.
const DIFF_MIN: f64 = 0.8;
const DIFF_PART: f64 = 0.4;
/// The colors of the pictures: the stitches of the file, kept stitches,
/// runs, satins, fills, the places that differ, the background.
const FILE: Rgb = Rgb(70, 70, 70);
const KEPT: Rgb = Rgb(150, 150, 150);
const RUN: Rgb = Rgb(40, 110, 235);
const SATIN: Rgb = Rgb(240, 140, 20);
const FILL: Rgb = Rgb(30, 170, 80);
const DIFFER: Color32 = Color32::from_rgba_premultiplied(140, 20, 20, 110);
/// The outlines on the picture of the file: the ones that became objects,
/// the others.
const OUTLINE_USED: Color32 = Color32::from_rgb(200, 30, 200);
const OUTLINE_OTHER: Color32 = Color32::from_rgb(230, 160, 230);
/// A click snaps to an outline point or a needle point this near, mm.
const SNAP_MM: f64 = 3.0;
const BACKGROUND: Rgb = Rgb(250, 250, 250);

/// A stitch: from a needle point to the next, mm.
type Stitch = (Point, Point);

/// A stitch object and the objects that its stitches become.
pub struct Target {
    pub id: ObjectId,
    s: StitchDesign,
    /// The objects that the stitches make with no outlines.
    base: Vec<Made>,
    pub made: Vec<Made>,
    /// The outline of each made object, if it has one.
    pub from: Vec<Option<usize>>,
    /// For each made object: make it (`true`) or keep its stitches.
    pub make: Vec<bool>,
    /// The stitches that each made object sews again.
    sewn: Vec<Vec<(Point, Point)>>,
}

/// The Recognize Objects window.
pub struct RecognizeDialog {
    pub targets: Vec<Target>,
    pictures: Option<Pictures>,
    /// The object of the list row under the pointer.
    hovered: Option<(usize, usize)>,
    /// The outlines of the design, if the user opened them.
    pub outlines: Option<Outlines>,
    opts: PlanOptions,
}

/// The outlines that the user opened, and how they are on the stitches.
pub struct Outlines {
    /// The name of the file.
    pub file: String,
    /// The outlines as the file has them.
    raw: Vec<Outline>,
    /// From the file to the stitches.
    pub transform: Transform,
    /// The outlines on the stitches, each area on its own.
    pub placed: Vec<Outline>,
    /// Align by two points: the points clicked (an outline point, then the
    /// same point of the stitches, two times).
    pub aligning: Option<Vec<Point>>,
    /// Changes with each new place of the outlines (for the pictures).
    version: u64,
}

/// The two pictures for one choice of the objects to make.
struct Pictures {
    choice: Vec<Vec<bool>>,
    /// The version of the outlines.
    version: u64,
    file: TextureHandle,
    objects: TextureHandle,
    /// The area of the pictures, mm.
    bbox: BBox,
    /// The cells that differ, mm.
    differ: Vec<BBox>,
    cell_mm: f64,
}

/// The color of a made object in the picture of the objects.
fn kind_color(kind: &ObjectKind) -> Rgb {
    match kind {
        ObjectKind::Running { .. } => RUN,
        ObjectKind::Satin { .. } => SATIN,
        ObjectKind::Fill { .. } => FILL,
        _ => KEPT,
    }
}

/// The stitches of the records `rs` of `s`.
fn segments(s: &StitchDesign, rs: &[Range<usize>]) -> Vec<(Point, Point)> {
    rs.iter()
        .flat_map(|r| r.clone())
        .filter(|&i| i > 0 && s.records[i].kind == StitchKind::Stitch)
        .map(|i| (s.records[i - 1].pos, s.records[i].pos))
        .collect()
}

/// The stitches that each made object sews: the plan of the objects (with
/// the plan options `opts`), or the stitches of the file for stitches.
fn sew_again(s: &StitchDesign, made: &[Made], opts: &PlanOptions) -> Vec<Vec<(Point, Point)>> {
    let mut d = Design::new("");
    let t = d.add_thread("", Rgb(0, 0, 0));
    let ids: Vec<Option<ObjectId>> = made
        .iter()
        .map(|m| m.recognized().then(|| d.add_object(t, m.kind.clone())))
        .collect();
    let plan = emb_stitch::build_plan(&d, opts, &mut emb_stitch::StitchCache::default());
    made.iter()
        .zip(ids)
        .map(|(m, id)| match id {
            Some(id) => plan
                .spans
                .iter()
                .filter(|sp| sp.object == id)
                .flat_map(|sp| segments(&plan.stitches, std::slice::from_ref(&sp.records)))
                .collect(),
            None => segments(s, &m.records),
        })
        .collect()
}

/// The objects of a target for its choice: the made objects in order; the
/// stitches that stay, one after the other, are one stitch object.
fn kinds_of(t: &Target) -> Vec<ObjectKind> {
    let mut out = Vec::new();
    let mut kept: Vec<Range<usize>> = Vec::new();
    for (m, &make) in t.made.iter().zip(&t.make) {
        if make && m.recognized() {
            if !kept.is_empty() {
                out.push(stitches_of(&t.s, &std::mem::take(&mut kept)));
            }
            out.push(m.kind.clone());
        } else {
            kept.extend(m.records.iter().cloned());
        }
    }
    if !kept.is_empty() {
        out.push(stitches_of(&t.s, &kept));
    }
    out
}

/// The thread of the segments in cells of `cell` mm.
fn thread_in_cells(segs: &[(Point, Point)], cell: f64) -> HashMap<(i64, i64), f64> {
    let mut out: HashMap<(i64, i64), f64> = HashMap::new();
    for &(a, b) in segs {
        let len = a.distance(b);
        let n = (len / (cell / 2.0)).ceil().max(1.0) as usize;
        for k in 0..n {
            let p = a.lerp(b, (k as f64 + 0.5) / n as f64);
            *out.entry(((p.x / cell).floor() as i64, (p.y / cell).floor() as i64))
                .or_default() += len / n as f64;
        }
    }
    out
}

/// The cells where the thread of `a` and `b` differ (see [`DIFF_MIN`]).
fn differ(a: &[(Point, Point)], b: &[(Point, Point)], cell: f64) -> Vec<BBox> {
    let (ma, mb) = (thread_in_cells(a, cell), thread_in_cells(b, cell));
    let mut keys: Vec<(i64, i64)> = ma.keys().chain(mb.keys()).copied().collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .filter(|k| {
            let (x, y) = (ma.get(k).copied().unwrap_or(0.0), mb.get(k).copied().unwrap_or(0.0));
            let most = x.max(y);
            most >= DIFF_MIN * cell && x.min(y) < DIFF_PART * most
        })
        .map(|(x, y)| {
            BBox::new(
                Point::new(x as f64 * cell, y as f64 * cell),
                Point::new((x + 1) as f64 * cell, (y + 1) as f64 * cell),
            )
        })
        .collect()
}

impl RecognizeDialog {
    fn choice(&self) -> Vec<Vec<bool>> {
        self.targets.iter().map(|t| t.make.clone()).collect()
    }

    fn version(&self) -> u64 {
        self.outlines.as_ref().map_or(0, |o| o.version)
    }

    /// The box of the stitches of all targets.
    fn stitches_bbox(&self) -> Option<BBox> {
        BBox::from_points(self.targets.iter().flat_map(|t| {
            t.s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
        }))
    }

    /// Opens the outlines `raw` of the file `file`: they go on the
    /// stitches by their sizes.
    pub fn set_outlines(&mut self, file: &str, raw: Vec<Outline>) {
        let t = match (part_outlines::outlines_bbox(&raw), self.stitches_bbox()) {
            (Some(o), Some(s)) => part_outlines::fit_by_size(o, s),
            _ => Transform::IDENTITY,
        };
        self.outlines = Some(Outlines {
            file: file.to_string(),
            raw,
            transform: t,
            placed: Vec::new(),
            aligning: None,
            version: 0,
        });
        self.place_outlines(t);
    }

    /// Puts the outlines on the stitches with `t` and makes the objects
    /// again; with no outlines, the objects of the stitches.
    pub fn place_outlines(&mut self, t: Transform) {
        let opts = self.opts.clone();
        let placed = match &mut self.outlines {
            Some(o) => {
                o.transform = t;
                o.version += 1;
                o.placed = part_outlines::split_areas(&part_outlines::transformed(&o.raw, &t));
                o.placed.clone()
            }
            None => Vec::new(),
        };
        for target in &mut self.targets {
            let (made, from) = if placed.is_empty() {
                (target.base.clone(), vec![None; target.base.len()])
            } else {
                part_outlines::with_outlines(&target.s, target.base.clone(), &placed)
            };
            target.make = made.iter().map(Made::recognized).collect();
            target.sewn = sew_again(&target.s, &made, &opts);
            target.made = made;
            target.from = from;
        }
        self.pictures = None;
    }

    /// The outlines that became objects.
    pub fn used_outlines(&self) -> usize {
        let mut used: Vec<usize> = self
            .targets
            .iter()
            .flat_map(|t| t.from.iter().flatten().copied())
            .collect();
        used.sort_unstable();
        used.dedup();
        used.len()
    }

    /// A click at `p` (mm) to align by two points: an outline point and a
    /// needle point in turn, each snapped to the nearest one. After the
    /// fourth click the outlines move.
    pub fn align_click(&mut self, p: Point) {
        let needles: Vec<Point> = self
            .targets
            .iter()
            .flat_map(|t| {
                t.s.records
                    .iter()
                    .filter(|r| r.kind == StitchKind::Stitch)
                    .map(|r| r.pos)
            })
            .collect();
        let Some(o) = &mut self.outlines else { return };
        let Some(clicks) = &mut o.aligning else { return };
        let snap = |pts: &mut dyn Iterator<Item = Point>| {
            pts.map(|q| (q.distance(p), q))
                .filter(|x| x.0 <= SNAP_MM)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map_or(p, |x| x.1)
        };
        let q = if clicks.len() % 2 == 0 {
            snap(&mut o.placed.iter().flat_map(|l| l.path.points().collect::<Vec<_>>()))
        } else {
            snap(&mut needles.into_iter())
        };
        clicks.push(q);
        if clicks.len() == 4 {
            let c = std::mem::take(clicks);
            o.aligning = None;
            if let Some(fit) = part_outlines::fit_two_points([c[0], c[2]], [c[1], c[3]]) {
                let t = o.transform.then(fit);
                self.place_outlines(t);
            }
        }
    }

    /// The stitches of the file, and the stitches of the objects for the
    /// choice (with their colors).
    fn stitches(&self) -> (Vec<Stitch>, Vec<(Stitch, Rgb)>) {
        let mut file = Vec::new();
        let mut objects = Vec::new();
        for t in &self.targets {
            for (k, m) in t.made.iter().enumerate() {
                let own = segments(&t.s, &m.records);
                if t.make[k] && m.recognized() {
                    let c = kind_color(&m.kind);
                    objects.extend(t.sewn[k].iter().map(|s| (*s, c)));
                } else {
                    objects.extend(own.iter().map(|s| (*s, KEPT)));
                }
                file.extend(own);
            }
        }
        (file, objects)
    }

    /// The stitch counts: in the file, and of the objects for the choice.
    pub fn counts(&self) -> (usize, usize) {
        let mut file = 0;
        let mut objects = 0;
        for t in &self.targets {
            for (k, m) in t.made.iter().enumerate() {
                file += m.stitch_count();
                objects += if t.make[k] && m.recognized() {
                    t.sewn[k].len()
                } else {
                    m.stitch_count()
                };
            }
        }
        (file, objects)
    }

    /// Makes the pictures for the choice.
    fn pictures(&self, ctx: &egui::Context) -> Option<Pictures> {
        let (file, objects) = self.stitches();
        let bbox = BBox::from_points(
            file.iter()
                .chain(objects.iter().map(|(s, _)| s))
                .flat_map(|(a, b)| [*a, *b]),
        )?
        .expand(1.0);
        let ppp = f64::from(ctx.pixels_per_point());
        let points_per_mm = (f64::from(PICTURE.x) / bbox.width()).min(f64::from(PICTURE.y) / bbox.height());
        let px_per_mm = points_per_mm * ppp;
        let thread_mm = (1.0 / px_per_mm).max(0.15);
        let seg = |(a, b): (Point, Point), color: Rgb| Segment {
            a,
            b,
            color,
            kind: SegmentKind::Stitch,
            ..Default::default()
        };
        let texture = |name: &str, segs: Vec<Segment>| {
            let pic = emb_render::raster::picture_at(&segs, &[], bbox, px_per_mm, thread_mm, Some(BACKGROUND));
            let img = egui::ColorImage::from_rgba_unmultiplied([pic.width as usize, pic.height as usize], &pic.rgba);
            ctx.load_texture(name, img, egui::TextureOptions::LINEAR)
        };
        let cell_mm = DIFF_CELL_MM.max(bbox.width().max(bbox.height()) / DIFF_CELLS);
        let object_segs: Vec<(Point, Point)> = objects.iter().map(|(s, _)| *s).collect();
        let differ = differ(&file, &object_segs, cell_mm);
        Some(Pictures {
            choice: self.choice(),
            version: self.version(),
            file: texture("recognize-file", file.into_iter().map(|s| seg(s, FILE)).collect()),
            objects: texture(
                "recognize-objects",
                objects.into_iter().map(|(s, c)| seg(s, c)).collect(),
            ),
            bbox,
            differ,
            cell_mm,
        })
    }
}

impl AppState {
    /// Recognize Objects: opens the window for the selected stitch objects.
    pub fn open_recognize_objects(&mut self) {
        let ids: Vec<ObjectId> = self.sel();
        if self.refuse_linked(&ids) {
            return;
        }
        let opts = self.plan_options();
        let design = self.doc.design();
        let targets: Vec<Target> = ids
            .iter()
            .filter_map(|id| design.object(*id))
            .filter_map(|o| {
                let (s, made) = recognize(&o.kind)?;
                if made.is_empty() {
                    return None;
                }
                let sewn = sew_again(&s, &made, &opts);
                Some(Target {
                    id: o.id,
                    make: made.iter().map(Made::recognized).collect(),
                    s,
                    from: vec![None; made.len()],
                    base: made.clone(),
                    made,
                    sewn,
                })
            })
            .collect();
        if targets.is_empty() {
            self.message = tr("Select stitch objects (from a machine file or a photo stitch) first.").into();
            return;
        }
        self.recognize_dialog = Some(RecognizeDialog {
            targets,
            pictures: None,
            hovered: None,
            outlines: None,
            opts,
        });
    }

    /// Asks for a vector file with the outlines of the design and puts them
    /// in the Recognize Objects window.
    fn open_outlines_dialog(&mut self, d: &mut RecognizeDialog) {
        let Some(p) = crate::dialog::FileDialog::new()
            .set_title(tr("Outlines of the design"))
            .add_filter(
                tr("Vector artwork (SVG, PDF, AI, EPS, EMF, WMF)"),
                &emb_formats::artwork::ARTWORK_EXTENSIONS,
            )
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let read = std::fs::read(&p)
            .map_err(|e| e.to_string())
            .and_then(|b| emb_formats::artwork::import_artwork(&ext, &b, true).map_err(|e| e.to_string()));
        match read.map(|i| outlines_of(&i)) {
            Ok(o) if !o.is_empty() => {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                d.set_outlines(&name, o);
            }
            Ok(_) => self.message = tr("The file has no filled or stroked shapes.").into(),
            Err(e) => self.message = fill("Cannot read {}: {}", &[&(p.display()), &e]),
        }
    }

    /// Accept: each stitch object becomes its objects, as one undo step.
    pub fn accept_recognized(&mut self, d: RecognizeDialog) {
        let work: Vec<(ObjectId, Vec<ObjectKind>)> = d.targets.iter().map(|t| (t.id, kinds_of(t))).collect();
        let recognized = work
            .iter()
            .flat_map(|(_, ks)| ks)
            .filter(|k| !matches!(k, ObjectKind::Stitches { .. }))
            .count();
        let new = self.doc.edit(tr("Recognize Objects"), |dd| {
            work.into_iter()
                .flat_map(|(id, ks)| crate::actions::replace_with(dd, id, ks))
                .collect::<Vec<_>>()
        });
        self.message = fill(
            "{} objects made from the stitches: {} with a stitch type, {} stitch objects.",
            &[&new.len(), &recognized, &(new.len().saturating_sub(recognized))],
        );
        self.selection.set_all(new);
    }
}

/// All stitch objects of `d` become their objects (Options: Recognize
/// objects when a machine file opens). The numbers of objects made and of
/// objects with a stitch type.
pub(crate) fn recognize_all(d: &mut Design) -> (usize, usize) {
    let ids: Vec<ObjectId> = d
        .objects()
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Stitches { .. }))
        .map(|o| o.id)
        .collect();
    let (mut made, mut recognized) = (0, 0);
    for id in ids {
        let Some((s, m)) = d.object(id).and_then(|o| recognize(&o.kind)) else {
            continue;
        };
        let t = Target {
            id,
            make: m.iter().map(Made::recognized).collect(),
            s,
            from: vec![None; m.len()],
            base: Vec::new(),
            made: m,
            sewn: Vec::new(),
        };
        let kinds = kinds_of(&t);
        recognized += kinds
            .iter()
            .filter(|k| !matches!(k, ObjectKind::Stitches { .. }))
            .count();
        made += kinds.len();
        crate::actions::replace_with(d, id, kinds);
    }
    (made, recognized)
}

/// The outlines of a vector file: its shapes with a fill are areas, the
/// others lines.
pub fn outlines_of(read: &emb_formats::svg::SvgImport) -> Vec<Outline> {
    read.objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Vector { path, style } => Some(Outline {
                path: path.clone(),
                filled: style.fill.is_some(),
                width_mm: style.stroke_mm,
                name: o.name.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// The rectangle of the picture of `bbox` in `rect` (fitted, centered), and
/// the map from mm to it.
fn fitted(rect: Rect, bbox: BBox) -> (Rect, impl Fn(Point) -> Pos2) {
    let s = (f64::from(rect.width()) / bbox.width()).min(f64::from(rect.height()) / bbox.height());
    let size = Vec2::new((bbox.width() * s) as f32, (bbox.height() * s) as f32);
    let inner = Rect::from_center_size(rect.center(), size);
    let min = bbox.min;
    let to = move |p: Point| inner.min + Vec2::new(((p.x - min.x) * s) as f32, ((p.y - min.y) * s) as f32);
    (inner, to)
}

/// The Recognize Objects window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.recognize_dialog.take() else {
        return;
    };
    let choice = d.choice();
    if d.pictures
        .as_ref()
        .is_none_or(|p| p.choice != choice || p.version != d.version())
    {
        d.pictures = d.pictures(ctx);
    }
    let mut open = true;
    let (mut accept, mut cancel, mut open_outlines) = (false, false, false);
    let mut click: Option<Point> = None;
    let (file_count, object_count) = d.counts();
    crate::dock::docked(
        egui::Window::new(tr("Recognize Objects")).id(egui::Id::new("Recognize Objects")),
        ctx,
        egui::Id::new("Recognize Objects"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        // The outlines: open, fit, align by two points.
        ui.horizontal_wrapped(|ui| {
            open_outlines = ui
                .button(tr("Outlines…"))
                .on_hover_text(tr(
                    "Open the outlines of the same design (SVG, PDF, AI, EPS, EMF or WMF). Each outline takes the \
                     kind and the settings of the stitches inside it.",
                ))
                .clicked();
            let used = d.used_outlines();
            let (mut fit, mut remove) = (false, false);
            if let Some(o) = &mut d.outlines {
                ui.label(fill(
                    "{}: {} outlines, {} made objects.",
                    &[&o.file, &o.placed.len(), &used],
                ));
                fit = ui
                    .button(tr("Fit by Size"))
                    .on_hover_text(tr("Put the outlines on the stitches by their sizes."))
                    .clicked();
                let mut two = o.aligning.is_some();
                if ui
                    .toggle_value(&mut two, tr("Align by Two Points"))
                    .on_hover_text(tr(
                        "Click a point of the outlines, then the same point of the stitches; then a second pair.",
                    ))
                    .changed()
                {
                    o.aligning = two.then(Vec::new);
                }
                remove = ui.button(tr("Remove Outlines")).clicked();
            }
            if fit && let Some(o) = &d.outlines {
                let (name, raw) = (o.file.clone(), o.raw.clone());
                d.set_outlines(&name, raw);
            } else if remove {
                d.outlines = None;
                d.place_outlines(Transform::IDENTITY);
            }
        });
        if let Some(clicks) = d.outlines.as_ref().and_then(|o| o.aligning.as_ref()) {
            ui.label(match clicks.len() {
                0 => tr("Click a point of the outlines."),
                1 => tr("Click the same point of the stitches."),
                2 => tr("Click a second point of the outlines."),
                _ => tr("Click the same point of the stitches."),
            });
        }
        if let Some(p) = &d.pictures {
            ui.horizontal(|ui| {
                for (title, tex, objects) in [(tr("File"), &p.file, false), (tr("Objects"), &p.objects, true)] {
                    ui.vertical(|ui| {
                        ui.strong(title);
                        let sense = if objects { Sense::hover() } else { Sense::click() };
                        let (rect, response) = ui.allocate_exact_size(PICTURE, sense);
                        let (inner, to) = fitted(rect, p.bbox);
                        let painter = ui.painter_at(rect);
                        // A click on the file picture aligns by two points.
                        if !objects
                            && response.clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            let s = f64::from(inner.width()) / p.bbox.width();
                            click = Some(Point::new(
                                p.bbox.min.x + f64::from(pos.x - inner.min.x) / s,
                                p.bbox.min.y + f64::from(pos.y - inner.min.y) / s,
                            ));
                        }
                        painter.image(
                            tex.id(),
                            inner,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        for c in &p.differ {
                            painter.rect_filled(Rect::from_two_pos(to(c.min), to(c.max)), 0.0, DIFFER);
                        }
                        // The outlines on the stitches of the file.
                        if !objects && let Some(o) = &d.outlines {
                            let used: Vec<usize> = d
                                .targets
                                .iter()
                                .flat_map(|t| t.from.iter().flatten().copied())
                                .collect();
                            for (j, l) in o.placed.iter().enumerate() {
                                let color = if used.contains(&j) { OUTLINE_USED } else { OUTLINE_OTHER };
                                for pl in l.path.flatten(0.2) {
                                    let mut pts: Vec<Pos2> = pl.points.iter().map(|q| to(*q)).collect();
                                    if (pl.closed || l.filled)
                                        && let Some(&f) = pts.first()
                                    {
                                        pts.push(f);
                                    }
                                    painter.add(egui::Shape::line(pts, Stroke::new(1.2, color)));
                                }
                            }
                            for (k, q) in o.aligning.iter().flatten().enumerate() {
                                let c = if k % 2 == 0 { OUTLINE_USED } else { Color32::BLACK };
                                painter.circle_stroke(to(*q), 5.0, Stroke::new(2.0, c));
                            }
                        }
                        // The object of the row under the pointer.
                        if objects && let Some((ti, mi)) = d.hovered {
                            let kind = &d.targets[ti].made[mi].kind;
                            for path in kind.paths() {
                                for pl in path.flatten(0.1) {
                                    let mut pts: Vec<Pos2> = pl.points.iter().map(|q| to(*q)).collect();
                                    if pl.closed
                                        && let Some(&f) = pts.first()
                                    {
                                        pts.push(f);
                                    }
                                    painter.add(egui::Shape::line(pts, Stroke::new(2.0, Color32::BLACK)));
                                }
                            }
                        }
                    });
                }
            });
            let area = p.differ.len() as f64 * p.cell_mm * p.cell_mm;
            ui.label(fill(
                "Stitches: {} in the file, {} as objects. Places that differ (red): {} mm².",
                &[&file_count, &object_count, &format!("{area:.0}")],
            ));
        }
        ui.horizontal_wrapped(|ui| {
            for (c, text) in [
                (RUN, tr("Running")),
                (SATIN, tr("Satin")),
                (FILL, tr("Fill")),
                (KEPT, tr("Stitches")),
            ] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 10.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, Color32::from_rgb(c.0, c.1, c.2));
                ui.label(text);
            }
        });
        ui.separator();
        let mut hovered = None;
        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            egui::Grid::new("recognize objects list")
                .num_columns(3)
                .striped(true)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    for (ti, t) in d.targets.iter_mut().enumerate() {
                        for mi in 0..t.made.len() {
                            let m = &t.made[mi];
                            let name = if t.from[mi].is_some() {
                                fill("{} (outline)", &[&tr(m.kind.type_name())])
                            } else {
                                tr(m.kind.type_name()).to_string()
                            };
                            let first = if m.recognized() {
                                ui.checkbox(&mut t.make[mi], name).on_hover_text(tr(
                                    "Make this object. Clear the box to keep its stitches as they are.",
                                ))
                            } else {
                                ui.label(name).on_hover_text(tr(
                                    "The program does not recognize these stitches: they stay stitches.",
                                ))
                            };
                            let detail = ui.label(crate::sequence_panel::kind_detail(&m.kind));
                            let now = if t.make[mi] && m.recognized() {
                                t.sewn[mi].len()
                            } else {
                                m.stitch_count()
                            };
                            let count = ui.label(fill("{} → {} stitches", &[&m.stitch_count(), &now]));
                            if first.hovered() || detail.hovered() || count.hovered() {
                                hovered = Some((ti, mi));
                            }
                            ui.end_row();
                        }
                    }
                });
        });
        d.hovered = hovered;
        ui.separator();
        ui.horizontal(|ui| {
            accept = ui
                .button(tr("Accept"))
                .on_hover_text(tr("Make the objects (one undo step)."))
                .clicked();
            if ui.button(tr("Recognize All")).clicked() {
                for t in &mut d.targets {
                    t.make = t.made.iter().map(Made::recognized).collect();
                }
            }
            if ui.button(tr("Keep All as Stitches")).clicked() {
                for t in &mut d.targets {
                    t.make.iter_mut().for_each(|m| *m = false);
                }
            }
            cancel = ui.button(tr("Cancel")).clicked();
        });
    });
    if let Some(p) = click {
        d.align_click(p);
    }
    if open_outlines {
        st.open_outlines_dialog(&mut d);
    }
    if accept {
        st.accept_recognized(d);
    } else if !(cancel || !open) {
        st.recognize_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    /// A design with one stitch object: the stitches of the sample design.
    fn stitch_harness() -> (Harness, ObjectId) {
        let mut h = Harness::new();
        let plan = h.st.plan_view().plan.clone();
        let d = emb_model::design_from_stitches(&plan.stitches, "sample");
        h.st.doc = emb_model::Document::new(d);
        let id = h.st.doc.design().objects()[0].id;
        (h, id)
    }

    #[test]
    fn recognize_objects_makes_the_objects_in_one_undo_step() {
        let (mut h, id) = stitch_harness();
        h.st.selection.set(id);
        h.st.run(crate::ribbon::Cmd::RecognizeObjects, &egui::Context::default());
        let d = h.st.recognize_dialog.take().expect("the window");
        let (file, objects) = d.counts();
        assert!(file > 0 && objects > 0);
        // The sample design has a fill, a satin and a run in one color: the
        // first color block becomes objects with stitch types.
        let kinds: Vec<&str> = d.targets[0].made.iter().map(|m| m.kind.type_name()).collect();
        assert!(kinds.iter().any(|k| *k != "Stitches"), "{kinds:?}");
        let before = h.st.doc.undo_len();
        h.st.accept_recognized(d);
        assert_eq!(h.st.doc.undo_len(), before + 1);
        assert_eq!(h.st.doc.undo_label(), Some("Recognize Objects"));
        assert!(h.st.doc.design().object(id).is_none());
        assert!(
            h.st.doc
                .design()
                .objects()
                .iter()
                .any(|o| !matches!(o.kind, ObjectKind::Stitches { .. }))
        );
    }

    #[test]
    fn kept_objects_stay_stitches_and_the_window_draws() {
        let (mut h, id) = stitch_harness();
        h.st.selection.set(id);
        h.st.open_recognize_objects();
        let mut d = h.st.recognize_dialog.take().expect("the window");
        for t in &mut d.targets {
            t.make.iter_mut().for_each(|m| *m = false);
        }
        // Kept all: the counts are the counts of the file, and nothing
        // differs.
        let (file, objects) = d.counts();
        assert_eq!(file, objects);
        let ctx = egui::Context::default();
        let p = d.pictures(&ctx).expect("pictures");
        assert!(p.differ.is_empty(), "{}", p.differ.len());
        let n = h.st.doc.design().objects().len();
        h.st.accept_recognized(d);
        // One stitch object for each stitch object: the same needle points.
        assert_eq!(h.st.doc.design().objects().len(), n);
        // The window draws.
        let (mut h, id) = stitch_harness();
        h.st.selection.set(id);
        h.st.open_recognize_objects();
        h.frame(Vec::new());
        assert!(h.st.recognize_dialog.is_some());
    }

    #[test]
    fn outlines_give_the_objects_their_shapes_and_two_points_align_them() {
        let mut h = Harness::new();
        // The outlines of the sample design, as Export Outlines writes them.
        let svg = emb_render::svg::outlines(h.st.doc.design()).expect("outlines");
        let outlines = outlines_of(&emb_formats::svg::import_svg_vectors(svg.as_bytes()).unwrap());
        assert!(!outlines.is_empty());
        let plan = h.st.plan_view().plan.clone();
        h.st.doc = emb_model::Document::new(emb_model::design_from_stitches(&plan.stitches, "sample"));
        let ids: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        h.st.selection.set_all(ids);
        h.st.open_recognize_objects();
        let mut d = h.st.recognize_dialog.take().expect("the window");
        d.set_outlines("sample.svg", outlines);
        let used = d.used_outlines();
        assert!(used > 0, "the outlines become objects");
        assert!(d.targets.iter().any(|t| t.from.iter().any(Option::is_some)));
        // Moved 6 mm off, fewer outlines fit; two points put them back.
        let o = d.outlines.as_ref().unwrap();
        let (t0, placed) = (o.transform, o.placed.clone());
        d.place_outlines(t0.then(Transform::translate(6.0, 4.0)));
        assert!(d.used_outlines() < used, "{} {used}", d.used_outlines());
        let corners = BBox::from_points(placed.iter().flat_map(|l| l.path.points().collect::<Vec<_>>())).unwrap();
        let shifted = |p: Point| p + Point::new(6.0, 4.0);
        // An outline point, then the same point of the stitches, two times.
        let a = placed
            .iter()
            .flat_map(|l| l.path.points().collect::<Vec<_>>())
            .min_by(|x, y| x.distance(corners.min).total_cmp(&y.distance(corners.min)))
            .unwrap();
        let b = placed
            .iter()
            .flat_map(|l| l.path.points().collect::<Vec<_>>())
            .min_by(|x, y| x.distance(corners.max).total_cmp(&y.distance(corners.max)))
            .unwrap();
        d.outlines.as_mut().unwrap().aligning = Some(Vec::new());
        for p in [shifted(a), a, shifted(b), b] {
            d.align_click(p);
        }
        assert!(d.outlines.as_ref().unwrap().aligning.is_none());
        assert!(d.used_outlines() + 1 >= used, "{} {used}", d.used_outlines());
        // The window draws with the outlines, and Accept makes the objects.
        let before = h.st.doc.undo_len();
        h.st.accept_recognized(d);
        assert_eq!(h.st.doc.undo_len(), before + 1);
        assert!(
            h.st.doc
                .design()
                .objects()
                .iter()
                .any(|o| matches!(o.kind, ObjectKind::Fill { .. } | ObjectKind::Satin { .. }))
        );
    }

    #[test]
    fn open_can_recognize_the_objects_at_once() {
        let (h, _) = stitch_harness();
        let mut d = h.st.doc.design().clone();
        let stitches = d.objects().len();
        let (made, recognized) = recognize_all(&mut d);
        assert!(recognized > 0 && made >= stitches);
        assert_eq!(d.objects().len(), made);
    }
}
