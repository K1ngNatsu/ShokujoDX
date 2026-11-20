//! A Japanese embroidery font (P20 Milestone 6, ADR 0146): the converter
//! with the Japanese characters of the test font of the lettering bench
//! (`tests/data/noto-sans-jp-bold-subset.ttf`, the reference set).

use emb_letter::{EmbFont, LetterSource};
use emb_model::LetterStitch;

#[test]
fn a_japanese_embroidery_font() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/noto-sans-jp-bold-subset.ttf");
    let font = emb_letter::Font::from_bytes(std::fs::read(path).expect("the test font"), 0).expect("a font");
    let chars = emb_letter::japanese::reference();
    let mut f = emb_letter::embfont::convert_with(&font, "Test JP", "", LetterStitch::Auto, &chars, false);
    assert!(f.has_cjk());
    // A letter for each character, and the space.
    assert_eq!(f.letters.len(), chars.len() + 1);
    // No fill letters: in Auto, wide columns are tatami columns, and the
    // font goes to 300 mm.
    assert_eq!(f.add_fill_letters(&font), 0);
    assert!(
        f.fill_letters.is_none() && f.max_height_mm >= 300.0,
        "{}",
        f.max_height_mm
    );
    // The score measures the Japanese characters (their two layers and
    // narrow columns).
    assert!(emb_letter::quality::measured_in(&f).contains(&'繍'));
    f.measure_quality(&font);
    let q = f.quality.expect("a score");
    assert!(
        q.score >= emb_letter::quality::LIMIT,
        "score {} {q:?} range {}-{}",
        q.score,
        f.min_height_mm,
        f.max_height_mm
    );
    assert!(q.overlap < 0.03, "two layers {}", q.overlap);
    // The height of Japanese text is the height of its characters.
    let h = LetterSource::character_height(&f).expect("the kanji");
    assert!(h > f.cap_height_mm, "{h} {}", f.cap_height_mm);
    // The font lists read the header of a large font only.
    let file = std::env::temp_dir().join(format!("emb-japanese-font-{}.embfont", std::process::id()));
    f.save(&file).expect("saved");
    let header = EmbFont::load_header(&file).expect("a header");
    let _ = std::fs::remove_file(&file);
    assert_eq!(header.family, "Test JP");
    assert_eq!(header.quality.map(|q| q.score), Some(q.score));
}
