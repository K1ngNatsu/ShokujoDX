//! Shaping (P6 Milestone 2): boolean operations on the areas of objects.
//!
//! Objects take part by their area: fills, vectors with closed shapes,
//! appliqué objects and satin columns (the area inside the column).
//! Lines, lettering and stitch objects do not take part.
//!
//! An object that sews later is in front of the objects before it.
//!
//! - Weld: one area made of all areas. Intersect: the area that all
//!   objects cover. Exclude: the area that an odd number of objects cover.
//! - Front minus back: the front object, with the others cut out. Back
//!   minus front: the back object, with the others cut out.
//! - Flatten: each object, with the objects in front of it cut out (the
//!   parts that do not show).
//! - Divide: the areas cut into pieces where they overlap. A piece where
//!   objects overlap belongs to the front object.
//! - Combine: vectors become one vector with all their shapes; appliqué
//!   objects become one appliqué object with all their pieces.
//!
//! The result of weld, intersect, exclude and back minus front has the
//! kind, thread and settings of the back object; front minus back those
//! of the front object. A satin column that keeps an area becomes a fill
//! with the current fill settings. With "keep originals", the objects stay
//! and the results come after them. The overlap amount makes a cut piece
//! go that far under the object that cut it, so that no fabric shows
//! between two pieces of embroidery.

use std::collections::BTreeSet;

use emb_geom::region::{self, Join, Region};
use emb_geom::{Path, Point};

use crate::applique::AppliquePiece;
use crate::{Design, ObjectId, ObjectKind};

/// Tolerance to flatten the outlines of the objects, in mm.
const FLATTEN_TOL_MM: f64 = 0.05;
/// Pieces smaller than this are left out, in mm².
const MIN_AREA_MM2: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeOp {
    Weld,
    Intersect,
    Exclude,
    FrontMinusBack,
    BackMinusFront,
    Flatten,
    Divide,
    Combine,
}

impl ShapeOp {
    pub const ALL: [ShapeOp; 8] = [
        ShapeOp::Weld,
        ShapeOp::Intersect,
        ShapeOp::Exclude,
        ShapeOp::FrontMinusBack,
        ShapeOp::BackMinusFront,
        ShapeOp::Flatten,
        ShapeOp::Divide,
        ShapeOp::Combine,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ShapeOp::Weld => "Weld",
            ShapeOp::Intersect => "Intersect",
            ShapeOp::Exclude => "Exclude",
            ShapeOp::FrontMinusBack => "Front minus back",
            ShapeOp::BackMinusFront => "Back minus front",
            ShapeOp::Flatten => "Flatten",
            ShapeOp::Divide => "Divide",
            ShapeOp::Combine => "Combine",
        }
    }
}

/// Settings of the shaping commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeOptions {
    /// The objects stay; the results come after them in the sew order.
    pub keep_originals: bool,
    /// Flatten, Divide and Remove overlaps stay linked: the cut is made
    /// again when the objects move (see `link`).
    pub linked: bool,
    /// How far a cut piece goes under the object that cut it, in mm.
    pub overlap_mm: f64,
}

impl Default for ShapeOptions {
    fn default() -> Self {
        ShapeOptions {
            keep_originals: false,
            linked: false,
            overlap_mm: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShapeError {
    #[error("Select two or more objects with an area (fills, vectors with closed shapes, appliqué, satin columns).")]
    TooFew,
    #[error("Nothing is left: the result has no area.")]
    NothingLeft,
    #[error("Combine needs two or more vectors, or two or more appliqué objects.")]
    CannotCombine,
    #[error("Select objects with an outline first (not stitch objects).")]
    NoOutline,
    #[error("Nothing to cut: no object under the selection overlaps it.")]
    NothingToCut,
    #[error(
        "Some of these objects are linked (a border trace or linked shapes): use Artwork > Unlink first. To change the stitches of one object, double-click it."
    )]
    AlreadyLinked,
    #[error("Set a border width, a gap width, or both.")]
    NothingToDo,
}

/// Returns `true` if an object of this kind takes part in shaping (a
/// quick check with no geometry work; [`area_of`] can still find no area).
pub fn takes_part(kind: &ObjectKind) -> bool {
    match kind {
        ObjectKind::Fill { .. } | ObjectKind::Applique { .. } | ObjectKind::Satin { .. } => true,
        ObjectKind::Vector { path, style } => style.fill.is_some() || path.is_closed(),
        ObjectKind::Running { .. }
        | ObjectKind::Stitches { .. }
        | ObjectKind::Lettering(_)
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => false,
    }
}

/// The area of an object kind, if it takes part in shaping.
pub fn area_of(kind: &ObjectKind) -> Option<Vec<Region>> {
    let path = match kind {
        ObjectKind::Fill { outer, holes, .. } => Path::joined(std::iter::once(outer).chain(holes)),
        ObjectKind::Vector { path, style } => {
            let closed = path.subpaths().iter().any(Path::is_closed);
            if style.fill.is_none() && !closed {
                return None;
            }
            path.clone()
        }
        ObjectKind::Applique { pieces, .. } => {
            let areas: Vec<Vec<Region>> = pieces
                .iter()
                .map(|p| even_odd(&Path::joined(std::iter::once(&p.outer).chain(&p.holes))))
                .collect();
            let all = areas.iter().fold(Vec::new(), |acc, a| region::union(&acc, a));
            return (!all.is_empty()).then_some(all);
        }
        ObjectKind::Satin { .. } => crate::vector::area_outline(kind),
        ObjectKind::Running { .. }
        | ObjectKind::Stitches { .. }
        | ObjectKind::Lettering(_)
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => return None,
    };
    let area = even_odd(&path);
    (!area.is_empty()).then_some(area)
}

/// The area inside the subpaths of `path` by the even-odd rule.
pub(crate) fn even_odd(path: &Path) -> Vec<Region> {
    path.flatten(FLATTEN_TOL_MM)
        .into_iter()
        .filter(|pl| pl.points.len() >= 3)
        .map(|pl| vec![Region::new(pl.points, Vec::new())])
        .fold(Vec::new(), |acc, r| region::xor(&acc, &r))
}

/// The area of `regions` without the pieces that are too small to sew.
pub(crate) fn clean(regions: Vec<Region>) -> Vec<Region> {
    regions.into_iter().filter(|r| r.area() >= MIN_AREA_MM2).collect()
}

/// A cutter made smaller by the overlap amount.
pub(crate) fn shrink(cutter: &[Region], overlap_mm: f64) -> Vec<Region> {
    if overlap_mm > 0.0 {
        region::offset(cutter, -overlap_mm, Join::Round)
    } else {
        cutter.to_vec()
    }
}

fn ring_path(ring: &[Point]) -> Path {
    Path::polygon(ring)
}

/// One object of the kind of `owner` that covers `regions` (also when
/// they are separate areas: one fill can have several outer shapes, as a
/// join makes). A satin column becomes a fill with `fill`. Empty if there
/// are no regions.
pub(crate) fn rebuild(owner: &ObjectKind, regions: &[Region], fill: &crate::FillParams) -> Vec<ObjectKind> {
    let fills = |params: &crate::FillParams| -> Vec<ObjectKind> {
        let outers: Vec<Path> = regions.iter().map(|r| ring_path(&r.outer)).collect();
        vec![ObjectKind::Fill {
            outer: Path::joined(&outers),
            holes: regions
                .iter()
                .flat_map(|r| r.holes.iter().map(|h| ring_path(h)))
                .collect(),
            params: params.clone(),
        }]
    };
    if regions.is_empty() {
        return Vec::new();
    }
    match owner {
        ObjectKind::Fill { params, .. } => fills(params),
        ObjectKind::Vector { style, .. } => {
            let rings: Vec<Path> = regions
                .iter()
                .flat_map(|r| std::iter::once(&r.outer).chain(&r.holes).map(|ring| ring_path(ring)))
                .collect();
            vec![ObjectKind::Vector {
                path: Path::joined(&rings),
                style: *style,
            }]
        }
        ObjectKind::Applique { params, .. } => vec![ObjectKind::Applique {
            pieces: regions
                .iter()
                .map(|r| AppliquePiece::new(ring_path(&r.outer), r.holes.iter().map(|h| ring_path(h)).collect()))
                .collect(),
            params: params.clone(),
        }],
        _ => fills(fill),
    }
}

/// One object that takes part: its id and area, in sew order.
pub(crate) struct Part {
    pub(crate) id: ObjectId,
    pub(crate) kind: ObjectKind,
    pub(crate) area: Vec<Region>,
}

/// The areas that a command (not Combine) makes of `parts`, each with the
/// index of its owner in `parts`, in sew order.
pub(crate) fn op_results(parts: &[Part], op: ShapeOp, overlap_mm: f64) -> Vec<(usize, Vec<Region>)> {
    let union_of = |ps: &[Part]| ps.iter().fold(Vec::new(), |acc, p| region::union(&acc, &p.area));
    let Some(last) = parts.len().checked_sub(1) else {
        return Vec::new();
    };
    match op {
        ShapeOp::Weld => vec![(0, union_of(parts))],
        ShapeOp::Intersect => vec![(
            0,
            parts[1..]
                .iter()
                .fold(parts[0].area.clone(), |acc, p| region::intersection(&acc, &p.area)),
        )],
        ShapeOp::Exclude => vec![(
            0,
            parts[1..]
                .iter()
                .fold(parts[0].area.clone(), |acc, p| region::xor(&acc, &p.area)),
        )],
        ShapeOp::FrontMinusBack => vec![(
            last,
            region::difference(&parts[last].area, &shrink(&union_of(&parts[..last]), overlap_mm)),
        )],
        ShapeOp::BackMinusFront => vec![(
            0,
            region::difference(&parts[0].area, &shrink(&union_of(&parts[1..]), overlap_mm)),
        )],
        ShapeOp::Flatten => (0..parts.len())
            .map(|i| {
                let front = union_of(&parts[i + 1..]);
                (i, region::difference(&parts[i].area, &shrink(&front, overlap_mm)))
            })
            .collect(),
        ShapeOp::Divide => divide(parts, overlap_mm),
        ShapeOp::Combine => Vec::new(),
    }
}

impl Design {
    /// Runs a shaping command on the objects `ids` (see the module notes)
    /// and returns the new objects in sew order. The objects that do not
    /// take part (lines, lettering) stay as they are.
    pub fn shape(&mut self, ids: &[ObjectId], op: ShapeOp, opts: ShapeOptions) -> Result<Vec<ObjectId>, ShapeError> {
        let mut parts: Vec<Part> = ids
            .iter()
            .filter_map(|id| self.object(*id))
            .filter_map(|o| {
                area_of(&o.kind).map(|area| Part {
                    id: o.id,
                    kind: o.kind.clone(),
                    area,
                })
            })
            .collect();
        parts.sort_by_key(|p| self.index_of(p.id));
        if parts.iter().any(|p| self.is_linked(p.id)) {
            return Err(ShapeError::AlreadyLinked);
        }
        if op == ShapeOp::Combine {
            return self.combine(&parts, opts);
        }
        if parts.len() < 2 {
            return Err(ShapeError::TooFew);
        }
        // Flatten and Divide can stay linked (see `link`).
        let link_op = match op {
            ShapeOp::Flatten => Some(crate::link::LinkOp::Flatten),
            ShapeOp::Divide => Some(crate::link::LinkOp::Divide),
            _ => None,
        };
        if let Some(lop) = link_op.filter(|_| opts.linked && !opts.keep_originals) {
            let ids: Vec<ObjectId> = parts.iter().map(|p| p.id).collect();
            return self.make_link(lop, &ids, &[], opts.overlap_mm);
        }
        let fill = self.current.fill.clone();
        // (owner, area) for each result, in sew order.
        let results: Vec<(usize, Vec<Region>)> = op_results(&parts, op, opts.overlap_mm);
        let results: Vec<(usize, Vec<Region>)> = results
            .into_iter()
            .map(|(o, r)| (o, clean(r)))
            .filter(|(_, r)| !r.is_empty())
            .collect();
        if results.is_empty() {
            return Err(ShapeError::NothingLeft);
        }
        let kinds: Vec<(usize, Vec<ObjectKind>)> = results
            .into_iter()
            .map(|(o, r)| (o, rebuild(&parts[o].kind, &r, &fill)))
            .collect();
        Ok(self.place(&parts, kinds, opts.keep_originals))
    }

    /// Remove overlaps: cuts the selected objects (with an area) out of the
    /// objects under them (the objects with an area that sew before them),
    /// so no area sews twice. `overlap_mm` keeps that much of each cut
    /// object under the cut. Returns the new pieces. An object that is all
    /// under the selection goes. With `linked`, the cut stays up to date
    /// when the objects move (see `link`).
    pub fn remove_overlaps(
        &mut self,
        ids: &[ObjectId],
        overlap_mm: f64,
        linked: bool,
    ) -> Result<Vec<ObjectId>, ShapeError> {
        let cutters: Vec<(usize, Vec<Region>)> = ids
            .iter()
            .filter_map(|id| self.object(*id))
            .filter_map(|o| Some((self.index_of(o.id)?, area_of(&o.kind)?)))
            .collect();
        let Some(last) = cutters.iter().map(|(i, _)| *i).max() else {
            return Err(ShapeError::NoOutline);
        };
        let mut targets = Vec::new();
        let mut results = Vec::new();
        for (i, o) in self.objects[..last].iter().enumerate() {
            if ids.contains(&o.id) || o.locked || self.is_linked(o.id) {
                continue;
            }
            let Some(area) = area_of(&o.kind) else { continue };
            let cutter = cutters
                .iter()
                .filter(|(ci, _)| *ci > i)
                .fold(Vec::new(), |acc, (_, a)| region::union(&acc, a));
            if region::intersection(&area, &cutter)
                .iter()
                .all(|r| r.area() < MIN_AREA_MM2)
            {
                continue;
            }
            let rest = clean(region::difference(&area, &shrink(&cutter, overlap_mm)));
            let n = targets.len();
            results.push((n, rebuild(&o.kind, &rest, &self.current.fill)));
            targets.push(Part {
                id: o.id,
                kind: o.kind.clone(),
                area,
            });
        }
        if targets.is_empty() {
            return Err(ShapeError::NothingToCut);
        }
        if linked {
            let cutter_ids: Vec<ObjectId> = ids
                .iter()
                .copied()
                .filter(|id| self.object(*id).is_some_and(|o| area_of(&o.kind).is_some()))
                .collect();
            let target_ids: Vec<ObjectId> = targets.iter().map(|p| p.id).collect();
            return self.make_link(
                crate::link::LinkOp::RemoveOverlaps,
                &target_ids,
                &cutter_ids,
                overlap_mm,
            );
        }
        // An object with nothing left has no result, so it goes.
        let results = results.into_iter().filter(|(_, k)| !k.is_empty()).collect();
        Ok(self.place(&targets, results, false))
    }

    fn combine(&mut self, parts: &[Part], opts: ShapeOptions) -> Result<Vec<ObjectId>, ShapeError> {
        if parts.len() < 2 {
            return Err(ShapeError::CannotCombine);
        }
        let kind = match &parts[0].kind {
            ObjectKind::Vector { style, .. } => {
                let mut paths = Vec::new();
                for p in parts {
                    let ObjectKind::Vector { path, .. } = &p.kind else {
                        return Err(ShapeError::CannotCombine);
                    };
                    paths.push(path.clone());
                }
                ObjectKind::Vector {
                    path: Path::joined(&paths),
                    style: *style,
                }
            }
            ObjectKind::Applique { params, .. } => {
                let mut pieces = Vec::new();
                for p in parts {
                    let ObjectKind::Applique { pieces: ps, .. } = &p.kind else {
                        return Err(ShapeError::CannotCombine);
                    };
                    pieces.extend(ps.iter().cloned());
                }
                ObjectKind::Applique {
                    pieces,
                    params: params.clone(),
                }
            }
            _ => return Err(ShapeError::CannotCombine),
        };
        Ok(self.place(parts, vec![(0, vec![kind])], opts.keep_originals))
    }

    /// Puts the results in the design: in the place of their owner (the
    /// other parts go), or after the last part with `keep`. Each new
    /// object takes the thread, group and functions of its owner.
    fn place(&mut self, parts: &[Part], results: Vec<(usize, Vec<ObjectKind>)>, keep: bool) -> Vec<ObjectId> {
        let owners: BTreeSet<usize> = results.iter().map(|(o, _)| *o).collect();
        let template: Vec<_> = parts.iter().map(|p| self.object(p.id).cloned()).collect();
        let mut at = if keep {
            parts
                .iter()
                .filter_map(|p| self.index_of(p.id))
                .max()
                .map_or(0, |i| i + 1)
        } else {
            for (i, p) in parts.iter().enumerate() {
                if !owners.contains(&i) {
                    self.remove_object(p.id);
                }
            }
            0
        };
        let mut new = Vec::new();
        let mut last_owner = None;
        for (o, kinds) in results {
            let Some(t) = template[o].as_ref() else { continue };
            if !keep && last_owner != Some(o) {
                // The first result of this owner goes where the owner is.
                at = self.index_of(t.id).unwrap_or(at);
                self.remove_object(t.id);
                last_owner = Some(o);
            }
            for kind in kinds {
                let id = self.insert_object(at, t.thread, kind);
                if let Some(obj) = self.object_mut(id) {
                    obj.group = t.group;
                    obj.functions = t.functions;
                    obj.name = t.name.clone();
                }
                at += 1;
                new.push(id);
            }
        }
        new
    }
}

/// The pieces of Divide: (owner, area) in sew order of the owners.
fn divide(parts: &[Part], overlap_mm: f64) -> Vec<(usize, Vec<Region>)> {
    let mut pieces: Vec<(usize, Vec<Region>)> = Vec::new();
    let mut covered: Vec<Region> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let cutter = shrink(&p.area, overlap_mm);
        let mut next = Vec::new();
        for (owner, area) in pieces {
            // The part under this object belongs to it (it is in front);
            // the rest stays with its owner, a little under the cut.
            let under = region::intersection(&area, &p.area);
            if !under.is_empty() {
                next.push((i, under));
            }
            let rest = region::difference(&area, &cutter);
            if !rest.is_empty() {
                next.push((owner, rest));
            }
        }
        let own = region::difference(&p.area, &covered);
        if !own.is_empty() {
            next.push((i, own));
        }
        covered = region::union(&covered, &p.area);
        pieces = next;
    }
    // Each connected piece is its own object.
    let mut out: Vec<(usize, Vec<Region>)> = pieces
        .into_iter()
        .flat_map(|(o, area)| area.into_iter().map(move |r| (o, vec![r])))
        .collect();
    out.sort_by_key(|(o, _)| *o);
    out
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;
    use crate::{FillParams, Rgb, VectorStyle};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square(x: f64, y: f64, s: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(x, y), p(x + s, y + s))),
            holes: Vec::new(),
            params: FillParams::default(),
        }
    }

    /// Two 20 mm squares that overlap in a 10 × 10 mm square, and a line.
    fn design() -> (Design, Vec<ObjectId>) {
        let mut d = Design::new("s");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        let a = d.add_object(red, square(0.0, 0.0, 20.0));
        let b = d.add_object(blue, square(10.0, 10.0, 20.0));
        let line = d.add_object(
            red,
            ObjectKind::Running {
                path: Path::new().move_to(p(0.0, 40.0)).line_to(p(10.0, 40.0)),
                params: Default::default(),
            },
        );
        (d, vec![a, b, line])
    }

    fn total_area(d: &Design, ids: &[ObjectId]) -> f64 {
        ids.iter()
            .filter_map(|id| d.object(*id))
            .filter_map(|o| area_of(&o.kind))
            .flatten()
            .map(|r| r.area())
            .sum()
    }

    #[test]
    fn weld_intersect_exclude() {
        let cases = [
            (ShapeOp::Weld, 700.0, 1),
            (ShapeOp::Intersect, 100.0, 1),
            // Two L shapes that touch at their corners: one fill with both.
            (ShapeOp::Exclude, 600.0, 1),
        ];
        for (op, area, n) in cases {
            let (mut d, ids) = design();
            let new = d.shape(&ids, op, ShapeOptions::default()).unwrap();
            assert_eq!(new.len(), n, "{op:?}");
            assert!(
                (total_area(&d, &new) - area).abs() < 0.01,
                "{op:?}: {}",
                total_area(&d, &new)
            );
            // The back object's place, thread and settings; the line stays.
            assert_eq!(d.objects().len(), n + 1, "{op:?}");
            assert_eq!(d.objects()[0].id, new[0]);
            assert_eq!(d.objects()[0].thread, d.threads[0].id);
            assert!(matches!(d.objects()[n].kind, ObjectKind::Running { .. }));
        }
    }

    #[test]
    fn minus_with_and_without_overlap() {
        let (mut d, ids) = design();
        let new = d.shape(&ids, ShapeOp::BackMinusFront, ShapeOptions::default()).unwrap();
        assert!((total_area(&d, &new) - 300.0).abs() < 0.01);
        assert_eq!(d.objects()[0].thread, d.threads[0].id, "the back object");

        let (mut d, ids) = design();
        let opts = ShapeOptions {
            overlap_mm: 1.0,
            ..ShapeOptions::default()
        };
        let new = d.shape(&ids, ShapeOp::FrontMinusBack, opts).unwrap();
        // 300 mm² plus a 1 mm band under the cut (two 9 mm edges and a corner).
        let a = total_area(&d, &new);
        assert!(a > 318.0 && a < 320.0, "{a}");
        assert_eq!(d.object(new[0]).unwrap().thread, d.threads[1].id, "the front object");
    }

    #[test]
    fn flatten_and_divide() {
        let (mut d, ids) = design();
        let new = d.shape(&ids, ShapeOp::Flatten, ShapeOptions::default()).unwrap();
        assert_eq!(new.len(), 2);
        assert!((total_area(&d, &new) - 700.0).abs() < 0.01, "nothing sews twice");

        let (mut d, ids) = design();
        let new = d.shape(&ids, ShapeOp::Divide, ShapeOptions::default()).unwrap();
        assert_eq!(new.len(), 3, "back part, overlap, front part");
        assert!((total_area(&d, &new) - 700.0).abs() < 0.01);
        // The overlap belongs to the front (blue) object.
        let blue = d.threads[1].id;
        let blue_area: f64 = new
            .iter()
            .filter(|id| d.object(**id).unwrap().thread == blue)
            .map(|id| total_area(&d, &[*id]))
            .sum();
        assert!((blue_area - 400.0).abs() < 0.01, "{blue_area}");
    }

    #[test]
    fn keep_originals_and_errors() {
        let (mut d, ids) = design();
        let opts = ShapeOptions {
            keep_originals: true,
            ..ShapeOptions::default()
        };
        let new = d.shape(&ids, ShapeOp::Weld, opts).unwrap();
        assert_eq!(d.objects().len(), 4);
        assert_eq!(d.index_of(new[0]), Some(2), "after the last object that takes part");

        let (mut d, ids) = design();
        assert_eq!(
            d.shape(&ids[..1], ShapeOp::Weld, ShapeOptions::default()),
            Err(ShapeError::TooFew)
        );
        let far = d.add_object(d.threads[0].id, square(100.0, 100.0, 5.0));
        assert_eq!(
            d.shape(&[ids[0], far], ShapeOp::Intersect, ShapeOptions::default()),
            Err(ShapeError::NothingLeft)
        );
        assert_eq!(
            d.shape(&ids[..2], ShapeOp::Combine, ShapeOptions::default()),
            Err(ShapeError::CannotCombine),
            "fills do not combine"
        );
    }

    #[test]
    fn vectors_weld_and_combine_as_vectors() {
        let mut d = Design::new("v");
        let t = d.add_thread("Black", Rgb(0, 0, 0));
        let vector = |x: f64| ObjectKind::Vector {
            path: Path::rect(BBox::new(p(x, 0.0), p(x + 20.0, 20.0))),
            style: VectorStyle::filled(Rgb(10, 20, 30)),
        };
        let a = d.add_object(t, vector(0.0));
        let b = d.add_object(t, vector(10.0));
        let new = d.shape(&[a, b], ShapeOp::Weld, ShapeOptions::default()).unwrap();
        let Some(ObjectKind::Vector { style, .. }) = d.object(new[0]).map(|o| &o.kind) else {
            panic!("not a vector")
        };
        assert_eq!(*style, VectorStyle::filled(Rgb(10, 20, 30)));
        assert!((total_area(&d, &new) - 600.0).abs() < 0.01);

        let c = d.add_object(t, vector(50.0));
        let new = d
            .shape(&[new[0], c], ShapeOp::Combine, ShapeOptions::default())
            .unwrap();
        assert_eq!(d.objects().len(), 1);
        let Some(ObjectKind::Vector { path, .. }) = d.object(new[0]).map(|o| &o.kind) else {
            panic!("not a vector")
        };
        assert_eq!(path.subpaths().len(), 2);
    }

    #[test]
    fn remove_overlaps_cuts_the_objects_under_the_selection() {
        let mut d = Design::new("r");
        let t = d.add_thread("Black", Rgb(0, 0, 0));
        let under = d.add_object(t, square(0.0, 0.0, 20.0));
        let hidden = d.add_object(t, square(12.0, 12.0, 4.0));
        let top = d.add_object(t, square(10.0, 10.0, 20.0));
        let after = d.add_object(t, square(15.0, 15.0, 20.0));
        let new = d.remove_overlaps(&[top], 0.0, false).unwrap();
        // The first square loses its 10 x 10 corner; the small square goes;
        // the square after the selection is not under it, so it stays.
        assert_eq!(new.len(), 1);
        assert!((total_area(&d, &new) - 300.0).abs() < 0.01);
        assert!(d.object(under).is_none() && d.object(hidden).is_none());
        assert!(d.object(top).is_some() && d.object(after).is_some());
        assert_eq!(d.index_of(new[0]), Some(0));
        assert_eq!(d.remove_overlaps(&[top], 0.0, false), Err(ShapeError::NothingToCut));
    }

    #[test]
    fn satin_takes_part_as_its_area() {
        let mut d = Design::new("s");
        let t = d.add_thread("Black", Rgb(0, 0, 0));
        let col = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: crate::SatinGeometry::from_centerline(
                    &Path::new().move_to(p(0.0, 5.0)).line_to(p(30.0, 5.0)),
                    4.0,
                ),
                params: Default::default(),
            },
        );
        let f = d.add_object(t, square(10.0, 0.0, 10.0));
        let new = d
            .shape(&[col, f], ShapeOp::BackMinusFront, ShapeOptions::default())
            .unwrap();
        assert_eq!(new.len(), 1, "the two pieces of the column, as one fill");
        let ObjectKind::Fill { outer, .. } = &d.object(new[0]).unwrap().kind else {
            panic!("not a fill")
        };
        assert_eq!(outer.subpaths().len(), 2);
        assert!(
            new.iter()
                .all(|id| matches!(d.object(*id).unwrap().kind, ObjectKind::Fill { .. }))
        );
        assert!((total_area(&d, &new) - 80.0).abs() < 0.01);
    }

    #[test]
    fn remove_overlaps_keeps_applique_as_applique() {
        let mut d = Design::new("a");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let under = d.add_object(
            t,
            ObjectKind::Applique {
                pieces: vec![AppliquePiece::new(
                    Path::rect(BBox::new(p(0.0, 0.0), p(30.0, 30.0))),
                    Vec::new(),
                )],
                params: Default::default(),
            },
        );
        let top = d.add_object(t, square(10.0, 10.0, 10.0));
        d.remove_overlaps(&[top], 0.0, false).unwrap();
        let o = d.objects().iter().find(|o| o.id != top).unwrap();
        let ObjectKind::Applique { pieces, .. } = &o.kind else {
            panic!("{:?}", o.kind)
        };
        // The square under the top one is a hole of the piece now.
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].holes.len(), 1);
        let _ = under;
    }
}
