//! A probe of the data that phase P17 reads from files (the audit of P17):
//! each number of an embroidery font file is set in turn to wrong values,
//! and text in the font is laid out, sewn and measured with a time limit.
//! A font file can come from a font package of another person. The probe
//! also sews text that is hard to shape, and applies letter corrections
//! with wrong values.
//! `cargo test --release -p emb-app -- --ignored probe_p17 --nocapture`.

use std::time::{Duration, Instant};

use emb_geom::Point;
use emb_model::{Design, Lettering, ObjectKind, Rgb};
use serde_json::Value;

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

/// A bundled font with three letters only, as the text of its file.
fn small_font(family: &str) -> Value {
    let mut f = emb_letter::bundled::embroidery(family).expect("bundled font");
    f.letters.retain(|c, _| "ABi ".contains(*c));
    f.kerning.retain(|pair, _| pair.chars().all(|c| "ABi ".contains(c)));
    serde_json::from_str(&f.to_json()).unwrap()
}

/// Lays out and sews `text` in `font`, measures the font and makes its
/// test sheet. Returns the number of stitches and of objects.
fn use_font(font: &emb_letter::EmbFont, text: &str) -> (usize, usize) {
    use emb_letter::LetterSource;
    let mut d = Design::new("probe");
    let t = d.add_thread("Black", Rgb(0, 0, 0));
    let mut l = Lettering::new(text, font.font_ref(), 20.0, Point::ZERO);
    emb_letter::relayout(&mut l, font);
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    let plan = emb_stitch::build_plan(&d, &emb_stitch::PlanOptions::default(), &mut Default::default());
    let mut stitches = plan.stitches.stitch_count();
    // The size range and the widths that the Fonts window and the
    // converter read.
    let mut again = font.clone();
    again.fit_size_range();
    let _ = emb_letter::quality::width_at(font, 0.1);
    let _ = emb_letter::quality::hole_width_at(font, 0.1);
    // The test sheet at the sizes of the range of the font.
    let sizes = emb_letter::sheet::sizes(font.size_range(emb_model::LetterStitch::Satin));
    let sheet = emb_letter::sheet::sheet(
        font,
        None,
        &emb_letter::sheet::Spec {
            name: "probe",
            sizes: &sizes,
            characters: &emb_letter::sheet::Characters::All,
            width_mm: 250.0,
        },
    );
    let objects = sheet.letters();
    let design = sheet.into_design("probe");
    let plan = emb_stitch::build_plan(&design, &emb_stitch::PlanOptions::default(), &mut Default::default());
    stitches += plan.stitches.stitch_count();
    (stitches, objects)
}

/// Runs `job` in a thread. A problem text if it panics, takes more than
/// `slow`, or does not end in `limit`.
fn timed<T: Send + 'static>(
    name: &str,
    slow: Duration,
    limit: Duration,
    job: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let start = Instant::now();
    let job = std::thread::spawn(job);
    while !job.is_finished() && start.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(2));
    }
    if !job.is_finished() {
        return Err(format!("{name}: over {limit:?}"));
    }
    match job.join() {
        Ok(_) if start.elapsed() > slow => Err(format!("{name}: {:?}", start.elapsed())),
        Ok(v) => Ok(v),
        Err(_) => Err(format!("{name}: panic")),
    }
}

#[test]
#[ignore]
fn probe_p17_font_files() {
    // The panics of the jobs are counted, not printed.
    std::panic::set_hook(Box::new(|_| {}));
    let (mut cases, mut refused) = (0, 0);
    let mut problems = Vec::new();
    // A routed font with satin columns and travel runs, and a fill font.
    for family in ["Anton", "Anton Fill", "Pacifico"] {
        let root = small_font(family);
        let mut paths = Vec::new();
        numbers(&root, &mut Vec::new(), &mut paths);
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
                let name = format!("{family}: {} = {value}", path.join("."));
                // To find a case that ends the process (a stack that is
                // too deep, no memory): EMB_PROBE_TRACE prints each case.
                if std::env::var_os("EMB_PROBE_TRACE").is_some() {
                    eprintln!("{name}");
                }
                match timed(&name, Duration::from_secs(3), Duration::from_secs(20), move || {
                    use_font(&font, "AiB A")
                }) {
                    Ok((stitches, _)) if stitches > 3_000_000 => problems.push(format!("{name}: {stitches} stitches")),
                    Ok(_) => {}
                    Err(e) => problems.push(e),
                }
            }
        }
    }
    println!("cases {cases}, refused {refused}, problems {}", problems.len());
    for p in problems.iter().take(60) {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}

#[test]
#[ignore]
fn probe_p17_texts_and_corrections() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut problems = Vec::new();
    // Text that is hard to shape (P17 Milestone 6): long text, directions
    // that change, marks on marks, characters that no font has.
    let texts: Vec<(&str, String)> = vec![
        ("long line", "Hamburgefonstiv ".repeat(400)),
        ("many lines", "Ab\n".repeat(800)),
        ("direction marks", "\u{202e}a\u{202d}b\u{2067}c\u{2069}".repeat(300)),
        ("marks on one letter", format!("a{}", "\u{301}".repeat(3000))),
        ("arabic with marks", "\u{644}\u{64e}\u{627}\u{651} ".repeat(400)),
        (
            "devanagari conjuncts",
            "\u{915}\u{94d}\u{937}\u{94d}\u{924}\u{94d}\u{930}\u{93f} ".repeat(300),
        ),
        ("joiners", "\u{200d}\u{200c}\u{feff}\u{200b}".repeat(2000)),
        ("no such characters", "\u{10ffff}\u{e000}\u{1f600}\u{4e00}".repeat(500)),
        (
            "mixed",
            "abc \u{5d0}\u{5d1} 123 \u{627}\u{628} \u{915}\u{916}".repeat(200),
        ),
    ];
    for (family, style) in [
        ("Lato", "Bold"),
        ("Tajawal", "Bold"),
        ("Poppins", "Bold"),
        ("Pacifico", "Regular"),
    ] {
        for (what, text) in &texts {
            let name = format!("{family}: {what}");
            let font = emb_letter::bundled::font(family, style).expect("bundled font");
            let text = text.clone();
            let job = move || {
                let mut d = Design::new("probe");
                let t = d.add_thread("Black", Rgb(0, 0, 0));
                let mut l = Lettering::new(&text, font.font_ref(), 10.0, Point::ZERO);
                emb_letter::relayout(&mut l, &font);
                let missing = l.missing_characters().len();
                d.add_object(t, ObjectKind::Lettering(Box::new(l)));
                let plan = emb_stitch::build_plan(&d, &emb_stitch::PlanOptions::default(), &mut Default::default());
                (plan.stitches.stitch_count(), missing)
            };
            if let Err(e) = timed(&name, Duration::from_secs(20), Duration::from_secs(90), job) {
                problems.push(e);
            }
        }
    }
    // Letter corrections with wrong values (P17 Milestone 4): each number
    // of a corrections file, and steps that name parts that are not there.
    let font = emb_letter::bundled::embroidery("Anton").expect("bundled font");
    let mut fixes = emb_letter::correction::Corrections::new("Anton");
    let mut shape = emb_letter::correction::LetterCorrection::from_letter(&font.letters[&'A']);
    shape.tuning.insert(
        0,
        emb_model::PartTuning {
            pull_mm: 0.1,
            angle_deg: 30.0,
            ..Default::default()
        },
    );
    fixes.letters.insert('A', shape);
    let root: Value = serde_json::from_str(&fixes.to_json()).unwrap();
    let mut paths = Vec::new();
    numbers(&root, &mut Vec::new(), &mut paths);
    let (mut cases, mut refused) = (0, 0);
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
            let Ok(fixes) = emb_letter::correction::Corrections::from_json(&v.to_string()) else {
                refused += 1;
                continue;
            };
            let name = format!("correction: {} = {value}", path.join("."));
            let mut font = font.clone();
            font.letters.retain(|c, _| "ABi ".contains(*c));
            let job = move || {
                let _ = fixes.apply(&mut font);
                use_font(&font, "AiB A")
            };
            if let Err(e) = timed(&name, Duration::from_secs(3), Duration::from_secs(20), job) {
                problems.push(e);
            }
        }
    }
    println!(
        "corrections: cases {cases}, refused {refused}; problems {}",
        problems.len()
    );
    for p in problems.iter().take(60) {
        println!("  {p}");
    }
    assert!(problems.is_empty());
}
