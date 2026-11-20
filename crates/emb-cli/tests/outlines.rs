//! The automated part of the exit test of P18 Milestone 7 (ADR 0137): a
//! corpus design, saved as outlines (SVG) and as DST, becomes objects of
//! the same kinds as the source: each outline takes the kind and the
//! settings of the stitches inside it (`emb_art::part_outlines`).

use std::collections::HashMap;

use emb_art::part_objects::recognize;
use emb_art::part_outlines::{Outline, fit_by_size, outlines_bbox, split_areas, transformed, with_outlines};
use emb_cli::corpus;
use emb_geom::BBox;
use emb_model::{ColumnPattern, Design, FillPattern, ObjectKind, RunStyle, StitchKind};

/// The kind of a source object that its stitches show, for the objects
/// that the test measures: fills of rows, satin columns, runs, and lines
/// with a satin border.
fn source_kind(kind: &ObjectKind) -> Option<&'static str> {
    match kind {
        ObjectKind::Fill { params, .. } if matches!(params.pattern, FillPattern::Tatami | FillPattern::Square) => {
            Some("fill")
        }
        ObjectKind::Satin { params, .. } if params.pattern.is_satin() => Some("satin"),
        ObjectKind::Running { params, .. } => {
            if params.ripple.is_some() || params.motif.is_some() || params.cut {
                None
            } else if let Some(b) = &params.border {
                (b.column.pattern == ColumnPattern::Satin).then_some("satin")
            } else {
                (params.style == RunStyle::Run).then_some("run")
            }
        }
        _ => None,
    }
}

fn made_kind(kind: &ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Fill { params, .. } if matches!(params.pattern, FillPattern::Columns { .. }) => "satin",
        ObjectKind::Fill { .. } => "fill",
        ObjectKind::Satin { .. } => "satin",
        ObjectKind::Running { .. } => "run",
        _ => "other",
    }
}

/// The design with each lettering object as the objects of its letter
/// parts (so each part has an outline of its own), and the indexes of the
/// runs of letters (travel under the strokes: their stitches go with the
/// column over them, ADR 0131).
fn with_letter_parts(d: &Design) -> (Design, std::collections::HashSet<usize>) {
    let mut out = d.clone();
    let mut travel = std::collections::HashSet::new();
    let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
    for id in ids {
        let Some(o) = out.object(id).cloned() else { continue };
        let ObjectKind::Lettering(l) = &o.kind else { continue };
        let parts: Vec<ObjectKind> = l.parts().cloned().collect();
        let at = out.objects().iter().position(|x| x.id == id).unwrap_or(0);
        out.remove_object(id);
        for (k, p) in parts.into_iter().enumerate() {
            if matches!(p, ObjectKind::Running { .. }) {
                travel.insert(at + k);
            }
            out.insert_object(at + k, o.thread, p);
        }
    }
    (out, travel)
}

#[derive(Default)]
struct Count {
    measured: usize,
    same: usize,
    /// Fills of the same kind, and those with the angle of the source.
    fills: usize,
    fill_angle: usize,
    lines: Vec<String>,
}

fn check(name: &str, c: &mut Count) {
    let Some(d) = corpus::build(name) else { return };
    let (d, travel) = with_letter_parts(&d);
    // The outlines as Export Outlines writes them, read as vectors. The id
    // of each shape names its object.
    let Some(svg) = emb_render::svg::outlines(&d) else {
        return;
    };
    let ids: HashMap<String, usize> = d
        .objects()
        .iter()
        .enumerate()
        .filter(|(_, o)| emb_model::vector::to_vector(&o.kind, emb_model::Rgb(0, 0, 0)).is_some())
        .map(|(i, o)| (o.name.clone().unwrap_or_else(|| format!("object-{}", i + 1)), i))
        .collect();
    let read = emb_formats::svg::import_svg_vectors(svg.as_bytes()).expect("the outlines read");
    let outlines: Vec<Outline> = read
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Vector { path, style } => Some(Outline {
                path: path.clone(),
                filled: style.fill.is_some(),
                width_mm: style.stroke_mm,
                name: o.name.clone(),
            }),
            _ => None,
        })
        .collect();
    // The stitches as DST, read again and opened as stitch objects.
    let opts = emb_cli::plan_options(&d, None).expect("plan options");
    let plan = emb_cli::plan(&d, &opts);
    if plan.stitches.stitch_count() == 0 {
        return;
    }
    let bytes = emb_formats::write_machine("dst", &plan.stitches, name, "2026-11-20").expect("DST");
    let file = emb_formats::read_machine("dst", &bytes).expect("DST reads");
    let opened = emb_model::design_from_stitches(&file, name);
    let stitches_box = BBox::from_points(
        file.records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos),
    )
    .expect("stitches");
    // Aligned by size, over the outlines of the objects that sew.
    let sewing: Vec<Outline> = outlines
        .iter()
        .filter(|o| {
            o.name
                .as_ref()
                .and_then(|n| ids.get(n))
                .is_some_and(|&i| d.objects()[i].kind.sews())
        })
        .cloned()
        .collect();
    let Some(ob) = outlines_bbox(&sewing) else { return };
    let aligned = split_areas(&transformed(&sewing, &fit_by_size(ob, stitches_box)));
    // The kinds that the outline of each source object became.
    let mut became: HashMap<usize, Vec<ObjectKind>> = HashMap::new();
    for o in opened.objects() {
        let Some((s, made)) = recognize(&o.kind) else { continue };
        let (with, from) = with_outlines(&s, made, &aligned);
        for (m, j) in with.iter().zip(from) {
            let Some(&src) = j.and_then(|j| aligned[j].name.as_ref()).and_then(|n| ids.get(n)) else {
                continue;
            };
            became.entry(src).or_default().push(m.kind.clone());
        }
    }
    let t = d.output_transform();
    for (i, o) in d.objects().iter().enumerate() {
        let Some(want) = source_kind(&o.kind) else { continue };
        if travel.contains(&i) || !ids.values().any(|&v| v == i) {
            continue;
        }
        c.measured += 1;
        let got: Vec<&'static str> = became
            .get(&i)
            .map(|v| v.iter().map(made_kind).collect())
            .unwrap_or_default();
        if got.contains(&want) {
            c.same += 1;
            if let (ObjectKind::Fill { params: src, .. }, Some(ks)) = (&o.kind, became.get(&i)) {
                c.fills += 1;
                let (sin, cos) = emb_geom::math::sin_cos_deg(src.angle_deg);
                let v = t.apply_vector(emb_geom::Point::new(cos, sin));
                let src_angle = emb_geom::math::to_degrees(emb_geom::math::atan2(v.y, v.x));
                if ks.iter().any(|k| match k {
                    ObjectKind::Fill { params, .. } => {
                        let d = (params.angle_deg - src_angle).rem_euclid(180.0);
                        d < 5.0 || d > 175.0
                    }
                    _ => false,
                }) {
                    c.fill_angle += 1;
                }
            }
        } else {
            c.lines.push(format!("{name}: object {i} ({want}) became {got:?}"));
        }
    }
}

#[test]
fn outlines_and_dst_become_objects_of_the_kinds_of_their_source() {
    let mut c = Count::default();
    for name in corpus::NAMES {
        check(name, &mut c);
    }
    let part = c.same as f64 / c.measured.max(1) as f64;
    let angle = c.fill_angle as f64 / c.fills.max(1) as f64;
    println!(
        "{} of {} objects ({:.1} %) became objects of their kind; fills with the angle of the source: {:.1} %",
        c.same,
        c.measured,
        100.0 * part,
        100.0 * angle
    );
    if let Ok(w) = std::env::var("OUTLINES_DESIGN") {
        for l in c.lines.iter().filter(|l| l.starts_with(&w)).take(30) {
            println!("{l}");
        }
    }
    let mut per: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for l in &c.lines {
        let key = l.split(": ").next().unwrap_or("").to_string()
            + " "
            + l.split('(').nth(1).and_then(|x| x.split(')').next()).unwrap_or("");
        *per.entry(key).or_default() += 1;
    }
    let mut per: Vec<_> = per.into_iter().collect();
    per.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in per.iter().take(30) {
        println!("{n:5} {k}");
    }
    assert!(c.measured > 300, "{}", c.measured);
    assert!(part >= 0.9, "{:.3}", part);
    assert!(angle >= 0.9, "{:.3}", angle);
}
