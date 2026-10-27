//! The stitch plan: all objects in sew order as one stitch list.
//!
//! See `docs/design/export-plan.md` (Milestone 1 decisions) for the rules:
//! routing, travel stitch or trim, lock stitches, color changes and
//! stitch-length limits. The output follows the `StitchDesign` contract:
//! mm, Y down, origin at the hoop center, design rotation applied.

use emb_geom::{Point, Transform};
use emb_model::{
    Design, MachineProfile, ObjectId, RunPass, RunStyle, StitchDesign, StitchKind, StitchRecord, Thread, TrimMode,
};

use crate::object::{Run, RunLink, StitchCache, StitchError, StopReason};
use crate::running;

/// Lock stitches shorter than this are left out, in mm.
const MIN_LOCK_MM: f64 = 0.1;

#[derive(Debug, Clone, PartialEq)]
pub struct PlanOptions {
    /// Longer stitches are split into equal parts.
    pub max_stitch_mm: f64,
    /// Shorter stitches are dropped (not lock stitches, not the first stitch after a jump).
    pub min_stitch_mm: f64,
    /// Same-color gaps up to this length get a travel stitch; longer gaps get a trim.
    pub trim_mm: f64,
    /// Length of tie-in and tie-off stitches.
    pub lock_mm: f64,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            max_stitch_mm: 12.1,
            min_stitch_mm: 0.3,
            trim_mm: 2.0,
            lock_mm: 0.5,
        }
    }
}

impl PlanOptions {
    pub fn for_machine(m: &MachineProfile) -> Self {
        let d = Self::default();
        Self {
            max_stitch_mm: m.max_stitch_length_mm,
            min_stitch_mm: m.min_stitch_length_mm,
            trim_mm: m.trim_gap_mm.unwrap_or(d.trim_mm),
            ..d
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanWarning {
    pub object: Option<ObjectId>,
    pub message: String,
}

/// Where one object starts and ends in the plan (output coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSpan {
    pub object: ObjectId,
    pub start: Point,
    pub end: Point,
    /// The records of the object in `StitchPlan::stitches` (with its travel
    /// and lock stitches; without the trims and jumps before it).
    pub records: std::ops::Range<usize>,
}

/// A stop of the machine in the plan, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanStop {
    /// The index of the stop record in `StitchPlan::stitches`.
    pub record: usize,
    /// The object that asks for the stop.
    pub object: ObjectId,
    pub reason: StopReason,
}

#[derive(Debug, Clone, Default)]
pub struct StitchPlan {
    pub stitches: StitchDesign,
    pub warnings: Vec<PlanWarning>,
    /// Start and end of each stitched object, in sew order.
    pub spans: Vec<ObjectSpan>,
    /// The stops, in sew order (not the color changes).
    pub stops: Vec<PlanStop>,
}

/// One run of a color block, routed: its object, needle points (output
/// coordinates) and how it joins the run before it.
struct Routed {
    id: ObjectId,
    points: Vec<Point>,
    link: RunLink,
    exact: bool,
    stop: Option<StopReason>,
    long_jumps: bool,
}

/// Builds the stitch plan for the design.
pub fn build_plan(design: &Design, opts: &PlanOptions, cache: &mut StitchCache) -> StitchPlan {
    let to_out = design.output_transform();
    let to_design = to_out.inverse().unwrap_or(Transform::IDENTITY);
    let mut b = Builder::new(opts);
    // Auto start (ADR 0032): the needle goes to the start point first, so
    // the first object starts from there.
    let anchor = |a: Option<emb_model::Anchor>| a.and_then(|a| design.anchor_point(a)).map(|p| to_out.apply(p));
    if let Some(p) = anchor(design.start_end.start) {
        b.jump(p);
    }
    let mut threads: Vec<Thread> = Vec::new();
    let mut warnings = Vec::new();
    // The object before sews over foam.
    let mut last_puff = false;
    let mut spans = Vec::new();

    for (thread_id, ids) in design.sewn_color_blocks() {
        // The runs of this block, routed in sew order.
        let mut block: Vec<Routed> = Vec::new();
        let mut needle = b.needle;
        for id in ids {
            let Some(obj) = design.object(id) else { continue };
            let stitches = cache.get(obj, to_design.apply(needle));
            let runs = match stitches.as_ref() {
                Ok(s) => &s.runs,
                Err(e) => {
                    if !matches!(e, StitchError::Empty) || warnings.len() < 1000 {
                        warnings.push(PlanWarning {
                            object: Some(id),
                            message: e.to_string(),
                        });
                    }
                    continue;
                }
            };
            let mut span: Option<ObjectSpan> = None;
            for (i, run) in runs.iter().enumerate() {
                // A start point set by the user wins over the needle position.
                let reference = match (i, obj.start) {
                    (0, Some(s)) => to_out.apply(s),
                    _ => needle,
                };
                // An end point set by the user sets the direction of the last run.
                let end = obj
                    .end
                    .filter(|_| i + 1 == runs.len() && (i > 0 || obj.start.is_none()))
                    .map(|e| to_out.apply(e));
                let pts = route(run, &to_out, reference, end);
                if let (Some(&first), Some(&last)) = (pts.first(), pts.last()) {
                    needle = last;
                    block.push(Routed {
                        id,
                        points: pts,
                        link: run.link,
                        exact: run.exact,
                        stop: run.stop,
                        long_jumps: run.long_jumps,
                    });
                    let s = span.get_or_insert(ObjectSpan {
                        object: id,
                        start: first,
                        end: last,
                        records: 0..0,
                    });
                    s.end = last;
                }
            }
            spans.extend(span);
        }
        if block.is_empty() {
            continue;
        }
        if !threads.is_empty() {
            b.color_change();
        }
        threads.push(design.thread(thread_id).cloned().unwrap_or_else(|| Thread {
            id: thread_id,
            name: format!("Thread {}", thread_id.0),
            color: emb_model::Rgb(0, 0, 0),
            brand: None,
            code: None,
        }));
        let mut prev: Option<ObjectId> = None;
        for (k, r) in block.iter().enumerate() {
            // Puff: a stop to put the foam on before the first of the puff
            // objects that sew one after the other (ADR 0050).
            if prev != Some(r.id) {
                let puff = design.object(r.id).is_some_and(|o| crate::object::is_puff(&o.kind));
                if puff && !last_puff {
                    b.pending_stop = Some((r.id, StopReason::PlaceFoam));
                }
                last_puff = puff;
            }
            let Routed {
                id,
                points: pts,
                link,
                exact,
                stop,
                long_jumps,
            } = r;
            let f = design.object(*id).map(|o| o.functions).unwrap_or_default();
            // A run that says how it joins (stitches from a machine file)
            // decides; between objects, the trim mode of the object before;
            // between the runs of one object, the gap.
            let join = match (link, prev) {
                (RunLink::Jump, _) => Join::Jump,
                (RunLink::Trim, _) => Join::Trim(TrimMode::Always),
                (RunLink::Travel, _) => Join::Travel,
                (RunLink::Auto, Some(p)) if p != *id => {
                    Join::Trim(design.object(p).map_or(TrimMode::Auto, |o| o.functions.trim))
                }
                (RunLink::Auto, _) => Join::Trim(TrimMode::Auto),
            };
            // The records of the run start after the trim and jump before it, if any.
            if let Some(reason) = stop {
                b.pending_stop = Some((*id, *reason));
            }
            let before = b.recs.len();
            b.long_jumps = *long_jumps;
            b.want_slow = f.slow;
            b.sew_run(pts, join, f.tie_in, *exact);
            b.long_jumps = false;
            b.tie_off_enabled = f.tie_off;
            let end = b.recs.len();
            let start = b.recs[before..end]
                .iter()
                .rposition(|r| {
                    matches!(
                        r.kind,
                        StitchKind::Jump | StitchKind::Trim | StitchKind::Slow | StitchKind::Fast
                    )
                })
                .map_or(before, |i| before + i + 1);
            if let Some(s) = spans.iter_mut().rev().find(|s| s.object == *id) {
                if s.records.is_empty() {
                    s.records = start..end;
                } else {
                    s.records.end = end;
                }
            }
            // A stop after the last run of the object, written when the next
            // run starts (after the trim, if there is one). A color change or
            // the end of the design comes after the last object of a block,
            // and the machine stops there anyway.
            let last_run = block.get(k + 1).is_none_or(|next| next.id != *id);
            if f.stop && last_run && k + 1 < block.len() {
                b.pending_stop = Some((*id, StopReason::Function));
            }
            prev = Some(*id);
        }
        b.finish_block();
    }
    // Auto end: after the last trim, the needle goes to the end point.
    if let Some(p) = anchor(design.start_end.end)
        && b.recs.iter().any(|r| r.kind == StitchKind::Stitch)
    {
        b.jump(p);
    }
    b.push(StitchKind::End, b.needle);

    StitchPlan {
        stitches: StitchDesign {
            threads,
            records: b.recs,
        },
        warnings,
        spans,
        stops: b.stops,
    }
}

impl StitchPlan {
    /// A line for each stop: the color, the object and what the operator
    /// does, for example "Color 1, object #2 Patch: Put the appliqué fabric on".
    pub fn stop_notes(&self, design: &Design) -> Vec<String> {
        self.stops
            .iter()
            .map(|s| {
                // A stop at a color change is in the new color.
                let color = 1 + self.stitches.records[..=s.record.min(self.stitches.records.len() - 1)]
                    .iter()
                    .filter(|r| r.kind == StitchKind::ColorChange)
                    .count();
                let n = design.index_of(s.object).map_or(0, |i| i + 1);
                let name = design
                    .object(s.object)
                    .map(|o| o.name.clone().unwrap_or_else(|| o.type_name().to_string()))
                    .unwrap_or_default();
                format!("Color {color}, object #{n} {name}: {}", s.reason.text())
            })
            .collect()
    }
}

/// Puts the run in output coordinates, chooses where it starts, and applies
/// the style (backstitch, stem stitch), bean stitch and the pass (backtrack,
/// repeat). With `end`, the run ends as near to `end` as it can.
fn route(run: &Run, to_out: &Transform, reference: Point, end: Option<Point>) -> Vec<Point> {
    let mut pts: Vec<Point> = run.points.iter().map(|p| to_out.apply(*p)).collect();
    if pts.len() < 2 {
        return pts;
    }
    if run.fixed {
        // Fill and satin: the object chose its start from the needle position.
    } else if run.closed {
        // Start at the needle point nearest to the reference (or to the end,
        // because a loop ends where it starts). The last point repeats the
        // first, so rotate the open list and close it again.
        pts.pop();
        let start = nearest(&pts, end.unwrap_or(reference));
        pts.rotate_left(start);
        pts.push(pts[0]);
    } else if let Some(e) = end {
        if e.distance(pts[0]) < e.distance(pts[pts.len() - 1]) {
            pts.reverse();
        }
    } else if reference.distance(pts[pts.len() - 1]) < reference.distance(pts[0]) {
        pts.reverse();
    }
    let pts = match run.style {
        RunStyle::Backstitch => running::backstitch(&pts),
        RunStyle::Stemstitch => running::stemstitch(&pts),
        RunStyle::Sculpture => running::sculpture(&pts),
        RunStyle::Run | RunStyle::Manual => pts,
    };
    let mut pts = running::bean(&pts, run.repeats);
    match run.pass {
        RunPass::Repeat if run.closed => {
            let again: Vec<Point> = pts[1..].to_vec();
            pts.extend(again);
        }
        RunPass::Backtrack | RunPass::Repeat => {
            let back: Vec<Point> = pts.iter().rev().skip(1).copied().collect();
            pts.extend(back);
        }
        RunPass::Once | RunPass::Redwork | RunPass::Branch => {}
    }
    pts
}

/// Index of the point nearest to `p` (the first one on a tie).
fn nearest(pts: &[Point], p: Point) -> usize {
    let mut best = 0;
    let mut best_d = f64::INFINITY;
    for (i, q) in pts.iter().enumerate() {
        let d = q.distance(p);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

/// How a run joins the run before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Join {
    /// By the trim mode: travel stitch, trim, or a jump with no trim.
    Trim(TrimMode),
    /// Always a jump with no trim (stitches from a machine file).
    Jump,
    /// Always a stitch to the next run (it is covered later).
    Travel,
}

struct Builder<'a> {
    opts: &'a PlanOptions,
    recs: Vec<StitchRecord>,
    needle: Point,
    /// The thread is in the fabric (not trimmed).
    sewing: bool,
    /// The needle position before the last stitch, for the tie-off direction.
    prev: Option<Point>,
    /// The object that sews now wants lock stitches before a trim.
    tie_off_enabled: bool,
    /// A stop waits for the start of the next run.
    pending_stop: Option<(ObjectId, StopReason)>,
    /// Auto jump for the run being sewn: long stitches become jumps.
    long_jumps: bool,
    stops: Vec<PlanStop>,
    /// The machine sews slowly now (after a `Slow` record).
    slow: bool,
    /// The object that sews next wants the slow speed (the `slow`
    /// function); the speed changes before its first stitch.
    want_slow: bool,
}

impl<'a> Builder<'a> {
    fn new(opts: &'a PlanOptions) -> Self {
        Self {
            opts,
            recs: Vec::new(),
            needle: Point::ZERO,
            sewing: false,
            prev: None,
            tie_off_enabled: true,
            pending_stop: None,
            stops: Vec::new(),
            long_jumps: false,
            slow: false,
            want_slow: false,
        }
    }

    fn push(&mut self, kind: StitchKind, p: Point) {
        self.recs.push(StitchRecord::new(p, kind));
    }

    fn jump(&mut self, p: Point) {
        if p != self.needle {
            self.push(StitchKind::Jump, p);
            self.needle = p;
        }
    }

    /// A needle penetration at `p`. `force` keeps it even if it is shorter
    /// than the minimum (lock stitches, travel stitches, the first stitch).
    fn stitch(&mut self, p: Point, force: bool) {
        let d = self.needle.distance(p);
        let first_after_jump = !matches!(self.recs.last(), Some(r) if r.kind == StitchKind::Stitch);
        if d < 1e-9 && !first_after_jump {
            return;
        }
        if d < self.opts.min_stitch_mm && !force && !first_after_jump {
            return;
        }
        if self.want_slow != self.slow {
            self.slow = self.want_slow;
            let kind = if self.slow { StitchKind::Slow } else { StitchKind::Fast };
            self.push(kind, self.needle);
        }
        let max = self.opts.max_stitch_mm.max(0.1);
        if d > max {
            let n = (d / max).ceil() as usize;
            let from = self.needle;
            // Auto jump: the frame moves with no needle point, so the
            // thread lies long; else needle points split the stitch.
            let kind = if self.long_jumps {
                StitchKind::Jump
            } else {
                StitchKind::Stitch
            };
            for k in 1..n {
                self.push(kind, from.lerp(p, k as f64 / n as f64));
            }
        }
        self.push(StitchKind::Stitch, p);
        self.prev = Some(self.needle);
        self.needle = p;
    }

    /// Two short stitches forward and back toward `toward`, ending where they started.
    fn lock(&mut self, toward: Point) {
        let at = self.needle;
        let Some(u) = (toward - at).normalized() else { return };
        let len = self.opts.lock_mm.min(at.distance(toward) / 2.0);
        // Shorter lock stitches hold nothing, and a machine file (0.1 mm
        // steps) puts them all on one point.
        if len < MIN_LOCK_MM {
            return;
        }
        let q = at + u * len;
        for p in [q, at, q, at] {
            self.stitch(p, true);
        }
    }

    fn tie_off(&mut self) {
        if !self.tie_off_enabled {
            return;
        }
        if let Some(prev) = self.prev {
            // The lock stitches belong to the object that ends: its speed.
            let want = std::mem::replace(&mut self.want_slow, self.slow);
            self.lock(prev);
            self.want_slow = want;
        }
    }

    fn trim(&mut self) {
        self.push(StitchKind::Trim, self.needle);
        self.sewing = false;
        self.prev = None;
    }

    /// Writes the stop that waits, if any.
    /// A stop before the first stitch of the design is dropped: the
    /// operator does that work before the start.
    /// Right after a color change the machine stops anyway: the stop is
    /// that color change (no second stop), with its reason.
    fn stop_here(&mut self) {
        if let Some((object, reason)) = self.pending_stop.take()
            && self.recs.iter().any(|r| r.kind == StitchKind::Stitch)
        {
            let at_change = self
                .recs
                .iter()
                .rposition(|r| matches!(r.kind, StitchKind::Stitch | StitchKind::ColorChange))
                .filter(|&i| self.recs[i].kind == StitchKind::ColorChange);
            match at_change {
                Some(i) => self.stops.push(PlanStop {
                    record: i,
                    object,
                    reason,
                }),
                None => {
                    self.stops.push(PlanStop {
                        record: self.recs.len(),
                        object,
                        reason,
                    });
                    self.push(StitchKind::Stop, self.needle);
                }
            }
        }
    }

    fn color_change(&mut self) {
        self.push(StitchKind::ColorChange, self.needle);
    }

    /// Sews one run. `join` says how to go from the last run to this one;
    /// `tie_in` adds lock stitches when the run starts after a trim; `exact`
    /// keeps stitches shorter than the minimum.
    fn sew_run(&mut self, run: &[Point], join: Join, tie_in: bool, exact: bool) {
        let Some((&first, rest)) = run.split_first() else {
            return;
        };
        let mode = match join {
            Join::Trim(m) => m,
            Join::Jump | Join::Travel => TrimMode::Never,
        };
        if self.sewing && join == Join::Travel {
            self.stop_here();
            self.stitch(first, true);
        } else if self.sewing && join == Join::Jump {
            self.stop_here();
            self.jump(first);
            self.stitch(first, true);
        } else if self.sewing {
            let near = self.needle.distance(first) <= self.opts.trim_mm;
            match mode {
                TrimMode::Always => {
                    self.tie_off();
                    self.trim();
                    self.stop_here();
                }
                TrimMode::Auto if !near => {
                    self.tie_off();
                    self.trim();
                    self.stop_here();
                }
                // Travel stitch to the next run of the same color.
                TrimMode::Auto | TrimMode::Never if near => {
                    self.stop_here();
                    self.stitch(first, true);
                }
                // A long gap with no trim: the thread stays and the needle jumps.
                TrimMode::Never | TrimMode::Auto => {
                    self.stop_here();
                    self.jump(first);
                    self.stitch(first, true);
                }
            }
        }
        if !self.sewing {
            self.stop_here();
            self.jump(first);
            self.stitch(first, true);
            // A point nearer than two lock lengths gives lock stitches that
            // hold nothing: lock toward the next point past it.
            if let Some(&next) = rest
                .iter()
                .find(|p| p.distance(first) >= 2.0 * MIN_LOCK_MM)
                .or(rest.first())
                && tie_in
            {
                self.lock(next);
            }
            self.sewing = true;
        }
        for &p in rest {
            self.stitch(p, exact);
        }
    }

    fn finish_block(&mut self) {
        if self.sewing {
            self.tie_off();
            self.trim();
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{FillParams, HoopSelection, ObjectKind, Rgb, Rotation, RunningParams};

    use super::*;

    fn kinds(p: &StitchPlan) -> Vec<StitchKind> {
        p.stitches.records.iter().map(|r| r.kind).collect()
    }

    fn count(p: &StitchPlan, k: StitchKind) -> usize {
        p.stitches.records.iter().filter(|r| r.kind == k).count()
    }

    /// The slow function: a `Slow` record before the first stitch of the
    /// object (after the jump to it), and `Fast` before the next object
    /// that is not slow. The lock stitches of an object have its speed.
    #[test]
    fn slow_objects_get_speed_records() {
        let mut d = Design::new("speed");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let line = |y: f64| ObjectKind::Running {
            path: Path::new().move_to(Point::new(0.0, y)).line_to(Point::new(30.0, y)),
            params: RunningParams::default(),
        };
        let a = d.add_object(t, line(0.0));
        let b = d.add_object(t, line(20.0));
        let c = d.add_object(t, line(40.0));
        d.object_mut(b).unwrap().functions.slow = true;
        let p = build_plan(&d, &PlanOptions::default(), &mut StitchCache::default());
        let k = kinds(&p);
        assert_eq!(count(&p, StitchKind::Slow), 1);
        assert_eq!(count(&p, StitchKind::Fast), 1);
        let slow = k.iter().position(|x| *x == StitchKind::Slow).unwrap();
        let fast = k.iter().position(|x| *x == StitchKind::Fast).unwrap();
        assert_eq!(k[slow - 1], StitchKind::Jump, "after the jump to object b");
        assert_eq!(k[slow + 1], StitchKind::Stitch);
        // The trims of b come before Fast (b's lock stitches are slow).
        assert!(k[slow..fast].contains(&StitchKind::Trim), "{k:?}");
        let spans: Vec<_> = p.spans.iter().map(|s| (s.object, s.records.clone())).collect();
        let sb = &spans.iter().find(|s| s.0 == b).unwrap().1;
        assert!(sb.start > slow, "the span of b starts after the speed record");
        let _ = (a, c);
        // No slow object: no speed records, so the other designs do not change.
        d.object_mut(b).unwrap().functions.slow = false;
        let p = build_plan(&d, &PlanOptions::default(), &mut StitchCache::default());
        assert!(!kinds(&p).iter().any(|x| x.is_speed()));
    }

    fn add_line(d: &mut Design, t: emb_model::ThreadId, a: (f64, f64), b: (f64, f64), len: f64) -> ObjectId {
        d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(Point::new(a.0, a.1)).line_to(Point::new(b.0, b.1)),
                params: RunningParams {
                    stitch_length_mm: len,
                    ..RunningParams::default()
                },
            },
        )
    }

    fn plan(d: &Design) -> StitchPlan {
        build_plan(d, &PlanOptions::default(), &mut StitchCache::default())
    }

    #[test]
    fn single_line_structure() {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, red, (5.0, 0.0), (15.0, 0.0), 2.5);
        let p = plan(&d);
        let k = kinds(&p);
        assert_eq!(k.first(), Some(&StitchKind::Jump));
        assert_eq!(&k[k.len() - 2..], &[StitchKind::Trim, StitchKind::End]);
        assert_eq!(p.stitches.threads.len(), 1);
        assert_eq!(count(&p, StitchKind::ColorChange), 0);
        // 5 line points + 4 tie-in + 4 tie-off.
        assert_eq!(count(&p, StitchKind::Stitch), 13);
        // Tie-in stitches are 0.5 mm.
        let s: Vec<Point> = p
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .collect();
        assert_eq!(s[0], Point::new(5.0, 0.0));
        assert!((s[1].x - 5.5).abs() < 1e-12);
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn vectors_sew_nothing_and_do_not_split_a_color() {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(255, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 255));
        add_line(&mut d, red, (0.0, 0.0), (10.0, 0.0), 2.5);
        let before = count(&plan(&d), StitchKind::Stitch);
        d.add_object(
            blue,
            ObjectKind::Vector {
                path: emb_geom::Path::rect(emb_geom::BBox::new(Point::new(0.0, 5.0), Point::new(9.0, 9.0))),
                style: emb_model::VectorStyle::filled(Rgb(0, 0, 255)),
            },
        );
        add_line(&mut d, red, (0.0, 1.0), (10.0, 1.0), 2.5);
        let p = plan(&d);
        assert_eq!(
            count(&p, StitchKind::ColorChange),
            0,
            "red, (artwork), red is one color"
        );
        assert_eq!(p.stitches.threads.len(), 1);
        assert!(count(&p, StitchKind::Stitch) > before);
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    }

    #[test]
    fn tie_in_locks_toward_a_point_far_enough() {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(255, 0, 0));
        d.add_object(
            red,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(0.0, 0.0))
                    .line_to(Point::new(0.05, 0.0))
                    .line_to(Point::new(0.05, 10.0)),
                params: RunningParams::default(),
            },
        );
        let p = plan(&d);
        let s: Vec<Point> = p
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .collect();
        // The lock stitches go 0.5 mm toward (0.05, 10), not 0.025 mm
        // toward (0.05, 0), which a machine file would put in one hole.
        assert!(s[1].distance(s[0]) > 0.4, "{:?}", &s[..4]);
    }

    #[test]
    fn small_gap_travels_large_gap_trims() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (0.0, 0.0), (10.0, 0.0), 2.5);
        add_line(&mut d, t, (11.0, 0.0), (20.0, 0.0), 2.5); // 1 mm gap
        add_line(&mut d, t, (40.0, 0.0), (50.0, 0.0), 2.5); // 20 mm gap
        let p = plan(&d);
        assert_eq!(
            count(&p, StitchKind::Trim),
            2,
            "one between lines 2 and 3, one at the end"
        );
    }

    /// Three lines of one color: a 1 mm gap, then a 20 mm gap.
    fn three_lines() -> (Design, [ObjectId; 3]) {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        let a = add_line(&mut d, t, (0.0, 0.0), (10.0, 0.0), 2.5);
        let b = add_line(&mut d, t, (11.0, 0.0), (20.0, 0.0), 2.5);
        let c = add_line(&mut d, t, (40.0, 0.0), (50.0, 0.0), 2.5);
        (d, [a, b, c])
    }

    #[test]
    fn auto_start_and_end_and_a_turned_hoop() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (10.0, 0.0), (20.0, 0.0), 2.0);
        d.start_end.start = Some(emb_model::Anchor::Point {
            point: Point::new(30.0, 0.0),
        });
        d.start_end.end = Some(emb_model::Anchor::HoopCenter);
        let mut cache = StitchCache::default();
        let plan = build_plan(&d, &PlanOptions::default(), &mut cache);
        let r = &plan.stitches.records;
        assert_eq!((r[0].kind, r[0].pos), (StitchKind::Jump, Point::new(30.0, 0.0)));
        // The line starts at its end near the start point.
        let first = r.iter().find(|x| x.kind == StitchKind::Stitch).unwrap();
        assert_eq!(first.pos, Point::new(20.0, 0.0));
        let n = r.len();
        assert_eq!((r[n - 2].kind, r[n - 2].pos), (StitchKind::Jump, Point::ZERO));
        assert_eq!(r[n - 3].kind, StitchKind::Trim);

        // A hoop turned 90° clockwise: the line right of the center is above it in the file.
        d.start_end = Default::default();
        d.hoop.angle_deg = 90.0;
        let plan = build_plan(&d, &PlanOptions::default(), &mut cache);
        let ys: Vec<f64> = plan
            .stitches
            .records
            .iter()
            .filter(|x| x.kind == StitchKind::Stitch)
            .map(|x| x.pos.y)
            .collect();
        assert!(ys.iter().all(|y| (-20.0 - 1e-9..=-10.0 + 1e-9).contains(y)), "{ys:?}");
    }

    #[test]
    fn trim_always_and_never() {
        let (mut d, [a, b, _]) = three_lines();
        d.object_mut(a).unwrap().functions.trim = TrimMode::Always;
        d.object_mut(b).unwrap().functions.trim = TrimMode::Never;
        let p = plan(&d);
        assert_eq!(count(&p, StitchKind::Trim), 2, "after line 1 (always) and at the end");
        // Line 2 to line 3 is a jump with the thread still in the fabric.
        let k = kinds(&p);
        let jumps: Vec<usize> = (0..k.len()).filter(|&i| k[i] == StitchKind::Jump).collect();
        // Line 1 starts at the origin, where the needle is, so it needs no jump.
        assert_eq!(jumps.len(), 2, "after the trim, and the 20 mm gap");
        assert_eq!(k[jumps[1] - 1], StitchKind::Stitch, "no trim before the last jump");
        assert_eq!(p.stitches.records[jumps[1]].pos, Point::new(40.0, 0.0));
    }

    #[test]
    fn no_lock_stitches_when_off() {
        let (d, _) = three_lines();
        let with = count(&plan(&d), StitchKind::Stitch);
        let mut d2 = d.clone();
        for id in d.objects().iter().map(|o| o.id).collect::<Vec<_>>() {
            let f = &mut d2.object_mut(id).unwrap().functions;
            f.tie_in = false;
            f.tie_off = false;
        }
        // Two trims, so two tie-ins and two tie-offs of four stitches each.
        assert_eq!(with - count(&plan(&d2), StitchKind::Stitch), 16);
    }

    #[test]
    fn stop_after_an_object() {
        let (mut d, [a, _, c]) = three_lines();
        d.object_mut(a).unwrap().functions.stop = true;
        // The last object: the end follows, so no stop.
        d.object_mut(c).unwrap().functions.stop = true;
        let p = plan(&d);
        assert_eq!(count(&p, StitchKind::Stop), 1);
        let i = p
            .stitches
            .records
            .iter()
            .position(|r| r.kind == StitchKind::Stop)
            .unwrap();
        assert_eq!(p.stitches.records[i].pos, Point::new(10.0, 0.0), "where line 1 ends");
        // The stop does not cut the thread: line 2 follows with a travel stitch.
        assert_eq!(p.stitches.records[i + 1].kind, StitchKind::Stitch);
        assert!(p.spans[0].records.end <= i);

        // With a trim after the object, the stop comes after the trim.
        d.object_mut(a).unwrap().functions.trim = TrimMode::Always;
        let p = plan(&d);
        let k = kinds(&p);
        let i = k.iter().position(|x| *x == StitchKind::Stop).unwrap();
        assert_eq!(k[i - 1], StitchKind::Trim);
        assert_eq!(k[i + 1], StitchKind::Jump);
    }

    #[test]
    fn applique_sews_in_order_with_stops() {
        use emb_model::{AppliqueParams, AppliquePiece};
        let mut d = Design::new("t");
        let t = d.add_thread("navy", Rgb(0, 0, 80));
        let square = Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 30.0)));
        let id = d.add_object(
            t,
            ObjectKind::Applique {
                pieces: vec![AppliquePiece::new(square, Vec::new())],
                params: AppliqueParams::default(),
            },
        );
        let p = plan(&d);
        assert_eq!(count(&p, StitchKind::Stop), 2);
        let reasons: Vec<StopReason> = p.stops.iter().map(|s| s.reason).collect();
        assert_eq!(reasons, [StopReason::PlaceFabric, StopReason::TrimFabric]);
        assert_eq!(
            p.stop_notes(&d),
            [
                "Color 1, object #1 Appliqué: Put the appliqué fabric on",
                "Color 1, object #1 Appliqué: Cut away the appliqué fabric outside the tack-down"
            ]
        );
        assert!(p.stops.iter().all(|s| s.object == id));
        for s in &p.stops {
            assert_eq!(p.stitches.records[s.record].kind, StitchKind::Stop);
        }
        // Stitches before the first stop (placement), between (tack-down) and after (cover).
        let (a, b) = (p.stops[0].record, p.stops[1].record);
        let stitches = |r: std::ops::Range<usize>| {
            p.stitches.records[r]
                .iter()
                .filter(|x| x.kind == StitchKind::Stitch)
                .count()
        };
        assert!(stitches(0..a) > 30 && stitches(a..b) > 30 && stitches(b..p.stitches.records.len()) > 100);
        // The thread is not cut at the stops.
        assert_eq!(count(&p, StitchKind::Trim), 1);
        // With no placement line and the object first, the fabric goes on before the start: no stop.
        if let ObjectKind::Applique { params, .. } = &mut d.object_mut(id).unwrap().kind {
            params.placement = false;
        }
        let p = plan(&d);
        assert_eq!(p.stops.len(), 1);
        assert_eq!(p.stops[0].reason, StopReason::TrimFabric);
    }

    #[test]
    fn routing_starts_at_the_nearer_end() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (0.0, 0.0), (10.0, 0.0), 2.5);
        // Drawn from far to near: the plan reverses it.
        add_line(&mut d, t, (30.0, 0.0), (10.5, 0.0), 2.5);
        let p = plan(&d);
        assert_eq!(
            count(&p, StitchKind::Trim),
            1,
            "0.5 mm gap after reversing: travel, no trim"
        );
    }

    #[test]
    fn cut_lines_trim_between_their_parts_and_have_no_locks() {
        let mut d = Design::new("cut");
        let t = d.add_thread("Black", Rgb(0, 0, 0));
        let square = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(20.0, 20.0))),
                params: RunningParams::default(),
            },
        );
        let new = d.make_cutwork(&[square], emb_model::cutwork::CutBrand::Bernina, 1.0);
        assert_eq!(new.len(), 2, "upright and level sides");
        let p = plan(&d);
        assert_eq!(p.stitches.color_count(), 2);
        // Each stitch is a cut of at most the penetration spacing: the
        // needle lifts (a trim) from one side to the next, with no travel
        // and no lock.
        let r = &p.stitches.records;
        for w in r.windows(2) {
            if w[0].kind == StitchKind::Stitch && w[1].kind == StitchKind::Stitch {
                let len = w[0].pos.distance(w[1].pos);
                assert!(len <= 1.0 + 1e-6, "{len} mm");
                assert!(len > 0.0 || w[0].pos == w[1].pos);
            }
        }
        assert!(r.iter().filter(|x| x.kind == StitchKind::Trim).count() >= 2);
    }

    #[test]
    fn puff_objects_stop_once_for_the_foam() {
        let column = |d: &mut Design, t, y: f64, puff: bool| {
            d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: emb_model::SatinGeometry::from_centerline(
                        &Path::new().move_to(Point::new(0.0, y)).line_to(Point::new(20.0, y)),
                        4.0,
                    ),
                    params: emb_model::SatinParams {
                        puff,
                        ..Default::default()
                    },
                },
            )
        };
        let mut d = Design::new("puff");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        column(&mut d, red, 0.0, false);
        let a = column(&mut d, red, 10.0, true);
        column(&mut d, red, 20.0, true);
        let p = plan(&d);
        // One stop, before the first puff column; the second needs none.
        assert_eq!(p.stops.len(), 1);
        assert_eq!((p.stops[0].object, p.stops[0].reason), (a, StopReason::PlaceFoam));
        assert_eq!(p.stitches.records[p.stops[0].record].kind, StitchKind::Stop);
        assert!(p.stop_notes(&d)[0].contains("foam"));
        // After a color change the machine stops anyway: no second stop.
        let mut d = Design::new("puff");
        column(&mut d, red, 0.0, false);
        let b = column(&mut d, blue, 10.0, true);
        let p = plan(&d);
        assert_eq!(p.stops.len(), 1);
        assert_eq!(p.stops[0].object, b);
        assert_eq!(p.stitches.records[p.stops[0].record].kind, StitchKind::ColorChange);
        assert_eq!(
            p.stitches.records.iter().filter(|r| r.kind == StitchKind::Stop).count(),
            0
        );
        assert!(p.stop_notes(&d)[0].starts_with("Color 2"), "{:?}", p.stop_notes(&d));
    }

    #[test]
    fn color_blocks_make_color_changes_and_threads() {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(255, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 255));
        add_line(&mut d, red, (0.0, 0.0), (10.0, 0.0), 2.5);
        add_line(&mut d, blue, (0.0, 5.0), (10.0, 5.0), 2.5);
        add_line(&mut d, red, (0.0, 10.0), (10.0, 10.0), 2.5);
        let p = plan(&d);
        assert_eq!(count(&p, StitchKind::ColorChange), 2);
        let names: Vec<&str> = p.stitches.threads.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["Red", "Blue", "Red"]);
        // A trim comes before each color change.
        let k = kinds(&p);
        for (i, kind) in k.iter().enumerate() {
            if *kind == StitchKind::ColorChange {
                assert_eq!(k[i - 1], StitchKind::Trim);
            }
        }
    }

    #[test]
    fn long_stitches_are_split() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (0.0, 0.0), (30.0, 0.0), 30.0);
        let opts = PlanOptions {
            max_stitch_mm: 7.0,
            ..PlanOptions::default()
        };
        let p = build_plan(&d, &opts, &mut StitchCache::default());
        let s: Vec<Point> = p
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .collect();
        assert!(s.windows(2).all(|w| w[0].distance(w[1]) <= 7.0 + 1e-9));
    }

    #[test]
    fn origin_is_hoop_center_and_rotation_applies() {
        let mut d = Design::new("t");
        d.hoop.selection = HoopSelection::Preset {
            id: "small_100x100".into(),
        };
        d.hoop.center = Point::new(100.0, 100.0);
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (110.0, 100.0), (120.0, 100.0), 10.0);
        let p = plan(&d);
        assert_eq!(p.stitches.records[0].pos, Point::new(10.0, 0.0));
        d.rotation = Rotation::Deg90;
        let p = plan(&d);
        // A quarter turn clockwise on screen: +X goes to +Y.
        assert_eq!(p.stitches.records[0].pos, Point::new(0.0, 10.0));
    }

    #[test]
    fn closed_loop_starts_near_needle_and_returns() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        d.add_object(
            t,
            ObjectKind::Running {
                path: Path::rect(emb_geom::BBox::new(Point::new(-10.0, -10.0), Point::new(10.0, 10.0))),
                params: RunningParams::default(),
            },
        );
        let p = plan(&d);
        let s: Vec<Point> = p
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .collect();
        // From the origin, the mid-side stitches (10 mm away) are nearer than the
        // corners (14 mm). Four are equally near; the first in path order wins.
        assert_eq!(s[0], Point::new(0.0, -10.0));
        // The loop closes: the last line point (before the tie-off) is the start.
        assert_eq!(s[s.len() - 5], s[0]);
    }

    #[test]
    fn start_point_overrides_routing() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        let id = add_line(&mut d, t, (0.0, 0.0), (10.0, 0.0), 2.5);
        d.object_mut(id).unwrap().start = Some(Point::new(10.0, 0.0));
        let p = plan(&d);
        assert_eq!(p.stitches.records[0].pos, Point::new(10.0, 0.0));
    }

    #[test]
    fn fill_and_satin_are_stitched_in_one_piece() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (-30.0, -30.0), (-20.0, -30.0), 2.5);
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::ellipse(Point::ZERO, 10.0, 10.0),
                holes: vec![],
                params: FillParams::default(),
            },
        );
        d.add_object(
            t,
            ObjectKind::Satin {
                geometry: emb_model::SatinGeometry {
                    rail_a: Path::new()
                        .move_to(Point::new(20.0, 0.0))
                        .line_to(Point::new(20.0, 30.0)),
                    rail_b: Path::new()
                        .move_to(Point::new(24.0, 0.0))
                        .line_to(Point::new(24.0, 30.0)),
                    rungs: vec![],
                },
                params: emb_model::SatinParams::default(),
            },
        );
        let p = plan(&d);
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        // Line, fill and satin are separate (gaps > 2 mm): three pieces, three trims.
        assert_eq!(count(&p, StitchKind::Trim), 3);
        // The fill starts on its side nearest to the end of the line.
        let recs = &p.stitches.records;
        let first_trim = recs.iter().position(|r| r.kind == StitchKind::Trim).unwrap();
        let fill_start = recs[first_trim + 1].pos;
        assert!(fill_start.x < 0.0 && fill_start.y < 0.0, "{fill_start:?}");
        // All fill stitches are in the circle (a Bézier circle is up to 0.03 % larger).
        let second_trim = first_trim
            + 1
            + recs[first_trim + 1..]
                .iter()
                .position(|r| r.kind == StitchKind::Trim)
                .unwrap();
        for r in &recs[first_trim + 1..second_trim] {
            assert!(r.pos.length() <= 10.0 * 1.0003, "{:?}", r.pos);
        }
        // 50 rows of about 6 stitches, plus the underlay.
        assert!(second_trim - first_trim > 350, "{}", second_trim - first_trim);
    }

    #[test]
    fn backtrack_ends_at_the_start_and_repeat_loops_twice() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        let id = add_line(&mut d, t, (0.0, 0.0), (10.0, 0.0), 2.5);
        if let Some(ObjectKind::Running { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
            params.pass = RunPass::Backtrack;
        }
        let p = plan(&d);
        let span = p.spans[0].clone();
        assert_eq!(span.start, span.end, "a backtracked line ends where it starts");
        // 4 stitches out and 4 back, plus the first stitch and the lock stitches.
        let stitches = count(&p, StitchKind::Stitch);
        let once = {
            let mut d1 = d.clone();
            if let Some(ObjectKind::Running { params, .. }) = d1.object_mut(id).map(|o| &mut o.kind) {
                params.pass = RunPass::Once;
            }
            count(&plan(&d1), StitchKind::Stitch)
        };
        assert_eq!(stitches, once + 4);
    }

    #[test]
    fn deterministic_and_cache_independent() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        add_line(&mut d, t, (0.0, 0.0), (17.3, 4.1), 2.7);
        add_line(&mut d, t, (25.0, 3.0), (2.0, 30.0), 3.1);
        let mut cache = StitchCache::default();
        let a = build_plan(&d, &PlanOptions::default(), &mut cache);
        let b = build_plan(&d, &PlanOptions::default(), &mut cache);
        let c = plan(&d);
        assert_eq!(a.stitches, b.stitches);
        assert_eq!(a.stitches, c.stitches);
    }

    #[test]
    fn empty_design_is_just_end() {
        let p = plan(&Design::new("t"));
        assert_eq!(kinds(&p), vec![StitchKind::End]);
    }
}
