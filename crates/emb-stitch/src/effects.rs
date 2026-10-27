//! Stitch effects (P5 Milestone 6) that are not a stitch type of their
//! own: random amounts for the hand-stitch effect and jagged edges, the
//! spacing of satin pairs (fractional spacing, auto spacing, accordion
//! spacing), lapped corners and cording.
//!
//! Not a port. Each effect follows what public embroidery documentation
//! says it does, and common digitizing practice:
//!
//! - Random amounts come from a hash of the point index and a seed, so the
//!   same object always sews the same way (ADR 0003).
//! - Hand-stitch effect: each needle point but the first and the last moves
//!   in a random direction by a random part of the amount.
//! - Satin spacing: the column is cut into fine pairs; a pair is used when
//!   the length since the last used pair, measured at the spacing fraction
//!   across the column, reaches the spacing times the accordion factor
//!   (at that fraction of the column length) and the auto-spacing factor
//!   (more open where the column is narrow).
//! - Lap corners: the line is cut at its sharp corners; each part is made
//!   half the column width longer past the corner, so the columns overlap.
//! - Cording: a run along the line, a stop to lay the cord on it, and an
//!   open zigzag that holds the cord down.

use emb_geom::math::{cos, sin};
use emb_geom::{Point, Polyline};
use emb_model::SatinParams;

use crate::satin::Column;

/// A number from 0 to 1 for index `i` and `seed` (splitmix64).
pub fn hash01(seed: u64, i: u64) -> f64 {
    let mut z = seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(i.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Hand-stitch effect: moves each needle point but the first and the last
/// by up to `amount` mm in a random direction.
pub fn hand(points: &mut [Point], amount: f64, seed: u64) {
    if amount <= 0.0 || points.len() < 3 {
        return;
    }
    let n = points.len();
    for (i, p) in points.iter_mut().enumerate().take(n - 1).skip(1) {
        let a = std::f64::consts::TAU * hash01(seed, 2 * i as u64);
        let r = amount * hash01(seed, 2 * i as u64 + 1);
        *p += Point::new(cos(a), sin(a)) * r;
    }
}

/// Fine pairs for the spacing effects, in mm along the middle.
const FINE_MM: f64 = 0.05;

/// The auto-spacing factor for a column `w` mm wide: more open where the
/// column is narrow, denser where it is wide.
pub fn auto_spacing_factor(w: f64) -> f64 {
    (1.0 + 0.3 * (3.0 - w) / 3.0).clamp(0.8, 1.3)
}

/// Point pairs across the column about `step` apart, with the spacing
/// effects of `p`. With no effects, the pairs of [`Column::pairs`].
pub fn spaced_pairs(col: &Column, p: &SatinParams, step: f64) -> Vec<(Point, Point)> {
    if p.spacing_at == 0.5 && !p.auto_spacing && p.accordion.is_none() {
        return col.pairs(step);
    }
    let fine = col.pairs(FINE_MM);
    let f = p.spacing_at.clamp(0.0, 1.0);
    let at = |pr: &(Point, Point)| pr.0.lerp(pr.1, f);
    let mut lens = vec![0.0];
    for w in fine.windows(2) {
        lens.push(lens[lens.len() - 1] + at(&w[0]).distance(at(&w[1])));
    }
    let total = lens[lens.len() - 1].max(1e-9);
    let mut out = vec![fine[0]];
    let mut last = 0.0;
    for (i, pr) in fine.iter().enumerate().skip(1) {
        let t = lens[i] / total;
        let mut want = step;
        if let Some(a) = &p.accordion {
            want *= a.factor(t);
        }
        if p.auto_spacing {
            want *= auto_spacing_factor(pr.0.distance(pr.1));
        }
        if lens[i] - last >= want - 1e-9 {
            out.push(*pr);
            last = lens[i];
        }
    }
    if out.last() != fine.last() {
        out.push(fine[fine.len() - 1]);
    }
    out
}

/// A turn sharper than this (the cosine between the two directions) is a
/// corner for cap and lap corners.
pub const CORNER_COS: f64 = 0.707;

/// The parts of a line between its sharp corners, each `extend` mm longer
/// past each corner (not past the ends of the line).
pub fn lap_parts(line: &Polyline, extend: f64) -> Vec<Vec<Point>> {
    let mut pts = line.points.clone();
    pts.dedup();
    if line.closed && pts.len() > 2 {
        pts.push(pts[0]);
    }
    if pts.len() < 3 {
        return vec![pts];
    }
    let mut parts: Vec<Vec<Point>> = vec![vec![pts[0]]];
    for i in 1..pts.len() - 1 {
        parts.last_mut().expect("not empty").push(pts[i]);
        let (d0, d1) = ((pts[i] - pts[i - 1]).normalized(), (pts[i + 1] - pts[i]).normalized());
        if let (Some(d0), Some(d1)) = (d0, d1)
            && d0.dot(d1) < CORNER_COS
        {
            parts.push(vec![pts[i]]);
        }
    }
    parts.last_mut().expect("not empty").push(pts[pts.len() - 1]);
    let n = parts.len();
    for (k, part) in parts.iter_mut().enumerate() {
        if part.len() < 2 {
            continue;
        }
        if k > 0
            && let Some(d) = (part[0] - part[1]).normalized()
        {
            part[0] += d * extend;
        }
        if k + 1 < n {
            let m = part.len();
            if let Some(d) = (part[m - 1] - part[m - 2]).normalized() {
                part[m - 1] += d * extend;
            }
        }
    }
    parts
}

/// Width and spacing of the zigzag that holds a cord down, in mm.
pub const CORD_ZIGZAG_MM: f64 = 2.0;
pub const CORD_SPACING_MM: f64 = 2.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_even_and_the_same_each_run() {
        let v: Vec<f64> = (0..1000).map(|i| hash01(7, i)).collect();
        assert!(v.iter().all(|x| (0.0..1.0).contains(x)));
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        assert!((mean - 0.5).abs() < 0.05, "{mean}");
        assert_eq!(hash01(7, 3), hash01(7, 3));
        assert_ne!(hash01(7, 3), hash01(8, 3));
    }

    #[test]
    fn hand_moves_inner_points_only() {
        let mut pts: Vec<Point> = (0..10).map(|i| Point::new(f64::from(i), 0.0)).collect();
        let before = pts.clone();
        hand(&mut pts, 0.5, 1);
        assert_eq!(pts[0], before[0]);
        assert_eq!(pts[9], before[9]);
        assert!(pts.iter().zip(&before).all(|(a, b)| a.distance(*b) <= 0.5 + 1e-9));
        assert!(pts.iter().zip(&before).filter(|(a, b)| a.distance(**b) > 0.01).count() > 5);
    }

    #[test]
    fn lap_parts_overlap_at_corners() {
        let line = Polyline {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0)],
            closed: false,
        };
        let parts = lap_parts(&line, 1.5);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], vec![Point::new(0.0, 0.0), Point::new(11.5, 0.0)]);
        assert_eq!(parts[1], vec![Point::new(10.0, -1.5), Point::new(10.0, 10.0)]);
    }

    use emb_geom::Path;
    use emb_model::{
        Accordion, Corner, Design, FillParams, LineBorder, ObjectKind, RunningParams, SatinGeometry, StitchKind,
        ThreadId, Underlay,
    };

    use crate::object::stitch_object;
    use crate::plan::{PlanOptions, build_plan};
    use crate::satin::satin;

    fn column(w: f64, l: f64) -> SatinGeometry {
        SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(l, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, w)).line_to(Point::new(l, w)),
            rungs: Vec::new(),
        }
    }

    fn plain() -> SatinParams {
        SatinParams {
            pull_compensation_mm: 0.0,
            short_stitches: false,
            underlay: Underlay::None,
            ..SatinParams::default()
        }
    }

    fn sew(g: &SatinGeometry, p: &SatinParams) -> Vec<Point> {
        satin(g, p, Point::new(-5.0, 0.0), None).unwrap()
    }

    #[test]
    fn pull_compensation_for_each_side_and_as_a_percent() {
        let p = SatinParams {
            pull_compensation_mm: 0.1,
            pull_b_mm: Some(0.4),
            pull_percent: 10.0,
            ..plain()
        };
        let pts = sew(&column(4.0, 10.0), &p);
        let (lo, hi) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q.y), b.max(q.y)));
        // Rail A: 0.1 + 10 % of 4 = 0.5 out; rail B: 0.4 + 0.4 = 0.8 out.
        assert!((lo + 0.5).abs() < 1e-9 && (hi - 4.8).abs() < 1e-9, "{lo} {hi}");
    }

    #[test]
    fn jagged_edges_move_in_and_hand_moves_points() {
        let p = SatinParams {
            jagged_mm: 1.0,
            ..plain()
        };
        let pts = sew(&column(4.0, 10.0), &p);
        let inner = pts.iter().filter(|q| q.y > 0.05 && q.y < 3.95).count();
        assert!(inner > pts.len() / 2, "{inner} of {}", pts.len());
        assert!(pts.iter().all(|q| q.y >= -1e-9 && q.y <= 4.0 + 1e-9));
        // The hand effect on a running line.
        let mut d = Design::new("t");
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::new().move_to(Point::ZERO).line_to(Point::new(20.0, 0.0)),
                params: RunningParams {
                    hand_mm: 0.4,
                    ..RunningParams::default()
                },
            },
        );
        let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
        let off = s.runs[0].points.iter().filter(|q| q.y.abs() > 0.01).count();
        assert!(off > 3, "{off}");
        assert!(s.runs[0].points.iter().all(|q| q.y.abs() <= 0.4 + 1e-9));
    }

    #[test]
    fn auto_jump_keeps_long_satin_stitches_as_jumps() {
        let mut d = Design::new("t");
        d.add_object(
            ThreadId(0),
            ObjectKind::Satin {
                geometry: column(14.0, 5.0),
                params: SatinParams {
                    auto_jump: true,
                    ..plain()
                },
            },
        );
        let opts = PlanOptions {
            max_stitch_mm: 10.0,
            ..PlanOptions::default()
        };
        let plan = build_plan(&d, &opts, &mut crate::StitchCache::default());
        let recs = &plan.stitches.records;
        // Each 14 mm stitch is a jump to its middle, then the stitch: no
        // needle point inside the column.
        let inside = recs
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch && r.pos.y > 0.5 && r.pos.y < 13.5)
            .count();
        let jumps = recs.iter().filter(|r| r.kind == StitchKind::Jump).count();
        assert!(inside <= 4, "{inside}");
        assert!(jumps > 10, "{jumps}");
    }

    #[test]
    fn spacing_effects() {
        // A half ring: inner rail radius 4, outer rail radius 10.
        let arc = |r: f64| {
            let pts: Vec<(Point, bool)> = (0..=36)
                .map(|i| {
                    let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 5.0);
                    (Point::new(r * c, r * s), true)
                })
                .collect();
            Path::through_points(&pts, false)
        };
        let ring = SatinGeometry {
            rail_a: arc(4.0),
            rail_b: arc(10.0),
            rungs: Vec::new(),
        };
        let n = |at: f64| {
            sew(
                &ring,
                &SatinParams {
                    spacing_at: at,
                    ..plain()
                },
            )
            .len()
        };
        // Spacing measured on the long outer rail gives more stitches.
        assert!(n(1.0) > n(0.5) && n(0.5) > n(0.0), "{} {} {}", n(0.0), n(0.5), n(1.0));
        // Accordion: the open end has fewer stitches.
        let (_, fade) = Accordion::presets().remove(0);
        let pts = sew(
            &column(3.0, 30.0),
            &SatinParams {
                accordion: Some(fade),
                ..plain()
            },
        );
        let (first, last) = (
            pts.iter().filter(|q| q.x < 10.0).count(),
            pts.iter().filter(|q| q.x > 20.0).count(),
        );
        assert!(first > 2 * last, "{first} {last}");
        // Auto spacing: a narrow column is more open than the set spacing.
        let open = sew(
            &column(1.0, 20.0),
            &SatinParams {
                auto_spacing: true,
                ..plain()
            },
        );
        assert!(open.len() < sew(&column(1.0, 20.0), &plain()).len());
    }

    #[test]
    fn cap_and_lap_corners() {
        let line = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(20.0, 0.0))
            .line_to(Point::new(20.0, 20.0));
        let mitre = SatinGeometry::from_centerline(&line, 4.0);
        let cap = SatinGeometry::from_centerline_capped(&line, 4.0);
        // The outer rail of the cap has one point more (the cut), and goes
        // less far into the corner.
        let far = |g: &SatinGeometry| g.rail_a.points().map(|p| p.x - p.y).fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(cap.rail_a.points().count(), mitre.rail_a.points().count() + 1);
        assert!(far(&cap) < far(&mitre));
        // Lap: two columns that overlap at the corner.
        let mut d = Design::new("t");
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: line,
                params: RunningParams {
                    border: Some(LineBorder {
                        corner: Corner::Lap,
                        ..LineBorder::new(4.0, emb_model::ColumnPattern::Satin)
                    }),
                    ..RunningParams::default()
                },
            },
        );
        let o = d.object(id).unwrap();
        let s = stitch_object(o, crate::object::entry_for(o, Point::ZERO).unwrap()).unwrap();
        assert_eq!(s.runs.len(), 2);
        assert!(s.runs[0].points.iter().any(|p| p.x > 21.5));
    }

    #[test]
    fn cording_sews_a_line_then_stops_then_a_zigzag() {
        let mut d = Design::new("t");
        let id = d.add_object(
            ThreadId(0),
            ObjectKind::Running {
                path: Path::new().move_to(Point::ZERO).line_to(Point::new(30.0, 0.0)),
                params: RunningParams {
                    cording: true,
                    ..RunningParams::default()
                },
            },
        );
        let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
        assert_eq!(s.runs.len(), 2);
        assert!(s.runs[0].points.iter().all(|p| p.y == 0.0));
        assert_eq!(s.runs[1].stop, Some(crate::StopReason::PlaceCord));
        assert!(s.runs[1].points.iter().any(|p| p.y.abs() > 0.9));
    }

    #[test]
    fn fill_effects() {
        let square = |params: FillParams| {
            let mut d = Design::new("t");
            let id = d.add_object(
                ThreadId(0),
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(20.0, 20.0))),
                    holes: Vec::new(),
                    params: FillParams {
                        angle_deg: 0.0,
                        ..params
                    },
                },
            );
            let s = stitch_object(d.object(id).unwrap(), Point::ZERO).unwrap();
            s.runs.into_iter().flat_map(|r| r.points).collect::<Vec<Point>>()
        };
        let rows = |pts: &[Point]| {
            let mut ys: Vec<i64> = pts.iter().map(|p| (p.y * 100.0).round() as i64).collect();
            ys.sort_unstable();
            ys.dedup();
            ys.len()
        };
        let none = || FillParams {
            underlay: Underlay::None,
            ..FillParams::default()
        };
        let plain = square(none());
        let (_, fade) = Accordion::presets().remove(0);
        let open = square(FillParams {
            accordion: Some(fade),
            ..none()
        });
        assert!(rows(&open) * 2 < rows(&plain), "{} {}", rows(&open), rows(&plain));
        let jagged = square(FillParams {
            jagged_mm: 1.5,
            ..none()
        });
        let short_ends = jagged.iter().filter(|p| p.x > 0.05 && p.x < 1.5).count();
        assert!(short_ends > 10, "{short_ends}");
        // Double underlay: two tatami layers that cross.
        let double = square(FillParams {
            underlay: Underlay::Double,
            ..FillParams::default()
        });
        assert!(double.len() > plain.len());
    }
}
