//! The automated parts of the exit tests of P18 Milestones 5 and 6 (ADR
//! 0135, ADR 0136): an Ink/Stitch file opens with its objects, their stitch
//! types, settings and colors, and it sews (a stitch plan and a machine
//! file); each corpus design written for Ink/Stitch opens with the same
//! objects. The exit tests of the owner use files of testers and Ink/Stitch.

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

/// What the reader gives for each object that the writer writes: its kind
/// ("fill", "satin", "run", "vector") and its color, in sew order. A line
/// gives one object for each subpath; a line with a satin border is a
/// satin column; a gradient gives one fill for each color.
fn expected(d: &emb_model::Design) -> Vec<(&'static str, emb_model::Rgb)> {
    use emb_model::ColumnPattern;
    fn add(kind: &ObjectKind, color: emb_model::Rgb, out: &mut Vec<(&'static str, emb_model::Rgb)>) {
        match kind {
            ObjectKind::Fill { outer, holes, .. } => {
                let mut subs = outer.subpaths();
                subs.extend(holes.iter().cloned());
                let n = emb_geom::region::nest(&subs).len();
                out.extend(std::iter::repeat_n(("fill", color), n));
            }
            ObjectKind::Satin { .. } => out.push(("satin", color)),
            ObjectKind::Running { path, params } => {
                let satin = params.ripple.is_none()
                    && params.border.as_ref().is_some_and(|b| {
                        matches!(b.column.pattern, ColumnPattern::Satin | ColumnPattern::EStitch) && b.width_mm > 1.0
                    });
                let n = path.subpaths().iter().filter(|s| s.cmds.len() > 1).count();
                out.extend(std::iter::repeat_n((if satin { "satin" } else { "run" }, color), n));
            }
            ObjectKind::Stitches { path, .. } => {
                let n = path.subpaths().iter().filter(|s| s.cmds.len() > 1).count();
                out.extend(std::iter::repeat_n(("run", color), n));
            }
            ObjectKind::Lettering(l) => {
                for p in l.parts() {
                    add(p, color, out);
                }
            }
            // A vector object has its own colors.
            ObjectKind::Vector { style, .. } => {
                if let Some(c) = style.fill.or(style.stroke) {
                    out.push(("vector", c));
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for o in d.objects() {
        let color = d.thread(o.thread).map_or(emb_model::Rgb(0, 0, 0), |t| t.color);
        add(&o.kind, color, &mut out);
    }
    out
}

fn category(k: &ObjectKind) -> &'static str {
    match k {
        ObjectKind::Fill { .. } => "fill",
        ObjectKind::Satin { .. } => "satin",
        ObjectKind::Running { .. } => "run",
        ObjectKind::Vector { .. } => "vector",
        _ => "other",
    }
}

#[test]
fn corpus_designs_written_for_inkstitch_open_with_the_same_objects() {
    let mut bad = Vec::new();
    let mut total = 0;
    for name in emb_cli::corpus::NAMES {
        let d = emb_cli::corpus::build(name).expect("corpus design");
        let w = emb_formats::inkstitch::write::write_inkstitch(&d, &emb_stitch::object::underlay_layers);
        let back = match emb_formats::inkstitch::import_inkstitch(w.svg.as_bytes()) {
            Ok(b) => b,
            Err(e) => {
                bad.push(format!("{name}: {e}"));
                continue;
            }
        };
        let want = expected(&d);
        // The steps of appliqué objects are compared in their own test.
        let got: Vec<(&str, emb_model::Rgb)> = back
            .objects
            .iter()
            .filter(|o| {
                !o.name
                    .as_deref()
                    .is_some_and(|n| [" placement", " tack-down", " cover"].iter().any(|s| n.ends_with(s)))
            })
            .map(|o| (category(&o.kind), o.color))
            .collect();
        total += want.len();
        if want != got {
            let at = want
                .iter()
                .zip(&got)
                .position(|(a, b)| a != b)
                .unwrap_or(want.len().min(got.len()));
            bad.push(format!(
                "{name}: {} objects written, {} read; first difference at {at}: {:?} / {:?}",
                want.len(),
                got.len(),
                want.get(at),
                got.get(at)
            ));
        }
        // The reader has no warnings for a file of the writer (but the
        // tiles of meander fills, which Ink/Stitch names in its own way).
        if !back
            .warnings
            .iter()
            .all(|w| w.contains("vector") || w.contains("meander_pattern"))
        {
            bad.push(format!("{name}: {:?}", back.warnings));
        }
    }
    assert!(bad.is_empty(), "{} objects; {}", total, bad.join("\n"));
    assert!(total > 1000, "{total}");
}

#[test]
fn written_fills_and_columns_keep_their_settings() {
    for name in ["satin-column", "fill-with-holes", "curved-fills", "outline-types"] {
        let d = emb_cli::corpus::build(name).expect("corpus design");
        let w = emb_formats::inkstitch::write::write_inkstitch(&d, &emb_stitch::object::underlay_layers);
        let back = emb_formats::inkstitch::import_inkstitch(w.svg.as_bytes()).unwrap();
        let mut read = back.objects.iter();
        for o in d.objects() {
            match &o.kind {
                ObjectKind::Fill { params, .. } => {
                    let Some(ObjectKind::Fill { params: p, .. }) = read.next().map(|r| &r.kind) else {
                        panic!("{name}: a fill")
                    };
                    let turn = (params.angle_deg - p.angle_deg).rem_euclid(180.0);
                    assert!(
                        turn < 1e-3 || turn > 180.0 - 1e-3,
                        "{name}: {} {}",
                        params.angle_deg,
                        p.angle_deg
                    );
                    assert!((params.spacing_mm - p.spacing_mm).abs() < 1e-3, "{name}");
                }
                ObjectKind::Satin { params, .. } => {
                    let Some(ObjectKind::Satin { params: p, .. }) = read.next().map(|r| &r.kind) else {
                        panic!("{name}: a satin")
                    };
                    assert!((params.spacing_mm - p.spacing_mm).abs() < 1e-3, "{name}");
                    assert!(
                        (params.pull_compensation_mm - p.pull_compensation_mm).abs() < 1e-3,
                        "{name}"
                    );
                    // The underlay that the column sews.
                    let sewn = emb_stitch::object::underlay_layers(&o.kind).unwrap();
                    let back = emb_stitch::object::underlay_layers(&ObjectKind::Satin {
                        geometry: Default::default(),
                        params: p.clone(),
                    })
                    .unwrap();
                    assert_eq!(sewn.len(), back.len(), "{name}");
                }
                _ => break,
            }
        }
    }
}

#[test]
fn an_applique_is_written_as_placement_tack_down_and_cover() {
    let d = emb_cli::corpus::build("applique-patch").expect("corpus design");
    let w = emb_formats::inkstitch::write::write_inkstitch(&d, &emb_stitch::object::underlay_layers);
    let back = emb_formats::inkstitch::import_inkstitch(w.svg.as_bytes()).unwrap();
    let steps: Vec<(&str, bool, &str)> = back
        .objects
        .iter()
        .filter_map(|o| {
            let n = o.name.as_deref()?;
            let step = [" placement", " tack-down", " cover"]
                .into_iter()
                .find(|s| n.ends_with(s))?;
            Some((step, o.functions.stop, category(&o.kind)))
        })
        .collect();
    assert!(!steps.is_empty(), "{}", w.svg);
    let first = |s: &str| steps.iter().position(|x| x.0 == s).unwrap();
    assert!(first(" placement") < first(" tack-down") && first(" tack-down") < first(" cover"));
    // A stop after the placement lines and after the tack-down lines.
    let last = |s: &str| steps.iter().rposition(|x| x.0 == s).unwrap();
    assert!(steps[last(" placement")].1 && steps[last(" tack-down")].1, "{steps:?}");
    assert!(
        steps.iter().filter(|s| s.0 == " cover").all(|s| s.2 == "satin"),
        "{steps:?}"
    );
    let plan = emb_cli::plan(&back.to_design("a"), &emb_stitch::PlanOptions::default());
    assert!(plan.stops.len() >= 2, "{:?}", plan.stops);
}
