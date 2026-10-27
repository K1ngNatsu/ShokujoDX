//! Machine files opened as stitch objects: each corpus design, written in
//! each format, read back, made into a design (`design_from_stitches`) and
//! stitched again, sews the same needle points.

use emb_cli::{Input, corpus, plan, stitches};
use emb_model::{StitchDesign, StitchKind, design_from_stitches};
use emb_stitch::PlanOptions;

/// Needle points in 0.1 mm (the file unit), in order.
fn needles(s: &StitchDesign) -> Vec<(i64, i64)> {
    s.records
        .iter()
        .filter(|r| r.kind == StitchKind::Stitch)
        .map(|r| ((r.pos.x * 10.0).round() as i64, (r.pos.y * 10.0).round() as i64))
        .collect()
}

fn count(s: &StitchDesign, k: StitchKind) -> usize {
    s.records.iter().filter(|r| r.kind == k).count()
}

#[test]
fn corpus_files_open_as_the_same_stitches() {
    for name in corpus::NAMES {
        let design = corpus::build(name).unwrap();
        let (s, _) = stitches(&Input::Design(Box::new(design)), None).unwrap();
        for ext in emb_formats::WRITE_EXTENSIONS {
            let bytes = emb_formats::write_machine(ext, &s, name, "20000101000000").unwrap();
            let file = emb_formats::read_machine(ext, &bytes).unwrap();
            let d = design_from_stitches(&file, name);
            let again = plan(&d, &PlanOptions::default()).stitches;
            assert_eq!(needles(&again), needles(&file), "{name}.{ext}");
            assert_eq!(
                count(&again, StitchKind::ColorChange),
                count(&file, StitchKind::ColorChange),
                "{name}.{ext}"
            );
            // JEF has no trims; the plan trims at the long jumps that the reader made trims.
            if ext != "jef" {
                assert_eq!(
                    count(&again, StitchKind::Trim),
                    count(&s, StitchKind::Trim),
                    "{name}.{ext}"
                );
            }
        }
    }
}
