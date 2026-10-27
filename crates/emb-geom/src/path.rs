use serde::{Deserialize, Serialize};

use crate::{BBox, Point, Transform};

/// Maximum recursion depth to flatten one cubic segment.
/// 2^16 segments is far more than a sewable curve needs.
const MAX_FLATTEN_DEPTH: u32 = 16;

/// Standard constant for a quarter circle with one cubic Bezier.
const QUARTER_CIRCLE_K: f64 = 0.552_284_749_830_793_4;

/// One drawing command of a [`Path`].
///
/// The serialized forms are short: `{"M":[x,y]}`, `{"L":[x,y]}`,
/// `{"C":[[x1,y1],[x2,y2],[x,y]]}` and `"Z"`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PathCmd {
    /// Starts a new subpath at the point.
    #[serde(rename = "M")]
    MoveTo(Point),
    /// Straight line to the point.
    #[serde(rename = "L")]
    LineTo(Point),
    /// Cubic Bezier curve with two control points and an end point.
    #[serde(rename = "C")]
    CubicTo(Point, Point, Point),
    /// Closes the current subpath with a line back to its start.
    #[serde(rename = "Z")]
    Close,
}

/// A vector path made of lines and cubic Bezier curves, in mm.
///
/// This is the geometry that the user edits. The stitch engine uses
/// [`Path::flatten`] to get polylines.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Path {
    pub cmds: Vec<PathCmd>,
}

/// A sequence of connected points. If `closed` is `true`, the last point
/// connects to the first point. The first point is not repeated at the end.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(mut self, p: Point) -> Self {
        self.cmds.push(PathCmd::MoveTo(p));
        self
    }

    pub fn line_to(mut self, p: Point) -> Self {
        self.cmds.push(PathCmd::LineTo(p));
        self
    }

    pub fn cubic_to(mut self, c1: Point, c2: Point, p: Point) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1, c2, p));
        self
    }

    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    /// Returns a closed polygon path through the points.
    pub fn polygon(points: &[Point]) -> Self {
        let mut path = Path::new();
        for (i, p) in points.iter().enumerate() {
            path = if i == 0 { path.move_to(*p) } else { path.line_to(*p) };
        }
        if !points.is_empty() {
            path = path.close();
        }
        path
    }

    /// Returns a closed rectangle path.
    pub fn rect(bbox: BBox) -> Self {
        Path::polygon(&[
            bbox.min,
            Point::new(bbox.max.x, bbox.min.y),
            bbox.max,
            Point::new(bbox.min.x, bbox.max.y),
        ])
    }

    /// Returns a closed rectangle path with rounded corners of radius `r`.
    /// The radius is limited to half of the shorter side.
    pub fn rounded_rect(bbox: BBox, r: f64) -> Self {
        let r = r.clamp(0.0, bbox.width().min(bbox.height()) / 2.0);
        if r == 0.0 {
            return Path::rect(bbox);
        }
        let k = r * QUARTER_CIRCLE_K;
        let (x0, y0, x1, y1) = (bbox.min.x, bbox.min.y, bbox.max.x, bbox.max.y);
        let p = Point::new;
        Path::new()
            .move_to(p(x0 + r, y0))
            .line_to(p(x1 - r, y0))
            .cubic_to(p(x1 - r + k, y0), p(x1, y0 + r - k), p(x1, y0 + r))
            .line_to(p(x1, y1 - r))
            .cubic_to(p(x1, y1 - r + k), p(x1 - r + k, y1), p(x1 - r, y1))
            .line_to(p(x0 + r, y1))
            .cubic_to(p(x0 + r - k, y1), p(x0, y1 - r + k), p(x0, y1 - r))
            .line_to(p(x0, y0 + r))
            .cubic_to(p(x0, y0 + r - k), p(x0 + r - k, y0), p(x0 + r, y0))
            .close()
    }

    /// Returns a closed ellipse path made of four cubic curves.
    pub fn ellipse(center: Point, rx: f64, ry: f64) -> Self {
        const K: f64 = QUARTER_CIRCLE_K;
        let (cx, cy) = (center.x, center.y);
        let (kx, ky) = (rx * K, ry * K);
        Path::new()
            .move_to(Point::new(cx + rx, cy))
            .cubic_to(
                Point::new(cx + rx, cy + ky),
                Point::new(cx + kx, cy + ry),
                Point::new(cx, cy + ry),
            )
            .cubic_to(
                Point::new(cx - kx, cy + ry),
                Point::new(cx - rx, cy + ky),
                Point::new(cx - rx, cy),
            )
            .cubic_to(
                Point::new(cx - rx, cy - ky),
                Point::new(cx - kx, cy - ry),
                Point::new(cx, cy - ry),
            )
            .cubic_to(
                Point::new(cx + kx, cy - ry),
                Point::new(cx + rx, cy - ky),
                Point::new(cx + rx, cy),
            )
            .close()
    }

    /// Makes a path through digitized points. Each point is a corner or a
    /// curve point (`curve = true`).
    ///
    /// - Two corners in sequence give a straight line.
    /// - A curve point gets a smooth tangent (Catmull-Rom: parallel to the
    ///   line from the point before to the point after), so the path goes
    ///   through it without a kink.
    /// - A corner has no tangent, so the path can turn sharply there.
    ///
    /// This is the usual "left click = corner, right click = curve" input of
    /// digitizing software.
    pub fn through_points(points: &[(Point, bool)], closed: bool) -> Path {
        let n = points.len();
        let mut path = Path::new();
        if n == 0 {
            return path;
        }
        let tangent = |i: usize| -> Point {
            let (p, curve) = points[i];
            if !curve {
                return Point::ZERO;
            }
            let prev = if i > 0 {
                Some(points[i - 1].0)
            } else if closed {
                Some(points[n - 1].0)
            } else {
                None
            };
            let next = if i + 1 < n {
                Some(points[i + 1].0)
            } else if closed {
                Some(points[0].0)
            } else {
                None
            };
            match (prev, next) {
                (Some(a), Some(b)) => (b - a) * 0.5,
                (None, Some(b)) => b - p,
                (Some(a), None) => p - a,
                (None, None) => Point::ZERO,
            }
        };
        path = path.move_to(points[0].0);
        let segs = if closed { n } else { n - 1 };
        for i in 0..segs {
            let j = (i + 1) % n;
            let (a, b) = (points[i].0, points[j].0);
            if !points[i].1 && !points[j].1 {
                path = path.line_to(b);
            } else {
                let c1 = a + tangent(i) * (1.0 / 3.0);
                let c2 = b - tangent(j) * (1.0 / 3.0);
                path = path.cubic_to(c1, c2, b);
            }
        }
        if closed {
            path = path.close();
        }
        path
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    /// Returns each subpath (from one move to the next) as its own path.
    /// Subpaths with no drawing command after the move are left out.
    pub fn subpaths(&self) -> Vec<Path> {
        let mut out = Vec::new();
        let mut cur = Path::new();
        let mut pen = Point::ZERO;
        let mut finish = |cur: &mut Path| {
            let p = std::mem::take(cur);
            if p.cmds.len() > 1 {
                out.push(p);
            }
        };
        for cmd in &self.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    finish(&mut cur);
                    cur = Path::new().move_to(p);
                    pen = p;
                }
                PathCmd::Close => {
                    cur.cmds.push(PathCmd::Close);
                    // A command after a close starts again from the start.
                    let start = match cur.cmds.first() {
                        Some(PathCmd::MoveTo(s)) => *s,
                        _ => pen,
                    };
                    finish(&mut cur);
                    cur = Path::new().move_to(start);
                    pen = start;
                }
                c => {
                    if cur.cmds.is_empty() {
                        cur = Path::new().move_to(pen);
                    }
                    cur.cmds.push(c);
                    pen = match c {
                        PathCmd::LineTo(p) | PathCmd::CubicTo(_, _, p) => p,
                        _ => pen,
                    };
                }
            }
        }
        finish(&mut cur);
        out
    }

    /// Returns `true` if the path ends with a close command.
    pub fn is_closed(&self) -> bool {
        matches!(self.cmds.last(), Some(PathCmd::Close))
    }

    /// Returns the path with a close command at the end.
    pub fn closed(self) -> Path {
        if self.is_closed() { self } else { self.close() }
    }

    /// Returns all subpaths of the paths as one path.
    pub fn joined<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Path {
        Path {
            cmds: paths.into_iter().flat_map(|p| p.cmds.iter().copied()).collect(),
        }
    }

    /// Returns all points in the commands, control points included.
    pub fn points(&self) -> impl Iterator<Item = Point> + '_ {
        self.cmds.iter().flat_map(|c| {
            let pts: [Option<Point>; 3] = match *c {
                PathCmd::MoveTo(p) | PathCmd::LineTo(p) => [Some(p), None, None],
                PathCmd::CubicTo(a, b, p) => [Some(a), Some(b), Some(p)],
                PathCmd::Close => [None, None, None],
            };
            pts.into_iter().flatten()
        })
    }

    /// Returns the bounding box of all points, control points included.
    /// The result can be larger than the exact curve bounds, but it always contains the curve.
    pub fn control_bbox(&self) -> Option<BBox> {
        BBox::from_points(self.points())
    }

    /// Returns a copy of the path with the transform applied to each point.
    pub fn transformed(&self, t: &Transform) -> Path {
        let cmds = self
            .cmds
            .iter()
            .map(|c| match *c {
                PathCmd::MoveTo(p) => PathCmd::MoveTo(t.apply(p)),
                PathCmd::LineTo(p) => PathCmd::LineTo(t.apply(p)),
                PathCmd::CubicTo(a, b, p) => PathCmd::CubicTo(t.apply(a), t.apply(b), t.apply(p)),
                PathCmd::Close => PathCmd::Close,
            })
            .collect();
        Path { cmds }
    }

    /// Converts the path to polylines. Each subpath gives one polyline.
    ///
    /// The distance between a flattened curve and the true curve is not more
    /// than `tolerance` mm. Subpaths with fewer than two distinct points are
    /// discarded. The result is deterministic.
    pub fn flatten(&self, tolerance: f64) -> Vec<Polyline> {
        let tolerance = if tolerance > 0.0 {
            tolerance
        } else {
            crate::DEFAULT_FLATTEN_TOLERANCE_MM
        };
        let mut out = Vec::new();
        let mut cur = Polyline::default();
        let mut pen = Point::ZERO;

        let finish = |cur: &mut Polyline, out: &mut Vec<Polyline>| {
            let mut line = std::mem::take(cur);
            // A closing point on the first point (also one that a full turn
            // of a circle misses by rounding) goes, so the polyline has no
            // closing segment of length zero: a stroke draws a spike there.
            if line.closed
                && line.points.len() > 1
                && line.points[0].distance(line.points[line.points.len() - 1]) <= SAME_POINT_MM
            {
                line.points.pop();
            }
            if line.points.len() >= 2 {
                out.push(line);
            }
        };

        for cmd in &self.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    finish(&mut cur, &mut out);
                    cur.points.push(p);
                    pen = p;
                }
                PathCmd::LineTo(p) => {
                    if cur.points.is_empty() {
                        cur.points.push(pen);
                    }
                    push_distinct(&mut cur.points, p);
                    pen = p;
                }
                PathCmd::CubicTo(c1, c2, p) => {
                    if cur.points.is_empty() {
                        cur.points.push(pen);
                    }
                    flatten_cubic(pen, c1, c2, p, tolerance, 0, &mut cur.points);
                    pen = p;
                }
                PathCmd::Close => {
                    if let Some(&start) = cur.points.first() {
                        cur.closed = true;
                        pen = start;
                    }
                    finish(&mut cur, &mut out);
                }
            }
        }
        finish(&mut cur, &mut out);
        out
    }
}

impl Polyline {
    /// Total length in mm, the closing segment included if the polyline is closed.
    pub fn length(&self) -> f64 {
        let open: f64 = self.points.windows(2).map(|w| w[0].distance(w[1])).sum();
        match (self.closed, self.points.first(), self.points.last()) {
            (true, Some(a), Some(b)) => open + a.distance(*b),
            _ => open,
        }
    }

    /// Returns the segments as point pairs, the closing segment included.
    pub fn segments(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        let n = self.points.len();
        let count = if self.closed && n > 2 { n } else { n.saturating_sub(1) };
        (0..count).map(move |i| (self.points[i], self.points[(i + 1) % n]))
    }
}

/// Douglas-Peucker simplification of an open polyline: removes points that
/// are nearer than `tol` to the line through their neighbors. The ends stay.
pub fn simplify(points: &[Point], tol: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((s, e)) = stack.pop() {
        let mut best = (0.0, 0);
        for i in s + 1..e {
            let d = points[i].distance_to_segment(points[s], points[e]);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tol {
            keep[best.1] = true;
            stack.push((s, best.1));
            stack.push((best.1, e));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// Points nearer than this are one point when a path is flattened
/// (rounding errors of curves, far below a stitch).
const SAME_POINT_MM: f64 = 1e-9;

fn push_distinct(points: &mut Vec<Point>, p: Point) {
    if points.last().is_none_or(|q| q.distance(p) > SAME_POINT_MM) {
        points.push(p);
    }
}

/// Adds the flattened cubic to `out`. `p0` must already be in `out`.
fn flatten_cubic(p0: Point, c1: Point, c2: Point, p3: Point, tol: f64, depth: u32, out: &mut Vec<Point>) {
    let flat = c1.distance_to_segment(p0, p3) <= tol && c2.distance_to_segment(p0, p3) <= tol;
    if flat || depth >= MAX_FLATTEN_DEPTH {
        push_distinct(out, p3);
        return;
    }
    // De Casteljau split at t = 0.5.
    let p01 = p0.midpoint(c1);
    let p12 = c1.midpoint(c2);
    let p23 = c2.midpoint(p3);
    let p012 = p01.midpoint(p12);
    let p123 = p12.midpoint(p23);
    let mid = p012.midpoint(p123);
    flatten_cubic(p0, p01, p012, mid, tol, depth + 1, out);
    flatten_cubic(mid, p123, p23, p3, tol, depth + 1, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closing_point_off_by_rounding_goes() {
        // A full turn of a circle ends 1e-16 mm from its start.
        let p = |x: f64, y: f64| Point::new(x, y);
        let path = Path::new()
            .move_to(p(10.0, 0.0))
            .line_to(p(0.0, 10.0))
            .line_to(p(-10.0, 0.0))
            .line_to(p(10.0, -2.4e-16))
            .close();
        let lines = path.flatten(0.1);
        assert_eq!(lines[0].points.len(), 3, "{:?}", lines[0].points);
        assert!(lines[0].segments().all(|(a, b)| a.distance(b) > 1.0));
    }

    #[test]
    fn subpaths_split_at_moves_and_closes() {
        let p = |x: f64, y: f64| Point::new(x, y);
        let path = Path::new()
            .move_to(p(0.0, 0.0))
            .line_to(p(1.0, 0.0))
            .line_to(p(1.0, 1.0))
            .close()
            .move_to(p(5.0, 5.0))
            .move_to(p(6.0, 6.0))
            .cubic_to(p(7.0, 6.0), p(8.0, 7.0), p(8.0, 8.0));
        let subs = path.subpaths();
        assert_eq!(subs.len(), 2, "a lone move is left out");
        assert!(subs[0].is_closed() && !subs[1].is_closed());
        assert_eq!(subs[1].cmds[0], PathCmd::MoveTo(p(6.0, 6.0)));
        assert_eq!(Path::joined(&subs), {
            let mut q = path.clone();
            q.cmds.remove(4);
            q
        });
        assert!(subs[1].clone().closed().is_closed());
    }
    use proptest::prelude::*;

    #[test]
    fn rect_flattens_to_closed_square() {
        let r = Path::rect(BBox::new(Point::ZERO, Point::new(10.0, 10.0)));
        let lines = r.flatten(0.05);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].closed);
        assert_eq!(lines[0].points.len(), 4);
        assert_eq!(lines[0].length(), 40.0);
    }

    #[test]
    fn circle_flatten_is_within_tolerance() {
        let tol = 0.05;
        let r = 25.0;
        let lines = Path::ellipse(Point::ZERO, r, r).flatten(tol);
        assert_eq!(lines.len(), 1);
        for p in &lines[0].points {
            // The 4-cubic circle itself has error of about 0.027 % of the radius.
            assert!((p.length() - r).abs() <= tol + r * 0.0003, "{p:?}");
        }
        let expected = 2.0 * core::f64::consts::PI * r;
        assert!((lines[0].length() - expected).abs() < 0.1);
    }

    #[test]
    fn simplify_keeps_corners_and_ends() {
        let mut pts: Vec<Point> = (0..=10)
            .map(|i| Point::new(f64::from(i), 0.001 * f64::from(i % 2)))
            .collect();
        pts.push(Point::new(10.0, 10.0));
        let s = simplify(&pts, 0.01);
        assert_eq!(
            s,
            vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0)]
        );
    }

    #[test]
    fn rounded_rect_bounds() {
        let b = BBox::new(Point::ZERO, Point::new(20.0, 10.0));
        let lines = Path::rounded_rect(b, 3.0).flatten(0.01);
        assert_eq!(lines.len(), 1);
        let fb = BBox::from_points(lines[0].points.iter().copied()).unwrap();
        assert!(fb.min.distance(b.min) < 1e-9 && fb.max.distance(b.max) < 1e-9);
        // Corners are cut off.
        assert!(!lines[0].points.contains(&b.min));
        // Perimeter: straight parts plus one full circle.
        let expected = 2.0 * (14.0 + 4.0) + 2.0 * core::f64::consts::PI * 3.0;
        assert!((lines[0].length() - expected).abs() < 0.05);
    }

    #[test]
    fn through_points_corners_and_curves() {
        let p = Point::new;
        // All corners: straight lines only.
        let square = Path::through_points(
            &[(p(0.0, 0.0), false), (p(10.0, 0.0), false), (p(10.0, 10.0), false)],
            true,
        );
        assert!(square.cmds.iter().all(|c| !matches!(c, PathCmd::CubicTo(..))));
        assert_eq!(square.flatten(0.01)[0].points.len(), 3);

        // A curve point between two corners: the path goes through it, smoothly.
        let arc = Path::through_points(
            &[(p(0.0, 0.0), false), (p(5.0, 5.0), true), (p(10.0, 0.0), false)],
            false,
        );
        let line = &arc.flatten(0.01)[0];
        assert_eq!(line.points.first(), Some(&p(0.0, 0.0)));
        assert_eq!(line.points.last(), Some(&p(10.0, 0.0)));
        assert!(line.points.contains(&p(5.0, 5.0)));
        // The tangent at the curve point is horizontal (parallel to corner-to-corner).
        let PathCmd::CubicTo(_, c2, end) = arc.cmds[1] else {
            panic!()
        };
        assert_eq!(end, p(5.0, 5.0));
        assert_eq!(c2.y, 5.0);

        // A closed loop of curve points makes a round shape.
        let circle_pts: Vec<(Point, bool)> = (0..8)
            .map(|i| {
                let a = crate::math::to_radians(f64::from(i) * 45.0);
                (p(10.0 * crate::math::cos(a), 10.0 * crate::math::sin(a)), true)
            })
            .collect();
        let round = Path::through_points(&circle_pts, true).flatten(0.01);
        for q in &round[0].points {
            assert!((q.length() - 10.0).abs() < 0.3, "{q:?}");
        }
        assert!(Path::through_points(&[], false).is_empty());
    }

    #[test]
    fn multiple_subpaths() {
        let p = Path::new()
            .move_to(Point::ZERO)
            .line_to(Point::new(1.0, 0.0))
            .move_to(Point::new(5.0, 5.0))
            .line_to(Point::new(6.0, 5.0))
            .line_to(Point::new(6.0, 6.0))
            .close();
        let lines = p.flatten(0.1);
        assert_eq!(lines.len(), 2);
        assert!(!lines[0].closed);
        assert!(lines[1].closed);
    }

    #[test]
    fn degenerate_subpaths_are_discarded() {
        let p = Path::new().move_to(Point::ZERO).move_to(Point::new(1.0, 1.0)).close();
        assert!(p.flatten(0.1).is_empty());
    }

    #[test]
    fn serialized_form_is_short() {
        let p = Path::new().move_to(Point::ZERO).line_to(Point::new(1.0, 2.0)).close();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, r#"[{"M":[0.0,0.0]},{"L":[1.0,2.0]},"Z"]"#);
        let back: Path = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    fn pt() -> impl Strategy<Value = Point> {
        (-200.0..200.0f64, -200.0..200.0f64).prop_map(|(x, y)| Point::new(x, y))
    }

    proptest! {
        #[test]
        fn cubic_flatten_invariants(p0 in pt(), c1 in pt(), c2 in pt(), p3 in pt(), tol in 0.01..1.0f64) {
            let path = Path::new().move_to(p0).cubic_to(c1, c2, p3);
            let bbox = path.control_bbox().unwrap().expand(1e-9);
            let lines = path.flatten(tol);
            for line in &lines {
                prop_assert_eq!(line.points.first().copied(), Some(p0));
                prop_assert_eq!(line.points.last().copied(), Some(p3));
                for p in &line.points {
                    prop_assert!(p.is_finite());
                    prop_assert!(bbox.contains(*p));
                }
            }
            // Same input gives the same output.
            prop_assert_eq!(lines, path.flatten(tol));
        }
    }
}
