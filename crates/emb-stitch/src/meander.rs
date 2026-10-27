//! Meander fill (P11 Milestone 4, ADR 0047): one line along the edges of a
//! tile pattern that fills the shape and never crosses itself.
//!
//! Not a port. Ink/Stitch has a meander fill made from tile patterns; this
//! follows what its public documentation says it looks like, in our own
//! method:
//!
//! 1. The corners of the tiles (squares, triangles or hexagons, `spacing`
//!    from one corner to the next, turned by the fill angle) that are
//!    inside the shape, a quarter spacing in from its edge, are the nodes.
//!    Two nodes are joined if the tile edge between them is inside.
//! 2. In each connected part, the line starts with one step from the node
//!    nearest to the needle.
//! 3. Then each step `u → v` of the line can become a detour `u → … → v`
//!    round one tile, through nodes that the line does not use yet. The
//!    new steps can get detours too. The steps are taken in an order that
//!    the seed sets (not the newest first, which makes long straight
//!    combs), so the line grows in bends in all directions. When no step can,
//!    the line goes one step on from one of its ends (near the edge of the
//!    shape a tile can be cut off), and the detours go on.
//! 4. When the ends cannot go on either, each step `a → v` can become a
//!    longer detour (round two tiles), or a node `a` of the line (between
//!    `u` and `v`) can give its place to a longer way from `u` to `v`
//!    through free nodes (with or without `a`). This fills the places that
//!    no detour round one tile reaches (most of all with hexagons, where
//!    the line often uses three sides of a tile, and near the edge). Then
//!    steps 3 and 4 go on until the line uses no more nodes.
//!
//! A line on the tile edges that uses each node one time cannot cross
//! itself.
//! 5. The corners are rounded (corner cutting), and the line gets needle
//!    points at the stitch length.
//!
//! The choices between neighbors and detours come from a fixed seed, so
//! the line is the same on each run.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use emb_geom::{Point, Polyline, Transform};
use emb_model::MeanderTile;

use crate::curves::sew_line;
use crate::running;
use crate::travel::{Sewer, Shape};

/// Smallest tile size that the generator accepts, in mm.
const MIN_SPACING_MM: f64 = 0.5;
/// Most nodes of a meander. With more, the tiles get larger.
const MAX_NODES: f64 = 250_000.0;
/// Seed of the choices.
const SEED: u64 = 0x4D45_414E;
/// Most rounds of step 4 for one part.
const MAX_ROUNDS: usize = 12;

/// The tile pattern as a graph: node places (turned coordinates) and the
/// neighbors of each node.
struct Lattice {
    places: Vec<Point>,
    links: Vec<Vec<usize>>,
    /// The longest detour (in steps) round one tile.
    detour: usize,
}

/// The nodes of `tile` with corners `s` apart in the box `lo`–`hi`
/// (turned coordinates) for which `keep` is true, and their links.
fn lattice(tile: MeanderTile, s: f64, lo: Point, hi: Point, keep: &dyn Fn(Point) -> bool) -> Lattice {
    let h3 = 3f64.sqrt() / 2.0;
    // Grid steps in x and y, and the place of grid cell (i, j).
    let (dx, dy) = match tile {
        MeanderTile::Square => (s, s),
        MeanderTile::Triangle => (s, s * h3),
        MeanderTile::Hexagon => (s * h3, s * 1.5),
    };
    let place = |i: i64, j: i64| -> Point {
        match tile {
            MeanderTile::Square => Point::new(i as f64 * dx, j as f64 * dy),
            MeanderTile::Triangle => Point::new(
                (i as f64 + if j.rem_euclid(2) == 1 { 0.5 } else { 0.0 }) * dx,
                j as f64 * dy,
            ),
            // A honeycomb drawn as bricks: every node has a left and a right
            // neighbor, and one up (i + j even) or down (odd).
            MeanderTile::Hexagon => Point::new(
                i as f64 * dx,
                j as f64 * dy + if (i + j).rem_euclid(2) == 1 { 0.5 * s } else { 0.0 },
            ),
        }
    };
    let neighbors = |i: i64, j: i64| -> Vec<(i64, i64)> {
        match tile {
            MeanderTile::Square => vec![(i + 1, j), (i - 1, j), (i, j + 1), (i, j - 1)],
            MeanderTile::Triangle => {
                let o = if j.rem_euclid(2) == 1 { 0 } else { -1 };
                vec![
                    (i + 1, j),
                    (i - 1, j),
                    (i + o, j - 1),
                    (i + o + 1, j - 1),
                    (i + o, j + 1),
                    (i + o + 1, j + 1),
                ]
            }
            MeanderTile::Hexagon => {
                let v = if (i + j).rem_euclid(2) == 0 { j - 1 } else { j + 1 };
                vec![(i + 1, j), (i - 1, j), (i, v)]
            }
        }
    };
    let (i0, i1) = ((lo.x / dx).floor() as i64 - 1, (hi.x / dx).ceil() as i64 + 1);
    let (j0, j1) = ((lo.y / dy).floor() as i64 - 1, (hi.y / dy).ceil() as i64 + 1);
    let width = (i1 - i0 + 1) as usize;
    let mut index = vec![usize::MAX; width * (j1 - j0 + 1) as usize];
    let slot = |i: i64, j: i64| -> Option<usize> {
        (i >= i0 && i <= i1 && j >= j0 && j <= j1).then(|| (j - j0) as usize * width + (i - i0) as usize)
    };
    let mut places = Vec::new();
    let mut cells = Vec::new();
    for j in j0..=j1 {
        for i in i0..=i1 {
            let p = place(i, j);
            if keep(p) {
                index[slot(i, j).expect("in the grid")] = places.len();
                places.push(p);
                cells.push((i, j));
            }
        }
    }
    let links = cells
        .iter()
        .enumerate()
        .map(|(a, &(i, j))| {
            neighbors(i, j)
                .into_iter()
                .filter_map(|(ni, nj)| slot(ni, nj).map(|k| index[k]))
                .filter(|&b| {
                    b != usize::MAX && {
                        let (p, q) = (places[a], places[b]);
                        keep(p.lerp(q, 0.25)) && keep(p.midpoint(q)) && keep(p.lerp(q, 0.75))
                    }
                })
                .collect()
        })
        .collect();
    let detour = match tile {
        MeanderTile::Square => 3,
        MeanderTile::Triangle => 2,
        MeanderTile::Hexagon => 5,
    };
    Lattice { places, links, detour }
}

/// A number in [0, 1) from the seed and a key.
fn pick(key: u64) -> f64 {
    crate::effects::hash01(SEED, key)
}

/// The neighbors of `n` in an order that the seed sets.
fn shuffled(links: &[usize], n: usize, salt: u64) -> Vec<usize> {
    let mut v = links.to_vec();
    v.sort_by(|a, b| {
        let ka = pick((n as u64) << 32 ^ (*a as u64) ^ salt);
        let kb = pick((n as u64) << 32 ^ (*b as u64) ^ salt);
        ka.total_cmp(&kb).then(a.cmp(b))
    });
    v
}

/// A path from `from` to `to` of 2 to `max` steps through free nodes.
fn find_detour(l: &Lattice, used: &[bool], from: usize, to: usize, max: usize) -> Option<Vec<usize>> {
    fn walk(l: &Lattice, used: &[bool], at: usize, to: usize, left: usize, path: &mut Vec<usize>) -> bool {
        for n in shuffled(&l.links[at], at, path.len() as u64) {
            if n == to && !path.is_empty() {
                return true;
            }
            if left > 1 && !used[n] && n != to && !path.contains(&n) {
                path.push(n);
                if walk(l, used, n, to, left - 1, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut path = Vec::new();
    walk(l, used, from, to, max, &mut path).then_some(path)
}

/// The meander lines of the lattice, one for each connected part, from the
/// part nearest to `entry`.
fn lines(l: &Lattice, entry: Point) -> Vec<Vec<Point>> {
    let n = l.places.len();
    let mut used = vec![false; n];
    let mut part = vec![usize::MAX; n];
    // The connected parts.
    let mut parts: Vec<Vec<usize>> = Vec::new();
    for s in 0..n {
        if part[s] != usize::MAX {
            continue;
        }
        let id = parts.len();
        let mut nodes = vec![s];
        part[s] = id;
        let mut i = 0;
        while i < nodes.len() {
            for &m in &l.links[nodes[i]] {
                if part[m] == usize::MAX {
                    part[m] = id;
                    nodes.push(m);
                }
            }
            i += 1;
        }
        parts.push(nodes);
    }
    let mut next = vec![usize::MAX; n];
    let mut prev = vec![usize::MAX; n];
    let mut out = Vec::new();
    let mut at = entry;
    let mut todo: Vec<usize> = (0..parts.len()).collect();
    while !todo.is_empty() {
        // The part with the node nearest to the needle, and that node.
        let (ti, start) = todo
            .iter()
            .enumerate()
            .flat_map(|(ti, &p)| parts[p].iter().map(move |&s| (ti, s)))
            .filter(|(_, s)| !used[*s])
            .min_by(|a, b| {
                l.places[a.1]
                    .distance(at)
                    .total_cmp(&l.places[b.1].distance(at))
                    .then(a.1.cmp(&b.1))
            })
            .expect("parts are not empty");
        todo.swap_remove(ti);
        // Step 2: the first node; step 3 makes the first step from it.
        let mut start = start;
        used[start] = true;
        let mut end = start;
        let free = |used: &[bool], n: usize| shuffled(&l.links[n], n, 7).into_iter().find(|&m| !used[m]);
        // The steps to try, in the order of the seed.
        let mut queue: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        let mut count = 0u64;
        let mut push = |queue: &mut BinaryHeap<Reverse<(u64, usize)>>, u: usize| {
            count += 1;
            let key = (pick((u as u64) << 24 ^ count) * (1u64 << 53) as f64) as u64;
            queue.push(Reverse((key, u)));
        };
        // Puts `path` between `u` and `v` (next to each other now).
        let link = |next: &mut [usize], prev: &mut [usize], used: &mut [bool], u: usize, v: usize, path: &[usize]| {
            let mut a = u;
            for &m in path {
                used[m] = true;
                next[a] = m;
                prev[m] = a;
                a = m;
            }
            next[a] = v;
            prev[v] = a;
        };
        let mut rounds = 0;
        loop {
            // Step 3: detours.
            while let Some(Reverse((_, u))) = queue.pop() {
                let v = next[u];
                if v == usize::MAX {
                    continue;
                }
                if let Some(path) = find_detour(l, &used, u, v, l.detour) {
                    link(&mut next, &mut prev, &mut used, u, v, &path);
                    let mut a = u;
                    while a != v {
                        push(&mut queue, a);
                        a = next[a];
                    }
                }
            }
            // One step on from an end.
            if let Some(m) = free(&used, end) {
                used[m] = true;
                next[end] = m;
                prev[m] = end;
                push(&mut queue, end);
                end = m;
                continue;
            }
            if let Some(m) = free(&used, start) {
                used[m] = true;
                next[m] = start;
                prev[start] = m;
                push(&mut queue, m);
                start = m;
                continue;
            }
            // Step 4: a node gives its place to a longer way round.
            if rounds >= MAX_ROUNDS {
                break;
            }
            rounds += 1;
            let mut nodes = Vec::new();
            let mut a = start;
            while a != usize::MAX {
                nodes.push(a);
                a = next[a];
            }
            let mut changed = false;
            let long = 2 * l.detour + 1;
            for &a in &nodes {
                let v = next[a];
                if used[a]
                    && v != usize::MAX
                    && prev[v] == a
                    && let Some(path) = find_detour(l, &used, a, v, long - 2)
                {
                    link(&mut next, &mut prev, &mut used, a, v, &path);
                    let mut b = a;
                    while b != v {
                        push(&mut queue, b);
                        b = next[b];
                    }
                    changed = true;
                    continue;
                }
                let (u, v) = (prev[a], next[a]);
                if !used[a] || u == usize::MAX || v == usize::MAX || next[u] != a {
                    continue;
                }
                used[a] = false;
                match find_detour(l, &used, u, v, long) {
                    Some(path) if path.len() >= 2 => {
                        if !path.contains(&a) {
                            next[a] = usize::MAX;
                            prev[a] = usize::MAX;
                        }
                        link(&mut next, &mut prev, &mut used, u, v, &path);
                        let mut b = u;
                        while b != v {
                            push(&mut queue, b);
                            b = next[b];
                        }
                        changed = true;
                    }
                    _ => used[a] = true,
                }
            }
            if !changed {
                break;
            }
        }
        let mut line = vec![l.places[start]];
        let mut u = start;
        while next[u] != usize::MAX {
            u = next[u];
            line.push(l.places[u]);
        }
        at = l.places[u];
        if line.len() >= 2 {
            out.push(line);
        }
    }
    out
}

/// Rounds the corners of an open line (two steps of corner cutting); the
/// ends stay.
fn rounded(line: &[Point]) -> Vec<Point> {
    let mut v = line.to_vec();
    for _ in 0..2 {
        if v.len() < 3 {
            break;
        }
        let mut out = vec![v[0]];
        for w in v.windows(2) {
            out.push(w[0].lerp(w[1], 0.25));
            out.push(w[0].lerp(w[1], 0.75));
        }
        out.push(v[v.len() - 1]);
        // The first and the last cut points are next to the ends.
        out.remove(1);
        out.remove(out.len() - 2);
        v = out;
    }
    v
}

/// Sews a meander fill.
pub fn meander(
    shape: &Shape,
    tile: MeanderTile,
    angle_deg: f64,
    spacing: f64,
    len: f64,
    entry: Point,
    sewer: &mut Sewer,
) {
    for line in meander_lines(shape, tile, angle_deg, spacing, entry) {
        let stitched = running::running_points(
            &Polyline {
                points: line,
                closed: false,
            },
            len,
        );
        if sewer.needle().is_some() {
            sewer.go_to(stitched[0]);
        }
        sew_line(sewer, &stitched);
    }
}

/// The meander lines of a shape with rounded corners (one for each
/// connected part), from the part nearest to `entry`. Quilt blocks use
/// them as lines (`emb_model::quilt`).
pub fn meander_lines(shape: &Shape, tile: MeanderTile, angle_deg: f64, spacing: f64, entry: Point) -> Vec<Vec<Point>> {
    let to = Transform::rotate_deg(-angle_deg);
    let back = Transform::rotate_deg(angle_deg);
    let mut spacing = spacing.max(MIN_SPACING_MM);
    let inner = shape.inset(spacing * 0.25);
    let turned: Vec<Point> = inner.rings().flatten().map(|p| to.apply(*p)).collect();
    let Some(b) = emb_geom::BBox::from_points(turned.iter().copied()) else {
        return Vec::new();
    };
    let area = b.width() * b.height();
    if area / (spacing * spacing) > MAX_NODES {
        spacing = (area / MAX_NODES).sqrt();
    }
    let keep = |p: Point| {
        let q = back.apply(p);
        inner.regions.iter().any(|r| r.contains(q))
    };
    let l = lattice(tile, spacing, b.min, b.max, &keep);
    lines(&l, to.apply(entry))
        .into_iter()
        .map(|line| rounded(&line).into_iter().map(|p| back.apply(p)).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::travel::Travel;

    fn square(size: f64) -> Shape {
        Shape::new(
            &[Polyline {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(size, 0.0),
                    Point::new(size, size),
                    Point::new(0.0, size),
                ],
                closed: true,
            }],
            &[],
        )
    }

    /// Two stitches cross (touching ends do not count).
    fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
        let r = b - a;
        let s = d - c;
        let den = r.cross(s);
        if den.abs() < 1e-12 {
            return false;
        }
        let t = (c - a).cross(s) / den;
        let u = (c - a).cross(r) / den;
        t > 1e-6 && t < 1.0 - 1e-6 && u > 1e-6 && u < 1.0 - 1e-6
    }

    #[test]
    fn each_tile_fills_the_shape_with_one_line_that_does_not_cross() {
        let shape = square(20.0);
        for tile in MeanderTile::ALL {
            let travel = Travel::new(&shape);
            let mut s = Sewer::new(&travel, 3.0);
            meander(&shape, tile, 0.0, 2.0, 2.0, Point::ZERO, &mut s);
            let runs = s.finish();
            assert_eq!(runs.len(), 1, "{tile:?}");
            let pts = &runs[0];
            // The line goes near most places of the square (the middle of a
            // hexagon is one spacing from its corners).
            let mut hit = 0;
            for i in 0..10 {
                for j in 0..10 {
                    let c = Point::new(1.0 + 2.0 * i as f64, 1.0 + 2.0 * j as f64);
                    if pts.iter().any(|p| p.distance(c) < 2.5) {
                        hit += 1;
                    }
                }
            }
            assert!(hit >= 85, "{tile:?}: {hit} of 100");
            let segs: Vec<(Point, Point)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
            for i in 0..segs.len() {
                for j in i + 2..segs.len() {
                    assert!(
                        !crosses(segs[i].0, segs[i].1, segs[j].0, segs[j].1),
                        "{tile:?}: stitches {i} and {j} cross"
                    );
                }
            }
            assert!(pts.iter().all(|p| p.x > 0.0 && p.x < 20.0 && p.y > 0.0 && p.y < 20.0));
        }
    }

    #[test]
    fn the_meander_is_the_same_on_each_run_and_turns_with_the_angle() {
        let shape = square(15.0);
        let run = |angle: f64| {
            let travel = Travel::new(&shape);
            let mut s = Sewer::new(&travel, 3.0);
            meander(&shape, MeanderTile::Hexagon, angle, 1.5, 2.0, Point::ZERO, &mut s);
            s.finish()
        };
        assert_eq!(run(30.0), run(30.0));
        assert_ne!(run(0.0), run(30.0));
    }

    #[test]
    fn the_line_uses_most_corners_of_the_tiles() {
        for tile in MeanderTile::ALL {
            let l = lattice(tile, 1.0, Point::new(0.0, 0.0), Point::new(30.0, 30.0), &|p| {
                p.x > 0.0 && p.x < 30.0 && p.y > 0.0 && p.y < 30.0
            });
            let used: usize = lines(&l, Point::ZERO).iter().map(Vec::len).sum();
            let share = used as f64 / l.places.len() as f64;
            assert!(share > 0.85, "{tile:?}: {share}");
            // A ring 17 mm wide.
            let c = Point::new(30.0, 30.0);
            let l = lattice(tile, 2.0, Point::new(0.0, 0.0), Point::new(60.0, 60.0), &|p| {
                (9.5..25.5).contains(&p.distance(c))
            });
            let used: usize = lines(&l, Point::ZERO).iter().map(Vec::len).sum();
            let share = used as f64 / l.places.len() as f64;
            assert!(share > 0.8, "{tile:?} ring: {share}");
        }
    }

    #[test]
    fn honeycomb_nodes_have_three_neighbors() {
        let l = lattice(
            MeanderTile::Hexagon,
            1.0,
            Point::new(-5.0, -5.0),
            Point::new(5.0, 5.0),
            &|_| true,
        );
        let middle = l
            .places
            .iter()
            .position(|p| p.length() < 1.0)
            .expect("a node near the middle");
        assert_eq!(l.links[middle].len(), 3);
        for &m in &l.links[middle] {
            assert!((l.places[m].distance(l.places[middle]) - 1.0).abs() < 1e-9);
        }
    }
}
