//! Rhinestone template files (P9 Milestone 4): a hole for each stone and
//! the registration marks, at real size, for a vinyl or craft cutter.
//!
//! SVG: millimetres (`width`, `height` and `viewBox` in mm), Y down. One
//! group of circles for each kind of stone (its title says the stone and
//! the number of holes), so a cutter program can cut one size at a time;
//! the marks are a group of crosses of their own. DXF: the R12 form,
//! millimetres (`$INSUNITS` 4), Y up, `CIRCLE` entities on a layer for each
//! kind of stone and `LINE` entities on the layer `MARKS`. These are the
//! public file descriptions of SVG 1.1 and DXF R12; no code is ported.

use emb_geom::Point;
use emb_model::bling::{MARK_ARM_MM, StoneTemplate};

/// The space round the template in the SVG picture, in mm.
const MARGIN_MM: f64 = 3.0;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A name with letters, digits and `-` only, for an SVG id or a DXF layer.
fn ident(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    t.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// The template as an SVG file.
pub fn svg(t: &StoneTemplate) -> String {
    let b = t
        .bbox()
        .unwrap_or(emb_geom::BBox::new(Point::ZERO, Point::ZERO))
        .expand(MARGIN_MM);
    let (w, h) = (b.width(), b.height());
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.3}mm\" height=\"{h:.3}mm\" \
         viewBox=\"{:.3} {:.3} {w:.3} {h:.3}\">\n",
        b.min.x, b.min.y
    );
    for (stone, n) in t.counts() {
        let holes: Vec<_> = t.holes.iter().filter(|h| h.stone == stone).collect();
        let d = holes.first().map_or(0.0, |h| h.diameter_mm);
        s.push_str(&format!(
            "  <g id=\"{}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.1\">\n    <title>{}: {n} hole{} of {d:.2} mm</title>\n",
            ident(&stone.label()),
            esc(&stone.label()),
            if n == 1 { "" } else { "s" }
        ));
        for h in holes {
            s.push_str(&format!(
                "    <circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\"/>\n",
                h.at.x,
                h.at.y,
                h.diameter_mm / 2.0
            ));
        }
        s.push_str("  </g>\n");
    }
    s.push_str(
        "  <g id=\"marks\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.1\">\n    <title>Registration marks</title>\n",
    );
    for m in &t.marks {
        let a = MARK_ARM_MM;
        s.push_str(&format!(
            "    <path d=\"M{:.3} {:.3} H{:.3} M{:.3} {:.3} V{:.3}\"/>\n",
            m.x - a,
            m.y,
            m.x + a,
            m.x,
            m.y - a,
            m.y + a
        ));
    }
    s.push_str("  </g>\n</svg>\n");
    s
}

/// The template as a DXF (R12) file.
pub fn dxf(t: &StoneTemplate) -> String {
    let mut s = String::new();
    let mut g = |code: i32, value: &str| {
        s.push_str(&format!("{code:>3}\r\n{value}\r\n"));
    };
    let f = |v: f64| format!("{v:.3}");
    let b = t.bbox().unwrap_or(emb_geom::BBox::new(Point::ZERO, Point::ZERO));
    g(0, "SECTION");
    g(2, "HEADER");
    g(9, "$ACADVER");
    g(1, "AC1009");
    g(9, "$INSUNITS");
    g(70, "4");
    g(9, "$EXTMIN");
    g(10, &f(b.min.x));
    g(20, &f(-b.max.y));
    g(9, "$EXTMAX");
    g(10, &f(b.max.x));
    g(20, &f(-b.min.y));
    g(0, "ENDSEC");
    g(0, "SECTION");
    g(2, "ENTITIES");
    for h in &t.holes {
        let layer = ident(&h.stone.label()).to_ascii_uppercase();
        g(0, "CIRCLE");
        g(8, &layer);
        g(10, &f(h.at.x));
        g(20, &f(-h.at.y));
        g(40, &f(h.diameter_mm / 2.0));
    }
    let a = MARK_ARM_MM;
    for m in &t.marks {
        for (p, q) in [
            (Point::new(m.x - a, m.y), Point::new(m.x + a, m.y)),
            (Point::new(m.x, m.y - a), Point::new(m.x, m.y + a)),
        ] {
            g(0, "LINE");
            g(8, "MARKS");
            g(10, &f(p.x));
            g(20, &f(-p.y));
            g(11, &f(q.x));
            g(21, &f(-q.y));
        }
    }
    g(0, "ENDSEC");
    g(0, "EOF");
    s
}

#[cfg(test)]
mod tests {
    use emb_model::bling::{Bling, BlingLayout, Stone};
    use emb_model::{Design, ObjectKind, Rgb};

    use super::*;

    fn template() -> StoneTemplate {
        let mut d = Design::new("t");
        let th = d.add_thread("Stones", Rgb(0, 0, 0));
        let gold = Stone {
            ss: 16,
            name: "Gold & co".into(),
            ..Stone::default()
        };
        let at = |x: f64| Point::new(x, 0.0);
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Stones {
                    points: vec![at(0.0), at(10.0)],
                },
            ))),
        );
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(
                gold,
                BlingLayout::Stones { points: vec![at(20.0)] },
            ))),
        );
        d.stone_template(0.3).unwrap()
    }

    #[test]
    fn svg_has_a_group_for_each_stone_at_real_size() {
        let t = template();
        let s = svg(&t);
        assert_eq!(s.matches("<circle").count(), 3);
        assert!(s.contains("<g id=\"ss10-crystal\""), "{s}");
        assert!(
            s.contains("<title>SS16 Gold &amp; co: 1 hole of 4.20 mm</title>"),
            "{s}"
        );
        assert!(s.contains("r=\"1.550\""));
        assert_eq!(s.matches("<path").count(), 4, "four marks");
        // usvg reads it, at the same size: 1 mm is 96/25.4 px.
        let tree = usvg::Tree::from_str(&s, &usvg::Options::default()).unwrap();
        let b = t.bbox().unwrap().expand(MARGIN_MM);
        let px = 96.0 / 25.4;
        assert!((f64::from(tree.size().width()) - b.width() * px).abs() < 0.01);
    }

    #[test]
    fn dxf_has_circles_and_marks() {
        let t = template();
        let s = dxf(&t);
        assert_eq!(s.matches("CIRCLE\r\n").count(), 3);
        assert_eq!(s.matches("LINE\r\n").count(), 8);
        assert!(s.contains("  8\r\nSS16-GOLD-CO\r\n"), "{s}");
        assert!(s.contains(" 40\r\n2.100\r\n"));
        assert!(s.ends_with("  0\r\nEOF\r\n"));
    }
}
