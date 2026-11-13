//! Close gaps: lines that were drawn one at a time become one line where
//! their ends are near (owner request, 2026-11-13).
//!
//! - Two open ends nearer than the gap meet at the point halfway between
//!   them. The nearest ends join first, and each end joins one time only.
//! - An overshoot: if a line goes across another line near the ends of
//!   both (less than the gap from each end), the two lines are cut at the
//!   crossing and their ends join there. The short pieces past the
//!   crossing go.
//! - When the two ends of one line (or of a chain of joined lines) are
//!   near, the line closes. Only a line longer than four gaps closes, so a
//!   short stroke does not become a small loop.
//! - Closed subpaths, and ends that are not near another end, do not change.
//!
//! The gaps are found between ends only: an end near the middle of another
//! line does not join it.

use crate::Point;
use crate::nodes::{Piece, Seg, SubNodes};

/// Short lines on each segment for the length and the overshoot test.
const SAMPLES: usize = 16;
/// Parameters nearer than this to 0 or 1 are the end of a segment.
const T_EPS: f64 = 1e-9;

/// Settings of [`close_gaps`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaps {
    /// Ends nearer than this join, in mm.
    pub gap_mm: f64,
    /// Cut lines that go across each other near their ends.
    pub overshoots: bool,
}

impl Gaps {
    /// The settings when the window opens.
    pub const STANDARD: Gaps = Gaps {
        gap_mm: 1.0,
        overshoots: true,
    };
}

impl Default for Gaps {
    fn default() -> Self {
        Gaps::STANDARD
    }
}

/// One subpath of the result.
#[derive(Debug, Clone, PartialEq)]
pub struct GapPart {
    pub sub: SubNodes,
    /// The indexes of the input subpaths in this part, in order.
    pub from: Vec<usize>,
    /// `false` if the part is an input subpath with no change.
    pub changed: bool,
}

/// The result of [`close_gaps`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GapResult {
    pub parts: Vec<GapPart>,
    /// The number of gaps that closed (two ends that joined).
    pub joins: usize,
    /// The number of parts that became closed.
    pub closed: usize,
}

fn is_open(s: &SubNodes) -> bool {
    !s.closed && s.nodes.len() >= 2
}

fn end_point(s: &SubNodes, last: bool) -> Point {
    if last { s.nodes[s.nodes.len() - 1] } else { s.nodes[0] }
}

/// The length of a subpath, from short lines on each segment.
fn length(s: &SubNodes) -> f64 {
    s.pieces(1, SAMPLES).iter().map(|p| p.a.distance(p.b)).sum()
}

/// Moves the first (or the last) node of an open subpath to `to`, with
/// its handle, so the direction at the end stays.
fn move_end(s: &mut SubNodes, last: bool, to: Point) {
    let i = if last { s.nodes.len() - 1 } else { 0 };
    let d = to - s.nodes[i];
    s.nodes[i] = to;
    if s.segs.is_empty() {
        return;
    }
    let j = if last { s.segs.len() - 1 } else { 0 };
    if let Seg::Cubic(c1, c2) = s.segs[j] {
        s.segs[j] = if last {
            Seg::Cubic(c1, c2 + d)
        } else {
            Seg::Cubic(c1 + d, c2)
        };
    }
}

/// The pieces of a subpath from one end (each from the end side), until
/// they are `reach` long.
fn tail(s: &SubNodes, last: bool, reach: f64) -> Vec<Piece> {
    let mut all = s.pieces(SAMPLES, SAMPLES);
    if last {
        all.reverse();
        for p in &mut all {
            std::mem::swap(&mut p.a, &mut p.b);
            std::mem::swap(&mut p.ta, &mut p.tb);
        }
    }
    let mut len = 0.0;
    let mut out = Vec::new();
    for p in all {
        len += p.a.distance(p.b);
        out.push(p);
        if len >= reach {
            break;
        }
    }
    out
}

/// A cut of an end: the end (see [`ends`]), the segment and the parameter.
#[derive(Debug, Clone, Copy)]
struct Cut {
    end: usize,
    seg: usize,
    t: f64,
}

/// The open ends: end `2k` is the start and `2k + 1` the last node of the
/// `k`-th open subpath.
fn ends(subs: &[SubNodes]) -> Vec<(usize, bool)> {
    subs.iter()
        .enumerate()
        .filter(|(_, s)| is_open(s))
        .flat_map(|(i, _)| [(i, false), (i, true)])
        .collect()
}

/// Cuts the lines that go across each other near their ends. Returns the
/// indexes of the subpaths that changed.
fn trim_overshoots(subs: &mut [SubNodes], gap: f64) -> Vec<usize> {
    let ends = ends(subs);
    let tails: Vec<Vec<Piece>> = ends
        .iter()
        .map(|&(i, last)| tail(&subs[i], last, gap.min(length(&subs[i]) / 3.0)))
        .collect();
    // (cost, the cut of each end, the crossing)
    let mut found: Vec<(f64, Cut, Cut, Point)> = Vec::new();
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            let (pi, pj) = (
                end_point(&subs[ends[i].0], ends[i].1),
                end_point(&subs[ends[j].0], ends[j].1),
            );
            if pi.distance(pj) > 2.0 * gap {
                continue;
            }
            let mut best: Option<(f64, Cut, Cut, Point)> = None;
            let mut li = 0.0;
            for a in &tails[i] {
                let mut lj = 0.0;
                for b in &tails[j] {
                    if let Some((u, v)) = Point::crossing(a.a, a.b, b.a, b.b) {
                        let x = a.a.lerp(a.b, u);
                        let cost = li + a.a.distance(x) + lj + b.a.distance(x);
                        if best.as_ref().is_none_or(|bst| cost < bst.0) {
                            let ci = Cut {
                                end: i,
                                seg: a.seg,
                                t: a.ta + (a.tb - a.ta) * u,
                            };
                            let cj = Cut {
                                end: j,
                                seg: b.seg,
                                t: b.ta + (b.tb - b.ta) * v,
                            };
                            best = Some((cost, ci, cj, x));
                        }
                    }
                    lj += b.a.distance(b.b);
                }
                li += a.a.distance(a.b);
            }
            found.extend(best);
        }
    }
    found.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.end.cmp(&b.1.end))
            .then(a.2.end.cmp(&b.2.end))
    });
    let mut used = vec![false; ends.len()];
    // For each subpath: the cut of its start and of its last node, with the crossing.
    let mut cuts: Vec<[Option<(Cut, Point)>; 2]> = vec![[None, None]; subs.len()];
    for (_, ci, cj, x) in found {
        if used[ci.end] || used[cj.end] {
            continue;
        }
        used[ci.end] = true;
        used[cj.end] = true;
        for c in [ci, cj] {
            let (sub, last) = ends[c.end];
            cuts[sub][usize::from(last)] = Some((c, x));
        }
    }
    let mut changed = Vec::new();
    for (i, [first, last]) in cuts.into_iter().enumerate() {
        if first.is_none() && last.is_none() {
            continue;
        }
        let s = &mut subs[i];
        if let Some((c, x)) = last {
            s.cut_after(c.seg, c.t);
            move_end(s, true, x);
        }
        if let Some((c, x)) = first {
            let mut t = c.t;
            if let Some((l, _)) = last
                && l.seg == c.seg
                && l.t > T_EPS
            {
                // The cut of the last node made the segment shorter.
                t /= l.t;
            }
            s.cut_before(c.seg, t);
            move_end(s, false, x);
        }
        changed.push(i);
    }
    changed
}

/// `a` then `b`: the last node of `a` and the first node of `b` meet
/// halfway between them.
fn meet(a: &mut SubNodes, mut b: SubNodes) {
    let m = end_point(a, true).midpoint(end_point(&b, false));
    move_end(a, true, m);
    move_end(&mut b, false, m);
    a.nodes.extend(b.nodes.into_iter().skip(1));
    a.segs.extend(b.segs);
}

/// Closes an open chain: its two ends meet halfway between them. Returns
/// `false` (and changes nothing) if the result would have no area.
fn close_chain(s: &mut SubNodes) -> bool {
    let has_curve = s.segs.iter().any(|g| matches!(g, Seg::Cubic(..)));
    if s.nodes.len() < 4 && !(has_curve && s.nodes.len() == 3) {
        return false;
    }
    let m = end_point(s, false).midpoint(end_point(s, true));
    move_end(s, true, m);
    move_end(s, false, m);
    s.nodes.pop();
    s.closed = true;
    true
}

/// A gap that joins each open end that has another open end (of another
/// subpath, or the other end of a long subpath) nearer than `max`: a little
/// more than the largest of those distances. `None` if no end has one.
pub fn suggested_gap(subs: &[SubNodes], max: f64) -> Option<f64> {
    let ends = ends(subs);
    let lens: Vec<f64> = subs.iter().map(length).collect();
    let mut gap: Option<f64> = None;
    for (i, &(si, li)) in ends.iter().enumerate() {
        let p = end_point(&subs[si], li);
        let nearest = ends
            .iter()
            .enumerate()
            .filter(|&(j, &(sj, _))| j != i && (sj != si || lens[si] > 4.0 * max))
            .map(|(_, &(sj, lj))| p.distance(end_point(&subs[sj], lj)))
            .fold(f64::INFINITY, f64::min);
        if nearest <= max {
            gap = Some(gap.map_or(nearest, |g| g.max(nearest)));
        }
    }
    // A little more, rounded up to 0.1 mm.
    gap.map(|g| ((g * 1.05 + 0.05) * 10.0).ceil() / 10.0)
}

/// The root of `i` in the chains (a union-find).
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Joins the open subpaths of `subs` whose ends are near (see the module
/// comment). The result has each input subpath in one part.
pub fn close_gaps(subs: &[SubNodes], g: &Gaps) -> GapResult {
    let gap = g.gap_mm.max(0.0);
    let mut subs = subs.to_vec();
    let mut changed = vec![false; subs.len()];
    if g.overshoots && gap > 0.0 {
        for i in trim_overshoots(&mut subs, gap) {
            changed[i] = true;
        }
    }
    let ends = ends(&subs);
    let n = subs.len();
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            let d = end_point(&subs[ends[i].0], ends[i].1).distance(end_point(&subs[ends[j].0], ends[j].1));
            if d <= gap {
                pairs.push((d, i, j));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    // The chains: a union-find of the subpaths, with the length of each chain.
    let mut parent: Vec<usize> = (0..n).collect();
    let mut chain_len: Vec<f64> = subs.iter().map(length).collect();
    let mut link: Vec<Option<usize>> = vec![None; ends.len()];
    for (_, i, j) in pairs {
        if link[i].is_some() || link[j].is_some() {
            continue;
        }
        let (ri, rj) = (root(&mut parent, ends[i].0), root(&mut parent, ends[j].0));
        if ri == rj {
            // The two free ends of one chain: it closes if it is long.
            if chain_len[ri] <= 4.0 * gap {
                continue;
            }
        } else {
            parent[rj] = ri;
            chain_len[ri] += chain_len[rj];
        }
        link[i] = Some(j);
        link[j] = Some(i);
    }
    // The end index of the start of each open subpath.
    let mut start_of = vec![None; n];
    for (e, &(i, last)) in ends.iter().enumerate() {
        if !last {
            start_of[i] = Some(e);
        }
    }
    let other = |e: usize| e ^ 1;
    let oriented = |s: &SubNodes, last: bool| if last { s.reversed() } else { s.clone() };
    let mut out = GapResult::default();
    let mut visited = vec![false; n];
    for i in 0..n {
        if visited[i] {
            continue;
        }
        let Some(e0) = start_of[i] else {
            visited[i] = true;
            out.parts.push(GapPart {
                sub: subs[i].clone(),
                from: vec![i],
                changed: changed[i],
            });
            continue;
        };
        // Go back to the free end of the chain (or round a closed chain).
        let mut begin = e0;
        let mut steps = 0;
        while let Some(q) = link[begin] {
            steps += 1;
            if ends[q].0 == i || steps > ends.len() {
                begin = e0;
                break;
            }
            begin = other(q);
        }
        let first = ends[begin].0;
        let mut cur = oriented(&subs[first], ends[begin].1);
        let mut from = vec![first];
        visited[first] = true;
        let mut out_end = other(begin);
        let mut closed = false;
        while let Some(q) = link[out_end] {
            let s = ends[q].0;
            if visited[s] {
                closed = close_chain(&mut cur);
                if closed {
                    out.joins += 1;
                    out.closed += 1;
                }
                break;
            }
            meet(&mut cur, oriented(&subs[s], ends[q].1));
            out.joins += 1;
            visited[s] = true;
            from.push(s);
            out_end = other(q);
        }
        let any_changed = closed || from.len() > 1 || from.iter().any(|&k| changed[k]);
        out.parts.push(GapPart {
            sub: cur,
            from,
            changed: any_changed,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Path;
    use crate::nodes::NodePath;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(pts: &[Point]) -> SubNodes {
        let mut path = Path::new().move_to(pts[0]);
        for q in &pts[1..] {
            path = path.line_to(*q);
        }
        NodePath::from_path(&path).subs.remove(0)
    }

    #[test]
    fn three_strokes_with_gaps_become_a_closed_triangle() {
        // Three sides of a triangle, each a stroke, with gaps of 0.5 mm.
        let subs = vec![
            line(&[p(0.0, 0.0), p(10.0, 0.0)]),
            line(&[p(10.3, 0.4), p(5.0, 9.0)]),
            line(&[p(0.2, 0.3), p(4.8, 8.7)]),
        ];
        let r = close_gaps(&subs, &Gaps::STANDARD);
        assert_eq!(r.parts.len(), 1, "{r:?}");
        assert_eq!(r.joins, 3);
        assert_eq!(r.closed, 1);
        let part = &r.parts[0];
        assert!(part.sub.closed && part.changed);
        assert_eq!(part.sub.nodes.len(), 3);
        let mut from = part.from.clone();
        from.sort_unstable();
        assert_eq!(from, vec![0, 1, 2]);
        // The ends met halfway.
        assert!(
            part.sub.nodes.iter().any(|q| q.distance(p(10.15, 0.2)) < 1e-9),
            "{part:?}"
        );
    }

    #[test]
    fn far_ends_and_closed_paths_do_not_change() {
        let square = NodePath::from_path(&Path::polygon(&[p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0)]))
            .subs
            .remove(0);
        let subs = vec![
            line(&[p(0.0, 20.0), p(10.0, 20.0)]),
            line(&[p(12.0, 20.0), p(20.0, 20.0)]),
            square.clone(),
        ];
        let r = close_gaps(&subs, &Gaps::STANDARD);
        assert_eq!(r.joins, 0);
        assert_eq!(r.parts.len(), 3);
        assert!(r.parts.iter().all(|x| !x.changed));
        assert_eq!(r.parts[2].sub, square);
        // A larger gap joins the two lines (2 mm apart).
        let r = close_gaps(
            &subs,
            &Gaps {
                gap_mm: 2.5,
                ..Gaps::STANDARD
            },
        );
        assert_eq!(r.joins, 1);
        assert_eq!(r.parts[0].sub.nodes, vec![p(0.0, 20.0), p(11.0, 20.0), p(20.0, 20.0)]);
    }

    #[test]
    fn the_nearest_ends_join_first() {
        // Three ends near one place: the two nearest join; the third stays.
        let subs = vec![
            line(&[p(-10.0, 0.0), p(0.0, 0.0)]),
            line(&[p(0.1, 0.0), p(10.0, 0.0)]),
            line(&[p(0.0, 0.6), p(0.0, 10.0)]),
        ];
        let r = close_gaps(&subs, &Gaps::STANDARD);
        assert_eq!(r.joins, 1);
        assert_eq!(r.parts.len(), 2);
        assert_eq!(r.parts[0].from, vec![0, 1]);
        assert!(!r.parts[1].changed);
    }

    #[test]
    fn overshoots_are_cut_at_the_crossing() {
        // An "L" drawn as two strokes that go 0.6 mm past each other.
        let subs = vec![line(&[p(0.0, -0.6), p(0.0, 10.0)]), line(&[p(-0.6, 0.0), p(10.0, 0.0)])];
        let r = close_gaps(&subs, &Gaps::STANDARD);
        assert_eq!(r.parts.len(), 1, "{r:?}");
        let nodes = &r.parts[0].sub.nodes;
        assert_eq!(nodes.len(), 3);
        assert!(nodes[1].distance(p(0.0, 0.0)) < 1e-9, "{nodes:?}");
        // Without the overshoot cut, the ends are 0.85 mm apart and meet halfway.
        let r = close_gaps(
            &subs,
            &Gaps {
                overshoots: false,
                ..Gaps::STANDARD
            },
        );
        assert_eq!(r.parts.len(), 1);
        assert!(r.parts[0].sub.nodes[1].distance(p(-0.3, -0.3)) < 1e-9);
    }

    #[test]
    fn a_curve_closes_and_keeps_its_shape() {
        // An open circle of four curves with a gap of 0.4 mm.
        let circle = Path::ellipse(Point::ZERO, 10.0, 10.0);
        let mut s = NodePath::from_path(&circle).subs.remove(0);
        // Open it: the last segment goes, the start moves a little.
        s.closed = false;
        let start = s.nodes[0];
        s.nodes.push(start + p(0.0, 0.4));
        let r = close_gaps(std::slice::from_ref(&s), &Gaps::STANDARD);
        assert_eq!(r.closed, 1);
        let c = &r.parts[0].sub;
        assert!(c.closed);
        assert_eq!(c.nodes.len(), s.nodes.len() - 1);
        assert!(c.segs.iter().all(|g| matches!(g, Seg::Cubic(..))));
        // A short stroke does not close.
        let short = line(&[p(0.0, 0.0), p(0.8, 0.0), p(0.8, 0.5), p(0.0, 0.5)]);
        assert_eq!(close_gaps(&[short], &Gaps::STANDARD).closed, 0);
    }

    #[test]
    fn two_drawn_shapes_close_into_one() {
        // Owner report (2026-11-13): two strokes drawn by hand, with gaps
        // of about 4 and 6 mm, must close into one shape (screen points of
        // the report, 0.385 mm each).
        let mm = |x: f64, y: f64| p(x * 0.385, y * 0.385);
        let top = {
            let path = Path::new()
                .move_to(mm(128.0, 213.0))
                .cubic_to(mm(95.0, 195.0), mm(95.0, 165.0), mm(110.0, 140.0))
                .line_to(mm(164.0, 55.0))
                .line_to(mm(252.0, 171.0));
            NodePath::from_path(&path).subs.remove(0)
        };
        let bottom = line(&[
            mm(130.0, 230.0),
            mm(80.0, 313.0),
            mm(193.0, 402.0),
            mm(273.0, 335.0),
            mm(194.0, 269.0),
            mm(259.0, 180.0),
        ]);
        let subs = vec![top, bottom];
        let gap = suggested_gap(&subs, 10.0).unwrap();
        assert!(gap > 6.5 && gap < 7.5, "{gap}");
        let r = close_gaps(
            &subs,
            &Gaps {
                gap_mm: gap,
                ..Gaps::STANDARD
            },
        );
        assert_eq!((r.parts.len(), r.joins, r.closed), (1, 2, 1), "{r:?}");
        assert!(r.parts[0].sub.closed);
        // With 1 mm, nothing joins.
        assert_eq!(close_gaps(&subs, &Gaps::STANDARD).joins, 0);
    }
}
