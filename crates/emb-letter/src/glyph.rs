//! The shapes of one glyph, in font units: fill regions and satin strokes.

use emb_geom::region::ring_contains;
use emb_geom::{Path, Point};
use emb_model::{FillParams, ObjectKind, SatinParams};

use crate::stroke::{Strokes, strokes};

/// Satin is used in `Auto` only when the strokes cover this much of the glyph.
pub const AUTO_MIN_COVERAGE: f64 = 0.85;
/// Satin is used in `Auto` only for strokes up to this width, in mm.
pub const AUTO_MAX_SATIN_MM: f64 = 8.0;

#[derive(Debug, Clone, Default)]
pub struct GlyphShapes {
    /// Outer rings with their holes.
    pub fills: Vec<(Vec<Point>, Vec<Vec<Point>>)>,
    pub strokes: Strokes,
}

impl GlyphShapes {
    pub fn new(rings: &[Vec<Point>], cap_height: f64) -> GlyphShapes {
        GlyphShapes {
            fills: regions(rings),
            strokes: strokes(rings, cap_height),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fills.is_empty()
    }

    /// `true` if satin suits the glyph at `scale` (mm for each font unit).
    pub fn satin_fits(&self, scale: f64) -> bool {
        !self.strokes.satins.is_empty()
            && self.strokes.coverage >= AUTO_MIN_COVERAGE
            && self.strokes.max_width * scale <= AUTO_MAX_SATIN_MM
    }

    /// Satin parts in font units.
    pub fn satin_parts(&self) -> Vec<ObjectKind> {
        self.strokes
            .satins
            .iter()
            .map(|g| ObjectKind::Satin {
                geometry: g.clone(),
                params: SatinParams::default(),
            })
            .collect()
    }

    /// Fill parts in font units.
    pub fn fill_parts(&self) -> Vec<ObjectKind> {
        self.fills
            .iter()
            .map(|(outer, holes)| ObjectKind::Fill {
                outer: Path::polygon(outer),
                holes: holes.iter().map(|h| Path::polygon(h)).collect(),
                params: FillParams::default(),
            })
            .collect()
    }
}

/// Outer rings and their holes: a ring inside an even number of other
/// rings is an outer ring; a ring inside an odd number is a hole of the
/// smallest outer ring around it.
fn regions(rings: &[Vec<Point>]) -> Vec<(Vec<Point>, Vec<Vec<Point>>)> {
    let area = |r: &[Point]| emb_geom::region::ring_signed_area(r).abs();
    let inside =
        |a: usize, b: usize| a != b && ring_contains(&rings[b], rings[a][0]) && area(&rings[a]) < area(&rings[b]);
    let depth: Vec<usize> = (0..rings.len())
        .map(|a| (0..rings.len()).filter(|&b| inside(a, b)).count())
        .collect();
    let mut out: Vec<(usize, Vec<Vec<Point>>)> = (0..rings.len())
        .filter(|&a| depth[a].is_multiple_of(2))
        .map(|a| (a, Vec::new()))
        .collect();
    for h in (0..rings.len()).filter(|&a| !depth[a].is_multiple_of(2)) {
        let parent = out
            .iter_mut()
            .filter(|(o, _)| inside(h, *o) && depth[*o] + 1 == depth[h])
            .min_by(|x, y| area(&rings[x.0]).total_cmp(&area(&rings[y.0])));
        if let Some((_, holes)) = parent {
            holes.push(rings[h].clone());
        }
    }
    out.into_iter().map(|(a, holes)| (rings[a].clone(), holes)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(c: f64, r: f64) -> Vec<Point> {
        vec![
            Point::new(c - r, -c - r),
            Point::new(c + r, -c - r),
            Point::new(c + r, -c + r),
            Point::new(c - r, -c + r),
        ]
    }

    #[test]
    fn rings_nest_into_regions() {
        // An outer square, a hole in it, and an island in the hole.
        let r = regions(&[square(50.0, 50.0), square(50.0, 30.0), square(50.0, 10.0)]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].1.len(), 1, "the big square has one hole");
        assert!(r[1].1.is_empty(), "the island has none");
    }
}
