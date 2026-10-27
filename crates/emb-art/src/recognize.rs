//! Recognize outlines (P6 Milestone 8, "process stitches"; ADR 0030): the
//! area that the stitches of a stitch object cover, as outlines, so the
//! object can become a fill that the stitch engine makes again (at any size,
//! with its density). Our own method:
//!
//! 1. Each stitch is drawn as a line as wide as a thread into a grid of
//!    [`CELL_MM`] cells (jumps are not drawn).
//! 2. Gaps up to two cells between stitches close (dilate, then erode).
//! 3. The covered cells are traced as areas (`autotrace`).

use emb_geom::{BBox, Point, Transform};

use crate::autotrace::{self, Area, TraceSettings};
use crate::reduce::{NONE, Reduced};

/// The size of a grid cell, in mm.
pub const CELL_MM: f64 = 0.2;
/// The width of a thread, in mm.
const THREAD_MM: f64 = 0.4;
/// Areas smaller than this are left out, in mm².
const MIN_AREA_MM2: f64 = 1.0;

/// The areas that the stitch lines cover. Each stitch is `(from, to)`, mm.
pub fn outlines(stitches: &[(Point, Point)]) -> Vec<Area> {
    let Some(bb) = BBox::from_points(stitches.iter().flat_map(|(a, b)| [*a, *b])) else {
        return Vec::new();
    };
    let margin = 4.0 * CELL_MM;
    let origin = Point::new(bb.min.x - margin, bb.min.y - margin);
    let w = ((bb.width() + 2.0 * margin) / CELL_MM).ceil() as usize + 1;
    let h = ((bb.height() + 2.0 * margin) / CELL_MM).ceil() as usize + 1;
    let mut on = vec![false; w * h];
    let r = THREAD_MM / 2.0;
    for &(a, b) in stitches {
        let lo = |v: f64, o: f64| (((v - r - o) / CELL_MM).floor().max(0.0)) as usize;
        let hi = |v: f64, o: f64, n: usize| ((((v + r - o) / CELL_MM).ceil()) as usize).min(n);
        let (x0, x1) = (lo(a.x.min(b.x), origin.x), hi(a.x.max(b.x), origin.x, w));
        let (y0, y1) = (lo(a.y.min(b.y), origin.y), hi(a.y.max(b.y), origin.y, h));
        for y in y0..y1 {
            for x in x0..x1 {
                let c = origin + Point::new((x as f64 + 0.5) * CELL_MM, (y as f64 + 0.5) * CELL_MM);
                if c.distance_to_segment(a, b) <= r {
                    on[y * w + x] = true;
                }
            }
        }
    }
    // Close gaps of up to two cells: grow by one cell, then shrink by one.
    let grow = |m: &[bool], value: bool| -> Vec<bool> {
        let mut out = m.to_vec();
        for y in 0..h {
            for x in 0..w {
                if m[y * w + x] == value {
                    continue;
                }
                let near = [
                    (-1i64, 0i64),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, 1),
                    (-1, 1),
                    (1, -1),
                ]
                .iter()
                .any(|(dx, dy)| {
                    let (qx, qy) = (x as i64 + dx, y as i64 + dy);
                    qx >= 0
                        && qy >= 0
                        && (qx as usize) < w
                        && (qy as usize) < h
                        && m[qy as usize * w + qx as usize] == value
                });
                if near {
                    out[y * w + x] = value;
                }
            }
        }
        out
    };
    let closed = grow(&grow(&on, true), false);
    let r = Reduced {
        width: w as u32,
        height: h as u32,
        palette: vec![emb_model::Rgb(0, 0, 0)],
        labels: closed.iter().map(|&c| if c { 0 } else { NONE }).collect(),
    };
    let s = TraceSettings {
        min_area_px: (MIN_AREA_MM2 / (CELL_MM * CELL_MM)) as usize,
        ..TraceSettings::default()
    };
    let to_mm = Transform::scale(CELL_MM, CELL_MM).then(Transform::translate(origin.x, origin.y));
    autotrace::trace(&r, &s).iter().map(|a| a.transformed(&to_mm)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_of_stitches_become_their_area() {
        // A 20 x 10 mm block of rows 0.4 mm apart, and a second block 5 mm away.
        let mut st = Vec::new();
        for (x0, x1) in [(0.0, 20.0), (25.0, 35.0)] {
            for k in 0..=25 {
                let y = f64::from(k) * 0.4;
                st.push((Point::new(x0, y), Point::new(x1, y)));
            }
        }
        let areas = outlines(&st);
        assert_eq!(areas.len(), 2);
        let bb = |a: &Area| BBox::from_points(a.outer.flatten(0.01).into_iter().flat_map(|p| p.points)).unwrap();
        let b = bb(&areas[0]);
        assert!(
            (b.width() - 20.4).abs() < 0.5 && (b.height() - 10.4).abs() < 0.5,
            "{b:?}"
        );
        assert!(areas.iter().all(|a| a.holes.is_empty()));
        assert!(outlines(&[]).is_empty());
    }
}
