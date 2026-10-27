//! Multi-hooping (P7 Milestone 2, ADR 0032): a design that is larger than
//! the hoop sews in several hoopings. The design keeps the layout
//! ([`HoopingLayout`]); the objects of each hooping are made from it when
//! they are needed ([`Design::hoopings`]), so an edit of the design changes
//! all hoopings.
//!
//! The layout is in view coordinates (after the design rotation), as the
//! guides are. How the objects go to the hoopings:
//!
//! 1. The split lines cut the plane into zones. Each zone goes to the
//!    hooping whose field (inside the margin) holds the most of the design
//!    in that zone.
//! 2. An object that fits in the field of the hooping that has most of it
//!    goes to that hooping whole, so it has no join.
//! 3. Else an object with an area (fill, appliqué, vector) is cut: each
//!    hooping gets the part in its zones made larger by half the join
//!    overlap, so the parts of two hoopings overlap by the join overlap and
//!    no fabric shows between them. A line (running stitch) is cut at the
//!    split lines with no overlap.
//! 4. Satin columns, lettering and stitch objects are not cut: they go
//!    whole to the hooping that has most of them, with a warning if they do
//!    not fit.
//!
//! Registration marks: where the zones of two hoopings touch, two small
//! crosses sew in a thread of their own, where both fields have space and
//! the design does not (if it can). The first hooping sews them at its end;
//! the later hooping sews them first, and the machine stops after them (a
//! color change), so the user can see that they sew on the old marks before
//! the design continues.

use emb_geom::region::{self, Join, Region};
use emb_geom::{BBox, Path, Point, Polyline, Transform};
use serde::{Deserialize, Serialize};

use crate::shaping::{area_of, clean, rebuild};
use crate::{Design, DesignObject, HoopPlacement, HoopPreset, HoopSelection, HoopShape, ObjectId, ObjectKind, Rgb};

/// How far the fields of two hoopings next to each other overlap at least
/// (the space for the registration marks), in mm.
pub const DEFAULT_OVERLAP_MM: f64 = 20.0;
/// How far the parts of a cut area go into the next hooping, in mm.
pub const DEFAULT_JOIN_MM: f64 = 1.0;
/// The space between the field edge and the stitches, in mm.
pub const DEFAULT_MARGIN_MM: f64 = 5.0;
/// Half the length of an arm of a registration mark, in mm.
pub const MARK_MM: f64 = 5.0;
/// The color of the registration mark thread.
pub const MARK_COLOR: Rgb = Rgb(230, 0, 120);
/// The hoop angle of a turned hooping. The Topaz 20 turns a design that is
/// too wide for its upright hoop the same way: on the machine the top of
/// the design is to the right.
pub const TURN_DEG: f64 = -90.0;
const FLATTEN_TOL_MM: f64 = 0.05;
/// Two hoopings get marks if their zones share this length of split line.
const MIN_SHARED_MM: f64 = 10.0;

/// One hooping: where the hoop goes on the design.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hooping {
    /// The hoop center, in view coordinates.
    pub center: Point,
    /// The hoop turns 90° on the fabric (a landscape hoop goes portrait).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turned: bool,
    /// For a multi-position hoop: the position (its field is the field of
    /// the hooping). The hoopings of one hoop move and turn together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

/// The hoopings of a large design (see the module notes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoopingLayout {
    /// The hoop preset id of all hoopings.
    pub hoop: String,
    /// The hoopings in sew order.
    pub hoopings: Vec<Hooping>,
    /// The split lines, in view coordinates. Each line goes on past its
    /// ends, across the whole design.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<Vec<Point>>,
    pub overlap_mm: f64,
    pub join_mm: f64,
    pub margin_mm: f64,
    /// Sew registration marks where two hoopings meet.
    pub marks: bool,
}

/// One hooping, made from the layout: a design of its own, with the hoop
/// center at the hooping center, ready for the stitch plan.
#[derive(Debug, Clone)]
pub struct HoopingOut {
    pub design: Design,
    /// Objects that do not fit, marks over the design.
    pub warnings: Vec<String>,
}

/// The half width and half height (in the hoop frame) of the part of the
/// field where stitches can go: a rectangle, inside an oval for oval hoops.
fn usable_half(hoop: &HoopPreset, margin: f64) -> (f64, f64) {
    let (w, h) = (
        (hoop.width_mm / 2.0 - margin).max(0.0),
        (hoop.height_mm / 2.0 - margin).max(0.0),
    );
    match hoop.shape {
        HoopShape::Rectangle | HoopShape::RoundedRectangle => (w, h),
        HoopShape::Oval | HoopShape::Circle => (w / 2f64.sqrt(), h / 2f64.sqrt()),
    }
}

fn rect_region(b: BBox) -> Region {
    Region::new(
        vec![b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)],
        Vec::new(),
    )
}

fn area(rs: &[Region]) -> f64 {
    rs.iter().map(Region::area).sum()
}

fn transform_regions(rs: &[Region], t: &Transform) -> Vec<Region> {
    rs.iter()
        .map(|r| {
            Region::new(
                r.outer.iter().map(|p| t.apply(*p)).collect(),
                r.holes
                    .iter()
                    .map(|h| h.iter().map(|p| t.apply(*p)).collect())
                    .collect(),
            )
        })
        .collect()
}

impl Hooping {
    /// The hoop angle of the hooping: `TURN_DEG` if it is turned, else 0.
    pub fn angle_deg(&self) -> f64 {
        if self.turned { TURN_DEG } else { 0.0 }
    }

    /// The hoop field of the hooping: the field of its position, or of the hoop.
    pub fn hoop_of(&self, hoop: &HoopPreset) -> HoopPreset {
        self.position
            .and_then(|i| hoop.position_preset(i))
            .unwrap_or_else(|| hoop.clone())
    }

    /// The usable field of the hooping, in view coordinates.
    pub fn field(&self, hoop: &HoopPreset, margin: f64) -> BBox {
        let (mut w, mut h) = usable_half(&self.hoop_of(hoop), margin);
        if self.turned {
            std::mem::swap(&mut w, &mut h);
        }
        BBox::new(self.center - Point::new(w, h), self.center + Point::new(w, h))
    }
}

/// The side of a split line: the part of the plane on its left, as a large
/// polygon. `size` is larger than the design.
fn side_of(line: &[Point], size: f64) -> Option<Region> {
    let (&a, &b) = (line.first()?, line.last()?);
    if line.len() < 2 {
        return None;
    }
    let dir_a = (line[1] - a).normalized()?;
    let dir_b = (b - line[line.len() - 2]).normalized()?;
    let mut pts = vec![a - dir_a * size];
    pts.extend_from_slice(line);
    pts.push(b + dir_b * size);
    let chord = (*pts.last()? - pts[0]).normalized()?;
    // The left of the chord (Y down: the normal (y, -x) points up for a
    // line to the right).
    let n = Point::new(chord.y, -chord.x);
    let (first, last) = (pts[0], *pts.last()?);
    pts.push(last + n * (2.0 * size));
    pts.push(first + n * (2.0 * size));
    Some(Region::new(pts, Vec::new()))
}

/// The zones that the split lines make of `bounds`.
fn zones(splits: &[Vec<Point>], bounds: BBox) -> Vec<Vec<Region>> {
    let size = 4.0 * (bounds.width() + bounds.height()).max(1.0);
    let mut out = vec![vec![rect_region(bounds)]];
    for s in splits {
        let Some(side) = side_of(s, size) else { continue };
        let side = [side];
        out = out
            .into_iter()
            .flat_map(|z| [region::intersection(&z, &side), region::difference(&z, &side)])
            .filter(|z| area(z) > 1e-6)
            .collect();
    }
    out
}

/// The hoopings of the positions of a multi-position hoop with its center
/// at `center` (view coordinates), turned (`TURN_DEG`) or not.
pub fn place_positions(hoop: &HoopPreset, center: Point, turned: bool) -> Vec<Hooping> {
    let turn = Transform::rotate_deg(if turned { TURN_DEG } else { 0.0 });
    hoop.positions
        .iter()
        .enumerate()
        .map(|(i, p)| Hooping {
            center: center + turn.apply(Point::new(p.x_mm, p.y_mm)),
            turned,
            position: Some(i),
        })
        .collect()
}

/// The center of the multi-position hoop of `h` (a hooping of a position).
pub fn hoop_center(hoop: &HoopPreset, h: &Hooping) -> Point {
    let Some(p) = h.position.and_then(|i| hoop.positions.get(i)) else {
        return h.center;
    };
    let turn = Transform::rotate_deg(h.angle_deg());
    h.center - turn.apply(Point::new(p.x_mm, p.y_mm))
}

/// Straight split lines between positions next to each other: half way
/// between their inner field edges, across the line between their centers.
fn position_splits(hoop: &HoopPreset, hoopings: &[Hooping], margin: f64) -> Vec<Vec<Point>> {
    let fields: Vec<BBox> = hoopings.iter().map(|h| h.field(hoop, margin)).collect();
    let mut lines: Vec<(bool, f64)> = Vec::new();
    for i in 0..fields.len() {
        for j in i + 1..fields.len() {
            let (a, b) = (fields[i].center(), fields[j].center());
            // Not if a third position is between them.
            let between = (0..fields.len()).filter(|k| *k != i && *k != j).any(|k| {
                let c = fields[k].center();
                let ab = b - a;
                let t = (c - a).dot(ab) / ab.dot(ab).max(1e-9);
                (0.05..0.95).contains(&t) && (a + ab * t).distance(c) < 0.5 * fields[k].width().min(fields[k].height())
            });
            if between {
                continue;
            }
            let (fi, fj) = (fields[i], fields[j]);
            let vertical = (b.x - a.x).abs() >= (b.y - a.y).abs();
            let at = if vertical {
                (fi.min.x.max(fj.min.x) + fi.max.x.min(fj.max.x)) / 2.0
            } else {
                (fi.min.y.max(fj.min.y) + fi.max.y.min(fj.max.y)) / 2.0
            };
            if !lines.iter().any(|(v, x)| *v == vertical && (x - at).abs() < 0.5) {
                lines.push((vertical, at));
            }
        }
    }
    let Some(all) = fields.iter().copied().reduce(BBox::union) else {
        return Vec::new();
    };
    lines
        .into_iter()
        .map(|(vertical, at)| {
            if vertical {
                vec![Point::new(at, all.min.y), Point::new(at, all.max.y)]
            } else {
                vec![Point::new(all.min.x, at), Point::new(all.max.x, at)]
            }
        })
        .collect()
}

impl HoopingLayout {
    /// Moves the hoopings of the positions of a multi-position hoop (they
    /// move together, as the hoop does) and the split lines by `d`.
    pub fn move_positions(&mut self, d: Point) {
        for h in self.hoopings.iter_mut().filter(|h| h.position.is_some()) {
            h.center += d;
        }
        for p in self.splits.iter_mut().flatten() {
            *p += d;
        }
    }

    /// Turns the multi-position hoop 90° (or back) about its center. The
    /// split lines are made again.
    pub fn turn_positions(&mut self, hoop: &HoopPreset) {
        let Some(first) = self.hoopings.iter().find(|h| h.position.is_some()).copied() else {
            return;
        };
        let kept: Vec<usize> = self.hoopings.iter().filter_map(|h| h.position).collect();
        self.hoopings = place_positions(hoop, hoop_center(hoop, &first), !first.turned)
            .into_iter()
            .filter(|h| h.position.is_some_and(|i| kept.contains(&i)))
            .collect();
        self.splits = position_splits(hoop, &self.hoopings, self.margin_mm);
    }

    /// The layout of a multi-position hoop round the design: the hoop
    /// center at the design center, turned if the design is the other way
    /// round from the hoop, a hooping for each position, split lines between
    /// positions next to each other, no registration marks (the fabric
    /// stays in the hoop).
    pub fn positions(design: &Design, hoop: &HoopPreset, margin_mm: f64) -> Option<HoopingLayout> {
        let ext = design.view_extents()?;
        let turned =
            (ext.height() > ext.width()) != (hoop.height_mm > hoop.width_mm) && hoop.height_mm != hoop.width_mm;
        let hoopings = place_positions(hoop, ext.center(), turned);
        let splits = position_splits(hoop, &hoopings, margin_mm);
        Some(HoopingLayout {
            hoop: hoop.id.clone(),
            hoopings,
            splits,
            overlap_mm: 0.0,
            join_mm: DEFAULT_JOIN_MM,
            margin_mm,
            marks: false,
        })
    }

    /// The fewest hoopings of `hoop` (straight or all turned) that cover the
    /// design, in rows and columns with straight split lines between them.
    /// The fields of two hoopings next to each other overlap by at least
    /// `overlap_mm`. A multi-position hoop takes its positions
    /// ([`HoopingLayout::positions`]). `None` for a design with no objects
    /// that sew.
    pub fn auto(design: &Design, hoop: &HoopPreset, overlap_mm: f64, margin_mm: f64) -> Option<HoopingLayout> {
        if !hoop.positions.is_empty() {
            return HoopingLayout::positions(design, hoop, margin_mm);
        }
        let ext = design.view_extents()?;
        let (hw, hh) = usable_half(hoop, margin_mm);
        // The number of hoopings along a side of `len` for a usable length `u`.
        let count = |len: f64, u: f64| -> Option<usize> {
            if len <= u {
                return Some(1);
            }
            let step = u - overlap_mm;
            (step > 0.0).then(|| (len / step).ceil() as usize)
        };
        let plan = |turned: bool| {
            let (uw, uh) = if turned {
                (2.0 * hh, 2.0 * hw)
            } else {
                (2.0 * hw, 2.0 * hh)
            };
            Some((count(ext.width(), uw)?, count(ext.height(), uh)?, turned))
        };
        let (cols, rows, turned) = [plan(false), plan(true)]
            .into_iter()
            .flatten()
            .min_by_key(|(c, r, t)| (c * r, *t))?;
        let (cw, ch) = (ext.width() / cols as f64, ext.height() / rows as f64);
        let mut hoopings = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                hoopings.push(Hooping {
                    center: Point::new(ext.min.x + (c as f64 + 0.5) * cw, ext.min.y + (r as f64 + 0.5) * ch),
                    turned,
                    position: None,
                });
            }
        }
        let mut splits = Vec::new();
        for c in 1..cols {
            let x = ext.min.x + c as f64 * cw;
            splits.push(vec![Point::new(x, ext.min.y), Point::new(x, ext.max.y)]);
        }
        for r in 1..rows {
            let y = ext.min.y + r as f64 * ch;
            splits.push(vec![Point::new(ext.min.x, y), Point::new(ext.max.x, y)]);
        }
        Some(HoopingLayout {
            hoop: hoop.id.clone(),
            hoopings,
            splits,
            overlap_mm,
            join_mm: DEFAULT_JOIN_MM,
            margin_mm,
            marks: true,
        })
    }
}

/// An object with its area (or its box) in design coordinates.
struct Item<'a> {
    obj: &'a DesignObject,
    area: Option<Vec<Region>>,
    measure: Vec<Region>,
}

impl Design {
    /// The box of the objects that sew, in view coordinates (the design
    /// rotation applied).
    pub fn view_extents(&self) -> Option<BBox> {
        let rot = self.rotation_transform();
        self.objects
            .iter()
            .filter(|o| o.kind.sews())
            .filter_map(|o| o.control_bbox())
            .map(|b| {
                let c = [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)];
                BBox::from_points(c.map(|p| rot.apply(p))).unwrap_or(b)
            })
            .reduce(BBox::union)
    }

    /// The hoopings of the layout of the design, in sew order (see the
    /// module notes). Empty if the design has no layout or `hoop` is not
    /// the hoop of the layout.
    pub fn hoopings(&self, hoop: &HoopPreset) -> Vec<HoopingOut> {
        let Some(layout) = self
            .hooping
            .as_ref()
            .filter(|l| l.hoop == hoop.id && !l.hoopings.is_empty())
        else {
            return Vec::new();
        };
        let Some(ext) = self.view_extents() else {
            return Vec::new();
        };
        let rot = self.rotation_transform();
        let Some(inv) = rot.inverse() else {
            return Vec::new();
        };
        let margin = layout.margin_mm;
        let fields: Vec<BBox> = layout.hoopings.iter().map(|h| h.field(hoop, margin)).collect();

        // 1. Zones, and the hooping of each zone.
        let bounds = ext.expand(50.0);
        let content = [rect_region(ext.expand(0.5))];
        let n = layout.hoopings.len();
        let mut zones_of: Vec<Vec<Region>> = vec![Vec::new(); n];
        for z in zones(&layout.splits, bounds) {
            let inside = region::intersection(&z, &content);
            if inside.is_empty() {
                continue;
            }
            let score = |k: usize| area(&region::intersection(&inside, &[rect_region(fields[k])]));
            let best = (0..n)
                .map(|k| (k, score(k)))
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|(k, s)| {
                    if s > 0.0 {
                        k
                    } else {
                        // Outside all fields: the nearest hooping.
                        let c = inside
                            .iter()
                            .filter_map(Region::bbox)
                            .reduce(BBox::union)
                            .map_or(ext.center(), |b| b.center());
                        (0..n)
                            .min_by(|a, b| {
                                layout.hoopings[*a]
                                    .center
                                    .distance(c)
                                    .total_cmp(&layout.hoopings[*b].center.distance(c))
                            })
                            .unwrap_or(0)
                    }
                })
                .unwrap_or(0);
            zones_of[best] = region::union(&zones_of[best], &z);
        }
        // The zones in design coordinates; and grown by half the join overlap.
        let zone_d: Vec<Vec<Region>> = zones_of.iter().map(|z| transform_regions(z, &inv)).collect();
        let grown_d: Vec<Vec<Region>> = zones_of
            .iter()
            .map(|z| transform_regions(&region::offset(z, layout.join_mm / 2.0, Join::Miter), &inv))
            .collect();

        // The design of each hooping, with no objects yet.
        let mut outs: Vec<HoopingOut> = layout
            .hoopings
            .iter()
            .enumerate()
            .map(|(k, h)| HoopingOut {
                design: self.hooping_shell(k, h, hoop, &inv),
                warnings: Vec::new(),
            })
            .collect();
        let field_hoops: Vec<HoopPreset> = layout.hoopings.iter().map(|h| h.hoop_of(hoop)).collect();
        let fits = |k: usize, d: &Design, b: Option<BBox>| {
            let out = d.output_transform();
            b.is_none_or(|b| {
                [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)]
                    .iter()
                    .all(|p| field_hoops[k].contains(out.apply(*p), margin))
            })
        };

        // 2.–4. The objects.
        let mut next_id = self.next_object_id().0;
        let items: Vec<Item> = self
            .objects
            .iter()
            .map(|o| {
                let area = match &o.kind {
                    ObjectKind::Fill { .. } | ObjectKind::Applique { .. } | ObjectKind::Vector { .. } => {
                        area_of(&o.kind)
                    }
                    _ => None,
                };
                let measure = area
                    .clone()
                    .or_else(|| o.control_bbox().map(|b| vec![rect_region(b.expand(0.1))]))
                    .unwrap_or_default();
                Item { obj: o, area, measure }
            })
            .collect();
        for (index, it) in items.iter().enumerate() {
            let o = it.obj;
            let label = format!("object #{} ({})", index + 1, o.type_name());
            let shares: Vec<f64> = zone_d
                .iter()
                .map(|z| area(&region::intersection(&it.measure, z)))
                .collect();
            let Some(main) = (0..n).max_by(|a, b| shares[*a].total_cmp(&shares[*b]).then(b.cmp(a))) else {
                continue;
            };
            let bbox = o.control_bbox();
            if fits(main, &outs[main].design, bbox) || !o.kind.sews() {
                outs[main].design.objects.push(self.objects[index].clone());
                continue;
            }
            let pieces: Vec<(usize, ObjectKind)> = match (&o.kind, &it.area) {
                (_, Some(a)) => (0..n)
                    .filter_map(|k| {
                        let part = clean(region::intersection(a, &grown_d[k]));
                        let kind = rebuild(&o.kind, &part, &self.current.fill).into_iter().next()?;
                        Some((k, kind))
                    })
                    .collect(),
                (ObjectKind::Running { path, params }, None) => {
                    let lines = path.flatten(FLATTEN_TOL_MM);
                    (0..n)
                        .flat_map(|k| {
                            region::clip_lines(&zone_d[k], &lines)
                                .into_iter()
                                .filter(|pl| pl.points.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() > 0.3)
                                .map(move |pl| {
                                    (
                                        k,
                                        ObjectKind::Running {
                                            path: polyline_path(&pl),
                                            params: params.clone(),
                                        },
                                    )
                                })
                        })
                        .collect()
                }
                _ => {
                    outs[main].design.objects.push(self.objects[index].clone());
                    outs[main].warnings.push(format!(
                        "Hooping {}: {label} does not fit the hoop and cannot be split.",
                        main + 1
                    ));
                    continue;
                }
            };
            let mut first = true;
            for (k, kind) in pieces {
                let mut piece = o.clone();
                if !first {
                    piece.id = ObjectId(next_id);
                    next_id += 1;
                }
                first = false;
                piece.kind = kind;
                // A new revision: a stitch cache keys on (id, revision).
                piece.revision = crate::object::fresh_revision();
                piece.start = None;
                piece.end = None;
                if !fits(k, &outs[k].design, piece.control_bbox()) {
                    outs[k].warnings.push(format!(
                        "Hooping {}: a part of {label} goes outside the hoop field.",
                        k + 1
                    ));
                }
                outs[k].design.objects.push(std::sync::Arc::new(piece));
            }
        }

        // Registration marks where two zones touch.
        if layout.marks {
            let design_area: Vec<Region> = items
                .iter()
                .filter(|it| it.obj.kind.sews())
                .fold(Vec::new(), |acc, it| {
                    region::union(&acc, &transform_regions(&it.measure, &rot))
                });
            let keep_off = region::offset(&design_area, 3.0, Join::Round);
            for i in 0..n {
                for j in i + 1..n {
                    let touch = region::intersection(&region::offset(&zones_of[i], 0.5, Join::Miter), &zones_of[j]);
                    // They share a part of a split line, not only a corner.
                    if area(&touch) < 0.5 * MIN_SHARED_MM {
                        continue;
                    }
                    let both = region::intersection(&[rect_region(fields[i])], &[rect_region(fields[j])]);
                    let (points, over) = mark_points(&both, &keep_off);
                    if points.is_empty() {
                        continue;
                    }
                    if over {
                        outs[j].warnings.push(format!(
                            "Hooping {}: the registration marks with hooping {} sew over the design (no free space in both fields).",
                            j + 1,
                            i + 1
                        ));
                    }
                    for p in points {
                        let at = inv.apply(p);
                        for (k, start) in [(i, false), (j, true)] {
                            let d = &mut outs[k].design;
                            let thread = mark_thread(d);
                            let mark = DesignObject {
                                id: ObjectId(next_id),
                                revision: crate::object::fresh_revision(),
                                name: Some(format!("Registration mark {}–{}", i + 1, j + 1)),
                                thread,
                                start: None,
                                end: None,
                                group: None,
                                locked: false,
                                functions: Default::default(),
                                kind: ObjectKind::Running {
                                    path: cross(at),
                                    params: Default::default(),
                                },
                            };
                            next_id += 1;
                            let arc = std::sync::Arc::new(mark);
                            if start {
                                let at = d.objects.iter().take_while(|o| o.thread == thread).count();
                                d.objects.insert(at, arc);
                            } else {
                                d.objects.push(arc);
                            }
                        }
                    }
                }
            }
        }
        outs
    }

    /// The design of hooping `k`, with no objects.
    fn hooping_shell(&self, k: usize, h: &Hooping, hoop: &HoopPreset, inv: &Transform) -> Design {
        let mut d = Design {
            objects: Vec::new(),
            ..self.clone()
        };
        d.name = format!("{} {}", self.name, k + 1).trim().to_string();
        d.hooping = None;
        d.product = None;
        d.repeat = None;
        d.start_end = Default::default();
        d.backdrop = None;
        d.guides.clear();
        d.monograms.clear();
        d.team_names = None;
        d.blends.clear();
        d.border_traces.clear();
        d.shape_links.clear();
        d.sketch_links.clear();
        d.hoop = HoopPlacement {
            selection: HoopSelection::Preset { id: hoop.id.clone() },
            center: inv.apply(h.center),
            angle_deg: h.angle_deg(),
            auto_center: false,
        };
        d
    }
}

/// The thread of the registration marks in `d` (added if it is not there).
fn mark_thread(d: &mut Design) -> crate::ThreadId {
    match d.threads.iter().find(|t| t.name == "Registration marks") {
        Some(t) => t.id,
        None => d.add_thread("Registration marks", MARK_COLOR),
    }
}

/// A cross of two runs at `c` (one line, so it sews with no trim).
fn cross(c: Point) -> Path {
    let a = MARK_MM;
    Path::new()
        .move_to(c + Point::new(-a, 0.0))
        .line_to(c + Point::new(a, 0.0))
        .line_to(c)
        .line_to(c + Point::new(0.0, -a))
        .line_to(c + Point::new(0.0, a))
}

fn polyline_path(pl: &Polyline) -> Path {
    let mut p = Path::new().move_to(pl.points[0]);
    for q in &pl.points[1..] {
        p = p.line_to(*q);
    }
    p
}

/// Up to two places for registration marks in `both` (the part that both
/// fields have), as far apart as they can be, where a mark does not touch
/// `keep_off` (the design). If there is no such place, two places in
/// `both` that are on the design, and `true`.
fn mark_points(both: &[Region], keep_off: &[Region]) -> (Vec<Point>, bool) {
    let Some(b) = both.iter().filter_map(Region::bbox).reduce(BBox::union) else {
        return (Vec::new(), false);
    };
    let s = MARK_MM + 1.0;
    if b.width() < 2.0 * s || b.height() < 2.0 * s {
        return (Vec::new(), false);
    }
    let free = region::difference(both, keep_off);
    let fits = |p: Point| {
        [
            (-s, -s),
            (s, -s),
            (s, s),
            (-s, s),
            (0.0, 0.0),
            (-s, 0.0),
            (s, 0.0),
            (0.0, -s),
            (0.0, s),
        ]
        .iter()
        .all(|(dx, dy)| free.iter().any(|r| r.contains(p + Point::new(*dx, *dy))))
    };
    let step = 2.5;
    let mut cands = Vec::new();
    let mut y = b.min.y + s;
    while y <= b.max.y - s + 1e-9 {
        let mut x = b.min.x + s;
        while x <= b.max.x - s + 1e-9 {
            let p = Point::new(x, y);
            if fits(p) {
                cands.push(p);
            }
            x += step;
        }
        y += step;
    }
    if cands.is_empty() {
        // Along the long side of the common part, a quarter from each end.
        let c = b.center();
        let (a, z) = if b.width() >= b.height() {
            let d = Point::new(b.width() / 4.0, 0.0);
            (c - d, c + d)
        } else {
            let d = Point::new(0.0, b.height() / 4.0);
            (c - d, c + d)
        };
        return (vec![a, z], true);
    }
    let mut best = (cands[0], cands[0], -1.0);
    for (i, p) in cands.iter().enumerate() {
        for q in &cands[i + 1..] {
            let d = p.distance(*q);
            if d > best.2 {
                best = (*p, *q, d);
            }
        }
    }
    if best.2 < 4.0 * MARK_MM {
        (vec![best.0], false)
    } else {
        (vec![best.0, best.1], false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FillParams, HoopCatalog, SatinGeometry, SatinParams, ThreadId};

    fn crown() -> HoopPreset {
        HoopCatalog::builtin().get("topaz20_crown").unwrap().clone()
    }

    fn fill(d: &mut Design, b: BBox) -> ObjectId {
        d.add_object(
            ThreadId(0),
            ObjectKind::Fill {
                outer: Path::rect(b),
                holes: Vec::new(),
                params: FillParams::default(),
            },
        )
    }

    fn big() -> Design {
        let mut d = Design::new("big");
        d.add_thread("Blue", Rgb(0, 0, 200));
        fill(&mut d, BBox::from_center_size(Point::ZERO, 360.0, 260.0));
        d
    }

    #[test]
    fn auto_layout_of_the_exit_test_design_is_two_by_two() {
        let d = big();
        let l = HoopingLayout::auto(&d, &crown(), DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM).unwrap();
        // The hoop is 200 mm across: turned, its long side is along the
        // long side of the design (2 × 2, not 3 × 2).
        assert_eq!(l.hoopings.len(), 4);
        assert!(l.hoopings.iter().all(|h| h.turned));
        assert_eq!(l.splits.len(), 2);
        // A design that fits needs one hooping and no split.
        let mut small = Design::new("s");
        fill(&mut small, BBox::from_center_size(Point::ZERO, 200.0, 150.0));
        let l = HoopingLayout::auto(&small, &crown(), DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM).unwrap();
        assert_eq!((l.hoopings.len(), l.splits.len()), (1, 0));
        // A tall design: the hoop stays upright.
        let mut tall = Design::new("t");
        fill(&mut tall, BBox::from_center_size(Point::ZERO, 180.0, 400.0));
        let l = HoopingLayout::auto(&tall, &crown(), DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM).unwrap();
        assert_eq!(l.hoopings.len(), 2);
        assert!(!l.hoopings[0].turned);
        // A wide design: the hoop turns.
        let mut wide = Design::new("w");
        fill(&mut wide, BBox::from_center_size(Point::ZERO, 400.0, 180.0));
        let l = HoopingLayout::auto(&wide, &crown(), DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM).unwrap();
        assert_eq!(l.hoopings.len(), 2);
        assert!(l.hoopings[0].turned);
    }

    #[test]
    fn a_large_fill_is_cut_with_the_join_overlap_and_each_part_fits() {
        let mut d = big();
        let hoop = crown();
        d.hooping = HoopingLayout::auto(&d, &hoop, DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM);
        // The fill covers all: marks would sew over it (tested below).
        d.hooping.as_mut().unwrap().marks = false;
        let outs = d.hoopings(&hoop);
        assert_eq!(outs.len(), 4);
        let mut total = 0.0;
        for (k, o) in outs.iter().enumerate() {
            assert!(o.warnings.is_empty(), "{k}: {:?}", o.warnings);
            let fills: Vec<&DesignObject> = o
                .design
                .objects()
                .iter()
                .filter(|x| matches!(x.kind, ObjectKind::Fill { .. }))
                .map(|x| x.as_ref())
                .collect();
            assert_eq!(fills.len(), 1);
            let a = area(&area_of(&fills[0].kind).unwrap());
            total += a;
            // Each part is a quarter and half the join overlap on its two inner sides.
            let expect = (180.0 + 0.5) * (130.0 + 0.5);
            assert!((a - expect).abs() < 1.0, "{k}: {a} {expect}");
        }
        // The parts overlap by the join overlap.
        assert!(total > 360.0 * 260.0);
        // Each hooping has its center at its part.
        let c = outs[0].design.hoop.center;
        assert!(c.distance(Point::new(-90.0, -65.0)) < 1e-6, "{c:?}");
    }

    #[test]
    fn objects_that_fit_stay_whole_and_lines_are_cut() {
        let mut d = big();
        // A small fill across the split line at x = 0, near the top.
        let small = fill(&mut d, BBox::from_center_size(Point::new(0.0, -100.0), 30.0, 20.0));
        // A long line across the design.
        d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(-170.0, 10.0))
                    .line_to(Point::new(170.0, 10.0)),
                params: Default::default(),
            },
        );
        // A satin column across the split: it cannot be cut.
        d.add_object(
            ThreadId(0),
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::new()
                        .move_to(Point::new(-170.0, 50.0))
                        .line_to(Point::new(170.0, 50.0)),
                    rail_b: Path::new()
                        .move_to(Point::new(-170.0, 54.0))
                        .line_to(Point::new(170.0, 54.0)),
                    rungs: vec![],
                },
                params: SatinParams::default(),
            },
        );
        let hoop = crown();
        d.hooping = HoopingLayout::auto(&d, &hoop, DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM);
        let outs = d.hoopings(&hoop);
        let count = |id: ObjectId| outs.iter().filter(|o| o.design.object(id).is_some()).count();
        assert_eq!(count(small), 1, "the small fill is whole in one hooping");
        let lines: usize = outs
            .iter()
            .map(|o| {
                o.design
                    .objects()
                    .iter()
                    .filter(|x| matches!(x.kind, ObjectKind::Running { .. }) && x.name.is_none())
                    .count()
            })
            .sum();
        assert_eq!(lines, 2, "the line is cut at x = 0");
        let warned: Vec<&String> = outs.iter().flat_map(|o| &o.warnings).collect();
        assert!(
            warned
                .iter()
                .any(|w| w.contains("Satin") && w.contains("cannot be split")),
            "{warned:?}"
        );
    }

    #[test]
    fn registration_marks_sew_at_the_end_and_again_at_the_start() {
        // A ring: the middle of the design is free for the marks.
        let mut d = Design::new("ring");
        d.add_thread("Blue", Rgb(0, 0, 200));
        let outer = Path::rect(BBox::from_center_size(Point::ZERO, 360.0, 180.0));
        let hole = Path::rect(BBox::from_center_size(Point::ZERO, 300.0, 120.0));
        d.add_object(
            ThreadId(0),
            ObjectKind::Fill {
                outer,
                holes: vec![hole],
                params: FillParams::default(),
            },
        );
        let hoop = crown();
        d.hooping = HoopingLayout::auto(&d, &hoop, DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM);
        let outs = d.hoopings(&hoop);
        assert_eq!(outs.len(), 2);
        let marks = |o: &HoopingOut| -> Vec<(usize, Point)> {
            o.design
                .objects()
                .iter()
                .enumerate()
                .filter(|(_, x)| x.name.as_deref().is_some_and(|n| n.starts_with("Registration")))
                .map(|(i, x)| (i, x.control_bbox().unwrap().center()))
                .collect()
        };
        let (a, b) = (marks(&outs[0]), marks(&outs[1]));
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 2);
        // The first hooping sews them last, the second first; same places.
        let n0 = outs[0].design.objects().len();
        assert!(a.iter().all(|(i, _)| *i >= n0 - 2));
        assert!(b.iter().all(|(i, _)| *i < 2));
        for ((_, p), (_, q)) in a.iter().zip(&b) {
            assert!(p.distance(*q) < 1e-9);
            // Not on the design: inside the hole.
            assert!(p.x.abs() < 150.0 - MARK_MM && p.y.abs() < 60.0 - MARK_MM, "{p:?}");
        }
        assert!(outs.iter().all(|o| o.warnings.is_empty()), "{:?}", outs[1].warnings);
        // Their own thread.
        let t = outs[1].design.objects()[0].thread;
        assert_eq!(outs[1].design.thread(t).unwrap().color, MARK_COLOR);
    }

    /// A test hoop: 300 × 150 mm in two positions of 160 × 150 mm.
    fn two_positions() -> HoopPreset {
        let p = |x: f64| crate::HoopPosition {
            x_mm: x,
            y_mm: 0.0,
            width_mm: 160.0,
            height_mm: 150.0,
        };
        HoopPreset {
            id: "two".into(),
            name: "Two positions".into(),
            brand: None,
            width_mm: 300.0,
            height_mm: 150.0,
            shape: HoopShape::Rectangle,
            corner_radius_mm: None,
            max_stitch_length_mm: None,
            positions: vec![p(-70.0), p(70.0)],
            cap: false,
        }
    }

    #[test]
    fn a_multi_position_hoop_makes_one_hooping_for_each_position() {
        let hoop = two_positions();
        let mut d = Design::new("long");
        d.add_thread("Blue", Rgb(0, 0, 200));
        fill(&mut d, BBox::from_center_size(Point::new(10.0, 5.0), 280.0, 120.0));
        let l = HoopingLayout::auto(&d, &hoop, DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM).unwrap();
        assert_eq!(l.hoopings.len(), 2);
        assert!(!l.marks);
        assert_eq!(l.hoopings[0].position, Some(0));
        assert!(l.hoopings[0].center.distance(Point::new(-60.0, 5.0)) < 1e-9);
        // One split line half way between the inner edges of the fields.
        assert_eq!(l.splits.len(), 1);
        assert!((l.splits[0][0].x - 10.0).abs() < 1e-9, "{:?}", l.splits);
        d.hooping = Some(l.clone());
        let outs = d.hoopings(&hoop);
        assert_eq!(outs.len(), 2);
        for o in &outs {
            assert!(o.warnings.is_empty(), "{:?}", o.warnings);
            // The checks use the 160 × 150 field of the position.
            let out = o.design.output_transform();
            let b = o.design.objects()[0].control_bbox().unwrap();
            assert!(out.apply(b.min).x >= -80.0 && out.apply(b.max).x <= 80.0);
        }
        // The hoop moves and turns as one.
        let mut l2 = l.clone();
        l2.move_positions(Point::new(5.0, 0.0));
        assert!(
            l2.hoopings[1]
                .center
                .distance(l.hoopings[1].center + Point::new(5.0, 0.0))
                < 1e-9
        );
        assert!((l2.splits[0][0].x - 15.0).abs() < 1e-9);
        l2.turn_positions(&hoop);
        assert!(l2.hoopings.iter().all(|h| h.turned));
        assert!(
            // Turned by TURN_DEG (−90°): the left position goes to the front.
            l2.hoopings[0].center.distance(Point::new(15.0, 5.0 + 70.0)) < 1e-9,
            "{:?}",
            l2.hoopings
        );
        assert_eq!(l2.splits.len(), 1);
        assert!((l2.splits[0][0].y - 5.0).abs() < 1e-9, "{:?}", l2.splits);
    }

    #[test]
    fn a_bent_split_line_makes_two_zones() {
        let line = vec![Point::new(0.0, -50.0), Point::new(10.0, 0.0), Point::new(0.0, 50.0)];
        let z = zones(&[line], BBox::from_center_size(Point::ZERO, 100.0, 100.0));
        assert_eq!(z.len(), 2);
        let total: f64 = z.iter().map(|r| area(r)).sum();
        assert!((total - 10000.0).abs() < 1e-3);
        // The right zone is smaller by the bend.
        let right = z
            .iter()
            .find(|r| r.iter().any(|g| g.contains(Point::new(40.0, 0.0))))
            .unwrap();
        assert!((area(right) - (5000.0 - 500.0)).abs() < 1e-3, "{}", area(right));
    }

    #[test]
    fn marks_over_the_design_give_a_warning() {
        // The fill covers both fields where they overlap, past the margin.
        let mut d = Design::new("full");
        d.add_thread("Blue", Rgb(0, 0, 200));
        fill(&mut d, BBox::from_center_size(Point::ZERO, 400.0, 190.0));
        let hoop = crown();
        d.hooping = HoopingLayout::auto(&d, &hoop, DEFAULT_OVERLAP_MM, DEFAULT_MARGIN_MM);
        let outs = d.hoopings(&hoop);
        assert_eq!(outs.len(), 2);
        assert!(
            outs[1].warnings.iter().any(|w| w.contains("sew over the design")),
            "{:?}",
            outs[1].warnings
        );
        // Unchanged design, no layout: no hoopings.
        let mut plain = big();
        plain.hooping = None;
        assert!(plain.hoopings(&hoop).is_empty());
    }
}
