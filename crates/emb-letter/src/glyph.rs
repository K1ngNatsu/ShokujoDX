//! The shapes of one glyph, in font units: fill regions and satin strokes.

use emb_geom::region::{self, Join, Region, ring_contains};
use emb_geom::{Path, Point, Polyline};
use emb_model::{ColumnPattern, FillParams, ObjectKind, SatinParams};

use crate::stroke::{Strokes, strokes};

/// Satin is used in `Auto` only when the strokes cover this much of the glyph.
pub const AUTO_MIN_COVERAGE: f64 = 0.85;
/// In `Auto`, a column up to this width (mm, [`column_width`]) is a satin
/// column, and its stitches longer than `SATIN_SPLIT_MM` (7 mm) are split.
/// A wider column is a tatami column (P20 Milestone 2): rows across the
/// column that follow the stroke. The value is the longest satin stitch
/// ([`crate::quality::LONG_MM`]).
pub const AUTO_MAX_SATIN_MM: f64 = crate::quality::LONG_MM;
/// The stitch length of the rows of a tatami column of `Auto`, in mm.
pub const AUTO_TATAMI_STITCH_MM: f64 = 3.5;
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

    /// `true` if the columns of the strokes cover enough of the glyph for
    /// `Auto` ([`AUTO_MIN_COVERAGE`]); else the letter is a fill.
    pub fn columns_cover(&self) -> bool {
        !self.strokes.satins.is_empty() && self.strokes.coverage >= AUTO_MIN_COVERAGE
    }

    /// The parts of the glyph in `Auto` at `scale` (mm for each font unit),
    /// in font units: the columns, each with its stitch
    /// ([`auto_column`]), if they cover the glyph; else fills.
    pub fn auto_parts(&self, scale: f64) -> Vec<ObjectKind> {
        if !self.columns_cover() {
            return self.fill_parts();
        }
        let mut parts = self.satin_parts();
        for p in &mut parts {
            auto_column(p, scale);
        }
        parts
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

/// The stitch of a column in `Auto` (P20 Milestone 2), chosen for each
/// column and not for the whole letter: a satin column of the standard
/// type that is wider than [`AUTO_MAX_SATIN_MM`] at `scale` (mm for each
/// unit of the part) becomes a tatami column. Other parts do not change.
pub fn auto_column(part: &mut ObjectKind, scale: f64) {
    let ObjectKind::Satin { geometry, params } = part else {
        return;
    };
    if !params.pattern.is_satin() {
        return;
    }
    if column_width(geometry).is_some_and(|w| w * scale > AUTO_MAX_SATIN_MM) {
        *params = params.with_type(ColumnPattern::Tatami {
            stitch_mm: AUTO_TATAMI_STITCH_MM,
        });
    }
}

/// A column keeps its width ([`column_width`]) over at least this many
/// times the width.
pub const KEEP_LENGTH: f64 = 1.5;

/// The width of a column between its two sides (P20 Milestone 2): the
/// largest width that the column keeps over a length of [`KEEP_LENGTH`]
/// times that width. Where a column crosses another stroke or another
/// stroke meets it, its rails go into that stroke, so the column is wider
/// there over about the width of that stroke: that width does not count.
/// A column that is shorter than that (a dot) has its middle width. In the
/// units of the column; `None` if it has no width.
pub fn column_width(g: &emb_model::SatinGeometry) -> Option<f64> {
    let coarse = crate::quality::column_widths(g, f64::INFINITY);
    let total: f64 = coarse.iter().map(|(_, l)| l).sum();
    if !(total.is_finite() && total > 0.0) {
        return None;
    }
    // The width at each pair of rail points, and the length along the
    // middle of the column: where the column is wider, its rails go round
    // the wide part and are longer than the column.
    let pairs = crate::quality::column_pairs(g, total / 400.0);
    let middle: Vec<Point> = pairs.iter().map(|(a, b, _)| a.lerp(*b, 0.5)).collect();
    let mut w: Vec<(f64, f64)> = (0..pairs.len())
        .map(|i| {
            let before = middle[i.saturating_sub(1)].distance(middle[i]);
            let after = middle[i].distance(middle[(i + 1).min(middle.len() - 1)]);
            (pairs[i].0.distance(pairs[i].1), (before + after) / 2.0)
        })
        .collect();
    w.retain(|(width, len)| width.is_finite() && *width > 0.0 && *len > 0.0);
    if w.is_empty() {
        return None;
    }
    // The longest piece of the column that is at least `x` wide.
    let run = |x: f64| -> f64 {
        let (mut best, mut now) = (0.0f64, 0.0);
        for &(width, len) in &w {
            now = if width >= x { now + len } else { 0.0 };
            best = best.max(now);
        }
        best
    };
    let mut sorted: Vec<f64> = w.iter().map(|(width, _)| *width).collect();
    sorted.sort_by(f64::total_cmp);
    // The test is true for the narrowest widths and false for the widest:
    // find where it changes.
    let keeps = |x: f64| run(x) >= KEEP_LENGTH * x;
    if !keeps(sorted[0]) {
        let mut by_width = w.clone();
        by_width.sort_by(|a, b| a.0.total_cmp(&b.0));
        let half = by_width.iter().map(|(_, l)| l).sum::<f64>() / 2.0;
        let mut sum = 0.0;
        return by_width
            .iter()
            .find(|(_, l)| {
                sum += l;
                sum >= half
            })
            .map(|(width, _)| *width);
    }
    let (mut lo, mut hi): (usize, usize) = (0, sorted.len() - 1);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if keeps(sorted[mid]) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Some(sorted[lo])
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

    /// A straight column along X from 0 to `len`, `w` wide, and `wide`
    /// wide from `from` to `to` (as where it crosses another stroke).
    fn column(len: f64, w: f64, bulge: Option<(f64, f64, f64)>) -> emb_model::SatinGeometry {
        let half = |x: f64| match bulge {
            Some((from, to, wide)) if (from..=to).contains(&x) => wide / 2.0,
            _ => w / 2.0,
        };
        let xs: Vec<f64> = (0..=(len as usize)).map(|i| i as f64).collect();
        let rail = |side: f64| {
            let mut p = Path::new().move_to(Point::new(0.0, side * half(0.0)));
            for &x in &xs[1..] {
                p = p.line_to(Point::new(x, side * half(x)));
            }
            p
        };
        emb_model::SatinGeometry {
            rail_a: rail(-1.0),
            rail_b: rail(1.0),
            rungs: xs
                .iter()
                .map(|&x| (Point::new(x, -half(x)), Point::new(x, half(x))))
                .collect(),
        }
    }

    #[test]
    fn the_width_of_a_column_is_between_its_sides() {
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 0.5);
        let plain = column(100.0, 10.0, None);
        assert!(near(column_width(&plain), 10.0), "{:?}", column_width(&plain));
        // Where it crosses a stroke 10 wide, the column is 20 wide over a
        // length of 10: that is not its width.
        let crossed = column(100.0, 10.0, Some((45.0, 55.0, 20.0)));
        assert!(near(column_width(&crossed), 10.0), "{:?}", column_width(&crossed));
        // A part that is wide over a length of more than 1.5 times its
        // width is the width of the column.
        let wide = column(100.0, 10.0, Some((20.0, 60.0, 20.0)));
        assert!(near(column_width(&wide), 20.0), "{:?}", column_width(&wide));
        // A dot: shorter than its width.
        let dot = column(6.0, 10.0, None);
        assert!(near(column_width(&dot), 10.0), "{:?}", column_width(&dot));
    }

    #[test]
    fn auto_chooses_the_stitch_of_each_column() {
        let part = |g: emb_model::SatinGeometry, params: SatinParams| ObjectKind::Satin { geometry: g, params };
        let pattern = |p: &ObjectKind| match p {
            ObjectKind::Satin { params, .. } => params.pattern,
            _ => unreachable!(),
        };
        // 10 units wide, 10 mm at a scale of 1: satin (its long stitches
        // are split). The crossing does not count.
        let mut p = part(column(100.0, 10.0, Some((45.0, 55.0, 20.0))), SatinParams::default());
        auto_column(&mut p, 1.0);
        assert_eq!(pattern(&p), ColumnPattern::Satin);
        // 13 mm: a tatami column, with the underlay of the satin.
        let satin = SatinParams {
            underlay: emb_model::Underlay::None,
            ..SatinParams::default()
        };
        let mut p = part(column(100.0, 10.0, None), satin);
        auto_column(&mut p, 1.3);
        let ObjectKind::Satin { params, .. } = &p else {
            unreachable!()
        };
        assert_eq!(
            params.pattern,
            ColumnPattern::Tatami {
                stitch_mm: AUTO_TATAMI_STITCH_MM
            }
        );
        assert_eq!(params.underlay, emb_model::Underlay::None);
        assert!(!params.short_stitches);
        // A column of another type keeps it.
        let mut p = part(
            column(100.0, 10.0, None),
            SatinParams::with_pattern(ColumnPattern::Zigzag),
        );
        auto_column(&mut p, 2.0);
        assert_eq!(pattern(&p), ColumnPattern::Zigzag);
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
