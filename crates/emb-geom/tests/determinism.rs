//! The engine crates use `emb_geom::math` for transcendental functions, not
//! the math library of the platform (ADR 0003). This test reads their source
//! files (not the tests) and fails on a call such as `x.sin()`.

use std::path::Path;

const ENGINE: [&str; 8] = [
    "emb-geom",
    "emb-model",
    "emb-stitch",
    "emb-formats",
    "emb-render",
    "emb-letter",
    "emb-art",
    "emb-library",
];

/// Calls of the platform math library. `sqrt`, `powi` and `to_radians` are
/// exact or the same on all platforms.
const CALLS: [&str; 20] = [
    ".sin(",
    ".cos(",
    ".tan(",
    ".asin(",
    ".acos(",
    ".atan(",
    ".atan2(",
    ".sin_cos(",
    ".exp(",
    ".exp2(",
    ".exp_m1(",
    ".ln(",
    ".ln_1p(",
    ".log(",
    ".log2(",
    ".log10(",
    ".powf(",
    ".hypot(",
    ".cbrt(",
    ".tanh(",
];

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn engine_crates_use_the_deterministic_math_functions() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for c in ENGINE {
        let mut files = Vec::new();
        rust_files(&crates.join(c).join("src"), &mut files);
        assert!(!files.is_empty(), "{c} has no source files");
        for f in files {
            let text = std::fs::read_to_string(&f).unwrap().replace("\r\n", "\n");
            // The tests of a file are at its end.
            let code = text.split("#[cfg(test)]").next().unwrap_or_default();
            for (n, line) in code.lines().enumerate() {
                let line = line.split("//").next().unwrap_or_default();
                if CALLS.iter().any(|c| line.contains(c)) {
                    found.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "use emb_geom::math:\n{}", found.join("\n"));
}
