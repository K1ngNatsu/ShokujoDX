//! A small PDF writer for pages (`page::Page`): text in the standard
//! Helvetica fonts (no embedded fonts), lines, rectangles and stitches as
//! vector lines. Our own code, written from the PDF 1.4 reference.
//!
//! The text encoding is WinAnsi: Latin-1 letters print, other characters
//! print as `?`. The content streams are not compressed.

use std::fmt::Write as _;

use emb_model::Rgb;

use crate::page::{Op, Page};

const PT_PER_MM: f64 = 72.0 / 25.4;

/// A number with at most two decimals, no trailing zeros.
fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

fn rgb(c: Rgb) -> String {
    format!(
        "{} {} {}",
        num(f64::from(c.0) / 255.0),
        num(f64::from(c.1) / 255.0),
        num(f64::from(c.2) / 255.0)
    )
}

/// A PDF string: WinAnsi bytes, with `(`, `)` and `\` escaped and the bytes
/// above 126 as octal escapes.
fn pdf_string(text: &str) -> String {
    let mut out = String::from("(");
    for ch in text.chars() {
        let b = match ch as u32 {
            0x20..=0x7e => ch as u8,
            0xa0..=0xff => ch as u32 as u8,
            0x2022 => 0x95, // bullet
            0x2013 => 0x96, // en dash
            0x2014 => 0x97, // em dash
            0x2026 => 0x85, // ellipsis
            _ => b'?',
        };
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push(')');
    out
}

/// The content stream of one page.
fn content(page: &Page) -> String {
    let h = page.height_mm;
    let x = |v: f64| num(v * PT_PER_MM);
    let y = |v: f64| num((h - v) * PT_PER_MM);
    let mut s = String::new();
    for op in &page.ops {
        match op {
            Op::Text {
                at,
                size_pt,
                bold,
                color,
                text,
            } => {
                let font = if *bold { "F2" } else { "F1" };
                let _ = writeln!(
                    s,
                    "BT /{font} {} Tf {} rg {} {} Td {} Tj ET",
                    num(*size_pt),
                    rgb(*color),
                    x(at.x),
                    y(at.y),
                    pdf_string(text)
                );
            }
            Op::Line { a, b, width_mm, color } => {
                let _ = writeln!(
                    s,
                    "{} w {} RG {} {} m {} {} l S",
                    num(width_mm * PT_PER_MM),
                    rgb(*color),
                    x(a.x),
                    y(a.y),
                    x(b.x),
                    y(b.y)
                );
            }
            Op::Rect { rect, fill, stroke } => {
                let r = format!(
                    "{} {} {} {} re",
                    x(rect.min.x),
                    y(rect.max.y),
                    num(rect.width() * PT_PER_MM),
                    num(rect.height() * PT_PER_MM)
                );
                match (fill, stroke) {
                    (Some(f), Some((w, c))) => {
                        let _ = writeln!(s, "{} w {} rg {} RG {r} B", num(w * PT_PER_MM), rgb(*f), rgb(*c));
                    }
                    (Some(f), None) => {
                        let _ = writeln!(s, "{} rg {r} f", rgb(*f));
                    }
                    (None, Some((w, c))) => {
                        let _ = writeln!(s, "{} w {} RG {r} S", num(w * PT_PER_MM), rgb(*c));
                    }
                    (None, None) => {}
                }
            }
            Op::Area { rings, fill } => {
                let _ = writeln!(s, "{} rg", rgb(*fill));
                for ring in rings.iter().filter(|r| r.len() >= 3) {
                    let _ = write!(s, "{} {} m", x(ring[0].x), y(ring[0].y));
                    for p in &ring[1..] {
                        let _ = write!(s, " {} {} l", x(p.x), y(p.y));
                    }
                    s.push_str(" h\n");
                }
                s.push_str("f*\n");
            }
            Op::Stitches { segments, width_mm } => {
                let _ = writeln!(s, "q 1 J 1 j {} w", num(width_mm * PT_PER_MM));
                let mut current: Option<Rgb> = None;
                let mut in_path = 0usize;
                for seg in segments {
                    if current != Some(seg.color) || in_path >= 2000 {
                        if in_path > 0 {
                            s.push_str("S\n");
                        }
                        let _ = writeln!(s, "{} RG", rgb(seg.color));
                        current = Some(seg.color);
                        in_path = 0;
                    }
                    let _ = writeln!(s, "{} {} m {} {} l", x(seg.a.x), y(seg.a.y), x(seg.b.x), y(seg.b.y));
                    in_path += 1;
                }
                if in_path > 0 {
                    s.push_str("S\n");
                }
                s.push_str("Q\n");
            }
        }
    }
    s
}

/// Writes the pages as one PDF file.
pub fn write(pages: &[Page], title: &str) -> Vec<u8> {
    // Objects: 1 catalog, 2 page tree, 3 and 4 fonts, 5 info, then a page
    // and its content for each page.
    let page_id = |i: usize| 6 + 2 * i;
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: Vec<String> = (0..pages.len()).map(|i| format!("{} 0 R", page_id(i))).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()).into_bytes());
    for font in ["Helvetica", "Helvetica-Bold"] {
        objects.push(
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>").into_bytes(),
        );
    }
    objects.push(format!("<< /Title {} /Producer (ShokujoDX) >>", pdf_string(title)).into_bytes());
    for (i, p) in pages.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                num(p.width_mm * PT_PER_MM),
                num(p.height_mm * PT_PER_MM),
                page_id(i) + 1
            )
            .into_bytes(),
        );
        let c = content(p);
        let mut stream = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        stream.extend(c.as_bytes());
        stream.extend(b"\nendstream");
        objects.push(stream);
    }

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend(body);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Point};

    use super::*;
    use crate::page::Paper;
    use crate::{Segment, SegmentKind};

    fn sample() -> Page {
        let mut p = Page::new(Paper::A4);
        p.text(15.0, 20.0, 12.0, true, "Patch (red) é ✓");
        p.line(Point::new(15.0, 25.0), Point::new(195.0, 25.0), 0.3, Rgb(0, 0, 0));
        p.ops.push(Op::Rect {
            rect: BBox::new(Point::new(15.0, 30.0), Point::new(25.0, 35.0)),
            fill: Some(Rgb(255, 0, 0)),
            stroke: Some((0.2, Rgb(0, 0, 0))),
        });
        p.ops.push(Op::Stitches {
            segments: vec![
                Segment {
                    a: Point::new(50.0, 50.0),
                    b: Point::new(60.0, 50.0),
                    color: Rgb(255, 0, 0),
                    kind: SegmentKind::Stitch,
                };
                3
            ],
            width_mm: 0.35,
        });
        p
    }

    #[test]
    fn structure_and_offsets() {
        let pdf = write(&[sample(), sample()], "Test");
        // One character per byte, so text positions are byte offsets.
        let text: String = pdf.iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.ends_with("%%EOF\n"));
        assert!(text.contains("/Count 2"));
        // Each xref entry points at "n 0 obj".
        let start: usize = text
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(text[start..].starts_with("xref"));
        let entries: Vec<usize> = text[start..]
            .lines()
            .skip(3)
            .take_while(|l| !l.starts_with("trailer"))
            .map(|l| l[..10].parse().unwrap())
            .collect();
        assert_eq!(entries.len(), 9);
        for (i, off) in entries.iter().enumerate() {
            assert!(
                text[*off..].starts_with(&format!("{} 0 obj", i + 1)),
                "object {}",
                i + 1
            );
        }
        // The stream lengths are right.
        for (i, _) in text.match_indices("/Length ") {
            let rest = &text[i + 8..];
            let len: usize = rest[..rest.find(' ').unwrap()].parse().unwrap();
            let body = rest.find("stream\n").unwrap() + 7;
            assert_eq!(&rest[body + len..body + len + 10], "\nendstream");
        }
    }

    #[test]
    fn text_and_drawing() {
        let c = content(&sample());
        assert!(c.contains("/F2 12 Tf"), "{c}");
        // y = (297 - 20) mm in points.
        assert!(c.contains(&format!("{} {} Td", num(15.0 * PT_PER_MM), num(277.0 * PT_PER_MM))));
        assert!(c.contains("(Patch \\(red\\) \\351 ?) Tj"), "{c}");
        assert!(c.contains("1 0 0 rg"));
        assert_eq!(c.matches(" m ").count(), 4, "one line and three stitches");
        assert_eq!(c.matches("RG\n").count(), 1, "one color for the stitches");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-0.001), "0");
        assert_eq!(num(2.346), "2.35");
    }
}
