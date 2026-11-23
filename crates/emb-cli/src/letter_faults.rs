//! The letter fault bench (phase P21, `docs/design/p21-plan.md`): the
//! faults of the letters of the stroke finder, measured on their needle
//! points against the outline of the source font.
//!
//! The four fonts with a source font in the program ([`FONTS`]) are
//! measured in two ways: the bundled embroidery font (made by the converter
//! with its steps after the stroke finder) and the TrueType path (the
//! letters that lettering in the TrueType font gets: the stroke finder and
//! the route). Each letter of [`chars`] is sewn at the design size of the
//! embroidery fonts (25 mm) with the default satin settings. The measures:
//!
//! - spikes: needle points more than [`SPIKE_MM`] outside the outline (the
//!   default pull compensation is 0.2 mm), near an end of a column or in
//!   its middle, and needle points more than [`BARE_MM`] outside with no
//!   pull compensation (a fault of the shape of the column);
//! - rail points: the pairs of the column more than [`RAIL_MM`] outside;
//! - crossing pairs: two pairs next to each other that cross (stitches on
//!   each other), and sides that go back against the column;
//! - fans: a side with needle points more than [`FAN_MM`] apart;
//! - bulges: a column more than [`BULGE`] times its middle width, away
//!   from its ends;
//! - notches: stitch lines that go out of the outline between two needle
//!   points that are nearer to it;
//! - open area: the area of the outline that no part covers, near a
//!   convex corner of the outline and elsewhere, and the corners that are
//!   less than [`CORNER_COVER`] covered;
//! - joints: a column that ends in an earlier column (on top of it) or in
//!   a later one (under it; a digitizer puts it under);
//! - travel runs outside the outline.

use emb_geom::region::ring_contains;
use emb_geom::{BBox, Path, Point, Transform};
use emb_letter::grid::Grid;
use emb_letter::{EmbFont, EmbLetter, Font};
use emb_model::{ObjectKind, SatinGeometry, SatinParams};
use emb_stitch::satin::{Column, satin};
use serde_json::{Value, json};

/// The outline of a letter: outer rings with their holes, in mm.
pub type Outline = Vec<(Vec<Point>, Vec<Vec<Point>>)>;

/// The embroidery fonts with their source font in the program: the family
/// of the embroidery font, and the family and the style of the source.
pub const FONTS: [(&str, &str, &str); 4] = [
    ("Block Sans", "Lato", "Bold"),
    ("Anton", "Anton", "Regular"),
    ("Pacifico", "Pacifico", "Regular"),
    ("Varsity", "Graduate", "Regular"),
];

/// The capital height of the measure, in mm.
pub const CAP_MM: f64 = 25.0;
/// A needle point this far outside (mm) with the default pull compensation
/// is a spike.
pub const SPIKE_MM: f64 = 0.4;
/// A needle point this far outside (mm) with no pull compensation.
pub const BARE_MM: f64 = 0.2;
/// A rail point this far outside (mm).
pub const RAIL_MM: f64 = 0.1;
/// Needle points on one side farther apart than this (mm) are a fan.
pub const FAN_MM: f64 = 0.9;
/// A column wider than this times its middle width is a bulge.
pub const BULGE: f64 = 1.5;
/// A convex corner is open if less than this part of the outline near it
/// (within [`CORNER_NEAR_MM`]) is covered.
pub const CORNER_COVER: f64 = 0.6;
pub const CORNER_NEAR_MM: f64 = 0.6;
/// Open area this near a convex corner (mm) is open area at a corner.
const CORNER_AREA_MM: f64 = 0.8;
/// The outline has a corner where it turns more than this (degrees) over
/// [`CORNER_SPAN_MM`] before and after a point.
const CORNER_DEG: f64 = 55.0;
const CORNER_SPAN_MM: f64 = 0.4;
/// The cells of the open area, in mm.
const CELL_MM: f64 = 0.05;

/// The faults of one letter, or the sum of the faults of several.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Faults {
    pub letters: usize,
    pub spike_end: usize,
    pub spike_mid: usize,
    pub bare_out: usize,
    pub rail_out_end: usize,
    pub rail_out_mid: usize,
    pub cross: usize,
    pub back_mm: f64,
    pub fan_mm: f64,
    pub bulge_mm: f64,
    pub notch: usize,
    pub run_out_mm: f64,
    pub open_corner: f64,
    pub open_other: f64,
    pub area: f64,
    pub corners: usize,
    pub corners_open: usize,
    pub on_top: usize,
    pub under: usize,
}

impl Faults {
    pub fn add(&mut self, o: &Faults) {
        self.letters += o.letters;
        self.spike_end += o.spike_end;
        self.spike_mid += o.spike_mid;
        self.bare_out += o.bare_out;
        self.rail_out_end += o.rail_out_end;
        self.rail_out_mid += o.rail_out_mid;
        self.cross += o.cross;
        self.back_mm += o.back_mm;
        self.fan_mm += o.fan_mm;
        self.bulge_mm += o.bulge_mm;
        self.notch += o.notch;
        self.run_out_mm += o.run_out_mm;
        self.open_corner += o.open_corner;
        self.open_other += o.open_other;
        self.area += o.area;
        self.corners += o.corners;
        self.corners_open += o.corners_open;
        self.on_top += o.on_top;
        self.under += o.under;
    }

    /// The open area as a part of the area of the outline, in percent.
    pub fn open_percent(&self) -> f64 {
        100.0 * (self.open_corner + self.open_other) / self.area.max(1e-9)
    }
}

/// The faults of one letter, with what a picture of it needs.
#[derive(Debug, Clone)]
pub struct Letter {
    pub name: String,
    pub faults: Faults,
    pub picture: Value,
}

/// The faults of one font in one way.
#[derive(Debug, Clone)]
pub struct FontFaults {
    pub family: String,
    /// "embroidery" or "truetype".
    pub way: &'static str,
    pub sum: Faults,
    pub letters: Vec<Letter>,
}

/// The characters of the measure.
pub fn chars() -> Vec<char> {
    ('A'..='Z').chain('a'..='z').chain('0'..='9').collect()
}

fn inside(o: &Outline, p: Point) -> bool {
    o.iter()
        .any(|(r, h)| ring_contains(r, p) && !h.iter().any(|h| ring_contains(h, p)))
}

fn rings(o: &Outline) -> impl Iterator<Item = &Vec<Point>> {
    o.iter().flat_map(|(r, h)| std::iter::once(r).chain(h.iter()))
}

/// How far `p` is outside the outline (0 inside).
pub fn outside_by(o: &Outline, p: Point) -> f64 {
    if inside(o, p) {
        return 0.0;
    }
    let mut best = f64::MAX;
    for r in rings(o) {
        for i in 0..r.len() {
            best = best.min(p.distance_to_segment(r[i], r[(i + 1) % r.len()]));
        }
    }
    best
}

/// The convex corners of the outline.
pub fn convex_corners(o: &Outline) -> Vec<Point> {
    let cos_max = emb_geom::math::cos(emb_geom::math::to_radians(CORNER_DEG));
    let mut out: Vec<(f64, Point)> = Vec::new();
    for r in rings(o) {
        let n = r.len();
        if n < 3 {
            continue;
        }
        let away = |i: usize, forward: bool| -> Point {
            let (mut j, mut left) = (i, CORNER_SPAN_MM);
            for _ in 0..n {
                let next = if forward { (j + 1) % n } else { (j + n - 1) % n };
                let d = r[j].distance(r[next]);
                if d >= left {
                    return r[j].lerp(r[next], left / d.max(1e-12));
                }
                left -= d;
                j = next;
            }
            r[j]
        };
        for (i, &c) in r.iter().enumerate() {
            let (Some(a), Some(b)) = ((c - away(i, false)).normalized(), (away(i, true) - c).normalized()) else {
                continue;
            };
            if a.dot(b) >= cos_max {
                continue;
            }
            // The inside is on the inner side of the turn.
            if let Some(inward) = (b - a).normalized()
                && inside(o, c + inward * 0.1)
            {
                out.push((a.dot(b), c));
            }
        }
    }
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut corners: Vec<Point> = Vec::new();
    for (_, c) in out {
        if corners.iter().all(|k| k.distance(c) > CORNER_SPAN_MM) {
            corners.push(c);
        }
    }
    corners
}

fn flat(p: &Path) -> Vec<Point> {
    p.flatten(0.02).into_iter().flat_map(|l| l.points).collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// The area that a part covers: the quads between the pairs of a column,
/// or the outline of a fill.
enum Cover {
    Quads(Vec<[Point; 4]>, Option<BBox>),
    Fill(Outline),
    None,
}

impl Cover {
    fn of(part: &ObjectKind) -> Cover {
        match part {
            ObjectKind::Satin { geometry, .. } => {
                let Some(c) = Column::new(geometry) else {
                    return Cover::None;
                };
                let p = c.pairs(0.1);
                let quads: Vec<[Point; 4]> = p.windows(2).map(|w| [w[0].0, w[1].0, w[1].1, w[0].1]).collect();
                let bb = BBox::from_points(quads.iter().flatten().copied());
                Cover::Quads(quads, bb)
            }
            ObjectKind::Fill { outer, holes, .. } => Cover::Fill(vec![(flat(outer), holes.iter().map(flat).collect())]),
            _ => Cover::None,
        }
    }

    fn holds(&self, p: Point) -> bool {
        match self {
            Cover::Quads(q, Some(bb)) => {
                p.x >= bb.min.x
                    && p.x <= bb.max.x
                    && p.y >= bb.min.y
                    && p.y <= bb.max.y
                    && q.iter().any(|q| ring_contains(q, p))
            }
            Cover::Fill(o) => inside(o, p),
            _ => false,
        }
    }
}

/// The faults of the parts of a letter (in sew order, in mm) against its
/// outline.
pub fn measure(name: &str, parts: &[ObjectKind], o: &Outline) -> Letter {
    let mut f = Faults {
        letters: 1,
        ..Default::default()
    };
    let params = SatinParams::default();
    let bare = SatinParams {
        pull_compensation_mm: 0.0,
        ..params.clone()
    };
    let covers: Vec<Cover> = parts.iter().map(Cover::of).collect();
    let mut needles = Vec::new();
    let mut spikes = Vec::new();
    let mut entry = Point::new(-5.0, 0.0);
    for part in parts {
        match part {
            ObjectKind::Satin { geometry, .. } => {
                if let Some(pts) = column_faults(geometry, &params, &bare, o, entry, &mut f, &mut spikes) {
                    if let Some(l) = pts.last() {
                        entry = *l;
                    }
                    needles.push(json!({"kind": "satin", "pts": pts.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()}));
                }
            }
            ObjectKind::Running { path, .. } => {
                let pts = flat(path);
                for w in pts.windows(2) {
                    let n = (w[0].distance(w[1]) / 0.05).ceil().max(1.0) as usize;
                    for k in 0..n {
                        let p = w[0].lerp(w[1], (k as f64 + 0.5) / n as f64);
                        if outside_by(o, p) > RAIL_MM {
                            f.run_out_mm += w[0].distance(w[1]) / n as f64;
                        }
                    }
                }
                if let Some(l) = pts.last() {
                    entry = *l;
                }
                needles.push(json!({"kind": "run", "pts": pts.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()}));
            }
            _ => {}
        }
    }
    // Joints: the middle of an end of a column in another part.
    for (j, part) in parts.iter().enumerate() {
        let ObjectKind::Satin { geometry, .. } = part else {
            continue;
        };
        let Some(col) = Column::new(geometry) else { continue };
        let (s, e) = (col.start(), col.end());
        for end in [s.0.midpoint(s.1), e.0.midpoint(e.1)] {
            for (i, c) in covers.iter().enumerate() {
                if i != j && matches!(c, Cover::Quads(..)) && c.holds(end) {
                    if i < j {
                        f.on_top += 1;
                    } else {
                        f.under += 1;
                    }
                }
            }
        }
    }
    let (open, corners) = open_area(o, &covers, &mut f);
    Letter {
        name: name.to_string(),
        picture: json!({
            "name": name,
            "outline": rings(o).map(|r| r.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "parts": needles,
            "spikes": spikes,
            "open": open,
            "corners": corners.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
            "faults": {
                "spikes": f.spike_end + f.spike_mid,
                "bare": f.bare_out,
                "cross": f.cross,
                "back_mm": f.back_mm,
                "bulge_mm": f.bulge_mm,
                "notch": f.notch,
                "open_corner": f.open_corner,
                "open_other": f.open_other,
                "corners_open": f.corners_open,
            },
        }),
        faults: f,
    }
}

/// The faults of one column; its needle points.
fn column_faults(
    g: &SatinGeometry,
    params: &SatinParams,
    bare: &SatinParams,
    o: &Outline,
    entry: Point,
    f: &mut Faults,
    spikes: &mut Vec<Value>,
) -> Option<Vec<Point>> {
    let col = Column::new(g)?;
    let pairs = col.pairs(0.2);
    let widths: Vec<f64> = pairs.iter().map(|(a, b)| a.distance(*b)).collect();
    let med = median(widths.clone());
    let mids: Vec<Point> = pairs.iter().map(|(a, b)| a.midpoint(*b)).collect();
    let mut along = vec![0.0];
    for w in mids.windows(2) {
        along.push(along[along.len() - 1] + w[0].distance(w[1]));
    }
    let total = *along.last()?;
    let near_end = |s: f64| s < med || total - s < med;
    for (i, (a, b)) in pairs.iter().enumerate() {
        for p in [a, b] {
            if outside_by(o, *p) > RAIL_MM {
                if near_end(along[i]) {
                    f.rail_out_end += 1;
                } else {
                    f.rail_out_mid += 1;
                }
            }
        }
        let Some(&(a1, b1)) = pairs.get(i + 1) else { continue };
        // Two pairs that cross (not two pairs with one point: a pivot).
        if let Some((u, v)) = Point::crossing(*a, *b, a1, b1)
            && (1e-3..=1.0 - 1e-3).contains(&u)
            && (1e-3..=1.0 - 1e-3).contains(&v)
        {
            f.cross += 1;
        }
        let (lo, hi) = (i.saturating_sub(3), (i + 4).min(pairs.len() - 1));
        let step = mids[i].distance(mids[i + 1]).max(0.05);
        if let Some(t) = (mids[hi] - mids[lo]).normalized()
            && ((a1 - *a).dot(t) < -0.02 || (b1 - *b).dot(t) < -0.02)
        {
            f.back_mm += step;
        }
        // Needle points on a side are two pairs apart.
        if 2.0 * a.distance(a1).max(b.distance(b1)) > FAN_MM {
            f.fan_mm += step;
        }
        if widths[i] > BULGE * med && !near_end(along[i]) {
            f.bulge_mm += mids[i].distance(mids[i + 1]);
        }
    }
    if let Some(pts) = satin(g, bare, entry, None) {
        f.bare_out += pts.iter().filter(|p| outside_by(o, **p) > BARE_MM).count();
    }
    let pts = satin(g, params, entry, None)?;
    for p in &pts {
        let out = outside_by(o, *p);
        if out > SPIKE_MM {
            let k = (0..mids.len())
                .min_by(|x, y| mids[*x].distance(*p).total_cmp(&mids[*y].distance(*p)))
                .unwrap_or(0);
            if near_end(along[k]) {
                f.spike_end += 1;
            } else {
                f.spike_mid += 1;
            }
            spikes.push(json!([p.x, p.y, out]));
        }
    }
    for w in pts.windows(2) {
        let ends = outside_by(o, w[0]).max(outside_by(o, w[1]));
        let worst = (1..10)
            .map(|k| outside_by(o, w[0].lerp(w[1], f64::from(k) / 10.0)))
            .fold(0.0, f64::max);
        if worst > 0.3 && worst > ends + 0.2 {
            f.notch += 1;
        }
    }
    Some(pts)
}

/// The open area of the outline and its convex corners (with the open
/// cells for a picture).
fn open_area(o: &Outline, covers: &[Cover], f: &mut Faults) -> (Vec<[f64; 2]>, Vec<Point>) {
    let Some(bb) = BBox::from_points(rings(o).flatten().copied()) else {
        return (Vec::new(), Vec::new());
    };
    let corners = convex_corners(o);
    f.corners = corners.len();
    let mut near_in = vec![0usize; corners.len()];
    let mut near_cov = vec![0usize; corners.len()];
    let mut open = Vec::new();
    // The cells of the outline and of the parts, filled on one grid.
    let grid = Grid::new(bb, CELL_MM, 1);
    let cell = grid.cell;
    let glyph = emb_letter::tune::outline_mask(&grid, o);
    let mut cover = vec![false; grid.w * grid.h];
    for c in covers {
        match c {
            Cover::Quads(quads, _) => {
                for q in quads {
                    emb_letter::grid::fill_into(&grid, &[q.to_vec()], &mut cover);
                }
            }
            Cover::Fill(fo) => {
                for (k, m) in emb_letter::tune::outline_mask(&grid, fo).into_iter().enumerate() {
                    cover[k] |= m;
                }
            }
            Cover::None => {}
        }
    }
    for j in 0..grid.h {
        for i in 0..grid.w {
            let k = grid.idx(i, j);
            if !glyph[k] {
                continue;
            }
            let p = grid.center(i, j);
            f.area += cell * cell;
            let covered = cover[k];
            let mut at_corner = false;
            for (k, c) in corners.iter().enumerate() {
                let d = c.distance(p);
                if d < CORNER_NEAR_MM {
                    near_in[k] += 1;
                    near_cov[k] += usize::from(covered);
                }
                at_corner |= d < CORNER_AREA_MM;
            }
            if !covered {
                if at_corner {
                    f.open_corner += cell * cell;
                } else {
                    f.open_other += cell * cell;
                }
                if (i + j) % 2 == 0 {
                    open.push([p.x, p.y]);
                }
            }
        }
    }
    f.corners_open = (0..corners.len())
        .filter(|&k| near_in[k] > 0 && (near_cov[k] as f64) < CORNER_COVER * near_in[k] as f64)
        .count();
    (open, corners)
}

/// The letter of `c` that lettering in the TrueType font `src` gets: the
/// columns of the stroke finder (or the fill letter if they do not cover
/// it), routed, in mm at [`CAP_MM`].
pub fn truetype_letter(src: &Font, c: char) -> Option<EmbLetter> {
    let g = src.glyph(c)?;
    let shapes = src.shapes(g);
    let s = CAP_MM / src.cap_height;
    let mut parts = if shapes.columns_cover() {
        shapes.satin_parts()
    } else {
        shapes.fill_parts()
    };
    for p in &mut parts {
        p.transform(&Transform::scale(s, s));
    }
    let letter = EmbLetter {
        advance_mm: src.advance(g) * s,
        parts,
        fixed: false,
        tuning: Vec::new(),
        fill_parts: Vec::new(),
    };
    Some(emb_letter::route::route_with_splits(&letter, CAP_MM).0)
}

/// The outline of `c` in `src`, in mm at [`CAP_MM`].
pub fn outline(src: &Font, c: char) -> Outline {
    let Some(g) = src.glyph(c) else { return Vec::new() };
    let s = CAP_MM / src.cap_height;
    let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect() };
    src.shapes(g)
        .fills
        .iter()
        .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect()))
        .collect()
}

/// Measures the fonts of [`FONTS`] in both ways. `truetype_only`: only the
/// TrueType path (the stroke finder of today).
pub fn run(truetype_only: bool) -> Vec<FontFaults> {
    run_on(truetype_only, &chars())
}

/// The letters of the snapshot test (`tests/letter_faults.rs`): letters
/// with corners, curves, crossings, junctions and slanted ends.
pub const SNAPSHOT_CHARS: &str = "AEGKMNRSVWXaegkmnrstwx249";

/// [`run`] for the characters `chars`.
pub fn run_on(truetype_only: bool, chars: &[char]) -> Vec<FontFaults> {
    let mut out = Vec::new();
    for (family, src_family, src_style) in FONTS {
        let Some(src) = emb_letter::bundled::font(src_family, src_style) else {
            continue;
        };
        let font: Option<EmbFont> = emb_letter::bundled::embroidery(family);
        let ways: &[&'static str] = if truetype_only {
            &["truetype"]
        } else {
            &["embroidery", "truetype"]
        };
        for &way in ways {
            let mut sum = Faults::default();
            let mut letters = Vec::new();
            for &c in chars {
                let o = outline(&src, c);
                if o.is_empty() {
                    continue;
                }
                let letter = match way {
                    "embroidery" => font.as_ref().and_then(|f| f.letters.get(&c).cloned()),
                    _ => truetype_letter(&src, c),
                };
                let Some(letter) = letter else { continue };
                if !letter.parts.iter().any(|p| matches!(p, ObjectKind::Satin { .. })) {
                    continue;
                }
                let tag = if way == "embroidery" { "emb" } else { "ttf" };
                let l = measure(&format!("{family} {tag} {c}"), &letter.parts, &o);
                sum.add(&l.faults);
                letters.push(l);
            }
            out.push(FontFaults {
                family: family.to_string(),
                way,
                sum,
                letters,
            });
        }
    }
    out
}

/// The measures as a table, and the worst letters of each measure.
pub fn report(fonts: &[FontFaults]) -> String {
    let mut s = table(fonts);
    s.push_str(&worst(fonts));
    s
}

/// The measures of each font as a table.
pub fn table(fonts: &[FontFaults]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "At {CAP_MM} mm, default satin settings. Spikes: needle points over {SPIKE_MM} mm outside (at ends / in the middle), and over {BARE_MM} mm with no pull compensation.\n\n"
    ));
    s.push_str("| Font | Way | Spikes | Bare | Rail pts out (ends / middle) | Crossing | Back mm | Fans mm | Bulges mm | Notches | Runs out mm | Open % (corners / other mm2) | Corners open | Ends on top / under |\n");
    s.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for f in fonts {
        let t = &f.sum;
        s.push_str(&format!(
            "| {} | {} | {} / {} | {} | {} / {} | {} | {:.1} | {:.1} | {:.1} | {} | {:.1} | {:.2} ({:.1} / {:.1}) | {} of {} | {} / {} |\n",
            f.family,
            f.way,
            t.spike_end,
            t.spike_mid,
            t.bare_out,
            t.rail_out_end,
            t.rail_out_mid,
            t.cross,
            t.back_mm,
            t.fan_mm,
            t.bulge_mm,
            t.notch,
            t.run_out_mm,
            t.open_percent(),
            t.open_corner,
            t.open_other,
            t.corners_open,
            t.corners,
            t.on_top,
            t.under,
        ));
    }
    s
}

/// The letters with the most faults of each measure.
fn worst(fonts: &[FontFaults]) -> String {
    let mut s = String::new();
    let all: Vec<&Letter> = fonts.iter().flat_map(|f| f.letters.iter()).collect();
    type Key = fn(&Faults) -> f64;
    let measures: [(&str, Key); 8] = [
        ("spikes", |f| (f.spike_end + f.spike_mid) as f64),
        ("crossing and back", |f| f.cross as f64 + f.back_mm),
        ("bulges (mm)", |f| f.bulge_mm),
        ("open at corners (mm2)", |f| f.open_corner),
        ("open elsewhere (mm2)", |f| f.open_other),
        ("corners open", |f| f.corners_open as f64),
        ("notches", |f| f.notch as f64),
        ("runs outside (mm)", |f| f.run_out_mm),
    ];
    for (what, key) in measures {
        let mut v: Vec<&&Letter> = all.iter().filter(|l| key(&l.faults) > 0.0).collect();
        v.sort_by(|a, b| key(&b.faults).total_cmp(&key(&a.faults)));
        let list: Vec<String> = v
            .iter()
            .take(10)
            .map(|l| format!("{} ({:.1})", l.name, key(&l.faults)))
            .collect();
        s.push_str(&format!("\nWorst {what}: {}", list.join(", ")));
    }
    s.push('\n');
    s
}

/// The pictures of all letters as JSON, for `tools/letter-faults.py`.
pub fn pictures(fonts: &[FontFaults]) -> String {
    let all: Vec<&Value> = fonts
        .iter()
        .flat_map(|f| f.letters.iter().map(|l| &l.picture))
        .collect();
    serde_json::to_string(&all).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    fn column(a: (Point, Point), b: (Point, Point)) -> ObjectKind {
        ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a: Path::new().move_to(a.0).line_to(a.1),
                rail_b: Path::new().move_to(b.0).line_to(b.1),
                rungs: Vec::new(),
            },
            params: SatinParams::default(),
        }
    }

    #[test]
    fn a_column_on_its_outline_has_no_faults_and_one_out_of_it_has_spikes() {
        let outline = vec![(bar(0.0, 20.0, 0.0, 4.0), Vec::new())];
        let good = column(
            (Point::new(0.0, 0.0), Point::new(20.0, 0.0)),
            (Point::new(0.0, 4.0), Point::new(20.0, 4.0)),
        );
        let f = measure("bar", &[good], &outline).faults;
        assert_eq!((f.spike_end + f.spike_mid, f.bare_out, f.cross), (0, 0, 0), "{f:?}");
        assert!(f.open_percent() < 0.5, "{f:?}");
        assert_eq!(f.corners, 4);
        assert_eq!(f.corners_open, 0);
        // The end goes 1 mm past the outline, and 3 mm of the other end is open.
        let bad = column(
            (Point::new(3.0, 0.0), Point::new(21.0, 0.0)),
            (Point::new(3.0, 4.0), Point::new(21.0, 4.0)),
        );
        let f = measure("bar", &[bad], &outline).faults;
        assert!(f.spike_end > 0 && f.bare_out > 0, "{f:?}");
        assert!(f.rail_out_end > 0 && f.rail_out_mid == 0, "{f:?}");
        assert_eq!(f.corners_open, 2, "{f:?}");
        assert!((f.open_corner + f.open_other - 12.0).abs() < 0.5, "{f:?}");
    }
}
