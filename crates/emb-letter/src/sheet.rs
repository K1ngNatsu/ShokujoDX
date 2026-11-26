//! Font test sheets (P17 Milestone 4, ADR 0112): a design with the letters
//! of a font at some sizes, to sew and to look at. Each size is a group: a
//! label with the name of the font and the size, then the letters in rows
//! that fit a width. The groups are lettering objects, so they sew as the
//! lettering of the user does.

use emb_geom::{BBox, Point, Transform};
use emb_model::{LetterStitch, Lettering, ObjectKind};

use crate::layout::relayout;
use crate::source::LetterSource;

/// The capital height of the labels, in mm, if the label font goes down
/// to it ([`label_height`]).
pub const LABEL_MM: f64 = 5.0;
/// The bundled embroidery font of the labels of the font test sheets and
/// the test swatches: a font for small text with open shapes. Atkinson
/// Hyperlegible (made from its bold weight) closed the holes of its digits
/// at 5 mm (the agent test of P22 Milestone 9).
pub const LABEL_FONT: &str = "Soft Round";

/// The capital height of the labels in the font `src`: [`LABEL_MM`], or
/// the smallest size of its satin columns if that is larger.
pub fn label_height(src: &dyn LetterSource) -> f64 {
    src.size_range(LetterStitch::Satin)
        .map_or(LABEL_MM, |(min, _)| min.max(LABEL_MM))
}
/// The space below a label, in mm.
const LABEL_GAP_MM: f64 = 3.0;
/// The space between two letters of a row, as a part of the size.
const LETTER_GAP: f64 = 0.2;
/// The space between two groups, as a part of the size of the first one,
/// and its smallest value in mm.
const GROUP_GAP: f64 = 0.5;
const MIN_GROUP_GAP_MM: f64 = 6.0;
/// The sizes of a font with no size range, in mm.
const NO_RANGE: [f64; 3] = [8.0, 15.0, 25.0];

pub const CAPITALS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
pub const DIGITS: &str = "0123456789";
pub const MARKS: &str = ".,:;!?'\"&@#%()+-/*=";

/// The characters of a test sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Characters {
    /// The capital and small letters, the digits and the common marks.
    #[default]
    Basic,
    /// All the characters that a converted font has.
    All,
    Capitals,
    Lowercase,
    /// The digits and the common marks.
    Digits,
    /// The letters with accents and the other characters after the basic set.
    Accented,
    /// The text of the user; its lines stay lines.
    Text(String),
}

impl Characters {
    /// The characters of a set, in the order of the sheet (not for
    /// [`Characters::Text`]).
    fn set(&self) -> Vec<char> {
        let basic = || [CAPITALS, LOWERCASE, DIGITS, MARKS].concat();
        match self {
            Characters::Basic => basic().chars().collect(),
            Characters::Capitals => CAPITALS.chars().collect(),
            Characters::Lowercase => LOWERCASE.chars().collect(),
            Characters::Digits => [DIGITS, MARKS].concat().chars().collect(),
            Characters::All => {
                let mut v: Vec<char> = basic().chars().collect();
                for c in crate::embfont::default_chars() {
                    if !v.contains(&c) {
                        v.push(c);
                    }
                }
                v
            }
            Characters::Accented => {
                let basic = basic();
                crate::embfont::default_chars()
                    .into_iter()
                    .filter(|c| !basic.contains(*c))
                    .collect()
            }
            Characters::Text(_) => Vec::new(),
        }
    }
}

/// The characters of `set`, in the order of a sheet (the characters of a
/// text in the order of the text, each one time).
pub fn characters(set: &Characters) -> Vec<char> {
    match set {
        Characters::Text(text) => {
            let mut v: Vec<char> = Vec::new();
            for c in text.chars().filter(|c| !c.is_whitespace()) {
                if !v.contains(&c) {
                    v.push(c);
                }
            }
            v
        }
        set => set.set(),
    }
}

/// The smallest, the middle and the largest size of a font with the size
/// range `range`, in mm. The middle is the size with the same ratio to
/// both ends (for 6 to 54 mm: 18 mm), rounded to 0.5 mm.
pub fn sizes(range: Option<(f64, f64)>) -> [f64; 3] {
    match range {
        Some((lo, hi)) if lo > 0.0 && hi >= lo => {
            let mid = ((lo * hi).sqrt() * 2.0).round() / 2.0;
            [lo, mid.clamp(lo, hi), hi]
        }
        _ => NO_RANGE,
    }
}

/// A size as text: `7 mm`, `7.5 mm`.
pub fn size_text(mm: f64) -> String {
    let s = format!("{mm:.1}");
    format!("{} mm", s.strip_suffix(".0").unwrap_or(&s))
}

/// What a test sheet has.
#[derive(Debug, Clone)]
pub struct Spec<'a> {
    /// The name in the labels.
    pub name: &'a str,
    /// The capital heights, in mm; one group for each.
    pub sizes: &'a [f64],
    pub characters: &'a Characters,
    /// The rows are not wider than this, in mm (a letter that is wider is
    /// alone in its row).
    pub width_mm: f64,
}

/// One group of a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub size_mm: f64,
    /// The area of the label and the letters.
    pub bbox: BBox,
    /// The number of letters (the label not counted).
    pub letters: usize,
}

/// A test sheet: lettering objects from the top down, the top left corner
/// at the origin.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub objects: Vec<ObjectKind>,
    pub groups: Vec<Group>,
}

impl Sheet {
    /// The area of the sheet.
    pub fn bbox(&self) -> Option<BBox> {
        self.groups.iter().map(|g| g.bbox).reduce(BBox::union)
    }

    pub fn letters(&self) -> usize {
        self.groups.iter().map(|g| g.letters).sum()
    }

    /// The sheet as a new design named `name`, with one black thread and
    /// the middle of the sheet at the origin (the middle of the hoop).
    pub fn into_design(self, name: &str) -> emb_model::Design {
        let mut d = emb_model::Design::new(name);
        let thread = d.add_thread("Black", emb_model::Rgb(0, 0, 0));
        let to_middle = self.bbox().map_or(Transform::IDENTITY, |b| {
            Transform::translate(-b.center().x, -b.center().y)
        });
        for mut kind in self.objects {
            kind.transform(&to_middle);
            d.add_object(thread, kind);
        }
        d
    }
}

fn area(l: &Lettering) -> Option<BBox> {
    l.parts().filter_map(|p| p.control_bbox()).reduce(BBox::union)
}

/// The font has a letter with stitches for `c`.
fn has(src: &dyn LetterSource, c: char, scale: f64) -> bool {
    src.letter(c)
        .is_some_and(|g| !src.parts(g, LetterStitch::Auto, scale).is_empty())
}

/// The rows of one group as a text. `tokens` stay whole where they fit;
/// `joiner` is between two tokens of a row.
fn rows(src: &dyn LetterSource, tokens: &[String], joiner: &str, scale: f64, gap: f64, width: f64) -> Vec<String> {
    let advance = |c: char| match src.letter(c) {
        Some(g) => src.advance(g) * scale + gap,
        None if c.is_whitespace() => src.space() * scale + gap,
        None => 0.0,
    };
    let measure = |s: &str| s.chars().map(advance).sum::<f64>();
    let join = measure(joiner);
    let mut out: Vec<String> = Vec::new();
    let mut row = String::new();
    let mut x = 0.0;
    for token in tokens {
        let w = measure(token);
        if !row.is_empty() && x + join + w - gap > width {
            out.push(std::mem::take(&mut row));
            x = 0.0;
        }
        if w - gap > width {
            // A token that is wider than a row: letter by letter.
            for c in token.chars() {
                let a = advance(c);
                if !row.is_empty() && x + a - gap > width {
                    out.push(std::mem::take(&mut row));
                    x = 0.0;
                }
                row.push(c);
                x += a;
            }
            continue;
        }
        if !row.is_empty() {
            row.push_str(joiner);
            x += join;
        }
        row.push_str(token);
        x += w;
    }
    if !row.is_empty() {
        out.push(row);
    }
    out
}

/// The text of one group (rows with line breaks) and the space between
/// its letters, for the size with `scale`.
fn group_text(src: &dyn LetterSource, characters: &Characters, size_mm: f64, scale: f64, width: f64) -> (String, f64) {
    match characters {
        Characters::Text(text) => {
            let lines: Vec<String> = text
                .lines()
                .flat_map(|line| {
                    let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
                    rows(src, &words, " ", scale, 0.0, width)
                })
                .collect();
            (lines.join("\n"), 0.0)
        }
        set => {
            let gap = LETTER_GAP * size_mm;
            let tokens: Vec<String> = set
                .set()
                .into_iter()
                .filter(|&c| has(src, c, scale))
                .map(String::from)
                .collect();
            (rows(src, &tokens, "", scale, gap, width).join("\n"), gap)
        }
    }
}

/// The test sheet of the font `src`. `label` is the font of the labels
/// (`None`: no labels). A size with no letters has no group.
pub fn sheet(src: &dyn LetterSource, label: Option<&dyn LetterSource>, spec: &Spec) -> Sheet {
    let mut out = Sheet::default();
    let mut y = 0.0;
    // Puts `l` with the top left corner of its area at (0, y). Returns the
    // area.
    let place =
        |mut l: Lettering, src: &dyn LetterSource, y: f64, objects: &mut Vec<ObjectKind>| -> Option<(BBox, usize)> {
            relayout(&mut l, src);
            let b = area(&l)?;
            let letters = l.letters.len();
            let mut kind = ObjectKind::Lettering(Box::new(l));
            kind.transform(&Transform::translate(-b.min.x, y - b.min.y));
            objects.push(kind);
            Some((
                BBox::new(Point::new(0.0, y), Point::new(b.width(), y + b.height())),
                letters,
            ))
        };
    for &size in spec.sizes {
        if !(size.is_finite() && size > 0.0) {
            continue;
        }
        let scale = size / src.cap_height();
        let (text, gap) = group_text(src, spec.characters, size, scale, spec.width_mm.max(size));
        if text.trim().is_empty() {
            continue;
        }
        // The objects of this group: they go in the sheet only if the
        // group has letters.
        let mut objects = Vec::new();
        let mut top = y;
        let mut bbox: Option<BBox> = None;
        if let Some(label_src) = label {
            let text = format!("{} {}", spec.name, size_text(size));
            let l = Lettering::new(&text, label_src.font_ref(), label_height(label_src), Point::ZERO);
            if let Some((b, _)) = place(l, label_src, top, &mut objects) {
                top = b.max.y + LABEL_GAP_MM;
                bbox = Some(b);
            }
        }
        let mut l = Lettering::new(&text, src.font_ref(), size, Point::ZERO);
        l.letter_spacing_mm = gap;
        let Some((b, letters)) = place(l, src, top, &mut objects) else {
            continue;
        };
        out.objects.extend(objects);
        y = b.max.y + (GROUP_GAP * size).max(MIN_GROUP_GAP_MM);
        out.groups.push(Group {
            size_mm: size,
            bbox: bbox.map_or(b, |x| x.union(b)),
            letters,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled;

    #[test]
    fn the_sizes_of_a_range() {
        assert_eq!(sizes(Some((6.0, 54.0))), [6.0, 18.0, 54.0]);
        assert_eq!(sizes(Some((7.5, 7.5))), [7.5, 7.5, 7.5]);
        assert_eq!(sizes(None), NO_RANGE);
        assert_eq!(size_text(7.0), "7 mm");
        assert_eq!(size_text(7.5), "7.5 mm");
    }

    #[test]
    fn a_sheet_has_each_letter_at_each_size_in_rows_that_fit() {
        let font = bundled::embroidery("Block Sans").unwrap();
        let label = bundled::embroidery(LABEL_FONT).unwrap();
        let range = LetterSource::size_range(&font, LetterStitch::Auto);
        let all = sizes(range);
        let spec = Spec {
            name: "Block Sans",
            sizes: &all[..2],
            characters: &Characters::Basic,
            width_mm: 150.0,
        };
        let s = sheet(&font, Some(&label), &spec);
        assert_eq!(s.groups.len(), 2);
        // A label and the letters for each size.
        assert_eq!(s.objects.len(), 4);
        let basic = CAPITALS.len() + LOWERCASE.len() + DIGITS.len() + MARKS.len();
        for g in &s.groups {
            assert_eq!(g.letters, basic, "all the basic characters at {}", g.size_mm);
            assert!(g.bbox.width() <= 150.0 + 1e-6, "{:?}", g.bbox);
        }
        // The groups go down the sheet and do not touch.
        assert!(s.groups[1].bbox.min.y > s.groups[0].bbox.max.y + 1.0);
        let b = s.bbox().unwrap();
        assert!(b.min.x.abs() < 1e-6 && b.min.y.abs() < 1e-6, "{b:?}");
        // The objects are where the groups say.
        let ObjectKind::Lettering(l) = &s.objects[1] else {
            panic!("lettering")
        };
        assert_eq!(l.height_mm, all[0]);
        let a = area(l).unwrap();
        assert!(
            a.min.x.abs() < 1e-6 && a.max.y <= s.groups[0].bbox.max.y + 1e-6,
            "{a:?}"
        );
        assert!(l.text.lines().count() > 1, "more than one row");
        // The label has the name and the size.
        let ObjectKind::Lettering(l) = &s.objects[0] else {
            panic!("lettering")
        };
        assert_eq!(l.text, format!("Block Sans {}", size_text(all[0])));
        assert_eq!(s.letters(), 2 * basic);
    }

    #[test]
    fn the_character_sets_and_the_text_of_the_user() {
        let font = bundled::embroidery("Block Sans").unwrap();
        let count = |c: Characters, width: f64| {
            let spec = Spec {
                name: "x",
                sizes: &[10.0],
                characters: &c,
                width_mm: width,
            };
            let s = sheet(&font, None, &spec);
            assert!(s.objects.len() <= 1);
            let rows = match s.objects.first() {
                Some(ObjectKind::Lettering(l)) => l.text.lines().count(),
                _ => 0,
            };
            (s.letters(), rows)
        };
        assert_eq!(count(Characters::Capitals, 400.0), (26, 1));
        assert_eq!(count(Characters::Lowercase, 400.0).0, 26);
        assert_eq!(count(Characters::Digits, 400.0).0, DIGITS.len() + MARKS.len());
        let (accented, _) = count(Characters::Accented, 400.0);
        let (all, _) = count(Characters::All, 400.0);
        assert!(accented > 60, "{accented}");
        assert_eq!(all, accented + count(Characters::Basic, 400.0).0);
        // The words of the user stay whole where they fit; a line break of
        // the user stays.
        let text = Characters::Text("Hamburg fonts\nSewn".into());
        assert_eq!(count(text.clone(), 400.0), (16, 2));
        assert_eq!(count(text, 60.0), (16, 3));
        // A letter that is wider than the width is alone in its row.
        assert_eq!(count(Characters::Capitals, 1.0), (26, 26));
        // No characters: no group.
        assert_eq!(count(Characters::Text("  ".into()), 100.0), (0, 0));
    }
}
