//! P8 exit test, part 1: a library of 1000 designs is searchable in less
//! than a second.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use emb_geom::Point;
use emb_library::{GroupKey, Index, Query, SortKey, group, scan, sort};
use emb_model::{HoopCatalog, Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

const WORDS: [&str; 10] = [
    "rose", "eagle", "anchor", "monogram", "heart", "star", "dragon", "leaf", "crest", "wave",
];
const FOLDERS: [&str; 5] = ["caps", "jackets", "towels/bath", "towels/kitchen", "patches"];

/// Design `k`: a spiral of stitches, 20 to 220 mm, with 1 to 6 colors.
fn design(k: usize) -> StitchDesign {
    let colors = 1 + k % 6;
    let size = 20.0 + (k * 37 % 200) as f64;
    let mut s = StitchDesign::default();
    for c in 0..colors {
        s.threads.push(Thread {
            id: ThreadId(c as u32),
            name: format!("{} {c}", ["Navy", "Gold", "Red", "White", "Green", "Black"][c]),
            color: Rgb(40 * c as u8, 120, 200 - 30 * c as u8),
            brand: None,
            code: None,
        });
        if c > 0 {
            s.records.push(StitchRecord::new(Point::ZERO, StitchKind::ColorChange));
        }
        for i in 0..60 {
            let a = i as f64 * 0.4;
            let r = size / 2.0 * (i as f64 / 60.0);
            s.records.push(StitchRecord::new(
                Point::new(r * a.cos(), r * a.sin()),
                StitchKind::Stitch,
            ));
        }
        s.records
            .push(StitchRecord::new(Point::new(size / 2.0, 0.0), StitchKind::Stitch));
        s.records
            .push(StitchRecord::new(Point::new(-size / 2.0, 0.0), StitchKind::Stitch));
    }
    s.records.push(StitchRecord::new(Point::ZERO, StitchKind::End));
    s
}

#[test]
fn a_1000_design_library_searches_in_under_a_second() {
    let root = std::env::temp_dir().join(format!("emb-library-1000-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for k in 0..1000 {
        let dir = root.join(FOLDERS[k % FOLDERS.len()]);
        std::fs::create_dir_all(&dir).unwrap();
        let ext = ["dst", "vp3", "pes", "exp"][k % 4];
        let name = format!("{} {} {k:04}", WORDS[k % 10], WORDS[(k / 10) % 10]);
        let bytes = emb_formats::write_machine(ext, &design(k), &name, "20260101000000").unwrap();
        std::fs::write(dir.join(format!("{name}.{ext}")), bytes).unwrap();
    }
    let roots: Vec<PathBuf> = vec![root.clone()];
    let thumbs = root.join(".thumbs");
    let t = Instant::now();
    let (index, report) = scan(&roots, &Index::default(), Some(&thumbs), |_, _| true);
    let first_scan = t.elapsed();
    assert_eq!(index.entries.len(), 1000);
    assert_eq!(report.failed, 0);

    // The index is saved and read back, as the app does at each start.
    let file = root.join(".index.json");
    index.save(&file).unwrap();
    let t = Instant::now();
    let index = Index::load(&file);
    let (index, report) = scan(&roots, &index, Some(&thumbs), |_, _| true);
    let second_scan = t.elapsed();
    assert_eq!((report.read, report.kept), (0, 1000));

    let hoops = HoopCatalog::builtin();
    let topaz: Vec<_> = ["topaz20_crown", "topaz20_splendid_square"]
        .iter()
        .map(|id| hoops.get(id).unwrap().clone())
        .collect();
    let searches = [
        ("rose", 190),
        ("rose eagle", 20),
        ("towels", 400),
        ("format:vp3 colors:<3", 0),
        ("navy fits:topaz20_crown", 0),
        ("stitches:>200 width:50-150", 0),
        ("folder:kitchen gold", 0),
        ("\"dragon crest\"", 10),
        ("nothing-like-this", 0),
    ];
    for (text, expect) in searches {
        let t = Instant::now();
        let q = Query::parse(text, &hoops);
        let mut found = q.run(&index);
        sort(&index, &mut found, SortKey::Size, true);
        let groups = group(&index, &found, GroupKey::Hoop, &topaz);
        let took = t.elapsed();
        println!(
            "{text:?}: {} found in {} groups, {:.1} ms",
            found.len(),
            groups.len(),
            took.as_secs_f64() * 1000.0
        );
        assert!(took < Duration::from_secs(1), "{text:?} took {took:?}");
        if expect > 0 {
            assert_eq!(found.len(), expect, "{text:?}");
        }
    }
    println!(
        "first scan {:.1} s, second scan {:.2} s",
        first_scan.as_secs_f64(),
        second_scan.as_secs_f64()
    );
    assert!(
        second_scan < Duration::from_secs(2),
        "an unchanged library scans fast: {second_scan:?}"
    );
    std::fs::remove_dir_all(&root).ok();
}
