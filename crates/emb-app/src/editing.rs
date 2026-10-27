//! Editing helpers (P10 Milestone 1): paste after selected, paste special,
//! close shape, reverse direction, swap sides, to single stones, and the
//! editing settings of the Options window.
//!
//! - Paste puts the copies where the editing settings say (Options): at the
//!   same place (the default), centered on the hoop or centered on the
//!   stitch cursor; and at the end of the sew order, or just after the
//!   selected objects.
//! - Paste special opens a window to choose the place for one paste. It
//!   has two more places: moved by a distance, and the first stitch at the
//!   stitch cursor.
//! - The model functions are in `emb_model::editing`.

use eframe::egui;
use emb_geom::{BBox, Point, Transform};
use emb_model::{ObjectId, ObjectKind};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::commands::Clipboard;
use crate::i18n::tr;
use crate::workspace::{Units, length_value};

/// Where pasted objects go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PastePlace {
    /// The objects keep their coordinates.
    #[default]
    Same,
    /// Moved by the distance of the paste settings.
    Offset,
    /// The center of their box at the hoop center.
    HoopCenter,
    /// The center of their box at the stitch cursor.
    CursorCenter,
    /// The first stitch at the stitch cursor.
    CursorStart,
}

impl PastePlace {
    pub const ALL: [PastePlace; 5] = [
        PastePlace::Same,
        PastePlace::Offset,
        PastePlace::HoopCenter,
        PastePlace::CursorCenter,
        PastePlace::CursorStart,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PastePlace::Same => tr("At the same place"),
            PastePlace::Offset => tr("Moved by a distance"),
            PastePlace::HoopCenter => tr("Centered on the hoop"),
            PastePlace::CursorCenter => tr("Centered on the stitch cursor"),
            PastePlace::CursorStart => tr("First stitch at the stitch cursor"),
        }
    }

    fn needs_cursor(self) -> bool {
        matches!(self, PastePlace::CursorCenter | PastePlace::CursorStart)
    }
}

/// How a paste puts the objects.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PasteHow {
    pub place: PastePlace,
    /// The distance of [`PastePlace::Offset`], in screen directions (right
    /// and down), mm.
    pub dx_mm: f64,
    pub dy_mm: f64,
    /// The copies go just after the last selected object in the sew order
    /// (else at the end).
    pub after_selected: bool,
}

impl Default for PasteHow {
    fn default() -> Self {
        PasteHow {
            place: PastePlace::Same,
            dx_mm: 5.0,
            dy_mm: 5.0,
            after_selected: false,
        }
    }
}

/// The editing settings (Options).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditSettings {
    /// What Paste (Ctrl+V) does.
    pub paste: PasteHow,
    /// The offset of a duplicate, in screen directions (right and down), mm.
    pub duplicate_offset_mm: f64,
    /// Reshape shows the handles of all nodes, not only of the selected node.
    pub all_handles: bool,
}

impl Default for EditSettings {
    fn default() -> Self {
        EditSettings {
            paste: PasteHow::default(),
            duplicate_offset_mm: 5.0,
            all_handles: false,
        }
    }
}

/// The Paste special window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PasteSpecial {
    pub how: PasteHow,
}

/// Objects whose direction reverse changes.
pub fn reversible(k: &ObjectKind) -> bool {
    match k {
        ObjectKind::Running { .. } | ObjectKind::Vector { .. } | ObjectKind::Satin { .. } => true,
        ObjectKind::Bling(b) => matches!(b.layout, emb_model::BlingLayout::Run { .. }),
        _ => false,
    }
}

/// Objects with an open part that close shape can close.
pub fn has_open_part(k: &ObjectKind) -> bool {
    let open = |p: &emb_geom::Path| {
        emb_geom::nodes::NodePath::from_path(p)
            .subs
            .iter()
            .any(|s| !s.closed && s.nodes.len() >= 2)
    };
    match k {
        ObjectKind::Running { path, .. } | ObjectKind::Vector { path, .. } => open(path),
        ObjectKind::Bling(b) => match &b.layout {
            emb_model::BlingLayout::Run { path } => open(path),
            _ => false,
        },
        _ => false,
    }
}

/// Stone runs and fills (not single stones).
pub fn stone_pattern(k: &ObjectKind) -> bool {
    matches!(k, ObjectKind::Bling(b) if !matches!(b.layout, emb_model::BlingLayout::Stones { .. }))
}

impl AppState {
    /// The number of selected objects for which `f` is true.
    pub fn selected_count(&self, f: fn(&ObjectKind) -> bool) -> usize {
        let d = self.doc.design();
        self.selection
            .ids()
            .iter()
            .filter_map(|id| d.object(*id))
            .filter(|o| f(&o.kind))
            .count()
    }

    /// The stitch cursor in design coordinates.
    pub fn cursor_design_point(&self) -> Option<Point> {
        let i = self.cursor?;
        let pv = self.plan_view.as_ref()?;
        let r = pv.plan.stitches.records.get(i)?;
        Some(self.doc.design().output_transform().inverse()?.apply(r.pos))
    }

    /// The first stitch of an object in design coordinates (its start point
    /// if it has one).
    fn first_stitch(&mut self, id: ObjectId) -> Option<Point> {
        if let Some(p) = self.doc.design().object(id).and_then(|o| o.start) {
            return Some(p);
        }
        // Where the object starts in the plan (as the Reshape markers).
        let inv = self.doc.design().output_transform().inverse()?;
        let span = self.plan_view().plan.spans.iter().find(|s| s.object == id)?;
        Some(inv.apply(span.start))
    }

    /// Pastes with the editing settings (Ctrl+V).
    pub fn paste(&mut self, from_system: Option<Clipboard>) {
        let how = self.settings.editing.paste;
        self.paste_how(from_system, how);
    }

    /// Pastes the clipboard (`from_system` if it is clipboard data from this
    /// app, else the internal clipboard) as `how` says.
    pub fn paste_how(&mut self, from_system: Option<Clipboard>, how: PasteHow) {
        let Some(clip) = from_system.or_else(|| self.clipboard.clone()) else {
            self.message = tr("Nothing to paste").into();
            return;
        };
        // The cursor in the plan of the design as it is now.
        self.plan_view();
        let cursor = self.cursor_design_point();
        if how.place.needs_cursor() && cursor.is_none() {
            self.message = tr("Set the stitch cursor first (the stitch player or the stitch list).").into();
            return;
        }
        let after = how.after_selected.then(|| self.selected().last().copied()).flatten();
        let label = if after.is_some() {
            "Paste After Selected"
        } else {
            "Paste"
        };
        // The paste and the move are one undo step.
        self.drag_counter += 1;
        let key = emb_model::MergeKey::new(format!("paste-{}", self.drag_counter));
        self.doc.end_merge();
        let new = self.doc.edit_merged(label, key.clone(), |d| match after {
            Some(a) => d.paste_objects_after(&clip.objects, &clip.threads, a),
            None => d.paste_objects(&clip.objects, &clip.threads),
        });
        let target = match how.place {
            PastePlace::CursorStart => {
                let first = new.first().copied().and_then(|id| self.first_stitch(id));
                first.zip(cursor).map(|(f, c)| c - f)
            }
            place => {
                let d = self.doc.design();
                let b: Option<BBox> = d.bbox_of(&new);
                match place {
                    PastePlace::Offset => {
                        let inv = d.rotation_transform().inverse().unwrap_or(Transform::IDENTITY);
                        Some(inv.apply_vector(Point::new(how.dx_mm, how.dy_mm)))
                    }
                    PastePlace::HoopCenter => b.map(|b| d.hoop.center - b.center()),
                    PastePlace::CursorCenter => b.zip(cursor).map(|(b, c)| c - b.center()),
                    _ => None,
                }
            }
        };
        if let Some(v) = target.filter(|v| v.length() > 1e-9) {
            let t = Transform::translate(v.x, v.y);
            self.doc
                .edit_merged(label, key.clone(), |d| d.transform_objects(&new, &t));
        }
        // The first object starts at the cursor (else the program can start
        // it at its other end).
        if how.place == PastePlace::CursorStart
            && let (Some(&first), Some(c)) = (new.first(), cursor)
        {
            self.doc.edit_merged(label, key, |d| {
                if let Some(o) = d.object_mut(first)
                    && o.start.is_none()
                {
                    o.start = Some(c);
                }
            });
        }
        self.doc.end_merge();
        self.message = crate::i18n::fill("Pasted {} object(s)", &[&(new.len())]);
        self.selection.set_all(new);
    }

    /// Paste after selected: at the place of the settings, just after the
    /// selection in the sew order.
    pub fn paste_after_selected(&mut self) {
        let how = PasteHow {
            after_selected: true,
            ..self.settings.editing.paste
        };
        self.paste_how(None, how);
    }

    pub fn open_paste_special(&mut self) {
        self.paste_special = Some(PasteSpecial {
            how: self.settings.editing.paste,
        });
    }

    /// Reverses the direction of the selected objects.
    pub fn reverse_selection(&mut self) {
        let ids = self.selected();
        let n = self.doc.edit("Reverse Direction", |d| d.reverse_objects(&ids));
        self.message = match n {
            0 => "Select lines, columns, vectors or stone runs.".into(),
            n => format!("Reversed the direction of {n} object(s)."),
        };
    }

    /// Swaps the sides of the selected satin columns.
    pub fn swap_sides(&mut self) {
        let ids = self.selected();
        let n = self.doc.edit("Swap Sides", |d| d.swap_column_sides(&ids));
        self.message = match n {
            0 => "Select satin columns.".into(),
            n => format!("Swapped the sides of {n} column(s)."),
        };
    }

    /// Closes the open parts of the selected lines, vectors and stone runs.
    pub fn close_shapes(&mut self, curve: bool) {
        let ids = self.selected();
        let label = if curve {
            "Close shape with a curve"
        } else {
            "Close Shape"
        };
        let n = self.doc.edit(label, |d| d.close_shapes(&ids, curve));
        self.message = match n {
            0 => "Select open lines, vectors or stone runs.".into(),
            n => format!("Closed {n} object(s)."),
        };
    }

    /// Changes the selected stone runs and fills to single stones.
    pub fn make_single_stones(&mut self) {
        let ids = self.selected();
        let n = self.doc.edit("To Single Stones", |d| d.to_single_stones(&ids));
        self.message = match n {
            0 => "Select stone runs or stone fills.".into(),
            n => format!("{n} object(s) are single stones now. Reshape moves each stone."),
        };
    }
}

/// The place choices and the "after selected" choice.
fn how_ui(ui: &mut egui::Ui, how: &mut PasteHow, units: Units, places: &[PastePlace]) {
    for &p in places {
        ui.radio_value(&mut how.place, p, p.label());
    }
    if places.contains(&PastePlace::Offset) {
        ui.add_enabled_ui(how.place == PastePlace::Offset, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Right"));
                length_value(ui, &mut how.dx_mm, units, -500.0, 500.0);
                ui.label(tr("Down"));
                length_value(ui, &mut how.dy_mm, units, -500.0, 500.0);
            });
        });
    }
    ui.checkbox(
        &mut how.after_selected,
        tr("Just after the selected objects in the sew order"),
    )
    .on_hover_text(tr("Else the copies go at the end of the sew order."));
}

/// The Paste special window.
pub fn paste_special_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut w) = st.paste_special else {
        return;
    };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    let has_cursor = st.cursor.is_some();
    crate::dock::docked(
        egui::Window::new(tr("Paste Special")).id(egui::Id::new("Paste Special")),
        ctx,
        egui::Id::new("Paste Special"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        ui.label(tr("Put the copies:"));
        how_ui(ui, &mut w.how, st.settings.units, &PastePlace::ALL);
        if w.how.place.needs_cursor() && !has_cursor {
            ui.colored_label(ui.visuals().warn_fg_color, tr("Set the stitch cursor first."));
        }
        ui.separator();
        ui.horizontal(|ui| {
            let can = st.clipboard.is_some() && (has_cursor || !w.how.place.needs_cursor());
            ok = ui.add_enabled(can, egui::Button::new(tr("Paste"))).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
        });
    });
    st.paste_special = (open && !ok && !cancel).then_some(w);
    if ok {
        st.paste_how(None, w.how);
    }
}

/// The editing settings in the Options window.
pub fn options_ui(ui: &mut egui::Ui, e: &mut EditSettings, units: Units) {
    ui.strong(tr("Editing"));
    egui::Grid::new("options-editing")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("Paste (Ctrl+V)"));
            ui.vertical(|ui| {
                how_ui(
                    ui,
                    &mut e.paste,
                    units,
                    &[
                        PastePlace::Same,
                        PastePlace::Offset,
                        PastePlace::HoopCenter,
                        PastePlace::CursorCenter,
                    ],
                );
            });
            ui.end_row();
            ui.label(tr("Duplicate offset"));
            length_value(ui, &mut e.duplicate_offset_mm, units, 0.0, 100.0)
                .on_hover_text(tr("A duplicate goes this far right and down."));
            ui.end_row();
            ui.label(tr("Reshape"));
            ui.checkbox(&mut e.all_handles, tr("Show the handles of all nodes"))
                .on_hover_text(tr("Else only the selected node shows its handles."));
            ui.end_row();
        });
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::RunningParams;

    use super::*;
    use crate::test_util::Harness;

    fn line(h: &mut Harness, x: f64) -> ObjectId {
        h.st.doc.edit("t", |d| {
            let t = match d.threads.first() {
                Some(t) => t.id,
                None => d.add_thread("T", emb_model::Rgb(0, 0, 0)),
            };
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(x, 0.0))
                        .line_to(Point::new(x + 10.0, 0.0)),
                    params: RunningParams::default(),
                },
            )
        })
    }

    #[test]
    fn paste_places() {
        let mut h = Harness::empty();
        let a = line(&mut h, 0.0);
        let b = line(&mut h, 20.0);
        h.st.selection.set(a);
        h.st.copy_selection(&h.ctx.clone());
        // At the same place, at the end.
        h.st.paste(None);
        let p1 = h.st.selected()[0];
        assert_eq!(h.st.doc.design().objects().last().unwrap().id, p1);
        // Paste after selected: just after the selection.
        h.st.selection.set(a);
        h.st.paste_after_selected();
        let p2 = h.st.selected()[0];
        let order: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        assert_eq!(order, vec![a, p2, b, p1]);
        // Centered on the hoop.
        h.st.paste_how(
            None,
            PasteHow {
                place: PastePlace::HoopCenter,
                ..PasteHow::default()
            },
        );
        let d = h.st.doc.design();
        let c = d.bbox_of(&h.st.selected()).unwrap().center();
        assert!(c.distance(d.hoop.center) < 1e-9);
        // Moved by a distance: one undo step takes the paste back.
        let n = h.st.doc.design().objects().len();
        h.st.paste_how(
            None,
            PasteHow {
                place: PastePlace::Offset,
                dx_mm: 3.0,
                dy_mm: 0.0,
                ..PasteHow::default()
            },
        );
        let b = h.st.doc.design().bbox_of(&h.st.selected()).unwrap();
        assert!((b.min.x - 3.0).abs() < 1e-9);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), n);
    }

    #[test]
    fn paste_at_the_stitch_cursor() {
        let mut h = Harness::empty();
        let a = line(&mut h, 0.0);
        let _b = line(&mut h, 30.0);
        h.st.selection.set(a);
        h.st.copy_selection(&h.ctx.clone());
        // No cursor: nothing happens.
        let n = h.st.doc.design().objects().len();
        let start = PasteHow {
            place: PastePlace::CursorStart,
            ..PasteHow::default()
        };
        h.st.paste_how(None, start);
        assert_eq!(h.st.doc.design().objects().len(), n);
        // The cursor at the last stitch.
        let last = h.st.plan_view().plan.stitches.records.len() - 1;
        h.st.cursor = Some(last);
        let c = h.st.cursor_design_point().unwrap();
        h.st.paste_how(None, start);
        let id = h.st.selected()[0];
        // The start of the object in the plan, not its start point.
        let inv = h.st.doc.design().output_transform().inverse().unwrap();
        let first = inv.apply(
            h.st.plan_view()
                .plan
                .spans
                .iter()
                .find(|s| s.object == id)
                .unwrap()
                .start,
        );
        assert!(first.distance(c) < 0.05, "{first:?} {c:?}");
        h.st.plan_view();
        let c = h.st.cursor_design_point().unwrap();
        h.st.paste_how(
            None,
            PasteHow {
                place: PastePlace::CursorCenter,
                ..PasteHow::default()
            },
        );
        let d = h.st.doc.design();
        assert!(d.bbox_of(&h.st.selected()).unwrap().center().distance(c) < 1e-9);
    }

    #[test]
    fn commands_reverse_close_and_single_stones() {
        let mut h = Harness::empty();
        let a = line(&mut h, 0.0);
        h.st.selection.set(a);
        assert_eq!(h.st.selected_count(reversible), 1);
        assert_eq!(h.st.selected_count(has_open_part), 1);
        h.st.reverse_selection();
        let ObjectKind::Running { path, .. } = &h.st.doc.design().object(a).unwrap().kind else {
            panic!()
        };
        assert_eq!(path.flatten(0.01)[0].points[0], Point::new(10.0, 0.0));
        // Two points: only a curve closes it.
        h.st.close_shapes(false);
        assert_eq!(h.st.selected_count(has_open_part), 1);
        h.st.close_shapes(true);
        assert_eq!(h.st.selected_count(has_open_part), 0);
        h.st.swap_sides();
        assert!(h.st.message.contains("satin"));
    }
}
