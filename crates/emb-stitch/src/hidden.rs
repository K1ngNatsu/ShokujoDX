//! Hidden travel (ADR 0082): between two objects of one color that are far
//! apart, travel stitches that objects sewn later cover, in place of a trim.
//!
//! Not a port. A grid of cells keeps, for each cell, the last place in the
//! sew order of an object that covers the cell (a dense fill or a satin).
//! A way from one object to the next is hidden when each cell on it is
//! covered by an object that sews after the travel. The way is a straight
//! line if one is hidden, else the shortest way through the covered cells
//! (A* on the grid), with its corners cut where a straight line is hidden.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use emb_geom::{BBox, Path, Point, Transform};
use emb_model::{Design, FillParams, FillPattern, ObjectId, ObjectKind};

/// The size of a cell of the grid, mm (larger for a large design).
const CELL_MM: f64 = 0.25;
/// The grid has at most this many cells on a side.
const MAX_SIDE: usize = 2000;
/// Rows or stitches farther apart than this do not cover a travel stitch, mm.
const COVER_SPACING_MM: f64 = 0.8;
/// The ends of a travel can be this far from a covered cell: they are on
/// the edge of an object, mm.
const END_MM: f64 = 0.6;
/// A hidden travel is at most this long; a longer way gets a trim, mm.
pub const MAX_TRAVEL_MM: f64 = 60.0;
/// The stitches of a hidden travel are at most this long, mm.
pub const TRAVEL_STITCH_MM: f64 = 2.5;
/// A search looks at most at this many cells.
const MAX_SEARCH: usize = 200_000;
/// Tolerance to flatten the shapes, mm.
const FLATTEN_MM: f64 = 0.05;

/// The covered cells of a design.
#[derive(Debug, Clone)]
pub struct Cover {
    min: Point,
    cell: f64,
    nx: usize,
    ny: usize,
    /// For each cell: 0, or 1 + the last place in the sew order of an
    /// object that covers it.
    last: Vec<u32>,
}

/// The rings (output coordinates) of the area that the object covers, if
/// its stitches are dense enough to hide a travel stitch under them.
fn cover_rings(kind: &ObjectKind, to_out: &Transform) -> Vec<Vec<Point>> {
    let rings = |p: &Path| -> Vec<Vec<Point>> {
        p.flatten(FLATTEN_MM)
            .into_iter()
            .map(|pl| pl.points.iter().map(|q| to_out.apply(*q)).collect())
            .collect()
    };
    match kind {
        ObjectKind::Fill { outer, holes, params } if dense_fill(params) => {
            let mut r = rings(outer);
            for h in holes {
                r.extend(rings(h));
            }
            r
        }
        ObjectKind::Satin { geometry, params }
            if params.pattern.is_satin() && params.spacing_mm <= COVER_SPACING_MM =>
        {
            let side = |p: &Path| -> Vec<Point> { rings(p).into_iter().flatten().collect() };
            let mut ring = side(&geometry.rail_a);
            ring.extend(side(&geometry.rail_b).into_iter().rev());
            vec![ring]
        }
        _ => Vec::new(),
    }
}

/// A fill whose rows cover all of its shape.
fn dense_fill(p: &FillParams) -> bool {
    p.spacing_mm <= COVER_SPACING_MM
        && matches!(
            p.pattern,
            FillPattern::Tatami
                | FillPattern::Columns { .. }
                | FillPattern::Square
                | FillPattern::DoubleSquare { .. }
                | FillPattern::Contour
                | FillPattern::Offset
                | FillPattern::Spiral
                | FillPattern::Florentine { .. }
                | FillPattern::ProgramSplit { .. }
                | FillPattern::FlexiSplit { .. }
                | FillPattern::UserSplit { .. }
        )
}

impl Cover {
    /// The cover of the objects of `design` (`places`: the place in the sew
    /// order of each object that sews). `None` if no object covers.
    pub fn new(design: &Design, places: &HashMap<ObjectId, u32>, to_out: &Transform) -> Option<Cover> {
        let mut shapes: Vec<(u32, Vec<Vec<Point>>)> = Vec::new();
        for o in design.objects() {
            let Some(&place) = places.get(&o.id) else { continue };
            let rings = cover_rings(&o.kind, to_out);
            if !rings.is_empty() {
                shapes.push((place, rings));
            }
        }
        let b = BBox::from_points(shapes.iter().flat_map(|(_, r)| r.iter().flatten().copied()))?;
        let margin = END_MM + CELL_MM;
        let (w, h) = (b.width() + 2.0 * margin, b.height() + 2.0 * margin);
        let cell = CELL_MM.max(w.max(h) / MAX_SIDE as f64);
        let (nx, ny) = ((w / cell).ceil() as usize + 1, (h / cell).ceil() as usize + 1);
        let mut cover = Cover {
            min: Point::new(b.min.x - margin, b.min.y - margin),
            cell,
            nx,
            ny,
            last: vec![0; nx * ny],
        };
        // The sew order: a later object writes over an earlier one.
        shapes.sort_by_key(|(place, _)| *place);
        for (place, rings) in &shapes {
            cover.fill(rings, place + 1);
        }
        Some(cover)
    }

    /// Sets the cells whose centers are inside `rings` (even-odd) to `v`.
    fn fill(&mut self, rings: &[Vec<Point>], v: u32) {
        let Some(b) = BBox::from_points(rings.iter().flatten().copied()) else {
            return;
        };
        let row = |y: f64| ((y - self.min.y) / self.cell - 0.5).max(0.0);
        let (j0, j1) = (
            row(b.min.y).ceil() as usize,
            (row(b.max.y).floor() as usize).min(self.ny - 1),
        );
        let mut xs = Vec::new();
        for j in j0..=j1 {
            let y = self.min.y + (j as f64 + 0.5) * self.cell;
            xs.clear();
            for r in rings {
                for (k, &p) in r.iter().enumerate() {
                    let q = r[(k + 1) % r.len()];
                    if (p.y <= y) != (q.y <= y) {
                        xs.push(p.x + (y - p.y) / (q.y - p.y) * (q.x - p.x));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for &[x0, x1] in xs.as_chunks::<2>().0 {
                let col = |x: f64| (x - self.min.x) / self.cell - 0.5;
                let i0 = col(x0).ceil().max(0.0) as usize;
                let i1 = col(x1).floor();
                if i1 < 0.0 {
                    continue;
                }
                for i in i0..=(i1 as usize).min(self.nx - 1) {
                    self.last[j * self.nx + i] = v;
                }
            }
        }
    }

    fn cell_of(&self, p: Point) -> Option<(usize, usize)> {
        let (x, y) = ((p.x - self.min.x) / self.cell, (p.y - self.min.y) / self.cell);
        (x >= 0.0 && y >= 0.0 && (x as usize) < self.nx && (y as usize) < self.ny).then_some((x as usize, y as usize))
    }

    fn center(&self, (i, j): (usize, usize)) -> Point {
        Point::new(
            self.min.x + (i as f64 + 0.5) * self.cell,
            self.min.y + (j as f64 + 0.5) * self.cell,
        )
    }

    /// The point `p` is hidden: covered by an object at the place `from`
    /// or later, or near an end of the way (`ends`).
    fn hidden_at(&self, p: Point, from: u32, ends: (Point, Point)) -> bool {
        if p.distance(ends.0) <= END_MM || p.distance(ends.1) <= END_MM {
            return true;
        }
        self.cell_of(p).is_some_and(|(i, j)| self.last[j * self.nx + i] > from)
    }

    /// Each point of the line from `p` to `q` is hidden.
    fn line_hidden(&self, p: Point, q: Point, from: u32, ends: (Point, Point)) -> bool {
        let n = (p.distance(q) / (self.cell / 2.0)).ceil().max(1.0) as usize;
        (0..=n).all(|k| self.hidden_at(p.lerp(q, k as f64 / n as f64), from, ends))
    }

    /// A hidden way from `a` to `b` for the object at the place `from` in
    /// the sew order (objects at this place or later cover it): the points
    /// of the way, `a` and `b` included. `None` if no way is hidden or the
    /// way is longer than [`MAX_TRAVEL_MM`].
    pub fn way(&self, a: Point, b: Point, from: u32) -> Option<Vec<Point>> {
        if a.distance(b) > MAX_TRAVEL_MM {
            return None;
        }
        let ends = (a, b);
        if self.line_hidden(a, b, from, ends) {
            return Some(vec![a, b]);
        }
        let cells = self.search(a, b, from)?;
        let mut pts: Vec<Point> = cells.iter().map(|&c| self.center(c)).collect();
        pts[0] = a;
        if let Some(l) = pts.last_mut() {
            *l = b;
        }
        // Cut the corners: from each point, the farthest point that a
        // hidden straight line reaches.
        let mut way = vec![a];
        let mut i = 0;
        while i + 1 < pts.len() {
            let mut k = i + 1;
            for m in (i + 2..pts.len()).rev() {
                if self.line_hidden(pts[i], pts[m], from, ends) {
                    k = m;
                    break;
                }
            }
            way.push(pts[k]);
            i = k;
        }
        let len: f64 = way.windows(2).map(|w| w[0].distance(w[1])).sum();
        (len <= MAX_TRAVEL_MM).then_some(way)
    }

    /// The shortest way of cells from the cell of `a` to the cell of `b`
    /// through hidden cells (8 neighbors).
    fn search(&self, a: Point, b: Point, from: u32) -> Option<Vec<(usize, usize)>> {
        let (start, goal) = (self.cell_of(a)?, self.cell_of(b)?);
        let ends = (a, b);
        let idx = |(i, j): (usize, usize)| j * self.nx + i;
        let mut dist: HashMap<usize, f64> = HashMap::new();
        let mut prev: HashMap<usize, (usize, usize)> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let h = |c: (usize, usize)| self.center(c).distance(self.center(goal));
        dist.insert(idx(start), 0.0);
        heap.push(Node {
            cost: h(start),
            cell: start,
        });
        let mut seen = 0;
        while let Some(Node { cell, .. }) = heap.pop() {
            if cell == goal {
                let mut out = vec![goal];
                let mut c = goal;
                while let Some(&p) = prev.get(&idx(c)) {
                    out.push(p);
                    c = p;
                }
                out.reverse();
                return Some(out);
            }
            seen += 1;
            if seen > MAX_SEARCH {
                return None;
            }
            let d = dist[&idx(cell)];
            for (di, dj) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (ni, nj) = (cell.0 as i64 + di, cell.1 as i64 + dj);
                if ni < 0 || nj < 0 || ni as usize >= self.nx || nj as usize >= self.ny {
                    continue;
                }
                let n = (ni as usize, nj as usize);
                if !self.hidden_at(self.center(n), from, ends) {
                    continue;
                }
                let nd = d + if di != 0 && dj != 0 {
                    std::f64::consts::SQRT_2
                } else {
                    1.0
                } * self.cell;
                // A way longer than a travel can be is not wanted.
                if nd + h(n) > MAX_TRAVEL_MM {
                    continue;
                }
                if dist.get(&idx(n)).is_none_or(|&old| nd + 1e-9 < old) {
                    dist.insert(idx(n), nd);
                    prev.insert(idx(n), cell);
                    heap.push(Node {
                        cost: nd + h(n),
                        cell: n,
                    });
                }
            }
        }
        None
    }
}

/// A cell of the search and its cost (the way so far and the straight
/// line to the goal). The heap gives the lowest cost first.
#[derive(Debug, Clone, Copy)]
struct Node {
    cost: f64,
    cell: (usize, usize),
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // The lowest cost first; on a tie the lower cell, so the result is the same each time.
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{Rgb, StitchKind, TrimMode};

    use super::*;
    use crate::plan::{PlanOptions, StitchPlan, build_plan};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
        Path::new()
            .move_to(Point::new(x0, y0))
            .line_to(Point::new(x1, y0))
            .line_to(Point::new(x1, y1))
            .line_to(Point::new(x0, y1))
            .close()
    }

    fn fill(outer: Path, holes: Vec<Path>) -> ObjectKind {
        ObjectKind::Fill {
            outer,
            holes,
            params: FillParams::default(),
        }
    }

    /// Two red squares 20 mm apart (trim mode `trim`), then a blue fill
    /// `cover` (with `holes`) that sews after them.
    fn design(trim: TrimMode, b_at: f64, cover: Path, holes: Vec<Path>) -> (Design, ObjectId, ObjectId) {
        let mut d = Design::new("hidden");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        let a = d.add_object(red, fill(rect(0.0, 0.0, 10.0, 10.0), Vec::new()));
        let b = d.add_object(red, fill(rect(b_at, 0.0, b_at + 10.0, 10.0), Vec::new()));
        d.add_object(blue, fill(cover, holes));
        for id in [a, b] {
            d.object_mut(id).unwrap().functions.trim = trim;
        }
        (d, a, b)
    }

    /// The records between the end of `a` and the start of `b`.
    fn between(p: &StitchPlan, a: ObjectId, b: ObjectId) -> Vec<emb_model::StitchRecord> {
        let span = |id| p.spans.iter().find(|s| s.object == id).unwrap().records.clone();
        let (ea, sb) = (span(a).end, span(b).start);
        p.stitches.records[ea..sb.max(ea)].to_vec()
    }

    fn trims(recs: &[emb_model::StitchRecord]) -> usize {
        recs.iter().filter(|r| r.kind == StitchKind::Trim).count()
    }

    #[test]
    fn a_covered_gap_gets_travel_in_place_of_a_trim() {
        let cover = rect(-1.0, -1.0, 41.0, 11.0);
        let opts = PlanOptions::default();
        let (d, a, b) = design(TrimMode::Auto, 30.0, cover.clone(), Vec::new());
        let auto = build_plan(&d, &opts, &mut Default::default());
        assert_eq!(trims(&between(&auto, a, b)), 1);
        let (d, a, b) = design(TrimMode::Hidden, 30.0, cover, Vec::new());
        let hidden = build_plan(&d, &opts, &mut Default::default());
        let gap = between(&hidden, a, b);
        assert_eq!(trims(&gap), 0, "{gap:?}");
        assert_eq!(hidden.stitches.trim_count() + 1, auto.stitches.trim_count());
        // The travel stitches are short.
        let pts: Vec<Point> = hidden.stitches.records.iter().map(|r| r.pos).collect();
        let ea = hidden.spans.iter().find(|s| s.object == a).unwrap().records.end;
        let sb = hidden.spans.iter().find(|s| s.object == b).unwrap().records.start;
        for w in pts[ea.saturating_sub(1)..=sb.min(pts.len() - 1)].windows(2) {
            assert!(w[0].distance(w[1]) <= TRAVEL_STITCH_MM + 1e-9);
        }
    }

    #[test]
    fn a_gap_with_no_cover_or_a_long_one_gets_a_trim() {
        let opts = PlanOptions::default();
        // The blue fill is far from the gap.
        let (d, a, b) = design(TrimMode::Hidden, 30.0, rect(0.0, 50.0, 40.0, 60.0), Vec::new());
        let p = build_plan(&d, &opts, &mut Default::default());
        assert_eq!(trims(&between(&p, a, b)), 1);
        // The gap is longer than a hidden travel can be.
        let (d, a, b) = design(TrimMode::Hidden, 90.0, rect(-1.0, -1.0, 101.0, 11.0), Vec::new());
        let p = build_plan(&d, &opts, &mut Default::default());
        assert_eq!(trims(&between(&p, a, b)), 1);
    }

    #[test]
    fn the_travel_goes_round_a_part_that_is_not_covered() {
        // A hole in the cover between the squares: the way goes above it.
        let hole = rect(15.0, -1.0, 25.0, 8.0);
        let (d, a, b) = design(TrimMode::Hidden, 30.0, rect(-1.0, -1.0, 41.0, 11.0), vec![hole]);
        let p = build_plan(&d, &PlanOptions::default(), &mut Default::default());
        let gap = between(&p, a, b);
        assert_eq!(trims(&gap), 0, "{gap:?}");
        // The hole in output coordinates.
        let to_out = d.output_transform();
        let (h0, h1) = (
            to_out.apply(Point::new(15.0, -1.0)),
            to_out.apply(Point::new(25.0, 8.0)),
        );
        let inside = |q: Point| {
            q.x > h0.x.min(h1.x) + 0.3
                && q.x < h0.x.max(h1.x) - 0.3
                && q.y > h0.y.min(h1.y) + 0.3
                && q.y < h0.y.max(h1.y) - 0.3
        };
        assert!(gap.iter().all(|r| !inside(r.pos)), "{gap:?}");
    }

    #[test]
    fn a_way_through_the_cells() {
        let mut d = Design::new("cover");
        let t = d.add_thread("t", Rgb(0, 0, 0));
        let c = d.add_object(t, fill(rect(0.0, 0.0, 20.0, 4.0), Vec::new()));
        let places = HashMap::from([(c, 3)]);
        let cover = Cover::new(&d, &places, &Transform::IDENTITY).unwrap();
        let (p, q) = (Point::new(1.0, 2.0), Point::new(19.0, 2.0));
        assert_eq!(cover.way(p, q, 3), Some(vec![p, q]), "covered by the object at place 3");
        assert_eq!(cover.way(p, q, 4), None, "not by an object that sews before");
        // An end on the edge of the cover is near enough.
        assert!(cover.way(Point::new(-0.4, 2.0), q, 3).is_some());
        assert!(cover.way(Point::new(1.0, 6.0), q, 3).is_none());
    }
}
