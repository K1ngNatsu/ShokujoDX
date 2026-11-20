//! Ink/Stitch files (P18 Milestone 5, ADR 0135).
//!
//! Ink/Stitch (GPL-3.0) keeps the settings of each shape of an SVG file in
//! attributes of its own namespace. [`import_inkstitch`] reads them, so the
//! shapes become objects with stitch types:
//!
//! - A shape with a fill becomes a fill: tatami, square, contour (rings,
//!   one spiral, two spirals), guided, circular, meander, cross stitch or
//!   a linear gradient (a group of fills, one for each color). A fill type
//!   that the program does not have is tatami, with a warning.
//! - A shape with a stroke and the satin setting becomes a satin column:
//!   two rails and the rungs across them, or a line with a wide stroke.
//! - Another shape with a stroke becomes a line: a run (with bean stitches
//!   and passes), a zigzag of the stroke width, a ripple or manual stitches.
//! - The commands (trim, stop, start and end points, the target point,
//!   ignore) and the settings for trims, stops and lock stitches go to the
//!   machine functions and the start and end points of the objects. A shape
//!   that Ink/Stitch does not sew becomes a vector object.
//! - The thread colors are the colors of the shapes. When the file names a
//!   thread palette that is a chart of the program, a thread with a color of
//!   the chart gets its name, brand and number.
//! - A setting that the program does not read gives a warning.
//!
//! Sources (rule 4): the names, the defaults and the meanings of the
//! settings, the commands, the markers, the metadata and the updates of
//! earlier files are from the source of Ink/Stitch, commit
//! 42e771e9d4338ce32a80f4d8c0df186a8c8210e7 (`lib/svg/tags.py`,
//! `lib/elements/`, `lib/commands.py`, `lib/marker.py`, `lib/update.py`,
//! `lib/metadata.py`, `lib/extensions/apply_palette.py`). [`rail_indices`]
//! is ported from `lib/elements/satin_column/satin_column.py`
//! (`rail_indices`) and [`rails_to_reverse`] from
//! `lib/elements/satin_column/rails.py` (`get_rails_to_reverse`), the same
//! commit.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::Range;

use emb_geom::{Path, PathCmd, Point, math, region, text};
use emb_model::object::SATIN_SPLIT_MM;
use emb_model::{
    Accordion, ColumnPattern, Corner, CrossStyle, FillParams, FillPattern, Functions, LineBorder, MeanderTile,
    ObjectKind, Rgb, Ripple, RunPass, RunStyle, RunningParams, SatinGeometry, SatinParams, TrimMode, Underlay,
    UnderlayLayer, VectorStyle,
};

use crate::svg::{FALLBACK, ImportedObject, MM_PER_PX, SvgError, SvgImport, subpaths};

/// The namespace of the settings of Ink/Stitch.
pub const NAMESPACE: &str = "http://inkstitch.org/namespace";
/// The newest SVG version of Ink/Stitch (`lib/update.py`).
pub const SVG_VERSION: u32 = 4;
const INKSCAPE: &str = "http://www.inkscape.org/namespaces/inkscape";
const XLINK: &str = "http://www.w3.org/1999/xlink";
/// Tolerance to flatten rails, rungs and guide lines, in mm.
const FLAT_MM: f64 = 0.05;
/// A shape with the satin setting and one subpath is a satin column if its
/// stroke is wider than this (the metadata `min_satin_stroke_width_mm`).
const SATIN_STROKE_MM: f64 = 1.0;
/// A subpath shorter than this is not a rail (0.1 px in Ink/Stitch).
const MIN_RAIL_MM: f64 = 0.1 * MM_PER_PX;
/// The tags of the shapes that Ink/Stitch sews (`EMBROIDERABLE_TAGS`).
const SHAPE_TAGS: &[&str] = &["path", "line", "polyline", "polygon", "rect", "ellipse", "circle"];
/// The prefix of the ids that the reader gives to elements with no id.
const ID_PREFIX: &str = "emb-inkstitch-";

/// All the settings of Ink/Stitch (`INKSTITCH_ATTRIBS` of `lib/svg/tags.py`).
/// A setting that is not in this list is from a newer version: it gives a
/// warning.
const SETTINGS: &[&str] = &[
    "min_stitch_length_mm",
    "min_jump_stitch_length_mm",
    "ties",
    "force_lock_stitches",
    "lock_start",
    "lock_start_scale_mm",
    "lock_start_scale_percent",
    "lock_custom_start",
    "lock_end",
    "lock_end_scale_mm",
    "lock_end_scale_percent",
    "lock_custom_end",
    "clone",
    "flip_angle",
    "polyline",
    "auto_fill",
    "fill",
    "fill_method",
    "angle",
    "contour_strategy",
    "guided_fill_strategy",
    "guided_fill_angle",
    "join_style",
    "avoid_self_crossing",
    "smoothness_mm",
    "clockwise",
    "reverse",
    "meander_pattern",
    "meander_scale_percent",
    "meander_angle",
    "expand_mm",
    "fill_underlay",
    "fill_underlay_angle",
    "fill_underlay_inset_mm",
    "fill_underlay_max_stitch_length_mm",
    "fill_underlay_row_spacing_mm",
    "fill_underlay_skip_last",
    "max_stitch_length_mm",
    "row_spacing_mm",
    "end_row_spacing_mm",
    "skip_last",
    "stitch_position_method",
    "staggers",
    "underlay_underpath",
    "underpath",
    "stop_at_ending_point",
    "flip",
    "clip",
    "rows_per_thread",
    "herringbone_width_mm",
    "tartan_angle",
    "enable_random_stitch_length",
    "random_stitch_length_jitter_percent",
    "gap_fill_rows",
    "pattern_size_mm",
    "fill_coverage",
    "cross_stitch_method",
    "max_cross_stitch_length_mm",
    "cross_offset_mm",
    "cross_rotation",
    "cross_thread_count",
    "canvas_grid_origin",
    "stroke_method",
    "bean_stitch_repeats",
    "repeats",
    "running_stitch_length_mm",
    "running_stitch_tolerance_mm",
    "cutwork_needle",
    "zigzag_width_mm",
    "manual_pattern_placement",
    "flip_copies",
    "line_count",
    "min_line_dist_mm",
    "satin_guide_pattern_position",
    "exponent",
    "flip_exponent",
    "skip_start",
    "skip_end",
    "scale_axis",
    "scale_start",
    "scale_end",
    "rotate_ripples",
    "grid_size_mm",
    "grid_first",
    "satin_column",
    "satin_method",
    "short_stitch_distance_mm",
    "short_stitch_inset",
    "reverse_rails",
    "swap_satin_rails",
    "center_walk_underlay",
    "center_walk_underlay_stitch_length_mm",
    "center_walk_underlay_stitch_tolerance_mm",
    "center_walk_underlay_repeats",
    "center_walk_underlay_position",
    "contour_underlay",
    "contour_underlay_stitch_length_mm",
    "contour_underlay_stitch_tolerance_mm",
    "contour_underlay_inset_mm",
    "contour_underlay_inset_percent",
    "push_compensation_mm",
    "zigzag_underlay",
    "zigzag_spacing_mm",
    "zigzag_underlay_inset_mm",
    "zigzag_underlay_inset_percent",
    "zigzag_underlay_spacing_mm",
    "zigzag_underlay_max_stitch_length_mm",
    "e_stitch",
    "stroke_pull_compensation_mm",
    "zigzag_angle",
    "pull_compensation_mm",
    "pull_compensation_percent",
    "stroke_first",
    "random_width_decrease_percent",
    "random_width_increase_percent",
    "random_zigzag_spacing_percent",
    "split_method",
    "split_staggers",
    "random_split_phase",
    "random_split_jitter_percent",
    "min_random_split_length_mm",
    "running_stitch_position",
    "start_at_nearest_point",
    "end_at_nearest_point",
    "invisible_layers",
    "layer_visibility",
    "ignore_object",
    "trim_after",
    "stop_after",
    "random_seed",
    "manual_stitch",
    "grid_size",
    "sew_stack_only",
    "sew_stack",
    "pattern_interval",
    "pattern_offset",
];

/// The Ink/Stitch defaults of the settings that the program does not read
/// (a value that is not the default gives a warning). A setting that is not
/// in this list has no value as its default.
const DEFAULTS: &[(&str, &str)] = &[
    ("enable_random_stitch_length", "false"),
    ("random_stitch_length_jitter_percent", "10"),
    ("clockwise", "true"),
    ("repeats", "1"),
    ("meander_scale_percent", "100"),
    ("zigzag_width_mm", "3"),
    ("cross_thread_count", "4"),
    ("canvas_grid_origin", "true"),
    ("fill_coverage", "50"),
    ("max_cross_stitch_length_mm", "11"),
    ("split_method", "default"),
    ("split_staggers", "4"),
    ("center_walk_underlay_position", "50"),
    ("flip_copies", "true"),
    ("exponent", "1"),
    ("scale_start", "100"),
    ("rotate_ripples", "true"),
    ("satin_guide_pattern_position", "default"),
    ("staggers", "4"),
];

/// The commands that the program reads or that have no effect on the
/// stitches (`lib/commands.py`).
const COMMANDS: &[&str] = &[
    "starting_point",
    "ending_point",
    "target_point",
    "autoroute_start",
    "autoroute_end",
    "stop",
    "trim",
    "ignore_object",
    "satin_cut_point",
    "ignore_layer",
];

/// Returns `true` if the SVG data has settings of Ink/Stitch: an element or
/// an attribute in its namespace, or an attribute of its first versions
/// (`embroider_`).
pub fn is_inkstitch(data: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(data) else {
        return false;
    };
    if !s.contains(NAMESPACE) && !s.contains("embroider_") {
        return false;
    }
    let Ok(doc) = parse(s) else {
        return false;
    };
    doc.descendants().filter(|n| n.is_element()).any(|n| {
        n.tag_name().namespace() == Some(NAMESPACE)
            || n.attributes().any(|a| {
                a.namespace() == Some(NAMESPACE) || (a.namespace().is_none() && a.name().starts_with("embroider_"))
            })
    })
}

fn parse(s: &str) -> Result<usvg::roxmltree::Document<'_>, usvg::roxmltree::Error> {
    let opt = usvg::roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    usvg::roxmltree::Document::parse_with_options(s, opt)
}

/// Reads an SVG file of Ink/Stitch: its shapes as objects with the stitch
/// types and the settings of Ink/Stitch, in the sew order of the file.
pub fn import_inkstitch(data: &[u8]) -> Result<SvgImport, SvgError> {
    let (file, found, tree) = read_file(data, false)?;
    drop(tree);
    Ok(build(&file, &found).0)
}

/// The glyphs of a font file of Ink/Stitch (a variant file such as
/// `ltr.svg`; ADR 0136).
#[derive(Debug, Clone, Default)]
pub struct GlyphFile {
    /// The glyphs in file order: the name of each one (the label of its
    /// layer after `GlyphLayer-`) and its objects, in mm on the page.
    pub glyphs: Vec<(String, Vec<ImportedObject>)>,
    /// The y of the guide named "baseline", in mm on the page: as measured
    /// from the top of the page, and as measured from the bottom (the files
    /// of Inkscape have the one or the other). `None`: no such guide.
    pub baseline: Option<[f64; 2]>,
    /// The point 0, 0 of the user units of the file, in mm on the page:
    /// the glyphs start at its x, and they stand on its y when the file has
    /// no baseline guide.
    pub origin: [f64; 2],
    /// mm for one user unit of the file.
    pub mm_per_unit: f64,
    pub warnings: Vec<String>,
}

/// Reads a font file of Ink/Stitch: the objects of each glyph layer (also
/// the hidden layers; Ink/Stitch shows each one when it reads it).
pub fn import_inkstitch_glyphs(data: &[u8]) -> Result<GlyphFile, SvgError> {
    let (file, found, tree) = read_file(data, true)?;
    let (import, ids) = build(&file, &found);
    let mut glyphs: Vec<(String, Vec<ImportedObject>)> = Vec::new();
    for (o, id) in import.objects.into_iter().zip(ids) {
        let Some(name) = file.element(&id).and_then(|e| e.glyph.clone()) else {
            continue;
        };
        match glyphs.iter_mut().find(|g| g.0 == name) {
            Some(g) => g.1.push(o),
            None => glyphs.push((name, vec![o])),
        }
    }
    let size = tree.size();
    let (vx, vy, vh, unit) = match file.view_box {
        Some([vx, vy, vw, vh]) if vw > 0.0 => (vx, vy, vh, f64::from(size.width()) / vw * MM_PER_PX),
        _ => (0.0, 0.0, f64::from(size.height()), MM_PER_PX),
    };
    Ok(GlyphFile {
        glyphs,
        baseline: file.baseline.map(|y| [(y - vy) * unit, (vh - y) * unit]),
        origin: [-vx * unit, -vy * unit],
        mm_per_unit: unit,
        warnings: import.warnings,
    })
}

/// Reads the XML and the shapes of a file of Ink/Stitch. `glyphs`: show
/// the hidden glyph layers of a font file.
fn read_file(data: &[u8], glyphs: bool) -> Result<(File, Found, usvg::Tree), SvgError> {
    let s = std::str::from_utf8(data).map_err(|e| SvgError::Parse(e.to_string()))?;
    let doc = parse(s).map_err(|e| SvgError::Parse(e.to_string()))?;
    let mut file = File::read(&doc, s, glyphs);
    let rewritten = file.rewrite(s);
    let tree = usvg::Tree::from_data(rewritten.as_bytes(), &usvg::Options::default())
        .map_err(|e| SvgError::Parse(e.to_string()))?;
    let mut found = Found::default();
    walk(tree.root(), None, &file, &mut found);
    Ok((file, found, tree))
}

/// The prefix of the labels of the glyph layers of a font of Ink/Stitch.
const GLYPH_PREFIX: &str = "GlyphLayer-";
const SODIPODI: &str = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd";

/// The settings and the links of one element of the file.
#[derive(Debug, Default)]
struct Element {
    /// The local name of the tag.
    tag: String,
    /// The name that the user gave (`inkscape:label`).
    label: Option<String>,
    /// The settings: the attributes in the namespace of Ink/Stitch (and the
    /// `embroider_` attributes of its first versions).
    params: BTreeMap<String, String>,
    /// The element has `embroider_` attributes (the first versions).
    legacy: bool,
    /// The marker of Ink/Stitch on the element (`guide-line`, `anchor-line`,
    /// `pattern`): it is not sewn.
    marker: Option<String>,
    /// The id of the parent element.
    parent: Option<String>,
    /// The id of the layer of the element.
    layer: Option<String>,
    /// The commands on the element: the command and the id of its symbol.
    commands: Vec<(String, String)>,
    /// A connector of a command: it is not sewn.
    connector: bool,
    /// The name of the glyph of a font file that the element is in.
    glyph: Option<String>,
}

/// A `use` element that is a copy of a shape or a group (a clone).
#[derive(Debug, Default)]
struct CloneUse {
    /// The ids of the shapes of the copy, in order: usvg gives the copies
    /// no id.
    shapes: Vec<String>,
    /// The fill angle of the clone (`angle`), if it has one.
    angle: Option<f64>,
    /// The clone turns the fill angle the other way (`flip_angle`).
    flip: bool,
    /// The clone is sewn (`clone`, default true).
    sews: bool,
}

/// What the reader found in the XML of the file.
#[derive(Debug, Default)]
struct File {
    /// The SVG version of Ink/Stitch (`inkstitch_svg_version`, 0 if none).
    version: u32,
    /// The thread palette of the file (`thread-palette`).
    palette: Option<String>,
    /// The stroke width above which a satin column with one subpath is a satin.
    satin_stroke_mm: f64,
    elements: HashMap<String, Element>,
    /// The symbols of commands: id to command.
    commands: HashMap<String, String>,
    clones: HashMap<String, CloneUse>,
    /// The layers with the command `ignore_layer`.
    ignored_layers: HashSet<String>,
    /// Changes of the text before `usvg` reads it.
    edits: Vec<(Range<usize>, String)>,
    warnings: Warnings,
    texts: usize,
    /// The `viewBox` of the root element.
    view_box: Option<[f64; 4]>,
    /// The y of the guide named "baseline", in user units, as written.
    baseline: Option<f64>,
}

impl File {
    fn read(doc: &usvg::roxmltree::Document, s: &str, glyphs: bool) -> File {
        let mut f = File {
            satin_stroke_mm: SATIN_STROKE_MM,
            ..File::default()
        };
        let existing: HashSet<&str> = doc
            .descendants()
            .filter_map(|n| n.attribute("id"))
            .filter(|id| !id.is_empty())
            .collect();
        // An id for each element.
        let mut ids: HashMap<usvg::roxmltree::NodeId, String> = HashMap::new();
        let mut k = 0;
        for n in doc.descendants().filter(|n| n.is_element()) {
            if let Some(id) = n.attribute("id").filter(|id| !id.is_empty()) {
                ids.insert(n.id(), id.to_string());
                continue;
            }
            let id = loop {
                k += 1;
                let id = format!("{ID_PREFIX}{k}");
                if !existing.contains(id.as_str()) {
                    break id;
                }
            };
            match n.attribute_node("id") {
                // An empty id gets a value.
                Some(a) => f.edits.push((a.range_value(), id.clone())),
                None => {
                    let start = n.range().start;
                    let rest = &s[start + 1..];
                    let len = rest
                        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                        .unwrap_or(rest.len());
                    let at = start + 1 + len;
                    f.edits.push((at..at, format!(" id=\"{id}\"")));
                }
            }
            ids.insert(n.id(), id);
        }
        let id_of = |n: usvg::roxmltree::Node| ids.get(&n.id()).cloned().unwrap_or_default();
        for n in doc.descendants().filter(|n| n.is_element()) {
            let tag = n.tag_name();
            if tag.namespace() == Some(NAMESPACE) {
                let value = n.text().map(json_text).unwrap_or_default();
                match tag.name() {
                    "inkstitch_svg_version" => {
                        f.version = value.trim().parse::<f64>().map_or(0, |v| v.max(0.0) as u32);
                    }
                    "thread-palette" if !value.is_empty() => f.palette = Some(value),
                    "min_satin_stroke_width_mm" => {
                        if let Ok(v) = value.trim().parse::<f64>() {
                            f.satin_stroke_mm = v;
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if tag.name() == "text" {
                f.texts += 1;
            }
            if tag.name() == "svg" && n.parent_element().is_none() {
                let v: Vec<f64> = n
                    .attribute("viewBox")
                    .unwrap_or("")
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter_map(|x| x.parse().ok())
                    .collect();
                if let [x, y, w, h] = v[..] {
                    f.view_box = Some([x, y, w, h]);
                }
            }
            if tag.namespace() == Some(SODIPODI)
                && tag.name() == "guide"
                && n.attribute((INKSCAPE, "label")) == Some("baseline")
            {
                f.baseline = n
                    .attribute("position")
                    .and_then(|p| p.split(',').nth(1))
                    .and_then(|y| y.trim().parse().ok());
            }
            let glyph_label = |a: usvg::roxmltree::Node| {
                a.attribute((INKSCAPE, "label"))
                    .and_then(|l| l.strip_prefix(GLYPH_PREFIX))
                    .filter(|_| a.tag_name().name() == "g")
                    .map(str::to_string)
            };
            // A hidden glyph layer is shown, as Ink/Stitch does.
            if glyphs && glyph_label(n).is_some() {
                if let Some(a) = n.attribute_node("style")
                    && a.value().contains("display:none")
                {
                    f.edits
                        .push((a.range_value(), a.value().replace("display:none", "display:inline")));
                }
                if let Some(a) = n.attribute_node("display")
                    && a.value().trim() == "none"
                {
                    f.edits.push((a.range_value(), "inline".into()));
                }
            }
            let mut el = Element {
                glyph: n.ancestors().find_map(glyph_label),
                tag: tag.name().to_string(),
                label: n
                    .attribute((INKSCAPE, "label"))
                    .map(str::to_string)
                    .filter(|l| !l.is_empty()),
                parent: n.parent_element().map(id_of),
                layer: n
                    .ancestors()
                    .skip(1)
                    .find(|a| a.is_element() && a.attribute((INKSCAPE, "groupmode")) == Some("layer"))
                    .map(id_of),
                connector: n.has_attribute((INKSCAPE, "connection-start"))
                    || n.has_attribute((INKSCAPE, "connection-end"))
                    || n.has_attribute((INKSCAPE, "connector-type")),
                ..Element::default()
            };
            for a in n.attributes() {
                if a.namespace() == Some(NAMESPACE) {
                    el.params.insert(a.name().to_string(), a.value().to_string());
                } else if a.namespace().is_none()
                    && let Some(name) = a.name().strip_prefix("embroider_")
                {
                    el.legacy = true;
                    el.params
                        .entry(name.to_string())
                        .or_insert_with(|| a.value().to_string());
                }
            }
            let style = n.attribute("style").unwrap_or("");
            el.marker = marker_of(style).or_else(|| n.attribute("marker-start").and_then(marker_of));
            let id = id_of(n);
            // Commands and clones are `use` elements.
            if tag.name() == "use" {
                let href = n
                    .attribute((XLINK, "href"))
                    .or_else(|| n.attribute("href"))
                    .unwrap_or("");
                if let Some(target) = href.strip_prefix('#') {
                    if let Some(name) = target.strip_prefix("inkstitch_") {
                        let name = name.split('-').next().unwrap_or(name);
                        f.commands.insert(id.clone(), command_name(name).to_string());
                        // Put a small shape at the point of the symbol, so
                        // that usvg gives its place in the design.
                        let num = |a: &str| n.attribute(a).and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
                        let (x, y) = (num("x"), num("y"));
                        let transform = n
                            .attribute("transform")
                            .map(|t| format!(" transform=\"{}\"", xml_escape(t)))
                            .unwrap_or_default();
                        f.edits.push((
                            n.range(),
                            format!(
                                "<path id=\"{}\" d=\"M {x} {y} l 0.01 0 l 0 0.01 z\" \
                                 style=\"display:inline;fill:#000000;stroke:none\"{transform}/>",
                                xml_escape(&id)
                            ),
                        ));
                    } else {
                        let p = Params::new(&el.params);
                        let shapes = doc
                            .descendants()
                            .find(|s| s.is_element() && s.attribute("id") == Some(target))
                            .map(|source| {
                                source
                                    .descendants()
                                    .filter(|s| s.is_element() && SHAPE_TAGS.contains(&s.tag_name().name()))
                                    .map(id_of)
                                    .collect()
                            })
                            .unwrap_or_default();
                        f.clones.insert(
                            id.clone(),
                            CloneUse {
                                shapes,
                                angle: p.float("angle"),
                                flip: p.bool("flip_angle") == Some(true),
                                sews: p.bool("clone") != Some(false),
                            },
                        );
                    }
                }
            }
            f.elements.insert(id, el);
        }
        // The connectors link the symbols of commands to the shapes.
        let mut linked: HashSet<String> = HashSet::new();
        let links: Vec<(String, String)> = doc
            .descendants()
            .filter(|n| n.is_element())
            .filter_map(|n| {
                let start = n.attribute((INKSCAPE, "connection-start"))?.trim_start_matches('#');
                let end = n.attribute((INKSCAPE, "connection-end"))?.trim_start_matches('#');
                if f.commands.contains_key(start) {
                    Some((start.to_string(), end.to_string()))
                } else if f.commands.contains_key(end) {
                    Some((end.to_string(), start.to_string()))
                } else {
                    None
                }
            })
            .collect();
        for (symbol, target) in links {
            if let (Some(cmd), Some(el)) = (f.commands.get(&symbol), f.elements.get_mut(&target)) {
                el.commands.push((cmd.clone(), symbol.clone()));
                linked.insert(symbol);
            }
        }
        // Commands with no connector: layer and global commands.
        let mut standalone: Vec<(&String, &String)> =
            f.commands.iter().filter(|(id, _)| !linked.contains(*id)).collect();
        standalone.sort();
        let mut ignored = Vec::new();
        for (id, cmd) in standalone {
            match cmd.as_str() {
                "ignore_layer" => {
                    if let Some(layer) = f.elements.get(id).and_then(|e| e.layer.clone()) {
                        ignored.push(layer);
                    }
                }
                "origin" | "stop_position" => f.warnings.add(Warning::Command(cmd.clone())),
                _ => {}
            }
        }
        f.ignored_layers.extend(ignored);
        f
    }

    /// The text of the file with an id on each element and a small shape in
    /// place of each symbol of a command.
    fn rewrite(&mut self, s: &str) -> String {
        let mut edits = std::mem::take(&mut self.edits);
        // An edit inside a symbol that the reader replaces is not made.
        let replaced: Vec<Range<usize>> = edits
            .iter()
            .filter(|(_, t)| t.starts_with('<'))
            .map(|(r, _)| r.clone())
            .collect();
        edits.retain(|(r, t)| t.starts_with('<') || !replaced.iter().any(|x| x.start <= r.start && r.end <= x.end));
        edits.sort_by(|a, b| b.0.start.cmp(&a.0.start).then(b.0.end.cmp(&a.0.end)));
        let mut out = s.to_string();
        for (r, t) in edits {
            out.replace_range(r, &t);
        }
        out
    }

    fn element(&self, id: &str) -> Option<&Element> {
        self.elements.get(id)
    }
}

/// The marker of Ink/Stitch in a style or a `marker-start` value.
fn marker_of(style: &str) -> Option<String> {
    let at = style.find("url(#inkstitch-")?;
    let rest = &style[at + "url(#inkstitch-".len()..];
    let end = rest.find("-marker")?;
    Some(rest[..end].to_string())
}

/// The command of the name of a symbol, with the names of the first
/// versions changed (`lib/update.py`, version 3).
fn command_name(name: &str) -> &str {
    match name {
        "fill_start" => "starting_point",
        "fill_end" => "ending_point",
        "satin_start" | "run_start" => "autoroute_start",
        "satin_end" | "run_end" => "autoroute_end",
        "ripple_target" => "target_point",
        n => n,
    }
}

/// The value of a JSON string or number in metadata text.
fn json_text(t: &str) -> String {
    let t = t.trim();
    match t.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => t.to_string(),
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;")
}

/// The paint of a fill or a stroke.
#[derive(Debug, Clone)]
enum Paint {
    Color(Rgb),
    /// A linear gradient: the stops (place, color) and the line, in mm.
    Linear {
        stops: Vec<(f64, Rgb)>,
        line: (Point, Point),
    },
    /// Another paint, with the color that Ink/Stitch uses for it.
    Other(Rgb),
}

impl Paint {
    /// The thread color: the color, or the first color of a gradient.
    fn color(&self) -> Rgb {
        match self {
            Paint::Color(c) | Paint::Other(c) => *c,
            Paint::Linear { stops, .. } => stops.first().map_or(FALLBACK, |s| s.1),
        }
    }
}

/// A shape of the file, as usvg gives it.
#[derive(Debug)]
struct Shape {
    id: String,
    /// The subpaths, in mm.
    subs: Vec<Path>,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
    /// The turn of the shape on the page, in degrees (clockwise on screen).
    rotation: f64,
    /// The id of the `use` element if the shape is a clone.
    clone: Option<String>,
}

#[derive(Debug)]
struct Stroke {
    paint: Paint,
    width_mm: f64,
    dashed: bool,
}

#[derive(Debug, Default)]
struct Found {
    shapes: Vec<Shape>,
    /// The points of the symbols of commands, in mm.
    points: HashMap<String, Point>,
    /// The lines with a marker, by the id of their parent: the marker and
    /// the points.
    marked: HashMap<String, Vec<(String, Vec<Point>)>>,
    images: usize,
}

fn rgb(c: usvg::Color) -> Rgb {
    Rgb(c.red, c.green, c.blue)
}

fn paint_of(paint: &usvg::Paint, t: usvg::Transform) -> Paint {
    let map = |x: f32, y: f32, g: usvg::Transform| {
        let mut p = usvg::tiny_skia_path::Point::from_xy(x, y);
        g.map_point(&mut p);
        t.map_point(&mut p);
        Point::new(f64::from(p.x) * MM_PER_PX, f64::from(p.y) * MM_PER_PX)
    };
    match paint {
        usvg::Paint::Color(c) => Paint::Color(rgb(*c)),
        usvg::Paint::LinearGradient(g) => {
            let stops: Vec<(f64, Rgb)> = g
                .stops()
                .iter()
                .map(|s| (f64::from(s.offset().get()), rgb(s.color())))
                .collect();
            if stops.is_empty() {
                return Paint::Other(FALLBACK);
            }
            let line = (map(g.x1(), g.y1(), g.transform()), map(g.x2(), g.y2(), g.transform()));
            Paint::Linear { stops, line }
        }
        usvg::Paint::RadialGradient(g) => Paint::Other(g.stops().first().map_or(FALLBACK, |s| rgb(s.color()))),
        usvg::Paint::Pattern(_) => Paint::Other(FALLBACK),
    }
}

/// A clone that the walk is in: the id of its `use` element and the
/// number of its shapes that the walk found.
struct InClone<'a> {
    id: &'a str,
    next: std::cell::Cell<usize>,
}

fn walk(group: &usvg::Group, clone: Option<&InClone>, file: &File, found: &mut Found) {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => {
                if file.clones.contains_key(g.id()) {
                    let inner = InClone {
                        id: g.id(),
                        next: std::cell::Cell::new(0),
                    };
                    walk(g, Some(&inner), file, found);
                } else {
                    walk(g, clone, file, found);
                }
            }
            usvg::Node::Path(p) => {
                let mut id = p.id();
                // In a clone, the shapes have no id: they are the shapes of
                // the source in order.
                if id.is_empty()
                    && let Some(c) = clone
                    && let Some(source) = file.clones.get(c.id).and_then(|u| u.shapes.get(c.next.get()))
                {
                    c.next.set(c.next.get() + 1);
                    id = source;
                }
                // Other paths with no id are the parts of markers.
                if id.is_empty() {
                    continue;
                }
                let subs = subpaths(p);
                if file.commands.contains_key(id) {
                    if let Some(PathCmd::MoveTo(a)) = subs.first().and_then(|s| s.cmds.first()) {
                        found.points.insert(id.to_string(), *a);
                    }
                    continue;
                }
                let Some(el) = file.element(id) else { continue };
                if el.connector {
                    continue;
                }
                if let Some(m) = &el.marker {
                    if clone.is_none()
                        && let Some(parent) = &el.parent
                    {
                        let pts: Vec<Point> = subs
                            .iter()
                            .flat_map(|s| s.flatten(FLAT_MM))
                            .flat_map(|pl| pl.points)
                            .collect();
                        found.marked.entry(parent.clone()).or_default().push((m.clone(), pts));
                    }
                    continue;
                }
                if !p.is_visible() || subs.is_empty() {
                    continue;
                }
                let t = p.abs_transform();
                let fill = p
                    .fill()
                    .filter(|f| f.opacity().get() > 0.0)
                    .map(|f| paint_of(f.paint(), t));
                let stroke = p.stroke().map(|s| {
                    let scale = f64::from(t.sx * t.sy - t.kx * t.ky).abs().sqrt();
                    Stroke {
                        paint: paint_of(s.paint(), t),
                        width_mm: f64::from(s.width().get()) * scale * MM_PER_PX,
                        dashed: s.dasharray().is_some(),
                    }
                });
                found.shapes.push(Shape {
                    id: id.to_string(),
                    subs,
                    fill,
                    stroke,
                    rotation: math::to_degrees(math::atan2(f64::from(t.ky), f64::from(t.sx))),
                    clone: clone.map(|c| c.id.to_string()),
                });
            }
            usvg::Node::Image(_) => found.images += 1,
            usvg::Node::Text(_) => {}
        }
    }
}

/// The settings of one element. It keeps the names that the reader asked
/// for, so the others can give warnings.
struct Params<'a> {
    map: &'a BTreeMap<String, String>,
    used: RefCell<BTreeSet<String>>,
}

impl<'a> Params<'a> {
    fn new(map: &'a BTreeMap<String, String>) -> Params<'a> {
        Params {
            map,
            used: RefCell::new(BTreeSet::new()),
        }
    }

    /// The value of a setting; `None` if it is not there or empty.
    fn str(&self, name: &str) -> Option<&'a str> {
        self.used.borrow_mut().insert(name.to_string());
        self.map.get(name).map(|v| v.trim()).filter(|v| !v.is_empty())
    }

    fn float(&self, name: &str) -> Option<f64> {
        self.str(name)?.parse::<f64>().ok().filter(|v| v.is_finite())
    }

    /// The numbers of a setting with one or more values.
    fn floats(&self, name: &str) -> Vec<f64> {
        self.str(name)
            .map(|v| {
                v.split(|c: char| c.is_whitespace() || c == ',')
                    .filter_map(|x| x.parse::<f64>().ok())
                    .filter(|x| x.is_finite())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn int(&self, name: &str) -> Option<i64> {
        let v = self.str(name)?;
        v.parse::<i64>()
            .ok()
            .or_else(|| v.parse::<f64>().ok().filter(|f| f.is_finite()).map(|f| f as i64))
    }

    fn bool(&self, name: &str) -> Option<bool> {
        self.str(name)
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "yes" | "y" | "true" | "t" | "1"))
    }

    /// Settings that the program does not read here: each one with a value
    /// that is not the default of Ink/Stitch gives a warning.
    fn not_read(&self, names: &[&str], warnings: &mut Warnings) {
        for name in names {
            if let Some(v) = self.str(name)
                && !is_default(name, v)
            {
                warnings.add(Warning::Setting((*name).to_string()));
            }
        }
    }
}

/// `true` if `value` is the default of the setting `name` (see [`DEFAULTS`]).
fn is_default(name: &str, value: &str) -> bool {
    let d = DEFAULTS.iter().find(|(n, _)| *n == name).map_or("", |(_, d)| d);
    let truth = |s: &str| match s.to_ascii_lowercase().as_str() {
        "yes" | "y" | "true" | "t" => Some(true),
        "no" | "n" | "false" | "f" | "none" => Some(false),
        _ => None,
    };
    let v = value.trim();
    if d.is_empty() {
        return v.is_empty() || truth(v) == Some(false) || v.parse::<f64>().is_ok_and(|x| x == 0.0);
    }
    if let (Ok(a), Ok(b)) = (v.parse::<f64>(), d.parse::<f64>()) {
        return (a - b).abs() < 1e-9;
    }
    if let (Some(a), Some(b)) = (truth(v), truth(d)) {
        return a == b;
    }
    v.eq_ignore_ascii_case(d)
}

/// What the reader could not read, with the number of times.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Warning {
    /// A setting with a value that the program does not read.
    Setting(String),
    /// A fill type that the program sews as tatami.
    FillType(String),
    /// A satin type that the program sews as satin.
    SatinType(String),
    /// A command that the program does not read.
    Command(String),
    /// A shape that Ink/Stitch does not sew (a vector object).
    Ignored,
    /// A satin column with one rail and a thin stroke.
    OneRail,
}

#[derive(Debug, Default)]
struct Warnings(BTreeMap<Warning, usize>);

impl Warnings {
    fn add(&mut self, w: Warning) {
        *self.0.entry(w).or_default() += 1;
    }

    fn texts(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(w, n)| match w {
                Warning::Setting(s) => text::fill("The Ink/Stitch setting \"{}\" is not read ({} object(s)).", &[s, n]),
                Warning::FillType(s) => text::fill(
                    "The Ink/Stitch fill type \"{}\" is sewn as tatami ({} object(s)).",
                    &[s, n],
                ),
                Warning::SatinType(s) => text::fill(
                    "The Ink/Stitch satin type \"{}\" is sewn as satin ({} object(s)).",
                    &[s, n],
                ),
                Warning::Command(s) => text::fill("The Ink/Stitch command \"{}\" is not read ({} time(s)).", &[s, n]),
                Warning::Ignored => text::fill(
                    "{} shape(s) that Ink/Stitch does not sew are vector objects with no stitches.",
                    &[n],
                ),
                Warning::OneRail => text::fill(
                    "{} satin column(s) with one line and a thin stroke are running lines.",
                    &[n],
                ),
            })
            .collect()
    }
}

/// The stitch types of Ink/Stitch for a shape (`lib/elements/utils/nodes.py`).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Type {
    Fill,
    Satin,
    Stroke,
}

/// The objects of the shapes, in order.
fn build(file: &File, found: &Found) -> (SvgImport, Vec<String>) {
    let mut warnings = Warnings(file.warnings.0.clone());
    let mut ids: Vec<String> = Vec::new();
    let mut out = SvgImport {
        chart: file.palette.clone(),
        ..SvgImport::default()
    };
    // The turn of each shape that is not a clone, for the fill angle of its clones.
    let mut turns: HashMap<&str, f64> = HashMap::new();
    for s in found.shapes.iter().filter(|s| s.clone.is_none()) {
        turns.entry(s.id.as_str()).or_insert(s.rotation);
    }
    let mut group = 0;
    for shape in &found.shapes {
        let Some(el) = file.element(&shape.id) else { continue };
        let clone = shape.clone.as_ref().and_then(|c| file.clones.get(c));
        let p = Params::new(&el.params);
        let has = |cmd: &str| el.commands.iter().any(|(c, _)| c == cmd);
        let name = el
            .label
            .clone()
            .or_else(|| (!shape.id.starts_with(ID_PREFIX)).then(|| shape.id.clone()));
        let ignored = p.bool("ignore_object") == Some(true)
            || has("ignore_object")
            || el.layer.as_ref().is_some_and(|l| file.ignored_layers.contains(l))
            || clone.is_some_and(|c| !c.sews);
        if ignored {
            let style = VectorStyle {
                fill: shape.fill.as_ref().map(Paint::color),
                stroke: shape.stroke.as_ref().map(|s| s.paint.color()),
                stroke_mm: shape
                    .stroke
                    .as_ref()
                    .map_or(emb_model::vector::DEFAULT_STROKE_MM, |s| s.width_mm),
            };
            out.objects.push(ImportedObject::new(
                ObjectKind::Vector {
                    path: Path::joined(&shape.subs),
                    style,
                },
                style.fill.or(style.stroke).unwrap_or(FALLBACK),
                name,
            ));
            ids.push(shape.id.clone());
            warnings.add(Warning::Ignored);
            continue;
        }
        let mut types = Vec::new();
        if shape.fill.is_some() {
            types.push(Type::Fill);
        }
        if let Some(s) = &shape.stroke {
            if p.bool("satin_column") == Some(true) {
                if shape.subs.len() > 1 || s.width_mm > file.satin_stroke_mm {
                    types.push(Type::Satin);
                } else {
                    warnings.add(Warning::OneRail);
                    types.push(Type::Stroke);
                }
            } else {
                types.push(Type::Stroke);
            }
        }
        if p.bool("stroke_first") == Some(true) {
            types.reverse();
        }
        let ctx = Ctx {
            file,
            found,
            el,
            shape,
            p: &p,
            clone,
            turn: shape.rotation - turns.get(shape.id.as_str()).copied().unwrap_or(0.0),
        };
        let mut objects = Vec::new();
        for t in types {
            objects.extend(match t {
                Type::Fill => ctx.fills(&mut warnings),
                Type::Satin => ctx.satin(&mut warnings),
                Type::Stroke => ctx.strokes(&mut warnings),
            });
        }
        if objects.is_empty() {
            continue;
        }
        for o in &mut objects {
            o.name.clone_from(&name);
            o.functions = ctx.functions();
        }
        if objects.len() > 1 {
            group += 1;
            for o in &mut objects {
                o.group = Some(group);
            }
        }
        for (cmd, symbol) in &el.commands {
            let point = found.points.get(symbol).copied();
            match cmd.as_str() {
                "starting_point" => objects[0].start = point,
                "ending_point" => {
                    if let Some(o) = objects.last_mut() {
                        o.end = point;
                    }
                }
                "trim" => {
                    if let Some(o) = objects.last_mut() {
                        o.functions.trim = TrimMode::Always;
                    }
                }
                "stop" => {
                    if let Some(o) = objects.last_mut() {
                        o.functions.stop = true;
                    }
                }
                c if COMMANDS.contains(&c) => {}
                c => warnings.add(Warning::Command(c.to_string())),
            }
        }
        // Settings of a newer version of Ink/Stitch.
        for name in el.params.keys() {
            if !SETTINGS.contains(&name.as_str()) && !is_default(name, &el.params[name]) {
                warnings.add(Warning::Setting(name.clone()));
            }
        }
        ids.extend(std::iter::repeat_n(shape.id.clone(), objects.len()));
        out.objects.extend(objects);
    }
    for list in found.marked.values() {
        for (m, _) in list {
            if m == "pattern" {
                warnings.add(Warning::Setting("pattern".into()));
            }
        }
    }
    out.warnings = warnings.texts();
    if file.texts > 0 {
        out.warnings.push(text::fill(
            "{} text element(s) not imported. Convert text to paths first.",
            &[&file.texts],
        ));
    }
    if found.images > 0 {
        out.warnings
            .push(text::fill("{} image(s) not imported.", &[&found.images]));
    }
    (out, ids)
}

/// What the builders of the objects of one shape need.
struct Ctx<'a> {
    file: &'a File,
    found: &'a Found,
    el: &'a Element,
    shape: &'a Shape,
    p: &'a Params<'a>,
    clone: Option<&'a CloneUse>,
    /// The turn of a clone against the shape that it copies, in degrees.
    turn: f64,
}

impl Ctx<'_> {
    fn version(&self) -> u32 {
        self.file.version
    }

    /// The point of the command `cmd` on the shape.
    fn command_point(&self, cmd: &str) -> Option<Point> {
        self.el
            .commands
            .iter()
            .find(|(c, _)| c == cmd)
            .and_then(|(_, s)| self.found.points.get(s).copied())
    }

    /// The guide lines of the shape: the lines with the guide marker in its group.
    fn guides(&self) -> Vec<Vec<Point>> {
        self.el
            .parent
            .as_ref()
            .and_then(|p| self.found.marked.get(p))
            .map(|v| {
                v.iter()
                    .filter(|(m, pts)| m == "guide-line" && pts.len() > 1)
                    .map(|(_, pts)| pts.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The machine functions of the settings `ties`, `trim_after` and `stop_after`.
    fn functions(&self) -> Functions {
        let p = self.p;
        let ties = match p.str("ties").map(str::to_ascii_lowercase).as_deref() {
            Some("true") => 0,
            Some("false") => 3,
            Some(_) => p.int("ties").unwrap_or(0),
            None => 0,
        };
        Functions {
            tie_in: matches!(ties, 0 | 1),
            tie_off: matches!(ties, 0 | 2),
            trim: if p.bool("trim_after") == Some(true) {
                TrimMode::Always
            } else {
                TrimMode::Auto
            },
            stop: p.bool("stop_after") == Some(true),
            slow: false,
        }
    }

    /// The fill angle of Ink/Stitch in degrees (counterclockwise on screen),
    /// with the turn of a clone.
    fn fill_angle(&self) -> f64 {
        let p = self.p;
        let mut a = p.float("angle").unwrap_or(0.0);
        if let Some(c) = self.clone {
            a = match c.angle {
                Some(ca) => ca,
                None => a - self.turn,
            };
            if c.flip {
                a = -a;
            }
        }
        a
    }

    fn fills(&self, warnings: &mut Warnings) -> Vec<ImportedObject> {
        let p = self.p;
        let Some(paint) = &self.shape.fill else {
            return Vec::new();
        };
        let legacy = self.version() < 1 && self.el.legacy;
        let mut method = p.str("fill_method").unwrap_or("tatami_fill").to_string();
        method = match method.as_str() {
            "0" | "auto_fill" => "tatami_fill".into(),
            "1" => "contour_fill".into(),
            "2" => "guided_fill".into(),
            "3" => "legacy_fill".into(),
            _ => method,
        };
        if self.version() < 4 && p.bool("auto_fill") == Some(false) {
            method = "legacy_fill".into();
        }
        let angle_ink = self.fill_angle();
        let spacing = p.float("row_spacing_mm").filter(|v| *v > 0.0).unwrap_or(0.25).max(0.1);
        let default_len = if self.version() < 4 { 3.0 } else { 4.0 };
        let max_len = p
            .float("max_stitch_length_mm")
            .filter(|v| *v > 0.0)
            .unwrap_or(default_len)
            .max(0.1);
        let run_len = p
            .floats("running_stitch_length_mm")
            .first()
            .copied()
            .unwrap_or(2.5)
            .max(0.1);
        // The underlay (default on, but off in the files of the first
        // versions that do not set it).
        let underlay_on = p.bool("fill_underlay").unwrap_or(!legacy);
        let underlay = if underlay_on {
            let angles = {
                let v = p.floats("fill_underlay_angle");
                if v.is_empty() { vec![angle_ink + 90.0] } else { v }
            };
            let ul_spacing = p
                .float("fill_underlay_row_spacing_mm")
                .filter(|v| *v > 0.0)
                .unwrap_or(spacing * 3.0);
            let inset = p.float("fill_underlay_inset_mm").unwrap_or(0.0);
            Underlay::Custom(
                angles
                    .iter()
                    .map(|a| UnderlayLayer::Tatami {
                        inset_mm: inset,
                        spacing_mm: ul_spacing,
                        // Relative to the top rows, clockwise on screen.
                        angle_deg: norm_deg(angle_ink - a),
                    })
                    .collect(),
            )
        } else {
            Underlay::None
        };
        let pull = p.floats("pull_compensation_mm");
        if pull.len() > 1 && (pull[0] - pull[1]).abs() > 1e-9 {
            warnings.add(Warning::Setting("pull_compensation_mm".into()));
        }
        let base = FillParams {
            angle_deg: norm_deg(-angle_ink),
            spacing_mm: spacing,
            stitch_length_mm: max_len,
            pull_compensation_mm: pull.first().copied().unwrap_or(0.0).max(0.0),
            underlay,
            ..FillParams::default()
        };
        let staggers = p.float("staggers").unwrap_or(4.0);
        let stagger_note = |w: &mut Warnings| {
            if (staggers - 4.0).abs() > 1e-9 && (staggers - 1.0).abs() > 1e-9 {
                w.add(Warning::Setting("staggers".into()));
            }
        };
        let mut gradient = None;
        let params = match method.as_str() {
            "tatami_fill" | "legacy_fill" => {
                stagger_note(warnings);
                p.not_read(
                    &[
                        "gap_fill_rows",
                        "enable_random_stitch_length",
                        "pull_compensation_percent",
                    ],
                    warnings,
                );
                let pattern = if (staggers - 1.0).abs() < 1e-9 {
                    FillPattern::Square
                } else {
                    FillPattern::Tatami
                };
                let mut fp = base.with_pattern(pattern);
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = max_len;
                fp.accordion = accordion(spacing, p.float("end_row_spacing_mm"), &mut fp.spacing_mm);
                fp
            }
            "contour_fill" => {
                p.not_read(
                    &[
                        "join_style",
                        "avoid_self_crossing",
                        "clockwise",
                        "smoothness_mm",
                        "repeats",
                        "bean_stitch_repeats",
                    ],
                    warnings,
                );
                let pattern = match p.int("contour_strategy").unwrap_or(0) {
                    1 => FillPattern::Spiral,
                    2 => FillPattern::Maze,
                    _ => FillPattern::Offset,
                };
                let mut fp = base.with_pattern(pattern);
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = max_len;
                fp
            }
            "guided_fill" => {
                let guides = self.guides();
                stagger_note(warnings);
                p.not_read(
                    &[
                        "guided_fill_strategy",
                        "guided_fill_angle",
                        "stitch_position_method",
                        "smoothness_mm",
                    ],
                    warnings,
                );
                let pattern = if guides.is_empty() {
                    FillPattern::Tatami
                } else {
                    FillPattern::Guided { guides }
                };
                let mut fp = base.with_pattern(pattern);
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = max_len;
                fp
            }
            "circular_fill" => {
                p.not_read(
                    &["repeats", "bean_stitch_repeats", "enable_random_stitch_length"],
                    warnings,
                );
                let mut fp = base.with_pattern(FillPattern::Circular {
                    center: self.command_point("target_point"),
                });
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = run_len;
                fp
            }
            "meander_fill" => {
                warnings.add(Warning::Setting("meander_pattern".into()));
                p.not_read(
                    &[
                        "meander_scale_percent",
                        "meander_angle",
                        "clip",
                        "smoothness_mm",
                        "zigzag_spacing_mm",
                        "repeats",
                        "bean_stitch_repeats",
                    ],
                    warnings,
                );
                base.with_pattern(FillPattern::Meander {
                    tile: MeanderTile::Square,
                })
            }
            "cross_stitch" => {
                p.not_read(
                    &[
                        "cross_thread_count",
                        "canvas_grid_origin",
                        "cross_offset_mm",
                        "cross_rotation",
                        "fill_coverage",
                        "max_cross_stitch_length_mm",
                    ],
                    warnings,
                );
                let style = match p.str("cross_stitch_method").unwrap_or("simple_cross") {
                    m if m.starts_with("half_cross") => CrossStyle::Diagonal,
                    m if m.starts_with("upright_cross") => CrossStyle::Upright,
                    m if m.starts_with("simple_cross") => CrossStyle::Full,
                    m => {
                        warnings.add(Warning::Setting(format!("cross_stitch_method={m}")));
                        CrossStyle::Full
                    }
                };
                let cell = p
                    .floats("pattern_size_mm")
                    .first()
                    .copied()
                    .filter(|v| *v > 0.0)
                    .unwrap_or(3.0);
                base.with_pattern(FillPattern::CrossStitch {
                    cell_mm: cell,
                    style,
                    passes: 1,
                })
            }
            "linear_gradient_fill" => {
                stagger_note(warnings);
                let mut fp = base.with_pattern(FillPattern::Tatami);
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = max_len;
                if let Paint::Linear { stops, line } = paint
                    && stops.len() > 1
                {
                    let d = line.1 - line.0;
                    if d.length() > 1e-9 {
                        // The rows go across the line of the gradient.
                        fp.angle_deg = norm_deg(math::to_degrees(math::atan2(d.y, d.x)) + 90.0);
                    }
                    gradient = Some((stops.clone(), *line));
                }
                fp
            }
            m => {
                warnings.add(Warning::FillType(m.to_string()));
                let mut fp = base.with_pattern(FillPattern::Tatami);
                fp.spacing_mm = spacing;
                fp.stitch_length_mm = max_len;
                fp
            }
        };
        // The outlines: one fill for each outer shape, with its holes.
        let mut outlines = region::nest(&self.shape.subs);
        if let Some(d) = p.float("expand_mm").filter(|d| *d != 0.0) {
            let regions: Vec<region::Region> = outlines
                .iter()
                .filter_map(|(o, hs)| {
                    let outer = o.flatten(FLAT_MM).into_iter().next()?;
                    let holes: Vec<_> = hs
                        .iter()
                        .filter_map(|h| h.flatten(FLAT_MM).into_iter().next())
                        .collect();
                    Some(region::Region::from_polylines(&outer, &holes))
                })
                .collect();
            let grown = region::offset(&regions, d, region::Join::Round);
            if !grown.is_empty() {
                outlines = grown
                    .iter()
                    .map(|r| {
                        (
                            Path::polygon(&r.outer),
                            r.holes.iter().map(|h| Path::polygon(h)).collect(),
                        )
                    })
                    .collect();
            }
        }
        let mut out = Vec::new();
        for (outer, holes) in outlines {
            let outer = outer.closed();
            let holes: Vec<Path> = holes.into_iter().map(Path::closed).collect();
            match &gradient {
                Some((stops, line)) => {
                    let places: Vec<f64> = stops.iter().map(|s| s.0).collect();
                    for (fp, (_, color)) in emb_model::tartan::gradient_layers(&params, Some(*line), &places)
                        .into_iter()
                        .zip(stops)
                    {
                        out.push(ImportedObject::new(
                            ObjectKind::Fill {
                                outer: outer.clone(),
                                holes: holes.clone(),
                                params: fp,
                            },
                            *color,
                            None,
                        ));
                    }
                }
                None => out.push(ImportedObject::new(
                    ObjectKind::Fill {
                        outer,
                        holes,
                        params: params.clone(),
                    },
                    paint.color(),
                    None,
                )),
            }
        }
        out
    }

    fn satin(&self, warnings: &mut Warnings) -> Vec<ImportedObject> {
        let p = self.p;
        let Some(stroke) = &self.shape.stroke else {
            return Vec::new();
        };
        let Some(geometry) = self.satin_geometry(stroke.width_mm) else {
            warnings.add(Warning::OneRail);
            return self.strokes(warnings);
        };
        let mut pattern = ColumnPattern::Satin;
        if p.bool("e_stitch") == Some(true) {
            pattern = ColumnPattern::EStitch;
        }
        match p.str("satin_method") {
            None | Some("satin_column" | "0") => {}
            Some("e_stitch" | "1") => pattern = ColumnPattern::EStitch,
            Some("zigzag") => pattern = ColumnPattern::Zigzag,
            Some(m) => warnings.add(Warning::SatinType(m.to_string())),
        }
        p.not_read(
            &[
                "push_compensation_mm",
                "random_width_increase_percent",
                "random_width_decrease_percent",
                "random_zigzag_spacing_percent",
                "split_method",
                "random_split_jitter_percent",
                "random_split_phase",
                "min_random_split_length_mm",
                "split_staggers",
                "center_walk_underlay_position",
                "contour_underlay_inset_percent",
                "zigzag_underlay_inset_percent",
                "zigzag_underlay_max_stitch_length_mm",
            ],
            warnings,
        );
        let mut params = SatinParams::with_pattern(pattern);
        params.spacing_mm = p.float("zigzag_spacing_mm").filter(|v| *v > 0.0).unwrap_or(0.4);
        let pull = p.floats("pull_compensation_mm");
        params.pull_compensation_mm = pull.first().copied().unwrap_or(0.0).max(0.0);
        if let Some(b) = pull.get(1)
            && (b - params.pull_compensation_mm).abs() > 1e-9
        {
            params.pull_b_mm = Some(b.max(0.0));
        }
        let percent = p.floats("pull_compensation_percent");
        params.pull_percent = percent.first().copied().unwrap_or(0.0).max(0.0);
        if percent.len() > 1 && (percent[0] - percent[1]).abs() > 1e-9 {
            warnings.add(Warning::Setting("pull_compensation_percent".into()));
        }
        let inset = p.floats("short_stitch_inset");
        params.short_stitches = pattern.is_satin() && inset.first().copied().unwrap_or(15.0) > 0.0;
        params.split_mm = p
            .float("max_stitch_length_mm")
            .filter(|v| *v > 0.0)
            .unwrap_or(SATIN_SPLIT_MM);
        // The underlay, in the order of Ink/Stitch: center walk, contour, zigzag.
        let mut layers = Vec::new();
        if p.bool("center_walk_underlay") == Some(true) {
            let repeats = p.int("center_walk_underlay_repeats").unwrap_or(2).max(1);
            for _ in 0..(repeats + 1) / 2 {
                layers.push(UnderlayLayer::CenterWalk);
            }
        }
        let contour_inset = {
            let v = p.floats("contour_underlay_inset_mm");
            if v.is_empty() {
                0.4
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            }
        };
        if p.bool("contour_underlay") == Some(true) {
            layers.push(UnderlayLayer::EdgeWalk {
                inset_mm: contour_inset,
            });
        }
        if p.bool("zigzag_underlay") == Some(true) {
            let inset = {
                let v = p.floats("zigzag_underlay_inset_mm");
                if v.is_empty() {
                    contour_inset / 2.0
                } else {
                    v.iter().sum::<f64>() / v.len() as f64
                }
            };
            layers.push(UnderlayLayer::Zigzag {
                inset_mm: inset,
                spacing_mm: p
                    .float("zigzag_underlay_spacing_mm")
                    .filter(|v| *v > 0.0)
                    .unwrap_or(3.0),
            });
        }
        params.underlay = if layers.is_empty() {
            Underlay::None
        } else {
            Underlay::Custom(layers)
        };
        vec![ImportedObject::new(
            ObjectKind::Satin { geometry, params },
            stroke.paint.color(),
            None,
        )]
    }

    /// The rails and the rungs of a satin column; for a shape with one
    /// subpath, a column of the stroke width along it.
    fn satin_geometry(&self, width_mm: f64) -> Option<SatinGeometry> {
        let p = self.p;
        let subs = &self.shape.subs;
        if subs.len() == 1 {
            if width_mm <= self.file.satin_stroke_mm {
                return None;
            }
            return Some(SatinGeometry::from_centerline(&subs[0], width_mm));
        }
        let lines: Vec<Vec<Point>> = subs
            .iter()
            .map(|s| {
                s.flatten(FLAT_MM)
                    .into_iter()
                    .next()
                    .map(|pl| pl.points)
                    .unwrap_or_default()
            })
            .collect();
        let (ia, ib) = rail_indices(&lines);
        let (mut a, mut b) = (subs[ia].clone(), subs[ib].clone());
        let (mut la, mut lb) = (lines[ia].clone(), lines[ib].clone());
        if p.bool("swap_satin_rails") == Some(true) {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut la, &mut lb);
        }
        let choice = p
            .str("reverse_rails")
            .unwrap_or(if self.version() < 1 { "none" } else { "automatic" });
        let (rev_a, rev_b) = rails_to_reverse(choice, &la, &lb);
        if rev_a {
            a = a.reversed();
            la.reverse();
        }
        if rev_b {
            b = b.reversed();
            lb.reverse();
        }
        let mut rungs: Vec<(Point, Point)> = Vec::new();
        if subs.len() == 2 {
            // No rungs: the nodes of the rails pair up when the rails have the
            // same number of nodes (`_synthesize_rungs`).
            let (na, nb) = (nodes(&a), nodes(&b));
            if na.len() == nb.len() && na.len() > 2 {
                rungs.extend(
                    na[1..na.len() - 1]
                        .iter()
                        .copied()
                        .zip(nb[1..nb.len() - 1].iter().copied()),
                );
            }
        } else {
            for (i, line) in lines.iter().enumerate() {
                if i == ia || i == ib || line.len() < 2 {
                    continue;
                }
                rungs.push((touch(line, &la), touch(line, &lb)));
            }
        }
        Some(SatinGeometry {
            rail_a: a,
            rail_b: b,
            rungs,
        })
    }

    fn strokes(&self, warnings: &mut Warnings) -> Vec<ImportedObject> {
        let p = self.p;
        let Some(stroke) = &self.shape.stroke else {
            return Vec::new();
        };
        let v = self.version();
        // The method: in the files of the first versions, a stroke with no
        // dashes is a zigzag (`lib/update.py`, version 1).
        let method = match p.str("stroke_method") {
            Some("0") => "running_stitch",
            Some("1") => "ripple_stitch",
            Some(m) => m,
            None if v < 1 && p.bool("manual_stitch") == Some(true) => "manual_stitch",
            None if v < 2 && self.el.tag == "polyline" => "manual_stitch",
            None if v < 1 && !stroke.dashed => "zigzag_stitch",
            None => "running_stitch",
        };
        let mut params = RunningParams::default();
        let run_len = {
            let l = p.floats("running_stitch_length_mm");
            if l.len() > 1 {
                warnings.add(Warning::Setting("running_stitch_length_mm".into()));
            }
            l.first().copied().unwrap_or(if v < 1 { 1.5 } else { 2.5 }).max(0.1)
        };
        params.stitch_length_mm = run_len;
        // Bean stitches: n back and forth over each stitch.
        let beans: Vec<i64> = p
            .floats("bean_stitch_repeats")
            .iter()
            .map(|b| b.max(0.0) as i64)
            .collect();
        match beans.as_slice() {
            [] | [0] => {}
            [n] => params.repeats = (2 * n + 1) as u32,
            [1, 0] | [0, 1] => params.style = RunStyle::Sculpture,
            [n, ..] => {
                params.repeats = (2 * n + 1) as u32;
                warnings.add(Warning::Setting("bean_stitch_repeats".into()));
            }
        }
        let repeats = p.int("repeats").unwrap_or(1).max(1);
        params.pass = if repeats % 2 == 0 {
            RunPass::Backtrack
        } else {
            RunPass::Once
        };
        if repeats > 2 {
            warnings.add(Warning::Setting("repeats".into()));
        }
        params.cut = p.str("cutwork_needle").is_some_and(|n| n != "0");
        p.not_read(&["enable_random_stitch_length"], warnings);
        let mut manual = false;
        match method {
            "running_stitch" => {}
            "manual_stitch" => {
                params.style = RunStyle::Manual;
                manual = true;
            }
            "zigzag_stitch" => {
                p.not_read(&["zigzag_angle"], warnings);
                let mut column = SatinParams::with_pattern(ColumnPattern::Zigzag);
                column.spacing_mm = p
                    .floats("zigzag_spacing_mm")
                    .first()
                    .copied()
                    .filter(|v| *v > 0.0)
                    .unwrap_or(0.4);
                let pull = if v < 3 {
                    p.float("pull_compensation_mm").map(|x| x / 2.0)
                } else {
                    p.floats("stroke_pull_compensation_mm").first().copied()
                };
                column.pull_compensation_mm = pull.unwrap_or(0.0).max(0.0);
                params.border = Some(LineBorder {
                    width_mm: stroke.width_mm,
                    column,
                    corner: Corner::Mitre,
                });
            }
            "ripple_stitch" => {
                p.not_read(
                    &[
                        "min_line_dist_mm",
                        "staggers",
                        "skip_start",
                        "skip_end",
                        "flip_copies",
                        "exponent",
                        "flip_exponent",
                        "reverse",
                        "grid_size_mm",
                        "grid_first",
                        "scale_axis",
                        "scale_start",
                        "rotate_ripples",
                        "join_style",
                        "manual_pattern_placement",
                        "satin_guide_pattern_position",
                    ],
                    warnings,
                );
                let guide = self.guides().into_iter().next().unwrap_or_default();
                let end_percent = if guide.is_empty() {
                    0.0
                } else {
                    p.float("scale_end").unwrap_or(0.0).max(0.0)
                };
                params.ripple = Some(Ripple {
                    lines: p.int("line_count").unwrap_or(10).clamp(1, i64::from(Ripple::MAX_LINES)) as u32,
                    end_percent,
                    target: self.command_point("target_point"),
                    guide,
                });
            }
            m => {
                warnings.add(Warning::Setting(format!("stroke_method={m}")));
            }
        }
        self.shape
            .subs
            .iter()
            .map(|s| {
                let path = if manual { node_path(s) } else { s.clone() };
                ImportedObject::new(
                    ObjectKind::Running {
                        path,
                        params: params.clone(),
                    },
                    stroke.paint.color(),
                    None,
                )
            })
            .collect()
    }
}

/// An angle in (-180°, 180°].
fn norm_deg(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

/// The accordion spacing for an end row spacing: the spacing of the fill
/// becomes the smaller of the two, and the factor goes from the start to
/// the end of the rows.
fn accordion(start: f64, end: Option<f64>, spacing: &mut f64) -> Option<Accordion> {
    let end = end.filter(|e| *e > 0.0 && (*e - start).abs() > 1e-9)?;
    let small = start.min(end);
    *spacing = small;
    Some(Accordion {
        points: vec![(0.0, start / small), (1.0, end / small)],
    })
}

/// The nodes of a path (the end points of its segments).
fn nodes(p: &Path) -> Vec<Point> {
    p.cmds
        .iter()
        .filter_map(|c| match *c {
            PathCmd::MoveTo(a) | PathCmd::LineTo(a) | PathCmd::CubicTo(_, _, a) => Some(a),
            PathCmd::Close => None,
        })
        .collect()
}

/// A path of straight lines through the nodes of `p` (manual stitches: each
/// node is a needle point).
fn node_path(p: &Path) -> Path {
    let pts = nodes(p);
    let mut out = Path::new();
    for (i, q) in pts.iter().enumerate() {
        out = if i == 0 { out.move_to(*q) } else { out.line_to(*q) };
    }
    if p.is_closed()
        && let Some(first) = pts.first()
    {
        out = out.line_to(*first);
    }
    out
}

fn length(line: &[Point]) -> f64 {
    line.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn crosses(a: &[Point], b: &[Point]) -> bool {
    a.windows(2)
        .any(|s| b.windows(2).any(|t| Point::crossing(s[0], s[1], t[0], t[1]).is_some()))
}

/// The two rails among the subpaths of a satin column, in the order of the
/// path. Ported from Ink/Stitch (`rail_indices`, see the module header):
/// with three subpaths, the rails cross one other subpath; with more, the
/// rails cross more than two others; else the two longest are the rails.
fn rail_indices(lines: &[Vec<Point>]) -> (usize, usize) {
    let n = lines.len();
    if n <= 2 {
        return (0, 1.min(n.saturating_sub(1)));
    }
    let counts: Vec<usize> = (0..n)
        .map(|i| (0..n).filter(|&j| j != i && crosses(&lines[i], &lines[j])).count())
        .collect();
    let candidates: Vec<usize> = (0..n)
        .filter(|&i| length(&lines[i]) > MIN_RAIL_MM && if n == 3 { counts[i] == 1 } else { counts[i] > 2 })
        .collect();
    if let [a, b] = candidates[..] {
        return (a, b);
    }
    let mut by_length: Vec<usize> = (0..n).collect();
    by_length.sort_by(|&x, &y| length(&lines[y]).total_cmp(&length(&lines[x])));
    let (a, b) = (by_length[0], by_length[1]);
    (a.min(b), a.max(b))
}

/// The point at the fraction `f` of the length of a line.
fn at_fraction(line: &[Point], f: f64) -> Point {
    let total = length(line);
    let mut left = total * f;
    for w in line.windows(2) {
        let l = w[0].distance(w[1]);
        if left <= l && l > 0.0 {
            return w[0].lerp(w[1], left / l);
        }
        left -= l;
    }
    line.last().copied().unwrap_or(Point::ZERO)
}

/// Which rails to reverse (first, second). Ported from Ink/Stitch
/// (`get_rails_to_reverse`, see the module header): `automatic` reverses
/// the second rail if its points at the same fractions are farther from the
/// first rail than its points at the reversed fractions.
fn rails_to_reverse(choice: &str, a: &[Point], b: &[Point]) -> (bool, bool) {
    match choice {
        "first" => (true, false),
        "second" => (false, true),
        "both" => (true, true),
        "automatic" if a.len() > 1 && b.len() > 1 => {
            let fractions = (0..10).map(|i| f64::from(i) / 10.0);
            let same: f64 = fractions
                .clone()
                .map(|f| at_fraction(a, f).distance(at_fraction(b, f)))
                .sum();
            let reversed: f64 = fractions
                .map(|f| at_fraction(a, f).distance(at_fraction(b, 1.0 - f)))
                .sum();
            (false, same > reversed)
        }
        _ => (false, false),
    }
}

/// Where a rung touches a rail: the first crossing, else the end of the
/// rung nearest to the rail.
fn touch(rung: &[Point], rail: &[Point]) -> Point {
    for s in rung.windows(2) {
        for t in rail.windows(2) {
            if let Some((u, _)) = Point::crossing(s[0], s[1], t[0], t[1]) {
                return s[0].lerp(s[1], u);
            }
        }
    }
    let dist = |p: Point| {
        rail.windows(2)
            .map(|t| p.distance_to_segment(t[0], t[1]))
            .fold(f64::INFINITY, f64::min)
    };
    let (first, last) = (rung[0], rung[rung.len() - 1]);
    if dist(first) <= dist(last) { first } else { last }
}

pub mod write;

#[cfg(test)]
mod tests;
