//! Vector artwork import: one entry for each file type (SVG, PDF, AI, EPS
//! and PostScript, EMF, WMF). Each reader gives the same result as the SVG
//! import: fills and running lines, or vector objects (ADR 0024, 0027,
//! 0138). A CDR file gives a message: the format has no public description.

use crate::svg::{SvgError, SvgImport};
use emb_geom::text;

/// The file extensions of vector artwork that the app can import.
pub const ARTWORK_EXTENSIONS: [&str; 8] = ["svg", "pdf", "ai", "eps", "ps", "emf", "wmf", "cdr"];

/// Imports artwork of type `ext` (a file extension, any case): as vector
/// objects (`vectors`), else as fills and running lines.
pub fn import_artwork(ext: &str, data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    match ext.to_ascii_lowercase().as_str() {
        "svg" if vectors => crate::svg::import_svg_vectors(data),
        "svg" => crate::svg::import_svg(data),
        // An AI file is PDF inside, or PostScript (the old kind).
        "pdf" | "ai" | "eps" | "epsf" | "epsi" | "ps" if crate::eps::is_postscript(data) => {
            crate::eps::import_eps(data, vectors)
        }
        "pdf" | "ai" => crate::pdf::import_pdf(data, vectors),
        "eps" | "epsf" | "epsi" | "ps" if data.starts_with(b"%PDF") => crate::pdf::import_pdf(data, vectors),
        "eps" | "epsf" | "epsi" | "ps" => crate::eps::import_eps(data, vectors),
        "cdr" => Err(SvgError::Parse(
            text::tr(
                "CDR files have no public description, so the program cannot read them. In the program that made the file, save it as SVG, PDF or EPS.",
            )
            .into(),
        )),
        "emf" => crate::emf::import_emf(data, vectors),
        "wmf" => crate::wmf::import_wmf(data, vectors),
        other => Err(SvgError::Parse(text::fill("cannot import .{} artwork", &[&other]))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_goes_to_its_reader() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><rect width="5" height="5" fill="#f00"/></svg>"##;
        assert_eq!(import_artwork("SVG", svg, false).unwrap().objects.len(), 1);
        assert!(matches!(
            import_artwork("svg", svg, true).unwrap().objects[0].kind,
            emb_model::ObjectKind::Vector { .. }
        ));
        let pdf = crate::pdf::tests::pdf("0 0 10 10 re f");
        assert_eq!(import_artwork("ai", &pdf, false).unwrap().objects.len(), 1);
        assert!(import_artwork("xyz", b"%!PS", false).is_err());
        // PostScript in an AI or EPS file, and PDF in an EPS file.
        let ps = b"%!PS-Adobe-3.0 EPSF-3.0
%%BoundingBox: 0 0 10 10
0 0 10 10 rectfill";
        assert_eq!(import_artwork("ai", ps, false).unwrap().objects.len(), 1);
        assert_eq!(import_artwork("EPS", ps, false).unwrap().objects.len(), 1);
        assert_eq!(import_artwork("eps", &pdf, false).unwrap().objects.len(), 1);
        let cdr = import_artwork("cdr", b"RIFF", false).unwrap_err().to_string();
        assert!(cdr.contains("SVG, PDF or EPS"), "{cdr}");
    }
}
