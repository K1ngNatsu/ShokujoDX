//! Design repeats (P7 Milestone 6): the Repeats window, the copies on the
//! canvas (pale; they do not sew), and Make repeats.

use eframe::egui::{self, Rect, Shape};
use emb_model::{Anchor, MergeKey, RepeatSpec, StartEnd};

use crate::app::AppState;
use crate::canvas::stitch_mesh;
use crate::i18n::tr;

impl AppState {
    /// Changes the repeats as one undo step (a drag of a value is one step).
    fn edit_repeat(&mut self, merge: Option<&str>, f: impl FnOnce(&mut RepeatSpec)) {
        let change = |d: &mut emb_model::Design| {
            if let Some(r) = d.repeat.as_mut() {
                f(r);
            }
        };
        match merge {
            Some(k) => self
                .doc
                .edit_merged("Repeats", MergeKey::new(format!("repeat-{k}")), change),
            None => self.doc.edit("Repeats", change),
        }
    }

    /// Opens the Repeats window; a design with no repeats gets three copies side by side.
    pub fn open_repeats(&mut self) {
        if self.doc.design().repeat.is_none() {
            let spec = RepeatSpec::side_by_side(self.doc.design(), 0.0);
            self.doc.edit("Repeats", |d| d.repeat = Some(spec));
        }
        self.repeats_window = true;
    }

    /// Sets the start at the left and the end at the right of the design,
    /// and the step from the start to the end: a continuous border.
    pub fn join_end_to_start(&mut self) {
        self.doc.edit("Join end to start", |d| {
            if d.start_end.start.is_none() || d.start_end.end.is_none() {
                d.start_end = StartEnd {
                    start: Some(Anchor::DesignBox { x: 0.0, y: 0.5 }),
                    end: Some(Anchor::DesignBox { x: 1.0, y: 0.5 }),
                };
            }
            if let (Some(step), Some(r)) = (d.join_step(), d.repeat.as_mut()) {
                r.step_x = step;
                r.show = true;
            }
        });
        self.message = tr("Each copy now starts where the copy before it ends.").into();
    }

    /// Makes the copies as objects (one undo step).
    pub fn make_repeats(&mut self) {
        if self.doc.design().repeat.is_none() {
            self.message = tr("Set the repeats first (Design > Repeats).").into();
            return;
        }
        let new = self.doc.edit("Make Repeats", |d| d.make_repeats());
        self.message = crate::i18n::fill(
            "Made the repeats: {} new objects, copy by copy (Arrange > Group by Color puts each color together).",
            &[&(new.len())],
        );
    }
}

/// Draws the repeats (pale copies of the stitches).
pub fn draw(painter: &egui::Painter, rect: Rect, st: &mut AppState) {
    let Some(spec) = st.doc.design().repeat.filter(|r| r.show) else {
        return;
    };
    let base = st.shown_plan().segments.clone();
    let mut segs = Vec::with_capacity(base.len() * spec.offsets().len());
    for off in spec.offsets() {
        segs.extend(base.iter().map(|(s, c)| {
            let mut s = *s;
            s.a += off;
            s.b += off;
            (s, c.gamma_multiply(0.45))
        }));
    }
    let (mesh, _) = stitch_mesh(rect, &st.view, &segs, false);
    painter.add(Shape::mesh(mesh));
}

/// The Repeats window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.repeats_window {
        return;
    }
    let mut open = true;
    crate::dock::docked(
        egui::Window::new(tr("Repeats")).id(egui::Id::new("Repeats")),
        ctx,
        egui::Id::new("Repeats"),
        st.canvas_rect,
    )
    .open(&mut open)
    .resizable(false)
    .show(ctx, |ui| contents(ui, st));
    st.repeats_window = open;
}

fn contents(ui: &mut egui::Ui, st: &mut AppState) {
    ui.label(tr(
        "Show the design again in columns and rows, to check the join of a border that goes on \
         (a continuous design) or of designs that overlap. The copies do not sew until you make them.",
    ));
    let Some(r) = st.doc.design().repeat else {
        if ui.button(tr("Add repeats")).clicked() {
            st.open_repeats();
        }
        return;
    };
    let units = st.settings.units;
    let mut n = r;
    let mut show = r.show;
    if ui.checkbox(&mut show, tr("Show the repeats")).changed() {
        st.edit_repeat(None, |x| x.show = show);
    }
    egui::Grid::new("repeats").num_columns(3).show(ui, |ui| {
        ui.label(tr("Columns"));
        ui.add(
            egui::DragValue::new(&mut n.columns)
                .range(1..=20)
                .clamp_existing_to_range(false),
        );
        ui.end_row();
        ui.label(tr("Rows"));
        ui.add(
            egui::DragValue::new(&mut n.rows)
                .range(1..=20)
                .clamp_existing_to_range(false),
        );
        ui.end_row();
        ui.label(tr("Step to the right"));
        ui.horizontal(|ui| {
            crate::workspace::length_value(ui, &mut n.step_x.x, units, -2000.0, 2000.0);
            crate::workspace::length_value(ui, &mut n.step_x.y, units, -2000.0, 2000.0)
                .on_hover_text(tr("A step down too gives a slanted row."));
        });
        ui.end_row();
        ui.label(tr("Step down"));
        ui.horizontal(|ui| {
            crate::workspace::length_value(ui, &mut n.step_y.x, units, -2000.0, 2000.0)
                .on_hover_text(tr("A step to the side too gives a half-drop."));
            crate::workspace::length_value(ui, &mut n.step_y.y, units, -2000.0, 2000.0);
        });
        ui.end_row();
    });
    if n != r {
        st.edit_repeat(Some("values"), |x| *x = n);
    }
    ui.horizontal(|ui| {
        if ui
            .button(tr("Side by side"))
            .on_hover_text(tr("Steps the size of the design: the copies touch."))
            .clicked()
        {
            let s = RepeatSpec::side_by_side(st.doc.design(), 0.0);
            st.edit_repeat(None, |x| {
                x.step_x = s.step_x;
                x.step_y = s.step_y;
            });
        }
        if ui
            .button(tr("Join end to start"))
            .on_hover_text(tr(
                "Auto start at the left and auto end at the right of the design (if they are not set), \
                 and the step from the start to the end: the needle goes from copy to copy.",
            ))
            .clicked()
        {
            st.join_end_to_start();
        }
    });
    // How well the copies join.
    if let Some(step) = st.doc.design().join_step() {
        let gap = step.distance(r.step_x);
        let text = if gap < 0.05 {
            "The end of each copy is the start of the next.".to_string()
        } else {
            format!("The end of a copy is {} from the start of the next.", units.fmt(gap))
        };
        ui.weak(text);
    }
    ui.add_space(4.0);
    if ui
        .button(tr("Make Repeats"))
        .on_hover_text(tr("Make the copies as objects, after the design in the sew order."))
        .clicked()
    {
        st.make_repeats();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn repeats_show_join_and_make_in_one_step_each() {
        let mut h = Harness::new();
        let n = h.st.doc.design().objects().len();
        h.st.open_repeats();
        let r = h.st.doc.design().repeat.unwrap();
        assert_eq!((r.columns, r.rows), (3, 1));
        assert!(r.show);
        h.st.join_end_to_start();
        let d = h.st.doc.design();
        assert_eq!(d.repeat.unwrap().step_x, d.join_step().unwrap());
        let undo = h.st.doc.undo_len();
        h.st.make_repeats();
        assert_eq!(h.st.doc.design().objects().len(), 3 * n);
        assert_eq!(h.st.doc.undo_len(), undo + 1);
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), n);
    }

    #[test]
    fn the_window_and_the_copies_draw_with_no_change() {
        let mut h = Harness::new();
        h.st.open_repeats();
        let state = h.st.doc.state_id();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut h.st));
            out.textures_delta.clear();
            h.frame(vec![]);
        }
        assert_eq!(h.st.doc.state_id(), state);
    }
}
