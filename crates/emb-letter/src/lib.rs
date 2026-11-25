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
pub mod correction;
pub mod embfont;
pub mod font;
pub mod glyph;
pub mod grid;
pub mod inkstitch;
pub mod japanese;
pub mod keyboard;
pub mod layout;
pub mod monogram;
pub mod quality;
pub mod route;
pub mod shape;
pub mod sheet;
pub mod source;
mod split;
pub mod stroke;
pub mod style;
pub mod swatch;
pub mod teamnames;
pub mod tune;

pub use embfont::{EmbFont, EmbLetter};
pub use font::{Font, FontError, RouteMode, face_count, face_names};
pub use glyph::GlyphShapes;
pub use layout::{layout, layout_with_box, relayout};
pub use source::{AnyFont, LetterSource};

/// The fonts in `data/fonts/`, under the SIL Open Font License (the license
/// texts are next to the font files).
pub mod bundled {
    use super::{EmbFont, Font};

    /// (file name, bytes)
    pub const FILES: [(&str, &[u8]); 7] = [
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
        // P17 Milestone 6 (ADR 0114): a font for Arabic and a font for
        // Devanagari, from the google/fonts repository (the commit of
        // `tools/make-embroidery-fonts.py`), not changed. Both have Latin
        // letters too.
        (
            "Tajawal-Bold.ttf",
            include_bytes!("../../../data/fonts/Tajawal-Bold.ttf"),
        ),
        (
            "Poppins-Bold.ttf",
            include_bytes!("../../../data/fonts/Poppins-Bold.ttf"),
        ),
    ];

    /// The bundled font for Arabic text and the bundled font for
    /// Devanagari text (family, style), in [`FILES`].
    pub const ARABIC: (&str, &str) = ("Tajawal", "Bold");
    pub const DEVANAGARI: (&str, &str) = ("Poppins", "Bold");

    /// The embroidery fonts in `data/fonts/embroidery/` (Modified Versions
    /// of OFL fonts, ADR 0016): (family, the JSON compressed by `build.rs`;
    /// [`embroidery_text`] gives the text). `tools/make-embroidery-fonts.py`
    /// makes them; fonts with a Reserved Font Name have a new name (for
    /// example Bold Script: Lobster). The first nine came in P4 and P10,
    /// the next ones in P11 Milestone 2 (the first four of them for small
    /// text), and the fonts from [`P11_EMBROIDERY`] in P17 Milestone 7
    /// (ADR 0115). The fonts from [`P17_EMBROIDERY`] are the fill fonts
    /// (ADR 0117). Each one has a quality score of
    /// [`crate::quality::LIMIT`] or more ([`quality`]).
    pub const EMBROIDERY: [(&str, &[u8]); 68] = [
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
        (
            "Wide Tech",
            include_bytes!(concat!(env!("OUT_DIR"), "/wide-tech.embfont.z")),
        ),
        (
            "Plain Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/plain-sans.embfont.z")),
        ),
        (
            "Staatliches",
            include_bytes!(concat!(env!("OUT_DIR"), "/staatliches.embfont.z")),
        ),
        (
            "Square Tech",
            include_bytes!(concat!(env!("OUT_DIR"), "/square-tech.embfont.z")),
        ),
        (
            "Comic Neue",
            include_bytes!(concat!(env!("OUT_DIR"), "/comic-neue.embfont.z")),
        ),
        (
            "Sturdy Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/sturdy-sans.embfont.z")),
        ),
        (
            "Patrick Hand",
            include_bytes!(concat!(env!("OUT_DIR"), "/patrick-hand.embfont.z")),
        ),
        (
            "Chakra Petch",
            include_bytes!(concat!(env!("OUT_DIR"), "/chakra-petch.embfont.z")),
        ),
        (
            "Titillium Web",
            include_bytes!(concat!(env!("OUT_DIR"), "/titillium-web.embfont.z")),
        ),
        (
            "Sport Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/sport-sans.embfont.z")),
        ),
        (
            "Monoline Script",
            include_bytes!(concat!(env!("OUT_DIR"), "/monoline-script.embfont.z")),
        ),
        (
            "Amatic SC",
            include_bytes!(concat!(env!("OUT_DIR"), "/amatic-sc.embfont.z")),
        ),
        (
            "Friendly Hand",
            include_bytes!(concat!(env!("OUT_DIR"), "/friendly-hand.embfont.z")),
        ),
        (
            "Pangolin",
            include_bytes!(concat!(env!("OUT_DIR"), "/pangolin.embfont.z")),
        ),
        (
            "Soft Slab",
            include_bytes!(concat!(env!("OUT_DIR"), "/soft-slab.embfont.z")),
        ),
        ("Itim", include_bytes!(concat!(env!("OUT_DIR"), "/itim.embfont.z"))),
        (
            "Rajdhani",
            include_bytes!(concat!(env!("OUT_DIR"), "/rajdhani.embfont.z")),
        ),
        (
            "Racer Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/racer-sans.embfont.z")),
        ),
        (
            "Poppins",
            include_bytes!(concat!(env!("OUT_DIR"), "/poppins.embfont.z")),
        ),
        (
            "Indie Flower",
            include_bytes!(concat!(env!("OUT_DIR"), "/indie-flower.embfont.z")),
        ),
        (
            "Wide Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/wide-sans.embfont.z")),
        ),
        (
            "Bai Jamjuree",
            include_bytes!(concat!(env!("OUT_DIR"), "/bai-jamjuree.embfont.z")),
        ),
        (
            "Square Narrow",
            include_bytes!(concat!(env!("OUT_DIR"), "/square-narrow.embfont.z")),
        ),
        ("Alata", include_bytes!(concat!(env!("OUT_DIR"), "/alata.embfont.z"))),
        (
            "Neat Hand",
            include_bytes!(concat!(env!("OUT_DIR"), "/neat-hand.embfont.z")),
        ),
        (
            "Marker Hand",
            include_bytes!(concat!(env!("OUT_DIR"), "/marker-hand.embfont.z")),
        ),
        (
            "Barlow Condensed",
            include_bytes!(concat!(env!("OUT_DIR"), "/barlow-condensed.embfont.z")),
        ),
        (
            "Angle Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/angle-sans.embfont.z")),
        ),
        (
            "Balsamiq Sans",
            include_bytes!(concat!(env!("OUT_DIR"), "/balsamiq-sans.embfont.z")),
        ),
        // Three fonts of earlier versions, back after the audit of P21
        // (from [`P21_EMBROIDERY`]).
        (
            "Slab Heavy",
            include_bytes!(concat!(env!("OUT_DIR"), "/slab-heavy.embfont.z")),
        ),
        (
            "Fat Face Serif",
            include_bytes!(concat!(env!("OUT_DIR"), "/fat-face-serif.embfont.z")),
        ),
        (
            "Shrikhand",
            include_bytes!(concat!(env!("OUT_DIR"), "/shrikhand.embfont.z")),
        ),
        // The fill fonts (ADR 0117).
        (
            "Block Sans Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/block-sans-fill.embfont.z")),
        ),
        (
            "Varsity Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/varsity-fill.embfont.z")),
        ),
        (
            "Anton Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/anton-fill.embfont.z")),
        ),
        (
            "Archivo Black Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/archivo-black-fill.embfont.z")),
        ),
        (
            "Slab Heavy Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/slab-heavy-fill.embfont.z")),
        ),
        (
            "Poster Heavy Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/poster-heavy-fill.embfont.z")),
        ),
        (
            "Giant Round Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/giant-round-fill.embfont.z")),
        ),
        (
            "Funky Slab Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/funky-slab-fill.embfont.z")),
        ),
        (
            "Rubik Mono One Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/rubik-mono-one-fill.embfont.z")),
        ),
        (
            "Shrikhand Fill",
            include_bytes!(concat!(env!("OUT_DIR"), "/shrikhand-fill.embfont.z")),
        ),
    ];

    /// The tuned fonts of [`EMBROIDERY`] (P17 Milestone 5, ADR 0113): their
    /// letters are routed (few trims, travel under the strokes), and each
    /// letter was checked on proof pictures. The font lists show them
    /// first. `tools/make-embroidery-fonts.py` has the same list.
    /// The other fonts are routed too (ADR 0116), but their letters were
    /// not looked at one by one.
    pub const TUNED: [&str; 10] = [
        "Block Sans",
        "Friendly Serif",
        "Pacifico",
        "Atkinson Hyperlegible",
        "Varsity",
        "Bebas Neue",
        "Soft Round",
        "Geo Slab",
        "Brush Hand",
        "Vintage Script",
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
    pub const P10_EMBROIDERY: usize = 9;
    /// The number of bundled embroidery fonts before P17 Milestone 7 (the
    /// corpus designs `embroidery-fonts-5` to `-7` show the fonts from this
    /// one). Four fonts of earlier versions left the program then: their
    /// quality score was below the limit (ADR 0115). Three of them are back
    /// (from [`P21_EMBROIDERY`]); [`EARLIER`] is the other one.
    pub const P11_EMBROIDERY: usize = 26;
    /// The number of bundled embroidery fonts with columns before the audit
    /// of P21. The fonts from this one to [`P17_EMBROIDERY`] are three fonts
    /// of earlier versions that the owner brought back into the program
    /// (2026-11-24: the fonts of the package of earlier versions with a
    /// score over 85; ADR 0116). The corpus design `embroidery-fonts-9`
    /// shows them.
    pub const P21_EMBROIDERY: usize = 55;
    /// The number of bundled embroidery fonts before the fill fonts (ADR
    /// 0117; [`crate::EmbFont::fill`]). The fonts from this one are the
    /// fill fonts: the same source fonts as the fonts with their names
    /// (Block Sans Fill: Block Sans), with all letters as fills, for large
    /// text. The corpus design `embroidery-fonts-8` shows them. A new font
    /// with columns goes before them, a new fill font at the end.
    pub const P17_EMBROIDERY: usize = 58;

    /// The embroidery font of earlier versions that is not in the program
    /// now: its quality score was below [`crate::quality::LIMIT`] when it
    /// left it (75 after P21). It is in the font package
    /// `shokujodx-fonts-earlier.zip`. The other three fonts of that package
    /// had more than 85 after P21, and the owner brought them back
    /// ([`P21_EMBROIDERY`]).
    pub const EARLIER: [&str; 1] = ["Great Vibes"];

    /// The quality scores of the fonts of [`EMBROIDERY`], from their files
    /// (`build.rs` makes the table).
    const QUALITY: &[(&str, crate::quality::Quality)] = include!(concat!(env!("OUT_DIR"), "/embroidery_quality.rs"));

    /// `true` for a bundled fill font (ADR 0117), with no read of the font.
    pub fn is_fill(family: &str) -> bool {
        EMBROIDERY
            .iter()
            .position(|(f, _)| f.eq_ignore_ascii_case(family))
            .is_some_and(|i| i >= P17_EMBROIDERY)
    }

    /// The quality of a bundled embroidery font, with no read of the font.
    pub fn quality(family: &str) -> Option<crate::quality::Quality> {
        QUALITY
            .iter()
            .find(|(f, _)| f.eq_ignore_ascii_case(family))
            .map(|(_, q)| *q)
    }

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

    /// Each file of letter corrections in `data/fonts/embroidery/corrections`
    /// has the name of its font, and a bundled font has its corrections:
    /// `tools/make-embroidery-fonts.py` ran after the last change of the
    /// file (ADR 0112). The fonts of the font packages are checked in
    /// `emb-app` (`packaging.rs`), which reads the packages.
    #[test]
    fn the_bundled_fonts_have_their_corrections() {
        let dir = std::path::Path::new("../../data/fonts/embroidery/corrections");
        let Ok(files) = std::fs::read_dir(dir) else {
            return;
        };
        for path in files.flatten().map(|e| e.path()) {
            let name = path.display().to_string();
            assert!(
                path.extension().is_some_and(|e| e == correction::FILE_EXTENSION),
                "{name}"
            );
            let c = correction::Corrections::load(&path).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                path.file_name().and_then(|n| n.to_str()),
                Some(correction::file_name(&c.family).as_str()),
                "the file name of {}",
                c.family
            );
            let Some(font) = bundled::embroidery(&c.family) else {
                continue;
            };
            let missing = c.missing_in(&font);
            assert!(missing.is_empty(), "{name}: {missing:?}; make the fonts again");
        }
    }

    #[test]
    fn the_fill_fonts_are_the_last_fonts_of_the_list() {
        for (i, (family, _)) in bundled::EMBROIDERY.iter().enumerate() {
            let font = bundled::embroidery(family).unwrap();
            assert_eq!(font.fill, i >= bundled::P17_EMBROIDERY, "{family}");
            assert_eq!(font.fill, bundled::is_fill(family), "{family}");
            if !font.fill {
                continue;
            }
            // A fill font: fills only, no route, a range for large text.
            assert!(family.ends_with(" Fill"), "{family}");
            assert!(!font.tuned && !font.small_text, "{family}");
            for (c, letter) in &font.letters {
                assert!(!letter.fixed, "{family} {c:?}");
                assert!(
                    letter
                        .parts
                        .iter()
                        .all(|p| matches!(p, emb_model::ObjectKind::Fill { .. })),
                    "{family} {c:?}"
                );
            }
            assert!(font.min_height_mm >= quality::FILL_MIN_HEIGHT_MM, "{family}");
            assert_eq!(font.max_height_mm, source::MAX_HEIGHT_MM, "{family}");
            // The font with the same name and columns is in the program or
            // in a package.
            let plain = family.trim_end_matches(" Fill");
            assert!(
                bundled::embroidery(plain).is_some() || bundled::EARLIER.contains(&plain) || plain.contains(' '),
                "{family}"
            );
        }
        assert_eq!(bundled::EMBROIDERY.len() - bundled::P17_EMBROIDERY, 10);
    }

    /// The exit test of P17 Milestone 7: each bundled font has a quality
    /// score of the limit or more. The table of the scores is the scores
    /// in the font files.
    #[test]
    fn each_bundled_font_has_a_score_above_the_limit() {
        for (family, _) in bundled::EMBROIDERY {
            let font = bundled::embroidery(family).unwrap();
            let q = font
                .quality
                .unwrap_or_else(|| panic!("{family} has no quality score; make the fonts again"));
            assert!(q.passes(), "{family}: {q:?}");
            assert_eq!(bundled::quality(family), Some(q), "{family}");
            assert!(q.coverage > 0.9 && q.narrow < 0.2 && q.long < 0.2, "{family}: {q:?}");
        }
        // The new fonts of P17 Milestone 7 came into the program with the
        // score of a font of the program; a font keeps its place while its
        // score is the limit or more (ADR 0116; P21 Milestone 6: the score
        // counts the needle points, and Pangolin had 75 then).
        for (family, _) in &bundled::EMBROIDERY[bundled::P11_EMBROIDERY..] {
            let q = bundled::quality(family).unwrap();
            assert!(q.score >= quality::LIMIT, "{family}: {q:?}");
        }
        assert_eq!(bundled::quality("no such font"), None);
        // 65, and three fonts of earlier versions back after the audit of P21.
        assert_eq!(bundled::EMBROIDERY.len(), 68);
    }

    /// The score in a font file is the score of the font as it is. The
    /// source fonts of some bundled fonts are in the repository.
    #[test]
    fn the_score_in_the_file_is_the_score_of_the_font() {
        for (family, file) in [
            ("Block Sans", "Lato-Bold.ttf"),
            ("Anton", "Anton-Regular.ttf"),
            ("Pacifico", "Pacifico-Regular.ttf"),
            ("Varsity", "Graduate-Regular.ttf"),
        ] {
            let bytes = bundled::FILES.iter().find(|(n, _)| *n == file).unwrap().1;
            let source = Font::from_bytes(bytes.to_vec(), 0).unwrap();
            let font = bundled::embroidery(family).unwrap();
            assert_eq!(
                quality::measure(&font, &source),
                font.quality,
                "{family}; make the fonts again"
            );
            // The size range fits the columns: a new fit does not change it.
            let mut again = font.clone();
            again.fit_size_range();
            assert_eq!(
                (again.min_height_mm, again.max_height_mm),
                (font.min_height_mm, font.max_height_mm),
                "{family}"
            );
        }
    }

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
        // The tuned fonts are the fonts of the list.
        for (family, _) in bundled::EMBROIDERY {
            let tuned = bundled::embroidery(family).is_some_and(|f| f.tuned);
            assert_eq!(tuned, bundled::TUNED.contains(&family), "{family}");
        }
        // No family is in the list two times, and the fonts of earlier
        // versions are not in it.
        for (i, (family, _)) in bundled::EMBROIDERY.iter().enumerate() {
            assert!(
                bundled::EMBROIDERY[..i]
                    .iter()
                    .all(|(f, _)| !f.eq_ignore_ascii_case(family)),
                "{family}"
            );
            assert!(!bundled::EARLIER.contains(family), "{family}");
        }
        assert!(bundled::embroidery(bundled::DEFAULT_EMBROIDERY).is_some());
        assert_eq!(bundled::fonts().len(), bundled::FILES.len());
    }
}
