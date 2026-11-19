//! The shapes of one glyph, in font units: fill regions and satin strokes.

use emb_geom::region::{self, Join, Region, ring_contains};
use emb_geom::{Path, Point, Polyline};
use emb_model::{FillParams, ObjectKind, SatinParams};

use crate::stroke::{Strokes, strokes};

/// Satin is used in `Auto` only when the strokes cover this much of the glyph.
pub const AUTO_MIN_COVERAGE: f64 = 0.85;
/// Satin is used in `Auto` only for strokes up to this width, in mm.
pub const AUTO_MAX_SATIN_MM: f64 = 8.0;
/// A part of the glyph that the strokes do not cover is broad (a stroke
/// that the first pass lost) if a circle fits in it with this radius (a
/// factor of the widest stroke) or this radius (a factor of the capital
/// height), the smaller of the two. One very wide stroke (the bowl of a
/// heavy "p") does not make a lost stem a sliver.
const BROAD_RADIUS: f64 = 0.25;
const BROAD_RADIUS_CAP: f64 = 0.035;
/// A part smaller than this part of the area of the glyph gets no strokes:
/// a notch where strokes meet, not a lost stroke.
const LEFT_MIN_PART: f64 = 0.03;
/// A stroke of the second pass is kept if it covers at least this part of
/// the area of its part; smaller ones are bits at the corners of the part.
const LEFT_MIN_SHARE: f64 = 0.15;
/// A part that is not broad is a sliver along a stroke (the stroke is a
/// little narrower than the outline there) if at least this part of its
/// edge touches the strokes. Else it is a thin stroke on its own.
const SLIVER_TOUCH: f64 = 0.3;
/// The strokes of a part go this far (a factor of the widest stroke) into
/// the strokes around it, so that there is no gap at the join.
const LEFT_OVERLAP: f64 = 0.05;

#[derive(Debug, Clone, Default)]
pub struct GlyphShapes {
    /// Outer rings with their holes.
    pub fills: Vec<(Vec<Point>, Vec<Vec<Point>>)>,
    pub strokes: Strokes,
}

impl GlyphShapes {
    pub fn new(rings: &[Vec<Point>], cap_height: f64) -> GlyphShapes {
        let fills = regions(rings);
        let mut strokes = strokes(rings, cap_height);
        add_missed_strokes(&fills, &mut strokes, cap_height);
        GlyphShapes { fills, strokes }
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

/// The area of a satin: its two rails as one ring.
fn satin_region(g: &emb_model::SatinGeometry) -> Region {
    let a: Vec<Point> = g.rail_a.flatten(0.5).into_iter().flat_map(|p| p.points).collect();
    let b: Vec<Point> = g.rail_b.flatten(0.5).into_iter().flat_map(|p| p.points).collect();
    Region::new(a.into_iter().chain(b.into_iter().rev()).collect(), Vec::new())
}

/// The edges of a region as closed lines.
fn edges(r: &Region) -> Vec<Polyline> {
    std::iter::once(&r.outer)
        .chain(r.holes.iter())
        .map(|ring| Polyline {
            points: ring.clone(),
            closed: true,
        })
        .collect()
}

/// The area of a region given as rings (the first is the outer ring).
fn g_area(rings: &[Vec<Point>]) -> f64 {
    match rings.split_first() {
        Some((outer, holes)) => Region::new(outer.clone(), holes.to_vec()).area(),
        None => 0.0,
    }
}

fn length(lines: &[Polyline]) -> f64 {
    lines
        .iter()
        .map(|l| {
            let mut n: f64 = l.points.windows(2).map(|w| w[0].distance(w[1])).sum();
            if l.closed
                && let (Some(a), Some(b)) = (l.points.first(), l.points.last())
            {
                n += a.distance(*b);
            }
            n
        })
        .sum()
}

/// A second pass of the stroke finder, for a letter that the first pass
/// does not cover well enough for satin. The first pass can lose a stroke
/// where strokes meet (the stem of a "p", a leg of an "x"); Auto then made
/// the whole letter a fill, so a satin font got fill letters. Each part of
/// the glyph that the strokes do not cover is now:
///
/// - broad: a lost stroke; it gets its own strokes;
/// - a sliver along a stroke (the stroke is a little narrower than the
///   outline): no stitches, and it counts as covered;
/// - else a thin stroke on its own (a hairline): it gets its own strokes.
///
/// A very small part (a notch where strokes meet) counts as a sliver.
///
/// The coverage is then the part of the area of the glyph that the strokes
/// cover, with the slivers.
fn add_missed_strokes(fills: &[(Vec<Point>, Vec<Vec<Point>>)], s: &mut Strokes, cap_height: f64) {
    // A letter that is satin already stays as it is.
    if s.satins.is_empty() || s.coverage >= AUTO_MIN_COVERAGE || s.max_width <= 0.0 {
        return;
    }
    let glyph: Vec<Region> = fills.iter().map(|(o, h)| Region::new(o.clone(), h.clone())).collect();
    let glyph_area: f64 = glyph.iter().map(Region::area).sum();
    if glyph_area <= 0.0 {
        return;
    }
    let w = s.max_width;
    let covered = region::union(&s.satins.iter().map(satin_region).collect::<Vec<_>>(), &[]);
    // The edges of the satins, a little wide, to find the edges of a part
    // that touch them.
    let near = region::offset(&covered, 0.01 * w, Join::Miter);
    let mut slivers = 0.0;
    let mut added = Vec::new();
    for part in region::difference(&glyph, &covered) {
        if part.area() < LEFT_MIN_PART * glyph_area {
            slivers += part.area();
            continue;
        }
        let radius = (BROAD_RADIUS * w).min(BROAD_RADIUS_CAP * cap_height);
        let broad = !region::offset(std::slice::from_ref(&part), -radius, Join::Miter).is_empty();
        if !broad {
            let edge = edges(&part);
            let touch = length(&region::clip_lines(&near, &edge)) / length(&edge).max(1e-9);
            if touch >= SLIVER_TOUCH {
                slivers += part.area();
                continue;
            }
        }
        let grown = region::intersection(
            &region::offset(std::slice::from_ref(&part), LEFT_OVERLAP * w, Join::Miter),
            &glyph,
        );
        for g in grown {
            let rings: Vec<Vec<Point>> = std::iter::once(g.outer).chain(g.holes).collect();
            let more = strokes(&rings, cap_height);
            if more.coverage >= 0.5 {
                let min_area = LEFT_MIN_SHARE * g_area(&rings);
                for satin in more.satins {
                    if satin_region(&satin).area() >= min_area {
                        s.max_width = s.max_width.max(more.max_width);
                        added.push(satin);
                    }
                }
            }
        }
    }
    s.satins.extend(added);
    let areas: Vec<Region> = s.satins.iter().map(satin_region).collect();
    let on_glyph: f64 = region::intersection(&glyph, &region::union(&areas, &[]))
        .iter()
        .map(Region::area)
        .sum();
    s.coverage = ((on_glyph + slivers) / glyph_area).clamp(0.0, 1.0);
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

    /// The first pass of the stroke finder and the shapes with the second.
    fn passes(family: &str, c: char) -> (Strokes, GlyphShapes) {
        let f = crate::bundled::font(family, "Regular").expect("bundled font");
        let g = f.glyph(c).expect("glyph");
        let rings = f.contours(g);
        (strokes(&rings, f.cap_height), GlyphShapes::new(&rings, f.cap_height))
    }

    #[test]
    fn a_lost_stroke_gets_a_satin_and_the_letter_stays_satin() {
        // The first pass loses a stroke of these characters (the tail of
        // the comma, a stem of the pilcrow); Auto made them fill letters in
        // satin fonts. It finds the stems of "p", "q" and "u" now (the
        // stroke finder goes on along the pen stroke at a junction).
        for (family, c) in [("Lato", ','), ("Lato", '\u{b6}'), ("Pacifico", '\u{b6}')] {
            let (first, shapes) = passes(family, c);
            assert!(
                first.coverage < AUTO_MIN_COVERAGE,
                "{family} {c}: the first pass covers it now"
            );
            assert!(
                shapes.strokes.satins.len() > first.satins.len(),
                "{family} {c}: no satin for the lost stroke"
            );
            assert!(
                shapes.strokes.coverage >= AUTO_MIN_COVERAGE,
                "{family} {c}: coverage {:.2}",
                shapes.strokes.coverage
            );
        }
    }

    #[test]
    fn a_stem_goes_on_through_a_junction() {
        // The first pass lost the stem of these letters where it meets the
        // bowl or the curve. It finds it now: the direction of a stroke at
        // a junction is from a longer part of the stroke, and a stroke goes
        // on into a stroke of the same width first.
        for (family, c) in [("Anton", 'p'), ("Anton", 'q'), ("Pacifico", 'p'), ("Pacifico", 'u')] {
            let (first, shapes) = passes(family, c);
            assert!(
                first.coverage >= AUTO_MIN_COVERAGE,
                "{family} {c}: coverage {:.2}",
                first.coverage
            );
            assert_eq!(shapes.strokes.satins, first.satins, "{family} {c}");
        }
    }

    #[test]
    fn a_satin_letter_does_not_change() {
        // The second pass is only for letters that would be fills.
        for (family, c) in [("Lato", 'n'), ("Lato", 'B'), ("Anton", 'A'), ("Pacifico", 'a')] {
            let (first, shapes) = passes(family, c);
            assert!(first.coverage >= AUTO_MIN_COVERAGE);
            assert_eq!(shapes.strokes.satins, first.satins, "{family} {c}");
            assert_eq!(shapes.strokes.coverage, first.coverage);
        }
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
