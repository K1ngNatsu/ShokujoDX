//! The check and the fix of machine files (P22 Milestone 5, ADR 0172) on
//! the files of the program: each corpus design written as DST and read
//! back.

use emb_art::file_check::{Fault, Fixes, check, fix};
use emb_cli::{Input, corpus, stitches};
use emb_model::{StitchDesign, StitchKind, design_from_stitches};
use emb_stitch::PlanOptions;

fn dst(name: &str) -> StitchDesign {
    let design = corpus::build(name).unwrap();
    let (s, _) = stitches(&Input::Design(Box::new(design)), None).unwrap();
    let bytes = emb_formats::write_machine("dst", &s, name, "20000101000000").unwrap();
    emb_formats::read_machine("dst", &bytes).unwrap()
}

/// The files of the program have no long stitch, no color change to the
/// same thread, no long jump with no trim, no piece too short for the
/// trimmer and no travel that shows. (Short stitches come from the 0.1 mm
/// grid of the file and from short stitches of the design, which Design
/// info shows; cutwork and appliqué placement lines have no lock stitches
/// by design.)
#[test]
fn the_files_of_the_program_have_no_faults_of_the_plan() {
    for name in corpus::NAMES {
        let c = check(&dst(name), &PlanOptions::default());
        for f in [
            Fault::Long,
            Fault::SameThread,
            Fault::LongJump,
            Fault::ShortPiece,
            Fault::TravelShows,
        ] {
            assert_eq!(
                c.count(f),
                0,
                "{name}: {f:?} {:?}",
                c.found.iter().find(|x| x.fault == f)
            );
        }
    }
}

/// A fixed file has none of the faults that the fixes fix, and its needle
/// points of before keep their place (but the short stitches that went).
#[test]
fn a_fixed_file_keeps_its_needle_points() {
    let s = dst("content-families");
    let opts = PlanOptions::default();
    let before = check(&s, &opts);
    assert!(before.count(Fault::Short) > 0);
    let (out, n) = fix(&s, &opts, Fixes::ALL);
    assert_eq!(n.removed, before.count(Fault::Short));
    let after = check(&out, &opts);
    assert_eq!(after.count(Fault::Short), 0);
    assert_eq!(after.count(Fault::Long), 0);
    let points = |s: &StitchDesign| -> Vec<emb_geom::Point> {
        s.records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .collect()
    };
    let removed: Vec<usize> = before
        .found
        .iter()
        .filter(|f| f.fault == Fault::Short)
        .map(|f| f.record)
        .collect();
    let kept: Vec<emb_geom::Point> = s
        .records
        .iter()
        .enumerate()
        .filter(|(i, r)| r.kind == StitchKind::Stitch && !removed.contains(i))
        .map(|(_, r)| r.pos)
        .collect();
    let now = points(&out);
    let mut k = 0;
    for p in &now {
        if k < kept.len() && *p == kept[k] {
            k += 1;
        }
    }
    assert_eq!(k, kept.len(), "a needle point moved");
}

/// The stitch objects of a design opened from a file give the records of
/// the file to the check.
#[test]
fn the_stitch_objects_of_a_design_are_checked() {
    let file = dst("lettering-names");
    let d = design_from_stitches(&file, "names");
    let s = emb_cli::file_stitches(&Input::Design(Box::new(d))).unwrap();
    let opts = PlanOptions::default();
    assert_eq!(check(&s, &opts), check(&file, &opts));
    let empty = emb_model::Design::new("empty");
    assert!(emb_cli::file_stitches(&Input::Design(Box::new(empty))).is_err());
}

#[test]
fn the_fixes_and_the_report_of_the_command_line() {
    let f = emb_cli::parse_fixes("short, locks").unwrap();
    assert!(f.short && f.locks && !f.join && !f.long && !f.jumps);
    assert!(emb_cli::parse_fixes("short,other").is_err());
    let c = check(&dst("content-families"), &PlanOptions::default());
    let text = emb_cli::check_report(&c);
    assert!(text.starts_with("short: "), "{text}");
    assert!(text.contains("  at "), "{text}");
    assert_eq!(
        emb_cli::check_report(&emb_art::file_check::FileCheck::default()),
        "no faults\n"
    );
}
