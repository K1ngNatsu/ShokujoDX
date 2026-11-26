//! The check and the fix of a machine file (P22 Milestone 5, ADR 0172):
//! the faults of the stitches of a file as it is, and fixes that change
//! only the needle points of the faults.
//!
//! Not a port. The records are read as a machine sews them. A **piece**
//! is the needle points between two cuts of the thread (a trim, a color
//! change, the start and the end); a jump with no trim does not cut. The
//! checks:
//!
//! - a stitch shorter than the shortest stitch of the machine (a stitch
//!   in the same needle hole is not counted), or longer than its longest
//!   (both with [`GRID_MARGIN_MM`] for the 0.1 mm grid of the files);
//! - two color blocks of the same thread (color, brand and code), one
//!   after the other;
//! - a jump longer than the trim distance with no trim;
//! - a piece with fewer than [`MIN_PIECE_HOLES`] needle holes: too short
//!   for the trimmer (the stitches pull out);
//! - the start or the end of a piece with no lock stitches in its
//!   [`LOCK_POINTS`] first or last needle points: a needle point back in
//!   the hole of the needle point two before it, or three tiny stitches;
//! - travel that shows: the runs between two other parts of a chain (the
//!   parts of `crate::parts`) that go almost straight from one to the
//!   other (at most [`MAX_TRAVEL_MM`] long, and at most [`DIRECT`] times
//!   the distance between their ends: the curved rows of a spiral fill are
//!   runs too) are the travel of the file, measured with the measure of
//!   Milestone 1 (`emb_stitch::shown_travel`).
//!
//! The fixes, in this order: join the color blocks of the same thread
//! (with a trim where the move to the next block is longer than the trim
//! distance), remove the short stitches (the next stitch starts at the
//! needle point before), split the long stitches into equal stitches,
//! trim the long jumps, and add lock stitches (forward and back twice, as
//! the stitch plan makes them) at the starts and ends with no lock. The
//! other needle points stay where they are.

use std::ops::Range;

use emb_geom::Point;
use emb_model::{StitchDesign, StitchKind, StitchRecord, Thread};
use emb_stitch::PlanOptions;

use crate::parts::{self, PartKind};

/// Two needle points this near are in one needle hole, mm (the unit of
/// most machine files is 0.1 mm).
pub const HOLE_MM: f64 = 0.15;
/// The lock stitches of a start or an end are in its first or last this
/// many needle points.
pub const LOCK_POINTS: usize = 7;
/// Three stitches at most this long at a start or an end lock the thread
/// (tiny stitches), mm.
pub const TINY_LOCK_MM: f64 = 0.7;
/// A piece between two cuts with fewer needle holes than this is too
/// short for the trimmer.
pub const MIN_PIECE_HOLES: usize = 3;
/// A travel run with at least this much on top of sewn stitches is a
/// fault, mm.
pub const SHOWS_MM: f64 = 1.0;
/// A travel run is at most this long, mm.
pub const MAX_TRAVEL_MM: f64 = 60.0;
/// A travel run is at most this many times as long as the distance
/// between its ends (and 2 mm).
pub const DIRECT: f64 = 1.6;

/// A kind of fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fault {
    Short,
    Long,
    SameThread,
    LongJump,
    ShortPiece,
    NoLockStart,
    NoLockEnd,
    TravelShows,
}

impl Fault {
    pub const ALL: [Fault; 8] = [
        Fault::Short,
        Fault::Long,
        Fault::SameThread,
        Fault::LongJump,
        Fault::ShortPiece,
        Fault::NoLockStart,
        Fault::NoLockEnd,
        Fault::TravelShows,
    ];

    /// The name for the command line (English; the program has its own
    /// texts).
    pub fn cli_text(self) -> &'static str {
        match self {
            Fault::Short => "Stitches shorter than the shortest stitch of the machine",
            Fault::Long => "Stitches longer than the longest stitch of the machine",
            Fault::SameThread => "Color changes to the same thread",
            Fault::LongJump => "Long jumps with no trim",
            Fault::ShortPiece => "Pieces too short for the trimmer",
            Fault::NoLockStart => "Starts with no lock stitches",
            Fault::NoLockEnd => "Ends with no lock stitches",
            Fault::TravelShows => "Travel that shows on top of sewn stitches",
        }
    }

    /// The name for the command line.
    pub fn key(self) -> &'static str {
        match self {
            Fault::Short => "short",
            Fault::Long => "long",
            Fault::SameThread => "same-thread",
            Fault::LongJump => "long-jump",
            Fault::ShortPiece => "short-piece",
            Fault::NoLockStart => "no-lock-start",
            Fault::NoLockEnd => "no-lock-end",
            Fault::TravelShows => "travel-shows",
        }
    }
}

/// One fault and its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub fault: Fault,
    /// The record of the place: the stitch, the color change, the first
    /// jump, the first or last needle point of the piece, the first
    /// record of the travel run.
    pub record: usize,
    pub at: Point,
    /// The length of the stitch, the jump or the travel that shows, mm (0
    /// for the other faults).
    pub length_mm: f64,
}

/// The faults of a file, in the order of the records for each kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileCheck {
    pub found: Vec<Found>,
}

impl FileCheck {
    pub fn count(&self, fault: Fault) -> usize {
        self.found.iter().filter(|f| f.fault == fault).count()
    }

    pub fn is_empty(&self) -> bool {
        self.found.is_empty()
    }
}

/// The two threads are the same thread: color, brand and code.
fn same_thread(a: &Thread, b: &Thread) -> bool {
    a.color == b.color && a.brand == b.brand && a.code == b.code
}

/// The stitches of `recs`: the record of the needle point before and the
/// record of the stitch. A jump, a trim, a color change, a stop or the end
/// is not a stitch; a speed change is no move.
fn stitches(recs: &[StitchRecord]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut before: Option<usize> = None;
    for (i, r) in recs.iter().enumerate() {
        match r.kind {
            StitchKind::Stitch => {
                if let Some(b) = before {
                    out.push((b, i));
                }
                before = Some(i);
            }
            StitchKind::Slow | StitchKind::Fast => {}
            _ => before = None,
        }
    }
    out
}

/// The pieces of `recs`: the records of the needle points between two
/// cuts (a trim, a color change, the end). Pieces with no needle point are
/// left out.
fn pieces(recs: &[StitchRecord]) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut piece = Vec::new();
    for (i, r) in recs.iter().enumerate() {
        match r.kind {
            StitchKind::Stitch => piece.push(i),
            StitchKind::Trim | StitchKind::ColorChange | StitchKind::End if !piece.is_empty() => {
                out.push(std::mem::take(&mut piece));
            }
            _ => {}
        }
    }
    if !piece.is_empty() {
        out.push(piece);
    }
    out
}

/// The needle points `points` have [`MIN_PIECE_HOLES`] needle holes or
/// more: a piece that sews.
fn enough_holes(points: &[Point]) -> bool {
    let mut seen: Vec<Point> = Vec::with_capacity(MIN_PIECE_HOLES);
    for &p in points {
        if !seen.iter().any(|q| q.distance(p) <= HOLE_MM) {
            seen.push(p);
            if seen.len() >= MIN_PIECE_HOLES {
                return true;
            }
        }
    }
    false
}

/// The first needle points of a start (or the last ones of an end, in
/// reverse) lock the thread.
fn locks(points: &[Point]) -> bool {
    let n = points.len().min(LOCK_POINTS);
    let p = &points[..n];
    // Back into a needle hole, with a stitch between.
    let back =
        (0..n.saturating_sub(2)).any(|i| p[i + 2].distance(p[i]) <= HOLE_MM && p[i + 1].distance(p[i]) > HOLE_MM);
    // Three tiny stitches.
    let tiny = n >= 4 && p.windows(2).take(3).all(|w| w[0].distance(w[1]) <= TINY_LOCK_MM);
    back || tiny
}

/// The color changes from a block to a block of the same thread: the
/// record of each color change.
fn same_thread_changes(s: &StitchDesign) -> Vec<usize> {
    let mut out = Vec::new();
    let mut block = 0;
    for (i, r) in s.records.iter().enumerate() {
        if r.kind == StitchKind::ColorChange {
            if let (Some(a), Some(b)) = (s.threads.get(block), s.threads.get(block + 1))
                && same_thread(a, b)
            {
                out.push(i);
            }
            block += 1;
        }
    }
    out
}

/// The jumps longer than `trim_mm` with no trim: the record of the needle
/// point before, the record of the first jump, and the length.
fn long_jumps(recs: &[StitchRecord], trim_mm: f64) -> Vec<(usize, usize, f64)> {
    let mut out = Vec::new();
    // The needle point before the move, the first jump, and a cut.
    let mut from: Option<usize> = None;
    let mut first_jump: Option<usize> = None;
    let mut cut = false;
    for (i, r) in recs.iter().enumerate() {
        match r.kind {
            StitchKind::Stitch => {
                if let (Some(f), Some(j)) = (from, first_jump)
                    && !cut
                {
                    let d = recs[f].pos.distance(r.pos);
                    if d > trim_mm + 1e-9 {
                        out.push((f, j, d));
                    }
                }
                from = Some(i);
                first_jump = None;
                cut = false;
            }
            StitchKind::Jump => {
                first_jump.get_or_insert(i);
            }
            StitchKind::Trim | StitchKind::ColorChange | StitchKind::End => cut = true,
            StitchKind::Stop | StitchKind::Slow | StitchKind::Fast => {}
        }
    }
    out
}

/// The travel runs of the file: the run parts between two other parts of
/// one chain (no jump, trim, stop or color change between them). Each is a
/// range of records (each stitch ends at its record).
fn travel_runs(s: &StitchDesign) -> Vec<Range<usize>> {
    let ps = parts::parts(s);
    let recs = &s.records;
    // Only stitches (and speed changes) between two records.
    let joined = |a: usize, b: usize| {
        (a..b).all(|i| matches!(recs[i].kind, StitchKind::Stitch | StitchKind::Slow | StitchKind::Fast))
    };
    let mut out = Vec::new();
    for w in ps.windows(3) {
        let (before, run, after) = (&w[0], &w[1], &w[2]);
        if run.kind != PartKind::Run || before.kind == PartKind::Run || after.kind == PartKind::Run {
            continue;
        }
        if run.records.is_empty() || before.thread != run.thread || after.thread != run.thread {
            continue;
        }
        if !joined(before.records.end, run.records.start) || !joined(run.records.end, after.records.start) {
            continue;
        }
        // The run from the needle point before its first stitch.
        let from = recs[run.records.start.saturating_sub(1)].pos;
        let to = recs[run.records.end - 1].pos;
        let length: f64 = run
            .records
            .clone()
            .filter(|&i| i > 0)
            .map(|i| recs[i - 1].pos.distance(recs[i].pos))
            .sum();
        if length <= MAX_TRAVEL_MM && length <= DIRECT * from.distance(to) + 2.0 {
            out.push(run.records.clone());
        }
    }
    out
}

/// The travel runs that show on top of sewn stitches, with the length that
/// shows.
fn travel_that_shows(s: &StitchDesign) -> Vec<(Range<usize>, f64)> {
    use emb_stitch::shown_travel::{Shows, shown_travel};
    let runs = travel_runs(s);
    if runs.is_empty() {
        return Vec::new();
    }
    let plan = emb_stitch::StitchPlan {
        stitches: s.clone(),
        warnings: Vec::new(),
        spans: Vec::new(),
        stops: Vec::new(),
        travel: runs.clone(),
    };
    let shown = shown_travel(&plan);
    // The run of each piece: the run of the record where it starts (the
    // runs are in the order of the records). A search of all the runs for
    // each piece took seconds for a file with many runs (the audit of P22).
    let mut length = vec![0.0; runs.len()];
    for piece in shown.pieces.iter().filter(|p| p.shows == Shows::OnStitches) {
        let j = runs.partition_point(|r| r.start <= piece.record);
        if let Some(j) = j.checked_sub(1)
            && runs[j].contains(&piece.record)
        {
            length[j] += piece.length();
        }
    }
    runs.into_iter().zip(length).filter(|(_, l)| *l >= SHOWS_MM).collect()
}

/// Checks the records of a machine file, with the stitch lengths and the
/// trim distance of `opts`.
pub fn check(s: &StitchDesign, opts: &PlanOptions) -> FileCheck {
    let recs = &s.records;
    let mut found = Vec::new();
    let mut add = |fault, record: usize, length_mm| {
        found.push(Found {
            fault,
            record,
            at: recs[record].pos,
            length_mm,
        });
    };
    let lengths: Vec<(usize, f64)> = stitches(recs)
        .into_iter()
        .map(|(a, b)| (b, recs[a].pos.distance(recs[b].pos)))
        .collect();
    for &(i, len) in &lengths {
        if is_short(len, opts) {
            add(Fault::Short, i, len);
        }
    }
    for &(i, len) in &lengths {
        if is_long(len, opts) {
            add(Fault::Long, i, len);
        }
    }
    for i in same_thread_changes(s) {
        add(Fault::SameThread, i, 0.0);
    }
    for (from, _, len) in long_jumps(recs, opts.trim_mm) {
        add(Fault::LongJump, from, len);
    }
    let all = pieces(recs);
    for piece in &all {
        let points: Vec<Point> = piece.iter().map(|&i| recs[i].pos).collect();
        if !enough_holes(&points) {
            add(Fault::ShortPiece, piece[0], 0.0);
        }
    }
    for piece in &all {
        let points: Vec<Point> = piece.iter().map(|&i| recs[i].pos).collect();
        if enough_holes(&points) && !locks(&points) {
            add(Fault::NoLockStart, piece[0], 0.0);
        }
    }
    for piece in &all {
        let mut points: Vec<Point> = piece.iter().map(|&i| recs[i].pos).collect();
        points.reverse();
        if enough_holes(&points) && !locks(&points) {
            add(Fault::NoLockEnd, piece[piece.len() - 1], 0.0);
        }
    }
    for (run, len) in travel_that_shows(s) {
        add(Fault::TravelShows, run.start, len);
    }
    FileCheck { found }
}

/// A stitch shorter than the shortest stitch, with a margin for the
/// 0.1 mm grid of the files (a stitch of 0.3 mm on a slant is 0.28 mm on
/// the grid). A stitch in the same needle hole is not counted.
fn is_short(len: f64, opts: &PlanOptions) -> bool {
    len > 1e-6 && len < opts.min_stitch_mm - GRID_MARGIN_MM
}

/// A stitch longer than the longest stitch, with a margin for the 0.1 mm
/// grid of the files (the two ends of a stitch of the longest length).
fn is_long(len: f64, opts: &PlanOptions) -> bool {
    len > opts.max_stitch_mm + GRID_MARGIN_MM
}

/// The margin of [`is_short`] and [`is_long`] for the grid of the files,
/// mm.
pub const GRID_MARGIN_MM: f64 = 0.05;

/// The fixes to make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixes {
    pub join: bool,
    pub short: bool,
    pub long: bool,
    pub jumps: bool,
    pub locks: bool,
}

impl Fixes {
    pub const ALL: Fixes = Fixes {
        join: true,
        short: true,
        long: true,
        jumps: true,
        locks: true,
    };
    pub const NONE: Fixes = Fixes {
        join: false,
        short: false,
        long: false,
        jumps: false,
        locks: false,
    };
}

/// What a fix did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fixed {
    /// Color changes to the same thread that are gone.
    pub joined: usize,
    /// Short stitches that are gone.
    pub removed: usize,
    /// Long stitches that are split.
    pub split: usize,
    /// Long jumps that have a trim now.
    pub trimmed: usize,
    /// Starts and ends that have lock stitches now.
    pub locked: usize,
}

impl Fixed {
    pub fn total(&self) -> usize {
        self.joined + self.removed + self.split + self.trimmed + self.locked
    }
}

/// Fixes the faults `which` of the records of a machine file (see the
/// module notes). The needle points that no fix touches stay where they
/// are.
pub fn fix(s: &StitchDesign, opts: &PlanOptions, which: Fixes) -> (StitchDesign, Fixed) {
    let mut out = s.clone();
    let mut fixed = Fixed::default();
    if which.join {
        fixed.joined = join(&mut out, opts);
    }
    if which.short {
        fixed.removed = remove_short(&mut out.records, opts);
    }
    if which.long {
        fixed.split = split_long(&mut out.records, opts);
    }
    if which.jumps {
        fixed.trimmed = trim_jumps(&mut out.records, opts);
    }
    if which.locks {
        fixed.locked = add_locks(&mut out.records, opts);
    }
    (out, fixed)
}

/// Joins the color blocks of the same thread: the color change goes, and
/// the thread of the second block. Where the move to the next block is
/// longer than the trim distance, a trim stays there (if there is none).
fn join(s: &mut StitchDesign, opts: &PlanOptions) -> usize {
    let changes = same_thread_changes(s);
    if changes.is_empty() {
        return 0;
    }
    let recs = &s.records;
    let mut records = Vec::with_capacity(recs.len());
    let mut threads = Vec::with_capacity(s.threads.len());
    let mut block = 0;
    if let Some(t) = s.threads.first() {
        threads.push(t.clone());
    }
    let mut needle: Option<Point> = None;
    for (i, r) in recs.iter().enumerate() {
        if r.kind == StitchKind::ColorChange {
            block += 1;
            if changes.contains(&i) {
                let next = recs[i..].iter().find(|x| x.kind == StitchKind::Stitch).map(|x| x.pos);
                let trimmed = records
                    .iter()
                    .rev()
                    .take_while(|x: &&StitchRecord| x.kind != StitchKind::Stitch)
                    .any(|x| x.kind == StitchKind::Trim);
                if let (Some(a), Some(b)) = (needle, next)
                    && a.distance(b) > opts.trim_mm
                    && !trimmed
                {
                    records.push(StitchRecord::new(a, StitchKind::Trim));
                }
                continue;
            }
            if let Some(t) = s.threads.get(block) {
                threads.push(t.clone());
            }
        }
        if r.kind == StitchKind::Stitch {
            needle = Some(r.pos);
        }
        records.push(*r);
    }
    // Threads of blocks past the last color change stay.
    threads.extend(s.threads.iter().skip(block + 1).cloned());
    s.records = records;
    s.threads = threads;
    changes.len()
}

/// Removes the short stitches: the next stitch starts at the needle point
/// before.
fn remove_short(recs: &mut Vec<StitchRecord>, opts: &PlanOptions) -> usize {
    let mut out = Vec::with_capacity(recs.len());
    let mut before: Option<Point> = None;
    let mut removed = 0;
    for r in recs.iter() {
        match r.kind {
            StitchKind::Stitch => {
                if let Some(b) = before
                    && is_short(b.distance(r.pos), opts)
                {
                    removed += 1;
                    continue;
                }
                before = Some(r.pos);
            }
            StitchKind::Slow | StitchKind::Fast => {}
            _ => before = None,
        }
        out.push(*r);
    }
    *recs = out;
    removed
}

/// A long stitch up to this length is split. A longer one is a jump that
/// the file keeps as a stitch (a damaged file): the fix leaves it, and the
/// check shows it (the audit of P22: a file of 1,000 stitches of 1 km
/// became 78 million stitches).
pub const MAX_SPLIT_MM: f64 = 60.0;

/// Splits the long stitches into equal stitches (up to [`MAX_SPLIT_MM`]).
fn split_long(recs: &mut Vec<StitchRecord>, opts: &PlanOptions) -> usize {
    let mut out = Vec::with_capacity(recs.len());
    let mut before: Option<Point> = None;
    let mut split = 0;
    let max = opts.max_stitch_mm.max(0.1);
    for r in recs.iter() {
        match r.kind {
            StitchKind::Stitch => {
                if let Some(b) = before {
                    let len = b.distance(r.pos);
                    if is_long(len, opts) && len <= MAX_SPLIT_MM {
                        let n = (len / max).ceil() as usize;
                        for k in 1..n {
                            out.push(StitchRecord::new(
                                b.lerp(r.pos, k as f64 / n as f64),
                                StitchKind::Stitch,
                            ));
                        }
                        split += 1;
                    }
                }
                before = Some(r.pos);
            }
            StitchKind::Slow | StitchKind::Fast => {}
            _ => before = None,
        }
        out.push(*r);
    }
    *recs = out;
    split
}

/// Puts a trim before each long jump with no trim, at the needle point
/// before the jump.
fn trim_jumps(recs: &mut Vec<StitchRecord>, opts: &PlanOptions) -> usize {
    let jumps = long_jumps(recs, opts.trim_mm);
    for &(from, first_jump, _) in jumps.iter().rev() {
        recs.insert(first_jump, StitchRecord::new(recs[from].pos, StitchKind::Trim));
    }
    jumps.len()
}

/// Adds lock stitches at the starts and ends of the pieces with no lock.
fn add_locks(recs: &mut Vec<StitchRecord>, opts: &PlanOptions) -> usize {
    let far = 2.0 * emb_stitch::min_lock_mm(opts);
    // (the record after which the lock goes, its points), in order.
    let mut inserts: Vec<(usize, [Point; 4])> = Vec::new();
    for piece in pieces(recs) {
        let points: Vec<Point> = piece.iter().map(|&i| recs[i].pos).collect();
        if !enough_holes(&points) {
            continue;
        }
        if !locks(&points) {
            let at = points[0];
            let toward = points[1..]
                .iter()
                .find(|p| p.distance(at) >= far)
                .or(points.get(1))
                .copied();
            if let Some(l) = toward.and_then(|t| emb_stitch::lock_points(at, t, opts)) {
                inserts.push((piece[0], l));
            }
        }
        let mut back = points.clone();
        back.reverse();
        if !locks(&back) {
            let at = back[0];
            let toward = back[1..]
                .iter()
                .take(emb_stitch::LOCK_LOOK_BACK)
                .find(|p| p.distance(at) >= far)
                .or(back.get(1))
                .copied();
            if let Some(l) = toward.and_then(|t| emb_stitch::lock_points(at, t, opts)) {
                inserts.push((piece[piece.len() - 1], l));
            }
        }
    }
    let n = inserts.len();
    for (after, points) in inserts.into_iter().rev() {
        let new = points.map(|p| StitchRecord::new(p, StitchKind::Stitch));
        recs.splice(after + 1..after + 1, new);
    }
    n
}

#[cfg(test)]
mod tests {
    use emb_model::Rgb;

    use super::*;

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    fn st(x: f64, y: f64) -> StitchRecord {
        rec(x, y, StitchKind::Stitch)
    }

    /// A run along y with lock stitches at both ends, from x0 to x1 at 2 mm
    /// steps.
    fn locked_run(x0: f64, x1: f64, y: f64) -> Vec<StitchRecord> {
        let mut v = vec![rec(x0, y, StitchKind::Jump), st(x0, y)];
        let dir = if x1 > x0 { 1.0 } else { -1.0 };
        for p in [x0 + 0.5 * dir, x0, x0 + 0.5 * dir, x0] {
            v.push(st(p, y));
        }
        let mut x = x0;
        while (x1 - x) * dir > 2.0 {
            x += 2.0 * dir;
            v.push(st(x, y));
        }
        v.push(st(x1, y));
        for p in [x1 - 0.5 * dir, x1, x1 - 0.5 * dir, x1] {
            v.push(st(p, y));
        }
        v
    }

    fn thread(r: u8) -> Thread {
        Thread {
            color: Rgb(r, 0, 0),
            ..Default::default()
        }
    }

    fn opts() -> PlanOptions {
        PlanOptions {
            max_stitch_mm: 12.0,
            min_stitch_mm: 0.3,
            trim_mm: 2.0,
            lock_mm: 0.5,
        }
    }

    /// A file with each fault once (but travel that shows).
    fn faulty() -> StitchDesign {
        let mut r = Vec::new();
        // A run with a short stitch (4 to 4.1 mm) and lock stitches.
        r.extend(locked_run(0.0, 10.0, 0.0));
        let at = r
            .iter()
            .position(|x| x.pos == Point::new(6.0, 0.0))
            .expect("a needle point at 6 mm");
        r.insert(at, st(4.1, 0.0));
        // A long stitch (20 mm), then lock stitches.
        r.push(st(10.0, 20.0));
        r.push(st(10.0, 22.0));
        for y in [21.5, 22.0, 21.5, 22.0] {
            r.push(st(10.0, y));
        }
        // A long jump with no trim to a run with lock stitches.
        r.extend(locked_run(30.0, 40.0, 0.0));
        r.push(rec(40.0, 0.0, StitchKind::Trim));
        // A piece with no lock stitches.
        r.push(rec(45.0, 10.0, StitchKind::Jump));
        for k in 0..5 {
            r.push(st(45.0 + 2.0 * f64::from(k), 10.0));
        }
        r.push(rec(53.0, 10.0, StitchKind::Trim));
        // A color change to the same thread (red again).
        r.push(rec(53.0, 10.0, StitchKind::ColorChange));
        r.extend(locked_run(40.0, 50.0, 5.0));
        r.push(rec(50.0, 5.0, StitchKind::Trim));
        // A piece of two needle points.
        r.push(rec(60.0, 5.0, StitchKind::Jump));
        r.push(st(60.0, 5.0));
        r.push(st(61.0, 5.0));
        r.push(rec(61.0, 5.0, StitchKind::Trim));
        r.push(rec(61.0, 5.0, StitchKind::End));
        StitchDesign {
            threads: vec![thread(200), thread(200)],
            records: r,
        }
    }

    #[test]
    fn a_file_with_each_fault_has_each_fault() {
        let c = check(&faulty(), &opts());
        let counts: Vec<(Fault, usize)> = Fault::ALL.iter().map(|&f| (f, c.count(f))).collect();
        assert_eq!(
            counts,
            [
                (Fault::Short, 1),
                (Fault::Long, 1),
                (Fault::SameThread, 1),
                (Fault::LongJump, 1),
                (Fault::ShortPiece, 1),
                (Fault::NoLockStart, 1),
                (Fault::NoLockEnd, 1),
                (Fault::TravelShows, 0),
            ],
            "{:#?}",
            c.found
        );
    }

    /// The exit test of Milestone 5: each fault is fixed with the right
    /// count, and the needle points that no fix touches keep their place.
    #[test]
    fn each_fault_is_fixed_and_the_other_needle_points_stay() {
        let s = faulty();
        let (out, fixed) = fix(&s, &opts(), Fixes::ALL);
        assert_eq!(
            fixed,
            Fixed {
                joined: 1,
                removed: 1,
                split: 1,
                trimmed: 1,
                locked: 2,
            }
        );
        let after = check(&out, &opts());
        for f in [
            Fault::Short,
            Fault::Long,
            Fault::SameThread,
            Fault::LongJump,
            Fault::NoLockStart,
            Fault::NoLockEnd,
        ] {
            assert_eq!(after.count(f), 0, "{f:?}: {:#?}", after.found);
        }
        // The short piece has no fix.
        assert_eq!(after.count(Fault::ShortPiece), 1);
        assert_eq!(out.threads.len(), 1);
        // The needle points of before are all there, in their order, but the
        // short stitch that went.
        let points = |s: &StitchDesign| -> Vec<Point> {
            s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect()
        };
        let before: Vec<Point> = points(&s).into_iter().filter(|p| *p != Point::new(4.1, 0.0)).collect();
        let now = points(&out);
        let mut k = 0;
        for p in &now {
            if k < before.len() && *p == before[k] {
                k += 1;
            }
        }
        assert_eq!(k, before.len(), "a needle point moved or went");
        // The new needle points: one to split the long stitch, eight for
        // the locks.
        assert_eq!(now.len(), before.len() + 1 + 8);
    }

    #[test]
    fn each_fix_alone_does_only_its_fix() {
        let s = faulty();
        let one = |which: Fixes| fix(&s, &opts(), which).1;
        let none = Fixes::NONE;
        assert_eq!(one(none), Fixed::default());
        assert_eq!(one(Fixes { join: true, ..none }).total(), 1);
        assert_eq!(one(Fixes { short: true, ..none }).removed, 1);
        assert_eq!(one(Fixes { long: true, ..none }).split, 1);
        assert_eq!(one(Fixes { jumps: true, ..none }).trimmed, 1);
        assert_eq!(one(Fixes { locks: true, ..none }).locked, 2);
        // The trim of the long jump: its two ends have lock stitches.
        let (out, _) = fix(&s, &opts(), Fixes { jumps: true, ..none });
        let c = check(&out, &opts());
        assert_eq!((c.count(Fault::NoLockStart), c.count(Fault::NoLockEnd)), (1, 1));
    }

    /// A run that goes from one fill to the next across the first fill
    /// shows.
    #[test]
    fn travel_across_a_fill_shows() {
        let mut r = vec![rec(0.0, 0.0, StitchKind::Jump)];
        // Rows of a fill: 10 mm long, 0.4 mm apart, with 3 stitches each.
        let rows = |r: &mut Vec<StitchRecord>, x0: f64| {
            for k in 0..25 {
                let y = 0.4 * f64::from(k);
                let (a, b) = if k % 2 == 0 { (x0, x0 + 10.0) } else { (x0 + 10.0, x0) };
                for j in 0..=3 {
                    r.push(st(a + (b - a) * f64::from(j) / 3.0, y));
                }
            }
        };
        rows(&mut r, 0.0);
        // Travel across the fill to (7, 0.5), steep to the rows, then on the
        // fabric to the next fill.
        let end = r[r.len() - 1].pos;
        let mid = Point::new(7.0, 0.5);
        for k in 1..=5 {
            let p = end.lerp(mid, f64::from(k) / 5.0);
            r.push(st(p.x, p.y));
        }
        let next = Point::new(20.0, 0.0);
        for k in 1..=7 {
            let p = mid.lerp(next, f64::from(k) / 7.0);
            r.push(st(p.x, p.y));
        }
        rows(&mut r, 20.0);
        r.push(rec(30.0, 9.6, StitchKind::End));
        let s = StitchDesign {
            threads: vec![thread(10)],
            records: r,
        };
        let c = check(&s, &opts());
        let shows: Vec<&Found> = c.found.iter().filter(|f| f.fault == Fault::TravelShows).collect();
        assert_eq!(shows.len(), 1, "{:#?}", c.found);
        // The part over the fill (9.6 mm) less its ends and the edge of the
        // fill.
        assert!(shows[0].length_mm > 2.0 && shows[0].length_mm < 9.6, "{:?}", shows[0]);
    }

    /// The color changes of two blocks of different threads stay.
    /// A stitch of 100 mm (a jump that a damaged file keeps as a stitch)
    /// stays one stitch, and the check still shows it; a stitch of 30 mm is
    /// split.
    #[test]
    fn a_stitch_longer_than_the_split_limit_stays() {
        let file = |len: f64| StitchDesign {
            threads: vec![thread(200)],
            records: [
                locked_run(0.0, 10.0, 0.0),
                vec![st(10.0 + len, 0.0)],
                locked_run(10.0 + len, 20.0 + len, 0.0),
            ]
            .concat(),
        };
        let long = Fixes {
            long: true,
            ..Fixes::NONE
        };
        let (fixed, did) = fix(&file(100.0), &opts(), long);
        assert_eq!(did.split, 0);
        assert_eq!(fixed, file(100.0));
        assert_eq!(check(&fixed, &opts()).count(Fault::Long), 1);
        let (fixed, did) = fix(&file(30.0), &opts(), long);
        assert_eq!(did.split, 1);
        assert_eq!(check(&fixed, &opts()).count(Fault::Long), 0);
    }

    #[test]
    fn different_threads_are_not_joined() {
        let mut s = faulty();
        s.threads[1] = thread(20);
        assert_eq!(check(&s, &opts()).count(Fault::SameThread), 0);
        let (out, fixed) = fix(&s, &opts(), Fixes::ALL);
        assert_eq!(fixed.joined, 0);
        assert_eq!(out.threads.len(), 2);
    }
}
