//! Embroidery fonts (`.embfont`, ADR 0016): letters stored as embroidery
//! objects, with kerning and a recommended size range. A TrueType font
//! converts to one with [`convert`].

use std::collections::{BTreeMap, HashMap};

use emb_geom::Transform;
use emb_model::{FontRef, LetterStitch, ObjectKind};
use serde::{Deserialize, Serialize};

use crate::font::Font;
use crate::glyph::AUTO_MIN_COVERAGE;
use crate::source::LetterSource;

pub const FILE_EXTENSION: &str = "embfont";
const FORMAT: &str = "embfont";
const VERSION: u32 = 1;
/// The capital height at which the converter stores the letters, in mm.
pub const DESIGN_CAP_MM: f64 = 25.0;
/// Coordinates are rounded to this, in mm, to keep the files small.
const GRID_MM: f64 = 0.01;

/// The characters that [`convert`] takes from a font: printable ASCII,
/// the Latin-1 letters and signs, and common typographic marks.
pub fn default_chars() -> Vec<char> {
    let mut v: Vec<char> = ('!'..='~').collect();
    v.extend(('\u{A1}'..='\u{FF}').filter(|&c| c != '\u{AD}'));
    v.extend("–—‘’‚“”„…•€".chars());
    v
}

#[derive(Debug, thiserror::Error)]
pub enum EmbFontError {
    #[error("not an embroidery font: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("embroidery font version {0} is newer than this program reads ({VERSION})")]
    Version(u32),
    #[error("not an embroidery font file")]
    Format,
}

/// One letter of an embroidery font.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbLetter {
    pub advance_mm: f64,
    /// Satin and fill parts in mm at the design size, baseline at Y = 0.
    pub parts: Vec<ObjectKind>,
}

/// An embroidery font.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbFont {
    pub format: String,
    pub version: u32,
    pub family: String,
    #[serde(default)]
    pub style: String,
    /// Where the font comes from: source font, copyright, license.
    #[serde(default)]
    pub notes: String,
    /// The capital height of the stored letters.
    pub cap_height_mm: f64,
    pub line_height_mm: f64,
    pub space_mm: f64,
    pub min_height_mm: f64,
    pub max_height_mm: f64,
    /// The letters by character.
    pub letters: BTreeMap<char, EmbLetter>,
    /// Kerning of pairs of characters (two characters as the key), in mm at
    /// the design size.
    #[serde(default)]
    pub kerning: BTreeMap<String, f64>,
    /// A font for small text (letters 4 to 6 mm high, P11 Milestone 2).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub small_text: bool,
    #[serde(skip)]
    kern: HashMap<(char, char), f64>,
}

impl EmbFont {
    pub fn from_json(s: &str) -> Result<EmbFont, EmbFontError> {
        let mut f: EmbFont = serde_json::from_str(s)?;
        if f.format != FORMAT {
            return Err(EmbFontError::Format);
        }
        if f.version > VERSION {
            return Err(EmbFontError::Version(f.version));
        }
        f.index();
        Ok(f)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("an embroidery font serializes")
    }

    pub fn load(path: &std::path::Path) -> Result<EmbFont, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        EmbFont::from_json(&s).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.to_json()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// An alphabet with no letters yet (only the space), for letters made
    /// by hand (P4 Milestone 7).
    pub fn empty(family: &str, style: &str) -> EmbFont {
        let mut letters = BTreeMap::new();
        letters.insert(
            ' ',
            EmbLetter {
                advance_mm: round(0.3 * DESIGN_CAP_MM),
                parts: Vec::new(),
            },
        );
        EmbFont {
            format: FORMAT.into(),
            version: VERSION,
            family: family.to_string(),
            style: style.to_string(),
            notes: String::new(),
            cap_height_mm: DESIGN_CAP_MM,
            line_height_mm: round(1.4 * DESIGN_CAP_MM),
            space_mm: round(0.3 * DESIGN_CAP_MM),
            min_height_mm: crate::source::MIN_HEIGHT_MM,
            max_height_mm: crate::source::MAX_HEIGHT_MM,
            letters,
            kerning: BTreeMap::new(),
            small_text: false,
            kern: HashMap::new(),
        }
    }

    /// Sets the letter of `c`, with its coordinates rounded as in the file.
    pub fn set_letter(&mut self, c: char, mut letter: EmbLetter) {
        for p in &mut letter.parts {
            round_part(p);
        }
        letter.advance_mm = round(letter.advance_mm);
        self.letters.insert(c, letter);
    }

    /// Removes the letter of `c` and its kerning pairs.
    pub fn remove_letter(&mut self, c: char) {
        self.letters.remove(&c);
        self.kerning.retain(|k, _| !k.contains(c));
        self.index();
    }

    fn index(&mut self) {
        self.kern = self
            .kerning
            .iter()
            .filter_map(|(k, v)| {
                let mut c = k.chars();
                Some(((c.next()?, c.next()?), *v))
            })
            .collect();
    }
}

impl LetterSource for EmbFont {
    fn font_ref(&self) -> FontRef {
        FontRef::embroidery(&self.family, &self.style)
    }

    fn cap_height(&self) -> f64 {
        self.cap_height_mm
    }

    fn line_height(&self) -> f64 {
        self.line_height_mm
    }

    fn space(&self) -> f64 {
        self.space_mm
    }

    fn letter(&self, c: char) -> Option<u32> {
        self.letters.contains_key(&c).then_some(c as u32)
    }

    fn advance(&self, letter: u32) -> f64 {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .map_or(0.0, |l| l.advance_mm)
    }

    fn kerning(&self, left: u32, right: u32) -> f64 {
        match (char::from_u32(left), char::from_u32(right)) {
            (Some(a), Some(b)) => self.kern.get(&(a, b)).copied().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// The stored parts; an embroidery font keeps its own stitch types.
    fn parts(&self, letter: u32, _stitch: LetterStitch, _scale: f64) -> Vec<ObjectKind> {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .map(|l| l.parts.clone())
            .unwrap_or_default()
    }

    fn size_range(&self, _stitch: LetterStitch) -> Option<(f64, f64)> {
        Some((self.min_height_mm, self.max_height_mm))
    }
}

fn round(v: f64) -> f64 {
    // Divide by the whole number of steps in a mm: `x * 0.01` gives numbers
    // such as 3.3200000000000003, which make the file larger.
    let steps = (1.0 / GRID_MM).round();
    (v * steps).round() / steps
}

/// A letter from objects (satin, fill and running parts; lettering gives its
/// parts) drawn with the baseline at `baseline_y` and the capital height
/// `height_mm`, in design coordinates. The letter starts at the left of the
/// objects. `None` if there are no such parts.
pub fn letter_from_objects(objects: &[ObjectKind], baseline_y: f64, height_mm: f64) -> Option<EmbLetter> {
    let mut parts: Vec<ObjectKind> = Vec::new();
    for o in objects {
        match o {
            ObjectKind::Satin { .. } | ObjectKind::Fill { .. } | ObjectKind::Running { .. } => parts.push(o.clone()),
            ObjectKind::Lettering(l) => parts.extend(l.parts().cloned()),
            _ => {}
        }
    }
    let b = parts
        .iter()
        .filter_map(|p| p.control_bbox())
        .reduce(emb_geom::BBox::union)?;
    let s = DESIGN_CAP_MM / height_mm.max(1e-6);
    let t = Transform::translate(-b.min.x, -baseline_y).then(Transform::scale(s, s));
    for p in &mut parts {
        p.transform(&t);
    }
    Some(EmbLetter {
        advance_mm: b.width() * s + 0.08 * DESIGN_CAP_MM,
        parts,
    })
}

/// Rounds all coordinates of a part to [`GRID_MM`].
pub(crate) fn round_part(kind: &mut ObjectKind) {
    let r = |p: &mut emb_geom::Point| {
        p.x = round(p.x);
        p.y = round(p.y);
    };
    for path in kind.paths_mut() {
        for cmd in &mut path.cmds {
            match cmd {
                emb_geom::path::PathCmd::MoveTo(p) | emb_geom::path::PathCmd::LineTo(p) => r(p),
                emb_geom::path::PathCmd::CubicTo(a, b, p) => {
                    r(a);
                    r(b);
                    r(p);
                }
                emb_geom::path::PathCmd::Close => {}
            }
        }
    }
    if let ObjectKind::Satin { geometry, .. } = kind {
        for (a, b) in &mut geometry.rungs {
            r(a);
            r(b);
        }
    }
}

/// Converts a TrueType font to an embroidery font with the letters of
/// `chars`. `Auto` makes satin letters where the strokes cover the glyph,
/// else fill (the size range keeps satin columns narrow enough).
pub fn convert(font: &Font, family: &str, style: &str, stitch: LetterStitch, chars: &[char]) -> EmbFont {
    convert_with(font, family, style, stitch, chars, false)
}

/// [`convert`], and with `small_text` a font for small text: its size range
/// starts at 4 mm and at satin columns 0.8 mm wide.
pub fn convert_with(
    font: &Font,
    family: &str,
    style: &str,
    stitch: LetterStitch,
    chars: &[char],
    small_text: bool,
) -> EmbFont {
    let scale = DESIGN_CAP_MM / font.cap_height;
    let t = Transform::scale(scale, scale);
    let mut letters = BTreeMap::new();
    let mut glyphs = Vec::new();
    // The space is a letter with no parts, so that its kerning pairs stay.
    for c in std::iter::once(' ').chain(chars.iter().copied().filter(|&c| c != ' ')) {
        let Some(g) = font.glyph(c) else { continue };
        let shapes = font.shapes(g);
        if shapes.is_empty() && c != ' ' {
            continue;
        }
        let satin = match stitch {
            LetterStitch::Satin => !shapes.strokes.satins.is_empty(),
            LetterStitch::Fill => false,
            LetterStitch::Auto => !shapes.strokes.satins.is_empty() && shapes.strokes.coverage >= AUTO_MIN_COVERAGE,
        };
        let mut parts = if satin {
            shapes.satin_parts()
        } else {
            shapes.fill_parts()
        };
        for p in &mut parts {
            p.transform(&t);
            round_part(p);
        }
        letters.insert(
            c,
            EmbLetter {
                advance_mm: round(font.advance(g) * scale),
                parts,
            },
        );
        glyphs.push((c, g));
    }
    let mut kerning = BTreeMap::new();
    for &(a, ga) in &glyphs {
        for &(b, gb) in &glyphs {
            let k = round(font.kerning(ga, gb) * scale);
            if k != 0.0 {
                kerning.insert(format!("{a}{b}"), k);
            }
        }
    }
    // The stored letters cannot change to fill at large sizes, so the range
    // of satin letters is the satin range.
    let range = if stitch == LetterStitch::Fill {
        LetterStitch::Fill
    } else {
        LetterStitch::Satin
    };
    let sized = if small_text {
        font.stroke_ratio()
            .and_then(|r| crate::source::small_text_size_range(r, range))
    } else {
        LetterSource::size_range(font, range)
    };
    let (min, max) = sized.unwrap_or((source_min(), source_max()));
    let mut notes = format!("Converted from {} {}.", font.family, font.style);
    if !family.eq_ignore_ascii_case(&font.family) {
        notes.push_str(&format!(" A Modified Version with a new name ({family})."));
    }
    for extra in [font.copyright(), font.license()].into_iter().flatten() {
        notes.push(' ');
        notes.push_str(extra.trim());
    }
    let mut f = EmbFont {
        format: FORMAT.into(),
        version: VERSION,
        family: family.to_string(),
        style: style.to_string(),
        notes,
        cap_height_mm: DESIGN_CAP_MM,
        line_height_mm: round(font.line_height() * scale),
        space_mm: round(LetterSource::space(font) * scale),
        min_height_mm: min,
        max_height_mm: max,
        letters,
        kerning,
        small_text,
        kern: HashMap::new(),
    };
    f.index();
    f
}

fn source_min() -> f64 {
    crate::source::MIN_HEIGHT_MM
}

fn source_max() -> f64 {
    crate::source::MAX_HEIGHT_MM
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::Lettering;

    use super::*;
    use crate::{bundled, layout};

    #[test]
    fn convert_save_load_and_layout() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let e = convert(
            &f,
            "Block Sans",
            "",
            LetterStitch::Auto,
            &"AVHno ".chars().collect::<Vec<_>>(),
        );
        assert_eq!(e.letters.len(), 6);
        assert!(e.letters[&' '].parts.is_empty());
        assert!(e.kerning.get("AV").is_some_and(|k| *k < 0.0));
        assert!(e.min_height_mm >= 5.0 && e.min_height_mm < 10.0, "{}", e.min_height_mm);
        assert!(e.notes.contains("Converted from Lato Bold"));
        let back = EmbFont::from_json(&e.to_json()).unwrap();
        assert_eq!(back.letters, e.letters);
        assert!(back.kerning(u32::from('A'), u32::from('V')) < 0.0);

        // The same text with the TrueType font and the embroidery font
        // gives letters of the same size at the same places.
        let mut l = Lettering::new("HAV no", f.font_ref(), 20.0, Point::new(0.0, 0.0));
        let a = layout::layout(&l, &f);
        l.font = back.font_ref();
        let b = layout::layout(&l, &back);
        assert_eq!(a.len(), b.len());
        let bb = |x: &emb_model::Letter| {
            x.parts
                .iter()
                .filter_map(ObjectKind::control_bbox)
                .reduce(emb_geom::BBox::union)
                .unwrap()
        };
        for (x, y) in a.iter().zip(&b) {
            assert!(
                bb(x).min.distance(bb(y).min) < 0.05,
                "{} {:?} {:?}",
                x.text,
                bb(x),
                bb(y)
            );
        }
    }

    #[test]
    fn bad_files() {
        assert!(matches!(EmbFont::from_json("{}"), Err(EmbFontError::Parse(_))));
        let mut e = convert(
            &bundled::font("Lato", "Bold").unwrap(),
            "X",
            "",
            LetterStitch::Fill,
            &['I'],
        );
        e.version = 99;
        assert!(matches!(
            EmbFont::from_json(&e.to_json()),
            Err(EmbFontError::Version(99))
        ));
        e.version = 1;
        e.format = "other".into();
        assert!(matches!(EmbFont::from_json(&e.to_json()), Err(EmbFontError::Format)));
        assert!(default_chars().contains(&'é') && default_chars().contains(&'€'));
    }
}
