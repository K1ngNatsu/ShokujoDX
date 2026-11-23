//! Tuning of the letters of a converted font with the outline of the
//! source font (P17 Milestone 5, ADR 0113).
//!
//! [`extend_ends`]: the stroke finder ends a column where its stroke meets
//! another stroke. At a sharp corner (the top of "N", the corners of "Z")
//! that leaves a part of the outline with no stitches, and the two columns
//! do not touch. The ends of the columns go on into such a part, as far as
//! the outline, so the corner has stitches and the columns join.

use emb_geom::{BBox, Path, Point};
use emb_model::ObjectKind;

use crate::grid::{self, Grid};

/// The cells of the grid for each capital height.
const CELLS: f64 = 125.0;
/// An end goes on at most this far, as a factor of the width of the column.
const REACH: f64 = 3.5;
/// The part of the new area that can be outside the outline.
const SPILL: f64 = 0.08;
/// The part of one step that can be outside the outline.
const STEP_SPILL: f64 = 0.12;
/// A change is made only if it covers at least this much, as a part of the
/// square of the width of the column.
const MIN_GAIN: f64 = 0.02;
/// The end part of a column that can turn to the column that it meets,
/// and the body before it that gives the direction of the column, as
/// factors of the width of the column.
const TAIL: f64 = 1.2;
const BODY: f64 = 2.0;
/// The shortest body that gives a direction, as a factor of the width.
const MIN_BODY: f64 = 0.8;
/// An end part turns if the cosine of its angle to the body is less than
/// this (10 degrees).
const BENT_COS: f64 = 0.985;

/// The outline of a letter: outer rings with their holes.
pub type Outline = [(Vec<Point>, Vec<Vec<Point>>)];

struct Masks {
    grid: Grid,
    /// The cells of the outline.
    glyph: Vec<bool>,
    /// The number of parts on each cell.
    cover: Vec<u8>,
}

fn flat(path: &Path) -> Vec<Point> {
    path.flatten(0.02).into_iter().flat_map(|pl| pl.points).collect()
}

/// The cells of `grid` in the area of a part. A column that lies on
/// itself has its cells one time.
pub fn part_mask(grid: &Grid, kind: &ObjectKind) -> Vec<bool> {
    let mut mask = vec![false; grid.w * grid.h];
    match kind {
        ObjectKind::Satin { geometry, .. } => {
            let (a, b) = (geometry.rail_a.flatten(0.05), geometry.rail_b.flatten(0.05));
            match (a.first(), b.first()) {
                (Some(a), Some(b)) if a.closed && b.closed => {
                    // A ring column: between its two rings.
                    let mut inner = vec![false; grid.w * grid.h];
                    grid::fill_into(grid, std::slice::from_ref(&a.points), &mut mask);
                    grid::fill_into(grid, std::slice::from_ref(&b.points), &mut inner);
                    for (m, i) in mask.iter_mut().zip(&inner) {
                        *m ^= *i;
                    }
                }
                (Some(a), Some(b)) => {
                    let ring: Vec<Point> = a.points.iter().chain(b.points.iter().rev()).copied().collect();
                    grid::fill_into(grid, &[ring], &mut mask);
                }
                _ => {}
            }
        }
        ObjectKind::Fill { outer, holes, .. } => {
            let rings = |p: &Path| -> Vec<Vec<Point>> { p.flatten(0.05).into_iter().map(|pl| pl.points).collect() };
            grid::fill_into(grid, &rings(outer), &mut mask);
            let mut out = vec![false; grid.w * grid.h];
            grid::fill_into(grid, &holes.iter().flat_map(rings).collect::<Vec<_>>(), &mut out);
            for (m, o) in mask.iter_mut().zip(&out) {
                *m &= !o;
            }
        }
        _ => {}
    }
    mask
}

/// The cells of `grid` in the outline of a letter.
pub fn outline_mask(grid: &Grid, outline: &Outline) -> Vec<bool> {
    let mut glyph = vec![false; grid.w * grid.h];
    for (outer, holes) in outline {
        let mut one = vec![false; grid.w * grid.h];
        grid::fill_into(grid, std::slice::from_ref(outer), &mut one);
        let mut out = vec![false; grid.w * grid.h];
        grid::fill_into(grid, holes, &mut out);
        for ((g, o), h) in glyph.iter_mut().zip(&one).zip(&out) {
            *g |= *o && !*h;
        }
    }
    glyph
}

impl Masks {
    fn new(parts: &[ObjectKind], outline: &Outline, cap: f64) -> Option<Masks> {
        let points = outline
            .iter()
            .flat_map(|(o, h)| o.iter().chain(h.iter().flatten()))
            .copied();
        let bounds = BBox::from_points(points)?;
        let bounds = parts.iter().filter_map(|p| p.control_bbox()).fold(bounds, BBox::union);
        let grid = Grid::new(bounds, cap / CELLS, 3);
        let glyph = outline_mask(&grid, outline);
        let mut cover = vec![0u8; grid.w * grid.h];
        for p in parts {
            for (c, m) in cover.iter_mut().zip(part_mask(&grid, p)) {
                *c += u8::from(m);
            }
        }
        Some(Masks { grid, glyph, cover })
    }

    /// The cells of the polygon `ring`.
    fn cells(&self, ring: &[Point]) -> Vec<usize> {
        let mut mask = vec![false; self.grid.w * self.grid.h];
        grid::fill_into(&self.grid, &[ring.to_vec()], &mut mask);
        mask.iter().enumerate().filter(|(_, m)| **m).map(|(i, _)| i).collect()
    }

    /// For the cells of `ring` that are not in `taken`: how many are in
    /// the outline with no part on them, and how many are outside the
    /// outline.
    fn count(&self, ring: &[Point], taken: &[bool]) -> (usize, usize, Vec<usize>) {
        let cells: Vec<usize> = self.cells(ring).into_iter().filter(|i| !taken[*i]).collect();
        let gain = cells.iter().filter(|i| self.glyph[**i] && self.cover[**i] == 0).count();
        let spill = cells.iter().filter(|i| !self.glyph[**i]).count();
        (gain, spill, cells)
    }
}

/// The direction in which the line `pts` goes on after its last point.
fn out_dir(pts: &[Point]) -> Option<Point> {
    let last = *pts.last()?;
    pts.iter()
        .rev()
        .find(|p| p.distance(last) > 0.05)
        .and_then(|p| (last - *p).normalized())
}

/// How far the end (`a`, `b`) of a column goes on in the direction `d`:
/// `both` rail ends, then one of them alone. Returns the new end.
fn reach(m: &Masks, a: Point, b: Point, d: Point) -> Option<(Point, Point)> {
    let width = a.distance(b);
    let step = m.grid.cell;
    let steps = ((REACH * width) / step).ceil() as usize;
    let mut taken = vec![false; m.grid.w * m.grid.h];
    let mut total = 0usize;
    // Walks from (a, b) with the moves `da` and `db` for each step. Returns
    // the number of steps to the last one that covered new cells.
    let walk = |a: Point, b: Point, da: Point, db: Point, taken: &mut Vec<bool>, total: &mut usize| -> usize {
        let (mut best, mut gain_sum, mut spill_sum, mut cells_sum) = (0, 0usize, 0usize, 0usize);
        let mut pending: Vec<usize> = Vec::new();
        let mut kept: Vec<usize> = Vec::new();
        for k in 1..=steps {
            let (a0, b0) = (a + da * (k - 1) as f64, b + db * (k - 1) as f64);
            let (a1, b1) = (a + da * k as f64, b + db * k as f64);
            let (gain, spill, cells) = m.count(&[a0, a1, b1, b0], taken);
            // A step that is outside the outline ends the walk.
            if spill as f64 > STEP_SPILL * cells.len() as f64 + 1.0 {
                break;
            }
            gain_sum += gain;
            spill_sum += spill;
            cells_sum += cells.len();
            pending.extend(cells);
            if gain > 0 && (spill_sum as f64) <= SPILL * cells_sum as f64 + 2.0 {
                best = k;
                kept.append(&mut pending);
                *total += gain_sum;
                gain_sum = 0;
            }
        }
        for i in kept {
            taken[i] = true;
        }
        best
    };
    let k = walk(a, b, d * step, d * step, &mut taken, &mut total);
    let (a1, b1) = (a + d * (step * k as f64), b + d * (step * k as f64));
    let none = Point::new(0.0, 0.0);
    let ka = walk(a1, b1, d * step, none, &mut taken, &mut total);
    let a2 = a1 + d * (step * ka as f64);
    let kb = walk(a2, b1, none, d * step, &mut taken, &mut total);
    let b2 = b1 + d * (step * kb as f64);
    let enough = total as f64 * step * step >= MIN_GAIN * width * width;
    (enough && (a2.distance(a) > 1e-9 || b2.distance(b) > 1e-9)).then_some((a2, b2))
}

/// The end of a rail (the last point of `pts`) cut back by `dist` along
/// the rail: the points that stay, with the cut point last.
fn cut_back(pts: &[Point], dist: f64) -> Option<Vec<Point>> {
    let mut left = dist;
    for i in (1..pts.len()).rev() {
        let seg = pts[i].distance(pts[i - 1]);
        if seg >= left && seg > 0.0 {
            let mut out = pts[..i].to_vec();
            let cut = pts[i].lerp(pts[i - 1], left / seg);
            if out.last().is_none_or(|p| p.distance(cut) > 1e-9) {
                out.push(cut);
            }
            return (out.len() >= 2).then_some(out);
        }
        left -= seg;
    }
    None
}

fn length(pts: &[Point]) -> f64 {
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The direction of the body of a rail before its end part: from the point
/// `far` back along the rail to the point `near` back. `None` if the rail
/// is too short or its body is not straight.
fn body_dir(pts: &[Point], near: f64, far: f64) -> Option<Point> {
    let (a, b) = (cut_back(pts, far)?, cut_back(pts, near)?);
    let (from, to) = (*a.last()?, *b.last()?);
    let d = (to - from).normalized()?;
    // The points between them are on the line.
    let straight = b[a.len() - 1..].iter().all(|p| {
        let v = *p - from;
        (v.x * d.y - v.y * d.x).abs() <= 0.03 * (far - near)
    });
    straight.then_some(d)
}

fn polyline(pts: &[Point]) -> Path {
    let mut path = Path::new().move_to(pts[0]);
    for p in &pts[1..] {
        path = path.line_to(*p);
    }
    path
}

/// The cells of the outline with no part on them, and the cells of the
/// parts outside the outline.
fn faults(parts: &[ObjectKind], outline: &Outline, cap: f64) -> Option<(usize, usize)> {
    let m = Masks::new(parts, outline, cap)?;
    let open = m.glyph.iter().zip(&m.cover).filter(|(g, c)| **g && **c == 0).count();
    let out = m.glyph.iter().zip(&m.cover).filter(|(g, c)| !**g && **c > 0).count();
    Some((open, out))
}

/// The column `kind` with the end of its rails (the start with `at_end`
/// false) replaced: the rails as lines through `ra` and `rb` (each with
/// its end last). The rungs nearer than `drop` to the new end go.
fn with_rails(kind: &ObjectKind, mut ra: Vec<Point>, mut rb: Vec<Point>, at_end: bool, drop: f64) -> ObjectKind {
    let mut out = kind.clone();
    if let ObjectKind::Satin { geometry, .. } = &mut out {
        let (ea, eb) = (ra[ra.len() - 1], rb[rb.len() - 1]);
        if drop > 0.0 {
            geometry.rungs.retain(|(p, q)| {
                let (p, q) = if p.distance(ea) + q.distance(eb) <= p.distance(eb) + q.distance(ea) {
                    (*p, *q)
                } else {
                    (*q, *p)
                };
                p.distance(ea) > drop && q.distance(eb) > drop
            });
        }
        if !at_end {
            ra.reverse();
            rb.reverse();
        }
        geometry.rail_a = polyline(&ra);
        geometry.rail_b = polyline(&rb);
    }
    out
}

/// The rails of the column `kind` as lines, each with the end (the start
/// with `at_end` false) last. `None` for a part that is not an open column.
fn rails_to(kind: &ObjectKind, at_end: bool) -> Option<(Vec<Point>, Vec<Point>)> {
    let ObjectKind::Satin { geometry, .. } = kind else {
        return None;
    };
    let (mut ra, mut rb) = (flat(&geometry.rail_a), flat(&geometry.rail_b));
    if ra.len() < 2 || rb.len() < 2 || ra[0].distance(ra[ra.len() - 1]) < 1e-6 {
        return None;
    }
    if !at_end {
        ra.reverse();
        rb.reverse();
    }
    Some((ra, rb))
}

/// The column with its end longer in the direction `d`, from the end of
/// the rails `ra` and `rb`, if the outline has a free part there.
fn longer(
    parts: &[ObjectKind],
    i: usize,
    outline: &Outline,
    cap: f64,
    (ra, rb): (&[Point], &[Point]),
    d: Point,
    at_end: bool,
) -> Option<ObjectKind> {
    let m = Masks::new(parts, outline, cap)?;
    let (a, b) = (ra[ra.len() - 1], rb[rb.len() - 1]);
    let (a2, b2) = reach(&m, a, b, d)?;
    // The grid lets a rail along the edge of the outline go on past a
    // corner: each new piece stops where it meets the outline (P21
    // Milestone 6).
    let (a2, b2) = (clip_at_outline(a, a2, outline), clip_at_outline(b, b2, outline));
    let (mut na, mut nb) = (ra.to_vec(), rb.to_vec());
    if a2.distance(a) > 1e-9 {
        na.push(a2);
    }
    if b2.distance(b) > 1e-9 {
        nb.push(b2);
    }
    Some(with_rails(&parts[i], na, nb, at_end, 0.0))
}

/// A stitch shorter than this (a part of the capital height; 0.5 mm at 25
/// mm, 0.3 mm at 15 mm) that the tuning makes is too short.
const SHORT_STITCH: f64 = 0.02;

/// The line from `from` to `to`, up to where it crosses the outline (the
/// first crossing past `from`).
fn clip_at_outline(from: Point, to: Point, outline: &Outline) -> Point {
    let len = from.distance(to);
    if len < 1e-9 {
        return to;
    }
    let mut best = 1.0f64;
    for (outer, holes) in outline {
        for ring in std::iter::once(outer).chain(holes) {
            let n = ring.len();
            for k in 0..n {
                if let Some((u, _)) = Point::crossing(from, to, ring[k], ring[(k + 1) % n])
                    && u * len > 1e-6
                {
                    best = best.min(u);
                }
            }
        }
    }
    from.lerp(to, best)
}

/// The parts of `trial` that are not the same as in `parts` have no more
/// needle points out of the outline than they had (P21 Milestone 6: an end
/// that goes on to a corner of the outline must not go out of it).
fn no_new_spikes(parts: &[ObjectKind], trial: &[ObjectKind], outline: &Outline, cap: f64) -> bool {
    let changed: Vec<usize> = (0..parts.len()).filter(|&k| trial[k] != parts[k]).collect();
    let of = |v: &[ObjectKind]| -> Vec<ObjectKind> { changed.iter().map(|&k| v[k].clone()).collect() };
    let (old, new) = (of(parts), of(trial));
    crate::quality::spike_points(&new, outline) <= crate::quality::spike_points(&old, outline)
        && no_short_stitches(&old, &new, cap)
}

/// The columns `new` have no stitch shorter than [`SHORT_STITCH`] (a part of
/// the capital height) that is shorter than the shortest one of `old`: a
/// needle cannot sew it at the smaller sizes (P21 Milestone 6: the
/// shoulder of "n" in Lato Bold, 0.27 mm at 20 mm).
fn no_short_stitches(old: &[ObjectKind], new: &[ObjectKind], cap: f64) -> bool {
    let after = crate::quality::shortest_stitch(new);
    after >= SHORT_STITCH * cap || after >= crate::quality::shortest_stitch(old) - 1e-9
}

/// `after` is better than `before` (the cells from [`faults`]): the
/// outline has fewer cells with no stitches, by `min_gain` at least, and
/// not many more cells of the parts are outside the outline.
fn better(before: (usize, usize), after: (usize, usize), min_gain: f64) -> bool {
    let gain = before.0.saturating_sub(after.0) as f64;
    let spill = after.1.saturating_sub(before.1) as f64;
    gain >= min_gain && spill <= SPILL * gain + 3.0
}

/// Makes each end of a column longer as it is, where the outline has a
/// free part after it. `skip`: a part that stays. Returns the number of
/// ends that changed.
fn lengthen(parts: &mut [ObjectKind], outline: &Outline, cap: f64, skip: Option<usize>) -> usize {
    let cell = cap / CELLS;
    let mut changed = 0;
    for i in (0..parts.len()).filter(|i| Some(*i) != skip) {
        for at_end in [false, true] {
            let Some((ra, rb)) = rails_to(&parts[i], at_end) else {
                continue;
            };
            let width = ra[ra.len() - 1].distance(rb[rb.len() - 1]);
            // The direction of the end, and the directions of longer
            // parts of the column before it (the end can turn a little).
            let len = length(&ra).min(length(&rb));
            let chord = |back: f64| {
                let (a, b) = (cut_back(&ra, back)?, cut_back(&rb, back)?);
                let da = (ra[ra.len() - 1] - *a.last()?).normalized()?;
                let db = (rb[rb.len() - 1] - *b.last()?).normalized()?;
                (da + db).normalized()
            };
            let dirs = [
                out_dir(&ra)
                    .zip(out_dir(&rb))
                    .and_then(|(da, db)| (da + db).normalized()),
                chord((1.5 * width).min(0.5 * len)),
                chord((3.0 * width).min(0.9 * len)),
            ];
            let Some(before) = faults(parts, outline, cap) else {
                return changed;
            };
            let min_gain = (MIN_GAIN * width * width / (cell * cell)).max(2.0);
            let mut best: Option<((usize, usize), ObjectKind)> = None;
            for d in dirs.into_iter().flatten() {
                let Some(candidate) = longer(parts, i, outline, cap, (&ra, &rb), d, at_end) else {
                    continue;
                };
                let old = std::mem::replace(&mut parts[i], candidate);
                if let Some(after) = faults(parts, outline, cap)
                    && better(before, after, min_gain)
                    && best.as_ref().is_none_or(|(b, _)| after.0 < b.0)
                    && crate::quality::spike_points(std::slice::from_ref(&parts[i]), outline)
                        <= crate::quality::spike_points(std::slice::from_ref(&old), outline)
                    && no_short_stitches(std::slice::from_ref(&old), std::slice::from_ref(&parts[i]), cap)
                {
                    best = Some((after, parts[i].clone()));
                }
                parts[i] = old;
            }
            if let Some((_, part)) = best {
                parts[i] = part;
                changed += 1;
            }
        }
    }
    changed
}

/// Makes the ends of the satin columns of `parts` better where the outline
/// has a part with no stitches at an end (see the module notes):
///
/// 1. An end that turns to the column that it meets (the stroke finder
///    does this at sharp corners) is cut back to the straight body of its
///    column and goes on straight to the outline. The ends of the other
///    columns then go on into the part that the turned end had.
/// 2. Each end goes on as it is to the outline.
///
/// A change stays only if the outline then has fewer cells with no
/// stitches. `outline` and `parts` are in the same units; `cap` is the
/// capital height. Returns the number of ends that changed.
pub fn extend_ends(parts: &mut [ObjectKind], outline: &Outline, cap: f64) -> usize {
    let cell = cap / CELLS;
    let mut changed = 0;
    for i in 0..parts.len() {
        for at_end in [false, true] {
            let Some((ra, rb)) = rails_to(&parts[i], at_end) else {
                continue;
            };
            let (a, b) = (ra[ra.len() - 1], rb[rb.len() - 1]);
            let width = a.distance(b);
            // The end part and the body before it; a short column has a
            // shorter body.
            let near = TAIL * width;
            let far = ((TAIL + BODY) * width).min(0.9 * length(&ra).min(length(&rb)));
            if far - near < MIN_BODY * width {
                continue;
            }
            let (Some(da), Some(db)) = (body_dir(&ra, near, far), body_dir(&rb, near, far)) else {
                continue;
            };
            let (Some(ca), Some(cb)) = (cut_back(&ra, near), cut_back(&rb, near)) else {
                continue;
            };
            let Some(d) = (da + db).normalized() else { continue };
            let turned =
                |end: Point, cut: Point, body: Point| (end - cut).normalized().is_none_or(|t| t.dot(body) < BENT_COS);
            if !(turned(a, ca[ca.len() - 1], da) || turned(b, cb[cb.len() - 1], db)) {
                continue;
            }
            let Some(before) = faults(parts, outline, cap) else {
                return changed;
            };
            let mut trial = parts.to_vec();
            trial[i] = with_rails(&parts[i], ca.clone(), cb.clone(), at_end, near);
            let Some(straight) = longer(&trial, i, outline, cap, (&ca, &cb), d, at_end) else {
                continue;
            };
            trial[i] = straight;
            let others = lengthen(&mut trial, outline, cap, Some(i));
            let min_gain = (MIN_GAIN * width * width / (cell * cell)).max(2.0);
            if faults(&trial, outline, cap).is_some_and(|after| better(before, after, min_gain))
                && no_new_spikes(parts, &trial, outline, cap)
            {
                parts.clone_from_slice(&trial);
                changed += 1 + others;
            }
        }
    }
    changed + lengthen(parts, outline, cap, None)
}

/// A round part of the outline is not larger than this (a part of the
/// capital height).
const BLOB_MAX: f64 = 0.45;
/// The area of a round part is at least this part of the area of its box.
const BLOB_FULL: f64 = 0.42;
/// The column of a round part starts and ends this far from the ends of
/// the part (a part of its length), so its first and last stitch have a
/// length.
const BLOB_CAP: f64 = 0.04;
/// A round part gets a new column if its column covers less than this.
const BLOB_COVER: f64 = 0.94;

fn ring_area(ring: &[Point]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The two sides of the closed line `ring` between the places `lo` and
/// `hi` along the direction `u` (from the point `c`): each side from `lo`
/// to `hi`.
fn sides(ring: &[Point], c: Point, u: Point, lo: f64, hi: f64) -> Option<(Vec<Point>, Vec<Point>)> {
    let n = ring.len();
    let s = |p: Point| (p - c).dot(u);
    let first = (0..n).min_by(|a, b| s(ring[*a]).total_cmp(&s(ring[*b])))?;
    let last = (0..n).max_by(|a, b| s(ring[*a]).total_cmp(&s(ring[*b])))?;
    // The points from `first` to `last`, forward or backward round the ring.
    let chain = |forward: bool| -> Vec<Point> {
        let mut out = vec![ring[first]];
        let mut i = first;
        while i != last {
            i = if forward { (i + 1) % n } else { (i + n - 1) % n };
            out.push(ring[i]);
        }
        out
    };
    // The part of a chain between `lo` and `hi`, with the points at the
    // two limits.
    let clip = |pts: Vec<Point>| -> Option<Vec<Point>> {
        let mut out: Vec<Point> = Vec::new();
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (sa, sb) = (s(a), s(b));
            for limit in [lo, hi] {
                if (sa < limit) != (sb < limit) && (sb - sa).abs() > 1e-12 {
                    out.push(a.lerp(b, (limit - sa) / (sb - sa)));
                }
            }
            if sb >= lo && sb <= hi {
                out.push(b);
            }
        }
        out.dedup_by(|a, b| a.distance(*b) < 1e-9);
        (out.len() >= 2).then_some(out)
    };
    Some((clip(chain(true))?, clip(chain(false))?))
}

/// A small round part of the outline (a dot, a comma) whose column does not
/// cover it gets a column whose rails are the two sides of the part, so the
/// stitches go to its edge. A part with two or more columns (the stroke
/// finder cut its short middle line at a corner: the dots of a colon, P21
/// Milestone 6) gets one column along its long axis in place of them.
/// Returns the number of columns that changed.
pub fn round_parts(parts: &mut Vec<ObjectKind>, outline: &Outline, cap: f64) -> usize {
    let mut changed = 0;
    for (ring, holes) in outline {
        let Some(bb) = BBox::from_points(ring.iter().copied()) else {
            continue;
        };
        let round = holes.is_empty()
            && bb.width().max(bb.height()) <= BLOB_MAX * cap
            && ring_area(ring) >= BLOB_FULL * bb.width() * bb.height();
        if !round {
            continue;
        }
        // The one column of this part of the outline: the middle of its
        // box or the middle of its length is in the part (the column of a
        // comma is a curve).
        let middle = |kind: &ObjectKind| {
            let (ra, rb) = rails_to(kind, true)?;
            let (a, b) = (cut_back(&ra, 0.5 * length(&ra))?, cut_back(&rb, 0.5 * length(&rb))?);
            Some(a.last()?.midpoint(*b.last()?))
        };
        let inside: Vec<usize> = (0..parts.len())
            .filter(|i| {
                let in_ring = |p: Point| emb_geom::region::ring_contains(ring, p);
                parts[*i].control_bbox().is_some_and(|b| in_ring(b.center())) || middle(&parts[*i]).is_some_and(in_ring)
            })
            .collect();
        if inside.is_empty() || !inside.iter().all(|&i| matches!(parts[i], ObjectKind::Satin { .. })) {
            continue;
        }
        let i = inside[0];
        let Some((ra, rb)) = rails_to(&parts[i], true) else {
            continue;
        };
        let one = [(ring.clone(), Vec::new())];
        let old: Vec<ObjectKind> = inside.iter().map(|&k| parts[k].clone()).collect();
        let Some(before) = faults(&old, &one, cap) else {
            continue;
        };
        let cells = ring_area(ring) / ((cap / CELLS) * (cap / CELLS));
        if inside.len() == 1 && (before.0 as f64) < (1.0 - BLOB_COVER) * cells {
            continue;
        }
        // The column goes the way of the old one; for more columns, along
        // the long axis of the part.
        let u = if inside.len() == 1 {
            let start = ra[0].midpoint(rb[0]);
            let end = ra[ra.len() - 1].midpoint(rb[rb.len() - 1]);
            (end - start).normalized()
        } else {
            long_axis(ring)
        };
        let Some(u) = u else { continue };
        let c = bb.center();
        let s = |p: Point| (p - c).dot(u);
        let (lo, hi) = ring
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(s(*p)), hi.max(s(*p))));
        let cut = BLOB_CAP * (hi - lo);
        let Some((a, b)) = sides(ring, c, u, lo + cut, hi - cut) else {
            continue;
        };
        let mut candidate = parts[i].clone();
        if let ObjectKind::Satin { geometry, .. } = &mut candidate {
            geometry.rail_a = polyline(&a);
            geometry.rail_b = polyline(&b);
            geometry.rungs.clear();
        }
        let after = faults(std::slice::from_ref(&candidate), &one, cap);
        // One column in place of more: as good as they are, or better.
        let good = if inside.len() == 1 {
            after.is_some_and(|after| better(before, after, 2.0))
        } else {
            after.is_some_and(|after| after.0 <= before.0 && after.1 <= before.1 + 3)
        };
        if good {
            parts[i] = candidate;
            for &k in inside[1..].iter().rev() {
                parts.remove(k);
            }
            changed += 1;
        }
    }
    changed
}

/// The direction of the long axis of a ring (of its points).
fn long_axis(ring: &[Point]) -> Option<Point> {
    let n = ring.len() as f64;
    if n < 3.0 {
        return None;
    }
    let c = ring.iter().fold(Point::new(0.0, 0.0), |a, &p| a + p) * (1.0 / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in ring {
        let d = *p - c;
        sxx += d.x * d.x;
        syy += d.y * d.y;
        sxy += d.x * d.y;
    }
    let angle = 0.5 * emb_geom::math::atan2(2.0 * sxy, sxx - syy);
    Some(Point::new(emb_geom::math::cos(angle), emb_geom::math::sin(angle)))
}

#[cfg(test)]
mod tests {
    use emb_model::SatinGeometry;

    use super::*;

    fn column(a0: (f64, f64), a1: (f64, f64), b0: (f64, f64), b1: (f64, f64)) -> ObjectKind {
        let p = |v: (f64, f64)| Point::new(v.0, v.1);
        ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a: Path::new().move_to(p(a0)).line_to(p(a1)),
                rail_b: Path::new().move_to(p(b0)).line_to(p(b1)),
                rungs: Vec::new(),
            },
            params: Default::default(),
        }
    }

    fn rail_ends(k: &ObjectKind) -> ((Point, Point), (Point, Point)) {
        let ObjectKind::Satin { geometry, .. } = k else {
            panic!("a column")
        };
        let e = |path: &Path| {
            let v: Vec<Point> = path.points().collect();
            (v[0], v[v.len() - 1])
        };
        (e(&geometry.rail_a), e(&geometry.rail_b))
    }

    #[test]
    fn an_end_goes_on_to_the_outline() {
        // A bar 4 mm high and 20 mm long; its column stops at 14 mm.
        let outline = vec![(
            vec![
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 4.0),
                Point::new(0.0, 4.0),
            ],
            Vec::new(),
        )];
        let mut parts = vec![column((0.0, 0.0), (14.0, 0.0), (0.0, 4.0), (14.0, 4.0))];
        assert_eq!(extend_ends(&mut parts, &outline, 25.0), 1);
        let ((a0, a1), (b0, b1)) = rail_ends(&parts[0]);
        assert!(a0.distance(Point::new(0.0, 0.0)) < 1e-9 && b0.distance(Point::new(0.0, 4.0)) < 1e-9);
        assert!((a1.x - 20.0).abs() < 0.4 && (b1.x - 20.0).abs() < 0.4, "{a1:?} {b1:?}");
        assert!(a1.y.abs() < 1e-9 && (b1.y - 4.0).abs() < 1e-9);
        // The column now covers the outline: no more change.
        assert_eq!(extend_ends(&mut parts, &outline, 25.0), 0);
    }

    #[test]
    fn an_end_at_a_slanted_edge_goes_on_with_one_rail() {
        // The outline ends with a slanted edge: its top side is 6 mm
        // longer than its bottom side.
        let outline = vec![(
            vec![
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(14.0, 4.0),
                Point::new(0.0, 4.0),
            ],
            Vec::new(),
        )];
        let mut parts = vec![column((0.0, 0.0), (10.0, 0.0), (0.0, 4.0), (10.0, 4.0))];
        assert_eq!(extend_ends(&mut parts, &outline, 25.0), 1);
        let ((_, a1), (_, b1)) = rail_ends(&parts[0]);
        assert!((b1.x - 14.0).abs() < 1.1, "the short side: {b1:?}");
        assert!(a1.x > 18.0 && a1.x < 20.5, "the long side: {a1:?}");
    }

    #[test]
    fn a_dot_with_two_columns_gets_one() {
        // P21 Milestone 6: the stroke finder cut the short middle line of a
        // dot at a corner (the colon of Pacifico); the two columns left
        // parts of the dot with no stitches.
        let ring: Vec<Point> = (0..48)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 48.0;
                Point::new(10.0 + 2.5 * a.cos(), -20.0 + 2.0 * a.sin())
            })
            .collect();
        let outline = vec![(ring, Vec::new())];
        let mut parts = vec![
            column((8.0, -19.0), (10.0, -19.0), (8.0, -21.0), (10.0, -21.0)),
            column((10.0, -19.0), (11.5, -18.5), (10.0, -21.0), (11.5, -20.5)),
        ];
        let before = faults(&parts, &outline, 25.0).unwrap();
        assert_eq!(round_parts(&mut parts, &outline, 25.0), 1);
        assert_eq!(parts.len(), 1);
        let after = faults(&parts, &outline, 25.0).unwrap();
        assert!(after.0 < before.0, "{before:?} {after:?}");
    }

    #[test]
    fn an_end_does_not_go_on_out_of_the_outline() {
        // P21 Milestone 6: a change of an end that adds needle points out of
        // the outline does not stay.
        let outline = vec![(
            vec![
                Point::new(0.0, -2.0),
                Point::new(10.0, -2.0),
                Point::new(10.0, 2.0),
                Point::new(0.0, 2.0),
            ],
            Vec::new(),
        )];
        let parts = vec![column((0.0, -2.0), (8.0, -2.0), (0.0, 2.0), (8.0, 2.0))];
        let longer = vec![column((0.0, -2.0), (11.0, -2.0), (0.0, 2.0), (11.0, 2.0))];
        let shorter = vec![column((0.0, -2.0), (10.0, -2.0), (0.0, 2.0), (10.0, 2.0))];
        assert!(!no_new_spikes(&parts, &longer, &outline, 25.0));
        assert!(no_new_spikes(&parts, &shorter, &outline, 25.0));
    }

    #[test]
    fn a_dot_gets_a_column_to_its_edge() {
        // A dot 5 mm wide, with a square column in it.
        let ring: Vec<Point> = (0..48)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 48.0;
                Point::new(10.0 + 2.5 * a.cos(), -20.0 + 2.5 * a.sin())
            })
            .collect();
        let outline = vec![(ring, Vec::new())];
        let mut parts = vec![column((8.2, -18.2), (8.2, -21.8), (11.8, -18.2), (11.8, -21.8))];
        let before = faults(&parts, &outline, 25.0).unwrap();
        assert_eq!(round_parts(&mut parts, &outline, 25.0), 1);
        let after = faults(&parts, &outline, 25.0).unwrap();
        assert!(after.0 * 4 < before.0, "{before:?} {after:?}");
        assert!(after.1 <= before.1 + 3, "{before:?} {after:?}");
        // The column goes the same way: from the bottom to the top.
        let ((a0, a1), (b0, b1)) = rail_ends(&parts[0]);
        assert!(a0.y > a1.y && b0.y > b1.y);
        assert!(a0.distance(b0) > 0.5 && a1.distance(b1) > 0.5, "the ends have a width");
        // A column that covers its part stays.
        assert_eq!(round_parts(&mut parts, &outline, 25.0), 0);
        // A large part of the outline is not a dot.
        let bar = vec![(
            vec![
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 4.0),
                Point::new(0.0, 4.0),
            ],
            Vec::new(),
        )];
        let mut short = vec![column((0.0, 0.0), (10.0, 0.0), (0.0, 4.0), (10.0, 4.0))];
        assert_eq!(round_parts(&mut short, &bar, 25.0), 0);
    }

    #[test]
    fn an_end_in_another_column_stays() {
        // A "T": the stem ends in the bar, and the outline has no free
        // part there.
        let outline = vec![(
            vec![
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 4.0),
                Point::new(12.0, 4.0),
                Point::new(12.0, 25.0),
                Point::new(8.0, 25.0),
                Point::new(8.0, 4.0),
                Point::new(0.0, 4.0),
            ],
            Vec::new(),
        )];
        let mut parts = vec![
            column((0.0, 0.0), (20.0, 0.0), (0.0, 4.0), (20.0, 4.0)),
            column((8.0, 25.0), (8.0, 3.0), (12.0, 25.0), (12.0, 3.0)),
        ];
        let before = parts.clone();
        assert_eq!(extend_ends(&mut parts, &outline, 25.0), 0);
        assert_eq!(parts, before);
    }
}
