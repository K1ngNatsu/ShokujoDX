//! Objects from outlines and a stitch file (P18 Milestone 7, ADR 0137).
//!
//! Many programs save the outlines of a design as a vector file. With the
//! machine file of the same design, the outlines become the shapes of the
//! objects: each outline takes the kind, the settings and the thread of the
//! objects that the parts of the stitches inside it make (Milestones 1 to
//! 3, [`crate::part_objects`]).
//!
//! - The outlines and the stitches are aligned by their sizes
//!   ([`fit_by_size`]: the centers of their boxes, and the scale when the
//!   sizes are not the same) or by two points ([`fit_two_points`]).
//! - Each made object goes with the outline that has most of its needle
//!   points inside it or on its edge (an area), or near it (a line). When
//!   more outlines have them all, the smallest one takes them: a detail on
//!   a larger shape.
//! - An area with the stitches of a fill becomes a fill of the outline, with
//!   the settings of the fill. An area with the stitches of a column becomes
//!   a satin column of the outline, with the stitch directions of the
//!   column (its ends and its rungs are the angle lines), and the pull
//!   compensation that makes the column as wide as its stitches; an area
//!   with one hole is a ring column. A line with the stitches of a run
//!   becomes a run along the line; with the stitches of a column, a satin
//!   column of the width of the stitches along the line.
//! - The stitches that no outline takes stay as Recognize Objects makes
//!   them. No stitch is lost.

use std::ops::Range;

use emb_geom::{BBox, Path, Point, Transform, math, region};
use emb_model::{CurrentProperties, ObjectKind, SatinGeometry, StitchDesign, vector};

use crate::part_objects::{Made, points_of};

/// An outline of a vector file, in mm.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub path: Path,
    /// An area (a filled shape), or a line (a stroke only).
    pub filled: bool,
    /// The width of the stroke of a line, in mm.
    pub width_mm: f64,
    /// The name of the shape in the file.
    pub name: Option<String>,
}

/// A needle point on the edge of an area, or this near it, counts as in it
/// (the stitches go out by their pull compensation), mm.
const EDGE_MM: f64 = 0.8;
/// A needle point this near a line (more than half its stroke width) is on
/// it, mm.
const LINE_MM: f64 = 1.0;
/// An outline takes a made object when this part of its needle points is in
/// the outline.
const CLAIM: f64 = 0.7;
/// Outlines that have nearly as many of the points as the best one: the
/// smallest of them takes the object.
const NEARLY: f64 = 0.05;
/// The most needle points of a made object that the match looks at.
const SAMPLES: usize = 300;
/// Sizes that differ by less than this part are the same size.
const SAME_SIZE: f64 = 0.05;
/// Tolerance to flatten the outlines, mm.
const FLAT_MM: f64 = 0.1;

/// The box of the outlines.
pub fn outlines_bbox(outlines: &[Outline]) -> Option<BBox> {
    outlines
        .iter()
        .filter_map(|o| o.path.control_bbox())
        .reduce(BBox::union)
}

/// The transform that puts the outlines of the box `outlines` on the
/// stitches of the box `stitches`: the centers on each other, and a scale
/// when the sizes differ by more than 5 % (a file in other units).
pub fn fit_by_size(outlines: BBox, stitches: BBox) -> Transform {
    let size = |b: &BBox| b.width() + b.height();
    let ratio = if size(&outlines) > 0.0 {
        size(&stitches) / size(&outlines)
    } else {
        1.0
    };
    let s = if (ratio - 1.0).abs() > SAME_SIZE { ratio } else { 1.0 };
    let (c, d) = (outlines.center(), stitches.center());
    Transform::translate(-c.x, -c.y)
        .then(Transform::scale(s, s))
        .then(Transform::translate(d.x, d.y))
}

/// The transform that moves the two outline points `from` onto the two
/// stitch points `to` (a move, a turn and a scale). `None` if the points of
/// a pair are on each other.
pub fn fit_two_points(from: [Point; 2], to: [Point; 2]) -> Option<Transform> {
    let (u, v) = (from[1] - from[0], to[1] - to[0]);
    if u.length() < 1e-6 || v.length() < 1e-6 {
        return None;
    }
    let s = v.length() / u.length();
    let turn = math::to_degrees(math::atan2(v.y, v.x) - math::atan2(u.y, u.x));
    Some(
        Transform::translate(-from[0].x, -from[0].y)
            .then(Transform::scale(s, s))
            .then(Transform::rotate_deg(turn))
            .then(Transform::translate(to[0].x, to[0].y)),
    )
}

/// The outlines moved by `t`.
pub fn transformed(outlines: &[Outline], t: &Transform) -> Vec<Outline> {
    outlines
        .iter()
        .map(|o| Outline {
            path: o.path.transformed(t),
            ..o.clone()
        })
        .collect()
}

/// Each area of each outline as an outline of its own (an outer shape and
/// its holes); lines stay as they are.
pub fn split_areas(outlines: &[Outline]) -> Vec<Outline> {
    let mut out = Vec::new();
    for o in outlines {
        if !o.filled {
            out.push(o.clone());
            continue;
        }
        let closed: Vec<Path> = o.path.subpaths().into_iter().map(Path::closed).collect();
        for (outer, holes) in region::nest(&closed) {
            let mut subs = vec![outer];
            subs.extend(holes);
            out.push(Outline {
                path: Path::joined(&subs),
                ..o.clone()
            });
        }
    }
    out
}

/// An outline, flattened for the match.
struct Shape {
    rings: Vec<Vec<Point>>,
    bbox: BBox,
    /// The area of an area, the length of a line (to find the smallest).
    size: f64,
    /// The distance from a line that counts as on it.
    near: f64,
}

impl Shape {
    fn new(o: &Outline) -> Option<Shape> {
        let rings: Vec<Vec<Point>> = o
            .path
            .flatten(FLAT_MM)
            .into_iter()
            .map(|pl| {
                let mut pts = pl.points;
                if (o.filled || pl.closed) && pts.len() > 2 && pts.first() != pts.last() {
                    pts.push(pts[0]);
                }
                pts
            })
            .filter(|r| r.len() > 1)
            .collect();
        let near = if o.filled { EDGE_MM } else { o.width_mm / 2.0 + LINE_MM };
        let bbox = BBox::from_points(rings.iter().flatten().copied())?.expand(near);
        let size = if o.filled {
            let areas: Vec<f64> = rings.iter().map(|r| region::ring_signed_area(r).abs()).collect();
            let outer = areas.iter().copied().fold(0.0, f64::max);
            outer - (areas.iter().sum::<f64>() - outer)
        } else {
            rings
                .iter()
                .map(|r| r.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>())
                .sum()
        };
        Some(Shape {
            rings,
            bbox,
            size,
            near,
        })
    }

    fn edge_distance(&self, p: Point) -> f64 {
        self.rings
            .iter()
            .flat_map(|r| r.windows(2))
            .map(|w| p.distance_to_segment(w[0], w[1]))
            .fold(f64::INFINITY, f64::min)
    }

    /// The point is in the area (even-odd) or near its edge; near a line.
    fn has(&self, p: Point, filled: bool) -> bool {
        if !self.bbox.contains(p) {
            return false;
        }
        if filled && self.rings.iter().filter(|r| region::ring_contains(r, p)).count() % 2 == 1 {
            return true;
        }
        self.edge_distance(p) <= self.near
    }
}

/// Up to [`SAMPLES`] needle points of a made object, evenly.
fn samples(s: &StitchDesign, records: &[Range<usize>]) -> Vec<Point> {
    let all: Vec<Point> = records.iter().flat_map(|r| points_of(s, r)).collect();
    if all.len() <= SAMPLES {
        return all;
    }
    (0..SAMPLES).map(|i| all[i * all.len() / SAMPLES]).collect()
}

/// The kinds of objects that decide an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Fill,
    Satin,
    Run,
}

fn kind_of(k: &ObjectKind) -> Option<Kind> {
    match k {
        ObjectKind::Fill { .. } => Some(Kind::Fill),
        ObjectKind::Satin { .. } => Some(Kind::Satin),
        ObjectKind::Running { .. } => Some(Kind::Run),
        _ => None,
    }
}

/// The made objects of the stitches `s` with the shapes of the outlines
/// (already aligned with the stitches): an outline that takes made objects
/// becomes the objects of its shape, at the place in the sew order of its
/// first stitch; the other made objects stay. The second value is the
/// outline of each made object (`None`: as Recognize Objects made it).
pub fn with_outlines(s: &StitchDesign, made: Vec<Made>, outlines: &[Outline]) -> (Vec<Made>, Vec<Option<usize>>) {
    let shapes: Vec<Option<Shape>> = outlines.iter().map(Shape::new).collect();
    // The outline that takes each made object.
    let mut owner: Vec<Option<usize>> = vec![None; made.len()];
    for (i, m) in made.iter().enumerate() {
        let pts = samples(s, &m.records);
        if pts.is_empty() {
            continue;
        }
        let Some(mb) = BBox::from_points(pts.iter().copied()) else {
            continue;
        };
        let scores: Vec<(usize, f64, f64)> = shapes
            .iter()
            .enumerate()
            .filter_map(|(j, sh)| {
                let sh = sh.as_ref()?;
                if !sh.bbox.intersects(&mb) {
                    return None;
                }
                let n = pts.iter().filter(|p| sh.has(**p, outlines[j].filled)).count();
                Some((j, n as f64 / pts.len() as f64, sh.size))
            })
            .filter(|&(_, f, _)| f >= CLAIM)
            .collect();
        let best = scores.iter().map(|x| x.1).fold(0.0, f64::max);
        owner[i] = scores
            .iter()
            .filter(|x| x.1 >= best - NEARLY)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|x| x.0);
    }
    let mut out: Vec<(usize, Made, Option<usize>)> = Vec::new();
    let mut taken = vec![false; made.len()];
    for (j, o) in outlines.iter().enumerate() {
        let Some(sh) = &shapes[j] else { continue };
        let candidates: Vec<usize> = (0..made.len())
            .filter(|&i| owner[i] == Some(j) && kind_of(&made[i].kind).is_some())
            .collect();
        // The object that covers most of the outline decides the kind and
        // gives the settings (a piece of another object inside the outline
        // does not); the outline takes the objects of that kind.
        let cover = |i: usize| {
            let pts = samples(s, &made[i].records);
            BBox::from_points(pts).map_or(0.0, |b| {
                let w = (b.max.x.min(sh.bbox.max.x) - b.min.x.max(sh.bbox.min.x)).max(0.0);
                let h = (b.max.y.min(sh.bbox.max.y) - b.min.y.max(sh.bbox.min.y)).max(0.0);
                w * h / (sh.bbox.width() * sh.bbox.height()).max(1e-9)
            })
        };
        let Some(lead) = candidates.iter().copied().max_by(|&a, &b| {
            cover(a)
                .total_cmp(&cover(b))
                .then(made[a].stitch_count().cmp(&made[b].stitch_count()))
        }) else {
            continue;
        };
        let Some(kind) = kind_of(&made[lead].kind) else {
            continue;
        };
        let mine: Vec<usize> = candidates
            .into_iter()
            .filter(|&i| kind_of(&made[i].kind) == Some(kind))
            .collect();
        let Some(kinds) = object_of(o, kind, &made[lead].kind) else {
            continue;
        };
        let mut records: Vec<Range<usize>> = mine.iter().flat_map(|&i| made[i].records.clone()).collect();
        records.sort_by_key(|r| r.start);
        let first = records.first().map_or(usize::MAX, |r| r.start);
        for &i in &mine {
            taken[i] = true;
        }
        for (n, k) in kinds.into_iter().enumerate() {
            out.push((
                first,
                Made {
                    kind: k,
                    thread: made[lead].thread,
                    records: if n == 0 { records.clone() } else { Vec::new() },
                },
                Some(j),
            ));
        }
    }
    for (i, m) in made.into_iter().enumerate() {
        if !taken[i] {
            let first = m.records.first().map_or(usize::MAX, |r| r.start);
            out.push((first, m, None));
        }
    }
    out.sort_by_key(|x| x.0);
    out.into_iter().map(|(_, m, j)| (m, j)).unzip()
}

/// The mean width of a column, mm.
fn width_of(g: &SatinGeometry) -> Option<f64> {
    emb_stitch::satin::Column::new(g).map(|c| c.mean_width())
}

/// The objects of the outline `o` with the kind and the settings of the
/// made object `lead`. `None` if the outline cannot be such an object.
fn object_of(o: &Outline, kind: Kind, lead: &ObjectKind) -> Option<Vec<ObjectKind>> {
    let subs = o.path.subpaths();
    match (kind, lead, o.filled) {
        (Kind::Fill, ObjectKind::Fill { params, .. }, true) => {
            let closed: Vec<Path> = subs.into_iter().map(Path::closed).collect();
            let (outer, holes) = region::nest(&closed).into_iter().next()?;
            Some(vec![ObjectKind::Fill {
                outer,
                holes,
                params: params.clone(),
            }])
        }
        (Kind::Satin, ObjectKind::Satin { geometry, params }, true) => {
            let closed: Vec<Path> = subs.into_iter().map(Path::closed).collect();
            let (outer, holes) = region::nest(&closed).into_iter().next()?;
            let column = if holes.is_empty() {
                // The stitch directions of the column: its ends and rungs.
                let ends = |p: &Path| -> Option<(Point, Point)> {
                    let pts: Vec<Point> = p.flatten(FLAT_MM).into_iter().next()?.points;
                    Some((*pts.first()?, *pts.last()?))
                };
                let ((a0, a1), (b0, b1)) = (ends(&geometry.rail_a)?, ends(&geometry.rail_b)?);
                let mut lines = vec![(a0, b0)];
                lines.extend(geometry.rungs.iter().copied());
                lines.push((a1, b1));
                SatinGeometry::from_outline(&outer, &lines)
            } else {
                None
            };
            let made = match column {
                Some(g) => vec![ObjectKind::Satin {
                    geometry: g,
                    params: params.clone(),
                }],
                None => {
                    let props = CurrentProperties {
                        satin: params.clone(),
                        ..CurrentProperties::default()
                    };
                    let mut subs = vec![outer];
                    subs.extend(holes);
                    vector::from_vector(&Path::joined(&subs), vector::ConvertTo::Satin, &props, 1.0)
                }
            };
            // The stitches are wider than the outline by the pull
            // compensation.
            let stitched = width_of(geometry)?;
            let made: Vec<ObjectKind> = made
                .into_iter()
                .map(|mut k| {
                    if let ObjectKind::Satin { geometry: g, params: p } = &mut k
                        && let Some(w) = width_of(g)
                    {
                        p.pull_compensation_mm = ((stitched - w) / 2.0).clamp(0.0, 1.0);
                    }
                    k
                })
                .collect();
            (!made.is_empty()).then_some(made)
        }
        (Kind::Run, ObjectKind::Running { params, .. }, false) => Some(vec![ObjectKind::Running {
            path: o.path.clone(),
            params: params.clone(),
        }]),
        (Kind::Satin, ObjectKind::Satin { geometry, params }, false) => {
            let w = width_of(geometry)?;
            Some(
                subs.iter()
                    .map(|line| ObjectKind::Satin {
                        geometry: SatinGeometry::from_centerline(line, w),
                        params: params.clone(),
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
