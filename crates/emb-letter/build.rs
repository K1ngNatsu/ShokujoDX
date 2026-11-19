//! Compresses the bundled embroidery fonts (`data/fonts/embroidery/*.embfont`,
//! JSON) into the build folder, so that the program is smaller: the text of
//! a font is about six times its compressed size (P11 Milestone 2).
//!
//! It also writes the table of the quality scores of the fonts
//! (`embroidery_quality.rs`, P17 Milestone 7, ADR 0115), so that the
//! program shows and sorts the scores with no font read.

use std::io::Write;

/// The quality in a font file (see `emb_letter::quality::Quality`).
#[derive(serde::Deserialize)]
struct Quality {
    score: u8,
    coverage: f64,
    narrow: f64,
    long: f64,
    #[serde(default)]
    fills: f64,
}

/// The fields of a font file that the table needs.
#[derive(serde::Deserialize)]
struct Head {
    family: String,
    quality: Option<Quality>,
}

fn main() {
    let dir = std::path::Path::new("../../data/fonts/embroidery");
    println!("cargo:rerun-if-changed={}", dir.display());
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .expect("data/fonts/embroidery")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "embfont"))
        .collect();
    files.sort();
    let mut table = String::from("&[\n");
    for p in files {
        println!("cargo:rerun-if-changed={}", p.display());
        let text = std::fs::read(&p).expect("font file");
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&text).expect("compress");
        let name = p.file_name().expect("file name").to_string_lossy().into_owned();
        std::fs::write(out.join(format!("{name}.z")), z.finish().expect("compress")).expect("write");
        let head: Head = serde_json::from_slice(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        if let Some(q) = head.quality {
            table.push_str(&format!(
                "    ({:?}, crate::quality::Quality {{ score: {}, coverage: {:?}, narrow: {:?}, long: {:?}, fills: {:?} }}),\n",
                head.family, q.score, q.coverage, q.narrow, q.long, q.fills
            ));
        }
    }
    table.push_str("]\n");
    std::fs::write(out.join("embroidery_quality.rs"), table).expect("write");
}
