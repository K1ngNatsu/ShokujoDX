//! A grid of square cells over a glyph shape: fill, distance transform,
//! thinning and connected parts.
//!
//! Cell `(i, j)` is column `i`, row `j`. Rows go down (as Y). A cell is
//! inside when its center is inside the shape (nonzero winding).

use emb_geom::{BBox, Point};

/// A large distance for cells that are not yet reached (finite, so the
/// distance transform does no `inf - inf`).
const FAR: f64 = 1e18;

#[derive(Debug, Clone)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// The corner of cell (0, 0).
    pub origin: Point,
    pub cell: f64,
    pub inside: Vec<bool>,
}

impl Grid {
    /// An empty grid over `bounds` with `margin` empty cells on each side.
    pub fn new(bounds: BBox, cell: f64, margin: usize) -> Grid {
        let w = (bounds.width() / cell).ceil() as usize + 2 * margin + 1;
        let h = (bounds.height() / cell).ceil() as usize + 2 * margin + 1;
        let m = margin as f64 * cell;
        Grid {
            w,
            h,
            origin: Point::new(bounds.min.x - m, bounds.min.y - m),
            cell,
            inside: vec![false; w * h],
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        j * self.w + i
    }

    pub fn center(&self, i: usize, j: usize) -> Point {
        Point::new(
            self.origin.x + (i as f64 + 0.5) * self.cell,
            self.origin.y + (j as f64 + 0.5) * self.cell,
        )
    }

    pub fn center_of(&self, idx: usize) -> Point {
        self.center(idx % self.w, idx / self.w)
    }

    /// The cell that holds `p`.
    pub fn cell_of(&self, p: Point) -> Option<(usize, usize)> {
        let i = ((p.x - self.origin.x) / self.cell).floor();
        let j = ((p.y - self.origin.y) / self.cell).floor();
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.w && (j as usize) < self.h).then_some((i as usize, j as usize))
    }

    pub fn is_inside(&self, p: Point) -> bool {
        self.cell_of(p).is_some_and(|(i, j)| self.inside[self.idx(i, j)])
    }

    /// Marks the cells inside `rings` (nonzero winding) as inside.
    pub fn fill(&mut self, rings: &[Vec<Point>]) {
        let mut mask = std::mem::take(&mut self.inside);
        fill_into(self, rings, &mut mask);
        self.inside = mask;
    }

    /// The 8 neighbors of a cell that are in the grid.
    pub fn neighbors(&self, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let (i, j) = ((idx % self.w) as isize, (idx / self.w) as isize);
        const D: [(isize, isize); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];
        D.iter().filter_map(move |&(di, dj)| {
            let (a, b) = (i + di, j + dj);
            (a >= 0 && b >= 0 && (a as usize) < self.w && (b as usize) < self.h)
                .then(|| b as usize * self.w + a as usize)
        })
    }
}

/// Sets `mask` for the cells of `g` whose centers are inside `rings`
/// (nonzero winding). Only the rows that the rings cross are scanned.
pub fn fill_into(g: &Grid, rings: &[Vec<Point>], mask: &mut [bool]) {
    let Some(bb) = BBox::from_points(rings.iter().flatten().copied()) else {
        return;
    };
    let row = |y: f64| (y - g.origin.y) / g.cell - 0.5;
    let j0 = row(bb.min.y).ceil().max(0.0) as usize;
    let j1 = (row(bb.max.y).floor() + 1.0).clamp(0.0, g.h as f64) as usize;
    let mut xs: Vec<(f64, i32)> = Vec::new();
    for j in j0..j1 {
        let y = g.origin.y + (j as f64 + 0.5) * g.cell;
        xs.clear();
        for ring in rings {
            let n = ring.len();
            for k in 0..n {
                let (a, b) = (ring[k], ring[(k + 1) % n]);
                if (a.y <= y) != (b.y <= y) {
                    let x = a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
                    xs.push((x, if b.y > a.y { 1 } else { -1 }));
                }
            }
        }
        xs.sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut wind = 0;
        for k in 0..xs.len() {
            wind += xs[k].1;
            if wind != 0 && k + 1 < xs.len() {
                let col = |x: f64| ((x - g.origin.x) / g.cell - 0.5).ceil().clamp(0.0, g.w as f64) as usize;
                for i in col(xs[k].0)..col(xs[k + 1].0) {
                    mask[j * g.w + i] = true;
                }
            }
        }
    }
}

/// The distance of each inside cell center to the nearest outside cell
/// center, in cells (0 outside). Exact Euclidean distance (Felzenszwalb and
/// Huttenlocher, "Distance Transforms of Sampled Functions", 2012).
pub fn distance(g: &Grid) -> Vec<f64> {
    let (w, h) = (g.w, g.h);
    let mut d: Vec<f64> = g.inside.iter().map(|&v| if v { FAR } else { 0.0 }).collect();
    let n = w.max(h);
    let (mut f, mut out, mut v, mut z) = (vec![0.0; n], vec![0.0; n], vec![0usize; n], vec![0.0; n + 1]);
    for i in 0..w {
        for j in 0..h {
            f[j] = d[j * w + i];
        }
        edt_1d(&f[..h], &mut out[..h], &mut v, &mut z);
        for j in 0..h {
            d[j * w + i] = out[j];
        }
    }
    for j in 0..h {
        f[..w].copy_from_slice(&d[j * w..(j + 1) * w]);
        edt_1d(&f[..w], &mut out[..w], &mut v, &mut z);
        d[j * w..(j + 1) * w].copy_from_slice(&out[..w]);
    }
    d.iter().map(|&x| x.sqrt()).collect()
}

/// Squared distance transform of one line (lower envelope of parabolas).
fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let sect = |q: usize, p: usize| {
        let (qf, pf) = (q as f64, p as f64);
        ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * qf - 2.0 * pf)
    };
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let mut s = sect(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = sect(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, dq) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dv = q as f64 - v[k] as f64;
        *dq = dv * dv + f[v[k]];
    }
}

/// The skeleton of the inside cells: a line one cell wide in the middle of
/// each part (Zhang and Suen thinning, "A fast parallel algorithm for
/// thinning digital patterns", 1984), then corner cells of staircases are
/// removed so that each line cell has two neighbors.
pub fn thin(g: &Grid) -> Vec<bool> {
    let (w, h) = (g.w, g.h);
    let mut s = g.inside.clone();
    let at = |s: &[bool], i: usize, j: usize, di: isize, dj: isize| -> bool {
        let (a, b) = (i as isize + di, j as isize + dj);
        a >= 0 && b >= 0 && (a as usize) < w && (b as usize) < h && s[b as usize * w + a as usize]
    };
    // P2..P9: N, NE, E, SE, S, SW, W, NW.
    let ring = |s: &[bool], i: usize, j: usize| -> [bool; 8] {
        [
            at(s, i, j, 0, -1),
            at(s, i, j, 1, -1),
            at(s, i, j, 1, 0),
            at(s, i, j, 1, 1),
            at(s, i, j, 0, 1),
            at(s, i, j, -1, 1),
            at(s, i, j, -1, 0),
            at(s, i, j, -1, -1),
        ]
    };
    let mut del = Vec::new();
    loop {
        let mut changed = false;
        for step in 0..2 {
            del.clear();
            for j in 0..h {
                for i in 0..w {
                    if !s[j * w + i] {
                        continue;
                    }
                    let p = ring(&s, i, j);
                    let b = p.iter().filter(|&&x| x).count();
                    let a = (0..8).filter(|&k| !p[k] && p[(k + 1) % 8]).count();
                    let (n, e, so, we) = (p[0], p[2], p[4], p[6]);
                    let ok = if step == 0 {
                        !(n && e && so) && !(e && so && we)
                    } else {
                        !(n && e && we) && !(n && so && we)
                    };
                    if (2..=6).contains(&b) && a == 1 && ok {
                        del.push(j * w + i);
                    }
                }
            }
            for &k in &del {
                s[k] = false;
            }
            changed |= !del.is_empty();
        }
        if !changed {
            break;
        }
    }
    // Staircase corners: a cell with two side neighbors at a right angle and
    // nothing on the other side is not needed for connection.
    for j in 0..h {
        for i in 0..w {
            if !s[j * w + i] {
                continue;
            }
            let p = ring(&s, i, j);
            let corner = (0..4).any(|r| {
                let k = 2 * r;
                p[k] && p[(k + 2) % 8] && !p[(k + 4) % 8] && !p[(k + 5) % 8] && !p[(k + 6) % 8]
            });
            if corner {
                s[j * w + i] = false;
            }
        }
    }
    s
}

/// The connected parts of the inside cells (8 neighbors): a label for each
/// cell (`usize::MAX` outside) and the number of parts.
pub fn parts(g: &Grid) -> (Vec<usize>, usize) {
    let mut label = vec![usize::MAX; g.inside.len()];
    let mut n = 0;
    let mut stack = Vec::new();
    for start in 0..g.inside.len() {
        if !g.inside[start] || label[start] != usize::MAX {
            continue;
        }
        label[start] = n;
        stack.push(start);
        while let Some(c) = stack.pop() {
            for q in g.neighbors(c) {
                if g.inside[q] && label[q] == usize::MAX {
                    label[q] = n;
                    stack.push(q);
                }
            }
        }
        n += 1;
    }
    (label, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    fn grid_of(rings: &[Vec<Point>]) -> Grid {
        let bb = BBox::from_points(rings.iter().flatten().copied()).unwrap();
        let mut g = Grid::new(bb, 1.0, 2);
        g.fill(rings);
        g
    }

    #[test]
    fn fill_uses_nonzero_winding() {
        // A square with a hole (other direction) and an overlap (same direction).
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let mut hole = rect(3.0, 3.0, 7.0, 7.0);
        hole.reverse();
        let g = grid_of(&[outer.clone(), hole]);
        assert_eq!(g.inside.iter().filter(|&&v| v).count(), 100 - 16);
        assert!(!g.is_inside(Point::new(5.0, 5.0)) && g.is_inside(Point::new(1.5, 1.5)));
        let g = grid_of(&[outer, rect(3.0, 3.0, 7.0, 7.0)]);
        assert_eq!(g.inside.iter().filter(|&&v| v).count(), 100);
    }

    #[test]
    fn distance_is_euclidean() {
        let g = grid_of(&[rect(0.0, 0.0, 9.0, 9.0)]);
        let d = distance(&g);
        let c = g.cell_of(Point::new(4.5, 4.5)).unwrap();
        // The middle cell is 5 cells from the first outside cell.
        assert!((d[g.idx(c.0, c.1)] - 5.0).abs() < 1e-9);
        let corner = g.cell_of(Point::new(0.5, 0.5)).unwrap();
        assert!((d[g.idx(corner.0, corner.1)] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn thin_bar_is_one_line() {
        let g = grid_of(&[rect(0.0, 0.0, 40.0, 7.0)]);
        let s = thin(&g);
        let cells: Vec<usize> = (0..s.len()).filter(|&k| s[k]).collect();
        assert!(cells.len() > 25, "{}", cells.len());
        // Each cell of the line has at most two neighbors on the line, except near the ends.
        let many = cells
            .iter()
            .filter(|&&k| g.neighbors(k).filter(|&q| s[q]).count() > 2)
            .count();
        assert!(many <= 4, "{many}");
        let (_, n) = parts(&g);
        assert_eq!(n, 1);
    }
}
