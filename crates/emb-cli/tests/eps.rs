//! The automated part of the exit test of P18 Milestone 8 (ADR 0138): EPS
//! files of three kinds open as objects with their colors and shapes, and
//! sew; a CDR file gives the message. The three files draw the same badge:
//!
//! - `eps-simple.eps`: short procedures, a transform with y down, arcs, a
//!   picture with its data in hex after `image`, text;
//! - `eps-procset.eps`: a printing procedure set as a resource, aliases
//!   with `load def`, a spot color (Separation) with a tint procedure,
//!   `arct`, `stopped`, a Type 1 font with an encrypted part, a picture
//!   with ASCII85 and Flate data, `cvx exec`, a `for` loop, `save` and
//!   `restore`, the bounding box at the end;
//! - `eps-ai-script.ai`: an AI script (paths with `c`, `v`, `y`, a
//!   compound path, a clip path, a text object, a picture, a named color
//!   with a tint, a hidden layer).
//!
//! The PostScript of the files is our own. The exit test of the owner uses
//! the EPS and AI files of testers.

use emb_geom::BBox;
use emb_model::{ObjectKind, Rgb};

const SIMPLE: &[u8] = include_bytes!("data/eps-simple.eps");
const PROCSET: &[u8] = include_bytes!("data/eps-procset.eps");
const AI: &[u8] = include_bytes!("data/eps-ai-script.ai");

const MM: f64 = 25.4 / 72.0;

/// A box in points of the page (y up, page height 100) in mm (y down).
fn mm_box(x0: f64, y0: f64, x1: f64, y1: f64) -> BBox {
    BBox::new(
        emb_geom::Point::new(x0 * MM, (100.0 - y1) * MM),
        emb_geom::Point::new(x1 * MM, (100.0 - y0) * MM),
    )
}

fn near(a: &BBox, b: &BBox, tol: f64) -> bool {
    (a.min.x - b.min.x).abs() < tol
        && (a.min.y - b.min.y).abs() < tol
        && (a.max.x - b.max.x).abs() < tol
        && (a.max.y - b.max.y).abs() < tol
}

fn check_badge(name: &str, ext: &str, data: &[u8]) {
    let r = emb_formats::artwork::import_artwork(ext, data, false).unwrap();
    let kinds: Vec<(&str, usize, Rgb)> = r
        .objects
        .iter()
        .map(|o| match &o.kind {
            ObjectKind::Fill { holes, .. } => ("fill", holes.len(), o.color),
            ObjectKind::Running { .. } => ("line", 0, o.color),
            _ => ("other", 0, o.color),
        })
        .collect();
    let blue = Rgb(0, 128, 255);
    let red = Rgb(255, 0, 0);
    let green = Rgb(0, 153, 0);
    let black = Rgb(0, 0, 0);
    let yellow = Rgb(255, 255, 0);
    assert_eq!(
        kinds,
        [
            ("fill", 0, blue),
            ("fill", 1, red),
            ("fill", 0, green),
            ("line", 0, black),
            ("line", 0, black),
            ("fill", 0, yellow),
        ],
        "{name}: {:?}",
        r.warnings
    );
    let boxes: Vec<BBox> = r.objects.iter().map(|o| o.kind.control_bbox().unwrap()).collect();
    let expected = [
        mm_box(10.0, 10.0, 190.0, 90.0),
        mm_box(30.0, 20.0, 90.0, 80.0),
        mm_box(110.0, 20.0, 170.0, 75.0),
        mm_box(110.0, 20.0, 170.0, 75.0),
        mm_box(20.0, 95.0, 180.0, 95.0),
        mm_box(170.0, 70.0, 180.0, 80.0),
    ];
    for (i, (got, want)) in boxes.iter().zip(&expected).enumerate() {
        // The handles of the curves of a circle are outside it; 0.1 mm.
        let tol = if i == 1 { 4.5 } else { 0.1 };
        assert!(near(got, want, tol), "{name}: object {i}: {got:?} is not {want:?}");
    }
    // The ring has its hole in the middle.
    let ObjectKind::Fill { holes, .. } = &r.objects[1].kind else {
        unreachable!()
    };
    let hole = holes[0].control_bbox().unwrap();
    assert!((hole.center().x - 60.0 * MM).abs() < 0.1 && (hole.center().y - 50.0 * MM).abs() < 0.1);
    assert!((hole.width() - 30.0 * MM).abs() < 2.5, "{name}: {hole:?}");
    // The text and the picture are not imported: warnings say so.
    let w = r.warnings.join(" | ");
    assert!(w.contains("1 text object"), "{name}: {w}");
    assert!(w.contains("1 image"), "{name}: {w}");
    assert!(!w.contains("PostScript error"), "{name}: {w}");

    // As vectors: one object for each painted path, the triangle with its
    // fill and its stroke (2 pt).
    let v = emb_formats::artwork::import_artwork(ext, data, true).unwrap();
    assert_eq!(v.objects.len(), 5, "{name}");
    let ObjectKind::Vector { style, .. } = &v.objects[2].kind else {
        panic!("{name}: not a vector")
    };
    assert_eq!(style.fill, Some(green));
    assert_eq!(style.stroke, Some(black));
    assert!((style.stroke_mm - 2.0 * MM).abs() < 1e-6, "{name}: {}", style.stroke_mm);

    // It sews.
    let d = r.to_design(name);
    let plan = emb_cli::plan(&d, &emb_stitch::PlanOptions::default());
    let sewn: std::collections::HashSet<_> = plan.spans.iter().map(|s| s.object).collect();
    assert_eq!(sewn.len(), 6, "{name}: each object sews");
    assert!(plan.stitches.records.len() > 500, "{name}");
}

#[test]
fn an_eps_file_with_simple_procedures_opens() {
    check_badge("simple", "eps", SIMPLE);
}

#[test]
fn an_eps_file_with_a_printing_procset_opens() {
    check_badge("procset", "eps", PROCSET);
}

#[test]
fn an_ai_file_with_an_ai_script_opens() {
    check_badge("ai script", "ai", AI);
    let r = emb_formats::artwork::import_artwork("ai", AI, false).unwrap();
    assert!(
        r.warnings.iter().any(|w| w.contains("1 hidden layer")),
        "{:?}",
        r.warnings
    );
}

#[test]
fn a_cdr_file_gives_the_message() {
    let e = emb_formats::artwork::import_artwork("cdr", b"RIFF\x10\0\0\0CDR9vrsn", false)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("no public description") && e.contains("SVG, PDF or EPS"),
        "{e}"
    );
}
