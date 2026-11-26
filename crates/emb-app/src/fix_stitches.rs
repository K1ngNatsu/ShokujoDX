//! Fix Stitches (P22 Milestone 5, ADR 0172): the check and the fix of the
//! stitch objects of a design (a machine file opened as stitches).
//!
//! The stitch objects that sew one after the other are one machine file
//! for `emb_art::file_check`: their records come from
//! `emb_model::records_of_stitch_objects`, and the fixed records make the
//! objects again (`emb_model::objects_from_stitches`) in their place in
//! the sew order, with the threads of their color blocks.

use std::sync::Arc;

use eframe::egui;
use emb_art::file_check::{Fault, FileCheck, Fixed, Fixes};
use emb_model::{Design, ObjectId, ObjectKind};
use emb_stitch::PlanOptions;

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The window state.
#[derive(Debug, Clone)]
pub struct FixStitchesWindow {
    pub open: bool,
    pub fixes: Fixes,
    /// What the last fix did.
    pub message: Option<String>,
    /// The checks of the stitch objects: for the state of the design and
    /// the stitch lengths and trim distance (their bits).
    pub cache: Option<(u64, [u64; 3], Arc<Vec<RunCheck>>)>,
}

impl Default for FixStitchesWindow {
    fn default() -> Self {
        FixStitchesWindow {
            open: false,
            fixes: Fixes::ALL,
            message: None,
            cache: None,
        }
    }
}

/// The stitch objects of a design that sew one after the other, in sew
/// order.
pub fn stitch_runs(d: &Design) -> Vec<Vec<ObjectId>> {
    let mut out: Vec<Vec<ObjectId>> = Vec::new();
    let mut run = Vec::new();
    for o in d.objects() {
        if matches!(o.kind, ObjectKind::Stitches { .. }) {
            run.push(o.id);
        } else if !run.is_empty() {
            out.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

/// The check of one run of stitch objects: the objects, the object of
/// each record (its number in `ids`), and the faults.
#[derive(Debug)]
pub struct RunCheck {
    pub ids: Vec<ObjectId>,
    pub owner: Vec<usize>,
    pub check: FileCheck,
}

impl RunCheck {
    /// The object of the record `record`.
    pub fn object(&self, record: usize) -> Option<ObjectId> {
        self.owner.get(record).and_then(|&k| self.ids.get(k)).copied()
    }
}

/// The checks of the stitch objects of `d` with the stitch lengths and the
/// trim distance of `opts`. A start of an object with Tie-In and an end of
/// an object with Tie-Off have lock stitches in the stitch plan, so they
/// are not faults.
pub fn check_design(d: &Design, opts: &PlanOptions) -> Vec<RunCheck> {
    stitch_runs(d)
        .into_iter()
        .map(|ids| {
            let (s, owner) = emb_model::records_of_stitch_objects(d, &ids);
            let check = emb_art::file_check::check(&s, opts);
            let mut run = RunCheck {
                ids,
                owner,
                check: FileCheck::default(),
            };
            let functions = |record: usize| run.object(record).and_then(|id| d.object(id)).map(|o| o.functions);
            let found = check
                .found
                .into_iter()
                .filter(|f| match f.fault {
                    Fault::NoLockStart => !functions(f.record).is_some_and(|x| x.tie_in),
                    Fault::NoLockEnd => !functions(f.record).is_some_and(|x| x.tie_off),
                    _ => true,
                })
                .collect();
            run.check.found = found;
            run
        })
        .collect()
}

/// Fixes the stitch objects of `d` (each run of them as one file). Returns
/// what the fixes did. The objects of a run that all have Tie-In and
/// Tie-Off keep them, and get no lock stitches of their own (the stitch
/// plan adds them).
pub fn fix_design(d: &mut Design, opts: &PlanOptions, which: Fixes) -> Fixed {
    let mut total = Fixed::default();
    for ids in stitch_runs(d) {
        let (s, _) = emb_model::records_of_stitch_objects(d, &ids);
        let tied = ids
            .iter()
            .filter_map(|id| d.object(*id))
            .all(|o| o.functions.tie_in && o.functions.tie_off);
        let which = Fixes {
            locks: which.locks && !tied,
            ..which
        };
        let (fixed, n) = emb_art::file_check::fix(&s, opts, which);
        if n.total() == 0 {
            continue;
        }
        total.joined += n.joined;
        total.removed += n.removed;
        total.split += n.split;
        total.trimmed += n.trimmed;
        total.locked += n.locked;
        let Some(at) = ids.first().and_then(|id| d.index_of(*id)) else {
            continue;
        };
        for id in &ids {
            d.remove_object(*id);
        }
        let objects = emb_model::objects_from_stitches(&fixed);
        for (k, (block, kind, functions)) in objects.into_iter().enumerate() {
            let Some(thread) = fixed.threads.get(block).map(|t| t.id) else {
                continue;
            };
            let id = d.insert_object(at + k, thread, kind);
            if let Some(o) = d.object_mut(id) {
                o.functions = functions;
                o.functions.tie_in = tied;
                o.functions.tie_off = tied;
            }
        }
    }
    total
}

impl AppState {
    /// The stitch lengths of the machine of the design and the trim
    /// distance of the design, for the check of stitch objects.
    pub fn file_check_options(&self) -> PlanOptions {
        self.plan_options().for_design(self.doc.design())
    }

    /// The checks of the stitch objects of the design (cached for the
    /// state of the design).
    pub fn stitch_checks(&mut self) -> Arc<Vec<RunCheck>> {
        let opts = self.file_check_options();
        let key = [
            opts.min_stitch_mm.to_bits(),
            opts.max_stitch_mm.to_bits(),
            opts.trim_mm.to_bits(),
        ];
        let state = self.doc.state_id();
        if let Some((s, k, c)) = &self.fix_stitches.cache
            && *s == state
            && *k == key
        {
            return c.clone();
        }
        let checks = Arc::new(check_design(self.doc.design(), &opts));
        self.fix_stitches.cache = Some((state, key, checks.clone()));
        checks
    }

    /// Fixes the stitch objects of the design: one undo step.
    pub fn fix_stitches(&mut self, which: Fixes) -> Fixed {
        let opts = self.file_check_options();
        let n = self.doc.edit(tr("Fix Stitches"), |d| fix_design(d, &opts, which));
        self.selection.clear();
        n
    }
}

/// The text of what a fix did.
fn fixed_text(n: &Fixed) -> String {
    if n.total() == 0 {
        return tr("Nothing to fix.").into();
    }
    fill(
        "Fixed: {} color changes joined, {} short stitches removed, {} long stitches split, {} jumps trimmed, {} lock stitches added.",
        &[&n.joined, &n.removed, &n.split, &n.trimmed, &n.locked],
    )
}

/// The Fix Stitches window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.fix_stitches.open {
        return;
    }
    let mut open = true;
    let opts = st.file_check_options();
    let checks = st.stitch_checks();
    let count = |f: Fault| checks.iter().map(|c| c.check.count(f)).sum::<usize>();
    crate::dock::docked(
        egui::Window::new(tr("Fix Stitches")).id(egui::Id::new("FixStitches")),
        ctx,
        egui::Id::new("FixStitches"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(400.0)
    .show(ctx, |ui| {
        if checks.is_empty() {
            ui.label(tr(
                "The design has no stitch objects. Fix Stitches is for a machine file opened as stitches.",
            ));
            return;
        }
        ui.label(tr(
            "Fixes for the stitch objects (a machine file opened as stitches). The other needle points stay where they are.",
        ));
        ui.separator();
        let w = &mut st.fix_stitches;
        let row = |ui: &mut egui::Ui, on: &mut bool, text: String, n: usize, help: &str| {
            ui.add_enabled(n > 0, egui::Checkbox::new(on, text)).on_hover_text(help);
        };
        row(
            ui,
            &mut w.fixes.join,
            fill("Join color blocks of the same thread ({})", &[&count(Fault::SameThread)]),
            count(Fault::SameThread),
            tr("Remove a color change to the same thread (color, brand and code). The machine does not stop there."),
        );
        row(
            ui,
            &mut w.fixes.short,
            fill(
                "Remove stitches shorter than {} mm ({})",
                &[&format!("{:.1}", opts.min_stitch_mm), &count(Fault::Short)],
            ),
            count(Fault::Short),
            tr("Short stitches break the thread. The next stitch starts at the needle point before."),
        );
        row(
            ui,
            &mut w.fixes.long,
            fill(
                "Split stitches longer than {} mm ({})",
                &[&format!("{:.1}", opts.max_stitch_mm), &count(Fault::Long)],
            ),
            count(Fault::Long),
            tr("The machine cannot sew a stitch this long. It becomes equal stitches on the same line."),
        );
        row(
            ui,
            &mut w.fixes.jumps,
            fill(
                "Trim jumps longer than {} mm ({})",
                &[&format!("{:.1}", opts.trim_mm), &count(Fault::LongJump)],
            ),
            count(Fault::LongJump),
            tr("A long jump with no trim leaves a thread across the design."),
        );
        let locks = count(Fault::NoLockStart) + count(Fault::NoLockEnd);
        row(
            ui,
            &mut w.fixes.locks,
            fill("Add lock stitches at starts and ends ({})", &[&locks]),
            locks,
            tr("A start or an end with no lock stitches can come loose. The lock stitches go forward and back twice."),
        );
        let pieces = count(Fault::ShortPiece);
        let travel: f64 = checks
            .iter()
            .flat_map(|c| &c.check.found)
            .filter(|f| f.fault == Fault::TravelShows)
            .map(|f| f.length_mm)
            .sum();
        if pieces > 0 || travel > 0.0 {
            ui.separator();
            ui.weak(tr("No fix for these (Design info shows where they are):"));
            if pieces > 0 {
                ui.weak(fill("Pieces too short for the trimmer: {}", &[&pieces]));
            }
            if travel > 0.0 {
                ui.weak(fill(
                    "Travel that shows on top of sewn stitches: {} mm",
                    &[&format!("{travel:.0}")],
                ));
            }
        }
        if let Some(m) = &w.message {
            ui.separator();
            ui.label(m);
        }
        ui.separator();
        let f = w.fixes;
        let any = (f.join && count(Fault::SameThread) > 0)
            || (f.short && count(Fault::Short) > 0)
            || (f.long && count(Fault::Long) > 0)
            || (f.jumps && count(Fault::LongJump) > 0)
            || (f.locks && locks > 0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(any, egui::Button::new(tr("Apply Fixes")))
                .on_hover_text(tr("Make the fixes that are on. One undo step."))
                .clicked()
            {
                let n = st.fix_stitches(f);
                st.fix_stitches.message = Some(fixed_text(&n));
            }
            if ui.button(tr("Close")).clicked() {
                st.fix_stitches.open = false;
            }
        });
    });
    if !open {
        st.fix_stitches.open = false;
    }
    if !st.fix_stitches.open {
        st.fix_stitches.message = None;
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread};

    use super::*;
    use crate::test_util::Harness;

    fn st(x: f64, y: f64) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), StitchKind::Stitch)
    }

    /// A file with a short stitch, a long stitch, a long jump with no trim,
    /// a color change to the same thread and runs with no lock stitches.
    fn file() -> StitchDesign {
        let mut r = vec![StitchRecord::new(Point::ZERO, StitchKind::Jump)];
        for k in 0..6 {
            r.push(st(2.0 * f64::from(k), 0.0));
        }
        r.push(st(10.1, 0.0));
        r.push(st(10.1, 20.0));
        r.push(StitchRecord::new(Point::new(30.0, 0.0), StitchKind::Jump));
        for k in 0..6 {
            r.push(st(30.0 + 2.0 * f64::from(k), 0.0));
        }
        r.push(StitchRecord::new(Point::new(40.0, 0.0), StitchKind::Trim));
        r.push(StitchRecord::new(Point::new(40.0, 0.0), StitchKind::ColorChange));
        r.push(StitchRecord::new(Point::new(40.0, 5.0), StitchKind::Jump));
        for k in 0..6 {
            r.push(st(40.0 + 2.0 * f64::from(k), 5.0));
        }
        r.push(StitchRecord::new(Point::new(50.0, 5.0), StitchKind::End));
        let red = Thread {
            name: "Red".into(),
            color: Rgb(200, 0, 0),
            ..Default::default()
        };
        StitchDesign {
            threads: vec![red.clone(), red],
            records: r,
        }
    }

    fn harness_with_file() -> Harness {
        let mut h = Harness::empty();
        let d = emb_model::design_from_stitches(&file(), "file");
        h.st.doc.edit("Open", |x| *x = d);
        h
    }

    #[test]
    fn the_stitch_objects_are_checked_and_fixed_in_one_undo_step() {
        let mut h = harness_with_file();
        let opts = h.st.file_check_options();
        let before = check_design(h.st.doc.design(), &opts);
        assert_eq!(before.len(), 1);
        let c = &before[0].check;
        assert_eq!(c.count(Fault::Short), 1);
        assert_eq!(c.count(Fault::Long), 1);
        assert_eq!(c.count(Fault::LongJump), 1);
        assert_eq!(c.count(Fault::SameThread), 1);
        assert!(c.count(Fault::NoLockStart) > 0);
        // Each fault has its object.
        for f in &c.found {
            assert!(before[0].object(f.record).is_some(), "{f:?}");
        }
        let objects = h.st.doc.design().objects().len();
        let n = h.st.fix_stitches(Fixes::ALL);
        assert_eq!((n.joined, n.removed, n.split, n.trimmed), (1, 1, 1, 1));
        assert!(n.locked > 0);
        let after = check_design(h.st.doc.design(), &opts);
        for f in [
            Fault::Short,
            Fault::Long,
            Fault::LongJump,
            Fault::SameThread,
            Fault::NoLockStart,
            Fault::NoLockEnd,
        ] {
            assert_eq!(after[0].check.count(f), 0, "{f:?}");
        }
        // The color blocks of the same thread are one object now.
        assert!(h.st.doc.design().objects().len() < objects);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), objects);
        assert_eq!(check_design(h.st.doc.design(), &opts)[0].check, before[0].check);
    }

    #[test]
    fn design_info_warns_for_the_faults_of_the_file() {
        let mut h = harness_with_file();
        let info = h.st.design_info();
        let texts: Vec<&str> = info.issues.iter().map(|i| i.text.as_str()).collect();
        for want in [
            "Stitches longer than",
            "A color change to the same thread",
            "Jumps longer than",
            "Starts and ends with no lock stitches",
        ] {
            let issue = info.issues.iter().find(|i| i.text.starts_with(want));
            assert!(
                issue.is_some_and(|i| i.object.is_some() && i.at.is_some()),
                "{want}: {texts:?}"
            );
        }
        h.st.fix_stitches(Fixes::ALL);
        let info = h.st.design_info();
        assert!(
            !info.issues.iter().any(|i| i.text.contains("Fix Stitches")),
            "{:?}",
            info.issues
        );
    }

    /// Stitch objects with Tie-In and Tie-Off have lock stitches in the
    /// stitch plan: no fault, and the fix adds none and keeps them.
    #[test]
    fn objects_with_tie_in_and_tie_off_have_their_locks() {
        let mut h = harness_with_file();
        h.st.doc.edit("Tie", |d| {
            let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
            for id in ids {
                if let Some(o) = d.object_mut(id) {
                    o.functions.tie_in = true;
                    o.functions.tie_off = true;
                }
            }
        });
        let opts = h.st.file_check_options();
        let c = &check_design(h.st.doc.design(), &opts)[0].check;
        assert_eq!(c.count(Fault::NoLockStart) + c.count(Fault::NoLockEnd), 0);
        let n = h.st.fix_stitches(Fixes::ALL);
        assert_eq!(n.locked, 0);
        assert!(
            h.st.doc
                .design()
                .objects()
                .iter()
                .all(|o| o.functions.tie_in && o.functions.tie_off)
        );
    }

    #[test]
    fn other_objects_stay_in_their_place() {
        let mut h = harness_with_file();
        let thread = h.st.doc.design().threads[0].id;
        let fill = h.st.doc.edit("Fill", |d| {
            d.insert_object(
                1,
                thread,
                ObjectKind::Fill {
                    outer: emb_geom::Path::rect(emb_geom::BBox::new(Point::new(0.0, 30.0), Point::new(10.0, 40.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        assert_eq!(stitch_runs(h.st.doc.design()).len(), 2);
        h.st.fix_stitches(Fixes::ALL);
        let d = h.st.doc.design();
        let at = d.index_of(fill).expect("the fill stays");
        assert!(at > 0 && at + 1 < d.objects().len());
    }

    #[test]
    fn the_window_shows_and_fixes() {
        let mut h = harness_with_file();
        let draw = |h: &mut Harness| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
                ..Default::default()
            };
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(input, |ui| window(ui.ctx(), st));
            out.textures_delta.clear();
        };
        h.st.fix_stitches.open = true;
        draw(&mut h);
        assert!(h.st.fix_stitches.open);
        let mut empty = Harness::empty();
        empty.st.fix_stitches.open = true;
        draw(&mut empty);
        assert!(empty.st.fix_stitches.open);
    }
}
