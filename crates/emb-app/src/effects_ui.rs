//! The Effects sections of Object Properties (P5 Milestone 6): hand stitch,
//! cording, corners, auto jump, fractional and auto spacing, accordion
//! spacing (preset and custom profiles), jagged edge, trapunto and pull
//! compensation for each side and as a percent.

use std::ops::RangeInclusive;

use eframe::egui;
use emb_model::{
    Accordion, Corner, FillParams, LineBorder, MergeKey, ObjectId, ObjectKind, RunningParams, SatinParams,
};

use crate::app::AppState;
use crate::i18n::tr;
use crate::stitch_types::column_params;
use crate::words;

/// A slider for an effect. A drag is one undo step.
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    key: &str,
    label: &str,
    help: &str,
    value: f64,
    range: RangeInclusive<f64>,
    suffix: &str,
    set: impl FnOnce(&mut ObjectKind, f64),
) {
    let mut v = value;
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(help);
        let resp = ui.add(
            egui::Slider::new(&mut v, range)
                .clamping(egui::SliderClamping::Edits)
                .suffix(suffix)
                .fixed_decimals(2),
        );
        if resp.changed() {
            st.doc.edit_merged(
                format!("Change {}", label.to_lowercase()),
                MergeKey::new(format!("{}:fx:{key}", id.0)),
                |d| {
                    if let Some(o) = d.object_mut(id) {
                        set(&mut o.kind, v);
                    }
                },
            );
            if !resp.dragged() {
                st.doc.end_merge();
            }
        }
        if resp.drag_stopped() {
            st.doc.end_merge();
        }
    });
}

/// A checkbox for an effect: one undo step.
fn check(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    label: &str,
    help: &str,
    value: bool,
    set: impl FnOnce(&mut ObjectKind, bool),
) {
    let mut v = value;
    if ui.checkbox(&mut v, label).on_hover_text(help).changed() {
        st.doc.edit(format!("{} {}", label, if v { "on" } else { "off" }), |d| {
            if let Some(o) = d.object_mut(id) {
                set(&mut o.kind, v);
            }
        });
    }
}

fn fill_params(k: &mut ObjectKind) -> Option<&mut FillParams> {
    match k {
        ObjectKind::Fill { params, .. } => Some(params),
        _ => None,
    }
}

fn run_params(k: &mut ObjectKind) -> Option<&mut RunningParams> {
    match k {
        ObjectKind::Running { params, .. } => Some(params),
        _ => None,
    }
}

/// The accordion profile of an object, from its column or fill settings.
fn accordion_of(k: &mut ObjectKind) -> Option<&mut Option<Accordion>> {
    if let ObjectKind::Fill { params, .. } = k {
        return Some(&mut params.accordion);
    }
    column_params(k).map(|c| &mut c.accordion)
}

/// The accordion spacing: off, a preset, or a custom profile with points.
fn accordion_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, current: &Option<Accordion>) {
    let presets = Accordion::presets();
    let name = match current {
        None => "Off".to_string(),
        Some(a) => presets
            .iter()
            .find(|(_, p)| p == a)
            .map_or("Custom".to_string(), |(n, _)| n.to_string()),
    };
    let mut pick: Option<Option<Accordion>> = None;
    ui.horizontal(|ui| {
        ui.label(tr("Accordion spacing"))
            .on_hover_text(tr(words::ACCORDION_HELP));
        egui::ComboBox::from_id_salt("accordion")
            .selected_text(&name)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current.is_none(), tr("Off")).clicked() {
                    pick = Some(None);
                }
                for (n, p) in &presets {
                    if ui.selectable_label(name == *n, *n).clicked() {
                        pick = Some(Some(p.clone()));
                    }
                }
                if ui.selectable_label(name == "Custom", tr("Custom")).clicked() && name != "Custom" {
                    // Start from the current profile, or a plain one.
                    let start = current.clone().unwrap_or(Accordion {
                        points: vec![(0.0, 1.0), (1.0, 1.0)],
                    });
                    pick = Some(Some(Accordion {
                        // A point in the middle makes it custom.
                        points: {
                            let mut pts = start.points.clone();
                            let mid = start.factor(0.5);
                            if !pts.iter().any(|p| p.0 == 0.5) {
                                pts.push((0.5, mid + 0.5));
                                pts.sort_by(|a, b| a.0.total_cmp(&b.0));
                            }
                            pts
                        },
                    }));
                }
            });
    });
    if let Some(a) = pick {
        st.doc.edit("Change accordion spacing", |d| {
            if let Some(slot) = d.object_mut(id).and_then(|o| accordion_of(&mut o.kind)) {
                *slot = a;
            }
        });
    }
    // The points of a custom profile.
    if name == "Custom"
        && let Some(a) = current
    {
        let mut points = a.points.clone();
        let mut changed = false;
        let mut stop = false;
        let mut remove = None;
        for (i, (t, f)) in points.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}.", i + 1));
                let r1 = ui.add(
                    egui::DragValue::new(t)
                        .range(0.0..=1.0)
                        .clamp_existing_to_range(false)
                        .speed(0.01)
                        .prefix("at "),
                );
                let r2 = ui.add(
                    egui::DragValue::new(f)
                        .range(1.0..=10.0)
                        .clamp_existing_to_range(false)
                        .speed(0.05)
                        .prefix("× "),
                );
                changed |= r1.changed() || r2.changed();
                stop |= r1.drag_stopped() || r2.drag_stopped() || r1.lost_focus() || r2.lost_focus();
                if a.points.len() > 2 && ui.small_button(tr("Remove")).clicked() {
                    remove = Some(i);
                }
            });
        }
        let add = ui
            .small_button(tr("Add point"))
            .on_hover_text(tr("A new point halfway along the longest gap."))
            .clicked();
        if let Some(i) = remove {
            points.remove(i);
            changed = true;
            stop = true;
        }
        if add {
            let i = (1..points.len())
                .max_by(|&i, &j| (points[i].0 - points[i - 1].0).total_cmp(&(points[j].0 - points[j - 1].0)))
                .unwrap_or(1);
            let t = (points[i - 1].0 + points[i].0) / 2.0;
            points.insert(i, (t, a.factor(t)));
            changed = true;
            stop = true;
        }
        if changed {
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            st.doc.edit_merged(
                "Change accordion profile",
                MergeKey::new(format!("{}:fx:acc", id.0)),
                |d| {
                    if let Some(slot) = d.object_mut(id).and_then(|o| accordion_of(&mut o.kind)) {
                        *slot = Some(Accordion { points });
                    }
                },
            );
        }
        if stop {
            st.doc.end_merge();
        }
    }
}

fn hand_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, value: f64) {
    slider(
        ui,
        st,
        id,
        "hand",
        tr("Hand stitch"),
        tr(words::HAND_HELP),
        value,
        0.0..=1.5,
        " mm",
        |k, v| match k {
            ObjectKind::Running { params, .. } => params.hand_mm = v,
            ObjectKind::Fill { params, .. } => params.hand_mm = v,
            k => {
                if let Some(c) = column_params(k) {
                    c.hand_mm = v;
                }
            }
        },
    );
}

/// Effects of a plain running line: hand stitch and cording.
pub fn running_effects(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, p: &RunningParams) {
    egui::CollapsingHeader::new(tr("Effects"))
        .id_salt("fx_run")
        .show(ui, |ui| {
            hand_ui(ui, st, id, p.hand_mm);
            check(ui, st, id, tr("Cording"), tr(words::CORDING_HELP), p.cording, |k, v| {
                if let Some(r) = run_params(k) {
                    r.cording = v;
                }
            });
        });
}

/// The corner type of a line with a width.
pub fn corner_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, b: &LineBorder) {
    let mut c = b.corner;
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("Corners")).on_hover_text(tr(words::CORNER_HELP));
        for (v, name) in [
            (Corner::Mitre, tr("Mitre")),
            (Corner::Cap, tr("Cap")),
            (Corner::Lap, tr("Lap")),
        ] {
            changed |= ui.selectable_value(&mut c, v, name).changed();
        }
    });
    if changed {
        st.doc.edit("Change corners", |d| {
            if let Some(r) = d.object_mut(id).and_then(|o| run_params(&mut o.kind))
                && let Some(b) = r.border.as_mut()
            {
                b.corner = c;
            }
        });
    }
}

/// Effects of a column (satin column or line with a width).
pub fn column_effects(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, p: &SatinParams) {
    egui::CollapsingHeader::new(tr("Effects"))
        .id_salt("fx_col")
        .show(ui, |ui| {
            let set = |f: fn(&mut SatinParams, f64)| {
                move |k: &mut ObjectKind, v: f64| {
                    if let Some(c) = column_params(k) {
                        f(c, v);
                    }
                }
            };
            check(
                ui,
                st,
                id,
                tr("Auto jump"),
                tr(words::AUTO_JUMP_HELP),
                p.auto_jump,
                |k, v| {
                    if let Some(c) = column_params(k) {
                        c.auto_jump = v;
                    }
                },
            );
            slider(
                ui,
                st,
                id,
                "frac",
                tr("Spacing measured at"),
                tr(words::FRACTIONAL_HELP),
                p.spacing_at,
                0.0..=1.0,
                " of the width",
                set(|c, v| c.spacing_at = v),
            );
            check(
                ui,
                st,
                id,
                tr("Auto spacing"),
                tr(words::AUTO_SPACING_HELP),
                p.auto_spacing,
                |k, v| {
                    if let Some(c) = column_params(k) {
                        c.auto_spacing = v;
                    }
                },
            );
            accordion_ui(ui, st, id, &p.accordion);
            slider(
                ui,
                st,
                id,
                "jagged",
                tr("Jagged edge"),
                tr(words::JAGGED_HELP),
                p.jagged_mm,
                0.0..=5.0,
                " mm",
                set(|c, v| c.jagged_mm = v),
            );
            hand_ui(ui, st, id, p.hand_mm);
            slider(
                ui,
                st,
                id,
                "pct",
                tr("Pull compensation"),
                tr(words::PULL_PERCENT_HELP),
                p.pull_percent,
                0.0..=30.0,
                " % of the width",
                set(|c, v| c.pull_percent = v),
            );
            check(
                ui,
                st,
                id,
                tr("Other pull for rail B"),
                tr(words::PULL_SIDES_HELP),
                p.pull_b_mm.is_some(),
                |k, v| {
                    if let Some(c) = column_params(k) {
                        c.pull_b_mm = v.then_some(c.pull_compensation_mm);
                    }
                },
            );
            if let Some(b) = p.pull_b_mm {
                slider(
                    ui,
                    st,
                    id,
                    "pullb",
                    tr("Rail B pull"),
                    tr(words::PULL_SIDES_HELP),
                    b,
                    0.0..=1.0,
                    " mm",
                    set(|c, v| c.pull_b_mm = Some(v)),
                );
            }
        });
}

/// Effects of a fill.
pub fn fill_effects(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, p: &FillParams) {
    egui::CollapsingHeader::new(tr("Effects"))
        .id_salt("fx_fill")
        .show(ui, |ui| {
            check(
                ui,
                st,
                id,
                tr("Trapunto"),
                tr(words::TRAPUNTO_HELP),
                p.trapunto,
                |k, v| {
                    if let Some(f) = fill_params(k) {
                        f.trapunto = v;
                    }
                },
            );
            accordion_ui(ui, st, id, &p.accordion);
            slider(
                ui,
                st,
                id,
                "jagged",
                tr("Jagged edge"),
                tr(words::JAGGED_HELP),
                p.jagged_mm,
                0.0..=5.0,
                " mm",
                |k, v| {
                    if let Some(f) = fill_params(k) {
                        f.jagged_mm = v;
                    }
                },
            );
            hand_ui(ui, st, id, p.hand_mm);
        });
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{FillPattern, SatinGeometry};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn effects_sections_draw_open_with_no_edit() {
        let mut h = Harness::empty();
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            [
                d.add_object(
                    t,
                    ObjectKind::Running {
                        path: Path::new().move_to(Point::ZERO).line_to(Point::new(30.0, 0.0)),
                        params: RunningParams {
                            hand_mm: 0.3,
                            ..RunningParams::default()
                        },
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Satin {
                        geometry: SatinGeometry::from_centerline(
                            &Path::new()
                                .move_to(Point::new(0.0, 10.0))
                                .line_to(Point::new(30.0, 10.0)),
                            4.0,
                        ),
                        params: SatinParams {
                            pull_b_mm: Some(0.3),
                            accordion: Some(Accordion {
                                points: vec![(0.0, 1.0), (0.4, 3.0), (1.0, 1.0)],
                            }),
                            ..SatinParams::default()
                        },
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Fill {
                        outer: Path::rect(emb_geom::BBox::new(Point::new(0.0, 20.0), Point::new(30.0, 40.0))),
                        holes: Vec::new(),
                        params: FillParams {
                            trapunto: true,
                            accordion: Accordion::presets().first().map(|(_, a)| a.clone()),
                            pattern: FillPattern::Tatami,
                            ..FillParams::default()
                        },
                    },
                ),
            ]
        });
        let before = h.st.doc.undo_len();
        for id in ids {
            h.st.selection.set(id);
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
                    ..Default::default()
                };
                let st = &mut h.st;
                let mut out = h.ctx.run_ui(input, |ui| {
                    // Open every Effects section.
                    for salt in ["fx_run", "fx_col", "fx_fill"] {
                        let cid = ui.make_persistent_id(salt);
                        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), cid, true)
                            .store(ui.ctx());
                    }
                    egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
                });
                out.textures_delta.clear();
            }
        }
        assert_eq!(h.st.doc.undo_len(), before);
    }
}
