//! The stitch plan of the current design, for the Stitches view, the status
//! bar and export. It is rebuilt only when the design state changes; the
//! per-object stitches come from the `StitchCache`.
//!
//! Two ways to get it (ADR 0077):
//! - [`AppState::plan_view`]: the plan of the design as it is now. Export,
//!   output and the commands use it; it waits for the plan.
//! - [`AppState::shown_plan`]: for the drawing of each frame. A thread makes
//!   the new plan; the frame waits for it at most [`FRAME_WAIT`], then shows
//!   the last plan (`plan_waits`), so a large design (a design from a photo
//!   has hundreds of thousands of stitches) does not stop the program.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use eframe::egui::{self, Color32};
use emb_geom::Point;
use emb_render::Segment;
use emb_stitch::{PlanOptions, StitchCache, StitchPlan, build_plan};

use crate::app::AppState;
use crate::canvas::color32;

pub struct PlanView {
    /// `Document::state_id` that the plan was made from.
    pub state: u64,
    pub plan: Arc<StitchPlan>,
    /// Segments in canvas coordinates (the plan origin is the hoop center).
    pub segments: Arc<Vec<(Segment, Color32)>>,
    pub build_ms: f64,
    /// The thread that makes the plan of a newer state, if one runs. It has
    /// the stitch cache of the design; the cache comes back with the plan.
    job: Option<PlanJob>,
    /// The state whose plan thread stopped with an error: it is not tried
    /// again in the background.
    failed: Option<u64>,
    /// A plan thread asks this context to paint again when it ends, so the
    /// new plan shows with no input.
    repaint: Option<egui::Context>,
}

/// A thread that makes a stitch plan.
struct PlanJob {
    state: u64,
    rx: mpsc::Receiver<(Made, StitchCache)>,
}

/// What a plan thread made.
struct Made {
    state: u64,
    plan: StitchPlan,
    segments: Vec<(Segment, Color32)>,
    build_ms: f64,
}

/// While a drag goes on, a plan that took longer than this to make is made
/// again only when the drag stops for [`PAUSE_MS`] or ends, so the drag
/// stays smooth: the shapes move at once and the stitches follow.
pub const SLOW_PLAN_MS: f64 = 30.0;
/// See [`SLOW_PLAN_MS`].
pub const PAUSE_MS: f64 = 150.0;
/// A frame waits this long for a new plan, then it shows the last plan
/// while the thread goes on. The tests wait longer, so that they see each
/// plan in the frame that asks for it.
pub const FRAME_WAIT: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_millis(25)
};

/// Makes the plan of `design` and its segments in canvas coordinates.
fn make(state: u64, design: &emb_model::Design, opts: &PlanOptions, cache: &mut StitchCache) -> Made {
    let t0 = std::time::Instant::now();
    let plan = build_plan(design, opts, cache);
    cache.trim(design.objects().len());
    let back = design.from_hoop_transform();
    let segments = emb_render::segments(&plan.stitches)
        .into_iter()
        .map(|mut s| {
            s.a = back.apply(s.a);
            s.b = back.apply(s.b);
            (s, color32(s.color))
        })
        .collect();
    Made {
        state,
        plan,
        segments,
        build_ms: t0.elapsed().as_secs_f64() * 1000.0,
    }
}

impl AppState {
    /// Plan options from the machine profile of the design, or the defaults.
    pub fn plan_options(&self) -> PlanOptions {
        self.doc
            .design()
            .machine
            .as_deref()
            .and_then(|id| self.machines.get(id))
            .map(PlanOptions::for_machine)
            .unwrap_or_default()
    }

    /// The stitch plan for the current design state. It waits for a plan
    /// thread that runs, or makes the plan here.
    pub fn plan_view(&mut self) -> &PlanView {
        let state = self.doc.state_id();
        self.plan_waits = false;
        if let Some(job) = self.plan_view.as_mut().and_then(|p| p.job.take()) {
            match job.rx.recv() {
                Ok((made, cache)) => self.adopt(made, cache),
                Err(_) => self.lost_job(job.state),
            }
        }
        if self.plan_view.as_ref().is_none_or(|p| p.state != state) {
            let opts = self.plan_options();
            let mut cache = std::mem::take(&mut self.stitch_cache);
            let made = make(state, self.doc.design(), &opts, &mut cache);
            self.adopt(made, cache);
        }
        self.plan_view.as_ref().expect("set above")
    }

    /// The stitch plan to draw in this frame: the plan of the current state
    /// if a thread makes it within [`FRAME_WAIT`], else the last plan, with
    /// `plan_waits` set (the canvas paints again soon). See the module notes.
    pub fn shown_plan(&mut self) -> &PlanView {
        self.shown_plan_waiting(FRAME_WAIT)
    }

    /// As [`AppState::shown_plan`], with a wait of `wait` for the thread.
    fn shown_plan_waiting(&mut self, wait: Duration) -> &PlanView {
        let state = self.doc.state_id();
        self.plan_waits = false;
        // When the design state changed last.
        if self.plan_change.0 != state {
            self.plan_change = (state, std::time::Instant::now());
        }
        self.poll_plan(Duration::ZERO);
        let Some(p) = &self.plan_view else {
            // The first plan of the design is made here.
            return self.plan_view();
        };
        if p.state == state || p.failed == Some(state) {
            return self.plan_view.as_ref().expect("checked above");
        }
        let still_ms = self.plan_change.1.elapsed().as_secs_f64() * 1000.0;
        // During a drag a slow plan waits for a pause (see `SLOW_PLAN_MS`).
        let drag_waits = self.drag.is_some() && p.build_ms > SLOW_PLAN_MS && still_ms < PAUSE_MS;
        // One thread at a time: when it ends, the next frame starts one for
        // the newest state.
        let running = p.job.is_some();
        if !drag_waits || running {
            if !running {
                self.start_plan_job(state);
            }
            self.poll_plan(wait);
        }
        let p = self.plan_view.as_ref().expect("checked above");
        self.plan_waits = p.state != state;
        p
    }

    /// The context that paints the plan (the canvas gives it each frame).
    pub fn paint_plan_in(&mut self, ctx: &egui::Context) {
        if let Some(p) = self.plan_view.as_mut()
            && p.repaint.is_none()
        {
            p.repaint = Some(ctx.clone());
        }
    }

    /// Starts a thread that makes the plan of the design as it is now. If
    /// no thread can start, the plan is made here.
    fn start_plan_job(&mut self, state: u64) {
        let design = self.doc.design().clone();
        let opts = self.plan_options();
        let mut cache = std::mem::take(&mut self.stitch_cache);
        let repaint = self.plan_view.as_ref().and_then(|p| p.repaint.clone());
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("stitch-plan".into())
            .stack_size(crate::resources::JOB_STACK);
        let work = move || {
            let made = make(state, &design, &opts, &mut cache);
            let _ = tx.send((made, cache));
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        };
        match thread.spawn(work) {
            Ok(_) => {
                if let Some(p) = self.plan_view.as_mut() {
                    p.job = Some(PlanJob { state, rx });
                }
            }
            Err(_) => {
                // The cache went with the work that did not start.
                let mut cache = StitchCache::default();
                let opts = self.plan_options();
                let made = make(state, self.doc.design(), &opts, &mut cache);
                self.adopt(made, cache);
            }
        }
    }

    /// Takes the plan of the thread if it is done within `wait`.
    fn poll_plan(&mut self, wait: Duration) {
        let Some(job) = self.plan_view.as_mut().and_then(|p| p.job.take()) else {
            return;
        };
        match job.rx.recv_timeout(wait) {
            Ok((made, cache)) => self.adopt(made, cache),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(p) = self.plan_view.as_mut() {
                    p.job = Some(job);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => self.lost_job(job.state),
        }
    }

    /// A plan thread stopped with an error (its cache is lost): the last plan
    /// stays, and the state is not tried again in the background.
    fn lost_job(&mut self, state: u64) {
        if let Some(p) = self.plan_view.as_mut() {
            p.failed = Some(state);
        }
    }

    /// Makes `made` the plan of the view, and `cache` the stitch cache.
    fn adopt(&mut self, made: Made, cache: StitchCache) {
        self.stitch_cache = cache;
        // The stitch cursor stays in the new plan: at its end if it was
        // at the end of the old one (the player stops there). Else a
        // change that makes the plan shorter takes the cursor out of it,
        // and the status line of the cursor comes and goes.
        let old_len = self.plan_view.as_ref().map(|p| p.plan.stitches.records.len());
        let new_last = made.plan.stitches.records.len().saturating_sub(1);
        if let Some(c) = self.cursor.as_mut() {
            let at_end = old_len.is_some_and(|n| *c + 1 >= n);
            *c = if at_end { new_last } else { (*c).min(new_last) };
        }
        let (failed, repaint) = self
            .plan_view
            .as_ref()
            .map_or((None, None), |p| (p.failed, p.repaint.clone()));
        self.plan_view = Some(PlanView {
            state: made.state,
            plan: Arc::new(made.plan),
            segments: Arc::new(made.segments),
            build_ms: made.build_ms,
            job: None,
            failed,
            repaint,
        });
    }

    /// Position in canvas coordinates of a point of the plan.
    pub fn plan_to_canvas(&self, p: Point) -> Point {
        self.doc.design().from_hoop_transform().apply(p)
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn a_slow_plan_waits_during_a_drag() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        let first = st.shown_plan().plan.clone();
        // As if the plan took a long time to make.
        if let Some(p) = st.plan_view.as_mut() {
            p.build_ms = 10_000.0;
        }
        st.drag = Some(crate::manipulate::DragOp::Marquee {
            start: eframe::egui::Pos2::ZERO,
            current: eframe::egui::Pos2::ZERO,
            additive: false,
        });
        let id = st.doc.design().objects()[2].id;
        st.selection.set(id);
        st.nudge_selection(1.0, 0.0);
        assert!(
            Arc::ptr_eq(&first, &st.shown_plan().plan),
            "the old plan while the drag goes on"
        );
        assert!(st.plan_waits);
        // Export and the commands get the plan of the design as it is.
        assert!(!Arc::ptr_eq(&first, &st.plan_view().plan));
        assert!(!st.plan_waits);
    }

    /// A design with many large fills: its plan takes some time to make.
    fn large_design(st: &mut AppState) {
        st.doc.edit("Add fills", |d| {
            let t = d.add_thread("t", emb_model::Rgb(200, 30, 40));
            for k in 0..64 {
                let (x, y) = (f64::from(k % 8) * 12.0 - 48.0, f64::from(k / 8) * 12.0 - 30.0);
                let outer = emb_geom::Path::rect(emb_geom::BBox::new(Point::new(x, y), Point::new(x + 11.0, y + 11.0)));
                d.add_object(
                    t,
                    emb_model::ObjectKind::Fill {
                        outer,
                        holes: Vec::new(),
                        params: Default::default(),
                    },
                );
            }
        });
    }

    #[test]
    fn a_slow_plan_is_made_in_the_background_and_the_last_plan_shows() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        let small = st.shown_plan().plan.clone();
        assert!(!st.plan_waits);
        large_design(&mut st);
        // The frame does not wait: the last plan shows, and a thread makes the new one.
        let shown = st.shown_plan_waiting(Duration::ZERO).plan.clone();
        assert!(Arc::ptr_eq(&shown, &small));
        assert!(st.plan_waits);
        assert!(st.plan_view.as_ref().unwrap().job.is_some());
        // The next frames show it when it is done.
        let mut frames = 0;
        while Arc::ptr_eq(&st.shown_plan_waiting(Duration::from_millis(10)).plan, &small) {
            frames += 1;
            assert!(frames < 2000, "the plan thread did not end");
        }
        assert!(!st.plan_waits);
        let big = st.plan_view().plan.clone();
        assert!(big.stitches.stitch_count() > small.stitches.stitch_count());
        // The stitch cache came back with the plan: undo and redo make no
        // new object stitches.
        let (_, misses) = st.stitch_cache.stats();
        st.undo();
        st.redo();
        assert_eq!(st.plan_view().plan.stitches, big.stitches);
        assert_eq!(st.stitch_cache.stats().1, misses);
    }

    #[test]
    fn the_exact_plan_waits_for_the_thread() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st.shown_plan();
        large_design(&mut st);
        st.shown_plan_waiting(Duration::ZERO);
        // Export now: the plan of the design with the fills.
        let state = st.doc.state_id();
        let pv = st.plan_view();
        assert_eq!(pv.state, state);
        assert!(pv.job.is_none());
        assert!(!st.plan_waits);
    }

    #[test]
    fn plan_is_rebuilt_only_on_change_and_undo_is_a_cache_hit() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        let a = st.plan_view().plan.clone();
        let b = st.plan_view().plan.clone();
        assert!(Arc::ptr_eq(&a, &b));
        // The sample has fills, satins and lines; all are stitched.
        assert!(a.warnings.is_empty(), "{:?}", a.warnings);
        assert!(a.stitches.stitch_count() > 0);

        let id = st.doc.design().objects()[2].id; // the running line
        st.selection.set(id);
        st.nudge_selection(1.0, 0.0);
        let moved = st.plan_view().plan.clone();
        assert!(!Arc::ptr_eq(&a, &moved));
        let (_, misses_before) = st.stitch_cache.stats();
        st.undo();
        let back = st.plan_view().plan.clone();
        assert_eq!(back.stitches, a.stitches);
        assert_eq!(
            st.stitch_cache.stats().1,
            misses_before,
            "undo needs no new object stitches"
        );
    }
}
