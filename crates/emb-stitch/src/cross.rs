//! Stitches of cross-stitch objects (ADR 0038).
//!
//! All needle points are cell corners or cell centers. The crosses sew row
//! by row; then the backstitch lines sew, joined into long lines.

use std::collections::{BTreeMap, HashSet};

use emb_geom::Point;
use emb_model::cross::{Cell, CrossOrder, CrossStitch, Part};

use crate::object::Run;

/// No backstitch stitch is longer than this, in mm.
const MAX_LINE_STITCH_MM: f64 = 4.0;

/// A diagonal of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Diag {
    /// `/`: bottom left to top right.
    Up,
    /// `\`: top left to bottom right.
    Down,
}

impl Diag {
    fn other(self) -> Diag {
        match self {
            Diag::Up => Diag::Down,
            Diag::Down => Diag::Up,
        }
    }

    /// The ends of the diagonal of cell `i` (in row units), from left to right.
    fn ends(self, i: f64) -> ((f64, f64), (f64, f64)) {
        match self {
            Diag::Up => ((i, 1.0), (i + 1.0, 0.0)),
            Diag::Down => ((i, 0.0), (i + 1.0, 1.0)),
        }
    }
}

/// Points in grid units: x in columns, y in rows from the top of the row.
type Piece = Vec<(f64, f64)>;

/// Adds a leg from `a` to `b` that sews `passes` times (an odd number).
fn leg(out: &mut Piece, a: (f64, f64), b: (f64, f64), passes: u32) {
    out.push(a);
    out.push(b);
    for _ in 0..(passes.clamp(1, 5) - 1) / 2 {
        out.push(a);
        out.push(b);
    }
}

/// A row group of cells `a..=b`, sewn from left to right: the legs of
/// `first` out, and (for full crosses) the legs of the other diagonal
/// back. `full` is false for a group of half crosses (no way back). The
/// way back starts where the way out ends, or one cell side from it.
fn group_from_left(a: i32, b: i32, first: Diag, full: bool, order: CrossOrder, passes: u32) -> (Piece, Piece) {
    let (mut out, mut back) = (Piece::new(), Piece::new());
    let cols: Vec<f64> = (a..=b).map(f64::from).collect();
    if full && order == CrossOrder::Zigzag {
        // Out: the two diagonals in turn; each leg ends where the next starts.
        let mut d = first;
        for &i in &cols {
            let (s, e) = d.ends(i);
            leg(&mut out, s, e, passes);
            d = d.other();
        }
        // Back: the other diagonal of each cell, from right to left. It
        // starts at the other corner of the right side of the last cell.
        let mut d = d.other();
        for &i in cols.iter().rev() {
            let c = d.other();
            let (l, r) = c.ends(i);
            leg(&mut back, r, l, passes);
            d = c;
        }
        return (out, back);
    }
    // Out: the legs of `first`, with a travel stitch down (or up) the right
    // side of each cell to the start of the next leg.
    for (k, &i) in cols.iter().enumerate() {
        let (s, e) = first.ends(i);
        leg(&mut out, s, e, passes);
        if full || k + 1 < cols.len() {
            out.push((i + 1.0, 1.0 - e.1));
        }
    }
    if full {
        // Back: the legs of the other diagonal, from right to left, with a
        // travel stitch along the left side of each cell.
        let top = first.other();
        for (k, &i) in cols.iter().rev().enumerate() {
            let (l, r) = top.ends(i);
            leg(&mut back, r, l, passes);
            if k + 1 < cols.len() {
                back.push((i, 1.0 - l.1));
            }
        }
    }
    (out, back)
}

/// As [`group_from_left`], from the side of the needle: from the right
/// end, the group is mirrored, so the first legs are still `first`.
fn group(a: i32, b: i32, first: Diag, full: bool, order: CrossOrder, passes: u32, from_left: bool) -> (Piece, Piece) {
    if from_left {
        return group_from_left(a, b, first, full, order, passes);
    }
    let mirror = f64::from(a + b + 1);
    let flip = |p: Piece| p.into_iter().map(|(x, y)| (mirror - x, y)).collect();
    let (out, back) = group_from_left(a, b, first.other(), full, order, passes);
    (flip(out), flip(back))
}

/// One cell that is not in a group (quarter and three-quarter crosses):
/// the ways to sew it. The row takes the one that fits its neighbors best.
fn single(cell: &Cell, passes: u32) -> Vec<Piece> {
    let i = f64::from(cell.col);
    let legs = cell.part.legs();
    let at = |p: (f64, f64)| (i + p.0, p.1);
    match legs.as_slice() {
        // Quarter: out to the center and back, so it ends at a corner,
        // where the travel to the next piece can start.
        [(s, e)] => {
            let mut out = Piece::new();
            leg(&mut out, at(*s), at(*e), passes);
            out.push(at(*s));
            vec![out]
        }
        // Three-quarter: the quarter leg, then the half along the diagonal
        // from the center (that part sews twice, under the half); the half
        // either way; or all of it the other way round.
        [(s, e), (h0, h1)] => {
            let mut ways = Vec::new();
            for (a, b) in [(h0, h1), (h1, h0)] {
                let mut out = Piece::new();
                leg(&mut out, at(*s), at(*e), passes);
                out.push(at(*a));
                leg(&mut out, at(*a), at(*b), passes);
                ways.push(out.iter().rev().copied().collect());
                ways.push(out);
            }
            ways
        }
        _ => Vec::new(),
    }
}

/// What a group of cells next to each other in a row has.
fn group_kind(p: Part) -> Option<(Diag, bool)> {
    match p {
        Part::Full => Some((Diag::Up, true)),
        Part::HalfUp => Some((Diag::Up, false)),
        Part::HalfDown => Some((Diag::Down, false)),
        Part::Quarter(_) | Part::ThreeQuarter(_) => None,
    }
}

/// The runs of a cross-stitch object, from `entry` (mm).
pub fn cross_runs(x: &CrossStitch, entry: Point) -> Vec<Run> {
    let p = x.params;
    let mut rows: BTreeMap<i32, Vec<Cell>> = BTreeMap::new();
    for c in &x.cells {
        rows.entry(c.row).or_default().push(*c);
    }
    let mut at = x.to_grid(entry).unwrap_or(Point::ZERO);
    let mut order: Vec<(i32, Vec<Cell>)> = rows.into_iter().collect();
    // From the end (top or bottom row) nearer to the needle.
    if let (Some(first), Some(last)) = (order.first(), order.last())
        && (f64::from(last.0) + 0.5 - at.y).abs() < (f64::from(first.0) + 0.5 - at.y).abs()
    {
        order.reverse();
    }
    // The pieces in grid units (x in columns, y in rows).
    let mut pieces: Vec<Vec<Point>> = Vec::new();
    for (row, cells) in order {
        // Groups: cells next to each other with the same full or half cross.
        let mut groups: Vec<Vec<Cell>> = Vec::new();
        for c in cells {
            match groups.last_mut() {
                Some(g)
                    if group_kind(c.part).is_some()
                        && g[g.len() - 1].part == c.part
                        && g[g.len() - 1].col + 1 == c.col =>
                {
                    g.push(c)
                }
                _ => groups.push(vec![c]),
            }
        }
        let left = f64::from(groups[0][0].col);
        let right = f64::from(groups[groups.len() - 1][groups[groups.len() - 1].len() - 1].col + 1);
        // From the side of the row nearer to the needle.
        let from_left = (at.x - left).abs() <= (at.x - right).abs();
        if !from_left {
            groups.reverse();
        }
        // Out to the far end of the row: the single cells and the way out
        // of each group; then back: the ways back of the groups, from the
        // far end. So the partial crosses at the far end are on the way.
        // Each item has one way to sew it, or more (single cells).
        let mut items: Vec<Vec<Piece>> = Vec::new();
        let mut backs: Vec<Piece> = Vec::new();
        for g in groups {
            match group_kind(g[0].part) {
                Some((first, full)) => {
                    let (out, back) = group(g[0].col, g[g.len() - 1].col, first, full, p.order, p.passes, from_left);
                    items.push(vec![out]);
                    backs.push(back);
                }
                None => items.push(single(&g[0], p.passes)),
            }
        }
        items.extend(backs.into_iter().rev().map(|b| vec![b]));
        let to_points = |piece: &Piece| -> Vec<Point> {
            let mut points: Vec<Point> = piece
                .iter()
                .map(|&(gx, gy)| Point::new(gx, f64::from(row) + gy))
                .collect();
            points.dedup();
            points
        };
        let items: Vec<Vec<Vec<Point>>> = items
            .iter()
            .map(|ways| ways.iter().map(to_points).filter(|w| w.len() >= 2).collect())
            .filter(|ways: &Vec<Vec<Point>>| !ways.is_empty())
            .collect();
        for (k, ways) in items.iter().enumerate() {
            // The way that starts nearest to the needle and ends nearest to
            // the start of the next item (if it has one way only).
            let next = items.get(k + 1).filter(|w| w.len() == 1).map(|w| w[0][0]);
            let cost = |w: &Vec<Point>| w[0].distance(at) + next.map_or(0.0, |n| w[w.len() - 1].distance(n));
            let best = ways
                .iter()
                .min_by(|a, b| cost(a).total_cmp(&cost(b)))
                .expect("not empty");
            at = best[best.len() - 1];
            pieces.push(best.clone());
        }
    }
    // Travel to the next piece along the sides of the cells, where it can:
    // the pieces join into one run. Else a new run (the plan trims).
    let painted: HashSet<(i32, i32)> = x.cells.iter().map(|c| (c.col, c.row)).collect();
    let mut joined: Vec<Vec<Point>> = Vec::new();
    for piece in pieces {
        let route = joined
            .last()
            .and_then(|r| edge_route(r[r.len() - 1], piece[0], &painted));
        match (route, joined.last_mut()) {
            (Some(route), Some(r)) => {
                r.extend(route);
                r.extend(piece);
                r.dedup();
            }
            _ => joined.push(piece),
        }
    }
    let mut runs: Vec<Run> = joined
        .into_iter()
        .map(|r| Run::new(r.into_iter().map(|g| x.at(g.x, g.y)).collect()))
        .collect();
    runs.extend(line_runs(x, entry));
    runs
}

/// Travel stitches between pieces go no farther than this, in cells.
const MAX_ROUTE_CELLS: i32 = 3;

/// A route from the grid corner `a` to the grid corner `b` along the cell
/// sides, one cell for each stitch, with a painted cell on one side of
/// each stitch at least (so the crosses hide or frame it). `None` if a point is
/// not a corner, the route is long, or no route has painted cells all
/// along it. The route has `a` but not `b`.
fn edge_route(a: Point, b: Point, painted: &HashSet<(i32, i32)>) -> Option<Vec<Point>> {
    let corner = |p: Point| {
        let (x, y) = (p.x.round(), p.y.round());
        ((p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9).then_some((x as i32, y as i32))
    };
    let ((ax, ay), (bx, by)) = (corner(a)?, corner(b)?);
    if (bx - ax).abs() > MAX_ROUTE_CELLS || (by - ay).abs() > MAX_ROUTE_CELLS {
        return None;
    }
    // The painted cells on the two sides of the cell side from (x, y),
    // one step along (dx, dy).
    let sides = |x: i32, y: i32, dx: i32, dy: i32| {
        let (c1, c2) = if dx != 0 {
            let cx = x.min(x + dx);
            ((cx, y - 1), (cx, y))
        } else {
            let cy = y.min(y + dy);
            ((x - 1, cy), (x, cy))
        };
        usize::from(painted.contains(&c1)) + usize::from(painted.contains(&c2))
    };
    // Of the two routes (up or down first, or across first), the one with
    // fewer stitches on the edge of the crosses (a painted cell on one
    // side only).
    let mut best: Option<(usize, Vec<(i32, i32)>)> = None;
    for vertical_first in [true, false] {
        let mut route = vec![(ax, ay)];
        let mut ok = true;
        let mut edge = 0;
        let legs = if vertical_first {
            [(0, by - ay), (bx - ax, 0)]
        } else {
            [(bx - ax, 0), (0, by - ay)]
        };
        for (dx, dy) in legs {
            for _ in 0..(dx.abs() + dy.abs()) {
                let (x, y) = route[route.len() - 1];
                let step = (dx.signum(), dy.signum());
                let n = sides(x, y, step.0, step.1);
                ok &= n > 0;
                edge += usize::from(n == 1);
                route.push((x + step.0, y + step.1));
            }
        }
        if ok && best.as_ref().is_none_or(|(e, _)| edge < *e) {
            route.pop();
            best = Some((edge, route));
        }
    }
    best.map(|(_, route)| {
        route
            .into_iter()
            .map(|(x, y)| Point::new(f64::from(x), f64::from(y)))
            .collect()
    })
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

/// The backstitch lines, joined into trails that go through as many lines
/// as they can.
fn line_runs(x: &CrossStitch, entry: Point) -> Vec<Run> {
    let mut adj: BTreeMap<[i32; 2], Vec<usize>> = BTreeMap::new();
    for (i, l) in x.lines.iter().enumerate() {
        adj.entry(l[0]).or_default().push(i);
        adj.entry(l[1]).or_default().push(i);
    }
    let mut used = vec![false; x.lines.len()];
    let mut needle = entry;
    let mut runs = Vec::new();
    while used.iter().any(|u| !u) {
        // Start at an end of a trail (a point with an odd number of free
        // lines) if there is one, else at any point with a free line; the
        // one nearest to the needle.
        let free = |v: &[i32; 2], used: &[bool]| adj[v].iter().filter(|&&i| !used[i]).count();
        let candidates: Vec<[i32; 2]> = adj.keys().filter(|v| free(v, &used) > 0).copied().collect();
        let odd: Vec<[i32; 2]> = candidates.iter().filter(|v| free(v, &used) % 2 == 1).copied().collect();
        let pool = if odd.is_empty() { &candidates } else { &odd };
        let Some(&start) = pool.iter().min_by(|a, b| {
            x.half_point(**a)
                .distance(needle)
                .total_cmp(&x.half_point(**b).distance(needle))
        }) else {
            break;
        };
        let mut trail = vec![start];
        let mut v = start;
        while let Some(&i) = adj[&v].iter().find(|&&i| !used[i]) {
            used[i] = true;
            let l = x.lines[i];
            v = if l[0] == v { l[1] } else { l[0] };
            trail.push(v);
        }
        // Needle points: each point of the half grid on the line, and more
        // where a step is long.
        let mut points = vec![x.half_point(trail[0])];
        for w in trail.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let g = gcd(dx, dy).max(1);
            for k in 1..=g {
                let a = *points.last().expect("not empty");
                let b = x.half_point([w[0][0] + dx / g * k, w[0][1] + dy / g * k]);
                let n = (a.distance(b) / MAX_LINE_STITCH_MM).ceil().max(1.0) as usize;
                for j in 1..=n {
                    points.push(a.lerp(b, j as f64 / n as f64));
                }
            }
        }
        needle = *points.last().expect("not empty");
        runs.push(Run {
            fixed: false,
            repeats: x.params.line_repeats.clamp(1, 3),
            ..Run::new(points)
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use emb_model::cross::Corner;

    use super::*;

    fn grid() -> CrossStitch {
        CrossStitch::new(Point::ZERO, 2.0)
    }

    fn legs(runs: &[Run]) -> Vec<(Point, Point)> {
        runs.iter()
            .flat_map(|r| r.points.windows(2).map(|w| (w[0], w[1])))
            .collect()
    }

    fn on_grid(p: Point, step: f64) -> bool {
        let (x, y) = (p.x / step, p.y / step);
        (x - x.round()).abs() < 1e-9 && (y - y.round()).abs() < 1e-9
    }

    /// The legs of a diagonal of the cell (col, row) of a 2 mm grid, in either direction.
    fn count_diag(runs: &[Run], col: i32, row: i32, up: bool) -> usize {
        let (x, y) = (f64::from(col) * 2.0, f64::from(row) * 2.0);
        let (a, b) = if up {
            (Point::new(x, y + 2.0), Point::new(x + 2.0, y))
        } else {
            (Point::new(x, y), Point::new(x + 2.0, y + 2.0))
        };
        legs(runs)
            .iter()
            .filter(|(p, q)| {
                (p.distance(a) < 1e-9 && q.distance(b) < 1e-9) || (p.distance(b) < 1e-9 && q.distance(a) < 1e-9)
            })
            .count()
    }

    /// The index of the first leg on the diagonal, in sew order.
    fn first_diag(runs: &[Run], col: i32, row: i32, up: bool) -> usize {
        let (x, y) = (f64::from(col) * 2.0, f64::from(row) * 2.0);
        let (a, b) = if up {
            (Point::new(x, y + 2.0), Point::new(x + 2.0, y))
        } else {
            (Point::new(x, y), Point::new(x + 2.0, y + 2.0))
        };
        legs(runs)
            .iter()
            .position(|(p, q)| {
                (p.distance(a) < 1e-9 && q.distance(b) < 1e-9) || (p.distance(b) < 1e-9 && q.distance(a) < 1e-9)
            })
            .expect("the leg is there")
    }

    #[test]
    fn rows_sew_each_leg_once_with_the_top_leg_last() {
        for from_right in [false, true] {
            let mut x = grid();
            x.params.order = CrossOrder::Rows;
            for c in 0..5 {
                x.set(c, 0, Part::Full);
                x.set(c, 1, Part::Full);
            }
            let entry = if from_right { Point::new(10.0, 0.0) } else { Point::ZERO };
            let runs = cross_runs(&x, entry);
            assert_eq!(runs.len(), 1, "the rows join along the cell sides");
            for r in 0..2 {
                for c in 0..5 {
                    assert_eq!(count_diag(&runs, c, r, true), 1);
                    assert_eq!(count_diag(&runs, c, r, false), 1);
                    // The `\` leg is on top in each cell.
                    assert!(first_diag(&runs, c, r, true) < first_diag(&runs, c, r, false));
                }
            }
            // All needle points are cell corners.
            assert!(runs.iter().flat_map(|r| &r.points).all(|p| on_grid(*p, 2.0)));
            // The first row starts at the side of the needle.
            assert!(
                runs[0].points[0].distance(entry) <= 2.0 * 2f64.sqrt() + 1e-9,
                "{:?}",
                runs[0].points[0]
            );
        }
    }

    #[test]
    fn zigzag_has_no_travel_stitches_but_the_turn() {
        let mut x = grid();
        assert_eq!(x.params.order, CrossOrder::Zigzag, "the default");
        for c in 0..6 {
            x.set(c, 0, Part::Full);
        }
        let runs = cross_runs(&x, Point::ZERO);
        assert_eq!(runs.len(), 1);
        let l = legs(&runs);
        // 12 legs and one travel stitch along the side of the last cell.
        assert_eq!(l.len(), 13);
        let travel = l.iter().filter(|(p, q)| (p.distance(*q) - 2.0).abs() < 1e-9).count();
        assert_eq!(travel, 1);
        for c in 0..6 {
            assert_eq!(count_diag(&runs, c, 0, true), 1);
            assert_eq!(count_diag(&runs, c, 0, false), 1);
        }
    }

    #[test]
    fn partial_crosses() {
        let mut x = grid();
        x.set(0, 0, Part::HalfUp);
        x.set(1, 0, Part::HalfUp);
        x.set(3, 0, Part::Quarter(Corner::TopLeft));
        x.set(5, 0, Part::ThreeQuarter(Corner::TopLeft));
        let runs = cross_runs(&x, Point::ZERO);
        assert_eq!(runs.len(), 3, "the halves are one group");
        assert_eq!(count_diag(&runs, 0, 0, true), 1);
        assert_eq!(count_diag(&runs, 1, 0, true), 1);
        assert_eq!(count_diag(&runs, 0, 0, false), 0);
        // The quarter: from the corner to the center and back.
        assert_eq!(
            runs[1].points[runs[1].points.len() - 3..],
            [Point::new(6.0, 0.0), Point::new(7.0, 1.0), Point::new(6.0, 0.0)]
        );
        // Three-quarter: the quarter, then the `/` half (the one that does
        // not touch the top-left corner).
        assert_eq!(count_diag(&runs, 5, 0, true), 1);
        assert_eq!(runs[2].points[0], Point::new(10.0, 0.0));
        assert_eq!(runs[2].points[1], Point::new(11.0, 1.0));
    }

    #[test]
    fn rows_join_along_the_cell_sides_with_no_gap() {
        let mut x = grid();
        // A diamond of full crosses: each row has another width.
        for (r, (a, b)) in [(2, 3), (1, 4), (0, 5), (1, 4), (2, 3)].into_iter().enumerate() {
            for c in a..=b {
                x.set(c, r as i32, Part::Full);
            }
        }
        let runs = cross_runs(&x, Point::ZERO);
        // Each run starts where the run before it ends: no trims.
        for w in runs.windows(2) {
            let (e, s) = (w[0].points[w[0].points.len() - 1], w[1].points[0]);
            assert!(e.distance(s) < 1e-9, "{e:?} to {s:?}");
        }
        // No stitch is longer than a cell diagonal.
        let longest = legs(&runs).iter().map(|(a, b)| a.distance(*b)).fold(0.0, f64::max);
        assert!(longest <= 2.0 * 2f64.sqrt() + 1e-9);
        // An island in the first row, four empty cells away: no travel
        // over the empty cells, there and back.
        x.set(9, 0, Part::Full);
        let runs = cross_runs(&x, Point::ZERO);
        let gaps = runs
            .windows(2)
            .filter(|w| w[0].points[w[0].points.len() - 1].distance(w[1].points[0]) > 1e-9)
            .count();
        assert_eq!(gaps, 2);
    }

    #[test]
    fn far_three_quarter_crosses_need_no_travel_round_them() {
        let mut x = grid();
        for c in 1..4 {
            x.set(c, 0, Part::Full);
        }
        x.set(0, 0, Part::ThreeQuarter(Corner::TopRight));
        x.set(4, 0, Part::ThreeQuarter(Corner::TopLeft));
        let runs = cross_runs(&x, Point::ZERO);
        assert_eq!(runs.len(), 1);
        // All stitches are legs of crosses: the far three-quarter cross
        // ends where the way back starts, so even the turn of the zigzag
        // needs no stitch along a cell side.
        let sides = legs(&runs)
            .iter()
            .filter(|(a, b)| (a.distance(*b) - 2.0).abs() < 1e-9)
            .count();
        assert_eq!(sides, 0, "{:?}", runs[0].points);
    }

    #[test]
    fn passes_sew_each_leg_three_times() {
        let mut x = grid();
        x.params.passes = 3;
        x.set(0, 0, Part::HalfDown);
        let runs = cross_runs(&x, Point::ZERO);
        assert_eq!(count_diag(&runs, 0, 0, false), 3);
    }

    #[test]
    fn lines_join_into_trails_with_short_stitches() {
        let mut x = grid();
        // An L of two lines and a separate diagonal line 3 cells long.
        x.add_line([0, 0], [4, 0]);
        x.add_line([4, 0], [4, 4]);
        x.add_line([10, 0], [16, 6]);
        let runs = cross_runs(&x, Point::ZERO);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].points.first(), Some(&Point::ZERO));
        assert_eq!(runs[0].points.last(), Some(&Point::new(4.0, 4.0)));
        // Each half-grid point is a needle point: 1 mm steps.
        assert_eq!(runs[0].points.len(), 9);
        assert_eq!(runs[1].points.len(), 7);
        assert!(!runs[1].fixed);
    }

    #[test]
    fn a_turned_grid_turns_the_stitches() {
        let mut x = grid();
        x.set(0, 0, Part::Full);
        let plain = cross_runs(&x, Point::ZERO);
        let t = emb_geom::Transform::rotate_deg(90.0);
        x.transform(&t);
        let turned = cross_runs(&x, t.apply(Point::ZERO));
        let a: Vec<Point> = plain
            .iter()
            .flat_map(|r| r.points.iter().map(|p| t.apply(*p)))
            .collect();
        let b: Vec<Point> = turned.iter().flat_map(|r| r.points.clone()).collect();
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(p, q)| p.distance(*q) < 1e-9));
    }
}
