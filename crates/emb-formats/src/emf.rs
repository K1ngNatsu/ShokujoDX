//! EMF (Windows enhanced metafile) artwork import (P6 Milestone 5; ADR
//! 0027), from the public [MS-EMF] specification.
//!
//! The reader runs the drawing records: polygons, polylines and Bezier
//! curves (32-bit and 16-bit points), rectangles and ellipses, path
//! brackets (BeginPath … FillPath, StrokePath, StrokeAndFillPath), pens
//! and brushes (also the stock objects), SaveDC and RestoreDC, the world
//! transform and the mapping mode with the window and viewport. Text and
//! bitmaps are not imported (warnings). EMF+ records in comments are not
//! read; EMF+ files keep EMF records for readers that do not read EMF+.
//!
//! Device units go to mm with the size of the reference device in the
//! header (pixels and mm).
//!
//! The writer ([`write_emf`], P10 Milestone 6) writes each shape as a path
//! (MoveTo, LineTo, PolyBezierTo, CloseFigure) in a path bracket, filled
//! (even-odd) and stroked with its own pen and brush. One device unit is
//! 0.01 mm, so the file has the real size.

use emb_geom::Point;

use crate::meta::{Bytes, MetaObject, Painter, colorref, stock};
use crate::svg::{SvgError, SvgImport};

const EMR_HEADER: u32 = 1;
const EMR_EOF: u32 = 14;
/// The "EMF" signature in the header.
const SIGNATURE: u32 = 0x464D_4520;

/// The transform state of the device context.
#[derive(Debug, Clone, Copy)]
struct Dc {
    /// World transform: m11 m12 m21 m22 dx dy.
    world: [f64; 6],
    map_mode: u32,
    window_org: (f64, f64),
    window_ext: Option<(f64, f64)>,
    viewport_org: (f64, f64),
    viewport_ext: Option<(f64, f64)>,
    brush: Option<emb_model::Rgb>,
    pen: Option<(emb_model::Rgb, f64)>,
}

impl Dc {
    fn new() -> Dc {
        Dc {
            world: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            map_mode: 1,
            window_org: (0.0, 0.0),
            window_ext: None,
            viewport_org: (0.0, 0.0),
            viewport_ext: None,
            brush: Some(emb_model::Rgb(255, 255, 255)),
            pen: Some((emb_model::Rgb(0, 0, 0), 0.0)),
        }
    }
}

/// `a` then `b` for world transforms (row vectors: p × a × b).
fn mul(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

struct Emf {
    dc: Dc,
    saved: Vec<Dc>,
    objects: Vec<Option<MetaObject>>,
    /// mm per device unit (x, y).
    mm: (f64, f64),
}

impl Emf {
    /// Logical point to mm.
    fn to_mm(&self, x: f64, y: f64) -> Point {
        let w = self.dc.world;
        let (px, py) = (x * w[0] + y * w[2] + w[4], x * w[1] + y * w[3] + w[5]);
        // Metric mapping modes: logical units are physical units, y up.
        let metric = match self.dc.map_mode {
            2 => Some(0.1),
            3 => Some(0.01),
            4 => Some(0.254),
            5 => Some(0.0254),
            6 => Some(25.4 / 1440.0),
            _ => None,
        };
        if let Some(u) = metric {
            return Point::new((px - self.dc.window_org.0) * u, -(py - self.dc.window_org.1) * u);
        }
        let (dx, dy) = match (self.dc.map_mode, self.dc.window_ext, self.dc.viewport_ext) {
            (7 | 8, Some(we), Some(ve)) if we.0 != 0.0 && we.1 != 0.0 => (
                (px - self.dc.window_org.0) * ve.0 / we.0 + self.dc.viewport_org.0,
                (py - self.dc.window_org.1) * ve.1 / we.1 + self.dc.viewport_org.1,
            ),
            _ => (
                px - self.dc.window_org.0 + self.dc.viewport_org.0,
                py - self.dc.window_org.1 + self.dc.viewport_org.1,
            ),
        };
        Point::new(dx * self.mm.0, dy * self.mm.1)
    }

    /// A length in logical units (a pen width) to mm.
    fn len_mm(&self, l: f64) -> f64 {
        let a = self.to_mm(0.0, 0.0);
        let b = self.to_mm(l, 0.0);
        a.distance(b)
    }

    fn points32(&self, b: &mut Bytes, n: usize) -> Vec<Point> {
        (0..n)
            .map_while(|_| Some(self.to_mm(f64::from(b.i32()?), f64::from(b.i32()?))))
            .collect()
    }

    fn points16(&self, b: &mut Bytes, n: usize) -> Vec<Point> {
        (0..n)
            .map_while(|_| Some(self.to_mm(f64::from(b.i16()?), f64::from(b.i16()?))))
            .collect()
    }

    fn set(&mut self, i: usize, o: MetaObject) {
        if self.objects.len() <= i {
            self.objects.resize(i + 1, None);
        }
        self.objects[i] = Some(o);
    }
}

/// Reads the vector artwork of an EMF file.
pub fn import_emf(data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    let bad = |m: &str| SvgError::Parse(format!("EMF: {m}"));
    let mut head = Bytes { data, pos: 0 };
    if head.u32() != Some(EMR_HEADER) {
        return Err(bad("not an EMF file"));
    }
    // Header: type, size, bounds (16), frame (16), signature at byte 40,
    // ... device size in pixels at byte 72, in mm at byte 80.
    let at = |pos: usize| Bytes { data, pos };
    if at(40).u32() != Some(SIGNATURE) {
        return Err(bad("not an EMF file"));
    }
    let (dev_px, dev_mm) = {
        let mut b = at(72);
        let px = (b.i32(), b.i32());
        let mm = (b.i32(), b.i32());
        (px, mm)
    };
    let mm = match (dev_px, dev_mm) {
        ((Some(px), Some(py)), (Some(mx), Some(my))) if px > 0 && py > 0 && mx > 0 && my > 0 => {
            (f64::from(mx) / f64::from(px), f64::from(my) / f64::from(py))
        }
        // 96 dots per inch.
        _ => (25.4 / 96.0, 25.4 / 96.0),
    };
    let mut e = Emf {
        dc: Dc::new(),
        saved: Vec::new(),
        objects: Vec::new(),
        mm,
    };
    let mut p = Painter::new(vectors);
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let mut r = at(pos);
        let (Some(kind), Some(size)) = (r.u32(), r.u32()) else {
            break;
        };
        let size = size as usize;
        if size < 8 || pos + size > data.len() {
            return Err(bad("a record is cut off"));
        }
        let mut b = Bytes {
            data: &data[..pos + size],
            pos: pos + 8,
        };
        p.brush = e.dc.brush;
        p.pen = e.dc.pen.map(|(c, w)| (c, e.len_mm(w)));
        match kind {
            EMR_EOF => break,
            // Polylines and polygons: bounds (16 bytes), count, points.
            2..=6 | 85..=89 => {
                b.pos += 16;
                let n = b.u32().unwrap_or(0) as usize;
                let pts = if kind >= 85 {
                    e.points16(&mut b, n)
                } else {
                    e.points32(&mut b, n)
                };
                match kind {
                    2 | 85 => p.bezier(&pts),
                    3 | 86 => p.poly(&pts, true),
                    4 | 87 => p.poly(&pts, false),
                    5 | 88 => p.curves_to(&pts),
                    _ => {
                        for q in pts {
                            p.line_to(q);
                        }
                    }
                }
            }
            // PolyPolyline and PolyPolygon: bounds, count of polygons, count
            // of points, counts, points.
            7 | 8 | 90 | 91 => {
                b.pos += 16;
                let n = b.u32().unwrap_or(0) as usize;
                b.u32();
                let counts: Vec<usize> = (0..n).map_while(|_| b.u32().map(|c| c as usize)).collect();
                let polys: Vec<Vec<_>> = counts
                    .iter()
                    .map(|c| {
                        if kind >= 90 {
                            e.points16(&mut b, *c)
                        } else {
                            e.points32(&mut b, *c)
                        }
                    })
                    .collect();
                if kind == 8 || kind == 91 {
                    p.poly_polygon(&polys);
                } else {
                    for q in &polys {
                        p.poly(q, false);
                    }
                }
            }
            // Rectangle, RoundRect (as a rectangle), Ellipse: a box.
            42 | 44 | 41 => {
                let (l, t, rr, bb) = (b.i32(), b.i32(), b.i32(), b.i32());
                if let (Some(l), Some(t), Some(rr), Some(bb)) = (l, t, rr, bb) {
                    let a = e.to_mm(f64::from(l), f64::from(t));
                    let c = e.to_mm(f64::from(rr), f64::from(bb));
                    if kind == 41 { p.ellipse(a, c) } else { p.rect(a, c) }
                }
            }
            27 => {
                if let (Some(x), Some(y)) = (b.i32(), b.i32()) {
                    p.move_to(e.to_mm(f64::from(x), f64::from(y)));
                }
            }
            54 => {
                if let (Some(x), Some(y)) = (b.i32(), b.i32()) {
                    p.line_to(e.to_mm(f64::from(x), f64::from(y)));
                }
            }
            59 => {
                p.discard();
                p.in_path = true;
            }
            60 => p.in_path = false,
            61 => p.close_figure(),
            62 => p.paint(true, false),
            63 => p.paint(true, true),
            64 => p.paint(false, true),
            // AbortPath, SelectClipPath, FlattenPath and WidenPath are not drawn.
            67 | 68 => p.discard(),
            33 => e.saved.push(e.dc),
            34 => {
                let n = b.i32().unwrap_or(-1);
                for _ in 0..n.unsigned_abs().max(1) {
                    if let Some(dc) = e.saved.pop() {
                        e.dc = dc;
                    }
                }
            }
            35 | 36 => {
                let m: Vec<f64> = (0..6).map_while(|_| b.f32().map(f64::from)).collect();
                if m.len() == 6 {
                    let x = [m[0], m[1], m[2], m[3], m[4], m[5]];
                    e.dc.world = match (kind, b.u32()) {
                        (35, _) | (36, Some(4)) => x,
                        (36, Some(1)) => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                        (36, Some(2)) => mul(x, e.dc.world),
                        (36, Some(3)) => mul(e.dc.world, x),
                        _ => e.dc.world,
                    };
                }
            }
            9 => e.dc.window_ext = b.i32().zip(b.i32()).map(|(x, y)| (f64::from(x), f64::from(y))),
            10 => {
                if let Some(o) = b.i32().zip(b.i32()) {
                    e.dc.window_org = (f64::from(o.0), f64::from(o.1));
                }
            }
            11 => e.dc.viewport_ext = b.i32().zip(b.i32()).map(|(x, y)| (f64::from(x), f64::from(y))),
            12 => {
                if let Some(o) = b.i32().zip(b.i32()) {
                    e.dc.viewport_org = (f64::from(o.0), f64::from(o.1));
                }
            }
            17 => e.dc.map_mode = b.u32().unwrap_or(1),
            // CreatePen: index, style, width (x of a point), color.
            38 => {
                let (i, style, w) = (b.u32(), b.u32(), b.i32());
                b.i32();
                if let (Some(i), Some(style), Some(w), Some(c)) = (i, style, w, b.u32()) {
                    let pen = (style & 0xf != 5).then(|| (colorref(c), f64::from(w)));
                    e.set(i as usize, MetaObject::Pen(pen));
                }
            }
            // ExtCreatePen: index, bitmap offsets (4 × 4 bytes), style, width,
            // brush style, color.
            95 => {
                let i = b.u32();
                b.pos += 16;
                let (style, w, _bs, c) = (b.u32(), b.u32(), b.u32(), b.u32());
                if let (Some(i), Some(style), Some(w), Some(c)) = (i, style, w, c) {
                    let pen = (style & 0xf != 5).then(|| (colorref(c), f64::from(w)));
                    e.set(i as usize, MetaObject::Pen(pen));
                }
            }
            // CreateBrushIndirect: index, style, color, hatch.
            39 => {
                if let (Some(i), Some(style), Some(c)) = (b.u32(), b.u32(), b.u32()) {
                    let brush = (style != 1).then(|| colorref(c));
                    e.set(i as usize, MetaObject::Brush(brush));
                }
            }
            // Pattern and DIB brushes: a gray brush.
            93 | 94 => {
                if let Some(i) = b.u32() {
                    e.set(i as usize, MetaObject::Brush(Some(crate::svg::FALLBACK)));
                }
            }
            // Fonts, palettes and the other objects.
            82 | 49 | 100 => {
                if let Some(i) = b.u32() {
                    e.set(i as usize, MetaObject::Other);
                }
            }
            37 => {
                let i = b.u32().unwrap_or(0);
                let o = if i & 0x8000_0000 != 0 {
                    stock(i & 0x7fff_ffff)
                } else {
                    e.objects.get(i as usize).copied().flatten()
                };
                match o {
                    Some(MetaObject::Brush(c)) => e.dc.brush = c,
                    Some(MetaObject::Pen(pen)) => e.dc.pen = pen,
                    _ => {}
                }
            }
            40 => {
                if let Some(slot) = b.u32().and_then(|i| e.objects.get_mut(i as usize)) {
                    *slot = None;
                }
            }
            // Text.
            83 | 84 | 96 | 97 => p.text += 1,
            // Bitmaps.
            76 | 77 | 78 | 79 | 80 | 81 | 114 | 116 => p.images += 1,
            _ => {}
        }
        pos += size;
    }
    Ok(p.finish())
}

/// A shape for [`write_emf`]: a path in mm, its fill (even-odd) and its stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct EmfShape {
    pub path: emb_geom::Path,
    pub fill: Option<emb_model::Rgb>,
    pub stroke: Option<emb_model::Rgb>,
    pub stroke_mm: f64,
}

/// Device units of the written files in one mm.
const WRITE_UNITS_PER_MM: f64 = 100.0;

/// The records of a file that [`write_emf`] makes.
struct Records {
    bytes: Vec<u8>,
    count: u32,
}

impl Records {
    fn rec(&mut self, kind: u32, body: &[u8]) {
        self.bytes.extend((kind).to_le_bytes());
        self.bytes.extend((8 + body.len() as u32).to_le_bytes());
        self.bytes.extend(body);
        self.count += 1;
    }
}

fn le(values: &[i32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn to_colorref(c: emb_model::Rgb) -> i32 {
    i32::from(c.0) | (i32::from(c.1) << 8) | (i32::from(c.2) << 16)
}

/// Writes the shapes as an EMF file at real size (see the module notes).
pub fn write_emf(shapes: &[EmfShape]) -> Vec<u8> {
    use emb_geom::PathCmd;
    let u = |p: Point| {
        (
            (p.x * WRITE_UNITS_PER_MM).round() as i32,
            (p.y * WRITE_UNITS_PER_MM).round() as i32,
        )
    };
    let bounds = shapes
        .iter()
        .filter_map(|s| s.path.control_bbox().map(|b| b.expand(s.stroke_mm.max(0.0) / 2.0)))
        .reduce(emb_geom::BBox::union)
        .unwrap_or_else(|| emb_geom::BBox::new(Point::ZERO, Point::ZERO));
    let (x0, y0) = u(bounds.min);
    let (x1, y1) = u(bounds.max);
    let rect = le(&[x0, y0, x1, y1]);
    let mut r = Records {
        bytes: Vec::new(),
        count: 0,
    };
    // Even-odd fill (ALTERNATE).
    r.rec(19, &le(&[1]));
    for s in shapes {
        // Object 1: the brush; object 2: the pen.
        let brush = match s.fill {
            Some(c) => le(&[1, 0, to_colorref(c), 0]),
            None => le(&[1, 1, 0, 0]),
        };
        r.rec(39, &brush);
        let width = (s.stroke_mm.max(0.0) * WRITE_UNITS_PER_MM).round() as i32;
        let pen = match s.stroke {
            Some(c) => le(&[2, 0, width, 0, to_colorref(c)]),
            None => le(&[2, 5, 0, 0, 0]),
        };
        r.rec(38, &pen);
        r.rec(37, &le(&[1]));
        r.rec(37, &le(&[2]));
        r.rec(59, &[]);
        for cmd in &s.path.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    let (x, y) = u(p);
                    r.rec(27, &le(&[x, y]));
                }
                PathCmd::LineTo(p) => {
                    let (x, y) = u(p);
                    r.rec(54, &le(&[x, y]));
                }
                PathCmd::CubicTo(a, b, p) => {
                    let mut body = rect.clone();
                    body.extend(le(&[3]));
                    for q in [a, b, p] {
                        let (x, y) = u(q);
                        body.extend(le(&[x, y]));
                    }
                    r.rec(5, &body);
                }
                PathCmd::Close => r.rec(61, &[]),
            }
        }
        r.rec(60, &[]);
        let paint = match (s.fill.is_some(), s.stroke.is_some()) {
            (true, true) => 63,
            (true, false) => 62,
            _ => 64,
        };
        r.rec(paint, &rect);
        // The stock objects again, so the objects can go.
        r.rec(37, &le(&[0x8000_0000_u32 as i32 | 4]));
        r.rec(37, &le(&[0x8000_0000_u32 as i32 | 8]));
        r.rec(40, &le(&[1]));
        r.rec(40, &le(&[2]));
    }
    r.rec(14, &le(&[0, 16, 20]));
    // The header: bounds and frame (both in 0.01 mm), the signature, the
    // version, the size, the records, the handles, no description and no
    // palette, and a reference device of 1000 mm at 100 units per mm.
    let size = 88 + r.bytes.len();
    let mut out = Vec::with_capacity(size);
    out.extend(EMR_HEADER.to_le_bytes());
    out.extend(88u32.to_le_bytes());
    out.extend(&rect);
    out.extend(&rect);
    out.extend(SIGNATURE.to_le_bytes());
    out.extend(0x0001_0000u32.to_le_bytes());
    out.extend((size as u32).to_le_bytes());
    out.extend((r.count + 1).to_le_bytes());
    out.extend(3u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(le(&[0, 0, 0]));
    out.extend(le(&[100_000, 100_000, 1000, 1000]));
    out.extend(r.bytes);
    out
}

#[cfg(test)]
pub(crate) mod tests {
    #[test]
    fn written_files_read_back() {
        use emb_geom::{BBox, Path};
        let square = Path::rect(BBox::new(Point::new(10.0, 10.0), Point::new(40.0, 30.0)));
        let curve = Path::new().move_to(Point::new(0.0, 50.0)).cubic_to(
            Point::new(10.0, 40.0),
            Point::new(20.0, 60.0),
            Point::new(30.0, 50.0),
        );
        let shapes = [
            EmfShape {
                path: square.clone(),
                fill: Some(Rgb(200, 0, 0)),
                stroke: None,
                stroke_mm: 0.0,
            },
            EmfShape {
                path: curve.clone(),
                fill: None,
                stroke: Some(Rgb(0, 0, 200)),
                stroke_mm: 0.5,
            },
        ];
        let bytes = write_emf(&shapes);
        let back = import_emf(&bytes, true).unwrap();
        assert_eq!(back.objects.len(), 2, "{:?}", back.warnings);
        assert_eq!(back.objects[0].color, Rgb(200, 0, 0));
        assert_eq!(back.objects[1].color, Rgb(0, 0, 200));
        // The same size and place (the reader goes from 0.01 mm to mm).
        let b0 = back.objects[0]
            .kind
            .paths()
            .filter_map(Path::control_bbox)
            .reduce(BBox::union)
            .unwrap();
        assert!(
            (b0.min.x - 10.0).abs() < 0.02 && (b0.max.y - 30.0).abs() < 0.02,
            "{b0:?}"
        );
        let b1 = back.objects[1]
            .kind
            .paths()
            .filter_map(Path::control_bbox)
            .reduce(BBox::union)
            .unwrap();
        let want = curve.control_bbox().unwrap();
        assert!(
            b1.min.distance(want.min) < 0.02 && b1.max.distance(want.max) < 0.02,
            "{b1:?}"
        );
    }

    use emb_model::{ObjectKind, Rgb};

    use super::*;

    /// Writes EMF records.
    pub(crate) struct EmfWriter(pub Vec<u8>);

    impl EmfWriter {
        /// A header for a device of 1000 × 1000 pixels and 100 × 100 mm
        /// (0.1 mm per pixel).
        pub fn new() -> EmfWriter {
            let mut d = Vec::new();
            let mut v = |x: u32| d.extend_from_slice(&x.to_le_bytes());
            v(EMR_HEADER);
            v(88);
            for _ in 0..8 {
                v(0);
            }
            v(SIGNATURE);
            for _ in 0..7 {
                v(0);
            }
            v(1000);
            v(1000);
            v(100);
            v(100);
            EmfWriter(d)
        }

        pub fn record(&mut self, kind: u32, words: &[u32]) {
            self.0.extend_from_slice(&kind.to_le_bytes());
            self.0.extend_from_slice(&(8 + 4 * words.len() as u32).to_le_bytes());
            for w in words {
                self.0.extend_from_slice(&w.to_le_bytes());
            }
        }

        pub fn finish(mut self) -> Vec<u8> {
            self.record(EMR_EOF, &[0, 0, 0]);
            self.0
        }
    }

    fn i(v: i32) -> u32 {
        v as u32
    }

    #[test]
    fn polygon_with_brush_and_pen_in_mm() {
        let mut w = EmfWriter::new();
        // A red brush (0x000000FF) as object 1, a null pen (stock 8).
        w.record(39, &[1, 0, 0x0000_00ff, 0]);
        w.record(37, &[1]);
        w.record(37, &[0x8000_0008]);
        // A polygon of 3 points: bounds, count, points.
        w.record(3, &[0, 0, 0, 0, 3, 0, 0, 200, 0, 200, 100]);
        // A blue pen 10 px wide, a hollow brush, a polyline.
        w.record(38, &[2, 0, 10, 0, 0x00ff_0000]);
        w.record(37, &[2]);
        w.record(37, &[0x8000_0005]);
        w.record(4, &[0, 0, 0, 0, 2, 0, 300, 100, 300]);
        let r = import_emf(&w.finish(), false).unwrap();
        assert_eq!(r.objects.len(), 2);
        assert_eq!(r.objects[0].color, Rgb(255, 0, 0));
        assert!(matches!(r.objects[0].kind, ObjectKind::Fill { .. }));
        let b = r.objects[0].kind.control_bbox().unwrap();
        // 200 px × 0.1 mm.
        assert!(
            (b.width() - 20.0).abs() < 1e-9 && (b.height() - 10.0).abs() < 1e-9,
            "{b:?}"
        );
        assert_eq!(r.objects[1].color, Rgb(0, 0, 255));
        assert!(matches!(r.objects[1].kind, ObjectKind::Running { .. }));
    }

    #[test]
    fn path_bracket_world_transform_and_ellipse() {
        let mut w = EmfWriter::new();
        // World transform: scale 2, move 100 px.
        let f = |v: f32| v.to_bits();
        w.record(35, &[f(2.0), f(0.0), f(0.0), f(2.0), f(100.0), f(0.0)]);
        w.record(59, &[]);
        w.record(27, &[0, 0]);
        w.record(54, &[i(50), 0]);
        w.record(54, &[i(50), i(50)]);
        w.record(61, &[]);
        w.record(60, &[]);
        w.record(63, &[0, 0, 0, 0]);
        w.record(41, &[0, 0, 10, 10]);
        let r = import_emf(&w.finish(), true).unwrap();
        assert_eq!(r.objects.len(), 2);
        let ObjectKind::Vector { path, style } = &r.objects[0].kind else {
            panic!()
        };
        // The default brush is white, the default pen black.
        assert_eq!(style.fill, Some(Rgb(255, 255, 255)));
        let b = path.control_bbox().unwrap();
        assert!(
            (b.min.x - 10.0).abs() < 1e-9 && (b.width() - 10.0).abs() < 1e-9,
            "{b:?}"
        );
        let ObjectKind::Vector { path, .. } = &r.objects[1].kind else {
            panic!()
        };
        assert!((path.control_bbox().unwrap().width() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn wrong_files() {
        assert!(import_emf(b"not an emf", false).is_err());
        // A polygon record that says 200 bytes, with 8 bytes in the file.
        let mut d = EmfWriter::new().0;
        d.extend_from_slice(&[3, 0, 0, 0, 200, 0, 0, 0]);
        assert!(import_emf(&d, false).is_err(), "a record that is cut off");
    }
}
