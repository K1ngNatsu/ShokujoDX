//! The exit test of P18 Milestone 1 (ADR 0131): each design of the corpus,
//! written as DST and read again, is cut into parts (`emb_art::parts`);
//! 90 % of the stitches of the objects of a kind that the parts know (runs,
//! satin columns, fills of rows) are in a part of the kind of their source
//! object. Lettering is cut into its letter parts first (as Break Apart),
//! so each stitch has the kind of the part of a letter that made it.
//!
//! `cargo test --release -p emb-cli --test parts -- --nocapture` prints the
//! numbers of each design.

use emb_art::parts::{PartKind, kinds_of_records, parts};
use emb_cli::corpus;
use emb_model::{ColumnPattern, Design, FillPattern, ObjectKind, StitchKind};

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

#[derive(Default)]
struct Count {
    right: usize,
    all: usize,
    /// Stitches of objects that the parts do not know.
    unknown: usize,
    /// For each kind: the stitches in a part of that kind, and all.
    by_kind: [(usize, usize); 4],
}

fn count(name: &str) -> Option<Count> {
    let design = with_letter_parts(&corpus::build(name)?);
    let opts = emb_cli::plan_options(&design, None).ok()?;
    let plan = emb_cli::plan(&design, &opts);
    if plan.stitches.stitch_count() == 0 {
        return None;
    }
    // The kind of each record of the plan, from its object.
    let mut want: Vec<Option<Option<PartKind>>> = vec![None; plan.stitches.records.len()];
    for s in &plan.spans {
        let k = design.object(s.object).map(|o| expected(&o.kind));
        for r in s.records.clone() {
            want[r] = k;
        }
    }
    let bytes = emb_formats::write_machine("dst", &plan.stitches, name, "2026-11-19")?;
    let read = emb_formats::read_machine("dst", &bytes).ok()?;
    let got = kinds_of_records(&parts(&read), read.records.len());
    // Each stitch of the file is the next stitch of the plan at its place
    // (DST rounds to 0.1 mm and can cut a long stitch).
    let plan_stitches: Vec<(usize, emb_geom::Point)> = plan
        .stitches
        .records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == StitchKind::Stitch)
        .map(|(i, r)| (i, r.pos))
        .collect();
    let mut at = 0usize;
    let mut c = Count::default();
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
        let plan_record = plan_stitches[at].0;
        at = (at + 1).min(plan_stitches.len().saturating_sub(1));
        match want[plan_record] {
            Some(Some(w)) => {
                c.all += 1;
                let k = PartKind::ALL.iter().position(|x| *x == w).unwrap_or(3);
                c.by_kind[k].1 += 1;
                if got[i] == Some(w) {
                    c.right += 1;
                    c.by_kind[k].0 += 1;
                }
            }
            Some(None) => c.unknown += 1,
            None => {}
        }
    }
    Some(c)
}

#[test]
fn the_stitches_of_the_corpus_are_in_parts_of_their_kind() {
    let mut total = Count::default();
    for name in corpus::NAMES {
        let Some(c) = count(name) else { continue };
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
