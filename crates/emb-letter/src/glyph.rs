//! The shapes of one glyph, in font units: fill regions and satin strokes.

use emb_geom::region::{self, Join, Region, ring_contains};
use emb_geom::{Path, Point, Polyline};
use emb_model::{ColumnPattern, FillParams, ObjectKind, RunningParams, SatinParams};

use crate::stroke::{Strokes, strokes};

/// Satin is used in `Auto` only when the strokes cover this much of the glyph.
pub const AUTO_MIN_COVERAGE: f64 = 0.85;
/// In `Auto`, the columns of a text with a typical width up to this (mm,
/// [`auto_stitches`]) are satin columns, and their stitches longer than
/// `SATIN_SPLIT_MM` (7 mm) are split. Wider: tatami columns (P20), rows
/// across the column that follow the stroke. The value is the longest
/// satin stitch ([`crate::quality::LONG_MM`]).
pub const AUTO_MAX_SATIN_MM: f64 = crate::quality::LONG_MM;
/// In satin text, a column wider than this (mm) is a tatami column.
pub const AUTO_ALONE_MM: f64 = 1.5 * AUTO_MAX_SATIN_MM;
/// In `Auto`, a column narrower than this in its middle (mm) sews as a run
/// (P20 Milestone 5): the narrowest satin of the fonts for small text.
pub const AUTO_MIN_SATIN_MM: f64 = crate::source::SMALL_TEXT_MIN_SATIN_MM;
/// The run of a narrow column at least this long (mm) is a bean stitch (a
/// stroke of its own); a shorter one is a single run.
pub const AUTO_BEAN_MM: f64 = 2.0;
/// The stitch length of the run of a narrow column, in mm.
pub const AUTO_RUN_STITCH_MM: f64 = 1.8;
/// The step along a column for its middle width, in mm (as the lettering
/// bench measures it).
const THIN_STEP_MM: f64 = 0.25;
/// [`thin_run`] measures a column at this many places at most (a column of
/// 5 m at [`THIN_STEP_MM`]; a stitch object is at most 3 m).
const THIN_MAX_PAIRS: f64 = 20_000.0;
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
/// A sliver is at most this thick on average (a factor of the widest
/// stroke).
const SLIVER_THICK: f64 = 0.16;
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
        GlyphShapes::with_split(rings, cap_height, crate::stroke::Split::Off)
    }

    /// [`GlyphShapes::new`] with the outline split into strokes first
    /// (`stroke::Split::for_char` of the character of the glyph, ADR 0152).
    ///
    /// The cuts of elbows (ADR 0159), the cuts from notches that go along a
    /// stroke (ADR 0160), and then the bridges to curves (the brackets of
    /// serifs, the shoulder of an "n") and the cuts of balls, are kept only
    /// if the columns have no more faults than the split without them
    /// ([`split_faults`]): in some shapes (the bowls of a condensed font, a
    /// short hook at the end of a stroke, the stem of a heavy "n" beside its
    /// narrow counter) the pieces sew worse.
    pub fn with_split(rings: &[Vec<Point>], cap_height: f64, split: crate::stroke::Split) -> GlyphShapes {
        use crate::stroke::{Optional, Split};
        let fills = regions(rings);
        // A glyph much larger than its capital height (a damaged font) is
        // not a letter, and it is not split: each piece gets a grid of the
        // largest size (`grid::MAX_SIDE`), and the measure of the faults
        // sews the columns at 25 mm for each capital height, so such a
        // glyph took ten times as long as with no split (the audit of P21).
        let side =
            emb_geom::BBox::from_points(rings.iter().flatten().copied()).map_or(0.0, |b| b.width().max(b.height()));
        let split = if side > SPLIT_MAX_CAPS * cap_height {
            Split::Off
        } else {
            split
        };
        let make = |with: Optional| {
            let (mut strokes, used) = crate::stroke::strokes_split_soft(rings, cap_height, split, with);
            add_missed_strokes(&fills, &mut strokes, cap_height);
            tune_ends(&fills, &mut strokes, cap_height);
            overlap_joints(&fills, &mut strokes);
            untangle(&mut strokes, cap_height);
            (strokes, used)
        };
        let mut with = Optional::ALL;
        let (mut strokes, used) = make(with);
        let faults = |s: &Strokes| split_faults(s, &fills, cap_height);
        if used.elbows {
            let (without, _) = make(Optional { elbows: false, ..with });
            if faults(&without) < faults(&strokes) {
                strokes = without;
                with.elbows = false;
            }
        }
        if used.along {
            let (without, _) = make(Optional { along: false, ..with });
            if faults(&without) < faults(&strokes) {
                strokes = without;
                with.along = false;
            }
        }
        if used.soft {
            let (without, _) = make(Optional { soft: false, ..with });
            if faults(&without) < faults(&strokes) {
                strokes = without;
            }
        }
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

    /// The parts of the glyph in `Auto`, in font units: the satin columns
    /// if they cover the glyph, else fills. The layout chooses the stitch
    /// of the columns for the whole text ([`auto_stitches`]).
    pub fn auto_parts(&self) -> Vec<ObjectKind> {
        if self.columns_cover() {
            self.satin_parts()
        } else {
            self.fill_parts()
        }
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

/// The stitch of the columns of a text in `Auto` (P20 Milestones 2 and 3),
/// for the parts in mm of all the letters of the text with the stitch type
/// Auto. One stitch for all, so that the letters and their strokes look
/// the same: the typical width of the satin columns of the standard type
/// (the width that half of their length is narrower than,
/// [`column_width`]) decides. Up to [`AUTO_MAX_SATIN_MM`] they stay satin
/// (their long stitches are split), and only a column wider than
/// [`AUTO_ALONE_MM`] is a tatami column; wider, all are tatami columns.
/// With `thin`, a column narrower than [`AUTO_MIN_SATIN_MM`] in its middle
/// sews as a run first ([`thin_run`], P20 Milestone 5): for the columns that
/// the stroke finder makes from the outlines of a TrueType font (the letters
/// of an embroidery font were checked at their sizes). Other parts do not
/// change.
pub fn auto_stitches(parts: &mut [&mut ObjectKind], thin: bool) {
    // A column narrower than the narrowest satin sews as a run along its
    // middle (P20 Milestone 5).
    for p in parts.iter_mut().filter(|_| thin) {
        if let ObjectKind::Satin { geometry, params } = &**p
            && params.pattern.is_satin()
            && let Some(run) = thin_run(geometry)
        {
            **p = run;
        }
    }
    let widths: Vec<Option<(f64, f64)>> = parts
        .iter()
        .map(|p| match &**p {
            ObjectKind::Satin { geometry, params } if params.pattern.is_satin() => column_measure(geometry),
            _ => None,
        })
        .collect();
    let mut sorted: Vec<(f64, f64)> = widths.iter().flatten().copied().collect();
    if sorted.is_empty() {
        return;
    }
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let half = sorted.iter().map(|(_, l)| l).sum::<f64>() / 2.0;
    let mut sum = 0.0;
    let typical = sorted
        .iter()
        .find(|(_, l)| {
            sum += l;
            sum >= half
        })
        .map_or(sorted[0].0, |(w, _)| *w);
    let all = typical > AUTO_MAX_SATIN_MM;
    for (p, w) in parts.iter_mut().zip(widths) {
        let Some((w, _)) = w else { continue };
        if (all || w > AUTO_ALONE_MM)
            && let ObjectKind::Satin { params, .. } = &mut **p
        {
            *params = params.with_type(ColumnPattern::Tatami {
                stitch_mm: AUTO_TATAMI_STITCH_MM,
            });
        }
    }
}

/// The run of a column (in mm) that is narrower than
/// [`AUTO_MIN_SATIN_MM`] in its middle (the width that half of its length
/// is narrower than): its middle line, a bean stitch if it is at least
/// [`AUTO_BEAN_MM`] long. `None` for a wider column.
fn thin_run(g: &emb_model::SatinGeometry) -> Option<ObjectKind> {
    // At most `THIN_MAX_PAIRS` pairs: the time for lettering much larger
    // than a design can be went up with its height (the audit of P21).
    let long: f64 = crate::quality::column_widths(g, f64::INFINITY)
        .iter()
        .map(|(_, l)| l)
        .sum();
    let pairs = crate::quality::column_pairs(g, THIN_STEP_MM.max(long / THIN_MAX_PAIRS));
    let mut widths: Vec<(f64, f64)> = pairs
        .iter()
        .map(|(a, b, l)| (a.distance(*b), *l))
        .filter(|(w, l)| w.is_finite() && *l > 0.0)
        .collect();
    widths.sort_by(|a, b| a.0.total_cmp(&b.0));
    let half = widths.iter().map(|(_, l)| l).sum::<f64>() / 2.0;
    let mut sum = 0.0;
    let middle = widths.iter().find(|(_, l)| {
        sum += l;
        sum >= half
    })?;
    if middle.0 >= AUTO_MIN_SATIN_MM {
        return None;
    }
    let line: Vec<Point> = pairs.iter().map(|(a, b, _)| a.lerp(*b, 0.5)).collect();
    let (first, rest) = line.split_first()?;
    let mut path = Path::new().move_to(*first);
    for p in rest {
        path = path.line_to(*p);
    }
    let long: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
    Some(ObjectKind::Running {
        path,
        params: RunningParams {
            stitch_length_mm: AUTO_RUN_STITCH_MM,
            repeats: if long >= AUTO_BEAN_MM { 3 } else { 1 },
            ..Default::default()
        },
    })
}

/// The width of a column ([`column_width`]) and its length along its
/// middle.
fn column_measure(g: &emb_model::SatinGeometry) -> Option<(f64, f64)> {
    let width = column_width(g)?;
    let length: f64 = crate::quality::column_widths(g, f64::INFINITY)
        .iter()
        .map(|(_, l)| l)
        .sum();
    Some((width, length))
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

/// A round part of the letter (a dot) gets a column to its edge
/// ([`crate::tune::round_parts`], as the converter does for the embroidery
/// fonts; P20 Milestone 3).
fn tune_ends(fills: &[(Vec<Point>, Vec<Vec<Point>>)], s: &mut Strokes, cap_height: f64) {
    if s.satins.is_empty() {
        return;
    }
    let mut parts: Vec<ObjectKind> = s
        .satins
        .iter()
        .map(|g| ObjectKind::Satin {
            geometry: g.clone(),
            params: SatinParams::default(),
        })
        .collect();
    // The rings of the outline and the cell of the stroke finder, for the
    // rails of the columns that change.
    let rings: Vec<Vec<Point>> = fills
        .iter()
        .flat_map(|(o, h)| std::iter::once(o.clone()).chain(h.iter().cloned()))
        .collect();
    let cell = cap_height / crate::stroke::GRID_CELLS;
    if crate::tune::round_parts(&mut parts, fills, cap_height) == 0 {
        // The second pass of the stroke finder can add columns.
        crate::stroke::trim_column_ends(&mut s.satins, &rings, cell);
        return;
    }
    // The parts are the columns again (a round part can have one column
    // in place of more).
    s.satins = parts
        .into_iter()
        .filter_map(|p| match p {
            ObjectKind::Satin { geometry, .. } => Some(geometry),
            _ => None,
        })
        .collect();
    // The ends that went on into other columns stop at them again.
    crate::stroke::trim_column_ends(&mut s.satins, &rings, cell);
}

/// The trial of overlapped strokes (P22 Milestone 7, ADR 0174;
/// `stroke::overlap_trial`): the end of each column at a joint (a stroke
/// that ends on a stroke that goes through) goes on into that stroke, to
/// its middle. The route then sews the column first, so the stroke that
/// goes through covers the end (`route::joints`). An end goes on only as
/// far as the outline lets it.
pub(crate) fn overlap_joints(fills: &[(Vec<Point>, Vec<Vec<Point>>)], strokes: &mut crate::stroke::Strokes) {
    use emb_geom::region::ring_contains;
    let inside = |p: Point| {
        fills
            .iter()
            .any(|(o, h)| ring_contains(o, p) && !h.iter().any(|h| ring_contains(h, p)))
    };
    let joints = std::mem::take(&mut strokes.joints);
    for j in &joints {
        let (mid, len) = (j.a.midpoint(j.b), j.a.distance(j.b));
        let target = mid + j.into * j.depth;
        for g in &mut strokes.satins {
            let mut a: Vec<Point> = g.rail_a.points().collect();
            let mut b: Vec<Point> = g.rail_b.points().collect();
            if a.len() < 2 || b.len() < 2 {
                continue;
            }
            for start in [true, false] {
                let (end, inner) = if start {
                    (a[0].midpoint(b[0]), a[1].midpoint(b[1]))
                } else {
                    let (na, nb) = (a.len(), b.len());
                    (a[na - 1].midpoint(b[nb - 1]), a[na - 2].midpoint(b[nb - 2]))
                };
                // The end is at the bridge and the column goes into it.
                let Some(out) = (end - inner).normalized() else {
                    continue;
                };
                if end.distance(mid) > 0.6 * len || out.dot(j.into) < 0.5 {
                    continue;
                }
                let mut by = (target - end).dot(j.into);
                if by <= 0.0 {
                    continue;
                }
                let (pa, pb) = if start {
                    (a[0], b[0])
                } else {
                    (a[a.len() - 1], b[b.len() - 1])
                };
                // Shorter where the outline ends first.
                for _ in 0..4 {
                    if inside(pa + j.into * by) && inside(pb + j.into * by) {
                        break;
                    }
                    by *= 0.5;
                }
                if !(inside(pa + j.into * by) && inside(pb + j.into * by)) {
                    continue;
                }
                let shift = j.into * by;
                if start {
                    a.insert(0, pa + shift);
                    b.insert(0, pb + shift);
                } else {
                    a.push(pa + shift);
                    b.push(pb + shift);
                }
                g.rail_a = polyline(&a);
                g.rail_b = polyline(&b);
                break;
            }
        }
    }
}

fn polyline(pts: &[Point]) -> emb_geom::Path {
    let mut p = emb_geom::Path::new();
    if let Some((&first, rest)) = pts.split_first() {
        p = p.move_to(first);
        for &q in rest {
            p = p.line_to(q);
        }
    }
    p
}

/// The outline of a satin column: rail A, then rail B back. It crosses
/// itself where the rails cross.
fn satin_ring(g: &emb_model::SatinGeometry) -> Vec<Point> {
    let a: Vec<Point> = g.rail_a.flatten(0.5).into_iter().flat_map(|p| p.points).collect();
    let b: Vec<Point> = g.rail_b.flatten(0.5).into_iter().flat_map(|p| p.points).collect();
    a.into_iter().chain(b.into_iter().rev()).collect()
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
    let covered = region::fill_nonzero(&s.satins.iter().map(satin_ring).collect::<Vec<_>>());
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
            // The mean thickness of the part: a sliver is a thin band
            // along the edge of a stroke, not the narrow tail of a comma
            // (P21: with the rails on the outline, the tail of the comma
            // of Lato was not broad, and it counted as covered).
            let thick = 2.0 * part.area() / length(&edge).max(1e-9);
            if touch >= SLIVER_TOUCH && thick <= SLIVER_THICK * w {
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
                    let area: f64 = region::fill_nonzero(&[satin_ring(&satin)])
                        .iter()
                        .map(Region::area)
                        .sum();
                    if area >= min_area {
                        s.max_width = s.max_width.max(more.max_width);
                        added.push(satin);
                    }
                }
            }
        }
    }
    s.satins.extend(added);
    let areas = region::fill_nonzero(&s.satins.iter().map(satin_ring).collect::<Vec<_>>());
    let on_glyph: f64 = region::intersection(&glyph, &areas).iter().map(Region::area).sum();
    s.coverage = ((on_glyph + slivers) / glyph_area).clamp(0.0, 1.0);
}

/// No pair across a column crosses the pair before it (P21 Milestone 12):
/// where it would (a fast turn, a slanted end, the joint of a crossbar),
/// the column gets rungs on which one side stays
/// (`emb_stitch::satin::Column::untangled_rungs`).
fn untangle(strokes: &mut Strokes, cap_height: f64) {
    for g in &mut strokes.satins {
        untangle_column(g, cap_height);
    }
}

fn untangle_column(g: &mut emb_model::SatinGeometry, cap_height: f64) {
    if let Some(rungs) = emb_stitch::satin::Column::new(g).and_then(|c| c.untangled_rungs(UNTANGLE_STEP * cap_height)) {
        g.rungs = rungs;
    }
}

/// [`untangle`] for the parts of a letter with the capital height
/// `cap_height`: the layout does it last, as the pivots of the rungs do not
/// stay when the points are rounded (an embroidery font file).
pub fn untangle_parts(parts: &mut [ObjectKind], cap_height: f64) {
    for p in parts {
        if let ObjectKind::Satin { geometry, .. } = p {
            untangle_column(geometry, cap_height);
        }
    }
}

/// The pairs that [`untangle`] checks are this part of the capital height
/// apart (0.2 mm at 25 mm, as the letter fault bench).
const UNTANGLE_STEP: f64 = 0.008;

/// The capital height (mm) at which [`split_faults`] measures.
const SPLIT_CAP_MM: f64 = 25.0;
/// The pairs across a column for [`split_faults`] are this far apart (mm),
/// as in the letter fault bench.
const SPLIT_STEP_MM: f64 = 0.2;
/// A pair that crosses the next is a smaller fault than a needle point out
/// of the letter: it counts as this much (with each 1 mm2 open as 1).
const SPLIT_CROSS: f64 = 0.5;
/// A glyph larger than this many capital heights is not split
/// ([`GlyphShapes::with_split`]).
const SPLIT_MAX_CAPS: f64 = 10.0;

/// The faults of the columns of `strokes` in the outline `fills` (font
/// units), for the choice of a split: the needle points out of the outline
/// (`quality::spike_points`), the pairs across a column that cross the
/// next pair ([`SPLIT_CROSS`] each), and the open area in mm2, at a
/// capital height of [`SPLIT_CAP_MM`].
fn split_faults(strokes: &Strokes, fills: &[(Vec<Point>, Vec<Vec<Point>>)], cap_height: f64) -> f64 {
    let k = SPLIT_CAP_MM / cap_height.max(1e-9);
    let t = emb_geom::Transform::scale(k, k);
    let parts: Vec<ObjectKind> = strokes
        .satins
        .iter()
        .map(|g| {
            let mut p = ObjectKind::Satin {
                geometry: g.clone(),
                params: SatinParams::default(),
            };
            p.transform(&t);
            p
        })
        .collect();
    let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|&p| p * k).collect() };
    let outline: Vec<(Vec<Point>, Vec<Vec<Point>>)> = fills
        .iter()
        .map(|(o, h)| (scale(o), h.iter().map(scale).collect()))
        .collect();
    let area: f64 = outline
        .iter()
        .map(|(o, h)| {
            region::ring_signed_area(o).abs() - h.iter().map(|r| region::ring_signed_area(r).abs()).sum::<f64>()
        })
        .sum();
    let crossing = parts
        .iter()
        .filter_map(|p| match p {
            ObjectKind::Satin { geometry, .. } => emb_stitch::satin::Column::new(geometry),
            _ => None,
        })
        .map(|c| {
            c.pairs(SPLIT_STEP_MM)
                .windows(2)
                .filter(|w| emb_stitch::satin::pairs_cross(w[0], w[1]))
                .count()
        })
        .sum::<usize>();
    crate::quality::spike_points(&parts, &outline) as f64
        + SPLIT_CROSS * crossing as f64
        + (1.0 - strokes.coverage).max(0.0) * area
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
        // Since P21 Milestone 12 the first pass covers the comma of Lato (a
        // rail follows the side of the tail that swells).
        let mut second = 0;
        for (family, c) in [("Lato", ','), ("Lato", '\u{b6}'), ("Pacifico", '\u{b6}')] {
            let (first, shapes) = passes(family, c);
            if first.coverage < AUTO_MIN_COVERAGE {
                second += 1;
                assert!(
                    shapes.strokes.satins.len() > first.satins.len(),
                    "{family} {c}: no satin for the lost stroke"
                );
            }
            assert!(
                shapes.strokes.coverage >= AUTO_MIN_COVERAGE,
                "{family} {c}: coverage {:.2}",
                shapes.strokes.coverage
            );
        }
        assert!(second > 0, "no character needs the second pass");
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
            // The same columns (the rungs can change: `untangle`).
            let rails = |s: &Strokes| {
                s.satins
                    .iter()
                    .map(|g| (g.rail_a.clone(), g.rail_b.clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(rails(&shapes.strokes), rails(&first), "{family} {c}");
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
    fn auto_chooses_one_stitch_for_the_text() {
        let part = |len: f64, w: f64, params: SatinParams| ObjectKind::Satin {
            geometry: column(len, w, None),
            params,
        };
        let patterns = |parts: &[ObjectKind]| -> Vec<ColumnPattern> {
            parts
                .iter()
                .map(|p| match p {
                    ObjectKind::Satin { params, .. } => params.pattern,
                    _ => unreachable!(),
                })
                .collect()
        };
        let run = |parts: &mut Vec<ObjectKind>| {
            let mut refs: Vec<&mut ObjectKind> = parts.iter_mut().collect();
            auto_stitches(&mut refs, true);
        };
        let tatami = ColumnPattern::Tatami {
            stitch_mm: AUTO_TATAMI_STITCH_MM,
        };
        let satin = ColumnPattern::Satin;
        // Strokes of 11 to 13 mm: the typical width is 11.5 mm, so all are
        // satin (not one stitch for each column).
        let mut parts = vec![
            part(100.0, 11.0, SatinParams::default()),
            part(100.0, 11.5, SatinParams::default()),
            part(60.0, 13.0, SatinParams::default()),
        ];
        run(&mut parts);
        assert_eq!(patterns(&parts), [satin, satin, satin]);
        // A typical width of 12.5 mm: all are tatami columns, also the
        // narrow one, with the underlay of each.
        let no_underlay = SatinParams {
            underlay: emb_model::Underlay::None,
            ..SatinParams::default()
        };
        let mut parts = vec![
            part(100.0, 12.5, no_underlay),
            part(100.0, 13.0, SatinParams::default()),
            part(60.0, 9.0, SatinParams::default()),
        ];
        run(&mut parts);
        assert_eq!(patterns(&parts), [tatami, tatami, tatami]);
        let ObjectKind::Satin { params, .. } = &parts[0] else {
            unreachable!()
        };
        assert_eq!(params.underlay, emb_model::Underlay::None);
        assert!(!params.short_stitches);
        // In satin text, only a column much wider than the limit is a
        // tatami column; a column of another type keeps it.
        let mut parts = vec![
            part(100.0, 6.0, SatinParams::default()),
            part(100.0, 6.0, SatinParams::default()),
            part(60.0, 20.0, SatinParams::default()),
            part(60.0, 20.0, SatinParams::with_pattern(ColumnPattern::Zigzag)),
        ];
        run(&mut parts);
        assert_eq!(patterns(&parts), [satin, satin, tatami, ColumnPattern::Zigzag]);
    }

    /// A glyph much larger than its capital height is not split (the audit
    /// of P21): a "T" 11 capital heights high has the columns of the stroke
    /// finder with no split.
    #[test]
    fn a_glyph_much_larger_than_its_capital_height_is_not_split() {
        let t: Vec<Point> = [
            (-10.0, 0.0),
            (10.0, 0.0),
            (10.0, 1080.0),
            (50.0, 1080.0),
            (50.0, 1100.0),
            (-50.0, 1100.0),
            (-50.0, 1080.0),
            (-10.0, 1080.0),
        ]
        .iter()
        .map(|&(x, y)| Point::new(x, y))
        .collect();
        let rings = [t];
        let split = GlyphShapes::with_split(&rings, 100.0, crate::stroke::Split::Notches);
        let plain = GlyphShapes::new(&rings, 100.0);
        assert!(!plain.strokes.satins.is_empty());
        assert_eq!(split.strokes.satins, plain.strokes.satins);
    }

    /// A column is measured at a limited number of places (the audit of
    /// P21: the time for lettering much higher than a design can be went up
    /// with its height).
    #[test]
    fn a_very_long_column_is_measured_at_few_places() {
        let rail = |y: f64| Path::new().move_to(Point::new(0.0, y)).line_to(Point::new(1e9, y));
        let g = emb_model::SatinGeometry {
            rail_a: rail(-0.25),
            rail_b: rail(0.25),
            rungs: Vec::new(),
        };
        let Some(ObjectKind::Running { path, .. }) = thin_run(&g) else {
            panic!("a column of 0.5 mm is a run");
        };
        assert!(path.points().count() <= THIN_MAX_PAIRS as usize + 2);
    }

    #[test]
    fn a_thin_column_sews_as_a_run() {
        // P20 Milestone 5: a column narrower than the narrowest satin is a
        // run along its middle; a long one is a bean stitch.
        let runs = |len: f64, w: f64| -> Option<(usize, u32)> {
            let mut parts = [ObjectKind::Satin {
                geometry: column(len, w, None),
                params: SatinParams::default(),
            }];
            let mut refs: Vec<&mut ObjectKind> = parts.iter_mut().collect();
            auto_stitches(&mut refs, true);
            match &parts[0] {
                ObjectKind::Running { path, params } => Some((path.points().count(), params.repeats)),
                _ => None,
            }
        };
        assert!(runs(10.0, 1.0).is_none(), "a satin of 1 mm stays");
        let (points, repeats) = runs(10.0, 0.5).expect("a run");
        assert!(points >= 2);
        assert_eq!(repeats, 3, "a stroke of 10 mm: a bean stitch");
        assert_eq!(runs(1.0, 0.5).map(|r| r.1), Some(1), "a short piece: one run");
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
