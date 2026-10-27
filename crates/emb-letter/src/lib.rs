//! Lettering: TrueType and OpenType fonts as embroidery letters.
//!
//! - [`Font`]: a font face, its metrics, glyph outlines and kerning.
//! - [`stroke`]: satin columns from the strokes of a glyph shape.
//! - [`EmbFont`]: embroidery fonts (`.embfont`, ADR 0016) and the converter.
//! - [`LetterSource`]: the letters of either kind of font for layout.
//! - [`layout`](mod@layout): a lettering object's text as letters made of satin and
//!   fill parts.
//! - [`bitmap`]: BDF and Windows raster fonts as embroidery fonts.
//! - [`teamnames`]: the design of each name of a team names list, and all in one.
//! - [`keyboard`]: keyboard design collections (a design for each key).
//! - [`monogram`]: the objects of a monogram (initials, ornaments, border).
//! - [`bundled`]: the fonts that come with the app (SIL Open Font License).
//!
//! The engine crates do not read font files; the app gives the bytes.
//! See ADR 0015.
#![forbid(unsafe_code)]

pub mod bitmap;
pub mod embfont;
pub mod font;
pub mod glyph;
pub mod grid;
pub mod keyboard;
pub mod layout;
pub mod monogram;
pub mod source;
pub mod stroke;
pub mod teamnames;

pub use embfont::{EmbFont, EmbLetter};
pub use font::{Font, FontError, face_count, face_names};
pub use glyph::GlyphShapes;
pub use layout::{layout, layout_with_box, relayout};
pub use source::{AnyFont, LetterSource};

/// The fonts in `data/fonts/`, under the SIL Open Font License (the license
/// texts are next to the font files).
pub mod bundled {
    use super::{EmbFont, Font};

    /// (file name, bytes)
    pub const FILES: [(&str, &[u8]); 5] = [
        (
            "Lato-Regular.ttf",
            include_bytes!("../../../data/fonts/Lato-Regular.ttf"),
        ),
        ("Lato-Bold.ttf", include_bytes!("../../../data/fonts/Lato-Bold.ttf")),
        (
            "Pacifico-Regular.ttf",
            include_bytes!("../../../data/fonts/Pacifico-Regular.ttf"),
        ),
        (
            "Anton-Regular.ttf",
            include_bytes!("../../../data/fonts/Anton-Regular.ttf"),
        ),
        (
            "Graduate-Regular.ttf",
            include_bytes!("../../../data/fonts/Graduate-Regular.ttf"),
        ),
    ];

    /// The embroidery fonts in `data/fonts/embroidery/` (Modified Versions
    /// of OFL fonts, ADR 0016): (family, the JSON compressed by `build.rs`;
    /// [`embroidery_text`] gives the text). `tools/make-embroidery-fonts.py`
    /// makes them; fonts with a Reserved Font Name have a new name (for
    /// example Bold Script: Lobster). The first ten came in P4 and P10, the
    /// others in P11 Milestone 2 (the first four of them for small text).
    pub const EMBROIDERY: [(&str, &[u8]); 30] = [
        (
            "Block Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/block-sans.embfont.z")),
        ),
        ("Anton", include_bytes!(concat!(env!("OUT_DIR"), "/anton.embfont.z"))),
        (
            "Pacifico",
            include_bytes!(concat!(env!("OUT_DIR"), "/pacifico.embfont.z")),
        ),
        (
            "Varsity",
            include_bytes!(concat!(env!("OUT_DIR"), "/varsity.embfont.z")),
        ),
        (
            "Bebas Neue",
            include_bytes!(concat!(env!("OUT_DIR"), "/bebas-neue.embfont.z")),
        ),
        (
            "Bangers",
            include_bytes!(concat!(env!("OUT_DIR"), "/bangers.embfont.z")),
        ),
        (
            "Archivo Black",
            include_bytes!(concat!(env!("OUT_DIR"), "/archivo-black.embfont.z")),
        ),
        (
            "Bold Script",
            include_bytes!(concat!(env!("OUT_DIR"), "/bold-script.embfont.z")),
        ),
        (
            "Slab Heavy",
            include_bytes!(concat!(env!("OUT_DIR"), "/slab-heavy.embfont.z")),
        ),
        (
            "Casual Script",
            include_bytes!(concat!(env!("OUT_DIR"), "/casual-script.embfont.z")),
        ),
        (
            "Atkinson Hyperlegible",
            include_bytes!(concat!(env!("OUT_DIR"), "/atkinson-hyperlegible.embfont.z")),
        ),
        (
            "Soft Round",
            include_bytes!(concat!(env!("OUT_DIR"), "/soft-round.embfont.z")),
        ),
        (
            "Round Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/round-sans.embfont.z")),
        ),
        (
            "Round Heavy",
            include_bytes!(concat!(env!("OUT_DIR"), "/round-heavy.embfont.z")),
        ),
        (
            "Fjalla One",
            include_bytes!(concat!(env!("OUT_DIR"), "/fjalla-one.embfont.z")),
        ),
        (
            "Poster Bold",
            include_bytes!(concat!(env!("OUT_DIR"), "/poster-bold.embfont.z")),
        ),
        (
            "Geo Slab",
            include_bytes!(concat!(env!("OUT_DIR"), "/geo-slab.embfont.z")),
        ),
        (
            "Friendly Serif",
            include_bytes!(concat!(env!("OUT_DIR"), "/friendly-serif.embfont.z")),
        ),
        (
            "Slab Round",
            include_bytes!(concat!(env!("OUT_DIR"), "/slab-round.embfont.z")),
        ),
        (
            "Fat Face Serif",
            include_bytes!(concat!(env!("OUT_DIR"), "/fat-face-serif.embfont.z")),
        ),
        (
            "Shrikhand",
            include_bytes!(concat!(env!("OUT_DIR"), "/shrikhand.embfont.z")),
        ),
        (
            "Great Vibes",
            include_bytes!(concat!(env!("OUT_DIR"), "/great-vibes.embfont.z")),
        ),
        (
            "Vintage Script",
            include_bytes!(concat!(env!("OUT_DIR"), "/vintage-script.embfont.z")),
        ),
        (
            "Brush Hand",
            include_bytes!(concat!(env!("OUT_DIR"), "/brush-hand.embfont.z")),
        ),
        (
            "Retro Script",
            include_bytes!(concat!(env!("OUT_DIR"), "/retro-script.embfont.z")),
        ),
        (
            "Leckerli One",
            include_bytes!(concat!(env!("OUT_DIR"), "/leckerli-one.embfont.z")),
        ),
        ("Damion", include_bytes!(concat!(env!("OUT_DIR"), "/damion.embfont.z"))),
        (
            "Black Ops One",
            include_bytes!(concat!(env!("OUT_DIR"), "/black-ops-one.embfont.z")),
        ),
        ("Bungee", include_bytes!(concat!(env!("OUT_DIR"), "/bungee.embfont.z"))),
        (
            "Retro Display",
            include_bytes!(concat!(env!("OUT_DIR"), "/retro-display.embfont.z")),
        ),
    ];

    /// The text (JSON) of the bundled embroidery font `i` of [`EMBROIDERY`].
    pub fn embroidery_text(i: usize) -> Option<std::sync::Arc<str>> {
        use std::io::Read;
        use std::sync::{Arc, OnceLock};
        static TEXTS: OnceLock<Vec<OnceLock<Arc<str>>>> = OnceLock::new();
        let texts = TEXTS.get_or_init(|| EMBROIDERY.iter().map(|_| OnceLock::new()).collect());
        let (_, z) = EMBROIDERY.get(i)?;
        Some(
            texts[i]
                .get_or_init(|| {
                    let mut s = String::new();
                    flate2::read::DeflateDecoder::new(*z)
                        .read_to_string(&mut s)
                        .expect("a bundled font decompresses (tested)");
                    Arc::from(s)
                })
                .clone(),
        )
    }

    /// The number of bundled embroidery fonts before P10 (the corpus design
    /// `embroidery-fonts` shows these; `embroidery-fonts-2` the others).
    pub const FIRST_EMBROIDERY: usize = 4;
    /// The number of bundled embroidery fonts before P11 (`embroidery-fonts-2`
    /// shows the fonts from [`FIRST_EMBROIDERY`] to this one;
    /// `embroidery-fonts-3` and `-4` the fonts of P11 Milestone 2).
    pub const P10_EMBROIDERY: usize = 10;

    /// The default lettering font: an embroidery font, so that new
    /// lettering sews the same on each computer.
    pub const DEFAULT_EMBROIDERY: &str = "Block Sans";

    /// A bundled embroidery font by family.
    pub fn embroidery(family: &str) -> Option<EmbFont> {
        let i = EMBROIDERY.iter().position(|(f, _)| f.eq_ignore_ascii_case(family))?;
        EmbFont::from_json(&embroidery_text(i)?).ok()
    }

    /// All bundled fonts.
    pub fn fonts() -> Vec<Font> {
        FILES
            .iter()
            .filter_map(|(_, b)| Font::from_bytes(b.to_vec(), 0).ok())
            .collect()
    }

    /// A bundled font by family and style.
    pub fn font(family: &str, style: &str) -> Option<Font> {
        fonts()
            .into_iter()
            .find(|f| f.family.eq_ignore_ascii_case(family) && f.style.eq_ignore_ascii_case(style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_embroidery_fonts_load() {
        for (family, _) in bundled::EMBROIDERY {
            let f = bundled::embroidery(family).unwrap_or_else(|| panic!("{family}"));
            assert_eq!(f.family, family);
            assert!(f.letters.len() > 150, "{family}");
            let floor = if f.small_text {
                crate::source::SMALL_TEXT_MIN_HEIGHT_MM
            } else {
                crate::source::MIN_HEIGHT_MM
            };
            assert!(
                f.min_height_mm >= floor && f.max_height_mm > f.min_height_mm,
                "{family}"
            );
        }
        // The fonts for small text go down to 4 mm.
        let small: Vec<&str> = bundled::EMBROIDERY
            .iter()
            .map(|(f, _)| *f)
            .filter(|f| bundled::embroidery(f).is_some_and(|e| e.small_text))
            .collect();
        assert_eq!(
            small,
            ["Atkinson Hyperlegible", "Soft Round", "Round Sans", "Round Heavy"]
        );
        assert!(
            small
                .iter()
                .filter(|f| bundled::embroidery(f).unwrap().min_height_mm <= 4.0)
                .count()
                >= 3
        );
        assert!(bundled::embroidery(bundled::DEFAULT_EMBROIDERY).is_some());
        assert_eq!(bundled::fonts().len(), bundled::FILES.len());
    }
}
