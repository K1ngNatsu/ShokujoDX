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
const CASES: [&str; 7] = [
    "p22_travel_far_apart",
    "p22_travel_many",
    "p22_quicker",
    "p22_swatches",
    "p22_lean_and_bands",
    "p22_machine_files",
    "p22_mcp",
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

/// One message to the MCP server: the result of a tool, or the error.
fn mcp_call(s: &mut emb_cli::mcp::Session, tool: &str, args: serde_json::Value) -> serde_json::Value {
    let m = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": tool, "arguments": args}});
    let r = s.handle(&m).expect("a reply");
    if r.get("error").is_some() {
        r
    } else {
        r["result"].clone()
    }
}

/// A session with a fill, a run, a satin, a vector shape and lettering.
fn mcp_seeded() -> emb_cli::mcp::Session {
    use serde_json::json;
    let mut s = emb_cli::mcp::Session::new();
    for (stitch, d) in [
        ("fill", "M -10 -10 L 10 -10 L 10 10 L -10 10 Z"),
        ("run", "M -20 20 L 20 20"),
        ("satin", "M -20 30 L 20 30"),
        ("vector", "M 0 0 L 5 5"),
    ] {
        let r = mcp_call(&mut s, "add_shape", json!({"svg_path": d, "stitch": stitch}));
        assert_eq!(r["isError"], false, "{r}");
    }
    let r = mcp_call(&mut s, "add_lettering", json!({"text": "AB"}));
    assert_eq!(r["isError"], false, "{r}");
    s
}

/// Runs `f` on the session in a thread with a time limit; the session back,
/// or a problem text (then the session is lost).
fn mcp_timed<R: Send + 'static>(
    name: &str,
    s: emb_cli::mcp::Session,
    f: impl FnOnce(&mut emb_cli::mcp::Session) -> R + Send + 'static,
) -> Result<(emb_cli::mcp::Session, R, Duration), String> {
    timed(name, Duration::from_secs(5), Duration::from_secs(30), move || {
        let mut s = s;
        let r = f(&mut s);
        (s, r)
    })
    .map(|((s, r), took)| (s, r, took))
}

/// The leaves of a value of settings: the path of each number, flag and
/// text.
fn setting_leaves(v: &serde_json::Value, at: Vec<String>, out: &mut Vec<(Vec<String>, serde_json::Value)>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, x) in m {
                let mut p = at.clone();
                p.push(k.clone());
                setting_leaves(x, p, out);
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Null => {}
        _ => out.push((at, v.clone())),
    }
}

/// The MCP server (Milestone 9) with wrong arguments: each tool with each
/// argument of a wrong type and with hard numbers and texts; each setting
/// of each object kind with hard values (the design checked after each
/// change); and lines that are not messages.
#[test]
#[ignore]
fn p22_mcp() {
    use serde_json::{Value, json};
    let dir = std::env::temp_dir().join("shokujodx-p22-mcp");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The texts of the probe are also paths: relative ones go in the
    // folder of the probe (this case runs in a process of its own).
    std::env::set_current_dir(&dir).unwrap();
    let file = |n: &str| dir.join(n).to_string_lossy().to_string();
    // A project file and a machine file to open.
    let mut s = mcp_seeded();
    let r = mcp_call(&mut s, "save_design", json!({"path": file("seed.embproj")}));
    assert_eq!(r["isError"], false, "{r}");
    let dst = records(200, |i| StitchRecord::stitch((i % 20) as f64, (i / 20) as f64));
    emb_cli::write_output(&dst, &dir.join("seed.dst")).unwrap();

    let list = s
        .handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .unwrap();
    let tools = list["result"]["tools"].as_array().unwrap().clone();
    let big = "x".repeat(1_000_000);
    let numbers = [
        json!(0),
        json!(-1),
        json!(1e-300),
        json!(1e300),
        json!(-1e300),
        json!(1e9),
        json!(-1e9),
        json!(0.5),
        json!("1"),
        json!(true),
    ];
    let texts = [
        json!(""),
        json!(" "),
        json!(big),
        json!("\u{0}"),
        json!("../../../x"),
        json!("ア"),
        json!("-1"),
        json!("1e999"),
        json!("NaN"),
        json!("#zzzzzz"),
        json!("\n\n"),
        json!(5),
    ];
    let many: Vec<u64> = (1..=20_000).collect();
    let ids = [
        json!([]),
        json!([0]),
        json!([u64::MAX]),
        json!([1, 1, 1]),
        json!(many),
        json!([-1]),
        json!([1.5]),
        json!(["1"]),
        json!(5),
    ];
    let flags = [json!("true"), json!(1)];
    let text_lists = [json!([]), json!([""]), json!([big]), json!([1]), json!("x")];
    let objects = [json!({}), json!({"a": 1}), json!([1]), json!("x")];
    let default_for = |tool: &str, name: &str| -> Value {
        match (tool, name) {
            ("open_design", "path") => json!(file("seed.embproj")),
            ("check_machine_file" | "fix_machine_file", "path") => json!(file("seed.dst")),
            (_, "path") => json!(file("out.embproj")),
            (_, "out") => json!(file("out.dst")),
            (_, "folder") => json!(file("")),
            (_, "svg_path") => json!("M 0 0 L 10 0 L 10 10 Z"),
            (_, "stitch") => json!("fill"),
            (_, "text") => json!("A"),
            (_, "ids") => json!([1]),
            (_, "settings") => json!({"zz": 1}),
            (_, "color") => json!("#ff0000"),
            (_, "degrees") => json!(10),
            (_, "source") => json!("1"),
            (_, "setting") => json!("spacing"),
            (_, "from") => json!(0.3),
            (_, "to") => json!(0.5),
            (_, "what") => json!("fonts"),
            _ => json!(1),
        }
    };
    let mut calls = 0;
    let mut problems = 0;
    let mut slowest = (Duration::ZERO, String::new());
    let mut session = Some(mcp_seeded());
    for t in &tools {
        let tool = t["name"].as_str().unwrap().to_string();
        if tool == "show" {
            continue;
        }
        let schema = &t["inputSchema"];
        let required: Vec<String> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        for (param, p) in schema["properties"].as_object().unwrap() {
            let values: &[Value] = match p["type"].as_str().unwrap() {
                "number" => &numbers,
                "string" => &texts,
                "boolean" => &flags,
                "array" if p["items"]["type"] == "integer" => &ids,
                "array" => &text_lists,
                _ => &objects,
            };
            for v in values {
                let mut args = serde_json::Map::new();
                for r in &required {
                    args.insert(r.clone(), default_for(&tool, r));
                }
                args.insert(param.clone(), v.clone());
                let shown = v.to_string();
                let shown = if shown.len() > 40 {
                    format!("{}…", &shown[..40])
                } else {
                    shown
                };
                let label = format!("{tool} {param}={shown}");
                let s = session.take().unwrap_or_else(mcp_seeded);
                let tool2 = tool.clone();
                calls += 1;
                match mcp_timed(&label, s, move |s| {
                    let r = mcp_call(s, &tool2, Value::Object(args));
                    let check = mcp_call(s, "check_design", json!({}));
                    (r, check)
                }) {
                    Ok((s, (r, check), took)) => {
                        if took > slowest.0 {
                            slowest = (took, label.clone());
                        }
                        if check["isError"] != false {
                            problems += 1;
                            println!("  PROBLEM {label}: check_design after it: {check}");
                        }
                        if r.get("error").is_some() {
                            println!("  {label}: a protocol error {}", r["error"]);
                        }
                        session = Some(s);
                    }
                    Err(e) => {
                        problems += 1;
                        println!("  PROBLEM {e}");
                    }
                }
            }
        }
    }
    println!(
        "  tools: {calls} calls, {problems} problems, the slowest {:.2?} ({})",
        slowest.0, slowest.1
    );

    // Each setting of each kind with hard values.
    let hard = [
        json!(0),
        json!(-1),
        json!(1e-300),
        json!(1e300),
        json!(-1e300),
        json!(1e9),
        json!(0.001),
        json!("zz"),
        json!(true),
        json!([]),
        json!({}),
    ];
    let mut s = mcp_seeded();
    let listed = mcp_call(&mut s, "list_objects", json!({}));
    let objs = listed["structuredContent"]["objects"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let (mut tried, mut taken, mut problems, mut slowest) = (0, 0, 0, (Duration::ZERO, String::new()));
    let mut session = Some(s);
    for o in &objs {
        let id = o["id"].as_u64().unwrap();
        let kind = o["kind"].as_str().unwrap_or("?").to_string();
        let mut leaves = Vec::new();
        setting_leaves(&o["settings"], Vec::new(), &mut leaves);
        for (path, _) in &leaves {
            for v in &hard {
                let mut patch = v.clone();
                for k in path.iter().rev() {
                    patch = json!({ k.clone(): patch });
                }
                let label = format!("{kind} {}={v}", path.join("."));
                let s = session.take().unwrap_or_else(mcp_seeded);
                tried += 1;
                match mcp_timed(&label, s, move |s| {
                    let r = mcp_call(s, "set_settings", json!({"ids": [id], "settings": patch}));
                    let check = mcp_call(s, "check_design", json!({}));
                    let picture = mcp_call(s, "picture", json!({"px_per_mm": 2}));
                    if r["isError"] == false {
                        mcp_call(s, "undo", json!({}));
                    }
                    (r, check, picture)
                }) {
                    Ok((s, (r, check, picture), took)) => {
                        if took > slowest.0 {
                            slowest = (took, label.clone());
                        }
                        if r["isError"] == false {
                            taken += 1;
                        }
                        if check["isError"] != false || picture["isError"] != false {
                            problems += 1;
                            println!("  PROBLEM {label}: {check} {}", picture["content"][0]);
                        }
                        session = Some(s);
                    }
                    Err(e) => {
                        problems += 1;
                        println!("  PROBLEM {e}");
                    }
                }
            }
        }
    }
    println!(
        "  settings: {} objects, {tried} changes, {taken} taken, {problems} problems, the slowest {:.2?} ({})",
        objs.len(),
        slowest.0,
        slowest.1
    );

    // Lines that are not messages, through `serve`.
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    let lines = [
        "{".to_string(),
        "null".into(),
        "5".into(),
        "[]".into(),
        r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#.into(),
        deep,
        format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{}"}}"#, "x".repeat(5_000_000)),
        r#"{"jsonrpc":"2.0","id":{"a":[1,2]},"method":"ping"}"#.into(),
        r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#.into(),
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"delete_objects","arguments":{"ids":[1]}}}"#.into(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":5}"#.into(),
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":5}}"#.into(),
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"undo","arguments":[]}}"#.into(),
        r#"{"jsonrpc":"2.0","id":5,"result":{}}"#.into(),
        "\u{feff}{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"ping\"}".into(),
    ];
    let mut input = Vec::new();
    for l in &lines {
        input.extend_from_slice(l.as_bytes());
        input.push(b'\n');
    }
    input.extend_from_slice(&[0xff, 0xfe, b'\n']);
    input.extend_from_slice(br#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#);
    input.push(b'\n');
    let n = lines.len() + 2;
    match timed("serve", Duration::from_secs(5), Duration::from_secs(30), move || {
        let mut out = Vec::new();
        let r = emb_cli::mcp::serve(std::io::Cursor::new(input), &mut out);
        (r.map_err(|e| e.to_string()), String::from_utf8_lossy(&out).to_string())
    }) {
        Ok(((r, out), took)) => {
            let replies: Vec<&str> = out.lines().collect();
            let not_json = replies
                .iter()
                .filter(|l| serde_json::from_str::<Value>(l).is_err())
                .count();
            println!(
                "  serve: {n} lines, {} replies ({not_json} not JSON), the end {r:?}, {took:.2?}",
                replies.len()
            );
            for l in replies {
                let end = l.char_indices().nth(160).map_or(l.len(), |(i, _)| i);
                println!("    {}", &l[..end]);
            }
        }
        Err(e) => println!("  PROBLEM {e}"),
    }
    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
