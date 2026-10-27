//! The standard test corpus (spec, "Standard test corpus"; roadmap P1 exit).
//!
//! Each design is made in code, so it is the same on each run. The designs
//! are for the Topaz 20 in its 260 × 200 mm hoop (200 mm left to right,
//! 260 mm front to back), centered on the origin. A design that is wider
//! than the hoop sews with the hoop turned −90° (`fit_the_hoop`).

use emb_geom::{BBox, Path, Point, Transform};
use emb_model::{
    AppliqueParams, AppliquePiece, Baseline, ColumnPattern, CrossStyle, Design, Envelope, EnvelopeShape, FillParams,
    FillPattern, HoopSelection, Justify, LetterOrder, Lettering, LineBorder, ObjectKind, Rgb, RunStyle, RunningParams,
    SatinGeometry, SatinParams, ThreadId,
};

pub const NAMES: [&str; 58] = [
    "circle-fill",
    "satin-column",
    "s-curve",
    "fill-with-holes",
    "two-color-patch",
    "full-field",
    "applique-patch",
    "lettering-names",
    "embroidery-fonts",
    "embroidery-fonts-2",
    "embroidery-fonts-3",
    "embroidery-fonts-4",
    "lettering-circle",
    "lettering-bridge",
    "monogram",
    "monogram-templates",
    "team-names",
    "keyboard-designs",
    "custom-alphabet",
    "sewout-towel",
    "sewout-cap",
    "outline-types",
    "curved-fills",
    "texture-fills",
    "motifs",
    "patterns",
    "effects",
    "blends",
    "buttonholes",
    "borders",
    "vectors",
    "shaping",
    "arrangements",
    "border-trace",
    "autotrace",
    "smart-logo",
    "photo",
    "hoop-placement",
    "multi-hoop",
    "multi-position",
    "repeats",
    "cross-stitch",
    "cross-picture",
    "rhinestones",
    "tartan-fill",
    "gradient-fill",
    "meander-fill",
    "circular-fill",
    "guided-fill",
    "ripple-stitch",
    "lace-ornament",
    "lace-edge",
    "quilting",
    "quilt-block",
    "puff",
    "cap-order",
    "cutwork",
    "sketch-badge",
];

const MACHINE: &str = "husqvarna_topaz20";
const HOOP: &str = "topaz20_crown";

/// Makes the corpus design `name`, or `None` if there is no such design.
pub fn build(name: &str) -> Option<Design> {
    let mut d = match name {
        "circle-fill" => circle_fill(),
        "satin-column" => satin_column(),
        "s-curve" => s_curve(),
        "fill-with-holes" => fill_with_holes(),
        "two-color-patch" => two_color_patch(),
        "full-field" => full_field(),
        "applique-patch" => applique_patch(),
        "lettering-names" => lettering_names(),
        "embroidery-fonts" => embroidery_fonts(..emb_letter::bundled::FIRST_EMBROIDERY),
        "embroidery-fonts-2" => {
            embroidery_fonts(emb_letter::bundled::FIRST_EMBROIDERY..emb_letter::bundled::P10_EMBROIDERY)
        }
        "embroidery-fonts-3" => embroidery_fonts(emb_letter::bundled::P10_EMBROIDERY..20),
        "embroidery-fonts-4" => embroidery_fonts(20..),
        "lettering-circle" => lettering_circle(),
        "lettering-bridge" => lettering_bridge(LetterOrder::Closest),
        "monogram" => monogram(),
        "monogram-templates" => monogram_templates(),
        "team-names" => team_names(),
        "keyboard-designs" => keyboard_designs(),
        "custom-alphabet" => custom_alphabet(),
        "sewout-towel" => sewout_names(LetterOrder::Closest),
        "sewout-cap" => sewout_names(LetterOrder::CenterOut),
        "outline-types" => outline_types(),
        "curved-fills" => curved_fills(),
        "texture-fills" => texture_fills(),
        "motifs" => motifs(),
        "patterns" => patterns(),
        "effects" => effects(),
        "blends" => blends(),
        "buttonholes" => buttonholes(),
        "borders" => borders(),
        "vectors" => vectors(),
        "shaping" => shaping(),
        "arrangements" => arrangements(),
        "border-trace" => border_trace(),
        "autotrace" => autotrace(),
        "smart-logo" => smart_logo(),
        "photo" => photo(),
        "hoop-placement" => hoop_placement(),
        "multi-hoop" => multi_hoop(),
        "multi-position" => multi_position(),
        "repeats" => repeats(),
        "cross-stitch" => cross_stitch(),
        "cross-picture" => cross_picture(),
        "rhinestones" => rhinestones(),
        "tartan-fill" => tartan_fill(),
        "gradient-fill" => gradient_fill(),
        "meander-fill" => meander_fill(),
        "circular-fill" => circular_fill(),
        "guided-fill" => guided_fill(),
        "ripple-stitch" => ripple_stitch(),
        // P11 Milestone 5: the built-in lace templates.
        "lace-ornament" => emb_model::lace::ornament(),
        "lace-edge" => emb_model::lace::edge(),
        "quilting" => quilting(),
        "quilt-block" => quilt_block(),
        "puff" => puff(),
        "cap-order" => cap_order(),
        "cutwork" => cutwork(),
        "sketch-badge" => sketch_badge(),
        _ => return None,
    };
    d.name = name.to_string();
    d.machine = Some(MACHINE.into());
    d.hoop.selection = HoopSelection::Preset { id: HOOP.into() };
    fit_the_hoop(&mut d);
    Some(d)
}

/// The hoop field is upright: left to right is its short side. A design
/// that is not on the hoop center but fits (inside the safety margin) gets
/// the hoop at its middle. A design that fits only across the hoop sews
/// with the hoop turned `TURN_DEG` (−90°), as the Topaz 20 turns such a
/// file itself: on the machine the top of the design is to the right.
fn fit_the_hoop(d: &mut Design) {
    let field = hoop(HOOP).unwrap();
    let Some(e) = d.extents() else { return };
    if d.hooping.is_some() || d.hoop.angle_deg != 0.0 {
        return;
    }
    // Inside the safety margin, as the design checks want it.
    let m = emb_model::hooping::DEFAULT_MARGIN_MM;
    let inside = |w: f64, h: f64| {
        let (w, h) = (w / 2.0 - m, h / 2.0 - m);
        e.min.x >= -w && e.max.x <= w && e.min.y >= -h && e.max.y <= h
    };
    if inside(field.width_mm, field.height_mm) {
        return;
    }
    if field.fits(e.width(), e.height(), m) {
        d.hoop.center = e.center();
        return;
    }
    if !inside(field.height_mm, field.width_mm) {
        d.hoop.center = e.center();
    }
    d.hoop.angle_deg = emb_model::hooping::TURN_DEG;
}

fn fill(d: &mut Design, t: ThreadId, outer: Path, holes: Vec<Path>, angle: f64) {
    d.add_object(
        t,
        ObjectKind::Fill {
            outer,
            holes,
            params: FillParams {
                angle_deg: angle,
                ..FillParams::default()
            },
        },
    );
}

fn satin(d: &mut Design, t: ThreadId, geometry: SatinGeometry) {
    d.add_object(
        t,
        ObjectKind::Satin {
            geometry,
            params: SatinParams::default(),
        },
    );
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A 50 mm circle, tatami at 45°.
fn circle_fill() -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Red", Rgb(200, 30, 45));
    fill(&mut d, t, Path::ellipse(Point::ZERO, 25.0, 25.0), Vec::new(), 45.0);
    d
}

/// A straight satin column, 3 mm wide and 80 mm long.
fn satin_column() -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Blue", Rgb(30, 60, 140));
    let line = Path::new().move_to(p(0.0, -40.0)).line_to(p(0.0, 40.0));
    satin(&mut d, t, SatinGeometry::from_centerline(&line, 3.0));
    d
}

/// A satin S-curve, 5 mm wide, from a centerline (Column C).
fn s_curve() -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Green", Rgb(40, 150, 90));
    let line = Path::new()
        .move_to(p(-30.0, -30.0))
        .cubic_to(p(30.0, -40.0), p(30.0, 0.0), p(0.0, 0.0))
        .cubic_to(p(-30.0, 0.0), p(-30.0, 40.0), p(30.0, 30.0));
    satin(&mut d, t, SatinGeometry::from_centerline(&line, 5.0));
    d
}

/// A 70 × 50 mm rounded rectangle with two 16 mm holes, tatami at 0°.
fn fill_with_holes() -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Gold", Rgb(230, 170, 30));
    let outer = Path::rounded_rect(BBox::new(p(-35.0, -25.0), p(35.0, 25.0)), 6.0);
    let holes = vec![
        Path::ellipse(p(-16.0, 0.0), 8.0, 8.0),
        Path::ellipse(p(16.0, 0.0), 8.0, 8.0),
    ];
    fill(&mut d, t, outer, holes, 0.0);
    d
}

/// `tests/corpus/svg/two-color-patch.svg`, imported as the app does.
fn two_color_patch() -> Design {
    let svg = include_bytes!("../../../tests/corpus/svg/two-color-patch.svg");
    emb_formats::svg::import_svg(svg)
        .expect("the corpus SVG is valid")
        .to_design("")
}

/// Two names, as in the P4 exit test: 25 mm in Lato Bold and 10 mm in
/// Lato Regular, centered (bundled fonts, so the result is the same on
/// each computer).
fn lettering_names() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    for (t, text, style, height, y) in [
        (navy, "Anna", "Bold", 25.0, 0.0),
        (red, "Joshua", "Regular", 10.0, 25.0),
    ] {
        let font = emb_letter::bundled::font("Lato", style).expect("bundled font");
        let mut l = Lettering::new(text, font.font_ref(), height, p(0.0, y));
        l.justify = Justify::Center;
        emb_letter::relayout(&mut l, &font);
        d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    }
    d
}

/// One name in each bundled embroidery font (ADR 0016), 15 mm high (or
/// the smallest size of the font, if that is larger).
fn embroidery_fonts(
    fonts: impl std::slice::SliceIndex<[(&'static str, &'static [u8])], Output = [(&'static str, &'static [u8])]>,
) -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Navy", Rgb(25, 35, 90));
    let list = &emb_letter::bundled::EMBROIDERY[fonts];
    // Six lines fit the hoop at 26 mm apart; four at 30 mm (as before); more
    // go in two columns of five, 30 mm apart.
    let cols = if list.len() > 6 { 2 } else { 1 };
    let rows = list.len().div_ceil(cols);
    let step = if rows > 4 { 26.0 } else { 30.0 };
    let step = if cols > 1 { 30.0 } else { step };
    let top = -step * (rows as f64 - 1.0) / 2.0;
    for (i, (family, _)) in list.iter().enumerate() {
        let font = emb_letter::bundled::embroidery(family).expect("bundled embroidery font");
        let (col, row) = (i / rows, i % rows);
        let x = if cols > 1 { (col as f64 - 0.5) * 120.0 } else { 0.0 };
        let mut l = Lettering::new(
            "Team 26",
            emb_letter::LetterSource::font_ref(&font),
            15.0f64.max(font.min_height_mm),
            p(x, top + step * row as f64),
        );
        l.justify = Justify::Center;
        emb_letter::relayout(&mut l, &font);
        d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    }
    d
}

/// A badge, as in the P4 Milestone 3 exit test: a text over the top of a
/// circle and a text under it (one center), and a name in the middle with
/// changes by hand: kerning, a turned and larger letter, and a letter with
/// a changed shape. Block Sans (a bundled embroidery font).
fn lettering_circle() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let font = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).expect("bundled font");
    let font_ref = emb_letter::LetterSource::font_ref(&font);
    for (text, clockwise, radius) in [("EMBROIDERY", true, 40.0), ("CLUB", false, 52.0)] {
        let mut l = Lettering::new(text, font_ref.clone(), 10.0, p(0.0, 0.0));
        l.justify = Justify::Center;
        l.letter_spacing_mm = 1.0;
        l.baseline = Baseline::Circle {
            radius_mm: radius,
            clockwise,
        };
        emb_letter::relayout(&mut l, &font);
        d.add_object(navy, ObjectKind::Lettering(Box::new(l)));
    }
    let mut l = Lettering::new("Anna", font_ref, 15.0, p(0.0, 7.0));
    l.justify = Justify::Center;
    l.edit_mut(2).kern_mm = 1.5;
    let e = l.edit_mut(3);
    e.rotate_deg = 10.0;
    e.scale = 1.2;
    emb_letter::relayout(&mut l, &font);
    // The first part of the "A" one millimeter higher, kept as its shape.
    if let Some(part) = l.letters[0].parts.first_mut() {
        part.transform(&Transform::translate(0.0, -1.0));
    }
    l.keep_shape(0);
    emb_letter::relayout(&mut l, &font);
    d.add_object(red, ObjectKind::Lettering(Box::new(l)));
    d
}

/// A name in a bridge envelope, as in the P4 Milestone 4 exit test, in
/// `order`: "Joshua" in Varsity (a bundled embroidery font), 20 mm.
pub fn lettering_bridge(order: LetterOrder) -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Navy", Rgb(25, 35, 90));
    let font = emb_letter::bundled::embroidery("Varsity").expect("bundled font");
    let mut l = Lettering::new("Joshua", emb_letter::LetterSource::font_ref(&font), 20.0, p(0.0, 10.0));
    l.justify = Justify::Center;
    l.envelope = Envelope {
        shape: EnvelopeShape::Bridge,
        amount: 0.4,
    };
    l.order = order;
    emb_letter::relayout(&mut l, &font);
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    d
}

/// A three-letter monogram with a border from a template, as in the P4
/// Milestone 5 exit test: "Classic in a circle" with "JMK".
fn monogram() -> Design {
    let mut d = Design::new("");
    add_monogram(&mut d, 0, "JMK", 30.0, p(0.0, 0.0));
    d
}

/// Each built-in monogram template, 16 mm high, in two rows.
fn monogram_templates() -> Design {
    let mut d = Design::new("");
    for (i, initials) in ["ABC", "DEF", "GHI", "JKL", "MNO", "P"].iter().enumerate() {
        let at = p(-80.0 + 80.0 * (i % 3) as f64, -45.0 + 90.0 * (i / 3) as f64);
        add_monogram(&mut d, i, initials, 16.0, at);
    }
    d
}

/// Team names (P4 Milestone 6): a name and a number for six players, all
/// in one design, color by color.
fn team_names() -> Design {
    let font = std::sync::Arc::new(
        emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).expect("bundled font"),
    );
    let font_ref = emb_letter::LetterSource::font_ref(font.as_ref());
    let mut d = Design::new("Team");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let mut cols = Vec::new();
    for (t, text, h, y) in [(navy, "NAME", 10.0, -8.0), (red, "00", 20.0, 16.0)] {
        let mut l = Lettering::new(text, font_ref.clone(), h, p(0.0, y));
        l.justify = Justify::Center;
        emb_letter::relayout(&mut l, font.as_ref());
        cols.push(d.add_object(t, ObjectKind::Lettering(Box::new(l))));
    }
    let mut tn = emb_model::TeamNames {
        columns: cols
            .iter()
            .zip(["Name", "Number"])
            .map(|(id, title)| emb_model::TeamColumn {
                object: *id,
                title: title.into(),
                max_width_mm: Some(55.0),
            })
            .collect(),
        rows: vec![],
    };
    tn.import_text("Anna,7\nJoshua,10\nMaximiliana,23\nLee,4\nSofia,11\nKai,99", false);
    let mut fonts = move |_: &emb_model::FontRef| Some(emb_letter::AnyFont::Embroidery(font.clone()));
    emb_letter::teamnames::layout_design(&d, &tn, &mut fonts, 240.0, 10.0).expect("team layout")
}

/// "KDC" typed with the built-in keyboard design collection (letters over
/// a shadow in a second color).
fn keyboard_designs() -> Design {
    let font = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).expect("bundled font");
    let k = emb_letter::keyboard::shadow_collection(&font, "Shadow", Rgb(25, 35, 90), Rgb(180, 140, 40));
    let (objs, _) = emb_letter::keyboard::type_text(&k, "KDC", 25.0, 1.0, p(-30.0, 12.0));
    let mut d = Design::new("");
    for (color, kind) in objs {
        let t = d.thread_for_color(color);
        d.add_object(t, kind);
    }
    d
}

/// A three-letter alphabet made from drawn objects (P4 Milestone 7 exit
/// test), and "LIT" lettered with it at 30 mm.
fn custom_alphabet() -> Design {
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| ObjectKind::Fill {
        outer: Path::rect(BBox::new(p(x0, y0), p(x1, y1))),
        holes: vec![],
        params: FillParams::default(),
    };
    let mut f = emb_letter::EmbFont::empty("My Blocks", "");
    for (c, parts) in [
        ('I', vec![rect(0.0, -20.0, 4.0, 0.0)]),
        ('L', vec![rect(0.0, -20.0, 4.0, 0.0), rect(4.0, -4.0, 14.0, 0.0)]),
        ('T', vec![rect(0.0, -20.0, 14.0, -16.0), rect(5.0, -16.0, 9.0, 0.0)]),
    ] {
        let letter = emb_letter::embfont::letter_from_objects(&parts, 0.0, 20.0).expect("letter");
        f.set_letter(c, letter);
    }
    let mut d = Design::new("");
    let t = d.add_thread("Navy", Rgb(25, 35, 90));
    let mut l = Lettering::new("LIT", emb_letter::LetterSource::font_ref(&f), 30.0, p(0.0, 15.0));
    l.justify = Justify::Center;
    emb_letter::relayout(&mut l, &f);
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    d
}

/// The P4 exit test sew-out: names at 25 mm and 10 mm in a block font
/// (Block Sans) and a script font (Pacifico), in `order` (closest join for
/// the towel, center out for the cap). Write with `--fabric towel` or
/// `--fabric cap`.
/// P5 Milestone 1: each outline type on a wavy 80 mm line (left, blue) and
/// each column type on a tapered 80 mm column, 3 to 7 mm wide (right, red).
/// One row for each type; the first row on the left is a sculpture run.
fn outline_types() -> Design {
    let mut d = Design::new("");
    let blue = d.add_thread("Blue", Rgb(30, 60, 140));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let row = |i: usize| -80.0 + 15.0 * i as f64;
    let wave = |x0: f64, y: f64| {
        Path::new()
            .move_to(p(x0, y))
            .cubic_to(p(x0 + 25.0, y - 6.0), p(x0 + 55.0, y + 6.0), p(x0 + 80.0, y))
    };
    d.add_object(
        blue,
        ObjectKind::Running {
            path: wave(-100.0, row(0)),
            params: RunningParams {
                style: RunStyle::Sculpture,
                ..RunningParams::default()
            },
        },
    );
    for (i, pattern) in ColumnPattern::ALL.into_iter().enumerate() {
        let width = match pattern {
            ColumnPattern::Tatami { .. } | ColumnPattern::Contour { .. } => 6.0,
            ColumnPattern::EStitch | ColumnPattern::Square | ColumnPattern::Coil => 4.0,
            _ => 3.0,
        };
        d.add_object(
            blue,
            ObjectKind::Running {
                path: wave(-100.0, row(i + 1)),
                params: RunningParams {
                    border: Some(LineBorder::new(width, pattern)),
                    ..RunningParams::default()
                },
            },
        );
    }
    for (i, pattern) in ColumnPattern::ALL.into_iter().enumerate() {
        let y = row(i) + 9.0;
        let geometry = SatinGeometry {
            rail_a: Path::new().move_to(p(10.0, y - 1.5)).cubic_to(
                p(35.0, y - 8.0),
                p(65.0, y - 2.0),
                p(90.0, y - 3.5),
            ),
            rail_b: Path::new().move_to(p(10.0, y + 1.5)).cubic_to(
                p(35.0, y - 4.0),
                p(65.0, y + 2.0),
                p(90.0, y + 3.5),
            ),
            rungs: Vec::new(),
        };
        d.add_object(
            red,
            ObjectKind::Satin {
                geometry,
                params: SatinParams::with_pattern(pattern),
            },
        );
    }
    d
}

/// P5 Milestone 2: each curved fill type on a shape about 46 mm across, in
/// two rows of four (top: contour, offset, spiral, island coil; bottom:
/// radial, maze, Florentine with a wave guide, and a liquid satin column
/// with a sharp turn of its stitch angle).
fn curved_fills() -> Design {
    let mut d = Design::new("");
    let green = d.add_thread("Green", Rgb(40, 150, 90));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let xs = [-96.0, -32.0, 32.0, 96.0];
    let (top, bottom) = (-45.0, 45.0);
    let add = |d: &mut Design, t: ThreadId, outer: Path, holes: Vec<Path>, pattern: FillPattern| {
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes,
                params: FillParams::default().with_pattern(pattern),
            },
        );
    };
    // A leaf with a small hole for contour: the lines go round the hole.
    add(
        &mut d,
        green,
        Path::ellipse(p(xs[0], top), 26.0, 16.0),
        vec![Path::ellipse(p(xs[0] + 6.0, top), 3.0, 3.0)],
        FillPattern::Contour,
    );
    add(
        &mut d,
        green,
        Path::rounded_rect(BBox::new(p(xs[1] - 22.0, top - 20.0), p(xs[1] + 22.0, top + 20.0)), 6.0),
        Vec::new(),
        FillPattern::Offset,
    );
    add(
        &mut d,
        green,
        Path::ellipse(p(xs[2], top), 22.0, 22.0),
        Vec::new(),
        FillPattern::Spiral,
    );
    add(
        &mut d,
        green,
        Path::rounded_rect(
            BBox::new(p(xs[3] - 22.0, top - 22.0), p(xs[3] + 22.0, top + 22.0)),
            10.0,
        ),
        Vec::new(),
        FillPattern::IslandCoil,
    );
    // A sun: an eight-point star, lines from its middle.
    let star: Vec<Point> = (0..16)
        .map(|i| {
            let r = if i % 2 == 0 { 24.0 } else { 14.0 };
            let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 22.5);
            p(xs[0] + r * c, bottom + r * s)
        })
        .collect();
    add(
        &mut d,
        gold,
        Path::polygon(&star),
        Vec::new(),
        FillPattern::Radial { center: None },
    );
    add(
        &mut d,
        gold,
        Path::ellipse(p(xs[1], bottom), 23.0, 20.0),
        Vec::new(),
        FillPattern::Maze,
    );
    let guide = vec![
        p(xs[2] - 24.0, bottom),
        p(xs[2] - 12.0, bottom - 6.0),
        p(xs[2], bottom),
        p(xs[2] + 12.0, bottom + 6.0),
        p(xs[2] + 24.0, bottom),
    ];
    add(
        &mut d,
        gold,
        Path::rect(BBox::new(
            p(xs[2] - 22.0, bottom - 18.0),
            p(xs[2] + 22.0, bottom + 18.0),
        )),
        Vec::new(),
        FillPattern::Florentine { guide },
    );
    // A column 8 mm wide whose stitch angle lines turn 45° at once.
    let (x0, y0) = (xs[3] - 22.0, bottom);
    d.add_object(
        gold,
        ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a: Path::new().move_to(p(x0, y0 - 4.0)).line_to(p(x0 + 44.0, y0 - 4.0)),
                rail_b: Path::new().move_to(p(x0, y0 + 4.0)).line_to(p(x0 + 44.0, y0 + 4.0)),
                rungs: vec![
                    (p(x0 + 20.0, y0 - 4.0), p(x0 + 20.0, y0 + 4.0)),
                    (p(x0 + 22.0, y0 - 4.0), p(x0 + 28.0, y0 + 4.0)),
                    (p(x0 + 34.0, y0 - 4.0), p(x0 + 34.0, y0 + 4.0)),
                ],
            },
            params: SatinParams {
                liquid: true,
                ..SatinParams::default()
            },
        },
    );
    d
}

/// P5 Milestone 3: each texture fill type on a shape about 44 mm across, in
/// three rows of four. Top: satin, zigzag, E stitch and raised satin
/// across shapes; middle: coil, square, double square, string; bottom:
/// cross stitch, stipple, a laydown (two layers) under a satin ring, and a
/// line with a string outline.
fn texture_fills() -> Design {
    let mut d = Design::new("");
    let blue = d.add_thread("Blue", Rgb(30, 60, 140));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let cream = d.add_thread("Cream (laydown)", Rgb(240, 235, 220));
    let green = d.add_thread("Green", Rgb(40, 150, 90));
    let xs = [-96.0, -32.0, 32.0, 96.0];
    let ys = [-66.0, 0.0, 66.0];
    let add = |d: &mut Design, t: ThreadId, outer: Path, angle: f64, pattern: FillPattern| {
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params: FillParams {
                    angle_deg: angle,
                    ..FillParams::default().with_pattern(pattern)
                },
            },
        );
    };
    let col = |column| FillPattern::Columns { column };
    let rr = |x: f64, y: f64, w: f64, h: f64, r: f64| {
        Path::rounded_rect(BBox::new(p(x - w / 2.0, y - h / 2.0), p(x + w / 2.0, y + h / 2.0)), r)
    };
    add(
        &mut d,
        blue,
        Path::ellipse(p(xs[0], ys[0]), 22.0, 12.0),
        80.0,
        col(ColumnPattern::Satin),
    );
    add(
        &mut d,
        blue,
        rr(xs[1], ys[0], 40.0, 30.0, 6.0),
        90.0,
        col(ColumnPattern::Zigzag),
    );
    add(
        &mut d,
        blue,
        Path::ellipse(p(xs[2], ys[0]), 20.0, 20.0),
        90.0,
        col(ColumnPattern::EStitch),
    );
    add(
        &mut d,
        blue,
        Path::ellipse(p(xs[3], ys[0]), 22.0, 6.0),
        90.0,
        col(ColumnPattern::Raised { layers: 2 }),
    );
    add(
        &mut d,
        red,
        rr(xs[0], ys[1], 44.0, 16.0, 7.0),
        90.0,
        col(ColumnPattern::Coil),
    );
    add(
        &mut d,
        red,
        rr(xs[1], ys[1], 40.0, 30.0, 0.0),
        45.0,
        FillPattern::Square,
    );
    add(
        &mut d,
        red,
        Path::ellipse(p(xs[2], ys[1]), 20.0, 20.0),
        45.0,
        FillPattern::DoubleSquare { angle_deg: 90.0 },
    );
    add(
        &mut d,
        red,
        Path::ellipse(p(xs[3], ys[1]), 20.0, 20.0),
        0.0,
        FillPattern::String,
    );
    add(
        &mut d,
        green,
        rr(xs[0], ys[2], 40.0, 30.0, 0.0),
        0.0,
        FillPattern::CrossStitch {
            cell_mm: 2.5,
            style: CrossStyle::Full,
            passes: 1,
        },
    );
    add(
        &mut d,
        green,
        rr(xs[1], ys[2], 40.0, 34.0, 8.0),
        0.0,
        FillPattern::Stipple { style: RunStyle::Run },
    );
    // The laydown sews before the ring on it.
    add(
        &mut d,
        cream,
        Path::ellipse(p(xs[2], ys[2]), 22.0, 18.0),
        45.0,
        FillPattern::Laydown {
            layers: 2,
            angle_deg: 90.0,
        },
    );
    let ring = Path::ellipse(p(xs[2], ys[2]), 17.0, 13.0);
    satin(&mut d, green, SatinGeometry::from_centerline(&ring, 4.0));
    d.add_object(
        green,
        ObjectKind::Running {
            path: Path::new().move_to(p(xs[3] - 22.0, ys[2])).cubic_to(
                p(xs[3] - 8.0, ys[2] - 14.0),
                p(xs[3] + 8.0, ys[2] + 14.0),
                p(xs[3] + 22.0, ys[2]),
            ),
            params: RunningParams {
                border: Some(LineBorder::new(5.0, ColumnPattern::String { chord_mm: 4.0 })),
                ..RunningParams::default()
            },
        },
    );
    d
}

/// P5 Milestone 4: motif runs on four wavy lines (heart, scallop, leaf and
/// a motif made from points), motif columns on two tapered columns (peak,
/// flower), a bead run round a circle, and motif fills: plain (diamond),
/// with a gradient (peak), with the 3D warp (star), and staggered hearts.
fn motifs() -> Design {
    use emb_model::{Motif, MotifRun};
    let mut d = Design::new("");
    let purple = d.add_thread("Purple", Rgb(110, 50, 150));
    let teal = d.add_thread("Teal", Rgb(20, 130, 140));
    let orange = d.add_thread("Orange", Rgb(230, 120, 30));
    let m = |name: &str| Motif::builtin(name).expect("built-in motif");
    let wave = |x0: f64, y: f64| {
        Path::new()
            .move_to(p(x0, y))
            .cubic_to(p(x0 + 25.0, y - 8.0), p(x0 + 55.0, y + 8.0), p(x0 + 80.0, y))
    };
    let mine = Motif::from_points(
        "Steps",
        &[
            p(0.0, 0.0),
            p(0.0, -2.0),
            p(2.0, -2.0),
            p(2.0, -4.0),
            p(4.0, -4.0),
            p(4.0, 0.0),
        ],
    )
    .expect("a motif with width");
    for (i, (motif, len, h)) in [
        (m("Heart"), 7.0, 5.0),
        (m("Scallop"), 6.0, 3.0),
        (m("Leaf"), 8.0, 3.0),
        (mine, 5.0, 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        let (x0, y) = if i < 2 {
            (-120.0, -80.0 + 20.0 * i as f64)
        } else {
            (-120.0, -40.0 + 20.0 * (i - 2) as f64)
        };
        d.add_object(
            purple,
            ObjectKind::Running {
                path: wave(x0, y),
                params: RunningParams {
                    motif: Some(MotifRun {
                        length_mm: len,
                        height_mm: h,
                        ..MotifRun::new(motif)
                    }),
                    ..RunningParams::default()
                },
            },
        );
    }
    for (i, motif) in [m("Peak"), m("Flower")].into_iter().enumerate() {
        let y = -75.0 + 30.0 * i as f64;
        d.add_object(
            teal,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::new().move_to(p(10.0, y - 3.0)).line_to(p(110.0, y - 8.0)),
                    rail_b: Path::new().move_to(p(10.0, y + 3.0)).line_to(p(110.0, y + 8.0)),
                    rungs: Vec::new(),
                },
                params: SatinParams {
                    motif: Some(motif),
                    ..SatinParams::with_pattern(ColumnPattern::Motif { aspect: 1.0 })
                },
            },
        );
    }
    d.add_object(
        teal,
        ObjectKind::Running {
            path: Path::ellipse(p(60.0, -2.0), 14.0, 14.0),
            params: RunningParams {
                motif: Some(MotifRun {
                    length_mm: 6.0,
                    height_mm: 4.0,
                    ..MotifRun::new(m("Bead"))
                }),
                ..RunningParams::default()
            },
        },
    );
    let xs = [-96.0, -32.0, 32.0, 96.0];
    let y = 60.0;
    let fills = [
        (
            Path::rect(BBox::new(p(xs[0] - 22.0, y - 20.0), p(xs[0] + 22.0, y + 20.0))),
            m("Diamond"),
            1.0,
            false,
        ),
        (Path::ellipse(p(xs[1], y), 23.0, 20.0), m("Peak"), 2.0, false),
        (Path::ellipse(p(xs[2], y), 22.0, 22.0), m("Star"), 1.0, true),
        (
            Path::rounded_rect(BBox::new(p(xs[3] - 22.0, y - 20.0), p(xs[3] + 22.0, y + 20.0)), 8.0),
            m("Heart"),
            1.0,
            false,
        ),
    ];
    for (outer, motif, gradient, globe) in fills {
        d.add_object(
            orange,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params: FillParams {
                    angle_deg: 0.0,
                    ..FillParams::default().with_pattern(FillPattern::Motif {
                        motif,
                        length_mm: 6.0,
                        height_mm: 4.0,
                        stagger: 0.5,
                        gradient,
                        globe,
                    })
                },
            },
        );
    }
    d
}

/// P5 Milestone 5: program-split fills (brick, diamond, wave, herringbone,
/// scales, basket), a flexi split across a leaf, a user split with three
/// lines, tatami with carving stamps (heart; star; swirl and leaf), and a
/// program-split column.
fn patterns() -> Design {
    use emb_model::{SplitPattern, Stamp, StampPlacement};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let wine = d.add_thread("Wine", Rgb(130, 20, 50));
    let olive = d.add_thread("Olive", Rgb(100, 110, 30));
    let xs = [-96.0, -32.0, 32.0, 96.0];
    let ys = [-66.0, 0.0, 66.0];
    let rect = |x: f64, y: f64| Path::rect(BBox::new(p(x - 22.0, y - 18.0), p(x + 22.0, y + 18.0)));
    let add = |d: &mut Design, t: ThreadId, outer: Path, params: FillParams| {
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params,
            },
        );
    };
    let split = |name: &str| FillParams {
        angle_deg: 0.0,
        ..FillParams::default().with_pattern(FillPattern::ProgramSplit {
            pattern: SplitPattern::builtin(name).expect("built-in pattern"),
            scale: 1.0,
        })
    };
    for (i, name) in ["Brick", "Diamond", "Wave", "Herringbone"].into_iter().enumerate() {
        add(&mut d, navy, rect(xs[i], ys[0]), split(name));
    }
    add(&mut d, wine, rect(xs[0], ys[1]), split("Scales"));
    add(&mut d, wine, rect(xs[1], ys[1]), split("Basket"));
    add(
        &mut d,
        wine,
        Path::ellipse(p(xs[2], ys[1]), 24.0, 14.0),
        FillParams {
            angle_deg: 90.0,
            ..FillParams::default().with_pattern(FillPattern::FlexiSplit {
                pattern: SplitPattern::builtin("Zigzag").expect("built-in pattern"),
                scale: 1.0,
            })
        },
    );
    let (x, y) = (xs[3], ys[1]);
    add(
        &mut d,
        wine,
        rect(x, y),
        FillParams {
            angle_deg: 0.0,
            ..FillParams::default().with_pattern(FillPattern::UserSplit {
                lines: (0..3)
                    .map(|k| {
                        let x0 = x - 15.0 + 15.0 * f64::from(k);
                        vec![p(x0 - 6.0, y - 20.0), p(x0 + 6.0, y + 20.0)]
                    })
                    .collect(),
            })
        },
    );
    let stamped = |center: Point, stamps: Vec<(&str, f64, f64)>| FillParams {
        angle_deg: 0.0,
        stamps: stamps
            .into_iter()
            .map(|(name, size_mm, dx)| StampPlacement {
                stamp: Stamp::builtin(name).expect("built-in stamp"),
                center: center + p(dx, 0.0),
                size_mm,
                angle_deg: 0.0,
            })
            .collect(),
        ..FillParams::default()
    };
    add(
        &mut d,
        olive,
        rect(xs[0], ys[2]),
        stamped(p(xs[0], ys[2]), vec![("Heart", 26.0, 0.0)]),
    );
    add(
        &mut d,
        olive,
        Path::ellipse(p(xs[1], ys[2]), 20.0, 20.0),
        stamped(p(xs[1], ys[2]), vec![("Star", 30.0, 0.0)]),
    );
    add(
        &mut d,
        olive,
        rect(xs[3], ys[2]),
        stamped(p(xs[3], ys[2]), vec![("Swirl", 20.0, -10.0), ("Leaf", 18.0, 11.0)]),
    );
    // A program-split column, 10 mm wide, on a curve.
    let line = Path::new().move_to(p(xs[2] - 22.0, ys[2] + 8.0)).cubic_to(
        p(xs[2] - 8.0, ys[2] - 14.0),
        p(xs[2] + 8.0, ys[2] + 14.0),
        p(xs[2] + 22.0, ys[2] - 8.0),
    );
    d.add_object(
        olive,
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&line, 10.0),
            params: SatinParams {
                split_pattern: SplitPattern::builtin("Diamond"),
                ..SatinParams::with_pattern(ColumnPattern::ProgramSplit { scale: 1.0 })
            },
        },
    );
    d
}

/// P5 Milestone 6: stitch effects. Top row (satin): auto jump on a 10 mm
/// column (no splits; the Topaz takes stitches up to 12 mm), spacing measured at the inner rail of a ring, auto spacing on a
/// tapered column, accordion spacing. Middle: a jagged edge, pull
/// compensation for each side and as a percent, and lines with a width
/// round corners as mitre, cap and lap. Bottom: fills with trapunto (a
/// hole), accordion rows, a jagged edge, the hand-stitch effect and double
/// underlay; a hand-stitched line and cording.
fn effects() -> Design {
    use emb_model::{Accordion, Corner};
    let mut d = Design::new("");
    let teal = d.add_thread("Teal", Rgb(20, 130, 140));
    let brown = d.add_thread("Brown", Rgb(120, 70, 30));
    let plum = d.add_thread("Plum", Rgb(110, 40, 100));
    let preset = |name: &str| {
        Accordion::presets()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, a)| a)
    };
    let straight = |x: f64, y: f64, w0: f64, w1: f64, l: f64| SatinGeometry {
        rail_a: Path::new().move_to(p(x, y - w0 / 2.0)).line_to(p(x + l, y - w1 / 2.0)),
        rail_b: Path::new().move_to(p(x, y + w0 / 2.0)).line_to(p(x + l, y + w1 / 2.0)),
        rungs: Vec::new(),
    };
    let add_satin = |d: &mut Design, t: ThreadId, geometry: SatinGeometry, params: SatinParams| {
        d.add_object(t, ObjectKind::Satin { geometry, params });
    };
    let y0 = -75.0;
    add_satin(
        &mut d,
        teal,
        straight(-120.0, y0, 10.0, 10.0, 40.0),
        SatinParams {
            auto_jump: true,
            spacing_mm: 0.6,
            ..SatinParams::default()
        },
    );
    // A half ring 6 mm wide: spacing measured at the inner rail.
    let arc = |r: f64| {
        let pts: Vec<(Point, bool)> = (0..=18)
            .map(|i| {
                let (s, c) = emb_geom::math::sin_cos_deg(180.0 + f64::from(i) * 10.0);
                (p(-40.0 + r * c, y0 + 10.0 + r * s), true)
            })
            .collect();
        Path::through_points(&pts, false)
    };
    add_satin(
        &mut d,
        teal,
        SatinGeometry {
            rail_a: arc(10.0),
            rail_b: arc(16.0),
            rungs: Vec::new(),
        },
        SatinParams {
            spacing_at: 0.0,
            ..SatinParams::default()
        },
    );
    add_satin(
        &mut d,
        teal,
        straight(0.0, y0, 1.0, 7.0, 50.0),
        SatinParams {
            auto_spacing: true,
            ..SatinParams::default()
        },
    );
    add_satin(
        &mut d,
        teal,
        straight(65.0, y0, 6.0, 6.0, 55.0),
        SatinParams {
            accordion: preset("Fade out"),
            ..SatinParams::default()
        },
    );
    let y1 = -25.0;
    add_satin(
        &mut d,
        brown,
        straight(-120.0, y1, 8.0, 8.0, 40.0),
        SatinParams {
            jagged_mm: 2.0,
            ..SatinParams::default()
        },
    );
    add_satin(
        &mut d,
        brown,
        straight(-70.0, y1, 5.0, 5.0, 40.0),
        SatinParams {
            pull_compensation_mm: 0.1,
            pull_b_mm: Some(0.5),
            pull_percent: 8.0,
            ..SatinParams::default()
        },
    );
    for (i, corner) in [Corner::Mitre, Corner::Cap, Corner::Lap].into_iter().enumerate() {
        let x = -15.0 + 45.0 * i as f64;
        let line = Path::new()
            .move_to(p(x, y1 + 12.0))
            .line_to(p(x + 12.0, y1 - 12.0))
            .line_to(p(x + 24.0, y1 + 12.0))
            .line_to(p(x + 36.0, y1 - 12.0));
        d.add_object(
            brown,
            ObjectKind::Running {
                path: line,
                params: RunningParams {
                    border: Some(LineBorder {
                        corner,
                        ..LineBorder::new(4.0, ColumnPattern::Satin)
                    }),
                    ..RunningParams::default()
                },
            },
        );
    }
    let y2 = 30.0;
    let add_fill = |d: &mut Design, outer: Path, holes: Vec<Path>, params: FillParams| {
        d.add_object(plum, ObjectKind::Fill { outer, holes, params });
    };
    let sq = |x: f64, y: f64| Path::rect(BBox::new(p(x, y - 15.0), p(x + 40.0, y + 15.0)));
    add_fill(
        &mut d,
        sq(-120.0, y2),
        vec![Path::ellipse(p(-100.0, y2), 7.0, 7.0)],
        FillParams {
            trapunto: true,
            ..FillParams::default()
        },
    );
    add_fill(
        &mut d,
        sq(-70.0, y2),
        Vec::new(),
        FillParams {
            angle_deg: 0.0,
            accordion: preset("Dense middle"),
            // The rows are open, so an underlay would show.
            underlay: emb_model::Underlay::None,
            ..FillParams::default()
        },
    );
    add_fill(
        &mut d,
        Path::ellipse(p(0.0, y2), 20.0, 15.0),
        Vec::new(),
        FillParams {
            jagged_mm: 2.0,
            ..FillParams::default()
        },
    );
    add_fill(
        &mut d,
        sq(30.0, y2),
        Vec::new(),
        FillParams {
            hand_mm: 0.4,
            spacing_mm: 0.6,
            ..FillParams::default()
        },
    );
    add_fill(
        &mut d,
        sq(80.0, y2),
        Vec::new(),
        FillParams {
            underlay: emb_model::Underlay::Double,
            ..FillParams::default()
        },
    );
    let y3 = 75.0;
    let wave = |x0: f64| {
        Path::new()
            .move_to(p(x0, y3))
            .cubic_to(p(x0 + 30.0, y3 - 15.0), p(x0 + 60.0, y3 + 15.0), p(x0 + 90.0, y3))
    };
    d.add_object(
        plum,
        ObjectKind::Running {
            path: wave(-120.0),
            params: RunningParams {
                hand_mm: 0.5,
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    d.add_object(
        plum,
        ObjectKind::Running {
            path: wave(10.0),
            params: RunningParams {
                cording: true,
                ..RunningParams::default()
            },
        },
    );
    d
}

/// P5 Milestone 7: color blends. A two-color fill blend (red to gold), a
/// three-layer multi blend (navy, teal, white) and a satin blend.
fn blends() -> Design {
    use emb_model::BlendRecord;
    use emb_model::decor::blend_profile;
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let teal = d.add_thread("Teal", Rgb(20, 130, 140));
    let white = d.add_thread("White", Rgb(245, 245, 240));
    let layer = |d: &mut Design, t: ThreadId, kind: ObjectKind, g: u32| {
        let id = d.add_object(t, kind);
        if let Some(o) = d.object_mut(id) {
            o.group = Some(g);
        }
    };
    let fill = |outer: Path, start: f64, end: f64, underlay: bool| ObjectKind::Fill {
        outer,
        holes: Vec::new(),
        params: FillParams {
            angle_deg: 0.0,
            accordion: Some(blend_profile(start, end)),
            underlay: if underlay {
                emb_model::Underlay::Auto
            } else {
                emb_model::Underlay::None
            },
            ..FillParams::default()
        },
    };
    let rect = Path::rect(BBox::new(p(-110.0, -40.0), p(-30.0, 40.0)));
    layer(&mut d, red, fill(rect.clone(), 100.0, 10.0, true), 1);
    layer(&mut d, gold, fill(rect, 10.0, 100.0, false), 1);
    let oval = Path::ellipse(p(30.0, 0.0), 40.0, 40.0);
    layer(&mut d, navy, fill(oval.clone(), 100.0, 5.0, true), 2);
    layer(&mut d, teal, fill(oval.clone(), 20.0, 60.0, false), 2);
    layer(&mut d, white, fill(oval, 5.0, 100.0, false), 2);
    let column = |start: f64, end: f64, underlay: bool| ObjectKind::Satin {
        geometry: SatinGeometry::from_centerline(&Path::new().move_to(p(90.0, -60.0)).line_to(p(90.0, 60.0)), 12.0),
        params: SatinParams {
            accordion: Some(blend_profile(start, end)),
            underlay: if underlay {
                emb_model::Underlay::Auto
            } else {
                emb_model::Underlay::None
            },
            ..SatinParams::default()
        },
    };
    layer(&mut d, red, column(100.0, 10.0, true), 3);
    layer(&mut d, gold, column(10.0, 100.0, false), 3);
    d.blends = (1..=3).map(|group| BlendRecord { group }).collect();
    d
}

/// P5 Milestone 7: a string of five buttonholes across a line (a shirt
/// front), and two buttonholes along a line (a cuff).
fn buttonholes() -> Design {
    use emb_model::ButtonholeSpec;
    use emb_model::decor::{buttonhole, string_positions};
    let mut d = Design::new("");
    let t = d.add_thread("Navy", Rgb(25, 40, 100));
    let spec = ButtonholeSpec::default();
    let front = [p(-40.0, -80.0), p(-40.0, 80.0)];
    let cuff = [p(30.0, 0.0), p(90.0, 0.0)];
    for (line, count, across) in [(&front[..], 5, true), (&cuff[..], 2, false)] {
        for (c, a) in string_positions(line, count, across) {
            for kind in buttonhole(&spec, c, a) {
                d.add_object(t, kind);
            }
        }
    }
    d
}

/// P5 Milestone 7: each built-in border round a small fill, three rows of
/// four.
fn borders() -> Design {
    use emb_model::decor::{frame_objects, frames};
    let mut d = Design::new("");
    let fill_t = d.add_thread("Gold", Rgb(230, 170, 30));
    let frame_t = d.add_thread("Navy", Rgb(25, 40, 100));
    let boxes: Vec<BBox> = (0..frames().len())
        .map(|i| {
            let (col, row) = (i % 4, i / 4);
            BBox::from_center_size(p(-96.0 + 64.0 * col as f64, -66.0 + 66.0 * row as f64), 26.0, 16.0)
        })
        .collect();
    // All fills first, then all borders: two colors.
    for b in &boxes {
        fill(&mut d, fill_t, Path::rect(*b), Vec::new(), 45.0);
    }
    for (f, b) in frames().iter().zip(&boxes) {
        for kind in frame_objects(f, b, 3.0) {
            d.add_object(frame_t, kind);
        }
    }
    d
}

/// P6 Milestone 1: vector artwork converted to embroidery. A ring (fill
/// with a hole), a closed bar (turning satin), a wave (satin along a line)
/// and a star outline (running line), each made from a vector; one vector
/// stays artwork and sews nothing.
fn vectors() -> Design {
    use emb_model::vector::from_vector;
    use emb_model::{ConvertTo, CurrentProperties, VectorStyle};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let current = CurrentProperties::default();
    let ring = Path::joined([
        &Path::ellipse(p(-60.0, -30.0), 30.0, 30.0),
        &Path::ellipse(p(-60.0, -30.0), 15.0, 15.0),
    ]);
    let bar = Path::rect(BBox::new(p(10.0, -45.0), p(90.0, -35.0)));
    let wave = Path::new()
        .move_to(p(10.0, -10.0))
        .cubic_to(p(30.0, -30.0), p(50.0, 10.0), p(90.0, -10.0));
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 30.0 } else { 13.0 };
            let a = std::f64::consts::PI * (f64::from(i) / 5.0 - 0.5);
            p(-60.0 + r * a.cos(), 45.0 + r * a.sin())
        })
        .collect();
    let star = Path::polygon(&star);
    for (path, to, t) in [
        (&ring, ConvertTo::Fill, navy),
        (&bar, ConvertTo::Satin, gold),
        (&wave, ConvertTo::Satin, navy),
        (&star, ConvertTo::Outline, gold),
    ] {
        for kind in from_vector(path, to, &current, 4.0) {
            d.add_object(t, kind);
        }
    }
    // Artwork that stays a vector: no stitches in the files.
    d.add_object(
        navy,
        ObjectKind::Vector {
            path: Path::rect(BBox::new(p(10.0, 20.0), p(90.0, 70.0))),
            style: VectorStyle::filled(Rgb(25, 40, 100)),
        },
    );
    d
}

/// P6 Milestone 2: shaping. A square (red, behind) and a circle (blue, in
/// front) for each command: weld, intersect, exclude, front minus back,
/// back minus front, flatten, and divide with a 0.5 mm overlap.
fn shaping() -> Design {
    use emb_model::{ShapeOp, ShapeOptions};
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let blue = d.add_thread("Blue", Rgb(30, 90, 190));
    let ops = [
        ShapeOp::Weld,
        ShapeOp::Intersect,
        ShapeOp::Exclude,
        ShapeOp::FrontMinusBack,
        ShapeOp::BackMinusFront,
        ShapeOp::Flatten,
        ShapeOp::Divide,
    ];
    for (i, op) in ops.into_iter().enumerate() {
        let c = p(-90.0 + 60.0 * (i % 4) as f64, -35.0 + 70.0 * (i / 4) as f64);
        let square = Path::rect(BBox::new(c + p(-20.0, -20.0), c + p(10.0, 10.0)));
        let circle = Path::ellipse(c + p(6.0, 6.0), 14.0, 14.0);
        fill(&mut d, red, square, Vec::new(), 45.0);
        fill(&mut d, blue, circle, Vec::new(), 135.0);
        let ids: Vec<_> = d.objects()[d.objects().len() - 2..].iter().map(|o| o.id).collect();
        let opts = ShapeOptions {
            overlap_mm: if op == ShapeOp::Divide { 0.5 } else { 0.0 },
            ..ShapeOptions::default()
        };
        d.shape(&ids, op, opts).expect("shaping");
    }
    d
}

/// P6 Milestone 3: offsets and arrangements. Two outlines round a square,
/// a satin border round a square, an outline round a line (open offset),
/// mirror-merge (both), an array, a wreath, a kaleidoscope, and two squares
/// with the overlap removed.
fn arrangements() -> Design {
    use emb_geom::region::Join;
    use emb_model::{Arrangement, OffsetMake, OffsetSpec};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let last = |d: &Design| d.objects().last().map(|o| o.id).expect("object");
    let square = |d: &mut Design, t: ThreadId, x: f64, y: f64, s: f64| {
        fill(d, t, Path::rect(BBox::new(p(x, y), p(x + s, y + s))), Vec::new(), 45.0);
    };
    let triangle = |d: &mut Design, t: ThreadId, c: Point, s: f64| {
        let pts = [c + p(0.0, -s), c + p(s * 0.87, s * 0.5), c + p(-s * 0.87, s * 0.5)];
        fill(d, t, Path::polygon(&pts), Vec::new(), 0.0);
    };
    // Offsets.
    square(&mut d, navy, -110.0, -85.0, 16.0);
    let a = last(&d);
    let lines = OffsetSpec {
        distance_mm: 2.0,
        count: 2,
        spacing_mm: 2.5,
        corners: Join::Round,
        make: OffsetMake::Line,
    };
    d.add_offsets(&[a], &lines).expect("offsets");
    square(&mut d, navy, -70.0, -85.0, 16.0);
    let b = last(&d);
    let border = OffsetSpec {
        distance_mm: 1.0,
        make: OffsetMake::Border {
            width_mm: 3.0,
            place: emb_model::BorderPlace::Outside,
        },
        corners: Join::Miter,
        ..OffsetSpec::default()
    };
    d.add_offsets(&[b], &border).expect("border");
    d.add_object(
        gold,
        ObjectKind::Running {
            path: Path::new().move_to(p(-35.0, -80.0)).line_to(p(-5.0, -65.0)),
            params: RunningParams::default(),
        },
    );
    let line = last(&d);
    d.add_offsets(&[line], &OffsetSpec::default()).expect("open offset");
    // Arrangements.
    triangle(&mut d, gold, p(25.0, -75.0), 8.0);
    let t = last(&d);
    d.arrange(
        &[t],
        &Arrangement::MirrorMerge {
            horizontal: true,
            vertical: true,
            gap_mm: 3.0,
        },
    );
    d.add_object(
        navy,
        ObjectKind::Fill {
            outer: Path::ellipse(p(-105.0, -25.0), 4.0, 4.0),
            holes: Vec::new(),
            params: FillParams::default(),
        },
    );
    let c = last(&d);
    d.arrange(
        &[c],
        &Arrangement::Array {
            rows: 2,
            columns: 4,
            gap_x_mm: 3.0,
            gap_y_mm: 3.0,
        },
    );
    triangle(&mut d, gold, p(60.0, -25.0), 7.0);
    let w = last(&d);
    d.arrange(
        &[w],
        &Arrangement::Wreath {
            count: 6,
            radius_mm: 20.0,
        },
    );
    triangle(&mut d, navy, p(-60.0, 30.0), 6.0);
    let k = last(&d);
    d.arrange(
        &[k],
        &Arrangement::Kaleidoscope {
            count: 5,
            radius_mm: 20.0,
        },
    );
    // Remove overlaps: the gold square is cut out of the navy one.
    square(&mut d, navy, 40.0, 40.0, 20.0);
    square(&mut d, gold, 50.0, 50.0, 20.0);
    let top = last(&d);
    d.remove_overlaps(&[top], 0.5, false).expect("remove overlaps");
    d
}

/// P6 Milestone 4 (owner request): border trace. Three navy squares, each
/// with a gold circle on it: a white 3 mm satin border (circle, then
/// border), a 2 mm gap with no border, and a white 3 mm border with a
/// 1.5 mm gap round it that sews before the circle.
fn border_trace() -> Design {
    use emb_model::{TraceOrder, TraceSpec};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let white = d.add_thread("White", Rgb(245, 245, 240));
    let cases = [
        (3.0, 0.0, TraceOrder::FillThenBorder),
        (0.0, 2.0, TraceOrder::FillThenBorder),
        (3.0, 1.5, TraceOrder::BorderThenFill),
    ];
    for (i, (border, gap, order)) in cases.into_iter().enumerate() {
        let x = -105.0 + 75.0 * i as f64;
        fill(
            &mut d,
            navy,
            Path::rect(BBox::new(p(x, -30.0), p(x + 60.0, 30.0))),
            Vec::new(),
            45.0,
        );
        fill(
            &mut d,
            gold,
            Path::ellipse(p(x + 30.0, 0.0), 16.0, 16.0),
            Vec::new(),
            135.0,
        );
        let circle = d.objects().last().map(|o| o.id).expect("circle");
        let spec = TraceSpec {
            border_mm: border,
            gap_mm: gap,
            stitch: Default::default(),
            thread: white,
            order,
            place: Default::default(),
        };
        d.add_border_trace(&[circle], spec).expect("border trace");
    }
    d
}

fn sewout_names(order: LetterOrder) -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    for (family, text, height, y) in [
        ("Block Sans", "Anna", 25.0, -55.0),
        ("Pacifico", "Anna", 25.0, -10.0),
        ("Block Sans", "Joshua", 10.0, 30.0),
        ("Pacifico", "Joshua", 10.0, 55.0),
    ] {
        let font = emb_letter::bundled::embroidery(family).expect("bundled font");
        let mut l = Lettering::new(text, emb_letter::LetterSource::font_ref(&font), height, p(0.0, y));
        l.justify = Justify::Center;
        l.order = order;
        emb_letter::relayout(&mut l, &font);
        d.add_object(navy, ObjectKind::Lettering(Box::new(l)));
    }
    d
}

/// Adds built-in monogram template `index` in Block Sans.
fn add_monogram(d: &mut Design, index: usize, initials: &str, height: f64, at: Point) {
    let font = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).expect("bundled font");
    let font_ref = emb_letter::LetterSource::font_ref(&font);
    let mut spec = emb_model::MonogramTemplate::builtin(&font_ref)[index].spec.clone();
    spec.height_mm = height;
    spec.border.width_mm *= height / 30.0;
    spec.border.margin_mm *= height / 30.0;
    spec.ornaments.size_mm *= height / 30.0;
    let m = emb_letter::monogram::build(initials, &spec, &font, at).expect("monogram");
    emb_letter::monogram::add_to_design(d, initials, &spec, m, at, None);
}

/// An appliqué patch (P3 Milestone 7): an oval of fabric with a hole,
/// placement line, stop, tack-down, stop and cover, then a running line
/// in a second color across it.
fn applique_patch() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let hole = Path::ellipse(p(0.0, 0.0), 10.0, 10.0);
    d.add_object(
        navy,
        ObjectKind::Applique {
            pieces: vec![AppliquePiece::new(Path::ellipse(p(0.0, 0.0), 40.0, 28.0), vec![hole])],
            params: AppliqueParams {
                fabric: Some(Rgb(200, 30, 45)),
                ..AppliqueParams::default()
            },
        },
    );
    d.add_object(
        gold,
        ObjectKind::Running {
            path: Path::new().move_to(p(-30.0, 18.0)).line_to(p(30.0, 18.0)),
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    d
}

/// A large multi-color design that fills most of the hoop: a 220 × 160 mm
/// panel with a satin border, three fills on top, and running-stitch detail.
fn full_field() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let white = d.add_thread("White", Rgb(240, 240, 240));
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));

    let panel = BBox::new(p(-110.0, -80.0), p(110.0, 80.0));
    fill(&mut d, navy, Path::rounded_rect(panel, 15.0), Vec::new(), 30.0);
    let border = Path::rounded_rect(BBox::new(p(-108.0, -78.0), p(108.0, 78.0)), 13.0);
    satin(&mut d, white, SatinGeometry::from_centerline(&border, 4.0));
    for (i, x) in [-60.0, 0.0, 60.0].into_iter().enumerate() {
        let t = if i == 1 { gold } else { red };
        fill(
            &mut d,
            t,
            Path::ellipse(p(x, 0.0), 24.0, 24.0),
            Vec::new(),
            120.0 + 30.0 * i as f64,
        );
    }
    for y in [-50.0, 50.0] {
        d.add_object(
            white,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(p(-90.0, y))
                    .cubic_to(p(-30.0, y - 15.0), p(30.0, y + 15.0), p(90.0, y)),
                params: RunningParams {
                    repeats: 3,
                    ..RunningParams::default()
                },
            },
        );
    }
    d
}

/// P6 Milestone 6: auto-trace and click-to-stitch on a picture made in
/// code (180 x 100 px, 0.8 mm a pixel): white, a red ring, a blue bar on a
/// slant and a green box. The trace of the box stays a vector (no
/// stitches); a click fill on the ring (with its hole), a click satin and
/// a click center line on the bar, a click outline on the box.
fn autotrace() -> Design {
    use emb_art::autotrace::{TraceSettings, background, trace, trace_at};
    use emb_art::click::{ClickStitch, objects};
    use emb_art::reduce::Reduced;
    use emb_model::{CurrentProperties, VectorStyle};
    let (w, h) = (180u32, 100u32);
    let labels = (0..h)
        .flat_map(|y| (0..w).map(move |x| (f64::from(x) + 0.5, f64::from(y) + 0.5)))
        .map(|(x, y)| {
            let r = (x - 40.0).hypot(y - 50.0);
            // The distance to the middle line of the bar, from (85, 30) to (170, 10).
            let (a, b) = (p(85.0, 30.0), p(170.0, 10.0));
            let bar = p(x, y).distance_to_segment(a, b);
            if (14.0..32.0).contains(&r) {
                1
            } else if bar < 4.0 {
                2
            } else if (100.0..170.0).contains(&x) && (60.0..92.0).contains(&y) {
                3
            } else {
                0
            }
        })
        .collect();
    let palette = vec![Rgb(255, 255, 255), Rgb(200, 30, 45), Rgb(30, 90, 190), Rgb(40, 150, 70)];
    let r = Reduced {
        width: w,
        height: h,
        palette: palette.clone(),
        labels,
    };
    let to_mm = Transform::scale(0.8, 0.8).then(Transform::translate(-72.0, -40.0));
    let s = TraceSettings::default();
    let mut d = Design::new("");
    let threads: Vec<ThreadId> = ["White", "Red", "Blue", "Green"]
        .iter()
        .zip(&palette)
        .map(|(n, c)| d.add_thread(*n, *c))
        .collect();
    let bg = background(&r);
    for a in trace(&r, &s)
        .into_iter()
        .filter(|a| Some(a.color) != bg && a.color == 3)
    {
        let a = a.transformed(&to_mm);
        d.add_object(
            threads[3],
            ObjectKind::Vector {
                path: a.path(),
                style: VectorStyle::filled(palette[3]),
            },
        );
    }
    let current = CurrentProperties::default();
    for (x, y, how) in [
        (12, 50, ClickStitch::Fill),
        (128, 20, ClickStitch::Satin),
        (128, 20, ClickStitch::CenterLine),
        (130, 75, ClickStitch::Outline),
    ] {
        let a = trace_at(&r, x, y, &s).expect("a color area").transformed(&to_mm);
        for kind in objects(&a.outer, &a.holes, how, &current) {
            d.add_object(threads[a.color as usize], kind);
        }
    }
    d
}

/// P6 Milestone 7: the steps of the Picture to design wizard on a "photo"
/// made in code (240 x 140 px): a wall that shades from dark to light with
/// noise, a navy ring, a green leaf and a narrow gold bar. The edge color
/// is removed, and the wall in the ring with a click (as a user does); the
/// picture is trimmed and simplified to 0.5 mm a pixel and
/// three colors, and digitized with Auto stitches (fills, and a satin for
/// the bar), the gold last.
fn smart_logo() -> Design {
    use emb_art::reduce::{ReduceSpec, reduce};
    use emb_art::smart::{ColorChoice, SmartSpec, digitize, resample};
    use emb_art::{Rgba, RgbaImage, background};
    use emb_model::CurrentProperties;
    let mut img = RgbaImage::from_fn(240, 140, |x, y| {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let noise = ((x * 7 + y * 13) % 7) as u8;
        let r = (fx - 60.0).hypot(fy - 70.0);
        // A leaf: two circles that overlap.
        let leaf = (fx - 140.0).hypot(fy - 85.0) < 40.0 && (fx - 140.0).hypot(fy - 35.0) < 40.0;
        if (22.0..42.0).contains(&r) {
            Rgba([25, 40, 100 + noise, 255])
        } else if leaf {
            Rgba([40, 150, 70 + noise, 255])
        } else if (175..230).contains(&x) && (66..74).contains(&y) {
            // A bar 4 mm wide: a satin (Auto measures the true width since
            // P12 Milestone 1; the bar was 6 mm, and the old measure saw 4.9 mm).
            Rgba([230, 170, 30 + noise, 255])
        } else {
            let g = 150 + (x * 60 / 240) as u8 + noise;
            Rgba([g, g, g - 6, 255])
        }
    });
    background::remove_edges(&mut img, background::DEFAULT_TOLERANCE);
    background::remove_at(&mut img, 60, 70, background::DEFAULT_TOLERANCE, true);
    let (img, at) = background::trim(&img).expect("the shapes stay");
    // 0.5 mm a picture pixel; simplified to 0.5 mm a pixel (the same size).
    let width_mm = f64::from(img.width()) * 0.5;
    let small = resample(&img, img.width());
    let r = reduce(
        &small,
        &ReduceSpec {
            colors: 3,
            min_speck_px: 4,
            clean_edges: true,
        },
    );
    let choices = vec![ColorChoice::default(); r.palette.len()];
    // The gold (the smallest area) sews last.
    let order: Vec<u16> = (0..r.palette.len() as u16).collect();
    let to_mm = Transform::scale(0.5, 0.5)
        .then(Transform::translate(f64::from(at.0) * 0.5, f64::from(at.1) * 0.5))
        .then(Transform::translate(
            -width_mm / 2.0 - f64::from(at.0) * 0.5,
            -35.0 - f64::from(at.1) * 0.5,
        ));
    let objs = digitize(
        &r,
        &to_mm,
        &choices,
        &order,
        &SmartSpec::default(),
        &CurrentProperties::default(),
    );
    let mut d = Design::new("");
    let threads: Vec<ThreadId> = r.palette.iter().map(|c| d.add_thread(c.to_hex(), *c)).collect();
    for o in objs {
        d.add_object(threads[o.color as usize], o.kind);
    }
    d
}

/// A "portrait" made in code (120 x 140 px, clear round it): an oval
/// face lit from the upper left, dark hair, eyes and a mouth.
pub fn portrait() -> emb_art::RgbaImage {
    use emb_art::{Rgba, RgbaImage};
    RgbaImage::from_fn(120, 140, |x, y| {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let (dx, dy) = ((fx - 60.0) / 50.0, (fy - 75.0) / 62.0);
        let r2 = dx * dx + dy * dy;
        if r2 > 1.0 {
            return Rgba([0, 0, 0, 0]);
        }
        // Light from the upper left on a ball.
        let z = (1.0 - r2).sqrt();
        let light = (0.25 + 0.75 * (-0.45 * dx - 0.45 * dy + 0.77 * z).max(0.0)).min(1.0);
        let skin = |k: f64| Rgba([(235.0 * k) as u8, (190.0 * k) as u8, (160.0 * k) as u8, 255]);
        let eye = ((fx - 42.0) / 9.0).powi(2) + ((fy - 66.0) / 5.0).powi(2) < 1.0
            || ((fx - 78.0) / 9.0).powi(2) + ((fy - 66.0) / 5.0).powi(2) < 1.0;
        let mouth = ((fx - 60.0) / 20.0).powi(2) + ((fy - 98.0) / 8.0).powi(2) < 1.0 && fy > 98.0;
        if fy < 40.0 + 10.0 * (fx - 60.0).abs() / 50.0 {
            Rgba([60, 40, 30, 255])
        } else if eye {
            Rgba([30, 30, 40, 255])
        } else if mouth {
            Rgba([150, 50, 50, 255])
        } else {
            skin(light)
        }
    })
}

/// P6 Milestone 8: photo stitch of the portrait (60 x 70 mm each): rows in
/// one thread, color rows in four threads, and a scribble.
fn photo() -> Design {
    use emb_art::photo::{PhotoSpec, PhotoStyle, photo};
    let img = portrait();
    let mut d = Design::new("");
    for (k, style) in [PhotoStyle::Rows, PhotoStyle::ColorRows, PhotoStyle::Scribble]
        .into_iter()
        .enumerate()
    {
        let to_mm = Transform::scale(0.5, 0.5).then(Transform::translate(-100.0 + 70.0 * k as f64, -35.0));
        let spec = PhotoSpec {
            style,
            angle_deg: if style == PhotoStyle::Rows { 15.0 } else { 0.0 },
            ..PhotoSpec::default()
        };
        for o in photo(&img, &to_mm, &spec) {
            let t = d.thread_for_color(o.color);
            d.add_object(t, o.kind);
        }
    }
    d
}

/// P7 Milestone 1: a turned hoop with auto start and end (ADR 0032). A
/// bar on a 35° slant with a ring at its middle; the hoop turns −55°, so the
/// bar goes from the front to the back in the machine file (the long side
/// of the hoop). The needle starts at the front end of the design and ends
/// at the hoop center.
fn hoop_placement() -> Design {
    let mut d = Design::new("");
    let blue = d.add_thread("Blue", Rgb(30, 60, 170));
    let gold = d.add_thread("Gold", Rgb(220, 170, 40));
    let turn = Transform::rotate_deg(35.0);
    let bar = Path::rect(BBox::from_center_size(p(0.0, 0.0), 240.0, 30.0)).transformed(&turn);
    fill(&mut d, blue, bar, vec![], 35.0);
    satin(
        &mut d,
        gold,
        SatinGeometry {
            rail_a: Path::ellipse(p(0.0, 0.0), 24.0, 24.0),
            rail_b: Path::ellipse(p(0.0, 0.0), 19.0, 19.0),
            rungs: vec![],
        },
    );
    // The bar is along the long side of the turned hoop.
    d.hoop.angle_deg = -55.0;
    d.start_end = emb_model::StartEnd {
        start: Some(emb_model::Anchor::DesignBox { x: 0.5, y: 1.0 }),
        end: Some(emb_model::Anchor::HoopCenter),
    };
    d
}

/// P7 Milestone 2: a 360 × 260 mm design for the 260 × 200 hoop, in four
/// hoopings (the P7 exit test): a frame (a fill 18 mm wide), a star (a
/// fill) in the middle, a running-stitch circle round the star, and a
/// satin ring near each corner (each one fits a hooping whole).
fn multi_hoop() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(20, 40, 110));
    let red = d.add_thread("Red", Rgb(200, 30, 40));
    let gold = d.add_thread("Gold", Rgb(220, 170, 40));
    let frame = Path::rect(BBox::from_center_size(p(0.0, 0.0), 360.0, 260.0));
    let inside = Path::rect(BBox::from_center_size(p(0.0, 0.0), 324.0, 224.0));
    fill(&mut d, navy, frame, vec![inside], 45.0);
    let star: Vec<Point> = (0..10)
        .map(|k| {
            let a = emb_geom::math::to_radians(-90.0 + 36.0 * k as f64);
            let r = if k % 2 == 0 { 95.0 } else { 40.0 };
            p(r * emb_geom::math::cos(a), r * emb_geom::math::sin(a))
        })
        .collect();
    fill(&mut d, red, Path::polygon(&star), vec![], 0.0);
    d.add_object(
        gold,
        ObjectKind::Running {
            path: Path::ellipse(p(0.0, 0.0), 104.0, 104.0),
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    for (x, y) in [(-135.0, -85.0), (135.0, -85.0), (-135.0, 85.0), (135.0, 85.0)] {
        satin(
            &mut d,
            gold,
            SatinGeometry {
                rail_a: Path::ellipse(p(x, y), 12.0, 12.0),
                rail_b: Path::ellipse(p(x, y), 8.0, 8.0),
                rungs: vec![],
            },
        );
    }
    let hoop = emb_model::HoopCatalog::builtin().get(HOOP).cloned();
    d.hooping = hoop.and_then(|h| emb_model::HoopingLayout::auto(&d, &h, 20.0, 5.0));
    d
}

/// P7 Milestone 6: a continuous border. One motif, 60 mm long: a triple
/// run wave from its left end to its right end and a diamond fill in the
/// middle; auto start at the left and auto end at the right; the step of
/// the repeats from the start to the end ("join end to start"); four
/// copies made as objects (240 mm). The wave of each copy starts where the
/// wave before it ends.
fn repeats() -> Design {
    let mut d = Design::new("");
    let green = d.add_thread("Green", Rgb(20, 120, 60));
    let gold = d.add_thread("Gold", Rgb(220, 170, 40));
    let x0 = -120.0;
    let wave: Vec<(Point, bool)> = (0..=12)
        .map(|k| {
            let x = 5.0 * k as f64;
            (
                p(
                    x0 + x,
                    -8.0 * emb_geom::math::sin(emb_geom::math::to_radians(30.0 * k as f64)),
                ),
                true,
            )
        })
        .collect();
    d.add_object(
        green,
        ObjectKind::Running {
            path: Path::through_points(&wave, false),
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    let diamond = [
        p(x0 + 30.0, -9.0),
        p(x0 + 39.0, 0.0),
        p(x0 + 30.0, 9.0),
        p(x0 + 21.0, 0.0),
    ];
    fill(&mut d, gold, Path::polygon(&diamond), vec![], 0.0);
    d.start_end = emb_model::StartEnd {
        start: Some(emb_model::Anchor::DesignBox { x: 0.0, y: 0.5 }),
        end: Some(emb_model::Anchor::DesignBox { x: 1.0, y: 0.5 }),
    };
    let step = d.join_step().unwrap_or(p(60.0, 0.0));
    d.repeat = Some(emb_model::RepeatSpec {
        columns: 4,
        rows: 1,
        step_x: step,
        step_y: Point::ZERO,
        show: true,
    });
    d.make_repeats();
    d
}

/// A cross-stitch heart on a 2.5 mm grid (P9 Milestone 1): full crosses,
/// three-quarter crosses at the lower sides, quarter crosses at the top,
/// half crosses for a highlight, a backstitch frame, and a green bar sewn
/// in rows (all top legs the same way).
fn cross_stitch() -> Design {
    use emb_model::cross::{Corner, CrossOrder, CrossStitch, Part};
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(200, 30, 45));
    let white = d.add_thread("White", Rgb(245, 245, 245));
    let dark = d.add_thread("Dark red", Rgb(110, 15, 25));
    let green = d.add_thread("Green", Rgb(30, 130, 60));
    let heart = [
        "..XXX...XXX..",
        ".XXXXX.XXXXX.",
        "XXXXXXXXXXXXX",
        "XXXXXXXXXXXXX",
        "XXXXXXXXXXXXX",
        ".XXXXXXXXXXX.",
        "..XXXXXXXXX..",
        "...XXXXXXX...",
        "....XXXXX....",
        ".....XXX.....",
        "......X......",
    ];
    let grid = CrossStitch::new(p(-16.25, -20.0), 2.5);
    let mut cells = Vec::new();
    for (r, row) in heart.iter().enumerate() {
        let cols: Vec<i32> = row
            .char_indices()
            .filter(|(_, c)| *c == 'X')
            .map(|(i, _)| i as i32)
            .collect();
        for &c in &cols {
            cells.push((c, r as i32, Part::Full));
        }
        // The lower sides of the heart: three-quarter crosses outside the edge.
        if r >= 5 {
            let (first, last) = (cols[0], cols[cols.len() - 1]);
            cells.push((first - 1, r as i32, Part::ThreeQuarter(Corner::TopRight)));
            cells.push((last + 1, r as i32, Part::ThreeQuarter(Corner::TopLeft)));
        }
    }
    // The dip and the outer corners of the top: quarter crosses.
    cells.push((6, 0, Part::Quarter(Corner::BottomLeft)));
    cells.push((1, 0, Part::Quarter(Corner::BottomRight)));
    cells.push((11, 0, Part::Quarter(Corner::BottomLeft)));
    d.cross_paint(&grid, red, &cells);
    d.cross_paint(
        &grid,
        white,
        &[(2, 2, Part::HalfUp), (3, 2, Part::HalfUp), (2, 3, Part::HalfUp)],
    );
    // A backstitch frame one cell outside the heart (half-grid points).
    let frame = [[-4, -2], [30, -2], [30, 24], [-4, 24]];
    for k in 0..4 {
        d.cross_add_line(&grid, dark, frame[k], frame[(k + 1) % 4]);
    }
    // A zigzag bar under the frame.
    let bar: Vec<(i32, i32, Part)> = (0..13)
        .flat_map(|c| [(c, 14, Part::Full), (c, 15, Part::Full)])
        .collect();
    if let Some(id) = d.cross_paint(&grid, green, &bar)
        && let Some(ObjectKind::CrossStitch(x)) = d.object_mut(id).map(|o| &mut o.kind)
    {
        x.params.order = CrossOrder::Rows;
    }
    d
}

/// P9 Milestone 2: the portrait (60 x 70 mm) as cross stitch on a 2.5 mm
/// grid, in 6 colors, with three-quarter crosses at the edges and no
/// groups of fewer than 3 cells.
fn cross_picture() -> Design {
    use emb_art::cross::{CrossSpec, cross_stitch};
    use emb_model::cross::CrossStitch;
    let mut d = Design::new("");
    let img = portrait();
    // 0.5 mm for each pixel, centered on the origin.
    let t = Transform::scale(0.5, 0.5).then(Transform::translate(-30.0, -35.0));
    let grid = CrossStitch::new(Point::ZERO, 2.5);
    let spec = CrossSpec {
        colors: 6,
        ..CrossSpec::default()
    };
    for c in cross_stitch(&img, &t, &grid, &spec) {
        let th = d.thread_for_color(c.color);
        d.add_object(th, ObjectKind::CrossStitch(Box::new(c.cross)));
    }
    d
}

/// P9 Milestone 3: rhinestones. A sewn running-stitch frame; inside it the
/// word "BLING" as stones along the center lines (SS10 crystal), a
/// heart filled with stones in rows (SS6 red), a circle filled in rings
/// (SS8 gold), a run round the circle (SS16 black) and three single
/// stones (SS20 blue). No stone overlaps another.
fn rhinestones() -> Design {
    use emb_model::bling::{Bling, BlingFill, BlingLayout, STONE_COLORS, Stone};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let frame = Path::rect(BBox::new(p(-60.0, -40.0), p(60.0, 40.0)));
    d.add_object(
        navy,
        ObjectKind::Running {
            path: frame,
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    let stone = |ss: u32, color: usize| Stone {
        ss,
        color: STONE_COLORS[color].1,
        name: STONE_COLORS[color].0.into(),
    };
    let mut add = |stone: Stone, layout: BlingLayout| {
        d.add_object(navy, ObjectKind::Bling(Box::new(Bling::new(stone, layout))));
    };
    // The word, as one line of stones along the middle of each stroke.
    let font = emb_letter::bundled::font("Anton", "Regular").expect("bundled font");
    let mut l = Lettering::new("BLING", font.font_ref(), 25.0, p(0.0, -8.0));
    l.justify = Justify::Center;
    emb_letter::relayout(&mut l, &font);
    if let Some(ObjectKind::Vector { path, .. }) =
        emb_model::vector::to_vector(&ObjectKind::Lettering(Box::new(l)), Rgb(0, 0, 0))
    {
        let subs: Vec<Path> = path.subpaths().into_iter().map(Path::closed).collect();
        let current = emb_model::CurrentProperties::default();
        let lines: Vec<Path> = emb_geom::region::nest(&subs)
            .iter()
            .flat_map(|(o, hs)| emb_art::click::objects(o, hs, emb_art::click::ClickStitch::CenterLine, &current))
            .filter_map(|k| match k {
                ObjectKind::Running { path, .. } => Some(path),
                _ => None,
            })
            .collect();
        add(
            stone(10, 0),
            BlingLayout::Run {
                path: Path::joined(&lines),
            },
        );
    }
    // A heart in rows: the usual heart curve, 22 mm wide.
    let heart: Vec<Point> = (0..72)
        .map(|k| {
            let t = emb_geom::math::to_radians(5.0 * f64::from(k));
            let (s, c) = (emb_geom::math::sin(t), emb_geom::math::cos(t));
            let c2 = emb_geom::math::cos(2.0 * t);
            let c3 = emb_geom::math::cos(3.0 * t);
            let c4 = emb_geom::math::cos(4.0 * t);
            p(
                -38.0 + 0.7 * 16.0 * s * s * s,
                10.0 - 0.7 * (13.0 * c - 5.0 * c2 - 2.0 * c3 - c4),
            )
        })
        .collect();
    let heart = Path::polygon(&heart);
    add(
        stone(6, 4),
        BlingLayout::Fill {
            outer: heart,
            holes: Vec::new(),
            fill: BlingFill::Rows { angle_deg: 0.0 },
        },
    );
    // A circle in rings, and a run round it.
    add(
        stone(8, 2),
        BlingLayout::Fill {
            outer: Path::ellipse(p(38.0, 18.0), 12.0, 12.0),
            holes: Vec::new(),
            fill: BlingFill::Rings,
        },
    );
    add(
        stone(16, 1),
        BlingLayout::Run {
            path: Path::ellipse(p(38.0, 18.0), 16.0, 16.0),
        },
    );
    add(
        stone(20, 6),
        BlingLayout::Stones {
            points: vec![p(-10.0, 25.0), p(0.0, 28.0), p(10.0, 25.0)],
        },
    );
    // P9 Milestone 5: the sewn marks for the template, last.
    d.add_template_marks();
    d
}

/// The id of the test hoop with two positions (not in `data/hoops.toml`).
pub const TEST_POSITION_HOOP: &str = "test_two_positions";

/// A test hoop of 300 × 150 mm in two positions of 160 × 150 mm, 140 mm
/// apart (made up for the tests; not a real product).
pub fn test_position_hoop() -> emb_model::HoopPreset {
    let at = |x: f64| emb_model::HoopPosition {
        x_mm: x,
        y_mm: 0.0,
        width_mm: 160.0,
        height_mm: 150.0,
    };
    emb_model::HoopPreset {
        id: TEST_POSITION_HOOP.into(),
        name: "Test hoop, two positions 300 x 150".into(),
        brand: None,
        width_mm: 300.0,
        height_mm: 150.0,
        shape: emb_model::HoopShape::Rectangle,
        corner_radius_mm: None,
        max_stitch_length_mm: None,
        positions: vec![at(-70.0), at(70.0)],
        cap: false,
    }
}

/// A hoop by its id: a built-in hoop or the test hoop with two positions.
pub fn hoop(id: &str) -> Option<emb_model::HoopPreset> {
    emb_model::HoopCatalog::builtin()
        .get(id)
        .cloned()
        .or_else(|| (id == TEST_POSITION_HOOP).then(test_position_hoop))
}

/// P7 Milestone 4: a 280 × 120 mm banner on the test hoop with two
/// positions: an oval fill with a hole, and a running-stitch wave across.
/// Each position sews its half; the fabric stays in the hoop.
fn multi_position() -> Design {
    let mut d = Design::new("");
    let teal = d.add_thread("Teal", Rgb(0, 130, 130));
    let white = d.add_thread("White", Rgb(245, 245, 245));
    let oval = Path::ellipse(p(0.0, 0.0), 140.0, 60.0);
    let hole = Path::ellipse(p(0.0, 0.0), 110.0, 35.0);
    fill(&mut d, teal, oval, vec![hole], 90.0);
    let wave: Vec<(Point, bool)> = (0..=24)
        .map(|k| {
            let x = -120.0 + 10.0 * k as f64;
            (
                p(
                    x,
                    15.0 * emb_geom::math::sin(emb_geom::math::to_radians(30.0 * k as f64)),
                ),
                true,
            )
        })
        .collect();
    d.add_object(
        white,
        ObjectKind::Running {
            path: Path::through_points(&wave, false),
            params: RunningParams {
                repeats: 3,
                ..RunningParams::default()
            },
        },
    );
    d.hooping = emb_model::HoopingLayout::positions(&d, &test_position_hoop(), 5.0);
    d
}

/// P11 Milestone 6: echo quilting round a heart in a 100 mm square (a
/// border vector), and a stipple background round a star in a second
/// square below it.
fn quilting() -> Design {
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(190, 30, 50));
    let cream = d.add_thread("Cream", Rgb(240, 230, 200));
    let square = |cy: f64| Path::rect(BBox::from_center_size(p(0.0, cy), 100.0, 100.0));
    let border = |d: &mut Design, cy: f64| {
        d.add_object(
            cream,
            ObjectKind::Vector {
                path: square(cy),
                style: emb_model::VectorStyle::stroked(Rgb(120, 120, 120)),
            },
        )
    };
    let heart = Path::new()
        .move_to(p(0.0, -32.0))
        .cubic_to(p(-30.0, -52.0), p(-30.0, -82.0), p(0.0, -70.0))
        .cubic_to(p(30.0, -82.0), p(30.0, -52.0), p(0.0, -32.0))
        .close();
    let heart = d.add_object(
        red,
        ObjectKind::Fill {
            outer: heart,
            holes: Vec::new(),
            params: FillParams::default(),
        },
    );
    let top = border(&mut d, -55.0);
    let spec = emb_model::quilt::EchoSpec {
        distance_mm: 3.0,
        spacing_mm: 6.0,
        rows: 8,
    };
    let echo = d.echo_path(&[heart, top], &spec).expect("echo lines");
    d.add_object(
        cream,
        ObjectKind::Running {
            path: echo,
            params: RunningParams {
                stitch_length_mm: 3.0,
                ..RunningParams::default()
            },
        },
    );
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 32.0 } else { 14.0 };
            let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 36.0 - 90.0);
            p(r * c, 55.0 + r * s)
        })
        .collect();
    let star = d.add_object(
        red,
        ObjectKind::Fill {
            outer: Path::polygon(&star),
            holes: Vec::new(),
            params: FillParams::default(),
        },
    );
    let bottom = border(&mut d, 55.0);
    let area = d.background_area(&[star, bottom], 1.5, 0.0).expect("a background");
    d.add_object(
        cream,
        emb_model::quilt::fill_of(
            &area,
            FillParams {
                spacing_mm: 3.0,
                stitch_length_mm: 3.0,
                ..FillParams::default().with_pattern(FillPattern::Stipple { style: RunStyle::Run })
            },
        ),
    );
    d
}

/// P11 Milestone 6: an edge-to-edge quilt block of 300 × 300 mm, a wave
/// pattern repeated over it with a line round it, in four hoopings of the
/// Topaz 20 hoop (the exit test of the milestone).
fn quilt_block() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(30, 40, 90));
    let wave = emb_geom::Polyline {
        points: (0..=8)
            .map(|i| {
                let x = 5.0 * f64::from(i);
                p(x, if i % 2 == 1 { -4.0 } else { 0.0 })
            })
            .collect(),
        closed: false,
    };
    let lines = emb_model::quilt::block_lines(&[wave], p(0.0, 0.0), 300.0, 300.0, p(0.0, 8.0));
    let parts: Vec<Path> = lines
        .iter()
        .map(|l| {
            l.points[1..]
                .iter()
                .fold(Path::new().move_to(l.points[0]), |a, q| a.line_to(*q))
        })
        .collect();
    let line = RunningParams {
        stitch_length_mm: 3.0,
        ..RunningParams::default()
    };
    d.add_object(
        navy,
        ObjectKind::Running {
            path: Path::joined(&parts),
            params: line.clone(),
        },
    );
    d.add_object(
        navy,
        ObjectKind::Running {
            path: Path::rect(BBox::from_center_size(p(0.0, 0.0), 300.0, 300.0)),
            params: line,
        },
    );
    d.hooping = emb_model::HoopingLayout::auto(
        &d,
        &hoop(HOOP).expect("the hoop"),
        emb_model::hooping::DEFAULT_OVERLAP_MM,
        emb_model::hooping::DEFAULT_MARGIN_MM,
    );
    d
}

/// P11 Milestone 7: puff (3D foam) letters "PUFF" in Block Sans 30 mm and
/// a puff satin ring, after a flat line, so the machine stops once to put
/// the foam on.
fn puff() -> Design {
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(200, 30, 40));
    d.add_object(
        red,
        ObjectKind::Running {
            path: Path::rect(BBox::from_center_size(p(0.0, 0.0), 120.0, 80.0)),
            params: RunningParams::default(),
        },
    );
    let font = emb_letter::bundled::embroidery("Block Sans").expect("bundled font");
    let mut l = Lettering::new("PUFF", emb_letter::LetterSource::font_ref(&font), 30.0, p(0.0, -5.0));
    l.justify = Justify::Center;
    l.satin.puff = true;
    l.satin.spacing_mm = emb_model::object::PUFF_SPACING_MM;
    emb_letter::relayout(&mut l, &font);
    d.add_object(red, ObjectKind::Lettering(Box::new(l)));
    d.add_object(
        red,
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&Path::ellipse(p(0.0, 25.0), 12.0, 6.0), 5.0),
            params: SatinParams {
                puff: true,
                spacing_mm: emb_model::object::PUFF_SPACING_MM,
                ..SatinParams::default()
            },
        },
    );
    d
}

/// P11 Milestone 7: a cap front in cap order: a star over "TEAM" (Block
/// Sans 15 mm, from the middle letter out) and two dots at the sides, made
/// in a mixed order and then put in cap order.
fn cap_order() -> Design {
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let dot = |x: f64| Path::ellipse(p(x, 10.0), 5.0, 5.0);
    let fill = |path: Path| ObjectKind::Fill {
        outer: path,
        holes: Vec::new(),
        params: FillParams::default(),
    };
    d.add_object(navy, fill(dot(-55.0)));
    d.add_object(navy, fill(dot(55.0)));
    let font = emb_letter::bundled::embroidery("Block Sans").expect("bundled font");
    let mut l = Lettering::new("TEAM", emb_letter::LetterSource::font_ref(&font), 15.0, p(0.0, 20.0));
    l.justify = Justify::Center;
    l.order = LetterOrder::CenterOut;
    emb_letter::relayout(&mut l, &font);
    d.add_object(navy, ObjectKind::Lettering(Box::new(l)));
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 14.0 } else { 6.0 };
            let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 36.0 - 90.0);
            p(r * c, -12.0 + r * s)
        })
        .collect();
    d.add_object(gold, fill(Path::polygon(&star)));
    let order = emb_stitch::order::cap_order(&d);
    d.sequence_in_order(&order);
    d
}

/// P11 Milestone 8: cutwork for the Husqvarna Viking kit: a running line
/// A badge from a parametric sketch (P14 Milestone 4, ADR 0075): two
/// circles round one center and a hexagon in them, fully constrained by
/// dimensions. Its regions become a satin ring, a fill with the hexagon as
/// a hole, and a fill of the hexagon (an island); a satin line goes along a
/// construction line under the hexagon. The sketch object stays in the
/// design and sews nothing; the objects are linked to it (Milestone 5), so
/// a change of its dimensions makes them again.
fn sketch_badge() -> Design {
    use emb_model::CurrentProperties;
    use emb_model::LinkSource;
    use emb_model::sketch::{CurveTo, RegionTo, SketchObject};
    use emb_sketch::{Constraint, Geometry, Sketch};
    let mut d = Design::new("");
    let navy = d.add_thread("Navy", Rgb(25, 40, 100));
    let gold = d.add_thread("Gold", Rgb(230, 170, 30));
    let white = d.add_thread("White", Rgb(245, 245, 245));
    let mut s = Sketch::new();
    let outer = s.add_circle(p(0.0, 0.0), 44.0).unwrap();
    let Geometry::Circle { center, .. } = s.curve(outer).unwrap().geometry else {
        unreachable!()
    };
    s.add_constraint(Constraint::Fix {
        point: center,
        at: p(0.0, 0.0),
    })
    .unwrap();
    let inner = s.add_circle(p(0.0, 0.0), 39.0).unwrap();
    s.add_constraint(Constraint::Concentric { a: outer, b: inner }).unwrap();
    s.add_constraint(Constraint::Diameter { curve: outer, mm: 90.0 })
        .unwrap();
    s.add_constraint(Constraint::Diameter { curve: inner, mm: 78.0 })
        .unwrap();
    // A hexagon of 16 mm round the center, a side at the bottom.
    let hex: Vec<_> = (0..6)
        .map(|i| {
            let a = std::f64::consts::PI / 3.0 * f64::from(i) + std::f64::consts::PI / 6.0;
            s.add_point(p(15.0 * a.cos(), 15.0 * a.sin()))
        })
        .collect();
    let sides: Vec<_> = (0..6)
        .map(|i| s.add_line_between(hex[i], hex[(i + 1) % 6]).unwrap())
        .collect();
    let ring = s.add_circle(p(0.0, 0.0), 16.0).unwrap();
    s.set_construction(ring, true);
    let Geometry::Circle { center: rc, .. } = s.curve(ring).unwrap().geometry else {
        unreachable!()
    };
    s.add_constraint(Constraint::Coincident { a: rc, b: center }).unwrap();
    s.add_constraint(Constraint::Radius { curve: ring, mm: 16.0 }).unwrap();
    for q in &hex {
        s.add_constraint(Constraint::OnCurve { point: *q, curve: ring })
            .unwrap();
    }
    for w in sides.windows(2) {
        s.add_constraint(Constraint::Equal { a: w[0], b: w[1] }).unwrap();
    }
    s.add_constraint(Constraint::Vertical { line: sides[1] }).unwrap();
    // A construction line under the hexagon for a satin line.
    let bar = s.add_line(p(-22.0, 26.0), p(22.0, 26.0));
    s.set_construction(bar, true);
    s.add_constraint(Constraint::Horizontal { line: bar }).unwrap();
    assert!(s.solve().converged, "the badge solves");
    let sketch = SketchObject::new(s);
    let current = CurrentProperties::default();
    let regions = sketch.regions();
    assert_eq!(regions.len(), 3, "the ring, the disc with its hole, the hexagon");
    // The objects are linked to the sketch (ADR 0076); the sketch goes last
    // in the sew order.
    let sk = d.add_object(navy, ObjectKind::Sketch(Box::new(sketch.clone())));
    for r in &regions {
        let (to, t) = if r.curves.contains(&outer) {
            // The ring: a satin column round it.
            (RegionTo::Satin, gold)
        } else if !r.holes.is_empty() {
            // The disc with the hexagon as a hole.
            (RegionTo::Fill, navy)
        } else {
            (RegionTo::Fill, white)
        };
        let source = LinkSource::Region {
            key: sketch.region_key(r),
            to,
            width_mm: 4.0,
            color: Rgb(0, 0, 0),
        };
        d.make_from_sketch(sk, source, &current, t);
    }
    let source = LinkSource::Curve {
        curve: bar,
        to: CurveTo::SatinLine(3.0),
    };
    d.make_from_sketch(sk, source, &current, gold);
    d.move_object(sk, d.objects().len());
    d
}

/// round a circle and a leaf, the cut lines of the four needles, then satin
/// borders over the cut edges.
fn cutwork() -> Design {
    let mut d = Design::new("");
    let white = d.add_thread("White", Rgb(245, 245, 240));
    let leaf = Path::new()
        .move_to(p(10.0, 0.0))
        .cubic_to(p(20.0, -12.0), p(40.0, -12.0), p(50.0, 0.0))
        .cubic_to(p(40.0, 12.0), p(20.0, 12.0), p(10.0, 0.0))
        .close();
    let shapes = [Path::ellipse(p(-25.0, 0.0), 15.0, 15.0), leaf];
    for s in &shapes {
        d.add_object(
            white,
            ObjectKind::Running {
                path: s.clone(),
                params: RunningParams::default(),
            },
        );
    }
    let cuts: Vec<emb_model::ObjectId> = shapes
        .iter()
        .map(|s| {
            d.add_object(
                white,
                ObjectKind::Running {
                    path: s.clone(),
                    params: RunningParams::default(),
                },
            )
        })
        .collect();
    d.make_cutwork(&cuts, emb_model::cutwork::CutBrand::Husqvarna, 1.0);
    for s in &shapes {
        d.add_object(
            white,
            ObjectKind::Running {
                path: s.clone(),
                params: RunningParams {
                    border: Some(LineBorder::new(3.0, ColumnPattern::Satin)),
                    ..RunningParams::default()
                },
            },
        );
    }
    d
}

/// Adds the layers of a color fill (a tartan or a gradient): one fill of
/// `outer` for each `(thread, settings)`, in one group.
fn add_color_fill(d: &mut Design, outer: &Path, layers: Vec<(ThreadId, FillParams)>) {
    let g = d.new_group_id();
    for (t, params) in layers {
        let id = d.add_object(
            t,
            ObjectKind::Fill {
                outer: outer.clone(),
                holes: Vec::new(),
                params,
            },
        );
        if let Some(o) = d.object_mut(id) {
            o.group = Some(g);
        }
    }
}

/// The threads of the colors of a sett (by their codes).
fn sett_threads(d: &mut Design, sett: &emb_model::Sett) -> Vec<ThreadId> {
    sett.colors
        .iter()
        .map(|c| {
            let (name, rgb) = emb_model::tartan::code_color(c);
            d.add_thread(name, rgb)
        })
        .collect()
}

/// P11 Milestone 4: a tartan of four colors (a mirrored sett) on a 90 mm
/// square, and a two-color check turned 45° on a 60 mm circle.
fn tartan_fill() -> Design {
    let mut d = Design::new("");
    let sett = emb_model::Sett::parse("K4 R24 K4 G16 Y2 G16", 0.5, true).expect("a sett");
    let threads = sett_threads(&mut d, &sett);
    let square = Path::rounded_rect(BBox::new(p(-95.0, -45.0), p(-5.0, 45.0)), 6.0);
    let base = FillParams {
        angle_deg: 0.0,
        ..FillParams::default()
    };
    let layers = emb_model::tartan::tartan_layers(&base, &sett);
    add_color_fill(&mut d, &square, threads.into_iter().zip(layers).collect());
    let check = emb_model::Sett::parse("W8 B8", 0.5, false).expect("a sett");
    let threads = sett_threads(&mut d, &check);
    let base = FillParams {
        angle_deg: 45.0,
        ..FillParams::default()
    };
    let layers = emb_model::tartan::tartan_layers(&base, &check);
    add_color_fill(
        &mut d,
        &Path::ellipse(p(50.0, 0.0), 30.0, 30.0),
        threads.into_iter().zip(layers).collect(),
    );
    d
}

/// P11 Milestone 4: a gradient of three colors across a 100 × 50 mm
/// rectangle, one of two colors at 45° in an oval, and one color that
/// fades to the fabric (a gradient fill of one color alone).
fn gradient_fill() -> Design {
    let mut d = Design::new("");
    let blue = d.add_thread("Blue", Rgb(30, 60, 160));
    let violet = d.add_thread("Violet", Rgb(130, 50, 150));
    let pink = d.add_thread("Pink", Rgb(230, 100, 150));
    let yellow = d.add_thread("Yellow", Rgb(240, 200, 40));
    let level = FillParams {
        angle_deg: 0.0,
        ..FillParams::default()
    };
    let layers = emb_model::tartan::gradient_layers(&level, None, &[0.0, 0.5, 1.0]);
    add_color_fill(
        &mut d,
        &Path::rect(BBox::new(p(-50.0, -80.0), p(50.0, -30.0))),
        [blue, violet, pink].into_iter().zip(layers).collect(),
    );
    let turned = FillParams {
        angle_deg: 45.0,
        ..FillParams::default()
    };
    let layers = emb_model::tartan::gradient_layers(&turned, None, &[0.2, 0.8]);
    add_color_fill(
        &mut d,
        &Path::ellipse(p(-30.0, 30.0), 28.0, 22.0),
        [yellow, pink].into_iter().zip(layers).collect(),
    );
    d.add_object(
        blue,
        ObjectKind::Fill {
            outer: Path::rounded_rect(BBox::new(p(15.0, 10.0), p(65.0, 50.0)), 8.0),
            holes: Vec::new(),
            params: level.with_pattern(FillPattern::Gradient {
                line: None,
                stops: vec![0.0, 1.0],
                color: 0,
            }),
        },
    );
    d
}

/// P11 Milestone 4: a meander fill of each tile pattern on a shape about
/// 50 mm across: squares, triangles (turned 30°) and hexagons in a ring.
fn meander_fill() -> Design {
    let mut d = Design::new("");
    let teal = d.add_thread("Teal", Rgb(20, 130, 130));
    let tile = |tile, angle_deg: f64| FillParams {
        angle_deg,
        ..FillParams::default().with_pattern(FillPattern::Meander { tile })
    };
    d.add_object(
        teal,
        ObjectKind::Fill {
            outer: Path::rounded_rect(BBox::new(p(-85.0, -25.0), p(-35.0, 25.0)), 8.0),
            holes: Vec::new(),
            params: tile(emb_model::MeanderTile::Square, 0.0),
        },
    );
    d.add_object(
        teal,
        ObjectKind::Fill {
            outer: Path::ellipse(p(0.0, 0.0), 25.0, 25.0),
            holes: Vec::new(),
            params: tile(emb_model::MeanderTile::Triangle, 30.0),
        },
    );
    d.add_object(
        teal,
        ObjectKind::Fill {
            outer: Path::ellipse(p(60.0, 0.0), 26.0, 26.0),
            holes: vec![Path::ellipse(p(60.0, 0.0), 9.0, 9.0)],
            params: tile(emb_model::MeanderTile::Hexagon, 0.0),
        },
    );
    d
}

/// P11 Milestone 4: circular fills: a 60 mm circle round its middle, a
/// star round a point near one of its tips, and a square with a hole.
fn circular_fill() -> Design {
    let mut d = Design::new("");
    let orange = d.add_thread("Orange", Rgb(230, 120, 30));
    let circular = |center| FillParams::default().with_pattern(FillPattern::Circular { center });
    d.add_object(
        orange,
        ObjectKind::Fill {
            outer: Path::ellipse(p(-65.0, 0.0), 30.0, 30.0),
            holes: Vec::new(),
            params: circular(None),
        },
    );
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 28.0 } else { 12.0 };
            let (s, c) = emb_geom::math::sin_cos_deg(f64::from(i) * 36.0 - 90.0);
            p(r * c, r * s)
        })
        .collect();
    d.add_object(
        orange,
        ObjectKind::Fill {
            outer: Path::polygon(&star),
            holes: Vec::new(),
            params: circular(Some(p(0.0, -18.0))),
        },
    );
    d.add_object(
        orange,
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(40.0, -25.0), p(90.0, 25.0))),
            holes: vec![Path::rect(BBox::new(p(58.0, -7.0), p(72.0, 7.0)))],
            params: circular(None),
        },
    );
    d
}

/// P11 Milestone 4: guided fills: a 100 × 50 mm band whose rows change
/// from a straight line at the top to a wave at the bottom, and a leaf
/// with one curved guide along it.
fn guided_fill() -> Design {
    let mut d = Design::new("");
    let blue = d.add_thread("Sky blue", Rgb(70, 150, 220));
    let green = d.add_thread("Leaf green", Rgb(60, 140, 60));
    let wave: Vec<Point> = (0..=20)
        .map(|i| {
            let x = -50.0 + 5.0 * f64::from(i);
            p(x, -32.0 + 6.0 * emb_geom::math::sin(f64::from(i) * 0.6))
        })
        .collect();
    d.add_object(
        blue,
        ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(-50.0, -80.0), p(50.0, -30.0))),
            holes: Vec::new(),
            params: FillParams::default().with_pattern(FillPattern::Guided {
                guides: vec![vec![p(-50.0, -78.0), p(50.0, -78.0)], wave],
            }),
        },
    );
    let leaf = Path::new()
        .move_to(p(-45.0, 40.0))
        .cubic_to(p(-20.0, 0.0), p(25.0, 0.0), p(45.0, 30.0))
        .cubic_to(p(20.0, 60.0), p(-25.0, 65.0), p(-45.0, 40.0))
        .close();
    d.add_object(
        green,
        ObjectKind::Fill {
            outer: leaf,
            holes: Vec::new(),
            params: FillParams::default().with_pattern(FillPattern::Guided {
                guides: vec![
                    (0..=10)
                        .map(|i| {
                            let t = f64::from(i) / 10.0;
                            p(-45.0 + 90.0 * t, 40.0 - 18.0 * t + 8.0 * t * t)
                        })
                        .collect(),
                ],
            }),
        },
    );
    d
}

/// P11 Milestone 4: ripple stitches: a 50 mm circle in ten rings to its
/// middle, a heart to a point near its top, and a wave that moves down a
/// guide in eight copies.
fn ripple_stitch() -> Design {
    let mut d = Design::new("");
    let red = d.add_thread("Red", Rgb(200, 30, 40));
    let navy = d.add_thread("Navy", Rgb(20, 30, 90));
    let ripple = |r: emb_model::Ripple| RunningParams {
        ripple: Some(r),
        ..RunningParams::default()
    };
    d.add_object(
        red,
        ObjectKind::Running {
            path: Path::ellipse(p(-60.0, -40.0), 25.0, 25.0),
            params: ripple(emb_model::Ripple::default()),
        },
    );
    let heart = Path::new()
        .move_to(p(60.0, -20.0))
        .cubic_to(p(30.0, -40.0), p(35.0, -70.0), p(60.0, -55.0))
        .cubic_to(p(85.0, -70.0), p(90.0, -40.0), p(60.0, -20.0))
        .close();
    d.add_object(
        red,
        ObjectKind::Running {
            path: heart,
            params: ripple(emb_model::Ripple {
                lines: 8,
                end_percent: 10.0,
                target: Some(p(60.0, -45.0)),
                guide: Vec::new(),
            }),
        },
    );
    let wave = (1..=12).fold(Path::new().move_to(p(-70.0, 20.0)), |path, i| {
        let x = -70.0 + 10.0 * f64::from(i);
        path.line_to(p(x, if i % 2 == 1 { 14.0 } else { 20.0 }))
    });
    d.add_object(
        navy,
        ObjectKind::Running {
            path: wave,
            params: ripple(emb_model::Ripple {
                lines: 8,
                end_percent: 100.0,
                target: None,
                guide: vec![p(-70.0, 20.0), p(-60.0, 50.0), p(-70.0, 76.0)],
            }),
        },
    );
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_has_rows_color_rows_and_a_scribble() {
        let d = build("photo").unwrap();
        // Rows (1) + color rows (4) + scribble (1).
        assert_eq!(d.objects().len(), 6);
        assert!(
            d.objects()
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::Stitches { .. }))
        );
    }

    #[test]
    fn smart_logo_has_fills_and_a_satin() {
        let d = build("smart-logo").unwrap();
        let fills = d
            .objects()
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Fill { .. }))
            .count();
        let satins = d
            .objects()
            .iter()
            .filter(|o| matches!(o.kind, ObjectKind::Satin { .. }))
            .count();
        assert!(fills >= 2 && satins == 1, "{fills} fills, {satins} satins");
        assert_eq!(d.threads.len(), 3);
        // The ring keeps its hole, and the green is the leaf color.
        assert!(d.threads.iter().any(|t| t.color.1 > 140 && t.color.0 < 60));
        assert!(
            d.objects()
                .iter()
                .any(|o| matches!(&o.kind, ObjectKind::Fill { holes, .. } if !holes.is_empty()))
        );
    }

    #[test]
    fn the_quilt_block_sews_in_four_hoopings() {
        let d = build("quilt-block").unwrap();
        let l = d.hooping.as_ref().expect("hoopings");
        assert_eq!(l.hoopings.len(), 4);
    }

    #[test]
    fn the_lace_designs_have_no_loose_part() {
        for name in ["lace-ornament", "lace-edge"] {
            let d = build(name).unwrap();
            let plan = emb_stitch::plan::build_plan(
                &d,
                &emb_stitch::plan::PlanOptions::default(),
                &mut emb_stitch::StitchCache::default(),
            );
            assert!(emb_stitch::lace::is_lace(&d, &emb_model::FabricCatalog::builtin()));
            assert_eq!(emb_stitch::lace::loose_parts(&d, &plan), Vec::new(), "{name}");
        }
    }

    #[test]
    fn autotrace_has_each_kind_of_object() {
        let d = build("autotrace").unwrap();
        let kinds: Vec<&str> = d.objects().iter().map(|o| o.kind.type_name()).collect();
        assert_eq!(kinds.len(), 5, "{kinds:?}");
        assert!(matches!(&d.objects()[1].kind, ObjectKind::Fill { holes, .. } if holes.len() == 1));
        assert!(matches!(d.objects()[2].kind, ObjectKind::Satin { .. }));
        assert!(matches!(d.objects()[0].kind, ObjectKind::Vector { .. }));
    }

    #[test]
    fn all_names_build_and_fit_the_hoop() {
        let hoops = emb_model::HoopCatalog::builtin();
        let hoop = hoops.get(HOOP).unwrap();
        for name in NAMES {
            let d = build(name).unwrap();
            // A design in several hoopings fits hooping by hooping (tests/golden.rs).
            if let Some(l) = &d.hooping {
                let outs = d.hoopings(&super::hoop(&l.hoop).unwrap());
                assert!(outs.len() >= 2, "{name}");
                assert!(outs.iter().all(|o| o.warnings.is_empty()), "{name}");
                continue;
            }
            // In the hoop frame: the hoop angle and the design rotation applied.
            let out = d.output_transform();
            for o in d.objects() {
                for path in o.kind.paths() {
                    for pl in path.flatten(0.1) {
                        for pt in &pl.points {
                            let q = out.apply(*pt);
                            assert!(hoop.contains(q, -0.01), "{name}: {q:?} is outside the hoop");
                        }
                    }
                }
            }
        }
        assert!(build("nothing").is_none());
    }
}
