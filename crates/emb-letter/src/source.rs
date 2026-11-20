//! Where layout gets its letters: a TrueType font or an embroidery font.

use std::sync::Arc;

use emb_model::{FontRef, LetterStitch, ObjectKind, PartTuning};

use crate::embfont::EmbFont;
use crate::font::Font;
use crate::shape::ShapedGlyph;

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
    /// The height of the Japanese, Chinese or Korean characters of the
    /// source (the middle height of some common kanji), if it has them
    /// (P20 Milestone 5).
    fn character_height(&self) -> Option<f64> {
        None
    }
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
    /// The recommended height range in mm for lettering of `text` whose
    /// height is the height of its characters (`Lettering::character_height`):
    /// [`LetterSource::size_range`], and for text with Japanese, Chinese or
    /// Korean characters in a TrueType font, a range from the narrowest
    /// columns of those characters (P20 Milestone 5).
    fn text_size_range(&self, text: &str, stitch: LetterStitch) -> Option<(f64, f64)> {
        let _ = text;
        self.size_range(stitch)
    }
    /// The parts of the letter sew in the order of the source, each column
    /// from its start to its end (a letter that a person put in order).
    /// `stitch` and `scale` are those of [`LetterSource::parts`]: the
    /// answer is for the parts that it gives.
    fn fixed(&self, _letter: u32, _stitch: LetterStitch, _scale: f64) -> bool {
        false
    }
    /// The settings that the source gives to each part of the letter, in
    /// the order of the parts (empty: none), for the parts that
    /// [`LetterSource::parts`] gives with `stitch` and `scale`.
    fn tuning(&self, _letter: u32, _stitch: LetterStitch, _scale: f64) -> Vec<PartTuning> {
        Vec::new()
    }
    /// The glyphs of one line of text, shaped (see [`crate::shape`]): the
    /// letters of the source for the line, with ligatures, the forms that
    /// depend on the place of a letter, marks, and text from right to
    /// left. `None`: the source has a letter for each character, and the
    /// layout uses [`LetterSource::letter`] and [`LetterSource::kerning`].
    fn shape(&self, _line: &str, _kerning: bool) -> Option<Vec<ShapedGlyph>> {
        None
    }
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

    fn character_height(&self) -> Option<f64> {
        self.character_height
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

    /// The columns of the letter in the order of their route (P20
    /// Milestone 4, [`crate::font::RouteMode`]) when it has one, else as
    /// the stroke finder gives them; or its fills.
    fn parts(&self, letter: u32, stitch: LetterStitch, _scale: f64) -> Vec<ObjectKind> {
        let glyph = letter as u16;
        let shapes = self.shapes(glyph);
        if !columns(&shapes, stitch) {
            return shapes.fill_parts();
        }
        match self.route(glyph) {
            Some(routed) => (*routed).clone(),
            None => shapes.satin_parts(),
        }
    }

    /// A letter with its route sews in that order.
    fn fixed(&self, letter: u32, stitch: LetterStitch, _scale: f64) -> bool {
        let glyph = letter as u16;
        columns(&self.shapes(glyph), stitch) && self.has_route(glyph)
    }

    fn size_range(&self, stitch: LetterStitch) -> Option<(f64, f64)> {
        size_range(self.stroke_ratio()?, stitch)
    }

    /// For text with Japanese, Chinese or Korean characters: the smallest
    /// height at which the columns of these characters are at least
    /// [`SMALL_TEXT_MIN_SATIN_MM`] wide, all but the narrowest
    /// [`crate::quality::RANGE_TAIL`] of their length (narrower columns sew
    /// as runs, `glyph::auto_stitches`); the largest height of
    /// [`LetterSource::size_range`].
    fn text_size_range(&self, text: &str, stitch: LetterStitch) -> Option<(f64, f64)> {
        let range = self.size_range(stitch);
        let Some(h) = self.character_height else {
            return range;
        };
        if !text.chars().any(crate::layout::is_cjk) {
            return range;
        }
        let mut seen = Vec::new();
        let mut widths: Vec<(f64, f64)> = Vec::new();
        for c in text.chars().filter(|c| crate::layout::is_cjk(*c)) {
            if seen.contains(&c) {
                continue;
            }
            seen.push(c);
            let Some(g) = self.glyph(c) else { continue };
            let shapes = self.shapes(g);
            if !columns(&shapes, stitch) {
                continue;
            }
            for s in &shapes.strokes.satins {
                widths.extend(crate::quality::column_widths(s, h / 200.0));
            }
        }
        widths.retain(|(w, l)| w.is_finite() && *w > 0.0 && *l > 0.0);
        if widths.is_empty() {
            return range;
        }
        widths.sort_by(|a, b| a.0.total_cmp(&b.0));
        let tail = crate::quality::RANGE_TAIL * widths.iter().map(|(_, l)| l).sum::<f64>();
        let mut sum = 0.0;
        let narrow = widths
            .iter()
            .find(|(_, l)| {
                sum += l;
                sum > tail
            })
            .map_or(widths[0].0, |(w, _)| *w);
        let lo = ((SMALL_TEXT_MIN_SATIN_MM * h / narrow * 2.0).ceil() / 2.0).max(MIN_HEIGHT_MM);
        let hi = range.map_or(MAX_HEIGHT_MM, |r| r.1).max(lo);
        Some((lo, hi))
    }

    fn shape(&self, line: &str, kerning: bool) -> Option<Vec<ShapedGlyph>> {
        self.shape_line(line, kerning)
    }
}

/// `true` if the letter sews as columns with the stitch type `stitch`.
fn columns(shapes: &crate::glyph::GlyphShapes, stitch: LetterStitch) -> bool {
    match stitch {
        LetterStitch::Satin => !shapes.strokes.satins.is_empty(),
        LetterStitch::Fill => false,
        LetterStitch::Auto => shapes.columns_cover(),
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
