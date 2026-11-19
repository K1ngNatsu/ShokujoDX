//! Strands (P18 view plan, Milestone 3, ADR 0124): the stitches of a
//! stitch plan as the surface of the view needs them. A strand is one
//! stitch with its thread (color, kind, weight) and the foam under it.
//!
//! The stitch plan does not mark which stitches are underlay. The surface
//! does not need it: the underlay sews first, so the stitches of the top
//! lie on it ([`crate::surface`]).
//!
//! [`pieces`] gives what is not thread (ADR 0128): appliqué fabric,
//! rhinestones, the padding of trapunto fills and the holes of the cut
//! lines of cutting needles. The cut lines are not thread ([`Marks::cuts`]).

use std::ops::Range;

use emb_geom::{Path, Point};
use emb_model::{Design, ObjectId, ObjectKind, Rgb, StitchDesign, StitchKind, ThreadKind};

use crate::FALLBACK_COLOR;
use crate::surface::{Piece, PieceKind};

/// The height of puff foam under its columns, mm (3 mm foam, pressed by
/// the stitches).
pub const FOAM_MM: f64 = 2.0;
/// The height of the padding under a trapunto fill, mm.
pub const PAD_MM: f64 = 1.5;
/// The width of the hole that a cut line that is not closed makes, mm.
pub const SLIT_MM: f64 = 0.25;

/// What the objects of a design do to their stitches in the view.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Marks {
    /// The record ranges that sew over foam (puff columns and puff
    /// lettering), with the height of the foam.
    pub lifts: Vec<(Range<usize>, f64)>,
    /// The record ranges of cut lines (cutting needles, ADR 0051): they are
    /// not thread, they cut holes ([`pieces`]).
    pub cuts: Vec<Range<usize>>,
}

/// One stitch on the fabric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strand {
    pub a: Point,
    pub b: Point,
    pub color: Rgb,
    pub kind: ThreadKind,
    pub weight: u16,
    /// The foam under the stitch (puff), mm; 0 for none.
    pub lift_mm: f64,
    /// The record of the plan at the end of the stitch: the stitch player
    /// shows the stitch from this record on.
    pub record: u32,
}

/// The stitches of `design` in sew order (jumps and the movements after a
/// trim are not stitches). The records in a range of `marks.lifts` have the
/// foam of the range under them; the records of `marks.cuts` are not
/// thread.
pub fn strands(design: &StitchDesign, marks: &Marks) -> Vec<Strand> {
    let lifts = &marks.lifts;
    let mut out = Vec::with_capacity(design.records.len());
    let mut thread_index = 0usize;
    let mut last: Option<Point> = None;
    let mut cut = false;
    let thread = |i: usize| design.threads.get(i);
    for (i, r) in design.records.iter().enumerate() {
        match r.kind {
            StitchKind::Stitch | StitchKind::Jump => {
                if let Some(a) = last
                    && a != r.pos
                    && r.kind == StitchKind::Stitch
                    && !cut
                    && !marks.cuts.iter().any(|c| c.contains(&i))
                {
                    let t = thread(thread_index);
                    out.push(Strand {
                        a,
                        b: r.pos,
                        color: t.map_or(FALLBACK_COLOR, |t| t.color),
                        kind: t.map_or(ThreadKind::default(), |t| t.kind),
                        weight: t.map_or(emb_model::thread::DEFAULT_WEIGHT, |t| t.weight),
                        lift_mm: lifts.iter().find(|(range, _)| range.contains(&i)).map_or(0.0, |l| l.1),
                        record: i as u32,
                    });
                }
                if r.kind == StitchKind::Stitch {
                    cut = false;
                }
                last = Some(r.pos);
            }
            StitchKind::Trim => cut = true,
            StitchKind::ColorChange => {
                thread_index += 1;
                cut = true;
            }
            StitchKind::Slow | StitchKind::Fast | StitchKind::Stop | StitchKind::End => {}
        }
    }
    out
}

/// The marks of the objects of `design`: the foam under puff columns and
/// puff lettering, and the cut lines. `spans` are the objects of the stitch
/// plan and their records.
pub fn marks(design: &Design, spans: impl IntoIterator<Item = (ObjectId, Range<usize>)>) -> Marks {
    let mut m = Marks::default();
    for (id, r) in spans {
        match design.object(id).map(|o| &o.kind) {
            Some(ObjectKind::Satin { params, .. }) if params.puff => m.lifts.push((r, FOAM_MM)),
            Some(ObjectKind::Lettering(l)) if l.satin.puff => m.lifts.push((r, FOAM_MM)),
            Some(ObjectKind::Running { params, .. }) if params.cut => m.cuts.push(r),
            _ => {}
        }
    }
    m
}

/// The rings of `paths` in output coordinates (`to_out`).
fn rings_of<'a>(paths: impl IntoIterator<Item = &'a Path>, to_out: &emb_geom::Transform) -> Vec<Vec<Point>> {
    paths
        .into_iter()
        .flat_map(|p| p.flatten(0.1))
        .map(|pl| pl.points.iter().map(|q| to_out.apply(*q)).collect())
        .collect()
}

/// The lines of `parts` joined where an end of one is at an end of another
/// (the cutting needles cut a line in parts by direction), with `true` for
/// a line that closes.
fn join_lines(mut parts: Vec<Vec<Point>>) -> Vec<(Vec<Point>, bool)> {
    const NEAR: f64 = 0.05;
    let near = |a: Point, b: Point| (a.x - b.x).abs() <= NEAR && (a.y - b.y).abs() <= NEAR;
    let mut out = Vec::new();
    while let Some(mut line) = parts.pop() {
        while let (Some(&first), Some(&last)) = (line.first(), line.last()) {
            if line.len() > 2 && near(first, last) {
                line.pop();
                out.push((line, true));
                break;
            }
            let next = parts
                .iter()
                .position(|p| p.first().is_some_and(|q| near(*q, last)) || p.last().is_some_and(|q| near(*q, last)));
            let Some(k) = next else {
                out.push((line, false));
                break;
            };
            let mut p = parts.swap_remove(k);
            if !p.first().is_some_and(|q| near(*q, last)) {
                p.reverse();
            }
            line.extend(p.into_iter().skip(1));
        }
    }
    out
}

/// A thin ring round an open line (a slit `SLIT_MM` wide).
fn slit(points: &[Point]) -> Vec<Point> {
    let half = SLIT_MM / 2.0;
    let side = |k: usize| {
        let (a, b) = (points[k.saturating_sub(1)], points[(k + 1).min(points.len() - 1)]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        (-dy / len * half, dx / len * half)
    };
    let left = (0..points.len()).map(|k| {
        let (nx, ny) = side(k);
        Point::new(points[k].x + nx, points[k].y + ny)
    });
    let right: Vec<Point> = (0..points.len())
        .rev()
        .map(|k| {
            let (nx, ny) = side(k);
            Point::new(points[k].x - nx, points[k].y - ny)
        })
        .collect();
    left.chain(right).collect()
}

/// What is not thread in `design` (ADR 0128), in output coordinates (as the
/// stitch plan), in the order of their height: the padding under trapunto
/// fills, the holes of the cut lines, the appliqué fabric (the pieces of
/// each object by layer, in their colors and the weave of the object) and
/// the rhinestones.
pub fn pieces(design: &Design) -> Vec<Piece> {
    let to_out = design.output_transform();
    let mut out = Vec::new();
    for o in design.objects() {
        if let ObjectKind::Fill { outer, holes, params } = &o.kind
            && params.trapunto
        {
            out.push(Piece {
                rings: rings_of(std::iter::once(outer).chain(holes), &to_out),
                color: Rgb(255, 255, 255),
                kind: PieceKind::Pad { height: PAD_MM },
            });
        }
    }
    // The cut lines of all cutting needles, joined again into whole lines.
    let mut cuts: Vec<Vec<Point>> = Vec::new();
    for o in design.objects() {
        if let ObjectKind::Running { path, params } = &o.kind
            && params.cut
        {
            for pl in path.flatten(0.1) {
                let mut pts: Vec<Point> = pl.points.iter().map(|q| to_out.apply(*q)).collect();
                if pl.closed
                    && let Some(&first) = pts.first()
                {
                    pts.push(first);
                }
                if pts.len() >= 2 {
                    cuts.push(pts);
                }
            }
        }
    }
    for (line, closed) in join_lines(cuts) {
        out.push(Piece {
            rings: vec![if closed { line } else { slit(&line) }],
            color: crate::sheen::BEHIND,
            kind: PieceKind::Hole,
        });
    }
    for o in design.objects() {
        if let ObjectKind::Applique { pieces, params } = &o.kind {
            let mut pieces: Vec<&emb_model::AppliquePiece> = pieces.iter().collect();
            pieces.sort_by_key(|pc| pc.layer);
            for pc in pieces {
                out.push(Piece::fabric(
                    rings_of(std::iter::once(&pc.outer).chain(pc.holes.iter()), &to_out),
                    pc.fabric_color(params),
                    params.fabric_weave(),
                ));
            }
        }
    }
    for (c, r, color) in crate::sheet::stones_of(design).0 {
        let disk = (0..32)
            .map(|i| {
                let a = emb_geom::math::to_radians(11.25 * f64::from(i));
                Point::new(c.x + r * emb_geom::math::cos(a), c.y + r * emb_geom::math::sin(a))
            })
            .collect();
        out.push(Piece {
            rings: vec![disk],
            color,
            kind: PieceKind::Stone { center: c, radius: r },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use emb_model::{StitchRecord, Thread, ThreadId};

    use super::*;

    #[test]
    fn stitches_become_strands_with_their_thread() {
        let rec = |x: f64, k| StitchRecord::new(Point::new(x, 0.0), k);
        let design = StitchDesign {
            threads: vec![
                Thread {
                    id: ThreadId(0),
                    color: Rgb(1, 2, 3),
                    kind: ThreadKind::Rayon,
                    weight: 60,
                    ..Default::default()
                },
                Thread {
                    id: ThreadId(1),
                    color: Rgb(9, 9, 9),
                    ..Default::default()
                },
            ],
            records: vec![
                rec(0.0, StitchKind::Jump),
                rec(1.0, StitchKind::Stitch),
                rec(2.0, StitchKind::Stitch),
                rec(2.0, StitchKind::Trim),
                rec(9.0, StitchKind::Jump),
                rec(9.0, StitchKind::ColorChange),
                rec(10.0, StitchKind::Stitch),
                rec(11.0, StitchKind::Stitch),
                rec(11.0, StitchKind::End),
            ],
        };
        let marks = Marks {
            lifts: vec![(6..8, 2.0)],
            cuts: Vec::new(),
        };
        let s = strands(&design, &marks);
        // The jump after the trim and the movement after the color change
        // are not stitches (as in `crate::segments`).
        let xs: Vec<(f64, f64)> = s.iter().map(|s| (s.a.x, s.b.x)).collect();
        assert_eq!(xs, vec![(0.0, 1.0), (1.0, 2.0), (10.0, 11.0)]);
        assert_eq!(
            (s[0].kind, s[0].weight, s[0].color),
            (ThreadKind::Rayon, 60, Rgb(1, 2, 3))
        );
        assert_eq!((s[2].color, s[2].lift_mm), (Rgb(9, 9, 9), 2.0));
        assert_eq!(s[0].lift_mm, 0.0);
        assert_eq!((s[0].record, s[1].record, s[2].record), (1, 2, 7));
    }
}
