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
    insta::assert_snapshot!(lettering_bench::report(&b, false));
}
