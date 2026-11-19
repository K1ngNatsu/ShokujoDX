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
        assert_eq!(m.shots.len(), view::ZOOMS.len(), "{}", r.name);
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
