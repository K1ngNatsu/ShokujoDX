//! Quilting helpers (P11 Milestone 6, ADR 0049): echo quilting, a fill of
//! the background round shapes and appliqué, and edge-to-edge quilt blocks.
//!
//! Not a port; common quilting practice:
//!
//! - Echo quilting: lines round the selected shapes (their areas joined),
//!   `distance` out and then one `spacing` apart. They are one line: each
//!   ring starts at its point nearest to the end of the ring before, and a
//!   short step joins them. With a border shape in the selection (a shape
//!   that holds all the others), the rings stop at its edge.
//! - Background: the area of the border shape (or a box round the
//!   selection) without the selected objects and a gap round them, as one
//!   fill (stipple or meander, so the background goes flat and the shapes
//!   stand up).
//! - Quilt block: the selected lines, repeated in rows and columns (a step
//!   of the size of their box, so the copies touch) over a block, cut at
//!   the block edge. Line ends that meet are joined, so a pattern made to
//!   join gives long lines. The block then sews in hoopings (P7): lines are
//!   cut at the split lines with no overlap, so the parts of two hoopings
//!   meet.

use emb_geom::region::{self, Join, Region};
use emb_geom::{BBox, Path, Point, Polyline, Transform};

use crate::shaping::{ShapeError, area_of};
use crate::{Design, ObjectId, ObjectKind};

/// Most echo rings.
pub const MAX_ECHOES: u32 = 100;
/// Line ends nearer than this are joined into one line, in mm.
const JOIN_MM: f64 = 0.5;
/// Most copies of a quilt block pattern.
const MAX_TILES: usize = 10_000;

/// The settings of echo quilting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchoSpec {
    /// The distance of the first ring from the shapes, in mm.
    pub distance_mm: f64,
    /// The distance between two rings, in mm.
    pub spacing_mm: f64,
    /// The number of rings.
    pub rows: u32,
}

impl Default for EchoSpec {
    fn default() -> Self {
        EchoSpec {
            distance_mm: 3.0,
            spacing_mm: 6.0,
            rows: 5,
        }
    }
}

/// The selection split into its border shape (if one holds all the others)
/// and the other objects.
pub fn border_of(design: &Design, ids: &[ObjectId]) -> (Option<ObjectId>, Vec<ObjectId>) {
    let areas: Vec<(ObjectId, Option<Vec<Region>>)> = ids
        .iter()
        .filter_map(|id| design.object(*id))
        .map(|o| (o.id, area_of(&o.kind)))
        .collect();
    if areas.len() >= 2 {
        for (id, a) in &areas {
            let Some(a) = a else { continue };
            let others: Vec<ObjectId> = ids.iter().copied().filter(|x| x != id).collect();
            let rest = design.area_of_objects(&others);
            let outside: f64 = region::difference(&rest, a).iter().map(Region::area).sum();
            if !rest.is_empty() && outside < 1e-3 {
                return (Some(*id), others);
            }
        }
    }
    (None, ids.to_vec())
}

/// The place of `p` on the closed `ring` as a length along it from its
/// first point, if `p` is on the ring.
fn on_ring(ring: &[Point], p: Point) -> Option<f64> {
    let mut at = 0.0;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if p.distance_to_segment(a, b) < 0.05 {
            return Some(at + a.distance(p).min(a.distance(b)));
        }
        at += a.distance(b);
    }
    None
}

/// The point of segment `a`–`b` nearest to `p`.
fn nearest_on(a: Point, b: Point, p: Point) -> Point {
    let v = b - a;
    let l = v.dot(v);
    let t = if l > 0.0 {
        ((p - a).dot(v) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + v * t
}

/// The corners of `ring` between the places `from` and `to` (lengths along
/// it), the shorter way round.
fn along_ring(ring: &[Point], from: f64, to: f64) -> Vec<Point> {
    let n = ring.len();
    let mut at = vec![0.0];
    for i in 0..n {
        at.push(at[i] + ring[i].distance(ring[(i + 1) % n]));
    }
    let total = at[n];
    if total <= 0.0 {
        return Vec::new();
    }
    let forward = (to - from).rem_euclid(total);
    let ahead = |s: f64| (s - from).rem_euclid(total);
    let mut corners: Vec<(f64, Point)> = (0..n)
        .map(|i| (ahead(at[i]), ring[i]))
        .filter(|(d, _)| *d > 1e-9)
        .collect();
    if forward <= total / 2.0 {
        corners.retain(|(d, _)| *d < forward);
        corners.sort_by(|a, b| a.0.total_cmp(&b.0));
    } else {
        corners.retain(|(d, _)| *d > forward);
        corners.sort_by(|a, b| b.0.total_cmp(&a.0));
    }
    corners.into_iter().map(|(_, p)| p).collect()
}

/// Orders closed rings and open lines into one line, each from its end (or
/// for a ring, its point) nearest to the end of the one before. A step
/// from the border to the border goes along the border (`borders`), not
/// across the rings.
fn chain(mut items: Vec<Polyline>, start: Point, borders: &[Vec<Point>]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    let mut at = start;
    while !items.is_empty() {
        let mut best = (f64::INFINITY, 0, 0, false);
        for (i, it) in items.iter().enumerate() {
            if it.closed {
                for (j, p) in it.points.iter().enumerate() {
                    let d = p.distance(at);
                    if d < best.0 {
                        best = (d, i, j, false);
                    }
                }
            } else {
                let (a, b) = (it.points[0], it.points[it.points.len() - 1]);
                if a.distance(at) < best.0 {
                    best = (a.distance(at), i, 0, false);
                }
                if b.distance(at) < best.0 {
                    best = (b.distance(at), i, 0, true);
                }
            }
        }
        let (_, i, j, reverse) = best;
        let it = items.swap_remove(i);
        let mut pts = it.points;
        if it.closed {
            pts.rotate_left(j);
            pts.push(pts[0]);
        } else if reverse {
            pts.reverse();
        }
        if !out.is_empty() {
            for ring in borders {
                let Some(b) = on_ring(ring, pts[0]) else { continue };
                match on_ring(ring, at) {
                    Some(a) => out.extend(along_ring(ring, a, b)),
                    None => {
                        // From inside: out to the nearest point of the
                        // border if that is nearer, then along it.
                        let near = (0..ring.len())
                            .map(|i| nearest_on(ring[i], ring[(i + 1) % ring.len()], at))
                            .min_by(|p, q| p.distance(at).total_cmp(&q.distance(at)));
                        if let Some(q) = near.filter(|q| q.distance(at) < pts[0].distance(at))
                            && let Some(a) = on_ring(ring, q)
                        {
                            out.push(q);
                            out.extend(along_ring(ring, a, b));
                        }
                    }
                }
                break;
            }
        }
        at = pts[pts.len() - 1];
        out.extend(pts);
    }
    out.dedup_by(|a, b| a.distance(*b) < 1e-9);
    out
}

impl Design {
    /// The areas of the objects joined (lettering by the area of its
    /// letters); lines are left out.
    pub fn area_of_objects(&self, ids: &[ObjectId]) -> Vec<Region> {
        let mut area = Vec::new();
        for o in ids.iter().filter_map(|id| self.object(*id)) {
            let a = match &o.kind {
                ObjectKind::Lettering(_) => Some(crate::shaping::even_odd(&crate::vector::area_outline(&o.kind))),
                k => area_of(k),
            };
            if let Some(a) = a {
                area = region::union(&area, &a);
            }
        }
        area
    }

    /// The echo quilting line round the objects `ids` (see the module
    /// notes), as one open path.
    pub fn echo_path(&self, ids: &[ObjectId], spec: &EchoSpec) -> Result<Path, ShapeError> {
        let (border, shapes) = border_of(self, ids);
        let (area, lines) = self.outline_sources(&shapes);
        if area.is_empty() && lines.is_empty() {
            return Err(ShapeError::NoOutline);
        }
        let border_area = border.and_then(|b| self.object(b)).and_then(|o| area_of(&o.kind));
        let mut items: Vec<Polyline> = Vec::new();
        for k in 0..spec.rows.clamp(1, MAX_ECHOES) {
            let d = spec.distance_mm.max(0.0) + f64::from(k) * spec.spacing_mm.max(0.1);
            let mut regions = if d == 0.0 {
                area.clone()
            } else {
                region::offset(&area, d, Join::Round)
            };
            if d > 0.0 {
                regions = region::union(&regions, &region::offset_lines(&lines, d, Join::Round));
            }
            let rings: Vec<Polyline> = regions
                .iter()
                .flat_map(|r| std::iter::once(&r.outer).chain(&r.holes))
                .filter(|r| r.len() >= 3)
                .map(|r| Polyline {
                    points: r.clone(),
                    closed: true,
                })
                .collect();
            match &border_area {
                Some(b) => {
                    for ring in rings {
                        // A ring inside the border stays closed.
                        let inside = region::difference(&[Region::new(ring.points.clone(), Vec::new())], b);
                        if inside.iter().map(Region::area).sum::<f64>() < 1e-6 {
                            items.push(ring);
                        } else {
                            items.extend(region::clip_lines(b, &[ring]));
                        }
                    }
                }
                None => items.extend(rings),
            }
        }
        if items.is_empty() {
            return Err(ShapeError::NothingLeft);
        }
        let start = area
            .first()
            .and_then(|r| r.outer.first().copied())
            .or_else(|| lines.first().map(|l| l.points[0]))
            .unwrap_or(Point::ZERO);
        let borders: Vec<Vec<Point>> = border_area
            .iter()
            .flatten()
            .flat_map(|r| std::iter::once(r.outer.clone()).chain(r.holes.iter().cloned()))
            .collect();
        let pts = chain(items, start, &borders);
        Ok(pts[1..].iter().fold(Path::new().move_to(pts[0]), |p, q| p.line_to(*q)))
    }

    /// The background area round the objects `ids`: the border shape in
    /// the selection, or the box round the objects made larger by
    /// `margin_mm`, without the objects and a gap of `gap_mm` round them.
    pub fn background_area(&self, ids: &[ObjectId], gap_mm: f64, margin_mm: f64) -> Result<Vec<Region>, ShapeError> {
        let (border, shapes) = border_of(self, ids);
        let (area, lines) = self.outline_sources(&shapes);
        if area.is_empty() && lines.is_empty() {
            return Err(ShapeError::NoOutline);
        }
        let mut keep_out = region::offset(&area, gap_mm.max(0.0), Join::Round);
        if !lines.is_empty() {
            keep_out = region::union(&keep_out, &region::offset_lines(&lines, gap_mm.max(0.1), Join::Round));
        }
        let outer = match border.and_then(|b| self.object(b)).and_then(|o| area_of(&o.kind)) {
            Some(b) => b,
            None => {
                let b = keep_out
                    .iter()
                    .flat_map(|r| r.outer.iter().copied())
                    .fold(None::<BBox>, |acc, p| {
                        let one = BBox::new(p, p);
                        Some(acc.map_or(one, |a| a.union(one)))
                    })
                    .ok_or(ShapeError::NoOutline)?;
                let m = margin_mm.max(0.0);
                let b = BBox::new(b.min - Point::new(m, m), b.max + Point::new(m, m));
                vec![Region::new(
                    vec![b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)],
                    Vec::new(),
                )]
            }
        };
        let bg = crate::shaping::clean(region::difference(&outer, &keep_out));
        if bg.is_empty() {
            return Err(ShapeError::NothingLeft);
        }
        Ok(bg)
    }
}

/// A fill of `regions` (the outer rings as the outline, the holes as holes).
pub fn fill_of(regions: &[Region], params: crate::FillParams) -> ObjectKind {
    let outers: Vec<Path> = regions
        .iter()
        .filter(|r| r.outer.len() >= 3)
        .map(|r| Path::polygon(&r.outer))
        .collect();
    let holes = regions
        .iter()
        .flat_map(|r| r.holes.iter())
        .filter(|h| h.len() >= 3)
        .map(|h| Path::polygon(h))
        .collect();
    ObjectKind::Fill {
        outer: Path::joined(&outers),
        holes,
        params,
    }
}

/// The lines of a quilt block of `width` × `height` mm centered on
/// `center`: `lines` repeated in rows and columns at the size of their box
/// (plus the gap across, `gap.x`, and down, `gap.y`), cut at the block
/// edge, with the ends that meet joined.
pub fn block_lines(lines: &[Polyline], center: Point, width: f64, height: f64, gap: Point) -> Vec<Polyline> {
    let Some(tile) = lines
        .iter()
        .filter_map(|l| BBox::from_points(l.points.iter().copied()))
        .reduce(BBox::union)
    else {
        return Vec::new();
    };
    let (sx, sy) = (tile.width() + gap.x, tile.height() + gap.y);
    if sx.is_nan() || sy.is_nan() || sx <= 0.1 || sy <= 0.1 {
        return Vec::new();
    }
    let block = BBox::from_center_size(center, width.max(1.0), height.max(1.0));
    // Copies so that one is where the pattern is and they cover the block.
    let (i0, i1) = (
        ((block.min.x - tile.max.x) / sx).floor() as i64,
        ((block.max.x - tile.min.x) / sx).ceil() as i64,
    );
    let (j0, j1) = (
        ((block.min.y - tile.max.y) / sy).floor() as i64,
        ((block.max.y - tile.min.y) / sy).ceil() as i64,
    );
    if ((i1 - i0 + 1) * (j1 - j0 + 1)) as usize > MAX_TILES {
        return Vec::new();
    }
    let mut copies = Vec::new();
    for j in j0..=j1 {
        for i in i0..=i1 {
            let t = Transform::translate(i as f64 * sx, j as f64 * sy);
            for l in lines {
                let mut pts: Vec<Point> = l.points.iter().map(|p| t.apply(*p)).collect();
                if l.closed {
                    pts.push(pts[0]);
                }
                copies.push(Polyline {
                    points: pts,
                    closed: false,
                });
            }
        }
    }
    let rect = Region::new(
        vec![
            block.min,
            Point::new(block.max.x, block.min.y),
            block.max,
            Point::new(block.min.x, block.max.y),
        ],
        Vec::new(),
    );
    join_ends(region::clip_lines(&[rect], &copies))
}

/// Joins lines whose ends are nearer than [`JOIN_MM`].
fn join_ends(mut lines: Vec<Polyline>) -> Vec<Polyline> {
    let mut out: Vec<Polyline> = Vec::new();
    while let Some(mut cur) = lines.pop() {
        loop {
            let end = cur.points[cur.points.len() - 1];
            let start = cur.points[0];
            let found = lines.iter().enumerate().find_map(|(i, l)| {
                let (a, b) = (l.points[0], l.points[l.points.len() - 1]);
                if a.distance(end) < JOIN_MM {
                    Some((i, false, false))
                } else if b.distance(end) < JOIN_MM {
                    Some((i, true, false))
                } else if b.distance(start) < JOIN_MM {
                    Some((i, false, true))
                } else if a.distance(start) < JOIN_MM {
                    Some((i, true, true))
                } else {
                    None
                }
            });
            let Some((i, reverse, before)) = found else { break };
            let mut l = lines.swap_remove(i).points;
            if reverse {
                l.reverse();
            }
            if before {
                l.extend(cur.points.into_iter().skip(1));
                cur.points = l;
            } else {
                cur.points.extend(l.into_iter().skip(1));
            }
        }
        if cur.points[0].distance(cur.points[cur.points.len() - 1]) < 1e-9 && cur.points.len() > 3 {
            cur.points.pop();
            cur.closed = true;
        }
        out.push(cur);
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FillParams, Rgb, RunningParams, ThreadId};

    fn fill(d: &mut Design, t: ThreadId, outer: Path) -> ObjectId {
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params: FillParams::default(),
            },
        )
    }

    fn design() -> (Design, ThreadId) {
        let mut d = Design::new("q");
        let t = d.add_thread("Blue", Rgb(0, 0, 200));
        (d, t)
    }

    #[test]
    fn echo_rings_are_one_line_at_the_spacing() {
        let (mut d, t) = design();
        let heart = fill(&mut d, t, Path::ellipse(Point::ZERO, 10.0, 10.0));
        let spec = EchoSpec {
            distance_mm: 2.0,
            spacing_mm: 4.0,
            rows: 3,
        };
        let path = d.echo_path(&[heart], &spec).unwrap();
        let lines = path.flatten(0.05);
        assert_eq!(lines.len(), 1, "one line");
        // Points at 12, 16 and 20 mm from the middle, and none farther.
        let r: Vec<f64> = lines[0].points.iter().map(|p| p.length()).collect();
        for want in [12.0, 16.0, 20.0] {
            assert!(r.iter().any(|x| (x - want).abs() < 0.05), "{want}");
        }
        assert!(r.iter().all(|x| *x < 20.1 && *x > 11.9));
    }

    #[test]
    fn a_border_shape_stops_the_echoes_and_holds_the_background() {
        let (mut d, t) = design();
        let shape = fill(&mut d, t, Path::ellipse(Point::ZERO, 10.0, 10.0));
        let border = fill(
            &mut d,
            t,
            Path::rect(BBox::new(Point::new(-30.0, -30.0), Point::new(30.0, 30.0))),
        );
        assert_eq!(border_of(&d, &[shape, border]), (Some(border), vec![shape]));
        let spec = EchoSpec {
            distance_mm: 3.0,
            spacing_mm: 5.0,
            rows: 8,
        };
        let path = d.echo_path(&[shape, border], &spec).unwrap();
        let pts: Vec<Point> = path.flatten(0.05).into_iter().flat_map(|l| l.points).collect();
        // Rings up to 48 mm out are cut at the border.
        assert!(pts.iter().all(|p| p.x.abs() <= 30.0 + 1e-6 && p.y.abs() <= 30.0 + 1e-6));
        // The steps from one cut piece to the next go along the border: no
        // stitch goes across the square far from its edge and the rings.
        for w in pts.windows(2) {
            let m = w[0].midpoint(w[1]);
            let on_edge = m.x.abs() > 30.0 - 1e-6 || m.y.abs() > 30.0 - 1e-6;
            assert!(on_edge || w[0].distance(w[1]) < 6.0, "{w:?}");
        }
        assert!(
            pts.iter().any(|p| p.length() > 35.0),
            "the outer rings reach the corners"
        );
        // The background: the border without the shape and the gap.
        let bg = d.background_area(&[shape, border], 2.0, 0.0).unwrap();
        let area: f64 = bg.iter().map(Region::area).sum();
        let want = 60.0 * 60.0 - std::f64::consts::PI * 12.0 * 12.0;
        assert!((area - want).abs() < 3.0, "{area} {want}");
        // No border: a box round the shape, 5 mm larger.
        let bg = d.background_area(&[shape], 2.0, 5.0).unwrap();
        let area: f64 = bg.iter().map(Region::area).sum();
        let want = 34.0 * 34.0 - std::f64::consts::PI * 12.0 * 12.0;
        assert!((area - want).abs() < 3.0, "{area} {want}");
    }

    #[test]
    fn a_block_repeats_the_pattern_and_joins_its_lines() {
        // A wave 20 mm long that ends level with its start: copies join.
        let wave = Polyline {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(5.0, -3.0),
                Point::new(10.0, 0.0),
                Point::new(15.0, 3.0),
                Point::new(20.0, 0.0),
            ],
            closed: false,
        };
        let lines = block_lines(&[wave], Point::ZERO, 100.0, 30.0, Point::new(0.0, 4.0));
        // Rows 10 mm apart over 30 mm: about three rows, each one line across.
        assert!((3..=4).contains(&lines.len()), "{}", lines.len());
        for l in &lines {
            let xs: Vec<f64> = l.points.iter().map(|p| p.x).collect();
            let (lo, hi) = xs
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
            assert!((lo + 50.0).abs() < 1e-6 && (hi - 50.0).abs() < 1e-6, "{lo} {hi}");
            assert!(l.points.iter().all(|p| p.y.abs() <= 15.0 + 1e-6));
        }
        assert!(block_lines(&[], Point::ZERO, 100.0, 100.0, Point::ZERO).is_empty());
    }

    #[test]
    fn lines_round_a_line_are_echoes_too() {
        let (mut d, t) = design();
        let line = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(Point::ZERO).line_to(Point::new(20.0, 0.0)),
                params: RunningParams::default(),
            },
        );
        let path = d.echo_path(&[line], &EchoSpec::default()).unwrap();
        assert!(!path.is_empty());
    }
}
