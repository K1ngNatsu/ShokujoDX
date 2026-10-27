//! The Team names window (P4 Milestone 6, ADR 0020): the columns (lettering
//! objects of the design), the list of names with quantities, a pasted or
//! CSV import, and the output: one machine file for each name (one step),
//! or all names in one design in a new tab.

use std::path::{Path, PathBuf};

use eframe::egui;
use emb_model::{HoopSelection, MergeKey, ObjectKind, TeamColumn, TeamNames, TeamRow};

use crate::a11y::Named;
use crate::app::AppState;
use crate::i18n::tr;

/// Space between the names in the all-in-one design, in mm.
const LAYOUT_GAP_MM: f64 = 10.0;

#[derive(Debug, Clone, Default)]
pub struct TeamNamesWindow {
    pub open: bool,
    /// Text to import.
    pub paste: String,
    /// The first line of an import is a header.
    pub header: bool,
}

/// A file name part from a name: no characters that file systems refuse.
fn file_part(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let t = t.trim().to_string();
    if t.is_empty() { "name".into() } else { t }
}

impl AppState {
    pub fn open_team_names(&mut self) {
        self.fonts.start_scan();
        self.team_window.open = true;
        if self.doc.design().team_names.is_none()
            && let Some(id) = self.selection.single()
            && matches!(
                self.doc.design().object(id).map(|o| &o.kind),
                Some(ObjectKind::Lettering(_))
            )
        {
            self.team_add_column(id);
        }
    }

    /// Changes the team names of the design (merged while `key` is the same).
    fn set_team_names(&mut self, tn: TeamNames, key: Option<&str>) {
        match key {
            Some(k) => self
                .doc
                .edit_merged("Change team names", MergeKey::new(format!("team-{k}")), |d| {
                    d.team_names = Some(tn)
                }),
            None => self.doc.edit("Change team names", |d| d.team_names = Some(tn)),
        }
    }

    /// Makes lettering object `id` a column of the team names.
    pub fn team_add_column(&mut self, id: emb_model::ObjectId) {
        let mut tn = self.doc.design().team_names.clone().unwrap_or_default();
        if tn.columns.iter().any(|c| c.object == id) {
            return;
        }
        let title = if tn.columns.is_empty() { "Name" } else { "Number" };
        tn.columns.push(TeamColumn {
            object: id,
            title: title.into(),
            max_width_mm: None,
        });
        tn.fit_rows();
        self.set_team_names(tn, None);
    }

    fn fonts_fn(&mut self) -> impl FnMut(&emb_model::FontRef) -> Option<emb_letter::AnyFont> + '_ {
        |r| self.fonts.get(r)
    }

    /// Writes a machine file for each name (each row, once) to `folder`, in
    /// the formats of the output profile (or the machine format). Returns
    /// the files.
    pub fn team_save_each(&mut self, folder: &Path) -> Result<Vec<PathBuf>, String> {
        let d = self.doc.design().clone();
        let tn = d
            .team_names
            .clone()
            .filter(|t| !t.rows.is_empty())
            .ok_or("The list has no names.")?;
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let opts = d
            .machine
            .as_deref()
            .and_then(|m| self.machines.get(m))
            .map(emb_stitch::PlanOptions::for_machine)
            .unwrap_or_default();
        let mut formats: Vec<String> = self
            .settings
            .output
            .formats
            .iter()
            .filter(|e| emb_formats::WRITE_EXTENSIONS.contains(&e.as_str()))
            .cloned()
            .collect();
        if formats.is_empty() {
            let def = d
                .machine
                .as_deref()
                .and_then(|m| self.machines.get(m))
                .map(|m| m.default_format.clone())
                .unwrap_or_else(|| "dst".into());
            formats.push(def);
        }
        let base = file_part(&d.name);
        let date = crate::output::local_stamp();
        let machine = self.machine_for_files(d.machine.as_deref());
        let mut written: Vec<PathBuf> = Vec::new();
        let mut used = std::collections::HashSet::new();
        for row in &tn.rows {
            let rd = emb_letter::teamnames::row_design(&d, &tn, row, &mut self.fonts_fn())?;
            let plan = emb_stitch::build_plan(&rd, &opts, &mut emb_stitch::StitchCache::default());
            let mut name = format!("{base} - {}", file_part(row.label()));
            let mut n = 2;
            while !used.insert(name.to_lowercase()) {
                name = format!("{base} - {} ({n})", file_part(row.label()));
                n += 1;
            }
            let needles = self.catalogs().needle_plan(&rd, &plan.stitches);
            for ext in &formats {
                let bytes = emb_formats::write_machine_with(
                    ext,
                    &plan.stitches,
                    &name,
                    &date,
                    machine.as_ref(),
                    needles.as_ref(),
                )
                .ok_or_else(|| format!(".{ext} cannot be written"))?;
                let path = folder.join(format!("{name}.{ext}"));
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                written.push(path);
            }
        }
        Ok(written)
    }

    /// All names in one design, in a new tab.
    pub fn team_layout(&mut self) -> Result<(), String> {
        let d = self.doc.design().clone();
        let tn = d
            .team_names
            .clone()
            .filter(|t| !t.rows.is_empty())
            .ok_or("The list has no names.")?;
        let width = match &d.hoop.selection {
            HoopSelection::Preset { id } => self.hoops.get(id).map_or(200.0, |h| h.width_mm),
            HoopSelection::OpenField => 200.0,
        };
        let mut all = emb_letter::teamnames::layout_design(&d, &tn, &mut self.fonts_fn(), width, LAYOUT_GAP_MM)?;
        all.name = format!("{} (all names)", d.name);
        self.start_design(all);
        self.message = crate::i18n::fill(
            "{} names in one design. Check that they fit the hoop.",
            &[&(tn.total())],
        );
        Ok(())
    }

    /// Shows a row in the design: the column objects get its values.
    pub fn team_show_row(&mut self, i: usize) -> Result<(), String> {
        let d = self.doc.design().clone();
        let tn = d.team_names.clone().ok_or("No team names.")?;
        let row = tn.rows.get(i).ok_or("No such row.")?.clone();
        // Show every column, also an empty one, so that the objects stay.
        let filled = TeamRow {
            values: row
                .values
                .iter()
                .zip(&tn.columns)
                .map(|(v, c)| {
                    if v.trim().is_empty() {
                        c.title.clone()
                    } else {
                        v.clone()
                    }
                })
                .collect(),
            quantity: 1,
        };
        let rd = emb_letter::teamnames::row_design(&d, &tn, &filled, &mut self.fonts_fn())?;
        self.doc.edit("Show name", |dd| {
            for c in &tn.columns {
                if let (Some(o), Some(n)) = (rd.object(c.object), dd.object_mut(c.object)) {
                    n.kind = o.kind.clone();
                }
            }
        });
        Ok(())
    }
}

/// The Team names window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.team_window.open {
        return;
    }
    let mut open = true;
    let mut tn = st.doc.design().team_names.clone().unwrap_or_default();
    let before = tn.clone();
    let (mut save_each, mut layout, mut show, mut add_col, mut import_file) = (false, false, None, false, false);
    let mut w = st.team_window.clone();
    let selected_lettering = st.selection.single().filter(|id| {
        matches!(
            st.doc.design().object(*id).map(|o| &o.kind),
            Some(ObjectKind::Lettering(_))
        ) && !tn.columns.iter().any(|c| c.object == *id)
    });
    crate::dock::docked(
        egui::Window::new(tr("Team Names")).id(egui::Id::new("Team Names")),
        ctx,
        egui::Id::new("Team Names"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .default_width(460.0)
    .show(ctx, |ui| {
        ui.strong(tr("Columns"));
        if tn.columns.is_empty() {
            ui.weak(tr(
                "Select a lettering object of the design (for example the name) and add it as a column.",
            ));
        }
        let mut remove = None;
        for (i, c) in tn.columns.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut c.title).desired_width(90.0));
                let mut fit = c.max_width_mm.is_some();
                if ui
                    .checkbox(&mut fit, tr("Fit to width"))
                    .on_hover_text(tr("A longer text gets smaller, so that it is at most this wide."))
                    .changed()
                {
                    c.max_width_mm = fit.then_some(80.0);
                }
                if let Some(m) = &mut c.max_width_mm {
                    ui.add(
                        egui::DragValue::new(m)
                            .range(5.0..=400.0)
                            .clamp_existing_to_range(false)
                            .speed(0.5)
                            .suffix(" mm"),
                    );
                }
                if ui.small_button(tr("Remove")).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            tn.columns.remove(i);
            for r in &mut tn.rows {
                if i < r.values.len() {
                    r.values.remove(i);
                }
            }
        }
        add_col = ui
            .add_enabled(
                selected_lettering.is_some(),
                egui::Button::new(tr("Add the selected lettering as a column")),
            )
            .clicked();
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(tr("Names"));
            ui.weak(crate::i18n::fill(
                "{} rows, {} to sew",
                &[&(tn.rows.len()), &(tn.total())],
            ));
        });
        let ncol = tn.columns.len();
        let mut delete = None;
        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            egui::Grid::new("team_rows").striped(true).show(ui, |ui| {
                for c in &tn.columns {
                    ui.strong(&c.title);
                }
                ui.strong(tr("Qty"));
                ui.end_row();
                for (i, r) in tn.rows.iter_mut().enumerate() {
                    r.values.resize(ncol.max(1), String::new());
                    for v in r.values.iter_mut().take(ncol.max(1)) {
                        ui.add(egui::TextEdit::singleline(v).desired_width(120.0));
                    }
                    ui.add(
                        egui::DragValue::new(&mut r.quantity)
                            .range(1..=999)
                            .clamp_existing_to_range(false),
                    );
                    if ui
                        .small_button(tr("Show"))
                        .on_hover_text(tr("Show this name in the design."))
                        .clicked()
                    {
                        show = Some(i);
                    }
                    if ui
                        .small_button(egui_phosphor::regular::X)
                        .on_hover_text(tr("Delete this row."))
                        .named(tr("Delete this row."))
                        .clicked()
                    {
                        delete = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(i) = delete {
            tn.rows.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Add a row")).clicked() {
                tn.rows.push(TeamRow {
                    values: vec![String::new(); ncol.max(1)],
                    quantity: 1,
                });
            }
            import_file = ui.button(tr("Import CSV…")).clicked();
        });
        ui.collapsing(tr("Paste a list"), |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut w.paste)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr("One name for each line. Columns: tab, comma or semicolon.")),
            );
            ui.horizontal(|ui| {
                ui.checkbox(&mut w.header, tr("The first line is a header"));
                if ui.button(tr("Add these names")).clicked() {
                    tn.import_text(&w.paste, w.header);
                    w.paste.clear();
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            let ready = !tn.rows.is_empty() && !tn.columns.is_empty();
            save_each = ui
                .add_enabled(ready, egui::Button::new(tr("Save each name…")))
                .on_hover_text(tr("One machine file for each name, in a folder, in one step."))
                .clicked();
            layout = ui
                .add_enabled(ready, egui::Button::new(tr("All in one design")))
                .on_hover_text(tr(
                    "All names in one design (a new tab), color by color, for one hooping.",
                ))
                .clicked();
        });
    });
    w.open = open;
    st.team_window = w;
    if tn != before {
        let typing = ctx.memory(|m| m.focused().is_some());
        st.set_team_names(tn, Some("list"));
        if !typing {
            st.doc.end_merge_of(&MergeKey::new("team-list"));
        }
    }
    if add_col && let Some(id) = selected_lettering {
        st.team_add_column(id);
    }
    if import_file
        && let Some(p) = rfd::FileDialog::new()
            .set_title("Import names")
            .add_filter("Name lists", &["csv", "txt", "tsv"])
            .set_directory(st.dialog_dir())
            .pick_file()
    {
        match std::fs::read_to_string(&p) {
            Ok(text) => {
                let mut tn = st.doc.design().team_names.clone().unwrap_or_default();
                let n = tn.import_text(&text, st.team_window.header);
                st.set_team_names(tn, None);
                st.message = crate::i18n::fill("{} names added from {}", &[&n, &(p.display())]);
            }
            Err(e) => st.message = format!("{}: {e}", p.display()),
        }
    }
    if let Some(i) = show
        && let Err(e) = st.team_show_row(i)
    {
        st.message = e;
    }
    if save_each
        && let Some(folder) = rfd::FileDialog::new()
            .set_title("Save each name: folder")
            .set_directory(st.dialog_dir())
            .pick_folder()
    {
        st.message = match st.team_save_each(&folder) {
            Ok(files) => format!("Saved {} files in {}", files.len(), folder.display()),
            Err(e) => e,
        };
    }
    if layout && let Err(e) = st.team_layout() {
        st.message = e;
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::{FontRef, HoopCatalog, MachineCatalog};

    use super::*;
    use crate::lettering::LetteringDialog;

    fn team(st: &mut AppState) {
        st.add_lettering(LetteringDialog::new(
            Point::new(0.0, 0.0),
            "NAME",
            FontRef::embroidery("Block Sans", ""),
            12.0,
        ));
        let id = st.doc.design().objects()[0].id;
        st.selection.set(id);
        st.open_team_names();
        let mut tn = st.doc.design().team_names.clone().unwrap();
        let names: Vec<String> = (1..=20).map(|i| format!("Player {i}")).collect();
        tn.import_text(&names.join("\n"), false);
        st.set_team_names(tn, None);
    }

    #[test]
    fn twenty_names_export_in_one_step() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        team(&mut st);
        assert_eq!(st.doc.design().team_names.as_ref().unwrap().rows.len(), 20);
        let dir = std::env::temp_dir().join(format!("emb-team-{}", std::process::id()));
        let files = st.team_save_each(&dir).unwrap();
        assert_eq!(files.len(), 20, "one file for each name");
        for f in &files {
            let bytes = std::fs::read(f).unwrap();
            let ext = f.extension().unwrap().to_string_lossy().to_string();
            let s = emb_formats::read_machine(&ext, &bytes).unwrap();
            assert!(s.stitch_count() > 100, "{}", f.display());
        }
        assert!(files[0].file_name().unwrap().to_string_lossy().contains("Player 1"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn all_in_one_design_and_show() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        team(&mut st);
        st.team_show_row(2).unwrap();
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(l.text, "Player 3");
        st.team_layout().unwrap();
        assert_eq!(st.tab_count(), 2);
        assert_eq!(st.doc.design().objects().len(), 20);
        assert!(st.doc.design().team_names.is_none());
    }

    #[test]
    fn the_window_draws() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        team(&mut st);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.team_window.open);
        assert_eq!(
            st.doc.design().team_names.as_ref().unwrap().rows.len(),
            20,
            "drawing changes nothing"
        );
    }
}
