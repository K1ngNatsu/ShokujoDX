//! Pictures of design files for the file manager (P11 Milestone 3): the
//! thumbnail handlers of Windows, macOS and Linux all use these functions.
//!
//! - [`sniff`]: the kind of a design file from its first bytes. A Windows
//!   thumbnail handler gets the bytes of the file, not its name.
//! - [`stitches_of`]: the stitches of the bytes of a project or machine file.
//! - [`picture`]: the design with the surface of the 3D look (ADR 0130, at
//!   the fast level of detail), at most `px` pixels on its longest side,
//!   on a transparent background.

use emb_model::{Design, MachineCatalog, StitchDesign, project};
use std::ops::Range;

use emb_model::ObjectId;
use emb_render::raster::{Look, Picture};
use emb_stitch::{PlanOptions, StitchCache, build_plan};

/// The kinds of files that [`sniff`] knows from their first bytes. The
/// Windows handler is registered for these only; other formats have no
/// mark at the start of the file.
pub const SNIFFED: [&str; 7] = [project::FILE_EXTENSION, "pes", "pec", "vp3", "dst", "hus", "jef"];

/// The largest picture, in pixels.
pub const MAX_PX: u32 = 1024;

/// The kind of a design file (its usual extension) from its first bytes.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    let at = |k: usize, m: &[u8]| bytes.get(k..k + m.len()) == Some(m);
    let u32_at = |k: usize| {
        bytes
            .get(k..k + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if at(0, b"#PES") {
        return Some("pes");
    }
    if at(0, b"#PEC0001") {
        return Some("pec");
    }
    if at(0, b"%vsm%") {
        return Some("vp3");
    }
    if at(0, b"LA:") {
        return Some("dst");
    }
    if at(0, &[0x5b, 0xaf, 0xc8, 0x00]) {
        return Some("hus");
    }
    // A project file is JSON with the format name near the start.
    let head = &bytes[..bytes.len().min(256)];
    let text = String::from_utf8_lossy(head);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    if text.starts_with('{') && text.contains(&format!("\"{}\"", project::FORMAT)) {
        return Some(project::FILE_EXTENSION);
    }
    // JEF: the stitches start after the header and 8 bytes for each color.
    if let (Some(offset), Some(colors)) = (u32_at(0), u32_at(24))
        && colors < 1000
        && offset == 0x74 + 8 * colors
        && (offset as usize) < bytes.len()
    {
        return Some("jef");
    }
    None
}

/// The stitches of the bytes of a design file of kind `ext` (a project file
/// gets its stitch plan), and the design of a project file.
pub fn stitches_of(ext: &str, bytes: &[u8]) -> Result<(StitchDesign, Option<Design>), String> {
    plan_of(ext, bytes).map(|(s, d)| (s, d.map(|(d, _)| d)))
}

/// The records of each object of a stitch plan.
type Spans = Vec<(ObjectId, Range<usize>)>;

/// As [`stitches_of`], with the records of each object of the plan of a
/// project file.
fn plan_of(ext: &str, bytes: &[u8]) -> Result<(StitchDesign, Option<(Design, Spans)>), String> {
    if ext.eq_ignore_ascii_case(project::FILE_EXTENSION) {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let d = project::from_str(text).map_err(|e| e.to_string())?;
        let machines = MachineCatalog::builtin();
        let opts = d
            .machine
            .as_deref()
            .and_then(|id| machines.get(id))
            .map_or_else(PlanOptions::default, PlanOptions::for_machine);
        let plan = build_plan(&d, &opts, &mut StitchCache::default());
        let spans = plan.spans.iter().map(|s| (s.object, s.records.clone())).collect();
        return Ok((plan.stitches, Some((d, spans))));
    }
    emb_formats::read_machine(&ext.to_ascii_lowercase(), bytes).map(|s| (s, None))
}

/// The picture of the bytes of a design file: of kind `ext`, else of the
/// kind that [`sniff`] finds. At most `px` pixels on its longest side
/// (1 to [`MAX_PX`]); `None` if the file is not a design or has no stitches.
pub fn picture(bytes: &[u8], ext: Option<&str>, px: u32) -> Option<Picture> {
    let ext = ext
        .map(str::to_ascii_lowercase)
        .or_else(|| sniff(bytes).map(str::to_string))?;
    let (s, plan) = plan_of(&ext, bytes).ok()?;
    if s.stitch_count() == 0 {
        return None;
    }
    let areas = plan
        .as_ref()
        .map(|(d, _)| emb_render::sheet::picture_areas(d))
        .unwrap_or_default();
    // The size of the stitches (not of the jumps: a reader can add a jump
    // from the hoop corner) and of the areas.
    let b = emb_geom::BBox::from_points(
        s.records
            .iter()
            .filter(|r| r.kind == emb_model::StitchKind::Stitch)
            .map(|r| r.pos),
    )
    .into_iter()
    .chain(emb_geom::BBox::from_points(
        areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied()),
    ))
    .reduce(emb_geom::BBox::union)?;
    let margin = 1.0;
    let b = b.expand(margin);
    let px = f64::from(px.clamp(1, MAX_PX));
    // The longest side is `px` pixels (the size is rounded up).
    let px_per_mm = (px - 0.01) / b.width().max(b.height()).max(1e-3);
    let e = match plan {
        Some((d, spans)) => emb_render::picture::Embroidery::of_design(&d, &s, spans, None),
        None => emb_render::picture::Embroidery::of_stitches(&s),
    };
    let p = e.picture(
        b,
        px_per_mm,
        &Look::three_d(None),
        None,
        emb_render::picture::FAST_DETAIL,
        &|_| true,
    );
    (p.width > 0 && p.height > 0).then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_model::{Rgb, StitchRecord, Thread, ThreadId};

    fn square() -> StitchDesign {
        // Rows 0.5 mm apart of stitches 2 mm long (JEF has no stitches over 12.7 mm).
        let mut records = Vec::new();
        for i in 0..=40 {
            let y = f64::from(i) * 0.5;
            for k in 0..=10 {
                let x = f64::from(if i % 2 == 0 { k } else { 10 - k }) * 2.0;
                records.push(StitchRecord::stitch(x, y));
            }
        }
        StitchDesign {
            threads: vec![Thread {
                id: ThreadId(0),
                name: "Red".into(),
                color: Rgb(200, 30, 40),
                brand: None,
                code: None,
                ..Default::default()
            }],
            records,
        }
    }

    #[test]
    fn each_sniffed_format_is_found_from_its_bytes() {
        let s = square();
        // HUS can be read but not written: its mark only.
        assert_eq!(sniff(&[0x5b, 0xaf, 0xc8, 0x00, 1, 2, 3]), Some("hus"));
        for ext in SNIFFED.into_iter().filter(|e| *e != "hus") {
            let bytes = if ext == project::FILE_EXTENSION {
                project::to_string(&emb_model::design_from_stitches(&s, "square")).into_bytes()
            } else {
                emb_formats::write_machine(ext, &s, "square", "20260101000000").unwrap()
            };
            assert_eq!(sniff(&bytes), Some(ext), "{ext}");
            let p = picture(&bytes, None, 128).unwrap_or_else(|| panic!("{ext}: no picture"));
            assert!(
                p.width.max(p.height) <= 128 && p.width.max(p.height) >= 120,
                "{ext}: {}×{}",
                p.width,
                p.height
            );
            // The thread shows (red, but DST has no colors).
            let red = p.rgba.chunks(4).any(|c| c[3] > 200 && c[0] > c[2]);
            assert!(red || (ext == "dst" && p.rgba.chunks(4).any(|c| c[3] > 200)), "{ext}");
        }
    }

    #[test]
    fn other_bytes_give_no_picture() {
        assert_eq!(sniff(b""), None);
        assert_eq!(sniff(b"hello, this is not embroidery"), None);
        assert_eq!(sniff(br#"{"name": "a JSON file of another program"}"#), None);
        assert!(picture(b"#PES0001 damaged", None, 64).is_none());
        assert!(picture(&[0u8; 300], None, 64).is_none());
        // A machine file with no mark (EXP) has a picture when its kind is given.
        let exp = emb_formats::write_machine("exp", &square(), "x", "20260101000000").unwrap();
        assert!(picture(&exp, Some("exp"), 64).is_some());
    }
}
