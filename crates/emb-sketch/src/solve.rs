//! The constraint solver and the analysis of a sketch (ADR 0068).
//!
//! The equations are split into groups that share no variables. Each
//! group is solved with damped Newton steps of the smallest weighted size:
//! `Δ = −W⁻¹Jᵀ(JW⁻¹Jᵀ + λI)⁻¹F` (a sparse Cholesky solution), with a short
//! line search; λ goes up when a step does not make the error smaller. The smallest step keeps
//! a sketch with free motions near to where it is; a large weight keeps a
//! dragged point near to the pointer.

use std::collections::BTreeMap;

use emb_geom::Point;

use crate::equations::{Equation, Layout, Var, equations};
use crate::linalg::{RowBasis, sparse_cholesky_solve};
use crate::{ConstraintId, CurveId, PointId, Sketch};

/// An equation is true when it is smaller than this, in mm.
pub const TOLERANCE: f64 = 1e-10;
/// The most Newton steps for each group. A group that solves stops early,
/// and so does one that cannot come nearer (a conflict). A shape near to a
/// degenerate one (a triangle with angles of 0.3°) needs about 160 steps.
const MAX_STEPS: usize = 500;
/// The weight of the variables of a dragged point.
const DRAG_WEIGHT: f64 = 1e4;
/// A constraint whose equation is larger than this after the solution is
/// in conflict.
const CONFLICT: f64 = 1e-6;

/// The result of a solution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveReport {
    /// True if all the equations are true.
    pub converged: bool,
    /// The largest equation after the solution, in mm (or radians for an
    /// angle, or the sine of an angle for parallel and perpendicular).
    pub max_error: f64,
    /// The most Newton steps of a group.
    pub steps: usize,
}

/// The free motions of a sketch and its bad constraints.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Analysis {
    /// The free motions (degrees of freedom): the variables less the rank
    /// of the equations. 0: fully constrained.
    pub free: usize,
    /// The points that cannot move.
    pub fixed_points: Vec<PointId>,
    /// The curves whose points and radius cannot move.
    pub fixed_curves: Vec<CurveId>,
    /// Constraints that repeat others (the newer one).
    pub redundant: Vec<ConstraintId>,
    /// Constraints that conflict with others (the newer one).
    pub conflicting: Vec<ConstraintId>,
}

impl Analysis {
    pub fn is_fully_constrained(&self) -> bool {
        self.free == 0 && self.conflicting.is_empty()
    }
}

/// The groups of equations that share variables: the equations and the
/// variables of each.
fn groups(eqs: &[Equation], n: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for e in eqs {
        for w in e.vars.windows(2) {
            let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut by_root: BTreeMap<usize, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
    for (k, e) in eqs.iter().enumerate() {
        if let Some(&v) = e.vars.first() {
            let r = find(&mut parent, v);
            by_root.entry(r).or_default().0.push(k);
        }
    }
    for v in 0..n {
        let r = find(&mut parent, v);
        if let Some(g) = by_root.get_mut(&r) {
            g.1.push(v);
        }
    }
    by_root.into_values().collect()
}

/// The derivatives of equation `e` at `x`, by central differences:
/// (variable, derivative).
fn gradient(e: &Equation, x: &mut [f64]) -> Vec<(usize, f64)> {
    e.vars
        .iter()
        .map(|&v| {
            let x0 = x[v];
            let h = 1e-7 * (1.0 + x0.abs());
            x[v] = x0 + h;
            let up = (e.f)(x);
            x[v] = x0 - h;
            let down = (e.f)(x);
            x[v] = x0;
            (v, (up - down) / (2.0 * h))
        })
        .collect()
}

fn max_abs(f: &[f64]) -> f64 {
    f.iter().fold(0.0, |m, v| m.max(v.abs()))
}

fn sum_sq(f: &[f64]) -> f64 {
    f.iter().map(|v| v * v).sum()
}

/// Solves one group. `weight(v)` is the weight of variable `v`. Returns
/// (converged, max error, steps).
fn solve_group(eqs: &[&Equation], vars: &[usize], x: &mut [f64], weight: &dyn Fn(usize) -> f64) -> (bool, f64, usize) {
    let m = eqs.len();
    let local: BTreeMap<usize, usize> = vars.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    let inv_w: Vec<f64> = vars.iter().map(|v| 1.0 / weight(*v)).collect();
    let eval = |x: &[f64]| -> Vec<f64> { eqs.iter().map(|e| (e.f)(x)).collect() };
    let mut f = eval(x);
    let mut lambda = 1e-12;
    for step in 0..MAX_STEPS {
        let err = max_abs(&f);
        if err < TOLERANCE {
            return (true, err, step);
        }
        // The sparse Jacobian, with local variable numbers.
        let jac: Vec<Vec<(usize, f64)>> = eqs
            .iter()
            .map(|e| gradient(e, x).into_iter().map(|(v, d)| (local[&v], d)).collect())
            .collect();
        let before = sum_sq(&f);
        let mut improved = false;
        // The equations of each variable.
        let mut by_var: Vec<Vec<usize>> = vec![Vec::new(); vars.len()];
        for (i, row) in jac.iter().enumerate() {
            for &(c, _) in row {
                by_var[c].push(i);
            }
        }
        for _ in 0..16 {
            // A = J W⁻¹ Jᵀ + λI (on and under the diagonal), from the
            // equations of each variable.
            let mut a: BTreeMap<(usize, usize), f64> = BTreeMap::new();
            for (c, rows) in by_var.iter().enumerate() {
                let rows: Vec<(usize, f64)> = rows
                    .iter()
                    .filter_map(|&i| jac[i].iter().find(|(v, _)| *v == c).map(|(_, d)| (i, *d)))
                    .collect();
                for &(i, di) in &rows {
                    for &(j, dj) in &rows {
                        if i >= j {
                            *a.entry((i, j)).or_insert(0.0) += di * dj * inv_w[c];
                        }
                    }
                }
            }
            let scale = (0..m)
                .map(|i| a.get(&(i, i)).copied().unwrap_or(0.0))
                .fold(0.0, f64::max)
                .max(1e-12);
            for i in 0..m {
                *a.entry((i, i)).or_insert(0.0) += lambda * scale;
            }
            let Some(y) = sparse_cholesky_solve(m, &a, &f) else {
                lambda = (lambda * 100.0).max(1e-9);
                continue;
            };
            // Δ = −W⁻¹Jᵀy.
            let mut delta = vec![0.0; vars.len()];
            for (row, yi) in jac.iter().zip(&y) {
                for &(c, d) in row {
                    delta[c] -= d * yi * inv_w[c];
                }
            }
            // Line search.
            let x0: Vec<f64> = vars.iter().map(|v| x[*v]).collect();
            let mut t = 1.0;
            for _ in 0..3 {
                for (k, v) in vars.iter().enumerate() {
                    x[*v] = x0[k] + t * delta[k];
                }
                let trial = eval(x);
                if sum_sq(&trial) < before && trial.iter().all(|v| v.is_finite()) {
                    f = trial;
                    improved = true;
                    break;
                }
                t *= 0.5;
            }
            if improved {
                lambda = (lambda / 3.0).max(1e-12);
                break;
            }
            for (k, v) in vars.iter().enumerate() {
                x[*v] = x0[k];
            }
            lambda = (lambda * 10.0).max(1e-6);
        }
        if !improved {
            let err = max_abs(&f);
            return (err < TOLERANCE, err, step);
        }
    }
    let err = max_abs(&f);
    (err < TOLERANCE, err, MAX_STEPS)
}

impl Sketch {
    /// Moves the points and radii so that the constraints are true, with
    /// the smallest change.
    pub fn solve(&mut self) -> SolveReport {
        self.solve_weighted(&[])
    }

    /// Moves point `point` to `to` and solves, with the point as near to
    /// `to` as the constraints let it: first with the point held at `to`;
    /// if the constraints do not let it be there, with a large weight on
    /// the point.
    pub fn drag(&mut self, point: PointId, to: Point) -> SolveReport {
        self.drag_points(&[(point, to)])
    }

    /// As [`Sketch::drag`] for more points at a time (for example the two
    /// ends of a line that moves).
    pub fn drag_points(&mut self, moves: &[(PointId, Point)]) -> SolveReport {
        let moves: Vec<(PointId, Point)> = moves
            .iter()
            .copied()
            .filter(|(p, to)| to.is_finite() && self.point(*p).is_some())
            .collect();
        if moves.is_empty() {
            return self.solve();
        }
        let mut held = self.clone();
        let n = held.constraints.len();
        for &(point, to) in &moves {
            held.set_point(point, to);
            let _ = held.add_constraint(crate::Constraint::Fix { point, at: to });
        }
        let r = held.solve();
        if r.converged {
            held.constraints.truncate(n);
            held.next_id = self.next_id;
            *self = held;
            return r;
        }
        for &(point, to) in &moves {
            self.set_point(point, to);
        }
        let heavy: Vec<PointId> = moves.iter().map(|(p, _)| *p).collect();
        self.solve_weighted(&heavy)
    }

    fn solve_weighted(&mut self, heavy: &[PointId]) -> SolveReport {
        let layout = Layout::of(self);
        let eqs = equations(self, &layout);
        let mut x = layout.x.clone();
        let heavy_vars: Vec<usize> = heavy
            .iter()
            .filter_map(|p| layout.point(*p))
            .flat_map(|i| [i, i + 1])
            .collect();
        let weight = |v: usize| if heavy_vars.contains(&v) { DRAG_WEIGHT } else { 1.0 };
        let mut report = SolveReport {
            converged: true,
            max_error: 0.0,
            steps: 0,
        };
        for (eq_ids, vars) in groups(&eqs, x.len()) {
            let group: Vec<&Equation> = eq_ids.iter().map(|k| &eqs[*k]).collect();
            let (ok, err, steps) = solve_group(&group, &vars, &mut x, &weight);
            report.converged &= ok;
            report.max_error = report.max_error.max(err);
            report.steps = report.steps.max(steps);
        }
        if x.iter().all(|v| v.is_finite()) {
            layout.write_back(&x, self);
            self.update_driven();
        } else {
            report.converged = false;
        }
        report
    }

    /// The free motions, the fixed points and curves, and the constraints
    /// that repeat or conflict, at the sketch as it is now (solve first).
    pub fn analyze(&self) -> Analysis {
        let layout = Layout::of(self);
        let eqs = equations(self, &layout);
        let mut x = layout.x.clone();
        let mut out = Analysis::default();
        let mut fixed_vars = vec![false; x.len()];
        let mut rank = 0;
        for (eq_ids, vars) in groups(&eqs, x.len()) {
            let local: BTreeMap<usize, usize> = vars.iter().enumerate().map(|(i, v)| (*v, i)).collect();
            let mut basis = RowBasis::new();
            for k in eq_ids {
                let e = &eqs[k];
                let mut row = vec![0.0; vars.len()];
                for (v, d) in gradient(e, &mut x) {
                    row[local[&v]] = d;
                }
                if !basis.add(row)
                    && let Some(id) = e.owner
                {
                    let list = if (e.f)(&x).abs() > CONFLICT {
                        &mut out.conflicting
                    } else {
                        &mut out.redundant
                    };
                    if !list.contains(&id) {
                        list.push(id);
                    }
                }
            }
            rank += basis.rank();
            for (j, v) in vars.iter().enumerate() {
                fixed_vars[*v] = basis.in_span(j) > 1.0 - 1e-6;
            }
        }
        out.free = x.len() - rank;
        // A constraint can be in both lists if its two equations differ;
        // a conflict counts first.
        let conflicting = out.conflicting.clone();
        out.redundant.retain(|id| !conflicting.contains(id));
        let fixed = |var: Var| {
            layout
                .vars
                .iter()
                .position(|v| *v == var)
                .is_some_and(|i| fixed_vars[i])
        };
        out.fixed_points = self
            .points
            .iter()
            .filter(|p| fixed(Var::X(p.id)) && fixed(Var::Y(p.id)))
            .map(|p| p.id)
            .collect();
        out.fixed_curves = self
            .curves
            .iter()
            .filter(|c| {
                c.points().iter().all(|p| out.fixed_points.contains(p)) && (c.is_line() || fixed(Var::Radius(c.id)))
            })
            .map(|c| c.id)
            .collect();
        out
    }
}
