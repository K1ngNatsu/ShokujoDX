//! The constraint commands: the constraint that the picks of a command
//! make, the dimension of the Dimension tool and its kinds, and the text
//! of a constraint.

use super::*;

/// A constraint command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Coincident,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Collinear,
    Tangent,
    Equal,
    Concentric,
    Midpoint,
    OnCurve,
    Symmetric,
    Fix,
}

impl Kind {
    pub const ALL: [Kind; 13] = [
        Kind::Coincident,
        Kind::Horizontal,
        Kind::Vertical,
        Kind::Parallel,
        Kind::Perpendicular,
        Kind::Collinear,
        Kind::Tangent,
        Kind::Equal,
        Kind::Concentric,
        Kind::Midpoint,
        Kind::OnCurve,
        Kind::Symmetric,
        Kind::Fix,
    ];

    /// The name of the command (as on the ribbon).
    pub fn name(self) -> &'static str {
        match self {
            Kind::Coincident => tr("Coincident"),
            Kind::Horizontal => tr("Horizontal"),
            Kind::Vertical => tr("Vertical"),
            Kind::Parallel => tr("Parallel"),
            Kind::Perpendicular => tr("Perpendicular"),
            Kind::Collinear => tr("Collinear"),
            Kind::Tangent => tr("Tangent"),
            Kind::Equal => tr("Equal"),
            Kind::Concentric => tr("Concentric"),
            Kind::Midpoint => tr("Midpoint"),
            Kind::OnCurve => tr("On Curve"),
            Kind::Symmetric => tr("Symmetric"),
            Kind::Fix => tr("Fix"),
        }
    }

    /// What to click, for the status line.
    pub fn prompt(self) -> &'static str {
        match self {
            Kind::Coincident => tr("Click two points."),
            Kind::Horizontal | Kind::Vertical => tr("Click a line, or two points."),
            Kind::Parallel | Kind::Perpendicular | Kind::Collinear => tr("Click two lines."),
            Kind::Tangent => tr("Click a line and a circle or an arc, or two circles or arcs."),
            Kind::Equal => tr("Click two lines, or two circles or arcs."),
            Kind::Concentric => tr("Click two circles or arcs."),
            Kind::Midpoint => tr("Click a point and a line."),
            Kind::OnCurve => tr("Click a point and a curve."),
            Kind::Symmetric => tr("Click two points and the line of symmetry."),
            Kind::Fix => tr("Click a point."),
        }
    }
}

/// The result of the picks of a constraint command.
pub(super) enum Build {
    Done(Constraint),
    More,
    Wrong(&'static str),
}

impl Build {
    pub(super) fn done(self) -> Option<Constraint> {
        match self {
            Build::Done(c) => Some(c),
            _ => None,
        }
    }
}

pub(super) fn is_line(s: &Sketch, c: CurveId) -> bool {
    s.curve(c).is_some_and(|c| c.is_line())
}

pub(super) fn ends(s: &Sketch, c: CurveId) -> Option<(PointId, PointId)> {
    match s.curve(c)?.geometry {
        Geometry::Line { a, b } => Some((a, b)),
        _ => None,
    }
}

/// The constraint of `kind` for `picks` (points and curves in click order).
pub(super) fn build(kind: Kind, s: &Sketch, picks: &[Pick]) -> Build {
    let pts: Vec<PointId> = picks
        .iter()
        .filter_map(|p| match p {
            Pick::Point(p) => Some(*p),
            _ => None,
        })
        .collect();
    let cs: Vec<CurveId> = picks
        .iter()
        .filter_map(|p| match p {
            Pick::Curve(c) => Some(*c),
            _ => None,
        })
        .collect();
    let lines: Vec<CurveId> = cs.iter().copied().filter(|c| is_line(s, *c)).collect();
    let round: Vec<CurveId> = cs.iter().copied().filter(|c| !is_line(s, *c)).collect();
    let wrong = |why| Build::Wrong(why);
    match kind {
        Kind::Coincident => match pts.as_slice() {
            [a, b] => Build::Done(Constraint::Coincident { a: *a, b: *b }),
            [_] if cs.is_empty() => Build::More,
            [] if cs.is_empty() => Build::More,
            _ => wrong(tr("Coincident needs two points.")),
        },
        Kind::Horizontal | Kind::Vertical => {
            let h = kind == Kind::Horizontal;
            match (lines.as_slice(), pts.as_slice(), round.is_empty()) {
                ([l], [], true) => Build::Done(if h {
                    Constraint::Horizontal { line: *l }
                } else {
                    Constraint::Vertical { line: *l }
                }),
                // Two points: a distance of 0 across.
                ([], [a, b], true) => Build::Done(if h {
                    Constraint::VerticalDistance { a: *a, b: *b, mm: 0.0 }
                } else {
                    Constraint::HorizontalDistance { a: *a, b: *b, mm: 0.0 }
                }),
                ([], [_] | [], true) => Build::More,
                _ => wrong(tr("Horizontal and vertical need a line, or two points.")),
            }
        }
        Kind::Parallel | Kind::Perpendicular | Kind::Collinear => {
            match (lines.as_slice(), pts.is_empty() && round.is_empty()) {
                ([a, b], true) => Build::Done(match kind {
                    Kind::Parallel => Constraint::Parallel { a: *a, b: *b },
                    Kind::Perpendicular => Constraint::Perpendicular { a: *a, b: *b },
                    _ => Constraint::Collinear { a: *a, b: *b },
                }),
                ([_] | [], true) => Build::More,
                _ => wrong(tr("This constraint needs two lines.")),
            }
        }
        Kind::Tangent => match (cs.as_slice(), pts.is_empty()) {
            ([a, b], true) if !(is_line(s, *a) && is_line(s, *b)) => Build::Done(Constraint::Tangent {
                a: *a,
                b: *b,
                inside: tangent_inside(s, *a, *b),
            }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr(
                "Tangent needs a line and a circle or an arc, or two circles or arcs.",
            )),
        },
        Kind::Equal => match (cs.as_slice(), pts.is_empty()) {
            ([a, b], true) if is_line(s, *a) == is_line(s, *b) => Build::Done(Constraint::Equal { a: *a, b: *b }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr("Equal needs two lines, or two circles or arcs.")),
        },
        Kind::Concentric => match (round.as_slice(), pts.is_empty() && lines.is_empty()) {
            ([a, b], true) => Build::Done(Constraint::Concentric { a: *a, b: *b }),
            ([_] | [], true) => Build::More,
            _ => wrong(tr("Concentric needs two circles or arcs.")),
        },
        Kind::Midpoint => match (pts.as_slice(), lines.as_slice(), round.is_empty()) {
            ([p], [l], true) => Build::Done(Constraint::Midpoint { point: *p, line: *l }),
            ([_] | [], [_] | [], true) => Build::More,
            _ => wrong(tr("Midpoint needs a point and a line.")),
        },
        Kind::OnCurve => match (pts.as_slice(), cs.as_slice()) {
            ([p], [c]) => Build::Done(Constraint::OnCurve { point: *p, curve: *c }),
            ([_] | [], [_] | []) => Build::More,
            _ => wrong(tr("On curve needs a point and a curve.")),
        },
        Kind::Symmetric => match (pts.as_slice(), lines.as_slice(), round.is_empty()) {
            ([a, b], [l], true) => Build::Done(Constraint::Symmetric { a: *a, b: *b, line: *l }),
            ([] | [_] | [_, _], [] | [_], true) => Build::More,
            _ => wrong(tr("Symmetric needs two points and a line.")),
        },
        Kind::Fix => match (pts.as_slice(), cs.is_empty()) {
            ([p], true) => match s.point(*p) {
                Some(at) => Build::Done(Constraint::Fix { point: *p, at }),
                None => wrong(tr("Fix needs a point.")),
            },
            ([], true) => Build::More,
            _ => wrong(tr("Fix needs a point.")),
        },
    }
}

/// True if two round curves touch with one inside the other (as they are
/// now), for a tangent constraint.
fn tangent_inside(s: &Sketch, a: CurveId, b: CurveId) -> bool {
    let center = |c: CurveId| match s.curve(c)?.geometry {
        Geometry::Circle { center, radius } | Geometry::Arc { center, radius, .. } => Some((s.point(center)?, radius)),
        Geometry::Line { .. } | Geometry::Bezier { .. } => None,
    };
    match (center(a), center(b)) {
        (Some((ca, ra)), Some((cb, rb))) => {
            let d = ca.distance(cb);
            (d - (ra - rb).abs()).abs() < (d - (ra + rb)).abs()
        }
        _ => false,
    }
}

/// A constraint as text: its name and its value.
pub(super) fn constraint_text(units: crate::workspace::Units, c: &Constraint) -> String {
    match c.value() {
        Some(v) if matches!(c, Constraint::Angle { .. }) => format!("{} {v:.1}°", tr(c.name())),
        Some(v) => format!("{} {}", tr(c.name()), units.fmt(v.abs())),
        None => tr(c.name()).to_string(),
    }
}

/// The kind of a length or a distance by the place of its label `at`: above
/// or below the two points a horizontal distance, left or right of them a
/// vertical distance, else along them.
pub(super) fn orient(s: &Sketch, c: Constraint, at: Point) -> Constraint {
    let (a, b) = match c {
        Constraint::Distance { a, b, .. }
        | Constraint::HorizontalDistance { a, b, .. }
        | Constraint::VerticalDistance { a, b, .. } => (a, b),
        Constraint::Length { line, .. } => match ends(s, line) {
            Some(e) => e,
            None => return c,
        },
        _ => return c,
    };
    let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
        return c;
    };
    let d = pb - pa;
    // A line that is horizontal or vertical has one kind only.
    if d.x.abs() < 1e-6 || d.y.abs() < 1e-6 {
        return c;
    }
    let in_x = at.x > pa.x.min(pb.x) && at.x < pa.x.max(pb.x);
    let in_y = at.y > pa.y.min(pb.y) && at.y < pa.y.max(pb.y);
    match (in_x, in_y) {
        (true, false) => Constraint::HorizontalDistance { a, b, mm: d.x },
        (false, true) => Constraint::VerticalDistance { a, b, mm: d.y },
        _ => match c {
            Constraint::HorizontalDistance { .. } | Constraint::VerticalDistance { .. } => Constraint::Distance {
                a,
                b,
                mm: pa.distance(pb),
            },
            other => other,
        },
    }
}

/// The other kinds of a pending dimension (for the right-click menu): the
/// name and the dimension.
pub(super) fn dimension_kinds(s: &Sketch, c: &Constraint) -> Vec<(&'static str, Constraint)> {
    match *c {
        Constraint::Radius { curve, .. } | Constraint::Diameter { curve, .. } | Constraint::ArcLength { curve, .. } => {
            let Some(r) = s.radius(curve) else { return Vec::new() };
            let mut out = vec![
                (tr("Radius"), Constraint::Radius { curve, mm: r }),
                (tr("Diameter"), Constraint::Diameter { curve, mm: 2.0 * r }),
            ];
            let arc = Constraint::ArcLength { curve, mm: 0.0 };
            if let Some(len) = s.measure(&arc) {
                out.push((tr("Arc Length"), arc.with_value(len)));
            }
            out
        }
        Constraint::Distance { a, b, .. }
        | Constraint::HorizontalDistance { a, b, .. }
        | Constraint::VerticalDistance { a, b, .. } => points_kinds(s, a, b),
        Constraint::Length { line, .. } => match ends(s, line) {
            Some((a, b)) => {
                let mut out = points_kinds(s, a, b);
                if let Some(l) = s.measure(c) {
                    out[0].1 = Constraint::Length { line, mm: l };
                }
                out
            }
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn points_kinds(s: &Sketch, a: PointId, b: PointId) -> Vec<(&'static str, Constraint)> {
    let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
        return Vec::new();
    };
    vec![
        (
            tr("Aligned"),
            Constraint::Distance {
                a,
                b,
                mm: pa.distance(pb),
            },
        ),
        (
            tr("Horizontal"),
            Constraint::HorizontalDistance { a, b, mm: pb.x - pa.x },
        ),
        (tr("Vertical"), Constraint::VerticalDistance { a, b, mm: pb.y - pa.y }),
    ]
}

/// The dimension for the picks of the Dimension tool, with the value as
/// the sketch is now. A line waits for the next click: another line (an
/// angle), a point (the distance to the line) or an empty place (`empty`:
/// its length).
pub(super) fn dimension_of(s: &Sketch, picks: &[Pick], empty: bool) -> Build {
    let line_distance = |p: PointId, l: CurveId| match (ends(s, l), s.point(p)) {
        (Some((a, b)), Some(q)) => match (s.point(a), s.point(b)) {
            (Some(pa), Some(pb)) if pa.distance(pb) > 1e-9 => Build::Done(Constraint::LineDistance {
                point: p,
                line: l,
                mm: (pb - pa).cross(q - pa) / pa.distance(pb) * if s.is_centerline(l) { 2.0 } else { 1.0 },
            }),
            _ => Build::Wrong(tr("No such line.")),
        },
        _ => Build::Wrong(tr("No such line.")),
    };
    match picks {
        [Pick::Curve(c)] if is_line(s, *c) && !empty => Build::More,
        [Pick::Curve(c)] => match s.curve(*c).map(|c| c.geometry) {
            Some(Geometry::Line { a, b }) => match (s.point(a), s.point(b)) {
                (Some(pa), Some(pb)) => Build::Done(Constraint::Length {
                    line: *c,
                    mm: pa.distance(pb),
                }),
                _ => Build::Wrong(tr("No such line.")),
            },
            Some(Geometry::Circle { radius, .. }) => Build::Done(Constraint::Diameter {
                curve: *c,
                mm: 2.0 * radius,
            }),
            Some(Geometry::Arc { radius, .. }) => Build::Done(Constraint::Radius { curve: *c, mm: radius }),
            Some(Geometry::Bezier { .. }) => {
                Build::Wrong(tr("A spline takes no dimension: give dimensions to its points."))
            }
            None => Build::Wrong(tr("No such curve.")),
        },
        [Pick::Point(a), Pick::Point(b)] => match (s.point(*a), s.point(*b)) {
            (Some(pa), Some(pb)) => Build::Done(Constraint::Distance {
                a: *a,
                b: *b,
                mm: pa.distance(pb),
            }),
            _ => Build::Wrong(tr("No such point.")),
        },
        [Pick::Curve(a), Pick::Curve(b)] if is_line(s, *a) && is_line(s, *b) => {
            let dir = |c: CurveId| {
                let (p, q) = ends(s, c)?;
                Some(s.point(q)? - s.point(p)?)
            };
            match (dir(*a), dir(*b)) {
                (Some(da), Some(db)) => Build::Done(Constraint::Angle {
                    a: *a,
                    b: *b,
                    deg: math::to_degrees(math::atan2(da.cross(db), da.dot(db))),
                }),
                _ => Build::Wrong(tr("No such line.")),
            }
        }
        [Pick::Point(p), Pick::Curve(l)] | [Pick::Curve(l), Pick::Point(p)] if is_line(s, *l) => line_distance(*p, *l),
        [Pick::Point(_)] => Build::More,
        [Pick::Curve(c), _] if !is_line(s, *c) => Build::Wrong(tr("Click one circle or arc for its size.")),
        _ => Build::Wrong(tr("Click a line, a circle or an arc, two points or two lines.")),
    }
}

/// True if two constraints are the same kind on the same parts (the
/// values and the fixed place do not count).
pub(super) fn same_constraint(a: &Constraint, b: &Constraint) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b) && {
        let sorted = |c: &Constraint| {
            let (mut p, mut q) = (c.points(), c.curves());
            p.sort();
            q.sort();
            (p, q)
        };
        sorted(a) == sorted(b)
    }
}
