//! The About window: the version and build of the program, the license,
//! and the folders of the settings, backups and crash reports (for test
//! users who send feedback).

use eframe::egui;

use crate::app::AppState;
use crate::i18n::tr;

/// The version of the program.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The git commit that the program is built from (`+changes` if files had
/// changes that were not in a commit), from `build.rs`.
pub const BUILD: &str = env!("EMB_BUILD");
/// The date of that commit.
pub const BUILD_DATE: &str = env!("EMB_BUILD_DATE");

/// One line that says which program this is, for feedback and crash reports.
pub fn build_text() -> String {
    let date = if BUILD_DATE.is_empty() {
        String::new()
    } else {
        format!(", {BUILD_DATE}")
    };
    format!(
        "ShokujoDX {VERSION} (build {BUILD}{date}), {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// The folder of the settings, backups and crash reports.
pub fn user_folder() -> Option<std::path::PathBuf> {
    crate::workspace::settings_path()?
        .parent()
        .map(std::path::Path::to_path_buf)
}

/// The About window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.about_window {
        return;
    }
    let mut open = true;
    egui::Window::new(tr("About"))
        .id(egui::Id::new("About"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading(tr("ShokujoDX"));
            ui.label(crate::i18n::fill("Version {}, build {}", &[&VERSION, &BUILD]));
            if !BUILD_DATE.is_empty() {
                ui.weak(crate::i18n::fill("Built from the code of {}", &[&BUILD_DATE]));
            }
            ui.add_space(4.0);
            ui.label(tr(
                "Free software under the GNU General Public License, version 3 or later.",
            ));
            ui.label(tr(
                "ShokujoDX is not affiliated with or endorsed by any embroidery software company. Other product names are the trademarks of their owners.",
            ));
            ui.label(tr(
                "This is a test build. Save your work often, and send your notes with the feedback form.",
            ));
            ui.add_space(4.0);
            if let Some(dir) = user_folder() {
                ui.label(tr("Settings, backups and crash reports are in:"));
                ui.monospace(dir.display().to_string());
            }
            ui.add_space(4.0);
            if ui
                .button(tr("Copy build information"))
                .on_hover_text(tr("Put the version, build and system on the clipboard, for a report."))
                .clicked()
            {
                ui.ctx().copy_text(build_text());
                st.message = tr("Build information copied.").into();
            }
        });
    st.about_window = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_text_names_the_version_and_the_build() {
        let t = build_text();
        assert!(t.starts_with(&format!("ShokujoDX {VERSION} (build ")), "{t}");
        assert!(t.contains(BUILD));
        assert!(t.contains(std::env::consts::OS));
    }
}
