//! A probe of the EPS reader (the audit of P18): each operator with wrong
//! operands, and programs that ask for much time or memory, run with the
//! limits of the program and a time limit. An EPS file comes from a
//! customer of a shop, so the reader must stop with no crash and no hang.
//! `cargo test -p emb-formats -- --ignored probe_p18_eps_operators
//! --nocapture` (a debug build finds the overflows of whole numbers); the
//! heavy programs run one for each process (`EPS_PROBE_CASE=<n>`, in
//! release), as a case that is not stopped takes all the memory.

use std::time::{Duration, Instant};

use super::ai::AI;
use super::graphics::GRAPHICS;
use super::import_with;
use super::ops::CORE;
use super::vm::Limits;

/// Runs `job` in a thread. A problem text if it panics, takes more than
/// `slow`, or does not end in `limit`.
fn timed<T: Send + 'static>(
    name: &str,
    slow: Duration,
    limit: Duration,
    job: impl FnOnce() -> T + Send + 'static,
) -> Result<(T, Duration), String> {
    let start = Instant::now();
    let job = std::thread::Builder::new().stack_size(8 << 20).spawn(job).unwrap();
    while !job.is_finished() && start.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(1));
    }
    if !job.is_finished() {
        return Err(format!("{name}: over {limit:?}"));
    }
    let took = start.elapsed();
    match job.join() {
        Ok(_) if took > slow => Err(format!("{name}: {took:?}")),
        Ok(v) => Ok((v, took)),
        Err(_) => Err(format!("{name}: panic")),
    }
}

fn eps(program: &str) -> Vec<u8> {
    format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n{program}\n").into_bytes()
}

fn ai(program: &str) -> Vec<u8> {
    format!("%!PS-Adobe-3.0\n%%BoundingBox: 0 0 100 100\n%AI5_FileFormat 3.0\n%%EndSetup\n{program}\n%%PageTrailer\n")
        .into_bytes()
}

/// Wrong operands for each operator.
const OPERANDS: &[&str] = &[
    "",
    "0",
    "-1",
    "2147483647",
    "-2147483648",
    "-2147483648 -1",
    "2147483647 2147483647",
    "1e308",
    "-1e308 1e308",
    "1e-308",
    "1e308 1e308 1e308 1e308 1e308 1e308",
    "0 0 0 0 0 0",
    "-1 -1 -1 -1 -1 -1",
    "()",
    "(abc) 2147483647",
    "(abc) -1",
    "(abc) 1 2147483647",
    "(abc) 2 -5",
    "[ ]",
    "[1 2 3] -1 5",
    "[1 2 3] 2 2147483647",
    "[1 2 3] 1 (x)",
    "<< >>",
    "<< /a 1 >> /a",
    "{ }",
    "{ 1 }",
    "mark",
    "null",
    "true",
    "/n",
    "/n 1",
    "matrix",
    "[1e308 0 0 1e308 0 0]",
    "[0 0 0 0 0 0]",
    "0 0 1e300 0 360",
    "0 0 moveto 1 1 lineto",
    "0 0 moveto 1e300 1e300 lineto",
    "currentfile",
    "save",
    "100000000",
    "100000000 100000000 8 [1 0 0 1 0 0] { }",
    "1 1 1 [1 0 0 1 0 0] (x)",
    "[ /Indexed /DeviceRGB 2147483647 (abc) ]",
    "[ /Separation /x /DeviceRGB { } ]",
    "[ /Pattern ]",
];

/// `true` for a point within the limit of a project file.
fn project_limit_ok(p: emb_geom::Point) -> bool {
    let m = emb_model::project::MAX_COORD_MM;
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= m && p.y.abs() <= m
}

/// Shapes outside the limit of a project, or not finite.
fn far_shapes(r: &crate::svg::SvgImport) -> usize {
    r.objects
        .iter()
        .filter(|o| {
            !o.kind
                .control_bbox()
                .is_none_or(|b| project_limit_ok(b.min) && project_limit_ok(b.max))
        })
        .count()
}

#[test]
#[ignore]
fn probe_p18_eps_operators() {
    std::panic::set_hook(Box::new(|_| {}));
    let limits = Limits {
        steps: 2_000_000,
        memory: 64 << 20,
    };
    let mut problems = Vec::new();
    let mut cases = 0;
    let mut slowest = (Duration::ZERO, String::new());
    let names: Vec<(&str, bool)> = CORE
        .iter()
        .chain(GRAPHICS)
        .map(|(n, _)| (*n, false))
        .chain(AI.iter().map(|(n, _)| (*n, true)))
        .collect();
    for (op, in_ai) in names {
        for operands in OPERANDS {
            // Each operator twice, so a second call sees what the first left.
            let program = format!("0 0 10 10 rectfill {operands} {op} {operands} {op} 0 0 10 10 rectfill");
            let data = if in_ai { ai(&program) } else { eps(&program) };
            let name = format!("{op} with `{operands}`");
            cases += 1;
            let job = move || import_with(&data, false, limits).map(|r| far_shapes(&r));
            match timed(&name, Duration::from_secs(2), Duration::from_secs(20), job) {
                Ok((Ok(far), took)) => {
                    if far > 0 {
                        problems.push(format!("{name}: {far} shape(s) out of range"));
                    }
                    if took > slowest.0 {
                        slowest = (took, name);
                    }
                }
                Ok((Err(_), _)) => {}
                Err(e) => problems.push(e),
            }
        }
    }
    println!("{cases} cases, slowest {:?} ({})", slowest.0, slowest.1);
    for p in &problems {
        println!("PROBLEM {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
}

/// Programs that ask for much time or memory, with the limits of the
/// program.
const HEAVY: &[(&str, &str)] = &[
    ("loop with no end", "{ } loop"),
    ("strings with no end", "{ 1000 string pop } loop"),
    ("a large array", "/a 1000000000 array def"),
    ("a large string", "2147483647 string"),
    ("a path with no end", "0 0 moveto { 1 1 rlineto } loop"),
    ("subpaths with no end", "{ 0 0 moveto 1 1 lineto } loop"),
    ("fills with no end", "{ 0 0 1 1 rectfill } loop"),
    ("circles with no end", "{ 0 0 1 0 360 arc fill } loop"),
    ("strokes with no end", "{ 0 0 moveto 1 1 lineto stroke } loop"),
    (
        "copies of a long path",
        "0 0 moveto 1 1 1000000 { pop 1 1 rlineto } for { gsave } loop",
    ),
    (
        "fills of a long path",
        "0 0 moveto 1 1 1000000 { pop 1 0 rlineto 0 1 rlineto } for { gsave fill grestore } loop",
    ),
    ("recursion", "/r { r } def r"),
    ("operands with no end", "{ 1 } loop"),
    ("marks with no end", "{ mark } loop"),
    ("dictionaries with no end", "{ 1 dict begin } loop"),
    ("saves with no end", "{ save } loop"),
    ("gsaves with no end", "{ gsave } loop"),
    ("a for loop with a step of 0", "0 0 1 { pop } for"),
    ("a for loop with a tiny step", "0 1e-300 1 { pop } for"),
    (
        "a procedure in itself, bound",
        "/p { 1 2 } def /p load 0 /p load put /p load bind",
    ),
    (
        "a form that paints itself",
        "/f << /FormType 1 /BBox [0 0 10 10] /Matrix [1 0 0 1 0 0] /PaintProc { pop f execform } >> def f execform",
    ),
    (
        "a pattern that paints itself",
        "/p << /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 /PaintProc { pop p setpattern 0 0 10 10 rectfill } >> matrix makepattern def /Pattern setcolorspace p setcolor 0 0 10 10 rectfill",
    ),
    ("far coordinates", "1e30 1e30 moveto 1e30 1 lineto 1 1e30 lineto fill"),
    (
        "a large scale",
        "1e300 1e300 scale 0 0 moveto 10 0 lineto 10 10 lineto fill",
    ),
    (
        "a large scale, stroked",
        "1e300 1e300 scale 0 0 moveto 10 0 lineto 10 10 lineto stroke",
    ),
    (
        "a scale of 0",
        "0 0 scale 0 0 moveto 10 0 lineto 10 10 lineto fill 1 1 itransform",
    ),
    ("a circle 2 km wide", "0 0 3e6 0 360 arc fill"),
    ("a very wide line", "1e30 setlinewidth 0 0 moveto 10 10 lineto stroke"),
    ("a dash of 0", "[0] 0 setdash 0 0 moveto 100 100 lineto stroke"),
    (
        "a tiny flatness",
        "0.000001 setflat 0 0 1000 0 360 arc flattenpath fill",
    ),
    (
        "a deep nest, executed",
        "/s 200000 string def 0 1 99999 { s exch 123 put } for 100000 1 199999 { s exch 125 put } for s cvx exec",
    ),
    (
        "a large image",
        "100000 100000 8 [1 0 0 1 0 0] currentfile /ASCIIHexDecode filter image",
    ),
    (
        "a large picture mask",
        "100000 100000 true [1 0 0 1 0 0] { (x) } imagemask",
    ),
    ("many rectangles in one array", "[ 0 1 2000000 { pop 0 } for ] rectfill"),
    ("a long string read", "currentfile 100000000 string readstring"),
    ("a radix number", "36#ZZZZZZZZZZZZZZZZZZZZ 16#FFFFFFFFFFFFFFFF"),
    (
        "tokens with no end",
        "{ (1 2 3) { token not { exit } if pop } loop } loop",
    ),
];

#[test]
#[ignore]
fn probe_p18_eps_heavy_programs() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut problems = Vec::new();
    let mut cases: Vec<(String, Vec<u8>)> = HEAVY
        .iter()
        .map(|(name, p)| {
            (
                name.to_string(),
                eps(&format!("0 0 10 10 rectfill {p} 0 0 10 10 rectfill")),
            )
        })
        .collect();
    // The same programs in an AI file.
    for (name, p) in HEAVY.iter().take(10) {
        cases.push((format!("{name} (AI)"), ai(&format!("0 0 m 10 0 L 10 10 L f {p}"))));
    }
    // A Flate bomb: 512 MB of zeros, read through a filter, a reusable
    // stream and as picture data.
    let zeros = {
        use std::io::Write;
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        let block = vec![0u8; 1 << 20];
        for _ in 0..512 {
            z.write_all(&block).unwrap();
        }
        z.finish().unwrap()
    };
    let bomb = |program: &str| {
        let mut d = eps(program);
        d.extend_from_slice(&zeros);
        d.extend_from_slice(b"\n0 0 10 10 rectfill\n");
        d
    };
    cases.push((
        "a Flate bomb, read".into(),
        bomb("currentfile /FlateDecode filter { dup 65535 string readstring not { exit } if pop } loop"),
    ));
    cases.push((
        "a Flate bomb, reusable".into(),
        bomb("currentfile /FlateDecode filter /ReusableStreamDecode filter"),
    ));
    cases.push((
        "a Flate bomb, a picture".into(),
        bomb("30000 30000 8 [1 0 0 1 0 0] currentfile /FlateDecode filter image"),
    ));
    // One case for each process (`EPS_PROBE_CASE`), so a script can watch
    // the memory of each one; with no case, the number of cases.
    let Some(only) = std::env::var("EPS_PROBE_CASE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    else {
        println!("CASES {}", cases.len());
        return;
    };
    for (name, data) in cases.into_iter().skip(only).take(1) {
        let size = data.len();
        let job =
            move || import_with(&data, false, Limits::default()).map(|r| (r.objects.len(), far_shapes(&r), r.warnings));
        match timed(&name, Duration::from_secs(5), Duration::from_secs(120), job) {
            Ok((Ok((n, far, w)), took)) => {
                println!("{name}: {n} objects, {took:.2?}, {size} bytes; {w:?}");
                if far > 0 {
                    problems.push(format!("{name}: {far} shape(s) out of range"));
                }
                if n > 100_000 {
                    problems.push(format!("{name}: {n} objects"));
                }
            }
            Ok((Err(e), took)) => println!("{name}: refused in {took:.2?}: {e}"),
            Err(e) => {
                println!("PROBLEM {e}");
                problems.push(e)
            }
        }
    }
    for p in &problems {
        println!("PROBLEM {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
}
