//! A sew order with less travel (owner report, 2026-10-24: too many jumps,
//! in an order that goes to and fro over the design).
//!
//! Not a port. A nearest-neighbor order, which is common practice:
//!
//! - Only the objects of one color block change places, so the color
//!   changes stay the same. Objects that sew nothing (vectors, rhinestones)
//!   stay where they are and end a block.
//! - Sew order is stacking order: an object never moves before an object
//!   that it overlaps (by boxes, as `Design::optimize_color_changes`), so
//!   what is on top stays on top.
//! - The needle starts at the start point of the design, or else at the
//!   hoop center. The next object is the one, of those that can go next,
//!   with a point (its start point if it has one, else a point of its
//!   outline) nearest to the needle. The needle then goes to where that
//!   object ends: the stitch engine sews it from there to find out.
//!
//! The stitch plan then sews short gaps with a travel stitch and trims at
//! long ones, so a shorter way between the objects also means fewer trims.

use emb_geom::{BBox, Point, Transform};
use emb_model::{Design, DesignObject, ObjectId, RunPass};

use crate::object::{Run, StitchCache};

/// An object is tested at this many points of its outline, most.
const MAX_POINTS: usize = 200;
/// Outlines are flattened to this tolerance for the test points, in mm.
const FLAT_MM: f64 = 0.2;

/// All objects of the design in a sew order with less travel (see the
/// module comment). The result has each object one time.
pub fn optimized_order(design: &Design, cache: &mut StitchCache) -> Vec<ObjectId> {
    let to_design = design.output_transform().inverse().unwrap_or(Transform::IDENTITY);
    let mut needle = design
        .start_end
        .start
        .and_then(|a| design.anchor_point(a))
        .unwrap_or_else(|| to_design.apply(Point::ZERO));
    let objects = design.objects();
    let mut out = Vec::with_capacity(objects.len());
    let mut i = 0;
    while i < objects.len() {
        if !objects[i].kind.sews() {
            out.push(objects[i].id);
            i += 1;
            continue;
        }
        let thread = objects[i].thread;
        let end = objects[i..]
            .iter()
            .position(|o| !o.kind.sews() || o.thread != thread)
            .map_or(objects.len(), |n| i + n);
        let block: Vec<&DesignObject> = objects[i..end].iter().map(|o| o.as_ref()).collect();
        for id in block_order(&block, &mut needle, cache) {
            out.push(id);
        }
        i = end;
    }
    out
}

/// The width of the bands of a cap order, in mm.
pub const CAP_BAND_MM: f64 = 10.0;

/// All objects of the design in a cap order (P11 Milestone 7, ADR 0050):
/// in each color block, the objects from the center of the hoop out (in
/// bands of [`CAP_BAND_MM`] to the left and right) and, in a band, from the
/// bottom up, so the cap fabric does not move. Colors stay, and an object
/// never moves before an object that it overlaps (as in
/// [`optimized_order`]). The result has each object one time.
pub fn cap_order(design: &Design) -> Vec<ObjectId> {
    let to_out = design.output_transform();
    let objects = design.objects();
    let mut out = Vec::with_capacity(objects.len());
    let mut i = 0;
    while i < objects.len() {
        if !objects[i].kind.sews() {
            out.push(objects[i].id);
            i += 1;
            continue;
        }
        let thread = objects[i].thread;
        let end = objects[i..]
            .iter()
            .position(|o| !o.kind.sews() || o.thread != thread)
            .map_or(objects.len(), |n| i + n);
        let block: Vec<&DesignObject> = objects[i..end].iter().map(|o| o.as_ref()).collect();
        let boxes: Vec<Option<BBox>> = block.iter().map(|o| o.control_bbox()).collect();
        let keys: Vec<(i64, f64)> = boxes
            .iter()
            .map(|b| {
                let c = b.map_or(Point::ZERO, |b| to_out.apply(b.center()));
                ((c.x.abs() / CAP_BAND_MM).floor() as i64, -c.y)
            })
            .collect();
        let mut done = vec![false; block.len()];
        for _ in 0..block.len() {
            let free = (0..block.len()).filter(|&j| {
                !done[j]
                    && (0..j)
                        .all(|k| done[k] || !matches!((&boxes[k], &boxes[j]), (Some(a), Some(b)) if a.intersects(b)))
            });
            let Some(j) = free.min_by(|&a, &b| {
                keys[a]
                    .0
                    .cmp(&keys[b].0)
                    .then(keys[a].1.total_cmp(&keys[b].1))
                    .then(a.cmp(&b))
            }) else {
                break;
            };
            done[j] = true;
            out.push(block[j].id);
        }
        i = end;
    }
    out
}

/// The order of one color block, from `needle`; `needle` becomes the end
/// of the last object.
fn block_order(block: &[&DesignObject], needle: &mut Point, cache: &mut StitchCache) -> Vec<ObjectId> {
    let boxes: Vec<Option<BBox>> = block.iter().map(|o| o.control_bbox()).collect();
    // The objects before `j` in the block that `j` overlaps: they sew first.
    let before: Vec<Vec<usize>> = (0..block.len())
        .map(|j| {
            (0..j)
                .filter(|&i| matches!((&boxes[i], &boxes[j]), (Some(a), Some(b)) if a.intersects(b)))
                .collect()
        })
        .collect();
    let points: Vec<Vec<Point>> = block.iter().map(|o| test_points(o)).collect();
    let mut done = vec![false; block.len()];
    let mut out = Vec::with_capacity(block.len());
    for _ in 0..block.len() {
        // Ties go to the earlier object, so the result is deterministic.
        let mut best: Option<(f64, usize)> = None;
        for j in 0..block.len() {
            if done[j] || before[j].iter().any(|&i| !done[i]) {
                continue;
            }
            let d = points[j]
                .iter()
                .map(|p| p.distance(*needle))
                .fold(f64::INFINITY, f64::min);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, j));
            }
        }
        let Some((_, j)) = best else { break };
        done[j] = true;
        out.push(block[j].id);
        if let Ok(s) = cache.get(block[j], *needle).as_ref() {
            for run in &s.runs {
                *needle = run_end(run, *needle);
            }
        }
    }
    out
}

/// The points where an object can start: its start point, or points of its
/// outline (at most [`MAX_POINTS`]).
fn test_points(o: &DesignObject) -> Vec<Point> {
    if let Some(s) = o.start {
        return vec![s];
    }
    let pts: Vec<Point> = o
        .kind
        .paths()
        .flat_map(|p| p.flatten(FLAT_MM))
        .flat_map(|l| l.points)
        .collect();
    if pts.is_empty() {
        return o.control_bbox().map(|b| vec![b.center()]).unwrap_or_default();
    }
    let step = pts.len().div_ceil(MAX_POINTS).max(1);
    pts.into_iter().step_by(step).collect()
}

/// Where the needle is after `run`, when it is at `needle` before it (as
/// the stitch plan routes it: fills and satins as they are, a loop back to
/// its start, a line from its nearer end).
fn run_end(run: &Run, needle: Point) -> Point {
    let (Some(&first), Some(&last)) = (run.points.first(), run.points.last()) else {
        return needle;
    };
    if run.fixed {
        return last;
    }
    if run.closed {
        return run
            .points
            .iter()
            .copied()
            .min_by(|a, b| a.distance(needle).total_cmp(&b.distance(needle)))
            .unwrap_or(first);
    }
    let start_at_first = needle.distance(first) <= needle.distance(last);
    let back = matches!(run.pass, RunPass::Backtrack | RunPass::Repeat);
    match (start_at_first, back) {
        (true, false) | (false, true) => last,
        (true, true) | (false, false) => first,
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{ObjectKind, Rgb, RunningParams};

    use super::*;
    use crate::{PlanOptions, build_plan};

    fn dot(d: &mut Design, t: emb_model::ThreadId, x: f64, y: f64) -> ObjectId {
        let ring: Vec<Point> = (0..24)
            .map(|i| {
                let a = std::f64::consts::TAU * f64::from(i) / 24.0;
                Point::new(x + 3.0 * a.cos(), y + 3.0 * a.sin())
            })
            .collect();
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::polygon(&ring),
                holes: Vec::new(),
                params: Default::default(),
            },
        )
    }

    fn trims(d: &Design) -> usize {
        build_plan(d, &PlanOptions::default(), &mut StitchCache::default())
            .stitches
            .trim_count()
    }

    #[test]
    fn a_cap_order_goes_from_the_center_out_and_from_the_bottom_up() {
        let mut d = Design::new("");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        // Dots in a row and one above the middle, in a mixed order.
        let left = dot(&mut d, red, -40.0, 0.0);
        let top = dot(&mut d, red, 0.0, -20.0);
        let right = dot(&mut d, red, 40.0, 0.0);
        let middle = dot(&mut d, red, 0.0, 0.0);
        let near = dot(&mut d, red, 15.0, 0.0);
        let order = cap_order(&d);
        assert_eq!(order, vec![middle, top, near, left, right]);
        // An object on top stays on top.
        let mut d2 = Design::new("");
        let t = d2.add_thread("Red", Rgb(200, 0, 0));
        let under = dot(&mut d2, t, 30.0, 0.0);
        let over = dot(&mut d2, t, 31.0, 0.0);
        let mid = dot(&mut d2, t, 0.0, 0.0);
        assert_eq!(cap_order(&d2), vec![mid, under, over]);
    }

    #[test]
    fn dots_sew_nearest_first_and_colors_stay() {
        let mut d = Design::new("");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        // A row of dots in a mixed order, then a blue line.
        let xs = [0.0, 40.0, 10.0, 30.0, 20.0];
        let ids: Vec<ObjectId> = xs.iter().map(|&x| dot(&mut d, red, x - 20.0, 0.0)).collect();
        let line = d.add_object(
            blue,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(-20.0, 20.0))
                    .line_to(Point::new(20.0, 20.0)),
                params: RunningParams::default(),
            },
        );
        let order = optimized_order(&d, &mut StitchCache::default());
        assert_eq!(order.len(), 6);
        assert_eq!(order[5], line, "the color blocks stay");
        // From the hoop center (0, 0): the middle dot, then along the row,
        // which is less travel than the order as drawn.
        let x_of = |id: ObjectId| xs[ids.iter().position(|i| *i == id).unwrap()] - 20.0;
        let travel = |row: &[ObjectId]| {
            let mut at = 0.0;
            let mut sum = 0.0;
            for id in row {
                sum += (x_of(*id) - at).abs();
                at = x_of(*id);
            }
            sum
        };
        assert_eq!(x_of(order[0]), 0.0);
        assert!(travel(&order[..5]) <= 60.0 + 1e-9, "{}", travel(&order[..5]));
        assert!(travel(&ids) > 100.0);
        let mut opt = d.clone();
        assert!(opt.sequence_in_order(&order));
        assert_eq!(trims(&opt), trims(&d), "dots apart: a trim between each");
    }

    #[test]
    fn overlapping_objects_keep_their_order() {
        let mut d = Design::new("");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        // A far dot, then a big dot over the middle, then a dot under the
        // big one: the last one must stay after the big one.
        let far = dot(&mut d, red, 60.0, 0.0);
        let big = dot(&mut d, red, 0.0, 0.0);
        let small = dot(&mut d, red, 1.0, 0.0);
        let order = optimized_order(&d, &mut StitchCache::default());
        let at = |id| order.iter().position(|x| *x == id).unwrap();
        assert!(at(big) < at(small));
        assert_eq!(at(far), 2, "the far dot goes last");
    }
}
