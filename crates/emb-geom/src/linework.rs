//! Line work for lines drawn by hand (owner request, 2026-11-13, ADR 0092):
//!
//! - [`trim_piece`]: the part of a line between the two crossings (with
//!   other lines or with itself) on each side of a click goes, as a "trim
//!   between crossings" tool does.
//! - [`enclosed_area`]: the area that lines enclose round a point (Smart
//!   Fill). Gaps narrower than the band width close.

use crate::nodes::{Piece, SubNodes};
use crate::path::Polyline;
use crate::region::{self, Join, Region};
use crate::{BBox, Point};

/// Short lines on each cubic segment.
const SAMPLES: usize = 32;
/// Crossings nearer than this along the line are one crossing, in mm.
const SAME: f64 = 1e-6;

fn pieces(sub: &SubNodes) -> Vec<Piece> {
    sub.pieces(1, SAMPLES)
}

/// A place on a subpath: its distance from the start, the segment and the
/// parameter.
#[derive(Debug, Clone, Copy)]
struct Place {
    s: f64,
    seg: usize,
    t: f64,
}

fn place(p: &Piece, u: f64) -> Place {
    Place {
        s: p.s + p.a.distance(p.b) * u,
        seg: p.seg,
        t: p.ta + (p.tb - p.ta) * u,
    }
}

/// The places where `sub` crosses the lines `others` or itself.
fn crossings(sub: &SubNodes, ps: &[Piece], others: &[Polyline]) -> Vec<Place> {
    let mut out = Vec::new();
    for p in ps {
        for (c, d) in others.iter().flat_map(Polyline::segments) {
            if let Some((u, _)) = Point::crossing(p.a, p.b, c, d) {
                out.push(place(p, u));
            }
        }
    }
    // Crossings with itself: pieces that do not touch along the line.
    let k = ps.len();
    for i in 0..k {
        for j in i + 2..k {
            if sub.closed && i == 0 && j == k - 1 {
                continue;
            }
            if let Some((u, v)) = Point::crossing(ps[i].a, ps[i].b, ps[j].a, ps[j].b) {
                out.push(place(&ps[i], u));
                out.push(place(&ps[j], v));
            }
        }
    }
    out.sort_by(|a, b| a.s.total_cmp(&b.s));
    out.dedup_by(|a, b| (a.s - b.s).abs() < SAME);
    out
}

/// `sub` before the place `p`.
fn before(sub: &SubNodes, p: Place) -> SubNodes {
    let mut s = sub.clone();
    s.cut_after(p.seg, p.t);
    s
}

/// `sub` after the place `p`.
fn after(sub: &SubNodes, p: Place) -> SubNodes {
    let mut s = sub.clone();
    s.cut_before(p.seg, p.t);
    s
}

/// `sub` from place `a` to place `b` (a before b).
fn between(sub: &SubNodes, a: Place, b: Place) -> SubNodes {
    let mut s = sub.clone();
    s.cut_after(b.seg, b.t);
    // The cut made segment `b.seg` shorter.
    let t = if a.seg == b.seg && b.t > 1e-9 { a.t / b.t } else { a.t };
    s.cut_before(a.seg, t);
    s
}

/// A closed subpath as an open one from node 0 round to node 0.
fn unrolled(sub: &SubNodes) -> SubNodes {
    let mut s = sub.clone();
    if s.closed {
        s.nodes.push(s.nodes[0]);
        s.closed = false;
    }
    s
}

/// A part with a length (not only a point).
fn is_line(s: &SubNodes) -> bool {
    s.nodes.len() >= 2 && pieces(s).iter().map(|p| p.a.distance(p.b)).sum::<f64>() > SAME
}

/// The parts of `sub` that stay when the part under `click` (between the
/// crossings before and after it) goes. `None` if `click` is farther than
/// `reach` from `sub`. An empty list: all of `sub` goes.
pub fn trim_piece(sub: &SubNodes, others: &[Polyline], click: Point, reach: f64) -> Option<Vec<SubNodes>> {
    let ps = pieces(sub);
    let (near, d) = ps
        .iter()
        .map(|p| (p, click.distance_to_segment(p.a, p.b)))
        .min_by(|x, y| x.1.total_cmp(&y.1))?;
    if d > reach {
        return None;
    }
    let len = near.a.distance(near.b);
    let u = if len > 0.0 {
        ((click - near.a).dot(near.b - near.a) / (len * len)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let at = place(near, u).s;
    let total = ps.last().map_or(0.0, |p| p.s + p.a.distance(p.b));
    // Crossings at the ends of an open line leave nothing on that side.
    let cs: Vec<Place> = crossings(sub, &ps, others)
        .into_iter()
        .filter(|c| sub.closed || (c.s > SAME && c.s < total - SAME))
        .collect();
    let lo = cs.iter().rev().find(|c| c.s < at).copied();
    let hi = cs.iter().find(|c| c.s > at).copied();
    let keep: Vec<SubNodes> = if !sub.closed {
        let mut v = Vec::new();
        if let Some(lo) = lo {
            v.push(before(sub, lo));
        }
        if let Some(hi) = hi {
            v.push(after(sub, hi));
        }
        v
    } else {
        if cs.len() < 2 {
            return Some(Vec::new());
        }
        let open = unrolled(sub);
        match (lo, hi) {
            // The part that goes is inside the loop: the rest goes round
            // through the first node.
            (Some(lo), Some(hi)) => {
                let mut a = after(&open, hi);
                let b = before(&open, lo);
                a.nodes.extend(b.nodes.into_iter().skip(1));
                a.segs.extend(b.segs);
                vec![a]
            }
            // The part that goes goes through the first node.
            _ => vec![between(&open, cs[0], cs[cs.len() - 1])],
        }
    };
    Some(keep.into_iter().filter(is_line).collect())
}

/// The area round `p` that the lines enclose, with its holes (lines inside
/// it). The lines are bands `2 * half` wide first, so gaps narrower than
/// that close; the area then grows back to the middle of the bands. `None`
/// if `p` is on a line or the lines do not go all round it.
pub fn enclosed_area(lines: &[Polyline], p: Point, half: f64) -> Option<Region> {
    let bands = region::offset_lines(lines, half, Join::Round);
    let bb = lines.iter().flat_map(|l| l.points.iter()).fold(None::<BBox>, |b, q| {
        Some(b.map_or(BBox::new(*q, *q), |b| b.union(BBox::new(*q, *q))))
    })?;
    let m = 10.0 * half + 1.0;
    let outer = BBox::new(bb.min - Point::new(m, m), bb.max + Point::new(m, m));
    let frame = Region::new(
        vec![
            outer.min,
            Point::new(outer.max.x, outer.min.y),
            outer.max,
            Point::new(outer.min.x, outer.max.y),
        ],
        Vec::new(),
    );
    let free = region::difference(&[frame], &bands);
    let face = free.into_iter().find(|r| r.contains(p))?;
    // The area outside all the lines touches the frame.
    let edge = |q: &Point| {
        (q.x - outer.min.x).abs() < 1e-6
            || (q.x - outer.max.x).abs() < 1e-6
            || (q.y - outer.min.y).abs() < 1e-6
            || (q.y - outer.max.y).abs() < 1e-6
    };
    if face.outer.iter().any(edge) {
        return None;
    }
    region::offset(&[face], half, Join::Miter)
        .into_iter()
        .find(|r| r.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Path;
    use crate::nodes::NodePath;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn sub(pts: &[Point], closed: bool) -> SubNodes {
        let path = if closed {
            Path::polygon(pts)
        } else {
            pts[1..].iter().fold(Path::new().move_to(pts[0]), |a, q| a.line_to(*q))
        };
        NodePath::from_path(&path).subs.remove(0)
    }

    fn pl(pts: &[Point]) -> Polyline {
        Polyline {
            points: pts.to_vec(),
            closed: false,
        }
    }

    #[test]
    fn the_part_between_two_crossings_goes() {
        // A line across two vertical lines; click between them.
        let s = sub(&[p(0.0, 0.0), p(30.0, 0.0)], false);
        let others = [pl(&[p(10.0, -5.0), p(10.0, 5.0)]), pl(&[p(20.0, -5.0), p(20.0, 5.0)])];
        let keep = trim_piece(&s, &others, p(15.0, 0.2), 1.0).unwrap();
        assert_eq!(keep.len(), 2);
        assert_eq!(keep[0].nodes, vec![p(0.0, 0.0), p(10.0, 0.0)]);
        assert_eq!(keep[1].nodes, vec![p(20.0, 0.0), p(30.0, 0.0)]);
        // An overshoot: click past the last crossing; the end goes.
        let keep = trim_piece(&s, &others, p(25.0, 0.0), 1.0).unwrap();
        assert_eq!(keep.len(), 1);
        assert_eq!(keep[0].nodes, vec![p(0.0, 0.0), p(20.0, 0.0)]);
        // No crossing: all of it goes. Too far: nothing.
        assert_eq!(trim_piece(&s, &[], p(5.0, 0.0), 1.0), Some(Vec::new()));
        assert_eq!(trim_piece(&s, &[], p(5.0, 3.0), 1.0), None);
    }

    #[test]
    fn a_closed_line_opens_where_the_part_goes() {
        // A square crossed by a line through its left and right sides.
        let s = sub(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)], true);
        let others = [pl(&[p(-5.0, 5.0), p(15.0, 5.0)])];
        // Click on the top side: the top half goes, the bottom half stays.
        let keep = trim_piece(&s, &others, p(5.0, 0.0), 1.0).unwrap();
        assert_eq!(keep.len(), 1);
        let k = &keep[0];
        assert!(!k.closed);
        assert!(k.nodes.contains(&p(10.0, 10.0)) && k.nodes.contains(&p(0.0, 10.0)));
        assert!(!k.nodes.contains(&p(0.0, 0.0)));
        // Click on the bottom side: the top half stays.
        let keep = trim_piece(&s, &others, p(5.0, 10.0), 1.0).unwrap();
        assert!(keep[0].nodes.contains(&p(0.0, 0.0)) && keep[0].nodes.contains(&p(10.0, 0.0)));
    }

    #[test]
    fn a_line_that_crosses_itself_loses_its_loop() {
        // A line with a loop: it crosses itself at (5, 0).
        let s = sub(
            &[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)],
            false,
        );
        let keep = trim_piece(&s, &[], p(10.0, 2.0), 1.0).unwrap();
        assert_eq!(keep.len(), 2);
        assert_eq!(keep[0].nodes, vec![p(0.0, 0.0), p(5.0, 0.0)]);
        assert_eq!(keep[1].nodes, vec![p(5.0, 0.0), p(5.0, -5.0)]);
    }

    #[test]
    fn the_area_inside_crossing_lines_with_a_small_gap() {
        // Four strokes that cross at the corners of a 20 mm square; one
        // corner has a gap of 0.2 mm.
        let lines = [
            pl(&[p(-2.0, 0.0), p(22.0, 0.0)]),
            pl(&[p(20.0, -2.0), p(20.0, 22.0)]),
            pl(&[p(22.0, 20.0), p(0.2, 20.0)]),
            pl(&[p(0.0, 19.8), p(0.0, -2.0)]),
        ];
        let r = enclosed_area(&lines, p(10.0, 10.0), 0.15).unwrap();
        assert!((r.area() - 400.0).abs() < 2.0, "{}", r.area());
        // Outside the lines: not enclosed.
        assert!(enclosed_area(&lines, p(30.0, 10.0), 0.15).is_none());
        // A larger gap does not close.
        let open = [
            lines[0].clone(),
            lines[1].clone(),
            pl(&[p(22.0, 20.0), p(3.0, 20.0)]),
            lines[3].clone(),
        ];
        assert!(enclosed_area(&open, p(10.0, 10.0), 0.15).is_none());
    }

    #[test]
    fn an_island_inside_is_a_hole() {
        let square = |a: f64, b: f64| Polyline {
            points: vec![p(a, a), p(b, a), p(b, b), p(a, b)],
            closed: true,
        };
        let r = enclosed_area(&[square(0.0, 20.0), square(8.0, 12.0)], p(3.0, 3.0), 0.15).unwrap();
        assert_eq!(r.holes.len(), 1);
        assert!((r.area() - 384.0).abs() < 2.0, "{}", r.area());
    }
}
