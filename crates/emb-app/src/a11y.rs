//! Screen reader names (P10 Milestone 7). egui gives the screen reader
//! (AccessKit) the text of each widget; a button with only an icon, a
//! button that the program paints itself and the canvas get their names
//! here.

use eframe::egui;

/// Gives the response the name `name` for the screen reader, as a button.
pub fn named(resp: egui::Response, name: &str) -> egui::Response {
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
    resp
}

/// As [`named`], for a button that is on or off.
pub fn named_toggle(resp: egui::Response, name: &str, on: bool) -> egui::Response {
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, name));
    resp
}

/// A response with a name for the screen reader.
pub trait Named {
    /// The screen reader name of an icon button (also its tooltip, usually).
    fn named(self, name: &str) -> egui::Response;
}

impl Named for egui::Response {
    fn named(self, name: &str) -> egui::Response {
        named(self, name)
    }
}
