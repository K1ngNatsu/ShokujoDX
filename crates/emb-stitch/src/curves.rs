//! Curved fill types (P5 Milestone 2): contour, offset, spiral, island
//! coil, radial, maze and Florentine.
//!
//! Not a port. Each type follows what public embroidery documentation says
//! it looks like, and common digitizing practice:
//!
//! - Offset: rings inside the edge, the first half a row in, then one row
//!   apart, sewn from the edge to the middle. Holes get rings too.
//! - Spiral: as offset, but each ring blends into the next ring while it
//!   goes round, so the line is one continuous spiral. Where the inside
//!   splits into parts or has holes, the rest is sewn as offset rings.
//! - Maze: a spiral two rows apart to the middle, then a second spiral back
//!   out between its rows. The line is open and ends near its start.
//! - Island coil: rings of coil loops, each ring as wide as the spacing.
//! - Contour: the outline is cut into two sides at its first and last
//!   points along the fill angle. Each line keeps a fraction of the way
//!   from one side to the other, as on a contour column.
//! - Radial: lines from a center point (by default the middle of the box
//!   round the shape) to the edge. A line starts where it
//!   is one spacing from its neighbors, so the lines do not crowd at the
//!   center: line `k` with `2^z` as its largest power-of-two factor starts
//!   at `spacing · n / (2π · 2^z)` for `n` lines.
//! - Florentine: tatami rows that bend like a guide line. The shape is
//!   sheared so that the guide becomes straight, filled with tatami, and
//!   sheared back, so the rows keep the tatami pattern.
//!
//! Only `sqrt`, `sin`, `cos` and `atan2` from `emb_geom::math` are used (ADR 0003).

use emb_geom::math::{cos, sin, sin_cos_deg};
use emb_geom::{Path, Point, Polyline};
use emb_model::{ColumnPattern, FillParams, FillPattern, SatinGeometry, SatinParams, Underlay};

use crate::fill::{Tatami, sew_tatami};
use crate::object::FILL_STAGGERS;
use crate::running;
use crate::travel::{Sewer, Shape, Travel};

/// Smallest row spacing that the generator accepts, in mm.
const MIN_SPACING_MM: f64 = 0.2;
/// Most rings of an offset, spiral or maze fill.
const MAX_LEVELS: usize = 1000;
/// Most lines of a radial fill.
const MAX_RAYS: usize = 1 << 13;
/// Coil rings are this fraction of the spacing wide, so they do not touch.
const COIL_WIDTH: f64 = 0.85;
/// The guide of a Florentine fill and the edge of the shape are sampled this often, in mm.
const WARP_STEP_MM: f64 = 0.5;

/// Sews the top stitching of a fill of any type but tatami.
pub fn sew(shape: &Shape, p: &FillParams, entry: Point, sewer: &mut Sewer) {
    let spacing = p.spacing_mm.max(MIN_SPACING_MM);
    let len = p.stitch_length_mm.max(running::MIN_SETTING_MM);
    match &p.pattern {
        FillPattern::Tatami => {}
        FillPattern::Offset => {
            let levels = levels(shape, spacing, 0.5, 1.0);
            for l in loops_of(&levels, entry, len) {
                sew_line(sewer, &l);
            }
        }
        FillPattern::Spiral => {
            for l in spiral(&levels(shape, spacing, 0.5, 1.0), entry, len) {
                sew_line(sewer, &l);
            }
        }
        FillPattern::Maze => {
            let inward = spiral(&levels(shape, spacing, 0.5, 2.0), entry, len);
            let at = inward.last().and_then(|l| l.last().copied()).unwrap_or(entry);
            let mut outer = levels(shape, spacing, 1.5, 2.0);
            outer.reverse();
            for l in inward.iter().chain(spiral(&outer, at, len).iter()) {
                sew_line(sewer, l);
            }
        }
        FillPattern::IslandCoil => island_coil(shape, spacing, len, sewer),
        FillPattern::Contour => contour(shape, p.angle_deg, spacing, len, sewer),
        FillPattern::Radial { center } => radial(shape, *center, spacing, len, sewer),
        FillPattern::Florentine { guide } => florentine(shape, p, guide, entry, sewer),
        FillPattern::Tartan { sett, color } => crate::tartan::tartan(shape, p, sett, *color, entry, sewer),
        FillPattern::Meander { tile } => crate::meander::meander(shape, *tile, p.angle_deg, spacing, len, entry, sewer),
        FillPattern::Circular { center } => crate::rings::circular(shape, *center, spacing, len, entry, sewer),
        FillPattern::Guided { guides } => crate::rings::guided(shape, guides, p.angle_deg, spacing, len, entry, sewer),
        FillPattern::Columns { .. }
        | FillPattern::Square
        | FillPattern::DoubleSquare { .. }
        | FillPattern::String
        | FillPattern::CrossStitch { .. }
        | FillPattern::Stipple { .. }
        | FillPattern::FlexiSplit { .. }
        | FillPattern::Laydown { .. }
        | FillPattern::Mesh { .. } => crate::textures::sew(shape, p, entry, sewer),
        // Sewn as tatami rows (`object::fill_runs`).
        FillPattern::ProgramSplit { .. } | FillPattern::UserSplit { .. } | FillPattern::Gradient { .. } => {}
        FillPattern::Motif {
            motif,
            length_mm,
            height_mm,
            stagger,
            gradient,
            globe,
        } => {
            let f = crate::motifs::MotifFill {
                motif,
                length_mm: *length_mm,
                height_mm: *height_mm,
                stagger: *stagger,
                gradient: *gradient,
                globe: *globe,
            };
            crate::motifs::fill(shape, p, &f, sewer);
        }
    }
}

/// Goes to the start of `line` the shortest way, then sews it.
pub(crate) fn sew_line(sewer: &mut Sewer, line: &[Point]) {
    if let Some((&first, rest)) = line.split_first() {
        sewer.step_to(first);
        sewer.stitch(rest.iter().copied());
    }
}

/// The regions of one inset level, as (outer ring, holes).
type Level = Vec<(Vec<Point>, Vec<Vec<Point>>)>;

/// The rings of the shape moved in by `(first + k · every) · spacing`, for
/// k = 0, 1, ...: for each level, the regions as (outer, holes).
fn levels(shape: &Shape, spacing: f64, first: f64, every: f64) -> Vec<Level> {
    let mut out = Vec::new();
    for k in 0..MAX_LEVELS {
        let inset = shape.inset(spacing * (first + every * k as f64));
        if inset.is_empty() {
            break;
        }
        out.push(
            inset
                .regions
                .iter()
                .map(|r| (r.outer.clone(), r.holes.clone()))
                .collect(),
        );
    }
    out
}

/// A closed ring as needle points about `len` apart, starting at the
/// ring point nearest to `at` and ending there again.
fn ring_from(ring: &[Point], at: Point, len: f64) -> Vec<Point> {
    let mut pts = ring.to_vec();
    pts.dedup();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        return pts;
    }
    let start = nearest(&pts, at);
    pts.rotate_left(start);
    running::running_points(
        &Polyline {
            points: pts,
            closed: true,
        },
        len,
    )
}

fn nearest(pts: &[Point], at: Point) -> usize {
    (0..pts.len())
        .min_by(|&i, &j| pts[i].distance(at).total_cmp(&pts[j].distance(at)))
        .unwrap_or(0)
}

/// All rings of all levels as closed loops, from the edge to the middle;
/// each loop starts near the end of the loop before it.
fn loops_of(levels: &[Level], entry: Point, len: f64) -> Vec<Vec<Point>> {
    let mut at = entry;
    let mut out = Vec::new();
    for level in levels {
        let mut rings: Vec<&Vec<Point>> = level.iter().flat_map(|(o, h)| std::iter::once(o).chain(h)).collect();
        while !rings.is_empty() {
            // The ring with the point nearest to the needle next.
            let i = (0..rings.len())
                .min_by(|&i, &j| {
                    let d = |r: &Vec<Point>| r[nearest(r, at)].distance(at);
                    d(rings[i]).total_cmp(&d(rings[j]))
                })
                .unwrap_or(0);
            let l = ring_from(rings.remove(i), at, len);
            if let Some(&last) = l.last() {
                at = last;
            }
            out.push(l);
        }
    }
    out
}

/// A closed ring, from its point nearest to `at`, with the length along it.
struct Loop {
    pts: Vec<Point>,
    at: Vec<f64>,
}

impl Loop {
    fn new(ring: &[Point], from: Point) -> Loop {
        let mut pts = ring.to_vec();
        if pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        let start = nearest(&pts, from);
        pts.rotate_left(start);
        pts.push(pts[0]);
        let mut at = vec![0.0];
        for w in pts.windows(2) {
            at.push(at[at.len() - 1] + w[0].distance(w[1]));
        }
        Loop { pts, at }
    }

    fn len(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    fn point(&self, s: f64) -> Point {
        let s = s.clamp(0.0, self.len());
        let i = self.at.partition_point(|&l| l <= s).clamp(1, self.pts.len() - 1);
        let seg = self.at[i] - self.at[i - 1];
        let t = if seg > 0.0 { (s - self.at[i - 1]) / seg } else { 0.0 };
        self.pts[i - 1].lerp(self.pts[i], t)
    }
}

/// A spiral through the levels while each level is one ring with no
/// holes: each turn blends from its ring to the next ring. The rest are
/// separate loops. Returns the lines in sew order.
fn spiral(levels: &[Level], entry: Point, len: f64) -> Vec<Vec<Point>> {
    let single = |k: usize| levels.get(k).is_some_and(|l| l.len() == 1 && l[0].1.is_empty());
    let mut line: Vec<Point> = Vec::new();
    let mut at = entry;
    let mut k = 0;
    while single(k) && single(k + 1) {
        let a = Loop::new(&levels[k][0].0, at);
        let b = Loop::new(&levels[k + 1][0].0, a.pts[0]);
        if a.len() <= 0.0 || b.len() <= 0.0 {
            break;
        }
        // Enough points for the longer ring of the two (a maze goes outward).
        let n = ((a.len().max(b.len()) / len).ceil() as usize).max(8);
        for i in 0..n {
            let t = i as f64 / n as f64;
            line.push(a.point(t * a.len()).lerp(b.point(t * b.len()), t));
        }
        at = b.pts[0];
        k += 1;
    }
    let mut out = Vec::new();
    if !line.is_empty() {
        // The last turn goes round the last ring of the spiral.
        let last = ring_from(&levels[k][0].0, at, len);
        at = last.last().copied().unwrap_or(at);
        line.extend(last);
        line.dedup();
        out.push(line);
        k += 1;
    }
    out.extend(loops_of(levels.get(k..).unwrap_or(&[]), at, len));
    out
}

fn ring_path(ring: &[Point]) -> Option<Path> {
    let (&first, rest) = ring.split_first()?;
    Some(
        rest.iter()
            .fold(Path::new().move_to(first), |p, q| p.line_to(*q))
            .close(),
    )
}

fn island_coil(shape: &Shape, spacing: f64, len: f64, sewer: &mut Sewer) {
    let params = SatinParams {
        spacing_mm: len,
        pull_compensation_mm: 0.0,
        underlay: Underlay::None,
        ..SatinParams::with_pattern(ColumnPattern::Coil)
    };
    let mut at = sewer.needle().unwrap_or_default();
    for level in levels(shape, spacing, 0.5, 1.0) {
        for (outer, holes) in &level {
            for ring in std::iter::once(outer).chain(holes) {
                let Some(path) = ring_path(ring) else { continue };
                let g = SatinGeometry::from_centerline(&path, spacing * COIL_WIDTH);
                if let Some(pts) = crate::satin::satin(&g, &params, at, None) {
                    at = pts.last().copied().unwrap_or(at);
                    sew_line(sewer, &pts);
                }
            }
        }
    }
}

/// The parts of `line` inside the shape. A stitch whose middle is outside
/// is cut out; the cut ends are on the edge.
pub(crate) fn clip(line: &[Point], shape: &Shape) -> Vec<Vec<Point>> {
    let inside = |p: Point| shape.regions.iter().any(|r| r.contains(p));
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    for w in line.windows(2) {
        // Cut the stitch where it crosses an edge.
        let mut ts: Vec<f64> = vec![0.0, 1.0];
        for ring in shape.rings() {
            for i in 0..ring.len() {
                if let Some(t) = cross(w[0], w[1], ring[i], ring[(i + 1) % ring.len()]) {
                    ts.push(t);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for tw in ts.windows(2) {
            let (a, b) = (w[0].lerp(w[1], tw[0]), w[0].lerp(w[1], tw[1]));
            if inside(a.midpoint(b)) {
                if cur.last() != Some(&a) {
                    if cur.len() >= 2 {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur = vec![a];
                }
                cur.push(b);
            } else if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

/// Where segment `p`–`q` crosses segment `a`–`b`, as a fraction of `p`–`q`.
fn cross(p: Point, q: Point, a: Point, b: Point) -> Option<f64> {
    let r = q - p;
    let s = b - a;
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (a - p).cross(s) / den;
    let u = (a - p).cross(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

fn contour(shape: &Shape, angle_deg: f64, spacing: f64, len: f64, sewer: &mut Sewer) {
    let (s, c) = sin_cos_deg(angle_deg);
    let dir = Point::new(c, s);
    for region in &shape.regions {
        let Some((a, b)) = crate::textures::halves(&region.outer, dir) else {
            continue;
        };
        let poly = |v: &[Point]| v[1..].iter().fold(Path::new().move_to(v[0]), |p, q| p.line_to(*q));
        let g = SatinGeometry {
            rail_a: poly(&a),
            rail_b: poly(&b),
            rungs: Vec::new(),
        };
        let Some(col) = crate::satin::Column::new(&g) else {
            continue;
        };
        let pairs = col.pairs(0.2);
        for (i, mut line) in crate::column::contour_lines(&pairs, spacing, len)
            .into_iter()
            .enumerate()
        {
            if i % 2 == 1 {
                line.reverse();
            }
            for piece in clip(&line, shape) {
                sew_line(sewer, &piece);
            }
        }
    }
}

/// The largest power of two that divides `k` (for k > 0).
fn power_of_two_factor(k: usize) -> usize {
    1 << k.trailing_zeros()
}

fn radial(shape: &Shape, center: Option<Point>, spacing: f64, len: f64, sewer: &mut Sewer) {
    let pts: Vec<Point> = shape.rings().flatten().copied().collect();
    let Some(bbox) = emb_geom::BBox::from_points(pts.iter().copied()) else {
        return;
    };
    let c = center.unwrap_or(bbox.center());
    let reach = pts.iter().map(|p| p.distance(c)).fold(0.0, f64::max) + 1.0;
    let want = (std::f64::consts::TAU * reach / spacing).ceil().max(8.0) as usize;
    let n = want.next_power_of_two().min(MAX_RAYS);
    for k in 0..n {
        let th = std::f64::consts::TAU * k as f64 / n as f64;
        let u = Point::new(cos(th), sin(th));
        let inner = if k == 0 {
            0.0
        } else {
            spacing * n as f64 / (std::f64::consts::TAU * power_of_two_factor(k) as f64)
        };
        if inner >= reach {
            continue;
        }
        let ray = [c + u * inner, c + u * reach];
        let mut pieces = clip(&ray, shape);
        if k % 2 == 1 {
            // Every second line is sewn toward the center.
            pieces.reverse();
            for p in &mut pieces {
                p.reverse();
            }
        }
        for piece in pieces {
            let line = Polyline {
                points: piece,
                closed: false,
            };
            sew_line(sewer, &running::running_points(&line, len));
        }
    }
}

/// The guide as a height over its chord: points (x, y) in the frame of the
/// chord (x along it), sorted by x, with the parts that turn back left out.
struct Guide {
    origin: Point,
    along: Point,
    side: Point,
    xs: Vec<f64>,
    ys: Vec<f64>,
}

impl Guide {
    fn new(guide: &[Point]) -> Option<Guide> {
        let (first, last) = (*guide.first()?, *guide.last()?);
        let along = (last - first).normalized()?;
        let side = Point::new(-along.y, along.x);
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for p in guide {
            let x = (*p - first).dot(along);
            if xs.last().is_none_or(|&l| x > l + 1e-6) {
                xs.push(x);
                ys.push((*p - first).dot(side));
            }
        }
        (xs.len() >= 2).then_some(Guide {
            origin: first,
            along,
            side,
            xs,
            ys,
        })
    }

    /// The height of the guide at `x`; constant past its ends.
    fn y(&self, x: f64) -> f64 {
        let i = self.xs.partition_point(|&l| l <= x);
        if i == 0 {
            return self.ys[0];
        }
        if i >= self.xs.len() {
            return self.ys[self.ys.len() - 1];
        }
        let t = (x - self.xs[i - 1]) / (self.xs[i] - self.xs[i - 1]);
        self.ys[i - 1] + t * (self.ys[i] - self.ys[i - 1])
    }

    /// Shears `p` so that the guide becomes straight.
    fn flat(&self, p: Point) -> Point {
        let x = (p - self.origin).dot(self.along);
        p - self.side * self.y(x)
    }

    fn bent(&self, p: Point) -> Point {
        let x = (p - self.origin).dot(self.along);
        p + self.side * self.y(x)
    }
}

/// Points along a closed ring, no more than `step` apart.
fn densify(ring: &[Point], step: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let n = (a.distance(b) / step).ceil().max(1.0) as usize;
        out.extend((0..n).map(|k| a.lerp(b, k as f64 / n as f64)));
    }
    out
}

fn florentine(shape: &Shape, p: &FillParams, guide: &[Point], entry: Point, sewer: &mut Sewer) {
    let rows = |angle_deg: f64| Tatami {
        spacing_mm: p.spacing_mm.max(MIN_SPACING_MM),
        angle_deg,
        stitch_len_mm: p.stitch_length_mm,
        pull_comp_mm: p.pull_compensation_mm,
        staggers: FILL_STAGGERS,
        split: crate::fill::RowSplit::default(),
    };
    let Some(g) = Guide::new(guide) else {
        // No guide: straight rows at the fill angle.
        sew_tatami(shape, &rows(p.angle_deg), entry, None, sewer);
        return;
    };
    let line = |r: &[Point]| Polyline {
        points: densify(r, WARP_STEP_MM).into_iter().map(|q| g.flat(q)).collect(),
        closed: true,
    };
    let (outers, holes): (Vec<Polyline>, Vec<Polyline>) = (
        shape.regions.iter().map(|r| line(&r.outer)).collect(),
        shape
            .regions
            .iter()
            .flat_map(|r| r.holes.iter().map(|h| line(h)))
            .collect(),
    );
    let flat = Shape::new(&outers, &holes);
    let travel = Travel::new(&flat);
    let mut flat_sewer = Sewer::new(&travel, crate::object::TRAVEL_LEN_MM);
    let angle = emb_geom::math::to_degrees(emb_geom::math::atan2(g.along.y, g.along.x));
    sew_tatami(&flat, &rows(angle), g.flat(entry), None, &mut flat_sewer);
    for run in flat_sewer.finish() {
        let pts: Vec<Point> = run.into_iter().map(|q| g.bent(q)).collect();
        sew_line(sewer, &pts);
    }
}

#[cfg(test)]
mod tests {
    use emb_model::FillPattern;

    use super::*;

    fn square(size: f64) -> Shape {
        let pts = vec![
            Point::new(0.0, 0.0),
            Point::new(size, 0.0),
            Point::new(size, size),
            Point::new(0.0, size),
        ];
        Shape::new(
            &[Polyline {
                points: pts,
                closed: true,
            }],
            &[],
        )
    }

    fn sewn(shape: &Shape, pattern: FillPattern) -> Vec<Vec<Point>> {
        let travel = Travel::new(shape);
        let mut sewer = Sewer::new(&travel, 3.0);
        let p = FillParams::default().with_pattern(pattern);
        sew(shape, &p, Point::ZERO, &mut sewer);
        sewer.finish()
    }

    fn inside(shape: &Shape, runs: &[Vec<Point>]) -> bool {
        let grown = shape.inset(-0.05);
        runs.iter()
            .flatten()
            .all(|q| grown.regions.iter().any(|r| r.contains(*q)))
    }

    #[test]
    fn every_type_stays_inside_and_covers_the_middle() {
        let shape = square(20.0);
        let curved = FillPattern::all().into_iter().filter(|p| {
            matches!(
                p,
                FillPattern::Contour
                    | FillPattern::Offset
                    | FillPattern::Spiral
                    | FillPattern::IslandCoil
                    | FillPattern::Radial { .. }
                    | FillPattern::Maze
                    | FillPattern::Florentine { .. }
                    | FillPattern::Tartan { .. }
                    | FillPattern::Meander { .. }
                    | FillPattern::Circular { .. }
                    | FillPattern::Guided { .. }
            )
        });
        for pattern in curved {
            let runs = sewn(&shape, pattern.clone());
            let pts: Vec<Point> = runs.iter().flatten().copied().collect();
            assert!(pts.len() > 50, "{pattern:?}: {}", pts.len());
            assert!(inside(&shape, &runs), "{pattern:?}");
            // Some needle point within 1.5 mm of the middle.
            let mid = Point::new(10.0, 10.0);
            let near = pts.iter().map(|q| q.distance(mid)).fold(f64::INFINITY, f64::min);
            assert!(near < 1.5, "{pattern:?}: {near}");
        }
    }

    #[test]
    fn spiral_is_one_line() {
        let runs = sewn(&square(20.0), FillPattern::Spiral);
        assert_eq!(runs.len(), 1);
        // No stitch longer than the stitch length (and a little for corners).
        assert!(runs[0].windows(2).all(|w| w[0].distance(w[1]) < 3.0));
    }

    #[test]
    fn offset_rings_are_a_row_apart() {
        let shape = square(20.0);
        let runs = sewn(&shape, FillPattern::Offset);
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        // Rings at 0.2, 0.6, 1.0 ... mm from the left edge (x along y = 10).
        let mut xs: Vec<i64> = pts
            .iter()
            .filter(|q| (q.y - 10.0).abs() < 2.0 && q.x < 5.0)
            .map(|q| (q.x * 10.0).round() as i64)
            .collect();
        xs.sort_unstable();
        xs.dedup();
        assert!(xs.starts_with(&[2, 6, 10]), "{xs:?}");
    }

    #[test]
    fn maze_ends_near_its_start() {
        let runs = sewn(&square(30.0), FillPattern::Maze);
        let (first, last) = (runs[0][0], *runs.last().unwrap().last().unwrap());
        assert!(first.distance(last) < 6.0, "{first:?} {last:?}");
    }

    #[test]
    fn radial_lines_meet_at_the_center() {
        let c = Point::new(10.0, 10.0);
        let runs = sewn(&square(20.0), FillPattern::Radial { center: Some(c) });
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        let near = pts.iter().filter(|q| q.distance(c) < 1.0).count();
        let far = pts.iter().filter(|q| q.distance(c) > 9.0).count();
        // Few lines reach the center; many reach the edge.
        assert!(near > 0 && near * 10 < far, "{near} {far}");
    }

    #[test]
    fn contour_lines_skip_holes() {
        let hole = Polyline {
            points: vec![
                Point::new(8.0, 8.0),
                Point::new(12.0, 8.0),
                Point::new(12.0, 12.0),
                Point::new(8.0, 12.0),
            ],
            closed: true,
        };
        let outer = Polyline {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 20.0),
                Point::new(0.0, 20.0),
            ],
            closed: true,
        };
        let shape = Shape::new(&[outer], &[hole]);
        let runs = sewn(&shape, FillPattern::Contour);
        let pts: Vec<Point> = runs.iter().flatten().copied().collect();
        assert!(pts.len() > 100);
        // Stitches do not cross the hole (only travel along its edge).
        let crossing = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| {
                let m = w[0].midpoint(w[1]);
                m.x > 8.2 && m.x < 11.8 && m.y > 8.2 && m.y < 11.8
            })
            .count();
        assert_eq!(crossing, 0);
    }

    #[test]
    fn florentine_rows_follow_the_guide() {
        let shape = square(30.0);
        // A guide that rises 6 mm in the middle.
        let guide = vec![Point::new(0.0, 15.0), Point::new(15.0, 9.0), Point::new(30.0, 15.0)];
        let runs = sewn(&shape, FillPattern::Florentine { guide });
        assert!(inside(&shape, &runs));
        // The longest stitches of each row are near the slope of the guide,
        // not horizontal: many stitches have a clear vertical part.
        let sloped = runs
            .iter()
            .flat_map(|r| r.windows(2))
            .filter(|w| {
                let d = w[1] - w[0];
                d.length() > 1.0 && (d.y / d.length()).abs() > 0.2
            })
            .count();
        assert!(sloped > 100, "{sloped}");
        let flat = sewn(&shape, FillPattern::Florentine { guide: Vec::new() });
        assert!(!flat.is_empty());
    }
}
