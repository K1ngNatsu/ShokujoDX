//! The lettering bench of Japanese text (P20 Milestone 1,
//! `docs/design/p20-plan.md`): the numbers of the reference set in the test
//! font (`tests/data/noto-sans-jp-bold-subset.ttf`, made by
//! `tools/make-test-font-jp.py`). A change of the snapshot means that the
//! lettering engine sews Japanese text another way: say why in the commit
//! message (CLAUDE.md, rule 7). To accept changes:
//! `INSTA_UPDATE=always cargo test -p emb-cli --test lettering_bench`.

use emb_cli::lettering_bench::{self, Measure};

fn font() -> emb_letter::Font {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/noto-sans-jp-bold-subset.ttf");
    emb_letter::Font::from_bytes(std::fs::read(path).expect("the test font"), 0).expect("a font")
}

#[test]
fn lettering_bench() {
    let f = font();
    assert_eq!((f.family.as_str(), f.style.as_str()), ("Noto Sans JP", "Bold"));
    let b = lettering_bench::run(&f);
    assert!(
        b.missing.is_empty(),
        "the test font has all characters: {:?}",
        b.missing
    );
    let n = lettering_bench::characters().len();
    assert_eq!(b.rows.len(), n * lettering_bench::SIZES.len());
    // The strokes of the font overlap (a variable font): the outline of each
    // character is one area, so the parts stay in it and a fill letter has
    // one layer (region::without_overlaps).
    let bad: Vec<&Measure> = b.rows.iter().filter(|m| m.outside > 0.05).collect();
    assert!(bad.is_empty(), "parts outside the shape: {bad:?}");
    for m in &b.rows {
        if m.kinds.satin + m.kinds.split + m.kinds.tatami_column + m.kinds.run == 0 {
            assert!(m.overlap < 1e-3, "a fill letter with two layers: {m:?}");
        }
    }
    // The exit test of P20 Milestone 2: the stitch of each column. No
    // character is all fill (tatami) from 15 mm to 80 mm, and each column
    // wider than 8 mm has split stitches or is a tatami column.
    let all_fill: Vec<String> = b
        .rows
        .iter()
        .filter(|m| m.kinds.fill > 0 && m.kinds.satin + m.kinds.split + m.kinds.tatami_column == 0)
        .map(|m| format!("{} {} mm", m.c, m.size_mm))
        .collect();
    assert!(all_fill.is_empty(), "all fill: {all_fill:?}");
    let unsplit: Vec<&Measure> = b.rows.iter().filter(|m| m.wide_unsplit > 0).collect();
    assert!(unsplit.is_empty(), "wide satin with no split stitches: {unsplit:?}");
    // P20 Milestone 3: crossings and junctions have one layer, and the
    // columns cover the characters to the corners of their ends. The exit
    // test of the plan: at least 95 % covered for each character (met), and
    // at most 3 % two layers (17 of the 248 characters are not there yet:
    // the levels of today).
    let mean = |f: fn(&Measure) -> f64| b.rows.iter().map(f).sum::<f64>() / b.rows.len() as f64;
    assert!(mean(|m| m.overlap) < 0.02, "two layers: {}", mean(|m| m.overlap));
    assert!(mean(|m| m.coverage) > 0.98, "covered: {}", mean(|m| m.coverage));
    // From 15 mm: at 10 mm the thin strokes are runs (Milestone 5), which
    // cover a line, not an area.
    let bad: Vec<String> = b
        .rows
        .iter()
        .filter(|m| m.overlap > 0.06 || (m.size_mm >= 15.0 && m.coverage < 0.95))
        .map(|m| format!("{} {} mm: {:.3} {:.3}", m.c, m.size_mm, m.overlap, m.coverage))
        .collect();
    assert!(bad.is_empty(), "two layers over 6 % or covered under 95 %: {bad:?}");
    // P20 Milestone 4: the letters have routes. The exit test: the trims
    // of 刺繍東京 at 20 mm are at most half of the trims with no routes
    // (48).
    let trims = lettering_bench::trims(&f, "刺繍東京", 20.0);
    assert!(trims <= 24, "刺繍東京 at 20 mm: {trims} trims");
    // P20 Milestone 5, the exit test: at 10 mm no satin column is narrower
    // than 0.8 mm (narrower columns are runs), and 刺繍 at a height of
    // 20 mm is 20 mm high (± 5 %).
    let narrow: Vec<&Measure> = b
        .rows
        .iter()
        .filter(|m| m.size_mm == 10.0 && m.narrowest_mm.is_some_and(|w| w < lettering_bench::NARROW_MM))
        .collect();
    assert!(narrow.is_empty(), "satin columns under 0.8 mm at 10 mm: {narrow:?}");
    let mut l = emb_model::Lettering::new(
        "刺繍",
        emb_letter::LetterSource::font_ref(&f),
        20.0,
        emb_geom::Point::ZERO,
    );
    emb_letter::relayout(&mut l, &f);
    let high = l
        .letters
        .iter()
        .flat_map(|x| x.parts.iter())
        .filter_map(|p| p.paths().filter_map(|q| q.control_bbox()).reduce(emb_geom::BBox::union))
        .reduce(emb_geom::BBox::union)
        .map_or(0.0, |b| b.height());
    assert!((high - 20.0).abs() <= 1.0, "刺繍 at 20 mm is {high:.2} mm high");
    insta::assert_snapshot!(lettering_bench::report(&b, false));
}
