//! Embroidery fonts (`.embfont`, ADR 0016): letters stored as embroidery
//! objects, with kerning and a recommended size range. A TrueType font
//! converts to one with [`convert`].

use std::collections::{BTreeMap, HashMap};

use emb_geom::{Transform, text};
use emb_model::{FontRef, LetterStitch, ObjectKind, PartTuning};
use serde::{Deserialize, Serialize};

use crate::font::Font;
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
    /// A number of the file is not in its range; the text names it.
    #[error("{}", text::fill("the embroidery font has a wrong value: {}", &[.0]))]
    Invalid(String),
}

/// The largest size and the largest coordinate in a font file, in mm at
/// the design size of the font (the converter makes letters with a capital
/// height of [`DESIGN_CAP_MM`]). A file with a larger value is not a font
/// that the program made, and a very large value makes very large letters
/// (the audit of P17).
pub const MAX_FONT_MM: f64 = 1000.0;
/// The largest capital height of a size range, in mm.
pub const MAX_RANGE_MM: f64 = 10_000.0;

/// Checks one letter of a font file: its width and the coordinates of its
/// parts are numbers in the range of a font ([`MAX_FONT_MM`]), and each
/// part is a column, a fill or a run. `Err`: what is wrong.
pub fn check_letter(letter: &EmbLetter) -> Result<(), &'static str> {
    let ok = |p: &emb_geom::Point| {
        p.x.is_finite() && p.y.is_finite() && p.x.abs() <= MAX_FONT_MM && p.y.abs() <= MAX_FONT_MM
    };
    if !(letter.advance_mm.is_finite() && letter.advance_mm.abs() <= MAX_FONT_MM) {
        return Err("advance_mm");
    }
    if !letter.fill_parts.iter().all(|p| matches!(p, ObjectKind::Fill { .. })) {
        return Err("fill_parts");
    }
    for part in letter.parts.iter().chain(&letter.fill_parts) {
        let rungs: &[(emb_geom::Point, emb_geom::Point)] = match part {
            ObjectKind::Satin { geometry, .. } => &geometry.rungs,
            ObjectKind::Fill { .. } | ObjectKind::Running { .. } => &[],
            _ => return Err("a part that is not a column, a fill or a run"),
        };
        if !rungs.iter().all(|(a, b)| ok(a) && ok(b)) {
            return Err("parts");
        }
        let holes: &[emb_geom::Path] = match part {
            ObjectKind::Fill { holes, .. } => holes,
            _ => &[],
        };
        for path in part.paths().chain(holes) {
            if !path.points().all(|p| ok(&p)) {
                return Err("parts");
            }
            if let Some(b) = path.control_bbox()
                && !(ok(&b.min) && ok(&b.max))
            {
                return Err("parts");
            }
        }
    }
    for t in &letter.tuning {
        if !(t.pull_mm.is_finite() && t.pull_mm.abs() <= 10.0 && t.angle_deg.is_finite()) {
            return Err("tuning");
        }
    }
    Ok(())
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
    /// The letter as fills (ADR 0120): the outline of the source font, for
    /// the sizes at which the columns of `parts` are too wide
    /// ([`EmbFont::fill_letters`]). Empty: the letter has `parts` only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fill_parts: Vec<ObjectKind>,
}

/// The sizes of the fill letters of a font of satin letters (ADR 0120).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FillSizes {
    /// Above this capital height the letters sew as their fill letters
    /// ([`EmbLetter::fill_parts`]), in mm: the largest size of the columns.
    pub from_mm: f64,
    /// The smallest capital height at which the fill letters sew well, in
    /// mm (the smallest size of the fill font with the same letters,
    /// ADR 0117). It is for lettering with the stitch type Fill.
    pub min_mm: f64,
}

/// The fields of a font file that the font lists show
/// ([`EmbFont::load_header`]).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EmbFontHeader {
    pub format: String,
    pub version: u32,
    pub family: String,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub tuned: bool,
    #[serde(default)]
    pub fill: bool,
    #[serde(default)]
    pub quality: Option<crate::quality::Quality>,
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
    /// The font has the fill letter of each letter with columns too
    /// ([`EmbLetter::fill_parts`], ADR 0120): the layout uses the columns
    /// up to `from_mm` and the fills above it, so the font goes up to
    /// `max_height_mm` in one look. `None`: the letters have one shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_letters: Option<FillSizes>,
    /// The largest capital height at which the columns sew as satin, in a
    /// font with Japanese, Chinese or Korean letters (P20 Milestone 6, ADR
    /// 0146): above it, the stitch type Auto makes them tatami columns
    /// (ADR 0142, 0143), up to `max_height_mm`. `None`: the columns end at
    /// the fill letters or at `max_height_mm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub satin_max_mm: Option<f64>,
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
        f.validate().map_err(EmbFontError::Invalid)?;
        f.index();
        Ok(f)
    }

    /// Checks the numbers of a font file (the audit of P17): the sizes of
    /// the font, its size range, each letter ([`check_letter`]), the
    /// kerning and the quality. A font file can come from another person
    /// (a font package). `Err`: the name of the wrong value in the file.
    pub fn validate(&self) -> Result<(), String> {
        let size = |v: f64, least: f64, most: f64| v.is_finite() && v >= least && v <= most;
        if !size(self.cap_height_mm, 1.0, MAX_FONT_MM) {
            return Err("cap_height_mm".into());
        }
        if !size(self.line_height_mm, 0.01, MAX_FONT_MM) {
            return Err("line_height_mm".into());
        }
        if !size(self.space_mm, 0.0, MAX_FONT_MM) {
            return Err("space_mm".into());
        }
        if !size(self.min_height_mm, 0.1, MAX_RANGE_MM) {
            return Err("min_height_mm".into());
        }
        if !size(self.max_height_mm, self.min_height_mm, MAX_RANGE_MM) {
            return Err("max_height_mm".into());
        }
        if let Some(m) = self.satin_max_mm
            && !size(m, self.min_height_mm, self.max_height_mm)
        {
            return Err("satin_max_mm".into());
        }
        if let Some(f) = &self.fill_letters
            && !(size(f.from_mm, self.min_height_mm, self.max_height_mm) && size(f.min_mm, 0.1, MAX_RANGE_MM))
        {
            return Err("fill_letters".into());
        }
        for (c, letter) in &self.letters {
            check_letter(letter).map_err(|what| format!("letters.{c}: {what}"))?;
        }
        if let Some((pair, _)) = self
            .kerning
            .iter()
            .find(|(_, k)| !(k.is_finite() && k.abs() <= MAX_FONT_MM))
        {
            return Err(format!("kerning.{pair}"));
        }
        if let Some(q) = &self.quality {
            let part = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
            let parts = [q.coverage, q.narrow, q.long, q.fills, q.outside, q.overlap];
            if q.score > 100 || !parts.into_iter().all(part) {
                return Err("quality".into());
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("an embroidery font serializes")
    }

    pub fn load(path: &std::path::Path) -> Result<EmbFont, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        EmbFont::from_json(&s).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The names and the marks of a font file, with no letters: for the
    /// font lists, so that a large font (a Japanese font with its kanji,
    /// P20 Milestone 6) is read in full only when it is used. The letters
    /// are not checked.
    pub fn load_header(path: &std::path::Path) -> Result<EmbFontHeader, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let h: EmbFontHeader = serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))?;
        if h.format != FORMAT {
            return Err(format!("{}: {}", path.display(), EmbFontError::Format));
        }
        if h.version > VERSION {
            return Err(format!("{}: {}", path.display(), EmbFontError::Version(h.version)));
        }
        // The same rules as `load` for these fields (the audit of P20: a
        // header with a family of spaces was refused, and the font read).
        Ok(h)
    }

    /// `true` if the font has Japanese, Chinese or Korean letters.
    pub fn has_cjk(&self) -> bool {
        self.letters.keys().any(|c| crate::layout::is_cjk(*c))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.to_json()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A font of letters made by another program (a font of Ink/Stitch, ADR
    /// 0136): the letters and the sizes in mm, at the design size
    /// `cap_height_mm`. A font with fills only is a fill font.
    #[allow(clippy::too_many_arguments)]
    pub fn from_letters(
        family: &str,
        notes: String,
        cap_height_mm: f64,
        line_height_mm: f64,
        space_mm: f64,
        range: (f64, f64),
        letters: BTreeMap<char, EmbLetter>,
        kerning: BTreeMap<String, f64>,
    ) -> EmbFont {
        let fill = letters
            .values()
            .flat_map(|l| &l.parts)
            .all(|p| matches!(p, ObjectKind::Fill { .. }));
        let mut f = EmbFont {
            format: FORMAT.into(),
            version: VERSION,
            family: family.to_string(),
            style: "Regular".into(),
            notes,
            cap_height_mm,
            line_height_mm,
            space_mm,
            min_height_mm: range.0,
            max_height_mm: range.1,
            fill_letters: None,
            satin_max_mm: None,
            letters,
            kerning,
            small_text: false,
            tuned: false,
            fill,
            quality: None,
            kern: HashMap::new(),
        };
        f.index();
        f
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
                fill_parts: Vec::new(),
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
            fill_letters: None,
            satin_max_mm: None,
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
        match &mut self.fill_letters {
            // The columns end where the fill letters start.
            Some(f) => {
                f.from_mm = f
                    .from_mm
                    .min(hi.max(crate::quality::FILL_MIN_HEIGHT_MM))
                    .max(self.min_height_mm);
            }
            None => self.max_height_mm = self.max_height_mm.min(hi).max(self.min_height_mm),
        }
    }

    /// The largest capital height at which the letters sew as columns, in
    /// mm: where the fill letters start, or the end of the size range.
    pub fn column_max_mm(&self) -> f64 {
        self.fill_letters
            .map(|f| f.from_mm)
            .or(self.satin_max_mm)
            .unwrap_or(self.max_height_mm)
    }

    /// Gives each letter with columns its fill letter too (ADR 0120): the
    /// outline of `source`, the font that this font was converted from
    /// ([`crate::quality::fill_letter`]). The columns keep their size
    /// range, the fills start at its end (not below
    /// [`crate::quality::FILL_MIN_HEIGHT_MM`]), and the range of the font
    /// goes up to [`crate::source::MAX_HEIGHT_MM`]. Run it last, after the
    /// size range of the columns is made. A fill font and a font for small
    /// text get none. Returns the number of the fill letters.
    pub fn add_fill_letters(&mut self, source: &Font) -> usize {
        if self.fill || self.small_text || self.fill_letters.is_some() {
            return 0;
        }
        // A font with Japanese, Chinese or Korean letters gets none (P20
        // Milestone 6): in Auto, wide columns are tatami columns that
        // follow the strokes (ADR 0142, 0143), not one fill, so the font
        // goes up to the largest size with its columns.
        if self.has_cjk() {
            self.satin_max_mm = Some(self.max_height_mm);
            self.max_height_mm = self.max_height_mm.max(crate::source::MAX_HEIGHT_MM);
            return 0;
        }
        let mut made = Vec::new();
        for (c, letter) in &self.letters {
            if letter.parts.iter().any(|p| matches!(p, ObjectKind::Satin { .. }))
                && let Some(fill) = crate::quality::fill_letter(self, source, *c)
            {
                made.push((*c, fill.parts));
            }
        }
        if made.is_empty() {
            return 0;
        }
        let count = made.len();
        for (c, parts) in made {
            if let Some(letter) = self.letters.get_mut(&c) {
                letter.fill_parts = parts;
            }
        }
        // The smallest size of the fill letters: the start of the range of
        // the fill font with these letters.
        let mut twin = self.clone();
        twin.fill = true;
        twin.min_height_mm = crate::source::MIN_HEIGHT_MM;
        twin.max_height_mm = crate::source::MAX_HEIGHT_MM;
        for letter in twin.letters.values_mut() {
            if !letter.fill_parts.is_empty() {
                letter.parts = std::mem::take(&mut letter.fill_parts);
            }
        }
        twin.fit_size_range();
        let from_mm = self.max_height_mm.max(crate::quality::FILL_MIN_HEIGHT_MM);
        self.max_height_mm = self.max_height_mm.max(crate::source::MAX_HEIGHT_MM);
        self.fill_letters = Some(FillSizes {
            from_mm,
            min_mm: twin.min_height_mm.min(self.max_height_mm),
        });
        count
    }

    /// `true` if the letter sews as its fill letter at the scale `scale`
    /// (mm for each unit of the font) with the stitch type `stitch` of the
    /// lettering: Fill, or Automatic above the largest size of the
    /// columns. A letter with no fill letter has one shape.
    fn uses_fill(&self, letter: &EmbLetter, stitch: LetterStitch, scale: f64) -> bool {
        if letter.fill_parts.is_empty() {
            return false;
        }
        match stitch {
            LetterStitch::Fill => true,
            LetterStitch::Satin => false,
            LetterStitch::Auto => self
                .fill_letters
                .is_some_and(|f| scale * self.cap_height_mm > f.from_mm + 1e-6),
        }
    }

    /// Measures the quality of the font again ([`crate::quality::measure`]),
    /// after a change of its letters. `source` is the font that this font
    /// was converted from.
    pub fn measure_quality(&mut self, source: &Font) {
        self.quality = crate::quality::measure(self, source);
    }

    /// The best of columns and fill for each letter (ADR 0119): for each
    /// letter with columns, the fill letter of its character is made too
    /// ([`crate::quality::fill_letter`]), and it takes the place of the
    /// letter if its score is better than the shape score of the letter
    /// ([`crate::quality::LetterQuality::shape_score`]) by
    /// [`crate::quality::FILL_GAIN`]: the columns leave a part of the
    /// shape open, sew outside it or lie on each other. Run
    /// it after the steps that change the columns (the ends, the
    /// corrections, the route). The letters of `keep` stay as they are
    /// (the letters that a person corrected). `source` is the font that
    /// this font was converted from. Returns the characters that are fill
    /// letters now.
    pub fn choose_fills(&mut self, source: &Font, keep: &[char]) -> Vec<char> {
        use crate::quality::{FILL_GAIN, FILL_LETTER, fill_letter, letter_quality, outline};
        if self.fill {
            return Vec::new();
        }
        let mut better = Vec::new();
        for (c, letter) in &self.letters {
            let columns = letter.parts.iter().any(|p| matches!(p, ObjectKind::Satin { .. }));
            if !columns || keep.contains(c) {
                continue;
            }
            let shape = outline(self, source, *c);
            let Some(now) = letter_quality(self, letter, &shape) else {
                continue;
            };
            // No fill letter has a score of more than `FILL_LETTER`.
            if now.shape_score() + FILL_GAIN >= FILL_LETTER {
                continue;
            }
            let Some(fill) = fill_letter(self, source, *c) else {
                continue;
            };
            if letter_quality(self, &fill, &shape).is_some_and(|q| q.score() > now.shape_score() + FILL_GAIN) {
                better.push((*c, fill));
            }
        }
        let changed = better.iter().map(|(c, _)| *c).collect();
        for (c, fill) in better {
            self.letters.insert(c, fill);
        }
        changed
    }

    /// Rounds the coordinates of all letters as in the file (the converter
    /// does it when the font is done, before its score).
    pub fn round_letters(&mut self) {
        for letter in self.letters.values_mut() {
            for p in letter.parts.iter_mut().chain(letter.fill_parts.iter_mut()) {
                round_part(p);
            }
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

    /// Makes the ends of the columns longer where the outline of the
    /// source font has a part with no stitches ([`crate::tune::extend_ends`]).
    /// `source` is the font that this font was converted from. Returns the
    /// number of ends that changed. Not for kana and kanji: their ends
    /// that stop at a crossing would be long again (ADR 0146); lettering in
    /// TrueType fonts does the same (P21 Milestone 6).
    pub fn extend_ends(&mut self, source: &Font) -> usize {
        let s = self.cap_height_mm / source.cap_height;
        let scale = |r: &Vec<emb_geom::Point>| -> Vec<emb_geom::Point> {
            r.iter().map(|p| emb_geom::Point::new(p.x * s, p.y * s)).collect()
        };
        let cap = self.cap_height_mm;
        let mut changed = 0;
        for (c, letter) in &mut self.letters {
            // A letter with settings for its parts (corrections) keeps its
            // parts: the round parts can take the place of more columns.
            if crate::layout::is_cjk(*c) || !letter.tuning.is_empty() {
                continue;
            }
            let Some(g) = source.glyph(*c) else { continue };
            let outline: Vec<_> = source
                .shapes(g)
                .fills
                .iter()
                .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect::<Vec<_>>()))
                .collect();
            changed += crate::tune::round_parts(&mut letter.parts, &outline, cap)
                + crate::tune::extend_ends(&mut letter.parts, &outline, cap);
        }
        changed
    }

    /// Routes each letter that is not fixed ([`crate::route::route`]):
    /// the order and the directions of its parts with the fewest trims, and
    /// travel runs under the parts that sew later. A letter that a person
    /// put in order stays as it is. Returns the trims and the runs of the
    /// routed letters.
    pub fn route(&mut self) -> crate::route::Routed {
        self.route_until(&Default::default(), &Default::default(), None)
    }

    /// [`EmbFont::route`] with the font that this font was converted from:
    /// the travel runs stay in the outline of each letter (P21 Milestone 6,
    /// ADR 0153).
    pub fn route_from(&mut self, source: Option<&Font>) -> crate::route::Routed {
        self.route_until(&Default::default(), &Default::default(), source)
    }

    /// The number of letters that [`EmbFont::route`] routes.
    pub fn letters_to_route(&self) -> usize {
        self.letters
            .values()
            .filter(|l| !l.fixed && !l.parts.is_empty())
            .count()
    }

    /// [`EmbFont::route`] for a job with a progress bar and a Stop button:
    /// `done` counts the letters that are routed, and with `stop` set no
    /// more letters start (the letters that are not routed stay as they
    /// are).
    pub fn route_until(
        &mut self,
        done: &std::sync::atomic::AtomicUsize,
        stop: &std::sync::atomic::AtomicBool,
        source: Option<&Font>,
    ) -> crate::route::Routed {
        use std::sync::atomic::Ordering;
        let cap = self.cap_height_mm;
        // The outline of each letter in the source font, in mm (as
        // `extend_ends`): its rings.
        let outlines: BTreeMap<char, Vec<Vec<emb_geom::Point>>> = source.map_or_else(BTreeMap::new, |src| {
            let s = cap / src.cap_height;
            self.letters
                .iter()
                .filter(|(_, l)| !l.fixed && !l.parts.is_empty())
                .filter_map(|(c, _)| {
                    let g = src.glyph(*c)?;
                    let rings = src
                        .shapes(g)
                        .fills
                        .iter()
                        .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
                        .map(|r| r.iter().map(|p| emb_geom::Point::new(p.x * s, p.y * s)).collect())
                        .collect();
                    Some((*c, rings))
                })
                .collect()
        });
        // A letter of a script from right to left is routed from its right
        // (P21 Milestone 5).
        type Job<'a> = (bool, Option<&'a Vec<Vec<emb_geom::Point>>>, &'a mut EmbLetter);
        let todo: Vec<Job> = self
            .letters
            .iter_mut()
            .filter(|(_, l)| !l.fixed && !l.parts.is_empty())
            .map(|(c, l)| (crate::route::is_rtl(*c), outlines.get(c), l))
            .collect();
        // The letters do not depend on each other: each thread takes the
        // next letter that no thread has taken (ADR 0161; with a group of
        // letters for each thread, the threads with the slow letters, such
        // as the "M" and "W" of a serif font, worked long after the others
        // were done: Friendly Serif took 35 s, 9 s now). The cuts of a
        // letter are tried on its thread: more threads for them gave little
        // more (two: 7.7 s), and the font tool makes some fonts at the same
        // time.
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(16);
        let todo = std::sync::Mutex::new(todo);
        let found: Vec<crate::route::Routed> = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..threads)
                .map(|_| {
                    scope.spawn(|| {
                        let mut sum = crate::route::Routed::default();
                        loop {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            let Some((rtl, outline, letter)) = todo.lock().ok().and_then(|mut t| t.pop()) else {
                                break;
                            };
                            let (mut routed, found) =
                                crate::route::route_with_splits_dir(letter, cap, outline.map(|o| o.as_slice()), rtl, 1);
                            for p in &mut routed.parts {
                                round_part(p);
                            }
                            *letter = routed;
                            done.fetch_add(1, Ordering::Relaxed);
                            sum.trims += found.trims;
                            sum.runs += found.runs;
                            sum.splits += found.splits;
                            sum.on_top += found.on_top;
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
                on_top: a.on_top + b.on_top,
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

    fn character_height(&self) -> Option<f64> {
        crate::font::character_height_of(|c| {
            let letter = self.letters.get(&c)?;
            let b = letter
                .parts
                .iter()
                .filter_map(|p| p.paths().filter_map(|q| q.control_bbox()).reduce(emb_geom::BBox::union))
                .reduce(emb_geom::BBox::union)?;
            Some(b.height())
        })
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

    /// The stored parts; an embroidery font keeps its own stitch types. A
    /// letter with a fill letter (ADR 0120) gives its fills for large
    /// lettering ([`EmbFont::uses_fill`]). In `Auto`, the layout chooses
    /// the stitch of the satin columns for the text
    /// ([`crate::glyph::auto_stitches`]).
    fn parts(&self, letter: u32, stitch: LetterStitch, scale: f64) -> Vec<ObjectKind> {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .map(|l| {
                if self.uses_fill(l, stitch, scale) {
                    l.fill_parts.clone()
                } else {
                    l.parts.clone()
                }
            })
            .unwrap_or_default()
    }

    fn size_range(&self, stitch: LetterStitch) -> Option<(f64, f64)> {
        Some(match (&self.fill_letters, stitch) {
            (Some(f), LetterStitch::Satin) => (self.min_height_mm, f.from_mm),
            (Some(f), LetterStitch::Fill) => (f.min_mm, self.max_height_mm.max(f.min_mm)),
            // The satin columns of a Japanese font end where Auto makes
            // tatami columns.
            (None, LetterStitch::Satin) => (self.min_height_mm, self.column_max_mm()),
            _ => (self.min_height_mm, self.max_height_mm),
        })
    }

    fn fixed(&self, letter: u32, stitch: LetterStitch, scale: f64) -> bool {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .is_some_and(|l| l.fixed && !self.uses_fill(l, stitch, scale))
    }

    fn tuning(&self, letter: u32, stitch: LetterStitch, scale: f64) -> Vec<PartTuning> {
        char::from_u32(letter)
            .and_then(|c| self.letters.get(&c))
            .filter(|l| !self.uses_fill(l, stitch, scale))
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
        fill_parts: Vec::new(),
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
            LetterStitch::Auto => shapes.columns_cover(),
        };
        let mut parts = if satin {
            shapes.satin_parts()
        } else {
            shapes.fill_parts()
        };
        // The coordinates are rounded when the font is done
        // (`EmbFont::round_letters`): the tuning of the ends sees the exact
        // columns (P21 Milestone 6: rounded to 0.01 mm, the ends of Anton
        // "S" went on less far than in lettering).
        for p in &mut parts {
            p.transform(&t);
        }
        letters.insert(
            c,
            EmbLetter {
                advance_mm: round(font.advance(g) * scale),
                parts,
                fixed: false,
                tuning: Vec::new(),
                fill_parts: Vec::new(),
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
        fill_letters: None,
        satin_max_mm: None,
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
mod validate_tests {
    use super::*;

    fn font() -> EmbFont {
        let source = crate::bundled::font("Lato", "Bold").unwrap();
        convert(&source, "Check", "", LetterStitch::Auto, &['A', 'o'])
    }

    fn wrong(change: impl Fn(&mut EmbFont)) -> String {
        let mut f = font();
        change(&mut f);
        match EmbFont::from_json(&f.to_json()) {
            Err(EmbFontError::Invalid(what)) => what,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_font_file_with_a_wrong_number_is_refused() {
        let f = font();
        assert_eq!(f.validate(), Ok(()));
        assert!(EmbFont::from_json(&f.to_json()).is_ok());
        // Each bundled font passes.
        for (family, _) in crate::bundled::EMBROIDERY {
            assert_eq!(
                crate::bundled::embroidery(family).unwrap().validate(),
                Ok(()),
                "{family}"
            );
        }
        assert_eq!(wrong(|f| f.cap_height_mm = 1e-9), "cap_height_mm");
        assert_eq!(wrong(|f| f.cap_height_mm = 1e9), "cap_height_mm");
        assert_eq!(wrong(|f| f.line_height_mm = -5.0), "line_height_mm");
        assert_eq!(wrong(|f| f.space_mm = 1e9), "space_mm");
        assert_eq!(wrong(|f| f.min_height_mm = 0.0), "min_height_mm");
        assert_eq!(wrong(|f| f.max_height_mm = f.min_height_mm - 1.0), "max_height_mm");
        assert_eq!(wrong(|f| f.max_height_mm = 1e9), "max_height_mm");
        assert_eq!(
            wrong(|f| f.letters.get_mut(&'A').unwrap().advance_mm = 1e9),
            "letters.A: advance_mm"
        );
        assert_eq!(
            wrong(|f| {
                if let ObjectKind::Satin { geometry, .. } = &mut f.letters.get_mut(&'A').unwrap().parts[0] {
                    geometry.rail_a = geometry.rail_a.transformed(&Transform::translate(1e9, 0.0));
                }
            }),
            "letters.A: parts"
        );
        assert_eq!(
            wrong(|f| {
                if let ObjectKind::Satin { geometry, .. } = &mut f.letters.get_mut(&'o').unwrap().parts[0] {
                    geometry
                        .rungs
                        .push((emb_geom::Point::new(0.0, 0.0), emb_geom::Point::new(0.0, 5000.0)));
                }
            }),
            "letters.o: parts"
        );
        assert_eq!(
            wrong(|f| {
                f.kerning.insert("Ao".into(), -1e9);
            }),
            "kerning.Ao"
        );
        assert_eq!(wrong(|f| f.quality.as_mut().unwrap().score = 200), "quality");
        assert_eq!(wrong(|f| f.quality.as_mut().unwrap().narrow = -1.0), "quality");
        // A part that is not a column, a fill or a run is not a letter part.
        let text = wrong(|f| {
            let l = emb_model::Lettering::new("x", f.font_ref(), 10.0, emb_geom::Point::ZERO);
            f.letters
                .get_mut(&'A')
                .unwrap()
                .parts
                .push(ObjectKind::Lettering(Box::new(l)));
        });
        assert!(text.starts_with("letters.A: a part that is not"), "{text}");
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::{ColumnPattern, Lettering};

    use super::*;
    use crate::{bundled, layout};

    #[test]
    fn auto_makes_wide_columns_tatami_columns() {
        // P20 Milestones 2 and 3: in Auto, the columns of a text with a
        // typical width above the satin limit are tatami columns, in a
        // TrueType font and in an embroidery font with no fill letters.
        // Before, a TrueType letter with one wide column was all fill.
        let f = bundled::font("Lato", "Bold").unwrap();
        let stem = f.shapes(f.glyph('I').unwrap()).strokes.typical_width;
        let e = convert(&f, "Block Sans", "", LetterStitch::Auto, &['I', 'H']);
        let tatami = Some(ColumnPattern::Tatami {
            stitch_mm: crate::glyph::AUTO_TATAMI_STITCH_MM,
        });
        let kinds = |l: &Lettering| -> Vec<Option<ColumnPattern>> {
            // The columns (not the travel runs of the routes).
            l.letters
                .iter()
                .flat_map(|x| x.parts.iter())
                .filter(|p| !matches!(p, ObjectKind::Running { .. }))
                .map(|p| match p {
                    ObjectKind::Satin { params, .. } => Some(params.pattern),
                    _ => None,
                })
                .collect()
        };
        for (stroke_mm, want) in [
            (6.0, Some(ColumnPattern::Satin)),
            (10.0, Some(ColumnPattern::Satin)),
            (16.0, tatami),
        ] {
            let height = stroke_mm / stem * f.cap_height;
            for stitch in [LetterStitch::Auto, LetterStitch::Satin] {
                let mut l = Lettering::new("HI", f.font_ref(), height, Point::ZERO);
                l.stitch = stitch;
                crate::relayout(&mut l, &f);
                let want = if stitch == LetterStitch::Auto {
                    want
                } else {
                    Some(ColumnPattern::Satin)
                };
                assert!(!kinds(&l).is_empty());
                assert!(
                    kinds(&l).iter().all(|k| *k == want),
                    "{stroke_mm} mm {stitch:?}: {:?}",
                    kinds(&l)
                );
                let mut l = Lettering::new("HI", e.font_ref(), height, Point::ZERO);
                l.stitch = stitch;
                crate::relayout(&mut l, &e);
                assert!(
                    kinds(&l).iter().all(|k| *k == want),
                    "embroidery font {stroke_mm} mm: {:?}",
                    kinds(&l)
                );
            }
        }
    }

    #[test]
    fn convert_save_load_and_layout() {
        let f = bundled::font("Lato", "Bold").unwrap();
        let mut e = convert(
            &f,
            "Block Sans",
            "",
            LetterStitch::Auto,
            &"AVHno ".chars().collect::<Vec<_>>(),
        );
        // The steps of the font tool (`--extend`): lettering in the TrueType
        // font has them too (P21 Milestone 6).
        e.extend_ends(&f);
        e.round_letters();
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
