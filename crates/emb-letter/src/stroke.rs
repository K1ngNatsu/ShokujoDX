//! Satin columns from the strokes of a glyph shape (our own method; see
//! ADR 0015 and `docs/design/p4-plan.md`).
//!
//! The outline of a Latin letter or digit, or of a kana or kanji, is first
//! split into strokes at its concave corners ([`Split`], `crate::split`,
//! P21 Milestone 8, ADR 0152), and the steps below run on each piece.
//!
//! 1. Fill a grid of about [`GRID_CELLS`] cells for the capital height, and
//!    find the distance of each cell to the edge.
//! 2. Thin the shape to its middle lines and make a graph: nodes at line ends
//!    and junctions, edges along the lines.
//! 3. Remove short side branches (from corners and serifs), join the nodes
//!    that are very near, and at each junction pair the two edges that
//!    continue most straight. Paired edges make one stroke.
//! 4. Smooth each stroke, cut it at sharp corners, and extend free ends to
//!    the edge of the shape. A stroke that ends where another stroke goes
//!    through (the stem of a "T") ends a little inside the edge of that
//!    stroke, not at its middle, so the thread does not build up at the
//!    join (P11 Milestone 2; the density map of ADR 0044 showed 6 to 9
//!    layers there).
//! 5. Where two strokes cross, the longer one goes through, and the other
//!    is cut into two strokes that end a little inside its edge (P20
//!    Milestone 3): one layer of thread at a crossing, not two.
//! 6. Each point of a stroke takes the nearest point of the outline on each
//!    side (its foot), and the rails of the satin column follow the outline
//!    from foot to foot (P21 Milestone 1, ADR 0147). Where a side passes
//!    the place where another stroke meets or crosses (an opening), the rail
//!    goes straight across it, so the column has the width of its stroke
//!    there and not the size of the junction.
//! 7. The end of a column that lies in other columns (a corner, a
//!    junction, an end that was extended through another stroke) is cut
//!    back to [`END_OVERLAP`] inside them.
//!
//! A part of the shape with almost no middle line (a dot) gets one stroke
//! along its long axis.

use std::collections::HashMap;

use emb_geom::region::ring_contains;
use emb_geom::{BBox, Path, Point};
use emb_model::SatinGeometry;

use crate::grid::{Grid, distance, fill_into, parts, thin};

/// Grid cells for the capital height.
pub const GRID_CELLS: f64 = 200.0;
/// A branch from a line end to a junction is removed when it is shorter
/// than this many times the stroke half-width at the junction.
const PRUNE: f64 = 1.6;
/// In a piece of a split shape, the branches of ends can be up to this
/// many times longer (see [`piece`]), if the columns lose at most
/// [`PIECE_COVER_LOSS`] of the coverage and the longest line across a
/// column is at most [`PIECE_WIDER`] times as long (a slanted end of "X":
/// 1.4; a serif on the two sides of a stem is not a branch: the stem went
/// across it, with stitches twice as long).
const PIECE_PRUNE: f64 = 1.8;
const PIECE_COVER_LOSS: f64 = 0.003;
const PIECE_WIDER: f64 = 1.5;
/// A piece of a stroke shorter than this many half-widths of another stroke
/// (not counting its extended ends) does not cut back an extended end of
/// that stroke: it is a side stroke (a flag serif).
const SIDE_PIECE: f64 = 2.0;
/// A branch is a side branch only if it stays inside this many
/// half-widths of the junction (counting its own half-width): a branch to
/// a corner does, a short thick arm (the bar of a condensed "T") does not.
const SPUR_REACH: f64 = 1.5;
/// Junctions nearer than this many half-widths are one junction.
const JOIN: f64 = 0.8;
/// Two edges at a junction continue as one stroke when the angle between
/// them is at least this (degrees), as a pen goes on through a crossing.
const STRAIGHT_DEG: f64 = 140.0;
/// The direction of an edge at a junction is the direction of its part
/// from this many half-widths of the junction to [`TANGENT_TO`] half-widths
/// (along the edge; a short edge: from a quarter of it to its end). At the
/// junction, the middle line of an edge bends to the middle of the
/// junction: the two halves of a serif on a stem leave it as a "V", not
/// as one line. Far from the junction, a curved stroke has another
/// direction (the hook of an "f").
const TANGENT_FROM: f64 = 0.5;
const TANGENT_TO: f64 = 2.0;
/// Two edges with different widths continue as one stroke less well: the
/// angle between them counts as this much less (degrees) for an edge with
/// no width, in proportion (a pen does not change its width in a crossing).
const WIDTH_DEG: f64 = 30.0;
/// A stroke is cut where it turns more than this (degrees) over about
/// three half-widths.
const CORNER_DEG: f64 = 70.0;
/// A stroke is also cut where it turns more than this (degrees) and the
/// outline has a corner on the outer side of the turn. The middle line of
/// a stroke with a square corner on its outside and a round one on its
/// inside turns about 70 degrees there, a little more or a little less:
/// with the first limit alone, a "D" was cut at its corners and a "B" of
/// the same font was not.
const CORNER_AT_OUTLINE_DEG: f64 = 50.0;
/// The outline has a corner where it turns more than this (degrees) over
/// [`OUTLINE_CORNER_SPAN`] of the capital height before and after a point.
const OUTLINE_CORNER_DEG: f64 = 55.0;
const OUTLINE_CORNER_SPAN: f64 = 0.03;
/// A corner of the outline is at a turn of a stroke if it is at most this
/// many half-widths from the middle line there, on the outer side of the
/// turn, and its two sides go along the stroke before and after the turn
/// (at most [`OUTLINE_CORNER_ALONG_DEG`] degrees off). The corners of the
/// end of a stroke have one side across the stroke: they are not at a turn.
const OUTLINE_CORNER_REACH: f64 = 2.1;
const OUTLINE_CORNER_ALONG_DEG: f64 = 30.0;

/// A corner of the outline: the point and the directions of its two sides.
type Corner = (Point, Point, Point);
/// A foot (the point of a rail) is at most this far from its middle point,
/// a factor of the half-width there (the distance to the edge, at most
/// 1.3 times the middle half-width of the stroke), and [`FOOT_SLACK`]
/// cells more. Farther, the side is in an opening: a place where another
/// stroke meets or crosses (P21 Milestone 1).
const FOOT_REACH: f64 = 1.6;
const FOOT_SLACK: f64 = 1.5;
/// A foot is in this angle (degrees) of the normal of the middle line,
/// on a segment of the outline in [`FOOT_ALONG_DEG`] of the direction of
/// the middle line: a side of the stroke, not the outline of another
/// stroke where it meets this one.
const FOOT_CONE_DEG: f64 = 55.0;
const FOOT_ALONG_DEG: f64 = 40.0;
/// A foot that stays at the foot before (as the feet would go back) for
/// longer than this many times its distance from the middle line (along
/// the middle line) is not the inside of a curve: the side is in an
/// opening there.
const STAY_MAX: f64 = 3.5;
/// A rail follows the outline between two feet if the outline between them
/// is at most this many times as long as the line between them (and
/// [`FOLLOW_SLACK`] cells), and its points are at most [`FOLLOW_NEAR`]
/// times the half-width of the column from the middle line (and one
/// cell). Else the outline goes away into another stroke: an opening.
const FOLLOW_ARC: f64 = 2.0;
const FOLLOW_SLACK: f64 = 2.0;
const FOLLOW_NEAR: f64 = 1.6;
/// A rail also follows the outline round a short arm that is not a stroke
/// of its own (the graph of the middle lines took it as a side branch: the
/// top of the stroke of ぁ above its bar): an outline at most this many
/// times as long as the line between the feet, at most [`STUB_NEAR`]
/// half-widths from the middle line. The column covers the arm, as the
/// lines square to the middle line did before. Milestone 3 makes such arms
/// strokes.
const STUB_ARC: f64 = 3.5;
const STUB_NEAR: f64 = 2.0;
/// Points of the outline that a rail follows go if they are nearer than
/// this (a factor of the cell) to the line through the points that stay.
const FOLLOW_THIN: f64 = 0.1;
/// Near an end at a junction or a crossing, the width of a column is held
/// for this many half-widths.
const HOLD: f64 = 0.5;
/// A stroke that ends where another stroke goes through goes into it this
/// part of its half-width (from its edge), so there is no gap at the join.
const JOIN_OVERLAP: f64 = 0.3;
/// Two strokes cross (and the shorter is cut) where their middle lines
/// cross and each goes on past the other on both sides, at least this many
/// half-widths of the other from the crossing. Else one ends on the other
/// (a T or an L), and the cut of [`trim_start_out_of`] is for it.
const CROSS_PAST: f64 = 1.5;
/// At a free end, the rails follow the outline from a pair this far from
/// the end (a factor of the width of the stroke) to the corners of the end,
/// where the outline turns more than this (degrees) from the side.
const END_BACK: f64 = 0.8;
const END_TURN_DEG: f64 = 70.0;
/// The two points of the mitre of a corner are at most this many
/// half-widths from the middle line (P21 Milestone 2), and the outer
/// corner is in this angle (degrees) of the line that halves the turn.
const MITRE_REACH: f64 = 2.6;
const MITRE_OUTER_DEG: f64 = 45.0;
/// A mitre is at least this angle (degrees) off the direction of each
/// piece.
const MITRE_ACROSS_DEG: f64 = 25.0;
/// A mitre is only where the middle line turns at most this (degrees).
const MITRE_TURN_MAX_DEG: f64 = 110.0;
/// A mitre is at most this many widths long (a square corner: 1.41).
const MITRE_MAX: f64 = 1.6;
/// The outline round a free end is at most this many widths long
/// ([`free_end`]); longer, it goes into another stroke.
const END_LOOP_MAX: f64 = 8.0;
/// The front of a free end: the points of the outline at most this part of
/// the width behind its farthest point (the end edge of a square end).
const END_FLAT: f64 = 0.15;
/// A side of a free end can go back this part of the width (a bump).
const END_BACK_SLACK: f64 = 0.15;
/// A free end stops where it is narrower than this part of the width (a
/// pointed or a round end).
const END_TIP: f64 = 0.06;
/// The front of a free end is an end edge if its points are at most this
/// part of the width from a straight line; else the end is round.
const END_STRAIGHT: f64 = 0.03;
/// The pairs of a free end are at most this part of the width apart.
const END_STEP: f64 = 0.1;
/// The traced sides of an end stay this near (a factor of the width of the
/// stroke) to the place where the middle line meets the outline.
const END_NEAR_TIP: f64 = 1.5;
/// A traced end stops where the column is narrower than this part of the
/// width of the stroke.
const END_MIN_WIDTH: f64 = 0.3;
/// The end of a column goes this far into the columns that it ends in (a
/// factor of its width there), so that there is no gap between them.
const END_OVERLAP: f64 = 0.08;
/// The end of a column is cut back where it lies in its own part farther
/// than this many times its middle width along it ([`trim_ends`]).
const SELF_GAP: f64 = 3.0;
/// The points of [`trim_ends`] across a column are this part of its width
/// in from its sides.
const SIDE_IN: f64 = 0.12;
/// Near its end (this many times its width), a column of [`trim_ends`] can
/// be out of the others before it goes into them.
const END_LEAD: f64 = 0.6;

const NONE: usize = usize::MAX;

/// The split of the outline of a glyph into strokes before the stroke
/// finder (P21 Milestone 8, ADR 0152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Split {
    /// No split: the stroke finder on the whole glyph.
    #[default]
    Off,
    /// Bridges between pairs of concave corners (kana and kanji).
    Bridges,
    /// Bridges, and a cut at each sharp concave corner with no bridge (the
    /// notch of a "V"; Latin letters and digits).
    Notches,
}

impl Split {
    /// The split for the glyph of a character: the scripts that the trial
    /// of P21 Milestone 8 measured. Other characters (punctuation, Hangul,
    /// Arabic, Devanagari, Greek, Cyrillic) are not split.
    pub fn for_char(c: char) -> Split {
        let latin =
            c.is_ascii_alphanumeric() || (c.is_alphabetic() && matches!(c as u32, 0x00C0..=0x024F | 0x1E00..=0x1EFF));
        if latin {
            Split::Notches
        } else if crate::layout::is_cjk(c) && !matches!(c as u32, 0xAC00..=0xD7AF) {
            Split::Bridges
        } else {
            Split::Off
        }
    }
}

/// Satin columns for a glyph, with checks of how well they fit it.
#[derive(Debug, Clone, Default)]
pub struct Strokes {
    pub satins: Vec<SatinGeometry>,
    /// The part of the shape that the columns cover, 0 to 1.
    pub coverage: f64,
    /// The widest column, in font units.
    pub max_width: f64,
    /// The middle width of the columns (over their rungs), in font units.
    pub typical_width: f64,
}

#[derive(Debug, Clone)]
struct Node {
    pos: Point,
    /// Half-width of the shape at the node.
    r: f64,
}

#[derive(Debug, Clone)]
struct Edge {
    /// Node at the start and at the end (`NONE` for a closed edge).
    ends: [usize; 2],
    pts: Vec<Point>,
    closed: bool,
    alive: bool,
}

/// One end of an edge: (edge, 0 for the start or 1 for the end).
type Half = (usize, usize);

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

fn length(p: &[Point]) -> f64 {
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

impl Graph {
    fn build(g: &Grid, skel: &[bool], radius: &dyn Fn(usize) -> f64) -> Graph {
        let count: Vec<usize> = (0..skel.len())
            .map(|k| {
                if skel[k] {
                    g.neighbors(k).filter(|&q| skel[q]).count()
                } else {
                    0
                }
            })
            .collect();
        let is_node = |k: usize| skel[k] && count[k] != 2;
        // Nodes: line ends alone; junction cells that touch are one node.
        let mut node_of = vec![NONE; skel.len()];
        let mut nodes: Vec<Node> = Vec::new();
        for k in 0..skel.len() {
            if !is_node(k) || node_of[k] != NONE {
                continue;
            }
            let id = nodes.len();
            let mut cluster = vec![k];
            node_of[k] = id;
            if count[k] >= 3 {
                let mut i = 0;
                while i < cluster.len() {
                    let c = cluster[i];
                    for q in g.neighbors(c) {
                        if is_node(q) && count[q] >= 3 && node_of[q] == NONE {
                            node_of[q] = id;
                            cluster.push(q);
                        }
                    }
                    i += 1;
                }
            }
            let n = cluster.len() as f64;
            let pos = cluster.iter().fold(Point::new(0.0, 0.0), |a, &c| a + g.center_of(c)) * (1.0 / n);
            let r = cluster.iter().map(|&c| radius(c)).fold(0.0, f64::max);
            nodes.push(Node { pos, r });
        }
        let mut edges = Vec::new();
        let mut visited = vec![false; skel.len()];
        let mut direct: Vec<(usize, usize)> = Vec::new();
        for k in 0..skel.len() {
            if !is_node(k) {
                continue;
            }
            for q in g.neighbors(k).collect::<Vec<_>>() {
                if !skel[q] {
                    continue;
                }
                if is_node(q) {
                    let (a, b) = (node_of[k], node_of[q]);
                    let key = (a.min(b), a.max(b));
                    if a != b && !direct.contains(&key) {
                        direct.push(key);
                        edges.push(Edge {
                            ends: [a, b],
                            pts: vec![g.center_of(k), g.center_of(q)],
                            closed: false,
                            alive: true,
                        });
                    }
                    continue;
                }
                if visited[q] {
                    continue;
                }
                let mut pts = vec![g.center_of(k), g.center_of(q)];
                visited[q] = true;
                let (mut prev, mut cur) = (k, q);
                let mut end = NONE;
                loop {
                    let next = g
                        .neighbors(cur)
                        .find(|&x| skel[x] && x != prev && (is_node(x) || !visited[x]));
                    match next {
                        Some(x) if is_node(x) => {
                            pts.push(g.center_of(x));
                            end = node_of[x];
                            break;
                        }
                        Some(x) => {
                            visited[x] = true;
                            pts.push(g.center_of(x));
                            (prev, cur) = (cur, x);
                        }
                        None => break,
                    }
                }
                if end == NONE {
                    // A dead end (should not occur): the line ends here.
                    end = nodes.len();
                    nodes.push(Node {
                        pos: *pts.last().unwrap_or(&pts[0]),
                        r: radius(cur),
                    });
                }
                edges.push(Edge {
                    ends: [node_of[k], end],
                    pts,
                    closed: false,
                    alive: true,
                });
            }
        }
        // Loops with no node.
        for k in 0..skel.len() {
            if !skel[k] || is_node(k) || visited[k] {
                continue;
            }
            visited[k] = true;
            let mut pts = vec![g.center_of(k)];
            let (mut prev, mut cur) = (NONE, k);
            while let Some(x) = g.neighbors(cur).find(|&x| skel[x] && x != prev && !visited[x]) {
                visited[x] = true;
                pts.push(g.center_of(x));
                (prev, cur) = (cur, x);
            }
            if pts.len() >= 3 {
                edges.push(Edge {
                    ends: [NONE, NONE],
                    pts,
                    closed: true,
                    alive: true,
                });
            }
        }
        Graph { nodes, edges }
    }

    fn halves(&self, n: usize) -> Vec<Half> {
        let mut v = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            if !e.alive || e.closed {
                continue;
            }
            for (k, end) in e.ends.iter().enumerate() {
                if *end == n {
                    v.push((i, k));
                }
            }
        }
        v
    }

    fn degree(&self, n: usize) -> usize {
        self.halves(n).len()
    }

    /// The points of an edge from end `k` to the other end.
    fn points_from(&self, (e, k): Half) -> Vec<Point> {
        let mut p = self.edges[e].pts.clone();
        if k == 1 {
            p.reverse();
        }
        p
    }

    /// Removes short branches from line ends to junctions. The side
    /// branches of a junction are found first and go together (P21
    /// Milestone 3): at a square end of a stroke, the middle line forks to
    /// the two corners of the end; one by one, the second branch was no
    /// longer a side branch when the first went, and the line bent to one
    /// corner. Together, the line ends at the fork, in the middle. Where all
    /// the branches of a junction are side branches (a small part), the
    /// longest stays.
    fn prune(&mut self, radius_at: &dyn Fn(Point) -> f64, scale: f64) -> bool {
        let mut spurs: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..self.edges.len() {
            let e = &self.edges[i];
            if !e.alive || e.closed {
                continue;
            }
            let (a, b) = (e.ends[0], e.ends[1]);
            let (da, db) = (self.degree(a), self.degree(b));
            let junction = match (da, db) {
                (1, d) if d >= 3 => b,
                (d, 1) if d >= 3 => a,
                _ => continue,
            };
            let pos = self.nodes[junction].pos;
            let r = self.side_radius(junction, i, radius_at);
            let reach = self.edges[i]
                .pts
                .iter()
                .map(|&p| p.distance(pos) + radius_at(p))
                .fold(0.0, f64::max);
            if length(&self.edges[i].pts) < scale * PRUNE * r && reach < scale * SPUR_REACH * r {
                spurs.entry(junction).or_default().push(i);
            }
        }
        let mut changed = false;
        let mut junctions: Vec<usize> = spurs.keys().copied().collect();
        junctions.sort_unstable();
        for junction in junctions {
            let mut list = spurs.remove(&junction).unwrap_or_default();
            if list.len() >= self.degree(junction) {
                list.sort_by(|&x, &y| length(&self.edges[y].pts).total_cmp(&length(&self.edges[x].pts)));
                list.remove(0);
            }
            for i in list {
                self.edges[i].alive = false;
                changed = true;
            }
        }
        changed
    }

    /// The half-width of the strokes that a branch `spur` at the node `n`
    /// hangs on (P21 Milestone 3): the middle half-width of the other edges
    /// of the node, from one to three times the half-width of the node from
    /// it. Not the half-width of the node: where strokes cross, the circle
    /// in the crossing is larger than the strokes, and the short arms of a
    /// crossing (the bar of a "t") were taken as side branches. The
    /// half-width of the node if the other edges have no points there.
    fn side_radius(&self, n: usize, spur: usize, radius_at: &dyn Fn(Point) -> f64) -> f64 {
        let (pos, r) = (self.nodes[n].pos, self.nodes[n].r);
        let mut v: Vec<f64> = self
            .halves(n)
            .into_iter()
            .filter(|&(e, _)| e != spur)
            .flat_map(|(e, _)| self.edges[e].pts.iter().copied())
            .filter(|p| (r..=3.0 * r).contains(&p.distance(pos)))
            .map(radius_at)
            .collect();
        if v.is_empty() {
            return r;
        }
        v.sort_by(f64::total_cmp);
        v[v.len() / 2].min(r)
    }

    /// Joins the two edges at each node with two edges.
    fn merge(&mut self) {
        loop {
            let Some((_, h)) = (0..self.nodes.len())
                .map(|n| (n, self.halves(n)))
                .find(|(_, h)| h.len() == 2)
            else {
                return;
            };
            let ((e1, k1), (e2, k2)) = (h[0], h[1]);
            if e1 == e2 {
                // A loop through this node only.
                let e = &mut self.edges[e1];
                e.closed = true;
                e.ends = [NONE, NONE];
                if e.pts.len() > 2 && e.pts.first().unwrap().distance(*e.pts.last().unwrap()) < 1e-9 {
                    e.pts.pop();
                }
                continue;
            }
            let mut pts = self.points_from((e1, k1));
            pts.reverse();
            pts.extend(self.points_from((e2, k2)).into_iter().skip(1));
            let ends = [self.edges[e1].ends[1 - k1], self.edges[e2].ends[1 - k2]];
            self.edges[e1].alive = false;
            self.edges[e2].alive = false;
            self.edges.push(Edge {
                ends,
                pts,
                closed: false,
                alive: true,
            });
        }
    }

    /// Makes one junction of two junctions that a short edge joins.
    fn join_near(&mut self) -> bool {
        for i in 0..self.edges.len() {
            let e = &self.edges[i];
            if !e.alive || e.closed || e.ends[0] == e.ends[1] {
                continue;
            }
            let (a, b) = (e.ends[0], e.ends[1]);
            if self.degree(a) < 3 || self.degree(b) < 3 {
                continue;
            }
            let r = self.nodes[a].r.max(self.nodes[b].r);
            if length(&e.pts) < JOIN * r {
                self.edges[i].alive = false;
                self.nodes[a].pos = self.nodes[a].pos.midpoint(self.nodes[b].pos);
                self.nodes[a].r = r;
                for e in self.edges.iter_mut().filter(|e| e.alive) {
                    for end in &mut e.ends {
                        if *end == b {
                            *end = a;
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    /// The direction in which a half-edge leaves its node, and the point
    /// of the edge where the direction is taken (for its width): the
    /// direction of the edge away from the junction, where the junction
    /// does not bend its middle line.
    fn direction(&self, h: Half) -> Option<(Point, Point)> {
        let pts = self.points_from(h);
        let node = &self.nodes[self.edges[h.0].ends[h.1]];
        let len = length(&pts);
        let from = (TANGENT_FROM * node.r).min(0.25 * len);
        let to = (TANGENT_TO * node.r).min(len).max(from + 1e-9);
        let (mut d, mut a, mut b) = (0.0, pts[0], *pts.last()?);
        let mut found = false;
        for w in pts.windows(2) {
            d += w[0].distance(w[1]);
            if !found && d >= from {
                a = w[1];
                found = true;
            }
            if d >= to {
                b = w[1];
                break;
            }
        }
        let dir = (b - a)
            .normalized()
            .or_else(|| (b - node.pos).normalized())
            .or_else(|| (b - pts[0]).normalized())?;
        Some((dir, a.midpoint(b)))
    }

    /// At each junction, links the pairs of half-edges that continue most
    /// straight, with widths that are about the same.
    fn links(&self, radius_at: &dyn Fn(Point) -> f64) -> HashMap<Half, Half> {
        let mut link = HashMap::new();
        for n in 0..self.nodes.len() {
            let hs = self.halves(n);
            if hs.len() < 3 {
                continue;
            }
            let dirs: Vec<Option<(Point, f64)>> = hs
                .iter()
                .map(|&h| self.direction(h).map(|(d, at)| (d, radius_at(at))))
                .collect();
            let mut pairs = Vec::new();
            for i in 0..hs.len() {
                for j in i + 1..hs.len() {
                    if let (Some((a, ra)), Some((b, rb))) = (dirs[i], dirs[j]) {
                        // The angle between the two edges (180: one line),
                        // less for a change of the width.
                        let angle = emb_geom::math::to_degrees(emb_geom::math::atan2(a.cross(b).abs(), a.dot(b)));
                        let same = ra.min(rb) / ra.max(rb).max(1e-9);
                        let angle = angle - WIDTH_DEG * (1.0 - same);
                        if angle >= STRAIGHT_DEG {
                            pairs.push((-angle, i, j));
                        }
                    }
                }
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut used = vec![false; hs.len()];
            for (_, i, j) in pairs {
                if !used[i] && !used[j] && hs[i].0 != hs[j].0 {
                    used[i] = true;
                    used[j] = true;
                    link.insert(hs[i], hs[j]);
                    link.insert(hs[j], hs[i]);
                }
            }
        }
        link
    }

    /// The strokes: chains of linked edges, with a flag for each end that is
    /// a free line end (not a junction).
    fn chains(&self, radius_at: &dyn Fn(Point) -> f64) -> Vec<Raw> {
        let link = self.links(radius_at);
        let mut done = vec![false; self.edges.len()];
        let mut out = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            if !e.alive || done[i] {
                continue;
            }
            if e.closed {
                done[i] = true;
                out.push(Raw {
                    pts: e.pts.clone(),
                    closed: true,
                    free: [false, false],
                    meets: [false, false],
                    r_fix: None,
                });
                continue;
            }
            // Walk back from the start of edge i to the start of the chain.
            let mut h: Half = (i, 0);
            let mut closed = false;
            let mut guard = 0;
            while let Some(&(f, kf)) = link.get(&h) {
                if f == i {
                    closed = true;
                    break;
                }
                h = (f, 1 - kf);
                guard += 1;
                if guard > self.edges.len() {
                    break;
                }
            }
            // `h` is the free end of the chain: enter there.
            let (mut e, mut k) = if closed { (i, 0) } else { h };
            let start_node = self.edges[e].ends[k];
            let mut pts: Vec<Point> = Vec::new();
            let mut end_node;
            loop {
                done[e] = true;
                let p = self.points_from((e, k));
                if pts.is_empty() {
                    pts = p;
                } else {
                    pts.extend(p.into_iter().skip(1));
                }
                end_node = self.edges[e].ends[1 - k];
                match link.get(&(e, 1 - k)) {
                    Some(&(f, kf)) if !done[f] => (e, k) = (f, kf),
                    _ => break,
                }
            }
            let free = |n: usize| n != NONE && self.degree(n) == 1;
            // Another stroke goes through the node: two of its edges are linked there.
            let through = |n: usize| n != NONE && link.keys().any(|&(f, kf)| self.edges[f].ends[kf] == n);
            out.push(Raw {
                closed,
                free: [!closed && free(start_node), !closed && free(end_node)],
                meets: [!closed && through(start_node), !closed && through(end_node)],
                pts,
                r_fix: None,
            });
        }
        out
    }
}

/// A stroke before smoothing.
#[derive(Debug, Clone)]
struct Raw {
    pts: Vec<Point>,
    closed: bool,
    /// The start and the end are free line ends (to extend to the edge).
    free: [bool; 2],
    /// The start and the end are where another stroke goes through.
    meets: [bool; 2],
    /// A fixed half-width (for the long axis of a dot).
    r_fix: Option<f64>,
}

/// The point of a line nearest to `p`, and its distance.
fn nearest(line: &[Point], p: Point) -> (Point, f64) {
    let mut best = (line[0], line[0].distance(p));
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ab = b - a;
        let len2 = ab.dot(ab);
        let t = if len2 > 0.0 {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + ab * t;
        let d = q.distance(p);
        if d < best.1 {
            best = (q, d);
        }
    }
    best
}

/// Cuts the start of a stroke that lies in other strokes, up to the point
/// where it is out of them but [`JOIN_OVERLAP`] of their half-width: a point
/// is in a stroke while it is nearer to its middle line than
/// `(1 − JOIN_OVERLAP)` times its half-width (the one given, else the
/// half-width at the nearest point). The cut is after the last point in the
/// first half of the stroke that is in another stroke (an extended end can
/// go across the other stroke and out again), so a short stroke keeps most
/// of its length. Returns the number of points that went.
///
/// With `own` (the half-width of the stroke; 0: none), a point is in
/// another stroke only if the stroke is in it with its whole width there:
/// the two ends of the line across the stroke are both that near to the
/// middle line of the other stroke. A stroke that leaves another stroke at
/// a small angle (a diagonal of "M" at its stem) then keeps its start: a
/// cut at its middle line would leave the outer part of its width with no
/// stitches.
fn trim_start_out_of(
    pts: &mut Vec<Point>,
    others: &[(&[Point], Option<f64>)],
    radius_at: &dyn Fn(Point) -> f64,
    step: f64,
    own: f64,
) -> usize {
    trim_start(pts, others, radius_at, step, own, false)
}

/// [`trim_start_out_of`] for the new end of a stroke cut at a crossing
/// (P20 Milestone 3): the cut is where the stroke first comes out of the
/// stroke that goes through. A stroke that crosses the same stroke two
/// times (the diagonal of ぬ through its loop) keeps its part between the
/// crossings.
fn trim_start_at_exit(
    pts: &mut Vec<Point>,
    others: &[(&[Point], Option<f64>)],
    radius_at: &dyn Fn(Point) -> f64,
    step: f64,
    own: f64,
) -> usize {
    trim_start(pts, others, radius_at, step, own, true)
}

fn trim_start(
    pts: &mut Vec<Point>,
    others: &[(&[Point], Option<f64>)],
    radius_at: &dyn Fn(Point) -> f64,
    step: f64,
    own: f64,
    first_exit: bool,
) -> usize {
    if others.is_empty() || pts.len() < 2 {
        return 0;
    }
    let inside = |p: Point, along: Point| {
        let across = along.normalized().map(|u| Point::new(-u.y, u.x) * own);
        others.iter().any(|(l, r)| {
            let near = |p: Point| {
                let (q, d) = nearest(l, p);
                d < (1.0 - JOIN_OVERLAP) * r.unwrap_or_else(|| radius_at(q))
            };
            match across.filter(|_| own > 0.0) {
                Some(n) => near(p + n) && near(p - n),
                None => near(p),
            }
        })
    };
    let half = 0.5 * length(pts);
    let step = step.max(1e-9);
    // The direction of the stroke at its start, over about its width.
    let ahead = |k: usize| {
        let mut j = k + 1;
        while j + 1 < pts.len() && pts[j].distance(pts[k]) < own.max(step) {
            j += 1;
        }
        pts[j] - pts[k]
    };
    // Samples of the first half: the segment, the point and whether it is in.
    let mut samples: Vec<(usize, Point, bool)> = vec![(0, pts[0], inside(pts[0], ahead(0)))];
    let mut walked = 0.0;
    'walk: for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let d = a.distance(b);
        let n = (d / step).ceil().max(1.0) as usize;
        let along = ahead(k);
        for s in 1..=n {
            let t = s as f64 / n as f64;
            if walked + d * t > half {
                break 'walk;
            }
            let p = a.lerp(b, t);
            samples.push((k, p, inside(p, along)));
        }
        walked += d;
    }
    let last_in = if first_exit {
        // The end of the run of samples in the other stroke from the start.
        match samples.iter().position(|s| !s.2) {
            Some(0) | None => return 0,
            Some(out) => out - 1,
        }
    } else {
        let Some(last_in) = samples.iter().rposition(|s| s.2) else {
            return 0;
        };
        last_in
    };
    let Some(&(k, p, _)) = samples.get(last_in + 1) else {
        // In other strokes up to the middle: keep the stroke.
        return 0;
    };
    pts.drain(..=k);
    pts.insert(0, p);
    k
}

/// Moving average over `win` points on each side; open lines keep their ends.
fn smooth(p: &[Point], closed: bool, win: usize) -> Vec<Point> {
    let n = p.len();
    if n < 3 || win == 0 {
        return p.to_vec();
    }
    (0..n)
        .map(|i| {
            if !closed && (i == 0 || i == n - 1) {
                return p[i];
            }
            let mut sum = Point::new(0.0, 0.0);
            let mut c = 0.0;
            for d in -(win as isize)..=win as isize {
                let j = i as isize + d;
                let j = if closed {
                    j.rem_euclid(n as isize) as usize
                } else if j < 0 || j >= n as isize {
                    continue;
                } else {
                    j as usize
                };
                sum += p[j];
                c += 1.0;
            }
            sum * (1.0 / c)
        })
        .collect()
}

/// Points about `step` apart along the line (the ends stay).
fn resample(p: &[Point], closed: bool, step: f64) -> Vec<Point> {
    let mut pts = p.to_vec();
    if closed && let Some(&f) = pts.first() {
        pts.push(f);
    }
    let total = length(&pts);
    if total < 1e-9 || pts.len() < 2 {
        return p.to_vec();
    }
    let n = (total / step).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(n + 1);
    let (mut seg, mut acc) = (0usize, 0.0);
    for q in 0..=n {
        let want = total * q as f64 / n as f64;
        while seg + 1 < pts.len() - 1 && acc + pts[seg].distance(pts[seg + 1]) < want {
            acc += pts[seg].distance(pts[seg + 1]);
            seg += 1;
        }
        let l = pts[seg].distance(pts[seg + 1]);
        let t = if l > 0.0 {
            ((want - acc) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(pts[seg].lerp(pts[seg + 1], t));
    }
    if closed {
        out.pop();
    }
    out
}

/// The distance along `dir` from `o` to the nearest outline segment, up to `max`.
fn ray(o: Point, dir: Point, max: f64, segs: &[(Point, Point)]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for &(a, b) in segs {
        let ab = b - a;
        let den = dir.cross(ab);
        if den.abs() < 1e-12 {
            continue;
        }
        let ao = a - o;
        let t = ao.cross(ab) / den;
        let u = ao.cross(dir) / den;
        if t > 1e-9 && t <= max && (-1e-9..=1.0 + 1e-9).contains(&u) && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    }
    best
}

/// One stroke after smoothing and cutting, ready for rails.
struct Piece {
    pts: Vec<Point>,
    closed: bool,
    /// Points from this index on (and before `ext[0]`) are extensions past
    /// the middle line: they take the width of the last point on it.
    ext: [usize; 2],
    /// Near an end at a junction, the width is held for this length (the
    /// lines across the stroke would reach into the other stroke there).
    hold: [f64; 2],
    /// The start and the end are the free ends of the pen stroke (not at
    /// another stroke, not at a cut of a corner).
    free: [bool; 2],
    /// The mitre of an end at a cut of a corner (P21 Milestone 2): the
    /// inner and the outer corner of the outline.
    mitre: [Option<(Point, Point)>; 2],
}

/// The corners of the outline: the points where it turns more than
/// [`OUTLINE_CORNER_DEG`] over a short length (a curve of the font is many
/// short lines with small turns, and has none).
fn outline_corners(rings: &[Vec<Point>], cap_height: f64) -> Vec<Corner> {
    let span = (OUTLINE_CORNER_SPAN * cap_height).max(1e-9);
    let cos_max = emb_geom::math::cos(emb_geom::math::to_radians(OUTLINE_CORNER_DEG));
    let mut out: Vec<(f64, Corner)> = Vec::new();
    for ring in rings {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        // The point of the ring at the distance `span` from point `i`, in
        // the direction `step` (1 or n - 1).
        let away = |i: usize, step: usize| -> Point {
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
        };
        for (i, &at) in ring.iter().enumerate() {
            let (a, b) = (at - away(i, n - 1), away(i, 1) - at);
            if let (Some(a), Some(b)) = (a.normalized(), b.normalized())
                && a.dot(b) < cos_max
            {
                out.push((a.dot(b), (at, a, b)));
            }
        }
    }
    // The sharpest point of each corner.
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut corners: Vec<Corner> = Vec::new();
    for (_, c) in out {
        if corners.iter().all(|k| k.0.distance(c.0) > span) {
            corners.push(c);
        }
    }
    corners
}

/// Cuts a stroke where it turns more than [`CORNER_DEG`], or more than
/// [`CORNER_AT_OUTLINE_DEG`] at one of the `corners` of the outline.
/// Returns the point indices of each piece.
fn cut_corners(
    p: &[Point],
    closed: bool,
    free: [bool; 2],
    reach: f64,
    corners: &[Corner],
) -> Vec<(Vec<usize>, bool, [bool; 2])> {
    let n = p.len();
    if n < 5 {
        return vec![((0..n).collect(), closed, free)];
    }
    let spacing = length(p) / (n - 1) as f64;
    let k = ((reach / spacing.max(1e-9)).round() as usize).clamp(1, n / 4 + 1);
    let cos_max = emb_geom::math::cos(emb_geom::math::to_radians(CORNER_DEG));
    let cos_at_corner = emb_geom::math::cos(emb_geom::math::to_radians(CORNER_AT_OUTLINE_DEG));
    let cos_along = emb_geom::math::cos(emb_geom::math::to_radians(OUTLINE_CORNER_ALONG_DEG));
    // `reach` is 1.5 half-widths of the stroke.
    let near = OUTLINE_CORNER_REACH * reach / 1.5;
    let at = |i: isize| -> Point { p[i.rem_euclid(n as isize) as usize] };
    let mut turn = vec![1.0f64; n];
    // The turn is at a corner of the outline on its outer side.
    let mut cornered = vec![false; n];
    // The point of the stroke nearest to each corner: a corner is at a
    // turn only if the turn is the part of the stroke nearest to it. In a
    // heavy "S", the corners of the two ends are near the turns of its
    // middle, but they are nearer to the ends.
    let nearest_to: Vec<usize> = corners
        .iter()
        .map(|c| {
            (0..n)
                .min_by(|&x, &y| p[x].distance(c.0).total_cmp(&p[y].distance(c.0)))
                .unwrap_or(0)
        })
        .collect();
    for (i, t) in turn.iter_mut().enumerate() {
        let here = p[i];
        let i = i as isize;
        if !closed && (i < k as isize || i + (k as isize) >= n as isize) {
            continue;
        }
        let (a, b) = (at(i) - at(i - k as isize), at(i + k as isize) - at(i));
        if let (Some(a), Some(b)) = (a.normalized(), b.normalized()) {
            *t = a.dot(b);
            // `b - a` points to the inner side of the turn.
            let inward = b - a;
            // The outline can go the other way than the stroke.
            let along = |x: Point, y: Point| x.dot(a).abs() >= cos_along && y.dot(b).abs() >= cos_along;
            cornered[i as usize] = *t < cos_at_corner
                && corners
                    .iter()
                    .zip(&nearest_to)
                    .any(|(&(c, side_a, side_b), &at_stroke)| {
                        let apart = at_stroke.abs_diff(i as usize);
                        let apart = if closed { apart.min(n - apart) } else { apart };
                        c.distance(here) <= near
                            && apart <= k
                            && (c - here).dot(inward) < 0.0
                            && (along(side_a, side_b) || along(side_b, side_a))
                    });
        }
    }
    // Local minima of the cosine below the limit, at least 2k apart.
    let mut cuts: Vec<usize> = Vec::new();
    let mut order: Vec<usize> = (0..n).filter(|&i| turn[i] < cos_max || cornered[i]).collect();
    order.sort_by(|&a, &b| turn[a].total_cmp(&turn[b]));
    for i in order {
        let near = cuts.iter().any(|&c| {
            let d = c.abs_diff(i);
            let d = if closed { d.min(n - d) } else { d };
            d < 2 * k
        });
        if !near {
            cuts.push(i);
        }
    }
    if cuts.is_empty() {
        return vec![((0..n).collect(), closed, free)];
    }
    cuts.sort_unstable();
    let mut out = Vec::new();
    if closed {
        for w in 0..cuts.len() {
            let (s, e) = (cuts[w], cuts[(w + 1) % cuts.len()]);
            let mut seg = Vec::new();
            let mut i = s;
            loop {
                seg.push(i);
                if i == e && seg.len() > 1 {
                    break;
                }
                i = (i + 1) % n;
            }
            out.push((seg, false, [true, true]));
        }
    } else {
        let mut s = 0;
        for (w, &c) in cuts.iter().enumerate() {
            out.push(((s..=c).collect(), false, [if w == 0 { free[0] } else { true }, true]));
            s = c;
        }
        out.push(((s..n).collect(), false, [true, free[1]]));
    }
    out
}

/// Extends free ends along their direction to the edge of the shape.
/// One piece of a stroke: the stroke, its fixed half-width, its middle
/// half-width and the piece.
type Placed = (usize, Option<f64>, f64, Piece);

/// Cuts the shorter of two strokes that cross (P20 Milestone 3): it is two
/// pieces that end a little inside the edge of the longer one
/// ([`trim_start_out_of`]), so the crossing has one layer of thread. A
/// stroke along the long axis of a dot and a closed stroke are not cut.
fn cut_crossings(all: &mut Vec<Placed>, radius_at: &dyn Fn(Point) -> f64, cell: f64) {
    // Each cut removes one crossing; the guard is for a case that does not
    // end.
    for _ in 0..4 * all.len() + 8 {
        let Some((cut, through, k, x)) = next_crossing(all) else {
            return;
        };
        let (_, _, r_cut, ref piece) = all[cut];
        let hold = 1.5 * r_cut;
        let Some((mut a, mut b)) = split_piece(piece, k, x, hold) else {
            return;
        };
        let through_line = all[through].3.pts.clone();
        let other = [(through_line.as_slice(), Some(all[through].2))];
        let gone = trim_start_at_exit(&mut b.pts, &other, radius_at, cell, r_cut);
        b.ext[1] = b.ext[1].saturating_sub(gone).max(1);
        a.pts.reverse();
        trim_start_at_exit(&mut a.pts, &other, radius_at, cell, r_cut);
        a.pts.reverse();
        a.ext[1] = a.pts.len();
        a.ext[0] = a.ext[0].min(a.ext[1].saturating_sub(1));
        let stroke = all[cut].0;
        all[cut].3 = a;
        all.insert(cut + 1, (stroke, None, r_cut, b));
    }
}

/// The first crossing of two pieces: the piece to cut, the piece that goes
/// through, the segment of the cut piece and the point.
fn next_crossing(all: &[Placed]) -> Option<(usize, usize, usize, Point)> {
    let boxes: Vec<Option<BBox>> = all.iter().map(|p| BBox::from_points(p.3.pts.iter().copied())).collect();
    for a in 0..all.len() {
        for b in a + 1..all.len() {
            let (Some(ba), Some(bb)) = (boxes[a], boxes[b]) else {
                continue;
            };
            if ba.max.x < bb.min.x || bb.max.x < ba.min.x || ba.max.y < bb.min.y || bb.max.y < ba.min.y {
                continue;
            }
            if all[a].1.is_some() || all[b].1.is_some() {
                continue;
            }
            let (la, lb) = (length(&all[a].3.pts), length(&all[b].3.pts));
            let (through, cut) = if la >= lb { (a, b) } else { (b, a) };
            let (t, c) = (&all[through], &all[cut]);
            if c.3.closed || c.3.pts.len() < 2 || t.3.pts.len() < 2 {
                continue;
            }
            let (lc, lt) = (length(&c.3.pts), length(&t.3.pts));
            for (k, j, x) in crossings(&c.3.pts, &t.3.pts) {
                let on_c = length(&c.3.pts[..=k]) + c.3.pts[k].distance(x);
                let on_t = length(&t.3.pts[..=j]) + t.3.pts[j].distance(x);
                let past = |on: f64, total: f64, r: f64| on > CROSS_PAST * r && total - on > CROSS_PAST * r;
                if past(on_c, lc, t.2) && (t.3.closed || past(on_t, lt, c.2)) {
                    return Some((cut, through, k, x));
                }
            }
        }
    }
    None
}

/// The points where two lines cross: the segment of `a`, the segment of
/// `b` and the point.
fn crossings(a: &[Point], b: &[Point]) -> Vec<(usize, usize, Point)> {
    let mut out = Vec::new();
    for i in 0..a.len().saturating_sub(1) {
        for j in 0..b.len().saturating_sub(1) {
            if let Some((u, _)) = Point::crossing(a[i], a[i + 1], b[j], b[j + 1]) {
                out.push((i, j, a[i].lerp(a[i + 1], u)));
            }
        }
    }
    out
}

/// A piece cut at `x` on its segment `k` into two pieces. The new ends hold
/// their width for `hold` (they are in another stroke). `None` if the cut
/// is in an extension of the piece.
fn split_piece(p: &Piece, k: usize, x: Point, hold: f64) -> Option<(Piece, Piece)> {
    if k < p.ext[0] || k + 1 >= p.ext[1] || k + 1 >= p.pts.len() {
        return None;
    }
    let mut first = p.pts[..=k].to_vec();
    first.push(x);
    let mut second = vec![x];
    second.extend_from_slice(&p.pts[k + 1..]);
    let n1 = first.len();
    Some((
        Piece {
            pts: first,
            closed: false,
            ext: [p.ext[0], n1],
            hold: [p.hold[0], hold],
            free: [p.free[0], false],
            mitre: [p.mitre[0], None],
        },
        Piece {
            pts: second,
            closed: false,
            ext: [0, p.ext[1] - k],
            hold: [hold, p.hold[1]],
            free: [false, p.free[1]],
            mitre: [None, p.mitre[1]],
        },
    ))
}

fn extend(mut pts: Vec<Point>, free: [bool; 2], g: &Grid, reach: f64) -> Piece {
    let n0 = pts.len();
    let step = g.cell * 0.5;
    let back = |p: &[Point], from_end: bool| -> Option<Point> {
        let n = p.len();
        if n < 2 {
            return None;
        }
        let k = (n / 3).max(1);
        if from_end {
            (p[n - 1] - p[n - 1 - k]).normalized()
        } else {
            (p[0] - p[k]).normalized()
        }
    };
    let walk = |from: Point, dir: Point| -> Vec<Point> {
        let mut v = Vec::new();
        let mut s = step;
        while s < 4.0 * reach && g.is_inside(from + dir * s) {
            s += step;
        }
        let last = from + dir * (s - 0.5 * step);
        if last.distance(from) > step {
            v.push(last);
        }
        v
    };
    let mut ext = [0, n0];
    if free[1]
        && let Some(d) = back(&pts, true)
    {
        let add = walk(pts[n0 - 1], d);
        pts.extend(add);
    }
    if free[0]
        && let Some(d) = back(&pts[..n0], false)
    {
        let add = walk(pts[0], d);
        ext = [add.len(), ext[1] + add.len()];
        let mut v = add;
        v.reverse();
        v.extend(pts);
        pts = v;
    }
    Piece {
        pts,
        closed: false,
        ext,
        hold: [0.0; 2],
        free,
        mitre: [None; 2],
    }
}

/// Satin columns for a glyph shape (rings in font units, Y down).
pub fn strokes(rings: &[Vec<Point>], cap_height: f64) -> Strokes {
    strokes_split(rings, cap_height, Split::Off)
}

/// [`strokes`] after the split `split` of the outline: the columns of each
/// piece, as one.
pub fn strokes_split(rings: &[Vec<Point>], cap_height: f64, split: Split) -> Strokes {
    strokes_split_soft(rings, cap_height, split, true).0
}

/// [`strokes_split`], with or without the bridges to curves (`soft`; for
/// the scripts with notches), and `true` if the split has such a bridge.
pub(crate) fn strokes_split_soft(rings: &[Vec<Point>], cap_height: f64, split: Split, soft: bool) -> (Strokes, bool) {
    if split != Split::Off
        && let Some(s) = crate::split::split(rings, cap_height, split == Split::Notches, soft)
    {
        let parts: Vec<(Strokes, f64)> = s.pieces.iter().map(|p| (piece(&p.rings, cap_height), p.area)).collect();
        return (merged(parts), s.soft);
    }
    (whole(rings, cap_height), false)
}

/// The columns of the pieces of a shape as one, with the coverage of each
/// piece in proportion to its area.
fn merged(parts: Vec<(Strokes, f64)>) -> Strokes {
    let total: f64 = parts.iter().map(|p| p.1).sum::<f64>().max(1e-12);
    let mut out = Strokes::default();
    for (s, area) in parts {
        out.coverage += s.coverage * area / total;
        out.max_width = out.max_width.max(s.max_width);
        out.satins.extend(s.satins);
    }
    let mut widths: Vec<f64> = out
        .satins
        .iter()
        .flat_map(|s| s.rungs.iter().map(|(a, b)| a.distance(*b)))
        .collect();
    widths.sort_by(f64::total_cmp);
    out.typical_width = widths.get(widths.len() / 2).copied().unwrap_or(out.max_width);
    out
}

/// The stroke finder on a piece of a split shape. A piece is one stroke,
/// but where it ends on a bridge at a slant, or with the curve of a
/// bracket of a serif, its middle line forks at the end, and the branches
/// can be longer than side branches: with the branches of ends cut back as
/// far as [`PIECE_PRUNE`] times the usual length, the piece keeps fewer
/// columns if they cover it as well.
fn piece(rings: &[Vec<Point>], cap_height: f64) -> Strokes {
    let s = whole_with(rings, cap_height, 1.0);
    if s.satins.len() < 2 {
        return s;
    }
    let t = whole_with(rings, cap_height, PIECE_PRUNE);
    // The longest line across a column (a stitch).
    let widest = |x: &Strokes| {
        x.satins
            .iter()
            .flat_map(|g| g.rungs.iter().map(|(a, b)| a.distance(*b)))
            .fold(0.0, f64::max)
    };
    if t.satins.len() < s.satins.len()
        && t.coverage >= s.coverage - PIECE_COVER_LOSS
        && widest(&t) <= PIECE_WIDER * widest(&s)
    {
        t
    } else {
        s
    }
}

/// The stroke finder on a whole shape.
fn whole(rings: &[Vec<Point>], cap_height: f64) -> Strokes {
    whole_with(rings, cap_height, 1.0)
}

/// [`whole`] with side branches pruned up to `prune` times the usual
/// length.
fn whole_with(rings: &[Vec<Point>], cap_height: f64, prune: f64) -> Strokes {
    let Some(bb) = BBox::from_points(rings.iter().flatten().copied()) else {
        return Strokes::default();
    };
    let cell = Grid::cell_for(bb, (cap_height / GRID_CELLS).max(1e-6));
    let mut g = Grid::new(bb, cell, 3);
    g.fill(rings);
    let inside_count = g.inside.iter().filter(|&&v| v).count();
    if inside_count == 0 {
        return Strokes::default();
    }
    let dist = distance(&g);
    // Half-width at a cell: the distance to the edge, not to the next outside center.
    let radius = |k: usize| (dist[k] - 0.5).max(0.5) * cell;
    let radius_at = |p: Point| g.cell_of(p).map_or(0.5 * cell, |(i, j)| radius(g.idx(i, j)));
    let skel = thin(&g);
    let mut graph = Graph::build(&g, &skel, &radius);
    loop {
        let pruned = graph.prune(&radius_at, prune);
        graph.merge();
        let joined = graph.join_near();
        if !pruned && !joined {
            break;
        }
    }
    let raws = graph.chains(&radius_at);

    // Parts of the shape with almost no middle line get a stroke along their long axis.
    let (label, count) = parts(&g);
    let mut part_len = vec![0.0; count];
    let mut part_r = vec![0.0f64; count];
    for (k, &l) in label.iter().enumerate() {
        if l != NONE {
            part_r[l] = part_r[l].max(radius(k));
        }
    }
    let part_of = |p: Point| g.cell_of(p).map(|(i, j)| label[g.idx(i, j)]).filter(|&l| l != NONE);
    for r in &raws {
        if let Some(l) = r.pts.iter().find_map(|&p| part_of(p)) {
            part_len[l] += length(&r.pts);
        }
    }
    let mut raws: Vec<Raw> = raws
        .into_iter()
        .filter(|r| {
            r.pts
                .iter()
                .find_map(|&p| part_of(p))
                .is_none_or(|l| part_len[l] >= 1.2 * part_r[l])
        })
        .collect();
    for l in 0..count {
        if part_len[l] < 1.2 * part_r[l] {
            raws.push(long_axis(&g, &label, l, part_r[l]));
        }
    }

    let segs: Vec<(Point, Point)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |k| (r[k], r[(k + 1) % r.len()])))
        .collect();
    let outline = Rings::new(rings);
    let mut out = Strokes::default();
    let mut cover = vec![false; g.inside.len()];
    let lines: Vec<Vec<Point>> = raws.iter().map(|r| r.pts.clone()).collect();
    let corners = outline_corners(rings, cap_height);
    // The middle half-width of each stroke. At a junction the half-width
    // of the shape is the half-width of the widest stroke there; a thin
    // stroke (a serif) that goes through keeps its own.
    let middle = |r: &Raw| -> f64 {
        let mut v: Vec<f64> = r.pts.iter().map(|&p| r.r_fix.unwrap_or_else(|| radius_at(p))).collect();
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).copied().unwrap_or(cell).max(cell)
    };
    let middles: Vec<f64> = raws.iter().map(middle).collect();
    // The pieces of all strokes: the stroke, its fixed half-width, its
    // middle half-width and the piece.
    let mut all: Vec<(usize, Option<f64>, f64, Piece)> = Vec::new();
    for (i, mut raw) in raws.into_iter().enumerate() {
        // An end where another stroke goes through: from the middle of that
        // stroke back to a little inside its edge.
        for end in 0..2 {
            if !raw.meets[end] || raw.r_fix.is_some() {
                continue;
            }
            let at = if end == 0 {
                raw.pts[0]
            } else {
                raw.pts[raw.pts.len() - 1]
            };
            let r = radius_at(at);
            // The strokes that go through the node (not the ones that end there).
            let through: Vec<(&[Point], Option<f64>)> = lines
                .iter()
                .enumerate()
                .filter(|(j, l)| {
                    *j != i
                        && l.len() >= 2
                        && nearest(l, at).1 < r
                        && l[0].distance(at) > r
                        && l[l.len() - 1].distance(at) > r
                })
                .map(|(j, l)| (l.as_slice(), Some(middles[j].min(r))))
                .collect();
            if end == 1 {
                raw.pts.reverse();
            }
            trim_start_out_of(&mut raw.pts, &through, &radius_at, cell, middles[i]);
            if end == 1 {
                raw.pts.reverse();
            }
        }
        let fixed = |_: Point| raw.r_fix.unwrap_or(0.0);
        let width_at: &dyn Fn(Point) -> f64 = if raw.r_fix.is_some() { &fixed } else { &radius_at };
        let mut sorted: Vec<f64> = raw.pts.iter().map(|&p| width_at(p)).collect();
        sorted.sort_by(f64::total_cmp);
        let r_med = sorted.get(sorted.len() / 2).copied().unwrap_or(cell).max(cell);
        let win = ((0.6 * r_med / cell).round() as usize).max(1);
        // Find the corners on a smoothed line, then smooth each piece alone,
        // so that the pieces stay straight up to the corner.
        let smoothed = smooth(&raw.pts, raw.closed, win);
        let cuts = cut_corners(&smoothed, raw.closed, raw.free, 1.5 * r_med, &corners);
        let count = cuts.len();
        // The mitre at a cut point of the smoothed line (P21 Milestone 2).
        let ns = smoothed.len();
        let span = {
            let spacing = length(&smoothed) / ns.saturating_sub(1).max(1) as f64;
            ((1.5 * r_med / spacing.max(1e-9)).round() as usize).clamp(1, ns / 4 + 1)
        };
        // The middle half-width of each piece: a mitre is for two pieces of
        // about one width (not a thin diagonal on a thick stem).
        let piece_r: Vec<f64> = cuts
            .iter()
            .map(|(idx, _, _)| {
                let mut v: Vec<f64> = idx.iter().map(|&i| width_at(raw.pts[i])).collect();
                v.sort_by(f64::total_cmp);
                v.get(v.len() / 2).copied().unwrap_or(r_med).max(cell)
            })
            .collect();
        let mitre = |c: usize, r: f64| -> Option<(Point, Point)> {
            if raw.r_fix.is_some() {
                return None;
            }
            let at = |i: isize| -> Point {
                if raw.closed {
                    smoothed[i.rem_euclid(ns as isize) as usize]
                } else {
                    smoothed[i.clamp(0, ns as isize - 1) as usize]
                }
            };
            let (c, k) = (c as isize, span as isize);
            let u1 = (at(c) - at(c - k)).normalized()?;
            let u2 = (at(c + k) - at(c)).normalized()?;
            // Not at a sharp point (the V of an "M", a "W"): its outer
            // corner is far from the middle line. The smoothed line rounds
            // the point, so the turn is measured over a longer part.
            let (v1, v2) = (
                (at(c) - at(c - 2 * k)).normalized()?,
                (at(c + 2 * k) - at(c)).normalized()?,
            );
            if v1.dot(v2) < emb_geom::math::cos(emb_geom::math::to_radians(MITRE_TURN_MAX_DEG)) {
                return None;
            }
            // The mitre is short (not the long line of a sharp corner, where
            // the column would fan out) and in the letter (not across a
            // counter at a turn of a curve).
            mitre_at(at(c), u1, u2, r, &outline, &corners, &segs).filter(|&(p, q)| {
                p.distance(q) <= MITRE_MAX * 2.0 * r && (1..10).all(|k| g.is_inside(p.lerp(q, f64::from(k) / 10.0)))
            })
        };
        let mut pieces: Vec<Piece> = cuts
            .into_iter()
            .enumerate()
            .map(|(k, (idx, closed, free))| {
                let p: Vec<Point> = idx.iter().map(|&i| raw.pts[i]).collect();
                let p = smooth(&p, closed, win);
                if closed {
                    let n = p.len();
                    Piece {
                        pts: p,
                        closed,
                        ext: [0, n],
                        hold: [0.0; 2],
                        free: [false; 2],
                        mitre: [None; 2],
                    }
                } else {
                    // The ends at a cut of a corner end on its mitre.
                    let cut = [raw.closed || k > 0, raw.closed || k + 1 < count];
                    let m = piece_r.len();
                    let (before, after) = (piece_r[(k + m - 1) % m], piece_r[(k + 1) % m]);
                    let mitres = [
                        idx.first()
                            .filter(|_| cut[0])
                            .and_then(|&c| mitre(c, piece_r[k].min(before))),
                        idx.last()
                            .filter(|_| cut[1])
                            .and_then(|&c| mitre(c, piece_r[k].min(after))),
                    ];
                    let extended = [free[0] && mitres[0].is_none(), free[1] && mitres[1].is_none()];
                    let mut piece = extend(p, extended, &g, r_med);
                    piece.mitre = mitres;
                    for ((hold, free), m) in piece.hold.iter_mut().zip(free).zip(mitres) {
                        if !free && m.is_none() && raw.r_fix.is_none() {
                            *hold = HOLD * r_med;
                        }
                    }
                    // The ends of the pen stroke, not the ends at a cut of
                    // a corner (they go on into the corner).
                    piece.free = [
                        k == 0 && !raw.closed && raw.free[0],
                        k + 1 == count && !raw.closed && raw.free[1],
                    ];
                    piece
                }
            })
            .collect();
        // At a corner the piece before keeps the corner, and this piece
        // starts where it comes out of it (the apex of an "A").
        let n = pieces.len();
        if raw.r_fix.is_none() && n >= 2 {
            let first = if raw.closed { 0 } else { 1 };
            for k in first..n {
                let before = pieces[(k + n - 1) % n].pts.clone();
                let piece = &mut pieces[k];
                if piece.closed || piece.ext[0] > 0 || piece.mitre[0].is_some() {
                    continue;
                }
                let gone = trim_start_out_of(&mut piece.pts, &[(&before, None)], &radius_at, cell, 0.0);
                piece.ext[1] = piece.ext[1].saturating_sub(gone).max(1);
            }
        }
        all.extend(pieces.into_iter().map(|p| (i, raw.r_fix, r_med, p)));
    }
    // An end that was extended to the edge of the shape into an earlier
    // piece (the two legs of an "A" with a flat top): it stops a little
    // inside the edge of that piece.
    for k in 0..all.len() {
        let (_, fixed, _, ref piece) = all[k];
        if fixed.is_some() || piece.closed {
            continue;
        }
        // The pieces before this one (also of the same stroke: the legs of
        // an "A" are two pieces cut at the apex). The extended parts are
        // near the edge of the shape, where the half-width is small: the
        // middle half-width of the stroke counts. Not a short piece just
        // before this one on the same middle line, cut from it at a corner
        // (a flag serif at the top of a stem): the stem goes on through it
        // to the edge, and the flag stops at the stem (`trim_ends`). Cut
        // back, the stem stopped under the flag, and the flag went on across
        // the stem at a slant and out of the letter (P21: "l" in Bree
        // Serif). A serif on the two sides of a stem is a stroke of its own
        // and still cuts the stem back.
        let (stroke, r_this) = (all[k].0, all[k].2);
        let earlier: Vec<(Vec<Point>, f64)> = all[..k]
            .iter()
            .enumerate()
            .filter(|&(j, (s, f, _, p))| {
                let core = &p.pts[p.ext[0].min(p.pts.len())..p.ext[1].min(p.pts.len())];
                let flag = *s == stroke && j + 1 == k && !p.closed && length(core) < SIDE_PIECE * r_this;
                f.is_none() && p.pts.len() >= 2 && !flag
            })
            .map(|(_, (_, _, r, p))| (p.pts.clone(), *r))
            .collect();
        let earlier: Vec<(&[Point], Option<f64>)> = earlier.iter().map(|(l, r)| (l.as_slice(), Some(*r))).collect();
        let r_med = all[k].2;
        // The cut is for the extended part of the end. A stroke that goes
        // through a junction (the foot of an "L" with a serif on the other
        // side of its stem) is in the other stroke at the junction, far
        // from its end: it keeps its end. So a cut longer than the
        // extended part and the half-width of the stroke is not made.
        let cut = |pts: &mut Vec<Point>, extended: usize| -> usize {
            let mut trial = pts.clone();
            let gone = trim_start_out_of(&mut trial, &earlier, &radius_at, cell, 0.0);
            let most = length(&pts[..=extended.min(pts.len() - 1)]) + r_med;
            if gone > 0 && length(pts) - length(&trial) <= most {
                *pts = trial;
                gone
            } else {
                0
            }
        };
        let piece = &mut all[k].3;
        if piece.ext[0] > 0 {
            let gone = cut(&mut piece.pts, piece.ext[0]);
            piece.ext[0] = piece.ext[0].saturating_sub(gone);
            piece.ext[1] = piece.ext[1].saturating_sub(gone).max(piece.ext[0] + 1);
        }
        let n = piece.pts.len();
        if piece.ext[1] < n {
            piece.pts.reverse();
            let gone = cut(&mut piece.pts, n - piece.ext[1]);
            piece.pts.reverse();
            // The tail extension had `n - ext[1]` points; `gone` of them went.
            let tail = (n - piece.ext[1]).saturating_sub(gone);
            piece.ext[1] = piece.pts.len().saturating_sub(tail).max(piece.ext[0] + 1);
        }
    }
    cut_crossings(&mut all, &radius_at, cell);
    let mut columns: Vec<Rails> = Vec::new();
    for (_, fixed, r_med, piece) in &all {
        let fixed_at = |_: Point| fixed.unwrap_or(0.0);
        let width_at: &dyn Fn(Point) -> f64 = if fixed.is_some() { &fixed_at } else { &radius_at };
        if let Some((a, b, closed, width)) = rails(piece, *r_med, width_at, &segs, &outline, cell) {
            out.max_width = out.max_width.max(width);
            columns.push(Rails { a, b, closed });
        }
    }
    trim_ends(&mut columns);
    for c in columns.iter().filter(|c| c.sewable(cell)) {
        let (satin, quads) = c.geometry(Some((&outline, cell)));
        for q in &quads {
            fill_into(&g, std::slice::from_ref(q), &mut cover);
        }
        out.satins.push(satin);
    }
    let covered = (0..cover.len()).filter(|&k| cover[k] && g.inside[k]).count();
    out.coverage = covered as f64 / inside_count as f64;
    let mut widths: Vec<f64> = out
        .satins
        .iter()
        .flat_map(|s| s.rungs.iter().map(|(a, b)| a.distance(*b)))
        .collect();
    widths.sort_by(f64::total_cmp);
    out.typical_width = widths.get(widths.len() / 2).copied().unwrap_or(out.max_width);
    out
}

/// A stroke through a part of the shape along its long axis (for dots).
fn long_axis(g: &Grid, label: &[usize], l: usize, r: f64) -> Raw {
    let cells: Vec<Point> = (0..label.len())
        .filter(|&k| label[k] == l)
        .map(|k| g.center_of(k))
        .collect();
    let n = cells.len() as f64;
    let c = cells.iter().fold(Point::new(0.0, 0.0), |a, &p| a + p) * (1.0 / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in &cells {
        let d = *p - c;
        sxx += d.x * d.x;
        syy += d.y * d.y;
        sxy += d.x * d.y;
    }
    let angle = 0.5 * emb_geom::math::atan2(2.0 * sxy, sxx - syy);
    let u = Point::new(emb_geom::math::cos(angle), emb_geom::math::sin(angle));
    let (lo, hi) = cells
        .iter()
        .map(|p| (*p - c).dot(u))
        .fold((f64::MAX, f64::MIN), |(a, b), t| (a.min(t), b.max(t)));
    let (a, b) = (c + u * lo, c + u * hi);
    let steps = ((hi - lo) / g.cell).ceil().max(2.0) as usize;
    Raw {
        pts: (0..=steps).map(|i| a.lerp(b, i as f64 / steps as f64)).collect(),
        closed: false,
        free: [false, false],
        meets: [false, false],
        r_fix: Some(r),
    }
}

/// The rings of the outline of a glyph, with the length along each ring at
/// each of its points: the rails of the columns are made from them (P21
/// Milestone 1).
struct Rings<'a> {
    rings: &'a [Vec<Point>],
    /// For each ring, the length along it at each point, and its whole
    /// length last.
    at: Vec<Vec<f64>>,
}

/// A point of the outline: the point, its ring and the length along the
/// ring to it.
#[derive(Debug, Clone, Copy)]
struct Foot {
    at: Point,
    ring: usize,
    s: f64,
}

impl<'a> Rings<'a> {
    fn new(rings: &'a [Vec<Point>]) -> Rings<'a> {
        let at = rings
            .iter()
            .map(|r| {
                let mut v = vec![0.0];
                for k in 0..r.len() {
                    v.push(v[k] + r[k].distance(r[(k + 1) % r.len()]));
                }
                v
            })
            .collect();
        Rings { rings, at }
    }

    fn len(&self, ring: usize) -> f64 {
        self.at[ring].last().copied().unwrap_or(0.0)
    }

    /// The nearest point of the outline to `p` (of the segments in the
    /// angle [`FOOT_ALONG_DEG`] of the direction of the middle line, the
    /// normal `dir` turned, whose nearest point is in the angle
    /// [`FOOT_CONE_DEG`] of `dir`), at most `reach` away; with `dir` zero,
    /// of all segments.
    fn nearest(&self, p: Point, dir: Point, reach: f64) -> Option<Foot> {
        self.nearest_with(p, dir, reach, true)
    }

    /// [`Rings::nearest`]; with `along` false, of all the segments whose
    /// nearest point is in the angle of `dir`.
    fn nearest_with(&self, p: Point, dir: Point, reach: f64, along: bool) -> Option<Foot> {
        let cos = emb_geom::math::cos(emb_geom::math::to_radians(FOOT_CONE_DEG));
        let cos_along = emb_geom::math::cos(emb_geom::math::to_radians(FOOT_ALONG_DEG));
        let tangent = dir.perp();
        let mut best: Option<(f64, Foot)> = None;
        for (r, ring) in self.rings.iter().enumerate() {
            let m = ring.len();
            for k in 0..m {
                let (a, b) = (ring[k], ring[(k + 1) % m]);
                let q = p.nearest_on_segment(a, b);
                let v = q - p;
                let d = v.length();
                if d > reach || best.is_some_and(|(bd, _)| d >= bd) {
                    continue;
                }
                if dir != Point::ZERO {
                    if d > 1e-12 && v.dot(dir) < cos * d {
                        continue;
                    }
                    let seg = b - a;
                    if along && seg.dot(tangent).abs() < cos_along * seg.length() {
                        continue;
                    }
                }
                let foot = Foot {
                    at: q,
                    ring: r,
                    s: self.at[r][k] + a.distance(q),
                };
                best = Some((d, foot));
            }
        }
        best.map(|(_, f)| f)
    }

    /// The length along the ring from `a` to `b` the short way: positive
    /// in the direction of the ring. Both are on the same ring.
    fn delta(&self, a: &Foot, b: &Foot) -> f64 {
        let l = self.len(a.ring);
        let mut d = b.s - a.s;
        if d > 0.5 * l {
            d -= l;
        } else if d < -0.5 * l {
            d += l;
        }
        d
    }

    /// The points of the ring strictly between `a` and `b`, the short way,
    /// in order from `a`.
    fn between(&self, a: &Foot, b: &Foot) -> Vec<Point> {
        let (ring, at) = (&self.rings[a.ring], &self.at[a.ring]);
        let (m, l) = (ring.len(), self.len(a.ring));
        let d = self.delta(a, b);
        if m == 0 || l <= 0.0 || d.abs() < 1e-12 {
            return Vec::new();
        }
        // How far point `k` is past `a` in the direction of travel.
        let past = |k: usize| {
            let x = if d > 0.0 { at[k] - a.s } else { a.s - at[k] };
            x.rem_euclid(l)
        };
        let mut ks: Vec<usize> = (0..m)
            .filter(|&k| past(k) > 1e-12 && past(k) < d.abs() - 1e-12)
            .collect();
        ks.sort_by(|&x, &y| past(x).total_cmp(&past(y)));
        ks.into_iter().map(|k| ring[k]).collect()
    }

    /// The points of the ring from `a` to `b` (both on it), the way of the
    /// ring with `forward`, else against it, with `a` first and `b` last.
    /// `None` if that way is longer than `limit`.
    fn path(&self, a: &Foot, b: &Foot, forward: bool, limit: f64) -> Option<Vec<Point>> {
        if a.ring != b.ring {
            return None;
        }
        let (ring, at) = (&self.rings[a.ring], &self.at[a.ring]);
        let l = self.len(a.ring);
        if ring.is_empty() || l <= 0.0 {
            return None;
        }
        let d = if forward { b.s - a.s } else { a.s - b.s }.rem_euclid(l);
        if d > limit {
            return None;
        }
        let past = |k: usize| {
            let x = if forward { at[k] - a.s } else { a.s - at[k] };
            x.rem_euclid(l)
        };
        let mut ks: Vec<usize> = (0..ring.len())
            .filter(|&k| past(k) > 1e-12 && past(k) < d - 1e-12)
            .collect();
        ks.sort_by(|&x, &y| past(x).total_cmp(&past(y)));
        let mut out = vec![a.at];
        out.extend(ks.into_iter().map(|k| ring[k]));
        out.push(b.at);
        Some(out)
    }

    /// The rail goes along the outline from `fa` to `fb` (the feet of the
    /// middle points `ma` and `mb`): they are on one ring, and the outline
    /// between them stays near the column. Else the outline goes away into
    /// another stroke between them (an opening).
    fn follows(&self, fa: &Foot, fb: &Foot, ma: Point, mb: Point, cell: f64) -> bool {
        if fa.ring != fb.ring {
            return false;
        }
        let chord = fa.at.distance(fb.at);
        let arc = self.delta(fa, fb).abs();
        if arc > STUB_ARC * chord + FOLLOW_SLACK * cell {
            return false;
        }
        let h = fa.at.distance(ma).max(fb.at.distance(mb));
        let far = self
            .between(fa, fb)
            .iter()
            .map(|v| v.distance_to_segment(ma, mb))
            .fold(0.0, f64::max);
        let near =
            |arc_max: f64, near_max: f64| arc <= arc_max * chord + FOLLOW_SLACK * cell && far <= near_max * h + cell;
        near(FOLLOW_ARC, FOLLOW_NEAR) || near(STUB_ARC, STUB_NEAR)
    }

    /// The points of the outline between two rail points next to each
    /// other (`p` and `q`, of the pairs with the middles `mp` and `mq`),
    /// if both are on the outline and the rail follows it there: fewer of
    /// them where the outline is almost straight.
    fn along(&self, p: Point, q: Point, mp: Point, mq: Point, cell: f64) -> Vec<Point> {
        let tol = 0.02 * cell;
        let (Some(fa), Some(fb)) = (self.nearest(p, Point::ZERO, tol), self.nearest(q, Point::ZERO, tol)) else {
            return Vec::new();
        };
        if !self.follows(&fa, &fb, mp, mq, cell) {
            return Vec::new();
        }
        thin_line(p, &self.between(&fa, &fb), q, FOLLOW_THIN * cell)
    }
}

/// The points of `mid` (a line from `a` to `b`) that stay when the points
/// nearer than `tol` to the line through the others go (Douglas and
/// Peucker).
fn thin_line(a: Point, mid: &[Point], b: Point, tol: f64) -> Vec<Point> {
    let Some((k, d)) = mid
        .iter()
        .enumerate()
        .map(|(k, p)| (k, p.distance_to_segment(a, b)))
        .max_by(|x, y| x.1.total_cmp(&y.1))
    else {
        return Vec::new();
    };
    if d <= tol {
        return Vec::new();
    }
    let mut out = thin_line(a, &mid[..k], mid[k], tol);
    out.push(mid[k]);
    out.extend(thin_line(mid[k], &mid[k + 1..], b, tol));
    out
}

/// The middle line of a piece, for its rails.
struct Middle<'a> {
    pts: &'a [Point],
    /// The normal at each point (to the side of rail A).
    nrm: &'a [Point],
    /// The points that get no foot (extensions and held points).
    skip: &'a [bool],
    /// The length along the line at each point.
    along: &'a [f64],
    closed: bool,
}

/// The rail of one side of a piece (`sign` 1: rail A, -1: rail B; P21
/// Milestone 1). Each point of the middle line that is not skipped takes
/// the nearest point of the outline on that side (its foot), in the angle
/// [`FOOT_CONE_DEG`] of its normal, at most [`FOOT_REACH`] half-widths
/// away. The rail follows the outline from foot to foot (`Rails::geometry`
/// adds the points of the outline).
///
/// - Where a side passes an opening (a place where another stroke meets or
///   crosses: no foot near enough, or two feet with an outline between
///   them that goes away into the other stroke), the rail goes straight
///   across it, from the foot before to the foot after. A foot that stays
///   at the corner of an opening (as the middle line goes on past it) is
///   in the opening too.
/// - The feet go along the outline one way only: a foot that would go back
///   stays at the foot before it (the inside of a tight curve), so the
///   stitches do not cross.
///
/// `None` for the points that are skipped.
fn side_rail(
    line: &Middle,
    sign: f64,
    outline: &Rings,
    segs: &[(Point, Point)],
    radius_at: &dyn Fn(Point) -> f64,
    cell: f64,
) -> Vec<Option<Point>> {
    let (pts, n) = (line.pts, line.pts.len());
    let mut out: Vec<Option<Point>> = vec![None; n];
    let body: Vec<usize> = (0..n).filter(|&i| !line.skip[i]).collect();
    if body.is_empty() {
        return out;
    }
    let dir = |i: usize| line.nrm[i] * sign;
    let mut feet: Vec<Option<Foot>> = vec![None; n];
    for &i in &body {
        let reach = FOOT_REACH * radius_at(pts[i]) + FOOT_SLACK * cell;
        feet[i] = outline.nearest(pts[i], dir(i), reach);
    }
    // The body points next to each other (positions in `body`), and the
    // last with the first in a ring.
    let count = body.len();
    let mut pairs: Vec<(usize, usize)> = (1..count).map(|q| (q - 1, q)).collect();
    if line.closed && count > 2 {
        pairs.push((count - 1, 0));
    }
    let same = |a: &Option<Foot>, b: &Foot| a.is_some_and(|a| a.at.distance(b.at) <= 1e-3 * cell);
    let mut open: Vec<bool> = body.iter().map(|&i| feet[i].is_none()).collect();
    for &(qa, qb) in &pairs {
        let (i, j) = (body[qa], body[qb]);
        let (Some(fa), Some(fb)) = (feet[i], feet[j]) else {
            continue;
        };
        if same(&feet[i], &fb) || outline.follows(&fa, &fb, pts[i], pts[j], cell) {
            continue;
        }
        // A jump of the foot: the feet that stayed at its two ends (but the
        // first and the last of them) are in the opening.
        let mut q = qa;
        while q > 0 && same(&feet[body[q - 1]], &fa) {
            open[q] = true;
            q -= 1;
        }
        let mut q = qb;
        while q + 1 < count && same(&feet[body[q + 1]], &fb) {
            open[q] = true;
            q += 1;
        }
    }
    // One way: the direction in which the feet go along their rings.
    let mut way = 0.0;
    for &(qa, qb) in &pairs {
        let (i, j) = (body[qa], body[qb]);
        if let (false, false, Some(fa), Some(fb)) = (open[qa], open[qb], feet[i], feet[j])
            && outline.follows(&fa, &fb, pts[i], pts[j], cell)
        {
            way += outline.delta(&fa, &fb);
        }
    }
    if way != 0.0 {
        let mut last: Option<(usize, Foot)> = None;
        // The positions of the feet that stay at `last`.
        let mut stays: Vec<usize> = Vec::new();
        let end_stays = |stays: &mut Vec<usize>, last: Option<(usize, Foot)>, open: &mut Vec<bool>| {
            if let (Some(&q_end), Some((l, lf))) = (stays.last(), last) {
                let h = lf.at.distance(pts[l]).max(cell);
                if (line.along[body[q_end]] - line.along[l]).abs() > STAY_MAX * h {
                    for &q in stays.iter() {
                        open[q] = true;
                    }
                }
            }
            stays.clear();
        };
        for (q, &i) in body.iter().enumerate() {
            let Some(f) = feet[i].filter(|_| !open[q]) else {
                end_stays(&mut stays, last, &mut open);
                last = None;
                continue;
            };
            if let Some((l, lf)) = last
                && lf.ring == f.ring
                && outline.delta(&lf, &f) * way.signum() < -0.25 * cell
                && outline.follows(&lf, &f, pts[l], pts[i], cell)
            {
                feet[i] = Some(lf);
                stays.push(q);
                continue;
            }
            end_stays(&mut stays, last, &mut open);
            last = Some((i, f));
        }
        end_stays(&mut stays, last, &mut open);
    }
    // The feet, and straight lines across the openings.
    let closed_at = |q: usize| !open[q] && feet[body[q]].is_some();
    // The body point nearest to position `q` (before it with `back`) that
    // has a foot.
    let find = |q: usize, back: bool| -> Option<usize> {
        (1..count)
            .map(|k| {
                if line.closed {
                    Some(if back { (q + count - k) % count } else { (q + k) % count })
                } else if back {
                    q.checked_sub(k)
                } else {
                    Some(q + k).filter(|&x| x < count)
                }
            })
            .take_while(Option::is_some)
            .flatten()
            .find(|&k| closed_at(k))
    };
    for q in 0..count {
        let i = body[q];
        if closed_at(q) {
            out[i] = feet[i].map(|f| f.at);
            continue;
        }
        let d = dir(i);
        // The width of the side along the normal at a foot.
        let width = |k: usize| {
            let j = body[k];
            feet[j].map_or(0.0, |f| (f.at - pts[j]).dot(dir(j))).max(0.25 * cell)
        };
        let far = 4.0 * radius_at(pts[i]) + 2.0 * cell;
        let mut p = match (find(q, true), find(q, false)) {
            (Some(ka), Some(kb)) => {
                let fa = feet[body[ka]].map_or(pts[i], |f| f.at);
                let fb = feet[body[kb]].map_or(pts[i], |f| f.at);
                match Point::crossing(pts[i], pts[i] + d * far, fa, fb) {
                    Some((u, _)) if u > 0.0 => pts[i] + d * (u * far),
                    _ => {
                        let (la, lb) = (line.along[body[ka]], line.along[body[kb]]);
                        let t = if (lb - la).abs() > 1e-12 {
                            ((line.along[i] - la) / (lb - la)).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };
                        fa.lerp(fb, t)
                    }
                }
            }
            (Some(k), None) | (None, Some(k)) => pts[i] + d * width(k),
            (None, None) => pts[i] + d * radius_at(pts[i]),
        };
        // Not past the outline.
        let len = (p - pts[i]).length();
        if let Some(t) = ray(pts[i], d, len, segs).filter(|t| *t < len) {
            p = pts[i] + d * t;
        }
        out[i] = Some(p);
    }
    out
}

/// The mitre of a corner of a stroke at the point `q` of its middle line
/// (P21 Milestone 2): the line from the inner corner of the outline to its
/// outer corner. The two pieces of the stroke end on it, so the corner has
/// stitches to its point and the pieces do not lie on each other. `u1` and
/// `u2`: the directions of the middle line into the corner and out of it;
/// `r`: the half-width of the stroke. The inner corner is the nearest point
/// of the outline on the inner side of the turn; the outer corner is a
/// corner of the outline on the outer side, near the line that halves the
/// turn (the point of an "N", a "V"), else the point of the outline on that
/// line (a flat apex of an "A", a round outer side). `None` if they are not
/// there or the line is not across the stroke.
fn mitre_at(
    q: Point,
    u1: Point,
    u2: Point,
    r: f64,
    outline: &Rings,
    corners: &[Corner],
    segs: &[(Point, Point)],
) -> Option<(Point, Point)> {
    let inward = (u2 - u1).normalized()?;
    let out = inward * -1.0;
    let reach = MITRE_REACH * r;
    let inner = outline.nearest_with(q, inward, reach, false)?.at;
    let cos = emb_geom::math::cos(emb_geom::math::to_radians(MITRE_OUTER_DEG));
    let along_out = |c: Point| (c - q).normalized().map_or(-1.0, |v| v.dot(out));
    let outer = match corners
        .iter()
        .map(|c| c.0)
        .filter(|&c| c.distance(q) <= reach && along_out(c) >= cos)
        .max_by(|&x, &y| along_out(x).total_cmp(&along_out(y)))
    {
        Some(c) => c,
        // A round outer side: the point on the line that halves the turn.
        // A corner farther out (a sharp point) has no mitre.
        None if corners
            .iter()
            .any(|c| c.0.distance(q) <= 2.0 * reach && along_out(c.0) >= cos) =>
        {
            return None;
        }
        None => ray(q, out, reach, segs).map(|t| q + out * t)?,
    };
    let len = inner.distance(outer);
    // The mitre goes across both pieces, not along one of them.
    let d = (outer - inner).normalized()?;
    let across = emb_geom::math::cos(emb_geom::math::to_radians(MITRE_ACROSS_DEG));
    (len > 0.8 * r
        && (inner - q).dot(inward) > 0.0
        && (outer - q).dot(out) > 0.0
        && d.dot(u1).abs() <= across
        && d.dot(u2).abs() <= across)
        .then_some((inner, outer))
}

/// Ends the rails `a` and `b` of a column on the mitre `m` (at the start
/// with `end` 0, at the end with 1): the pairs past the line of the mitre
/// go, and the two points of the mitre are the last pair (P21 Milestone 2).
fn close_mitre(a: &mut Vec<Point>, b: &mut Vec<Point>, end: usize, m: (Point, Point)) {
    let n = a.len();
    if n < 2 {
        return;
    }
    let (m1, m2) = m;
    let side = |p: Point| (m2 - m1).cross(p - m1);
    // The side of the column: a pair back from the end by the length of
    // the mitre (the middle of a column that curves round, a "G", can be
    // on the other side of the line of the mitre).
    let mid = m1.midpoint(m2);
    let len = m1.distance(m2);
    let order: Vec<usize> = if end == 1 {
        (0..n).rev().collect()
    } else {
        (0..n).collect()
    };
    let at = order
        .iter()
        .copied()
        .find(|&i| a[i].midpoint(b[i]).distance(mid) >= len)
        .unwrap_or(order[n - 1]);
    let inside = side(a[at].midpoint(b[at])).signum();
    if inside == 0.0 {
        return;
    }
    let keep: Vec<bool> = (0..n)
        .map(|i| side(a[i]) * inside > 0.0 && side(b[i]) * inside > 0.0)
        .collect();
    // The pair that stays nearest the mitre; rail A takes the point of the
    // mitre on its side (as the end pair of before).
    let k = if end == 1 {
        (0..n).rev().find(|&i| keep[i]).unwrap_or(0)
    } else {
        (0..n).find(|&i| keep[i]).unwrap_or(n - 1)
    };
    // A mitre that would take more than half of the column is not the
    // corner of this column.
    let gone = if end == 1 { n - 1 - k } else { k };
    if 2 * gone > n {
        return;
    }
    let last = if end == 1 { n - 1 } else { 0 };
    let (ma, mb) = if a[last].distance(m1) + b[last].distance(m2) <= a[last].distance(m2) + b[last].distance(m1) {
        (m1, m2)
    } else {
        (m2, m1)
    };
    if end == 1 {
        a.truncate(k + 1);
        b.truncate(k + 1);
        a.push(ma);
        b.push(mb);
    } else {
        a.drain(..k);
        b.drain(..k);
        a.insert(0, ma);
        b.insert(0, mb);
    }
}

/// The pairs of a free end of a column (the end of a pen stroke; P21
/// Milestone 2), from the pair (`a_k`, `b_k`) on the outline to the end, in
/// the direction `dir` of the stroke. The outline from `a_k` round the end
/// to `b_k` is the end: its two sides go to the front of the stroke (its
/// points farthest along `dir`), and the pairs are the points of the two
/// sides at the same places along `dir`. So the stitches stay across the
/// stroke and end on the outline: on the end edge of a square end (the
/// last pair is its two corners), on the slant of an end cut at a slant,
/// round a round end. Where the two sides come nearer than [`END_TIP`] of
/// the width `w` (a pointed or a round end), the column ends. `None` if the
/// two points are not on one ring, or the outline round the end is longer
/// than [`END_LOOP_MAX`] widths (the end goes on into another stroke), or
/// a side goes back against `dir` (an end that curves back).
fn free_end(
    a_k: Point,
    b_k: Point,
    dir: Point,
    outline: &Rings,
    w: f64,
    cell: f64,
    step: f64,
) -> Option<Vec<(Point, Point)>> {
    let tol = 0.05 * cell;
    let fa = outline.nearest(a_k, Point::ZERO, tol)?;
    let fb = outline.nearest(b_k, Point::ZERO, tol)?;
    if fa.ring != fb.ring {
        return None;
    }
    let o = a_k.midpoint(b_k);
    let proj = |p: Point| (p - o).dot(dir);
    // The two ways round the ring from `fa` to `fb`: the end is the shorter
    // of those that go to the front.
    let path = [true, false]
        .into_iter()
        .filter_map(|forward| outline.path(&fa, &fb, forward, END_LOOP_MAX * w))
        .filter(|p| p.iter().map(|q| proj(*q)).fold(f64::MIN, f64::max) > 0.2 * w)
        .min_by(|x, y| length(x).total_cmp(&length(y)))?;
    let ps: Vec<f64> = path.iter().map(|p| proj(*p)).collect();
    let top = ps.iter().copied().fold(f64::MIN, f64::max);
    // The front: an end edge (straight), else the farthest point of a
    // round end.
    let front = |flat: f64| -> Option<(usize, usize)> {
        Some((
            ps.iter().position(|t| *t >= top - flat)?,
            ps.iter().rposition(|t| *t >= top - flat)?,
        ))
    };
    let (mut ia, mut ib) = front(END_FLAT * w)?;
    if path[ia..=ib]
        .iter()
        .any(|p| p.distance_to_segment(path[ia], path[ib]) > END_STRAIGHT * w)
    {
        (ia, ib) = front(0.01 * w)?;
    }
    let side_a: Vec<Point> = path[..=ia].to_vec();
    let side_b: Vec<Point> = path[ib..].iter().rev().copied().collect();
    // Each side goes to the front (a little back is a bump of the outline).
    let goes_front = |side: &[Point]| {
        let mut most = f64::MIN;
        side.iter().all(|p| {
            most = most.max(proj(*p));
            proj(*p) >= most - END_BACK_SLACK * w
        })
    };
    if !goes_front(&side_a) || !goes_front(&side_b) {
        return None;
    }
    // The point of a side where it first comes to `t` along `dir`.
    let at = |side: &[Point], t: f64| -> Point {
        let Some(&first) = side.first() else { return o };
        if proj(first) >= t {
            return first;
        }
        for s in side.windows(2) {
            let (p0, p1) = (proj(s[0]), proj(s[1]));
            if p1 >= t && p0 < t {
                return s[0].lerp(s[1], (t - p0) / (p1 - p0));
            }
        }
        *side.last().unwrap_or(&o)
    };
    let (ea, eb) = (*side_a.last()?, *side_b.last()?);
    let t0 = proj(a_k).max(proj(b_k));
    let t1 = proj(ea).min(proj(eb));
    // The places of the pairs: even steps, and the points of the outline
    // (the near corner of an end at a slant is one of them).
    // Finer steps than the column: the end has its shape in a short length.
    let step = step.min(END_STEP * w);
    let m = ((t1 - t0) / step.max(1e-9)).ceil().max(1.0) as usize;
    let mut ts: Vec<f64> = (1..m).map(|q| t0 + (t1 - t0) * q as f64 / m as f64).collect();
    ts.extend(
        side_a
            .iter()
            .chain(&side_b)
            .map(|p| proj(*p))
            .filter(|t| *t > t0 && *t < t1),
    );
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|x, y| (*x - *y).abs() < 0.1 * step);
    let mut pairs: Vec<(Point, Point)> = ts.iter().map(|&t| (at(&side_a, t), at(&side_b, t))).collect();
    pairs.push((ea, eb));
    // No pair on the pair before it (or on the first pair).
    let mut last = (a_k, b_k);
    pairs.retain(|&(p, q)| {
        let new = p.distance(last.0) + q.distance(last.1) > 1e-6 * w;
        if new {
            last = (p, q);
        }
        new
    });
    if let Some(narrow) = pairs.iter().position(|(p, q)| p.distance(*q) < END_TIP * w) {
        pairs.truncate(narrow);
    }
    Some(pairs)
}

/// The satin column of a piece, its widest width and the quads it covers.
/// The points of an extension `ext` (from its first point, on the middle
/// line) up to where it leaves the outline. The walk on the grid can stop a
/// part of a cell past the outline, and the width of a point out of the
/// outline is not held at the outline: the column went out of the letter
/// (P21: the flag of "l" in Bree Serif).
fn in_outline(ext: &[Point], outline: &Rings, segs: &[(Point, Point)]) -> Vec<Point> {
    let inside = |p: Point| outline.rings.iter().filter(|r| ring_contains(r, p)).count() % 2 == 1;
    let mut out: Vec<Point> = Vec::with_capacity(ext.len());
    for &p in ext {
        let Some(&prev) = out.last() else {
            out.push(p);
            continue;
        };
        if inside(p) {
            out.push(p);
            continue;
        }
        let u = segs
            .iter()
            .filter_map(|&(c, d)| Point::crossing(prev, p, c, d).map(|(u, _)| u))
            .fold(1.0f64, f64::min);
        let q = prev.lerp(p, u);
        if q.distance(prev) > 1e-9 {
            out.push(q);
        }
        break;
    }
    out
}

fn rails(
    piece: &Piece,
    r_med: f64,
    radius_at: &dyn Fn(Point) -> f64,
    segs: &[(Point, Point)],
    outline: &Rings,
    cell: f64,
) -> Option<(Vec<Point>, Vec<Point>, bool, f64)> {
    // A short stroke (a dot) gets more points, so that its round sides
    // are round.
    let step = (0.7 * r_med).min(length(&piece.pts) / 12.0).max(cell);
    // Resample the middle part and the extensions apart, so the extension
    // flags stay right.
    let mid = &piece.pts[piece.ext[0]..piece.ext[1].max(piece.ext[0])];
    if mid.len() < 2 && piece.pts.len() < 2 {
        return None;
    }
    let mut pts = Vec::new();
    let mut is_ext = Vec::new();
    if piece.ext[0] > 0 {
        let mut head = resample(&piece.pts[..=piece.ext[0]], false, step);
        head.reverse();
        let mut head = in_outline(&head, outline, segs);
        head.reverse();
        let k = head.len().saturating_sub(1);
        pts.extend_from_slice(&head[..k]);
        is_ext.extend(std::iter::repeat_n(true, k));
    }
    let body = if mid.len() >= 2 {
        resample(mid, piece.closed, step)
    } else {
        mid.to_vec()
    };
    let nb = body.len();
    pts.extend_from_slice(&body);
    is_ext.extend(std::iter::repeat_n(false, nb));
    if piece.ext[1] < piece.pts.len() {
        let tail = in_outline(&resample(&piece.pts[piece.ext[1] - 1..], false, step), outline, segs);
        pts.extend_from_slice(&tail[1.min(tail.len())..]);
        is_ext.extend(std::iter::repeat_n(true, tail.len().saturating_sub(1)));
    }
    let n = pts.len();
    if n < 2 {
        return None;
    }
    // Points near an end at a junction take the width of the first point
    // past the hold length (as extensions do).
    let mut along = vec![0.0; n];
    for i in 1..n {
        along[i] = along[i - 1] + pts[i - 1].distance(pts[i]);
    }
    let total = along[n - 1];
    let held: Vec<bool> = (0..n)
        .map(|i| along[i] < piece.hold[0] || total - along[i] < piece.hold[1])
        .collect();
    if (0..n).all(|i| is_ext[i] || held[i]) {
        // A short stroke: measure at the middle point only.
        let mid = n / 2;
        for (i, e) in is_ext.iter_mut().enumerate() {
            *e = i != mid;
        }
    } else {
        for i in 0..n {
            is_ext[i] |= held[i];
        }
    }
    let closed = piece.closed && n >= 3;
    let tangent = |i: usize| -> Point {
        let (a, b) = if closed {
            (pts[(i + n - 1) % n], pts[(i + 1) % n])
        } else {
            (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)])
        };
        (b - a).normalized().unwrap_or(Point::new(1.0, 0.0))
    };
    let cap = 1.3 * r_med;
    let nrm: Vec<Point> = (0..n).map(|i| tangent(i).perp()).collect();
    // The rails of the middle points are the outline (P21 Milestone 1).
    let line = Middle {
        pts: &pts,
        nrm: &nrm,
        skip: &is_ext,
        along: &along,
        closed,
    };
    let capped = |p: Point| radius_at(p).min(cap);
    let (rail_a, rail_b) = (
        side_rail(&line, 1.0, outline, segs, &capped, cell),
        side_rail(&line, -1.0, outline, segs, &capped, cell),
    );
    // The width of each middle point on each side, along its normal.
    let mut half: Vec<(f64, f64)> = vec![(0.0, 0.0); n];
    for i in 0..n {
        if let (Some(pa), Some(pb)) = (rail_a[i], rail_b[i]) {
            half[i] = (
                (pa - pts[i]).dot(nrm[i]).max(0.25 * cell),
                (pts[i] - pb).dot(nrm[i]).max(0.25 * cell),
            );
        }
    }
    // Extensions take the width of the nearest middle point.
    let first = (0..n).find(|&i| !is_ext[i])?;
    let last = (0..n).rev().find(|&i| !is_ext[i]).unwrap_or(first);
    for i in 0..n {
        if is_ext[i] {
            // The held width, but not past the outline (a pointed end narrows).
            let (hp, hm) = if i < first { half[first] } else { half[last] };
            let hp = ray(pts[i], nrm[i], hp, segs).unwrap_or(hp);
            let hm = ray(pts[i], -nrm[i], hm, segs).unwrap_or(hm);
            half[i] = (hp.max(0.25 * cell), hm.max(0.25 * cell));
        }
    }
    let mut a: Vec<Point> = (0..n)
        .map(|i| rail_a[i].unwrap_or(pts[i] + nrm[i] * half[i].0))
        .collect();
    let mut b: Vec<Point> = (0..n)
        .map(|i| rail_b[i].unwrap_or(pts[i] - nrm[i] * half[i].1))
        .collect();
    let width = (0..n).map(|i| a[i].distance(b[i])).fold(0.0, f64::max);
    // At a free end, each rail follows its side of the outline to the
    // corner of the end: an end that the font cuts at a slant (or gives a
    // shape) is covered to its corners. The middle line bends to the sharp
    // corner of such an end, so the rails start from a pair farther back.
    if !closed {
        let w = 2.0 * r_med;
        for end in [1, 0] {
            // A cut of a corner: the mitre (P21 Milestone 2).
            if let Some(m) = piece.mitre[end] {
                close_mitre(&mut a, &mut b, end, m);
                continue;
            }
            if !piece.free[end] {
                continue;
            }
            let tip = if end == 0 { 0 } else { n - 1 };
            let from_tip = |i: usize| (along[i] - along[tip]).abs();
            let order: Vec<usize> = if end == 0 {
                (0..n).collect()
            } else {
                (0..n).rev().collect()
            };
            // The outline round the end, from a pair a little back from it
            // (the middle line bends to a corner of the end), or from the
            // middle of a short column (P21 Milestone 2).
            let far = order
                .iter()
                .filter(|&&i| !is_ext[i])
                .map(|&i| from_tip(i))
                .fold(0.0, f64::max);
            let back = (END_BACK * w).min(0.5 * far);
            let free = order
                .iter()
                .find(|&&i| !is_ext[i] && from_tip(i) >= back - 1e-9)
                .filter(|&&k| k < a.len())
                .and_then(|&k| {
                    // The direction of the stroke: square to the pair (the
                    // middle line bends at the end).
                    let across = (b[k] - a[k]).normalized()?.perp();
                    let dir = if across.dot(pts[tip] - pts[k]) >= 0.0 {
                        across
                    } else {
                        across * -1.0
                    };
                    free_end(a[k], b[k], dir, outline, w, cell, step).map(|pairs| (k, pairs))
                });
            if let Some((k, pairs)) = free {
                if end == 1 {
                    a.truncate(k + 1);
                    b.truncate(k + 1);
                    a.extend(pairs.iter().map(|p| p.0));
                    b.extend(pairs.iter().map(|p| p.1));
                } else {
                    let mut na: Vec<Point> = pairs.iter().rev().map(|p| p.0).collect();
                    let mut nb: Vec<Point> = pairs.iter().rev().map(|p| p.1).collect();
                    na.extend_from_slice(&a[k..]);
                    nb.extend_from_slice(&b[k..]);
                    a = na;
                    b = nb;
                }
                continue;
            }
            // Else the sides of before (traced to the corners of the end).
            if total < 2.0 * END_BACK * w + w {
                continue;
            }
            let Some(&k) = order
                .iter()
                .find(|&&i| !is_ext[i] && from_tip(i) >= END_BACK * w && i < a.len())
            else {
                continue;
            };
            let Some(dir) = (pts[tip] - pts[k]).normalized() else {
                continue;
            };
            let reach = 1.5 * (from_tip(k) + w);
            let (Some(path_a), Some(path_b)) = (
                trace_side(outline.rings, a[k], dir, reach, w),
                trace_side(outline.rings, b[k], dir, reach, w),
            ) else {
                continue;
            };
            // The corners of an end are near the place where the middle
            // line meets the outline (P21 Milestone 1). A side that goes
            // farther goes along another stroke.
            if path_a
                .iter()
                .chain(&path_b)
                .any(|p| p.distance(pts[tip]) > END_NEAR_TIP * w)
            {
                continue;
            }
            // Pairs along the two paths, at the same parts of their lengths.
            let (la, lb) = (length(&path_a), length(&path_b));
            let m = ((la.max(lb) / step).ceil() as usize).max(1);
            let mut pairs: Vec<(Point, Point)> = (1..=m)
                .map(|q| {
                    let f = q as f64 / m as f64;
                    (point_at(&path_a, f * la), point_at(&path_b, f * lb))
                })
                .collect();
            // Where the two paths come to one point (a pointed end), the
            // stitches across would be too short: the column ends where it
            // is still `END_MIN_WIDTH` of the stroke wide.
            if let Some(narrow) = pairs.iter().position(|(p, q)| p.distance(*q) < END_MIN_WIDTH * w) {
                pairs.truncate(narrow);
            }
            if pairs.is_empty() {
                continue;
            }
            if end == 1 {
                a.truncate(k + 1);
                b.truncate(k + 1);
                a.extend(pairs.iter().map(|p| p.0));
                b.extend(pairs.iter().map(|p| p.1));
            } else {
                let mut na: Vec<Point> = pairs.iter().rev().map(|p| p.0).collect();
                let mut nb: Vec<Point> = pairs.iter().rev().map(|p| p.1).collect();
                na.extend_from_slice(&a[k..]);
                nb.extend_from_slice(&b[k..]);
                a = na;
                b = nb;
            }
        }
    }
    Some((a, b, closed, width))
}

/// The path along the outline from `from` (a point of a rail, on the
/// outline) in the direction `dir` (along the stroke, to its end), to the
/// corner where the outline turns from the side of the stroke: the first
/// place where it goes more than `END_TURN_DEG` off the direction of the
/// side, at most `reach` long.
/// `None` if `from` is not on the outline or the outline there does not go
/// along the stroke, or if the side goes on along the stroke for `reach`
/// with no corner (it goes into another stroke).
fn trace_side(rings: &[Vec<Point>], from: Point, dir: Point, reach: f64, w: f64) -> Option<Vec<Point>> {
    // The nearest segment of the outline.
    let mut best: Option<(usize, usize, Point, f64)> = None;
    for (r, ring) in rings.iter().enumerate() {
        let n = ring.len();
        for j in 0..n {
            let q = from.nearest_on_segment(ring[j], ring[(j + 1) % n]);
            let d = q.distance(from);
            if best.is_none_or(|b| d < b.3) {
                best = Some((r, j, q, d));
            }
        }
    }
    let (r, j, start, d) = best?;
    if d > 0.15 * w {
        return None;
    }
    let ring = &rings[r];
    let n = ring.len();
    let seg = ring[(j + 1) % n] - ring[j];
    let side = seg.normalized()?;
    // Along the ring in the direction of the end of the stroke.
    let forward = side.dot(dir) >= 0.0;
    let side = if forward { side } else { side * -1.0 };
    if side.dot(dir) < 0.5 {
        return None;
    }
    let max_cos = emb_geom::math::cos(emb_geom::math::to_radians(END_TURN_DEG));
    let mut path = vec![start];
    let mut walked = 0.0;
    let mut v = if forward { (j + 1) % n } else { j };
    for _ in 0..n {
        let c = path[path.len() - 1];
        let next = ring[v];
        let Some(s) = (next - c).normalized() else {
            v = if forward { (v + 1) % n } else { (v + n - 1) % n };
            continue;
        };
        if s.dot(side) < max_cos {
            break;
        }
        let step = c.distance(next);
        if walked + step > reach {
            // The outline goes on along the stroke past its end: into
            // another stroke, not to a corner of the end (P21 Milestone 1).
            return None;
        }
        walked += step;
        path.push(next);
        v = if forward { (v + 1) % n } else { (v + n - 1) % n };
    }
    (path.len() >= 2).then_some(path)
}

/// The point at the length `at` along a line.
fn point_at(line: &[Point], at: f64) -> Point {
    let mut left = at;
    for w in line.windows(2) {
        let d = w[0].distance(w[1]);
        if left <= d {
            return w[0].lerp(w[1], if d > 0.0 { left / d } else { 0.0 });
        }
        left -= d;
    }
    line[line.len() - 1]
}

/// Cuts back the ends of columns that lie in other columns again
/// ([`trim_ends`]), after a step that makes ends longer (the ends that go
/// on to the outline, `glyph::tune_ends`) or adds columns (the second pass
/// of the stroke finder). A closed column stays as it is. `rings`: the
/// outline of the glyph, so that the rails of a column that changes follow
/// it again; `cell`: the cell of the grid of the stroke finder.
pub fn trim_column_ends(satins: &mut Vec<SatinGeometry>, rings: &[Vec<Point>], cell: f64) {
    let mut columns: Vec<Rails> = satins
        .iter()
        .map(|g| {
            let closed = g.rail_a.is_closed();
            let total: f64 = crate::quality::column_widths(g, f64::INFINITY)
                .iter()
                .map(|(_, l)| l)
                .sum();
            let pairs = if closed || total.is_nan() || total <= 0.0 {
                Vec::new()
            } else {
                crate::quality::column_pairs(g, total / 80.0)
            };
            Rails {
                a: pairs.iter().map(|p| p.0).collect(),
                b: pairs.iter().map(|p| p.1).collect(),
                closed,
            }
        })
        .collect();
    // Closed columns are in the others as their rails.
    for (c, g) in columns.iter_mut().zip(satins.iter()) {
        if c.closed {
            let flat = |p: &Path| -> Vec<Point> { p.flatten(1e-3).into_iter().flat_map(|l| l.points).collect() };
            c.a = flat(&g.rail_a);
            c.b = flat(&g.rail_b);
        }
    }
    let before: Vec<(Vec<Point>, Vec<Point>)> = columns.iter().map(|c| (c.a.clone(), c.b.clone())).collect();
    trim_ends(&mut columns);
    let outline = Rings::new(rings);
    let mut out = Vec::new();
    for ((c, g), (a, b)) in columns.iter().zip(satins.iter()).zip(before) {
        // A column that did not change keeps its rails and rungs.
        if c.closed || (c.a == a && c.b == b) {
            out.push(g.clone());
        } else if c.sewable(cell) {
            out.push(c.geometry(Some((&outline, cell))).0);
        }
    }
    *satins = out;
}

/// The two rails of a column: the points of rail A and of rail B, in pairs
/// across the column.
struct Rails {
    a: Vec<Point>,
    b: Vec<Point>,
    closed: bool,
}

impl Rails {
    /// The column has two pairs or more and some length on each rail. A
    /// cut end can leave a column of two pairs with one rail at one point
    /// (P21 Milestone 9: the apex of Varsity "a"), which has no stitch
    /// across it and that an Ink/Stitch file cannot keep.
    fn sewable(&self, cell: f64) -> bool {
        let len = |v: &[Point]| v.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
        self.a.len() >= 2 && (self.closed || (len(&self.a) > 0.1 * cell && len(&self.b) > 0.1 * cell))
    }

    /// The satin column and the quads that it covers. With `outline` (the
    /// rings and the cell of the grid), each rail follows the outline
    /// between two of its points that are on it (P21 Milestone 1).
    fn geometry(&self, outline: Option<(&Rings, f64)>) -> (SatinGeometry, Vec<Vec<Point>>) {
        let (pa, pb, closed) = (&self.a, &self.b, self.closed);
        let n = pa.len();
        let m = if closed { n } else { n - 1 };
        // The points of the outline after each point of a rail.
        let extra = |side: &[Point]| -> Vec<Vec<Point>> {
            (0..m)
                .map(|i| {
                    let j = (i + 1) % n;
                    outline.map_or(Vec::new(), |(o, cell)| {
                        o.along(side[i], side[j], pa[i].midpoint(pb[i]), pa[j].midpoint(pb[j]), cell)
                    })
                })
                .collect()
        };
        let (ea, eb) = (extra(pa), extra(pb));
        let mut quads = Vec::new();
        for i in 0..m {
            let j = (i + 1) % n;
            let mut q = vec![pa[i]];
            q.extend(&ea[i]);
            q.extend([pa[j], pb[j]]);
            q.extend(eb[i].iter().rev());
            q.push(pb[i]);
            quads.push(q);
        }
        let full = |side: &[Point], e: &[Vec<Point>]| -> Vec<Point> {
            let mut v = Vec::new();
            for (i, &p) in side.iter().enumerate() {
                v.push(p);
                if let Some(x) = e.get(i) {
                    v.extend(x);
                }
            }
            v
        };
        let (a, b) = (full(pa, &ea), full(pb, &eb));
        let path = |p: &[Point]| {
            if closed {
                Path::polygon(p)
            } else {
                let mut path = Path::new().move_to(p[0]);
                for &q in &p[1..] {
                    path = path.line_to(q);
                }
                path
            }
        };
        // A rung at every second point keeps the stitches across the stroke.
        let rungs = (1..n.saturating_sub(1)).step_by(2).map(|i| (pa[i], pb[i])).collect();
        (
            SatinGeometry {
                rail_a: path(&a),
                rail_b: path(&b),
                rungs,
            },
            quads,
        )
    }

    /// The part of an open column from [`SELF_GAP`] times its middle width
    /// from its start (along its middle) to its end. `None` if it has fewer
    /// than two pairs.
    fn far_part(&self) -> Option<Rails> {
        let n = self.a.len();
        if self.closed || n < 2 {
            return None;
        }
        let mut widths: Vec<f64> = (0..n).map(|i| self.a[i].distance(self.b[i])).collect();
        widths.sort_by(f64::total_cmp);
        let width = widths[n / 2];
        let mut along = 0.0;
        for i in 1..n {
            along += self.a[i - 1]
                .midpoint(self.b[i - 1])
                .distance(self.a[i].midpoint(self.b[i]));
            if along >= SELF_GAP * width {
                return (n - i >= 2).then(|| Rails {
                    a: self.a[i..].to_vec(),
                    b: self.b[i..].to_vec(),
                    closed: false,
                });
            }
        }
        None
    }

    /// The pair of points at `u` along the column (a pair index and a part
    /// of the way to the next pair).
    fn at(&self, u: f64) -> (Point, Point) {
        let n = self.a.len();
        let i = (u.floor().max(0.0) as usize).min(n - 1);
        let j = (i + 1).min(n - 1);
        let t = (u - i as f64).clamp(0.0, 1.0);
        (self.a[i].lerp(self.a[j], t), self.b[i].lerp(self.b[j], t))
    }

    /// `true` if `p` is in the column, at least `depth` from its edge. An
    /// open column is the quads between its pairs: a column that goes
    /// round and over itself (the loop of あ) has no one ring.
    fn holds(&self, p: Point, depth: f64) -> bool {
        let inside = if self.closed {
            ring_contains(&self.a, p) != ring_contains(&self.b, p)
        } else {
            (0..self.a.len().saturating_sub(1)).any(|i| {
                let quad = [self.a[i], self.a[i + 1], self.b[i + 1], self.b[i]];
                ring_contains(&quad, p)
            })
        };
        if !inside {
            return false;
        }
        let near = |line: &[Point], closed: bool| -> f64 {
            let n = line.len();
            let m = if closed { n } else { n - 1 };
            (0..m)
                .map(|i| p.distance_to_segment(line[i], line[(i + 1) % n]))
                .fold(f64::MAX, f64::min)
        };
        let mut edge = near(&self.a, self.closed).min(near(&self.b, self.closed));
        if !self.closed {
            let (na, nb) = (self.a.len(), self.b.len());
            edge = edge
                .min(p.distance_to_segment(self.a[0], self.b[0]))
                .min(p.distance_to_segment(self.a[na - 1], self.b[nb - 1]));
        }
        edge > depth
    }
}

/// Cuts back the ends of the columns that lie in other columns (P20
/// Milestone 3): the end of a stroke at a corner or a junction, or an end
/// that was extended through another stroke. A pair of points across the
/// column is in the others if its two points and its middle are in them,
/// more than [`END_OVERLAP`] of the width from their edge; the end moves to
/// the first pair that is not. The columns in order: a column that is cut
/// does not cut the one after it at the same place (a corner stays
/// covered). A column that lies all in others goes.
fn trim_ends(columns: &mut [Rails]) {
    for k in 0..columns.len() {
        if columns[k].closed || columns[k].a.len() < 2 {
            continue;
        }
        let (before, rest) = columns.split_at_mut(k);
        let (this, after) = rest.split_first_mut().expect("column k");
        let others: Vec<&Rails> = before.iter().chain(after.iter()).filter(|c| c.a.len() >= 2).collect();
        for end in 0..2 {
            if end == 1 {
                this.a.reverse();
                this.b.reverse();
            }
            // The part of this column far from the end is an other column
            // too: a stroke that goes round and ends on itself (the bowl of
            // a script "P"; P21 Milestone 1).
            let far = this.far_part();
            if others.is_empty() && far.is_none() {
                if end == 1 {
                    this.a.reverse();
                    this.b.reverse();
                }
                continue;
            }
            // Points a little in from the sides: at a corner, the sides of the
            // two columns are the same edge of the letter.
            let in_others = |p: Point, depth: f64| {
                others.iter().any(|o| o.holds(p, depth)) || far.as_ref().is_some_and(|f| f.holds(p, depth))
            };
            let covered = |pa: Point, pb: Point| {
                let depth = END_OVERLAP * pa.distance(pb);
                [pa.lerp(pb, SIDE_IN), pa.lerp(pb, 0.5), pa.lerp(pb, 1.0 - SIDE_IN)]
                    .iter()
                    .all(|&p| in_others(p, depth))
            };
            // One side (0: rail A, 1: rail B) of the pair at `u` is in the others.
            let side_in = |r: &Rails, u: f64, side: usize| {
                let (pa, pb) = r.at(u);
                let t = if side == 0 { SIDE_IN } else { 1.0 - SIDE_IN };
                in_others(pa.lerp(pb, t), END_OVERLAP * pa.distance(pb))
            };
            let n = this.a.len();
            // The pairs near the end can be out of the others: at a corner
            // the end of a column is on the side of the other column.
            let mut along = 0.0;
            let mut any = false;
            let mut first_in = 0;
            let mut s = n;
            for i in 0..n {
                if i > 0 {
                    along += this.a[i - 1]
                        .lerp(this.b[i - 1], 0.5)
                        .distance(this.a[i].lerp(this.b[i], 0.5));
                }
                if covered(this.a[i], this.b[i]) {
                    if !any {
                        first_in = i;
                    }
                    any = true;
                } else if any || along >= END_LEAD * this.a[i].distance(this.b[i]) {
                    s = i;
                    break;
                }
            }
            // A column in the others from the first pair to the last goes.
            // A short column that goes into the others only at its far end
            // (the end of a short stroke that a crossing cut, the strokes
            // of 草) stays: its pairs before are not a lead into them (P21).
            if !any || (s == n && first_in > 0) {
                if end == 1 {
                    this.a.reverse();
                    this.b.reverse();
                }
                continue;
            }
            if s == n {
                this.a.clear();
                this.b.clear();
                break;
            }
            if s > 0 {
                // The place between the last pair in the others and the
                // first that is not.
                let (mut lo, mut hi) = (0.0, 1.0);
                for _ in 0..12 {
                    let t = 0.5 * (lo + hi);
                    if covered(this.a[s - 1].lerp(this.a[s], t), this.b[s - 1].lerp(this.b[s], t)) {
                        lo = t;
                    } else {
                        hi = t;
                    }
                }
                let u = (s - 1) as f64 + lo;
                // Each side goes on to where it comes out of the others: the
                // end follows an edge that is not square to the column (a
                // stroke that meets another at a slant).
                let width = {
                    let (pa, pb) = this.at(u);
                    pa.distance(pb)
                };
                let mut ends = [u; 2];
                for (side, end_u) in ends.iter_mut().enumerate() {
                    let mut j = u.floor() as usize + 1;
                    while j < n && side_in(this, j as f64, side) {
                        j += 1;
                    }
                    let (mut lo, mut hi) = (u.max((j - 1) as f64), j.min(n - 1) as f64);
                    for _ in 0..12 {
                        let t = 0.5 * (lo + hi);
                        if side_in(this, t, side) {
                            lo = t;
                        } else {
                            hi = t;
                        }
                    }
                    // Not more slant than the width of the column.
                    let mid = |v: f64| {
                        let (pa, pb) = this.at(v);
                        pa.lerp(pb, 0.5)
                    };
                    while lo > u && mid(lo).distance(mid(u)) > width {
                        lo = u + 0.8 * (lo - u);
                    }
                    *end_u = lo;
                }
                let first = (this.at(ends[0]).0, this.at(ends[1]).1);
                let keep = (ends[0].max(ends[1]).floor() as usize + 1).min(n - 1);
                this.a.drain(..keep);
                this.b.drain(..keep);
                this.a.insert(0, first.0);
                this.b.insert(0, first.1);
            }
            if end == 1 {
                this.a.reverse();
                this.b.reverse();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_extension_stops_at_the_outline() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let rings = [square];
        let outline = Rings::new(&rings);
        let segs: Vec<(Point, Point)> = (0..4).map(|k| (rings[0][k], rings[0][(k + 1) % 4])).collect();
        let ext = [
            Point::new(5.0, 5.0),
            Point::new(8.0, 5.0),
            Point::new(11.0, 5.0),
            Point::new(14.0, 5.0),
        ];
        let kept = in_outline(&ext, &outline, &segs);
        assert_eq!(kept.len(), 3, "{kept:?}");
        assert!(kept[2].distance(Point::new(10.0, 5.0)) < 1e-9, "{kept:?}");
    }

    use super::*;

    #[test]
    fn the_split_is_for_the_scripts_that_the_trial_measured() {
        for c in ['A', 'z', '7', 'é', 'Ł'] {
            assert_eq!(Split::for_char(c), Split::Notches, "{c}");
        }
        for c in ['あ', 'サ', '刺', '龍'] {
            assert_eq!(Split::for_char(c), Split::Bridges, "{c}");
        }
        for c in ['한', '&', '。', 'Ж', 'Ω', 'ب'] {
            assert_eq!(Split::for_char(c), Split::Off, "{c}");
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    /// A stroke of the width `w` along the line from `a` through `b` to
    /// `c`. `round`: the outer side of the turn at `b` is an arc, not a
    /// sharp corner.
    fn bent(a: Point, b: Point, c: Point, w: f64, round: bool) -> Vec<Point> {
        let side = |p: Point, q: Point| {
            let d = (q - p).normalized().unwrap();
            Point::new(-d.y, d.x) * (w / 2.0)
        };
        let (n1, n2) = (side(a, b), side(b, c));
        // Where the two edges of one side meet.
        let m = (n1 + n2) * 0.5;
        let meet = |s: f64| b + m * (s * (w / 2.0) * (w / 2.0) / m.dot(m));
        // The outer side of the turn is the left side if the line turns
        // to the right.
        let left_is_outer = (b - a).cross(c - b) < 0.0;
        let corner = |s: f64, from: Point, to: Point| -> Vec<Point> {
            if !round || (s > 0.0) != left_is_outer {
                return vec![meet(s)];
            }
            (0..=8)
                .map(|i| {
                    let v = from.lerp(to, i as f64 / 8.0).normalized().unwrap();
                    b + v * (w / 2.0)
                })
                .collect()
        };
        let mut out = vec![a + n1];
        out.extend(corner(1.0, n1, n2));
        out.extend([c + n2, c - n2]);
        out.extend(corner(-1.0, n2 * -1.0, n1 * -1.0));
        out.push(a - n1);
        out
    }

    #[test]
    fn the_short_arms_of_a_crossing_are_strokes() {
        // A "t": a stem 100 wide and a bar 100 high that goes 40 past each
        // side of the stem (0.8 of the half-width of the stem). The circle
        // in the crossing is larger than the strokes; measured with it, the
        // arms were side branches, and the stem covered them with a bulge
        // (P21 Milestone 3).
        let p = |x: f64, y: f64| Point::new(x, y);
        let ring = vec![
            p(0.0, 0.0),
            p(100.0, 0.0),
            p(100.0, 250.0),
            p(140.0, 250.0),
            p(140.0, 350.0),
            p(100.0, 350.0),
            p(100.0, 700.0),
            p(0.0, 700.0),
            p(0.0, 350.0),
            p(-40.0, 350.0),
            p(-40.0, 250.0),
            p(0.0, 250.0),
        ];
        let s = strokes(&[ring], 700.0);
        assert!(s.satins.len() >= 2, "{} columns", s.satins.len());
        // A column of an arm goes to the end of each arm, across the bar.
        let arm = |x_end: f64| {
            s.satins.iter().any(|g| {
                g.rungs
                    .iter()
                    .chain(std::iter::once(&(
                        g.rail_a.points().last().unwrap_or_default(),
                        g.rail_b.points().last().unwrap_or_default(),
                    )))
                    .any(|(a, b)| (a.x - x_end).abs() < 15.0 && (b.x - x_end).abs() < 15.0 && (a.y - b.y).abs() > 60.0)
            })
        };
        assert!(arm(-40.0) || arm(140.0), "no column of an arm");
        assert!(s.coverage > 0.95, "{}", s.coverage);
    }

    #[test]
    fn a_turn_at_a_corner_of_the_outline_is_a_cut() {
        let turn = |deg: f64, round: bool| {
            let a = emb_geom::math::to_radians(deg);
            let c = Point::new(400.0 + 400.0 * emb_geom::math::cos(a), -400.0 * emb_geom::math::sin(a));
            let ring = bent(Point::new(0.0, 0.0), Point::new(400.0, 0.0), c, 90.0, round);
            strokes(&[ring], 700.0)
        };
        // The middle line of a bent stroke turns less than its edges (it
        // goes round the turn), so this turn is not a corner for
        // CORNER_DEG. With a sharp corner of the outline on its outer
        // side, the pen stopped there: two columns. With a round outer
        // side, the pen went on: one column.
        let sharp = turn(75.0, false);
        assert_eq!(sharp.satins.len(), 2);
        assert!(sharp.coverage > 0.9, "{}", sharp.coverage);
        assert_eq!(turn(75.0, true).satins.len(), 1);
        // A small turn is no cut, also at a sharp corner of the outline.
        assert_eq!(turn(40.0, false).satins.len(), 1);
    }

    /// The part of the area of the columns that two columns cover.
    fn two_layers(s: &Strokes) -> f64 {
        use emb_geom::region::{self, Region};
        let flat = |p: &Path| -> Vec<Point> { p.flatten(0.5).into_iter().flat_map(|l| l.points).collect() };
        let areas: Vec<Region> = s
            .satins
            .iter()
            .map(|g| {
                let mut ring = flat(&g.rail_a);
                ring.extend(flat(&g.rail_b).into_iter().rev());
                Region::new(ring, Vec::new())
            })
            .collect();
        let all: f64 = region::union(&areas, &[]).iter().map(Region::area).sum();
        let mut two = 0.0;
        for i in 0..areas.len() {
            for j in i + 1..areas.len() {
                two += region::intersection(std::slice::from_ref(&areas[i]), std::slice::from_ref(&areas[j]))
                    .iter()
                    .map(Region::area)
                    .sum::<f64>();
            }
        }
        two / all
    }

    fn ring(points: &[(f64, f64)]) -> Vec<Point> {
        points.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    #[test]
    fn a_crossing_has_one_layer() {
        // A cross: a bar 100 wide and 700 high, and a bar 100 wide and 500
        // long across it. The longer bar goes through, the other is cut
        // into two columns that stop at its sides (P20 Milestone 3), and
        // the column that goes through has the width of its bar there.
        let cross = ring(&[
            (300.0, -700.0),
            (400.0, -700.0),
            (400.0, -400.0),
            (550.0, -400.0),
            (550.0, -300.0),
            (400.0, -300.0),
            (400.0, 0.0),
            (300.0, 0.0),
            (300.0, -300.0),
            (50.0, -300.0),
            (50.0, -400.0),
            (300.0, -400.0),
        ]);
        let s = strokes(&[cross], 700.0);
        assert_eq!(s.satins.len(), 3);
        assert!(s.coverage > 0.95, "{}", s.coverage);
        assert!((s.max_width - 100.0).abs() < 15.0, "a bulge: {}", s.max_width);
        assert!(two_layers(&s) < 0.02, "two layers: {}", two_layers(&s));
    }

    #[test]
    fn a_corner_has_one_layer() {
        // An "L": the end of one column stops a little inside the other.
        let l = ring(&[
            (0.0, -700.0),
            (100.0, -700.0),
            (100.0, -100.0),
            (500.0, -100.0),
            (500.0, 0.0),
            (0.0, 0.0),
        ]);
        let s = strokes(&[l], 700.0);
        assert_eq!(s.satins.len(), 2);
        assert!(s.coverage > 0.95, "{}", s.coverage);
        assert!(two_layers(&s) < 0.02, "two layers: {}", two_layers(&s));
    }

    #[test]
    fn a_bar_is_one_column_that_covers_it() {
        // A vertical bar 100 wide and 700 high (an "I").
        let s = strokes(&[rect(0.0, -700.0, 100.0, 0.0)], 700.0);
        assert_eq!(s.satins.len(), 1, "{:?}", s.satins.len());
        assert!(s.coverage > 0.95, "{}", s.coverage);
        assert!((s.max_width - 100.0).abs() < 10.0, "{}", s.max_width);
        assert!((s.typical_width - 100.0).abs() < 5.0, "{}", s.typical_width);
    }

    #[test]
    fn a_t_is_two_columns() {
        let s = strokes(
            &[vec![
                Point::new(0.0, -700.0),
                Point::new(500.0, -700.0),
                Point::new(500.0, -610.0),
                Point::new(295.0, -610.0),
                Point::new(295.0, 0.0),
                Point::new(205.0, 0.0),
                Point::new(205.0, -610.0),
                Point::new(0.0, -610.0),
            ]],
            700.0,
        );
        assert_eq!(s.satins.len(), 2);
        assert!(s.coverage > 0.9, "{}", s.coverage);
        // The stem ends a little inside the bar (bar from y -700 to -610,
        // its middle at -655), not at the middle of the bar.
        let ys = |g: &SatinGeometry| -> Vec<f64> {
            g.rail_a
                .flatten(1.0)
                .iter()
                .chain(g.rail_b.flatten(1.0).iter())
                .flat_map(|l| l.points.iter().map(|p| p.y).collect::<Vec<_>>())
                .collect()
        };
        let height = |g: &SatinGeometry| {
            let y = ys(g);
            y.iter().copied().fold(f64::MIN, f64::max) - y.iter().copied().fold(f64::MAX, f64::min)
        };
        let stem = s.satins.iter().max_by(|a, b| height(a).total_cmp(&height(b))).unwrap();
        let top = ys(stem).into_iter().fold(f64::MAX, f64::min);
        assert!((-640.0..-610.0).contains(&top), "the stem ends at y {top}");
    }

    #[test]
    fn a_ring_is_a_closed_column() {
        let circle = |r: f64, rev: bool| {
            let mut v: Vec<Point> = (0..96)
                .map(|i| {
                    let a = i as f64 / 96.0 * std::f64::consts::TAU;
                    Point::new(350.0 + r * emb_geom::math::cos(a), -350.0 + r * emb_geom::math::sin(a))
                })
                .collect();
            if rev {
                v.reverse();
            }
            v
        };
        let s = strokes(&[circle(350.0, false), circle(260.0, true)], 700.0);
        assert_eq!(s.satins.len(), 1);
        let pl = s.satins[0].rail_a.flatten(0.1);
        assert!(pl[0].closed);
        assert!(s.coverage > 0.9, "{}", s.coverage);
    }

    #[test]
    fn an_l_is_cut_at_the_corner_and_a_dot_gets_a_stroke() {
        let l = vec![
            Point::new(0.0, -700.0),
            Point::new(100.0, -700.0),
            Point::new(100.0, -100.0),
            Point::new(450.0, -100.0),
            Point::new(450.0, 0.0),
            Point::new(0.0, 0.0),
        ];
        let s = strokes(&[l], 700.0);
        assert_eq!(s.satins.len(), 2);
        assert!(s.coverage > 0.9, "{}", s.coverage);
        let dot = strokes(&[rect(0.0, -120.0, 120.0, 0.0)], 700.0);
        assert_eq!(dot.satins.len(), 1);
        assert!(dot.coverage > 0.8, "{}", dot.coverage);
    }
}
