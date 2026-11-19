//! Letter corrections (ADR 0112): the changes that a person makes to single
//! letters of an embroidery font, in a file of their own. A new conversion
//! of the font applies them again, so a corrected letter stays corrected.
//!
//! A correction of a letter is one of these, or both:
//!
//! - **A new shape** (`parts`): the parts of the letter, in sew order, in
//!   mm at the design size of the font. It does not use the converted
//!   letter, so it always applies.
//! - **Steps on the converted letter**, in this order: `order` (the sew
//!   order of the parts), `reverse` (the parts that sew from their other
//!   end), `join` (a part and the next one become one column with a mitre
//!   corner). The steps are for a letter with `of` parts; if a new
//!   conversion gives another number of parts, the correction does not
//!   apply and [`Corrections::apply`] reports it.
//!
//! `tuning` gives a part its own underlay, pull compensation or fill angle,
//! and `advance_mm` sets the width of the letter. A letter with a shape or
//! with steps sews in the order of the font (`EmbLetter::fixed`).

use std::collections::BTreeMap;

use emb_geom::text;
use emb_model::{ObjectKind, PartTuning};
use serde::{Deserialize, Serialize};

use crate::embfont::{DESIGN_CAP_MM, EmbFont, EmbLetter, round_part};

pub const FILE_EXTENSION: &str = "json";
const FORMAT: &str = "embfont-corrections";
const VERSION: u32 = 1;

/// The file name of the corrections of the font `family`: small letters,
/// digits and `-`, as the files of the bundled fonts (`Block Sans`:
/// `block-sans.json`).
pub fn file_name(family: &str) -> String {
    let s: String = family
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-');
    format!("{}.{FILE_EXTENSION}", if s.is_empty() { "font" } else { s })
}

#[derive(Debug, thiserror::Error)]
pub enum CorrectionError {
    #[error("{}", text::fill("not a file of letter corrections: {}", &[.0]))]
    Parse(#[from] serde_json::Error),
    #[error("{}", text::fill("letter corrections of version {} are newer than this program reads ({})", &[.0, &VERSION]))]
    Version(u32),
    #[error("{}", text::tr("not a file of letter corrections"))]
    Format,
}

/// The correction of one letter. See the module notes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LetterCorrection {
    /// A new shape: the parts in sew order, in mm at the design size, the
    /// baseline at Y = 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<ObjectKind>>,
    /// The number of parts of the converted letter that the steps are for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<usize>,
    /// The sew order: each part of the converted letter one time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<usize>,
    /// The parts (in the new order) that sew from their other end.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reverse: Vec<usize>,
    /// The parts (in the new order) that join the part after them: the two
    /// columns become one column with a mitre corner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub join: Vec<usize>,
    /// The settings of parts, by their place after the steps.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tuning: BTreeMap<usize, PartTuning>,
    /// The parts sew in this order. With no value: yes for a letter with a
    /// shape or with steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,
    /// The width of the letter (the distance to the next letter), in mm at
    /// the design size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_mm: Option<f64>,
}

impl LetterCorrection {
    /// The correction that gives `letter` as it is: its shape, its
    /// settings and its width.
    pub fn from_letter(letter: &EmbLetter) -> LetterCorrection {
        LetterCorrection {
            parts: Some(letter.parts.clone()),
            tuning: letter
                .tuning
                .iter()
                .enumerate()
                .filter(|(_, t)| !t.is_empty())
                .map(|(i, t)| (i, t.clone()))
                .collect(),
            fixed: Some(letter.fixed),
            advance_mm: Some(letter.advance_mm),
            ..Default::default()
        }
    }

    fn has_steps(&self) -> bool {
        !(self.order.is_empty() && self.reverse.is_empty() && self.join.is_empty())
    }

    /// The corrected letter. `letter` is the letter of the font (`None`: the
    /// font does not have it; then the correction must have a shape).
    pub fn apply(&self, letter: Option<&EmbLetter>) -> Result<EmbLetter, String> {
        let new = self.corrected(letter)?;
        // A corrections file can come from another person: the numbers of
        // the letter are in the range of a font (the audit of P17).
        crate::embfont::check_letter(&new)
            .map_err(|what| text::fill("the embroidery font has a wrong value: {}", &[&what]))?;
        Ok(new)
    }

    fn corrected(&self, letter: Option<&EmbLetter>) -> Result<EmbLetter, String> {
        let mut parts = match (&self.parts, letter) {
            (Some(parts), _) => parts.clone(),
            (None, Some(l)) => {
                let n = l.parts.len();
                match self.of {
                    Some(of) if of == n => {}
                    Some(of) => {
                        return Err(text::fill(
                            "the letter now has {} parts; the correction is for {}",
                            &[&n, &of],
                        ));
                    }
                    None if self.has_steps() || !self.tuning.is_empty() => {
                        return Err(text::tr("the correction does not give the number of parts (of)").into());
                    }
                    None => {}
                }
                l.parts.clone()
            }
            (None, None) => return Err(text::tr("the font does not have this letter").into()),
        };
        let n = parts.len();
        if !self.order.is_empty() {
            let mut seen = vec![false; n];
            for &i in &self.order {
                if i >= n || std::mem::replace(&mut seen[i], true) {
                    return Err(text::tr("the order does not have each part one time").into());
                }
            }
            if self.order.len() != n {
                return Err(text::tr("the order does not have each part one time").into());
            }
            parts = self.order.iter().map(|&i| parts[i].clone()).collect();
        }
        for &i in &self.reverse {
            let reversed = parts.get_mut(i).is_some_and(emb_model::editing::reverse_kind);
            if !reversed {
                return Err(text::fill("part {} has no direction to reverse", &[&i]));
            }
        }
        // From the last join to the first, so the numbers stay correct.
        let mut joins = self.join.clone();
        joins.sort_unstable();
        joins.dedup();
        for &i in joins.iter().rev() {
            let joined = match (parts.get(i), parts.get(i + 1)) {
                (Some(ObjectKind::Satin { geometry: a, .. }), Some(ObjectKind::Satin { geometry: b, .. })) => {
                    emb_model::mitre::join(a, b)
                }
                _ => None,
            };
            let Some(joined) = joined else {
                return Err(text::fill(
                    "parts {} and {} are not two columns that meet at a corner",
                    &[&i, &(i + 1)],
                ));
            };
            if let ObjectKind::Satin { geometry, .. } = &mut parts[i] {
                *geometry = joined;
            }
            parts.remove(i + 1);
        }
        let mut tuning = vec![PartTuning::default(); parts.len()];
        for (&i, t) in &self.tuning {
            match tuning.get_mut(i) {
                Some(slot) => *slot = t.clone(),
                None => return Err(text::fill("there is no part {} for the settings", &[&i])),
            }
        }
        if tuning.iter().all(PartTuning::is_empty) {
            tuning.clear();
        }
        for p in &mut parts {
            round_part(p);
        }
        let advance_mm = self
            .advance_mm
            .or(letter.map(|l| l.advance_mm))
            .unwrap_or_else(|| advance_of(&parts));
        Ok(EmbLetter {
            advance_mm,
            fixed: self.fixed.unwrap_or(self.parts.is_some() || self.has_steps()),
            parts,
            tuning,
            fill_parts: letter.map(|l| l.fill_parts.clone()).unwrap_or_default(),
        })
    }
}

/// The width of a letter with no width: its parts and a small space.
fn advance_of(parts: &[ObjectKind]) -> f64 {
    parts
        .iter()
        .filter_map(|p| p.control_bbox())
        .reduce(emb_geom::BBox::union)
        .map_or(0.3 * DESIGN_CAP_MM, |b| b.max.x.max(0.0) + 0.08 * DESIGN_CAP_MM)
}

/// A correction that did not apply.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub letter: char,
    pub text: String,
}

/// The letter corrections of one font.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Corrections {
    pub format: String,
    pub version: u32,
    /// The family of the font that the corrections are for.
    pub family: String,
    /// Who made the corrections, and why.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    pub letters: BTreeMap<char, LetterCorrection>,
}

impl Corrections {
    pub fn new(family: &str) -> Corrections {
        Corrections {
            format: FORMAT.into(),
            version: VERSION,
            family: family.to_string(),
            notes: String::new(),
            letters: BTreeMap::new(),
        }
    }

    pub fn from_json(s: &str) -> Result<Corrections, CorrectionError> {
        let c: Corrections = serde_json::from_str(s)?;
        if c.format != FORMAT {
            return Err(CorrectionError::Format);
        }
        if c.version > VERSION {
            return Err(CorrectionError::Version(c.version));
        }
        Ok(c)
    }

    /// The text of the file: one line for each level, so a change of one
    /// letter is a small change of the file.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("corrections serialize");
        s.push('\n');
        s
    }

    pub fn load(path: &std::path::Path) -> Result<Corrections, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Corrections::from_json(&s).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, self.to_json()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Changes the letters of `font`. Returns the corrections that did not
    /// apply; their letters stay as they are.
    pub fn apply(&self, font: &mut EmbFont) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (&c, correction) in &self.letters {
            match correction.apply(font.letters.get(&c)) {
                Ok(letter) => font.set_letter(c, letter),
                Err(text) => problems.push(Problem { letter: c, text }),
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{LetterStitch, Lettering, SatinGeometry, Underlay};

    use super::*;
    use crate::{bundled, embfont, layout};

    fn font() -> EmbFont {
        let f = bundled::font("Lato", "Bold").unwrap();
        embfont::convert(
            &f,
            "Block Sans",
            "",
            LetterStitch::Auto,
            &"HTLio".chars().collect::<Vec<_>>(),
        )
    }

    fn column(from: (f64, f64), to: (f64, f64)) -> ObjectKind {
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(
                &Path::new()
                    .move_to(Point::new(from.0, from.1))
                    .line_to(Point::new(to.0, to.1)),
                3.0,
            ),
            params: Default::default(),
        }
    }

    #[test]
    fn steps_change_the_order_and_the_direction() {
        let mut f = font();
        let before = f.letters[&'H'].clone();
        let n = before.parts.len();
        assert!(n >= 3, "H has three strokes: {n}");
        let order: Vec<usize> = (0..n).rev().collect();
        let mut c = Corrections::new("Block Sans");
        c.letters.insert(
            'H',
            LetterCorrection {
                of: Some(n),
                order: order.clone(),
                reverse: vec![0],
                tuning: BTreeMap::from([(
                    1,
                    PartTuning {
                        pull_mm: 0.1,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
        assert!(c.apply(&mut f).is_empty());
        let after = &f.letters[&'H'];
        assert!(after.fixed);
        assert_eq!(after.advance_mm, before.advance_mm);
        assert_eq!(after.parts[1], before.parts[n - 2]);
        // The first part is the last one, the other way.
        let mut back = after.parts[0].clone();
        assert!(emb_model::editing::reverse_kind(&mut back));
        assert_eq!(back, before.parts[n - 1]);
        assert_eq!(after.tuning.len(), n);
        assert_eq!(after.tuning[1].pull_mm, 0.1);
        // The file keeps all of it.
        let back = Corrections::from_json(&c.to_json()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn a_corrected_letter_stays_corrected_after_a_new_conversion() {
        let mut c = Corrections::new("Block Sans");
        // The stem and the bar, each to the outer edge of the other.
        let shape = vec![column((2.0, 0.0), (2.0, -25.0)), column((0.5, -23.5), (14.0, -23.5))];
        c.letters.insert(
            'T',
            LetterCorrection {
                parts: Some(shape),
                join: vec![0],
                advance_mm: Some(17.0),
                ..Default::default()
            },
        );
        let n = font().letters[&'L'].parts.len();
        c.letters.insert(
            'L',
            LetterCorrection {
                of: Some(n),
                order: (0..n).rev().collect(),
                ..Default::default()
            },
        );
        let mut first = font();
        assert!(c.apply(&mut first).is_empty());
        // The two columns of the shape are one column with a corner.
        assert_eq!(first.letters[&'T'].parts.len(), 1);
        assert_eq!(first.letters[&'T'].advance_mm, 17.0);
        assert!(first.letters[&'T'].fixed);
        // The font is made again: the same letters.
        let mut second = font();
        assert!(c.apply(&mut second).is_empty());
        assert_eq!(first.letters, second.letters);
        // The saved font keeps the order and the settings.
        let back = EmbFont::from_json(&first.to_json()).unwrap();
        assert_eq!(back.letters, first.letters);
        // A shape applies also to a font that has the correction already
        // (the corrections of a user on a bundled font).
        let mut again = first.clone();
        let mut shapes = Corrections::new("Block Sans");
        shapes
            .letters
            .insert('T', LetterCorrection::from_letter(&first.letters[&'T']));
        assert!(shapes.apply(&mut again).is_empty());
        assert_eq!(again.letters[&'T'], first.letters[&'T']);
    }

    #[test]
    fn a_correction_that_does_not_fit_is_reported() {
        let mut f = font();
        let before = f.letters.clone();
        let n = f.letters[&'H'].parts.len();
        let mut c = Corrections::new("Block Sans");
        let step = |of: usize, order: Vec<usize>| LetterCorrection {
            of: Some(of),
            order,
            ..Default::default()
        };
        // Another number of parts, a bad order, a letter that is not there,
        // a fill with no direction, steps with no number of parts.
        c.letters.insert('H', step(n + 1, Vec::new()));
        c.letters.insert('T', step(f.letters[&'T'].parts.len(), vec![0, 0]));
        c.letters.insert('Z', step(1, Vec::new()));
        c.letters.insert(
            'i',
            LetterCorrection {
                order: vec![0],
                ..Default::default()
            },
        );
        let problems = c.apply(&mut f);
        let letters: String = problems.iter().map(|p| p.letter).collect();
        assert_eq!(letters, "HTZi", "{problems:?}");
        assert!(problems[0].text.contains(&format!("{n}")), "{}", problems[0].text);
        assert_eq!(f.letters, before, "the letters stay");
        // Bad files.
        assert!(matches!(Corrections::from_json("{}"), Err(CorrectionError::Parse(_))));
        let mut bad = Corrections::new("X");
        bad.version = 9;
        assert!(matches!(
            Corrections::from_json(&bad.to_json()),
            Err(CorrectionError::Version(9))
        ));
        bad.version = 1;
        bad.format = "other".into();
        assert!(matches!(
            Corrections::from_json(&bad.to_json()),
            Err(CorrectionError::Format)
        ));
    }

    #[test]
    fn lettering_takes_the_order_and_the_settings_of_the_font() {
        let mut f = font();
        let n = f.letters[&'H'].parts.len();
        let heavy = Underlay::Heavy;
        let mut c = Corrections::new("Block Sans");
        c.letters.insert(
            'H',
            LetterCorrection {
                of: Some(n),
                order: (0..n).rev().collect(),
                tuning: BTreeMap::from([(
                    0,
                    PartTuning {
                        underlay: Some(heavy.clone()),
                        pull_mm: 0.15,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
        assert!(c.apply(&mut f).is_empty());
        let f = EmbFont::from_json(&f.to_json()).unwrap();
        let mut l = Lettering::new("HT", crate::LetterSource::font_ref(&f), 20.0, Point::new(0.0, 0.0));
        l.satin.pull_compensation_mm = 0.2;
        layout::relayout(&mut l, &f);
        let (h, t) = (&l.letters[0], &l.letters[1]);
        assert!(h.fixed && !t.fixed);
        let ObjectKind::Satin { params, .. } = &h.parts[0] else {
            panic!("a column")
        };
        assert_eq!(params.underlay, heavy);
        assert!((params.pull_compensation_mm - 0.35).abs() < 1e-9);
        let ObjectKind::Satin { params, .. } = &h.parts[1] else {
            panic!("a column")
        };
        assert_eq!(params.underlay, l.satin.underlay);
        assert!((params.pull_compensation_mm - 0.2).abs() < 1e-9);
        // A change of the settings of the object keeps the settings of
        // the font.
        l.satin.pull_compensation_mm = 0.3;
        l.apply_params();
        let ObjectKind::Satin { params, .. } = &l.letters[0].parts[0] else {
            panic!("a column")
        };
        assert!((params.pull_compensation_mm - 0.45).abs() < 1e-9);
        // The corner option of the object does not join a fixed letter.
        l.mitre_corners = true;
        layout::relayout(&mut l, &f);
        assert_eq!(l.letters[0].parts.len(), n);
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name("Block Sans"), "block-sans.json");
        assert_eq!(file_name(" Fat Face Serif "), "fat-face-serif.json");
        assert_eq!(file_name("書体"), "font.json");
    }
}
