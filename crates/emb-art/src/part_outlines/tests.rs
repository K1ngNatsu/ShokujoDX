use emb_model::{Design, FillParams, RunningParams, SatinParams, StitchKind};

use super::*;
use crate::part_objects::objects;
use crate::parts::parts;

/// A fill (a square with a hole), a satin column and a run, in three colors.
fn design() -> Design {
    let mut d = Design::new("outlines");
    let square = |x: f64, y: f64, s: f64| {
        Path::polygon(&[
            Point::new(x, y),
            Point::new(x + s, y),
            Point::new(x + s, y + s),
            Point::new(x, y + s),
        ])
    };
    let red = d.add_thread("red", emb_model::Rgb(200, 0, 0));
    d.add_object(
        red,
        ObjectKind::Fill {
            outer: square(0.0, 0.0, 30.0),
            holes: vec![square(10.0, 10.0, 8.0)],
            params: FillParams {
                angle_deg: 20.0,
                spacing_mm: 0.4,
                ..FillParams::default()
            },
        },
    );
    let blue = d.add_thread("blue", emb_model::Rgb(0, 0, 200));
    let line = Path::new().move_to(Point::new(40.0, 0.0)).cubic_to(
        Point::new(50.0, 10.0),
        Point::new(40.0, 20.0),
        Point::new(50.0, 30.0),
    );
    d.add_object(
        blue,
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&line, 4.0),
            params: SatinParams::default(),
        },
    );
    let green = d.add_thread("green", emb_model::Rgb(0, 160, 0));
    d.add_object(
        green,
        ObjectKind::Running {
            path: Path::new()
                .move_to(Point::new(0.0, 40.0))
                .line_to(Point::new(50.0, 45.0)),
            params: RunningParams {
                stitch_length_mm: 2.0,
                ..RunningParams::default()
            },
        },
    );
    d
}

/// The stitches of the design, one stitch list for each color (as Recognize
/// Objects works on each stitch object).
fn colors(d: &Design) -> Vec<StitchDesign> {
    let plan = emb_stitch::build_plan(
        d,
        &emb_stitch::PlanOptions::default(),
        &mut emb_stitch::StitchCache::default(),
    );
    let mut out = vec![StitchDesign {
        threads: plan.stitches.threads.clone(),
        records: Vec::new(),
    }];
    for r in &plan.stitches.records {
        if r.kind == StitchKind::ColorChange {
            out.push(StitchDesign {
                threads: plan.stitches.threads.clone(),
                records: Vec::new(),
            });
        } else {
            out.last_mut().unwrap().records.push(*r);
        }
    }
    out.retain(|s| s.records.iter().any(|r| r.kind == StitchKind::Stitch));
    out
}

/// The outlines of the objects, as Export Outlines writes them.
fn outlines_of(d: &Design) -> Vec<Outline> {
    d.objects()
        .iter()
        .filter_map(|o| vector::to_vector(&o.kind, emb_model::Rgb(0, 0, 0)))
        .filter_map(|k| match k {
            ObjectKind::Vector { path, style } => Some(Outline {
                path,
                filled: style.fill.is_some(),
                width_mm: style.stroke_mm,
                name: None,
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn each_outline_takes_the_kind_and_the_settings_of_its_stitches() {
    let d = design();
    let outlines = split_areas(&outlines_of(&d));
    assert_eq!(outlines.len(), 3);
    let mut kinds = Vec::new();
    for s in colors(&d) {
        let made = objects(&s, &parts(&s));
        let total: usize = made.iter().map(Made::stitch_count).sum();
        let (with, from) = with_outlines(&s, made, &outlines);
        let used: std::collections::BTreeSet<usize> = from.iter().flatten().copied().collect();
        assert_eq!(used.len(), 1, "one outline for each color");
        // No stitch is lost.
        assert_eq!(with.iter().map(Made::stitch_count).sum::<usize>(), total);
        kinds.extend(with.into_iter().filter(Made::recognized).map(|m| m.kind));
    }
    assert_eq!(kinds.len(), 3, "{kinds:?}");
    // The fill has the shape of its outline (with the hole) and the angle
    // and spacing of its rows.
    let ObjectKind::Fill { outer, holes, params } = &kinds[0] else {
        panic!("{:?}", kinds[0])
    };
    assert_eq!(holes.len(), 1);
    let b = outer.control_bbox().unwrap();
    assert!(
        (b.width() - 30.0).abs() < 1e-6 && (b.height() - 30.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!((params.angle_deg - 20.0).rem_euclid(180.0) < 3.0 || (params.angle_deg - 20.0).rem_euclid(180.0) > 177.0);
    assert!((params.spacing_mm - 0.4).abs() < 0.04, "{}", params.spacing_mm);
    // The satin column has the sides of the outline and the pull
    // compensation that its stitches show.
    let ObjectKind::Satin { geometry, params } = &kinds[1] else {
        panic!("{:?}", kinds[1])
    };
    let w = width_of(geometry).unwrap();
    assert!((w - 4.0).abs() < 0.3, "{w}");
    let pull = SatinParams::default().pull_compensation_mm;
    assert!(
        (params.pull_compensation_mm - pull).abs() < 0.15,
        "{}",
        params.pull_compensation_mm
    );
    // The run is the line of the outline with the stitch length of the run.
    let ObjectKind::Running { path, params } = &kinds[2] else {
        panic!("{:?}", kinds[2])
    };
    assert_eq!(path.points().next(), Some(Point::new(0.0, 40.0)));
    assert!(
        (params.stitch_length_mm - 2.0).abs() < 0.2,
        "{}",
        params.stitch_length_mm
    );
}

#[test]
fn outlines_are_put_on_the_stitches_by_size_or_by_two_points() {
    let d = design();
    let outlines = outlines_of(&d);
    let b = outlines_bbox(&outlines).unwrap();
    // A file in other units, at another place.
    let moved = transformed(
        &outlines,
        &Transform::scale(3.78, 3.78).then(Transform::translate(100.0, -50.0)),
    );
    let mb = outlines_bbox(&moved).unwrap();
    let t = fit_by_size(mb, b);
    let back = outlines_bbox(&transformed(&moved, &t)).unwrap();
    assert!(back.min.distance(b.min) < 1e-6 && back.max.distance(b.max) < 1e-6);
    // The same size: only a move.
    let t = fit_by_size(b.expand(0.3), b);
    assert!(t.apply(Point::new(5.0, 5.0)).distance(Point::new(5.0, 5.0)) < 1e-9);
    // Two points: a turn, a scale and a move.
    let turn = Transform::rotate_deg(30.0)
        .then(Transform::scale(2.0, 2.0))
        .then(Transform::translate(7.0, 3.0));
    let (p, q) = (Point::new(1.0, 2.0), Point::new(10.0, -4.0));
    let fit = fit_two_points([turn.apply(p), turn.apply(q)], [p, q]).unwrap();
    let r = Point::new(-3.0, 8.0);
    assert!(fit.apply(turn.apply(r)).distance(r) < 1e-9);
    assert!(fit_two_points([p, p], [p, q]).is_none());
}

#[test]
fn a_detail_goes_to_the_smallest_outline_and_lone_stitches_stay() {
    let d = design();
    let mut outlines = outlines_of(&d);
    // A large box under all of it: it takes nothing that a smaller outline has.
    outlines.insert(
        0,
        Outline {
            path: Path::polygon(&[
                Point::new(-5.0, -5.0),
                Point::new(60.0, -5.0),
                Point::new(60.0, 60.0),
                Point::new(-5.0, 60.0),
            ]),
            filled: true,
            width_mm: 0.0,
            name: Some("box".into()),
        },
    );
    // No outline for the run: its stitches stay as they are made.
    outlines.pop();
    let outlines = split_areas(&outlines);
    let s = &colors(&d)[0];
    let (with, from) = with_outlines(s, objects(s, &parts(s)), &outlines);
    let used: Vec<usize> = from.iter().flatten().copied().collect();
    assert_eq!(used, vec![1], "the fill outline, not the box");
    assert!(with.iter().any(|m| matches!(m.kind, ObjectKind::Fill { .. })));
    let s = &colors(&d)[2];
    let made = objects(s, &parts(s));
    let n = made.len();
    let (with, from) = with_outlines(s, made, &outlines);
    // The run is in the box, but a box does not take run stitches.
    assert!(from.iter().all(Option::is_none), "{from:?}");
    assert_eq!(with.len(), n);
}
