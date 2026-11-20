use emb_geom::Point;
use emb_model::{Underlay, UnderlayLayer};

use super::*;

/// A small font in the format of Ink/Stitch: A (a satin column), V (a
/// fill), "fi" (a ligature); the glyph layers are hidden but one, and the
/// baseline guide is up from the bottom of the page (y 70 from the top).
const LTR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
 xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:inkstitch="http://inkstitch.org/namespace"
 width="100mm" height="100mm" viewBox="0 0 100 100">
 <sodipodi:namedview id="nv"><sodipodi:guide position="0,30" orientation="0,1" id="g1" inkscape:label="baseline"/></sodipodi:namedview>
 <metadata><inkstitch:inkstitch_svg_version>4</inkstitch:inkstitch_svg_version></metadata>
 <g inkscape:groupmode="layer" inkscape:label="GlyphLayer-A" id="la" style="display:none">
  <path d="M 2 70 L 5 50 M 4 70 L 7 50 M 1 60 L 7 60" style="fill:none;stroke:#000000;stroke-width:0.3"
   inkstitch:satin_column="true" inkstitch:pull_compensation_mm="0.3" inkstitch:center_walk_underlay="true"/>
  <path d="M 5 50 L 8 70 M 7 50 L 10 70 M 5 60 L 11 60" style="fill:none;stroke:#000000;stroke-width:0.3"
   inkstitch:satin_column="true"/>
 </g>
 <g inkscape:groupmode="layer" inkscape:label="GlyphLayer-V" id="lv" style="display:none">
  <path d="M 0 50 L 4 50 L 6 65 L 8 50 L 12 50 L 7 70 L 5 70 Z" style="fill:#000000"
   inkstitch:angle="30" inkstitch:fill_underlay="false"/>
 </g>
 <g inkscape:groupmode="layer" inkscape:label="GlyphLayer-fi" id="lfi">
  <path d="M 0 70 L 0 50" style="fill:none;stroke:#000000"/>
 </g>
</svg>"##;

const JSON: &str = r#"{"name": "Test Font", "font_license": "SIL Open Font License v1.1",
 "original_font": "Test Sans", "original_font_url": "https://example.org/test",
 "leading": 30, "horiz_adv_x": {"A": 12}, "horiz_adv_x_space": 5,
 "kerning_pairs": {"A V": 2, "VA": 1}, "min_scale": 0.5, "max_scale": 2, "letter_case": "upper"}"#;

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emb-inkstitch-font-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ltr.svg"), LTR).unwrap();
    std::fs::write(dir.join("font.json"), format!("\u{feff}{JSON}")).unwrap();
    std::fs::write(
        dir.join("LICENSE"),
        "This font is licensed under the OFL.\n\nThe full text.",
    )
    .unwrap();
    dir
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

fn bbox(l: &EmbLetter) -> BBox {
    l.parts
        .iter()
        .filter_map(ObjectKind::control_bbox)
        .reduce(BBox::union)
        .unwrap()
}

#[test]
fn a_font_of_inkstitch_becomes_an_embroidery_font() {
    let dir = folder("convert");
    assert!(is_inkstitch_font(&dir));
    let c = convert_inkstitch_font(&dir.join("font.json")).unwrap();
    let f = &c.font;
    assert_eq!(f.family, "Test Font");
    assert_eq!(c.license, "SIL Open Font License v1.1");
    assert!(c.license_text.as_deref().unwrap().contains("OFL"));
    assert!(
        f.notes.contains("Test Sans") && f.notes.contains("licensed under the OFL"),
        "{}",
        f.notes
    );
    // A, V and a copy of A for "a" (the font is upper case); "fi" is not a letter.
    let chars: String = f.letters.keys().collect();
    assert_eq!(chars, "AVav");
    assert!(
        c.warnings.iter().any(|w| w.contains("more than one character")),
        "{:?}",
        c.warnings
    );
    let a = &f.letters[&'A'];
    assert!(a.fixed);
    assert_eq!(a.parts.len(), 2);
    assert!(a.parts.iter().all(|p| matches!(p, ObjectKind::Satin { .. })));
    // The letter stands on the baseline (y 0) and starts at x 0.
    let b = bbox(a);
    assert!(
        close(b.max.y, 0.0) && close(b.min.y, -20.0) && close(b.min.x, 2.0),
        "{b:?}"
    );
    assert!(close(a.advance_mm, 12.0));
    assert_eq!(a.tuning.len(), 2);
    assert!(close(a.tuning[0].pull_mm, 0.3 - letter_satin().pull_compensation_mm));
    assert_eq!(
        a.tuning[0].underlay,
        Some(Underlay::Custom(vec![UnderlayLayer::CenterWalk]))
    );
    let v = &f.letters[&'V'];
    assert!(matches!(v.parts[0], ObjectKind::Fill { .. }));
    // The advance of a glyph with none is its right edge.
    assert!(close(v.advance_mm, 12.0));
    assert!(close(v.tuning[0].angle_deg, -30.0 - letter_fill().angle_deg));
    assert_eq!(v.tuning[0].underlay, Some(Underlay::None));
    // Kerning is subtracted in Ink/Stitch and added in the program.
    assert!(close(f.kerning["AV"], -2.0) && close(f.kerning["VA"], -1.0));
    assert!(close(f.cap_height_mm, 20.0));
    assert!(close(f.min_height_mm, 10.0) && close(f.max_height_mm, 40.0));
    assert!(close(f.line_height_mm, 30.0) && close(f.space_mm, 5.0));
    assert!(!f.fill);
    // The file reads again.
    let back = EmbFont::from_json(&f.to_json()).unwrap();
    assert_eq!(back.letters.len(), 4);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_folder_with_no_glyph_file_is_not_a_font() {
    let dir = folder("empty");
    std::fs::remove_file(dir.join("ltr.svg")).unwrap();
    assert!(convert_inkstitch_font(&dir).unwrap_err().contains("ltr.svg"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_letters_of_a_converted_font_sew_in_lettering() {
    let dir = folder("layout");
    let font = convert_inkstitch_font(&dir).unwrap().font;
    let letters = crate::layout::layout(
        &emb_model::Lettering::new(
            "AVA",
            emb_model::FontRef::embroidery("Test Font", "Regular"),
            10.0,
            Point::ZERO,
        ),
        &font,
    );
    assert_eq!(letters.len(), 3);
    // At half size: A starts 1 mm right of the pen and advances 6 mm, and
    // V (at its pen) moves 1 mm back (the kerning).
    let x = |i: usize| {
        letters[i]
            .parts
            .iter()
            .filter_map(ObjectKind::control_bbox)
            .reduce(BBox::union)
            .unwrap()
            .min
            .x
    };
    let bb = |i: usize| {
        letters[i]
            .parts
            .iter()
            .filter_map(ObjectKind::control_bbox)
            .reduce(BBox::union)
            .unwrap()
    };
    assert!(
        close(x(1) - x(0), 6.0 - 1.0 - 1.0),
        "{} {} {:?} {:?}",
        x(0),
        x(1),
        bb(0),
        bb(1)
    );
    std::fs::remove_dir_all(&dir).ok();
}
