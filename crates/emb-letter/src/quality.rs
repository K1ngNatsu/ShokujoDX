//! The quality score of an embroidery font (P17 Milestone 7, ADR 0115): a
//! number from 0 to 100 that says how well the letters of the font sew,
//! before the user sews them.
//!
//! The score comes from three measures of each letter. They need the font
//! that the embroidery font was converted from, so the converter makes the
//! score and the font file keeps it ([`crate::EmbFont::quality`]):
//!
//! - **Shape:** the part of the outline of the source font that the parts
//!   of the letter cover. A letter for which the stroke finder found no
//!   columns sews as a fill (ADR 0100); it has its full shape, but not the
//!   look of the other letters, so its shape counts as [`FILL_LETTER`].
//! - **Narrow:** the part of the length of the satin columns that is too
//!   narrow to sew well at the smallest size of the font.
//! - **Long:** the part of the length of the satin columns whose stitches
//!   are too long at the largest size of the font.
//!
//! A fill font ([`crate::EmbFont::fill`], ADR 0117) has no columns. Its
//! narrow measure is the part of the area of a letter that is too narrow
//! for a fill at the smallest size, and each hole that
//! is too narrow to stay open counts as [`HOLE_CLOSED`]. It has no long
//! stitches: the rows of a fill have stitches of a set length.

use emb_geom::{BBox, Path, Point};
use emb_model::{ObjectKind, SatinGeometry};
use serde::{Deserialize, Serialize};

use crate::embfont::{EmbFont, EmbLetter};
use crate::font::Font;
use crate::grid::{self, Grid};
use crate::sheet::{Characters, characters};
use crate::source::{MIN_FILL_MM, MIN_SATIN_MM, SMALL_TEXT_MIN_SATIN_MM};
use crate::tune::{self, Outline};

/// The cell of the coverage grid, in mm at the design size of the font.
const CELL_MM: f64 = 0.1;
/// The distance between two measures of the width of a column, in mm at
/// the design size.
const STEP_MM: f64 = 0.25;
/// A column is too narrow below this part of the narrowest column of the
/// size range ([`MIN_SATIN_MM`], or [`SMALL_TEXT_MIN_SATIN_MM`] for a font
/// for small text): 0.8 mm, or 0.53 mm. The thin strokes of a plain font
/// (the bar of an "e") are about three quarters of its stems, so they are
/// not too narrow.
pub const NARROW: f64 = 2.0 / 3.0;
/// The cell of the grid that measures the widths of fill letters, in mm at
/// the design size of the font.
const FILL_CELL_MM: f64 = 0.25;
/// The narrowest hole of a fill letter that stays open, in mm. The pull of
/// the stitches and the width of the thread close a narrower hole.
pub const MIN_HOLE_MM: f64 = 1.0;
/// A hole of a fill letter that is too narrow at the smallest size counts
/// as this part of the letter too narrow: the letter is harder to read.
pub const HOLE_CLOSED: f64 = 0.15;
/// The size range of a fill font leaves at most this part of the area of
/// its letters narrower than [`MIN_FILL_MM`] at its smallest size. It is
/// smaller than [`RANGE_TAIL`]: the hairlines of a serif font are a small
/// part of its area (5 to 10 %), and the tips of the sharp corners, which
/// are narrow at each size, are less than 1 % of the area of a heavy font.
pub const FILL_TAIL: f64 = 0.05;
/// No size range of a fill font starts below this capital height, in mm.
/// A smaller letter has few rows of stitches and a rough edge; satin
/// letters look better there.
pub const FILL_MIN_HEIGHT_MM: f64 = 15.0;
/// A satin stitch longer than this is too long, in mm: the longest stitch
/// of the common machine formats is 12.1 mm, and a longer stitch is loose.
pub const LONG_MM: f64 = 12.0;
/// The shape value (0 to 1) of a letter that sews as a fill.
pub const FILL_LETTER: f64 = 0.5;
/// The shape value of a letter with columns is 0 at this coverage and
/// below, and 1 at [`FULL`] and above. Below [`EMPTY`] the converter does
/// not use the columns of the stroke finder
/// ([`crate::glyph::AUTO_MIN_COVERAGE`]); a letter with a tenth of its
/// shape not sewn has a gap that the eye sees. Some cells at the edges are
/// always open, so no letter has all of its outline.
pub const EMPTY: f64 = crate::glyph::AUTO_MIN_COVERAGE;
pub const FULL: f64 = 0.97;
/// How much the narrow and the long parts of the columns take from the
/// score of a letter: a letter with this part of its columns too narrow
/// (or too long) loses all of its score. Long stitches take less: the
/// stitch engine splits them, so the letter sews, with another look.
pub const NARROW_ALL: f64 = 0.5;
pub const LONG_ALL: f64 = 1.0;
/// The size range of a converted font leaves at most this part of the
/// length of its columns too narrow at its smallest size, and at most this
/// part too long at its largest size ([`crate::EmbFont::fit_size_range`]).
pub const RANGE_TAIL: f64 = 0.1;
/// A bundled font has at least this score; a font below it is not in the
/// program and not in a font package. On the proof pictures, the fonts
/// below this score have letters with gaps that the eye sees.
pub const LIMIT: u8 = 65;
/// A new font is in the program if it has at least this score; a font
/// from [`LIMIT`] to this score is in a font package.
pub const BUNDLE: u8 = 85;

/// The quality of an embroidery font.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quality {
    /// 0 (does not sew well) to 100.
    pub score: u8,
    /// The mean part of the outlines of the letters that the parts cover
    /// (0 to 1).
    pub coverage: f64,
    /// The mean part of the columns that is too narrow at the smallest
    /// size (0 to 1).
    pub narrow: f64,
    /// The mean part of the columns whose stitches are too long at the
    /// largest size (0 to 1).
    pub long: f64,
    /// The letters that sew as a fill, as a part of the measured letters.
    #[serde(default)]
    pub fills: f64,
}

impl Quality {
    /// `true` if a font with this quality can be a bundled font.
    pub fn passes(&self) -> bool {
        self.score >= LIMIT
    }
}

/// The measures of one letter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LetterQuality {
    /// The part of the outline that the parts cover.
    pub coverage: f64,
    pub narrow: f64,
    pub long: f64,
    /// The letter has no satin column, in a font of satin letters. The
    /// letters of a fill font do not have this mark.
    pub fill: bool,
}

impl LetterQuality {
    /// The score of the letter, 0 to 1.
    pub fn score(&self) -> f64 {
        let shape = if self.fill {
            FILL_LETTER
        } else {
            ((self.coverage - EMPTY) / (FULL - EMPTY)).clamp(0.0, 1.0)
        };
        let narrow = (1.0 - self.narrow / NARROW_ALL).clamp(0.0, 1.0);
        let long = (1.0 - self.long / LONG_ALL).clamp(0.0, 1.0);
        shape * narrow * long
    }
}

fn flat(path: &Path) -> Vec<Point> {
    path.flatten(0.05).into_iter().flat_map(|pl| pl.points).collect()
}

/// A rail as points with the length along it at each point.
struct Rail {
    pts: Vec<Point>,
    at: Vec<f64>,
}

impl Rail {
    fn new(path: &Path) -> Option<Rail> {
        let pts = flat(path);
        if pts.len() < 2 {
            return None;
        }
        let mut at = vec![0.0];
        for w in pts.windows(2) {
            at.push(at[at.len() - 1] + w[0].distance(w[1]));
        }
        Some(Rail { pts, at })
    }

    fn len(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    /// The point at the length `l` along the rail.
    fn point(&self, l: f64) -> Point {
        let i = self.at.partition_point(|a| *a <= l).clamp(1, self.pts.len() - 1);
        let seg = self.at[i] - self.at[i - 1];
        let f = if seg > 0.0 {
            ((l - self.at[i - 1]) / seg).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.pts[i - 1] + (self.pts[i] - self.pts[i - 1]) * f
    }

    /// The length along the rail of the point of it nearest to `p`.
    fn project(&self, p: Point) -> f64 {
        let mut best = (f64::MAX, 0.0);
        for (i, w) in self.pts.windows(2).enumerate() {
            let d = w[1] - w[0];
            let n = d.x * d.x + d.y * d.y;
            let f = if n > 0.0 {
                (((p.x - w[0].x) * d.x + (p.y - w[0].y) * d.y) / n).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dist = (w[0] + d * f).distance(p);
            if dist < best.0 {
                best = (dist, self.at[i] + f * (self.at[i + 1] - self.at[i]));
            }
        }
        best.1
    }
}

/// The widths of a column along its length: (width, length of the piece of
/// the column with this width), in the units of the column. The stitches go
/// between the points of the two rails that the rungs pair.
pub fn column_widths(g: &SatinGeometry, step: f64) -> Vec<(f64, f64)> {
    let (Some(a), Some(b)) = (Rail::new(&g.rail_a), Rail::new(&g.rail_b)) else {
        return Vec::new();
    };
    let mut inner: Vec<(f64, f64)> = g
        .rungs
        .iter()
        .map(|(pa, pb)| (a.project(*pa), b.project(*pb)))
        .collect();
    inner.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut rungs = vec![(0.0, 0.0)];
    for r in inner {
        // A rung that would cross the rung before it is not used.
        let last = rungs[rungs.len() - 1];
        if r.0 > last.0 + 1e-6 && r.1 > last.1 + 1e-6 && r.0 < a.len() - 1e-6 && r.1 < b.len() - 1e-6 {
            rungs.push(r);
        }
    }
    rungs.push((a.len(), b.len()));
    let mut out = Vec::new();
    for w in rungs.windows(2) {
        let (la, lb) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let mid = (la + lb) / 2.0;
        let n = (mid / step.max(1e-6)).ceil().max(1.0) as usize;
        for q in 0..n {
            let f = (q as f64 + 0.5) / n as f64;
            let width = a.point(w[0].0 + f * la).distance(b.point(w[0].1 + f * lb));
            out.push((width, mid / n as f64));
        }
    }
    out
}

/// The local width of each inside cell of `g`, in cells: the diameter of
/// the largest circle in the area that has the cell in it (0 outside).
fn local_width(g: &Grid) -> Vec<f64> {
    let d = grid::distance(g);
    let mut radius = vec![0.0f64; d.len()];
    for (idx, &here) in d.iter().enumerate() {
        if here <= 0.0 {
            continue;
        }
        let (i, j) = (idx % g.w, idx / g.w);
        // The circle of a cell is in the circle of a neighbor that is
        // one step farther from the edge: such a cell draws no circle.
        let slope = g.neighbors(idx).any(|n| {
            let step = if n % g.w != i && n / g.w != j {
                std::f64::consts::SQRT_2
            } else {
                1.0
            };
            d[n] >= here + step - 1e-6
        });
        if slope {
            continue;
        }
        let reach = here.ceil() as isize;
        for dj in -reach..=reach {
            let b = j as isize + dj;
            if b < 0 || b as usize >= g.h {
                continue;
            }
            for di in -reach..=reach {
                let a = i as isize + di;
                if a < 0 || a as usize >= g.w || ((di * di + dj * dj) as f64) >= here * here {
                    continue;
                }
                let k = b as usize * g.w + a as usize;
                if radius[k] < here {
                    radius[k] = here;
                }
            }
        }
    }
    radius
        .iter()
        .zip(&d)
        .map(|(r, d)| {
            if *d > 0.0 {
                // The distance is from the center of a cell to the center
                // of the nearest outside cell.
                (2.0 * r.max(*d) - 1.0).max(1.0)
            } else {
                0.0
            }
        })
        .collect()
}

/// The widths of the fill parts of a letter, in the units of the parts
/// (mm at the design size of the font).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FillWidths {
    /// The strokes: (local width, area with this width). The local width
    /// of a place is the diameter of the largest circle in the letter that
    /// has the place in it.
    pub strokes: Vec<(f64, f64)>,
    /// The width of each hole: the diameter of the largest circle in it.
    pub holes: Vec<f64>,
}

/// The widths of the strokes and of the holes of the fill parts of
/// `parts`.
pub fn fill_widths(parts: &[ObjectKind]) -> FillWidths {
    let fills: Vec<&ObjectKind> = parts.iter().filter(|p| matches!(p, ObjectKind::Fill { .. })).collect();
    let Some(bounds) = fills.iter().filter_map(|p| p.control_bbox()).reduce(BBox::union) else {
        return FillWidths::default();
    };
    let mut g = Grid::new(bounds, FILL_CELL_MM, 2);
    for part in &fills {
        let mask = tune::part_mask(&g, part);
        for (cell, m) in g.inside.iter_mut().zip(mask) {
            *cell |= m;
        }
    }
    let area = g.inside.clone();
    let strokes = local_width(&g)
        .into_iter()
        .filter(|w| *w > 0.0)
        .map(|w| (w * FILL_CELL_MM, FILL_CELL_MM * FILL_CELL_MM))
        .collect();
    let mut holes = Vec::new();
    for part in &fills {
        let ObjectKind::Fill { holes: rings, .. } = part else {
            continue;
        };
        for ring in rings {
            let pts: Vec<Vec<Point>> = ring.flatten(0.05).into_iter().map(|pl| pl.points).collect();
            let mut mask = vec![false; g.w * g.h];
            grid::fill_into(&g, &pts, &mut mask);
            // A part in the hole (an island) is not hole.
            for (m, a) in mask.iter_mut().zip(&area) {
                *m &= !*a;
            }
            g.inside = mask;
            let far = grid::distance(&g).into_iter().fold(0.0, f64::max);
            // A hole with no cell center in it is narrower than a cell.
            holes.push(if far > 0.0 {
                (2.0 * far - 1.0).max(1.0) * FILL_CELL_MM
            } else {
                0.5 * FILL_CELL_MM
            });
        }
    }
    FillWidths { strokes, holes }
}

/// The outline of the letter `c` of the source font, in mm at the design
/// size of `font`: outer rings with their holes.
pub fn outline(font: &EmbFont, source: &Font, c: char) -> Vec<(Vec<Point>, Vec<Vec<Point>>)> {
    let Some(g) = source.glyph(c) else {
        return Vec::new();
    };
    let s = font.cap_height_mm / source.cap_height;
    let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect() };
    source
        .shapes(g)
        .fills
        .iter()
        .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect()))
        .collect()
}

/// The part of `outline` that the parts of a letter cover (0 to 1). `None`
/// with no outline.
pub fn coverage(parts: &[ObjectKind], outline: &Outline) -> Option<f64> {
    let bounds = parts
        .iter()
        .filter_map(|p| p.control_bbox())
        .chain(BBox::from_points(
            outline
                .iter()
                .flat_map(|(o, h)| o.iter().chain(h.iter().flatten()))
                .copied(),
        ))
        .reduce(BBox::union)?;
    if outline.is_empty() {
        return None;
    }
    let grid = Grid::new(bounds, CELL_MM, 2);
    let mut cover = vec![false; grid.w * grid.h];
    for part in parts {
        for (c, m) in cover.iter_mut().zip(tune::part_mask(&grid, part)) {
            *c |= m;
        }
    }
    let glyph = tune::outline_mask(&grid, outline);
    let area = glyph.iter().filter(|g| **g).count();
    if area == 0 {
        return None;
    }
    let both = glyph.iter().zip(&cover).filter(|(g, c)| **g && **c).count();
    Some(both as f64 / area as f64)
}

/// The narrowest column that sews well at the smallest size of `font`, in
/// mm: below it a column is too narrow.
pub fn narrow_mm(font: &EmbFont) -> f64 {
    NARROW
        * if font.small_text {
            SMALL_TEXT_MIN_SATIN_MM
        } else {
            MIN_SATIN_MM
        }
}

/// The measures of a letter of `font` with the outline `outline` of the
/// source font. `None` for a letter with no parts or no outline.
pub fn letter_quality(font: &EmbFont, letter: &EmbLetter, outline: &Outline) -> Option<LetterQuality> {
    let cover = coverage(&letter.parts, outline)?;
    let (small, large) = (
        font.min_height_mm / font.cap_height_mm.max(1e-6),
        font.max_height_mm / font.cap_height_mm.max(1e-6),
    );
    if font.fill {
        // A fill font: the widths of the strokes and of the holes.
        let widths = fill_widths(&letter.parts);
        let total: f64 = widths.strokes.iter().map(|w| w.1).sum();
        if total > 0.0 {
            let thin: f64 = widths
                .strokes
                .iter()
                .filter(|w| w.0 * small < NARROW * MIN_FILL_MM)
                .map(|w| w.1)
                .sum();
            let closed = widths
                .holes
                .iter()
                .filter(|w| **w * small < NARROW * MIN_HOLE_MM)
                .count();
            return Some(LetterQuality {
                coverage: cover,
                narrow: (thin / total + HOLE_CLOSED * closed as f64).min(1.0),
                long: 0.0,
                fill: false,
            });
        }
    }
    let limit = narrow_mm(font);
    let (mut total, mut narrow, mut long) = (0.0, 0.0, 0.0);
    for part in &letter.parts {
        let ObjectKind::Satin { geometry, .. } = part else {
            continue;
        };
        for (width, len) in column_widths(geometry, STEP_MM) {
            total += len;
            if width * small < limit {
                narrow += len;
            }
            if width * large > LONG_MM {
                long += len;
            }
        }
    }
    let fill = total <= 0.0;
    Some(LetterQuality {
        coverage: cover,
        narrow: if fill { 0.0 } else { narrow / total },
        long: if fill { 0.0 } else { long / total },
        fill,
    })
}

/// The characters that the score measures: the capital letters, the small
/// letters and the digits. Marks have thin strokes in most fonts (the
/// arms of "*"), and a text has few of them.
pub fn measured() -> Vec<char> {
    [Characters::Capitals, Characters::Lowercase]
        .iter()
        .flat_map(characters)
        .chain(crate::sheet::DIGITS.chars())
        .collect()
}

/// The measures of the characters of [`measured`] that have parts in
/// `font`. `source` is the font that `font` was converted from.
pub fn letters(font: &EmbFont, source: &Font) -> Vec<(char, LetterQuality)> {
    measured()
        .into_iter()
        .filter_map(|c| {
            let letter = font.letters.get(&c).filter(|l| !l.parts.is_empty())?;
            Some((c, letter_quality(font, letter, &outline(font, source, c))?))
        })
        .collect()
}

/// The widths of the holes of the measured letters of a fill font, in mm
/// at the design size: the width below which `part` (0 to 1) of the holes
/// are. `None` with no hole.
pub fn hole_width_at(font: &EmbFont, part: f64) -> Option<f64> {
    let mut all: Vec<f64> = measured()
        .into_iter()
        .filter_map(|c| font.letters.get(&c))
        .flat_map(|l| fill_widths(&l.parts).holes)
        .collect();
    all.sort_by(f64::total_cmp);
    let at = ((part * all.len() as f64).floor() as usize).min(all.len().checked_sub(1)?);
    Some(all[at])
}

/// The widths of the columns of the measured letters of `font`, in mm at
/// the design size: the width below which `part` (0 to 1) of the length of
/// the columns is. For a fill font: the local width below which `part` of
/// the area of its fill letters is. `None` with no column.
pub fn width_at(font: &EmbFont, part: f64) -> Option<f64> {
    let letters = measured().into_iter().filter_map(|c| font.letters.get(&c));
    let mut all: Vec<(f64, f64)> = if font.fill {
        letters.flat_map(|l| fill_widths(&l.parts).strokes).collect()
    } else {
        letters
            .flat_map(|l| l.parts.iter())
            .filter_map(|p| match p {
                ObjectKind::Satin { geometry, .. } => Some(column_widths(geometry, STEP_MM)),
                _ => None,
            })
            .flatten()
            .collect()
    };
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = all.iter().map(|x| x.1).sum();
    let mut sum = 0.0;
    for (width, len) in &all {
        sum += len;
        if sum >= part * total {
            return Some(*width);
        }
    }
    None
}

/// The quality from the measures of the letters. `None` with no letters.
pub fn of_letters(rows: &[(char, LetterQuality)]) -> Option<Quality> {
    if rows.is_empty() {
        return None;
    }
    let n = rows.len() as f64;
    let mean = |f: &dyn Fn(&LetterQuality) -> f64| rows.iter().map(|(_, q)| f(q)).sum::<f64>() / n;
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    Some(Quality {
        score: (100.0 * mean(&|q| q.score())).round().clamp(0.0, 100.0) as u8,
        coverage: round(mean(&|q| q.coverage)),
        narrow: round(mean(&|q| q.narrow)),
        long: round(mean(&|q| q.long)),
        fills: round(mean(&|q| f64::from(u8::from(q.fill)))),
    })
}

/// The quality of `font`, which was converted from `source`. `None` if the
/// font has none of the basic characters.
pub fn measure(font: &EmbFont, source: &Font) -> Option<Quality> {
    of_letters(&letters(font, source))
}

#[cfg(test)]
mod tests {
    use emb_model::LetterStitch;

    use super::*;
    use crate::bundled;
    use crate::embfont::convert;

    fn line(pts: &[Point]) -> Path {
        pts[1..]
            .iter()
            .fold(Path::new().move_to(pts[0]), |path, p| path.line_to(*p))
    }

    fn column(width: f64, len: f64) -> SatinGeometry {
        SatinGeometry {
            rail_a: line(&[Point::new(0.0, 0.0), Point::new(0.0, len)]),
            rail_b: line(&[Point::new(width, 0.0), Point::new(width, len)]),
            rungs: Vec::new(),
        }
    }

    #[test]
    fn the_widths_of_a_column() {
        let w = column_widths(&column(2.0, 10.0), 0.25);
        assert_eq!(w.len(), 40);
        assert!(
            w.iter()
                .all(|(width, len)| (width - 2.0).abs() < 1e-9 && (len - 0.25).abs() < 1e-9)
        );
        // A column that gets wider: 1 mm at its start, 3 mm at its end.
        let g = SatinGeometry {
            rail_a: line(&[Point::new(0.0, 0.0), Point::new(0.0, 10.0)]),
            rail_b: line(&[Point::new(1.0, 0.0), Point::new(3.0, 10.0)]),
            rungs: Vec::new(),
        };
        let w = column_widths(&g, 0.25);
        assert!(w[0].0 < 1.1 && w[w.len() - 1].0 > 2.9, "{:?}", (w[0], w[w.len() - 1]));
        let below_two: f64 = w.iter().filter(|x| x.0 < 2.0).map(|x| x.1).sum();
        let all: f64 = w.iter().map(|x| x.1).sum();
        assert!((below_two / all - 0.5).abs() < 0.05, "{below_two} of {all}");
        // A rung sets which points of the rails the stitches join.
        let mut g = column(2.0, 10.0);
        g.rungs = vec![(Point::new(0.0, 2.0), Point::new(2.0, 8.0))];
        let w = column_widths(&g, 0.25);
        assert!(w.iter().any(|x| x.0 > 4.0), "the stitches slant");
    }

    #[test]
    fn the_score_of_a_letter() {
        let q = |coverage, narrow, long| LetterQuality {
            coverage,
            narrow,
            long,
            fill: false,
        };
        assert!((q(1.0, 0.0, 0.0).score() - 1.0).abs() < 1e-9);
        assert!((q(FULL, 0.0, 0.0).score() - 1.0).abs() < 1e-9);
        assert_eq!(q(EMPTY, 0.0, 0.0).score(), 0.0);
        assert_eq!(q(0.3, 0.0, 0.0).score(), 0.0);
        assert!((q(0.91, 0.0, 0.0).score() - 0.5).abs() < 1e-9);
        let fill = LetterQuality {
            coverage: 1.0,
            narrow: 0.0,
            long: 0.0,
            fill: true,
        };
        assert_eq!(fill.score(), FILL_LETTER);
        // Narrow columns and long stitches take from the score.
        assert!((q(1.0, 0.25, 0.0).score() - 0.5).abs() < 1e-9);
        assert!((q(1.0, 0.0, 0.25).score() - 0.75).abs() < 1e-9);
        assert_eq!(q(1.0, 0.6, 0.0).score(), 0.0);
        // A better coverage is a better score.
        assert!(q(0.95, 0.0, 0.0).score() > q(0.9, 0.0, 0.0).score());
        assert!(of_letters(&[]).is_none());
        let font = of_letters(&[('a', q(1.0, 0.0, 0.0)), ('b', q(0.91, 0.5, 0.0))]).unwrap();
        assert_eq!(font.score, 50);
        assert!((font.coverage - 0.955).abs() < 1e-9 && (font.narrow - 0.25).abs() < 1e-9);
        assert!(!font.passes());
    }

    #[test]
    fn the_quality_of_a_converted_font() {
        let source = bundled::font("Lato", "Bold").unwrap();
        let chars: Vec<char> = "HIiLo3".chars().collect();
        let font = convert(&source, "Block Sans", "", LetterStitch::Auto, &chars);
        let rows = letters(&font, &source);
        assert_eq!(rows.len(), 6);
        let by = |c: char| rows.iter().find(|r| r.0 == c).unwrap().1;
        // "I" is one even column that covers its outline.
        let i = by('I');
        assert!(
            i.coverage > 0.97 && i.narrow == 0.0 && i.long == 0.0 && !i.fill,
            "{i:?}"
        );
        assert!(i.score() > 0.99);
        let q = measure(&font, &source).unwrap();
        assert!(q.score >= LIMIT && q.coverage > 0.9, "{q:?}");
        assert_eq!(font.quality, Some(q), "the converter writes the quality");
        // The quality stays in the file.
        let back = EmbFont::from_json(&font.to_json()).unwrap();
        assert_eq!(back.quality, Some(q));

        // A column that is too narrow at the smallest size, and one whose
        // stitches are too long at the largest size.
        let mut thin = font.clone();
        thin.min_height_mm = 1.0;
        let l = letter_quality(&thin, &thin.letters[&'I'], &outline(&thin, &source, 'I')).unwrap();
        assert!(l.narrow > 0.99 && l.long == 0.0, "{l:?}");
        assert_eq!(l.score(), 0.0);
        let mut wide = font.clone();
        wide.max_height_mm = 200.0;
        let l = letter_quality(&wide, &wide.letters[&'I'], &outline(&wide, &source, 'I')).unwrap();
        assert!(l.long > 0.99 && l.narrow == 0.0, "{l:?}");
        // A font for small text can have narrower columns.
        assert!((narrow_mm(&font) - 0.8).abs() < 1e-9);
        thin.small_text = true;
        assert!((narrow_mm(&thin) - 0.8 * 2.0 / 3.0).abs() < 1e-9);

        // A letter that sews as a fill in a font of satin letters counts
        // as a fill letter.
        let mut fills = convert(&source, "Fills", "", LetterStitch::Fill, &chars);
        assert!(fills.fill, "the converter marks a fill font");
        fills.fill = false;
        let rows = letters(&fills, &source);
        assert!(rows.iter().all(|r| r.1.fill && r.1.coverage > 0.97), "{rows:?}");
        let q = of_letters(&rows).unwrap();
        assert_eq!(q.fills, 1.0);
        assert!(q.score < LIMIT, "{q:?}");
    }

    fn fill(outer: BBox, holes: &[BBox]) -> ObjectKind {
        ObjectKind::Fill {
            outer: Path::rect(outer),
            holes: holes.iter().map(|h| Path::rect(*h)).collect(),
            params: emb_model::FillParams::default(),
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BBox {
        BBox::new(Point::new(x0, y0), Point::new(x1, y1))
    }

    #[test]
    fn the_widths_of_fill_letters() {
        // A bar 5 mm wide and 20 mm long, with a square hole of 2 mm.
        let w = fill_widths(&[fill(rect(0.0, 0.0, 5.0, 20.0), &[rect(1.5, 9.0, 3.5, 11.0)])]);
        let area: f64 = w.strokes.iter().map(|x| x.1).sum();
        assert!((area - 96.0).abs() < 3.0, "{area}");
        // Most of the bar is 5 mm wide. Next to the hole it is 1.5 mm
        // wide, and the tips of the corners are narrow.
        let part = |from: f64, to: f64| -> f64 {
            w.strokes
                .iter()
                .filter(|x| x.0 >= from && x.0 < to)
                .map(|x| x.1)
                .sum::<f64>()
                / area
        };
        assert!(part(4.5, 5.5) > 0.8, "{}", part(4.5, 5.5));
        assert!(part(1.0, 2.0) > 0.03 && part(1.0, 2.0) < 0.12, "{}", part(1.0, 2.0));
        assert!(part(0.0, 1.0) < 0.02, "{}", part(0.0, 1.0));
        assert_eq!(w.holes.len(), 1);
        assert!((w.holes[0] - 2.0).abs() < 0.3, "{:?}", w.holes);
        // Parts that are not fills have no widths.
        assert_eq!(fill_widths(&[]), FillWidths::default());
    }

    #[test]
    fn the_quality_of_a_fill_font() {
        let source = bundled::font("Anton", "Regular").unwrap();
        let chars: Vec<char> = "HILoae38".chars().collect();
        let font = convert(&source, "Heavy Fill", "", LetterStitch::Fill, &chars);
        assert!(font.fill);
        // The range starts at the smallest size of fill fonts and has no
        // largest size of its own.
        assert_eq!(
            (font.min_height_mm, font.max_height_mm),
            (FILL_MIN_HEIGHT_MM, crate::source::MAX_HEIGHT_MM)
        );
        let rows = letters(&font, &source);
        assert_eq!(rows.len(), chars.len());
        for (c, l) in &rows {
            assert!(!l.fill && l.long == 0.0 && l.coverage > 0.97, "{c}: {l:?}");
            assert!(l.narrow < 0.1 && l.score() > 0.8, "{c}: {l:?}");
        }
        let q = font.quality.unwrap();
        assert!(q.score >= BUNDLE && q.fills == 0.0 && q.long == 0.0, "{q:?}");
        // A size at which the strokes are too narrow for a fill, and the
        // holes close.
        let mut small = font.clone();
        small.min_height_mm = 2.0;
        let l = letter_quality(&small, &small.letters[&'I'], &outline(&small, &source, 'I')).unwrap();
        assert!(l.narrow > 0.99, "{l:?}");
        let (o, i) = (
            letter_quality(&font, &font.letters[&'o'], &outline(&font, &source, 'o')).unwrap(),
            letter_quality(&small, &small.letters[&'o'], &outline(&small, &source, 'o')).unwrap(),
        );
        assert!(o.narrow < HOLE_CLOSED && i.narrow >= 0.99, "{o:?} {i:?}");
        // A font with thin strokes has a range that starts higher: 5 % of
        // the area of its letters can be narrower than 2 mm.
        let thin = bundled::font("Lato", "Regular").unwrap();
        let thin = convert(&thin, "Thin Fill", "", LetterStitch::Fill, &chars);
        assert!(thin.min_height_mm > 18.0, "{}", thin.min_height_mm);
        let w = width_at(&thin, FILL_TAIL).unwrap() * thin.min_height_mm / thin.cap_height_mm;
        assert!((MIN_FILL_MM - 0.1..MIN_FILL_MM + 0.3).contains(&w), "{w}");
        // A hole that is too narrow counts for the letter.
        small.min_height_mm = FILL_MIN_HEIGHT_MM;
        let mut holed = small.letters[&'I'].clone();
        let Some(b) = holed.parts[0].control_bbox() else {
            panic!()
        };
        let c = b.center();
        if let ObjectKind::Fill { holes, .. } = &mut holed.parts[0] {
            holes.push(Path::rect(rect(c.x - 0.4, c.y - 0.4, c.x + 0.4, c.y + 0.4)));
        }
        let plain = letter_quality(&small, &small.letters[&'I'], &outline(&small, &source, 'I')).unwrap();
        let with_hole = letter_quality(&small, &holed, &outline(&small, &source, 'I')).unwrap();
        assert!(
            with_hole.narrow >= plain.narrow + HOLE_CLOSED,
            "{plain:?} {with_hole:?}"
        );
        assert_eq!(hole_width_at(&small, 0.0), hole_width_at(&font, 0.0));
    }

    #[test]
    fn a_letter_with_a_part_not_sewn_has_a_lower_score() {
        let source = bundled::font("Lato", "Bold").unwrap();
        let font = convert(&source, "Block Sans", "", LetterStitch::Auto, &['H']);
        let full = letter_quality(&font, &font.letters[&'H'], &outline(&font, &source, 'H')).unwrap();
        let mut less = font.letters[&'H'].clone();
        less.parts.pop();
        let part = letter_quality(&font, &less, &outline(&font, &source, 'H')).unwrap();
        assert!(part.coverage < full.coverage - 0.1, "{part:?} {full:?}");
        assert!(part.score() < full.score());
        // No outline: no measure.
        assert!(letter_quality(&font, &font.letters[&'H'], &[]).is_none());
    }
}
