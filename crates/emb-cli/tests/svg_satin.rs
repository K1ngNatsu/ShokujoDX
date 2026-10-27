//! Satin from SVG shapes, from the file to the stitch plan (owner report,
//! 2026-10-24: satin of an SVG made heavy overlaps that stress the machine).
//!
//! Each shape is sewn in two ways: imported as vectors and converted to
//! satin, and imported as fills with the satin stitch type. The test counts
//! how many stitches cross each 0.5 mm cell. Where stitches pile up, a few
//! cells have many times the usual count; an even satin has none.

use std::collections::{HashMap, HashSet};

use emb_model::{ColumnPattern, ConvertTo, Design, FillPattern, ObjectKind, StitchKind};

const SHAPES: &[(&str, &str)] = &[
    (
        "crescent",
        r#"<path d="M 74 32 A 30 30 0 1 0 74 68 L 66 62 A 20 20 0 1 1 66 38 Z"/>"#,
    ),
    (
        "ring",
        r#"<path fill-rule="evenodd" d="M 80 50 A 30 30 0 1 0 20 50 A 30 30 0 1 0 80 50 Z M 68 50 A 18 18 0 1 1 32 50 A 18 18 0 1 1 68 50 Z"/>"#,
    ),
    (
        "comb",
        r#"<path d="M 20 15 L 80 15 L 80 30 L 40 30 L 40 42 L 70 42 L 70 57 L 40 57 L 40 70 L 80 70 L 80 85 L 20 85 Z"/>"#,
    ),
    (
        "swoosh",
        r#"<path d="M 10 80 C 40 40 70 30 95 20 C 70 45 45 62 10 80 Z"/>"#,
    ),
    (
        "star",
        r#"<path d="M 50 10 L 61 38 L 91 38 L 67 56 L 76 86 L 50 68 L 24 86 L 33 56 L 9 38 L 39 38 Z"/>"#,
    ),
];

fn svg(body: &str) -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100" fill="#204080">{body}</svg>"##
    )
    .into_bytes()
}

/// Imported as vectors, then Convert > Satin.
fn converted(body: &str) -> Design {
    let vectors = emb_formats::svg::import_svg_vectors(&svg(body)).unwrap().to_design("v");
    let mut d = Design::new("satin");
    for o in vectors.objects() {
        if let ObjectKind::Vector { path, .. } = &o.kind {
            for k in emb_model::vector::from_vector(path, ConvertTo::Satin, &vectors.current, 4.0) {
                d.add_object(o.thread, k);
            }
        }
    }
    d
}

/// Imported as fills, with the satin stitch type.
fn satin_fill(body: &str) -> Design {
    let mut d = emb_formats::svg::import_svg(&svg(body)).unwrap().to_design("f");
    let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
    for id in ids {
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
            *params = params.with_pattern(FillPattern::Columns {
                column: ColumnPattern::Satin,
            });
        }
    }
    d
}

/// The median and the 99th percentile of the stitches that cross each
/// 0.5 mm cell (cells with stitches only), and the number of trims.
fn coverage(d: &Design) -> (u32, u32, usize) {
    let plan = emb_cli::plan(d, &emb_stitch::PlanOptions::default());
    let cell = 0.5;
    let mut grid: HashMap<(i64, i64), u32> = HashMap::new();
    let mut prev = None;
    for r in &plan.stitches.records {
        match r.kind {
            StitchKind::Stitch => {
                if let Some(p) = prev {
                    let n = (r.pos.distance(p) / 0.1).ceil().max(1.0) as usize;
                    let cells: HashSet<(i64, i64)> = (0..=n)
                        .map(|i| {
                            let q = p.lerp(r.pos, i as f64 / n as f64);
                            ((q.x / cell).floor() as i64, (q.y / cell).floor() as i64)
                        })
                        .collect();
                    for c in cells {
                        *grid.entry(c).or_default() += 1;
                    }
                }
                prev = Some(r.pos);
            }
            StitchKind::Jump | StitchKind::Trim | StitchKind::ColorChange => prev = None,
            _ => {}
        }
    }
    let mut v: Vec<u32> = grid.into_values().collect();
    v.sort_unstable();
    let at = |f: f64| v[((v.len() - 1) as f64 * f) as usize];
    (at(0.5), at(0.99), plan.stitches.trim_count())
}

#[test]
fn satin_from_svg_has_no_pile_ups() {
    for (name, body) in SHAPES {
        for (how, d) in [("convert", converted(body)), ("fill", satin_fill(body))] {
            let (median, p99, trims) = coverage(&d);
            // Before the fix: up to 16 times the median (crescent, satin fill).
            assert!(p99 <= 3 * median, "{name} {how}: p99 {p99}, median {median}");
            // One part: no trims inside the shape.
            assert_eq!(trims, 1, "{name} {how}");
        }
    }
}

/// The places of the density map over the default limit (P11 Milestone 1).
fn build_up(d: &Design) -> Vec<emb_render::density::HotSpot> {
    let plan = emb_cli::plan(d, &emb_stitch::PlanOptions::default());
    emb_render::density::DensityMap::of(&plan.stitches)
        .unwrap()
        .hot_spots(emb_render::density::DEFAULT_LIMIT)
}

#[test]
fn the_density_map_shows_the_old_pile_up_and_none_now() {
    for (name, body) in SHAPES {
        for (how, d) in [("convert", converted(body)), ("fill", satin_fill(body))] {
            assert!(build_up(&d).is_empty(), "{name} {how}: {:?}", build_up(&d));
        }
    }
    // Before the fix, the hole of a ring became a second column on top of
    // the first one: the map shows it all round the ring.
    let mut d = converted(SHAPES[1].1);
    let first = d.objects()[0].clone();
    d.add_object(first.thread, first.kind.clone());
    let spots = build_up(&d);
    let area: f64 = spots.iter().map(|s| s.area_mm2).sum();
    assert!(area > 1000.0, "{spots:?}");
}

#[test]
fn a_ring_converts_to_one_column() {
    // Before, the hole became a second column on top of the first.
    let d = converted(SHAPES[1].1);
    assert_eq!(d.objects().len(), 1);
    assert!(matches!(d.objects()[0].kind, ObjectKind::Satin { .. }));
}
