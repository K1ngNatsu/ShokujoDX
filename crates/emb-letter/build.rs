//! Compresses the bundled embroidery fonts (`data/fonts/embroidery/*.embfont`,
//! JSON) into the build folder, so that the program is smaller: the text of
//! a font is about six times its compressed size (P11 Milestone 2).

use std::io::Write;

fn main() {
    let dir = std::path::Path::new("../../data/fonts/embroidery");
    println!("cargo:rerun-if-changed={}", dir.display());
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    for e in std::fs::read_dir(dir).expect("data/fonts/embroidery").flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "embfont") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", p.display());
        let text = std::fs::read(&p).expect("font file");
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&text).expect("compress");
        let name = p.file_name().expect("file name").to_string_lossy().into_owned();
        std::fs::write(out.join(format!("{name}.z")), z.finish().expect("compress")).expect("write");
    }
}
