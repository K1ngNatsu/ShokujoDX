//! Click-to-stitch: embroidery objects from one traced color area (P6
//! Milestone 6; ADR 0028).
//!
//! - **Fill**: a fill with the outline and the holes of the area; or with
//!   no holes (the holes sew over).
//! - **Satin**: satin columns along the middle lines of the area, from the
//!   stroke finder of the satin letters (`emb_letter::stroke`), so narrow
//!   areas and letters share one method.
//! - **Center line**: a running line along the same middle lines.
//! - **Outline**: a running line round the outline and the holes.

use emb_geom::{BBox, Path, Point};
use emb_model::{CurrentProperties, ObjectKind};

/// Tolerance to flatten the area outline for the stroke finder, in mm.
const FLATTEN_MM: f64 = 0.05;
/// A rail point this near the outline counts as inside, in mm.
const ON_EDGE_MM: f64 = 0.05;

/// What a click on a color area makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClickStitch {
    #[default]
    Fill,
    FillNoHoles,
    Satin,
    CenterLine,
    Outline,
}

impl ClickStitch {
    pub const ALL: [ClickStitch; 5] = [
        ClickStitch::Fill,
        ClickStitch::FillNoHoles,
        ClickStitch::Satin,
        ClickStitch::CenterLine,
        ClickStitch::Outline,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ClickStitch::Fill => "Click fill",
            ClickStitch::FillNoHoles => "Click fill, no holes",
            ClickStitch::Satin => "Click satin",
            ClickStitch::CenterLine => "Click center line",
            ClickStitch::Outline => "Click outline",
        }
    }
}

/// A closed ring of points from a closed path, with the area on the left
/// (`outer`) or on the right (a hole), so that nonzero winding gives the area.
fn ring(p: &Path, outer: bool) -> Option<Vec<Point>> {
    let mut pts = p.flatten(FLATTEN_MM).into_iter().next()?.points;
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len();
    let twice: f64 = (0..n).map(|i| pts[i].cross(pts[(i + 1) % n])).sum();
    if (twice > 0.0) != outer {
        pts.reverse();
    }
    Some(pts)
}

/// The embroidery objects that `how` makes from an area with the outline
/// `outer` and the `holes` (in mm), with the current properties. Empty if
/// nothing can be made.
pub fn objects(outer: &Path, holes: &[Path], how: ClickStitch, current: &CurrentProperties) -> Vec<ObjectKind> {
    match how {
        ClickStitch::Fill | ClickStitch::FillNoHoles => vec![ObjectKind::Fill {
            outer: outer.clone(),
            holes: if how == ClickStitch::Fill {
                holes.to_vec()
            } else {
                Vec::new()
            },
            params: current.fill.clone(),
        }],
        ClickStitch::Outline => vec![ObjectKind::Running {
            path: Path::joined(std::iter::once(outer).chain(holes)),
            params: current.running.clone(),
        }],
        ClickStitch::Satin => satins(outer, holes)
            .into_iter()
            .map(|geometry| ObjectKind::Satin {
                geometry,
                params: current.satin.clone(),
            })
            .collect(),
        ClickStitch::CenterLine => {
            let lines: Vec<Path> = satins(outer, holes)
                .iter()
                .filter(|g| g.rungs.len() >= 2)
                .map(|g| {
                    let mid: Vec<Point> = g.rungs.iter().map(|(a, b)| a.lerp(*b, 0.5)).collect();
                    let mid = emb_geom::path::simplify(&mid, FLATTEN_MM);
                    let mut p = Path::new().move_to(mid[0]);
                    for q in &mid[1..] {
                        p = p.line_to(*q);
                    }
                    if g.rail_a.is_closed() { p.close() } else { p }
                })
                .collect();
            if lines.is_empty() {
                return Vec::new();
            }
            vec![ObjectKind::Running {
                path: Path::joined(&lines),
                params: current.running.clone(),
            }]
        }
    }
}

/// Satin columns along the middle lines of the area.
fn satins(outer: &Path, holes: &[Path]) -> Vec<emb_model::SatinGeometry> {
    let mut rings: Vec<Vec<Point>> = ring(outer, true).into_iter().collect();
    if rings.is_empty() {
        return Vec::new();
    }
    rings.extend(holes.iter().filter_map(|h| ring(h, false)));
    let Some(bb) = BBox::from_points(rings.iter().flatten().copied()) else {
        return Vec::new();
    };
    // The grid of the stroke finder has GRID_CELLS cells for its size: about
    // 16 cells across a typical width (twice the area over the length of
    // the edge), and at most 800 along the area.
    let area: f64 = rings
        .iter()
        .map(|r| (0..r.len()).map(|i| r[i].cross(r[(i + 1) % r.len()])).sum::<f64>() / 2.0)
        .sum();
    let edge: f64 = rings
        .iter()
        .map(|r| (0..r.len()).map(|i| r[i].distance(r[(i + 1) % r.len()])).sum::<f64>())
        .sum();
    let width = if edge > 0.0 { 2.0 * area.abs() / edge } else { 0.0 };
    let cell = (width / 16.0).max(bb.width().max(bb.height()) / 800.0).max(1e-3);
    let mut satins = emb_letter::stroke::strokes(&rings, cell * emb_letter::stroke::GRID_CELLS).satins;
    for g in &mut satins {
        keep_inside(g, &rings);
    }
    satins
}

/// Pulls each rail point that is outside the area back to the outline,
/// along the line from the middle of its rung. The stroke finder gives the
/// end of a column the width just before it: right for the square ends of
/// letters, but it sticks out of a round end.
fn keep_inside(g: &mut emb_model::SatinGeometry, rings: &[Vec<Point>]) {
    let a: Vec<Point> = g.rail_a.points().collect();
    let b: Vec<Point> = g.rail_b.points().collect();
    if a.len() != b.len() {
        return;
    }
    let segs: Vec<(Point, Point)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |k| (r[k], r[(k + 1) % r.len()])))
        .collect();
    let inside = |p: Point| {
        let crossings = segs
            .iter()
            .filter(|(s, e)| (s.y > p.y) != (e.y > p.y) && p.x < s.x + (p.y - s.y) * (e.x - s.x) / (e.y - s.y))
            .count();
        crossings % 2 == 1 || segs.iter().any(|(s, e)| p.distance_to_segment(*s, *e) < ON_EDGE_MM)
    };
    // The nearest point of the outline.
    let nearest = |p: Point| -> Point {
        segs.iter()
            .map(|&(s, e)| {
                let f = e - s;
                let len2 = f.dot(f);
                let t = if len2 > 0.0 {
                    ((p - s).dot(f) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                s + f * t
            })
            .min_by(|x, y| x.distance(p).total_cmp(&y.distance(p)))
            .unwrap_or(p)
    };
    // The first point of the outline on the line from `m` to `p` (the
    // nearest point of the outline if `m` is outside too).
    let pull = |m: Point, p: Point| -> Point {
        if inside(p) {
            return p;
        }
        if !inside(m) {
            return nearest(p);
        }
        let d = p - m;
        let t = segs
            .iter()
            .filter_map(|&(s, e)| {
                let f = e - s;
                let den = d.cross(f);
                if den.abs() < 1e-12 {
                    return None;
                }
                let t = (s - m).cross(f) / den;
                let u = (s - m).cross(d) / den;
                ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
            })
            .fold(f64::INFINITY, f64::min);
        if t.is_finite() { m + d * t } else { m }
    };
    let mid: Vec<Point> = a.iter().zip(&b).map(|(p, q)| p.lerp(*q, 0.5)).collect();
    let na: Vec<Point> = a.iter().zip(&mid).map(|(p, m)| pull(*m, *p)).collect();
    let nb: Vec<Point> = b.iter().zip(&mid).map(|(p, m)| pull(*m, *p)).collect();
    let moved = |old: &[Point], new: &[Point], p: Point| old.iter().position(|q| *q == p).map_or(p, |i| new[i]);
    for (p, q) in &mut g.rungs {
        *p = moved(&a, &na, *p);
        *q = moved(&b, &nb, *q);
    }
    let path = |p: &[Point], closed: bool| {
        if closed {
            return Path::polygon(p);
        }
        let mut path = Path::new().move_to(p[0]);
        for &q in &p[1..] {
            path = path.line_to(q);
        }
        path
    };
    g.rail_a = path(&na, g.rail_a.is_closed());
    g.rail_b = path(&nb, g.rail_b.is_closed());
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_geom::BBox;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
        Path::rect(BBox::new(Point::new(x0, y0), Point::new(x1, y1)))
    }

    #[test]
    fn each_kind_of_click_makes_its_object() {
        let current = CurrentProperties::default();
        let outer = rect(0.0, 0.0, 40.0, 4.0);
        let hole = rect(18.0, 1.0, 22.0, 3.0);
        let fill = objects(&outer, std::slice::from_ref(&hole), ClickStitch::Fill, &current);
        assert!(matches!(&fill[..], [ObjectKind::Fill { holes, .. }] if holes.len() == 1));
        let no_holes = objects(&outer, std::slice::from_ref(&hole), ClickStitch::FillNoHoles, &current);
        assert!(matches!(&no_holes[..], [ObjectKind::Fill { holes, .. }] if holes.is_empty()));
        let outline = objects(&outer, std::slice::from_ref(&hole), ClickStitch::Outline, &current);
        assert!(matches!(&outline[..], [ObjectKind::Running { path, .. }] if path.subpaths().len() == 2));

        // A bar gives one satin column along it, as wide as the bar.
        let satin = objects(&outer, &[], ClickStitch::Satin, &current);
        let [ObjectKind::Satin { geometry, .. }] = &satin[..] else {
            panic!("{satin:?}");
        };
        let widths: Vec<f64> = geometry.rungs.iter().map(|(a, b)| a.distance(*b)).collect();
        let mid = widths[widths.len() / 2];
        assert!((mid - 4.0).abs() < 0.4, "{widths:?}");
        // The center line runs along the middle of the bar.
        let line = objects(&outer, &[], ClickStitch::CenterLine, &current);
        let [ObjectKind::Running { path, .. }] = &line[..] else {
            panic!("{line:?}");
        };
        let pts: Vec<Point> = path.points().collect();
        assert!(pts.iter().all(|p| (p.y - 2.0).abs() < 0.3), "{pts:?}");
        let len = pts[0].distance(pts[pts.len() - 1]);
        assert!(len > 30.0, "{len}");
    }

    #[test]
    fn satin_rails_stay_in_a_bar_with_round_ends() {
        // A bar 30 mm long and 4 mm wide with half-circle ends, on a slant.
        let (a, b) = (Point::new(0.0, 10.0), Point::new(30.0, 3.0));
        let d = (b - a).normalized().unwrap();
        let n = d.perp();
        let mut pts = Vec::new();
        for (c, from) in [(b, -90.0f64), (a, 90.0)] {
            for k in 0..=12 {
                let t = (from + 15.0 * f64::from(k)).to_radians();
                pts.push(c + (d * t.cos() + n * t.sin()) * 2.0);
            }
        }
        let outer = Path::polygon(&pts);
        let satin = objects(&outer, &[], ClickStitch::Satin, &CurrentProperties::default());
        let [ObjectKind::Satin { geometry, .. }] = &satin[..] else {
            panic!("{satin:?}");
        };
        let ring = ring(&outer, true).unwrap();
        let near = |p: Point| {
            (0..ring.len())
                .map(|i| p.distance_to_segment(ring[i], ring[(i + 1) % ring.len()]))
                .fold(f64::MAX, f64::min)
        };
        for p in geometry.rail_a.points().chain(geometry.rail_b.points()) {
            // Inside: nearer to the middle line than the half width.
            let off = p.distance_to_segment(a, b);
            assert!(off <= 2.0 + 0.1 || near(p) < 0.1, "{p:?} is {off} from the middle");
        }
    }
}
