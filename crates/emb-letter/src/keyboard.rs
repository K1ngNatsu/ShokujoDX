//! Keyboard design collections (P4 Milestone 6, ADR 0020): typing with key
//! designs, a key design from objects, and the built-in collection.

use emb_geom::{BBox, Point, Transform};
use emb_model::{KeyDesign, KeyObject, KeyboardCollection, ObjectKind, Rgb};

use crate::embfont::EmbFont;

/// The objects of `text` typed with `k` at `height_mm` (the capital height),
/// with the baseline start at `at`, and the characters that have no key
/// design. A line break starts a new line below. The objects sew color by
/// color (in the order the colors first come), so the machine changes
/// color as few times as possible and a part of one key cannot sew over
/// the key before it in another color.
pub fn type_text(
    k: &KeyboardCollection,
    text: &str,
    height_mm: f64,
    spacing_mm: f64,
    at: Point,
) -> (Vec<(Rgb, ObjectKind)>, Vec<char>) {
    let s = height_mm / k.height_mm.max(1e-6);
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for (line_no, line) in text.split('\n').enumerate() {
        let mut x = 0.0;
        let y = line_no as f64 * 1.4 * height_mm;
        for c in line.chars() {
            if c == ' ' {
                x += k.space_mm * s + spacing_mm;
                continue;
            }
            let Some(key) = k.key(c) else {
                if !missing.contains(&c) {
                    missing.push(c);
                }
                continue;
            };
            let t = Transform::scale(s, s).then(Transform::translate(at.x + x, at.y + y));
            for o in &key.objects {
                let mut kind = o.kind.clone();
                kind.transform(&t);
                out.push((o.color, kind));
            }
            x += key.advance_mm * s + spacing_mm;
        }
    }
    let mut colors: Vec<Rgb> = Vec::new();
    for (c, _) in &out {
        if !colors.contains(c) {
            colors.push(*c);
        }
    }
    out.sort_by_key(|(c, _)| colors.iter().position(|x| x == c));
    (out, missing)
}

/// A key design from objects of a design: the bottom of the objects is the
/// baseline and their height becomes the design height `height_mm`.
pub fn key_from_objects(objects: &[(Rgb, ObjectKind)], height_mm: f64) -> Option<KeyDesign> {
    let b = objects
        .iter()
        .filter_map(|(_, k)| k.control_bbox())
        .reduce(BBox::union)?;
    let s = height_mm / b.height().max(1e-6);
    let t = Transform::translate(-b.min.x, -b.max.y).then(Transform::scale(s, s));
    let objects = objects
        .iter()
        .map(|(color, kind)| {
            let mut kind = kind.clone();
            kind.transform(&t);
            KeyObject { color: *color, kind }
        })
        .collect();
    Some(KeyDesign {
        advance_mm: b.width() * s + 0.12 * height_mm,
        objects,
    })
}

/// The built-in collection: the letters of an embroidery font over a
/// shadow (the same letter moved down and right) in a second color.
pub fn shadow_collection(font: &EmbFont, name: &str, main: Rgb, shadow: Rgb) -> KeyboardCollection {
    let mut k = KeyboardCollection::new(name, font.cap_height_mm);
    k.space_mm = font.space_mm;
    let d = 0.06 * font.cap_height_mm;
    let step = Transform::translate(d, d);
    for (c, letter) in &font.letters {
        if letter.parts.is_empty() {
            continue;
        }
        let mut objects: Vec<KeyObject> = letter
            .parts
            .iter()
            .map(|p| {
                let mut kind = p.clone();
                kind.transform(&step);
                KeyObject { color: shadow, kind }
            })
            .collect();
        objects.extend(letter.parts.iter().map(|p| KeyObject {
            color: main,
            kind: p.clone(),
        }));
        k.keys.insert(
            c.to_string(),
            KeyDesign {
                advance_mm: letter.advance_mm + d,
                objects,
            },
        );
    }
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled;

    #[test]
    fn typing_with_a_collection() {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let k = shadow_collection(&f, "Shadow", Rgb(0, 0, 0), Rgb(200, 0, 0));
        assert!(k.keys.len() > 90);
        let (objs, missing) = type_text(&k, "AB ☃", 20.0, 1.0, Point::new(10.0, 30.0));
        assert_eq!(missing, ['☃']);
        let a = k.key('A').unwrap().objects.len();
        let b = k.key('B').unwrap().objects.len();
        assert_eq!(objs.len(), a + b);
        // Two colors: the shadow first, then the letter, for each key.
        assert_eq!(objs[0].0, Rgb(200, 0, 0));
        let bb = objs
            .iter()
            .filter_map(|(_, k)| k.control_bbox())
            .reduce(BBox::union)
            .unwrap();
        assert!((bb.max.y - 30.0).abs() < 2.5, "on the baseline: {bb:?}");
        assert!(bb.height() > 19.0 && bb.height() < 24.0, "{bb:?}");
        // A key design from objects: the bottom left at the origin, at the design height.
        let key = key_from_objects(&objs, 50.0).unwrap();
        let kb = key
            .objects
            .iter()
            .filter_map(|o| o.kind.control_bbox())
            .reduce(BBox::union)
            .unwrap();
        assert!(kb.min.x.abs() < 1e-9 && kb.max.y.abs() < 1e-9 && (kb.height() - 50.0).abs() < 1e-9);
    }
}
