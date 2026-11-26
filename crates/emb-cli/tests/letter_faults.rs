//! The letter fault bench (phase P21, `docs/design/p21-plan.md`) as a
//! snapshot: the faults of the letters of `SNAPSHOT_CHARS` of the four fonts
//! with a source font in the program, as lettering in the source font
//! (the stroke finder of today), against their outlines. A change of the
//! stroke finder, the ends, the route or the stitch engine shows here.
//! Accept a change with `INSTA_UPDATE=always cargo test -p emb-cli --test
//! letter_faults` and say in the commit message why the numbers changed.

#[test]
fn letter_faults() {
    let chars: Vec<char> = emb_cli::letter_faults::SNAPSHOT_CHARS.chars().collect();
    let fonts = emb_cli::letter_faults::run_on(true, &chars);
    assert_eq!(fonts.len(), 4, "the four fonts");
    insta::assert_snapshot!(emb_cli::letter_faults::table(&fonts));
}

/// The measures against the eye (P22 Milestone 6, ADR 0173): on the letters
/// of `SNAPSHOT_CHARS` of the judged set, both ways, the measures agree
/// with the eye on 90 % or more.
#[test]
fn the_measures_agree_with_the_eye() {
    let judged = emb_cli::letter_faults::load_judged(include_str!("data/judged-letters.json")).expect("the judged set");
    let chars: Vec<char> = emb_cli::letter_faults::SNAPSHOT_CHARS.chars().collect();
    let fonts = emb_cli::letter_faults::run_on(false, &chars);
    let c = emb_cli::letter_faults::compare(&fonts, &judged);
    assert!(c.total >= 8 * chars.len() - 10, "{}", c.text);
    assert!(c.agree * 10 >= c.total * 9, "{}", c.text);
}

/// All the letters of the judged set (release: `cargo test --release -p
/// emb-cli --test letter_faults -- --ignored`).
#[test]
#[ignore]
fn the_measures_agree_with_the_eye_on_all_letters() {
    let judged = emb_cli::letter_faults::load_judged(include_str!("data/judged-letters.json")).expect("the judged set");
    let c = emb_cli::letter_faults::compare(&emb_cli::letter_faults::run(), &judged);
    println!("{}", c.text);
    assert_eq!(c.total, judged.len());
    assert!(c.agree * 10 >= c.total * 9, "{}", c.text);
}
