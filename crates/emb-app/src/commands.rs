//! Commands on the selection, and the right-click menus that offer them.
//!
//! The keyboard, the main menus, the canvas right-click menu and the
//! Sequence panel right-click menu all call these functions, so each
//! command behaves the same everywhere.

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use emb_geom::{BBox, Point, Transform};
use emb_model::edit::{rotate_about, scale_about};
use emb_model::{Design, DesignObject, HoopSelection, ObjectId, Rotation, Step, Thread};
use serde::{Deserialize, Serialize};

use crate::app::{AppState, REDO, SEQ_BACK, SEQ_END, SEQ_FORWARD, SEQ_START, UNDO, ZOOM_EXTENTS};
use crate::canvas::color32;
use crate::i18n::tr;

pub const DUPLICATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
pub const ROTATE_CW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);
pub const ROTATE_CCW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::R);
pub const MIRROR_H: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::H);
pub const MIRROR_V: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::V);
/// Hide the selected objects on screen; with nothing selected, show all.
pub const HIDE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::V);

/// Arrow-key nudge distance, in mm. Shift gives the large step.
/// Nudge step in the tests (the app uses the settings).
#[cfg(test)]
pub const NUDGE_MM: f64 = 0.5;

const CLIPBOARD_FORMAT: &str = "emb-clipboard-1";

/// Objects on the clipboard, with the threads that they use. The system
/// clipboard gets the same data as JSON, so a paste works between windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clipboard {
    format: String,
    pub objects: Vec<DesignObject>,
    pub threads: Vec<Thread>,
}

impl Clipboard {
    pub fn from_json(s: &str) -> Option<Clipboard> {
        let c: Clipboard = serde_json::from_str(s).ok()?;
        (c.format == CLIPBOARD_FORMAT).then_some(c)
    }
}

impl AppState {
    pub(crate) fn selected(&self) -> Vec<ObjectId> {
        self.selection.in_sew_order(self.doc.design())
    }

    /// Applies a transform made from the selection box, as one undo step.
    pub fn transform_selection(&mut self, label: &str, make: impl FnOnce(BBox) -> Transform) {
        let ids = self.selected();
        let Some(b) = self.doc.design().bbox_of(&ids) else {
            self.message = tr("Select objects first").into();
            return;
        };
        let t = make(b);
        self.doc.edit(label, |d| d.transform_objects(&ids, &t));
        self.message = label.to_string();
    }

    /// Rotates the selection about its center. Positive turns clockwise on screen.
    pub fn rotate_selection(&mut self, deg: f64) {
        let label = format!("Rotate {deg:+}°");
        self.transform_selection(&label, |b| rotate_about(b.center(), deg));
    }

    /// Mirrors the selection about its center. `horizontal` flips left-right on screen.
    pub fn mirror_selection(&mut self, horizontal: bool) {
        // With a design rotation of 90° or 270°, screen X is design Y.
        let quarter = matches!(self.doc.design().rotation, Rotation::Deg90 | Rotation::Deg270);
        let flip_x = horizontal != quarter;
        let label = if horizontal {
            "Mirror Left-Right"
        } else {
            "Mirror Up-Down"
        };
        self.transform_selection(label, |b| {
            if flip_x {
                scale_about(b.center(), -1.0, 1.0)
            } else {
                scale_about(b.center(), 1.0, -1.0)
            }
        });
    }

    /// Moves the selection by a screen direction (`dx`, `dy` in mm). Repeated
    /// nudges join into one undo step.
    pub fn nudge_selection(&mut self, dx: f64, dy: f64) {
        let ids = self.selected();
        if ids.is_empty() {
            return;
        }
        let inv = self
            .doc
            .design()
            .rotation_transform()
            .inverse()
            .unwrap_or(Transform::IDENTITY);
        let d = inv.apply_vector(Point::new(dx, dy));
        self.doc
            .edit_merged("Nudge", emb_model::MergeKey::new("nudge"), |design| {
                design.transform_objects(&ids, &Transform::translate(d.x, d.y))
            });
    }

    pub fn duplicate_selection(&mut self) {
        let ids = self.selected();
        if ids.is_empty() {
            return;
        }
        let inv = self
            .doc
            .design()
            .rotation_transform()
            .inverse()
            .unwrap_or(Transform::IDENTITY);
        let o = self.settings.editing.duplicate_offset_mm;
        let off = inv.apply_vector(Point::new(o, o));
        let new = self.doc.edit("Duplicate", |d| d.duplicate_objects(&ids, off));
        self.selection.set_all(new);
        self.message = tr("Duplicate").into();
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        let d = self.doc.design();
        let objects: Vec<DesignObject> = self
            .selected()
            .into_iter()
            .filter_map(|id| d.object(id).cloned())
            .collect();
        if objects.is_empty() {
            return;
        }
        let threads = d.threads.clone();
        let clip = Clipboard {
            format: CLIPBOARD_FORMAT.into(),
            objects,
            threads,
        };
        if let Ok(json) = serde_json::to_string(&clip) {
            ctx.copy_text(json);
        }
        self.message = crate::i18n::fill("Copied {} object(s)", &[&(clip.objects.len())]);
        self.clipboard = Some(clip);
    }

    pub fn cut_selection(&mut self, ctx: &egui::Context) {
        self.copy_selection(ctx);
        let ids = self.selected();
        if ids.is_empty() {
            return;
        }
        self.doc.edit("Cut", |d| {
            for id in &ids {
                d.remove_object(*id);
            }
        });
        self.selection.clear();
    }

    /// Selects all objects that the canvas shows.
    pub fn select_all(&mut self) {
        let ids: Vec<ObjectId> = self
            .doc
            .design()
            .objects()
            .iter()
            .filter(|o| self.view_filter.shows(o))
            .map(|o| o.id)
            .collect();
        self.selection.set_all(ids);
    }

    /// Alt+V: hides the selected objects on screen (they still sew and
    /// save); with nothing selected, shows all objects again.
    pub fn hide_selection(&mut self) {
        if self.selection.is_empty() {
            if self.view_filter.is_active() {
                self.view_filter = crate::viewing::ViewFilter::default();
                self.message = tr("All objects are shown.").into();
            }
            return;
        }
        let n = self.selection.len();
        self.view_filter.hidden.extend(self.selection.ids().iter().copied());
        self.selection.clear();
        self.message = crate::i18n::fill(
            "{} object{} hidden on screen (they still sew). Alt+V with nothing selected shows all.",
            &[&n, &(if n == 1 { "" } else { "s" })],
        );
    }

    /// The hoop center, or the design origin in open field.
    fn hoop_center(&self) -> Point {
        self.doc.design().hoop.center
    }

    /// Moves the selection (or the full design if nothing is selected) to the hoop center.
    pub fn center_in_hoop(&mut self) {
        let ids = if self.selection.is_empty() {
            self.doc.design().objects().iter().map(|o| o.id).collect()
        } else {
            self.selected()
        };
        let target = self.hoop_center();
        let label = if self.selection.is_empty() {
            "Center Design in Hoop"
        } else {
            "Center in Hoop"
        };
        self.resequence(label, |d| d.center_objects_at(&ids, target));
    }

    pub fn zoom_to_selection(&mut self) {
        let d = self.doc.design();
        let t = d.rotation_transform();
        if let Some(b) = d.bbox_of(&self.selected()) {
            let corners = [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)];
            self.zoom_to = BBox::from_points(corners.map(|p| t.apply(p))).map(|b| b.expand(2.0));
        }
    }

    pub fn set_thread(&mut self, thread: emb_model::ThreadId) {
        let ids = self.selected();
        self.doc.edit("Change thread", |d| {
            for id in &ids {
                if let Some(o) = d.object_mut(*id) {
                    o.thread = thread;
                }
            }
        });
    }
}

fn item(ui: &mut egui::Ui, enabled: bool, label: impl Into<String>, shortcut: &str) -> bool {
    let clicked = ui
        .add_enabled(enabled, egui::Button::new(label.into()).shortcut_text(shortcut))
        .clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn undo_redo_items(ui: &mut egui::Ui, st: &mut AppState) {
    let fmt = |s: &KeyboardShortcut| ui.ctx().format_shortcut(s);
    let (undo_key, redo_key) = (fmt(&UNDO), fmt(&REDO));
    let undo = st
        .doc
        .undo_label()
        .map(|l| crate::i18n::fill("Undo {}", &[&l]))
        .unwrap_or_else(|| tr("Undo").into());
    if item(ui, st.doc.can_undo(), undo, &undo_key) {
        st.undo();
    }
    let redo = st
        .doc
        .redo_label()
        .map(|l| crate::i18n::fill("Redo {}", &[&l]))
        .unwrap_or_else(|| tr("Redo").into());
    if item(ui, st.doc.can_redo(), redo, &redo_key) {
        st.redo();
    }
}

/// Right-click menu for selected objects (canvas and Sequence panel).
pub fn object_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let ctx = ui.ctx().clone();
    let fmt = |s: &KeyboardShortcut| ctx.format_shortcut(s);
    let n = st.selection.len();
    ui.label(
        egui::RichText::new(if n == 1 {
            tr("1 object").to_string()
        } else {
            crate::i18n::fill("{} objects", &[&n])
        })
        .weak(),
    );
    if let Some(target) = st.edit_target() {
        let (text, hint) = match target {
            crate::lettering::EditTarget::Monogram => (tr("Edit Monogram…"), ""),
            crate::lettering::EditTarget::Lettering(_) => (tr("Edit Text…"), tr("Double-click")),
        };
        if item(ui, true, text, hint) {
            st.edit_selection();
        }
        ui.separator();
    }
    // A sketch, or an object made from one (ADR 0076).
    if let Some(sketch) = st.selection.single().and_then(|id| st.doc.design().sketch_for(id)) {
        if item(ui, true, tr("Edit Sketch"), tr("Double-click")) {
            st.open_sketch(sketch);
        }
        ui.separator();
    }
    // Vectors to embroidery, and embroidery to vectors (ADR 0024).
    let s = crate::ribbon::SelInfo::of(st);
    if s.vectors > 0 || s.n > 0 {
        ui.menu_button(tr("Convert"), |ui| {
            for to in emb_model::ConvertTo::ALL {
                let text = crate::i18n::fill("Vector to {}", &[&tr(to.name()).to_lowercase()]);
                if item(ui, s.vectors > 0, &text, "") {
                    st.convert_vectors(to);
                }
            }
            if item(ui, s.n > s.vectors, tr("Embroidery to Vector"), "") {
                st.convert_to_vectors();
            }
            if item(ui, s.vectors > 0, tr("Vector to Sketch"), "") {
                st.vectors_to_sketch();
            }
        });
        if s.shapes >= 2 || s.vectors >= 2 || s.appliques >= 2 {
            ui.menu_button(tr("Shaping"), |ui| {
                for op in emb_model::ShapeOp::ALL {
                    let cmd = crate::ribbon::Cmd::Shape(op);
                    if item(ui, st.can(cmd, &s), cmd.meta().1, "") {
                        st.shape_selection(op);
                    }
                }
            });
        }
        ui.separator();
    }
    undo_redo_items(ui, st);
    ui.separator();

    let cmd = |k: Key| fmt(&KeyboardShortcut::new(Modifiers::COMMAND, k));
    if item(ui, true, tr("Cut"), &cmd(Key::X)) {
        st.cut_selection(&ctx);
    }
    if item(ui, true, tr("Copy"), &cmd(Key::C)) {
        st.copy_selection(&ctx);
    }
    if item(ui, st.clipboard.is_some(), tr("Paste"), &cmd(Key::V)) {
        st.paste(None);
    }
    if item(ui, st.clipboard.is_some(), tr("Paste After Selected"), "") {
        st.paste_after_selected();
    }
    if item(ui, st.clipboard.is_some(), tr("Paste Special…"), "") {
        st.open_paste_special();
    }
    if item(ui, true, tr("Duplicate"), &fmt(&DUPLICATE)) {
        st.duplicate_selection();
    }
    if item(ui, true, tr("Delete"), "Del") {
        st.delete_selected();
    }
    ui.separator();

    ui.menu_button(tr("Rotate and Mirror"), |ui| {
        if item(ui, true, tr("Rotate 90° Clockwise"), &fmt(&ROTATE_CW)) {
            st.rotate_selection(90.0);
        }
        if item(ui, true, tr("Rotate 90° Counterclockwise"), &fmt(&ROTATE_CCW)) {
            st.rotate_selection(-90.0);
        }
        if item(ui, true, tr("Rotate 180°"), "") {
            st.rotate_selection(180.0);
        }
        if item(ui, true, tr("Rotate 15° Clockwise"), "") {
            st.rotate_selection(15.0);
        }
        if item(ui, true, tr("Rotate 15° Counterclockwise"), "") {
            st.rotate_selection(-15.0);
        }
        ui.separator();
        if item(ui, true, tr("Mirror Left-Right"), &fmt(&MIRROR_H)) {
            st.mirror_selection(true);
        }
        if item(ui, true, tr("Mirror Up-Down"), &fmt(&MIRROR_V)) {
            st.mirror_selection(false);
        }
    });
    if item(ui, true, tr("Center in Hoop"), "") {
        st.center_in_hoop();
    }
    crate::menus::edit_items(ui, st);
    ui.menu_button(tr("Sew Order"), |ui| {
        let ops: [(&str, KeyboardShortcut, Step); 2] = [
            (tr("Move Back One"), SEQ_BACK, Step::Backward),
            (tr("Move Forward One"), SEQ_FORWARD, Step::Forward),
        ];
        if item(ui, true, tr("Move to Start"), &fmt(&SEQ_START)) {
            st.sequence_selected("Move to Start", |d, ids| d.move_objects_to_start(ids));
        }
        for (label, key, step) in ops {
            if item(ui, true, label, &fmt(&key)) {
                st.sequence_selected(label, |d, ids| d.step_objects(ids, step));
            }
        }
        if item(ui, true, tr("Move to End"), &fmt(&SEQ_END)) {
            st.sequence_selected("Move to End", |d, ids| d.move_objects_to_end(ids));
        }
        if item(ui, n > 1, tr("Sequence by Click Order"), "") {
            let ids = st.selection.ids().to_vec();
            st.resequence("Sequence by Click Order", |d| d.sequence_in_order(&ids));
        }
    });
    ui.menu_button(tr("Thread"), |ui| {
        let threads = st.doc.design().threads.clone();
        for t in threads {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::Vec2::splat(12.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 2.0, color32(t.color));
                if ui.button(&t.name).clicked() {
                    st.set_thread(t.id);
                    ui.close();
                }
            });
        }
    });
    ui.separator();
    if item(ui, true, tr("Zoom to Selection"), "") {
        st.zoom_to_selection();
    }
    if item(
        ui,
        true,
        tr("Zoom Extents"),
        &crate::i18n::fill("{} / double-click wheel", &[&fmt(&ZOOM_EXTENTS)]),
    ) {
        st.fit_requested = true;
    }
    if item(ui, true, tr("Properties…"), "") {
        st.focus_properties = true;
    }
}

/// Right-click menu for the empty canvas.
pub fn canvas_menu(ui: &mut egui::Ui, st: &mut AppState) {
    if !st.selection.is_empty() {
        object_menu(ui, st);
        return;
    }
    let ctx = ui.ctx().clone();
    let fmt = |s: &KeyboardShortcut| ctx.format_shortcut(s);
    undo_redo_items(ui, st);
    ui.separator();
    if item(
        ui,
        st.clipboard.is_some(),
        tr("Paste"),
        &fmt(&KeyboardShortcut::new(Modifiers::COMMAND, Key::V)),
    ) {
        st.paste(None);
    }
    if item(ui, st.clipboard.is_some(), tr("Paste Special…"), "") {
        st.open_paste_special();
    }
    let has_objects = !st.doc.design().objects().is_empty();
    if item(
        ui,
        has_objects,
        tr("Select All"),
        &fmt(&KeyboardShortcut::new(Modifiers::COMMAND, Key::A)),
    ) {
        st.select_all();
    }
    ui.separator();
    let in_hoop = matches!(st.doc.design().hoop.selection, HoopSelection::Preset { .. });
    if item(ui, has_objects && in_hoop, tr("Center Design in Hoop"), "") {
        st.center_in_hoop();
    }
    if item(ui, true, tr("Rotate Design 90° in Hoop"), "") {
        st.doc
            .edit("Rotate Design", |d: &mut Design| d.rotation = d.rotation.turned_cw());
    }
    ui.separator();
    if item(
        ui,
        true,
        tr("Zoom Extents"),
        &crate::i18n::fill("{} / double-click wheel", &[&fmt(&ZOOM_EXTENTS)]),
    ) {
        st.fit_requested = true;
    }
    if item(ui, true, tr("Zoom 100 %"), "") {
        st.view.zoom = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn clipboard_round_trip_through_json() {
        let mut st = state();
        let ctx = egui::Context::default();
        st.select_all();
        st.copy_selection(&ctx);
        let json = serde_json::to_string(st.clipboard.as_ref().unwrap()).unwrap();
        let clip = Clipboard::from_json(&json).unwrap();
        let n = st.doc.design().objects().len();
        st.paste(Some(clip));
        assert_eq!(st.doc.design().objects().len(), 2 * n);
        assert_eq!(st.selection.len(), n);
        assert!(Clipboard::from_json("plain text").is_none());
    }

    #[test]
    fn mirror_follows_screen_axis_when_design_is_rotated() {
        let mut st = state();
        let id = st.doc.design().objects()[3].id; // The star: not symmetric up-down.
        st.selection.set(id);
        let before = st.doc.design().object(id).cloned().unwrap();
        st.doc.edit("Rotate Design", |d| d.rotation = Rotation::Deg90);
        st.mirror_selection(true);
        // Screen left-right with the design turned 90° is design up-down: X stays.
        let after = st.doc.design().object(id).cloned().unwrap();
        let (b0, b1) = (before.control_bbox().unwrap(), after.control_bbox().unwrap());
        assert!((b0.min.x - b1.min.x).abs() < 1e-9 && (b0.max.x - b1.max.x).abs() < 1e-9);
        assert_eq!(st.doc.undo_label(), Some("Mirror Left-Right"));
    }

    #[test]
    fn nudges_join_into_one_undo_step() {
        let mut st = state();
        st.selection.set(st.doc.design().objects()[0].id);
        let steps = st.doc.undo_len();
        for _ in 0..5 {
            st.nudge_selection(NUDGE_MM, 0.0);
        }
        assert_eq!(st.doc.undo_len(), steps + 1);
        st.rotate_selection(90.0);
        assert_eq!(st.doc.undo_len(), steps + 2);
    }

    #[test]
    fn duplicate_selects_the_copies() {
        let mut st = state();
        let id = st.doc.design().objects()[0].id;
        st.selection.set(id);
        st.duplicate_selection();
        assert_eq!(st.selection.len(), 1);
        assert_ne!(st.selection.single(), Some(id));
    }
}
