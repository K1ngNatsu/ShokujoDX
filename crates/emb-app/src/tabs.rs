//! Design tabs (several designs open) and the split view. See
//! `docs/design/p3-plan.md` (Milestone 8).
//!
//! The active design keeps its state in the `AppState` fields, as before.
//! The other open designs wait in `AppState::tabs` as a [`DocSlot`]. A tab
//! change moves the state of the active design into its slot and takes the
//! state of the new one out.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use emb_model::{Design, Document, ObjectId};

use crate::a11y::Named;
use crate::app::AppState;
use crate::canvas::CanvasView;
use crate::file::Pending;
use crate::i18n::tr;
use crate::selection::Selection;

/// The state of an open design that is not the active one.
pub struct DocSlot {
    pub doc: Document,
    pub file_path: Option<PathBuf>,
    pub view: CanvasView,
    pub view_history: Vec<CanvasView>,
    pub selection: Selection,
    pub seq_collapsed: BTreeSet<ObjectId>,
    pub current_thread: Option<emb_model::ThreadId>,
    pub stitch_cache: emb_stitch::StitchCache,
    pub plan_view: Option<crate::preview::PlanView>,
    pub backdrop_tex: Option<crate::backdrop::BackdropTexture>,
    pub view_filter: crate::viewing::ViewFilter,
    pub cursor: Option<usize>,
    pub info_cache: Option<(u64, f64, std::sync::Arc<crate::info::DesignInfo>)>,
    /// The id of the design for its backup file.
    pub doc_id: u64,
}

/// A view of the design next to the main one (View > Split view), with its
/// own zoom, pan and thread look.
#[derive(Debug, Clone)]
pub struct SplitView {
    pub view: CanvasView,
    pub fit_requested: bool,
    pub thread_look: bool,
    pub canvas_rect: Option<egui::Rect>,
}

impl Default for SplitView {
    fn default() -> Self {
        SplitView {
            view: CanvasView::default(),
            fit_requested: true,
            thread_look: true,
            canvas_rect: None,
        }
    }
}

static NEXT_DOC_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// An id for a new open design, unique in this process.
pub fn fresh_doc_id() -> u64 {
    NEXT_DOC_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl AppState {
    /// Moves the state of the active design out, and leaves an empty one.
    fn take_slot(&mut self) -> DocSlot {
        DocSlot {
            doc: std::mem::replace(&mut self.doc, Document::new(Design::new("Untitled"))),
            file_path: self.file_path.take(),
            view: std::mem::take(&mut self.view),
            view_history: std::mem::take(&mut self.view_history),
            selection: std::mem::take(&mut self.selection),
            seq_collapsed: std::mem::take(&mut self.seq_collapsed),
            current_thread: self.current_thread.take(),
            stitch_cache: std::mem::take(&mut self.stitch_cache),
            plan_view: self.plan_view.take(),
            backdrop_tex: self.backdrop_tex.take(),
            view_filter: std::mem::take(&mut self.view_filter),
            cursor: self.cursor.take(),
            info_cache: self.info_cache.take(),
            doc_id: std::mem::replace(&mut self.doc_id, fresh_doc_id()),
        }
    }

    fn put_slot(&mut self, s: DocSlot) {
        self.doc = s.doc;
        self.file_path = s.file_path;
        self.view = s.view;
        self.view_history = s.view_history;
        self.selection = s.selection;
        self.seq_collapsed = s.seq_collapsed;
        self.current_thread = s.current_thread;
        self.stitch_cache = s.stitch_cache;
        self.plan_view = s.plan_view;
        self.backdrop_tex = s.backdrop_tex;
        self.view_filter = s.view_filter;
        self.cursor = s.cursor;
        self.info_cache = s.info_cache;
        self.doc_id = s.doc_id;
        self.reset_transient();
    }

    /// Clears the state of a tool, drag or dialog that belongs to the design
    /// that was active.
    fn reset_transient(&mut self) {
        self.drag = None;
        self.draw = Default::default();
        self.reshape = Default::default();
        self.reshape_menu_hit = None;
        self.measure = None;
        self.hovered = None;
        self.zoom_to = None;
        self.stitch_list_shown = None;
        self.export = None;
        self.output = None;
        self.preview = None;
        self.player.playing = false;
        self.hoopings_ui = Default::default();
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// The active design has nothing to lose: no objects, no changes and no file.
    pub fn is_blank(&self) -> bool {
        self.doc.design().objects().is_empty() && !self.doc.is_modified() && self.file_path.is_none()
    }

    /// Makes `i` the active tab.
    pub fn switch_tab(&mut self, i: usize) {
        if i == self.active_tab || i >= self.tabs.len() {
            return;
        }
        // The Picture to design wizard is for the picture of this tab.
        self.wizard = None;
        let cur = self.take_slot();
        self.tabs[self.active_tab] = Some(cur);
        if let Some(s) = self.tabs[i].take() {
            self.put_slot(s);
        }
        self.active_tab = i;
        self.split.fit_requested = true;
    }

    /// Opens an empty tab after the others and makes it active. A blank
    /// active design is used again instead.
    pub fn open_tab(&mut self) {
        if self.is_blank() {
            return;
        }
        let cur = self.take_slot();
        self.tabs[self.active_tab] = Some(cur);
        self.tabs.push(None);
        self.active_tab = self.tabs.len() - 1;
        self.reset_transient();
        self.fit_requested = true;
        self.split.fit_requested = true;
    }

    /// Closes the active tab with no question (see `Pending::CloseTab`).
    /// The last tab gets a new empty design.
    pub fn close_tab(&mut self) {
        self.backups_forget(self.doc_id);
        if self.tabs.len() == 1 {
            let mut d = Design::new("Untitled");
            d.machine = self.doc.design().machine.clone();
            d.hoop.selection = self.doc.design().hoop.selection.clone();
            self.take_slot();
            self.doc = Document::new(d);
            self.reset_transient();
            self.fit_requested = true;
            return;
        }
        self.tabs.remove(self.active_tab);
        self.active_tab = self.active_tab.min(self.tabs.len() - 1);
        if let Some(s) = self.tabs[self.active_tab].take() {
            self.put_slot(s);
        }
        self.split.fit_requested = true;
    }

    /// The name of tab `i`: the file name, or the design name.
    pub fn tab_title(&self, i: usize) -> (String, bool) {
        let (doc, path) = match self.tabs.get(i).and_then(Option::as_ref) {
            Some(s) => (&s.doc, s.file_path.as_deref()),
            None => (&self.doc, self.file_path.as_deref()),
        };
        let name = path
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| doc.design().name.clone());
        (name, doc.is_modified() && !doc.design().objects().is_empty())
    }

    /// Tabs whose design has changes that are not saved.
    pub fn unsaved_tabs(&self) -> Vec<usize> {
        (0..self.tabs.len()).filter(|i| self.tab_title(*i).1).collect()
    }

    /// Each open design with its id and file, the active one included.
    pub fn open_docs(&self) -> Vec<(u64, &Document, Option<&Path>)> {
        self.tabs
            .iter()
            .map(|t| match t {
                Some(s) => (s.doc_id, &s.doc, s.file_path.as_deref()),
                None => (self.doc_id, &self.doc, self.file_path.as_deref()),
            })
            .collect()
    }

    /// The tab whose file is `path`, if it is open.
    pub fn tab_of_file(&self, path: &Path) -> Option<usize> {
        (0..self.tabs.len()).find(|i| {
            let p = match &self.tabs[*i] {
                Some(s) => s.file_path.as_deref(),
                None => self.file_path.as_deref(),
            };
            p == Some(path)
        })
    }
}

/// The split view: the canvas with the view of the split, so the user can
/// work in either view. Its thread look is on by default.
pub fn split_canvas(ui: &mut egui::Ui, st: &mut AppState) {
    let mut v = std::mem::take(&mut st.split);
    canvas_with(ui, st, &mut v);
    st.split = v;
}

/// The canvas with the zoom, pan and thread look of `v` (the split view
/// and the views of View > New view).
pub fn canvas_with(ui: &mut egui::Ui, st: &mut AppState, v: &mut SplitView) {
    let swap = |st: &mut AppState, v: &mut SplitView| {
        std::mem::swap(&mut st.view, &mut v.view);
        std::mem::swap(&mut st.fit_requested, &mut v.fit_requested);
        std::mem::swap(&mut st.show_thread_look, &mut v.thread_look);
        std::mem::swap(&mut st.canvas_rect, &mut v.canvas_rect);
    };
    // Zoom commands are for the main view; the pointer position is from
    // the view under the pointer.
    let zoom_to = st.zoom_to.take();
    let cursor = st.cursor_mm;
    swap(st, v);
    ui.horizontal(|ui| {
        ui.toggle_value(&mut st.show_thread_look, tr("Thread Look"));
        if ui.small_button(tr("Zoom to design")).clicked() {
            st.fit_requested = true;
        }
    });
    crate::canvas::show(ui, st);
    swap(st, v);
    st.zoom_to = zoom_to;
    if st.cursor_mm.is_none() {
        st.cursor_mm = cursor;
    }
}

/// The row of design tabs above the canvas.
pub fn strip(ui: &mut egui::Ui, st: &mut AppState) {
    let mut switch = None;
    let mut close = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for i in 0..st.tab_count() {
            let (name, modified) = st.tab_title(i);
            let text = if modified { format!("● {name}") } else { name };
            let active = i == st.active_tab;
            let label = if active {
                RichText::new(text).strong()
            } else {
                RichText::new(text)
            };
            if ui.selectable_label(active, label).clicked() {
                switch = Some(i);
            }
            if ui
                .small_button("×")
                .on_hover_text(tr("Close this design"))
                .named(tr("Close this design"))
                .clicked()
            {
                close = Some(i);
            }
            ui.separator();
        }
        if ui
            .small_button("+")
            .on_hover_text(tr("New design in a new tab"))
            .named(tr("New design in a new tab"))
            .clicked()
        {
            st.request(Pending::New);
        }
    });
    if let Some(i) = switch {
        st.switch_tab(i);
    }
    if let Some(i) = close {
        st.switch_tab(i);
        st.request(Pending::CloseTab);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn tabs_keep_their_designs() {
        let mut h = Harness::new();
        let n_first = h.st.doc.design().objects().len();
        assert!(n_first > 0);
        h.st.selection.set(h.id(0));
        h.st.new_design();
        assert_eq!(h.st.tab_count(), 2);
        assert_eq!(h.st.active_tab, 1);
        assert!(h.st.doc.design().objects().is_empty());
        assert!(h.st.selection.is_empty());
        // A new design on a blank tab uses the same tab.
        h.st.new_design();
        assert_eq!(h.st.tab_count(), 2);
        h.st.add_sample_objects();
        h.st.switch_tab(0);
        assert_eq!(h.st.doc.design().objects().len(), n_first);
        assert_eq!(h.st.selection.len(), 1, "the selection of the tab comes back");
        // Undo works for each design on its own.
        let steps = h.st.doc.undo_len();
        h.st.switch_tab(1);
        h.st.undo();
        assert!(h.st.doc.design().objects().is_empty());
        h.st.switch_tab(0);
        assert_eq!(h.st.doc.undo_len(), steps);
        assert_eq!(h.st.unsaved_tabs(), vec![0], "tab 1 is empty again");
    }

    #[test]
    fn close_and_quit_ask_for_each_unsaved_design() {
        let mut h = Harness::new();
        h.st.new_design();
        h.st.add_sample_objects();
        assert_eq!(h.st.unsaved_tabs(), vec![0, 1]);
        h.st.request(Pending::Quit);
        assert_eq!(h.st.confirm, Some(Pending::Quit));
        let first = h.st.active_tab;
        // "Don't save" goes to the next unsaved design.
        h.st.confirm = None;
        h.st.perform(Pending::Quit);
        assert_eq!(h.st.confirm, Some(Pending::Quit));
        assert_ne!(h.st.active_tab, first);
        assert!(!h.st.close_now);
        h.st.confirm = None;
        h.st.perform(Pending::Quit);
        assert!(h.st.close_now);

        // Close: asks, then closes the tab.
        let mut h = Harness::new();
        h.st.new_design();
        h.st.add_sample_objects();
        h.st.request(Pending::CloseTab);
        assert_eq!(h.st.confirm, Some(Pending::CloseTab));
        h.st.confirm = None;
        h.st.perform(Pending::CloseTab);
        assert_eq!(h.st.tab_count(), 1);
        // The last tab closes to an empty design.
        h.st.close_tab();
        assert_eq!(h.st.tab_count(), 1);
        assert!(h.st.is_blank());
    }

    #[test]
    fn open_uses_a_new_tab_and_finds_an_open_file() {
        let mut h = Harness::new();
        let dir = std::env::temp_dir().join(format!("emb-tabs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.embproj");
        assert!(h.st.save_to(&path));
        h.st.new_design();
        h.st.add_sample_objects();
        h.st.open_path(&path);
        assert_eq!(h.st.tab_count(), 2, "the open file is shown, not opened again");
        assert_eq!(h.st.active_tab, 0);
        h.st.switch_tab(1);
        h.st.close_tab();
        h.st.close_tab();
        assert!(h.st.is_blank());
        h.st.open_path(&path);
        assert_eq!(h.st.tab_count(), 1, "a blank tab is used again");
        assert_eq!(h.st.file_path.as_deref(), Some(path.as_path()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
