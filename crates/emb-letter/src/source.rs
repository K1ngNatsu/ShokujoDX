//! Where layout gets its letters: a TrueType font or an embroidery font.

use std::sync::Arc;

use emb_model::{FontRef, LetterStitch, ObjectKind};

use crate::embfont::EmbFont;
use crate::font::Font;

/// The narrowest satin column that sews well, in mm.
pub const MIN_SATIN_MM: f64 = 1.2;
/// The widest satin column of the recommended range, in mm.
pub const MAX_SATIN_MM: f64 = 10.0;
/// The narrowest fill stroke that sews well, in mm.
pub const MIN_FILL_MM: f64 = 2.0;
/// No recommended size is below this capital height, in mm.
pub const MIN_HEIGHT_MM: f64 = 5.0;
/// The narrowest satin column of a font for small text (P11 Milestone 2),
/// in mm: letters 4 to 6 mm high have columns 0.8 to 1.2 mm wide.
pub const SMALL_TEXT_MIN_SATIN_MM: f64 = 0.8;
/// The smallest capital height of a font for small text, in mm.
pub const SMALL_TEXT_MIN_HEIGHT_MM: f64 = 4.0;
/// The largest lettering in the recommended range when fill takes over
/// wide strokes, in mm.
pub const MAX_HEIGHT_MM: f64 = 300.0;

/// The letters of a font for layout. Units are the units of the source
/// (font units for TrueType, mm at the design size for embroidery fonts),
/// with the baseline at Y = 0 and Y down.
pub trait LetterSource {
    fn font_ref(&self) -> FontRef;
    fn cap_height(&self) -> f64;
    /// The distance between two baselines.
    fn line_height(&self) -> f64;
    /// The advance of a space when the source has no space letter.
    fn space(&self) -> f64;
    /// The letter for a character, if the source has one.
    fn letter(&self, c: char) -> Option<u32>;
    fn advance(&self, letter: u32) -> f64;
    fn kerning(&self, left: u32, right: u32) -> f64;
    /// The parts of a letter. `scale` (mm for each unit) and `stitch` choose
    /// between satin and fill where the source can.
    fn parts(&self, letter: u32, stitch: LetterStitch, scale: f64) -> Vec<ObjectKind>;
    /// The recommended capital height range in mm.
    fn size_range(&self, stitch: LetterStitch) -> Option<(f64, f64)>;
}

/// The recommended height range for strokes `ratio` times the capital
/// height wide (see ADR 0016).
pub fn size_range(ratio: f64, stitch: LetterStitch) -> Option<(f64, f64)> {
    size_range_from(ratio, stitch, MIN_SATIN_MM, MIN_HEIGHT_MM)
}

/// [`size_range`] for a font for small text: narrower satin columns and a
/// smaller capital height are in the range.
pub fn small_text_size_range(ratio: f64, stitch: LetterStitch) -> Option<(f64, f64)> {
    size_range_from(ratio, stitch, SMALL_TEXT_MIN_SATIN_MM, SMALL_TEXT_MIN_HEIGHT_MM)
}

fn size_range_from(ratio: f64, stitch: LetterStitch, min_satin: f64, min_height: f64) -> Option<(f64, f64)> {
    if !(ratio.is_finite() && ratio > 0.0) {
        return None;
    }
    let (min_stroke, max) = match stitch {
        LetterStitch::Satin => (min_satin, MAX_SATIN_MM / ratio),
        LetterStitch::Auto => (min_satin, MAX_HEIGHT_MM),
        LetterStitch::Fill => (MIN_FILL_MM, MAX_HEIGHT_MM),
    };
    let lo = ((min_stroke / ratio).max(min_height) * 2.0).ceil() / 2.0;
    let hi = max.floor().max(lo);
    Some((lo, hi))
}

impl LetterSource for Font {
    fn font_ref(&self) -> FontRef {
        Font::font_ref(self)
    }

    fn cap_height(&self) -> f64 {
        self.cap_height
    }

    fn line_height(&self) -> f64 {
        Font::line_height(self)
    }

    fn space(&self) -> f64 {
        self.glyph(' ')
            .map_or(0.25 * self.units_per_em, |g| Font::advance(self, g))
    }

    fn letter(&self, c: char) -> Option<u32> {
        self.glyph(c).map(u32::from)
    }

    fn advance(&self, letter: u32) -> f64 {
        Font::advance(self, letter as u16)
    }

    fn kerning(&self, left: u32, right: u32) -> f64 {
        Font::kerning(self, left as u16, right as u16)
    }

    fn parts(&self, letter: u32, stitch: LetterStitch, scale: f64) -> Vec<ObjectKind> {
        let shapes = self.shapes(letter as u16);
        let satin = match stitch {
            LetterStitch::Satin => !shapes.strokes.satins.is_empty(),
            LetterStitch::Fill => false,
            LetterStitch::Auto => shapes.satin_fits(scale),
        };
        if satin {
            shapes.satin_parts()
        } else {
            shapes.fill_parts()
        }
    }

    fn size_range(&self, stitch: LetterStitch) -> Option<(f64, f64)> {
        size_range(self.stroke_ratio()?, stitch)
    }
}

/// A TrueType font or an embroidery font.
#[derive(Debug, Clone)]
pub enum AnyFont {
    TrueType(Arc<Font>),
    Embroidery(Arc<EmbFont>),
}

impl AnyFont {
    pub fn source(&self) -> &dyn LetterSource {
        match self {
            AnyFont::TrueType(f) => f.as_ref(),
            AnyFont::Embroidery(f) => f.as_ref(),
        }
    }
}
