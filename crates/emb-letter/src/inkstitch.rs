//! Lettering fonts of Ink/Stitch as embroidery fonts (P18 Milestone 6,
//! ADR 0136).
//!
//! A font of Ink/Stitch is a folder with:
//!
//! - `font.json`: the name, the license, the line pitch (`leading`), the
//!   advance of each glyph (`horiz_adv_x`, else `horiz_adv_x_default`, else
//!   the right edge of the glyph), the kerning of pairs (`kerning_pairs`,
//!   subtracted from the advance), the word spacing (`horiz_adv_x_space`),
//!   the scale range (`min_scale`, `max_scale`) and the letter case. The
//!   lengths are in the user units of the glyph file.
//! - `LICENSE`: the license text of the font.
//! - a glyph file for the direction from left to right (`ltr.svg`, `→.svg`
//!   in older fonts, or a folder of that name with SVG files): one layer
//!   `GlyphLayer-<name>` for each glyph, with the objects of the letter and
//!   their settings of Ink/Stitch (`emb_formats::inkstitch`). The glyph
//!   starts at x 0 and stands on the guide named "baseline".
//!
//! Each glyph with a name of one character becomes a letter: its parts in
//! the order of the font (a person digitized them, so the letter is
//! fixed), with the underlay, the pull compensation and the fill angle of
//! each part as its tuning (the lettering object sets the other settings).
//!
//! Sources (rule 4): the file names, the keys of `font.json` and the rules
//! of the layout are from the source of Ink/Stitch, commit
//! 42e771e9d4338ce32a80f4d8c0df186a8c8210e7 (`lib/lettering/font.py`,
//! `font_variant.py`, `glyph.py`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use emb_geom::{BBox, Transform, text};
use emb_model::lettering::{letter_fill, letter_satin};
use emb_model::{ObjectKind, PartTuning};
use serde_json::Value;

use crate::embfont::{EmbFont, EmbLetter};

/// The license that Ink/Stitch gives a font with no `font_license`.
const DEFAULT_LICENSE: &str = "SIL Open Font License v1.1";

/// A font of Ink/Stitch, converted.
#[derive(Debug, Clone)]
pub struct Converted {
    pub font: EmbFont,
    /// The license name of `font.json`.
    pub license: String,
    /// The text of the `LICENSE` file, if the folder has one.
    pub license_text: Option<String>,
    /// What the converter could not keep, for the user.
    pub warnings: Vec<String>,
}

/// The folder of a font of Ink/Stitch: `path` itself, or the folder of a
/// `font.json` file.
pub fn font_folder(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    }
}

/// Returns `true` if the folder has the `font.json` of a font of Ink/Stitch.
pub fn is_inkstitch_font(dir: &Path) -> bool {
    dir.join("font.json").is_file()
}

/// The glyph files of the direction from left to right.
fn glyph_files(dir: &Path) -> Vec<PathBuf> {
    for name in ["ltr", "→"] {
        let file = dir.join(format!("{name}.svg"));
        if file.is_file() {
            return vec![file];
        }
        let sub = dir.join(name);
        if sub.is_dir()
            && let Ok(entries) = std::fs::read_dir(&sub)
        {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")))
                .collect();
            files.sort();
            if !files.is_empty() {
                return files;
            }
        }
    }
    Vec::new()
}

fn read_text(path: &Path) -> Result<String, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(s.trim_start_matches('\u{feff}').to_string())
}

/// Converts the font of Ink/Stitch in `dir` (or the folder of a
/// `font.json`).
pub fn convert_inkstitch_font(dir: &Path) -> Result<Converted, String> {
    let dir = font_folder(dir);
    let json: Value =
        serde_json::from_str(&read_text(&dir.join("font.json"))?).map_err(|e| text::fill("font.json: {}", &[&e]))?;
    let files = glyph_files(&dir);
    if files.is_empty() {
        return Err(text::fill(
            "{}: the folder has no glyph file (ltr.svg) of an Ink/Stitch font.",
            &[&(dir.display())],
        ));
    }
    let mut glyphs: Vec<(String, Vec<emb_formats::svg::ImportedObject>)> = Vec::new();
    let mut warnings = Vec::new();
    let mut place = None;
    for f in &files {
        let data = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let g = emb_formats::inkstitch::import_inkstitch_glyphs(&data).map_err(|e| format!("{}: {e}", f.display()))?;
        warnings.extend(g.warnings.iter().cloned());
        if place.is_none() {
            place = Some((g.baseline, g.origin, g.mm_per_unit));
        }
        glyphs.extend(g.glyphs);
    }
    let (baseline, origin, unit) = place.unwrap_or((None, [0.0, 0.0], 1.0));
    let num = |key: &str, default: f64| json.get(key).and_then(Value::as_f64).unwrap_or(default);
    let name = json
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    // The baseline: the guide, as Inkscape wrote it (from the top or from
    // the bottom of the page: the one where most glyphs stand), else y 0.
    let bottoms: Vec<f64> = {
        let mut v: Vec<f64> = glyphs
            .iter()
            .filter_map(|(_, objs)| objs.iter().filter_map(|o| o.kind.control_bbox()).reduce(BBox::union))
            .map(|b| b.max.y)
            .collect();
        v.sort_by(f64::total_cmp);
        v
    };
    let median = bottoms.get(bottoms.len() / 2).copied().unwrap_or(origin[1]);
    let base_y = match baseline {
        Some([top, bottom]) => {
            if (top - median).abs() <= (bottom - median).abs() {
                top
            } else {
                bottom
            }
        }
        None => origin[1],
    };
    let to_letter = Transform::translate(-origin[0], -base_y);
    let advances = json.get("horiz_adv_x").and_then(Value::as_object);
    let default_advance = json.get("horiz_adv_x_default").and_then(Value::as_f64);
    let (satin, fill) = (letter_satin(), letter_fill());
    let mut letters: BTreeMap<char, EmbLetter> = BTreeMap::new();
    let mut long_names = 0;
    for (gname, objs) in &glyphs {
        let mut chars = gname.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            long_names += 1;
            continue;
        };
        let mut parts = Vec::new();
        let mut tuning = Vec::new();
        for o in objs {
            let mut kind = o.kind.clone();
            let t = match &kind {
                ObjectKind::Satin { params, .. } => PartTuning {
                    underlay: Some(params.underlay.clone()),
                    pull_mm: params.pull_compensation_mm - satin.pull_compensation_mm,
                    angle_deg: 0.0,
                },
                ObjectKind::Fill { params, .. } => PartTuning {
                    underlay: Some(params.underlay.clone()),
                    pull_mm: params.pull_compensation_mm - fill.pull_compensation_mm,
                    angle_deg: params.angle_deg - fill.angle_deg,
                },
                ObjectKind::Running { .. } => PartTuning::default(),
                _ => continue,
            };
            kind.transform(&to_letter);
            parts.push(kind);
            tuning.push(t);
        }
        if parts.is_empty() {
            continue;
        }
        let right = parts
            .iter()
            .filter_map(ObjectKind::control_bbox)
            .reduce(BBox::union)
            .map_or(0.0, |b| b.max.x);
        let advance_mm = advances
            .and_then(|a| a.get(gname.as_str()))
            .and_then(Value::as_f64)
            .or(default_advance)
            .map_or(right, |a| a * unit);
        letters.insert(
            c,
            EmbLetter {
                advance_mm,
                parts,
                fixed: true,
                tuning,
                fill_parts: Vec::new(),
            },
        );
    }
    if letters.is_empty() {
        return Err(text::fill("{}: the font has no letters.", &[&(dir.display())]));
    }
    if long_names > 0 {
        warnings.push(text::fill(
            "{} glyph(s) with a name of more than one character (ligatures, forms of letters) are not used.",
            &[&long_names],
        ));
    }
    // Typed letters of the other case.
    let case = json.get("letter_case").and_then(Value::as_str).unwrap_or("");
    let copies: Vec<(char, EmbLetter)> = match case {
        "upper" => letters
            .iter()
            .filter(|(c, _)| c.is_uppercase())
            .flat_map(|(c, l)| c.to_lowercase().map(|lc| (lc, l.clone())).collect::<Vec<_>>())
            .collect(),
        "lower" => letters
            .iter()
            .filter(|(c, _)| c.is_lowercase())
            .flat_map(|(c, l)| c.to_uppercase().map(|uc| (uc, l.clone())).collect::<Vec<_>>())
            .collect(),
        _ => Vec::new(),
    };
    for (c, l) in copies {
        letters.entry(c).or_insert(l);
    }
    // Kerning: "A V" or "AV" in Ink/Stitch, subtracted from the advance.
    let mut kerning = BTreeMap::new();
    if let Some(pairs) = json.get("kerning_pairs").and_then(Value::as_object) {
        for (key, v) in pairs {
            let Some(k) = v.as_f64() else { continue };
            let pair: Vec<char> = match key.split_once(' ') {
                Some((a, b)) => a.chars().chain(b.chars()).collect(),
                None => key.chars().collect(),
            };
            if let [a, b] = pair[..]
                && k != 0.0
            {
                kerning.insert(format!("{a}{b}"), -k * unit);
            }
        }
    }
    if json.get("text_direction").and_then(Value::as_str) == Some("rtl") {
        warnings.push(text::tr("The font is for text from right to left; the letters are kept.").to_string());
    }
    // The capital height: of H, else of the tallest letter.
    let height = |l: &EmbLetter| {
        l.parts
            .iter()
            .filter_map(ObjectKind::control_bbox)
            .reduce(BBox::union)
            .map_or(0.0, |b| -b.min.y)
    };
    let cap = letters
        .get(&'H')
        .map(height)
        .filter(|h| *h > 0.0)
        .unwrap_or_else(|| letters.values().map(height).fold(0.0, f64::max))
        .max(1.0);
    let (lo, hi) = (num("min_scale", 1.0), num("max_scale", 1.0));
    // A range of one size is the font with no range: a little smaller and
    // larger.
    let range = if hi > lo && lo > 0.0 {
        (cap * lo, cap * hi)
    } else {
        (cap * 0.8, cap * 1.25)
    };
    let license = json
        .get("font_license")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or(DEFAULT_LICENSE)
        .to_string();
    let license_text = read_text(&dir.join("LICENSE")).ok();
    let mut notes = format!("Converted from the Ink/Stitch font \"{name}\". License: {license}.");
    if let Some(of) = json
        .get("original_font")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        notes.push_str(&format!(" Original font: {of}"));
        if let Some(url) = json
            .get("original_font_url")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            notes.push_str(&format!(" ({})", url.trim()));
        }
        notes.push('.');
    }
    if let Some(first) = license_text.as_deref().and_then(|t| t.split("\n\n").next()) {
        notes.push(' ');
        notes.push_str(&first.split_whitespace().collect::<Vec<_>>().join(" "));
    }
    let font = EmbFont::from_letters(
        &name,
        notes,
        cap,
        num("leading", 100.0) * unit,
        num("horiz_adv_x_space", 20.0) * unit,
        range,
        letters,
        kerning,
    );
    font.validate()
        .map_err(|e| text::fill("the converted font has a wrong value: {}", &[&e]))?;
    Ok(Converted {
        font,
        license,
        license_text,
        warnings,
    })
}

#[cfg(test)]
mod tests;
