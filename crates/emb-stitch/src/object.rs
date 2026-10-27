//! Stitches for one object, and the cache keyed by `(ObjectId, revision)`.
//!
//! Fill and satin objects start where the needle is (or at the start point
//! that the user set). For them the cache key also has the needle position,
//! rounded to [`ENTRY_GRID_MM`]. The object is stitched from the rounded
//! position, so the result is a function of the key only.

use std::collections::HashMap;
use std::sync::Arc;

use emb_geom::{Point, Polyline};
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
            StopReason::Function => "Stop",
            StopReason::PlaceFabric => "Put the appliqué fabric on",
            StopReason::TrimFabric => "Cut away the appliqué fabric outside the tack-down",
            StopReason::PlaceCord => "Lay the cord along the sewn line",
            StopReason::PlaceFoam => "Put the foam on (puff); tear the foam away after the puff objects",
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
        }
    }
}

/// The subpaths of a stitch object as needle points. All points stay, also
/// repeated ones; curves count as their end points.
pub fn stitch_subpaths(path: &emb_geom::Path) -> Vec<Vec<Point>> {
    use emb_geom::path::PathCmd;
    let mut out: Vec<Vec<Point>> = Vec::new();
    for cmd in &path.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => out.push(vec![p]),
            PathCmd::LineTo(p) | PathCmd::CubicTo(_, _, p) => match out.last_mut() {
                Some(s) => s.push(p),
                None => out.push(vec![p]),
            },
            PathCmd::Close => {
                if let Some(s) = out.last_mut()
                    && let Some(&first) = s.first()
                {
                    s.push(first);
                }
            }
        }
    }
    out
}

/// The stitches of one object: one or more runs in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectStitches {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StitchError {
    #[error("the object has no stitchable geometry")]
    Empty,
    #[error("the object is too large to sew (more than 3 m, or a setting that makes it so large)")]
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
        // The settings of the lettering and of each letter.
        ObjectKind::Lettering(l) => serde_json::to_value(l),
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
                    runs: runs.into_iter().map(Run::new).collect(),
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

/// Sewing state of a lettering object.
struct LetterSewer<'a> {
    l: &'a emb_model::Lettering,
    runs: Vec<Run>,
    needle: Point,
    long_gap: RunLink,
    smart: bool,
}

impl LetterSewer<'_> {
    /// Sews `part` from `from` (to `exit`). `unsewn` are the parts that sew
    /// from now on, `part` too: their stitches can cover a connection.
    fn sew(&mut self, part: &ObjectKind, from: Point, exit: Option<Point>, unsewn: &[&ObjectKind]) {
        let Ok(mut st) = stitch_kind(part, from, exit) else {
            return;
        };
        if let Some(first) = st.runs.first_mut()
            && !self.runs.is_empty()
            && let Some(&p) = first.points.first()
        {
            let short = p.distance(self.needle) <= self.l.trim_over_mm;
            let covered = || {
                let areas: Vec<Vec<Vec<Point>>> = unsewn.iter().map(|k| part_area(k)).collect();
                hidden(self.needle, p, &areas)
            };
            first.link = if short || (self.smart && covered()) {
                RunLink::Travel
            } else {
                self.long_gap
            };
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
    let smart = l.order != LetterOrder::Text;
    let mut w = LetterSewer {
        l,
        runs: Vec::new(),
        needle: entry,
        long_gap: match l.connection {
            LetterConnection::Trim => RunLink::Trim,
            LetterConnection::Jump => RunLink::Jump,
        },
        smart,
    };
    let seq = letter_sequence(l, entry);
    for (n, &li) in seq.iter().enumerate() {
        let letter = &l.letters[li];
        // Where this letter should end: near the next letter.
        let target = match seq.get(n + 1) {
            None => exit,
            Some(_) if !smart => None,
            Some(&next) => letter_box(&l.letters[next])
                .zip(letter_box(letter))
                .map(|(nb, b)| nearest_in(&nb, b.center())),
        };
        if smart {
            let plan = plan_letter(&letter.parts, w.needle, target, l.trim_over_mm, w.runs.is_empty());
            let last = plan.len().saturating_sub(1);
            for (k, &(pi, from, end)) in plan.iter().enumerate() {
                let unsewn: Vec<&ObjectKind> = plan[k..].iter().map(|&(i, _, _)| &letter.parts[i]).collect();
                let to = if k == last { end.or(target) } else { end };
                w.sew(&letter.parts[pi], from, to, &unsewn);
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
            w.sew(part, needle, if last { exit } else { None }, &[]);
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

/// Needle paths of a fill: underlay, then the top rows. Underlay tatami
/// angles are relative to the fill angle.
fn fill_runs(shape: &Shape, p: &FillParams, entry: Point, exit: Option<Point>) -> Vec<Vec<Point>> {
    if shape.is_empty() {
        return Vec::new();
    }
    let travel = Travel::new(shape);
    let mut sewer = Sewer::new(&travel, TRAVEL_LEN_MM);
    sewer.edge_only = p.trapunto;
    let layers = match &p.underlay {
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
    };
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
    if let Some(split) = row_split(shape, p) {
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
        crate::curves::sew(shape, p, entry, &mut sewer);
    }
    sewer.finish()
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
