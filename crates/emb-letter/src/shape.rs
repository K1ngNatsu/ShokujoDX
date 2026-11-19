//! Text shaping (P17 Milestone 6, ADR 0114): from the characters of a line
//! to the glyphs of the font and their places.
//!
//! Many writing systems need more than one glyph for each character in
//! turn: Arabic letters have a form for their place in the word and join,
//! Devanagari letters make conjuncts and some vowel signs go before their
//! consonant, and Latin fonts have ligatures ("fi") and forms that depend
//! on the letters next to them (script fonts). Arabic and Hebrew go from
//! right to left, and a line can have both directions.
//!
//! The work is done by two libraries in Rust: `unicode-bidi` (the Unicode
//! bidirectional algorithm: the runs of a line and their directions) and
//! `harfrust` (a port of HarfBuzz: the glyphs of each run, with the
//! substitution and positioning tables of the font).

use harfrust::{Direction, Feature, Script, ShapeOptions, ShaperData, Tag, UnicodeBuffer, script};
use unicode_bidi::BidiInfo;

/// One glyph of a shaped line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// The glyph of the font (a letter of the source).
    pub letter: u32,
    /// The index (in characters of the line) of the first character of the
    /// cluster that the glyph is a part of. A cluster is one or more
    /// characters that shape together: a base with its marks, a ligature.
    pub cluster: usize,
    /// The place of the glyph on the line, in font units, with Y down: the
    /// glyphs are in visual order, and X goes from the left end of the
    /// line to the right.
    pub x: f64,
    pub y: f64,
    /// How far the pen goes on after the glyph.
    pub advance: f64,
    /// The glyph is in a run from right to left.
    pub rtl: bool,
}

/// The script of a character, as the shaping library sees it.
fn script_of(c: char) -> Script {
    let mut b = UnicodeBuffer::new();
    let mut utf8 = [0u8; 4];
    b.push_str(c.encode_utf8(&mut utf8));
    b.guess_segment_properties();
    b.script()
}

/// A script that does not set the script of a run: marks, digits, spaces
/// and signs take the script of the letters next to them.
fn weak(s: Script) -> bool {
    s == script::COMMON || s == script::INHERITED || s == script::UNKNOWN
}

/// The parts of the byte range `run` of `line` with one script each, in
/// the order of the text: (bytes, script).
fn script_runs(line: &str, run: std::ops::Range<usize>) -> Vec<(std::ops::Range<usize>, Script)> {
    let mut out: Vec<(std::ops::Range<usize>, Script)> = Vec::new();
    for (at, c) in line[run.clone()].char_indices() {
        let (at, s) = (run.start + at, script_of(c));
        let end = at + c.len_utf8();
        match out.last_mut() {
            // A weak character goes with the run before it.
            Some((range, _)) if weak(s) => range.end = end,
            Some((range, script)) if *script == s || weak(*script) => {
                range.end = end;
                *script = s;
            }
            _ => out.push((at..end, s)),
        }
    }
    out
}

/// The glyphs of `line` (one line, with no line break) in the font `data`
/// (the face `index` of the file), in visual order. `kerning`: use the
/// kerning of the font. `None` if the shaping library cannot read the
/// font.
pub fn shape_line(data: &[u8], index: u32, line: &str, kerning: bool) -> Option<Vec<ShapedGlyph>> {
    let font = harfrust::FontRef::from_index(data, index).ok()?;
    let shaper_data = ShaperData::new(&font);
    let shaper = shaper_data.shaper(&font).build();
    let features: Vec<Feature> = if kerning {
        Vec::new()
    } else {
        vec![Feature::new(Tag::new(b"kern"), 0, ..)]
    };
    // The index in characters of each byte of the line.
    let mut char_at = vec![0usize; line.len() + 1];
    for (n, (at, c)) in line.char_indices().enumerate() {
        for slot in char_at.iter_mut().skip(at).take(c.len_utf8()) {
            *slot = n;
        }
    }
    let mut out = Vec::new();
    if line.is_empty() {
        return Some(out);
    }
    let bidi = BidiInfo::new(line, None);
    let mut pen = 0.0;
    for para in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(para, para.range.clone());
        for run in runs {
            let rtl = levels[run.start].is_rtl();
            let mut parts = script_runs(line, run);
            if rtl {
                // The last part of the text is at the left.
                parts.reverse();
            }
            for (range, script) in parts {
                let mut buffer = UnicodeBuffer::new();
                buffer.push_str(&line[range.clone()]);
                buffer.set_direction(if rtl {
                    Direction::RightToLeft
                } else {
                    Direction::LeftToRight
                });
                if !weak(script) {
                    buffer.set_script(script);
                }
                buffer.guess_segment_properties();
                let glyphs = shaper.shape(buffer, ShapeOptions::new().features(&features));
                for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
                    let byte = range.start + info.cluster as usize;
                    out.push(ShapedGlyph {
                        letter: info.glyph_id,
                        cluster: char_at[byte.min(line.len())],
                        x: pen + f64::from(pos.x_offset),
                        y: -f64::from(pos.y_offset),
                        advance: f64::from(pos.x_advance),
                        rtl,
                    });
                    pen += f64::from(pos.x_advance);
                }
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled;

    fn file(name: &str) -> &'static [u8] {
        bundled::FILES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, bytes)| *bytes)
            .unwrap()
    }

    fn lato() -> &'static [u8] {
        file("Lato-Bold.ttf")
    }

    /// The glyphs of `text` in the bundled font `name`: (glyph, cluster).
    fn glyphs(name: &str, text: &str) -> Vec<(u32, usize)> {
        shape_line(file(name), 0, text, true)
            .unwrap()
            .iter()
            .map(|g| (g.letter, g.cluster))
            .collect()
    }

    /// The exit test of P17 Milestone 6: the test texts give the glyphs of
    /// a reference shaping. The reference is the output of the shaping
    /// library (harfrust 0.12, which follows HarfBuzz 13.0.0) for the
    /// bundled fonts, written down here after a check of the sewn words by
    /// eye. A change of the library or of a font that changes a glyph shows
    /// here.
    #[test]
    fn the_test_texts_give_the_glyphs_of_the_reference() {
        // Arabic, "salam" (seen, lam, alef, meem), from right to left: the
        // meem is at the left. Lam and alef are one ligature.
        let salam = "\u{0633}\u{0644}\u{0627}\u{0645}";
        assert_eq!(glyphs("Tajawal-Bold.ttf", salam), [(292, 3), (473, 1), (417, 0)]);
        // Devanagari, "kshatriya": two conjuncts, and the vowel sign I
        // before the conjunct that it goes with.
        let kshatriya = "\u{0915}\u{094D}\u{0937}\u{0924}\u{094D}\u{0930}\u{093F}\u{092F}";
        assert_eq!(
            glyphs("Poppins-Bold.ttf", kshatriya),
            [(70, 0), (639, 3), (164, 3), (61, 7)]
        );
        // A Latin script font: the ligature "ff", and two forms of "t".
        assert_eq!(
            glyphs("Pacifico-Regular.ttf", "office tt"),
            [
                (472, 0),
                (818, 1),
                (435, 3),
                (393, 4),
                (603, 5),
                (3, 6),
                (519, 7),
                (724, 8)
            ]
        );
        // A text font: the ligature "ffi".
        assert_eq!(glyphs("Lato-Bold.ttf", "office"), [(111, 0), (76, 1), (49, 4), (59, 5)]);
    }

    #[test]
    fn arabic_letters_join_and_go_from_right_to_left() {
        let salam = "\u{0633}\u{0644}\u{0627}\u{0645}";
        let g = shape_line(file("Tajawal-Bold.ttf"), 0, salam, true).unwrap();
        assert!(g.iter().all(|x| x.rtl));
        // Visual order: the first letter of the text is at the right.
        assert!(g.windows(2).all(|w| w[1].x > w[0].x && w[1].cluster < w[0].cluster));
        // Seen at the start of a word is not the seen that stands alone;
        // meem after alef (which does not join to the left) is.
        let alone = |c: &str| glyphs("Tajawal-Bold.ttf", c)[0].0;
        let seen = g.last().unwrap().letter;
        assert_ne!(seen, alone("\u{0633}"));
        assert_eq!(g[0].letter, alone("\u{0645}"));
        // Digits in the Arabic text read from left to right, and they are
        // at the left of the word before them.
        let line = format!("{salam} 12");
        let g = shape_line(file("Tajawal-Bold.ttf"), 0, &line, true).unwrap();
        let x = |cluster: usize| g.iter().find(|v| v.cluster == cluster).unwrap().x;
        assert!(x(5) < x(6), "1 is before 2");
        assert!(x(6) < x(3), "the digits are at the left of the word");
        assert!(g.iter().any(|v| v.rtl) && g.iter().any(|v| !v.rtl));
        // A Latin word and an Arabic word in one line.
        let line = format!("ab {salam}");
        let g = shape_line(file("Tajawal-Bold.ttf"), 0, &line, true).unwrap();
        assert_eq!(g[0].cluster, 0);
        assert!(!g[0].rtl && g.last().unwrap().rtl);
        assert_eq!(g.last().unwrap().cluster, 3, "the seen is at the right end");
    }

    #[test]
    fn devanagari_has_conjuncts_and_a_vowel_sign_before_its_consonant() {
        let kshatriya = "\u{0915}\u{094D}\u{0937}\u{0924}\u{094D}\u{0930}\u{093F}\u{092F}";
        let g = shape_line(file("Poppins-Bold.ttf"), 0, kshatriya, true).unwrap();
        // Eight characters are four glyphs in three clusters.
        assert_eq!(g.len(), 4);
        assert!(g.iter().all(|x| !x.rtl && x.letter != 0));
        // The cluster "tri" (ta, virama, ra, vowel sign I) has two glyphs:
        // the vowel sign, which comes last in the text, is at the left.
        let tri: Vec<&ShapedGlyph> = g.iter().filter(|x| x.cluster == 3).collect();
        assert_eq!(tri.len(), 2);
        let sign_alone = glyphs("Poppins-Bold.ttf", "\u{093F}");
        assert!(tri[0].x < tri[1].x);
        assert!(sign_alone.iter().all(|(id, _)| *id != tri[1].letter));
    }

    #[test]
    fn a_plain_line_has_a_glyph_for_each_character_from_left_to_right() {
        let g = shape_line(lato(), 0, "Hat 12", true).unwrap();
        assert_eq!(g.len(), 6);
        assert_eq!(g.iter().map(|x| x.cluster).collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5]);
        assert!(g.iter().all(|x| !x.rtl && x.letter != 0 && x.advance > 0.0));
        assert!(g.windows(2).all(|w| w[1].x > w[0].x));
        assert_eq!(shape_line(lato(), 0, "", true), Some(Vec::new()));
        assert!(shape_line(b"not a font", 0, "a", true).is_none());
    }

    #[test]
    fn kerning_moves_a_pair_and_can_be_off() {
        let width = |kerning: bool| {
            let g = shape_line(lato(), 0, "AV", kerning).unwrap();
            g[1].x - g[0].x
        };
        assert!(width(true) < width(false) - 10.0, "{} {}", width(true), width(false));
    }

    #[test]
    fn the_script_runs_of_a_line() {
        // Digits and spaces go with the letters before them; a new script
        // starts a new run.
        let line = "ab 12 \u{0915}\u{093F} 3";
        let runs = script_runs(line, 0..line.len());
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert_eq!(runs[0].1, script::LATIN);
        assert_eq!(&line[runs[0].0.clone()], "ab 12 ");
        assert_eq!(runs[1].1, script::DEVANAGARI);
        // A line that starts with a digit: the digit takes the script of
        // the letters after it.
        let runs = script_runs("1a", 0..2);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1, script::LATIN);
    }
}
