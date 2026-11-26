//! A probe of what phase P18 reads from the files of other programs (the
//! audit of P18): stitch files that are hard to recognize, outlines with
//! wrong sizes, Ink/Stitch files with wrong settings and Ink/Stitch fonts
//! with wrong numbers. Each case runs in a thread with a time limit; the
//! made objects are sewn. The EPS reader has its own probe
//! (`emb_formats::eps::probe`).
//! `cargo test --release -p emb-cli --test p18_probe -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use emb_art::part_objects::{Made, recognize};
use emb_art::part_outlines::{Outline, fit_by_size, fit_two_points, outlines_bbox, transformed, with_outlines};
use emb_geom::{BBox, Path, Point};
use emb_model::{Design, ObjectKind, Rgb, StitchDesign, StitchKind, StitchRecord, design_from_stitches};
use emb_stitch::PlanOptions;

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

fn far(b: &BBox) -> bool {
    let m = emb_model::project::MAX_COORD_MM;
    let bad = |p: Point| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > m || p.y.abs() > m;
    bad(b.min) || bad(b.max)
}

/// The objects that `made` gives (the app's Accept: the recognized kinds,
/// and the stitches of the rest), as a design, and its stitch count.
fn sew_made(made: &[Made]) -> (usize, usize) {
    let mut d = Design::new("probe");
    let t = d.add_thread("Black", Rgb(0, 0, 0));
    for m in made {
        d.add_object(t, m.kind.clone());
    }
    let objects = d.objects().len();
    let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
    (objects, plan.stitches.stitch_count())
}

/// Recognizes each stitch object of the stitch list `s` and sews the made
/// objects; with `outlines`, the outlines give the objects their shapes
/// first. Returns the number of made objects and of their stitches.
fn recognize_and_sew(s: &StitchDesign, outlines: &[Outline]) -> (usize, usize) {
    let d = design_from_stitches(s, "probe");
    let (mut objects, mut stitches) = (0, 0);
    for o in d.objects() {
        let Some((sd, made)) = recognize(&o.kind) else { continue };
        let made = if outlines.is_empty() {
            made
        } else {
            with_outlines(&sd, made, outlines).0
        };
        let (n, k) = sew_made(&made);
        objects += n;
        stitches += k;
    }
    (objects, stitches)
}

/// A small random number source (the same numbers on each run).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn st(x: f64, y: f64) -> StitchRecord {
    StitchRecord::stitch(x, y)
}
fn jump(x: f64, y: f64) -> StitchRecord {
    StitchRecord::new(Point::new(x, y), StitchKind::Jump)
}
fn trim(x: f64, y: f64) -> StitchRecord {
    StitchRecord::new(Point::new(x, y), StitchKind::Trim)
}

fn zigzag(out: &mut Vec<StitchRecord>, at: Point, n: usize, width: f64, step: f64) {
    for i in 0..n {
        let side = if i % 2 == 0 { 0.0 } else { width };
        out.push(st(at.x + i as f64 * step, at.y + side));
    }
}

/// Rows from (x0, y0): `n` rows of `len` mm, `spacing` apart, needle points
/// `stitch` apart, at `angle` degrees.
fn rows(out: &mut Vec<StitchRecord>, at: Point, n: usize, len: f64, spacing: f64, stitch: f64, angle: f64) {
    let (x0, y0) = (at.x, at.y);
    let (s, c) = emb_geom::math::sin_cos_deg(angle);
    let k = (len / stitch).ceil().max(1.0) as usize;
    for r in 0..n {
        for j in 0..=k {
            let along = if r % 2 == 0 {
                j as f64 * stitch
            } else {
                len - j as f64 * stitch
            };
            let across = r as f64 * spacing;
            out.push(st(x0 + along * c - across * s, y0 + along * s + across * c));
        }
    }
}

// The cases need values made one after the other.
#[allow(clippy::vec_init_then_push)]
fn stitch_cases() -> Vec<(&'static str, StitchDesign)> {
    let one = |records: Vec<StitchRecord>| StitchDesign {
        threads: vec![Default::default()],
        records,
    };
    let mut cases = Vec::new();
    cases.push(("no stitches", one(vec![])));
    cases.push(("one stitch", one(vec![st(0.0, 0.0)])));
    cases.push(("10,000 stitches at one point", one(vec![st(5.0, 5.0); 10_000])));
    cases.push((
        "a stitch of 1 km",
        one(vec![st(0.0, 0.0), st(1_000_000.0, 0.0), st(0.0, 0.0)]),
    ));
    let mut v = Vec::new();
    zigzag(&mut v, Point::new(999_900.0, 999_900.0), 2_000, 4.0, 0.4);
    cases.push(("a column at the edge of the range", one(v)));
    let mut v = Vec::new();
    for i in 0..20_000 {
        v.push(st(if i % 2 == 0 { 0.0 } else { 5.0 }, 0.0));
    }
    cases.push(("a column of no width", one(v)));
    let mut v = Vec::new();
    rows(&mut v, Point::new(0.0, 0.0), 750, 300.0, 0.4, 3.0, 0.0);
    rows(&mut v, Point::new(0.0, 0.0), 750, 300.0, 0.4, 3.0, 45.0);
    rows(&mut v, Point::new(0.0, 0.0), 750, 300.0, 0.4, 3.0, 90.0);
    cases.push(("three large fills of rows", one(v)));
    let mut v = Vec::new();
    rows(&mut v, Point::new(0.0, 0.0), 100_000, 10.0, 0.001, 10.0, 30.0);
    cases.push(("rows 0.001 mm apart", one(v)));
    let mut r = Lcg(1);
    cases.push((
        "100,000 random stitches",
        one((0..100_000).map(|_| st(r.next() * 100.0, r.next() * 100.0)).collect()),
    ));
    let mut v = Vec::new();
    for i in 0..50_000 {
        let (x, y) = ((i % 200) as f64, (i / 200) as f64);
        v.push(jump(x, y));
        v.push(st(x, y));
        v.push(st(x + 0.5, y));
    }
    cases.push(("50,000 parts after jumps", one(v)));
    let mut v = Vec::new();
    for i in 0..20_000 {
        let (x, y) = ((i % 100) as f64 * 2.0, (i / 100) as f64 * 2.0);
        v.push(jump(x, y));
        zigzag(&mut v, Point::new(x, y), 6, 1.0, 0.3);
        v.push(trim(x + 1.8, y));
    }
    cases.push(("20,000 short columns with trims", one(v)));
    let mut v = Vec::new();
    for _ in 0..1_000 {
        zigzag(&mut v, Point::new(0.0, 0.0), 100, 5.0, 0.4);
        zigzag(&mut v, Point::new(0.0, 0.0), 100, 5.0, 0.4);
    }
    cases.push(("one column sewn 2,000 times", one(v)));
    let mut v = Vec::new();
    for _ in 0..500 {
        rows(&mut v, Point::new(0.0, 0.0), 40, 20.0, 0.5, 3.0, 0.0);
    }
    cases.push(("one fill sewn 500 times", one(v)));
    let mut d = StitchDesign {
        threads: vec![Default::default(); 5_000],
        records: Vec::new(),
    };
    for i in 0..5_000 {
        zigzag(
            &mut d.records,
            Point::new((i % 70) as f64 * 3.0, (i / 70) as f64 * 3.0),
            10,
            2.0,
            0.3,
        );
        d.records
            .push(StitchRecord::new(Point::new(0.0, 0.0), StitchKind::ColorChange));
    }
    cases.push(("5,000 colors", d));
    cases
}

#[test]
#[ignore]
fn probe_p18_stitch_files() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut problems = Vec::new();
    for (name, s) in stitch_cases() {
        let n = s.records.len();
        // Recognized alone, and with one outline round all and a small one.
        for with in [false, true] {
            let outlines = if with {
                let b = s.extents().unwrap_or(BBox::new(Point::ZERO, Point::new(1.0, 1.0)));
                vec![
                    Outline {
                        path: Path::rect(b),
                        filled: true,
                        width_mm: 0.0,
                        name: None,
                    },
                    Outline {
                        path: Path::rect(BBox::new(b.min, b.min + Point::new(5.0, 5.0))),
                        filled: true,
                        width_mm: 0.0,
                        name: None,
                    },
                ]
            } else {
                Vec::new()
            };
            let label = format!("{name}{}", if with { " (outlines)" } else { "" });
            let s = s.clone();
            // One fill sewn 500 times took 28 s at the audit of P18: the
            // area of its rows made a piece for each pair of rows on one
            // line. Now the same rows of a line are one (0.4 s).
            match timed(&label, Duration::from_secs(10), Duration::from_secs(300), move || {
                recognize_and_sew(&s, &outlines)
            }) {
                Ok(((objects, stitches), took)) => {
                    println!("{label}: {n} records, {objects} objects, {stitches} stitches, {took:.2?}");
                }
                Err(e) => {
                    println!("PROBLEM {e}");
                    problems.push(e);
                }
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The time of Recognize Objects for each corpus design as DST: the window
/// does it on the thread of the window when it opens.
#[test]
#[ignore]
fn probe_p18_recognize_times() {
    let mut times = Vec::new();
    for name in emb_cli::corpus::NAMES {
        let Some(d) = emb_cli::corpus::build(name) else {
            continue;
        };
        let plan = emb_cli::plan(&d, &PlanOptions::default());
        let Some(bytes) = emb_formats::write_machine("dst", &plan.stitches, name, "2026-11-20") else {
            continue;
        };
        let Ok(read) = emb_formats::read_machine("dst", &bytes) else {
            continue;
        };
        let start = Instant::now();
        let (objects, _) = recognize_and_sew(&read, &[]);
        times.push((start.elapsed(), name.to_string(), read.records.len(), objects));
    }
    times.sort();
    for (t, name, n, objects) in times.iter().rev().take(8) {
        println!("{name}: {n} records, {objects} objects, {t:.2?}");
    }
    let total: Duration = times.iter().map(|t| t.0).sum();
    println!("{} designs, {total:.2?} in all", times.len());
}

#[test]
#[ignore]
fn probe_p18_outlines() {
    std::panic::set_hook(Box::new(|_| {}));
    let b = |x0: f64, y0: f64, x1: f64, y1: f64| BBox::new(Point::new(x0, y0), Point::new(x1, y1));
    // The fits with sizes of 0 and very large sizes.
    for (o, s) in [
        (b(0.0, 0.0, 0.0, 0.0), b(0.0, 0.0, 10.0, 10.0)),
        (b(0.0, 0.0, 10.0, 10.0), b(5.0, 5.0, 5.0, 5.0)),
        (b(0.0, 0.0, 1e-12, 1e-12), b(0.0, 0.0, 100.0, 100.0)),
        (b(-1e6, -1e6, 1e6, 1e6), b(0.0, 0.0, 1e-6, 1e-6)),
    ] {
        let t = fit_by_size(o, s);
        let p = t.apply(o.max);
        println!("fit by size {o:?} on {s:?}: {p:?}");
        assert!(p.x.is_finite() && p.y.is_finite());
    }
    assert!(fit_two_points([Point::ZERO; 2], [Point::ZERO, Point::new(1.0, 0.0)]).is_none());
    let t = fit_two_points(
        [Point::ZERO, Point::new(1e-5, 0.0)],
        [Point::ZERO, Point::new(1e6, 0.0)],
    );
    println!(
        "two points 10 µm apart onto 1 km: {:?}",
        t.map(|t| t.apply(Point::new(1.0, 1.0)))
    );

    // Many small outlines, outlines far away and outlines of no size on a
    // corpus design.
    let d = emb_cli::corpus::build("two-color-patch").unwrap();
    let plan = emb_cli::plan(&d, &PlanOptions::default());
    let s = plan.stitches.clone();
    let sb = s.extents().unwrap();
    let mut problems = Vec::new();
    let square = |x: f64, y: f64, w: f64| Outline {
        path: Path::rect(BBox::new(Point::new(x, y), Point::new(x + w, y + w))),
        filled: true,
        width_mm: 0.0,
        name: None,
    };
    let mut many = Vec::new();
    for i in 0..10_000 {
        many.push(square(sb.min.x + (i % 100) as f64, sb.min.y + (i / 100) as f64, 1.0));
    }
    let line = |a: Point, z: Point, w: f64| Outline {
        path: Path::new().move_to(a).line_to(z),
        filled: false,
        width_mm: w,
        name: None,
    };
    let cases: Vec<(&str, Vec<Outline>)> = vec![
        ("10,000 small squares", many),
        ("far away", vec![square(1e6, 1e6, 10.0)]),
        (
            "no size",
            vec![square(sb.min.x, sb.min.y, 0.0), line(sb.min, sb.min, 0.0)],
        ),
        ("a line of width 1e9", vec![line(sb.min, sb.max, 1e9)]),
        ("a line of width -5", vec![line(sb.min, sb.max, -5.0)]),
        (
            "fit onto a 1 km box",
            transformed(
                &[square(0.0, 0.0, 10.0)],
                &fit_by_size(outlines_bbox(&[square(0.0, 0.0, 10.0)]).unwrap(), b(0.0, 0.0, 1e6, 1e6)),
            ),
        ),
    ];
    for (name, outlines) in cases {
        let s = s.clone();
        match timed(name, Duration::from_secs(20), Duration::from_secs(300), move || {
            recognize_and_sew(&s, &outlines)
        }) {
            Ok(((objects, stitches), took)) => println!("{name}: {objects} objects, {stitches} stitches, {took:.2?}"),
            Err(e) => {
                println!("PROBLEM {e}");
                problems.push(e);
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The names of the settings that the Ink/Stitch reader reads (its list
/// `SETTINGS`, from the source).
fn inkstitch_settings() -> Vec<String> {
    let src = include_str!("../../emb-formats/src/inkstitch.rs");
    let start = src.find("const SETTINGS: &[&str] = &[").unwrap();
    let end = start + src[start..].find("];").unwrap();
    src[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

const WRONG: &[&str] = &[
    "0",
    "-5",
    "1e-9",
    "1e9",
    "1e19",
    "2147483648",
    "nan",
    "inf",
    "-inf",
    "abc",
    "",
    "1e9 1e9 1e9",
    "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
    "True",
];

/// Sets `inkstitch:<name>` to `value` on each shape of the file that has
/// Ink/Stitch settings.
fn with_setting(svg: &str, name: &str, value: &str) -> String {
    let attr = format!("inkstitch:{name}=\"");
    let mut out = String::new();
    for (i, part) in svg.split("<path").enumerate() {
        if i == 0 {
            out.push_str(part);
            continue;
        }
        out.push_str("<path");
        let end = part.find('>').unwrap_or(part.len());
        let (tag, rest) = part.split_at(end);
        let mut tag = tag.to_string();
        if let Some(at) = tag.find(&attr) {
            let v0 = at + attr.len();
            let v1 = v0 + tag[v0..].find('"').unwrap();
            tag.replace_range(v0..v1, value);
        } else if tag.contains("inkstitch:") {
            let close = if tag.ends_with('/') { tag.len() - 1 } else { tag.len() };
            tag.insert_str(close, &format!(" {attr}{value}\" "));
        }
        out.push_str(&tag);
        out.push_str(rest);
    }
    out
}

#[test]
#[ignore]
fn probe_p18_inkstitch_files() {
    std::panic::set_hook(Box::new(|_| {}));
    let badge = include_str!("data/inkstitch-badge.svg");
    let settings = inkstitch_settings();
    assert!(settings.len() > 100, "{}", settings.len());
    let mut problems = Vec::new();
    let (mut cases, mut slowest) = (0, (Duration::ZERO, String::new()));
    // A run can start at a case (`INK_PROBE_FROM`), after a case that took
    // all the memory.
    let from: usize = std::env::var("INK_PROBE_FROM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let skip: Vec<usize> = std::env::var("INK_PROBE_SKIP")
        .unwrap_or_default()
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    for name in &settings {
        for value in WRONG {
            cases += 1;
            if cases <= from || skip.contains(&cases) {
                continue;
            }
            let file = with_setting(badge, name, value);
            let label = format!("{name}=\"{value}\"");
            // The case before it runs: a case that takes all the memory ends
            // the process.
            {
                use std::io::Write;
                println!("CASE {cases} {label}");
                let _ = std::io::stdout().flush();
            }
            let job = move || {
                let imported = emb_formats::svg::import_svg(file.as_bytes()).ok()?;
                let d = imported.to_design("probe");
                let far_objects = d
                    .objects()
                    .iter()
                    .filter(|o| o.kind.control_bbox().is_some_and(|b| far(&b)))
                    .count();
                let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
                Some((plan.stitches.stitch_count(), far_objects))
            };
            match timed(&label, Duration::from_secs(5), Duration::from_secs(120), job) {
                Ok((Some((stitches, far_objects)), took)) => {
                    if took > slowest.0 {
                        slowest = (took, label.clone());
                    }
                    if stitches > 500_000 {
                        problems.push(format!("{label}: {stitches} stitches"));
                    }
                    if far_objects > 0 {
                        problems.push(format!("{label}: {far_objects} objects out of range"));
                    }
                }
                Ok((None, _)) => {}
                Err(e) => problems.push(e),
            }
        }
    }
    println!(
        "{} settings, {cases} cases, slowest {:.2?} ({})",
        settings.len(),
        slowest.0,
        slowest.1
    );
    for p in &problems {
        println!("PROBLEM {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
}

/// A small font in the format of Ink/Stitch (as the tests of
/// `emb_letter::inkstitch`).
const LTR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
 xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:inkstitch="http://inkstitch.org/namespace"
 width="WIDTHmm" height="100mm" viewBox="0 0 VIEW 100">
 <sodipodi:namedview id="nv"><sodipodi:guide position="0,GUIDE" orientation="0,1" id="g1" inkscape:label="baseline"/></sodipodi:namedview>
 <g inkscape:groupmode="layer" inkscape:label="GlyphLayer-A" id="la" transform="TRANSFORM">
  <path d="M 2 70 L 5 50 M 4 70 L 7 50 M 1 60 L 7 60" style="fill:none;stroke:#000000;stroke-width:0.3"
   inkstitch:satin_column="true" inkstitch:pull_compensation_mm="PULL"/>
 </g>
 <g inkscape:groupmode="layer" inkscape:label="GlyphLayer-V" id="lv">
  <path d="M 0 50 L 4 50 L 6 65 L 8 50 L 12 50 L 7 70 L 5 70 Z" style="fill:#000000"
   inkstitch:angle="30" inkstitch:row_spacing_mm="ROWS"/>
 </g>
</svg>"##;

const JSON: &str = r#"{"name": "Probe", "font_license": "SIL Open Font License v1.1",
 "leading": LEADING, "horiz_adv_x": {"A": ADV}, "horiz_adv_x_space": SPACE,
 "kerning_pairs": {"A V": KERN}, "min_scale": MIN, "max_scale": MAX}"#;

#[test]
#[ignore]
fn probe_p18_inkstitch_fonts() {
    std::panic::set_hook(Box::new(|_| {}));
    let defaults = [
        ("WIDTH", "100"),
        ("VIEW", "100"),
        ("GUIDE", "30"),
        ("TRANSFORM", "translate(0,0)"),
        ("PULL", "0.3"),
        ("ROWS", "0.25"),
        ("LEADING", "30"),
        ("ADV", "12"),
        ("SPACE", "5"),
        ("KERN", "2"),
        ("MIN", "0.5"),
        ("MAX", "2"),
    ];
    let wrong: &[(&str, &[&str])] = &[
        ("WIDTH", &["0", "1e-9", "1e9"]),
        ("VIEW", &["0", "1e-9", "1e9", "-100"]),
        ("GUIDE", &["1e9", "-1e9", "nan"]),
        (
            "TRANSFORM",
            &[
                "scale(1e9)",
                "scale(0)",
                "scale(1e-9)",
                "translate(1e9,0)",
                "rotate(1e9)",
            ],
        ),
        ("PULL", &["1e9", "-1e9"]),
        ("ROWS", &["0", "1e-9", "-1"]),
        ("LEADING", &["0", "-5", "1e9", "1e300"]),
        ("ADV", &["0", "-5", "1e9", "1e300"]),
        ("SPACE", &["0", "-5", "1e9", "1e300"]),
        ("KERN", &["-1e9", "1e9", "1e300"]),
        ("MIN", &["0", "-5", "1e-9", "1e9", "1e300"]),
        ("MAX", &["0", "-5", "1e-9", "1e9", "1e300"]),
    ];
    let dir = std::env::temp_dir().join(format!("emb-p18-probe-font-{}", std::process::id()));
    let mut problems = Vec::new();
    let (mut cases, mut refused) = (0, 0);
    for (key, values) in wrong {
        for value in *values {
            cases += 1;
            let mut ltr = LTR.to_string();
            let mut json = JSON.to_string();
            for (k, v) in defaults {
                let v = if k == *key { *value } else { v };
                ltr = ltr.replace(k, v);
                json = json.replace(k, v);
            }
            let case = dir.join(format!("{key}-{cases}"));
            std::fs::create_dir_all(&case).unwrap();
            std::fs::write(case.join("ltr.svg"), &ltr).unwrap();
            std::fs::write(case.join("font.json"), &json).unwrap();
            let label = format!("{key} = {value}");
            let job = move || {
                use emb_letter::LetterSource;
                let c = emb_letter::inkstitch::convert_inkstitch_font(&case.join("font.json"))?;
                let font = c.font;
                let mut d = Design::new("probe");
                let t = d.add_thread("Black", Rgb(0, 0, 0));
                let mut l = emb_model::Lettering::new("AV A", font.font_ref(), 20.0, Point::ZERO);
                emb_letter::relayout(&mut l, &font);
                d.add_object(t, ObjectKind::Lettering(Box::new(l)));
                let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
                Ok::<_, String>(plan.stitches.stitch_count())
            };
            match timed(&label, Duration::from_secs(5), Duration::from_secs(120), job) {
                Ok((Ok(n), took)) => println!("{label}: {n} stitches, {took:.2?}"),
                Ok((Err(e), _)) => {
                    refused += 1;
                    println!("{label}: refused: {e}");
                }
                Err(e) => {
                    println!("PROBLEM {e}");
                    problems.push(e);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("{cases} cases, {refused} refused");
    assert!(problems.is_empty(), "{problems:#?}");
}
