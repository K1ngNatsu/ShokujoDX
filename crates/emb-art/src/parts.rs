//! The parts of a stitch file (P18 Milestone 1, ADR 0131): the stitches of
//! a machine file cut into parts of one kind, the first step to make
//! objects with stitch types from them.
//!
//! Not a port. The method:
//!
//! 1. **Chains**: the stitches between trims, jumps, stops and color
//!    changes (a stitch longer than [`MAX_STITCH_MM`] is a jump too).
//! 2. **Legs**: in a chain, the stitches that go on in about the same
//!    direction (a turn of less than [`LEG_TURN_DEG`]) make a leg. A stitch
//!    shorter than [`TINY_MM`] (a lock stitch) has no direction of its own.
//! 3. **Back and forth**: a sequence of legs in which each leg goes back
//!    along the one before it (a short turn leg between them can be there)
//!    is a **column** (stitches from side to side: one stitch, or a few, to
//!    cross) or **rows** (long parallel lines of several stitches each, at
//!    one angle). Legs that go back on the same line into their needle
//!    holes (a bean stitch) are a run. The other legs are a **run** (stitches along a line). A short
//!    piece of a chain that is none of these is **other**.
//! 4. **Underlay**: a part that a later part of the same color covers (the
//!    area of its stitches, [`COVER_CELL_MM`] cells) goes with that part:
//!    the underlay and the travel under a column or a fill. A column or
//!    rows is underlay if it is lighter: less thread in a cell, or (under
//!    rows) a row spacing 1.25 times as open as the rows over them, or (under
//!    rows) a short part, too short to show its spacing. A run,
//!    a short part or a lighter part between two parts of the underlay of a
//!    part goes with it too. A run or a
//!    lighter part between two parts of the underlay of a part goes with it
//!    too.
//!
//! A part that the program does not recognize stays a stitch part; no
//! stitch is lost (the plan of the phase).

use std::ops::Range;

use emb_geom::Point;
use emb_model::{StitchDesign, StitchKind};

/// A stitch longer than this is a jump (a file that has no jumps), mm.
pub const MAX_STITCH_MM: f64 = 12.5;
/// The stitches of a leg turn less than this from one to the next, degrees.
pub const LEG_TURN_DEG: f64 = 30.0;
/// A stitch shorter than this has no direction of its own (a lock stitch),
/// mm.
pub const TINY_MM: f64 = 0.35;
/// The cells of the area that a part covers, mm.
pub const COVER_CELL_MM: f64 = 0.25;
/// The part of the cells of an underlay that the later parts cover.
const COVERED: f64 = 0.85;
/// An underlay has less thread in a cell than this part of the thread of
/// the part over it.
const LIGHTER: f64 = 0.8;
/// A part of at most this many stitches between two parts of the underlay
/// of a part goes with that part, and under rows it is underlay.
const BETWEEN_STITCHES: usize = 16;
/// Two legs go back and forth if the cosine of their angle is below this.
pub(crate) const BACK: f64 = -0.6;
/// The turn legs between two rows are at most this long, and at most half
/// as long as the shorter row, but this long can always be a turn, mm.
const TURN_MM: f64 = 4.0;
const TURN_MIN_MM: f64 = 1.0;
/// A column or rows have at least this many legs.
const MIN_LEGS: usize = 4;
/// Rows with no turns: the legs have at least this many stitches
/// (median), else they are a column (a wide satin with split stitches).
const ROW_STITCHES: f64 = 3.0;
/// Rows: at least this part of the legs come after a turn leg.
const TURNED: f64 = 0.5;
/// Rows: the legs are at this angle to their mean at most, degrees, but
/// for this part of their length.
const ROW_SPREAD_DEG: f64 = 12.0;
const ROW_OFF: f64 = 0.15;
/// Two legs that go back on the same line (a bean stitch) are at most this
/// far from each other, mm.
const SAME_LINE_MM: f64 = 0.12;
/// A bean stitch goes back into its needle holes: at least this part of
/// its legs (after the first two) end at most [`SAME_HOLE_MM`] from a
/// needle point of an earlier leg. A dense column (needle points 0.2 mm
/// apart on a side, or short stitches) has legs back on nearly the same
/// line too, but each needle point is a new one (P21 Milestone 13: the
/// arms of letters at 10 mm were runs).
const BEAN_HOLES: f64 = 0.3;
const SAME_HOLE_MM: f64 = 0.06;

/// The kind of a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// Stitches along a line.
    Run,
    /// Stitches from side to side (a satin column).
    Column,
    /// Long parallel lines of stitches (a fill).
    Rows,
    /// Stitches of another pattern.
    Other,
}

impl PartKind {
    pub const ALL: [PartKind; 4] = [PartKind::Run, PartKind::Column, PartKind::Rows, PartKind::Other];

    /// The name (English; translated where it shows).
    pub fn label(self) -> &'static str {
        match self {
            PartKind::Run => "Run",
            PartKind::Column => "Column",
            PartKind::Rows => "Rows",
            PartKind::Other => "Other",
        }
    }
}

/// A part of a stitch file.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub kind: PartKind,
    /// The stitch records of the part (each stitch ends at its record).
    pub records: Range<usize>,
    /// The thread (its number in the colors of the file).
    pub thread: usize,
    /// The part that this part is the underlay of (its number).
    pub underlay_of: Option<usize>,
}

/// The kind that the stitches of part `i` show: the kind of the part that
/// it is the underlay of.
pub fn shown_kind(parts: &[Part], i: usize) -> PartKind {
    let mut k = i;
    for _ in 0..parts.len() {
        match parts[k].underlay_of {
            Some(j) if j != k => k = j,
            _ => break,
        }
    }
    parts[k].kind
}

/// The kind that each record shows (`None` for a record that is not a
/// stitch of a part).
pub fn kinds_of_records(parts: &[Part], records: usize) -> Vec<Option<PartKind>> {
    let mut out = vec![None; records];
    for (i, p) in parts.iter().enumerate() {
        let k = shown_kind(parts, i);
        for r in p.records.clone() {
            if let Some(o) = out.get_mut(r) {
                *o = Some(k);
            }
        }
    }
    out
}

/// A leg: stitches `start..end` of a chain (stitch `k` goes from point `k`
/// to point `k + 1`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Leg {
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// From the first point to the last.
    pub(crate) a: Point,
    pub(crate) b: Point,
}

impl Leg {
    pub(crate) fn len(&self) -> f64 {
        self.a.distance(self.b)
    }

    pub(crate) fn dir(&self) -> (f64, f64) {
        let l = self.len().max(1e-9);
        ((self.b.x - self.a.x) / l, (self.b.y - self.a.y) / l)
    }

    pub(crate) fn stitches(&self) -> usize {
        self.end - self.start
    }

    fn mid(&self) -> Point {
        Point::new((self.a.x + self.b.x) / 2.0, (self.a.y + self.b.y) / 2.0)
    }
}

pub(crate) fn dot(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

/// The parts of the stitches of `s` (see the module text), in sew order.
pub fn parts(s: &StitchDesign) -> Vec<Part> {
    let mut out = Vec::new();
    for (thread, records) in chains(s) {
        // The points of the chain: where the needle is before its first
        // stitch, then the end of each stitch.
        let first = records.start;
        let before = if first > 0 {
            s.records[first - 1].pos
        } else {
            s.records[first].pos
        };
        let mut points = vec![before];
        points.extend(s.records[records.clone()].iter().map(|r| r.pos));
        for (kind, stitches) in chain_parts(&points) {
            out.push(Part {
                kind,
                records: first + stitches.start..first + stitches.end,
                thread,
                underlay_of: None,
            });
        }
    }
    find_underlay(s, &mut out);
    out
}

/// The chains of stitches of `s`: the thread and the stitch records.
fn chains(s: &StitchDesign) -> Vec<(usize, Range<usize>)> {
    let mut out = Vec::new();
    let mut thread = 0usize;
    let mut start: Option<usize> = None;
    let mut last: Option<Point> = None;
    for (i, r) in s.records.iter().enumerate() {
        let long = r.kind == StitchKind::Stitch && last.is_some_and(|p| p.distance(r.pos) > MAX_STITCH_MM);
        if r.kind != StitchKind::Stitch || long {
            if let Some(a) = start.take() {
                out.push((thread, a..i));
            }
            if r.kind == StitchKind::ColorChange {
                thread += 1;
            }
            if long {
                // The long stitch is the move to the start of a chain.
                start = None;
                last = Some(r.pos);
                continue;
            }
        } else if start.is_none() {
            start = Some(i);
        }
        last = Some(r.pos);
    }
    if let Some(a) = start {
        out.push((thread, a..s.records.len()));
    }
    out.retain(|(_, r)| !r.is_empty());
    out
}

/// The legs of the points of a chain (see the module text).
pub(crate) fn legs(points: &[Point]) -> Vec<Leg> {
    let n = points.len().saturating_sub(1);
    let mut out: Vec<Leg> = Vec::new();
    let turn = emb_geom::math::cos(emb_geom::math::to_radians(LEG_TURN_DEG));
    for k in 0..n {
        let (a, b) = (points[k], points[k + 1]);
        let len = a.distance(b);
        let joined = out.last_mut().is_some_and(|leg| {
            if len < TINY_MM || leg.len() < TINY_MM {
                return false;
            }
            let d = ((b.x - a.x) / len, (b.y - a.y) / len);
            dot(leg.dir(), d) >= turn
        });
        match (joined, out.last_mut()) {
            (true, Some(leg)) => {
                leg.end = k + 1;
                leg.b = b;
            }
            _ => out.push(Leg {
                start: k,
                end: k + 1,
                a,
                b,
            }),
        }
    }
    out
}

/// The parts of one chain: the kind and the stitches of each (stitch `k`
/// ends at point `k + 1`).
fn chain_parts(points: &[Point]) -> Vec<(PartKind, Range<usize>)> {
    let legs = legs(points);
    let n = legs.len();
    // The kind of each leg; `None`: not decided yet.
    let mut kind: Vec<Option<PartKind>> = vec![None; n];
    // Sequences of legs that go back and forth, with turn legs between.
    let mut i = 0;
    while i < n {
        let mut seq = vec![i];
        let mut j = i;
        // The legs back that come after a turn leg.
        let mut turned = 0usize;
        loop {
            // The next leg goes back, or a short turn leg and then one that
            // goes back.
            let goes_back = |a: usize, b: usize| {
                legs[a].len() >= TINY_MM && legs[b].len() >= TINY_MM && dot(legs[a].dir(), legs[b].dir()) < BACK
            };
            // A turn: one or two short legs between two rows (along a
            // slanted edge the turn is longer than the row spacing).
            let turn = |from: usize, to: usize| {
                let len: f64 = legs[from..to].iter().map(Leg::len).sum();
                len <= TURN_MM
                    .max(0.0)
                    .min(0.5 * legs[j].len().min(legs[to].len()))
                    .max(TURN_MIN_MM)
            };
            // The turn first: at the end of a row at an angle to the edge,
            // the turn leg itself can go back a little.
            if j + 2 < n && turn(j + 1, j + 2) && goes_back(j, j + 2) {
                j += 2;
                seq.push(j);
                turned += 1;
            } else if j + 3 < n && turn(j + 1, j + 3) && goes_back(j, j + 3) {
                j += 3;
                seq.push(j);
                turned += 1;
            } else if j + 1 < n && goes_back(j, j + 1) {
                j += 1;
                seq.push(j);
            } else {
                break;
            }
        }
        if seq.len() >= MIN_LEGS {
            let k = back_and_forth(&legs, &seq, turned);
            for leg in kind.iter_mut().take(j + 1).skip(i) {
                *leg = Some(k);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    // The other legs: runs, or other where a run is very short and between
    // parts of other kinds.
    for k in kind.iter_mut() {
        if k.is_none() {
            *k = Some(PartKind::Run);
        }
    }
    // Consecutive legs of one kind are one part.
    let mut out: Vec<(PartKind, Range<usize>)> = Vec::new();
    for (leg, k) in legs.iter().zip(kind) {
        let k = k.unwrap_or(PartKind::Other);
        match out.last_mut() {
            Some((last, r)) if *last == k => r.end = leg.end,
            _ => out.push((k, leg.start..leg.end)),
        }
    }
    merge_small(&mut out, points);
    out
}

/// The kind of a sequence of legs that go back and forth (`turned` of them
/// after a turn leg): rows, a column, or a run (a bean stitch: back and
/// forth on one line). Rows have a turn between most rows (a stitch along
/// the edge), a column goes back from the needle point where it came (a
/// wide satin can cross in a few stitches); rows with no turns have three
/// stitches or more each.
fn back_and_forth(legs: &[Leg], seq: &[usize], turned: usize) -> PartKind {
    // Back on the same line (a bean stitch): a leg ends where the leg
    // before it started, or the legs are not side by side.
    let mut across: Vec<f64> = seq
        .windows(2)
        .map(|w| {
            let (a, b) = (&legs[w[0]], &legs[w[1]]);
            let d = a.dir();
            let (mx, my) = (b.mid().x - a.mid().x, b.mid().y - a.mid().y);
            (mx * d.1 - my * d.0).abs().min(b.b.distance(a.a))
        })
        .collect();
    across.sort_by(f64::total_cmp);
    // The mean angle of the legs (as lines: a leg and the leg back have one
    // angle).
    let (mut c, mut s) = (0.0, 0.0);
    for &k in seq {
        let d = legs[k].dir();
        let a2 = 2.0 * emb_geom::math::atan2(d.1, d.0);
        c += emb_geom::math::cos(a2);
        s += emb_geom::math::sin(a2);
    }
    let mean = emb_geom::math::atan2(s, c) / 2.0;
    if across[across.len() / 2] < SAME_LINE_MM {
        // The legs that end in a needle hole of an earlier leg.
        let again = (2..seq.len())
            .filter(|&i| {
                let end = legs[seq[i]].b;
                seq[..i - 1].iter().any(|&k| {
                    let l = &legs[k];
                    l.a.distance(end) <= SAME_HOLE_MM || l.b.distance(end) <= SAME_HOLE_MM
                })
            })
            .count();
        if again as f64 >= BEAN_HOLES * seq.len().saturating_sub(2) as f64 {
            return PartKind::Run;
        }
    }
    let mut stitches: Vec<f64> = seq.iter().map(|&k| legs[k].stitches() as f64).collect();
    stitches.sort_by(f64::total_cmp);
    let per_leg = stitches[stitches.len() / 2];
    // The part of the length of the legs that is at an angle to the mean
    // (short rows at a curved edge can turn a little).
    let (mut off, mut all) = (0.0, 0.0);
    for &k in seq {
        let d = legs[k].dir();
        let a = emb_geom::math::atan2(d.1, d.0) - mean;
        // As a line: 0 to 90°.
        let a = emb_geom::math::to_degrees(a).rem_euclid(180.0);
        let len = legs[k].len();
        all += len;
        if a.min(180.0 - a) > ROW_SPREAD_DEG {
            off += len;
        }
    }
    let turns = turned as f64 >= TURNED * (seq.len() - 1) as f64;
    if (turns || per_leg >= ROW_STITCHES) && off <= ROW_OFF * all {
        PartKind::Rows
    } else {
        PartKind::Column
    }
}

/// A run of at most two stitches between two parts of one kind goes with
/// them (a stitch that ties two parts of a fill); a part of tiny stitches
/// only goes with the part before it (a lock stitch).
fn merge_small(parts: &mut Vec<(PartKind, Range<usize>)>, points: &[Point]) {
    let tiny = |r: &Range<usize>| r.clone().all(|k| points[k].distance(points[k + 1]) < TINY_MM);
    let mut i = 1;
    while i < parts.len() {
        let (k, r) = parts[i].clone();
        let short = r.len() <= 2 && k == PartKind::Run;
        let same_sides = i + 1 < parts.len() && parts[i - 1].0 == parts[i + 1].0 && parts[i - 1].0 != PartKind::Run;
        if short && same_sides {
            let end = parts[i + 1].1.end;
            parts[i - 1].1.end = end;
            parts.drain(i..=i + 1);
            continue;
        }
        if tiny(&r) {
            parts[i - 1].1.end = r.end;
            parts.remove(i);
            continue;
        }
        i += 1;
    }
    // Tiny stitches at the start go with the next part.
    if parts.len() > 1 && tiny(&parts[0].1) {
        let start = parts[0].1.start;
        parts.remove(0);
        parts[0].1.start = start;
    }
}

/// The cells of a grid of [`COVER_CELL_MM`] that the stitches of `records`
/// go through, each once, and the stitch length in them, mm.
fn cells_of(s: &StitchDesign, records: &Range<usize>) -> (Vec<(i64, i64)>, f64) {
    let cell = COVER_CELL_MM;
    let mut cells = std::collections::HashSet::new();
    let mut length = 0.0;
    for r in records.clone() {
        let b = s.records[r].pos;
        let a = if r > 0 { s.records[r - 1].pos } else { b };
        length += a.distance(b);
        let n = (a.distance(b) / (cell / 2.0)).ceil().max(1.0) as usize;
        for k in 0..=n {
            let t = k as f64 / n as f64;
            let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            cells.insert(((p.x / cell).floor() as i64, (p.y / cell).floor() as i64));
        }
    }
    let mut v: Vec<(i64, i64)> = cells.into_iter().collect();
    v.sort_unstable();
    (v, length)
}

/// The underlay (see the module text): from the last part to the first,
/// a map keeps for each cell the next column or rows of the same thread
/// that goes through it or next to it. A part goes with a later part if the
/// later parts together cover [`COVERED`] of its cells: a run (an underlay
/// run, or travel under them), or a column or rows that is lighter (less
/// thread in a cell: a zigzag or a fill underlay) than the part that covers
/// most of them.
fn find_underlay(s: &StitchDesign, parts: &mut [Part]) {
    let n = parts.len();
    let cells: Vec<(Vec<(i64, i64)>, f64)> = parts.iter().map(|p| cells_of(s, &p.records)).collect();
    // The thread in a cell of each part.
    let density: Vec<f64> = cells
        .iter()
        .map(|(c, len)| if c.is_empty() { 0.0 } else { len / c.len() as f64 })
        .collect();
    // The row spacing of each part that goes back and forth: rows sew each
    // cell once, so rows (or a zigzag) under rows are lighter by their
    // spacing, not by the thread in a cell.
    let spacing: Vec<Option<f64>> = parts
        .iter()
        .map(|p| {
            matches!(p.kind, PartKind::Rows | PartKind::Column)
                .then(|| crate::part_fills::part_shape(s, p))
                .flatten()
                .map(|r| r.spacing_mm)
        })
        .collect();
    let rows: Vec<bool> = parts.iter().map(|p| p.kind == PartKind::Rows).collect();
    let lighter = |i: usize, j: usize| match (spacing[i], spacing[j]) {
        (Some(a), Some(b)) if rows[j] => a * LIGHTER > b,
        _ => density[i] < LIGHTER * density[j],
    };
    let mut next: std::collections::HashMap<(i64, i64), usize> = std::collections::HashMap::new();
    for i in (0..n).rev() {
        if i + 1 < n && parts[i + 1].thread != parts[i].thread {
            next.clear();
        }
        let (own, _) = &cells[i];
        if !own.is_empty() {
            let mut by: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
            let mut covered = 0usize;
            for &(x, y) in own {
                // The cell or one next to it (a thread is about two cells
                // wide).
                let hit = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
                    .filter_map(|c| next.get(&c).copied())
                    .min();
                if let Some(j) = hit {
                    covered += 1;
                    *by.entry(j).or_default() += 1;
                }
            }
            if covered as f64 >= COVERED * own.len() as f64
                && let Some((&j, _)) = by.iter().max_by_key(|(j, c)| (**c, std::cmp::Reverse(**j)))
                && (parts[i].kind == PartKind::Run
                    || lighter(i, j)
                    || (rows[j] && parts[i].records.len() <= BETWEEN_STITCHES))
            {
                parts[i].underlay_of = Some(j);
            }
        }
        // Only a column or rows has an underlay.
        if matches!(parts[i].kind, PartKind::Column | PartKind::Rows) {
            for &c in own {
                next.insert(c, i);
            }
        }
    }
    // A run, a short part or a lighter part between two parts of the
    // underlay of one part (or the underlay and the part) goes with it too:
    // a piece of an underlay that its cells do not cover, or that is too
    // short to show its spacing.
    for i in 1..n.saturating_sub(1) {
        let thread = parts[i].thread;
        if parts[i].underlay_of.is_some() || parts[i - 1].thread != thread || parts[i + 1].thread != thread {
            continue;
        }
        if let Some(j) = parts[i - 1].underlay_of
            && j > i
            && (parts[i + 1].underlay_of == Some(j) || i + 1 == j)
            && (parts[i].kind == PartKind::Run || parts[i].records.len() <= BETWEEN_STITCHES || lighter(i, j))
        {
            parts[i].underlay_of = Some(j);
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::StitchRecord;

    use super::*;

    fn design(points: &[(f64, f64)]) -> StitchDesign {
        StitchDesign {
            threads: vec![Default::default()],
            records: points.iter().map(|&(x, y)| StitchRecord::stitch(x, y)).collect(),
        }
    }

    /// A satin column 4 mm wide along x, from 0 to `len` mm, 0.4 mm apart.
    fn satin(len: f64) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        let mut x = 0.0;
        let mut side = 0.0;
        while x <= len {
            out.push((x, side));
            side = if side == 0.0 { 4.0 } else { 0.0 };
            x += 0.2;
        }
        out
    }

    /// Tatami rows 20 mm long at 0°, 0.4 mm apart, stitches 3.5 mm.
    fn rows(n: usize) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for k in 0..n {
            let y = 10.0 + 0.4 * k as f64;
            let xs: Vec<f64> = (0..=6).map(|i| (i as f64 * 3.5).min(20.0)).collect();
            if k % 2 == 0 {
                out.extend(xs.iter().map(|&x| (x, y)));
            } else {
                out.extend(xs.iter().rev().map(|&x| (x, y)));
            }
        }
        out
    }

    #[test]
    fn a_satin_is_a_column_and_rows_are_rows() {
        let s = design(&satin(10.0));
        let p = parts(&s);
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].kind, PartKind::Column);
        let r = design(&rows(20));
        let p = parts(&r);
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].kind, PartKind::Rows);
    }

    #[test]
    fn rows_at_an_angle_with_turns_on_an_edge_are_rows() {
        // Rows at 45° between two upright edges (a turn of 0.57 mm on each).
        let mut pts = Vec::new();
        for k in 0..30 {
            let dy = -0.57 * k as f64;
            let (a, b) = ((-162.0, 110.95 + dy), (-180.0, 92.95 + dy));
            let (from, to) = if k % 2 == 0 { (a, b) } else { (b, a) };
            for i in 0..=7 {
                let t = (i as f64 * 3.5 / 25.46).min(1.0);
                pts.push((from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t));
            }
        }
        let p = parts(&design(&pts));
        assert!(p.iter().all(|p| p.kind == PartKind::Rows), "{p:?}");
    }

    /// The time of the parts of the stress design of 100,000 stitches. Run
    /// with `cargo test --release -p emb-art -- --ignored parts_time
    /// --nocapture`.
    #[test]
    #[ignore]
    fn parts_time() {
        let s = stress();
        let t0 = std::time::Instant::now();
        let p = parts(&s);
        println!("{} parts of {} records in {:?}", p.len(), s.records.len(), t0.elapsed());
    }

    /// A design of 100,000 stitches: rows of a fill, satin columns and runs.
    fn stress() -> StitchDesign {
        let mut pts = Vec::new();
        for block in 0..10 {
            let x0 = 60.0 * block as f64;
            pts.extend(rows(600).into_iter().map(|(x, y)| (x + x0, y)));
            pts.extend(satin(400.0).into_iter().map(|(x, y)| (x + x0, y + 300.0)));
        }
        design(&pts)
    }

    #[test]
    fn a_line_is_a_run_and_a_bean_stitch_too() {
        let line: Vec<(f64, f64)> = (0..30).map(|k| (2.5 * k as f64, 0.3 * k as f64)).collect();
        assert_eq!(parts(&design(&line))[0].kind, PartKind::Run);
        // Each stitch three times: forward, back, forward.
        let mut bean = vec![(0.0, 0.0)];
        for k in 1..20 {
            let (a, b) = (2.5 * (k - 1) as f64, 2.5 * k as f64);
            bean.extend([(b, 0.0), (a, 0.0), (b, 0.0)]);
        }
        let p = parts(&design(&bean));
        assert!(p.iter().all(|p| p.kind == PartKind::Run), "{p:?}");
        // A bean stitch along a curve (a circle of 15 mm): the middles of
        // its legs are not on one line, but it goes back into its holes.
        let at = |k: usize| {
            let a = 2.5 * k as f64 / 15.0;
            (15.0 * emb_geom::math::cos(a), 15.0 * emb_geom::math::sin(a))
        };
        let mut curve = vec![at(0)];
        for k in 1..20 {
            curve.extend([at(k), at(k - 1), at(k)]);
        }
        let p = parts(&design(&curve));
        assert!(p.iter().all(|p| p.kind == PartKind::Run), "{p:?}");
    }

    /// A narrow column of a small letter (1.6 mm wide, needle points 0.18 mm
    /// apart on a side) with short stitches (every second stitch on each
    /// side 0.4 mm short of the edge): two stitches back and forth are on
    /// nearly the same line, but the column goes to the side; it is a
    /// column, not a bean stitch (P21 Milestone 13).
    #[test]
    fn a_dense_column_with_short_stitches_is_a_column() {
        let mut pts = Vec::new();
        for j in 0..40 {
            let x = 0.09 * j as f64;
            let short = (j / 2) % 2 == 1;
            let inset = if short { 0.4 } else { 0.0 };
            let y = if j % 2 == 0 { inset } else { 1.6 - inset };
            pts.push((x, y));
        }
        let p = parts(&design(&pts));
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].kind, PartKind::Column, "{p:?}");
    }

    #[test]
    fn a_trim_cuts_and_underlay_goes_with_the_column() {
        // A run along the middle of the satin (its underlay), then the satin.
        let mut pts: Vec<(f64, f64)> = (0..=8).map(|k| (1.25 * k as f64, 2.0)).collect();
        pts.extend(satin(10.0));
        let mut s = design(&pts);
        // A trim and a run far away.
        s.records
            .push(StitchRecord::new(Point::new(30.0, 0.0), StitchKind::Trim));
        s.records
            .extend((0..10).map(|k| StitchRecord::stitch(30.0 + 2.0 * k as f64, 0.0)));
        let p = parts(&s);
        let kinds: Vec<PartKind> = (0..p.len()).map(|i| shown_kind(&p, i)).collect();
        assert_eq!(p[0].kind, PartKind::Run);
        assert_eq!(p[0].underlay_of, Some(1), "{p:?}");
        assert_eq!(kinds, vec![PartKind::Column, PartKind::Column, PartKind::Run], "{p:?}");
        // No stitch is lost: each stitch is in a part.
        let k = kinds_of_records(&p, s.records.len());
        let stitches = s.records.iter().filter(|r| r.kind == StitchKind::Stitch).count();
        assert_eq!(k.iter().filter(|k| k.is_some()).count(), stitches);
    }
}
