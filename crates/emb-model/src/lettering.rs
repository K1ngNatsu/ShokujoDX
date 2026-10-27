//! Lettering objects: a text in a font, as letters made of satin and fill parts.
//!
//! The object keeps the letter shapes (ADR 0015), so a design opens and sews
//! without the font. `emb-letter` makes the shapes from the font; the app
//! makes them again when the text, the font or a setting changes.
//!
//! Lettering space: the baseline of the first line starts at the origin, X
//! goes to the right, Y goes down, in mm. `placement` maps lettering space
//! to the design. The parts are stored in design coordinates (placement
//! applied), so the other object code uses them as they are.
//!
//! Layout puts the letters on straight lines; the [`Baseline`] then moves
//! them onto an arc, a circle or a path. Changes to single letters by hand
//! (kerning, move, size, turn, a changed shape) are [`LetterEdit`]s, kept by
//! the character index in the text (ADR 0017). An [`Envelope`] bends the
//! whole text after that; the sew order and the connections of the letters
//! are settings of the object (ADR 0018).

use emb_geom::{BBox, Point, Transform};
use serde::{Deserialize, Serialize};

use crate::{Baseline, Envelope, FillParams, ObjectKind, SatinParams, Underlay};

/// The default lettering height (capital height), in mm.
pub const DEFAULT_HEIGHT_MM: f64 = 25.0;

/// The kind of font file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontKind {
    /// A TrueType or OpenType font: letters come from glyph outlines.
    #[default]
    TrueType,
    /// An embroidery font (`.embfont`, ADR 0016): letters are stored as
    /// embroidery objects.
    Embroidery,
}

impl FontKind {
    fn is_true_type(&self) -> bool {
        *self == FontKind::TrueType
    }
}

/// A font by its names, as the font file gives them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FontRef {
    /// Family name, for example "Lato".
    pub family: String,
    /// Style (subfamily) name, for example "Bold".
    pub style: String,
    #[serde(default, skip_serializing_if = "FontKind::is_true_type")]
    pub kind: FontKind,
}

impl FontRef {
    /// A TrueType or OpenType font.
    pub fn new(family: &str, style: &str) -> FontRef {
        FontRef {
            family: family.to_string(),
            style: style.to_string(),
            kind: FontKind::TrueType,
        }
    }

    /// An embroidery font.
    pub fn embroidery(family: &str, style: &str) -> FontRef {
        FontRef {
            kind: FontKind::Embroidery,
            ..FontRef::new(family, style)
        }
    }

    /// The same names and kind, in any letter case.
    pub fn same(&self, o: &FontRef) -> bool {
        self.kind == o.kind && self.family.eq_ignore_ascii_case(&o.family) && self.style.eq_ignore_ascii_case(&o.style)
    }

    /// "Lato Bold"; the family alone for the regular style.
    pub fn label(&self) -> String {
        if self.style.is_empty() || self.style.eq_ignore_ascii_case("regular") {
            self.family.clone()
        } else {
            format!("{} {}", self.family, self.style)
        }
    }
}

/// How the lines of a lettering object align to the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Justify {
    #[default]
    Left,
    Center,
    Right,
}

/// The stitch type of the letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LetterStitch {
    /// Satin where the strokes are narrow enough, else fill (for each letter).
    #[default]
    Auto,
    Satin,
    Fill,
}

/// The order in which the letters sew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LetterOrder {
    /// In the order of the text.
    #[default]
    Text,
    /// From the middle letter to the right end, then from the middle to the
    /// left end (for caps: the fabric moves less).
    CenterOut,
    /// The nearest letter next; each letter ends near the next one.
    Closest,
}

/// How a letter joins the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LetterConnection {
    /// A travel stitch over a short gap, a trim over a longer one.
    #[default]
    Trim,
    /// No trims in the lettering: a long gap is a jump (for machines with
    /// no trimmer; cut the jumps by hand).
    Jump,
}

/// The gap above which letters get a trim, in mm (the default).
pub const TRIM_OVER_MM: f64 = 1.0;

/// One letter: its text (one character) and its parts, in sew order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Letter {
    pub text: String,
    /// The line of the text that the letter is on (from 0).
    #[serde(default)]
    pub line: usize,
    /// The index of the character in the text (all characters, with the
    /// line breaks).
    #[serde(default)]
    pub index: usize,
    /// From the letter frame (mm, the pen point on the baseline at the
    /// origin) to lettering space: the place on the line and the baseline,
    /// not the edit of the letter.
    #[serde(default)]
    pub frame: Transform,
    /// The middle of the letter box in the letter frame: the anchor on the
    /// baseline is `(center.x, 0)`; edits turn and scale the letter about
    /// `center`.
    #[serde(default)]
    pub center: Point,
    /// Satin and fill parts in design coordinates.
    pub parts: Vec<ObjectKind>,
}

/// A letter shape changed by hand, kept in the design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetterShape {
    /// The character that the shape is for.
    pub text: String,
    /// The lettering height when the shape was made; the shape scales with
    /// the height.
    pub height_mm: f64,
    /// The parts in the letter frame, before the edit (see [`Letter::frame`]).
    pub parts: Vec<ObjectKind>,
}

/// Changes to one letter by hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetterEdit {
    /// The character index in the text (see [`Letter::index`]).
    pub index: usize,
    /// Extra space before the letter, in mm. It also moves the letters after
    /// it on the same line (manual kerning).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub kern_mm: f64,
    /// Move along the baseline (X) and down from it (Y), in mm. Only this
    /// letter moves.
    #[serde(default, skip_serializing_if = "is_origin")]
    pub offset: Point,
    /// Size factor about the middle of the letter.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f64,
    /// Turn about the middle of the letter, clockwise, in degrees.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotate_deg: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<LetterShape>,
    /// The stitch type of this letter (TrueType fonts), if not the one of
    /// the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitch: Option<LetterStitch>,
    /// The underlay of this letter, if not the one of the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlay: Option<Underlay>,
}

impl LetterEdit {
    pub fn new(index: usize) -> LetterEdit {
        LetterEdit {
            index,
            kern_mm: 0.0,
            offset: Point::ZERO,
            scale: 1.0,
            rotate_deg: 0.0,
            shape: None,
            stitch: None,
            underlay: None,
        }
    }

    /// No change.
    pub fn is_empty(&self) -> bool {
        *self == LetterEdit::new(self.index)
    }

    /// Size, turn and move, in the letter frame of a letter with `center`.
    pub fn transform(&self, center: Point) -> Transform {
        Transform::translate(-center.x, -center.y)
            .then(Transform::scale(self.scale, self.scale))
            .then(Transform::rotate_deg(self.rotate_deg))
            .then(Transform::translate(center.x + self.offset.x, center.y + self.offset.y))
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_one(v: &f64) -> bool {
    *v == 1.0
}

fn is_origin(p: &Point) -> bool {
    *p == Point::ZERO
}

/// A lettering object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lettering {
    /// The text; `\n` starts a new line.
    pub text: String,
    pub font: FontRef,
    /// Capital height (the height of "H"), in mm.
    pub height_mm: f64,
    /// Extra space after each letter, in mm (can be negative).
    #[serde(default)]
    pub letter_spacing_mm: f64,
    /// Line distance as a factor of the line height of the font.
    #[serde(default = "one")]
    pub line_spacing: f64,
    #[serde(default)]
    pub justify: Justify,
    #[serde(default, skip_serializing_if = "Baseline::is_straight")]
    pub baseline: Baseline,
    /// Bends the text (with a straight baseline only).
    #[serde(default, skip_serializing_if = "Envelope::is_none")]
    pub envelope: Envelope,
    /// The text box that the envelope bends, in lettering space, from the
    /// last layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope_box: Option<BBox>,
    #[serde(default)]
    pub order: LetterOrder,
    #[serde(default)]
    pub connection: LetterConnection,
    /// Gaps between letters longer than this get a trim (or a jump), in mm.
    #[serde(default = "trim_over")]
    pub trim_over_mm: f64,
    /// Use the kerning of the font.
    #[serde(default = "yes")]
    pub kerning: bool,
    #[serde(default)]
    pub stitch: LetterStitch,
    #[serde(default = "letter_satin")]
    pub satin: SatinParams,
    #[serde(default = "letter_fill")]
    pub fill: FillParams,
    /// The recommended height range of the font (min, max) in mm, from the
    /// last layout. The design checks warn outside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_mm: Option<(f64, f64)>,
    /// From lettering space to the design.
    #[serde(default)]
    pub placement: Transform,
    /// Changes to single letters, in index order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<LetterEdit>,
    #[serde(default)]
    pub letters: Vec<Letter>,
}

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

fn trim_over() -> f64 {
    TRIM_OVER_MM
}

/// Satin settings for letters: as a normal satin.
pub fn letter_satin() -> SatinParams {
    SatinParams::default()
}

/// Fill settings for letters: rows at 45 degrees.
pub fn letter_fill() -> FillParams {
    FillParams::default()
}

impl Lettering {
    /// A lettering object with no letters yet, with the baseline start at `at`.
    pub fn new(text: &str, font: FontRef, height_mm: f64, at: Point) -> Lettering {
        Lettering {
            text: text.to_string(),
            font,
            height_mm,
            letter_spacing_mm: 0.0,
            line_spacing: 1.0,
            justify: Justify::default(),
            baseline: Baseline::Straight,
            envelope: Envelope::default(),
            envelope_box: None,
            order: LetterOrder::Text,
            connection: LetterConnection::Trim,
            trim_over_mm: TRIM_OVER_MM,
            kerning: true,
            stitch: LetterStitch::default(),
            satin: letter_satin(),
            fill: letter_fill(),
            range_mm: None,
            placement: Transform::translate(at.x, at.y),
            edits: Vec::new(),
            letters: Vec::new(),
        }
    }

    /// The edit of the character at `index`, if it has one.
    pub fn edit(&self, index: usize) -> Option<&LetterEdit> {
        self.edits.iter().find(|e| e.index == index)
    }

    /// The edit of the character at `index`; a new empty one if it has none.
    pub fn edit_mut(&mut self, index: usize) -> &mut LetterEdit {
        let i = match self.edits.binary_search_by_key(&index, |e| e.index) {
            Ok(i) => i,
            Err(i) => {
                self.edits.insert(i, LetterEdit::new(index));
                i
            }
        };
        &mut self.edits[i]
    }

    /// Removes the edits that change nothing.
    pub fn prune_edits(&mut self) {
        self.edits.retain(|e| !e.is_empty());
    }

    /// Removes all changes by hand from the character at `index`.
    pub fn reset_letter(&mut self, index: usize) {
        self.edits.retain(|e| e.index != index);
    }

    /// The transform from the letter frame of letter `letter` to the
    /// design, with the edit of the letter (not the envelope).
    pub fn letter_to_design(&self, letter: usize) -> Option<Transform> {
        Some(self.letter_to_lettering(letter)?.then(self.placement))
    }

    /// A point of the letter frame of letter `letter` in the design, with
    /// the envelope.
    pub fn letter_point(&self, letter: usize, p: Point) -> Option<Point> {
        let q = self.letter_to_lettering(letter)?.apply(p);
        let q = self.warp().map_or(q, |(e, b)| e.warp(&b, q));
        Some(self.placement.apply(q))
    }

    /// Changes the edit of letter `letter` with `f` and moves its parts to
    /// match, with no new layout (so with no font). Use a new layout for a
    /// change of `kern_mm` or `shape`: they move other letters or change the
    /// parts.
    pub fn change_letter(&mut self, letter: usize, f: impl FnOnce(&mut LetterEdit)) {
        let Some(old) = self.letter_to_lettering(letter).and_then(|t| t.inverse()) else {
            return;
        };
        let index = self.letters[letter].index;
        f(self.edit_mut(index));
        self.prune_edits();
        let Some(new) = self.letter_to_lettering(letter) else {
            return;
        };
        let t = old.then(new);
        let Some(unplace) = self.placement.inverse() else {
            return;
        };
        let (place, warp) = (self.placement, self.warp());
        for p in &mut self.letters[letter].parts {
            crate::envelope::map_points(p, &|q| {
                let q = unplace.apply(q);
                let q = warp.map_or(q, |(e, b)| e.unwarp(&b, q));
                let q = t.apply(q);
                place.apply(warp.map_or(q, |(e, b)| e.warp(&b, q)))
            });
        }
        self.apply_params();
    }

    /// The envelope and its box, if the text is bent.
    pub fn warp(&self) -> Option<(Envelope, BBox)> {
        (!self.envelope.is_none() && self.baseline.is_straight())
            .then_some(self.envelope_box.map(|b| (self.envelope, b)))
            .flatten()
    }

    /// The transform from the letter frame of letter `letter` to lettering
    /// space (before the envelope), with the edit of the letter.
    pub fn letter_to_lettering(&self, letter: usize) -> Option<Transform> {
        let l = self.letters.get(letter)?;
        let edit = self
            .edit(l.index)
            .map_or(Transform::IDENTITY, |e| e.transform(l.center));
        Some(edit.then(l.frame))
    }

    /// The letter (index in `letters`) that path `k` of the object (see
    /// `ObjectKind::paths`) is in.
    pub fn letter_of_path(&self, k: usize) -> Option<usize> {
        let mut first = 0;
        for (i, l) in self.letters.iter().enumerate() {
            let n: usize = l.parts.iter().map(|p| p.paths().count()).sum();
            if k < first + n {
                return Some(i);
            }
            first += n;
        }
        None
    }

    /// The paths of letter `letter`, as indexes into `ObjectKind::paths`.
    pub fn paths_of_letter(&self, letter: usize) -> std::ops::Range<usize> {
        let count = |l: &Letter| l.parts.iter().map(|p| p.paths().count()).sum::<usize>();
        let first = self.letters.iter().take(letter).map(count).sum();
        first..first + self.letters.get(letter).map_or(0, count)
    }

    /// Keeps the current parts of letter `letter` (for example after a
    /// change of its nodes) as its shape, so that a new layout keeps them.
    pub fn keep_shape(&mut self, letter: usize) {
        let Some(l) = self.letters.get(letter) else { return };
        // The shape is before the edit and the envelope.
        let (Some(inv), Some(unplace)) = (
            self.letter_to_lettering(letter).and_then(|t| t.inverse()),
            self.placement.inverse(),
        ) else {
            return;
        };
        let warp = self.warp();
        let parts = l
            .parts
            .iter()
            .map(|p| {
                let mut p = p.clone();
                crate::envelope::map_points(&mut p, &|q| {
                    let q = unplace.apply(q);
                    inv.apply(warp.map_or(q, |(e, b)| e.unwarp(&b, q)))
                });
                p
            })
            .collect();
        let shape = LetterShape {
            text: l.text.clone(),
            height_mm: self.height_mm,
            parts,
        };
        let index = l.index;
        self.edit_mut(index).shape = Some(shape);
    }

    /// Moves the edits to a new text: the characters before and after the
    /// changed part keep their edits; the changed characters lose them.
    pub fn retarget_edits(&mut self, new_text: &str) {
        if self.edits.is_empty() {
            return;
        }
        let old: Vec<char> = self.text.chars().collect();
        let new: Vec<char> = new_text.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let max_suffix = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a == b)
            .count();
        let (old_len, new_len) = (old.len(), new.len());
        self.edits.retain_mut(|e| {
            if e.index < prefix {
                true
            } else if e.index >= old_len - suffix && e.index < old_len {
                e.index = e.index + new_len - old_len;
                true
            } else {
                false
            }
        });
    }

    /// All parts of all letters, in sew order.
    pub fn parts(&self) -> impl Iterator<Item = &ObjectKind> + '_ {
        self.letters.iter().flat_map(|l| l.parts.iter())
    }

    pub fn parts_mut(&mut self) -> impl Iterator<Item = &mut ObjectKind> + '_ {
        self.letters.iter_mut().flat_map(|l| l.parts.iter_mut())
    }

    /// Sets new letters, made in lettering space: applies the placement and
    /// the satin and fill settings of the object.
    pub fn set_letters(&mut self, mut letters: Vec<Letter>) {
        let t = self.placement;
        for part in letters.iter_mut().flat_map(|l| l.parts.iter_mut()) {
            part.transform(&t);
        }
        self.letters = letters;
        self.apply_params();
    }

    /// Moves the vertical scale of the placement into the lettering: the
    /// height, the spacing, the changes by hand, the baseline and the
    /// envelope box get larger or smaller, and the placement keeps only the
    /// turn, the move and a stretch (the width as a part of the height). The
    /// letters in the design do not change. After a scale on the canvas,
    /// `height_mm` is the real capital height again.
    pub fn scale_into_height(&mut self) {
        let s = self.placement.apply_vector(Point::new(0.0, 1.0)).length();
        if !(s.is_finite() && s > 1e-9) || (s - 1.0).abs() < 1e-12 {
            return;
        }
        let k = Transform::scale(s, s);
        self.placement = Transform::scale(1.0 / s, 1.0 / s).then(self.placement);
        self.height_mm *= s;
        self.letter_spacing_mm *= s;
        for e in &mut self.edits {
            e.kern_mm *= s;
            e.offset = e.offset * s;
        }
        match &mut self.baseline {
            Baseline::Straight => {}
            Baseline::Arc { radius_mm, .. } | Baseline::Circle { radius_mm, .. } => *radius_mm *= s,
            Baseline::Path { path } => *path = path.transformed(&k),
        }
        self.envelope_box = self.envelope_box.map(|b| BBox::new(k.apply(b.min), k.apply(b.max)));
        for l in &mut self.letters {
            l.frame.e *= s;
            l.frame.f *= s;
            l.center = l.center * s;
        }
    }

    /// Copies the satin and fill settings of the object to the parts, with
    /// the underlay of each letter that has its own. The fill angle of each
    /// part keeps the turn of the placement.
    pub fn apply_params(&mut self) {
        let (satin, fill, t) = (self.satin.clone(), self.fill.clone(), self.placement);
        let underlays: Vec<Option<Underlay>> = self
            .letters
            .iter()
            .map(|l| self.edit(l.index).and_then(|e| e.underlay.clone()))
            .collect();
        for (letter, underlay) in self.letters.iter_mut().zip(underlays) {
            for part in &mut letter.parts {
                match part {
                    ObjectKind::Satin { params, .. } => {
                        *params = satin.clone();
                        if let Some(u) = &underlay {
                            params.underlay = u.clone();
                        }
                    }
                    ObjectKind::Fill { params, .. } => {
                        *params = fill.clone();
                        params.angle_deg = crate::edit::transform_angle(fill.angle_deg, &t);
                        if let Some(u) = &underlay {
                            params.underlay = u.clone();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// A text for the design checks if the height is outside the
    /// recommended range of the font.
    pub fn size_warning(&self) -> Option<String> {
        let (lo, hi) = self.range_mm?;
        let what = if self.height_mm < lo - 1e-6 {
            "smaller"
        } else if self.height_mm > hi + 1e-6 {
            "larger"
        } else {
            return None;
        };
        Some(format!(
            "Lettering {:.1} mm is {what} than the size range of {} ({lo:.0}–{hi:.0} mm)",
            self.height_mm,
            self.font.label()
        ))
    }

    /// The lines of the text.
    pub fn lines(&self) -> Vec<&str> {
        self.text.split('\n').collect()
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;

    use super::*;
    use crate::SatinGeometry;

    fn square(at: f64) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(emb_geom::BBox::new(Point::new(at, -10.0), Point::new(at + 5.0, 0.0))),
            holes: vec![],
            params: FillParams::default(),
        }
    }

    #[test]
    fn letters_get_placement_and_settings() {
        let mut l = Lettering::new("AB", FontRef::new("Lato", "Bold"), 10.0, Point::new(100.0, 50.0));
        l.fill.spacing_mm = 0.5;
        l.satin.spacing_mm = 0.3;
        l.set_letters(vec![
            Letter {
                text: "A".into(),
                line: 0,
                index: 0,
                frame: Transform::IDENTITY,
                center: Point::ZERO,
                parts: vec![square(0.0)],
            },
            Letter {
                text: "B".into(),
                line: 0,
                index: 0,
                frame: Transform::IDENTITY,
                center: Point::ZERO,
                parts: vec![ObjectKind::Satin {
                    geometry: SatinGeometry::from_centerline(
                        &Path::new()
                            .move_to(Point::new(7.0, 0.0))
                            .line_to(Point::new(7.0, -10.0)),
                        1.0,
                    ),
                    params: SatinParams::default(),
                }],
            },
        ]);
        let b = l
            .parts()
            .next()
            .unwrap()
            .paths()
            .next()
            .unwrap()
            .control_bbox()
            .unwrap();
        assert_eq!(b.min, Point::new(100.0, 40.0));
        let ObjectKind::Fill { params, .. } = &l.letters[0].parts[0] else {
            panic!()
        };
        assert_eq!(params.spacing_mm, 0.5);
        let ObjectKind::Satin { params, .. } = &l.letters[1].parts[0] else {
            panic!()
        };
        assert_eq!(params.spacing_mm, 0.3);
        assert_eq!(FontRef::new("Lato", "Regular").label(), "Lato");
        assert_eq!(l.font.label(), "Lato Bold");
        assert!(FontRef::new("lato", "BOLD").same(&l.font));
        assert!(!FontRef::embroidery("Lato", "Bold").same(&l.font));
        l.range_mm = Some((8.0, 40.0));
        assert!(l.size_warning().is_none());
        l.height_mm = 6.0;
        assert_eq!(
            l.size_warning().as_deref(),
            Some("Lettering 6.0 mm is smaller than the size range of Lato Bold (8–40 mm)")
        );
    }

    #[test]
    fn change_one_letter_with_no_font() {
        let mut l = Lettering::new("AB", FontRef::new("Lato", "Bold"), 10.0, Point::new(100.0, 50.0));
        let letter = |at: f64, index: usize| Letter {
            text: "A".into(),
            line: 0,
            index,
            frame: Transform::translate(at, 0.0),
            center: Point::new(2.5, -5.0),
            parts: vec![square(at)],
        };
        l.set_letters(vec![letter(0.0, 0), letter(7.0, 1)]);
        assert_eq!(l.paths_of_letter(1), 1..2);
        assert_eq!(l.letter_of_path(1), Some(1));
        assert_eq!(l.letter_of_path(2), None);
        let bb = |l: &Lettering, i: usize| l.letters[i].parts[0].control_bbox().unwrap();
        let before = bb(&l, 1);
        l.change_letter(1, |e| {
            e.offset = Point::new(1.0, 2.0);
            e.rotate_deg = 90.0;
        });
        let after = bb(&l, 1);
        // Turned a quarter about the middle (109.5, 45) of the letter box, then moved.
        assert!((after.width() - before.height()).abs() < 1e-9);
        assert!(after.center().distance(Point::new(110.5, 47.0)) < 1e-9, "{after:?}");
        assert_eq!(bb(&l, 0), square_box(100.0));
        // Back again: the edit is removed.
        l.change_letter(1, |e| {
            e.offset = Point::ZERO;
            e.rotate_deg = 0.0;
        });
        assert!(l.edits.is_empty());
        assert!(bb(&l, 1).min.distance(before.min) < 1e-9);
    }

    fn square_box(x: f64) -> emb_geom::BBox {
        emb_geom::BBox::new(Point::new(x, 40.0), Point::new(x + 5.0, 50.0))
    }

    #[test]
    fn edits_follow_text_changes() {
        let mut l = Lettering::new("Anna", FontRef::new("Lato", "Bold"), 10.0, Point::ZERO);
        for i in 0..4 {
            l.edit_mut(i).kern_mm = i as f64 + 1.0;
        }
        assert_eq!(l.edits.iter().map(|e| e.index).collect::<Vec<_>>(), [0, 1, 2, 3]);
        // "Anna" to "Annia": the changed middle loses its edits, the end moves.
        l.retarget_edits("Annia");
        assert_eq!(
            l.edits.iter().map(|e| (e.index, e.kern_mm)).collect::<Vec<_>>(),
            [(0, 1.0), (1, 2.0), (2, 3.0), (4, 4.0)]
        );
        l.text = "Annia".into();
        l.retarget_edits("Annika");
        assert_eq!(
            l.edits.iter().map(|e| (e.index, e.kern_mm)).collect::<Vec<_>>(),
            [(0, 1.0), (1, 2.0), (2, 3.0), (5, 4.0)]
        );
        l.text = "Annika".into();
        l.retarget_edits("Annika!");
        assert_eq!(l.edits.len(), 4);
        l.edit_mut(5).kern_mm = 0.0;
        l.prune_edits();
        assert_eq!(l.edits.len(), 3);
        l.reset_letter(0);
        assert_eq!(l.edits.len(), 2);
        assert!(l.edit(1).is_some() && l.edit(0).is_none());
    }

    #[test]
    fn serde_round_trip_and_defaults() {
        let mut l = Lettering::new("Hi\nthere", FontRef::new("Lato", "Regular"), 12.0, Point::new(1.0, 2.0));
        l.baseline = Baseline::Arc {
            radius_mm: 30.0,
            clockwise: false,
        };
        l.edit_mut(0).rotate_deg = 5.0;
        l.set_letters(vec![Letter {
            text: "H".into(),
            line: 0,
            index: 0,
            frame: Transform::IDENTITY,
            center: Point::ZERO,
            parts: vec![square(0.0)],
        }]);
        let kind = ObjectKind::Lettering(Box::new(l));
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(serde_json::from_str::<ObjectKind>(&json).unwrap(), kind);
        let min: Lettering =
            serde_json::from_str(r#"{"text":"A","font":{"family":"F","style":""},"height_mm":5}"#).unwrap();
        assert!(min.kerning && min.line_spacing == 1.0 && min.letters.is_empty());
        assert_eq!(min.lines(), ["A"]);
    }
}
