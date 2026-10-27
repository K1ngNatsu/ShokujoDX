//! Design repeats (P7 Milestone 6): the design shown again in columns and
//! rows, to check the join of a continuous design (a border strip that
//! goes on and on) or of an overlapping design, and Make repeats, which
//! makes the copies as objects.
//!
//! The steps are in view coordinates (after the design rotation), as the
//! canvas shows them. With auto start and end at the two ends of the design
//! (ADR 0032), "join end to start" sets the step so that the end of each
//! copy is the start of the next: the needle goes from copy to copy.

use emb_geom::{Point, Transform};
use serde::{Deserialize, Serialize};

use crate::{Design, ObjectId};

/// The most copies that a repeat shows or makes.
pub const MAX_COPIES: u32 = 100;

/// The repeats of a design.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RepeatSpec {
    pub columns: u32,
    pub rows: u32,
    /// From a copy to the copy right of it (mm, view coordinates).
    pub step_x: Point,
    /// From a copy to the copy below it.
    pub step_y: Point,
    /// Show the copies on the canvas (they do not sew).
    #[serde(default)]
    pub show: bool,
}

impl RepeatSpec {
    /// Three copies side by side, `gap` mm apart (a negative gap overlaps them).
    pub fn side_by_side(design: &Design, gap: f64) -> RepeatSpec {
        let (w, h) = design.view_extents().map_or((50.0, 50.0), |b| (b.width(), b.height()));
        RepeatSpec {
            columns: 3,
            rows: 1,
            step_x: Point::new(w + gap, 0.0),
            step_y: Point::new(0.0, h + gap),
            show: true,
        }
    }

    /// The offsets of the copies (not the design itself), view coordinates.
    pub fn offsets(&self) -> Vec<Point> {
        let (c, r) = (self.columns.max(1), self.rows.max(1));
        (0..r)
            .flat_map(|j| (0..c).map(move |i| (i, j)))
            .filter(|&(i, j)| (i, j) != (0, 0))
            .take(MAX_COPIES as usize)
            .map(|(i, j)| self.step_x * f64::from(i) + self.step_y * f64::from(j))
            .collect()
    }
}

impl Design {
    /// The step from the start point to the end point of the design (view
    /// coordinates): the step that makes the end of each copy the start of
    /// the next. `None` without both auto start and auto end.
    pub fn join_step(&self) -> Option<Point> {
        let rot = self.rotation_transform();
        let s = self.anchor_point(self.start_end.start?)?;
        let e = self.anchor_point(self.start_end.end?)?;
        Some(rot.apply(e) - rot.apply(s))
    }

    /// Makes the repeats as objects: a copy of all objects for each offset,
    /// copy by copy at the end of the sew order. The repeats stop showing
    /// (the copies are there now). Returns the new objects.
    pub fn make_repeats(&mut self) -> Vec<ObjectId> {
        let Some(spec) = self.repeat else {
            return Vec::new();
        };
        let Some(inv) = self.rotation_transform().inverse() else {
            return Vec::new();
        };
        let ids: Vec<ObjectId> = self.objects.iter().map(|o| o.id).collect();
        let mut new = Vec::new();
        for off in spec.offsets() {
            // The step in design coordinates (turn only, no move).
            let d = inv.apply(off) - inv.apply(Point::ZERO);
            let copies = self.duplicate_objects(&ids, Point::ZERO);
            self.transform_objects(&copies, &Transform::translate(d.x, d.y));
            new.extend(copies);
        }
        if let Some(r) = self.repeat.as_mut() {
            r.show = false;
        }
        new
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path};

    use super::*;
    use crate::{Anchor, ObjectKind, Rotation, ThreadId};

    fn line(d: &mut Design, a: Point, b: Point) {
        d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::new().move_to(a).line_to(b),
                params: Default::default(),
            },
        );
    }

    #[test]
    fn offsets_in_columns_and_rows() {
        let s = RepeatSpec {
            columns: 3,
            rows: 2,
            step_x: Point::new(10.0, 0.0),
            step_y: Point::new(2.0, 5.0),
            show: true,
        };
        let o = s.offsets();
        assert_eq!(o.len(), 5);
        assert_eq!(o[0], Point::new(10.0, 0.0));
        assert_eq!(o[4], Point::new(22.0, 5.0));
        let many = RepeatSpec {
            columns: 50,
            rows: 50,
            ..s
        };
        assert_eq!(many.offsets().len(), MAX_COPIES as usize);
    }

    #[test]
    fn join_end_to_start_and_make_the_copies() {
        let mut d = Design::new("border");
        line(&mut d, Point::new(0.0, 0.0), Point::new(40.0, 0.0));
        d.start_end.start = Some(Anchor::DesignBox { x: 0.0, y: 0.5 });
        d.start_end.end = Some(Anchor::DesignBox { x: 1.0, y: 0.5 });
        let step = d.join_step().unwrap();
        assert_eq!(step, Point::new(40.0, 0.0));
        d.repeat = Some(RepeatSpec {
            columns: 3,
            rows: 1,
            step_x: step,
            step_y: Point::ZERO,
            show: true,
        });
        let new = d.make_repeats();
        assert_eq!(new.len(), 2);
        let b = d.extents().unwrap();
        assert_eq!((b.min.x, b.max.x), (0.0, 120.0));
        assert!(!d.repeat.unwrap().show);
        // Each copy starts where the one before ends.
        let starts: Vec<f64> = d.objects().iter().map(|o| o.control_bbox().unwrap().min.x).collect();
        assert_eq!(starts, vec![0.0, 40.0, 80.0]);
    }

    #[test]
    fn steps_are_on_the_canvas_with_the_design_turned() {
        let mut d = Design::new("t");
        line(&mut d, Point::new(0.0, 0.0), Point::new(0.0, 30.0));
        d.rotation = Rotation::Deg90;
        // On the canvas the line goes across: a step to the right.
        d.repeat = Some(RepeatSpec {
            columns: 2,
            rows: 1,
            step_x: Point::new(30.0, 0.0),
            step_y: Point::ZERO,
            show: true,
        });
        d.make_repeats();
        let rot = d.rotation_transform();
        let view: Vec<BBox> = d
            .objects()
            .iter()
            .map(|o| {
                let b = o.control_bbox().unwrap();
                BBox::from_points([rot.apply(b.min), rot.apply(b.max)]).unwrap()
            })
            .collect();
        assert!((view[1].min.x - view[0].min.x - 30.0).abs() < 1e-9, "{view:?}");
        assert!((view[1].min.y - view[0].min.y).abs() < 1e-9);
    }
}
