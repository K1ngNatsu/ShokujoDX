//! My assets (P10 Milestone 6): one window for the user's own assets, to
//! import, export and share them.
//!
//! - Each kind of asset is a folder next to the settings file: templates,
//!   embroidery fonts (also the alphabets), motifs, thread charts and
//!   clipart; the user's hoops are one file (`hoops.toml`).
//! - Import copies files into the folder of their kind (hoops: the new
//!   hoops go into the hoop file) and makes the program read them again.
//! - Export writes the files of a kind in a zip file.
//! - A package is one zip file with a folder for each kind and
//!   `favorites.json` (the favorite styles and my stones), to move all the
//!   assets to another computer or to another user. Import a package puts
//!   each file into its folder (a file with the same name is replaced) and
//!   adds the favorites and the stones that are not there yet.

use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// A kind of asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Templates,
    Fonts,
    Motifs,
    Charts,
    Clipart,
    Hoops,
}

impl AssetKind {
    pub const ALL: [AssetKind; 6] = [
        AssetKind::Templates,
        AssetKind::Fonts,
        AssetKind::Motifs,
        AssetKind::Charts,
        AssetKind::Clipart,
        AssetKind::Hoops,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Templates => tr("Templates"),
            AssetKind::Fonts => tr("Embroidery fonts and alphabets"),
            AssetKind::Motifs => tr("Motifs"),
            AssetKind::Charts => tr("Thread charts"),
            AssetKind::Clipart => tr("Clipart"),
            AssetKind::Hoops => tr("My Hoops"),
        }
    }

    /// The folder of the kind in a package.
    pub fn package_folder(self) -> &'static str {
        match self {
            AssetKind::Templates => "templates",
            AssetKind::Fonts => "fonts",
            AssetKind::Motifs => "motifs",
            AssetKind::Charts => "threads",
            AssetKind::Clipart => "clipart",
            AssetKind::Hoops => "hoops",
        }
    }

    /// The file extensions of the kind.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            AssetKind::Templates => &[emb_model::template::FILE_EXTENSION],
            AssetKind::Fonts => &[emb_letter::embfont::FILE_EXTENSION],
            AssetKind::Motifs => &[emb_model::motif::FILE_EXTENSION],
            AssetKind::Charts => &["gpl", "csv"],
            AssetKind::Clipart => &[emb_model::project::FILE_EXTENSION],
            AssetKind::Hoops => &["toml"],
        }
    }

    /// The folder of the kind (for the hoops: the folder of the hoop file).
    pub fn dir(self) -> Option<PathBuf> {
        match self {
            AssetKind::Templates => crate::styles::templates_dir(),
            AssetKind::Fonts => crate::lettering::user_fonts_dir(),
            AssetKind::Motifs => crate::motifs::motifs_dir(),
            AssetKind::Charts => crate::threads::user_chart_dir(),
            AssetKind::Clipart => crate::clipart::clipart_dir(),
            AssetKind::Hoops => crate::hoops::user_hoops_path().and_then(|p| p.parent().map(Path::to_path_buf)),
        }
    }

    fn takes(self, path: &Path) -> bool {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        self.extensions().contains(&ext.as_str())
    }
}

/// The files of a kind in a folder (for the hoops: the hoop file only).
pub fn files_in(kind: AssetKind, dir: &Path) -> Vec<PathBuf> {
    if kind == AssetKind::Hoops {
        let p = dir.join("hoops.toml");
        return if p.is_file() { vec![p] } else { Vec::new() };
    }
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && kind.takes(p))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The favorites of a package.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageFavorites {
    pub favorite_styles: Vec<emb_model::Style>,
    pub my_stones: Vec<emb_model::Stone>,
}

/// The name of the favorites file in a package.
pub const FAVORITES_FILE: &str = "favorites.json";

/// The files of a package: each file of each kind in its folder, and the favorites.
pub fn package_files(dirs: &[(AssetKind, PathBuf)], favorites: &PackageFavorites) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (kind, dir) in dirs {
        for p in files_in(*kind, dir) {
            if let (Some(name), Ok(bytes)) = (p.file_name(), std::fs::read(&p)) {
                out.push((format!("{}/{}", kind.package_folder(), name.to_string_lossy()), bytes));
            }
        }
    }
    let fav = serde_json::to_vec_pretty(favorites).unwrap_or_default();
    out.push((FAVORITES_FILE.to_string(), fav));
    out
}

/// The window state.
#[derive(Debug, Clone, Default)]
pub struct AssetsWindow {
    pub open: bool,
}

impl AppState {
    /// Reads the assets of a kind again after a change of its folder.
    fn reload_assets(&mut self, kind: AssetKind) {
        match kind {
            AssetKind::Fonts => {
                if let Some(dir) = kind.dir() {
                    self.fonts.add_user_fonts(&dir);
                }
            }
            AssetKind::Charts => {
                self.charts = crate::threads::load_charts().0;
                self.chart_index = self.chart_index.min(self.charts.len().saturating_sub(1));
            }
            AssetKind::Clipart => self.scan_clipart(),
            // Templates and motifs are read when their windows show; hoops
            // go in when they are imported.
            AssetKind::Templates | AssetKind::Motifs | AssetKind::Hoops => {}
        }
    }

    /// Adds the hoops of a hoop file that the user does not have yet.
    /// Returns the number of new hoops.
    fn merge_hoops(&mut self, text: &str) -> Result<usize, String> {
        let other = emb_model::HoopCatalog::from_toml_str(text).map_err(|e| e.to_string())?;
        let mut user = self.user_hoops.clone();
        let mut n = 0;
        for h in other.hoops {
            if user.get(&h.id).is_none() && self.hoops.get(&h.id).is_none() {
                user.hoops.push(h);
                n += 1;
            }
        }
        if n > 0 {
            self.add_user_hoops(user);
            self.save_user_hoops();
        }
        Ok(n)
    }

    /// Puts files of a kind into its folder (`dir`). Returns how many went in.
    pub fn import_assets_into(
        &mut self,
        kind: AssetKind,
        dir: &Path,
        files: &[(String, Vec<u8>)],
    ) -> Result<usize, String> {
        let mut n = 0;
        for (name, bytes) in files {
            if !kind.takes(Path::new(name)) {
                continue;
            }
            if kind == AssetKind::Hoops {
                n += self.merge_hoops(&String::from_utf8_lossy(bytes))?;
                continue;
            }
            // Only the file name: never a path out of the folder.
            let Some(file) = Path::new(name).file_name() else {
                continue;
            };
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let to = dir.join(file);
            std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
            n += 1;
        }
        self.reload_assets(kind);
        Ok(n)
    }

    /// The favorites that go in a package.
    pub fn package_favorites(&self) -> PackageFavorites {
        PackageFavorites {
            favorite_styles: self.settings.favorite_styles.clone(),
            my_stones: self.settings.my_stones.clone(),
        }
    }

    /// Imports a package (see the module notes). Returns a summary.
    pub fn import_package(&mut self, bytes: &[u8], dirs: &[(AssetKind, PathBuf)]) -> Result<String, String> {
        let files = crate::share::unzip(bytes)?;
        let mut lines = Vec::new();
        for (kind, dir) in dirs {
            let prefix = format!("{}/", kind.package_folder());
            let mine: Vec<(String, Vec<u8>)> = files
                .iter()
                .filter_map(|(n, b)| n.strip_prefix(&prefix).map(|rest| (rest.to_string(), b.clone())))
                .collect();
            if !mine.is_empty() {
                let n = self.import_assets_into(*kind, dir, &mine)?;
                lines.push(format!("{}: {n}", kind.label()));
            }
        }
        if let Some((_, b)) = files.iter().find(|(n, _)| n == FAVORITES_FILE) {
            let fav: PackageFavorites = serde_json::from_slice(b).map_err(|e| format!("{FAVORITES_FILE}: {e}"))?;
            let mut s = self.settings.clone();
            let (mut styles, mut stones) = (0, 0);
            for f in fav.favorite_styles {
                if s.favorite_styles.len() < crate::viewtools::MAX_FAVORITES
                    && !s.favorite_styles.iter().any(|x| x.name == f.name)
                {
                    s.favorite_styles.push(f);
                    styles += 1;
                }
            }
            for st in fav.my_stones {
                if s.my_stones.len() < crate::bling::MAX_MY_STONES && !s.my_stones.contains(&st) {
                    s.my_stones.push(st);
                    stones += 1;
                }
            }
            self.set_settings(s);
            lines.push(format!("Favorite styles: {styles}, my stones: {stones}"));
        }
        Ok(lines.join("; "))
    }

    /// The folders of the kinds on this computer.
    fn asset_dirs() -> Vec<(AssetKind, PathBuf)> {
        AssetKind::ALL.iter().filter_map(|k| k.dir().map(|d| (*k, d))).collect()
    }

    fn import_assets_dialog(&mut self, kind: AssetKind) {
        let Some(dir) = kind.dir() else { return };
        let Some(paths) = rfd::FileDialog::new()
            .set_title(format!("Import {}", kind.label().to_lowercase()))
            .add_filter(kind.label(), kind.extensions())
            .pick_files()
        else {
            return;
        };
        let files: Vec<(String, Vec<u8>)> = paths
            .iter()
            .filter_map(|p| Some((p.file_name()?.to_string_lossy().into_owned(), std::fs::read(p).ok()?)))
            .collect();
        match self.import_assets_into(kind, &dir, &files) {
            Ok(n) => self.message = crate::i18n::fill("{}: {} imported", &[&(kind.label()), &n]),
            Err(e) => self.message = e,
        }
    }

    fn export_assets_dialog(&mut self, kinds: &[AssetKind], name: &str) {
        let dirs: Vec<(AssetKind, PathBuf)> = Self::asset_dirs()
            .into_iter()
            .filter(|(k, _)| kinds.contains(k))
            .collect();
        let fav = if kinds.len() > 1 {
            self.package_favorites()
        } else {
            PackageFavorites::default()
        };
        let mut files = package_files(&dirs, &fav);
        if kinds.len() == 1 {
            files.retain(|(n, _)| n != FAVORITES_FILE);
        }
        let Some(p) = rfd::FileDialog::new()
            .set_title("Export (zip file)")
            .add_filter("Zip file", &["zip"])
            .set_file_name(format!("{name}.zip"))
            .save_file()
        else {
            return;
        };
        match std::fs::write(&p, crate::share::zip(&files)) {
            Ok(()) => self.message = crate::i18n::fill("Wrote {} ({} files)", &[&(p.display()), &(files.len())]),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot write the file".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                })
            }
        }
    }

    fn import_package_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title("Import a package of assets")
            .add_filter("Zip file", &["zip"])
            .pick_file()
        else {
            return;
        };
        let r = std::fs::read(&p)
            .map_err(|e| e.to_string())
            .and_then(|b| self.import_package(&b, &Self::asset_dirs()));
        match r {
            Ok(summary) => {
                self.notice = Some(Notice {
                    title: "Package imported".into(),
                    lines: vec![summary],
                })
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot import the package".into(),
                    lines: vec![p.display().to_string(), e],
                })
            }
        }
    }
}

/// The My assets window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.assets_window.open {
        return;
    }
    let mut open = true;
    let mut import = None;
    let mut export = None;
    let (mut export_all, mut import_all) = (false, false);
    egui::Window::new(tr("My Assets"))
        .id(egui::Id::new("My Assets"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(tr(
                "Your own templates, fonts, motifs, thread charts, clipart and hoops, for all designs.",
            ));
            egui::Grid::new("assets")
                .num_columns(5)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    for kind in AssetKind::ALL {
                        let dir = kind.dir();
                        let n = dir.as_deref().map_or(0, |d| files_in(kind, d).len());
                        ui.label(kind.label());
                        ui.weak(if kind == AssetKind::Hoops {
                            format!("{} hoops", st.user_hoops.hoops.len())
                        } else {
                            format!("{n} files")
                        });
                        if ui.button(tr("Import…")).clicked() {
                            import = Some(kind);
                        }
                        if ui.add_enabled(n > 0, egui::Button::new(tr("Export…"))).clicked() {
                            export = Some(kind);
                        }
                        if ui
                            .add_enabled(dir.is_some(), egui::Button::new(tr("Open folder")))
                            .clicked()
                            && let Some(d) = &dir
                        {
                            let _ = std::fs::create_dir_all(d);
                            let _ = crate::share::open_with_system(d);
                        }
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.label(crate::i18n::fill(
                "Favorite styles: {}, my stones: {} (they go in a package too).",
                &[&(st.settings.favorite_styles.len()), &(st.settings.my_stones.len())],
            ));
            ui.horizontal(|ui| {
                export_all = ui
                    .button(tr("Export all as a package…"))
                    .on_hover_text(tr(
                        "One zip file with all your assets, for another computer or another user.",
                    ))
                    .clicked();
                import_all = ui
                    .button(tr("Import a package…"))
                    .on_hover_text(tr("Put the assets of a package into your folders."))
                    .clicked();
            });
        });
    if let Some(k) = import {
        st.import_assets_dialog(k);
    }
    if let Some(k) = export {
        st.export_assets_dialog(&[k], k.package_folder());
    }
    if export_all {
        st.export_assets_dialog(&AssetKind::ALL, "embroidery assets");
    }
    if import_all {
        st.import_package_dialog();
    }
    if !open {
        st.assets_window.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_package_goes_to_another_computer() {
        let base = std::env::temp_dir().join(format!("emb-assets-{}", std::process::id()));
        let from = base.join("a");
        let to = base.join("b");
        let dirs = |root: &Path| -> Vec<(AssetKind, PathBuf)> {
            AssetKind::ALL
                .iter()
                .map(|k| (*k, root.join(k.package_folder())))
                .collect()
        };
        // On the first computer: a motif, a chart and a hoop.
        let a = dirs(&from);
        for (k, d) in &a {
            std::fs::create_dir_all(d).unwrap();
            match k {
                AssetKind::Motifs => std::fs::write(d.join("leaf.embmotif"), "{}").unwrap(),
                AssetKind::Charts => std::fs::write(d.join("mine.gpl"), "GIMP Palette\n0 0 0 Black\n").unwrap(),
                AssetKind::Hoops => std::fs::write(
                    d.join("hoops.toml"),
                    "[[hoop]]\nid = \"my_150\"\nname = \"My 150\"\nshape = \"rectangle\"\nwidth_mm = 150.0\nheight_mm = 150.0\n",
                )
                .unwrap(),
                _ => {}
            }
            std::fs::write(d.join("notes.txt"), "not an asset").unwrap();
        }
        let fav = PackageFavorites {
            favorite_styles: Vec::new(),
            my_stones: vec![emb_model::Stone::default()],
        };
        let files = package_files(&a, &fav);
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"motifs/leaf.embmotif") && names.contains(&"threads/mine.gpl"),
            "{names:?}"
        );
        assert!(names.contains(&"hoops/hoops.toml") && names.contains(&FAVORITES_FILE));
        assert!(!names.iter().any(|n| n.ends_with("notes.txt")));
        // On the second computer.
        let mut h = Harness::empty();
        h.st.save_settings = false;
        let summary = h.st.import_package(&crate::share::zip(&files), &dirs(&to)).unwrap();
        assert!(to.join("motifs/leaf.embmotif").is_file(), "{summary}");
        assert!(to.join("threads/mine.gpl").is_file());
        assert!(h.st.hoops.get("my_150").is_some(), "the hoop is there: {summary}");
        assert_eq!(h.st.settings.my_stones.len(), 1);
        // The same package again: no second stone.
        h.st.import_package(&crate::share::zip(&files), &dirs(&to)).unwrap();
        assert_eq!(h.st.settings.my_stones.len(), 1);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn the_window_draws() {
        let mut h = Harness::empty();
        h.st.assets_window.open = true;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let ctx = ui.ctx().clone();
            window(&ctx, &mut h.st);
        });
        out.textures_delta.clear();
        assert!(h.st.assets_window.open);
    }
}
