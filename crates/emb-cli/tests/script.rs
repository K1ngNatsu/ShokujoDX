//! `emb-cli script` (ADR 0067): each sample script runs on each corpus
//! design, and a script with inputs changes a corpus design.

use std::collections::BTreeMap;

use emb_cli::{Input, corpus, run_script, script_input};
use emb_model::ObjectKind;
use emb_script::{InputValue, SAMPLES};

#[test]
fn each_sample_runs_on_the_corpus() {
    for name in corpus::NAMES {
        let design = corpus::build(name).unwrap();
        for (file, src) in SAMPLES {
            let input = Input::Design(Box::new(design.clone()));
            run_script(src, input, name, &BTreeMap::new(), None).unwrap_or_else(|e| panic!("{file} on {name}: {e}"));
        }
    }
}

#[test]
fn inputs_from_the_command_line() {
    assert_eq!(
        script_input("spacing=0.6").unwrap(),
        ("spacing".into(), InputValue::Number(0.6))
    );
    assert_eq!(script_input("on=true").unwrap().1, InputValue::Bool(true));
    assert_eq!(script_input("who=Ann").unwrap().1, InputValue::Text("Ann".into()));
    assert!(script_input("nothing").is_err());

    let design = corpus::build("satin-column").unwrap();
    let src = SAMPLES.iter().find(|(f, _)| *f == "satin-density.rhai").unwrap().1;
    let mut inputs = BTreeMap::new();
    inputs.insert("spacing".to_string(), InputValue::Number(0.7));
    let out = run_script(src, Input::Design(Box::new(design)), "satin-column", &inputs, None).unwrap();
    assert!(out.changed);
    assert!(out.design.objects().iter().any(|o| matches!(
        &o.kind,
        ObjectKind::Satin { params, .. } if params.spacing_mm == 0.7
    )));
}
