//! PDF artwork import, also AI artwork files that are PDF files inside (P6 Milestone 5; ADR 0027).
//!
//! The reader runs the drawing operators of the first page: paths (`m`,
//! `l`, `c`, `v`, `y`, `h`, `re`), painting (`f`, `S`, `B` and the
//! others), colors (gray, RGB and CMYK), the transform (`cm`, `q`, `Q`)
//! and form XObjects (`Do`). Clipping is not used. Text and images are
//! not imported (warnings). The PDF file structure is read with `lopdf`.
//!
//! PDF units are points (1/72 inch) with y up; the result is in mm with
//! y down and the top-left of the page at (0, 0).

use emb_geom::{Path, Point};
use emb_model::Rgb;
use lopdf::{Dictionary, Document, Object, content::Content};

use crate::svg::{Painted, SvgError, SvgImport, add_painted};

/// mm per PDF point.
const MM_PER_PT: f64 = 25.4 / 72.0;
/// Form XObjects can draw other forms; stop at this depth.
const MAX_FORM_DEPTH: usize = 12;

/// A 2D affine transform as in PDF: [a b c d e f].
#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix([f64; 6]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// `self` first, then `o` (PDF: `self × o`).
    fn then(&self, o: &Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = o.0;
        Matrix([
            a * a2 + b * c2,
            a * b2 + b * d2,
            c * a2 + d * c2,
            c * b2 + d * d2,
            e * a2 + f * c2 + e2,
            e * b2 + f * d2 + f2,
        ])
    }

    /// How much the transform scales lengths (for line widths).
    fn scale(&self) -> f64 {
        let [a, b, c, d, ..] = self.0;
        (a * d - b * c).abs().sqrt()
    }
}

#[derive(Debug, Clone, Copy)]
struct GState {
    ctm: Matrix,
    fill: Rgb,
    stroke: Rgb,
    line_width: f64,
}

/// The state of the reader while it runs the operators.
struct Reader<'a> {
    doc: &'a Document,
    /// Device (points, y up) to mm (y down).
    page: Matrix,
    out: SvgImport,
    vectors: bool,
    path: Path,
    subs: Vec<Path>,
    start: Point,
    pen: Point,
    count: usize,
    text: usize,
    images: usize,
    patterns: usize,
}

fn num(o: &Object) -> f64 {
    o.as_float().map_or(0.0, f64::from)
}

fn nums(ops: &[Object]) -> Vec<f64> {
    ops.iter().map(num).collect()
}

fn cmyk(c: f64, m: f64, y: f64, k: f64) -> Rgb {
    let ch = |v: f64| ((1.0 - v.clamp(0.0, 1.0)) * (1.0 - k.clamp(0.0, 1.0)) * 255.0).round() as u8;
    Rgb(ch(c), ch(m), ch(y))
}

fn gray(g: f64) -> Rgb {
    let v = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb(v, v, v)
}

fn rgb(r: f64, g: f64, b: f64) -> Rgb {
    let ch = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb(ch(r), ch(g), ch(b))
}

/// A color from the operands of `sc`, `scn`, `SC` or `SCN`: one value is
/// gray, three are RGB, four are CMYK. A pattern name gives `None`.
fn any_color(ops: &[Object]) -> Option<Rgb> {
    if ops.iter().any(|o| matches!(o, Object::Name(_))) {
        return None;
    }
    let v = nums(ops);
    match v.len() {
        1 => Some(gray(v[0])),
        3 => Some(rgb(v[0], v[1], v[2])),
        4 => Some(cmyk(v[0], v[1], v[2], v[3])),
        _ => None,
    }
}

impl Reader<'_> {
    fn to_mm(&self, g: &GState, x: f64, y: f64) -> Point {
        let (dx, dy) = g.ctm.apply(x, y);
        let (mx, my) = self.page.apply(dx, dy);
        Point::new(mx, my)
    }

    fn end_sub(&mut self) {
        if self.path.cmds.len() > 1 {
            self.subs.push(std::mem::take(&mut self.path));
        }
        self.path = Path::new();
    }

    fn paint(&mut self, g: &GState, fill: bool, stroke: bool) {
        self.end_sub();
        let subs = std::mem::take(&mut self.subs);
        if !(fill || stroke) || subs.is_empty() {
            return;
        }
        self.count += 1;
        let width = (g.line_width * g.ctm.scale()).max(0.0) * MM_PER_PT;
        let painted = Painted {
            subs,
            fill: fill.then_some(g.fill),
            stroke: stroke.then_some((g.stroke, width.max(0.1))),
            name: Some(format!("path {}", self.count)),
        };
        add_painted(&mut self.out, painted, self.vectors);
    }

    /// Runs the operators of a content stream with `resources`.
    fn run(&mut self, data: &[u8], resources: Option<&Dictionary>, g0: GState, depth: usize) -> Result<(), SvgError> {
        let content = Content::decode(data).map_err(|e| SvgError::Parse(format!("PDF content: {e}")))?;
        let mut g = g0;
        let mut stack: Vec<GState> = Vec::new();
        for op in &content.operations {
            let o = &op.operands;
            let v = || nums(o);
            match op.operator.as_str() {
                "q" => stack.push(g),
                "Q" => g = stack.pop().unwrap_or(g0),
                "cm" => {
                    let m = v();
                    if m.len() == 6 {
                        g.ctm = Matrix([m[0], m[1], m[2], m[3], m[4], m[5]]).then(&g.ctm);
                    }
                }
                "w" => g.line_width = v().first().copied().unwrap_or(1.0),
                "g" => g.fill = gray(v().first().copied().unwrap_or(0.0)),
                "G" => g.stroke = gray(v().first().copied().unwrap_or(0.0)),
                "rg" | "RG" => {
                    let c = v();
                    if c.len() == 3 {
                        let col = rgb(c[0], c[1], c[2]);
                        if op.operator == "rg" {
                            g.fill = col;
                        } else {
                            g.stroke = col;
                        }
                    }
                }
                "k" | "K" => {
                    let c = v();
                    if c.len() == 4 {
                        let col = cmyk(c[0], c[1], c[2], c[3]);
                        if op.operator == "k" {
                            g.fill = col;
                        } else {
                            g.stroke = col;
                        }
                    }
                }
                "sc" | "scn" | "SC" | "SCN" => {
                    let col = any_color(o).unwrap_or_else(|| {
                        self.patterns += 1;
                        crate::svg::FALLBACK
                    });
                    if op.operator.starts_with('s') {
                        g.fill = col;
                    } else {
                        g.stroke = col;
                    }
                }
                "m" => {
                    let p = v();
                    if p.len() == 2 {
                        self.end_sub();
                        let q = self.to_mm(&g, p[0], p[1]);
                        self.path = Path::new().move_to(q);
                        self.start = q;
                        self.pen = q;
                    }
                }
                "l" => {
                    let p = v();
                    if p.len() == 2 {
                        let q = self.to_mm(&g, p[0], p[1]);
                        if self.path.cmds.is_empty() {
                            self.path = Path::new().move_to(self.pen);
                        }
                        self.path = std::mem::take(&mut self.path).line_to(q);
                        self.pen = q;
                    }
                }
                "c" | "v" | "y" => {
                    let p = v();
                    let (c1, c2, end) = match (op.operator.as_str(), p.len()) {
                        ("c", 6) => (
                            self.to_mm(&g, p[0], p[1]),
                            self.to_mm(&g, p[2], p[3]),
                            self.to_mm(&g, p[4], p[5]),
                        ),
                        ("v", 4) => (self.pen, self.to_mm(&g, p[0], p[1]), self.to_mm(&g, p[2], p[3])),
                        ("y", 4) => {
                            let e = self.to_mm(&g, p[2], p[3]);
                            (self.to_mm(&g, p[0], p[1]), e, e)
                        }
                        _ => continue,
                    };
                    if self.path.cmds.is_empty() {
                        self.path = Path::new().move_to(self.pen);
                    }
                    self.path = std::mem::take(&mut self.path).cubic_to(c1, c2, end);
                    self.pen = end;
                }
                "h" => {
                    if !self.path.cmds.is_empty() {
                        self.path = std::mem::take(&mut self.path).close();
                        self.pen = self.start;
                    }
                }
                "re" => {
                    let r = v();
                    if r.len() == 4 {
                        self.end_sub();
                        let (x, y, w, h) = (r[0], r[1], r[2], r[3]);
                        let pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|(a, b)| self.to_mm(&g, a, b));
                        self.subs.push(Path::polygon(&pts));
                        self.pen = pts[0];
                    }
                }
                "f" | "F" | "f*" => self.paint(&g, true, false),
                "S" => self.paint(&g, false, true),
                "s" => {
                    self.path = std::mem::take(&mut self.path).closed();
                    self.paint(&g, false, true);
                }
                "B" | "B*" => self.paint(&g, true, true),
                "b" | "b*" => {
                    self.path = std::mem::take(&mut self.path).closed();
                    self.paint(&g, true, true);
                }
                // End the path with no paint (also after a clip, W or W*).
                "n" => {
                    self.end_sub();
                    self.subs.clear();
                }
                "BT" => self.text += 1,
                "BI" => self.images += 1,
                "Do" => {
                    let Some(name) = o.first().and_then(|n| n.as_name().ok()) else {
                        continue;
                    };
                    self.x_object(name, resources, &g, depth)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Draws XObject `name`: a form runs its own operators; an image is
    /// counted.
    fn x_object(
        &mut self,
        name: &[u8],
        resources: Option<&Dictionary>,
        g: &GState,
        depth: usize,
    ) -> Result<(), SvgError> {
        let doc = self.doc;
        let Some(stream) = resources
            .and_then(|r| r.get_deref(b"XObject", doc).ok())
            .and_then(|x| x.as_dict().ok())
            .and_then(|x| x.get_deref(name, doc).ok())
            .and_then(|s| s.as_stream().ok())
        else {
            return Ok(());
        };
        let subtype = stream.dict.get(b"Subtype").and_then(Object::as_name).unwrap_or(b"");
        if subtype == b"Image" {
            self.images += 1;
            return Ok(());
        }
        if subtype != b"Form" || depth >= MAX_FORM_DEPTH {
            return Ok(());
        }
        let m = stream
            .dict
            .get(b"Matrix")
            .and_then(Object::as_array)
            .map(|a| nums(a))
            .ok()
            .filter(|m| m.len() == 6)
            .map_or(Matrix::IDENTITY, |m| Matrix([m[0], m[1], m[2], m[3], m[4], m[5]]));
        let inner = GState {
            ctm: m.then(&g.ctm),
            ..*g
        };
        let res = stream
            .dict
            .get_deref(b"Resources", doc)
            .ok()
            .and_then(|r| r.as_dict().ok())
            .or(resources);
        let data = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());
        // Paths are not shared across a form.
        self.end_sub();
        self.subs.clear();
        self.run(&data, res, inner, depth + 1)
    }
}

/// The value of a page key, from the page or from its parents.
fn inherited<'a>(doc: &'a Document, page: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    let mut d = page;
    for _ in 0..MAX_FORM_DEPTH {
        if let Ok(v) = d.get_deref(key, doc) {
            return Some(v);
        }
        d = d.get_deref(b"Parent", doc).ok()?.as_dict().ok()?;
    }
    None
}

/// Reads the vector artwork of the first page of a PDF (or AI) file.
/// `vectors`: one vector object for each painted path, else fills and
/// running lines (as the SVG import).
pub fn import_pdf(data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    if !data.starts_with(b"%PDF") {
        if data.starts_with(b"%!PS") {
            return Err(SvgError::Parse(
                "this is a PostScript file (an old AI or EPS file): save it as PDF or SVG first".into(),
            ));
        }
        return Err(SvgError::Parse("not a PDF file".into()));
    }
    let doc = Document::load_mem(data).map_err(|e| SvgError::Parse(format!("PDF: {e}")))?;
    let pages = doc.get_pages();
    let (_, &page_id) = pages
        .iter()
        .next()
        .ok_or_else(|| SvgError::Parse("the PDF has no pages".into()))?;
    let page = doc
        .get_dictionary(page_id)
        .map_err(|e| SvgError::Parse(format!("PDF page: {e}")))?;
    let media = inherited(&doc, page, b"MediaBox")
        .and_then(|m| m.as_array().ok())
        .map(|a| nums(a))
        .filter(|m| m.len() == 4)
        .unwrap_or_else(|| vec![0.0, 0.0, 612.0, 792.0]);
    let (x0, y1) = (media[0].min(media[2]), media[1].max(media[3]));
    // Points with y up to mm with y down, from the top-left of the page.
    let page_to_mm = Matrix([MM_PER_PT, 0.0, 0.0, -MM_PER_PT, -x0 * MM_PER_PT, y1 * MM_PER_PT]);
    let resources = inherited(&doc, page, b"Resources").and_then(|r| r.as_dict().ok());
    let content = doc.get_page_content(page_id);
    let mut r = Reader {
        doc: &doc,
        page: page_to_mm,
        out: SvgImport::default(),
        vectors,
        path: Path::new(),
        subs: Vec::new(),
        start: Point::ZERO,
        pen: Point::ZERO,
        count: 0,
        text: 0,
        images: 0,
        patterns: 0,
    };
    let g = GState {
        ctm: Matrix::IDENTITY,
        fill: Rgb(0, 0, 0),
        stroke: Rgb(0, 0, 0),
        line_width: 1.0,
    };
    r.run(&content, resources, g, 0)?;
    let mut out = r.out;
    if pages.len() > 1 {
        out.warnings
            .push(format!("Only the first of {} pages imported.", pages.len()));
    }
    if r.text > 0 {
        out.warnings.push(format!(
            "{} text block(s) not imported. Convert text to outlines first.",
            r.text
        ));
    }
    if r.images > 0 {
        out.warnings.push(format!("{} image(s) not imported.", r.images));
    }
    if r.patterns > 0 {
        out.warnings
            .push(format!("{} gradient or pattern paint(s) imported as gray.", r.patterns));
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use emb_model::ObjectKind;
    use lopdf::{Stream, dictionary};

    use super::*;

    /// A one-page PDF (100 × 100 pt) with these page operators.
    pub(crate) fn pdf(ops: &str) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content = doc.add_object(Stream::new(dictionary! {}, ops.as_bytes().to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page.into()],
                "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn filled_ring_and_stroked_line_in_mm() {
        // A 72 pt square with a square hole (even-odd), red; a blue line.
        let data = pdf("1 0 0 rg 0 0 72 72 re 18 18 36 36 re f* \
                        0 0 1 RG 2 w 0 100 m 72 100 l S");
        let r = import_pdf(&data, false).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.objects.len(), 2);
        let ObjectKind::Fill { outer, holes, .. } = &r.objects[0].kind else {
            panic!("not a fill")
        };
        assert_eq!(holes.len(), 1);
        assert_eq!(r.objects[0].color, Rgb(255, 0, 0));
        // 72 pt is 25.4 mm; y down from the top of the 100 pt page.
        let b = outer.control_bbox().unwrap();
        assert!((b.width() - 25.4).abs() < 1e-6, "{b:?}");
        assert!((b.max.y - 100.0 * MM_PER_PT).abs() < 1e-6, "{b:?}");
        assert!(matches!(r.objects[1].kind, ObjectKind::Running { .. }));
        assert_eq!(r.objects[1].color, Rgb(0, 0, 255));
        // The line is at the top of the page (y = 100 pt).
        assert!(r.objects[1].kind.control_bbox().unwrap().min.y.abs() < 1e-6);
    }

    #[test]
    fn transforms_curves_cmyk_and_vectors() {
        // Scale 2 and move; a curve closed and filled and stroked in CMYK.
        let data = pdf("q 2 0 0 2 10 10 cm 0 1 1 0 k 0 0 0 1 K 0.5 w \
                        0 0 m 10 0 l 10 10 10 10 0 10 c h B Q 0.5 g 0 0 1 1 re f");
        let r = import_pdf(&data, true).unwrap();
        assert_eq!(r.objects.len(), 2);
        let ObjectKind::Vector { path, style } = &r.objects[0].kind else {
            panic!("not a vector")
        };
        assert_eq!(style.fill, Some(Rgb(255, 0, 0)), "CMYK 0 1 1 0 is red");
        assert_eq!(style.stroke, Some(Rgb(0, 0, 0)));
        // Line width 0.5 pt × scale 2 = 1 pt.
        assert!((style.stroke_mm - MM_PER_PT).abs() < 1e-6);
        let b = path.control_bbox().unwrap();
        assert!((b.width() - 20.0 * MM_PER_PT).abs() < 1e-6, "{b:?}");
        assert!((b.min.x - 10.0 * MM_PER_PT).abs() < 1e-6);
        // After Q the transform is back: a 1 pt gray square at (0, 0).
        let ObjectKind::Vector { style, .. } = &r.objects[1].kind else {
            panic!()
        };
        assert_eq!(style.fill, Some(Rgb(128, 128, 128)));
    }

    #[test]
    fn text_images_and_wrong_files() {
        let data = pdf("BT /F1 12 Tf (Hi) Tj ET 0 0 10 10 re W n 0 0 5 5 re f");
        let r = import_pdf(&data, false).unwrap();
        assert_eq!(r.objects.len(), 1, "the clip path is not imported");
        assert!(r.warnings.iter().any(|w| w.contains("text")));
        assert!(
            import_pdf(b"%!PS-Adobe-3.0", false)
                .unwrap_err()
                .to_string()
                .contains("PostScript")
        );
        assert!(import_pdf(b"hello", false).is_err());
    }
}
