//! Picture to design (P6 Milestone 7, guided smart design; ADR 0029): the
//! steps of the Picture to design wizard after the background is removed,
//! as pure functions, so the app and the test corpus use the same code.
//!
//! 1. [`resample`]: the picture at the detail the user chooses (a pixel
//!    size in mm). Transparent pixels do not bleed their color into the
//!    edge.
//! 2. Color reduction (`reduce`), then [`merge_colors`]: the colors the
//!    user keeps, drops or merges into another color.
//! 3. [`digitize`]: each color area is traced (`autotrace`) and becomes
//!    embroidery as its color says ([`SmartStitch`]); the colors sew in
//!    the order given, and the areas of one color from the nearest one on.
//!    Fills grow by half the overlap amount, so areas that touch overlap
//!    and the fabric does not show between them when the stitches pull.
//!
//! P12 Milestone 2 (ADR 0053), in [`digitize`]:
//!
//! - **Layers.** A small hole of an area that only colors that sew later
//!   fill (a detail inside the area) is no hole: the area sews over it and
//!   the detail sews on top. So a fill does not break into parts round
//!   each detail, and no gap opens there when the stitches pull.
//! - **Fill angles.** Each fill sews along the long axis of its area (from
//!   the second moments of its area); a round area keeps the angle of the
//!   current fill settings. A fill that touches a fill made before it gets
//!   an angle at least [`ANGLE_APART_DEG`] from it, so their edge shows.
//! - **Small areas.** Auto makes an area smaller than `small_mm2` a satin,
//!   or a run along its middle if it is narrower than [`RUN_MAX_MM`]; a
//!   small fill (if satin is not possible, or the color is a fill) has no
//!   underlay. Specks smaller than the speck size of Simplify go into the
//!   color round them before this.
//!
//! P12 Milestone 3 (ADR 0054): **line art.** With Auto, a thin, long, dark
//! area (an outline of a drawing: at most [`LINE_MAX_MM`] wide, at least
//! [`LINE_RATIO`] times as long as wide, lightness at most [`LINE_DARK_L`])
//! is a run along its middle (narrower than [`LINE_RUN_MAX_MM`]) or a narrow
//! satin, with any number of holes. The lines sew after all areas, on top
//! of them; for the layers, they count as the last color. A long area of
//! any color narrower than [`LINE_RUN_MAX_MM`] is a run too, in its place.

use std::collections::HashMap;

use emb_geom::region::{self, Join, Region};
use emb_geom::{BBox, Path, Point, Transform, math};
use emb_model::{CurrentProperties, ObjectKind, Rgb, Underlay, VectorStyle};
use image::{Rgba, RgbaImage, imageops::FilterType};

use crate::autotrace::{self, Area, TraceSettings};
use crate::click::{self, ClickStitch};
use crate::reduce::{NONE, Reduced};

/// Tolerance to flatten an area outline, in mm.
const FLATTEN_MM: f64 = 0.05;
/// Auto tries a satin only on an area with this many holes or fewer (a
/// ring has one, a letter B two). An area of a noisy picture can have
/// hundreds of holes (specks of other colors): it is no column, and the
/// satin finder takes seconds on it.
const SATIN_MAX_HOLES: usize = 2;
/// Fills that touch sew at angles at least this far apart, in degrees.
pub const ANGLE_APART_DEG: f64 = 30.0;
/// A small area narrower than this is a run along its middle, in mm (a
/// satin column this narrow does not cover).
pub const RUN_MAX_MM: f64 = 0.8;
/// A line of line art narrower than this is a run, in mm; a wider one a
/// satin. A satin this narrow is wider than the drawn line (its pull
/// compensation and the thread), a run is near it.
pub const LINE_RUN_MAX_MM: f64 = 1.0;
/// A run of a line this wide or wider is a triple (bean) run, in mm.
pub const BEAN_MIN_MM: f64 = 0.6;
/// An area is long (it has a long axis) if it is this many times as long
/// as wide (as the square root of the ratio of its second moments).
const LONG_RATIO: f64 = 1.3;
/// The cells of the grid that finds the details in a hole, mm.
const DETAIL_CELL_MM: f64 = 5.0;
/// Line art: an area at most this wide, mm...
pub const LINE_MAX_MM: f64 = 2.0;
/// ...this many times as long as wide or more...
pub const LINE_RATIO: f64 = 8.0;
/// ...and as dark as this or darker (CIELAB lightness, 0 to 100).
pub const LINE_DARK_L: f64 = 40.0;

/// The wizard works on the picture at most this many pixels on its long
/// side, so each step stays quick (ADR 0029).
pub const WORK_PX: u32 = 1200;

/// The long side of a design from a picture is at most this, mm: a very
/// tall picture at a normal width made a design of hundreds of metres and
/// millions of stitches (audit of P11 and P12).
pub const MAX_SIDE_MM: f64 = 1000.0;

/// `width_mm` for a picture of `w` × `h` pixels, made smaller if the long
/// side of the design is more than [`MAX_SIDE_MM`].
pub fn fit_width(width_mm: f64, w: u32, h: u32) -> f64 {
    let (w, h) = (f64::from(w.max(1)), f64::from(h.max(1)));
    width_mm.min(MAX_SIDE_MM).min(MAX_SIDE_MM * w / h)
}

/// The picture the wizard works on: at most [`WORK_PX`] on its long side.
pub fn work_picture(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= WORK_PX {
        return img.clone();
    }
    // At least 1: a very tall, thin picture is made smaller too.
    resample(
        img,
        ((f64::from(w) * f64::from(WORK_PX) / f64::from(w.max(h))).round() as u32).max(1),
    )
}

/// The work picture of a picture file, as [`work_picture`] of the decoded
/// file gives it, with less memory (ADR 0077): the decoder uses at most
/// `max_bytes`, and the file is made smaller before it is converted.
pub fn decode_work(data: &[u8], max_bytes: u64) -> Result<RgbaImage, String> {
    let (mut img, smaller) = crate::decode_fit_info(data, WORK_PX, max_bytes, true)?;
    if smaller {
        // Each pixel clear or opaque, as `resample` makes them.
        for p in img.pixels_mut() {
            if p[3] >= 128 {
                p[3] = 255;
            } else {
                *p = Rgba([0, 0, 0, 0]);
            }
        }
    }
    Ok(img)
}

/// The settings of the Simplify step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimplifySpec {
    /// The size of one pixel of the simplified picture, in mm.
    pub pixel_mm: f64,
    pub colors: usize,
    /// Specks smaller than this take the color round them, in mm².
    pub speck_mm2: f64,
    pub clean_edges: bool,
    /// The radius of a median against noise, in pixels of the simplified
    /// picture (0: none).
    pub denoise: u32,
    /// Edge-preserving smoothing before the color reduction: the passes of
    /// `adjust::smooth` on the simplified picture (0: none).
    pub smooth: u32,
}

/// The size in pixels of the simplified picture of a picture `w` × `h`
/// pixels that is `width_mm` wide: pixels of `pixel_mm`, never more pixels
/// than the picture has, and at least 8 across.
pub fn detail_px(w: u32, h: u32, width_mm: f64, pixel_mm: f64) -> (u32, u32) {
    let target = (width_mm / pixel_mm.max(0.01)).round() as u32;
    let px = target.clamp(8.min(w), w.max(1));
    let ph = ((f64::from(h) * f64::from(px) / f64::from(w.max(1))).round() as u32).max(1);
    (px, ph)
}

/// The Simplify step: the picture after the background step (`content`,
/// `width_mm` wide) resampled to the detail, smoothed and color-reduced;
/// with `chart` (thread colors), to threads of the chart.
pub fn simplify(content: &RgbaImage, width_mm: f64, spec: &SimplifySpec, chart: Option<&[Rgb]>) -> Reduced {
    let (px, _) = detail_px(content.width(), content.height(), width_mm, spec.pixel_mm);
    let small = crate::adjust::smooth(
        &crate::adjust::median(&resample(content, px), spec.denoise),
        spec.smooth,
    );
    let px_mm = width_mm / f64::from(px.max(1));
    crate::reduce::reduce_with(
        &small,
        &crate::reduce::ReduceSpec {
            colors: spec.colors,
            min_speck_px: (spec.speck_mm2 / (px_mm * px_mm)).round() as usize,
            clean_edges: spec.clean_edges,
        },
        chart,
    )
}

/// The picture `width_px` pixels wide (the height follows). Colors are
/// averaged with their transparency (premultiplied), and each pixel ends
/// clear or opaque.
pub fn resample(img: &RgbaImage, width_px: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || width_px == 0 || width_px == w {
        return img.clone();
    }
    let height_px = ((f64::from(h) * f64::from(width_px) / f64::from(w)).round() as u32).max(1);
    let filter = if width_px < w {
        FilterType::Triangle
    } else {
        FilterType::Nearest
    };
    let pre = RgbaImage::from_fn(w, h, |x, y| {
        let p = img.get_pixel(x, y);
        let a = u16::from(p[3]);
        let m = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
        Rgba([m(p[0]), m(p[1]), m(p[2]), p[3]])
    });
    let mut out = image::imageops::resize(&pre, width_px, height_px, filter);
    for p in out.pixels_mut() {
        let a = u16::from(p[3]);
        if a >= 128 {
            for c in 0..3 {
                p[c] = ((u16::from(p[c]) * 255 + a / 2) / a).min(255) as u8;
            }
            p[3] = 255;
        } else {
            *p = Rgba([0, 0, 0, 0]);
        }
    }
    out
}

/// What a color of the picture becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartStitch {
    /// Satin for narrow areas, else fill.
    #[default]
    Auto,
    Fill,
    Satin,
    Outline,
    /// A vector shape with no stitches.
    Artwork,
}

impl SmartStitch {
    pub const ALL: [SmartStitch; 5] = [
        SmartStitch::Auto,
        SmartStitch::Fill,
        SmartStitch::Satin,
        SmartStitch::Outline,
        SmartStitch::Artwork,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SmartStitch::Auto => "Auto (satin if narrow)",
            SmartStitch::Fill => "Fill",
            SmartStitch::Satin => "Satin",
            SmartStitch::Outline => "Outline",
            SmartStitch::Artwork => "Artwork only (no stitches)",
        }
    }
}

/// The choice for one color of the reduced picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorChoice {
    /// `false`: the color is left out (as the background).
    pub keep: bool,
    /// The color takes the place of this one (an index in the palette).
    pub merge_into: Option<u16>,
    pub stitch: SmartStitch,
}

impl Default for ColorChoice {
    fn default() -> Self {
        ColorChoice {
            keep: true,
            merge_into: None,
            stitch: SmartStitch::Auto,
        }
    }
}

/// The color that each palette color becomes after the merges, or `None`
/// if it is left out. A chain of merges is followed; a loop stops.
pub fn final_colors(choices: &[ColorChoice]) -> Vec<Option<u16>> {
    let n = choices.len();
    (0..n)
        .map(|i| {
            let mut c = i;
            for _ in 0..n {
                match choices[c].merge_into {
                    Some(t) if (t as usize) < n && t as usize != c => c = t as usize,
                    _ => break,
                }
            }
            choices[c].keep.then_some(c as u16)
        })
        .collect()
}

/// The picture with the merges and the left-out colors of `choices` (the
/// palette stays, so the color indexes stay).
pub fn merge_colors(r: &Reduced, choices: &[ColorChoice]) -> Reduced {
    let to = final_colors(choices);
    Reduced {
        labels: r
            .labels
            .iter()
            .map(|&l| match to.get(l as usize) {
                Some(Some(c)) => *c,
                Some(None) => NONE,
                None => l,
            })
            .collect(),
        ..r.clone()
    }
}

/// Settings of [`digitize`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmartSpec {
    pub trace: TraceSettings,
    /// Fills grow by half of this, so areas that touch overlap by this, in mm.
    pub overlap_mm: f64,
    /// Auto makes a satin of an area narrower than this, in mm.
    pub satin_max_mm: f64,
    /// A detail inside an area up to this size sews on top of the area,
    /// which has no hole there, in mm². 0: each detail is a hole.
    pub layer_max_mm2: f64,
    /// Auto makes an area smaller than this a satin or a run, and a fill
    /// this small has no underlay, in mm².
    pub small_mm2: f64,
    /// Each fill along the long axis of its area, fills that touch
    /// [`ANGLE_APART_DEG`] apart; `false`: the angle of the current fill
    /// settings.
    pub fill_angles: bool,
    /// Auto sews thin, long, dark areas (outlines) as lines, last.
    pub line_art: bool,
}

impl Default for SmartSpec {
    fn default() -> Self {
        SmartSpec {
            trace: TraceSettings::default(),
            overlap_mm: 0.3,
            satin_max_mm: 5.0,
            layer_max_mm2: 25.0,
            small_mm2: 4.0,
            fill_angles: true,
            line_art: true,
        }
    }
}

/// One object of the design, with its palette color.
#[derive(Debug, Clone)]
pub struct SmartObject {
    pub color: u16,
    pub kind: ObjectKind,
}

/// The embroidery for the picture `r` (pixels to mm by `to_mm`): the colors
/// in `order` (palette indexes; colors not in it are left out), each as its
/// choice says.
pub fn digitize(
    r: &Reduced,
    to_mm: &Transform,
    choices: &[ColorChoice],
    order: &[u16],
    spec: &SmartSpec,
    current: &CurrentProperties,
) -> Vec<SmartObject> {
    let merged = merge_colors(r, choices);
    let (map, traced) = autotrace::trace_map(&merged, &spec.trace);
    let mm_per_px = to_mm.apply_vector(Point::new(1.0, 0.0)).distance(Point::ZERO);
    let areas: Vec<Area> = traced.iter().map(|a| a.transformed(to_mm)).collect();
    let rank: HashMap<u16, usize> = order.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let color_of = |c: u16| r.palette.get(c as usize).copied().unwrap_or(Rgb(0, 0, 0));
    let line: Vec<bool> = areas
        .iter()
        .map(|a| {
            spec.line_art
                && choices
                    .get(a.color as usize)
                    .is_some_and(|c| c.stitch == SmartStitch::Auto)
                && is_line(a, color_of(a.color), mm_per_px)
        })
        .collect();
    // The place of each area in the sew order: lines after all colors.
    let area_rank: Vec<Option<usize>> = areas
        .iter()
        .zip(&line)
        .map(|(a, &l)| rank.get(&a.color).map(|&k| if l { k + order.len() } else { k }))
        .collect();
    let over = layered_holes(&areas, &area_rank, mm_per_px, spec.layer_max_mm2);
    let mut near: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in map.touching() {
        near.entry(a).or_default().push(b);
        near.entry(b).or_default().push(a);
    }
    // The fill angle of each area that is a fill so far (by map index).
    let mut angles: HashMap<usize, f64> = HashMap::new();
    let mut out = Vec::new();
    let mut lines = Vec::new();
    let mut pen: Option<Point> = None;
    for &c in order {
        let Some(choice) = choices.get(c as usize) else {
            continue;
        };
        let mut todo: Vec<(usize, Point)> = areas
            .iter()
            .enumerate()
            .filter(|(_, a)| a.color == c)
            .filter_map(|(i, a)| Some((i, a.outer.control_bbox()?.center())))
            .collect();
        while !todo.is_empty() {
            let k = pen.map_or(0, |p| {
                (0..todo.len())
                    .min_by(|&i, &j| todo[i].1.distance(p).total_cmp(&todo[j].1.distance(p)))
                    .unwrap_or(0)
            });
            let (i, center) = todo.remove(k);
            pen = Some(center);
            let mut a = areas[i].clone();
            // The holes that details on top fill are sewn over.
            let keep: Vec<(Path, f64)> = std::mem::take(&mut a.holes)
                .into_iter()
                .zip(std::mem::take(&mut a.hole_px))
                .zip(&over[i])
                .filter(|(_, over)| !**over)
                .map(|(h, _)| h)
                .collect();
            (a.holes, a.hole_px) = keep.into_iter().unzip();
            let color = color_of(c);
            if line[i] {
                let kinds = if a.width_px * mm_per_px < LINE_RUN_MAX_MM {
                    center_run(&a, mm_per_px, current)
                } else {
                    click::objects(&a.outer, &a.holes, ClickStitch::Satin, current)
                };
                if !kinds.is_empty() {
                    lines.extend(kinds.into_iter().map(|kind| SmartObject { color: c, kind }));
                    continue;
                }
            }
            let mut kinds = area_objects(&a, choice.stitch, color, spec, mm_per_px, current);
            if spec.fill_angles {
                for kind in &mut kinds {
                    if let ObjectKind::Fill { params, .. } = kind {
                        let taken: Vec<f64> = near
                            .get(&a.index)
                            .into_iter()
                            .flatten()
                            .filter_map(|b| angles.get(b).copied())
                            .collect();
                        params.angle_deg = choose_angle(long_axis_deg(&a).unwrap_or(params.angle_deg), &taken);
                        angles.insert(a.index, params.angle_deg);
                    }
                }
            }
            for kind in kinds {
                out.push(SmartObject { color: c, kind });
            }
        }
    }
    out.extend(lines);
    out
}

/// A run along the middle of an area: a triple (bean) run if the area is
/// [`BEAN_MIN_MM`] wide or more, so the line is as wide as it is drawn.
fn center_run(a: &Area, mm_per_px: f64, current: &CurrentProperties) -> Vec<ObjectKind> {
    let mut kinds = click::objects(&a.outer, &a.holes, ClickStitch::CenterLine, current);
    if a.width_px * mm_per_px >= BEAN_MIN_MM {
        for k in &mut kinds {
            if let ObjectKind::Running { params, .. } = k {
                params.repeats = 3;
            }
        }
    }
    kinds
}

/// A long area narrower than [`LINE_RUN_MAX_MM`], of any color.
fn is_thin_line(a: &Area, mm_per_px: f64) -> bool {
    let width = a.width_px * mm_per_px;
    let length = a.pixels as f64 * mm_per_px * mm_per_px / width.max(1e-9);
    width < LINE_RUN_MAX_MM && length >= LINE_RATIO * width
}

/// Line art (see the module notes): a thin, long, dark area.
fn is_line(a: &Area, color: Rgb, mm_per_px: f64) -> bool {
    let width = a.width_px * mm_per_px;
    let length = a.pixels as f64 * mm_per_px * mm_per_px / width.max(1e-9);
    width <= LINE_MAX_MM && length >= LINE_RATIO * width && emb_model::Lab::of(color).l <= LINE_DARK_L
}

/// For each area (in mm) and each of its holes: `true` if the area sews
/// over the hole, because the hole is at most `max_mm2` and the areas in it
/// fill it (by their pixels) and sew later (by the `rank` of each area in the sew order;
/// `None`: left out). A hole with an area that sews earlier, that is left
/// out, or with clear pixels stays a hole.
fn layered_holes(areas: &[Area], rank: &[Option<usize>], mm_per_px: f64, max_mm2: f64) -> Vec<Vec<bool>> {
    let cell = |p: Point| {
        (
            (p.x / DETAIL_CELL_MM).floor() as i64,
            (p.y / DETAIL_CELL_MM).floor() as i64,
        )
    };
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, a) in areas.iter().enumerate() {
        grid.entry(cell(a.inside)).or_default().push(i);
    }
    let px_mm2 = mm_per_px * mm_per_px;
    areas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let Some(ra) = rank[i] else {
                return vec![false; a.holes.len()];
            };
            a.holes
                .iter()
                .zip(&a.hole_px)
                .map(|(h, &px)| {
                    let Some(ring) = h.flatten(FLATTEN_MM).into_iter().next().map(|p| p.points) else {
                        return false;
                    };
                    let hole_mm2 = px * px_mm2;
                    let Some(bb) = BBox::from_points(ring.iter().copied()) else {
                        return false;
                    };
                    if max_mm2 <= 0.0 || hole_mm2 > max_mm2 || hole_mm2 <= 0.0 {
                        return false;
                    }
                    let ((x0, y0), (x1, y1)) = (cell(bb.min), cell(bb.max));
                    let mut filled = 0.0;
                    for cx in x0..=x1 {
                        for cy in y0..=y1 {
                            for &b in grid.get(&(cx, cy)).into_iter().flatten() {
                                let d = &areas[b];
                                if !bb.contains(d.inside) || !region::ring_contains(&ring, d.inside) {
                                    continue;
                                }
                                if rank[b].is_none_or(|rb| rb <= ra) {
                                    return false;
                                }
                                filled += d.pixels as f64 * px_mm2;
                            }
                        }
                    }
                    // Areas under 4 pixels are not traced.
                    filled >= 0.95 * hole_mm2
                })
                .collect()
        })
        .collect()
}

/// The direction of the long axis of an area in mm, in degrees in
/// (-90, 90] (as the fill angle: 0 along X), or `None` if the area is about
/// as long as it is wide.
fn long_axis_deg(a: &Area) -> Option<f64> {
    let rings = rings(a);
    let o = *rings.first()?.first()?;
    // Area, first and second moments (outline +, holes -), about `o`.
    let (mut m, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, ring) in rings.iter().enumerate() {
        let signed = region::ring_signed_area(ring);
        let f = if (signed > 0.0) == (i == 0) { 1.0 } else { -1.0 };
        let n = ring.len();
        for k in 0..n {
            let (p, q) = (ring[k] - o, ring[(k + 1) % n] - o);
            let c = f * (p.x * q.y - q.x * p.y);
            m += c / 2.0;
            sx += (p.x + q.x) * c / 6.0;
            sy += (p.y + q.y) * c / 6.0;
            sxx += (p.x * p.x + p.x * q.x + q.x * q.x) * c / 12.0;
            syy += (p.y * p.y + p.y * q.y + q.y * q.y) * c / 12.0;
            sxy += (p.x * q.y + 2.0 * p.x * p.y + 2.0 * q.x * q.y + q.x * p.y) * c / 24.0;
        }
    }
    if m.abs() < 1e-9 {
        return None;
    }
    let (cx, cy) = (sx / m, sy / m);
    let (vxx, vyy, vxy) = (sxx / m - cx * cx, syy / m - cy * cy, sxy / m - cx * cy);
    let half = (vxx - vyy) / 2.0;
    let root = (half * half + vxy * vxy).sqrt();
    let (l1, l2) = ((vxx + vyy) / 2.0 + root, (vxx + vyy) / 2.0 - root);
    if l2 > 0.0 && l1 < LONG_RATIO * LONG_RATIO * l2 {
        return None;
    }
    Some(normal_deg(math::to_degrees(math::atan2(2.0 * vxy, vxx - vyy)) / 2.0))
}

/// An angle as a fill angle: in (-90, 90].
fn normal_deg(a: f64) -> f64 {
    let a = (a + 90.0).rem_euclid(180.0) - 90.0;
    if a <= -90.0 { a + 180.0 } else { a }
}

/// The fill angle nearest to `wanted` that is at least [`ANGLE_APART_DEG`]
/// from each angle of `taken` (fills that touch); if none is, the one
/// farthest from them.
fn choose_angle(wanted: f64, taken: &[f64]) -> f64 {
    let apart = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(180.0);
        d.min(180.0 - d)
    };
    let least = |a: f64| taken.iter().map(|&t| apart(a, t)).fold(f64::INFINITY, f64::min);
    let tries = [0.0, 30.0, -30.0, 45.0, -45.0, 60.0, -60.0, 90.0].map(|d| normal_deg(wanted + d));
    tries
        .iter()
        .copied()
        .find(|&a| least(a) >= ANGLE_APART_DEG - 1e-9)
        .unwrap_or_else(|| {
            tries
                .into_iter()
                .max_by(|&a, &b| least(a).total_cmp(&least(b)))
                .unwrap_or(wanted)
        })
}

/// The rings of an area in mm: the outline, then the holes.
fn rings(a: &Area) -> Vec<Vec<Point>> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .filter_map(|p| p.flatten(FLATTEN_MM).into_iter().next())
        .map(|pl| pl.points)
        .filter(|r| r.len() >= 3)
        .collect()
}

/// The mean width of an area (from the distance map of its pixels, so a
/// ragged edge does not make it narrow) and its length (its area over the
/// width), in mm; `mm_per_px` is the size of a picture pixel.
fn width_and_length(a: &Area, mm_per_px: f64) -> (f64, f64) {
    let width = a.width_px * mm_per_px;
    let area = a.pixels as f64 * mm_per_px * mm_per_px;
    if width <= 0.0 {
        return (0.0, 0.0);
    }
    (width, area / width)
}

fn area_objects(
    a: &Area,
    how: SmartStitch,
    color: Rgb,
    spec: &SmartSpec,
    mm_per_px: f64,
    current: &CurrentProperties,
) -> Vec<ObjectKind> {
    let satin = || click::objects(&a.outer, &a.holes, ClickStitch::Satin, current);
    let small = a.pixels as f64 * mm_per_px * mm_per_px < spec.small_mm2;
    // A thin, long line of any color: a satin this narrow sews wider than
    // the line, so a run along its middle.
    if how == SmartStitch::Auto && spec.line_art && is_thin_line(a, mm_per_px) {
        let run = center_run(a, mm_per_px, current);
        if !run.is_empty() {
            return run;
        }
    }
    if small && how == SmartStitch::Auto {
        let line = if a.width_px * mm_per_px < RUN_MAX_MM {
            center_run(a, mm_per_px, current)
        } else {
            satin()
        };
        if !line.is_empty() {
            return line;
        }
    }
    let mut kinds = area_kinds(a, how, color, spec, mm_per_px, current);
    // A small fill has no underlay: it would make a hard knot.
    if small {
        for k in &mut kinds {
            if let ObjectKind::Fill { params, .. } = k {
                params.underlay = Underlay::None;
            }
        }
    }
    kinds
}

/// The objects of an area as its stitch choice says.
fn area_kinds(
    a: &Area,
    how: SmartStitch,
    color: Rgb,
    spec: &SmartSpec,
    mm_per_px: f64,
    current: &CurrentProperties,
) -> Vec<ObjectKind> {
    let satin = || click::objects(&a.outer, &a.holes, ClickStitch::Satin, current);
    match how {
        SmartStitch::Artwork => vec![ObjectKind::Vector {
            path: a.path(),
            style: VectorStyle::filled(color),
        }],
        SmartStitch::Outline => click::objects(&a.outer, &a.holes, ClickStitch::Outline, current),
        SmartStitch::Fill => vec![grown_fill(a, spec.overlap_mm, current)],
        SmartStitch::Satin => {
            let s = satin();
            if s.is_empty() {
                vec![grown_fill(a, spec.overlap_mm, current)]
            } else {
                s
            }
        }
        SmartStitch::Auto => {
            let (width, length) = width_and_length(a, mm_per_px);
            let s = if width <= spec.satin_max_mm && length >= 3.0 * width && a.holes.len() <= SATIN_MAX_HOLES {
                satin()
            } else {
                Vec::new()
            };
            if s.is_empty() {
                vec![grown_fill(a, spec.overlap_mm, current)]
            } else {
                s
            }
        }
    }
}

/// A fill of the area, grown by half of `overlap_mm`.
fn grown_fill(a: &Area, overlap_mm: f64, current: &CurrentProperties) -> ObjectKind {
    let plain = || ObjectKind::Fill {
        outer: a.outer.clone(),
        holes: a.holes.clone(),
        params: current.fill.clone(),
    };
    if overlap_mm <= 0.0 {
        return plain();
    }
    let mut rs = rings(a).into_iter();
    let Some(outer) = rs.next() else { return plain() };
    let area = [Region {
        outer,
        holes: rs.collect(),
    }];
    let grown = region::offset(&area, overlap_mm / 2.0, Join::Round);
    if grown.is_empty() {
        return plain();
    }
    let outers: Vec<Path> = grown.iter().map(|g| Path::polygon(&g.outer)).collect();
    ObjectKind::Fill {
        outer: Path::joined(&outers),
        holes: grown
            .iter()
            .flat_map(|g| g.holes.iter().map(|h| Path::polygon(h)))
            .collect(),
        params: current.fill.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reduce::{ReduceSpec, reduce};

    /// White, with a big red disk and a thin blue bar.
    fn picture() -> RgbaImage {
        RgbaImage::from_fn(200, 100, |x, y| {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            if (fx - 50.0).hypot(fy - 50.0) < 40.0 {
                Rgba([200, 20, 30, 255])
            } else if (110..190).contains(&x) && (45..55).contains(&y) {
                Rgba([20, 40, 200, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        })
    }

    #[test]
    fn resample_keeps_clear_edges_clean() {
        // Red on a clear background whose hidden color is green.
        let img = RgbaImage::from_fn(40, 40, |x, _| {
            if x < 20 {
                Rgba([200, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 0])
            }
        });
        let small = resample(&img, 10);
        assert_eq!(small.dimensions(), (10, 10));
        for p in small.pixels() {
            assert!(p[3] == 0 || (p[3] == 255 && p[1] == 0), "{p:?}");
        }
        assert_eq!(resample(&img, 40), img);
    }

    #[test]
    fn merges_and_left_out_colors() {
        let c = |keep, merge_into| ColorChoice {
            keep,
            merge_into,
            ..ColorChoice::default()
        };
        // 0 kept, 1 into 2, 2 into 0, 3 left out, 4 into 3 (left out), 5 into itself.
        let choices = [
            c(true, None),
            c(true, Some(2)),
            c(true, Some(0)),
            c(false, None),
            c(true, Some(3)),
            c(true, Some(5)),
        ];
        assert_eq!(
            final_colors(&choices),
            vec![Some(0), Some(0), Some(0), None, None, Some(5)]
        );
        // A loop stops.
        let looped = [c(true, Some(1)), c(true, Some(0))];
        assert_eq!(final_colors(&looped).len(), 2);
    }

    #[test]
    fn a_picture_becomes_a_fill_and_a_satin_in_order() {
        let r = reduce(
            &picture(),
            &ReduceSpec {
                colors: 3,
                ..ReduceSpec::default()
            },
        );
        let white = r.palette.iter().position(|c| c.0 > 240 && c.1 > 240).unwrap() as u16;
        let red = r.palette.iter().position(|c| c.0 > 150 && c.1 < 80).unwrap() as u16;
        let blue = r.palette.iter().position(|c| c.2 > 150 && c.0 < 80).unwrap() as u16;
        let mut choices = vec![ColorChoice::default(); r.palette.len()];
        choices[white as usize].keep = false;
        // 0.25 mm a pixel: the disk is 20 mm, the bar 20 x 2.5 mm.
        let t = Transform::scale(0.25, 0.25);
        let spec = SmartSpec::default();
        let current = CurrentProperties::default();
        let objs = digitize(&r, &t, &choices, &[blue, red, white], &spec, &current);
        assert_eq!(objs.len(), 2, "{objs:?}");
        assert!(matches!(objs[0].kind, ObjectKind::Satin { .. }) && objs[0].color == blue);
        let ObjectKind::Fill { outer, .. } = &objs[1].kind else {
            panic!("{:?}", objs[1].kind)
        };
        // The fill grew by half the overlap on each side.
        let width = |p: &Path| {
            emb_geom::BBox::from_points(p.flatten(0.01).into_iter().flat_map(|pl| pl.points))
                .unwrap()
                .width()
        };
        let w = width(outer);
        let plain = SmartSpec {
            overlap_mm: 0.0,
            ..spec
        };
        let objs0 = digitize(&r, &t, &choices, &[blue, red, white], &plain, &current);
        let ObjectKind::Fill { outer: outer0, .. } = &objs0[1].kind else {
            panic!("{:?}", objs0[1].kind)
        };
        let w0 = width(outer0);
        assert!((w - w0 - 0.3).abs() < 0.02 && (w0 - 20.0).abs() < 0.6, "{w} {w0}");
        // Merged into the red, the bar is part of a red area; as artwork, a vector.
        choices[blue as usize].merge_into = Some(red);
        let objs = digitize(&r, &t, &choices, &[red], &spec, &current);
        assert!(objs.iter().all(|o| o.color == red));
        choices[red as usize].stitch = SmartStitch::Artwork;
        let objs = digitize(&r, &t, &choices, &[red], &spec, &current);
        assert!(objs.iter().all(|o| matches!(o.kind, ObjectKind::Vector { .. })));
    }

    /// A clear picture with flat areas: `f` gives the color of a pixel.
    fn flat(w: u32, h: u32, f: impl Fn(u32, u32) -> Option<[u8; 3]>) -> Reduced {
        let img = RgbaImage::from_fn(w, h, |x, y| match f(x, y) {
            Some(c) => Rgba([c[0], c[1], c[2], 255]),
            None => Rgba([0, 0, 0, 0]),
        });
        crate::reduce::exact(&img, 8).unwrap()
    }

    fn holes(k: &ObjectKind) -> usize {
        match k {
            ObjectKind::Fill { holes, .. } => holes.len(),
            _ => usize::MAX,
        }
    }

    #[test]
    fn small_details_sew_on_top_and_large_ones_stay_holes() {
        // 0.25 mm a pixel: a blue box 50 x 25 mm with a red square of 4 mm
        // (16 mm²) and a red square of 8 mm (64 mm²) in it.
        let r = flat(200, 100, |x, y| {
            let red =
                ((30..46).contains(&x) && (40..56).contains(&y)) || ((120..152).contains(&x) && (30..62).contains(&y));
            Some(if red { [200, 20, 20] } else { [20, 40, 200] })
        });
        let blue = r.palette.iter().position(|c| c.2 > 150).unwrap() as u16;
        let red = 1 - blue;
        let t = Transform::scale(0.25, 0.25);
        let choices = vec![ColorChoice::default(); 2];
        let current = CurrentProperties::default();
        let spec = SmartSpec::default();
        // Blue first: the small red square sews on it; the large one is a hole.
        let objs = digitize(&r, &t, &choices, &[blue, red], &spec, &current);
        assert_eq!(holes(&objs[0].kind), 1, "{:?}", objs[0].kind);
        // With no layers: two holes.
        let none = SmartSpec {
            layer_max_mm2: 0.0,
            ..spec
        };
        assert_eq!(
            holes(&digitize(&r, &t, &choices, &[blue, red], &none, &current)[0].kind),
            2
        );
        // Red first: a detail that sews before the area stays a hole.
        let objs = digitize(&r, &t, &choices, &[red, blue], &spec, &current);
        let b = objs.iter().find(|o| o.color == blue).unwrap();
        assert_eq!(holes(&b.kind), 2);
        // A clear hole (no color in it) stays a hole.
        let r = flat(200, 100, |x, y| {
            (!((30..46).contains(&x) && (40..56).contains(&y))).then_some([20, 40, 200])
        });
        let objs = digitize(&r, &t, &[ColorChoice::default()], &[0], &spec, &current);
        assert_eq!(holes(&objs[0].kind), 1);
    }

    fn angle(k: &ObjectKind) -> f64 {
        match k {
            ObjectKind::Fill { params, .. } => params.angle_deg,
            _ => panic!("{k:?}"),
        }
    }

    #[test]
    fn fills_sew_along_their_long_axis_and_touching_fills_differ() {
        let t = Transform::scale(0.25, 0.25);
        let current = CurrentProperties::default();
        let spec = SmartSpec::default();
        // A wide box (40 x 10 mm) and a tall one (10 x 20 mm) apart.
        let r = flat(300, 100, |x, y| {
            if (10..170).contains(&x) && (30..70).contains(&y) {
                Some([200, 20, 20])
            } else if (220..260).contains(&x) && (10..90).contains(&y) {
                Some([20, 40, 200])
            } else {
                None
            }
        });
        let choices = vec![
            ColorChoice {
                stitch: SmartStitch::Fill,
                ..ColorChoice::default()
            };
            2
        ];
        let objs = digitize(&r, &t, &choices, &[0, 1], &spec, &current);
        let red = objs.iter().find(|o| r.palette[o.color as usize].0 > 150).unwrap();
        let blue = objs.iter().find(|o| r.palette[o.color as usize].2 > 150).unwrap();
        assert!(angle(&red.kind).abs() < 1.0, "{}", angle(&red.kind));
        assert!((angle(&blue.kind) - 90.0).abs() < 1.0, "{}", angle(&blue.kind));
        // Two wide boxes that touch: the second is 30° or more from the first.
        let r = flat(200, 100, |x, y| {
            if !(10..190).contains(&x) || !(20..80).contains(&y) {
                None
            } else if y < 50 {
                Some([200, 20, 20])
            } else {
                Some([20, 40, 200])
            }
        });
        let objs = digitize(&r, &t, &choices, &[0, 1], &spec, &current);
        let (a, b) = (angle(&objs[0].kind), angle(&objs[1].kind));
        assert!(a.abs() < 1.0 && (b - a).abs() >= ANGLE_APART_DEG - 1e-6, "{a} {b}");
        // Off: the angle of the current fill settings.
        let off = SmartSpec {
            fill_angles: false,
            ..spec
        };
        let objs = digitize(&r, &t, &choices, &[0, 1], &off, &current);
        assert!(objs.iter().all(|o| angle(&o.kind) == current.fill.angle_deg));
        assert_eq!(choose_angle(10.0, &[0.0]), 40.0);
        assert_eq!(normal_deg(-90.0), 90.0);
        assert_eq!(normal_deg(135.0), -45.0);
    }

    #[test]
    fn small_areas_are_satins_or_runs_and_small_fills_have_no_underlay() {
        let t = Transform::scale(0.25, 0.25);
        let current = CurrentProperties::default();
        let spec = SmartSpec::default();
        // A dot of 1.5 x 1.5 mm and a hair line 0.5 mm wide, 6 mm long.
        let r = flat(100, 60, |x, y| {
            if (10..16).contains(&x) && (10..16).contains(&y) {
                Some([200, 20, 20])
            } else if (40..64).contains(&x) && (30..32).contains(&y) {
                Some([20, 40, 200])
            } else {
                None
            }
        });
        let choices = vec![ColorChoice::default(); 2];
        let objs = digitize(&r, &t, &choices, &[0, 1], &spec, &current);
        let red = objs.iter().find(|o| r.palette[o.color as usize].0 > 150).unwrap();
        let blue = objs.iter().find(|o| r.palette[o.color as usize].2 > 150).unwrap();
        assert!(matches!(red.kind, ObjectKind::Satin { .. }), "{:?}", red.kind);
        assert!(matches!(blue.kind, ObjectKind::Running { .. }), "{:?}", blue.kind);
        // As fills, they have no underlay.
        let fills = vec![
            ColorChoice {
                stitch: SmartStitch::Fill,
                ..ColorChoice::default()
            };
            2
        ];
        for o in digitize(&r, &t, &fills, &[0, 1], &spec, &current) {
            let ObjectKind::Fill { params, .. } = &o.kind else {
                panic!("{:?}", o.kind)
            };
            assert_eq!(params.underlay, Underlay::None);
        }
    }

    #[test]
    fn line_art_is_lines_on_top_of_the_areas() {
        // 0.25 mm a pixel: a black outline 6 px (1.5 mm) wide round a yellow
        // box, a black line 2 px (0.5 mm) wide in the box, and a light blue
        // bar as wide as the outline.
        let r = flat(240, 120, |x, y| {
            let in_box = (10..150).contains(&x) && (10..110).contains(&y);
            let edge = in_box && (!(16..144).contains(&x) || !(16..104).contains(&y));
            if edge || ((40..120).contains(&x) && (60..62).contains(&y)) {
                Some([10, 10, 10])
            } else if in_box {
                Some([250, 210, 60])
            } else if (170..230).contains(&x) && (50..56).contains(&y) {
                Some([150, 200, 250])
            } else {
                None
            }
        });
        let black = r.palette.iter().position(|c| c.0 < 50).unwrap() as u16;
        let yellow = r.palette.iter().position(|c| c.2 < 100).unwrap() as u16;
        let blue = r.palette.iter().position(|c| c.2 > 200).unwrap() as u16;
        let t = Transform::scale(0.25, 0.25);
        let choices = vec![ColorChoice::default(); 3];
        let current = CurrentProperties::default();
        let spec = SmartSpec::default();
        // Black first in the list: the lines still sew last.
        let objs = digitize(&r, &t, &choices, &[black, yellow, blue], &spec, &current);
        let last_area = objs.iter().rposition(|o| o.color != black).unwrap();
        let first_line = objs.iter().position(|o| o.color == black).unwrap();
        assert!(first_line > last_area, "{objs:?}");
        // The outline is a satin, the thin line a run.
        let lines: Vec<&ObjectKind> = objs.iter().filter(|o| o.color == black).map(|o| &o.kind).collect();
        assert!(lines.iter().any(|k| matches!(k, ObjectKind::Satin { .. })), "{lines:?}");
        assert!(
            lines.iter().any(|k| matches!(k, ObjectKind::Running { .. })),
            "{lines:?}"
        );
        // The yellow sews under the thin line (no hole for it); the light
        // bar is no line: a satin before the lines.
        let y = objs.iter().find(|o| o.color == yellow).unwrap();
        assert_eq!(holes(&y.kind), 0);
        let bar = objs.iter().position(|o| o.color == blue).unwrap();
        assert!(matches!(objs[bar].kind, ObjectKind::Satin { .. }) && bar < first_line);
        // A thin line of any color (under 1 mm) is a run in its place;
        // 0.75 mm wide, a triple run.
        let thin = flat(100, 40, |x, y| {
            ((10..90).contains(&x) && (18..21).contains(&y)).then_some([150, 200, 250])
        });
        let objs = digitize(&thin, &t, &[ColorChoice::default()], &[0], &spec, &current);
        let ObjectKind::Running { params, .. } = &objs[0].kind else {
            panic!("{:?}", objs[0].kind)
        };
        assert_eq!(params.repeats, 3);
        // Off: the outline sews as an area in its place in the list.
        let off = SmartSpec {
            line_art: false,
            ..spec
        };
        let objs = digitize(&r, &t, &choices, &[black, yellow, blue], &off, &current);
        assert_eq!(objs[0].color, black);
    }

    #[test]
    fn the_long_side_of_a_design_is_at_most_a_metre() {
        // A strip 1 x 5000 pixels, 100 mm wide, would be 500 m tall.
        let w = fit_width(100.0, 1, 5000);
        assert!((w * 5000.0 - MAX_SIDE_MM).abs() < 1e-6, "{w}");
        assert_eq!(fit_width(100.0, 400, 300), 100.0);
        assert_eq!(fit_width(5000.0, 400, 300), MAX_SIDE_MM);
    }
}
