//! Writes an SVG of the satin columns of a bundled font (a check by eye).
//! Usage: cargo run -p emb-letter --example strokes_svg -- <family> <style> <out.svg> [text]
use emb_geom::Point;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = emb_letter::bundled::font(&a[1], &a[2]).expect("font");
    let text = a
        .get(4)
        .cloned()
        .unwrap_or_else(|| "ABCDEFGHIJKLM\nNOPQRSTUVWXYZ\nabcdefghijklm\nnopqrstuvwxyz\n0123456789&?!".into());
    let mut svg = String::new();
    let (mut x, mut y) = (0.0, f.cap_height * 1.2);
    let mut maxx: f64 = 0.0;
    let mut report = String::new();
    for c in text.chars() {
        if c == '\n' {
            x = 0.0;
            y += f.line_height();
            continue;
        }
        let Some(g) = f.glyph(c) else {
            x += 300.0;
            continue;
        };
        let s = f.shapes(g);
        let pts = |p: &emb_geom::Path| -> String {
            p.flatten(1.0)
                .iter()
                .map(|pl| {
                    let mut d = String::new();
                    for (i, q) in pl.points.iter().enumerate() {
                        d += &format!("{}{:.1},{:.1} ", if i == 0 { "M" } else { "L" }, q.x + x, q.y + y);
                    }
                    if pl.closed {
                        d += "Z";
                    }
                    d
                })
                .collect()
        };
        for (o, holes) in &s.fills {
            let mut d = String::new();
            for r in std::iter::once(o).chain(holes.iter()) {
                for (i, q) in r.iter().enumerate() {
                    d += &format!("{}{:.1},{:.1} ", if i == 0 { "M" } else { "L" }, q.x + x, q.y + y);
                }
                d += "Z ";
            }
            svg += &format!("<path d='{d}' fill='#ddd' fill-rule='evenodd'/>\n");
        }
        for (k, sat) in s.strokes.satins.iter().enumerate() {
            let col = ["#c03", "#06c", "#090", "#c60", "#909", "#088"][k % 6];
            svg += &format!(
                "<path d='{}' fill='none' stroke='{col}' stroke-width='4'/>\n",
                pts(&sat.rail_a)
            );
            svg += &format!(
                "<path d='{}' fill='none' stroke='{col}' stroke-width='4'/>\n",
                pts(&sat.rail_b)
            );
            for (p, q) in &sat.rungs {
                let (p, q) = (*p + Point::new(x, y), *q + Point::new(x, y));
                svg += &format!(
                    "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='{col}' stroke-width='1.5' opacity='0.6'/>\n",
                    p.x, p.y, q.x, q.y
                );
            }
        }
        report += &format!(
            "{c}: {} satins, coverage {:.2}, width {:.0}\n",
            s.strokes.satins.len(),
            s.strokes.coverage,
            s.strokes.max_width
        );
        x += f.advance(g) + 60.0;
        maxx = maxx.max(x);
    }
    let h = y + f.line_height() * 0.5;
    std::fs::write(&a[3], format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {maxx:.0} {h:.0}' width='{:.0}' height='{:.0}'><rect width='100%' height='100%' fill='white'/>\n{svg}</svg>", maxx / 3.0, h / 3.0)).unwrap();
    print!("{report}");
}
