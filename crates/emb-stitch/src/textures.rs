//! Texture fill types (P5 Milestone 3): column types across a shape
//! (satin, zigzag, E stitch, raised satin, coil), square, double square,
//! string, cross stitch, stipple and laydown.
//!
//! Not a port. Each type follows what public embroidery documentation says
//! it looks like, and common digitizing practice:
//!
//! - Column types across a shape: lines at the fill angle cut the shape
//!   (holes too) into sections, as the rows of tatami do. In a section each
//!   line crosses the shape one time, so the two sides of the section are
//!   the rails of a column and no stitch goes outside the shape or over
//!   another section. Rungs on the lines keep the stitches at the fill
//!   angle. The sections are sewn nearest first, with travel inside the
//!   shape. (Before, the outline was cut into two halves only, so in a
//!   shape with bends or holes the stitches fanned out, went outside and
//!   were cut into small pieces with travel between them.)
//! - With a direction line, the column types go across the line at its
//!   angle, so they turn with it: lines across the shape every
//!   [`GUIDED_STEP_MM`] along it (`emb_geom::spine::cross_lines`) are the
//!   rungs of sections, and a gap (the line crosses a hole) starts a new
//!   section. The parts of the shape that the lines do not reach get
//!   sections at the angle of the whole line.
//! - Square: tatami rows with the needle points in line, so the stitches
//!   make a grid of even lengths. Double square: two such layers, the
//!   second turned (90° by default).
//! - String: points on the edge one spacing apart; straight lines from
//!   each point to the point a third of the way round, and back to the next
//!   point. The lines overlap in curves, as in string art.
//! - Cross stitch: crosses on a grid that starts at the design origin, so
//!   the crosses of all objects line up. Only whole cells inside the shape
//!   get a cross. Each row is sewn forward with one leg of each cross and
//!   back with the other leg; the needle goes along the cell edges between
//!   the legs.
//! - Stipple: a meander that fills the shape and never crosses itself. The
//!   cells of a grid inside the shape get a random spanning tree (a fixed
//!   seed, so the result is the same on each run); the line goes round the
//!   tree, one spacing from it, and its corners are rounded.
//! - Laydown: one or two layers of open tatami rows, the second turned.
//! - Lace mesh (P11 Milestone 5): two layers of open rows, the second
//!   turned; each row goes there and back on the same needle points, so
//!   it is two threads. The rows of the second layer cross the first, so
//!   the net holds together when the stabilizer washes away.

use emb_geom::region::{self, Join, Region};
use emb_geom::{Path, Point, Polyline, Transform};
use emb_model::{
    ColumnPattern, CrossStyle, DirectionLine, FillParams, FillPattern, ObjectKind, RunStyle, SatinGeometry,
    SatinParams, Underlay,
};

use crate::curves::{clip, sew_line};
use crate::fill::{Tatami, sew_tatami};
use crate::object::FILL_STAGGERS;
use crate::running;
use crate::travel::{Sewer, Shape};

/// Smallest spacing that the generator accepts, in mm.
const MIN_SPACING_MM: f64 = 0.2;
/// Smallest cross stitch cell, in mm.
const MIN_CELL_MM: f64 = 1.0;
/// Most cells of a stipple or cross stitch grid on one side.
const MAX_CELLS: i64 = 2000;
/// Seed of the stipple tree, so a stipple is the same on each run.
const STIPPLE_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Sews the top stitching of a texture fill.
pub fn sew(shape: &Shape, p: &FillParams, entry: Point, sewer: &mut Sewer) {
    let spacing = p.spacing_mm.max(MIN_SPACING_MM);
    let len = p.stitch_length_mm.max(running::MIN_SETTING_MM);
    let rows = |angle_deg: f64, staggers: u32, pull: f64| Tatami {
        spacing_mm: spacing,
        angle_deg,
        stitch_len_mm: len,
        pull_comp_mm: pull,
        staggers,
        split: crate::fill::RowSplit::default(),
    };
    match &p.pattern {
        FillPattern::Columns { column } => columns(shape, p, *column, None, entry, sewer),
        FillPattern::FlexiSplit { pattern, scale } => columns(
            shape,
            p,
            ColumnPattern::ProgramSplit { scale: *scale },
            Some(pattern.clone()),
            entry,
            sewer,
        ),
        FillPattern::Square => sew_tatami(shape, &rows(p.angle_deg, 1, p.pull_compensation_mm), entry, None, sewer),
        FillPattern::DoubleSquare { angle_deg } => {
            sew_tatami(shape, &rows(p.angle_deg, 1, p.pull_compensation_mm), entry, None, sewer);
            let at = sewer.needle().unwrap_or(entry);
            let second = rows(p.angle_deg + angle_deg, 1, p.pull_compensation_mm);
            sew_tatami(shape, &second, at, None, sewer);
        }
        FillPattern::Laydown { layers, angle_deg } => {
            sew_tatami(shape, &rows(p.angle_deg, FILL_STAGGERS, 0.0), entry, None, sewer);
            if *layers >= 2 {
                let at = sewer.needle().unwrap_or(entry);
                sew_tatami(
                    shape,
                    &rows(p.angle_deg + angle_deg, FILL_STAGGERS, 0.0),
                    at,
                    None,
                    sewer,
                );
            }
        }
        FillPattern::Mesh { angle_deg } => {
            // Two layers of rows that go there and back (free-standing lace).
            for angle in [p.angle_deg, p.angle_deg + angle_deg] {
                let t = Tatami {
                    split: crate::fill::RowSplit {
                        double: true,
                        ..crate::fill::RowSplit::default()
                    },
                    ..rows(angle, FILL_STAGGERS, 0.0)
                };
                let at = sewer.needle().unwrap_or(entry);
                sew_tatami(shape, &t, at, None, sewer);
            }
        }
        FillPattern::String => string(shape, spacing, len, sewer),
        FillPattern::CrossStitch { cell_mm, style, passes } => cross(shape, *cell_mm, *style, *passes, entry, sewer),
        FillPattern::Stipple { style } => stipple(shape, spacing, len, *style, entry, sewer),
        _ => {}
    }
}

fn poly(v: &[Point]) -> Path {
    v[1..].iter().fold(Path::new().move_to(v[0]), |p, q| p.line_to(*q))
}

/// The ring with a vertex in the middle of its points of least (or
/// greatest) projection on `n`, and the index of that vertex. A flat edge
/// across `n` is cut in its middle, not at a corner.
fn with_middle(ring: &[Point], n: Point, least: bool) -> Option<(Vec<Point>, usize)> {
    let proj = |p: Point| if least { p.dot(n) } else { -p.dot(n) };
    let best = ring.iter().map(|p| proj(*p)).fold(f64::INFINITY, f64::min);
    let scale = ring.iter().map(|p| p.length()).fold(1.0, f64::max);
    let on = |i: usize| proj(ring[i % ring.len()]) <= best + 1e-9 * scale;
    let len = ring.len();
    // The first point of the run of points with the extreme projection.
    let first = (0..len)
        .find(|&i| on(i) && !on(i + len - 1))
        .or_else(|| (0..len).find(|&i| on(i)))?;
    let mut run = vec![first];
    while run.len() < len && on(run[run.len() - 1] + 1) {
        run.push((run[run.len() - 1] + 1) % len);
    }
    if run.len() == 1 {
        return Some((ring.to_vec(), first));
    }
    // The middle of the run, by length.
    let total: f64 = run.windows(2).map(|w| ring[w[0]].distance(ring[w[1]])).sum();
    let mut left = total / 2.0;
    for w in run.windows(2) {
        let d = ring[w[0]].distance(ring[w[1]]);
        if left <= d {
            let mid = ring[w[0]].lerp(ring[w[1]], if d > 0.0 { left / d } else { 0.0 });
            let mut out = ring.to_vec();
            out.insert(w[0] + 1, mid);
            return Some((out, w[0] + 1));
        }
        left -= d;
    }
    Some((ring.to_vec(), first))
}

/// The two halves of a ring between its points with the least and the
/// greatest projection on `n`, both from the least one. A flat edge across
/// `n` is cut in its middle.
pub(crate) fn halves(ring: &[Point], n: Point) -> Option<(Vec<Point>, Vec<Point>)> {
    if ring.len() < 3 {
        return None;
    }
    let (first, hi) = with_middle(ring, n, false)?;
    let (ring, lo) = with_middle(&first, n, true)?;
    // A vertex added at `lo` moves the vertices after it by one.
    let hi = if ring.len() > first.len() && lo <= hi {
        hi + 1
    } else {
        hi
    };
    if lo == hi {
        return None;
    }
    let len = ring.len();
    let walk = |step: usize| {
        let mut v = vec![ring[lo]];
        let mut i = lo;
        while i != hi {
            i = (i + step) % len;
            v.push(ring[i]);
        }
        v
    };
    Some((walk(1), walk(len - 1)))
}

/// The scan lines that cut a shape into column sections are this far
/// apart, in mm. The rails go through their ends, and each line is a rung.
const SECTION_ROW_MM: f64 = 0.25;

/// A column section: its geometry and the middles of its first and last
/// lines (inside the shape).
pub(crate) struct Section {
    pub geometry: SatinGeometry,
    pub ends: (Point, Point),
}

/// The column sections of `shape` for stitches at `angle_deg` (see the
/// module comment), in design coordinates. `spacing` is the stitch spacing.
pub(crate) fn column_sections(shape: &Shape, angle_deg: f64, spacing: f64) -> Vec<Section> {
    let to_rows = Transform::rotate_deg(-angle_deg);
    let from_rows = Transform::rotate_deg(angle_deg);
    let rings: Vec<Vec<Point>> = shape
        .rings()
        .map(|r| r.iter().map(|p| to_rows.apply(*p)).collect())
        .collect();
    let h = SECTION_ROW_MM;
    let back = |u: f64, v: f64| from_rows.apply(Point::new(u, v));
    let rows = crate::fill::row_segments(&rings, h);
    // Whether row `k` has a segment that overlaps `u0..u1` (another section).
    let by_k: std::collections::HashMap<i64, &Vec<crate::fill::Seg>> =
        rows.iter().filter_map(|r| r.first().map(|s| (s.k, r))).collect();
    let touches = |k: i64, s: &crate::fill::Seg| {
        by_k.get(&k)
            .is_some_and(|row| row.iter().any(|t| t.u0 < s.u1 && s.u0 < t.u1))
    };
    // A section goes half a row past its first and last line. Where another
    // section follows, it goes a quarter spacing more: the first and the
    // last stitch of a satin are slanted, so the two sections overlap by
    // that much and leave no gap.
    let past = |k: i64, s: &crate::fill::Seg| h / 2.0 + if touches(k, s) { spacing / 4.0 } else { 0.0 };
    crate::fill::sections(rows.clone())
        .into_iter()
        .map(|sec| {
            let (first, last) = (sec[0], sec[sec.len() - 1]);
            // The ends past the first and the last line follow the slope of
            // the sides, so they stay on the edge (and meet at a point).
            let past_end = |s: crate::fill::Seg, next: Option<&crate::fill::Seg>, e: f64, dir: f64| {
                let (mut u0, mut u1) = (s.u0, s.u1);
                if let Some(n) = next {
                    let f = e / (n.v - s.v).abs().max(1e-9);
                    u0 -= (n.u0 - s.u0) * f;
                    u1 -= (n.u1 - s.u1) * f;
                }
                if u0 > u1 {
                    let m = (u0 + u1) / 2.0;
                    (u0, u1) = (m, m);
                }
                (back(u0, s.v + dir * e), back(u1, s.v + dir * e))
            };
            let (a0, b0) = past_end(first, sec.get(1), past(first.k - 1, &first), -1.0);
            let (a1, b1) = past_end(
                last,
                sec.len().checked_sub(2).and_then(|i| sec.get(i)),
                past(last.k + 1, &last),
                1.0,
            );
            let mut a = vec![a0];
            let mut b = vec![b0];
            let mut rungs = Vec::new();
            for s in &sec {
                let (pa, pb) = (back(s.u0, s.v), back(s.u1, s.v));
                a.push(pa);
                b.push(pb);
                rungs.push((pa, pb));
            }
            a.push(a1);
            b.push(b1);
            Section {
                ends: (
                    back((first.u0 + first.u1) / 2.0, first.v),
                    back((last.u0 + last.u1) / 2.0, last.v),
                ),
                geometry: SatinGeometry {
                    rail_a: poly(&a),
                    rail_b: poly(&b),
                    rungs,
                },
            }
        })
        .collect()
}

/// Lines across the shape along a direction line are this far apart, in mm.
const GUIDED_STEP_MM: f64 = 0.5;
/// Parts of the shape that the lines of a direction line do not reach, and
/// that are smaller than this, are not sewn (slivers along the edge), in mm².
const MIN_LEFT_MM2: f64 = 1.0;

/// The column sections along a direction line (see the module comment).
pub(crate) fn guided_sections(shape: &Shape, dir: &DirectionLine, spacing: f64) -> Vec<Section> {
    let mut out = Vec::new();
    let mut covered: Vec<Region> = Vec::new();
    let mut flush = |run: &mut Vec<(Point, Point)>, out: &mut Vec<Section>| {
        if run.len() >= 2 {
            let a: Vec<Point> = run.iter().map(|l| l.0).collect();
            let b: Vec<Point> = run.iter().map(|l| l.1).collect();
            let mut ring = a.clone();
            ring.extend(b.iter().rev());
            covered.push(Region::new(ring, Vec::new()));
            out.push(Section {
                ends: (
                    run[0].0.midpoint(run[0].1),
                    run[run.len() - 1].0.midpoint(run[run.len() - 1].1),
                ),
                geometry: SatinGeometry {
                    rail_a: poly(&a),
                    rail_b: poly(&b),
                    rungs: std::mem::take(run),
                },
            });
        }
        run.clear();
    };
    for region in &shape.regions {
        let mut rings = vec![region.outer.clone()];
        rings.extend(region.holes.iter().cloned());
        let mut run: Vec<(Point, Point)> = Vec::new();
        for l in emb_geom::spine::cross_lines(&rings, &dir.points, dir.angle_deg, GUIDED_STEP_MM) {
            if let Some(&last) = run.last()
                && l.0.midpoint(l.1).distance(last.0.midpoint(last.1)) > 4.0 * GUIDED_STEP_MM
            {
                flush(&mut run, &mut out);
            }
            run.push(l);
        }
        flush(&mut run, &mut out);
    }
    let left: Vec<Region> = region::difference(&shape.regions, &region::union(&covered, &[]))
        .into_iter()
        .filter(|r| r.area() >= MIN_LEFT_MM2 && !region::offset(std::slice::from_ref(r), -0.2, Join::Miter).is_empty())
        .collect();
    if !left.is_empty()
        && let (Some(first), Some(last)) = (dir.points.first(), dir.points.last())
    {
        let chord = *last - *first;
        let angle = emb_geom::math::to_degrees(emb_geom::math::atan2(chord.y, chord.x)) + dir.angle_deg;
        out.extend(column_sections(&Shape { regions: left }, angle, spacing));
    }
    out
}

fn columns(
    shape: &Shape,
    p: &FillParams,
    column: ColumnPattern,
    split_pattern: Option<emb_model::SplitPattern>,
    entry: Point,
    sewer: &mut Sewer,
) {
    let params = SatinParams {
        spacing_mm: p.spacing_mm,
        pull_compensation_mm: p.pull_compensation_mm,
        short_stitches: column.is_satin(),
        underlay: Underlay::None,
        split_mm: p.stitch_length_mm,
        pattern: column,
        liquid: false,
        motif: None,
        split_pattern,
        accordion: p.accordion.clone(),
        jagged_mm: p.jagged_mm,
        ..SatinParams::default()
    };
    let spacing = p.spacing_mm.max(MIN_SPACING_MM);
    let mut todo = match &p.direction {
        Some(dir) if dir.points.len() >= 2 => guided_sections(shape, dir, spacing),
        _ => column_sections(shape, p.angle_deg, spacing),
    };
    sew_sections(&mut todo, &params, entry, sewer);
}

/// Sews column sections, each time the one with an end nearest to the
/// needle, with travel inside the shape to its start.
pub(crate) fn sew_sections(todo: &mut Vec<Section>, params: &SatinParams, entry: Point, sewer: &mut Sewer) {
    while !todo.is_empty() {
        let at = sewer.needle().unwrap_or(entry);
        // Ties go to the first section, so the result is deterministic.
        let mut best = (f64::INFINITY, 0);
        for (i, sec) in todo.iter().enumerate() {
            let d = sec.ends.0.distance(at).min(sec.ends.1.distance(at));
            if d < best.0 {
                best = (d, i);
            }
        }
        let sec = todo.remove(best.1);
        let Some(pts) = crate::satin::satin_with(&sec.geometry, params, at, None, true) else {
            continue;
        };
        // The first needle point can be a little outside (pull
        // compensation), so the travel goes to the middle of the first line,
        // inside the shape. The stitch from there lies under the first stitch.
        let near = if sec.ends.0.distance(at) <= sec.ends.1.distance(at) {
            sec.ends.0
        } else {
            sec.ends.1
        };
        sewer.go_to(near);
        sewer.stitch(pts);
        // The same for the travel to the next section: it starts in the
        // middle of the last line, along the last stitch.
        if !todo.is_empty() {
            let far = if near == sec.ends.0 { sec.ends.1 } else { sec.ends.0 };
            sewer.stitch([far]);
        }
    }
}

/// Points on a closed ring, `step` apart along it.
fn resample(ring: &[Point], step: f64) -> Vec<Point> {
    let mut closed = ring.to_vec();
    closed.push(ring[0]);
    running::running_points(
        &Polyline {
            points: closed,
            closed: false,
        },
        step,
    )
    .into_iter()
    .skip(1)
    .collect()
}

fn string(shape: &Shape, spacing: f64, len: f64, sewer: &mut Sewer) {
    for region in &shape.regions {
        let pts = resample(&region.outer, spacing);
        let n = pts.len();
        if n < 6 {
            continue;
        }
        let k = n / 3;
        for i in 0..n {
            for chord in [[pts[i], pts[(i + k) % n]], [pts[(i + k) % n], pts[(i + 1) % n]]] {
                let line = Polyline {
                    points: chord.to_vec(),
                    closed: false,
                };
                for piece in clip(&running::running_points(&line, len), shape) {
                    sew_line(sewer, &piece);
                }
            }
        }
    }
}

/// Grid cells of `size` (with the indices of their top left corner) whose
/// test points are all inside the shape, row by row.
fn cells(shape: &Shape, size: f64, test: &[(f64, f64)]) -> Vec<(i64, Vec<i64>)> {
    let pts: Vec<Point> = shape.rings().flatten().copied().collect();
    let Some(b) = emb_geom::BBox::from_points(pts) else {
        return Vec::new();
    };
    let (i0, i1) = ((b.min.x / size).floor() as i64, (b.max.x / size).ceil() as i64);
    let (j0, j1) = ((b.min.y / size).floor() as i64, (b.max.y / size).ceil() as i64);
    if i1 - i0 > MAX_CELLS || j1 - j0 > MAX_CELLS {
        return Vec::new();
    }
    let inside = |p: Point| shape.regions.iter().any(|r| r.contains(p));
    (j0..j1)
        .map(|j| {
            let row = (i0..i1)
                .filter(|&i| {
                    test.iter()
                        .all(|&(fx, fy)| inside(Point::new((i as f64 + fx) * size, (j as f64 + fy) * size)))
                })
                .collect();
            (j, row)
        })
        .collect()
}

/// Leg `a` to `b`, `passes` times (an odd number: it ends at `b`).
fn leg(out: &mut Vec<Point>, a: Point, b: Point, passes: u32) {
    out.push(a);
    out.push(b);
    for _ in 0..passes.max(1) / 2 {
        out.push(a);
        out.push(b);
    }
}

fn cross(shape: &Shape, cell: f64, style: CrossStyle, passes: u32, entry: Point, sewer: &mut Sewer) {
    let c = cell.max(MIN_CELL_MM);
    let corners = [(0.02, 0.02), (0.98, 0.02), (0.98, 0.98), (0.02, 0.98)];
    let mut runs: Vec<Vec<Point>> = Vec::new();
    for (j, row) in cells(shape, c, &corners) {
        // Runs of cells next to each other.
        let mut groups: Vec<Vec<i64>> = Vec::new();
        for i in row {
            match groups.last_mut() {
                Some(g) if *g.last().expect("not empty") + 1 == i => g.push(i),
                _ => groups.push(vec![i]),
            }
        }
        for g in groups {
            let at = |i: i64, fx: f64, fy: f64| Point::new((i as f64 + fx) * c, (j as f64 + fy) * c);
            let mut path = Vec::new();
            match style {
                CrossStyle::Full | CrossStyle::Diagonal => {
                    // Forward: the "/" legs; between them, down the right edge.
                    for &i in &g {
                        leg(&mut path, at(i, 0.0, 1.0), at(i, 1.0, 0.0), passes);
                        path.push(at(i, 1.0, 1.0));
                    }
                    if style == CrossStyle::Full {
                        // Back: the "\" legs; between them, down the left edge.
                        for &i in g.iter().rev() {
                            leg(&mut path, at(i, 1.0, 1.0), at(i, 0.0, 0.0), passes);
                            path.push(at(i, 0.0, 1.0));
                        }
                    }
                }
                CrossStyle::Upright => {
                    // Forward: the level legs, one line across the run.
                    let (first, last) = (g[0], g[g.len() - 1]);
                    leg(&mut path, at(first, 0.0, 0.5), at(last, 1.0, 0.5), passes);
                    // Back: the upright legs, down and up in turn, along the cell edges.
                    for (k, &i) in g.iter().rev().enumerate() {
                        let (top, bottom) = (at(i, 0.5, 0.0), at(i, 0.5, 1.0));
                        if k % 2 == 0 {
                            leg(&mut path, top, bottom, passes);
                        } else {
                            leg(&mut path, bottom, top, passes);
                        }
                    }
                }
            }
            path.dedup();
            runs.push(path);
        }
    }
    // The rows in turn, each from the end nearer to the needle.
    let mut at = entry;
    for mut r in runs {
        if r.len() < 2 {
            continue;
        }
        if r[r.len() - 1].distance(at) < r[0].distance(at) {
            r.reverse();
        }
        at = r[r.len() - 1];
        sew_line(sewer, &r);
    }
}

/// Lines count as columns this wide for the laydown area, in mm.
const LAYDOWN_LINE_MM: f64 = 1.0;

/// The areas of an object kind as regions: fills and appliqué pieces with
/// their holes, columns between their rails, lines as narrow columns, and
/// the parts of lettering.
fn kind_regions(k: &ObjectKind) -> Vec<Region> {
    let flat = |p: &Path| p.flatten(0.1);
    let fill = |outer: &Path, holes: &[Path]| {
        let holes: Vec<Polyline> = holes.iter().flat_map(flat).collect();
        Shape::new(&flat(outer), &holes).regions
    };
    let column = |g: &SatinGeometry| {
        let (a, b) = (flat(&g.rail_a), flat(&g.rail_b));
        match (a.first(), b.first()) {
            // A ring column: the area between two closed rails.
            (Some(a), Some(b)) if a.closed && b.closed => {
                let (outer, inner) = if Region::new(a.points.clone(), Vec::new()).area()
                    >= Region::new(b.points.clone(), Vec::new()).area()
                {
                    (a, b)
                } else {
                    (b, a)
                };
                Shape::new(std::slice::from_ref(outer), std::slice::from_ref(inner)).regions
            }
            (Some(a), Some(b)) => {
                let ring = Polyline {
                    points: a.points.iter().chain(b.points.iter().rev()).copied().collect(),
                    closed: true,
                };
                Shape::new(&[ring], &[]).regions
            }
            _ => Vec::new(),
        }
    };
    match k {
        ObjectKind::Fill { outer, holes, .. } => fill(outer, holes),
        ObjectKind::Satin { geometry, .. } => column(geometry),
        ObjectKind::Running { path, params } => {
            let w = params.border.as_ref().map_or(LAYDOWN_LINE_MM, |b| b.width_mm);
            flat(path)
                .into_iter()
                .flat_map(|pl| {
                    let Some((&first, rest)) = pl.points.split_first() else {
                        return Vec::new();
                    };
                    let mut line = rest.iter().fold(Path::new().move_to(first), |p, q| p.line_to(*q));
                    if pl.closed {
                        line = line.close();
                    }
                    column(&SatinGeometry::from_centerline(&line, w))
                })
                .collect()
        }
        ObjectKind::Applique { pieces, .. } => pieces.iter().flat_map(|p| fill(&p.outer, &p.holes)).collect(),
        ObjectKind::Lettering(l) => l.parts().flat_map(kind_regions).collect(),
        ObjectKind::CrossStitch(x) => cross_regions(x),
        ObjectKind::Stitches { .. } | ObjectKind::Vector { .. } | ObjectKind::Bling(_) | ObjectKind::Sketch(_) => {
            Vec::new()
        }
    }
}

/// The cells of a cross-stitch object as regions: one for each group of
/// cells next to each other in a row.
fn cross_regions(x: &emb_model::CrossStitch) -> Vec<Region> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < x.cells.len() {
        let (row, a) = (x.cells[i].row, x.cells[i].col);
        let mut b = a;
        while i + 1 < x.cells.len() && x.cells[i + 1].row == row && x.cells[i + 1].col == b + 1 {
            i += 1;
            b += 1;
        }
        let (r, a, b) = (f64::from(row), f64::from(a), f64::from(b + 1));
        let ring = vec![x.at(a, r), x.at(b, r), x.at(b, r + 1.0), x.at(a, r + 1.0)];
        out.extend(
            Shape::new(
                &[Polyline {
                    points: ring,
                    closed: true,
                }],
                &[],
            )
            .regions,
        );
        i += 1;
    }
    out
}

/// The area of a laydown under `kinds`: their areas joined and moved out
/// by `margin_mm`, with no holes, as outer rings.
pub fn laydown_area(kinds: &[&ObjectKind], margin_mm: f64) -> Vec<Vec<Point>> {
    let regions: Vec<Region> = kinds.iter().flat_map(|k| kind_regions(k)).collect();
    let joined = region::union(&regions, &[]);
    region::offset(&joined, margin_mm, Join::Round)
        .into_iter()
        .filter(|r| r.area() > 1.0)
        .map(|r| r.outer)
        .collect()
}

/// A small fast random number generator (xorshift), with a fixed seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// A corner node of a stipple cell: the cell and the corner (0 top left,
/// 1 top right, 2 bottom right, 3 bottom left).
type Node = ((i64, i64), u8);

/// The meander round a spanning tree of the grid cells, as closed loops
/// (one for each part of the grid that is connected).
fn meanders(shape: &Shape, spacing: f64) -> Vec<Vec<Point>> {
    use std::collections::{HashMap, HashSet};
    let size = 2.0 * spacing;
    // The four corner points of the line in a cell must be inside the shape,
    // a quarter spacing in from its edge.
    let inner = shape.inset(spacing * 0.25);
    let test = [(0.25, 0.25), (0.75, 0.25), (0.75, 0.75), (0.25, 0.75)];
    let set: HashSet<(i64, i64)> = cells(&inner, size, &test)
        .into_iter()
        .flat_map(|(j, row)| row.into_iter().map(move |i| (i, j)))
        .collect();
    let mut order: Vec<(i64, i64)> = set.iter().copied().collect();
    order.sort_unstable_by_key(|&(i, j)| (j, i));
    // A random spanning tree of each connected part (depth first).
    let mut rng = Rng(STIPPLE_SEED);
    let mut seen: HashSet<(i64, i64)> = HashSet::new();
    let mut tree: HashSet<((i64, i64), (i64, i64))> = HashSet::new();
    for &start in &order {
        if !seen.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        while let Some(&cur) = stack.last() {
            let mut next: Vec<(i64, i64)> = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dy)| (cur.0 + dx, cur.1 + dy))
                .filter(|c| set.contains(c) && !seen.contains(c))
                .collect();
            if next.is_empty() {
                stack.pop();
                continue;
            }
            let pick = next.swap_remove((rng.next() % next.len() as u64) as usize);
            seen.insert(pick);
            tree.insert((cur.min(pick), cur.max(pick)));
            stack.push(pick);
        }
    }
    let joined = |a: (i64, i64), b: (i64, i64)| tree.contains(&(a.min(b), a.max(b)));
    // Corner nodes of each cell: 0 top left, 1 top right, 2 bottom right, 3 bottom left.
    let mut adj: HashMap<Node, Vec<Node>> = HashMap::new();
    let mut link = |a: Node, b: Node| {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    };
    for &c in &order {
        let (i, j) = c;
        let up = (i, j - 1);
        if joined(c, up) {
            link((c, 0), (up, 3));
            link((c, 1), (up, 2));
        } else {
            link((c, 0), (c, 1));
        }
        let left = (i - 1, j);
        if joined(c, left) {
            link((c, 0), (left, 1));
            link((c, 3), (left, 2));
        } else {
            link((c, 0), (c, 3));
        }
        if !joined(c, (i + 1, j)) {
            link((c, 1), (c, 2));
        }
        if !joined(c, (i, j + 1)) {
            link((c, 3), (c, 2));
        }
    }
    let pos = |((i, j), k): Node| {
        let (fx, fy) = test[k as usize];
        Point::new((i as f64 + fx) * size, (j as f64 + fy) * size)
    };
    let mut done: HashSet<Node> = HashSet::new();
    let mut loops = Vec::new();
    for &c in &order {
        let start = (c, 0u8);
        if done.contains(&start) {
            continue;
        }
        let mut lp = Vec::new();
        let (mut prev, mut cur) = (start, start);
        loop {
            done.insert(cur);
            lp.push(pos(cur));
            let Some(n) = adj.get(&cur) else { break };
            // The neighbor that is not the node before (any one at the start).
            let next = if n[0] != prev || cur == prev || n.len() < 2 {
                n[0]
            } else {
                n[1]
            };
            if next == start || done.contains(&next) {
                break;
            }
            prev = cur;
            cur = next;
        }
        if lp.len() >= 4 {
            loops.push(lp);
        }
    }
    loops
}

/// Rounds the corners of a closed ring (four steps of corner cutting), so
/// the meander has soft curves.
fn round_corners(mut ring: Vec<Point>) -> Vec<Point> {
    for _ in 0..4 {
        let n = ring.len();
        ring = (0..n)
            .flat_map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                [a.lerp(b, 0.25), a.lerp(b, 0.75)]
            })
            .collect();
    }
    ring
}

fn stipple(shape: &Shape, spacing: f64, len: f64, style: RunStyle, entry: Point, sewer: &mut Sewer) {
    let mut at = entry;
    for lp in meanders(shape, spacing) {
        let mut ring = round_corners(lp);
        let start = (0..ring.len())
            .min_by(|&i, &j| ring[i].distance(at).total_cmp(&ring[j].distance(at)))
            .unwrap_or(0);
        ring.rotate_left(start);
        ring.push(ring[0]);
        let line = Polyline {
            points: ring,
            closed: false,
        };
        let pts = match style {
            RunStyle::Backstitch => running::backstitch(&running::running_points(&line, len / 2.0)),
            RunStyle::Stemstitch => running::stemstitch(&running::running_points(&line, len / 2.0)),
            _ => running::running_points(&line, len),
        };
        at = pts.last().copied().unwrap_or(at);
        sew_line(sewer, &pts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::travel::Travel;

    fn rect(w: f64, h: f64) -> Polyline {
        Polyline {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(w, 0.0),
                Point::new(w, h),
                Point::new(0.0, h),
            ],
            closed: true,
        }
    }

    fn sewn(shape: &Shape, pattern: FillPattern) -> Vec<Vec<Point>> {
        let travel = Travel::new(shape);
        let mut sewer = Sewer::new(&travel, 3.0);
        let p = FillParams {
            angle_deg: 0.0,
            ..FillParams::default().with_pattern(pattern)
        };
        sew(shape, &p, Point::ZERO, &mut sewer);
        sewer.finish()
    }

    fn inside(shape: &Shape, runs: &[Vec<Point>]) -> bool {
        let grown = shape.inset(-0.05);
        runs.iter()
            .flatten()
            .all(|q| grown.regions.iter().any(|r| r.contains(*q)))
    }

    fn texture_types() -> Vec<FillPattern> {
        FillPattern::all()
            .into_iter()
            .filter(|p| {
                matches!(
                    p,
                    FillPattern::Columns { .. }
                        | FillPattern::Square
                        | FillPattern::DoubleSquare { .. }
                        | FillPattern::String
                        | FillPattern::CrossStitch { .. }
                        | FillPattern::Stipple { .. }
                        | FillPattern::Laydown { .. }
                )
            })
            .collect()
    }

    #[test]
    fn a_lace_mesh_is_two_layers_of_double_rows() {
        let shape = Shape::new(&[rect(30.0, 20.0)], &[]);
        let runs = sewn(&shape, FillPattern::Mesh { angle_deg: 90.0 });
        assert!(inside(&shape, &runs));
        let stitches: Vec<(Point, Point)> = runs.iter().flat_map(|r| r.windows(2)).map(|w| (w[0], w[1])).collect();
        // Level rows (the fill angle 0°) and upright rows (turned 90°).
        let level = stitches
            .iter()
            .filter(|(a, b)| (a.y - b.y).abs() < 1e-9 && (a.x - b.x).abs() > 0.5)
            .count();
        let upright = stitches
            .iter()
            .filter(|(a, b)| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() > 0.5)
            .count();
        assert!(level > 50 && upright > 50, "{level} {upright}");
        // Each stitch of a row is sewn there and back: the same two needle
        // points again the other way.
        let key = |p: Point| ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64);
        let set: std::collections::HashSet<_> = stitches.iter().map(|(a, b)| (key(*a), key(*b))).collect();
        let row_stitches: Vec<&(Point, Point)> = stitches
            .iter()
            .filter(|(a, b)| (a.y - b.y).abs() < 1e-9 && a.y > 1.0 && a.y < 19.0 && (a.x - b.x).abs() > 0.5)
            .collect();
        assert!(row_stitches.iter().all(|(a, b)| set.contains(&(key(*b), key(*a)))));
        // Rows 1.6 mm apart: an open net.
        let mut ys: Vec<i64> = row_stitches.iter().map(|(a, _)| (a.y * 10.0).round() as i64).collect();
        ys.sort_unstable();
        ys.dedup();
        assert!(ys.windows(2).all(|w| w[1] - w[0] == 16), "{ys:?}");
    }

    #[test]
    fn every_type_stays_inside() {
        let shape = Shape::new(&[rect(30.0, 20.0)], &[]);
        for pattern in texture_types() {
            let runs = sewn(&shape, pattern.clone());
            let n: usize = runs.iter().map(Vec::len).sum();
            // Zigzag at 1.5 mm over 20 mm: about 27 needle points.
            assert!(n > 20, "{pattern:?}: {n}");
            assert!(inside(&shape, &runs), "{pattern:?}");
        }
    }

    #[test]
    fn satin_fill_goes_across_at_the_angle() {
        let shape = Shape::new(&[rect(30.0, 20.0)], &[]);
        let runs = sewn(
            &shape,
            FillPattern::Columns {
                column: ColumnPattern::Satin,
            },
        );
        // At 0° the stitches go from the left side to the right side: long
        // level stitches, split at 7 mm.
        let level = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| (w[1] - w[0]).x.abs() > 5.0 && (w[1] - w[0]).y.abs() < 0.5)
            .count();
        assert!(level > 100, "{level}");
    }

    /// All needle points are in the shape or less than 0.15 mm out of it.
    /// (Each section of a column fill goes half a scan line past its last
    /// line; at a round tip that can be a little outside.)
    fn near_inside(shape: &Shape, runs: &[Vec<Point>]) -> bool {
        let grown = shape.inset(-0.15);
        runs.iter()
            .flatten()
            .all(|q| grown.regions.iter().any(|r| r.contains(*q)))
    }

    /// A C shape round (0, 0): radii 20 and 30, open on the right.
    fn c_shape() -> Shape {
        let arc = |r: f64, from: f64, to: f64| -> Vec<Point> {
            (0..=56)
                .map(|i| {
                    let a = (from + (to - from) * f64::from(i) / 56.0).to_radians();
                    Point::new(r * a.cos(), r * a.sin())
                })
                .collect()
        };
        let mut points = arc(30.0, 40.0, 320.0);
        points.extend(arc(20.0, 320.0, 40.0));
        Shape::new(&[Polyline { points, closed: true }], &[])
    }

    #[test]
    fn satin_fill_does_not_go_outside_a_bent_shape() {
        // At 0° the stitches go level, across the gap of the C. Before, the
        // two halves of the outline were the rails, and stitches went over
        // the gap and were cut into pieces.
        let shape = c_shape();
        let runs = sewn(
            &shape,
            FillPattern::Columns {
                column: ColumnPattern::Satin,
            },
        );
        // (Travel along the edge is on the edge: 0.05 mm out counts as in.)
        let grown = shape.inset(-0.05);
        let outside = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| !grown.regions.iter().any(|g| g.contains(w[0].midpoint(w[1]))))
            .count();
        assert_eq!(outside, 0);
        assert!(near_inside(&shape, &runs));
    }

    #[test]
    fn satin_fill_spacing_is_across_the_stitches() {
        // A long, flat parallelogram: the sides are almost along the level
        // stitches, 10 mm long for 2 mm across. About 2 / 0.2 stitches, not
        // 10 / 0.2.
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(50.0, 2.0),
            Point::new(10.0, 2.0),
        ];
        let shape = Shape::new(&[Polyline { points, closed: true }], &[]);
        let p = FillParams {
            angle_deg: 0.0,
            stitch_length_mm: 100.0,
            ..FillParams::default().with_pattern(FillPattern::Columns {
                column: ColumnPattern::Satin,
            })
        };
        let travel = Travel::new(&shape);
        let mut sewer = Sewer::new(&travel, 2.0);
        sew(&shape, &p, Point::ZERO, &mut sewer);
        let long = sewer
            .finish()
            .iter()
            .flat_map(|r| r.windows(2).map(|w| w[0].distance(w[1])).collect::<Vec<_>>())
            .filter(|l| *l > 20.0)
            .count();
        assert!((8..=16).contains(&long), "{long}");
    }

    #[test]
    fn a_direction_line_turns_the_satin() {
        // A line round the middle of the C: the stitches go across the C,
        // from the inner edge to the outer edge, all round it.
        let shape = c_shape();
        let points: Vec<Point> = (0..=56)
            .map(|i| {
                let a = (40.0 + 5.0 * f64::from(i)).to_radians();
                Point::new(25.0 * a.cos(), 25.0 * a.sin())
            })
            .collect();
        let p = FillParams {
            direction: Some(emb_model::DirectionLine {
                points,
                angle_deg: 90.0,
            }),
            ..FillParams::default().with_pattern(FillPattern::Columns {
                column: ColumnPattern::Satin,
            })
        };
        let travel = Travel::new(&shape);
        let mut sewer = Sewer::new(&travel, 2.0);
        sew(&shape, &p, Point::ZERO, &mut sewer);
        let runs = sewer.finish();
        assert!(near_inside(&shape, &runs));
        let across: Vec<f64> = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| w[0].distance(w[1]) > 4.0)
            .map(|w| {
                // The angle between the stitch and the radius at its middle.
                let (d, m) = (w[1] - w[0], w[0].midpoint(w[1]));
                (d.dot(m) / (d.length() * m.length())).abs()
            })
            .collect();
        assert!(across.len() > 200, "{}", across.len());
        let radial = across.iter().filter(|c| **c > 0.94).count();
        assert!(
            radial as f64 > 0.95 * across.len() as f64,
            "{radial} of {}",
            across.len()
        );
    }

    #[test]
    fn column_fills_skip_holes() {
        let hole = Polyline {
            points: vec![
                Point::new(10.0, 5.0),
                Point::new(20.0, 5.0),
                Point::new(20.0, 15.0),
                Point::new(10.0, 15.0),
            ],
            closed: true,
        };
        let shape = Shape::new(&[rect(30.0, 20.0)], &[hole]);
        let runs = sewn(
            &shape,
            FillPattern::Columns {
                column: ColumnPattern::Zigzag,
            },
        );
        let over = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| {
                let m = w[0].midpoint(w[1]);
                m.x > 10.5 && m.x < 19.5 && m.y > 5.5 && m.y < 14.5
            })
            .count();
        assert_eq!(over, 0);
    }

    #[test]
    fn cross_stitch_lines_up_with_the_origin() {
        let shape = Shape::new(&[rect(10.0, 5.0)], &[]);
        let runs = sewn(
            &shape,
            FillPattern::CrossStitch {
                cell_mm: 2.5,
                style: CrossStyle::Full,
                passes: 1,
            },
        );
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        // Every needle point is a grid corner.
        assert!(pts.iter().all(|q| {
            let (x, y) = (q.x / 2.5, q.y / 2.5);
            (x - x.round()).abs() < 1e-9 && (y - y.round()).abs() < 1e-9
        }));
        // 4 × 2 crosses: 16 diagonal legs.
        let legs = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| ((w[1] - w[0]).x.abs() - 2.5).abs() < 1e-9 && ((w[1] - w[0]).y.abs() - 2.5).abs() < 1e-9)
            .count();
        assert_eq!(legs, 16);
    }

    #[test]
    fn stipple_is_one_closed_line_that_fills() {
        let shape = Shape::new(&[rect(30.0, 30.0)], &[]);
        let runs = sewn(&shape, FillPattern::Stipple { style: RunStyle::Run });
        assert_eq!(runs.len(), 1, "one meander");
        let r = &runs[0];
        assert!(r[0].distance(r[r.len() - 1]) < 1e-9, "it ends where it starts");
        // It reaches every 5 mm square of the inside.
        for gx in 0..6 {
            for gy in 0..6 {
                let c = Point::new(2.5 + 5.0 * gx as f64, 2.5 + 5.0 * gy as f64);
                assert!(r.iter().any(|q| q.distance(c) < 3.5), "{c:?}");
            }
        }
        // The same each run.
        assert_eq!(sewn(&shape, FillPattern::Stipple { style: RunStyle::Run }), runs);
    }

    #[test]
    fn laydown_area_covers_the_objects_with_a_margin() {
        let fill = ObjectKind::Fill {
            outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(10.0, 10.0))),
            holes: vec![Path::rect(emb_geom::BBox::new(
                Point::new(3.0, 3.0),
                Point::new(7.0, 7.0),
            ))],
            params: FillParams::default(),
        };
        let line = ObjectKind::Running {
            path: Path::new()
                .move_to(Point::new(20.0, 5.0))
                .line_to(Point::new(30.0, 5.0)),
            params: emb_model::RunningParams::default(),
        };
        let rings = laydown_area(&[&fill, &line], 2.0);
        assert_eq!(rings.len(), 2, "two areas apart, no holes");
        let area: f64 = rings.iter().map(|r| Region::new(r.clone(), Vec::new()).area()).sum();
        // 14 × 14 with round corners, and 14 × 5 with round ends.
        assert!(area > 180.0 && area < 196.0 + 70.0, "{area}");
        let inside = |p: Point| rings.iter().any(|r| emb_geom::region::ring_contains(r, p));
        assert!(inside(Point::new(5.0, 5.0)), "the hole is covered");
        assert!(inside(Point::new(-1.5, 5.0)) && !inside(Point::new(-2.5, 5.0)));
    }

    #[test]
    fn double_square_has_two_directions() {
        let shape = Shape::new(&[rect(20.0, 20.0)], &[]);
        let runs = sewn(&shape, FillPattern::DoubleSquare { angle_deg: 90.0 });
        let steps: Vec<Point> = runs.iter().flat_map(|r| r.windows(2)).map(|w| w[1] - w[0]).collect();
        let level = steps.iter().filter(|d| d.x.abs() > 2.0 && d.y.abs() < 0.01).count();
        let upright = steps.iter().filter(|d| d.y.abs() > 2.0 && d.x.abs() < 0.01).count();
        assert!(level > 50 && upright > 50, "{level} {upright}");
    }
}
