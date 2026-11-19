//! The pictures of the reference set of the 3D view (P18 view plan,
//! Milestone 1, ADR 0122): the hash of each picture of `emb-cli
//! view-bench`. A change of a hash is a change of the look of the view.
//!
//! The test writes each picture that changed to `target/tmp/view-bench`,
//! so you can look at it (`emb-cli view-compare` puts it next to the
//! picture before). To accept a change:
//! `INSTA_UPDATE=always cargo test -p emb-cli --test view_bench`, then say
//! why the look changed in the commit message (as for the golden
//! snapshots, CLAUDE.md rule 7).

use std::collections::BTreeMap;
use std::path::Path;

use emb_cli::view;

#[test]
fn the_pictures_of_the_reference_set() {
    let snap = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/view_bench__pictures.snap");
    // `<design>-<zoom> <width>x<height> <hash>` on each line.
    let old: BTreeMap<String, String> = std::fs::read_to_string(&snap)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            Some((w.next()?.to_string(), w.nth(1)?.to_string()))
        })
        .collect();
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("view-bench");
    let (mut lines, mut changed) = (Vec::new(), Vec::new());
    for r in view::REFERENCE {
        let m = view::run(&r, false);
        assert_eq!(m.shots.len(), 2 * view::ZOOMS.len(), "{}", r.name);
        for s in &m.shots {
            let h = format!("{:016x}", s.hash);
            lines.push(format!("{} {}x{} {h}", s.name, s.picture.width, s.picture.height));
            if old.get(&s.name) != Some(&h) {
                std::fs::create_dir_all(&out).unwrap();
                let path = out.join(format!("{}.png", s.name));
                std::fs::write(&path, s.picture.png()).unwrap();
                changed.push(path.display().to_string());
            }
        }
    }
    if !changed.is_empty() {
        eprintln!("The pictures that changed:\n{}", changed.join("\n"));
    }
    insta::assert_snapshot!("pictures", lines.join("\n"));
}

/// The exit test of Milestone 3 of the view (ADR 0124): the surface of the
/// reference set has the heights of a sew-out.
#[test]
fn the_surface_has_the_heights_of_a_sew_out() {
    use emb_geom::{BBox, Point};
    let laid = |name: &str| {
        let d = emb_cli::corpus::build(name).unwrap();
        let out = d.output_transform();
        (view::lay_design(&d), out)
    };
    let area = |out: &emb_geom::Transform, x: f64, y: f64, w: f64, h: f64| {
        BBox::from_center_size(out.apply(Point::new(x, y)), w, h)
    };
    // A border on a fill is higher than the fill.
    let (l, out) = laid("view-fill-border");
    let border = l.max_height_in(area(&out, 15.6, 15.6, 2.0, 2.0));
    let fill = l.max_height_in(area(&out, 0.0, 0.0, 16.0, 16.0));
    assert!(border > fill + 0.1, "border {border}, fill {fill}");
    // A satin with underlay is higher than one with none (the middle of
    // each column).
    let (l, out) = laid("view-underlay");
    let none = l.mean_height_in(area(&out, -40.0, 0.0, 2.0, 30.0));
    let double = l.mean_height_in(area(&out, -25.0, 0.0, 2.0, 30.0));
    assert!(double > none + 0.05, "with underlay {double}, with none {none}");
    // Puff is the highest of the set.
    let (l, _) = laid("view-puff");
    let all = BBox::new(Point::new(-200.0, -200.0), Point::new(200.0, 200.0));
    let puff = l.max_height_in(all);
    assert!(puff > emb_render::strand::FOAM_MM, "{puff}");
    for r in view::REFERENCE.iter().filter(|r| r.name != "view-puff") {
        let h = laid(r.name).0.max_height_in(all);
        assert!(h < puff - 0.5, "{}: {h} (puff {puff})", r.name);
    }
}
