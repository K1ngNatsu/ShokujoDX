//! Satin columns: zigzag stitches between two rails, with underlay.
//!
//! Not a port. The method follows the public Ink/Stitch documentation for
//! satin columns (rails and rungs) and common digitizing practice:
//!
//! - Rungs cut the rails into sections. In a section, a point at fraction
//!   `f` of rail A pairs with the point at the same fraction of rail B. The
//!   start and the end of the rails are rungs too.
//! - Spacing is measured along the middle of the column: the mean length
//!   of the two rail parts. It is the distance between two needle points on
//!   the same side, so the thread crosses the column every half spacing.
//! - Pull compensation makes each stitch longer at both ends.
//! - Short stitches: where needle points on one side are nearer than
//!   [`SHORT_DIST_MM`] (the inside of a tight curve), every second stitch on
//!   that side ends [`SHORT_INSET`] of the width before the rail.
//! - Auto split: a stitch longer than `split_mm` (default 7 mm) gets needle points
//!   along it. Their positions change from stitch to stitch, so they do not
//!   make a line.
//! - Underlay layers each go from the start of the column to the end and
//!   back, so the top stitching always starts at the start and ends at the end.

use emb_geom::{Path, Point, Polyline};
use emb_model::{SatinGeometry, SatinParams, Underlay, UnderlayLayer};

use crate::object::FLATTEN_TOL_MM;
use crate::running;

/// Needle points on one side nearer than this make short stitches.
pub const SHORT_DIST_MM: f64 = 0.25;
/// A short stitch ends this fraction of the column width before the rail.
pub const SHORT_INSET: f64 = 0.15;
/// The underlay of a column of a letter stops this far before another
/// part of the letter, in mm: the top stitches of that part go past its
/// outline (pull compensation), and a joint is not a sharp line (ADR 0163).
pub const JOINT_MARGIN_MM: f64 = 1.0;
/// Smallest split length that the generator accepts, in mm.
pub const MIN_SPLIT_MM: f64 = 1.0;
/// The liquid effect samples the rails this often, in mm.
const LIQUID_STEP_MM: f64 = 0.5;
/// Seed of the jagged edge of satin.
const JAGGED_SEED: u64 = 0x4A47;
/// Stitch length of center walk and edge walk underlay.
pub const WALK_LEN_MM: f64 = 2.0;
/// `Auto` underlay: columns wider than this get edge walk and zigzag, narrower ones a center walk.
pub const AUTO_WIDE_MM: f64 = 2.0;
/// `Auto` underlay: columns narrower than this get no underlay.
pub const AUTO_MIN_MM: f64 = 1.0;
const AUTO_INSET_MM: f64 = 0.4;
/// [`Column::untangled_rungs`] makes at most this number of pairs between
/// two rungs.
const UNTANGLE_MAX_PAIRS: f64 = 20_000.0;
const AUTO_ZIGZAG_SPACING_MM: f64 = 2.0;
const HEAVY_ZIGZAG_SPACING_MM: f64 = 1.2;

/// A rail flattened to a polyline, with the length along it at each point.
struct Rail {
    pts: Vec<Point>,
    at_len: Vec<f64>,
}

impl Rail {
    fn new(path: &Path) -> Option<Rail> {
        let mut pts: Vec<Point> = Vec::new();
        for pl in path.flatten(FLATTEN_TOL_MM) {
            // A closed rail (a ring) ends where it starts.
            let close = pl.closed.then(|| pl.points.first().copied()).flatten();
            for p in pl.points.into_iter().chain(close) {
                if pts.last() != Some(&p) {
                    pts.push(p);
                }
            }
        }
        if pts.len() < 2 {
            return None;
        }
        let mut at_len = vec![0.0];
        for w in pts.windows(2) {
            at_len.push(at_len[at_len.len() - 1] + w[0].distance(w[1]));
        }
        Some(Rail { pts, at_len })
    }

    fn len(&self) -> f64 {
        self.at_len[self.at_len.len() - 1]
    }

    fn at(&self, s: f64) -> Point {
        let s = s.clamp(0.0, self.len());
        let i = self.at_len.partition_point(|&l| l <= s).clamp(1, self.pts.len() - 1);
        let seg = self.at_len[i] - self.at_len[i - 1];
        let t = if seg > 0.0 { (s - self.at_len[i - 1]) / seg } else { 0.0 };
        self.pts[i - 1].lerp(self.pts[i], t)
    }

    /// Length along the rail of the rail point nearest to `p`.
    fn project(&self, p: Point) -> f64 {
        self.project_near(p, None)
    }

    /// [`Rail::project`]; where the rail passes `p` more than one time (as
    /// near), the place nearest to the part `near` (0 to 1) of its length.
    fn project_near(&self, p: Point, near: Option<f64>) -> f64 {
        let places: Vec<(f64, f64)> = self
            .pts
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let d = w[1] - w[0];
                let l2 = d.length_sq();
                let t = if l2 > 0.0 {
                    ((p - w[0]).dot(d) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (
                    p.distance(w[0].lerp(w[1], t)),
                    self.at_len[i] + t * (self.at_len[i + 1] - self.at_len[i]),
                )
            })
            .collect();
        let best = places.iter().map(|x| x.0).fold(f64::INFINITY, f64::min);
        let tie = best + 1e-9 * self.len().max(1.0);
        let len = self.len().max(1e-12);
        places
            .iter()
            .filter(|x| x.0 <= tie)
            .min_by(|x, y| match near {
                Some(f) => (x.1 / len - f).abs().total_cmp(&(y.1 / len - f).abs()),
                None => x.1.total_cmp(&y.1),
            })
            .map_or(0.0, |x| x.1)
    }

    /// The places of the two points of a rung on rails `a` and `b`. A rail
    /// that goes over the same points two times (a stroke of a letter that
    /// goes round and ends on itself) has two places for a point: the
    /// place in the part of the column of the other point is taken.
    fn rung(a: &Rail, b: &Rail, pa: Point, pb: Point) -> (f64, f64) {
        let sa = a.project(pa);
        let sb = b.project_near(pb, Some(sa / a.len().max(1e-12)));
        let sa = a.project_near(pa, Some(sb / b.len().max(1e-12)));
        (sa, sb)
    }
}

/// The column: rails and the rung positions on each rail.
pub struct Column {
    a: Rail,
    b: Rail,
    /// Pairs of lengths along rail A and rail B, increasing in both.
    rungs: Vec<(f64, f64)>,
    /// Pairs are spaced evenly over the whole column, not section by section
    /// (the dense rungs of the liquid effect).
    even: bool,
    /// With `even`: the distance between two rungs is measured across the
    /// stitches (from the middle of one rung to the line of the other), not
    /// along the rails. For the sections of a column fill, whose rungs are
    /// parallel: an edge almost along the stitches then does not crowd them.
    across: bool,
}

impl Column {
    pub fn new(g: &SatinGeometry) -> Option<Column> {
        let a = Rail::new(&g.rail_a)?;
        let b = Rail::new(&g.rail_b)?;
        let mut inner: Vec<(f64, f64)> = g.rungs.iter().map(|(pa, pb)| Rail::rung(&a, &b, *pa, *pb)).collect();
        inner.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut rungs = vec![(0.0, 0.0)];
        for r in inner {
            // A rung that would cross the previous one is not used.
            let last = rungs[rungs.len() - 1];
            if r.0 > last.0 + 1e-6 && r.1 > last.1 + 1e-6 && r.0 < a.len() - 1e-6 && r.1 < b.len() - 1e-6 {
                rungs.push(r);
            }
        }
        rungs.push((a.len(), b.len()));
        Some(Column {
            a,
            b,
            rungs,
            even: false,
            across: false,
        })
    }

    /// Point pairs across the column, about `step` apart along the middle,
    /// from the start to the end. The first and last pairs are the rail ends.
    pub fn pairs(&self, step: f64) -> Vec<(Point, Point)> {
        if self.even {
            return self.even_pairs(step);
        }
        let step = step.max(0.05);
        let mut out = vec![(self.a.at(0.0), self.b.at(0.0))];
        for w in self.rungs.windows(2) {
            let (la, lb) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let n = ((la + lb) / 2.0 / step).round().max(1.0) as usize;
            for q in 1..=n {
                let f = q as f64 / n as f64;
                out.push((self.a.at(w[0].0 + f * la), self.b.at(w[0].1 + f * lb)));
            }
        }
        out
    }

    /// As [`Column::pairs`], at even steps of the middle length over the
    /// whole column: not a pair at each rung. Where rungs come close (the
    /// pivots of a column that turns), the pairs do not crowd. The
    /// underlay uses these pairs (ADR 0163).
    pub fn even_pairs(&self, step: f64) -> Vec<(Point, Point)> {
        let step = step.max(0.05);
        let mut out = vec![(self.a.at(0.0), self.b.at(0.0))];
        // The middle length at each rung, then pairs at even steps of it.
        let mut mid = vec![0.0];
        for w in self.rungs.windows(2) {
            let d = if self.across {
                let (a0, b0) = (self.a.at(w[0].0), self.b.at(w[0].1));
                let m1 = self.a.at(w[1].0).midpoint(self.b.at(w[1].1));
                let r = b0 - a0;
                match r.normalized() {
                    Some(u) => u.cross(m1 - a0).abs(),
                    None => a0.distance(m1),
                }
            } else {
                ((w[1].0 - w[0].0) + (w[1].1 - w[0].1)) / 2.0
            };
            mid.push(mid[mid.len() - 1] + d);
        }
        let total = mid[mid.len() - 1];
        let n = (total / step).round().max(1.0) as usize;
        for q in 1..=n {
            let m = total * q as f64 / n as f64;
            let j = mid.partition_point(|&l| l <= m).clamp(1, mid.len() - 1);
            let seg = mid[j] - mid[j - 1];
            let f = if seg > 0.0 { (m - mid[j - 1]) / seg } else { 0.0 };
            let (r0, r1) = (self.rungs[j - 1], self.rungs[j]);
            out.push((self.a.at(r0.0 + f * (r1.0 - r0.0)), self.b.at(r0.1 + f * (r1.1 - r0.1))));
        }
        out
    }

    /// Rungs with which no pair across the column crosses the pair before
    /// it, for pairs about `step` apart: where a pair would cross the one
    /// before it (a fast turn, a slanted end), one side stays and the other
    /// goes on (a pivot: the side that stays goes on a millionth of the
    /// length of the rails, so that the rung comes after the one before
    /// it). `None` if no pair crosses. The rungs are in the order of the
    /// column, with the rail points of each.
    pub fn untangled_rungs(&self, step: f64) -> Option<Vec<(Point, Point)>> {
        // At most [`UNTANGLE_MAX_PAIRS`] pairs between two rungs: a step
        // of 0 (lettering with a height that is not more than 0) asked for
        // more memory than a computer has (the audit of P21).
        let longest = self.a.len().max(self.b.len());
        if !longest.is_finite() {
            return None;
        }
        let step = step.max(longest / UNTANGLE_MAX_PAIRS).max(1e-9);
        // The places of the pairs on the two rails.
        let mut target: Vec<(f64, f64)> = vec![(0.0, 0.0)];
        for w in self.rungs.windows(2) {
            let (la, lb) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let n = ((la + lb) / 2.0 / step).round().max(1.0) as usize;
            for q in 1..=n {
                let f = q as f64 / n as f64;
                target.push((w[0].0 + f * la, w[0].1 + f * lb));
            }
        }
        let line = |r: (f64, f64)| (self.a.at(r.0), self.b.at(r.1));
        let cross = |x: (f64, f64), y: (f64, f64)| pairs_cross(line(x), line(y));
        let tiny = 1e-6 * (self.a.len() + self.b.len()).max(1e-9);
        let mut out: Vec<(f64, f64)> = vec![target[0]];
        let mut changed = false;
        for &t in &target[1..] {
            let last = out[out.len() - 1];
            if !cross(last, t) {
                out.push(t);
                continue;
            }
            changed = true;
            // A pivot on one side: the side that went on less stays.
            let stay_b = (t.0, last.1 + tiny);
            let stay_a = (last.0 + tiny, t.1);
            let ok = |c: (f64, f64)| c.0 > last.0 && c.1 > last.1 && !cross(last, c);
            let pick = match (ok(stay_a), ok(stay_b)) {
                (true, true) => {
                    if t.0 - last.0 <= t.1 - last.1 {
                        stay_a
                    } else {
                        stay_b
                    }
                }
                (true, false) => stay_a,
                (false, true) => stay_b,
                (false, false) => continue,
            };
            out.push(pick);
        }
        if !changed {
            return None;
        }
        // The ends are the ends of the rails; the rungs between them, with
        // the places that the lines between their neighbours give dropped.
        let end = (self.a.len(), self.b.len());
        while out.len() > 1 && (out[out.len() - 1].0 >= end.0 - tiny || out[out.len() - 1].1 >= end.1 - tiny) {
            out.pop();
        }
        out.push(end);
        let mut kept: Vec<(f64, f64)> = vec![out[0]];
        for k in 1..out.len() - 1 {
            let (p, q) = (kept[kept.len() - 1], out[k + 1]);
            let along = |x: f64, x0: f64, x1: f64| {
                if (x1 - x0).abs() > 1e-12 {
                    (x - x0) / (x1 - x0)
                } else {
                    0.0
                }
            };
            let fa = along(out[k].0, p.0, q.0);
            let fb = along(out[k].1, p.1, q.1);
            if (fa - fb).abs() > 1e-3 {
                kept.push(out[k]);
            }
        }
        Some(kept[1..].iter().map(|&r| line(r)).collect())
    }

    /// Liquid effect: the pairing of the rails changes evenly. The rungs
    /// become a dense list whose rail-B lengths are a moving mean over
    /// about two column widths, so sharp changes of stitch angle spread out.
    pub fn make_liquid(&mut self) {
        if self.rungs.len() <= 2 {
            return;
        }
        let window = 2.0 * self.mean_width().max(1.0);
        let (la, lb) = (self.a.len(), self.b.len());
        let n = ((la / LIQUID_STEP_MM).ceil() as usize).max(2);
        let xs: Vec<f64> = (0..=n).map(|i| la * i as f64 / n as f64).collect();
        let map = |x: f64| {
            let i = self.rungs.partition_point(|r| r.0 <= x).clamp(1, self.rungs.len() - 1);
            let (r0, r1) = (self.rungs[i - 1], self.rungs[i]);
            let t = if r1.0 > r0.0 { (x - r0.0) / (r1.0 - r0.0) } else { 0.0 };
            r0.1 + t * (r1.1 - r0.1)
        };
        let ys: Vec<f64> = xs.iter().map(|&x| map(x)).collect();
        let half = ((window / 2.0) / (la / n as f64)).round().max(1.0) as usize;
        let mut rungs = vec![(0.0, 0.0)];
        for (i, &x) in xs.iter().enumerate().take(n).skip(1) {
            // A window of the same size on both sides, smaller near the ends.
            let h = half.min(i).min(n - i);
            let (lo, hi) = (i - h, i + h);
            let y = ys[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64;
            let last = rungs[rungs.len() - 1];
            if x > last.0 + 1e-6 && y > last.1 + 1e-6 && y < lb - 1e-6 {
                rungs.push((x, y));
            }
        }
        rungs.push((la, lb));
        self.rungs = rungs;
        self.even = true;
    }

    /// Mean distance between the rails.
    pub fn mean_width(&self) -> f64 {
        let p = self.pairs(1.0);
        p.iter().map(|(a, b)| a.distance(*b)).sum::<f64>() / p.len() as f64
    }

    /// The point pair at the start of the column.
    pub fn start(&self) -> (Point, Point) {
        (self.a.at(0.0), self.b.at(0.0))
    }

    pub fn end(&self) -> (Point, Point) {
        (self.a.at(self.a.len()), self.b.at(self.b.len()))
    }
}

/// Moves both points of each pair toward each other by `inset` (to the middle if the column is narrower).
pub(crate) fn inset_pairs(pairs: &[(Point, Point)], inset: f64) -> Vec<(Point, Point)> {
    pairs
        .iter()
        .map(|&(a, b)| {
            let w = a.distance(b);
            if w <= 2.0 * inset || w == 0.0 {
                let m = a.midpoint(b);
                (m, m)
            } else {
                let u = (b - a) * (1.0 / w);
                (a + u * inset, b - u * inset)
            }
        })
        .collect()
}

pub(crate) fn walk(points: Vec<Point>) -> Vec<Point> {
    running::running_points(&Polyline { points, closed: false }, WALK_LEN_MM)
}

/// The underlay layers that `Auto` selects for a column of this width.
pub fn auto_underlay(mean_width: f64) -> Vec<UnderlayLayer> {
    if mean_width > AUTO_WIDE_MM {
        vec![
            UnderlayLayer::EdgeWalk {
                inset_mm: AUTO_INSET_MM,
            },
            UnderlayLayer::Zigzag {
                inset_mm: AUTO_INSET_MM,
                spacing_mm: AUTO_ZIGZAG_SPACING_MM,
            },
        ]
    } else if mean_width >= AUTO_MIN_MM {
        vec![UnderlayLayer::CenterWalk]
    } else {
        Vec::new()
    }
}

/// The underlay layers that `Double` selects: two zigzag layers that cross
/// (each goes out and back), or a center walk for narrow columns.
pub fn double_underlay(mean_width: f64) -> Vec<UnderlayLayer> {
    if mean_width >= AUTO_MIN_MM {
        let inset_mm = AUTO_INSET_MM.min(mean_width * 0.25);
        vec![
            UnderlayLayer::Zigzag {
                inset_mm,
                spacing_mm: AUTO_ZIGZAG_SPACING_MM,
            },
            UnderlayLayer::Zigzag {
                inset_mm,
                spacing_mm: HEAVY_ZIGZAG_SPACING_MM,
            },
        ]
    } else {
        vec![UnderlayLayer::CenterWalk]
    }
}

/// The underlay layers that `Light` selects: a center walk for wide
/// columns only.
pub fn light_underlay(mean_width: f64) -> Vec<UnderlayLayer> {
    if mean_width > AUTO_WIDE_MM {
        vec![UnderlayLayer::CenterWalk]
    } else {
        Vec::new()
    }
}

/// The underlay layers that `Heavy` selects: one layer more than `Auto`,
/// and a denser zigzag.
pub fn heavy_underlay(mean_width: f64) -> Vec<UnderlayLayer> {
    let inset_mm = AUTO_INSET_MM.min(mean_width * 0.25);
    if mean_width > AUTO_WIDE_MM {
        vec![
            UnderlayLayer::CenterWalk,
            UnderlayLayer::EdgeWalk { inset_mm },
            UnderlayLayer::Zigzag {
                inset_mm,
                spacing_mm: HEAVY_ZIGZAG_SPACING_MM,
            },
        ]
    } else if mean_width >= AUTO_MIN_MM {
        vec![
            UnderlayLayer::CenterWalk,
            UnderlayLayer::Zigzag {
                inset_mm,
                spacing_mm: HEAVY_ZIGZAG_SPACING_MM,
            },
        ]
    } else {
        vec![UnderlayLayer::CenterWalk]
    }
}

/// The underlay layers that `Lace` selects (free-standing lace, P11
/// Milestone 5): the edge walk two times on the same needle points, so its
/// threads lock, and a dense zigzag; a narrow column gets the center walk
/// two times.
pub fn lace_underlay(mean_width: f64) -> Vec<UnderlayLayer> {
    if mean_width >= AUTO_MIN_MM {
        let inset_mm = AUTO_INSET_MM.min(mean_width * 0.25);
        vec![
            UnderlayLayer::EdgeWalk { inset_mm },
            UnderlayLayer::EdgeWalk { inset_mm },
            UnderlayLayer::Zigzag {
                inset_mm,
                spacing_mm: HEAVY_ZIGZAG_SPACING_MM,
            },
        ]
    } else {
        vec![UnderlayLayer::CenterWalk, UnderlayLayer::CenterWalk]
    }
}

/// The underlay layers of the setting `u` for a column of `mean_width`.
pub fn underlay_layers(u: &Underlay, mean_width: f64) -> Vec<UnderlayLayer> {
    match u {
        Underlay::Auto => auto_underlay(mean_width),
        Underlay::Light => light_underlay(mean_width),
        Underlay::Heavy => heavy_underlay(mean_width),
        Underlay::Double => double_underlay(mean_width),
        Underlay::Lace => lace_underlay(mean_width),
        Underlay::None => Vec::new(),
        Underlay::Custom(l) => l.clone(),
    }
}

/// Needle points of one underlay layer, from the start of the column to the end and back.
fn underlay_layer(col: &Column, layer: &UnderlayLayer) -> Vec<Point> {
    match *layer {
        UnderlayLayer::CenterWalk => {
            let mids: Vec<Point> = col.even_pairs(0.5).iter().map(|(a, b)| a.midpoint(*b)).collect();
            let mut pts = walk(mids);
            let back: Vec<Point> = pts.iter().rev().skip(1).copied().collect();
            pts.extend(back);
            pts
        }
        UnderlayLayer::EdgeWalk { inset_mm } => {
            let pairs = inset_pairs(&col.even_pairs(0.5), inset_mm);
            let mut pts = walk(pairs.iter().map(|p| p.0).collect());
            pts.extend(walk(pairs.iter().rev().map(|p| p.1).collect()));
            pts
        }
        UnderlayLayer::Zigzag { inset_mm, spacing_mm } => {
            let pairs = inset_pairs(&col.even_pairs(spacing_mm.max(0.2) / 2.0), inset_mm);
            let mut pts: Vec<Point> = pairs
                .iter()
                .enumerate()
                .map(|(j, (a, b))| if j.is_multiple_of(2) { *a } else { *b })
                .collect();
            // Back with the other phase, so the two passes cross.
            pts.extend(
                pairs
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(j, (a, b))| if (j + pairs.len()).is_multiple_of(2) { *a } else { *b }),
            );
            pts
        }
        // A tatami underlay is for fills.
        UnderlayLayer::Tatami { .. } => Vec::new(),
    }
}

/// Satin top stitching: zigzag through `pairs` (one needle point a pair, sides
/// in turn) from the first pair to the last.
pub(crate) fn satin_top(pairs: &[(Point, Point)], p: &SatinParams) -> Vec<Point> {
    let (comp_a, comp_b) = (p.pull_compensation_mm, p.pull_b_mm.unwrap_or(p.pull_compensation_mm));
    let n = pairs.len();
    // Needle point on the side of stitch j, before compensation.
    let side = |j: usize| if j.is_multiple_of(2) { pairs[j].0 } else { pairs[j].1 };
    let mut out: Vec<Point> = Vec::with_capacity(n * 2);
    for (j, &(a, b)) in pairs.iter().enumerate() {
        let (this, other) = if j.is_multiple_of(2) { (a, b) } else { (b, a) };
        let w = this.distance(other);
        let mut pt = this;
        if w > 0.0 {
            let u = (this - other) * (1.0 / w);
            let comp = if j.is_multiple_of(2) { comp_a } else { comp_b } + w * p.pull_percent / 100.0;
            pt = this + u * comp;
            // Jagged edge: the needle point moves in at random.
            if p.jagged_mm > 0.0 {
                pt -= u * (p.jagged_mm.min(w * 0.45) * crate::effects::hash01(JAGGED_SEED, j as u64));
            }
            // Short stitch: every second stitch on a side with crowded needle points.
            if p.short_stitches && j >= 2 && (j / 2) % 2 == 1 && side(j).distance(side(j - 2)) < SHORT_DIST_MM {
                pt = this - u * (SHORT_INSET * w);
            }
        }
        // Auto split of a long stitch from the previous needle point (not
        // with auto jump: the stitch stays long).
        if !p.auto_jump
            && let Some(&prev) = out.last()
        {
            let len = prev.distance(pt);
            let split = p.split_mm.max(MIN_SPLIT_MM);
            if len > split {
                let parts = (len / split).ceil() as usize;
                // Fractions measured from the side A end; they move by half
                // a part from one stitch to the next.
                let shift = if j.is_multiple_of(2) { 0.25 } else { -0.25 };
                let (from_a, to_b) = if j.is_multiple_of(2) { (pt, prev) } else { (prev, pt) };
                let mut mid: Vec<Point> = (1..parts)
                    .map(|q| from_a.lerp(to_b, (q as f64 + shift) / parts as f64))
                    .collect();
                if j.is_multiple_of(2) {
                    mid.reverse();
                }
                out.extend(mid);
            }
        }
        out.push(pt);
    }
    out
}

/// All needle points of a satin column: underlay, then top stitching.
/// The column starts at the end nearer to `entry`; if `exit` is set, it
/// starts at the end farther from `exit` instead, so it ends near `exit`.
pub fn satin(g: &SatinGeometry, p: &SatinParams, entry: Point, exit: Option<Point>) -> Option<Vec<Point>> {
    satin_with(g, p, entry, exit, false, &[])
}

/// As [`satin`], for a column of a letter with other parts (`sewn`: the
/// areas of the other satin and fill parts of the letter, each a list of
/// rings). The underlay leaves out the places where another part lies, and
/// [`JOINT_MARGIN_MM`] round them: at a joint of the letter two columns lie
/// on each other, so the fabric has two layers of satin there, and
/// underlay under them only makes the joint stiff (ADR 0163). The underlay
/// goes across such a place with a walk, under the top stitches of the
/// column.
pub(crate) fn satin_on(
    g: &SatinGeometry,
    p: &SatinParams,
    entry: Point,
    exit: Option<Point>,
    sewn: &[Vec<Vec<Point>>],
) -> Option<Vec<Point>> {
    satin_with(g, p, entry, exit, false, sewn)
}

/// A walk from the last point of `out` to `to` where the step is longer
/// than a walk stitch (an underlay with its ends left out), so no long
/// stitch goes there.
fn walk_to(out: &mut Vec<Point>, to: Option<&Point>) {
    if let (Some(&from), Some(&to)) = (out.last(), to)
        && from.distance(to) > WALK_LEN_MM
    {
        out.extend(walk(vec![from, to]).into_iter().skip(1));
        out.pop();
    }
}

/// The points of an underlay with the points in `sewn` left out: the
/// points before and after such a place are joined by a walk.
fn underlay_off(points: Vec<Point>, sewn: &[Vec<Vec<Point>>]) -> Vec<Point> {
    let near = |q: Point, r: &[Point]| {
        (0..r.len()).any(|i| q.distance_to_segment(r[i], r[(i + 1) % r.len()]) < JOINT_MARGIN_MM)
    };
    let inside = |q: Point| {
        sewn.iter().any(|rings| {
            rings.iter().filter(|r| emb_geom::region::ring_contains(r, q)).count() % 2 == 1
                || rings.iter().any(|r| near(q, r))
        })
    };
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    let mut skipped = false;
    for q in points {
        if inside(q) {
            skipped = true;
            continue;
        }
        if skipped && let Some(&last) = out.last() {
            out.extend(walk(vec![last, q]).into_iter().skip(1));
        } else {
            out.push(q);
        }
        skipped = false;
    }
    out
}

/// As [`satin`]. With `across`, the spacing is measured across the
/// stitches over the whole column (see `Column::across`); the rungs must
/// be parallel and dense (a section of a column fill).
pub(crate) fn satin_with(
    g: &SatinGeometry,
    p: &SatinParams,
    entry: Point,
    exit: Option<Point>,
    across: bool,
    sewn: &[Vec<Vec<Point>>],
) -> Option<Vec<Point>> {
    let mut col = Column::new(g)?;
    let (s, e) = (col.start(), col.end());
    let (ms, me) = (s.0.midpoint(s.1), e.0.midpoint(e.1));
    let reverse = match exit {
        Some(x) => x.distance(ms) < x.distance(me),
        None => entry.distance(me) < entry.distance(ms),
    };
    if reverse {
        let reversed = SatinGeometry {
            rail_a: reverse_path(&g.rail_a),
            rail_b: reverse_path(&g.rail_b),
            rungs: g.rungs.clone(),
        };
        col = Column::new(&reversed)?;
    }
    if p.liquid {
        col.make_liquid();
    }
    if across {
        col.even = true;
        col.across = true;
    }
    let layers = underlay_layers(&p.underlay, col.mean_width());
    let mut out: Vec<Point> = Vec::new();
    for layer in &layers {
        // Puff: no rows across the column, they press the foam flat.
        if p.puff && !matches!(layer, UnderlayLayer::CenterWalk | UnderlayLayer::EdgeWalk { .. }) {
            continue;
        }
        let points = underlay_layer(&col, layer);
        let points = if sewn.is_empty() {
            points
        } else {
            // The needle comes to the start of the column: from there a
            // walk goes to the first point that the underlay keeps.
            if out.is_empty()
                && let Some(&start) = points.first()
            {
                out.push(start);
            }
            underlay_off(points, sewn)
        };
        walk_to(&mut out, points.first());
        out.extend(points);
    }
    if p.puff {
        out.extend(puff_cap(&col, true));
    }
    // The accordion profile goes from the start of the column as drawn, also
    // when the column is sewn from its end.
    let mirrored;
    let p = match (&p.accordion, reverse) {
        (Some(a), true) => {
            let mut points: Vec<(f64, f64)> = a.points.iter().map(|&(t, f)| (1.0 - t, f)).collect();
            points.reverse();
            mirrored = SatinParams {
                accordion: Some(emb_model::Accordion { points }),
                ..p.clone()
            };
            &mirrored
        }
        _ => p,
    };
    let top = crate::column::top(&col, p);
    // An underlay with its ends left out (on another part of a letter)
    // ends away from the start of the top stitches: a walk goes there.
    walk_to(&mut out, top.first());
    // Omit last stitch: the column ends on the rail before the last one.
    let keep = if p.omit_last && p.pattern.is_satin() && top.len() > 3 {
        top.len() - 1
    } else {
        top.len()
    };
    out.extend(&top[..keep]);
    if p.puff {
        out.extend(puff_cap(&col, false));
    }
    out.dedup();
    Some(out)
}

/// How far puff end stitches go into the column, and past its end, in mm.
const PUFF_CAP_IN_MM: f64 = 1.2;
const PUFF_CAP_OUT_MM: f64 = 0.4;
/// The spacing of the puff end stitches across the column, in mm.
const PUFF_CAP_SPACING_MM: f64 = 0.3;

/// Puff end stitches at the start (or the end) of the column: stitches
/// along the column over its end, one spacing apart across it, so the
/// needle cuts the foam at the end and the end is closed.
fn puff_cap(col: &Column, start: bool) -> Vec<Point> {
    let pairs = col.pairs(0.5);
    if pairs.len() < 2 {
        return Vec::new();
    }
    // A ring (a closed column) has no ends.
    let (first, last) = (pairs[0], pairs[pairs.len() - 1]);
    if first.0.distance(last.0) < 0.5 && first.1.distance(last.1) < 0.5 {
        return Vec::new();
    }
    let (end, next) = if start {
        (pairs[0], pairs[1])
    } else {
        (pairs[pairs.len() - 1], pairs[pairs.len() - 2])
    };
    let mid = end.0.midpoint(end.1);
    let Some(inward) = (next.0.midpoint(next.1) - mid).normalized() else {
        return Vec::new();
    };
    let width = end.0.distance(end.1);
    let n = ((width / PUFF_CAP_SPACING_MM).ceil() as usize).max(2);
    // The end stitches go across from rail A to rail B, so the top stitching
    // (which starts at rail A) follows from the start cap.
    let (a, b) = if start { (end.1, end.0) } else { (end.0, end.1) };
    (0..=n)
        .map(|i| {
            let on = a.lerp(b, i as f64 / n as f64);
            if i % 2 == 0 {
                on + inward * PUFF_CAP_IN_MM
            } else {
                on - inward * PUFF_CAP_OUT_MM
            }
        })
        .collect()
}

/// `true` if two pairs across a column cross each other (not two pairs with
/// one point on a side: a pivot). The letter score, the letter fault bench,
/// the choice of a split and the untangled rungs use this one test.
pub fn pairs_cross(a: (Point, Point), b: (Point, Point)) -> bool {
    Point::crossing(a.0, a.1, b.0, b.1)
        .is_some_and(|(u, v)| (1e-3..=1.0 - 1e-3).contains(&u) && (1e-3..=1.0 - 1e-3).contains(&v))
}

/// The path from its end to its start, flattened.
fn reverse_path(path: &Path) -> Path {
    let mut pts: Vec<Point> = path
        .flatten(FLATTEN_TOL_MM)
        .into_iter()
        .flat_map(|l| l.points)
        .collect();
    pts.reverse();
    let mut it = pts.into_iter();
    let Some(first) = it.next() else { return Path::new() };
    it.fold(Path::new().move_to(first), |p, q| p.line_to(q))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step that is not more than 0 (lettering with such a height) gives
    /// a limited number of pairs (the audit of P21: a step of 0 asked for a
    /// pair at each billionth of a mm, more memory than a computer has).
    #[test]
    fn untangled_rungs_with_a_step_of_0_are_limited() {
        let line = |pts: &[(f64, f64)]| {
            let mut p = emb_geom::Path::new().move_to(Point::new(pts[0].0, pts[0].1));
            for &(x, y) in &pts[1..] {
                p = p.line_to(Point::new(x, y));
            }
            p
        };
        let g = SatinGeometry {
            rail_a: line(&[(0.0, 0.0), (10.0, 0.0)]),
            rail_b: line(&[(0.0, 2.0), (6.0, 2.0), (4.0, 3.0), (10.0, 3.0)]),
            rungs: Vec::new(),
        };
        let c = Column::new(&g).unwrap();
        for step in [0.0, -0.2, 1e-12, f64::NAN] {
            let rungs = c.untangled_rungs(step).expect("pairs that cross");
            assert!(
                rungs.len() <= UNTANGLE_MAX_PAIRS as usize + 2,
                "{step}: {}",
                rungs.len()
            );
        }
    }

    /// A rail that goes back for a bit (the inside of a fast turn): pairs
    /// across the column cross; with the untangled rungs, none does, and
    /// the column still goes from end to end.
    #[test]
    fn untangled_rungs_have_no_crossing_pairs() {
        let line = |pts: &[(f64, f64)]| {
            let mut p = emb_geom::Path::new().move_to(Point::new(pts[0].0, pts[0].1));
            for &(x, y) in &pts[1..] {
                p = p.line_to(Point::new(x, y));
            }
            p
        };
        let crossings = |c: &Column| {
            let pairs = c.pairs(0.2);
            pairs.windows(2).filter(|w| pairs_cross(w[0], w[1])).count()
        };
        let mut g = SatinGeometry {
            rail_a: line(&[(0.0, 0.0), (10.0, 0.0)]),
            rail_b: line(&[(0.0, 2.0), (6.0, 2.0), (4.0, 3.0), (10.0, 3.0)]),
            rungs: Vec::new(),
        };
        let before = Column::new(&g).unwrap();
        assert!(crossings(&before) > 0);
        g.rungs = before.untangled_rungs(0.2).expect("rungs");
        let after = Column::new(&g).unwrap();
        assert_eq!(crossings(&after), 0);
        let pairs = after.pairs(0.2);
        assert!(pairs[0].0.distance(Point::new(0.0, 0.0)) < 1e-9);
        assert!(pairs[pairs.len() - 1].1.distance(Point::new(10.0, 3.0)) < 1e-9);
        // A column with no crossing pairs keeps its rungs.
        let plain = SatinGeometry {
            rail_a: line(&[(0.0, 0.0), (10.0, 0.0)]),
            rail_b: line(&[(0.0, 2.0), (10.0, 2.0)]),
            rungs: Vec::new(),
        };
        assert!(Column::new(&plain).unwrap().untangled_rungs(0.2).is_none());
    }

    fn straight(w: f64, l: f64) -> SatinGeometry {
        SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(l, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, w)).line_to(Point::new(l, w)),
            rungs: Vec::new(),
        }
    }

    fn plain(spacing: f64) -> SatinParams {
        SatinParams {
            spacing_mm: spacing,
            pull_compensation_mm: 0.0,
            short_stitches: false,
            underlay: Underlay::None,
            split_mm: emb_model::object::SATIN_SPLIT_MM,
            ..SatinParams::default()
        }
    }

    /// A straight column 20 mm long and 4 mm wide, with the rungs of
    /// `extra` (pairs across it) and its two ends.
    fn ruled(rungs: Vec<(Point, Point)>) -> SatinGeometry {
        SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(20.0, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, 4.0)).line_to(Point::new(20.0, 4.0)),
            rungs,
        }
    }

    /// The underlay goes at even steps along the column, not at each rung:
    /// many rungs close together (a column that pivots) do not crowd it
    /// (ADR 0163). The top stitches still have a needle point at each rung.
    #[test]
    fn the_underlay_does_not_crowd_at_close_rungs() {
        let zigzag = SatinParams {
            underlay: Underlay::Custom(vec![UnderlayLayer::Zigzag {
                inset_mm: 0.4,
                spacing_mm: 2.0,
            }]),
            ..plain(0.4)
        };
        let none = plain(0.4);
        let count = |g: &SatinGeometry, p: &SatinParams| satin(g, p, Point::ZERO, None).unwrap().len();
        // 40 rungs in the first 2 mm of the column.
        let close: Vec<(Point, Point)> = (1..=40)
            .map(|i| {
                let x = 0.05 * f64::from(i);
                (Point::new(x, 0.0), Point::new(x, 4.0))
            })
            .collect();
        let (plain_g, ruled) = (ruled(Vec::new()), ruled(close));
        let underlay = |g: &SatinGeometry| count(g, &zigzag) - count(g, &none);
        assert_eq!(
            underlay(&ruled),
            underlay(&plain_g),
            "the same underlay with close rungs"
        );
        assert!(
            count(&ruled, &none) > count(&plain_g, &none),
            "the top stitches keep the rungs"
        );
    }

    /// On another part of a letter, the underlay is left out (with the
    /// joint margin) and crossed with a walk; the top stitches stay.
    #[test]
    fn the_underlay_stops_on_another_part() {
        let p = SatinParams {
            underlay: Underlay::Custom(vec![UnderlayLayer::Zigzag {
                inset_mm: 0.4,
                spacing_mm: 2.0,
            }]),
            ..plain(0.4)
        };
        let g = ruled(Vec::new());
        // Another column across the first 6 mm.
        let other = vec![vec![
            Point::new(-1.0, -1.0),
            Point::new(6.0, -1.0),
            Point::new(6.0, 5.0),
            Point::new(-1.0, 5.0),
        ]];
        let near = |pts: &[Point]| pts.iter().filter(|q| q.x < 6.0 + JOINT_MARGIN_MM).count();
        let all = satin(&g, &p, Point::ZERO, None).unwrap();
        let on = satin_on(&g, &p, Point::ZERO, None, &[other]).unwrap();
        let top = satin(&g, &plain(0.4), Point::ZERO, None).unwrap();
        assert!(near(&on) < near(&all), "{} of {}", near(&on), near(&all));
        // There, only the top stitches and a walk across.
        assert!(near(&on) <= near(&top) + (2.0 * (6.0 + JOINT_MARGIN_MM) / WALK_LEN_MM).ceil() as usize + 4);
        // Past the other part, the underlay is there.
        let far = |pts: &[Point]| pts.iter().filter(|q| q.x > 10.0).count();
        assert_eq!(far(&on), far(&all));
    }

    #[test]
    fn straight_column_zigzags_at_half_spacing() {
        let pts = satin(&straight(3.0, 10.0), &plain(0.4), Point::new(-5.0, 0.0), None).unwrap();
        // 10 mm at 0.2 mm: 50 steps, 51 needle points, alternating sides.
        assert_eq!(pts.len(), 51);
        for (j, p) in pts.iter().enumerate() {
            assert_eq!(p.y, if j.is_multiple_of(2) { 0.0 } else { 3.0 });
            assert!((p.x - j as f64 * 0.2).abs() < 1e-9);
        }
    }

    #[test]
    fn omit_last_ends_on_the_other_rail() {
        let mut p = plain(0.4);
        let all = satin(&straight(3.0, 10.0), &p, Point::new(-5.0, 0.0), None).unwrap();
        p.omit_last = true;
        let less = satin(&straight(3.0, 10.0), &p, Point::new(-5.0, 0.0), None).unwrap();
        assert_eq!(less.len(), all.len() - 1);
        assert_eq!(less[..], all[..all.len() - 1]);
        assert_ne!(less.last().unwrap().y, all.last().unwrap().y);
    }

    #[test]
    fn starts_at_the_end_nearer_to_entry() {
        let pts = satin(&straight(3.0, 10.0), &plain(0.4), Point::new(20.0, 0.0), None).unwrap();
        assert!((pts[0].x - 10.0).abs() < 1e-9);
        assert!(pts[pts.len() - 1].x.abs() < 1e-9);
    }

    #[test]
    fn exit_sets_the_far_end() {
        let pts = satin(
            &straight(3.0, 10.0),
            &plain(0.4),
            Point::new(-5.0, 0.0),
            Some(Point::new(-5.0, 0.0)),
        )
        .unwrap();
        assert!(pts[pts.len() - 1].x.abs() < 1e-9, "ends at x = 0, near the exit");
    }

    #[test]
    fn pull_compensation_widens() {
        let mut p = plain(0.4);
        p.pull_compensation_mm = 0.2;
        let pts = satin(&straight(3.0, 10.0), &p, Point::ZERO, None).unwrap();
        let (lo, hi) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q.y), b.max(q.y)));
        assert!((lo + 0.2).abs() < 1e-9 && (hi - 3.2).abs() < 1e-9);
    }

    #[test]
    fn rungs_set_the_stitch_direction() {
        // Rail B is shifted 4 mm. Without rungs, stitches pair the rail starts and are slanted.
        let mut g = SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(20.0, 0.0)),
            rail_b: Path::new()
                .move_to(Point::new(-4.0, 3.0))
                .line_to(Point::new(24.0, 3.0)),
            rungs: Vec::new(),
        };
        let slant = |pts: &[Point]| pts[20].x - pts[21].x;
        let free = satin(&g, &plain(0.4), Point::new(-10.0, 0.0), None).unwrap();
        g.rungs = vec![
            (Point::new(5.0, 0.0), Point::new(5.0, 3.0)),
            (Point::new(15.0, 0.0), Point::new(15.0, 3.0)),
        ];
        let ruled = satin(&g, &plain(0.4), Point::new(-10.0, 0.0), None).unwrap();
        assert!(slant(&ruled).abs() < slant(&free).abs());
        // At a rung there is a needle point (on side A or B, by the zigzag phase).
        assert!(
            ruled
                .iter()
                .any(|p| (p.x - 5.0).abs() < 1e-9 && (p.y == 0.0 || p.y == 3.0))
        );
    }

    #[test]
    fn short_stitches_on_the_inside_of_a_curve() {
        // A half ring: inner radius 2, outer radius 6.
        let arc = |r: f64| {
            let pts: Vec<(Point, bool)> = (0..=36)
                .map(|i| {
                    let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 5.0);
                    (Point::new(r * c, r * s), true)
                })
                .collect();
            Path::through_points(&pts, false)
        };
        let g = SatinGeometry {
            rail_a: arc(2.0),
            rail_b: arc(6.0),
            rungs: Vec::new(),
        };
        let mut p = plain(0.4);
        let long = satin(&g, &p, Point::new(4.0, -1.0), None).unwrap();
        p.short_stitches = true;
        let short = satin(&g, &p, Point::new(4.0, -1.0), None).unwrap();
        assert_eq!(long.len(), short.len());
        let shortened = long.iter().zip(&short).filter(|(a, b)| a.distance(**b) > 0.1).count();
        // Every second inner needle point (a quarter of all points) is moved inward.
        assert!(
            shortened > long.len() / 5 && shortened < long.len() / 3,
            "{shortened} of {}",
            long.len()
        );
        // No needle point inside the inner rail (within the flatten tolerance).
        assert!(short.iter().all(|q| q.length() > 2.0 - 2.0 * FLATTEN_TOL_MM));
    }

    #[test]
    fn liquid_spreads_a_sharp_turn() {
        // Rungs that turn the stitches 45° at once in the middle of a 40 mm column.
        let g = SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(40.0, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, 4.0)).line_to(Point::new(40.0, 4.0)),
            rungs: vec![
                (Point::new(19.0, 0.0), Point::new(19.0, 4.0)),
                (Point::new(21.0, 0.0), Point::new(25.0, 4.0)),
                (Point::new(30.0, 0.0), Point::new(30.0, 4.0)),
            ],
        };
        // The largest sudden change in how fast the slant changes: the
        // second difference of the slant of the stitches across (A to B).
        let jump = |p: &SatinParams| {
            let pts = satin(&g, p, Point::new(-5.0, 0.0), None).unwrap();
            let slants: Vec<f64> = pts
                .windows(2)
                .map(|w| {
                    if w[0].y < w[1].y {
                        w[1].x - w[0].x
                    } else {
                        w[0].x - w[1].x
                    }
                })
                .collect();
            slants
                .windows(5)
                .map(|s| (s[4] - 2.0 * s[2] + s[0]).abs())
                .fold(0.0, f64::max)
        };
        let mut p = plain(0.4);
        let sharp = jump(&p);
        p.liquid = true;
        let smooth = jump(&p);
        assert!(smooth < sharp / 2.0, "{smooth} {sharp}");
    }

    #[test]
    fn accordion_goes_from_the_drawn_start_in_both_directions() {
        let (_, fade) = emb_model::Accordion::presets().remove(0);
        let p = SatinParams {
            accordion: Some(fade),
            ..plain(0.4)
        };
        // Dense at x = 0 (the drawn start), open at x = 30, from either end.
        for entry in [Point::new(-5.0, 0.0), Point::new(35.0, 0.0)] {
            let pts = satin(&straight(3.0, 30.0), &p, entry, None).unwrap();
            let (start, end) = (
                pts.iter().filter(|q| q.x < 10.0).count(),
                pts.iter().filter(|q| q.x > 20.0).count(),
            );
            assert!(start > 2 * end, "{entry:?}: {start} {end}");
        }
    }

    #[test]
    fn wide_stitches_are_split() {
        let pts = satin(&straight(10.0, 2.0), &plain(0.4), Point::ZERO, None).unwrap();
        for w in pts.windows(2) {
            assert!(w[0].distance(w[1]) <= emb_model::object::SATIN_SPLIT_MM, "{w:?}");
        }
        // The split points do not line up from stitch to stitch.
        let ys: std::collections::BTreeSet<i64> = pts
            .iter()
            .filter(|p| p.y > 0.1 && p.y < 9.9)
            .map(|p| (p.y * 1000.0).round() as i64)
            .collect();
        assert_eq!(ys.len(), 2);
    }

    #[test]
    fn puff_has_no_rows_across_and_closed_ends() {
        let p = SatinParams {
            puff: true,
            underlay: Underlay::Heavy,
            ..plain(0.25)
        };
        let col = straight(5.0, 30.0);
        let pts = satin(&col, &p, Point::ZERO, None).unwrap();
        // Needle points past both ends of the column (x < 0 and x > 30).
        assert!(pts.iter().any(|q| q.x < -0.3), "the start end is closed");
        assert!(pts.iter().any(|q| q.x > 30.3), "the end is closed");
        // No zigzag underlay: before the first stitch past the start, only
        // walks along the rails or the middle (y is the same in a stitch).
        let flat = SatinParams {
            puff: false,
            ..p.clone()
        };
        let plain_pts = satin(&col, &flat, Point::ZERO, None).unwrap();
        let first_cap = pts.iter().position(|q| q.x < -0.3).unwrap();
        let across = |v: &[Point]| v.windows(2).filter(|w| (w[0].y - w[1].y).abs() > 2.0).count();
        // (Only the step from the center walk to the edge walk, and the
        // turn of the edge walk from one rail to the other.)
        assert!(across(&pts[..first_cap]) <= 2, "no rows across the column under puff");
        assert!(
            across(&plain_pts) > across(&pts[first_cap..]),
            "heavy underlay has rows across"
        );
    }

    #[test]
    fn lace_underlay_sews_the_edge_two_times() {
        let wide = lace_underlay(3.0);
        assert_eq!(wide.len(), 3);
        assert_eq!(wide[0], wide[1], "the same edge walk two times");
        assert!(matches!(wide[2], UnderlayLayer::Zigzag { .. }));
        assert_eq!(
            lace_underlay(0.8),
            vec![UnderlayLayer::CenterWalk, UnderlayLayer::CenterWalk]
        );
        let p = SatinParams {
            underlay: Underlay::Lace,
            ..plain(0.4)
        };
        let lace = satin(&straight(3.0, 30.0), &p, Point::ZERO, None).unwrap();
        let none = satin(&straight(3.0, 30.0), &plain(0.4), Point::ZERO, None).unwrap();
        assert!(lace.len() > none.len() + 40);
    }

    #[test]
    fn auto_underlay_by_width() {
        assert_eq!(auto_underlay(3.0).len(), 2);
        assert_eq!(auto_underlay(1.5), vec![UnderlayLayer::CenterWalk]);
        assert!(auto_underlay(0.5).is_empty());
        assert_eq!(light_underlay(3.0), vec![UnderlayLayer::CenterWalk]);
        assert!(light_underlay(1.5).is_empty());
        assert_eq!(heavy_underlay(3.0).len(), 3);
        assert_eq!(heavy_underlay(1.5).len(), 2);
        assert_eq!(heavy_underlay(0.5), vec![UnderlayLayer::CenterWalk]);
        let count = |u: Underlay| {
            let p = SatinParams {
                underlay: u,
                ..plain(0.4)
            };
            satin(&straight(4.0, 30.0), &p, Point::ZERO, None).unwrap().len()
        };
        assert!(count(Underlay::Light) < count(Underlay::Auto));
        assert!(count(Underlay::Auto) < count(Underlay::Heavy));
        // Underlay comes first and returns to the start; the top ends at the far end.
        let p = SatinParams {
            underlay: Underlay::Auto,
            ..plain(0.4)
        };
        let pts = satin(&straight(4.0, 30.0), &p, Point::ZERO, None).unwrap();
        let top = satin(&straight(4.0, 30.0), &plain(0.4), Point::ZERO, None).unwrap();
        assert!(pts.len() > top.len() + 20);
        assert!(
            pts[..pts.len() - top.len()]
                .iter()
                .all(|q| q.y >= 0.4 - 1e-9 && q.y <= 3.6 + 1e-9)
        );
        assert_eq!(&pts[pts.len() - top.len()..], &top[..]);
    }
}
