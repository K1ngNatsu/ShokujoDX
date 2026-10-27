//! Minimum cut of a graph with a source and a sink (for `cutout`).
//!
//! The method of Boykov and Kolmogorov ("An Experimental Comparison of
//! Min-Cut/Max-Flow Algorithms for Energy Minimization in Vision", IEEE
//! PAMI 2004): two search trees grow from the source and from the sink;
//! when they touch, flow goes along the path; nodes cut off from their tree
//! look for a new parent. Our own implementation of the published method.

use std::collections::VecDeque;

const NONE: u32 = u32::MAX;
const TERMINAL: u32 = u32::MAX - 1;
const ORPHAN: u32 = u32::MAX - 2;

/// A graph of nodes `0..n` with arcs between them and to the terminals.
pub struct Graph {
    first: Vec<u32>,
    head: Vec<u32>,
    next: Vec<u32>,
    cap: Vec<f64>,
    /// The residual terminal capacity: > 0 from the source, < 0 to the sink.
    tr: Vec<f64>,
    flow: f64,
    parent: Vec<u32>,
    sink: Vec<bool>,
    ts: Vec<u32>,
    dist: Vec<u32>,
    active: VecDeque<u32>,
    in_active: Vec<bool>,
    orphans: VecDeque<u32>,
    time: u32,
}

impl Graph {
    pub fn new(n: usize) -> Graph {
        Graph {
            first: vec![NONE; n],
            head: Vec::new(),
            next: Vec::new(),
            cap: Vec::new(),
            tr: vec![0.0; n],
            flow: 0.0,
            parent: vec![NONE; n],
            sink: vec![false; n],
            ts: vec![0; n],
            dist: vec![0; n],
            active: VecDeque::new(),
            in_active: vec![false; n],
            orphans: VecDeque::new(),
            time: 0,
        }
    }

    fn arc(&mut self, from: u32, to: u32, cap: f64) {
        let a = self.head.len() as u32;
        self.head.push(to);
        self.cap.push(cap);
        self.next.push(self.first[from as usize]);
        self.first[from as usize] = a;
    }

    /// An edge with capacity `ij` from `i` to `j` and `ji` back.
    pub fn add_edge(&mut self, i: usize, j: usize, ij: f64, ji: f64) {
        self.arc(i as u32, j as u32, ij);
        self.arc(j as u32, i as u32, ji);
    }

    /// Capacities from the source to `i` and from `i` to the sink.
    pub fn add_terminal(&mut self, i: usize, source: f64, sink: f64) {
        let d = self.tr[i];
        let (s, t) = if d > 0.0 {
            (source + d, sink)
        } else {
            (source, sink - d)
        };
        self.flow += s.min(t);
        self.tr[i] = s - t;
    }

    fn arcs(&self, i: u32) -> impl Iterator<Item = u32> + '_ {
        let mut a = self.first[i as usize];
        std::iter::from_fn(move || {
            (a != NONE).then(|| {
                let cur = a;
                a = self.next[cur as usize];
                cur
            })
        })
    }

    fn activate(&mut self, i: u32) {
        if !self.in_active[i as usize] {
            self.in_active[i as usize] = true;
            self.active.push_back(i);
        }
    }

    /// The tail of arc `a`.
    fn tail(&self, a: u32) -> u32 {
        self.head[(a ^ 1) as usize]
    }

    /// Computes the maximum flow; returns it.
    pub fn maxflow(&mut self) -> f64 {
        let n = self.tr.len();
        for i in 0..n {
            if self.tr[i] != 0.0 {
                self.parent[i] = TERMINAL;
                self.sink[i] = self.tr[i] < 0.0;
                self.dist[i] = 1;
                self.activate(i as u32);
            }
        }
        while let Some(i) = self.active.pop_front() {
            self.in_active[i as usize] = false;
            if self.parent[i as usize] == NONE {
                continue;
            }
            // Grow the tree of `i`; stop at the first arc to the other tree.
            let mut found = NONE;
            let arcs: Vec<u32> = self.arcs(i).collect();
            let is_sink = self.sink[i as usize];
            for a in arcs {
                let residual = if is_sink {
                    self.cap[(a ^ 1) as usize]
                } else {
                    self.cap[a as usize]
                };
                if residual <= 0.0 {
                    continue;
                }
                let j = self.head[a as usize];
                let ju = j as usize;
                if self.parent[ju] == NONE {
                    self.sink[ju] = is_sink;
                    self.parent[ju] = a ^ 1;
                    self.ts[ju] = self.ts[i as usize];
                    self.dist[ju] = self.dist[i as usize] + 1;
                    self.activate(j);
                } else if self.sink[ju] != is_sink {
                    found = if is_sink { a ^ 1 } else { a };
                    break;
                } else if self.ts[ju] <= self.ts[i as usize] && self.dist[ju] > self.dist[i as usize] {
                    // A shorter path to the root for `j`.
                    self.parent[ju] = a ^ 1;
                    self.ts[ju] = self.ts[i as usize];
                    self.dist[ju] = self.dist[i as usize] + 1;
                }
            }
            self.time += 1;
            if found != NONE {
                // `i` can grow more after the augmentation.
                self.in_active[i as usize] = true;
                self.active.push_front(i);
                self.augment(found);
                while let Some(o) = self.orphans.pop_front() {
                    self.adopt(o);
                }
            }
        }
        self.flow
    }

    /// Sends the most flow along the path through arc `mid` (from the
    /// source tree to the sink tree).
    fn augment(&mut self, mid: u32) {
        let mut b = self.cap[mid as usize];
        let mut i = self.tail(mid);
        while self.parent[i as usize] != TERMINAL {
            let a = self.parent[i as usize] ^ 1;
            b = b.min(self.cap[a as usize]);
            i = self.head[self.parent[i as usize] as usize];
        }
        b = b.min(self.tr[i as usize]);
        let mut i = self.head[mid as usize];
        while self.parent[i as usize] != TERMINAL {
            let a = self.parent[i as usize];
            b = b.min(self.cap[a as usize]);
            i = self.head[a as usize];
        }
        b = b.min(-self.tr[i as usize]);

        self.cap[mid as usize] -= b;
        self.cap[(mid ^ 1) as usize] += b;
        let mut i = self.tail(mid);
        while self.parent[i as usize] != TERMINAL {
            let p = self.parent[i as usize];
            let a = p ^ 1;
            self.cap[a as usize] -= b;
            self.cap[p as usize] += b;
            let up = self.head[p as usize];
            if self.cap[a as usize] <= 0.0 {
                self.parent[i as usize] = ORPHAN;
                self.orphans.push_back(i);
            }
            i = up;
        }
        self.tr[i as usize] -= b;
        if self.tr[i as usize] <= 0.0 {
            self.parent[i as usize] = ORPHAN;
            self.orphans.push_back(i);
        }
        let mut i = self.head[mid as usize];
        while self.parent[i as usize] != TERMINAL {
            let a = self.parent[i as usize];
            self.cap[a as usize] -= b;
            self.cap[(a ^ 1) as usize] += b;
            let up = self.head[a as usize];
            if self.cap[a as usize] <= 0.0 {
                self.parent[i as usize] = ORPHAN;
                self.orphans.push_back(i);
            }
            i = up;
        }
        self.tr[i as usize] += b;
        if self.tr[i as usize] >= 0.0 {
            self.parent[i as usize] = ORPHAN;
            self.orphans.push_back(i);
        }
        self.flow += b;
    }

    /// The distance from `j` to its terminal, if its path to the root has
    /// no orphan; marks the path with the time.
    fn origin(&mut self, j: u32) -> Option<u32> {
        let mut d = 0u32;
        let mut k = j;
        loop {
            let ku = k as usize;
            if self.ts[ku] == self.time {
                d += self.dist[ku];
                break;
            }
            let p = self.parent[ku];
            d += 1;
            if p == TERMINAL {
                self.ts[ku] = self.time;
                self.dist[ku] = 1;
                break;
            }
            if p == ORPHAN || p == NONE {
                return None;
            }
            k = self.head[p as usize];
        }
        let found = d;
        let mut k = j;
        while self.ts[k as usize] != self.time {
            self.ts[k as usize] = self.time;
            self.dist[k as usize] = d;
            d -= 1;
            k = self.head[self.parent[k as usize] as usize];
        }
        Some(found)
    }

    /// Finds a new parent for the orphan `i` in its tree, or frees it.
    fn adopt(&mut self, i: u32) {
        let iu = i as usize;
        let is_sink = self.sink[iu];
        let arcs: Vec<u32> = self.arcs(i).collect();
        let mut best = (NONE, u32::MAX);
        for &a in &arcs {
            // The arc from the parent to `i` (source tree) or from `i` to the parent (sink tree) has room.
            let residual = if is_sink {
                self.cap[a as usize]
            } else {
                self.cap[(a ^ 1) as usize]
            };
            if residual <= 0.0 {
                continue;
            }
            let j = self.head[a as usize];
            let ju = j as usize;
            if self.parent[ju] == NONE || self.sink[ju] != is_sink {
                continue;
            }
            if let Some(d) = self.origin(j)
                && d < best.1
            {
                best = (a, d);
            }
        }
        if best.0 != NONE {
            self.parent[iu] = best.0;
            self.ts[iu] = self.time;
            self.dist[iu] = best.1 + 1;
            return;
        }
        // No parent: `i` is free; its children are orphans, and the
        // neighbors in its tree can grow into it again.
        for &a in &arcs {
            let j = self.head[a as usize];
            let ju = j as usize;
            if self.parent[ju] == NONE || self.sink[ju] != is_sink {
                continue;
            }
            let residual = if is_sink {
                self.cap[a as usize]
            } else {
                self.cap[(a ^ 1) as usize]
            };
            if residual > 0.0 {
                self.activate(j);
            }
            let p = self.parent[ju];
            if p != TERMINAL && p != ORPHAN && self.head[p as usize] == i {
                self.parent[ju] = ORPHAN;
                self.orphans.push_back(j);
            }
        }
        self.parent[iu] = NONE;
    }

    /// After [`Graph::maxflow`]: `true` if node `i` is on the source side
    /// of the minimum cut.
    pub fn on_source_side(&self, i: usize) -> bool {
        self.parent[i] != NONE && !self.sink[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_graph_has_its_known_flow_and_cut() {
        // s -> 0 (3), s -> 1 (2), 0 -> 1 (1), 0 -> t (2), 1 -> t (3).
        let mut g = Graph::new(2);
        g.add_terminal(0, 3.0, 2.0);
        g.add_terminal(1, 2.0, 3.0);
        g.add_edge(0, 1, 1.0, 0.0);
        assert!((g.maxflow() - 5.0).abs() < 1e-9);
        // A chain s -> 0 -> 1 -> 2 -> t with a weak middle edge.
        let mut g = Graph::new(3);
        g.add_terminal(0, 10.0, 0.0);
        g.add_terminal(2, 0.0, 10.0);
        g.add_edge(0, 1, 5.0, 5.0);
        g.add_edge(1, 2, 1.0, 1.0);
        assert!((g.maxflow() - 1.0).abs() < 1e-9);
        assert!(g.on_source_side(0) && g.on_source_side(1) && !g.on_source_side(2));
    }

    #[test]
    fn a_grid_is_cut_along_its_weak_column() {
        // 10 x 10 grid; the left column to the source, the right to the
        // sink; edges are strong except across column 6 | 7.
        let (w, h) = (10, 10);
        let mut g = Graph::new(w * h);
        for y in 0..h {
            g.add_terminal(y * w, 100.0, 0.0);
            g.add_terminal(y * w + w - 1, 0.0, 100.0);
            for x in 0..w {
                let i = y * w + x;
                if x + 1 < w {
                    let c = if x == 6 { 1.0 } else { 10.0 };
                    g.add_edge(i, i + 1, c, c);
                }
                if y + 1 < h {
                    g.add_edge(i, i + w, 10.0, 10.0);
                }
            }
        }
        assert!((g.maxflow() - 10.0).abs() < 1e-9);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(g.on_source_side(y * w + x), x <= 6, "{x} {y}");
            }
        }
    }
}
