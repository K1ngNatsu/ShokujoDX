//! Threads, thread charts and colorways (roadmap P3, Milestone 1).
//!
//! The Threads panel has three parts:
//! - Colorway: select, add, rename and delete colorways; the background color.
//! - Design threads: edit name and color, select the thread for new objects,
//!   and a table of all colorways when there is more than one.
//! - Thread chart: search a chart; click a chart thread to give its look to
//!   the current thread, Ctrl+click to add it as a new thread.

use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use emb_model::{MergeKey, Rgb, ThreadChart, ThreadId, ThreadLook, colorway::chart_look};

use crate::app::AppState;
use crate::canvas::color32;
use crate::file::Notice;
use crate::i18n::tr;

/// The folder for imported charts: `threads` next to the settings file.
pub fn user_chart_dir() -> Option<std::path::PathBuf> {
    crate::workspace::settings_path().and_then(|p| p.parent().map(|d| d.join("threads")))
}

/// The built-in charts, then the charts in `data/threads` (next to the program
/// or in the working folder), then the charts that the user imported.
pub fn load_charts() -> (Vec<ThreadChart>, Vec<String>) {
    let mut charts = ThreadChart::builtin();
    let mut errors = Vec::new();
    let dirs = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("data").join("threads"))),
        Some(std::path::PathBuf::from("data").join("threads")),
        user_chart_dir(),
    ];
    for dir in dirs.into_iter().flatten() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        files.sort();
        for p in files {
            let ext = p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
            if !matches!(ext.as_deref(), Some("gpl" | "csv")) {
                continue;
            }
            match ThreadChart::load(&p) {
                Ok(c) if charts.iter().any(|x| x.name == c.name) => {}
                Ok(c) => charts.push(c),
                Err(e) => errors.push(format!("{}: {e}", p.display())),
            }
        }
    }
    (charts, errors)
}

/// Rgb to egui color and back.
fn rgb(c: Color32) -> Rgb {
    Rgb(c.r(), c.g(), c.b())
}

impl AppState {
    /// The chart that the Threads panel shows.
    pub fn chart(&self) -> Option<&ThreadChart> {
        self.charts.get(self.chart_index).or_else(|| self.charts.first())
    }

    /// The thread for new objects: the one the user picked, else the first.
    pub fn current_thread_id(&self) -> Option<ThreadId> {
        let threads = &self.doc.design().threads;
        self.current_thread
            .filter(|t| threads.iter().any(|x| x.id == *t))
            .or_else(|| threads.first().map(|t| t.id))
    }

    /// Asks for a chart file (GIMP palette or CSV), copies it to the user
    /// chart folder and shows it.
    pub fn import_chart_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title("Import thread chart")
            .add_filter("Thread charts (GIMP palette, CSV)", &["gpl", "csv"])
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        self.import_chart_file(&p);
    }

    pub fn import_chart_file(&mut self, p: &std::path::Path) {
        let chart = match ThreadChart::load(p) {
            Ok(c) => c,
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot import the thread chart".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                });
                return;
            }
        };
        if self.save_settings
            && let Some(dir) = user_chart_dir()
            && let Some(name) = p.file_name()
        {
            let copied = std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(p, dir.join(name)));
            if let Err(e) = copied {
                self.message = crate::i18n::fill("Chart is open, but not saved for the next start: {}", &[&e]);
            }
        }
        let n = chart.threads.len();
        let name = chart.name.clone();
        match self.charts.iter().position(|c| c.name == chart.name) {
            Some(i) => {
                self.charts[i] = chart;
                self.chart_index = i;
            }
            None => {
                self.charts.push(chart);
                self.chart_index = self.charts.len() - 1;
            }
        }
        self.message = crate::i18n::fill("Thread chart \"{}\" imported ({} threads)", &[&name, &n]);
    }

    /// Gives all used threads the look of the nearest thread of the current chart.
    pub fn match_to_chart(&mut self) {
        let Some(chart) = self.chart().cloned() else { return };
        let ids = self.doc.design().used_threads();
        if ids.is_empty() {
            self.message = tr("No threads are used").into();
            return;
        }
        self.doc
            .edit("Match threads to chart", |d| d.match_threads(&chart, &ids));
        self.message = crate::i18n::fill("{} threads matched to {}", &[&(ids.len()), &chart.name]);
    }

    pub fn remove_unused_threads(&mut self) {
        let mut probe = self.doc.design().clone();
        let n = probe.remove_unused_threads();
        if n == 0 {
            self.message = tr("No unused threads").into();
            return;
        }
        self.doc.edit("Remove unused threads", |d| d.remove_unused_threads());
        self.message = crate::i18n::fill("{} unused threads removed", &[&n]);
    }

    pub fn cycle_colors(&mut self) {
        if self.doc.design().used_threads().len() < 2 {
            self.message = tr("Cycle colors needs two or more colors").into();
            return;
        }
        self.doc.edit("Cycle used colors", |d| d.cycle_used_colors());
    }

    /// Gives the selection its own thread.
    pub fn split_thread(&mut self) {
        let ids = self.selection.ids().to_vec();
        if let Some(t) = self.doc.edit("Split Thread", |d| d.split_thread(&ids)) {
            self.current_thread = Some(t);
        }
    }

    pub fn new_colorway(&mut self) {
        let n = self.doc.design().colorways.len().max(1) + 1;
        self.doc
            .edit("New Colorway", |d| d.add_colorway(format!("Colorway {n}")));
    }

    pub fn delete_colorway(&mut self) {
        let i = self.doc.design().colorway;
        self.doc.edit("Delete Colorway", |d| d.remove_colorway(i));
    }

    /// Moves the current colorway one place up or down the list.
    pub fn move_colorway(&mut self, up: bool) {
        let i = self.doc.design().colorway;
        let n = self.doc.design().colorways.len();
        if (up && i > 0) || (!up && i + 1 < n) {
            self.doc.edit("Move colorway", |d| d.move_colorway(i, up));
        }
    }

    pub fn switch_colorway(&mut self, i: usize) {
        if i != self.doc.design().colorway || self.doc.design().colorways.is_empty() {
            self.doc.edit("Change colorway", |d| d.switch_colorway(i));
        }
    }

    /// Sets the look of a thread in the current colorway.
    fn set_look(&mut self, id: ThreadId, look: ThreadLook, label: &str, merge: Option<String>) {
        let i = self.doc.design().colorway;
        let f = |d: &mut emb_model::Design| d.set_colorway_look(i, id, look);
        match merge {
            Some(k) => self.doc.edit_merged(label, MergeKey::new(k), f),
            None => self.doc.edit(label, f),
        }
    }

    /// Gives chart thread `k` to the current thread, or adds it as a new
    /// thread when `add` is true.
    pub fn use_chart_thread(&mut self, k: usize, add: bool) {
        let Some(chart) = self.chart() else { return };
        let look = chart_look(chart, k);
        match self.current_thread_id() {
            Some(id) if !add => self.set_look(id, look, "Set thread from chart", None),
            _ => {
                let id = self.doc.edit("Add thread from chart", |d| {
                    let id = d.add_thread(look.name.clone(), look.color);
                    let i = d.colorway;
                    d.set_colorway_look(i, id, look);
                    id
                });
                self.current_thread = Some(id);
            }
        }
    }
}

/// A color swatch that opens a color picker. Returns the new color when the
/// user changes it.
fn swatch_picker(ui: &mut egui::Ui, id: egui::Id, color: Rgb, size: f32, selected: bool, tip: &str) -> Option<Rgb> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, color32(color));
    let ring = if selected {
        egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
    } else {
        egui::Stroke::new(1.0, ui.visuals().weak_text_color())
    };
    p.rect_stroke(rect, 3.0, ring, egui::StrokeKind::Outside);
    let resp = resp.on_hover_text(tip);
    let mut out = None;
    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let mut c = color32(color);
            if egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque) {
                out = Some(rgb(c));
            }
        });
    out
}

/// The Threads panel.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        colorway_section(ui, st);
        ui.separator();
        threads_section(ui, st);
        ui.separator();
        egui::CollapsingHeader::new(tr("Color wheel"))
            .id_salt("color_wheel")
            .show(ui, |ui| crate::viewtools::color_wheel(ui, st));
        ui.separator();
        chart_section(ui, st);
    });
}

fn colorway_section(ui: &mut egui::Ui, st: &mut AppState) {
    let d = st.doc.design();
    let names: Vec<String> = d.colorways.iter().map(|c| c.name.clone()).collect();
    let current = d.colorway;
    let background = d.background;
    ui.horizontal(|ui| {
        ui.strong(tr("Colorway"));
        if names.is_empty() {
            ui.weak(tr("one (the design colors)"));
        } else {
            egui::ComboBox::from_id_salt("colorway")
                .selected_text(names.get(current).cloned().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        if ui.selectable_label(i == current, n).clicked() {
                            st.switch_colorway(i);
                        }
                    }
                });
        }
        if ui
            .button(tr("New"))
            .on_hover_text(tr("A copy of the current colors, to change"))
            .clicked()
        {
            st.new_colorway();
        }
        if ui
            .add_enabled(names.len() > 1, egui::Button::new(tr("Delete")))
            .clicked()
        {
            st.delete_colorway();
        }
        let up = ui
            .add_enabled(current > 0, egui::Button::new(egui_phosphor::regular::ARROW_UP))
            .on_hover_text(tr("Move this colorway up the list"));
        if up.clicked() {
            st.move_colorway(true);
        }
        let down = ui
            .add_enabled(
                current + 1 < names.len(),
                egui::Button::new(egui_phosphor::regular::ARROW_DOWN),
            )
            .on_hover_text(tr("Move this colorway down the list"));
        if down.clicked() {
            st.move_colorway(false);
        }
    });
    if let Some(name) = names.get(current) {
        let mut text = name.clone();
        ui.horizontal(|ui| {
            ui.label(tr("Name"));
            if ui.text_edit_singleline(&mut text).changed() {
                st.doc.edit_merged(
                    "Rename colorway",
                    MergeKey::new(format!("colorway-name-{current}")),
                    |d| {
                        if let Some(c) = d.colorways.get_mut(current) {
                            c.name = text;
                        }
                    },
                );
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label(tr("Background"));
        let shown = background.unwrap_or(Rgb(255, 255, 255));
        let tip = "The fabric color, inside the hoop. It changes with the colorway.";
        if let Some(c) = swatch_picker(ui, ui.id().with("background"), shown, 18.0, false, tip) {
            st.doc
                .edit_merged("Background color", MergeKey::new("background"), |d| {
                    d.background = Some(c)
                });
        }
        if ui
            .add_enabled(background.is_some(), egui::Button::new(tr("None")))
            .clicked()
        {
            st.doc.edit("Background color", |d| d.background = None);
        }
    });
}

fn threads_section(ui: &mut egui::Ui, st: &mut AppState) {
    let d = st.doc.design();
    let threads = d.threads.clone();
    let colorways: Vec<String> = d.colorways.iter().map(|c| c.name.clone()).collect();
    let current_cw = d.colorway;
    let uses = |id: ThreadId| d.objects().iter().filter(|o| o.thread == id).count();
    let counts: Vec<usize> = threads.iter().map(|t| uses(t.id)).collect();
    let current = st.current_thread_id();
    let stock = st.settings.thread_stock.clone();
    ui.strong(tr("Design threads"));
    if threads.is_empty() {
        ui.weak(tr("No threads. Add one, or Ctrl+click a chart thread below."));
    }
    egui::Grid::new("threads").num_columns(4).striped(true).show(ui, |ui| {
        for (t, n) in threads.iter().zip(&counts) {
            let picked = swatch_picker(
                ui,
                ui.id().with(("thread", t.id.0)),
                t.color,
                18.0,
                current == Some(t.id),
                tr("Click: change the color"),
            );
            if let Some(c) = picked {
                let mut look = ThreadLook::of(t);
                look.color = c;
                st.set_look(t.id, look, "Thread color", Some(format!("thread-color-{}", t.id.0)));
            }
            let mut name = t.name.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(120.0));
            if r.changed() {
                let mut look = ThreadLook::of(t);
                look.name = name;
                st.set_look(t.id, look, "Thread name", Some(format!("thread-name-{}", t.id.0)));
            }
            let detail = match (&t.brand, &t.code) {
                (Some(b), Some(c)) => format!("{b} {c}"),
                (Some(b), None) => b.clone(),
                (None, Some(c)) => c.clone(),
                (None, None) => String::new(),
            };
            in_stock_mark(ui, &stock, &ThreadLook::of(t), &detail);
            let use_text = match n {
                0 => "unused".to_string(),
                1 => "1 object".to_string(),
                n => format!("{n} objects"),
            };
            let r = ui
                .selectable_label(current == Some(t.id), use_text)
                .on_hover_text(tr("Click: use this thread for new objects"));
            if r.clicked() {
                st.current_thread = Some(t.id);
            }
            ui.end_row();
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Add thread")).clicked() {
            let id = st.doc.edit("Add thread", |d| {
                let n = d.threads.len() + 1;
                d.add_thread(format!("Thread {n}"), Rgb(128, 128, 128))
            });
            st.current_thread = Some(id);
        }
        if ui.button(tr("Remove Unused")).clicked() {
            st.remove_unused_threads();
        }
        if ui
            .button(tr("Cycle Colors"))
            .on_hover_text(tr("Each used thread takes the color of the next one"))
            .clicked()
        {
            st.cycle_colors();
        }
        if ui
            .add_enabled(!st.selection.is_empty(), egui::Button::new(tr("Split Thread")))
            .on_hover_text(tr("Give the selection its own thread, so its color can change alone"))
            .clicked()
        {
            st.split_thread();
        }
    });

    // All colorways side by side.
    if colorways.len() > 1 {
        ui.add_space(4.0);
        ui.strong(tr("Colorways"));
        let looks: Vec<Vec<(ThreadId, ThreadLook)>> = (0..colorways.len())
            .map(|i| st.doc.design().colorway_looks(i))
            .collect();
        egui::ScrollArea::horizontal().id_salt("colorway_table").show(ui, |ui| {
            egui::Grid::new("colorway_table").show(ui, |ui| {
                ui.label("");
                for (i, name) in colorways.iter().enumerate() {
                    let text = RichText::new(name);
                    let text = if i == current_cw { text.strong() } else { text };
                    if ui
                        .selectable_label(i == current_cw, text)
                        .on_hover_text(tr("Click: show this colorway"))
                        .clicked()
                    {
                        st.switch_colorway(i);
                    }
                }
                ui.end_row();
                for (row, t) in threads.iter().enumerate() {
                    ui.label(&t.name);
                    for (i, cw) in looks.iter().enumerate() {
                        let Some((_, look)) = cw.get(row) else { continue };
                        let id = ui.id().with(("cw", i, t.id.0));
                        let tip = format!("{}: {}", colorways[i], look.name);
                        if let Some(c) = swatch_picker(ui, id, look.color, 16.0, false, &tip) {
                            let mut l = look.clone();
                            l.color = c;
                            st.doc
                                .edit_merged("Colorway color", MergeKey::new(format!("cw-{i}-{}", t.id.0)), |d| {
                                    d.set_colorway_look(i, t.id, l)
                                });
                        }
                    }
                    ui.end_row();
                }
            });
        });
    }
}

/// The brand and code of a thread, with a mark if the thread is in stock.
fn in_stock_mark(ui: &mut egui::Ui, stock: &emb_model::stock::ThreadStock, look: &ThreadLook, detail: &str) {
    match stock.find(look).map(|i| stock.threads[i].spools).filter(|n| *n > 0) {
        Some(n) => {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(70, 170, 90), "●")
                    .on_hover_text(crate::i18n::fill("In stock: {} spools", &[&n]));
                ui.weak(detail);
            });
        }
        None => {
            ui.weak(detail);
        }
    }
}

fn chart_section(ui: &mut egui::Ui, st: &mut AppState) {
    ui.horizontal(|ui| {
        ui.strong(tr("Thread Chart"));
        let names: Vec<String> = st.charts.iter().map(|c| c.name.clone()).collect();
        let shown = st.chart().map(|c| c.name.clone()).unwrap_or_default();
        egui::ComboBox::from_id_salt("chart")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut st.chart_index, i, n);
                }
            });
        if ui
            .button(tr("Import…"))
            .on_hover_text(tr("A GIMP palette (.gpl) or CSV file (code,name,#rrggbb)"))
            .clicked()
        {
            st.import_chart_dialog();
        }
    });
    let mut only = st.stock_only();
    let has_stock = st.charts.iter().any(|c| c.name == emb_model::stock::STOCK_CHART);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has_stock, egui::Checkbox::new(&mut only, tr("Only threads in stock")))
            .on_hover_text(tr(
                "The chart, Match Design to Chart, the picture tools and Picture to Design use only the threads in stock (Design > Thread Stock).",
            ))
            .changed()
        {
            st.set_stock_only(only);
        }
        if ui.small_button(tr("Thread Stock")).clicked() {
            st.show_stock = true;
        }
    });
    if ui
        .button(tr("Match design to chart"))
        .on_hover_text(tr("Each used thread gets the nearest color of this chart"))
        .clicked()
    {
        st.match_to_chart();
    }
    ui.horizontal(|ui| {
        ui.label(tr("Search"));
        ui.add(
            egui::TextEdit::singleline(&mut st.chart_query)
                .hint_text(tr("name or code"))
                .desired_width(160.0),
        );
    });
    let Some(chart) = st.chart() else { return };
    let hits = chart.search(&st.chart_query);
    let stock = &st.settings.thread_stock;
    let rows: Vec<(usize, Rgb, String)> = hits
        .iter()
        .map(|&k| {
            let t = &chart.threads[k];
            let mut text = match &t.code {
                Some(c) => format!("{c}  {}", t.name),
                None => t.name.clone(),
            };
            // A mark for a thread in stock.
            if !stock.is_empty() && stock.has(&chart_look(chart, k)) {
                text.push_str("  ●");
            }
            (k, t.color, text)
        })
        .collect();
    ui.weak(crate::i18n::fill(
        "{} threads. Click: use for the current thread. Ctrl+click: add as a new thread.",
        &[&(rows.len())],
    ));
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt("chart_list")
        .max_height(260.0)
        .show_rows(ui, 20.0, rows.len(), |ui, range| {
            for (k, color, text) in &rows[range] {
                let r = ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color32(*color));
                    ui.add(egui::Button::new(text.as_str()).frame(false))
                });
                if r.inner.clicked() {
                    picked = Some((*k, ui.input(|i| i.modifiers.command)));
                }
            }
        });
    if let Some((k, add)) = picked {
        st.use_chart_thread(k, add);
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn match_cycle_split_and_undo() {
        let mut st = state();
        let start = st.doc.design().clone();
        st.match_to_chart();
        assert!(
            st.doc
                .design()
                .threads
                .iter()
                .all(|t| t.brand.as_deref() == Some("Brother PEC"))
        );
        st.cycle_colors();
        st.select_all();
        st.split_thread();
        let n = st.doc.design().threads.len();
        assert_eq!(n, start.threads.len() + 1);
        st.remove_unused_threads();
        assert_eq!(st.doc.design().threads.len(), 1, "all objects are on the new thread");
        for _ in 0..4 {
            st.undo();
        }
        assert!(st.doc.design().same_content(&start));
    }

    #[test]
    fn chart_thread_to_current_or_new() {
        let mut st = state();
        let first = st.doc.design().threads[0].id;
        st.current_thread = Some(first);
        st.use_chart_thread(0, false);
        assert_eq!(st.doc.design().thread(first).unwrap().name, "Prussian Blue");
        let n = st.doc.design().threads.len();
        st.use_chart_thread(1, true);
        assert_eq!(st.doc.design().threads.len(), n + 1);
        assert_ne!(st.current_thread, Some(first));
    }

    #[test]
    fn colorways_from_the_app() {
        let mut st = state();
        let red = st.doc.design().threads[0].id;
        st.new_colorway();
        st.current_thread = Some(red);
        st.use_chart_thread(10, false);
        let changed = st.doc.design().thread(red).unwrap().color;
        st.switch_colorway(0);
        assert_ne!(st.doc.design().thread(red).unwrap().color, changed);
        st.switch_colorway(1);
        assert_eq!(st.doc.design().thread(red).unwrap().color, changed);
        st.delete_colorway();
        assert_eq!(st.doc.design().colorways.len(), 1);
    }

    #[test]
    fn import_a_chart_file() {
        let dir = std::env::temp_dir().join(format!("emb-chart-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("mine.csv");
        std::fs::write(&p, "code,name,color\n1,Snow,#fafafa\n2,Coal,#101010\n").unwrap();
        let mut st = state();
        st.import_chart_file(&p);
        assert_eq!(st.chart().unwrap().name, "mine");
        assert_eq!(st.chart().unwrap().threads.len(), 2);
        std::fs::write(&p, "nonsense").unwrap();
        st.import_chart_file(&p);
        assert!(st.notice.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn panel_draws() {
        let mut st = state();
        st.new_colorway();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| panel(ui, &mut st));
            });
            out.textures_delta.clear();
        }
    }
}
