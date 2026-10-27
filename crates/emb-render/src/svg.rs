//! The design picture as SVG: each run of stitches in one color is one line
//! in the thread color, in sew order, over the appliqué fabric areas.
//! Units are mm, so the picture prints and imports at its real size.
//!
//! [`outlines`] writes the shapes of the objects as SVG paths (owner
//! request, P6 Milestone 1), for vector programs and cutters.

use std::fmt::Write;

use emb_geom::{BBox, Path, PathCmd, Point};
use emb_model::{Design, ObjectKind, Rgb};

use crate::raster::Areas;
use crate::{Segment, SegmentKind};

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// A number with at most two decimals and no "-0".
fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    let r = if r == 0.0 { 0.0 } else { r };
    format!("{r}")
}

/// The SVG text of the stitches. `thread_mm` is the line width;
/// `margin_mm` is the space round the stitches.
pub fn picture(segments: &[Segment], areas: &Areas, thread_mm: f64, margin_mm: f64, background: Option<Rgb>) -> String {
    let stitches: Vec<&Segment> = segments.iter().filter(|s| s.kind == SegmentKind::Stitch).collect();
    // The stitches and the areas (appliqué fabric, rhinestones).
    let b = BBox::from_points(
        stitches
            .iter()
            .flat_map(|s| [s.a, s.b])
            .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied())),
    )
    .unwrap_or(BBox::new(Point::ZERO, Point::ZERO))
    .expand(margin_mm + thread_mm);
    let p = |q: Point| format!("{},{}", num(q.x), num(q.y));
    let mut s = String::new();
    let _ = writeln!(s, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="{x} {y} {w} {h}">"#,
        x = num(b.min.x),
        y = num(b.min.y),
        w = num(b.width()),
        h = num(b.height()),
    );
    if let Some(c) = background {
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
            num(b.min.x),
            num(b.min.y),
            num(b.width()),
            num(b.height()),
            hex(c)
        );
    }
    for (rings, color) in areas {
        let mut d = String::new();
        for r in rings.iter().filter(|r| r.len() > 2) {
            let _ = write!(d, "M{}", p(r[0]));
            for q in &r[1..] {
                let _ = write!(d, " L{}", p(*q));
            }
            d.push_str(" Z ");
        }
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}" fill-rule="evenodd"/>"#,
            d.trim_end(),
            hex(*color)
        );
    }
    let _ = writeln!(
        s,
        r#"<g fill="none" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">"#,
        num(thread_mm)
    );
    // Connected stitches of one color make one line.
    let mut i = 0;
    while i < stitches.len() {
        let color = stitches[i].color;
        let mut pts = vec![stitches[i].a, stitches[i].b];
        let mut j = i + 1;
        while j < stitches.len() && stitches[j].color == color && stitches[j].a == stitches[j - 1].b {
            pts.push(stitches[j].b);
            j += 1;
        }
        let list: Vec<String> = pts.iter().map(|q| p(*q)).collect();
        let _ = writeln!(s, r#"<polyline stroke="{}" points="{}"/>"#, hex(color), list.join(" "));
        i = j;
    }
    s.push_str("</g>\n</svg>\n");
    s
}

/// The SVG path data of `path` (lines and curves as they are).
fn path_data(path: &Path) -> String {
    let p = |q: Point| format!("{},{}", num(q.x), num(q.y));
    let parts: Vec<String> = path
        .cmds
        .iter()
        .map(|c| match *c {
            PathCmd::MoveTo(a) => format!("M{}", p(a)),
            PathCmd::LineTo(a) => format!("L{}", p(a)),
            PathCmd::CubicTo(a, b, c) => format!("C{} {} {}", p(a), p(b), p(c)),
            PathCmd::Close => "Z".into(),
        })
        .collect();
    parts.join(" ")
}

/// Escapes text for an XML attribute.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The shapes of the objects as SVG paths, in sew order, in mm with the
/// hoop center at (0, 0) (as the machine files). Vectors keep their fill
/// and stroke; embroidery objects are their outlines in the thread color:
/// areas filled (even-odd), lines stroked. Stitch objects from machine
/// files are left out. `None` if there is nothing to write.
pub fn outlines(design: &Design) -> Option<String> {
    let shapes = outline_shapes(design);
    outlines_svg(&shapes)
}

/// The shapes of [`outlines`]: each path (mm, hoop center at (0, 0)), its
/// style and its name. Also for other vector formats (EMF).
pub fn outline_shapes(design: &Design) -> Vec<(Path, emb_model::VectorStyle, String)> {
    let t = design.output_transform();
    let mut shapes: Vec<(Path, emb_model::VectorStyle, String)> = Vec::new();
    for (i, o) in design.objects().iter().enumerate() {
        let color = design.thread(o.thread).map_or(Rgb(0, 0, 0), |th| th.color);
        let vector = match &o.kind {
            ObjectKind::Vector { .. } => Some(o.kind.clone()),
            k => emb_model::vector::to_vector(k, color),
        };
        if let Some(ObjectKind::Vector { path, style }) = vector {
            let id = o.name.clone().unwrap_or_else(|| format!("object-{}", i + 1));
            shapes.push((path.transformed(&t), style, id));
        }
    }
    shapes
}

fn outlines_svg(shapes: &[(Path, emb_model::VectorStyle, String)]) -> Option<String> {
    let b = shapes
        .iter()
        .filter_map(|(path, style, _)| path.control_bbox().map(|b| b.expand(style.stroke_mm)))
        .reduce(BBox::union)?
        .expand(1.0);
    let mut s = String::new();
    let _ = writeln!(s, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="{x} {y} {w} {h}">"#,
        x = num(b.min.x),
        y = num(b.min.y),
        w = num(b.width()),
        h = num(b.height()),
    );
    for (path, style, id) in shapes {
        let fill = style.fill.map_or("none".to_string(), hex);
        let stroke = match style.stroke {
            Some(c) => format!(r#" stroke="{}" stroke-width="{}""#, hex(c), num(style.stroke_mm)),
            None => String::new(),
        };
        let _ = writeln!(
            s,
            r#"<path id="{}" d="{}" fill="{fill}" fill-rule="evenodd"{stroke}/>"#,
            attr(id),
            path_data(path)
        );
    }
    s.push_str("</svg>\n");
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: (f64, f64), b: (f64, f64), color: Rgb, kind: SegmentKind) -> Segment {
        Segment {
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            color,
            kind,
        }
    }

    #[test]
    fn outlines_of_objects_and_vectors_as_paths() {
        use emb_model::{FillParams, RunningParams, VectorStyle};
        let mut d = Design::new("o");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let fill = d.add_object(
            red,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0))),
                holes: Vec::new(),
                params: FillParams::default(),
            },
        );
        d.object_mut(fill).unwrap().name = Some("a<b".into());
        d.add_object(
            red,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(0.0, 20.0))
                    .line_to(Point::new(10.0, 20.0)),
                params: RunningParams::default(),
            },
        );
        d.add_object(
            red,
            ObjectKind::Vector {
                path: Path::new().move_to(Point::new(0.0, 30.0)).cubic_to(
                    Point::new(1.0, 31.0),
                    Point::new(2.0, 31.0),
                    Point::new(3.0, 30.0),
                ),
                style: VectorStyle {
                    fill: None,
                    stroke: Some(Rgb(0, 0, 255)),
                    stroke_mm: 0.5,
                },
            },
        );
        let svg = outlines(&d).unwrap();
        assert!(
            svg.contains(r##"<path id="a&lt;b" d="M0,0 L10,0 L10,10 L0,10 Z" fill="#c80000" fill-rule="evenodd"/>"##),
            "{svg}"
        );
        assert!(
            svg.contains(r##"d="M0,20 L10,20" fill="none" fill-rule="evenodd" stroke="#c80000""##),
            "{svg}"
        );
        assert!(
            svg.contains(
                r##"d="M0,30 C1,31 2,31 3,30" fill="none" fill-rule="evenodd" stroke="#0000ff" stroke-width="0.5""##
            ),
            "{svg}"
        );
        assert!(outlines(&Design::new("empty")).is_none());
    }

    #[test]
    fn runs_of_stitches_are_lines_in_thread_colors() {
        let red = Rgb(200, 0, 0);
        let blue = Rgb(0, 0, 255);
        let segs = [
            seg((0.0, 0.0), (1.0, 0.0), red, SegmentKind::Stitch),
            seg((1.0, 0.0), (2.0, 0.5), red, SegmentKind::Stitch),
            seg((2.0, 0.5), (8.0, 8.0), red, SegmentKind::Jump),
            seg((8.0, 8.0), (9.0, 8.0), blue, SegmentKind::Stitch),
        ];
        let fabric = vec![(
            vec![vec![Point::new(0.0, 0.0), Point::new(5.0, 0.0), Point::new(5.0, 5.0)]],
            Rgb(1, 2, 3),
        )];
        let svg = picture(&segs, &fabric, 0.4, 1.0, None);
        assert!(
            svg.contains(r##"<polyline stroke="#c80000" points="0,0 1,0 2,0.5"/>"##),
            "{svg}"
        );
        assert!(svg.contains(r##"<polyline stroke="#0000ff" points="8,8 9,8"/>"##));
        assert!(svg.contains(r##"<path d="M0,0 L5,0 L5,5 Z" fill="#010203""##));
        // 1.4 mm round the stitches (x 0 to 9, y 0 to 8).
        assert!(
            svg.contains(r#"width="11.8mm" height="10.8mm" viewBox="-1.4 -1.4 11.8 10.8""#),
            "{svg}"
        );
        assert!(!svg.contains("NaN"));
        assert_eq!(svg.matches("<polyline").count(), 2);
    }
}
