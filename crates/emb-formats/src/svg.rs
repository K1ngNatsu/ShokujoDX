//! SVG import: vector artwork to embroidery objects.
//!
//! - A path with a fill becomes one fill object per outer shape. Subpaths
//!   inside an outer shape become its holes (even-odd nesting).
//! - A path with a stroke becomes one running-stitch object per subpath.
//!   A path with both gets both, the fill first.
//! - The fill or stroke color becomes the thread color. Gradients and
//!   patterns use a gray placeholder and give a warning.
//! - Text and embedded images are not imported (warning). Convert text to
//!   paths in the vector editor first.
//! - [`SvgImport::as_lines`] makes every shape a running-stitch line: the
//!   outline (and the holes) of a fill, and each stroke.
//! - [`import_svg_vectors`] keeps each path as one vector object with no
//!   stitches (ADR 0024), with its fill, stroke and stroke width.
//!
//! SVG user units are CSS pixels (96 per inch); `usvg` resolves physical
//! units such as `mm`. The result is in mm with the SVG origin at (0, 0).

use emb_geom::{BBox, Path, Point, region};
use emb_model::{FillParams, ObjectKind, Rgb, RunningParams, VectorStyle};

/// mm per CSS pixel.
const MM_PER_PX: f64 = 25.4 / 96.0;
/// Placeholder color for paints that are not a plain color.
pub(crate) const FALLBACK: Rgb = Rgb(128, 128, 128);

#[derive(Debug, thiserror::Error)]
pub enum SvgError {
    #[error("cannot read SVG: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedObject {
    pub kind: ObjectKind,
    pub color: Rgb,
    /// The SVG element id, if it has one.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SvgImport {
    /// In SVG paint order (first is painted first, so it sews first).
    pub objects: Vec<ImportedObject>,
    /// Things that were not imported or were changed, for the user.
    pub warnings: Vec<String>,
}

impl SvgImport {
    pub fn bbox(&self) -> Option<BBox> {
        self.objects
            .iter()
            .flat_map(|o| o.kind.paths().filter_map(Path::control_bbox).collect::<Vec<_>>())
            .reduce(BBox::union)
    }

    /// Each object as a running-stitch line: a fill becomes one line object
    /// with its outline and its holes; strokes stay lines.
    pub fn as_lines(mut self) -> SvgImport {
        for o in &mut self.objects {
            if let ObjectKind::Fill { outer, holes, .. } = &o.kind {
                let mut path = outer.clone();
                for h in holes {
                    path.cmds.extend(h.cmds.iter().copied());
                }
                o.kind = ObjectKind::Running {
                    path,
                    params: emb_model::RunningParams::default(),
                };
            }
        }
        self
    }

    /// A new design with the imported objects, centered on the origin (the
    /// hoop center). Objects of one color share a thread.
    pub fn to_design(&self, name: &str) -> emb_model::Design {
        let mut d = emb_model::Design::new(name);
        let shift = self.bbox().map(|b| Point::ZERO - b.center()).unwrap_or(Point::ZERO);
        let t = emb_geom::Transform::translate(shift.x, shift.y);
        for o in &self.objects {
            let thread = d.thread_for_color(o.color);
            let mut kind = o.kind.clone();
            kind.transform(&t);
            let id = d.add_object(thread, kind);
            if let Some(obj) = d.object_mut(id) {
                obj.name = o.name.clone();
            }
        }
        d
    }
}

/// Imports the paths of an SVG file as fill and running-stitch objects.
pub fn import_svg(data: &[u8]) -> Result<SvgImport, SvgError> {
    import(data, false)
}

/// Imports the paths of an SVG file as vector objects (artwork with no
/// stitches): one for each path, with its fill and stroke.
pub fn import_svg_vectors(data: &[u8]) -> Result<SvgImport, SvgError> {
    import(data, true)
}

fn import(data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &opt).map_err(|e| SvgError::Parse(e.to_string()))?;
    let mut out = SvgImport::default();
    let mut counts = Counts {
        vectors,
        ..Counts::default()
    };
    walk(tree.root(), &mut out, &mut counts);
    // usvg is built without text support, so it drops text elements. Count
    // them in the XML, so the user knows that something is missing.
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(doc) = usvg::roxmltree::Document::parse(s)
    {
        counts.text = doc.descendants().filter(|n| n.tag_name().name() == "text").count();
    }
    if counts.text > 0 {
        out.warnings.push(format!(
            "{} text element(s) not imported. Convert text to paths first.",
            counts.text
        ));
    }
    if counts.images > 0 {
        out.warnings.push(format!("{} image(s) not imported.", counts.images));
    }
    if counts.gradients > 0 {
        out.warnings.push(format!(
            "{} gradient or pattern paint(s) imported as gray.",
            counts.gradients
        ));
    }
    Ok(out)
}

#[derive(Default)]
struct Counts {
    /// Import as vector objects.
    vectors: bool,
    text: usize,
    images: usize,
    gradients: usize,
}

fn walk(group: &usvg::Group, out: &mut SvgImport, counts: &mut Counts) {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(g, out, counts),
            usvg::Node::Path(p) => import_path(p, out, counts),
            usvg::Node::Text(_) => counts.text += 1,
            usvg::Node::Image(_) => counts.images += 1,
        }
    }
}

fn paint_color(paint: &usvg::Paint, counts: &mut Counts) -> Rgb {
    match paint {
        usvg::Paint::Color(c) => Rgb(c.red, c.green, c.blue),
        _ => {
            counts.gradients += 1;
            FALLBACK
        }
    }
}

/// Converts the path data with its absolute transform to subpaths in mm.
fn subpaths(p: &usvg::Path) -> Vec<Path> {
    let t = p.abs_transform();
    let map = |q: usvg::tiny_skia_path::Point| {
        let (x, y) = (f64::from(q.x), f64::from(q.y));
        Point::new(
            (x * f64::from(t.sx) + y * f64::from(t.kx) + f64::from(t.tx)) * MM_PER_PX,
            (x * f64::from(t.ky) + y * f64::from(t.sy) + f64::from(t.ty)) * MM_PER_PX,
        )
    };
    let mut out = Vec::new();
    let mut cur = Path::new();
    let mut pen = Point::ZERO;
    for seg in p.data().segments() {
        use usvg::tiny_skia_path::PathSegment as S;
        match seg {
            S::MoveTo(a) => {
                if cur.cmds.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur = Path::new();
                pen = map(a);
                cur = cur.move_to(pen);
            }
            S::LineTo(a) => {
                pen = map(a);
                cur = cur.line_to(pen);
            }
            S::QuadTo(c, a) => {
                // Degree elevation: quadratic to cubic.
                let (c, a) = (map(c), map(a));
                let c1 = pen + (c - pen) * (2.0 / 3.0);
                let c2 = a + (c - a) * (2.0 / 3.0);
                cur = cur.cubic_to(c1, c2, a);
                pen = a;
            }
            S::CubicTo(c1, c2, a) => {
                pen = map(a);
                cur = cur.cubic_to(map(c1), map(c2), pen);
            }
            S::Close => {
                cur = cur.close();
            }
        }
    }
    if cur.cmds.len() > 1 {
        out.push(cur);
    }
    out
}

fn import_path(p: &usvg::Path, out: &mut SvgImport, counts: &mut Counts) {
    if !p.is_visible() {
        return;
    }
    let subs = subpaths(p);
    if subs.is_empty() {
        return;
    }
    let fill = p.fill().map(|f| paint_color(f.paint(), counts));
    let stroke = p.stroke().map(|s| {
        // The width in mm, with the scale of the transform.
        let t = p.abs_transform();
        let scale = f64::from(t.sx * t.sy - t.kx * t.ky).abs().sqrt();
        (
            paint_color(s.paint(), counts),
            f64::from(s.width().get()) * scale * MM_PER_PX,
        )
    });
    let painted = Painted {
        subs,
        fill,
        stroke,
        name: (!p.id().is_empty()).then(|| p.id().to_string()),
    };
    add_painted(out, painted, counts.vectors);
}

/// A painted path of vector artwork (SVG, PDF, EMF, WMF), in mm.
pub(crate) struct Painted {
    pub subs: Vec<Path>,
    pub fill: Option<Rgb>,
    /// The color and the width of the stroke, in mm.
    pub stroke: Option<(Rgb, f64)>,
    pub name: Option<String>,
}

/// Adds the objects of a painted path: one vector (`vectors`), or a fill
/// object for each outer shape with its holes and a running line for each
/// stroked subpath.
pub(crate) fn add_painted(out: &mut SvgImport, p: Painted, vectors: bool) {
    if p.subs.is_empty() || (p.fill.is_none() && p.stroke.is_none()) {
        return;
    }
    if vectors {
        let style = VectorStyle {
            fill: p.fill,
            stroke: p.stroke.map(|s| s.0),
            stroke_mm: p.stroke.map_or(emb_model::vector::DEFAULT_STROKE_MM, |s| s.1),
        };
        out.objects.push(ImportedObject {
            kind: ObjectKind::Vector {
                path: Path::joined(&p.subs),
                style,
            },
            color: p.fill.or(style.stroke).unwrap_or(FALLBACK),
            name: p.name,
        });
        return;
    }
    if let Some(color) = p.fill {
        for (outer, holes) in region::nest(&p.subs) {
            out.objects.push(ImportedObject {
                kind: ObjectKind::Fill {
                    outer: outer.closed(),
                    holes: holes.into_iter().map(Path::closed).collect(),
                    params: FillParams::default(),
                },
                color,
                name: p.name.clone(),
            });
        }
    }
    if let Some((color, _)) = p.stroke {
        for s in &p.subs {
            out.objects.push(ImportedObject {
                kind: ObjectKind::Running {
                    path: s.clone(),
                    params: RunningParams::default(),
                },
                color,
                name: p.name.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(body: &str) -> Vec<u8> {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{body}</svg>"#
        )
        .into_bytes()
    }

    #[test]
    fn rect_fill_in_mm() {
        let r = import_svg(&svg(
            r##"<rect id="box" x="10" y="20" width="30" height="40" fill="#ff0000"/>"##,
        ))
        .unwrap();
        assert_eq!(r.objects.len(), 1);
        let o = &r.objects[0];
        assert_eq!(o.color, Rgb(255, 0, 0));
        assert_eq!(o.name.as_deref(), Some("box"));
        let b = r.bbox().unwrap();
        // The viewBox maps 1 unit to 1 mm.
        assert!((b.min.x - 10.0).abs() < 1e-3 && (b.min.y - 20.0).abs() < 1e-3);
        assert!((b.width() - 30.0).abs() < 1e-3 && (b.height() - 40.0).abs() < 1e-3);
        assert!(matches!(o.kind, ObjectKind::Fill { ref holes, .. } if holes.is_empty()));
    }

    #[test]
    fn ring_becomes_fill_with_hole_and_stroke_becomes_running() {
        let body = r##"<path fill="#00ff00" stroke="#000000" fill-rule="evenodd"
            d="M0,0 H50 V50 H0 Z M10,10 H40 V40 H10 Z"/>"##;
        let r = import_svg(&svg(body)).unwrap();
        let fills: Vec<_> = r
            .objects
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Fill { .. }))
            .collect();
        let runs: Vec<_> = r
            .objects
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Running { .. }))
            .collect();
        assert_eq!(fills.len(), 1);
        let ObjectKind::Fill { holes, .. } = &fills[0].kind else {
            unreachable!()
        };
        assert_eq!(holes.len(), 1);
        assert_eq!(runs.len(), 2, "one running object per subpath");
        assert_eq!(runs[0].color, Rgb(0, 0, 0));
        // The fill comes first in sew order.
        assert!(matches!(r.objects[0].kind, ObjectKind::Fill { .. }));
    }

    #[test]
    fn island_inside_hole_is_its_own_fill() {
        let body = r##"<path fill="blue" fill-rule="evenodd"
            d="M0,0 H60 V60 H0 Z M10,10 H50 V50 H10 Z M20,20 H40 V40 H20 Z"/>"##;
        let r = import_svg(&svg(body)).unwrap();
        assert_eq!(r.objects.len(), 2);
        let holes: Vec<usize> = r
            .objects
            .iter()
            .map(|o| match &o.kind {
                ObjectKind::Fill { holes, .. } => holes.len(),
                _ => 99,
            })
            .collect();
        assert_eq!(holes, vec![1, 0]);
    }

    #[test]
    fn transforms_and_curves_and_warnings() {
        let body = r##"<g transform="translate(50,0)"><circle cx="10" cy="10" r="10" fill="red"/></g>
            <text x="0" y="0">Hi</text>
            <rect width="5" height="5" fill="url(#g)"/>
            <defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs>"##;
        let r = import_svg(&svg(body)).unwrap();
        let circle = r.objects.iter().find(|o| o.color == Rgb(255, 0, 0)).unwrap();
        let b = circle.kind.paths().next().unwrap().control_bbox().unwrap();
        assert!((b.center().x - 60.0).abs() < 1e-3 && (b.width() - 20.0).abs() < 1e-3);
        assert!(r.warnings.iter().any(|w| w.contains("text")));
        assert!(r.warnings.iter().any(|w| w.contains("gradient")));
        assert!(import_svg(b"not svg").is_err());
    }

    #[test]
    fn paths_as_vectors_keep_fill_stroke_and_holes() {
        let body = r##"<g transform="scale(2)"><path id="ring" fill="#00ff00" stroke="#000000" stroke-width="0.5"
            fill-rule="evenodd" d="M0,0 H20 V20 H0 Z M5,5 H15 V15 H5 Z"/></g>
            <path stroke="#0000ff" fill="none" d="M0,50 L30,50"/>"##;
        let r = import_svg_vectors(&svg(body)).unwrap();
        assert_eq!(r.objects.len(), 2, "one vector for each path");
        let ObjectKind::Vector { path, style } = &r.objects[0].kind else {
            panic!("not a vector")
        };
        assert_eq!(path.subpaths().len(), 2);
        assert_eq!(style.fill, Some(Rgb(0, 255, 0)));
        assert_eq!(style.stroke, Some(Rgb(0, 0, 0)));
        assert!((style.stroke_mm - 1.0).abs() < 1e-6, "{}", style.stroke_mm);
        assert_eq!(r.objects[0].color, Rgb(0, 255, 0));
        assert_eq!(r.objects[0].name.as_deref(), Some("ring"));
        let ObjectKind::Vector { style, .. } = &r.objects[1].kind else {
            panic!("not a vector")
        };
        assert_eq!((style.fill, style.stroke), (None, Some(Rgb(0, 0, 255))));
        assert!((r.bbox().unwrap().width() - 40.0).abs() < 1e-3);
    }

    #[test]
    fn corpus_two_color_patch() {
        let data = include_bytes!("../../../tests/corpus/svg/two-color-patch.svg");
        let r = import_svg(data).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        let kinds: Vec<&str> = r.objects.iter().map(|o| o.kind.type_name()).collect();
        assert_eq!(kinds, vec!["Fill", "Fill", "Running"]);
        // The ring is one fill with one hole.
        let ObjectKind::Fill { holes, .. } = &r.objects[1].kind else {
            unreachable!()
        };
        assert_eq!(holes.len(), 1);
        let b = r.bbox().unwrap();
        assert!((b.width() - 72.0).abs() < 0.5, "{}", b.width());
        let colors: std::collections::BTreeSet<_> = r.objects.iter().map(|o| o.color.to_hex()).collect();
        assert_eq!(colors.len(), 2);
    }
}
