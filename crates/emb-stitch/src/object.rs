//! Stitches for one object, and the cache keyed by `(ObjectId, revision)`.
//!
//! Fill and satin objects start where the needle is (or at the start point
//! that the user set). For them the cache key also has the needle position,
//! rounded to [`ENTRY_GRID_MM`]. The object is stitched from the rounded
//! position, so the result is a function of the key only.

use std::collections::HashMap;
use std::sync::Arc;

use emb_geom::{Point, Polyline, text};
use emb_model::{
    DesignObject, FillParams, FillPattern, LineBorder, ObjectId, ObjectKind, RunPass, RunStyle, SatinGeometry,
    Underlay, UnderlayLayer,
};

use crate::fill::{Tatami, sew_tatami};
use crate::network;
use crate::running;
use crate::satin;
use crate::travel::{Sewer, Shape, Travel};

/// Tolerance to flatten object geometry for stitching, in mm.
pub const FLATTEN_TOL_MM: f64 = 0.05;
/// The needle position for fill and satin objects is rounded to this grid.
pub const ENTRY_GRID_MM: f64 = 1.0;
/// Rows until the tatami needle point pattern repeats.
pub const FILL_STAGGERS: u32 = 4;
/// Stitch length of travel inside a fill.
pub const TRAVEL_LEN_MM: f64 = 3.0;
/// `Auto` fill underlay: tatami at 90° to the top rows, this far inside the edge.
const AUTO_FILL_INSET_MM: f64 = 0.5;
const AUTO_FILL_SPACING_MM: f64 = 2.0;
/// `Heavy` fill underlay: row spacing of the second tatami layer.
const HEAVY_FILL_SPACING_MM: f64 = 3.0;
/// `Auto` fill underlay is only for shapes larger than this, in mm².
const AUTO_FILL_MIN_AREA: f64 = 20.0;
/// Stitch length of fill underlay and edge walk.
const UNDERLAY_LEN_MM: f64 = 3.0;

/// One continuous line of needle points, before routing.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub points: Vec<Point>,
    /// The first point is also the last point. The plan can start it at any of its points.
    pub closed: bool,
    /// Bean-stitch repeats, applied after the plan chose the direction.
    pub repeats: u32,
    /// The order is fixed (fill, satin, network): the plan does not reverse
    /// the run or start it at another point.
    pub fixed: bool,
    /// Backstitch and stem stitch: `points` are at half the stitch length,
    /// and the plan makes the pattern after it chose the direction.
    pub style: RunStyle,
    /// Backtrack or repeat, applied after the direction is chosen.
    pub pass: RunPass,
    /// How the run joins the run before it in the same object.
    pub link: RunLink,
    /// Keep every needle point, also stitches shorter than the machine
    /// minimum (manual stitches, stitches from a machine file).
    pub exact: bool,
    /// The machine stops before the run (appliqué, cording).
    pub stop: Option<StopReason>,
    /// Auto jump: stitches longer than the machine allows are sewn as
    /// jumps, not split with needle points.
    pub long_jumps: bool,
    /// The stitches of the run that are travel: the needle goes from one
    /// part of the object to the next, and the stitches are not a part of
    /// its look. Ranges of segments: segment `k` goes from `points[k]` to
    /// `points[k + 1]`. Empty for most runs. The plan keeps them in
    /// `StitchPlan::travel` (P22 Milestone 1).
    pub travel: Vec<std::ops::Range<usize>>,
}

/// Why the machine stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The stop function of an object.
    Function,
    /// Put the appliqué fabric on.
    PlaceFabric,
    /// Cut away the appliqué fabric outside the tack-down.
    TrimFabric,
    /// Lay the cord along the line (cording).
    PlaceCord,
    /// Put the foam on for puff columns (P11 Milestone 7).
    PlaceFoam,
}

impl StopReason {
    /// What the operator does at the stop.
    pub fn text(self) -> &'static str {
        match self {
            StopReason::Function => text::tr("Stop"),
            StopReason::PlaceFabric => text::tr("Put the appliqué fabric on"),
            StopReason::TrimFabric => text::tr("Cut away the appliqué fabric outside the tack-down"),
            StopReason::PlaceCord => text::tr("Lay the cord along the sewn line"),
            StopReason::PlaceFoam => text::tr("Put the foam on (puff); tear the foam away after the puff objects"),
        }
    }
}

/// How a run joins the run before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunLink {
    /// A travel stitch when the gap is short, else a trim.
    #[default]
    Auto,
    /// A jump: the thread is not cut and the needle does not go in between.
    Jump,
    /// A trim.
    Trim,
    /// A stitch, also over a long gap (the stitches after it cover it).
    Travel,
}

impl Run {
    /// A run with a fixed order (fill, satin) and the default settings.
    pub fn new(points: Vec<Point>) -> Run {
        Run {
            points,
            closed: false,
            repeats: 1,
            fixed: true,
            style: RunStyle::Run,
            pass: RunPass::Once,
            link: RunLink::Auto,
            exact: false,
            stop: None,
            long_jumps: false,
            travel: Vec::new(),
        }
    }
}

/// The subpaths of a stitch object as needle points (one function for the
/// stitch engine and for the records of stitch objects).
pub use emb_model::stitch_subpaths;

/// The stitches of one object: one or more runs in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectStitches {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StitchError {
    #[error("{}", text::tr("the object has no stitchable geometry"))]
    Empty,
    #[error(
        "{}",
        text::tr("the object is too large to sew (more than 3 m, or a setting that makes it so large)")
    )]
    TooLarge,
}

/// The largest object that the program sews: larger than any hoop and any
/// layout of hoopings. A larger object comes from a damaged file or a wrong
/// value, and its stitches would fill all the memory.
pub const MAX_OBJECT_MM: f64 = 3000.0;

/// The largest percent setting (pull compensation as a percent of the width).
const MAX_PERCENT: f64 = 1000.0;

/// True if the object, with its settings, is larger than [`MAX_OBJECT_MM`].
/// The settings are each distance (a field that ends in `_mm`: pull
/// compensation, the cover of appliqué, the size of a motif, the hand
/// effect, …), each percent, and the points in the settings (guide lines,
/// direction lines). So a new setting is in the check too.
pub fn too_large(obj: &DesignObject) -> bool {
    let Some(b) = obj.control_bbox() else {
        return false;
    };
    let params = match &obj.kind {
        ObjectKind::Running { params, .. } => serde_json::to_value(params),
        ObjectKind::Satin { params, .. } => serde_json::to_value(params),
        ObjectKind::Fill { params, .. } => serde_json::to_value(params),
        ObjectKind::Applique { params, .. } => serde_json::to_value(params),
        // The settings of the lettering and of each letter, but not the
        // sizes that the letters have already: the height, the spacing, the
        // kerning and the underline are in the outline of the letters, and
        // the trim length is not a size. With them, lettering 1 m high was
        // refused when it was wider than 1 m (the audit of P20).
        ObjectKind::Lettering(l) => serde_json::to_value(l).map(|mut v| {
            if let Some(m) = v.as_object_mut() {
                for key in ["height_mm", "letter_spacing_mm", "trim_over_mm", "underline"] {
                    m.remove(key);
                }
                for edit in m.get_mut("edits").and_then(|e| e.as_array_mut()).into_iter().flatten() {
                    if let Some(e) = edit.as_object_mut() {
                        e.remove("kern_mm");
                        if let Some(shape) = e.get_mut("shape").and_then(|s| s.as_object_mut()) {
                            shape.remove("height_mm");
                        }
                    }
                }
            }
            v
        }),
        _ => Ok(serde_json::Value::Null),
    };
    let mut l = Largest::default();
    if let Ok(v) = params {
        l.add(&v, None);
    }
    let size = b.width().max(b.height()) + 2.0 * l.mm;
    // A NaN setting is `null` in JSON; a NaN size is a wrong coordinate.
    size.is_nan() || size > MAX_OBJECT_MM || l.percent > MAX_PERCENT || l.point > emb_model::project::MAX_COORD_MM
}

/// The largest absolute values in the settings: of a distance (`…_mm`), of
/// a percent (`…_percent`) and of a number in a list (the coordinates of
/// points).
#[derive(Default)]
struct Largest {
    mm: f64,
    percent: f64,
    point: f64,
}

impl Largest {
    fn add(&mut self, v: &serde_json::Value, key: Option<&str>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    self.add(x, Some(k));
                }
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    self.add(x, None);
                }
            }
            serde_json::Value::Number(n) => {
                let n = n.as_f64().unwrap_or(0.0).abs();
                match key {
                    Some(k) if k.ends_with("_mm") => self.mm = self.mm.max(n),
                    Some(k) if k.ends_with("_percent") => self.percent = self.percent.max(n),
                    Some(_) => {}
                    None => self.point = self.point.max(n),
                }
            }
            _ => {}
        }
    }
}

/// The needle position that a fill or satin object starts from: the start
/// point that the user set, else `needle` rounded to [`ENTRY_GRID_MM`].
/// `None` for objects whose stitches do not depend on it.
pub fn entry_for(obj: &DesignObject, needle: Point) -> Option<Point> {
    match &obj.kind {
        ObjectKind::Running { params, .. }
            if params.border.is_none() && !matches!(params.pass, RunPass::Redwork | RunPass::Branch) =>
        {
            None
        }
        ObjectKind::Stitches { .. } | ObjectKind::Vector { .. } | ObjectKind::Bling(_) => None,
        _ => Some(obj.start.unwrap_or_else(|| {
            let g = ENTRY_GRID_MM;
            Point::new((needle.x / g).round() * g, (needle.y / g).round() * g)
        })),
    }
}

/// Stitches one object. A pure, deterministic function of the object and
/// `entry` (see [`entry_for`]; running objects do not use it).
pub fn stitch_object(obj: &DesignObject, entry: Point) -> Result<ObjectStitches, StitchError> {
    if too_large(obj) {
        return Err(StitchError::TooLarge);
    }
    let mut s = stitch_kind(&obj.kind, entry, obj.end)?;
    let hand = match &obj.kind {
        ObjectKind::Running { params, .. } => params.hand_mm,
        ObjectKind::Satin { params, .. } => params.hand_mm,
        ObjectKind::Fill { params, .. } => params.hand_mm,
        _ => 0.0,
    };
    for (i, r) in s.runs.iter_mut().enumerate() {
        crate::effects::hand(&mut r.points, hand, HAND_SEED + i as u64);
    }
    Ok(s)
}

/// The object sews over foam (puff, ADR 0050): a puff satin column, a line
/// with a puff border, or lettering with puff satin.
pub fn is_puff(kind: &ObjectKind) -> bool {
    match kind {
        ObjectKind::Satin { params, .. } => params.puff,
        ObjectKind::Running { params, .. } => params.border.as_ref().is_some_and(|b| b.column.puff),
        ObjectKind::Lettering(l) => l.satin.puff,
        _ => false,
    }
}

/// Seed of the hand-stitch effect.
const HAND_SEED: u64 = 0x48414E44;

/// Stitches one object kind from `entry`, ending near `exit` if it is set.
fn stitch_kind(kind: &ObjectKind, entry: Point, exit: Option<Point>) -> Result<ObjectStitches, StitchError> {
    match kind {
        // Artwork and rhinestones sew nothing (ADR 0024, 0039).
        ObjectKind::Vector { .. } | ObjectKind::Bling(_) | ObjectKind::Sketch(_) => Ok(ObjectStitches::default()),
        ObjectKind::Running { path, params } => {
            let lines = path.flatten(FLATTEN_TOL_MM);
            let runs: Vec<Run> = if params.cording {
                cording_runs(&lines, params.stitch_length_mm)
            } else if let Some(r) = &params.ripple {
                lines
                    .iter()
                    .map(|pl| {
                        let line = Polyline {
                            points: crate::ripple::ripple_line(pl, r),
                            closed: false,
                        };
                        let style = if params.style == RunStyle::Manual {
                            RunStyle::Run
                        } else {
                            params.style
                        };
                        let len = match style {
                            RunStyle::Backstitch | RunStyle::Stemstitch => params.stitch_length_mm / 2.0,
                            _ => params.stitch_length_mm,
                        };
                        Run {
                            repeats: params.repeats,
                            style,
                            fixed: false,
                            ..Run::new(running::running_points(&line, len))
                        }
                    })
                    .collect()
            } else if let Some(m) = &params.motif {
                lines
                    .iter()
                    .map(|pl| Run {
                        closed: pl.closed,
                        fixed: false,
                        ..Run::new(crate::motifs::run(pl, m, params.stitch_length_mm))
                    })
                    .collect()
            } else if let Some(border) = &params.border {
                border_runs(&lines, border, entry, exit)
            } else if matches!(params.pass, RunPass::Redwork | RunPass::Branch) {
                let branch = params.pass == RunPass::Branch;
                network::network_runs(&lines, entry, params.stitch_length_mm, branch)
                    .into_iter()
                    .map(Run::new)
                    .collect()
            } else {
                lines
                    .iter()
                    .map(|pl| {
                        let points = match params.style {
                            RunStyle::Run | RunStyle::Sculpture => running::running_points(pl, params.stitch_length_mm),
                            RunStyle::Backstitch | RunStyle::Stemstitch => {
                                running::running_points(pl, params.stitch_length_mm / 2.0)
                            }
                            RunStyle::Manual => {
                                let mut p = pl.points.clone();
                                p.dedup();
                                if pl.closed && p.len() > 2 {
                                    p.push(p[0]);
                                }
                                p
                            }
                        };
                        Run {
                            points,
                            closed: pl.closed,
                            repeats: params.repeats,
                            fixed: false,
                            style: params.style,
                            pass: params.pass,
                            link: RunLink::Auto,
                            exact: params.style == RunStyle::Manual,
                            stop: None,
                            long_jumps: false,
                            travel: Vec::new(),
                        }
                    })
                    .collect()
            };
            let mut runs: Vec<Run> = runs.into_iter().filter(|r| r.points.len() >= 2).collect();
            // Cut lines (cutwork needles): a trim between the parts, so the
            // needle lifts in each file format (VP3 has no jump of its own,
            // and a stitch there would cut the fabric).
            if params.cut {
                for (i, r) in runs.iter_mut().enumerate() {
                    if i > 0 {
                        r.link = RunLink::Trim;
                    }
                    r.repeats = 1;
                }
            }
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches { runs })
            }
        }
        ObjectKind::Fill { outer, holes, params } => {
            let outers = outer.flatten(FLATTEN_TOL_MM);
            let holes: Vec<Polyline> = holes.iter().flat_map(|h| h.flatten(FLATTEN_TOL_MM)).collect();
            let runs = fill_runs(&Shape::new(&outers, &holes), params, entry, exit);
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches {
                    runs: runs
                        .into_iter()
                        .map(|(points, travel)| Run {
                            travel,
                            ..Run::new(points)
                        })
                        .collect(),
                })
            }
        }
        ObjectKind::Satin { geometry, params } => match satin::satin(geometry, params, entry, exit) {
            Some(points) if points.len() >= 2 => Ok(ObjectStitches {
                runs: vec![Run {
                    long_jumps: params.auto_jump,
                    ..Run::new(points)
                }],
            }),
            _ => Err(StitchError::Empty),
        },
        ObjectKind::Lettering(l) => {
            let runs = lettering_runs(l, entry, exit);
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches { runs })
            }
        }
        ObjectKind::Applique { pieces, params } => {
            let runs = crate::applique::applique_runs(pieces, params, entry);
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches { runs })
            }
        }
        ObjectKind::CrossStitch(x) => {
            let runs = crate::cross::cross_runs(x, entry);
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches { runs })
            }
        }
        ObjectKind::Stitches { path, trims } => {
            let runs: Vec<Run> = stitch_subpaths(path)
                .into_iter()
                .enumerate()
                .map(|(i, mut points)| {
                    // One needle point: a stitch in place.
                    if points.len() == 1 {
                        points.push(points[0]);
                    }
                    Run {
                        style: RunStyle::Manual,
                        link: match i {
                            0 => RunLink::Auto,
                            i if trims.contains(&i) => RunLink::Trim,
                            _ => RunLink::Jump,
                        },
                        exact: true,
                        ..Run::new(points)
                    }
                })
                .collect();
            if runs.is_empty() {
                Err(StitchError::Empty)
            } else {
                Ok(ObjectStitches { runs })
            }
        }
    }
}

/// Cording: for each part of the line, a run along it, then a trim and a
/// stop to lay the cord, then an open zigzag over the cord.
fn cording_runs(lines: &[Polyline], stitch_len: f64) -> Vec<Run> {
    let zigzag = emb_model::SatinParams {
        spacing_mm: crate::effects::CORD_SPACING_MM,
        pull_compensation_mm: 0.0,
        underlay: Underlay::None,
        ..emb_model::SatinParams::with_pattern(emb_model::ColumnPattern::Zigzag)
    };
    let mut place = Vec::new();
    let mut cover = Vec::new();
    for pl in lines {
        let pts = running::running_points(pl, stitch_len);
        if pts.len() < 2 {
            continue;
        }
        let end = pts[pts.len() - 1];
        place.push(Run {
            closed: pl.closed,
            fixed: false,
            ..Run::new(pts)
        });
        let Some(path) = path_of_polyline(pl) else { continue };
        let g = SatinGeometry::from_centerline(&path, crate::effects::CORD_ZIGZAG_MM);
        if let Some(z) = satin::satin(&g, &zigzag, end, None) {
            cover.push(Run::new(z));
        }
    }
    // A trim, then the stop: the operator lays the cord, and the zigzag
    // starts with lock stitches.
    if let Some(first) = cover.first_mut() {
        first.stop = Some(StopReason::PlaceCord);
        first.link = RunLink::Trim;
    }
    place.into_iter().chain(cover).collect()
}

/// A path through the points of a polyline (closed if it is closed).
fn path_of_polyline(pl: &Polyline) -> Option<emb_geom::Path> {
    let (&first, rest) = pl.points.split_first()?;
    let path = rest
        .iter()
        .fold(emb_geom::Path::new().move_to(first), |p, q| p.line_to(*q));
    Some(if pl.closed { path.close() } else { path })
}

/// A line with a border: each subpath sews as a column of the border width
/// along it, from the end nearer to the needle. Cap corners cut the outer
/// edge straight; lap corners sew each part between corners as a column of
/// its own, longer by half the width so the parts overlap.
fn border_runs(lines: &[Polyline], border: &LineBorder, entry: Point, exit: Option<Point>) -> Vec<Run> {
    let mut parts: Vec<emb_geom::Path> = Vec::new();
    for pl in lines {
        if border.corner == emb_model::Corner::Lap {
            for part in crate::effects::lap_parts(pl, border.width_mm / 2.0) {
                let line = Polyline {
                    points: part,
                    closed: false,
                };
                parts.extend(path_of_polyline(&line));
            }
        } else {
            parts.extend(path_of_polyline(pl));
        }
    }
    let cap = border.corner == emb_model::Corner::Cap;
    let mut needle = entry;
    let mut runs = Vec::new();
    for (i, path) in parts.iter().enumerate() {
        let geometry = if cap {
            SatinGeometry::from_centerline_capped(path, border.width_mm)
        } else {
            SatinGeometry::from_centerline(path, border.width_mm)
        };
        let last = i + 1 == parts.len();
        if let Some(points) = satin::satin(&geometry, &border.column, needle, if last { exit } else { None })
            && points.len() >= 2
        {
            needle = points[points.len() - 1];
            runs.push(Run {
                long_jumps: border.column.auto_jump,
                ..Run::new(points)
            });
        }
    }
    runs
}

/// The box of the parts of a letter.
fn letter_box(l: &emb_model::Letter) -> Option<emb_geom::BBox> {
    l.parts
        .iter()
        .filter_map(|p| p.control_bbox())
        .reduce(emb_geom::BBox::union)
}

/// The point of `b` nearest `p`.
fn nearest_in(b: &emb_geom::BBox, p: Point) -> Point {
    Point::new(p.x.clamp(b.min.x, b.max.x), p.y.clamp(b.min.y, b.max.y))
}

/// The letters (indexes into `letters`) in the sew order of the object.
fn letter_sequence(l: &emb_model::Lettering, entry: Point) -> Vec<usize> {
    use emb_model::LetterOrder;
    let n = l.letters.len();
    match l.order {
        LetterOrder::Text => (0..n).collect(),
        LetterOrder::CenterOut => {
            let m = n / 2;
            (m..n).chain((0..m).rev()).collect()
        }
        LetterOrder::Closest => {
            // The nearest letter next, from each possible first letter; the
            // shortest route wins (from the needle, through the letters).
            let boxes: Vec<Option<emb_geom::BBox>> = l.letters.iter().map(letter_box).collect();
            let dist = |i: usize, p: Point| boxes[i].map_or(f64::MAX, |b| nearest_in(&b, p).distance(p));
            let route = |first: usize| -> (f64, Vec<usize>) {
                let mut left: Vec<usize> = (0..n).filter(|&i| i != first).collect();
                let mut out = vec![first];
                let mut cost = dist(first, entry);
                let mut at = boxes[first].map_or(entry, |b| b.center());
                while !left.is_empty() {
                    let k = (0..left.len())
                        .min_by(|&a, &b| dist(left[a], at).total_cmp(&dist(left[b], at)))
                        .unwrap_or(0);
                    let i = left.remove(k);
                    cost += dist(i, at);
                    if let Some(b) = boxes[i] {
                        at = b.center();
                    }
                    out.push(i);
                }
                (cost, out)
            };
            (0..n)
                .map(route)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, r)| r)
                .unwrap_or_default()
        }
    }
}

/// The area of a lettering part as rings (even-odd): where its stitches
/// cover the fabric.
fn part_area(k: &ObjectKind) -> Vec<Vec<Point>> {
    let rings = |p: &emb_geom::Path| -> Vec<Polyline> { p.flatten(0.1) };
    match k {
        ObjectKind::Satin { geometry, .. } => {
            let (a, b) = (rings(&geometry.rail_a), rings(&geometry.rail_b));
            match (a.first(), b.first()) {
                // A ring column (as in "O"): the area between two closed rails.
                (Some(a), Some(b)) if a.closed && b.closed => vec![a.points.clone(), b.points.clone()],
                (Some(a), Some(b)) => vec![a.points.iter().chain(b.points.iter().rev()).copied().collect()],
                _ => Vec::new(),
            }
        }
        ObjectKind::Fill { outer, holes, .. } => std::iter::once(outer)
            .chain(holes)
            .flat_map(rings)
            .map(|pl| pl.points)
            .collect(),
        _ => Vec::new(),
    }
}

/// The travel from `a` to `b` is under one of `areas` (so the stitches of
/// that area cover it): every point of it, except near its ends, is inside.
fn hidden(a: Point, b: Point, areas: &[Vec<Vec<Point>>]) -> bool {
    let len = a.distance(b);
    let steps = (len / 0.25).ceil().max(1.0) as usize;
    (0..=steps).map(|i| a.lerp(b, i as f64 / steps as f64)).all(|p| {
        p.distance(a) < 0.3
            || p.distance(b) < 0.3
            || areas
                .iter()
                .any(|rings| rings.iter().filter(|r| emb_geom::region::ring_contains(r, p)).count() % 2 == 1)
    })
}

/// The ends of a satin column (the middle of each end), if it has two.
fn satin_ends(k: &ObjectKind) -> Option<(Point, Point)> {
    let ObjectKind::Satin { geometry, .. } = k else {
        return None;
    };
    let ends = |path: &emb_geom::Path| {
        let pts: Vec<Point> = path.points().collect();
        Some((*pts.first()?, *pts.last()?))
    };
    let ((a0, a1), (b0, b1)) = (ends(&geometry.rail_a)?, ends(&geometry.rail_b)?);
    (a0.distance(a1) > 1e-6).then(|| (a0.midpoint(b0), a1.midpoint(b1)))
}

/// The start edge and the end edge of a part that has them: the ends of
/// the two rails of a satin column, or the ends of an open line of run
/// stitches (each as an edge with no length).
fn part_edges(k: &ObjectKind) -> Option<((Point, Point), (Point, Point))> {
    match k {
        ObjectKind::Satin { geometry, .. } => {
            let ends = |path: &emb_geom::Path| {
                let pts: Vec<Point> = path.points().collect();
                Some((*pts.first()?, *pts.last()?))
            };
            let ((a0, a1), (b0, b1)) = (ends(&geometry.rail_a)?, ends(&geometry.rail_b)?);
            (a0.distance(a1) > 1e-6).then_some(((a0, b0), (a1, b1)))
        }
        _ => part_ends(k).map(|(a, b)| ((a, a), (b, b))),
    }
}

/// The shortest stitch of a travel run in a fixed letter, in mm.
const MIN_TRAVEL_MM: f64 = 0.4;

/// Takes out the points of `points` that are nearer than `min` to the
/// point before them. The first and the last point stay.
fn thin_points(points: &mut Vec<Point>, min: f64) {
    if points.len() < 3 {
        return;
    }
    let last = points[points.len() - 1];
    let mut out = vec![points[0]];
    for &p in &points[1..points.len() - 1] {
        let near_last = p.distance(last) < min;
        if out.last().is_some_and(|q| q.distance(p) >= min) && !near_last {
            out.push(p);
        }
    }
    out.push(last);
    *points = out;
}

/// The travel of a fixed letter from `needle` to the first point of `run`
/// along the edges where the font joins two parts: along the end edge of
/// the part before to `p`, to `q` on the start edge of this part, and along
/// that edge to the first point. The two points go into `run` as travel. Not
/// when the straight line goes near them already, or when the parts that
/// sew from now on (`unsewn`) cover the straight line.
fn along_edges(run: &mut Run, needle: Point, p: Point, q: Point, unsewn: &[&ObjectKind]) {
    let Some(&start) = run.points.first() else {
        return;
    };
    let off = |w: Point| w.distance_to_segment(needle, start) > EDGE_WAY_MM;
    if !off(p) && !off(q) {
        return;
    }
    let areas: Vec<Vec<Vec<Point>>> = unsewn.iter().map(|k| part_area(k)).collect();
    if hidden(needle, start, &areas) {
        return;
    }
    let mut way: Vec<Point> = Vec::with_capacity(2);
    for w in [p, q] {
        let last = way.last().copied().unwrap_or(needle);
        if w.distance(last) >= MIN_TRAVEL_MM && w.distance(start) >= MIN_TRAVEL_MM {
            way.push(w);
        }
    }
    let k = way.len();
    if k == 0 {
        return;
    }
    run.points.splice(0..0, way);
    for r in &mut run.travel {
        *r = r.start + k..r.end + k;
    }
    run.travel.insert(0, 0..k);
}

/// The points of the edges `a` and `b` that are nearest to each other
/// (from steps along each edge).
fn nearest_points(a: (Point, Point), b: (Point, Point)) -> (Point, Point) {
    const STEPS: usize = 16;
    let along = |e: (Point, Point), i: usize| e.0.lerp(e.1, i as f64 / STEPS as f64);
    let mut best = (f64::MAX, a.0, b.0);
    for i in 0..=STEPS {
        for j in 0..=STEPS {
            let (p, q) = (along(a, i), along(b, j));
            if p.distance(q) < best.0 {
                best = (p.distance(q), p, q);
            }
        }
    }
    (best.1, best.2)
}

/// The start and the end of a part that has them: a satin column with two
/// ends, or an open line of run stitches.
fn part_ends(k: &ObjectKind) -> Option<(Point, Point)> {
    match k {
        ObjectKind::Running { path, .. } => {
            let pts: Vec<Point> = path.points().collect();
            let (a, b) = (*pts.first()?, *pts.last()?);
            (a.distance(b) > 1e-6).then_some((a, b))
        }
        _ => satin_ends(k),
    }
}

/// The point of the outline of `k` nearest `p` (where a fill starts).
fn nearest_on_outline(k: &ObjectKind, p: Point) -> Point {
    let mut best = (f64::MAX, p);
    for path in k.paths() {
        for pl in path.flatten(0.1) {
            let n = pl.points.len();
            let segs = if pl.closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (a, b) = (pl.points[i], pl.points[(i + 1) % n]);
                let ab = b - a;
                let t = if ab.length_sq() > 0.0 {
                    ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let q = a + ab * t;
                if q.distance(p) < best.0 {
                    best = (q.distance(p), q);
                }
            }
        }
    }
    best.1
}

/// Letters with more parts than this get no full search of their sew order.
const MAX_SEARCH_PARTS: usize = 9;

/// One way to sew a part: its index, and the start and end of a satin
/// (`None`: the part starts at the point of its outline nearest the needle).
type Mode = (usize, Option<(Point, Point)>);

/// The sew order of the parts of one letter, with the fewest trims, then
/// the shortest travel, then the end nearest `target`. A connection needs
/// no trim when it is at most `trim_over` long or the parts that sew after
/// it cover it. `free_entry`: the first connection is not counted (it joins
/// the object before). Returns (part, start, end) for each part; the end is
/// `None` for a part with no fixed ends.
fn plan_letter(
    parts: &[ObjectKind],
    needle: Point,
    target: Option<Point>,
    trim_over: f64,
    free_entry: bool,
) -> Vec<(usize, Point, Option<Point>)> {
    let n = parts.len();
    if n == 0 {
        return Vec::new();
    }
    let modes: Vec<Mode> = (0..n)
        .flat_map(|i| match satin_ends(&parts[i]) {
            Some((a, b)) => vec![(i, Some((a, b))), (i, Some((b, a)))],
            None => vec![(i, None)],
        })
        .collect();
    let center = |i: usize| parts[i].control_bbox().map_or(needle, |b| b.center());
    let start_of = |m: &Mode, from: Point| m.1.map_or_else(|| nearest_on_outline(&parts[m.0], from), |(s, _)| s);
    let end_of = |m: &Mode| m.1.map_or_else(|| center(m.0), |(_, e)| e);
    if n > MAX_SEARCH_PARTS {
        // Too many parts to search: the order of the font, each from its end nearest the needle.
        let mut at = needle;
        return (0..n)
            .map(|i| {
                let m = modes
                    .iter()
                    .filter(|m| m.0 == i)
                    .min_by(|a, b| start_of(a, at).distance(at).total_cmp(&start_of(b, at).distance(at)))
                    .copied()
                    .unwrap_or((i, None));
                let s = start_of(&m, at);
                at = end_of(&m);
                (i, s, m.1.map(|(_, e)| e))
            })
            .collect();
    }
    let areas: Vec<Vec<Vec<Point>>> = parts.iter().map(part_area).collect();
    let inside = |k: usize, p: Point| {
        areas[k]
            .iter()
            .filter(|r| emb_geom::region::ring_contains(r, p))
            .count()
            % 2
            == 1
    };
    // For a connection from `a` to `b`: for each point on it, the parts that cover it.
    let cover = |a: Point, b: Point| -> Vec<u32> {
        let steps = (a.distance(b) / 0.25).ceil().max(1.0) as usize;
        (0..=steps)
            .map(|i| a.lerp(b, i as f64 / steps as f64))
            .filter(|p| p.distance(a) >= 0.3 && p.distance(b) >= 0.3)
            .map(|p| (0..n).filter(|&k| inside(k, p)).fold(0u32, |m, k| m | 1 << k))
            .collect()
    };
    // Connections from the needle (from 0) and from the end of each mode (from 1 + mode).
    let froms: Vec<Point> = std::iter::once(needle).chain(modes.iter().map(&end_of)).collect();
    struct Link {
        dist: f64,
        short: bool,
        cover: Vec<u32>,
    }
    let links: Vec<Vec<Link>> = froms
        .iter()
        .map(|&f| {
            modes
                .iter()
                .map(|m| {
                    let s = start_of(m, f);
                    let dist = f.distance(s);
                    Link {
                        dist,
                        short: dist <= trim_over,
                        cover: if dist <= trim_over { Vec::new() } else { cover(f, s) },
                    }
                })
                .collect()
        })
        .collect();
    let trim = |link: &Link, unsewn: u32| u32::from(!(link.short || link.cover.iter().all(|c| c & unsewn != 0)));
    let all: u32 = (1 << n) - 1;
    let size = 1usize << n;
    // best[mask][mode] = (trims, travel, previous mode)
    let mut best: Vec<Vec<Option<(u32, f64, usize)>>> = vec![vec![None; modes.len()]; size];
    for (j, m) in modes.iter().enumerate() {
        let link = &links[0][j];
        let t = if free_entry { 0 } else { trim(link, all) };
        best[1 << m.0][j] = Some((t, link.dist, usize::MAX));
    }
    let better = |a: (u32, f64), b: Option<(u32, f64, usize)>| b.is_none_or(|b| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1));
    for mask in 1..size {
        for i in 0..modes.len() {
            let Some((t0, d0, _)) = best[mask][i] else { continue };
            if mask & (1 << modes[i].0) == 0 {
                continue;
            }
            let unsewn = all & !(mask as u32);
            for (j, m) in modes.iter().enumerate() {
                if mask & (1 << m.0) != 0 {
                    continue;
                }
                let link = &links[1 + i][j];
                let cost = (t0 + trim(link, unsewn), d0 + link.dist);
                let next = mask | (1 << m.0);
                if better(cost, best[next][j]) {
                    best[next][j] = Some((cost.0, cost.1, i));
                }
            }
        }
    }
    let finish = |j: usize| target.map_or(0.0, |t| end_of(&modes[j]).distance(t));
    let Some(mut j) = (0..modes.len())
        .filter(|&j| best[size - 1][j].is_some())
        .min_by(|&a, &b| {
            let (ta, da, _) = best[size - 1][a].unwrap_or((u32::MAX, 0.0, 0));
            let (tb, db, _) = best[size - 1][b].unwrap_or((u32::MAX, 0.0, 0));
            ta.cmp(&tb).then((da + finish(a)).total_cmp(&(db + finish(b))))
        })
    else {
        return Vec::new();
    };
    let mut order = Vec::with_capacity(n);
    let mut mask = size - 1;
    loop {
        order.push(j);
        let Some((_, _, prev)) = best[mask][j] else { break };
        mask &= !(1 << modes[j].0);
        if prev == usize::MAX {
            break;
        }
        j = prev;
    }
    order.reverse();
    let mut at = needle;
    order
        .into_iter()
        .map(|j| {
            let m = modes[j];
            let s = start_of(&m, at);
            at = end_of(&m);
            (m.0, s, m.1.map(|(_, e)| e))
        })
        .collect()
}

/// How a part of a fixed letter (ADR 0112) joins the part before it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FixedJoin {
    /// With a trim, if the gap is longer than the trim distance and the
    /// parts that sew from now on do not cover it.
    Apart,
    /// With no trim: the end edge of the part before is near the start
    /// edge of this part (the first point and the second point are the
    /// nearest points of the two edges), or the parts that sew from now on
    /// cover the way between them.
    Near(Point, Point),
}

/// A way along the edges is used when a point of it is farther than this
/// from the straight line, mm (the half width of a thread and a groove).
const EDGE_WAY_MM: f64 = 0.3;

/// Sewing state of a lettering object.
struct LetterSewer<'a> {
    l: &'a emb_model::Lettering,
    runs: Vec<Run>,
    needle: Point,
    long_gap: RunLink,
    /// The satin and fill parts of the letter that sews, with their areas
    /// and `true` once they are sewn: a satin sews no underlay where
    /// another of them lies (ADR 0163), and a puff column no end stitches
    /// where one sewn before it lies (ADR 0165).
    parts: Vec<(&'a ObjectKind, Vec<Vec<Point>>, bool)>,
}

impl<'a> LetterSewer<'a> {
    /// The parts of `letter` that a satin does not sew its underlay on.
    fn start_letter(&mut self, letter: &'a emb_model::Letter) {
        self.parts = letter
            .parts
            .iter()
            .filter(|p| matches!(p, ObjectKind::Satin { .. } | ObjectKind::Fill { .. }))
            .map(|p| (p, part_area(p), false))
            .filter(|(_, a, _)| !a.is_empty())
            .collect();
    }
}

impl LetterSewer<'_> {
    /// Sews `part` from `from` (to `exit`). `unsewn` are the parts that sew
    /// from now on, `part` too: with `smart`, their stitches can cover a
    /// connection. `fixed`: a part of a fixed letter, and how it joins the
    /// part before it.
    fn sew(
        &mut self,
        part: &ObjectKind,
        from: Point,
        exit: Option<Point>,
        unsewn: &[&ObjectKind],
        smart: bool,
        fixed: Option<FixedJoin>,
    ) {
        let others: Vec<Vec<Vec<Point>>> = self
            .parts
            .iter()
            .filter(|(p, _, _)| !std::ptr::eq(*p, part))
            .map(|(_, a, _)| a.clone())
            .collect();
        let before: Vec<Vec<Vec<Point>>> = self
            .parts
            .iter()
            .filter(|(p, _, sewn)| *sewn && !std::ptr::eq(*p, part))
            .map(|(_, a, _)| a.clone())
            .collect();
        if let Some(entry) = self.parts.iter_mut().find(|(p, _, _)| std::ptr::eq(*p, part)) {
            entry.2 = true;
        }
        let made = match part {
            ObjectKind::Satin { geometry, params } if !others.is_empty() => {
                match satin::satin_on(geometry, params, from, exit, &others, &before) {
                    Some(points) if points.len() >= 2 => Ok(ObjectStitches {
                        runs: vec![Run {
                            long_jumps: params.auto_jump,
                            ..Run::new(points)
                        }],
                    }),
                    _ => Err(StitchError::Empty),
                }
            }
            _ => stitch_kind(part, from, exit),
        };
        let Ok(mut st) = made else {
            return;
        };
        let near = match fixed {
            Some(FixedJoin::Near(p, q)) => Some((p, q)),
            _ => None,
        };
        let joined = near.is_some();
        let fixed = fixed.is_some();
        // A travel run of a fixed letter turns in the columns that hide it.
        // At a small size two turns are near, and each turn is a needle
        // point: the points that are too near go.
        if fixed && matches!(part, ObjectKind::Running { .. }) {
            for run in &mut st.runs {
                thin_points(&mut run.points, MIN_TRAVEL_MM);
            }
        }
        if let Some(first) = st.runs.first_mut()
            && !self.runs.is_empty()
            && let Some(&p) = first.points.first()
        {
            let short = p.distance(self.needle) <= self.l.trim_over_mm;
            let covered = || {
                let areas: Vec<Vec<Vec<Point>>> = unsewn.iter().map(|k| part_area(k)).collect();
                hidden(self.needle, p, &areas)
            };
            first.link = if short || joined || (smart && covered()) {
                RunLink::Travel
            } else {
                self.long_gap
            };
            // In a fixed letter, a part starts where the part before it
            // ends: the first needle point is the same place again.
            if fixed
                && first.link == RunLink::Travel
                && p.distance(self.needle) < MIN_TRAVEL_MM
                && first.points.len() > 1
            {
                first.points.remove(0);
            }
            // The font joins the two parts at their edges: the needle goes
            // along the end edge of the part before (on its end stitches)
            // and along the start edge of this part (under its first
            // stitches). The straight line from the needle to the first
            // point went across the column sewn before when the needle and
            // the first point are at far ends of the two edges: 11 mm in
            // a letter of Fat Face Serif at 42.5 mm (P22 Milestone 1).
            if let Some((p, q)) = near
                && first.link == RunLink::Travel
            {
                along_edges(first, self.needle, p, q, unsewn);
            }
        }
        // A run in a fixed letter is a travel run (the route makes it under
        // the parts that sew later), in the direction of the route: the
        // plan must not turn it round. As the first run of a lettering the
        // plan turned it to the needle, and a straight travel stitch of up to
        // 27 mm went back over the fabric to the next part (P22 Milestone 1).
        if fixed && matches!(part, ObjectKind::Running { .. }) {
            for run in &mut st.runs {
                let all = 0..run.points.len().saturating_sub(1);
                run.travel = Vec::from([all]);
                run.fixed = true;
            }
        }
        if let Some(p) = st.runs.last().and_then(|r| r.points.last()) {
            self.needle = *p;
        }
        self.runs.extend(st.runs);
    }
}

/// The runs of a lettering object: the letters in the sew order of the
/// object. A part joins the one before it with a travel stitch when the gap
/// is at most `trim_over_mm`, else with a trim (or a jump, with the jump
/// connection). The last part of the last letter ends near `exit` if it is
/// set.
///
/// In text order, each letter sews the part nearest the needle next. The
/// center-out and closest orders optimize the connections (ADR 0018): the
/// parts of each letter sew in the order and direction with the fewest
/// trims (a longer gap needs no trim when the parts that sew after it cover
/// it), then the shortest travel, ending near the next letter.
fn lettering_runs(l: &emb_model::Lettering, entry: Point, exit: Option<Point>) -> Vec<Run> {
    use emb_model::{LetterConnection, LetterOrder};
    // A knockout leaves out the area that objects in front cut (ADR 0093).
    let cut = crate::knockout::knocked_out(l);
    let l = cut.as_ref().unwrap_or(l);
    let smart = l.order != LetterOrder::Text;
    let mut w = LetterSewer {
        l,
        runs: Vec::new(),
        needle: entry,
        long_gap: match l.connection {
            LetterConnection::Trim => RunLink::Trim,
            LetterConnection::Jump => RunLink::Jump,
        },
        parts: Vec::new(),
    };
    let seq = letter_sequence(l, entry);
    for (n, &li) in seq.iter().enumerate() {
        let letter = &l.letters[li];
        w.start_letter(letter);
        // Where this letter should end: near the next letter.
        let target = match seq.get(n + 1) {
            None => exit,
            Some(_) if !smart => None,
            Some(&next) => letter_box(&l.letters[next])
                .zip(letter_box(letter))
                .map(|(nb, b)| nearest_in(&nb, b.center())),
        };
        if letter.fixed {
            // A letter that a person put in order (ADR 0112): the parts
            // in the order of the font, each column from its start to its
            // end. A part with no ends starts near the needle and ends
            // near the start of the next part.
            let last = letter.parts.len().saturating_sub(1);
            // The end edge of the part before in this letter, if it has
            // ends.
            let mut anchor: Option<(Point, Point)> = None;
            for (k, part) in letter.parts.iter().enumerate() {
                let ends = part_ends(part);
                let from = ends.map_or(w.needle, |(a, _)| a);
                let next = letter.parts.get(k + 1).and_then(part_ends).map(|(a, _)| a);
                let to = ends.map(|(_, b)| b).or(next).or(if k == last {
                    target.or(exit.filter(|_| n + 1 == seq.len()))
                } else {
                    None
                });
                let unsewn: Vec<&ObjectKind> = letter.parts[k..].iter().collect();
                // The font joins the end of the part before with the start
                // of this part: the two edges are near, or the parts that
                // sew from now on cover the way between them. The needle
                // is on the end edge of the column before, and the first
                // stitch is on the start edge of this column: a travel
                // stitch along an edge lies on the end stitches of its
                // column, so it does not show.
                let edges = part_edges(part);
                let join = anchor
                    .zip(edges)
                    .map(|(a, (s, _))| nearest_points(a, s))
                    .filter(|&(p, q)| {
                        p.distance(q) <= l.trim_over_mm || {
                            let areas: Vec<Vec<Vec<Point>>> = unsewn.iter().map(|k| part_area(k)).collect();
                            hidden(p, q, &areas)
                        }
                    })
                    .map_or(FixedJoin::Apart, |(p, q)| FixedJoin::Near(p, q));
                w.sew(part, from, to, &unsewn, true, Some(join));
                anchor = edges.map(|(_, b)| b);
            }
            continue;
        }
        if smart {
            let plan = plan_letter(&letter.parts, w.needle, target, l.trim_over_mm, w.runs.is_empty());
            let last = plan.len().saturating_sub(1);
            for (k, &(pi, from, end)) in plan.iter().enumerate() {
                let unsewn: Vec<&ObjectKind> = plan[k..].iter().map(|&(i, _, _)| &letter.parts[i]).collect();
                let to = if k == last { end.or(target) } else { end };
                w.sew(&letter.parts[pi], from, to, &unsewn, true, None);
            }
            continue;
        }
        let mut left: Vec<&ObjectKind> = letter.parts.iter().collect();
        while !left.is_empty() {
            let needle = w.needle;
            let dist = |k: &ObjectKind| {
                k.control_bbox()
                    .map_or(f64::MAX, |b| nearest_in(&b, needle).distance(needle))
            };
            let i = (0..left.len())
                .min_by(|&a, &b| dist(left[a]).total_cmp(&dist(left[b])))
                .unwrap_or(0);
            let part = left.remove(i);
            let last = left.is_empty() && n + 1 == seq.len();
            w.sew(part, needle, if last { exit } else { None }, &[], false, None);
        }
    }
    w.runs
}

/// The underlay layers that `Auto` selects for a fill shape.
pub fn auto_fill_underlay(shape: &Shape) -> Vec<UnderlayLayer> {
    let area: f64 = shape.regions.iter().map(|r| r.area()).sum();
    if area < AUTO_FILL_MIN_AREA || shape.inset(AUTO_FILL_INSET_MM).is_empty() {
        return Vec::new();
    }
    vec![UnderlayLayer::Tatami {
        inset_mm: AUTO_FILL_INSET_MM,
        spacing_mm: AUTO_FILL_SPACING_MM,
        angle_deg: 90.0,
    }]
}

/// The underlay layers that `Light` selects for a fill shape: an edge walk
/// for shapes that `Auto` gives underlay.
pub fn light_fill_underlay(shape: &Shape) -> Vec<UnderlayLayer> {
    if auto_fill_underlay(shape).is_empty() {
        return Vec::new();
    }
    vec![UnderlayLayer::EdgeWalk {
        inset_mm: AUTO_FILL_INSET_MM,
    }]
}

/// The underlay layers that `Heavy` selects for a fill shape: an edge walk,
/// the tatami of `Auto`, and a second tatami along the top rows. A small
/// shape gets the edge walk only.
pub fn heavy_fill_underlay(shape: &Shape) -> Vec<UnderlayLayer> {
    if shape.inset(AUTO_FILL_INSET_MM).is_empty() {
        return Vec::new();
    }
    let mut v = vec![UnderlayLayer::EdgeWalk {
        inset_mm: AUTO_FILL_INSET_MM,
    }];
    let auto = auto_fill_underlay(shape);
    if !auto.is_empty() {
        v.extend(auto);
        v.push(UnderlayLayer::Tatami {
            inset_mm: AUTO_FILL_INSET_MM,
            spacing_mm: HEAVY_FILL_SPACING_MM,
            angle_deg: 0.0,
        });
    }
    v
}

/// The underlay layers that `Lace` selects for a fill shape (free-standing
/// lace): the edge walk two times on the same needle points, so its threads
/// lock, and the tatami of `Auto` across the top rows.
pub fn lace_fill_underlay(shape: &Shape) -> Vec<UnderlayLayer> {
    if shape.inset(AUTO_FILL_INSET_MM).is_empty() {
        return Vec::new();
    }
    let edge = UnderlayLayer::EdgeWalk {
        inset_mm: AUTO_FILL_INSET_MM,
    };
    let mut v = vec![edge.clone(), edge];
    v.extend(auto_fill_underlay(shape));
    v
}

/// The underlay layers of the setting `u` for a fill shape.
fn fill_underlay_layers(shape: &Shape, u: &Underlay) -> Vec<UnderlayLayer> {
    match u {
        Underlay::Auto => auto_fill_underlay(shape),
        Underlay::Light => light_fill_underlay(shape),
        Underlay::Heavy => heavy_fill_underlay(shape),
        Underlay::Double => {
            let mut v = auto_fill_underlay(shape);
            if let Some(UnderlayLayer::Tatami {
                inset_mm, spacing_mm, ..
            }) = v.first().cloned()
            {
                v = vec![
                    UnderlayLayer::Tatami {
                        inset_mm,
                        spacing_mm,
                        angle_deg: 45.0,
                    },
                    UnderlayLayer::Tatami {
                        inset_mm,
                        spacing_mm,
                        angle_deg: -45.0,
                    },
                ];
            }
            v
        }
        Underlay::Lace => lace_fill_underlay(shape),
        Underlay::None => Vec::new(),
        Underlay::Custom(l) => l.clone(),
    }
}

/// The underlay layers that a fill or a satin column sews: the layers of
/// its underlay setting for its shape or its width (the writer of Ink/Stitch
/// files needs them, ADR 0136). `None` for the other kinds. Tatami angles
/// are relative to the fill angle.
pub fn underlay_layers(kind: &ObjectKind) -> Option<Vec<UnderlayLayer>> {
    match kind {
        ObjectKind::Fill { outer, holes, params } => {
            let outers = outer.flatten(FLATTEN_TOL_MM);
            let holes: Vec<Polyline> = holes.iter().flat_map(|h| h.flatten(FLATTEN_TOL_MM)).collect();
            Some(fill_underlay_layers(&Shape::new(&outers, &holes), &params.underlay))
        }
        // A band after the first has no underlay (the first band sews the
        // underlay of the whole column).
        ObjectKind::Satin { params, .. } if params.active_band().is_some_and(|b| !b.sews_underlay()) => {
            Some(Vec::new())
        }
        ObjectKind::Satin { geometry, params } => {
            let width = crate::satin::Column::new(geometry).map_or(0.0, |c| c.mean_width());
            Some(crate::satin::underlay_layers(&params.underlay, width))
        }
        _ => None,
    }
}

/// The row split of a fill sewn as tatami rows (tatami, program split, user
/// split, with their carving stamps); `None` for the other fill types.
fn row_split(shape: &Shape, p: &FillParams) -> Option<crate::fill::RowSplit> {
    let mut split = crate::fill::RowSplit::default();
    match &p.pattern {
        FillPattern::Tatami => {}
        FillPattern::Gradient { line, stops, color } => {
            split.keep = Some(crate::tartan::gradient_keep(shape, p.angle_deg, *line, stops, *color));
        }
        FillPattern::ProgramSplit { pattern, scale } => {
            // A tile less than 0.5 mm wide (a wrong value) would give no end of needle points.
            split.pattern = Some((pattern.rows.clone(), (pattern.width_mm * scale.max(0.1)).max(0.5)));
        }
        FillPattern::UserSplit { lines } => {
            split.lines = lines.clone();
            split.only_lines = true;
        }
        _ => return None,
    }
    if p.pattern.takes_stamps() {
        split.lines.extend(p.stamps.iter().flat_map(|s| s.lines()));
    }
    split.jagged_mm = p.jagged_mm;
    split.accordion = p.accordion.clone();
    Some(split)
}

/// Needle paths, each with the ranges of its segments that are travel.
type MarkedRuns = Vec<(Vec<Point>, Vec<std::ops::Range<usize>>)>;

/// Needle paths of a fill: underlay, then the top rows. Underlay tatami
/// angles are relative to the fill angle.
fn fill_runs(shape: &Shape, p: &FillParams, entry: Point, exit: Option<Point>) -> MarkedRuns {
    if shape.is_empty() {
        return Vec::new();
    }
    let travel = Travel::new(shape);
    let mut sewer = Sewer::new(&travel, TRAVEL_LEN_MM);
    sewer.edge_only = p.trapunto;
    // All travel of the fill, also of its underlay, goes along the top rows
    // where it can: the top rows cover it best there. Only straight rows
    // have a direction: the rows of the other patterns (circular, guided,
    // contour...) are curves, and the angle of the fill sent their travel
    // straight across them (P22 Milestone 1).
    let split = row_split(shape, p);
    if split.is_some() {
        sewer.rows = Some(emb_geom::Transform::rotate_deg(p.angle_deg).apply_vector(Point::new(1.0, 0.0)));
    }
    let layers = fill_underlay_layers(shape, &p.underlay);
    for layer in &layers {
        match *layer {
            UnderlayLayer::Tatami {
                inset_mm,
                spacing_mm,
                angle_deg,
            } => {
                let t = Tatami {
                    spacing_mm,
                    angle_deg: p.angle_deg + angle_deg,
                    stitch_len_mm: UNDERLAY_LEN_MM,
                    pull_comp_mm: 0.0,
                    staggers: FILL_STAGGERS,
                    split: crate::fill::RowSplit::default(),
                };
                sew_tatami(&shape.inset(inset_mm), &t, entry, None, &mut sewer);
            }
            UnderlayLayer::EdgeWalk { inset_mm } => {
                for ring in shape.inset(inset_mm).rings() {
                    let line = Polyline {
                        points: ring.clone(),
                        closed: true,
                    };
                    let mut pts = running::running_points(&line, UNDERLAY_LEN_MM);
                    // Start the loop at its point nearest to the needle.
                    let at = sewer.needle().unwrap_or(entry);
                    pts.pop();
                    let start = (0..pts.len())
                        .min_by(|&i, &j| pts[i].distance(at).total_cmp(&pts[j].distance(at)))
                        .unwrap_or(0);
                    pts.rotate_left(start);
                    if let Some(&first) = pts.first() {
                        pts.push(first);
                        sewer.go_to(first);
                        sewer.stitch(pts.into_iter().skip(1));
                    }
                }
            }
            // These layers are for satin columns.
            UnderlayLayer::CenterWalk | UnderlayLayer::Zigzag { .. } => {}
        }
    }
    if let Some(split) = split {
        let top = Tatami {
            spacing_mm: p.spacing_mm,
            angle_deg: p.angle_deg,
            stitch_len_mm: p.stitch_length_mm,
            pull_comp_mm: p.pull_compensation_mm,
            staggers: FILL_STAGGERS,
            split,
        };
        sew_tatami(shape, &top, entry, exit, &mut sewer);
    } else {
        // Rings and guided rows: the travel between the parts goes round
        // the rows that are sewn, else the needle path starts again (P22
        // Milestone 1: their travel went straight across sewn rows). The
        // other curved patterns keep their travel (a satin fill of a ring
        // has no way round its sewn columns and got trims).
        if matches!(p.pattern, FillPattern::Circular { .. } | FillPattern::Guided { .. }) {
            sewer.track_sewn();
        }
        crate::curves::sew(shape, p, entry, &mut sewer);
    }
    sewer.finish_with_travel()
}

pub type CachedStitches = Arc<Result<ObjectStitches, StitchError>>;

/// Cache of object stitches. The key `(ObjectId, revision)` changes when the
/// object changes, and undo brings back an old key, so an undo needs no
/// regeneration. Old entries are removed when the cache is too large.
#[derive(Debug, Default)]
pub struct StitchCache {
    map: HashMap<CacheKey, (CachedStitches, u64)>,
    tick: u64,
    hits: u64,
    misses: u64,
}

/// Object id, revision, and the entry point in µm (see [`entry_for`]).
type CacheKey = (ObjectId, u64, Option<(i64, i64)>);

/// Entries kept beyond the objects in the current design (for undo).
const EXTRA_ENTRIES: usize = 512;

impl StitchCache {
    /// The stitches of `obj` when the needle is at `needle` (design coordinates).
    pub fn get(&mut self, obj: &DesignObject, needle: Point) -> CachedStitches {
        self.tick += 1;
        let entry = entry_for(obj, needle);
        let um = |v: f64| (v * 1000.0).round() as i64;
        let key = (obj.id, obj.revision, entry.map(|p| (um(p.x), um(p.y))));
        if let Some((v, used)) = self.map.get_mut(&key) {
            *used = self.tick;
            self.hits += 1;
            return v.clone();
        }
        self.misses += 1;
        let v = Arc::new(stitch_object(obj, entry.unwrap_or(needle)));
        self.map.insert(key, (v.clone(), self.tick));
        v
    }

    /// Removes the least recently used entries above `live + EXTRA_ENTRIES`.
    pub fn trim(&mut self, live: usize) {
        let max = live + EXTRA_ENTRIES;
        if self.map.len() <= max {
            return;
        }
        let mut by_use: Vec<(CacheKey, u64)> = self.map.iter().map(|(k, (_, t))| (*k, *t)).collect();
        by_use.sort_by_key(|(_, t)| *t);
        let remove = self.map.len() - max;
        for (k, _) in by_use.into_iter().take(remove) {
            self.map.remove(&k);
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// (hits, misses) since the cache was made.
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{Design, RunningParams, ThreadId};

    use super::*;

    fn line_design() -> (Design, ObjectId) {
        let mut d = Design::new("t");
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::new().move_to(Point::ZERO).line_to(Point::new(10.0, 0.0)),
                params: RunningParams::default(),
            },
        );
        (d, id)
    }

    /// A lettering with one letter of two upright columns: the first one
    /// at X = 20 from the top down, the second one at X = 0 from the
    /// bottom up.
    fn two_columns(fixed: bool) -> emb_model::Lettering {
        let column = |x: f64, from: f64, to: f64| ObjectKind::Satin {
            geometry: emb_model::SatinGeometry::from_centerline(
                &Path::new().move_to(Point::new(x, from)).line_to(Point::new(x, to)),
                2.0,
            ),
            params: Default::default(),
        };
        let mut l = emb_model::Lettering::new("H", Default::default(), 10.0, Point::ZERO);
        l.letters = vec![emb_model::Letter {
            text: "H".into(),
            parts: vec![column(20.0, -10.0, 0.0), column(0.0, 0.0, -10.0)],
            fixed,
            ..Default::default()
        }];
        l
    }

    #[test]
    fn a_fixed_letter_sews_in_the_order_and_direction_of_the_font() {
        let ends = |l: &emb_model::Lettering| {
            let runs = lettering_runs(l, Point::new(0.0, -10.0), None);
            let first = runs.first().and_then(|r| r.points.first()).copied().unwrap();
            let last = runs.last().and_then(|r| r.points.last()).copied().unwrap();
            (first, last)
        };
        // The stitch engine starts with the column at the needle, at the
        // end that is nearest.
        let (first, _) = ends(&two_columns(false));
        assert!(first.distance(Point::new(0.0, -10.0)) < 1.5, "{first:?}");
        // The order of the font: the column at X = 20 first, from its top
        // to its bottom; then the column at X = 0, from its bottom to its
        // top.
        let (first, last) = ends(&two_columns(true));
        assert!(first.distance(Point::new(20.0, -10.0)) < 1.5, "{first:?}");
        assert!(last.distance(Point::new(0.0, -10.0)) < 1.5, "{last:?}");
        // The closest order of the letters does not change a fixed letter.
        let mut l = two_columns(true);
        l.order = emb_model::LetterOrder::Closest;
        assert_eq!(ends(&l), (first, last));
    }

    #[test]
    fn a_fixed_letter_joins_parts_along_the_end_of_a_column() {
        // A column 6 mm wide from the bottom to the top, a travel run that
        // starts on its end edge 2.5 mm from the middle, and a column that
        // starts where the run ends.
        let column = |from: (f64, f64), to: (f64, f64), width: f64| ObjectKind::Satin {
            geometry: emb_model::SatinGeometry::from_centerline(
                &Path::new()
                    .move_to(Point::new(from.0, from.1))
                    .line_to(Point::new(to.0, to.1)),
                width,
            ),
            params: Default::default(),
        };
        let run = ObjectKind::Running {
            path: Path::new()
                .move_to(Point::new(2.5, -20.0))
                .line_to(Point::new(2.5, -23.0))
                .line_to(Point::new(12.0, -23.0)),
            params: RunningParams::default(),
        };
        let mut l = emb_model::Lettering::new("L", Default::default(), 10.0, Point::ZERO);
        l.letters = vec![emb_model::Letter {
            text: "L".into(),
            parts: vec![
                column((0.0, 0.0), (0.0, -20.0), 6.0),
                run,
                // The run goes under this column, which sews after it.
                column((12.0, -23.0), (-3.0, -23.0), 6.0),
            ],
            fixed: true,
            ..Default::default()
        }];
        let trims = |l: &emb_model::Lettering| {
            lettering_runs(l, Point::ZERO, None)
                .iter()
                .skip(1)
                .filter(|r| r.link == RunLink::Trim)
                .count()
        };
        assert_eq!(trims(&l), 0);
        // A run that starts away from the end of the column needs a trim.
        if let ObjectKind::Running { path, .. } = &mut l.letters[0].parts[1] {
            *path = Path::new()
                .move_to(Point::new(9.0, -12.0))
                .line_to(Point::new(12.0, -23.0));
        }
        assert_eq!(trims(&l), 1);
    }

    #[test]
    fn a_fixed_letter_goes_along_the_edges_where_two_columns_meet() {
        let rails = |a: [(f64, f64); 2], b: [(f64, f64); 2]| {
            let line = |p: [(f64, f64); 2]| {
                Path::new()
                    .move_to(Point::new(p[0].0, p[0].1))
                    .line_to(Point::new(p[1].0, p[1].1))
            };
            ObjectKind::Satin {
                geometry: emb_model::SatinGeometry {
                    rail_a: line(a),
                    rail_b: line(b),
                    rungs: Vec::new(),
                },
                params: Default::default(),
            }
        };
        // A column from left to right (0 to `len`, 5 mm wide), and a column
        // 10 mm wide down from its lower side: the end edge of the first
        // and the start edge of the second meet at (`len`, 5). The second
        // starts at its rail A, at (0, 5).
        let mut far = 0;
        for len in [10.0, 10.2, 10.4, 10.6] {
            let mut l = emb_model::Lettering::new("L", Default::default(), 10.0, Point::ZERO);
            l.letters = vec![emb_model::Letter {
                text: "L".into(),
                parts: vec![
                    rails([(0.0, 0.0), (len, 0.0)], [(0.0, 5.0), (len, 5.0)]),
                    rails([(0.0, 5.0), (0.0, 15.0)], [(len, 5.0), (len, 15.0)]),
                ],
                fixed: true,
                ..Default::default()
            }];
            let runs = lettering_runs(&l, Point::new(-5.0, 0.0), None);
            assert_eq!(runs.len(), 2);
            assert_eq!(runs[1].link, RunLink::Travel);
            let needle = *runs[0].points.last().unwrap();
            if needle.distance(Point::new(len, 0.0)) < 0.5 {
                far += 1;
            }
            // The travel: from the needle to the end of the travel points of
            // the second column. No point of it is inside the first column.
            let k = runs[1].travel.first().map_or(0, |r| r.end);
            let way: Vec<Point> = std::iter::once(needle)
                .chain(runs[1].points[..=k].iter().copied())
                .collect();
            for w in way.windows(2) {
                for i in 0..=20 {
                    let p = w[0].lerp(w[1], f64::from(i) / 20.0);
                    let inside = p.x > 0.5 && p.x < len - 0.5 && p.y > 0.5 && p.y < 4.5;
                    assert!(
                        !inside,
                        "len {len}: the travel {way:?} goes across the first column at {p:?}"
                    );
                }
            }
        }
        assert!(far > 0, "a case with the needle at the far end of the first column");
    }

    #[test]
    fn running_object_has_one_run() {
        let (d, id) = line_design();
        let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
        assert_eq!(s.runs.len(), 1);
        assert_eq!(s.runs[0].points.len(), 5);
    }

    fn square_fill(params: FillParams) -> DesignObject {
        let mut d = Design::new("t");
        let outer = Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(20.0, 20.0)));
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Fill {
                outer,
                holes: vec![],
                params,
            },
        );
        d.object(id).unwrap().clone()
    }

    #[test]
    fn the_colors_of_a_gradient_share_the_rows_of_one_fill() {
        let rows = |runs: &[Run]| -> std::collections::BTreeSet<i64> {
            runs.iter()
                .flat_map(|r| r.points.windows(2))
                .filter(|w| (w[0].y - w[1].y).abs() < 1e-9 && (w[0].x - w[1].x).abs() > 1.0)
                .map(|w| (w[0].y * 100.0).round() as i64)
                .collect()
        };
        let base = FillParams {
            angle_deg: 0.0,
            underlay: Underlay::None,
            ..FillParams::default()
        };
        let tatami = stitch_object(&square_fill(base.clone()), Point::ZERO).unwrap();
        let all = rows(&tatami.runs);
        let mut seen = std::collections::BTreeSet::new();
        for color in 0..3 {
            let p = FillParams {
                pattern: FillPattern::Gradient {
                    line: Some((Point::new(0.0, 0.0), Point::new(0.0, 20.0))),
                    stops: vec![0.0, 0.5, 1.0],
                    color,
                },
                ..base.clone()
            };
            let own = rows(&stitch_object(&square_fill(p), Point::ZERO).unwrap().runs);
            assert!(own.len() > 5, "color {color}: {}", own.len());
            assert!(own.is_disjoint(&seen), "a row of two colors");
            seen.extend(own);
        }
        assert_eq!(seen, all, "the colors together sew each row of the fill");
    }

    #[test]
    fn a_line_with_a_ripple_sews_its_copies() {
        let mut d = Design::new("t");
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::ellipse(Point::ZERO, 10.0, 10.0),
                params: RunningParams {
                    ripple: Some(emb_model::Ripple::default()),
                    ..RunningParams::default()
                },
            },
        );
        let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
        assert_eq!(s.runs.len(), 1);
        let pts = &s.runs[0].points;
        // Ten rings in to the middle: about 10 × 2π × 5 mm of line.
        assert!(pts.len() > 100, "{}", pts.len());
        assert!(pts[pts.len() - 1].length() < 1e-6);
        assert!(pts.windows(2).all(|w| w[0].distance(w[1]) <= 2.5 + 1e-6));
    }

    #[test]
    fn objects_too_large_to_sew_are_refused() {
        let fill = |size: f64, pull: f64| {
            let mut d = Design::new("t");
            let params = FillParams {
                pull_compensation_mm: pull,
                ..FillParams::default()
            };
            let outer = Path::rect(emb_geom::BBox::from_center_size(Point::ZERO, size, size));
            let id = d.add_object(
                ThreadId(0),
                ObjectKind::Fill {
                    outer,
                    holes: vec![],
                    params,
                },
            );
            stitch_object(d.object(id).unwrap(), Point::ZERO)
        };
        assert!(fill(20.0, 0.2).is_ok());
        assert_eq!(fill(1e9, 0.2), Err(StitchError::TooLarge), "a wrong coordinate");
        assert_eq!(fill(20.0, 1e9), Err(StitchError::TooLarge), "a wrong pull compensation");
        assert_eq!(fill(f64::NAN, 0.2), Err(StitchError::TooLarge));
    }

    #[test]
    fn lettering_is_measured_by_its_letters() {
        // The audit of P20: two letters 1 m high (2.2 m wide) were refused,
        // because the height counted as a margin round the letters.
        let lettering = |height: f64, width: f64, pull: f64| {
            let mut l = emb_model::Lettering::new("AB", emb_model::FontRef::new("Lato", "Bold"), height, Point::ZERO);
            let params = FillParams {
                pull_compensation_mm: pull,
                ..FillParams::default()
            };
            l.letters.push(emb_model::Letter {
                text: "AB".into(),
                parts: vec![ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(width, height))),
                    holes: vec![],
                    params,
                }],
                ..Default::default()
            });
            let mut d = Design::new("t");
            let id = d.add_object(ThreadId(0), ObjectKind::Lettering(Box::new(l)));
            too_large(d.object(id).unwrap())
        };
        assert!(!lettering(1000.0, 2200.0, 0.2));
        assert!(lettering(1000.0, 3100.0, 0.2), "larger than 3 m");
        assert!(lettering(20.0, 50.0, 1e9), "a wrong pull compensation of a letter");
    }

    #[test]
    fn cache_hits_until_the_object_changes() {
        let (mut d, id) = line_design();
        let mut c = StitchCache::default();
        let a = c.get(d.object(id).unwrap(), Point::ZERO);
        let b = c.get(d.object(id).unwrap(), Point::ZERO);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(c.stats(), (1, 1));
        if let ObjectKind::Running { params, .. } = &mut d.object_mut(id).unwrap().kind {
            params.stitch_length_mm = 2.0;
        }
        let n = c.get(d.object(id).unwrap(), Point::ZERO);
        assert!(!Arc::ptr_eq(&a, &n));
        assert_eq!(n.as_ref().as_ref().unwrap().runs[0].points.len(), 6);
    }

    #[test]
    fn underlay_weights_of_a_fill() {
        let count = |underlay: Underlay| {
            let mut d = Design::new("t");
            let id = d.add_object(
                ThreadId(0),
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(20.0, 20.0))),
                    holes: Vec::new(),
                    params: FillParams {
                        underlay,
                        ..FillParams::default()
                    },
                },
            );
            let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
            s.runs.iter().map(|r| r.points.len()).sum::<usize>()
        };
        let (none, light, auto, heavy) = (
            count(Underlay::None),
            count(Underlay::Light),
            count(Underlay::Auto),
            count(Underlay::Heavy),
        );
        assert!(
            none < light && light < auto && auto < heavy,
            "{none} {light} {auto} {heavy}"
        );
    }

    #[test]
    fn a_line_with_a_border_sews_as_a_column() {
        let (mut d, id) = line_design();
        if let ObjectKind::Running { params, .. } = &mut d.object_mut(id).unwrap().kind {
            params.border = Some(LineBorder::new(3.0, emb_model::ColumnPattern::EStitch));
        }
        let o = d.object(id).unwrap();
        // The column starts at the end nearer to the needle.
        let entry = entry_for(o, Point::new(11.0, 0.0)).expect("a border depends on the needle");
        let s = stitch_object(o, entry).unwrap();
        assert_eq!(s.runs.len(), 1);
        let pts = &s.runs[0].points;
        assert!(s.runs[0].fixed);
        assert!((pts[0].x - 10.0).abs() < 1e-6 && pts[pts.len() - 1].x.abs() < 1e-6);
        // 3 mm wide around the line (and the default pull compensation).
        let (lo, hi) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q.y), b.max(q.y)));
        assert!(lo < -1.4 && hi > 1.4 && lo > -1.8 && hi < 1.8, "{lo} {hi}");
    }

    #[test]
    fn cache_trim_keeps_recent() {
        let (mut d, id) = line_design();
        let mut c = StitchCache::default();
        for _ in 0..(EXTRA_ENTRIES + 10) {
            d.object_mut(id).unwrap();
            c.get(d.object(id).unwrap(), Point::ZERO);
        }
        c.trim(1);
        assert_eq!(c.len(), 1 + EXTRA_ENTRIES);
        // The newest entry is still there.
        let before = c.stats().0;
        c.get(d.object(id).unwrap(), Point::ZERO);
        assert_eq!(c.stats().0, before + 1);
    }
}
