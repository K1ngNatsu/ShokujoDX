//! Bitmap fonts as embroidery fonts (P4 Milestone 7, ADR 0021): BDF (the
//! Glyph Bitmap Distribution Format, a public text format) and Windows
//! raster fonts (`.fon`: FNT resources in a New Executable file, a public
//! format). The outline of the pixels of each glyph goes to the glyph
//! shapes of TrueType letters, so the letters are satin or fill.

use std::collections::BTreeMap;

use emb_geom::{Point, Transform};
use emb_model::LetterStitch;

use crate::embfont::{DESIGN_CAP_MM, EmbFont, EmbLetter, round_part};
use crate::glyph::{AUTO_MIN_COVERAGE, GlyphShapes};

/// One glyph: rows of pixels from the top, `x` pixels right of the pen and
/// with its top row `top` pixels below the baseline (negative: above).
#[derive(Debug, Clone, PartialEq)]
pub struct BitmapGlyph {
    pub advance: i32,
    pub x: i32,
    pub top: i32,
    pub rows: Vec<Vec<bool>>,
}

/// A bitmap font face.
#[derive(Debug, Clone, PartialEq)]
pub struct BitmapFont {
    pub family: String,
    pub style: String,
    pub notes: String,
    /// Pixels above the baseline.
    pub ascent: i32,
    pub descent: i32,
    pub glyphs: BTreeMap<char, BitmapGlyph>,
}

impl BitmapFont {
    /// The capital height in pixels: the top of "H" (or the ascent).
    pub fn cap_height(&self) -> i32 {
        self.glyphs
            .get(&'H')
            .and_then(|g| {
                let first = g.rows.iter().position(|r| r.iter().any(|&p| p))?;
                Some(-(g.top + first as i32))
            })
            .filter(|&h| h > 0)
            .unwrap_or(self.ascent.max(1))
    }

    /// "8 px" and the face, for lists.
    pub fn label(&self) -> String {
        let style = if self.style.is_empty() {
            String::new()
        } else {
            format!(" {}", self.style)
        };
        format!("{}{style}, {} px", self.family, self.ascent + self.descent)
    }
}

/// Reads a BDF font.
pub fn read_bdf(text: &str) -> Result<BitmapFont, String> {
    let mut f = BitmapFont {
        family: String::new(),
        style: String::new(),
        notes: String::new(),
        ascent: 0,
        descent: 0,
        glyphs: BTreeMap::new(),
    };
    let mut lines = text.lines();
    if !lines.clone().next().is_some_and(|l| l.starts_with("STARTFONT")) {
        return Err("not a BDF font".into());
    }
    let mut font_name = String::new();
    let mut unicode = true;
    let unquote = |s: &str| s.trim().trim_matches('"').to_string();
    let int = |s: Option<&str>| s.and_then(|v| v.parse::<i32>().ok());
    while let Some(line) = lines.next() {
        let mut w = line.split_whitespace();
        let key = w.next().unwrap_or("");
        let rest = line.get(key.len()..).unwrap_or("").trim();
        match key {
            "FONT" => font_name = rest.to_string(),
            "FAMILY_NAME" => f.family = unquote(rest),
            "WEIGHT_NAME" => f.style = unquote(rest),
            "COPYRIGHT" => f.notes = unquote(rest),
            "FONT_ASCENT" => f.ascent = int(w.next()).unwrap_or(0),
            "FONT_DESCENT" => f.descent = int(w.next()).unwrap_or(0),
            "CHARSET_REGISTRY" => {
                let r = unquote(rest).to_ascii_uppercase();
                unicode = r.starts_with("ISO10646") || r.starts_with("ISO8859");
            }
            "STARTCHAR" => {
                let (mut code, mut adv, mut bbx) = (-1i32, 0i32, (0i32, 0i32, 0i32, 0i32));
                let mut rows: Vec<Vec<bool>> = Vec::new();
                for l in lines.by_ref() {
                    let mut w = l.split_whitespace();
                    match w.next().unwrap_or("") {
                        "ENCODING" => code = int(w.next()).unwrap_or(-1),
                        "DWIDTH" => adv = int(w.next()).unwrap_or(0),
                        "BBX" => {
                            let v: Vec<i32> = w.filter_map(|x| x.parse().ok()).collect();
                            if v.len() == 4 {
                                bbx = (v[0], v[1], v[2], v[3]);
                            }
                        }
                        "BITMAP" => {}
                        "ENDCHAR" => break,
                        hex if bbx.0 > 0 && rows.len() < bbx.1 as usize => {
                            let bits: Vec<bool> = (0..bbx.0 as usize)
                                .map(|i| {
                                    let byte = hex
                                        .get(2 * (i / 8)..2 * (i / 8) + 2)
                                        .and_then(|b| u8::from_str_radix(b, 16).ok())
                                        .unwrap_or(0);
                                    byte & (0x80 >> (i % 8)) != 0
                                })
                                .collect();
                            rows.push(bits);
                        }
                        _ => {}
                    }
                }
                let c = (code >= 0 && unicode).then(|| char::from_u32(code as u32)).flatten();
                if let Some(c) = c {
                    let (_, h, xoff, yoff) = bbx;
                    f.glyphs.insert(
                        c,
                        BitmapGlyph {
                            advance: adv,
                            x: xoff,
                            top: -(yoff + h),
                            rows,
                        },
                    );
                }
            }
            _ => {}
        }
    }
    if f.family.is_empty() {
        f.family = font_name.split('-').nth(2).unwrap_or("Bitmap font").to_string();
    }
    if f.ascent == 0 {
        f.ascent = f.glyphs.values().map(|g| -g.top).max().unwrap_or(1);
    }
    if f.glyphs.is_empty() {
        return Err("the BDF font has no characters".into());
    }
    Ok(f)
}

/// Windows-1252 characters 0x80 to 0x9F (0: none).
const CP1252: [u16; 32] = [
    0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D, 0, 0,
    0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0, 0x017E, 0x0178,
];

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Reads the raster fonts of a Windows `.fon` file (each size is one face).
pub fn read_fon(bytes: &[u8]) -> Result<Vec<BitmapFont>, String> {
    let bad = || "not a Windows raster font file".to_string();
    if bytes.get(0..2) != Some(b"MZ") {
        return Err(bad());
    }
    let ne = u32_at(bytes, 0x3C).ok_or_else(bad)? as usize;
    if bytes.get(ne..ne + 2) != Some(b"NE") {
        return Err(bad());
    }
    let table = ne + usize::from(u16_at(bytes, ne + 0x24).ok_or_else(bad)?);
    let shift = u16_at(bytes, table).ok_or_else(bad)?;
    let mut at = table + 2;
    let mut fonts = Vec::new();
    loop {
        let ty = u16_at(bytes, at).ok_or_else(bad)?;
        if ty == 0 {
            break;
        }
        let count = usize::from(u16_at(bytes, at + 2).ok_or_else(bad)?);
        at += 8;
        for i in 0..count {
            let e = at + i * 12;
            if ty == 0x8008 {
                let off = (usize::from(u16_at(bytes, e).ok_or_else(bad)?)) << shift;
                let len = (usize::from(u16_at(bytes, e + 2).ok_or_else(bad)?)) << shift;
                if let Some(res) = bytes.get(off..(off + len).min(bytes.len()))
                    && let Some(f) = read_fnt(res)
                {
                    fonts.push(f);
                }
            }
        }
        at += count * 12;
    }
    if fonts.is_empty() {
        return Err("the file has no raster fonts (vector fonts are not read)".into());
    }
    Ok(fonts)
}

/// The bitmap font faces of a file by its extension (`bdf` or `fon`), the
/// largest first (the most pixels give the best letters).
pub fn read_file(ext: &str, bytes: &[u8]) -> Result<Vec<BitmapFont>, String> {
    let mut faces = match ext.to_ascii_lowercase().as_str() {
        "bdf" => vec![read_bdf(&String::from_utf8_lossy(bytes))?],
        "fon" | "fnt" => read_fon(bytes)?,
        e => return Err(format!(".{e} is not a bitmap font")),
    };
    faces.sort_by_key(|f| -(f.ascent + f.descent));
    Ok(faces)
}

/// The file extensions of bitmap fonts.
pub const EXTENSIONS: [&str; 2] = ["bdf", "fon"];

/// One FNT resource (version 2 or 3), raster only.
fn read_fnt(b: &[u8]) -> Option<BitmapFont> {
    let version = u16_at(b, 0)?;
    if !matches!(version, 0x200 | 0x300) || u16_at(b, 66)? & 1 != 0 {
        return None;
    }
    let cstr = |o: usize| -> String {
        let s = b.get(o..).unwrap_or(&[]);
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        String::from_utf8_lossy(&s[..end]).trim().to_string()
    };
    let copyright = cstr(6).chars().take(60).collect::<String>();
    let ascent = i32::from(u16_at(b, 74)?);
    let weight = u16_at(b, 83)?;
    let charset = *b.get(85)?;
    let height = usize::from(u16_at(b, 88)?);
    let first = *b.get(95)?;
    let last = *b.get(96)?;
    let face = cstr(u32_at(b, 105)? as usize);
    let (table, entry) = if version == 0x200 { (118, 4) } else { (148, 6) };
    let mut glyphs = BTreeMap::new();
    for (k, code) in (first..=last).enumerate() {
        let e = table + k * entry;
        let width = usize::from(u16_at(b, e)?);
        let off = if version == 0x200 {
            usize::from(u16_at(b, e + 2)?)
        } else {
            u32_at(b, e + 2)? as usize
        };
        let c = match code {
            0x20..=0x7E | 0xA0..=0xFF => char::from(code),
            0x80..=0x9F if charset == 0 => match CP1252[usize::from(code - 0x80)] {
                0 => continue,
                u => char::from_u32(u32::from(u))?,
            },
            _ => continue,
        };
        if charset != 0 && code >= 0x80 {
            continue;
        }
        // Columns of 8 pixels, each column `height` bytes from the top.
        let cols = width.div_ceil(8);
        let rows: Vec<Vec<bool>> = (0..height)
            .map(|r| {
                (0..width)
                    .map(|x| {
                        b.get(off + (x / 8) * height + r)
                            .is_some_and(|byte| byte & (0x80 >> (x % 8)) != 0)
                    })
                    .collect()
            })
            .collect();
        let _ = cols;
        glyphs.insert(
            c,
            BitmapGlyph {
                advance: width as i32,
                x: 0,
                top: -ascent,
                rows,
            },
        );
    }
    let style = match weight {
        0..=350 => "Light",
        351..=550 => "",
        _ => "Bold",
    };
    Some(BitmapFont {
        family: if face.is_empty() { "Raster font".into() } else { face },
        style: style.into(),
        notes: copyright,
        ascent,
        descent: height as i32 - ascent,
        glyphs,
    })
}

/// The outline of the pixels of a glyph as rings, in pixels (Y down, the
/// baseline at 0). Pixels that touch only at a corner stay apart.
pub fn pixel_rings(g: &BitmapGlyph) -> Vec<Vec<Point>> {
    let h = g.rows.len() as i32;
    let w = g.rows.iter().map(Vec::len).max().unwrap_or(0) as i32;
    let on = |x: i32, y: i32| x >= 0 && y >= 0 && y < h && g.rows[y as usize].get(x as usize).copied().unwrap_or(false);
    // Directed edges with the pixel on their left (in Y-down coordinates:
    // each ring turns the same way round).
    let mut next: BTreeMap<(i32, i32), Vec<(i32, i32)>> = BTreeMap::new();
    let mut add = |a: (i32, i32), b: (i32, i32)| next.entry(a).or_default().push(b);
    for y in 0..h {
        for x in 0..w {
            if !on(x, y) {
                continue;
            }
            if !on(x, y - 1) {
                add((x + 1, y), (x, y));
            }
            if !on(x - 1, y) {
                add((x, y), (x, y + 1));
            }
            if !on(x, y + 1) {
                add((x, y + 1), (x + 1, y + 1));
            }
            if !on(x + 1, y) {
                add((x + 1, y + 1), (x + 1, y));
            }
        }
    }
    let mut rings = Vec::new();
    while let Some((&start, _)) = next.iter().find(|(_, v)| !v.is_empty()) {
        let mut ring = vec![start];
        let mut prev = start;
        let mut cur = next.get_mut(&start).and_then(Vec::pop).unwrap_or(start);
        while cur != start {
            let d_in = (cur.0 - prev.0, cur.1 - prev.1);
            let outs = next.get_mut(&cur).filter(|v| !v.is_empty());
            let Some(outs) = outs else { break };
            // At a corner with two ways on, turn so that diagonal pixels stay apart.
            let pick = (0..outs.len())
                .min_by_key(|&i| {
                    let d = (outs[i].0 - cur.0, outs[i].1 - cur.1);
                    let cross = d_in.0 * d.1 - d_in.1 * d.0;
                    -cross
                })
                .unwrap_or(0);
            let n = outs.swap_remove(pick);
            ring.push(cur);
            prev = cur;
            cur = n;
        }
        // Remove the points in the middle of straight sides.
        let n = ring.len();
        let pts: Vec<Point> = (0..n)
            .filter(|&i| {
                let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0) != 0
            })
            .map(|i| Point::new(f64::from(ring[i].0 + g.x), f64::from(ring[i].1 + g.top)))
            .collect();
        if pts.len() >= 3 {
            rings.push(pts);
        }
    }
    rings
}

/// A bitmap font as an embroidery font. Each glyph's pixel outline becomes
/// satin or fill letters as for TrueType fonts.
pub fn to_embfont(b: &BitmapFont, family: &str, stitch: LetterStitch) -> EmbFont {
    let cap = f64::from(b.cap_height());
    let s = DESIGN_CAP_MM / cap;
    let t = Transform::scale(s, s);
    let mut f = EmbFont::empty(family, &b.style);
    f.notes = format!("Made from the bitmap font {}. {}", b.label(), b.notes)
        .trim()
        .to_string();
    f.line_height_mm = f64::from(b.ascent + b.descent) * s * 1.1;
    if let Some(space) = b.glyphs.get(&' ') {
        f.space_mm = f64::from(space.advance) * s;
        f.set_letter(
            ' ',
            EmbLetter {
                advance_mm: f.space_mm,
                parts: Vec::new(),
            },
        );
    }
    for (&c, g) in &b.glyphs {
        let rings = pixel_rings(g);
        if rings.is_empty() {
            continue;
        }
        let shapes = GlyphShapes::new(&rings, cap);
        let satin = match stitch {
            LetterStitch::Satin => !shapes.strokes.satins.is_empty(),
            LetterStitch::Fill => false,
            LetterStitch::Auto => !shapes.strokes.satins.is_empty() && shapes.strokes.coverage >= AUTO_MIN_COVERAGE,
        };
        let mut parts = if satin {
            shapes.satin_parts()
        } else {
            shapes.fill_parts()
        };
        for p in &mut parts {
            p.transform(&t);
            round_part(p);
        }
        f.set_letter(
            c,
            EmbLetter {
                advance_mm: f64::from(g.advance) * s,
                parts,
            },
        );
    }
    // One pixel is the stroke width: the size range of the font.
    let ratio = 1.0 / cap;
    let range = if stitch == LetterStitch::Fill {
        LetterStitch::Fill
    } else {
        LetterStitch::Satin
    };
    if let Some((lo, hi)) = crate::source::size_range(ratio, range) {
        f.min_height_mm = lo;
        f.max_height_mm = hi;
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    const BDF: &str = "STARTFONT 2.1
FONT -test-Tiny-Medium-R-Normal--8-80-75-75-C-60-ISO10646-1
SIZE 8 75 75
FONTBOUNDINGBOX 6 8 0 -1
STARTPROPERTIES 4
FAMILY_NAME \"Tiny\"
WEIGHT_NAME \"Medium\"
FONT_ASCENT 7
FONT_DESCENT 1
ENDPROPERTIES
CHARS 3
STARTCHAR H
ENCODING 72
SWIDTH 600 0
DWIDTH 6 0
BBX 5 7 0 0
BITMAP
88
88
88
F8
88
88
88
ENDCHAR
STARTCHAR O
ENCODING 79
DWIDTH 6 0
BBX 5 7 0 0
BITMAP
70
88
88
88
88
88
70
ENDCHAR
STARTCHAR space
ENCODING 32
DWIDTH 4 0
BBX 0 0 0 0
BITMAP
ENDCHAR
ENDFONT
";

    #[test]
    fn bdf_reads_and_converts() {
        let f = read_bdf(BDF).unwrap();
        assert_eq!((f.family.as_str(), f.ascent, f.descent), ("Tiny", 7, 1));
        assert_eq!(f.glyphs.len(), 3);
        let h = &f.glyphs[&'H'];
        assert_eq!(
            (h.top, h.rows.len(), h.rows[3].iter().filter(|&&p| p).count()),
            (-7, 7, 5)
        );
        assert_eq!(f.cap_height(), 7);
        // "H" is one ring; "O" is two (outline and counter).
        assert_eq!(pixel_rings(h).len(), 1);
        assert_eq!(pixel_rings(&f.glyphs[&'O']).len(), 2);
        let e = to_embfont(&f, "Tiny Stitch", LetterStitch::Auto);
        let l = &e.letters[&'H'];
        assert!(!l.parts.is_empty());
        let b = l
            .parts
            .iter()
            .filter_map(|p| p.control_bbox())
            .reduce(emb_geom::BBox::union)
            .unwrap();
        assert!((b.height() - 25.0).abs() < 2.0, "{b:?}");
        assert!(b.max.y.abs() < 1.0, "on the baseline: {b:?}");
        assert!(e.letters.contains_key(&' '));
        assert!(read_bdf("hello").is_err());
    }

    /// A minimal `.fon` file: an NE file with one FNT 2.0 resource with
    /// the characters "H" and "I", 8 pixels high.
    fn tiny_fon() -> Vec<u8> {
        let height = 8usize;
        let (first, last) = (b'H', b'I');
        let n = usize::from(last - first) + 2;
        let table = 118;
        let bits_at = table + n * 4;
        let glyph_h: [u8; 8] = [0x00, 0x88, 0x88, 0xF8, 0x88, 0x88, 0x88, 0x00];
        let glyph_i: [u8; 8] = [0x00, 0xE0, 0x40, 0x40, 0x40, 0x40, 0xE0, 0x00];
        let face_at = bits_at + 16;
        let mut fnt = vec![0u8; face_at + 5];
        fnt[0..2].copy_from_slice(&0x200u16.to_le_bytes());
        fnt[74..76].copy_from_slice(&7u16.to_le_bytes()); // ascent
        fnt[83..85].copy_from_slice(&400u16.to_le_bytes());
        fnt[88..90].copy_from_slice(&(height as u16).to_le_bytes());
        fnt[95] = first;
        fnt[96] = last;
        fnt[105..109].copy_from_slice(&(face_at as u32).to_le_bytes());
        for (k, w) in [(0usize, 6u16), (1, 4)] {
            let e = table + k * 4;
            fnt[e..e + 2].copy_from_slice(&w.to_le_bytes());
            fnt[e + 2..e + 4].copy_from_slice(&((bits_at + k * 8) as u16).to_le_bytes());
        }
        fnt[bits_at..bits_at + 8].copy_from_slice(&glyph_h);
        fnt[bits_at + 8..bits_at + 16].copy_from_slice(&glyph_i);
        fnt[face_at..face_at + 4].copy_from_slice(b"Pix\0");
        // MZ header, NE header at 0x40, resource table at NE + 0x40.
        let ne = 0x40;
        let rt = 0x40;
        let res_at = 0x200;
        let mut f = vec![0u8; res_at];
        f[0..2].copy_from_slice(b"MZ");
        f[0x3C..0x40].copy_from_slice(&(ne as u32).to_le_bytes());
        f[ne..ne + 2].copy_from_slice(b"NE");
        f[ne + 0x24..ne + 0x26].copy_from_slice(&(rt as u16).to_le_bytes());
        let t = ne + rt;
        f[t..t + 2].copy_from_slice(&4u16.to_le_bytes()); // shift: units of 16 bytes
        f[t + 2..t + 4].copy_from_slice(&0x8008u16.to_le_bytes());
        f[t + 4..t + 6].copy_from_slice(&1u16.to_le_bytes());
        let e = t + 10;
        f[e..e + 2].copy_from_slice(&((res_at >> 4) as u16).to_le_bytes());
        f[e + 2..e + 4].copy_from_slice(&(fnt.len().div_ceil(16) as u16).to_le_bytes());
        f.extend_from_slice(&fnt);
        f.resize(res_at + fnt.len().div_ceil(16) * 16, 0);
        f
    }

    #[test]
    fn fon_reads_and_converts() {
        let fonts = read_fon(&tiny_fon()).unwrap();
        assert_eq!(fonts.len(), 1);
        let f = &fonts[0];
        assert_eq!(f.family, "Pix");
        assert_eq!(f.glyphs.len(), 2);
        assert_eq!(f.glyphs[&'H'].rows[3].iter().filter(|&&p| p).count(), 5);
        assert_eq!(f.cap_height(), 6);
        let e = to_embfont(f, "Pix Stitch", LetterStitch::Fill);
        assert!(matches!(e.letters[&'I'].parts[0], emb_model::ObjectKind::Fill { .. }));
        assert!(read_fon(b"MZ").is_err());
    }
}
