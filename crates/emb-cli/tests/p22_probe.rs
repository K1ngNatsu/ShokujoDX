//! A probe of phase P22 (the audit of P22): designs and stitch files that
//! are hard or wrong for the new code of the phase. Travel that shows
//! (Milestone 1) far apart and in great numbers, Quicker to Sew (Milestone
//! 2) with wrong settings, test swatches (Milestone 3) with wrong ranges,
//! satin lean and bands (Milestone 4) with wrong numbers, and the check and
//! fix of machine files (Milestone 5) with large, empty and damaged files.
//! Each case runs in a thread with a time limit; `probe_p22` runs each case
//! in a process of its own, so a case that stops its process stops only
//! itself.
//! `cargo test --release -p emb-cli --test p22_probe -- --ignored --nocapture probe_p22`

use std::time::{Duration, Instant};

use emb_art::file_check::{self, Fixes};
use emb_geom::{BBox, Path, Point};
use emb_letter::swatch::{SwatchSetting, SwatchSpec};
use emb_model::{
    ColumnBand, Design, FillParams, ObjectKind, Rgb, RunningParams, SatinGeometry, SatinParams, StitchDesign,
    StitchKind, StitchRecord,
};
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

/// A problem text for a stitch that is not a real number or is out of the
/// range of a project.
fn bad_stitch(s: &StitchDesign) -> Option<String> {
    let m = emb_model::project::MAX_COORD_MM;
    s.records
        .iter()
        .find(|r| !r.pos.x.is_finite() || !r.pos.y.is_finite() || r.pos.x.abs() > m || r.pos.y.abs() > m)
        .map(|r| format!("a stitch at {:?}", r.pos))
}

fn square(c: Point, side: f64) -> Path {
    Path::rect(BBox::new(
        Point::new(c.x - side / 2.0, c.y - side / 2.0),
        Point::new(c.x + side / 2.0, c.y + side / 2.0),
    ))
}

fn fill(c: Point, side: f64) -> ObjectKind {
    ObjectKind::Fill {
        outer: square(c, side),
        holes: vec![],
        params: FillParams::default(),
    }
}

/// Two small fills of one thread, 2 mm apart: travel between them.
fn cluster(d: &mut Design, t: emb_model::ThreadId, at: Point) {
    d.add_object(t, fill(at, 5.0));
    d.add_object(t, fill(Point::new(at.x + 7.0, at.y), 5.0));
}

fn satin(len: f64, params: SatinParams) -> ObjectKind {
    ObjectKind::Satin {
        geometry: SatinGeometry {
            rail_a: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(len, 0.0)),
            rail_b: Path::new().move_to(Point::new(0.0, 4.0)).line_to(Point::new(len, 4.0)),
            rungs: vec![],
        },
        params,
    }
}

fn line(len: f64) -> ObjectKind {
    ObjectKind::Running {
        path: Path::new()
            .move_to(Point::new(0.0, 10.0))
            .line_to(Point::new(len, 10.0)),
        params: RunningParams::default(),
    }
}

fn records(n: usize, f: impl Fn(usize) -> StitchRecord) -> StitchDesign {
    StitchDesign {
        records: (0..n).map(f).collect(),
        ..Default::default()
    }
}

/// The names of the cases.
const CASES: [&str; 6] = [
    "p22_travel_far_apart",
    "p22_travel_many",
    "p22_quicker",
    "p22_swatches",
    "p22_lean_and_bands",
    "p22_machine_files",
];

/// Runs each case in a process of its own, and lists the result of each.
#[test]
#[ignore]
fn probe_p22() {
    let exe = std::env::current_exe().unwrap();
    for case in CASES {
        let start = Instant::now();
        let out = std::process::Command::new(&exe)
            .args(["--ignored", "--exact", "--nocapture", case])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("  ")).collect();
        println!(
            "{case}: {} in {:.1?}",
            if out.status.success() {
                "ok".to_string()
            } else {
                format!("the process stopped ({})", out.status)
            },
            start.elapsed()
        );
        for l in lines {
            println!("{l}");
        }
    }
}

/// Travel at two places of a design that are far apart (both in the range
/// of a project): the measure of the travel that shows.
#[test]
#[ignore]
fn p22_travel_far_apart() {
    // (Before the audit, the maps of the measure had a cell of 1 mm for
    // each place of the box of all the travel: 10^10 cells for travel
    // 100 m apart, more memory than a computer has.)
    for (name, a, b) in [
        ("1 m apart", Point::new(0.0, 0.0), Point::new(1_000.0, 1_000.0)),
        ("10 m apart", Point::new(0.0, 0.0), Point::new(10_000.0, 10_000.0)),
        ("100 m apart", Point::new(0.0, 0.0), Point::new(100_000.0, 100_000.0)),
        (
            "1.8 km apart",
            Point::new(-900_000.0, -900_000.0),
            Point::new(900_000.0, 900_000.0),
        ),
    ] {
        let r = timed(name, Duration::from_secs(2), Duration::from_secs(60), move || {
            let mut d = Design::new("probe");
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            cluster(&mut d, t, a);
            cluster(&mut d, t, b);
            let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
            let travel = plan.travel.len();
            let shown = emb_stitch::shown_travel::shown_travel(&plan);
            (travel, shown.pieces.len())
        });
        match r {
            Ok(((t, p), took)) => println!("  {name}: {t} travel runs, {p} pieces that show, {took:.2?}"),
            Err(e) => println!("  PROBLEM {e}"),
        }
    }
}

/// Many objects with travel between them: the time of the measure.
#[test]
#[ignore]
fn p22_travel_many() {
    for n in [1_000usize, 5_000, 20_000] {
        let name = format!("{n} fills in a row");
        let r = timed(&name, Duration::from_secs(10), Duration::from_secs(120), move || {
            let mut d = Design::new("probe");
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            for i in 0..n {
                let (x, y) = ((i % 100) as f64 * 4.0, (i / 100) as f64 * 4.0);
                d.add_object(t, fill(Point::new(x, y), 2.5));
            }
            let start = Instant::now();
            let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
            let planned = start.elapsed();
            let start = Instant::now();
            let shown = emb_stitch::shown_travel::shown_travel(&plan);
            (plan.travel.len(), shown.pieces.len(), planned, start.elapsed())
        });
        match r {
            Ok(((t, p, planned, measured), _)) => {
                println!("  {name}: {t} travel runs, {p} pieces; plan {planned:.2?}, measure {measured:.2?}")
            }
            Err(e) => println!("  PROBLEM {e}"),
        }
    }
}

/// Quicker to Sew with settings that are not real numbers, below 0 or very
/// large, and on a design of many threads and objects.
#[test]
#[ignore]
fn p22_quicker() {
    use emb_model::quicker::{QuickerToSew, apply};
    let base = || {
        let mut d = Design::new("probe");
        for i in 0..400u32 {
            let t = d.add_thread(
                format!("T{i}"),
                Rgb((i % 7) as u8 * 30, (i % 5) as u8 * 40, (i % 3) as u8 * 80),
            );
            let c = Point::new((i % 20) as f64 * 6.0, (i / 20) as f64 * 6.0);
            d.add_object(t, fill(c, if i % 9 == 0 { 0.6 } else { 4.0 }));
        }
        d
    };
    let settings: Vec<(&str, QuickerToSew)> = vec![
        ("standard", QuickerToSew::STANDARD),
        (
            "not numbers",
            QuickerToSew {
                merge_threads: Some(f64::NAN),
                small_details_mm: Some(f64::NAN),
                fewer_color_changes: true,
                gap_mm: Some(f64::NAN),
            },
        ),
        (
            "below 0",
            QuickerToSew {
                merge_threads: Some(-5.0),
                small_details_mm: Some(-1.0),
                fewer_color_changes: true,
                gap_mm: Some(-3.0),
            },
        ),
        (
            "very large",
            QuickerToSew {
                merge_threads: Some(1e300),
                small_details_mm: Some(1e300),
                fewer_color_changes: true,
                gap_mm: Some(1e300),
            },
        ),
    ];
    for (name, s) in settings {
        let r = timed(name, Duration::from_secs(5), Duration::from_secs(60), move || {
            let mut d = base();
            let report = apply(&mut d, &s, &|_| 1);
            let range = emb_model::project::out_of_range(&d, None);
            let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
            (
                report.merged.len(),
                report.removed.len(),
                d.objects().len(),
                range,
                bad_stitch(&plan.stitches),
            )
        });
        match r {
            Ok(((m, rm, left, range, bad), took)) => println!(
                "  {name}: {m} threads merged, {rm} objects removed, {left} left, out of range: {range:?}, {bad:?}, {took:.2?}"
            ),
            Err(e) => println!("  PROBLEM {e}"),
        }
    }
}

/// Test swatches of each setting with wrong ranges and numbers of copies.
#[test]
#[ignore]
fn p22_swatches() {
    let design = || {
        let mut d = Design::new("probe");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        let ids = vec![
            d.add_object(t, satin(30.0, SatinParams::default())),
            d.add_object(t, fill(Point::new(15.0, 20.0), 10.0)),
            d.add_object(t, line(30.0)),
        ];
        (d, ids)
    };
    let ranges: [(&str, f64, f64, usize); 6] = [
        ("0 copies", 1.0, 2.0, 0),
        ("one copy", 1.0, 1.0, 1),
        ("a million copies", 0.5, 5.0, 1_000_000),
        ("from 1e9 to -1e9", 1e9, -1e9, 5),
        ("not numbers", f64::NAN, f64::INFINITY, 5),
        ("0 to 0", 0.0, 0.0, 6),
    ];
    for setting in SwatchSetting::ALL {
        for (label, from, to, copies) in ranges {
            let name = format!("{} {label}", setting.name());
            let r = timed(&name, Duration::from_secs(5), Duration::from_secs(60), move || {
                let (d, ids) = design();
                let spec = SwatchSpec {
                    setting,
                    from,
                    to,
                    copies,
                    field: Some((100.0, 100.0)),
                };
                let made = emb_letter::swatch::swatches(&d, &ids, &spec, None, &|v| format!("{v:.2}"));
                made.map(|s| {
                    let range = emb_model::project::out_of_range(&s.design, None);
                    let plan = emb_stitch::build_plan(&s.design, &PlanOptions::default(), &mut Default::default());
                    (
                        s.values.len(),
                        range,
                        bad_stitch(&plan.stitches),
                        plan.stitches.stitch_count(),
                    )
                })
            });
            match r {
                Ok((Ok((n, None, None, stitches)), took)) if took > Duration::from_secs(1) => {
                    println!("  {name}: {n} copies, {stitches} stitches, {took:.2?}")
                }
                Ok((Ok((_, None, None, _)), _)) | Ok((Err(_), _)) => {}
                Ok((Ok((n, range, bad, _)), _)) => println!("  PROBLEM {name}: {n} copies, {range:?} {bad:?}"),
                Err(e) => println!("  PROBLEM {e}"),
            }
        }
    }
    println!("  (the other cases: no problem, under 1 s)");
}

/// Satin columns with lean and bands of wrong numbers: the stitches and
/// the Ink/Stitch file.
#[test]
#[ignore]
fn p22_lean_and_bands() {
    let cases: Vec<(&str, f64, SatinParams)> = vec![
        (
            "lean 89°",
            30.0,
            SatinParams {
                lean_deg: 89.0,
                ..Default::default()
            },
        ),
        (
            "lean -1e300",
            30.0,
            SatinParams {
                lean_deg: -1e300,
                ..Default::default()
            },
        ),
        (
            "lean not a number",
            30.0,
            SatinParams {
                lean_deg: f64::NAN,
                ..Default::default()
            },
        ),
        (
            "band 5 of 0",
            30.0,
            SatinParams {
                band: Some(ColumnBand {
                    index: 5,
                    count: 0,
                    overlap_mm: 0.0,
                }),
                ..Default::default()
            },
        ),
        (
            "band 1 of 4e9",
            30.0,
            SatinParams {
                band: Some(ColumnBand {
                    index: 1,
                    count: u32::MAX,
                    overlap_mm: 0.0,
                }),
                ..Default::default()
            },
        ),
        (
            "overlap 1e300",
            30.0,
            SatinParams {
                band: Some(ColumnBand {
                    index: 1,
                    count: 3,
                    overlap_mm: 1e300,
                }),
                ..Default::default()
            },
        ),
        (
            "overlap not a number",
            30.0,
            SatinParams {
                band: Some(ColumnBand {
                    index: 1,
                    count: 3,
                    overlap_mm: f64::NAN,
                }),
                ..Default::default()
            },
        ),
        (
            "a band of a column 1.5 km long",
            1_500_000.0,
            SatinParams {
                band: Some(ColumnBand {
                    index: 0,
                    count: 3,
                    overlap_mm: 0.3,
                }),
                ..Default::default()
            },
        ),
    ];
    for (name, len, params) in cases {
        let r = timed(name, Duration::from_secs(5), Duration::from_secs(120), move || {
            let mut d = Design::new("probe");
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            d.add_object(t, satin(len, params));
            let plan = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default());
            let ink = emb_formats::inkstitch::write::write_inkstitch(&d, &emb_stitch::object::underlay_layers);
            (plan.stitches.stitch_count(), bad_stitch(&plan.stitches), ink.svg.len())
        });
        match r {
            Ok(((n, bad, svg), took)) => {
                println!(
                    "  {name}: {n} stitches, {bad:?}, Ink/Stitch file {} kB, {took:.2?}",
                    svg / 1000
                )
            }
            Err(e) => println!("  PROBLEM {e}"),
        }
    }
}

/// The check and the fix of machine files: large, empty and damaged files.
#[test]
#[ignore]
fn p22_machine_files() {
    let stitch = |x: f64, y: f64| StitchRecord::stitch(x, y);
    let at = |x: f64, y: f64, kind| StitchRecord::new(Point::new(x, y), kind);
    let mut cases: Vec<(&str, StitchDesign)> = vec![
        ("empty", StitchDesign::default()),
        ("only an end", records(1, |_| at(0.0, 0.0, StitchKind::End))),
        (
            "only jumps",
            records(10_000, |i| at((i % 100) as f64, (i / 100) as f64, StitchKind::Jump)),
        ),
        (
            "only color changes",
            records(10_000, |_| at(0.0, 0.0, StitchKind::ColorChange)),
        ),
        ("400,000 stitches at one point", records(400_000, |_| stitch(5.0, 5.0))),
        (
            "400,000 stitches in a square of 100 mm",
            records(400_000, |i| {
                let k = (i as u64).wrapping_mul(2_654_435_761) % 10_000;
                stitch((k % 100) as f64, (k / 100) as f64)
            }),
        ),
        (
            "100,000 color changes between stitches",
            records(300_000, |i| match i % 3 {
                2 => at(0.0, 0.0, StitchKind::ColorChange),
                k => stitch((i % 50) as f64 + k as f64, (i / 1000) as f64),
            }),
        ),
        (
            "200,000 trims between stitches",
            records(400_000, |i| {
                if i % 2 == 1 {
                    at(0.0, 0.0, StitchKind::Trim)
                } else {
                    stitch((i % 40) as f64, 0.0)
                }
            }),
        ),
        (
            "stitches of 1 km",
            records(1_000, |i| {
                stitch(if i % 2 == 0 { -500_000.0 } else { 500_000.0 }, i as f64)
            }),
        ),
    ];
    // Travel in two far corners of a design, as a machine file.
    let mut d = Design::new("probe");
    let t = d.add_thread("Red", Rgb(200, 0, 0));
    cluster(&mut d, t, Point::new(-900_000.0, -900_000.0));
    cluster(&mut d, t, Point::new(900_000.0, 900_000.0));
    let far = emb_stitch::build_plan(&d, &PlanOptions::default(), &mut Default::default()).stitches;
    cases.push(("travel in two corners 1.8 km apart", far));
    for (name, s) in cases {
        let r = timed(name, Duration::from_secs(10), Duration::from_secs(180), move || {
            let opts = PlanOptions::default();
            let start = Instant::now();
            let found = file_check::check(&s, &opts);
            let checked = start.elapsed();
            let start = Instant::now();
            let (fixed, did) = file_check::fix(&s, &opts, Fixes::ALL);
            let fix_time = start.elapsed();
            let again = file_check::fix(&fixed, &opts, Fixes::ALL).1.total();
            let after = file_check::check(&fixed, &opts);
            let objects = emb_model::objects_from_stitches(&fixed).len();
            (
                found.found.len(),
                after.found.len(),
                did.total(),
                again,
                bad_stitch(&fixed),
                fixed.stitch_count() as i64 - s.stitch_count() as i64,
                objects,
                checked,
                fix_time,
            )
        });
        match r {
            Ok(((before, after, did, again, bad, more, objects, checked, fix_time), _)) => println!(
                "  {name}: {before} faults, {did} fixes, {after} faults after, a second fix {again}, stitches {more:+}, {objects} objects, {bad:?}; check {checked:.2?}, fix {fix_time:.2?}"
            ),
            Err(e) => println!("  PROBLEM {e}"),
        }
    }
}
