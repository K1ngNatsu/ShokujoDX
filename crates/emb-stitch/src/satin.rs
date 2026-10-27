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
        let mut best = (f64::INFINITY, 0.0);
        for (i, w) in self.pts.windows(2).enumerate() {
            let d = w[1] - w[0];
            let l2 = d.length_sq();
            let t = if l2 > 0.0 {
                ((p - w[0]).dot(d) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dist = p.distance(w[0].lerp(w[1], t));
            if dist < best.0 {
                best = (dist, self.at_len[i] + t * (self.at_len[i + 1] - self.at_len[i]));
            }
        }
        best.1
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
        let mut inner: Vec<(f64, f64)> = g
            .rungs
            .iter()
            .map(|(pa, pb)| (a.project(*pa), b.project(*pb)))
            .collect();
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
        let step = step.max(0.05);
        let mut out = vec![(self.a.at(0.0), self.b.at(0.0))];
        if self.even {
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
            return out;
        }
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

/// Needle points of one underlay layer, from the start of the column to the end and back.
fn underlay_layer(col: &Column, layer: &UnderlayLayer) -> Vec<Point> {
    match *layer {
        UnderlayLayer::CenterWalk => {
            let mids: Vec<Point> = col.pairs(0.5).iter().map(|(a, b)| a.midpoint(*b)).collect();
            let mut pts = walk(mids);
            let back: Vec<Point> = pts.iter().rev().skip(1).copied().collect();
            pts.extend(back);
            pts
        }
        UnderlayLayer::EdgeWalk { inset_mm } => {
            let pairs = inset_pairs(&col.pairs(0.5), inset_mm);
            let mut pts = walk(pairs.iter().map(|p| p.0).collect());
            pts.extend(walk(pairs.iter().rev().map(|p| p.1).collect()));
            pts
        }
        UnderlayLayer::Zigzag { inset_mm, spacing_mm } => {
            let pairs = inset_pairs(&col.pairs(spacing_mm.max(0.2) / 2.0), inset_mm);
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
    satin_with(g, p, entry, exit, false)
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
    let layers = match &p.underlay {
        Underlay::Auto => auto_underlay(col.mean_width()),
        Underlay::Light => light_underlay(col.mean_width()),
        Underlay::Heavy => heavy_underlay(col.mean_width()),
        Underlay::Double => double_underlay(col.mean_width()),
        Underlay::Lace => lace_underlay(col.mean_width()),
        Underlay::None => Vec::new(),
        Underlay::Custom(l) => l.clone(),
    };
    let mut out: Vec<Point> = Vec::new();
    for layer in &layers {
        // Puff: no rows across the column, they press the foam flat.
        if p.puff && !matches!(layer, UnderlayLayer::CenterWalk | UnderlayLayer::EdgeWalk { .. }) {
            continue;
        }
        out.extend(underlay_layer(&col, layer));
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
