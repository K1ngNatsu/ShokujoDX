//! The Scripts window (Edit > Scripts, P13 Milestone 4, ADR 0067): the
//! sample scripts and the user's scripts (`.rhai` files in the `scripts`
//! folder of the settings folder), the inputs of a script, Run, and the
//! messages. The engine work is in `emb_script`; a script changes a copy
//! of the design, and the program puts the result in the document with one
//! edit (one undo step).

use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui;
use emb_script::{InputValue, Limits, ScriptInfo};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// A script that the window can run.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptEntry {
    /// The file name (for a sample) or the path (for a user script).
    pub file: String,
    pub path: Option<PathBuf>,
    pub source: String,
    pub info: ScriptInfo,
}

impl ScriptEntry {
    pub fn is_sample(&self) -> bool {
        self.path.is_none()
    }
}

/// The state of the Scripts window.
#[derive(Debug, Clone, Default)]
pub struct ScriptsWindow {
    pub list: Vec<ScriptEntry>,
    pub selected: usize,
    /// The input values of the selected script.
    pub inputs: BTreeMap<String, InputValue>,
    /// The messages of the last run, and its error.
    pub messages: Vec<String>,
    pub error: Option<String>,
    pub loaded: bool,
}

/// The folder of the user's scripts.
pub fn scripts_dir() -> Option<PathBuf> {
    crate::about::user_folder().map(|d| d.join("scripts"))
}

/// The sample scripts and the scripts of `dir`, each with its title.
pub fn load_scripts(dir: Option<&std::path::Path>) -> Vec<ScriptEntry> {
    let mut list: Vec<ScriptEntry> = emb_script::SAMPLES
        .iter()
        .map(|(file, src)| ScriptEntry {
            file: file.to_string(),
            path: None,
            source: src.to_string(),
            info: ScriptInfo::parse(src, file),
        })
        .collect();
    let mut user: Vec<ScriptEntry> = dir
        .and_then(|d| std::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(emb_script::FILE_EXTENSION))
        })
        .filter_map(|p| {
            // A file larger than 1 MB is not a script.
            if std::fs::metadata(&p).map_or(true, |m| m.len() > 1 << 20) {
                return None;
            }
            let source = std::fs::read_to_string(&p).ok()?;
            let file = p.file_name()?.to_string_lossy().into_owned();
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            Some(ScriptEntry {
                info: ScriptInfo::parse(&source, &stem),
                file,
                path: Some(p),
                source,
            })
        })
        .collect();
    user.sort_by_key(|s| s.info.title.to_lowercase());
    list.extend(user);
    list
}

impl AppState {
    /// Reads the list of scripts again.
    pub fn reload_scripts(&mut self) {
        let keep = self.scripts.list.get(self.scripts.selected).map(|s| s.file.clone());
        self.scripts.list = load_scripts(scripts_dir().as_deref());
        self.scripts.selected = keep
            .and_then(|f| self.scripts.list.iter().position(|s| s.file == f))
            .unwrap_or(0);
        self.scripts.loaded = true;
        self.reset_script_inputs();
    }

    fn reset_script_inputs(&mut self) {
        self.scripts.inputs = self
            .scripts
            .list
            .get(self.scripts.selected)
            .map(|s| s.info.defaults())
            .unwrap_or_default();
    }

    /// Runs script `source` (title `title`) on the open design and its
    /// selection. A change is one undo step; an error changes nothing.
    pub fn run_script(&mut self, title: &str, source: &str, inputs: &BTreeMap<String, InputValue>) {
        let machine = self
            .doc
            .design()
            .machine
            .as_deref()
            .and_then(|m| self.machines.get(m))
            .cloned();
        let selection = self.selection.in_sew_order(self.doc.design());
        let result = emb_script::run(
            source,
            self.doc.design(),
            &selection,
            inputs,
            machine.as_ref(),
            &Limits::default(),
        );
        match result {
            Ok(out) => {
                self.scripts.messages = out.messages;
                self.scripts.error = None;
                if out.changed {
                    let new = out.design;
                    self.doc.edit(format!("Script: {title}"), move |d| *d = new);
                    self.selection.set_all(out.selection);
                    self.message = fill("The script \"{}\" ran. Undo takes back its changes.", &[&title]);
                } else {
                    self.message = fill("The script \"{}\" ran. It changed nothing.", &[&title]);
                }
            }
            Err(e) => {
                self.scripts.messages = e.messages.clone();
                self.scripts.error = Some(e.to_string());
                self.message = fill("The script \"{}\" stopped: {}", &[&title, &e]);
            }
        }
    }

    /// Copies sample script `k` to the user's script folder, so the user
    /// can change it.
    fn copy_sample(&mut self, k: usize) {
        let (Some(s), Some(dir)) = (self.scripts.list.get(k).cloned(), scripts_dir()) else {
            return;
        };
        let path = dir.join(&s.file);
        let r = std::fs::create_dir_all(&dir).and_then(|_| {
            if path.exists() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "the file is there already",
                ))
            } else {
                std::fs::write(&path, &s.source)
            }
        });
        match r {
            Ok(()) => {
                self.message = fill("Copied to {}. Change it in a text editor.", &[&path.display()]);
                self.reload_scripts();
                crate::library::show_in_file_manager(&path);
            }
            Err(e) => self.message = fill("Not copied: {}", &[&e]),
        }
    }
}

/// The Scripts window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_scripts {
        return;
    }
    if !st.scripts.loaded {
        st.reload_scripts();
    }
    let mut open = true;
    let mut w = std::mem::take(&mut st.scripts);
    let selected_count = st.selection.len();
    let mut run = false;
    let mut reload = false;
    let mut copy = None;
    let mut open_folder = false;
    let before = w.selected;
    crate::dock::docked(
        egui::Window::new(tr("Scripts")).id(egui::Id::new("Scripts")),
        ctx,
        egui::Id::new("Scripts"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.horizontal(|ui| {
            let shown = w.list.get(w.selected).map(|s| s.info.title.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("script_pick")
                .selected_text(shown)
                .width(260.0)
                .show_ui(ui, |ui| {
                    for (i, s) in w.list.iter().enumerate() {
                        let text = if s.is_sample() {
                            fill("{} (sample)", &[&s.info.title])
                        } else {
                            s.info.title.clone()
                        };
                        ui.selectable_value(&mut w.selected, i, text);
                    }
                });
            if crate::a11y::named(
                ui.button(egui_phosphor::regular::ARROWS_CLOCKWISE),
                tr("Read the scripts again"),
            )
            .on_hover_text(tr("Read the scripts of your folder again."))
            .clicked()
            {
                reload = true;
            }
        });
        let Some(s) = w.list.get(w.selected).cloned() else {
            ui.label(tr("No scripts."));
            return;
        };
        if !s.info.description.is_empty() {
            ui.label(&s.info.description);
        }
        if !s.info.inputs.is_empty() {
            ui.separator();
            egui::Grid::new("script_inputs")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    for i in &s.info.inputs {
                        ui.label(&i.label);
                        let v = w.inputs.entry(i.name.clone()).or_insert_with(|| i.default.clone());
                        match v {
                            InputValue::Number(n) => {
                                ui.add(egui::DragValue::new(n).speed(0.1));
                            }
                            InputValue::Bool(b) => {
                                ui.checkbox(b, "");
                            }
                            InputValue::Text(t) => {
                                ui.text_edit_singleline(t);
                            }
                        }
                        ui.end_row();
                    }
                });
        }
        ui.separator();
        ui.weak(if selected_count == 0 {
            tr("The script works on all the design (nothing is selected).").to_string()
        } else {
            fill("The script gets the {} selected objects.", &[&selected_count])
        });
        ui.horizontal(|ui| {
            if ui
                .button(tr("Run"))
                .on_hover_text(tr("Run the script. Undo takes back all its changes in one step."))
                .clicked()
            {
                run = true;
            }
            if s.is_sample()
                && ui
                    .button(tr("Copy to My Scripts"))
                    .on_hover_text(tr(
                        "Copy this sample to your scripts folder, to change it in a text editor.",
                    ))
                    .clicked()
            {
                copy = Some(w.selected);
            }
            if ui
                .button(tr("Open Scripts Folder"))
                .on_hover_text(tr("Put .rhai files here. See Help > Scripts."))
                .clicked()
            {
                open_folder = true;
            }
        });
        if let Some(e) = &w.error {
            ui.colored_label(egui::Color32::from_rgb(220, 60, 50), e);
        }
        if !w.messages.is_empty() {
            ui.separator();
            ui.strong(tr("Messages"));
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for m in &w.messages {
                    ui.label(m);
                }
            });
        }
    });
    let picked = w.selected != before;
    st.scripts = w;
    if picked {
        st.reset_script_inputs();
        st.scripts.messages.clear();
        st.scripts.error = None;
    }
    if reload {
        st.reload_scripts();
    }
    if run && let Some(s) = st.scripts.list.get(st.scripts.selected).cloned() {
        let inputs = st.scripts.inputs.clone();
        st.run_script(&s.info.title, &s.source, &inputs);
    }
    if let Some(k) = copy {
        st.copy_sample(k);
    }
    if open_folder && let Some(dir) = scripts_dir() {
        match std::fs::create_dir_all(&dir) {
            Ok(()) => crate::library::show_in_file_manager(&dir.join(".")),
            Err(e) => st.message = fill("Not opened: {}", &[&e]),
        }
    }
    if !open {
        st.show_scripts = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_samples_are_listed_and_user_scripts_follow() {
        let dir = std::env::temp_dir().join(format!("emb-scripts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.rhai"), "//! Beta\nprint(1);").unwrap();
        std::fs::write(dir.join("a.rhai"), "print(2);").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a script").unwrap();
        let list = load_scripts(Some(&dir));
        std::fs::remove_dir_all(&dir).ok();
        let n = emb_script::SAMPLES.len();
        assert_eq!(list.len(), n + 2);
        assert!(list[..n].iter().all(ScriptEntry::is_sample));
        // Sorted by title; a script with no title takes its file name.
        assert_eq!(list[n].info.title, "a");
        assert_eq!(list[n + 1].info.title, "Beta");
    }

    #[test]
    fn a_script_is_one_undo_step_and_an_error_changes_nothing() {
        let mut h = Harness::new();
        let before =
            h.st.doc
                .design()
                .threads
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>();
        let src = "for t in threads() { set_thread_name(t.id, \"X \" + t.name); } print(\"done\");";
        h.st.run_script("Rename", src, &BTreeMap::new());
        assert_eq!(h.st.scripts.messages, vec!["done".to_string()]);
        assert!(h.st.doc.design().threads.iter().all(|t| t.name.starts_with("X ")));
        h.st.doc.undo();
        let after =
            h.st.doc
                .design()
                .threads
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>();
        assert_eq!(after, before);
        // An error: the design stays, the error shows.
        let n = h.st.doc.design().objects().len();
        h.st.run_script("Bad", "remove(objects()[0]); throw \"stop\";", &BTreeMap::new());
        assert_eq!(h.st.doc.design().objects().len(), n);
        assert!(h.st.scripts.error.as_deref().unwrap().contains("stop"));
    }

    #[test]
    fn the_window_draws() {
        let mut h = Harness::new();
        h.st.show_scripts = true;
        let objects = h.st.doc.design().objects().len();
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert!(h.st.show_scripts);
        assert!(h.st.scripts.list.len() >= emb_script::SAMPLES.len());
        assert_eq!(h.st.doc.design().objects().len(), objects, "drawing changes nothing");
    }
}
