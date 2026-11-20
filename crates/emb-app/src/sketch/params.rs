//! The parameters of a sketch (ADR 0078): the Parameters window, and the
//! texts of values and of the errors of expressions.

use super::*;

impl AppState {
    /// A change of the parameters or expressions of the open sketch: one
    /// undo step, and no step if it is refused.
    pub(super) fn edit_params<R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Sketch) -> Result<R, emb_sketch::ExprError>,
    ) -> Result<R, emb_sketch::ExprError> {
        let mut err = None;
        let r = self.try_edit_sketch(label, |sk| {
            f(sk).map_err(|e| {
                err = Some(e);
                EditError::Missing
            })
        });
        match r {
            Some(Ok(v)) => Ok(v),
            _ => Err(err.unwrap_or(emb_sketch::ExprError::BadValue)),
        }
    }

    /// Opens or closes the Parameters window of the open sketch.
    pub fn toggle_parameters(&mut self) {
        if let Some(e) = &mut self.sketch {
            e.params.open = !e.params.open;
        }
    }

    pub(super) fn param_action(&mut self, a: ParamAction) {
        let units = self.settings.units;
        let r = match a {
            ParamAction::Set(name, text) => self.edit_params("Parameter", |sk| {
                first_units(sk, units);
                sk.set_parameter(&name, &text)
            }),
            ParamAction::Rename(old, new) => self.edit_params("Rename Parameter", |sk| sk.rename_parameter(&old, &new)),
            ParamAction::Remove(name) => self.edit_params("Remove Parameter", |sk| sk.remove_parameter(&name)),
            ParamAction::Units(u) => self.edit_params("Parameter Units", |sk| {
                sk.units = u;
                sk.apply_parameters()
            }),
        };
        if let Some(e) = &mut self.sketch {
            e.params.error = r.err().map(|e| expr_message(&e));
            e.params.names.clear();
            e.params.exprs.clear();
        }
    }
}

/// The units of plain numbers in expressions: the units of the Options
/// when the sketch gets its first parameter or expression.
pub(super) fn first_units(sk: &mut Sketch, units: crate::workspace::Units) {
    if sk.parameters.is_empty() && sk.exprs.is_empty() {
        sk.units = match units {
            crate::workspace::Units::Mm => emb_sketch::Units::Mm,
            crate::workspace::Units::Inch => emb_sketch::Units::Inch,
        };
    }
}

/// The message of an error of an expression, translated.
pub fn expr_message(e: &emb_sketch::ExprError) -> String {
    let (t, v) = e.parts();
    let text = match v {
        Some(v) => fill(t, &[&v]),
        None => tr(t).to_string(),
    };
    let mut c = text.chars();
    match c.next() {
        Some(f) => format!("{}{}.", f.to_uppercase(), c.as_str()),
        None => text,
    }
}

/// The text of a value box: the expression of the dimension, else its
/// value.
pub(super) fn box_text(units: crate::workspace::Units, s: &Sketch, id: ConstraintId, c: &Constraint) -> String {
    match s.dimension_expr(id) {
        Some(e) => e.to_string(),
        None => value_text(units, c),
    }
}

/// A value of a parameter as text: a length in `units`, else a number.
fn quantity_text(q: emb_sketch::expr::Quantity, units: crate::workspace::Units, scale: f64) -> String {
    match q.length {
        1 => units.fmt(q.value * scale),
        0 => {
            let t = format!("{:.4}", q.value);
            t.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        n => format!("{:.2} mm^{n}", q.value),
    }
}

/// The Parameters window of the open sketch (ADR 0078): the named values
/// and their expressions.
pub(super) fn parameters_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.sketch.as_ref().is_some_and(|e| e.params.open) {
        return;
    }
    let Some(s) = st.sketch_obj().map(|o| o.sketch().clone()) else {
        return;
    };
    let units = st.settings.units;
    let values = s.parameter_values().unwrap_or_default();
    let mut open = true;
    let mut action = None;
    let id = egui::Id::new("Sketch Parameters");
    let rect = st.canvas_rect;
    let Some(e) = st.sketch.as_mut() else { return };
    let ui_state = &mut e.params;
    crate::dock::docked(egui::Window::new(tr("Parameters")).id(id), ctx, id, rect)
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.weak(tr(
                "Named values for the dimensions. Type a name or an expression (for example width / 20) in the value box of a dimension.",
            ));
            egui::Grid::new("sketch_params").num_columns(4).striped(true).show(ui, |ui| {
                ui.strong(tr("Name"));
                ui.strong(tr("Expression"));
                ui.strong(tr("Value"));
                ui.end_row();
                for p in &s.parameters {
                    let name = ui_state.names.entry(p.name.clone()).or_insert_with(|| p.name.clone());
                    let r = ui.add(egui::TextEdit::singleline(name).desired_width(90.0));
                    if r.lost_focus() && *name != p.name {
                        action = Some(ParamAction::Rename(p.name.clone(), name.clone()));
                    }
                    let text = ui_state.exprs.entry(p.name.clone()).or_insert_with(|| p.expr.clone());
                    let r = ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                    if r.lost_focus() && *text != p.expr {
                        action = Some(ParamAction::Set(p.name.clone(), text.clone()));
                    }
                    ui.label(
                        values
                            .get(&p.name)
                            .map_or_else(|| "—".to_string(), |q| quantity_text(*q, units, s.expr_scale())),
                    );
                    if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the parameter")).clicked() {
                        action = Some(ParamAction::Remove(p.name.clone()));
                    }
                    ui.end_row();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut ui_state.new_name)
                        .hint_text(tr("width"))
                        .desired_width(90.0),
                );
                let r = ui.add(
                    egui::TextEdit::singleline(&mut ui_state.new_expr)
                        .hint_text(tr("80 mm"))
                        .desired_width(140.0),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button(tr("Add")).clicked() || enter) && !ui_state.new_name.trim().is_empty() {
                    action = Some(ParamAction::Set(
                        std::mem::take(&mut ui_state.new_name),
                        std::mem::take(&mut ui_state.new_expr),
                    ));
                }
                ui.end_row();
            });
            ui.horizontal(|ui| {
                ui.label(tr("Numbers with no unit are in"));
                for (u, name) in [(emb_sketch::Units::Mm, "mm"), (emb_sketch::Units::Inch, "in")] {
                    if ui.selectable_label(s.units == u, name).clicked() && s.units != u {
                        action = Some(ParamAction::Units(u));
                    }
                }
            });
            if (s.expr_scale() - 1.0).abs() > 1e-9 {
                ui.weak(fill(
                    "The sketch was scaled: the lengths of the expressions are × {}.",
                    &[&format!("{:.3}", s.expr_scale())],
                ));
            }
            if let Some(err) = &ui_state.error {
                ui.colored_label(ui.visuals().warn_fg_color, err);
            }
        });
    if !open && let Some(e) = &mut st.sketch {
        e.params.open = false;
    }
    if let Some(a) = action {
        st.param_action(a);
    }
}

/// The value of a dimension as text for the value box: an angle in
/// degrees, a size in the units of the Options.
pub(super) fn value_text(units: crate::workspace::Units, c: &Constraint) -> String {
    let v = c.value().unwrap_or(0.0);
    match c {
        Constraint::Angle { .. } => format!("{v:.2}"),
        _ => {
            let x = units.of_mm(v.abs());
            if units == crate::workspace::Units::Mm {
                format!("{x:.2}")
            } else {
                format!("{x:.3}")
            }
        }
    }
}
