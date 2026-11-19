//! A small PDF writer for pages (`page::Page`): text in the standard
//! Helvetica fonts (no embedded fonts), lines, rectangles and stitches as
//! vector lines, and pictures as JPEG images. Our own code, written from
//! the PDF 1.4 reference.
//!
//! The text encoding is WinAnsi: Latin-1 letters print. Chinese, Japanese
//! and Korean characters print in a standard Japanese font of the PDF
//! reader (not embedded; the reader uses a font of the computer). Other
//! characters print as `?`. The content streams are not compressed.

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
            0x20ac => 0x80, // euro
            0x201e => 0x84, // low double quote
            0x2018 => 0x91, // quotes
            0x2019 => 0x92,
            0x201c => 0x93,
            0x201d => 0x94,
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

/// The JPEG quality of the pictures.
const JPEG_QUALITY: u8 = 90;

/// A picture as JPEG bytes (transparent pixels on white), its width and
/// its height. `None` if the encoder fails.
/// The name of the Japanese font in the pages.
const WIDE_FONT: &str = "F3";

/// A PDF string of wide characters for the Japanese font: each character
/// as four hexadecimal digits (UCS-2). A character outside that range
/// prints as the mark 〓.
fn wide_string(text: &str) -> String {
    let mut out = String::from("<");
    for ch in text.chars() {
        let code = u16::try_from(ch as u32).unwrap_or(0x3013);
        let _ = write!(out, "{code:04X}");
    }
    out.push('>');
    out
}

/// The text as parts for the fonts: `true` for a part of wide characters.
fn runs(text: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    for c in text.chars() {
        let w = crate::page::wide(c);
        match out.last_mut() {
            Some((last, part)) if *last == w => part.push(c),
            _ => out.push((w, c.to_string())),
        }
    }
    out
}

/// True if a page has text for the Japanese font.
fn has_wide(pages: &[Page]) -> bool {
    pages.iter().flat_map(|p| &p.ops).any(|op| match op {
        Op::Text { text, .. } => text.chars().any(crate::page::wide),
        _ => false,
    })
}

/// The title of the document: a WinAnsi string, or UTF-16 if the title has
/// wide characters.
fn title_string(title: &str) -> String {
    if !title.chars().any(crate::page::wide) {
        return pdf_string(title);
    }
    let mut out = String::from("<FEFF");
    for unit in title.encode_utf16() {
        let _ = write!(out, "{unit:04X}");
    }
    out.push('>');
    out
}

fn jpeg(pic: &crate::raster::Picture) -> Option<(Vec<u8>, u32, u32)> {
    let rgb: Vec<u8> = pic
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            // Over white, in linear light (ADR 0123).
            let a = f64::from(p[3]) / 255.0;
            let over = |c: u8| crate::light::mix(c, 255, a);
            [over(p[0]), over(p[1]), over(p[2])]
        })
        .collect();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode(&rgb, pic.width, pic.height, image::ExtendedColorType::Rgb8)
        .ok()?;
    Some((out, pic.width, pic.height))
}

/// The content stream of one page. The pictures are `/Im0`, `/Im1`, … in
/// the order of the page.
fn content(page: &Page) -> String {
    let h = page.height_mm;
    let x = |v: f64| num(v * PT_PER_MM);
    let y = |v: f64| num((h - v) * PT_PER_MM);
    let mut s = String::new();
    let mut image = 0usize;
    for op in &page.ops {
        match op {
            Op::Image { rect, .. } => {
                let _ = writeln!(
                    s,
                    "q {} 0 0 {} {} {} cm /Im{image} Do Q",
                    num(rect.width() * PT_PER_MM),
                    num(rect.height() * PT_PER_MM),
                    x(rect.min.x),
                    y(rect.max.y)
                );
                image += 1;
            }
            Op::Text {
                at,
                size_pt,
                bold,
                color,
                text,
            } => {
                let font = if *bold { "F2" } else { "F1" };
                let runs = runs(text);
                if runs.iter().any(|(wide, _)| *wide) {
                    // Each part in its font; the reader puts a part after
                    // the part before it.
                    let _ = write!(s, "BT {} rg {} {} Td", rgb(*color), x(at.x), y(at.y));
                    for (wide, part) in &runs {
                        let (font, string) = if *wide {
                            (WIDE_FONT, wide_string(part))
                        } else {
                            (font, pdf_string(part))
                        };
                        let _ = write!(s, " /{font} {} Tf {string} Tj", num(*size_pt));
                    }
                    let _ = writeln!(s, " ET");
                    continue;
                }
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

/// The `/XObject` resources of a page with the pictures `ids` (object
/// numbers), or nothing.
fn xobjects(ids: &[usize]) -> String {
    if ids.is_empty() {
        return String::new();
    }
    let list: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(k, id)| format!("/Im{k} {id} 0 R"))
        .collect();
    format!(" /XObject << {} >>", list.join(" "))
}

/// Writes the pages as one PDF file.
pub fn write(pages: &[Page], title: &str) -> Vec<u8> {
    // Objects: 1 catalog, 2 page tree, 3 and 4 fonts, 5 info, then a page
    // and its content for each page, then the pictures.
    let page_id = |i: usize| 6 + 2 * i;
    let mut pictures: Vec<Vec<u8>> = Vec::new();
    let mut page_images: Vec<Vec<usize>> = Vec::new();
    for p in pages {
        let mut ids = Vec::new();
        for op in &p.ops {
            if let Op::Image { picture, .. } = op {
                let body = match jpeg(picture) {
                    Some((bytes, w, h)) => {
                        let mut b = format!(
                            "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                            bytes.len()
                        )
                        .into_bytes();
                        b.extend(bytes);
                        b.extend(b"\nendstream");
                        b
                    }
                    // A white pixel, if the picture cannot be encoded.
                    None => b"<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 1 >>\nstream\n\xff\nendstream".to_vec(),
                };
                ids.push(page_id(pages.len()) + pictures.len());
                pictures.push(body);
            }
        }
        page_images.push(ids);
    }
    // The Japanese font (three objects after the pictures), only in a
    // document that has text for it.
    let wide_id = page_id(pages.len()) + pictures.len();
    let wide_font = if has_wide(pages) {
        format!(" /{WIDE_FONT} {wide_id} 0 R")
    } else {
        String::new()
    };
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: Vec<String> = (0..pages.len()).map(|i| format!("{} 0 R", page_id(i))).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()).into_bytes());
    for font in ["Helvetica", "Helvetica-Bold"] {
        objects.push(
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>").into_bytes(),
        );
    }
    objects.push(format!("<< /Title {} /Producer (ShokujoDX) >>", title_string(title)).into_bytes());
    for (i, p) in pages.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R{wide_font} >>{} >> /Contents {} 0 R >>",
                num(p.width_mm * PT_PER_MM),
                num(p.height_mm * PT_PER_MM),
                xobjects(&page_images[i]),
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
    objects.extend(pictures);
    if !wide_font.is_empty() {
        // A standard font for Japanese (Adobe-Japan1), with its characters
        // by their Unicode numbers. All characters are 1 em wide.
        const NAME: &str = "HeiseiKakuGo-W5";
        objects.push(
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /{NAME} /Encoding /UniJIS-UCS2-H /DescendantFonts [{} 0 R] >>",
                wide_id + 1
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{NAME} /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> /FontDescriptor {} 0 R /DW 1000 >>",
                wide_id + 2
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Type /FontDescriptor /FontName /{NAME} /Flags 4 /FontBBox [-92 -250 1010 922] /ItalicAngle 0 /Ascent 752 /Descent -221 /CapHeight 737 /StemV 114 >>"
            )
            .into_bytes(),
        );
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
                    ..Default::default()
                };
                3
            ],
            width_mm: 0.35,
        });
        p
    }

    /// A page with a picture: red at the top, transparent at the bottom.
    fn with_picture() -> Page {
        let mut p = sample();
        let mut rgba = vec![0u8; 40 * 20 * 4];
        for px in rgba[..40 * 10 * 4].as_chunks_mut::<4>().0 {
            *px = [255, 0, 0, 255];
        }
        p.ops.push(Op::Image {
            rect: BBox::new(Point::new(20.0, 100.0), Point::new(100.0, 140.0)),
            picture: std::sync::Arc::new(crate::raster::Picture {
                width: 40,
                height: 20,
                rgba,
            }),
        });
        p
    }

    #[test]
    fn a_picture_is_a_jpeg_image_of_its_page() {
        let pdf = write(&[sample(), with_picture()], "Test");
        let text: String = pdf.iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
        assert_eq!(text.matches("/Subtype /Image").count(), 1);
        assert!(text.contains("/Width 40 /Height 20 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"));
        // The first page has no pictures; the second has /Im0 (object 10).
        assert_eq!(text.matches("/XObject << /Im0 10 0 R >>").count(), 1);
        let c = content(&with_picture());
        let (w, h) = (num(80.0 * PT_PER_MM), num(40.0 * PT_PER_MM));
        assert!(
            c.contains(&format!(
                "q {w} 0 0 {h} {} {} cm /Im0 Do Q",
                num(20.0 * PT_PER_MM),
                num(157.0 * PT_PER_MM)
            )),
            "{c}"
        );
        // The JPEG decodes: red at the top, white (not black) at the bottom.
        let (bytes, ..) = jpeg(match &with_picture().ops.last().unwrap() {
            Op::Image { picture, .. } => picture,
            _ => unreachable!(),
        })
        .unwrap();
        let img = image::load_from_memory(&bytes).unwrap().to_rgb8();
        let (top, bottom) = (img.get_pixel(20, 2).0, img.get_pixel(20, 17).0);
        assert!(top[0] > 220 && top[1] < 40, "{top:?}");
        assert!(bottom.iter().all(|&c| c > 230), "{bottom:?}");
    }

    #[test]
    fn structure_and_offsets() {
        let pdf = write(&[sample(), with_picture()], "Test");
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
        assert_eq!(entries.len(), 10);
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

    /// Japanese text prints in the Japanese font, part by part, and a
    /// document with no such text has no such font.
    #[test]
    fn japanese_text_has_its_font() {
        let mut p = sample();
        p.text(10.0, 10.0, 9.0, false, "枠: Crown 260 × 200 (€5)");
        let pdf = String::from_utf8_lossy(&write(&[p], "作業シート")).into_owned();
        // 枠 is U+67A0; the Latin part follows in Helvetica.
        assert!(
            pdf.contains(r"/F3 9 Tf <67A0> Tj /F1 9 Tf (: Crown 260 \327 200 \(\2005\)) Tj ET"),
            "{pdf}"
        );
        assert!(pdf.contains("/F1 3 0 R /F2 4 0 R /F3 8 0 R >>"), "{pdf}");
        assert!(pdf.contains(
            "8 0 obj
<< /Type /Font /Subtype /Type0 /BaseFont /HeiseiKakuGo-W5 /Encoding /UniJIS-UCS2-H /DescendantFonts [9 0 R] >>"
        ));
        assert!(pdf.contains(
            "/CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> /FontDescriptor 10 0 R /DW 1000 >>"
        ));
        assert!(pdf.contains("/Title <FEFF4F5C696D30B730FC30C8>"), "the title in UTF-16");
        let plain = String::from_utf8_lossy(&write(&[sample()], "Patch")).into_owned();
        assert!(!plain.contains("F3") && !plain.contains("Heisei"));
        assert_eq!(
            runs("a枠b"),
            [
                (false, "a".to_string()),
                (true, "枠".to_string()),
                (false, "b".to_string())
            ]
        );
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
