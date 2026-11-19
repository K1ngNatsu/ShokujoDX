//! Strands (P18 view plan, Milestone 3, ADR 0124): the stitches of a
//! stitch plan as the surface of the view needs them. A strand is one
//! stitch with its thread (color, kind, weight) and the foam under it.
//!
//! The stitch plan does not mark which stitches are underlay. The surface
//! does not need it: the underlay sews first, so the stitches of the top
//! lie on it ([`crate::surface`]).

use std::ops::Range;

use emb_geom::Point;
use emb_model::{Design, ObjectId, ObjectKind, Rgb, StitchDesign, StitchKind, ThreadKind};

use crate::FALLBACK_COLOR;

/// The height of puff foam under its columns, mm (3 mm foam, pressed by
/// the stitches).
pub const FOAM_MM: f64 = 2.0;

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
}

/// The stitches of `design` in sew order (jumps and the movements after a
/// trim are not stitches). The records in a range of `lifts` have the foam
/// of the range under them.
pub fn strands(design: &StitchDesign, lifts: &[(Range<usize>, f64)]) -> Vec<Strand> {
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
                {
                    let t = thread(thread_index);
                    out.push(Strand {
                        a,
                        b: r.pos,
                        color: t.map_or(FALLBACK_COLOR, |t| t.color),
                        kind: t.map_or(ThreadKind::default(), |t| t.kind),
                        weight: t.map_or(emb_model::thread::DEFAULT_WEIGHT, |t| t.weight),
                        lift_mm: lifts.iter().find(|(range, _)| range.contains(&i)).map_or(0.0, |l| l.1),
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

/// The record ranges of the objects of `design` that sew over foam (puff
/// columns and puff lettering), with the height of the foam. `spans` are
/// the objects of the stitch plan and their records.
pub fn lifts(design: &Design, spans: impl IntoIterator<Item = (ObjectId, Range<usize>)>) -> Vec<(Range<usize>, f64)> {
    spans
        .into_iter()
        .filter(|(id, _)| {
            design.object(*id).is_some_and(|o| match &o.kind {
                ObjectKind::Satin { params, .. } => params.puff,
                ObjectKind::Lettering(l) => l.satin.puff,
                _ => false,
            })
        })
        .map(|(_, r)| (r, FOAM_MM))
        .collect()
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
        let s = strands(&design, &[(6..8, 2.0)]);
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
    }
}
