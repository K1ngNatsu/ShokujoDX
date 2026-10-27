//! Filled regions (polygons with holes) and boolean and offset operations.
//!
//! The operations use the `geo` crate with its `i_overlay` back end (pure Rust).
//! See ADR 0002.

use geo::algorithm::buffer::{BufferStyle, LineJoin};
use geo::{BooleanOps, Buffer, Coord, LineString, MultiLineString, MultiPolygon, Polygon};

use crate::{BBox, DEFAULT_FLATTEN_TOLERANCE_MM, Path, Point, Polyline};

/// A filled area: one outer ring and zero or more holes.
///
/// Rings do not repeat the first point at the end.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Region {
    pub outer: Vec<Point>,
    pub holes: Vec<Vec<Point>>,
}

/// Groups closed subpaths into outer shapes with their holes, by even-odd
/// nesting: a subpath inside an odd number of others is a hole of the
/// smallest outer shape that contains it. Subpaths with no area are left
/// out. The outer shapes keep the order of `subs`.
pub fn nest(subs: &[Path]) -> Vec<(Path, Vec<Path>)> {
    let rings: Vec<Vec<Point>> = subs
        .iter()
        .map(|s| s.flatten(NEST_TOL_MM).into_iter().flat_map(|pl| pl.points).collect())
        .collect();
    let areas: Vec<f64> = rings.iter().map(|r| ring_signed_area(r).abs()).collect();
    let n = rings.len();
    // `containers[i]`: rings that contain ring i (tested with its first point).
    let containers: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let Some(&p) = rings[i].first() else { return Vec::new() };
            (0..n)
                .filter(|&j| j != i && areas[j] > areas[i] && ring_contains(&rings[j], p))
                .collect()
        })
        .collect();
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        if areas[i] > 0.0 && containers[i].len().is_multiple_of(2) {
            groups.push((i, Vec::new()));
        }
    }
    for i in 0..n {
        if areas[i] == 0.0 || containers[i].len().is_multiple_of(2) {
            continue;
        }
        // Smallest containing outer shape (even depth).
        let parent = containers[i]
            .iter()
            .copied()
            .filter(|&j| containers[j].len().is_multiple_of(2))
            .min_by(|&a, &b| areas[a].total_cmp(&areas[b]));
        if let Some(pj) = parent
            && let Some(g) = groups.iter_mut().find(|(o, _)| *o == pj)
        {
            g.1.push(i);
        }
    }
    groups
        .into_iter()
        .map(|(o, hs)| (subs[o].clone(), hs.into_iter().map(|h| subs[h].clone()).collect()))
        .collect()
}

/// Tolerance to flatten subpaths for the nesting test of [`nest`].
const NEST_TOL_MM: f64 = 0.05;

/// The corner style for [`offset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// Sharp corners. Corners sharper than about 10° are cut off.
    Miter,
    /// Sharp corners, but corners sharper than 45° are cut off, so no
    /// corner goes more than about 2.6 times the distance out.
    ShortMiter,
    /// Rounded corners, approximated within the default flatten tolerance.
    Round,
    /// Corners cut off with a straight line.
    Bevel,
}

impl Region {
    pub fn new(outer: Vec<Point>, holes: Vec<Vec<Point>>) -> Self {
        Self { outer, holes }
    }

    /// Makes a region from closed polylines. Open polylines are treated as closed.
    pub fn from_polylines(outer: &Polyline, holes: &[Polyline]) -> Self {
        Self {
            outer: outer.points.clone(),
            holes: holes.iter().map(|h| h.points.clone()).collect(),
        }
    }

    /// Area in mm². Holes are subtracted.
    pub fn area(&self) -> f64 {
        let holes: f64 = self.holes.iter().map(|h| ring_signed_area(h).abs()).sum();
        ring_signed_area(&self.outer).abs() - holes
    }

    pub fn bbox(&self) -> Option<BBox> {
        BBox::from_points(self.outer.iter().copied())
    }

    /// Returns `true` if the point is inside the outer ring and not inside a hole.
    pub fn contains(&self, p: Point) -> bool {
        ring_contains(&self.outer, p) && !self.holes.iter().any(|h| ring_contains(h, p))
    }

    pub fn to_geo(&self) -> Polygon<f64> {
        Polygon::new(
            ring_to_geo(&self.outer),
            self.holes.iter().map(|h| ring_to_geo(h)).collect(),
        )
    }

    pub fn from_geo(p: &Polygon<f64>) -> Self {
        Self {
            outer: ring_from_geo(p.exterior()),
            holes: p.interiors().iter().map(ring_from_geo).collect(),
        }
    }
}

/// Signed area of a ring. With the Y axis down, a positive value means the
/// ring goes clockwise on screen.
pub fn ring_signed_area(ring: &[Point]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        sum += a.cross(b);
    }
    sum / 2.0
}

/// Even-odd point-in-ring test.
pub fn ring_contains(ring: &[Point], p: Point) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn ring_to_geo(ring: &[Point]) -> LineString<f64> {
    // `Polygon::new` closes the ring.
    LineString::new(ring.iter().map(|p| Coord { x: p.x, y: p.y }).collect())
}

fn ring_from_geo(ls: &LineString<f64>) -> Vec<Point> {
    let mut pts: Vec<Point> = ls.coords().map(|c| Point::new(c.x, c.y)).collect();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

fn to_multi(regions: &[Region]) -> MultiPolygon<f64> {
    MultiPolygon::new(regions.iter().map(Region::to_geo).collect())
}

fn from_multi(mp: &MultiPolygon<f64>) -> Vec<Region> {
    mp.0.iter().map(Region::from_geo).collect()
}

/// Area that is in `a` or in `b`.
pub fn union(a: &[Region], b: &[Region]) -> Vec<Region> {
    from_multi(&to_multi(a).union(&to_multi(b)))
}

/// Area that is in `a` and not in `b`.
pub fn difference(a: &[Region], b: &[Region]) -> Vec<Region> {
    from_multi(&to_multi(a).difference(&to_multi(b)))
}

/// Area that is in `a` and in `b`.
pub fn intersection(a: &[Region], b: &[Region]) -> Vec<Region> {
    from_multi(&to_multi(a).intersection(&to_multi(b)))
}

/// Moves the boundary of each region outward by `distance` mm.
/// A negative distance moves the boundary inward. Regions that become empty are removed.
/// The areas that are in `a` or in `b`, but not in both.
pub fn xor(a: &[Region], b: &[Region]) -> Vec<Region> {
    from_multi(&to_multi(a).xor(&to_multi(b)))
}

pub fn offset(regions: &[Region], distance: f64, join: Join) -> Vec<Region> {
    if distance == 0.0 {
        return regions.to_vec();
    }
    let style = BufferStyle::new(distance).line_join(line_join(join, distance));
    from_multi(&to_multi(regions).buffer_with_style(style))
}

/// The area within `distance` mm of the lines (round ends). Empty if
/// `distance` is not positive.
pub fn offset_lines(lines: &[Polyline], distance: f64, join: Join) -> Vec<Region> {
    if distance <= 0.0 {
        return Vec::new();
    }
    let ls: Vec<LineString<f64>> = lines
        .iter()
        .map(|pl| {
            let mut pts = pl.points.clone();
            if pl.closed
                && let Some(&first) = pts.first()
            {
                pts.push(first);
            }
            ring_to_geo(&pts)
        })
        .collect();
    let style = BufferStyle::new(distance).line_join(line_join(join, distance));
    from_multi(&MultiLineString::new(ls).buffer_with_style(style))
}

/// The parts of the lines that are inside the regions, as open lines.
pub fn clip_lines(regions: &[Region], lines: &[Polyline]) -> Vec<Polyline> {
    let ls: Vec<LineString<f64>> = lines
        .iter()
        .map(|pl| {
            let mut pts = pl.points.clone();
            if pl.closed
                && let Some(&first) = pts.first()
            {
                pts.push(first);
            }
            ring_to_geo(&pts)
        })
        .collect();
    to_multi(regions)
        .clip(&MultiLineString::new(ls), false)
        .0
        .iter()
        .map(|l| Polyline {
            points: l.coords().map(|c| Point::new(c.x, c.y)).collect(),
            closed: false,
        })
        .filter(|pl| pl.points.len() >= 2)
        .collect()
}

fn line_join(join: Join, distance: f64) -> LineJoin<f64> {
    match join {
        Join::Miter => LineJoin::Miter(crate::math::to_radians(10.0)),
        Join::ShortMiter => LineJoin::Miter(crate::math::to_radians(45.0)),
        Join::Bevel => LineJoin::Bevel,
        Join::Round => {
            // i_overlay takes segment length / radius. The sagitta of a chord
            // of length L on radius R is about L² / 8R, so L / R = sqrt(8 tol / R).
            // i_overlay rounds the segment count down, so use half of that step
            // to stay inside the tolerance.
            let r = distance.abs();
            LineJoin::Round(0.5 * (8.0 * DEFAULT_FLATTEN_TOLERANCE_MM / r).sqrt().min(1.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_miter_cuts_a_sharp_inner_corner() {
        // A square with a notch of 20° from the top: a sharp inner corner.
        let r = Region::new(
            vec![
                Point::new(0.0, 0.0),
                Point::new(8.24, 0.0),
                Point::new(10.0, 10.0),
                Point::new(11.76, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 20.0),
                Point::new(0.0, 20.0),
            ],
            Vec::new(),
        );
        let deepest = |join: Join| {
            offset(std::slice::from_ref(&r), -0.3, join)
                .iter()
                .flat_map(|x| x.outer.iter())
                .filter(|p| (p.x - 10.0).abs() < 2.0)
                .map(|p| p.y)
                .fold(0.0, f64::max)
        };
        // The long miter goes 0.3 / sin 10° below the tip of the notch
        // (y = 10); the short one is cut off near it.
        assert!(deepest(Join::Miter) > 11.5, "{}", deepest(Join::Miter));
        assert!(deepest(Join::ShortMiter) < 10.0 + 0.8, "{}", deepest(Join::ShortMiter));
    }

    #[test]
    fn clip_lines_keeps_the_inside_parts() {
        let square = Region::new(
            vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            Vec::new(),
        );
        let line = Polyline {
            points: vec![Point::new(-5.0, 5.0), Point::new(15.0, 5.0)],
            closed: false,
        };
        let parts = clip_lines(&[square], &[line]);
        assert_eq!(parts.len(), 1);
        let len: f64 = parts[0].points.windows(2).map(|w| w[0].distance(w[1])).sum();
        assert!((len - 10.0).abs() < 1e-6, "{len}");
    }

    #[test]
    fn offset_lines_makes_a_band_round_a_line() {
        let line = Polyline {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            closed: false,
        };
        let band = offset_lines(&[line], 1.0, Join::Round);
        assert_eq!(band.len(), 1);
        // A 10 x 2 mm band with round ends: 20 + pi mm².
        let a: f64 = band.iter().map(Region::area).sum();
        assert!((a - (20.0 + std::f64::consts::PI)).abs() < 0.1, "{a}");
        assert!(offset_lines(&[], 1.0, Join::Round).is_empty());
    }

    fn square(x: f64, y: f64, s: f64) -> Region {
        Region::new(
            vec![
                Point::new(x, y),
                Point::new(x + s, y),
                Point::new(x + s, y + s),
                Point::new(x, y + s),
            ],
            vec![],
        )
    }

    #[test]
    fn area_with_hole() {
        let mut r = square(0.0, 0.0, 10.0);
        r.holes.push(square(2.0, 2.0, 2.0).outer);
        assert_eq!(r.area(), 96.0);
        assert!(r.contains(Point::new(1.0, 1.0)));
        assert!(!r.contains(Point::new(3.0, 3.0)));
        assert!(!r.contains(Point::new(11.0, 1.0)));
    }

    #[test]
    fn boolean_ops() {
        let a = [square(0.0, 0.0, 10.0)];
        let b = [square(5.0, 0.0, 10.0)];
        let u: f64 = union(&a, &b).iter().map(Region::area).sum();
        let d: f64 = difference(&a, &b).iter().map(Region::area).sum();
        let i: f64 = intersection(&a, &b).iter().map(Region::area).sum();
        assert!((u - 150.0).abs() < 1e-6, "{u}");
        assert!((d - 50.0).abs() < 1e-6, "{d}");
        assert!((i - 50.0).abs() < 1e-6, "{i}");
    }

    #[test]
    fn offset_grows_and_shrinks() {
        let a = [square(0.0, 0.0, 10.0)];
        let grown: f64 = offset(&a, 1.0, Join::Miter).iter().map(Region::area).sum();
        assert!((grown - 144.0).abs() < 1e-6, "{grown}");
        let shrunk: f64 = offset(&a, -1.0, Join::Miter).iter().map(Region::area).sum();
        assert!((shrunk - 64.0).abs() < 1e-6, "{shrunk}");
        assert!(offset(&a, -6.0, Join::Miter).is_empty());
    }

    #[test]
    fn round_offset_is_within_tolerance() {
        let sq = square(0.0, 0.0, 10.0);
        let d = 1.0;
        let out = offset(std::slice::from_ref(&sq), d, Join::Round);
        assert_eq!(out.len(), 1);
        let dist_to_square = |p: Point| {
            let n = sq.outer.len();
            (0..n)
                .map(|i| p.distance_to_segment(sq.outer[i], sq.outer[(i + 1) % n]))
                .fold(f64::INFINITY, f64::min)
        };
        let ring = &out[0].outer;
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            // Vertices are on the true offset curve. Chord midpoints are inside
            // it by not more than the flatten tolerance.
            assert!((dist_to_square(a) - d).abs() < 1e-6, "{a:?}");
            let m = dist_to_square(a.midpoint(b));
            assert!(m >= d - DEFAULT_FLATTEN_TOLERANCE_MM && m <= d + 1e-6, "{m}");
        }
    }

    #[test]
    fn offset_is_deterministic() {
        let a = [square(0.0, 0.0, 10.0), square(20.0, 3.0, 4.0)];
        assert_eq!(offset(&a, 0.7, Join::Round), offset(&a, 0.7, Join::Round));
    }
}
