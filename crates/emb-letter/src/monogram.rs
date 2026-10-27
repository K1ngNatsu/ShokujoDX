//! Monograms (P4 Milestone 5, ADR 0019): the objects of a monogram from its
//! initials and a [`MonogramSpec`].
//!
//! The initials are one lettering object. The styles use the tools of
//! lettering: letter edits (size and position of single letters) and
//! envelopes. The ornaments are small fill shapes (a scroll is a triple
//! run) of our own design, at the sides or above and below the initials.
//! The border goes round the initials and the ornaments.

use emb_geom::{BBox, Path, Point, Transform, math};
use emb_model::{
    BorderShape, BorderStitch, Design, Envelope, EnvelopeShape, FillParams, Justify, Lettering, MonogramRecord,
    MonogramSpec, MonogramStyle, ObjectId, ObjectKind, OrnamentPlace, OrnamentShape, RunningParams, SatinGeometry,
    SatinParams,
};

use crate::layout::relayout;
use crate::source::LetterSource;

/// The objects of a monogram, in design coordinates.
#[derive(Debug, Clone)]
pub struct MonogramObjects {
    pub letters: Lettering,
    pub ornaments: Vec<ObjectKind>,
    pub borders: Vec<ObjectKind>,
}

/// The initials that a monogram uses: up to three characters that are not
/// spaces (one for the single style).
pub fn initials(text: &str, style: MonogramStyle) -> String {
    let max = if style == MonogramStyle::Single { 1 } else { 3 };
    text.chars().filter(|c| !c.is_whitespace()).take(max).collect()
}

/// Size of the side initials of the classic style, as a part of the middle one.
const CLASSIC_SIDE: f64 = 0.65;
/// Size of the small initials of the stacked style.
const STACKED_SMALL: f64 = 0.45;

/// Makes the objects of the monogram of `text` with its middle at `center`.
/// `None` if the font has none of the initials.
pub fn build(
    text: &str,
    spec: &MonogramSpec,
    src: &(impl LetterSource + ?Sized),
    center: Point,
) -> Option<MonogramObjects> {
    let text = initials(text, spec.style);
    let h = spec.height_mm.max(1.0);
    let mut l = Lettering::new(&text, src.font_ref(), h, Point::ZERO);
    l.justify = Justify::Center;
    l.letter_spacing_mm = 0.05 * h;
    let n = text.chars().count();
    match spec.style {
        MonogramStyle::Single | MonogramStyle::Block => {}
        MonogramStyle::Classic if n == 3 => {
            l.height_mm = h * CLASSIC_SIDE;
            relayout(&mut l, src);
            let s = 1.0 / CLASSIC_SIDE;
            let adv = 2.0 * l.letters.get(1)?.center.x;
            let room = (s - 1.0) * adv / 2.0;
            l.edit_mut(1).scale = s;
            l.edit_mut(1).kern_mm = room;
            l.edit_mut(2).kern_mm = room;
        }
        MonogramStyle::Classic => {}
        MonogramStyle::Stacked if n >= 2 => {
            relayout(&mut l, src);
            let first = l.letters.first()?;
            let x0 = first.frame.apply(Point::new(2.0 * first.center.x, 0.0)).x + 0.08 * h;
            let col = l.letters[1..]
                .iter()
                .map(|x| 2.0 * x.center.x * STACKED_SMALL)
                .fold(0.0, f64::max);
            let half = STACKED_SMALL * h / 2.0;
            let targets = if n == 2 { vec![-h / 2.0] } else { vec![-h + half, -half] };
            let moves: Vec<(usize, Point)> = l.letters[1..]
                .iter()
                .zip(targets)
                .map(|(x, ty)| {
                    let now = x.frame.apply(x.center);
                    (x.index, Point::new(x0 + col / 2.0 - now.x, ty - now.y))
                })
                .collect();
            for (index, offset) in moves {
                let e = l.edit_mut(index);
                e.scale = STACKED_SMALL;
                e.offset = offset;
            }
        }
        MonogramStyle::Stacked => {}
        MonogramStyle::Diamond => {
            l.envelope = Envelope {
                shape: EnvelopeShape::Diamond,
                amount: 0.6,
            };
        }
        MonogramStyle::Oval => {
            l.envelope = Envelope {
                shape: EnvelopeShape::Oval,
                amount: 0.45,
            };
        }
    }
    relayout(&mut l, src);
    if l.letters.is_empty() {
        return None;
    }
    // The initials: their middle at `center`.
    let b = parts_box(l.parts())?;
    let mut kind = ObjectKind::Lettering(Box::new(l));
    kind.transform(&Transform::translate(center.x - b.center().x, center.y - b.center().y));
    let ObjectKind::Lettering(l) = kind else { unreachable!() };
    let letters_box = parts_box(l.parts())?;

    let ornaments = ornaments(spec, &letters_box);
    let content = ornaments
        .iter()
        .filter_map(|k| k.control_bbox())
        .fold(letters_box, BBox::union);
    let borders = borders(spec, &content);
    Some(MonogramObjects {
        letters: *l,
        ornaments,
        borders,
    })
}

/// Adds a monogram to `d` as one group: the initials, the ornaments and the
/// border, in this sew order, each in the color of the spec. It goes at the
/// end of the sew order, or (with `replace`) where the objects of that
/// monogram group were. The design keeps its record. Returns the new objects.
pub fn add_to_design(
    d: &mut Design,
    text: &str,
    spec: &MonogramSpec,
    m: MonogramObjects,
    center: Point,
    replace: Option<u32>,
) -> Vec<ObjectId> {
    let mut at = d.objects().len();
    let group = match replace {
        Some(g) => {
            let old: Vec<ObjectId> = d
                .objects()
                .iter()
                .filter(|o| o.group == Some(g))
                .map(|o| o.id)
                .collect();
            if let Some(i) = old.first().and_then(|id| d.objects().iter().position(|o| o.id == *id)) {
                at = i;
            }
            for id in old {
                d.remove_object(id);
            }
            d.monograms.retain(|r| r.group != g);
            g
        }
        None => d
            .new_group_id()
            .max(d.monograms.iter().map(|r| r.group + 1).max().unwrap_or(1)),
    };
    let kinds: Vec<(emb_model::Rgb, ObjectKind)> =
        std::iter::once((spec.letter_color, ObjectKind::Lettering(Box::new(m.letters))))
            .chain(m.ornaments.into_iter().map(|k| (spec.ornament_color, k)))
            .chain(m.borders.into_iter().map(|k| (spec.border_color, k)))
            .collect();
    let count = kinds.len();
    let mut ids = Vec::with_capacity(count);
    for (k, (color, kind)) in kinds.into_iter().enumerate() {
        // Each object but the last trims: a travel stitch between them
        // (an ornament and the border, two rings) would show.
        let trim = k + 1 < count;
        let t = d.thread_for_color(color);
        let id = d.insert_object(at, t, kind);
        at += 1;
        if let Some(o) = d.object_mut(id) {
            o.group = Some(group);
            if trim {
                o.functions.trim = emb_model::TrimMode::Always;
            }
        }
        ids.push(id);
    }
    d.monograms.push(MonogramRecord {
        group,
        initials: initials(text, spec.style),
        spec: spec.clone(),
        center,
    });
    ids
}

fn parts_box<'a>(parts: impl Iterator<Item = &'a ObjectKind>) -> Option<BBox> {
    parts.filter_map(|p| p.control_bbox()).reduce(BBox::union)
}

/// An ornament of size 1 at the origin, pointing to the right.
fn ornament(shape: OrnamentShape) -> Option<ObjectKind> {
    let p = Point::new;
    let fill = |outer: Path| ObjectKind::Fill {
        outer,
        holes: vec![],
        params: FillParams::default(),
    };
    Some(match shape {
        OrnamentShape::None => return None,
        OrnamentShape::Dot => fill(Path::ellipse(Point::ZERO, 0.5, 0.5)),
        OrnamentShape::Diamond => fill(Path::polygon(&[p(-0.5, 0.0), p(0.0, -0.35), p(0.5, 0.0), p(0.0, 0.35)])),
        OrnamentShape::Star => {
            let pts: Vec<Point> = (0..10)
                .map(|i| {
                    let r = if i % 2 == 0 { 0.5 } else { 0.21 };
                    let (s, c) = emb_geom::math::sin_cos_deg(-90.0 + 36.0 * f64::from(i));
                    p(r * c, r * s + 0.05)
                })
                .collect();
            fill(Path::polygon(&pts))
        }
        OrnamentShape::Heart => fill(
            Path::new()
                .move_to(p(0.0, 0.45))
                .cubic_to(p(-0.6, 0.0), p(-0.45, -0.55), p(0.0, -0.22))
                .cubic_to(p(0.45, -0.55), p(0.6, 0.0), p(0.0, 0.45))
                .close(),
        ),
        OrnamentShape::Leaf => fill(
            Path::new()
                .move_to(p(-0.5, 0.0))
                .cubic_to(p(-0.2, -0.32), p(0.2, -0.3), p(0.5, 0.0))
                .cubic_to(p(0.2, 0.3), p(-0.2, 0.32), p(-0.5, 0.0))
                .close(),
        ),
        OrnamentShape::Scroll => ObjectKind::Running {
            path: Path::new()
                .move_to(p(-0.5, 0.05))
                .cubic_to(p(-0.25, -0.3), p(0.05, 0.3), p(0.3, 0.0))
                .cubic_to(p(0.48, -0.2), p(0.3, -0.42), p(0.14, -0.27))
                .cubic_to(p(0.04, -0.17), p(0.14, -0.06), p(0.24, -0.13)),
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    })
}

/// The ornaments of `spec` round the box of the initials.
fn ornaments(spec: &MonogramSpec, b: &BBox) -> Vec<ObjectKind> {
    let Some(unit) = ornament(spec.ornaments.shape) else {
        return Vec::new();
    };
    let s = spec.ornaments.size_mm.max(1.0);
    let gap = 0.12 * spec.height_mm + 0.5;
    let c = b.center();
    let mut places: Vec<(Point, bool)> = Vec::new();
    if matches!(spec.ornaments.place, OrnamentPlace::Sides | OrnamentPlace::All) {
        places.push((Point::new(b.min.x - gap - s / 2.0, c.y), true));
        places.push((Point::new(b.max.x + gap + s / 2.0, c.y), false));
    }
    if matches!(spec.ornaments.place, OrnamentPlace::TopBottom | OrnamentPlace::All) {
        places.push((Point::new(c.x, b.min.y - gap - s / 2.0), false));
        places.push((Point::new(c.x, b.max.y + gap + s / 2.0), true));
    }
    places
        .into_iter()
        .map(|(at, mirror)| {
            let mut k = unit.clone();
            let sx = if mirror { -s } else { s };
            k.transform(&Transform::scale(sx, s).then(Transform::translate(at.x, at.y)));
            k
        })
        .collect()
}

/// The closed line of a border shape round box `b` at distance `d`.
fn border_line(shape: BorderShape, b: &BBox, d: f64) -> Option<Path> {
    let c = b.center();
    let (w, h) = (b.width() / 2.0, b.height() / 2.0);
    Some(match shape {
        BorderShape::None => return None,
        BorderShape::Circle | BorderShape::DoubleCircle => {
            let r = math::hypot(w, h) + d;
            Path::ellipse(c, r, r)
        }
        BorderShape::Oval => Path::ellipse(c, w * std::f64::consts::SQRT_2 + d, h * std::f64::consts::SQRT_2 + d),
        BorderShape::Square => {
            let s = w.max(h) + d;
            Path::rect(BBox::new(Point::new(c.x - s, c.y - s), Point::new(c.x + s, c.y + s)))
        }
        BorderShape::RoundedSquare => {
            let r = (0.25 * h.min(w) + d).max(1.0);
            Path::rounded_rect(b.expand(d), r)
        }
        BorderShape::Diamond => {
            // |x|/a + |y|/b = 1 through the corners of the box.
            let (a, bb) = (2.0 * w + d * 1.5, 2.0 * h + d * 1.5);
            Path::polygon(&[
                Point::new(c.x - a, c.y),
                Point::new(c.x, c.y - bb),
                Point::new(c.x + a, c.y),
                Point::new(c.x, c.y + bb),
            ])
        }
    })
}

/// The border objects of `spec` round box `b`.
fn borders(spec: &MonogramSpec, b: &BBox) -> Vec<ObjectKind> {
    let bd = &spec.border;
    let width = bd.width_mm.clamp(0.8, 10.0);
    let stitch = |line: Path, stitch: BorderStitch| match stitch {
        BorderStitch::Satin => ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&line, width),
            params: SatinParams::default(),
        },
        BorderStitch::Run | BorderStitch::Triple => ObjectKind::Running {
            path: line,
            params: RunningParams {
                repeats: if stitch == BorderStitch::Triple { 3 } else { 1 },
                ..RunningParams::default()
            },
        },
    };
    let inner = match bd.stitch {
        BorderStitch::Satin => bd.margin_mm + width / 2.0,
        _ => bd.margin_mm,
    };
    let Some(line) = border_line(bd.shape, b, inner) else {
        return Vec::new();
    };
    let mut out = vec![stitch(line, bd.stitch)];
    if bd.shape == BorderShape::DoubleCircle
        && let Some(outer) = border_line(bd.shape, b, inner + width / 2.0 + 2.0)
    {
        out.push(stitch(outer, BorderStitch::Triple));
    }
    out
}

#[cfg(test)]
mod tests {
    use emb_model::{Border, FontRef, Ornaments};

    use super::*;
    use crate::bundled;

    fn spec(style: MonogramStyle) -> MonogramSpec {
        MonogramSpec {
            style,
            ..MonogramSpec::new(FontRef::embroidery(bundled::DEFAULT_EMBROIDERY, ""))
        }
    }

    fn letter_heights(m: &MonogramObjects) -> Vec<f64> {
        m.letters
            .letters
            .iter()
            .map(|x| parts_box(x.parts.iter()).unwrap().height())
            .collect()
    }

    #[test]
    fn styles() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let at = Point::new(10.0, 20.0);
        for style in MonogramStyle::ALL {
            let m = build("J m K", &spec(style), &f, at).unwrap();
            let b = parts_box(m.letters.parts()).unwrap();
            assert!(b.center().distance(at) < 1e-6, "{style:?} {b:?}");
            let n = if style == MonogramStyle::Single { 1 } else { 3 };
            assert_eq!(m.letters.letters.len(), n, "{style:?}");
            assert!(m.borders.is_empty() && m.ornaments.is_empty());
        }
        let classic = letter_heights(&build("JMK", &spec(MonogramStyle::Classic), &f, at).unwrap());
        assert!(
            classic[1] > classic[0] * 1.4 && classic[1] > classic[2] * 1.4,
            "{classic:?}"
        );
        assert!(
            (classic[1] - 30.0).abs() < 1.0,
            "the middle is the full height: {classic:?}"
        );
        // Stacked: the small initials are one above the other, to the right.
        let m = build("JMK", &spec(MonogramStyle::Stacked), &f, at).unwrap();
        let bx: Vec<BBox> = m
            .letters
            .letters
            .iter()
            .map(|x| parts_box(x.parts.iter()).unwrap())
            .collect();
        assert!(bx[1].max.y < bx[2].min.y, "{bx:?}");
        assert!(bx[1].min.x > bx[0].max.x && bx[2].min.x > bx[0].max.x);
        assert!(bx[1].height() < bx[0].height() * 0.6);
        assert_eq!(initials("a b c d", MonogramStyle::Block), "abc");
    }

    #[test]
    fn a_monogram_in_a_design_is_one_group_and_can_change() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut d = Design::new("");
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(-5.0, -5.0), Point::new(5.0, 5.0))),
                holes: vec![],
                params: FillParams::default(),
            },
        );
        let mut s = spec(MonogramStyle::Classic);
        s.border.shape = BorderShape::Circle;
        let m = build("JMK", &s, &f, Point::ZERO).unwrap();
        let ids = add_to_design(&mut d, "JMK", &s, m, Point::ZERO, None);
        assert_eq!(ids.len(), 2);
        let g = d.object(ids[0]).unwrap().group.unwrap();
        assert!(ids.iter().all(|id| d.object(*id).unwrap().group == Some(g)));
        assert_eq!(d.monograms.len(), 1);
        assert_ne!(
            d.object(ids[0]).unwrap().thread,
            d.object(ids[1]).unwrap().thread,
            "letters and border have their own colors"
        );
        // Change it: the same group, in the same place in the sew order.
        s.border.shape = BorderShape::DoubleCircle;
        let m = build("ABC", &s, &f, Point::ZERO).unwrap();
        let ids = add_to_design(&mut d, "ABC", &s, m, Point::ZERO, Some(g));
        assert_eq!(ids.len(), 3);
        assert_eq!(d.objects().len(), 4);
        assert_eq!(d.objects()[1].id, ids[0]);
        assert_eq!(d.monograms.len(), 1);
        assert_eq!(d.monograms[0].initials, "ABC");
    }

    #[test]
    fn borders_and_ornaments_go_round() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        for shape in BorderShape::ALL {
            for orn in OrnamentShape::ALL {
                let mut s = spec(MonogramStyle::Classic);
                s.border = Border {
                    shape,
                    ..Border::default()
                };
                s.ornaments = Ornaments {
                    shape: orn,
                    place: OrnamentPlace::All,
                    ..Ornaments::default()
                };
                let m = build("ABC", &s, &f, Point::ZERO).unwrap();
                let content = m
                    .ornaments
                    .iter()
                    .filter_map(|k| k.control_bbox())
                    .fold(parts_box(m.letters.parts()).unwrap(), BBox::union);
                let n = match shape {
                    BorderShape::None => 0,
                    BorderShape::DoubleCircle => 2,
                    _ => 1,
                };
                assert_eq!(m.borders.len(), n, "{shape:?}");
                assert_eq!(m.ornaments.len(), if orn == OrnamentShape::None { 0 } else { 4 });
                for border in &m.borders {
                    let bb = border.control_bbox().unwrap();
                    assert!(
                        bb.min.x < content.min.x && bb.max.x > content.max.x && bb.min.y < content.min.y,
                        "{shape:?} {orn:?}: {bb:?} {content:?}"
                    );
                }
            }
        }
    }
}
