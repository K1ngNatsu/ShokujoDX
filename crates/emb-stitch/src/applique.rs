//! Appliqué stitches: placement lines, tack-down and cover, with the stops
//! between them. See `emb_model::applique` for the sew order.

use emb_geom::region::{ring_contains, ring_signed_area};
use emb_geom::{Path, Point, Polyline};
use emb_model::{AppliqueParams, AppliquePiece, SatinGeometry, SatinParams, TackDown, Underlay, outline_edges};

use crate::object::{FLATTEN_TOL_MM, Run, RunLink, StopReason};
use crate::{running, satin};

/// At sharp corners an offset point goes no farther out than this many offsets.
const MAX_MITER: f64 = 2.0;

/// One ring of a piece: its points (not closed: the last point is not the
/// first), where each edge starts in `points`, and whether the material is
/// outside the ring (a hole).
struct Ring {
    points: Vec<Point>,
    edge_starts: Vec<usize>,
    hole: bool,
}

impl Ring {
    fn new(path: &Path, hole: bool) -> Option<Ring> {
        let edges = outline_edges(path, FLATTEN_TOL_MM);
        let mut points = Vec::new();
        let mut edge_starts = Vec::new();
        for e in &edges {
            edge_starts.push(points.len());
            // The last point of an edge is the first point of the next one.
            points.extend_from_slice(&e[..e.len() - 1]);
        }
        (points.len() >= 3 && ring_signed_area(&points).abs() > 1e-9).then_some(Ring {
            points,
            edge_starts,
            hole,
        })
    }

    /// The points moved by `d` mm into the material (negative: away from it).
    fn offset(&self, d: f64) -> Vec<Point> {
        let pts = &self.points;
        let n = pts.len();
        if d == 0.0 {
            return pts.clone();
        }
        // The side of the segment normals that is inside the ring.
        let inside = (0..n)
            .find_map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let nrm = (b - a).perp().normalized()?;
                let probe = a.midpoint(b) + nrm * 1e-3;
                Some(if ring_contains(pts, probe) { 1.0 } else { -1.0 })
            })
            .unwrap_or(1.0);
        let sign = if self.hole { -inside } else { inside };
        let seg_normal = |i: usize| (pts[(i + 1) % n] - pts[i]).perp().normalized().unwrap_or_default() * sign;
        (0..n)
            .map(|i| {
                let (prev, next) = (seg_normal((i + n - 1) % n), seg_normal(i));
                let m = (prev + next).normalized().unwrap_or(next);
                let cos_half = m.dot(next).max(1.0 / MAX_MITER);
                pts[i] + m * (d / cos_half)
            })
            .collect()
    }

    /// The runs of edges that get cover, as open lists of point indices
    /// (the last index can be `n`, which is point 0 again). `None`: all
    /// edges get cover (a closed ring).
    fn covered_spans(&self, open: &[usize]) -> Option<Vec<Vec<usize>>> {
        let m = self.edge_starts.len();
        let open: Vec<usize> = open.iter().copied().filter(|e| *e < m).collect();
        if open.is_empty() || self.hole {
            return None;
        }
        let n = self.points.len();
        let end_of = |e: usize| if e + 1 < m { self.edge_starts[e + 1] } else { n };
        let mut spans: Vec<Vec<usize>> = Vec::new();
        // Start after an open edge, so that a span does not wrap in the middle.
        let first = (open[0] + 1) % m;
        let mut cur: Vec<usize> = Vec::new();
        for k in 0..m {
            let e = (first + k) % m;
            if open.contains(&e) {
                if cur.len() >= 2 {
                    spans.push(std::mem::take(&mut cur));
                }
                cur.clear();
                continue;
            }
            let (s, t) = (self.edge_starts[e], end_of(e));
            if cur.last() != Some(&s) {
                cur.push(s);
            }
            cur.extend(s + 1..=t);
        }
        if cur.len() >= 2 {
            spans.push(cur);
        }
        Some(spans)
    }
}

fn polygon(pts: &[Point]) -> Path {
    Path::polygon(pts)
}

fn polyline_path(pts: &[Point]) -> Path {
    pts[1..].iter().fold(Path::new().move_to(pts[0]), |p, q| p.line_to(*q))
}

/// The ring points from the one nearest to `at`, so that the placement
/// line, the tack-down and the cover of a ring start at the same place.
fn from_nearest(mut pts: Vec<Point>, at: Point) -> Vec<Point> {
    if let Some(i) = (0..pts.len()).min_by(|&i, &j| pts[i].distance(at).total_cmp(&pts[j].distance(at))) {
        pts.rotate_left(i);
    }
    pts
}

/// A running loop along the ring, from its first point back to it.
fn loop_run(ring: Vec<Point>, stitch_mm: f64) -> Run {
    let line = Polyline {
        points: ring,
        closed: true,
    };
    Run {
        closed: true,
        ..Run::new(running::running_points(&line, stitch_mm))
    }
}

/// A satin over the centerline `center` (closed if `closed`).
fn satin_run(center: &[Point], closed: bool, width: f64, p: &SatinParams, entry: Point) -> Option<Run> {
    if center.len() < 2 {
        return None;
    }
    let path = if closed { polygon(center) } else { polyline_path(center) };
    let g = SatinGeometry::from_centerline(&path, width);
    satin::satin(&g, p, entry, None)
        .filter(|pts| pts.len() >= 2)
        .map(Run::new)
}

/// The runs of an appliqué object in sew order. The first tack-down run of
/// each layer and the first cover run have a stop before them (see the
/// params); a layer over another has a stop before its placement line, to
/// cut the fabric of the layer before.
pub fn applique_runs(pieces: &[AppliquePiece], p: &AppliqueParams, entry: Point) -> Vec<Run> {
    // Each ring with the open edges and the layer of its piece, the layers in order.
    let mut layers: Vec<u8> = pieces.iter().map(|pc| pc.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    let rings: Vec<(Ring, &[usize], u8)> = layers
        .iter()
        .flat_map(|&l| pieces.iter().filter(move |pc| pc.layer == l))
        .flat_map(|pc| {
            let outer = Ring::new(&pc.outer, false).map(|r| (r, pc.open_edges.as_slice(), pc.layer));
            let holes = pc
                .holes
                .iter()
                .filter_map(|h| Ring::new(h, true).map(|r| (r, &[][..], pc.layer)));
            outer.into_iter().chain(holes)
        })
        .collect();
    let mut runs: Vec<Run> = Vec::new();

    // Where each ring starts: its outline point nearest to the needle.
    let starts: Vec<Point> = rings
        .iter()
        .map(|(r, _, _)| from_nearest(r.points.clone(), entry)[0])
        .collect();
    let zigzag = SatinParams {
        spacing_mm: p.tack_spacing_mm.max(0.3),
        pull_compensation_mm: 0.0,
        short_stitches: false,
        underlay: Underlay::None,
        split_mm: 99.0,
        ..SatinParams::default()
    };
    // The runs where a new step starts on the rings of the step before.
    let mut steps = Vec::new();
    for (li, &layer) in layers.iter().enumerate() {
        let layer_start = runs.len();
        let of_layer = || rings.iter().zip(&starts).filter(move |((_, _, l), _)| *l == layer);
        if p.placement {
            for ((r, _, _), at) in of_layer() {
                runs.push(loop_run(from_nearest(r.points.clone(), *at), p.placement_stitch_mm));
            }
        }
        let tack_start = runs.len();
        for ((r, _, _), at) in of_layer() {
            let line = from_nearest(r.offset(p.tack_inset_mm), *at);
            match p.tack_down {
                TackDown::Run => runs.push(loop_run(line, p.placement_stitch_mm)),
                TackDown::Zigzag => runs.extend(satin_run(&line, true, p.tack_width_mm, &zigzag, *at)),
                TackDown::None => {}
            }
        }
        // The fabric goes on after the placement line, or at the start of
        // the layer if there is no placement line.
        if let Some(r) = runs.get_mut(tack_start) {
            r.stop = Some(StopReason::PlaceFabric);
        }
        // The fabric of the layer before is cut before this layer starts.
        if li > 0
            && p.trim_stop
            && let Some(r) = runs.get_mut(layer_start)
            && r.stop.is_none()
        {
            r.stop = Some(StopReason::TrimFabric);
        }
        steps.push(tack_start);
    }

    let cover_start = runs.len();
    if p.cover {
        let w = p.cover_width_mm.max(0.5);
        // The centerline of the cover, so that it goes `cover_outside_mm` outside the outline.
        let shift = w / 2.0 - p.cover_outside_mm;
        for ((r, open, _), at) in rings.iter().zip(&starts) {
            let center = r.offset(shift);
            match r.covered_spans(open) {
                None => runs.extend(satin_run(&from_nearest(center, *at), true, w, &p.cover_satin, *at)),
                Some(spans) => {
                    for s in spans {
                        let pts: Vec<Point> = s.iter().map(|i| center[i % center.len()]).collect();
                        runs.extend(satin_run(&pts, false, w, &p.cover_satin, *at));
                    }
                }
            }
        }
        if p.trim_stop
            && cover_start > 0
            && let Some(r) = runs.get_mut(cover_start)
        {
            r.stop = Some(StopReason::TrimFabric);
        }
    }
    // From one step to the next on the same ring the gap is short and the
    // cover sews over it: a stitch, not a trim.
    let near = p.cover_width_mm.max(0.5) + 1.0;
    steps.push(cover_start);
    for k in steps {
        if k > 0
            && k < runs.len()
            && let Some(&end) = runs[k - 1].points.last()
            && runs[k].points.first().is_some_and(|s| s.distance(end) <= near)
        {
            runs[k].link = RunLink::Travel;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    #[test]
    fn layers_sew_one_after_another_and_the_covers_last() {
        let big = square(40.0);
        let mut small = AppliquePiece::new(
            Path::rect(BBox::new(Point::new(10.0, 10.0), Point::new(30.0, 30.0))),
            Vec::new(),
        );
        small.layer = 1;
        let p = AppliqueParams::default();
        let runs = applique_runs(&[small.clone(), big.clone()], &p, Point::ZERO);
        let stops: Vec<(usize, StopReason)> = runs
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.stop.map(|s| (i, s)))
            .collect();
        // Layer 1: placement, (place), tack-down; layer 2: (trim) placement,
        // (place), tack-down; (trim) the covers.
        assert_eq!(
            stops.iter().map(|s| s.1).collect::<Vec<_>>(),
            [
                StopReason::PlaceFabric,
                StopReason::TrimFabric,
                StopReason::PlaceFabric,
                StopReason::TrimFabric
            ]
        );
        assert_eq!(stops.iter().map(|s| s.0).collect::<Vec<_>>(), [1, 2, 3, 4]);
        // The first layer is the large piece, though it comes second in the list.
        let first = BBox::from_points(runs[0].points.iter().copied()).unwrap();
        assert!(first.width() > 35.0, "{first:?}");
        // The covers: the bottom layer first.
        let cover_bottom = BBox::from_points(runs[4].points.iter().copied()).unwrap();
        assert!(cover_bottom.width() > 35.0);
        // One layer: the same runs as before the layers.
        let one = applique_runs(std::slice::from_ref(&big), &p, Point::ZERO);
        assert_eq!(one.iter().filter(|r| r.stop.is_some()).count(), 2);
    }

    use super::*;

    fn square(size: f64) -> AppliquePiece {
        AppliquePiece::new(Path::rect(BBox::new(Point::ZERO, Point::new(size, size))), Vec::new())
    }

    fn bbox(r: &Run) -> BBox {
        BBox::from_points(r.points.iter().copied()).unwrap()
    }

    #[test]
    fn placement_tack_down_cover_with_stops() {
        let runs = applique_runs(&[square(30.0)], &AppliqueParams::default(), Point::ZERO);
        assert_eq!(runs.len(), 3);
        let stops: Vec<Option<StopReason>> = runs.iter().map(|r| r.stop).collect();
        assert_eq!(
            stops,
            [None, Some(StopReason::PlaceFabric), Some(StopReason::TrimFabric)]
        );
        // The placement line is on the outline, the tack-down inside it.
        let (pl, tack, cover) = (bbox(&runs[0]), bbox(&runs[1]), bbox(&runs[2]));
        assert!((pl.min.x - 0.0).abs() < 1e-9 && (pl.max.x - 30.0).abs() < 1e-9);
        assert!(
            (tack.min.x - 0.5).abs() < 1e-6 && (tack.max.x - 29.5).abs() < 1e-6,
            "{tack:?}"
        );
        // The cover goes 1 mm outside (plus pull compensation) and 2 mm inside.
        assert!(cover.min.x < -0.9 && cover.min.x > -1.5, "{cover:?}");
        assert!(runs[0].closed && runs[0].fixed);
        // All three start at the corner nearest to the needle.
        assert!(runs[0].points[0].distance(runs[1].points[0]) < 1.0);
        assert!(runs[2].fixed);
    }

    #[test]
    fn options_change_the_runs() {
        let p = AppliqueParams {
            placement: false,
            trim_stop: false,
            tack_down: TackDown::Zigzag,
            ..AppliqueParams::default()
        };
        let runs = applique_runs(&[square(30.0)], &p, Point::ZERO);
        assert_eq!(runs.len(), 2);
        assert_eq!(
            runs[0].stop,
            Some(StopReason::PlaceFabric),
            "the fabric goes on before the tack-down"
        );
        assert_eq!(runs[1].stop, None);
        let no_cover = AppliqueParams {
            cover: false,
            ..AppliqueParams::default()
        };
        assert_eq!(applique_runs(&[square(30.0)], &no_cover, Point::ZERO).len(), 2);
        assert!(applique_runs(&[], &AppliqueParams::default(), Point::ZERO).is_empty());
    }

    #[test]
    fn holes_and_two_pieces_sew_each_step_together() {
        let hole = Path::rect(BBox::new(Point::new(10.0, 10.0), Point::new(20.0, 20.0)));
        let with_hole = AppliquePiece::new(Path::rect(BBox::new(Point::ZERO, Point::new(30.0, 30.0))), vec![hole]);
        let mut other = square(10.0);
        other.outer = other.outer.transformed(&emb_geom::Transform::translate(50.0, 0.0));
        let runs = applique_runs(&[with_hole, other], &AppliqueParams::default(), Point::ZERO);
        assert_eq!(runs.len(), 9, "three rings, three steps");
        assert_eq!(runs[3].stop, Some(StopReason::PlaceFabric));
        assert_eq!(runs[6].stop, Some(StopReason::TrimFabric));
        assert!(
            runs.iter().filter(|r| r.stop.is_some()).count() == 2,
            "one stop for each step"
        );
        // The tack-down of the hole is outside the hole (in the material).
        let hole_tack = bbox(&runs[4]);
        assert!(hole_tack.min.x < 10.0 && hole_tack.max.x > 20.0, "{hole_tack:?}");
    }

    #[test]
    fn partial_cover_skips_open_edges() {
        let mut pc = square(30.0);
        pc.open_edges = vec![0];
        let runs = applique_runs(&[pc.clone()], &AppliqueParams::default(), Point::ZERO);
        assert_eq!(runs.len(), 3);
        let cover = bbox(&runs[2]);
        // Edge 0 is the top edge (y = 0): the cover does not go above it.
        assert!(cover.min.y > 0.4, "{cover:?}");
        pc.open_edges = vec![0, 2];
        let runs = applique_runs(&[pc], &AppliqueParams::default(), Point::ZERO);
        assert_eq!(runs.len(), 4, "two cover parts");
        assert_eq!(runs[2].stop, Some(StopReason::TrimFabric));
        assert_eq!(runs[3].stop, None);
    }
}
