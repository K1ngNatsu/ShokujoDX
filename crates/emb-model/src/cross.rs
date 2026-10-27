//! Cross-stitch objects (ADR 0038): crosses on a grid of square cells.
//!
//! Cell (column `c`, row `r`) has its top-left corner at
//! `origin + c·u + r·v`. Rows go down (+Y) and columns go right (+X) on a
//! new grid. Backstitch lines join points of the half grid: half-grid
//! point `(x, y)` is at `origin + (x/2)·u + (y/2)·v`.

use emb_geom::{BBox, Point, Transform};
use serde::{Deserialize, Serialize};

/// Cell size of a new grid, in mm.
pub const DEFAULT_CELL_MM: f64 = 2.5;
/// Smallest and largest cell size, in mm.
pub const MIN_CELL_MM: f64 = 1.0;
pub const MAX_CELL_MM: f64 = 10.0;

/// A corner of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomRight,
        Corner::BottomLeft,
    ];

    /// The corner in cell units, from the top-left corner of the cell.
    pub fn offset(self) -> (f64, f64) {
        match self {
            Corner::TopLeft => (0.0, 0.0),
            Corner::TopRight => (1.0, 0.0),
            Corner::BottomRight => (1.0, 1.0),
            Corner::BottomLeft => (0.0, 1.0),
        }
    }

    /// The corner across the cell.
    pub fn opposite(self) -> Corner {
        match self {
            Corner::TopLeft => Corner::BottomRight,
            Corner::TopRight => Corner::BottomLeft,
            Corner::BottomRight => Corner::TopLeft,
            Corner::BottomLeft => Corner::TopRight,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Corner::TopLeft => "tl",
            Corner::TopRight => "tr",
            Corner::BottomRight => "br",
            Corner::BottomLeft => "bl",
        }
    }
}

/// What a cell has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Part {
    /// Both legs; the `\` leg is on top.
    #[default]
    Full,
    /// The `/` leg only (bottom left to top right).
    HalfUp,
    /// The `\` leg only (top left to bottom right).
    HalfDown,
    /// From the corner to the center of the cell.
    Quarter(Corner),
    /// The half cross that does not touch the corner, and a quarter cross
    /// from the corner.
    ThreeQuarter(Corner),
}

impl Part {
    /// All parts, in the order of the tool list.
    pub fn all() -> Vec<Part> {
        let mut v = vec![Part::Full, Part::HalfUp, Part::HalfDown];
        v.extend(Corner::ALL.map(Part::Quarter));
        v.extend(Corner::ALL.map(Part::ThreeQuarter));
        v
    }

    /// A short name for menus and the status line.
    pub fn name(self) -> &'static str {
        match self {
            Part::Full => "Full cross",
            Part::HalfUp => "Half cross /",
            Part::HalfDown => "Half cross \\",
            Part::Quarter(_) => "Quarter cross",
            Part::ThreeQuarter(_) => "Three-quarter cross",
        }
    }

    /// The same kind of part with a different corner (quarter and
    /// three-quarter crosses); other parts do not change.
    pub fn at_corner(self, c: Corner) -> Part {
        match self {
            Part::Quarter(_) => Part::Quarter(c),
            Part::ThreeQuarter(_) => Part::ThreeQuarter(c),
            p => p,
        }
    }

    /// The legs of the part, in cell units from the top-left corner of the
    /// cell. The top leg is last.
    pub fn legs(self) -> Vec<((f64, f64), (f64, f64))> {
        let center = (0.5, 0.5);
        let up = ((0.0, 1.0), (1.0, 0.0));
        let down = ((0.0, 0.0), (1.0, 1.0));
        match self {
            Part::Full => vec![up, down],
            Part::HalfUp => vec![up],
            Part::HalfDown => vec![down],
            Part::Quarter(c) => vec![(c.offset(), center)],
            Part::ThreeQuarter(c) => {
                let half = match c {
                    Corner::TopLeft | Corner::BottomRight => up,
                    Corner::TopRight | Corner::BottomLeft => down,
                };
                vec![(c.offset(), center), half]
            }
        }
    }
}

impl From<Part> for String {
    fn from(p: Part) -> String {
        match p {
            Part::Full => "x".into(),
            Part::HalfUp => "/".into(),
            Part::HalfDown => "\\".into(),
            Part::Quarter(c) => format!("q{}", c.code()),
            Part::ThreeQuarter(c) => format!("t{}", c.code()),
        }
    }
}

impl TryFrom<String> for Part {
    type Error = String;

    fn try_from(s: String) -> Result<Part, String> {
        match s.as_str() {
            "x" => return Ok(Part::Full),
            "/" => return Ok(Part::HalfUp),
            "\\" => return Ok(Part::HalfDown),
            _ => {}
        }
        let corner = Corner::ALL.into_iter().find(|c| s.get(1..) == Some(c.code()));
        match (s.get(..1), corner) {
            (Some("q"), Some(c)) => Ok(Part::Quarter(c)),
            (Some("t"), Some(c)) => Ok(Part::ThreeQuarter(c)),
            _ => Err(format!("unknown cross-stitch cell \"{s}\"")),
        }
    }
}

/// One cell of the grid with its part. The saved form is `[col, row, part]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "(i32, i32, Part)", into = "(i32, i32, Part)")]
pub struct Cell {
    // The field order gives the sort order: row by row.
    pub row: i32,
    pub col: i32,
    pub part: Part,
}

impl From<(i32, i32, Part)> for Cell {
    fn from((col, row, part): (i32, i32, Part)) -> Cell {
        Cell { row, col, part }
    }
}

impl From<Cell> for (i32, i32, Part) {
    fn from(c: Cell) -> (i32, i32, Part) {
        (c.col, c.row, c.part)
    }
}

/// How the full crosses of a row sew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossOrder {
    /// The `/` legs out and the `\` legs back, so all top legs go the same
    /// way. A travel stitch goes along the side of each cell.
    Rows,
    /// The two legs in turn, out and back: no travel stitches between the
    /// cells, but the top leg changes from cell to cell.
    #[default]
    Zigzag,
}

/// Stitch settings of a cross-stitch object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CrossParams {
    /// How many times each leg sews (1, 3 or 5).
    pub passes: u32,
    pub order: CrossOrder,
    /// How many times each backstitch line sews (1 or 3).
    pub line_repeats: u32,
}

impl Default for CrossParams {
    fn default() -> Self {
        CrossParams {
            passes: 1,
            order: CrossOrder::Zigzag,
            line_repeats: 1,
        }
    }
}

/// A backstitch line between two points of the half grid.
pub type Line = [[i32; 2]; 2];

/// Crosses on a grid, in one thread color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossStitch {
    /// The top-left corner of cell (0, 0).
    pub origin: Point,
    /// One cell along a row (to the next column).
    pub u: Point,
    /// One cell down a column (to the next row).
    pub v: Point,
    /// Sorted by row and column; one entry for each cell.
    #[serde(default)]
    pub cells: Vec<Cell>,
    /// Backstitch lines, each with its first point before its second.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Line>,
    #[serde(default)]
    pub params: CrossParams,
}

impl CrossStitch {
    /// An empty grid with square cells of `cell_mm`, with the corner of
    /// cell (0, 0) at `origin`.
    pub fn new(origin: Point, cell_mm: f64) -> CrossStitch {
        let c = cell_mm.clamp(MIN_CELL_MM, MAX_CELL_MM);
        CrossStitch {
            origin,
            u: Point::new(c, 0.0),
            v: Point::new(0.0, c),
            cells: Vec::new(),
            lines: Vec::new(),
            params: CrossParams::default(),
        }
    }

    /// An empty object on the same grid, with the same settings.
    pub fn same_grid_empty(&self) -> CrossStitch {
        CrossStitch {
            cells: Vec::new(),
            lines: Vec::new(),
            ..self.clone()
        }
    }

    /// The cell size in mm (the length of one cell along a row).
    pub fn cell_mm(&self) -> f64 {
        self.u.length()
    }

    /// Changes the cell size and keeps the top-left corner of cell (0, 0).
    pub fn set_cell_mm(&mut self, mm: f64) {
        let mm = mm.clamp(MIN_CELL_MM, MAX_CELL_MM);
        for w in [&mut self.u, &mut self.v] {
            if let Some(n) = w.normalized() {
                *w = n * mm;
            }
        }
    }

    /// Grid units (cells) to mm.
    pub fn grid_transform(&self) -> Transform {
        Transform {
            a: self.u.x,
            b: self.u.y,
            c: self.v.x,
            d: self.v.y,
            e: self.origin.x,
            f: self.origin.y,
        }
    }

    /// The point at `(x, y)` cells from the top-left corner of cell (0, 0).
    pub fn at(&self, x: f64, y: f64) -> Point {
        self.origin + self.u * x + self.v * y
    }

    /// Half-grid point to mm.
    pub fn half_point(&self, h: [i32; 2]) -> Point {
        self.at(f64::from(h[0]) / 2.0, f64::from(h[1]) / 2.0)
    }

    /// The point `p` (mm) in cells, or `None` for a flat grid.
    pub fn to_grid(&self, p: Point) -> Option<Point> {
        self.grid_transform().inverse().map(|t| t.apply(p))
    }

    /// The cell that holds `p`, and the corner of that cell nearest to it.
    pub fn cell_at(&self, p: Point) -> Option<(i32, i32, Corner)> {
        let g = self.to_grid(p)?;
        let (c, r) = (g.x.floor(), g.y.floor());
        if !c.is_finite() || !r.is_finite() || c.abs() > 1e6 || r.abs() > 1e6 {
            return None;
        }
        let (fx, fy) = (g.x - c, g.y - r);
        let corner = match (fx >= 0.5, fy >= 0.5) {
            (false, false) => Corner::TopLeft,
            (true, false) => Corner::TopRight,
            (true, true) => Corner::BottomRight,
            (false, true) => Corner::BottomLeft,
        };
        Some((c as i32, r as i32, corner))
    }

    /// The half-grid point nearest to `p`.
    pub fn half_point_near(&self, p: Point) -> Option<[i32; 2]> {
        let g = self.to_grid(p)?;
        let (x, y) = ((g.x * 2.0).round(), (g.y * 2.0).round());
        (x.abs() < 2e6 && y.abs() < 2e6).then_some([x as i32, y as i32])
    }

    fn find(&self, col: i32, row: i32) -> Result<usize, usize> {
        self.cells.binary_search_by(|c| (c.row, c.col).cmp(&(row, col)))
    }

    /// The part in a cell.
    pub fn get(&self, col: i32, row: i32) -> Option<Part> {
        self.find(col, row).ok().map(|i| self.cells[i].part)
    }

    /// Puts `part` into a cell (in place of what it had). Returns `true`
    /// if the cell changed.
    pub fn set(&mut self, col: i32, row: i32, part: Part) -> bool {
        match self.find(col, row) {
            Ok(i) if self.cells[i].part == part => false,
            Ok(i) => {
                self.cells[i].part = part;
                true
            }
            Err(i) => {
                self.cells.insert(i, Cell { row, col, part });
                true
            }
        }
    }

    /// Empties a cell. Returns `true` if it had a part.
    pub fn remove(&mut self, col: i32, row: i32) -> bool {
        match self.find(col, row) {
            Ok(i) => {
                self.cells.remove(i);
                true
            }
            Err(_) => false,
        }
    }

    /// Adds a backstitch line. Returns `false` for a line of no length or
    /// a line that is there already.
    pub fn add_line(&mut self, a: [i32; 2], b: [i32; 2]) -> bool {
        if a == b {
            return false;
        }
        let line = if a <= b { [a, b] } else { [b, a] };
        if self.lines.contains(&line) {
            return false;
        }
        self.lines.push(line);
        true
    }

    /// Removes the backstitch lines nearer to `p` than `tol` (mm). Returns
    /// `true` if one went.
    pub fn remove_lines_near(&mut self, p: Point, tol: f64) -> bool {
        let n = self.lines.len();
        let t = self.grid_transform();
        let at = |h: [i32; 2]| t.apply(Point::new(f64::from(h[0]) / 2.0, f64::from(h[1]) / 2.0));
        self.lines.retain(|l| p.distance_to_segment(at(l[0]), at(l[1])) > tol);
        self.lines.len() != n
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.lines.is_empty()
    }

    /// `true` if `o` is on the same grid (the same cells at the same places).
    pub fn same_grid(&self, o: &CrossStitch) -> bool {
        let close = |a: Point, b: Point| a.distance(b) < 1e-6;
        close(self.origin, o.origin) && close(self.u, o.u) && close(self.v, o.v)
    }

    /// The box of the cells and lines, in mm.
    pub fn bbox(&self) -> Option<BBox> {
        let mut g: Option<BBox> = None;
        let mut add = |p: Point| match &mut g {
            Some(b) => b.include(p),
            None => g = Some(BBox::new(p, p)),
        };
        for c in &self.cells {
            add(Point::new(f64::from(c.col), f64::from(c.row)));
            add(Point::new(f64::from(c.col) + 1.0, f64::from(c.row) + 1.0));
        }
        for l in &self.lines {
            for h in l {
                add(Point::new(f64::from(h[0]) / 2.0, f64::from(h[1]) / 2.0));
            }
        }
        // The box of the grid box: the grid is affine, so its corners are enough.
        let g = g?;
        let t = self.grid_transform();
        BBox::from_points(
            [g.min, Point::new(g.max.x, g.min.y), g.max, Point::new(g.min.x, g.max.y)].map(|p| t.apply(p)),
        )
    }

    /// `true` if `p` is in a cell with a part, or nearer than `tol` to a line.
    pub fn hit_test(&self, p: Point, tol: f64) -> bool {
        if let Some((c, r, _)) = self.cell_at(p)
            && self.get(c, r).is_some()
        {
            return true;
        }
        self.lines
            .iter()
            .any(|l| p.distance_to_segment(self.half_point(l[0]), self.half_point(l[1])) <= tol)
    }

    /// Applies the transform to the grid.
    pub fn transform(&mut self, t: &Transform) {
        self.origin = t.apply(self.origin);
        self.u = t.apply_vector(self.u);
        self.v = t.apply_vector(self.v);
    }

    /// The cells whose center is inside the rings (even-odd), in grid
    /// order.
    pub fn cells_inside(&self, rings: &[Vec<Point>]) -> Vec<(i32, i32)> {
        let Some(inv) = self.grid_transform().inverse() else {
            return Vec::new();
        };
        let grid: Vec<Vec<Point>> = rings
            .iter()
            .map(|r| r.iter().map(|p| inv.apply(*p)).collect())
            .collect();
        let Some(b) = BBox::from_points(grid.iter().flatten().copied()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let (r0, r1) = (b.min.y.floor() as i32, b.max.y.ceil() as i32);
        let (c0, c1) = (b.min.x.floor() as i32, b.max.x.ceil() as i32);
        for row in r0..r1 {
            for col in c0..c1 {
                let p = Point::new(f64::from(col) + 0.5, f64::from(row) + 0.5);
                let n = grid.iter().filter(|r| emb_geom::region::ring_contains(r, p)).count();
                if n % 2 == 1 {
                    out.push((col, row));
                }
            }
        }
        out
    }

    /// The number of cells with each kind of part: full, half, quarter,
    /// three-quarter.
    pub fn counts(&self) -> [usize; 4] {
        let mut n = [0; 4];
        for c in &self.cells {
            n[match c.part {
                Part::Full => 0,
                Part::HalfUp | Part::HalfDown => 1,
                Part::Quarter(_) => 2,
                Part::ThreeQuarter(_) => 3,
            }] += 1;
        }
        n
    }
}

impl CrossStitch {
    /// The parts of the object that touch: cells next to each other (also
    /// at a corner) are one part. The backstitch lines are one more part.
    /// One part gives one object.
    pub fn components(&self) -> Vec<CrossStitch> {
        let mut seen = vec![false; self.cells.len()];
        let mut out = Vec::new();
        for start in 0..self.cells.len() {
            if seen[start] {
                continue;
            }
            let mut part = self.same_grid_empty();
            let mut todo = vec![start];
            seen[start] = true;
            while let Some(i) = todo.pop() {
                let c = self.cells[i];
                part.set(c.col, c.row, c.part);
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        if let Ok(j) = self.find(c.col + dc, c.row + dr)
                            && !seen[j]
                        {
                            seen[j] = true;
                            todo.push(j);
                        }
                    }
                }
            }
            out.push(part);
        }
        if !self.lines.is_empty() {
            let mut lines = self.same_grid_empty();
            lines.lines = self.lines.clone();
            out.push(lines);
        }
        out
    }
}

/// Edits of the cross-stitch objects on one grid. A cell has one color: a
/// cell that goes into one object leaves the other objects on the grid.
/// Locked objects do not change.
impl crate::Design {
    /// The unlocked cross-stitch objects on the grid of `grid`, in sew order.
    pub fn cross_objects_on(&self, grid: &CrossStitch) -> Vec<crate::ObjectId> {
        self.objects()
            .iter()
            .filter(|o| !o.locked && matches!(&o.kind, crate::ObjectKind::CrossStitch(x) if x.same_grid(grid)))
            .map(|o| o.id)
            .collect()
    }

    fn cross_mut(&mut self, id: crate::ObjectId) -> Option<&mut CrossStitch> {
        match &mut self.object_mut(id)?.kind {
            crate::ObjectKind::CrossStitch(x) => Some(x),
            _ => None,
        }
    }

    fn cross_ref(&self, id: crate::ObjectId) -> Option<&CrossStitch> {
        match &self.object(id)?.kind {
            crate::ObjectKind::CrossStitch(x) => Some(x),
            _ => None,
        }
    }

    /// The last object on the grid with the thread, in the sew order.
    fn cross_with_thread(&self, grid: &CrossStitch, thread: crate::ThreadId) -> Option<crate::ObjectId> {
        self.cross_objects_on(grid)
            .into_iter()
            .rev()
            .find(|id| self.object(*id).is_some_and(|o| o.thread == thread))
    }

    /// The object on the grid with the thread that gets new cells, or a new
    /// object at the end of the sew order.
    fn cross_target(&mut self, grid: &CrossStitch, thread: crate::ThreadId) -> crate::ObjectId {
        self.cross_with_thread(grid, thread).unwrap_or_else(|| {
            self.add_object(thread, crate::ObjectKind::CrossStitch(Box::new(grid.same_grid_empty())))
        })
    }

    /// Removes the cells from the objects on the grid other than `keep`,
    /// and the objects that become empty. Returns `true` if one changed.
    fn cross_take_cells(&mut self, grid: &CrossStitch, cells: &[(i32, i32)], keep: Option<crate::ObjectId>) -> bool {
        let mut changed = false;
        for id in self.cross_objects_on(grid) {
            if Some(id) == keep {
                continue;
            }
            let has = self
                .cross_ref(id)
                .is_some_and(|x| cells.iter().any(|&(c, r)| x.get(c, r).is_some()));
            if !has {
                continue;
            }
            if let Some(x) = self.cross_mut(id) {
                for &(c, r) in cells {
                    x.remove(c, r);
                }
            }
            changed = true;
            if self.cross_ref(id).is_some_and(CrossStitch::is_empty) {
                self.remove_object(id);
            }
        }
        changed
    }

    /// Puts parts into cells of the grid, in the color of `thread`. Returns
    /// the object that has them, or `None` if nothing changed.
    pub fn cross_paint(
        &mut self,
        grid: &CrossStitch,
        thread: crate::ThreadId,
        cells: &[(i32, i32, Part)],
    ) -> Option<crate::ObjectId> {
        if cells.is_empty() {
            return None;
        }
        let at: Vec<(i32, i32)> = cells.iter().map(|&(c, r, _)| (c, r)).collect();
        let target = self.cross_with_thread(grid, thread);
        let has_all = target
            .and_then(|id| self.cross_ref(id))
            .is_some_and(|x| cells.iter().all(|&(c, r, p)| x.get(c, r) == Some(p)));
        let elsewhere = self
            .cross_objects_on(grid)
            .into_iter()
            .filter(|id| Some(*id) != target)
            .any(|id| {
                self.cross_ref(id)
                    .is_some_and(|x| at.iter().any(|&(c, r)| x.get(c, r).is_some()))
            });
        if has_all && !elsewhere {
            return None;
        }
        let target = self.cross_target(grid, thread);
        self.cross_take_cells(grid, &at, Some(target));
        if let Some(x) = self.cross_mut(target) {
            for &(c, r, p) in cells {
                x.set(c, r, p);
            }
        }
        Some(target)
    }

    /// Empties cells of the grid in all its objects. Returns `true` if one
    /// had a part.
    pub fn cross_erase(&mut self, grid: &CrossStitch, cells: &[(i32, i32)]) -> bool {
        self.cross_take_cells(grid, cells, None)
    }

    /// Removes the backstitch lines of the grid nearer to `p` than `tol`
    /// (mm). Returns `true` if one went.
    pub fn cross_erase_lines(&mut self, grid: &CrossStitch, p: Point, tol: f64) -> bool {
        let mut changed = false;
        for id in self.cross_objects_on(grid) {
            let near = self.cross_ref(id).is_some_and(|x| x.clone().remove_lines_near(p, tol));
            if !near {
                continue;
            }
            if let Some(x) = self.cross_mut(id) {
                x.remove_lines_near(p, tol);
            }
            changed = true;
            if self.cross_ref(id).is_some_and(CrossStitch::is_empty) {
                self.remove_object(id);
            }
        }
        changed
    }

    /// Adds a backstitch line to the grid, in the color of `thread`.
    /// Returns the object that has it, or `None` for a line of no length or
    /// a line that the object has already.
    pub fn cross_add_line(
        &mut self,
        grid: &CrossStitch,
        thread: crate::ThreadId,
        a: [i32; 2],
        b: [i32; 2],
    ) -> Option<crate::ObjectId> {
        let line = if a <= b { [a, b] } else { [b, a] };
        let there = self
            .cross_with_thread(grid, thread)
            .and_then(|id| self.cross_ref(id))
            .is_some_and(|x| x.lines.contains(&line));
        if a == b || there {
            return None;
        }
        let target = self.cross_target(grid, thread);
        if let Some(x) = self.cross_mut(target) {
            x.add_line(a, b);
        }
        Some(target)
    }
}

/// The cell size in mm of an Aida fabric with `count` cells to the inch.
pub fn aida_cell_mm(count: u32) -> f64 {
    25.4 / f64::from(count.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> CrossStitch {
        CrossStitch::new(Point::new(10.0, 20.0), 2.5)
    }

    #[test]
    fn cells_stay_sorted_and_unique() {
        let mut x = grid();
        assert!(x.set(3, 1, Part::Full));
        assert!(x.set(0, 1, Part::HalfUp));
        assert!(x.set(5, 0, Part::Full));
        assert!(!x.set(5, 0, Part::Full), "no change");
        assert!(x.set(5, 0, Part::Quarter(Corner::TopLeft)));
        let order: Vec<(i32, i32)> = x.cells.iter().map(|c| (c.col, c.row)).collect();
        assert_eq!(order, vec![(5, 0), (0, 1), (3, 1)]);
        assert_eq!(x.get(5, 0), Some(Part::Quarter(Corner::TopLeft)));
        assert!(x.remove(0, 1));
        assert!(!x.remove(0, 1));
        assert_eq!(x.cells.len(), 2);
    }

    #[test]
    fn cell_at_finds_the_cell_and_corner() {
        let x = grid();
        // Cell (2, 3) is from (15, 27.5) to (17.5, 30).
        assert_eq!(x.cell_at(Point::new(15.2, 27.6)), Some((2, 3, Corner::TopLeft)));
        assert_eq!(x.cell_at(Point::new(17.4, 29.9)), Some((2, 3, Corner::BottomRight)));
        assert_eq!(x.cell_at(Point::new(9.0, 19.0)), Some((-1, -1, Corner::BottomRight)));
        assert_eq!(x.half_point_near(Point::new(16.3, 28.7)), Some([5, 7]));
    }

    #[test]
    fn transform_keeps_cells_on_the_grid() {
        let mut x = grid();
        x.set(1, 1, Part::Full);
        let center = x.at(1.5, 1.5);
        let t = Transform::rotate_deg(30.0).then(Transform::translate(5.0, -3.0));
        x.transform(&t);
        let moved = t.apply(center);
        assert!(x.at(1.5, 1.5).distance(moved) < 1e-9);
        let (c, r, _) = x.cell_at(moved).unwrap();
        assert_eq!((c, r), (1, 1));
        assert!(x.hit_test(moved, 0.1));
        assert!((x.cell_mm() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn bbox_covers_cells_and_lines() {
        let mut x = grid();
        x.set(0, 0, Part::Full);
        x.set(3, 1, Part::HalfUp);
        x.add_line([0, 0], [10, 0]);
        let b = x.bbox().unwrap();
        assert!((b.min.x - 10.0).abs() < 1e-9 && (b.min.y - 20.0).abs() < 1e-9);
        // Columns 0 to 4 (the line goes to x = 5 cells), rows 0 to 1.
        assert!((b.max.x - 22.5).abs() < 1e-9, "{b:?}");
        assert!((b.max.y - 25.0).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn lines_are_unique_and_erasable() {
        let mut x = grid();
        assert!(x.add_line([2, 0], [0, 0]));
        assert!(!x.add_line([0, 0], [2, 0]), "same line");
        assert!(!x.add_line([1, 1], [1, 1]), "no length");
        assert_eq!(x.lines, vec![[[0, 0], [2, 0]]]);
        assert!(!x.remove_lines_near(Point::new(11.0, 22.0), 0.5));
        assert!(x.remove_lines_near(Point::new(11.0, 20.2), 0.5));
        assert!(x.lines.is_empty());
    }

    #[test]
    fn cells_inside_uses_cell_centers() {
        let x = CrossStitch::new(Point::ZERO, 2.0);
        // A 6 × 4 mm box from (0, 0), with a 2 × 2 hole at (2, 0).
        let outer = vec![
            Point::new(0.0, 0.0),
            Point::new(6.0, 0.0),
            Point::new(6.0, 4.0),
            Point::new(0.0, 4.0),
        ];
        let hole = vec![
            Point::new(2.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 2.0),
            Point::new(2.0, 2.0),
        ];
        let cells = x.cells_inside(&[outer, hole]);
        assert_eq!(cells, vec![(0, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn saved_form_is_compact_and_reads_back() {
        let mut x = grid();
        for (i, p) in Part::all().into_iter().enumerate() {
            x.set(i as i32, 0, p);
        }
        x.add_line([0, 0], [1, 1]);
        let json = serde_json::to_string(&x).unwrap();
        assert!(json.contains(r#"[0,0,"x"]"#), "{json}");
        assert!(json.contains(r#"[3,0,"qtl"]"#), "{json}");
        let back: CrossStitch = serde_json::from_str(&json).unwrap();
        assert_eq!(back, x);
        assert!(serde_json::from_str::<Part>(r#""qxx""#).is_err());
    }

    fn design_with_threads() -> (crate::Design, crate::ThreadId, crate::ThreadId) {
        let mut d = crate::Design::new("t");
        let red = d.add_thread("Red", crate::Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", crate::Rgb(0, 0, 200));
        (d, red, blue)
    }

    fn cross_of(d: &crate::Design, id: crate::ObjectId) -> CrossStitch {
        match &d.object(id).unwrap().kind {
            crate::ObjectKind::CrossStitch(x) => (**x).clone(),
            _ => panic!("not a cross-stitch object"),
        }
    }

    #[test]
    fn paint_makes_one_object_for_each_color_and_a_cell_has_one_color() {
        let (mut d, red, blue) = design_with_threads();
        let g = grid();
        let a = d
            .cross_paint(&g, red, &[(0, 0, Part::Full), (1, 0, Part::Full)])
            .unwrap();
        let b = d.cross_paint(&g, red, &[(2, 0, Part::Full)]).unwrap();
        assert_eq!(a, b, "the same object");
        assert_eq!(d.cross_paint(&g, red, &[(2, 0, Part::Full)]), None, "no change");
        let c = d.cross_paint(&g, blue, &[(1, 0, Part::HalfUp)]).unwrap();
        assert_ne!(a, c);
        assert_eq!(cross_of(&d, a).get(1, 0), None, "the cell left the red object");
        assert_eq!(cross_of(&d, c).get(1, 0), Some(Part::HalfUp));
        // Painting the blue cell red again empties the blue object: it goes.
        d.cross_paint(&g, red, &[(1, 0, Part::Full)]);
        assert!(d.object(c).is_none());
        assert_eq!(d.objects().len(), 1);
        // Another grid: another object.
        let other = CrossStitch::new(Point::new(1.0, 1.0), 2.5);
        let e = d.cross_paint(&other, red, &[(0, 0, Part::Full)]).unwrap();
        assert_ne!(e, a);
    }

    #[test]
    fn erase_removes_cells_and_lines_and_empty_objects() {
        let (mut d, red, blue) = design_with_threads();
        let g = grid();
        let a = d
            .cross_paint(&g, red, &[(0, 0, Part::Full), (1, 0, Part::Full)])
            .unwrap();
        let b = d.cross_paint(&g, blue, &[(5, 5, Part::Full)]).unwrap();
        assert!(d.cross_erase(&g, &[(1, 0), (5, 5)]));
        assert!(!d.cross_erase(&g, &[(9, 9)]));
        assert_eq!(cross_of(&d, a).cells.len(), 1);
        assert!(d.object(b).is_none());
        let l = d.cross_add_line(&g, blue, [0, 0], [4, 0]).unwrap();
        assert_eq!(d.cross_add_line(&g, blue, [4, 0], [0, 0]), None, "there already");
        assert!(d.cross_erase_lines(&g, Point::new(12.0, 20.0), 0.5));
        assert!(d.object(l).is_none());
    }

    #[test]
    fn locked_objects_do_not_change() {
        let (mut d, red, _) = design_with_threads();
        let g = grid();
        let a = d.cross_paint(&g, red, &[(0, 0, Part::Full)]).unwrap();
        d.object_mut(a).unwrap().locked = true;
        assert!(!d.cross_erase(&g, &[(0, 0)]));
        let b = d.cross_paint(&g, red, &[(1, 0, Part::Full)]).unwrap();
        assert_ne!(a, b, "a new object");
    }

    #[test]
    fn components_split_at_gaps_and_join_at_corners() {
        let mut x = grid();
        x.set(0, 0, Part::Full);
        x.set(1, 1, Part::Full);
        x.set(5, 0, Part::Full);
        x.add_line([0, 0], [2, 0]);
        let parts = x.components();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].cells.len(), 2, "corner neighbors");
        assert_eq!(parts[1].cells.len(), 1);
        assert_eq!(parts[2].lines.len(), 1);
        assert!(parts.iter().all(|p| p.same_grid(&x)));
    }

    #[test]
    fn aida_counts() {
        assert!((aida_cell_mm(14) - 1.814).abs() < 1e-3);
        assert!((aida_cell_mm(10) - 2.54).abs() < 1e-9);
    }

    #[test]
    fn three_quarter_has_the_half_that_misses_the_corner() {
        for c in Corner::ALL {
            let legs = Part::ThreeQuarter(c).legs();
            let half = legs[1];
            assert!(half.0 != c.offset() && half.1 != c.offset(), "{c:?}");
            assert_eq!(legs[0], (c.offset(), (0.5, 0.5)));
        }
    }
}
