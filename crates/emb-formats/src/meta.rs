//! Parts that the Windows metafile readers (EMF and WMF) share: the pens
//! and brushes, and a painter that collects the shapes of a drawing and
//! paints them (P6 Milestone 5; ADR 0027).

use emb_geom::{Path, Point};
use emb_model::Rgb;

use crate::svg::{Painted, SvgImport, add_painted};

/// The control point distance of a quarter ellipse made of one cubic curve.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// A pen or a brush of the object table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MetaObject {
    /// A brush: its color, or `None` for a hollow (null) brush.
    Brush(Option<Rgb>),
    /// A pen: its color and width in logical units, or `None` for a null pen.
    Pen(Option<(Rgb, f64)>),
    /// A font, palette, region or other object that does not draw shapes.
    Other,
}

/// A COLORREF (0x00BBGGRR) as RGB.
pub(crate) fn colorref(c: u32) -> Rgb {
    Rgb((c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8)
}

/// The stock objects of GDI (index | 0x80000000 in EMF).
pub(crate) fn stock(i: u32) -> Option<MetaObject> {
    Some(match i {
        0 => MetaObject::Brush(Some(Rgb(255, 255, 255))),
        1 => MetaObject::Brush(Some(Rgb(192, 192, 192))),
        2 => MetaObject::Brush(Some(Rgb(128, 128, 128))),
        3 => MetaObject::Brush(Some(Rgb(64, 64, 64))),
        4 => MetaObject::Brush(Some(Rgb(0, 0, 0))),
        5 => MetaObject::Brush(None),
        6 => MetaObject::Pen(Some((Rgb(255, 255, 255), 0.0))),
        7 => MetaObject::Pen(Some((Rgb(0, 0, 0), 0.0))),
        8 => MetaObject::Pen(None),
        _ => return None,
    })
}

/// Collects shapes and paints them with the current brush and pen.
pub(crate) struct Painter {
    pub out: SvgImport,
    pub vectors: bool,
    pub brush: Option<Rgb>,
    /// The pen color and width in mm.
    pub pen: Option<(Rgb, f64)>,
    /// Inside a path bracket (BeginPath … EndPath), shapes wait for a
    /// FillPath or StrokePath.
    pub in_path: bool,
    /// Inside a PolyPolygon: the polygons wait to be painted as one shape.
    poly_waits: bool,
    path: Path,
    subs: Vec<Path>,
    pub pen_at: Point,
    start: Point,
    count: usize,
    pub text: usize,
    pub images: usize,
}

impl Painter {
    pub fn new(vectors: bool) -> Painter {
        Painter {
            out: SvgImport::default(),
            vectors,
            brush: Some(Rgb(255, 255, 255)),
            pen: Some((Rgb(0, 0, 0), 0.1)),
            in_path: false,
            poly_waits: false,
            path: Path::new(),
            subs: Vec::new(),
            pen_at: Point::ZERO,
            start: Point::ZERO,
            count: 0,
            text: 0,
            images: 0,
        }
    }

    fn end_sub(&mut self) {
        if self.path.cmds.len() > 1 {
            self.subs.push(std::mem::take(&mut self.path));
        }
        self.path = Path::new();
    }

    pub fn move_to(&mut self, p: Point) {
        self.end_sub();
        self.path = Path::new().move_to(p);
        self.pen_at = p;
        self.start = p;
    }

    pub fn line_to(&mut self, p: Point) {
        if self.path.cmds.is_empty() {
            self.path = Path::new().move_to(self.pen_at);
        }
        self.path = std::mem::take(&mut self.path).line_to(p);
        self.pen_at = p;
        if !self.in_path {
            // A line outside a path bracket is drawn at once with the pen.
            self.paint(false, true);
            self.path = Path::new().move_to(p);
        }
    }

    /// Cubic curves from the pen: each three points are two control points
    /// and an end point.
    pub fn curves_to(&mut self, pts: &[Point]) {
        if self.path.cmds.is_empty() {
            self.path = Path::new().move_to(self.pen_at);
        }
        for c in pts.as_chunks::<3>().0 {
            self.path = std::mem::take(&mut self.path).cubic_to(c[0], c[1], c[2]);
            self.pen_at = c[2];
        }
    }

    pub fn close_figure(&mut self) {
        if !self.path.cmds.is_empty() {
            self.path = std::mem::take(&mut self.path).close();
            self.pen_at = self.start;
        }
    }

    /// A polygon or polyline (`closed`); filled and stroked at once
    /// outside a path bracket (a polyline is only stroked).
    pub fn poly(&mut self, pts: &[Point], closed: bool) {
        if pts.len() < 2 {
            return;
        }
        self.end_sub();
        let mut p = Path::new().move_to(pts[0]);
        for q in &pts[1..] {
            p = p.line_to(*q);
        }
        if closed {
            p = p.close();
        }
        self.subs.push(p);
        if !self.in_path && !self.poly_waits {
            self.paint(closed, true);
        }
    }

    /// A Bezier curve set: the first point, then three points per curve.
    pub fn bezier(&mut self, pts: &[Point]) {
        if pts.is_empty() {
            return;
        }
        self.end_sub();
        self.pen_at = pts[0];
        self.path = Path::new().move_to(pts[0]);
        self.curves_to(&pts[1..]);
        if !self.in_path {
            self.paint(false, true);
        }
    }

    /// The polygons of a PolyPolygon: one shape, filled even-odd.
    pub fn poly_polygon(&mut self, polys: &[Vec<Point>]) {
        self.poly_waits = true;
        for p in polys {
            self.poly(p, true);
        }
        self.poly_waits = false;
        if !self.in_path {
            self.paint(true, true);
        }
    }

    pub fn rect(&mut self, a: Point, b: Point) {
        let pts = [a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
        self.poly(&pts, true);
    }

    pub fn ellipse(&mut self, a: Point, b: Point) {
        self.end_sub();
        let c = a.midpoint(b);
        let (rx, ry) = ((b.x - a.x).abs() / 2.0, (b.y - a.y).abs() / 2.0);
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let p = |x: f64, y: f64| Point::new(c.x + x, c.y + y);
        let e = Path::new()
            .move_to(p(rx, 0.0))
            .cubic_to(p(rx, ky), p(kx, ry), p(0.0, ry))
            .cubic_to(p(-kx, ry), p(-rx, ky), p(-rx, 0.0))
            .cubic_to(p(-rx, -ky), p(-kx, -ry), p(0.0, -ry))
            .cubic_to(p(kx, -ry), p(rx, -ky), p(rx, 0.0))
            .close();
        self.subs.push(e);
        if !self.in_path {
            self.paint(true, true);
        }
    }

    /// Paints the collected shapes with the brush (`fill`) and the pen
    /// (`stroke`), and forgets them.
    pub fn paint(&mut self, fill: bool, stroke: bool) {
        self.end_sub();
        let subs = std::mem::take(&mut self.subs);
        let fill = self.brush.filter(|_| fill);
        let stroke = self.pen.filter(|_| stroke);
        if subs.is_empty() || (fill.is_none() && stroke.is_none()) {
            return;
        }
        self.count += 1;
        let painted = Painted {
            subs,
            fill,
            stroke: stroke.map(|(c, w)| (c, w.max(0.1))),
            name: Some(format!("shape {}", self.count)),
        };
        add_painted(&mut self.out, painted, self.vectors);
    }

    /// Forgets the collected shapes (AbortPath, a clip path).
    pub fn discard(&mut self) {
        self.end_sub();
        self.subs.clear();
    }

    /// The import with warnings for text and images.
    pub fn finish(mut self) -> SvgImport {
        if self.text > 0 {
            self.out.warnings.push(format!(
                "{} text record(s) not imported. Convert text to outlines first.",
                self.text
            ));
        }
        if self.images > 0 {
            self.out
                .warnings
                .push(format!("{} image(s) not imported.", self.images));
        }
        self.out
    }
}

/// A little-endian reader of record data.
pub(crate) struct Bytes<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl Bytes<'_> {
    pub fn u16(&mut self) -> Option<u16> {
        let b = self.data.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn i16(&mut self) -> Option<i16> {
        self.u16().map(|v| v as i16)
    }
    pub fn u32(&mut self) -> Option<u32> {
        let b = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    pub fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
}
