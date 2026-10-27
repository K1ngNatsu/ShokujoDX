//! Sewing a network of lines in one piece: redwork and branching.
//!
//! Not a port. The method is a depth-first walk of a graph, which is
//! common knowledge:
//!
//! 1. Lines are cut where they cross each other and where an end of one line
//!    touches another line ([`TOUCH_MM`]). Ends nearer than that are one node.
//! 2. Redwork: from the node nearest to the needle, each line is sewn out and
//!    later back, depth first. Each line is sewn two times, and the needle
//!    ends where it started.
//! 3. Branching: the same walk, but the path to the node farthest from the
//!    start (along the lines) is sewn last and only one time. So the
//!    longest branch is not sewn back.
//! 4. Lines that do not touch are separate needle paths; the stitch plan
//!    connects them with a travel stitch or a trim.

use emb_geom::{Point, Polyline};

use crate::running;

/// Ends and crossings nearer than this are connected, in mm.
pub const TOUCH_MM: f64 = 0.5;

/// A piece of a line between two nodes.
#[derive(Debug, Clone)]
struct Edge {
    a: usize,
    b: usize,
    pts: Vec<Point>,
    len: f64,
}

/// Where segment `p`-`q` crosses segment `a`-`b`, as the parameter on `p`-`q`.
fn cross_t(p: Point, q: Point, a: Point, b: Point) -> Option<(f64, f64)> {
    let r = q - p;
    let s = b - a;
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (a - p).cross(s) / den;
    let u = (a - p).cross(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some((t, u))
}

/// Lines cut at crossings and touch points.
fn split_lines(lines: &[Vec<Point>]) -> Vec<Vec<Point>> {
    // For each line: cut parameters as (segment index, t).
    let mut cuts: Vec<Vec<(usize, f64)>> = vec![Vec::new(); lines.len()];
    for i in 0..lines.len() {
        for j in 0..lines.len() {
            if i == j {
                continue;
            }
            // Crossings (each pair once, both lines get the cut).
            if i < j {
                for (si, w) in lines[i].windows(2).enumerate() {
                    for (sj, v) in lines[j].windows(2).enumerate() {
                        if let Some((t, u)) = cross_t(w[0], w[1], v[0], v[1]) {
                            cuts[i].push((si, t));
                            cuts[j].push((sj, u));
                        }
                    }
                }
            }
            // An end of line j that touches line i.
            for e in [lines[j][0], lines[j][lines[j].len() - 1]] {
                for (si, w) in lines[i].windows(2).enumerate() {
                    let d = w[1] - w[0];
                    let l2 = d.length_sq();
                    let t = if l2 > 0.0 {
                        ((e - w[0]).dot(d) / l2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    if w[0].lerp(w[1], t).distance(e) < TOUCH_MM {
                        cuts[i].push((si, t));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (line, mut c) in lines.iter().zip(cuts) {
        c.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
        let mut piece = vec![line[0]];
        let mut ci = 0;
        for s in 0..line.len() - 1 {
            while ci < c.len() && c[ci].0 == s {
                let p = line[s].lerp(line[s + 1], c[ci].1);
                ci += 1;
                // A cut at the start or the end of the line is not a cut.
                if piece.last().is_some_and(|q| q.distance(p) < 1e-6) || p.distance(line[line.len() - 1]) < 1e-6 {
                    continue;
                }
                piece.push(p);
                out.push(std::mem::replace(&mut piece, vec![p]));
            }
            if piece.last() != Some(&line[s + 1]) {
                piece.push(line[s + 1]);
            }
        }
        if piece.len() >= 2 {
            out.push(piece);
        }
    }
    out.retain(|p| p.len() >= 2 && p.windows(2).any(|w| w[0].distance(w[1]) > 1e-9));
    out
}

/// Nodes (clustered ends) and edges.
fn graph(pieces: Vec<Vec<Point>>) -> (Vec<Point>, Vec<Edge>) {
    let mut nodes: Vec<Point> = Vec::new();
    let node = |p: Point, nodes: &mut Vec<Point>| -> usize {
        if let Some(i) = nodes.iter().position(|q| q.distance(p) < TOUCH_MM) {
            i
        } else {
            nodes.push(p);
            nodes.len() - 1
        }
    };
    let mut edges = Vec::new();
    for pts in pieces {
        let a = node(pts[0], &mut nodes);
        let b = node(pts[pts.len() - 1], &mut nodes);
        let len = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        edges.push(Edge { a, b, pts, len });
    }
    (nodes, edges)
}

/// Sews edge `e` from node `from` to the other node.
fn sew_edge(e: &Edge, from: usize, stitch_len: f64, out: &mut Vec<Point>) {
    let mut pts = e.pts.clone();
    if e.a != from {
        pts.reverse();
    }
    let line = Polyline {
        points: pts,
        closed: false,
    };
    let sewn = running::running_points(&line, stitch_len);
    for p in sewn {
        if out.last().is_none_or(|q| q.distance(p) > 1e-9) {
            out.push(p);
        }
    }
}

/// Needle paths for a network of lines (see the module comment).
/// `branch` makes a branching walk, else a redwork walk.
pub fn network_runs(lines: &[Polyline], entry: Point, stitch_len: f64, branch: bool) -> Vec<Vec<Point>> {
    let lines: Vec<Vec<Point>> = lines
        .iter()
        .map(|l| {
            let mut p = l.points.clone();
            if l.closed && p.len() > 2 {
                p.push(p[0]);
            }
            p
        })
        .filter(|p| p.len() >= 2)
        .collect();
    let (nodes, edges) = graph(split_lines(&lines));
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (k, e) in edges.iter().enumerate() {
        adj[e.a].push(k);
        if e.b != e.a {
            adj[e.b].push(k);
        }
    }
    let other = |k: usize, n: usize| if edges[k].a == n { edges[k].b } else { edges[k].a };
    let mut edge_done = vec![false; edges.len()];
    let mut node_seen = vec![false; nodes.len()];
    let mut runs = Vec::new();
    let mut needle = entry;
    while let Some(root) = (0..nodes.len())
        .filter(|&n| !node_seen[n] && !adj[n].is_empty())
        .min_by(|&i, &j| nodes[i].distance(needle).total_cmp(&nodes[j].distance(needle)))
    {
        // Depth-first tree of this component, with path lengths from the root.
        let mut parent: Vec<Option<usize>> = vec![None; nodes.len()];
        let mut dist = vec![f64::INFINITY; nodes.len()];
        let mut order = vec![root];
        dist[root] = 0.0;
        node_seen[root] = true;
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            for &k in &adj[n] {
                let m = other(k, n);
                if !node_seen[m] {
                    node_seen[m] = true;
                    parent[m] = Some(k);
                    dist[m] = dist[n] + edges[k].len;
                    order.push(m);
                    stack.push(m);
                }
            }
        }
        // Branching: the tree edges to the farthest node are sewn last, one time.
        let mut last_path = vec![false; edges.len()];
        if branch {
            let far = order
                .iter()
                .copied()
                .max_by(|&i, &j| dist[i].total_cmp(&dist[j]))
                .unwrap_or(root);
            let mut n = far;
            while let Some(k) = parent[n] {
                last_path[k] = true;
                n = other(k, n);
            }
        }
        let mut out = vec![nodes[root]];
        walk(
            root,
            &edges,
            &adj,
            &parent,
            &last_path,
            &mut edge_done,
            stitch_len,
            &mut out,
        );
        needle = *out.last().unwrap_or(&needle);
        if out.len() >= 2 {
            runs.push(out);
        }
    }
    runs
}

/// Sews all edges below node `n`. Edges on the last path are sewn last and
/// not back; all others go out and back.
#[allow(clippy::too_many_arguments)]
fn walk(
    n: usize,
    edges: &[Edge],
    adj: &[Vec<usize>],
    parent: &[Option<usize>],
    last_path: &[bool],
    done: &mut [bool],
    stitch_len: f64,
    out: &mut Vec<Point>,
) {
    let other = |k: usize| if edges[k].a == n { edges[k].b } else { edges[k].a };
    let mut ks: Vec<usize> = adj[n].iter().copied().filter(|&k| !done[k]).collect();
    // The last-path edge goes last.
    ks.sort_by_key(|&k| last_path[k]);
    for k in ks {
        if done[k] {
            continue;
        }
        done[k] = true;
        let m = other(k);
        sew_edge(&edges[k], n, stitch_len, out);
        if parent[m] == Some(k) {
            walk(m, edges, adj, parent, last_path, done, stitch_len, out);
        }
        if !last_path[k] {
            sew_edge(&edges[k], m, stitch_len, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(pts: &[(f64, f64)]) -> Polyline {
        Polyline {
            points: pts.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            closed: false,
        }
    }

    /// A plus sign: two lines that cross in the middle.
    fn plus() -> Vec<Polyline> {
        vec![line(&[(-10.0, 0.0), (10.0, 0.0)]), line(&[(0.0, -10.0), (0.0, 10.0)])]
    }

    fn length(pts: &[Point]) -> f64 {
        pts.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    #[test]
    fn redwork_sews_each_line_twice_and_returns() {
        let runs = network_runs(&plus(), Point::new(-12.0, 0.0), 2.5, false);
        assert_eq!(runs.len(), 1, "one needle path");
        let r = &runs[0];
        assert_eq!(r[0], Point::new(-10.0, 0.0));
        assert_eq!(*r.last().unwrap(), Point::new(-10.0, 0.0), "ends where it starts");
        assert!((length(r) - 80.0).abs() < 1e-6, "40 mm of lines, each sewn two times");
        assert!(r.windows(2).all(|w| w[0].distance(w[1]) <= 2.5 + 1e-9));
    }

    #[test]
    fn branching_does_not_return_on_the_longest_branch() {
        let runs = network_runs(&plus(), Point::new(-12.0, 0.0), 2.5, true);
        let r = &runs[0];
        // Start arm (10) + one arm out and back (20) + one arm out and back (20) + last arm (10).
        assert!((length(r) - 60.0).abs() < 1e-6, "{}", length(r));
        assert!(r.last().unwrap().distance(Point::new(-10.0, 0.0)) > 9.0);
    }

    #[test]
    fn a_touching_end_connects_and_separate_lines_are_separate_runs() {
        // A T: the stem ends on the bar. And a separate line far away.
        let lines = vec![
            line(&[(-10.0, 0.0), (10.0, 0.0)]),
            line(&[(0.0, 0.2), (0.0, 10.0)]),
            line(&[(50.0, 0.0), (60.0, 0.0)]),
        ];
        let runs = network_runs(&lines, Point::ZERO, 3.0, false);
        assert_eq!(runs.len(), 2);
        assert!((length(&runs[0]) - 2.0 * 29.8).abs() < 0.5, "{}", length(&runs[0]));
    }
}
