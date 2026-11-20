//! The labels of the constraints on the canvas: the dimensions with their
//! leader lines, and the short marks of the other constraints.

use super::*;

/// The label of a constraint on the canvas: where, the text, and the
/// points of its leader line (for a dimension).
pub(super) struct Label {
    pub(super) id: ConstraintId,
    pub(super) rect: Rect,
    text: String,
    pub(super) leader: Option<(Pos2, Pos2)>,
    /// The extension lines of a length or a distance.
    ext: Option<[(Pos2, Pos2); 2]>,
    pub(super) dimension: bool,
}

/// The short mark of a constraint that is not a dimension.
pub(super) fn mark(c: &Constraint) -> &'static str {
    match c {
        Constraint::Horizontal { .. } | Constraint::VerticalDistance { .. } => "H",
        Constraint::Vertical { .. } | Constraint::HorizontalDistance { .. } => "V",
        Constraint::Parallel { .. } => "//",
        Constraint::Perpendicular { .. } => "⊥",
        Constraint::Collinear { .. } => "C",
        Constraint::Tangent { .. } => "T",
        Constraint::Equal { .. } => "=",
        Constraint::Concentric { .. } => "◎",
        Constraint::Midpoint { .. } => "M",
        Constraint::OnCurve { .. } => "O",
        Constraint::Symmetric { .. } => "S",
        Constraint::Fix { .. } => "F",
        _ => "",
    }
}

/// The labels of the constraints of the open sketch.
pub(super) fn labels(obj: &SketchObject, map: &ViewMap, units: crate::workspace::Units) -> Vec<Label> {
    labels_with(obj, map, units, true)
}

/// The labels; with `marks` false only the dimensions.
pub(super) fn labels_with(
    obj: &SketchObject,
    map: &ViewMap,
    units: crate::workspace::Units,
    marks: bool,
) -> Vec<Label> {
    let s = obj.sketch();
    let to = |p: Point| map.to_screen(obj.to_design(p));
    let pt = |id: PointId| s.point(id).map(to);
    let mid = |c: CurveId| -> Option<Pos2> {
        match s.curve(c)?.geometry {
            Geometry::Line { a, b } => Some(pt(a)?.lerp(pt(b)?, 0.5)),
            Geometry::Circle { center, radius } => Some(to(s.point(center)? + Point::new(radius, 0.0))),
            Geometry::Arc { start, .. } => pt(start),
            Geometry::Bezier { .. } => Some(to(s.shape(c)?.at(0.5))),
        }
    };
    let mut out = Vec::new();
    // Marks of one curve stack along it.
    let mut stack: std::collections::BTreeMap<CurveId, f32> = Default::default();
    let font_h = 14.0;
    // A length or a distance goes on the side away from the middle of the
    // sketch, so it does not cover the shape.
    let middle = {
        let n = s.points.len().max(1) as f32;
        let sum = s.points.iter().fold(Vec2::ZERO, |acc, p| acc + to(p.pos).to_vec2());
        Pos2::new(sum.x / n, sum.y / n)
    };
    let away = |pa: Pos2, pb: Pos2| -> Vec2 {
        let d = (pb - pa).normalized();
        let n = Vec2::new(-d.y, d.x);
        let m = pa.lerp(pb, 0.5);
        if n.dot(m - middle) < 0.0 { -n * 22.0 } else { n * 22.0 }
    };
    let mut ext = None;
    // A label that the user put somewhere.
    let placed = |id: ConstraintId| s.label_at.get(&id).map(|p| to(*p));
    // The offset of a length from its line, and the place of its label.
    let offset = |pa: Pos2, pb: Pos2, id: ConstraintId| -> (Vec2, Pos2) {
        let m = pa.lerp(pb, 0.5);
        match placed(id) {
            Some(at) => {
                let d = (pb - pa).normalized();
                let n = Vec2::new(-d.y, d.x);
                (n * n.dot(at - m), at)
            }
            None => {
                let n = away(pa, pb);
                (n, m + n)
            }
        }
    };
    for sc in &s.constraints {
        let c = &sc.constraint;
        if !marks && !c.is_dimension() {
            continue;
        }
        let (anchor, text, leader, dimension) = match *c {
            Constraint::Coincident { .. } => continue,
            Constraint::HorizontalDistance { a, b, mm } | Constraint::VerticalDistance { a, b, mm } if mm != 0.0 => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let across = matches!(c, Constraint::HorizontalDistance { .. });
                let m = pa.lerp(pb, 0.5);
                // The dimension line is horizontal (or vertical), on the
                // side away from the middle of the sketch, or at the label.
                let at = placed(sc.id).unwrap_or_else(|| {
                    if across {
                        let y = if middle.y > m.y {
                            pa.y.min(pb.y) - 22.0
                        } else {
                            pa.y.max(pb.y) + 22.0
                        };
                        Pos2::new(m.x, y)
                    } else {
                        let x = if middle.x > m.x {
                            pa.x.min(pb.x) - 22.0
                        } else {
                            pa.x.max(pb.x) + 22.0
                        };
                        Pos2::new(x, m.y)
                    }
                });
                let (qa, qb) = if across {
                    (Pos2::new(pa.x, at.y), Pos2::new(pb.x, at.y))
                } else {
                    (Pos2::new(at.x, pa.y), Pos2::new(at.x, pb.y))
                };
                ext = Some([(pa, qa), (pb, qb)]);
                (at, units.fmt(mm.abs()), Some((qa, qb)), true)
            }
            Constraint::ArcLength { curve, mm } => {
                let Some(sh @ emb_sketch::Shape::Arc { c: cp, sweep, .. }) = s.shape(curve) else {
                    continue;
                };
                let mid = to(sh.at(sweep / 2.0));
                let out = (mid - to(cp)).normalized();
                (
                    placed(sc.id).unwrap_or(mid + out * 24.0),
                    format!("⌒ {}", units.fmt(mm)),
                    None,
                    true,
                )
            }
            Constraint::Distance { a, b, mm } => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let (n, at) = offset(pa, pb, sc.id);
                ext = Some([(pa, pa + n * 1.15), (pb, pb + n * 1.15)]);
                (at, units.fmt(mm.abs()), Some((pa + n, pb + n)), true)
            }
            Constraint::LineDistance { point, line, mm } => {
                let (Some((a, b)), Some(q)) = (ends(s, line), s.point(point)) else {
                    continue;
                };
                let (Some(pa), Some(pb)) = (s.point(a), s.point(b)) else {
                    continue;
                };
                let d = pb - pa;
                let foot = pa + d * ((q - pa).dot(d) / d.length_sq().max(1e-12));
                let (sq, sf) = (to(q), to(foot));
                // To a centerline: a diameter.
                let text = if s.is_centerline(line) {
                    format!("⌀ {}", units.fmt(mm.abs()))
                } else {
                    units.fmt(mm.abs())
                };
                (
                    placed(sc.id).unwrap_or(sq.lerp(sf, 0.5) + Vec2::new(10.0, 0.0)),
                    text,
                    Some((sq, sf)),
                    true,
                )
            }
            Constraint::Length { line, mm } => {
                let Some((a, b)) = ends(s, line) else { continue };
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                let (n, at) = offset(pa, pb, sc.id);
                ext = Some([(pa, pa + n * 1.15), (pb, pb + n * 1.15)]);
                (at, units.fmt(mm), Some((pa + n, pb + n)), true)
            }
            Constraint::Radius { curve, mm } | Constraint::Diameter { curve, mm } => {
                let Some((center, r)) = (match s.curve(curve).map(|c| c.geometry) {
                    Some(Geometry::Circle { center, radius } | Geometry::Arc { center, radius, .. }) => {
                        Some((center, radius))
                    }
                    _ => None,
                }) else {
                    continue;
                };
                let Some(cp) = s.point(center) else { continue };
                // An arc: the label at the middle of the arc, out of it.
                let dir = match s.shape(curve) {
                    Some(sh @ emb_sketch::Shape::Arc { sweep, .. }) => (sh.at(sweep / 2.0) - cp).normalized(),
                    _ => None,
                }
                .unwrap_or(Point::new(
                    std::f64::consts::FRAC_1_SQRT_2,
                    -std::f64::consts::FRAC_1_SQRT_2,
                ));
                let edge = to(cp + dir * r);
                let (edge, at) = match placed(sc.id) {
                    // The leader goes to the label.
                    Some(at) => {
                        let rs = edge.distance(to(cp));
                        (to(cp) + (at - to(cp)).normalized() * rs, at)
                    }
                    None => (edge, edge + (edge - to(cp)).normalized() * 26.0),
                };
                let prefix = if matches!(c, Constraint::Diameter { .. }) {
                    "⌀"
                } else {
                    "R"
                };
                (at, format!("{prefix} {}", units.fmt(mm)), Some((to(cp), edge)), true)
            }
            Constraint::Angle { a, b, deg } => {
                let (Some(ma), Some(mb)) = (mid(a), mid(b)) else {
                    continue;
                };
                (
                    placed(sc.id).unwrap_or(ma.lerp(mb, 0.5)),
                    format!("{deg:.1}°"),
                    None,
                    true,
                )
            }
            Constraint::Fix { point, .. } | Constraint::Midpoint { point, .. } | Constraint::OnCurve { point, .. } => {
                let Some(p) = pt(point) else { continue };
                (p + Vec2::new(10.0, -12.0), mark(c).to_string(), None, false)
            }
            Constraint::Symmetric { a, .. } => {
                let Some(p) = pt(a) else { continue };
                (p + Vec2::new(10.0, -12.0), mark(c).to_string(), None, false)
            }
            Constraint::HorizontalDistance { a, b, .. } | Constraint::VerticalDistance { a, b, .. } => {
                let (Some(pa), Some(pb)) = (pt(a), pt(b)) else { continue };
                (
                    pa.lerp(pb, 0.5) + Vec2::new(8.0, -10.0),
                    mark(c).to_string(),
                    None,
                    false,
                )
            }
            _ => {
                let curves = c.curves();
                let Some(&first) = curves.first() else { continue };
                let Some(m) = mid(first) else { continue };
                let k = stack.entry(first).or_insert(0.0);
                let at = m + Vec2::new(14.0 + *k, 14.0);
                *k += 16.0;
                (at, mark(c).to_string(), None, false)
            }
        };
        // A driven dimension in parentheses.
        let text = if sc.driven { format!("({text})") } else { text };
        // A dimension with an expression: its parameter, or ƒx (ADR 0078).
        let text = match s.dimension_expr(sc.id) {
            Some(e) if emb_sketch::expr::valid_name(e) => format!("{e} = {text}"),
            Some(_) => format!("ƒx {text}"),
            None => text,
        };
        let w = 8.0 * text.chars().count() as f32 + 6.0;
        let rect = Rect::from_center_size(anchor, Vec2::new(w, font_h + 2.0));
        out.push(Label {
            id: sc.id,
            rect,
            text,
            leader,
            ext: ext.take(),
            dimension,
        });
    }
    out
}

/// Draws a label: its leader and extension lines, its box and its text.
/// `alpha` below 1 draws a ghost (the dimension that a click makes).
pub(super) fn draw_label(
    painter: &egui::Painter,
    l: &Label,
    color: Color32,
    fill: Color32,
    text_color: Color32,
    frame: Stroke,
    alpha: f32,
) {
    let fade = |c: Color32| c.gamma_multiply(alpha);
    if let Some((p, q)) = l.leader {
        painter.line_segment([p, q], Stroke::new(1.0, fade(color.gamma_multiply(0.7))));
    }
    for (p, q) in l.ext.iter().flatten() {
        painter.line_segment([*p, *q], Stroke::new(0.8, fade(color.gamma_multiply(0.45))));
    }
    painter.rect_filled(l.rect, 3.0, fade(fill));
    painter.rect_stroke(
        l.rect,
        3.0,
        Stroke::new(frame.width, fade(frame.color)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        l.rect.center(),
        egui::Align2::CENTER_CENTER,
        &l.text,
        egui::FontId::proportional(13.0),
        fade(text_color),
    );
}
