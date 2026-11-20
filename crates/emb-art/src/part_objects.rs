//! Objects from the parts of a stitch file (P18 Milestone 2, ADR 0132): a
//! run becomes a running object and a column a satin column, with the
//! settings that its stitches show. The rows of a fill become a fill
//! object (`crate::part_fills`, P18 Milestone 3, ADR 0133). The other
//! parts stay stitches.
//!
//! Not a port. The method:
//!
//! - The lock stitches at the ends of a part (short stitches back and forth
//!   that come back to their first point) are not a part of the line.
//! - **Run**: when most stitches go back on themselves and on again (each
//!   stitch three times), the run is a bean stitch. The line is the needle
//!   points, with fewer points where they are on a straight line
//!   ([`LINE_TOL_MM`]); the stitch length is the length of most stitches,
//!   to 0.1 mm.
//! - **Column**: the stitches across go in passes from one end of the
//!   column to the other (an underlay zigzag goes out and back, then the
//!   top stitches go out). The densest passes are top stitches; a lighter
//!   pass before one is its underlay. In a pass, the needle points are on
//!   the two sides in turn: they make the two rails, made smooth. The end of
//!   a rail where no stitch starts is half a step out. Rungs keep the stitch
//!   angle where the pairs of the rails do not give it ([`RUNG_TOL_MM`]).
//!   The spacing is two times the length of the column divided by the
//!   number of stitches across. The pull compensation is 0, because the
//!   needle points are on the sides already. A needle point of a short
//!   stitch (in from its side, on the inside of a curve) is not on a rail.
//! - **Underlay**: a lighter pass is a zigzag layer (its inset and spacing;
//!   a pass out and a pass back are one layer). An underlay run along the
//!   middle of the column is a center walk, along the two sides an edge
//!   walk; another run under the column (travel) is a running object of
//!   its own. When the layers are the layers of an underlay setting (Auto,
//!   Light, Heavy, Double) at the width of the column, the object gets that
//!   setting.
//!
//! A part that does not become an object stays stitches: no stitch is lost.

use std::ops::Range;

use emb_geom::{Path, Point};
use emb_model::{
    ColumnPattern, ObjectKind, RunningParams, SatinGeometry, SatinParams, StitchDesign, StitchKind, Underlay,
    UnderlayLayer,
};

use crate::part_fills::{FillGroup, can_be_travel, fill_items, part_members};
use crate::parts::{self, BACK, Leg, Part, PartKind, TINY_MM, dot};

/// The points of a lock stitch stay this near to their first point, mm.
pub const LOCK_RADIUS_MM: f64 = 1.0;
/// Two needle points nearer than this are one point (a machine file has
/// steps of 0.1 mm), mm.
pub const SAME_MM: f64 = 0.15;
/// The line of a run has no point nearer than this to the straight line
/// through the points next to it, mm.
pub const LINE_TOL_MM: f64 = 0.1;
/// A run is a bean stitch when this part of its stitches sew three times.
const BEAN: f64 = 0.6;
/// The rails are made smooth over this length on each side of a point, mm.
pub const RAIL_SMOOTH_MM: f64 = 0.6;
/// The rails have no point nearer than this to the straight line through
/// the points next to it, mm.
pub const RAIL_TOL_MM: f64 = 0.05;
/// A rung is put where the pairs of the rails are farther than this from
/// the pairs of the stitches, mm.
pub const RUNG_TOL_MM: f64 = 0.25;
/// A pass ends where the stitches go back this far along the column, mm.
const PASS_TURN_MM: f64 = 0.5;
/// Passes with a spacing of at most this times the spacing of the densest
/// pass are top stitches; the others are lighter (underlay).
const DENSE: f64 = 1.25;
/// A column with this spacing or more is a zigzag (open satin), mm.
pub const ZIGZAG_FROM_MM: f64 = 1.0;
/// A needle point of a short stitch is this far in from the line of its
/// side (a part of the width), and at least `SHORT_MIN_MM`.
const SHORT_IN: f64 = 0.08;
const SHORT_MIN_MM: f64 = 0.12;
/// Needle points on one side nearer than this get short stitches (as
/// `emb_stitch::satin::SHORT_DIST_MM`), mm.
const CROWDED_MM: f64 = 0.25;
/// A stitch across is at least this part of the median stitch across, and
/// at most this many times as long (not the travel into the column).
const ACROSS: f64 = 0.3;
const ACROSS_MAX: f64 = 4.0;
/// A needle point at an end of a pass that is this many times the median
/// step from the needle point before it on its side, and `END_JUMP_MM`, is
/// not a part of the column.
const END_JUMP: f64 = 4.0;
const END_JUMP_MM: f64 = 1.0;
/// A column has at least this many stitches across, and is this long, mm.
const MIN_ACROSS: usize = 4;
const MIN_COLUMN_MM: f64 = 0.6;
/// An underlay run covers this part of the length of the column, an
/// underlay pass this part.
const COVER: f64 = 0.6;
const PASS_COVER: f64 = 0.5;
/// This part of the points of an underlay is inside the column.
const INSIDE: f64 = 0.9;
/// A center walk is this near to the middle (a part of the width), or
/// `CENTER_MIN_MM`.
const CENTER_OFF: f64 = 0.15;
const CENTER_MIN_MM: f64 = 0.2;
/// The layers of an underlay setting: the inset within this, mm, and the
/// zigzag spacing within this part.
const LAYER_INSET_MM: f64 = 0.3;
const LAYER_SPACING: f64 = 0.2;

/// An object made from parts of a stitch file.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// A running or satin object, or `Stitches` for parts that did not
    /// become an object.
    pub kind: ObjectKind,
    /// The thread (its number in the colors of the file).
    pub thread: usize,
    /// The stitch records of the file that the object sews, in sew order.
    pub records: Vec<Range<usize>>,
}

impl Made {
    /// `true` for an object with a stitch type, `false` for stitches.
    pub fn recognized(&self) -> bool {
        !matches!(self.kind, ObjectKind::Stitches { .. })
    }
}

/// An item while the objects are made.
pub(crate) enum Item {
    Object(Box<ObjectKind>, usize, Vec<Range<usize>>),
    Stitches(usize, Range<usize>),
}

/// The objects of the stitches of `s` and their parts (`parts::parts`), in
/// sew order (see the module text). The records of the objects together
/// are the records of the parts.
pub fn objects(s: &StitchDesign, parts: &[Part]) -> Vec<Made> {
    let n = parts.len();
    // The part that each part sews for (itself, or the part that it is the
    // underlay of).
    let root: Vec<usize> = (0..n)
        .map(|i| {
            let mut k = i;
            for _ in 0..n {
                match parts[k].underlay_of {
                    Some(j) if j != k => k = j,
                    _ => break,
                }
            }
            k
        })
        .collect();
    // The parts that sew for a column or for rows go with it.
    let of_root = |i: usize| root[i] != i && matches!(parts[root[i]].kind, PartKind::Column | PartKind::Rows);
    let mut under: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if of_root(i) {
            under[root[i]].push(i);
        }
    }
    let mut items = Vec::new();
    // The parts of rows of one fill, and the short runs after them (travel
    // if more rows of the fill come after them).
    let mut group: Option<FillGroup> = None;
    let mut runs: Vec<usize> = Vec::new();
    let close = |group: &mut Option<FillGroup>, runs: &mut Vec<usize>, items: &mut Vec<Item>| {
        if let Some(g) = group.take() {
            fill_items(s, parts, &g, items);
        }
        for r in runs.drain(..) {
            run_item(s, &parts[r], items);
        }
    };
    for (i, p) in parts.iter().enumerate() {
        if of_root(i) {
            // With its column or its rows.
            continue;
        }
        if p.kind == PartKind::Rows && root[i] == i {
            match part_members(s, parts, i, &under[i]) {
                Some(members) => {
                    for (shape, member) in members {
                        if let Some(g) = group.as_mut()
                            && g.thread == p.thread
                            && g.shape.same_fill(&shape)
                        {
                            g.members.push(member);
                            g.travel.append(&mut runs);
                            continue;
                        }
                        close(&mut group, &mut runs, &mut items);
                        group = Some(FillGroup {
                            thread: p.thread,
                            shape,
                            members: vec![member],
                            travel: Vec::new(),
                        });
                    }
                }
                None => {
                    close(&mut group, &mut runs, &mut items);
                    for &u in &under[i] {
                        items.push(Item::Stitches(parts[u].thread, parts[u].records.clone()));
                    }
                    items.push(Item::Stitches(p.thread, p.records.clone()));
                }
            }
            continue;
        }
        // A column part with the rows of the open fill (rows of one stitch
        // in a narrow section) is rows of the fill.
        if p.kind == PartKind::Column && root[i] == i {
            let fill_rows = group.as_ref().filter(|g| g.thread == p.thread).and_then(|g| {
                part_members(s, parts, i, &under[i]).filter(|ms| ms.iter().all(|(shape, _)| g.shape.same_fill(shape)))
            });
            if let (Some(ms), Some(g)) = (fill_rows, group.as_mut()) {
                g.members.extend(ms.into_iter().map(|(_, m)| m));
                g.travel.append(&mut runs);
                continue;
            }
        }
        if root[i] == i && can_be_travel(p) && group.as_ref().is_some_and(|g| g.thread == p.thread) {
            runs.push(i);
            continue;
        }
        close(&mut group, &mut runs, &mut items);
        match p.kind {
            PartKind::Run if root[i] == i => run_item(s, p, &mut items),
            PartKind::Column if root[i] == i => column_items(s, parts, i, &under[i], &mut items),
            _ => items.push(Item::Stitches(p.thread, p.records.clone())),
        }
    }
    close(&mut group, &mut runs, &mut items);
    finish(s, items)
}

/// A running object of a run part, or its stitches.
pub(crate) fn run_item(s: &StitchDesign, p: &Part, items: &mut Vec<Item>) {
    match run_object(&points_of(s, &p.records)) {
        Some(kind) => items.push(Item::Object(Box::new(kind), p.thread, vec![p.records.clone()])),
        None => items.push(Item::Stitches(p.thread, p.records.clone())),
    }
}

/// The needle points of the stitches `r` of `s`: where the needle is before
/// the first, then the end of each stitch.
pub(crate) fn points_of(s: &StitchDesign, r: &Range<usize>) -> Vec<Point> {
    let before = if r.start > 0 {
        s.records[r.start - 1].pos
    } else {
        s.records[r.start].pos
    };
    std::iter::once(before)
        .chain(s.records[r.clone()].iter().map(|x| x.pos))
        .collect()
}

/// The made objects; stitches of one thread one after the other are one
/// stitch object (a new part of it after a jump or a trim).
fn finish(s: &StitchDesign, items: Vec<Item>) -> Vec<Made> {
    let mut out: Vec<Made> = Vec::new();
    let mut open: Option<(usize, Vec<Range<usize>>)> = None;
    let close = |open: &mut Option<(usize, Vec<Range<usize>>)>, out: &mut Vec<Made>| {
        if let Some((thread, records)) = open.take() {
            out.push(Made {
                kind: stitches_kind(s, &records),
                thread,
                records,
            });
        }
    };
    for item in items {
        match item {
            Item::Stitches(thread, r) => match &mut open {
                Some((t, rs)) if *t == thread => rs.push(r),
                _ => {
                    close(&mut open, &mut out);
                    open = Some((thread, vec![r]));
                }
            },
            Item::Object(kind, thread, records) => {
                close(&mut open, &mut out);
                out.push(Made {
                    kind: *kind,
                    thread,
                    records,
                });
            }
        }
    }
    close(&mut open, &mut out);
    out
}

/// A stitch object of the stitch records `records` (a new part where they
/// do not follow one another, after a trim if the file has one there).
fn stitches_kind(s: &StitchDesign, records: &[Range<usize>]) -> ObjectKind {
    let mut path = Path::new();
    let mut trims = Vec::new();
    let mut subpaths = 0usize;
    let mut last: Option<usize> = None;
    for r in records {
        let mut pts = points_of(s, r).into_iter();
        if last == Some(r.start) {
            pts.next();
        } else {
            if let Some(end) = last
                && s.records[end.min(r.start)..r.start]
                    .iter()
                    .any(|x| x.kind == StitchKind::Trim)
            {
                trims.push(subpaths);
            }
            if let Some(first) = pts.next() {
                path = path.move_to(first);
                subpaths += 1;
            }
        }
        for p in pts {
            path = path.line_to(p);
        }
        last = Some(r.end);
    }
    ObjectKind::Stitches { path, trims }
}

/// The points without the lock stitches at the ends and without the tiny
/// stitches, and the index of each point in `points`.
pub(crate) fn clean(points: &[Point]) -> (Vec<Point>, Vec<usize>) {
    let n = points.len();
    if n < 2 {
        return (points.to_vec(), (0..n).collect());
    }
    let near = |a: Point, b: Point| a.distance(b) < SAME_MM;
    // A lock at the start: back at the first point after two stitches or
    // more, with all the points near it.
    let mut from = 0;
    for k in (2..n.min(7)).rev() {
        if near(points[k], points[0]) && points[..k].iter().all(|p| p.distance(points[0]) <= LOCK_RADIUS_MM) {
            from = k;
            break;
        }
    }
    let mut to = n - 1;
    for k in (2..n.min(7)).rev() {
        let j = n - 1 - k;
        if j > from
            && near(points[j], points[n - 1])
            && points[j..].iter().all(|p| p.distance(points[n - 1]) <= LOCK_RADIUS_MM)
        {
            to = j;
            break;
        }
    }
    let mut pts = vec![points[from]];
    let mut index = vec![from];
    for (i, p) in points.iter().enumerate().take(to + 1).skip(from + 1) {
        if p.distance(pts[pts.len() - 1]) >= TINY_MM {
            pts.push(*p);
            index.push(i);
        } else if i == to && pts.len() > 1 {
            // The last point stays the last point.
            let k = pts.len() - 1;
            pts[k] = *p;
            index[k] = i;
        }
    }
    (pts, index)
}

/// The line of a run and the number of times that it sews each stitch (1,
/// or 3 for a bean stitch).
pub fn run_line(points: &[Point]) -> Option<(Vec<Point>, u32)> {
    let (pts, _) = clean(points);
    if pts.len() < 2 {
        return None;
    }
    // Each stitch three times: there, back and there again.
    let mut line = vec![pts[0]];
    let mut triples = 0usize;
    let mut k = 1;
    while k < pts.len() {
        let p = pts[k];
        let last = line[line.len() - 1];
        if k + 2 < pts.len() && pts[k + 1].distance(last) < SAME_MM && pts[k + 2].distance(p) < SAME_MM {
            triples += 1;
            k += 3;
        } else {
            k += 1;
        }
        line.push(p);
    }
    let forward = line.len() - 1;
    if forward >= 2 && triples as f64 >= BEAN * forward as f64 {
        Some((line, 3))
    } else {
        Some((pts, 1))
    }
}

/// A running object of the needle points of a run (see the module text).
pub fn run_object(points: &[Point]) -> Option<ObjectKind> {
    let (line, repeats) = run_line(points)?;
    let mut lengths: Vec<f64> = line
        .windows(2)
        .map(|w| w[0].distance(w[1]))
        .filter(|l| *l >= TINY_MM)
        .collect();
    if lengths.is_empty() {
        return None;
    }
    lengths.sort_by(f64::total_cmp);
    // Most stitches: the engine gives the stitches between two corners one
    // length, at most the stitch length.
    let most = lengths[((lengths.len() - 1) as f64 * 0.9).round() as usize];
    let stitch_length_mm = ((most * 10.0).round() / 10.0).clamp(0.5, parts::MAX_STITCH_MM);
    let closed = line.len() > 3
        && line[0].distance(line[line.len() - 1]) < SAME_MM
        && line.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() > 3.0;
    let mut pts = emb_geom::path::simplify(&line, LINE_TOL_MM);
    if closed && pts.len() > 3 {
        pts.pop();
    }
    let mut path = Path::new().move_to(pts[0]);
    for p in &pts[1..] {
        path = path.line_to(*p);
    }
    if closed && pts.len() > 2 {
        path = path.close();
    }
    Some(ObjectKind::Running {
        path,
        params: RunningParams {
            stitch_length_mm,
            repeats,
            ..Default::default()
        },
    })
}

/// A pass of a column: the stitches across from one end to the other.
#[derive(Debug, Clone)]
pub struct Pass {
    /// The needle points where the stitches across turn, in sew order (on
    /// the two sides in turn).
    pub turns: Vec<Point>,
    /// The length and the number of stitches of each stitch across.
    pub across: Vec<(f64, usize)>,
    /// The stitches of the pass (stitch `k` ends at point `k + 1` of the
    /// points of the part).
    pub stitches: Range<usize>,
}

impl Pass {
    /// Two times the length along the column (the middles of the stitches
    /// across) divided by the number of stitches across.
    fn spacing(&self) -> f64 {
        let mids: Vec<Point> = self.turns.windows(2).map(|w| w[0].midpoint(w[1])).collect();
        let along: f64 = mids.windows(2).map(|w| w[0].distance(w[1])).sum();
        2.0 * along / (mids.len().max(2) - 1) as f64
    }
}

/// The passes of the needle points of a column part (see the module text).
pub fn passes(points: &[Point]) -> Vec<Pass> {
    let (pts, index) = clean(points);
    let legs = parts::legs(&pts);
    if legs.len() < 3 {
        return Vec::new();
    }
    let mut lens: Vec<f64> = legs.iter().map(Leg::len).collect();
    lens.sort_by(f64::total_cmp);
    let median = lens[lens.len() / 2];
    let long = |k: usize| (ACROSS * median..=ACROSS_MAX * median).contains(&legs[k].len());
    let back = |a: usize, b: usize| dot(legs[a].dir(), legs[b].dir()) < BACK;
    let mut out = Vec::new();
    let mut k = 0;
    while k < legs.len() {
        if !long(k) {
            k += 1;
            continue;
        }
        let mut e = k;
        while e + 1 < legs.len() && long(e + 1) && back(e, e + 1) {
            e += 1;
        }
        if e + 1 - k >= 3 {
            split_passes(&legs[k..=e], &index, &mut out);
        }
        k = e + 1;
    }
    out
}

/// Cuts legs that go back and forth into passes where the column turns
/// back.
fn split_passes(legs: &[Leg], index: &[usize], out: &mut Vec<Pass>) {
    let n = legs.len();
    // The direction across, the same way for all legs.
    let mut u = vec![legs[0].dir()];
    for leg in &legs[1..] {
        let d = leg.dir();
        let last = u[u.len() - 1];
        u.push(if dot(d, last) >= 0.0 { d } else { (-d.0, -d.1) });
    }
    // The length along the column at each leg (across: the mean of two
    // stitches across, as a zigzag slants them one way and the other).
    let mid = |k: usize| legs[k].a.midpoint(legs[k].b);
    let mut at = vec![0.0];
    for k in 1..n {
        let m = mid(k) - mid(k - 1);
        let (cx, cy) = (u[k - 1].0 + u[k].0, u[k - 1].1 + u[k].1);
        let l = (cx * cx + cy * cy).sqrt().max(1e-9);
        at.push(at[k - 1] + (-m.x * cy + m.y * cx) / l);
    }
    // The turns back, with a hysteresis.
    let mut cuts = Vec::new();
    let mut dir = 0.0f64;
    let (mut lo, mut hi) = (at[0], at[0]);
    let (mut ext, mut ext_at) = (at[0], 0usize);
    for (k, &a) in at.iter().enumerate().skip(1) {
        if dir == 0.0 {
            lo = lo.min(a);
            hi = hi.max(a);
            if a - lo > PASS_TURN_MM {
                dir = 1.0;
            } else if hi - a > PASS_TURN_MM {
                dir = -1.0;
            }
            (ext, ext_at) = (a, k);
        } else if (a - ext) * dir > 0.0 {
            (ext, ext_at) = (a, k);
        } else if (ext - a) * dir > PASS_TURN_MM {
            cuts.push(ext_at);
            dir = -dir;
            (ext, ext_at) = (a, k);
        }
    }
    // The passes: legs `from..=to`.
    let mut from = 0;
    for to in cuts.into_iter().chain(std::iter::once(n - 1)) {
        if to < from {
            continue;
        }
        let seq = &legs[from..=to];
        let mut turns = vec![seq[0].a];
        turns.extend(seq.iter().map(|l| l.b));
        out.push(Pass {
            turns,
            across: seq.iter().map(|l| (l.len(), l.stitches())).collect(),
            stitches: index[seq[0].start]..index[seq[seq.len() - 1].end],
        });
        from = to + 1;
    }
}

/// A polyline with the length along it.
struct Line {
    pts: Vec<Point>,
    at: Vec<f64>,
}

impl Line {
    fn new(pts: Vec<Point>) -> Line {
        let mut at = vec![0.0];
        for w in pts.windows(2) {
            at.push(at[at.len() - 1] + w[0].distance(w[1]));
        }
        Line { pts, at }
    }

    fn len(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    fn at(&self, s: f64) -> Point {
        let s = s.clamp(0.0, self.len());
        let i = self.at.partition_point(|&l| l <= s).clamp(1, self.pts.len() - 1);
        let seg = self.at[i] - self.at[i - 1];
        let t = if seg > 0.0 { (s - self.at[i - 1]) / seg } else { 0.0 };
        self.pts[i - 1].lerp(self.pts[i], t)
    }

    fn path(&self) -> Path {
        let mut path = Path::new().move_to(self.pts[0]);
        for p in &self.pts[1..] {
            path = path.line_to(*p);
        }
        path
    }
}

/// The points made smooth: each point is the mean of the points within
/// `radius` along the line (nearer points count more); the ends stay.
fn smooth(pts: &[Point], radius: f64) -> Vec<Point> {
    let line = Line::new(pts.to_vec());
    let n = pts.len();
    let mut out = pts.to_vec();
    for (i, o) in out.iter_mut().enumerate().take(n.saturating_sub(1)).skip(1) {
        let (mut sx, mut sy, mut sw) = (0.0, 0.0, 0.0);
        let mut j = i;
        while j > 0 && line.at[i] - line.at[j - 1] < radius {
            j -= 1;
        }
        while j < n && line.at[j] - line.at[i] < radius {
            let w = 1.0 - (line.at[j] - line.at[i]).abs() / radius;
            sx += pts[j].x * w;
            sy += pts[j].y * w;
            sw += w;
            j += 1;
        }
        if sw > 0.0 {
            *o = Point::new(sx / sw, sy / sw);
        }
    }
    out
}

/// What the stitches of a top pass of a column show.
#[derive(Debug, Clone)]
pub struct ColumnShape {
    pub geometry: SatinGeometry,
    /// Two times the length along the middle divided by the number of
    /// stitches across.
    pub spacing_mm: f64,
    /// Needle points of short stitches were found, or no needle points are
    /// crowded.
    pub short_stitches: bool,
    /// The split length that gives each stitch across its number of
    /// stitches.
    pub split_mm: f64,
}

/// The turns of a pass without a needle point at an end that is far from
/// the needle point before it on its side (a travel stitch into the column
/// or out of it that goes back like a stitch across).
fn trim_ends(turns: &[Point]) -> Vec<Point> {
    let mut t = turns.to_vec();
    if t.len() < 5 {
        return t;
    }
    let steps: Vec<f64> = t.windows(3).map(|w| w[0].distance(w[2])).collect();
    let far = END_JUMP_MM.max(END_JUMP * median(steps));
    for _ in 0..3 {
        if t.len() > MIN_ACROSS + 1 && t[t.len() - 1].distance(t[t.len() - 3]) > far {
            t.pop();
        }
        if t.len() > MIN_ACROSS + 1 && t[0].distance(t[2]) > far {
            t.remove(0);
        }
    }
    t
}

/// The shape of a column from the needle points of a top pass (see the
/// module text).
pub fn column_shape(pass: &Pass) -> Option<ColumnShape> {
    let t = &trim_ends(&pass.turns);
    let n = t.len().checked_sub(1)?;
    if n < MIN_ACROSS {
        return None;
    }
    // Short stitches: a needle point in from the line between the needle
    // points next to it on its side.
    let mut short = vec![false; t.len()];
    let mut crowded = false;
    for j in 2..=n {
        if t[j].distance(t[j - 2]) < CROWDED_MM {
            crowded = true;
        }
        if j + 2 > n {
            continue;
        }
        let q = t[j].nearest_on_segment(t[j - 2], t[j + 2]);
        let other = t[j - 1].midpoint(t[j + 1]);
        let w = t[j].distance(other);
        let Some(c) = (other - t[j]).normalized() else { continue };
        if (t[j] - q).dot(c) > SHORT_MIN_MM.max(SHORT_IN * w) {
            short[j] = true;
        }
    }
    let found_short = short.iter().any(|s| *s);
    // The direction along the column at each stitch across (forward: the
    // way the pass goes).
    let mut along: Vec<Point> = (0..n)
        .map(|j| {
            let d = if j % 2 == 0 { t[j + 1] - t[j] } else { t[j] - t[j + 1] };
            d.normalized().unwrap_or(Point::ZERO).perp()
        })
        .collect();
    let progress: f64 = (1..n)
        .map(|j| (t[j].midpoint(t[j + 1]) - t[j - 1].midpoint(t[j])).dot(along[j - 1]))
        .sum();
    if progress < 0.0 {
        for v in &mut along {
            *v = *v * -1.0;
        }
    }
    // A needle point is on its rail if it is not a short stitch and it goes
    // forward from the point before it on its side (at a sharp turn the
    // needle points on the inside pivot and go back a little).
    let mut on_rail = vec![false; t.len()];
    for first in [0, 1] {
        let mut last: Option<Point> = None;
        for j in (first..=n).step_by(2) {
            let dir = along[j.min(n - 1)] + along[j.saturating_sub(1).min(n - 1)];
            if !short[j] && last.is_none_or(|l| (t[j] - l).dot(dir) > 0.0) {
                on_rail[j] = true;
                last = Some(t[j]);
            }
        }
    }
    // The rails, and the length along its rail at each needle point (from
    // the order of the points, not the nearest point: a rail can come near
    // itself, round a small counter).
    let mut at: Vec<Option<f64>> = vec![None; t.len()];
    // Half a step out, at most half of the median step on a side (at a
    // pointed end the steps can be long).
    let half = 0.5 * median(t.windows(3).map(|w| w[0].distance(w[2])).collect());
    let out = |from: Point, back: Point| {
        let d = from - back;
        let l = d.length();
        if l > 2.0 * half && l > 0.0 {
            from + d * (half / l)
        } else {
            from + d * 0.5
        }
    };
    let mut rail = |first: usize| -> Option<Line> {
        let mut pts: Vec<Point> = Vec::new();
        let mut index: Vec<Option<usize>> = Vec::new();
        if first > 0 {
            pts.push(out(t[first], t[first + 2]));
            index.push(None);
        }
        for j in (first..=n).step_by(2).filter(|&j| on_rail[j]) {
            if pts.last().is_some_and(|l| l.distance(t[j]) < 1e-6) {
                continue;
            }
            pts.push(t[j]);
            index.push(Some(j));
        }
        let last = first + (n - first) / 2 * 2;
        if last < n {
            pts.push(out(t[last], t[last - 2]));
            index.push(None);
        }
        if pts.len() < 2 {
            return None;
        }
        let smooth = Line::new(smooth(&pts, RAIL_SMOOTH_MM));
        let line = Line::new(emb_geom::path::simplify(&smooth.pts, RAIL_TOL_MM));
        let scale = line.len() / smooth.len().max(1e-9);
        for (k, j) in index.iter().enumerate() {
            if let Some(j) = j {
                at[*j] = Some(smooth.at[k] * scale);
            }
        }
        Some(line)
    };
    let a = rail(0)?;
    let b = rail(1)?;
    let mid_len = (a.len() + b.len()) / 2.0;
    if mid_len < MIN_COLUMN_MM {
        return None;
    }
    // The pairs of the stitches: a needle point and the middle of the two
    // needle points next to it (on the other side).
    let mut samples = vec![Point::new(0.0, 0.0)];
    for j in 1..n {
        let (Some(own), Some(o0), Some(o1)) = (at[j], at[j - 1], at[j + 1]) else {
            continue;
        };
        let other = (o0 + o1) / 2.0;
        let s = if j % 2 == 0 {
            Point::new(own, other)
        } else {
            Point::new(other, own)
        };
        let last = samples[samples.len() - 1];
        if s.x > last.x + 1e-6 && s.y > last.y + 1e-6 && s.x < a.len() - 1e-6 && s.y < b.len() - 1e-6 {
            samples.push(s);
        }
    }
    samples.push(Point::new(a.len(), b.len()));
    let kept = emb_geom::path::simplify(&samples, RUNG_TOL_MM);
    let rungs: Vec<(Point, Point)> = kept[1..kept.len() - 1].iter().map(|s| (a.at(s.x), b.at(s.y))).collect();
    // The split length: the default if it gives the stitches across their
    // number of stitches, else from the longest stitch of a split.
    let default = emb_model::object::SATIN_SPLIT_MM;
    let gives = |split: f64, len: f64, count: usize| ((len / split - 1e-9).ceil().max(1.0) as usize) == count;
    let fits = pass.across.iter().filter(|&&(l, c)| gives(default, l, c)).count();
    let split_mm = if fits as f64 >= 0.9 * pass.across.len() as f64 {
        default
    } else if pass.across.iter().all(|&(_, c)| c == 1) {
        let longest = pass.across.iter().map(|&(l, _)| l).fold(0.0, f64::max);
        (longest + 0.5).max(default)
    } else {
        pass.across
            .iter()
            .filter(|&&(_, c)| c > 1)
            .map(|&(l, c)| l / c as f64)
            .fold(0.0, f64::max)
            .max(1.0)
            + 0.05
    };
    Some(ColumnShape {
        geometry: SatinGeometry {
            rail_a: a.path(),
            rail_b: b.path(),
            rungs,
        },
        spacing_mm: 2.0 * mid_len / n as f64,
        short_stitches: found_short || !crowded,
        split_mm,
    })
}

/// The needle points of an underlay of a column.
#[derive(Debug, Clone)]
pub enum UnderPoints {
    /// The needle points of a run.
    Run(Vec<Point>),
    /// A lighter pass across the column.
    Pass(Pass),
}

/// Where points are in a column: for each point, the pair across the
/// column (its number), how far across (0 at rail A, 1 at rail B), and the
/// width there; `None` for a point outside.
fn locate(pairs: &[(Point, Point)], pts: &[Point]) -> Vec<Option<(usize, f64, f64)>> {
    pts.iter()
        .map(|&p| {
            let mut best: Option<(f64, usize, f64, f64)> = None;
            for (j, &(a, b)) in pairs.iter().enumerate() {
                let d = b - a;
                let l2 = d.length_sq();
                let t = if l2 > 0.0 { (p - a).dot(d) / l2 } else { 0.5 };
                let dist = p.distance(a.lerp(b, t.clamp(0.0, 1.0)));
                if best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, j, t, l2.sqrt()));
                }
            }
            let (dist, j, t, w) = best?;
            (dist <= 0.5 && (-0.05..=1.05).contains(&t)).then_some((j, t.clamp(0.0, 1.0), w))
        })
        .collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The underlay layers of a column from its underlay, and the numbers of
/// the runs (in `under`) that are not a layer (travel).
fn layers(g: &SatinGeometry, under: &[UnderPoints]) -> (Vec<UnderlayLayer>, Vec<usize>) {
    let runs = || {
        under
            .iter()
            .enumerate()
            .filter(|(_, u)| matches!(u, UnderPoints::Run(_)))
            .map(|(k, _)| k)
            .collect()
    };
    let Some(col) = emb_stitch::satin::Column::new(g) else {
        return (Vec::new(), runs());
    };
    let pairs = col.pairs(0.5);
    let np = pairs.len().max(2);
    // The length along the middle at each pair.
    let mut along = vec![0.0];
    for w in pairs.windows(2) {
        along.push(along[along.len() - 1] + w[0].0.midpoint(w[0].1).distance(w[1].0.midpoint(w[1].1)));
    }
    let cover = |js: &[usize]| -> f64 {
        match (js.iter().min(), js.iter().max()) {
            (Some(&lo), Some(&hi)) => (hi - lo) as f64 / (np - 1) as f64,
            _ => 0.0,
        }
    };
    let mut out = Vec::new();
    let mut travel = Vec::new();
    // Zigzag passes (inset, spacing); two alike passes are one layer (out
    // and back).
    let mut zig: Vec<(f64, f64)> = Vec::new();
    let flush = |zig: &mut Vec<(f64, f64)>, out: &mut Vec<UnderlayLayer>| {
        let mut k = 0;
        while k < zig.len() {
            let (i0, s0) = zig[k];
            let (inset, spacing, step) = match zig.get(k + 1) {
                Some(&(i1, s1)) if (s1 / s0 - 1.0).abs() <= 0.25 => ((i0 + i1) / 2.0, (s0 + s1) / 2.0, 2),
                _ => (i0, s0, 1),
            };
            out.push(UnderlayLayer::Zigzag {
                inset_mm: round2(inset),
                spacing_mm: round2(spacing),
            });
            k += step;
        }
        zig.clear();
    };
    for (k, u) in under.iter().enumerate() {
        match u {
            UnderPoints::Run(pts) => {
                let (pts, _) = clean(pts);
                let inside: Vec<(usize, f64, f64)> = locate(&pairs, &pts).into_iter().flatten().collect();
                if inside.is_empty() || (inside.len() as f64) < INSIDE * pts.len() as f64 {
                    travel.push(k);
                    continue;
                }
                let w = median(inside.iter().map(|x| x.2).collect());
                let off = median(inside.iter().map(|x| (x.1 - 0.5).abs() * x.2).collect());
                let js: Vec<usize> = inside.iter().map(|x| x.0).collect();
                let side_a: Vec<usize> = inside.iter().filter(|x| x.1 < 0.5).map(|x| x.0).collect();
                let side_b: Vec<usize> = inside.iter().filter(|x| x.1 >= 0.5).map(|x| x.0).collect();
                if off <= CENTER_MIN_MM.max(CENTER_OFF * w) && cover(&js) >= COVER {
                    flush(&mut zig, &mut out);
                    out.push(UnderlayLayer::CenterWalk);
                } else if cover(&side_a) >= COVER && cover(&side_b) >= COVER {
                    flush(&mut zig, &mut out);
                    let inset = median(inside.iter().map(|x| x.1.min(1.0 - x.1) * x.2).collect());
                    out.push(UnderlayLayer::EdgeWalk {
                        inset_mm: round2(inset),
                    });
                } else {
                    travel.push(k);
                }
            }
            UnderPoints::Pass(p) => {
                let inside: Vec<(usize, f64, f64)> = locate(&pairs, &p.turns).into_iter().flatten().collect();
                let js: Vec<usize> = inside.iter().map(|x| x.0).collect();
                if (inside.len() as f64) < INSIDE * p.turns.len() as f64 || cover(&js) < PASS_COVER {
                    continue;
                }
                let inset = median(inside.iter().map(|x| x.1.min(1.0 - x.1) * x.2).collect());
                let (lo, hi) = (
                    js.iter().min().copied().unwrap_or(0),
                    js.iter().max().copied().unwrap_or(0),
                );
                let spacing = 2.0 * (along[hi] - along[lo]) / (p.turns.len() - 1).max(1) as f64;
                zig.push((inset, spacing));
            }
        }
    }
    flush(&mut zig, &mut out);
    (out, travel)
}

/// The underlay setting of the layers at the mean width `w`: the setting
/// of the engine that has these layers, else the layers.
fn underlay_setting(found: Vec<UnderlayLayer>, w: f64) -> Underlay {
    use emb_stitch::satin::{auto_underlay, double_underlay, heavy_underlay, light_underlay};
    let alike = |a: &UnderlayLayer, b: &UnderlayLayer| match (a, b) {
        (UnderlayLayer::CenterWalk, UnderlayLayer::CenterWalk) => true,
        (UnderlayLayer::EdgeWalk { inset_mm: x }, UnderlayLayer::EdgeWalk { inset_mm: y }) => {
            (x - y).abs() <= LAYER_INSET_MM
        }
        (
            UnderlayLayer::Zigzag {
                inset_mm: x,
                spacing_mm: s,
            },
            UnderlayLayer::Zigzag {
                inset_mm: y,
                spacing_mm: t,
            },
        ) => (x - y).abs() <= LAYER_INSET_MM && (s / t - 1.0).abs() <= LAYER_SPACING,
        _ => false,
    };
    let same = |set: Vec<UnderlayLayer>| set.len() == found.len() && set.iter().zip(&found).all(|(a, b)| alike(a, b));
    for (setting, set) in [
        (Underlay::Auto, auto_underlay(w)),
        (Underlay::Light, light_underlay(w)),
        (Underlay::Heavy, heavy_underlay(w)),
        (Underlay::Double, double_underlay(w)),
    ] {
        if same(set) {
            return setting;
        }
    }
    if found.is_empty() {
        Underlay::None
    } else {
        Underlay::Custom(found)
    }
}

/// A satin object of a top pass and its underlay (in sew order), and the
/// numbers of the runs of the underlay that are not a layer (travel).
pub fn column_object(top: &Pass, under: &[UnderPoints]) -> Option<(ObjectKind, Vec<usize>)> {
    let shape = column_shape(top)?;
    let zigzag = shape.spacing_mm >= ZIGZAG_FROM_MM;
    let (found, travel) = layers(&shape.geometry, under);
    let w = emb_stitch::satin::Column::new(&shape.geometry)
        .map(|c| c.mean_width())
        .unwrap_or(0.0);
    let mut params = if zigzag {
        SatinParams::with_pattern(ColumnPattern::Zigzag)
    } else {
        SatinParams::default()
    };
    params.spacing_mm = round2(shape.spacing_mm);
    params.pull_compensation_mm = 0.0;
    params.short_stitches = shape.short_stitches && !zigzag;
    params.split_mm = round2(shape.split_mm);
    params.underlay = underlay_setting(found, w);
    Some((
        ObjectKind::Satin {
            geometry: shape.geometry,
            params,
        },
        travel,
    ))
}

/// The items of the column part `i` with the parts of its underlay
/// (`under`, in sew order).
fn column_items(s: &StitchDesign, parts: &[Part], i: usize, under: &[usize], items: &mut Vec<Item>) {
    let part = &parts[i];
    let ps = passes(&points_of(s, &part.records));
    if ps.is_empty() {
        for &u in under {
            items.push(Item::Stitches(parts[u].thread, parts[u].records.clone()));
        }
        items.push(Item::Stitches(part.thread, part.records.clone()));
        return;
    }
    let base = part.records.start;
    // The records of each pass: from its first stitch to the first stitch
    // of the next pass (the first pass from the start of the part).
    let ranges: Vec<Range<usize>> = (0..ps.len())
        .map(|k| {
            let start = if k == 0 { base } else { base + ps[k].stitches.start };
            let end = match ps.get(k + 1) {
                Some(next) => base + next.stitches.start,
                None => part.records.end,
            };
            start..end.max(start)
        })
        .collect();
    // Only a pass with enough stitches across can be a column.
    let full = |p: &Pass| p.across.len() >= MIN_ACROSS;
    let densest = ps
        .iter()
        .filter(|p| full(p))
        .map(Pass::spacing)
        .fold(f64::INFINITY, f64::min);
    let dense: Vec<bool> = ps.iter().map(|p| full(p) && p.spacing() <= DENSE * densest).collect();
    // The underlay that waits for its top pass: the needle points, the part
    // of each run, and the records.
    let mut wait: Vec<(Option<usize>, UnderPoints)> = Vec::new();
    let mut wait_records: Vec<Range<usize>> = Vec::new();
    for &u in under {
        let pts = points_of(s, &parts[u].records);
        if parts[u].kind == PartKind::Column {
            wait.extend(passes(&pts).into_iter().map(|p| (None, UnderPoints::Pass(p))));
        } else {
            wait.push((Some(u), UnderPoints::Run(pts)));
        }
        wait_records.push(parts[u].records.clone());
    }
    for (k, p) in ps.into_iter().enumerate() {
        if !dense[k] {
            wait.push((None, UnderPoints::Pass(p)));
            wait_records.push(ranges[k].clone());
            continue;
        }
        let (of, points): (Vec<Option<usize>>, Vec<UnderPoints>) = std::mem::take(&mut wait).into_iter().unzip();
        let records = std::mem::take(&mut wait_records);
        match column_object(&p, &points) {
            Some((kind, travel)) => {
                // Travel under the column: runs of their own, before it.
                let travel: Vec<usize> = travel.into_iter().filter_map(|k| of[k]).collect();
                for &u in &travel {
                    run_item(s, &parts[u], items);
                }
                let mut own: Vec<Range<usize>> = records
                    .into_iter()
                    .filter(|r| !travel.iter().any(|&u| parts[u].records == *r))
                    .collect();
                own.push(ranges[k].clone());
                own.retain(|r| !r.is_empty());
                items.push(Item::Object(Box::new(kind), part.thread, own));
            }
            None => {
                for r in records.into_iter().chain(std::iter::once(ranges[k].clone())) {
                    if !r.is_empty() {
                        items.push(Item::Stitches(part.thread, r));
                    }
                }
            }
        }
    }
    // Lighter passes after the last top pass.
    for r in wait_records {
        if !r.is_empty() {
            items.push(Item::Stitches(part.thread, r));
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::StitchRecord;

    use super::*;
    use crate::parts::parts;

    fn design(points: &[Point]) -> StitchDesign {
        StitchDesign {
            threads: vec![Default::default()],
            records: points.iter().map(|p| StitchRecord::stitch(p.x, p.y)).collect(),
        }
    }

    /// The needle points of a satin of the engine: rails `w` mm apart along
    /// a curve, with the settings `p`.
    fn engine_satin(w: f64, p: &SatinParams) -> (SatinGeometry, Vec<Point>) {
        let line = Path::new().move_to(Point::new(0.0, 0.0)).cubic_to(
            Point::new(15.0, -10.0),
            Point::new(30.0, 10.0),
            Point::new(45.0, 0.0),
        );
        let g = SatinGeometry::from_centerline(&line, w);
        let pts = emb_stitch::satin::satin(&g, p, Point::new(-5.0, 0.0), None).expect("stitches");
        (g, pts)
    }

    fn mean_width(g: &SatinGeometry) -> f64 {
        emb_stitch::satin::Column::new(g).map(|c| c.mean_width()).unwrap_or(0.0)
    }

    #[test]
    fn a_satin_of_the_engine_comes_back_with_its_width_spacing_and_underlay() {
        let p = SatinParams::default();
        let (g, pts) = engine_satin(4.0, &p);
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let satins: Vec<&Made> = made
            .iter()
            .filter(|m| matches!(m.kind, ObjectKind::Satin { .. }))
            .collect();
        assert_eq!(satins.len(), 1, "{made:?}");
        let ObjectKind::Satin { geometry, params } = &satins[0].kind else {
            unreachable!()
        };
        // The sides are where the needle points are: the width with the pull
        // compensation on each side.
        let sewn = mean_width(&g) + 2.0 * p.pull_compensation_mm;
        assert!(
            (mean_width(geometry) - sewn).abs() < 0.1,
            "{} {sewn}",
            mean_width(geometry)
        );
        assert!((params.spacing_mm - p.spacing_mm).abs() < 0.04, "{}", params.spacing_mm);
        assert_eq!(params.pull_compensation_mm, 0.0);
        // Auto at this width: an edge walk and a zigzag.
        assert_eq!(params.underlay, Underlay::Auto, "{:?}", params.underlay);
        // No stitch is lost.
        let all: usize = made.iter().flat_map(|m| m.records.iter()).map(|r| r.len()).sum();
        assert_eq!(all, s.records.len());
    }

    #[test]
    fn a_satin_with_other_underlay_gets_its_layers() {
        let p = SatinParams {
            underlay: Underlay::Custom(vec![
                UnderlayLayer::CenterWalk,
                UnderlayLayer::Zigzag {
                    inset_mm: 0.8,
                    spacing_mm: 3.0,
                },
            ]),
            spacing_mm: 0.5,
            ..Default::default()
        };
        let (_, pts) = engine_satin(5.0, &p);
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let Some(ObjectKind::Satin { params, .. }) = made
            .iter()
            .map(|m| &m.kind)
            .find(|k| matches!(k, ObjectKind::Satin { .. }))
        else {
            panic!("{made:?}")
        };
        let Underlay::Custom(layers) = &params.underlay else {
            panic!("{:?}", params.underlay)
        };
        assert_eq!(layers.len(), 2, "{layers:?}");
        assert_eq!(layers[0], UnderlayLayer::CenterWalk);
        let UnderlayLayer::Zigzag { inset_mm, spacing_mm } = layers[1] else {
            panic!("{layers:?}")
        };
        // The inset from the sewn side: the inset and the pull compensation.
        assert!((inset_mm - 1.0).abs() < 0.2, "{inset_mm}");
        assert!((spacing_mm - 3.0).abs() < 0.4, "{spacing_mm}");
        assert!((params.spacing_mm - 0.5).abs() < 0.05, "{}", params.spacing_mm);
    }

    #[test]
    fn a_run_gets_its_line_and_stitch_length_and_a_bean_stitch_its_repeats() {
        let line: Vec<Point> = (0..=20).map(|k| Point::new(2.5 * k as f64, 0.0)).collect();
        let Some(ObjectKind::Running { path, params }) = run_object(&line) else {
            panic!()
        };
        assert_eq!(params.stitch_length_mm, 2.5);
        assert_eq!(params.repeats, 1);
        // A straight line has two points.
        assert_eq!(path.points().count(), 2);
        let bean = emb_stitch::running::bean(&line, 3);
        let Some(ObjectKind::Running { params, .. }) = run_object(&bean) else {
            panic!()
        };
        assert_eq!((params.stitch_length_mm, params.repeats), (2.5, 3));
        // Lock stitches at the ends are not a part of the line.
        let mut locked = vec![line[0], Point::new(0.5, 0.0), line[0], Point::new(0.5, 0.0), line[0]];
        locked.extend(&line[1..]);
        let (l, _) = run_line(&locked).unwrap();
        assert_eq!(l[0], line[0]);
        assert!(l.windows(2).all(|w| w[1].x > w[0].x), "{l:?}");
    }

    #[test]
    fn an_open_column_is_a_zigzag() {
        let p = SatinParams {
            spacing_mm: 1.5,
            ..SatinParams::with_pattern(ColumnPattern::Zigzag)
        };
        let (_, pts) = engine_satin(3.0, &p);
        let s = design(&pts);
        let made = objects(&s, &parts(&s));
        let Some(ObjectKind::Satin { params, .. }) = made
            .iter()
            .map(|m| &m.kind)
            .find(|k| matches!(k, ObjectKind::Satin { .. }))
        else {
            panic!("{made:?}")
        };
        assert_eq!(params.pattern, ColumnPattern::Zigzag);
        assert!((params.spacing_mm - 1.5).abs() < 0.15, "{}", params.spacing_mm);
    }

    /// The time of the objects of many satins and runs. Run with `cargo test
    /// --release -p emb-art -- --ignored objects_time --nocapture`.
    #[test]
    #[ignore]
    fn objects_time() {
        let p = SatinParams::default();
        let mut pts = Vec::new();
        for k in 0..40 {
            let (_, satin) = engine_satin(3.0 + 0.1 * k as f64, &p);
            pts.extend(satin.iter().map(|q| Point::new(q.x, q.y + 20.0 * k as f64)));
            pts.extend((0..40).map(|i| Point::new(2.5 * i as f64, 20.0 * k as f64 + 8.0)));
        }
        let s = design(&pts);
        let t0 = std::time::Instant::now();
        let p = parts(&s);
        let t1 = std::time::Instant::now();
        let made = objects(&s, &p);
        println!(
            "{} records: parts {:?}, {} objects {:?}",
            s.records.len(),
            t1 - t0,
            made.len(),
            t1.elapsed()
        );
    }
}
