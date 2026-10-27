//! Editing helpers (P10 Milestone 1).
//!
//! - Reverse direction: lines, satin columns, vectors and stone runs go
//!   the other way. The start and end points change places. The direction
//!   sets where a motif, a border or a stone run starts and which way it
//!   points; a plain line still sews from the end that is nearer.
//! - Swap sides: the two rails of a satin column change places, so the
//!   settings of each side (pull compensation, where the spacing is
//!   measured) go to the other side.
//! - Close shape: open lines, vectors and stone runs get a segment from
//!   their last point to their first point, a line or a curve.
//! - Paste after: copies go just after an object in the sew order.
//! - To single stones: a stone run or fill becomes single stones at the
//!   places where the design shows them, so each stone can move alone.

use emb_geom::{Path, Point};

use crate::bling::BlingLayout;
use crate::{Design, DesignObject, ObjectId, ObjectKind, SatinGeometry};

/// The column the other way along: both rails reversed. The rungs stay.
fn reversed_column(g: &SatinGeometry) -> SatinGeometry {
    SatinGeometry {
        rail_a: g.rail_a.reversed(),
        rail_b: g.rail_b.reversed(),
        rungs: g.rungs.clone(),
    }
}

/// Reverses the paths of `kind` that have a direction. Returns `false` for
/// kinds with no direction.
fn reverse_kind(kind: &mut ObjectKind) -> bool {
    match kind {
        ObjectKind::Running { path, .. } | ObjectKind::Vector { path, .. } => {
            *path = path.reversed();
            true
        }
        ObjectKind::Satin { geometry, params } => {
            *geometry = reversed_column(geometry);
            // The accordion profile goes from the start of the column.
            if let Some(a) = &mut params.accordion {
                a.points = a.points.iter().rev().map(|&(t, f)| (1.0 - t, f)).collect();
            }
            true
        }
        ObjectKind::Bling(b) => match &mut b.layout {
            BlingLayout::Run { path } => {
                *path = path.reversed();
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// The paths of `kind` that close shape can close.
fn open_paths(kind: &mut ObjectKind) -> Vec<&mut Path> {
    match kind {
        ObjectKind::Running { path, .. } | ObjectKind::Vector { path, .. } => vec![path],
        ObjectKind::Bling(b) => match &mut b.layout {
            BlingLayout::Run { path } => vec![path],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

impl Design {
    /// Reverses the direction of the objects (see the module notes).
    /// Returns the number of objects that changed.
    pub fn reverse_objects(&mut self, ids: &[ObjectId]) -> usize {
        let mut n = 0;
        for &id in ids {
            let Some(o) = self.object(id) else { continue };
            let mut kind = o.kind.clone();
            if reverse_kind(&mut kind)
                && let Some(o) = self.object_mut(id)
            {
                o.kind = kind;
                std::mem::swap(&mut o.start, &mut o.end);
                n += 1;
            }
        }
        n
    }

    /// Swaps the two sides of the satin columns. Returns the number of
    /// columns that changed.
    pub fn swap_column_sides(&mut self, ids: &[ObjectId]) -> usize {
        let mut n = 0;
        for &id in ids {
            if !matches!(self.object(id).map(|o| &o.kind), Some(ObjectKind::Satin { .. })) {
                continue;
            }
            let Some(DesignObject {
                kind: ObjectKind::Satin { geometry, params },
                ..
            }) = self.object_mut(id)
            else {
                continue;
            };
            std::mem::swap(&mut geometry.rail_a, &mut geometry.rail_b);
            for r in &mut geometry.rungs {
                *r = (r.1, r.0);
            }
            params.spacing_at = 1.0 - params.spacing_at;
            if let Some(b) = params.pull_b_mm {
                params.pull_b_mm = Some(params.pull_compensation_mm);
                params.pull_compensation_mm = b;
            }
            n += 1;
        }
        n
    }

    /// Closes the open parts of lines, vectors and stone runs, with a line
    /// or (`curve`) a curve. Returns the number of objects that changed.
    pub fn close_shapes(&mut self, ids: &[ObjectId], curve: bool) -> usize {
        let mut n = 0;
        for &id in ids {
            let Some(o) = self.object(id) else { continue };
            let mut kind = o.kind.clone();
            let closed: usize = open_paths(&mut kind)
                .into_iter()
                .map(|p| {
                    let (new, k) = p.close_open(curve);
                    *p = new;
                    k
                })
                .sum();
            if closed > 0
                && let Some(o) = self.object_mut(id)
            {
                o.kind = kind;
                // The end point of a closed line is its start.
                o.end = None;
                n += 1;
            }
        }
        n
    }

    /// Like [`Design::paste_objects`], but the copies go just after `after`
    /// in the sew order (at the end if `after` is not in the design).
    pub fn paste_objects_after(
        &mut self,
        objects: &[DesignObject],
        threads: &[crate::Thread],
        after: ObjectId,
    ) -> Vec<ObjectId> {
        let ids = self.paste_objects(objects, threads);
        let next = self
            .index_of(after)
            .and_then(|i| self.objects().get(i + 1))
            .map(|o| o.id)
            .filter(|id| !ids.contains(id));
        self.move_objects_before(&ids, next);
        ids
    }

    /// Changes stone runs and fills to single stones at the places of
    /// [`Design::stones`] (stones that the overlap removal leaves out do not
    /// come back). Returns the number of objects that changed.
    pub fn to_single_stones(&mut self, ids: &[ObjectId]) -> usize {
        let stones = self.stones();
        let mut n = 0;
        for &id in ids {
            let is_pattern = matches!(
                self.object(id).map(|o| &o.kind),
                Some(ObjectKind::Bling(b)) if !matches!(b.layout, BlingLayout::Stones { .. })
            );
            if !is_pattern {
                continue;
            }
            let points: Vec<Point> = stones.iter().filter(|s| s.object == id).map(|s| s.at).collect();
            if let Some(DesignObject {
                kind: ObjectKind::Bling(b),
                ..
            }) = self.object_mut(id)
            {
                b.layout = BlingLayout::Stones { points };
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;

    use super::*;
    use crate::bling::{Bling, Stone};
    use crate::{Rgb, RunningParams, SatinParams};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn design() -> (Design, crate::ThreadId) {
        let mut d = Design::new("t");
        let t = d.add_thread("T", Rgb(0, 0, 0));
        (d, t)
    }

    fn line() -> Path {
        Path::new()
            .move_to(p(0.0, 0.0))
            .line_to(p(10.0, 0.0))
            .line_to(p(10.0, 10.0))
    }

    #[test]
    fn reverse_lines_columns_and_runs() {
        let (mut d, t) = design();
        let run = d.add_object(
            t,
            ObjectKind::Running {
                path: line(),
                params: RunningParams::default(),
            },
        );
        d.object_mut(run).unwrap().start = Some(p(0.0, 0.0));
        let col = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)),
                    rail_b: Path::new().move_to(p(0.0, 3.0)).line_to(p(20.0, 3.0)),
                    rungs: Vec::new(),
                },
                params: SatinParams {
                    accordion: Some(crate::Accordion {
                        points: vec![(0.0, 1.0), (0.25, 2.0), (1.0, 1.0)],
                    }),
                    ..SatinParams::default()
                },
            },
        );
        let fill = d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(emb_geom::BBox::new(p(0.0, 0.0), p(5.0, 5.0))),
                holes: Vec::new(),
                params: Default::default(),
            },
        );
        assert_eq!(d.reverse_objects(&[run, col, fill]), 2);
        let o = d.object(run).unwrap();
        assert_eq!(o.start, None);
        assert_eq!(o.end, Some(p(0.0, 0.0)));
        let ObjectKind::Running { path, .. } = &o.kind else {
            panic!()
        };
        assert_eq!(path.flatten(0.01)[0].points[0], p(10.0, 10.0));
        let ObjectKind::Satin { geometry, params } = &d.object(col).unwrap().kind else {
            panic!()
        };
        assert_eq!(geometry.rail_a.flatten(0.01)[0].points[0], p(20.0, 0.0));
        assert_eq!(
            params.accordion.as_ref().unwrap().points,
            vec![(0.0, 1.0), (0.75, 2.0), (1.0, 1.0)]
        );
    }

    #[test]
    fn swap_sides_moves_the_side_settings() {
        let (mut d, t) = design();
        let col = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::new().move_to(p(0.0, 0.0)).line_to(p(20.0, 0.0)),
                    rail_b: Path::new().move_to(p(0.0, 3.0)).line_to(p(20.0, 3.0)),
                    rungs: vec![(p(5.0, 0.0), p(6.0, 3.0))],
                },
                params: SatinParams {
                    spacing_at: 0.2,
                    pull_compensation_mm: 0.1,
                    pull_b_mm: Some(0.3),
                    ..SatinParams::default()
                },
            },
        );
        assert_eq!(d.swap_column_sides(&[col]), 1);
        let ObjectKind::Satin { geometry, params } = &d.object(col).unwrap().kind else {
            panic!()
        };
        assert_eq!(geometry.rail_a.flatten(0.01)[0].points[0], p(0.0, 3.0));
        assert_eq!(geometry.rungs, vec![(p(6.0, 3.0), p(5.0, 0.0))]);
        assert!((params.spacing_at - 0.8).abs() < 1e-12);
        assert_eq!((params.pull_compensation_mm, params.pull_b_mm), (0.3, Some(0.1)));
    }

    #[test]
    fn close_shapes_closes_open_lines() {
        let (mut d, t) = design();
        let run = d.add_object(
            t,
            ObjectKind::Running {
                path: line(),
                params: RunningParams::default(),
            },
        );
        d.object_mut(run).unwrap().end = Some(p(10.0, 10.0));
        assert_eq!(d.close_shapes(&[run], false), 1);
        let o = d.object(run).unwrap();
        let ObjectKind::Running { path, .. } = &o.kind else {
            panic!()
        };
        assert!(path.is_closed());
        assert_eq!(o.end, None);
        assert_eq!(d.close_shapes(&[run], true), 0, "closed already");
    }

    #[test]
    fn paste_after_goes_after_the_object() {
        let (mut d, t) = design();
        let mk = |d: &mut Design| {
            d.add_object(
                t,
                ObjectKind::Running {
                    path: line(),
                    params: RunningParams::default(),
                },
            )
        };
        let a = mk(&mut d);
        let b = mk(&mut d);
        let c = mk(&mut d);
        let copy = vec![d.object(c).unwrap().clone()];
        let threads = d.threads.clone();
        let new = d.paste_objects_after(&copy, &threads, a);
        let order: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        assert_eq!(order, vec![a, new[0], b, c]);
        // After the last object: at the end.
        let new2 = d.paste_objects_after(&copy, &threads, c);
        assert_eq!(d.objects().last().unwrap().id, new2[0]);
    }

    #[test]
    fn to_single_stones_keeps_the_places() {
        let (mut d, t) = design();
        let run = d.add_object(
            t,
            ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Run {
                    path: Path::new().move_to(p(0.0, 0.0)).line_to(p(30.0, 0.0)),
                },
            ))),
        );
        let before: Vec<Point> = d.stones().iter().map(|s| s.at).collect();
        assert!(before.len() > 3);
        assert_eq!(d.to_single_stones(&[run]), 1);
        let after: Vec<Point> = d.stones().iter().map(|s| s.at).collect();
        assert_eq!(before, after);
        assert_eq!(d.to_single_stones(&[run]), 0, "single stones already");
    }
}
