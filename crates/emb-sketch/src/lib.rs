//! Parametric sketches (P14, ADR 0068): points, lines, arcs and circles
//! with geometric constraints and dimensions, and the solver that keeps
//! them true.
//!
//! - [`Sketch`]: the points, curves and constraints. Sizes are mm, angles
//!   degrees.
//! - [`Sketch::solve`] and [`Sketch::drag`]: move the variables so that the
//!   constraints are true ([`solve`]).
//! - [`Sketch::analyze`]: the free motions, and the constraints that repeat
//!   or conflict.
//! - [`infer`]: the constraints that the program adds while the user draws;
//!   [`edit`]: trim, extend, split, fillet, chamfer, offset, mirror,
//!   patterns and relax (P14 Milestone 3).
//! - [`params`] and [`expr`]: named parameters and dimensions with
//!   expressions (P14 Milestone 6).
//!
//! The methods are textbook numerical methods in our own code (no port of
//! another solver). This crate has no UI dependency.
#![forbid(unsafe_code)]

pub mod edit;
mod equations;
pub mod expr;
pub mod import;
pub mod infer;
mod linalg;
pub mod params;
pub mod regions;
pub mod shape;
pub mod solve;

use emb_geom::{Path, Point, math};
use serde::{Deserialize, Serialize};

pub use edit::EditError;
pub use expr::{ExprError, Units};
pub use params::Parameter;
pub use regions::{RegionKey, SketchRegion};
pub use shape::Shape;
pub use solve::{Analysis, SolveReport};

/// The id of a point of a sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PointId(pub u32);

/// The id of a curve of a sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CurveId(pub u32);

/// The id of a constraint of a sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstraintId(pub u32);

/// A point of a sketch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SketchPoint {
    pub id: PointId,
    pub pos: Point,
    /// A center point (a format: it shows as a cross, for example the
    /// place of a hole or an eyelet).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub center: bool,
}

/// The geometry of a curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Geometry {
    Line {
        a: PointId,
        b: PointId,
    },
    Circle {
        center: PointId,
        radius: f64,
    },
    /// Counterclockwise from `start` to `end` round `center`.
    Arc {
        center: PointId,
        start: PointId,
        end: PointId,
        radius: f64,
    },
    /// A spline piece: a cubic Bézier curve from `start` to `end` with the
    /// handles `c1` and `c2` (P14 Milestone 6, ADR 0079).
    Bezier {
        start: PointId,
        c1: PointId,
        c2: PointId,
        end: PointId,
    },
}

/// A curve of a sketch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub id: CurveId,
    pub geometry: Geometry,
    /// A construction curve helps to draw and does not become embroidery.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub construction: bool,
    /// A centerline (a construction line): a dimension from a point to it
    /// is a diameter.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub centerline: bool,
}

impl Curve {
    pub fn is_line(&self) -> bool {
        matches!(self.geometry, Geometry::Line { .. })
    }

    /// True for a circle or an arc.
    pub fn is_round(&self) -> bool {
        matches!(self.geometry, Geometry::Circle { .. } | Geometry::Arc { .. })
    }

    /// True for a spline piece.
    pub fn is_spline(&self) -> bool {
        matches!(self.geometry, Geometry::Bezier { .. })
    }

    /// The points of the curve (the center of a circle or an arc first; the
    /// ends of a spline piece, then its handles).
    pub fn points(&self) -> Vec<PointId> {
        match self.geometry {
            Geometry::Line { a, b } => vec![a, b],
            Geometry::Circle { center, .. } => vec![center],
            Geometry::Arc { center, start, end, .. } => vec![center, start, end],
            Geometry::Bezier { start, c1, c2, end } => vec![start, end, c1, c2],
        }
    }

    /// The end points of the curve (none for a circle).
    pub fn ends(&self) -> Option<(PointId, PointId)> {
        match self.geometry {
            Geometry::Line { a, b } => Some((a, b)),
            Geometry::Arc { start, end, .. } | Geometry::Bezier { start, end, .. } => Some((start, end)),
            Geometry::Circle { .. } => None,
        }
    }
}

/// A constraint or a dimension.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// Two points at the same place.
    Coincident {
        a: PointId,
        b: PointId,
    },
    Horizontal {
        line: CurveId,
    },
    Vertical {
        line: CurveId,
    },
    Parallel {
        a: CurveId,
        b: CurveId,
    },
    Perpendicular {
        a: CurveId,
        b: CurveId,
    },
    /// Two lines on one straight line.
    Collinear {
        a: CurveId,
        b: CurveId,
    },
    /// Two lines of the same length, or two circles or arcs of the same
    /// radius.
    Equal {
        a: CurveId,
        b: CurveId,
    },
    /// A line and a circle or an arc, or two circles or arcs, that touch.
    /// `inside`: one circle is inside the other.
    Tangent {
        a: CurveId,
        b: CurveId,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        inside: bool,
    },
    /// Two circles or arcs with the same center.
    Concentric {
        a: CurveId,
        b: CurveId,
    },
    /// A point at the middle of a line.
    Midpoint {
        point: PointId,
        line: CurveId,
    },
    /// A point on a line (or its extension), a circle or an arc.
    OnCurve {
        point: PointId,
        curve: CurveId,
    },
    /// Two points symmetric about a line.
    Symmetric {
        a: PointId,
        b: PointId,
        line: CurveId,
    },
    /// A point that does not move.
    Fix {
        point: PointId,
        at: Point,
    },
    /// Dimension: the distance between two points.
    Distance {
        a: PointId,
        b: PointId,
        mm: f64,
    },
    /// Dimension: `b.x − a.x`.
    HorizontalDistance {
        a: PointId,
        b: PointId,
        mm: f64,
    },
    /// Dimension: `b.y − a.y`.
    VerticalDistance {
        a: PointId,
        b: PointId,
        mm: f64,
    },
    /// Dimension: the distance from a point to a line (its extension too),
    /// positive on the left of the line from its first point to its second.
    LineDistance {
        point: PointId,
        line: CurveId,
        mm: f64,
    },
    /// Dimension: the length of a line.
    Length {
        line: CurveId,
        mm: f64,
    },
    /// Dimension: the angle from line `a` to line `b`, counterclockwise.
    Angle {
        a: CurveId,
        b: CurveId,
        deg: f64,
    },
    Radius {
        curve: CurveId,
        mm: f64,
    },
    Diameter {
        curve: CurveId,
        mm: f64,
    },
    /// Dimension: the length along an arc.
    ArcLength {
        curve: CurveId,
        mm: f64,
    },
}

impl Constraint {
    /// True for a dimension (a constraint with a value).
    pub fn is_dimension(&self) -> bool {
        self.value().is_some()
    }

    /// The value of a dimension.
    pub fn value(&self) -> Option<f64> {
        match *self {
            Constraint::Distance { mm, .. }
            | Constraint::HorizontalDistance { mm, .. }
            | Constraint::VerticalDistance { mm, .. }
            | Constraint::LineDistance { mm, .. }
            | Constraint::Length { mm, .. }
            | Constraint::Radius { mm, .. }
            | Constraint::Diameter { mm, .. }
            | Constraint::ArcLength { mm, .. } => Some(mm),
            Constraint::Angle { deg, .. } => Some(deg),
            _ => None,
        }
    }

    /// The constraint with a new value (a dimension), else itself.
    pub fn with_value(mut self, v: f64) -> Constraint {
        match &mut self {
            Constraint::Distance { mm, .. }
            | Constraint::HorizontalDistance { mm, .. }
            | Constraint::VerticalDistance { mm, .. }
            | Constraint::LineDistance { mm, .. }
            | Constraint::Length { mm, .. }
            | Constraint::Radius { mm, .. }
            | Constraint::Diameter { mm, .. }
            | Constraint::ArcLength { mm, .. } => *mm = v,
            Constraint::Angle { deg, .. } => *deg = v,
            _ => {}
        }
        self
    }

    /// The names of all kinds (for the translations).
    pub const NAMES: [&'static str; 22] = [
        "Coincident",
        "Horizontal",
        "Vertical",
        "Parallel",
        "Perpendicular",
        "Collinear",
        "Equal",
        "Tangent",
        "Concentric",
        "Midpoint",
        "On curve",
        "Symmetric",
        "Fix",
        "Distance",
        "Horizontal distance",
        "Vertical distance",
        "Distance to line",
        "Length",
        "Angle",
        "Radius",
        "Diameter",
        "Arc length",
    ];

    /// The name of the kind (English; the app translates it).
    pub fn name(&self) -> &'static str {
        match self {
            Constraint::Coincident { .. } => "Coincident",
            Constraint::Horizontal { .. } => "Horizontal",
            Constraint::Vertical { .. } => "Vertical",
            Constraint::Parallel { .. } => "Parallel",
            Constraint::Perpendicular { .. } => "Perpendicular",
            Constraint::Collinear { .. } => "Collinear",
            Constraint::Equal { .. } => "Equal",
            Constraint::Tangent { .. } => "Tangent",
            Constraint::Concentric { .. } => "Concentric",
            Constraint::Midpoint { .. } => "Midpoint",
            Constraint::OnCurve { .. } => "On curve",
            Constraint::Symmetric { .. } => "Symmetric",
            Constraint::Fix { .. } => "Fix",
            Constraint::Distance { .. } => "Distance",
            Constraint::HorizontalDistance { .. } => "Horizontal distance",
            Constraint::VerticalDistance { .. } => "Vertical distance",
            Constraint::LineDistance { .. } => "Distance to line",
            Constraint::Length { .. } => "Length",
            Constraint::Angle { .. } => "Angle",
            Constraint::Radius { .. } => "Radius",
            Constraint::Diameter { .. } => "Diameter",
            Constraint::ArcLength { .. } => "Arc length",
        }
    }

    /// The points of the constraint.
    pub fn points(&self) -> Vec<PointId> {
        match *self {
            Constraint::Coincident { a, b }
            | Constraint::Distance { a, b, .. }
            | Constraint::HorizontalDistance { a, b, .. }
            | Constraint::VerticalDistance { a, b, .. } => vec![a, b],
            Constraint::Midpoint { point, .. }
            | Constraint::OnCurve { point, .. }
            | Constraint::Fix { point, .. }
            | Constraint::LineDistance { point, .. } => {
                vec![point]
            }
            Constraint::Symmetric { a, b, .. } => vec![a, b],
            _ => Vec::new(),
        }
    }

    /// The curves of the constraint.
    pub fn curves(&self) -> Vec<CurveId> {
        match *self {
            Constraint::Horizontal { line }
            | Constraint::Vertical { line }
            | Constraint::Length { line, .. }
            | Constraint::Midpoint { line, .. }
            | Constraint::Symmetric { line, .. }
            | Constraint::LineDistance { line, .. } => vec![line],
            Constraint::OnCurve { curve, .. }
            | Constraint::Radius { curve, .. }
            | Constraint::Diameter { curve, .. }
            | Constraint::ArcLength { curve, .. } => {
                vec![curve]
            }
            Constraint::Parallel { a, b }
            | Constraint::Perpendicular { a, b }
            | Constraint::Collinear { a, b }
            | Constraint::Equal { a, b }
            | Constraint::Tangent { a, b, .. }
            | Constraint::Concentric { a, b }
            | Constraint::Angle { a, b, .. } => vec![a, b],
            _ => Vec::new(),
        }
    }

    /// The constraint with its points and curves changed by `p` and `c`
    /// (for copies of curves).
    pub fn map_ids(self, p: impl Fn(PointId) -> PointId, c: impl Fn(CurveId) -> CurveId) -> Constraint {
        use Constraint as K;
        match self {
            K::Coincident { a, b } => K::Coincident { a: p(a), b: p(b) },
            K::Horizontal { line } => K::Horizontal { line: c(line) },
            K::Vertical { line } => K::Vertical { line: c(line) },
            K::Parallel { a, b } => K::Parallel { a: c(a), b: c(b) },
            K::Perpendicular { a, b } => K::Perpendicular { a: c(a), b: c(b) },
            K::Collinear { a, b } => K::Collinear { a: c(a), b: c(b) },
            K::Equal { a, b } => K::Equal { a: c(a), b: c(b) },
            K::Tangent { a, b, inside } => K::Tangent {
                a: c(a),
                b: c(b),
                inside,
            },
            K::Concentric { a, b } => K::Concentric { a: c(a), b: c(b) },
            K::Midpoint { point, line } => K::Midpoint {
                point: p(point),
                line: c(line),
            },
            K::OnCurve { point, curve } => K::OnCurve {
                point: p(point),
                curve: c(curve),
            },
            K::Symmetric { a, b, line } => K::Symmetric {
                a: p(a),
                b: p(b),
                line: c(line),
            },
            K::Fix { point, at } => K::Fix { point: p(point), at },
            K::Distance { a, b, mm } => K::Distance { a: p(a), b: p(b), mm },
            K::HorizontalDistance { a, b, mm } => K::HorizontalDistance { a: p(a), b: p(b), mm },
            K::VerticalDistance { a, b, mm } => K::VerticalDistance { a: p(a), b: p(b), mm },
            K::LineDistance { point, line, mm } => K::LineDistance {
                point: p(point),
                line: c(line),
                mm,
            },
            K::Length { line, mm } => K::Length { line: c(line), mm },
            K::Angle { a, b, deg } => K::Angle { a: c(a), b: c(b), deg },
            K::Radius { curve, mm } => K::Radius { curve: c(curve), mm },
            K::Diameter { curve, mm } => K::Diameter { curve: c(curve), mm },
            K::ArcLength { curve, mm } => K::ArcLength { curve: c(curve), mm },
        }
    }
}

/// A constraint of a sketch with its id.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SketchConstraint {
    pub id: ConstraintId,
    pub constraint: Constraint,
    /// A driven dimension only shows the size: it is not an equation, and
    /// its value follows the sketch.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub driven: bool,
}

/// Why a sketch refuses a change.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SketchError {
    #[error("the sketch has no such point or curve")]
    Missing,
    #[error("{0}")]
    WrongCurves(&'static str),
    #[error("a dimension must be a number in its range")]
    BadValue,
    #[error("a radius must be more than 0")]
    BadRadius,
}

/// The largest size in a sketch, in mm (no embroidery is larger).
pub const MAX_MM: f64 = 10_000.0;

/// A sketch.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sketch {
    pub points: Vec<SketchPoint>,
    pub curves: Vec<Curve>,
    pub constraints: Vec<SketchConstraint>,
    /// The next id (points, curves and constraints share it).
    pub next_id: u32,
    /// Where the user put the label of a dimension, in sketch coordinates
    /// (else the program puts it).
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        with = "id_map"
    )]
    pub label_at: std::collections::BTreeMap<ConstraintId, Point>,
    /// The named parameters (see `params`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// The expressions of dimensions (see `params`).
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        with = "id_map"
    )]
    pub exprs: std::collections::BTreeMap<ConstraintId, String>,
    /// The units of plain numbers in expressions where a length is
    /// necessary.
    #[serde(default, skip_serializing_if = "Units::is_mm")]
    pub units: Units,
    /// The factor of the lengths of expressions after a scale of the
    /// sketch (0 is 1; see `Sketch::expr_scale`).
    #[serde(default, skip_serializing_if = "unscaled")]
    pub expr_scale: f64,
}

fn unscaled(k: &f64) -> bool {
    *k == 0.0 || *k == 1.0
}

/// Maps with constraint ids as keys: the keys are texts in the file
/// ("23"). A project is read through a JSON value, which does not change
/// a text key back into an id by itself.
mod id_map {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::ConstraintId;

    pub fn serialize<S: Serializer, V: Serialize>(m: &BTreeMap<ConstraintId, V>, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(m.iter().map(|(k, v)| (k.0.to_string(), v)))
    }

    pub fn deserialize<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
        d: D,
    ) -> Result<BTreeMap<ConstraintId, V>, D::Error> {
        let m: BTreeMap<String, V> = BTreeMap::deserialize(d)?;
        m.into_iter()
            .map(|(k, v)| {
                k.parse::<u32>()
                    .map(|k| (ConstraintId(k), v))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

impl Sketch {
    pub fn new() -> Sketch {
        Sketch::default()
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id.max(
            self.points
                .iter()
                .map(|p| p.id.0 + 1)
                .chain(self.curves.iter().map(|c| c.id.0 + 1))
                .chain(self.constraints.iter().map(|c| c.id.0 + 1))
                .max()
                .unwrap_or(0),
        );
        self.next_id = id + 1;
        id
    }

    pub fn point(&self, id: PointId) -> Option<Point> {
        self.points.iter().find(|p| p.id == id).map(|p| p.pos)
    }

    pub fn set_point(&mut self, id: PointId, pos: Point) {
        if let Some(p) = self.points.iter_mut().find(|p| p.id == id) {
            p.pos = pos;
        }
    }

    pub fn curve(&self, id: CurveId) -> Option<&Curve> {
        self.curves.iter().find(|c| c.id == id)
    }

    pub fn constraint(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.iter().find(|c| c.id == id).map(|c| &c.constraint)
    }

    /// Adds a point.
    pub fn add_point(&mut self, pos: Point) -> PointId {
        let id = PointId(self.take_id());
        self.points.push(SketchPoint { id, pos, center: false });
        id
    }

    fn add_curve(&mut self, geometry: Geometry) -> CurveId {
        let id = CurveId(self.take_id());
        self.curves.push(Curve {
            id,
            geometry,
            construction: false,
            centerline: false,
        });
        id
    }

    /// Adds a line from `a` to `b` with two new points.
    pub fn add_line(&mut self, a: Point, b: Point) -> CurveId {
        let (pa, pb) = (self.add_point(a), self.add_point(b));
        self.add_curve(Geometry::Line { a: pa, b: pb })
    }

    /// Adds a line between two points of the sketch.
    pub fn add_line_between(&mut self, a: PointId, b: PointId) -> Result<CurveId, SketchError> {
        if self.point(a).is_none() || self.point(b).is_none() {
            return Err(SketchError::Missing);
        }
        Ok(self.add_curve(Geometry::Line { a, b }))
    }

    /// Adds a circle with a new center point.
    pub fn add_circle(&mut self, center: Point, radius: f64) -> Result<CurveId, SketchError> {
        if !(radius.is_finite() && radius > 0.0 && radius <= MAX_MM) {
            return Err(SketchError::BadRadius);
        }
        let c = self.add_point(center);
        Ok(self.add_curve(Geometry::Circle { center: c, radius }))
    }

    /// Adds an arc counterclockwise from `start` to `end` round `center`,
    /// with three new points. The radius is the distance to `start`; `end`
    /// goes on the circle.
    pub fn add_arc(&mut self, center: Point, start: Point, end: Point) -> Result<CurveId, SketchError> {
        let r = center.distance(start);
        if !(r.is_finite() && r > 1e-9 && r <= MAX_MM) {
            return Err(SketchError::BadRadius);
        }
        let dir = (end - center).normalized().ok_or(SketchError::BadRadius)?;
        let (c, s, e) = (
            self.add_point(center),
            self.add_point(start),
            self.add_point(center + dir * r),
        );
        Ok(self.add_curve(Geometry::Arc {
            center: c,
            start: s,
            end: e,
            radius: r,
        }))
    }

    /// Adds a circle round point `center` of the sketch.
    pub fn add_circle_on(&mut self, center: PointId, radius: f64) -> Result<CurveId, SketchError> {
        if self.point(center).is_none() {
            return Err(SketchError::Missing);
        }
        if !(radius.is_finite() && radius > 0.0 && radius <= MAX_MM) {
            return Err(SketchError::BadRadius);
        }
        Ok(self.add_curve(Geometry::Circle { center, radius }))
    }

    /// Adds an arc counterclockwise from `start` to `end` round `center`
    /// (points of the sketch) with radius `radius`.
    pub fn add_arc_between(
        &mut self,
        center: PointId,
        start: PointId,
        end: PointId,
        radius: f64,
    ) -> Result<CurveId, SketchError> {
        if [center, start, end].iter().any(|p| self.point(*p).is_none()) {
            return Err(SketchError::Missing);
        }
        if !(radius.is_finite() && radius > 1e-9 && radius <= MAX_MM) {
            return Err(SketchError::BadRadius);
        }
        Ok(self.add_curve(Geometry::Arc {
            center,
            start,
            end,
            radius,
        }))
    }

    /// Makes a point a center point, or not.
    pub fn set_center_point(&mut self, id: PointId, on: bool) {
        if let Some(p) = self.points.iter_mut().find(|p| p.id == id) {
            p.center = on;
        }
    }

    /// True if point `id` is a center point.
    pub fn is_center_point(&self, id: PointId) -> bool {
        self.points.iter().any(|p| p.id == id && p.center)
    }

    /// Makes a line a centerline (also a construction line), or not.
    pub fn set_centerline(&mut self, id: CurveId, on: bool) {
        let Some(c) = self.curves.iter_mut().find(|c| c.id == id && c.is_line()) else {
            return;
        };
        if c.centerline == on {
            return;
        }
        c.centerline = on;
        c.construction = on || c.construction;
        // A distance to the line becomes a diameter, or a distance again.
        let k = if on { 2.0 } else { 0.5 };
        for x in &mut self.constraints {
            if let Constraint::LineDistance { line, mm, .. } = &mut x.constraint
                && *line == id
            {
                *mm *= k;
            }
        }
    }

    /// True if curve `id` is a centerline.
    pub fn is_centerline(&self, id: CurveId) -> bool {
        self.curve(id).is_some_and(|c| c.centerline)
    }

    /// Makes a curve a construction curve, or not.
    pub fn set_construction(&mut self, id: CurveId, on: bool) {
        if let Some(c) = self.curves.iter_mut().find(|c| c.id == id) {
            c.construction = on;
        }
    }

    /// Checks that the constraint has the points and the kinds of curves
    /// that it needs.
    fn check(&self, c: &Constraint) -> Result<(), SketchError> {
        for p in c.points() {
            self.point(p).ok_or(SketchError::Missing)?;
        }
        let curves: Vec<&Curve> = c
            .curves()
            .iter()
            .map(|id| self.curve(*id).ok_or(SketchError::Missing))
            .collect::<Result<_, _>>()?;
        let lines = curves.iter().all(|c| c.is_line());
        let round = curves.iter().all(|c| c.is_round());
        let need = |ok: bool, why: &'static str| if ok { Ok(()) } else { Err(SketchError::WrongCurves(why)) };
        match c {
            Constraint::Horizontal { .. }
            | Constraint::Vertical { .. }
            | Constraint::Parallel { .. }
            | Constraint::Perpendicular { .. }
            | Constraint::Collinear { .. }
            | Constraint::Midpoint { .. }
            | Constraint::Symmetric { .. }
            | Constraint::Length { .. }
            | Constraint::LineDistance { .. }
            | Constraint::Angle { .. } => need(lines, "this constraint needs lines")?,
            Constraint::Concentric { .. } | Constraint::Radius { .. } | Constraint::Diameter { .. } => {
                need(round, "this constraint needs circles or arcs")?
            }
            Constraint::Equal { .. } => need(lines || round, "equal needs two lines, or two circles or arcs")?,
            Constraint::ArcLength { .. } => need(
                curves.iter().all(|c| matches!(c.geometry, Geometry::Arc { .. })),
                "arc length needs an arc",
            )?,
            Constraint::Tangent { .. } => need(
                !lines,
                "tangent needs a line and a circle or an arc, or two circles or arcs",
            )?,
            _ => {}
        }
        if let [a, b] = c.curves().as_slice()
            && a == b
        {
            return Err(SketchError::WrongCurves("choose two different curves"));
        }
        // A tangent to a spline piece is at an end that the two curves share.
        if let Constraint::Tangent { a, b, .. } = c
            && curves.iter().any(|c| c.is_spline())
        {
            let ends = |id: CurveId| self.curve(id).and_then(|c| c.ends()).map(|(p, q)| [p, q]);
            let (Some(ea), Some(eb)) = (ends(*a), ends(*b)) else {
                return Err(SketchError::WrongCurves("a tangent to a spline needs a shared end"));
            };
            let meet = ea
                .iter()
                .any(|p| eb.iter().any(|q| p == q || self.joined(*p).contains(q)));
            if !meet {
                return Err(SketchError::WrongCurves("a tangent to a spline needs a shared end"));
            }
        }
        if let Some(v) = c.value() {
            let ok = match c {
                Constraint::Angle { .. } => v.is_finite() && v.abs() <= 360.0,
                Constraint::HorizontalDistance { .. }
                | Constraint::VerticalDistance { .. }
                | Constraint::LineDistance { .. } => v.is_finite() && v.abs() <= MAX_MM,
                Constraint::Radius { .. } | Constraint::Diameter { .. } | Constraint::ArcLength { .. } => {
                    v.is_finite() && v > 0.0 && v <= MAX_MM
                }
                _ => v.is_finite() && (0.0..=MAX_MM).contains(&v),
            };
            if !ok {
                return Err(SketchError::BadValue);
            }
        }
        Ok(())
    }

    /// Adds a constraint (it is not solved yet: call [`Sketch::solve`]).
    pub fn add_constraint(&mut self, c: Constraint) -> Result<ConstraintId, SketchError> {
        self.check(&c)?;
        let id = ConstraintId(self.take_id());
        self.constraints.push(SketchConstraint {
            id,
            constraint: c,
            driven: false,
        });
        Ok(id)
    }

    /// Adds a dimension with the value that the sketch has now. If the
    /// sketch sets that size already (the dimension repeats or conflicts),
    /// the dimension is driven. Returns the id and true if it is driven.
    pub fn add_dimension(&mut self, c: Constraint) -> Result<(ConstraintId, bool), SketchError> {
        let mut test = self.clone();
        let id = test.add_constraint(c)?;
        let a = test.analyze();
        let driven = a.redundant.contains(&id) || a.conflicting.contains(&id);
        let id = self.add_constraint(c)?;
        if driven {
            self.set_driven(id, true);
        }
        Ok((id, driven))
    }

    /// True for a driven dimension.
    pub fn is_driven(&self, id: ConstraintId) -> bool {
        self.constraints.iter().any(|c| c.id == id && c.driven)
    }

    /// Makes a dimension driven (it only shows the size), or driving again
    /// with the size that the sketch has now.
    pub fn set_driven(&mut self, id: ConstraintId, on: bool) {
        let now = self.constraint(id).and_then(|c| self.measure(c));
        if let Some(c) = self
            .constraints
            .iter_mut()
            .find(|c| c.id == id && c.constraint.is_dimension())
        {
            c.driven = on;
            if let Some(v) = now {
                c.constraint = c.constraint.with_value(v);
            }
        }
    }

    /// The size that the sketch has now for dimension `c`.
    pub fn measure(&self, c: &Constraint) -> Option<f64> {
        let p = |id: PointId| self.point(id);
        let dir = |l: CurveId| match self.curve(l)?.geometry {
            Geometry::Line { a, b } => Some((p(a)?, p(b)? - p(a)?)),
            _ => None,
        };
        Some(match *c {
            Constraint::Distance { a, b, .. } => p(a)?.distance(p(b)?),
            Constraint::HorizontalDistance { a, b, .. } => p(b)?.x - p(a)?.x,
            Constraint::VerticalDistance { a, b, .. } => p(b)?.y - p(a)?.y,
            Constraint::LineDistance { point, line, .. } => {
                let (a, d) = dir(line)?;
                let k = if self.is_centerline(line) { 2.0 } else { 1.0 };
                k * d.cross(p(point)? - a) / d.length().max(1e-12)
            }
            Constraint::Length { line, .. } => dir(line)?.1.length(),
            Constraint::Angle { a, b, .. } => {
                let (da, db) = (dir(a)?.1, dir(b)?.1);
                math::to_degrees(math::atan2(da.cross(db), da.dot(db)))
            }
            Constraint::Radius { curve, .. } => self.radius(curve)?,
            Constraint::Diameter { curve, .. } => 2.0 * self.radius(curve)?,
            Constraint::ArcLength { curve, .. } => match self.shape(curve)? {
                Shape::Arc { r, sweep, .. } => r * sweep,
                _ => return None,
            },
            _ => return None,
        })
    }

    /// The values of the driven dimensions follow the sketch.
    pub fn update_driven(&mut self) {
        let values: Vec<(usize, f64)> = self
            .constraints
            .iter()
            .enumerate()
            .filter(|(_, c)| c.driven)
            .filter_map(|(i, c)| Some((i, self.measure(&c.constraint)?)))
            .collect();
        for (i, v) in values {
            self.constraints[i].constraint = self.constraints[i].constraint.with_value(v);
        }
    }

    /// Changes the value of a dimension.
    pub fn set_value(&mut self, id: ConstraintId, v: f64) -> Result<(), SketchError> {
        let c = self.constraint(id).ok_or(SketchError::Missing)?.with_value(v);
        self.check(&c)?;
        if let Some(x) = self.constraints.iter_mut().find(|x| x.id == id) {
            x.constraint = c;
        }
        Ok(())
    }

    pub fn remove_constraint(&mut self, id: ConstraintId) -> bool {
        let n = self.constraints.len();
        self.constraints.retain(|c| c.id != id);
        self.constraints.len() != n
    }

    /// Removes a curve, its constraints and its points that no other curve
    /// uses.
    pub fn remove_curve(&mut self, id: CurveId) -> bool {
        let Some(c) = self.curve(id).copied() else {
            return false;
        };
        self.curves.retain(|x| x.id != id);
        self.constraints.retain(|x| !x.constraint.curves().contains(&id));
        for p in c.points() {
            if !self.curves.iter().any(|x| x.points().contains(&p)) {
                self.remove_point(p);
            }
        }
        true
    }

    /// Removes a point that no curve uses, and its constraints.
    pub fn remove_point(&mut self, id: PointId) -> bool {
        if self.curves.iter().any(|x| x.points().contains(&id)) {
            return false;
        }
        let n = self.points.len();
        self.points.retain(|p| p.id != id);
        self.constraints.retain(|x| !x.constraint.points().contains(&id));
        self.points.len() != n
    }

    /// The radius of a circle or an arc.
    pub fn radius(&self, id: CurveId) -> Option<f64> {
        match self.curve(id)?.geometry {
            Geometry::Circle { radius, .. } | Geometry::Arc { radius, .. } => Some(radius),
            Geometry::Line { .. } | Geometry::Bezier { .. } => None,
        }
    }

    /// `points` and the handles of the spline pieces that end at them: a
    /// drag of the end of a spline piece moves its handle with it.
    pub fn with_handles(&self, points: &[PointId]) -> Vec<PointId> {
        let mut out = points.to_vec();
        for c in &self.curves {
            if let Geometry::Bezier { start, c1, c2, end } = c.geometry {
                if points.contains(&start) {
                    out.push(c1);
                }
                if points.contains(&end) {
                    out.push(c2);
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Adds a spline piece between points `start` and `end` with handles
    /// `c1` and `c2` (points of the sketch).
    pub fn add_bezier_between(
        &mut self,
        start: PointId,
        c1: PointId,
        c2: PointId,
        end: PointId,
    ) -> Result<CurveId, SketchError> {
        if [start, c1, c2, end].iter().any(|p| self.point(*p).is_none()) {
            return Err(SketchError::Missing);
        }
        Ok(self.add_curve(Geometry::Bezier { start, c1, c2, end }))
    }

    /// Adds a spline through `points` (at least two): one piece between
    /// each two points, with handles from the points before and after
    /// (a Catmull-Rom curve), and a tangent constraint at each joint so the
    /// spline stays smooth. A `closed` spline goes back to the first point.
    /// Returns the pieces.
    pub fn add_spline(&mut self, points: &[Point], closed: bool) -> Result<Vec<CurveId>, SketchError> {
        let n = points.len();
        if n < 2 || (closed && n < 3) {
            return Err(SketchError::WrongCurves("a spline needs two points or more"));
        }
        let ids: Vec<PointId> = points.iter().map(|p| self.add_point(*p)).collect();
        let at = |i: isize| -> Point {
            if closed {
                points[i.rem_euclid(n as isize) as usize]
            } else {
                points[i.clamp(0, n as isize - 1) as usize]
            }
        };
        let count = if closed { n } else { n - 1 };
        let mut pieces = Vec::new();
        for i in 0..count as isize {
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            let h1 = p1 + (p2 - p0) * (1.0 / 6.0);
            let h2 = p2 - (p3 - p1) * (1.0 / 6.0);
            let (c1, c2) = (self.add_point(h1), self.add_point(h2));
            let end = ids[(i as usize + 1) % n];
            pieces.push(self.add_bezier_between(ids[i as usize], c1, c2, end)?);
        }
        let joints = if closed { count } else { count - 1 };
        for k in 0..joints {
            let (a, b) = (pieces[k], pieces[(k + 1) % pieces.len()]);
            self.add_constraint(Constraint::Tangent { a, b, inside: false })?;
        }
        Ok(pieces)
    }

    /// The path of a curve: a line, or cubic curves for a circle or an arc
    /// (at most 90° each).
    pub fn curve_path(&self, id: CurveId) -> Option<Path> {
        let c = self.curve(id)?;
        let p = |id: PointId| self.point(id);
        Some(match c.geometry {
            Geometry::Line { a, b } => Path::new().move_to(p(a)?).line_to(p(b)?),
            Geometry::Circle { center, radius } => Path::ellipse(p(center)?, radius, radius),
            Geometry::Arc {
                center,
                start,
                end,
                radius,
            } => {
                let c = p(center)?;
                let a0 = math::atan2(p(start)?.y - c.y, p(start)?.x - c.x);
                let mut a1 = math::atan2(p(end)?.y - c.y, p(end)?.x - c.x);
                while a1 <= a0 + 1e-12 {
                    a1 += std::f64::consts::TAU;
                }
                arc_path(c, radius, a0, a1)
            }
            Geometry::Bezier { start, c1, c2, end } => Path::new().move_to(p(start)?).cubic_to(p(c1)?, p(c2)?, p(end)?),
        })
    }
}

/// Cubic curves for the arc of a circle from angle `a0` to `a1` (radians,
/// counterclockwise), at most 90° each.
pub fn arc_path(c: Point, r: f64, a0: f64, a1: f64) -> Path {
    let sweep = a1 - a0;
    let n = (sweep / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * math::tan(step / 4.0);
    let at = |a: f64| Point::new(c.x + r * math::cos(a), c.y + r * math::sin(a));
    let tangent = |a: f64| Point::new(-math::sin(a), math::cos(a));
    let mut path = Path::new().move_to(at(a0));
    for i in 0..n {
        let (s, e) = (a0 + step * i as f64, a0 + step * (i + 1) as f64);
        path = path.cubic_to(at(s) + tangent(s) * (k * r), at(e) - tangent(e) * (k * r), at(e));
    }
    path
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edit;
