//! Test swatches (P22 Milestone 3, ADR 0170): copies of some objects in a
//! grid, with one setting in steps from one value to another and the value
//! as lettering under each copy. The user sews them on the fabric of the
//! design and keeps the setting that looks best.
//!
//! Not a port. The idea is from zigzag (an open-source program under
//! Apache-2.0); the code is our own. The module is in `emb-letter` because
//! the labels are lettering (`layout::relayout`).

use emb_geom::path::PathCmd;
use emb_geom::{BBox, Path, Point, Polyline, Transform, text};
use emb_model::{Design, LetterStitch, Lettering, ObjectId, ObjectKind, SatinGeometry, Underlay};

use crate::layout::relayout;
use crate::source::LetterSource;

/// The space between two copies, mm.
pub const GAP_MM: f64 = 8.0;
/// The space between a copy and its label, mm.
const LABEL_GAP_MM: f64 = 3.0;
/// The capital height of the labels, at least, mm.
pub const LABEL_MM: f64 = 5.0;
/// The most copies.
pub const MAX_COPIES: usize = 24;

/// The setting that changes from copy to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SwatchSetting {
    /// The spacing of the rows of a fill or of the stitches of a column.
    #[default]
    Spacing,
    /// The stitch length of a fill or of a line.
    StitchLength,
    /// The pull compensation of a fill or of a column.
    PullCompensation,
    /// The width of a satin column or of a line with a border.
    Width,
    /// The angle of the rows of a fill.
    Angle,
    /// The underlay: none, light, auto, heavy, double.
    Underlay,
}

/// The underlay of each copy for [`SwatchSetting::Underlay`].
pub const UNDERLAYS: [Underlay; 5] = [
    Underlay::None,
    Underlay::Light,
    Underlay::Auto,
    Underlay::Heavy,
    Underlay::Double,
];

impl SwatchSetting {
    pub const ALL: [SwatchSetting; 6] = [
        SwatchSetting::Spacing,
        SwatchSetting::StitchLength,
        SwatchSetting::PullCompensation,
        SwatchSetting::Width,
        SwatchSetting::Angle,
        SwatchSetting::Underlay,
    ];

    /// The name (English; the program shows it with `tr`).
    pub fn name(self) -> &'static str {
        match self {
            SwatchSetting::Spacing => "Spacing",
            SwatchSetting::StitchLength => "Stitch length",
            SwatchSetting::PullCompensation => "Pull compensation",
            SwatchSetting::Width => "Width",
            SwatchSetting::Angle => "Angle",
            SwatchSetting::Underlay => "Underlay",
        }
    }

    /// The values are lengths, in mm.
    pub fn is_length(self) -> bool {
        !matches!(self, SwatchSetting::Angle | SwatchSetting::Underlay)
    }

    /// The range of the values that make sense, for the fields of the window.
    pub fn range(self) -> (f64, f64) {
        match self {
            SwatchSetting::Spacing => (0.15, 5.0),
            SwatchSetting::StitchLength => (1.0, 12.0),
            SwatchSetting::PullCompensation => (0.0, 1.5),
            SwatchSetting::Width => (0.5, 12.0),
            SwatchSetting::Angle => (-180.0, 180.0),
            SwatchSetting::Underlay => (0.0, (UNDERLAYS.len() - 1) as f64),
        }
    }

    /// The value of the setting in `kind`, if it has the setting.
    pub fn value_in(self, kind: &ObjectKind) -> Option<f64> {
        use SwatchSetting::*;
        match (self, kind) {
            (Spacing, ObjectKind::Fill { params, .. }) => Some(params.spacing_mm),
            (StitchLength, ObjectKind::Fill { params, .. }) => Some(params.stitch_length_mm),
            (PullCompensation, ObjectKind::Fill { params, .. }) => Some(params.pull_compensation_mm),
            (Angle, ObjectKind::Fill { params, .. }) => Some(params.angle_deg),
            (Underlay, ObjectKind::Fill { params, .. }) => underlay_index(&params.underlay),
            (Spacing, ObjectKind::Satin { params, .. }) => Some(params.spacing_mm),
            (PullCompensation, ObjectKind::Satin { params, .. }) => Some(params.pull_compensation_mm),
            (Width, ObjectKind::Satin { geometry, .. }) => satin_width(geometry),
            (Underlay, ObjectKind::Satin { params, .. }) => underlay_index(&params.underlay),
            (StitchLength, ObjectKind::Running { params, .. }) if params.border.is_none() => {
                Some(params.stitch_length_mm)
            }
            (Width, ObjectKind::Running { params, .. }) => params.border.as_ref().map(|b| b.width_mm),
            (Spacing, ObjectKind::Running { params, .. }) => params.border.as_ref().map(|b| b.column.spacing_mm),
            (PullCompensation, ObjectKind::Running { params, .. }) => {
                params.border.as_ref().map(|b| b.column.pull_compensation_mm)
            }
            (Underlay, ObjectKind::Running { params, .. }) => {
                params.border.as_ref().and_then(|b| underlay_index(&b.column.underlay))
            }
            (Spacing, ObjectKind::Lettering(l)) => Some(l.satin.spacing_mm),
            (PullCompensation, ObjectKind::Lettering(l)) => Some(l.satin.pull_compensation_mm),
            (Underlay, ObjectKind::Lettering(l)) => underlay_index(&l.satin.underlay),
            _ => None,
        }
    }

    /// Sets the setting of `kind` to `v`. `false` if `kind` does not have it.
    pub fn set(self, kind: &mut ObjectKind, v: f64) -> bool {
        use SwatchSetting::*;
        let underlay = || UNDERLAYS[(v.round().max(0.0) as usize).min(UNDERLAYS.len() - 1)].clone();
        match (self, kind) {
            (Spacing, ObjectKind::Fill { params, .. }) => params.spacing_mm = v,
            (StitchLength, ObjectKind::Fill { params, .. }) => params.stitch_length_mm = v,
            (PullCompensation, ObjectKind::Fill { params, .. }) => params.pull_compensation_mm = v,
            (Angle, ObjectKind::Fill { params, .. }) => params.angle_deg = v,
            (Underlay, ObjectKind::Fill { params, .. }) => params.underlay = underlay(),
            (Spacing, ObjectKind::Satin { params, .. }) => params.spacing_mm = v,
            (PullCompensation, ObjectKind::Satin { params, .. }) => params.pull_compensation_mm = v,
            (Width, ObjectKind::Satin { geometry, .. }) => {
                let Some(w) = satin_width(geometry).filter(|w| *w > 1e-6) else {
                    return false;
                };
                *geometry = widened(geometry, v / w);
            }
            (Underlay, ObjectKind::Satin { params, .. }) => params.underlay = underlay(),
            (StitchLength, ObjectKind::Running { params, .. }) if params.border.is_none() => {
                params.stitch_length_mm = v;
            }
            (Width | Spacing | PullCompensation | Underlay, ObjectKind::Running { params, .. }) => {
                let Some(b) = params.border.as_mut() else {
                    return false;
                };
                match self {
                    Width => b.width_mm = v,
                    Spacing => b.column.spacing_mm = v,
                    PullCompensation => b.column.pull_compensation_mm = v,
                    _ => b.column.underlay = underlay(),
                }
            }
            (Spacing | PullCompensation | Underlay, ObjectKind::Lettering(l)) => {
                match self {
                    Spacing => {
                        l.satin.spacing_mm = v;
                        l.fill.spacing_mm = v;
                    }
                    PullCompensation => {
                        l.satin.pull_compensation_mm = v;
                        l.fill.pull_compensation_mm = v;
                    }
                    _ => {
                        l.satin.underlay = underlay();
                        l.fill.underlay = underlay();
                    }
                }
                l.apply_params();
            }
            _ => return false,
        }
        true
    }
}

fn underlay_index(u: &Underlay) -> Option<f64> {
    UNDERLAYS.iter().position(|x| x == u).map(|i| i as f64)
}

/// The mean width of a satin column: the mean distance from the points of
/// one rail to the other rail.
pub fn satin_width(g: &SatinGeometry) -> Option<f64> {
    let a: Vec<Point> = g.rail_a.flatten(0.1).into_iter().flat_map(|p| p.points).collect();
    let b: Vec<Polyline> = g.rail_b.flatten(0.1);
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let d: f64 = a.iter().map(|p| p.distance(nearest(&b, *p))).sum();
    Some(d / a.len() as f64)
}

/// The nearest point of `lines` to `p`.
fn nearest(lines: &[Polyline], p: Point) -> Point {
    let mut best = (f64::INFINITY, p);
    for l in lines {
        for w in l.points.windows(2) {
            let q = p.nearest_on_segment(w[0], w[1]);
            let d = q.distance(p);
            if d < best.0 {
                best = (d, q);
            }
        }
        if l.points.len() == 1 {
            best = (best.0.min(l.points[0].distance(p)), l.points[0]);
        }
    }
    best.1
}

/// The column `g` with its width times `k`: each point of a rail moves
/// from the middle between it and the other rail.
pub fn widened(g: &SatinGeometry, k: f64) -> SatinGeometry {
    let (fa, fb) = (g.rail_a.flatten(0.1), g.rail_b.flatten(0.1));
    let from = |p: Point, other: &[Polyline]| {
        let m = p.midpoint(nearest(other, p));
        m + (p - m) * k
    };
    let rail = |path: &Path, other: &[Polyline]| Path {
        cmds: path
            .cmds
            .iter()
            .map(|c| match *c {
                PathCmd::MoveTo(p) => PathCmd::MoveTo(from(p, other)),
                PathCmd::LineTo(p) => PathCmd::LineTo(from(p, other)),
                PathCmd::CubicTo(a, b, p) => PathCmd::CubicTo(from(a, other), from(b, other), from(p, other)),
                PathCmd::Close => PathCmd::Close,
            })
            .collect(),
    };
    SatinGeometry {
        rail_a: rail(&g.rail_a, &fb),
        rail_b: rail(&g.rail_b, &fa),
        rungs: g
            .rungs
            .iter()
            .map(|&(a, b)| {
                let m = a.midpoint(b);
                (m + (a - m) * k, m + (b - m) * k)
            })
            .collect(),
    }
}

/// What the swatches change, and where they go.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchSpec {
    pub setting: SwatchSetting,
    /// The value of the first copy and of the last copy (for
    /// [`SwatchSetting::Underlay`], indices in [`UNDERLAYS`]).
    pub from: f64,
    pub to: f64,
    pub copies: usize,
    /// The width and the height of the field of the hoop, mm: the grid has
    /// as many columns as fit. `None`: one row.
    pub field: Option<(f64, f64)>,
}

impl SwatchSpec {
    /// The value of each copy, in equal steps.
    pub fn values(&self) -> Vec<f64> {
        if self.setting == SwatchSetting::Underlay {
            return (0..UNDERLAYS.len()).map(|i| i as f64).collect();
        }
        let n = self.copies.clamp(1, MAX_COPIES);
        if n == 1 {
            return vec![self.from];
        }
        (0..n)
            .map(|i| self.from + (self.to - self.from) * i as f64 / (n - 1) as f64)
            .collect()
    }
}

/// The swatches: a new design, and its grid.
#[derive(Debug, Clone)]
pub struct Swatches {
    pub design: Design,
    pub values: Vec<f64>,
    pub columns: usize,
    pub rows: usize,
    /// The box of each copy (without its label) and of each label.
    pub copies: Vec<BBox>,
    pub labels: Vec<BBox>,
    /// The size of all of it, mm.
    pub size: (f64, f64),
    /// It fits the field of the hoop.
    pub fits: bool,
}

/// The text of a label: the value of the setting (lengths with
/// `length`, which gives the units of the program).
pub fn label_text(setting: SwatchSetting, v: f64, length: &dyn Fn(f64) -> String) -> String {
    match setting {
        SwatchSetting::Angle => format!("{v:.0}°"),
        SwatchSetting::Underlay => {
            let u = &UNDERLAYS[(v.round().max(0.0) as usize).min(UNDERLAYS.len() - 1)];
            text::tr(match u {
                Underlay::None => "None",
                Underlay::Light => "Light",
                Underlay::Heavy => "Heavy",
                Underlay::Double => "Double",
                _ => "Auto",
            })
            .to_string()
        }
        _ => length(v),
    }
}

/// The test swatches of the objects `ids` of `design`: a new design with a
/// copy of them for each value of `spec`, in a grid round the center of
/// the hoop, each with the label of its value under it (in the font
/// `label`; `None`: no labels). The objects that do not have the setting
/// are in each copy as they are.
pub fn swatches(
    design: &Design,
    ids: &[ObjectId],
    spec: &SwatchSpec,
    label: Option<&dyn LetterSource>,
    length: &dyn Fn(f64) -> String,
) -> Result<Swatches, String> {
    let objects: Vec<_> = ids.iter().filter_map(|id| design.object(*id)).cloned().collect();
    if objects.is_empty() {
        return Err(text::tr("Select the objects for the swatches.").into());
    }
    if !objects.iter().any(|o| spec.setting.value_in(&o.kind).is_some()) {
        return Err(text::fill(
            "No selected object has the setting {}.",
            &[&text::tr(spec.setting.name())],
        ));
    }
    let group = objects
        .iter()
        .filter_map(|o| o.control_bbox())
        .reduce(BBox::union)
        .ok_or_else(|| text::tr("The objects have no size.").to_string())?;
    let values = spec.values();
    // The labels first: their sizes make the cells.
    let label_height = label.map_or(LABEL_MM, |src| {
        src.size_range(LetterStitch::Satin)
            .map_or(LABEL_MM, |(min, _)| min.max(LABEL_MM))
    });
    let labels: Vec<Option<Lettering>> = values
        .iter()
        .map(|&v| {
            let src = label?;
            let mut l = Lettering::new(
                &label_text(spec.setting, v, length),
                src.font_ref(),
                label_height,
                Point::ZERO,
            );
            relayout(&mut l, src);
            Some(l)
        })
        .collect();
    let label_box = |l: &Lettering| l.parts().filter_map(|p| p.control_bbox()).reduce(BBox::union);
    let label_size = labels
        .iter()
        .flatten()
        .filter_map(label_box)
        .fold((0.0_f64, 0.0_f64), |(w, h), b| (w.max(b.width()), h.max(b.height())));
    let cell_w = group.width().max(label_size.0) + GAP_MM;
    let cell_h = group.height()
        + if label.is_some() {
            LABEL_GAP_MM + label_size.1
        } else {
            0.0
        }
        + GAP_MM;
    let n = values.len();
    let columns = match spec.field {
        Some((w, _)) => (((w + GAP_MM) / cell_w).floor() as usize).clamp(1, n),
        None => n,
    };
    let rows = n.div_ceil(columns);
    let size = (columns as f64 * cell_w - GAP_MM, rows as f64 * cell_h - GAP_MM);
    let fits = spec.field.is_none_or(|(w, h)| size.0 <= w + 1e-9 && size.1 <= h + 1e-9);

    let mut out = Design::new(text::fill("{} swatches", &[&text::tr(spec.setting.name())]));
    out.threads = design.threads.clone();
    out.machine = design.machine.clone();
    out.hoop = design.hoop.clone();
    out.fabric = design.fabric.clone();
    out.fabric_mix = design.fabric_mix;
    out.no_topping = design.no_topping;
    out.background = design.background;
    out.trim_gap_mm = design.trim_gap_mm;
    let center = design.hoop.center;
    let origin = Point::new(center.x - size.0 / 2.0, center.y - size.1 / 2.0);
    let label_thread = objects[0].thread;
    let (mut copy_boxes, mut label_boxes) = (Vec::new(), Vec::new());
    for (i, (&v, l)) in values.iter().zip(labels).enumerate() {
        let (col, row) = (i % columns, i / columns);
        let cell = Point::new(origin.x + col as f64 * cell_w, origin.y + row as f64 * cell_h);
        // The copy, in the middle of the width of its cell.
        let width = cell_w - GAP_MM;
        let move_copy = Transform::translate(
            cell.x + (width - group.width()) / 2.0 - group.min.x,
            cell.y - group.min.y,
        );
        let mut copy_box: Option<BBox> = None;
        for o in &objects {
            let mut kind = o.kind.clone();
            spec.setting.set(&mut kind, v);
            kind.transform(&move_copy);
            copy_box = match (copy_box, kind.control_bbox()) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (a, b) => a.or(b),
            };
            let id = out.add_object(o.thread, kind);
            if let Some(new) = out.object_mut(id) {
                new.name = o.name.clone();
                new.functions = o.functions;
            }
        }
        copy_boxes.extend(copy_box);
        if let Some(l) = l
            && let Some(b) = label_box(&l)
        {
            let top = cell.y + group.height() + LABEL_GAP_MM;
            let t = Transform::translate(cell.x + (width - b.width()) / 2.0 - b.min.x, top - b.min.y);
            let mut kind = ObjectKind::Lettering(Box::new(l));
            kind.transform(&t);
            if let ObjectKind::Lettering(l) = &kind
                && let Some(b) = label_box(l)
            {
                label_boxes.push(b);
            }
            out.add_object(label_thread, kind);
        }
    }
    // The copies do not overlap: their colors sew together.
    out.optimize_color_changes();
    Ok(Swatches {
        design: out,
        values,
        columns,
        rows,
        copies: copy_boxes,
        labels: label_boxes,
        size,
        fits,
    })
}

#[cfg(test)]
mod tests {
    use emb_model::{FillParams, Rgb, SatinParams};

    use super::*;

    fn mm(v: f64) -> String {
        format!("{v:.2} mm")
    }

    fn label_font() -> crate::embfont::EmbFont {
        crate::bundled::embroidery("Atkinson Hyperlegible").expect("a bundled font for small text")
    }

    fn satin() -> (Design, ObjectId) {
        let mut d = Design::new("satin");
        let t = d.add_thread("Red", Rgb(200, 30, 30));
        let center = Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(30.0, 0.0));
        let id = d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry::from_centerline(&center, 4.0),
                params: SatinParams::default(),
            },
        );
        (d, id)
    }

    /// The exit test of Milestone 3: five copies of a satin with spacing
    /// from 0.30 mm to 0.50 mm have these spacings and these labels, and
    /// no label touches a copy.
    #[test]
    fn five_copies_of_a_satin_with_their_spacings_and_labels() {
        let (d, id) = satin();
        let spec = SwatchSpec {
            setting: SwatchSetting::Spacing,
            from: 0.30,
            to: 0.50,
            copies: 5,
            field: Some((200.0, 260.0)),
        };
        let font = label_font();
        let s = swatches(&d, &[id], &spec, Some(&font), &mm).unwrap();
        let spacings: Vec<f64> = s
            .design
            .objects()
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Satin { params, .. } => Some(params.spacing_mm),
                _ => None,
            })
            .collect();
        for (a, b) in spacings.iter().zip([0.30, 0.35, 0.40, 0.45, 0.50]) {
            assert!((a - b).abs() < 1e-9, "{spacings:?}");
        }
        let labels: Vec<String> = s
            .design
            .objects()
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Lettering(l) => Some(l.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["0.30 mm", "0.35 mm", "0.40 mm", "0.45 mm", "0.50 mm"]);
        assert_eq!(s.copies.len(), 5);
        assert_eq!(s.labels.len(), 5);
        for l in &s.labels {
            for c in &s.copies {
                assert!(!l.intersects(c), "a label touches a copy: {l:?} {c:?}");
            }
        }
        assert!(s.fits);
        assert!(s.columns * s.rows >= 5);
    }

    #[test]
    fn each_setting_changes_the_kinds_that_have_it() {
        let (mut d, satin) = satin();
        let t = d.threads[0].id;
        let fill = d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(0.0, 10.0), Point::new(20.0, 30.0))),
                holes: Vec::new(),
                params: FillParams::default(),
            },
        );
        let line = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new()
                    .move_to(Point::new(0.0, 40.0))
                    .line_to(Point::new(20.0, 40.0)),
                params: emb_model::RunningParams::default(),
            },
        );
        let has = |id: ObjectId, s: SwatchSetting| s.value_in(&d.object(id).unwrap().kind).is_some();
        use SwatchSetting::*;
        assert!(has(fill, Spacing) && has(fill, StitchLength) && has(fill, PullCompensation));
        assert!(has(fill, Angle) && has(fill, Underlay) && !has(fill, Width));
        assert!(has(satin, Spacing) && has(satin, PullCompensation) && has(satin, Width));
        assert!(has(satin, Underlay) && !has(satin, Angle) && !has(satin, StitchLength));
        assert!(has(line, StitchLength) && !has(line, Spacing) && !has(line, Width));
        for s in SwatchSetting::ALL {
            for id in [satin, fill, line] {
                let Some(v) = s.value_in(&d.object(id).unwrap().kind) else {
                    continue;
                };
                // A new value: the kind has it after `set`.
                let new = if s == Underlay { (v + 1.0) % 5.0 } else { v * 1.5 + 1.0 };
                let mut k = d.object(id).unwrap().kind.clone();
                assert!(s.set(&mut k, new), "{s:?} {id:?}");
                let got = s.value_in(&k).unwrap();
                let tol = if s == Width { 0.05 * new } else { 1e-9 };
                assert!((got - new).abs() <= tol, "{s:?}: {got} for {new}");
            }
        }
    }

    #[test]
    fn a_grid_larger_than_the_hoop_does_not_fit() {
        let (d, id) = satin();
        let spec = SwatchSpec {
            setting: SwatchSetting::Width,
            from: 2.0,
            to: 8.0,
            copies: 24,
            field: Some((100.0, 100.0)),
        };
        let s = swatches(&d, &[id], &spec, None, &mm).unwrap();
        assert_eq!(s.columns, 2, "two 30 mm copies with the gap in 100 mm");
        assert!(!s.fits, "{:?}", s.size);
        // Underlay: one copy for each kind of underlay.
        let spec = SwatchSpec {
            setting: SwatchSetting::Underlay,
            ..spec
        };
        let s = swatches(&d, &[id], &spec, None, &mm).unwrap();
        assert_eq!(s.values.len(), UNDERLAYS.len());
    }
}
