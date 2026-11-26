//! Writes a design as an SVG file of Ink/Stitch (P18 Milestone 6, ADR 0136).
//!
//! Each object that Ink/Stitch has becomes a path with the settings of
//! Ink/Stitch, in the sew order of the design:
//!
//! - A fill becomes a fill with its fill type (tatami, square, contour,
//!   guided, circular, meander, cross stitch, linear gradient; the other
//!   types are tatami), its angle, spacing and stitch length and its
//!   underlay rows.
//! - A satin column becomes a path of the two rails and the rungs, with its
//!   spacing, pull compensation, split length and underlay.
//! - A line becomes a stroke: a run (bean stitches, two passes), a zigzag,
//!   a satin column along the line, a ripple or manual stitches. The
//!   stitches of a machine file are manual stitches.
//! - A lettering object becomes the parts of its letters, in a group.
//! - A vector object is a shape that Ink/Stitch does not sew.
//!
//! Trims, stops and lock stitches are settings of the paths; start and end
//! points and the centers of circular fills and ripples are commands. A
//! setting or an object that Ink/Stitch does not have gives a warning. The
//! names and the meanings of the settings are those of the reader (see the
//! header of [`super`]).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use emb_geom::{BBox, Path, PathCmd, Point, math, region, text};
use emb_model::{
    AppliqueParams, AppliquePiece, ColumnPattern, CrossStyle, Design, DesignObject, FillParams, FillPattern, Functions,
    ObjectKind, Rgb, RunPass, RunStyle, RunningParams, SatinGeometry, SatinParams, TackDown, ThreadChart, TrimMode,
    UnderlayLayer,
};

use super::{NAMESPACE, at_fraction, length};

/// The underlay layers that an object sews, with tatami angles relative to
/// the fill angle (`emb_stitch::object::underlay_layers`).
pub type UnderlayOf<'a> = &'a dyn Fn(&ObjectKind) -> Option<Vec<UnderlayLayer>>;

/// A design written as an Ink/Stitch file.
#[derive(Debug, Clone)]
pub struct Written {
    /// The text of the SVG file.
    pub svg: String,
    /// What the file does not have, for the user.
    pub warnings: Vec<String>,
}

/// Space round the design on the page, in mm.
const MARGIN_MM: f64 = 10.0;
/// The stroke width of runs and of the rails of columns, in mm.
const LINE_MM: f64 = 0.3;
/// A column along a line needs a wider stroke than this (the metadata
/// `min_satin_stroke_width_mm` of Ink/Stitch).
const SATIN_STROKE_MM: f64 = 1.0;
/// The rungs go out past the rails by this part of their length, so they
/// cross the rails.
const RUNG_OUT: f64 = 0.1;
/// Tolerance to flatten rails, in mm.
const FLAT_MM: f64 = 0.05;

/// Writes `design` as an SVG file of Ink/Stitch (one mm is one unit of the
/// file). `underlay_of` gives the underlay layers of fills and columns.
pub fn write_inkstitch(design: &Design, underlay_of: UnderlayOf) -> Written {
    let bbox = design
        .objects()
        .iter()
        .filter_map(|o| o.control_bbox())
        .reduce(BBox::union)
        .unwrap_or(BBox {
            min: Point::ZERO,
            max: Point::new(10.0, 10.0),
        });
    let shift = Point::new(MARGIN_MM, MARGIN_MM) - bbox.min;
    let (w, h) = (bbox.width() + 2.0 * MARGIN_MM, bbox.height() + 2.0 * MARGIN_MM);
    let mut wr = Writer {
        shift,
        underlay_of,
        body: String::new(),
        symbols: BTreeSet::new(),
        guide_marker: false,
        gradients: String::new(),
        warnings: BTreeMap::new(),
        n: 0,
    };
    let objects = design.objects();
    let mut done: HashSet<usize> = HashSet::new();
    for (i, o) in objects.iter().enumerate() {
        if done.contains(&i) {
            continue;
        }
        let color = design.thread(o.thread).map_or(Rgb(0, 0, 0), |t| t.color);
        // The colors of a gradient are one shape with a gradient paint.
        if let ObjectKind::Fill {
            outer,
            params:
                FillParams {
                    pattern: FillPattern::Gradient { color: 0, stops, .. },
                    ..
                },
            ..
        } = &o.kind
        {
            let mut colors = vec![(0u32, color)];
            for (j, m) in objects.iter().enumerate().skip(i + 1) {
                if let ObjectKind::Fill {
                    outer: o2,
                    params:
                        FillParams {
                            pattern:
                                FillPattern::Gradient {
                                    color: k, stops: s2, ..
                                },
                            ..
                        },
                    ..
                } = &m.kind
                    && o2 == outer
                    && s2 == stops
                    && m.group == o.group
                {
                    colors.push((*k, design.thread(m.thread).map_or(Rgb(0, 0, 0), |t| t.color)));
                    done.insert(j);
                }
            }
            colors.sort_by_key(|c| c.0);
            wr.object(o, &o.kind, color, Some(&colors));
            continue;
        }
        wr.object(o, &o.kind, color, None);
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str(&format!(
        "<!-- Written by ShokujoDX for Ink/Stitch: {} -->\n",
        xml(&design.name).replace("--", "- -")
    ));
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"\n \
         xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\"\n \
         xmlns:sodipodi=\"http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd\"\n \
         xmlns:inkstitch=\"{NAMESPACE}\"\n \
         width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\" version=\"1.1\">\n",
        num(w),
        num(h),
        num(w),
        num(h)
    ));
    out.push_str(" <sodipodi:namedview id=\"namedview\" inkscape:document-units=\"mm\"/>\n");
    out.push_str(" <metadata id=\"metadata\">\n");
    out.push_str(&format!(
        "  <inkstitch:inkstitch_svg_version>{}</inkstitch:inkstitch_svg_version>\n",
        super::SVG_VERSION
    ));
    if let Some(chart) = palette(design) {
        out.push_str(&format!(
            "  <inkstitch:thread-palette>\"{}\"</inkstitch:thread-palette>\n",
            xml(&chart)
        ));
    }
    out.push_str(" </metadata>\n <defs id=\"defs\">\n");
    for s in &wr.symbols {
        out.push_str(&format!(
            "  <symbol id=\"inkstitch_{s}\" style=\"opacity:0.7\"><title>{s}</title>\
             <circle r=\"1\" style=\"fill:#000000\"/></symbol>\n"
        ));
    }
    if wr.guide_marker {
        out.push_str(
            "  <marker id=\"inkstitch-guide-line-marker\" style=\"overflow:visible\" orient=\"auto\" \
             markerWidth=\"0.1\" markerHeight=\"0.1\"><circle r=\"0.5\" \
             style=\"fill:#fafafa;stroke:#ff5500;stroke-width:0.2\"/></marker>\n",
        );
    }
    out.push_str(&wr.gradients);
    out.push_str(" </defs>\n");
    out.push_str(&format!(
        " <g id=\"layer1\" inkscape:groupmode=\"layer\" inkscape:label=\"{}\">\n",
        xml(&design.name)
    ));
    out.push_str(&wr.body);
    out.push_str(" </g>\n</svg>\n");
    Written {
        svg: out,
        warnings: wr.warning_texts(),
    }
}

/// The chart of the threads, when all of them are of one brand that is a
/// chart of the program (the thread palette of Ink/Stitch).
fn palette(design: &Design) -> Option<String> {
    let brands: BTreeSet<&str> = design.threads.iter().filter_map(|t| t.brand.as_deref()).collect();
    if brands.len() != 1 || design.threads.iter().any(|t| t.brand.is_none()) {
        return None;
    }
    let brand = *brands.iter().next()?;
    ThreadChart::builtin()
        .into_iter()
        .find(|c| c.name == brand)
        .map(|c| c.name)
}

/// A number with at most four decimals and no trailing zeros.
fn num(v: f64) -> String {
    let s = format!("{:.4}", if v.abs() < 5e-5 { 0.0 } else { v });
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// An angle in (-180°, 180°].
fn norm(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

/// What the file cannot have.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Warning {
    /// An object type that Ink/Stitch does not have (translated).
    Type(&'static str),
    /// A setting that Ink/Stitch does not have (translated).
    Setting(&'static str),
    /// A fill type that is written as tatami.
    FillType(&'static str),
    /// A column type that is written as satin.
    ColumnType(&'static str),
    /// A line type that is written as a run (translated).
    LineType(&'static str),
}

struct Writer<'a> {
    shift: Point,
    underlay_of: UnderlayOf<'a>,
    body: String,
    /// The command symbols that the file uses.
    symbols: BTreeSet<&'static str>,
    guide_marker: bool,
    gradients: String,
    warnings: BTreeMap<Warning, usize>,
    n: usize,
}

/// The settings of one path, as attribute names (without the prefix) and values.
type Params = Vec<(&'static str, String)>;

impl Writer<'_> {
    fn warn(&mut self, w: Warning) {
        *self.warnings.entry(w).or_default() += 1;
    }

    fn warning_texts(&self) -> Vec<String> {
        self.warnings
            .iter()
            .map(|(w, n)| match w {
                Warning::Type(t) => text::fill(
                    "{} object(s) of the type \"{}\" are not written: Ink/Stitch does not have them.",
                    &[n, t],
                ),
                Warning::Setting(s) => text::fill(
                    "The setting \"{}\" is not written: Ink/Stitch does not have it ({} object(s)).",
                    &[s, n],
                ),
                Warning::FillType(t) => text::fill(
                    "The fill type \"{}\" is written as tatami ({} object(s)).",
                    &[&text::tr(t), n],
                ),
                Warning::ColumnType(t) => text::fill(
                    "The column type \"{}\" is written as satin ({} object(s)).",
                    &[&text::tr(t), n],
                ),
                Warning::LineType(t) => text::fill(
                    "The line type \"{}\" is written as a running stitch ({} object(s)).",
                    &[t, n],
                ),
            })
            .collect()
    }

    fn id(&mut self, prefix: &str) -> String {
        self.n += 1;
        format!("{prefix}{}", self.n)
    }

    fn p(&self, p: Point) -> String {
        let q = p + self.shift;
        format!("{},{}", num(q.x), num(q.y))
    }

    /// The path data of `path`, moved onto the page.
    fn d(&self, path: &Path) -> String {
        let mut s = String::new();
        for c in &path.cmds {
            if !s.is_empty() {
                s.push(' ');
            }
            match *c {
                PathCmd::MoveTo(a) => s.push_str(&format!("M {}", self.p(a))),
                PathCmd::LineTo(a) => s.push_str(&format!("L {}", self.p(a))),
                PathCmd::CubicTo(a, b, e) => s.push_str(&format!("C {} {} {}", self.p(a), self.p(b), self.p(e))),
                PathCmd::Close => s.push('Z'),
            }
        }
        s
    }

    fn line_d(&self, pts: &[Point]) -> String {
        pts.iter()
            .enumerate()
            .map(|(i, q)| format!("{} {}", if i == 0 { "M" } else { "L" }, self.p(*q)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Writes one path and returns its id.
    fn path(&mut self, d: &str, style: &str, params: &[(&str, String)], label: Option<&str>, indent: &str) -> String {
        let id = self.id("path");
        let mut s = format!("{indent}<path id=\"{id}\"");
        if let Some(l) = label {
            s.push_str(&format!(" inkscape:label=\"{}\"", xml(l)));
        }
        s.push_str(&format!(" d=\"{d}\" style=\"{style}\""));
        for (k, v) in params {
            s.push_str(&format!(" inkstitch:{k}=\"{}\"", xml(v)));
        }
        s.push_str("/>\n");
        self.body.push_str(&s);
        id
    }

    /// A command of Ink/Stitch on the path `target`: a symbol at `at` and
    /// its connector.
    fn command(&mut self, cmd: &'static str, target: &str, at: Point, indent: &str) {
        self.symbols.insert(cmd);
        let (g, c, u) = (
            self.id("command_group"),
            self.id("command_connector"),
            self.id("command_use"),
        );
        let hidden = matches!(cmd, "starting_point" | "ending_point");
        let style = if hidden {
            "display:none;fill:none;stroke:#000000;stroke-width:0.1"
        } else {
            "fill:none;stroke:#000000;stroke-width:0.1;stroke-opacity:0.5"
        };
        let pt = self.p(at);
        let q = at + self.shift;
        self.body.push_str(&format!(
            "{indent}<g id=\"{g}\" inkscape:label=\"Ink/Stitch Command: {cmd}\">\
             <path id=\"{c}\" d=\"M {pt} {pt}\" style=\"{style}\" inkscape:connector-type=\"polyline\" \
             inkscape:connection-start=\"#{u}\" inkscape:connection-end=\"#{target}\"/>\
             <use id=\"{u}\" xlink:href=\"#inkstitch_{cmd}\" x=\"{}\" y=\"{}\" width=\"100%\" height=\"100%\"/></g>\n",
            num(q.x),
            num(q.y)
        ));
    }

    /// The settings of the machine functions.
    fn functions(&mut self, f: &Functions, last: bool, params: &mut Params) {
        let ties = match (f.tie_in, f.tie_off) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        };
        if ties != 0 {
            params.push(("ties", ties.to_string()));
        }
        if last && f.trim == TrimMode::Always {
            params.push(("trim_after", "true".into()));
        }
        if last && f.stop {
            params.push(("stop_after", "true".into()));
        }
        if f.slow {
            self.warn(Warning::Setting(text::tr("Slow")));
        }
    }

    /// Writes the object `o` with the geometry and the settings of `kind`
    /// (its own, or a part of a lettering object).
    fn object(&mut self, o: &DesignObject, kind: &ObjectKind, color: Rgb, gradient: Option<&[(u32, Rgb)]>) {
        let label = o.name.as_deref();
        let ids: Vec<String> = match kind {
            ObjectKind::Fill { outer, holes, params } => {
                self.fill(o, kind, outer, holes, params, color, gradient, label, "  ", true)
            }
            ObjectKind::Satin { geometry, params } => self.satin(o, kind, geometry, params, color, label, "  ", true),
            ObjectKind::Running { path, params } => self.running(o, path, params, color, label, "  ", true),
            ObjectKind::Stitches { path, trims } => self.stitches(path, trims, color, label),
            ObjectKind::Lettering(l) => {
                let g = self.id("lettering");
                self.body.push_str(&format!(
                    "  <g id=\"{g}\" inkscape:label=\"{}\">\n",
                    xml(label.unwrap_or(&l.text))
                ));
                let mut ids = Vec::new();
                let n = l.parts().count();
                for (i, part) in l.parts().enumerate() {
                    let last = i + 1 == n;
                    ids.extend(match part {
                        ObjectKind::Fill { outer, holes, params } => {
                            self.fill(o, part, outer, holes, params, color, None, None, "   ", last)
                        }
                        ObjectKind::Satin { geometry, params } => {
                            self.satin(o, part, geometry, params, color, None, "   ", last)
                        }
                        ObjectKind::Running { path, params } => self.running(o, path, params, color, None, "   ", last),
                        _ => Vec::new(),
                    });
                }
                self.body.push_str("  </g>\n");
                ids
            }
            ObjectKind::Vector { path, style } => {
                let fill = style.fill.map_or("none".to_string(), hex);
                let stroke = style.stroke.map_or("none".to_string(), hex);
                let st = format!(
                    "fill:{fill};fill-rule:evenodd;stroke:{stroke};stroke-width:{}",
                    num(style.stroke_mm)
                );
                let d = self.d(path);
                self.path(&d, &st, &[("ignore_object", "true".into())], label, "  ");
                return;
            }
            ObjectKind::Applique { pieces, params } => self.applique(o, pieces, params, color),
            ObjectKind::CrossStitch(_) => {
                self.warn(Warning::Type(text::tr("Cross stitch")));
                return;
            }
            ObjectKind::Bling(_) => {
                self.warn(Warning::Type(text::tr("Rhinestones")));
                return;
            }
            // A sketch sews nothing; its objects are objects of their own.
            ObjectKind::Sketch(_) => return,
        };
        // The start and end points are commands on the first and last path.
        if let (Some(at), Some(first)) = (o.start, ids.first()) {
            let first = first.clone();
            self.command("starting_point", &first, at, "  ");
        }
        if let (Some(at), Some(last)) = (o.end, ids.last()) {
            let last = last.clone();
            self.command("ending_point", &last, at, "  ");
        }
    }

    /// The settings of the machine functions for each path of an object:
    /// the trim and the stop on the last one.
    fn with_functions(&mut self, o: &DesignObject, params: &Params, last: bool) -> Params {
        let mut p = params.clone();
        let f = o.functions;
        self.functions(&f, last, &mut p);
        p
    }

    #[allow(clippy::too_many_arguments)]
    fn fill(
        &mut self,
        o: &DesignObject,
        kind: &ObjectKind,
        outer: &Path,
        holes: &[Path],
        p: &FillParams,
        color: Rgb,
        gradient: Option<&[(u32, Rgb)]>,
        label: Option<&str>,
        indent: &str,
        last: bool,
    ) -> Vec<String> {
        let mut params: Params = vec![
            ("angle", num(norm(-p.angle_deg))),
            ("row_spacing_mm", num(p.spacing_mm)),
            ("max_stitch_length_mm", num(p.stitch_length_mm)),
        ];
        if p.pull_compensation_mm > 0.0 {
            params.push(("pull_compensation_mm", num(p.pull_compensation_mm)));
        }
        let mut guides: &[Vec<Point>] = &[];
        let mut target = None;
        let mut paint = hex(color);
        match &p.pattern {
            FillPattern::Tatami => params.push(("fill_method", "tatami_fill".into())),
            FillPattern::Square => {
                params.push(("fill_method", "tatami_fill".into()));
                params.push(("staggers", "1".into()));
            }
            FillPattern::Offset | FillPattern::Spiral | FillPattern::Maze => {
                params.push(("fill_method", "contour_fill".into()));
                let s = match p.pattern {
                    FillPattern::Spiral => "1",
                    FillPattern::Maze => "2",
                    _ => "0",
                };
                params.push(("contour_strategy", s.into()));
            }
            FillPattern::Guided { guides: g } => {
                params.push(("fill_method", "guided_fill".into()));
                guides = g;
            }
            FillPattern::Circular { center } => {
                params.push(("fill_method", "circular_fill".into()));
                target = *center;
            }
            FillPattern::Meander { .. } => {
                params.push(("fill_method", "meander_fill".into()));
                self.warn(Warning::Setting(text::tr("Meander tile")));
            }
            FillPattern::CrossStitch { cell_mm, style, passes } => {
                params.push(("fill_method", "cross_stitch".into()));
                params.push(("pattern_size_mm", num(*cell_mm)));
                let m = match style {
                    CrossStyle::Full => "simple_cross",
                    CrossStyle::Diagonal => "half_cross",
                    CrossStyle::Upright => "upright_cross",
                };
                params.push(("cross_stitch_method", m.into()));
                if *passes != 1 {
                    self.warn(Warning::Setting(text::tr("Passes")));
                }
            }
            FillPattern::Gradient { line, stops, .. } => match gradient {
                Some(colors) if colors.len() > 1 => {
                    params.push(("fill_method", "linear_gradient_fill".into()));
                    let (a, b) = line.unwrap_or_else(|| gradient_line(outer, p.angle_deg));
                    let id = self.id("gradient");
                    let (a, b) = (a + self.shift, b + self.shift);
                    let mut g = format!(
                        "  <linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" \
                         x2=\"{}\" y2=\"{}\">",
                        num(a.x),
                        num(a.y),
                        num(b.x),
                        num(b.y)
                    );
                    for (k, c) in colors {
                        let at = stops.get(*k as usize).copied().unwrap_or(0.0);
                        g.push_str(&format!(
                            "<stop offset=\"{}\" style=\"stop-color:{}\"/>",
                            num(at),
                            hex(*c)
                        ));
                    }
                    g.push_str("</linearGradient>\n");
                    self.gradients.push_str(&g);
                    paint = format!("url(#{id})");
                }
                _ => params.push(("fill_method", "tatami_fill".into())),
            },
            other => {
                params.push(("fill_method", "tatami_fill".into()));
                self.warn(Warning::FillType(other.name()));
            }
        }
        // The underlay: rows at angles; one spacing and one inset for all.
        let layers = (self.underlay_of)(kind).unwrap_or_default();
        let rows: Vec<(f64, f64, f64)> = layers
            .iter()
            .filter_map(|l| match *l {
                UnderlayLayer::Tatami {
                    inset_mm,
                    spacing_mm,
                    angle_deg,
                } => Some((inset_mm, spacing_mm, angle_deg)),
                _ => None,
            })
            .collect();
        if rows.len() < layers.len() {
            self.warn(Warning::Setting(text::tr("Edge walk underlay of a fill")));
        }
        match rows.first() {
            None => params.push(("fill_underlay", "false".into())),
            Some(&(inset, spacing, _)) => {
                if rows
                    .iter()
                    .any(|r| (r.0 - inset).abs() > 1e-6 || (r.1 - spacing).abs() > 1e-6)
                {
                    self.warn(Warning::Setting(text::tr("Underlay layers with other spacings")));
                }
                params.push(("fill_underlay", "true".into()));
                let angles: Vec<String> = rows.iter().map(|r| num(norm(-(p.angle_deg + r.2)))).collect();
                params.push(("fill_underlay_angle", angles.join(" ")));
                params.push(("fill_underlay_row_spacing_mm", num(spacing)));
                params.push(("fill_underlay_inset_mm", num(inset)));
            }
        }
        if let Some(a) = &p.accordion {
            match a.points.as_slice() {
                [(t0, f0), (t1, f1)] if *t0 == 0.0 && *t1 == 1.0 => {
                    // The row spacing goes from the first to the last row.
                    if let Some(i) = params.iter().position(|(k, _)| *k == "row_spacing_mm") {
                        params[i].1 = num(p.spacing_mm * f0.max(1.0));
                    }
                    params.push(("end_row_spacing_mm", num(p.spacing_mm * f1.max(1.0))));
                }
                _ => self.warn(Warning::Setting(text::tr("Accordion spacing"))),
            }
        }
        if !p.stamps.is_empty() {
            self.warn(Warning::Setting(text::tr("Carving stamps")));
        }
        if p.trapunto {
            self.warn(Warning::Setting(text::tr("Trapunto")));
        }
        if p.jagged_mm > 0.0 {
            self.warn(Warning::Setting(text::tr("Jagged edge")));
        }
        if p.hand_mm > 0.0 {
            self.warn(Warning::Setting(text::tr("Hand-stitch effect")));
        }
        if p.direction.is_some() {
            self.warn(Warning::Setting(text::tr("Direction line")));
        }
        let params = self.with_functions(o, &params, last);
        let mut d = self.d(&outer.clone().closed());
        for h in holes {
            d.push(' ');
            d.push_str(&self.d(&h.clone().closed()));
        }
        let style = format!("fill:{paint};fill-rule:evenodd;stroke:none");
        let id = if guides.is_empty() {
            self.path(&d, &style, &params, label, indent)
        } else {
            // The guide lines are in the group of the fill, with the marker.
            self.guide_marker = true;
            let g = self.id("group");
            self.body.push_str(&format!("{indent}<g id=\"{g}\">\n"));
            let inner = format!("{indent} ");
            let id = self.path(&d, &style, &params, label, &inner);
            for guide in guides {
                let gd = self.line_d(guide);
                self.path(
                    &gd,
                    "fill:none;stroke:#000000;stroke-width:0.3;marker-start:url(#inkstitch-guide-line-marker)",
                    &[],
                    None,
                    &inner,
                );
            }
            self.body.push_str(&format!("{indent}</g>\n"));
            id
        };
        if let Some(c) = target {
            self.command("target_point", &id, c, indent);
        }
        vec![id]
    }

    #[allow(clippy::too_many_arguments)]
    fn satin(
        &mut self,
        o: &DesignObject,
        kind: &ObjectKind,
        g: &SatinGeometry,
        p: &SatinParams,
        color: Rgb,
        label: Option<&str>,
        indent: &str,
        last: bool,
    ) -> Vec<String> {
        // A band of a column (P22 Milestone 4) is a column with the rails
        // of its band.
        let band = p.active_band();
        let band_geometry = band.and_then(|b| g.band(&b));
        let band_params = band.map(|b| p.banded(&b));
        let g = band_geometry.as_ref().unwrap_or(g);
        let p = band_params.as_ref().unwrap_or(p);
        let mut params: Params = vec![("satin_column", "true".into())];
        self.column_params(p, &mut params);
        self.satin_underlay(kind, &mut params);
        let params = self.with_functions(o, &params, last);
        let mut d = format!("{} {}", self.d(&g.rail_a), self.d(&g.rail_b));
        for (a, b) in rungs(g) {
            d.push(' ');
            d.push_str(&self.line_d(&[a, b]));
        }
        let style = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(LINE_MM));
        vec![self.path(&d, &style, &params, label, indent)]
    }

    /// The settings of a column: type, spacing, pull compensation, short
    /// stitches and split.
    fn column_params(&mut self, p: &SatinParams, params: &mut Params) {
        match p.pattern {
            ColumnPattern::Satin => {}
            ColumnPattern::EStitch => params.push(("satin_method", "e_stitch".into())),
            ColumnPattern::Zigzag => params.push(("satin_method", "zigzag".into())),
            other => self.warn(Warning::ColumnType(other.name())),
        }
        params.push(("zigzag_spacing_mm", num(p.spacing_mm)));
        let pull = p.pull_compensation_mm;
        match p.pull_b_mm {
            Some(b) if (b - pull).abs() > 1e-9 => {
                params.push(("pull_compensation_mm", format!("{} {}", num(pull), num(b))))
            }
            _ if pull > 0.0 => params.push(("pull_compensation_mm", num(pull))),
            _ => {}
        }
        if p.pull_percent > 0.0 {
            params.push(("pull_compensation_percent", num(p.pull_percent)));
        }
        if !p.short_stitches {
            params.push(("short_stitch_inset", "0".into()));
        }
        params.push(("max_stitch_length_mm", num(p.split_mm)));
        params.push(("reverse_rails", "none".into()));
        for (on, name) in [
            (p.liquid, text::tr("Liquid effect")),
            (p.auto_spacing, text::tr("Auto spacing")),
            (p.accordion.is_some(), text::tr("Accordion spacing")),
            (p.jagged_mm > 0.0, text::tr("Jagged edge")),
            (p.hand_mm > 0.0, text::tr("Hand-stitch effect")),
            (p.puff, text::tr("Puff")),
            (p.omit_last, text::tr("Omit the last stitch")),
            (p.lean_deg.abs() > 1e-9, text::tr("Lean")),
            (p.auto_jump, text::tr("Auto jump")),
            ((p.spacing_at - 0.5).abs() > 1e-9, text::tr("Fractional spacing")),
        ] {
            if on {
                self.warn(Warning::Setting(name));
            }
        }
    }

    /// The underlay of a satin column: center walk, contour and zigzag, in
    /// the order of Ink/Stitch.
    fn satin_underlay(&mut self, kind: &ObjectKind, params: &mut Params) {
        let layers = (self.underlay_of)(kind).unwrap_or_default();
        let mut walks = 0;
        let mut edge = None;
        let mut zigzag = None;
        let mut order_ok = true;
        let mut stage = 0;
        for l in &layers {
            let s = match *l {
                UnderlayLayer::CenterWalk => {
                    walks += 1;
                    0
                }
                UnderlayLayer::EdgeWalk { inset_mm } => {
                    if edge.replace(inset_mm).is_some() {
                        order_ok = false;
                    }
                    1
                }
                UnderlayLayer::Zigzag { inset_mm, spacing_mm } => {
                    if zigzag.replace((inset_mm, spacing_mm)).is_some() {
                        order_ok = false;
                    }
                    2
                }
                UnderlayLayer::Tatami { .. } => {
                    self.warn(Warning::Setting(text::tr("Tatami underlay of a column")));
                    continue;
                }
            };
            if s < stage {
                order_ok = false;
            }
            stage = s;
        }
        if !order_ok {
            self.warn(Warning::Setting(text::tr("Underlay order")));
        }
        if walks > 0 {
            params.push(("center_walk_underlay", "true".into()));
            params.push(("center_walk_underlay_repeats", (2 * walks).to_string()));
        }
        if let Some(inset) = edge {
            params.push(("contour_underlay", "true".into()));
            params.push(("contour_underlay_inset_mm", num(inset)));
        }
        if let Some((inset, spacing)) = zigzag {
            params.push(("zigzag_underlay", "true".into()));
            params.push(("zigzag_underlay_inset_mm", num(inset)));
            params.push(("zigzag_underlay_spacing_mm", num(spacing)));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn running(
        &mut self,
        o: &DesignObject,
        path: &Path,
        p: &RunningParams,
        color: Rgb,
        label: Option<&str>,
        indent: &str,
        last: bool,
    ) -> Vec<String> {
        let mut params: Params = Vec::new();
        let thin = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(LINE_MM));
        let mut style = thin.clone();
        let mut per_subpath = false;
        let mut target = None;
        let mut guide: &[Point] = &[];
        if let Some(r) = &p.ripple {
            params.push(("stroke_method", "ripple_stitch".into()));
            params.push(("line_count", r.lines.to_string()));
            target = r.target;
            guide = &r.guide;
            if !guide.is_empty() {
                params.push(("scale_end", num(r.end_percent)));
            }
        } else if let Some(b) = &p.border {
            let wide = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(b.width_mm));
            match b.column.pattern {
                ColumnPattern::Satin | ColumnPattern::EStitch if b.width_mm > SATIN_STROKE_MM => {
                    // A column of the stroke width along each line.
                    params.push(("satin_column", "true".into()));
                    self.column_params(&b.column, &mut params);
                    style = wide;
                    per_subpath = true;
                }
                pattern => {
                    if pattern != ColumnPattern::Zigzag {
                        self.warn(Warning::ColumnType(pattern.name()));
                    }
                    params.push(("stroke_method", "zigzag_stitch".into()));
                    params.push(("zigzag_spacing_mm", num(b.column.spacing_mm)));
                    if b.column.pull_compensation_mm > 0.0 {
                        params.push(("stroke_pull_compensation_mm", num(b.column.pull_compensation_mm)));
                    }
                    style = wide;
                }
            }
            if b.corner != emb_model::Corner::Mitre {
                self.warn(Warning::Setting(text::tr("Corners")));
            }
        } else {
            if p.motif.is_some() {
                self.warn(Warning::LineType(text::tr("Motif run")));
            }
            match p.style {
                RunStyle::Run => params.push(("stroke_method", "running_stitch".into())),
                RunStyle::Manual => params.push(("stroke_method", "manual_stitch".into())),
                RunStyle::Sculpture => {
                    params.push(("stroke_method", "running_stitch".into()));
                    params.push(("bean_stitch_repeats", "1 0".into()));
                }
                RunStyle::Backstitch => {
                    params.push(("stroke_method", "running_stitch".into()));
                    self.warn(Warning::LineType(text::tr("Backstitch")));
                }
                RunStyle::Stemstitch => {
                    params.push(("stroke_method", "running_stitch".into()));
                    self.warn(Warning::LineType(text::tr("Stem stitch")));
                }
            }
            if p.repeats > 1 && p.style != RunStyle::Sculpture {
                params.push(("bean_stitch_repeats", ((p.repeats - 1) / 2).to_string()));
                if p.repeats.is_multiple_of(2) {
                    self.warn(Warning::Setting(text::tr("Repeats")));
                }
            }
            params.push(("running_stitch_length_mm", num(p.stitch_length_mm)));
        }
        match p.pass {
            RunPass::Once => {}
            RunPass::Backtrack | RunPass::Repeat => params.push(("repeats", "2".into())),
            RunPass::Redwork => self.warn(Warning::LineType(text::tr("Redwork"))),
            RunPass::Branch => self.warn(Warning::LineType(text::tr("Branch"))),
        }
        if p.cut {
            params.push(("cutwork_needle", "1".into()));
        }
        if p.hand_mm > 0.0 {
            self.warn(Warning::Setting(text::tr("Hand-stitch effect")));
        }
        if p.cording {
            self.warn(Warning::Setting(text::tr("Cording")));
        }
        let subs: Vec<Path> = if per_subpath {
            path.subpaths()
        } else {
            vec![path.clone()]
        };
        let path_of = |s: &Path| -> Path {
            if p.style == RunStyle::Manual {
                super::node_path(s)
            } else {
                s.clone()
            }
        };
        let mut ids = Vec::new();
        let n = subs.len();
        for (i, s) in subs.iter().enumerate() {
            let pr = self.with_functions(o, &params, last && i + 1 == n);
            let d = self.d(&path_of(s));
            if guide.is_empty() {
                ids.push(self.path(&d, &style, &pr, label, indent));
            } else {
                self.guide_marker = true;
                let g = self.id("group");
                self.body.push_str(&format!("{indent}<g id=\"{g}\">\n"));
                let inner = format!("{indent} ");
                ids.push(self.path(&d, &style, &pr, label, &inner));
                let gd = self.line_d(guide);
                self.path(
                    &gd,
                    "fill:none;stroke:#000000;stroke-width:0.3;marker-start:url(#inkstitch-guide-line-marker)",
                    &[],
                    None,
                    &inner,
                );
                self.body.push_str(&format!("{indent}</g>\n"));
            }
        }
        if let (Some(c), Some(id)) = (target, ids.first()) {
            let id = id.clone();
            self.command("target_point", &id, c, indent);
        }
        ids
    }

    /// An appliqué object as Ink/Stitch users sew one: the placement lines
    /// and a stop, the tack-down lines and a stop, and a column of the cover
    /// width along each edge.
    fn applique(&mut self, o: &DesignObject, pieces: &[AppliquePiece], p: &AppliqueParams, color: Rgb) -> Vec<String> {
        let name = o.name.as_deref().unwrap_or("Appliqué");
        let thin = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(LINE_MM));
        let regions: Vec<region::Region> = pieces
            .iter()
            .filter_map(|piece| {
                let outer = piece.outer.flatten(FLAT_MM).into_iter().next()?;
                let holes: Vec<_> = piece
                    .holes
                    .iter()
                    .filter_map(|h| h.flatten(FLAT_MM).into_iter().next())
                    .collect();
                Some(region::Region::from_polylines(&outer, &holes))
            })
            .collect();
        let rings = |rs: &[region::Region]| -> Vec<Path> {
            rs.iter()
                .flat_map(|r| std::iter::once(&r.outer).chain(r.holes.iter()))
                .filter(|ring| ring.len() > 2)
                .map(|ring| Path::polygon(ring))
                .collect()
        };
        if pieces.iter().any(|piece| !piece.open_edges.is_empty()) {
            self.warn(Warning::Setting(text::tr("Open edges")));
        }
        let mut ids = Vec::new();
        let mut step = |w: &mut Self, label: String, paths: Vec<Path>, params: Params, style: &str, stop: bool| {
            let n = paths.len();
            for (i, path) in paths.iter().enumerate() {
                let mut pr = params.clone();
                if stop && i + 1 == n {
                    pr.push(("stop_after", "true".into()));
                }
                let d = w.d(path);
                ids.push(w.path(&d, style, &pr, Some(&label), "  "));
            }
        };
        if p.placement {
            let params = vec![
                ("stroke_method", "running_stitch".into()),
                ("running_stitch_length_mm", num(p.placement_stitch_mm)),
            ];
            step(self, format!("{name} placement"), rings(&regions), params, &thin, true);
        }
        let inset = rings(&region::offset(&regions, -p.tack_inset_mm, region::Join::Miter));
        match p.tack_down {
            TackDown::Run => {
                let params = vec![("stroke_method", "running_stitch".into())];
                step(self, format!("{name} tack-down"), inset, params, &thin, p.trim_stop);
            }
            TackDown::Zigzag => {
                let params = vec![
                    ("stroke_method", "zigzag_stitch".into()),
                    ("zigzag_spacing_mm", num(p.tack_spacing_mm)),
                ];
                let style = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(p.tack_width_mm));
                step(self, format!("{name} tack-down"), inset, params, &style, p.trim_stop);
            }
            TackDown::None => {}
        }
        if p.cover {
            // The middle of the cover is half its width inside its outer edge.
            let middle = rings(&region::offset(
                &regions,
                p.cover_outside_mm - p.cover_width_mm / 2.0,
                region::Join::Round,
            ));
            let style = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(p.cover_width_mm));
            for ring in middle {
                let kind = ObjectKind::Satin {
                    geometry: SatinGeometry::from_centerline(&ring, p.cover_width_mm),
                    params: p.cover_satin.clone(),
                };
                let mut params: Params = vec![("satin_column", "true".into())];
                self.column_params(&p.cover_satin, &mut params);
                self.satin_underlay(&kind, &mut params);
                let d = self.d(&ring);
                ids.push(self.path(&d, &style, &params, Some(&format!("{name} cover")), "  "));
            }
        }
        // The functions of the object come with its last path.
        if let Some(last) = ids.last().cloned() {
            let mut pr = Vec::new();
            self.functions(&o.functions, true, &mut pr);
            if !pr.is_empty() {
                let attrs: String = pr
                    .iter()
                    .map(|(k, v)| format!(" inkstitch:{k}=\"{}\"", xml(v)))
                    .collect();
                let tag = format!("id=\"{last}\"");
                if let Some(at) = self.body.rfind(&tag) {
                    self.body.insert_str(at + tag.len(), &attrs);
                }
            }
        }
        ids
    }

    /// The stitches of a machine file: manual stitches, one path for the
    /// subpaths between two trims.
    fn stitches(&mut self, path: &Path, trims: &[usize], color: Rgb, label: Option<&str>) -> Vec<String> {
        let subs = path.subpaths();
        let mut groups: Vec<Vec<Path>> = Vec::new();
        for (i, s) in subs.into_iter().enumerate() {
            if groups.is_empty() || trims.contains(&i) {
                groups.push(Vec::new());
            }
            if let Some(g) = groups.last_mut() {
                g.push(s);
            }
        }
        let style = format!("fill:none;stroke:{};stroke-width:{}", hex(color), num(LINE_MM));
        let n = groups.len();
        let mut ids = Vec::new();
        for (i, g) in groups.iter().enumerate() {
            let mut params: Params = vec![("stroke_method", "manual_stitch".into())];
            if i + 1 < n {
                params.push(("trim_after", "true".into()));
            }
            let d = g.iter().map(|s| self.d(s)).collect::<Vec<_>>().join(" ");
            ids.push(self.path(&d, &style, &params, label, "  "));
        }
        ids
    }
}

/// The line of a gradient with no line: across the rows, from one side of
/// the shape to the other.
fn gradient_line(outer: &Path, angle_deg: f64) -> (Point, Point) {
    let (s, c) = math::sin_cos_deg(angle_deg + 90.0);
    let dir = Point::new(c, s);
    let pts: Vec<Point> = outer.flatten(FLAT_MM).into_iter().flat_map(|l| l.points).collect();
    let center = outer.control_bbox().map_or(Point::ZERO, |b| b.center());
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
        let t = (*p - center).dot(dir);
        (lo.min(t), hi.max(t))
    });
    if !lo.is_finite() {
        return (center, center + dir);
    }
    (center + dir * lo, center + dir * hi)
}

/// The rungs of a column for Ink/Stitch: its rungs, or one in the middle
/// when it has none (with no rung and the same number of nodes on both
/// rails, Ink/Stitch pairs the nodes); a third one when it has two (with
/// two, Ink/Stitch takes the two longest subpaths as the rails). Each rung
/// goes out past the rails, so it crosses them. A rung in the middle of a
/// section pairs the rails as the section does.
fn rungs(g: &SatinGeometry) -> Vec<(Point, Point)> {
    let line = |p: &Path| {
        p.flatten(FLAT_MM)
            .into_iter()
            .next()
            .map(|l| l.points)
            .unwrap_or_default()
    };
    let (a, b) = (line(&g.rail_a), line(&g.rail_b));
    if a.len() < 2 || b.len() < 2 {
        return Vec::new();
    }
    let mut rungs = g.rungs.clone();
    match rungs.len() {
        0 => rungs.push((at_fraction(&a, 0.5), at_fraction(&b, 0.5))),
        2 => {
            // In the middle of the section before the first rung.
            let (fa, fb) = (fraction_of(&a, rungs[0].0), fraction_of(&b, rungs[0].1));
            rungs.insert(0, (at_fraction(&a, fa / 2.0), at_fraction(&b, fb / 2.0)));
        }
        _ => {}
    }
    rungs
        .into_iter()
        .map(|(p, q)| {
            let v = (q - p) * RUNG_OUT;
            (p - v, q + v)
        })
        .collect()
}

/// The fraction of the length of a line at its point nearest to `p`.
fn fraction_of(line: &[Point], p: Point) -> f64 {
    let total = length(line);
    if total <= 0.0 {
        return 0.0;
    }
    let mut best = (f64::INFINITY, 0.0);
    let mut at = 0.0;
    for w in line.windows(2) {
        let q = p.nearest_on_segment(w[0], w[1]);
        let d = q.distance(p);
        if d < best.0 {
            best = (d, at + w[0].distance(q));
        }
        at += w[0].distance(w[1]);
    }
    best.1 / total
}

#[cfg(test)]
mod tests;
