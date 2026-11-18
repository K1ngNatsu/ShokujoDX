//! Text layout: letters on lines, with kerning, spacing and justification,
//! then on the baseline, with the changes to single letters by hand, then
//! bent by the envelope.

use emb_geom::{BBox, Point, Transform};
use emb_model::{Justify, Letter, Lettering, ObjectKind, PartTuning};

use crate::source::LetterSource;
use crate::style;

/// A letter on a straight line before justification.
struct Placed {
    index: usize,
    c: char,
    letter: u32,
    x: f64,
    advance: f64,
}

/// The letters of `l` in lettering space (see `emb_model::lettering`). The
/// parts have the settings of the source; [`Lettering::set_letters`]
/// applies the placement and the settings of the object.
pub fn layout(l: &Lettering, src: &(impl LetterSource + ?Sized)) -> Vec<Letter> {
    layout_with_box(l, src).0
}

/// As [`layout`], with the text box that the envelope bends: from the
/// start of the first letter to the end of the last one, from the capital
/// height of the first line to the baseline of the last line. `None` with
/// no envelope.
pub fn layout_with_box(l: &Lettering, src: &(impl LetterSource + ?Sized)) -> (Vec<Letter>, Option<BBox>) {
    if l.direction.is_stacked() {
        return (stacked(l, src), None);
    }
    let scale = l.height_mm / src.cap_height();
    // A made bold makes each letter wider on both sides (v0.3 plan, Stage 5).
    let bold_w = if l.bold { style::bold_width(l.height_mm) } else { 0.0 };
    let line_step = src.line_height() * scale * l.line_spacing.max(0.0);
    let path_length = l.baseline.path_length();
    let mut letters = Vec::new();
    let mut index = 0;
    // The text box for the envelope.
    let mut span: Option<(f64, f64)> = None;
    let mut bottom = 0.0;
    for (line_no, line) in l.text.split('\n').enumerate() {
        let y = line_no as f64 * line_step;
        let mut placed: Vec<Placed> = Vec::new();
        let mut x = 0.0;
        let mut prev: Option<u32> = None;
        for c in line.chars() {
            let i = index;
            index += 1;
            x += l.edit(i).map_or(0.0, |e| e.kern_mm);
            let Some(g) = src.letter(c) else {
                // No letter: a space (if it is one) or nothing.
                if c.is_whitespace() {
                    x += src.space() * scale + l.letter_spacing_mm;
                }
                prev = None;
                continue;
            };
            if l.kerning
                && let Some(p) = prev
            {
                x += src.kerning(p, g) * scale;
            }
            let advance = src.advance(g) * scale + 2.0 * bold_w;
            placed.push(Placed {
                index: i,
                c,
                letter: g,
                x,
                advance,
            });
            x += advance + l.letter_spacing_mm;
            prev = Some(g);
        }
        // The line break.
        index += 1;
        let width = (x - l.letter_spacing_mm).max(0.0);
        let shift = match (l.justify, path_length) {
            (Justify::Left, _) => 0.0,
            (Justify::Center, None) => -width / 2.0,
            (Justify::Right, None) => -width,
            (Justify::Center, Some(len)) => (len - width) / 2.0,
            (Justify::Right, Some(len)) => len - width,
        };
        // The underline goes from the start of the first letter to the end
        // of the last one.
        let line_span = placed
            .first()
            .zip(placed.last())
            .map(|(a, b)| (a.x + shift, b.x + shift + b.advance));
        for p in placed {
            let edit = l.edit(p.index);
            let text = p.c.to_string();
            let (mut parts, fixed, tuning) = letter_parts(l, src, p.letter, &text, edit, scale);
            if parts.is_empty() {
                continue;
            }
            if bold_w > 0.0 {
                // The letter starts after its left side.
                for q in &mut parts {
                    q.transform(&Transform::translate(bold_w, 0.0));
                }
            }
            let px = p.x + shift;
            let half = p.advance / 2.0;
            let frame = Transform::translate(px, y).then(l.baseline.letter_transform(px + half, y, half));
            let center = Point::new(half, -l.height_mm / 2.0);
            let t = edit.map_or(Transform::IDENTITY, |e| e.transform(center)).then(frame);
            for q in &mut parts {
                q.transform(&t);
            }
            let (x0, x1) = span.map_or((px, px + p.advance), |(a, b)| (a.min(px), b.max(px + p.advance)));
            span = Some((x0, x1));
            bottom = y;
            letters.push(Letter {
                text,
                line: line_no,
                index: p.index,
                frame,
                center,
                parts,
                fixed,
                tuning,
            });
        }
        // The underline is a letter with no text, at the index of the line
        // break (no edit uses that index).
        if let (Some(u), Some((x0, x1))) = (&l.underline, line_span)
            && let Some(geometry) = style::underline(&l.baseline, y, x0, x1, u)
        {
            letters.push(Letter {
                text: String::new(),
                line: line_no,
                index: index - 1,
                frame: Transform::IDENTITY,
                center: Point::new((x0 + x1) / 2.0, y),
                parts: vec![ObjectKind::Satin {
                    geometry,
                    params: l.satin.clone(),
                }],
                fixed: false,
                tuning: Vec::new(),
            });
        }
    }
    let warp = !l.envelope.is_none() && l.baseline.is_straight();
    let Some((x0, x1)) = span.filter(|_| warp) else {
        return (letters, None);
    };
    let b = BBox::new(Point::new(x0, -l.height_mm), Point::new(x1, bottom));
    for q in letters.iter_mut().flat_map(|x| x.parts.iter_mut()) {
        l.envelope.warp_kind(&b, q);
    }
    (letters, Some(b))
}

/// The parts of one letter in letter space (the baseline at y = 0, scaled
/// to the lettering): the shape of an edit of the letter, else the letter
/// of the font, with the made bold and italic of `l`. Also: the parts sew
/// in this order ([`Letter::fixed`]), and the settings that the font gives
/// to each part ([`Letter::tuning`]); a shape of an edit has none of these.
fn letter_parts(
    l: &Lettering,
    src: &(impl LetterSource + ?Sized),
    letter: u32,
    text: &str,
    edit: Option<&emb_model::LetterEdit>,
    scale: f64,
) -> (Vec<ObjectKind>, bool, Vec<PartTuning>) {
    let (mut parts, fixed, tuning) = match edit.and_then(|e| e.shape.as_ref()).filter(|s| s.text == text) {
        Some(shape) => {
            let k = l.height_mm / shape.height_mm;
            let mut parts = shape.parts.clone();
            for q in &mut parts {
                q.transform(&Transform::scale(k, k));
            }
            (parts, false, Vec::new())
        }
        None => {
            let stitch = edit.and_then(|e| e.stitch).unwrap_or(l.stitch);
            let mut parts = src.parts(letter, stitch, scale);
            for q in &mut parts {
                q.transform(&Transform::scale(scale, scale));
            }
            // A letter that a person put in order keeps its joins.
            let fixed = src.fixed(letter);
            let mut tuning = src.tuning(letter);
            if l.mitre_corners && !fixed {
                emb_model::mitre::join_parts(&mut parts);
            }
            // The settings are for the parts as the font has them.
            if tuning.len() != parts.len() || tuning.iter().all(PartTuning::is_empty) {
                tuning.clear();
            }
            (parts, fixed, tuning)
        }
    };
    if l.bold {
        let w = style::bold_width(l.height_mm);
        for q in &mut parts {
            style::embolden(q, w);
        }
    }
    if l.italic {
        let t = style::italic();
        for q in &mut parts {
            q.transform(&t);
        }
    }
    (parts, fixed, tuning)
}

/// The space between stacked letters, as a factor of the capital height.
const STACK_GAP: f64 = 0.3;
/// The step of a space in stacked text, as a factor of the letter step.
const STACK_SPACE: f64 = 0.6;

/// Stacked text (v0.3 plan, Stage 5): each letter upright, centered on the
/// vertical line of its column, from top to bottom (`TextDirection::Down`)
/// or with the first letter at the bottom (`Up`). Each line of the text is
/// a column; the columns go from left to right. Justify puts the top
/// (Left), the middle (Center) or the bottom (Right) of each column at the
/// origin. A straight baseline only: no envelope and no underline.
fn stacked(l: &Lettering, src: &(impl LetterSource + ?Sized)) -> Vec<Letter> {
    let scale = l.height_mm / src.cap_height();
    let bold_w = if l.bold { style::bold_width(l.height_mm) } else { 0.0 };
    let step = l.height_mm * (1.0 + STACK_GAP) + l.letter_spacing_mm;
    // The columns are as far apart as the widest letter of the text.
    let widest = l
        .text
        .chars()
        .filter_map(|c| src.letter(c))
        .map(|g| src.advance(g) * scale + 2.0 * bold_w)
        .fold(0.0, f64::max);
    let column_step = (widest + l.height_mm * STACK_GAP) * l.line_spacing.max(0.0);
    let mut letters = Vec::new();
    let mut index = 0;
    for (line_no, line) in l.text.split('\n').enumerate() {
        let cx = line_no as f64 * column_step;
        let mut chars: Vec<(usize, char)> = Vec::new();
        for c in line.chars() {
            chars.push((index, c));
            index += 1;
        }
        // The line break.
        index += 1;
        if l.direction == emb_model::TextDirection::Up {
            chars.reverse();
        }
        // (index, char, letter, advance, baseline y) of each letter.
        let mut column = Vec::new();
        let mut y = 0.0;
        for (i, c) in chars {
            y += l.edit(i).map_or(0.0, |e| e.kern_mm);
            match src.letter(c) {
                Some(g) => {
                    let advance = src.advance(g) * scale + 2.0 * bold_w;
                    column.push((i, c, g, advance, y + l.height_mm));
                    y += step;
                }
                None if c.is_whitespace() => y += step * STACK_SPACE,
                None => {}
            }
        }
        let length = column.last().map_or(0.0, |c| c.4);
        let shift = match l.justify {
            Justify::Left => 0.0,
            Justify::Center => -length / 2.0,
            Justify::Right => -length,
        };
        for (i, c, g, advance, base) in column {
            let edit = l.edit(i);
            let text = c.to_string();
            let (mut parts, fixed, tuning) = letter_parts(l, src, g, &text, edit, scale);
            if parts.is_empty() {
                continue;
            }
            if bold_w > 0.0 {
                for q in &mut parts {
                    q.transform(&Transform::translate(bold_w, 0.0));
                }
            }
            let half = advance / 2.0;
            let frame = Transform::translate(cx - half, base + shift);
            let center = Point::new(half, -l.height_mm / 2.0);
            let t = edit.map_or(Transform::IDENTITY, |e| e.transform(center)).then(frame);
            for q in &mut parts {
                q.transform(&t);
            }
            letters.push(Letter {
                text,
                line: line_no,
                index: i,
                frame,
                center,
                parts,
                fixed,
                tuning,
            });
        }
    }
    letters
}

/// Makes the letters of `l` again from `src`, in place, with the size range
/// of the font.
pub fn relayout(l: &mut Lettering, src: &(impl LetterSource + ?Sized)) {
    let (letters, b) = layout_with_box(l, src);
    l.range_mm = src.size_range(l.stitch);
    l.envelope_box = b;
    l.set_letters(letters);
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Transform};
    use emb_model::{Baseline, Envelope, EnvelopeShape, FontRef, ObjectKind};

    use super::*;
    use crate::bundled;
    use emb_model::LetterStitch;

    fn bbox(letters: &[Letter]) -> BBox {
        letters
            .iter()
            .flat_map(|l| l.parts.iter())
            .filter_map(|p| p.paths().filter_map(|q| q.control_bbox()).reduce(BBox::union))
            .reduce(BBox::union)
            .unwrap()
    }

    fn lettering(text: &str) -> Lettering {
        Lettering::new(text, FontRef::new("Lato", "Bold"), 20.0, Point::new(0.0, 0.0))
    }

    #[test]
    fn height_is_the_capital_height() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let letters = layout(&lettering("H"), &f);
        let b = bbox(&letters);
        assert!((b.height() - 20.0).abs() < 0.3, "{}", b.height());
        assert!(b.max.y.abs() < 0.3, "on the baseline: {}", b.max.y);
        assert!(letters[0].parts.iter().all(|p| matches!(p, ObjectKind::Satin { .. })));
    }

    #[test]
    fn spaces_lines_justification_and_kerning() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("A B\nCD");
        let letters = layout(&l, &f);
        assert_eq!(letters.iter().map(|x| x.text.as_str()).collect::<String>(), "ABCD");
        assert_eq!(letters[2].line, 1);
        assert!(bbox(&letters[2..]).min.y > 10.0, "the second line is lower");
        l.justify = Justify::Right;
        assert!(bbox(&layout(&l, &f)).max.x < 0.5);
        l.text = "AV".into();
        let kerned = bbox(&layout(&l, &f)).width();
        l.kerning = false;
        assert!(bbox(&layout(&l, &f)).width() > kerned + 0.1);
        l.stitch = LetterStitch::Fill;
        assert!(
            layout(&l, &f)
                .iter()
                .flat_map(|x| &x.parts)
                .all(|p| matches!(p, ObjectKind::Fill { .. }))
        );
    }

    #[test]
    fn size_ranges() {
        let bold = bundled::font("Lato", "Bold").unwrap();
        let regular = bundled::font("Lato", "Regular").unwrap();
        let (lo_b, hi_b) = LetterSource::size_range(&bold, LetterStitch::Satin).unwrap();
        let (lo_r, _) = LetterSource::size_range(&regular, LetterStitch::Satin).unwrap();
        assert!(lo_b < lo_r, "thicker strokes sew smaller: {lo_b} {lo_r}");
        assert!(lo_b >= 5.0 && hi_b > 30.0, "{lo_b} {hi_b}");
        let (lo_f, _) = LetterSource::size_range(&bold, LetterStitch::Fill).unwrap();
        assert!(lo_f > lo_b);
        let mut l = lettering("Hi");
        relayout(&mut l, &bold);
        assert_eq!(l.range_mm, LetterSource::size_range(&bold, LetterStitch::Auto));
    }

    fn center(l: &Letter) -> Point {
        let b = bbox(std::slice::from_ref(l));
        b.min.midpoint(b.max)
    }

    #[test]
    fn letters_on_a_circle() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut l = lettering("TEAM");
        l.font = LetterSource::font_ref(&f);
        l.justify = Justify::Center;
        l.baseline = Baseline::Circle {
            radius_mm: 40.0,
            clockwise: true,
        };
        let letters = layout(&l, &f);
        assert_eq!(letters.len(), 4);
        for x in &letters {
            let r = center(x).length();
            // The middle of each letter box is about half a capital height outside
            // the circle.
            assert!((r - 50.0).abs() < 4.0, "{} {r}", x.text);
        }
        // Over the top, from left to right.
        assert!(center(&letters[0]).x < 0.0 && center(&letters[3]).x > 0.0);
        assert!(letters.iter().all(|x| center(x).y < -25.0));
        // Under the bottom (counter-clockwise), the letters hang inside.
        l.baseline = Baseline::Circle {
            radius_mm: 40.0,
            clockwise: false,
        };
        let letters = layout(&l, &f);
        for x in &letters {
            assert!((center(x).length() - 30.0).abs() < 4.0, "{}", x.text);
        }
        assert!(
            center(&letters[0]).x < 0.0 && center(&letters[0]).y > 10.0,
            "{:?}",
            center(&letters[0])
        );
    }

    #[test]
    fn justify_on_a_path() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("Hi");
        l.baseline = Baseline::Path {
            path: Path::new()
                .move_to(Point::new(0.0, 0.0))
                .line_to(Point::new(100.0, 0.0)),
        };
        let left = bbox(&layout(&l, &f));
        assert!(left.min.x < 3.0);
        l.justify = Justify::Right;
        let right = bbox(&layout(&l, &f));
        assert!(right.max.x > 97.0 && right.max.x < 100.5, "{right:?}");
        l.justify = Justify::Center;
        let mid = bbox(&layout(&l, &f));
        assert!((mid.min.x + mid.max.x - 100.0).abs() < 3.0, "{mid:?}");
    }

    #[test]
    fn edits_of_single_letters() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("HIH");
        let plain = layout(&l, &f);
        // Kerning before the "I" moves the "I" and the last "H".
        l.edit_mut(1).kern_mm = 3.0;
        let kerned = layout(&l, &f);
        assert!((center(&kerned[0]).x - center(&plain[0]).x).abs() < 1e-9);
        for i in 1..3 {
            assert!((center(&kerned[i]).x - center(&plain[i]).x - 3.0).abs() < 1e-6);
        }
        // A move changes only its letter.
        l.edits.clear();
        l.edit_mut(1).offset = Point::new(0.0, 5.0);
        let moved = layout(&l, &f);
        assert!((center(&moved[1]).y - center(&plain[1]).y - 5.0).abs() < 1e-6);
        assert!((center(&moved[2]).x - center(&plain[2]).x).abs() < 1e-9);
        // Size and turn are about the middle of the letter.
        l.edits.clear();
        let e = l.edit_mut(0);
        e.scale = 1.5;
        e.rotate_deg = 90.0;
        let turned = layout(&l, &f);
        let (a, b) = (bbox(&plain[0..1]), bbox(&turned[0..1]));
        assert!((b.width() - a.height() * 1.5).abs() < 0.5, "{a:?} {b:?}");
        assert!(center(&turned[0]).distance(center(&plain[0])) < 1.0);
        // An edit for a character with no letter (the space) only kerns.
        l.edits.clear();
        l.text = "H H".into();
        l.edit_mut(1).kern_mm = 2.0;
        let spaced = layout(&l, &f);
        assert_eq!(spaced[1].index, 2);
    }

    #[test]
    fn a_shape_changed_by_hand_stays() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("HI");
        l.placement = Transform::rotate_deg(30.0).then(Transform::translate(10.0, 20.0));
        l.edit_mut(1).rotate_deg = 10.0;
        relayout(&mut l, &f);
        // Move the "I" 2 mm to the right in the design, as the Reshape tool
        // does, and keep it.
        for p in &mut l.letters[1].parts {
            p.transform(&Transform::translate(2.0, 0.0));
        }
        let changed = l.letters[1].parts.clone();
        l.keep_shape(1);
        relayout(&mut l, &f);
        let near = |a: &[ObjectKind], b: &[ObjectKind]| {
            let pa: Vec<Point> = a
                .iter()
                .flat_map(|p| p.paths().flat_map(|q| q.points().collect::<Vec<_>>()))
                .collect();
            let pb: Vec<Point> = b
                .iter()
                .flat_map(|p| p.paths().flat_map(|q| q.points().collect::<Vec<_>>()))
                .collect();
            pa.len() == pb.len() && pa.iter().zip(&pb).all(|(x, y)| x.distance(*y) < 1e-6)
        };
        assert!(near(&l.letters[1].parts, &changed), "the new layout keeps the shape");
        // A larger height scales the shape.
        let before = bbox(&l.letters[1..2]).width();
        l.height_mm *= 2.0;
        relayout(&mut l, &f);
        assert!((bbox(&l.letters[1..2]).width() / before - 2.0).abs() < 0.01);
        // Another character at that index does not take the shape.
        l.edits[0].shape.as_mut().unwrap().text = "X".into();
        relayout(&mut l, &f);
        assert!(!near(&l.letters[1].parts, &changed));
    }

    #[test]
    fn a_bridge_envelope_bends_the_text() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut l = lettering("HIIIH");
        l.font = LetterSource::font_ref(&f);
        let flat = layout(&l, &f);
        l.envelope = Envelope {
            shape: EnvelopeShape::Bridge,
            amount: 0.5,
        };
        let (bent, b) = layout_with_box(&l, &f);
        let b = b.unwrap();
        assert!((b.height() - 20.0).abs() < 1e-9 && b.width() > 40.0, "{b:?}");
        let h = |x: &[Letter], i: usize| bbox(&x[i..=i]).height();
        // The middle letter is about half again as tall; the ends less so.
        assert!(h(&bent, 2) > h(&flat, 2) * 1.4, "{} {}", h(&bent, 2), h(&flat, 2));
        assert!(h(&bent, 0) < h(&bent, 2));
        // The bottom stays on the baseline.
        assert!(bbox(&bent).max.y.abs() < 0.3);
        // No envelope on a curved baseline.
        l.baseline = Baseline::Arc {
            radius_mm: 50.0,
            clockwise: true,
        };
        assert!(layout_with_box(&l, &f).1.is_none());
    }

    #[test]
    fn a_letter_keeps_its_place_in_the_envelope() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("HIH");
        l.envelope = Envelope {
            shape: EnvelopeShape::ArchOver,
            amount: 0.4,
        };
        relayout(&mut l, &f);
        // A move by hand without a new layout is the same as with one.
        l.change_letter(1, |e| e.offset = Point::new(0.0, 2.0));
        let moved = l.letters[1].parts.clone();
        relayout(&mut l, &f);
        let pts = |k: &[ObjectKind]| -> Vec<Point> {
            k.iter()
                .flat_map(|p| p.paths().flat_map(|q| q.points().collect::<Vec<_>>()))
                .collect()
        };
        let (a, b) = (pts(&moved), pts(&l.letters[1].parts));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x.distance(*y) < 1e-6));
        // A kept shape comes back the same through the envelope.
        let before = pts(&l.letters[1].parts);
        l.keep_shape(1);
        relayout(&mut l, &f);
        let after = pts(&l.letters[1].parts);
        assert!(after.len() >= before.len());
        let worst = before
            .iter()
            .map(|p| after.iter().map(|q| p.distance(*q)).fold(f64::MAX, f64::min))
            .fold(0.0, f64::max);
        assert!(worst < 0.05, "{worst}");
    }

    #[test]
    fn one_letter_with_its_own_stitch_type() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("II");
        l.stitch = LetterStitch::Satin;
        l.edit_mut(1).stitch = Some(LetterStitch::Fill);
        let x = layout(&l, &f);
        assert!(x[0].parts.iter().all(|p| matches!(p, ObjectKind::Satin { .. })));
        assert!(x[1].parts.iter().all(|p| matches!(p, ObjectKind::Fill { .. })));
        // And its own underlay.
        l.edit_mut(0).underlay = Some(emb_model::Underlay::None);
        relayout(&mut l, &f);
        let underlay = |k: &ObjectKind| match k {
            ObjectKind::Satin { params, .. } => params.underlay.clone(),
            ObjectKind::Fill { params, .. } => params.underlay.clone(),
            _ => unreachable!(),
        };
        assert!(
            l.letters[0]
                .parts
                .iter()
                .all(|p| underlay(p) == emb_model::Underlay::None)
        );
        assert!(
            l.letters[1]
                .parts
                .iter()
                .all(|p| underlay(p) == emb_model::Underlay::Auto)
        );
    }

    #[test]
    fn a_scale_on_the_canvas_is_a_new_height() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        for (baseline, envelope) in [
            (Baseline::Straight, EnvelopeShape::None),
            (Baseline::Straight, EnvelopeShape::Bridge),
            (
                Baseline::Arc {
                    radius_mm: 40.0,
                    clockwise: true,
                },
                EnvelopeShape::None,
            ),
        ] {
            let mut l = lettering("Anna");
            l.font = LetterSource::font_ref(&f);
            l.baseline = baseline.clone();
            l.envelope = Envelope {
                shape: envelope,
                amount: 0.4,
            };
            l.edit_mut(1).kern_mm = 2.0;
            l.edit_mut(2).offset = Point::new(0.5, 1.0);
            l.placement = Transform::rotate_deg(20.0).then(Transform::translate(5.0, 7.0));
            relayout(&mut l, &f);
            l.range_mm = Some((10.0, 30.0));
            // A scale by 1.5 on the canvas.
            let mut kind = ObjectKind::Lettering(Box::new(l));
            let t = Transform::translate(-3.0, -4.0)
                .then(Transform::scale(1.5, 1.5))
                .then(Transform::translate(3.0, 4.0));
            kind.transform(&t);
            let ObjectKind::Lettering(mut l) = kind else { panic!() };
            assert!((l.height_mm - 30.0).abs() < 1e-9, "{}", l.height_mm);
            assert!(l.size_warning().is_none());
            let scaled = bbox(&l.letters);
            // A new layout from the font gives the same letters.
            relayout(&mut l, &f);
            let again = bbox(&l.letters);
            assert!(
                scaled.min.distance(again.min) < 0.05 && scaled.max.distance(again.max) < 0.05,
                "{baseline:?} {envelope:?}: {scaled:?} {again:?}"
            );
            // Larger again (above a range of 10 to 30 mm): the size warning shows.
            l.range_mm = Some((10.0, 30.0));
            let mut kind = ObjectKind::Lettering(l);
            kind.transform(&Transform::scale(1.2, 1.2));
            let ObjectKind::Lettering(l) = kind else { panic!() };
            assert!((l.height_mm - 36.0).abs() < 1e-9);
            assert!(l.size_warning().is_some_and(|w| w.contains("larger")));
        }
    }

    #[test]
    fn auto_uses_fill_for_wide_strokes() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut l = lettering("I");
        l.height_mm = 120.0;
        assert!(matches!(layout(&l, &f)[0].parts[0], ObjectKind::Fill { .. }));
        l.height_mm = 10.0;
        assert!(matches!(layout(&l, &f)[0].parts[0], ObjectKind::Satin { .. }));
    }
}

#[cfg(test)]
mod style_tests {
    use emb_geom::BBox;
    use emb_model::{FontRef, TextDirection, Underline};

    use super::*;
    use crate::bundled;

    fn bbox(letters: &[Letter]) -> BBox {
        letters
            .iter()
            .flat_map(|l| l.parts.iter())
            .filter_map(|p| p.paths().filter_map(|q| q.control_bbox()).reduce(BBox::union))
            .reduce(BBox::union)
            .unwrap()
    }

    fn lettering(text: &str) -> Lettering {
        Lettering::new(
            text,
            FontRef::embroidery(bundled::DEFAULT_EMBROIDERY, ""),
            20.0,
            Point::new(0.0, 0.0),
        )
    }

    #[test]
    fn made_bold_and_italic_change_the_letters() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut l = lettering("HIL");
        let plain = bbox(&layout(&l, &f));
        l.bold = true;
        let bold = bbox(&layout(&l, &f));
        assert!(bold.width() > plain.width() + 2.0, "{plain:?} {bold:?}");
        assert!(
            bold.height() > plain.height(),
            "the strokes are wider at the top and bottom too"
        );
        l.bold = false;
        l.italic = true;
        let letters = layout(&l, &f);
        // The top of the "I" moves to the right; its bottom does not.
        let i = &letters[1];
        let b = bbox(std::slice::from_ref(i));
        let plain_i = bbox(&layout(&lettering("HIL"), &f)[1..2]);
        assert!(b.max.x > plain_i.max.x + 2.0, "{plain_i:?} {b:?}");
        assert!((b.min.x - plain_i.min.x).abs() < 1.0);
    }

    #[test]
    fn an_underline_is_under_each_line() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut l = lettering("AB\nCDE");
        l.underline = Some(Underline::for_height(20.0));
        let letters = layout(&l, &f);
        let bars: Vec<&Letter> = letters.iter().filter(|x| x.text.is_empty()).collect();
        assert_eq!(bars.len(), 2);
        let line0 = bbox(
            &letters
                .iter()
                .filter(|x| x.line == 0 && !x.text.is_empty())
                .cloned()
                .collect::<Vec<_>>(),
        );
        let bar0 = bbox(&[bars[0].clone()]);
        assert!(bar0.min.y > 0.0, "under the baseline: {bar0:?}");
        assert!(bar0.min.x <= line0.min.x + 0.5 && bar0.max.x >= line0.max.x - 0.5);
        // Stacked text has no underline.
        l.direction = TextDirection::Down;
        assert!(layout(&l, &f).iter().all(|x| !x.text.is_empty()));
    }

    #[test]
    fn stacked_letters_go_down_or_up_on_one_line() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut l = lettering("OPEN");
        l.direction = TextDirection::Down;
        let letters = layout(&l, &f);
        assert_eq!(letters.len(), 4);
        let boxes: Vec<BBox> = letters.iter().map(|x| bbox(std::slice::from_ref(x))).collect();
        // Each letter is under the one before it, on one vertical line.
        for w in boxes.windows(2) {
            assert!(w[1].min.y > w[0].max.y, "{w:?}");
            assert!((w[1].center().x - w[0].center().x).abs() < 2.0, "{w:?}");
        }
        assert_eq!(letters[0].text, "O");
        // Up: the first letter is at the bottom.
        l.direction = TextDirection::Up;
        let up = layout(&l, &f);
        assert_eq!(up[0].text, "N");
        assert_eq!(up[3].text, "O");
        // A second line is a column to the right.
        l.direction = TextDirection::Down;
        l.text = "AB\nCD".into();
        let two = layout(&l, &f);
        let c = two.iter().find(|x| x.text == "C").unwrap();
        let a = two.iter().find(|x| x.text == "A").unwrap();
        assert!(bbox(std::slice::from_ref(c)).min.x > bbox(std::slice::from_ref(a)).max.x);
    }
}
