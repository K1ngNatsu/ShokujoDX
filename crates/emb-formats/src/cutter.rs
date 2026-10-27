//! Appliqué cutter files (P8 Milestone 7): the cut line of each appliqué
//! piece as SVG or DXF, at real size, for a fabric cutting machine (a
//! cutting plotter, a die-cut or laser program).
//!
//! The cut line is the outline of the piece (the placement line) and its
//! holes; the caller can make it larger or smaller first. One SVG path or
//! one group of DXF polylines for each piece, named after the piece. The
//! pieces keep their places in the design, so the cutter program shows
//! them as they are on the design; it can move them on the fabric.
//!
//! SVG: millimetres (`width`, `height` and `viewBox` in mm), Y down, black
//! lines of 0.1 mm, no fill. DXF: the R12 form (most cutter programs read
//! it), millimetres (`$INSUNITS` 4), Y up, closed `POLYLINE` entities on a
//! layer for each piece. These are the public file descriptions of SVG 1.1
//! and DXF R12; no code is ported.

use emb_geom::Point;

/// One appliqué piece: a name and its closed rings in mm (Y down).
#[derive(Debug, Clone, PartialEq)]
pub struct CutPiece {
    pub name: String,
    pub rings: Vec<Vec<Point>>,
}

/// The appliqué pieces of `design` (of the objects `ids`, or of all
/// objects), in machine coordinates (the design rotation and the hoop
/// angle applied, as the stitch plan), so that the pieces have the
/// orientation that they have in the hoop.
pub fn pieces_of(design: &emb_model::Design, ids: Option<&[emb_model::ObjectId]>) -> Vec<CutPiece> {
    let to_out = design.output_transform();
    let mut out = Vec::new();
    for (n, o) in design.objects().iter().enumerate() {
        if ids.is_some_and(|ids| !ids.contains(&o.id)) {
            continue;
        }
        let emb_model::ObjectKind::Applique { pieces, .. } = &o.kind else {
            continue;
        };
        let base = o.name.clone().unwrap_or_else(|| format!("Applique {}", n + 1));
        for (k, pc) in pieces.iter().enumerate() {
            let rings: Vec<Vec<Point>> = std::iter::once(&pc.outer)
                .chain(pc.holes.iter())
                .flat_map(|path| path.flatten(0.05))
                .map(|pl| pl.points.iter().map(|q| to_out.apply(*q)).collect())
                .collect();
            let name = if pieces.len() > 1 {
                format!("{base} piece {}", k + 1)
            } else {
                base.clone()
            };
            out.push(CutPiece { name, rings });
        }
    }
    out
}

/// The space round the pieces in the SVG picture, in mm.
const MARGIN_MM: f64 = 5.0;

fn bounds(pieces: &[CutPiece]) -> Option<(Point, Point)> {
    let mut pts = pieces.iter().flat_map(|p| p.rings.iter().flatten());
    let first = *pts.next()?;
    let (mut lo, mut hi) = (first, first);
    for q in pts {
        lo = Point::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    Some((lo, hi))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The pieces as an SVG file.
pub fn svg(pieces: &[CutPiece]) -> String {
    let (lo, hi) = bounds(pieces).unwrap_or((Point::ZERO, Point::ZERO));
    let (x0, y0) = (lo.x - MARGIN_MM, lo.y - MARGIN_MM);
    let (w, h) = (hi.x - lo.x + 2.0 * MARGIN_MM, hi.y - lo.y + 2.0 * MARGIN_MM);
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.3}mm\" height=\"{h:.3}mm\" \
         viewBox=\"{x0:.3} {y0:.3} {w:.3} {h:.3}\">\n"
    );
    for (i, p) in pieces.iter().enumerate() {
        let mut d = String::new();
        for ring in p.rings.iter().filter(|r| r.len() >= 3) {
            for (k, q) in ring.iter().enumerate() {
                d.push_str(&format!("{}{:.3} {:.3} ", if k == 0 { "M" } else { "L" }, q.x, q.y));
            }
            d.push_str("Z ");
        }
        s.push_str(&format!(
            "  <path id=\"piece-{}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.1\" fill-rule=\"evenodd\" d=\"{}\">\
             <title>{}</title></path>\n",
            i + 1,
            d.trim_end(),
            esc(&p.name)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// A DXF layer name: letters, digits, `-` and `_` only.
fn layer(name: &str, i: usize) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .take(28)
        .collect();
    format!("{}_{}", if clean.is_empty() { "PIECE" } else { &clean }, i + 1)
}

/// The pieces as a DXF (R12) file.
pub fn dxf(pieces: &[CutPiece]) -> String {
    let mut s = String::new();
    let mut g = |code: i32, value: &str| {
        s.push_str(&format!("{code:>3}\r\n{value}\r\n"));
    };
    let (lo, hi) = bounds(pieces).unwrap_or((Point::ZERO, Point::ZERO));
    g(0, "SECTION");
    g(2, "HEADER");
    g(9, "$ACADVER");
    g(1, "AC1009");
    g(9, "$INSUNITS");
    g(70, "4");
    g(9, "$EXTMIN");
    g(10, &format!("{:.3}", lo.x));
    g(20, &format!("{:.3}", -hi.y));
    g(9, "$EXTMAX");
    g(10, &format!("{:.3}", hi.x));
    g(20, &format!("{:.3}", -lo.y));
    g(0, "ENDSEC");
    g(0, "SECTION");
    g(2, "ENTITIES");
    for (i, p) in pieces.iter().enumerate() {
        let layer = layer(&p.name, i);
        for ring in p.rings.iter().filter(|r| r.len() >= 3) {
            g(0, "POLYLINE");
            g(8, &layer);
            g(66, "1");
            g(70, "1");
            for q in ring {
                g(0, "VERTEX");
                g(8, &layer);
                g(10, &format!("{:.3}", q.x));
                g(20, &format!("{:.3}", -q.y));
            }
            g(0, "SEQEND");
            g(8, &layer);
        }
    }
    g(0, "ENDSEC");
    g(0, "EOF");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, s: f64) -> Vec<Point> {
        vec![
            Point::new(x, 0.0),
            Point::new(x + s, 0.0),
            Point::new(x + s, s),
            Point::new(x, s),
        ]
    }

    fn pieces() -> Vec<CutPiece> {
        vec![
            CutPiece {
                name: "Heart & star".into(),
                rings: vec![square(0.0, 40.0), square(10.0, 10.0)],
            },
            CutPiece {
                name: "Leaf".into(),
                rings: vec![square(60.0, 20.0)],
            },
        ]
    }

    #[test]
    fn svg_is_at_real_size() {
        let s = svg(&pieces());
        // 80 × 40 mm of pieces and 5 mm round them.
        assert!(s.contains("width=\"90.000mm\" height=\"50.000mm\""), "{s}");
        assert!(s.contains("viewBox=\"-5.000 -5.000 90.000 50.000\""));
        assert_eq!(s.matches("<path").count(), 2);
        assert_eq!(s.matches('Z').count(), 3, "a hole is a ring of the same path");
        assert!(s.contains("<title>Heart &amp; star</title>"));
        // usvg reads it, at the same size.
        let tree = usvg::Tree::from_str(&s, &usvg::Options::default()).unwrap();
        let mm_per_px = 25.4 / 96.0;
        assert!((f64::from(tree.size().width()) * mm_per_px - 90.0).abs() < 0.01);
    }

    #[test]
    fn pieces_of_an_applique_object_in_the_turned_hoop() {
        let mut d = emb_model::Design::new("t");
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        let outer = emb_geom::Path::rect(emb_geom::BBox::new(Point::new(10.0, 0.0), Point::new(30.0, 10.0)));
        d.add_object(
            t,
            emb_model::ObjectKind::Applique {
                pieces: vec![emb_model::AppliquePiece::new(outer, Vec::new())],
                params: Default::default(),
            },
        );
        d.add_object(
            t,
            emb_model::ObjectKind::Running {
                path: emb_geom::Path::new().move_to(Point::ZERO).line_to(Point::new(5.0, 0.0)),
                params: Default::default(),
            },
        );
        let p = pieces_of(&d, None);
        assert_eq!(p.len(), 1, "only the appliqué object");
        assert_eq!(p[0].name, "Applique 1");
        let xs = |p: &CutPiece| p.rings[0].iter().map(|q| q.x).fold(f64::MIN, f64::max);
        assert!((xs(&p[0]) - 30.0).abs() < 1e-9);
        // A hoop turned 90°: the piece turns as the stitches do.
        d.hoop.angle_deg = 90.0;
        let p = pieces_of(&d, None);
        assert!(xs(&p[0]) < 11.0, "{:?}", p[0].rings[0]);
    }

    #[test]
    fn dxf_has_a_closed_polyline_for_each_ring() {
        let s = dxf(&pieces());
        assert!(s.starts_with("  0\r\nSECTION\r\n"));
        assert!(s.ends_with("  0\r\nEOF\r\n"));
        assert_eq!(s.matches("POLYLINE").count(), 3);
        assert_eq!(s.matches("VERTEX").count(), 12);
        assert!(s.contains("Heart___star_1") && s.contains("Leaf_2"));
        // Y is up: the square from y 0 to 40 goes from 0 to -40.
        assert!(s.contains(" 20\r\n-40.000\r\n"));
    }
}
