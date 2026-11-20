use super::*;

const HEAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:inkstitch="http://inkstitch.org/namespace"
 width="100mm" height="100mm" viewBox="0 0 100 100">"#;

/// The symbols of the commands, as Ink/Stitch puts them in the file.
const DEFS: &str = r#"<defs>
 <symbol id="inkstitch_trim"><path d="M 0 0 L 2 2" style="stroke:#000000"/></symbol>
 <symbol id="inkstitch_stop"><path d="M 0 0 L 2 2" style="stroke:#000000"/></symbol>
 <symbol id="inkstitch_starting_point"><circle r="1" style="fill:#000000"/></symbol>
 <symbol id="inkstitch_ending_point"><circle r="1" style="fill:#000000"/></symbol>
 <symbol id="inkstitch_target_point"><circle r="1" style="fill:#000000"/></symbol>
 <symbol id="inkstitch_ignore_object"><circle r="1" style="fill:#000000"/></symbol>
 <symbol id="inkstitch_ignore_layer"><circle r="1" style="fill:#000000"/></symbol>
</defs>"#;

/// An SVG file of Ink/Stitch of the SVG version `version` (none: no
/// metadata) with `body` in a layer. One unit is one mm.
fn file(version: Option<u32>, body: &str) -> Vec<u8> {
    let meta = version
        .map(|v| format!("<metadata><inkstitch:inkstitch_svg_version>{v}</inkstitch:inkstitch_svg_version></metadata>"))
        .unwrap_or_default();
    format!("{HEAD}{meta}{DEFS}<g inkscape:groupmode=\"layer\" id=\"layer1\">{body}</g></svg>").into_bytes()
}

fn read(version: Option<u32>, body: &str) -> SvgImport {
    import_inkstitch(&file(version, body)).unwrap()
}

/// A command on `target`: the symbol at (x, y) and its connector.
fn command(cmd: &str, n: u32, target: &str, x: f64, y: f64) -> String {
    format!(
        r##"<g id="command_group{n}"><path id="connector{n}" d="M {x},{y} 50,50" style="fill:none;stroke:#000000"
 inkscape:connector-type="polyline" inkscape:connection-start="#use{n}" inkscape:connection-end="#{target}"/>
 <use id="use{n}" xlink:href="#inkstitch_{cmd}" x="{x}" y="{y}" width="100%" height="100%"/></g>"##
    )
}

fn fill_params(o: &ImportedObject) -> &FillParams {
    match &o.kind {
        ObjectKind::Fill { params, .. } => params,
        k => panic!("not a fill: {k:?}"),
    }
}

/// The same within the precision of usvg (f32).
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-4
}

fn near_all(a: &[Point], b: &[Point]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| near(*p, *q))
}

#[test]
fn inkstitch_files_are_told_from_other_svg_files() {
    let plain = format!("{HEAD}<rect width=\"10\" height=\"10\"/></svg>");
    assert!(!is_inkstitch(plain.as_bytes()));
    assert!(is_inkstitch(&file(Some(4), "<rect width=\"10\" height=\"10\"/>")));
    let param = format!("{HEAD}<rect width=\"10\" height=\"10\" inkstitch:angle=\"30\"/></svg>");
    assert!(is_inkstitch(param.as_bytes()));
    let legacy = format!("{HEAD}<rect width=\"10\" height=\"10\" embroider_angle=\"30\"/></svg>");
    assert!(is_inkstitch(legacy.as_bytes()));
    // Import Artwork reads the settings.
    let i = crate::svg::import_svg(param.as_bytes()).unwrap();
    assert!(close(fill_params(&i.objects[0]).angle_deg, -30.0));
}

#[test]
fn a_fill_gets_its_angle_spacing_stitch_length_and_underlay() {
    let i = read(
        Some(4),
        r##"<rect id="a" x="10" y="10" width="30" height="20" style="fill:#ff0000"
 inkstitch:angle="30" inkstitch:row_spacing_mm="0.3" inkstitch:max_stitch_length_mm="3.5"
 inkstitch:fill_underlay_angle="0 90" inkstitch:fill_underlay_row_spacing_mm="2"
 inkstitch:fill_underlay_inset_mm="0.5" inkstitch:pull_compensation_mm="0.2"/>
 <rect id="b" x="50" y="10" width="30" height="20" style="fill:#0000ff"/>
 <rect id="c" x="10" y="50" width="30" height="20" style="fill:#00ff00" inkstitch:fill_underlay="false"
 inkstitch:staggers="1" inkstitch:end_row_spacing_mm="0.5"/>"##,
    );
    assert_eq!(i.objects.len(), 3, "{:?}", i.warnings);
    let a = fill_params(&i.objects[0]);
    assert_eq!(i.objects[0].color, Rgb(255, 0, 0));
    assert_eq!(i.objects[0].name.as_deref(), Some("a"));
    // Ink/Stitch turns counterclockwise on screen, the program clockwise.
    assert!(close(a.angle_deg, -30.0));
    assert!(close(a.spacing_mm, 0.3));
    assert!(close(a.stitch_length_mm, 3.5));
    assert!(close(a.pull_compensation_mm, 0.2));
    assert_eq!(a.pattern, FillPattern::Tatami);
    assert_eq!(
        a.underlay,
        Underlay::Custom(vec![
            UnderlayLayer::Tatami {
                inset_mm: 0.5,
                spacing_mm: 2.0,
                angle_deg: 30.0
            },
            UnderlayLayer::Tatami {
                inset_mm: 0.5,
                spacing_mm: 2.0,
                angle_deg: -60.0
            },
        ])
    );
    // The defaults of Ink/Stitch.
    let b = fill_params(&i.objects[1]);
    assert!(close(b.angle_deg, 0.0));
    assert!(close(b.spacing_mm, 0.25));
    assert!(close(b.stitch_length_mm, 4.0));
    assert_eq!(
        b.underlay,
        Underlay::Custom(vec![UnderlayLayer::Tatami {
            inset_mm: 0.0,
            spacing_mm: 0.75,
            angle_deg: -90.0
        }])
    );
    // One stagger is a square fill; the end spacing is an accordion.
    let c = fill_params(&i.objects[2]);
    assert_eq!(c.underlay, Underlay::None);
    assert_eq!(c.pattern, FillPattern::Square);
    assert!(close(c.spacing_mm, 0.25));
    assert_eq!(c.accordion.as_ref().unwrap().points, vec![(0.0, 1.0), (1.0, 2.0)]);
    assert!(i.warnings.is_empty(), "{:?}", i.warnings);
    // The fill is where the shape is (mm).
    let b = i.bbox().unwrap();
    assert!(close(b.min.x, 10.0) && close(b.max.y, 70.0), "{b:?}");
}

#[test]
fn the_fill_types_of_inkstitch_become_the_fill_types_of_the_program() {
    let i = read(
        Some(4),
        &format!(
            r##"<rect id="contour" width="20" height="20" inkstitch:fill_method="contour_fill" inkstitch:contour_strategy="1"/>
 <rect id="double" width="20" height="20" inkstitch:fill_method="contour_fill" inkstitch:contour_strategy="2"/>
 <rect id="circle" x="30" width="20" height="20" inkstitch:fill_method="circular_fill"/>
 {}
 <g id="guided"><rect id="gfill" y="30" width="20" height="20" inkstitch:fill_method="guided_fill"/>
 <path id="guide" d="M 0 35 C 10 30 10 45 20 40" style="fill:none;stroke:#000000;marker-start:url(#inkstitch-guide-line-marker)"/></g>
 <rect id="tartan" y="60" width="20" height="20" inkstitch:fill_method="tartan_fill"/>
 <rect id="cross" x="30" y="60" width="20" height="20" inkstitch:fill_method="cross_stitch" inkstitch:pattern_size_mm="2.5"
 inkstitch:cross_stitch_method="half_cross"/>"##,
            command("target_point", 1, "circle", 35.0, 5.0)
        ),
    );
    let kinds: Vec<FillPattern> = i.objects.iter().map(|o| fill_params(o).pattern.clone()).collect();
    assert_eq!(kinds.len(), 6, "the guide line is not an object: {kinds:?}");
    assert_eq!(kinds[0], FillPattern::Spiral);
    assert_eq!(kinds[1], FillPattern::Maze);
    let FillPattern::Circular { center: Some(c) } = kinds[2] else {
        panic!("{:?}", kinds[2])
    };
    assert!(near(c, Point::new(35.0, 5.0)));
    let FillPattern::Guided { guides } = &kinds[3] else {
        panic!("{:?}", kinds[3])
    };
    assert_eq!(guides.len(), 1);
    assert!(near(guides[0][0], Point::new(0.0, 35.0)));
    assert_eq!(kinds[4], FillPattern::Tatami);
    assert_eq!(
        kinds[5],
        FillPattern::CrossStitch {
            cell_mm: 2.5,
            style: CrossStyle::Diagonal,
            passes: 1
        }
    );
    assert!(i.warnings.iter().any(|w| w.contains("tartan_fill")), "{:?}", i.warnings);
}

#[test]
fn a_satin_column_gets_its_rails_rungs_and_underlay() {
    // Rails: down at x 10 and up at x 20 (reversed by Automatic); three rungs.
    let i = read(
        Some(4),
        r##"<path id="s" d="M 10 10 L 10 50 M 20 50 L 20 10 M 5 20 L 25 20 M 5 30 L 25 30 M 25 40 L 5 40"
 style="fill:none;stroke:#123456;stroke-width:0.5" inkstitch:satin_column="true"
 inkstitch:zigzag_spacing_mm="0.5" inkstitch:pull_compensation_mm="0.2 0.3" inkstitch:center_walk_underlay="true"
 inkstitch:contour_underlay="true" inkstitch:zigzag_underlay="true" inkstitch:zigzag_underlay_spacing_mm="2"
 inkstitch:max_stitch_length_mm="5"/>"##,
    );
    assert_eq!(i.objects.len(), 1);
    assert_eq!(i.objects[0].color, Rgb(0x12, 0x34, 0x56));
    let ObjectKind::Satin { geometry, params } = &i.objects[0].kind else {
        panic!("{:?}", i.objects[0].kind)
    };
    let a: Vec<Point> = geometry.rail_a.points().collect();
    let b: Vec<Point> = geometry.rail_b.points().collect();
    assert!(near(a[0], Point::new(10.0, 10.0)));
    assert!(near(b[0], Point::new(20.0, 10.0)), "rail B goes the same way as rail A");
    assert_eq!(geometry.rungs.len(), 3);
    assert!(near(geometry.rungs[0].0, Point::new(10.0, 20.0)));
    assert!(near(geometry.rungs[0].1, Point::new(20.0, 20.0)));
    // A rung drawn from right to left touches the rails at the same places.
    assert!(near(geometry.rungs[2].0, Point::new(10.0, 40.0)));
    assert!(close(params.spacing_mm, 0.5));
    assert!(close(params.pull_compensation_mm, 0.2));
    assert_eq!(params.pull_b_mm, Some(0.3));
    assert!(close(params.split_mm, 5.0));
    assert_eq!(params.pattern, ColumnPattern::Satin);
    assert_eq!(
        params.underlay,
        Underlay::Custom(vec![
            UnderlayLayer::CenterWalk,
            UnderlayLayer::EdgeWalk { inset_mm: 0.4 },
            UnderlayLayer::Zigzag {
                inset_mm: 0.2,
                spacing_mm: 2.0
            },
        ])
    );
}

#[test]
fn satin_columns_with_no_rungs_or_one_line() {
    let i = read(
        Some(4),
        r##"<path id="two" d="M 10 10 L 10 30 L 15 50 M 20 10 L 20 30 L 25 50" style="fill:none;stroke:#000000"
 inkstitch:satin_column="true" inkstitch:satin_method="e_stitch" inkstitch:swap_satin_rails="true"/>
 <path id="wide" d="M 40 10 L 40 50" style="fill:none;stroke:#000000;stroke-width:4" inkstitch:satin_column="true"/>
 <path id="thin" d="M 60 10 L 60 50" style="fill:none;stroke:#000000;stroke-width:0.5" inkstitch:satin_column="true"/>"##,
    );
    assert_eq!(i.objects.len(), 3);
    let ObjectKind::Satin { geometry, params } = &i.objects[0].kind else {
        panic!()
    };
    // The rails are swapped; the middle nodes pair up as a rung.
    assert!(near(geometry.rail_a.points().next().unwrap(), Point::new(20.0, 10.0)));
    assert_eq!(geometry.rungs.len(), 1);
    assert!(near(geometry.rungs[0].0, Point::new(20.0, 30.0)));
    assert!(near(geometry.rungs[0].1, Point::new(10.0, 30.0)));
    assert_eq!(params.pattern, ColumnPattern::EStitch);
    assert_eq!(params.underlay, Underlay::None);
    // A wide stroke is a column along the line.
    let ObjectKind::Satin { geometry, .. } = &i.objects[1].kind else {
        panic!()
    };
    let start = geometry.rail_a.points().next().unwrap();
    assert!(close(start.y, 10.0) && close((start.x - 40.0).abs(), 2.0), "{start:?}");
    // A thin stroke stays a line.
    assert!(matches!(i.objects[2].kind, ObjectKind::Running { .. }));
    assert!(i.warnings.iter().any(|w| w.contains("one line")), "{:?}", i.warnings);
}

#[test]
fn strokes_are_runs_beans_zigzags_ripples_and_manual_stitches() {
    let i = read(
        Some(4),
        &format!(
            r##"<path id="run" d="M 10 10 L 50 10" style="fill:none;stroke:#000000"
 inkstitch:running_stitch_length_mm="2" inkstitch:bean_stitch_repeats="1" inkstitch:repeats="2"/>
 <path id="zig" d="M 10 20 L 50 20" style="fill:none;stroke:#000000;stroke-width:2"
 inkstitch:stroke_method="zigzag_stitch" inkstitch:zigzag_spacing_mm="0.6" inkstitch:stroke_pull_compensation_mm="0.1"/>
 <path id="manual" d="M 10 30 C 20 25 30 35 40 30 L 50 30" style="fill:none;stroke:#000000"
 inkstitch:stroke_method="manual_stitch"/>
 <path id="ripple" d="M 60 60 L 90 60 L 90 90 L 60 90 Z" style="fill:none;stroke:#000000"
 inkstitch:stroke_method="ripple_stitch" inkstitch:line_count="7"/>
 <path id="sculpt" d="M 10 40 L 50 40" style="fill:none;stroke:#000000" inkstitch:bean_stitch_repeats="1 0"/>
 {}"##,
            command("target_point", 1, "ripple", 70.0, 70.0)
        ),
    );
    let run = |k: usize| match &i.objects[k].kind {
        ObjectKind::Running { path, params } => (path.clone(), params.clone()),
        k => panic!("{k:?}"),
    };
    let (_, p) = run(0);
    assert!(close(p.stitch_length_mm, 2.0));
    assert_eq!(p.repeats, 3);
    assert_eq!(p.pass, RunPass::Backtrack);
    let (_, p) = run(1);
    let border = p.border.unwrap();
    assert!(close(border.width_mm, 2.0));
    assert_eq!(border.column.pattern, ColumnPattern::Zigzag);
    assert!(close(border.column.spacing_mm, 0.6));
    assert!(close(border.column.pull_compensation_mm, 0.1));
    let (path, p) = run(2);
    assert_eq!(p.style, RunStyle::Manual);
    assert!(
        near_all(
            &path.points().collect::<Vec<_>>(),
            &[Point::new(10.0, 30.0), Point::new(40.0, 30.0), Point::new(50.0, 30.0)]
        ),
        "the nodes are the needle points: {path:?}"
    );
    let (_, p) = run(3);
    let r = p.ripple.unwrap();
    assert_eq!(r.lines, 7);
    assert!(near(r.target.unwrap(), Point::new(70.0, 70.0)));
    assert_eq!(run(4).1.style, RunStyle::Sculpture);
    assert!(i.warnings.is_empty(), "{:?}", i.warnings);
}

#[test]
fn files_of_the_first_versions_read_their_old_settings() {
    // No version: the settings of the first versions.
    let i = read(
        None,
        r##"<path id="zig" d="M 10 10 L 50 10" style="fill:none;stroke:#000000;stroke-width:2"/>
 <path id="dash" d="M 10 20 L 50 20" style="fill:none;stroke:#000000;stroke-dasharray:1,1" embroider_ties="False"/>
 <rect id="old" x="10" y="30" width="20" height="20" embroider_angle="45" embroider_row_spacing_mm="0.4"/>"##,
    );
    assert_eq!(i.objects.len(), 3);
    // A stroke with no dashes was a zigzag.
    let ObjectKind::Running { params, .. } = &i.objects[0].kind else {
        panic!()
    };
    assert!(params.border.is_some());
    let ObjectKind::Running { params, .. } = &i.objects[1].kind else {
        panic!()
    };
    assert!(params.border.is_none());
    assert!(close(params.stitch_length_mm, 1.5));
    assert!(!i.objects[1].functions.tie_in && !i.objects[1].functions.tie_off);
    let f = fill_params(&i.objects[2]);
    assert!(close(f.angle_deg, -45.0));
    assert!(close(f.spacing_mm, 0.4));
    assert!(close(f.stitch_length_mm, 3.0));
    assert_eq!(f.underlay, Underlay::None, "no underlay in the first versions");
}

#[test]
fn commands_give_trims_stops_start_and_end_points() {
    let body = format!(
        r##"<rect id="shape" x="10" y="10" width="30" height="30" style="fill:#ff0000"/>
 {}{}{}{}
 <rect id="skip" x="50" y="10" width="10" height="10" style="fill:#00ff00"/>{}
 <rect id="after" x="50" y="30" width="10" height="10" style="fill:#0000ff" inkstitch:trim_after="true"
 inkstitch:ties="1"/>"##,
        command("trim", 1, "shape", 10.0, 12.0),
        command("stop", 2, "shape", 40.0, 12.0),
        command("starting_point", 3, "shape", 10.0, 10.0),
        command("ending_point", 4, "shape", 40.0, 40.0),
        command("ignore_object", 5, "skip", 55.0, 10.0),
    );
    let i = read(Some(4), &body);
    assert_eq!(
        i.objects.len(),
        3,
        "the symbols and connectors are not objects: {:?}",
        i.objects
    );
    let o = &i.objects[0];
    assert_eq!(o.functions.trim, TrimMode::Always);
    assert!(o.functions.stop);
    assert!(near(o.start.unwrap(), Point::new(10.0, 10.0)));
    assert!(near(o.end.unwrap(), Point::new(40.0, 40.0)));
    assert!(matches!(i.objects[1].kind, ObjectKind::Vector { .. }));
    let after = &i.objects[2];
    assert_eq!(after.functions.trim, TrimMode::Always);
    assert!(after.functions.tie_in && !after.functions.tie_off);
    assert!(i.warnings.iter().any(|w| w.contains("vector")), "{:?}", i.warnings);
    // The start point moves with the design.
    let d = i.to_design("t");
    let b = i.bbox().unwrap();
    let shift = Point::ZERO - b.center();
    assert!(near(d.objects()[0].start.unwrap(), Point::new(10.0, 10.0) + shift));
}

#[test]
fn elements_with_an_empty_id_or_no_id_are_read() {
    let i = read(
        Some(4),
        r##"<rect id="" width="10" height="10" inkstitch:angle="10"/>
 <g><rect x="20" width="10" height="10" inkstitch:angle="20"/></g>
 <use xlink:href="#inkstitch_trim" x="1" y="1"/>"##,
    );
    assert_eq!(i.objects.len(), 2, "{:?}", i.warnings);
    assert!(close(fill_params(&i.objects[0]).angle_deg, -10.0));
    assert!(close(fill_params(&i.objects[1]).angle_deg, -20.0));
}

#[test]
fn a_layer_with_ignore_layer_is_not_sewn() {
    let svg = format!(
        "{HEAD}<metadata><inkstitch:inkstitch_svg_version>4</inkstitch:inkstitch_svg_version></metadata>{DEFS}\
         <g inkscape:groupmode=\"layer\" id=\"l1\"><rect width=\"10\" height=\"10\"/></g>\
         <g inkscape:groupmode=\"layer\" id=\"l2\"><use xlink:href=\"#inkstitch_ignore_layer\" x=\"0\" y=\"-10\"/>\
         <rect x=\"20\" width=\"10\" height=\"10\"/></g></svg>"
    );
    let i = import_inkstitch(svg.as_bytes()).unwrap();
    assert_eq!(i.objects.len(), 2);
    assert!(matches!(i.objects[0].kind, ObjectKind::Fill { .. }));
    assert!(matches!(i.objects[1].kind, ObjectKind::Vector { .. }));
    // Elements with no id get no name.
    assert_eq!(i.objects[0].name, None);
}

#[test]
fn a_fill_and_a_stroke_of_one_shape_are_a_group() {
    let i = read(
        Some(4),
        r##"<rect id="both" width="20" height="20" style="fill:#ff0000;stroke:#0000ff" inkstitch:stroke_first="true"/>
 <rect id="one" x="30" width="20" height="20" style="fill:#ff0000"/>"##,
    );
    assert_eq!(i.objects.len(), 3);
    assert!(
        matches!(i.objects[0].kind, ObjectKind::Running { .. }),
        "the stroke first"
    );
    assert!(matches!(i.objects[1].kind, ObjectKind::Fill { .. }));
    assert_eq!(i.objects[0].group, Some(1));
    assert_eq!(i.objects[1].group, Some(1));
    assert_eq!(i.objects[2].group, None);
    let d = i.to_design("g");
    assert_eq!(d.objects()[0].group, d.objects()[1].group);
    assert!(d.objects()[0].group.is_some());
}

#[test]
fn settings_that_are_not_read_give_warnings() {
    let i = read(
        Some(4),
        r##"<rect id="a" width="20" height="20" inkstitch:gap_fill_rows="2" inkstitch:a_new_setting="5"
 inkstitch:random_seed="abc" inkstitch:running_stitch_tolerance_mm="0.3" inkstitch:clockwise="true"/>
 <rect id="b" x="30" width="20" height="20" inkstitch:gap_fill_rows="3"/>"##,
    );
    assert_eq!(i.objects.len(), 2);
    let all = i.warnings.join("\n");
    assert!(all.contains("\"gap_fill_rows\" is not read (2 object(s))"), "{all}");
    assert!(all.contains("a_new_setting"), "{all}");
    // Settings with no effect on the stitches and defaults give no warning.
    assert!(
        !all.contains("random_seed") && !all.contains("tolerance") && !all.contains("clockwise"),
        "{all}"
    );
}

#[test]
fn the_thread_palette_names_the_threads() {
    let svg = format!(
        "{HEAD}<metadata><inkstitch:inkstitch_svg_version>4</inkstitch:inkstitch_svg_version>\
         <inkstitch:thread-palette>\"Madeira Polyneon\"</inkstitch:thread-palette></metadata>\
         <rect width=\"10\" height=\"10\" style=\"fill:#b7c3c5\"/><rect x=\"20\" width=\"10\" height=\"10\" \
         style=\"fill:#010203\"/></svg>"
    );
    let i = import_inkstitch(svg.as_bytes()).unwrap();
    assert_eq!(i.chart.as_deref(), Some("Madeira Polyneon"));
    let d = i.to_design("p");
    assert_eq!(d.threads[0].name, "Celestial Blue");
    assert_eq!(d.threads[0].code.as_deref(), Some("1610"));
    assert_eq!(d.threads[0].brand.as_deref(), Some("Madeira Polyneon"));
    // A color that is not in the chart keeps a plain name.
    assert_eq!(d.threads[1].brand, None);
}

#[test]
fn a_clone_turns_its_fill_angle() {
    let i = read(
        Some(4),
        r##"<rect id="src" x="10" y="10" width="20" height="10" style="fill:#ff0000" inkstitch:angle="0"/>
 <use id="copy" xlink:href="#src" transform="rotate(90 50 50)"/>
 <use id="fixed" xlink:href="#src" transform="translate(0 50)" inkstitch:angle="20"/>"##,
    );
    assert_eq!(i.objects.len(), 3);
    assert!(close(fill_params(&i.objects[0]).angle_deg, 0.0));
    assert!(
        close(fill_params(&i.objects[1]).angle_deg, 90.0),
        "{}",
        fill_params(&i.objects[1]).angle_deg
    );
    assert!(close(fill_params(&i.objects[2]).angle_deg, -20.0));
}

#[test]
fn a_linear_gradient_fill_is_a_group_of_fills() {
    let i = read(
        Some(4),
        r##"<defs><linearGradient id="g" x1="0" y1="0" x2="20" y2="0" gradientUnits="userSpaceOnUse">
 <stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs>
 <rect id="grad" width="20" height="20" style="fill:url(#g)" inkstitch:fill_method="linear_gradient_fill"/>"##,
    );
    assert_eq!(i.objects.len(), 2);
    assert_eq!(i.objects[0].color, Rgb(255, 0, 0));
    assert_eq!(i.objects[1].color, Rgb(0, 0, 255));
    assert_eq!(i.objects[0].group, i.objects[1].group);
    let p = fill_params(&i.objects[1]);
    let FillPattern::Gradient { stops, color, line } = &p.pattern else {
        panic!("{:?}", p.pattern)
    };
    assert_eq!((stops.clone(), *color), (vec![0.0, 1.0], 1));
    let (a, b) = line.unwrap();
    assert!(near(a, Point::new(0.0, 0.0)) && near(b, Point::new(20.0, 0.0)));
    // The rows go across the gradient line.
    assert!(close(p.angle_deg.abs(), 90.0));
}
