use std::collections::BTreeMap;
use std::fmt::Write;

use emb_geom::{BBox, Path, Point};
use emb_model::{FillParams, RunningParams, SatinGeometry, SatinParams};

use super::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Two fills, a satin column and a line, in two threads (one with a code).
fn design() -> Design {
    let mut d = Design::new("Badge");
    let red = d.add_thread("Red", Rgb(200, 20, 30));
    let navy = d.add_thread("Navy", Rgb(0, 0, 80));
    let cw = d.colorway;
    let mut look = emb_model::ThreadLook::of(d.thread(red).unwrap());
    look.brand = Some("Madeira Polyneon".into());
    look.code = Some("1747".into());
    d.set_colorway_look(cw, red, look);
    for x in [0.0, 50.0] {
        d.add_object(
            red,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(p(x, 0.0), p(x + 30.0, 20.0))),
                holes: Vec::new(),
                params: FillParams::default(),
            },
        );
    }
    d.add_object(
        navy,
        ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a: Path::new().move_to(p(0.0, 30.0)).line_to(p(60.0, 30.0)),
                rail_b: Path::new().move_to(p(0.0, 34.0)).line_to(p(60.0, 34.0)),
                rungs: Vec::new(),
            },
            params: SatinParams::default(),
        },
    );
    d.add_object(
        navy,
        ObjectKind::Running {
            path: Path::new().move_to(p(0.0, 40.0)).line_to(p(60.0, 40.0)),
            params: RunningParams::default(),
        },
    );
    d
}

fn run_default(src: &str, d: &Design, sel: &[ObjectId]) -> Result<Outcome, ScriptError> {
    run(src, d, sel, &BTreeMap::new(), None, &Limits::default())
}

fn sample(name: &str) -> &'static str {
    SAMPLES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s).unwrap()
}

/// A short text of a design, for the snapshots.
fn summary(d: &Design) -> String {
    let mut out = String::new();
    for (i, o) in d.objects().iter().enumerate() {
        let t = d.thread(o.thread).map_or("?".into(), |t| t.name.clone());
        let extra = match &o.kind {
            ObjectKind::Satin { params, .. } => format!(" spacing {}", params.spacing_mm),
            _ => String::new(),
        };
        writeln!(out, "{}. {} ({t}){extra}", i + 1, kind_name(&o.kind)).unwrap();
    }
    for t in &d.threads {
        writeln!(out, "thread {}: {}", t.id.0, t.name).unwrap();
    }
    out
}

#[test]
fn the_header_gives_the_title_and_the_inputs() {
    let info = ScriptInfo::parse(sample("check-before-output.rhai"), "x");
    assert_eq!(info.title, "Check Before Output");
    assert!(info.description.starts_with("Checks the design"));
    assert_eq!(info.inputs.len(), 4);
    assert_eq!(info.inputs[0].name, "max_colors");
    assert_eq!(info.inputs[0].label, "Most colors");
    assert_eq!(info.inputs[0].default, InputValue::Number(12.0));
    let other = ScriptInfo::parse(
        "//! T\n//! input on: true \"Turn on\"\n//! input who: \"Ann\" \"Name\"\n//! input bad name: 1\nprint(1);",
        "x",
    );
    assert_eq!(other.inputs.len(), 2, "a bad input line is left out");
    assert_eq!(other.inputs[0].default, InputValue::Bool(true));
    assert_eq!(other.inputs[1].default, InputValue::Text("Ann".into()));
    assert_eq!(ScriptInfo::parse("print(1);", "file").title, "file");
}

#[test]
fn each_sample_compiles() {
    for (name, src) in SAMPLES {
        check(src).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!ScriptInfo::parse(src, name).title.is_empty());
    }
}

#[test]
fn sample_outline_fills() {
    let d = design();
    let out = run_default(sample("outline-fills.rhai"), &d, &[]).unwrap();
    assert!(out.changed);
    assert_eq!(out.messages, vec!["2 outlines added.".to_string()]);
    insta::assert_snapshot!(summary(&out.design), @r"
    1. fill (Red)
    2. running (Red)
    3. fill (Red)
    4. running (Red)
    5. satin (Navy) spacing 0.4
    6. running (Navy)
    thread 0: Red
    thread 1: Navy
    ");
    // With a selection, only the selected fill.
    let first = d.objects()[0].id;
    let out = run_default(sample("outline-fills.rhai"), &d, &[first]).unwrap();
    assert_eq!(out.design.objects().len(), 5);
}

#[test]
fn sample_satin_density() {
    let d = design();
    let mut inputs = BTreeMap::new();
    inputs.insert("spacing".to_string(), InputValue::Number(0.6));
    let out = run(sample("satin-density.rhai"), &d, &[], &inputs, None, &Limits::default()).unwrap();
    insta::assert_snapshot!(summary(&out.design), @r"
    1. fill (Red)
    2. fill (Red)
    3. satin (Navy) spacing 0.6
    4. running (Navy)
    thread 0: Red
    thread 1: Navy
    ");
    // A value out of range stops the script with its message.
    inputs.insert("spacing".to_string(), InputValue::Number(5.0));
    let e = run(sample("satin-density.rhai"), &d, &[], &inputs, None, &Limits::default()).unwrap_err();
    assert_eq!(e.message, "Use a spacing from 0.2 to 2.0 mm.");
    assert!(e.line.is_some());
}

#[test]
fn sample_name_threads_by_code() {
    let out = run_default(sample("name-threads-by-code.rhai"), &design(), &[]).unwrap();
    insta::assert_snapshot!(summary(&out.design), @r"
    1. fill (1747 Red)
    2. fill (1747 Red)
    3. satin (Navy) spacing 0.4
    4. running (Navy)
    thread 0: 1747 Red
    thread 1: Navy
    ");
    // A second run changes nothing.
    let again = run_default(sample("name-threads-by-code.rhai"), &out.design, &[]).unwrap();
    assert!(!again.changed);
    assert_eq!(again.messages, vec!["0 threads renamed.".to_string()]);
}

#[test]
fn sample_check_before_output() {
    let mut inputs = BTreeMap::new();
    inputs.insert("max_colors".to_string(), InputValue::Number(1.0));
    let out = run(
        sample("check-before-output.rhai"),
        &design(),
        &[],
        &inputs,
        None,
        &Limits::default(),
    )
    .unwrap();
    assert!(!out.changed, "the check changes nothing");
    assert!(
        out.messages.iter().any(|m| m == "2 colors: more than 1."),
        "{:?}",
        out.messages
    );
    assert_eq!(out.messages.last().map(String::as_str), Some("1 problems found."));
    let ok = run_default(sample("check-before-output.rhai"), &design(), &[]).unwrap();
    assert_eq!(ok.messages, vec!["No problems found.".to_string()]);
}

#[test]
fn settings_are_maps_and_whole_numbers_fit() {
    let d = design();
    let fill = d.objects()[0].id.0;
    let src = format!(
        "let s = settings({fill}); s.spacing_mm = 1; set_settings({fill}, s); print(settings({fill}).spacing_mm);"
    );
    let out = run_default(&src, &d, &[]).unwrap();
    assert_eq!(out.messages, vec!["1.0".to_string()]);
    let ObjectKind::Fill { params, .. } = &out.design.objects()[0].kind else {
        panic!("a fill")
    };
    assert_eq!(params.spacing_mm, 1.0);
    // A setting of the wrong kind is an error, and the design does not change.
    let bad = format!("let s = settings({fill}); s.spacing_mm = \"wide\"; set_settings({fill}, s);");
    let e = run_default(&bad, &d, &[]).unwrap_err();
    assert!(e.message.contains("do not fit a fill"), "{e}");
}

#[test]
fn transforms_threads_order_and_shaping() {
    let d = design();
    let ids: Vec<u64> = d.objects().iter().map(|o| o.id.0).collect();
    let src = format!(
        "move_objects([{a}], 10, 0.5); scale_objects([{b}], 2); rotate_objects([{b}], 90);
         let t = add_thread(\"Gold\", \"#d4af37\"); set_thread({c}, t); set_thread_code(t, \"Isacord\", \"0822\");
         move_to({d}, 0); select([{d}]);
         let w = shape([{a}, {b}], \"weld\"); print(w.len());",
        a = ids[0],
        b = ids[1],
        c = ids[2],
        d = ids[3]
    );
    let out = run_default(&src, &d, &[]).unwrap();
    let nd = &out.design;
    assert_eq!(nd.objects()[0].id.0, ids[3], "moved to the start");
    assert_eq!(out.selection, vec![ObjectId(ids[3])]);
    let gold = nd.threads.iter().find(|t| t.name == "Gold").unwrap();
    assert_eq!(gold.code.as_deref(), Some("0822"));
    assert_eq!(nd.object(ObjectId(ids[2])).unwrap().thread, gold.id);
    assert_eq!(
        out.messages,
        vec!["1".to_string()],
        "the two fills touch after the moves: one weld"
    );
}

#[test]
fn a_script_with_no_end_stops() {
    let limits = Limits {
        time: Duration::from_millis(200),
        ..Limits::default()
    };
    let e = run(
        "let x = 0; loop { x += 1; }",
        &design(),
        &[],
        &BTreeMap::new(),
        None,
        &limits,
    )
    .unwrap_err();
    assert!(
        e.message.contains("ran longer than") || e.message.contains("too many operations"),
        "{e}"
    );
    let few = Limits {
        operations: 1000,
        ..Limits::default()
    };
    let e = run(
        "let x = 0; loop { x += 1; }",
        &design(),
        &[],
        &BTreeMap::new(),
        None,
        &few,
    )
    .unwrap_err();
    assert!(e.message.contains("too many operations"), "{e}");
}

#[test]
fn a_script_cannot_reach_files_or_programs() {
    for src in [
        "let f = open_file(\"C:/Windows/win.ini\");",
        "let t = read_file(\"/etc/passwd\");",
        "import \"os\" as os;",
        "let x = eval(\"1\"); system(\"calc\");",
    ] {
        assert!(run_default(src, &design(), &[]).is_err(), "{src} must fail");
    }
}

#[test]
fn too_many_objects_and_errors_change_nothing() {
    let d = design();
    let id = d.objects()[0].id.0;
    let limits = Limits {
        max_objects: 10,
        ..Limits::default()
    };
    let src = format!("for i in 0..20 {{ add_outline([{id}], 1.0 + i); }}");
    let e = run(&src, &d, &[], &BTreeMap::new(), None, &limits).unwrap_err();
    assert!(e.message.contains("more than 10"), "{e}");
    let e = run_default("print(\"before\"); remove(9999);", &d, &[]).unwrap_err();
    assert_eq!(e.messages, vec!["before".to_string()]);
    assert!(e.message.contains("no such object"), "{e}");
    assert_eq!(e.line, Some(1));
    // A syntax error has its line.
    let e = check("let x = ;\n").unwrap_err();
    assert_eq!(e.line, Some(1));
}

#[test]
fn info_gives_the_numbers_of_the_stitch_plan() {
    let out = run_default(
        "let i = info(); print(`${i.colors} ${i.stitches > 0}`);",
        &design(),
        &[],
    )
    .unwrap();
    assert_eq!(out.messages, vec!["2 true".to_string()]);
}
