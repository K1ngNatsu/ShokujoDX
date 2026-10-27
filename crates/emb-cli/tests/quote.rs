//! `emb-cli quote` (ADR 0064): the price and the run time of an order of a
//! corpus design.

use emb_cli::{Input, QuoteOptions, corpus, quote, quote_text};
use emb_model::quote::{Order, PriceList};

#[test]
fn quote_of_a_corpus_design() {
    let design = corpus::build("two-color-patch").unwrap();
    let o = QuoteOptions {
        list: PriceList::from_toml_str("per_1000 = 1.0\nsetup = 20\n[[breaks]]\nfrom = 10\npercent_off = 50\n")
            .unwrap(),
        order: Order {
            quantity: 12,
            ..Order::default()
        },
        currency: "$".into(),
        heads: Some(4),
    };
    let input = Input::Design(Box::new(design));
    let (q, run, s) = quote(&input, Some("tajima"), &o).unwrap();
    let k = s.stitch_count() as f64 / 1000.0;
    let each = emb_model::quote::cents(k);
    let half = emb_model::quote::cents(-each / 2.0);
    assert_eq!(q.total, emb_model::quote::cents(12.0 * each + 12.0 * half + 20.0));
    let run = run.expect("a machine gives a run time");
    assert_eq!((run.heads, run.cycles), (4, 3));
    let text = quote_text(&q, Some(&run), "$");
    assert!(text.contains("Embroidery") && text.contains("Quantity price") && text.contains("Setup"));
    assert!(text.contains("12 pieces"), "{text}");
    assert!(
        text.contains("run time:") && text.contains("on 4 heads, 3 cycles"),
        "{text}"
    );
}
