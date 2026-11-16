//! Made styles and the underline of lettering (v0.3 plan, Stage 5).
//!
//! A font with a bold or an italic face uses it (the app selects the
//! face). For a font with none, the layout makes the style:
//!
//! - Bold: each satin column is wider (each rail goes out from the other
//!   rail) and each fill is larger (its outline goes out), by
//!   [`bold_width`] on each side. The advance of each letter gets the
//!   two sides, so the letters do not touch.
//! - Italic: the letters slant by [`ITALIC_DEG`] about their baseline.
//!
//! An underline is a satin bar under each line of horizontal text, along
//! the baseline (also a curved one).

use emb_geom::region::{self, Join};
use emb_geom::{Path, Point, Transform, math};
use emb_model::{Baseline, ObjectKind, SatinGeometry, Underline};

/// The slant of a made italic, in degrees.
pub const ITALIC_DEG: f64 = 12.0;
/// Tolerance to flatten rails and outlines, in mm.
const TOL_MM: f64 = 0.05;
/// The distance between the points of an underline, in mm.
const UNDERLINE_STEP_MM: f64 = 1.0;

/// How far a made bold makes each side of a stroke go out, in mm, for
/// letters of capital height `height_mm`.
pub fn bold_width(height_mm: f64) -> f64 {
    (0.04 * height_mm).clamp(0.15, 0.8)
}

/// The slant of a made italic: the top of a letter goes to the right
/// (lettering space has y down; the baseline is y = 0).
pub fn italic() -> Transform {
    Transform {
        a: 1.0,
        b: 0.0,
        c: -math::tan(math::to_radians(ITALIC_DEG)),
        d: 1.0,
        e: 0.0,
        f: 0.0,
    }
}

/// Makes the strokes of one letter part wider by `w` on each side: the
/// rails of a satin column go out, the outline of a fill goes out. Other
/// parts (lines) do not change.
pub fn embolden(kind: &mut ObjectKind, w: f64) {
    match kind {
        ObjectKind::Satin { geometry, .. } => *geometry = wider_column(geometry, w),
        ObjectKind::Fill { .. } => {
            let Some(area) = emb_model::shaping::area_of(kind) else {
                return;
            };
            let grown = region::offset(&area, w, Join::Round);
            if grown.is_empty() {
                return;
            }
            if let ObjectKind::Fill { outer, holes, .. } = kind {
                let outers: Vec<Path> = grown.iter().map(|r| Path::polygon(&r.outer)).collect();
                *outer = Path::joined(&outers);
                *holes = grown
                    .iter()
                    .flat_map(|r| r.holes.iter().map(|h| Path::polygon(h)))
                    .collect();
            }
        }
        _ => {}
    }
}

/// The column `g` with each rail `w` farther from the other rail.
fn wider_column(g: &SatinGeometry, w: f64) -> SatinGeometry {
    let flat = |p: &Path| p.flatten(TOL_MM).into_iter().next();
    let (Some(a), Some(b)) = (flat(&g.rail_a), flat(&g.rail_b)) else {
        return g.clone();
    };
    // Each rail point goes out from the other rail.
    let out = |pts: &[Point], other: &[Point], other_closed: bool| -> Vec<Point> {
        pts.iter()
            .map(
                |p| match nearest(other, other_closed, *p).and_then(|q| (*p - q).normalized()) {
                    Some(d) => *p + d * w,
                    None => *p,
                },
            )
            .collect()
    };
    let mut na = out(&a.points, &b.points, b.closed);
    let mut nb = out(&b.points, &a.points, a.closed);
    let open = !a.closed && !b.closed;
    // The column also grows at its ends, along the column, so a stroke
    // grows the same on all sides (a stem gets as tall as the bowls that
    // meet it).
    if open && na.len() >= 2 && nb.len() >= 2 {
        let mid = |x: Point, y: Point| (x + y) * 0.5;
        let (la, lb) = (na.len() - 1, nb.len() - 1);
        if let Some(d) = (mid(na[0], nb[0]) - mid(na[1], nb[1])).normalized() {
            na[0] += d * w;
            nb[0] += d * w;
        }
        if let Some(d) = (mid(na[la], nb[lb]) - mid(na[la - 1], nb[lb - 1])).normalized() {
            na[la] += d * w;
            nb[lb] += d * w;
        }
    }
    let path = |pts: &[Point], closed: bool| if closed { Path::polygon(pts) } else { open_path(pts) };
    let rail_a = path(&na, a.closed);
    let rail_b = path(&nb, b.closed);
    // The stitch direction lines keep their ends on the new rails.
    let rungs = g
        .rungs
        .iter()
        .map(|&(p, q)| {
            let d = (p - q).normalized().unwrap_or(Point::ZERO);
            let p2 = nearest(&na, a.closed, p + d * w).unwrap_or(p);
            let q2 = nearest(&nb, b.closed, q - d * w).unwrap_or(q);
            (p2, q2)
        })
        .collect();
    SatinGeometry { rail_a, rail_b, rungs }
}

/// The nearest point to `p` on the polyline `pts`.
fn nearest(pts: &[Point], closed: bool, p: Point) -> Option<Point> {
    let n = pts.len();
    if n == 1 {
        return Some(pts[0]);
    }
    let segs = if closed { n } else { n.saturating_sub(1) };
    (0..segs)
        .map(|i| p.nearest_on_segment(pts[i], pts[(i + 1) % n]))
        .min_by(|x, y| x.distance(p).total_cmp(&y.distance(p)))
}

fn open_path(pts: &[Point]) -> Path {
    let mut path = Path::new();
    if let Some((first, rest)) = pts.split_first() {
        path = path.move_to(*first);
        for p in rest {
            path = path.line_to(*p);
        }
    }
    path
}

/// The underline of the line at `y` from `x0` to `x1` (lettering space,
/// before the envelope): a satin bar `u.gap_mm` under the baseline,
/// `u.thickness_mm` wide, along `baseline`.
pub fn underline(baseline: &Baseline, y: f64, x0: f64, x1: f64, u: &Underline) -> Option<SatinGeometry> {
    if !(x1 > x0 && u.thickness_mm > 0.0) {
        return None;
    }
    let n = (((x1 - x0) / UNDERLINE_STEP_MM).ceil() as usize).clamp(1, 4000);
    let rail = |dy: f64| -> Path {
        let pts: Vec<Point> = (0..=n)
            .map(|i| {
                let x = x0 + (x1 - x0) * i as f64 / n as f64;
                baseline.letter_transform(x, y, 0.0).apply(Point::new(x, y + dy))
            })
            .collect();
        open_path(&pts)
    };
    let gap = u.gap_mm.max(0.0);
    Some(SatinGeometry {
        rail_a: rail(gap),
        rail_b: rail(gap + u.thickness_mm),
        rungs: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// The width of column `g` at the middle point of rail A.
    fn width_at_middle(g: &SatinGeometry) -> f64 {
        let a = g.rail_a.flatten(TOL_MM).remove(0).points;
        let b = g.rail_b.flatten(TOL_MM).remove(0).points;
        let mid = a[a.len() / 2];
        mid.distance(nearest(&b, false, mid).unwrap())
    }

    #[test]
    fn bold_makes_columns_wider_and_fills_larger() {
        let g = SatinGeometry::from_centerline(&open_path(&[p(0.0, 0.0), p(0.0, -10.0)]), 2.0);
        let mut k = ObjectKind::Satin {
            geometry: g,
            params: Default::default(),
        };
        embolden(&mut k, 0.5);
        let ObjectKind::Satin { geometry, .. } = &k else {
            panic!()
        };
        assert!(
            (width_at_middle(geometry) - 3.0).abs() < 0.05,
            "{}",
            width_at_middle(geometry)
        );
        let mut f = ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(0.0, 0.0), p(4.0, 4.0))),
            holes: Vec::new(),
            params: Default::default(),
        };
        embolden(&mut f, 0.5);
        let area: f64 = emb_model::shaping::area_of(&f).unwrap().iter().map(|r| r.area()).sum();
        assert!(area > 24.0 && area < 25.0, "{area}");
    }

    #[test]
    fn italic_slants_the_top_to_the_right() {
        let t = italic();
        assert_eq!(t.apply(p(0.0, 0.0)), p(0.0, 0.0));
        let top = t.apply(p(0.0, -10.0));
        assert!(top.x > 2.0 && top.x < 2.2, "{top:?}");
    }

    #[test]
    fn an_underline_goes_under_the_baseline() {
        let u = Underline {
            gap_mm: 1.0,
            thickness_mm: 2.0,
        };
        let g = underline(&Baseline::Straight, 0.0, 0.0, 30.0, &u).unwrap();
        let a = g.rail_a.flatten(TOL_MM).remove(0).points;
        let b = g.rail_b.flatten(TOL_MM).remove(0).points;
        assert!(a.iter().all(|q| (q.y - 1.0).abs() < 1e-9));
        assert!(b.iter().all(|q| (q.y - 3.0).abs() < 1e-9));
        assert!((a[0].x).abs() < 1e-9 && (a.last().unwrap().x - 30.0).abs() < 1e-9);
        assert!(underline(&Baseline::Straight, 0.0, 5.0, 5.0, &u).is_none());
    }
}
