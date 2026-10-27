//! Offsets and arrangements (P6 Milestone 3).
//!
//! Offsets: outlines at a distance round the selected objects (outside)
//! or inside them. The areas of the objects are joined first, so one
//! outline goes round objects that touch. A line gets an outline round it
//! (an open offset). Several outlines at a spacing are outline offsets.
//! Each outline is a running line, a vector, or a border (a line with a
//! satin width). The border sits outside the outline (so that it does not
//! cover the object), centered on it, or inside it ([`BorderPlace`],
//! owner request of 2026-10-26).
//!
//! Arrangements: copies of the selected objects. Mirror-merge puts a mirror
//! copy next to the selection (right, below, or both and the corner). An
//! array puts copies in rows and columns. A wreath turns copies round a
//! center below the selection; a kaleidoscope also mirrors every second
//! copy.

use emb_geom::region::{self, Join, Region};
use emb_geom::{BBox, Path, Point, Transform};

use crate::shaping::{ShapeError, area_of, even_odd};
use serde::{Deserialize, Serialize};

use crate::{Design, LineBorder, ObjectId, ObjectKind, Rgb, RunningParams, VectorStyle};

/// Tolerance to flatten lines for open offsets, in mm.
const FLATTEN_TOL_MM: f64 = 0.05;

/// Where a border sits on the line that it follows (an outline of
/// Offsets, or the edge of the objects of a border trace). Outside and
/// inside are away from the middle of the shape and toward it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderPlace {
    /// The inner edge of the border is on the line.
    #[default]
    Outside,
    /// The middle of the border is on the line.
    Center,
    /// The outer edge of the border is on the line.
    Inside,
}

impl BorderPlace {
    pub const ALL: [BorderPlace; 3] = [BorderPlace::Outside, BorderPlace::Center, BorderPlace::Inside];

    pub fn name(self) -> &'static str {
        match self {
            BorderPlace::Outside => "Outside the line",
            BorderPlace::Center => "Centered on the line",
            BorderPlace::Inside => "Inside the line",
        }
    }

    /// How far out from the line the middle of a border `width_mm` wide
    /// goes (negative: in).
    pub fn middle_mm(self, width_mm: f64) -> f64 {
        match self {
            BorderPlace::Outside => width_mm / 2.0,
            BorderPlace::Center => 0.0,
            BorderPlace::Inside => -width_mm / 2.0,
        }
    }

    pub fn is_outside(&self) -> bool {
        *self == BorderPlace::Outside
    }
}

/// What an offset outline becomes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OffsetMake {
    /// A running line with the current running settings.
    Line,
    /// A vector with a line in the thread color.
    Vector,
    /// A satin border of this width, at `place` on the outline.
    Border { width_mm: f64, place: BorderPlace },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetSpec {
    /// The distance of the first outline: outside if positive, inside if
    /// negative, on the outline if zero.
    pub distance_mm: f64,
    /// The number of outlines.
    pub count: u32,
    /// The distance between two outlines.
    pub spacing_mm: f64,
    pub corners: Join,
    pub make: OffsetMake,
}

impl Default for OffsetSpec {
    fn default() -> Self {
        OffsetSpec {
            distance_mm: 2.0,
            count: 1,
            spacing_mm: 2.0,
            corners: Join::Round,
            make: OffsetMake::Line,
        }
    }
}

/// A set of copies of the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arrangement {
    /// A mirror copy to the right (`horizontal`), below (`vertical`), or
    /// both (and a copy turned 180° in the corner). `gap_mm` is the space
    /// between the selection and its mirror copy.
    MirrorMerge {
        horizontal: bool,
        vertical: bool,
        gap_mm: f64,
    },
    /// Copies in rows and columns with gaps between them.
    Array {
        rows: u32,
        columns: u32,
        gap_x_mm: f64,
        gap_y_mm: f64,
    },
    /// `count` objects in all round a center `radius_mm` below the middle
    /// of the selection.
    Wreath { count: u32, radius_mm: f64 },
    /// As a wreath, with a mirror copy in each segment.
    Kaleidoscope { count: u32, radius_mm: f64 },
}

/// A mirror about the line through `c` at `angle_deg`.
pub fn mirror_about(c: Point, angle_deg: f64) -> Transform {
    Transform::translate(-c.x, -c.y)
        .then(Transform::rotate_deg(-angle_deg))
        .then(Transform::scale(1.0, -1.0))
        .then(Transform::rotate_deg(angle_deg))
        .then(Transform::translate(c.x, c.y))
}

fn rotate_about(c: Point, deg: f64) -> Transform {
    Transform::translate(-c.x, -c.y)
        .then(Transform::rotate_deg(deg))
        .then(Transform::translate(c.x, c.y))
}

/// The closed outlines of `regions` as one path.
pub(crate) fn outline_path(regions: &[Region]) -> Path {
    let rings: Vec<Path> = regions
        .iter()
        .flat_map(|r| std::iter::once(&r.outer).chain(&r.holes))
        .filter(|ring| ring.len() >= 3)
        .map(|ring| Path::polygon(ring))
        .collect();
    Path::joined(&rings)
}

impl Design {
    /// The areas of the objects (lettering by the area of its letters) and
    /// the lines of the running objects.
    pub(crate) fn outline_sources(&self, ids: &[ObjectId]) -> (Vec<Region>, Vec<emb_geom::Polyline>) {
        let mut area = Vec::new();
        let mut lines = Vec::new();
        for o in ids.iter().filter_map(|id| self.object(*id)) {
            match &o.kind {
                ObjectKind::Running { path, .. } => lines.extend(path.flatten(FLATTEN_TOL_MM)),
                ObjectKind::Lettering(_) => {
                    area = region::union(&area, &even_odd(&crate::vector::area_outline(&o.kind)));
                }
                k => {
                    if let Some(a) = area_of(k) {
                        area = region::union(&area, &a);
                    }
                }
            }
        }
        (area, lines)
    }

    /// Adds offset outlines of the objects `ids` after the last of them
    /// (see the module notes). Returns the new objects.
    pub fn add_offsets(&mut self, ids: &[ObjectId], spec: &OffsetSpec) -> Result<Vec<ObjectId>, ShapeError> {
        let (area, lines) = self.outline_sources(ids);
        if area.is_empty() && lines.is_empty() {
            return Err(ShapeError::NoOutline);
        }
        let first = ids
            .iter()
            .filter_map(|id| self.object(*id))
            .next()
            .ok_or(ShapeError::NoOutline)?;
        let thread = first.thread;
        let color = self.thread(thread).map_or(Rgb(0, 0, 0), |t| t.color);
        let at = ids
            .iter()
            .filter_map(|id| self.index_of(*id))
            .max()
            .map_or(0, |i| i + 1);
        let sign = if spec.distance_mm < 0.0 { -1.0 } else { 1.0 };
        // The middle of a border, from its outline (out is positive, also
        // for outlines inside the objects).
        let border_middle = match spec.make {
            OffsetMake::Border { width_mm, place } => place.middle_mm(width_mm),
            _ => 0.0,
        };
        let mut kinds = Vec::new();
        for k in 0..spec.count.max(1) {
            let d = spec.distance_mm + sign * f64::from(k) * spec.spacing_mm + border_middle;
            let mut regions = if d == 0.0 {
                area.clone()
            } else {
                region::offset(&area, d, spec.corners)
            };
            if d > 0.0 {
                regions = region::union(&regions, &region::offset_lines(&lines, d, spec.corners));
            }
            let path = outline_path(&regions);
            if path.is_empty() {
                continue;
            }
            kinds.push(match spec.make {
                OffsetMake::Line => ObjectKind::Running {
                    path,
                    params: self.current.running.clone(),
                },
                OffsetMake::Vector => ObjectKind::Vector {
                    path,
                    style: VectorStyle::stroked(color),
                },
                OffsetMake::Border { width_mm, .. } => ObjectKind::Running {
                    path,
                    params: RunningParams {
                        border: Some(LineBorder {
                            width_mm,
                            column: self.current.satin.clone(),
                            corner: Default::default(),
                        }),
                        ..self.current.running.clone()
                    },
                },
            });
        }
        if kinds.is_empty() {
            return Err(ShapeError::NothingLeft);
        }
        Ok(kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| self.insert_object(at + i, thread, kind))
            .collect())
    }

    /// Adds the copies of an arrangement of the objects `ids` and returns
    /// them. The copies come at the end of the sew order.
    pub fn arrange(&mut self, ids: &[ObjectId], arrangement: &Arrangement) -> Vec<ObjectId> {
        let Some(b) = ids
            .iter()
            .filter_map(|id| self.object(*id))
            .filter_map(|o| o.control_bbox())
            .reduce(BBox::union)
        else {
            return Vec::new();
        };
        let mut new = Vec::new();
        for t in arrangement_transforms(&b, arrangement) {
            let copies = self.duplicate_objects(ids, Point::ZERO);
            self.transform_objects(&copies, &t);
            new.extend(copies);
        }
        new
    }
}

/// The transform of each copy of an arrangement of objects in box `b`.
pub fn arrangement_transforms(b: &BBox, arrangement: &Arrangement) -> Vec<Transform> {
    match *arrangement {
        Arrangement::MirrorMerge {
            horizontal,
            vertical,
            gap_mm,
        } => {
            let x = b.max.x + gap_mm / 2.0;
            let y = b.max.y + gap_mm / 2.0;
            let h = mirror_about(Point::new(x, 0.0), 90.0);
            let v = mirror_about(Point::new(0.0, y), 0.0);
            match (horizontal, vertical) {
                (true, true) => vec![h, v, h.then(v)],
                (true, false) => vec![h],
                (false, true) => vec![v],
                (false, false) => Vec::new(),
            }
        }
        Arrangement::Array {
            rows,
            columns,
            gap_x_mm,
            gap_y_mm,
        } => {
            let (dx, dy) = (b.width() + gap_x_mm, b.height() + gap_y_mm);
            (0..rows.max(1))
                .flat_map(|r| (0..columns.max(1)).map(move |c| (r, c)))
                .filter(|&(r, c)| (r, c) != (0, 0))
                .map(|(r, c)| Transform::translate(f64::from(c) * dx, f64::from(r) * dy))
                .collect()
        }
        Arrangement::Wreath { count, radius_mm } => {
            let c = b.center() + Point::new(0.0, radius_mm);
            let n = count.max(1);
            (1..n)
                .map(|k| rotate_about(c, 360.0 * f64::from(k) / f64::from(n)))
                .collect()
        }
        Arrangement::Kaleidoscope { count, radius_mm } => {
            let c = b.center() + Point::new(0.0, radius_mm);
            let n = count.max(1);
            let step = 360.0 / f64::from(n);
            // The selection is above the center (at -90°); the mirror line
            // is half a segment from it.
            let m = mirror_about(c, -90.0 + step / 2.0);
            let mut out: Vec<Transform> = (1..n).map(|k| rotate_about(c, step * f64::from(k))).collect();
            out.extend((0..n).map(|k| m.then(rotate_about(c, step * f64::from(k)))));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FillParams;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn design() -> (Design, ObjectId, ObjectId) {
        let mut d = Design::new("a");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let fill = d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(p(0.0, 0.0), p(20.0, 10.0))),
                holes: Vec::new(),
                params: FillParams::default(),
            },
        );
        let line = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(p(0.0, 30.0)).line_to(p(20.0, 30.0)),
                params: RunningParams::default(),
            },
        );
        (d, fill, line)
    }

    fn bbox(d: &Design, id: ObjectId) -> BBox {
        d.object(id).unwrap().control_bbox().unwrap()
    }

    #[test]
    fn outside_inside_and_several_outlines() {
        let (mut d, fill, _) = design();
        let spec = OffsetSpec {
            distance_mm: 2.0,
            corners: Join::Miter,
            ..OffsetSpec::default()
        };
        let new = d.add_offsets(&[fill], &spec).unwrap();
        assert_eq!(new.len(), 1);
        let b = bbox(&d, new[0]);
        assert!(
            (b.width() - 24.0).abs() < 1e-6 && (b.height() - 14.0).abs() < 1e-6,
            "{b:?}"
        );
        assert_eq!(d.index_of(new[0]), Some(1), "after the object");

        let spec = OffsetSpec {
            distance_mm: -1.0,
            count: 3,
            spacing_mm: 1.5,
            corners: Join::Miter,
            ..OffsetSpec::default()
        };
        let new = d.add_offsets(&[fill], &spec).unwrap();
        assert_eq!(new.len(), 3);
        let widths: Vec<f64> = new.iter().map(|id| bbox(&d, *id).width()).collect();
        for (w, want) in widths.iter().zip([18.0, 15.0, 12.0]) {
            assert!((w - want).abs() < 1e-6, "{widths:?}");
        }
        // Inside a 10 mm high shape, an outline 6 mm in has nothing left.
        let spec = OffsetSpec {
            distance_mm: -6.0,
            ..OffsetSpec::default()
        };
        assert_eq!(d.add_offsets(&[fill], &spec), Err(ShapeError::NothingLeft));
    }

    /// The owner request of 2026-10-26: a border outside its outline,
    /// centered on it, or inside it; outside and inside do not turn round
    /// for an outline inside the objects.
    #[test]
    fn a_border_sits_outside_centered_or_inside_its_outline() {
        // (distance, place, the width of the line in the middle of the
        // border round the 20 × 10 mm fill)
        for (distance, place, width) in [
            (1.0, BorderPlace::Outside, 26.0),
            (1.0, BorderPlace::Center, 22.0),
            (1.0, BorderPlace::Inside, 18.0),
            (0.0, BorderPlace::Center, 20.0),
            (-2.0, BorderPlace::Outside, 20.0),
            (-2.0, BorderPlace::Inside, 12.0),
        ] {
            let (mut d, fill, _) = design();
            let spec = OffsetSpec {
                distance_mm: distance,
                make: OffsetMake::Border { width_mm: 4.0, place },
                corners: Join::Miter,
                ..OffsetSpec::default()
            };
            let new = d.add_offsets(&[fill], &spec).unwrap();
            let b = bbox(&d, new[0]);
            assert!((b.width() - width).abs() < 1e-6, "{distance} {place:?}: {b:?}");
        }
    }

    #[test]
    fn open_offset_border_and_vector() {
        let (mut d, fill, line) = design();
        let new = d
            .add_offsets(
                &[line],
                &OffsetSpec {
                    distance_mm: 1.0,
                    ..OffsetSpec::default()
                },
            )
            .unwrap();
        let b = bbox(&d, new[0]);
        assert!(
            (b.width() - 22.0).abs() < 0.05 && (b.height() - 2.0).abs() < 0.05,
            "{b:?}"
        );

        let spec = OffsetSpec {
            distance_mm: 0.5,
            make: OffsetMake::Border {
                width_mm: 3.0,
                place: BorderPlace::Outside,
            },
            corners: Join::Miter,
            ..OffsetSpec::default()
        };
        let new = d.add_offsets(&[fill], &spec).unwrap();
        let o = d.object(new[0]).unwrap();
        let ObjectKind::Running { params, .. } = &o.kind else {
            panic!()
        };
        assert_eq!(params.border.as_ref().map(|b| b.width_mm), Some(3.0));
        // The middle of the border is 0.5 + 1.5 mm out, so its inner edge
        // is 0.5 mm from the fill.
        let b = bbox(&d, new[0]);
        assert!((b.width() - 24.0).abs() < 1e-6, "{b:?}");

        let spec = OffsetSpec {
            make: OffsetMake::Vector,
            ..OffsetSpec::default()
        };
        let new = d.add_offsets(&[fill, line], &spec).unwrap();
        assert!(
            matches!(d.object(new[0]).unwrap().kind, ObjectKind::Vector { style, .. } if style.stroke == Some(Rgb(200, 0, 0)))
        );
    }

    #[test]
    fn mirror_merge_array_wreath_kaleidoscope() {
        let count = |a: Arrangement| {
            let (mut d, fill, _) = design();
            let new = d.arrange(&[fill], &a);
            (d, new)
        };
        let (d, new) = count(Arrangement::MirrorMerge {
            horizontal: true,
            vertical: false,
            gap_mm: 2.0,
        });
        assert_eq!(new.len(), 1);
        let b = bbox(&d, new[0]);
        assert!((b.min.x - 22.0).abs() < 1e-9 && (b.max.x - 42.0).abs() < 1e-9, "{b:?}");
        let (_, new) = count(Arrangement::MirrorMerge {
            horizontal: true,
            vertical: true,
            gap_mm: 0.0,
        });
        assert_eq!(new.len(), 3);

        let (d, new) = count(Arrangement::Array {
            rows: 2,
            columns: 3,
            gap_x_mm: 5.0,
            gap_y_mm: 1.0,
        });
        assert_eq!(new.len(), 5);
        let last = bbox(&d, *new.last().unwrap());
        assert!(
            (last.min.x - 50.0).abs() < 1e-9 && (last.min.y - 11.0).abs() < 1e-9,
            "{last:?}"
        );

        let (d, new) = count(Arrangement::Wreath {
            count: 4,
            radius_mm: 30.0,
        });
        assert_eq!(new.len(), 3);
        // The copy opposite the selection is below the center (10, 35).
        let opposite = bbox(&d, new[1]).center();
        assert!(opposite.distance(p(10.0, 65.0)) < 1e-9, "{opposite:?}");

        let (_, new) = count(Arrangement::Kaleidoscope {
            count: 6,
            radius_mm: 30.0,
        });
        assert_eq!(new.len(), 11, "5 turned copies and 6 mirror copies");
    }
}
