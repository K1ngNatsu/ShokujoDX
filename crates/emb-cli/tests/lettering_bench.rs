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
    // columns cover the characters. The levels of today (the exit test of
    // the plan is at most 3 % two layers and at least 95 % covered for each
    // character; 17 and 11 of the 248 characters are not there yet).
    let mean = |f: fn(&Measure) -> f64| b.rows.iter().map(f).sum::<f64>() / b.rows.len() as f64;
    assert!(mean(|m| m.overlap) < 0.02, "two layers: {}", mean(|m| m.overlap));
    assert!(mean(|m| m.coverage) > 0.97, "covered: {}", mean(|m| m.coverage));
    let bad: Vec<String> = b
        .rows
        .iter()
        .filter(|m| m.overlap > 0.06 || m.coverage < 0.93)
        .map(|m| format!("{} {} mm: {:.3} {:.3}", m.c, m.size_mm, m.overlap, m.coverage))
        .collect();
    assert!(bad.is_empty(), "two layers over 6 % or covered under 93 %: {bad:?}");
    insta::assert_snapshot!(lettering_bench::report(&b, false));
}
