//! The Arrange menu and the edit commands of the object menu.

use eframe::egui;
use emb_model::RunPass;

use crate::actions::Align;
use crate::app::AppState;
use crate::draw::Tool;
use crate::i18n::tr;

impl AppState {
    pub fn arrange_menu(&mut self, ui: &mut egui::Ui) {
        let n = self.selection.len();
        ui.menu_button(tr("Align"), |ui| {
            for (label, a) in [
                (tr("Left"), Align::Left),
                (tr("Centers Left-Right"), Align::CenterX),
                (tr("Right"), Align::Right),
                (tr("Top"), Align::Top),
                (tr("Centers Up-Down"), Align::CenterY),
                (tr("Bottom"), Align::Bottom),
            ] {
                if ui.add_enabled(n > 1, egui::Button::new(label)).clicked() {
                    self.align(a);
                    ui.close();
                }
            }
        });
        ui.menu_button(tr("Space Evenly"), |ui| {
            if ui.add_enabled(n > 2, egui::Button::new(tr("Left to Right"))).clicked() {
                self.space_evenly(true);
                ui.close();
            }
            if ui.add_enabled(n > 2, egui::Button::new(tr("Top to Bottom"))).clicked() {
                self.space_evenly(false);
                ui.close();
            }
        });
        ui.menu_button(tr("Make Same Size"), |ui| {
            ui.weak(tr("As the first selected object"));
            for (label, w, h) in [
                (tr("Width"), true, false),
                (tr("Height"), false, true),
                (tr("Width and Height"), true, true),
            ] {
                if ui.add_enabled(n > 1, egui::Button::new(label)).clicked() {
                    self.make_same_size(w, h);
                    ui.close();
                }
            }
        });
        ui.separator();
        let (mut sx, mut sy, mut rot) = self.arrange_values;
        ui.horizontal(|ui| {
            ui.label(tr("Rotate by"));
            ui.add(
                egui::DragValue::new(&mut rot)
                    .range(-360.0..=360.0)
                    .clamp_existing_to_range(false)
                    .suffix("°"),
            );
            if ui.add_enabled(n > 0, egui::Button::new(tr("Rotate"))).clicked() {
                self.rotate_selection(rot);
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Skew"));
            ui.add(
                egui::DragValue::new(&mut sx)
                    .range(-80.0..=80.0)
                    .clamp_existing_to_range(false)
                    .suffix("° X"),
            );
            ui.add(
                egui::DragValue::new(&mut sy)
                    .range(-80.0..=80.0)
                    .clamp_existing_to_range(false)
                    .suffix("° Y"),
            );
            if ui.add_enabled(n > 0, egui::Button::new(tr("Skew"))).clicked() {
                self.skew_selection(sx, sy);
            }
        });
        self.arrange_values = (sx, sy, rot);
        if ui
            .button(tr("Mirror about a line…"))
            .on_hover_text(Tool::MirrorLine.tooltip())
            .clicked()
        {
            self.set_tool(Tool::MirrorLine);
            ui.close();
        }
        if ui
            .button(tr("Transform by reference points…"))
            .on_hover_text(Tool::RefPoints.tooltip())
            .clicked()
        {
            self.set_tool(Tool::RefPoints);
            ui.close();
        }
        ui.separator();
        if ui.add_enabled(n > 1, egui::Button::new(tr("Group"))).clicked() {
            self.group_selection();
        }
        if ui.add_enabled(n > 0, egui::Button::new(tr("Ungroup"))).clicked() {
            self.ungroup_selection();
        }
        if ui.add_enabled(n > 0, egui::Button::new(tr("Lock"))).clicked() {
            self.lock_selection();
        }
        let any_locked = self.doc.design().objects().iter().any(|o| o.locked);
        if ui
            .add_enabled(any_locked, egui::Button::new(tr("Unlock All")))
            .clicked()
        {
            self.unlock_all();
        }
    }
}

/// Edit commands for the selected objects, in the object menu.
pub fn edit_items(ui: &mut egui::Ui, st: &mut AppState) {
    let crate::ribbon::SelInfo {
        fills,
        holes,
        lines,
        single_line,
        single_shape,
        ..
    } = crate::ribbon::SelInfo::of(st);
    ui.menu_button(tr("Edit Objects"), |ui| {
        if ui
            .button(tr("Break Apart"))
            .on_hover_text(tr("One object per separate part"))
            .clicked()
        {
            st.break_apart();
            ui.close();
        }
        if ui
            .add(egui::Button::new(tr("Join")).shortcut_text(ui.ctx().format_shortcut(&crate::join::JOIN)))
            .on_hover_text(tr("One object from the selected objects of one kind and color"))
            .clicked()
        {
            st.join_selection();
            ui.close();
        }
        if ui.button(tr("Knife…")).on_hover_text(Tool::Knife.tooltip()).clicked() {
            st.set_tool(Tool::Knife);
            ui.close();
        }
        ui.separator();
        let open = st.selected_count(crate::editing::has_open_part) > 0;
        if ui.add_enabled(open, egui::Button::new(tr("Close Shape"))).clicked() {
            st.close_shapes(false);
            ui.close();
        }
        if ui
            .add_enabled(open, egui::Button::new(tr("Close with Curve")))
            .clicked()
        {
            st.close_shapes(true);
            ui.close();
        }
        let reversible = st.selected_count(crate::editing::reversible) > 0;
        if ui
            .add_enabled(reversible, egui::Button::new(tr("Reverse Direction")))
            .clicked()
        {
            st.reverse_selection();
            ui.close();
        }
        let satins = st.selected_count(|k| matches!(k, emb_model::ObjectKind::Satin { .. })) > 0;
        if ui.add_enabled(satins, egui::Button::new(tr("Swap Sides"))).clicked() {
            st.swap_sides();
            ui.close();
        }
        if st.selected_count(crate::editing::stone_pattern) > 0 && ui.button(tr("To Single Stones")).clicked() {
            st.make_single_stones();
            ui.close();
        }
        ui.separator();
        if ui.add_enabled(fills == 1, egui::Button::new(tr("Add Hole…"))).clicked() {
            st.set_tool(Tool::AddHole);
            ui.close();
        }
        if ui.add_enabled(holes, egui::Button::new(tr("Remove Holes"))).clicked() {
            st.remove_holes();
            ui.close();
        }
        if ui
            .add_enabled(holes, egui::Button::new(tr("Fill Holes")))
            .on_hover_text(tr("A new fill object in each hole"))
            .clicked()
        {
            st.fill_holes();
            ui.close();
        }
        if ui
            .add_enabled(single_shape, egui::Button::new(tr("Stitch Angle…")))
            .clicked()
        {
            st.set_tool(Tool::StitchAngle);
            ui.close();
        }
        if ui
            .add_enabled(single_shape, egui::Button::new(tr("Direction Line…")))
            .on_hover_text(tr("Stitches across a line that you draw through the shape"))
            .clicked()
        {
            st.set_tool(Tool::DirectionLine);
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(single_line, egui::Button::new(tr("Continue Line")))
            .clicked()
        {
            st.continue_line();
            ui.close();
        }
        if ui
            .add_enabled(lines > 0, egui::Button::new(tr("Redwork")))
            .on_hover_text(tr(
                "Join the lines into one object; each line is sewn two times and it ends where it starts",
            ))
            .clicked()
        {
            st.make_network(RunPass::Redwork);
            ui.close();
        }
        if ui
            .add_enabled(lines > 0, egui::Button::new(tr("Branching")))
            .on_hover_text(tr(
                "Join the lines into one object, sewn with the fewest lines sewn two times",
            ))
            .clicked()
        {
            st.make_network(RunPass::Branch);
            ui.close();
        }
        if ui
            .button(tr("Closest Join"))
            .on_hover_text(tr(
                "Each object ends at the point nearest to the next one, which starts there",
            ))
            .clicked()
        {
            st.closest_join();
            ui.close();
        }
    });
    ui.menu_button(tr("Arrange"), |ui| st.arrange_menu(ui));
}
