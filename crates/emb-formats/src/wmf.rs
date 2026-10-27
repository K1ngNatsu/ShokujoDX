//! WMF (Windows metafile) artwork import (P6 Milestone 5; ADR 0027), from
//! the public [MS-WMF] specification.
//!
//! The reader runs the drawing records: polygons, polylines, PolyPolygon,
//! rectangles, round rectangles (as rectangles), ellipses, MoveTo and
//! LineTo, pens and brushes in the object table, SaveDC and RestoreDC,
//! and the window origin and extent. Text and bitmaps are not imported
//! (warnings).
//!
//! A placeable WMF (key 0x9AC6CDD7) gives its size in units per inch, and
//! its frame; the window maps to the frame. A WMF with no placeable header
//! is read at 96 units per inch.

use emb_geom::Point;

use crate::meta::{Bytes, MetaObject, Painter, colorref};
use crate::svg::{SvgError, SvgImport};

const PLACEABLE_KEY: u32 = 0x9AC6_CDD7;

#[derive(Debug, Clone, Copy)]
struct Dc {
    window_org: (f64, f64),
    window_ext: Option<(f64, f64)>,
    brush: Option<emb_model::Rgb>,
    pen: Option<(emb_model::Rgb, f64)>,
}

struct Wmf {
    dc: Dc,
    saved: Vec<Dc>,
    objects: Vec<Option<MetaObject>>,
    /// The frame of a placeable file in its units: left, top, width, height.
    frame: Option<(f64, f64, f64, f64)>,
    /// mm per unit.
    mm: f64,
}

impl Wmf {
    fn to_mm(&self, x: f64, y: f64) -> Point {
        let (wx, wy) = (x - self.dc.window_org.0, y - self.dc.window_org.1);
        let (ux, uy) = match (self.frame, self.dc.window_ext) {
            // The window fills the frame.
            (Some((_, _, fw, fh)), Some((ex, ey))) if ex != 0.0 && ey != 0.0 => (wx * fw / ex, wy * fh / ey),
            _ => (wx, wy),
        };
        Point::new(ux * self.mm, uy * self.mm)
    }

    fn len_mm(&self, l: f64) -> f64 {
        self.to_mm(0.0, 0.0).distance(self.to_mm(l, 0.0))
    }

    /// Points as (x, y) pairs of 16-bit numbers.
    fn points(&self, b: &mut Bytes, n: usize) -> Vec<Point> {
        (0..n)
            .map_while(|_| Some(self.to_mm(f64::from(b.i16()?), f64::from(b.i16()?))))
            .collect()
    }

    /// A new object goes in the first free place of the object table.
    fn add(&mut self, o: MetaObject) {
        match self.objects.iter_mut().find(|s| s.is_none()) {
            Some(s) => *s = Some(o),
            None => self.objects.push(Some(o)),
        }
    }

    /// A box given as bottom, right, top, left.
    fn box_points(&self, b: &mut Bytes) -> Option<(Point, Point)> {
        let (bottom, right, top, left) = (b.i16()?, b.i16()?, b.i16()?, b.i16()?);
        Some((
            self.to_mm(f64::from(left), f64::from(top)),
            self.to_mm(f64::from(right), f64::from(bottom)),
        ))
    }
}

/// Reads the vector artwork of a WMF file.
pub fn import_wmf(data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    let bad = |m: &str| SvgError::Parse(format!("WMF: {m}"));
    let mut b = Bytes { data, pos: 0 };
    let mut w = Wmf {
        dc: Dc {
            window_org: (0.0, 0.0),
            window_ext: None,
            brush: Some(emb_model::Rgb(255, 255, 255)),
            pen: Some((emb_model::Rgb(0, 0, 0), 0.0)),
        },
        saved: Vec::new(),
        objects: Vec::new(),
        frame: None,
        mm: 25.4 / 96.0,
    };
    if b.u32() == Some(PLACEABLE_KEY) {
        b.u16();
        let (l, t, r, bt) = (b.i16(), b.i16(), b.i16(), b.i16());
        let inch = b.u16().unwrap_or(0);
        if let (Some(l), Some(t), Some(r), Some(bt)) = (l, t, r, bt)
            && inch > 0
        {
            w.frame = Some((f64::from(l), f64::from(t), f64::from(r - l), f64::from(bt - t)));
            w.mm = 25.4 / f64::from(inch);
        }
        b.pos = 22;
    } else {
        b.pos = 0;
    }
    // The header: type (1 or 2), header size (9 words), version, ...
    let start = b.pos;
    let (kind, words) = (b.u16(), b.u16());
    if !matches!(kind, Some(1 | 2)) || words != Some(9) {
        return Err(bad("not a WMF file"));
    }
    let mut pos = start + 18;
    let mut p = Painter::new(vectors);
    while pos + 6 <= data.len() {
        let mut r = Bytes { data, pos };
        let (Some(size), Some(func)) = (r.u32(), r.u16()) else {
            break;
        };
        let size = size as usize * 2;
        if size < 6 || pos + size > data.len() {
            return Err(bad("a record is cut off"));
        }
        let mut b = Bytes {
            data: &data[..pos + size],
            pos: pos + 6,
        };
        p.brush = w.dc.brush;
        p.pen = w.dc.pen.map(|(c, width)| (c, w.len_mm(width)));
        match func {
            0x0000 => break,
            0x020B => {
                if let (Some(y), Some(x)) = (b.i16(), b.i16()) {
                    w.dc.window_org = (f64::from(x), f64::from(y));
                }
            }
            0x020C => {
                if let (Some(y), Some(x)) = (b.i16(), b.i16()) {
                    w.dc.window_ext = Some((f64::from(x), f64::from(y)));
                }
            }
            0x0214 => {
                if let (Some(y), Some(x)) = (b.i16(), b.i16()) {
                    p.move_to(w.to_mm(f64::from(x), f64::from(y)));
                }
            }
            0x0213 => {
                if let (Some(y), Some(x)) = (b.i16(), b.i16()) {
                    p.line_to(w.to_mm(f64::from(x), f64::from(y)));
                }
            }
            0x0324 | 0x0325 => {
                let n = b.i16().unwrap_or(0).max(0) as usize;
                let pts = w.points(&mut b, n);
                p.poly(&pts, func == 0x0324);
            }
            0x0538 => {
                let n = b.i16().unwrap_or(0).max(0) as usize;
                let counts: Vec<usize> = (0..n).map_while(|_| b.i16().map(|c| c.max(0) as usize)).collect();
                let polys: Vec<Vec<Point>> = counts.iter().map(|c| w.points(&mut b, *c)).collect();
                p.poly_polygon(&polys);
            }
            0x041B | 0x061C | 0x0418 => {
                // RoundRect has the corner size first.
                if func == 0x061C {
                    b.pos += 4;
                }
                if let Some((a, c)) = w.box_points(&mut b) {
                    if func == 0x0418 { p.ellipse(a, c) } else { p.rect(a, c) }
                }
            }
            // CreateBrushIndirect: style, color, hatch.
            0x02FC => {
                let (style, c) = (b.u16(), b.u32());
                let brush = match (style, c) {
                    (Some(1), _) => None,
                    (_, Some(c)) => Some(colorref(c)),
                    _ => None,
                };
                w.add(MetaObject::Brush(brush));
            }
            // CreatePenIndirect: style, width (x, y), color.
            0x02FA => {
                let (style, width, _) = (b.u16(), b.i16(), b.i16());
                let c = b.u32();
                let pen = match (style, width, c) {
                    (Some(s), Some(wd), Some(c)) if s & 0xf != 5 => Some((colorref(c), f64::from(wd))),
                    _ => None,
                };
                w.add(MetaObject::Pen(pen));
            }
            // Pattern brushes: a gray brush. Fonts, palettes and regions take
            // a place in the table too.
            0x01F9 | 0x0142 => w.add(MetaObject::Brush(Some(crate::svg::FALLBACK))),
            0x02FB | 0x00F7 | 0x06FF => w.add(MetaObject::Other),
            0x012D => {
                let i = b.u16().unwrap_or(0) as usize;
                match w.objects.get(i).copied().flatten() {
                    Some(MetaObject::Brush(c)) => w.dc.brush = c,
                    Some(MetaObject::Pen(pen)) => w.dc.pen = pen,
                    _ => {}
                }
            }
            0x01F0 => {
                if let Some(s) = b.u16().and_then(|i| w.objects.get_mut(i as usize)) {
                    *s = None;
                }
            }
            0x001E => w.saved.push(w.dc),
            0x0127 => {
                if let Some(dc) = w.saved.pop() {
                    w.dc = dc;
                }
            }
            0x0521 | 0x0A32 => p.text += 1,
            0x0F43 | 0x0B41 | 0x0940 | 0x0B23 | 0x0922 => p.images += 1,
            _ => {}
        }
        pos += size;
    }
    Ok(p.finish())
}

#[cfg(test)]
mod tests {
    use emb_model::{ObjectKind, Rgb};

    use super::*;

    /// Writes a placeable WMF with a frame of 0..1000 units at 1000 units
    /// per inch, and these records (function, 16-bit words).
    fn wmf(records: &[(u16, Vec<u16>)]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&PLACEABLE_KEY.to_le_bytes());
        for v in [0u16, 0, 0, 1000, 1000, 1000, 0, 0, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        // Header: type 1, 9 words, version 0x300, size, objects 4, max, 0.
        for v in [1u16, 9, 0x300, 0, 0, 4, 0, 0, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        let mut rec = |func: u16, words: &[u16]| {
            d.extend_from_slice(&(3 + words.len() as u32).to_le_bytes());
            d.extend_from_slice(&func.to_le_bytes());
            for w in words {
                d.extend_from_slice(&w.to_le_bytes());
            }
        };
        for (f, w) in records {
            rec(*f, w);
        }
        rec(0, &[]);
        d
    }

    #[test]
    fn polygon_and_rectangle_with_objects_in_mm() {
        let data = wmf(&[
            // Window 0..1000: the window fills the frame.
            (0x020C, vec![1000, 1000]),
            // Brush 0: green; pen 1: null.
            (0x02FC, vec![0, 0xff00, 0x0000, 0]),
            (0x02FA, vec![5, 0, 0, 0, 0]),
            (0x012D, vec![0]),
            (0x012D, vec![1]),
            // A triangle: 3 points.
            (0x0324, vec![3, 0, 0, 500, 0, 500, 250]),
            // Delete brush 0; a new red brush takes its place (index 0).
            (0x01F0, vec![0]),
            (0x02FC, vec![0, 0x00ff, 0x0000, 0]),
            (0x012D, vec![0]),
            // A rectangle: bottom, right, top, left.
            (0x041B, vec![1000, 1000, 900, 900]),
        ]);
        let r = import_wmf(&data, false).unwrap();
        assert_eq!(r.objects.len(), 2);
        assert_eq!(r.objects[0].color, Rgb(0, 255, 0));
        let b = r.objects[0].kind.control_bbox().unwrap();
        // 500 units at 1000 per inch: 12.7 mm.
        assert!(
            (b.width() - 12.7).abs() < 1e-9 && (b.height() - 6.35).abs() < 1e-9,
            "{b:?}"
        );
        assert_eq!(r.objects[1].color, Rgb(255, 0, 0));
        assert!(matches!(r.objects[1].kind, ObjectKind::Fill { .. }));
    }

    #[test]
    fn poly_polygon_has_a_hole_and_wrong_files() {
        let data = wmf(&[
            (0x020C, vec![1000, 1000]),
            (0x02FA, vec![5, 0, 0, 0, 0]),
            (0x012D, vec![0]),
            (
                0x0538,
                vec![
                    2, 4, 4, 0, 0, 400, 0, 400, 400, 0, 400, 100, 100, 300, 100, 300, 300, 100, 300,
                ],
            ),
        ]);
        let r = import_wmf(&data, false).unwrap();
        assert_eq!(r.objects.len(), 1);
        let ObjectKind::Fill { holes, .. } = &r.objects[0].kind else {
            panic!()
        };
        assert_eq!(holes.len(), 1);
        assert!(import_wmf(b"not a wmf file at all", false).is_err());
    }
}
