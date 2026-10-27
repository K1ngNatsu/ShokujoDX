//! TrueType and OpenType fonts: names, metrics, glyph outlines and kerning.
//!
//! Units are font units with Y down (the font's Y axis is flipped), so the
//! baseline is at Y = 0 and capitals are at negative Y.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use emb_geom::{Path, Point};
use emb_model::FontRef;
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
use ttf_parser::{Face, GlyphId, Tag};

use crate::glyph::GlyphShapes;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FontError {
    #[error("not a TrueType or OpenType font: {0}")]
    Parse(String),
}

/// One font face, with a cache of the glyph shapes made from it.
pub struct Font {
    data: Arc<Vec<u8>>,
    index: u32,
    pub family: String,
    pub style: String,
    pub units_per_em: f64,
    /// Height of the capitals (of "H").
    pub cap_height: f64,
    pub ascender: f64,
    /// Negative: below the baseline.
    pub descender: f64,
    pub line_gap: f64,
    cache: Mutex<HashMap<u16, Arc<GlyphShapes>>>,
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Font({} {})", self.family, self.style)
    }
}

/// The number of faces in a font file (a collection has more than one).
pub fn face_count(data: &[u8]) -> u32 {
    ttf_parser::fonts_in_collection(data).unwrap_or(1)
}

/// The best name for `ids` (in order of preference): English first, then any.
fn name(face: &Face, ids: &[u16]) -> Option<String> {
    for &id in ids {
        let mut any = None;
        for n in face.names() {
            if n.name_id != id {
                continue;
            }
            if let Some(s) = n.to_string().filter(|s| !s.trim().is_empty()) {
                if n.language_id == 0x0409 {
                    return Some(s);
                }
                any.get_or_insert(s);
            }
        }
        if any.is_some() {
            return any;
        }
    }
    None
}

/// The family and style names of a face, without reading its glyphs.
pub fn face_names(data: &[u8], index: u32) -> Option<(String, String)> {
    let face = Face::parse(data, index).ok()?;
    let family = name(&face, &[16, 1])?;
    let style = name(&face, &[17, 2]).unwrap_or_else(|| "Regular".to_string());
    Some((family, style))
}

impl Font {
    pub fn from_bytes(data: Vec<u8>, index: u32) -> Result<Font, FontError> {
        let data = Arc::new(data);
        let face = Face::parse(&data, index).map_err(|e| FontError::Parse(e.to_string()))?;
        // Typographic family and subfamily first (16, 17), then the legacy names (1, 2).
        let family = name(&face, &[16, 1]).unwrap_or_else(|| "Unnamed".to_string());
        let style = name(&face, &[17, 2]).unwrap_or_else(|| "Regular".to_string());
        let units_per_em = f64::from(face.units_per_em().max(1));
        let h_top = face
            .glyph_index('H')
            .and_then(|h| face.glyph_bounding_box(h))
            .map(|b| f64::from(b.y_max));
        let cap_height = cap_height_of(face.capital_height().map(f64::from), h_top, units_per_em);
        let (ascender, descender, line_gap) = (
            f64::from(face.ascender()),
            f64::from(face.descender()),
            f64::from(face.line_gap()),
        );
        Ok(Font {
            data,
            index,
            family,
            style,
            units_per_em,
            cap_height,
            ascender,
            descender,
            line_gap,
            cache: Mutex::new(HashMap::new()),
        })
    }

    fn face(&self) -> Face<'_> {
        Face::parse(&self.data, self.index).expect("parsed in from_bytes")
    }

    pub fn font_ref(&self) -> FontRef {
        FontRef::new(&self.family, &self.style)
    }

    /// The distance between two baselines.
    pub fn line_height(&self) -> f64 {
        let h = self.ascender - self.descender + self.line_gap;
        if h > 0.0 { h } else { 1.2 * self.units_per_em }
    }

    pub fn glyph(&self, c: char) -> Option<u16> {
        self.face().glyph_index(c).map(|g| g.0).filter(|&g| g != 0)
    }

    pub fn advance(&self, glyph: u16) -> f64 {
        f64::from(self.face().glyph_hor_advance(GlyphId(glyph)).unwrap_or(0))
    }

    /// The outline of a glyph as closed rings (flattened curves), Y down.
    pub fn contours(&self, glyph: u16) -> Vec<Vec<Point>> {
        let mut b = Builder::default();
        if self.face().outline_glyph(GlyphId(glyph), &mut b).is_none() {
            return Vec::new();
        }
        let tol = self.units_per_em / 2000.0;
        b.path
            .flatten(tol)
            .into_iter()
            .map(|pl| {
                let mut p = pl.points;
                if p.len() > 1 && p.first() == p.last() {
                    p.pop();
                }
                p
            })
            .filter(|p| p.len() >= 3)
            .collect()
    }

    /// The horizontal kerning between two glyphs (usually negative): the
    /// OpenType pair positioning of the "kern" feature, else the `kern` table.
    pub fn kerning(&self, left: u16, right: u16) -> f64 {
        let face = self.face();
        let (l, r) = (GlyphId(left), GlyphId(right));
        if let Some(v) = gpos_kerning(&face, l, r) {
            return f64::from(v);
        }
        face.tables()
            .kern
            .and_then(|k| {
                k.subtables
                    .into_iter()
                    .filter(|s| s.horizontal && !s.variable)
                    .find_map(|s| s.glyphs_kerning(l, r))
            })
            .map_or(0.0, f64::from)
    }

    /// The copyright notice of the font (name 0).
    pub fn copyright(&self) -> Option<String> {
        name(&self.face(), &[0])
    }

    /// The license description of the font (name 13).
    pub fn license(&self) -> Option<String> {
        name(&self.face(), &[13])
    }

    /// The typical stroke width as a part of the capital height: the middle
    /// value over a few plain letters.
    pub fn stroke_ratio(&self) -> Option<f64> {
        let mut v: Vec<f64> = "IlHnoiu"
            .chars()
            .filter_map(|c| self.glyph(c))
            .map(|g| self.shapes(g).strokes.typical_width)
            .filter(|w| *w > 0.0)
            .collect();
        v.sort_by(f64::total_cmp);
        Some(*v.get(v.len() / 2)? / self.cap_height)
    }

    /// The shapes of a glyph (cached).
    pub fn shapes(&self, glyph: u16) -> Arc<GlyphShapes> {
        if let Some(s) = self.cache.lock().ok().and_then(|c| c.get(&glyph).cloned()) {
            return s;
        }
        let s = Arc::new(GlyphShapes::new(&self.contours(glyph), self.cap_height));
        if let Ok(mut c) = self.cache.lock() {
            c.insert(glyph, s.clone());
        }
        s
    }
}

/// The x-advance change of the first glyph of a pair in the lookups of the
/// "kern" feature. `None` if no subtable has the pair.
fn gpos_kerning(face: &Face, left: GlyphId, right: GlyphId) -> Option<i16> {
    let gpos = face.tables().gpos?;
    let kern = Tag::from_bytes(b"kern");
    let mut seen = Vec::new();
    for feature in gpos.features.into_iter().filter(|f| f.tag == kern) {
        for index in feature.lookup_indices {
            if seen.contains(&index) {
                continue;
            }
            seen.push(index);
            let Some(lookup) = gpos.lookups.get(index) else {
                continue;
            };
            for k in 0..lookup.subtables.len() {
                let Some(PositioningSubtable::Pair(pair)) = lookup.subtables.get::<PositioningSubtable>(k) else {
                    continue;
                };
                match pair {
                    PairAdjustment::Format1 { coverage, sets } => {
                        if let Some(set) = coverage.get(left).and_then(|i| sets.get(i))
                            && let Some((v, _)) = set.get(right)
                        {
                            return Some(v.x_advance);
                        }
                    }
                    PairAdjustment::Format2 {
                        coverage,
                        classes,
                        matrix,
                    } => {
                        if coverage.contains(left)
                            && let Some((v, _)) = matrix.get((classes.0.get(left), classes.1.get(right)))
                        {
                            return Some(v.x_advance);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Collects a glyph outline as a path with Y down.
#[derive(Default)]
struct Builder {
    path: Path,
    last: Point,
}

fn pt(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), -f64::from(y))
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).move_to(self.last);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).line_to(self.last);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // The cubic with the same curve as the quadratic.
        let (p0, q, p) = (self.last, pt(x1, y1), pt(x, y));
        let c1 = p0 + (q - p0) * (2.0 / 3.0);
        let c2 = p + (q - p) * (2.0 / 3.0);
        self.last = p;
        self.path = std::mem::take(&mut self.path).cubic_to(c1, c2, p);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).cubic_to(pt(x1, y1), pt(x2, y2), self.last);
    }

    fn close(&mut self) {
        self.path = std::mem::take(&mut self.path).close();
    }
}

/// The height of the capitals in font units: the value of the font (OS/2),
/// else the top of 'H', else 0.7 em. Some fonts have a wrong value (for
/// example 0.15 em): a value out of 0.3 to 1.2 em is not used.
fn cap_height_of(declared: Option<f64>, h_top: Option<f64>, units_per_em: f64) -> f64 {
    let ok = |v: &f64| (0.3 * units_per_em..=1.2 * units_per_em).contains(v);
    declared
        .filter(ok)
        .or_else(|| h_top.filter(ok))
        .unwrap_or(0.7 * units_per_em)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_wrong_cap_height_is_not_used() {
        use super::cap_height_of;
        assert_eq!(cap_height_of(Some(700.0), Some(710.0), 1000.0), 700.0);
        // A font that declares 301 of 2048 units (as Courgette): the 'H'.
        assert_eq!(cap_height_of(Some(301.0), Some(1434.0), 2048.0), 1434.0);
        assert_eq!(cap_height_of(Some(0.0), None, 1000.0), 700.0);
        assert_eq!(cap_height_of(None, Some(5000.0), 1000.0), 700.0);
    }

    use crate::bundled;

    #[test]
    fn names_metrics_and_outlines() {
        let f = bundled::font("Lato", "Bold").unwrap();
        assert_eq!((f.family.as_str(), f.style.as_str()), ("Lato", "Bold"));
        assert!(f.cap_height > 0.6 * f.units_per_em && f.cap_height < 0.8 * f.units_per_em);
        let o = f.glyph('O').unwrap();
        assert_eq!(f.contours(o).len(), 2, "outer ring and counter");
        // Capitals are above the baseline: negative Y.
        let top = f
            .contours(f.glyph('H').unwrap())
            .iter()
            .flatten()
            .map(|p| p.y)
            .fold(0.0, f64::min);
        assert!((top + f.cap_height).abs() < 2.0, "{top}");
        assert!(f.advance(o) > 0.0);
        assert!(f.glyph('\u{10FFFF}').is_none());
    }

    #[test]
    fn kerning_pairs() {
        let f = bundled::font("Lato", "Regular").unwrap();
        let (a, v) = (f.glyph('A').unwrap(), f.glyph('V').unwrap());
        assert!(f.kerning(a, v) < 0.0, "A V kerns closer: {}", f.kerning(a, v));
        let (h, i) = (f.glyph('H').unwrap(), f.glyph('I').unwrap());
        assert_eq!(f.kerning(h, i), 0.0);
    }
}
