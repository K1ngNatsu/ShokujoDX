//! The ribbon: tabs at the top of the window with the commands in panels,
//! as in CAD programs. See ADR 0010.
//!
//! - The File button opens the application menu (new, open, save, import,
//!   export). Next to it are New, Open, Save, Undo and Redo.
//! - Each tab has panels. A panel has large buttons (icon above the name)
//!   and columns of up to three small buttons (icon left of the name). The
//!   panel name is below its buttons.
//! - Double-click a tab, or click the arrow at the right, to hide the panels
//!   and get more space for the design. Click a tab to show them again.
//! - Commands that need a selection are dimmed when there is none.
//!
//! The files: `commands` (the commands and their names), `run` (what
//! they can do and do), `tabs` (the panels of each tab), `widgets` (the
//! buttons and panels), `controls` (the other controls of the panels)
//! and the drawing of the ribbon here.

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Vec2, vec2};
use egui_phosphor::regular as ph;
use emb_model::{HoopSelection, ObjectKind, RunPass, Step};
use serde::{Deserialize, Serialize};

use crate::actions::Align;
use crate::app::AppState;
use crate::draw::Tool;
use crate::file::Pending;
use crate::functions::Func;
use crate::i18n::tr;
use crate::icons::{self, Drawing, Icon};
use crate::vectors::ArtworkImport;
use crate::viewing::{Travel, ViewFilter};

mod commands;
mod controls;
mod run;
mod tabs;
#[cfg(test)]
mod tests;
mod widgets;

pub use commands::*;
use controls::*;
pub use run::*;
pub use tabs::*;
pub use widgets::*;

/// The tabs of the ribbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RibbonTab {
    #[default]
    Digitize,
    /// Lettering and personalization (monograms, team names).
    Lettering,
    Artwork,
    /// Specialty decoration (P9): cross stitch and rhinestones.
    Specialty,
    Edit,
    Arrange,
    Color,
    View,
    Design,
    Hooping,
    /// Help, tutorials and the start page (P10 Milestone 4).
    Help,
    /// Sketch mode (P14 Milestone 2): it shows only while a sketch is open.
    Sketch,
}

impl RibbonTab {
    pub const ALL: [RibbonTab; 12] = [
        RibbonTab::Digitize,
        RibbonTab::Lettering,
        RibbonTab::Artwork,
        RibbonTab::Specialty,
        RibbonTab::Edit,
        RibbonTab::Arrange,
        RibbonTab::Color,
        RibbonTab::View,
        RibbonTab::Design,
        RibbonTab::Hooping,
        RibbonTab::Help,
        RibbonTab::Sketch,
    ];

    /// The tabs that show: all, or (simple mode) the tabs of the simple
    /// tasks. The Sketch tab shows only in sketch mode (`show`).
    pub fn visible(simple: bool) -> &'static [RibbonTab] {
        const NORMAL: [RibbonTab; 11] = [
            RibbonTab::Digitize,
            RibbonTab::Lettering,
            RibbonTab::Artwork,
            RibbonTab::Specialty,
            RibbonTab::Edit,
            RibbonTab::Arrange,
            RibbonTab::Color,
            RibbonTab::View,
            RibbonTab::Design,
            RibbonTab::Hooping,
            RibbonTab::Help,
        ];
        const SIMPLE: [RibbonTab; 6] = [
            RibbonTab::Digitize,
            RibbonTab::Lettering,
            RibbonTab::Edit,
            RibbonTab::View,
            RibbonTab::Design,
            RibbonTab::Help,
        ];
        if simple { &SIMPLE } else { &NORMAL }
    }

    /// The panels of the tab in simple mode (large buttons only), or all.
    fn panels_in(self, simple: bool) -> &'static [Panel] {
        if !simple {
            return self.panels();
        }
        match self {
            RibbonTab::Digitize => S_DIGITIZE,
            RibbonTab::Lettering => S_LETTERING,
            RibbonTab::Edit => S_EDIT,
            RibbonTab::View => S_VIEW,
            RibbonTab::Design => S_DESIGN,
            _ => self.panels(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RibbonTab::Digitize => tr("Digitize"),
            RibbonTab::Lettering => tr("Lettering"),
            RibbonTab::Artwork => tr("Artwork"),
            RibbonTab::Specialty => tr("Specialty"),
            RibbonTab::Edit => tr("Edit"),
            RibbonTab::Arrange => tr("Arrange"),
            RibbonTab::Color => tr("Color"),
            RibbonTab::View => tr("View"),
            RibbonTab::Design => tr("Design"),
            RibbonTab::Hooping => tr("Hooping"),
            RibbonTab::Help => tr("Help"),
            RibbonTab::Sketch => tr("Sketch"),
        }
    }

    fn panels(self) -> &'static [Panel] {
        match self {
            RibbonTab::Digitize => DIGITIZE,
            RibbonTab::Lettering => LETTERING,
            RibbonTab::Artwork => ARTWORK,
            RibbonTab::Specialty => SPECIALTY,
            RibbonTab::Edit => EDIT,
            RibbonTab::Arrange => ARRANGE,
            RibbonTab::Color => COLOR,
            RibbonTab::View => VIEW,
            RibbonTab::Design => DESIGN,
            RibbonTab::Hooping => HOOPING,
            RibbonTab::Help => HELP,
            RibbonTab::Sketch => SKETCH,
        }
    }
}

// ---- The ribbon ----

/// File > Recent designs.
fn recent_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let recent = st.settings.recent.clone();
    ui.add_enabled_ui(!recent.is_empty(), |ui| {
        ui.menu_button(
            format!("{}  {}", ph::CLOCK_COUNTER_CLOCKWISE, tr("Recent designs")),
            |ui| {
                for path in &recent {
                    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
                    if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                        st.open_recent(path);
                        ui.close();
                    }
                }
            },
        );
    });
}

/// The File menu.
fn file_menu(ui: &mut egui::Ui, st: &mut AppState) {
    let ctx = ui.ctx().clone();
    let s = SelInfo::of(st);
    let item = |ui: &mut egui::Ui, st: &mut AppState, cmd: Cmd, text: &str| {
        let (icon, _, tip) = cmd.meta();
        let glyph = match icon {
            Icon::Glyph(g) => g,
            Icon::Drawn(_) => "",
        };
        let b = egui::Button::new(format!("{glyph}  {text}")).shortcut_text(cmd.keys(&ctx).unwrap_or_default());
        if ui.add_enabled(st.can(cmd, &s), b).on_hover_text(tip).clicked() {
            st.run(cmd, &ctx);
        }
    };
    item(ui, st, Cmd::New, tr("New"));
    item(ui, st, Cmd::NewFromTemplate, tr("New from Template…"));
    item(ui, st, Cmd::Open, tr("Open…"));
    recent_menu(ui, st);
    item(ui, st, Cmd::Library, tr("Design Library"));
    item(ui, st, Cmd::Save, tr("Save"));
    item(ui, st, Cmd::SaveAs, tr("Save As…"));
    item(ui, st, Cmd::SaveAsTemplate, tr("Save as Template…"));
    item(ui, st, Cmd::Close, tr("Close"));
    item(ui, st, Cmd::CloseAll, tr("Close All"));
    ui.separator();
    item(ui, st, Cmd::LoadPicture, tr("Load Backdrop Picture…"));
    item(ui, st, Cmd::ImportSvg, tr("Import Artwork (SVG)…"));
    item(ui, st, Cmd::ImportSvgLines, tr("Import Artwork as Lines (SVG)…"));
    item(ui, st, Cmd::ImportSvgVectors, tr("Import Artwork as Vectors (SVG)…"));
    item(ui, st, Cmd::InsertMachineFile, tr("Insert Machine File…"));
    item(ui, st, Cmd::Export, tr("Export Machine File…"));
    item(ui, st, Cmd::SendToUsb, tr("Send to USB"));
    item(ui, st, Cmd::SendToUsbChoose, tr("Send to USB Stick…"));
    item(ui, st, Cmd::SendByEmail, tr("Send by Email…"));
    item(ui, st, Cmd::ExportSvg, tr("Export Design Picture (SVG)…"));
    item(ui, st, Cmd::ExportOutlines, tr("Export Outlines (SVG, EMF)…"));
    item(ui, st, Cmd::CutterFile, tr("Appliqué Cutter File (SVG, DXF)…"));
    item(ui, st, Cmd::OutputDesign, tr("Output Design…"));
    item(ui, st, Cmd::PrintPreview, tr("Print Preview…"));
    item(ui, st, Cmd::BatchOutput, tr("Batch Output…"));
    ui.separator();
    item(ui, st, Cmd::Assets, tr("My Assets…"));
    item(ui, st, Cmd::Options, tr("Options…"));
    item(ui, st, Cmd::About, tr("About…"));
    ui.separator();
    if ui.button(crate::i18n::fill("{}  Exit", &[&(ph::SIGN_OUT)])).clicked() {
        st.request(Pending::Quit);
    }
}

/// Shows the ribbon: the tab row and the panels of the current tab.
pub fn show(ui: &mut egui::Ui, st: &mut AppState) {
    #[cfg(test)]
    RECTS.with(|r| r.borrow_mut().clear());
    let s = SelInfo::of(st);
    let ctx = ui.ctx().clone();
    // The command under the pointer, for F1 (set again while the buttons draw).
    st.hover_cmd = None;
    let mut settings = st.settings.clone();
    let (tab_before, collapsed_before) = (settings.ribbon_tab, settings.ribbon_collapsed);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let accent = ui.visuals().selection.bg_fill;
        let file = egui::Button::new(RichText::new(tr("File")).strong().color(Color32::WHITE))
            .fill(accent)
            .min_size(vec2(52.0, SMALL_H));
        let resp = ui.add(file);
        note_rect("File", resp.rect);
        egui::Popup::menu(&resp).show(|ui| file_menu(ui, st));
        ui.add_space(6.0);
        for cmd in QUICK {
            let (icon, label, _) = cmd.meta();
            let r = icon_button(ui, icon, label, st.can(cmd, &s));
            if tooltip(r, &ctx, cmd).clicked() {
                st.run(cmd, &ctx);
            }
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        let sketching = st.sketch.is_some();
        if !sketching && settings.ribbon_tab == RibbonTab::Sketch {
            settings.ribbon_tab = RibbonTab::Digitize;
        }
        let mut tabs = RibbonTab::visible(settings.simple_mode).to_vec();
        if sketching {
            tabs.push(RibbonTab::Sketch);
        }
        for tab in tabs {
            let active = settings.ribbon_tab == tab && !settings.ribbon_collapsed;
            let text = RichText::new(tab.label()).size(13.5);
            let text = if active { text.strong() } else { text };
            let r = ui.add(egui::Button::selectable(active, text).min_size(vec2(64.0, SMALL_H)));
            note_rect(tab.label(), r.rect);
            if active {
                let y = r.rect.bottom() - 1.0;
                let stroke = Stroke::new(2.0, ui.visuals().selection.stroke.color);
                ui.painter().hline(r.rect.x_range().shrink(6.0), y, stroke);
            }
            if r.double_clicked() {
                settings.ribbon_tab = tab;
                settings.ribbon_collapsed = !settings.ribbon_collapsed;
            } else if r.clicked() {
                settings.ribbon_tab = tab;
                settings.ribbon_collapsed = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (glyph, tip) = if settings.ribbon_collapsed {
                (ph::CARET_DOWN, tr("Show the ribbon"))
            } else {
                (ph::CARET_UP, tr("Hide the ribbon (or double-click a tab)"))
            };
            if icon_button(ui, Icon::Glyph(glyph), tr("Collapse"), true)
                .on_hover_text(tip)
                .clicked()
            {
                settings.ribbon_collapsed = !settings.ribbon_collapsed;
            }
        });
    });
    if !settings.ribbon_collapsed {
        ui.add_space(2.0);
        // The width of the window, outside the scroll area (inside it the
        // width has no end).
        let width = ui.available_width();
        egui::ScrollArea::horizontal()
            .id_salt("ribbon_panels")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                let tab = settings.ribbon_tab;
                let folded = folded_panels(st, tab, width);
                let _row = ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (i, p) in tab.panels_in(settings.simple_mode).iter().enumerate() {
                        let x0 = ui.cursor().min.x;
                        if folded[i] {
                            folded_panel(ui, st, &s, p);
                        } else {
                            panel(ui, st, &s, p);
                        }
                        let w = ui.cursor().min.x - x0;
                        if st.ribbon_widths.insert((tab, i, folded[i]), w) != Some(w) {
                            ui.ctx().request_repaint();
                        }
                    }
                });
                #[cfg(test)]
                PANELS_W.with(|w| w.set(_row.response.rect.width()));
            });
    }
    // The ribbon state is a user setting; the folded ribbon stays after a
    // restart, the tab does not (the program starts on Digitize).
    // Only a change of the tab here: a command of the ribbon (for example
    // Start Sketch) can change the tab too.
    let mut new = st.settings.clone();
    if settings.ribbon_tab != tab_before {
        new.ribbon_tab = settings.ribbon_tab;
    }
    if settings.ribbon_collapsed != collapsed_before {
        new.ribbon_collapsed = settings.ribbon_collapsed;
    }
    st.set_settings(new);
}
