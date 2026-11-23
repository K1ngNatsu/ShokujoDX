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
}

/// The split of a shape: its pieces and its bridges.
#[derive(Debug, Clone, Default)]
pub(crate) struct Split {
    pub pieces: Vec<Piece>,
    /// The bridges and the cuts.
    #[cfg_attr(not(test), allow(dead_code))]
    pub bridges: Vec<(Point, Point)>,
    /// Some bridges go to a curve (soft bridges).
    pub soft: bool,
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
    let tol = BRIDGE_CLEAR * width;
    for r in rings {
        for k in 0..r.len() {
            let (c, d) = (r[k], r[(k + 1) % r.len()]);
            if crosses(a.p, b.p, c, d, tol) {
                return None;
            }
        }
    }
    for t in [0.25, 0.5, 0.75] {
        let q = a.p.lerp(b.p, t);
        if winding(rings, q) == 0 {
            return None;
        }
        let near = rings
            .iter()
            .flat_map(|r| (0..r.len()).map(move |k| q.distance_to_segment(r[k], r[(k + 1) % r.len()])))
            .fold(f64::MAX, f64::min);
        if near < tol {
            return None;
        }
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
/// a cut too, and with `soft` also a bridge to a curve.
pub(crate) fn split(rings: &[Vec<Point>], cap_height: f64, notches: bool, soft: bool) -> Option<Split> {
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
        })
        .collect();
    // A sharp concave corner with no bridge (the notch of a "V", the inner
    // corners of an "N" or a "Z") gets a cut along the line that halves
    // it, to the outline across: two strokes meet there at a point.
    let cos_notch = emb_geom::math::cos(emb_geom::math::to_radians(NOTCH_DEG));
    for (i, c) in corners.iter().enumerate() {
        if !notches || used[i] || c.d_in.dot(c.d_out) > cos_notch {
            continue;
        }
        let Some(dir) = (c.d_in - c.d_out).normalized() else {
            continue;
        };
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
        let Some((u, r, k, v)) = best else { continue };
        let h = c.p.lerp(far, u);
        let n = rings[r].len();
        let seg = rings[r][k].distance(rings[r][(k + 1) % n]);
        // A cut that meets the outline across at a small angle goes along
        // a stroke, not across the place where two strokes meet (the
        // shoulder of an "n": the stroke under the notch goes on).
        let along = (rings[r][(k + 1) % n] - rings[r][k])
            .normalized()
            .map_or(0.0, |d| angle(dir, d));
        if along.min(180.0 - along) < CUT_END_MIN_DEG {
            continue;
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
        if winding(&rings, c.p.midpoint(end)) == 0
            || links
                .iter()
                .any(|l| crosses(c.p, end, l.pa, l.pb, 1e-9) || crosses(l.pa, l.pb, c.p, end, 1e-9))
        {
            continue;
        }
        used[i] = true;
        links.push(Link {
            a: (c.ring, Spot::Vertex(c.at)),
            b: (r, spot),
            pa: c.p,
            pb: end,
            cut: true,
            soft: false,
        });
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
            });
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
            Ok(pieces) => {
                return Some(Split {
                    pieces,
                    bridges: links.iter().map(|l| (l.pa, l.pb)).collect(),
                    soft: links.iter().any(|l| l.soft),
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

/// A place on a ring: a vertex, or a point on the segment after a vertex
/// (the part of the segment).
#[derive(Debug, Clone, Copy)]
enum Spot {
    Vertex(usize),
    On(usize, f64),
}

/// A bridge, or a cut from a notch.
#[derive(Debug, Clone, Copy)]
struct Link {
    a: (usize, Spot),
    b: (usize, Spot),
    pa: Point,
    pb: Point,
    cut: bool,
    soft: bool,
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
        .map(|(r, vs, bs, a)| Piece {
            rings: vec![moved_ends(r, vs, bs, &at, cuts)],
            area: *a,
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
            pieces[i].rings.push(moved_ends(r, vs, bs, &at, cuts));
        }
    }
    Some(Ok(pieces))
}

/// The ring of a face with each bridge that is an end of the piece (and
/// each cut from a notch) moved out of it by [`BRIDGE_OVERLAP`] of its
/// length (into the piece on the other side).
fn moved_ends(r: &[Point], vs: &[usize], bs: &[usize], at: &[Point], cuts: &[bool]) -> Vec<Point> {
    let n = vs.len();
    let cos_turn = emb_geom::math::cos(emb_geom::math::to_radians(END_TURN_DEG));
    let mut out = Vec::with_capacity(n + 4);
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
        let shift = Point::new(u.y, -u.x) * (BRIDGE_OVERLAP * p.distance(q));
        out.push(p + shift);
        out.push(q + shift);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(pts: &[(f64, f64)]) -> Vec<Point> {
        pts.iter().map(|&(x, y)| Point::new(x, y)).collect()
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
        let s = split(&[t], 600.0, true, true).expect("a split");
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
        let s = split(&[plus], 1000.0, true, true).expect("a split");
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
        assert!(split(&[outer, hole], 700.0, true, true).is_none());
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
        let s = split(&[v], 600.0, true, true).expect("a split");
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
        let s = split(&[stem_with_bracket()], 600.0, true, true).expect("a split");
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
        assert!(split(&[stem_with_bracket()], 600.0, true, false).is_none());
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
        let s = split(&[y], 600.0, true, true).expect("a split");
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
        assert!(split(&[l], 600.0, true, true).is_none());
        let outer = poly(&[(0.0, 0.0), (600.0, 0.0), (600.0, 600.0), (0.0, 600.0)]);
        let hole = poly(&[(100.0, 100.0), (100.0, 500.0), (500.0, 500.0), (500.0, 100.0)]);
        assert!(split(&[outer, hole], 600.0, true, true).is_none());
    }

    #[test]
    fn the_middle_bar_of_a_box_with_two_holes_is_a_piece() {
        // "日": a frame with a bar in the middle (two holes).
        let outer = poly(&[(0.0, 0.0), (500.0, 0.0), (500.0, 900.0), (0.0, 900.0)]);
        let low = poly(&[(100.0, 100.0), (100.0, 400.0), (400.0, 400.0), (400.0, 100.0)]);
        let high = poly(&[(100.0, 500.0), (100.0, 800.0), (400.0, 800.0), (400.0, 500.0)]);
        let s = split(&[outer, low, high], 900.0, true, true).expect("a split");
        assert_eq!(s.bridges.len(), 2, "{s:?}");
        assert_eq!(s.pieces.len(), 2, "{s:?}");
        // The frame has one hole (the two holes and the bar).
        let frame = s.pieces.iter().max_by(|a, b| a.area.total_cmp(&b.area)).unwrap();
        assert_eq!(frame.rings.len(), 2, "{s:?}");
    }
}
