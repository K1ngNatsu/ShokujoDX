//! The check of free-standing lace (P11 Milestone 5, ADR 0048): each part
//! of the design must hold to the others, because the stabilizer washes
//! away.
//!
//! Not a port; common sense of lace digitizing:
//!
//! 1. The thread of each object is its stitches in the plan, from its first
//!    needle point (the travel from the object before is not part of it):
//!    lines that a jump, a trim or a color change cuts.
//! 2. Two lines hold together where their stitches cross or come nearer
//!    than [`TOUCH_MM`]. A line holds to itself.
//! 3. The parts are the groups of lines that hold together. The part with
//!    the most thread is the lace; each other part is loose: it falls off
//!    when the stabilizer washes away.

use std::collections::HashMap;

use emb_geom::{BBox, Point, Transform};
use emb_model::{Design, ObjectId, StitchKind};

use crate::plan::StitchPlan;

/// Stitches nearer than this hold together, in mm.
pub const TOUCH_MM: f64 = 0.3;
/// Parts with less thread than this are not reported (a lock stitch alone), in mm.
const MIN_PART_MM: f64 = 1.0;
/// The size of a cell of the grid that finds stitches near each other, in mm.
const CELL_MM: f64 = 2.0;

/// A loose part of a lace design.
#[derive(Debug, Clone, PartialEq)]
pub struct LoosePart {
    /// The object with the most thread in the part.
    pub object: ObjectId,
    /// The middle of the part, in design coordinates.
    pub center: Point,
    /// The thread of the part on the fabric, in mm.
    pub length_mm: f64,
}

/// A stitch of line `.0` from `.1` to `.2`.
type Stitch = (usize, Point, Point);

/// Union-find with path halving.
fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The distance between segments `a`–`b` and `c`–`d`.
fn seg_distance(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let r = b - a;
    let s = d - c;
    let den = r.cross(s);
    if den.abs() > 1e-12 {
        let t = (c - a).cross(s) / den;
        let u = (c - a).cross(r) / den;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            return 0.0;
        }
    }
    let to_seg = |p: Point, q0: Point, q1: Point| {
        let v = q1 - q0;
        let l = v.dot(v);
        let t = if l > 0.0 {
            ((p - q0).dot(v) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        p.distance(q0 + v * t)
    };
    to_seg(a, c, d)
        .min(to_seg(b, c, d))
        .min(to_seg(c, a, b))
        .min(to_seg(d, a, b))
}

/// The design is free-standing lace: its fabric is a lace fabric (water-
/// soluble stabilizer), or it has a lace mesh.
pub fn is_lace(design: &Design, fabrics: &emb_model::FabricCatalog) -> bool {
    fabrics.of(design).is_lace()
        || design.objects().iter().any(|o| {
            matches!(&o.kind, emb_model::ObjectKind::Fill { params, .. }
                if matches!(params.pattern, emb_model::FillPattern::Mesh { .. }))
        })
}

/// The loose parts of the plan of `design`, largest first. Empty when the
/// whole design holds together.
pub fn loose_parts(design: &Design, plan: &StitchPlan) -> Vec<LoosePart> {
    let recs = &plan.stitches.records;
    // Lines: (object, points), in output coordinates.
    let mut lines: Vec<(ObjectId, Vec<Point>)> = Vec::new();
    for span in &plan.spans {
        let range = span.records.start.min(recs.len())..span.records.end.min(recs.len());
        let first = range
            .clone()
            .find(|&i| recs[i].pos.distance(span.start) < 1e-6)
            .unwrap_or(range.start);
        let mut cur: Vec<Point> = Vec::new();
        for r in &recs[first..range.end] {
            if r.kind == StitchKind::Stitch {
                cur.push(r.pos);
            } else if !cur.is_empty() {
                lines.push((span.object, std::mem::take(&mut cur)));
            }
        }
        if !cur.is_empty() {
            lines.push((span.object, cur));
        }
    }
    if lines.len() < 2 {
        return Vec::new();
    }
    // The stitches in a grid of cells.
    let mut cells: HashMap<(i64, i64), Vec<Stitch>> = HashMap::new();
    let cell = |p: f64| (p / CELL_MM).floor() as i64;
    for (li, (_, pts)) in lines.iter().enumerate() {
        let segs: Vec<(Point, Point)> = if pts.len() == 1 {
            vec![(pts[0], pts[0])]
        } else {
            pts.windows(2).map(|w| (w[0], w[1])).collect()
        };
        for (a, b) in segs {
            let (x0, x1) = (cell(a.x.min(b.x) - TOUCH_MM), cell(a.x.max(b.x) + TOUCH_MM));
            let (y0, y1) = (cell(a.y.min(b.y) - TOUCH_MM), cell(a.y.max(b.y) + TOUCH_MM));
            // A wrong (very long) stitch is in at most a few cells across.
            if (x1 - x0) * (y1 - y0) > 10_000 {
                continue;
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    cells.entry((x, y)).or_default().push((li, a, b));
                }
            }
        }
    }
    let mut parent: Vec<usize> = (0..lines.len()).collect();
    let mut keys: Vec<&(i64, i64)> = cells.keys().collect();
    keys.sort_unstable();
    for k in keys {
        let segs = &cells[k];
        for i in 0..segs.len() {
            for j in i + 1..segs.len() {
                let (li, a, b) = segs[i];
                let (lj, c, d) = segs[j];
                if li == lj {
                    continue;
                }
                let (ri, rj) = (find(&mut parent, li), find(&mut parent, lj));
                if ri != rj && seg_distance(a, b, c, d) <= TOUCH_MM {
                    parent[ri] = rj;
                }
            }
        }
    }
    // The parts: thread length, the thread of each object, the box.
    struct Part {
        length: f64,
        by_object: HashMap<ObjectId, f64>,
        bbox: Option<BBox>,
    }
    let mut parts: HashMap<usize, Part> = HashMap::new();
    for (li, (obj, pts)) in lines.iter().enumerate() {
        let root = find(&mut parent, li);
        let len: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        let part = parts.entry(root).or_insert(Part {
            length: 0.0,
            by_object: HashMap::new(),
            bbox: None,
        });
        part.length += len;
        *part.by_object.entry(*obj).or_default() += len;
        let b = BBox::from_points(pts.iter().copied());
        part.bbox = match (part.bbox, b) {
            (Some(x), Some(y)) => Some(x.union(y)),
            (x, y) => x.or(y),
        };
    }
    let mut all: Vec<Part> = parts.into_values().collect();
    all.sort_by(|a, b| b.length.total_cmp(&a.length));
    let back = design.output_transform().inverse().unwrap_or(Transform::IDENTITY);
    all.into_iter()
        .skip(1)
        .filter(|p| p.length >= MIN_PART_MM)
        .filter_map(|p| {
            let object = p
                .by_object
                .iter()
                .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(o, _)| *o)?;
            Some(LoosePart {
                object,
                center: back.apply(p.bbox?.center()),
                length_mm: p.length,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{FillParams, FillPattern, ObjectKind, Rgb, RunningParams};

    use super::*;
    use crate::object::StitchCache;
    use crate::plan::{PlanOptions, build_plan};

    fn check(d: &Design) -> Vec<LoosePart> {
        let plan = build_plan(d, &PlanOptions::default(), &mut StitchCache::default());
        loose_parts(d, &plan)
    }

    fn mesh(d: &mut Design, t: emb_model::ThreadId) -> ObjectId {
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::ellipse(Point::ZERO, 20.0, 20.0),
                holes: Vec::new(),
                params: FillParams::default().with_pattern(FillPattern::Mesh { angle_deg: 90.0 }),
            },
        )
    }

    fn line(d: &mut Design, t: emb_model::ThreadId, a: Point, b: Point) -> ObjectId {
        d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(a).line_to(b),
                params: RunningParams::default(),
            },
        )
    }

    #[test]
    fn parts_that_touch_the_mesh_hold() {
        let mut d = Design::new("lace");
        let t = d.add_thread("White", Rgb(250, 250, 250));
        mesh(&mut d, t);
        // A line across the mesh, and a line from the mesh out past its edge.
        line(&mut d, t, Point::new(-15.0, 0.0), Point::new(15.0, 0.0));
        line(&mut d, t, Point::new(10.0, 10.0), Point::new(30.0, 30.0));
        assert!(check(&d).is_empty(), "{:?}", check(&d));
    }

    #[test]
    fn a_part_away_from_the_mesh_is_loose() {
        let mut d = Design::new("lace");
        let t = d.add_thread("White", Rgb(250, 250, 250));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        mesh(&mut d, t);
        let away = line(&mut d, blue, Point::new(40.0, -5.0), Point::new(40.0, 5.0));
        let loose = check(&d);
        assert_eq!(loose.len(), 1, "{loose:?}");
        assert_eq!(loose[0].object, away);
        assert!(
            loose[0].center.distance(Point::new(40.0, 0.0)) < 0.6,
            "{:?}",
            loose[0].center
        );
        // 10 mm, and the lock stitches at its ends.
        assert!((10.0..14.0).contains(&loose[0].length_mm), "{}", loose[0].length_mm);
        // A line that joins it to the mesh: it holds.
        line(&mut d, blue, Point::new(15.0, 0.0), Point::new(41.0, 0.0));
        assert!(check(&d).is_empty());
    }

    #[test]
    fn lace_designs_by_fabric_or_mesh() {
        let fabrics = emb_model::FabricCatalog::builtin();
        let mut d = Design::new("lace");
        let t = d.add_thread("White", Rgb(250, 250, 250));
        line(&mut d, t, Point::ZERO, Point::new(10.0, 0.0));
        assert!(!is_lace(&d, &fabrics));
        d.fabric = Some("water_soluble".into());
        assert!(is_lace(&d, &fabrics));
        d.fabric = None;
        mesh(&mut d, t);
        assert!(is_lace(&d, &fabrics));
    }

    #[test]
    fn segment_distances() {
        let p = Point::new;
        assert_eq!(seg_distance(p(0.0, 0.0), p(2.0, 2.0), p(0.0, 2.0), p(2.0, 0.0)), 0.0);
        assert!((seg_distance(p(0.0, 0.0), p(2.0, 0.0), p(0.0, 1.0), p(2.0, 1.0)) - 1.0).abs() < 1e-12);
        assert!((seg_distance(p(0.0, 0.0), p(1.0, 0.0), p(3.0, 0.0), p(4.0, 0.0)) - 2.0).abs() < 1e-12);
    }
}
