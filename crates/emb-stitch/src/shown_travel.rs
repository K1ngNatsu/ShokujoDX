//! Travel that shows (P22 Milestone 1, ADR 0167): the travel stitches of a
//! plan that no stitch sewn after them covers, where they lie on top of
//! stitches sewn before them or on the fabric.
//!
//! Not a port. The travel stitches are the records of
//! `StitchPlan::travel`. Each travel stitch is measured at points
//! [`STEP_MM`] apart:
//!
//! - a stitch that is not travel and sews after it, nearer than
//!   [`COVER_MM`], covers the point: it does not show;
//! - else the stitch sewn last before it at the point is the top of the
//!   embroidery there. If the travel crosses it (at [`CROSS_DEG`] or more),
//!   the travel shows on top of sewn stitches, but at the edge of an area:
//!   where the needle path of the stitches under it turns (the ends of the
//!   rows, the sides of a satin column) nearer than [`EDGE_MM`]. Travel
//!   along the edge goes 0.3 mm inside it, and the rows go out over the
//!   edge by their pull compensation: the travel lies in the turns. If the
//!   travel goes along the top stitch (nearer than [`GROOVE_MM`]), it lies
//!   between the rows: it does not show;
//! - else, with no stitch near, the travel shows on the fabric.
//!
//! The first and the last [`END_MM`] of each travel run join the stitches
//! before and after it, and are not measured.

use std::collections::HashMap;

use emb_geom::Point;
use emb_model::{ObjectId, StitchKind};

use crate::plan::StitchPlan;

/// A stitch covers a travel stitch at most this far from it, mm (the half
/// widths of two threads).
pub const COVER_MM: f64 = 0.3;
/// The distance of the points that are measured on a travel stitch, mm.
pub const STEP_MM: f64 = 0.2;
/// The ends of a travel run that are not measured, mm.
pub const END_MM: f64 = 0.5;
/// A travel stitch crosses a stitch under it at this angle or more, degrees.
pub const CROSS_DEG: f64 = 20.0;
/// A travel stitch this near to a turn of the needle path of the stitches
/// under it (the end of a row, the side of a satin column) is at the edge
/// of an area, mm.
pub const EDGE_MM: f64 = 1.2;
/// A travel stitch along a stitch this near lies between the rows, mm
/// (about a row spacing).
pub const GROOVE_MM: f64 = 0.45;
/// The needle path turns at a needle point where the direction changes by
/// more than this, degrees.
const TURN_DEG: f64 = 60.0;
/// The size of a cell of the grid of stitches, mm.
const CELL_MM: f64 = 1.0;

/// Where a piece of travel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shows {
    /// On top of stitches sewn before it.
    OnStitches,
    /// On the fabric.
    OnFabric,
}

/// A piece of a travel stitch that shows (output coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct ShownPiece {
    pub from: Point,
    pub to: Point,
    pub shows: Shows,
    /// The object that the travel stitch is a part of.
    pub object: Option<ObjectId>,
    /// The record of the travel stitch where the piece starts.
    pub record: usize,
}

impl ShownPiece {
    pub fn length(&self) -> f64 {
        self.from.distance(self.to)
    }
}

/// The travel of a plan that shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShownTravel {
    pub pieces: Vec<ShownPiece>,
    /// The length of all travel stitches of the plan, mm.
    pub travel_mm: f64,
}

impl ShownTravel {
    /// The length of the travel that shows on top of sewn stitches, mm.
    pub fn on_stitches_mm(&self) -> f64 {
        self.length(Shows::OnStitches)
    }

    /// The length of the travel that shows on the fabric, mm.
    pub fn on_fabric_mm(&self) -> f64 {
        self.length(Shows::OnFabric)
    }

    fn length(&self, shows: Shows) -> f64 {
        self.pieces
            .iter()
            .filter(|p| p.shows == shows)
            .map(ShownPiece::length)
            .fold(0.0, |a, b| a + b)
    }

    /// The travel that shows of each object with travel that shows, in the
    /// order of the plan (one pass over the pieces).
    pub fn by_object(&self) -> Vec<ObjectTravel> {
        let mut out: Vec<ObjectTravel> = Vec::new();
        let mut index: HashMap<ObjectId, usize> = HashMap::new();
        for (i, p) in self.pieces.iter().enumerate() {
            let Some(id) = p.object else { continue };
            let k = *index.entry(id).or_insert_with(|| {
                out.push(ObjectTravel {
                    object: id,
                    on_stitches_mm: 0.0,
                    longest_on_stitches: None,
                    longest_on_fabric: None,
                });
                out.len() - 1
            });
            let o = &mut out[k];
            let len = p.length();
            let longest = match p.shows {
                Shows::OnStitches => {
                    o.on_stitches_mm += len;
                    &mut o.longest_on_stitches
                }
                Shows::OnFabric => &mut o.longest_on_fabric,
            };
            // The last of the longest pieces, as `Iterator::max_by`.
            if longest.is_none_or(|j| self.pieces[j].length() <= len) {
                *longest = Some(i);
            }
        }
        out
    }
}

/// The travel that shows of one object (see [`ShownTravel::by_object`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTravel {
    pub object: ObjectId,
    /// The length on top of sewn stitches, mm.
    pub on_stitches_mm: f64,
    /// The longest piece on top of sewn stitches and on the fabric (indices
    /// in [`ShownTravel::pieces`]).
    pub longest_on_stitches: Option<usize>,
    pub longest_on_fabric: Option<usize>,
}

/// One stitch of the plan: from the needle to the record, at the place of
/// the record in the plan.
#[derive(Debug, Clone, Copy)]
struct Seg {
    a: Point,
    b: Point,
    record: usize,
    /// The needle path turns (or starts, or ends) at `a`, at `b`.
    turn_a: bool,
    turn_b: bool,
}

/// The travel of `plan` that shows.
pub fn shown_travel(plan: &StitchPlan) -> ShownTravel {
    let recs = &plan.stitches.records;
    let mut travel_ranges = plan.travel.iter().peekable();
    let turn_cos = emb_geom::math::sin_cos_deg(TURN_DEG).1;
    // The stitches that are not travel, with the turns of the needle path.
    let mut stitches: Vec<Seg> = Vec::with_capacity(recs.len());
    // The travel stitches, in runs: each run starts where the one before
    // it stops.
    let mut runs: Vec<Vec<Seg>> = Vec::new();
    let mut needle: Option<Point> = None;
    // The stitch before, if the thread goes on from it: its direction and,
    // if it is not travel, its place in `stitches`.
    let mut before: Option<(Point, Option<usize>)> = None;
    for (i, r) in recs.iter().enumerate() {
        if r.kind != StitchKind::Stitch {
            if matches!(r.kind, StitchKind::Jump | StitchKind::Trim) {
                needle = Some(r.pos);
                before = None;
            }
            continue;
        }
        let Some(a) = needle.replace(r.pos) else { continue };
        let d = r.pos - a;
        let len = d.length();
        if len <= 1e-6 {
            continue;
        }
        while travel_ranges.peek().is_some_and(|t| t.end <= i) {
            travel_ranges.next();
        }
        let travel = travel_ranges.peek().is_some_and(|t| t.contains(&i));
        let turn = before.is_none_or(|(u, _)| u.dot(d) < turn_cos * u.length() * len);
        let joined = before.is_some();
        if let Some((_, Some(k))) = before {
            stitches[k].turn_b = turn;
        }
        let seg = Seg {
            a,
            b: r.pos,
            record: i,
            turn_a: turn,
            turn_b: true,
        };
        if travel {
            let goes_on = joined && before.is_some_and(|(_, k)| k.is_none());
            match runs.last_mut() {
                Some(run) if goes_on => run.push(seg),
                _ => runs.push(vec![seg]),
            }
            before = Some((d, None));
        } else {
            stitches.push(seg);
            before = Some((d, Some(stitches.len() - 1)));
        }
    }
    let travel_mm = runs.iter().flatten().map(|s| s.a.distance(s.b)).fold(0.0, |a, b| a + b);
    if runs.is_empty() {
        return ShownTravel::default();
    }
    // The cells that the travel goes through, and the stitches and the
    // turns of the needle path near them. A cell is on a stitch if its
    // middle is this near to it.
    let on = CELL_MM * std::f64::consts::FRAC_1_SQRT_2;
    let mut wanted = CellMap::new(CELL_MM);
    // The cells of the turns: as far as [`EDGE_MM`] (two cells) round the
    // cells of the travel.
    let mut turn_cells = CellMap::new(CELL_MM);
    for t in runs.iter().flatten() {
        for c in wanted.cells_of(t.a, t.b, 0.0) {
            if wanted.middle(c).distance_to_segment(t.a, t.b) <= on {
                wanted.mark(c);
                for dx in -TURN_REACH..=TURN_REACH {
                    for dy in -TURN_REACH..=TURN_REACH {
                        turn_cells.mark((c.0 + dx, c.1 + dy));
                    }
                }
            }
        }
    }
    // Most stitches are far from all travel: the blocks of the marked cells
    // leave them out with one look.
    let mut grid: Vec<Vec<u32>> = vec![Vec::new(); wanted.count];
    let mut turns: Vec<Vec<Point>> = vec![Vec::new(); turn_cells.count];
    let reach = GROOVE_MM + on;
    for (k, s) in stitches.iter().enumerate() {
        wanted.each_marked_in(s.a, s.b, GROOVE_MM, |c, slot| {
            if wanted.middle(c).distance_to_segment(s.a, s.b) <= reach {
                grid[slot].push(k as u32);
            }
        });
        for (turn, q) in [(s.turn_a, s.a), (s.turn_b, s.b)] {
            if turn && let Some(slot) = turn_cells.slot(turn_cells.cell(q)) {
                turns[slot].push(q);
            }
        }
    }
    let cell = |p: Point| wanted.cell(p);
    let objects = record_objects(plan);
    let cross_sin = emb_geom::math::sin_cos_deg(CROSS_DEG).0;
    let mut pieces: Vec<ShownPiece> = Vec::new();
    for run in &runs {
        let total: f64 = run.iter().map(|s| s.a.distance(s.b)).sum();
        let mut along = 0.0;
        for t in run {
            let len = t.a.distance(t.b);
            let dir = (t.b - t.a) * (1.0 / len);
            let n = (len / STEP_MM).ceil().max(1.0) as usize;
            let step = len / n as f64;
            let object = object_at(&objects, t.record);
            for k in 0..n {
                let mid = along + (k as f64 + 0.5) * step;
                if mid < END_MM || total - mid < END_MM {
                    continue;
                }
                let p = t.a + dir * ((k as f64 + 0.5) * step);
                let near = Near {
                    stitches: &stitches,
                    cell: wanted.slot(cell(p)).map_or(&[], |k| grid[k].as_slice()),
                    turn_cells: &turn_cells,
                    turns: &turns,
                };
                let Some(shows) = shows_at(p, dir, t.record, &near, cross_sin) else {
                    continue;
                };
                let (from, to) = (t.a + dir * (k as f64 * step), t.a + dir * ((k + 1) as f64 * step));
                match pieces.last_mut() {
                    Some(last) if last.shows == shows && last.object == object && last.to.distance(from) < 1e-9 => {
                        last.to = to;
                    }
                    _ => pieces.push(ShownPiece {
                        from,
                        to,
                        shows,
                        object,
                        record: t.record,
                    }),
                }
            }
            along += len;
        }
    }
    ShownTravel { pieces, travel_mm }
}

/// A map of square cells of `size` mm with marked cells. A marked cell
/// has a number (its slot), from 0 to `count`. The map keeps only the
/// blocks of cells that have a marked cell (the audit of P22: a map of all
/// the cells of the box of the travel needed 10^10 cells for travel 100 m
/// apart, and design info makes this measure for each design).
struct CellMap {
    size: f64,
    /// The blocks of [`BLOCK`] × [`BLOCK`] cells with a marked cell: the
    /// slot of each cell of the block, or `u32::MAX`.
    blocks: HashMap<(i64, i64), Box<[u32; CELLS_IN_BLOCK]>, Cells>,
    /// The first and the last cell of the box of the marked cells.
    bounds: ((i64, i64), (i64, i64)),
    count: usize,
}

/// The cells of a block of [`CellMap`].
const CELLS_IN_BLOCK: usize = (BLOCK * BLOCK) as usize;

/// A quick hash for the cells of a [`CellMap`] (two whole numbers; the
/// hash of the standard library is made for keys from outside).
#[derive(Clone, Copy, Default)]
struct Cells;

impl std::hash::BuildHasher for Cells {
    type Hasher = CellHasher;
    fn build_hasher(&self) -> CellHasher {
        CellHasher(0)
    }
}

#[derive(Clone, Copy)]
struct CellHasher(u64);

impl std::hash::Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(u64::from(*b));
        }
    }

    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(26) ^ v).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// The side of a block of [`CellMap`], cells.
const BLOCK: i64 = 16;

impl CellMap {
    fn new(size: f64) -> CellMap {
        CellMap {
            size,
            blocks: HashMap::default(),
            bounds: ((i64::MAX, i64::MAX), (i64::MIN, i64::MIN)),
            count: 0,
        }
    }

    fn cell(&self, p: Point) -> (i64, i64) {
        ((p.x / self.size).floor() as i64, (p.y / self.size).floor() as i64)
    }

    /// The middle of the cell `c`.
    fn middle(&self, c: (i64, i64)) -> Point {
        Point::new((c.0 as f64 + 0.5) * self.size, (c.1 as f64 + 0.5) * self.size)
    }

    /// The cells of the box of `a` and `b`, larger by `grow` mm.
    fn cells_of(&self, a: Point, b: Point, grow: f64) -> impl Iterator<Item = (i64, i64)> + use<> {
        let (lo, hi) = self.box_of(a, b, grow);
        (lo.0..=hi.0).flat_map(move |x| (lo.1..=hi.1).map(move |y| (x, y)))
    }

    /// The first and the last cell of the box of `a` and `b`, larger by
    /// `grow` mm.
    fn box_of(&self, a: Point, b: Point, grow: f64) -> ((i64, i64), (i64, i64)) {
        let lo = self.cell(Point::new(a.x.min(b.x) - grow, a.y.min(b.y) - grow));
        let hi = self.cell(Point::new(a.x.max(b.x) + grow, a.y.max(b.y) + grow));
        (lo, hi)
    }

    /// The block of the cell `c`, and the place of the cell in it.
    fn in_block(c: (i64, i64)) -> ((i64, i64), usize) {
        let block = (c.0.div_euclid(BLOCK), c.1.div_euclid(BLOCK));
        (block, (c.1.rem_euclid(BLOCK) * BLOCK + c.0.rem_euclid(BLOCK)) as usize)
    }

    fn mark(&mut self, c: (i64, i64)) {
        let (block, i) = Self::in_block(c);
        let cells = self
            .blocks
            .entry(block)
            .or_insert_with(|| Box::new([u32::MAX; CELLS_IN_BLOCK]));
        if cells[i] == u32::MAX {
            cells[i] = self.count as u32;
            self.count += 1;
            let (lo, hi) = &mut self.bounds;
            *lo = (lo.0.min(c.0), lo.1.min(c.1));
            *hi = (hi.0.max(c.0), hi.1.max(c.1));
        }
    }

    fn slot(&self, c: (i64, i64)) -> Option<usize> {
        let ((x0, y0), (x1, y1)) = self.bounds;
        if c.0 < x0 || c.0 > x1 || c.1 < y0 || c.1 > y1 {
            return None;
        }
        let (block, i) = Self::in_block(c);
        let s = self.blocks.get(&block)?[i];
        (s != u32::MAX).then_some(s as usize)
    }

    /// Calls `f` for each marked cell of the box of `a` and `b`, larger by
    /// `grow` mm, with its slot: through the blocks of the box, or through
    /// all the blocks when the box has more.
    fn each_marked_in(&self, a: Point, b: Point, grow: f64, mut f: impl FnMut((i64, i64), usize)) {
        let (mut lo, mut hi) = self.box_of(a, b, grow);
        // Most stitches are far from all travel.
        lo = (lo.0.max(self.bounds.0.0), lo.1.max(self.bounds.0.1));
        hi = (hi.0.min(self.bounds.1.0), hi.1.min(self.bounds.1.1));
        if lo.0 > hi.0 || lo.1 > hi.1 {
            return;
        }
        let (b0, b1) = (
            (lo.0.div_euclid(BLOCK), lo.1.div_euclid(BLOCK)),
            (hi.0.div_euclid(BLOCK), hi.1.div_euclid(BLOCK)),
        );
        let mut in_block = |block: (i64, i64), cells: &[u32; CELLS_IN_BLOCK]| {
            let (x0, y0) = (block.0 * BLOCK, block.1 * BLOCK);
            for y in lo.1.max(y0)..=hi.1.min(y0 + BLOCK - 1) {
                for x in lo.0.max(x0)..=hi.0.min(x0 + BLOCK - 1) {
                    let s = cells[((y - y0) * BLOCK + (x - x0)) as usize];
                    if s != u32::MAX {
                        f((x, y), s as usize);
                    }
                }
            }
        };
        let in_box = (b1.0 - b0.0 + 1).saturating_mul(b1.1 - b0.1 + 1);
        if in_box <= self.blocks.len() as i64 {
            for bx in b0.0..=b1.0 {
                for by in b0.1..=b1.1 {
                    if let Some(cells) = self.blocks.get(&(bx, by)) {
                        in_block((bx, by), cells);
                    }
                }
            }
        } else {
            for (block, cells) in &self.blocks {
                in_block(*block, cells);
            }
        }
    }
}

/// The cells round a cell where a turn of the needle path can be nearer
/// than [`EDGE_MM`] to a point of the cell.
const TURN_REACH: i64 = 2;

/// The stitches near a point that is measured.
struct Near<'a> {
    stitches: &'a [Seg],
    /// The stitches of the cell of the point (indices in `stitches`).
    cell: &'a [u32],
    /// The turns of the needle path near the travel, by cell.
    turn_cells: &'a CellMap,
    turns: &'a [Vec<Point>],
}

impl Near<'_> {
    /// A turn of the needle path is nearer than [`EDGE_MM`] to `p`.
    fn turn_near(&self, p: Point) -> bool {
        let (x, y) = self.turn_cells.cell(p);
        (-TURN_REACH..=TURN_REACH).any(|dx| {
            (-TURN_REACH..=TURN_REACH).any(|dy| {
                self.turn_cells
                    .slot((x + dx, y + dy))
                    .is_some_and(|k| self.turns[k].iter().any(|q| q.distance(p) < EDGE_MM))
            })
        })
    }
}

/// Where the travel stitch of record `record` shows at `p` (it goes in the
/// direction `dir`), from the stitches near it.
fn shows_at(p: Point, dir: Point, record: usize, near: &Near, cross_sin: f64) -> Option<Shows> {
    // The stitch sewn last before the travel at `p`, and if the travel
    // crosses it.
    let mut top: Option<(usize, bool)> = None;
    for &k in near.cell {
        let s = near.stitches[k as usize];
        let d = p.distance_to_segment(s.a, s.b);
        if d > GROOVE_MM {
            continue;
        }
        let crosses = (dir.cross(s.b - s.a) / s.a.distance(s.b)).abs() >= cross_sin;
        if d > COVER_MM && crosses {
            continue;
        }
        if s.record > record {
            if d <= COVER_MM {
                return None;
            }
            continue;
        }
        if top.is_none_or(|(r, _)| s.record > r) {
            top = Some((s.record, crosses));
        }
    }
    match top {
        None => Some(Shows::OnFabric),
        Some((_, false)) => None,
        Some((_, true)) if near.turn_near(p) => None,
        Some((_, true)) => Some(Shows::OnStitches),
    }
}

/// The objects of the plan by their records: (first record, end, object), in order.
fn record_objects(plan: &StitchPlan) -> Vec<(usize, usize, ObjectId)> {
    let mut v: Vec<(usize, usize, ObjectId)> = plan
        .spans
        .iter()
        .map(|s| (s.records.start, s.records.end, s.object))
        .collect();
    v.sort_by_key(|s| s.0);
    v
}

fn object_at(objects: &[(usize, usize, ObjectId)], record: usize) -> Option<ObjectId> {
    let k = objects.partition_point(|o| o.0 <= record);
    let o = objects.get(k.checked_sub(1)?)?;
    (record < o.1).then_some(o.2)
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{Design, FillParams, ObjectKind, Rgb, StitchDesign, StitchRecord, Thread, TrimMode, Underlay};

    use super::*;
    use crate::{PlanOptions, StitchCache, build_plan};

    /// A plan of stitches and travel: each item is a line of needle points
    /// and if it is travel. The lines follow one another with no trim.
    fn plan(lines: &[(&[(f64, f64)], bool)]) -> StitchPlan {
        let mut records = vec![StitchRecord::new(
            Point::new(lines[0].0[0].0, lines[0].0[0].1),
            StitchKind::Jump,
        )];
        let mut travel: Vec<std::ops::Range<usize>> = Vec::new();
        for (pts, is_travel) in lines {
            let start = records.len();
            for &(x, y) in pts.iter() {
                records.push(StitchRecord::new(Point::new(x, y), StitchKind::Stitch));
            }
            if *is_travel {
                // The stitch to the first point of the line is travel too.
                travel.push(start..records.len());
            }
        }
        records.push(StitchRecord::new(records.last().unwrap().pos, StitchKind::End));
        StitchPlan {
            stitches: StitchDesign {
                threads: vec![Thread::default()],
                records,
            },
            travel,
            ..StitchPlan::default()
        }
    }

    /// Rows from x = 0 to 20 mm, 0.4 mm apart, from y = `y0` for `n` rows.
    fn rows(y0: f64, n: usize) -> Vec<(f64, f64)> {
        let mut v = Vec::new();
        for k in 0..n {
            let y = y0 + 0.4 * k as f64;
            if k % 2 == 0 {
                v.extend([(0.0, y), (5.0, y), (10.0, y), (15.0, y), (20.0, y)]);
            } else {
                v.extend([(20.0, y), (15.0, y), (10.0, y), (5.0, y), (0.0, y)]);
            }
        }
        v
    }

    #[test]
    fn travel_across_sewn_rows_shows() {
        // 25 rows (y 0 to 9.6), then travel straight across them, from the
        // last row up to y = 0, as the travel of before ADR 0166.
        let r = rows(0.0, 25);
        let p = plan(&[(&r, false), (&[(5.0, 9.6), (5.0, 6.6), (5.0, 3.6), (5.0, 0.0)], true)]);
        let s = shown_travel(&p);
        assert!((s.travel_mm - 24.6).abs() < 1e-6, "{}", s.travel_mm);
        // The travel from (0, 9.6) to (5, 9.6) lies along the last row; the
        // 9.6 mm up across the rows shows, but its last 0.5 mm.
        assert!((s.on_stitches_mm() - 9.1).abs() < 0.25, "{}", s.on_stitches_mm());
        assert_eq!(s.on_fabric_mm(), 0.0);
    }

    #[test]
    fn travel_under_rows_sewn_after_it_does_not_show() {
        // Travel up on the fabric, then the rows over it.
        let r = rows(0.0, 25);
        let p = plan(&[(&[(5.0, 9.6), (5.0, 6.6), (5.0, 3.6), (5.0, 0.0)], true), (&r, false)]);
        let s = shown_travel(&p);
        assert_eq!(s.pieces, Vec::new());
    }

    #[test]
    fn travel_along_the_rows_and_at_their_turns_does_not_show() {
        let r = rows(0.0, 25);
        // Along the rows, between two of them (y 4.2), and along the turns
        // of the rows at x = 20.1.
        let p = plan(&[
            (&r, false),
            (&[(20.1, 9.6), (20.1, 4.2), (15.0, 4.2), (10.0, 4.2), (5.0, 4.2)], true),
        ]);
        let s = shown_travel(&p);
        assert_eq!(s.on_stitches_mm(), 0.0, "{:?}", s.pieces);
        assert_eq!(s.on_fabric_mm(), 0.0, "{:?}", s.pieces);
    }

    #[test]
    fn travel_on_the_fabric_shows() {
        let r = rows(0.0, 5);
        let p = plan(&[(&r, false), (&[(20.0, 4.0), (20.0, 7.0)], true), (&rows(8.0, 5), false)]);
        let s = shown_travel(&p);
        // From the end of the last row (20, 1.6) up to y 7, but the ends
        // of the travel run: 1.6 + 0.5 to 7 - 0.5.
        assert_eq!(s.on_stitches_mm(), 0.0);
        assert!((s.on_fabric_mm() - 4.4).abs() < 0.25, "{}", s.on_fabric_mm());
    }

    /// A design: a disc with a hexagon hole, sewn by the plan. The travel
    /// between the sections of the fill does not show (ADR 0166), and the
    /// plan marks it.
    #[test]
    fn the_travel_of_a_fill_with_a_hole_is_marked_and_hidden() {
        let mut d = Design::new("disc");
        let t = d.add_thread("Navy", Rgb(20, 30, 90));
        let hexagon: Vec<Point> = (0..6)
            .map(|i| {
                let (s, c) = emb_geom::math::sin_cos_deg(60.0 * f64::from(i) + 30.0);
                Point::new(15.0 * c, 15.0 * s)
            })
            .collect();
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::ellipse(Point::ZERO, 39.0, 39.0),
                holes: vec![Path::polygon(&hexagon)],
                params: FillParams {
                    angle_deg: 45.0,
                    underlay: Underlay::None,
                    ..FillParams::default()
                },
            },
        );
        let p = build_plan(&d, &PlanOptions::default(), &mut StitchCache::default());
        let s = shown_travel(&p);
        assert!(s.travel_mm > 10.0, "the plan marks the travel: {}", s.travel_mm);
        assert!(s.on_stitches_mm() < 1.0, "{:?}", s.pieces);
    }

    /// Hidden travel (ADR 0082) between two objects, under an object that
    /// sews after it: it does not show. With a trim at a long gap there is
    /// no travel.
    #[test]
    fn hidden_travel_does_not_show() {
        let mut d = Design::new("hidden");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        let square = |x: f64, w: f64, h: f64| ObjectKind::Fill {
            outer: Path::rect(emb_geom::BBox::new(Point::new(x, -h / 2.0), Point::new(x + w, h / 2.0))),
            holes: Vec::new(),
            params: FillParams {
                underlay: Underlay::None,
                ..FillParams::default()
            },
        };
        let a = d.add_object(red, square(-30.0, 10.0, 10.0));
        d.add_object(red, square(20.0, 10.0, 10.0));
        // A band in another color, sewn last, covers both and the way.
        d.add_object(blue, square(-32.0, 64.0, 12.0));
        d.object_mut(a).unwrap().functions.trim = TrimMode::Hidden;
        let p = build_plan(&d, &PlanOptions::default(), &mut StitchCache::default());
        let s = shown_travel(&p);
        assert!(s.travel_mm > 20.0, "hidden travel: {}", s.travel_mm);
        assert_eq!(s.on_stitches_mm() + s.on_fabric_mm(), 0.0, "{:?}", s.pieces);
        let mut d2 = d.clone();
        d2.object_mut(a).unwrap().functions.trim = TrimMode::Auto;
        let p = build_plan(&d2, &PlanOptions::default(), &mut StitchCache::default());
        assert_eq!(shown_travel(&p).travel_mm, 0.0, "a trim, no travel");
    }
}
