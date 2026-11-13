//! The buttons and panels of the ribbon, and the help that is made from
//! the panels.

use super::*;

// ---- Widgets ----

pub(super) const LARGE: Vec2 = vec2(60.0, 68.0);
pub(super) const SMALL_H: f32 = 22.0;
pub(super) const ROW_GAP: f32 = 1.0;
pub(super) const CONTENT_H: f32 = 3.0 * SMALL_H + 2.0 * ROW_GAP;
pub(super) const TITLE_H: f32 = 15.0;

#[cfg(test)]
thread_local! {
    /// Button rectangles by name, so tests can click them.
    pub static RECTS: std::cell::RefCell<Vec<(String, Rect)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// The width of the panels of the tab that showed last.
    pub static PANELS_W: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

pub(super) fn note_rect(_label: &str, _rect: Rect) {
    #[cfg(test)]
    RECTS.with(|r| r.borrow_mut().push((_label.to_string(), _rect)));
}

/// Paints the background of a button for its state.
pub(super) fn frame(ui: &egui::Ui, rect: Rect, resp: &egui::Response, on: bool, enabled: bool) {
    let v = ui.visuals();
    let fill = if on {
        v.selection.bg_fill.gamma_multiply(0.45)
    } else if enabled && resp.hovered() {
        v.widgets.hovered.weak_bg_fill
    } else {
        return;
    };
    let stroke = if on {
        Stroke::new(1.0, v.selection.stroke.color.gamma_multiply(0.6))
    } else {
        Stroke::NONE
    };
    ui.painter().rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
}

pub(super) fn colors(ui: &egui::Ui, enabled: bool) -> (Color32, Color32) {
    let v = ui.visuals();
    let fg = if enabled {
        v.text_color()
    } else {
        v.weak_text_color().gamma_multiply(0.6)
    };
    (fg, v.panel_fill)
}

pub(super) fn tooltip(resp: egui::Response, ctx: &egui::Context, cmd: Cmd) -> egui::Response {
    let (_, name, tip) = cmd.meta();
    let text = match cmd.keys(ctx) {
        Some(k) => format!("{name} ({k})\n{tip}\nF1: help"),
        None => format!("{name}\n{tip}\nF1: help"),
    };
    resp.on_hover_text(&text).on_disabled_hover_text(text)
}

/// The id of the first ribbon button that can take the keyboard focus (F6).
pub(super) const FIRST_BUTTON: &str = "ribbon_first_button";

/// Keeps the id of the first button of the frame that can take the focus.
pub(super) fn note_focus(ui: &egui::Ui, resp: &egui::Response) {
    if resp.sense.senses_click() {
        let key = egui::Id::new(FIRST_BUTTON);
        let pass = ui.ctx().cumulative_pass_nr();
        ui.ctx().data_mut(|d| {
            let have = d.get_temp::<(u64, egui::Id)>(key);
            if have.is_none_or(|(p, _)| p != pass) {
                d.insert_temp(key, (pass, resp.id));
            }
        });
    }
}

/// F6: the keyboard focus goes to the first ribbon button. Then Tab and
/// Shift+Tab go from button to button, and Enter or Space runs one.
pub fn focus_ribbon(ctx: &egui::Context) -> bool {
    let id = ctx.data(|d| d.get_temp::<(u64, egui::Id)>(egui::Id::new(FIRST_BUTTON)));
    match id {
        Some((_, id)) => {
            ctx.memory_mut(|m| m.request_focus(id));
            true
        }
        None => false,
    }
}

pub(super) fn sense(enabled: bool) -> Sense {
    if enabled { Sense::click() } else { Sense::hover() }
}

/// The widest that a large button gets, so that its name has two lines at
/// most.
pub(super) const LARGE_MAX_W: f32 = 120.0;

/// The width of a large button and the font size of its name: two lines
/// at most (the button gets wider for a long name, also of a folded panel),
/// and no word wider than the button (a long word of a translation makes
/// the text smaller), so no word breaks in its middle.
pub(super) fn large_fit(ui: &egui::Ui, label: &str) -> (f32, f32) {
    let painter = ui.painter();
    let color = ui.visuals().text_color();
    let rows = |size: f32, w: f32| {
        painter
            .layout(label.to_string(), FontId::proportional(size), color, w - 4.0)
            .rows
            .len()
    };
    let widest = |size: f32| {
        label
            .split_whitespace()
            .map(|w| {
                painter
                    .layout_no_wrap(w.to_string(), FontId::proportional(size), color)
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
    };
    let mut size = 11.5;
    while size > 8.5 && widest(size) > LARGE.x - 4.0 {
        size -= 0.5;
    }
    let mut w = LARGE.x.max((widest(size) + 4.0).min(LARGE_MAX_W));
    while w < LARGE_MAX_W && rows(size, w) > 2 {
        w += 4.0;
    }
    (w.ceil(), size)
}

/// A large button: the icon above the name (two lines at most).
pub(super) fn large(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool, enabled: bool) -> egui::Response {
    let (w, size) = large_fit(ui, label);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, LARGE.y), sense(enabled));
    let resp = crate::a11y::named_toggle(resp, label, on);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let (fg, bg) = colors(ui, enabled);
    let icon_rect = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 20.0), Vec2::splat(28.0));
    icons::paint(ui.painter(), icon_rect, icon, fg, bg);
    let galley = ui
        .painter()
        .layout(label.to_string(), FontId::proportional(size), fg, w - 4.0);
    let pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 38.0);
    ui.painter().galley(pos, galley, fg);
    resp
}

/// A small button: the icon left of the name. `menu` adds a down arrow.
pub(super) fn small(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool, enabled: bool, menu: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let (fg, bg) = colors(ui, enabled);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let arrow = if menu { 12.0 } else { 0.0 };
    let size = vec2(4.0 + 17.0 + 5.0 + galley.size().x + arrow + 6.0, SMALL_H);
    let (rect, resp) = ui.allocate_exact_size(size, sense(enabled));
    let resp = crate::a11y::named_toggle(resp, label, on);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let icon_rect = Rect::from_center_size(egui::pos2(rect.left() + 12.5, rect.center().y), Vec2::splat(16.0));
    icons::paint(ui.painter(), icon_rect, icon, fg, bg);
    let text_pos = egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, fg);
    if menu {
        ui.painter().text(
            egui::pos2(rect.right() - 9.0, rect.center().y),
            Align2::CENTER_CENTER,
            ph::CARET_DOWN,
            FontId::proportional(10.0),
            fg,
        );
    }
    resp
}

/// A button with only an icon (the quick-access buttons).
pub(super) fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SMALL_H), sense(enabled));
    let resp = crate::a11y::named(resp, label);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, false, enabled);
    let (fg, bg) = colors(ui, enabled);
    icons::paint(ui.painter(), rect.shrink(3.0), icon, fg, bg);
    resp
}

pub(super) fn cmd_button(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, cmd: Cmd, big: bool) {
    let (icon, label, _) = cmd.meta();
    let enabled = st.can(cmd, s);
    let on = st.is_on(cmd);
    let resp = if big {
        large(ui, icon, label, on, enabled)
    } else {
        small(ui, icon, label, on, enabled, false)
    };
    let ctx = ui.ctx().clone();
    let resp = tooltip(resp, &ctx, cmd);
    if resp.hovered() {
        st.hover_cmd = Some(cmd);
    }
    if resp.clicked() {
        st.run(cmd, &ctx);
    }
}

/// A small button that opens a menu.
pub(super) fn dropdown(
    ui: &mut egui::Ui,
    icon: &'static str,
    label: &str,
    tip: &str,
    enabled: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let resp = small(ui, Icon::Glyph(icon), label, false, enabled, true).on_hover_text(tip);
    egui::Popup::menu(&resp).show(content);
}

/// A column of controls with the height of three small buttons.
pub(super) fn column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        add(ui);
    });
}

/// The panels of `tab` that fold into one button, so that the panels fit
/// in `width` (the panels on the right fold first). Before the widths of
/// all panels are known, none folds.
pub(super) fn folded_panels(st: &AppState, tab: RibbonTab, width: f32) -> Vec<bool> {
    let n = tab.panels_in(st.settings.simple_mode).len();
    let w = |i: usize, f: bool| st.ribbon_widths.get(&(tab, i, f)).copied();
    let mut folded = vec![false; n];
    let Some(mut total) = (0..n).map(|i| w(i, false)).sum::<Option<f32>>() else {
        return folded;
    };
    for i in (0..n).rev() {
        if total <= width {
            break;
        }
        total -= w(i, false).unwrap_or(0.0) - w(i, true).unwrap_or(FOLDED_W);
        folded[i] = true;
    }
    folded
}

/// The width of a folded panel before it is measured.
pub(super) const FOLDED_W: f32 = LARGE.x + 13.0;

/// A command as the help shows it.
pub struct HelpCommand {
    pub label: &'static str,
    pub tip: &'static str,
    pub keys: Option<String>,
}

/// A ribbon panel as the help shows it.
pub struct HelpPanel {
    pub title: &'static str,
    pub commands: Vec<HelpCommand>,
}

/// The panels and commands of a ribbon tab, for its help topic.
pub fn help_panels(tab: RibbonTab, ctx: &egui::Context) -> Vec<HelpPanel> {
    tab.panels()
        .iter()
        .map(|p| HelpPanel {
            title: tr(p.title),
            commands: p
                .items
                .iter()
                .flat_map(|item| match item {
                    Large(c) => std::slice::from_ref(c),
                    Small(cs) | Custom(_, cs) => cs,
                })
                .map(|&cmd| {
                    let (_, label, tip) = cmd.meta();
                    HelpCommand {
                        label,
                        tip,
                        keys: cmd.keys(ctx),
                    }
                })
                .collect(),
        })
        .collect()
}

/// The ribbon tab that has the command.
pub fn tab_of(cmd: Cmd) -> Option<RibbonTab> {
    RibbonTab::ALL.into_iter().find(|t| {
        t.panels().iter().any(|p| {
            p.items.iter().any(|item| match item {
                Large(c) => *c == cmd,
                Small(cs) | Custom(_, cs) => cs.contains(&cmd),
            })
        })
    })
}

/// The icon of a panel: the icon of its first command.
pub(super) fn panel_icon(p: &Panel) -> Icon {
    p.items
        .iter()
        .find_map(|item| match item {
            Large(c) => Some(c.meta().0),
            Small(cs) => cs.first().map(|c| c.meta().0),
            Custom(..) => None,
        })
        .unwrap_or(Icon::Glyph(ph::DOTS_THREE))
}

/// The label of a folded panel: its title and a caret.
pub(super) fn folded_label(p: &Panel) -> String {
    format!("{} {}", tr(p.title), ph::CARET_DOWN)
}

/// A panel that does not fit: one large button with the panel title; a
/// click shows the panel under it.
pub(super) fn folded_panel(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, p: &Panel) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let resp = ui
            .horizontal_top(|ui| {
                ui.set_min_height(LARGE.y.max(CONTENT_H));
                large(ui, panel_icon(p), &folded_label(p), false, true)
            })
            .inner
            .on_hover_text(crate::i18n::fill(
                "The {} tools (the window is too narrow to show them all)",
                &[&tr(p.title)],
            ));
        egui::Popup::menu(&resp)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .layout(egui::Layout::left_to_right(egui::Align::Min))
            .show(|ui| panel(ui, st, s, p));
        let color = ui.visuals().weak_text_color();
        let galley = ui
            .painter()
            .layout_no_wrap(tr(p.title).to_string(), FontId::proportional(11.0), color);
        let (r, _) = ui.allocate_exact_size(vec2(LARGE.x.max(galley.size().x), TITLE_H), Sense::hover());
        ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
    });
    let (r, _) = ui.allocate_exact_size(vec2(11.0, LARGE.y + TITLE_H), Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().vline(r.center().x, r.y_range().shrink(3.0), stroke);
}

pub(super) fn panel(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, p: &Panel) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let content = ui
            .horizontal_top(|ui| {
                ui.set_min_height(LARGE.y.max(CONTENT_H));
                for item in p.items {
                    match item {
                        Large(cmd) => cmd_button(ui, st, s, *cmd, true),
                        Small(cmds) => column(ui, |ui| {
                            for cmd in *cmds {
                                cmd_button(ui, st, s, *cmd, false);
                            }
                        }),
                        Custom(add, _) => column(ui, |ui| add(ui, st, s)),
                    }
                }
            })
            .response
            .rect;
        let color = ui.visuals().weak_text_color();
        let galley = ui
            .painter()
            .layout_no_wrap(tr(p.title).to_string(), FontId::proportional(11.0), color);
        let (r, _) = ui.allocate_exact_size(vec2(content.width().max(galley.size().x), TITLE_H), Sense::hover());
        ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
    });
    // A line between the panels.
    let (r, _) = ui.allocate_exact_size(vec2(11.0, LARGE.y + TITLE_H), Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().vline(r.center().x, r.y_range().shrink(3.0), stroke);
}
