//! Vector artwork import: one entry for each file type (SVG, PDF and
//! PDF-based AI, EMF, WMF). Each reader gives the same result as the SVG
//! import: fills and running lines, or vector objects (ADR 0024, 0027).

use crate::svg::{SvgError, SvgImport};

/// The file extensions of vector artwork that the app can import.
pub const ARTWORK_EXTENSIONS: [&str; 5] = ["svg", "pdf", "ai", "emf", "wmf"];

/// Imports artwork of type `ext` (a file extension, any case): as vector
/// objects (`vectors`), else as fills and running lines.
pub fn import_artwork(ext: &str, data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    match ext.to_ascii_lowercase().as_str() {
        "svg" if vectors => crate::svg::import_svg_vectors(data),
        "svg" => crate::svg::import_svg(data),
        "pdf" | "ai" => crate::pdf::import_pdf(data, vectors),
        "emf" => crate::emf::import_emf(data, vectors),
        "wmf" => crate::wmf::import_wmf(data, vectors),
        other => Err(SvgError::Parse(format!("cannot import .{other} artwork"))),
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
        assert!(import_artwork("eps", b"%!PS", false).is_err());
    }
}
