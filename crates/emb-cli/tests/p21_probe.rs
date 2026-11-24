//! A probe of phase P21 (the audit of P21): outlines that are hard or wrong
//! for the split of the outline of a letter and for the stroke finder after
//! it (teeth, steps, rough edges, rings that cross, numbers that are not
//! real), lettering at wrong sizes, the fonts of a folder as lettering,
//! columns that are hard for the untangled rungs, and dense columns in a
//! stitch file. Each case runs in a thread with a time limit.
//! `cargo test --release -p emb-cli --test p21_probe -- --ignored --nocapture`
//! (`EMB_FONT_DIR=<folder>`: the fonts for the lettering, for example the
//! fonts of the computer).

use std::sync::Arc;
use std::time::{Duration, Instant};

use emb_geom::{Path, Point, Transform};
use emb_letter::stroke::Split;
use emb_letter::{Font, GlyphShapes, LetterSource};
use emb_model::{Design, Lettering, ObjectKind, Rgb, SatinGeometry, SatinParams, StitchDesign, StitchRecord};

const TEST_FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/noto-sans-jp-bold-subset.ttf");

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

/// Sews `d`. The stitch count, and a problem text for a stitch that is not
/// a real number or is out of the range of a project.
fn sew(d: &Design) -> (usize, Option<String>) {
    let plan = emb_stitch::build_plan(d, &emb_stitch::PlanOptions::default(), &mut Default::default());
    let m = emb_model::project::MAX_COORD_MM;
    let bad = plan
        .stitches
        .records
        .iter()
        .find(|r| !r.pos.x.is_finite() || !r.pos.y.is_finite() || r.pos.x.abs() > m || r.pos.y.abs() > m)
        .map(|r| format!("a stitch at {:?}", r.pos));
    (plan.stitches.stitch_count(), bad)
}

fn sew_parts(parts: Vec<ObjectKind>) -> (usize, Option<String>) {
    let mut d = Design::new("probe");
    let t = d.add_thread("Black", Rgb(0, 0, 0));
    for p in parts {
        d.add_object(t, p);
    }
    sew(&d)
}

/// Lays out and sews `text` in `src` at `height` mm.
fn letter_and_sew(src: &dyn LetterSource, text: &str, height: f64) -> (usize, Option<String>) {
    let mut l = Lettering::new(text, src.font_ref(), height, Point::ZERO);
    emb_letter::relayout(&mut l, src);
    let mut d = Design::new("probe");
    let t = d.add_thread("Black", Rgb(0, 0, 0));
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    sew(&d)
}

fn polar(c: Point, r: f64, a: f64) -> Point {
    Point::new(c.x + r * emb_geom::math::cos(a), c.y + r * emb_geom::math::sin(a))
}

/// A ring: `n` points on a circle of radius `r` round `c`.
fn circle(c: Point, r: f64, n: usize, clockwise: bool) -> Vec<Point> {
    let turn = if clockwise { -1.0 } else { 1.0 };
    (0..n)
        .map(|i| polar(c, r, i as f64 / n as f64 * std::f64::consts::TAU * turn))
        .collect()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn poly(pts: &[(f64, f64)]) -> Vec<Point> {
    pts.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// Numbers from 0 to 1 that are the same in each run.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `ring` with a point each `step` on its sides, each moved up to `by`.
fn rough(ring: &[Point], step: f64, by: f64, seed: u64) -> Vec<Point> {
    let mut noise = Noise(seed);
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let n = (a.distance(b) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let p = a + (b - a) * (k as f64 / n as f64);
            out.push(Point::new(
                p.x + (noise.next() - 0.5) * 2.0 * by,
                p.y + (noise.next() - 0.5) * 2.0 * by,
            ));
        }
    }
    out
}

/// A wheel of `n` pointed teeth: a sharp notch between each two.
fn gear(c: Point, outer: f64, inner: f64, n: usize) -> Vec<Point> {
    (0..2 * n)
        .map(|i| {
            polar(
                c,
                if i % 2 == 0 { outer } else { inner },
                i as f64 * std::f64::consts::PI / n as f64,
            )
        })
        .collect()
}

/// A bar with `n` teeth that hang from it, as one ring.
fn comb(n: usize) -> Vec<Point> {
    let mut out = vec![Point::new(0.0, 640.0), Point::new(0.0, 600.0)];
    let pitch = 660.0 / n as f64;
    for i in 0..n {
        let x = i as f64 * pitch;
        out.extend(poly(&[
            (x, 0.0),
            (x + pitch * 0.55, 0.0),
            (x + pitch * 0.55, 600.0),
            (x + pitch, 600.0),
        ]));
    }
    out.push(Point::new(660.0, 640.0));
    out
}

/// A band of `n` steps from the bottom left to the top right: an elbow at
/// each step.
fn stairs(n: usize, thick: f64) -> Vec<Point> {
    let s = 700.0 / n as f64;
    let mut out = Vec::new();
    for i in 0..n {
        out.push(Point::new(i as f64 * s + thick, i as f64 * s));
        out.push(Point::new((i + 1) as f64 * s + thick, i as f64 * s));
    }
    out.push(Point::new(n as f64 * s + thick, n as f64 * s));
    for i in (0..n).rev() {
        out.push(Point::new((i + 1) as f64 * s, (i + 1) as f64 * s));
        out.push(Point::new(i as f64 * s, (i + 1) as f64 * s));
    }
    out.push(Point::new(0.0, 0.0));
    out
}

/// A stroke of the width `w` that winds `turns` times out from `c`.
fn spiral(c: Point, turns: f64, w: f64) -> Vec<Point> {
    let n = (turns * 90.0) as usize;
    let at = |i: usize, off: f64| {
        let a = i as f64 / n as f64 * turns * std::f64::consts::TAU;
        polar(c, 40.0 + 300.0 * i as f64 / n as f64 + off, a)
    };
    (0..=n)
        .map(|i| at(i, w / 2.0))
        .chain((0..=n).rev().map(|i| at(i, -w / 2.0)))
        .collect()
}

const E: &[(f64, f64)] = &[
    (0.0, 0.0),
    (420.0, 0.0),
    (420.0, 110.0),
    (120.0, 110.0),
    (120.0, 290.0),
    (380.0, 290.0),
    (380.0, 400.0),
    (120.0, 400.0),
    (120.0, 590.0),
    (420.0, 590.0),
    (420.0, 700.0),
    (0.0, 700.0),
];
const Z: &[(f64, f64)] = &[
    (0.0, 0.0),
    (460.0, 0.0),
    (460.0, 110.0),
    (160.0, 110.0),
    (460.0, 600.0),
    (460.0, 700.0),
    (10.0, 700.0),
    (10.0, 590.0),
    (300.0, 590.0),
    (0.0, 100.0),
];
/// A heavy "n": a narrow counter between two stems.
const N: &[(f64, f64)] = &[
    (0.0, 0.0),
    (210.0, 0.0),
    (210.0, 330.0),
    (250.0, 330.0),
    (250.0, 0.0),
    (460.0, 0.0),
    (460.0, 430.0),
    (360.0, 520.0),
    (230.0, 520.0),
    (210.0, 500.0),
    (210.0, 520.0),
    (0.0, 520.0),
];

/// What the steps of a letter gave.
struct Steps {
    /// The columns with the split, and with no split.
    columns: usize,
    plain: usize,
    /// The part of the rings that the columns cover (`quality::areas`):
    /// with no split, with the split, after the tuning of the ends, and
    /// after the route.
    cover_plain: f64,
    cover_split: f64,
    cover_tuned: f64,
    cover_routed: f64,
    stitches: usize,
    bad: Option<String>,
    /// The time of each step.
    times: Vec<(&'static str, Duration)>,
}

/// The steps of the app and of the converter on the rings of a Latin
/// letter: the split and the stroke finder, the tuning of the ends, the
/// route in the outline, the untangled rungs of the layout, and the
/// stitches.
fn letter_steps(rings: Vec<Vec<Point>>, cap: f64) -> Steps {
    let mut times = Vec::new();
    let mut start = Instant::now();
    let mut step = |name: &'static str| {
        times.push((name, start.elapsed()));
        start = Instant::now();
    };
    let shapes = GlyphShapes::with_split(&rings, cap, Split::Notches);
    step("split");
    let plain = GlyphShapes::new(&rings, cap);
    step("no split");
    let design_cap = emb_letter::embfont::DESIGN_CAP_MM;
    let s = design_cap / cap;
    let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect() };
    let mut parts = shapes.satin_parts();
    for p in &mut parts {
        p.transform(&Transform::scale(s, s));
    }
    let fills: Vec<(Vec<Point>, Vec<Vec<Point>>)> = shapes
        .fills
        .iter()
        .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect()))
        .collect();
    let cover = |parts: &[ObjectKind]| emb_letter::quality::areas(parts, &fills).map_or(0.0, |a| a.coverage);
    let cover_split = cover(&parts);
    let cover_plain = {
        let mut parts = plain.satin_parts();
        for p in &mut parts {
            p.transform(&Transform::scale(s, s));
        }
        cover(&parts)
    };
    step("measure");
    emb_letter::tune::round_parts(&mut parts, &fills, design_cap);
    emb_letter::tune::extend_ends(&mut parts, &fills, design_cap);
    step("ends");
    let cover_tuned = cover(&parts);
    step("measure");
    let letter = emb_letter::EmbLetter {
        advance_mm: 20.0,
        parts,
        fixed: false,
        tuning: Vec::new(),
        fill_parts: Vec::new(),
    };
    let outline: Vec<Vec<Point>> = rings.iter().filter(|r| r.len() > 2).map(scale).collect();
    let (routed, _) = emb_letter::route::route_with_splits_dir(&letter, design_cap, Some(&outline), false);
    step("route");
    let mut parts = routed.parts;
    emb_letter::glyph::untangle_parts(&mut parts, design_cap);
    step("untangle");
    let cover_routed = cover(&parts);
    step("measure");
    let (stitches, bad) = sew_parts(parts);
    step("stitches");
    Steps {
        columns: shapes.strokes.satins.len(),
        plain: plain.strokes.satins.len(),
        cover_plain,
        cover_split,
        cover_tuned,
        cover_routed,
        stitches,
        bad,
        times,
    }
}

#[test]
#[ignore]
fn probe_p21_outlines() {
    std::panic::set_hook(Box::new(|_| {}));
    let cap = 700.0;
    let o = Point::new(400.0, 350.0);
    let star: Vec<Point> = (0..5)
        .map(|i| polar(o, 300.0, i as f64 * 2.0 * std::f64::consts::TAU / 5.0))
        .collect();
    let dense = |pts: &[(f64, f64)], step: f64| rough(&poly(pts), step, 0.0, 1);
    let scaled =
        |pts: &[(f64, f64)], s: f64| -> Vec<Point> { pts.iter().map(|&(x, y)| Point::new(x * s, y * s)).collect() };
    // A lattice of 12 by 12 strokes: 144 crossings.
    let mut lattice = Vec::new();
    for i in 0..12 {
        let x = 30.0 + i as f64 * 55.0;
        lattice.push(rect(x, 0.0, x + 30.0, 700.0));
        lattice.push(rect(0.0, x, 700.0, x + 30.0));
    }
    let mut with_nan = poly(E);
    with_nan[4] = Point::new(f64::NAN, 290.0);
    let mut with_inf = poly(E);
    with_inf[4] = Point::new(f64::INFINITY, 290.0);
    let mut with_huge = poly(E);
    with_huge[4] = Point::new(1e300, 290.0);
    // The name, the rings, the capital height, and `true` if the numbers
    // are wrong (then only a panic or a hang is a problem).
    let cases: Vec<(&str, Vec<Vec<Point>>, f64, bool)> = vec![
        ("an E", vec![poly(E)], cap, false),
        ("a Z", vec![poly(Z)], cap, false),
        ("a heavy n", vec![poly(N)], cap, false),
        (
            "a ring of 20,000 points",
            vec![circle(o, 300.0, 20_000, false), circle(o, 200.0, 20_000, true)],
            cap,
            false,
        ),
        ("a wheel of 60 teeth", vec![gear(o, 340.0, 250.0, 60)], cap, false),
        ("a wheel of 300 teeth", vec![gear(o, 340.0, 300.0, 300)], cap, false),
        ("a wheel of 1,500 teeth", vec![gear(o, 340.0, 330.0, 1500)], cap, false),
        (
            "a wheel of 300 teeth with a hole of 300 teeth",
            vec![gear(o, 340.0, 300.0, 300), {
                let mut hole = gear(o, 200.0, 160.0, 300);
                hole.reverse();
                hole
            }],
            cap,
            false,
        ),
        ("a comb of 60 teeth, one ring", vec![comb(60)], cap, false),
        ("a comb of 200 teeth, one ring", vec![comb(200)], cap, false),
        ("stairs of 12 steps", vec![stairs(12, 60.0)], cap, false),
        ("stairs of 100 steps", vec![stairs(100, 60.0)], cap, false),
        ("stairs of 400 steps", vec![stairs(400, 30.0)], cap, false),
        (
            "a rough stem, 1,600 points",
            vec![rough(&rect(0.0, 0.0, 100.0, 700.0), 1.0, 3.0, 7)],
            cap,
            false,
        ),
        (
            "a rough E, 3,200 points",
            vec![rough(&poly(E), 1.0, 4.0, 11)],
            cap,
            false,
        ),
        (
            "a rough ring, 6,000 points",
            vec![rough(&circle(o, 320.0, 60, false), 0.5, 5.0, 3), {
                let mut hole = rough(&circle(o, 200.0, 60, false), 0.5, 5.0, 5);
                hole.reverse();
                hole
            }],
            cap,
            false,
        ),
        ("an E with 12,000 points on its sides", vec![dense(E, 0.25)], cap, false),
        ("a Z with 9,000 points on its sides", vec![dense(Z, 0.25)], cap, false),
        (
            "an E with each point three times",
            vec![poly(E).into_iter().flat_map(|p| [p, p, p]).collect()],
            cap,
            false,
        ),
        (
            "an E the wrong way round",
            vec![poly(E).into_iter().rev().collect()],
            cap,
            false,
        ),
        ("a spiral of 8 turns", vec![spiral(o, 8.0, 16.0)], cap, false),
        ("a spiral of 30 turns", vec![spiral(o, 30.0, 5.0)], cap, false),
        ("a star that crosses itself", vec![star], cap, false),
        (
            "50 rings in each other",
            (0..50)
                .map(|i| circle(o, 340.0 - 6.5 * i as f64, 64, i % 2 == 1))
                .collect(),
            cap,
            false,
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
            cap,
            false,
        ),
        ("a lattice of 24 strokes that overlap", lattice.clone(), cap, false),
        (
            "a lattice of 24 strokes, one area",
            emb_geom::region::without_overlaps(&lattice),
            cap,
            false,
        ),
        ("40 letters E on each other", vec![poly(E); 40], cap, false),
        ("a sliver", vec![rect(0.0, 0.0, 700.0, 0.001)], cap, false),
        ("all points the same", vec![vec![o; 50]], cap, false),
        ("two points", vec![vec![o, Point::new(500.0, 500.0)]], cap, false),
        ("an empty ring", vec![vec![]], cap, false),
        ("no rings", vec![], cap, false),
        ("an E of 1e-6 units", vec![scaled(E, 1e-9)], cap * 1e-9, false),
        ("an E of 1e9 units", vec![scaled(E, 1e6)], cap * 1e6, false),
        ("an E 1,000 times its capital height", vec![poly(E)], 0.7, false),
        (
            "an E a thousandth of its capital height",
            vec![poly(E)],
            700_000.0,
            false,
        ),
        ("an E with a point that is not a number", vec![with_nan], cap, true),
        ("an E with a point at infinity", vec![with_inf], cap, true),
        ("an E with a point at 1e300", vec![with_huge], cap, true),
        ("an E with the capital height 0", vec![poly(E)], 0.0, true),
        ("an E with a capital height below 0", vec![poly(E)], -700.0, true),
        (
            "an E with a capital height that is not a number",
            vec![poly(E)],
            f64::NAN,
            true,
        ),
    ];
    let only = std::env::var("EMB_PROBE_ONLY").ok();
    let mut problems = Vec::new();
    for (what, rings, cap, wrong) in cases {
        if only.as_ref().is_some_and(|o| !o.split(';').any(|o| what.contains(o))) {
            continue;
        }
        // The layout waits for the split (at most 10 s here); the app makes
        // the other steps in the route thread of the font.
        match timed(what, Duration::from_secs(300), Duration::from_secs(300), move || {
            letter_steps(rings, cap)
        }) {
            Ok((s, took)) => {
                let split = s.times.first().map_or(Duration::ZERO, |t| t.1);
                if split > Duration::from_secs(10) {
                    problems.push(format!("{what}: the split takes {split:.2?}"));
                }
                if !wrong && s.cover_routed < s.cover_split - 0.02 {
                    problems.push(format!(
                        "{what}: covered {:.1} %, {:.1} % after the route",
                        s.cover_split * 100.0,
                        s.cover_routed * 100.0
                    ));
                }
                println!(
                    "{what}: {} columns ({} with no split), covered {:.1} % ({:.1} % with no split, {:.1} % after the ends, {:.1} % after the route), {} stitches, {took:.2?}",
                    s.columns,
                    s.plain,
                    s.cover_split * 100.0,
                    s.cover_plain * 100.0,
                    s.cover_tuned * 100.0,
                    s.cover_routed * 100.0,
                    s.stitches,
                );
                if took > Duration::from_secs(1) {
                    let times: Vec<String> = s.times.iter().map(|(n, t)| format!("{n} {t:.2?}")).collect();
                    println!("  ({})", times.join(", "));
                }
                if let Some(b) = s.bad {
                    if wrong {
                        println!("  (wrong numbers in: {b})");
                    } else {
                        problems.push(format!("{what}: {b}"));
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

#[test]
#[ignore]
fn probe_p21_sizes() {
    std::panic::set_hook(Box::new(|_| {}));
    let font = Arc::new(Font::from_bytes(std::fs::read(TEST_FONT).expect("the test font"), 0).expect("a font"));
    let mut problems = Vec::new();
    for height in [
        25.0,
        0.0,
        -25.0,
        1e-9,
        1e-300,
        0.01,
        2900.0,
        1e4,
        1e5,
        1e6,
        1e7,
        1e9,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let f = font.clone();
        let what = format!("Hamburg 2026 at {height} mm");
        match timed(&what, Duration::from_secs(20), Duration::from_secs(120), move || {
            letter_and_sew(&*f, "Hamburg 2026", height)
        }) {
            Ok(((stitches, bad), took)) => {
                println!("{what}: {stitches} stitches, {took:.2?}");
                if let Some(b) = bad {
                    problems.push(format!("{what}: {b}"));
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

/// The letters, digits and marks of a font of the computer as lettering at
/// 25 mm: the split of each glyph, the route and the stitches.
#[test]
#[ignore]
fn probe_p21_fonts() {
    std::panic::set_hook(Box::new(|_| {}));
    let Ok(dir) = std::env::var("EMB_FONT_DIR") else {
        println!("EMB_FONT_DIR is not set: no fonts");
        return;
    };
    let text = "ABCDEFGHIJKLM\nNOPQRSTUVWXYZ\nabcdefghijklm\nnopqrstuvwxyz\n0123456789\n\u{C4}\u{D6}\u{DC}\u{DF}\u{E9}\u{F1}\u{E7}\u{F8}\u{C5}&@?!%";
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("the folder of the fonts")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        })
        .collect();
    files.sort();
    // Some fonts at the same time: each on a thread of its own.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new((Vec::new(), Vec::new(), 0usize));
    let workers = std::thread::available_parallelism().map_or(4, |n| (n.get() / 2).max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = files.get(i) else { break };
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let Ok(bytes) = std::fs::read(path) else {
                        results.lock().unwrap().2 += 1;
                        continue;
                    };
                    let read = timed(&name, Duration::from_secs(20), Duration::from_secs(120), move || {
                        Font::from_bytes(bytes, 0).ok().map(Arc::new)
                    });
                    let font = match read {
                        Ok((Some(f), _)) => f,
                        Ok((None, _)) => {
                            results.lock().unwrap().2 += 1;
                            continue;
                        }
                        Err(e) => {
                            results.lock().unwrap().0.push(e);
                            continue;
                        }
                    };
                    let sewn = timed(&name, Duration::from_secs(300), Duration::from_secs(900), move || {
                        letter_and_sew(&*font, text, 25.0)
                    });
                    let mut r = results.lock().unwrap();
                    match sewn {
                        Ok(((stitches, bad), took)) => {
                            println!("{name}: {stitches} stitches, {took:.2?}");
                            r.1.push((took, name.clone()));
                            if let Some(b) = bad {
                                r.0.push(format!("{name}: {b}"));
                            }
                        }
                        Err(e) => r.0.push(e),
                    }
                }
            });
        }
    });
    let (problems, mut times, not_read) = results.into_inner().unwrap();
    times.sort();
    let all: Duration = times.iter().map(|t| t.0).sum();
    println!(
        "{} fonts sewn in {all:.0?}, {not_read} files not read as a font",
        times.len()
    );
    for (took, name) in times.iter().rev().take(8) {
        println!("  slow: {name} {took:.2?}");
    }
    println!("problems {}", problems.len());
    for p in &problems {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}

fn line(pts: &[(f64, f64)]) -> Path {
    let mut it = pts.iter().map(|&(x, y)| Point::new(x, y));
    let Some(first) = it.next() else { return Path::new() };
    it.fold(Path::new().move_to(first), |p, q| p.line_to(q))
}

fn column(a: &[(f64, f64)], b: &[(f64, f64)]) -> ObjectKind {
    ObjectKind::Satin {
        geometry: SatinGeometry {
            rail_a: line(a),
            rail_b: line(b),
            rungs: Vec::new(),
        },
        params: SatinParams::default(),
    }
}

/// Columns that are hard for the untangled rungs (the last step of the
/// layout of lettering), then sewn.
#[test]
#[ignore]
fn probe_p21_columns() {
    std::panic::set_hook(Box::new(|_| {}));
    let zigzag = |n: usize, back: f64| -> Vec<(f64, f64)> {
        let mut x = 0.0;
        (0..n)
            .map(|i| {
                x += if i % 2 == 0 { 1.0 } else { -back };
                (x, 3.0)
            })
            .collect()
    };
    let straight = [(0.0, 0.0), (40.0, 0.0)];
    // The name, the column, the capital height, and `true` if the numbers
    // are wrong.
    let cases: Vec<(&str, ObjectKind, f64, bool)> = vec![
        (
            "a straight column",
            column(&straight, &[(0.0, 3.0), (40.0, 3.0)]),
            25.0,
            false,
        ),
        (
            "rails that cross",
            column(&straight, &[(0.0, 3.0), (40.0, -3.0)]),
            25.0,
            false,
        ),
        (
            "a rail the other way",
            column(&straight, &[(40.0, 3.0), (0.0, 3.0)]),
            25.0,
            false,
        ),
        (
            "a rail that goes back at each step, 200 points",
            column(&straight, &zigzag(200, 0.6)),
            25.0,
            false,
        ),
        (
            "a rail that goes back at each step, 40,000 points",
            column(&straight, &zigzag(40_000, 0.999)),
            25.0,
            false,
        ),
        (
            "a rail that does not go on, 2,000 points",
            column(&straight, &zigzag(2_000, 1.0)),
            25.0,
            false,
        ),
        ("a rail of one point", column(&straight, &[(0.0, 3.0)]), 25.0, false),
        ("a rail with no points", column(&straight, &[]), 25.0, false),
        (
            "rails with no length",
            column(&[(5.0, 5.0), (5.0, 5.0)], &[(5.0, 5.0), (5.0, 5.0)]),
            25.0,
            false,
        ),
        ("the two rails the same", column(&straight, &straight), 25.0, false),
        (
            "a column of 2.9 m",
            column(&[(0.0, 0.0), (2900.0, 0.0)], &[(0.0, 3.0), (2900.0, -3.0)]),
            25.0,
            false,
        ),
        (
            "rails that cross, capital height 0.5 mm",
            column(&straight, &[(0.0, 3.0), (40.0, -3.0)]),
            0.5,
            false,
        ),
        (
            "rails that cross, capital height 0",
            column(&straight, &[(0.0, 3.0), (40.0, -3.0)]),
            0.0,
            true,
        ),
        (
            "rails that cross, capital height below 0",
            column(&straight, &[(0.0, 3.0), (40.0, -3.0)]),
            -25.0,
            true,
        ),
        (
            "rails that cross, capital height not a number",
            column(&straight, &[(0.0, 3.0), (40.0, -3.0)]),
            f64::NAN,
            true,
        ),
        (
            "a rail with a point that is not a number",
            column(&straight, &[(0.0, 3.0), (f64::NAN, 3.0), (40.0, -3.0)]),
            25.0,
            true,
        ),
        (
            "a rail with a point at infinity",
            column(&straight, &[(0.0, 3.0), (f64::INFINITY, 3.0), (40.0, -3.0)]),
            25.0,
            true,
        ),
    ];
    let mut problems = Vec::new();
    for (what, part, cap, wrong) in cases {
        match timed(what, Duration::from_secs(5), Duration::from_secs(60), move || {
            let mut parts = vec![part];
            emb_letter::glyph::untangle_parts(&mut parts, cap);
            let rungs = match &parts[0] {
                ObjectKind::Satin { geometry, .. } => geometry.rungs.len(),
                _ => 0,
            };
            (rungs, sew_parts(parts))
        }) {
            Ok(((rungs, (stitches, bad)), took)) => {
                println!("{what}: {rungs} rungs, {stitches} stitches, {took:.2?}");
                if let Some(b) = bad {
                    if wrong {
                        println!("  (wrong numbers in: {b})");
                    } else {
                        problems.push(format!("{what}: {b}"));
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

/// Dense columns and lines in a stitch file: the rule for a bean stitch
/// (P21 Milestone 13) looks at the needle holes of the legs before a leg.
#[test]
#[ignore]
fn probe_p21_stitch_files() {
    std::panic::set_hook(Box::new(|_| {}));
    let one = |records: Vec<StitchRecord>| StitchDesign {
        records,
        ..Default::default()
    };
    // A column of `n` stitches across `width`, `step` along for each.
    let zigzag = |n: usize, width: f64, step: f64| -> Vec<StitchRecord> {
        (0..n)
            .map(|i| StitchRecord::stitch(i as f64 * step, if i % 2 == 0 { 0.0 } else { width }))
            .collect()
    };
    // A bean stitch: each stitch forward, back and forward.
    let bean = |n: usize, len: f64| -> Vec<StitchRecord> {
        (0..n)
            .flat_map(|i| {
                let x = i as f64 * len;
                [x + len, x, x + len].map(|x| StitchRecord::stitch(x, 0.0))
            })
            .collect()
    };
    let cases: Vec<(String, StitchDesign)> = [20_000, 100_000, 400_000]
        .into_iter()
        .flat_map(|n| {
            [
                (
                    format!("a column of {n} stitches, 0.02 mm apart"),
                    one(zigzag(n, 3.0, 0.02)),
                ),
                (
                    format!("a column of {n} stitches, 0.2 mm apart"),
                    one(zigzag(n, 3.0, 0.2)),
                ),
                (
                    format!("{n} stitches on one line, in two holes"),
                    one(zigzag(n, 3.0, 0.0)),
                ),
                (format!("a bean stitch of {n} stitches"), one(bean(n / 3, 2.5))),
            ]
        })
        .collect();
    let mut problems = Vec::new();
    for (what, design) in cases {
        match timed(&what, Duration::from_secs(10), Duration::from_secs(300), move || {
            let parts = emb_art::parts::parts(&design);
            let kinds: Vec<String> = parts.iter().take(3).map(|p| format!("{:?}", p.kind)).collect();
            (parts.len(), kinds)
        }) {
            Ok(((n, kinds), took)) => println!("{what}: {n} parts ({}), {took:.2?}", kinds.join(", ")),
            Err(e) => problems.push(e),
        }
    }
    println!("problems {}", problems.len());
    for p in &problems {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}

/// The time of the steps of each glyph of one font (`EMB_FONT=<file>`,
/// `EMB_FONT_TEXT=<characters>`): the outline, the stroke finder with no
/// split and with the split, and the route.
#[test]
#[ignore]
fn probe_p21_font_times() {
    let Ok(file) = std::env::var("EMB_FONT") else {
        println!("EMB_FONT is not set: no font");
        return;
    };
    let text = std::env::var("EMB_FONT_TEXT")
        .unwrap_or_else(|_| "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".into());
    let font = Font::from_bytes(std::fs::read(&file).expect("the font file"), 0).expect("a font");
    // The route of the columns of `shapes`, as `Font::route` makes it.
    let route_of = |shapes: &GlyphShapes, rings: &[Vec<Point>]| -> usize {
        let cap = emb_letter::embfont::DESIGN_CAP_MM;
        let s = cap / font.cap_height;
        let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect() };
        let mut parts = shapes.satin_parts();
        for p in &mut parts {
            p.transform(&Transform::scale(s, s));
        }
        let fills: Vec<(Vec<Point>, Vec<Vec<Point>>)> = shapes
            .fills
            .iter()
            .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect()))
            .collect();
        emb_letter::tune::round_parts(&mut parts, &fills, cap);
        emb_letter::tune::extend_ends(&mut parts, &fills, cap);
        let letter = emb_letter::EmbLetter {
            advance_mm: 20.0,
            parts,
            fixed: false,
            tuning: Vec::new(),
            fill_parts: Vec::new(),
        };
        let outline: Vec<Vec<Point>> = rings.iter().map(scale).collect();
        emb_letter::route::route_with_splits_dir(&letter, cap, Some(&outline), false)
            .0
            .parts
            .len()
    };
    let mut sum = [Duration::ZERO; 5];
    for c in text.chars() {
        // A symbol font has no glyph for a letter in its tables, but the
        // text shaping gives one (the layout uses it).
        let shaped = || {
            font.shape_line(&c.to_string(), false)
                .and_then(|v| v.first().map(|g| g.letter as u16))
                .filter(|g| *g != 0)
        };
        let Some(g) = font.glyph(c).or_else(shaped) else {
            continue;
        };
        // The step that runs now (a glyph that does not end shows it).
        let now = |step: &str| {
            use std::io::Write;
            eprint!("\r{c}: {step}        ");
            std::io::stderr().flush().ok();
        };
        now("outline");
        let t = Instant::now();
        let rings = font.contours(g);
        let outline = t.elapsed();
        now("no split");
        let t = Instant::now();
        let plain = GlyphShapes::new(&rings, font.cap_height);
        let no_split = t.elapsed();
        now("split");
        let t = Instant::now();
        let shapes = font.shapes(g);
        let split = t.elapsed();
        now("route");
        let t = Instant::now();
        let parts = font.route(g).map_or(0, |r| r.len());
        let route = t.elapsed();
        now("route with no split");
        let t = Instant::now();
        let plain_parts = route_of(&plain, &rings);
        let plain_route = t.elapsed();
        eprint!("\r");
        println!(
            "{c}: {} points, {} columns ({} with no split), {parts} parts ({plain_parts}); outline {outline:.1?}, no split {no_split:.1?}, split {split:.1?}, route {route:.1?}, route with no split {plain_route:.1?}",
            rings.iter().map(Vec::len).sum::<usize>(),
            shapes.strokes.satins.len(),
            plain.strokes.satins.len(),
        );
        for (s, t) in sum.iter_mut().zip([outline, no_split, split, route, plain_route]) {
            *s += t;
        }
    }
    println!(
        "all: outline {:.2?}, no split {:.2?}, split {:.2?}, route {:.2?}, route with no split {:.2?}",
        sum[0], sum[1], sum[2], sum[3], sum[4]
    );
}
