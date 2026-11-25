//! A split of the outline of a letter into strokes at its concave corners
//! (P21 Milestone 8, ADR 0152; `docs/design/p21-plan.md`).
//!
//! Where a stroke ends on another stroke or crosses it, the outline has two
//! concave corners (corners that turn into the shape), one on each side of
//! the stroke that ends. A straight line between them (a bridge) continues
//! the edge of the stroke that goes through. The outline cut along all the
//! bridges gives pieces, and each piece is one stroke with no junction.
//! The method is from the description of the stroke extraction of Make Me
//! a Hanzi (<https://github.com/skishore/makemeahanzi>); its code has no
//! license, and no code is copied.

use emb_geom::Point;

use crate::stroke::Optional;

/// A corner is concave if the outline turns into the shape at least this
/// much (degrees) over [`CORNER_SPAN`] of the capital height before and
/// after it.
const CONCAVE_DEG: f64 = 40.0;
const CORNER_SPAN: f64 = 0.02;
/// A bridge is at most this many times the typical width of the strokes
/// long. Longer (2 widths in the trial), a bridge went across the place
/// where a stroke meets two more (under the bar of 木 in 林), and the
/// piece under it had a junction at its end, with an open area.
const BRIDGE_MAX: f64 = 1.3;
/// At each end, a bridge is at most this angle (degrees) off the direction
/// of the outline that it continues.
const ALIGN_MAX_DEG: f64 = 35.0;
/// At each end, the other side of the corner (a side of the stroke that
/// ends on the bridge) is at least this angle (degrees) off the bridge.
const CUT_MIN_DEG: f64 = 35.0;
/// The points of a bridge are at least this part of the typical width from
/// the outline (not at its ends).
const BRIDGE_CLEAR: f64 = 0.05;
/// The end of a piece at a bridge goes this part of the length of the
/// bridge into the piece on the other side, so there is no gap at the
/// joint.
const BRIDGE_OVERLAP: f64 = 0.1;
/// The end of a piece is at a bridge if its outline turns at least this
/// much (degrees) at both ends of the bridge; else the bridge is a side of
/// the piece (the edge of the stroke that goes through).
const END_TURN_DEG: f64 = 45.0;
/// A concave corner with no bridge is a notch (two strokes meet at a point
/// there) if the outline turns at least this much (degrees).
const NOTCH_DEG: f64 = 110.0;
/// The cut from a notch is at most this many times the typical width of
/// the strokes long.
const NOTCH_MAX: f64 = 2.5;
/// The cut from a notch meets the outline across at this angle (degrees)
/// or more.
const CUT_END_MIN_DEG: f64 = 45.0;
/// A concave corner with no bridge where two strokes meet at a corner and
/// both end (an elbow) turns at least this much (degrees), less than a
/// notch.
const ELBOW_DEG: f64 = 80.0;
/// The outer corner of an elbow is at most this many times the typical
/// width from the concave corner, and the cut is at most as long.
const ELBOW_MAX: f64 = 2.5;
/// The outer corner turns at least this much (degrees).
const ELBOW_CONVEX_DEG: f64 = 30.0;
/// The width of each stroke is measured this part of the typical width
/// from the concave corner along its side.
const ELBOW_SPAN: f64 = 0.5;
/// The outer corner is at the width of each stroke from the side of the
/// other stroke, within this part of the width.
const ELBOW_FIT: f64 = 0.25;
/// A cut from a notch goes across the place where strokes
/// meet: some point of it (at a quarter, the middle or three quarters) is
/// at least this part of its length from the outline. A cut nearer the
/// outline all along goes along a stroke (P21 Milestone 14: in a heavy
/// "n" the cut from the notch on top of the stem went down the stem beside
/// the narrow counter, and the piece of the stem went into the counter).
/// The cuts across strokes are 0.23 or more; along strokes 0.14 or less.
/// Such a cut is optional. (The cut of an elbow along the hairline serif
/// of a fat face font goes along a stroke too, and the serif sews well as
/// a piece of its own; the cuts of elbows are optional already.)
const CUT_CLEAR: f64 = 0.15;
/// A ball at the end of a stroke (the terminals of "S", "9" and "r" in a
/// serif font, P21 Milestone 11) meets the stroke at a concave corner that
/// turns at least this much (degrees), less than a notch.
const BALL_MIN_DEG: f64 = 70.0;
/// The cut of a ball goes across the stroke: at most this many times the
/// typical width long.
const BALL_CUT_MAX: f64 = 1.6;
/// The ball is round: 4 pi times its area over the square of its outline
/// (1 for a circle, 0.79 for a square) is at least this.
const BALL_ROUND: f64 = 0.7;
/// The ball is at most this part of the area of the glyph.
const BALL_PART: f64 = 0.35;
/// The ball is wider than the stroke: a circle of its area is at least
/// this many times as wide as the cut.
const BALL_WIDER: f64 = 1.2;
/// A bridge from a concave corner to a curve (a soft bridge) is at most
/// this many times the typical width of the strokes long: it goes across
/// the end of one stroke, and the thick stroke of a serif font is wider
/// than the typical width.
const SOFT_MAX: f64 = 2.0;
/// At the curved end of a soft bridge, the direction of the outline before
/// the bridge is measured over this part of the typical width (the curve
/// turns slowly).
const SOFT_SPAN: f64 = 0.5;
/// A soft bridge ends at least this part of the typical width from the
/// ends of the other bridges (but at its own corner).
const SOFT_NEAR: f64 = 0.25;
/// A straight edge of the outline is a segment at least this part of the
/// typical width long (a curve of a font is many short segments).
const BRACKET_EDGE: f64 = 0.15;
/// A bracket of a serif is a curve at most this part of the typical width
/// long.
const BRACKET_MAX: f64 = 1.0;
/// The curve goes with the stroke that goes through only if the stroke
/// that ends is at most this angle (degrees) off square to the bridge.
const BRACKET_SQUARE_DEG: f64 = 20.0;

/// A piece of the shape: its rings (an outer ring and its holes, with the
/// shape on the left of each) and its area before its ends were moved into
/// the pieces next to it.
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    pub rings: Vec<Vec<Point>>,
    pub area: f64,
    /// For a ball cut from its stroke: the direction of its column, square
    /// to the cut and away from the stroke, so its stitches lie as the
    /// last stitches of the stroke (P21 Milestone 11).
    pub axis: Option<Point>,
    /// The ends of the piece at bridges: where its stroke ends on a stroke
    /// that goes through (P22 Milestone 7).
    pub joints: Vec<Joint>,
}

/// An end of a stroke on a stroke that goes through (a joint), for the
/// trial of overlapped strokes (P22 Milestone 7, ADR 0174): the bridge,
/// the direction into the stroke that goes through, and the distance from
/// the bridge to the middle of that stroke (half its width there), in the
/// units of the outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    pub a: Point,
    pub b: Point,
    pub into: Point,
    pub depth: f64,
}

/// The split of a shape: its pieces and its bridges.
#[derive(Debug, Clone, Default)]
pub(crate) struct Split {
    pub pieces: Vec<Piece>,
    /// The bridges and the cuts (the tests read them).
    #[cfg(test)]
    pub bridges: Vec<(Point, Point)>,
    /// Some bridges go to a curve (soft bridges), or cut a ball.
    pub soft: bool,
    /// Some cuts are at elbows.
    pub elbows: bool,
    /// Some cuts from notches go along a stroke.
    pub along: bool,
}

/// A concave corner: the vertex (ring, index), and the directions of the
/// outline into it and out of it.
#[derive(Debug, Clone, Copy)]
struct Concave {
    ring: usize,
    at: usize,
    p: Point,
    d_in: Point,
    d_out: Point,
}

fn angle(a: Point, b: Point) -> f64 {
    emb_geom::math::to_degrees(emb_geom::math::atan2(a.cross(b).abs(), a.dot(b)))
}

/// The winding number of the rings round `p`.
fn winding(rings: &[Vec<Point>], p: Point) -> i32 {
    let mut w = 0;
    for r in rings {
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            let left = (b - a).cross(p - a);
            if a.y <= p.y {
                if b.y > p.y && left > 0.0 {
                    w += 1;
                }
            } else if b.y <= p.y && left < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

/// The rings with the shape on the left of each (nonzero winding), with no
/// points that repeat.
fn inside_left(rings: &[Vec<Point>], eps: f64) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    for ring in rings {
        let mut r: Vec<Point> = Vec::with_capacity(ring.len());
        for &p in ring {
            if r.last().is_none_or(|q: &Point| q.distance(p) > 1e-9) {
                r.push(p);
            }
        }
        while r.len() > 1 && r[0].distance(r[r.len() - 1]) <= 1e-9 {
            r.pop();
        }
        if r.len() < 3 {
            continue;
        }
        // The side of the longest segment that is in the shape.
        let k = (0..r.len())
            .max_by(|&i, &j| {
                let li = r[i].distance(r[(i + 1) % r.len()]);
                let lj = r[j].distance(r[(j + 1) % r.len()]);
                li.total_cmp(&lj)
            })
            .unwrap_or(0);
        let (a, b) = (r[k], r[(k + 1) % r.len()]);
        let left = (b - a).normalized().map_or(a, |u| a.midpoint(b) + u.perp() * eps);
        if winding(rings, left) == 0 {
            r.reverse();
        }
        out.push(r);
    }
    out
}

/// The point `span` along the outline from vertex `i` of `ring`, forward
/// or back.
fn away(ring: &[Point], i: usize, span: f64, forward: bool) -> Point {
    let n = ring.len();
    let step = if forward { 1 } else { n - 1 };
    let (mut j, mut left) = (i, span);
    for _ in 0..n {
        let next = (j + step) % n;
        let d = ring[j].distance(ring[next]);
        if d >= left {
            return ring[j].lerp(ring[next], left / d.max(1e-12));
        }
        left -= d;
        j = next;
    }
    ring[j]
}

/// The directions of the outline into vertex `i` of `ring` and out of it,
/// over `span_in` and `span_out`.
fn directions(ring: &[Point], i: usize, span_in: f64, span_out: f64) -> Option<(Point, Point)> {
    let p = ring[i];
    Some((
        (p - away(ring, i, span_in, false)).normalized()?,
        (away(ring, i, span_out, true) - p).normalized()?,
    ))
}

fn concave_corners(rings: &[Vec<Point>], span: f64) -> Vec<Concave> {
    let cos_max = emb_geom::math::cos(emb_geom::math::to_radians(CONCAVE_DEG));
    let mut found: Vec<(f64, Concave)> = Vec::new();
    for (ri, ring) in rings.iter().enumerate() {
        for (i, &p) in ring.iter().enumerate() {
            let Some((a, b)) = directions(ring, i, span, span) else {
                continue;
            };
            // A turn to the right goes into the shape (the shape is on the left).
            if a.cross(b) < 0.0 && a.dot(b) < cos_max {
                found.push((
                    a.dot(b),
                    Concave {
                        ring: ri,
                        at: i,
                        p,
                        d_in: a,
                        d_out: b,
                    },
                ));
            }
        }
    }
    // The sharpest point of each corner.
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<Concave> = Vec::new();
    for (_, c) in found {
        if out.iter().all(|k| k.p.distance(c.p) > span) {
            out.push(c);
        }
    }
    out
}

/// True if the segments `a`-`b` and `c`-`d` cross or touch away from the
/// ends `a` and `b`.
fn crosses(a: Point, b: Point, c: Point, d: Point, tol: f64) -> bool {
    match Point::crossing(a, b, c, d) {
        Some((u, _)) => {
            let l = a.distance(b).max(1e-12);
            u * l > tol && (1.0 - u) * l > tol
        }
        None => false,
    }
}

/// The cost of a bridge from corner `a` to corner `b` (it continues the
/// outline into `a` and out of `b`), or `None` if it is not a bridge.
fn bridge_cost(a: &Concave, b: &Concave, rings: &[Vec<Point>], width: f64) -> Option<f64> {
    bridge_cost_max(a, b, rings, width, BRIDGE_MAX)
}

/// [`bridge_cost`] with a bridge at most `max` times `width` long.
fn bridge_cost_max(a: &Concave, b: &Concave, rings: &[Vec<Point>], width: f64, max: f64) -> Option<f64> {
    let l = a.p.distance(b.p);
    if l > max * width || l < 1e-9 {
        return None;
    }
    let u = (b.p - a.p).normalized()?;
    let (align_a, align_b) = (angle(u, a.d_in), angle(u, b.d_out));
    if align_a > ALIGN_MAX_DEG || align_b > ALIGN_MAX_DEG {
        return None;
    }
    // The stroke that ends on the bridge is on its right (out of the shape
    // side of the outline that it continues), at both ends.
    let right = Point::new(u.y, -u.x);
    let sin_cut = emb_geom::math::sin(emb_geom::math::to_radians(CUT_MIN_DEG));
    if a.d_out.dot(right) < sin_cut || (-b.d_in).dot(right) < sin_cut {
        return None;
    }
    // In the shape, clear of the outline.
    if !clear_inside(a.p, b.p, rings, width) {
        return None;
    }
    Some(l / width + (align_a + align_b) / ALIGN_MAX_DEG)
}

/// The best soft bridge of the concave corner `c` on each side: a bridge to
/// a vertex of the outline where the outline turns into the shape over a
/// curve (the bracket of a serif, the inside of the shoulder of an "n"),
/// not at a corner. With `c` as its start (it continues the outline into
/// `c`), and with `c` as its end (it continues the outline out of `c`).
/// Each is (cost, start, end).
#[allow(clippy::type_complexity)]
fn soft_bridges(c: &Concave, rings: &[Vec<Point>], width: f64, span: f64) -> [Option<(f64, Concave, Concave)>; 2] {
    let far = SOFT_SPAN * width;
    let mut best: [Option<(f64, Concave, Concave)>; 2] = [None, None];
    for (r, ring) in rings.iter().enumerate() {
        for (k, &p) in ring.iter().enumerate() {
            let l = p.distance(c.p);
            if l <= span || l > SOFT_MAX * width {
                continue;
            }
            let (Some((near_in, near_out)), Some((far_in, far_out))) =
                (directions(ring, k, span, span), directions(ring, k, far, far))
            else {
                continue;
            };
            let v = |d_in, d_out| Concave {
                ring: r,
                at: k,
                p,
                d_in,
                d_out,
            };
            // The curve is before the end of the bridge, and after its start.
            let (end, start) = (v(far_in, near_out), v(near_in, far_out));
            for (side, a, b) in [(0, c, &end), (1, &start, c)] {
                if let Some(cost) = bridge_cost_max(a, b, rings, width, SOFT_MAX)
                    && best[side].as_ref().is_none_or(|x| cost < x.0)
                {
                    best[side] = Some((cost, *a, *b));
                }
            }
        }
    }
    // A short curve between two straight edges (the bracket of a serif)
    // goes with the stroke that goes through, where the stroke that ends
    // is square to it: the bridge ends where the curve starts on the edge
    // of the stroke that ends. Kept in the stroke that ends, the curve made
    // its end wide on one side, and its middle line bent to the curve (P21:
    // the stems of "n" and "r" in Bree Serif). Not for a stroke at a slant
    // (the arms of "Y"): the curve on its wide side is long, and the serif
    // took a part of the arm.
    let square = emb_geom::math::cos(emb_geom::math::to_radians(90.0 - BRACKET_SQUARE_DEG));
    for (side, found) in best.iter_mut().enumerate() {
        let Some((_, a, b)) = found else { continue };
        let curved = if side == 0 { *b } else { *a };
        let ring = &rings[curved.ring];
        let Some(at) = bracket_start(ring, curved.at, side == 1, width) else {
            continue;
        };
        let n = ring.len();
        let edge = ring[if side == 1 { (at + 1) % n } else { (at + n - 1) % n }];
        let (Some(e), Some(u)) = ((ring[at] - edge).normalized(), (b.p - a.p).normalized()) else {
            continue;
        };
        if e.dot(u).abs() > square {
            continue;
        }
        let moved = Concave {
            at,
            p: rings[curved.ring][at],
            ..curved
        };
        let (na, nb) = if side == 0 { (*a, moved) } else { (moved, *b) };
        if na.p.distance(nb.p) <= SOFT_MAX * width && clear_inside(na.p, nb.p, rings, width) {
            *a = na;
            *b = nb;
        }
    }
    best
}

/// The vertex where a short curve that ends at vertex `k` of `ring` starts
/// on a straight edge: along the ring from `k` (forward, or back), the near
/// end of the first segment at least [`BRACKET_EDGE`] of the typical width
/// long, if it comes within [`BRACKET_MAX`] of the width. `None` if the
/// outline curves on (the inside of the shoulder of an "n"), or if `k` is
/// on the edge already.
fn bracket_start(ring: &[Point], k: usize, forward: bool, width: f64) -> Option<usize> {
    let n = ring.len();
    let step = |j: usize| if forward { (j + 1) % n } else { (j + n - 1) % n };
    let (mut j, mut walked) = (k, 0.0);
    for _ in 0..n {
        let next = step(j);
        let len = ring[j].distance(ring[next]);
        if len >= BRACKET_EDGE * width {
            return (j != k).then_some(j);
        }
        walked += len;
        if walked > BRACKET_MAX * width {
            return None;
        }
        j = next;
    }
    None
}

/// The cut from `a` to `b` goes along a stroke: each of its points at a
/// quarter, the middle and three quarters is nearer the outline than
/// [`CUT_CLEAR`] of its length.
fn along_a_stroke(a: Point, b: Point, rings: &[Vec<Point>]) -> bool {
    let len = a.distance(b);
    [0.25, 0.5, 0.75].iter().all(|&t| {
        let q = a.lerp(b, t);
        rings
            .iter()
            .flat_map(|r| (0..r.len()).map(move |k| q.distance_to_segment(r[k], r[(k + 1) % r.len()])))
            .fold(f64::MAX, f64::min)
            < CUT_CLEAR * len
    })
}

/// The line from `a` to `b` is in the shape and clear of the outline (but
/// at its ends).
fn clear_inside(a: Point, b: Point, rings: &[Vec<Point>], width: f64) -> bool {
    let tol = BRIDGE_CLEAR * width;
    let crossing = rings
        .iter()
        .any(|r| (0..r.len()).any(|k| crosses(a, b, r[k], r[(k + 1) % r.len()], tol)));
    !crossing
        && [0.25, 0.5, 0.75].iter().all(|&t| {
            let q = a.lerp(b, t);
            winding(rings, q) != 0
                && rings
                    .iter()
                    .flat_map(|r| (0..r.len()).map(move |k| q.distance_to_segment(r[k], r[(k + 1) % r.len()])))
                    .fold(f64::MAX, f64::min)
                    >= tol
        })
}

/// The split of the shape in `rings` (nonzero winding), or `None` if it has
/// no bridges. With `notches`, a sharp concave corner with no bridge gets
/// a cut too (with `with.along` also a cut that goes along a stroke), with
/// `with.elbows` also the corner of an elbow, and with `with.soft` also a
/// bridge to a curve and the cut of a ball.
pub(crate) fn split(rings: &[Vec<Point>], cap_height: f64, notches: bool, with: Optional) -> Option<Split> {
    let Optional { soft, elbows, along } = with;
    let rings = inside_left(rings, 1e-4 * cap_height.max(1e-9));
    let area: f64 = rings
        .iter()
        .map(|r| emb_geom::region::ring_signed_area(r))
        .sum::<f64>()
        .abs();
    let perimeter: f64 = rings
        .iter()
        .map(|r| (0..r.len()).map(|k| r[k].distance(r[(k + 1) % r.len()])).sum::<f64>())
        .sum();
    if perimeter <= 0.0 || area <= 0.0 {
        return None;
    }
    // The typical width of a stroke: a long stroke of width w and length l
    // has an area of about w * l and an outline of about 2 * l.
    let width = 2.0 * area / perimeter;
    let corners = concave_corners(&rings, CORNER_SPAN * cap_height);
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..corners.len() {
        for j in 0..corners.len() {
            if i != j
                && let Some(c) = bridge_cost(&corners[i], &corners[j], &rings, width)
            {
                candidates.push((c, i, j));
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut used = vec![false; corners.len()];
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for (_, i, j) in candidates {
        if used[i] || used[j] {
            continue;
        }
        let (a, b) = (corners[i].p, corners[j].p);
        if chosen.iter().any(|&(k, l)| {
            crosses(a, b, corners[k].p, corners[l].p, 1e-9) || crosses(corners[k].p, corners[l].p, a, b, 1e-9)
        }) {
            continue;
        }
        used[i] = true;
        used[j] = true;
        chosen.push((i, j));
    }
    let mut links: Vec<Link> = chosen
        .iter()
        .map(|&(i, j)| Link {
            a: (corners[i].ring, Spot::Vertex(corners[i].at)),
            b: (corners[j].ring, Spot::Vertex(corners[j].at)),
            pa: corners[i].p,
            pb: corners[j].p,
            cut: false,
            soft: false,
            elbow: false,
            along: false,
            axis: None,
        })
        .collect();
    // A sharp concave corner with no bridge (the notch of a "V", the inner
    // corners of an "N" or a "Z") gets a cut along the line that halves
    // it, to the outline across: two strokes meet there at a point. Where
    // two strokes meet at a corner and both end there (an elbow: the
    // corners of a "Z", "7" or "4"), one side goes on to the outline
    // across (a butt joint).
    let cos_notch = emb_geom::math::cos(emb_geom::math::to_radians(NOTCH_DEG));
    let cos_elbow = emb_geom::math::cos(emb_geom::math::to_radians(ELBOW_DEG));
    let span = CORNER_SPAN * cap_height;
    for (i, c) in corners.iter().enumerate() {
        if !notches || used[i] || c.d_in.dot(c.d_out) > cos_elbow {
            continue;
        }
        let sharp = c.d_in.dot(c.d_out) <= cos_notch;
        let Some(link) = sharp
            .then(|| notch_cut(c, &rings, width).filter(|l| along || !l.along))
            .flatten()
            .or_else(|| elbows.then(|| elbow_cut(c, &rings, width, span)).flatten())
        else {
            continue;
        };
        if winding(&rings, link.pa.midpoint(link.pb)) == 0
            || links
                .iter()
                .any(|l| crosses(link.pa, link.pb, l.pa, l.pb, 1e-9) || crosses(l.pa, l.pb, link.pa, link.pb, 1e-9))
        {
            continue;
        }
        used[i] = true;
        links.push(link);
    }
    // In a font with serifs, a stroke often meets another at a sharp corner
    // on one side and over a curve on the other (a bracket of a serif, the
    // shoulder of an "n"): a concave corner with no bridge and no cut gets
    // a bridge to the curve, on each side where there is one, the cheapest
    // first.
    if notches && soft {
        let span = CORNER_SPAN * cap_height;
        let mut soft: Vec<(f64, usize, Concave, Concave)> = Vec::new();
        for (i, c) in corners.iter().enumerate() {
            if !used[i] {
                for (cost, a, b) in soft_bridges(c, &rings, width, span).into_iter().flatten() {
                    soft.push((cost, i, a, b));
                }
            }
        }
        soft.sort_by(|x, y| x.0.total_cmp(&y.0));
        // A bridge ends near another only at its own corner (the fork of a
        // "Y" has a bridge on each side): where a curve has a concave
        // corner too, the bridges of both made a thin piece between them.
        let near = SOFT_NEAR * width;
        for (_, i, a, b) in soft {
            let corner = corners[i].p;
            let other = if a.p.distance(corner) < 1e-9 { b.p } else { a.p };
            let close = |l: &Link| {
                [l.pa, l.pb]
                    .iter()
                    .any(|&e| e.distance(other) < near || (e.distance(corner) < near && e.distance(corner) > 1e-9))
            };
            if links
                .iter()
                .any(|l| close(l) || crosses(a.p, b.p, l.pa, l.pb, 1e-9) || crosses(l.pa, l.pb, a.p, b.p, 1e-9))
            {
                continue;
            }
            used[i] = true;
            links.push(Link {
                a: (a.ring, Spot::Vertex(a.at)),
                b: (b.ring, Spot::Vertex(b.at)),
                pa: a.p,
                pb: b.p,
                cut: false,
                soft: true,
                elbow: false,
                along: false,
                axis: None,
            });
        }
    }
    // A ball at the end of a stroke meets it at a concave corner that is
    // not sharp enough for a notch, with the round side of the ball on the
    // other side: a cut across the stroke at the corner makes the ball a
    // piece of its own (it is a soft link: the glyph keeps it only if its
    // columns sew no worse).
    if notches && soft {
        let cos_ball = emb_geom::math::cos(emb_geom::math::to_radians(BALL_MIN_DEG));
        for (i, c) in corners.iter().enumerate() {
            if used[i] || c.d_in.dot(c.d_out) > cos_ball {
                continue;
            }
            if let Some(link) = ball_cut(c, &rings, width, area, &links) {
                used[i] = true;
                links.push(link);
            }
        }
    }
    // A link that does not cut the shape (a face has it on both sides: it
    // joins the outline to a hole, as at the top of the bowl of a "P") is a
    // slit, not a cut: it goes, and the faces are made again.
    loop {
        if links.is_empty() {
            return None;
        }
        let (cut_rings, ends) = with_spots(&rings, &links);
        let cuts: Vec<bool> = links.iter().map(|l| l.cut).collect();
        match faces(&cut_rings, &ends, &cuts)? {
            Ok(mut pieces) => {
                // The ball of a cut is the piece just past the middle of
                // the cut.
                for l in &links {
                    let Some(u) = l.axis else { continue };
                    let probe = l.pa.midpoint(l.pb) + u * (0.25 * width);
                    if let Some(p) = pieces.iter_mut().find(|p| {
                        p.rings
                            .first()
                            .is_some_and(|r| emb_geom::region::ring_contains(r, probe))
                    }) {
                        p.axis = Some(u);
                    }
                }
                return Some(Split {
                    pieces,
                    #[cfg(test)]
                    bridges: links.iter().map(|l| (l.pa, l.pb)).collect(),
                    soft: links.iter().any(|l| l.soft),
                    elbows: links.iter().any(|l| l.elbow),
                    along: links.iter().any(|l| l.along),
                });
            }
            Err(slits) => {
                let mut k = 0;
                links.retain(|_| {
                    k += 1;
                    !slits.contains(&(k - 1))
                });
            }
        }
    }
}

/// The cut from the notch `c` along the line that halves it, to the
/// outline across, if it meets the outline across the stroke (not along
/// it).
fn notch_cut(c: &Concave, rings: &[Vec<Point>], width: f64) -> Option<Link> {
    let dir = (c.d_in - c.d_out).normalized()?;
    let reach = NOTCH_MAX * width;
    let far = c.p + dir * reach;
    let mut best: Option<(f64, usize, usize, f64)> = None;
    for (r, ring) in rings.iter().enumerate() {
        let n = ring.len();
        for k in 0..n {
            if r == c.ring && (k == c.at || (k + 1) % n == c.at) {
                continue;
            }
            if let Some((u, v)) = Point::crossing(c.p, far, ring[k], ring[(k + 1) % n])
                && u * reach > 1e-9
                && best.is_none_or(|b| u < b.0)
            {
                best = Some((u, r, k, v));
            }
        }
    }
    let (u, r, k, v) = best?;
    let h = c.p.lerp(far, u);
    let n = rings[r].len();
    let seg = rings[r][k].distance(rings[r][(k + 1) % n]);
    // A cut that meets the outline across at a small angle goes along a
    // stroke, not across the place where two strokes meet (the shoulder
    // of an "n": the stroke under the notch goes on).
    let along = (rings[r][(k + 1) % n] - rings[r][k])
        .normalized()
        .map_or(0.0, |d| angle(dir, d));
    if along.min(180.0 - along) < CUT_END_MIN_DEG {
        return None;
    }
    let tol = BRIDGE_CLEAR * width;
    let spot = if v * seg < tol {
        Spot::Vertex(k)
    } else if (1.0 - v) * seg < tol {
        Spot::Vertex((k + 1) % n)
    } else {
        Spot::On(k, v)
    };
    let end = match spot {
        Spot::Vertex(q) => rings[r][q],
        Spot::On(..) => h,
    };
    let along = along_a_stroke(c.p, end, rings);
    Some(Link {
        a: (c.ring, Spot::Vertex(c.at)),
        b: (r, spot),
        pa: c.p,
        pb: end,
        cut: true,
        soft: false,
        elbow: false,
        along,
        axis: None,
    })
}

/// The width of the stroke whose side goes through `from` in the
/// direction `side`: from `from`, square to the side into the shape, to
/// the outline across.
fn width_across(from: Point, side: Point, rings: &[Vec<Point>], reach: f64) -> Option<f64> {
    let normal = [side.perp(), side.perp() * -1.0]
        .into_iter()
        .find(|n| winding(rings, from + *n * (1e-3 * reach)) != 0)?;
    let far = from + normal * reach;
    let mut best: Option<f64> = None;
    for ring in rings {
        let n = ring.len();
        for k in 0..n {
            if let Some((u, _)) = Point::crossing(from, far, ring[k], ring[(k + 1) % n])
                && u * reach > 1e-6 * reach
                && best.is_none_or(|b| u < b)
            {
                best = Some(u);
            }
        }
    }
    best.map(|u| u * reach)
}

/// The cut of an elbow at the concave corner `c` (two strokes meet at a
/// corner and both end there: the corners of a "Z", "7" or "4"). The
/// outline has a convex corner across (the outer corner) at the width of
/// each stroke from the side of the other stroke at `c`. The cut goes on
/// along one side at `c` to the outline across (a butt joint: one stroke
/// goes on to the outer corner, and the other ends square to its side):
/// the shorter of the two. Not a mitre: the ends of a mitre are pointed
/// (half the angle of the corner), and satin stitches go out of them.
/// `span`: the span of the directions at the corners.
fn elbow_cut(c: &Concave, rings: &[Vec<Point>], width: f64, span: f64) -> Option<Link> {
    let dir = (c.d_in - c.d_out).normalized()?;
    let reach = ELBOW_MAX * width;
    // The width of each stroke near the corner.
    let w_in = width_across(
        away(&rings[c.ring], c.at, ELBOW_SPAN * width, false),
        c.d_in,
        rings,
        reach,
    )?;
    let w_out = width_across(
        away(&rings[c.ring], c.at, ELBOW_SPAN * width, true),
        c.d_out,
        rings,
        reach,
    )?;
    let off = |p: Point, side: Point| side.cross(p - c.p).abs();
    let cos_convex = emb_geom::math::cos(emb_geom::math::to_radians(ELBOW_CONVEX_DEG));
    let outer = rings.iter().enumerate().any(|(r, ring)| {
        ring.iter().enumerate().any(|(k, &p)| {
            let l = p.distance(c.p);
            if l <= span || l > reach {
                return false;
            }
            let near = |d: f64, w: f64| (d - w).abs() <= ELBOW_FIT * w;
            if (p - c.p).dot(dir) <= 0.0 || !near(off(p, c.d_in), w_in) || !near(off(p, c.d_out), w_out) {
                return false;
            }
            // A turn to the left goes out of the shape.
            directions(&rings[r], k, span, span).is_some_and(|(a, b)| a.cross(b) > 0.0 && a.dot(b) <= cos_convex)
        })
    });
    if !outer {
        return None;
    }
    let tol = BRIDGE_CLEAR * width;
    let mut best: Option<(f64, Link)> = None;
    for d in [c.d_in, c.d_out * -1.0] {
        let far = c.p + d * reach;
        let mut hit: Option<(f64, usize, usize, f64)> = None;
        for (r, ring) in rings.iter().enumerate() {
            let n = ring.len();
            for k in 0..n {
                if r == c.ring && (k == c.at || (k + 1) % n == c.at) {
                    continue;
                }
                if let Some((u, v)) = Point::crossing(c.p, far, ring[k], ring[(k + 1) % n])
                    && u * reach > tol
                    && hit.is_none_or(|h| u < h.0)
                {
                    hit = Some((u, r, k, v));
                }
            }
        }
        let Some((u, r, k, v)) = hit else { continue };
        let n = rings[r].len();
        let (p0, p1) = (rings[r][k], rings[r][(k + 1) % n]);
        let along = (p1 - p0).normalized().map_or(0.0, |e| angle(d, e));
        if along.min(180.0 - along) < CUT_END_MIN_DEG {
            continue;
        }
        let seg = p0.distance(p1);
        let spot = if v * seg < tol {
            Spot::Vertex(k)
        } else if (1.0 - v) * seg < tol {
            Spot::Vertex((k + 1) % n)
        } else {
            Spot::On(k, v)
        };
        let end = match spot {
            Spot::Vertex(q) => rings[r][q],
            Spot::On(..) => c.p.lerp(far, u),
        };
        if !clear_inside(c.p, end, rings, width) {
            continue;
        }
        let l = c.p.distance(end);
        if best.as_ref().is_none_or(|b| l < b.0) {
            best = Some((
                l,
                Link {
                    a: (c.ring, Spot::Vertex(c.at)),
                    b: (r, spot),
                    pa: c.p,
                    pb: end,
                    cut: true,
                    soft: false,
                    elbow: true,
                    along: false,
                    axis: None,
                },
            ));
        }
    }
    best.map(|b| b.1)
}

/// The cut of a ball at the concave corner `c` (see [`BALL_MIN_DEG`]): from
/// the corner, square to the side of the stroke, to the outline across the
/// stroke. The ball is the outline from the corner the other way, round to
/// the end of the cut; it must be round ([`BALL_ROUND`]), wider than the
/// stroke ([`BALL_WIDER`]) and a small part of the glyph ([`BALL_PART`]).
/// `area`: the area of the glyph; `width`: the typical width.
fn ball_cut(c: &Concave, rings: &[Vec<Point>], width: f64, area: f64, links: &[Link]) -> Option<Link> {
    let ring = &rings[c.ring];
    let n = ring.len();
    if n < 4 {
        return None;
    }
    let reach = BALL_CUT_MAX * width;
    let mut best: Option<(f64, Link)> = None;
    // The side of the stroke comes into the corner (the ball is after it
    // on the ring) or goes out of it (the ball is before it).
    for (side, ball_after) in [(c.d_in, true), (c.d_out, false)] {
        for normal in [side.perp(), side.perp() * -1.0] {
            if winding(rings, c.p + normal * (0.05 * width)) == 0 {
                continue;
            }
            let far = c.p + normal * reach;
            let mut hit: Option<(f64, usize, f64)> = None;
            for k in 0..n {
                if k == c.at || (k + 1) % n == c.at {
                    continue;
                }
                if let Some((u, v)) = Point::crossing(c.p, far, ring[k], ring[(k + 1) % n])
                    && u * reach > 1e-9
                    && hit.is_none_or(|h| u < h.0)
                {
                    hit = Some((u, k, v));
                }
            }
            let Some((u, k, v)) = hit else { continue };
            // The cut must not leave the shape or cross another ring.
            let end = c.p.lerp(far, u);
            if winding(rings, c.p.midpoint(end)) == 0
                || rings.iter().enumerate().any(|(r, other)| {
                    r != c.ring
                        && (0..other.len())
                            .any(|q| Point::crossing(c.p, end, other[q], other[(q + 1) % other.len()]).is_some())
                })
                || links
                    .iter()
                    .any(|l| crosses(c.p, end, l.pa, l.pb, 1e-9) || crosses(l.pa, l.pb, c.p, end, 1e-9))
            {
                continue;
            }
            // The outline of the ball: from the corner, round to the cut.
            let mut ball = vec![c.p];
            let mut q = c.at;
            loop {
                q = if ball_after { (q + 1) % n } else { (q + n - 1) % n };
                ball.push(ring[q]);
                let last = if ball_after { q == k } else { q == (k + 1) % n };
                if last || ball.len() > n {
                    break;
                }
            }
            ball.push(end);
            let a = emb_geom::region::ring_signed_area(&ball).abs();
            let p: f64 = (0..ball.len())
                .map(|j| ball[j].distance(ball[(j + 1) % ball.len()]))
                .sum();
            let round = 4.0 * std::f64::consts::PI * a / (p * p).max(1e-12);
            let wide = 2.0 * (a / std::f64::consts::PI).sqrt();
            if round < BALL_ROUND || a > BALL_PART * area || wide < BALL_WIDER * c.p.distance(end) {
                continue;
            }
            let seg = ring[k].distance(ring[(k + 1) % n]);
            let tol = BRIDGE_CLEAR * width;
            let spot = if v * seg < tol {
                Spot::Vertex(k)
            } else if (1.0 - v) * seg < tol {
                Spot::Vertex((k + 1) % n)
            } else {
                Spot::On(k, v)
            };
            let pb = match spot {
                Spot::Vertex(q) => ring[q],
                Spot::On(..) => end,
            };
            // Square to the cut, into the ball (to the middle of its
            // outline).
            let middle = ball.iter().fold(Point::ZERO, |s, &q| s + q) * (1.0 / ball.len() as f64);
            let Some(mut into) = (pb - c.p).normalized().map(|d| d.perp()) else {
                continue;
            };
            if into.dot(middle - c.p.midpoint(pb)) < 0.0 {
                into = into * -1.0;
            }
            if best.as_ref().is_none_or(|b| round > b.0) {
                best = Some((
                    round,
                    Link {
                        a: (c.ring, Spot::Vertex(c.at)),
                        b: (c.ring, spot),
                        pa: c.p,
                        pb,
                        cut: true,
                        soft: true,
                        elbow: false,
                        along: false,
                        axis: Some(into),
                    },
                ));
            }
        }
    }
    best.map(|b| b.1)
}

/// A place on a ring: a vertex, or a point on the segment after a vertex
/// (the part of the segment).
#[derive(Debug, Clone, Copy)]
enum Spot {
    Vertex(usize),
    On(usize, f64),
}

/// A link between two places of the outline: a bridge, a cut from a notch
/// or at an elbow, a bridge to a curve, or the cut of a ball.
#[derive(Debug, Clone, Copy)]
struct Link {
    a: (usize, Spot),
    b: (usize, Spot),
    pa: Point,
    pb: Point,
    cut: bool,
    soft: bool,
    /// The cut of an elbow.
    elbow: bool,
    /// A cut from a notch that goes along a stroke ([`CUT_CLEAR`]).
    along: bool,
    /// The cut of a ball: the direction from the cut into the ball.
    axis: Option<Point>,
}

/// The rings with a vertex at each end of a link that is on a segment, and
/// the ends of the links as (ring, vertex).
#[allow(clippy::type_complexity)]
fn with_spots(rings: &[Vec<Point>], links: &[Link]) -> (Vec<Vec<Point>>, Vec<((usize, usize), (usize, usize))>) {
    let mut inserts: Vec<(usize, usize, f64)> = Vec::new();
    for l in links {
        for (r, spot) in [l.a, l.b] {
            if let Spot::On(k, v) = spot {
                inserts.push((r, k, v));
            }
        }
    }
    let mut out = Vec::with_capacity(rings.len());
    // For each ring: the new index of each old vertex, and of each insert.
    let mut index: Vec<Vec<usize>> = Vec::with_capacity(rings.len());
    let mut placed: Vec<((usize, usize, u64), usize)> = Vec::new();
    for (r, ring) in rings.iter().enumerate() {
        let mut pts = Vec::with_capacity(ring.len());
        let mut idx = Vec::with_capacity(ring.len());
        for k in 0..ring.len() {
            idx.push(pts.len());
            pts.push(ring[k]);
            let mut here: Vec<f64> = inserts.iter().filter(|i| i.0 == r && i.1 == k).map(|i| i.2).collect();
            here.sort_by(f64::total_cmp);
            for v in here {
                placed.push(((r, k, v.to_bits()), pts.len()));
                pts.push(ring[k].lerp(ring[(k + 1) % ring.len()], v));
            }
        }
        out.push(pts);
        index.push(idx);
    }
    let at = |r: usize, spot: Spot| -> (usize, usize) {
        match spot {
            Spot::Vertex(k) => (r, index[r][k]),
            Spot::On(k, v) => (
                r,
                placed
                    .iter()
                    .find(|p| p.0 == (r, k, v.to_bits()))
                    .map_or(index[r][k], |p| p.1),
            ),
        }
    };
    let ends = links.iter().map(|l| (at(l.a.0, l.a.1), at(l.b.0, l.b.1))).collect();
    (out, ends)
}

/// The pieces of the shape cut along the bridges: the faces of the outline
/// and the bridges, with the shape on the left. `Err` with the bridges that
/// a face has on both sides (slits).
#[allow(clippy::type_complexity)]
fn faces(
    rings: &[Vec<Point>],
    bridges: &[((usize, usize), (usize, usize))],
    cuts: &[bool],
) -> Option<Result<Vec<Piece>, Vec<usize>>> {
    // Vertices: (ring, index) as one number.
    let start: Vec<usize> = rings
        .iter()
        .scan(0, |s, r| {
            let v = *s;
            *s += r.len();
            Some(v)
        })
        .collect();
    let id = |(r, k): (usize, usize)| start[r] + k;
    let total = start.last().map_or(0, |s| s + rings.last().map_or(0, Vec::len));
    let mut at: Vec<Point> = Vec::with_capacity(total);
    let mut next: Vec<usize> = Vec::with_capacity(total);
    for (r, ring) in rings.iter().enumerate() {
        for k in 0..ring.len() {
            at.push(ring[k]);
            next.push(id((r, (k + 1) % ring.len())));
        }
    }
    // Directed edges: (from, to, the bridge or NONE).
    const NONE: usize = usize::MAX;
    let mut edges: Vec<(usize, usize, usize)> = (0..total).map(|v| (v, next[v], NONE)).collect();
    for (b, &(p, q)) in bridges.iter().enumerate() {
        edges.push((id(p), id(q), b));
        edges.push((id(q), id(p), b));
    }
    let mut out_of: Vec<Vec<usize>> = vec![Vec::new(); total];
    for (e, &(from, _, _)) in edges.iter().enumerate() {
        out_of[from].push(e);
    }
    let mut used = vec![false; edges.len()];
    // Each cycle: its vertices and, for each edge, its bridge.
    let mut cycles: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
    for first in 0..edges.len() {
        if used[first] {
            continue;
        }
        let (mut vs, mut bs) = (Vec::new(), Vec::new());
        let mut e = first;
        for _ in 0..=edges.len() {
            used[e] = true;
            let (from, to, bridge) = edges[e];
            vs.push(from);
            bs.push(bridge);
            let d = at[to] - at[from];
            // The next edge: the largest turn to the left (the face on the left).
            let back = |f: usize| edges[f].1 == from && edges[f].2 == bridge && bridge != NONE;
            let pick = out_of[to].iter().copied().filter(|&f| !back(f)).max_by(|&f, &g| {
                let turn = |f: usize| {
                    let o = at[edges[f].1] - at[to];
                    emb_geom::math::atan2(d.cross(o), d.dot(o))
                };
                turn(f).total_cmp(&turn(g))
            })?;
            if pick == first {
                break;
            }
            if used[pick] {
                return None;
            }
            e = pick;
        }
        cycles.push((vs, bs));
    }
    let mut slits: Vec<usize> = Vec::new();
    for (_, bs) in &cycles {
        for (k, &b) in bs.iter().enumerate() {
            if b != NONE && bs[k + 1..].contains(&b) && !slits.contains(&b) {
                slits.push(b);
            }
        }
    }
    if !slits.is_empty() {
        return Some(Err(slits));
    }
    // Outer rings (positive area) and holes (negative).
    let ring_of = |vs: &[usize]| -> Vec<Point> { vs.iter().map(|&v| at[v]).collect() };
    let mut outers: Vec<(Vec<Point>, Vec<usize>, Vec<usize>, f64)> = Vec::new();
    let mut holes: Vec<(Vec<Point>, Vec<usize>, Vec<usize>)> = Vec::new();
    for (vs, bs) in cycles {
        let r = ring_of(&vs);
        let a = emb_geom::region::ring_signed_area(&r);
        if a > 0.0 {
            outers.push((r, vs, bs, a));
        } else if a < 0.0 {
            holes.push((r, vs, bs));
        }
    }
    let mut pieces: Vec<Piece> = outers
        .iter()
        .map(|(r, vs, bs, a)| {
            let (ring, joints) = moved_ends(r, vs, bs, &at, cuts, rings);
            Piece {
                rings: vec![ring],
                area: *a,
                axis: None,
                joints,
            }
        })
        .collect();
    for (r, vs, bs) in &holes {
        // The smallest outer ring round a point next to the hole in the shape.
        let (a, b) = (r[0], r[1 % r.len()]);
        let probe = (b - a)
            .normalized()
            .map_or(a, |u| a.midpoint(b) + u.perp() * 1e-6 * (1.0 + a.length()));
        let owner = outers
            .iter()
            .enumerate()
            .filter(|(_, o)| emb_geom::region::ring_contains(&o.0, probe))
            .min_by(|x, y| x.1.3.total_cmp(&y.1.3))
            .map(|(i, _)| i);
        if let Some(i) = owner {
            let (ring, joints) = moved_ends(r, vs, bs, &at, cuts, rings);
            pieces[i].rings.push(ring);
            pieces[i].joints.extend(joints);
        }
    }
    Some(Ok(pieces))
}

/// The ring of a face with each bridge that is an end of the piece (and
/// each cut from a notch) moved out of it by [`BRIDGE_OVERLAP`] of its
/// length (into the piece on the other side); an end that would go out of
/// the shape in `rings` stays. Also the joints of the piece: its ends at
/// bridges (not at cuts), with the depth to the middle of the stroke on
/// the other side.
fn moved_ends(
    r: &[Point],
    vs: &[usize],
    bs: &[usize],
    at: &[Point],
    cuts: &[bool],
    rings: &[Vec<Point>],
) -> (Vec<Point>, Vec<Joint>) {
    let n = vs.len();
    let cos_turn = emb_geom::math::cos(emb_geom::math::to_radians(END_TURN_DEG));
    let mut out = Vec::with_capacity(n + 4);
    let mut joints = Vec::new();
    for k in 0..n {
        out.push(r[k]);
        if bs[k] == usize::MAX {
            continue;
        }
        let (p, q) = (at[vs[k]], at[vs[(k + 1) % n]]);
        let (prev, after) = (at[vs[(k + n - 1) % n]], at[vs[(k + 2) % n]]);
        let (Some(u), Some(d0), Some(d1)) = ((q - p).normalized(), (p - prev).normalized(), (after - q).normalized())
        else {
            continue;
        };
        if !cuts[bs[k]] && (d0.dot(u) > cos_turn || u.dot(d1) > cos_turn) {
            continue;
        }
        let normal = Point::new(u.y, -u.x);
        if !cuts[bs[k]] || crate::stroke::overlap_trial_cuts() {
            // The width of the stroke on the other side: from the middle of
            // the bridge to where the shape ends, in small steps.
            let (m, len) = (p.midpoint(q), p.distance(q));
            let step = 0.02 * len;
            let mut across = 0.0;
            while across < 6.0 * len && winding(rings, m + normal * (across + step)) != 0 {
                across += step;
            }
            if across > step {
                joints.push(Joint {
                    a: p,
                    b: q,
                    into: normal,
                    depth: 0.5 * across,
                });
            }
        }
        let shift = normal * (BRIDGE_OVERLAP * p.distance(q));
        // An end that would go out of the shape stays. Where a cut meets
        // the outline at a slant, the moved end went out of the letter, and
        // the column of the piece with it (P21 Milestone 14: the top of the
        // diagonal of an "N" went 1.6 mm out of the stem). Measured: also
        // less area with two layers than an end taken back to the outline.
        let moved = |e: Point| if winding(rings, e + shift) != 0 { e + shift } else { e };
        out.push(moved(p));
        out.push(moved(q));
    }
    (out, joints)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(pts: &[(f64, f64)]) -> Vec<Point> {
        pts.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    /// A stroke that ends in a ball (the terminal of an "r" in a serif
    /// font): the ball is a piece of its own, cut across the stroke at the
    /// notch where they meet (P21 Milestone 11).
    #[test]
    fn a_ball_is_cut_from_its_stroke() {
        use emb_geom::region::{Region, ring_contains, union};
        // The top side of the stroke goes on round the ball; its other side
        // meets the ball at a corner of 90 degrees.
        let bar = Region::new(
            poly(&[(-2000.0, -100.0), (1000.0, -100.0), (1000.0, 100.0), (-2000.0, 100.0)]),
            Vec::new(),
        );
        let (cx, cy, r) = (1000.0, 100.0, 200.0);
        let circle: Vec<Point> = (0..64)
            .map(|k| {
                let t = std::f64::consts::TAU * k as f64 / 64.0;
                Point::new(cx + r * t.cos(), cy + r * t.sin())
            })
            .collect();
        let shape = union(&[bar], &[Region::new(circle, Vec::new())]);
        assert_eq!(shape.len(), 1);
        let s = split(
            std::slice::from_ref(&shape[0].outer),
            1000.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert!(s.soft, "{s:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        let ball = s.pieces.iter().find(|p| {
            p.rings
                .iter()
                .any(|ring| ring_contains(ring, Point::new(cx, cy + 50.0)))
        });
        let ball = ball.expect("a piece with the ball");
        let disc = std::f64::consts::PI * r * r;
        assert!(ball.area > 0.8 * disc && ball.area < 1.5 * disc, "{} {disc}", ball.area);
        // The cut is across the stroke, at the corner.
        let (a, b) = s.bridges[0];
        assert!((a.x - 800.0).abs() < 1.0 && (b.x - 800.0).abs() < 1.0, "{a:?} {b:?}");
        // The column of the ball goes on in the direction of the stroke,
        // away from it, so its stitches lie as the stitches of the stroke.
        let u = ball.axis.expect("the direction of the ball");
        assert!(u.x > 0.99, "{u:?}");
        // With no soft links, the stroke and the ball are one piece.
        assert!(
            split(
                std::slice::from_ref(&shape[0].outer),
                1000.0,
                true,
                Optional {
                    soft: false,
                    elbows: true,
                    along: true
                }
            )
            .is_none_or(|p| p.pieces.len() == 1)
        );
    }

    #[test]
    fn a_t_is_a_bar_and_a_stem() {
        // A "T": a bar 600 wide and 100 high on a stem 100 wide.
        let t = poly(&[
            (250.0, 0.0),
            (350.0, 0.0),
            (350.0, 500.0),
            (600.0, 500.0),
            (600.0, 600.0),
            (0.0, 600.0),
            (0.0, 500.0),
            (250.0, 500.0),
        ]);
        let s = split(
            &[t],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert_eq!(s.bridges.len(), 1, "{s:?}");
        let (a, b) = s.bridges[0];
        assert!((a.y - 500.0).abs() < 1e-9 && (b.y - 500.0).abs() < 1e-9, "{a:?} {b:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        let mut areas: Vec<f64> = s.pieces.iter().map(|p| p.area).collect();
        areas.sort_by(f64::total_cmp);
        assert!(
            (areas[0] - 50_000.0).abs() < 1e-6 && (areas[1] - 60_000.0).abs() < 1e-6,
            "{areas:?}"
        );
        // The stem goes into the bar; the bar keeps its straight edge.
        let stem = s.pieces.iter().find(|p| (p.area - 50_000.0).abs() < 1e-6).unwrap();
        let top = stem.rings[0].iter().map(|p| p.y).fold(f64::MIN, f64::max);
        assert!((top - 510.0).abs() < 1e-6, "{top}");
        let bar = s.pieces.iter().find(|p| (p.area - 60_000.0).abs() < 1e-6).unwrap();
        let bottom = bar.rings[0].iter().map(|p| p.y).fold(f64::MAX, f64::min);
        assert!((bottom - 500.0).abs() < 1e-6, "{bottom}");
    }

    #[test]
    fn a_cross_keeps_one_stroke_whole() {
        let plus = poly(&[
            (450.0, 0.0),
            (550.0, 0.0),
            (550.0, 450.0),
            (1000.0, 450.0),
            (1000.0, 530.0),
            (550.0, 530.0),
            (550.0, 1000.0),
            (450.0, 1000.0),
            (450.0, 530.0),
            (0.0, 530.0),
            (0.0, 450.0),
            (450.0, 450.0),
        ]);
        let s = split(
            &[plus],
            1000.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert_eq!(s.bridges.len(), 2, "{s:?}");
        assert_eq!(s.pieces.len(), 3, "{s:?}");
        // The cut is across the thinner stroke (the shorter bridges): the
        // wider stroke stays whole.
        let whole = s.pieces.iter().map(|p| p.area).fold(0.0, f64::max);
        assert!((whole - 100_000.0).abs() < 1e-6, "{s:?}");
    }

    #[test]
    fn a_bridge_that_does_not_cut_the_shape_goes() {
        // A "P" with a square bowl: the bowl meets the stem at an "L" at
        // the top (no bridge) and at a "T" at the bottom (a bridge from the
        // outline to the hole: a slit).
        let outer = poly(&[
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 300.0),
            (400.0, 300.0),
            (400.0, 700.0),
            (0.0, 700.0),
        ]);
        let hole = poly(&[(100.0, 400.0), (100.0, 600.0), (300.0, 600.0), (300.0, 400.0)]);
        assert!(
            split(
                &[outer, hole],
                700.0,
                true,
                Optional {
                    soft: true,
                    elbows: false,
                    along: true
                }
            )
            .is_none()
        );
    }

    #[test]
    fn a_v_is_cut_at_its_notch() {
        let v = poly(&[
            (0.0, 600.0),
            (100.0, 600.0),
            (250.0, 150.0),
            (400.0, 600.0),
            (500.0, 600.0),
            (300.0, 0.0),
            (200.0, 0.0),
        ]);
        let s = split(
            &[v],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert_eq!(s.bridges.len(), 1, "{s:?}");
        let (a, b) = s.bridges[0];
        assert!(
            a.distance(Point::new(250.0, 150.0)) < 1e-9 && b.distance(Point::new(250.0, 0.0)) < 1e-9,
            "{a:?} {b:?}"
        );
        assert_eq!(s.pieces.len(), 2, "{s:?}");
    }

    /// A stem under a slab serif: a sharp corner on its left, a curve (a
    /// bracket) on its right.
    fn stem_with_bracket() -> Vec<Point> {
        let mut r = poly(&[(200.0, 0.0), (300.0, 0.0), (300.0, 460.0)]);
        for k in 1..8 {
            let t = emb_geom::math::to_radians(180.0 - 90.0 * f64::from(k) / 8.0);
            r.push(Point::new(
                340.0 + 40.0 * emb_geom::math::cos(t),
                460.0 + 40.0 * emb_geom::math::sin(t),
            ));
        }
        r.extend(poly(&[
            (340.0, 500.0),
            (500.0, 500.0),
            (500.0, 600.0),
            (0.0, 600.0),
            (0.0, 500.0),
            (200.0, 500.0),
        ]));
        r
    }

    #[test]
    fn a_stem_with_a_bracket_gets_a_bridge_to_the_curve() {
        let s = split(
            &[stem_with_bracket()],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert!(s.soft, "{s:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        // The stem is square to the serif: the bracket goes with the serif,
        // and the bridge ends where the curve starts on the stem.
        let (a, b) = s.bridges[0];
        let ends = [a, b];
        assert!(
            ends.iter().any(|p| p.distance(Point::new(200.0, 500.0)) < 1e-9),
            "{ends:?}"
        );
        assert!(
            ends.iter().any(|p| p.distance(Point::new(300.0, 460.0)) < 1e-9),
            "{ends:?}"
        );
        // With no bridges to curves, the shape has no split.
        assert!(
            split(
                &[stem_with_bracket()],
                600.0,
                true,
                Optional {
                    soft: false,
                    elbows: true,
                    along: true
                }
            )
            .is_none()
        );
    }

    #[test]
    fn a_fork_gets_a_bridge_on_each_side() {
        // A "Y": two arms meet over a stem, at a notch of less than
        // `NOTCH_DEG`.
        let y = poly(&[
            (250.0, 0.0),
            (350.0, 0.0),
            (350.0, 300.0),
            (520.0, 600.0),
            (420.0, 600.0),
            (300.0, 390.0),
            (180.0, 600.0),
            (80.0, 600.0),
            (250.0, 300.0),
        ]);
        let s = split(
            &[y],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        let notch = Point::new(300.0, 390.0);
        let at_notch = s
            .bridges
            .iter()
            .filter(|(a, b)| a.distance(notch) < 1e-9 || b.distance(notch) < 1e-9)
            .count();
        assert_eq!(at_notch, 2, "{s:?}");
        assert_eq!(s.pieces.len(), 3, "{s:?}");
    }

    #[test]
    fn an_l_and_a_ring_have_no_bridge() {
        let l = poly(&[
            (0.0, 0.0),
            (500.0, 0.0),
            (500.0, 100.0),
            (100.0, 100.0),
            (100.0, 600.0),
            (0.0, 600.0),
        ]);
        assert!(
            split(
                &[l],
                600.0,
                true,
                Optional {
                    soft: true,
                    elbows: false,
                    along: true
                }
            )
            .is_none()
        );
        let outer = poly(&[(0.0, 0.0), (600.0, 0.0), (600.0, 600.0), (0.0, 600.0)]);
        let hole = poly(&[(100.0, 100.0), (100.0, 500.0), (500.0, 500.0), (500.0, 100.0)]);
        assert!(
            split(
                &[outer, hole],
                600.0,
                true,
                Optional {
                    soft: true,
                    elbows: false,
                    along: true
                }
            )
            .is_none()
        );
    }

    /// An "L": the stem and the bar meet at a corner and both end there (an
    /// elbow). One side at the concave corner goes on to the outline across
    /// (a butt joint), square to it, so each piece is a rectangle (P21
    /// Milestone 13).
    #[test]
    fn an_elbow_gets_a_butt_joint() {
        let l = poly(&[
            (0.0, 0.0),
            (500.0, 0.0),
            (500.0, 100.0),
            (100.0, 100.0),
            (100.0, 600.0),
            (0.0, 600.0),
        ]);
        let s = split(
            &[l],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        )
        .expect("a split");
        assert!(s.elbows, "{s:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        let (a, b) = s.bridges[0];
        let inner = Point::new(100.0, 100.0);
        let (from, to) = if a.distance(inner) < b.distance(inner) {
            (a, b)
        } else {
            (b, a)
        };
        assert!(from.distance(inner) < 1e-6, "{s:?}");
        let butt = [Point::new(100.0, 0.0), Point::new(0.0, 100.0)];
        assert!(butt.iter().any(|p| p.distance(to) < 1e-6), "{s:?}");
        // The pieces are the stem (100 x 600) and the rest of the bar (400
        // x 100), or the bar (500 x 100) and the rest of the stem (100 x
        // 500).
        let mut areas: Vec<f64> = s.pieces.iter().map(|p| p.area).collect();
        areas.sort_by(f64::total_cmp);
        let rects = [[40_000.0, 60_000.0], [50_000.0, 50_000.0]];
        assert!(
            rects
                .iter()
                .any(|r| (areas[0] - r[0]).abs() < 1.0 && (areas[1] - r[1]).abs() < 1.0),
            "{s:?}"
        );
    }

    /// The outer corner of an elbow is at the width of each stroke from the
    /// other stroke: with the corner cut off (a chamfer), the shape has no
    /// elbow; and the corner of a serif is not an elbow of the stem.
    #[test]
    fn an_elbow_needs_its_outer_corner() {
        let chamfered = poly(&[
            (40.0, 0.0),
            (500.0, 0.0),
            (500.0, 100.0),
            (100.0, 100.0),
            (100.0, 600.0),
            (0.0, 600.0),
            (0.0, 40.0),
        ]);
        assert!(
            split(
                &[chamfered],
                600.0,
                true,
                Optional {
                    soft: true,
                    elbows: true,
                    along: true
                }
            )
            .is_none()
        );
        // A stem on a long foot that goes 150 past it on the left (a
        // serif): the corner of the foot is not at the width of the stem
        // from its side.
        let foot = poly(&[
            (0.0, 0.0),
            (650.0, 0.0),
            (650.0, 100.0),
            (250.0, 100.0),
            (250.0, 600.0),
            (150.0, 600.0),
            (150.0, 100.0),
            (0.0, 100.0),
        ]);
        let s = split(
            &[foot],
            600.0,
            true,
            Optional {
                soft: true,
                elbows: true,
                along: true,
            },
        );
        assert!(s.as_ref().is_none_or(|s| !s.elbows), "{s:?}");
    }

    /// A heavy "n" (font units, Y down): the notch on top of the stem, and a
    /// narrow counter open at the bottom. The line that halves the notch
    /// goes down the stem beside the counter, along the stroke: that cut is
    /// optional (P21 Milestone 14).
    #[test]
    fn a_cut_along_a_stroke_is_optional() {
        let n = poly(&[
            (50.0, 0.0),
            (50.0, -1200.0),
            (600.0, -1200.0),
            (640.0, -960.0),
            (690.0, -1100.0),
            (1000.0, -1240.0),
            (1400.0, -1000.0),
            (1400.0, 0.0),
            (812.0, 0.0),
            (812.0, -600.0),
            (730.0, -754.0),
            (648.0, -600.0),
            (648.0, 0.0),
        ]);
        let notch = Point::new(640.0, -960.0);
        let from_notch = |s: &Option<Split>| {
            s.as_ref().is_some_and(|s| {
                s.bridges
                    .iter()
                    .any(|(a, b)| a.distance(notch) < 1e-6 || b.distance(notch) < 1e-6)
            })
        };
        let all = Optional {
            soft: true,
            elbows: true,
            along: true,
        };
        let with = split(std::slice::from_ref(&n), 1240.0, true, all);
        assert!(from_notch(&with), "{with:?}");
        assert!(with.as_ref().is_some_and(|s| s.along), "{with:?}");
        let without = split(&[n], 1240.0, true, Optional { along: false, ..all });
        assert!(!from_notch(&without), "{without:?}");
    }

    /// The pieces of an "N" (the outline of Funky Slab, Y down): the end of
    /// each piece at a cut goes a little into the piece on the other side,
    /// but not out of the letter. The cut from the notch at the foot of the
    /// stem meets the left side at a slant, and the top of the diagonal went
    /// 1.6 mm out of the stem (P21 Milestone 14).
    #[test]
    fn the_pieces_stay_in_the_letter() {
        let n = poly(&[
            (123.0, -1597.0),
            (285.0, -1597.0),
            (1157.0, -975.0),
            (1157.0, -1597.0),
            (1720.0, -1597.0),
            (1720.0, 0.0),
            (1505.0, 0.0),
            (684.0, -549.0),
            (684.0, 0.0),
            (123.0, 0.0),
        ]);
        let s = split(std::slice::from_ref(&n), 1597.0, true, Optional::ALL).expect("a split");
        assert!(s.pieces.len() >= 2, "{s:?}");
        let near_outline = |p: Point| (0..n.len()).any(|k| p.distance_to_segment(n[k], n[(k + 1) % n.len()]) < 1e-6);
        for piece in &s.pieces {
            for &p in piece.rings.iter().flatten() {
                assert!(
                    winding(std::slice::from_ref(&n), p) != 0 || near_outline(p),
                    "{p:?} in {s:?}"
                );
            }
        }
    }

    #[test]
    fn the_middle_bar_of_a_box_with_two_holes_is_a_piece() {
        // "日": a frame with a bar in the middle (two holes).
        let outer = poly(&[(0.0, 0.0), (500.0, 0.0), (500.0, 900.0), (0.0, 900.0)]);
        let low = poly(&[(100.0, 100.0), (100.0, 400.0), (400.0, 400.0), (400.0, 100.0)]);
        let high = poly(&[(100.0, 500.0), (100.0, 800.0), (400.0, 800.0), (400.0, 500.0)]);
        let s = split(
            &[outer, low, high],
            900.0,
            true,
            Optional {
                soft: true,
                elbows: false,
                along: true,
            },
        )
        .expect("a split");
        assert_eq!(s.bridges.len(), 2, "{s:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        // The frame has one hole (the two holes and the bar).
        let frame = s.pieces.iter().max_by(|a, b| a.area.total_cmp(&b.area)).unwrap();
        assert_eq!(frame.rings.len(), 2, "{s:?}");
    }
}
