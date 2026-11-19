//! Embroidery fonts (`.embfont`, ADR 0016): letters stored as embroidery
//! objects, with kerning and a recommended size range. A TrueType font
//! converts to one with [`convert`].

use std::collections::{BTreeMap, HashMap};

use emb_geom::{Transform, text};
use emb_model::{FontRef, LetterStitch, ObjectKind, PartTuning};
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
    #[error("{}", text::fill("not an embroidery font: {}", &[.0]))]
    Parse(#[from] serde_json::Error),
    #[error("{}", text::fill("embroidery font version {} is newer than this program reads ({})", &[.0, &VERSION]))]
    Version(u32),
    #[error("{}", text::tr("not an embroidery font file"))]
    Format,
}

/// One letter of an embroidery font.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbLetter {
    pub advance_mm: f64,
    /// Satin and fill parts in mm at the design size, baseline at Y = 0.
    pub parts: Vec<ObjectKind>,
    /// The parts sew in this order, each column from its start to its end
    /// (a person put them in order; ADR 0112).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
    /// The settings of each part, in the order of `parts` (empty: none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tuning: Vec<PartTuning>,
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
    /// A tuned font (P17 Milestone 5, ADR 0113): its letters are routed
    /// and each one was checked.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tuned: bool,
    /// A fill font (ADR 0117): the converter made all its letters as
    /// fills. It is for large text and for heavy fonts, whose letters
    /// have no clear columns. Its score and its size range come from the
    /// widths of the strokes and of the holes of its letters.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fill: bool,
    /// The quality score of the font (P17 Milestone 7, ADR 0115). The
    /// converter makes it, because it needs the font that this font was
    /// converted from. `None`: not measured (an alphabet made by hand, a
    /// bitmap font, a file of an earlier version).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<crate::quality::Quality>,
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
                fixed: false,
                tuning: Vec::new(),
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
            tuned: false,
            fill: false,
            quality: None,
            kern: HashMap::new(),
        }
    }

    /// Makes the size range fit the columns of the letters (ADR 0115). The
    /// range of the converter comes from the typical stroke of the font. A
    /// font with thin and thick strokes (a script, a serif font with
    /// hairlines) then has thin strokes that are too narrow at the smallest
    /// size, and thick strokes with stitches that are too long at the
    /// largest size. The range gets smaller until at most
    /// [`crate::quality::RANGE_TAIL`] of the length of the columns is too
    /// narrow at the smallest size, and at most that part is too long at
    /// the largest size. It never gets larger.
    ///
    /// The range of a fill font (ADR 0117) starts where at most
    /// [`crate::quality::FILL_TAIL`] of the area of its letters is
    /// narrower than [`crate::source::MIN_FILL_MM`] and at most
    /// [`crate::quality::RANGE_TAIL`] of its holes are narrower than
    /// [`crate::quality::MIN_HOLE_MM`], and not below
    /// [`crate::quality::FILL_MIN_HEIGHT_MM`].
    pub fn fit_size_range(&mut self) {
        use crate::quality::{LONG_MM, RANGE_TAIL, narrow_mm, width_at};
        if self.fill {
            use crate::quality::{FILL_MIN_HEIGHT_MM, FILL_TAIL, MIN_HOLE_MM, hole_width_at};
            let size = |limit: f64, width: Option<f64>| -> f64 {
                match width {
                    Some(w) if w > 0.0 => (limit / w * self.cap_height_mm * 2.0).ceil() / 2.0,
                    _ => 0.0,
                }
            };
            let lo = size(crate::source::MIN_FILL_MM, width_at(self, FILL_TAIL))
                .max(size(MIN_HOLE_MM, hole_width_at(self, RANGE_TAIL)))
                .max(FILL_MIN_HEIGHT_MM);
            self.min_height_mm = self.min_height_mm.max(lo);
            self.max_height_mm = self.max_height_mm.max(self.min_height_mm);
            return;
        }
        let (Some(thin), Some(wide)) = (width_at(self, RANGE_TAIL), width_at(self, 1.0 - RANGE_TAIL)) else {
            return;
        };
        if thin <= 0.0 || wide <= 0.0 {
            return;
        }
        let lo = (narrow_mm(self) / thin * self.cap_height_mm * 2.0).ceil() / 2.0;
        let hi = (LONG_MM / wide * self.cap_height_mm).floor();
        self.min_height_mm = self.min_height_mm.max(lo);
        self.max_height_mm = self.max_height_mm.min(hi).max(self.min_height_mm);
    }

    /// Measures the quality of the font again ([`crate::quality::measure`]),
    /// after a change of its letters. `source` is the font that this font
    /// was converted from.
    pub fn measure_quality(&mut self, source: &Font) {
        self.quality = crate::quality::measure(self, source);
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

    /// Makes the ends of the columns longer where the outline of the
    /// source font has a part with no stitches ([`crate::tune::extend_ends`]).
    /// `source` is the font that this font was converted from. Returns the
    /// number of ends that changed.
    pub fn extend_ends(&mut self, source: &Font) -> usize {
        let s = self.cap_height_mm / source.cap_height;
        let scale = |r: &Vec<emb_geom::Point>| -> Vec<emb_geom::Point> {
            r.iter().map(|p| emb_geom::Point::new(p.x * s, p.y * s)).collect()
        };
        let cap = self.cap_height_mm;
        let mut changed = 0;
        for (c, letter) in &mut self.letters {
            let Some(g) = source.glyph(*c) else { continue };
            let outline: Vec<_> = source
                .shapes(g)
                .fills
                .iter()
                .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect::<Vec<_>>()))
                .collect();
            let n = crate::tune::round_parts(&mut letter.parts, &outline, cap)
                + crate::tune::extend_ends(&mut letter.parts, &outline, cap);
            if n > 0 {
                for p in &mut letter.parts {
                    round_part(p);
                }
            }
            changed += n;
        }
        changed
    }

    /// Routes each letter that is not fixed ([`crate::route::route`]):
    /// the order and the directions of its parts with the fewest trims, and
    /// travel runs under the parts that sew later. A letter that a person
    /// put in order stays as it is. Returns the trims and the runs of the
    /// routed letters.
    pub fn route(&mut self) -> crate::route::Routed {
        let cap = self.cap_height_mm;
        let mut todo: Vec<&mut EmbLetter> = self
            .letters
            .values_mut()
            .filter(|l| !l.fixed && !l.parts.is_empty())
            .collect();
        // The letters do not depend on each other: one thread for each
        // group of them.
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(16);
        let size = todo.len().div_ceil(threads).max(1);
        let found: Vec<crate::route::Routed> = std::thread::scope(|scope| {
            let jobs: Vec<_> = todo
                .chunks_mut(size)
                .map(|group| {
                    scope.spawn(move || {
                        let mut sum = crate::route::Routed::default();
                        for letter in group.iter_mut() {
                            let (mut routed, found) = crate::route::route_with_splits(letter, cap);
                            for p in &mut routed.parts {
                                round_part(p);
                            }
                            **letter = routed;
                            sum.trims += found.trims;
                            sum.runs += found.runs;
                            sum.splits += found.splits;
                        }
                        sum
                    })
                })
                .collect();
            jobs.into_iter().filter_map(|j| j.join().ok()).collect()
        });
        found
            .into_iter()
            .fold(crate::route::Routed::default(), |a, b| crate::route::Routed {
                trims: a.trims + b.trims,
                runs: a.runs + b.runs,
                splits: a.splits + b.splits,
            })
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

    fn fixed(&self, letter: u32) -> bool {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .is_some_and(|l| l.fixed)
    }

    fn tuning(&self, letter: u32) -> Vec<PartTuning> {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .map(|l| l.tuning.clone())
            .unwrap_or_default()
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
    letter_from(
        objects,
        &LetterFrame {
            baseline_y,
            height_mm,
            origin_x: None,
            keep_order: false,
            keep_settings: false,
        },
    )
}

/// Where a letter is drawn in a design, and what a letter made from the
/// objects keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LetterFrame {
    /// The Y of the baseline, in design mm.
    pub baseline_y: f64,
    /// The capital height as drawn, in mm.
    pub height_mm: f64,
    /// The X where the letter starts (`None`: the left of the objects, and
    /// the letter gets a width from its objects).
    pub origin_x: Option<f64>,
    /// The parts sew in the order and the direction of the objects.
    pub keep_order: bool,
    /// The underlay, the pull compensation and the fill angle of each
    /// object, where they are not the standard settings of lettering.
    pub keep_settings: bool,
}

/// The settings of `parts` that are not the standard settings of lettering
/// (empty: all are standard).
pub fn tuning_of(parts: &[ObjectKind]) -> Vec<PartTuning> {
    let (satin, fill) = (
        emb_model::lettering::letter_satin(),
        emb_model::lettering::letter_fill(),
    );
    let own = |u: &emb_model::Underlay, standard: &emb_model::Underlay| (u != standard).then(|| u.clone());
    let tuning: Vec<PartTuning> = parts
        .iter()
        .map(|p| match p {
            ObjectKind::Satin { params, .. } => PartTuning {
                underlay: own(&params.underlay, &satin.underlay),
                pull_mm: round(params.pull_compensation_mm - satin.pull_compensation_mm),
                angle_deg: 0.0,
            },
            ObjectKind::Fill { params, .. } => PartTuning {
                underlay: own(&params.underlay, &fill.underlay),
                pull_mm: round(params.pull_compensation_mm - fill.pull_compensation_mm),
                angle_deg: round((params.angle_deg - fill.angle_deg).rem_euclid(180.0)),
            },
            _ => PartTuning::default(),
        })
        .collect();
    if tuning.iter().all(PartTuning::is_empty) {
        Vec::new()
    } else {
        tuning
    }
}

/// The parts of `letter` as objects in a design: the capital height
/// `height_mm`, the start of the letter on the baseline at `at`, with the
/// standard settings of lettering and the settings of the letter.
/// [`letter_from`] makes the letter from them again.
pub fn objects_of_letter(font: &EmbFont, letter: &EmbLetter, at: emb_geom::Point, height_mm: f64) -> Vec<ObjectKind> {
    let s = height_mm / font.cap_height_mm.max(1e-6);
    let t = Transform::scale(s, s).then(Transform::translate(at.x, at.y));
    let (satin, fill) = (
        emb_model::lettering::letter_satin(),
        emb_model::lettering::letter_fill(),
    );
    let tuned = letter.tuning.len() == letter.parts.len();
    letter
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut p = p.clone();
            p.transform(&t);
            let tune = tuned.then(|| &letter.tuning[i]);
            let pull = tune.map_or(0.0, |t| t.pull_mm);
            match &mut p {
                ObjectKind::Satin { params, .. } => {
                    *params = satin.clone();
                    params.pull_compensation_mm = (params.pull_compensation_mm + pull).max(0.0);
                    if let Some(u) = tune.and_then(|t| t.underlay.as_ref()) {
                        params.underlay = u.clone();
                    }
                }
                ObjectKind::Fill { params, .. } => {
                    *params = fill.clone();
                    params.angle_deg += tune.map_or(0.0, |t| t.angle_deg);
                    params.pull_compensation_mm = (params.pull_compensation_mm + pull).max(0.0);
                    if let Some(u) = tune.and_then(|t| t.underlay.as_ref()) {
                        params.underlay = u.clone();
                    }
                }
                _ => {}
            }
            p
        })
        .collect()
}

/// A letter from objects (see [`letter_from_objects`]) drawn in `frame`.
pub fn letter_from(objects: &[ObjectKind], frame: &LetterFrame) -> Option<EmbLetter> {
    let (baseline_y, height_mm) = (frame.baseline_y, frame.height_mm);
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
    let left = frame.origin_x.unwrap_or(b.min.x);
    let t = Transform::translate(-left, -baseline_y).then(Transform::scale(s, s));
    let tuning = if frame.keep_settings {
        tuning_of(&parts)
    } else {
        Vec::new()
    };
    for p in &mut parts {
        p.transform(&t);
    }
    Some(EmbLetter {
        advance_mm: (b.max.x - left).max(0.0) * s + 0.08 * DESIGN_CAP_MM,
        parts,
        fixed: frame.keep_order,
        tuning,
    })
}

/// Rounds all coordinates of a part to [`GRID_MM`].
pub fn round_part(kind: &mut ObjectKind) {
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
                fixed: false,
                tuning: Vec::new(),
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
        tuned: false,
        fill: stitch == LetterStitch::Fill,
        quality: None,
        kern: HashMap::new(),
    };
    f.index();
    f.fit_size_range();
    f.measure_quality(font);
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
