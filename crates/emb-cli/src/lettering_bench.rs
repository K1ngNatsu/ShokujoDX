//! The lettering bench (P20 Milestone 1, `docs/design/p20-plan.md`): how
//! the lettering engine sews Japanese text in a TrueType or OpenType font.
//!
//! For each character of the reference set ([`characters`]) at each size
//! of [`SIZES`] (the height of the lettering: the height of the characters,
//! ADR 0145), [`measure`] takes the parts that the lettering gets in Auto
//! (before the route) and measures:
//!
//! - the stitch of each part: a satin column, a satin column with split
//!   stitches (wider than its split length), a tatami column, a fill or a
//!   run;
//! - the part of the shape of the character that the parts cover, the area
//!   of the parts outside it, and the part of the area of the parts that
//!   two or more parts cover (two layers of thread: `quality::areas`);
//! - the narrowest column (the middle width of the narrowest one) and the
//!   widest column (its width between its two sides,
//!   `glyph::column_width`), the columns wider than [`WIDE_MM`] and of
//!   them the satin columns with no split stitches (P20 Milestone 2: none).
//!
//! It also counts the trims of the reference words ([`WORDS`]). The test
//! `tests/lettering_bench.rs` keeps the numbers of the test font
//! (`tests/data/`), so a change of the engine shows in them.

use emb_geom::{Point, Transform};
use emb_letter::{Font, LetterSource, quality};
use emb_model::{ColumnPattern, Design, Lettering, ObjectKind, Rgb};

/// The heights of the lettering, mm: the height of the characters (P20
/// Milestone 5).
pub const SIZES: [f64; 5] = [10.0, 15.0, 25.0, 50.0, 80.0];
/// A column narrower than this does not sew well as satin, mm (P20
/// Milestone 5).
pub const NARROW_MM: f64 = 0.8;
/// A column wider than this is too wide for satin with no split
/// stitches, mm (P20 Milestone 2): it must have split stitches or be a
/// tatami column.
pub const WIDE_MM: f64 = 8.0;

/// The hiragana: the 46 basic characters, the 25 with voicing marks and
/// the 9 small ones.
pub const HIRAGANA: &str = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん\
がぎぐげござじずぜぞだぢづでどばびぶべぼぱぴぷぺぽぁぃぅぇぉっゃゅょ";
/// The katakana: the 46 basic characters, ヴ, the 9 small ones and the
/// long vowel mark.
pub const KATAKANA: &str = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン\
ヴァィゥェォッャュョー";
/// The 80 kanji of the first school year.
pub const KANJI_FIRST_YEAR: &str = "一右雨円王音下火花貝学気九休玉金空月犬見五口校左三山子四糸字耳七車手十出女小上森人水正生青夕石赤千川先早草足村大男竹中虫町天田土二日入年白八百文木本名目立力林六";
/// Kanji of the measures of 2026-11-20 that are not of the first year:
/// many strokes, many crossings, and words that a shop sews.
pub const KANJI_MORE: &str = "愛夢心龍鶴桜刺繍東京都道府県国語書漢海星光風雪侍忍武";
/// Japanese punctuation.
pub const PUNCTUATION: &str = "、。「」・";
/// Words that a shop sews.
pub const WORDS: [&str; 8] = [
    "刺繍",
    "東京",
    "ありがとう",
    "サムライ",
    "龍",
    "山田太郎",
    "さくら",
    "日本",
];

/// The characters of the reference set, each one time, in the order of
/// the sets.
pub fn characters() -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in [HIRAGANA, KATAKANA, KANJI_FIRST_YEAR, KANJI_MORE, PUNCTUATION]
        .iter()
        .flat_map(|s| s.chars())
    {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// The number of parts of each stitch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kinds {
    pub satin: usize,
    /// Satin columns wider than their split length: they get split
    /// stitches.
    pub split: usize,
    /// Columns with the column type Tatami.
    pub tatami_column: usize,
    pub fill: usize,
    pub run: usize,
}

impl Kinds {
    fn add(&mut self, o: Kinds) {
        self.satin += o.satin;
        self.split += o.split;
        self.tatami_column += o.tatami_column;
        self.fill += o.fill;
        self.run += o.run;
    }

    /// The parts that follow the strokes (all but fills).
    fn columns(&self) -> usize {
        self.satin + self.split + self.tatami_column + self.run
    }
}

/// The measures of one character at one size.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    pub c: char,
    pub size_mm: f64,
    pub kinds: Kinds,
    /// The part of the shape that the parts cover, 0 to 1.
    pub coverage: f64,
    /// The area of the parts outside the shape, as a part of its area.
    pub outside: f64,
    /// The part of the area of the parts that two or more parts cover.
    pub overlap: f64,
    /// The middle width of the narrowest column, mm.
    pub narrowest_mm: Option<f64>,
    /// The width of the widest column between its two sides
    /// (`glyph::column_width`), mm.
    pub widest_mm: Option<f64>,
    /// The columns wider than [`WIDE_MM`].
    pub wide_columns: usize,
    /// The columns wider than [`WIDE_MM`] that sew as satin with no split
    /// stitches.
    pub wide_unsplit: usize,
}

/// The widths of a column (mm): its middle width (half of its length is
/// narrower) and its widest place (the longest stitch across it, with no
/// pull compensation).
fn widths(g: &emb_model::SatinGeometry) -> Option<(f64, f64)> {
    let mut w = quality::column_widths(g, 0.25);
    w.retain(|(width, len)| width.is_finite() && *len > 0.0);
    let total: f64 = w.iter().map(|(_, l)| l).sum();
    if total <= 0.0 {
        return None;
    }
    let widest = w.iter().map(|(width, _)| *width).fold(f64::NAN, f64::max);
    w.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut sum = 0.0;
    let middle = w
        .iter()
        .find(|(_, l)| {
            sum += l;
            sum >= total / 2.0
        })
        .map_or(w[0].0, |(width, _)| *width);
    Some((middle, if widest.is_nan() { middle } else { widest }))
}

/// The measures of character `c` of `font` at the height `size_mm`, in
/// Auto. `None` if the font has no such character or it has no shape.
pub fn measure(font: &Font, c: char, size_mm: f64) -> Option<Measure> {
    let g = font.glyph(c)?;
    let scale = size_mm / emb_letter::layout::height_unit(font, &c.to_string(), true);
    let to_mm = Transform::scale(scale, scale);
    // The parts before the route (P20 Milestone 4): the route changes
    // their order, not their areas. The trims of the words are with routes.
    let mut parts = font.shapes(g).auto_parts();
    for p in &mut parts {
        p.transform(&to_mm);
    }
    // The stitch of the columns, as the layout chooses it for a text of
    // this one character.
    emb_letter::glyph::auto_stitches(&mut parts.iter_mut().collect::<Vec<_>>(), true);
    let ring = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| to_mm.apply(*p)).collect() };
    let outline: Vec<(Vec<Point>, Vec<Vec<Point>>)> = font
        .shapes(g)
        .fills
        .iter()
        .map(|(outer, holes)| (ring(outer), holes.iter().map(ring).collect()))
        .collect();
    let areas = quality::areas(&parts, &outline)?;
    let mut kinds = Kinds::default();
    let (mut narrowest, mut widest): (Option<f64>, Option<f64>) = (None, None);
    let (mut wide_columns, mut wide_unsplit) = (0, 0);
    for p in &parts {
        match p {
            ObjectKind::Satin { geometry, params } => {
                let w = widths(geometry);
                if let Some((middle, _)) = w {
                    narrowest = Some(narrowest.map_or(middle, |n| n.min(middle)));
                }
                let real = emb_letter::glyph::column_width(geometry);
                if let Some(real) = real {
                    widest = Some(widest.map_or(real, |x| x.max(real)));
                }
                let split = w.is_some_and(|(_, top)| top > params.split_mm);
                let tatami = matches!(params.pattern, ColumnPattern::Tatami { .. });
                if real.is_some_and(|r| r > WIDE_MM) {
                    wide_columns += 1;
                    if !tatami && !split {
                        wide_unsplit += 1;
                    }
                }
                if tatami {
                    kinds.tatami_column += 1;
                } else if split {
                    kinds.split += 1;
                } else {
                    kinds.satin += 1;
                }
            }
            ObjectKind::Running { .. } => kinds.run += 1,
            _ => kinds.fill += 1,
        }
    }
    Some(Measure {
        c,
        size_mm,
        kinds,
        coverage: areas.coverage,
        outside: areas.outside,
        overlap: areas.overlap,
        narrowest_mm: narrowest,
        widest_mm: widest,
        wide_columns,
        wide_unsplit,
    })
}

/// The trims of `text` in `src` at the height `size_mm`, sewn alone (one
/// lettering object, the stitch type Auto).
pub fn trims(src: &dyn LetterSource, text: &str, size_mm: f64) -> usize {
    let mut d = Design::new("bench");
    let thread = d.add_thread("Black", Rgb(0, 0, 0));
    let mut l = Lettering::new(text, src.font_ref(), size_mm, Point::ZERO);
    emb_letter::relayout(&mut l, src);
    d.add_object(thread, ObjectKind::Lettering(Box::new(l)));
    let Ok(opts) = crate::plan_options(&d, None) else {
        return 0;
    };
    crate::plan(&d, &opts).stitches.trim_count().saturating_sub(1)
}

/// The results of the bench for one font.
#[derive(Debug, Clone)]
pub struct Bench {
    pub font: String,
    /// The measures, by size and then in the order of [`characters`].
    pub rows: Vec<Measure>,
    /// The characters that the font does not have.
    pub missing: Vec<char>,
    /// The trims of each word at each size of [`SIZES`].
    pub words: Vec<(&'static str, Vec<usize>)>,
}

/// Runs the bench on `font`.
pub fn run(font: &Font) -> Bench {
    let chars = characters();
    let missing: Vec<char> = chars.iter().copied().filter(|c| font.glyph(*c).is_none()).collect();
    let rows = SIZES
        .iter()
        .flat_map(|&s| chars.iter().filter_map(move |&c| measure(font, c, s)))
        .collect();
    let words = WORDS
        .iter()
        .map(|w| (*w, SIZES.iter().map(|&s| trims(font, w, s)).collect()))
        .collect();
    Bench {
        font: format!("{} {}", font.family, font.style),
        rows,
        missing,
        words,
    }
}

fn pct(v: f64) -> String {
    format!("{:.1} %", v * 100.0)
}

/// The report of the bench: for each size, the characters that are all
/// fill or partly fill, the stitches of the parts, the areas (the worst
/// character in brackets) and the widths; then the trims of the words.
/// With `letters`, one line for each character and size too.
pub fn report(b: &Bench, letters: bool) -> String {
    let mut out = String::new();
    let n = characters().len();
    out.push_str(&format!(
        "Lettering bench: {}; {} characters of the reference set{}\n",
        b.font,
        n - b.missing.len(),
        if b.missing.is_empty() {
            String::new()
        } else {
            format!(
                " ({} missing: {})",
                b.missing.len(),
                b.missing.iter().collect::<String>()
            )
        }
    ));
    for &size in &SIZES {
        let rows: Vec<&Measure> = b.rows.iter().filter(|m| m.size_mm == size).collect();
        if rows.is_empty() {
            continue;
        }
        let mut kinds = Kinds::default();
        for m in &rows {
            kinds.add(m.kinds);
        }
        let all_fill: String = rows
            .iter()
            .filter(|m| m.kinds.fill > 0 && m.kinds.columns() == 0)
            .map(|m| m.c)
            .collect();
        let part_fill: String = rows
            .iter()
            .filter(|m| m.kinds.fill > 0 && m.kinds.columns() > 0)
            .map(|m| m.c)
            .collect();
        let worst = |f: fn(&Measure) -> f64, max: bool| -> (f64, char) {
            let pick = rows
                .iter()
                .map(|m| (f(m), m.c))
                .reduce(|a, b| if (b.0 > a.0) == max && b.0 != a.0 { b } else { a });
            pick.unwrap_or((0.0, ' '))
        };
        let mean = |f: fn(&Measure) -> f64| rows.iter().map(|m| f(m)).sum::<f64>() / rows.len() as f64;
        let (cov_min, cov_c) = worst(|m| m.coverage, false);
        let (out_max, out_c) = worst(|m| m.outside, true);
        let (two_max, two_c) = worst(|m| m.overlap, true);
        let narrow = rows
            .iter()
            .filter_map(|m| m.narrowest_mm.map(|w| (w, m.c)))
            .reduce(|a, b| if b.0 < a.0 { b } else { a });
        let wide = rows
            .iter()
            .filter_map(|m| m.widest_mm.map(|w| (w, m.c)))
            .reduce(|a, b| if b.0 > a.0 { b } else { a });
        let under = rows
            .iter()
            .filter(|m| m.narrowest_mm.is_some_and(|w| w < NARROW_MM))
            .count();
        let wide_columns: usize = rows.iter().map(|m| m.wide_columns).sum();
        let wide_unsplit: usize = rows.iter().map(|m| m.wide_unsplit).sum();
        out.push_str(&format!("\n{size} mm\n"));
        out.push_str(&format!(
            "  characters all fill: {} {}\n",
            all_fill.chars().count(),
            all_fill
        ));
        out.push_str(&format!(
            "  characters partly fill: {} {}\n",
            part_fill.chars().count(),
            part_fill
        ));
        out.push_str(&format!(
            "  parts: satin {}, split satin {}, tatami column {}, fill {}, run {}\n",
            kinds.satin, kinds.split, kinds.tatami_column, kinds.fill, kinds.run
        ));
        out.push_str(&format!(
            "  covered: mean {}, lowest {} ({cov_c})\n",
            pct(mean(|m| m.coverage)),
            pct(cov_min)
        ));
        out.push_str(&format!(
            "  outside: mean {}, highest {} ({out_c})\n",
            pct(mean(|m| m.outside)),
            pct(out_max)
        ));
        out.push_str(&format!(
            "  two layers: mean {}, highest {} ({two_c}); characters over 3 %: {}\n",
            pct(mean(|m| m.overlap)),
            pct(two_max),
            rows.iter().filter(|m| m.overlap > 0.03).count()
        ));
        out.push_str(&format!(
            "  narrowest column: {}; characters with a column under {NARROW_MM} mm: {under}\n",
            narrow.map_or("none".into(), |(w, c)| format!("{w:.2} mm ({c})"))
        ));
        out.push_str(&format!(
            "  widest column: {}; columns over {WIDE_MM} mm: {wide_columns}, satin with no split stitches: {wide_unsplit}\n",
            wide.map_or("none".into(), |(w, c)| format!("{w:.2} mm ({c})"))
        ));
    }
    out.push_str("\nTrims of the words at ");
    out.push_str(&SIZES.iter().map(|s| format!("{s}")).collect::<Vec<_>>().join(", "));
    out.push_str(" mm\n");
    for (w, t) in &b.words {
        out.push_str(&format!(
            "  {w}: {}\n",
            t.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
        ));
    }
    if letters {
        out.push_str("\nCharacter\tsize mm\tsatin\tsplit\ttatami column\tfill\trun\tcovered\toutside\ttwo layers\tnarrowest mm\twidest mm\n");
        for m in &b.rows {
            let k = m.kinds;
            let mm = |v: Option<f64>| v.map_or("-".into(), |w| format!("{w:.2}"));
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\n",
                m.c,
                m.size_mm,
                k.satin,
                k.split,
                k.tatami_column,
                k.fill,
                k.run,
                m.coverage,
                m.outside,
                m.overlap,
                mm(m.narrowest_mm),
                mm(m.widest_mm)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_set_has_each_character_one_time() {
        let c = characters();
        assert_eq!(c.len(), 80 + 57 + 80 + 26 + 5, "{}", c.len());
        assert_eq!(KANJI_FIRST_YEAR.chars().count(), 80);
        let mut sorted = c.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), c.len());
    }
}
