//! The controls of the panels that are not plain buttons (drop-down
//! lists, pickers and small forms).

use super::*;

// ---- Custom controls ----

pub(super) fn select_by(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    dropdown(
        ui,
        ph::PALETTE,
        tr("By Color"),
        tr("Select all objects of a color."),
        true,
        |ui| {
            let threads = st.doc.design().threads.clone();
            for t in threads {
                if ui.button(&t.name).clicked() {
                    st.select_by_thread(t.id);
                }
            }
        },
    );
    dropdown(
        ui,
        ph::SHAPES,
        tr("By Type"),
        tr("Select all objects of a stitch type."),
        true,
        |ui| {
            for (label, name) in [
                (tr("Running lines"), "Running"),
                (tr("Satin columns"), "Satin"),
                (tr("Fills"), "Fill"),
            ] {
                if ui.button(label).clicked() {
                    st.select_by_type(name);
                }
            }
        },
    );
}

/// The outline type of the selected lines and the column type of the
/// selected columns, each one undo step.
pub(super) fn stitch_types(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    use emb_model::ColumnPattern;
    dropdown(
        ui,
        ph::WAVE_TRIANGLE,
        tr("Outline Type"),
        tr("Sew the selected lines as a plain line, or as a column of a stitch type along the line."),
        s.lines > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            let mut pick = None;
            if ui
                .button(tr("Line"))
                .on_hover_text(tr(crate::words::OUTLINE_HELP))
                .clicked()
            {
                pick = Some(None);
            }
            for p in ColumnPattern::ALL {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::column_help(&p))
                    .clicked()
                {
                    pick = Some(Some(p));
                }
            }
            if let Some(p) = pick {
                let n = crate::stitch_types::set_outline(st, &ids, p);
                st.message = crate::i18n::fill(
                    "Outline type of {} lines: {}.",
                    &[&n, &(p.map_or(tr("Line"), |p| tr(p.name())))],
                );
            }
        },
    );
    dropdown(
        ui,
        ph::SPIRAL,
        tr("Fill Type"),
        tr("The stitch type of the selected fills."),
        s.fills > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            for p in emb_model::FillPattern::all() {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::fill_help(&p))
                    .clicked()
                {
                    let n = crate::stitch_types::set_fill(st, &ids, p.clone());
                    st.message = crate::i18n::fill("Fill type of {} fills: {}.", &[&n, &tr(p.name())]);
                }
            }
        },
    );
    dropdown(
        ui,
        ph::WAVE_SQUARE,
        tr("Column Type"),
        tr("The stitch type of the selected satin columns and of lines with a width."),
        s.columns > 0,
        |ui| {
            let ids = st.selection.in_sew_order(st.doc.design());
            for p in ColumnPattern::ALL {
                if ui
                    .button(tr(p.name()))
                    .on_hover_text(crate::words::column_help(&p))
                    .clicked()
                {
                    let n = crate::stitch_types::set_column(st, &ids, p);
                    st.message = crate::i18n::fill("Column type of {} objects: {}.", &[&n, &tr(p.name())]);
                }
            }
        },
    );
}

/// Keep originals and the overlap amount of the shaping commands.
pub(super) fn shaping_options(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    ui.vertical(|ui| {
        ui.checkbox(&mut st.shaping.keep_originals, tr("Keep originals"))
            .on_hover_text(tr("The selected objects stay; the result comes after them."));
        ui.checkbox(&mut st.shaping.linked, tr("Keep linked")).on_hover_text(tr(
            "Flatten, Divide and Remove overlaps are cut again when their objects move. Unlink ends it.",
        ));
        ui.horizontal(|ui| {
            ui.label(tr("Overlap")).on_hover_text(tr(
                "How far a cut piece goes under the shape that cut it, so no fabric shows between the pieces. \
                 Also for Remove overlaps and the knife.",
            ));
            ui.add(
                egui::DragValue::new(&mut st.shaping.overlap_mm)
                    .range(0.0..=5.0)
                    .clamp_existing_to_range(false)
                    .speed(0.05)
                    .fixed_decimals(1)
                    .suffix(" mm"),
            );
        });
    });
}

pub(super) fn rotate_skew(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let (mut sx, mut sy, mut rot) = st.arrange_values;
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut rot)
                .range(-360.0..=360.0)
                .clamp_existing_to_range(false)
                .suffix("°"),
        );
        let r = small(
            ui,
            Icon::Glyph(ph::ARROW_CLOCKWISE),
            tr("Rotate"),
            false,
            s.n > 0,
            false,
        )
        .on_hover_text(tr("Turn the selection by this angle (clockwise)."));
        if r.clicked() {
            st.rotate_selection(rot);
        }
    });
    ui.horizontal(|ui| {
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
    });
    let r = small(ui, Icon::Glyph(ph::PARALLELOGRAM), tr("Skew"), false, s.n > 0, false)
        .on_hover_text(tr("Slant the selection by these angles."));
    if r.clicked() {
        st.skew_selection(sx, sy);
    }
    st.arrange_values = (sx, sy, rot);
}

/// Show all and the Show colors menu, in one column.
pub(super) fn show_colors(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::ShowAll, false);
    dropdown(
        ui,
        ph::PALETTE,
        tr("Show Colors"),
        tr("Show or hide each color."),
        true,
        |ui| {
            let threads = st.doc.design().threads.clone();
            for t in threads {
                let mut on = st.view_filter.threads.as_ref().is_none_or(|s| s.contains(&t.id));
                if ui.checkbox(&mut on, &t.name).changed() {
                    let all: std::collections::BTreeSet<_> = st.doc.design().threads.iter().map(|t| t.id).collect();
                    let set = st.view_filter.threads.get_or_insert(all.clone());
                    if on {
                        set.insert(t.id);
                    } else {
                        set.remove(&t.id);
                    }
                    if *set == all {
                        st.view_filter.threads = None;
                    }
                }
            }
        },
    );
}

/// Center at stitch and travel by function, in one column.
pub(super) fn cursor_tools(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::CenterAtCursor, false);
    function_pick(ui, st, s);
}

/// Remove guides, dark mode and the Panels menu, in one column.
pub(super) fn workspace_more(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::RemoveGuides, false);
    cmd_button(ui, st, s, Cmd::Toggle(Flag::DarkMode), false);
    panels_menu(ui, st, s);
}

/// Artwork, Stitches and the Thread look menu (flat or 3D, the light, the
/// fabric texture), in one column.
pub(super) fn look_column(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Artwork), false);
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Stitches), false);
    let (icon, label, tip) = Cmd::Toggle(Flag::ThreadLook).meta();
    let resp = small(ui, icon, label, st.show_thread_look, true, true).on_hover_text(tip);
    egui::Popup::menu(&resp).show(|ui| crate::viewtools::look_ui(ui, st));
}

/// Overview, the Views menu (split view, new view, other design) and
/// Previous view, in one column.
pub(super) fn views_column(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    cmd_button(ui, st, s, Cmd::Toggle(Flag::Overview), false);
    let ctx = ui.ctx().clone();
    let resp = small(
        ui,
        Icon::Glyph(ph::COLUMNS),
        tr("Views"),
        st.show_split || st.other_design.is_some(),
        true,
        true,
    )
    .on_hover_text(tr(
        "More views: the split view, new views of the design, and another design next to it.",
    ));
    egui::Popup::menu(&resp).show(|ui| {
        for cmd in [Cmd::Toggle(Flag::SplitView), Cmd::NewView, Cmd::OtherDesign] {
            let (_, label, tip) = cmd.meta();
            let text = if st.is_on(cmd) {
                format!("✔ {label}")
            } else {
                label.to_string()
            };
            let r = ui
                .add_enabled(st.can(cmd, s), egui::Button::new(text))
                .on_hover_text(tip);
            if r.hovered() {
                st.hover_cmd = Some(cmd);
            }
            if r.clicked() {
                st.run(cmd, &ctx);
                ui.close();
            }
        }
    });
    cmd_button(ui, st, s, Cmd::PreviousView, false);
}

/// View > Panels: open and close each panel.
pub(super) fn panels_menu(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    dropdown(
        ui,
        ph::SIDEBAR_SIMPLE,
        tr("Panels"),
        tr("Open and close the panels."),
        true,
        |ui| {
            for (tab, name) in crate::viewtools::PANELS {
                let mut on = st.panel_shown(tab);
                if ui.checkbox(&mut on, tr(name)).changed() {
                    st.set_panel_shown(tab, on);
                }
            }
        },
    );
}

/// The kind of function that travel by function goes to.
pub(super) fn function_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let tip = crate::i18n::fill(
        "Travel by function: the functions that Shift+Page Up and Shift+Page Down go to. Now: {}.",
        &[&st.function_travel.label()],
    );
    dropdown(ui, ph::FLOW_ARROW, tr("Functions"), &tip, true, |ui| {
        for f in crate::viewtools::FunctionTravel::ALL {
            if ui.selectable_label(st.function_travel == f, f.label()).clicked() {
                st.function_travel = f;
            }
        }
    });
}

/// Design > Favorites: apply a favorite style to the selection.
pub(super) fn favorites(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let names: Vec<String> = st.settings.favorite_styles.iter().map(|f| f.name.clone()).collect();
    let mut apply = None;
    dropdown(
        ui,
        ph::STAR,
        tr("Favorites"),
        tr("Your favorite styles, for all designs. Make a style a favorite with its star in the Styles panel."),
        true,
        |ui| {
            if names.is_empty() {
                ui.weak(tr("No favorites. Click the star of a style in the Styles panel."));
            }
            for (i, name) in names.iter().enumerate() {
                if ui
                    .add_enabled(s.n > 0, egui::Button::new(format!("{}  {name}", i + 1)))
                    .on_disabled_hover_text(tr("Select objects first"))
                    .clicked()
                {
                    apply = Some(i);
                    ui.close();
                }
            }
        },
    );
    if let Some(i) = apply {
        st.apply_favorite(i);
    }
}

pub(super) fn chart_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    ui.label(tr("Chart"));
    let shown = st.chart().map(|c| c.name.clone()).unwrap_or_default();
    let names: Vec<String> = st.charts.iter().map(|c| c.name.clone()).collect();
    egui::ComboBox::from_id_salt("ribbon_chart")
        .selected_text(shown)
        .width(150.0)
        .show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut st.chart_index, i, n);
            }
        });
}

pub(super) fn colorway_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let d = st.doc.design();
    let names: Vec<String> = d.colorways.iter().map(|c| c.name.clone()).collect();
    let current = d.colorway;
    ui.label(tr("Current"));
    let shown = names
        .get(current)
        .cloned()
        .unwrap_or_else(|| tr("Design colors").into());
    egui::ComboBox::from_id_salt("ribbon_colorway")
        .selected_text(shown)
        .width(150.0)
        .show_ui(ui, |ui| {
            if names.is_empty() {
                ui.weak(tr("Only the design colors. Click New colorway to add one."));
            }
            for (i, n) in names.iter().enumerate() {
                if ui.selectable_label(i == current, n).clicked() {
                    st.switch_colorway(i);
                }
            }
        });
}

pub(super) fn background_pick(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let bg = st.doc.design().background;
    ui.label(tr("Background"));
    let mut c = bg.map(crate::canvas::color32).unwrap_or(Color32::WHITE);
    let r = egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque)
        .on_hover_text(tr("The fabric color inside the hoop. It changes with the colorway."));
    if r.changed() {
        let rgb = emb_model::Rgb(c.r(), c.g(), c.b());
        st.doc
            .edit_merged(tr("Background color"), emb_model::MergeKey::new("background"), |d| {
                d.background = Some(rgb)
            });
    }
    let enabled = bg.is_some();
    let r = small(ui, Icon::Glyph(ph::X), tr("No Background"), false, enabled, false)
        .on_hover_text(tr("No fabric color: the canvas shows the design on white."));
    if r.clicked() {
        st.doc.edit(tr("Background color"), |d| d.background = None);
    }
}

/// The machine and hoop of the design.
/// The zoom factor list.
pub(super) fn zoom_factor(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    ui.vertical(|ui| {
        ui.label(tr("Zoom"));
        crate::corners::zoom_combo(ui, st);
        ui.horizontal(|ui| {
            for (cmd, text) in [(Cmd::ZoomTwice(false), "½×"), (Cmd::ZoomTwice(true), "2×")] {
                let (_, _, tip) = cmd.meta();
                if ui.small_button(text).on_hover_text(tip).clicked() && st.can(cmd, s) {
                    st.run(cmd, &ui.ctx().clone());
                }
            }
        });
    });
}

/// The fabric of the design, and a list to apply a style to the selection.
pub(super) fn style_picker(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    let styles: Vec<String> = st.doc.design().styles.iter().map(|x| x.name.clone()).collect();
    let mut apply = None;
    egui::Grid::new("style_picker")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            ui.label(tr("Fabric"));
            ui.label(st.fabric_name());
            ui.end_row();
            ui.label(tr("Style"));
            ui.add_enabled_ui(s.n > 0 && !styles.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("apply_style")
                    .selected_text(if styles.is_empty() {
                        tr("No styles")
                    } else {
                        tr("Apply to selection")
                    })
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in styles.iter().enumerate() {
                            if ui.selectable_label(false, name).clicked() {
                                apply = Some(i);
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr("Give the selected objects the settings of a style."));
            });
            ui.end_row();
        });
    if let Some(i) = apply {
        st.apply_style(i);
    }
}

pub(super) fn machine_hoop(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    let design = st.doc.design();
    let machine = design.machine.clone();
    let hoop = design.hoop.selection.clone();
    let machine_name = machine
        .as_deref()
        .and_then(|id| st.machines.get(id))
        .map(|m| m.name.clone())
        .unwrap_or_else(|| tr("None").into());
    let hoop_name = match &hoop {
        HoopSelection::OpenField => tr("Open field").to_string(),
        HoopSelection::Preset { id } => st.hoops.get(id).map(|h| h.name.clone()).unwrap_or_else(|| id.clone()),
    };
    let (mut new_machine, mut new_hoop) = (None, None);
    egui::Grid::new("machine_hoop")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            ui.label(tr("Machine"));
            egui::ComboBox::from_id_salt("machine")
                .selected_text(machine_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for m in &st.machines.machines {
                        if ui
                            .selectable_label(machine.as_deref() == Some(m.id.as_str()), &m.name)
                            .clicked()
                        {
                            new_machine = Some(m.id.clone());
                        }
                    }
                });
            ui.end_row();
            ui.label(tr("Hoop"));
            egui::ComboBox::from_id_salt("hoop")
                .selected_text(hoop_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(hoop == HoopSelection::OpenField, tr("Open field"))
                        .clicked()
                    {
                        new_hoop = Some(HoopSelection::OpenField);
                    }
                    // My hoops first.
                    let mine = &st.settings.my_hoops;
                    let (first, rest): (Vec<_>, Vec<_>) = st.hoops.hoops.iter().partition(|h| mine.contains(&h.id));
                    for (k, list) in [first, rest].into_iter().enumerate() {
                        if list.is_empty() {
                            continue;
                        }
                        if !mine.is_empty() {
                            ui.separator();
                            ui.weak(if k == 0 { tr("My Hoops") } else { tr("Other hoops") });
                        }
                        for h in list {
                            let sel = HoopSelection::Preset { id: h.id.clone() };
                            let mut text = format!("{}  ({} × {} mm)", h.name, h.width_mm, h.height_mm);
                            if !st.hoop_fits_machine(&h.id) {
                                text.push_str(&format!("  ⚠ {}", tr("not for this machine")));
                            }
                            if ui.selectable_label(hoop == sel, text).clicked() {
                                new_hoop = Some(sel);
                            }
                        }
                    }
                });
            ui.end_row();
        });
    if let Some(m) = new_machine {
        st.doc.edit(tr("Change machine"), |d| d.machine = Some(m));
    }
    if let Some(sel) = new_hoop {
        st.doc.edit(tr("Change hoop"), |d| {
            // In a hoop, the hoop center goes to the center of the design. Object coordinates do not change.
            if d.hoop.selection == HoopSelection::OpenField
                && let Some(e) = d.extents()
            {
                d.hoop.center = e.center();
            }
            d.hoop.selection = sel;
        });
        st.fit_requested = true;
    }
}
