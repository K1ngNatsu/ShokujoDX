//! Object Properties panel.

use std::ops::RangeInclusive;

use eframe::egui::{self, Sense, Vec2};
use emb_model::{
    ColumnPattern, CrossStyle, FillParams, FillPattern, MergeKey, ObjectId, ObjectKind, RunStyle, RunningParams,
    SatinParams, Underlay,
};

use crate::i18n::tr;
use crate::stitch_types::column_params;

use crate::app::AppState;
use crate::canvas::color32;
use crate::words;

/// A slider for one parameter. A drag is one undo step; see `Document::edit_merged`.
#[allow(clippy::too_many_arguments)]
fn param_slider(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    key: &str,
    label: &str,
    value: f64,
    range: RangeInclusive<f64>,
    word: (fn(f64) -> &'static str, &str),
    set: impl FnOnce(&mut ObjectKind, f64),
) {
    let mut v = value;
    let (word, help) = word;
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(help);
        let resp = ui.add(
            egui::Slider::new(&mut v, range)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" mm")
                .fixed_decimals(2),
        );
        ui.weak(format!("({})", word(v)));
        if resp.changed() {
            st.doc.edit_merged(
                format!("Change {}", label.to_lowercase()),
                MergeKey::new(format!("{}:{key}", id.0)),
                |d| {
                    if let Some(o) = d.object_mut(id) {
                        set(&mut o.kind, v);
                    }
                },
            );
            if !resp.dragged() {
                // A click or a key press is one step of its own.
                st.doc.end_merge();
            }
        }
        if resp.drag_stopped() {
            st.doc.end_merge();
        }
    });
}

/// Thread picker that applies to all `ids` as one undo step.
fn thread_picker(ui: &mut egui::Ui, st: &mut AppState, ids: &[ObjectId]) {
    let design = st.doc.design();
    let threads = design.threads.clone();
    let used: Vec<_> = ids
        .iter()
        .filter_map(|id| design.object(*id).map(|o| o.thread))
        .collect();
    let current = match used.as_slice() {
        [first, rest @ ..] if rest.iter().all(|t| t == first) => threads.iter().find(|t| t.id == *first),
        _ => None,
    };
    let mut new_thread = None;
    ui.horizontal(|ui| {
        ui.label(tr("Thread"));
        if let Some(t) = current {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
            ui.painter().rect_filled(r, 2.0, color32(t.color));
        }
        egui::ComboBox::from_id_salt("thread")
            .selected_text(current.map(|t| t.name.clone()).unwrap_or_else(|| "(mixed)".into()))
            .show_ui(ui, |ui| {
                for t in &threads {
                    if ui
                        .selectable_label(Some(t.id) == current.map(|c| c.id), &t.name)
                        .clicked()
                    {
                        new_thread = Some(t.id);
                    }
                }
            });
    });
    if let Some(t) = new_thread {
        let ids = ids.to_vec();
        st.doc.edit("Change thread", |d| {
            for id in ids {
                if let Some(o) = d.object_mut(id) {
                    o.thread = t;
                }
            }
        });
    }
}

pub fn properties(ui: &mut egui::Ui, st: &mut AppState) {
    if st.sketch.is_some() {
        crate::sketch::edit_panel(ui, st);
        return;
    }
    object_properties(ui, st);
    if st.selection.len() == 1 {
        ui.separator();
        crate::functions::section(ui, st);
    }
    ui.separator();
    crate::backdrop::panel(ui, st);
}

/// For a selection that is one group: a button for each of its objects,
/// which selects that object alone (as a double-click on the canvas).
fn group_members(ui: &mut egui::Ui, st: &mut AppState, ids: &[emb_model::ObjectId]) {
    let d = st.doc.design();
    let groups: Vec<Option<u32>> = ids.iter().map(|id| d.object(*id).and_then(|o| o.group)).collect();
    let Some(Some(g)) = groups.first().copied() else { return };
    if groups.iter().any(|x| *x != Some(g)) {
        return;
    }
    ui.strong(tr("Objects of the group"));
    let rows: Vec<(emb_model::ObjectId, String)> = ids
        .iter()
        .filter_map(|id| {
            let o = d.object(*id)?;
            let n = d.index_of(*id).map_or(0, |i| i + 1);
            let thread = d.thread(o.thread).map(|t| t.name.clone()).unwrap_or_default();
            Some((
                *id,
                format!(
                    "#{n} {} ({thread})",
                    o.name.clone().unwrap_or_else(|| o.type_name().to_string())
                ),
            ))
        })
        .collect();
    let mut pick = None;
    for (id, text) in &rows {
        if ui
            .button(text)
            .on_hover_text(tr("Select this object alone to edit it."))
            .clicked()
        {
            pick = Some(*id);
        }
    }
    ui.weak(tr("Or double-click an object of the group on the canvas."));
    if let Some(id) = pick {
        st.selection.enter(id, g);
    }
    ui.separator();
}

fn object_properties(ui: &mut egui::Ui, st: &mut AppState) {
    if st.selection.len() > 1 {
        let ids = st.selection.in_sew_order(st.doc.design());
        ui.heading(crate::i18n::fill("{} objects", &[&(ids.len())]));
        thread_picker(ui, st, &ids);
        ui.separator();
        transform_section(ui, st, &ids);
        ui.separator();
        crate::functions::section(ui, st);
        ui.separator();
        group_members(ui, st, &ids);
        ui.weak(tr("Select one object to edit its stitch settings."));
        return;
    }
    let Some(id) = st.selection.single() else {
        ui.weak(tr("Select an object to see its properties."));
        return;
    };
    let Some(obj) = st.doc.design().object(id).cloned() else {
        ui.weak(tr("Select an object to see its properties."));
        return;
    };

    ui.heading(obj.type_name());
    thread_picker(ui, st, &[id]);
    ui.separator();
    transform_section(ui, st, &[id]);
    ui.separator();

    match &obj.kind {
        ObjectKind::Lettering(l) => crate::lettering::properties(ui, st, id, l),
        ObjectKind::Running { params, .. } => {
            outline_type_ui(ui, st, id, params);
            if let Some(r) = &params.ripple {
                ripple_section(ui, st, id, r, params.stitch_length_mm);
            } else if let Some(r) = &params.motif {
                motif_run_section(ui, st, id, r, params.stitch_length_mm);
            } else {
                match &params.border {
                    None => {
                        param_slider(
                            ui,
                            st,
                            id,
                            "len",
                            tr("Stitch length"),
                            params.stitch_length_mm,
                            1.0..=7.0,
                            (words::length, tr(words::LENGTH_HELP)),
                            |k, v| {
                                if let ObjectKind::Running { params, .. } = k {
                                    params.stitch_length_mm = v;
                                }
                            },
                        );
                        let mut p = params.clone();
                        let changed = ui.horizontal(|ui| crate::draw::run_type_combo(ui, &mut p)).inner;
                        if changed {
                            st.doc.edit("Change run type", |d| {
                                if let Some(ObjectKind::Running { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                                {
                                    *params = p;
                                }
                            });
                        }
                        crate::effects_ui::running_effects(ui, st, id, params);
                    }
                    Some(border) => {
                        param_slider(
                            ui,
                            st,
                            id,
                            "width",
                            tr("Width"),
                            border.width_mm,
                            0.5..=12.0,
                            (words::width, tr(words::WIDTH_HELP)),
                            |k, v| {
                                if let ObjectKind::Running { params, .. } = k
                                    && let Some(b) = params.border.as_mut()
                                {
                                    b.width_mm = v;
                                }
                            },
                        );
                        crate::effects_ui::corner_ui(ui, st, id, border);
                        column_section(ui, st, id, &border.column, false);
                    }
                }
            }
        }
        ObjectKind::Satin { params, .. } => column_section(ui, st, id, params, true),
        ObjectKind::Fill { params, .. } => fill_section(ui, st, id, params),
        ObjectKind::Applique { pieces, params } => crate::applique::properties(ui, st, id, pieces, params),
        ObjectKind::Stitches { path, trims } => {
            let subs = emb_stitch::stitch_subpaths(path);
            let n: usize = subs.iter().map(Vec::len).sum();
            ui.label(crate::i18n::fill(
                "{} needle points in {} parts ({} trims).",
                &[&n, &(subs.len()), &(trims.len())],
            ));
            ui.weak(
                tr("Stitches from a machine file are sewn as they are. Reshape moves, adds and deletes single needle points. Scaling changes the stitch length and density."),
            );
        }
        ObjectKind::Vector { path, style } => crate::vectors::properties(ui, st, id, path, style),
        ObjectKind::CrossStitch(x) => crate::cross::properties(ui, st, id, x),
        ObjectKind::Bling(b) => crate::bling::properties(ui, st, id, b),
        ObjectKind::Sketch(s) => crate::sketch::properties(ui, st, id, s),
    }
    crate::sketch::made_from_sketch_ui(ui, st, id);

    ui.separator();
    ui.weak(crate::i18n::fill(
        "Object id {}, revision {}",
        &[&obj.id.0, &obj.revision],
    ))
    .on_hover_text(tr(
        "The stitch cache key is (id, revision). Undo restores the old revision.",
    ));
}

fn underlay_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, current: &Underlay) {
    let mut value = current.clone();
    if underlay_combo(ui, "props_underlay", &mut value) {
        st.doc.edit("Change underlay", |d| {
            let Some(o) = d.object_mut(id) else { return };
            if let ObjectKind::Fill { params, .. } = &mut o.kind {
                params.underlay = value;
            } else if let Some(c) = column_params(&mut o.kind) {
                c.underlay = value;
            }
        });
    }
}

/// The outline type of a line: a plain run, or a column type with a width.
fn outline_type_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, params: &RunningParams) {
    let ripple = params.ripple.is_some();
    let motif = params.motif.is_some() && !ripple;
    let current = params
        .border
        .as_ref()
        .map(|b| b.column.pattern)
        .filter(|_| !motif && !ripple);
    let mut pick: Option<Option<ColumnPattern>> = None;
    let mut motif_run = false;
    let mut make_ripple = false;
    ui.horizontal(|ui| {
        ui.label(tr("Outline Type")).on_hover_text(tr(words::OUTLINE_HELP));
        let shown = if ripple {
            "Ripple"
        } else if motif {
            "Motif run"
        } else {
            current.map_or("Line", |p| p.name())
        };
        egui::ComboBox::from_id_salt("outline_type")
            .selected_text(tr(shown))
            .show_ui(ui, |ui| {
                let line = current.is_none() && !motif && !ripple;
                if ui.selectable_label(line, tr("Line")).clicked() && !line {
                    pick = Some(None);
                }
                if ui
                    .selectable_label(ripple, tr("Ripple"))
                    .on_hover_text(tr(words::RIPPLE_HELP))
                    .clicked()
                    && !ripple
                {
                    make_ripple = true;
                }
                if ui
                    .selectable_label(motif, tr("Motif run"))
                    .on_hover_text(tr("A motif repeated along the line."))
                    .clicked()
                    && !motif
                {
                    motif_run = true;
                }
                for p in ColumnPattern::ALL {
                    let on = current.is_some_and(|c| c.same_type(&p));
                    if ui
                        .selectable_label(on, tr(p.name()))
                        .on_hover_text(words::column_help(&p))
                        .clicked()
                        && !on
                    {
                        pick = Some(Some(p));
                    }
                }
            });
    });
    if let Some(choice) = pick {
        crate::stitch_types::set_outline(st, &[id], choice);
    }
    if motif_run {
        st.apply_motif(&[id], &emb_model::Motif::default());
    }
    if make_ripple {
        crate::stitch_types::set_ripple(st, &[id], Some(emb_model::Ripple::default()));
    }
}

/// Changes the ripple of the line `id`; a drag is one undo step.
fn edit_ripple(st: &mut AppState, id: ObjectId, key: &str, label: &str, f: &dyn Fn(&mut emb_model::Ripple)) {
    st.doc.edit_merged(
        label.to_string(),
        MergeKey::new(format!("{}:ripple:{key}", id.0)),
        |d| {
            if let Some(ObjectKind::Running { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                && let Some(r) = params.ripple.as_mut()
            {
                f(r);
            }
        },
    );
}

/// The settings of a ripple stitch.
fn ripple_section(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, r: &emb_model::Ripple, len: f64) {
    let mut lines = r.lines;
    let mut end = r.end_percent;
    ui.horizontal(|ui| {
        ui.label(tr("Lines"))
            .on_hover_text(tr("The number of lines, the line itself included."));
        let resp = ui.add(egui::Slider::new(&mut lines, 1..=100).clamping(egui::SliderClamping::Edits));
        if resp.changed() {
            edit_ripple(st, id, "lines", "Change ripple lines", &|r| r.lines = lines);
            if !resp.dragged() {
                st.doc.end_merge();
            }
        }
        if resp.drag_stopped() {
            st.doc.end_merge();
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Size of the last line")).on_hover_text(tr(
            "0 %: the lines go to the point. 100 %: all lines have the size of the line.",
        ));
        let resp = ui.add(
            egui::Slider::new(&mut end, 0.0..=200.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" %")
                .fixed_decimals(0),
        );
        if resp.changed() {
            edit_ripple(st, id, "end", "Change ripple size", &|r| r.end_percent = end);
            if !resp.dragged() {
                st.doc.end_merge();
            }
        }
        if resp.drag_stopped() {
            st.doc.end_merge();
        }
    });
    // The point, as an offset from the middle of the line.
    if let Some(middle) = st
        .doc
        .design()
        .object(id)
        .and_then(|o| o.control_bbox())
        .map(|b| b.center())
    {
        let off = r.target.map_or(emb_geom::Point::ZERO, |c| c - middle);
        let (mut x, mut y) = (off.x, off.y);
        let (mut changed, mut stop, mut middle_again) = (false, false, false);
        ui.horizontal(|ui| {
            ui.label(tr("Point")).on_hover_text(tr(
                "Where the lines get smaller to, measured from the middle of the line (right, down).",
            ));
            for v in [&mut x, &mut y] {
                let resp = ui.add(egui::DragValue::new(v).speed(0.2).suffix(" mm").fixed_decimals(1));
                changed |= resp.changed();
                stop |= resp.drag_stopped() || resp.lost_focus();
            }
            middle_again = r.target.is_some() && ui.small_button(tr("Middle")).clicked();
        });
        if middle_again {
            st.doc.edit("Ripple point", |d| {
                if let Some(ObjectKind::Running { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                    && let Some(r) = params.ripple.as_mut()
                {
                    r.target = None;
                }
            });
        } else if changed {
            let c = middle + emb_geom::Point::new(x, y);
            edit_ripple(st, id, "target", "Ripple point", &|r| r.target = Some(c));
        }
        if stop {
            st.doc.end_merge();
        }
    }
    let mut remove = false;
    ui.horizontal(|ui| {
        if r.guide.is_empty() {
            ui.weak(tr("No guide line.")).on_hover_text(tr(
                "To add one, select this line and a guide line, then Edit > Stitch Types > Ripple Guide.",
            ));
        } else {
            ui.label(crate::i18n::fill("Guide line: {} points", &[&(r.guide.len())]));
            remove = ui.button(tr("Remove guide")).clicked();
        }
    });
    if remove {
        st.doc.edit("Remove ripple guide", |d| {
            if let Some(ObjectKind::Running { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                && let Some(r) = params.ripple.as_mut()
            {
                r.guide.clear();
            }
        });
    }
    param_slider(
        ui,
        st,
        id,
        "len",
        tr("Stitch length"),
        len,
        1.0..=7.0,
        (words::length, tr(words::LENGTH_HELP)),
        |k, v| {
            if let ObjectKind::Running { params, .. } = k {
                params.stitch_length_mm = v;
            }
        },
    );
}

/// The motif of an object, and a button to the Motifs window.
fn motif_row(ui: &mut egui::Ui, st: &mut AppState, m: &emb_model::Motif) {
    ui.horizontal(|ui| {
        ui.label(tr("Motif")).on_hover_text(tr(words::MOTIF_HELP));
        crate::motifs::preview(ui, m, Vec2::new(60.0, 22.0), ui.visuals().text_color());
        ui.label(&m.name);
        if ui.button(tr("Motifs…")).on_hover_text(tr(words::MOTIF_HELP)).clicked() {
            st.motifs_window.open = true;
        }
    });
}

/// The settings of a motif run.
fn motif_run_section(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, r: &emb_model::MotifRun, len: f64) {
    motif_row(ui, st, &r.motif);
    let set = |k: &mut ObjectKind, f: &dyn Fn(&mut emb_model::MotifRun)| {
        if let ObjectKind::Running { params, .. } = k
            && let Some(r) = params.motif.as_mut()
        {
            f(r);
        }
    };
    param_slider(
        ui,
        st,
        id,
        "motif_len",
        tr("Motif length"),
        r.length_mm,
        1.0..=30.0,
        (words::length, tr(words::LENGTH_HELP)),
        |k, v| set(k, &|r| r.length_mm = v),
    );
    param_slider(
        ui,
        st,
        id,
        "motif_height",
        tr("Motif height"),
        r.height_mm,
        0.5..=20.0,
        (words::width, tr(words::WIDTH_HELP)),
        |k, v| set(k, &|r| r.height_mm = v),
    );
    param_slider(
        ui,
        st,
        id,
        "motif_gap",
        tr("Gap"),
        r.gap_mm,
        0.0..=15.0,
        (words::open_spacing, tr(words::SPACING_HELP)),
        |k, v| set(k, &|r| r.gap_mm = v),
    );
    param_slider(
        ui,
        st,
        id,
        "len",
        tr("Longest stitch"),
        len,
        1.0..=7.0,
        (words::length, tr(words::LENGTH_HELP)),
        |k, v| {
            if let ObjectKind::Running { params, .. } = k {
                params.stitch_length_mm = v;
            }
        },
    );
}

/// A slider for a whole number of a column type. A drag is one undo step.
fn count_slider(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    label: &str,
    value: u32,
    range: RangeInclusive<u32>,
    set: impl FnOnce(&mut ColumnPattern, u32),
) {
    let mut v = value;
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ui.add(egui::Slider::new(&mut v, range).clamping(egui::SliderClamping::Edits));
        if resp.changed() {
            st.doc.edit_merged(
                format!("Change {}", label.to_lowercase()),
                MergeKey::new(format!("{}:{label}", id.0)),
                |d| {
                    if let Some(c) = d.object_mut(id).and_then(|o| column_params(&mut o.kind)) {
                        set(&mut c.pattern, v);
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

/// The column type and its settings, for a satin column or a line with a border.
fn column_section(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, params: &SatinParams, column: bool) {
    let mut pick = None;
    ui.horizontal(|ui| {
        ui.label(if column { "Column Type" } else { "Stitches" })
            .on_hover_text(tr(words::COLUMN_HELP));
        egui::ComboBox::from_id_salt("column_type")
            .selected_text(tr(params.pattern.name()))
            .show_ui(ui, |ui| {
                for p in ColumnPattern::ALL {
                    let on = params.pattern.same_type(&p);
                    if ui
                        .selectable_label(on, tr(p.name()))
                        .on_hover_text(words::column_help(&p))
                        .clicked()
                        && !on
                    {
                        pick = Some(p);
                    }
                }
            })
            .response
            .on_hover_text(words::column_help(&params.pattern));
    });
    if let Some(p) = pick {
        crate::stitch_types::set_column(st, &[id], p);
    }
    let pattern = params.pattern;
    let set_spacing = |k: &mut ObjectKind, v: f64| {
        if let Some(c) = column_params(k) {
            c.spacing_mm = v;
        }
    };
    match pattern {
        ColumnPattern::Satin
        | ColumnPattern::Raised { .. }
        | ColumnPattern::Tatami { .. }
        | ColumnPattern::Contour { .. } => param_slider(
            ui,
            st,
            id,
            "density",
            tr("Density"),
            params.spacing_mm,
            0.2..=1.5,
            (words::density, tr(words::DENSITY_HELP)),
            set_spacing,
        ),
        ColumnPattern::Zigzag
        | ColumnPattern::EStitch
        | ColumnPattern::Square
        | ColumnPattern::Coil
        | ColumnPattern::String { .. } => {
            let label = match pattern {
                ColumnPattern::Zigzag | ColumnPattern::String { .. } => "Spacing",
                ColumnPattern::EStitch => "Prong spacing",
                ColumnPattern::Square => "Tooth width",
                _ => "Loop spacing",
            };
            param_slider(
                ui,
                st,
                id,
                "density",
                label,
                params.spacing_mm,
                0.5..=5.0,
                (words::open_spacing, tr(words::SPACING_HELP)),
                set_spacing,
            );
        }
        // The motif sets its own length from the width.
        ColumnPattern::Motif { .. } => {}
        ColumnPattern::ProgramSplit { .. } => param_slider(
            ui,
            st,
            id,
            "density",
            tr("Density"),
            params.spacing_mm,
            0.2..=1.5,
            (words::density, tr(words::DENSITY_HELP)),
            set_spacing,
        ),
    }
    param_slider(
        ui,
        st,
        id,
        "pull",
        tr("Pull compensation"),
        params.pull_compensation_mm,
        0.0..=1.0,
        (words::pull, tr(words::PULL_HELP)),
        |k, v| {
            if let Some(c) = column_params(k) {
                c.pull_compensation_mm = v;
            }
        },
    );
    match pattern {
        ColumnPattern::Satin | ColumnPattern::Raised { .. } | ColumnPattern::Square => param_slider(
            ui,
            st,
            id,
            "split",
            tr("Split stitches over"),
            params.split_mm,
            1.0..=12.0,
            (words::split, tr(words::SPLIT_HELP)),
            |k, v| {
                if let Some(c) = column_params(k) {
                    c.split_mm = v;
                }
            },
        ),
        ColumnPattern::Tatami { stitch_mm } | ColumnPattern::Contour { stitch_mm } => param_slider(
            ui,
            st,
            id,
            "pattern_len",
            tr("Stitch length"),
            stitch_mm,
            1.0..=7.0,
            (words::length, tr(words::LENGTH_HELP)),
            |k, v| {
                if let Some(c) = column_params(k)
                    && let ColumnPattern::Tatami { stitch_mm } | ColumnPattern::Contour { stitch_mm } = &mut c.pattern
                {
                    *stitch_mm = v;
                }
            },
        ),
        _ => {}
    }
    if let ColumnPattern::ProgramSplit { scale } = pattern {
        let name = params.split_pattern.clone().unwrap_or_default().name;
        split_pattern_ui(ui, st, id, &name, scale, true);
    }
    if let ColumnPattern::Motif { aspect } = pattern {
        motif_row(ui, st, &params.motif.clone().unwrap_or_default());
        fill_free_slider(ui, st, id, tr("Motif length"), aspect, 0.3..=4.0, |k, v| {
            if let Some(c) = column_params(k)
                && let ColumnPattern::Motif { aspect } = &mut c.pattern
            {
                *aspect = v;
            }
        });
    }
    if let ColumnPattern::String { chord_mm } = pattern {
        param_slider(
            ui,
            st,
            id,
            "chord",
            tr("Chord length"),
            chord_mm,
            1.0..=12.0,
            (words::width, tr(words::CHORD_HELP)),
            |k, v| {
                if let Some(c) = column_params(k)
                    && let ColumnPattern::String { chord_mm } = &mut c.pattern
                {
                    *chord_mm = v;
                }
            },
        );
    }
    if let ColumnPattern::Raised { layers } = pattern {
        count_slider(ui, st, id, tr("Layers"), layers, 1..=5, |p, v| {
            if let ColumnPattern::Raised { layers } = p {
                *layers = v;
            }
        });
    }
    if column {
        let mut liquid = params.liquid;
        if ui
            .checkbox(&mut liquid, tr("Liquid"))
            .on_hover_text(tr(words::LIQUID_HELP))
            .changed()
        {
            st.doc.edit("Change liquid", |d| {
                if let Some(c) = d.object_mut(id).and_then(|o| column_params(&mut o.kind)) {
                    c.liquid = liquid;
                }
            });
        }
    }
    if matches!(pattern, ColumnPattern::Satin | ColumnPattern::Raised { .. }) {
        let mut short = params.short_stitches;
        if ui
            .checkbox(&mut short, tr("Short stitches at curves"))
            .on_hover_text(tr(words::SHORT_HELP))
            .changed()
        {
            st.doc.edit("Change short stitches", |d| {
                if let Some(c) = d.object_mut(id).and_then(|o| column_params(&mut o.kind)) {
                    c.short_stitches = short;
                }
            });
        }
    }
    let is_satin = matches!(
        st.doc.design().object(id).map(|o| &o.kind),
        Some(ObjectKind::Satin { .. })
    );
    if pattern.is_satin() {
        let mut puff = params.puff;
        if ui
            .checkbox(&mut puff, tr("Puff (3D foam)"))
            .on_hover_text(tr(words::PUFF_HELP))
            .changed()
        {
            crate::stitch_types::set_puff(st, &[id], puff);
        }
    }
    if is_satin && pattern.is_satin() {
        let mut omit = params.omit_last;
        if ui
            .checkbox(&mut omit, tr("Omit last stitch"))
            .on_hover_text(
                tr("The column ends one stitch early, on the other side. Use it where a column joins the next column, for a smoother join."),
            )
            .changed()
        {
            st.doc.edit("Change last stitch", |d| {
                if let Some(c) = d.object_mut(id).and_then(|o| column_params(&mut o.kind)) {
                    c.omit_last = omit;
                }
            });
        }
    }
    crate::effects_ui::column_effects(ui, st, id, params);
    underlay_ui(ui, st, id, &params.underlay);
}

/// A labeled list of the underlay settings. Returns `true` if the user
/// selected another one. Custom layers show only when the value has them.
pub fn underlay_combo(ui: &mut egui::Ui, salt: &str, value: &mut Underlay) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("Underlay")).on_hover_text(tr(words::UNDERLAY_HELP));
        egui::ComboBox::from_id_salt(salt)
            .selected_text(words::underlay(value))
            .show_ui(ui, |ui| {
                for u in [
                    Underlay::Light,
                    Underlay::Auto,
                    Underlay::Heavy,
                    Underlay::Double,
                    Underlay::Lace,
                    Underlay::None,
                ] {
                    let on = std::mem::discriminant(value) == std::mem::discriminant(&u);
                    if ui
                        .selectable_label(on, words::underlay(&u))
                        .on_hover_text(words::underlay_help(&u))
                        .clicked()
                        && !on
                    {
                        *value = u;
                        changed = true;
                    }
                }
            })
            .response
            .on_hover_text(words::underlay_help(value));
    });
    changed
}

/// Position, size, rotation and mirror for the selection, in screen
/// directions (the design-level rotation is taken into account).
/// X and Y are the center of the selection, measured from the hoop center.
fn transform_section(ui: &mut egui::Ui, st: &mut AppState, ids: &[ObjectId]) {
    use emb_geom::{Point, Transform};
    use emb_model::edit::scale_about;

    let design = st.doc.design();
    let Some(b) = design.bbox_of(ids) else { return };
    let rot = design.rotation_transform();
    let inv = rot.inverse().unwrap_or(Transform::IDENTITY);
    let hoop_c = design.hoop.center;
    let quarter = matches!(
        design.rotation,
        emb_model::Rotation::Deg90 | emb_model::Rotation::Deg270
    );
    let c = b.center();
    let offset = rot.apply(c) - hoop_c;
    let (w, h) = if quarter {
        (b.height(), b.width())
    } else {
        (b.width(), b.height())
    };
    let key_base = format!("props-transform-{}", ids.first().map(|i| i.0).unwrap_or(0));
    let lock_id = ui.id().with("lock_ratio");
    let mut lock = ui.data_mut(|d| *d.get_persisted_mut_or(lock_id, true));

    ui.strong(tr("Position and size"));
    let mut new_x = offset.x;
    let mut new_y = offset.y;
    let mut new_w = w;
    let mut new_h = h;
    let mut stop = false;
    let mut changed: Option<&str> = None;
    egui::Grid::new("transform_grid")
        .num_columns(4)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            let mut field = |ui: &mut egui::Ui, label: &str, v: &mut f64, min: f64, which: &'static str| {
                ui.label(label);
                let r = ui.add(
                    egui::DragValue::new(v)
                        .speed(0.1)
                        .range(min..=10_000.0)
                        // A straight line has no height: show 0, do not change it.
                        .clamp_existing_to_range(false)
                        .suffix(" mm")
                        .fixed_decimals(1),
                );
                if r.changed() {
                    changed = Some(which);
                }
                if r.drag_stopped() || r.lost_focus() {
                    stop = true;
                }
            };
            field(ui, "X", &mut new_x, -10_000.0, "x");
            field(ui, "Y", &mut new_y, -10_000.0, "y");
            ui.end_row();
            field(ui, "W", &mut new_w, 0.5, "w");
            field(ui, "H", &mut new_h, 0.5, "h");
            ui.end_row();
        });
    if ui.checkbox(&mut lock, tr("Keep proportions")).changed() {
        ui.data_mut(|d| d.insert_persisted(lock_id, lock));
    }

    if let Some(which) = changed {
        let t = match which {
            "x" | "y" => {
                let target = inv.apply(hoop_c + Point::new(new_x, new_y));
                let d = target - c;
                Transform::translate(d.x, d.y)
            }
            _ => {
                // Screen factors, then map to design axes.
                let (mut fx, mut fy) = (new_w / w.max(1e-9), new_h / h.max(1e-9));
                if lock {
                    let f = if which == "w" { fx } else { fy };
                    fx = f;
                    fy = f;
                }
                // A line has no height or width; do not scale that axis.
                if w < 1e-9 {
                    fx = 1.0;
                }
                if h < 1e-9 {
                    fy = 1.0;
                }
                let (sx, sy) = if quarter { (fy, fx) } else { (fx, fy) };
                scale_about(c, sx, sy)
            }
        };
        let label = if matches!(which, "x" | "y") { "Move" } else { "Resize" };
        let ids = ids.to_vec();
        st.doc
            .edit_merged(label, MergeKey::new(format!("{key_base}-{which}")), |d| {
                d.transform_objects(&ids, &t)
            });
    }
    if stop {
        st.doc.end_merge();
    }

    ui.horizontal(|ui| {
        ui.label(tr("Rotate"));
        for (text, deg) in [("⟲ 90°", -90.0), ("⟲ 15°", -15.0), ("⟳ 15°", 15.0), ("⟳ 90°", 90.0)] {
            if ui.small_button(text).clicked() {
                st.rotate_selection(deg);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Mirror"));
        if ui.small_button(tr("↔ Left-right")).clicked() {
            st.mirror_selection(true);
        }
        if ui.small_button(tr("↕ Up-down")).clicked() {
            st.mirror_selection(false);
        }
        if ui.small_button(tr("Center in Hoop")).clicked() {
            st.center_in_hoop();
        }
    });
}

/// The fill type and its settings.
fn fill_section(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, params: &FillParams) {
    let mut pick = None;
    ui.horizontal(|ui| {
        ui.label(tr("Fill Type")).on_hover_text(tr(words::FILL_HELP));
        egui::ComboBox::from_id_salt("fill_type")
            .selected_text(tr(params.pattern.name()))
            .show_ui(ui, |ui| {
                for p in FillPattern::all() {
                    let on = params.pattern.same_type(&p);
                    if ui
                        .selectable_label(on, tr(p.name()))
                        .on_hover_text(words::fill_help(&p))
                        .clicked()
                        && !on
                    {
                        pick = Some(p);
                    }
                }
            })
            .response
            .on_hover_text(words::fill_help(&params.pattern));
    });
    if let Some(p) = pick {
        crate::stitch_types::set_fill(st, &[id], p);
    }
    let pattern = &params.pattern;
    let set_fill = |k: &mut ObjectKind, f: &dyn Fn(&mut FillParams)| {
        if let ObjectKind::Fill { params, .. } = k {
            f(params);
        }
    };
    let column = match pattern {
        FillPattern::Columns { column } => Some(*column),
        _ => None,
    };
    let satin_like = column.is_some_and(|c| !c.is_open());
    type Word = fn(f64) -> &'static str;
    let spacing: Option<(&str, Word, RangeInclusive<f64>)> = match pattern {
        FillPattern::CrossStitch { .. } => None,
        FillPattern::Columns { column } => Some(match column {
            ColumnPattern::EStitch => (tr("Prong spacing"), words::open_spacing, 0.5..=5.0),
            ColumnPattern::Coil => (tr("Loop spacing"), words::open_spacing, 0.5..=5.0),
            ColumnPattern::Zigzag => (tr("Spacing"), words::open_spacing, 0.5..=5.0),
            _ => (tr("Density"), words::density, 0.2..=1.5),
        }),
        FillPattern::IslandCoil => Some((tr("Ring width"), words::open_spacing, 0.8..=5.0)),
        FillPattern::String => Some((tr("Point spacing"), words::open_spacing, 0.5..=5.0)),
        FillPattern::Maze | FillPattern::Stipple { .. } => Some((tr("Spacing"), words::open_spacing, 0.5..=5.0)),
        FillPattern::Laydown { .. } => Some((tr("Spacing"), words::open_spacing, 0.8..=5.0)),
        FillPattern::Motif { .. } => Some((tr("Row spacing"), words::open_spacing, 1.0..=30.0)),
        FillPattern::DoubleSquare { .. } => Some((tr("Spacing"), words::open_spacing, 0.3..=3.0)),
        FillPattern::Tartan { .. } => Some((tr("Row spacing"), words::open_spacing, 0.4..=3.0)),
        FillPattern::Meander { .. } => Some((tr("Tile size"), words::open_spacing, 0.8..=8.0)),
        FillPattern::Mesh { .. } => Some((tr("Hole size"), words::open_spacing, 0.8..=5.0)),
        _ => Some((tr("Density"), words::density, 0.2..=1.0)),
    };
    if let Some((label, word, range)) = spacing {
        param_slider(
            ui,
            st,
            id,
            "density",
            label,
            params.spacing_mm,
            range,
            (word, tr(words::DENSITY_HELP)),
            |k, v| set_fill(k, &|p| p.spacing_mm = v),
        );
    }
    let length: Option<(&str, Word, RangeInclusive<f64>)> = match pattern {
        FillPattern::CrossStitch { .. } | FillPattern::ProgramSplit { .. } | FillPattern::FlexiSplit { .. } => None,
        FillPattern::UserSplit { .. } => Some((tr("Longest stitch"), words::length, 1.0..=12.0)),
        FillPattern::Columns { .. } if satin_like => Some((tr("Split stitches over"), words::split, 1.0..=12.0)),
        FillPattern::Columns { .. } => None,
        FillPattern::IslandCoil => Some((tr("Loop spacing"), words::length, 1.0..=7.0)),
        FillPattern::Motif { .. } => Some((tr("Longest stitch"), words::length, 1.0..=7.0)),
        _ => Some((tr("Stitch length"), words::length, 1.0..=7.0)),
    };
    if let Some((label, word, range)) = length {
        param_slider(
            ui,
            st,
            id,
            "len",
            label,
            params.stitch_length_mm,
            range,
            (word, tr(words::LENGTH_HELP)),
            |k, v| set_fill(k, &|p| p.stitch_length_mm = v),
        );
    }
    if satin_like
        || matches!(
            pattern,
            FillPattern::Tatami
                | FillPattern::Florentine { .. }
                | FillPattern::Square
                | FillPattern::DoubleSquare { .. }
                | FillPattern::ProgramSplit { .. }
                | FillPattern::FlexiSplit { .. }
                | FillPattern::UserSplit { .. }
                | FillPattern::Tartan { .. }
                | FillPattern::Gradient { .. }
        )
    {
        param_slider(
            ui,
            st,
            id,
            "pull",
            tr("Pull compensation"),
            params.pull_compensation_mm,
            0.0..=1.0,
            (words::pull, tr(words::PULL_HELP)),
            |k, v| set_fill(k, &|p| p.pull_compensation_mm = v),
        );
    }
    let uses_angle = match pattern {
        FillPattern::Tatami
        | FillPattern::Contour
        | FillPattern::Columns { .. }
        | FillPattern::Square
        | FillPattern::DoubleSquare { .. }
        | FillPattern::Laydown { .. }
        | FillPattern::ProgramSplit { .. }
        | FillPattern::FlexiSplit { .. }
        | FillPattern::UserSplit { .. }
        | FillPattern::Tartan { .. }
        | FillPattern::Gradient { .. }
        | FillPattern::Meander { .. }
        | FillPattern::Mesh { .. } => true,
        FillPattern::Florentine { guide } => guide.is_empty(),
        FillPattern::Guided { guides } => guides.is_empty(),
        _ => false,
    };
    // A direction line sets the stitch angle instead.
    if let Some(dir) = &params.direction {
        let mut angle = dir.angle_deg;
        let mut remove = false;
        ui.horizontal(|ui| {
            ui.label(tr("Angle to the direction line"))
                .on_hover_text(tr("90°: the stitches go square across the direction line."));
            let resp = ui.add(
                egui::Slider::new(&mut angle, 1.0..=179.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix("°")
                    .fixed_decimals(0),
            );
            remove = ui
                .small_button(tr("Remove line"))
                .on_hover_text(tr("The stitches go at the fill angle again."))
                .clicked();
            if resp.changed() {
                st.doc
                    .edit_merged("Change direction angle", MergeKey::new(format!("{}:dir", id.0)), |d| {
                        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                            && let Some(dir) = &mut params.direction
                        {
                            dir.angle_deg = angle;
                        }
                    });
                if !resp.dragged() {
                    st.doc.end_merge();
                }
            }
            if resp.drag_stopped() {
                st.doc.end_merge();
            }
        });
        if remove {
            st.doc.edit("Remove direction line", |d| {
                if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    params.direction = None;
                }
            });
        }
    }
    if uses_angle && params.direction.is_none() {
        let mut angle = params.angle_deg;
        ui.horizontal(|ui| {
            ui.label(tr("Angle")).on_hover_text(tr(words::ANGLE_HELP));
            let resp = ui.add(
                egui::Slider::new(&mut angle, 0.0..=179.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix("°")
                    .fixed_decimals(0),
            );
            ui.weak(format!("({})", words::angle(angle)));
            if resp.changed() {
                st.doc
                    .edit_merged("Change angle", MergeKey::new(format!("{}:angle", id.0)), |d| {
                        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                            params.angle_deg = angle;
                        }
                    });
                if !resp.dragged() {
                    st.doc.end_merge();
                }
            }
            if resp.drag_stopped() {
                st.doc.end_merge();
            }
        });
    }
    match pattern {
        FillPattern::Columns {
            column: ColumnPattern::Raised { layers },
        } => fill_slider(ui, st, id, tr("Layers"), f64::from(*layers), 1.0..=5.0, "", |p, v| {
            if let FillPattern::Columns {
                column: ColumnPattern::Raised { layers },
            } = p
            {
                *layers = v.round() as u32;
            }
        }),
        FillPattern::DoubleSquare { angle_deg } => {
            fill_slider(ui, st, id, tr("Second layer"), *angle_deg, 0.0..=179.0, "°", |p, v| {
                if let FillPattern::DoubleSquare { angle_deg } = p {
                    *angle_deg = v;
                }
            })
        }
        FillPattern::Mesh { angle_deg } => fill_slider(
            ui,
            st,
            id,
            tr("Second layer"),
            *angle_deg,
            30.0..=150.0,
            "°",
            |p, v| {
                if let FillPattern::Mesh { angle_deg } = p {
                    *angle_deg = v;
                }
            },
        ),
        FillPattern::Laydown { layers, angle_deg } => {
            fill_slider(ui, st, id, tr("Layers"), f64::from(*layers), 1.0..=2.0, "", |p, v| {
                if let FillPattern::Laydown { layers, .. } = p {
                    *layers = v.round() as u32;
                }
            });
            if *layers >= 2 {
                fill_slider(ui, st, id, tr("Second layer"), *angle_deg, 0.0..=179.0, "°", |p, v| {
                    if let FillPattern::Laydown { angle_deg, .. } = p {
                        *angle_deg = v;
                    }
                });
            }
        }
        FillPattern::CrossStitch { cell_mm, style, passes } => {
            fill_slider(ui, st, id, tr("Cross size"), *cell_mm, 1.0..=8.0, " mm", |p, v| {
                if let FillPattern::CrossStitch { cell_mm, .. } = p {
                    *cell_mm = v;
                }
            });
            let mut new_style = *style;
            let mut bold = *passes >= 3;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(tr("Crosses"));
                for (s, name) in [
                    (CrossStyle::Full, "X"),
                    (CrossStyle::Diagonal, "/"),
                    (CrossStyle::Upright, "+"),
                ] {
                    changed |= ui.selectable_value(&mut new_style, s, name).changed();
                }
                changed |= ui
                    .checkbox(&mut bold, tr("Bold"))
                    .on_hover_text(tr("Each leg three times, for a thicker cross."))
                    .changed();
            });
            if changed {
                st.doc.edit("Change crosses", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                        && let FillPattern::CrossStitch { style, passes, .. } = &mut params.pattern
                    {
                        *style = new_style;
                        *passes = if bold { 3 } else { 1 };
                    }
                });
            }
        }
        FillPattern::Motif {
            motif,
            length_mm,
            height_mm,
            stagger,
            gradient,
            globe,
        } => {
            motif_row(ui, st, motif);
            fill_slider(ui, st, id, tr("Motif length"), *length_mm, 1.0..=30.0, " mm", |p, v| {
                if let FillPattern::Motif { length_mm, .. } = p {
                    *length_mm = v;
                }
            });
            fill_slider(ui, st, id, tr("Motif height"), *height_mm, 0.5..=20.0, " mm", |p, v| {
                if let FillPattern::Motif { height_mm, .. } = p {
                    *height_mm = v;
                }
            });
            fill_slider(
                ui,
                st,
                id,
                tr("Row offset"),
                *stagger,
                0.0..=1.0,
                " of a motif",
                |p, v| {
                    if let FillPattern::Motif { stagger, .. } = p {
                        *stagger = v;
                    }
                },
            );
            fill_slider(ui, st, id, tr("Gradient"), *gradient, 0.25..=4.0, " × size", |p, v| {
                if let FillPattern::Motif { gradient, .. } = p {
                    *gradient = v;
                }
            });
            let mut ball = *globe;
            if ui
                .checkbox(&mut ball, tr("3D warp (ball)"))
                .on_hover_text(tr("The rows bend like a ball: large in the middle, small at the edge."))
                .changed()
            {
                st.doc.edit("Change 3D warp", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                        && let FillPattern::Motif { globe, .. } = &mut params.pattern
                    {
                        *globe = ball;
                    }
                });
            }
        }
        FillPattern::Stipple { style } => {
            let mut new_style = *style;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(tr("Line"));
                for (s, name) in [
                    (RunStyle::Run, tr("Run")),
                    (RunStyle::Backstitch, tr("Backstitch")),
                    (RunStyle::Stemstitch, tr("Stem stitch")),
                ] {
                    changed |= ui.selectable_value(&mut new_style, s, name).changed();
                }
            });
            if changed {
                st.doc.edit("Change stipple line", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        params.pattern = FillPattern::Stipple { style: new_style };
                    }
                });
            }
        }
        _ => {}
    }
    match pattern {
        FillPattern::ProgramSplit { pattern: pat, scale } | FillPattern::FlexiSplit { pattern: pat, scale } => {
            split_pattern_ui(ui, st, id, &pat.name, *scale, false)
        }
        FillPattern::UserSplit { lines } => {
            ui.horizontal(|ui| {
                if lines.is_empty() {
                    ui.weak(tr("No split lines: the rows are split at the stitch length."))
                        .on_hover_text(tr(words::SPLIT_LINES_HELP));
                } else {
                    ui.label(crate::i18n::fill("{} split lines", &[&(lines.len())]))
                        .on_hover_text(tr(words::SPLIT_LINES_HELP));
                    if ui.button(tr("Remove Split Lines")).clicked() {
                        st.doc.edit("Remove Split Lines", |d| {
                            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                                params.pattern = FillPattern::UserSplit { lines: Vec::new() };
                            }
                        });
                    }
                }
            });
        }
        FillPattern::Radial { center } => radial_center_ui(ui, st, id, *center, false),
        FillPattern::Circular { center } => radial_center_ui(ui, st, id, *center, true),
        FillPattern::Meander { tile } => {
            let mut new_tile = *tile;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(tr("Tiles")).on_hover_text(tr(words::TILE_HELP));
                for t in emb_model::MeanderTile::ALL {
                    changed |= ui.selectable_value(&mut new_tile, t, tr(t.name())).changed();
                }
            });
            if changed {
                st.doc.edit("Change meander tiles", |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        params.pattern = FillPattern::Meander { tile: new_tile };
                    }
                });
            }
        }
        FillPattern::Guided { guides } => {
            ui.horizontal(|ui| {
                if guides.is_empty() {
                    ui.weak(tr("No guide lines: the rows are straight."))
                        .on_hover_text(tr(words::GUIDE_HELP));
                } else {
                    ui.label(crate::i18n::fill("{} guide lines", &[&(guides.len())]))
                        .on_hover_text(tr(words::GUIDE_HELP));
                    if ui.button(tr("Remove Guides")).clicked() {
                        st.doc.edit("Remove guide lines", |d| {
                            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                                params.pattern = FillPattern::Guided { guides: Vec::new() };
                            }
                        });
                    }
                }
            });
        }
        FillPattern::Tartan { sett, color } => {
            let code = sett.colors.get(*color as usize).cloned().unwrap_or_default();
            let (name, _) = emb_model::tartan::code_color(&code);
            ui.label(crate::i18n::fill("Sett: {}", &[&sett.text()]));
            ui.horizontal(|ui| {
                ui.weak(crate::i18n::fill("This fill sews the stripes of {}.", &[&tr(&name)]));
                if ui.button(tr("Tartan…")).clicked() {
                    st.color_fills.open = true;
                    st.color_fills.gradient = false;
                }
            });
        }
        FillPattern::Gradient { stops, color, .. } => {
            let at = stops.get(*color as usize).copied().unwrap_or(0.0) * 100.0;
            ui.horizontal(|ui| {
                ui.weak(crate::i18n::fill(
                    "Color {} of {}, at {} %.",
                    &[&(*color + 1), &stops.len(), &format!("{at:.0}")],
                ));
                if ui.button(tr("Gradient…")).clicked() {
                    st.color_fills.open = true;
                    st.color_fills.gradient = true;
                }
            });
        }
        FillPattern::Florentine { guide } => {
            ui.horizontal(|ui| {
                if guide.is_empty() {
                    ui.weak(tr("No guide line: the rows are straight."))
                        .on_hover_text(tr(words::GUIDE_HELP));
                } else {
                    ui.label(crate::i18n::fill("Guide line: {} points", &[&(guide.len())]))
                        .on_hover_text(tr(words::GUIDE_HELP));
                    if ui.button(tr("Remove guide")).clicked() {
                        st.doc.edit("Remove guide line", |d| {
                            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                                params.pattern = FillPattern::Florentine { guide: Vec::new() };
                            }
                        });
                    }
                }
            });
        }
        _ => {}
    }
    if pattern.takes_stamps() {
        stamps_ui(ui, st, id, params);
    }
    crate::effects_ui::fill_effects(ui, st, id, params);
    underlay_ui(ui, st, id, &params.underlay);
}

/// The program-split pattern and its scale, of a fill (or of a column).
fn split_pattern_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, current: &str, scale: f64, column: bool) {
    let mut pick = None;
    ui.horizontal(|ui| {
        ui.label(tr("Pattern")).on_hover_text(tr(words::SPLIT_PATTERN_HELP));
        egui::ComboBox::from_id_salt("split_pattern")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for p in emb_model::split::split_patterns() {
                    if ui.selectable_label(p.name == current, &p.name).clicked() && p.name != current {
                        pick = Some(p);
                    }
                }
            });
    });
    if let Some(p) = pick {
        st.doc.edit(format!("Pattern {}", p.name), |d| {
            let Some(o) = d.object_mut(id) else { return };
            match &mut o.kind {
                ObjectKind::Fill { params, .. } => match &mut params.pattern {
                    FillPattern::ProgramSplit { pattern, .. } | FillPattern::FlexiSplit { pattern, .. } => *pattern = p,
                    _ => {}
                },
                k => {
                    if let Some(c) = column_params(k) {
                        c.split_pattern = Some(p);
                    }
                }
            }
        });
    }
    let set = move |k: &mut ObjectKind, v: f64| match k {
        ObjectKind::Fill { params, .. } => match &mut params.pattern {
            FillPattern::ProgramSplit { scale, .. } | FillPattern::FlexiSplit { scale, .. } => *scale = v,
            _ => {}
        },
        k => {
            if column
                && let Some(c) = column_params(k)
                && let ColumnPattern::ProgramSplit { scale } = &mut c.pattern
            {
                *scale = v;
            }
        }
    };
    let mut v = scale;
    ui.horizontal(|ui| {
        ui.label(tr("Pattern size"));
        let resp = ui.add(
            egui::Slider::new(&mut v, 0.5..=4.0)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" ×")
                .fixed_decimals(2),
        );
        if resp.changed() {
            st.doc.edit_merged(
                "Change pattern size",
                MergeKey::new(format!("{}:split_scale", id.0)),
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

/// The carving stamps of a fill: add, size, turn, move and remove.
fn stamps_ui(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, params: &FillParams) {
    ui.separator();
    ui.strong(tr("Carving stamps")).on_hover_text(tr(words::STAMP_HELP));
    let middle = st
        .doc
        .design()
        .object(id)
        .and_then(|o| o.control_bbox())
        .map(|b| b.center())
        .unwrap_or_default();
    let mut remove = None;
    for (i, s) in params.stamps.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(&s.stamp.name);
            if ui.small_button(tr("Remove")).clicked() {
                remove = Some(i);
            }
        });
        let fields = [
            (tr("Size"), s.size_mm, 2.0..=150.0, " mm"),
            (tr("Turn"), s.angle_deg, -180.0..=180.0, "°"),
            (tr("Right"), s.center.x - middle.x, -150.0..=150.0, " mm"),
            (tr("Down"), s.center.y - middle.y, -150.0..=150.0, " mm"),
        ];
        for (k, (label, value, range, suffix)) in fields.into_iter().enumerate() {
            let mut v = value;
            ui.horizontal(|ui| {
                ui.label(label);
                let resp = ui.add(
                    egui::Slider::new(&mut v, range)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix(suffix)
                        .fixed_decimals(1),
                );
                if resp.changed() {
                    st.doc
                        .edit_merged("Change stamp", MergeKey::new(format!("{}:stamp:{i}:{k}", id.0)), |d| {
                            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                                && let Some(s) = params.stamps.get_mut(i)
                            {
                                match k {
                                    0 => s.size_mm = v,
                                    1 => s.angle_deg = v,
                                    2 => s.center.x = middle.x + v,
                                    _ => s.center.y = middle.y + v,
                                }
                            }
                        });
                    if !resp.dragged() {
                        st.doc.end_merge();
                    }
                }
                if resp.drag_stopped() {
                    st.doc.end_merge();
                }
            });
        }
    }
    if let Some(i) = remove {
        st.doc.edit("Remove stamp", |d| {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                && i < params.stamps.len()
            {
                params.stamps.remove(i);
            }
        });
    }
    let mut add = None;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("add_stamp")
            .selected_text(tr("Add a stamp…"))
            .show_ui(ui, |ui| {
                for s in emb_model::split::stamps() {
                    if ui.selectable_label(false, &s.name).clicked() {
                        add = Some(s);
                    }
                }
            });
    });
    if let Some(stamp) = add {
        let size = st
            .doc
            .design()
            .object(id)
            .and_then(|o| o.control_bbox())
            .map_or(20.0, |b| (b.width().min(b.height()) * 0.6).max(5.0));
        st.doc.edit(format!("Stamp {}", stamp.name), |d| {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                params.stamps.push(emb_model::StampPlacement {
                    stamp,
                    center: middle,
                    size_mm: size,
                    angle_deg: 0.0,
                });
            }
        });
    }
}

/// The center of a radial or a circular fill, as an offset from the middle
/// of the shape.
fn radial_center_ui(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    center: Option<emb_geom::Point>,
    circular: bool,
) {
    let pattern = move |center| {
        if circular {
            FillPattern::Circular { center }
        } else {
            FillPattern::Radial { center }
        }
    };
    let Some(middle) = st
        .doc
        .design()
        .object(id)
        .and_then(|o| o.control_bbox())
        .map(|b| b.center())
    else {
        return;
    };
    let off = center.map_or(emb_geom::Point::ZERO, |c| c - middle);
    let (mut x, mut y) = (off.x, off.y);
    let mut changed = false;
    let mut stop = false;
    ui.horizontal(|ui| {
        ui.label(tr("Center")).on_hover_text(tr(words::CENTER_HELP));
        for v in [&mut x, &mut y] {
            let r = ui.add(egui::DragValue::new(v).speed(0.2).suffix(" mm").fixed_decimals(1));
            changed |= r.changed();
            stop |= r.drag_stopped() || r.lost_focus();
        }
        if center.is_some() && ui.small_button(tr("Middle")).clicked() {
            st.doc.edit("Fill center", |d| {
                if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    params.pattern = pattern(None);
                }
            });
        }
    });
    if changed {
        let c = middle + emb_geom::Point::new(x, y);
        st.doc
            .edit_merged("Fill center", MergeKey::new(format!("{}:radial", id.0)), |d| {
                if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                    params.pattern = pattern(Some(c));
                }
            });
    }
    if stop {
        st.doc.end_merge();
    }
}

/// A slider for a setting of the fill type. A drag is one undo step.
#[allow(clippy::too_many_arguments)]
fn fill_slider(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    label: &str,
    value: f64,
    range: RangeInclusive<f64>,
    suffix: &str,
    set: impl FnOnce(&mut FillPattern, f64),
) {
    let mut v = value;
    let whole = suffix.is_empty();
    ui.horizontal(|ui| {
        ui.label(label);
        let mut slider = egui::Slider::new(&mut v, range)
            .clamping(egui::SliderClamping::Edits)
            .suffix(suffix);
        slider = if whole {
            slider.step_by(1.0).fixed_decimals(0)
        } else {
            slider.fixed_decimals(1)
        };
        let resp = ui.add(slider);
        if resp.changed() {
            st.doc.edit_merged(
                format!("Change {}", label.to_lowercase()),
                MergeKey::new(format!("{}:fill:{label}", id.0)),
                |d| {
                    if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                        set(&mut params.pattern, v);
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

/// A slider for a column setting with no unit. A drag is one undo step.
fn fill_free_slider(
    ui: &mut egui::Ui,
    st: &mut AppState,
    id: ObjectId,
    label: &str,
    value: f64,
    range: RangeInclusive<f64>,
    set: impl FnOnce(&mut ObjectKind, f64),
) {
    let mut v = value;
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ui.add(
            egui::Slider::new(&mut v, range)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" × width")
                .fixed_decimals(1),
        );
        if resp.changed() {
            st.doc.edit_merged(
                format!("Change {}", label.to_lowercase()),
                MergeKey::new(format!("{}:col:{label}", id.0)),
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
