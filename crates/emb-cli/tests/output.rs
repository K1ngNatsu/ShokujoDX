//! Batch output with a fabric: the files, the sheet data and the saved project.

use emb_cli::{Input, OutputOptions, corpus, output, set_fabric};
use emb_model::{ObjectKind, Underlay, project};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("emb-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn output_with_a_fabric() {
    let dir = temp_dir("output");
    let name = corpus::NAMES[0];
    let input = dir.join(format!("{name}.embproj"));
    project::save(&corpus::build(name).unwrap(), &input).unwrap();
    let out_dir = dir.join("out");
    let o = OutputOptions {
        formats: vec!["vp3".into(), "dst".into()],
        worksheet: true,
        approval: false,
        picture: false,
        svg: true,
        paper: emb_render::page::Paper::A4,
        fabric: Some("towel".into()),
        barcode: emb_render::barcode::BarcodeKind::Code128,
        stone_template: true,
        stone_clearance_mm: emb_model::bling::DEFAULT_CLEARANCE_MM,
        flat_picture: false,
    };
    let (files, data) = output(&input, &out_dir, None, &o).unwrap();
    assert_eq!(files.len(), 5, "{files:?}");
    assert!(files.iter().all(|f| f.is_file()));
    let svg = std::fs::read_to_string(out_dir.join(format!("{name}.svg"))).unwrap();
    assert!(svg.starts_with("<?xml") && svg.contains("<polyline"));
    assert_eq!(data.fabric.as_deref(), Some("Towel (terry)"));
    assert!(data.topping.is_some());
    let saved = project::load(&out_dir.join(format!("{name}.embproj"))).unwrap();
    assert_eq!(saved.fabric.as_deref(), Some("towel"));
    let heavy = saved.objects().iter().any(|o| match &o.kind {
        ObjectKind::Satin { params, .. } => params.underlay == Underlay::Heavy,
        ObjectKind::Fill { params, .. } => params.underlay == Underlay::Heavy,
        _ => false,
    });
    assert!(heavy, "the satin and fill objects have the towel underlay");

    let bad = OutputOptions {
        fabric: Some("velvet".into()),
        ..o
    };
    let e = output(&input, &out_dir, None, &bad).unwrap_err();
    assert!(e.contains("unknown fabric") && e.contains("towel"), "{e}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fabric_on_a_machine_file_is_an_error() {
    let mut i = Input::Stitches(emb_model::StitchDesign::default());
    assert!(set_fabric(&mut i, "towel").is_err());
    assert!(emb_cli::fabric_list().contains("towel"));
}
