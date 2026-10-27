//! More views (P10 Milestone 6):
//!
//! - View > New view: up to three more views of the design (with the split
//!   view, five views), each a dock tab with its own zoom, pan and thread
//!   look. They work as the split view: the design can be changed in each.
//! - View > Other design: an open design that is not the active one, next
//!   to the active one (a second group of design tabs). It is a preview
//!   with its own zoom and pan; Edit this design makes it the active one.

use std::sync::Arc;

use eframe::egui::{self, Color32, Sense, Shape};
use emb_render::Segment;

use crate::app::AppState;
use crate::canvas::{CanvasView, color32};
use crate::i18n::tr;
use crate::tabs::SplitView;

/// The most views that View > New view adds.
pub const MAX_VIEWS: usize = 3;

/// Stitch segments in canvas coordinates, with their colors.
type Segments = Arc<Vec<(Segment, Color32)>>;

/// The other design view.
#[derive(Default)]
pub struct OtherDesign {
    /// The design tab that it shows.
    pub tab: usize,
    pub view: CanvasView,
    pub fit: bool,
    /// The stitches of the design (canvas coordinates) and the state of the
    /// design they are made from.
    stitches: Option<(u64, u64, Segments)>,
}

impl AppState {
    /// Adds a view of the design (View > New view). Returns `false` if
    /// there are [`MAX_VIEWS`] already.
    pub fn new_view(&mut self) -> bool {
        let slot = match self.views.iter().position(Option::is_none) {
            Some(i) => i,
            None if self.views.len() < MAX_VIEWS => {
                self.views.push(None);
                self.views.len() - 1
            }
            None => {
                self.message = crate::i18n::fill("There can be {} more views. Close one first.", &[&MAX_VIEWS]);
                return false;
            }
        };
        self.views[slot] = Some(SplitView::default());
        true
    }

    /// Shows the other design view, with the first design tab that is not the active one.
    pub fn show_other_design(&mut self) {
        let Some(tab) = (0..self.tab_count()).find(|i| *i != self.active_tab) else {
            self.message = tr("Open a second design first (File > Open or New).").into();
            return;
        };
        self.other_design = Some(OtherDesign {
            tab,
            fit: true,
            ..Default::default()
        });
    }

    /// The stitches of design tab `i` (not the active one), in canvas
    /// coordinates, made once for each state of the design.
    fn other_stitches(&mut self, i: usize) -> Option<Segments> {
        let slot = self.tabs.get(i)?.as_ref()?;
        let key = (slot.doc_id, slot.doc.state_id());
        if let Some(o) = &self.other_design
            && let Some((id, state, segs)) = &o.stitches
            && (*id, *state) == key
        {
            return Some(segs.clone());
        }
        let d = slot.doc.design();
        let opts = d
            .machine
            .as_deref()
            .and_then(|m| self.machines.get(m))
            .map(emb_stitch::PlanOptions::for_machine)
            .unwrap_or_default();
        let plan = emb_stitch::build_plan(d, &opts, &mut emb_stitch::StitchCache::default());
        let back = d.from_hoop_transform();
        let segs: Vec<(Segment, Color32)> = emb_render::segments(&plan.stitches)
            .into_iter()
            .map(|mut s| {
                s.a = back.apply(s.a);
                s.b = back.apply(s.b);
                (s, color32(s.color))
            })
            .collect();
        let segs = Arc::new(segs);
        if let Some(o) = &mut self.other_design {
            o.stitches = Some((key.0, key.1, segs.clone()));
        }
        Some(segs)
    }
}

/// Puts the views of View > New view and the other design view in the
/// dock (right of the canvas, together), and takes out the closed ones.
pub fn sync_views(dock: &mut egui_dock::DockState<crate::app::Tab>, st: &AppState) {
    use crate::app::Tab;
    let wanted: Vec<(Tab, bool)> = (0..crate::views::MAX_VIEWS)
        .map(|i| (Tab::View(i as u8), st.views.get(i).is_some_and(Option::is_some)))
        .chain(std::iter::once((Tab::Other, st.other_design.is_some())))
        .collect();
    for (tab, on) in wanted {
        match (on, dock.find_tab(&tab)) {
            (true, None) => {
                // Next to a view that shows, else right of the canvas.
                let near = [Tab::Other, Tab::View(0), Tab::View(1), Tab::View(2), Tab::Split]
                    .iter()
                    .find_map(|t| dock.find_tab(t));
                match near {
                    Some(p) => dock[p.surface][p.node].append_tab(tab),
                    None => {
                        if let Some(p) = dock.find_tab(&Tab::Canvas) {
                            dock[p.surface].split_right(p.node, 0.5, vec![tab]);
                        }
                    }
                }
                if let Some(p) = dock.find_tab(&tab) {
                    let _ = dock.set_active_tab(p);
                }
            }
            (false, Some(p)) => {
                dock.remove_tab(p);
            }
            _ => {}
        }
    }
}

/// A view of the design (a New view tab): the canvas with the view of slot `i`.
pub fn view_canvas(ui: &mut egui::Ui, st: &mut AppState, i: usize) {
    let Some(mut v) = st.views.get_mut(i).and_then(Option::take) else {
        ui.weak(tr("This view is closed."));
        return;
    };
    crate::tabs::canvas_with(ui, st, &mut v);
    if let Some(slot) = st.views.get_mut(i) {
        *slot = Some(v);
    }
}

/// The other design view.
pub fn other_design(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(mut o) = st.other_design.take() else {
        ui.weak(tr("Use View > Other Design to show a second design here."));
        return;
    };
    let mut switch = None;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("other_design")
            .selected_text(st.tab_title(o.tab).0)
            .show_ui(ui, |ui| {
                for i in (0..st.tab_count()).filter(|i| *i != st.active_tab) {
                    if ui.selectable_label(o.tab == i, st.tab_title(i).0).clicked() {
                        o.tab = i;
                        o.fit = true;
                        o.stitches = None;
                    }
                }
            });
        if ui.small_button(tr("Zoom to design")).clicked() {
            o.fit = true;
        }
        if ui
            .button(tr("Edit this design"))
            .on_hover_text(tr("Make this design the active one (the active one shows here then)."))
            .clicked()
        {
            switch = Some(o.tab);
        }
    });
    if o.tab == st.active_tab || o.tab >= st.tab_count() {
        match (0..st.tab_count()).find(|i| *i != st.active_tab) {
            Some(i) => {
                o.tab = i;
                o.stitches = None;
                o.fit = true;
            }
            None => {
                ui.weak(tr("Only one design is open."));
                st.other_design = Some(o);
                return;
            }
        }
    }
    st.other_design = Some(o);
    let segs = st.other_stitches(st.other_design.as_ref().map_or(0, |o| o.tab));
    let Some(mut o) = st.other_design.take() else { return };
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    if let Some(segs) = &segs {
        if o.fit {
            o.fit = false;
            if let Some(b) = emb_render::bounds(&segs.iter().map(|s| s.0).collect::<Vec<_>>()) {
                o.view.fit(rect, b.expand(5.0));
            }
        }
        if let Some(pos) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                o.view.zoom_at(rect, pos, (f64::from(scroll) * 0.0015).exp());
            }
        }
        if response.dragged() {
            let d = response.drag_delta();
            let s = o.view.scale();
            o.view.center -= emb_geom::Point::new(f64::from(d.x) / s, f64::from(d.y) / s);
        }
        let mesh = if st.settings.look.three_d {
            let table = emb_render::thread3d::ShadeTable::new(st.settings.look.light);
            crate::canvas::thread_mesh_3d(rect, &o.view, segs, false, &table)
        } else {
            crate::canvas::thread_mesh(rect, &o.view, segs, false)
        };
        painter.with_clip_rect(rect).add(Shape::mesh(mesh));
    }
    st.other_design = Some(o);
    if let Some(i) = switch {
        let old = st.active_tab;
        st.switch_tab(i);
        // The design that was active shows here now.
        if let Some(o) = &mut st.other_design {
            o.tab = old;
            o.fit = true;
            o.stitches = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn views_go_in_the_dock_and_out() {
        let mut h = Harness::new();
        let mut dock = crate::app::default_dock();
        h.st.new_view();
        h.st.new_view();
        sync_views(&mut dock, &h.st);
        let (a, b) = (
            dock.find_tab(&crate::app::Tab::View(0)).unwrap(),
            dock.find_tab(&crate::app::Tab::View(1)).unwrap(),
        );
        assert_eq!((a.surface, a.node), (b.surface, b.node), "together");
        h.st.views[0] = None;
        sync_views(&mut dock, &h.st);
        assert!(dock.find_tab(&crate::app::Tab::View(0)).is_none());
        assert!(dock.find_tab(&crate::app::Tab::View(1)).is_some());
    }

    #[test]
    fn up_to_three_more_views() {
        let mut h = Harness::new();
        assert!(h.st.new_view() && h.st.new_view() && h.st.new_view());
        assert!(!h.st.new_view(), "three at most");
        // A closed view makes room.
        h.st.views[1] = None;
        assert!(h.st.new_view());
        assert!(h.st.views.iter().all(Option::is_some));
    }

    #[test]
    fn the_other_design_shows_and_switches() {
        let mut h = Harness::new();
        let first = h.st.doc.design().objects().len();
        h.st.new_design();
        assert_eq!(h.st.active_tab, 1);
        h.st.show_other_design();
        assert_eq!(h.st.other_design.as_ref().unwrap().tab, 0);
        let segs = h.st.other_stitches(0).unwrap();
        assert!(!segs.is_empty(), "the stitches of the other design");
        // Drawn in a frame, then Edit this design (as the button does).
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| other_design(ui, &mut h.st));
        });
        out.textures_delta.clear();
        h.st.switch_tab(0);
        assert_eq!(h.st.doc.design().objects().len(), first);
    }
}
