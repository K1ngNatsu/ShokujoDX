//! The exit tests of P18 Milestones 1 and 2 (ADR 0131, ADR 0132): each
//! design of the corpus, written as DST and read again, is cut into parts
//! (`emb_art::parts`) and made into objects (`emb_art::part_objects`).
//! Lettering is cut into its letter parts first (as Break Apart), so each
//! stitch has the kind of the part of a letter that made it.
//!
//! - Milestone 1: 90 % of the stitches of the objects of a kind that the
//!   parts know (runs, satin columns, fills of rows) are in a part of the
//!   kind of their source object.
//! - Milestone 2: the width of a recognized column is within 0.2 mm and its
//!   spacing within 10 % of its source object.
//!
//! `cargo test --release -p emb-cli --test parts -- --nocapture` prints the
//! numbers of each design.

use emb_art::part_objects::{Made, objects};
use emb_art::parts::{PartKind, kinds_of_records, parts};
use emb_cli::corpus;
use emb_geom::Point;
use emb_model::{
    ColumnPattern, Design, FillPattern, ObjectId, ObjectKind, SatinGeometry, SatinParams, StitchDesign, StitchKind,
};
use emb_stitch::StitchPlan;

/// The kind of part that the stitches of an object of `kind` are, if the
/// parts know it.
fn expected(kind: &ObjectKind) -> Option<PartKind> {
    let column = |c: &ColumnPattern| matches!(c, ColumnPattern::Satin | ColumnPattern::Zigzag);
    match kind {
        // A line with a border sews a column along it; a motif, a ripple
        // and cording are other kinds.
        ObjectKind::Running { params, .. } => match &params.border {
            Some(b) => column(&b.column.pattern).then_some(PartKind::Column),
            None if params.motif.is_some() || params.ripple.is_some() || params.cording => None,
            None => Some(PartKind::Run),
        },
        ObjectKind::Satin { params, .. } if column(&params.pattern) => Some(PartKind::Column),
        ObjectKind::Fill { params, .. } => match &params.pattern {
            FillPattern::Tatami
            | FillPattern::Square
            | FillPattern::ProgramSplit { .. }
            | FillPattern::UserSplit { .. } => Some(PartKind::Rows),
            FillPattern::Columns { column: c } if column(c) => Some(PartKind::Column),
            _ => None,
        },
        _ => None,
    }
}

/// The design with each lettering cut into its letter parts (in the place
/// of the lettering, with its thread).
fn with_letter_parts(d: &Design) -> Design {
    let mut out = d.clone();
    let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
    for id in ids {
        let Some(o) = out.object(id).cloned() else { continue };
        let ObjectKind::Lettering(l) = &o.kind else { continue };
        let parts: Vec<ObjectKind> = l.parts().cloned().collect();
        let at = out.objects().iter().position(|x| x.id == id).unwrap_or(0);
        out.remove_object(id);
        for (k, p) in parts.into_iter().enumerate() {
            out.insert_object(at + k, o.thread, p);
        }
    }
    out
}

/// A corpus design, its plan, and the plan written as DST and read again.
struct RoundTrip {
    design: Design,
    plan: StitchPlan,
    read: StitchDesign,
    /// For each record of the file: the record of the plan at its place.
    plan_of: Vec<Option<usize>>,
    /// For each record of the plan: its object.
    object_of: Vec<Option<ObjectId>>,
}

impl RoundTrip {
    fn new(name: &str) -> Option<RoundTrip> {
        let design = with_letter_parts(&corpus::build(name)?);
        let opts = emb_cli::plan_options(&design, None).ok()?;
        let plan = emb_cli::plan(&design, &opts);
        if plan.stitches.stitch_count() == 0 {
            return None;
        }
        let mut object_of = vec![None; plan.stitches.records.len()];
        for s in &plan.spans {
            for r in s.records.clone() {
                object_of[r] = Some(s.object);
            }
        }
        let bytes = emb_formats::write_machine("dst", &plan.stitches, name, "2026-11-19")?;
        let read = emb_formats::read_machine("dst", &bytes).ok()?;
        // Each stitch of the file is the next stitch of the plan at its
        // place (DST rounds to 0.1 mm and can cut a long stitch).
        let plan_stitches: Vec<(usize, Point)> = plan
            .stitches
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind == StitchKind::Stitch)
            .map(|(i, r)| (i, r.pos))
            .collect();
        let mut plan_of = vec![None; read.records.len()];
        let mut at = 0usize;
        for (i, r) in read.records.iter().enumerate() {
            if r.kind != StitchKind::Stitch {
                continue;
            }
            let near = plan_stitches[at..]
                .iter()
                .take(6)
                .position(|(_, p)| p.distance(r.pos) < 0.15);
            let Some(k) = near else { continue };
            at += k;
            plan_of[i] = Some(plan_stitches[at].0);
            at = (at + 1).min(plan_stitches.len().saturating_sub(1));
        }
        Some(RoundTrip {
            design,
            plan,
            read,
            plan_of,
            object_of,
        })
    }

    /// The source object of a record of the file.
    fn source(&self, record: usize) -> Option<ObjectId> {
        self.plan_of[record].and_then(|p| self.object_of[p])
    }

    /// The source object of most stitches of a made object, and the part of
    /// its stitches that are of it.
    fn source_of(&self, m: &Made) -> Option<(ObjectId, f64)> {
        let mut count: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
        let mut all = 0usize;
        for r in m.records.iter().flat_map(|r| r.clone()) {
            all += 1;
            if let Some(o) = self.source(r) {
                *count.entry(o).or_default() += 1;
            }
        }
        let (&o, &c) = count.iter().max_by_key(|(o, c)| (**c, std::cmp::Reverse(o.0)))?;
        Some((o, c as f64 / all.max(1) as f64))
    }
}

#[derive(Default)]
struct Count {
    right: usize,
    all: usize,
    /// Stitches of objects that the parts do not know.
    unknown: usize,
    /// For each kind: the stitches in a part of that kind, and all.
    by_kind: [(usize, usize); 4],
}

fn count(t: &RoundTrip) -> Count {
    let _ = &t.plan;
    let got = kinds_of_records(&parts(&t.read), t.read.records.len());
    let mut c = Count::default();
    for (i, g) in got.iter().enumerate() {
        let Some(o) = t.source(i) else { continue };
        match t.design.object(o).map(|o| expected(&o.kind)) {
            Some(Some(w)) => {
                c.all += 1;
                let k = PartKind::ALL.iter().position(|x| *x == w).unwrap_or(3);
                c.by_kind[k].1 += 1;
                if *g == Some(w) {
                    c.right += 1;
                    c.by_kind[k].0 += 1;
                }
            }
            Some(None) => c.unknown += 1,
            None => {}
        }
    }
    c
}

#[test]
fn the_stitches_of_the_corpus_are_in_parts_of_their_kind() {
    let mut total = Count::default();
    for name in corpus::NAMES {
        let Some(t) = RoundTrip::new(name) else { continue };
        let c = count(&t);
        if c.all > 0 {
            println!(
                "{name:<28} {:>6.1} % of {:>6} stitches ({} of other kinds)",
                100.0 * c.right as f64 / c.all as f64,
                c.all,
                c.unknown
            );
        }
        total.right += c.right;
        total.all += c.all;
        total.unknown += c.unknown;
        for k in 0..4 {
            total.by_kind[k].0 += c.by_kind[k].0;
            total.by_kind[k].1 += c.by_kind[k].1;
        }
    }
    let share = total.right as f64 / total.all.max(1) as f64;
    println!(
        "all: {:.1} % of {} stitches ({} of other kinds)",
        100.0 * share,
        total.all,
        total.unknown
    );
    for (k, (right, all)) in PartKind::ALL.iter().zip(total.by_kind) {
        if all > 0 {
            println!(
                "{:<7} {:.1} % of {all} stitches",
                k.label(),
                100.0 * right as f64 / all as f64
            );
        }
    }
    assert!(share >= 0.90, "{share}");
}

/// The column of a source object that the test can measure (a satin or a
/// line with a border, of the types satin and zigzag, with no effect that
/// moves the needle points): the rails of a satin in the coordinates of the
/// file, or the width of a border; and the settings.
enum Source {
    Satin(SatinGeometry),
    Border(f64),
}

fn source_column(d: &Design, kind: &ObjectKind) -> Option<(Source, SatinParams)> {
    let plain = |p: &SatinParams| {
        matches!(p.pattern, ColumnPattern::Satin | ColumnPattern::Zigzag)
            && !p.auto_spacing
            && p.accordion.is_none()
            && p.jagged_mm == 0.0
            && p.hand_mm == 0.0
            && !p.puff
            && p.spacing_at == 0.5
    };
    match kind {
        ObjectKind::Satin { geometry: g, params } if plain(params) => {
            let t = d.output_transform();
            Some((
                Source::Satin(SatinGeometry {
                    rail_a: g.rail_a.transformed(&t),
                    rail_b: g.rail_b.transformed(&t),
                    rungs: g.rungs.iter().map(|(a, b)| (t.apply(*a), t.apply(*b))).collect(),
                }),
                params.clone(),
            ))
        }
        ObjectKind::Running { params, .. } => {
            let b = params.border.as_ref()?;
            plain(&b.column).then(|| (Source::Border(b.width_mm), b.column.clone()))
        }
        _ => None,
    }
}

/// The points of a rail.
fn rail_points(p: &emb_geom::Path) -> Vec<Point> {
    p.flatten(0.05).into_iter().flat_map(|l| l.points).collect()
}

/// The distance from `p` to the line through `pts`.
fn distance_to(p: Point, pts: &[Point]) -> f64 {
    pts.windows(2)
        .map(|w| p.distance_to_segment(w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

/// The width of a column at each pair of its rails: the distance from each
/// side to the other side (not the length of a slanted stitch), with the
/// middle of the pair.
fn widths(g: &SatinGeometry, step: f64) -> Option<Vec<(Point, f64)>> {
    let (a, b) = (rail_points(&g.rail_a), rail_points(&g.rail_b));
    let pairs = emb_stitch::satin::Column::new(g)?.pairs(step);
    Some(
        pairs
            .iter()
            .map(|(p, q)| (p.midpoint(*q), (distance_to(*p, &b) + distance_to(*q, &a)) / 2.0))
            .collect(),
    )
}

/// The width of the recognized column minus the sewn width of the source
/// column (the rails and the pull compensation): the median over the pairs
/// of the recognized column, compared with the source at the same places
/// (a satin) or with the width of the border, mm; `None` if they have no
/// places in common.
fn width_difference(rec: &SatinGeometry, src: &Source, p: &SatinParams) -> Option<f64> {
    let rec = widths(rec, 0.5)?;
    let (comp_a, comp_b) = (p.pull_compensation_mm, p.pull_b_mm.unwrap_or(p.pull_compensation_mm));
    let mut diffs: Vec<f64> = match src {
        Source::Border(w) => {
            let sewn = w * (1.0 + 2.0 * p.pull_percent / 100.0) + comp_a + comp_b;
            rec.iter().map(|(_, r)| r - sewn).collect()
        }
        Source::Satin(g) => {
            // The sides that the engine sews: each pair moved out along its
            // stitch by the pull compensation.
            let pairs = emb_stitch::satin::Column::new(g)?.pairs(0.25);
            let out: Vec<(Point, Point)> = pairs
                .iter()
                .filter_map(|&(a, b)| {
                    let w = a.distance(b);
                    let u = (a - b).normalized()?;
                    let pct = w * p.pull_percent / 100.0;
                    Some((a + u * (comp_a + pct), b - u * (comp_b + pct)))
                })
                .collect();
            let line = |pts: Vec<Point>| {
                pts[1..]
                    .iter()
                    .fold(emb_geom::Path::new().move_to(pts[0]), |path, q| path.line_to(*q))
            };
            if out.len() < 2 {
                return None;
            }
            let sewn = SatinGeometry {
                rail_a: line(out.iter().map(|x| x.0).collect()),
                rail_b: line(out.iter().map(|x| x.1).collect()),
                rungs: out[1..out.len() - 1].to_vec(),
            };
            let src = widths(&sewn, 0.25)?;
            rec.iter()
                .filter_map(|(m, r)| {
                    let (dist, w) = src
                        .iter()
                        .map(|(c, w)| (c.distance(*m), *w))
                        .min_by(|x, y| x.0.total_cmp(&y.0))?;
                    (dist < 1.0).then(|| r - w)
                })
                .collect()
        }
    };
    if diffs.is_empty() {
        return None;
    }
    diffs.sort_by(f64::total_cmp);
    Some(diffs[diffs.len() / 2])
}

/// The spacing that the source column sews: the engine puts a whole number
/// of stitches in each section between two rungs, so a column with many
/// short sections sews more open or denser than its setting. Two times the
/// length along the middle divided by the stitches across, as for a
/// recognized column; the setting for a border and a liquid column.
fn sewn_spacing(src: &Source, p: &SatinParams) -> f64 {
    let Source::Satin(g) = src else { return p.spacing_mm };
    if p.liquid {
        return p.spacing_mm;
    }
    let len = |p: &emb_geom::Path| rail_points(p).windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
    match emb_stitch::satin::Column::new(g).map(|c| c.pairs(p.spacing_mm / 2.0).len()) {
        Some(n) if n > 1 => (len(&g.rail_a) + len(&g.rail_b)) / (n - 1) as f64,
        _ => p.spacing_mm,
    }
}

#[test]
fn recognized_columns_have_the_width_and_spacing_of_their_source() {
    // Stitches of measured columns that pass, and all; the columns that do
    // not pass.
    let (mut pass, mut all) = (0usize, 0usize);
    let mut failed: Vec<String> = Vec::new();
    // Stitches of source columns in a recognized satin, and all.
    let (mut in_satin, mut column_stitches) = (0usize, 0usize);
    // Recognized runs of plain source runs (90 % of their stitches from the
    // run: not the travel of the plan around a short run): with the stitch
    // length (10 %) and the bean setting of the source, and all.
    let (mut runs_right, mut runs) = (0usize, 0usize);
    for name in corpus::NAMES {
        let Some(t) = RoundTrip::new(name) else { continue };
        let made = objects(&t.read, &parts(&t.read));
        // The records of the file in each recognized satin.
        let mut satin = vec![false; t.read.records.len()];
        let (mut p, mut a) = (0usize, 0usize);
        for m in &made {
            if let ObjectKind::Running { params: rp, .. } = &m.kind
                && let Some((o, purity)) = t.source_of(m)
                && purity >= 0.9
                && let Some(ObjectKind::Running { params: sp, .. }) = t.design.object(o).map(|o| &o.kind)
                && sp.border.is_none()
                && sp.motif.is_none()
                && sp.ripple.is_none()
                && !sp.cording
                && sp.hand_mm == 0.0
                && sp.style == emb_model::RunStyle::Run
                && sp.pass == emb_model::RunPass::Once
            {
                runs += 1;
                let bean = |r: u32| r >= 3;
                if (rp.stitch_length_mm / sp.stitch_length_mm - 1.0).abs() <= 0.1
                    && bean(rp.repeats) == bean(sp.repeats)
                {
                    runs_right += 1;
                } else if std::env::var("PARTS_RUNS").is_ok() {
                    let stitches: usize = m.records.iter().map(|r| r.len()).sum();
                    println!(
                        "  run {name} object {}: {} stitches, length {} for {}, repeats {} for {}",
                        o.0, stitches, rp.stitch_length_mm, sp.stitch_length_mm, rp.repeats, sp.repeats
                    );
                }
            }
            let ObjectKind::Satin { geometry, params } = &m.kind else {
                continue;
            };
            for r in m.records.iter().flat_map(|r| r.clone()) {
                satin[r] = true;
            }
            let Some((o, purity)) = t.source_of(m) else { continue };
            let Some(obj) = t.design.object(o) else { continue };
            let Some((src, sp)) = source_column(&t.design, &obj.kind) else {
                continue;
            };
            let stitches: usize = m.records.iter().map(|r| r.len()).sum();
            let Some(dw) = width_difference(geometry, &src, &sp) else {
                continue;
            };
            let sewn = sewn_spacing(&src, &sp);
            let ratio = params.spacing_mm / sewn;
            a += stitches;
            if dw.abs() <= 0.2 && (ratio - 1.0).abs() <= 0.1 {
                p += stitches;
            } else {
                failed.push(format!(
                    "{name}: object {} ({} stitches, {:.0} % of it): width {dw:+.2} mm, spacing {:.2} for {:.2} mm (set {:.2})",
                    o.0,
                    stitches,
                    100.0 * purity,
                    params.spacing_mm,
                    sewn,
                    sp.spacing_mm
                ));
            }
        }
        for (i, s) in satin.iter().enumerate() {
            let Some(o) = t.source(i) else { continue };
            if t.design.object(o).map(|o| expected(&o.kind)) == Some(Some(PartKind::Column)) {
                column_stitches += 1;
                if *s {
                    in_satin += 1;
                }
            }
        }
        if a > 0 {
            println!(
                "{name:<28} {:>6.1} % of {:>6} stitches of measured columns",
                100.0 * p as f64 / a as f64,
                a
            );
        }
        pass += p;
        all += a;
    }
    for f in &failed {
        println!("  {f}");
    }
    let share = pass as f64 / all.max(1) as f64;
    println!(
        "all: {:.1} % of {all} stitches of measured columns; {:.1} % of {column_stitches} stitches of columns are in a satin",
        100.0 * share,
        100.0 * in_satin as f64 / column_stitches.max(1) as f64
    );
    println!(
        "runs: {:.1} % of {runs} recognized runs of plain runs have the stitch length and the bean setting of their source",
        100.0 * runs_right as f64 / runs.max(1) as f64
    );
    assert!(share >= 0.90, "{share}");
}

/// Pictures of the recognized columns of a corpus design: the stitches of
/// the file (gray), the rails (red and blue) and the rungs (green) of each
/// satin, and the stitches that the satin sews again (orange). Run with
/// `PARTS_DESIGN=<name> [PARTS_OBJECT=<id>] cargo test --release -p emb-cli
/// --test parts -- --ignored write_part_pictures --nocapture`; the pictures
/// are in `target/tmp/parts`.
#[test]
#[ignore]
fn write_part_pictures() {
    use emb_model::Rgb;
    use emb_render::{Segment, SegmentKind};
    let name = std::env::var("PARTS_DESIGN").unwrap_or_else(|_| "satin-column".into());
    let only: Option<u64> = std::env::var("PARTS_OBJECT").ok().and_then(|v| v.parse().ok());
    let t = RoundTrip::new(&name).expect("the design");
    let all_parts = parts(&t.read);
    let made = objects(&t.read, &all_parts);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/parts");
    std::fs::create_dir_all(&dir).unwrap();
    let seg = |a: Point, b: Point, color: Rgb| Segment {
        a,
        b,
        color,
        kind: SegmentKind::Stitch,
        ..Default::default()
    };
    for (k, m) in made.iter().enumerate() {
        let ObjectKind::Satin { geometry, params } = &m.kind else {
            continue;
        };
        let source = t.source_of(m).map(|x| x.0);
        if only.is_some_and(|o| source.map(|s| s.0) != Some(o)) {
            continue;
        }
        let mut segs = Vec::new();
        for r in m.records.iter().flat_map(|r| r.clone()) {
            if r > 0 {
                segs.push(seg(
                    t.read.records[r - 1].pos,
                    t.read.records[r].pos,
                    Rgb(170, 170, 170),
                ));
            }
        }
        // The stitches of the satin, sewn again.
        let mut d = Design::new("");
        let th = d.add_thread("t", Rgb(0, 0, 0));
        d.insert_object(0, th, m.kind.clone());
        let plan = emb_cli::plan(&d, &Default::default());
        for w in plan.stitches.records.windows(2) {
            if w[1].kind == StitchKind::Stitch {
                segs.push(seg(w[0].pos, w[1].pos, Rgb(240, 140, 20)));
            }
        }
        for (rail, color) in [
            (&geometry.rail_a, Rgb(220, 30, 30)),
            (&geometry.rail_b, Rgb(30, 60, 220)),
        ] {
            let pts: Vec<Point> = rail.points().collect();
            for w in pts.windows(2) {
                segs.push(seg(w[0], w[1], color));
            }
        }
        for (a, b) in &geometry.rungs {
            segs.push(seg(*a, *b, Rgb(30, 160, 60)));
        }
        let pic = emb_render::raster::picture(&segs, &[], 30.0, 0.06, 0.5, Some(Rgb(255, 255, 255)));
        let file = dir.join(format!("{name}-{k:03}-object-{}.png", source.map(|s| s.0).unwrap_or(0)));
        std::fs::write(&file, pic.png()).unwrap();
        println!(
            "{} {:?} spacing {} {:?}",
            file.display(),
            m.records,
            params.spacing_mm,
            params.underlay
        );
        if std::env::var("PARTS_LIST").is_ok() {
            let len = |p: &emb_geom::Path| rail_points(p).windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
            println!(
                "  rec rails {:.2} {:.2}, rungs {}",
                len(&geometry.rail_a),
                len(&geometry.rail_b),
                geometry.rungs.len()
            );
            if let Some(o) = source.and_then(|o| t.design.object(o))
                && let Some((Source::Satin(g), sp)) = source_column(&t.design, &o.kind)
            {
                let n = emb_stitch::satin::Column::new(&g)
                    .map(|c| c.pairs(sp.spacing_mm / 2.0).len())
                    .unwrap_or(0);
                println!(
                    "  src rails {:.2} {:.2}, rungs {}, pairs {n}, comp {}",
                    len(&g.rail_a),
                    len(&g.rail_b),
                    g.rungs.len(),
                    sp.pull_compensation_mm
                );
            }
            let lo = m.records.iter().map(|r| r.start).min().unwrap_or(0).saturating_sub(150);
            let hi = m.records.iter().map(|r| r.end).max().unwrap_or(0) + 30;
            for (i, p) in all_parts.iter().enumerate() {
                if p.records.end >= lo && p.records.start <= hi {
                    let src = t.source(p.records.start).map(|o| o.0);
                    println!(
                        "  part {i} {:?} {:?} under {:?} object {src:?}",
                        p.kind, p.records, p.underlay_of
                    );
                }
            }
        }
    }
}

/// A picture of a corpus design as its file (left) and as the objects that
/// its parts become, sewn again (right; runs blue, satins orange, stitches
/// gray). Run with `PARTS_DESIGN=<name> cargo test --release -p emb-cli
/// --test parts -- --ignored write_whole_picture --nocapture`; the picture
/// is in `target/tmp/parts`.
#[test]
#[ignore]
fn write_whole_picture() {
    use emb_model::Rgb;
    use emb_render::{Segment, SegmentKind};
    let name = std::env::var("PARTS_DESIGN").unwrap_or_else(|_| "view-lettering".into());
    let t = RoundTrip::new(&name).expect("the design");
    let made = objects(&t.read, &parts(&t.read));
    let seg = |a: Point, b: Point, color: Rgb| Segment {
        a,
        b,
        color,
        kind: SegmentKind::Stitch,
        ..Default::default()
    };
    let mut segs = Vec::new();
    for w in t.read.records.windows(2) {
        if w[1].kind == StitchKind::Stitch {
            segs.push(seg(w[0].pos, w[1].pos, Rgb(60, 60, 60)));
        }
    }
    let bb = emb_geom::BBox::from_points(t.read.records.iter().map(|r| r.pos)).expect("stitches");
    let dx = bb.width() + 10.0;
    let mut d = Design::new("");
    let colors = [Rgb(150, 150, 150), Rgb(40, 110, 235), Rgb(240, 140, 20)];
    let threads: Vec<_> = colors.iter().map(|c| d.add_thread("t", *c)).collect();
    for m in &made {
        let k = match m.kind {
            ObjectKind::Stitches { .. } => 0,
            ObjectKind::Running { .. } => 1,
            _ => 2,
        };
        let mut kind = m.kind.clone();
        for p in kind.paths_mut() {
            *p = p.transformed(&emb_geom::Transform::translate(dx, 0.0));
        }
        if let ObjectKind::Satin { geometry, .. } = &mut kind {
            for r in &mut geometry.rungs {
                *r = (r.0 + Point::new(dx, 0.0), r.1 + Point::new(dx, 0.0));
            }
        }
        let at = d.objects().len();
        d.insert_object(at, threads[k], kind);
    }
    let plan = emb_cli::plan(&d, &Default::default());
    // The colors of the threads of the plan, block by block.
    let mut block = 0usize;
    for w in plan.stitches.records.windows(2) {
        if w[1].kind == StitchKind::ColorChange {
            block += 1;
        }
        if w[1].kind == StitchKind::Stitch {
            let c = plan
                .stitches
                .threads
                .get(block)
                .map(|t| Rgb(t.color.0, t.color.1, t.color.2))
                .unwrap_or(Rgb(0, 0, 0));
            segs.push(seg(w[0].pos, w[1].pos, c));
        }
    }
    let pic = emb_render::raster::picture(&segs, &[], 8.0, 0.12, 2.0, Some(Rgb(255, 255, 255)));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/parts");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{name}-whole.png"));
    std::fs::write(&file, pic.png()).unwrap();
    let recognized = made.iter().filter(|m| m.recognized()).count();
    println!("{}: {} objects, {recognized} recognized", file.display(), made.len());
}
