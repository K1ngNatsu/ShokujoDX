//! The engine crates use `emb_geom::math` for transcendental functions, not
//! the math library of the platform (ADR 0003). This test reads their source
//! files (not the tests) and fails on a call such as `x.sin()`.
//!
//! The list has all engine crates of rule 9 of the spec (the audit of
//! 2026-11-20 found that the sketch and the script crates were not in it).

use std::path::Path;

const ENGINE: [&str; 10] = [
    "emb-geom",
    "emb-model",
    "emb-stitch",
    "emb-formats",
    "emb-render",
    "emb-letter",
    "emb-art",
    "emb-library",
    "emb-script",
    "emb-sketch",
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

/// The name of the module of `line` if it is `mod name;` or `mod name {`
/// (with or without `pub`), and `true` if it has its body in the file.
fn module_of(line: &str) -> Option<(&str, bool)> {
    let rest = line.trim();
    let rest = match rest.strip_prefix("pub") {
        Some(r) => r.trim_start_matches(|c: char| c != ' ').trim_start(),
        None => rest,
    };
    let rest = rest.strip_prefix("mod ")?;
    let name = rest.trim_end_matches(['{', ';', ' ']);
    Some((name, rest.ends_with('{')))
}

/// The code of a file that is not tests, as numbered lines, and the names
/// of the modules of other files that are only for tests
/// (`#[cfg(test)] mod name;`). The test module of a file is at its end.
fn code_lines(text: &str) -> (Vec<(usize, &str)>, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let (mut code, mut test_modules) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "#[cfg(test)]"
            && let Some((name, body)) = lines.get(i + 1).and_then(|l| module_of(l))
        {
            if body {
                break;
            }
            test_modules.push(name.to_string());
            i += 2;
            continue;
        }
        code.push((i + 1, lines[i]));
        i += 1;
    }
    (code, test_modules)
}

#[test]
fn engine_crates_use_the_deterministic_math_functions() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for c in ENGINE {
        let mut files = Vec::new();
        rust_files(&crates.join(c).join("src"), &mut files);
        assert!(!files.is_empty(), "{c} has no source files");
        let texts: Vec<(std::path::PathBuf, String)> = files
            .into_iter()
            .map(|f| {
                let text = std::fs::read_to_string(&f).unwrap().replace("\r\n", "\n");
                (f, text)
            })
            .collect();
        let test_modules: Vec<String> = texts.iter().flat_map(|(_, t)| code_lines(t).1).collect();
        for (f, text) in &texts {
            // A file, or a folder, of a module that is only for tests.
            let stem = f.file_stem().unwrap_or_default().to_string_lossy();
            let folder = f
                .parent()
                .and_then(|p| p.file_name())
                .unwrap_or_default()
                .to_string_lossy();
            if test_modules.iter().any(|m| *m == stem || *m == folder) {
                continue;
            }
            for (n, line) in code_lines(text).0 {
                let line = line.split("//").next().unwrap_or_default();
                if CALLS.iter().any(|c| line.contains(c)) {
                    found.push(format!("{}:{}: {}", f.display(), n, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "use emb_geom::math:\n{}", found.join("\n"));
}

#[test]
fn the_code_after_a_test_module_of_another_file_is_checked() {
    let text = "fn a() {}\n#[cfg(test)]\nmod probe;\nfn b() { x.sin() }\n#[cfg(test)]\nmod tests {\n    fn c() { x.cos() }\n}\n";
    let (code, test_modules) = code_lines(text);
    assert_eq!(test_modules, ["probe"]);
    let kept: Vec<&str> = code.iter().map(|(_, l)| *l).collect();
    assert_eq!(kept, ["fn a() {}", "fn b() { x.sin() }"]);
    assert_eq!(code[1].0, 4, "the line number of the file");
    assert_eq!(module_of("pub(crate) mod tests_edit;"), Some(("tests_edit", false)));
}
