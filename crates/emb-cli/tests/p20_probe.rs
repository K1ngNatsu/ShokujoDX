//! A probe of phase P20 (the audit of P20): Japanese text that is long,
//! large, small or odd in the test font of the lettering bench, outlines
//! that are hard for the stroke finder and the route, a Japanese embroidery
//! font file with wrong numbers, damaged font headers, and wrong axis
//! values of a variable font. Each case runs in a thread with a time limit;
//! the lettering is sewn.
//! `cargo test --release -p emb-cli --test p20_probe -- --ignored --nocapture`
//! (`EMB_VARIABLE_FONT=<file>`: a variable font for the axis values).

use std::sync::Arc;
use std::time::{Duration, Instant};

use emb_geom::Point;
use emb_letter::{Font, GlyphShapes, LetterSource};
use emb_model::{Design, LetterStitch, Lettering, ObjectKind, Rgb, TextDirection};
use serde_json::Value;

const TEST_FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/noto-sans-jp-bold-subset.ttf");

fn test_font() -> Font {
    Font::from_bytes(std::fs::read(TEST_FONT).expect("the test font"), 0).expect("a font")
}

/// Runs `job` in a thread. A problem text if it panics, takes more than
/// `slow`, or does not end in `limit`.
fn timed<T: Send + 'static>(
    name: &str,
    slow: Duration,
    limit: Duration,
    job: impl FnOnce() -> T + Send + 'static,
) -> Result<(T, Duration), String> {
    let start = Instant::now();
    let job = std::thread::Builder::new().stack_size(16 << 20).spawn(job).unwrap();
    while !job.is_finished() && start.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(2));
    }
    if !job.is_finished() {
        return Err(format!("{name}: over {limit:?}"));
    }
    let took = start.elapsed();
    match job.join() {
        Ok(_) if took > slow => Err(format!("{name}: {took:.2?}")),
        Ok(v) => Ok((v, took)),
        Err(_) => Err(format!("{name}: panic")),
    }
}

/// Sews `l` alone. The stitch count, and a problem text for a stitch that
/// is not a real number or is out of the range of a project.
fn sew(l: Lettering) -> (usize, Option<String>) {
    let mut d = Design::new("probe");
    let t = d.add_thread("Black", Rgb(0, 0, 0));
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    let plan = emb_stitch::build_plan(&d, &emb_stitch::PlanOptions::default(), &mut Default::default());
    let m = emb_model::project::MAX_COORD_MM;
    let bad = plan
        .stitches
        .records
        .iter()
        .find(|r| !r.pos.x.is_finite() || !r.pos.y.is_finite() || r.pos.x.abs() > m || r.pos.y.abs() > m)
        .map(|r| format!("a stitch at {:?}", r.pos));
    (plan.stitches.stitch_count(), bad)
}

/// Lays out and sews `text` in `src`, with `f` applied to the lettering.
fn letter_and_sew(src: &dyn LetterSource, text: &str, f: &dyn Fn(&mut Lettering)) -> (usize, Option<String>) {
    let mut l = Lettering::new(text, src.font_ref(), 15.0, Point::ZERO);
    l.character_height = true;
    f(&mut l);
    emb_letter::relayout(&mut l, src);
    sew(l)
}

type Change = Box<dyn Fn(&mut Lettering) + Send + Sync>;

/// `text` with a new line after each `n` characters.
fn lines(text: &str, n: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(n)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
#[ignore]
fn probe_p20_texts() {
    std::panic::set_hook(Box::new(|_| {}));
    let font = Arc::new(test_font());
    // The routes of the reference set first (RouteMode::Wait: the tools),
    // so the cases measure the layout and the stitches.
    let reference: String = emb_letter::japanese::reference().into_iter().collect();
    let f = font.clone();
    let r = reference.clone();
    match timed(
        "routes of the reference set",
        Duration::from_secs(120),
        Duration::from_secs(600),
        move || letter_and_sew(&*f, &r, &|_| {}),
    ) {
        Ok((_, took)) => println!("routes of the reference set: {took:.2?}"),
        Err(e) => println!("PROBLEM {e}"),
    }
    let none = || -> Change { Box::new(|_| {}) };
    let cases: Vec<(&str, String, Change)> = vec![
        // A stitch object is at most 3 m (`emb_stitch::MAX_OBJECT_MM`), so
        // the long texts have lines.
        (
            "the reference set 4 times, 40 characters a line",
            lines(&reference.repeat(4), 40),
            none(),
        ),
        (
            "the reference set in one line",
            reference.clone(),
            Box::new(|l| l.height_mm = 10.0),
        ),
        ("120 lines", "刺繍\n".repeat(120), none()),
        ("0.5 mm", "刺繍東京".into(), Box::new(|l| l.height_mm = 0.5)),
        ("2 mm", "刺繍東京".into(), Box::new(|l| l.height_mm = 2.0)),
        ("300 mm", "刺繍東京".into(), Box::new(|l| l.height_mm = 300.0)),
        ("1000 mm", "刺繍".into(), Box::new(|l| l.height_mm = 1000.0)),
        (
            "2000 mm (more than 3 m: no stitches)",
            "刺繍".into(),
            Box::new(|l| l.height_mm = 2000.0),
        ),
        ("not in the font", "𠮟🙂\u{E000}\u{FFFD}\u{10FFFF}".repeat(200), none()),
        ("marks on kana", format!("か{}", "\u{3099}\u{309A}".repeat(500)), none()),
        (
            "variation selectors",
            lines(&"刺\u{E0100}東\u{FE00}".repeat(100), 40),
            none(),
        ),
        ("full-width spaces only", "\u{3000}".repeat(500), none()),
        ("marks only", lines(&"。、・「」！？".repeat(40), 35), none()),
        ("small kana", lines(&"ゃゅょっぁぃぅぇぉ".repeat(20), 36), none()),
        (
            "stacked down",
            "刺繍東京\nありがとう".into(),
            Box::new(|l| l.direction = TextDirection::Down),
        ),
        (
            "stacked up",
            "刺繍東京".repeat(25),
            Box::new(|l| l.direction = TextDirection::Up),
        ),
        (
            "bold and italic",
            "刺繍東京龍".into(),
            Box::new(|l| {
                l.bold = true;
                l.italic = true;
            }),
        ),
        ("Latin only, character height", "Hamburg".into(), none()),
        ("Latin and kanji", "Tokyo 東京 2026".into(), none()),
        (
            "cap height",
            "刺繍東京".into(),
            Box::new(|l| l.character_height = false),
        ),
        (
            "letter spacing -50 mm",
            "刺繍東京".into(),
            Box::new(|l| l.letter_spacing_mm = -50.0),
        ),
        (
            "letter spacing 500 mm",
            "刺繍東京".into(),
            Box::new(|l| l.letter_spacing_mm = 500.0),
        ),
        (
            "satin",
            "刺繍東京龍".into(),
            Box::new(|l| l.stitch = LetterStitch::Satin),
        ),
        ("fill", "刺繍東京龍".into(), Box::new(|l| l.stitch = LetterStitch::Fill)),
        (
            "satin at 300 mm",
            "龍".into(),
            Box::new(|l| {
                l.stitch = LetterStitch::Satin;
                l.height_mm = 300.0;
            }),
        ),
        (
            "auto at 8 mm (thin runs)",
            lines(&reference, 40),
            Box::new(|l| l.height_mm = 8.0),
        ),
    ];
    let mut problems = Vec::new();
    for (what, text, change) in cases {
        let f = font.clone();
        let job = move || letter_and_sew(&*f, &text, &*change);
        match timed(what, Duration::from_secs(20), Duration::from_secs(120), job) {
            Ok(((stitches, bad), took)) => {
                println!("{what}: {stitches} stitches, {took:.2?}");
                if let Some(b) = bad {
                    problems.push(format!("{what}: {b}"));
                }
            }
            Err(e) => problems.push(e),
        }
    }
    // The size range of a text (the Lettering window, each frame).
    for text in ["刺繍", "Hamburg", "", "\u{3000}", "𠮟"] {
        let f = font.clone();
        let t = text.to_string();
        let job = move || {
            [LetterStitch::Auto, LetterStitch::Satin, LetterStitch::Fill]
                .into_iter()
                .map(|stitch| f.text_size_range(&t, stitch))
                .collect::<Vec<_>>()
        };
        match timed(
            &format!("size range of {text:?}"),
            Duration::from_secs(2),
            Duration::from_secs(30),
            job,
        ) {
            Ok((ranges, took)) => {
                println!("size range of {text:?}: {ranges:?}, {took:.2?}");
                for (lo, hi) in ranges.into_iter().flatten() {
                    if !(lo.is_finite() && hi.is_finite() && lo > 0.0 && lo <= hi) {
                        problems.push(format!("size range of {text:?}: {lo} to {hi}"));
                    }
                }
            }
            Err(e) => problems.push(e),
        }
    }
    println!("problems {}", problems.len());
    for p in &problems {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}

/// A ring: `n` points on a circle of radius `r` round `c`.
fn circle(c: Point, r: f64, n: usize, clockwise: bool) -> Vec<Point> {
    let turn = if clockwise { -1.0 } else { 1.0 };
    (0..n)
        .map(|i| {
            let a = i as f64 / n as f64 * std::f64::consts::TAU * turn;
            Point::new(c.x + r * emb_geom::math::cos(a), c.y + r * emb_geom::math::sin(a))
        })
        .collect()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

/// The steps of the app and of the converter on the rings of a glyph: the
/// shapes, the parts in Auto, and the route of the columns. The number of
/// parts in Auto and after the route, and the part of the rings that the
/// columns cover before and after the route.
fn shapes_and_route(rings: Vec<Vec<Point>>, cap: f64) -> (usize, usize, f64, f64) {
    let shapes = GlyphShapes::new(&rings, cap);
    let auto = shapes.auto_parts();
    let mut parts = shapes.satin_parts();
    let s = emb_letter::embfont::DESIGN_CAP_MM / cap;
    for p in &mut parts {
        p.transform(&emb_geom::Transform::scale(s, s));
    }
    let letter = emb_letter::EmbLetter {
        advance_mm: 20.0,
        parts,
        fixed: false,
        tuning: Vec::new(),
        fill_parts: Vec::new(),
    };
    let (routed, _) = emb_letter::route::route_with_splits(&letter, emb_letter::embfont::DESIGN_CAP_MM);
    let outline: Vec<(Vec<Point>, Vec<Vec<Point>>)> = rings
        .iter()
        .filter(|r| r.len() > 2)
        .map(|r| (r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect(), Vec::new()))
        .collect();
    let cover = |parts: &[ObjectKind]| emb_letter::quality::areas(parts, &outline).map_or(0.0, |a| a.coverage);
    (
        auto.len(),
        routed.parts.len(),
        cover(&letter.parts),
        cover(&routed.parts),
    )
}

#[test]
#[ignore]
fn probe_p20_outlines() {
    std::panic::set_hook(Box::new(|_| {}));
    let cap = 700.0;
    let o = Point::new(400.0, 350.0);
    let star: Vec<Point> = (0..5)
        .map(|i| {
            let a = i as f64 * 2.0 * std::f64::consts::TAU / 5.0;
            Point::new(
                o.x + 300.0 * emb_geom::math::cos(a),
                o.y + 300.0 * emb_geom::math::sin(a),
            )
        })
        .collect();
    let mut cases: Vec<(&str, Vec<Vec<Point>>)> = vec![
        (
            "a ring of 20,000 points",
            vec![circle(o, 300.0, 20_000, false), circle(o, 200.0, 20_000, true)],
        ),
        ("a sliver", vec![rect(0.0, 0.0, 700.0, 0.001)]),
        ("all points the same", vec![vec![o; 50]]),
        ("two points", vec![vec![o, Point::new(500.0, 500.0)]]),
        ("an empty ring", vec![vec![]]),
        ("1e7 units", vec![rect(0.0, 0.0, 1e7, 2e6)]),
        ("a star that crosses itself", vec![star]),
        (
            "50 rings in each other",
            (0..50)
                .map(|i| circle(o, 340.0 - 6.5 * i as f64, 64, i % 2 == 1))
                .collect(),
        ),
        (
            "400 dots",
            (0..400)
                .map(|i| {
                    circle(
                        Point::new((i % 20) as f64 * 35.0, (i / 20) as f64 * 35.0),
                        8.0,
                        16,
                        false,
                    )
                })
                .collect(),
        ),
    ];
    // A lattice of 12 by 12 strokes: 144 crossings.
    let mut lattice = Vec::new();
    for i in 0..12 {
        let x = 30.0 + i as f64 * 55.0;
        lattice.push(rect(x, 0.0, x + 30.0, 700.0));
        lattice.push(rect(0.0, x, 700.0, x + 30.0));
    }
    cases.push(("a lattice of 24 strokes", lattice));
    // Thin strokes that touch at their ends: a comb.
    cases.push((
        "a comb of 60 teeth",
        (0..60)
            .map(|i| rect(i as f64 * 11.0, 0.0, i as f64 * 11.0 + 6.0, 600.0))
            .chain([rect(0.0, 600.0, 666.0, 640.0)])
            .collect(),
    ));
    let mut problems = Vec::new();
    for (what, rings) in cases {
        match timed(what, Duration::from_secs(10), Duration::from_secs(120), move || {
            shapes_and_route(rings, cap)
        }) {
            Ok(((auto, routed, before, after), took)) => {
                println!(
                    "{what}: {auto} parts, {routed} routed, covered {:.1} % before and {:.1} % after the route, {took:.2?}",
                    before * 100.0,
                    after * 100.0
                );
                if after < before - 0.01 {
                    problems.push(format!("{what}: the route covers less"));
                }
            }
            Err(e) => problems.push(e),
        }
    }
    println!("problems {}", problems.len());
    for p in &problems {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}

/// The paths of the numbers in `v` (the first two items of each list).
fn numbers(v: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match v {
        Value::Number(_) => out.push(path.clone()),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate().take(2) {
                path.push(i.to_string());
                numbers(x, path, out);
                path.pop();
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                path.push(k.clone());
                numbers(x, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn at<'a>(v: &'a mut Value, path: &[String]) -> &'a mut Value {
    path.iter().fold(v, |v, k| match v {
        Value::Array(a) => &mut a[k.parse::<usize>().unwrap()],
        Value::Object(o) => o.get_mut(k).unwrap(),
        _ => unreachable!(),
    })
}

#[test]
#[ignore]
fn probe_p20_font_file() {
    std::panic::set_hook(Box::new(|_| {}));
    // A small Japanese embroidery font, made as the font tool makes the
    // Japanese fonts: routed, satin columns to their end, no fill letters.
    let font = test_font();
    let chars: Vec<char> = "あ刺 A".chars().collect();
    let mut f = emb_letter::embfont::convert_with(&font, "Probe JP", "", LetterStitch::Auto, &chars, false);
    f.add_fill_letters(&font);
    f.route();
    f.measure_quality(&font);
    assert!(f.satin_max_mm.is_some());
    let root: Value = serde_json::from_str(&f.to_json()).unwrap();
    let mut paths = Vec::new();
    numbers(&root, &mut Vec::new(), &mut paths);
    let (mut cases, mut refused) = (0, 0);
    let mut problems = Vec::new();
    for path in &paths {
        let whole = at(&mut root.clone(), path).is_u64();
        let values: Vec<Value> = if whole {
            vec![0.into(), 100_000.into(), 1_000_000_000u64.into()]
        } else {
            [0.0, -5.0, 1e-9, 1e9].map(|x| serde_json::json!(x)).to_vec()
        };
        for value in values {
            cases += 1;
            let mut v = root.clone();
            *at(&mut v, path) = value.clone();
            let Ok(font) = emb_letter::EmbFont::from_json(&v.to_string()) else {
                refused += 1;
                continue;
            };
            let name = format!("{} = {value}", path.join("."));
            let job = move || {
                let mut stitches = 0;
                for stitch in [LetterStitch::Auto, LetterStitch::Satin] {
                    for size in [10.0, 100.0] {
                        let (n, bad) = letter_and_sew(&font, "あ刺 A", &|l| {
                            l.stitch = stitch;
                            l.height_mm = size;
                        });
                        assert!(bad.is_none(), "{bad:?}");
                        stitches += n;
                    }
                }
                let _ = font.size_range(LetterStitch::Satin);
                let _ = font.column_max_mm();
                stitches
            };
            match timed(&name, Duration::from_secs(3), Duration::from_secs(30), job) {
                Ok((stitches, _)) if stitches > 3_000_000 => problems.push(format!("{name}: {stitches} stitches")),
                Ok(_) => {}
                Err(e) => problems.push(e),
            }
        }
    }
    println!("cases {cases}, refused {refused}, problems {}", problems.len());
    for p in problems.iter().take(60) {
        println!("  {p}");
    }
    // Damaged headers: the font lists read the header of a large font only.
    let good = f.to_json();
    let headers: Vec<(&str, String)> = vec![
        ("empty", String::new()),
        ("{}", "{}".into()),
        ("half", good[..good.len() / 2].to_string()),
        ("no family", good.replacen("Probe JP", "  ", 1)),
        ("other format", good.replacen("\"format\":\"", "\"format\":\"x", 1)),
        ("a long family", good.replacen("Probe JP", &"x".repeat(100_000), 1)),
        (
            "letters wrong",
            good.replacen("\"letters\":{", "\"letters\":{\"a\":5,", 1),
        ),
    ];
    let dir = std::env::temp_dir().join(format!("emb-p20-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut list_only = Vec::new();
    for (what, text) in headers {
        let file = dir.join("font.embfont");
        std::fs::write(&file, &text).unwrap();
        let header = emb_letter::EmbFont::load_header(&file);
        let full = emb_letter::EmbFont::load(&file);
        println!(
            "header {what}: header {}, font {}",
            if header.is_ok() { "read" } else { "refused" },
            if full.is_ok() { "read" } else { "refused" }
        );
        if header.is_ok() && full.is_err() {
            list_only.push(what);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("headers that read, of fonts that do not load: {list_only:?}");
    assert!(problems.is_empty());
}

#[test]
#[ignore]
fn probe_p20_axis_values() {
    std::panic::set_hook(Box::new(|_| {}));
    let file = std::env::var("EMB_VARIABLE_FONT").unwrap_or_else(|_| TEST_FONT.to_string());
    let data = std::fs::read(&file).expect("the font");
    let mut problems = Vec::new();
    let values: [(&str, f32); 9] = [
        ("wght", 0.0),
        ("wght", -5.0),
        ("wght", 1e9),
        ("wght", f32::NAN),
        ("wght", f32::INFINITY),
        ("", 700.0),
        ("wghtx", 700.0),
        ("\u{0}\u{0}\u{0}\u{0}", 700.0),
        ("ital", 1.0),
    ];
    for (axis, value) in values {
        let data = data.clone();
        let name = format!("{axis:?} = {value}");
        let job = move || {
            let font = Font::from_bytes_with(data, 0, &[(axis, value)]).ok()?;
            let (n, bad) = letter_and_sew(&font, "刺繍あA", &|_| {});
            Some((n, bad, font.cap_height, font.character_height))
        };
        match timed(&name, Duration::from_secs(20), Duration::from_secs(120), job) {
            Ok((Some((n, bad, cap, ch)), took)) => {
                println!("{name}: {n} stitches, cap {cap}, character height {ch:?}, {took:.2?}");
                if let Some(b) = bad {
                    problems.push(format!("{name}: {b}"));
                }
                if !cap.is_finite() || cap <= 0.0 {
                    problems.push(format!("{name}: cap height {cap}"));
                }
            }
            Ok((None, _)) => println!("{name}: refused"),
            Err(e) => problems.push(e),
        }
    }
    println!("problems {}", problems.len());
    for p in &problems {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}
