use emb_model::{Accordion, LineBorder, Ripple, ThreadId, Underlay};

use super::*;
use crate::inkstitch::import_inkstitch;

/// The layers of `Custom` and `None`; the tests do not use the others.
fn layers(kind: &ObjectKind) -> Option<Vec<UnderlayLayer>> {
    let u = match kind {
        ObjectKind::Fill { params, .. } => &params.underlay,
        ObjectKind::Satin { params, .. } => &params.underlay,
        _ => return None,
    };
    Some(match u {
        emb_model::Underlay::Custom(l) => l.clone(),
        _ => Vec::new(),
    })
}

fn square(x: f64, y: f64, s: f64) -> Path {
    Path::polygon(&[
        Point::new(x, y),
        Point::new(x + s, y),
        Point::new(x + s, y + s),
        Point::new(x, y + s),
    ])
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-3
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

fn add(d: &mut Design, color: Rgb, kind: ObjectKind) -> emb_model::ObjectId {
    let t = d.thread_for_color(color);
    d.add_object(t, kind)
}

fn thread(d: &mut Design, color: Rgb) -> ThreadId {
    d.thread_for_color(color)
}

/// Writes and reads again, and moves the objects back to the design
/// coordinates (the writer puts the design on a page with a margin).
fn round_trip(d: &Design) -> (Written, crate::svg::SvgImport) {
    let w = write_inkstitch(d, &layers);
    let back = import_inkstitch(w.svg.as_bytes()).unwrap_or_else(|e| panic!("{e}\n{}", w.svg));
    (w, back)
}

#[test]
fn fills_come_back_with_their_type_angle_spacing_and_underlay() {
    let mut d = Design::new("fills");
    let underlay = Underlay::Custom(vec![
        UnderlayLayer::Tatami {
            inset_mm: 0.5,
            spacing_mm: 2.0,
            angle_deg: 90.0,
        },
        UnderlayLayer::Tatami {
            inset_mm: 0.5,
            spacing_mm: 2.0,
            angle_deg: 0.0,
        },
    ]);
    let fill = |pattern, angle, underlay: Underlay| ObjectKind::Fill {
        outer: square(0.0, 0.0, 20.0),
        holes: vec![square(5.0, 5.0, 5.0)],
        params: FillParams {
            angle_deg: angle,
            spacing_mm: 0.35,
            stitch_length_mm: 3.0,
            pattern,
            underlay,
            ..FillParams::default()
        },
    };
    add(&mut d, Rgb(200, 0, 0), fill(FillPattern::Tatami, 30.0, underlay));
    add(&mut d, Rgb(0, 200, 0), fill(FillPattern::Square, -20.0, Underlay::None));
    add(&mut d, Rgb(0, 0, 200), fill(FillPattern::Maze, 0.0, Underlay::None));
    add(
        &mut d,
        Rgb(0, 0, 200),
        fill(
            FillPattern::Circular {
                center: Some(Point::new(3.0, 4.0)),
            },
            0.0,
            Underlay::None,
        ),
    );
    add(
        &mut d,
        Rgb(0, 0, 200),
        fill(
            FillPattern::Guided {
                guides: vec![vec![Point::new(0.0, 2.0), Point::new(20.0, 8.0)]],
            },
            0.0,
            Underlay::None,
        ),
    );
    add(&mut d, Rgb(0, 0, 200), fill(FillPattern::String, 0.0, Underlay::None));
    let mut acc = fill(FillPattern::Tatami, 0.0, Underlay::None);
    if let ObjectKind::Fill { params, .. } = &mut acc {
        params.accordion = Some(Accordion {
            points: vec![(0.0, 1.0), (1.0, 3.0)],
        });
    }
    add(&mut d, Rgb(0, 0, 200), acc);
    let (w, back) = round_trip(&d);
    assert_eq!(back.objects.len(), 7, "{}", w.svg);
    let p = |k: usize| match &back.objects[k].kind {
        ObjectKind::Fill { params, holes, .. } => (params.clone(), holes.len()),
        k => panic!("{k:?}"),
    };
    let (a, holes) = p(0);
    assert_eq!(holes, 1);
    assert_eq!(back.objects[0].color, Rgb(200, 0, 0));
    assert!(close(a.angle_deg, 30.0) && close(a.spacing_mm, 0.35) && close(a.stitch_length_mm, 3.0));
    assert_eq!(
        a.underlay,
        Underlay::Custom(vec![
            UnderlayLayer::Tatami {
                inset_mm: 0.5,
                spacing_mm: 2.0,
                angle_deg: 90.0
            },
            UnderlayLayer::Tatami {
                inset_mm: 0.5,
                spacing_mm: 2.0,
                angle_deg: 0.0
            },
        ])
    );
    assert_eq!(p(1).0.pattern, FillPattern::Square);
    assert!(close(p(1).0.angle_deg, -20.0));
    assert_eq!(p(1).0.underlay, Underlay::None);
    assert_eq!(p(2).0.pattern, FillPattern::Maze);
    let FillPattern::Circular { center: Some(c) } = p(3).0.pattern else {
        panic!("{:?}", p(3).0.pattern)
    };
    // The page has a margin: the reader gives page coordinates.
    let shift = back.bbox().unwrap().min - Point::ZERO;
    assert!(near(c - shift, Point::new(3.0, 4.0)), "{c:?}");
    let FillPattern::Guided { guides } = p(4).0.pattern else {
        panic!("{:?}", p(4).0.pattern)
    };
    assert_eq!(guides.len(), 1);
    assert_eq!(p(5).0.pattern, FillPattern::Tatami, "String is tatami");
    let acc = p(6).0;
    assert!(close(acc.spacing_mm, 0.35));
    let pts = acc.accordion.unwrap().points;
    assert!(
        pts.len() == 2 && close(pts[0].1, 1.0) && close(pts[1].1, 3.0),
        "{pts:?}"
    );
    assert!(w.warnings.iter().any(|t| t.contains("String")), "{:?}", w.warnings);
}

#[test]
fn columns_come_back_with_their_rails_rungs_and_underlay() {
    let mut d = Design::new("columns");
    let rail = |x: f64| Path::new().move_to(Point::new(x, 0.0)).line_to(Point::new(x, 40.0));
    let column = |rungs: Vec<(Point, Point)>, pattern| ObjectKind::Satin {
        geometry: SatinGeometry {
            rail_a: rail(0.0),
            rail_b: rail(4.0),
            rungs,
        },
        params: SatinParams {
            spacing_mm: 0.45,
            pull_compensation_mm: 0.2,
            pull_b_mm: Some(0.3),
            split_mm: 6.0,
            pattern,
            underlay: Underlay::Custom(vec![
                UnderlayLayer::CenterWalk,
                UnderlayLayer::EdgeWalk { inset_mm: 0.3 },
                UnderlayLayer::Zigzag {
                    inset_mm: 0.2,
                    spacing_mm: 2.5,
                },
            ]),
            ..SatinParams::default()
        },
    };
    let r = |y: f64| (Point::new(0.0, y), Point::new(4.0, y));
    add(&mut d, Rgb(10, 20, 30), column(Vec::new(), ColumnPattern::Satin));
    add(
        &mut d,
        Rgb(10, 20, 30),
        column(vec![r(10.0), r(30.0)], ColumnPattern::EStitch),
    );
    add(
        &mut d,
        Rgb(10, 20, 30),
        column(vec![r(10.0), r(20.0), r(30.0)], ColumnPattern::Coil),
    );
    let (w, back) = round_trip(&d);
    assert_eq!(back.objects.len(), 3, "{}", w.svg);
    for (k, n) in [(0, 1), (1, 3), (2, 3)] {
        let ObjectKind::Satin { geometry, params } = &back.objects[k].kind else {
            panic!("{:?}", back.objects[k].kind)
        };
        assert_eq!(geometry.rungs.len(), n, "column {k}");
        let a: Vec<Point> = geometry.rail_a.points().collect();
        let b: Vec<Point> = geometry.rail_b.points().collect();
        assert!(
            close(b[0].x - a[0].x, 4.0) && close(a[0].y, b[0].y),
            "rail A first, the same way"
        );
        // The rungs touch the rails where they were.
        assert!(
            geometry
                .rungs
                .iter()
                .all(|(p, q)| close(q.x - p.x, 4.0) && close(p.y, q.y))
        );
        assert!(close(params.spacing_mm, 0.45) && close(params.split_mm, 6.0));
        assert!(close(params.pull_compensation_mm, 0.2));
        assert!(close(params.pull_b_mm.unwrap(), 0.3));
        assert_eq!(
            params.underlay,
            Underlay::Custom(vec![
                UnderlayLayer::CenterWalk,
                UnderlayLayer::EdgeWalk { inset_mm: 0.3 },
                UnderlayLayer::Zigzag {
                    inset_mm: 0.2,
                    spacing_mm: 2.5
                },
            ])
        );
    }
    let pattern = |k: usize| match &back.objects[k].kind {
        ObjectKind::Satin { params, .. } => params.pattern,
        _ => unreachable!(),
    };
    assert_eq!(pattern(1), ColumnPattern::EStitch);
    assert_eq!(pattern(2), ColumnPattern::Satin, "coil is satin");
    assert!(w.warnings.iter().any(|t| t.contains("Coil")), "{:?}", w.warnings);
}

#[test]
fn lines_come_back_as_runs_zigzags_columns_ripples_and_manual_stitches() {
    let mut d = Design::new("lines");
    let line = |y: f64| {
        Path::new()
            .move_to(Point::new(0.0, y))
            .line_to(Point::new(30.0, y + 5.0))
    };
    let run = |p: RunningParams, y: f64| ObjectKind::Running {
        path: line(y),
        params: p,
    };
    add(
        &mut d,
        Rgb(1, 1, 1),
        run(
            RunningParams {
                stitch_length_mm: 2.0,
                repeats: 3,
                pass: RunPass::Backtrack,
                ..RunningParams::default()
            },
            0.0,
        ),
    );
    add(
        &mut d,
        Rgb(1, 1, 1),
        run(
            RunningParams {
                border: Some(LineBorder::new(2.0, ColumnPattern::Zigzag)),
                ..RunningParams::default()
            },
            10.0,
        ),
    );
    add(
        &mut d,
        Rgb(1, 1, 1),
        run(
            RunningParams {
                border: Some(LineBorder::new(3.0, ColumnPattern::Satin)),
                ..RunningParams::default()
            },
            20.0,
        ),
    );
    add(
        &mut d,
        Rgb(1, 1, 1),
        run(
            RunningParams {
                style: RunStyle::Manual,
                ..RunningParams::default()
            },
            30.0,
        ),
    );
    add(
        &mut d,
        Rgb(1, 1, 1),
        ObjectKind::Running {
            path: square(0.0, 40.0, 20.0),
            params: RunningParams {
                ripple: Some(Ripple {
                    lines: 8,
                    target: Some(Point::new(10.0, 50.0)),
                    ..Ripple::default()
                }),
                ..RunningParams::default()
            },
        },
    );
    let (w, back) = round_trip(&d);
    assert_eq!(back.objects.len(), 5, "{}", w.svg);
    let rp = |k: usize| match &back.objects[k].kind {
        ObjectKind::Running { params, .. } => params.clone(),
        k => panic!("{k:?}"),
    };
    let a = rp(0);
    assert!(close(a.stitch_length_mm, 2.0));
    assert_eq!((a.repeats, a.pass), (3, RunPass::Backtrack));
    let b = rp(1).border.unwrap();
    assert_eq!(b.column.pattern, ColumnPattern::Zigzag);
    assert!(close(b.width_mm, 2.0));
    assert!(
        matches!(back.objects[2].kind, ObjectKind::Satin { .. }),
        "a satin line is a column"
    );
    assert_eq!(rp(3).style, RunStyle::Manual);
    assert_eq!(rp(4).ripple.unwrap().lines, 8);
    assert!(w.warnings.is_empty(), "{:?}", w.warnings);
}

#[test]
fn functions_points_stitches_vectors_and_gradients_come_back() {
    let mut d = Design::new("misc");
    let id = add(
        &mut d,
        Rgb(9, 9, 9),
        ObjectKind::Fill {
            outer: square(0.0, 0.0, 10.0),
            holes: Vec::new(),
            params: FillParams::default(),
        },
    );
    {
        let o = d.object_mut(id).unwrap();
        o.functions.trim = TrimMode::Always;
        o.functions.stop = true;
        o.functions.tie_off = false;
        o.start = Some(Point::new(0.0, 0.0));
        o.end = Some(Point::new(10.0, 10.0));
        o.name = Some("Leaf & stem".into());
    }
    add(
        &mut d,
        Rgb(9, 9, 9),
        ObjectKind::Stitches {
            path: Path::new()
                .move_to(Point::new(20.0, 0.0))
                .line_to(Point::new(22.0, 1.0))
                .move_to(Point::new(25.0, 0.0))
                .line_to(Point::new(27.0, 1.0)),
            trims: vec![1],
        },
    );
    add(
        &mut d,
        Rgb(9, 9, 9),
        ObjectKind::Vector {
            path: square(30.0, 0.0, 5.0),
            style: emb_model::VectorStyle {
                fill: Some(Rgb(0, 128, 0)),
                ..emb_model::VectorStyle::default()
            },
        },
    );
    // A gradient of two colors: two fills in a group.
    let base = FillParams::default();
    let layers = emb_model::tartan::gradient_layers(&base, None, &[0.0, 1.0]);
    let t1 = thread(&mut d, Rgb(255, 0, 0));
    let t2 = thread(&mut d, Rgb(0, 0, 255));
    for (p, t) in layers.into_iter().zip([t1, t2]) {
        let id = d.add_object(
            t,
            ObjectKind::Fill {
                outer: square(40.0, 0.0, 10.0),
                holes: Vec::new(),
                params: p,
            },
        );
        d.object_mut(id).unwrap().group = Some(7);
    }
    let (w, back) = round_trip(&d);
    let shift = back.bbox().unwrap().min - Point::ZERO;
    let o = &back.objects[0];
    assert_eq!(o.functions.trim, TrimMode::Always);
    assert!(o.functions.stop && o.functions.tie_in && !o.functions.tie_off);
    assert!(near(o.start.unwrap() - shift, Point::ZERO));
    assert!(near(o.end.unwrap() - shift, Point::new(10.0, 10.0)));
    assert_eq!(o.name.as_deref(), Some("Leaf & stem"));
    // The stitches: two paths of manual stitches, a trim between them.
    let manual: Vec<_> = back.objects[1..3].iter().map(|o| (&o.kind, o.functions.trim)).collect();
    assert!(
        manual
            .iter()
            .all(|(k, _)| matches!(k, ObjectKind::Running { params, .. } if params.style == RunStyle::Manual))
    );
    assert_eq!(manual[0].1, TrimMode::Always);
    let kinds: Vec<&str> = back.objects.iter().map(|o| o.kind.type_name()).collect();
    assert!(
        matches!(back.objects[3].kind, ObjectKind::Vector { .. }),
        "{kinds:?} {}",
        w.svg
    );
    // The gradient is one shape with two colors.
    assert_eq!(w.svg.matches("<linearGradient").count(), 1);
    assert_eq!(back.objects.len(), 6);
    assert_eq!(back.objects[4].color, Rgb(255, 0, 0));
    assert_eq!(back.objects[5].color, Rgb(0, 0, 255));
    assert!(back.objects[4].group.is_some() && back.objects[4].group == back.objects[5].group);
    // Applique and rhinestones are not written.
    assert!(w.warnings.is_empty(), "{:?}", w.warnings);
}

#[test]
fn numbers_are_short() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(0.25), "0.25");
    assert_eq!(num(-0.00001), "0");
    assert_eq!(num(12.345678), "12.3457");
}
