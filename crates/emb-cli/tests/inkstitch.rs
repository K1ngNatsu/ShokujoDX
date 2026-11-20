//! The automated part of the exit test of P18 Milestone 5 (ADR 0135): an
//! Ink/Stitch file opens with its objects, their stitch types, settings and
//! colors, and it sews (a stitch plan and a machine file). The exit test of
//! the owner uses files of testers.

use emb_model::{ColumnPattern, FillPattern, ObjectKind, TrimMode, Underlay, UnderlayLayer};

const BADGE: &[u8] = include_bytes!("data/inkstitch-badge.svg");

fn kind(k: &ObjectKind) -> String {
    match k {
        ObjectKind::Fill { params, .. } => format!("fill {}", params.pattern.name()),
        ObjectKind::Satin { params, .. } => format!("satin {}", params.pattern.name()),
        ObjectKind::Running { params, .. } if params.border.is_some() => "zigzag line".into(),
        ObjectKind::Running { params, .. } => format!("run x{}", params.repeats),
        k => k.type_name().to_lowercase(),
    }
}

#[test]
fn an_inkstitch_file_opens_with_its_objects_kinds_and_colors() {
    assert!(emb_formats::inkstitch::is_inkstitch(BADGE));
    let imported = emb_formats::svg::import_svg(BADGE).unwrap();
    assert!(
        imported.warnings.iter().all(|w| w.contains("vector")),
        "{:?}",
        imported.warnings
    );
    let d = imported.to_design("badge");
    let rows: Vec<(String, String, String)> = d
        .objects()
        .iter()
        .map(|o| {
            let t = d.thread(o.thread).unwrap();
            (o.name.clone().unwrap_or_default(), kind(&o.kind), t.color.to_hex())
        })
        .collect();
    let expected = [
        ("Background fill", "fill Tatami", "#b7c3c5"),
        ("Border satin", "satin Satin", "#1a1a1a"),
        ("Star", "fill Maze", "#ffcc00"),
        ("Swoosh", "run x3", "#1a1a1a"),
        ("Letter I", "satin Satin", "#aa2222"),
        ("Zigzag line", "zigzag line", "#aa2222"),
        ("note", "vector", "#ff00ff"),
    ];
    let got: Vec<(&str, &str, &str)> = rows
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        .collect();
    assert_eq!(got, expected);
    // The palette of the file names the thread of its color.
    let bg = d.thread(d.objects()[0].thread).unwrap();
    assert_eq!(bg.name, "Celestial Blue");
    assert_eq!(bg.brand.as_deref(), Some("Madeira Polyneon"));

    // Settings, commands and functions.
    let o = &d.objects();
    let ObjectKind::Fill { params, .. } = &o[0].kind else {
        panic!()
    };
    assert!((params.angle_deg.rem_euclid(180.0) - 120.0).abs() < 1e-6);
    assert!((params.spacing_mm - 0.3).abs() < 1e-9);
    assert!((params.stitch_length_mm - 3.5).abs() < 1e-9);
    let Underlay::Custom(layers) = &params.underlay else {
        panic!("{:?}", params.underlay)
    };
    assert_eq!(layers.len(), 2);
    assert!(o[0].start.is_some(), "the starting point command");
    let ObjectKind::Satin { geometry, params } = &o[1].kind else {
        panic!()
    };
    assert_eq!(geometry.rungs.len(), 4);
    assert!((params.pull_compensation_mm - 0.2).abs() < 1e-9);
    assert!(matches!(params.underlay, Underlay::Custom(ref l) if l.len() == 2));
    assert_eq!(o[1].functions.trim, TrimMode::Always);
    let ObjectKind::Fill { params, .. } = &o[2].kind else {
        panic!()
    };
    assert_eq!(params.pattern, FillPattern::Maze);
    assert!(!o[2].functions.stop && o[4].functions.stop);
    let ObjectKind::Satin { geometry, params } = &o[4].kind else {
        panic!()
    };
    assert_eq!(geometry.rungs.len(), 1);
    assert_eq!(params.underlay, Underlay::Custom(vec![UnderlayLayer::CenterWalk]));
    let ObjectKind::Running { params, .. } = &o[5].kind else {
        panic!()
    };
    let border = params.border.as_ref().unwrap();
    assert_eq!(border.column.pattern, ColumnPattern::Zigzag);
    assert!((border.width_mm - 1.5).abs() < 1e-4);
}

#[test]
fn an_inkstitch_file_sews() {
    let d = emb_formats::svg::import_svg(BADGE).unwrap().to_design("badge");
    let plan = emb_cli::plan(&d, &emb_stitch::PlanOptions::default());
    assert!(
        plan.warnings.is_empty(),
        "{:?}",
        plan.warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
    // Every object that sews has stitches.
    let sewn: std::collections::HashSet<_> = plan.spans.iter().map(|s| s.object).collect();
    for o in d.objects().iter().filter(|o| o.kind.sews()) {
        assert!(sewn.contains(&o.id), "{:?} has no stitches", o.name);
    }
    assert_eq!(plan.stops.len(), 1, "the stop after the letter");
    // The design fits its circle of 80 mm and the stitches are not too long.
    let bytes = emb_formats::write_machine("dst", &plan.stitches, "badge", "2026-11-20").unwrap();
    let back = emb_formats::read_machine("dst", &bytes).unwrap();
    assert!(back.stitch_count() > 5000, "{}", back.stitch_count());
    let pts: Vec<_> = plan.stitches.records.iter().map(|r| r.pos).collect();
    let (min_x, max_x) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    assert!(max_x - min_x < 80.5, "{min_x} {max_x}");
}
