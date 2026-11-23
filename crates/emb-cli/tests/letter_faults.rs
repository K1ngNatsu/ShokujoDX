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
