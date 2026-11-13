//! Lettering knockout (ADR 0093): the stitches of a lettering leave out
//! the area that objects in front cut (`Lettering::knockout`).
//!
//! - A fill keeps its area outside the cut.
//! - A satin column stays a satin column. Its rungs are sampled each
//!   [`RUNG_STEP_MM`]; each rung keeps its longest part outside the cut. A
//!   new column starts where a rung has no part, or where its part does not
//!   touch the part of the rung before it.
//! - A line keeps its parts outside the cut.
//!
//! A letter with no part left does not sew.

use emb_geom::region::Region;
use emb_geom::{BBox, Path, Point, Polyline};
use emb_model::{Letter, Lettering, ObjectKind, SatinGeometry};

use crate::satin::Column;
use crate::travel::Shape;

/// The distance between the sampled rungs of a satin column, in mm.
const RUNG_STEP_MM: f64 = 0.15;
/// A rung part narrower than this does not sew, in mm.
const MIN_WIDTH_MM: f64 = 0.2;
/// Each this many sampled rungs, one becomes a rung of the new column.
const RUNG_EVERY: usize = 5;
/// A line part shorter than this does not sew, in mm.
const MIN_LINE_MM: f64 = 0.3;
/// Fill pieces smaller than this do not sew, in mm².
const MIN_AREA_MM2: f64 = 0.05;
/// Tolerance to flatten outlines, in mm.
const FLATTEN_TOL_MM: f64 = 0.05;

/// The cut area, with the box of each region for quick tests.
struct Cut {
    regions: Vec<Region>,
    boxes: Vec<BBox>,
    all: BBox,
}

impl Cut {
    fn new(regions: Vec<Region>) -> Option<Cut> {
        let boxes: Vec<BBox> = regions.iter().filter_map(Region::bbox).collect();
        let all = boxes.iter().copied().reduce(BBox::union)?;
        Some(Cut { regions, boxes, all })
    }

    fn contains(&self, p: Point) -> bool {
        self.regions
            .iter()
            .zip(&self.boxes)
            .any(|(r, b)| b.contains(p) && r.contains(p))
    }

    /// The parts of the segment `a`-`b` outside the cut, as (from, to)
    /// fractions of the segment, in order.
    fn outside(&self, a: Point, b: Point) -> Vec<(f64, f64)> {
        let seg = BBox::new(
            Point::new(a.x.min(b.x), a.y.min(b.y)),
            Point::new(a.x.max(b.x), a.y.max(b.y)),
        );
        let mut ts = vec![0.0, 1.0];
        for (r, bx) in self.regions.iter().zip(&self.boxes) {
            if !bx.intersects(&seg) {
                continue;
            }
            for ring in std::iter::once(&r.outer).chain(&r.holes) {
                for (i, &c) in ring.iter().enumerate() {
                    let d = ring[(i + 1) % ring.len()];
                    if let Some((u, _)) = Point::crossing(a, b, c, d) {
                        ts.push(u);
                    }
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        let mut out: Vec<(f64, f64)> = Vec::new();
        for w in ts.windows(2) {
            if self.contains(a.lerp(b, (w[0] + w[1]) / 2.0)) {
                continue;
            }
            match out.last_mut() {
                Some(last) if (last.1 - w[0]).abs() < 1e-9 => last.1 = w[1],
                _ => out.push((w[0], w[1])),
            }
        }
        out
    }
}

/// The lettering with the knockout applied: a copy whose letters leave
/// out the cut area. `None` if it has no knockout with an area.
pub fn knocked_out(l: &Lettering) -> Option<Lettering> {
    let cut = Cut::new(l.knockout.as_ref()?.regions())?;
    let letters = l
        .letters
        .iter()
        .filter_map(|letter| {
            let parts: Vec<ObjectKind> = letter.parts.iter().flat_map(|p| cut_part(p, &cut)).collect();
            (!parts.is_empty()).then(|| Letter {
                parts,
                ..letter.clone()
            })
        })
        .collect();
    let mut out = l.clone();
    out.knockout = None;
    out.letters = letters;
    Some(out)
}

/// The parts of one letter part outside the cut.
fn cut_part(part: &ObjectKind, cut: &Cut) -> Vec<ObjectKind> {
    if !part.control_bbox().is_some_and(|b| b.intersects(&cut.all)) {
        return vec![part.clone()];
    }
    match part {
        ObjectKind::Fill { outer, holes, params } => {
            let holes: Vec<Polyline> = holes.iter().flat_map(|h| h.flatten(FLATTEN_TOL_MM)).collect();
            let area = Shape::new(&outer.flatten(FLATTEN_TOL_MM), &holes).regions;
            let rest: Vec<Region> = emb_geom::region::difference(&area, &cut.regions)
                .into_iter()
                .filter(|r| r.area() >= MIN_AREA_MM2)
                .collect();
            if rest.is_empty() {
                return Vec::new();
            }
            let outers: Vec<Path> = rest.iter().map(|r| Path::polygon(&r.outer)).collect();
            vec![ObjectKind::Fill {
                outer: Path::joined(&outers),
                holes: rest
                    .iter()
                    .flat_map(|r| r.holes.iter().map(|h| Path::polygon(h)))
                    .collect(),
                params: params.clone(),
            }]
        }
        ObjectKind::Satin { geometry, params } => cut_satin(geometry, cut)
            .into_iter()
            .map(|geometry| ObjectKind::Satin {
                geometry,
                params: params.clone(),
            })
            .collect(),
        ObjectKind::Running { path, params } => {
            let lines = cut_lines(path, cut);
            if lines.is_empty() {
                return Vec::new();
            }
            vec![ObjectKind::Running {
                path: Path::joined(&lines),
                params: params.clone(),
            }]
        }
        other => vec![other.clone()],
    }
}

/// The kept part of a rung (its two ends) and its place on the rung (from
/// and to, as fractions).
type Kept = ((Point, Point), (f64, f64));

/// The parts of the column `g` outside the cut, as columns.
fn cut_satin(g: &SatinGeometry, cut: &Cut) -> Vec<SatinGeometry> {
    let Some(col) = Column::new(g) else {
        return vec![g.clone()];
    };
    let pairs = col.pairs(RUNG_STEP_MM);
    // Each rung: its kept part and the fractions of the rung.
    let mut full = true;
    let kept: Vec<Option<Kept>> = pairs
        .iter()
        .map(|&(a, b)| {
            let parts = cut.outside(a, b);
            if parts.len() != 1 || parts[0] != (0.0, 1.0) {
                full = false;
            }
            let (u0, u1) = parts.into_iter().max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))?;
            ((u1 - u0) * a.distance(b) >= MIN_WIDTH_MM).then(|| ((a.lerp(b, u0), a.lerp(b, u1)), (u0, u1)))
        })
        .collect();
    if full {
        return vec![g.clone()];
    }
    // Runs of rungs whose parts touch the part before.
    let mut runs: Vec<(usize, Vec<(Point, Point)>)> = Vec::new();
    let mut prev: Option<(f64, f64)> = None;
    for (i, k) in kept.iter().enumerate() {
        let Some((pair, span)) = *k else {
            prev = None;
            continue;
        };
        let touches = prev.is_some_and(|p| span.0 <= p.1 && span.1 >= p.0);
        match runs.last_mut() {
            Some((_, run)) if touches => run.push(pair),
            _ => runs.push((i, vec![pair])),
        }
        prev = Some(span);
    }
    // A ring column: the last run goes on into the first one.
    let ring = g.rail_a.is_closed() && g.rail_b.is_closed();
    let last = pairs.len() - 1;
    if ring
        && runs.len() >= 2
        && runs[0].0 == 0
        && runs.last().is_some_and(|(start, run)| start + run.len() - 1 == last)
        && let (Some((_, s0)), Some((_, s1))) = (kept[0], kept[last])
        && s0.0 <= s1.1
        && s0.1 >= s1.0
    {
        let (_, first) = runs.remove(0);
        if let Some((_, run)) = runs.last_mut() {
            run.extend(first.into_iter().skip(1));
        }
    }
    runs.into_iter()
        .filter(|(_, run)| run.len() >= 2)
        .map(|(_, run)| {
            let rail = |pts: Vec<Point>| pts[1..].iter().fold(Path::new().move_to(pts[0]), |p, q| p.line_to(*q));
            let rungs = run[1..run.len() - 1].iter().copied().step_by(RUNG_EVERY).collect();
            SatinGeometry {
                rail_a: rail(run.iter().map(|p| p.0).collect()),
                rail_b: rail(run.iter().map(|p| p.1).collect()),
                rungs,
            }
        })
        .collect()
}

/// The parts of the lines of `path` outside the cut, each as a subpath.
fn cut_lines(path: &Path, cut: &Cut) -> Vec<Path> {
    let mut out = Vec::new();
    for pl in path.flatten(FLATTEN_TOL_MM) {
        let mut pts = pl.points.clone();
        if pl.closed
            && let Some(&f) = pts.first()
        {
            pts.push(f);
        }
        let mut current: Vec<Point> = Vec::new();
        let mut flush = |current: &mut Vec<Point>| {
            let len: f64 = current.windows(2).map(|w| w[0].distance(w[1])).sum();
            if current.len() >= 2 && len >= MIN_LINE_MM {
                out.push(
                    current[1..]
                        .iter()
                        .fold(Path::new().move_to(current[0]), |p, q| p.line_to(*q)),
                );
            }
            current.clear();
        };
        for w in pts.windows(2) {
            for (u0, u1) in cut.outside(w[0], w[1]) {
                let (p0, p1) = (w[0].lerp(w[1], u0), w[0].lerp(w[1], u1));
                if current.last().is_none_or(|l| l.distance(p0) > 1e-9) {
                    flush(&mut current);
                    current.push(p0);
                }
                current.push(p1);
            }
        }
        flush(&mut current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_model::SatinParams;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square(x: f64, y: f64, s: f64) -> Region {
        Region::new(vec![p(x, y), p(x + s, y), p(x + s, y + s), p(x, y + s)], Vec::new())
    }

    fn bounds(g: &SatinGeometry) -> BBox {
        let pts = [&g.rail_a, &g.rail_b]
            .into_iter()
            .flat_map(|r| r.flatten(0.05))
            .flat_map(|pl| pl.points);
        BBox::from_points(pts).unwrap()
    }

    fn column() -> SatinGeometry {
        SatinGeometry::from_centerline(&Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)), 4.0)
    }

    #[test]
    fn a_cut_across_a_column_makes_two_columns() {
        let cut = Cut::new(vec![Region::new(
            vec![p(8.0, -5.0), p(12.0, -5.0), p(12.0, 5.0), p(8.0, 5.0)],
            Vec::new(),
        )])
        .unwrap();
        let parts = cut_satin(&column(), &cut);
        assert_eq!(parts.len(), 2);
        for g in &parts {
            let b = bounds(g);
            assert!(b.max.x <= 8.0 + 1e-6 || b.min.x >= 12.0 - 1e-6, "{b:?}");
            assert!((b.height() - 4.0).abs() < 0.01);
        }
    }

    #[test]
    fn a_cut_along_a_side_makes_the_column_narrower() {
        // The cut covers y from 1 to 5: the column keeps y from -2 to 1.
        let cut = Cut::new(vec![square(-5.0, 1.0, 30.0)]).unwrap();
        let parts = cut_satin(&column(), &cut);
        assert_eq!(parts.len(), 1);
        let b = bounds(&parts[0]);
        assert!((b.height() - 3.0).abs() < 0.01, "{b:?}");
        assert!((b.width() - 20.0).abs() < 0.2);
    }

    #[test]
    fn a_far_cut_keeps_the_column_and_a_full_cut_removes_it() {
        let far = Cut::new(vec![square(50.0, 50.0, 4.0)]).unwrap();
        assert_eq!(cut_satin(&column(), &far), vec![column()]);
        let all = Cut::new(vec![square(-5.0, -5.0, 30.0)]).unwrap();
        assert!(cut_satin(&column(), &all).is_empty());
    }

    #[test]
    fn a_ring_column_cut_once_stays_one_column() {
        let g = SatinGeometry {
            rail_a: Path::rect(BBox::new(p(-1.5, -1.5), p(21.5, 21.5))),
            rail_b: Path::rect(BBox::new(p(1.5, 1.5), p(18.5, 18.5))),
            rungs: Vec::new(),
        };
        let cut = Cut::new(vec![square(8.0, 15.0, 4.0)]).unwrap();
        assert_eq!(cut_satin(&g, &cut).len(), 1);
    }

    #[test]
    fn fills_and_lines_keep_the_parts_outside() {
        let cut = Cut::new(vec![square(5.0, -5.0, 10.0)]).unwrap();
        let fill = ObjectKind::Fill {
            outer: Path::polygon(&[p(0.0, 0.0), p(20.0, 0.0), p(20.0, 4.0), p(0.0, 4.0)]),
            holes: Vec::new(),
            params: Default::default(),
        };
        let [ObjectKind::Fill { outer, .. }] = &cut_part(&fill, &cut)[..] else {
            panic!("one fill");
        };
        assert_eq!(outer.subpaths().len(), 2);
        let line = ObjectKind::Running {
            path: Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)),
            params: Default::default(),
        };
        let [ObjectKind::Running { path, .. }] = &cut_part(&line, &cut)[..] else {
            panic!("one line");
        };
        assert_eq!(path.subpaths().len(), 2);
        let satin = ObjectKind::Satin {
            geometry: column(),
            params: SatinParams::default(),
        };
        assert_eq!(cut_part(&satin, &cut).len(), 2);
    }

    #[test]
    fn a_lettering_sews_round_its_knockout() {
        let mut l = Lettering::new("I", Default::default(), 10.0, Point::ZERO);
        l.letters = vec![Letter {
            text: "I".into(),
            line: 0,
            index: 0,
            frame: emb_geom::Transform::IDENTITY,
            center: Point::ZERO,
            parts: vec![ObjectKind::Satin {
                geometry: column(),
                params: SatinParams::default(),
            }],
        }];
        let mut d = emb_model::Design::new("k");
        let t = d.add_thread("a", emb_model::Rgb(0, 0, 0));
        let plain = d.add_object(t, ObjectKind::Lettering(Box::new(l.clone())));
        let hole = Region::new(vec![p(8.0, -5.0), p(12.0, -5.0), p(12.0, 5.0), p(8.0, 5.0)], Vec::new());
        l.knockout = Some(emb_model::Knockout {
            overlap_mm: 0.0,
            area: vec![emb_model::CutArea::from_region(&hole)],
        });
        let cut = d.add_object(t, ObjectKind::Lettering(Box::new(l)));
        let points = |id| -> Vec<Point> {
            crate::object::stitch_object(d.object(id).unwrap(), Point::ZERO)
                .unwrap()
                .runs
                .into_iter()
                .flat_map(|r| r.points)
                .collect()
        };
        let inside = |q: &Point| q.x > 8.2 && q.x < 11.8;
        assert!(points(plain).iter().any(inside));
        let sewn = points(cut);
        assert!(!sewn.is_empty());
        assert!(!sewn.iter().any(inside), "a needle point in the cut");
    }
}
