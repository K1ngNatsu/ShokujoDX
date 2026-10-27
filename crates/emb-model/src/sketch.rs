//! Sketch objects (P14 Milestone 2, ADR 0069): an object kind with a
//! parametric sketch (`emb_sketch`) and no stitches.
//!
//! The sketch has its own coordinates (mm): its horizontal is horizontal
//! in the sketch. The placement puts the sketch in the design (a move, a
//! turn and a mirror); a scale scales the sketch itself and its
//! dimensions, so the dimensions stay real mm. The object keeps the path
//! of each curve in design coordinates, made again when the sketch or the
//! placement changes, so the box, the hit test and the drawing work as for
//! other objects.
//!
//! Milestone 4 (ADR 0075): the closed regions of a sketch become fills,
//! satins, appliqué or vectors; its curves become runs, satin lines, or a
//! satin column between two curves. The stitch settings are the current
//! properties of the design.
//!
//! Milestone 5 (ADR 0076): the objects keep a link to the sketch
//! (`sketch_link`) and are made again after a change of it.

use emb_geom::{Path, Point, Transform};
use emb_sketch::{Constraint, CurveId, RegionKey, Sketch, SketchRegion};
use serde::{Deserialize, Serialize};

use crate::vector::{ConvertTo, VectorStyle, from_vector};
use crate::{AppliquePiece, CurrentProperties, ObjectKind, Rgb, SatinGeometry};

/// What a region of a sketch becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionTo {
    Fill,
    /// A satin column where the shape lets it, else a fill with satin
    /// columns (as Convert to satin of a vector).
    Satin,
    Applique,
    Vector,
}

impl RegionTo {
    pub const ALL: [RegionTo; 4] = [RegionTo::Fill, RegionTo::Satin, RegionTo::Applique, RegionTo::Vector];

    /// The name (English; the app translates it).
    pub fn name(self) -> &'static str {
        match self {
            RegionTo::Fill => "Fill",
            RegionTo::Satin => "Satin",
            RegionTo::Applique => "Appliqué",
            RegionTo::Vector => "Vector",
        }
    }
}

/// What a curve of a sketch becomes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveTo {
    Run,
    /// A satin line along the curve, this wide (mm).
    SatinLine(f64),
}

/// The objects of one region: `color` is the color of a vector.
pub fn region_objects(
    r: &SketchRegion,
    to: RegionTo,
    current: &CurrentProperties,
    width_mm: f64,
    color: Rgb,
) -> Vec<ObjectKind> {
    match to {
        RegionTo::Fill => vec![ObjectKind::Fill {
            outer: r.outer.clone(),
            holes: r.holes.clone(),
            params: current.fill.clone(),
        }],
        RegionTo::Satin => {
            let all: Vec<&Path> = std::iter::once(&r.outer).chain(r.holes.iter()).collect();
            from_vector(&Path::joined(all), ConvertTo::Satin, current, width_mm)
        }
        RegionTo::Applique => vec![ObjectKind::Applique {
            pieces: vec![AppliquePiece::new(r.outer.clone(), r.holes.clone())],
            params: current.applique.clone(),
        }],
        RegionTo::Vector => {
            let all: Vec<&Path> = std::iter::once(&r.outer).chain(r.holes.iter()).collect();
            vec![ObjectKind::Vector {
                path: Path::joined(all),
                style: VectorStyle::filled(color),
            }]
        }
    }
}

/// The objects of curves (paths in design coordinates): one for each.
pub fn curve_objects(paths: &[Path], to: CurveTo, current: &CurrentProperties) -> Vec<ObjectKind> {
    paths
        .iter()
        .map(|path| match to {
            CurveTo::Run => ObjectKind::Running {
                path: path.clone(),
                params: current.running.clone(),
            },
            CurveTo::SatinLine(w) => ObjectKind::Satin {
                geometry: SatinGeometry::from_centerline(path, w),
                params: current.satin.clone(),
            },
        })
        .collect()
}

/// What the project file keeps of a sketch object.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SketchData {
    sketch: Sketch,
    #[serde(default = "identity", skip_serializing_if = "is_identity")]
    placement: Transform,
}

fn identity() -> Transform {
    Transform::IDENTITY
}

fn is_identity(t: &Transform) -> bool {
    *t == Transform::IDENTITY
}

/// A sketch, its placement, and the paths of its curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "SketchData", into = "SketchData")]
pub struct SketchObject {
    sketch: Sketch,
    placement: Transform,
    /// The path of each curve in design coordinates, in the order of
    /// `sketch.curves`.
    paths: Vec<(CurveId, Path)>,
}

impl From<SketchData> for SketchObject {
    fn from(d: SketchData) -> Self {
        let mut o = SketchObject::new(d.sketch);
        o.placement = d.placement;
        o.rebuild();
        o
    }
}

impl From<SketchObject> for SketchData {
    fn from(o: SketchObject) -> Self {
        SketchData {
            sketch: o.sketch,
            placement: o.placement,
        }
    }
}

impl SketchObject {
    pub fn new(sketch: Sketch) -> SketchObject {
        let mut o = SketchObject {
            sketch,
            placement: Transform::IDENTITY,
            paths: Vec::new(),
        };
        o.rebuild();
        o
    }

    pub fn sketch(&self) -> &Sketch {
        &self.sketch
    }

    /// The transform from sketch coordinates to design coordinates.
    pub fn placement(&self) -> Transform {
        self.placement
    }

    /// A design point in sketch coordinates.
    pub fn to_sketch(&self, p: Point) -> Point {
        self.placement.inverse().map_or(p, |t| t.apply(p))
    }

    /// A sketch point in design coordinates.
    pub fn to_design(&self, p: Point) -> Point {
        self.placement.apply(p)
    }

    /// Changes the sketch (the paths are made again).
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Sketch) -> R) -> R {
        let r = f(&mut self.sketch);
        // The label places of removed dimensions go.
        let ids: std::collections::BTreeSet<_> = self.sketch.constraints.iter().map(|c| c.id).collect();
        self.sketch.label_at.retain(|id, _| ids.contains(id));
        self.sketch.exprs.retain(|id, _| ids.contains(id));
        self.rebuild();
        r
    }

    fn rebuild(&mut self) {
        let t = self.placement;
        self.paths = self
            .sketch
            .curves
            .iter()
            .filter_map(|c| Some((c.id, self.sketch.curve_path(c.id)?.transformed(&t))))
            .collect();
    }

    /// The path of each curve, in design coordinates.
    pub fn curve_paths(&self) -> &[(CurveId, Path)] {
        &self.paths
    }

    /// The closed regions of the sketch in design coordinates, the largest
    /// first.
    pub fn regions(&self) -> Vec<SketchRegion> {
        let t = self.placement;
        self.sketch.regions().iter().map(|r| r.transformed(&t)).collect()
    }

    /// The region at design point `p`.
    pub fn region_at(&self, p: Point) -> Option<SketchRegion> {
        Some(self.sketch.region_at(self.to_sketch(p))?.transformed(&self.placement))
    }

    /// The key of region `r` (design coordinates); the point of the key is
    /// in sketch coordinates.
    pub fn region_key(&self, r: &SketchRegion) -> RegionKey {
        let k = r.key();
        RegionKey {
            sides: k.sides,
            at: self.to_sketch(k.at),
        }
    }

    /// The region of `key` now, in design coordinates (`Sketch::find_region`).
    pub fn find_region(&self, key: &RegionKey) -> Option<SketchRegion> {
        Some(self.sketch.find_region(key)?.transformed(&self.placement))
    }

    /// The path of curve `id` in design coordinates, forward or back.
    pub fn curve_path_dir(&self, id: CurveId, forward: bool) -> Option<Path> {
        Some(self.sketch.curve_path_dir(id, forward)?.transformed(&self.placement))
    }

    /// A satin column between curves `a` and `b` (its two rails, in the
    /// same direction), with the current satin settings.
    pub fn satin_between(&self, a: CurveId, b: CurveId, current: &CurrentProperties) -> Option<ObjectKind> {
        let rail_a = self.curve_path_dir(a, true)?;
        let ends = |p: &Path| Some((p.points().next()?, p.points().last()?));
        let (a0, a1) = ends(&rail_a)?;
        let fwd = self.curve_path_dir(b, true)?;
        let (b0, b1) = ends(&fwd)?;
        let rail_b = if a0.distance(b0) + a1.distance(b1) <= a0.distance(b1) + a1.distance(b0) {
            fwd
        } else {
            self.curve_path_dir(b, false)?
        };
        Some(ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a,
                rail_b,
                rungs: Vec::new(),
            },
            params: current.satin.clone(),
        })
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> + '_ {
        self.paths.iter().map(|(_, p)| p)
    }

    /// Applies a transform of the design. The move, turn and mirror go into
    /// the placement; the scale (the mean scale of `t`) scales the sketch,
    /// its radii, its fixed places and its dimensions (not its angles). A
    /// scale that is not the same in x and y stays in the placement.
    pub fn transform(&mut self, t: &Transform) {
        let det = t.a * t.d - t.b * t.c;
        let s = det.abs().sqrt();
        if !(s.is_finite() && s > 1e-9) {
            return;
        }
        // design' = t(P(x)) = P'(s·x)  ⇒  P' = t ∘ P ∘ scale(1/s).
        self.placement = Transform::scale(1.0 / s, 1.0 / s).then(self.placement).then(*t);
        if (s - 1.0).abs() > 1e-12 {
            let k = Transform::scale(s, s);
            for p in &mut self.sketch.points {
                p.pos = k.apply(p.pos);
            }
            for c in &mut self.sketch.curves {
                if let emb_sketch::Geometry::Circle { radius, .. } | emb_sketch::Geometry::Arc { radius, .. } =
                    &mut c.geometry
                {
                    *radius *= s;
                }
            }
            for at in self.sketch.label_at.values_mut() {
                *at = k.apply(*at);
            }
            // The lengths of the expressions scale too (ADR 0078).
            if !self.sketch.parameters.is_empty() || !self.sketch.exprs.is_empty() {
                self.sketch.expr_scale = self.sketch.expr_scale() * s;
            }
            for c in &mut self.sketch.constraints {
                c.constraint = match c.constraint {
                    Constraint::Fix { point, at } => Constraint::Fix { point, at: k.apply(at) },
                    Constraint::Angle { .. } => c.constraint,
                    other => match other.value() {
                        Some(v) => other.with_value(v * s),
                        None => other,
                    },
                };
            }
        }
        self.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_become_fills_satins_applique_and_vectors() {
        // A ring and a disc in it.
        let mut s = Sketch::new();
        s.add_circle(Point::new(0.0, 0.0), 40.0).unwrap();
        s.add_circle(Point::new(0.0, 0.0), 30.0).unwrap();
        let mut o = SketchObject::new(s);
        o.transform(&Transform::translate(100.0, 0.0));
        let regions = o.regions();
        assert_eq!(regions.len(), 2);
        let ring = regions.iter().find(|r| !r.holes.is_empty()).unwrap();
        assert!(o.region_at(Point::new(135.0, 0.0)).is_some_and(|r| r.holes.len() == 1));
        let current = CurrentProperties::default();
        let fill = region_objects(ring, RegionTo::Fill, &current, 4.0, Rgb(0, 0, 0));
        assert!(matches!(&fill[0], ObjectKind::Fill { holes, .. } if holes.len() == 1));
        let min_x = fill[0]
            .paths()
            .flat_map(|p| p.points())
            .map(|q| q.x)
            .fold(f64::MAX, f64::min);
        assert!((min_x - 60.0).abs() < 0.5, "{min_x}");
        // A ring sews as one satin column round it.
        let satin = region_objects(ring, RegionTo::Satin, &current, 4.0, Rgb(0, 0, 0));
        assert_eq!(satin.len(), 1);
        assert!(matches!(satin[0], ObjectKind::Satin { .. } | ObjectKind::Fill { .. }));
        let app = region_objects(ring, RegionTo::Applique, &current, 4.0, Rgb(0, 0, 0));
        assert!(matches!(&app[0], ObjectKind::Applique { pieces, .. } if pieces[0].holes.len() == 1));
        let v = region_objects(ring, RegionTo::Vector, &current, 4.0, Rgb(1, 2, 3));
        assert!(matches!(v[0], ObjectKind::Vector { .. }));
    }

    #[test]
    fn curves_become_runs_satin_lines_and_columns() {
        let mut s = Sketch::new();
        let a = s.add_line(Point::new(0.0, 0.0), Point::new(50.0, 0.0));
        // The second line goes the other way: the rails still pair.
        let b = s.add_line(Point::new(50.0, 8.0), Point::new(0.0, 8.0));
        let o = SketchObject::new(s);
        let current = CurrentProperties::default();
        let paths: Vec<Path> = o.curve_paths().iter().map(|(_, p)| p.clone()).collect();
        assert_eq!(curve_objects(&paths, CurveTo::Run, &current).len(), 2);
        assert!(matches!(
            curve_objects(&paths[..1], CurveTo::SatinLine(3.0), &current)[0],
            ObjectKind::Satin { .. }
        ));
        let Some(ObjectKind::Satin { geometry, .. }) = o.satin_between(a, b, &current) else {
            panic!("a satin column")
        };
        let start = |p: &Path| p.points().next().unwrap();
        assert!(start(&geometry.rail_a).distance(start(&geometry.rail_b)) < 8.0 + 1e-9);
    }

    fn object() -> SketchObject {
        let mut s = Sketch::new();
        let l = s.add_line(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        s.add_constraint(Constraint::Length { line: l, mm: 10.0 }).unwrap();
        s.add_constraint(Constraint::Horizontal { line: l }).unwrap();
        s.add_circle(Point::new(5.0, 5.0), 2.0).unwrap();
        SketchObject::new(s)
    }

    #[test]
    fn the_paths_follow_the_sketch() {
        let mut o = object();
        assert_eq!(o.curve_paths().len(), 2);
        o.edit(|s| {
            s.add_line(Point::new(0.0, 0.0), Point::new(0.0, 10.0));
        });
        assert_eq!(o.paths().count(), 3);
    }

    #[test]
    fn a_turn_goes_into_the_placement_and_a_scale_into_the_sketch() {
        let mut o = object();
        o.transform(&Transform::rotate_deg(90.0));
        // The line stays horizontal in the sketch; in the design it is upright.
        let mut s = o.sketch().clone();
        assert!(s.solve().converged);
        let (_, path) = &o.curve_paths()[0];
        let pts: Vec<Point> = path.points().collect();
        assert!((pts[0].x - pts[1].x).abs() < 1e-9, "upright in the design");
        assert!((o.to_sketch(o.to_design(Point::new(3.0, 4.0))) - Point::new(3.0, 4.0)).length() < 1e-9);
        // A scale of 2: the length becomes 20 mm, the radius 4 mm.
        o.transform(&Transform::scale(2.0, 2.0));
        let s = o.sketch();
        assert!(s.constraints.iter().any(|c| c.constraint.value() == Some(20.0)));
        let r = s.curves.iter().find_map(|c| s.radius(c.id)).unwrap();
        assert!((r - 4.0).abs() < 1e-9);
        let (_, path) = &o.curve_paths()[0];
        let pts: Vec<Point> = path.points().collect();
        assert!((pts[0].distance(pts[1]) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn json_keeps_the_sketch_and_the_placement() {
        let mut o = object();
        o.transform(&Transform::translate(5.0, 7.0));
        let text = serde_json::to_string(&o).unwrap();
        assert!(!text.contains("paths"));
        let back: SketchObject = serde_json::from_str(&text).unwrap();
        assert_eq!(back, o);
        assert_eq!(back.curve_paths().len(), 2);
        // An object with no move has no placement in the file.
        assert!(!serde_json::to_string(&object()).unwrap().contains("placement"));
        // Label places and expressions (maps by dimension) load from a
        // project, which goes through a JSON value.
        o.edit(|s| {
            let id = s.constraints[0].id;
            s.label_at.insert(id, Point::new(1.0, 2.0));
            s.set_parameter("w", "12 mm").unwrap();
            s.set_dimension_expr(id, "w").unwrap();
        });
        let mut d = crate::Design::new("s");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        d.add_object(t, ObjectKind::Sketch(Box::new(o)));
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert!(back.same_content(&d));
    }
}
