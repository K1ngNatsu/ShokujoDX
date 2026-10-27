//! Small linear algebra for the solver (ADR 0068): a sparse Cholesky
//! solution (reverse Cuthill-McKee order, envelope storage) and an
//! orthonormal row basis for the rank.

use std::collections::BTreeMap;

/// The reverse Cuthill-McKee order of a graph: an order that keeps the
/// neighbors of each node near to it, so the envelope is narrow.
fn rcm(adj: &[Vec<usize>]) -> Vec<usize> {
    let m = adj.len();
    let mut seen = vec![false; m];
    let mut order = Vec::with_capacity(m);
    let mut by_degree: Vec<usize> = (0..m).collect();
    by_degree.sort_by_key(|&i| adj[i].len());
    for &start in &by_degree {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            order.push(i);
            let mut next: Vec<usize> = adj[i].iter().copied().filter(|j| !seen[*j]).collect();
            next.sort_by_key(|&j| adj[j].len());
            next.dedup();
            for j in next {
                if !seen[j] {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
    }
    order.reverse();
    order
}

/// Solves `A y = b` for a symmetric positive definite `A` of size `m`,
/// given by its entries on and under the diagonal (`(i, j)` with `i ≥ j`;
/// an entry that is there two times is added). `None` if `A` is not
/// positive definite.
pub(crate) fn sparse_cholesky_solve(m: usize, lower: &BTreeMap<(usize, usize), f64>, b: &[f64]) -> Option<Vec<f64>> {
    let mut adj = vec![Vec::new(); m];
    for &(i, j) in lower.keys() {
        if i != j {
            adj[i].push(j);
            adj[j].push(i);
        }
    }
    let order = rcm(&adj);
    let mut pos = vec![0; m];
    for (k, &o) in order.iter().enumerate() {
        pos[o] = k;
    }
    let place = |i: usize, j: usize| {
        let (a, b) = (pos[i], pos[j]);
        if a >= b { (a, b) } else { (b, a) }
    };
    // Each row r keeps the columns first[r]..=r.
    let mut first: Vec<usize> = (0..m).collect();
    for &(i, j) in lower.keys() {
        let (r, c) = place(i, j);
        first[r] = first[r].min(c);
    }
    let mut rows: Vec<Vec<f64>> = (0..m).map(|r| vec![0.0; r - first[r] + 1]).collect();
    for (&(i, j), &v) in lower {
        let (r, c) = place(i, j);
        rows[r][c - first[r]] += v;
    }
    // The factor L (A = L Lᵀ), in place.
    for r in 0..m {
        let (done, rest) = rows.split_at_mut(r);
        let row = &mut rest[0];
        let fr = first[r];
        for c in fr..r {
            let fc = first[c];
            let lc = &done[c];
            let mut s = row[c - fr];
            for k in fr.max(fc)..c {
                s -= row[k - fr] * lc[k - fc];
            }
            row[c - fr] = s / lc[c - fc];
        }
        let mut d = row[r - fr];
        for k in fr..r {
            d -= row[k - fr] * row[k - fr];
        }
        if d <= 0.0 || !d.is_finite() {
            return None;
        }
        row[r - fr] = d.sqrt();
    }
    // L z = P b, then Lᵀ y = z.
    let mut z: Vec<f64> = order.iter().map(|&o| b[o]).collect();
    for r in 0..m {
        let fr = first[r];
        let mut s = z[r];
        for k in fr..r {
            s -= rows[r][k - fr] * z[k];
        }
        z[r] = s / rows[r][r - fr];
    }
    let mut y = vec![0.0; m];
    for r in (0..m).rev() {
        let fr = first[r];
        y[r] = z[r] / rows[r][r - fr];
        for k in fr..r {
            z[k] -= rows[r][k - fr] * y[r];
        }
    }
    let mut out = vec![0.0; m];
    for (k, &o) in order.iter().enumerate() {
        out[o] = y[k];
    }
    Some(out)
}

/// An orthonormal basis of rows, made one row at a time (modified
/// Gram-Schmidt, two passes).
pub(crate) struct RowBasis {
    rows: Vec<Vec<f64>>,
}

/// A row that keeps less than this part of its length after the
/// projection adds nothing to the basis.
const DEPENDENT: f64 = 1e-7;

impl RowBasis {
    pub fn new() -> RowBasis {
        RowBasis { rows: Vec::new() }
    }

    pub fn rank(&self) -> usize {
        self.rows.len()
    }

    /// Adds `row` if it is not in the span of the basis. Returns `true` if
    /// it was added.
    pub fn add(&mut self, mut row: Vec<f64>) -> bool {
        let norm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm < 1e-12 {
            return false;
        }
        for v in &mut row {
            *v /= norm;
        }
        for _ in 0..2 {
            for b in &self.rows {
                let d: f64 = row.iter().zip(b).map(|(a, b)| a * b).sum();
                for (v, bv) in row.iter_mut().zip(b) {
                    *v -= d * bv;
                }
            }
        }
        let rest = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        if rest < DEPENDENT {
            return false;
        }
        for v in &mut row {
            *v /= rest;
        }
        self.rows.push(row);
        true
    }

    /// The part of unit vector `e_j` in the span of the basis (1: all of
    /// it).
    pub fn in_span(&self, j: usize) -> f64 {
        self.rows.iter().map(|r| r[j] * r[j]).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sparse_cholesky_solves() {
        // A = [[4, 1, 0, 0], [1, 3, 0, 1], [0, 0, 2, 0], [0, 1, 0, 5]].
        let full = [
            [4.0, 1.0, 0.0, 0.0],
            [1.0, 3.0, 0.0, 1.0],
            [0.0, 0.0, 2.0, 0.0],
            [0.0, 1.0, 0.0, 5.0],
        ];
        let mut lower = BTreeMap::new();
        for (i, row) in full.iter().enumerate() {
            for (j, v) in row.iter().enumerate().take(i + 1) {
                if *v != 0.0 {
                    lower.insert((i, j), *v);
                }
            }
        }
        let b = [1.0, 2.0, 3.0, 4.0];
        let y = sparse_cholesky_solve(4, &lower, &b).unwrap();
        for (i, row) in full.iter().enumerate() {
            let back: f64 = row.iter().zip(&y).map(|(a, y)| a * y).sum();
            assert!((back - b[i]).abs() < 1e-12, "row {i}");
        }
        let mut bad = BTreeMap::new();
        bad.insert((0, 0), -1.0);
        assert!(sparse_cholesky_solve(1, &bad, &[1.0]).is_none());
    }

    #[test]
    fn the_order_keeps_a_chain_narrow() {
        // A chain 0-5-1-4-2-3 in a mixed numbering.
        let edges = [(0, 5), (5, 1), (1, 4), (4, 2), (2, 3)];
        let mut adj = vec![Vec::new(); 6];
        for (a, b) in edges {
            adj[a].push(b);
            adj[b].push(a);
        }
        let order = rcm(&adj);
        let mut pos = [0usize; 6];
        for (k, o) in order.iter().enumerate() {
            pos[*o] = k;
        }
        for (a, b) in edges {
            assert_eq!(pos[a].abs_diff(pos[b]), 1, "neighbors are next to each other");
        }
    }

    #[test]
    fn the_basis_finds_dependent_rows() {
        let mut b = RowBasis::new();
        assert!(b.add(vec![1.0, 0.0, 0.0]));
        assert!(b.add(vec![1.0, 1.0, 0.0]));
        assert!(!b.add(vec![2.0, 3.0, 0.0]), "in the span");
        assert_eq!(b.rank(), 2);
        assert!((b.in_span(0) - 1.0).abs() < 1e-12);
        assert!(b.in_span(2) < 1e-12);
    }
}
