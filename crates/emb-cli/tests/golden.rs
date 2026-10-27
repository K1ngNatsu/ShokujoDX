//! Golden tests: the stitch plan summary of each corpus design.
//!
//! A change of a snapshot means the stitches changed. Do not accept a new
//! snapshot without a note in the commit message that says why (CLAUDE.md, rule 7).
//! To accept changes: `INSTA_UPDATE=always cargo test -p emb-cli --test golden`,
//! then review the diff of `tests/snapshots/`.

use emb_cli::{Input, corpus, stitches, summary};

fn golden(name: &str) {
    let design = corpus::build(name).unwrap();
    // VP3 has no jump record: the jumps to the auto start and end points
    // are not in the file (ADR 0032).
    let se = design.start_end;
    let (s, warnings) = stitches(&Input::Design(Box::new(design)), None).unwrap();
    // (No start jump when the start point is the first stitch.)
    let r = &s.records;
    let jump = emb_model::StitchKind::Jump;
    let lead = if se.start.is_some() {
        r.iter().take_while(|x| x.kind == jump).count().saturating_sub(1)
    } else {
        0
    };
    let tail = usize::from(se.end.is_some() && r.len() >= 2 && r[r.len() - 2].kind == jump);
    let lost = lead + tail;
    assert!(warnings.is_empty(), "{name}: {warnings:?}");
    // The VP3 file reads back with the same records.
    let back = emb_formats::vp3::read(&emb_formats::vp3::write(&s)).unwrap();
    assert_eq!(back.records.len() + lost, s.records.len(), "{name}");
    // U01 and PEC (P8 Milestone 6) read back with the same stitches and colors.
    for ext in ["u01", "pec"] {
        let bytes = emb_formats::write_machine(ext, &s, name, "20260101000000").unwrap();
        let back = emb_formats::read_machine(ext, &bytes).unwrap();
        assert_eq!(back.stitch_count(), s.stitch_count(), "{name}.{ext}");
        // PEC has no stop: a stop is a color change to the same thread (as in PES).
        let stops = if ext == "pec" {
            s.records
                .iter()
                .filter(|r| r.kind == emb_model::StitchKind::Stop)
                .count()
        } else {
            0
        };
        assert_eq!(back.color_count(), s.color_count() + stops, "{name}.{ext}");
    }
    insta::assert_snapshot!(name, summary(&s));
}

#[test]
fn circle_fill() {
    golden("circle-fill");
}

#[test]
fn satin_column() {
    golden("satin-column");
}

#[test]
fn s_curve() {
    golden("s-curve");
}

#[test]
fn fill_with_holes() {
    golden("fill-with-holes");
}

#[test]
fn two_color_patch() {
    golden("two-color-patch");
}

#[test]
fn full_field() {
    golden("full-field");
}

#[test]
fn applique_patch() {
    golden("applique-patch");
}

#[test]
fn lettering_names() {
    golden("lettering-names");
}

#[test]
fn embroidery_fonts() {
    golden("embroidery-fonts");
}

#[test]
fn embroidery_fonts_2() {
    golden("embroidery-fonts-2");
}

#[test]
fn embroidery_fonts_3() {
    golden("embroidery-fonts-3");
}

#[test]
fn embroidery_fonts_4() {
    golden("embroidery-fonts-4");
}

#[test]
fn lettering_circle() {
    golden("lettering-circle");
}

#[test]
fn lettering_bridge() {
    golden("lettering-bridge");
}

#[test]
fn monogram() {
    golden("monogram");
}

#[test]
fn monogram_templates() {
    golden("monogram-templates");
}

#[test]
fn team_names() {
    golden("team-names");
}

#[test]
fn keyboard_designs() {
    golden("keyboard-designs");
}

#[test]
fn custom_alphabet() {
    golden("custom-alphabet");
}

#[test]
fn sewout_towel() {
    golden("sewout-towel");
}

#[test]
fn sewout_cap() {
    golden("sewout-cap");
}

#[test]
fn outline_types() {
    golden("outline-types");
}

#[test]
fn curved_fills() {
    golden("curved-fills");
}

#[test]
fn texture_fills() {
    golden("texture-fills");
}

#[test]
fn motifs() {
    golden("motifs");
}

#[test]
fn patterns() {
    golden("patterns");
}

#[test]
fn effects() {
    golden("effects");
}

#[test]
fn blends() {
    golden("blends");
}

#[test]
fn buttonholes() {
    golden("buttonholes");
}

#[test]
fn borders() {
    golden("borders");
}

#[test]
fn vectors() {
    golden("vectors");
}

#[test]
fn shaping() {
    golden("shaping");
}

#[test]
fn arrangements() {
    golden("arrangements");
}

#[test]
fn border_trace() {
    golden("border-trace");
}

#[test]
fn autotrace() {
    golden("autotrace");
}

#[test]
fn smart_logo() {
    golden("smart-logo");
}

#[test]
fn photo() {
    golden("photo");
}

/// P7 Milestone 2: the 360 × 260 mm design in four hoopings of the
/// 260 × 200 hoop (200 mm left to right). Each hooping is a design of its own; all of its stitches
/// are inside the hoop field.
#[test]
fn multi_hoop() {
    let d = corpus::build("multi-hoop").unwrap();
    let hoop = emb_model::HoopCatalog::builtin().get("topaz20_crown").unwrap().clone();
    let outs = d.hoopings(&hoop);
    assert_eq!(outs.len(), 4);
    for (k, o) in outs.iter().enumerate() {
        assert!(o.warnings.is_empty(), "hooping {}: {:?}", k + 1, o.warnings);
        let (s, warnings) = stitches(&Input::Design(Box::new(o.design.clone())), None).unwrap();
        assert!(warnings.is_empty(), "hooping {}: {warnings:?}", k + 1);
        let out = s
            .records
            .iter()
            .filter(|r| r.kind == emb_model::StitchKind::Stitch)
            .find(|r| !hoop.contains(r.pos, 0.0));
        assert!(out.is_none(), "hooping {}: {out:?}", k + 1);
        insta::assert_snapshot!(format!("multi-hoop-{}", k + 1), summary(&s));
    }
}

/// P7 Milestone 4: the banner in the two positions of the test hoop. Each
/// position file has its stitches inside the 160 × 150 field of the position.
#[test]
fn multi_position() {
    let d = corpus::build("multi-position").unwrap();
    let hoop = corpus::test_position_hoop();
    let outs = d.hoopings(&hoop);
    assert_eq!(outs.len(), 2);
    for (k, o) in outs.iter().enumerate() {
        assert!(o.warnings.is_empty(), "position {}: {:?}", k + 1, o.warnings);
        let (s, warnings) = stitches(&Input::Design(Box::new(o.design.clone())), None).unwrap();
        assert!(warnings.is_empty(), "position {}: {warnings:?}", k + 1);
        let field = hoop.position_preset(k).unwrap();
        let out = s
            .records
            .iter()
            .filter(|r| r.kind == emb_model::StitchKind::Stitch)
            .find(|r| !field.contains(r.pos, 0.0));
        assert!(out.is_none(), "position {}: {out:?}", k + 1);
        insta::assert_snapshot!(format!("multi-position-{}", k + 1), summary(&s));
    }
}

#[test]
fn repeats() {
    golden("repeats");
}

/// P7 Milestone 6: in the made repeats, each wave starts at the end of the
/// wave before it (the step is from the start point to the end point).
#[test]
fn repeats_join_end_to_start() {
    let d = corpus::build("repeats").unwrap();
    let waves: Vec<emb_geom::BBox> = d
        .objects()
        .iter()
        .filter(|o| matches!(o.kind, emb_model::ObjectKind::Running { .. }))
        .map(|o| o.control_bbox().unwrap())
        .collect();
    assert_eq!(waves.len(), 4);
    for w in waves.windows(2) {
        assert!((w[1].min.x - w[0].max.x).abs() < 1e-6, "{w:?}");
    }
}

#[test]
fn hoop_placement() {
    golden("hoop-placement");
}

/// P7 Milestone 1: the hoop turns −55° with the bar, so in the machine
/// file the bar goes from the front to the back (Y down is to the front);
/// the file starts with a jump to the front end of the design and ends at
/// the hoop center.
#[test]
fn a_turned_hoop_puts_the_bar_on_the_long_side() {
    let d = corpus::build("hoop-placement").unwrap();
    let (s, _) = stitches(&Input::Design(Box::new(d)), None).unwrap();
    let sewn = s.records.iter().filter(|r| r.kind == emb_model::StitchKind::Stitch);
    let e = emb_geom::BBox::from_points(sewn.map(|r| r.pos)).unwrap();
    assert!(e.height() > 235.0 && e.width() < 52.0, "{e:?}");
    let first = &s.records[0];
    assert_eq!(first.kind, emb_model::StitchKind::Jump);
    // The front end of the bar, as it sits in the hoop.
    assert!(
        first.pos.distance(emb_geom::Point::new(0.0, e.max.y)) < 10.0,
        "{:?}",
        first.pos
    );
    let last = s
        .records
        .iter()
        .rev()
        .find(|r| r.kind != emb_model::StitchKind::End)
        .unwrap();
    assert_eq!(last.pos, emb_geom::Point::ZERO);
}

/// P4 Milestone 4 exit test: the name in a bridge envelope sews with fewer
/// trims in closest-join order than in text order (the default).
#[test]
fn closest_join_saves_trims() {
    let trims = |order| {
        let d = corpus::lettering_bridge(order);
        stitches(&Input::Design(Box::new(d)), None).unwrap().0.trim_count()
    };
    let (text, closest) = (
        trims(emb_model::LetterOrder::Text),
        trims(emb_model::LetterOrder::Closest),
    );
    assert!(closest * 3 < text * 2, "closest {closest}, text {text}");
}

#[test]
fn cross_stitch() {
    golden("cross-stitch");
}

/// P9 Milestone 1: the needle goes in at the cell corners and centers of
/// the 2.5 mm grid (and the half points of the backstitch). Only the lock
/// stitches are off the grid.
#[test]
fn cross_stitch_sews_on_the_grid() {
    let d = corpus::build("cross-stitch").unwrap();
    let (s, _) = stitches(&Input::Design(Box::new(d)), None).unwrap();
    let sewn: Vec<emb_geom::Point> = s
        .records
        .iter()
        .filter(|r| r.kind == emb_model::StitchKind::Stitch)
        .map(|r| r.pos)
        .collect();
    // The grid corner is at (-16.25, -20.0); the half grid is 1.25 mm.
    let on = |p: &emb_geom::Point| {
        let (x, y) = ((p.x + 16.25) / 1.25, (p.y + 20.0) / 1.25);
        (x - x.round()).abs() < 0.02 && (y - y.round()).abs() < 0.02
    };
    let n = sewn.iter().filter(|p| on(p)).count();
    // The lock stitches: a few at each trim and color change.
    let breaks = s.trim_count() + 4;
    assert!(
        sewn.len() - n <= 6 * breaks,
        "{n} of {} stitches on the grid",
        sewn.len()
    );
    assert!(s.trim_count() <= 8, "{} trims", s.trim_count());
}

#[test]
fn cross_picture() {
    golden("cross-picture");
}

#[test]
fn rhinestones() {
    golden("rhinestones");
}

/// P9 Milestone 3: the stones of the corpus design: each layout has
/// stones, no stone overlaps another, and the stones sew nothing (the
/// machine file has the frame only).
#[test]
fn rhinestones_have_no_overlaps() {
    let d = corpus::build("rhinestones").unwrap();
    let stones = d.stones();
    let objects: Vec<emb_model::ObjectId> = d
        .objects()
        .iter()
        .filter(|o| matches!(o.kind, emb_model::ObjectKind::Bling(_)))
        .map(|o| o.id)
        .collect();
    assert_eq!(objects.len(), 5);
    for id in &objects {
        assert!(stones.iter().any(|s| s.object == *id), "{id:?}");
    }
    assert!(stones.iter().all(|s| !s.overlaps));
    for (i, a) in stones.iter().enumerate() {
        for b in &stones[i + 1..] {
            assert!(a.at.distance(b.at) >= a.radius + b.radius + emb_model::bling::MIN_CLEAR_MM - 1e-9);
        }
    }
    assert!(stones.len() > 150, "{}", stones.len());
}

/// P9 Milestone 5: the crosses that sew last are the registration marks
/// of the template: each template mark (design rotation) is a sewn cross
/// (output coordinates) of the same design point.
#[test]
fn rhinestone_template_marks_sew() {
    let d = corpus::build("rhinestones").unwrap();
    let centers = d.sewn_mark_centers().expect("the design has its marks");
    let t = d.stone_template(0.3).unwrap();
    let (rot, out) = (d.rotation_transform(), d.output_transform());
    let (s, _) = stitches(&Input::Design(Box::new(d.clone())), None).unwrap();
    let sewn: Vec<emb_geom::Point> = s
        .records
        .iter()
        .filter(|r| r.kind == emb_model::StitchKind::Stitch)
        .map(|r| r.pos)
        .collect();
    assert_eq!(t.marks.len(), 4);
    for (m, c) in t.marks.iter().zip(&centers) {
        assert!(m.distance(rot.apply(*c)) < 1e-9);
        // The ends of the arms are needle points.
        let a = emb_model::bling::MARK_ARM_MM;
        for q in [emb_geom::Point::new(c.x - a, c.y), emb_geom::Point::new(c.x, c.y + a)] {
            let q = out.apply(q);
            assert!(sewn.iter().any(|p| p.distance(q) < 0.06), "{q:?}");
        }
    }
    // The marks are the last color.
    assert_eq!(
        s.threads.last().map(|t| t.name.as_str()),
        Some(emb_model::bling::MARK_THREAD.0)
    );
}

#[test]
fn tartan_fill() {
    golden("tartan-fill");
}

#[test]
fn gradient_fill() {
    golden("gradient-fill");
}

#[test]
fn meander_fill() {
    golden("meander-fill");
}

#[test]
fn circular_fill() {
    golden("circular-fill");
}

#[test]
fn guided_fill() {
    golden("guided-fill");
}

#[test]
fn ripple_stitch() {
    golden("ripple-stitch");
}

#[test]
fn lace_ornament() {
    golden("lace-ornament");
}

#[test]
fn lace_edge() {
    golden("lace-edge");
}

#[test]
fn quilting() {
    golden("quilting");
}

#[test]
fn quilt_block() {
    golden("quilt-block");
}

#[test]
fn puff() {
    golden("puff");
}

#[test]
fn cap_order() {
    golden("cap-order");
}

#[test]
fn cutwork() {
    golden("cutwork");
}

#[test]
fn sketch_badge() {
    golden("sketch-badge");
}

/// P14 Milestone 5: the badge with larger diameters (90 to 120 mm, 74 to
/// 104 mm). The linked objects are made again with their settings and
/// threads, and the badge sews in the Topaz 20 hoop before and after.
#[test]
fn sketch_badge_after_a_change() {
    use emb_model::ObjectKind;
    use emb_sketch::Constraint;
    let hoop = emb_model::HoopCatalog::builtin().get("topaz20_crown").unwrap().clone();
    let sews_in_the_hoop = |d: &emb_model::Design| {
        let (s, warnings) = stitches(&Input::Design(Box::new(d.clone())), None).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let out = s
            .records
            .iter()
            .filter(|r| r.kind == emb_model::StitchKind::Stitch)
            .find(|r| !hoop.contains(r.pos, 0.0));
        assert!(out.is_none(), "{out:?}");
        s
    };
    let d = corpus::build("sketch-badge").unwrap();
    let before = sews_in_the_hoop(&d);
    let sk = d.objects().last().unwrap().id;
    let threads: Vec<_> = d.objects().iter().map(|o| (o.id, o.thread)).collect();
    let mut doc = emb_model::Document::new(d);
    let lost = doc.edit("Diameters", |d| {
        if let Some(ObjectKind::Sketch(s)) = d.object_mut(sk).map(|o| &mut o.kind) {
            s.edit(|s| {
                let dims: Vec<_> = s
                    .constraints
                    .iter()
                    .filter_map(|c| match c.constraint {
                        Constraint::Diameter { mm, .. } => Some((c.id, mm + 30.0)),
                        _ => None,
                    })
                    .collect();
                for (id, mm) in dims {
                    s.set_value(id, mm).unwrap();
                }
                assert!(s.solve().converged);
            });
        }
        d.refresh_sketch_links(sk)
    });
    assert_eq!(lost, 0);
    let d = doc.design();
    let after = sews_in_the_hoop(d);
    assert_eq!(
        d.objects().iter().map(|o| (o.id, o.thread)).collect::<Vec<_>>(),
        threads
    );
    let size = |s: &emb_model::StitchDesign| s.extents().map(|b| b.width()).unwrap_or(0.0);
    assert!(
        (size(&after) - size(&before) - 30.0).abs() < 1.0,
        "{} {}",
        size(&before),
        size(&after)
    );
    assert!(after.stitch_count() > before.stitch_count());
    // One undo: the badge as it was.
    doc.undo();
    assert_eq!(sews_in_the_hoop(doc.design()).stitch_count(), before.stitch_count());
}

/// P14 Milestone 6: a badge template with three parameters (the size, the
/// border and the hexagon) makes badges of three sizes; each sews in the
/// Topaz 20 hoop with its objects made again.
#[test]
fn sketch_badge_template_makes_three_sizes() {
    use emb_model::{ObjectKind, template};
    use emb_sketch::Constraint;
    let mut d = corpus::build("sketch-badge").unwrap();
    let sk = d.objects().last().unwrap().id;
    let Some(ObjectKind::Sketch(s)) = d.object_mut(sk).map(|o| &mut o.kind) else {
        panic!("the badge has a sketch")
    };
    s.edit(|s| {
        s.set_parameter("size", "90 mm").unwrap();
        s.set_parameter("border", "8 mm").unwrap();
        s.set_parameter("hex", "size / 5.625").unwrap();
        let dims: Vec<_> = s
            .constraints
            .iter()
            .filter_map(|c| match c.constraint {
                Constraint::Diameter { mm, .. } if mm > 80.0 => Some((c.id, "size")),
                Constraint::Diameter { .. } => Some((c.id, "size - 2 * border")),
                Constraint::Radius { .. } => Some((c.id, "hex")),
                _ => None,
            })
            .collect();
        assert_eq!(dims.len(), 3);
        for (id, e) in dims {
            s.set_dimension_expr(id, e).unwrap();
        }
        assert!(s.solve().converged);
    });
    // Saved as a template and opened again.
    let t = template::from_design(&d, "Badge", true);
    let t = emb_model::project::from_str(&emb_model::project::to_string(&t)).unwrap();
    let asks: Vec<String> = template::parameters(&t).into_iter().map(|p| p.name).collect();
    assert_eq!(asks, ["size", "border", "hex"]);
    let hoop = emb_model::HoopCatalog::builtin().get("topaz20_crown").unwrap().clone();
    let mut widths = Vec::new();
    for (size, border, hex) in [
        ("60 mm", "6 mm", "size / 6"),
        ("90 mm", "8 mm", "16 mm"),
        ("120", "10", "size / 5"),
    ] {
        let mut n = template::new_design(&t);
        for (name, value) in [("size", size), ("border", border), ("hex", hex)] {
            assert_eq!(n.set_sketch_parameter(sk, name, value), Ok(0), "{name} = {value}");
        }
        let (s, warnings) = stitches(&Input::Design(Box::new(n.clone())), None).unwrap();
        assert!(warnings.is_empty(), "{size}: {warnings:?}");
        let out = s
            .records
            .iter()
            .filter(|r| r.kind == emb_model::StitchKind::Stitch)
            .find(|r| !hoop.contains(r.pos, 0.0));
        assert!(out.is_none(), "{size}: {out:?}");
        widths.push(s.extents().unwrap().width());
    }
    for (w, size) in widths.iter().zip([60.0, 90.0, 120.0]) {
        assert!((w - size).abs() < 2.0, "{widths:?}");
    }
}

/// P14 Milestone 6: an SVG logo (a rounded frame, a ring, a shield with
/// arcs and a free curve) becomes a sketch that solves, with the
/// constraints that the program finds and no repeated ones.
#[test]
fn an_svg_logo_becomes_a_sketch_that_solves() {
    use emb_model::ObjectKind;
    use emb_sketch::Constraint;
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="80mm" viewBox="0 0 100 80">
  <rect x="5" y="5" width="90" height="70" rx="8" fill="#123a7a"/>
  <circle cx="30" cy="40" r="14" fill="none" stroke="#e0a020" stroke-width="2"/>
  <path d="M 60 20 L 85 20 L 85 40 A 12.5 12.5 0 0 1 60 40 Z" fill="#ffffff"/>
  <path d="M 15 70 C 30 55, 50 85, 70 65" fill="none" stroke="#ffffff"/>
</svg>"##;
    let imported = emb_formats::svg::import_svg_vectors(svg).unwrap();
    let paths: Vec<emb_geom::Path> = imported
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Vector { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 4);
    let mut s = emb_sketch::import::from_paths(&paths, emb_sketch::import::TOL);
    assert!(s.solve().converged);
    let a = s.analyze();
    assert!(a.conflicting.is_empty() && a.redundant.is_empty(), "{a:?}");
    let count = |f: fn(&Constraint) -> bool| s.constraints.iter().filter(|c| f(&c.constraint)).count();
    assert!(count(|c| matches!(c, Constraint::Horizontal { .. })) >= 3);
    assert!(count(|c| matches!(c, Constraint::Vertical { .. })) >= 3);
    assert!(
        count(|c| matches!(c, Constraint::Tangent { .. })) >= 8,
        "the corners of the frame"
    );
    let round = s.curves.iter().filter(|c| c.is_round()).count();
    assert!(round >= 6, "four corners, the ring and the arc of the shield: {round}");
    assert!(s.curves.iter().any(|c| c.is_spline()), "the free curve");
    // The frame, the ring and the shield are regions; a dimension changes
    // the sketch and it solves again.
    assert!(s.regions().len() >= 3, "{}", s.regions().len());
    let line = s.curves.iter().find(|c| c.is_line()).unwrap().id;
    let (id, _) = s
        .add_dimension(Constraint::Length { line, mm: 0.0 }.with_value(1.0))
        .unwrap();
    s.set_value(id, s.measure(&Constraint::Length { line, mm: 0.0 }).unwrap() + 5.0)
        .unwrap();
    assert!(s.solve().converged);
}
