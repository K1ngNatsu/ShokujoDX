//! Sketches from vector paths and DXF drawings (P14 Milestone 6, ADR 0079).
//!
//! The pieces of the drawing become sketch curves: lines, arcs and
//! circles, and spline pieces for the other curves. Ends nearer than the
//! tolerance are one point, so the shapes stay closed; centers nearer than
//! it are one point too (concentric circles and arcs). Then the program
//! finds constraints: horizontal and vertical lines, tangents where two
//! curves meet smoothly, and equal lengths and radii. Constraints that the
//! others make true already are removed, so the sketch has no repeated
//! constraints.
//!
//! DXF: the text form (ASCII) with the entities LINE, ARC, CIRCLE,
//! LWPOLYLINE (with bulges) and POLYLINE with VERTEX; `$INSUNITS` gives the
//! units (else mm). The y axis of a DXF goes up, so the drawing is turned
//! over to the design (y down). The format is public (the DXF reference);
//! the reader is our own code.

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use emb_geom::{Path, PathCmd, Point, math};

use crate::shape::{cubic_at, cubic_d1};
use crate::{Constraint, CurveId, Geometry, PointId, Sketch};

/// A piece of a drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece {
    Line(Point, Point),
    /// Counterclockwise (in sketch coordinates) from `a0` to `a1`
    /// (radians).
    Arc {
        c: Point,
        r: f64,
        a0: f64,
        a1: f64,
    },
    Circle {
        c: Point,
        r: f64,
    },
    Cubic([Point; 4]),
}

/// Why a drawing does not become a sketch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("the file is not a DXF drawing in text form")]
    NotDxf,
    #[error("the drawing has no lines, arcs or circles")]
    Empty,
}

/// The tolerance of the import in mm: ends nearer than this are one point.
pub const TOL: f64 = 0.05;
/// The angle in degrees within which a line is horizontal or vertical, and
/// two curves meet smoothly.
pub const ANGLE_DEG: f64 = 1.0;

/// The pieces of `path`: lines, and its cubic curves as arcs where they are
/// arcs of a circle (within `tol`), else as cubics. Arcs of one circle in a
/// row become one arc, or a circle.
pub fn pieces_of(path: &Path, tol: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    for sub in path.subpaths() {
        let mut ps: Vec<Piece> = Vec::new();
        let (mut cur, mut start) = (None::<Point>, None::<Point>);
        for cmd in &sub.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    cur = Some(p);
                    start = Some(p);
                }
                PathCmd::LineTo(q) => {
                    if let Some(c) = cur
                        && c.distance(q) > tol
                    {
                        ps.push(Piece::Line(c, q));
                    }
                    cur = Some(q);
                }
                PathCmd::CubicTo(c1, c2, q) => {
                    if let Some(c) = cur {
                        ps.push(cubic_piece([c, c1, c2, q], tol));
                    }
                    cur = Some(q);
                }
                PathCmd::Close => {
                    if let (Some(c), Some(s)) = (cur, start)
                        && c.distance(s) > tol
                    {
                        ps.push(Piece::Line(c, s));
                    }
                    cur = start;
                }
            }
        }
        out.extend(join_arcs(ps, tol));
    }
    out
}

/// A cubic as a line (handles on the chord), an arc (on a circle within
/// `tol`), else a cubic.
fn cubic_piece(p: [Point; 4], tol: f64) -> Piece {
    let chord = p[3] - p[0];
    let len = chord.length();
    let off = |q: Point| {
        if len > 1e-12 {
            chord.cross(q - p[0]).abs() / len
        } else {
            q.distance(p[0])
        }
    };
    if len > tol && off(p[1]) <= tol && off(p[2]) <= tol {
        let along = |q: Point| (q - p[0]).dot(chord) / (len * len);
        if (-0.01..=1.01).contains(&along(p[1])) && (-0.01..=1.01).contains(&along(p[2])) {
            return Piece::Line(p[0], p[3]);
        }
    }
    let m = cubic_at(&p, 0.5);
    if let Some(c) = circle_through(p[0], m, p[3]) {
        let r = c.distance(p[0]);
        let on = (1..10).all(|i| (cubic_at(&p, i as f64 / 10.0).distance(c) - r).abs() <= tol);
        if on && r > tol {
            let ang = |q: Point| math::atan2(q.y - c.y, q.x - c.x);
            let ccw = (p[0] - c).cross(cubic_d1(&p, 0.0)) > 0.0;
            let (a0, a1) = if ccw {
                (ang(p[0]), ang(p[3]))
            } else {
                (ang(p[3]), ang(p[0]))
            };
            return Piece::Arc { c, r, a0, a1 };
        }
    }
    Piece::Cubic(p)
}

/// The center of the circle through three points (`None` if they are on a
/// line).
fn circle_through(a: Point, b: Point, c: Point) -> Option<Point> {
    let d = 2.0 * ((b - a).cross(c - a));
    if d.abs() < 1e-12 {
        return None;
    }
    let (ab, ac) = (b - a, c - a);
    let (l1, l2) = (ab.length_sq(), ac.length_sq());
    let ux = (ac.y * l1 - ab.y * l2) / d;
    let uy = (ab.x * l2 - ac.x * l1) / d;
    Some(a + Point::new(ux, uy))
}

/// The sweep of an arc from `a0` to `a1` counterclockwise, in (0, 2π].
fn sweep(a0: f64, a1: f64) -> f64 {
    let s = (a1 - a0).rem_euclid(TAU);
    if s < 1e-12 { TAU } else { s }
}

/// Arcs of one circle that follow each other become one arc, or a circle.
fn join_arcs(ps: Vec<Piece>, tol: f64) -> Vec<Piece> {
    let same = |a: &Piece, b: &Piece| match (a, b) {
        (
            Piece::Arc { c, r, a0, a1 },
            Piece::Arc {
                c: c2,
                r: r2,
                a0: b0,
                a1: b1,
            },
        ) => {
            if c.distance(*c2) > tol || (r - r2).abs() > tol {
                return None;
            }
            // One goes on where the other ends (either way round).
            let at = |a: f64| *c + Point::new(math::cos(a), math::sin(a)) * *r;
            if at(*a1).distance(at(*b0)) <= tol {
                Some(Piece::Arc {
                    c: *c,
                    r: *r,
                    a0: *a0,
                    a1: *a0 + sweep(*a0, *a1) + sweep(*b0, *b1),
                })
            } else if at(*b1).distance(at(*a0)) <= tol {
                Some(Piece::Arc {
                    c: *c,
                    r: *r,
                    a0: *b0,
                    a1: *b0 + sweep(*b0, *b1) + sweep(*a0, *a1),
                })
            } else {
                None
            }
        }
        _ => None,
    };
    let mut out: Vec<Piece> = Vec::new();
    for p in ps {
        if let Some(last) = out.last()
            && let Some(j) = same(last, &p)
        {
            *out.last_mut().expect("a last piece") = j;
            continue;
        }
        out.push(p);
    }
    // The last and the first of a closed shape.
    if out.len() >= 2
        && let Some(j) = same(&out[out.len() - 1], &out[0])
    {
        out.pop();
        out[0] = j;
    }
    out.into_iter()
        .map(|p| match p {
            Piece::Arc { c, r, a0, a1 } if a1 - a0 >= TAU - 1e-6 => Piece::Circle { c, r },
            other => other,
        })
        .collect()
}

/// A sketch of `pieces`, with the constraints that the program finds.
pub fn sketch_of(pieces: &[Piece], tol: f64) -> Sketch {
    let mut s = Sketch::new();
    let mut verts: Vec<(Point, PointId)> = Vec::new();
    let mut vertex = |s: &mut Sketch, p: Point| -> PointId {
        if let Some((_, id)) = verts.iter().find(|(q, _)| q.distance(p) <= tol) {
            return *id;
        }
        let id = s.add_point(p);
        verts.push((p, id));
        id
    };
    let at = |c: Point, r: f64, a: f64| c + Point::new(math::cos(a), math::sin(a)) * r;
    for piece in pieces {
        match *piece {
            Piece::Line(a, b) => {
                let (pa, pb) = (vertex(&mut s, a), vertex(&mut s, b));
                if pa != pb {
                    let _ = s.add_line_between(pa, pb);
                }
            }
            Piece::Arc { c, r, a0, a1 } => {
                let pc = vertex(&mut s, c);
                let (ps, pe) = (vertex(&mut s, at(c, r, a0)), vertex(&mut s, at(c, r, a1)));
                if ps != pe {
                    let _ = s.add_arc_between(pc, ps, pe, r);
                }
            }
            Piece::Circle { c, r } => {
                let pc = vertex(&mut s, c);
                let _ = s.add_circle_on(pc, r);
            }
            Piece::Cubic(p) => {
                let (a, b) = (vertex(&mut s, p[0]), vertex(&mut s, p[3]));
                let (h1, h2) = (s.add_point(p[1]), s.add_point(p[2]));
                let _ = s.add_bezier_between(a, h1, h2, b);
            }
        }
    }
    find_constraints(&mut s, tol);
    s
}

/// The direction of curve `c` away from its end `p` (as the sketch is
/// now).
fn away(s: &Sketch, c: CurveId, p: PointId) -> Option<Point> {
    let pt = |q: PointId| s.point(q);
    let d = match s.curve(c)?.geometry {
        Geometry::Line { a, b } => {
            let o = if a == p { b } else { a };
            pt(o)? - pt(p)?
        }
        Geometry::Bezier { start, c1, c2, .. } => {
            let h = if start == p { c1 } else { c2 };
            let d = pt(h)? - pt(p)?;
            if d.length() > 1e-9 {
                d
            } else {
                // A handle on its end: the direction to the other handle.
                pt(if start == p { c2 } else { c1 })? - pt(p)?
            }
        }
        Geometry::Arc { center, start, .. } => {
            let radial = pt(p)? - pt(center)?;
            // Counterclockwise from the start: forward at the start, back
            // at the end.
            if start == p {
                radial.perp()
            } else {
                radial.perp() * -1.0
            }
        }
        Geometry::Circle { .. } => return None,
    };
    d.normalized()
}

/// Adds the constraints that the drawing shows (see the module text).
fn find_constraints(s: &mut Sketch, tol: f64) {
    let first = s.next_id;
    let k = math::sin(ANGLE_DEG.to_radians());
    let curves: Vec<crate::Curve> = s.curves.clone();
    // Horizontal and vertical lines.
    for c in &curves {
        if let Geometry::Line { a, b } = c.geometry
            && let (Some(pa), Some(pb)) = (s.point(a), s.point(b))
            && let Some(d) = (pb - pa).normalized()
        {
            if d.y.abs() <= k {
                let _ = s.add_constraint(Constraint::Horizontal { line: c.id });
            } else if d.x.abs() <= k {
                let _ = s.add_constraint(Constraint::Vertical { line: c.id });
            }
        }
    }
    // Tangents where two curves (not two lines) meet smoothly.
    let mut at: BTreeMap<PointId, Vec<CurveId>> = BTreeMap::new();
    for c in &curves {
        if let Some((a, b)) = c.ends() {
            at.entry(a).or_default().push(c.id);
            at.entry(b).or_default().push(c.id);
        }
    }
    for (p, list) in &at {
        let [a, b] = list.as_slice() else { continue };
        let lines = |c: &CurveId| s.curve(*c).is_some_and(|c| c.is_line());
        if lines(a) && lines(b) {
            continue;
        }
        if let (Some(u), Some(v)) = (away(s, *a, *p), away(s, *b, *p))
            && u.dot(v) < 0.0
            && u.cross(v).abs() <= k
        {
            let _ = s.add_constraint(Constraint::Tangent {
                a: *a,
                b: *b,
                inside: false,
            });
        }
    }
    // Equal lengths and radii: each one to the first of its size.
    let mut lengths: Vec<(f64, CurveId)> = Vec::new();
    let mut radii: Vec<(f64, CurveId)> = Vec::new();
    for c in &curves {
        let size = match c.geometry {
            Geometry::Line { a, b } => s.point(a).zip(s.point(b)).map(|(p, q)| p.distance(q)),
            _ => s.radius(c.id),
        };
        let Some(v) = size else { continue };
        let list = if c.is_line() {
            &mut lengths
        } else if c.is_round() {
            &mut radii
        } else {
            continue;
        };
        match list.iter().find(|(w, _)| (w - v).abs() <= tol) {
            Some((_, first)) => {
                let _ = s.add_constraint(Constraint::Equal { a: *first, b: c.id });
            }
            None => list.push((v, c.id)),
        }
    }
    // The constraints that the others make true already go. A solve makes
    // the near sizes exact, which can show more of them: again until none.
    for _ in 0..5 {
        s.solve();
        let a = s.analyze();
        let bad = |id: &crate::ConstraintId| id.0 >= first && (a.redundant.contains(id) || a.conflicting.contains(id));
        if !s.constraints.iter().any(|c| bad(&c.id)) {
            break;
        }
        s.constraints.retain(|c| !bad(&c.id));
    }
}

/// A sketch of the paths (design coordinates), with the constraints that
/// the program finds.
pub fn from_paths(paths: &[Path], tol: f64) -> Sketch {
    let pieces: Vec<Piece> = paths.iter().flat_map(|p| pieces_of(p, tol)).collect();
    sketch_of(&pieces, tol)
}

/// The pairs of group codes and values of a DXF in text form.
fn pairs(text: &str) -> Vec<(i32, &str)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines
        .chunks(2)
        .filter_map(|c| match c {
            [code, value] => Some((code.parse().ok()?, *value)),
            _ => None,
        })
        .collect()
}

/// The pieces of a DXF drawing in text form, in mm, with y down.
pub fn dxf_pieces(text: &str) -> Result<Vec<Piece>, ImportError> {
    let p = pairs(text);
    if !p.iter().any(|(c, v)| *c == 0 && *v == "SECTION") {
        return Err(ImportError::NotDxf);
    }
    // The units: $INSUNITS 1 inch, 2 foot, 4 mm, 5 cm, 6 m.
    let mut k = 1.0;
    if let Some(i) = p.iter().position(|(c, v)| *c == 9 && *v == "$INSUNITS")
        && let Some((70, u)) = p.get(i + 1)
    {
        k = match u.trim() {
            "1" => 25.4,
            "2" => 304.8,
            "5" => 10.0,
            "6" => 1000.0,
            _ => 1.0,
        };
    }
    // Only the ENTITIES section.
    let start = p
        .windows(2)
        .position(|w| w[0] == (0, "SECTION") && w[1] == (2, "ENTITIES"))
        .ok_or(ImportError::Empty)?;
    let mut out = Vec::new();
    let mut i = start + 2;
    let num = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);
    let flip = |x: f64, y: f64| Point::new(x * k, -y * k);
    // An arc of the drawing (y up, counterclockwise from d0 to d1 degrees)
    // in sketch coordinates (y down): counterclockwise there from −d1 to −d0.
    let arc = |c: Point, r: f64, d0: f64, d1: f64| Piece::Arc {
        c,
        r,
        a0: (-d1).to_radians(),
        a1: (-d1).to_radians() + sweep(d0.to_radians(), d1.to_radians()),
    };
    while i < p.len() {
        let (code, value) = p[i];
        if code == 0 && value == "ENDSEC" {
            break;
        }
        if code != 0 {
            i += 1;
            continue;
        }
        // The group codes of this entity, to the next 0.
        let mut j = i + 1;
        while j < p.len() && p[j].0 != 0 {
            j += 1;
        }
        let g = &p[i + 1..j];
        let get = |c: i32| g.iter().find(|(x, _)| *x == c).map(|(_, v)| num(v));
        match value {
            "LINE" => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (get(10), get(20), get(11), get(21)) {
                    out.push(Piece::Line(flip(x1, y1), flip(x2, y2)));
                }
            }
            "CIRCLE" => {
                if let (Some(x), Some(y), Some(r)) = (get(10), get(20), get(40)) {
                    out.push(Piece::Circle {
                        c: flip(x, y),
                        r: r * k,
                    });
                }
            }
            "ARC" => {
                if let (Some(x), Some(y), Some(r), Some(d0), Some(d1)) = (get(10), get(20), get(40), get(50), get(51)) {
                    out.push(arc(flip(x, y), r * k, d0, d1));
                }
            }
            "LWPOLYLINE" => {
                let closed = get(70).is_some_and(|f| (f as i64) & 1 == 1);
                // Each vertex: 10, 20 and a bulge 42 (the tangent of a
                // quarter of the angle of the arc to the next vertex).
                let mut verts: Vec<(Point, f64)> = Vec::new();
                for (c, v) in g {
                    match c {
                        10 => verts.push((Point::new(num(v), 0.0), 0.0)),
                        20 => {
                            if let Some(last) = verts.last_mut() {
                                last.0.y = num(v);
                            }
                        }
                        42 => {
                            if let Some(last) = verts.last_mut() {
                                last.1 = num(v);
                            }
                        }
                        _ => {}
                    }
                }
                out.extend(polyline(&verts, closed, k));
            }
            "POLYLINE" => {
                let closed = get(70).is_some_and(|f| (f as i64) & 1 == 1);
                let mut verts: Vec<(Point, f64)> = Vec::new();
                let mut m = j;
                while m < p.len() && p[m] == (0, "VERTEX") {
                    let mut n = m + 1;
                    while n < p.len() && p[n].0 != 0 {
                        n += 1;
                    }
                    let vg = &p[m + 1..n];
                    let vget = |c: i32| vg.iter().find(|(x, _)| *x == c).map(|(_, v)| num(v));
                    verts.push((
                        Point::new(vget(10).unwrap_or(0.0), vget(20).unwrap_or(0.0)),
                        vget(42).unwrap_or(0.0),
                    ));
                    m = n;
                }
                out.extend(polyline(&verts, closed, k));
                j = m;
            }
            _ => {}
        }
        i = j;
    }
    if out.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(out)
}

/// The pieces of a polyline of the drawing (y up, units `k` to mm): lines,
/// and arcs where a vertex has a bulge.
fn polyline(verts: &[(Point, f64)], closed: bool, k: f64) -> Vec<Piece> {
    let n = verts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    let flip = |p: Point| Point::new(p.x * k, -p.y * k);
    let mut out = Vec::new();
    for i in 0..count {
        let (a, bulge) = verts[i];
        let b = verts[(i + 1) % n].0;
        if bulge.abs() < 1e-9 {
            out.push(Piece::Line(flip(a), flip(b)));
            continue;
        }
        // The included angle and the center, in the drawing (y up).
        let theta = 4.0 * math::atan2(bulge, 1.0);
        let chord = b - a;
        let c_len = chord.length();
        if c_len < 1e-12 {
            continue;
        }
        let r = c_len / (2.0 * math::sin(theta.abs() / 2.0));
        let mid = a.lerp(b, 0.5);
        let h = (r * r - (c_len / 2.0).powi(2)).max(0.0).sqrt();
        let n_left = chord.perp() * (1.0 / c_len);
        // A bulge > 0 goes counterclockwise; the center is on the left for
        // an angle under 180°.
        let side = if (bulge > 0.0) == (theta.abs() < PI) { 1.0 } else { -1.0 };
        let center = mid + n_left * (h * side);
        let d = |p: Point| math::to_degrees(math::atan2(p.y - center.y, p.x - center.x));
        let (d0, d1) = if bulge > 0.0 { (d(a), d(b)) } else { (d(b), d(a)) };
        let c = flip(center);
        out.push(Piece::Arc {
            c,
            r: r.abs() * k,
            a0: (-d1).to_radians(),
            a1: (-d1).to_radians() + sweep(d0.to_radians(), d1.to_radians()),
        });
    }
    join_arcs(out, TOL)
}

/// A sketch of a DXF drawing in text form.
pub fn from_dxf(text: &str) -> Result<Sketch, ImportError> {
    Ok(sketch_of(&dxf_pieces(text)?, TOL))
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn count(s: &Sketch, f: impl Fn(&Constraint) -> bool) -> usize {
        s.constraints.iter().filter(|c| f(&c.constraint)).count()
    }

    #[test]
    fn a_rounded_rectangle_and_a_circle_become_lines_arcs_and_constraints() {
        // A logo: a rounded rectangle (arcs as cubics), a circle in it and
        // a wave (a free curve).
        let frame = Path::rounded_rect(BBox::new(p(0.0, 0.0), p(60.0, 40.0)), 6.0);
        let dot = Path::ellipse(p(30.0, 20.0), 8.0, 8.0);
        let wave = Path::new()
            .move_to(p(5.0, 35.0))
            .cubic_to(p(15.0, 25.0), p(25.0, 45.0), p(35.0, 35.0));
        let s = from_paths(&[frame, dot, wave], TOL);
        let kinds = |f: fn(&crate::Curve) -> bool| s.curves.iter().filter(|c| f(c)).count();
        assert_eq!(kinds(|c| c.is_line()), 4, "{:?}", s.curves);
        assert_eq!(kinds(|c| matches!(c.geometry, Geometry::Arc { .. })), 4);
        assert_eq!(kinds(|c| matches!(c.geometry, Geometry::Circle { .. })), 1);
        assert_eq!(kinds(|c| c.is_spline()), 1);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Horizontal { .. })), 2);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Vertical { .. })), 2);
        assert_eq!(count(&s, |c| matches!(c, Constraint::Tangent { .. })), 8);
        assert!(count(&s, |c| matches!(c, Constraint::Equal { .. })) >= 3);
        // It solves, with no repeated or conflicting constraints.
        let mut t = s.clone();
        assert!(t.solve().converged);
        let a = t.analyze();
        assert!(a.redundant.is_empty() && a.conflicting.is_empty(), "{a:?}");
        // The closed shapes are regions: the frame with the dot as a hole,
        // and the dot.
        assert_eq!(s.regions().len(), 2);
    }

    const DXF: &str = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\n0\n10\n0\n20\n0\n11\n40\n21\n0\n\
0\nLINE\n8\n0\n10\n40\n20\n0\n11\n40\n21\n20\n\
0\nARC\n8\n0\n10\n20\n20\n20\n40\n20\n50\n0\n51\n180\n\
0\nLINE\n8\n0\n10\n0\n20\n20\n11\n0\n21\n0\n\
0\nCIRCLE\n8\n0\n10\n20\n20\n20\n40\n5\n\
0\nLWPOLYLINE\n8\n0\n90\n2\n70\n1\n10\n60\n20\n0\n42\n1\n10\n80\n20\n0\n42\n1\n\
0\nENDSEC\n0\nEOF\n";

    #[test]
    fn a_dxf_drawing_becomes_a_sketch() {
        let s = from_dxf(DXF).unwrap();
        // An arch: three lines and a half circle on top (y down: above is
        // negative y), the circle in it, and a circle of two arcs.
        let arcs: Vec<_> = s
            .curves
            .iter()
            .filter(|c| matches!(c.geometry, Geometry::Arc { .. }))
            .collect();
        assert_eq!(arcs.len(), 1, "{:?}", s.curves);
        let circles = s
            .curves
            .iter()
            .filter(|c| matches!(c.geometry, Geometry::Circle { .. }))
            .count();
        assert_eq!(circles, 2, "the circle and the polyline of two half circles");
        // The top of the arch is up (y down: negative y).
        let arch_top = s.shape(arcs[0].id).unwrap().nearest(p(20.0, -40.0)).0;
        assert!((arch_top - p(20.0, -40.0)).length() < 1e-6, "{arch_top:?}");
        // The arch and its circle share the center: concentric.
        let r = s.regions();
        assert_eq!(r.len(), 3, "the arch, its circle, the polyline circle");
        let arch = r.iter().find(|r| !r.holes.is_empty()).unwrap();
        let expected = 40.0 * 20.0 + PI * 200.0 - PI * 25.0;
        assert!((arch.area - expected).abs() < 0.5, "{} {expected}", arch.area);
        assert!(count(&s, |c| matches!(c, Constraint::Tangent { .. })) >= 2);
        // Inches.
        let inch = DXF.replace("$INSUNITS\n70\n4", "$INSUNITS\n70\n1");
        let s = from_dxf(&inch).unwrap();
        let big = s.regions().iter().map(|r| r.area).fold(0.0, f64::max);
        assert!(big > 25.4 * 25.4 * 100.0);
        assert_eq!(from_dxf("hello"), Err(ImportError::NotDxf));
    }
}
