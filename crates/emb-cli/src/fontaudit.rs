//! The audit of an embroidery font (P17 Milestone 5): measures of each
//! letter that show where a letter needs a correction (ADR 0112), and the
//! geometry of the letters for proof pictures.
//!
//! - **Coverage:** the part of the outline of the source font that the
//!   parts of the letter cover, the part of the letter that is outside the
//!   outline, and the part that two or more parts cover (too much thread).
//! - **Widths:** the narrowest and the widest satin column, which set the
//!   smallest and the largest size at which the letter sews well.
//! - **Trims:** the trims in the letter when it sews alone, at each size.

use emb_geom::{BBox, Point};
use emb_letter::grid::Grid;
use emb_letter::{EmbFont, Font, LetterSource};
use emb_model::{Design, Lettering, ObjectKind, Rgb};

/// The cell of the coverage grid, in mm at the design size of the font.
const CELL_MM: f64 = 0.1;
/// A satin column narrower than this does not sew well, in mm.
pub const NARROW_MM: f64 = 0.9;
/// A satin column wider than this has stitches that are too long, in mm.
pub const WIDE_MM: f64 = 9.0;

/// The measures of one letter.
#[derive(Debug, Clone, PartialEq)]
pub struct LetterAudit {
    pub letter: char,
    pub parts: usize,
    /// The parts that are fills.
    pub fills: usize,
    pub fixed: bool,
    /// The part of the outline that the letter covers (0 to 1). `None`
    /// with no source font or no outline.
    pub coverage: Option<f64>,
    /// The area of the letter outside the outline, as a part of the area
    /// of the outline.
    pub outside: Option<f64>,
    /// The part of the area of the letter that two or more parts cover.
    pub overlap: f64,
    /// The narrowest and the widest satin column (over their rungs), in mm
    /// at the design size. 0 with no column.
    pub min_width_mm: f64,
    pub max_width_mm: f64,
    /// The trims in the letter when it sews alone, at each size.
    pub trims: Vec<usize>,
    /// The stitches at the first size.
    pub stitches: usize,
}

fn ring(path: &emb_geom::Path) -> Vec<Vec<Point>> {
    path.flatten(0.05).into_iter().map(|pl| pl.points).collect()
}

/// The area of a part as rings (even-odd). A run has no area.
pub fn part_rings(part: &ObjectKind) -> Vec<Vec<Point>> {
    match part {
        ObjectKind::Satin { geometry, .. } => {
            let (a, b) = (geometry.rail_a.flatten(0.05), geometry.rail_b.flatten(0.05));
            match (a.first(), b.first()) {
                (Some(a), Some(b)) if a.closed && b.closed => vec![a.points.clone(), b.points.clone()],
                (Some(a), Some(b)) => vec![a.points.iter().chain(b.points.iter().rev()).copied().collect()],
                _ => Vec::new(),
            }
        }
        ObjectKind::Fill { outer, holes, .. } => ring(outer).into_iter().chain(holes.iter().flat_map(ring)).collect(),
        _ => Vec::new(),
    }
}

/// The widths of a satin column at its rungs (at its ends with no rungs).
fn widths(part: &ObjectKind) -> Vec<f64> {
    let ObjectKind::Satin { geometry, .. } = part else {
        return Vec::new();
    };
    let mut w: Vec<f64> = geometry.rungs.iter().map(|(a, b)| a.distance(*b)).collect();
    if w.is_empty() {
        let ends = |p: &emb_geom::Path| {
            let pts: Vec<Point> = p.points().collect();
            pts.first().copied().zip(pts.last().copied())
        };
        if let (Some((a0, a1)), Some((b0, b1))) = (ends(&geometry.rail_a), ends(&geometry.rail_b)) {
            w = vec![a0.distance(b0), a1.distance(b1)];
        }
    }
    w
}

pub use emb_letter::quality::outline;

/// The quality of a font in one line (ADR 0115).
pub fn quality_line(q: &emb_letter::quality::Quality) -> String {
    format!(
        "quality {}: cover {:.1} %, too narrow {:.1} %, too long {:.1} %, fill letters {:.1} %, outside {:.1} %, two \
         layers {:.1} %",
        q.score,
        q.coverage * 100.0,
        q.narrow * 100.0,
        q.long * 100.0,
        q.fills * 100.0,
        q.outside * 100.0,
        q.overlap * 100.0
    )
}

/// The quality of `font` (converted from `source`) and the letters with
/// the lowest scores.
pub fn quality_table(font: &EmbFont, source: &Font) -> String {
    let mut rows = emb_letter::quality::letters(font, source);
    let Some(q) = emb_letter::quality::of_letters(&rows) else {
        return String::new();
    };
    let mut out = quality_line(&q);
    let w = |part: f64| emb_letter::quality::width_at(font, part).unwrap_or(0.0);
    out.push_str(&format!(
        " | range {}-{} mm, widths at 25 mm: 5 % {:.2}, 10 % {:.2}, 50 % {:.2}, 90 % {:.2}, 95 % {:.2}",
        font.min_height_mm,
        font.column_max_mm(),
        w(0.05),
        w(0.1),
        w(0.5),
        w(0.9),
        w(0.95)
    ));
    out.push('\n');
    rows.sort_by(|a, b| a.1.score().total_cmp(&b.1.score()));
    for (c, l) in rows.iter().take(12).filter(|r| r.1.score() < 0.9) {
        out.push_str(&format!(
            "  {:<5} score {:>3.0}: cover {:5.1} %, too narrow {:5.1} %, too long {:5.1} %, outside {:5.1} %, two layers \
             {:5.1} %{}\n",
            format!("{c:?}"),
            l.score() * 100.0,
            l.coverage * 100.0,
            l.narrow * 100.0,
            l.long * 100.0,
            l.outside * 100.0,
            l.overlap * 100.0,
            if l.fill { ", fill" } else { "" }
        ));
    }
    out
}

/// The measures of each letter of `font` and of the fill letter of the
/// same character, one line for each (a tab between the values): the
/// character, `columns` or `fill` (what the letter is), then for the letter
/// and for its fill letter: score, coverage, outside, two layers, too
/// narrow, too long.
pub fn letters_table(font: &EmbFont, source: &Font) -> String {
    let mut out = String::new();
    for (c, q) in emb_letter::quality::letters(font, source) {
        let fill = emb_letter::quality::fill_letter(font, source, c)
            .and_then(|l| emb_letter::quality::letter_quality(font, &l, &outline(font, source, c)));
        let row = |q: &emb_letter::quality::LetterQuality| {
            format!(
                "{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
                q.score(),
                q.coverage,
                q.outside,
                q.overlap,
                q.narrow,
                q.long
            )
        };
        let is_fill = font.letters[&c]
            .parts
            .iter()
            .all(|p| !matches!(p, ObjectKind::Satin { .. }));
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            c,
            if is_fill { "fill" } else { "columns" },
            row(&q),
            fill.as_ref().map(row).unwrap_or_default()
        ));
    }
    out
}

/// The trims and the stitches of the letter `c` when it sews alone at the
/// capital height `size_mm`.
fn sewn(font: &EmbFont, c: char, size_mm: f64) -> (usize, usize) {
    let mut d = Design::new("audit");
    let thread = d.add_thread("Black", Rgb(0, 0, 0));
    let mut l = Lettering::new(&c.to_string(), font.font_ref(), size_mm, Point::ZERO);
    emb_letter::relayout(&mut l, font);
    d.add_object(thread, ObjectKind::Lettering(Box::new(l)));
    let Ok(opts) = crate::plan_options(&d, None) else {
        return (0, 0);
    };
    let plan = crate::plan(&d, &opts);
    // The plan ends with a trim; the other trims are in the letter.
    (
        plan.stitches.trim_count().saturating_sub(1),
        plan.stitches.stitch_count(),
    )
}

/// The text for [`text_trims`]: each small letter, with capitals and digits.
pub const SAMPLE_TEXT: &str = "The Quick Brown Fox Jumps over the lazy dog 0123456789";

/// The trims of `text` as one lettering object at the capital height
/// `size_mm`: the trims in the letters and between them.
pub fn text_trims(font: &EmbFont, text: &str, size_mm: f64) -> usize {
    let mut d = Design::new("audit");
    let thread = d.add_thread("Black", Rgb(0, 0, 0));
    let mut l = Lettering::new(text, font.font_ref(), size_mm, Point::ZERO);
    emb_letter::relayout(&mut l, font);
    d.add_object(thread, ObjectKind::Lettering(Box::new(l)));
    let Ok(opts) = crate::plan_options(&d, None) else {
        return 0;
    };
    crate::plan(&d, &opts).stitches.trim_count().saturating_sub(1)
}

/// The measures of the letter `c` of `font`. `source` is the font that
/// `font` was converted from; `sizes` are the capital heights for the trims.
pub fn audit_letter(font: &EmbFont, source: Option<&Font>, c: char, sizes: &[f64]) -> Option<LetterAudit> {
    let letter = font.letters.get(&c)?;
    let shape = source.map(|s| outline(font, s, c)).unwrap_or_default();
    let bounds = letter
        .parts
        .iter()
        .filter_map(|p| p.control_bbox())
        .chain(BBox::from_points(
            shape
                .iter()
                .flat_map(|(o, h)| o.iter().chain(h.iter().flatten()))
                .copied(),
        ))
        .reduce(BBox::union);
    let (mut coverage, mut outside, mut overlap) = (None, None, 0.0);
    if let Some(bounds) = bounds {
        let grid = Grid::new(bounds, CELL_MM, 2);
        let mut cover = vec![0u8; grid.w * grid.h];
        for part in &letter.parts {
            for (c, m) in cover.iter_mut().zip(emb_letter::tune::part_mask(&grid, part)) {
                *c += u8::from(m);
            }
        }
        let covered = cover.iter().filter(|c| **c > 0).count();
        let twice = cover.iter().filter(|c| **c > 1).count();
        overlap = twice as f64 / covered.max(1) as f64;
        if !shape.is_empty() {
            let glyph = emb_letter::tune::outline_mask(&grid, &shape);
            let area = glyph.iter().filter(|g| **g).count().max(1) as f64;
            let both = glyph.iter().zip(&cover).filter(|(g, c)| **g && **c > 0).count();
            coverage = Some(both as f64 / area);
            outside = Some((covered - both) as f64 / area);
        }
    }
    let all_widths: Vec<f64> = letter.parts.iter().flat_map(widths).collect();
    let sewn: Vec<(usize, usize)> = sizes.iter().map(|s| sewn(font, c, *s)).collect();
    Some(LetterAudit {
        letter: c,
        parts: letter.parts.len(),
        fills: letter
            .parts
            .iter()
            .filter(|p| matches!(p, ObjectKind::Fill { .. }))
            .count(),
        fixed: letter.fixed,
        coverage,
        outside,
        overlap,
        min_width_mm: if all_widths.is_empty() {
            0.0
        } else {
            all_widths.iter().copied().fold(f64::MAX, f64::min)
        },
        max_width_mm: all_widths.iter().copied().fold(0.0, f64::max),
        trims: sewn.iter().map(|s| s.0).collect(),
        stitches: sewn.first().map_or(0, |s| s.1),
    })
}

/// The measures of the letters `chars` of `font` that have parts.
pub fn audit(font: &EmbFont, source: Option<&Font>, chars: &[char], sizes: &[f64]) -> Vec<LetterAudit> {
    chars
        .iter()
        .filter(|c| font.letters.get(c).is_some_and(|l| !l.parts.is_empty()))
        .filter_map(|c| audit_letter(font, source, *c, sizes))
        .collect()
}

/// What needs a look in a letter, as short words (empty: nothing).
pub fn flags(a: &LetterAudit, font: &EmbFont) -> Vec<String> {
    let mut out = Vec::new();
    if a.fills > 0 {
        out.push("fill".to_string());
    }
    if a.coverage.is_some_and(|c| c < 0.93) {
        out.push("gaps".to_string());
    }
    if a.outside.is_some_and(|o| o > 0.06) {
        out.push("outside".to_string());
    }
    if a.overlap > 0.12 {
        out.push("overlap".to_string());
    }
    let (small, large) = (
        font.min_height_mm / font.cap_height_mm,
        font.column_max_mm() / font.cap_height_mm,
    );
    if a.min_width_mm > 0.0 && a.min_width_mm * small < NARROW_MM {
        out.push("narrow".to_string());
    }
    if a.max_width_mm * large > WIDE_MM {
        out.push("wide".to_string());
    }
    if a.trims.iter().any(|t| *t > 0) {
        out.push("trims".to_string());
    }
    out
}

/// The audit as a table: one line for each letter, then the sums.
pub fn table(font: &EmbFont, rows: &[LetterAudit], sizes: &[f64]) -> String {
    let pct = |v: Option<f64>| v.map_or("    -".to_string(), |v| format!("{:5.1}", v * 100.0));
    let mut out = format!(
        "{} ({}-{} mm): trims at {}\nletter parts fixed  cover outside overlap  width mm    trims  look at\n",
        font.family,
        font.min_height_mm,
        font.column_max_mm(),
        sizes.iter().map(|s| format!("{s} mm")).collect::<Vec<_>>().join(", "),
    );
    for a in rows {
        let trims: Vec<String> = a.trims.iter().map(|t| t.to_string()).collect();
        out.push_str(&format!(
            "{:<6} {:>5} {:>5}  {} {}   {}  {:4.2}-{:<5.2} {:<6} {}\n",
            format!("{:?}", a.letter),
            a.parts,
            if a.fixed { "yes" } else { "no" },
            pct(a.coverage),
            pct(a.outside),
            pct(Some(a.overlap)),
            a.min_width_mm,
            a.max_width_mm,
            trims.join("/"),
            flags(a, font).join(" "),
        ));
    }
    let n = rows.len().max(1) as f64;
    let mean = |f: &dyn Fn(&LetterAudit) -> Option<f64>| {
        let v: Vec<f64> = rows.iter().filter_map(f).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    };
    let sums: Vec<String> = (0..sizes.len())
        .map(|i| rows.iter().map(|a| a.trims[i]).sum::<usize>().to_string())
        .collect();
    out.push_str(&format!(
        "{} letters: cover {} %, outside {} %, overlap {} %, trims {} ({:.2} for each letter at the first size), stitches {}\n",
        rows.len(),
        pct(mean(&|a| a.coverage)).trim(),
        pct(mean(&|a| a.outside)).trim(),
        pct(mean(&|a| Some(a.overlap))).trim(),
        sums.join("/"),
        rows.iter().map(|a| a.trims.first().copied().unwrap_or(0)).sum::<usize>() as f64 / n,
        rows.iter().map(|a| a.stitches).sum::<usize>(),
    ));
    let words: Vec<String> = sizes
        .iter()
        .map(|s| text_trims(font, SAMPLE_TEXT, *s).to_string())
        .collect();
    out.push_str(&format!(
        "text of {} letters: trims {}\n",
        SAMPLE_TEXT.chars().filter(|c| !c.is_whitespace()).count(),
        words.join("/")
    ));
    out
}

/// The geometry of the letters for a proof picture, as JSON: for each
/// letter its outline, its parts (area, rungs, start and end) and its
/// measures.
pub fn proof(font: &EmbFont, source: Option<&Font>, rows: &[LetterAudit]) -> String {
    let pts = |r: &[Point]| -> Vec<[f64; 2]> { r.iter().map(|p| [round(p.x), round(p.y)]).collect() };
    let letters: Vec<serde_json::Value> = rows
        .iter()
        .filter_map(|a| {
            let letter = font.letters.get(&a.letter)?;
            let parts: Vec<serde_json::Value> = letter
                .parts
                .iter()
                .map(|p| {
                    let rungs: Vec<[[f64; 2]; 2]> = match p {
                        ObjectKind::Satin { geometry, .. } => geometry
                            .rungs
                            .iter()
                            .map(|(a, b)| [[round(a.x), round(a.y)], [round(b.x), round(b.y)]])
                            .collect(),
                        _ => Vec::new(),
                    };
                    let ends = match p {
                        ObjectKind::Satin { geometry, .. } => {
                            let e = |path: &emb_geom::Path| {
                                let v: Vec<Point> = path.points().collect();
                                v.first().copied().zip(v.last().copied())
                            };
                            e(&geometry.rail_a)
                                .zip(e(&geometry.rail_b))
                                .map(|((a0, a1), (b0, b1))| (a0.midpoint(b0), a1.midpoint(b1)))
                        }
                        _ => None,
                    };
                    serde_json::json!({
                        "kind": match p {
                            ObjectKind::Satin { .. } => "satin",
                            ObjectKind::Fill { .. } => "fill",
                            ObjectKind::Running { .. } => "run",
                            _ => "other",
                        },
                        "line": match p {
                            ObjectKind::Running { path, .. } => Some(pts(&path.points().collect::<Vec<_>>())),
                            _ => None,
                        },
                        "rings": part_rings(p).iter().map(|r| pts(r)).collect::<Vec<_>>(),
                        "rails": match p {
                            ObjectKind::Satin { geometry, .. } => {
                                let flat = |path: &emb_geom::Path| -> Vec<Point> {
                                    path.flatten(0.05).into_iter().flat_map(|pl| pl.points).collect()
                                };
                                Some([pts(&flat(&geometry.rail_a)), pts(&flat(&geometry.rail_b))])
                            }
                            _ => None,
                        },
                        "rungs": rungs,
                        "start": ends.map(|e| [round(e.0.x), round(e.0.y)]),
                        "end": ends.map(|e| [round(e.1.x), round(e.1.y)]),
                    })
                })
                .collect();
            Some(serde_json::json!({
                "letter": a.letter.to_string(),
                "advance": letter.advance_mm,
                "fixed": letter.fixed,
                "outline": source.map(|s| {
                    outline(font, s, a.letter)
                        .iter()
                        .flat_map(|(o, h)| std::iter::once(pts(o)).chain(h.iter().map(|r| pts(r))))
                        .collect::<Vec<_>>()
                }),
                "parts": parts,
                "coverage": a.coverage,
                "outside": a.outside,
                "overlap": a.overlap,
                "trims": a.trims,
                "flags": flags(a, font),
            }))
        })
        .collect();
    serde_json::json!({
        "family": font.family,
        "cap_height": font.cap_height_mm,
        "min": font.min_height_mm,
        "max": font.column_max_mm(),
        "letters": letters,
    })
    .to_string()
}

fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use emb_model::LetterStitch;

    use super::*;

    #[test]
    fn the_audit_of_a_converted_font() {
        let source = emb_letter::bundled::font("Lato", "Bold").unwrap();
        let chars: Vec<char> = "HIio".chars().collect();
        let font = emb_letter::embfont::convert(&source, "Block Sans", "", LetterStitch::Auto, &chars);
        let rows = audit(&font, Some(&source), &chars, &[10.0, 30.0]);
        assert_eq!(rows.len(), 4);
        let by = |c: char| rows.iter().find(|a| a.letter == c).unwrap();
        // "I" is one column that covers its outline, with no trim.
        let i = by('I');
        assert_eq!((i.parts, i.fills, i.fixed), (1, 0, false));
        assert!(i.coverage.unwrap() > 0.97 && i.outside.unwrap() < 0.05, "{i:?}");
        assert!(i.overlap < 0.01 && i.trims == [0, 0], "{i:?}");
        assert!(i.min_width_mm > 2.0 && i.max_width_mm < 6.0, "{i:?}");
        assert!(i.stitches > 20);
        // The dot of "i" is away from its stem: a trim at the large size.
        assert!(by('i').parts >= 2 && by('i').trims[1] >= 1, "{:?}", by('i'));
        // "o" is a ring.
        assert!(by('o').coverage.unwrap() > 0.9, "{:?}", by('o'));
        // With no source font there is no outline.
        let alone = audit(&font, None, &chars, &[10.0]);
        assert!(alone.iter().all(|a| a.coverage.is_none() && a.trims.len() == 1));
        let text = table(&font, &rows, &[10.0, 30.0]);
        assert!(text.contains("4 letters") && text.contains("'H'"), "{text}");
        let json: serde_json::Value = serde_json::from_str(&proof(&font, Some(&source), &rows)).unwrap();
        assert_eq!(json["letters"].as_array().unwrap().len(), 4);
        assert!(json["letters"][0]["parts"][0]["rings"][0].as_array().unwrap().len() >= 4);
    }

    #[test]
    fn the_table_of_the_letters_and_their_fill_letters() {
        let source = emb_letter::bundled::font("Lato", "Bold").unwrap();
        let chars: Vec<char> = "HI.".chars().collect();
        let font = emb_letter::embfont::convert(&source, "Block Sans", "", LetterStitch::Auto, &chars);
        let text = letters_table(&font, &source);
        // One line for each letter (not for the mark): the character, what
        // it is, six values of the letter and six of its fill letter.
        let lines: Vec<Vec<&str>> = text.lines().map(|l| l.split('\t').collect()).collect();
        assert_eq!(lines.len(), 2, "{text}");
        for l in &lines {
            assert_eq!((l.len(), l[1]), (14, "columns"), "{l:?}");
            assert!(l[2..].iter().all(|v| v.parse::<f64>().is_ok()), "{l:?}");
        }
        // "I" is a good column; its fill letter is too small in this font.
        let i = lines.iter().find(|l| l[0] == "I").unwrap();
        assert!(
            i[2].parse::<f64>().unwrap() > 0.9 && i[8].parse::<f64>().unwrap() == 0.0,
            "{i:?}"
        );
        let line = quality_line(&font.quality.unwrap());
        assert!(line.contains("outside") && line.contains("two layers"), "{line}");
    }
}
