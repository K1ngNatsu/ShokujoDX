//! The picture bench (P12 Milestone 1): the numbers of the Picture to
//! design steps on the test pictures of the program. A change of the
//! snapshot means the results of the wizard changed: say why in the commit
//! message (CLAUDE.md, rule 7). To accept changes:
//! `INSTA_UPDATE=always cargo test -p emb-cli --test bench`.

#[test]
fn picture_bench() {
    let results: Vec<_> = emb_cli::bench::pictures().iter().map(emb_cli::bench::run).collect();
    insta::assert_snapshot!(emb_cli::bench::report(&results, false));
}
