//! Quicker to Sew (P22 Milestone 2, ADR 0169): one window that makes a
//! design quicker to sew (`emb_model::quicker`): merge threads that look
//! almost the same, remove small details, fewer color changes, and the
//! trim distance of the design. A table shows the threads, color changes,
//! trims, stitches, sew time and the travel that shows (ADR 0167), before
//! and after; the canvas shows the result and its travel while the values
//! change. OK keeps it as one undo step, Cancel goes back (as Clean Up).

use std::collections::HashMap;

use eframe::egui;
use emb_model::quicker::{self, QuickerToSew};
use emb_model::{Design, MergeKey, StitchKind, ThreadId};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The numbers of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Numbers {
    pub threads: usize,
    pub color_changes: usize,
    pub trims: usize,
    pub stitches: usize,
    pub minutes: Option<f64>,
    pub travel_on_stitches_mm: f64,
    pub travel_on_fabric_mm: f64,
}

/// The Quicker to Sew window.
#[derive(Debug, Clone)]
pub struct QuickerDialog {
    pub settings: QuickerToSew,
    /// The design before the change.
    original: Design,
    /// The stitches of each thread before the change.
    sews: HashMap<ThreadId, usize>,
    pub before: Numbers,
    key: MergeKey,
    /// The last settings applied to the design.
    applied: Option<QuickerToSew>,
}

/// The stitches of each thread of a plan.
fn stitches_by_thread(s: &emb_model::StitchDesign) -> HashMap<ThreadId, usize> {
    let mut out = HashMap::new();
    let mut block = 0;
    for r in &s.records {
        match r.kind {
            StitchKind::ColorChange => block += 1,
            StitchKind::Stitch => {
                if let Some(t) = s.threads.get(block) {
                    *out.entry(t.id).or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }
    out
}

impl AppState {
    /// The numbers of the design as it is now, if its plan is made.
    pub fn quicker_numbers(&mut self) -> Option<Numbers> {
        let state = self.doc.state_id();
        if self.shown_plan().state != state {
            return None;
        }
        let info = self.design_info();
        Some(Numbers {
            threads: info.threads,
            color_changes: info.colors.len().saturating_sub(1),
            trims: info.trims,
            stitches: info.stitches,
            minutes: info.minutes,
            travel_on_stitches_mm: info.travel.on_stitches_mm(),
            travel_on_fabric_mm: info.travel.on_fabric_mm(),
        })
    }

    pub fn open_quicker(&mut self) {
        if self.doc.design().objects().is_empty() {
            return;
        }
        // The numbers before: the exact plan of the design as it is now.
        let sews = stitches_by_thread(&self.plan_view().plan.stitches);
        let Some(before) = self.quicker_numbers() else {
            return;
        };
        self.drag_counter += 1;
        let mut settings = self.quicker_settings;
        // A design with its own trim distance starts with it.
        if let Some(g) = self.doc.design().trim_gap_mm {
            settings.gap_mm = Some(g);
        }
        self.quicker_dialog = Some(QuickerDialog {
            settings,
            original: self.doc.design().clone(),
            sews,
            before,
            key: MergeKey::new(format!("quicker-{}", self.drag_counter)),
            applied: None,
        });
    }

    /// Shows the result of the current settings on the canvas.
    pub fn preview_quicker(&mut self, d: &mut QuickerDialog) {
        if d.applied == Some(d.settings) {
            return;
        }
        let (original, s, sews) = (d.original.clone(), d.settings, d.sews.clone());
        self.doc.edit_merged(tr("Quicker to Sew"), d.key.clone(), |dd| {
            *dd = original;
            quicker::apply(dd, &s, &|t| sews.get(&t).copied().unwrap_or(0));
        });
        d.applied = Some(d.settings);
    }

    /// OK (`keep`) or Cancel.
    pub fn close_quicker(&mut self, mut d: QuickerDialog, keep: bool) {
        self.quicker_settings = d.settings;
        if keep {
            self.preview_quicker(&mut d);
            self.doc.end_merge();
            self.message = tr("The design is quicker to sew.").into();
        } else if d.applied.is_some() {
            self.doc.end_merge();
            self.undo();
        }
    }
}

/// A value that can be on or off: a check box and a slider.
fn optional(
    ui: &mut egui::Ui,
    value: &mut Option<f64>,
    label: &str,
    hint: &str,
    range: std::ops::RangeInclusive<f64>,
    standard: f64,
    suffix: &str,
) {
    let mut on = value.is_some();
    ui.checkbox(&mut on, label).on_hover_text(hint);
    let mut v = value.unwrap_or(standard);
    ui.add_enabled(
        on,
        crate::workspace::slider(&mut v, range)
            .clamping(egui::SliderClamping::Edits)
            .fixed_decimals(1)
            .suffix(suffix),
    );
    ui.end_row();
    *value = on.then_some(v);
}

/// The Quicker to Sew window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.quicker_dialog.take() else {
        return;
    };
    let after = st.quicker_numbers();
    let machine_gap = st.plan_options().trim_mm;
    let units = st.settings.units;
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Quicker to Sew")).id(egui::Id::new("Quicker to Sew")),
        ctx,
        egui::Id::new("Quicker to Sew"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let s = &mut d.settings;
        egui::Grid::new("quicker_settings")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                optional(
                    ui,
                    &mut s.merge_threads,
                    tr("Merge similar threads"),
                    tr("Threads whose colors differ by this or less become one thread: the one that sews more. About 2 is the smallest difference that you can see."),
                    1.0..=15.0,
                    QuickerToSew::STANDARD.merge_threads.unwrap_or_default(),
                    "",
                );
                optional(
                    ui,
                    &mut s.small_details_mm,
                    tr("Remove small details"),
                    tr("Fills, satin columns and lines smaller than this go (not in groups, such as monograms)."),
                    0.3..=5.0,
                    QuickerToSew::STANDARD.small_details_mm.unwrap_or_default(),
                    " mm",
                );
                optional(
                    ui,
                    &mut s.gap_mm,
                    tr("Sew across gaps up to"),
                    tr("A gap to the next part of the same color up to this length gets a travel stitch, not a trim. Off: the trim distance of the machine."),
                    0.0..=20.0,
                    QuickerToSew::STANDARD.gap_mm.unwrap_or_default(),
                    " mm",
                );
            });
        ui.checkbox(&mut s.fewer_color_changes, tr("Fewer color changes"))
            .on_hover_text(tr("Merge color blocks where no overlapping object is in the way. Keeps what is on top."));
        if s.gap_mm.is_none() {
            ui.weak(fill("The machine trims at gaps longer than {}.", &[&units.fmt(machine_gap)]));
        }
        ui.separator();
        let b = d.before;
        egui::Grid::new("quicker_numbers")
            .num_columns(3)
            .spacing([16.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.strong(tr("Before"));
                ui.strong(tr("After"));
                ui.end_row();
                let wait = || "…".to_string();
                let mut row = |name: &str, f: &dyn Fn(&Numbers) -> String| {
                    ui.label(name);
                    ui.label(f(&b));
                    ui.label(after.as_ref().map_or_else(wait, f));
                    ui.end_row();
                };
                row(tr("Threads"), &|n| n.threads.to_string());
                row(tr("Color changes"), &|n| n.color_changes.to_string());
                row(tr("Trims"), &|n| n.trims.to_string());
                row(tr("Stitches"), &|n| n.stitches.to_string());
                row(tr("Sew time"), &|n| {
                    n.minutes.map_or_else(|| "—".into(), crate::info::time_text)
                });
                row(tr("Travel on stitches"), &|n| units.fmt(n.travel_on_stitches_mm));
                row(tr("Travel on the fabric"), &|n| units.fmt(n.travel_on_fabric_mm));
            });
        ui.weak(tr("The canvas marks the travel that shows: red on stitches, blue on the fabric."));
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button(tr("OK")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
            if ui
                .button(tr("Standard"))
                .on_hover_text(tr("The settings when the window first opens."))
                .clicked()
            {
                d.settings = QuickerToSew::STANDARD;
            }
        });
    });
    if after.is_none() {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    if cancel || !open {
        st.close_quicker(d, false);
    } else if ok {
        st.close_quicker(d, true);
    } else {
        st.preview_quicker(&mut d);
        st.quicker_dialog = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    /// A design with two reds that look the same, a blue between them, and
    /// a speck far away.
    fn design(h: &mut Harness) {
        use emb_geom::{BBox, Path, Point};
        use emb_model::{FillParams, ObjectKind, Rgb};
        h.st.doc.edit("Design", |d| {
            *d = Design::new("Quicker");
            let red = d.add_thread("Red", Rgb(200, 30, 30));
            let red2 = d.add_thread("Red 2", Rgb(201, 31, 31));
            let blue = d.add_thread("Blue", Rgb(20, 40, 200));
            let square = |x: f64, y: f64, s: f64| ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(x, y), Point::new(x + s, y + s))),
                holes: Vec::new(),
                params: FillParams::default(),
            };
            d.add_object(red, square(-30.0, -10.0, 20.0));
            d.add_object(blue, square(-5.0, 30.0, 10.0));
            d.add_object(red2, square(12.0, -10.0, 20.0));
            d.add_object(red, square(40.0, 20.0, 0.5));
        });
        h.frame(vec![]);
    }

    /// Opens the window and shows the result of its settings.
    fn open(h: &mut Harness) {
        h.st.open_quicker();
        let mut d = h.st.quicker_dialog.take().expect("open");
        h.st.preview_quicker(&mut d);
        h.st.quicker_dialog = Some(d);
    }

    #[test]
    fn ok_is_one_undo_step_and_the_numbers_are_the_ones_shown() {
        let mut h = Harness::new();
        design(&mut h);
        let undo_before = h.st.doc.undo_label().map(str::to_string);
        let t = std::time::Instant::now();
        open(&mut h);
        let before = h.st.quicker_dialog.as_ref().unwrap().before;
        assert_eq!(before.threads, 3);
        // The numbers after show within a second.
        let mut shown = None;
        while t.elapsed().as_secs_f64() < 1.0 {
            h.frame(vec![]);
            if let Some(n) = h.st.quicker_numbers() {
                shown = Some(n);
                break;
            }
        }
        let shown = shown.expect("the numbers after show within a second");
        assert_eq!(shown.threads, 2, "the reds merge: {shown:?}");
        assert!(shown.color_changes < before.color_changes, "{before:?} {shown:?}");
        let d = h.st.quicker_dialog.take().unwrap();
        h.st.close_quicker(d, true);
        h.frame(vec![]);
        assert_eq!(h.st.quicker_numbers(), Some(shown), "the numbers after OK");
        assert_eq!(h.st.doc.design().objects().len(), 3, "the speck went");
        assert_eq!(h.st.doc.design().trim_gap_mm, Some(3.0));
        // One undo step goes back to the design before.
        h.st.undo();
        assert_eq!(h.st.doc.undo_label().map(str::to_string), undo_before);
        assert_eq!(h.st.doc.design().objects().len(), 4);
    }

    #[test]
    fn cancel_goes_back_to_the_same_design() {
        let mut h = Harness::new();
        design(&mut h);
        let text = emb_model::project::to_string(h.st.doc.design());
        open(&mut h);
        assert_ne!(emb_model::project::to_string(h.st.doc.design()), text);
        let d = h.st.quicker_dialog.take().unwrap();
        h.st.close_quicker(d, false);
        assert_eq!(emb_model::project::to_string(h.st.doc.design()), text);
    }
}
